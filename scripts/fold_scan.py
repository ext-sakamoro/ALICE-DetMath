#!/usr/bin/env python3
"""Which repositories use alice-det-math's SEMANTICS_ID, read from their main branches.

scripts/fold_check.py builds the crates of .github/fold-consumers.txt against a commit that moves
SEMANTICS_ID. That list was kept by hand and missed two crates; this scan derives it. For every
public repository of the owner (non-fork, non-archived), at its `main` branch (not the default branch,
which can be another one; a repository without main is named on its own line):

  1. the manifests (`Cargo.toml` files, fetched alone in a blobless sparse clone) are read: a
     repository that does not depend on alice-det-math directly is done;
  2. its Rust sources (outside tests/, examples/, benches/, fuzz/) are fetched and read:
       breaks  a fold table that panics when it has no row for the identifier
               (`has no row in <NAME>_BY_DET_MATH`), or a compile-time assert that names
               `alice_det_math::SEMANTICS_ID` (`const _: () = assert!(` within 3 lines): a move
               fails the crate's compile; or a fold by function (any mention of `function_pin` or
               `FUNCTION_PINS`): a function it names that this checkout no longer has fails its compile
       moves   another use of `alice_det_math::SEMANTICS_ID`: a move changes an identifier the
               crate derives from it, without failing its compile

`check` compares the result with .github/fold-consumers.txt (the breakers) and
.github/fold-movers.txt (the others) and fails on any difference, and when no repository was
scanned. A repository the token cannot list (a private one, with the workflow's token) is not
scanned: the scan says so on its own line, it never counts as "no use".

Results are cached by repository, main's SHA and the scan's rules (this file's hash, `--cache
FILE`), so an unchanged repository is not fetched again until the rules change. A crate reaching the identifier through another crate's
re-export (`pub use alice_det_math::SEMANTICS_ID` in a dependency) is not followed; the
re-exporting crate is listed as moving.

  fold_scan.py check [--owner O] [--cache FILE]       exit 0 / 1 (differs), 2 (input error)
  fold_scan.py print [--owner O] [--cache FILE]       the two lists as found
"""
from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = "alice-det-math"
CONSUMERS = ".github/fold-consumers.txt"
MOVERS = ".github/fold-movers.txt"
SKIP_DIRS = {"tests", "examples", "benches", "fuzz", "target"}
TABLE_PANIC_RE = re.compile(r"has no row in [A-Z0-9_]*_BY_DET_MATH")
ASSERT_RE = re.compile(r"const\s+_\s*:\s*\(\)\s*=\s*assert!\(")
USE_RE = re.compile(r"alice_det_math::SEMANTICS_ID\b")
FUNCTION_PIN_RE = re.compile(r"\b(?:function_pin|FUNCTION_PINS)\b")
DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
# the scan's rules: this file's bytes. A cached result found under other rules is scanned again, so a
# change of the classification reaches repositories whose main did not move
RULES = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


class InputError(Exception):
    pass


@dataclass
class Repo:
    name: str
    url: str


@dataclass
class Found:
    depends: bool = False
    breaks: list[str] = field(default_factory=list)   # files
    moves: list[str] = field(default_factory=list)
    defines_det_math: bool = False


def git(args: list[str], cwd: Path | None = None) -> str:
    p = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if p.returncode != 0:
        raise InputError(f"git {' '.join(args[:3])}: {(p.stderr or p.stdout).strip()[-400:]}")
    return p.stdout


def list_repos(owner: str) -> list[Repo]:
    p = subprocess.run(["gh", "api", "--paginate", f"users/{owner}/repos?per_page=100&type=owner",
                        "-q", '.[] | select(.fork | not) | select(.archived | not) | [.name, .clone_url, .visibility] | @tsv'],
                       capture_output=True, text=True)
    if p.returncode != 0:
        raise InputError(f"listing the repositories of {owner}: {p.stderr.strip()[-400:]}")
    out = []
    for ln in p.stdout.splitlines():
        name, url, vis = ln.split("\t")
        # public ones only, whatever the token can see: the same set locally and in CI
        if vis == "public":
            out.append(Repo(name, url))
    return out


