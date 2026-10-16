//! [`MetricWeights::lipschitz`] is the closed form for every weight triple,
//! and [`MetricWeights::new`] accepts exactly the triples whose constant is a
//! normal `f32`.
//!
//! The closed form is `sqrt((w1+winf)^2 + 2·w1^2) + w2`. Evaluated directly in
//! `f32` its radical underflows for `w1 + winf < 2^-63` (to `0` below about
//! `2^-75`) and overflows above `2^64`: `lipschitz()` then returned `0`, a
//! value with few correct bits, or `inf` for weights whose constant is an
//! ordinary `f32`, and [`MetricWeights::normalised`] divided by it (`0/0` is a
//! NaN whose sign differs between targets). The radical is now taken of the
//! weights scaled by an exact power of two outside `[2^-60, 2^60]`.
//!
//! oracle: the closed form evaluated in `f64` (no under/overflow for any `f32`
//! weights), the requirement that a normalised metric is 1-Lipschitz, and —
//! inside `[2^-60, 2^60]`, where nothing under/overflows — the previous `f32`
//! formula, which must be reproduced bit for bit.

use alice_det_math::metric::{MetricError, MetricWeights};
use alice_det_math::sqrt;

/// the closed form in `f64`
fn closed(w1: f32, w2: f32, winf: f32) -> f64 {
    let (a, b, c) = (f64::from(w1), f64::from(w2), f64::from(winf));
    ((a + c) * (a + c) + 2.0 * a * a).sqrt() + b
}

/// the formula as it was before the scaling (verbatim)
fn previous(w1: f32, w2: f32, winf: f32) -> f32 {
    let s = w1 + winf;
    sqrt(s * s + 2.0 * w1 * w1) + w2
}

/// ulp distance between two positive finite `f32`
fn ulps(a: f32, b: f32) -> u32 {
    a.to_bits().abs_diff(b.to_bits())
}

const LO: f32 = f32::from_bits((127 - 60) << 23); // 2^-60
const HI: f32 = f32::from_bits((127 + 60) << 23); // 2^60

fn xorshift(s: &mut u32) -> u32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    *s
}

/// Where `s = w1 + winf` is in `[2^-60, 2^60]` nothing under/overflows, and
/// `lipschitz()` is the previous formula bit for bit: a dense grid, two million
/// random triples in the domain, and every `s` within 64 ulp of both ends.
#[test]
fn unchanged_bit_for_bit_where_nothing_under_or_overflows() {
    let mut n = 0u64;
    let mut check = |w1: f32, w2: f32, winf: f32| {
        let s = w1 + winf;
        if !(LO..=HI).contains(&s) || !w2.is_finite() {
            return;
        }
        let Ok(w) = MetricWeights::new(w1, w2, winf) else {
            panic!("({w1:e}, {w2:e}, {winf:e}) rejected inside the safe domain");
        };
        assert_eq!(
            w.lipschitz().to_bits(),
            previous(w1, w2, winf).to_bits(),
            "({w1:e}, {w2:e}, {winf:e})"
        );
        n += 1;
    };
    // dense: each weight over 0 and 64 values per binade across the domain
    let mut vals = vec![0.0f32];
    for e in -60..=59 {
        for i in 0..64u32 {
            vals.push(f32::from_bits((((127 + e) as u32) << 23) | (i << 17)));
        }
    }
    for &a in vals.iter().step_by(7) {
        for &b in vals.iter().step_by(97) {
            for &c in vals.iter().step_by(11) {
                check(a, b, c);
            }
        }
    }
    // random bit patterns restricted to the domain
    let mut st = 0x9e37_79b9u32;
    for _ in 0..2_000_000 {
        let pick = |st: &mut u32| {
            let e = 67 + xorshift(st) % 120; // biased exponents 2^-60 ..= 2^59
            f32::from_bits((e << 23) | (xorshift(st) & 0x007f_ffff))
        };
        let (a, b, c) = (pick(&mut st), pick(&mut st), pick(&mut st));
        let zero = xorshift(&mut st) % 4; // also each weight alone at 0
        match zero {
            0 => check(0.0, b, c),
            1 => check(a, b, 0.0),
            _ => check(a, b, c),
        }
    }
    // the ends of the range, ±64 ulp, through each of w1 / winf
    for edge in [LO, HI] {
        for d in -64i32..=64 {
            let v = f32::from_bits(edge.to_bits().wrapping_add_signed(d));
            check(v, 0.0, 0.0);
            check(0.0, 0.0, v);
            check(v, 1.0, 0.0);
        }
    }
    assert!(n > 2_000_000, "compared {n}");
}

