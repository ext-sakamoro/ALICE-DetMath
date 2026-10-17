//! ALICE-Law G4: the double-precision kernels `k_sin64` / `k_cos64` keep the single caller
//! their contraction evidence was measured through.
//!
//! Their only caller is the f32 `tan` (`src/single.rs`); `sin64` / `cos64` / `tan64` use
//! `k_sin_tail` / `k_cos_tail`. Measured 2026-10-18 with `cargo test --release --test
//! exhaustive_f32 tan -- --ignored` (all 2^32 inputs, about 37 s, also run by
//! `.github/workflows/exhaustive-f32.yml`): a contraction (one-rounding multiply-add) at the
//! polynomial step of either kernel (`S1 + z * r`, `C1 + z * (…)`) moves the f32 `tan`
//! digest, and one at the final combination (`x + v * (…)`, `(1 - w) - hz + z * r`) leaves
//! every one of the 2^32 outputs unchanged, so at that output it is equivalent and no test
//! can tell it apart. That equivalence holds only for `tan`'s rounding to f32: a second
//! caller, or a public path to the kernels, would expose their double result, where the
//! final combination's contraction is visible, and then needs contraction goldens of its own.
//! This test fails until those are added and the expected count here is raised with them.
//! A scan that read no source file fails.

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

/// `(file, line)` of every call of `name(` in the library sources, with line comments
/// removed and the definition `fn name(` excluded
fn calls(files: &[(PathBuf, String)], name: &str) -> Vec<(String, usize)> {
    let needle = format!("{name}(");
    let def = format!("fn {name}(");
    let mut out = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.contains(&def) {
                continue;
            }
            let n = code.matches(&needle).count();
            for _ in 0..n {
                out.push((path.display().to_string(), i + 1));
            }
        }
    }
    out
}

#[test]
fn k_sin64_and_k_cos64_keep_their_single_caller_and_stay_crate_private() {
    let mut paths = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut paths,
    );
    let files: Vec<(PathBuf, String)> = paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("readable source");
            (p, text)
        })
        .collect();
    assert!(!files.is_empty(), "read no source file (compared nothing)");
    let all: String = files.iter().map(|(_, t)| t.as_str()).collect();
    for name in ["k_sin64", "k_cos64"] {
        let found = calls(&files, name);
        assert_eq!(
            found.len(),
            1,
            "{name} has {} caller(s) {found:?}; its contraction evidence covers exactly one, \
             the f32 `tan` (src/single.rs): a new caller needs contraction goldens of its own \
             before this count is raised",
            found.len()
        );
        assert!(
            found[0].0.ends_with("single.rs"),
            "{name}'s caller moved to {found:?}, not the f32 `tan` the evidence was measured through"
        );
        assert!(
            all.contains(&format!("pub(crate) fn {name}(")),
            "{name} must stay pub(crate): a public path would expose its double result"
        );
        assert!(
            !all.lines()
                .any(|l| l.trim_start().starts_with("pub use") && l.contains(name)),
            "{name} must not be re-exported publicly"
        );
    }
}
