#!/usr/bin/env python3
"""Generate tests/data/powf64_reference.txt: correctly rounded x^y.

The reference is independent of every libm: each input pair is a pair of exact
binary doubles, `mpmath` evaluates the power at 2400 bits, and the result is
rounded to the nearest double (ties to even) by comparing the three candidate
doubles around it in high precision. A platform libm is not usable as the
reference even when it happens to be correctly rounded on the machine that
runs the test: whether the measured difference stays inside the documented
bound would then depend on which libm the test was linked with.

`powf64` evaluates `exp64(y·ln64 x)` with `y·ln64 x` carried as a
double-double, so its error budget (16 ulp, measured 13) is far wider than the
other functions'. That makes the reference's own accuracy less critical, but
also makes a drifting bound harder to notice, which is the reason to pin it
against the true value rather than against whatever libm is linked in.

Inputs: the documented domain `x in [1e-3, 1e3]`, `|y| <= 8` — the same region
the accuracy test measured before this table existed — on a grid, plus the
in-domain cases where the double-double argument matters: `x` within a few ulp
of 1, integer and half-integer `y`, and `y` of either sign near zero.

Deliberately NOT covered, and why: `|y|` far above the documented bound with
`x` within one ulp of 1. There the residual step `l_lo = x*exp(-l_hi) - 1`
degenerates, because `exp64(-l_hi)` rounds to exactly 1 and the expression
collapses to `x - 1`, which is the whole logarithm rather than its residual;
`t_lo += y*l_lo` then adds a second copy of `y*ln x` and the effective exponent
is doubled. Measured at `x = 1 - 1 ulp`, `y = 1e15`: 8.9e14 ulp, with
`ln(ours)/ln(true) = 2.06`. Inside the documented domain the doubled term is
below half an ulp of the result, so the contract still holds. Widening the
domain is a change to `src/`, so these inputs stay out of this table until
that is decided rather than being folded in with a relaxed bound.

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

# src/double.rs::powf64 forwards these thresholds to exp64
OVERFLOW = 709.782_712_893_384
UNDERFLOW = -745.133_219_101_941_1


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


def in_domain(x: float, y: float) -> bool:
    return X_LO <= x <= X_HI and abs(y) <= Y_ABS


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
    # log-uniform inside the domain
    for _ in range(700):
        out.append((10.0 ** rng.uniform(-3, 3), rng.uniform(-Y_ABS, Y_ABS)))
    seen, uniq = set(), []
    for x, y in out:
        k = (bits(x), bits(y))
        if k in seen or not (math.isfinite(x) and math.isfinite(y)):
            continue
        if not in_domain(x, y):
            continue  # see the note at the top of this file
        seen.add(k)
        uniq.append((x, y))
    if not uniq:
        raise SystemExit("no pairs inside the documented domain")
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
