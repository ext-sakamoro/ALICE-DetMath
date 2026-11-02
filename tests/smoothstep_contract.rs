//! The `smoothstep` contract: a rising ramp needs `edge0 < edge1`, a falling ramp is
//! [`smoothstep_rev`], and an inverted interval is refused instead of being read as a step.
//!
//! GLSL leaves `edge0 >= edge1` undefined and GPUs return the reversed slope, so a shader-style
//! `smoothstep(h, h - e, x)` used to give a step here and a slope on the GPU. These tests pin the
//! refusal, the falling ramp's closed form, and that nothing else moved.

use alice_det_math::metric::{smoothstep, smoothstep_rev};

#[test]
#[should_panic(expected = "smoothstep: edge0 (1.0) must be less than edge1 (0.0)")]
fn inverted_edges_panic_naming_both_edges() {
    let _ = smoothstep(1.0, 0.0, 0.5);
}

#[test]
#[should_panic(expected = "smoothstep: edge0 (0.25) must be less than edge1 (-3.5)")]
fn inverted_edges_panic_whatever_x_is() {
    // x outside the interval on either side must not short-circuit the check
    let _ = smoothstep(0.25, -3.5, 100.0);
}

#[test]
#[should_panic(expected = "smoothstep: edge0 (inf) must be less than edge1 (-inf)")]
fn inverted_infinite_edges_panic() {
    let _ = smoothstep(f32::INFINITY, f32::NEG_INFINITY, 0.0);
}

#[test]
#[should_panic(expected = "smoothstep_rev: edge_lo (1.0) must be less than edge_hi (0.0)")]
fn smoothstep_rev_with_its_edges_swapped_panics_under_its_own_name() {
    // `smoothstep_rev(hi, lo, x)` with `hi < lo` is the same mistake the other way round
    let _ = smoothstep_rev(0.0, 1.0, 0.5);
}

#[test]
fn a_nan_argument_gives_the_canonical_nan_even_with_inverted_edges() {
    let nan = f32::NAN.to_bits();
    for (e0, e1, x) in [
        (f32::NAN, 0.0, 0.5),
        (1.0, f32::NAN, 0.5),
        (1.0, 0.0, f32::NAN),
        (0.0, 1.0, f32::NAN),
        (f32::from_bits(0xffc0_0001), 0.0, 0.5),
    ] {
        assert_eq!(smoothstep(e0, e1, x).to_bits(), nan, "{e0} {e1} {x}");
        assert_eq!(smoothstep_rev(e0, e1, x).to_bits(), nan, "rev {e0} {e1} {x}");
    }
}

/// Increasing edges are the old contract unchanged: these are the bits 0.6.1 returned.
#[test]
fn increasing_edges_keep_their_bits() {
    let cases: [(f32, f32, f32, u32); 8] = [
        (0.0, 1.0, 0.5, 0x3f00_0000),
        (0.0, 1.0, 0.25, 0x3e20_0000),
        (0.0, 1.0, -1.0, 0x0000_0000),
        (0.0, 1.0, 2.0, 0x3f80_0000),
        (-1.0, 3.0, 0.0, 0x3e20_0000),
        (2.0, 2.5, 2.1, 0x3dd4_fddb),
        (f32::NEG_INFINITY, f32::INFINITY, 0.0, 0x7fc0_0000),
        (0.0, f32::INFINITY, 1.0, 0x0000_0000),
    ];
    for (e0, e1, x, bits) in cases {
        assert_eq!(
            smoothstep(e0, e1, x).to_bits(),
            bits,
            "smoothstep({e0}, {e1}, {x}) = {}",
            smoothstep(e0, e1, x)
        );
    }
}

