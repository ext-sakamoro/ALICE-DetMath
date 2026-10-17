#!/usr/bin/env python3
"""scripts/exhaustive_count.py LOG NAME[,NAME...]

Fails unless LOG (the output of `cargo test --test exhaustive_f32 -- --ignored --nocapture`)
has, for every NAME, exactly one line `EXHAUSTIVE <NAME> inputs swept 4294967296 digest
<16 hex>`, and no such line for a name not given. A run that swept fewer inputs, skipped a
function or printed nothing is red: a test that did not run reports no failure.
"""
import re
import sys
from pathlib import Path

ALL_INPUTS = 1 << 32
LINE = re.compile(r"EXHAUSTIVE (\w+) inputs swept (\d+) digest ([0-9a-f]{16})")


def check(text: str, names: list[str]) -> list[str]:
    errors = []
    seen: dict[str, list[int]] = {}
    for m in LINE.finditer(text):
        seen.setdefault(m.group(1), []).append(int(m.group(2)))
    if not seen:
        return ["no EXHAUSTIVE line in the log (0 functions swept)"]
    for name in names:
        counts = seen.get(name, [])
        if len(counts) != 1:
            errors.append(f"{name}: {len(counts)} EXHAUSTIVE lines, expected 1")
        elif counts[0] != ALL_INPUTS:
            errors.append(f"{name}: inputs swept {counts[0]}, expected {ALL_INPUTS}")
    for name in sorted(set(seen) - set(names)):
        errors.append(f"{name}: swept but not expected")
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 3 or not argv[2]:
        print(__doc__, file=sys.stderr)
        return 2
    names = argv[2].split(",")
    text = Path(argv[1]).read_text(encoding="utf-8", errors="replace")
    errors = check(text, names)
    for e in errors:
        print(f"exhaustive_count: {e}", file=sys.stderr)
    if errors:
        return 1
    print(f"exhaustive_count: {len(names)} functions, {ALL_INPUTS} inputs swept each")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
