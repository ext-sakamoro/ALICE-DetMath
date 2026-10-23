//! Oracle for `check_semantics_against` / `verify_semantics_against` / `SemanticsMismatch`.
//!
//! The comparison every ALICE crate makes against its own identifier: a stored 32-byte
//! identifier is this crate's current one, one of its earlier ones, or neither. Expected
//! values come from the definition, not from the code under test: the identifiers here are
//! chosen byte patterns, the hex in the message is formatted by the test itself, and for the
//! crate's own identifiers the result must equal the existing `check_semantics`.

use alice_det_math::semantics::{
    check_semantics_against, verify_semantics_against, SemanticsMismatch,
};
use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};

const CUR: [u8; 32] = [0x11; 32];
const OLD_A: [u8; 32] = [0x22; 32];
const OLD_B: [u8; 32] = [0x33; 32];
const OTHER: [u8; 32] = [0x44; 32];

fn hex(b: &[u8; 32]) -> String {
    use std::fmt::Write;
    b.iter().fold(String::new(), |mut s, x| {
        write!(s, "{x:02x}").unwrap();
        s
    })
}

#[test]
fn the_three_outcomes() {
    let prev = [OLD_A, OLD_B];
    assert_eq!(
        check_semantics_against(&CUR, &CUR, &prev),
        SemanticsCheck::Verified
    );
    assert_eq!(
        check_semantics_against(&OLD_A, &CUR, &prev),
        SemanticsCheck::VerifiedPrevious(OLD_A)
    );
    assert_eq!(
        check_semantics_against(&OLD_B, &CUR, &prev),
        SemanticsCheck::VerifiedPrevious(OLD_B)
    );
    assert_eq!(
        check_semantics_against(&OTHER, &CUR, &prev),
        SemanticsCheck::Mismatch
    );
}

#[test]
fn no_earlier_identifiers_accepts_only_the_current_one() {
    assert_eq!(
        check_semantics_against(&CUR, &CUR, &[]),
        SemanticsCheck::Verified
    );
    assert_eq!(
        check_semantics_against(&OLD_A, &CUR, &[]),
        SemanticsCheck::Mismatch
    );
}

#[test]
fn the_current_identifier_wins_over_a_listed_copy_of_it() {
    // a list that also names the current identifier still reads it as current
    assert_eq!(
        check_semantics_against(&CUR, &CUR, &[OLD_A, CUR]),
        SemanticsCheck::Verified
    );
}

#[test]
fn every_position_in_the_earlier_list_is_found_and_carries_the_stored_value() {
    let prev: Vec<[u8; 32]> = (0u8..8).map(|i| [0x50 + i; 32]).collect();
    for (i, id) in prev.iter().enumerate() {
        assert_eq!(
            check_semantics_against(id, &CUR, &prev),
            SemanticsCheck::VerifiedPrevious(*id),
            "position {i}"
        );
        // the order of the list does not change the answer
        let mut reversed = prev.clone();
        reversed.reverse();
        assert_eq!(
            check_semantics_against(id, &CUR, &reversed),
            SemanticsCheck::VerifiedPrevious(*id)
        );
    }
}

#[test]
fn one_bit_off_in_any_byte_is_a_mismatch() {
    let prev = [OLD_A];
    for base in [CUR, OLD_A] {
        for byte in 0..32 {
            for bit in 0..8 {
                let mut near = base;
                near[byte] ^= 1 << bit;
                assert_eq!(
                    check_semantics_against(&near, &CUR, &prev),
                    SemanticsCheck::Mismatch,
                    "byte {byte} bit {bit}"
                );
            }
        }
    }
}

#[test]
fn on_its_own_identifiers_it_agrees_with_check_semantics() {
    let mut ids = vec![SEMANTICS_ID, OTHER];
    ids.extend_from_slice(PREVIOUS_SEMANTICS_IDS);
    let mut near = SEMANTICS_ID;
    near[31] ^= 1;
    ids.push(near);
    for id in &ids {
        assert_eq!(
            check_semantics_against(id, &SEMANTICS_ID, PREVIOUS_SEMANTICS_IDS),
            check_semantics(id),
            "{}",
            hex(id)
        );
    }
    assert!(
        !PREVIOUS_SEMANTICS_IDS.is_empty(),
        "the previous path was compared"
    );
}

#[test]
fn usable_in_a_const_context() {
    const C: SemanticsCheck = check_semantics_against(&CUR, &CUR, &[OLD_A]);
    const P: SemanticsCheck = check_semantics_against(&OLD_A, &CUR, &[OLD_A]);
    const M: SemanticsCheck = check_semantics_against(&OTHER, &CUR, &[OLD_A]);
    assert_eq!(
        (C, P, M),
        (
            SemanticsCheck::Verified,
            SemanticsCheck::VerifiedPrevious(OLD_A),
            SemanticsCheck::Mismatch
        )
    );
}

#[test]
fn verify_returns_the_accepting_outcomes_and_names_both_identifiers_otherwise() {
    let prev = [OLD_A];
    assert_eq!(
        verify_semantics_against(&CUR, &CUR, &prev),
        Ok(SemanticsCheck::Verified)
    );
    assert_eq!(
        verify_semantics_against(&OLD_A, &CUR, &prev),
        Ok(SemanticsCheck::VerifiedPrevious(OLD_A))
    );
    let err = verify_semantics_against(&OTHER, &CUR, &prev).unwrap_err();
    assert_eq!(
        err,
        SemanticsMismatch {
            stored: OTHER,
            expected: CUR
        }
    );
    assert_eq!(
        err.to_string(),
        format!(
            "semantics identifier mismatch: stored {}, expected {}",
            hex(&OTHER),
            hex(&CUR)
        )
    );
}

#[test]
fn the_mismatch_is_a_std_error() {
    fn takes(_: &dyn std::error::Error) {}
    takes(&SemanticsMismatch {
        stored: OTHER,
        expected: CUR,
    });
}
