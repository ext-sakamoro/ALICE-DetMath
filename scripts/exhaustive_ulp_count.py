#!/usr/bin/env python3
"""scripts/exhaustive_ulp_count.py LOG

Reads the lines `EXHAUSTIVE_ULP <name> inputs <n> max <k> flagged <f>` in LOG (the output of
`cargo test --test exhaustive_ulp_f32 -- --ignored --nocapture`). Fails unless asin and acos
each printed exactly one line, with every f32 input of [-1, 1] and a maximum within the stated
bound, and no other function printed one; a log without lines fails as compared nothing. This is
independent of the tests' own asserts.
"""
import re
import sys
from pathlib import Path

INPUTS = 2 * (0x3F80_0000 + 1)
STATED = {"asin": 1, "acos": 1}
LINE = re.compile(r"EXHAUSTIVE_ULP (\w+) inputs (\d+) max (\d+) flagged (\d+)\b")


def check(log: str) -> list[str]:
    seen: dict[str, tuple[int, int]] = {}
    errors = []
    for m in LINE.finditer(log):
        name, n, k = m.group(1), int(m.group(2)), int(m.group(3))
        if name in seen:
            errors.append(f"{name}: printed twice")
        seen[name] = (n, k)
    if not seen:
        return ["no EXHAUSTIVE_ULP line (compared nothing)"]
    for name in sorted(set(seen) - set(STATED)):
        errors.append(f"{name}: not a function this check knows")
    for name, bound in STATED.items():
        if name not in seen:
            errors.append(f"{name}: no line")
            continue
        n, k = seen[name]
        if n != INPUTS:
            errors.append(f"{name}: {n} inputs, not {INPUTS}")
        if k > bound:
            errors.append(f"{name}: max {k} steps, over the stated {bound}")
    return errors


def main() -> int:
    errors = check(Path(sys.argv[1]).read_text(encoding="utf-8"))
    for e in errors:
        print(f"exhaustive-ulp: {e}")
    print(f"exhaustive-ulp: {'FAIL' if errors else 'ok'} ({len(STATED)} functions)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
