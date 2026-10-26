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
//! (the last takes about 20 minutes on 10 cores; it finds 1 / 2 / 1 inputs). The `P4` pin
//! and the `LG5` pin are out of reach of drawing (about one input in 2^42 near the top of the
//! step's variable): `--enumerate STEP LO HI` walks every value of the variable in a range and
//! visits only those whose step can round differently (the step's condition solved, not
//! tested). The `P4` pin is the only output change of the 54 batches of 2^36 values of `t` below
//! `0x3fbebedfa43fe5c9` (2^41.75 values), the `LG5` pin the only one of the 108 batches of 2^36
//! values of `w` below `0x3f4c6529c58fcd1d` (2^42.75 values), each batch a few seconds on 10
//! cores; the batch that found each pin, alone:
//!
//! ```text
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --enumerate P4 3fbebb7fa43fe5c9 3fbebb8fa43fe5c9
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --enumerate LG5 3f4c5e69c58fcd1d 3f4c5e79c58fcd1d
//! ```
//!
//! The `erf64` pins (every step of its `PP` / `QQ`, `PA` / `QA`, `RA` / `SA` and `RB` / `SB`
//! polynomials, `QQ0` / `QA0` / `SA0` / `SB0` naming the outer `1 + s * (…)`) and the `powf64`
//! pins of `CASES_POW` (the `e_pow` log polynomial `L1` … `L5` and exponential polynomial `P1` …
//! `P4`, with both arguments) come from `tools/contraction-search --pow-erf`, which models every
//! one of these 58 steps: 2^20 draws per step find 35, 2^28 find 13 more and 2^34 find 6 more.
//! The other four are walked over the step's variable with
//! `--pow-erf-enumerate` (inputs built backward from the variable and kept only when the copy's
//! step reads that value): `L4` in one 2^32 slice, `L5` in the top 2^36 values of `s2`, `P4` at
//! the `t` of the exp64 `P4` pin with 64 bases `x` per `t`, and `SB6` in the 133rd batch of 2^36
//! values of `s` below `1/x²` at the branch switch:
//!
//! ```text
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --pow-erf --log2-n 34
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --pow-erf-enumerate powf64:L4 3f84000000000000 3f84000100000000
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --pow-erf-enumerate powf64:L5 3f84e65f49685e38 3f84e66f49685e38
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --pow-erf-enumerate powf64:P4 3fbebb87a4d247b0 3fbebb87a4d247b1
//! cargo run --release --manifest-path tools/contraction-search/Cargo.toml -- --pow-erf-enumerate erf64:SB6 3fbf53daeb85363d 3fbf53eaeb85363d
//! ```
//!
//! CI runs `--check-pins tests/contraction_goldens.rs`, which re-checks that every pin of a step
//! the tool models still separates it. Debug and release give the same bits (no
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
    // exp64 `P4 + t * P5` (tools/contraction-search --enumerate)
    ("exp64", "P4", 0x3fd62cbe02873b96, 0x3ff6a014e11fc3d9),
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
    // ln64 `LG5 + w * LG7` (tools/contraction-search --enumerate)
    ("ln64", "LG5", 0x3ff6a024e491a349, 0x3fd62ceb4e859d92),
    // erf64 (tools/contraction-search --pow-erf)
    ("erf64", "PA4", 0xbff3fbd9712e7643, 0xbfed866e93f5cbcf),
    ("erf64", "PA3", 0xbff3bb1cf8062f47, 0xbfed671eb09d3306),
    ("erf64", "PA2", 0x3ff3d0d0891d29c8, 0x3fed71c0eef1a8a8),
    ("erf64", "PA1", 0xbfeb11b727cb2b52, 0xbfe896e75dc8ef96),
    ("erf64", "PA0", 0x3ff2a33a34f1d0b1, 0x3fecd1014ca32efd),
    ("erf64", "QA3", 0xbfebfd2274583da6, 0xbfe915a38a4df932),
    ("erf64", "QA2", 0xbfeb1bb968d16cd9, 0xbfe89c6b73b03202),
    ("erf64", "QA1", 0x3ff3a810e288a55d, 0x3fed5dabe6a4ef22),
    ("erf64", "QA0", 0x3ff38ae79d5bfed7, 0x3fed4efed130528f),
    ("erf64", "RA6", 0x3ff4c6392dc63b1b, 0x3fede0a61402c1b3),
    ("erf64", "RA5", 0xbff72fcd7426fa75, 0xbfeeb4e24bb0661f),
    ("erf64", "RA4", 0x3ff69439df9c859a, 0x3fee8775ecf98661),
    ("erf64", "RA3", 0xbff513b2ded18cd1, 0xbfee0042550a2bf3),
    ("erf64", "RA2", 0xbff52875c0e3d0ef, 0xbfee08776d5de3d6),
    ("erf64", "RA1", 0x3ff6b96f05a1c4fc, 0x3fee92c6b09daa8c),
    ("erf64", "RA0", 0x3ff49d596ddb584e, 0x3fedcf558d61ae3a),
    ("erf64", "SA6", 0x3ff5af7322e34051, 0x3fee3b36527d43de),
    ("erf64", "SA5", 0xbff417837bcf0f6c, 0xbfed9370f41513f5),
    ("erf64", "SA4", 0x3ff48b11d91df7b6, 0x3fedc77201a840b0),
    ("erf64", "SA3", 0x3ff504a738d913cc, 0x3fedfa3e0d03ca9e),
    ("erf64", "SA2", 0xbff46c1223be7d6d, 0xbfedb9dbec3e1c42),
    ("erf64", "SA1", 0xbff5bab7b80cd134, 0xbfee3f3f82bcc222),
    ("erf64", "SA0", 0xbff751fe3dceefbc, 0xbfeebe37dc2289d7),
    ("erf64", "RB0", 0x400accf5306d11d1, 0x3feffffb77a72397),
    ("erf64", "SB1", 0xc007e46f8d6ac728, 0xbfefffcd945c2357),
    ("erf64", "PP3", 0x3fead205af57af16, 0x3fe87388b64ecb54),
    ("erf64", "PP2", 0xbfea5a977d823bf0, 0xbfe82ff5ea9aa7a4),
    ("erf64", "PP1", 0xbfe868b70b6a24ab, 0xbfe704776f695736),
    ("erf64", "PP0", 0x3fe5ed6029cadfdb, 0x3fe55c057348325b),
    ("erf64", "QQ3", 0xbfeaffb8159c24fa, 0xbfe88cf50252c3ad),
    ("erf64", "QQ2", 0xbfea73678699433e, 0xbfe83e22641ee987),
    ("erf64", "QQ1", 0x3fe8ac3ec95e6be2, 0x3fe72ec8e2fcaa84),
    ("erf64", "QQ0", 0xbfeafd4cad634b90, 0xbfe88b9df0193910),
    ("erf64", "PA5", 0x3ff3c131abb4ac6e, 0x3fed6a1d13573a8f),
    ("erf64", "QA5", 0xbff39370ea76a4e7, 0xbfed535169ee84d4),
    ("erf64", "QA4", 0xbff3f9569768265d, 0xbfed853d7a562ed6),
    ("erf64", "SA7", 0xbff4b524a7f0b2bd, 0xbfedd977c0c29ea9),
    ("erf64", "RB4", 0xc00708ff44f93409, 0xbfefff9e4811776f),
    ("erf64", "RB3", 0x4008f80b737a41a3, 0x3fefffeab6079505),
    ("erf64", "RB2", 0xc007ad1b6f9dee31, 0xbfefffc44d391fee),
    ("erf64", "RB1", 0xc0074c2661aff032, 0xbfefffb00200683f),
    ("erf64", "SB3", 0xc0075f6d2c325817, 0xbfefffb480c8d858),
    ("erf64", "SB2", 0x4007078eccafc850, 0x3fefff9ddd141c4f),
    ("erf64", "SB0", 0xc00a550978a661f5, 0xbfeffff933ec9a0c),
    ("erf64", "QQ4", 0x3fe88bdbaf4bf6b6, 0x3fe71a8e5199e0a9),
    ("erf64", "RB5", 0x40073f52db29fcf0, 0x3feffface091c45c),
    ("erf64", "SB5", 0x40073c124ad487d7, 0x3fefffac10adcbb1),
    ("erf64", "SB4", 0x4007028686fd0c08, 0x3fefff9c6397bcaf),
    ("erf64", "SB6", 0x4006de6f769f3cbf, 0x3fefff912e882416),
    // end erf64
];

