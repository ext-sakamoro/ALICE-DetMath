//! `tests/support/law_bits.rs`: the bits a Law digest hashes are the float's own bits, and a NaN
//! other than the canonical one is refused, never mapped to it.
//!
//! oracle: the IEEE 754 bit patterns below (the canonical quiet NaNs, the x86-64 default NaN
//! with the sign bit set, a payload NaN, signed zeros and infinities).

#[path = "support/law_bits.rs"]
mod law_bits;

use law_bits::{
    bits32, bits64, out32, out64, pattern32, pattern64, CANONICAL_NAN32, CANONICAL_NAN64,
};

#[test]
fn the_canonical_nans_are_the_positive_quiet_nans_with_an_empty_payload() {
    assert_eq!(CANONICAL_NAN32, 0x7fc0_0000);
    assert_eq!(CANONICAL_NAN64, 0x7ff8_0000_0000_0000);
    assert_eq!(bits32(f32::from_bits(CANONICAL_NAN32)), CANONICAL_NAN32);
    assert_eq!(bits64(f64::from_bits(CANONICAL_NAN64)), CANONICAL_NAN64);
    assert_eq!(
        out32(f32::from_bits(CANONICAL_NAN32)),
        CANONICAL_NAN32.to_le_bytes()
    );
    assert_eq!(
        out64(f64::from_bits(CANONICAL_NAN64)),
        CANONICAL_NAN64.to_le_bytes()
    );
}

#[test]
fn a_number_hashes_its_own_bits() {
    let xs32 = [
        0.0f32,
        -0.0,
        1.5,
        -2.25,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for x in xs32 {
        assert_eq!(bits32(x), x.to_bits(), "{x:e}");
        assert_eq!(out32(x), x.to_bits().to_le_bytes(), "{x:e}");
    }
    let xs64 = [
        0.0f64,
        -0.0,
        1.5,
        -2.25,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for x in xs64 {
        assert_eq!(bits64(x), x.to_bits(), "{x:e}");
        assert_eq!(out64(x), x.to_bits().to_le_bytes(), "{x:e}");
    }
    // the bytes are little-endian on every target
    assert_eq!(out32(1.0), [0x00, 0x00, 0x80, 0x3f]);
    assert_eq!(out64(1.0), [0, 0, 0, 0, 0, 0, 0xf0, 0x3f]);
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn the_x86_default_nan_is_refused_f32() {
    let _ = bits32(f32::from_bits(0xffc0_0000));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn the_x86_default_nan_is_refused_f64() {
    let _ = bits64(f64::from_bits(0xfff8_0000_0000_0000));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0x7fc00001")]
fn a_payload_nan_is_refused_f32() {
    let _ = out32(f32::from_bits(0x7fc0_0001));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0x7ff0000000000001")]
fn a_signalling_nan_is_refused_f64() {
    let _ = out64(f64::from_bits(0x7ff0_0000_0000_0001));
}

#[test]
fn an_input_pattern_keeps_any_nan() {
    for b in [0xffc0_0000u32, 0x7fc0_0001, 0x7f80_0001, CANONICAL_NAN32] {
        assert_eq!(pattern32(f32::from_bits(b)), b, "{b:#010x}");
    }
    for b in [
        0xfff8_0000_0000_0000u64,
        0x7ff0_0000_0000_0001,
        CANONICAL_NAN64,
    ] {
        assert_eq!(pattern64(f64::from_bits(b)), b, "{b:#018x}");
    }
}