/// Everywhere `new` accepts, `lipschitz()` is within 2 ulp of the closed form
/// (the formula rounds twice, `sqrt` then `+ w2`: it is 2 ulp off inside
/// `[2^-60, 2^60]` too, measured, so the scaled branch is as accurate as the
/// unscaled one)
/// and `normalised()` is finite and 1-Lipschitz; `new` rejects exactly where
/// the closed form is not a normal `f32` (up to the last ulp at the two
/// boundaries). Every 2^12-th bit pattern, each weight alone and all three.
#[test]
fn accepted_exactly_where_the_constant_is_a_normal_f32() {
    let (mut accepted, mut rejected, mut worst) = (0u64, 0u64, 0u32);
    for bits in (1..0x7f80_0000u32).step_by(0x1000).chain([0x7f7f_ffff]) {
        let v = f32::from_bits(bits);
        for (w1, w2, winf) in [(v, 0.0, 0.0), (0.0, v, 0.0), (0.0, 0.0, v), (v, v, v)] {
            let k = closed(w1, w2, winf) as f32;
            let at_edge = ulps(k, f32::MIN_POSITIVE) <= 1 || ulps(k, f32::MAX) <= 1;
            match MetricWeights::new(w1, w2, winf) {
                Ok(w) => {
                    accepted += 1;
                    assert!(k.is_normal() || at_edge, "{v:e}: accepted, constant {k:e}");
                    let u = ulps(w.lipschitz(), k);
                    worst = worst.max(u);
                    assert!(u <= 2, "{v:e}: lipschitz {:e} vs {k:e}", w.lipschitz());
                    let (a, b, c) = w.normalised().weights();
                    assert!(a.is_finite() && b.is_finite() && c.is_finite(), "{v:e}");
                    let one = closed(a, b, c);
                    assert!((one - 1.0).abs() < 1e-6, "{v:e}: normalised {one}");
                }
                Err(e) => {
                    rejected += 1;
                    assert_eq!(e, MetricError::Degenerate, "{v:e}");
                    assert!(!k.is_normal() || at_edge, "{v:e}: rejected, constant {k:e}");
                }
            }
        }
    }
    assert!(
        accepted > 100_000 && rejected > 1000,
        "{accepted} / {rejected}"
    );
    assert!(worst <= 2, "worst {worst} ulp");
}

/// The examples: tiny weights whose constant is subnormal, and huge ones
/// whose constant overflows, are degenerate; the smallest normal and the
/// largest finite weight in a single norm are not.
#[test]
fn the_edges_of_each_norm() {
    let t = f32::from_bits(1);
    let m = f32::MIN_POSITIVE;
    for w in [
        (t, 0.0, 0.0),
        (0.0, t, 0.0),
        (0.0, 0.0, t),
        (t, t, t),
        (f32::MAX, 0.0, 0.0),
        (f32::MAX, 0.0, f32::MAX),
    ] {
        assert_eq!(
            MetricWeights::new(w.0, w.1, w.2),
            Err(MetricError::Degenerate),
            "{w:?}"
        );
    }
    for w in [
        (m, 0.0, 0.0),
        (0.0, m, 0.0),
        (0.0, 0.0, m),
        (0.0, f32::MAX, 0.0),
        (0.0, 0.0, f32::MAX),
    ] {
        assert!(MetricWeights::new(w.0, w.1, w.2).is_ok(), "{w:?}");
    }
}

