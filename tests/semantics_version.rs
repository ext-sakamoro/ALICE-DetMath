//! The version names the arithmetic: the crate's `SEMANTICS_ID` must be the identifier
//! `scripts/semantics_versions.toml` records for this crate's own version. An identifier that
//! moves without a new version (a changed arithmetic under an unchanged version number) fails
//! here, and so does a version with no row.

use std::collections::BTreeMap;

const TABLE: &str = include_str!("../scripts/semantics_versions.toml");

/// `"version" = "hex"` rows, comments and blank lines skipped; a malformed or repeated row is an `Err`
/// naming it (a returned error rather than a panic, so the refusal is checked the same way on every
/// target, including those where a panic aborts the process)
fn rows(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once('=').ok_or_else(|| {
            format!(
                "semantics_versions.toml:{}: not `\"version\" = \"hex\"`",
                n + 1
            )
        })?;
        let k = k.trim().trim_matches('"').to_string();
        let v = v.trim().trim_matches('"').to_string();
        if !(v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        {
            return Err(format!(
                "semantics_versions.toml:{}: {k}: not 64 lower-case hex digits",
                n + 1
            ));
        }
        if out.insert(k.clone(), v).is_some() {
            return Err(format!("semantics_versions.toml: {k} has two rows"));
        }
    }
    Ok(out)
}

fn hex(id: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    id.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[test]
fn the_crate_version_names_its_semantics_id() {
    let table = rows(TABLE).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        !table.is_empty(),
        "semantics_versions.toml: no row (compared nothing)"
    );
    let version = env!("CARGO_PKG_VERSION");
    let recorded = table.get(version).unwrap_or_else(|| {
        panic!(
            "alice-det-math {version} has no row in scripts/semantics_versions.toml: a new \
             identifier needs a new version and its row"
        )
    });
    assert_eq!(
        &hex(&alice_det_math::SEMANTICS_ID),
        recorded,
        "alice-det-math {version}: SEMANTICS_ID is not the identifier its version names \
         (a changed identifier needs a new version, not a changed row)"
    );
}

#[test]
fn one_identifier_names_one_version() {
    let table = rows(TABLE).unwrap_or_else(|e| panic!("{e}"));
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for (version, id) in &table {
        if let Some(other) = seen.insert(id, version) {
            panic!("semantics_versions.toml: {id:.8} is recorded for both {other} and {version}");
        }
    }
}

#[test]
fn a_malformed_row_is_refused() {
    let bad = [
        "\"1.0.0\" = \"zz\"",
        "\"1.0.0\"",
        "\"1.0.0\" = \"A2A11B9B826227FA7A9ED8AB2C1CE63DBB07AD4C39AA2C46A363DDEA6B47DBE9\"",
    ];
    for text in bad {
        let got = rows(text);
        assert!(got.is_err(), "{text:?} was read: {got:?}");
    }
    let twice =
        "\"1.0.0\" = \"a2a11b9b826227fa7a9ed8ab2c1ce63dbb07ad4c39aa2c46a363ddea6b47dbe9\"\n\
                 \"1.0.0\" = \"85e0884623b154a3b2b264374907b6cbac09307ca3f9a08339e48c567d3cb315\"";
    assert_eq!(
        rows(twice),
        Err("semantics_versions.toml: 1.0.0 has two rows".to_string())
    );
}
