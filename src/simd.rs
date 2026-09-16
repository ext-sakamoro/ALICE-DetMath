//! `f32x8` versions of the `f32` kernels (feature `simd`).
//!
//! Every function here is lane-for-lane **bit-identical** to its scalar
//! counterpart in the crate root: the same constants, the same operation
//! order, no `mul_add` (which would fuse on FMA targets and not on others),
//! and every branch of the scalar code replaced by a mask + blend over
//! values that were computed the same way. `tests/simd_parity.rs` pins this
//! over dense sweeps and the special values.
//!
//! [`sin`], [`cos`], [`sin_cos`], [`exp`], [`ln`], [`round`] and [`sqrt`]
//! are vectorised. The functions whose scalar version evaluates an `f64`
//! kernel ([`atan2`], [`atan`], [`asin`], [`acos`], [`tan`], [`tanh`],
//! [`powf`], [`cbrt`], [`hypot`]) are evaluated per lane with the scalar
//! code; they are provided so callers have one namespace for both shapes.

use bytemuck::cast;
use wide::{f32x8, i32x8, CmpEq, CmpGt, CmpLe, CmpLt};

use crate::single::{
    COS_P, EXP_HI, EXP_LO, EXP_P, FRAC_2_PI, LG1, LG2, LG3, LG4, LN2_HI, LN2_LO, LOG2E, PIO2_1,
    PIO2_2, PIO2_3, SIN_P, SQRT2_BITS,
};

#[inline(always)]
fn splat(v: f32) -> f32x8 {
    f32x8::splat(v)
}

#[inline(always)]
fn isplat(v: i32) -> i32x8 {
    i32x8::splat(v)
}

/// Sign flip that is exactly the scalar `-x` (flips the sign bit of ±0 too;
/// `wide`'s `Neg` is `0.0 - x`, which turns `+0.0` into `+0.0`).
#[inline(always)]
fn neg(x: f32x8) -> f32x8 {
    x ^ splat(-0.0)
}

/// `f32::round` per lane — nearest integer, ties away from zero (musl
/// `roundf`, same formula as [`crate::round`]).
#[must_use]
pub fn round(x: f32x8) -> f32x8 {
    const TOINT: f32 = 8_388_608.0; // 2^23
    let bits: i32x8 = cast(x);
    let e: i32x8 = (bits >> 23i32) & isplat(0xff);
    let integral: f32x8 = cast(e.cmp_gt(isplat(0x7f + 23 - 1)));
    let small: f32x8 = cast(e.cmp_lt(isplat(0x7f - 1)));
    let sign = x & splat(-0.0);
    let ax = x.abs();
    let y0 = ax + splat(TOINT) - splat(TOINT) - ax;
    let y_gt = y0 + ax - splat(1.0);
    let y_le = y0 + ax + splat(1.0);
    let y_mid = y0 + ax;
    let y = y0
        .cmp_gt(splat(0.5))
        .blend(y_gt, y0.cmp_le(splat(-0.5)).blend(y_le, y_mid));
    // y ≥ 1 here, so restoring the sign is a plain OR (scalar: `-y`)
    let y = y | sign;
    integral.blend(x, small.blend(sign, y))
}

/// `f32::sqrt` per lane, correctly rounded; canonical NaN for negative / NaN
/// lanes (the hardware default NaN's sign bit differs between `x86` and
/// `AArch64`, so it is replaced here as in [`crate::sqrt`]).
#[must_use]
pub fn sqrt(x: f32x8) -> f32x8 {
    let bad = x.is_nan() | x.cmp_lt(splat(0.0));
    bad.blend(splat(f32::NAN), x.sqrt())
}

#[inline(always)]
fn sin_poly(r: f32x8) -> f32x8 {
    let z = r * r;
    let p = ((splat(SIN_P[0]) * z + splat(SIN_P[1])) * z + splat(SIN_P[2])) * z * r;
    r + p
}

#[inline(always)]
fn cos_poly(r: f32x8) -> f32x8 {
    let z = r * r;
    let p = ((splat(COS_P[0]) * z + splat(COS_P[1])) * z + splat(COS_P[2])) * z * z;
    splat(1.0) - splat(0.5) * z + p
}