/// `("powf64", step the input separates, x bits, y bits, expected output bits)` (tools/contraction-search
/// --pow-erf: the `e_pow` log polynomial `L1` … `L5` and exponential polynomial `P1` … `P4`)
const CASES_POW: &[(&str, &str, u64, u64, u64)] = &[
    (
        "powf64",
        "L1",
        0x3ff370d96ab8e0c5,
        0xc090a7e0bc8f47f8,
        0x2d35c5eaeb55f15c,
    ),
    (
        "powf64",
        "P1",
        0x3ff33c1ddac6f828,
        0xc0a97b98add06160,
        0x09c6b83b95040e06,
    ),
    (
        "powf64",
        "L2",
        0x3ff3764bbde7337f,
        0xc0a0fd8ed9e193b9,
        0x198561f46987e92e,
    ),
    (
        "powf64",
        "L3",
        0x3ff38832f5fd70f4,
        0x4097dc76c5d0d4c1,
        0x5b65f9fa6c95eba0,
    ),
    (
        "powf64",
        "P3",
        0x3ff36da82300c09f,
        0x4091d3a7d3778e8c,
        0x53e7968d3e5f7619,
    ),
    (
        "powf64",
        "P2",
        0x3ffb5d83e44bcf41,
        0x408d90aa12147029,
        0x6db715ac12bb3e07,
    ),
    (
        "powf64",
        "L4",
        0x3ff3aee4ab032da1,
        0x40a824253e7cbc7d,
        0x79a82ce5cfcdf2c9,
    ),
    (
        "powf64",
        "L5",
        0x3ff3988d3f290452,
        0xc0aa9b5a6003eb01,
        0x01adfec0c3280a97,
    ),
    (
        "powf64",
        "P4",
        0x3ff8c49245ad0e4c,
        0xbfe95fc0b2a3297b,
        0x3fe6a127ef2294b3,
    ),
];

fn eval(name: &str, x: f64) -> f64 {
    match name {
        "exp64" => dm::exp64(x),
        "atan64" => dm::atan64(x),
        "asin64" => dm::asin64(x),
        "acos64" => dm::acos64(x),
        "ln64" => dm::ln64(x),
        "log10_64" => dm::log10_64(x),
        "erf64" => dm::erf64(x),
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

#[test]
fn powf64_contraction_pins() {
    let mut compared = 0;
    let mut wrong = Vec::new();
    for &(name, step, x, y, want) in CASES_POW {
        let got = dm::powf64(f64::from_bits(x), f64::from_bits(y)).to_bits();
        if got != want {
            wrong.extend([format!(
                "{name}({x:#018x}, {y:#018x}) [{step}] = {got:#018x}, pinned {want:#018x}"
            )]);
        }
        compared += 1;
    }
    assert!(
        wrong.is_empty(),
        "{} of {compared} pins moved:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert_eq!(compared, CASES_POW.len());
    assert!(compared > 0, "compared nothing");
}
