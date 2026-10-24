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
PKG = ["Cargo.toml", "LICENSE-Proprietary", "TRADEMARK_NOTICE", "src/lib.rs"]


class LicenseCheck(unittest.TestCase):
    def setUp(self) -> None:
        self.d = Path(tempfile.mkdtemp())
        for name in ["Cargo.toml", "README.md", "README_JP.md", *lc.REQUIRED]:
            shutil.copy(ROOT / name, self.d / name)

    def test_the_repo_as_it_is_is_green(self) -> None:
        checked, problems = lc.check(self.d, PKG)
        self.assertGreater(checked, 0)
        self.assertEqual(problems, [])

    def edit(self, name: str, old: str, new: str) -> None:
        p = self.d / name
        text = p.read_text(encoding="utf-8")
        self.assertIn(old, text)
        p.write_text(text.replace(old, new), encoding="utf-8")

    def test_apache_in_cargo_toml_is_red(self) -> None:
        self.edit("Cargo.toml", 'license = "LicenseRef-Proprietary"', 'license = "Apache-2.0"')
        self.assertTrue(any("Cargo.toml license" in x for x in lc.check(self.d, PKG)[1]))

    def test_missing_publish_false_is_red(self) -> None:
        self.edit("Cargo.toml", "publish = false\n", "")
        self.assertTrue(any("publish = false" in x for x in lc.check(self.d, PKG)[1]))

    def test_missing_licence_text_is_red(self) -> None:
        (self.d / "LICENSE-Proprietary").unlink()
        self.assertTrue(any("LICENSE-Proprietary is missing" in x for x in lc.check(self.d, PKG)[1]))

    def test_licence_text_not_packaged_is_red(self) -> None:
        pkg = [p for p in PKG if p != "LICENSE-Proprietary"]
        self.assertTrue(any("LICENSE-Proprietary is not in the published package" in x for x in lc.check(self.d, pkg)[1]))

    def test_leftover_old_licence_files_are_red(self) -> None:
        for name in ["LICENSE-APACHE", "LICENSE-MIT", "NOTICE"]:
            (self.d / name).write_text("old", encoding="utf-8")
            self.assertTrue(any(f"{name} exists" in x for x in lc.check(self.d, PKG)[1]), name)
            (self.d / name).unlink()

    def test_old_licence_file_in_package_is_red(self) -> None:
        self.assertTrue(any("NOTICE is in the published package" in x for x in lc.check(self.d, PKG + ["NOTICE"])[1]))

    def test_readme_still_apache_is_red(self) -> None:
        self.edit("README_JP.md", "License: LicenseRef-Proprietary", "License: Apache-2.0")
        self.assertTrue(any("README_JP.md: License line" in x for x in lc.check(self.d, PKG)[1]))

    def test_readme_without_licence_link_is_red(self) -> None:
        self.edit("README.md", "](LICENSE-Proprietary)", ")")
        self.assertTrue(any("README.md: does not link LICENSE-Proprietary" in x for x in lc.check(self.d, PKG)[1]))


if __name__ == "__main__":
    unittest.main()
