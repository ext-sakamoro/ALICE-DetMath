//! The value printed in the documentation of `SEMANTICS_ID` (the `text`
//! block in `src/lib.rs`, which docs.rs shows) is the constant itself.
//!
//! The constant is re-recorded from the golden pins; the doc block is
//! written by hand next to it, so it can be left behind when the pins move.
//! A reader who copies the documented value to check stored data would then
//! compare against an identifier no build produces.

use std::fs;
use std::path::Path;

fn documented_ids(src: &str) -> Vec<String> {
    // the doc comment right above `pub const SEMANTICS_ID`
    let at = src
        .find("pub const SEMANTICS_ID")
        .expect("src/lib.rs defines SEMANTICS_ID");
    let doc: Vec<&str> = src[..at]
        .lines()
        .rev()
        .take_while(|l| l.trim_start().starts_with("///"))
        .collect();
    doc.iter()
        .rev()
        .map(|l| l.trim_start().trim_start_matches("///").trim())
        .filter(|l| l.len() == 64 && l.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_owned)
        .collect()
}

fn hex(id: &[u8; 32]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_documented_value_is_the_constant() {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("read src/lib.rs");
    let ids = documented_ids(&src);
    assert_eq!(
        ids.len(),
        1,
        "the SEMANTICS_ID doc block shows exactly one 64-hex value: {ids:?}"
    );
    assert_eq!(ids[0], hex(&alice_det_math::SEMANTICS_ID));
}

#[test]
fn the_reader_finds_a_value_in_a_doc_block_and_ignores_other_lines() {
    let src = "/// text\n///\n/// ```text\n/// 00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\n/// ```\n/// 0011\npub const SEMANTICS_ID: [u8; 32] = [0; 32];\n";
    assert_eq!(
        documented_ids(src),
        vec!["00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff".to_owned()]
    );
    assert!(documented_ids("pub const SEMANTICS_ID: [u8; 32] = [0; 32];\n").is_empty());
}
