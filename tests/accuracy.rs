//! Accuracy of every function against a correctly rounded reference: the
//! platform `f64` libm (≤ 1 ulp of 53 bits) rounded once to `f32`. Comparing
//! against the platform `f32` libm directly would make the bound itself
//! platform-dependent (MSVC `cbrtf` and macOS `cbrtf` disagree by 1 ulp).
//!
//! These are the *oracle* tests (the value is right); `golden.rs` pins the
//! bits (the value is the same everywhere).

#![allow(clippy::cast_precision_loss, clippy::cast_lossless)]

use alice_det_math::*;

fn ulp_diff32(a: f32, b: f32) -> u32 {
    if a == b {
        return 0;
    }
    let ia = a.to_bits() as i32;
    let ib = b.to_bits() as i32;
    (ia - ib).unsigned_abs()
}

fn ulp_diff64(a: f64, b: f64) -> u64 {
    if a == b {
        return 0;
    }
    let ia = a.to_bits() as i64;
    let ib = b.to_bits() as i64;
    (ia - ib).unsigned_abs()
}

fn r32(f: impl Fn(f64) -> f64) -> impl Fn(f32) -> f32 {
    move |x| f(f64::from(x)) as f32
}

/// Dense sweep helper: max ulp distance to the reference over `n` points in [lo, hi].
fn sweep32(lo: f32, hi: f32, n: u32, f: impl Fn(f32) -> f32, g: impl Fn(f32) -> f32) -> u32 {
    let mut worst = 0;
    for i in 0..=n {
        let x = lo + (hi - lo) * (i as f32 / n as f32);
        let a = f(x);
        let b = g(x);
        assert!(
            a.is_finite() == b.is_finite(),
            "finite mismatch at {x}: {a} vs {b}"
        );
        if a.is_finite() {
            worst = worst.max(ulp_diff32(a, b));
        }
    }
    worst
}

#[test]
fn sin_cos_within_2_ulp_of_correctly_rounded() {
    assert!(sweep32(-100.0, 100.0, 400_000, sin, r32(f64::sin)) <= 2);
    assert!(sweep32(-100.0, 100.0, 400_000, cos, r32(f64::cos)) <= 2);
    assert_eq!(sin(0.0), 0.0);
    assert_eq!(cos(0.0), 1.0);
    assert!(sin(f32::INFINITY).is_nan());
}

#[test]
fn sin_cos_pair_is_bit_identical_to_separate_calls() {
    for i in 0..=400_000u32 {
        let x = -100.0 + 200.0 * (i as f32 / 400_000.0);
        let (s, c) = sin_cos(x);
        assert_eq!(s.to_bits(), sin(x).to_bits(), "sin({x})");
        assert_eq!(c.to_bits(), cos(x).to_bits(), "cos({x})");
    }
    let (s, c) = sin_cos(f32::NAN);
    assert!(s.is_nan() && c.is_nan());
}