def main_sha(url: str) -> str | None:
    """the tip of `main` (not the default branch: a repository's default can be another branch, and
    the fold check builds main), or None when there is no main"""
    out = git(["ls-remote", url, "refs/heads/main"]).split()
    if not out:
        return None
    if not re.fullmatch(r"[0-9a-f]{40}", out[0]):
        raise InputError(f"{url}: unreadable main ({out[0]!r})")
    return out[0]


def depends_on_det_math(doc: dict) -> bool:
    tables = [doc.get(t) for t in DEP_TABLES] + [(doc.get("workspace") or {}).get("dependencies")]
    for spec in (doc.get("target") or {}).values():
        if isinstance(spec, dict):
            tables += [spec.get(t) for t in DEP_TABLES]
    for t in tables:
        if isinstance(t, dict):
            for key, spec in t.items():
                if (spec.get("package", key) if isinstance(spec, dict) else key) == PACKAGE:
                    return True
    return False


def classify(path: str, text: str) -> str | None:
    """'breaks', 'moves' or None for one source file"""
    if TABLE_PANIC_RE.search(text):
        return "breaks"
    lines = text.splitlines()
    for i, ln in enumerate(lines):
        if ASSERT_RE.search(ln) and any(USE_RE.search(x) for x in lines[i:i + 4]):
            return "breaks"
    code = "\n".join(ln for ln in lines if not ln.lstrip().startswith("//"))
    if FUNCTION_PIN_RE.search(code):
        return "breaks"
    return "moves" if USE_RE.search(code) else None


def scan_repo(url: str, sha: str, work: Path) -> Found:
    d = work / "r"
    git(["clone", "-q", "--depth", "1", "--filter=blob:none", "--no-checkout", "--sparse", "--branch", "main", url, str(d)])
    git(["sparse-checkout", "set", "--no-cone", "/**/Cargo.toml", "/Cargo.toml"], cwd=d)
    git(["-c", "advice.detachedHead=false", "checkout", "-q", sha], cwd=d)
    found = Found()
    for m in d.rglob("Cargo.toml"):
        if SKIP_DIRS & set(m.relative_to(d).parts):
            continue
        try:
            doc = tomllib.loads(m.read_text(encoding="utf-8"))
        except (tomllib.TOMLDecodeError, UnicodeDecodeError):
            continue
        if (doc.get("package") or {}).get("name") == PACKAGE:
            return Found(defines_det_math=True)   # the crate itself: its source defines what the others use
        if depends_on_det_math(doc):
            found.depends = True
    if not found.depends:
        return found
    git(["sparse-checkout", "set", "--no-cone", "/**/Cargo.toml", "/Cargo.toml", "/**/*.rs", "/*.rs"], cwd=d)
    for f in sorted(d.rglob("*.rs")):
        rel = f.relative_to(d)
        if SKIP_DIRS & set(rel.parts):
            continue
        kind = classify(str(rel), f.read_text(encoding="utf-8", errors="replace"))
        if kind == "breaks":
            found.breaks.append(str(rel))
        elif kind == "moves":
            found.moves.append(str(rel))
    return found


