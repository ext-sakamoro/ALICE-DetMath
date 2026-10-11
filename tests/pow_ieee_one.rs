//! `pow(+1, y) = 1` for every `y`, NaN included (IEEE 754 `pow`, C `pow`),
//! and `pow(x, ±0) = 1` for every `x`, NaN included.
//!
//! At `x = 1` the logarithm is exactly `0`, and forming `y · ln x` exactly
//! (Dekker's two-product) splits `y` by multiplying it by `2^27 + 1`, which
//! overflows for `|y|` near `f64::MAX` or infinite: `inf − inf` is a NaN whose
//! sign differs between targets, where the answer is exactly `1`.
//!
//! oracle: the IEEE 754 special cases above.

use alice_det_math::{powf, powf64};

const Y64: [f64; 13] = [
    0.0,
    -0.0,
    f64::from_bits(1),
    -f64::from_bits(1),
    f64::MIN_POSITIVE,
    1.0,
    -1.0,
    1.0e300,
    f64::MAX,
    -f64::MAX,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

const Y32: [f32; 13] = [
    0.0,
    -0.0,
    f32::from_bits(1),
    -f32::from_bits(1),
    f32::MIN_POSITIVE,
    1.0,
    -1.0,
    1.0e30,
    f32::MAX,
    -f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

#[test]
fn one_to_any_power_is_one() {
    for y in Y64 {
        assert_eq!(
            powf64(1.0, y).to_bits(),
            1.0_f64.to_bits(),
            "powf64(1, {y:e})"
        );
    }
    for y in Y32 {
        assert_eq!(powf(1.0, y).to_bits(), 1.0_f32.to_bits(), "powf(1, {y:e})");
    }
}

#[test]
fn anything_to_the_zeroth_power_is_one() {
    for x in Y64 {
        for y in [0.0, -0.0] {
            assert_eq!(
                powf64(x, y).to_bits(),
                1.0_f64.to_bits(),
                "powf64({x:e}, {y:e})"
            );
        }
    }
    for x in Y32 {
        for y in [0.0, -0.0] {
            assert_eq!(
                powf(x, y).to_bits(),
                1.0_f32.to_bits(),
                "powf({x:e}, {y:e})"
            );
        }
    }
}
