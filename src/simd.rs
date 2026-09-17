//! `f32x8` versions of the `f32` kernels (feature `simd`).
//!
//! Every function here is lane-for-lane **bit-identical** to its scalar
//! counterpart in the crate root: the same constants, the same operation
//! order, no `mul_add` (which would fuse on FMA targets and not on others),
//! and every branch of the scalar code replaced by a mask + blend over
//! values that were computed the same way. `tests/simd_parity.rs` pins this
//! over dense sweeps and the special values.
//!
//! The kernels are written with `f32` arithmetic, compares + `blend`, lane
//! shifts and `f32 ↔ i32` conversions only. Where the scalar code
//! manipulates bits, the SIMD code uses an *exactly equivalent* float
//! operation: scaling by a power of two instead of masking the exponent,
//! `-0.0 - x` instead of flipping the sign bit, `kf - 4·floor(kf/4)` instead
//! of `k & 3`. (`wide` 0.7 has no NEON path for integer / bitwise lane ops,
//! which run 5–10× slower than the float ops there.)
//!
//! [`sin`], [`cos`], [`sin_cos`], [`exp`], [`ln`], [`round`] and [`sqrt`]
//! are vectorised. The functions whose scalar version evaluates an `f64`
//! kernel ([`atan2`], [`atan`], [`asin`], [`acos`], [`tan`], [`tanh`],
//! [`powf`], [`cbrt`], [`hypot`]) are evaluated per lane with the scalar
//! code; they are provided so callers have one namespace for both shapes.

use bytemuck::cast;
use wide::{f32x8, i32x8, CmpEq, CmpGe, CmpGt, CmpLe, CmpLt};

use crate::single::{
    COS_P, EXP_HI, EXP_LO, EXP_P, FRAC_2_PI, LG1, LG2, LG3, LG4, LN2_HI, LN2_LO, LOG2E, PIO2_1,
    PIO2_2, PIO2_3, SIN_P, SQRT2_BITS,
};

/// Constant vectors as `const` items: they live in rodata and load with one
/// instruction. Building them at run time (`f32x8::splat`, or even
/// `f32x8::new([v; 8])` in an expression) lowers to a `memset_pattern16`
/// *call* per constant on aarch64 with wide 0.7 — that was the whole cost of
/// these kernels (27 calls per `sin_cos`).
const TOINT: f32 = 8_388_608.0; // 2^23
const C_NEG0P0: f32x8 = f32x8::new([-0.0; 8]);
const C_NEG0P5: f32x8 = f32x8::new([-0.5; 8]);
const C_NEG126P0: f32x8 = f32x8::new([-126.0; 8]);
const C_NEG25P0: f32x8 = f32x8::new([-25.0; 8]);
const C_NEG2147483648P0: f32x8 = f32x8::new([-2_147_483_648.0; 8]);
const C_0P0: f32x8 = f32x8::new([0.0; 8]);
const C_0P25: f32x8 = f32x8::new([0.25; 8]);
const C_0P5: f32x8 = f32x8::new([0.5; 8]);
const C_1P0: f32x8 = f32x8::new([1.0; 8]);
const C_127P0: f32x8 = f32x8::new([127.0; 8]);
const C_2P0: f32x8 = f32x8::new([2.0; 8]);
const C_2147483648P0: f32x8 = f32x8::new([2_147_483_648.0; 8]);
const C_3P0: f32x8 = f32x8::new([3.0; 8]);
const C_33554432P0: f32x8 = f32x8::new([33_554_432.0; 8]);
const C_4P0: f32x8 = f32x8::new([4.0; 8]);
const C_COSP0: f32x8 = f32x8::new([COS_P[0]; 8]);
const C_COSP1: f32x8 = f32x8::new([COS_P[1]; 8]);
const C_COSP2: f32x8 = f32x8::new([COS_P[2]; 8]);
const C_EXPHI: f32x8 = f32x8::new([EXP_HI; 8]);
const C_EXPLO: f32x8 = f32x8::new([EXP_LO; 8]);
const C_EXPP0: f32x8 = f32x8::new([EXP_P[0]; 8]);
const C_EXPP1: f32x8 = f32x8::new([EXP_P[1]; 8]);
const C_EXPP2: f32x8 = f32x8::new([EXP_P[2]; 8]);
const C_EXPP3: f32x8 = f32x8::new([EXP_P[3]; 8]);
const C_EXPP4: f32x8 = f32x8::new([EXP_P[4]; 8]);
const C_FRAC2PI: f32x8 = f32x8::new([FRAC_2_PI; 8]);
const C_LG1: f32x8 = f32x8::new([LG1; 8]);
const C_LG2: f32x8 = f32x8::new([LG2; 8]);
const C_LG3: f32x8 = f32x8::new([LG3; 8]);
const C_LG4: f32x8 = f32x8::new([LG4; 8]);
const C_LN2HI: f32x8 = f32x8::new([LN2_HI; 8]);
const C_LN2LO: f32x8 = f32x8::new([LN2_LO; 8]);
const C_LOG2E: f32x8 = f32x8::new([LOG2E; 8]);
const C_PIO21: f32x8 = f32x8::new([PIO2_1; 8]);
const C_PIO22: f32x8 = f32x8::new([PIO2_2; 8]);
const C_PIO23: f32x8 = f32x8::new([PIO2_3; 8]);
const C_SINP0: f32x8 = f32x8::new([SIN_P[0]; 8]);
const C_SINP1: f32x8 = f32x8::new([SIN_P[1]; 8]);
const C_SINP2: f32x8 = f32x8::new([SIN_P[2]; 8]);
const C_TOINT: f32x8 = f32x8::new([TOINT; 8]);
const C_INFINITY: f32x8 = f32x8::new([f32::INFINITY; 8]);
const C_MAX: f32x8 = f32x8::new([f32::MAX; 8]);
const C_MINPOSITIVE: f32x8 = f32x8::new([f32::MIN_POSITIVE; 8]);
const C_NAN: f32x8 = f32x8::new([f32::NAN; 8]);
const C_NEGINFINITY: f32x8 = f32x8::new([f32::NEG_INFINITY; 8]);
const C_SQRT2M: f32x8 = f32x8::new([f32::from_bits(SQRT2_BITS); 8]);

