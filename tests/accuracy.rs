//! Accuracy of every function against a correctly rounded reference: the
//! platform `f64` libm (≤ 1 ulp of 53 bits) rounded once to `f32`. Comparing
//! against the platform `f32` libm directly would make the bound itself
//! platform-dependent (MSVC `cbrtf` and macOS `cbrtf` disagree by 1 ulp).
//!
//! These are the *oracle* tests (the value is right); `golden.rs` pins the
//! bits (the value is the same everywhere).

#![allow(clippy::cast_precision_loss, clippy::cast_lossless)]

use alice_det_math::*;

fn ulp_diff32(a: f32, b: f32) -> u32 {
    if a == b {
        return 0;
    }
    let ia = a.to_bits() as i32;
    let ib = b.to_bits() as i32;
    (ia - ib).unsigned_abs()
}

fn ulp_diff64(a: f64, b: f64) -> u64 {
    if a == b {
        return 0;
    }
    let ia = a.to_bits() as i64;
    let ib = b.to_bits() as i64;
    (ia - ib).unsigned_abs()
}

fn r32(f: impl Fn(f64) -> f64) -> impl Fn(f32) -> f32 {
    move |x| f(f64::from(x)) as f32
}

/// Dense sweep helper: max ulp distance to the reference over `n` points in [lo, hi].
fn sweep32(lo: f32, hi: f32, n: u32, f: impl Fn(f32) -> f32, g: impl Fn(f32) -> f32) -> u32 {
    let mut worst = 0;
    for i in 0..=n {
        let x = lo + (hi - lo) * (i as f32 / n as f32);
        let a = f(x);
        let b = g(x);
        assert!(
            a.is_finite() == b.is_finite(),
            "finite mismatch at {x}: {a} vs {b}"
        );
        if a.is_finite() {
            worst = worst.max(ulp_diff32(a, b));
        }
    }
    worst
}

#[test]
fn sin_cos_within_2_ulp_of_correctly_rounded() {
    assert!(sweep32(-100.0, 100.0, 400_000, sin, r32(f64::sin)) <= 2);
    assert!(sweep32(-100.0, 100.0, 400_000, cos, r32(f64::cos)) <= 2);
    assert_eq!(sin(0.0), 0.0);
    assert_eq!(cos(0.0), 1.0);
    assert!(sin(f32::INFINITY).is_nan());
}

#[test]
fn sin_cos_pair_is_bit_identical_to_separate_calls() {
    for i in 0..=400_000u32 {
        let x = -100.0 + 200.0 * (i as f32 / 400_000.0);
        let (s, c) = sin_cos(x);
        assert_eq!(s.to_bits(), sin(x).to_bits(), "sin({x})");
        assert_eq!(c.to_bits(), cos(x).to_bits(), "cos({x})");
    }
    let (s, c) = sin_cos(f32::NAN);
    assert!(s.is_nan() && c.is_nan());
}

