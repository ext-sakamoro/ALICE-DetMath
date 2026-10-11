//! `sin` / `cos` near their zeros (`x` within 64 ulp of `k·π/2`).
//!
//! The result there is `±sin(r)` for a tiny reduced argument `r = x − k·π/2`,
//! so every error of the reduction is an error of the same relative size in the
//! result. A sweep over the whole domain never lands on these points: the fuzz
//! target found `cos(0xc280ce28)` (≈ −20.5π) 6 ulp off when a dense uniform
//! sweep of `[-100, 100]` found nothing above 2 ulp.
//!
//! The reference is the platform `f64` libm rounded once to `f32`, as in
//! `accuracy.rs` (near these points the extra 29 bits leave no doubt about the
//! rounded value). The bound is the documented 2 ulp, in the documented domain
//! `|x| ≤ 100` (`k ≤ 63`) and in the measured extension `|x| ≤ 8192`
//! (`k ≤ 5215`).

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use alice_det_math::{cos, sin, sin_cos};

/// ulp distance that is right across zero and across signs
fn ulps(a: f32, b: f32) -> u32 {
    if a == b {
        return 0;
    }
    let key = |v: f32| {
        let i = v.to_bits() as i32;
        if i < 0 {
            i32::MIN - i
        } else {
            i
        }
    };
    (i64::from(key(a)) - i64::from(key(b))).unsigned_abs() as u32
}

/// every `f32` within `n` ulp of `k·π/2` for `0 < |k| ≤ kmax`
fn near_zeros(kmax: i64, n: i32) -> Vec<f32> {
    let mut out = Vec::new();
    for k in -kmax..=kmax {
        if k == 0 {
            continue;
        }
        let x0 = (k as f64 * core::f64::consts::FRAC_PI_2) as f32;
        for d in -n..=n {
            let x = f32::from_bits((i64::from(x0.to_bits()) + i64::from(d)) as u32);
            if x.is_finite() {
                out.push(x);
            }
        }
    }
    out
}

/// worst ulp of `f` against the reference, the input where it is, and how many points
fn worst(xs: &[f32], f: impl Fn(f32) -> f32, reference: impl Fn(f64) -> f64) -> (u32, f32, usize) {
    let mut w = (0, 0.0, 0);
    for &x in xs {
        let u = ulps(f(x), reference(f64::from(x)) as f32);
        w.2 += 1;
        if u > w.0 {
            w = (u, x, w.2);
        }
    }
    w.2 = xs.len();
    w
}

fn check(kmax: i64) {
    let xs = near_zeros(kmax, 64);
    // both kinds of zero are in the set: sin's (even k) and cos's (odd k)
    assert!(xs.len() > 2 * 64 * 2 * kmax as usize, "{} points", xs.len());
    for (name, (u, x, n)) in [
        ("sin", worst(&xs, sin, f64::sin)),
        ("cos", worst(&xs, cos, f64::cos)),
        ("sin_cos.0", worst(&xs, |x| sin_cos(x).0, f64::sin)),
        ("sin_cos.1", worst(&xs, |x| sin_cos(x).1, f64::cos)),
    ] {
        assert!(n > 0, "{name}: nothing compared");
        assert!(u <= 2, "{name}: {u} ulp at x = {x:e} (0x{:08x}), {n} points within 64 ulp of k·π/2, |k| <= {kmax}", x.to_bits());
    }
}

#[test]
fn sin_and_cos_are_within_2_ulp_near_their_zeros_in_the_documented_domain() {
    check(63); // |x| <= 100
}

#[test]
fn sin_and_cos_are_within_2_ulp_near_their_zeros_up_to_8192() {
    check(5215); // |x| <= 8192
}

#[test]
fn the_inputs_the_fuzz_target_and_the_sweep_found() {
    for (bits, s_or_c) in [
        (0xc280_ce28_u32, 'c'),
        (0xc096_cbe4, 'c'),
        (0xc296_cbe4, 's'),
    ] {
        let x = f32::from_bits(bits);
        let (got, want) = if s_or_c == 'c' {
            (cos(x), f64::from(x).cos() as f32)
        } else {
            (sin(x), f64::from(x).sin() as f32)
        };
        assert!(
            ulps(got, want) <= 2,
            "0x{bits:08x}: {got:e} vs {want:e}: {} ulp",
            ulps(got, want)
        );
    }
}

#[cfg(feature = "simd")]
mod simd {
    use super::{near_zeros, ulps};
    use alice_det_math::{cos, simd, sin};
    use wide::f32x8;

    /// the SIMD lanes against the same reference, and bit for bit against the scalar path
    #[test]
    fn simd_sin_and_cos_are_within_2_ulp_near_their_zeros() {
        let xs = near_zeros(5215, 64);
        let mut worst = (0, 0.0_f32);
        for chunk in xs.chunks(8) {
            let mut lanes = [0.0_f32; 8];
            lanes[..chunk.len()].copy_from_slice(chunk);
            let v = f32x8::new(lanes);
            let (s, c) = simd::sin_cos(v);
            let (s1, c1) = (simd::sin(v), simd::cos(v));
            for (i, &x) in chunk.iter().enumerate() {
                let (rs, rc) = (f64::from(x).sin() as f32, f64::from(x).cos() as f32);
                assert_eq!(
                    s.to_array()[i].to_bits(),
                    sin(x).to_bits(),
                    "simd sin != scalar at 0x{:08x}",
                    x.to_bits()
                );
                assert_eq!(
                    c.to_array()[i].to_bits(),
                    cos(x).to_bits(),
                    "simd cos != scalar at 0x{:08x}",
                    x.to_bits()
                );
                for u in [
                    ulps(s.to_array()[i], rs),
                    ulps(c.to_array()[i], rc),
                    ulps(s1.to_array()[i], rs),
                    ulps(c1.to_array()[i], rc),
                ] {
                    if u > worst.0 {
                        worst = (u, x);
                    }
                }
            }
        }
        assert!(!xs.is_empty());
        assert!(
            worst.0 <= 2,
            "simd: {} ulp at x = {:e} (0x{:08x})",
            worst.0,
            worst.1,
            worst.1.to_bits()
        );
    }
}
