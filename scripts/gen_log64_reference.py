#!/usr/bin/env python3
"""Generate tests/data/log64_reference.txt: correctly rounded ln / log2 / log10.

The three logarithms share a domain and a reduction, so one table of inputs
serves all of them and the file carries three result columns rather than three
files carrying the same inputs.

The reference is independent of every libm: each input is an exact binary
double, `mpmath` evaluates the logarithm at 2400 bits, and the result is
rounded to the nearest double (ties to even) by comparing the three candidate
doubles around it in high precision. A platform libm is not usable as the
reference even when it happens to be correctly rounded on the machine that
runs the test: whether the measured difference stays inside the documented
bound would then depend on which libm the test was linked with.

Inputs (all finite and positive; zero, negative and NaN are asserted
separately in the test): the subnormal range, which `ln64` normalises by
multiplying by 2^54; both sides of the `m >= sqrt(2)` fold that moves the
mantissa into `[sqrt(2)/2, sqrt(2))`; the neighbourhood of 1, where the
exponent and the fraction cancel; a sparse walk over the powers of two (every
one of them is checked for exactness by the test itself, which computes them
rather than reading them here); the powers of ten that are exact doubles;
`f64::MAX` and the smallest subnormal; and log-uniform values across every
binade.

Each line: `<x bits> <ln bits> <log2 bits> <log10 bits>`, 16 hex digits each.

Usage: `uv run --with mpmath python3 scripts/gen_log64_reference.py`
(or any Python with mpmath installed). The output is deterministic.
"""

from __future__ import annotations

import math
import os
import random
import struct

import mpmath

mpmath.mp.prec = 2400

# src/double.rs: the mantissa fold of ln64 / log2_64 / log10_64
SQRT2_FOLD_BITS = 0x3FF6_A09E_667F_3BCD


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


def inputs() -> list[float]:
    rng = random.Random(20261007)
    xs: list[float] = []
    # a sparse walk over the powers of two, including subnormal ones. Only a
    # sample is needed here: `tests/accuracy.rs` already asserts that *every*
    # power of two returns its exponent exactly, computed in the test rather
    # than read from this file, so carrying all 2098 of them would add ~117 KB
    # of table without adding a check.
    for k in range(-1074, 1024, 16):
        xs.append(math.ldexp(1.0, k) if k >= -1022 else from_bits(1 << (k + 1074)))
    # the powers of ten that are exact doubles, and their neighbours
    for k in range(0, 23):
        x = float(10**k)
        xs += [x, from_bits(bits(x) - 1), from_bits(bits(x) + 1)]
    # the mantissa fold, in several binades
    for e in range(-1020, 1021, 97):
        for d in (-2, -1, 0, 1, 2):
            xs.append(math.ldexp(from_bits(SQRT2_FOLD_BITS + d), e))
    # the neighbourhood of 1, where the exponent and the fraction cancel
    for d in range(-40, 41):
        xs.append(from_bits(bits(1.0) + d))
    for i in range(200):
        xs += [1.0 + 1e-3 * (i + 1) / 200, 1.0 - 1e-3 * (i + 1) / 200]
    # the subnormal range (the 2^54 pre-scale) and the top of the range
    for k in range(0, 52, 3):
        xs.append(from_bits(1 << k))
    xs += [from_bits(1), from_bits(0x000F_FFFF_FFFF_FFFF), 2.2250738585072014e-308,
           1.7976931348623157e308, from_bits(0x7FEF_FFFF_FFFF_FFFF)]
    # log-uniform over every binade
    for _ in range(900):
        e = rng.randrange(-1022, 1024)
        m = rng.randrange(1 << 52, 1 << 53)
        x = math.ldexp(m, e - 52)
        if math.isfinite(x) and x > 0:
            xs.append(x)
    out, seen = [], set()
    for x in xs:
        b = bits(x)
        if b not in seen and math.isfinite(x) and x > 0:
            seen.add(b)
            out.append(x)
    return sorted(out)


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, "tests", "data", "log64_reference.txt")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    lines = []
    for x in inputs():
        mx = mpmath.mpf(x)
        ln = round_nearest(mpmath.log(mx))
        l2 = round_nearest(mpmath.log(mx, 2))
        l10 = round_nearest(mpmath.log(mx, 10))
        lines.append(f"{bits(x):016x} {bits(ln):016x} {bits(l2):016x} {bits(l10):016x}")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("# x ln(x) log2(x) log10(x): IEEE 754 binary64 bits, correctly rounded"
                " (mpmath, 2400 bits)\n")
        f.write("# generated by scripts/gen_log64_reference.py\n")
        f.write("# Not produced by any libm: a libm that is correctly rounded on one\n")
        f.write("# machine and a ulp off on another would make the bound depend on the\n")
        f.write("# machine rather than on this crate.\n")
        f.write("\n".join(lines) + "\n")
    print(f"{len(lines)} inputs -> {path}")


if __name__ == "__main__":
    main()
