//! `f32` kernels: Cephes `sinf` / `cosf` / `expf`, musl `logf`, plus
//! `powf` / `powi` / `cbrt` / `hypot`, and the `f32` entry points of the
//! double-precision fdlibm kernels in [`crate::double`].

use crate::double::{
    acos64, asin64, atan2_64, atan64, exp64, k_cos64, k_sin64, ln64, reduce_pio2_64,
};
use crate::ops::{round, sqrt};

/// π/2 split into three single-precision pieces (Cephes `DP1..3` × 2) so
/// `x - k·π/2` is computed with ~24 extra bits for `|k| < 2^24`.
pub(crate) const PIO2_1: f32 = 1.570_312_5;
pub(crate) const PIO2_2: f32 = 4.837_512_969_970_703_125e-4;
pub(crate) const PIO2_3: f32 = 7.549_789_948_768_648e-8;
pub(crate) const FRAC_2_PI: f32 = core::f32::consts::FRAC_2_PI;

/// Cephes `sinf` polynomial coefficients on `[-π/4, π/4]`.
pub(crate) const SIN_P: [f32; 3] = [-1.951_529_589_1e-4, 8.332_160_873_6e-3, -1.666_665_461_1e-1];
/// Cephes `cosf` polynomial coefficients on `[-π/4, π/4]`.
pub(crate) const COS_P: [f32; 3] = [
    2.443_315_711_809_948e-5,
    -1.388_731_625_493_765e-3,
    4.166_664_568_298_827e-2,
];

/// Sine polynomial on `[-π/4, π/4]` (Cephes `sinf`).
#[inline(always)]
fn sin_poly(r: f32) -> f32 {
    let z = r * r;
    let p = ((SIN_P[0] * z + SIN_P[1]) * z + SIN_P[2]) * z * r;
    r + p
}

/// Cosine polynomial on `[-π/4, π/4]` (Cephes `cosf`).
#[inline(always)]
fn cos_poly(r: f32) -> f32 {
    let z = r * r;
    let p = ((COS_P[0] * z + COS_P[1]) * z + COS_P[2]) * z * z;
    1.0 - 0.5 * z + p
}

/// Reduce `x` to `(k mod 4, r)` with `x = k·π/2 + r`, `|r| ≤ π/4`.
#[inline(always)]
fn reduce_pio2(x: f32) -> (i32, f32) {
    let kf = round(x * FRAC_2_PI);
    let r = ((x - kf * PIO2_1) - kf * PIO2_2) - kf * PIO2_3;
    // `kf` is integral and |kf| < 2^31 for every finite f32 we accept; the
    // cast saturates for huge inputs, which only affects the (already
    // meaningless) quadrant of astronomically large arguments.
    let k = (kf as i32) & 3;
    (k, r)
}

/// Deterministic `sin(x)`.
#[must_use]
pub fn sin(x: f32) -> f32 {
    if !x.is_finite() {
        return f32::NAN;
    }
    let (k, r) = reduce_pio2(x);
    match k {
        0 => sin_poly(r),
        1 => cos_poly(r),
        2 => -sin_poly(r),
        _ => -cos_poly(r),
    }
}

/// Deterministic `cos(x)`.
#[must_use]
pub fn cos(x: f32) -> f32 {
    if !x.is_finite() {
        return f32::NAN;
    }
    let (k, r) = reduce_pio2(x);
    match k {
        0 => cos_poly(r),
        1 => -sin_poly(r),
        2 => -cos_poly(r),
        _ => sin_poly(r),
    }
}