#[test]
fn atan_asin_acos_tan_tanh_within_1_ulp_of_correctly_rounded() {
    assert!(sweep32(-1e4, 1e4, 400_000, atan, r32(f64::atan)) <= 1);
    assert!(sweep32(-4.0, 4.0, 400_000, atan, r32(f64::atan)) <= 1);
    assert!(sweep32(-1.0, 1.0, 400_000, asin, r32(f64::asin)) <= 1);
    {
        let mut worst = 0;
        let mut wx = 0.0f32;
        for i in 0..=400_000u32 {
            let x = -1.0 + 2.0 * (i as f32 / 400_000.0);
            let d = ulp_diff32(acos(x), (f64::from(x)).acos() as f32);
            if d > worst {
                worst = d;
                wx = x;
            }
        }
        assert!(
            worst <= 1,
            "acos worst {worst} ulp at {wx}: {} vs {}",
            acos(wx),
            (f64::from(wx)).acos() as f32
        );
    }
    assert!(sweep32(-100.0, 100.0, 400_000, tan, r32(f64::tan)) <= 1);
    assert!(sweep32(-12.0, 12.0, 400_000, tanh, r32(f64::tanh)) <= 1);
    assert!(sweep32(-1e-3, 1e-3, 100_000, tanh, r32(f64::tanh)) <= 1);
    // atan2: every quadrant, axes, mixed magnitudes
    let mut worst = 0;
    for iy in -200i32..=200 {
        for ix in -200i32..=200 {
            let (y, x) = (iy as f32 * 0.37, ix as f32 * 1.13);
            let a = atan2(y, x);
            let b = (f64::from(y)).atan2(f64::from(x)) as f32;
            worst = worst.max(ulp_diff32(a, b));
        }
    }
    assert!(worst <= 1, "atan2 worst {worst} ulp");
    for &(y, x) in &[
        (0.0f32, 1.0f32),
        (0.0, -1.0),
        (-0.0, -1.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (1e30, 1e-30),
        (1e-30, -1e30),
    ] {
        let a = atan2(y, x);
        let b = (f64::from(y)).atan2(f64::from(x)) as f32;
        assert!(
            a.to_bits() == b.to_bits() || ulp_diff32(a, b) <= 1,
            "atan2({y}, {x}) = {a} vs {b}"
        );
    }
    assert!(asin(1.5).is_nan() && acos(-1.5).is_nan() && tan(f32::INFINITY).is_nan());
    assert_eq!(asin(1.0), core::f32::consts::FRAC_PI_2);
    assert_eq!(acos(1.0), 0.0);
    assert_eq!(tanh(0.0), 0.0);
    assert_eq!(tanh(50.0), 1.0);
    assert_eq!(tanh(-50.0), -1.0);
    // double-precision entry points against the platform libm (≤ 1 ulp slack, see crate docs)
    let mut worst64 = 0;
    for i in 0..=200_000 {
        let x = -1e6 + 2e6 * (i as f64 / 200_000.0);
        worst64 = worst64.max(ulp_diff64(atan64(x), x.atan()));
    }
    assert!(worst64 <= 1, "atan64 worst {worst64} ulp");
}

#[test]
fn exp_within_2_ulp_of_correctly_rounded() {
    assert!(sweep32(-87.0, 88.0, 400_000, exp, r32(f64::exp)) <= 2);
    assert_eq!(exp(0.0), 1.0);
    assert_eq!(exp(100.0), f32::INFINITY);
    assert_eq!(exp(-200.0), 0.0);
    // subnormal tail is gradual
    assert!(exp(-100.0) > 0.0 && exp(-100.0) < f32::MIN_POSITIVE);
}

#[test]
fn ln_within_1_ulp_of_correctly_rounded() {
    // log-spaced sweep
    let mut worst = 0;
    for i in 0..=200_000u32 {
        let x = 10f32.powf(-30.0 + 60.0 * (i as f32 / 200_000.0));
        worst = worst.max(ulp_diff32(ln(x), r32(f64::ln)(x)));
    }
    assert!(worst <= 1, "ln worst ulp {worst}");
    assert_eq!(ln(1.0), 0.0);
    assert_eq!(ln(0.0), f32::NEG_INFINITY);
    assert!(ln(-1.0).is_nan());
    // subnormal input
    assert!(ulp_diff32(ln(1.0e-40), r32(f64::ln)(1.0e-40)) <= 1);
}

/// Every `f32` power of two, including the subnormal ones, built from bit
/// patterns (a grid made with the platform `powf` would itself vary).
fn powers_of_two32() -> Vec<(i32, f32)> {
    (-149..=127)
        .map(|k| {
            let x = if k >= -126 {
                f32::from_bits(((k + 127) as u32) << 23)
            } else {
                f32::from_bits(1u32 << (k + 149))
            };
            (k, x)
        })
        .collect()
}

/// Every `f64` power of two, including the subnormal ones.
fn powers_of_two64() -> Vec<(i32, f64)> {
    (-1074..=1023)
        .map(|k| {
            let x = if k >= -1022 {
                f64::from_bits(((k + 1023) as u64) << 52)
            } else {
                f64::from_bits(1u64 << (k + 1074))
            };
            (k, x)
        })
        .collect()
}

#[test]
fn log2_log10_within_1_ulp_of_correctly_rounded() {
    // oracle: the f64 libm rounded once to f32 (correctly rounded f32)
    let mut worst2 = 0;
    let mut worst10 = 0;
    for i in 0..=200_000u32 {
        let x = 10f32.powf(-30.0 + 60.0 * (i as f32 / 200_000.0));
        worst2 = worst2.max(ulp_diff32(log2(x), r32(f64::log2)(x)));
        worst10 = worst10.max(ulp_diff32(log10(x), r32(f64::log10)(x)));
    }
    println!("log2 max {worst2} ulp, log10 max {worst10} ulp");
    assert!(worst2 <= 1, "log2 worst ulp {worst2}");
    assert!(worst10 <= 1, "log10 worst ulp {worst10}");
    // the interval around 1, where `y + log2(m)` would cancel without the
    // reduction that keeps `m` below 1 for a negative exponent
    let mut worst_near1 = 0;
    for i in 0..=200_000u32 {
        let x = 0.5 + 1.0 * (i as f32 / 200_000.0);
        worst_near1 = worst_near1.max(ulp_diff32(log2(x), r32(f64::log2)(x)));
    }
    assert!(worst_near1 <= 1, "log2 near 1 worst ulp {worst_near1}");

    // closed form: log2 of a power of two is its exponent, exactly
    let pows = powers_of_two32();
    assert_eq!(pows.len(), 277);
    for (k, x) in pows {
        assert_eq!(log2(x), k as f32, "log2(2^{k}) = {}", log2(x));
    }
    assert_eq!(log2(1.0), 0.0);
    assert_eq!(log2(2.0), 1.0);
    assert_eq!(log2(0.5), -1.0);
    assert_eq!(log2(1024.0), 10.0);
    // closed form: log10 of 10^k for the k where 10^k is an exact f32
    for k in 0..=10i32 {
        let x = 10f32.powi(k);
        assert_eq!(log10(x), k as f32, "log10(10^{k}) = {}", log10(x));
    }
    assert_eq!(log10(1.0), 0.0);
    assert_eq!(log10(10.0), 1.0);
    assert_eq!(log10(100.0), 2.0);
    // the identity that makes these logarithms, checked against ln
    let mut worst_id = 0;
    for i in 0..=100_000u32 {
        let x = 10f32.powf(-20.0 + 40.0 * (i as f32 / 100_000.0));
        let via_ln = (f64::from(ln(x)) * core::f64::consts::LOG2_E) as f32;
        worst_id = worst_id.max(ulp_diff32(log2(x), via_ln));
    }
    assert!(worst_id <= 2, "log2 vs ln·log2(e) worst ulp {worst_id}");
}

#[test]
fn log2_64_log10_64_within_2_ulp_of_libm() {
    let mut worst2 = (0u64, 0.0f64);
    let mut worst10 = (0u64, 0.0f64);
    for i in 0..=400_000u32 {
        let x = 10f64.powf(-300.0 + 600.0 * (f64::from(i) / 400_000.0));
        let d2 = ulp_diff64(log2_64(x), x.log2());
        let d10 = ulp_diff64(log10_64(x), x.log10());
        if d2 > worst2.0 {
            worst2 = (d2, x);
        }
        if d10 > worst10.0 {
            worst10 = (d10, x);
        }
    }
    println!(
        "log2_64 max {} ulp (at {:e}), log10_64 max {} ulp (at {:e})",
        worst2.0, worst2.1, worst10.0, worst10.1
    );
    assert!(worst2.0 <= 2, "log2_64 {} ulp at {:e}", worst2.0, worst2.1);
    assert!(
        worst10.0 <= 2,
        "log10_64 {} ulp at {:e}",
        worst10.0,
        worst10.1
    );
    // around 1, where the exponent and the fraction would cancel
    let mut worst_near1 = 0;
    for i in 0..=400_000u32 {
        let x = 0.5 + 1.0 * (f64::from(i) / 400_000.0);
        worst_near1 = worst_near1.max(ulp_diff64(log2_64(x), x.log2()));
    }
    assert!(worst_near1 <= 2, "log2_64 near 1 worst ulp {worst_near1}");

    // closed form: exact on every power of two, down to the last subnormal
    let pows = powers_of_two64();
    assert_eq!(pows.len(), 2098);
    for (k, x) in pows {
        assert_eq!(log2_64(x), f64::from(k), "log2_64(2^{k}) = {}", log2_64(x));
    }
    assert_eq!(log2_64(1.0), 0.0);
    assert_eq!(log2_64(2.0), 1.0);
    assert_eq!(log2_64(0.5), -1.0);
    assert_eq!(log2_64(f64::MIN_POSITIVE), -1022.0);
    assert_eq!(log2_64(f64::from_bits(1)), -1074.0);
    // closed form: 10^k is an exact f64 for 0 ≤ k ≤ 22
    for k in 0..=22i32 {
        let x = 10f64.powi(k);
        assert_eq!(log10_64(x), f64::from(k), "log10_64(10^{k})");
    }
    assert_eq!(log10_64(1.0), 0.0);
    // log2(x) = log10(x) / log10(2) up to the conversion's own rounding
    let mut worst_id = 0;
    for i in 0..=100_000u32 {
        let x = 10f64.powf(-200.0 + 400.0 * (f64::from(i) / 100_000.0));
        let d = (log2_64(x) * core::f64::consts::LOG10_2 - log10_64(x)).abs();
        let scale = log10_64(x).abs().max(1.0);
        worst_id = worst_id.max((d / scale * 1.0e18) as u64);
    }
    assert!(worst_id < 1_000, "log2/log10 identity drift {worst_id}e-18");
}

/// `(x, tan x)` as `f64` bit patterns, correctly rounded by an independent
/// high-precision evaluation — *not* by a libm, which is off by up to 3 ulp at
/// several of these points (measured on macOS: 3 ulp at `x = −7.59915`,
/// `−99.214`, `−924500`, `−9.52435e17`). Reproduce with
///
/// ```text
/// python3 -c 'import mpmath,struct; mpmath.mp.prec=2400
/// b=lambda v: struct.unpack("<Q", struct.pack("<d", v))[0]
/// f=lambda h: struct.unpack("<d", struct.pack("<Q", h))[0]
/// print([hex(b(float(mpmath.tan(mpmath.mpf(f(h)))))) for h in [0x3fb999999999999a]])'
/// ```
///
/// The inputs cover the `2^-27` shortcut, both sides of the kernel's `0.6744`
/// branch, the `small` reduction (1 to 4 multiples of π/2), the Cody–Waite
/// region and its `2^20·π/2` threshold, the Payne–Hanek region up to
/// `f64::MAX`, the three `f64` closest to π/2, and negatives.
const TAN64_REFERENCE: &[(u64, u64)] = &[
    (0x0000_0000_0000_0001, 0x0000_0000_0000_0001),
    (0x000f_ffff_ffff_ffff, 0x000f_ffff_ffff_ffff),
    (0x0010_0000_0000_0000, 0x0010_0000_0000_0000),
    (0x3e30_0000_0000_0000, 0x3e30_0000_0000_0000),
    (0x3e40_0000_0000_0000, 0x3e40_0000_0000_0000),
    (0x3e50_0000_0000_0000, 0x3e50_0000_0000_0000),
    (0x3bc7_9ca1_0c92_4223, 0x3bc7_9ca1_0c92_4223),
    (0x3fb9_9999_9999_999a, 0x3fb9_af88_7743_0b80),
    (0x3fe0_0000_0000_0000, 0x3fe1_7b4f_5bf3_474a),
    (0x3fe5_93dd_97f6_2b6b, 0x3fe9_93ad_96d4_80cb),
    (0x3fe5_94af_4f0d_844d, 0x3fe9_9505_4ea0_0c37),
    (0x3fe5_9581_0624_dd2f, 0x3fe9_965d_147d_8180),
    (0x3fe6_6666_6666_6666, 0x3fea_f406_c2fc_78ae),
    (0x3fe9_21fb_5444_2d18, 0x3fef_ffff_ffff_ffff),
    (0x3fe9_21fb_5444_2d15, 0x3fef_ffff_ffff_fff9),
    (0x3ff7_5f9a_6049_a4d2, 0x4022_1da3_52e0_8a44),
    (0x3ff9_e300_4f1d_42ed, 0xc035_3477_861d_1174),
    (0x4007_5f9a_6049_a4d2, 0xbfcc_9c84_0aea_a4e9),
    (0x4009_e300_4f1d_42ed, 0x3fb8_32f9_d77a_112d),
    (0x4011_87b3_c837_3b9d, 0x4007_5db7_f003_c4a6),
    (0x4013_6a40_3b55_f232, 0xc01c_1aff_bfad_ce6f),
    (0x4017_5f9a_6049_a4d2, 0xbfde_1db9_a59e_3b3a),
    (0x4019_e300_4f1d_42ed, 0x3fc8_6ad4_8ef4_1b7d),
    (0x401d_3780_f85c_0e06, 0x3ffa_1c10_83c2_3fcf),
    (0x4020_2de0_3172_49d4, 0xc010_a944_57ca_4c84),
    (0x4021_87b3_c837_3b9d, 0xbfe8_d25d_d684_54df),
    (0x4023_6a40_3b55_f232, 0x3fd2_97fd_f511_7d0e),
    (0x4024_73a7_1440_7037, 0x3ff0_82be_7e23_fec8),
    (0x4026_a6a0_4539_9a90, 0xc007_5db7_f003_c4a0),
    (0x4027_5f9a_6049_a4d2, 0xbff3_5736_a5ca_a0b3),
    (0x4029_e300_4f1d_42ed, 0x3fd9_56e2_6e56_d2ae),
    (0x402a_4b8d_ac52_d96c, 0x3fe5_0523_6176_6d19),
    (0x402d_1f60_5900_eb4b, 0xc001_b7d1_34c6_37e8),
    (0x402b_574b_c6a7_ef9e, 0x3fff_c288_f97c_d9f1),
    (0x401e_6587_93dd_97f6, 0x400e_b5f4_b8c5_94f2),
    (0x4058_cdb2_2d0e_5604, 0xc00e_d650_f28a_a836),
    (0x4059_0000_0000_0000, 0xbfe2_ca74_d62b_5d38),
    (0x408f_4000_0000_0000, 0x3ff7_8672_9f34_311a),
    (0x40c8_1cd6_c8b4_3958, 0xbfef_ba58_3632_3a4e),
    (0x4129_21fb_5444_2d18, 0xbdc1_a626_3314_5c07),
    (0x4139_1b8c_3ad6_8a3c, 0xbff4_5c92_4ea0_d2a9),
    (0x4139_286a_6db1_cff4, 0x3ff4_5c92_4e89_b4fd),
    (0x41cd_cd65_0000_0000, 0x3fe4_d8b2_49e3_dba7),
    (0x412c_36a8_0000_0000, 0x403f_202c_37a6_7b9a),
    (0x43aa_6f75_2c2d_4c60, 0x3fef_385a_ab4e_e9fa),
    (0x4480_f0cf_064d_d592, 0xbffa_0f79_c1b6_b257),
    (0x54b2_49ad_2594_c37d, 0xbfda_5807_d6f7_6f7d),
    (0x7e37_e43c_8800_759c, 0x3ff6_be41_1f37_ac77),
    (0x7fef_ffff_ffff_ffff, 0xbf74_530c_fe72_9484),
    (0x7fe0_0000_0000_0000, 0xbfe5_ce6b_4c0d_02a3),
    (0x3ff9_21fb_5444_2d18, 0x434d_0296_7c31_cdb5),
    (0x3ff9_21fb_5444_2d17, 0x4329_153d_9443_ed0b),
    (0x3ff9_21fb_5444_2d19, 0xc336_17a1_5494_767a),
    (0x4012_d97c_7f33_21d2, 0x4333_570e_fd76_8923),
    (0x401f_6a7a_2955_385e, 0x4327_3545_3027_d7c4),
    (0xbfb9_9999_9999_999a, 0xbfb9_af88_7743_0b80),
    (0xbfe5_94af_4f0d_844d, 0xbfe9_9505_4ea0_0c37),
    (0xbfe6_6666_6666_6666, 0xbfea_f406_c2fc_78ae),
    (0xc02b_574b_c6a7_ef9e, 0xbfff_c288_f97c_d9f1),
    (0xc058_cdb2_2d0e_5604, 0x400e_d650_f28a_a836),
    (0xc1cd_cd65_0000_0000, 0xbfe4_d8b2_49e3_dba7),
    (0xd4b2_49ad_2594_c37d, 0x3fda_5807_d6f7_6f7d),
    // more of the Payne–Hanek region, including the two binades where every
    // f64 is an integer (2^52, 2^53) and both signs of large magnitudes
    (0x4202_a05f_2000_0000, 0xbfe1_de00_0f44_3f50), // 1e10
    (0x426d_1a94_a200_0000, 0xbfe8_b6bb_0174_398f), // 1e12
    (0x430c_6bf5_2634_0000, 0xbffa_c236_00a9_5be4), // 1e15
    (0x4330_0000_0000_0000, 0xbffc_cef2_838d_a5ca), // 2^52
    (0x4340_0000_0000_0000, 0x3ff9_b33a_f5ae_241f), // 2^53
    (0x43ab_c16d_674e_c800, 0xc020_c6ef_fbd6_0ad2), // 1e18
    (0xc3ab_c16d_674e_c800, 0x4020_c6ef_fbd6_0ad2), // −1e18
    (0x4520_8b2a_2c28_0291, 0xbfd4_8406_33f9_3616), // 1e25
    (0x4a51_1b0e_c57e_649a, 0xbfe1_8859_16b6_549d), // 1e50
    (0xdf13_8d35_2e50_96af, 0x3fee_8eff_dfae_250d), // −1e150
    (0x6974_e718_d7d7_625a, 0xbfea_ef78_45d4_1e88), // 1e200
    (0x7830_0000_0000_0000, 0x402c_074d_a74b_10f0), // 2^900
    (0xfe70_0000_0000_0000, 0x3fc4_a41d_560c_08cc), // −2^1000
    (0x422d_4223_fc1f_977b, 0xbed2_c8fe_b2a2_f182), // 2π·1e10, a near-multiple of π
];

const TAN64_REFERENCE_FILE: &str = include_str!("data/tan64_reference.txt");

/// `(x, tan x)` rows of the committed tangent reference, correctly rounded by
/// the same independent 2400-bit evaluation as the sine / cosine table and
/// over the same inputs — not by a libm, which disagrees with the true value
/// by up to ~1.0e5 ulp in the Payne–Hanek range depending on the platform.
fn tan64_reference() -> Vec<(f64, f64)> {
    let rows: Vec<(f64, f64)> = TAN64_REFERENCE_FILE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut it = l
                .split_whitespace()
                .map(|h| f64::from_bits(u64::from_str_radix(h, 16).expect("hex")));
            (it.next().expect("x"), it.next().expect("tan"))
        })
        .collect();
    // a truncated or renamed table must not turn the oracle into a no-op
    assert!(rows.len() > 6000, "reference table has {} rows", rows.len());
    rows
}

