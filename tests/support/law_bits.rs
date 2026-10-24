//! The bits a Law digest hashes for a float.
//!
//! A digest over the output bits of a function is the same on every target only when every
//! NaN it hashes is the canonical one, `0x7FC0_0000` / `0x7FF8_0000_0000_0000` (the positive
//! quiet NaN with an empty payload). A NaN raised by the arithmetic itself carries the
//! platform's default NaN instead (sign bit set on x86-64, clear on `AArch64`), so a function
//! that lets one through moves the digest on one target only. [`bits32`] / [`bits64`] (and
//! [`out32`] / [`out64`], their little-endian bytes) refuse any other NaN: the leak turns
//! the digest test red on every target, at the call that produced it. Nothing is mapped to
//! the canonical NaN here, which would hide the leak.
//!
//! An input built from integer bit patterns may hold any NaN on purpose (a payload, a sign
//! bit): [`pattern32`] / [`pattern64`] give its bits as they are. They are for inputs only,
//! never for a value a function returned.
//!
//! A test file that computes a digest reads float bits only through this module
//! (`scripts/law_bits_check.py`). The file is the same in every repository that pins one.

#![allow(dead_code)]

/// The canonical `f32` NaN.
pub const CANONICAL_NAN32: u32 = 0x7fc0_0000;
/// The canonical `f64` NaN.
pub const CANONICAL_NAN64: u64 = 0x7ff8_0000_0000_0000;

/// The bits of an output, refusing a NaN other than the canonical one.
#[must_use]
#[track_caller]
pub fn bits32(x: f32) -> u32 {
    let b = x.to_bits();
    assert!(
        !x.is_nan() || b == CANONICAL_NAN32,
        "a non-canonical NaN {b:#010x} reached a Law digest (the canonical one is {CANONICAL_NAN32:#010x})"
    );
    b
}

/// The bits of an output, refusing a NaN other than the canonical one.
#[must_use]
#[track_caller]
pub fn bits64(x: f64) -> u64 {
    let b = x.to_bits();
    assert!(
        !x.is_nan() || b == CANONICAL_NAN64,
        "a non-canonical NaN {b:#018x} reached a Law digest (the canonical one is {CANONICAL_NAN64:#018x})"
    );
    b
}

/// [`bits32`] as little-endian bytes.
#[must_use]
#[track_caller]
pub fn out32(x: f32) -> [u8; 4] {
    bits32(x).to_le_bytes()
}

/// [`bits64`] as little-endian bytes.
#[must_use]
#[track_caller]
pub fn out64(x: f64) -> [u8; 8] {
    bits64(x).to_le_bytes()
}

/// The bits of an input built from a bit pattern, as they are (any NaN).
#[must_use]
pub fn pattern32(x: f32) -> u32 {
    x.to_bits()
}

/// The bits of an input built from a bit pattern, as they are (any NaN).
#[must_use]
pub fn pattern64(x: f64) -> u64 {
    x.to_bits()
}
