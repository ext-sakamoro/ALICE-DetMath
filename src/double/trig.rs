//! `sin` / `cos` in double precision over the whole `f64` range.
//!
//! A line-by-line port of musl `src/math/{sin,cos,sincos}.c`, `__sin.c`,
//! `__cos.c`, `__rem_pio2.c` and `__rem_pio2_large.c` (themselves fdlibm
//! 5.3 with FreeBSD's fixes). Every operation is an IEEE 754 basic operation
//! in the source's order; the C `double_t` is `double` on every target this
//! crate supports, which is what Rust evaluates. Only two things differ from
//! the C source, both at the entry points:
//!
//! * `sin(±inf)` / `cos(NaN)` return the canonical NaN instead of `x - x`,
//!   whose sign is the platform's default NaN sign (`+` on aarch64 / wasm32,
//!   `−` on `x86_64`);
//! * the `FORCE_EVAL` statements that only raise floating-point exception
//!   flags are dropped (Rust does not observe the flags).

use super::{C1, C2, C3, C4, C5, C6, S1, S2, S3, S4, S5, S6};

// ---------------------------------------------------------------------------
// __sin.c / __cos.c: kernels on [-π/4, π/4] with the reduction's tail `y`
// ---------------------------------------------------------------------------

/// musl `__sin(x, y, iy)`: `sin(x + y)` for `|x + y| ≤ π/4`, `y` the tail of
/// the reduced argument; `iy == false` means `y` is 0 (not reduced).
#[inline(always)]
fn k_sin_tail(x: f64, y: f64, iy: bool) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = S2 + z * (S3 + z * S4) + z * w * (S5 + z * S6);
    let v = z * x;
    if iy {
        x - ((z * (0.5 * y - v * r) - y) - v * S1)
    } else {
        x + v * (S1 + z * r)
    }
}

/// musl `__cos(x, y)`: `cos(x + y)` for `|x + y| ≤ π/4`.
#[inline(always)]
fn k_cos_tail(x: f64, y: f64) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = z * (C1 + z * (C2 + z * C3)) + w * w * (C4 + z * (C5 + z * C6));
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    w + (((1.0 - w) - hz) + (z * r - x * y))
}

// ---------------------------------------------------------------------------
// __rem_pio2.c
// ---------------------------------------------------------------------------

/// `1.5 / DBL_EPSILON`: adding and subtracting it rounds to an integer.
const TOINT: f64 = 6_755_399_441_055_744.0;
/// π/4 (`0x1.921fb54442d18p-1`)
const PIO4: f64 = core::f64::consts::FRAC_PI_4;
/// 2/π, 53 bits
const INVPIO2: f64 = 6.366_197_723_675_813_824_33e-01;
/// first 33 bits of π/2
const PIO2_1: f64 = 1.570_796_326_734_125_614_17e+00;
/// π/2 − `PIO2_1`
const PIO2_1T: f64 = 6.077_100_506_506_192_249_32e-11;
/// second 33 bits of π/2
const PIO2_2: f64 = 6.077_100_506_303_965_976_60e-11;
/// π/2 − (`PIO2_1` + `PIO2_2`)
const PIO2_2T: f64 = 2.022_266_248_795_950_631_54e-21;
/// third 33 bits of π/2
const PIO2_3: f64 = 2.022_266_248_711_166_455_80e-21;
/// π/2 − (`PIO2_1` + `PIO2_2` + `PIO2_3`)
const PIO2_3T: f64 = 8.478_427_660_368_899_569_97e-32;

