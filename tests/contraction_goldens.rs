//! ALICE-Law G4: argument-targeted pins for contractions the dense sample does not see.
//!
//! A multiply-add fused into one rounding at the leading term of a double-precision
//! polynomial moves only a few outputs in a million, which the ~2^21-point sweep of
//! `tests/dense_golden.rs` can miss. Each input here was found by building the crate with
//! that one contraction (`a.mul_add(b, c)` in place of `c + a * b` at the named step) and
//! comparing about 2^20 outputs per function with the unmodified build; the expected bits
//! are the unmodified ones, so the contraction at the named step changes at least the pinned
//! output (measured 2026-10-18, each mutant reverted). Debug and release give the same bits
//! (no contraction is enabled in either profile), and both run in CI on every runner.
use alice_det_math as dm;

/// `(function, step the input separates, input bits, expected output bits)`
type Case = (&'static str, &'static str, u64, u64);

const CASES: &[Case] = &[
    // exp64: `P1 + t * (…)` in `c = r - t * (…)`
    ("exp64", "P1", 0x403e_1e55_1477_9197, 0x42a5_e23f_5657_3e26),
    ("exp64", "P1", 0xc082_1341_d849_6019, 0x0bc7_2e57_1324_28c0),
    ("exp64", "P1", 0xc059_90bb_3690_7738, 0x36b6_2157_35a5_bcb5),
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