/// `smoothstep_rev(hi, lo, x)` is `1 − smoothstep(lo, hi, x)`, bit for bit.
#[test]
fn smoothstep_rev_is_one_minus_the_rising_ramp_bit_for_bit() {
    let mut compared = 0usize;
    let edges: [(f32, f32); 6] = [
        (0.0, 1.0),
        (-2.0, 3.0),
        (0.1, 0.100_01),
        (-1.0e-30, 1.0e-30),
        (5.0, 1.0e6),
        (f32::MIN_POSITIVE, 1.0),
    ];
    for (lo, hi) in edges {
        for i in -64..=64 {
            let x = lo + (hi - lo) * (i as f32 / 32.0);
            let want = 1.0 - smoothstep(lo, hi, x);
            assert_eq!(
                smoothstep_rev(hi, lo, x).to_bits(),
                want.to_bits(),
                "smoothstep_rev({hi}, {lo}, {x})"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 6 * 129);
}

/// The falling ramp against its closed form `1 − (3t² − 2t³)`, `t = clamp((x − lo)/(hi − lo))`,
/// evaluated in `f64`.
#[test]
fn smoothstep_rev_matches_the_closed_form() {
    let mut compared = 0usize;
    for (lo, hi) in [(0.0f32, 1.0f32), (-3.0, 5.0), (10.0, 10.5)] {
        for i in -20..=120 {
            let x = lo + (hi - lo) * (i as f32 / 100.0);
            let t = ((f64::from(x) - f64::from(lo)) / (f64::from(hi) - f64::from(lo))).clamp(0.0, 1.0);
            let want = 1.0 - t * t * (3.0 - 2.0 * t);
            let got = f64::from(smoothstep_rev(hi, lo, x));
            assert!(
                (got - want).abs() <= 4.0e-7,
                "smoothstep_rev({hi}, {lo}, {x}) = {got}, closed form {want}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 3 * 141);
    // exact at the ends and the midpoint
    assert_eq!(smoothstep_rev(1.0, 0.0, -1.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(smoothstep_rev(1.0, 0.0, 0.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(smoothstep_rev(1.0, 0.0, 0.5).to_bits(), 0.5f32.to_bits());
    assert_eq!(smoothstep_rev(1.0, 0.0, 1.0).to_bits(), 0.0f32.to_bits());
    assert_eq!(smoothstep_rev(1.0, 0.0, 2.0).to_bits(), 0.0f32.to_bits());
}

/// Golden bits of the falling ramp (recorded with this release).
#[test]
fn smoothstep_rev_golden_bits() {
    let cases: [(f32, f32, f32, u32); 5] = [
        (1.0, 0.0, 0.25, 0x3f58_0000),
        (1.0, 0.0, 0.75, 0x3e20_0000),
        (3.0, -1.0, 0.0, 0x3f58_0000),
        (2.5, 2.0, 2.1, 0x3f65_6045),
        (1.0, 0.0, 0.1, 0x3f78_d4fe),
    ];
    for (hi, lo, x, bits) in cases {
        assert_eq!(
            smoothstep_rev(hi, lo, x).to_bits(),
            bits,
            "smoothstep_rev({hi}, {lo}, {x}) = {}",
            smoothstep_rev(hi, lo, x)
        );
    }
}

/// The SIMD kernel refuses the same intervals: one inverted lane is enough.
#[cfg(feature = "simd")]
mod simd {
    use alice_det_math::simd;
    use wide::f32x8;

    #[test]
    #[should_panic(expected = "smoothstep: edge0 must be less than edge1 in every lane")]
    fn one_inverted_lane_panics() {
        let e0 = f32x8::new([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        let e1 = f32x8::new([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0]);
        let _ = simd::smoothstep(e0, e1, f32x8::splat(0.5));
    }

    #[test]
    fn a_nan_lane_is_not_an_inverted_lane() {
        let e0 = f32x8::new([f32::NAN, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let e1 = f32x8::new([0.0, f32::NAN, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        let r = simd::smoothstep(e0, e1, f32x8::splat(0.5)).to_array();
        assert_eq!(r[0].to_bits(), f32::NAN.to_bits());
        assert_eq!(r[1].to_bits(), f32::NAN.to_bits());
        assert_eq!(r[2].to_bits(), 0.5f32.to_bits());
    }
}
