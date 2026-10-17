//! The exact results the kernels reach through cancellation, pinned bit for bit
//! (ALICE-Law G4).
//!
//! At these inputs the mathematical result is exact (a zero or one), and the kernel gets
//! there by subtracting quantities that cancel exactly: the reduction `x - k·ln2` of
//! `exp` at 0, `ln` of a mantissa of exactly 1, `acos(1)` as `π/2 − asin(1)`'s complement,
//! `atan2` of a zero numerator, `hypot` of two zeros. A contracted multiply-add, a
//! reassociated sum or a lost sign turns the exact result into a rounding residue or the
//! wrong zero, which the comparison of bits sees on every profile and architecture.
//!
//! oracle: IEEE 754-2019 and C99 Annex F (F.10): `log(1) = +0` in every base,
//! `acos(1) = +0`, `atan2(±0, x > 0) = ±0`, `exp(±0) = 1`, `hypot(±0, ±0) = +0`; `pow`'s
//! exact cases are in `tests/pow_ieee_one.rs`, and the odd functions' signed zeros in
//! `tests/signed_zero.rs`.

use alice_det_math as dm;

fn bits32(x: f32) -> u32 {
    x.to_bits()
}

fn bits64(x: f64) -> u64 {
    x.to_bits()
}

#[test]
fn f32_kernels_reach_the_exact_result() {
    let cases: [(&str, f32, u32); 12] = [
        ("ln(1)", dm::ln(1.0), 0),
        ("log2(1)", dm::log2(1.0), 0),
        ("log10(1)", dm::log10(1.0), 0),
        ("acos(1)", dm::acos(1.0), 0),
        ("atan2(+0, 1)", dm::atan2(0.0, 1.0), 0),
        ("atan2(-0, 1)", dm::atan2(-0.0, 1.0), 0x8000_0000),
        ("atan2(+0, 2.5)", dm::atan2(0.0, 2.5), 0),
        ("exp(+0)", dm::exp(0.0), 1.0f32.to_bits()),
        ("exp(-0)", dm::exp(-0.0), 1.0f32.to_bits()),
        ("hypot(+0, -0)", dm::hypot(0.0, -0.0), 0),
        ("hypot(-0, -0)", dm::hypot(-0.0, -0.0), 0),
        ("powi(-3.5, 0)", dm::powi(-3.5, 0), 1.0f32.to_bits()),
    ];
    for (name, got, want) in cases {
        assert_eq!(
            bits32(got),
            want,
            "{name} = {got:?} ({:#010x})",
            bits32(got)
        );
    }
}

#[test]
fn f64_kernels_reach_the_exact_result() {
    let cases: [(&str, f64, u64); 9] = [
        ("ln64(1)", dm::ln64(1.0), 0),
        ("log2_64(1)", dm::log2_64(1.0), 0),
        ("log10_64(1)", dm::log10_64(1.0), 0),
        ("acos64(1)", dm::acos64(1.0), 0),
        ("atan2_64(+0, 1)", dm::atan2_64(0.0, 1.0), 0),
        (
            "atan2_64(-0, 1)",
            dm::atan2_64(-0.0, 1.0),
            0x8000_0000_0000_0000,
        ),
        (
            "atan2_64(-0, 7)",
            dm::atan2_64(-0.0, 7.0),
            0x8000_0000_0000_0000,
        ),
        ("exp64(+0)", dm::exp64(0.0), 1.0f64.to_bits()),
        ("exp64(-0)", dm::exp64(-0.0), 1.0f64.to_bits()),
    ];
    for (name, got, want) in cases {
        assert_eq!(
            bits64(got),
            want,
            "{name} = {got:?} ({:#018x})",
            bits64(got)
        );
    }
}
