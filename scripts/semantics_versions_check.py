#!/usr/bin/env python3
"""scripts/semantics_versions.toml is append-only: every row the base has stays, with the same identifier.

tests/semantics_version.rs holds the crate's SEMANTICS_ID to the row of its own version; this keeps that
row honest, so an identifier cannot move under an unchanged version by editing its row instead of adding
a version. A base row removed or changed is a finding; new rows pass. Comparing no row of the base fails
(exit 2), as does a table that cannot be read.

    python3 scripts/semantics_versions_check.py --base <git ref>        (the base's table from git)
    python3 scripts/semantics_versions_check.py --base-file <path>      (a base table on disk)
    python3 scripts/semantics_versions_check.py --ci --event E --ref R --before SHA

`--ci` takes the base from scripts/ci_base.py (shared with the other ALICE crates): on a push to main the
commit main stood at before the push (`github.event.before`), elsewhere the merge-base with origin/main; a
base that cannot be named fails (exit 2). Every run prints the base and the rule that named it.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ci_base import ci_base, describe  # noqa: E402  (scripts/ci_base.py, a shared file)

TABLE = Path(__file__).resolve().parent / "semantics_versions.toml"
ROW = re.compile(r'^"([^"]+)"\s*=\s*"([0-9a-f]{64})"$')


def rows(text: str) -> dict[str, str]:
    out: dict[str, str] = {}
    for n, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        m = ROW.match(line)
        if not m:
            raise ValueError(f"line {n}: not `\"version\" = \"<64 lower-case hex>\"`: {line!r}")
        if m.group(1) in out:
            raise ValueError(f"line {n}: {m.group(1)} has two rows")
        out[m.group(1)] = m.group(2)
    return out


def findings(base: dict[str, str], head: dict[str, str]) -> list[str]:
    out = []
    for version, ident in sorted(base.items()):
        if version not in head:
            out.append(f"{version}: the base records {ident[:8]}, and the row is gone (rows are append-only)")
        elif head[version] != ident:
            out.append(f"{version}: the base records {ident[:8]}, now {head[version][:8]} (a new identifier "
                       "needs a new version, not a changed row)")
    return out


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--base")
    g.add_argument("--base-file", type=Path)
    g.add_argument("--ci", action="store_true")
    ap.add_argument("--event", default="")
    ap.add_argument("--ref", default="")
    ap.add_argument("--before", default="")
    ap.add_argument("--table", type=Path, default=TABLE)
    a = ap.parse_args(argv)
    try:
        if a.ci:
            a.base, rule = ci_base(a.event, a.ref, a.before)
            print(f"semantics_versions_check: {describe(a.base, rule, a.event, a.ref)}")
        elif a.base is not None:
            print(f"semantics_versions_check: base {a.base} (rule --base)")
        else:
            print(f"semantics_versions_check: base {a.base_file} (rule --base-file)")
        head = rows(a.table.read_text(encoding="utf-8"))
        if a.base_file is not None:
            base_text = a.base_file.read_text(encoding="utf-8")
        else:
            rel = "scripts/semantics_versions.toml"
            present = subprocess.run(["git", "cat-file", "-e", f"{a.base}^{{tree}}"], capture_output=True)
            if present.returncode == 0 and subprocess.run(["git", "cat-file", "-e", f"{a.base}:{rel}"],
                                                          capture_output=True).returncode != 0:
                # the commit range adds the table: there is no base row to keep yet (an empty table at the base
                # is not this case and still fails as nothing compared)
                print(f"semantics_versions_check: {a.base} has no {rel} (the table is new here), {len(head)} row(s) now")
                return 0
            r = subprocess.run(["git", "show", f"{a.base}:{rel}"], capture_output=True, text=True)
            if r.returncode != 0:
                print(f"semantics_versions_check: {a.base}:{rel} is not readable: {r.stderr.strip()}", file=sys.stderr)
                return 2
            base_text = r.stdout
        base = rows(base_text)
    except (OSError, ValueError) as e:
        print(f"semantics_versions_check: {e}", file=sys.stderr)
        return 2
    if not base:
        print("semantics_versions_check: the base has no row (compared nothing)", file=sys.stderr)
        return 2
    found = findings(base, head)
    for f in found:
        print(f"semantics_versions_check: {f}")
    print(f"semantics_versions_check: compared {len(base)} base row(s), {len(head)} now, {len(found)} finding(s)")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
