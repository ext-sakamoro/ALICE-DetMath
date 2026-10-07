#!/usr/bin/env python3
"""Generate tests/data/sin_cos64_reference.txt: correctly rounded sin / cos.

The reference is independent of every libm: each input is an exact binary
double, `mpmath` evaluates sin and cos at 2400 bits (enough to reduce any
finite double modulo pi/2 with more than 1300 bits to spare), and the result
is rounded to the nearest double (ties to even) by comparing the three
candidate doubles around it in high precision.

Inputs (all finite; the special values are asserted in the test itself):
small and medium arguments, every branch threshold of the fdlibm reduction,
near multiples of pi/2, the hardest argument-reduction cases per binade
(continued-fraction convergents of 2^(e+1)/pi), and log-uniform huge values up
to f64::MAX.

Each line: `<x bits> <sin bits> <cos bits>`, 16 hex digits each.

Usage: `uv run --with mpmath python3 scripts/gen_sin_cos64_reference.py`
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


def hard_cases() -> list[float]:
    """Per binade, the double q·2^e closest to a multiple of pi/2 (q < 2^53)."""
    out = []
    for e in range(-60, 972, 3):
        alpha = mpmath.mpf(2) ** (e + 1) / mpmath.pi  # x·2/pi for x = q·2^e is q·alpha
        frac = alpha - mpmath.floor(alpha)
        # convergents of frac: q_k is the denominator
        a = frac
        q_prev, q = 0, 1
        best = None
        for _ in range(200):
            if a == 0:
                break
            a = 1 / a
            ai = int(mpmath.floor(a))
            a -= ai
            q_prev, q = q, ai * q + q_prev
            if q >= 2**53:
                break
            best = q
        if best is not None and best > 1:
            x = float(mpmath.mpf(best) * mpmath.mpf(2) ** e)
            if math.isfinite(x) and x > 0:
                out.append(x)
    return out


def inputs() -> list[float]:
    rng = random.Random(20261007)
    xs: list[float] = []
    # tiny, subnormal, and the |x| < 2^-26 / 2^-27·sqrt(2) shortcut boundaries
    xs += [5e-324, 2.2250738585072009e-308, 2.2250738585072014e-308, 1e-300, 1e-200, 1e-20]
    for b in (0x3E400000_00000000, 0x3E46A09E_00000000, 0x3E500000_00000000, 0x3FE921FB_00000000,
              0x3FE921FB_54442D18):
        for d in (-2, -1, 0, 1, 2):
            xs.append(from_bits(b + d))
    # every high-word threshold of __rem_pio2 and its neighbours
    for hi in (0x400F6A7A, 0x4002D97C, 0x401C463B, 0x4015FDBC, 0x4012D97C, 0x401921FB,
               0x413921FB, 0x3FF921FB, 0x400921FB):
        for lo in (0, 1, 0x54442D18, 0xFFFFFFFF):
            xs.append(from_bits((hi << 32) | lo))
    # near multiples of pi/2 (the double nearest k·pi/2 and ±2 ulp)
    for k in list(range(1, 65)) + [rng.randrange(65, 1 << 21) for _ in range(150)] + \
            [rng.randrange(1 << 21, 1 << 40) for _ in range(50)]:
        c = float(k * mpmath.pi / 2)
        for d in (-2, -1, 0, 1, 2):
            xs.append(from_bits(bits(c) + d))
    xs += [1.5707963267948966, 3.141592653589793, 4.71238898038469, 6.283185307179586,
           1e5, 1e6, 1e9, 1e15, 1e22, 1e100, 1e200, 1e300, 1.7976931348623157e308,
           8.98846567431158e307, 6381956970095103 * 2.0**797, 2.0**1023, 2.0**60, 2.0**63 * 3]
    xs += hard_cases()
    # uniform small / medium
    xs += [rng.uniform(-4.0, 4.0) for _ in range(200)]
    xs += [rng.uniform(-1e5, 1e5) for _ in range(200)]
    xs += [rng.uniform(-1.65e6, 1.65e6) for _ in range(200)]
    # log-uniform over every binade up to f64::MAX (the Payne-Hanek path)
    for _ in range(800):
        e = rng.randrange(-1022, 1024)
        m = rng.randrange(1 << 52, 1 << 53)
        x = math.ldexp(m, e - 52)
        if math.isfinite(x):
            xs.append(x)
    # both signs of everything
    out, seen = [], set()
    for x in xs:
        for y in (x, -x):
            b = bits(y)
            if b not in seen and math.isfinite(y):
                seen.add(b)
                out.append(y)
    return out


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, "tests", "data", "sin_cos64_reference.txt")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    xs = inputs()
    lines = []
    for x in xs:
        if x == 0.0:
            continue  # ±0 are asserted bitwise in the test
        mx = mpmath.mpf(x)
        s = round_nearest(mpmath.sin(mx))
        c = round_nearest(mpmath.cos(mx))
        lines.append(f"{bits(x):016x} {bits(s):016x} {bits(c):016x}")
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("# x sin(x) cos(x): IEEE 754 binary64 bits, correctly rounded (mpmath, 2400 bits)\n")
        f.write("# generated by scripts/gen_sin_cos64_reference.py\n")
        f.write("\n".join(lines) + "\n")
    print(f"{len(lines)} inputs -> {path}")


if __name__ == "__main__":
    main()
