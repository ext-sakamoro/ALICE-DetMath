//! Accuracy of [`erf`] / [`erf64`] against correctly rounded values from an
//! independent 2400-bit evaluation (`mpmath`, `scripts/gen_erf_reference.py`),
//! not from any libm. Each test counts the inputs that reach each branch of the
//! fdlibm algorithm and fails if one is unmeasured, and fails if the table is
//! shorter than expected (a comparison over no rows passes nothing).

use alice_det_math::{erf, erf64};

fn ulp_diff32(a: f32, b: f32) -> u32 {
    if a == b {
        return 0;
    }
    (a.to_bits() as i32 - b.to_bits() as i32).unsigned_abs()
}

fn ulp_diff64(a: f64, b: f64) -> u64 {
    if a == b {
        return 0;
    }
    (a.to_bits() as i64 - b.to_bits() as i64).unsigned_abs()
}

fn hex_row<const N: usize>(line: &str) -> [f64; N] {
    let mut it = line
        .split_whitespace()
        .map(|h| f64::from_bits(u64::from_str_radix(h, 16).expect("hex")));
    let mut out = [0.0; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = it.next().unwrap_or_else(|| panic!("column {i} missing"));
    }
    assert!(it.next().is_none(), "unexpected extra column");
    out
}

fn data_rows(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
}

const ERF64_REFERENCE: &str = include_str!("data/erf64_reference.txt");
const ERF32_REFERENCE: &str = include_str!("data/erf32_reference.txt");

/// The branch of the fdlibm `s_erf.c` algorithm that `|x|` falls in, so that
/// every one of them is shown to be measured.
fn erf_branch(ax: f64) -> usize {
    if ax < 3.725_290_298_461_914e-9 {
        0 // 2^-28 shortcut
    } else if ax < 0.84375 {
        1
    } else if ax < 1.25 {
        2
    } else if ax < 2.857_141_494_750_976_6 {
        3 // fdlibm's 1/0.35 switch, high word 0x4006_db6d
    } else if ax < 6.0 {
        4
    } else {
        5 // ±1 for binary64
    }
}

/// oracle: correctly rounded erf (`mpmath` at 2400 bits,
/// `scripts/gen_erf_reference.py`), every branch of the reduction reached.
#[test]
fn erf64_within_1_ulp_of_correctly_rounded_reference() {
    let rows: Vec<(f64, f64)> = data_rows(ERF64_REFERENCE)
        .map(|l| hex_row::<2>(l).into())
        .collect();
    assert!(rows.len() > 6000, "erf64 reference has {} rows", rows.len());
    let mut per_branch = [0usize; 6];
    let mut worst = (0u64, 0.0f64);
    for &(x, e) in &rows {
        per_branch[erf_branch(x.abs())] += 1;
        let d = ulp_diff64(erf64(x), e);
        if d > worst.0 {
            worst = (d, x);
        }
    }
    println!(
        "erf64 max {} ulp over {} reference rows (at {:e}), per branch {per_branch:?}",
        worst.0,
        rows.len(),
        worst.1
    );
    assert!(
        per_branch.iter().all(|&n| n > 0),
        "a branch is unmeasured: {per_branch:?}"
    );
    assert!(worst.0 <= 1, "erf64 {} ulp at {:e}", worst.0, worst.1);
}

/// oracle: correctly rounded binary32 erf from the same independent
/// evaluation, every branch reached.
#[test]
fn erf_within_1_ulp_of_correctly_rounded_reference() {
    let rows: Vec<(f32, f32)> = data_rows(ERF32_REFERENCE)
        .map(|l| {
            let mut it = l
                .split_whitespace()
                .map(|h| f32::from_bits(u32::from_str_radix(h, 16).expect("hex")));
            let x = it.next().expect("x");
            let e = it.next().expect("erf");
            assert!(it.next().is_none(), "unexpected extra column");
            (x, e)
        })
        .collect();
    assert!(rows.len() > 7000, "erf32 reference has {} rows", rows.len());
    let mut per_branch = [0usize; 6];
    let mut worst = (0u32, 0.0f32);
    for &(x, e) in &rows {
        per_branch[erf_branch(f64::from(x).abs())] += 1;
        let d = ulp_diff32(erf(x), e);
        if d > worst.0 {
            worst = (d, x);
        }
    }
    println!(
        "erf max {} ulp over {} reference rows (at {:e}), per branch {per_branch:?}",
        worst.0,
        rows.len(),
        worst.1
    );
    assert!(
        per_branch.iter().all(|&n| n > 0),
        "a branch is unmeasured: {per_branch:?}"
    );
    assert!(worst.0 <= 1, "erf {} ulp at {:e}", worst.0, worst.1);
}
