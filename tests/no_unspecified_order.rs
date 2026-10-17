//! ALICE-Law G13: no output of the crate can depend on an order Rust leaves unspecified.
//!
//! `clippy.toml` bans `HashMap` / `HashSet` (and `RandomState`); this test scans the
//! library sources for the other forms the gate names, a thread or a parallel iterator
//! (whose partial results could be combined in a thread-count dependent order), and for
//! the hash collections as text, and requires none. The crate is `no_std` without
//! `alloc` by default and single-threaded; a new site fails here. A scan that read no
//! source file fails too.

use std::path::{Path, PathBuf};

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
    let forms = [
        "std::thread",
        "thread::spawn",
        "thread::scope",
        "rayon",
        "par_iter",
        "par_chunks",
        "into_par_iter",
        "HashMap",
        "HashSet",
    ];
    let mut found = Vec::new();
    let mut lines = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).expect("source");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            lines += 1;
            for form in forms {
                if code.contains(form) {
                    found.push(format!(
                        "{}:{}: `{form}`",
                        f.strip_prefix(root).expect("in the crate").display(),
                        n + 1
                    ));
                }
            }
        }
    }
    assert!(lines > 100, "read {lines} lines (compared nothing)");
    assert!(found.is_empty(), "G13 forms in src/:\n{}", found.join("\n"));
}
