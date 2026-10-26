//! `hypot` of two subnormal operands against an exact integer oracle.
//!
//! A subnormal `f32` is `a·2⁻¹⁴⁹` with an integer `0 < a < 2²³`, so
//! `hypot(a·2⁻¹⁴⁹, b·2⁻¹⁴⁹) = √(a² + b²)·2⁻¹⁴⁹`. The result is below `2⁻¹²⁵`,
//! where the `f32` grid spacing is `2⁻¹⁴⁹` (the subnormals and the first normal
//! binade), so the correctly rounded value is `k·2⁻¹⁴⁹` with `k` the integer
//! nearest `√(a² + b²)`, and its bit pattern is `k` itself. `√n` is never
//! halfway between two integers for an integer `n`, so there is no tie to
//! break. The oracle is integer arithmetic only: no `libm`, no float reference.

use alice_det_math::hypot;

/// The integer nearest `√n`.
fn nearest_isqrt(n: u64) -> u64 {
    let mut s = (n as f64).sqrt() as u64;
    while s * s > n {
        s -= 1;
    }
    while (s + 1) * (s + 1) <= n {
        s += 1;
    }
    // round up when n − s² > s, i.e. √n > s + ½ (n is an integer, so ≥ s + 1)
    if n - s * s > s {
        s + 1
    } else {
        s
    }
}

fn check(a: u32, b: u32) {
    let (x, y) = (f32::from_bits(a), f32::from_bits(b));
    let want = nearest_isqrt(u64::from(a) * u64::from(a) + u64::from(b) * u64::from(b));
    for (sx, sy) in [(x, y), (-x, y), (x, -y), (-x, -y), (y, x)] {
        let got = hypot(sx, sy);
        assert_eq!(
            u64::from(got.to_bits()),
            want,
            "hypot({a:#010x}, {b:#010x}) = {got:e}, correctly rounded bits {want:#010x}"
        );
    }
}

#[test]
fn smallest_pair_is_the_smallest_subnormal() {
    let t = f32::from_bits(1);
    assert_eq!(hypot(t, t).to_bits(), 1);
}

#[test]
fn subnormal_pairs_are_correctly_rounded() {
    const MAX: u32 = (1 << 23) - 1;
    let mut edges: Vec<u32> = vec![
        1,
        2,
        3,
        4,
        5,
        7,
        8,
        1 << 11,
        1 << 12,
        (1 << 22) - 1,
        1 << 22,
        MAX - 1,
        MAX,
    ];
    edges.extend((0..23).map(|e| 1u32 << e));
    let mut compared = 0u64;
    for &a in &edges {
        for &b in &edges {
            check(a, b);
            compared += 1;
        }
    }
    for a in (1..=MAX).step_by(4093) {
        for b in (1..=MAX).step_by(4099) {
            check(a, b);
            compared += 1;
        }
    }
    // the sweep must have compared something (a gate that compares 0 pairs passes silently)
    assert!(compared > 4_000_000, "compared only {compared} pairs");
}

#[test]
fn a_subnormal_against_zero_is_exact() {
    for a in [1u32, 2, 12345, (1 << 23) - 1] {
        let x = f32::from_bits(a);
        assert_eq!(hypot(x, 0.0).to_bits(), a);
        assert_eq!(hypot(-0.0, -x).to_bits(), a);
    }
}