#[test]
fn tan64_within_1_ulp_of_correctly_rounded_reference() {
    // a truncated table must not turn the oracle into a no-op
    assert_eq!(TAN64_REFERENCE.len(), 76);
    let mut worst = (0u64, 0.0f64);
    let mut huge = 0;
    for &(xb, tb) in TAN64_REFERENCE {
        let x = f64::from_bits(xb);
        let d = ulp_diff64(tan64(x), f64::from_bits(tb));
        if d > worst.0 {
            worst = (d, x);
        }
        if x.abs() > 1.0e9 {
            huge += 1;
        }
    }
    // the large-argument (Payne–Hanek) path must actually be exercised
    assert!(huge >= 10, "only {huge} rows above 1e9");
    println!(
        "tan64 max {} ulp vs the high-precision reference (at {:e})",
        worst.0, worst.1
    );
    assert!(worst.0 <= 1, "tan64 {} ulp at {:e}", worst.0, worst.1);

    // the committed table, over the same inputs as the sine / cosine
    // reference: 6234 points, 1964 of them in the Payne–Hanek range
    let mut worst_t = (0u64, 0.0f64);
    let mut huge_t = 0;
    let rows = tan64_reference();
    for (x, t) in &rows {
        let d = ulp_diff64(tan64(*x), *t);
        if d > worst_t.0 {
            worst_t = (d, *x);
        }
        if x.abs() > 1.0e9 {
            huge_t += 1;
        }
    }
    assert!(huge_t > 500, "only {huge_t} rows above 1e9");
    println!(
        "tan64 max {} ulp over {} reference rows ({huge_t} above 1e9, at {:e})",
        worst_t.0,
        rows.len(),
        worst_t.1
    );
    assert!(worst_t.0 <= 1, "tan64 {} ulp at {:e}", worst_t.0, worst_t.1);

    // The platform libm is *not* a reference here and is deliberately not
    // asserted against: glibc is ~1.0e5 ulp off at x = 0x6404c96c11134d36
    // (verified against 2400 bits: this crate is exact there and glibc is
    // not), while macOS is within 2 ulp, so any bound on this number would
    // pass or fail according to which runner executed it — a platform-
    // dependent gate in a crate whose whole claim is platform independence.
    // It is printed because the size of the gap is worth seeing.
    let mut worst_libm = (0u64, 0.0f64);
    for (x, _) in &rows {
        let d = ulp_diff64(tan64(*x), x.tan());
        if d > worst_libm.0 {
            worst_libm = (d, *x);
        }
    }
    println!(
        "tan64 vs the platform libm: max {} ulp at {:e} (reported, not asserted)",
        worst_libm.0, worst_libm.1
    );
    // closed form: tan is odd, bitwise
    for x in identity_inputs() {
        let x = x.abs();
        assert_eq!(tan64(-x).to_bits(), (-tan64(x)).to_bits(), "tan64(-{x:e})");
    }
    // closed form: tan = sin/cos (the quotient costs an extra rounding and
    // loses relative accuracy where cos is near zero, so the bound is looser)
    let mut worst_q = 0;
    for i in 0..=400_000u32 {
        let x = -20.0 + 40.0 * (f64::from(i) / 400_000.0);
        let (s, c) = sin_cos64(x);
        let q = s / c;
        if q.is_finite() {
            worst_q = worst_q.max(ulp_diff64(tan64(x), q));
        }
    }
    println!("tan64 vs sin64/cos64 max {worst_q} ulp");
    assert!(worst_q <= 4, "tan64 vs sin64/cos64 worst ulp {worst_q}");
    assert_eq!(tan64(0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(tan64(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(tan64(core::f64::consts::FRAC_PI_4), 0.999_999_999_999_999_9);
    assert_eq!(
        tan64(core::f64::consts::PI).to_bits(),
        (-1.224_646_799_147_353_2e-16f64).to_bits()
    );
    // subnormals and the tiny-argument shortcut: tan x = x
    for b in [1u64, 2, 0x000f_ffff_ffff_ffff, 0x0008_0000_0000_0000] {
        for x in [f64::from_bits(b), -f64::from_bits(b)] {
            assert_eq!(tan64(x).to_bits(), x.to_bits());
        }
    }
}

/// The domain edges of the new logarithms and the pole of the tangent: what
/// each one returns is fixed here, and it is what [`ln`] / [`ln64`] already
/// do at the same inputs (`-inf` at zero, the canonical NaN below it) — a
/// logarithm that disagreed with `ln` about its own domain would be worse
/// than one that is a ulp off.
#[test]
fn logarithm_domain_edges_and_tangent_poles_are_pinned() {
    let canon32 = f32::NAN.to_bits();
    let canon64 = f64::NAN.to_bits();

    // zero (both signs) is -inf, as for `ln`
    for x in [0.0f32, -0.0] {
        assert_eq!(ln(x), f32::NEG_INFINITY);
        assert_eq!(log2(x), f32::NEG_INFINITY, "log2({x})");
        assert_eq!(log10(x), f32::NEG_INFINITY, "log10({x})");
    }
    for x in [0.0f64, -0.0] {
        assert_eq!(ln64(x), f64::NEG_INFINITY);
        assert_eq!(log2_64(x), f64::NEG_INFINITY, "log2_64({x})");
        assert_eq!(log10_64(x), f64::NEG_INFINITY, "log10_64({x})");
    }
    // negative (including -inf and the smallest subnormal) is the canonical NaN
    for x in [
        -1.0f32,
        -f32::MIN_POSITIVE,
        -f32::from_bits(1),
        -f32::MAX,
        f32::NEG_INFINITY,
    ] {
        assert_eq!(ln(x).to_bits(), canon32);
        assert_eq!(log2(x).to_bits(), canon32, "log2({x:e})");
        assert_eq!(log10(x).to_bits(), canon32, "log10({x:e})");
    }
    for x in [
        -1.0f64,
        -f64::MIN_POSITIVE,
        -f64::from_bits(1),
        -f64::MAX,
        f64::NEG_INFINITY,
    ] {
        assert_eq!(ln64(x).to_bits(), canon64);
        assert_eq!(log2_64(x).to_bits(), canon64, "log2_64({x:e})");
        assert_eq!(log10_64(x).to_bits(), canon64, "log10_64({x:e})");
    }
    // +inf is +inf
    assert_eq!(log2(f32::INFINITY), f32::INFINITY);
    assert_eq!(log10(f32::INFINITY), f32::INFINITY);
    assert_eq!(log2_64(f64::INFINITY), f64::INFINITY);
    assert_eq!(log10_64(f64::INFINITY), f64::INFINITY);
    // the extremes of the finite range stay finite (no overflow in the
    // exponent reconstruction, and the subnormal pre-scaling does not
    // flush to zero)
    for x in [
        f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x007f_ffff),
    ] {
        assert!(log2(x).is_finite(), "log2({x:e}) = {}", log2(x));
        assert!(log10(x).is_finite(), "log10({x:e}) = {}", log10(x));
    }
    for x in [
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(0x000f_ffff_ffff_ffff),
    ] {
        assert!(log2_64(x).is_finite(), "log2_64({x:e})");
        assert!(log10_64(x).is_finite(), "log10_64({x:e})");
    }

    // the tangent has a pole at every odd multiple of π/2, but no f64 *is*
    // one: the value at the nearest f64 is large and finite, and it is the
    // tangent of that f64 (not of the pole)
    let pole = core::f64::consts::FRAC_PI_2;
    assert!(tan64(pole).is_finite(), "tan64(π/2) = {}", tan64(pole));
    assert!(tan64(pole) > 1.0e16, "tan64(π/2) = {}", tan64(pole));
    // the correctly rounded value, from the reference table rather than from
    // the platform libm (which differs by runner)
    let reference = |x: f64| -> f64 {
        let (_, t) = TAN64_REFERENCE
            .iter()
            .map(|&(xb, tb)| (f64::from_bits(xb), f64::from_bits(tb)))
            .find(|&(rx, _)| rx.to_bits() == x.to_bits())
            .expect("the pole neighbourhood is in TAN64_REFERENCE");
        t
    };
    assert_eq!(tan64(pole).to_bits(), reference(pole).to_bits());
    for m in [1i32, 3, 5] {
        let x = f64::from(m) * pole;
        assert!(
            ulp_diff64(tan64(x), reference(x)) <= 1,
            "tan64({m}·π/2) = {}",
            tan64(x)
        );
    }
    // every one of the first nine stays finite, and the odd multiples (where
    // the pole is) are the large ones
    for m in 1..=9i32 {
        let x = f64::from(m) * pole;
        let t = tan64(x);
        assert!(t.is_finite(), "tan64({m}·π/2) = {t}");
        if m % 2 == 1 {
            assert!(t.abs() > 1.0e15, "tan64({m}·π/2) = {t}");
        } else {
            assert!(t.abs() < 1.0e-15, "tan64({m}·π/2) = {t}");
        }
    }
    // and the neighbours of the nearest f64 straddle the pole: the sign flips
    let below = f64::from_bits(pole.to_bits() - 1);
    let above = f64::from_bits(pole.to_bits() + 1);
    assert!(tan64(below) > 0.0 && tan64(above) < 0.0);
    // ±inf is the canonical NaN, as for sin64 / cos64
    for x in [f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(tan64(x).to_bits(), canon64, "tan64({x})");
    }
}

#[test]
fn cbrt_within_1_ulp_of_correctly_rounded() {
    let mut worst = 0;
    for i in 0..=200_000u32 {
        let x = 10f32.powf(-30.0 + 60.0 * (i as f32 / 200_000.0));
        worst = worst.max(ulp_diff32(cbrt(x), r32(f64::cbrt)(x)));
        worst = worst.max(ulp_diff32(cbrt(-x), r32(f64::cbrt)(-x)));
    }
    assert!(worst <= 1, "cbrt worst ulp {worst}");
    assert_eq!(cbrt(0.0), 0.0);
    assert_eq!(cbrt(8.0), 2.0);
    assert_eq!(cbrt(-27.0), -3.0);
    assert!(ulp_diff32(cbrt(1.0e-40), r32(f64::cbrt)(1.0e-40)) <= 1);
    // huge inputs: the Newton step's y³ must not overflow (was inf/inf = NaN)
    for &x in &[f32::MAX, 3.168_086_2e38, 1.0e37, 7.922_816_3e28, -f32::MAX] {
        assert!(cbrt(x).is_finite(), "cbrt({x:e}) = {}", cbrt(x));
        assert!(ulp_diff32(cbrt(x), r32(f64::cbrt)(x)) <= 1, "cbrt({x:e})");
    }
}

#[test]
fn hypot_matches_sqrt_formula_and_scales() {
    assert_eq!(hypot(3.0, 4.0), 5.0);
    assert_eq!(hypot(0.0, 0.0), 0.0);
    assert_eq!(hypot(-3.0, 4.0), 5.0);
    // libm hypot is correctly rounded; ours is sqrt(x²+y²) → ≤ 1 ulp apart
    let mut worst = 0;
    for i in 0..=100_000u32 {
        let x = 1.0e-3 + 1.0e3 * (i as f32 / 100_000.0);
        let y = 1.0e3 - x;
        worst = worst.max(ulp_diff32(
            hypot(x, y),
            (f64::from(x).hypot(f64::from(y))) as f32,
        ));
    }
    assert!(worst <= 1, "hypot worst ulp {worst}");
    // no overflow / underflow where the naive formula would fail
    assert!(hypot(1.0e30, 1.0e30).is_finite());
    assert!(hypot(1.0e-30, 1.0e-30) > 0.0);
}

#[test]
fn powf_within_1_ulp_of_correctly_rounded() {
    let mut worst = 0;
    for i in 0..=400u32 {
        let x = 10f32.powf(-3.0 + 6.0 * (i as f32 / 400.0));
        for j in 0..=160u32 {
            let y = -8.0 + 16.0 * (j as f32 / 160.0);
            worst = worst.max(ulp_diff32(
                powf(x, y),
                (f64::from(x).powf(f64::from(y))) as f32,
            ));
        }
    }
    assert!(worst <= 1, "powf worst ulp {worst}");
    assert_eq!(powf(2.0, 0.0), 1.0);
    assert_eq!(powf(0.0, 2.0), 0.0);
    assert_eq!(powf(0.0, -1.0), f32::INFINITY);
    assert!(powf(-2.0, 0.5).is_nan());
}

#[test]
fn powi_matches_repeated_multiplication() {
    for n in 0..=12 {
        let mut expect = 1.0f32;
        for _ in 0..n {
            expect *= 1.7;
        }
        // repeated squaring differs from a left fold by rounding; accept ≤ 2 ulp
        assert!(ulp_diff32(powi(1.7, n), expect) <= 2, "n = {n}");
    }
    assert_eq!(powi(2.0, 10), 1024.0);
    assert_eq!(powi(2.0, -2), 0.25);
    assert_eq!(powi(5.0, 0), 1.0);
}

#[test]
fn exp64_ln64_within_2_ulp_of_libm() {
    let mut worst_e = 0;
    for i in 0..=400_000u32 {
        let x = -700.0 + 1400.0 * (f64::from(i) / 400_000.0);
        worst_e = worst_e.max(ulp_diff64(exp64(x), x.exp()));
    }
    assert!(worst_e <= 2, "exp64 worst ulp {worst_e}");
    let mut worst_l = 0;
    for i in 0..=400_000u32 {
        let x = 10f64.powf(-300.0 + 600.0 * (f64::from(i) / 400_000.0));
        worst_l = worst_l.max(ulp_diff64(ln64(x), x.ln()));
    }
    assert!(worst_l <= 2, "ln64 worst ulp {worst_l}");
    assert_eq!(exp64(0.0), 1.0);
    assert_eq!(ln64(1.0), 0.0);
    assert_eq!(exp64(1000.0), f64::INFINITY);
    assert_eq!(exp64(-800.0), 0.0);
    assert_eq!(ln64(0.0), f64::NEG_INFINITY);
    assert!(ulp_diff64(ln64(1.0e-310), (1.0e-310f64).ln()) <= 2);
}

#[test]
fn powf64_within_16_ulp_of_libm() {
    let mut worst = 0;
    for i in 0..=400u32 {
        let x = 10f64.powf(-3.0 + 6.0 * (f64::from(i) / 400.0));
        for j in 0..=160u32 {
            let y = -8.0 + 16.0 * (f64::from(j) / 160.0);
            worst = worst.max(ulp_diff64(powf64(x, y), x.powf(y)));
        }
    }
    assert!(worst <= 16, "powf64 worst ulp {worst}");
}

/// Bit-level pins carried over from `alice-physics` 1.2.0 `det_math`: the
/// extraction must not have changed a single bit (the physics golden
/// scenarios depend on it).
#[test]
fn bit_exact_pins() {
    assert_eq!(sin(1.0f32).to_bits(), 0x3f57_6aa5);
    assert_eq!(cos(1.0f32).to_bits(), 0x3f0a_5140);
    assert_eq!(exp(1.0f32).to_bits(), 0x402d_f854);
    assert_eq!(ln(10.0f32).to_bits(), 0x4013_5d8e);
    assert_eq!(cbrt(10.0f32).to_bits(), 0x4009_e242);
    assert_eq!(powf(3.0f32, 2.5).to_bits(), 0x4179_6a52);
    assert_eq!(hypot(1.5f32, 2.5).to_bits(), 0x403a_9728);
    assert_eq!(exp64(1.0f64).to_bits(), 0x4005_bf0a_8b14_576a);
    assert_eq!(ln64(10.0f64).to_bits(), 0x4002_6bb1_bbb5_5516);
    assert_eq!(powf64(3.0f64, 2.5).to_bits(), 0x402f_2d4a_4563_563d);
}

/// A NaN input must produce a platform-independent NaN: either the canonical
/// `f32::NAN` / `f64::NAN` bits or (for the pass-through functions `round`
/// and `cbrt`) the input bits unchanged. Letting a NaN *propagate* through
/// arithmetic is not allowed — which payload / sign survives `1.0 - NaN`
/// differs between aarch64, x86 and wasm (found by the wasm golden lane).
#[test]
fn nan_inputs_yield_canonical_or_pass_through_nan() {
    let payloads32 = [
        0x7fc0_0001u32,
        0xffc0_0000,
        0x7f80_0001, // signalling
        0xffbf_ffff,
        0x7ffd_2ca3,
    ];
    let canon = f32::NAN.to_bits();
    for &b in &payloads32 {
        let x = f32::from_bits(b);
        let same = |name: &str, v: f32| {
            assert_eq!(
                v.to_bits(),
                canon,
                "{name}({b:#010x}) = {:#010x}",
                v.to_bits()
            );
        };
        same("sin", sin(x));
        same("cos", cos(x));
        same("sin_cos.0", sin_cos(x).0);
        same("sin_cos.1", sin_cos(x).1);
        same("exp", exp(x));
        same("ln", ln(x));
        same("log2", log2(x));
        same("log10", log10(x));
        same("powf", powf(x, 2.0));
        same("powf", powf(2.0, x));
        same("powi", powi(x, 3));
        same("powi", powi(x, -2));
        assert_eq!(powi(x, 0), 1.0);
        same("hypot", hypot(x, 1.0));
        same("hypot", hypot(1.0, x));
        same("atan", atan(x));
        same("atan2", atan2(x, 1.0));
        same("atan2", atan2(1.0, x));
        same("asin", asin(x));
        same("acos", acos(x));
        same("tan", tan(x));
        same("tanh", tanh(x));
        same("sqrt", sqrt(x));
        assert_eq!(round(x).to_bits(), b, "round passes NaN through");
        assert_eq!(cbrt(x).to_bits(), b, "cbrt passes NaN through");
    }
    let canon64 = f64::NAN.to_bits();
    for &b in &[
        0x7ff8_0000_0000_0001u64,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0x7ffd_2ca3_de58_4b97,
    ] {
        let x = f64::from_bits(b);
        for (name, v) in [
            ("atan64", atan64(x)),
            ("atan2_64", atan2_64(x, 1.0)),
            ("atan2_64", atan2_64(1.0, x)),
            ("asin64", asin64(x)),
            ("acos64", acos64(x)),
            ("exp64", exp64(x)),
            ("ln64", ln64(x)),
            ("log2_64", log2_64(x)),
            ("log10_64", log10_64(x)),
            ("tan64", tan64(x)),
            ("powf64", powf64(x, 2.0)),
            ("powf64", powf64(2.0, x)),
            ("sqrt64", sqrt64(x)),
        ] {
            assert_eq!(
                v.to_bits(),
                canon64,
                "{name}({b:#018x}) = {:#018x}",
                v.to_bits()
            );
        }
        assert_eq!(round64(x).to_bits(), b, "round64 passes NaN through");
    }
}

/// Beyond the single-precision reduction (`|x| > 2^24`) the trig functions
/// produce `±inf` / NaN by arithmetic; the NaN must be the canonical one, not
/// the platform default NaN (whose sign differs between `x86` and `AArch64` —
/// found by the `x86_64` golden lanes).
#[test]
fn huge_arguments_never_leak_a_platform_nan() {
    for &b in &[
        0x6b11_d792u32,
        0xeb11_d792,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x4f00_0000,
        0x5f00_0000,
        0x6f00_0000,
    ] {
        let x = f32::from_bits(b);
        for (name, v) in [
            ("sin", sin(x)),
            ("cos", cos(x)),
            ("sin_cos.0", sin_cos(x).0),
            ("sin_cos.1", sin_cos(x).1),
            ("tan", tan(x)),
        ] {
            assert!(
                !v.is_nan() || v.to_bits() == f32::NAN.to_bits(),
                "{name}({b:#010x}) = {:#010x} is a non-canonical NaN",
                v.to_bits()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// sin64 / cos64 / sin_cos64
// oracle: correctly rounded values from an independent high-precision
// evaluation (mpmath at 2400 bits, scripts/gen_sin_cos64_reference.py), not
// the platform libm — so the bound below carries no libm slack.
// ---------------------------------------------------------------------------

const SIN_COS64_REFERENCE: &str = include_str!("data/sin_cos64_reference.txt");

/// `(x, sin x, cos x)` rows of the committed reference table.
fn sin_cos64_reference() -> Vec<(f64, f64, f64)> {
    let rows: Vec<(f64, f64, f64)> = SIN_COS64_REFERENCE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut it = l
                .split_whitespace()
                .map(|h| f64::from_bits(u64::from_str_radix(h, 16).expect("hex")));
            (
                it.next().expect("x"),
                it.next().expect("sin"),
                it.next().expect("cos"),
            )
        })
        .collect();
    // a truncated or renamed table must not turn the oracle into a no-op
    assert!(rows.len() > 6000, "reference table has {} rows", rows.len());
    rows
}

#[test]
fn sin_cos64_within_1_ulp_of_correctly_rounded_reference() {
    let mut worst_s = (0u64, 0.0f64);
    let mut worst_c = (0u64, 0.0f64);
    let mut huge = 0;
    for (x, s, c) in sin_cos64_reference() {
        let ds = ulp_diff64(sin64(x), s);
        let dc = ulp_diff64(cos64(x), c);
        if ds > worst_s.0 {
            worst_s = (ds, x);
        }
        if dc > worst_c.0 {
            worst_c = (dc, x);
        }
        if x.abs() > 1.0e9 {
            huge += 1;
        }
    }
    // the large-argument (Payne–Hanek) path must actually be exercised
    assert!(huge > 500, "only {huge} rows above 1e9");
    println!(
        "sin64 max {} ulp (at {:e}), cos64 max {} ulp (at {:e})",
        worst_s.0, worst_s.1, worst_c.0, worst_c.1
    );
    assert!(worst_s.0 <= 1, "sin64 {} ulp at {:e}", worst_s.0, worst_s.1);
    assert!(worst_c.0 <= 1, "cos64 {} ulp at {:e}", worst_c.0, worst_c.1);
}

#[test]
fn sin_cos64_special_values() {
    let canonical = f64::NAN.to_bits();
    for x in [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling payload
        f64::from_bits(0xfff8_dead_beef_0001), // negative quiet payload
    ] {
        assert_eq!(sin64(x).to_bits(), canonical, "sin64({:#x})", x.to_bits());
        assert_eq!(cos64(x).to_bits(), canonical, "cos64({:#x})", x.to_bits());
        let (s, c) = sin_cos64(x);
        assert_eq!((s.to_bits(), c.to_bits()), (canonical, canonical));
    }
    assert_eq!(sin64(0.0).to_bits(), 0.0f64.to_bits());
    assert_eq!(sin64(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(cos64(0.0).to_bits(), 1.0f64.to_bits());
    assert_eq!(cos64(-0.0).to_bits(), 1.0f64.to_bits());
    // subnormals: sin x = x exactly, cos x = 1
    for b in [1u64, 2, 0x000f_ffff_ffff_ffff, 0x0008_0000_0000_0000] {
        for x in [f64::from_bits(b), -f64::from_bits(b)] {
            assert_eq!(sin64(x).to_bits(), x.to_bits());
            assert_eq!(cos64(x).to_bits(), 1.0f64.to_bits());
        }
    }
}

/// Inputs for the bitwise identities: the reference table, a dense sweep, and
/// a bit-pattern walk over the whole finite range.
fn identity_inputs() -> Vec<f64> {
    let mut v: Vec<f64> = sin_cos64_reference().into_iter().map(|r| r.0).collect();
    for i in 0..=100_000u32 {
        v.push(-50.0 + 100.0 * (f64::from(i) / 100_000.0));
    }
    for i in 0..=50_000u64 {
        v.push(f64::from_bits(1 + (0x7fef_ffff_ffff_fffe / 50_000) * i));
    }
    v
}

#[test]
fn sin64_is_odd_and_cos64_is_even_bitwise() {
    for x in identity_inputs() {
        let x = x.abs();
        assert_eq!(sin64(-x).to_bits(), (-sin64(x)).to_bits(), "sin64(-{x:e})");
        assert_eq!(cos64(-x).to_bits(), cos64(x).to_bits(), "cos64(-{x:e})");
    }
}

#[test]
fn sin_cos64_is_bit_identical_to_separate_calls() {
    let mut xs = identity_inputs();
    xs.extend([0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN]);
    for x in xs {
        let (s, c) = sin_cos64(x);
        assert_eq!(s.to_bits(), sin64(x).to_bits(), "sin({x:e})");
        assert_eq!(c.to_bits(), cos64(x).to_bits(), "cos({x:e})");
    }
}