/// `n` and `(y0, y1)` with `x − n·π/2 = y0 + y1`, `|y0 + y1| ≤ π/4` (up to
/// the last bits), for finite `x` with `|x| > π/4` (musl `__rem_pio2`).
#[inline]
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let bits = x.to_bits();
    let sign = (bits >> 63) != 0;
    let ix = ((bits >> 32) & 0x7fff_ffff) as u32;

    // `|x| ~<= 9π/4` without cancellation: one to four multiples of π/2
    // subtracted with the 33 + 53 bit split, good to 85 bits.
    let small = |k: f64, n: i32| -> (i32, f64, f64) {
        if sign {
            let z = x + k * PIO2_1;
            let y0 = z + k * PIO2_1T;
            let y1 = (z - y0) + k * PIO2_1T;
            (-n, y0, y1)
        } else {
            let z = x - k * PIO2_1;
            let y0 = z - k * PIO2_1T;
            let y1 = (z - y0) - k * PIO2_1T;
            (n, y0, y1)
        }
    };

    let mut medium = false;
    if ix <= 0x400f_6a7a {
        // |x| ~<= 5π/4
        if (ix & 0xfffff) == 0x921fb {
            // |x| ~= π/2 or 2π/2: cancellation, use the medium case
            medium = true;
        } else if ix <= 0x4002_d97c {
            // |x| ~<= 3π/4
            return small(1.0, 1);
        } else {
            return small(2.0, 2);
        }
    } else if ix <= 0x401c_463b {
        // |x| ~<= 9π/4
        if ix <= 0x4015_fdbc {
            // |x| ~<= 7π/4
            if ix == 0x4012_d97c {
                // |x| ~= 3π/2
                medium = true;
            } else {
                return small(3.0, 3);
            }
        } else if ix == 0x4019_21fb {
            // |x| ~= 4π/2
            medium = true;
        } else {
            return small(4.0, 4);
        }
    } else if ix < 0x4139_21fb {
        // |x| ~< 2^20·(π/2)
        medium = true;
    }

    if medium {
        // rint(x / (π/2))
        let mut fn_ = x * INVPIO2 + TOINT - TOINT;
        let mut n = fn_ as i32;
        let mut r = x - fn_ * PIO2_1;
        let mut w = fn_ * PIO2_1T; // 1st round, good to 85 bits
                                   // Matters only with directed rounding; kept as in the source.
        if r - w < -PIO4 {
            n -= 1;
            fn_ -= 1.0;
            r = x - fn_ * PIO2_1;
            w = fn_ * PIO2_1T;
        } else if r - w > PIO4 {
            n += 1;
            fn_ += 1.0;
            r = x - fn_ * PIO2_1;
            w = fn_ * PIO2_1T;
        }
        let mut y0 = r - w;
        let ex = (ix >> 20) as i32;
        let mut ey = ((y0.to_bits() >> 52) & 0x7ff) as i32;
        if ex - ey > 16 {
            // 2nd round, good to 118 bits
            let t = r;
            w = fn_ * PIO2_2;
            r = t - w;
            w = fn_ * PIO2_2T - ((t - r) - w);
            y0 = r - w;
            ey = ((y0.to_bits() >> 52) & 0x7ff) as i32;
            if ex - ey > 49 {
                // 3rd round, good to 151 bits, covers all cases
                let t = r;
                w = fn_ * PIO2_3;
                r = t - w;
                w = fn_ * PIO2_3T - ((t - r) - w);
                y0 = r - w;
            }
        }
        let y1 = (r - y0) - w;
        return (n, y0, y1);
    }

    // All other (large) arguments. Inf / NaN never reach here (the entry
    // points return first). z = scalbn(|x|, −ilogb(x) + 23)
    let mut z = f64::from_bits((bits & (u64::MAX >> 12)) | ((0x3ff + 23) << 52));
    let mut tx = [0.0f64; 3];
    let mut i = 0;
    while i < 2 {
        tx[i] = f64::from(z as i32);
        z = (z - tx[i]) * 16_777_216.0; // 0x1p24
        i += 1;
    }
    tx[i] = z;
    // skip zero terms, the first term is non-zero (exact IEEE test, as in the source)
    #[allow(clippy::while_float)]
    while tx[i] == 0.0 {
        i -= 1;
    }
    let e0 = (ix >> 20) as i32 - (0x3ff + 23);
    let (n, ty0, ty1) = rem_pio2_large(&tx[..=i], e0);
    if sign {
        (-n, -ty0, -ty1)
    } else {
        (n, ty0, ty1)
    }
}

// ---------------------------------------------------------------------------
// __rem_pio2_large.c (prec = 1, double)
// ---------------------------------------------------------------------------

/// `jk` for double precision (`init_jk[1]`).
const JK: usize = 4;

/// 2/π in 24-bit chunks: `2/π = Σ IPIO2[i]·2^(−24(i+1))`. 66 chunks cover
/// every double exponent (`jv + jk + recompute` stays below 66).
const IPIO2: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

/// π/2 in 24-bit pieces (each has its low 32 bits zero).
const PIO2: [f64; 8] = [
    1.570_796_251_296_997_070_31e+00, // 0x3FF921FB, 0x40000000
    7.549_789_415_861_596_353_35e-08, // 0x3E74442D, 0x00000000
    5.390_302_529_957_764_765_54e-15, // 0x3CF84698, 0x80000000
    3.282_003_415_807_912_941_23e-22, // 0x3B78CC51, 0x60000000
    1.270_655_753_080_676_073_49e-29, // 0x39F01B83, 0x80000000
    1.229_333_089_811_113_289_32e-36, // 0x387A2520, 0x40000000
    2.733_700_538_164_645_596_24e-44, // 0x36E38222, 0x80000000
    2.167_416_838_778_048_194_44e-51, // 0x3569F31D, 0x00000000
];

