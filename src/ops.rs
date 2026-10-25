//! Deterministic `round` / `sqrt` built from IEEE 754 basic operations.
//!
//! `f32::round` and `f32::sqrt` are exact functions (every correct libm and
//! every IEEE `sqrt` instruction return the same bits), but `core` does not
//! expose them on a `no_std` target. These implementations reproduce them
//! bit-for-bit from `+ - * /`, comparisons and integer arithmetic so the rest
//! of the crate can call them on every target:
//!
//! * [`round`] / [`round64`]: musl `roundf` / `round` (the `x + 2^23 - 2^23`
//!   trick, round-half-away-from-zero, sign of zero preserved). A NaN input
//!   is returned unchanged (the hardware would quiet a signalling NaN).
//! * [`sqrt`] / [`sqrt64`]: the hardware instruction when `std` is enabled,
//!   otherwise an integer digit-by-digit square root with IEEE
//!   round-to-nearest-even. Both are correctly rounded, so the bits agree.
//!
//! `sqrt(NaN)` and `sqrt(x < 0)` return the canonical positive quiet NaN on
//! every target (a hardware `sqrt` of a negative number returns the *default*
//! NaN, whose sign bit differs between `x86` and `AArch64`).
//!
//! * [`max`] / [`min`] / [`max64`] / [`min64`]: `f32::max` / `f32::min` (and
//!   the `f64` forms) leave the sign of an equal pair of zeros unspecified,
//!   and the targets differ (`AArch64`'s hardware `fmax`/`fmin` already
//!   return `+0`/`-0` there, matching IEEE 754-2019's `maximumNumber` /
//!   `minimumNumber`; `x86_64`'s `maxss`/`minss` return one of the two
//!   operands unchanged, so `0.0f32.max(-0.0)` is `-0.0` there, and even that
//!   differs between a debug build and one the optimizer has reordered). A
//!   NaN operand yields the other operand (as `f32::max`/`min` already do);
//!   every other pair is ordered by value. Pure comparisons, so the result is
//!   the same bits on every target and optimization level.

/// `f32::round` — nearest integer, ties away from zero (musl `roundf`).
#[inline]
#[must_use]
pub fn round(x: f32) -> f32 {
    const TOINT: f32 = 8_388_608.0; // 2^23
    let bits = x.to_bits();
    let e = (bits >> 23) & 0xff;
    if e >= 0x7f + 23 {
        // already integral or inf; a NaN of any payload → the canonical NaN
        return if x.is_nan() { f32::NAN } else { x };
    }
    let neg = bits >> 31 != 0;
    let ax = if neg { -x } else { x };
    if e < 0x7f - 1 {
        // |x| < 0.5 → ±0 with the sign of x
        return f32::from_bits(bits & 0x8000_0000);
    }
    let mut y = ax + TOINT - TOINT - ax;
    if y > 0.5 {
        y = y + ax - 1.0;
    } else if y <= -0.5 {
        y = y + ax + 1.0;
    } else {
        y += ax;
    }
    if neg {
        -y
    } else {
        y
    }
}

/// `f64::round` — nearest integer, ties away from zero (musl `round`).
#[inline]
#[must_use]
pub fn round64(x: f64) -> f64 {
    const TOINT: f64 = 4_503_599_627_370_496.0; // 2^52
    let bits = x.to_bits();
    let e = (bits >> 52) & 0x7ff;
    if e >= 0x3ff + 52 {
        return if x.is_nan() { f64::NAN } else { x };
    }
    let neg = bits >> 63 != 0;
    let ax = if neg { -x } else { x };
    if e < 0x3ff - 1 {
        return f64::from_bits(bits & 0x8000_0000_0000_0000);
    }
    let mut y = ax + TOINT - TOINT - ax;
    if y > 0.5 {
        y = y + ax - 1.0;
    } else if y <= -0.5 {
        y = y + ax + 1.0;
    } else {
        y += ax;
    }
    if neg {
        -y
    } else {
        y
    }
}

/// `f32::sqrt`, correctly rounded; canonical NaN for negative / NaN input.
#[inline]
#[must_use]
pub fn sqrt(x: f32) -> f32 {
    if x.is_nan() || x < 0.0 {
        return f32::NAN;
    }
    if x == 0.0 || x.is_infinite() {
        return x; // ±0 → ±0, +inf → +inf
    }
    #[cfg(feature = "std")]
    {
        x.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        soft_sqrt32(x)
    }
}

