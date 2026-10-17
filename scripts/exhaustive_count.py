#!/usr/bin/env python3
"""scripts/exhaustive_count.py LOG DIGESTS

Compares the lines `EXHAUSTIVE <name> inputs swept <n> digest <16 hex>` in LOG (the output
of `cargo test --test exhaustive_f32 -- --ignored --nocapture`) with DIGESTS
(tests/exhaustive_f32.digests: `<name> <16 hex>` per line, `#` comments). Fails unless
every function in DIGESTS printed exactly one line, with 2^32 inputs swept and its pinned
digest, and no other function printed one; an empty DIGESTS or a log without lines fails as
compared nothing. This is independent of the tests' own asserts: a test whose failure was
lost (a step status taken from tee, a filtered or skipped test) cannot pass it.
"""
import re
import sys
from pathlib import Path

ALL_INPUTS = 1 << 32
LINE = re.compile(r"EXHAUSTIVE (\w+) inputs swept (\d+) digest ([0-9a-f]{16})\b")
PIN = re.compile(r"^(\w+) ([0-9a-f]{16})$")


def read_pins(text: str) -> tuple[dict[str, str], list[str]]:
    pins: dict[str, str] = {}
    errors = []
    for i, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        m = PIN.match(line)
        if not m:
            errors.append(f"digests line {i}: '{raw}' is not '<name> <16 hex>'")
        elif m.group(1) in pins:
            errors.append(f"digests line {i}: {m.group(1)} pinned twice")
        else:
            pins[m.group(1)] = m.group(2)
    if not pins and not errors:
        errors.append("no pinned digest (compared nothing)")
    return pins, errors


def check(log: str, pins: dict[str, str]) -> list[str]:
    seen: dict[str, list[tuple[int, str]]] = {}
    for m in LINE.finditer(log):
        seen.setdefault(m.group(1), []).append((int(m.group(2)), m.group(3)))
    if not seen:
        return ["no EXHAUSTIVE line in the log (compared nothing)"]
    errors = []
    for name, want in pins.items():
        got = seen.get(name, [])
        if len(got) != 1:
            errors.append(f"{name}: {len(got)} EXHAUSTIVE lines, expected 1")
            continue
        n, digest = got[0]
        if n != ALL_INPUTS:
            errors.append(f"{name}: inputs swept {n}, expected {ALL_INPUTS}")
        if digest != want:
            errors.append(f"{name}: digest {digest}, pinned {want}")
    for name in sorted(set(seen) - set(pins)):
        errors.append(f"{name}: swept but has no pin")
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    pins, errors = read_pins(Path(argv[2]).read_text(encoding="utf-8"))
    if not errors:
        errors = check(Path(argv[1]).read_text(encoding="utf-8", errors="replace"), pins)
    for e in errors:
        print(f"exhaustive_count: {e}", file=sys.stderr)
    if errors:
        return 1
    print(f"exhaustive_count: {len(pins)} functions, {ALL_INPUTS} inputs swept each, "
          "every digest equal to its pin")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
