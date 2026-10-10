#!/usr/bin/env python3
"""The declared licence, the files shipped with the crate and the README say
the same thing.

Checks (each one counted; 0 checks run is a failure):
- `Cargo.toml` `license` is exactly EXPECTED
- every file in REQUIRED exists in the repo and in `cargo package --list`
- no file in FORBIDDEN exists in the repo or in the package
- README.md / README_JP.md each carry `License: EXPECTED` and no other
  `License:` line naming MIT

usage: scripts/license_check.py [--no-package]   (--no-package skips cargo)
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPECTED = "Apache-2.0"
REQUIRED = ["LICENSE-APACHE", "NOTICE", "TRADEMARK_NOTICE"]
FORBIDDEN = ["LICENSE-MIT", "LICENSE"]
READMES = ["README.md", "README_JP.md"]


def check(root: Path, package_list: list[str] | None) -> tuple[int, list[str]]:
    problems: list[str] = []
    checked = 0
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r'^license\s*=\s*"([^"]*)"', manifest, re.M)
    checked += 1
    if not m:
        problems.append("Cargo.toml has no license field")
    elif m.group(1) != EXPECTED:
        problems.append(f"Cargo.toml license is {m.group(1)!r}, expected {EXPECTED!r}")
    for name in REQUIRED:
        checked += 1
        if not (root / name).is_file():
            problems.append(f"{name} is missing")
        if package_list is not None:
            checked += 1
            if name not in package_list:
                problems.append(f"{name} is not in the published package")
    for name in FORBIDDEN:
        checked += 1
        if (root / name).exists():
            problems.append(f"{name} exists but the crate is {EXPECTED} only")
        if package_list is not None and name in package_list:
            problems.append(f"{name} is in the published package")
    for readme in READMES:
        checked += 1
        text = (root / readme).read_text(encoding="utf-8")
        lines = [l.strip() for l in text.splitlines() if l.strip().startswith("License:")]
        if lines != [f"License: {EXPECTED}"]:
            problems.append(f"{readme}: License line(s) {lines!r}, expected exactly one 'License: {EXPECTED}'")
    return checked, problems


def package_list(root: Path) -> list[str]:
    out = subprocess.run(
        ["cargo", "package", "--list", "--allow-dirty", "--quiet"],
        cwd=root, capture_output=True, text=True, check=True,
    ).stdout
    return [l.strip() for l in out.splitlines() if l.strip()]


def main() -> int:
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")
        except (AttributeError, ValueError):
            pass
    pkg = None if "--no-package" in sys.argv else package_list(ROOT)
    checked, problems = check(ROOT, pkg)
    for p in problems:
        print(f"error: {p}", file=sys.stderr)
    if checked == 0:
        print("error: license check compared nothing", file=sys.stderr)
        return 1
    print(f"license check: {checked} checks, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