/// Exact negation for every input (`-0.0 - x`): `+0.0 → -0.0`, NaN stays
/// NaN. (`wide`'s `Neg` is `0.0 - x`, which turns `+0.0` into `+0.0`.)
#[inline(always)]
fn neg(x: f32x8) -> f32x8 {
    C_NEG0P0 - x
}

/// `2^k` from a float that holds a small integer `k` (`|k| ≤ 127`): the
/// biased exponent `k + 127` is formed in float (exact) and shifted into place.
#[inline(always)]
fn pow2(k: f32x8) -> f32x8 {
    cast((k + C_127P0).trunc_int() << 23i32)
}

/// `single::nearest_int` per lane: `trunc(t + (t < 0 ? -0.5 : 0.5))`.
#[inline(always)]
fn nearest_int(t: f32x8) -> f32x8 {
    let half = t.cmp_lt(C_0P0).blend(C_NEG0P5, C_0P5);
    f32x8::from_i32x8((t + half).trunc_int())
}

/// `f32::round` per lane — nearest integer, ties away from zero (musl
/// `roundf`, same formula as [`crate::round`]).
#[inline]
#[must_use]
pub fn round(x: f32x8) -> f32x8 {
    let ax = x.abs();
    // e >= 0x7f + 23  ⇔  |x| >= 2^23 (or inf / NaN)
    let integral = !ax.cmp_lt(C_TOINT);
    // e < 0x7f - 1  ⇔  |x| < 0.5
    let small = ax.cmp_lt(C_0P5);
    let neg_x = x.cmp_lt(C_0P0);
    let y0 = ax + C_TOINT - C_TOINT - ax;
    let y_ax = y0 + ax;
    let y_gt = y_ax - C_1P0;
    let y_le = y_ax + C_1P0;
    let y = y0
        .cmp_gt(C_0P5)
        .blend(y_gt, y0.cmp_le(C_NEG0P5).blend(y_le, y_ax));
    // y ≥ 1 here; the scalar `if neg { -y }` is the sign blend
    let y = neg_x.blend(neg(y), y);
    // |x| < 0.5 → ±0 with the sign of x (`bits & 0x8000_0000`): `x * 0.0`
    // keeps the sign of x, including for -0.0 (which is not `< 0`)
    let signed_zero = x * C_0P0;
    integral.blend(x, small.blend(signed_zero, y))
}

/// `f32::sqrt` per lane, correctly rounded; canonical NaN for negative / NaN
/// lanes (the hardware default NaN's sign bit differs between `x86` and
/// `AArch64`, so it is replaced here as in [`crate::sqrt`]).
#[inline]
#[must_use]
pub fn sqrt(x: f32x8) -> f32x8 {
    let nan = C_NAN;
    let v = x.cmp_lt(C_0P0).blend(nan, x.sqrt());
    x.cmp_eq(x).blend(v, nan)
}

#[inline(always)]
fn sin_poly(r: f32x8) -> f32x8 {
    let z = r * r;
    let p = ((C_SINP0 * z + C_SINP1) * z + C_SINP2) * z * r;
    r + p
}

