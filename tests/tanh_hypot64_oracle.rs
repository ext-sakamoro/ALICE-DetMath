//! Oracles of `tanh64` and `hypot64`. The expected values are correctly rounded references computed
//! with mpmath 1.4.1 at 200 bits (the generator is in the commit that added this file's tables),
//! never the functions' own output; the identities (odd symmetry, Pythagorean triples scaled by
//! powers of two, inf / NaN rules) are exact by construction.
//!
//! Bounds, declared before measuring: `tanh64` within 2 ULP of the correctly rounded value and
//! `hypot64` within 1 ULP.

use alice_det_math::double::{hypot64, tanh64};

// tanh: (x, correctly rounded tanh(x)), mpmath 1.4.1 at 200 bits
const TANH_REF: &[(u64, u64)] = &[
    (0x0000000000000001, 0x0000000000000001), // 5e-324
    (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359), // 1e-300
    (0x16687e92154ef7ac, 0x16687e92154ef7ac), // 1e-200
    (0x3bc79ca10c924223, 0x3bc79ca10c924223), // 1e-20
    (0x3ddb7cdfd9d7bdbb, 0x3ddb7cdfd9d7bdbb), // 1e-10
    (0x3e2ffffffffdb73d, 0x3e2ffffffffdb73d), // 3.7252902984e-09
    (0x3ee4f8b588e368f1, 0x3ee4f8b588e06854), // 1e-05
    (0x3f50624dd2f1a9fc, 0x3f50624d77516ce2), // 0.001
    (0x3fb0000000000000, 0x3faff55997e030d7), // 0.0625
    (0x3fb999999999999a, 0x3fb983d7795f413a), // 0.1
    (0x3fd0000000000000, 0x3fcf597ea69a1c86), // 0.25
    (0x3fe0000000000000, 0x3fdd9353d7568af3), // 0.5
    (0x3fe193ea7aad030b, 0x3fe0000000000000), // 0.5493061443340549
    (0x3fe8000000000000, 0x3fe45323e552f228), // 0.75
    (0x3ff0000000000000, 0x3fe85efab514f394), // 1.0
    (0x3ff8000000000000, 0x3fecf6f9786df577), // 1.5
    (0x4000000000000000, 0x3feed9505e1bc3d4), // 2.0
    (0x4008000000000000, 0x3fefd77d111a0b00), // 3.0
    (0x4014000000000000, 0x3fefff419668df11), // 5.0
    (0x401e000000000000, 0x3feffffeb78a3c73), // 7.5
    (0x4024000000000000, 0x3feffffffdc96f35), // 10.0
    (0x402e000000000000, 0x3feffffffffff96a), // 15.0
    (0x4032000000000000, 0x3feffffffffffffc), // 18.0
    (0x4032800000000000, 0x3feffffffffffffe), // 18.5
    (0x4033000000000000, 0x3fefffffffffffff), // 19.0
    (0x40330fbe76c8b439, 0x3fefffffffffffff), // 19.0615
    (0x403319999999999a, 0x3ff0000000000000), // 19.1
    (0x4034000000000000, 0x3ff0000000000000), // 20.0
    (0x4036000000000000, 0x3ff0000000000000), // 22.0
    (0x403e000000000000, 0x3ff0000000000000), // 30.0
    (0x4059000000000000, 0x3ff0000000000000), // 100.0
    (0x4086300000000000, 0x3ff0000000000000), // 710.0
    (0x7e37e43c8800759c, 0x3ff0000000000000), // 1e+300
];
// hypot: (x, y, correctly rounded sqrt(x^2 + y^2)), mpmath 1.4.1 at 200 bits
const HYPOT_REF: &[(u64, u64, u64)] = &[
    (0x4008000000000000, 0x4010000000000000, 0x4014000000000000), // (3.0, 4.0)
    (0x4014000000000000, 0x4028000000000000, 0x402a000000000000), // (5.0, 12.0)
    (0x4020000000000000, 0x402e000000000000, 0x4031000000000000), // (8.0, 15.0)
    (0x401c000000000000, 0x4038000000000000, 0x4039000000000000), // (7.0, 24.0)
    (0x4034000000000000, 0x4035000000000000, 0x403d000000000000), // (20.0, 21.0)
    (0x7e51eb2d66005835, 0x7e57e43c8800759c, 0x7e5ddd4baa009303), // (3e+300, 4e+300)
    (0x01c01297d23ab683, 0x01c56e1fc2f8f359, 0x01cac9a7b3b7302f), // (3e-300, 4e-300)
    (0x7fe1ccf385ebc8a0, 0x7fe1ccf385ebc8a0, 0x7fe92c80954c51f5), // (1e+308, 1e+308)
    (0x7fefffffffffffff, 0x7e37e43c8800759c, 0x7fefffffffffffff), // (1.7976931348623157e+308, 1e+300)
    (0x00000000000007e8, 0x00000000000007e8, 0x0000000000000b2e), // (1e-320, 1e-320)
    (0x0000000000000001, 0x0000000000000001, 0x0000000000000001), // (5e-324, 5e-324)
    (0x3ff0000000000000, 0x3bc79ca10c924223, 0x3ff0000000000000), // (1.0, 1e-20)
    (0x3ff0000000000000, 0x3ff0000000000000, 0x3ff6a09e667f3bcd), // (1.0, 1.0)
    (0x3fb999999999999a, 0x3fc999999999999a, 0x3fcc9f25c5bfedda), // (0.1, 0.2)
    (0x40fe240c9fbe76c9, 0x3f35097c80841ede, 0x40fe240c9fbe76c9), // (123456.789, 0.000321)
    (0x6570000000000000, 0x6570000000000000, 0x6576a09e667f3bcd), // (4.149515568880993e+180, 4.149515568880993e+180)
    (0x1a70000000000000, 0x1a70000000000000, 0x1a76a09e667f3bcd), // (2.409919865102884e-181, 2.409919865102884e-181)
    (0x5ff1e667904707f5, 0x5ff1e667904707f5, 0x5ff9507fa7ee4df2), // (1.5e+154, 1.5e+154)
    (0x1fe9be34ac46b18f, 0x1fe9be34ac46b18f, 0x1ff233faab66b109), // (6e-155, 6e-155)
];

