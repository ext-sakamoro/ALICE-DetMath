//! Closed-form oracles for the metric kernels (`metric` module).
//!
//! Written before the implementation: every expected value here comes from a
//! derivation or an independent brute-force reference, never from calling the
//! function under test.
//!
//! # Oracles used
//!
//! * `‖(1,1,1)‖₁ = 3`, `‖(1,1,1)‖₂ = √3`, `‖(1,1,1)‖∞ = 1` — definition.
//! * Norm equivalence in 3D: `‖x‖∞ ≤ ‖x‖₂ ≤ ‖x‖₁`, `‖x‖₁ ≤ √3‖x‖₂`,
//!   `‖x‖₂ ≤ √3‖x‖∞` — standard, tight at `(1,1,1)` / `e₁`.
//! * Lipschitz constant of `g(h) = w₁‖h‖₁ + w₂‖h‖₂ + w∞‖h‖∞` with respect to
//!   the Euclidean metric. `g` is positively homogeneous of degree 1, so
//!   `Lip(g) = max_{‖h‖₂=1} g(h)`. Ordering `h` descending and taking it
//!   non-negative (all three norms are symmetric under sign and permutation)
//!   turns `g − w₂` into the linear functional `⟨u, h⟩` with
//!   `u = (w₁+w∞, w₁, w₁)`, whose maximum over the unit sphere is `‖u‖₂` and
//!   is attained at `u/‖u‖₂`, which is itself descending and non-negative:
//!
//!   ```text
//!   Lip(w) = √((w₁ + w∞)² + 2·w₁²) + w₂
//!   ```
//!
//! * The minimum of the same functional is *not* at `−u/‖u‖₂` (outside the
//!   sorted non-negative cone), so it sits on an extreme ray of that cone —
//!   `(1,0,0)`, `(1,1,0)/√2`, `(1,1,1)/√3`:
//!
//!   ```text
//!   min_{‖h‖₂=1} g(h) = min_{k∈{1,2,3}} (k·w₁ + w∞)/√k + w₂
//!   ```
//!
//!   A ball of radius `r` in the metric therefore reaches Euclidean radius
//!   `r / min`, which is the factor an axis-aligned bound must grow by.
//! * Both closed forms are checked against a brute-force sweep over a dense
//!   integer direction grid (an independent reference that shares no code
//!   with the implementation).

use alice_det_math::metric::{
    clamp, lerp, norm_l1, norm_l2, norm_linf, smoothstep, MetricError, MetricWeights,
};

const SQRT2: f32 = 1.414_213_562_373_095_1;
const SQRT3: f32 = 1.732_050_807_568_877_2;

/// Every integer direction in `[-N, N]³` except the origin, normalised.
/// Contains the extreme rays `(1,0,0)`, `(1,1,0)`, `(1,1,1)` exactly.
fn directions(n: i32) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for i in -n..=n {
        for j in -n..=n {
            for k in -n..=n {
                if i == 0 && j == 0 && k == 0 {
                    continue;
                }
                let v = [i as f32, j as f32, k as f32];
                let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                out.push([v[0] / len, v[1] / len, v[2] / len]);
            }
        }
    }
    out
}

/// Brute-force reference: evaluates the weighted norm directly from the
/// definition, with no call into the implementation under test.
fn reference_g(w: (f32, f32, f32), h: [f32; 3]) -> f32 {
    let l1 = h[0].abs() + h[1].abs() + h[2].abs();
    let l2 = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
    let linf = h[0].abs().max(h[1].abs()).max(h[2].abs());
    w.0 * l1 + w.1 * l2 + w.2 * linf
}

#[test]
fn basis_norms_match_their_definition() {
    let v = [1.0, 1.0, 1.0];
    assert_eq!(norm_l1(v), 3.0);
    assert_eq!(norm_l2(v), SQRT3);
    assert_eq!(norm_linf(v), 1.0);

    let e1 = [1.0, 0.0, 0.0];
    assert_eq!(norm_l1(e1), 1.0);
    assert_eq!(norm_l2(e1), 1.0);
    assert_eq!(norm_linf(e1), 1.0);

    // sign symmetry is exact, not approximate
    assert_eq!(norm_l1([-2.0, 3.0, -4.0]), 9.0);
    assert_eq!(norm_linf([-2.0, 3.0, -4.0]), 4.0);
}

