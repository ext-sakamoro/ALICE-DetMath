//! The clamp and the selection Law (ALICE-Law G2), at the bit level.
//!
//! oracle: written from the rule, not from the implementation. `clamp(x, lo, hi)` is
//! `min(max(x, lo), hi)` with this crate's G2 `max` / `min` (IEEE maximumNumber /
//! minimumNumber with `-0 < +0`), except that a NaN `x` gives the canonical NaN:
//!
//! - an equal pair of zeros follows the G2 order: `clamp(-0, +0, hi)` is `+0`
//! - a NaN bound is no bound (maximumNumber / minimumNumber return the other operand):
//!   `clamp(1, NaN, 2) = 1`
//! - inverted bounds: the high bound wins (`min` is applied last): `clamp(5, 3, 1) = 1`
//!
//! `argmax` / `argmin` select the first element equal to the G2 maximum / minimum of the
//! non-NaN elements: a NaN is never selected, `+0` is above `-0`, and among equal values
//! the first one is kept. [`Select`] is the same rule for a stream of
//! values that carry a payload.

use alice_det_math::metric::clamp;
use alice_det_math::{argmax, argmin, clamp64, Select};

const NAN32: u32 = 0x7fc0_0000;
const NAN64: u64 = 0x7ff8_0000_0000_0000;
const NEG_ZERO: u32 = 0x8000_0000;
const POS_ZERO: u32 = 0x0000_0000;

/// (x, lo, hi, expected bits) for f32, from the rule above
fn table32() -> Vec<(f32, f32, f32, u32)> {
    let nan = f32::NAN;
    let payload_nan = f32::from_bits(0xffc0_1234);
    vec![
        (0.5, 0.0, 1.0, 0.5f32.to_bits()),
        (-1.0, 0.0, 1.0, POS_ZERO),
        (2.0, 0.0, 1.0, 1.0f32.to_bits()),
        // an equal pair of zeros: the G2 order
        (-0.0, 0.0, 1.0, POS_ZERO),
        (0.0, -0.0, 1.0, POS_ZERO),
        (0.0, -1.0, -0.0, NEG_ZERO),
        (-0.0, -1.0, 0.0, NEG_ZERO),
        // a NaN bound is no bound
        (1.0, nan, 2.0, 1.0f32.to_bits()),
        (3.0, 0.0, nan, 3.0f32.to_bits()),
        (-5.0, nan, nan, (-5.0f32).to_bits()),
        // inverted bounds: the high bound wins
        (5.0, 3.0, 1.0, 1.0f32.to_bits()),
        (0.0, 3.0, 1.0, 1.0f32.to_bits()),
        (2.0, 3.0, 1.0, 1.0f32.to_bits()),
        // a NaN x: the canonical NaN, whatever its payload
        (nan, 0.0, 1.0, NAN32),
        (payload_nan, 0.0, 1.0, NAN32),
        (nan, nan, nan, NAN32),
        // infinities
        (f32::INFINITY, 0.0, 1.0, 1.0f32.to_bits()),
        (f32::NEG_INFINITY, 0.0, 1.0, POS_ZERO),
        (7.0, f32::NEG_INFINITY, f32::INFINITY, 7.0f32.to_bits()),
    ]
}

#[test]
fn clamp_follows_the_g2_rule_bit_for_bit() {
    for (x, lo, hi, want) in table32() {
        let got = clamp(x, lo, hi).to_bits();
        assert_eq!(
            got, want,
            "clamp({x:?}, {lo:?}, {hi:?}) = {got:#010x}, want {want:#010x}"
        );
    }
}

#[test]
fn clamp64_follows_the_same_rule() {
    for (x, lo, hi, want) in table32() {
        let want64 = if want == NAN32 {
            NAN64
        } else {
            f64::from(f32::from_bits(want)).to_bits()
        };
        let got = clamp64(f64::from(x), f64::from(lo), f64::from(hi)).to_bits();
        assert_eq!(
            got, want64,
            "clamp64({x:?}, {lo:?}, {hi:?}) = {got:#018x}, want {want64:#018x}"
        );
    }
    // the payload of a NaN x is not kept
    assert_eq!(
        clamp64(f64::from_bits(0xfff8_0000_dead_beef), 0.0, 1.0).to_bits(),
        NAN64
    );
}

