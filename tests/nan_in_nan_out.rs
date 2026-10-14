//! A NaN in any argument gives the canonical NaN, whatever its position.
//!
//! IEEE 754 propagates a NaN operand through every arithmetic operation; a
//! function built from comparisons (`if a > b { a } else { b }`) does not — a
//! comparison with NaN is false, so the NaN is kept in one position and
//! dropped in another. `norm_linf([NaN, 1, 1])` returned `1` while
//! `norm_linf([1, 1, NaN])` returned NaN. The exceptions are the IEEE 754
//! special cases whose value does not depend on the NaN argument:
//! `pow(+1, NaN) = 1`, `pow(NaN, ±0) = 1` and `hypot(±inf, NaN) = +inf`.
//!
//! oracle: the rule above, every multi-argument public function, the NaN in
//! each position and the other arguments over the IEEE special values.

mod common;

use alice_det_math::metric::{self, MetricWeights};
use alice_det_math::{atan2, atan2_64, hypot, max, max64, min, min64, powf, powf64};
use common::specials::{SPECIALS32, SPECIALS64};

const NAN32: u32 = 0x7fc0_0000;
const NAN64: u64 = 0x7ff8_0000_0000_0000;

/// an IEEE exception: the value is defined whatever the NaN argument is
fn exempt(name: &str, args: &[f64]) -> bool {
    match name {
        "powf" | "powf64" => args[0] == 1.0 || args[1] == 0.0,
        "hypot" => args.iter().any(|a| a.is_infinite()),
        // a NaN operand yields the *other* operand: the result is NaN (and
        // must be canonical) only when both are NaN; with exactly one NaN
        // the defined value is the other, non-NaN operand, so this file's
        // "must be canonical NaN" check does not apply (that value is
        // checked by tests/golden.rs's mutation tests instead)
        "max" | "min" | "max64" | "min64" => !(args[0].is_nan() && args[1].is_nan()),
        _ => false,
    }
}

struct Check {
    bad: Vec<String>,
    n: usize,
}

impl Check {
    fn f32(&mut self, name: &str, args: &[f32], v: f32) {
        let a: Vec<f64> = args.iter().map(|&x| f64::from(x)).collect();
        if exempt(name, &a) {
            return;
        }
        self.n += 1;
        if v.to_bits() != NAN32 {
            self.bad
                .push(format!("{name}{args:?} = {v:?} ({:#010x})", v.to_bits()));
        }
    }
    fn f64(&mut self, name: &str, args: &[f64], v: f64) {
        if exempt(name, args) {
            return;
        }
        self.n += 1;
        if v.to_bits() != NAN64 {
            self.bad.push(format!("{name}{args:?} = {v:?}"));
        }
    }
}

#[test]
fn a_nan_in_any_position_gives_the_canonical_nan() {
    let mut c = Check {
        bad: Vec::new(),
        n: 0,
    };
    let nan = f32::NAN;
    let w = MetricWeights::new(0.3, 0.5, 0.2).unwrap();
    for &p in &SPECIALS32 {
        for &q in &SPECIALS32 {
            for pos in 0..3 {
                let mut v = [p, q, q];
                v[pos] = nan;
                c.f32("norm_l1", &v, metric::norm_l1(v));
                c.f32("norm_l2", &v, metric::norm_l2(v));
                c.f32("norm_linf", &v, metric::norm_linf(v));
                c.f32("metric_norm", &v, w.norm(v));
                for named in [MetricWeights::L1, MetricWeights::L2, MetricWeights::LINF] {
                    c.f32("metric_norm(named)", &v, named.norm(v));
                }
                c.f32("lerp", &v, metric::lerp(v[0], v[1], v[2]));
                c.f32("clamp", &v, metric::clamp(v[0], v[1], v[2]));
                c.f32("smoothstep", &v, metric::smoothstep(v[0], v[1], v[2]));
            }
            for pos in 0..2 {
                let mut v = [p, q];
                v[pos] = nan;
                c.f32("powf", &v, powf(v[0], v[1]));
                c.f32("hypot", &v, hypot(v[0], v[1]));
                c.f32("atan2", &v, atan2(v[0], v[1]));
                c.f32("max", &v, max(v[0], v[1]));
                c.f32("min", &v, min(v[0], v[1]));
            }
        }
        c.f32("axis_extent", &[nan], w.axis_extent(nan));
        c.f32("euclidean_radius", &[nan], w.euclidean_radius(nan));
    }
    for &p in &SPECIALS64 {
        for pos in 0..2 {
            let mut v = [p, p];
            v[pos] = f64::NAN;
            c.f64("powf64", &v, powf64(v[0], v[1]));
            c.f64("atan2_64", &v, atan2_64(v[0], v[1]));
            c.f64("max64", &v, max64(v[0], v[1]));
            c.f64("min64", &v, min64(v[0], v[1]));
        }
    }
    assert!(c.n > 5000, "compared {}", c.n);
    let mut fns: Vec<&str> = c.bad.iter().map(|b| b.split('[').next().unwrap()).collect();
    fns.sort_unstable();
    fns.dedup();
    assert!(
        c.bad.is_empty(),
        "{} cases in {fns:?}; first {:?}",
        c.bad.len(),
        c.bad.iter().take(3).collect::<Vec<_>>()
    );
}

#[cfg(feature = "simd")]
mod simd {
    use super::{exempt, NAN32};
    use crate::common::specials::SPECIALS32;
    use alice_det_math::simd;
    use wide::f32x8;

    type L2 = (&'static str, fn(f32x8, f32x8) -> f32x8);
    type L3 = (&'static str, fn(f32x8, f32x8, f32x8) -> f32x8);

    /// the SIMD multi-argument functions, the same rule per lane
    #[test]
    fn a_nan_in_any_lane_argument_gives_the_canonical_nan() {
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
        let w = simd::MetricWeightsX8::splat(
            alice_det_math::metric::MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
        );
        let (mut n, mut bad) = (0, Vec::new());
        for &p in &SPECIALS32 {
            for pos in 0..3 {
                let mut a = [f32x8::splat(p); 3];
                a[pos] = f32x8::splat(f32::NAN);
                let args = [
                    if pos == 0 { f64::NAN } else { f64::from(p) },
                    if pos == 1 { f64::NAN } else { f64::from(p) },
                ];
                let mut see = |name: &str, v: f32x8, two_arg: bool| {
                    if two_arg && (pos == 2 || exempt(name, &args)) {
                        return;
                    }
                    n += 1;
                    if v.to_array()[0].to_bits() != NAN32 {
                        bad.push(format!(
                            "simd {name} p={p:e} pos={pos}: {:?}",
                            v.to_array()[0]
                        ));
                    }
                };
                for (name, f) in two {
                    see(name, f(a[0], a[1]), true);
                }
                for (name, f) in three {
                    see(name, f(a[0], a[1], a[2]), false);
                }
                see("metric_norm", simd::metric_norm(w, a[0], a[1], a[2]), false);
            }
        }
        assert!(n > 100, "{n}");
        assert!(
            bad.is_empty(),
            "{} cases; {:?}",
            bad.len(),
            bad.iter().take(6).collect::<Vec<_>>()
        );
    }
}
