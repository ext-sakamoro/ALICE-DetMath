//! `tests/support/digest_bits.rs` and `alice_det_math::law_bits`: the bits a Law digest hashes are the float's own bits, and a NaN
//! other than the canonical one is refused, never mapped to it.
//!
//! oracle: the IEEE 754 bit patterns below (the canonical quiet NaNs, the x86-64 default NaN
//! with the sign bit set, a payload NaN, signed zeros and infinities).

#[path = "support/digest_bits.rs"]
mod digest_bits;

use alice_det_math::law_bits::{
    CANONICAL_NAN_F32_BITS as CANONICAL_NAN32, CANONICAL_NAN_F64_BITS as CANONICAL_NAN64,
};
use digest_bits::{bits32, bits64, out32, out64, pattern32, pattern64};

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

// the public API (`alice_det_math::law_bits`) that the helpers above wrap

use alice_det_math::law_bits::{f32_bits, f64_bits, NonCanonicalNan};

#[test]
fn the_api_returns_the_bits_of_every_number_and_of_the_canonical_nan() {
    for b in [
        0u32,
        0x8000_0000,
        1,
        0x3f80_0000,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        CANONICAL_NAN32,
    ] {
        assert_eq!(f32_bits(f32::from_bits(b)), Ok(b), "{b:#010x}");
    }
    for b in [
        0u64,
        0x8000_0000_0000_0000,
        1,
        0x3ff0_0000_0000_0000,
        0x7fef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        CANONICAL_NAN64,
    ] {
        assert_eq!(f64_bits(f64::from_bits(b)), Ok(b), "{b:#018x}");
    }
}

/// Every f32 NaN bit pattern: exponent all ones, mantissa not zero, either sign (2 × (2^23 − 1)
/// patterns). Exactly one of them, the canonical NaN, is accepted.
#[test]
fn the_api_refuses_every_f32_nan_but_the_canonical_one() {
    let mut refused = 0u32;
    let mut accepted = 0u32;
    for sign in [0u32, 0x8000_0000] {
        for mantissa in 1..=0x007f_ffffu32 {
            let b = sign | 0x7f80_0000 | mantissa;
            match f32_bits(f32::from_bits(b)) {
                Ok(got) => {
                    assert_eq!((got, b), (CANONICAL_NAN32, CANONICAL_NAN32));
                    accepted += 1;
                }
                Err(e) => {
                    assert_eq!(e, NonCanonicalNan::F32(b));
                    refused += 1;
                }
            }
        }
    }
    assert_eq!((accepted, refused), (1, 2 * 0x007f_ffff - 1));
}

/// f64 NaNs: both signs, the quiet bit on and off, a walk over the payload bits (each single
/// bit, and every bit). The canonical NaN is the only one accepted.
#[test]
fn the_api_refuses_f64_nans_but_the_canonical_one() {
    let mut payloads: Vec<u64> = (0..51).map(|i| 1u64 << i).collect();
    payloads.extend([0x0007_ffff_ffff_ffff, 0x0000_0000_dead_beef]);
    let mut n = 0;
    for sign in [0u64, 0x8000_0000_0000_0000] {
        for quiet in [0u64, 0x0008_0000_0000_0000] {
            for &p in &payloads {
                let b = sign | 0x7ff0_0000_0000_0000 | quiet | p;
                assert_eq!(
                    f64_bits(f64::from_bits(b)),
                    Err(NonCanonicalNan::F64(b)),
                    "{b:#018x}"
                );
                n += 1;
            }
        }
    }
    // the quiet NaN with no payload: canonical with sign 0, refused with the sign bit (x86-64)
    assert_eq!(
        f64_bits(f64::from_bits(0x7ff8_0000_0000_0000)),
        Ok(CANONICAL_NAN64)
    );
    assert_eq!(
        f64_bits(f64::from_bits(0xfff8_0000_0000_0000)),
        Err(NonCanonicalNan::F64(0xfff8_0000_0000_0000))
    );
    assert_eq!(n, 2 * 2 * 53);
}

#[test]
fn a_refusal_names_the_bits_and_the_canonical_nan() {
    assert_eq!(
        NonCanonicalNan::F32(0xffc0_0000).to_string(),
        "non-canonical NaN 0xffc00000 (the canonical f32 NaN is 0x7fc00000)"
    );
    assert_eq!(
        NonCanonicalNan::F64(0xfff8_0000_0000_0000).to_string(),
        "non-canonical NaN 0xfff8000000000000 (the canonical f64 NaN is 0x7ff8000000000000)"
    );
    let e: &dyn std::error::Error = &NonCanonicalNan::F64(1);
    assert!(e.source().is_none());
}
