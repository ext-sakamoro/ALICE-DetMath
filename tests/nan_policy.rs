//! Every NaN this crate returns is the canonical one.
//!
//! The canonical NaN is the positive quiet NaN with an empty payload:
//! `0x7FC0_0000` (`f32::NAN`) and `0x7FF8_0000_0000_0000` (`f64::NAN`). A NaN
//! raised by the arithmetic itself (`0/0`, `inf − inf`, `0·inf`, `sqrt` of a
//! negative) carries the platform's default NaN instead, whose sign bit is set
//! on x86-64 and clear on `AArch64`, and a NaN argument may carry any payload.
//! Either would make the same call return different bits on different
//! targets, so every public function returns the canonical NaN whenever its
//! result is NaN.
//!
//! oracle: the bit patterns above, over every public scalar function at every
//! special argument (`common::specials`), and every SIMD lane.

mod common;

use common::specials::{visit, Sink, SPECIALS32};

type F1 = (&'static str, fn(f32) -> f32);
type F2 = (&'static str, fn(f32, f32) -> f32);
type F3 = (&'static str, fn(f32, f32, f32) -> f32);
type D1 = (&'static str, fn(f64) -> f64);

const NAN32: u32 = 0x7fc0_0000;
const NAN64: u64 = 0x7ff8_0000_0000_0000;

#[test]
fn the_canonical_nans_are_the_standard_constants() {
    assert_eq!(f32::NAN.to_bits(), NAN32);
    assert_eq!(f64::NAN.to_bits(), NAN64);
}

#[derive(Default)]
struct NanCheck {
    name: &'static str,
    nans: usize,
    values: usize,
    bad: Vec<String>,
}

impl Sink for NanCheck {
    fn name(&mut self, name: &'static str) {
        self.name = name;
    }
    fn v32(&mut self, v: f32) {
        self.values += 1;
        if v.is_nan() {
            self.nans += 1;
            if v.to_bits() != NAN32 {
                self.bad
                    .push(format!("{}: {:#010x}", self.name, v.to_bits()));
            }
        }
    }
    fn v64(&mut self, v: f64) {
        self.values += 1;
        if v.is_nan() {
            self.nans += 1;
            if v.to_bits() != NAN64 {
                self.bad
                    .push(format!("{}: {:#018x}", self.name, v.to_bits()));
            }
        }
    }
}

#[test]
fn every_nan_a_public_function_returns_is_canonical() {
    let mut c = NanCheck::default();
    visit(&mut c, alice_det_math::sin);
    assert!(
        c.values > 30_000 && c.nans > 1000,
        "{} values, {} NaN",
        c.values,
        c.nans
    );
    let mut by_fn: Vec<&str> = c.bad.iter().map(|b| b.split(':').next().unwrap()).collect();
    by_fn.dedup();
    assert!(
        c.bad.is_empty(),
        "{} non-canonical NaN in {by_fn:?}; first: {:?}",
        c.bad.len(),
        c.bad.first()
    );
}

/// The norm of a metric with a zero weight at an infinite vector forms `0·inf`,
/// which raises the target's default NaN (negative on x86-64): the named
/// metrics `L1` / `L2` / `LINF` each have two zero weights.
#[test]
fn a_zero_weight_times_an_infinite_norm_is_canonical() {
    use alice_det_math::metric::MetricWeights;
    let mut n = 0;
    for w in [MetricWeights::L1, MetricWeights::L2, MetricWeights::LINF] {
        for &x in &SPECIALS32 {
            for &y in &SPECIALS32 {
                for &z in &SPECIALS32 {
                    let v = w.norm([x, y, z]);
                    if v.is_nan() {
                        n += 1;
                        assert_eq!(v.to_bits(), NAN32, "{w:?}.norm([{x:e}, {y:e}, {z:e}])");
                    }
                }
            }
        }
    }
    assert!(n > 100, "{n} NaN");
}

/// A NaN argument with a payload or the sign bit set comes back canonical,
/// from every one-argument function (and the two- and three-argument ones
/// with the NaN in each position, the other arguments `1`).
#[test]
fn a_nan_argument_with_any_payload_comes_back_canonical() {
    use alice_det_math::*;
    let payloads32 = [
        0x7fc0_0001u32,
        0xffc0_0000,
        0xffc0_0001,
        0x7f80_0001,
        0xff80_0001,
        0x7fff_ffff,
    ];
    let payloads64 = [
        0x7ff8_0000_0000_0001u64,
        0xfff8_0000_0000_0000,
        0xfff8_0000_0000_0001,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0001,
        0x7fff_ffff_ffff_ffff,
    ];
    let one32: [F1; 17] = [
        ("sin", sin),
        ("cos", cos),
        ("sin_cos.0", |x| sin_cos(x).0),
        ("sin_cos.1", |x| sin_cos(x).1),
        ("exp", exp),
        ("ln", ln),
        ("log2", log2),
        ("log10", log10),
        ("cbrt", cbrt),
        ("atan", atan),
        ("asin", asin),
        ("acos", acos),
        ("tan", tan),
        ("tanh", tanh),
        ("round", round),
        ("sqrt", sqrt),
        ("powi", |x| powi(x, 3)),
    ];
    let two32: [F2; 5] = [
        ("powf", powf),
        ("hypot", hypot),
        ("atan2", atan2),
        ("max", alice_det_math::max),
        ("min", alice_det_math::min),
    ];
    let three32: [F3; 3] = [
        ("lerp", metric::lerp),
        ("clamp", metric::clamp),
        ("smoothstep", metric::smoothstep),
    ];
    let one64: [D1; 15] = [
        ("sin64", sin64),
        ("cos64", cos64),
        ("sin_cos64.0", |x| sin_cos64(x).0),
        ("sin_cos64.1", |x| sin_cos64(x).1),
        ("tan64", tan64),
        ("atan64", atan64),
        ("asin64", asin64),
        ("acos64", acos64),
        ("exp64", exp64),
        ("ln64", ln64),
        ("log2_64", log2_64),
        ("log10_64", log10_64),
        ("round64", round64),
        ("sqrt64", sqrt64),
        ("powf64.y", |y| powf64(2.0, y)),
    ];
    let mut bad = Vec::new();
    let mut n = 0;
    for b in payloads32 {
        let x = f32::from_bits(b);
        let mut see = |name: &str, v: f32| {
            n += 1;
            if v.is_nan() && v.to_bits() != NAN32 {
                bad.push(format!("{name}({b:#010x}) = {:#010x}", v.to_bits()));
            }
        };
        for (name, f) in one32 {
            see(name, f(x));
        }
        for (name, f) in two32 {
            see(name, f(x, 1.0));
            see(name, f(1.5, x));
        }
        for (name, f) in three32 {
            see(name, f(x, 1.0, 0.5));
            see(name, f(0.5, x, 1.0));
            see(name, f(0.0, 1.0, x));
        }
        let w = metric::MetricWeights::new(0.3, 0.5, 0.2).unwrap();
        for v in [[x, 1.0, 1.0], [1.0, x, 1.0], [1.0, 1.0, x]] {
            see("metric_norm", w.norm(v));
            see("norm_l1", metric::norm_l1(v));
            see("norm_l2", metric::norm_l2(v));
            see("norm_linf", metric::norm_linf(v));
        }
        see("axis_extent", w.axis_extent(x));
        see("euclidean_radius", w.euclidean_radius(x));
    }
    for b in payloads64 {
        let x = f64::from_bits(b);
        for (name, f) in one64 {
            n += 1;
            let v = f(x);
            if v.is_nan() && v.to_bits() != NAN64 {
                bad.push(format!("{name}({b:#018x}) = {:#018x}", v.to_bits()));
            }
        }
        for (name, v) in [
            ("atan2_64.y", atan2_64(x, 1.0)),
            ("atan2_64.x", atan2_64(1.0, x)),
            ("powf64.x", powf64(x, 2.0)),
            ("max64.y", alice_det_math::max64(1.5, x)),
            ("max64.x", alice_det_math::max64(x, 1.5)),
            ("min64.y", alice_det_math::min64(1.5, x)),
            ("min64.x", alice_det_math::min64(x, 1.5)),
        ] {
            n += 1;
            if v.is_nan() && v.to_bits() != NAN64 {
                bad.push(format!("{name}({b:#018x}) = {:#018x}", v.to_bits()));
            }
        }
    }
    assert!(n > 300, "compared {n}");
    assert!(bad.is_empty(), "{} non-canonical: {bad:#?}", bad.len());
}

#[cfg(feature = "simd")]
mod simd {
    use super::{NAN32, SPECIALS32};
    use alice_det_math::simd;
    use wide::f32x8;

    type L1 = (&'static str, fn(f32x8) -> f32x8);
    type L2 = (&'static str, fn(f32x8, f32x8) -> f32x8);
    type L3 = (&'static str, fn(f32x8, f32x8, f32x8) -> f32x8);

    fn lanes(xs: &[f32]) -> Vec<f32x8> {
        xs.chunks(8)
            .map(|c| {
                let mut a = [0.0; 8];
                a[..c.len()].copy_from_slice(c);
                f32x8::new(a)
            })
            .collect()
    }

    #[test]
    fn every_nan_a_simd_lane_returns_is_canonical() {
        let one: [L1; 13] = [
            ("round", simd::round),
            ("sqrt", simd::sqrt),
            ("sin", simd::sin),
            ("cos", simd::cos),
            ("sin_cos.0", |x| simd::sin_cos(x).0),
            ("sin_cos.1", |x| simd::sin_cos(x).1),
            ("exp", simd::exp),
            ("ln", simd::ln),
            ("atan", simd::atan),
            ("asin", simd::asin),
            ("acos", simd::acos),
            ("tan", simd::tan),
            ("tanh", simd::tanh),
        ];
        let cbrt: L1 = ("cbrt", simd::cbrt);
        let two: [L2; 3] = [
            ("atan2", simd::atan2),
            ("powf", simd::powf),
            ("hypot", simd::hypot),
        ];
        let three: [L3; 3] = [
            ("lerp", simd::lerp),
            ("clamp", simd::clamp),
            ("smoothstep", simd::smoothstep),
        ];
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut zs = Vec::new();
        // the specials plus NaNs with a payload or the sign bit
        let mut args = SPECIALS32.to_vec();
        args.extend([0x7fc0_0001u32, 0xffc0_0000, 0x7f80_0001, 0xffc0_0001].map(f32::from_bits));
        for &x in &args {
            for &y in &args {
                for &z in &args {
                    xs.push(x);
                    ys.push(y);
                    zs.push(z);
                }
            }
        }
        let (vx, vy, vz) = (lanes(&xs), lanes(&ys), lanes(&zs));
        let (mut nans, mut bad) = (0usize, Vec::new());
        let mut see = |name: &str, out: f32x8| {
            for v in out.to_array() {
                if v.is_nan() {
                    nans += 1;
                    if v.to_bits() != NAN32 {
                        bad.push(format!("{name}: {:#010x}", v.to_bits()));
                    }
                }
            }
        };
        for i in 0..vx.len() {
            for (n, f) in one.iter().chain([&cbrt]) {
                see(n, f(vx[i]));
            }
            for (n, f) in &two {
                see(n, f(vx[i], vy[i]));
            }
            for (n, f) in &three {
                see(n, f(vx[i], vy[i], vz[i]));
            }
            let w = simd::MetricWeightsX8::splat(
                alice_det_math::metric::MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
            );
            see("metric_norm", simd::metric_norm(w, vx[i], vy[i], vz[i]));
        }
        assert!(nans > 1000, "{nans} NaN lanes");
        let mut by_fn: Vec<&str> = bad.iter().map(|b| b.split(':').next().unwrap()).collect();
        by_fn.dedup();
        assert!(
            bad.is_empty(),
            "{} non-canonical NaN lanes in {by_fn:?}; first {:?}",
            bad.len(),
            bad.first()
        );
    }
}
