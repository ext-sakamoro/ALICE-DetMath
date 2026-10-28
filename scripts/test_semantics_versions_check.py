#!/usr/bin/env python3
"""Tests of semantics_versions_check.py (tables on disk; the CI base in a throwaway git repository)."""
from __future__ import annotations

import contextlib
import io
import os
import subprocess
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


D = "dd43c59f" + "0" * 56
E = "6fc214e9" + "1" * 56


class CiBase(unittest.TestCase):
    """--ci in a repository with an origin/main: a push to main is judged against main before the push"""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.d = Path(self._tmp.name)
        self._cwd = os.getcwd()
        self.addCleanup(os.chdir, self._cwd)
        os.chdir(self.d)
        env = {"GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}
        self._env = {**os.environ, **env, "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
                     "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
        self.git("init", "-q", "-b", "main")
        (self.d / "scripts").mkdir()

    def git(self, *args: str) -> str:
        return subprocess.run(["git", *args], cwd=self.d, env=self._env, check=True, capture_output=True,
                              text=True).stdout.strip()

    def commit(self, table: str) -> str:
        (self.d / "scripts" / "semantics_versions.toml").write_text(table, encoding="utf-8")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "c")
        return self.git("rev-parse", "HEAD")

    def check(self, before: str, ref: str = "refs/heads/main", event: str = "push") -> tuple[int, str]:
        self.git("update-ref", "refs/remotes/origin/main", "HEAD")
        out = io.StringIO()
        old_env = os.environ.copy()
        os.environ.update(self._env)
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
                rc = svc.main(["--ci", "--event", event, "--ref", ref, "--before", before,
                               "--table", str(self.d / "scripts" / "semantics_versions.toml")])
        finally:
            os.environ.clear()
            os.environ.update(old_env)
        return rc, out.getvalue()

    def test_rewriting_a_row_that_stood_on_main_fails(self):
        before = self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{D}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        rc, out = self.check(before)
        self.assertEqual(rc, 1, out)
        self.assertIn("0.6.1: the base records dd43c59f, now 6fc214e9", out)

    def test_a_row_added_and_changed_within_one_push_passes(self):
        before = self.commit(f'"0.6.0" = "{A}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{D}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        rc, out = self.check(before)
        self.assertEqual(rc, 0, out)
        self.assertIn(f"base {before} (rule before, push refs/heads/main)", out)
        self.assertIn("compared 1 base row(s), 2 now, 0 finding(s)", out)

    def test_a_before_this_clone_lacks_fails(self):
        self.commit(f'"0.6.0" = "{A}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        rc, out = self.check("1234567890abcdef1234567890abcdef12345678")
        self.assertEqual(rc, 2, out)
        self.assertIn("is not in this clone", out)

    def test_a_before_that_is_not_an_ancestor_fails(self):
        self.commit(f'"0.6.0" = "{A}"\n')
        side = self.git("commit-tree", "-m", "side", self.git("rev-parse", "HEAD^{tree}"))
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        rc, out = self.check(side)
        self.assertEqual(rc, 2, out)
        self.assertIn("is not an ancestor of HEAD", out)

    def test_head_on_origin_main_off_main_says_it_used_the_parent(self):
        parent = self.commit(f'"0.6.0" = "{A}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        rc, out = self.check("0" * 40, ref="refs/heads/ci/x")
        self.assertEqual(rc, 0, out)
        self.assertIn(f"base {parent} (rule HEAD~1, push refs/heads/ci/x)", out)

    def test_an_all_zero_or_missing_before_on_main_fails(self):
        self.commit(f'"0.6.0" = "{A}"\n')
        for before, why in (("0" * 40, "all-zero before"), ("", "needs github.event.before as a full SHA"),
                            ("abc", "needs github.event.before as a full SHA")):
            rc, out = self.check(before)
            self.assertEqual(rc, 2, (before, out))
            self.assertIn(why, out)

    def test_off_main_the_base_is_the_merge_base(self):
        base = self.commit(f'"0.6.0" = "{A}"\n')
        self.git("update-ref", "refs/remotes/origin/main", base)
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{D}"\n')
        self.commit(f'"0.6.0" = "{A}"\n"0.6.1" = "{E}"\n')
        out = io.StringIO()
        old_env = os.environ.copy()
        os.environ.update(self._env)
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
                rc = svc.main(["--ci", "--event", "push", "--ref", "refs/heads/ci/x", "--before", "0" * 40,
                               "--table", str(self.d / "scripts" / "semantics_versions.toml")])
        finally:
            os.environ.clear()
            os.environ.update(old_env)
        self.assertEqual(rc, 0, out.getvalue())
        self.assertIn(f"base {base} (rule merge-base, push refs/heads/ci/x)", out.getvalue())


if __name__ == "__main__":
    unittest.main()