/// `x · 2^n` (musl `scalbn`): exact except where the result is subnormal,
/// where it is rounded once.
#[inline]
fn scalbn(x: f64, mut n: i32) -> f64 {
    let mut y = x;
    if n > 1023 {
        y *= f64::from_bits(0x7fe0_0000_0000_0000); // 0x1p1023
        n -= 1023;
        if n > 1023 {
            y *= f64::from_bits(0x7fe0_0000_0000_0000);
            n -= 1023;
            if n > 1023 {
                n = 1023;
            }
        }
    } else if n < -1022 {
        // make sure the final n < -53 to avoid double rounding in the
        // subnormal range: 0x1p-1022 * 0x1p53 = 0x1p-969
        y *= f64::from_bits(0x0360_0000_0000_0000);
        n += 1022 - 53;
        if n < -1022 {
            y *= f64::from_bits(0x0360_0000_0000_0000);
            n += 1022 - 53;
            if n < -1022 {
                n = -1022;
            }
        }
    }
    y * f64::from_bits(((0x3ff + n) as u64) << 52)
}

/// `floor(x)` for `0 ≤ x < 2^63` (the only range `rem_pio2_large` needs):
/// truncation toward zero is the floor there, and exact.
#[inline]
fn floor_nonneg(x: f64) -> f64 {
    debug_assert!((0.0..9.223_372_036_854_775_808e18).contains(&x));
    (x as i64) as f64
}

/// Payne–Hanek reduction (musl `__rem_pio2_large`, `prec = 1`): `x` is the
/// input split into 24-bit chunks scaled by `2^e0`; returns `n mod 8` and
/// `x − n·π/2 = y0 + y1`.
fn rem_pio2_large(x: &[f64], e0: i32) -> (i32, f64, f64) {
    let mut iq = [0i32; 20];
    let mut f = [0.0f64; 20];
    let mut fq = [0.0f64; 20];
    let mut q = [0.0f64; 20];

    let jk = JK as i32;
    let jp = jk;

    // determine jx, jv, q0; note that 3 > q0
    let jx = x.len() as i32 - 1;
    let mut jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    let mut q0 = e0 - 24 * (jv + 1);

    // set up f[0] to f[jx+jk] where f[jx+jk] = ipio2[jv+jk]
    let m = jx + jk;
    for (j, fi) in (jv - jx..).zip(f.iter_mut().take((m + 1) as usize)) {
        *fi = if j < 0 {
            0.0
        } else {
            f64::from(IPIO2[j as usize])
        };
    }

    // compute q[0], q[1], ... q[jk]
    for i in 0..=jk {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw += x[j as usize] * f[(jx + i - j) as usize];
        }
        q[i as usize] = fw;
    }

    let mut jz = jk;
    let mut z;
    let mut n;
    let mut ih;
    loop {
        // distill q[] into iq[] reversingly
        z = q[jz as usize];
        let mut i = 0usize;
        let mut j = jz;
        while j > 0 {
            let fw = f64::from((5.960_464_477_539_062_5e-8 * z) as i32); // 0x1p-24
            iq[i] = (z - 16_777_216.0 * fw) as i32;
            z = q[(j - 1) as usize] + fw;
            i += 1;
            j -= 1;
        }

        // compute n
        z = scalbn(z, q0); // actual value of z
        z -= 8.0 * floor_nonneg(z * 0.125); // trim off integer >= 8
        n = z as i32;
        z -= f64::from(n);
        ih = 0;
        if q0 > 0 {
            // need iq[jz-1] to determine n
            let k = (jz - 1) as usize;
            let i = iq[k] >> (24 - q0);
            n += i;
            iq[k] -= i << (24 - q0);
            ih = iq[k] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            // q > 0.5
            n += 1;
            let mut carry = 0;
            for v in iq.iter_mut().take(jz as usize) {
                // compute 1 - q
                let j = *v;
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        *v = 0x100_0000 - j;
                    }
                } else {
                    *v = 0xff_ffff - j;
                }
            }
            if q0 > 0 {
                // rare case: chance is 1 in 12
                match q0 {
                    1 => iq[(jz - 1) as usize] &= 0x7f_ffff,
                    2 => iq[(jz - 1) as usize] &= 0x3f_ffff,
                    _ => {}
                }
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry != 0 {
                    z -= scalbn(1.0, q0);
                }
            }
        }

        // check if recomputation is needed
        if z == 0.0 {
            let mut j = 0;
            let mut i = jz - 1;
            while i >= jk {
                j |= iq[i as usize];
                i -= 1;
            }
            if j == 0 {
                // need recomputation: k = number of terms needed
                let mut k = 1;
                while iq[(jk - k) as usize] == 0 {
                    k += 1;
                }
                for i in (jz + 1)..=(jz + k) {
                    // add q[jz+1] to q[jz+k]
                    f[(jx + i) as usize] = f64::from(IPIO2[(jv + i) as usize]);
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw += x[j as usize] * f[(jx + i - j) as usize];
                    }
                    q[i as usize] = fw;
                }
                jz += k;
                continue;
            }
        }
        break;
    }

    // chop off zero terms
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz as usize] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        // break z into 24-bit if necessary
        z = scalbn(z, -q0);
        if z >= 16_777_216.0 {
            let fw = f64::from((5.960_464_477_539_062_5e-8 * z) as i32);
            iq[jz as usize] = (z - 16_777_216.0 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz as usize] = fw as i32;
        } else {
            iq[jz as usize] = z as i32;
        }
    }

    // convert the integer "bit" chunks to floating-point values
    let mut fw = scalbn(1.0, q0);
    let mut i = jz;
    while i >= 0 {
        q[i as usize] = fw * f64::from(iq[i as usize]);
        fw *= 5.960_464_477_539_062_5e-8;
        i -= 1;
    }

    // compute PIO2[0, ..., jp] * q[jz, ..., 0]
    let mut i = jz;
    while i >= 0 {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= jp && k <= jz - i {
            fw += PIO2[k as usize] * q[(i + k) as usize];
            k += 1;
        }
        fq[(jz - i) as usize] = fw;
        i -= 1;
    }

    // compress fq[] into y[] (prec 1)
    let mut fw = 0.0;
    let mut i = jz;
    while i >= 0 {
        fw += fq[i as usize];
        i -= 1;
    }
    let y0 = if ih == 0 { fw } else { -fw };
    let mut fw = fq[0] - fw;
    for v in fq.iter().take(jz as usize + 1).skip(1) {
        fw += *v;
    }
    let y1 = if ih == 0 { fw } else { -fw };
    (n & 7, y0, y1)
}

