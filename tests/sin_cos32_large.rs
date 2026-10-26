//! `sin` / `cos` / `sin_cos` (binary32) for `|x| ≥ 2^13`, where the argument is
//! reduced in double precision.
//!
//! oracle: correctly rounded values from an independent 600-bit evaluation
//! (`mpmath`, `scripts/gen_sin_cos32_large_reference.py`), not from any libm.
//! The bound is 1 ulp for every row; the table must not be short (a comparison
//! over no rows passes nothing).

use alice_det_math::{cos, sin, sin_cos};

const REFERENCE: &str = include_str!("data/sin_cos32_large_reference.txt");

fn rows() -> Vec<(f32, f32, f32)> {
    REFERENCE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let v: Vec<f32> = l
                .split_whitespace()
                .map(|h| f32::from_bits(u32::from_str_radix(h, 16).expect("hex")))
                .collect();
            assert_eq!(v.len(), 3, "row {l}");
            (v[0], v[1], v[2])
        })
        .collect()
}

fn ulp_diff(a: f32, b: f32) -> u32 {
    if a.to_bits() == b.to_bits() {
        return 0;
    }
    if a.is_sign_negative() != b.is_sign_negative() {
        // only a ±0 pair could be close across the sign, and no row here is 0
        return u32::MAX;
    }
    (a.to_bits() as i32 - b.to_bits() as i32).unsigned_abs()
}

#[test]
fn large_arguments_within_1_ulp_of_correctly_rounded() {
    let rows = rows();
    assert!(rows.len() > 10_000, "reference has {} rows", rows.len());
    let (mut ws, mut wc) = ((0u32, 0.0f32), (0u32, 0.0f32));
    let mut exact = 0usize;
    for &(x, s, c) in &rows {
        assert!(x.abs() >= 8192.0, "row {x:e} is below the switch");
        let (ds, dc) = (ulp_diff(sin(x), s), ulp_diff(cos(x), c));
        if ds > ws.0 {
            ws = (ds, x);
        }
        if dc > wc.0 {
            wc = (dc, x);
        }
        if ds == 0 && dc == 0 {
            exact += 1;
        }
        // sin_cos is the pair of the separate calls, bit for bit
        let (ps, pc) = sin_cos(x);
        assert_eq!(ps.to_bits(), sin(x).to_bits(), "sin_cos({x:e}).0");
        assert_eq!(pc.to_bits(), cos(x).to_bits(), "sin_cos({x:e}).1");
    }
    println!(
        "binary32 sin / cos at |x| ≥ 2^13: max {} / {} ulp over {} rows ({exact} both exact), at {:e} / {:e}",
        ws.0,
        wc.0,
        rows.len(),
        ws.1,
        wc.1
    );
    assert!(ws.0 <= 1, "sin({:e}) is {} ulp off", ws.1, ws.0);
    assert!(wc.0 <= 1, "cos({:e}) is {} ulp off", wc.1, wc.0);
}

/// Every lane of the `f32x8` functions equals the scalar function, including
/// lanes on both sides of the switch and non-finite lanes in the same vector.
#[cfg(feature = "simd")]
#[test]
fn simd_lanes_match_scalar_across_the_switch() {
    use alice_det_math::simd;
    use wide::f32x8;
    let rows = rows();
    let mut compared = 0usize;
    for chunk in rows.chunks(4) {
        let mut lanes = [
            100.0f32,
            -8191.5,
            f32::INFINITY,
            f32::NAN,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        for (i, r) in chunk.iter().enumerate() {
            lanes[4 + i] = r.0;
        }
        let v = f32x8::new(lanes);
        let (s, c) = (simd::sin(v).to_array(), simd::cos(v).to_array());
        let (ps, pc) = simd::sin_cos(v);
        let (ps, pc) = (ps.to_array(), pc.to_array());
        for (i, &x) in lanes.iter().enumerate() {
            assert_eq!(
                s[i].to_bits(),
                sin(x).to_bits(),
                "simd sin lane {i} ({x:e})"
            );
            assert_eq!(
                c[i].to_bits(),
                cos(x).to_bits(),
                "simd cos lane {i} ({x:e})"
            );
            assert_eq!(ps[i].to_bits(), sin(x).to_bits(), "simd sin_cos.0 lane {i}");
            assert_eq!(pc[i].to_bits(), cos(x).to_bits(), "simd sin_cos.1 lane {i}");
            compared += 1;
        }
    }
    assert!(compared > 20_000, "compared {compared} lanes");
}
