#!/usr/bin/env python3
"""A test that pins a Law digest reads float bits only through tests/support/digest_bits.rs.

A digest over output bits moves on one target when a function returns the platform's NaN
(sign bit set on x86-64) instead of the canonical one. digest_bits.rs refuses such a NaN through
alice_det_math::law_bits (bits32 / bits64 / out32 / out64) and gives the bits of a bit-pattern input as they are
(pattern32 / pattern64). This check finds the test files that compute a digest (a SHA-256,
BLAKE3 or FNV-1a hasher in the file) and reports every `to_bits` in them: a raw `.to_bits()`
hashes a leaked NaN as it is, and one mapped to the canonical NaN before hashing hides it.

    law_bits_check.py [ROOT]

Exit 0 when no finding, 1 on a finding, 2 when tests/support/digest_bits.rs is missing or no
digest test was found (0 scanned is a failure, never a pass). `to_le_bytes()` called on a
float directly is not seen (the receiver's type is not known here).
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

HELPER = Path("tests/support/digest_bits.rs")
# a hasher in the file: the file computes a digest
DIGEST = re.compile(r"\bSha256\b|\bSha512\b|\bblake3::|\bBlake3\b|0xcbf2_?9ce4_?8422_?2325|0x0000_0100_0000_01b3|0x100000001b3")
TO_BITS = re.compile(r"\bto_bits\b")
LINE_COMMENT = re.compile(r"//.*")


def code_lines(text: str) -> list[tuple[int, str]]:
    """The lines with their `//` comments removed (doc and line comments)."""
    return [(n, LINE_COMMENT.sub("", line)) for n, line in enumerate(text.splitlines(), 1)]


def scan(root: Path) -> tuple[list[str], int]:
    findings: list[str] = []
    scanned = 0
    for p in sorted((root / "tests").rglob("*.rs")):
        rel = p.relative_to(root)
        if rel == HELPER:
            continue
        lines = code_lines(p.read_text(encoding="utf-8"))
        if not any(DIGEST.search(line) for _, line in lines):
            continue
        scanned += 1
        for n, line in lines:
            if TO_BITS.search(line):
                findings.append(
                    f"{rel}:{n}: `to_bits` in a digest test (read an output with digest_bits::bits32 / bits64 / "
                    f"out32 / out64, a bit-pattern input with digest_bits::pattern32 / pattern64): {line.strip()}"
                )
    return findings, scanned


def main(argv: list[str]) -> int:
    root = Path(argv[1]) if len(argv) > 1 else Path(".")
    if not (root / HELPER).is_file():
        print(f"law_bits_check: {HELPER} is missing", file=sys.stderr)
        return 2
    findings, scanned = scan(root)
    for f in findings:
        print(f)
    if scanned == 0:
        print("law_bits_check: 0 digest tests found under tests/ (scanned nothing)", file=sys.stderr)
        return 2
    print(f"law_bits_check: {scanned} digest test file(s), {len(findings)} finding(s)")
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
