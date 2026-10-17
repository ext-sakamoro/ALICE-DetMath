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
//! The tests are `#[ignore]`d (about 10 s each in release) and run by
//! `.github/workflows/exhaustive-f32.yml` with `--ignored`. Each prints
//! `EXHAUSTIVE <name> inputs swept <n> digest <hex>` and fails unless `n` is
//! 2^32; the workflow also fails unless 12 such lines with that count were printed.
//! `DET_MATH_PRINT_EXHAUSTIVE=1` prints the digests without comparing them, for
//! re-pinning after an intended change.
//!
//! oracle: the digests were produced on `aarch64` (macOS) and must be reproduced on
//! `x86_64` and `aarch64` Linux; they pin the current kernels, so an intended change
//! of an output re-pins them in the same commit.

use alice_det_math as dm;

const ALL_INPUTS: u64 = 1 << 32;

fn sweep(name: &str, f: fn(f32) -> f32, want: u64) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut n: u64 = 0;
    for b in 0..=u32::MAX {
        let y = f(f32::from_bits(b));
        let bits = if y.is_nan() { 0x7fc0_0000 } else { y.to_bits() };
        h = (h ^ u64::from(bits)).wrapping_mul(0x0000_0100_0000_01b3);
        n += 1;
    }
    eprintln!("EXHAUSTIVE {name} inputs swept {n} digest {h:016x}");
    assert_eq!(n, ALL_INPUTS, "{name}: swept {n} inputs, not 2^32");
    if std::env::var_os("DET_MATH_PRINT_EXHAUSTIVE").is_some() {
        return;
    }
    assert_eq!(
        h, want,
        "{name}: output bits moved somewhere in the 2^32 inputs ({h:016x})"
    );
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn sin() {
    sweep("sin", dm::sin, 0x364d_2ef8_6918_967a);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn cos() {
    sweep("cos", dm::cos, 0x519a_427f_4aeb_58a2);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn tan() {
    sweep("tan", dm::tan, 0x70e6_e772_2a47_5cf1);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn exp() {
    sweep("exp", dm::exp, 0xbb5e_ee00_f91f_8c16);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn ln() {
    sweep("ln", dm::ln, 0x0997_dfee_86d3_b261);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn log2() {
    sweep("log2", dm::log2, 0xa8e2_23da_7429_5e9d);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn log10() {
    sweep("log10", dm::log10, 0x3887_905d_0c09_a371);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn atan() {
    sweep("atan", dm::atan, 0x889a_1526_5f26_a8c5);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn asin() {
    sweep("asin", dm::asin, 0x0fed_4ead_768a_439d);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn acos() {
    sweep("acos", dm::acos, 0x8df4_8a8f_43f1_9857);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn tanh() {
    sweep("tanh", dm::tanh, 0x5a6f_06ee_44fb_4aed);
}

#[test]
#[ignore = "exhaustive: run by exhaustive-f32.yml with --ignored"]
fn cbrt() {
    sweep("cbrt", dm::cbrt, 0xd0c7_12cc_4e82_46a5);
}