/// `(k mod 4, r)` with `x = k·π/2 + r`; `trunc_int` saturates and maps NaN
/// to 0 exactly like the scalar `kf as i32`.
#[inline(always)]
fn reduce_pio2(x: f32x8) -> (i32x8, f32x8) {
    let kf = round(x * splat(FRAC_2_PI));
    let r = ((x - kf * splat(PIO2_1)) - kf * splat(PIO2_2)) - kf * splat(PIO2_3);
    (kf.trunc_int() & isplat(3), r)
}

#[inline(always)]
fn quadrant_masks(k: i32x8) -> (f32x8, f32x8, f32x8) {
    (
        cast(k.cmp_eq(isplat(0))),
        cast(k.cmp_eq(isplat(1))),
        cast(k.cmp_eq(isplat(2))),
    )
}

/// Deterministic `sin` per lane (`NaN` for non-finite lanes).
#[must_use]
pub fn sin(x: f32x8) -> f32x8 {
    let (k, r) = reduce_pio2(x);
    let (m0, m1, m2) = quadrant_masks(k);
    let s = sin_poly(r);
    let c = cos_poly(r);
    let v = m0.blend(s, m1.blend(c, m2.blend(neg(s), neg(c))));
    x.is_finite().blend(v, splat(f32::NAN))
}

/// Deterministic `cos` per lane (`NaN` for non-finite lanes).
#[must_use]
pub fn cos(x: f32x8) -> f32x8 {
    let (k, r) = reduce_pio2(x);
    let (m0, m1, m2) = quadrant_masks(k);
    let s = sin_poly(r);
    let c = cos_poly(r);
    let v = m0.blend(c, m1.blend(neg(s), m2.blend(neg(c), s)));
    x.is_finite().blend(v, splat(f32::NAN))
}

/// Deterministic `(sin, cos)` per lane from one range reduction.
#[must_use]
pub fn sin_cos(x: f32x8) -> (f32x8, f32x8) {
    let (k, r) = reduce_pio2(x);
    let (m0, m1, m2) = quadrant_masks(k);
    let s = sin_poly(r);
    let c = cos_poly(r);
    let sv = m0.blend(s, m1.blend(c, m2.blend(neg(s), neg(c))));
    let cv = m0.blend(c, m1.blend(neg(s), m2.blend(neg(c), s)));
    let finite = x.is_finite();
    let nan = splat(f32::NAN);
    (finite.blend(sv, nan), finite.blend(cv, nan))
}

/// `2^k` per lane from the exponent bits; `k` must be in `[-126, 127]`
/// (the caller splits the scale, as the scalar `exp` does).
#[inline(always)]
fn pow2i(k: i32x8) -> f32x8 {
    cast((k + isplat(127)) << 23i32)
}

/// Deterministic `exp` per lane (`+inf` above ≈ 88.72, `0` below ≈ −103.97,
/// gradual subnormal tail, `NaN` stays `NaN`).
#[must_use]
pub fn exp(x: f32x8) -> f32x8 {
    let nan = x.is_nan();
    let over = x.cmp_gt(splat(EXP_HI));
    let under = x.cmp_lt(splat(EXP_LO));
    // Clamp so the lanes that are blended out below never feed an
    // out-of-range exponent into `pow2i`; in-range lanes are unchanged.
    let xc = over.blend(splat(EXP_HI), under.blend(splat(EXP_LO), x));
    let kf = round(xc * splat(LOG2E));
    let k = kf.trunc_int();
    let r = (xc - kf * splat(LN2_HI)) - kf * splat(LN2_LO);
    let p = (((((splat(EXP_P[0]) * r + splat(EXP_P[1])) * r + splat(EXP_P[2])) * r
        + splat(EXP_P[3]))
        * r
        + splat(EXP_P[4]))
        * r
        + splat(0.5))
        * r
        * r
        + r
        + splat(1.0);
    // Scalar: k > 127 → p·2^127·2^(k−127); k < −126 → p·2^−126·2^(k+126);
    // else p·2^k. Multiplying by 2^0 = 1.0 is exact, so one formula covers all.
    let k_hi = k.max(isplat(-126)).min(isplat(127));
    let k_lo = k - k_hi;
    let v = p * pow2i(k_hi) * pow2i(k_lo);
    nan.blend(
        splat(f32::NAN),
        over.blend(splat(f32::INFINITY), under.blend(splat(0.0), v)),
    )
}

