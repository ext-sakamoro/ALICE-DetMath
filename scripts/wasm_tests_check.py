#!/usr/bin/env python3
"""Every test target ran on wasm32-wasip1, or is excluded with a reason.

Reads the log of `cargo test --target wasm32-wasip1 --lib --tests` and the crate's targets from
`cargo metadata --no-deps` (kind "lib" and "test"; the check and cargo cannot disagree about which
targets exist). A target passes when its `Running` line is followed by `test result: ok. N passed`
with N > 0. A target listed in `scripts/wasm_test_exclusions.toml` [targets] is not required to run.

libtest skips a `#[should_panic]` test where a panic aborts the process (wasm32-wasip1): it prints
`test <name> - should panic ... ignored`. Each such test must be listed in [should_panic] as
`"<target>::<name>"`, with its reason; a skip that is not listed fails, and so does a listed entry
that names nothing the run skipped (or a target that does not exist). A target or a should-panic
skip is never dropped quietly.

Fails (exit 1) on any finding, and (exit 2) when no target ran or the inputs cannot be read.

    python3 scripts/wasm_tests_check.py <cargo test log> [--metadata <file>] [--exclusions <file>]
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXCLUSIONS = ROOT / "scripts" / "wasm_test_exclusions.toml"
ANSI = re.compile(r"\x1b\[[0-9;]*m")
RUNNING = re.compile(r"^\s*Running (?:unittests )?(\S+) \(")
RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored")
SKIP = re.compile(r"^test (\S+) - should panic \.\.\. ignored")


def targets_from_metadata(meta: dict) -> dict[str, str]:
    """source path relative to the package root -> target name, for the lib and test targets"""
    out = {}
    for pkg in meta.get("packages", []):
        root = Path(pkg["manifest_path"]).parent
        for t in pkg.get("targets", []):
            if {"lib", "test"} & set(t.get("kind", [])):
                out[str(Path(t["src_path"]).relative_to(root))] = t["name"]
    return out


def parse_log(text: str) -> tuple[dict[str, tuple[str, int]], dict[str, list[str]]]:
    """(target source path -> (result, passed), target source path -> should-panic tests skipped)"""
    results: dict[str, tuple[str, int]] = {}
    skips: dict[str, list[str]] = {}
    current = None
    for raw in text.splitlines():
        line = ANSI.sub("", raw)
        m = RUNNING.match(line)
        if m:
            current = m.group(1)
            skips.setdefault(current, [])
            continue
        if current is None:
            continue
        m = SKIP.match(line)
        if m:
            skips[current].append(m.group(1))
            continue
        m = RESULT.match(line)
        if m:
            results[current] = (m.group(1), int(m.group(2)))
            current = None
    return results, skips


def findings(targets: dict[str, str], results, skips, excl: dict) -> tuple[list[str], int]:
    out = []
    by_name = {name: src for src, name in targets.items()}
    ex_targets = excl.get("targets", {})
    ex_panic = excl.get("should_panic", {})
    for name, why in {**ex_targets, **{k: v for k, v in ex_panic.items()}}.items():
        if not isinstance(why, str) or not why.strip():
            out.append(f"exclusion {name!r} has no reason")
    for name in ex_targets:
        if name not in by_name:
            out.append(f"[targets] {name!r} names no test target of this crate")
    ran = 0
    seen_skips = set()
    for src, name in sorted(targets.items()):
        if name in ex_targets:
            if src in results:
                out.append(f"{name} is excluded but ran ({src}): remove the exclusion")
            continue
        res = results.get(src)
        if res is None:
            out.append(f"{name} ({src}) did not run on wasm and is not excluded")
            continue
        status, passed = res
        if status != "ok":
            out.append(f"{name} ({src}): test result {status}")
        elif passed == 0:
            out.append(f"{name} ({src}): 0 tests passed")
        else:
            ran += 1
        for test in skips.get(src, []):
            key = f"{name}::{test}"
            seen_skips.add(key)
            if key not in ex_panic:
                out.append(f"{key} was skipped as should_panic (panic=abort) and is not listed in [should_panic]")
    for key in ex_panic:
        if key not in seen_skips:
            out.append(f"[should_panic] {key!r} was not skipped in this run: remove it or fix the name")
    return out, ran


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("log", type=Path)
    ap.add_argument("--metadata", type=Path)
    ap.add_argument("--exclusions", type=Path, default=EXCLUSIONS)
    a = ap.parse_args(argv)
    try:
        text = a.log.read_text(encoding="utf-8", errors="replace")
        if a.metadata:
            meta = json.loads(a.metadata.read_text(encoding="utf-8"))
        else:
            meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT,
                                             capture_output=True, text=True, check=True).stdout)
        excl = tomllib.loads(a.exclusions.read_text(encoding="utf-8"))
    except (OSError, ValueError, subprocess.CalledProcessError) as e:
        print(f"wasm_tests_check: {e}", file=sys.stderr)
        return 2
    targets = targets_from_metadata(meta)
    if not targets:
        print("wasm_tests_check: cargo metadata lists no lib or test target", file=sys.stderr)
        return 2
    results, skips = parse_log(text)
    found, ran = findings(targets, results, skips, excl)
    for f in found:
        print(f"wasm_tests_check: {f}")
    n_skip = sum(len(v) for v in skips.values())
    print(f"wasm_tests_check: {len(targets)} targets, {ran} ran on wasm, {len(excl.get('targets', {}))} excluded, "
          f"{n_skip} should_panic skipped ({len(excl.get('should_panic', {}))} listed), {len(found)} finding(s)")
    if ran == 0:
        print("wasm_tests_check: no target ran", file=sys.stderr)
        return 2
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
