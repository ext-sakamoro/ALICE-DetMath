//! The accuracy of the `f32` `asin` / `acos` on every input of `[-1, 1]` (2 130 706 434 inputs,
//! both zeros), against the correctly rounded `f32` of the exact value, in steps of the `f32`
//! grid. This is the measurement behind the accuracy table's "≤ 1 ulp" line for them in
//! `src/lib.rs`; `tests/accuracy.rs` samples 400 000 points, this test reads all of them.
//!
//! Reference: the platform `f64` `asin` / `acos` (within 1 ulp of `f64` on the targets the
//! workflow runs), rounded once to `f32`. That is the correctly rounded `f32` unless the exact
//! value lies near the midpoint between two `f32`: an input whose `f64` reference is within 4 ulp
//! of `f64` of such a midpoint is flagged and scored against both neighbours, keeping the larger
//! error, so a flagged input can only overstate the error.
//!
//! The sweeps are `#[ignore]`d (about 8 s each in release, one thread) and run by
//! `.github/workflows/exhaustive-f32.yml` with `--ignored`. Each prints
//! `EXHAUSTIVE_ULP <name> inputs <n> max <k> flagged <f>` and fails unless `n` is the count above
//! and `k` is at most [`STATED_STEPS`]; the workflow checks the printed lines independently
//! (`scripts/exhaustive_ulp_count.py`: a missing or repeated function, a wrong count, a maximum
//! over the bound, or no line at all is red). Measured on `aarch64` macOS, 2026-11-18: both
//! functions 0 steps on all but 6 inputs, each of those 6 flagged, maximum 1.
//!
//! The scorer's own tests (not ignored) check that it sees an error: outputs moved two steps up
//! score 2, and a reference next to a midpoint is scored against both neighbours.

use alice_det_math as dm;

/// The bound the accuracy table states for both functions.
const STATED_STEPS: u64 = 1;

/// The magnitudes `0 ..= 1.0`, each with both signs.
const TOP: u32 = 0x3f80_0000;
const ALL_INPUTS: u64 = 2 * (TOP as u64 + 1);

/// The position of `x` on the `f32` grid, with both zeros at 0.
fn key(x: f32) -> i64 {
    let b = x.to_bits() as i32;
    i64::from(if b < 0 { i32::MIN.wrapping_sub(b) } else { b })
}

fn steps(a: f32, b: f32) -> u64 {
    (key(a) - key(b)).unsigned_abs()
}

/// The next `f32` up (the crate's minimum Rust predates `f32::next_up`).
fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        x
    } else if x == 0.0 {
        f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn next_down(x: f32) -> f32 {
    -next_up(-x)
}

fn ulp64(x: f64) -> f64 {
    let a = x.abs();
    f64::from_bits(a.to_bits() + 1) - a
}

/// The correctly rounded candidates for the exact value near `r`: one when `r` is clear of a
/// midpoint between two `f32`, both neighbours when it is not.
fn candidates(r: f64) -> (f32, Option<f32>) {
    let c = r as f32;
    let d = r - f64::from(c);
    if d == 0.0 {
        return (c, None);
    }
    let n = if d > 0.0 { next_up(c) } else { next_down(c) };
    let mid = (f64::from(c) + f64::from(n)) / 2.0;
    if (r - mid).abs() <= 4.0 * ulp64(r) {
        (c, Some(n))
    } else {
        (c, None)
    }
}

/// The error of `got` against the reference `r`, in steps, and whether `r` was flagged.
fn score(got: f32, r: f64) -> (u64, bool) {
    let (c, other) = candidates(r);
    other.map_or_else(
        || (steps(got, c), false),
        |n| (steps(got, c).max(steps(got, n)), true),
    )
}

/// `(inputs, max steps, flagged)` over the magnitudes with bits `lo ..= top`, each with both signs.
/// One thread: the crate takes no result across threads (ALICE-Law G13).
fn sweep(
    f: fn(f32) -> f32,
    reference: fn(f64) -> f64,
    lo: u32,
    top: u32,
    bend: fn(f32) -> f32,
) -> (u64, u64, u64) {
    let (mut n, mut max, mut flagged) = (0u64, 0u64, 0u64);
    for m in lo..=top {
        for sign in [0u32, 0x8000_0000] {
            let x = f32::from_bits(m | sign);
            let (e, flag) = score(bend(f(x)), reference(f64::from(x)));
            n += 1;
            max = max.max(e);
            flagged += u64::from(flag);
        }
    }
    (n, max, flagged)
}

fn exhaustive(name: &str, f: fn(f32) -> f32, reference: fn(f64) -> f64) {
    let (n, max, flagged) = sweep(f, reference, 0, TOP, |y| y);
    // printed before the asserts, so the workflow's own check reads it whatever the asserts do
    eprintln!("EXHAUSTIVE_ULP {name} inputs {n} max {max} flagged {flagged}");
    assert_eq!(
        n, ALL_INPUTS,
        "{name}: {n} inputs, not every f32 of [-1, 1]"
    );
    assert!(
        max <= STATED_STEPS,
        "{name}: {max} steps from the correctly rounded value somewhere in [-1, 1]"
    );
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn asin() {
    exhaustive("asin", dm::asin, f64::asin);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn acos() {
    exhaustive("acos", dm::acos, f64::acos);
}

#[test]
fn the_scorer_sees_two_steps() {
    // every output moved two steps up, over 65 537 magnitudes from 0.5 (both signs): the sweep
    // scores 2, so one that compared nothing or misread the grid cannot pass; unmoved, the same
    // inputs stay within the bound
    let (lo, top) = (0x3f00_0000, 0x3f01_0000);
    let (n, max, _) = sweep(dm::asin, f64::asin, lo, top, |y| next_up(next_up(y)));
    assert_eq!(n, 2 * u64::from(top - lo + 1));
    assert!(max >= 2, "two steps up scored {max}");
    let (_, max, _) = sweep(dm::asin, f64::asin, lo, top, |y| y);
    assert!(max <= STATED_STEPS, "unmoved scored {max}");
}

#[test]
fn the_scorer_reads_both_zeros_as_one_point() {
    assert_eq!(steps(0.0, -0.0), 0);
    assert_eq!(steps(f32::from_bits(1), -f32::from_bits(1)), 2);
}

#[test]
fn a_reference_at_a_midpoint_is_scored_against_both_neighbours() {
    let c = 1.5_f32;
    let n = next_up(c);
    let mid = (f64::from(c) + f64::from(n)) / 2.0;
    // at the midpoint either neighbour may be the correct rounding: an output on the far side of
    // the other neighbour is two steps from one of them
    let (e, flagged) = score(next_up(n), mid);
    assert!(flagged);
    assert_eq!(e, 2);
    // clear of the midpoint only the nearest counts
    let (e, flagged) = score(c, f64::from(c) + ulp64(1.5) * 3.0);
    assert!(!flagged);
    assert_eq!(e, 0);
}
