#!/usr/bin/env python3
"""Generate tests/data/powf64_reference.txt: correctly rounded x^y.

The reference is independent of every libm: each input pair is a pair of exact
binary doubles, `mpmath` evaluates the power at 2400 bits, and the result is
rounded to the nearest double (ties to even) by comparing the three candidate
doubles around it in high precision. A platform libm is not usable as the
reference even when it happens to be correctly rounded on the machine that
runs the test: whether the measured difference stays inside the documented
bound would then depend on which libm the test was linked with.

`powf64` is fdlibm `e_pow.c`, which carries `log2 x` and `y·log2 x` in
simulated extra precision, so its bound is 1 ulp over every pair of finite
arguments. The table therefore covers the whole range, not only the region
the earlier `exp64(y·ln64 x)` evaluation was accurate in.

Inputs: `x in [1e-3, 1e3]`, `|y| <= 8` on a grid; `x` within a few ulp of 1
with `|y|` up to 1e300 (where the earlier evaluation doubled the exponent:
`x = 1 - 1 ulp`, `y = 1e15` was 8.9e14 ulp off); integer, half-integer and
tiny exponents; negative bases with integer exponents (the sign from the
parity); results next to the overflow threshold and in the subnormal range;
and a log-uniform sweep of `x` over `[1e-300, 1e300]` with `y` chosen so that
the result is finite. Results that round to 0 or overflow are left out (the
test asserts those edges by value).

Each line: `<x bits> <y bits> <pow(x, y) bits>`, 16 hex digits each.

Usage: `uv run --with mpmath python3 scripts/gen_powf64_reference.py`
(or any Python with mpmath installed). The output is deterministic.
"""

from __future__ import annotations

import math
import os
import random
import struct

import mpmath

mpmath.mp.prec = 2400

def bits(x: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", x))[0]


def from_bits(b: int) -> float:
    return struct.unpack("<d", struct.pack("<Q", b))[0]


def round_nearest(v: mpmath.mpf) -> float:
    """The double nearest to v (ties to even), decided in high precision."""
    c = float(v)
    best = None
    for cand in (math.nextafter(c, -math.inf), c, math.nextafter(c, math.inf)):
        d = abs(mpmath.mpf(cand) - v)
        key = (d, bits(cand) & 1)  # on an exact tie, the even mantissa wins
        if best is None or key < best[0]:
            best = (key, cand)
    return best[1]


X_LO, X_HI, Y_ABS = 1e-3, 1e3, 8.0


def pairs() -> list[tuple[float, float]]:
    rng = random.Random(20261007)
    out: list[tuple[float, float]] = []
    # the documented domain on a grid (coarser than the old 401 x 161 sweep of
    # 64561 points; the hard cases below carry the rest)
    for i in range(49):
        x = 10.0 ** (-3.0 + 6.0 * i / 48)
        for j in range(41):
            out.append((x, -Y_ABS + 2.0 * Y_ABS * j / 40))
    # x within a few ulp of 1, where y·ln x is tiny and the low half of the
    # double-double decides the result
    for d in list(range(-16, 17)):
        x = from_bits(bits(1.0) + d)
        for y in (-Y_ABS, -1.0, -0.5, 0.5, 1.0, Y_ABS):
            out.append((x, y))
    # integer and half-integer exponents, and tiny exponents of both signs
    for x in (0.001, 0.1, 0.5, 1.5, 2.0, 3.0, 7.0, 10.0, 123.456, 1000.0):
        for y in [float(k) for k in range(-8, 9)] + [0.5, -0.5, 1.5, -1.5, 2.5, -2.5]:
            out.append((x, y))
        for e in range(1, 60, 2):
            out.append((x, math.ldexp(1.0, -e)))
            out.append((x, -math.ldexp(1.0, -e)))
    # log-uniform inside [1e-3, 1e3] x [-8, 8]
    for _ in range(700):
        out.append((10.0 ** rng.uniform(-3, 3), rng.uniform(-Y_ABS, Y_ABS)))
    # x next to 1 with a large |y|: y·ln x must not lose its low half
    for d in list(range(-16, 17)):
        x = from_bits(bits(1.0) + d)
        for y in (1e3, 1e6, 1e10, 1e15, 2.0 ** 52, 1e16, 1e17, 1e18):
            out.append((x, y))
            out.append((x, -y))
    # negative bases with integer exponents: the sign from the parity
    for x in (-0.001, -0.1, -0.5, -0.75, -1.5, -2.0, -3.0, -7.0, -10.0, -123.456, -1000.0):
        for k in list(range(-40, 41)) + [101, 1001, -1001, 1000, -1000]:
            out.append((x, float(k)))
    for d in list(range(-8, 9)):
        x = -from_bits(bits(1.0) + d)
        for y in (2.0 ** 40 + 1, 2.0 ** 40, 2.0 ** 52 + 1, 2.0 ** 52, 2.0 ** 53, 1e18):
            out.append((x, y))
            out.append((x, -y))
    # results next to overflow (2^1024) and in the subnormal range
    for x in (2.0, 3.0, 10.0, 1.5, 0.5, 0.1):
        lg = math.log2(x)
        for target in (1023.9, 1023.99, 1023.999, -1022.5, -1030.0, -1050.0, -1070.0, -1074.0):
            y = target / lg
            out.append((x, y))
            out.append((x, math.nextafter(y, math.inf)))
            out.append((x, math.nextafter(y, -math.inf)))
    # log-uniform x over the whole range, y chosen so the result is finite
    for _ in range(1500):
        x = 10.0 ** rng.uniform(-300, 300)
        lg = abs(math.log2(x)) or 1.0
        out.append((x, rng.uniform(-1000.0, 1000.0) / lg))
    # subnormal bases
    for b in (1, 2, 3, 1 << 20, (1 << 52) - 1):
        x = from_bits(b)
        for y in (0.5, 0.25, 0.75, 1.0 / 3.0, 0.9, 0.99):
            out.append((x, y))
    seen, uniq = set(), []
    for x, y in out:
        k = (bits(x), bits(y))
        if k in seen or not (math.isfinite(x) and math.isfinite(y)):
            continue
        seen.add(k)
        uniq.append((x, y))
    if not uniq:
        raise SystemExit("no pairs")
    return sorted(uniq, key=lambda p: (bits(p[0]), bits(p[1])))


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, "tests", "data", "powf64_reference.txt")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    lines = []
    for x, y in pairs():
        v = round_nearest(mpmath.power(mpmath.mpf(x), mpmath.mpf(y)))
        if not math.isfinite(v) or v == 0.0:
            continue  # the test asserts the overflow / underflow edges by value
        lines.append(f"{bits(x):016x} {bits(y):016x} {bits(v):016x}")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("# x y pow(x, y): IEEE 754 binary64 bits, correctly rounded"
                " (mpmath, 2400 bits)\n")
        f.write("# generated by scripts/gen_powf64_reference.py\n")
        f.write("# Not produced by any libm: a libm that is correctly rounded on one\n")
        f.write("# machine and a ulp off on another would make the bound depend on the\n")
        f.write("# machine rather than on this crate.\n")
        f.write("\n".join(lines) + "\n")
    print(f"{len(lines)} pairs -> {path}")


if __name__ == "__main__":
    main()
