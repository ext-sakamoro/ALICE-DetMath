//! `simd::*` must be lane-for-lane bit-identical to the scalar functions —
//! over dense sweeps, the special values (±0, ±inf, NaN, subnormals, the
//! `exp` overflow / underflow edges, `|x| ≥ 2^31` where `as i32` saturates)
//! and a pseudo-random spray of every bit pattern class.

#![cfg(feature = "simd")]
#![allow(clippy::cast_precision_loss)]

use alice_det_math::simd;
use wide::f32x8;

fn lanes(v: f32x8) -> [f32; 8] {
    v.to_array()
}

fn check1(name: &str, f_scalar: impl Fn(f32) -> f32, f_simd: impl Fn(f32x8) -> f32x8, xs: &[f32]) {
    for chunk in xs.chunks(8) {
        let mut arr = [0.0f32; 8];
        arr[..chunk.len()].copy_from_slice(chunk);
        let got = lanes(f_simd(f32x8::new(arr)));
        for (i, &x) in arr.iter().enumerate() {
            let want = f_scalar(x);
            assert_eq!(
                got[i].to_bits(),
                want.to_bits(),
                "{name}({x:e} = {:#010x}): simd {:#010x} ({}) vs scalar {:#010x} ({})",
                x.to_bits(),
                got[i].to_bits(),
                got[i],
                want.to_bits(),
                want
            );
        }
    }
}

fn special_values() -> Vec<f32> {
    let mut v = vec![
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-40,
        -1.0e-40,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        0.5,
        -0.5,
        0.499_999_97,
        1.5,
        2.5,
        -2.5,
        1.0,
        -1.0,
        core::f32::consts::PI,
        -core::f32::consts::PI,
        core::f32::consts::FRAC_PI_2,
        core::f32::consts::FRAC_PI_4,
        88.722_84,
        88.722_85,
        88.8,
        -103.972_08,
        -103.972_09,
        -104.0,
        -87.0,
        -100.0,
        8_388_607.5,
        8_388_608.0,
        16_777_216.0,
        2_147_483_648.0,
        -2_147_483_648.0,
        4_294_967_296.0,
        1.0e10,
        -1.0e10,
        3.4e38,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
    ];
    // every exponent, a few mantissas each
    for e in 0u32..=255 {
        for m in [0u32, 1, 0x40_0000, 0x7f_ffff] {
            v.push(f32::from_bits(e << 23 | m));
            v.push(f32::from_bits(0x8000_0000 | e << 23 | m));
        }
    }
    v
}

fn sweep(lo: f32, hi: f32, n: u32) -> Vec<f32> {
    (0..=n)
        .map(|i| lo + (hi - lo) * (i as f32 / n as f32))
        .collect()
}

/// xorshift32 over all bit patterns (NaN payloads included).
fn spray(n: usize, seed: u32) -> Vec<f32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            f32::from_bits(s)
        })
        .collect()
}

fn all_inputs() -> Vec<f32> {
    let mut v = special_values();
    v.extend(sweep(-100.0, 100.0, 200_000));
    v.extend(sweep(-1.0, 1.0, 50_000));
    v.extend(sweep(-1000.0, 1000.0, 50_000));
    v.extend(spray(200_000, 0x9e37_79b9));
    v
}

#[test]
fn round_matches_scalar_bit_for_bit() {
    check1("round", alice_det_math::round, simd::round, &all_inputs());
}

#[test]
fn sqrt_matches_scalar_bit_for_bit() {
    check1("sqrt", alice_det_math::sqrt, simd::sqrt, &all_inputs());
}

#[test]
fn sin_cos_match_scalar_bit_for_bit() {
    let xs = all_inputs();
    check1("sin", alice_det_math::sin, simd::sin, &xs);
    check1("cos", alice_det_math::cos, simd::cos, &xs);
    check1(
        "sin_cos.0",
        |x| alice_det_math::sin_cos(x).0,
        |v| simd::sin_cos(v).0,
        &xs,
    );
    check1(
        "sin_cos.1",
        |x| alice_det_math::sin_cos(x).1,
        |v| simd::sin_cos(v).1,
        &xs,
    );
}

#[test]
fn exp_matches_scalar_bit_for_bit() {
    let mut xs = all_inputs();
    xs.extend(sweep(-104.0, 89.0, 200_000));
    check1("exp", alice_det_math::exp, simd::exp, &xs);
}

#[test]
fn ln_matches_scalar_bit_for_bit() {
    let mut xs = all_inputs();
    xs.extend((0..=200_000u32).map(|i| 10f32.powf(-40.0 + 80.0 * (i as f32 / 200_000.0))));
    check1("ln", alice_det_math::ln, simd::ln, &xs);
}

#[test]
fn per_lane_wrappers_match_scalar() {
    let xs: Vec<f32> = special_values()
        .into_iter()
        .chain(sweep(-4.0, 4.0, 4000))
        .collect();
    check1("atan", alice_det_math::atan, simd::atan, &xs);
    check1("asin", alice_det_math::asin, simd::asin, &xs);
    check1("acos", alice_det_math::acos, simd::acos, &xs);
    check1("tan", alice_det_math::tan, simd::tan, &xs);
    check1("tanh", alice_det_math::tanh, simd::tanh, &xs);
    check1("cbrt", alice_det_math::cbrt, simd::cbrt, &xs);
    let ys = f32x8::new([0.37, -1.13, 0.0, -0.0, 3.0, f32::INFINITY, f32::NAN, 1.0e30]);
    let xv = f32x8::new([1.13, 0.37, -1.0, -1.0, 4.0, 1.0, 1.0, 1.0e-30]);
    let a = lanes(simd::atan2(ys, xv));
    let h = lanes(simd::hypot(ys, xv));
    let p = lanes(simd::powf(xv.abs(), ys));
    for i in 0..8 {
        let (y, x) = (lanes(ys)[i], lanes(xv)[i]);
        assert_eq!(
            a[i].to_bits(),
            alice_det_math::atan2(y, x).to_bits(),
            "atan2 lane {i}"
        );
        assert_eq!(
            h[i].to_bits(),
            alice_det_math::hypot(y, x).to_bits(),
            "hypot lane {i}"
        );
        assert_eq!(
            p[i].to_bits(),
            alice_det_math::powf(x.abs(), y).to_bits(),
            "powf lane {i}"
        );
    }
}