/// Deterministic `(sin(x), cos(x))` from one range reduction; bit-identical
/// to calling [`sin`] and [`cos`] separately.
#[must_use]
pub fn sin_cos(x: f32) -> (f32, f32) {
    if !x.is_finite() {
        return (f32::NAN, f32::NAN);
    }
    let (k, r) = reduce_pio2(x);
    let s = sin_poly(r);
    let c = cos_poly(r);
    match k {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

pub(crate) const LOG2E: f32 = core::f32::consts::LOG2_E;
pub(crate) const LN2_HI: f32 = 0.693_145_751_953_125;
pub(crate) const LN2_LO: f32 = 1.428_606_765_330_187e-6;
/// Cephes `expf` polynomial for `e^r`, `|r| ≤ ln2/2` (highest degree first).
pub(crate) const EXP_P: [f32; 5] = [
    1.987_569_150_0e-4,
    1.398_199_950_7e-3,
    8.333_451_907_3e-3,
    4.166_579_589_4e-2,
    1.666_666_545_9e-1,
];
/// `exp` overflows to `+inf` above this and underflows to `0` below [`EXP_LO`].
pub(crate) const EXP_HI: f32 = 88.722_84;
/// See [`EXP_HI`].
pub(crate) const EXP_LO: f32 = -103.972_08;

/// Build `2^k` for `-126 ≤ k ≤ 127` directly from the exponent bits.
#[inline(always)]
fn pow2i(k: i32) -> f32 {
    debug_assert!((-126..=127).contains(&k));
    f32::from_bits(((k + 127) as u32) << 23)
}

/// Deterministic `exp(x)`.
///
/// Overflows to `+inf` above ≈ 88.72, underflows to `0.0` below ≈ −103.97
/// (subnormal results are produced via a two-step scale, so the tail is
/// gradual, not a cliff).
#[must_use]
pub fn exp(x: f32) -> f32 {
    if x.is_nan() {
        return f32::NAN;
    }
    if x > EXP_HI {
        return f32::INFINITY;
    }
    if x < EXP_LO {
        return 0.0;
    }
    let kf = round(x * LOG2E);
    let k = kf as i32;
    let r = (x - kf * LN2_HI) - kf * LN2_LO;
    let p = (((((EXP_P[0] * r + EXP_P[1]) * r + EXP_P[2]) * r + EXP_P[3]) * r + EXP_P[4]) * r
        + 0.5)
        * r
        * r
        + r
        + 1.0;
    // Scale by 2^k. Split the scale when k leaves the normal exponent range
    // so subnormal results and the overflow edge are handled without UB.
    if k > 127 {
        p * pow2i(127) * pow2i(k - 127)
    } else if k < -126 {
        p * pow2i(-126) * pow2i(k + 126)
    } else {
        p * pow2i(k)
    }
}

// musl `logf` coefficients.
pub(crate) const LG1: f32 = 0.666_666_626_93;
pub(crate) const LG2: f32 = 0.400_009_721_52;
pub(crate) const LG3: f32 = 0.284_987_866_88;
pub(crate) const LG4: f32 = 0.242_790_788_41;
/// Mantissa bit pattern of `sqrt(2)` used to fold `m` into `[√2/2, √2)`.
pub(crate) const SQRT2_BITS: u32 = 0x3fb5_04f3;

/// Deterministic natural logarithm `ln(x)`.
///
/// `ln(0) = -inf`, `ln(x < 0) = NaN`, `ln(inf) = inf`.
#[must_use]
pub fn ln(x: f32) -> f32 {
    if x.is_nan() || x < 0.0 {
        return f32::NAN;
    }
    if x == 0.0 {
        return f32::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f32::INFINITY;
    }
    // Normalise subnormals so the exponent extraction below is exact.
    let mut bits = x.to_bits();
    let mut k: i32 = 0;
    if bits < 0x0080_0000 {
        let scaled = x * 33_554_432.0; // 2^25
        bits = scaled.to_bits();
        k -= 25;
    }
    // Reduce mantissa to [sqrt(2)/2, sqrt(2)).
    k += ((bits >> 23) as i32) - 127;
    bits = (bits & 0x007f_ffff) | 0x3f80_0000; // m in [1, 2)
    if bits >= SQRT2_BITS {
        // m >= sqrt(2): halve it
        bits -= 0x0080_0000;
        k += 1;
    }
    let m = f32::from_bits(bits);
    let f = m - 1.0;
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (LG2 + w * LG4);
    let t2 = z * (LG1 + w * LG3);
    let r = t2 + t1;
    let hfsq = 0.5 * f * f;
    let kf = k as f32;
    kf * LN2_HI - ((hfsq - (s * (hfsq + r) + kf * LN2_LO)) - f)
}

/// Deterministic `x^y` for real `y`.
///
/// Evaluated as `exp64(y · ln64 x)` in double precision and rounded once to
/// `f32`, so the argument of the exponential carries ~29 spare bits and the
/// result is within 1 ulp of the correctly rounded value over the measured
/// domain. Special cases follow IEEE `pow` for the inputs the engine uses:
/// `x < 0 → NaN` (non-integer `y` is the common case here, so no
/// integer-exponent sign rule is attempted), `x == 0 → 0 / 1 / inf` for
/// `y > 0 / y == 0 / y < 0`, `y == 0 → 1`.
#[must_use]
pub fn powf(x: f32, y: f32) -> f32 {
    if y == 0.0 {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() || x < 0.0 {
        return f32::NAN;
    }
    if x == 0.0 {
        return if y > 0.0 { 0.0 } else { f32::INFINITY };
    }
    exp64(f64::from(y) * ln64(f64::from(x))) as f32
}

/// Deterministic integer power by repeated squaring (`n` may be negative).
///
/// Unlike `f32::powi`, which lowers to a compiler-rt routine, the
/// multiplication order here is fixed by this source.
#[must_use]
pub fn powi(x: f32, n: i32) -> f32 {
    if n == 0 {
        return 1.0;
    }
    if x.is_nan() {
        // canonical NaN: propagating the input payload through the
        // multiplications below is platform-dependent
        return f32::NAN;
    }
    let mut base = if n < 0 { 1.0 / x } else { x };
    let mut e = n.unsigned_abs();
    let mut acc = 1.0f32;
    while e > 0 {
        if e & 1 == 1 {
            acc *= base;
        }
        e >>= 1;
        if e > 0 {
            base *= base;
        }
    }
    acc
}

/// Deterministic cube root.
///
/// Bit-hack initial estimate (`bits / 3 + 0x2a51_37a0`) followed by three
/// Newton steps, then the sign is restored. `±0`, `±inf` and `NaN` pass
/// through.
#[must_use]
pub fn cbrt(x: f32) -> f32 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let ax = x.abs();
    // Subnormals: scale up by 2^24 (an exact cube-friendly power, 2^(3·8)),
    // take the root, scale back by 2^8.
    let (ax, post) = if ax.to_bits() < 0x0080_0000 {
        (ax * 16_777_216.0, 1.0 / 256.0)
    } else {
        (ax, 1.0)
    };
    let mut y = f32::from_bits(ax.to_bits() / 3 + 0x2a51_37a0);
    for _ in 0..3 {
        let y2 = y * y;
        y -= (y2 * y - ax) / (3.0 * y2);
    }
    let y = y * post;
    if x < 0.0 {
        -y
    } else {
        y
    }
}

/// Deterministic `sqrt(x² + y²)` with power-of-two scaling so intermediate
/// squares neither overflow nor flush to zero.
#[must_use]
pub fn hypot(x: f32, y: f32) -> f32 {
    let ax = x.abs();
    let ay = y.abs();
    if ax.is_infinite() || ay.is_infinite() {
        return f32::INFINITY;
    }
    if ax.is_nan() || ay.is_nan() {
        return f32::NAN;
    }
    let m = if ax > ay { ax } else { ay };
    if m == 0.0 {
        return 0.0;
    }
    // Scaling by powers of two is exact, so the result is bit-identical to
    // the unscaled formula whenever that formula does not over/underflow.
    let (scale, unscale) = if m > 1.0e18 {
        (
            1.0 / 18_446_744_073_709_551_616.0,
            18_446_744_073_709_551_616.0,
        ) // 2^-64, 2^64
    } else if m < 1.0e-18 {
        (
            18_446_744_073_709_551_616.0,
            1.0 / 18_446_744_073_709_551_616.0,
        )
    } else {
        (1.0, 1.0)
    };
    let sx = ax * scale;
    let sy = ay * scale;
    sqrt(sx * sx + sy * sy) * unscale
}

// ---------------------------------------------------------------------------
// f32 inverse trigonometric / tan / tanh
//
// Evaluated in double precision with fdlibm's algorithms (basic IEEE
// operations only) and rounded once to `f32`, so the `f32` result is within
// 1 ulp of correctly rounded over the measured domains (see tests).
// ---------------------------------------------------------------------------

/// Deterministic `atan(x)`.
#[must_use]
pub fn atan(x: f32) -> f32 {
    atan64(f64::from(x)) as f32
}

/// Deterministic `atan2(y, x)` in `(-π, π]`, IEEE special cases as fdlibm.
#[must_use]
pub fn atan2(y: f32, x: f32) -> f32 {
    atan2_64(f64::from(y), f64::from(x)) as f32
}

/// Deterministic `asin(x)`; `NaN` outside `[-1, 1]`.
#[must_use]
pub fn asin(x: f32) -> f32 {
    asin64(f64::from(x)) as f32
}

/// Deterministic `acos(x)`; `NaN` outside `[-1, 1]`.
#[must_use]
pub fn acos(x: f32) -> f32 {
    acos64(f64::from(x)) as f32
}

/// Deterministic `tan(x)` (`sin/cos` of the double-precision kernels).
///
/// Argument reduction is exact for `|x| < 2^20·π/2`; beyond that the result
/// is still deterministic but loses accuracy (as `sin` / `cos` do).
#[must_use]
pub fn tan(x: f32) -> f32 {
    if !x.is_finite() {
        return f32::NAN;
    }
    let (k, r) = reduce_pio2_64(f64::from(x));
    let (s, c) = (k_sin64(r), k_cos64(r));
    // tan(x + kπ/2): even k → sin/cos, odd k → -cos/sin
    let t = if k & 1 == 0 { s / c } else { -c / s };
    t as f32
}

/// Deterministic `tanh(x)`.
///
/// `|x| < 2^-14 → x` (the cubic term is below half an `f32` ulp), otherwise
/// `(e^{2|x|} − 1) / (e^{2|x|} + 1)` in double precision via [`exp64`].
#[must_use]
pub fn tanh(x: f32) -> f32 {
    if x.is_nan() {
        return f32::NAN;
    }
    let ax = x.abs();
    if ax < 6.103_515_625e-5 {
        return x;
    }
    if ax > 10.0 {
        // 1 - 2e^{-20} rounds to 1.0 in f32
        return if x < 0.0 { -1.0 } else { 1.0 };
    }
    let e = exp64(2.0 * f64::from(ax));
    let t = ((e - 1.0) / (e + 1.0)) as f32;
    if x < 0.0 {
        -t
    } else {
        t
    }
}