// ---------------------------------------------------------------------------
// sin.c / cos.c / sincos.c
// ---------------------------------------------------------------------------

/// `|x| ~< π/4` (high word of π/4).
const PIO4_HI_WORD: u32 = 0x3fe9_21fb;

#[inline(always)]
fn high_word(x: f64) -> u32 {
    ((x.to_bits() >> 32) as u32) & 0x7fff_ffff
}

/// `sin(x)` from the quadrant and the reduced argument.
#[inline(always)]
fn sin_quadrant(n: i32, y0: f64, y1: f64) -> f64 {
    match n & 3 {
        0 => k_sin_tail(y0, y1, true),
        1 => k_cos_tail(y0, y1),
        2 => -k_sin_tail(y0, y1, true),
        _ => -k_cos_tail(y0, y1),
    }
}

/// `cos(x)` from the quadrant and the reduced argument.
#[inline(always)]
fn cos_quadrant(n: i32, y0: f64, y1: f64) -> f64 {
    match n & 3 {
        0 => k_cos_tail(y0, y1),
        1 => -k_sin_tail(y0, y1, true),
        2 => -k_cos_tail(y0, y1),
        _ => k_sin_tail(y0, y1, true),
    }
}

/// Deterministic `sin(x)` in double precision over the whole range.
///
/// musl `sin.c`: fdlibm kernels, Cody–Waite reduction up to `2^20·π/2`,
/// Payne–Hanek beyond. `sin(±0) = ±0`, `sin(±inf) = sin(NaN) = NaN`
/// (canonical).
#[inline]
#[must_use]
pub fn sin64(x: f64) -> f64 {
    let ix = high_word(x);
    if ix <= PIO4_HI_WORD {
        if ix < 0x3e50_0000 {
            // |x| < 2^-26 (includes ±0 and subnormals)
            return x;
        }
        return k_sin_tail(x, 0.0, false);
    }
    if ix >= 0x7ff0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2(x);
    sin_quadrant(n, y0, y1)
}

/// Deterministic `cos(x)` in double precision over the whole range.
///
/// musl `cos.c`. `cos(±0) = 1`, `cos(±inf) = cos(NaN) = NaN` (canonical).
#[inline]
#[must_use]
pub fn cos64(x: f64) -> f64 {
    let ix = high_word(x);
    if ix <= PIO4_HI_WORD {
        if ix < 0x3e46_a09e {
            // |x| < 2^-27·√2
            return 1.0;
        }
        return k_cos_tail(x, 0.0);
    }
    if ix >= 0x7ff0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2(x);
    cos_quadrant(n, y0, y1)
}

/// `(sin64(x), cos64(x))` with one argument reduction.
///
/// Bit-identical to the two separate calls: the small-argument shortcuts are
/// those of [`sin64`] and [`cos64`].
#[inline]
#[must_use]
pub fn sin_cos64(x: f64) -> (f64, f64) {
    let ix = high_word(x);
    if ix <= PIO4_HI_WORD {
        return (sin64(x), cos64(x));
    }
    if ix >= 0x7ff0_0000 {
        return (f64::NAN, f64::NAN);
    }
    let (n, y0, y1) = rem_pio2(x);
    (sin_quadrant(n, y0, y1), cos_quadrant(n, y0, y1))
}