def scan(repos: list[Repo], cache_path: Path | None,
         sha_of: Callable[[str], str | None] = main_sha, scanner: Callable[[str, str, Path], Found] = scan_repo
         ) -> tuple[dict[str, Found], int, int, list[str]]:
    """(found per repository, repositories scanned, of which fetched now, repositories without main)"""
    cache: dict = {}
    if cache_path and cache_path.is_file():
        try:
            cache = json.loads(cache_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            cache = {}
    results: dict[str, Found] = {}
    fetched = 0

    def one(r: Repo) -> tuple[str, str | None, Found | None, bool]:
        sha = sha_of(r.url)
        if sha is None:
            return r.name, None, None, False
        c = cache.get(r.name)
        if c and c.get("sha") == sha and c.get("rules") == RULES:
            return r.name, sha, Found(c["depends"], c["breaks"], c["moves"], c.get("defines_det_math", False)), False
        with tempfile.TemporaryDirectory(prefix="fold-scan-") as t:
            return r.name, sha, scanner(r.url, sha, Path(t)), True

    no_main: list[str] = []
    with concurrent.futures.ThreadPoolExecutor(8) as ex:
        for name, sha, f, new in ex.map(one, repos):
            if f is None:
                no_main.append(name)
                continue
            results[name] = f
            fetched += new
            cache[name] = {"sha": sha, "rules": RULES, "depends": f.depends, "breaks": f.breaks, "moves": f.moves,
                           "defines_det_math": f.defines_det_math}
    if cache_path:
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        cache_path.write_text(json.dumps(cache, indent=0, sort_keys=True), encoding="utf-8")
    return results, len(results), fetched, sorted(no_main)


def read_list(path: Path) -> list[str]:
    try:
        return sorted(ln.strip() for ln in path.read_text(encoding="utf-8").splitlines()
                      if ln.strip() and not ln.lstrip().startswith("#"))
    except OSError as e:
        raise InputError(f"{path}: {e}") from None


def report(results: dict[str, Found], scanned: int, fetched: int, owner: str, no_main: list[str]) -> tuple[list[str], list[str]]:
    breaks = sorted(n for n, f in results.items() if f.breaks)
    moves = sorted(n for n, f in results.items() if f.moves and not f.breaks)
    users = sum(1 for f in results.values() if f.depends)
    print(f"fold scan: {scanned} repositories of {owner} scanned at main "
          f"({fetched} fetched, {scanned - fetched} from the cache); {users} depend on {PACKAGE}")
    if no_main:
        print(f"fold scan: {len(no_main)} public repositories have no main branch, not scanned: {', '.join(no_main)}")
    for n in sorted(n for n, f in results.items() if f.defines_det_math):
        print(f"fold scan: {n} defines {PACKAGE}: not counted as a user of it")
    for n in breaks:
        print(f"fold scan: breaks {n}: {', '.join(results[n].breaks)}")
    for n in moves:
        print(f"fold scan: moves  {n}: {', '.join(results[n].moves)}")
    print(f"fold scan: private repositories of {owner} are not scanned (public ones only, a gap not a result)")
    return breaks, moves


def compare(breaks: list[str], moves: list[str], want_b: list[str], want_m: list[str]) -> list[str]:
    bad = []
    for label, verb, found, listed, f in (("breaks", "break", breaks, want_b, CONSUMERS),
                                          ("moves", "move", moves, want_m, MOVERS)):
        for n in sorted(set(found) - set(listed)):
            bad.append(f"{n} {label} at its main but is not in {f}")
        for n in sorted(set(listed) - set(found)):
            bad.append(f"{n} is in {f} but does not {verb} at its main")
    return bad


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("cmd", choices=("check", "print"))
    ap.add_argument("--owner", default="ext-sakamoro")
    ap.add_argument("--cache", type=Path)
    ap.add_argument("--root", type=Path, default=ROOT)
    a = ap.parse_args(argv)
    try:
        repos = list_repos(a.owner)
        if not repos:
            raise InputError(f"no repository of {a.owner} listed (0 scanned)")
        results, scanned, fetched, no_main = scan(repos, a.cache)
        if scanned == 0:
            raise InputError("0 repositories scanned")
        breaks, moves = report(results, scanned, fetched, a.owner, no_main)
        if a.cmd == "print":
            return 0
        want_b, want_m = read_list(a.root / CONSUMERS), read_list(a.root / MOVERS)
    except InputError as e:
        print(f"fold scan: {e}", file=sys.stderr)
        return 2
    bad = compare(breaks, moves, want_b, want_m)
    for b in bad:
        print(f"fold scan: {b}")
    print(f"fold scan: {len(want_b)} + {len(want_m)} listed, {len(breaks)} + {len(moves)} found, {len(bad)} difference(s)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
