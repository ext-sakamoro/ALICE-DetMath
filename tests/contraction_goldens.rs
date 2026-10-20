//! ALICE-Law G4: argument-targeted pins for contractions the dense sample does not see.
//!
//! A multiply-add fused into one rounding at a step of a double-precision polynomial moves
//! only a few outputs in a million (the leading terms) down to about one in 2^37 (the inner
//! steps), which the ~2^21-point sweep of `tests/dense_golden.rs` can miss. Each input here
//! separates the step it names: the expected bits are the unmodified build's, and the build
//! with that one step written as `a.mul_add(b, c)` gives other bits.
//!
//! The leading-term pins (`P1`, `AT0` … `AT3`, `PS0`, `QS1`, `LG1`, `LG2`) were found by
//! comparing about 2^20 (2^24 for `AT3` and the `LG` pair) outputs per function of the two
//! builds (measured 2026-10-18, each mutant reverted). The inner-step pins (`P2`, `P3`,
//! `AT4` … `AT8`, `PS4`, `LG3`, `LG4`) come from `tools/contraction-search`, which draws inputs
//! where the step's variable is largest, keeps those whose step rounds differently in one
//! rounding than in two (an error-free test at the step), then whose polynomial value changes,
//! and evaluates only those in full; its copies of the kernels are compared bit for bit with
//! the library on every evaluated input. The pins are the first ones (by draw index) of:
//!
//! ```text
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --log2-n 30 P2 AT4 AT5 PS4
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --log2-n 34 AT6 AT7 LG3
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --log2-n 37 P3 AT8 LG4
//! ```
//!
//! (the last takes about 20 minutes on 10 cores; it finds 1 / 2 / 1 inputs). CI runs
//! `--check-pins tests/contraction_goldens.rs`, which re-checks that every pin of a step the
//! tool models still separates it. Not reached: `exp64` `P4` and `ln64` `LG5` (no input whose
//! polynomial value changes among 2^34 draws each). Debug and release give the same bits (no
//! contraction is enabled in either profile), and both run in CI on every runner.
use alice_det_math as dm;

/// `(function, step the input separates, input bits, expected output bits)`
type Case = (&'static str, &'static str, u64, u64);

