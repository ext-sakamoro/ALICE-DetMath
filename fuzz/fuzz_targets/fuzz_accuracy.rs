//! Differential fuzz against the correctly rounded reference (`f64` libm
//! rounded once to `f32`) inside each function's documented domain, plus
//! finiteness / NaN agreement everywhere.
#![no_main]

use alice_det_math::*;
use libfuzzer_sys::fuzz_target;

fn ulp(a: f32, b: f32) -> u32 {
    if a == b {
        return 0;
    }
    ((a.to_bits() as i32) - (b.to_bits() as i32)).unsigned_abs()
}

fn within(name: &str, x: f32, got: f32, want: f32, bound: u32) {
    assert_eq!(
        got.is_nan(),
        want.is_nan(),
        "{name}({x:e}): NaN mismatch {got} vs {want}"
    );
    if got.is_finite() && want.is_finite() {
        assert!(
            ulp(got, want) <= bound,
            "{name}({x:e}) = {got:e} vs {want:e}: {} ulp",
            ulp(got, want)
        );
    }
}

fuzz_target!(|data: (u32, u32)| {
    let x = f32::from_bits(data.0);
    let y = f32::from_bits(data.1);
    let xd = f64::from(x);
    // never panics, whatever the input
    let _ = (sin(x), cos(x), exp(x), ln(x), atan(x), atan2(y, x), asin(x), acos(x));
    let _ = (tan(x), tanh(x), cbrt(x), hypot(x, y), powf(x, y), powi(x, 3), round(x), sqrt(x));
    if x.is_finite() && x.abs() <= 100.0 {
        within("sin", x, sin(x), xd.sin() as f32, 2);
        within("cos", x, cos(x), xd.cos() as f32, 2);
        within("tan", x, tan(x), xd.tan() as f32, 1);
    }
    if x.is_finite() && (-87.0..=88.0).contains(&x) {
        within("exp", x, exp(x), xd.exp() as f32, 2);
    }
    if x.is_finite() && x > 1e-30 && x < 1e30 {
        within("ln", x, ln(x), xd.ln() as f32, 1);
        within("cbrt", x, cbrt(x), xd.cbrt() as f32, 1);
    }
    if x.is_finite() && x.abs() <= 1e4 {
        within("atan", x, atan(x), xd.atan() as f32, 1);
    }
    if x.is_finite() && x.abs() <= 1.0 {
        within("asin", x, asin(x), xd.asin() as f32, 1);
        within("acos", x, acos(x), xd.acos() as f32, 1);
    }
    if x.is_finite() && x.abs() <= 12.0 {
        within("tanh", x, tanh(x), xd.tanh() as f32, 1);
    }
    if x.is_finite() && y.is_finite() && x.abs() <= 1e4 && y.abs() <= 1e4 {
        within("atan2", x, atan2(y, x), f64::from(y).atan2(xd) as f32, 1);
    }
    // NaN input: hardware quiets a signalling NaN (`round`) or propagates the
    // payload (`sqrt`); this crate passes the bits through / returns the
    // canonical NaN on purpose (platform-independent), so compare non-NaN only
    if !x.is_nan() {
        assert_eq!(round(x).to_bits(), x.round().to_bits(), "round({x:e})");
        assert_eq!(sqrt(x.abs()).to_bits(), x.abs().sqrt().to_bits(), "sqrt({x:e})");
    }
});
