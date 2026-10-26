//! `powf` of a negative base, and the signed zero / infinity cases of IEEE 754
//! `pow` (C99 Annex F.10.4.4).
//!
//! oracle: (1) the Annex F special cases, written from the standard, compared
//! bit for bit; (2) correctly rounded `x^n` for a negative `x` and an integer
//! `n` from an independent 400-bit evaluation (`mpmath`,
//! `scripts/gen_powf_negbase_reference.py`), not from any libm.

use alice_det_math::powf;

const NEG_BASE_REFERENCE: &str = include_str!("data/powf_negbase_reference.txt");

fn ulp_diff(a: f32, b: f32) -> u32 {
    if a.to_bits() == b.to_bits() {
        return 0;
    }
    if a.is_sign_negative() != b.is_sign_negative() {
        // a sign error is never "close"
        return u32::MAX;
    }
    (a.to_bits() as i32 - b.to_bits() as i32).unsigned_abs()
}

#[test]
fn annex_f_special_cases_bit_for_bit() {
    let inf = f32::INFINITY;
    let cases: &[(f32, f32, f32)] = &[
        // a negative base with an integer exponent: sign from the parity
        (-2.0, 3.0, -8.0),
        (-2.0, 2.0, 4.0),
        (-2.0, -1.0, -0.5),
        (-2.0, -2.0, 0.25),
        (-0.5, -3.0, -8.0),
        (-1.0, 7.0, -1.0),
        (-1.0, -8.0, 1.0),
        // |y| ≥ 2^24 is an even integer
        (-1.0, 16_777_216.0, 1.0),
        (-1.0, 3.0e38, 1.0),
        // a finite non-integer exponent: NaN
        (-2.0, 0.5, f32::NAN),
        (-2.0, 2.5, f32::NAN),
        (-inf, 0.5, inf),
        // pow(-1, ±inf) = 1
        (-1.0, inf, 1.0),
        (-1.0, -inf, 1.0),
        // a negative base and an infinite exponent: by |x|
        (-0.5, inf, 0.0),
        (-0.5, -inf, inf),
        (-2.0, inf, inf),
        (-2.0, -inf, 0.0),
        // ±0
        (0.0, 3.0, 0.0),
        (-0.0, 3.0, -0.0),
        (-0.0, 2.0, 0.0),
        (-0.0, 0.5, 0.0),
        (0.0, -3.0, inf),
        (-0.0, -3.0, -inf),
        (-0.0, -2.0, inf),
        (-0.0, -0.5, inf),
        (-0.0, inf, 0.0),
        (-0.0, -inf, inf),
        // -inf
        (-inf, 3.0, -inf),
        (-inf, 2.0, inf),
        (-inf, -3.0, -0.0),
        (-inf, -2.0, 0.0),
        (-inf, -0.5, 0.0),
        // +inf
        (inf, 2.0, inf),
        (inf, -2.0, 0.0),
        // the exponent zero and the base one win over everything
        (-2.0, 0.0, 1.0),
        (-0.0, -0.0, 1.0),
        (-inf, 0.0, 1.0),
        (f32::NAN, 0.0, 1.0),
        (1.0, f32::NAN, 1.0),
        // a NaN otherwise: the canonical NaN
        (-2.0, f32::NAN, f32::NAN),
        (f32::NAN, 3.0, f32::NAN),
    ];
    for &(x, y, want) in cases {
        let got = powf(x, y);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "powf({x:e}, {y:e}) = {got:e} ({:#010x}), IEEE {want:e} ({:#010x})",
            got.to_bits(),
            want.to_bits()
        );
    }
    assert!(cases.len() > 40);
}

#[test]
fn negative_base_integer_exponent_within_1_ulp_of_correctly_rounded() {
    let mut compared = 0usize;
    let mut exact = 0usize;
    let mut worst = (0u32, 0.0f32, 0.0f32);
    for line in NEG_BASE_REFERENCE.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let v: Vec<f32> = line
            .split_whitespace()
            .map(|h| f32::from_bits(u32::from_str_radix(h, 16).expect("hex")))
            .collect();
        assert_eq!(v.len(), 3, "row {line}");
        let (x, y, want) = (v[0], v[1], v[2]);
        let got = powf(x, y);
        let d = ulp_diff(got, want);
        if d == 0 {
            exact += 1;
        }
        if d > worst.0 {
            worst = (d, x, y);
        }
        compared += 1;
    }
    println!(
        "powf negative base: max {} ulp over {compared} rows ({exact} exact), at ({:e}, {:e})",
        worst.0, worst.1, worst.2
    );
    assert!(compared > 6000, "compared only {compared} rows");
    assert!(
        worst.0 <= 1,
        "powf({:e}, {:e}) is {} ulp off",
        worst.1,
        worst.2,
        worst.0
    );
}

/// For an integer exponent the magnitude is exactly the positive base's.
#[test]
fn negative_base_magnitude_is_the_positive_bases() {
    let mut compared = 0usize;
    for i in 1..2000u32 {
        let x = f32::from_bits(0x3c00_0000 + i * 0x0001_7f3b);
        for n in -25..=25i32 {
            let y = n as f32;
            let (p, q) = (powf(x, y), powf(-x, y));
            assert_eq!(q.abs().to_bits(), p.to_bits(), "|powf(-{x:e}, {n})|");
            assert_eq!(
                q.is_sign_negative(),
                n & 1 != 0,
                "sign of powf(-{x:e}, {n})"
            );
            compared += 1;
        }
    }
    assert!(compared > 90_000);
}
