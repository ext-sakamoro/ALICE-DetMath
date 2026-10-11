//! Every public scalar function of the crate evaluated at the IEEE special
//! values: shared by the `specials` golden pin and the NaN policy test, so a
//! function added here is both pinned and checked.

#![allow(dead_code)] // each test binary uses part of this module

use alice_det_math::metric::{MetricError, MetricWeights};

/// The IEEE special values, signed: ±0, the smallest subnormal, the smallest
/// normal, ±1, the largest finite, ±inf and NaN.
pub const SPECIALS32: [f32; 13] = [
    0.0,
    -0.0,
    f32::from_bits(1),
    -f32::from_bits(1),
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0,
    -1.0,
    f32::MAX,
    -f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
];

pub const SPECIALS64: [f64; 13] = [
    0.0,
    -0.0,
    f64::from_bits(1),
    -f64::from_bits(1),
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
    1.0,
    -1.0,
    f64::MAX,
    -f64::MAX,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

/// Receives, in a fixed order, each function's pin name and then its outputs.
pub trait Sink {
    fn name(&mut self, name: &'static str);
    fn v32(&mut self, v: f32);
    fn v64(&mut self, v: f64);
}

/// A metric's three weights folded into one `f32` (powers of two, left to
/// right: IEEE-basic, the same on every target).
pub fn fold_weights(w: MetricWeights) -> f32 {
    let (l1, l2, linf) = w.weights();
    l1 + 2.0 * l2 + 4.0 * linf
}

fn f1(s: &mut impl Sink, name: &'static str, f: impl Fn(f32) -> f32) {
    s.name(name);
    for x in SPECIALS32 {
        s.v32(f(x));
    }
}

fn f2(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32) -> f32) {
    s.name(name);
    for x in SPECIALS32 {
        for y in SPECIALS32 {
            s.v32(f(x, y));
        }
    }
}

fn f3(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32, f32) -> f32) {
    s.name(name);
    for x in SPECIALS32 {
        for y in SPECIALS32 {
            for z in SPECIALS32 {
                s.v32(f(x, y, z));
            }
        }
    }
}

fn d1(s: &mut impl Sink, name: &'static str, f: impl Fn(f64) -> f64) {
    s.name(name);
    for x in SPECIALS64 {
        s.v64(f(x));
    }
}

fn d2(s: &mut impl Sink, name: &'static str, f: impl Fn(f64, f64) -> f64) {
    s.name(name);
    for x in SPECIALS64 {
        for y in SPECIALS64 {
            s.v64(f(x, y));
        }
    }
}

/// Every public scalar function, under its golden pin name. `sin` is passed
/// in (`sin_under_test`, a name the glob import below cannot shadow) so a
/// test can substitute a broken one and see the result move.
pub fn visit(s: &mut impl Sink, sin_under_test: fn(f32) -> f32) {
    use alice_det_math::*;
    let weights = |a: f32, b: f32, c: f32| MetricWeights::new(a.abs(), b.abs(), c.abs());
    f1(s, "sin", sin_under_test);
    f1(s, "cos", cos);
    // both halves, each in full (not folded: a NaN in one must stay visible)
    s.name("sin_cos");
    for x in SPECIALS32 {
        let (a, b) = sin_cos(x);
        s.v32(a);
        s.v32(b);
    }
    f1(s, "exp", exp);
    f1(s, "ln", ln);
    f1(s, "log2", log2);
    f1(s, "log10", log10);
    f2(s, "powf", powf);
    s.name("powi");
    for x in SPECIALS32 {
        for n in [-3, -2, -1, 0, 1, 2, 3, i32::MIN, i32::MAX] {
            s.v32(powi(x, n));
        }
    }
    f1(s, "cbrt", cbrt);
    f2(s, "hypot", hypot);
    f1(s, "atan", atan);
    f2(s, "atan2", atan2);
    f1(s, "asin", asin);
    f1(s, "acos", acos);
    f1(s, "tan", tan);
    f1(s, "tanh", tanh);
    f1(s, "round", round);
    f1(s, "sqrt", sqrt);
    f3(s, "smoothstep", metric::smoothstep);
    // a mixed metric, and the named ones (two zero weights each: `0·inf`)
    s.name("metric_norm");
    for w in [
        MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
        MetricWeights::L1,
        MetricWeights::L2,
        MetricWeights::LINF,
    ] {
        for x in SPECIALS32 {
            for y in SPECIALS32 {
                for z in SPECIALS32 {
                    s.v32(w.norm([x, y, z]));
                }
            }
        }
    }
    f3(s, "metric_lipschitz", |a, b, c| {
        weights(a, b, c).map_or(-1.0, MetricWeights::lipschitz)
    });
    f3(s, "norm_l1", |x, y, z| metric::norm_l1([x, y, z]));
    f3(s, "norm_l2", |x, y, z| metric::norm_l2([x, y, z]));
    f3(s, "norm_linf", |x, y, z| metric::norm_linf([x, y, z]));
    f3(s, "lerp", metric::lerp);
    f3(s, "clamp", metric::clamp);
    f3(s, "metric_new", |a, b, c| {
        match MetricWeights::new(a, b, c) {
            Ok(w) => fold_weights(w),
            Err(MetricError::NotFinite) => -1.0,
            Err(MetricError::NotConvex) => -2.0,
            Err(MetricError::Degenerate) => -3.0,
            Err(_) => -4.0,
        }
    });
    f3(s, "metric_weights", |a, b, c| {
        weights(a, b, c).map_or(-1.0, fold_weights)
    });
    f3(s, "metric_minimum", |a, b, c| {
        weights(a, b, c).map_or(-1.0, MetricWeights::minimum)
    });
    f3(s, "metric_axis_extent", |a, b, c| {
        weights(a, b, c).map_or(-1.0, |w| w.axis_extent(a))
    });
    f3(s, "metric_euclidean_radius", |a, b, c| {
        weights(a, b, c).map_or(-1.0, |w| w.euclidean_radius(b))
    });
    s.name("metric_normalised");
    for a in SPECIALS32 {
        for b in SPECIALS32 {
            for c in SPECIALS32 {
                match weights(a, b, c) {
                    Ok(w) => {
                        let (x, y, z) = w.normalised().weights();
                        s.v32(x);
                        s.v32(y);
                        s.v32(z);
                    }
                    Err(_) => s.v32(-1.0),
                }
            }
        }
    }
    d1(s, "atan64", atan64);
    d2(s, "atan2_64", atan2_64);
    d1(s, "asin64", asin64);
    d1(s, "acos64", acos64);
    d1(s, "exp64", exp64);
    d1(s, "ln64", ln64);
    d1(s, "log2_64", log2_64);
    d1(s, "log10_64", log10_64);
    d1(s, "tan64", tan64);
    d2(s, "powf64", powf64);
    d1(s, "round64", round64);
    d1(s, "sqrt64", sqrt64);
    d1(s, "sin64", sin64);
    d1(s, "cos64", cos64);
    s.name("sin_cos64");
    for x in SPECIALS64 {
        let (a, b) = sin_cos64(x);
        s.v64(a);
        s.v64(b);
    }
}
