"""Teeth for scripts/license_check.py: each kind of drift is red."""
from __future__ import annotations

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import license_check as lc  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
PKG = ["Cargo.toml", "LICENSE-APACHE", "NOTICE", "TRADEMARK_NOTICE", "src/lib.rs"]


class LicenseCheck(unittest.TestCase):
    def setUp(self) -> None:
        self.d = Path(tempfile.mkdtemp())
        for name in ["Cargo.toml", "README.md", "README_JP.md", *lc.REQUIRED]:
            shutil.copy(ROOT / name, self.d / name)

    def test_the_repo_as_it_is_is_green(self) -> None:
        checked, problems = lc.check(self.d, PKG)
        self.assertGreater(checked, 0)
        self.assertEqual(problems, [])

    def test_dual_licence_in_cargo_toml_is_red(self) -> None:
        p = self.d / "Cargo.toml"
        p.write_text(p.read_text(encoding="utf-8").replace('license = "Apache-2.0"', 'license = "MIT OR Apache-2.0"'), encoding="utf-8")
        self.assertTrue(any("Cargo.toml license" in x for x in lc.check(self.d, PKG)[1]))

    def test_missing_notice_is_red(self) -> None:
        (self.d / "NOTICE").unlink()
        self.assertTrue(any("NOTICE is missing" in x for x in lc.check(self.d, PKG)[1]))

    def test_notice_not_packaged_is_red(self) -> None:
        pkg = [p for p in PKG if p != "NOTICE"]
        self.assertTrue(any("NOTICE is not in the published package" in x for x in lc.check(self.d, pkg)[1]))

    def test_leftover_mit_file_is_red(self) -> None:
        (self.d / "LICENSE-MIT").write_text("MIT", encoding="utf-8")
        self.assertTrue(any("LICENSE-MIT exists" in x for x in lc.check(self.d, PKG)[1]))

    def test_readme_still_dual_is_red(self) -> None:
        p = self.d / "README_JP.md"
        p.write_text(p.read_text(encoding="utf-8").replace("License: Apache-2.0", "License: MIT OR Apache-2.0"), encoding="utf-8")
        self.assertTrue(any("README_JP.md" in x for x in lc.check(self.d, PKG)[1]))


if __name__ == "__main__":
    unittest.main()
