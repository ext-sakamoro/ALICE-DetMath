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

/// `f32::round` — nearest integer, ties away from zero (musl `roundf`).
#[must_use]
pub fn round(x: f32) -> f32 {
    const TOINT: f32 = 8_388_608.0; // 2^23
    let bits = x.to_bits();
    let e = (bits >> 23) & 0xff;
    if e >= 0x7f + 23 {
        // already integral (or inf / NaN)
        return x;
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
#[must_use]
pub fn round64(x: f64) -> f64 {
    const TOINT: f64 = 4_503_599_627_370_496.0; // 2^52
    let bits = x.to_bits();
    let e = (bits >> 52) & 0x7ff;
    if e >= 0x3ff + 52 {
        return x;
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
}
