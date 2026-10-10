//! Reading arithmetic identifiers back from stored data.
//!
//! [`SEMANTICS_ID`] names the arithmetic of this
//! release. Data written by an earlier release carries that release's
//! identifier; [`check_semantics`] says whether this release reproduces it.
//!
//! Nothing here computes a floating-point value: the functions compare
//! 32-byte identifiers, so this module is outside the numeric surface the
//! golden pins cover (and `tests/golden.rs` checks that it stays so).

use crate::SEMANTICS_ID;

/// Identifiers of earlier releases whose functions return **the same bits** as
/// this release on every input, but which covered fewer functions.
///
/// | entry | release | why it differs from [`SEMANTICS_ID`] |
/// |---|---|---|
/// | `d2209b30…398e` | 0.4.0 | the 11 `metric` entry points had no pin (same bits, smaller coverage) |
///
/// An entry is added only together with that release's pin table in
/// `tests/golden.rs`, and the test proves two things about it: the table
/// folds to the entry, and every pin in it is reproduced by this release.
/// So data stamped with an entry here was computed with arithmetic this
/// release still performs, bit for bit. An identifier whose release changed
/// any function's output never appears here.
///
/// Writers always stamp [`SEMANTICS_ID`]. Readers use [`check_semantics`].
pub const PREVIOUS_SEMANTICS_IDS: &[[u8; 32]] = &[[
    0xd2, 0x20, 0x9b, 0x30, 0xf6, 0xf1, 0xf4, 0x5b, 0xaa, 0x1b, 0x63, 0x8b, 0xcd, 0xfe, 0xe3, 0x4a,
    0xc6, 0x47, 0x73, 0xb2, 0xe6, 0x3b, 0x9c, 0x08, 0x3b, 0x2e, 0x77, 0xaf, 0xc6, 0x91, 0x39, 0x8e,
]];

/// What a stored arithmetic identifier says about the data stamped with it.
///
/// Returned by [`check_semantics`]. The previous case is its own variant so
/// a reader never treats "computed by an earlier release with the same bits"
/// as silently equal to "computed by this release".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[must_use]
pub enum SemanticsCheck {
    /// The identifier is [`SEMANTICS_ID`].
    Verified,
    /// The identifier is one of [`PREVIOUS_SEMANTICS_IDS`]: an earlier
    /// release, same bits on every function it covered.
    VerifiedPrevious([u8; 32]),
    /// Unknown: the data was computed with arithmetic this release does not
    /// promise to reproduce.
    Mismatch,
}

impl SemanticsCheck {
    /// Accepting: [`Verified`](Self::Verified) or
    /// [`VerifiedPrevious`](Self::VerifiedPrevious).
    ///
    /// For readers of stored data (residuals, law identifiers, snapshots):
    /// the stored values are reproduced exactly by this release.
    #[must_use]
    pub const fn is_reproducible(self) -> bool {
        !matches!(self, Self::Mismatch)
    }

    /// Strict: [`Verified`](Self::Verified) only.
    ///
    /// For comparing identifiers as keys (a cache keyed on the identifier, a
    /// peer that must run exactly this release, a writer checking what it is
    /// about to stamp), where an earlier identifier is a different key even
    /// though the bits agree.
    #[must_use]
    pub const fn is_current(self) -> bool {
        matches!(self, Self::Verified)
    }
}

/// Classify an identifier read back from stored data.
///
/// ```
/// use alice_det_math::{check_semantics, SemanticsCheck, SEMANTICS_ID};
/// assert_eq!(check_semantics(&SEMANTICS_ID), SemanticsCheck::Verified);
/// assert_eq!(check_semantics(&[0; 32]), SemanticsCheck::Mismatch);
/// ```
pub const fn check_semantics(id: &[u8; 32]) -> SemanticsCheck {
    if bytes_eq(id, &SEMANTICS_ID) {
        return SemanticsCheck::Verified;
    }
    let mut i = 0;
    while i < PREVIOUS_SEMANTICS_IDS.len() {
        if bytes_eq(id, &PREVIOUS_SEMANTICS_IDS[i]) {
            return SemanticsCheck::VerifiedPrevious(PREVIOUS_SEMANTICS_IDS[i]);
        }
        i += 1;
    }
    SemanticsCheck::Mismatch
}

const fn bytes_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut i = 0;
    while i < 32 {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