#[test]
fn norm_equivalence_inequalities_hold_and_are_tight() {
    for v in directions(6) {
        let (l1, l2, li) = (norm_l1(v), norm_l2(v), norm_linf(v));
        assert!(li <= l2 + 1e-6, "linf ≤ l2 failed for {v:?}");
        assert!(l2 <= l1 + 1e-6, "l2 ≤ l1 failed for {v:?}");
        assert!(l1 <= SQRT3 * l2 + 1e-6, "l1 ≤ √3·l2 failed for {v:?}");
        assert!(l2 <= SQRT3 * li + 1e-6, "l2 ≤ √3·linf failed for {v:?}");
    }
    // tightness: (1,1,1)/√3 saturates l1 ≤ √3·l2, e₁ saturates l2 ≤ l1
    let d = [1.0 / SQRT3, 1.0 / SQRT3, 1.0 / SQRT3];
    assert!((norm_l1(d) - SQRT3).abs() < 1e-6);
    assert!((norm_linf(d) - 1.0 / SQRT3).abs() < 1e-6);
}

#[test]
fn pure_basis_weights_have_the_derived_lipschitz_and_minimum() {
    // L1: max ‖h‖₁ on the unit sphere = √3, min = 1
    assert!((MetricWeights::L1.lipschitz() - SQRT3).abs() < 1e-6);
    assert!((MetricWeights::L1.minimum() - 1.0).abs() < 1e-6);
    // L2: the Euclidean metric is exactly 1-Lipschitz in both directions
    assert_eq!(MetricWeights::L2.lipschitz(), 1.0);
    assert_eq!(MetricWeights::L2.minimum(), 1.0);
    // L∞: max = 1, min = 1/√3 → a metric ball reaches √3 times its radius
    assert!((MetricWeights::LINF.lipschitz() - 1.0).abs() < 1e-6);
    assert!((MetricWeights::LINF.minimum() - 1.0 / SQRT3).abs() < 1e-6);
    assert!((MetricWeights::LINF.euclidean_radius(1.0) - SQRT3).abs() < 1e-5);
}

#[test]
fn lipschitz_is_an_upper_bound_and_is_attained() {
    let grid = directions(9);
    for (a, b, c) in [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 1.0),
        (1.0, 1.0, 1.0),
        (2.0, 1.0, 0.0),
        (0.0, 1.0, 3.0),
        (1.0, 0.0, 4.0),
        (5.0, 2.0, 1.0),
    ] {
        let w = MetricWeights::new(a, b, c).expect("non-negative weights are a metric");
        let (n1, n2, ni) = w.weights();
        let mut worst = f32::NEG_INFINITY;
        let mut best = f32::INFINITY;
        for h in &grid {
            let g = reference_g((n1, n2, ni), *h);
            worst = worst.max(g);
            best = best.min(g);
        }
        let lip = w.lipschitz();
        let min = w.minimum();
        // upper / lower bound: the closed forms must never be violated
        assert!(
            worst <= lip + 1e-5,
            "sampled max {worst} exceeds closed form {lip} for {a},{b},{c}"
        );
        assert!(
            best >= min - 1e-5,
            "sampled min {best} below closed form {min} for {a},{b},{c}"
        );
        // attainment: the grid gets within 1% of both extremes
        assert!(
            (worst - lip).abs() < 0.01 * lip,
            "closed form {lip} not attained (sampled {worst}) for {a},{b},{c}"
        );
        assert!(
            (best - min).abs() < 0.01 * min,
            "closed form {min} not attained (sampled {best}) for {a},{b},{c}"
        );
    }
}

#[test]
fn weighted_norm_never_exceeds_its_lipschitz_times_the_euclidean_length() {
    for (a, b, c) in [(1.0, 0.0, 0.0), (1.0, 2.0, 3.0), (0.0, 0.0, 1.0)] {
        let w = MetricWeights::new(a, b, c).unwrap();
        for scale in [0.001_f32, 1.0, 1000.0] {
            for h in directions(5) {
                let v = [h[0] * scale, h[1] * scale, h[2] * scale];
                let n = w.norm(v);
                let e = norm_l2(v);
                assert!(n <= w.lipschitz() * e * (1.0 + 1e-5), "{v:?} {n} {e}");
                assert!(n >= w.minimum() * e * (1.0 - 1e-5), "{v:?} {n} {e}");
            }
        }
    }
}

