#!/usr/bin/env python3
"""Tests of semantics_versions_check.py (tables on disk, no git)."""
from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import semantics_versions_check as svc  # noqa: E402

A = "a2a11b9b826227fa7a9ed8ab2c1ce63dbb07ad4c39aa2c46a363ddea6b47dbe9"
B = "85e0884623b154a3b2b264374907b6cbac09307ca3f9a08339e48c567d3cb315"
C = "135c407d4249c1c200e4d1be79a12e848ce5c355fc3a918da965849e697ef000"


class Check(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.d = Path(self._tmp.name)

    def run_check(self, base: str, head: str) -> tuple[int, str]:
        (self.d / "base.toml").write_text(base, encoding="utf-8")
        (self.d / "head.toml").write_text(head, encoding="utf-8")
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            rc = svc.main(["--base-file", str(self.d / "base.toml"), "--table", str(self.d / "head.toml")])
        return rc, out.getvalue()

    def test_an_added_version_passes(self):
        rc, out = self.run_check(f'"0.5.1" = "{B}"\n', f'"0.5.1" = "{B}"\n"0.6.0" = "{A}"\n')
        self.assertEqual(rc, 0, out)
        self.assertIn("compared 1 base row(s), 2 now, 0 finding(s)", out)

    def test_a_changed_row_fails(self):
        rc, out = self.run_check(f'"0.5.1" = "{B}"\n', f'"0.5.1" = "{A}"\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("0.5.1: the base records 85e08846, now a2a11b9b", out)

    def test_a_removed_row_fails(self):
        rc, out = self.run_check(f'"0.5.0" = "{C}"\n"0.5.1" = "{B}"\n', f'"0.5.1" = "{B}"\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("0.5.0: the base records 135c407d, and the row is gone", out)

    def test_an_empty_base_compares_nothing(self):
        rc, out = self.run_check("# nothing\n", f'"0.6.0" = "{A}"\n')
        self.assertEqual(rc, 2, out)
        self.assertIn("compared nothing", out)

    def test_a_malformed_or_repeated_row_is_unreadable(self):
        for head in (f'"0.6.0" = "{A.upper()}"\n', '"0.6.0" = "zz"\n', f'"0.6.0" = "{A}"\n"0.6.0" = "{B}"\n'):
            rc, _ = self.run_check(f'"0.5.1" = "{B}"\n', head)
            self.assertEqual(rc, 2, head)

    def test_the_committed_table_reads(self):
        self.assertGreater(len(svc.rows(svc.TABLE.read_text(encoding="utf-8"))), 0)


if __name__ == "__main__":
    unittest.main()
