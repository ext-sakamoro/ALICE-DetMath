//! `powi64`: the closed form on exact cases, the special values, and the multiplication order pinned bit for bit.
//!
//! The exact cases need no reference: an integer power of 2 is a power of 2 (every product is exact while the result
//! is representable, subnormals included), and small integer powers of small integers are exact. The order is pinned
//! by results computed outside this crate (the same squaring order written out, evaluated in IEEE 754 binary64): for
//! `1.1^7` it differs in the last bit from the left-to-right product, so a different order fails here.

use alice_det_math::powi64;

/// 2^k as an f64, from its bits (normal and subnormal)
fn two_to(k: i32) -> f64 {
    if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else {
        f64::from_bits(1u64 << (k + 1074))
    }
}

#[test]
fn a_power_of_two_is_exact_from_the_smallest_subnormal_to_the_largest_finite() {
    let mut compared = 0;
    for k in -1074..=1023 {
        assert_eq!(powi64(2.0, k).to_bits(), two_to(k).to_bits(), "2^{k}");
        compared += 1;
    }
    assert_eq!(compared, 2098);
}

#[test]
fn small_integer_powers_are_exact() {
    for (x, n, want) in [
        (3.0, 4, 81.0),
        (-2.0, 3, -8.0),
        (-2.0, 4, 16.0),
        (1.5, 2, 2.25),
        (10.0, 3, 1000.0),
        (7.0, 1, 7.0),
        (-1.0, i32::MAX, -1.0),
        (-1.0, i32::MIN, 1.0),
        (1.0, i32::MIN, 1.0),
    ] {
        assert_eq!(powi64(x, n).to_bits(), f64::to_bits(want), "{x}^{n}");
    }
}

#[test]
fn the_special_values() {
    // n == 0 is 1 for every base, NaN included
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, 3.5] {
        assert_eq!(powi64(x, 0).to_bits(), 1.0f64.to_bits(), "{x}^0");
    }
    // a NaN base is the canonical NaN
    let payload = f64::from_bits(0x7ff8_0000_0000_0abc);
    assert_eq!(powi64(payload, 3).to_bits(), f64::NAN.to_bits());
    assert_eq!(powi64(-payload, -2).to_bits(), f64::NAN.to_bits());
    // zero and infinity, with their signs
    assert_eq!(powi64(0.0, -1), f64::INFINITY);
    assert_eq!(powi64(-0.0, -1), f64::NEG_INFINITY);
    assert_eq!(powi64(-0.0, 3).to_bits(), (-0.0f64).to_bits());
    assert_eq!(powi64(-0.0, 2).to_bits(), 0.0f64.to_bits());
    assert_eq!(powi64(f64::INFINITY, -1).to_bits(), 0.0f64.to_bits());
    assert_eq!(powi64(f64::NEG_INFINITY, 3), f64::NEG_INFINITY);
    // overflow and underflow
    assert_eq!(powi64(2.0, 1024), f64::INFINITY);
    assert_eq!(powi64(2.0, -1075).to_bits(), 0.0f64.to_bits());
    assert_eq!(powi64(0.5, i32::MIN), f64::INFINITY);
}

#[test]
fn the_multiplication_order_is_pinned() {
    // 1.1^7: base 1.1, bits 111: acc = 1.1; base = 1.1·1.1; acc ·= base; base ·= base; acc ·= base
    assert_eq!(powi64(1.1, 7).to_bits(), 0x3fff_2df1_fb5a_7ed7);
    // the left-to-right product lands one ulp away, so the order is what this pins
    assert_eq!(
        (1.1f64 * 1.1 * 1.1 * 1.1 * 1.1 * 1.1 * 1.1).to_bits(),
        0x3fff_2df1_fb5a_7ed8
    );
    // 1.1^-5: base 1/1.1, bits 101
    assert_eq!(powi64(1.1, -5).to_bits(), 0x3fe3_de96_64fd_b32c);
}