#[test]
fn atan_asin_acos_tan_tanh_within_1_ulp_of_correctly_rounded() {
    assert!(sweep32(-1e4, 1e4, 400_000, atan, r32(f64::atan)) <= 1);
    assert!(sweep32(-4.0, 4.0, 400_000, atan, r32(f64::atan)) <= 1);
    assert!(sweep32(-1.0, 1.0, 400_000, asin, r32(f64::asin)) <= 1);
    {
        let mut worst = 0;
        let mut wx = 0.0f32;
        for i in 0..=400_000u32 {
            let x = -1.0 + 2.0 * (i as f32 / 400_000.0);
            let d = ulp_diff32(acos(x), (f64::from(x)).acos() as f32);
            if d > worst {
                worst = d;
                wx = x;
            }
        }
        assert!(
            worst <= 1,
            "acos worst {worst} ulp at {wx}: {} vs {}",
            acos(wx),
            (f64::from(wx)).acos() as f32
        );
    }
    assert!(sweep32(-100.0, 100.0, 400_000, tan, r32(f64::tan)) <= 1);
    assert!(sweep32(-12.0, 12.0, 400_000, tanh, r32(f64::tanh)) <= 1);
    assert!(sweep32(-1e-3, 1e-3, 100_000, tanh, r32(f64::tanh)) <= 1);
    // atan2: every quadrant, axes, mixed magnitudes
    let mut worst = 0;
    for iy in -200i32..=200 {
        for ix in -200i32..=200 {
            let (y, x) = (iy as f32 * 0.37, ix as f32 * 1.13);
            let a = atan2(y, x);
            let b = (f64::from(y)).atan2(f64::from(x)) as f32;
            worst = worst.max(ulp_diff32(a, b));
        }
    }
    assert!(worst <= 1, "atan2 worst {worst} ulp");
    for &(y, x) in &[
        (0.0f32, 1.0f32),
        (0.0, -1.0),
        (-0.0, -1.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (1e30, 1e-30),
        (1e-30, -1e30),
    ] {
        let a = atan2(y, x);
        let b = (f64::from(y)).atan2(f64::from(x)) as f32;
        assert!(
            a.to_bits() == b.to_bits() || ulp_diff32(a, b) <= 1,
            "atan2({y}, {x}) = {a} vs {b}"
        );
    }
    assert!(asin(1.5).is_nan() && acos(-1.5).is_nan() && tan(f32::INFINITY).is_nan());
    assert_eq!(asin(1.0), core::f32::consts::FRAC_PI_2);
    assert_eq!(acos(1.0), 0.0);
    assert_eq!(tanh(0.0), 0.0);
    assert_eq!(tanh(50.0), 1.0);
    assert_eq!(tanh(-50.0), -1.0);
    // double-precision entry points against the platform libm (≤ 1 ulp slack, see crate docs)
    let mut worst64 = 0;
    for i in 0..=200_000 {
        let x = -1e6 + 2e6 * (i as f64 / 200_000.0);
        worst64 = worst64.max(ulp_diff64(atan64(x), x.atan()));
    }
    assert!(worst64 <= 1, "atan64 worst {worst64} ulp");
}

#[test]
fn exp_within_2_ulp_of_correctly_rounded() {
    assert!(sweep32(-87.0, 88.0, 400_000, exp, r32(f64::exp)) <= 2);
    assert_eq!(exp(0.0), 1.0);
    assert_eq!(exp(100.0), f32::INFINITY);
    assert_eq!(exp(-200.0), 0.0);
    // subnormal tail is gradual
    assert!(exp(-100.0) > 0.0 && exp(-100.0) < f32::MIN_POSITIVE);
}

#[test]
fn ln_within_1_ulp_of_correctly_rounded() {
    // log-spaced sweep
    let mut worst = 0;
    for i in 0..=200_000u32 {
        let x = 10f32.powf(-30.0 + 60.0 * (i as f32 / 200_000.0));
        worst = worst.max(ulp_diff32(ln(x), r32(f64::ln)(x)));
    }
    assert!(worst <= 1, "ln worst ulp {worst}");
    assert_eq!(ln(1.0), 0.0);
    assert_eq!(ln(0.0), f32::NEG_INFINITY);
    assert!(ln(-1.0).is_nan());
    // subnormal input
    assert!(ulp_diff32(ln(1.0e-40), r32(f64::ln)(1.0e-40)) <= 1);
}

#[test]
fn cbrt_within_1_ulp_of_correctly_rounded() {
    let mut worst = 0;
    for i in 0..=200_000u32 {
        let x = 10f32.powf(-30.0 + 60.0 * (i as f32 / 200_000.0));
        worst = worst.max(ulp_diff32(cbrt(x), r32(f64::cbrt)(x)));
        worst = worst.max(ulp_diff32(cbrt(-x), r32(f64::cbrt)(-x)));
    }
    assert!(worst <= 1, "cbrt worst ulp {worst}");
    assert_eq!(cbrt(0.0), 0.0);
    assert_eq!(cbrt(8.0), 2.0);
    assert_eq!(cbrt(-27.0), -3.0);
    assert!(ulp_diff32(cbrt(1.0e-40), r32(f64::cbrt)(1.0e-40)) <= 1);
}

#[test]
fn hypot_matches_sqrt_formula_and_scales() {
    assert_eq!(hypot(3.0, 4.0), 5.0);
    assert_eq!(hypot(0.0, 0.0), 0.0);
    assert_eq!(hypot(-3.0, 4.0), 5.0);
    // libm hypot is correctly rounded; ours is sqrt(x²+y²) → ≤ 1 ulp apart
    let mut worst = 0;
    for i in 0..=100_000u32 {
        let x = 1.0e-3 + 1.0e3 * (i as f32 / 100_000.0);
        let y = 1.0e3 - x;
        worst = worst.max(ulp_diff32(
            hypot(x, y),
            (f64::from(x).hypot(f64::from(y))) as f32,
        ));
    }
    assert!(worst <= 1, "hypot worst ulp {worst}");
    // no overflow / underflow where the naive formula would fail
    assert!(hypot(1.0e30, 1.0e30).is_finite());
    assert!(hypot(1.0e-30, 1.0e-30) > 0.0);
}

#[test]
fn powf_within_1_ulp_of_correctly_rounded() {
    let mut worst = 0;
    for i in 0..=400u32 {
        let x = 10f32.powf(-3.0 + 6.0 * (i as f32 / 400.0));
        for j in 0..=160u32 {
            let y = -8.0 + 16.0 * (j as f32 / 160.0);
            worst = worst.max(ulp_diff32(
                powf(x, y),
                (f64::from(x).powf(f64::from(y))) as f32,
            ));
        }
    }
    assert!(worst <= 1, "powf worst ulp {worst}");
    assert_eq!(powf(2.0, 0.0), 1.0);
    assert_eq!(powf(0.0, 2.0), 0.0);
    assert_eq!(powf(0.0, -1.0), f32::INFINITY);
    assert!(powf(-2.0, 0.5).is_nan());
}

#[test]
fn powi_matches_repeated_multiplication() {
    for n in 0..=12 {
        let mut expect = 1.0f32;
        for _ in 0..n {
            expect *= 1.7;
        }
        // repeated squaring differs from a left fold by rounding; accept ≤ 2 ulp
        assert!(ulp_diff32(powi(1.7, n), expect) <= 2, "n = {n}");
    }
    assert_eq!(powi(2.0, 10), 1024.0);
    assert_eq!(powi(2.0, -2), 0.25);
    assert_eq!(powi(5.0, 0), 1.0);
}

#[test]
fn exp64_ln64_within_2_ulp_of_libm() {
    let mut worst_e = 0;
    for i in 0..=400_000u32 {
        let x = -700.0 + 1400.0 * (f64::from(i) / 400_000.0);
        worst_e = worst_e.max(ulp_diff64(exp64(x), x.exp()));
    }
    assert!(worst_e <= 2, "exp64 worst ulp {worst_e}");
    let mut worst_l = 0;
    for i in 0..=400_000u32 {
        let x = 10f64.powf(-300.0 + 600.0 * (f64::from(i) / 400_000.0));
        worst_l = worst_l.max(ulp_diff64(ln64(x), x.ln()));
    }
    assert!(worst_l <= 2, "ln64 worst ulp {worst_l}");
    assert_eq!(exp64(0.0), 1.0);
    assert_eq!(ln64(1.0), 0.0);
    assert_eq!(exp64(1000.0), f64::INFINITY);
    assert_eq!(exp64(-800.0), 0.0);
    assert_eq!(ln64(0.0), f64::NEG_INFINITY);
    assert!(ulp_diff64(ln64(1.0e-310), (1.0e-310f64).ln()) <= 2);
}

#[test]
fn powf64_within_16_ulp_of_libm() {
    let mut worst = 0;
    for i in 0..=400u32 {
        let x = 10f64.powf(-3.0 + 6.0 * (f64::from(i) / 400.0));
        for j in 0..=160u32 {
            let y = -8.0 + 16.0 * (f64::from(j) / 160.0);
            worst = worst.max(ulp_diff64(powf64(x, y), x.powf(y)));
        }
    }
    assert!(worst <= 16, "powf64 worst ulp {worst}");
}

/// Bit-level pins carried over from `alice-physics` 1.2.0 `det_math`: the
/// extraction must not have changed a single bit (the physics golden
/// scenarios depend on it).
#[test]
fn bit_exact_pins() {
    assert_eq!(sin(1.0f32).to_bits(), 0x3f57_6aa5);
    assert_eq!(cos(1.0f32).to_bits(), 0x3f0a_5140);
    assert_eq!(exp(1.0f32).to_bits(), 0x402d_f854);
    assert_eq!(ln(10.0f32).to_bits(), 0x4013_5d8e);
    assert_eq!(cbrt(10.0f32).to_bits(), 0x4009_e242);
    assert_eq!(powf(3.0f32, 2.5).to_bits(), 0x4179_6a52);
    assert_eq!(hypot(1.5f32, 2.5).to_bits(), 0x403a_9728);
    assert_eq!(exp64(1.0f64).to_bits(), 0x4005_bf0a_8b14_576a);
    assert_eq!(ln64(10.0f64).to_bits(), 0x4002_6bb1_bbb5_5516);
    assert_eq!(powf64(3.0f64, 2.5).to_bits(), 0x402f_2d4a_4563_563d);
}

/// A NaN input must produce a platform-independent NaN: either the canonical
/// `f32::NAN` / `f64::NAN` bits or (for the pass-through functions `round`
/// and `cbrt`) the input bits unchanged. Letting a NaN *propagate* through
/// arithmetic is not allowed — which payload / sign survives `1.0 - NaN`
/// differs between aarch64, x86 and wasm (found by the wasm golden lane).
#[test]
fn nan_inputs_yield_canonical_or_pass_through_nan() {
    let payloads32 = [
        0x7fc0_0001u32,
        0xffc0_0000,
        0x7f80_0001, // signalling
        0xffbf_ffff,
        0x7ffd_2ca3,
    ];
    let canon = f32::NAN.to_bits();
    for &b in &payloads32 {
        let x = f32::from_bits(b);
        let same = |name: &str, v: f32| {
            assert_eq!(
                v.to_bits(),
                canon,
                "{name}({b:#010x}) = {:#010x}",
                v.to_bits()
            );
        };
        same("sin", sin(x));
        same("cos", cos(x));
        same("sin_cos.0", sin_cos(x).0);
        same("sin_cos.1", sin_cos(x).1);
        same("exp", exp(x));
        same("ln", ln(x));
        same("powf", powf(x, 2.0));
        same("powf", powf(2.0, x));
        same("powi", powi(x, 3));
        same("powi", powi(x, -2));
        assert_eq!(powi(x, 0), 1.0);
        same("hypot", hypot(x, 1.0));
        same("hypot", hypot(1.0, x));
        same("atan", atan(x));
        same("atan2", atan2(x, 1.0));
        same("atan2", atan2(1.0, x));
        same("asin", asin(x));
        same("acos", acos(x));
        same("tan", tan(x));
        same("tanh", tanh(x));
        same("sqrt", sqrt(x));
        assert_eq!(round(x).to_bits(), b, "round passes NaN through");
        assert_eq!(cbrt(x).to_bits(), b, "cbrt passes NaN through");
    }
    let canon64 = f64::NAN.to_bits();
    for &b in &[
        0x7ff8_0000_0000_0001u64,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0x7ffd_2ca3_de58_4b97,
    ] {
        let x = f64::from_bits(b);
        for (name, v) in [
            ("atan64", atan64(x)),
            ("atan2_64", atan2_64(x, 1.0)),
            ("atan2_64", atan2_64(1.0, x)),
            ("asin64", asin64(x)),
            ("acos64", acos64(x)),
            ("exp64", exp64(x)),
            ("ln64", ln64(x)),
            ("powf64", powf64(x, 2.0)),
            ("powf64", powf64(2.0, x)),
            ("sqrt64", sqrt64(x)),
        ] {
            assert_eq!(
                v.to_bits(),
                canon64,
                "{name}({b:#018x}) = {:#018x}",
                v.to_bits()
            );
        }
        assert_eq!(round64(x).to_bits(), b, "round64 passes NaN through");
    }
}
