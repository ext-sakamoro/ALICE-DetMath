#!/usr/bin/env python3
"""Tests of scripts/law_bits_check.py (fixture trees, no cargo)."""
from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import law_bits_check as lb  # noqa: E402

HELPER = "pub fn bits32(x: f32) -> u32 { x.to_bits() }\n"
CLEAN = """use sha2::{Digest, Sha256};
#[path = "support/digest_bits.rs"]
mod digest_bits;
fn h(xs: &[f32]) { let mut h = Sha256::new(); for &x in xs { h.update(digest_bits::out32(x)); } }
"""


class Tree:
    def __init__(self, d: str, files: dict[str, str], helper: bool = True):
        self.root = Path(d)
        if helper:
            files = {"tests/support/digest_bits.rs": HELPER, **files}
        for rel, text in files.items():
            p = self.root / rel
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text, encoding="utf-8")

    def run(self) -> tuple[int, list[str], int]:
        findings, scanned = lb.scan(self.root)
        return lb.main(["x", str(self.root)]), findings, scanned


class Cases(unittest.TestCase):
    def check(self, files: dict[str, str], helper: bool = True) -> tuple[int, list[str], int]:
        with tempfile.TemporaryDirectory() as d:
            return Tree(d, files, helper).run()

    def test_a_digest_test_through_the_helper_passes(self):
        self.assertEqual(self.check({"tests/golden.rs": CLEAN}), (0, [], 1))

    def test_a_raw_to_bits_in_a_digest_test_is_a_finding(self):
        src = CLEAN.replace("digest_bits::out32(x)", "x.to_bits().to_le_bytes()")
        rc, findings, scanned = self.check({"tests/golden.rs": src})
        self.assertEqual((rc, scanned, len(findings)), (1, 1, 1), findings)
        self.assertIn("tests/golden.rs:4", findings[0])

    def test_mapping_a_nan_to_the_canonical_one_is_a_finding(self):
        src = CLEAN + "fn f(x: f32) -> u32 { if x.is_nan() { 0x7fc0_0000 } else { x.to_bits() } }\n"
        rc, findings, _ = self.check({"tests/dense_golden.rs": src})
        self.assertEqual((rc, len(findings)), (1, 1), findings)

    def test_the_path_form_is_a_finding(self):
        src = CLEAN + "fn f(x: f64) -> u64 { f64::to_bits(x) }\n"
        self.assertEqual(self.check({"tests/golden.rs": src})[0], 1)

    def test_an_fnv_digest_is_a_digest_test(self):
        src = ("fn s(f: fn(f32) -> f32) { let mut h: u64 = 0xcbf2_9ce4_8422_2325;\n"
               "  for b in 0..=9u32 { h = (h ^ u64::from(f(f32::from_bits(b)).to_bits())).wrapping_mul(3); } }\n")
        rc, findings, scanned = self.check({"tests/golden.rs": CLEAN, "tests/exhaustive_f32.rs": src})
        self.assertEqual((rc, scanned, len(findings)), (1, 2, 1), findings)
        self.assertIn("exhaustive_f32.rs:2", findings[0])

    def test_blake3_is_a_digest(self):
        src = "fn d(x: f64) { let mut h = blake3::Hasher::new(); h.update(&x.to_bits().to_le_bytes()); }\n"
        self.assertEqual(self.check({"tests/law_id.rs": src})[:2][0], 1)

    def test_a_test_without_a_digest_is_not_scanned(self):
        src = "fn t() { assert_eq!(1.0f32.to_bits(), 0x3f80_0000); }\n"
        rc, findings, scanned = self.check({"tests/golden.rs": CLEAN, "tests/accuracy.rs": src})
        self.assertEqual((rc, findings, scanned), (0, [], 1))

    def test_a_comment_does_not_count(self):
        src = CLEAN + "// not x.to_bits() but digest_bits::bits32\n/// Sha256 over to_bits\n"
        self.assertEqual(self.check({"tests/golden.rs": src}), (0, [], 1))
        # a digest named only in a comment does not make a digest test
        self.assertEqual(self.check({"tests/golden.rs": CLEAN, "tests/a.rs": "// Sha256\nfn f(x: f32) -> u32 { x.to_bits() }\n"}),
                         (0, [], 1))

    def test_the_helper_itself_is_not_scanned(self):
        helper = HELPER + "use sha2::Sha256;\n"
        with tempfile.TemporaryDirectory() as d:
            t = Tree(d, {"tests/golden.rs": CLEAN}, helper=False)
            (t.root / "tests/support").mkdir(parents=True)
            (t.root / "tests/support/digest_bits.rs").write_text(helper, encoding="utf-8")
            self.assertEqual(t.run(), (0, [], 1))

    def test_a_nested_test_file_is_scanned(self):
        src = CLEAN.replace("digest_bits::out32(x)", "x.to_bits().to_le_bytes()")
        rc, findings, scanned = self.check({"tests/golden.rs": CLEAN, "tests/common/pins.rs": src})
        self.assertEqual((rc, scanned, len(findings)), (1, 2, 1), findings)

    def test_no_digest_test_fails(self):
        rc, _, scanned = self.check({"tests/accuracy.rs": "fn t() {}\n"})
        self.assertEqual((rc, scanned), (2, 0))

    def test_a_missing_helper_fails(self):
        self.assertEqual(self.check({"tests/golden.rs": CLEAN}, helper=False)[0], 2)


if __name__ == "__main__":
    unittest.main(buffer=True)
