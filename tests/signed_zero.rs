//! The odd functions keep the sign of zero: `f(−0) = −0`, `f(+0) = +0`.
//!
//! IEEE 754 and every libm define `sin`, `tan`, `atan`, `asin`, `tanh`, `cbrt`
//! (and `round`, `sqrt`) of `±0` as `±0`. The smallest subnormals `±2^-149`
//! are checked too: there the result is nonzero, so a sign lost at exactly
//! zero is a different failure from a sign lost by the arithmetic.
//!
//! oracle: the sign rule above, and the platform `f64` libm (which follows it)
//! for the value. The even half of `sin_cos` / `sin_cos64` is checked to be
//! exactly `1` at `±0`.

use alice_det_math as dm;

const ZEROS32: [f32; 2] = [0.0, -0.0];
const ZEROS64: [f64; 2] = [0.0, -0.0];

type Named32 = (&'static str, fn(f32) -> f32);
type Named64 = (&'static str, fn(f64) -> f64);

/// `(name, f)` for every `f32` odd function of the crate
fn odd32() -> Vec<Named32> {
    vec![
        ("sin", dm::sin),
        ("sin_cos.0", |x| dm::sin_cos(x).0),
        ("tan", dm::tan),
        ("atan", dm::atan),
        ("asin", dm::asin),
        ("tanh", dm::tanh),
        ("cbrt", dm::cbrt),
        ("round", dm::round),
        ("sqrt", dm::sqrt),
    ]
}

fn odd64() -> Vec<Named64> {
    vec![
        ("sin64", dm::sin64),
        ("sin_cos64.0", |x| dm::sin_cos64(x).0),
        ("tan64", dm::tan64),
        ("atan64", dm::atan64),
        ("asin64", dm::asin64),
        ("round64", dm::round64),
        ("sqrt64", dm::sqrt64),
    ]
}

#[test]
fn f32_odd_functions_return_the_zero_they_are_given() {
    let mut bad = Vec::new();
    let mut n = 0;
    for (name, f) in odd32() {
        for x in ZEROS32 {
            let y = f(x);
            n += 1;
            if y.to_bits() != x.to_bits() {
                bad.push(format!("{name}({x:?}) = {y:?}"));
            }
        }
    }
    assert_eq!(n, 18, "compared");
    assert!(bad.is_empty(), "sign of zero lost: {bad:?}");
}

#[test]
fn f64_odd_functions_return_the_zero_they_are_given() {
    let mut bad = Vec::new();
    let mut n = 0;
    for (name, f) in odd64() {
        for x in ZEROS64 {
            let y = f(x);
            n += 1;
            if y.to_bits() != x.to_bits() {
                bad.push(format!("{name}({x:?}) = {y:?}"));
            }
        }
    }
    assert_eq!(n, 14, "compared");
    assert!(bad.is_empty(), "sign of zero lost: {bad:?}");
}

#[test]
fn the_even_half_is_one_at_both_zeros() {
    for x in ZEROS32 {
        assert_eq!(dm::sin_cos(x).1.to_bits(), 1.0_f32.to_bits());
        assert_eq!(dm::cos(x).to_bits(), 1.0_f32.to_bits());
    }
    for x in ZEROS64 {
        assert_eq!(dm::sin_cos64(x).1.to_bits(), 1.0_f64.to_bits());
        assert_eq!(dm::cos64(x).to_bits(), 1.0_f64.to_bits());
    }
}

/// `±2^-149`: nonzero results keep the argument's sign and match libm
#[test]
fn the_smallest_subnormals_keep_their_sign() {
    let mut n = 0;
    for x in [f32::from_bits(1), -f32::from_bits(1)] {
        for (name, f) in odd32() {
            if name == "sqrt" && x < 0.0 {
                continue; // NaN, not a sign question
            }
            let y = f(x);
            n += 1;
            assert_eq!(
                y.is_sign_negative(),
                x.is_sign_negative(),
                "{name}({x:e}) = {y:e}"
            );
        }
    }
    assert_eq!(n, 17, "compared");
}

#[cfg(feature = "simd")]
mod simd {
    use alice_det_math::simd;
    use wide::f32x8;

    type NamedLanes = (&'static str, fn(f32x8) -> f32x8);

    /// every lane of the SIMD odd functions, `±0` and `±2^-149` side by side
    #[test]
    fn simd_odd_functions_keep_the_sign_of_zero() {
        let s = f32::from_bits(1);
        let v = f32x8::new([0.0, -0.0, s, -s, -0.0, 0.0, -s, s]);
        let fs: [NamedLanes; 9] = [
            ("sin", simd::sin),
            ("sin_cos.0", |x| simd::sin_cos(x).0),
            ("tan", simd::tan),
            ("atan", simd::atan),
            ("asin", simd::asin),
            ("tanh", simd::tanh),
            ("cbrt", simd::cbrt),
            ("round", simd::round),
            ("sqrt", simd::sqrt),
        ];
        let mut n = 0;
        for (name, f) in fs {
            let (i, o) = (v.to_array(), f(v).to_array());
            for k in 0..8 {
                if name == "sqrt" && i[k] < 0.0 {
                    continue;
                }
                n += 1;
                if i[k] == 0.0 {
                    assert_eq!(
                        o[k].to_bits(),
                        i[k].to_bits(),
                        "simd {name} lane {k}: {:?}",
                        o[k]
                    );
                } else {
                    assert_eq!(
                        o[k].is_sign_negative(),
                        i[k].is_sign_negative(),
                        "simd {name} lane {k}"
                    );
                }
            }
        }
        assert_eq!(n, 70, "compared");
    }
}
