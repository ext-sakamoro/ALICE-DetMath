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
/// 0.4.0's identifier is not here: that release changed the bits of `sin` /
/// `cos` / `sin_cos` (the reduction near their zeros and the sign of zero),
/// `round` / `round64` / `cbrt` / `smoothstep` and the other metric functions
/// (the canonical NaN), `MetricWeights::lipschitz` / `new` (exact for tiny and
/// huge weights) and both precisions of `powf` (`pow(1, y) = 1`), so neither
/// 0.4.0's identifier (`d2209b30…398e`) nor the one recorded on the main
/// branch before this change and never published (`a6fe7dc8…d6af`) is
/// reproduced, and both read as [`SemanticsCheck::Mismatch`].
/// `tests/golden.rs` keeps 0.4.0's pin table and proves that it differs from
/// this release in exactly the pins of those functions.
///
/// 0.5.0's identifier (`135c407d…ef000`) and the one of 0.5.1 / 0.5.2 (`85e08846…b315`) are
/// not here either: this release changes the bits of `clamp` (it now follows ALICE-Law G2,
/// `min(max(x, lo), hi)`), so data stamped with them was computed with a clamp this release no
/// longer performs. `tests/golden.rs` keeps both pin tables and proves that exactly `clamp` and
/// the aggregate `specials` pin differ. The list is empty in this release.
///
/// An entry is added only together with that release's pin table in
/// `tests/golden.rs`, and the test proves two things about it: the table
/// folds to the entry, and every pin in it is reproduced by this release.
/// So data stamped with an entry here was computed with arithmetic this
/// release still performs, bit for bit. An identifier whose release changed
/// any function's output never appears here.
///
/// # The `specials` pin is not a per-function pin
///
/// `tests/golden.rs`'s `specials` entry is a single hash folded over every
/// function the specials sweep (`tests/common/specials.rs`) visits — not one
/// function's output. That sweep grows whenever a release adds a public
/// numeric function (0.5.0 covered 48 functions, 0.5.1 covers 52), so a
/// stored `specials` hash can never be compared against the *live* `specials`
/// pin once the two releases cover different function sets: the aggregate
/// necessarily differs even when every function the old release covered
/// still returns the same bits. A release whose pin table includes
/// `specials` must instead freeze its own recomputation — the exact
/// historical call sequence, copied verbatim and never edited, run against
/// this release's current implementations — and `tests/golden.rs` compares
/// the *stored* `specials` hash against *that*, not against the live
/// aggregate. See `tests/golden.rs`'s `PREVIOUS_TABLES` /
/// `SpecialsRecompute` and the frozen `specials_v0_5_0` module for the
/// worked example. Every future release that adds a function and wants its
/// identifier accepted here repeats this: freeze one more historical sweep.
///
/// Writers always stamp [`SEMANTICS_ID`]. Readers use [`check_semantics`].
pub const PREVIOUS_SEMANTICS_IDS: &[[u8; 32]] = &[];

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
    check_semantics_against(id, &SEMANTICS_ID, PREVIOUS_SEMANTICS_IDS)
}

/// Classify a stored identifier against a crate's own `current` identifier and
/// its `previous` ones (the earlier identifiers whose pinned bits it still
/// reproduces).
///
/// The comparison every ALICE crate makes for its own identifier: the current
/// one is [`Verified`](SemanticsCheck::Verified) (even when `previous` also
/// lists it), a listed earlier one is
/// [`VerifiedPrevious`](SemanticsCheck::VerifiedPrevious) carrying the stored
/// value, anything else is [`Mismatch`](SemanticsCheck::Mismatch). The order of
/// `previous` does not change the result. [`check_semantics`] is this function
/// with this crate's [`SEMANTICS_ID`] and [`PREVIOUS_SEMANTICS_IDS`].
///
/// ```
/// use alice_det_math::semantics::check_semantics_against;
/// use alice_det_math::SemanticsCheck;
/// const OURS: [u8; 32] = [7; 32];
/// const OLD: [u8; 32] = [6; 32];
/// assert_eq!(check_semantics_against(&OURS, &OURS, &[OLD]), SemanticsCheck::Verified);
/// assert_eq!(check_semantics_against(&OLD, &OURS, &[OLD]), SemanticsCheck::VerifiedPrevious(OLD));
/// assert_eq!(check_semantics_against(&[0; 32], &OURS, &[OLD]), SemanticsCheck::Mismatch);
/// ```
pub const fn check_semantics_against(
    stored: &[u8; 32],
    current: &[u8; 32],
    previous: &[[u8; 32]],
) -> SemanticsCheck {
    if bytes_eq(stored, current) {
        return SemanticsCheck::Verified;
    }
    let mut i = 0;
    while i < previous.len() {
        if bytes_eq(stored, &previous[i]) {
            return SemanticsCheck::VerifiedPrevious(*stored);
        }
        i += 1;
    }
    SemanticsCheck::Mismatch
}

/// A stored identifier that is neither the crate's current identifier nor one
/// of its earlier ones, as returned by [`verify_semantics_against`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticsMismatch {
    /// The identifier the stored bytes or the peer carried.
    pub stored: [u8; 32],
    /// The current identifier it was compared with.
    pub expected: [u8; 32],
}

impl core::fmt::Display for SemanticsMismatch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("semantics identifier mismatch: stored ")?;
        for b in self.stored {
            write!(f, "{b:02x}")?;
        }
        f.write_str(", expected ")?;
        for b in self.expected {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SemanticsMismatch {}

/// [`check_semantics_against`] as a `Result`: the accepting outcomes
/// ([`Verified`](SemanticsCheck::Verified) and
/// [`VerifiedPrevious`](SemanticsCheck::VerifiedPrevious)) are `Ok`.
///
/// # Errors
///
/// [`SemanticsMismatch`] naming `stored` and `current` when `stored` is
/// neither `current` nor in `previous`.
pub const fn verify_semantics_against(
    stored: &[u8; 32],
    current: &[u8; 32],
    previous: &[[u8; 32]],
) -> Result<SemanticsCheck, SemanticsMismatch> {
    match check_semantics_against(stored, current, previous) {
        SemanticsCheck::Mismatch => Err(SemanticsMismatch {
            stored: *stored,
            expected: *current,
        }),
        accepted => Ok(accepted),
    }
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
