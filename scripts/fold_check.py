#!/usr/bin/env python3
"""Whether the crates that fold alice-det-math's SEMANTICS_ID still compile against this commit.

A crate that stamps its stored values with an identifier of its own folds det-math's
SEMANTICS_ID into it through a table (`ZIP_SEMANTICS_BY_DET_MATH` and the like): one row per
det-math identifier, and a const-eval panic (`... has no row in ...: add its fold`) when the
det-math it is built with has none. A det-math commit that moves SEMANTICS_ID therefore breaks
every such crate until it has the row. This tool builds each of them against this checkout.

The crates are listed in .github/fold-consumers.txt (one repository name per line). For each
one .github/fold-wave.toml says what to build:

  [consumers]
  ALICE-Zip = "main"                                        # its main must already have the row
  ALICE-SDF = { branch = "ci/<id>", sha = "<40 hex>" }      # its wave branch: the SHA must be the
                                                            # branch tip and its CI must be green

A listed crate without an entry, an entry for a crate not listed, and a list with no crate
fail. Subcommands:

  ids --base <ref>         SEMANTICS_ID of this checkout and of <ref> (a probe crate prints each);
                           writes moved=true|false to $GITHUB_OUTPUT; both must be read
  plan                     checks the list against the wave file; writes the matrix
  --override ALICE-X=ci/<branch>@<sha>[,...]   (or $FOLD_OVERRIDE, a manual run's input) puts a wave
                           branch in place of listed crates' entries, checked like a wave entry
  consumer --name <repo> --result <file>
                           fetches the crate at its entry's SHA (and the siblings its
                           .github/alice-siblings pins), points every requirement on
                           alice-det-math at this checkout's version, patches every source of
                           alice-det-math with this checkout, then checks with `cargo metadata`
                           that every alice-det-math in the resolve is this checkout (a patch
                           cargo ignored would build the old det-math and pass: red), and runs
                           `cargo check --lib`; a missing fold row is reported as such
  tally --results <dir>    one passing result per listed crate, and more than 0, or it fails

Exit 0 when the check holds, 1 when it does not, 2 on an input error.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = "alice-det-math"
CONSUMERS = ".github/fold-consumers.txt"
WAVE = ".github/fold-wave.toml"
SHA_RE = re.compile(r"[0-9a-f]{40}")
NAME_RE = re.compile(r"ALICE-[A-Za-z0-9-]+")
# a fold table without the row (`... has no row in <TABLE>: add its fold`), or a compile-time assert of the
# det-math identifier a crate recorded (ALICE-Font: `alice-det-math generation changed: re-record ...`)
# and a fold by function naming one this checkout does not have (`function_pin`)
MISSING_ROW_RE = re.compile(r"has no row in ([A-Z0-9_]+)|alice-det-math (generation) changed")
MISSING_FUNCTION = "not a function of alice-det-math"
MOVED_FUNCTION = "a function this crate calls moved in alice-det-math"
OK_CONCLUSIONS = {"success", "skipped", "neutral"}
DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


class InputError(Exception):
    pass


@dataclass(frozen=True)
class Entry:
    ref: str            # "main" or a ci/ branch
    sha: str | None     # the pinned SHA of a ci/ branch


def run(argv: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess:
    p = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if check and p.returncode != 0:
        raise InputError(f"{' '.join(argv[:4])} failed (exit {p.returncode}): {(p.stderr or p.stdout).strip()[-800:]}")
    return p


# --- the list and the wave file ---------------------------------------------------------

def read_consumers(path: Path) -> list[str]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as e:
        raise InputError(f"{path}: {e}") from None
    names = [ln.strip() for ln in lines if ln.strip() and not ln.lstrip().startswith("#")]
    for n in names:
        if not NAME_RE.fullmatch(n):
            raise InputError(f"{path}: {n!r} is not a repository name")
    if len(set(names)) != len(names):
        raise InputError(f"{path}: a repository is listed twice")
    if not names:
        raise InputError(f"{path}: no crate listed (0 consumers would be checked)")
    return names


def read_wave(path: Path, consumers: list[str]) -> dict[str, Entry]:
    try:
        doc = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as e:
        raise InputError(f"{path}: {e}") from None
    table = doc.get("consumers")
    if not isinstance(table, dict):
        raise InputError(f"{path}: no [consumers] table")
    missing = [n for n in consumers if n not in table]
    extra = [n for n in table if n not in consumers]
    if missing or extra:
        raise InputError(f"{path}: " + "; ".join(
            ([f"no entry for {', '.join(missing)}"] if missing else [])
            + ([f"an entry for a crate not in {CONSUMERS}: {', '.join(extra)}"] if extra else [])))
    out: dict[str, Entry] = {}
    for name in consumers:
        v = table[name]
        if v == "main":
            out[name] = Entry("main", None)
        elif isinstance(v, dict) and set(v) == {"branch", "sha"} and isinstance(v["branch"], str) \
                and v["branch"].startswith("ci/") and isinstance(v["sha"], str) and SHA_RE.fullmatch(v["sha"]):
            out[name] = Entry(v["branch"], v["sha"])
        else:
            raise InputError(f"{path}: {name} = {v!r}: needs \"main\" or {{ branch = \"ci/...\", sha = \"<40 hex>\" }}")
    return out


def apply_override(wave: dict[str, Entry], text: str) -> dict[str, Entry]:
    """`ALICE-X=ci/<branch>@<40 hex>[,ALICE-Y=...]` (a manual run's input) put in place of those crates'
    entries. Only a ci/ branch pinned by full SHA can be named, and it is checked like a wave entry (the
    SHA must be the branch tip with green CI), so an override can point a run at a wave branch but cannot
    make a crate pass that does not"""
    out = dict(wave)
    seen: set[str] = set()
    for part in [p.strip() for p in text.split(",") if p.strip()]:
        name, sep, rest = part.partition("=")
        branch, at, sha = rest.rpartition("@")
        if not sep or not at or name not in wave:
            raise InputError(f"override {part!r}: needs <listed crate>=ci/<branch>@<40 hex>")
        if not branch.startswith("ci/") or branch == "ci/" or not SHA_RE.fullmatch(sha):
            raise InputError(f"override {part!r}: only a ci/<name> branch pinned by its full SHA")
        if name in seen:
            raise InputError(f"override {part!r}: {name} is named twice")
        seen.add(name)
        out[name] = Entry(branch, sha)
    return out


# --- SEMANTICS_ID ------------------------------------------------------------------------

def probe_id(crate: Path, work: Path) -> str:
    """the SEMANTICS_ID of the det-math at `crate`, printed by a probe crate that depends on it by path"""
    probe = work / "probe"
    (probe / "src").mkdir(parents=True)
    (probe / "Cargo.toml").write_text(
        '[package]\nname = "fold-id-probe"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n[workspace]\n'
        f'[dependencies]\n{PACKAGE} = {{ path = {json.dumps(str(crate))}, default-features = false }}\n',
        encoding="utf-8")
    (probe / "src" / "main.rs").write_text(
        "fn main() { for b in alice_det_math::SEMANTICS_ID { print!(\"{b:02x}\"); } println!(); }\n", encoding="utf-8")
    # cargo runs in the det-math checkout: its rust-toolchain.toml picks the toolchain of every build here
    p = run(["cargo", "run", "-q", "--manifest-path", str(probe / "Cargo.toml")], cwd=crate, check=False)
    out = p.stdout.strip()
    if p.returncode != 0 or not re.fullmatch(r"[0-9a-f]{64}", out):
        raise InputError(f"the SEMANTICS_ID of {crate} could not be read (exit {p.returncode}): "
                         f"{(p.stderr or p.stdout).strip()[-800:]}")
    return out


def cmd_ids(a: argparse.Namespace) -> int:
    root = Path(a.candidate).resolve()
    with tempfile.TemporaryDirectory(prefix="fold-ids-") as t:
        t = Path(t)
        base = t / "base"
        base.mkdir()
        arch = subprocess.run(["git", "-C", str(root), "archive", "--format=tar", a.base], capture_output=True)
        if arch.returncode != 0:
            raise InputError(f"git archive {a.base}: {arch.stderr.decode(errors='replace').strip()}")
        subprocess.run(["tar", "-x", "-C", str(base)], input=arch.stdout, check=True)
        (t / "c").mkdir()
        (t / "b").mkdir()
        now = probe_id(root, t / "c")
        before = probe_id(base, t / "b")
    moved = now != before
    print(f"fold: SEMANTICS_ID at {a.base}: {before}")
    print(f"fold: SEMANTICS_ID here: {now}")
    print(f"fold: 2 identifiers read, moved: {'yes' if moved else 'no'}")
    write_output(a.github_output, {"moved": "true" if moved else "false", "id": now})
    return 0


def write_output(path: str | None, values: dict[str, str]) -> None:
    if path:
        with open(path, "a", encoding="utf-8") as f:
            for k, v in values.items():
                f.write(f"{k}={v}\n")


# --- one consumer --------------------------------------------------------------------------

def ls_remote(url: str, ref: str) -> str:
    p = run(["git", "ls-remote", url, f"refs/heads/{ref}"])
    shas = [ln.split("\t")[0] for ln in p.stdout.splitlines() if ln.endswith(f"\trefs/heads/{ref}")]
    if len(shas) != 1:
        raise InputError(f"{url}: no branch {ref}")
    return shas[0]


def gh_check_runs(owner: str, repo: str, sha: str) -> list[tuple[str, str, str]]:
    p = run(["gh", "api", "--paginate", f"repos/{owner}/{repo}/commits/{sha}/check-runs",
             "-q", ".check_runs[] | [.name, .status, (.conclusion // \"\")] | @tsv"])
    return [tuple(ln.split("\t")) for ln in p.stdout.splitlines() if ln.strip()]  # type: ignore[misc]


def resolve(name: str, entry: Entry, url: str, owner: str,
            check_runs: Callable[[str, str, str], list[tuple[str, str, str]]]) -> str:
    """the SHA to build: main's tip, or the pinned SHA of a wave branch that is still its tip with green CI"""
    tip = ls_remote(url, entry.ref)
    if entry.sha is None:
        return tip
    if tip != entry.sha:
        raise CheckFailed("stale-sha", f"{name}: {entry.ref} is at {tip[:12]}, the wave file pins {entry.sha[:12]}")
    runs = check_runs(owner, name, entry.sha)
    bad = [f"{n} {s}/{c or '-'}" for n, s, c in runs if s != "completed" or c not in OK_CONCLUSIONS]
    if not runs or bad:
        raise CheckFailed("ci-not-green", f"{name}: CI of {entry.ref} at {entry.sha[:12]} is not green "
                          f"({len(runs)} check run(s){': ' + ', '.join(bad[:8]) if bad else ''})")
    return entry.sha


class CheckFailed(Exception):
    def __init__(self, kind: str, msg: str):
        super().__init__(msg)
        self.kind = kind


def fetch(url: str, sha: str, dest: Path) -> None:
    dest.mkdir(parents=True)
    run(["git", "init", "-q"], cwd=dest)
    run(["git", "fetch", "-q", "--depth", "1", url, sha], cwd=dest)
    run(["git", "-c", "advice.detachedHead=false", "checkout", "-q", "FETCH_HEAD"], cwd=dest)


def siblings(tree: Path) -> list[tuple[str, str]]:
    f = tree / ".github" / "alice-siblings"
    if not f.is_file():
        return []
    out = []
    for ln in f.read_text(encoding="utf-8").splitlines():
        ln = ln.split("#", 1)[0].strip()
        if not ln:
            continue
        parts = ln.split()
        if len(parts) != 2 or not NAME_RE.fullmatch(parts[0]) or not SHA_RE.fullmatch(parts[1]):
            raise InputError(f"{f}: {ln!r}: needs `<Repo> <40-hex SHA>`")
        out.append((parts[0], parts[1]))
    return out


def manifests(layout: Path) -> list[Path]:
    return sorted(p for p in layout.rglob("Cargo.toml") if not ({"target", ".git"} & set(p.relative_to(layout).parts)))


def det_math_specs(doc: dict) -> list[object]:
    tables = [doc.get(t) for t in DEP_TABLES] + [(doc.get("workspace") or {}).get("dependencies")]
    for spec in (doc.get("target") or {}).values():
        if isinstance(spec, dict):
            tables += [spec.get(t) for t in DEP_TABLES]
    out = []
    for t in tables:
        if isinstance(t, dict):
            for key, spec in t.items():
                pkg = spec.get("package", key) if isinstance(spec, dict) else key
                if pkg == PACKAGE:
                    out.append(spec)
    return out


INLINE_RE = re.compile(r'^(\s*alice-det-math\s*=\s*)"[^"]*"', re.M)
TABLE_RE = re.compile(r'^(\s*alice-det-math\s*=\s*\{[^\n}]*?\bversion\s*=\s*)"[^"]*"', re.M)


def rewrite_versions(layout: Path, version: str) -> tuple[int, set[str]]:
    """every requirement on alice-det-math becomes `version` (in the fetched copies only); returns the
    number of requirements and the sources ("crates-io" or a git URL) they name"""
    count, sources = 0, set()
    for m in manifests(layout):
        text = m.read_text(encoding="utf-8")
        new = TABLE_RE.sub(lambda x: f'{x.group(1)}"={version}"', INLINE_RE.sub(lambda x: f'{x.group(1)}"={version}"', text))
        specs = det_math_specs(tomllib.loads(new))
        for spec in specs:
            if isinstance(spec, dict) and spec.get("workspace") is True:
                continue
            req = spec if isinstance(spec, str) else spec.get("version")
            if isinstance(spec, dict) and "path" in spec:
                raise InputError(f"{m}: alice-det-math by path ({spec['path']}): not a form this check can patch")
            if req is not None and req != f"={version}":
                raise InputError(f"{m}: a requirement on alice-det-math in a form the rewrite does not handle: {spec!r}")
            sources.add(spec["git"] if isinstance(spec, dict) and "git" in spec else "crates-io")
            count += 1
        if new != text:
            m.write_text(new, encoding="utf-8")
    return count, sources


def patch_config(sources: set[str], candidate: Path) -> list[str]:
    out = []
    for s in sorted(sources | {"crates-io"}):
        key = "patch.crates-io" if s == "crates-io" else f"patch.{json.dumps(s)}"
        out += ["--config", f"{key}.{PACKAGE}.path={json.dumps(str(candidate))}"]
    return out


def resolved_det_maths(manifest: Path, config: list[str], cwd: Path) -> list[tuple[str, list[str]]]:
    """each alice-det-math in the resolve: its directory, and who pulls it (`<package> <version>
    (alice-det-math <requirement>)` for each package of the resolve that depends on it)"""
    p = run(["cargo", "metadata", "-q", "--format-version", "1", "--manifest-path", str(manifest), *config],
            cwd=cwd, check=False)
    if p.returncode != 0:
        raise CheckFailed("build-failed", f"cargo metadata failed: {p.stderr.strip()[-1500:]}")
    meta = json.loads(p.stdout)
    nodes = (meta.get("resolve") or {}).get("nodes", [])
    in_resolve = {n["id"] for n in nodes}
    packages = {pk["id"]: pk for pk in meta["packages"]}
    out = []
    for pid, pk in packages.items():
        if pk["name"] != PACKAGE or pid not in in_resolve:
            continue
        via = []
        for n in nodes:
            if any(d.get("pkg") == pid for d in n.get("deps", [])):
                parent = packages[n["id"]]
                reqs = sorted({d["req"] for d in parent.get("dependencies", [])
                               if (d.get("rename") or d["name"]) == PACKAGE or d["name"] == PACKAGE})
                via.append(f"{parent['name']} {parent['version']} ({PACKAGE} {' / '.join(reqs) or '?'})")
        out.append((os.path.dirname(pk["manifest_path"]), sorted(via)))
    return out


def check_consumer(name: str, entry: Entry, candidate: Path, work: Path, url_base: str, owner: str,
                   check_runs=gh_check_runs) -> dict:
    version = tomllib.loads((candidate / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    res = {"name": name, "ref": entry.ref, "sha": None, "result": None, "detail": ""}
    try:
        sha = resolve(name, entry, f"{url_base}/{name}", owner, check_runs)
        res["sha"] = sha
        layout = work / "layout"
        fetch(f"{url_base}/{name}", sha, layout / name)
        for sib, ssha in siblings(layout / name):
            fetch(f"{url_base}/{sib}", ssha, layout / sib)
        n, sources = rewrite_versions(layout, version)
        if n == 0:
            raise CheckFailed("no-dependency", f"{name}: no requirement on {PACKAGE} found (0 compared)")
        config = patch_config(sources, candidate)
        manifest = layout / name / "Cargo.toml"
        resolved = resolved_det_maths(manifest, config, candidate)
        paths = [d for d, _ in resolved]
        want = os.path.realpath(candidate)
        others = sorted((d, via) for d, via in resolved if os.path.realpath(d) != want)
        if not resolved:
            raise CheckFailed("nothing-resolved", f"{name}: {n} requirement(s) on {PACKAGE} but none in the resolve "
                              "(an optional or target-only dependency the default build does not enable): "
                              "nothing would be compared")
        if others:
            where = "; ".join(f"{d} via {', '.join(via) or 'no dependent in the resolve'}" for d, via in others)
            raise CheckFailed("patch-ignored", f"{name}: alice-det-math in the resolve is not this checkout "
                              f"({len(resolved)} resolved; elsewhere: {where or 'none resolved'})")
        p = run(["cargo", "check", "-q", "--lib", "--manifest-path", str(manifest), *config], cwd=candidate, check=False)
        if p.returncode != 0:
            if MISSING_FUNCTION in p.stderr:
                raise CheckFailed("missing-row", f"{name}: its fold names a function this checkout does not have "
                                  f"(on main, or on the wave branch named in {WAVE})")
            if MOVED_FUNCTION in p.stderr:
                raise CheckFailed("missing-row", f"{name}: this checkout moves a function its fold records: "
                                  f"re-record it (on main, or on the wave branch named in {WAVE})")
            m = MISSING_ROW_RE.search(p.stderr)
            if m and m.group(1):
                raise CheckFailed("missing-row", f"{name}: {m.group(1)} has no row for this SEMANTICS_ID: "
                                  f"add its fold (on main, or on the wave branch named in {WAVE})")
            if m:
                raise CheckFailed("missing-row", f"{name}: its recorded det-math generation is not this SEMANTICS_ID: "
                                  f"re-record it (on main, or on the wave branch named in {WAVE})")
            raise CheckFailed("build-failed", f"{name}: cargo check failed:\n{p.stderr.strip()[-2000:]}")
        res.update(result="ok", detail=f"{n} requirement(s) on {PACKAGE}, {len(paths)} resolved to this checkout, "
                                        f"sources: {', '.join(sorted(sources))}")
    except CheckFailed as e:
        res.update(result=e.kind, detail=str(e))
    return res


def cmd_consumer(a: argparse.Namespace) -> int:
    root = Path(a.candidate).resolve()
    consumers = read_consumers(root / CONSUMERS)
    if a.name not in consumers:
        raise InputError(f"{a.name} is not listed in {CONSUMERS}")
    wave = read_wave(root / WAVE, consumers)
    if a.override:
        wave = apply_override(wave, a.override)
        print(f"fold: override from the run's input: {a.override}")
    entry = wave[a.name]
    with tempfile.TemporaryDirectory(prefix="fold-") as t:
        res = check_consumer(a.name, entry, root, Path(t), a.url_base, a.owner)
    at = f" at {res['sha'][:12]}" if res["sha"] else ""
    print(f"fold: {a.name} ({res['ref']}{at}): {res['result']}: {res['detail']}")
    Path(a.result).write_text(json.dumps(res) + "\n", encoding="utf-8")
    return 0 if res["result"] == "ok" else 1


def cmd_plan(a: argparse.Namespace) -> int:
    root = Path(a.candidate).resolve()
    consumers = read_consumers(root / CONSUMERS)
    wave = read_wave(root / WAVE, consumers)
    if a.override:
        wave = apply_override(wave, a.override)
        print(f"fold: override from the run's input: {a.override}")
    for n in consumers:
        e = wave[n]
        print(f"fold: {n}: {e.ref}" + (f" @ {e.sha}" if e.sha else ""))
    print(f"fold: {len(consumers)} consumer(s) to check")
    write_output(a.github_output, {"matrix": json.dumps(consumers, separators=(",", ":"))})
    return 0


def cmd_tally(a: argparse.Namespace) -> int:
    root = Path(a.candidate).resolve()
    consumers = read_consumers(root / CONSUMERS)
    got: dict[str, dict] = {}
    for f in sorted(Path(a.results).rglob("*.json")):
        r = json.loads(f.read_text(encoding="utf-8"))
        got[r["name"]] = r
    ok = [n for n in consumers if got.get(n, {}).get("result") == "ok"]
    for n in consumers:
        r = got.get(n)
        print(f"fold: {n}: {r['result'] if r else 'no result'}")
    print(f"fold: {len(ok)} of {len(consumers)} consumer(s) checked and passing")
    return 0 if ok and len(ok) == len(consumers) else 1


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--candidate", default=str(ROOT))
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("ids")
    s.add_argument("--base", required=True)
    s.add_argument("--github-output", default=os.environ.get("GITHUB_OUTPUT"))
    s = sub.add_parser("plan")
    s.add_argument("--github-output", default=os.environ.get("GITHUB_OUTPUT"))
    s.add_argument("--override", default=os.environ.get("FOLD_OVERRIDE", ""))
    s = sub.add_parser("consumer")
    s.add_argument("--name", required=True)
    s.add_argument("--result", required=True)
    s.add_argument("--owner", default="ext-sakamoro")
    s.add_argument("--url-base", default="https://github.com/ext-sakamoro")
    s.add_argument("--override", default=os.environ.get("FOLD_OVERRIDE", ""))
    s = sub.add_parser("tally")
    s.add_argument("--results", required=True)
    a = ap.parse_args(argv)
    try:
        return {"ids": cmd_ids, "plan": cmd_plan, "consumer": cmd_consumer, "tally": cmd_tally}[a.cmd](a)
    except InputError as e:
        print(f"fold: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