const CASES: &[Case] = &[
    // exp64: `P1 + t * (…)` in `c = r - t * (…)`
    ("exp64", "P1", 0x403e_1e55_1477_9197, 0x42a5_e23f_5657_3e26),
    ("exp64", "P1", 0xc082_1341_d849_6019, 0x0bc7_2e57_1324_28c0),
    ("exp64", "P1", 0xc059_90bb_3690_7738, 0x36b6_2157_35a5_bcb5),
    // exp64 inner steps `P2 + t * (…)` and `P3 + t * (…)` (tools/contraction-search)
    ("exp64", "P2", 0x407a517bb9f958fa, 0x65e6c248043529ac),
    ("exp64", "P2", 0xc0699d09b3ec876b, 0x2d74d6374702d36d),
    ("exp64", "P2", 0x407aed763d649376, 0x66c7cc03d9275006),
    ("exp64", "P3", 0xc043f4ee4ace3a88, 0x3c555c9d437b2669),
    // atan64: `AT[0] + w * (…)` (s1) and `AT[1] + w * (…)` (s2)
    (
        "atan64",
        "AT0",
        0xc004_6ab2_f098_b14c,
        0xbff3_2857_8688_22b0,
    ),
    (
        "atan64",
        "AT0",
        0xbfda_8676_1fb1_ace0,
        0xbfd9_2562_5311_8f2e,
    ),
    (
        "atan64",
        "AT0",
        0xc004_a168_d4b0_07fe,
        0xbff3_36c6_51e5_8d06,
    ),
    (
        "atan64",
        "AT1",
        0xbfda_6432_fd6b_0150,
        0xbfd9_081f_72ea_d0b1,
    ),
    // atan64 `AT[3] + w * (…)` (inner step of s2): a 2^24-input search found this one
    (
        "atan64",
        "AT3",
        0x3fd9_93c1_727c_82e6,
        0x3fd8_552d_795a_561c,
    ),
    // atan64 inner steps `AT[4]` … `AT[8]` (tools/contraction-search)
    ("atan64", "AT4", 0x3fda92d772f0e9e3, 0x3fd92ff252ff3c88),
    ("atan64", "AT4", 0x3fdaa83e900b4a68, 0x3fd942319278b6d6),
    ("atan64", "AT4", 0x3fda3d53ac4ef75b, 0x3fd8e6df46336719),
    ("atan64", "AT5", 0x3fd961a89dad2115, 0x3fd829ef3a0d9939),
    ("atan64", "AT5", 0xbfdb8322b6e0f78e, 0xbfd9fbd5f5692ce1),
    ("atan64", "AT5", 0x3fd99df6c58d8732, 0x3fd85dfa68fe1eeb),
    ("atan64", "AT6", 0x3fdb62b0ae3cc061, 0x3fd9e06e67cc23ec),
    ("atan64", "AT6", 0xbfdbcb0b7cda3dd7, 0xbfda386eb0e5136c),
    ("atan64", "AT6", 0x3fdbc9972a1c8ad3, 0x3fda37356ec2d0f2),
    ("atan64", "AT7", 0xbfdbaa83b15a47a2, 0xbfda1d0b277056e6),
    ("atan64", "AT7", 0x3fdad8a6600e5b1b, 0x3fd96b664ced06a4),
    ("atan64", "AT7", 0x3fda91c45b8788d5, 0x3fd92f07adf0b78d),
    ("atan64", "AT8", 0xbfdb95f680449f3e, 0xbfda0bb84b163f30),
    ("atan64", "AT8", 0xbfdad1eaa2adad68, 0xbfd965ac4498daac),
    // asin_pq (asin64 / acos64): `PS0 + t * (…)` (p) and `QS1 + t * (…)` (q)
    (
        "asin64",
        "PS0",
        0xbfd9_992a_3213_6cec,
        0xbfda_55d1_33bf_82fd,
    ),
    (
        "asin64",
        "PS0",
        0x3fdd_d3d6_0edb_232c,
        0x3fdf_0763_cb58_af17,
    ),
    (
        "acos64",
        "PS0",
        0x3fe6_e217_15d2_b992,
        0x3fe8_c4dc_2265_d9b0,
    ),
    (
        "acos64",
        "PS0",
        0xbfe7_05a2_3033_e600,
        0x4002_fd84_ef4d_61a0,
    ),
    (
        "asin64",
        "QS1",
        0x3fdb_7130_19ad_f650,
        0x3fdc_5c7f_d496_9f9a,
    ),
    (
        "asin64",
        "QS1",
        0xbfdb_2411_e53d_ba14,
        0xbfdc_073d_a2ca_e021,
    ),
    (
        "acos64",
        "QS1",
        0x3fdf_03f5_7e5f_a2cc,
        0x3ff1_09b6_ad48_1af9,
    ),
    (
        "acos64",
        "QS1",
        0x3fe3_1ec2_fdbf_b829,
        0x3fed_c5eb_a960_9616,
    ),
    // asin_pq inner step `PS4 + t * PS5` (tools/contraction-search)
    ("asin64", "PS4", 0xbfe1b91c40e5fc4d, 0xbfe2c883cfae03a0),
    ("asin64", "PS4", 0x3fe3be19ff2f90c8, 0x3fe5469a977c5fb5),
    ("asin64", "PS4", 0xbfde60f137504379, 0xbfdfa74a0bab8354),
    // ln64 kernel: `LG1 + w * (…)` (t2, seen through log10_64) and `LG2 + w * (…)` (t1); a
    // 2^24-input search near the kernel's reduced range found these, the 2^20 one none
    (
        "log10_64",
        "LG1",
        0x3fd6_fcbc_663e_0c94,
        0xbfdc_75d9_75ae_8644,
    ),
    (
        "log10_64",
        "LG1",
        0x3ff6_d119_4df2_23cb,
        0x3fc3_baae_cd87_044e,
    ),
    (
        "log10_64",
        "LG1",
        0x3fe7_2f6e_6daf_7cb8,
        0xbfc1_e97a_253a_8eea,
    ),
    ("ln64", "LG2", 0x3fe8_565e_18e8_b771, 0xbfd1_84ac_5f91_b541),
    // ln64 inner steps `LG3 + w * (…)` (t2) and `LG4 + w * LG6` (t1) (tools/contraction-search)
    ("ln64", "LG3", 0x3fe7734b08adf773, 0xbfd3e4f6fa022d62),
    ("ln64", "LG3", 0x3fe6f5e3a58b62be, 0xbfd53ed697fcad8f),
    ("ln64", "LG3", 0x3fe72308879bab79, 0xbfd4c17c11c8ccc3),
    ("ln64", "LG4", 0x3ff61060812c3e62, 0x3fd4911e6fdb0fb3),
];

fn eval(name: &str, x: f64) -> f64 {
    match name {
        "exp64" => dm::exp64(x),
        "atan64" => dm::atan64(x),
        "asin64" => dm::asin64(x),
        "acos64" => dm::acos64(x),
        "ln64" => dm::ln64(x),
        "log10_64" => dm::log10_64(x),
        other => panic!("no function {other}"),
    }
}

#[test]
fn leading_term_contraction_pins() {
    let mut compared = 0;
    let mut wrong = Vec::new();
    for &(name, step, x, want) in CASES {
        let got = eval(name, f64::from_bits(x)).to_bits();
        if got != want {
            wrong.push(format!(
                "{name}({x:#018x}) [{step}] = {got:#018x}, pinned {want:#018x}"
            ));
        }
        compared += 1;
    }
    assert!(
        wrong.is_empty(),
        "{} of {compared} pins moved:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert_eq!(compared, CASES.len(), "compared nothing");
    assert!(compared > 0, "compared nothing");
}