/// The smallest accepted weight of each single norm: the weight just below
/// it is rejected, and the constant it gives is the first normal `f32`
/// reached (`k(w) ≥ MIN_POSITIVE > k(w⁻)`, in the closed form).
#[test]
fn the_smallest_accepted_weight_per_norm() {
    for norm in 0..3 {
        let at = |v: f32| match norm {
            0 => (v, 0.0, 0.0),
            1 => (0.0, v, 0.0),
            _ => (0.0, 0.0, v),
        };
        let ok = |v: f32| {
            let w = at(v);
            MetricWeights::new(w.0, w.1, w.2).is_ok()
        };
        // binary search over the bit patterns of positive finite f32
        let (mut lo, mut hi) = (1u32, f32::MIN_POSITIVE.to_bits() * 2);
        assert!(!ok(f32::from_bits(lo)) && ok(f32::from_bits(hi)));
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if ok(f32::from_bits(mid)) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let (first, below) = (f32::from_bits(hi), f32::from_bits(lo));
        let (wf, wb) = (at(first), at(below));
        let kf = closed(wf.0, wf.1, wf.2) as f32;
        let kb = closed(wb.0, wb.1, wb.2) as f32;
        assert!(
            kf.is_normal() && !kb.is_normal(),
            "norm {norm}: {first:e} → {kf:e}, {below:e} → {kb:e}"
        );
        // every weight above it (sampled) is accepted up to where k overflows
        for b in (hi..0x7f00_0000).step_by(0x10_0000) {
            assert!(
                ok(f32::from_bits(b)),
                "norm {norm}: {:e} rejected",
                f32::from_bits(b)
            );
        }
    }
}

/// The lower end of the unscaled range, `2^-60`, decides bits: for
/// `s = w1 + winf` just above it and `w1 < 2^-63.5`, the unscaled `2·w1²`
/// rounds to a subnormal and the result is 1 ulp from the scaled one (both
/// within 1 ulp of the closed form). The values here are today's; each
/// would move if the end were `2^-50` (first four, `s` above `2^-60`:
/// unscaled today) or `2^-63` (last three, `s` below `2^-60`: scaled today). Found by a random search over that band.
///
/// The upper end, `2^60`, does not decide bits: above `2^50` scaling by
/// `2^-100` is exact and `w1²`, where it would go subnormal, is below the
/// last bit of `s²`; 2·10⁷ random triples between `2^49` and `2^64` gave the
/// same bits with the end at `2^50`, `2^60` or `2^63`.
#[test]
fn the_lower_end_of_the_unscaled_range_is_pinned() {
    // (w1, winf, lipschitz with the end at 2^-60, with the end moved)
    let cases: [(u32, u32, u32, u32); 7] = [
        (0x1e78_f321, 0x217c_1c3a, 0x2180_0794, 0x2180_0793), // 2^-50
        (0x1c85_e18d, 0x21a6_6479, 0x21a6_85f7, 0x21a6_85f8), // 2^-50
        (0x1e04_8b60, 0x221c_aca4, 0x221d_319e, 0x221d_319f), // 2^-50
        (0x1d67_95bb, 0x2187_f94c, 0x2188_6d7a, 0x2188_6d79), // 2^-50
        (0x1c14_fc17, 0x205e_06f7, 0x205e_9c56, 0x205e_9c57), // 2^-63
        (0x1f43_c2b2, 0x2002_ce56, 0x2040_9c61, 0x2040_9c62), // 2^-63
        (0x1abc_1788, 0x2057_bf9a, 0x2057_d71f, 0x2057_d720), // 2^-63
    ];
    for (w1, winf, now, moved) in cases {
        let (a, c) = (f32::from_bits(w1), f32::from_bits(winf));
        let s = a + c;
        assert!(s > f32::from_bits((127 - 63) << 23) && s < f32::from_bits((127 - 50) << 23));
        let w = MetricWeights::new(a, 0.0, c).unwrap();
        assert_eq!(w.lipschitz().to_bits(), now, "({a:e}, 0, {c:e})");
        // the unscaled formula is the one in force at and above 2^-60
        let unscaled = if s >= LO { now } else { moved };
        assert_eq!(previous(a, 0.0, c).to_bits(), unscaled, "({a:e}, 0, {c:e})");
        assert_ne!(now, moved);
        let k = closed(a, 0.0, c) as f32;
        assert!(
            ulps(f32::from_bits(now), k) <= 1 && ulps(f32::from_bits(moved), k) <= 1,
            "({a:e}, 0, {c:e})"
        );
    }
}