#[cfg(feature = "simd")]
#[test]
fn simd_clamp_follows_the_same_rule_in_every_lane() {
    use alice_det_math::simd;
    use wide::f32x8;
    let rows = table32();
    for chunk in rows.chunks(8) {
        let mut x = [0.0f32; 8];
        let mut lo = [0.0f32; 8];
        let mut hi = [0.0f32; 8];
        let mut want = [0u32; 8];
        for (i, &(a, b, c, w)) in chunk.iter().enumerate() {
            (x[i], lo[i], hi[i], want[i]) = (a, b, c, w);
        }
        // unused lanes: clamp(0, 0, 0) = +0
        let got = simd::clamp(f32x8::new(x), f32x8::new(lo), f32x8::new(hi)).to_array();
        for i in 0..chunk.len() {
            assert_eq!(
                got[i].to_bits(),
                want[i],
                "lane {i}: clamp({:?}, {:?}, {:?})",
                x[i],
                lo[i],
                hi[i]
            );
        }
    }
}

#[test]
fn argmax_selects_the_first_g2_maximum_and_never_a_nan() {
    let nan = f32::NAN;
    assert_eq!(argmax(&[]), None);
    assert_eq!(argmax(&[nan]), None);
    assert_eq!(argmax(&[nan, nan]), None);
    assert_eq!(argmax(&[1.0, nan, 3.0, 3.0]), Some(2));
    assert_eq!(argmax(&[nan, f32::NEG_INFINITY]), Some(1));
    assert_eq!(argmax(&[-0.0, 0.0]), Some(1)); // +0 is above -0
    assert_eq!(argmax(&[0.0, -0.0]), Some(0));
    assert_eq!(argmax(&[-0.0, -0.0]), Some(0)); // a tie keeps the first
    assert_eq!(argmax(&[f32::INFINITY, nan, f32::INFINITY]), Some(0));
}

#[test]
fn argmin_selects_the_first_g2_minimum_and_never_a_nan() {
    let nan = f32::NAN;
    assert_eq!(argmin(&[]), None);
    assert_eq!(argmin(&[nan]), None);
    assert_eq!(argmin(&[2.0, 1.0, 1.0]), Some(1));
    assert_eq!(argmin(&[0.0, -0.0]), Some(1)); // -0 is below +0
    assert_eq!(argmin(&[-0.0, 0.0]), Some(0));
    assert_eq!(argmin(&[nan, 5.0, nan, 4.0]), Some(3));
}

#[test]
fn the_streaming_selection_keeps_the_payload_of_the_first_g2_extreme() {
    let mut best = Select::greatest();
    for (v, name) in [
        (f32::NAN, "nan"),
        (-0.0, "neg zero"),
        (0.0, "pos zero"),
        (0.0, "second pos zero"),
    ] {
        best.offer(v, name);
    }
    assert_eq!(
        best.into_inner().map(|(v, n)| (v.to_bits(), n)),
        Some((POS_ZERO, "pos zero"))
    );

    let mut near = Select::least();
    for (v, i) in [(3.0, 0usize), (f32::NAN, 1), (0.0, 2), (-0.0, 3), (-0.0, 4)] {
        near.offer(v, i);
    }
    assert_eq!(
        near.into_inner().map(|(v, i)| (v.to_bits(), i)),
        Some((NEG_ZERO, 3))
    );

    let empty: Select<u8> = Select::greatest();
    assert_eq!(empty.into_inner(), None);
    let mut only_nan = Select::least();
    only_nan.offer(f32::NAN, ());
    assert_eq!(only_nan.into_inner(), None);
}

#[test]
fn the_streaming_and_slice_forms_agree() {
    // every ordered pair and triple of the special values: the slice form and the stream
    let specials = [
        f32::NAN,
        f32::NEG_INFINITY,
        -1.0,
        -0.0,
        0.0,
        1.0,
        f32::INFINITY,
    ];
    let mut compared = 0;
    for &a in &specials {
        for &b in &specials {
            for &c in &specials {
                let xs = [a, b, c];
                let mut mx = Select::greatest();
                let mut mn = Select::least();
                for (i, &v) in xs.iter().enumerate() {
                    mx.offer(v, i);
                    mn.offer(v, i);
                }
                assert_eq!(mx.into_inner().map(|(_, i)| i), argmax(&xs), "{xs:?}");
                assert_eq!(mn.into_inner().map(|(_, i)| i), argmin(&xs), "{xs:?}");
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 343);
}