#[test]
fn pure_l2_weights_reproduce_the_euclidean_norm_bit_for_bit() {
    for h in directions(4) {
        for scale in [0.5_f32, 3.0, 17.25] {
            let v = [h[0] * scale, h[1] * scale, h[2] * scale];
            assert_eq!(
                MetricWeights::L2.norm(v).to_bits(),
                norm_l2(v).to_bits(),
                "pure L2 must not perturb the Euclidean field"
            );
        }
    }
}

#[test]
fn normalised_metric_is_one_lipschitz() {
    // dividing by the Lipschitz constant is the correction that keeps a field
    // from over-reporting distance (the failure mode that tears a surface)
    for (a, b, c) in [(1.0, 0.0, 0.0), (3.0, 1.0, 2.0), (0.0, 0.0, 1.0)] {
        let w = MetricWeights::new(a, b, c).unwrap().normalised();
        assert!(
            w.lipschitz() <= 1.0 + 1e-6,
            "normalised metric must be 1-Lipschitz, got {}",
            w.lipschitz()
        );
        // and it stays a metric: still non-negative weights summing to > 0
        let (n1, n2, ni) = w.weights();
        assert!(n1 >= 0.0 && n2 >= 0.0 && ni >= 0.0);
        assert!(n1 + n2 + ni > 0.0);
    }
}

#[test]
fn non_convex_or_degenerate_weights_are_rejected() {
    // a negative weight makes the unit ball non-convex: not a metric, the
    // triangle inequality fails, and the caller must be told rather than
    // silently handed a field that tears
    assert_eq!(
        MetricWeights::new(-0.1, 1.0, 0.0),
        Err(MetricError::NotConvex)
    );
    assert_eq!(
        MetricWeights::new(0.0, 0.0, 0.0),
        Err(MetricError::Degenerate)
    );
    assert_eq!(
        MetricWeights::new(f32::NAN, 1.0, 0.0),
        Err(MetricError::NotFinite)
    );
    assert_eq!(
        MetricWeights::new(f32::INFINITY, 1.0, 0.0),
        Err(MetricError::NotFinite)
    );
}

#[test]
fn triangle_inequality_holds_for_every_admissible_weight() {
    let w = MetricWeights::new(1.0, 1.0, 1.0).unwrap();
    let pts = directions(3);
    for x in pts.iter().take(200) {
        for y in pts.iter().skip(37).take(120) {
            let sum = [x[0] + y[0], x[1] + y[1], x[2] + y[2]];
            assert!(
                w.norm(sum) <= w.norm(*x) + w.norm(*y) + 1e-5,
                "triangle inequality violated"
            );
        }
    }
}

#[test]
fn lerp_is_exact_at_both_endpoints() {
    // the skin of a blended region must leave the outside field untouched:
    // t = 0 has to return `a` bit for bit, t = 1 has to return `b`
    for (a, b) in [
        (0.0_f32, 1.0_f32),
        (-3.25, 7.5),
        (1e-7, 1e7),
        (123.456, -0.000_1),
    ] {
        assert_eq!(lerp(a, b, 0.0).to_bits(), a.to_bits());
        assert_eq!(lerp(a, b, 1.0).to_bits(), b.to_bits());
        // midpoint of a symmetric pair is exact too
        assert_eq!(lerp(-1.0, 1.0, 0.5), 0.0);
    }
}

#[test]
fn clamp_saturates_and_passes_nan_through() {
    assert_eq!(clamp(-5.0, 0.0, 1.0), 0.0);
    assert_eq!(clamp(5.0, 0.0, 1.0), 1.0);
    assert_eq!(clamp(0.25, 0.0, 1.0), 0.25);
    assert!(clamp(f32::NAN, 0.0, 1.0).is_nan());
    // inverted bounds are the caller's error, not a panic: low wins
    assert_eq!(clamp(0.5, 1.0, 0.0), 1.0);
}