#[inline(always)]
fn cos_poly(r: f32x8) -> f32x8 {
    let z = r * r;
    let p = ((C_COSP0 * z + C_COSP1) * z + C_COSP2) * z * z;
    C_1P0 - C_0P5 * z + p
}

/// `(k mod 4 as a float, r)` with `x = k·π/2 + r`.
///
/// `k mod 4` is `kf - 4·floor(kf/4)` (exact: `kf` is integral, and a
/// multiple of 4 once it exceeds 2^25), with the scalar's `kf as i32`
/// saturation reproduced: `kf ≥ 2^31 → 3`, `kf ≤ -2^31 → 0`, NaN → 0.
#[inline(always)]
fn reduce_pio2(x: f32x8) -> (f32x8, f32x8) {
    let kf = nearest_int(x * C_FRAC2PI);
    let r = ((x - kf * C_PIO21) - kf * C_PIO22) - kf * C_PIO23;
    let q = kf - C_4P0 * (kf * C_0P25).floor();
    let q = kf.cmp_ge(C_2147483648P0).blend(C_3P0, q);
    let q = kf.cmp_le(C_NEG2147483648P0).blend(C_0P0, q);
    (q, r)
}

/// Quadrant selection: `k & 1` swaps sin / cos, `k & 2` negates sin,
/// `(k + 1) & 2` negates cos — as blends on `q = k mod 4 ∈ {0, 1, 2, 3}`.
/// Pure selection and exact sign flips: bit-identical to the scalar `match`.
#[inline(always)]
fn quadrant(q: f32x8, s: f32x8, c: f32x8) -> (f32x8, f32x8) {
    let q1 = q.cmp_eq(C_1P0);
    let q2 = q.cmp_eq(C_2P0);
    let q3 = q.cmp_eq(C_3P0);
    let swap = q1 | q3;
    let sv = swap.blend(c, s);
    let cv = swap.blend(s, c);
    let sv = (q2 | q3).blend(neg(sv), sv);
    let cv = (q1 | q2).blend(neg(cv), cv);
    (sv, cv)
}

/// Scalar `!x.is_finite() → NaN` plus `single::canon` (a NaN produced by the
/// arithmetic on a huge argument gets the platform's default NaN sign).
#[inline(always)]
fn canon(x: f32x8, v: f32x8) -> f32x8 {
    let nan = C_NAN;
    let finite = x.abs().cmp_le(C_MAX);
    let v = v.cmp_eq(v).blend(v, nan);
    finite.blend(v, nan)
}

/// Deterministic `sin` per lane (`NaN` for non-finite lanes).
#[inline]
#[must_use]
pub fn sin(x: f32x8) -> f32x8 {
    let (q, r) = reduce_pio2(x);
    let (sv, _) = quadrant(q, sin_poly(r), cos_poly(r));
    canon(x, sv)
}

/// Deterministic `cos` per lane (`NaN` for non-finite lanes).
#[inline]
#[must_use]
pub fn cos(x: f32x8) -> f32x8 {
    let (q, r) = reduce_pio2(x);
    let (_, cv) = quadrant(q, sin_poly(r), cos_poly(r));
    canon(x, cv)
}

/// Deterministic `(sin, cos)` per lane from one range reduction.
#[inline]
#[must_use]
pub fn sin_cos(x: f32x8) -> (f32x8, f32x8) {
    let (q, r) = reduce_pio2(x);
    let (sv, cv) = quadrant(q, sin_poly(r), cos_poly(r));
    (canon(x, sv), canon(x, cv))
}

/// Deterministic `exp` per lane (`+inf` above ≈ 88.72, `0` below ≈ −103.97,
/// gradual subnormal tail, `NaN` stays `NaN`).
#[inline]
#[must_use]
pub fn exp(x: f32x8) -> f32x8 {
    let over = x.cmp_gt(C_EXPHI);
    let under = x.cmp_lt(C_EXPLO);
    // Clamp so the lanes that are blended out below never feed an
    // out-of-range exponent into `pow2`; in-range lanes are unchanged.
    let xc = over.blend(C_EXPHI, under.blend(C_EXPLO, x));
    let kf = nearest_int(xc * C_LOG2E);
    let r = (xc - kf * C_LN2HI) - kf * C_LN2LO;
    let p = (((((C_EXPP0 * r + C_EXPP1) * r + C_EXPP2) * r + C_EXPP3) * r + C_EXPP4) * r + C_0P5)
        * r
        * r
        + r
        + C_1P0;
    // Scalar: k > 127 → p·2^127·2^(k−127); k < −126 → p·2^−126·2^(k+126);
    // else p·2^k. Multiplying by 2^0 = 1.0 is exact, so one formula covers all.
    let k_hi = kf.max(C_NEG126P0).min(C_127P0);
    let k_lo = kf - k_hi;
    let v = p * pow2(k_hi) * pow2(k_lo);
    let v = under.blend(C_0P0, v);
    let v = over.blend(C_INFINITY, v);
    x.cmp_eq(x).blend(v, C_NAN)
}