/// `f64::sqrt`, correctly rounded; canonical NaN for negative / NaN input.
#[inline]
#[must_use]
pub fn sqrt64(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x.is_infinite() {
        return x;
    }
    #[cfg(feature = "std")]
    {
        x.sqrt()
    }
    #[cfg(not(feature = "std"))]
    {
        soft_sqrt64(x)
    }
}

/// `max` with a defined zero sign: `max(-0.0, 0.0)` and `max(0.0, -0.0)` are `+0.0`.
///
/// A NaN operand yields the other operand; if that operand is also NaN, the
/// canonical NaN (this crate's NaN policy), not whichever payload the
/// chosen operand carried.
#[inline]
#[must_use]
pub fn max(a: f32, b: f32) -> f32 {
    let r = if a.is_nan() || b > a {
        b
    } else if b.is_nan() || a > b || !a.is_sign_negative() {
        a
    } else {
        b
    };
    if r.is_nan() {
        f32::NAN
    } else {
        r
    }
}

/// `min` with a defined zero sign: `min(-0.0, 0.0)` and `min(0.0, -0.0)` are `-0.0`.
///
/// A NaN operand yields the other operand; if that operand is also NaN, the
/// canonical NaN (this crate's NaN policy), not whichever payload the
/// chosen operand carried.
#[inline]
#[must_use]
pub fn min(a: f32, b: f32) -> f32 {
    let r = if a.is_nan() || b < a {
        b
    } else if b.is_nan() || a < b || a.is_sign_negative() {
        a
    } else {
        b
    };
    if r.is_nan() {
        f32::NAN
    } else {
        r
    }
}

/// [`max`] for `f64`.
#[inline]
#[must_use]
pub fn max64(a: f64, b: f64) -> f64 {
    let r = if a.is_nan() || b > a {
        b
    } else if b.is_nan() || a > b || !a.is_sign_negative() {
        a
    } else {
        b
    };
    if r.is_nan() {
        f64::NAN
    } else {
        r
    }
}

/// [`min`] for `f64`.
#[inline]
#[must_use]
pub fn min64(a: f64, b: f64) -> f64 {
    let r = if a.is_nan() || b < a {
        b
    } else if b.is_nan() || a < b || a.is_sign_negative() {
        a
    } else {
        b
    };
    if r.is_nan() {
        f64::NAN
    } else {
        r
    }
}

/// `x` restricted to `[low, high]` for `f64`: `min64(max64(x, low), high)`, and the canonical
/// NaN for a NaN `x`. See [`crate::metric::clamp`] for the rule (the same for both widths).
#[inline]
#[must_use]
pub fn clamp64(x: f64, low: f64, high: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    min64(max64(x, low), high)
}

/// Whether `a` is above `b` in the G2 order (`-0 < +0`), for two numbers that are not NaN.
#[inline]
fn above(a: f32, b: f32) -> bool {
    a > b || (a == b && !a.is_sign_negative() && b.is_sign_negative())
}

/// The index of the first G2 maximum of `xs`.
///
/// That is the first element equal to the [`max`] of the elements that are not NaN: a NaN is
/// never selected, `+0` is above `-0`, and of equal values the first is
/// kept. `None` for an empty slice or one of NaNs only.
#[must_use]
pub fn argmax(xs: &[f32]) -> Option<usize> {
    let mut best = Select::greatest();
    for (i, &x) in xs.iter().enumerate() {
        best.offer(x, i);
    }
    best.into_inner().map(|(_, i)| i)
}

/// The index of the first G2 minimum of `xs`.
///
/// That is the first element equal to the [`min`] of the elements that are not NaN: a NaN is
/// never selected, `-0` is below `+0`, and of equal values the first is
/// kept. `None` for an empty slice or one of NaNs only.
#[must_use]
pub fn argmin(xs: &[f32]) -> Option<usize> {
    let mut best = Select::least();
    for (i, &x) in xs.iter().enumerate() {
        best.offer(x, i);
    }
    best.into_inner().map(|(_, i)| i)
}

/// A selection over a stream of values that carry a payload.
///
/// It keeps the value and payload of the first offer equal to the G2 maximum ([`Select::greatest`], [`argmax`] for a stream) or minimum
/// ([`Select::least`], [`argmin`] for a stream) of the offered values that are not NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct Select<T> {
    greatest: bool,
    best: Option<(f32, T)>,
}