#[test]
fn smoothstep_matches_the_cubic_and_is_flat_at_both_edges() {
    // oracle: 3t² − 2t³ with t = clamp((x−e0)/(e1−e0), 0, 1)
    for i in 0..=100 {
        let x = i as f32 / 100.0;
        let t = x;
        let want = 3.0 * t * t - 2.0 * t * t * t;
        let got = smoothstep(0.0, 1.0, x);
        assert!((got - want).abs() < 1e-6, "x={x} got={got} want={want}");
    }
    assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
    assert_eq!(smoothstep(0.0, 1.0, -10.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 10.0), 1.0);
    assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);

    // C¹ at the edges: the finite-difference slope vanishes as h → 0
    let h = 1e-3_f32;
    let slope_lo = (smoothstep(0.0, 1.0, h) - smoothstep(0.0, 1.0, 0.0)) / h;
    let slope_hi = (smoothstep(0.0, 1.0, 1.0) - smoothstep(0.0, 1.0, 1.0 - h)) / h;
    assert!(slope_lo < 0.01, "slope at low edge {slope_lo}");
    assert!(slope_hi < 0.01, "slope at high edge {slope_hi}");

    // monotone over the transition
    let mut prev = f32::NEG_INFINITY;
    for i in 0..=200 {
        let x = -0.5 + i as f32 / 100.0;
        let v = smoothstep(0.0, 1.0, x);
        assert!(v >= prev - 1e-7, "not monotone at {x}");
        prev = v;
    }
}

#[test]
fn smoothstep_max_slope_is_the_analytic_three_halves_over_width() {
    // d/dx of 3t²−2t³ peaks at t = 1/2 with value 3/2, so over a skin of
    // width W the weight field has gradient at most 1.5/W — this is the
    // number a blended field's tension is measured against
    for width in [0.25_f32, 1.0, 4.0] {
        let h = width / 4096.0;
        let mut max_slope = 0.0_f32;
        for i in 0..4096 {
            let x = i as f32 * h;
            let s = (smoothstep(0.0, width, x + h) - smoothstep(0.0, width, x)) / h;
            max_slope = max_slope.max(s);
        }
        let want = 1.5 / width;
        assert!(
            (max_slope - want).abs() < 0.01 * want,
            "width={width} measured={max_slope} analytic={want}"
        );
    }
}

#[test]
fn smoothstep_is_symmetric_about_the_midpoint() {
    for i in 0..=100 {
        let t = i as f32 / 100.0;
        let a = smoothstep(0.0, 1.0, t);
        let b = smoothstep(0.0, 1.0, 1.0 - t);
        assert!((a + b - 1.0).abs() < 1e-6, "t={t} {a} {b}");
    }
}

#[test]
fn degenerate_skin_width_is_a_step_not_a_division_by_zero() {
    // W = 0 is the "no skin" case: below the edge 0, at or above it 1
    assert_eq!(smoothstep(1.0, 1.0, 0.5), 0.0);
    assert_eq!(smoothstep(1.0, 1.0, 1.0), 1.0);
    assert_eq!(smoothstep(1.0, 1.0, 1.5), 1.0);
    assert!(smoothstep(1.0, 1.0, 0.5).is_finite());
}

#[test]
fn sqrt2_and_sqrt3_constants_used_by_the_oracles_are_right() {
    // guards the test's own constants (a wrong oracle is worse than no oracle)
    assert!((SQRT2 * SQRT2 - 2.0).abs() < 1e-6);
    assert!((SQRT3 * SQRT3 - 3.0).abs() < 1e-6);
}

#[test]
fn every_nan_these_kernels_create_is_the_canonical_one() {
    // `inf/inf` and `0·inf` raise the *default* NaN, whose sign bit is +
    // on aarch64 / wasm32 and − on x86_64. The golden hashes in
    // `tests/golden.rs` are recorded once and checked on every target, so a
    // generated NaN that keeps the hardware's sign splits them. Inputs that
    // are themselves NaN are excluded: those arms pass the payload through,
    // which every target does identically.
    let vals = [
        0.0_f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e30,
        -1.0e30,
    ];
    let weights = [
        MetricWeights::L1,
        MetricWeights::L2,
        MetricWeights::LINF,
        MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
    ];
    let canonical = f32::NAN.to_bits();
    let mut generated = 0usize;
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                let mut results = vec![smoothstep(a, b, c), lerp(a, b, c)];
                for w in weights {
                    results.push(w.norm([a, b, c]));
                }
                for r in results {
                    if r.is_nan() {
                        generated += 1;
                        assert_eq!(
                            r.to_bits(),
                            canonical,
                            "non-canonical NaN {:#010x} from ({a}, {b}, {c})",
                            r.to_bits()
                        );
                    }
                }
            }
        }
    }
    // the sweep has to actually reach the NaN-producing cases, or it proves
    // nothing
    assert!(generated > 100, "only {generated} NaN results exercised");
}
