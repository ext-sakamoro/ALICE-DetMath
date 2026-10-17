//! ALICE-Law G13: no output of the crate can depend on an order Rust leaves unspecified.
//!
//! `clippy.toml` bans `HashMap` / `HashSet` (and `RandomState`). This test is a second,
//! textual layer: it tokenizes the library sources (comments, string, raw-string, byte-string
//! and character literals removed, so a `//` inside a string does not hide the rest of the
//! line) and requires that no identifier names a thread, a parallel iterator or a hash
//! collection. Matching identifiers rather than paths also catches a renamed import
//! (`use std::{thread as t}`, `use std as s; s::thread::...`): the name `thread` still
//! appears in it. It does not resolve paths; a path-resolving clippy ban of
//! `std::thread::spawn` / `scope` / `Builder` is pending (see the G13 note in
//! `alice-law-conformance.toml`). A scan that read no source file fails.

use std::path::{Path, PathBuf};

/// The identifiers the scan refuses: a thread, a parallel iterator, a hash collection.
const NAMES: [&str; 10] = [
    "thread",
    "spawn",
    "rayon",
    "par_iter",
    "par_chunks",
    "par_chunks_mut",
    "into_par_iter",
    "par_bridge",
    "HashMap",
    "HashSet",
];

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The identifiers of Rust source text with their 1-based line, after removing line and
/// (nested) block comments, string and byte-string literals (with escapes), raw strings
/// of any `#` count, and character literals (a lifetime `'a` is kept as an identifier).
fn identifiers(src: &str) -> Vec<(usize, String)> {
    let c: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    let at = |k: usize| c.get(k).copied().unwrap_or('\0');
    while i < c.len() {
        let ch = c[i];
        if ch == '\n' {
            line += 1;
            i += 1;
        } else if ch == '/' && at(i + 1) == '/' {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if ch == '/' && at(i + 1) == '*' {
            let mut depth = 0;
            loop {
                if i >= c.len() {
                    break;
                }
                if c[i] == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if c[i] == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if c[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if (ch == 'r' || (ch == 'b' && at(i + 1) == 'r')) && {
            let s = if ch == 'b' { i + 2 } else { i + 1 };
            let mut k = s;
            while at(k) == '#' {
                k += 1;
            }
            at(k) == '"' && (k > s || at(s) == '"')
        } {
            // raw string: r"…", r#"…"#, br##"…"##
            let mut k = if ch == 'b' { i + 2 } else { i + 1 };
            let mut hashes = 0;
            while at(k) == '#' {
                hashes += 1;
                k += 1;
            }
            k += 1;
            loop {
                if k >= c.len() {
                    break;
                }
                if c[k] == '"' && (1..=hashes).all(|h| at(k + h) == '#') {
                    k += 1 + hashes;
                    break;
                }
                if c[k] == '\n' {
                    line += 1;
                }
                k += 1;
            }
            i = k;
        } else if ch == '"' || (ch == 'b' && at(i + 1) == '"') {
            let mut k = if ch == 'b' { i + 2 } else { i + 1 };
            while k < c.len() && c[k] != '"' {
                if c[k] == '\\' {
                    k += 1;
                }
                if at(k) == '\n' {
                    line += 1;
                }
                k += 1;
            }
            i = k + 1;
        } else if ch == '\'' || (ch == 'b' && at(i + 1) == '\'') {
            let s = if ch == 'b' { i + 1 } else { i };
            // a character literal closes within a few chars ('x', '\n', '\u{..}');
            // otherwise it is a lifetime and the name is read as an identifier next
            if at(s + 1) == '\\' {
                let mut k = s + 2;
                while k < c.len() && c[k] != '\'' {
                    k += 1;
                }
                i = k + 1;
            } else if at(s + 2) == '\'' {
                i = s + 3;
            } else {
                i = s + 1;
            }
        } else if ch.is_alphabetic() || ch == '_' {
            let s = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push((line, c[s..i].iter().collect()));
        } else {
            i += 1;
        }
    }
    out
}

fn refused(src: &str) -> Vec<(usize, String)> {
    identifiers(src)
        .into_iter()
        .filter(|(_, w)| NAMES.contains(&w.as_str()))
        .collect()
}

/// Canary: each refused name is found in code, through a renamed import and after a
/// string literal holding `//`, and is not found inside a comment, a string or a raw string.
#[test]
fn the_scan_finds_each_planted_name_and_ignores_literals_and_comments() {
    let mut found = 0;
    for name in NAMES {
        let code = format!("    let _ = {name}::planted();");
        assert_eq!(refused(&code).len(), 1, "{name} in {code:?}");
        found += 1;
        for hidden in [
            format!("    // {name}"),
            format!("    /* {name} /* nested */ {name} */"),
            format!("    let s = \"{name}\";"),
            format!("    let s = r#\"{name} \"quoted\"\"#;"),
            format!("    let s = b\"{name}\";"),
        ] {
            assert!(refused(&hidden).is_empty(), "{name} counted in {hidden:?}");
        }
    }
    assert_eq!(found, NAMES.len());
    // the evasions measured against the earlier text scan
    for (code, want) in [
        ("use std::{thread as t}; fn f() { t::spawn(|| 1); }", 2),
        (
            "use std as s; fn f() { s::thread::Builder::new().spawn(|| 1); }",
            2,
        ),
        ("let url = \"http://x\"; std::thread::spawn(|| 1);", 2),
        ("let c = '\"'; let d = '/'; std::thread::scope(|_| ());", 1),
        (
            "fn f<'a>(x: &'a u8) -> &'a u8 { std::thread::yield_now(); x }",
            1,
        ),
    ] {
        assert_eq!(refused(code).len(), want, "{code}");
    }
    // a line count survives multi-line literals and comments
    let lines = refused("let s = \"a\nb\";\n/* c\nd */\nthread");
    assert_eq!(lines, vec![(5, "thread".to_owned())]);
}

#[test]
fn no_thread_parallel_iterator_or_hash_collection_in_src() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    assert!(
        files.len() >= 3,
        "read {} source files (compared nothing)",
        files.len()
    );
    let mut found = Vec::new();
    let mut idents = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).expect("source");
        idents += identifiers(&text).len();
        for (n, name) in refused(&text) {
            found.push(format!(
                "{}:{n}: `{name}`",
                f.strip_prefix(root).expect("in the crate").display()
            ));
        }
    }
    assert!(
        idents > 1000,
        "read {idents} identifiers (compared nothing)"
    );
    assert!(found.is_empty(), "G13 names in src/:\n{}", found.join("\n"));
}