impl<T> Select<T> {
    /// Keeps the first offer equal to the G2 maximum: `+0` is above `-0`.
    #[must_use]
    pub const fn greatest() -> Self {
        Self {
            greatest: true,
            best: None,
        }
    }

    /// Keeps the first offer equal to the G2 minimum: `-0` is below `+0`.
    #[must_use]
    pub const fn least() -> Self {
        Self {
            greatest: false,
            best: None,
        }
    }

    /// Offer a value with its payload: kept when it is not NaN and strictly beyond the value
    /// kept so far in the selection's direction (a tie keeps the earlier offer).
    pub fn offer(&mut self, value: f32, payload: T) {
        if value.is_nan() {
            return;
        }
        let beyond = |kept: f32| {
            if self.greatest {
                above(value, kept)
            } else {
                above(kept, value)
            }
        };
        if self.best.as_ref().is_none_or(|(b, _)| beyond(*b)) {
            self.best = Some((value, payload));
        }
    }

    /// The value and payload kept, `None` when no value that is not NaN was offered.
    #[must_use]
    pub fn into_inner(self) -> Option<(f32, T)> {
        self.best
    }
}

/// `floor(sqrt(n))` and the remainder `n - floor(sqrt(n))²`, digit by digit.
#[cfg(any(not(feature = "std"), test))]
const fn isqrt_rem(n: u128) -> (u128, u128) {
    let mut rem = n;
    let mut root: u128 = 0;
    let mut bit: u128 = 1 << 126;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    (root, rem)
}

/// Correctly rounded square root of a positive finite binary float given as
/// its significand `man` (hidden bit included, `mant_bits + 1` bits wide
/// after normalisation) and unbiased exponent `e` (`x = man · 2^e`).
///
/// Returns the rounded significand (`mant_bits + 1` bits, hidden bit
/// included) and the unbiased exponent of its leading bit.
#[cfg(any(not(feature = "std"), test))]
fn sqrt_round(mut man: u128, mut e: i32, mant_bits: u32) -> (u128, i32) {
    // Make the exponent even so sqrt(x) = sqrt(man) · 2^(e/2).
    if e & 1 != 0 {
        man <<= 1;
        e -= 1;
    }
    // Scale so the integer root carries mant_bits + 3 significant bits
    // (guard, round and one spare) for every normalised `man`.
    let s = (mant_bits + 4) / 2 + 2;
    let big = man << (2 * s);
    let (t, rem) = isqrt_rem(big);
    // sqrt(x) = (t + frac) · 2^(e/2 - s), frac ∈ [0, 1), frac > 0 iff rem > 0.
    let nb = 128 - t.leading_zeros();
    let drop = nb - (mant_bits + 1);
    let mut kept = t >> drop;
    let dropped = t & ((1u128 << drop) - 1);
    let half = 1u128 << (drop - 1);
    let round_up = dropped > half || (dropped == half && (rem != 0 || kept & 1 == 1));
    let mut exp = e / 2 - s as i32 + drop as i32;
    if round_up {
        kept += 1;
        if kept == 1u128 << (mant_bits + 1) {
            kept >>= 1;
            exp += 1;
        }
    }
    (kept, exp + mant_bits as i32)
}

#[cfg(any(not(feature = "std"), test))]
fn soft_sqrt32(x: f32) -> f32 {
    let bits = x.to_bits();
    let mut exp = ((bits >> 23) & 0xff) as i32;
    let mut man = u128::from(bits & 0x007f_ffff);
    if exp == 0 {
        // subnormal: normalise so the hidden bit is at position 23
        let shift = man.leading_zeros() - (128 - 24);
        man <<= shift;
        exp = 1 - shift as i32;
    } else {
        man |= 0x0080_0000;
    }
    let (kept, e) = sqrt_round(man, exp - 127 - 23, 23);
    // the result of a square root is never subnormal or infinite
    f32::from_bits(((e + 127) as u32) << 23 | (kept as u32 & 0x007f_ffff))
}

