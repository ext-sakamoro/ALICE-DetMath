//! The bits a Law digest test hashes for a float, read through `alice_det_math::law_bits`.
//!
//! [`bits32`] / [`bits64`] (and [`out32`] / [`out64`], their little-endian bytes) are
//! `law_bits::f32_bits` / `f64_bits` with the refusal turned into a panic at the caller: a
//! function that lets the platform's NaN through (sign bit set on x86-64, clear on `AArch64`)
//! fails the digest test on every target, instead of moving the digest on one. Nothing is
//! mapped to the canonical NaN here, which would hide the leak.
//!
//! An input built from integer bit patterns may hold any NaN on purpose (a payload, a sign
//! bit): [`pattern32`] / [`pattern64`] give its bits as they are. They are for inputs only,
//! never for a value a function returned.
//!
//! A test file that computes a digest reads float bits only through this module
//! (`scripts/law_bits_check.py`). The file is the same in every repository that pins one.

#![allow(dead_code)]

use alice_det_math::law_bits;

/// The bits of an output, panicking on a NaN other than the canonical one.
#[must_use]
#[track_caller]
pub fn bits32(x: f32) -> u32 {
    law_bits::f32_bits(x).unwrap_or_else(|e| panic!("{e} reached a Law digest"))
}

/// The bits of an output, panicking on a NaN other than the canonical one.
#[must_use]
#[track_caller]
pub fn bits64(x: f64) -> u64 {
    law_bits::f64_bits(x).unwrap_or_else(|e| panic!("{e} reached a Law digest"))
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
