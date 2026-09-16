//! Every `simd::*` lane must be bit-identical to the scalar function for any
//! 8 input bit patterns (NaN payloads, subnormals, ±inf, huge magnitudes).
#![no_main]

use alice_det_math::simd;
use libfuzzer_sys::fuzz_target;
use wide::f32x8;

fn check(name: &str, xs: [f32; 8], got: f32x8, scalar: impl Fn(f32) -> f32) {
    let got = got.to_array();
    for i in 0..8 {
        let want = scalar(xs[i]);
        assert_eq!(
            got[i].to_bits(),
            want.to_bits(),
            "{name}({:#010x}) simd {:#010x} vs scalar {:#010x}",
            xs[i].to_bits(),
            got[i].to_bits(),
            want.to_bits()
        );
    }
}

fuzz_target!(|bits: [u32; 8]| {
    let xs = bits.map(f32::from_bits);
    let v = f32x8::new(xs);
    check("round", xs, simd::round(v), alice_det_math::round);
    check("sqrt", xs, simd::sqrt(v), alice_det_math::sqrt);
    check("sin", xs, simd::sin(v), alice_det_math::sin);
    check("cos", xs, simd::cos(v), alice_det_math::cos);
    check("exp", xs, simd::exp(v), alice_det_math::exp);
    check("ln", xs, simd::ln(v), alice_det_math::ln);
    let (s, c) = simd::sin_cos(v);
    check("sin_cos.0", xs, s, |x| alice_det_math::sin_cos(x).0);
    check("sin_cos.1", xs, c, |x| alice_det_math::sin_cos(x).1);
});
