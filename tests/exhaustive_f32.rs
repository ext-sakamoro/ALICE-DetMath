//! Every f32 input of the 12 unary f32 functions, pinned bit for bit (ALICE-Law G4).
//!
//! `tests/dense_golden.rs` samples about 2^21 points per function, which misses a
//! change of one rounding that moves only a few outputs (a contracted leading
//! multiply-add in a polynomial moves an output only where the two roundings
//! differ). This test evaluates each function at all 2^32 bit patterns and folds
//! the output bits into a 64-bit digest.
//!
//! The fold is `h = (h ^ bits) * P` with `P` odd: every step is a bijection of the
//! state for a fixed input word, so a change of any single output always changes
//! the digest. NaN outputs are folded as the canonical quiet NaN, so the pin is on
//! values, not payloads (payloads are checked by `tests/signed_zero.rs` and the
//! G2 tests).
//!
//! The pins are in `tests/exhaustive_f32.digests`. The tests are `#[ignore]`d (about
//! 10 s each in release) and run by `.github/workflows/exhaustive-f32.yml` with
//! `--ignored`. Each prints `EXHAUSTIVE <name> inputs swept <n> digest <hex>` and
//! then fails unless `n` is 2^32 and the digest is its pin; independently of the
//! test's own result, the workflow compares the printed lines with the digests file
//! (`scripts/exhaustive_count.py`: a wrong count or digest, a missing, extra or
//! repeated function, or an empty file is red). `DET_MATH_PRINT_EXHAUSTIVE=1` skips
//! the comparison, for re-pinning after an intended change.
//!
//! oracle: the digests were produced on `aarch64` (macOS) and must be reproduced on
//! `x86_64` and `aarch64` Linux; they pin the current kernels, so an intended change
//! of an output re-pins them in the same commit.

use alice_det_math as dm;

const ALL_INPUTS: u64 = 1 << 32;

const DIGESTS: &str = include_str!("exhaustive_f32.digests");

fn pinned(name: &str) -> u64 {
    let hex = DIGESTS
        .lines()
        .filter(|l| !l.starts_with('#'))
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(' '))
        .unwrap_or_else(|| panic!("{name}: no pin in tests/exhaustive_f32.digests"));
    u64::from_str_radix(hex.trim(), 16).expect("a 16-hex-digit digest")
}

fn sweep(name: &str, f: fn(f32) -> f32) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut n: u64 = 0;
    for b in 0..=u32::MAX {
        let y = f(f32::from_bits(b));
        let bits = if y.is_nan() { 0x7fc0_0000 } else { y.to_bits() };
        h = (h ^ u64::from(bits)).wrapping_mul(0x0000_0100_0000_01b3);
        n += 1;
    }
    // printed before the asserts, so a moved digest is in the log for the workflow's
    // comparison with the digests file as well as in the panic
    eprintln!("EXHAUSTIVE {name} inputs swept {n} digest {h:016x}");
    assert_eq!(n, ALL_INPUTS, "{name}: swept {n} inputs, not 2^32");
    if std::env::var_os("DET_MATH_PRINT_EXHAUSTIVE").is_some() {
        return;
    }
    let want = pinned(name);
    assert_eq!(
        h, want,
        "{name}: output bits moved somewhere in the 2^32 inputs ({h:016x}, pin {want:016x})"
    );
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn sin() {
    sweep("sin", dm::sin);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn cos() {
    sweep("cos", dm::cos);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn tan() {
    sweep("tan", dm::tan);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn exp() {
    sweep("exp", dm::exp);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn ln() {
    sweep("ln", dm::ln);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn log2() {
    sweep("log2", dm::log2);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn log10() {
    sweep("log10", dm::log10);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn atan() {
    sweep("atan", dm::atan);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn asin() {
    sweep("asin", dm::asin);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn acos() {
    sweep("acos", dm::acos);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn tanh() {
    sweep("tanh", dm::tanh);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn cbrt() {
    sweep("cbrt", dm::cbrt);
}