/// distance in units in the last place between two finite f64 of the same sign
fn ulps(a: f64, b: f64) -> u64 {
    assert_eq!(
        a.is_sign_negative(),
        b.is_sign_negative(),
        "{a} and {b} differ in sign"
    );
    a.to_bits().abs_diff(b.to_bits())
}

#[test]
fn tanh64_is_within_2_ulp_of_the_reference() {
    let mut worst = 0;
    for &(x, want) in TANH_REF {
        let (x, want) = (f64::from_bits(x), f64::from_bits(want));
        let got = tanh64(x);
        assert!(
            ulps(got, want) <= 2,
            "tanh64({x:e}) = {got:e}, reference {want:e}, {} ulp",
            ulps(got, want)
        );
        worst = worst.max(ulps(got, want));
    }
    println!(
        "tanh64: measured max {worst} ulp over {} points",
        TANH_REF.len()
    );
}

#[test]
fn tanh64_is_odd_bit_for_bit() {
    for &(x, _) in TANH_REF {
        let x = f64::from_bits(x);
        assert_eq!(
            tanh64(-x).to_bits(),
            (-tanh64(x)).to_bits(),
            "tanh64(-{x:e})"
        );
    }
    assert_eq!(tanh64(0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(
        tanh64(-0.0).to_bits(),
        (-0.0f64).to_bits(),
        "the sign of zero is kept"
    );
}

#[test]
fn tanh64_saturates_and_keeps_the_specials() {
    // 1 - tanh(x) < 2^-54 from x = 19.0615474653985 (mpmath): the result is +-1 there and beyond
    for x in [19.1, 20.0, 100.0, 1e300, f64::MAX, f64::INFINITY] {
        assert_eq!(tanh64(x), 1.0, "tanh64({x:e})");
        assert_eq!(tanh64(-x), -1.0, "tanh64(-{x:e})");
    }
    assert!(tanh64(f64::NAN).is_nan());
    // below the saturation point the result is still below 1
    assert!(tanh64(19.0) < 1.0);
}

#[test]
fn hypot64_is_within_1_ulp_of_the_reference() {
    let mut worst = 0;
    for &(x, y, want) in HYPOT_REF {
        let (x, y, want) = (f64::from_bits(x), f64::from_bits(y), f64::from_bits(want));
        let got = hypot64(x, y);
        assert!(
            got.is_finite(),
            "hypot64({x:e}, {y:e}) = {got:e} overflowed"
        );
        assert!(
            ulps(got, want) <= 1,
            "hypot64({x:e}, {y:e}) = {got:e}, reference {want:e}, {} ulp",
            ulps(got, want)
        );
        worst = worst.max(ulps(got, want));
    }
    println!(
        "hypot64: measured max {worst} ulp over {} pairs",
        HYPOT_REF.len()
    );
}

#[test]
fn hypot64_is_exact_on_pythagorean_triples_at_every_scale() {
    for (a, b, c) in [
        (3.0, 4.0, 5.0),
        (5.0, 12.0, 13.0),
        (8.0, 15.0, 17.0),
        (7.0, 24.0, 25.0),
        (20.0, 21.0, 29.0),
    ] {
        // a power of two scales every member exactly, from the subnormals to near the top
        for k in [-1070, -600, -300, 0, 300, 600, 1000] {
            let s = 2.0f64.powi(k);
            assert_eq!(hypot64(a * s, b * s), c * s, "({a}, {b}) at 2^{k}");
        }
    }
}

#[test]
fn hypot64_symmetries_and_specials() {
    for &(x, y, _) in HYPOT_REF {
        let (x, y) = (f64::from_bits(x), f64::from_bits(y));
        let h = hypot64(x, y).to_bits();
        for (a, b) in [(y, x), (-x, y), (x, -y), (-y, -x)] {
            assert_eq!(hypot64(a, b).to_bits(), h, "hypot64({a:e}, {b:e})");
        }
    }
    assert_eq!(hypot64(-7.5, 0.0), 7.5);
    assert_eq!(hypot64(0.0, -0.0).to_bits(), 0.0f64.to_bits());
    // an infinity wins over a NaN (the IEEE 754 hypot rule)
    assert_eq!(hypot64(f64::INFINITY, f64::NAN), f64::INFINITY);
    assert_eq!(hypot64(f64::NAN, f64::NEG_INFINITY), f64::INFINITY);
    assert!(hypot64(f64::NAN, 1.0).is_nan());
}