/// Deterministic natural logarithm per lane (`ln(0) = -inf`,
/// `ln(x < 0) = NaN`, `ln(inf) = inf`).
#[inline]
#[must_use]
pub fn ln(x: f32x8) -> f32x8 {
    // Normalise subnormals so the exponent extraction below is exact
    // (scalar: `bits < 0x0080_0000 → x·2^25, k = -25`).
    let sub = x.cmp_lt(C_MINPOSITIVE);
    let xs = sub.blend(x * C_33554432P0, x);
    let k = sub.blend(C_NEG25P0, C_0P0);
    // biased exponent of a positive normal number: `bits >> 23`
    let bits: i32x8 = cast(xs);
    let e = f32x8::from_i32x8(bits >> 23i32) - C_127P0;
    let k = k + e;
    // m = mantissa in [1, 2): scaling by 2^-e is exact, the same bits as
    // `(bits & 0x007f_ffff) | 0x3f80_0000`. Done as `2^(1-e)` then `·0.5`
    // because `2^-127` is not a normal number (`e = 127` for x ≥ 2^127).
    let m = (xs * pow2(C_1P0 - e)) * C_0P5;
    // Reduce mantissa to [sqrt(2)/2, sqrt(2)).
    let ge = m.cmp_ge(C_SQRT2M);
    let m = ge.blend(m * C_0P5, m);
    let kf = ge.blend(k + C_1P0, k);
    let f = m - C_1P0;
    let s = f / (C_2P0 + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (C_LG2 + w * C_LG4);
    let t2 = z * (C_LG1 + w * C_LG3);
    let r = t2 + t1;
    let hfsq = C_0P5 * f * f;
    let v = kf * C_LN2HI - ((hfsq - (s * (hfsq + r) + kf * C_LN2LO)) - f);
    let v = x.cmp_eq(C_INFINITY).blend(C_INFINITY, v);
    let v = x.cmp_eq(C_0P0).blend(C_NEGINFINITY, v);
    let v = x.cmp_lt(C_0P0).blend(C_NAN, v);
    x.cmp_eq(x).blend(v, C_NAN)
}

#[inline(always)]
fn map(x: f32x8, f: impl Fn(f32) -> f32) -> f32x8 {
    let a = x.to_array();
    f32x8::new([
        f(a[0]),
        f(a[1]),
        f(a[2]),
        f(a[3]),
        f(a[4]),
        f(a[5]),
        f(a[6]),
        f(a[7]),
    ])
}

#[inline(always)]
fn map2(x: f32x8, y: f32x8, f: impl Fn(f32, f32) -> f32) -> f32x8 {
    let a = x.to_array();
    let b = y.to_array();
    f32x8::new([
        f(a[0], b[0]),
        f(a[1], b[1]),
        f(a[2], b[2]),
        f(a[3], b[3]),
        f(a[4], b[4]),
        f(a[5], b[5]),
        f(a[6], b[6]),
        f(a[7], b[7]),
    ])
}

/// [`crate::atan2`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn atan2(y: f32x8, x: f32x8) -> f32x8 {
    map2(y, x, crate::atan2)
}

/// [`crate::atan`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn atan(x: f32x8) -> f32x8 {
    map(x, crate::atan)
}

/// [`crate::asin`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn asin(x: f32x8) -> f32x8 {
    map(x, crate::asin)
}

/// [`crate::acos`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn acos(x: f32x8) -> f32x8 {
    map(x, crate::acos)
}

/// [`crate::tan`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn tan(x: f32x8) -> f32x8 {
    map(x, crate::tan)
}

/// [`crate::tanh`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn tanh(x: f32x8) -> f32x8 {
    map(x, crate::tanh)
}

/// [`crate::powf`] per lane (scalar `f64` kernel).
#[inline]
#[must_use]
pub fn powf(x: f32x8, y: f32x8) -> f32x8 {
    map2(x, y, crate::powf)
}

/// [`crate::cbrt`] per lane.
#[inline]
#[must_use]
pub fn cbrt(x: f32x8) -> f32x8 {
    map(x, crate::cbrt)
}

/// [`crate::hypot`] per lane.
#[inline]
#[must_use]
pub fn hypot(x: f32x8, y: f32x8) -> f32x8 {
    map2(x, y, crate::hypot)
}