/// Deterministic natural logarithm per lane (`ln(0) = -inf`,
/// `ln(x < 0) = NaN`, `ln(inf) = inf`).
#[must_use]
pub fn ln(x: f32x8) -> f32x8 {
    let bad = x.is_nan() | x.cmp_lt(splat(0.0));
    let zero = x.cmp_eq(splat(0.0));
    let inf = x.cmp_eq(splat(f32::INFINITY));
    let bits: i32x8 = cast(x);
    // Normalise subnormals so the exponent extraction below is exact.
    let sub = bits.cmp_lt(isplat(0x0080_0000));
    let scaled: i32x8 = cast(x * splat(33_554_432.0)); // 2^25
    let bits = sub.blend(scaled, bits);
    let k = sub.blend(isplat(-25), isplat(0));
    // Reduce mantissa to [sqrt(2)/2, sqrt(2)).
    let k = k + ((bits >> 23i32) - isplat(127));
    let bits = (bits & isplat(0x007f_ffff)) | isplat(0x3f80_0000); // m in [1, 2)
    let ge = !bits.cmp_lt(isplat(SQRT2_BITS as i32));
    let bits = ge.blend(bits - isplat(0x0080_0000), bits);
    let k = ge.blend(k + isplat(1), k);
    let m: f32x8 = cast(bits);
    let f = m - splat(1.0);
    let s = f / (splat(2.0) + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (splat(LG2) + w * splat(LG4));
    let t2 = z * (splat(LG1) + w * splat(LG3));
    let r = t2 + t1;
    let hfsq = splat(0.5) * f * f;
    let kf = f32x8::from_i32x8(k);
    let v = kf * splat(LN2_HI) - ((hfsq - (s * (hfsq + r) + kf * splat(LN2_LO))) - f);
    bad.blend(
        splat(f32::NAN),
        zero.blend(splat(f32::NEG_INFINITY), inf.blend(splat(f32::INFINITY), v)),
    )
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
#[must_use]
pub fn atan2(y: f32x8, x: f32x8) -> f32x8 {
    map2(y, x, crate::atan2)
}

/// [`crate::atan`] per lane (scalar `f64` kernel).
#[must_use]
pub fn atan(x: f32x8) -> f32x8 {
    map(x, crate::atan)
}

/// [`crate::asin`] per lane (scalar `f64` kernel).
#[must_use]
pub fn asin(x: f32x8) -> f32x8 {
    map(x, crate::asin)
}

/// [`crate::acos`] per lane (scalar `f64` kernel).
#[must_use]
pub fn acos(x: f32x8) -> f32x8 {
    map(x, crate::acos)
}

/// [`crate::tan`] per lane (scalar `f64` kernel).
#[must_use]
pub fn tan(x: f32x8) -> f32x8 {
    map(x, crate::tan)
}

/// [`crate::tanh`] per lane (scalar `f64` kernel).
#[must_use]
pub fn tanh(x: f32x8) -> f32x8 {
    map(x, crate::tanh)
}

/// [`crate::powf`] per lane (scalar `f64` kernel).
#[must_use]
pub fn powf(x: f32x8, y: f32x8) -> f32x8 {
    map2(x, y, crate::powf)
}

/// [`crate::cbrt`] per lane.
#[must_use]
pub fn cbrt(x: f32x8) -> f32x8 {
    map(x, crate::cbrt)
}

/// [`crate::hypot`] per lane.
#[must_use]
pub fn hypot(x: f32x8, y: f32x8) -> f32x8 {
    map2(x, y, crate::hypot)
}
