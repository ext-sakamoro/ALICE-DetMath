//! The bits of a float that a Law identifier or digest is built from.
//!
//! A value's bits are the same on every target except for a NaN: one raised by the arithmetic
//! itself (`0/0`, `inf − inf`, `0·inf`) carries the platform's default NaN, whose sign bit is
//! set on x86-64 and clear on `AArch64`, and a NaN argument may carry any payload. A digest
//! over such bits moves on one target only. [`f32_bits`] / [`f64_bits`] return the bits of
//! any number and of the canonical NaN, the positive quiet NaN with an empty payload
//! ([`CANONICAL_NAN_F32_BITS`] / [`CANONICAL_NAN_F64_BITS`], the NaN every function of this
//! crate returns), and refuse every other NaN with [`NonCanonicalNan`]. Nothing is mapped to
//! the canonical NaN: a value that leaked a platform NaN is reported, not hidden.
//!
//! The test is on the bits alone (integer comparisons), so it is the same on every target.

use core::fmt;

/// The canonical `f32` NaN: positive, quiet, empty payload (`f32::NAN` on every target).
pub const CANONICAL_NAN_F32_BITS: u32 = 0x7fc0_0000;
/// The canonical `f64` NaN: positive, quiet, empty payload (`f64::NAN` on every target).
pub const CANONICAL_NAN_F64_BITS: u64 = 0x7ff8_0000_0000_0000;

const F32_EXP_MASK: u32 = 0x7f80_0000;
const F32_ABS_MASK: u32 = 0x7fff_ffff;
const F64_EXP_MASK: u64 = 0x7ff0_0000_0000_0000;
const F64_ABS_MASK: u64 = 0x7fff_ffff_ffff_ffff;

/// A NaN other than the canonical one, with its bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NonCanonicalNan {
    /// An `f32` NaN's bits.
    F32(u32),
    /// An `f64` NaN's bits.
    F64(u64),
}

impl fmt::Display for NonCanonicalNan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::F32(b) => write!(
                f,
                "non-canonical NaN {b:#010x} (the canonical f32 NaN is {CANONICAL_NAN_F32_BITS:#010x})"
            ),
            Self::F64(b) => write!(
                f,
                "non-canonical NaN {b:#018x} (the canonical f64 NaN is {CANONICAL_NAN_F64_BITS:#018x})"
            ),
        }
    }
}

impl core::error::Error for NonCanonicalNan {}

/// The bits of `v`, or [`NonCanonicalNan`] when `v` is a NaN other than the canonical one.
///
/// # Errors
///
/// A NaN whose bits are not [`CANONICAL_NAN_F32_BITS`].
#[inline]
pub fn f32_bits(v: f32) -> Result<u32, NonCanonicalNan> {
    let b = v.to_bits();
    if b & F32_ABS_MASK > F32_EXP_MASK && b != CANONICAL_NAN_F32_BITS {
        Err(NonCanonicalNan::F32(b))
    } else {
        Ok(b)
    }
}

/// The bits of `v`, or [`NonCanonicalNan`] when `v` is a NaN other than the canonical one.
///
/// # Errors
///
/// A NaN whose bits are not [`CANONICAL_NAN_F64_BITS`].
#[inline]
pub fn f64_bits(v: f64) -> Result<u64, NonCanonicalNan> {
    let b = v.to_bits();
    if b & F64_ABS_MASK > F64_EXP_MASK && b != CANONICAL_NAN_F64_BITS {
        Err(NonCanonicalNan::F64(b))
    } else {
        Ok(b)
    }
}
