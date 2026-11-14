//! `floor64` against two references that do not use it: the definition (the result is an integer, not above
//! `x`, and the next integer up is above `x`), checked on the value's bits, and the platform's `f64::floor`
//! (IEEE 754 `roundToIntegralTowardNegative`, exact on every platform), on boundaries and on 2^20 values
//! spread over every exponent.
//!
//! The policy for the values the definition does not settle is stated and pinned: `±0` keep their sign,
//! `±∞` are returned unchanged, every NaN becomes the canonical NaN (as `round64` / `sqrt64` do).

use alice_det_math::floor64;

/// `x` is an integer: its fractional part, computed from the bits, is zero
fn is_integral(x: f64) -> bool {
    let bits = x.to_bits();
    let e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    if x == 0.0 || e >= 52 {
        return true;
    }
    if e < 0 {
        return false;
    }
    let frac_mask = (1u64 << (52 - e)) - 1;
    bits & frac_mask == 0
}

fn check(x: f64) {
    let got = floor64(x);
    let want = x.floor();
    assert_eq!(
        got.to_bits(),
        want.to_bits(),
        "floor64({x:e}) = {got:e}, f64::floor = {want:e}"
    );
    if x.is_finite() {
        assert!(
            is_integral(got),
            "floor64({x:e}) = {got:e} is not an integer"
        );
        assert!(got <= x, "floor64({x:e}) = {got:e} is above x");
        // the next integer up is above x (exact: below 2^53 the integers are representable)
        if got.abs() < 9_007_199_254_740_992.0 {
            assert!(
                got + 1.0 > x,
                "floor64({x:e}) = {got:e}: {} is not above x",
                got + 1.0
            );
        }
    }
}

#[test]
fn boundaries() {
    let ulp_below_one = f64::from_bits(1.0f64.to_bits() - 1);
    let cases = [
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        0.25,
        -0.25,
        1.0,
        -1.0,
        2.0,
        -2.0,
        ulp_below_one,
        -ulp_below_one,
        1.0 + f64::EPSILON,
        -1.0 - f64::EPSILON,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        -f64::from_bits(1),
        4_503_599_627_370_495.5, // 2^52 - 0.5: the last value with a fraction
        -4_503_599_627_370_495.5, // its negative
        4_503_599_627_370_496.0, // 2^52
        -4_503_599_627_370_497.0, // past 2^52, integral
        9_007_199_254_740_993.0, // 2^53 + 1 (rounds to 2^53 on parse, still integral)
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
        123_456.789,
        -123_456.789,
        -3.0,
        -2.999_999_999_999_999_6,
    ];
    for x in cases {
        check(x);
    }
}

#[test]
fn values_the_definition_does_not_settle() {
    // signed zero keeps its sign
    assert_eq!(floor64(0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(floor64(-0.0).to_bits(), (-0.0f64).to_bits());
    // a negative value above -1 goes to -1, a positive one below 1 to +0 (not -0)
    assert_eq!(floor64(-1e-300).to_bits(), (-1.0f64).to_bits());
    assert_eq!(floor64(1e-300).to_bits(), 0.0f64.to_bits());
    assert_eq!(floor64(0.999_999).to_bits(), 0.0f64.to_bits());
    // infinities unchanged
    assert_eq!(floor64(f64::INFINITY), f64::INFINITY);
    assert_eq!(floor64(f64::NEG_INFINITY), f64::NEG_INFINITY);
    // every NaN (signalling, negative, with a payload) is the canonical NaN
    for bits in [
        0x7ff8_0000_0000_0000u64,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0x7ff4_dead_beef_0001,
    ] {
        assert_eq!(
            floor64(f64::from_bits(bits)).to_bits(),
            f64::NAN.to_bits(),
            "NaN {bits:#x}"
        );
    }
}

#[test]
fn every_exponent() {
    // xorshift64*, fixed seed: 2^20 values whose bits cover every exponent and sign
    let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut n = 0;
    for _ in 0..(1 << 20) {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        let bits = s.wrapping_mul(0x2545_f491_4f6c_dd1d);
        let x = f64::from_bits(bits);
        if x.is_nan() {
            continue;
        }
        check(x);
        n += 1;
    }
    assert!(n > 1_000_000, "only {n} values compared");
}

#[test]
fn near_every_integer_up_to_2_pow_12() {
    // the values on either side of each integer, where an off-by-one in the correction shows
    let mut n = 0;
    for k in -4096i32..=4096 {
        let m = f64::from(k);
        for x in [
            m,
            f64::from_bits(m.to_bits().wrapping_add(1)),
            f64::from_bits(m.to_bits().wrapping_sub(1)),
            m + 0.5,
            m - 0.5,
        ] {
            if x.is_finite() {
                check(x);
                n += 1;
            }
        }
    }
    assert!(n > 40_000, "only {n} values compared");
}