#[cfg(any(not(feature = "std"), test))]
fn soft_sqrt64(x: f64) -> f64 {
    let bits = x.to_bits();
    let mut exp = ((bits >> 52) & 0x7ff) as i32;
    let mut man = u128::from(bits & 0x000f_ffff_ffff_ffff);
    if exp == 0 {
        let shift = man.leading_zeros() - (128 - 53);
        man <<= shift;
        exp = 1 - shift as i32;
    } else {
        man |= 0x0010_0000_0000_0000;
    }
    let (kept, e) = sqrt_round(man, exp - 1023 - 52, 52);
    f64::from_bits(((e + 1023) as u64) << 52 | (kept as u64 & 0x000f_ffff_ffff_ffff))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_matches_std_on_edge_cases() {
        for &x in &[
            0.0f32,
            -0.0,
            0.3,
            -0.3,
            0.499_999_97,
            -0.499_999_97,
            0.5,
            -0.5,
            1.5,
            -1.5,
            2.5,
            -2.5,
            8_388_607.5,
            8_388_608.0,
            -8_388_608.0,
            1.0e-40,
            f32::MAX,
            f32::MIN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            assert_eq!(round(x).to_bits(), x.round().to_bits(), "round({x})");
        }
        assert!(round(f32::NAN).is_nan());
        for &x in &[
            0.0f64,
            -0.0,
            0.5,
            -0.5,
            2.5,
            -2.5,
            4_503_599_627_370_495.5,
            1.0e-310,
        ] {
            assert_eq!(round64(x).to_bits(), x.round().to_bits(), "round64({x})");
        }
    }

    #[test]
    fn round_matches_std_on_sweep() {
        for i in 0..=200_000u32 {
            let x = -1000.0 + 2000.0 * (i as f32 / 200_000.0);
            assert_eq!(round(x).to_bits(), x.round().to_bits(), "round({x})");
            let y = f64::from(x) * 1.000_000_1;
            assert_eq!(round64(y).to_bits(), y.round().to_bits(), "round64({y})");
        }
    }

    #[test]
    fn soft_sqrt_matches_hardware_bits() {
        // every exponent, several mantissas each, plus subnormals
        // positive finite inputs only: the public `sqrt` handles 0 / inf / NaN
        // before reaching the software root
        for e in 0u32..=254 {
            for m in [
                0u32, 1, 0x12_3456, 0x40_0000, 0x7f_ffff, 0x55_5555, 0x2a_aaaa,
            ] {
                if e == 0 && m == 0 {
                    continue;
                }
                let x = f32::from_bits(e << 23 | m);
                assert_eq!(soft_sqrt32(x).to_bits(), x.sqrt().to_bits(), "sqrt({x:e})");
            }
        }
        for i in 1..=200_000u32 {
            let x = 1.0e-3 + 1.0e3 * (i as f32 / 200_000.0);
            assert_eq!(soft_sqrt32(x).to_bits(), x.sqrt().to_bits(), "sqrt({x})");
        }
        for e in 0u64..=2046 {
            for m in [
                0u64,
                1,
                0x1234_5678_9abc,
                0x8_0000_0000_0000,
                0xf_ffff_ffff_ffff,
            ] {
                if e == 0 && m == 0 {
                    continue;
                }
                let x = f64::from_bits(e << 52 | m);
                assert_eq!(
                    soft_sqrt64(x).to_bits(),
                    x.sqrt().to_bits(),
                    "sqrt64({x:e})"
                );
            }
        }
        assert_eq!(sqrt(-0.0).to_bits(), (-0.0f32).to_bits());
        assert_eq!(sqrt(f32::INFINITY), f32::INFINITY);
        assert_eq!(sqrt(-1.0).to_bits(), f32::NAN.to_bits());
        assert_eq!(sqrt64(-1.0).to_bits(), f64::NAN.to_bits());
    }

    /// `max` / `min` oracle: every pair from a lattice of {-inf, -1, -subnormal,
    /// -0, 0, subnormal, 1, inf, NaN}, compared bit-for-bit against an expected
    /// value computed by rule (not with the functions under test). f32 and f64
    /// each get their own native lattice (so the f32 functions see a genuine
    /// f32 subnormal, not an f64 subnormal that flushed to 0.0 on cast).
    mod max_min_tests {
        use super::{max, max64, min, min64};

        fn expected_max64(a: f64, b: f64) -> f64 {
            match (a.is_nan(), b.is_nan()) {
                (true, true) => f64::NAN,
                (true, false) => b,
                (false, true) => a,
                _ if a > b => a,
                _ if b > a => b,
                _ => {
                    if a != 0.0 {
                        a
                    } else if a.is_sign_negative() && b.is_sign_negative() {
                        -0.0
                    } else {
                        0.0
                    }
                }
            }
        }

        fn expected_min64(a: f64, b: f64) -> f64 {
            match (a.is_nan(), b.is_nan()) {
                (true, true) => f64::NAN,
                (true, false) => b,
                (false, true) => a,
                _ if a < b => a,
                _ if b < a => b,
                _ => {
                    if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
                        -0.0
                    } else {
                        a
                    }
                }
            }
        }

        fn expected_max32(a: f32, b: f32) -> f32 {
            match (a.is_nan(), b.is_nan()) {
                (true, true) => f32::NAN,
                (true, false) => b,
                (false, true) => a,
                _ if a > b => a,
                _ if b > a => b,
                _ => {
                    if a != 0.0 {
                        a
                    } else if a.is_sign_negative() && b.is_sign_negative() {
                        -0.0
                    } else {
                        0.0
                    }
                }
            }
        }

        fn expected_min32(a: f32, b: f32) -> f32 {
            match (a.is_nan(), b.is_nan()) {
                (true, true) => f32::NAN,
                (true, false) => b,
                (false, true) => a,
                _ if a < b => a,
                _ if b < a => b,
                _ => {
                    if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
                        -0.0
                    } else {
                        a
                    }
                }
            }
        }

        // a bit-exact comparison, not "both NaN": this crate's NaN policy makes
        // every NaN this crate returns the canonical one (`tests/nan_policy.rs`,
        // `tests/nan_in_nan_out.rs`), so a reference function that expects a
        // payload-carrying NaN (`f64::NAN` only ever is the canonical one) and
        // compares bit-exact is the oracle that actually has teeth: treating
        // "both NaN" as equal regardless of payload let a payload NaN
        // (`max(0x7fa12345, 0xffc00001) = 0xffc00001`) through uncaught.
        fn same64(x: f64, y: f64) -> bool {
            x.to_bits() == y.to_bits()
        }

        fn same32(x: f32, y: f32) -> bool {
            x.to_bits() == y.to_bits()
        }

        const LATTICE64: [f64; 9] = [
            f64::NEG_INFINITY,
            -1.0,
            -5e-324, // f64::from_bits(1), the smallest-magnitude negative subnormal
            -0.0,
            0.0,
            5e-324, // f64::from_bits(1), the smallest-magnitude positive subnormal
            1.0,
            f64::INFINITY,
            f64::NAN,
        ];

        const LATTICE32: [f32; 9] = [
            f32::NEG_INFINITY,
            -1.0,
            -1e-45, // f32::from_bits(1), the smallest-magnitude negative subnormal
            -0.0,
            0.0,
            1e-45, // f32::from_bits(1), the smallest-magnitude positive subnormal
            1.0,
            f32::INFINITY,
            f32::NAN,
        ];

        #[test]
        fn lattice_values_are_actually_subnormal_or_the_extremes_they_claim_to_be() {
            // a sanity check on the lattice itself: if these literals ever
            // stopped meaning what their comments say (e.g. rounded to 0 or to
            // a normal number), the oracle below would silently stop covering
            // the cases it claims to
            assert!(LATTICE64[2].is_subnormal() && LATTICE64[2] < 0.0);
            assert!(LATTICE64[5].is_subnormal() && LATTICE64[5] > 0.0);
            assert!(LATTICE32[2].is_subnormal() && LATTICE32[2] < 0.0);
            assert!(LATTICE32[5].is_subnormal() && LATTICE32[5] > 0.0);
        }

        #[test]
        fn max64_and_min64_follow_the_rule_on_the_signed_zero_and_subnormal_lattice() {
            let mut compared = 0;
            for &a in &LATTICE64 {
                for &b in &LATTICE64 {
                    let (x, y) = (core::hint::black_box(a), core::hint::black_box(b));
                    assert!(
                        same64(max64(x, y), expected_max64(a, b)),
                        "max64({a:?}, {b:?}) = {:?}",
                        max64(x, y)
                    );
                    assert!(
                        same64(min64(x, y), expected_min64(a, b)),
                        "min64({a:?}, {b:?}) = {:?}",
                        min64(x, y)
                    );
                    compared += 2;
                }
            }
            assert_eq!(compared, 2 * 81);
        }

        #[test]
        fn max_and_min_follow_the_rule_on_the_signed_zero_and_subnormal_lattice() {
            let mut compared = 0;
            for &a in &LATTICE32 {
                for &b in &LATTICE32 {
                    let (x, y) = (core::hint::black_box(a), core::hint::black_box(b));
                    assert!(
                        same32(max(x, y), expected_max32(a, b)),
                        "max({a:?}, {b:?}) = {:?}",
                        max(x, y)
                    );
                    assert!(
                        same32(min(x, y), expected_min32(a, b)),
                        "min({a:?}, {b:?}) = {:?}",
                        min(x, y)
                    );
                    compared += 2;
                }
            }
            assert_eq!(compared, 2 * 81);
        }

        #[test]
        fn the_zero_pair_has_the_documented_sign_in_both_orders() {
            let (p, n) = (
                core::hint::black_box(0.0f32),
                core::hint::black_box(-0.0f32),
            );
            assert_eq!(max(p, n).to_bits(), 0x0000_0000);
            assert_eq!(max(n, p).to_bits(), 0x0000_0000);
            assert_eq!(min(p, n).to_bits(), 0x8000_0000);
            assert_eq!(min(n, p).to_bits(), 0x8000_0000);
            let (p, n) = (
                core::hint::black_box(0.0f64),
                core::hint::black_box(-0.0f64),
            );
            assert_eq!(max64(p, n).to_bits(), 0);
            assert_eq!(max64(n, p).to_bits(), 0);
            assert_eq!(min64(p, n).to_bits(), 0x8000_0000_0000_0000);
            assert_eq!(min64(n, p).to_bits(), 0x8000_0000_0000_0000);
        }

        /// A real defect found in review: when both operands are NaN, `max`
        /// / `min` returned one operand's raw payload instead of the
        /// canonical NaN this crate's NaN policy requires everywhere
        /// (`tests/nan_policy.rs`) — `max(f32::from_bits(0x7fa1_2345),
        /// f32::from_bits(0xffc0_0001))` returned `0xffc0_0001`, not
        /// `0x7fc0_0000`. The lattice/specials sweeps elsewhere in this
        /// module, and `tests/nan_in_nan_out.rs`, only ever put the
        /// canonical NaN into the grid, so none of them could have caught
        /// this; this test uses distinct payload-carrying bit patterns on
        /// both sides (never the canonical one on either side alone).
        #[test]
        fn both_operands_nan_with_different_payloads_yield_the_canonical_nan() {
            let payloads32 = [
                0x7fc0_0001u32, // quiet, positive, nonzero payload
                0xffc0_0000,    // quiet, negative, zero payload (the default NaN sign on x86-64)
                0xffc0_0001,    // quiet, negative, nonzero payload
                0x7fa1_2345,    // quiet, positive, a different nonzero payload
                0x7f80_0001,    // signalling, positive
                0xff80_0001,    // signalling, negative
            ];
            let mut compared = 0;
            for &ab in &payloads32 {
                for &bb in &payloads32 {
                    let (a, b) = (f32::from_bits(ab), f32::from_bits(bb));
                    assert_eq!(
                        max(a, b).to_bits(),
                        f32::NAN.to_bits(),
                        "max({ab:#010x}, {bb:#010x})"
                    );
                    assert_eq!(
                        min(a, b).to_bits(),
                        f32::NAN.to_bits(),
                        "min({ab:#010x}, {bb:#010x})"
                    );
                    compared += 2;
                }
            }
            let payloads64 = [
                0x7ff8_0000_0000_0001u64,
                0xfff8_0000_0000_0000,
                0xfff8_0000_0000_0002,
                0x7ff0_0000_0000_0001,
                0xfff0_0000_0000_0001,
            ];
            for &ab in &payloads64 {
                for &bb in &payloads64 {
                    let (a, b) = (f64::from_bits(ab), f64::from_bits(bb));
                    assert_eq!(
                        max64(a, b).to_bits(),
                        f64::NAN.to_bits(),
                        "max64({ab:#018x}, {bb:#018x})"
                    );
                    assert_eq!(
                        min64(a, b).to_bits(),
                        f64::NAN.to_bits(),
                        "min64({ab:#018x}, {bb:#018x})"
                    );
                    compared += 2;
                }
            }
            assert_eq!(compared, 2 * 36 + 2 * 25, "compared");
        }
    }
}
