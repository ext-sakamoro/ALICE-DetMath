#!/usr/bin/env python3
"""Tests of scripts/fold_scan.py: the classification of a source file, the scan of throwaway git
repositories (reached as file:// URLs, so no network), the cache by main's SHA, and the comparison
with the two lists.

oracle: each fixture's expected class is fixed by what the fixture writes (a panicking table, an
assert next to the identifier, a plain use, a comment), never by running the scan first."""
from __future__ import annotations

import contextlib
import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fold_scan as fs  # noqa: E402

GIT_ENV = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}
DEP = '[package]\nname = "x"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\nalice-det-math = "0.6"\n'
NO_DEP = '[package]\nname = "x"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\nserde = "1"\n'
TABLE = 'pub const X_SEMANTICS_BY_DET_MATH: &[u8] = &[];\nconst fn f() { panic!("alice-det-math SEMANTICS_ID has no row in X_SEMANTICS_BY_DET_MATH: add its fold"); }\n'
ASSERT = "const _: () = assert!(\n    same32(&alice_det_math::SEMANTICS_ID, &GEN),\n    \"changed\",\n);\n"
USE = "pub fn id() -> [u8; 32] { alice_det_math::SEMANTICS_ID }\n"


def git(d: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=d, check=True, capture_output=True, text=True,
                          env={**os.environ, **GIT_ENV}).stdout.strip()


def repo(d: Path, files: dict[str, str], branch: str = "main") -> str:
    d.mkdir(parents=True, exist_ok=True)
    if not (d / ".git").exists():
        git(d, "init", "-q", "-b", branch)
        git(d, "config", "uploadpack.allowFilter", "true")
    for k, v in files.items():
        (d / k).parent.mkdir(parents=True, exist_ok=True)
        (d / k).write_text(v, encoding="utf-8")
    git(d, "add", "-A")
    git(d, "commit", "-q", "-m", "c")
    return git(d, "rev-parse", "HEAD")


class Classify(unittest.TestCase):
    def test_each_form(self):
        self.assertEqual(fs.classify("a.rs", TABLE), "breaks")
        self.assertEqual(fs.classify("a.rs", ASSERT), "breaks")
        self.assertEqual(fs.classify("a.rs", USE), "moves")
        self.assertIsNone(fs.classify("a.rs", "// alice_det_math::SEMANTICS_ID in a comment\n"))
        self.assertIsNone(fs.classify("a.rs", "fn f() {}\n"))

    def test_a_per_function_fold_is_a_breaker(self):
        # function_pin panics at compile time on a name the candidate no longer has
        self.assertEqual(fs.classify("a.rs", 'const A: [u8; 32] = alice_det_math::function_pin("sin");\n'), "breaks")
        self.assertEqual(fs.classify("a.rs", 'use alice_det_math::function_pin;\nconst A: [u8; 32] = function_pin("sin");\n'), "breaks")
        self.assertIsNone(fs.classify("a.rs", "// function_pin(\"sin\") in a comment\n"))

    def test_every_way_of_reaching_the_function_table_is_a_breaker(self):
        for text in ("use alice_det_math::function_pin as fp;\nconst A: [u8; 32] = fp(\"sin\");\n",
                     "const F: fn(&str) -> [u8; 32] = alice_det_math::function_pin;\n",
                     "const T: &[(&str, [u8; 32])] = alice_det_math::FUNCTION_PINS;\n"):
            with self.subTest(text):
                self.assertEqual(fs.classify("a.rs", text), "breaks")

    def test_an_assert_far_from_the_identifier_is_a_use(self):
        far = "const _: () = assert!(true);\n\n\n\n\npub const A: [u8; 32] = alice_det_math::SEMANTICS_ID;\n"
        self.assertEqual(fs.classify("a.rs", far), "moves")


class ScanRepo(unittest.TestCase):
    def setUp(self):
        self.d = Path(tempfile.mkdtemp(prefix="fold-scan-test-"))
        self.addCleanup(shutil.rmtree, self.d, True)

    def scan(self, path: Path) -> fs.Found:
        sha = fs.main_sha(f"file://{path}")
        work = Path(tempfile.mkdtemp(dir=self.d))
        return fs.scan_repo(f"file://{path}", sha, work)

    def test_a_breaker_a_mover_and_a_non_user(self):
        repo(self.d / "b", {"Cargo.toml": DEP, "src/semantics.rs": TABLE, "src/lib.rs": USE})
        repo(self.d / "m", {"Cargo.toml": DEP, "src/lib.rs": USE})
        repo(self.d / "n", {"Cargo.toml": NO_DEP, "src/lib.rs": USE})
        b, m, n = self.scan(self.d / "b"), self.scan(self.d / "m"), self.scan(self.d / "n")
        self.assertEqual((b.depends, b.breaks, b.moves), (True, ["src/semantics.rs"], ["src/lib.rs"]))
        self.assertEqual((m.depends, m.breaks, m.moves), (True, [], ["src/lib.rs"]))
        self.assertEqual((n.depends, n.breaks, n.moves), (False, [], []))

    def test_the_crate_that_defines_det_math_is_not_its_own_consumer(self):
        # a tool in its repository depends on it, and its source defines function_pin
        repo(self.d / "own", {
            "Cargo.toml": '[package]\nname = "alice-det-math"\nversion = "0.6.1"\nedition = "2021"\n',
            "src/semantics.rs": 'const A: [u8; 32] = function_pin("sin");\n',
            "tools/t/Cargo.toml": DEP, "tools/t/src/lib.rs": USE})
        f = self.scan(self.d / "own")
        self.assertEqual((f.depends, f.breaks, f.moves), (False, [], []))
        self.assertTrue(f.defines_det_math)

    def test_the_excluded_repository_is_named(self):
        results = {"ALICE-DetMath": fs.Found(defines_det_math=True), "M": fs.Found(True, [], ["l.rs"])}
        with contextlib.redirect_stdout(io.StringIO()) as o:
            fs.report(results, 2, 0, "o", [])
        self.assertIn("ALICE-DetMath defines alice-det-math: not counted as a user of it", o.getvalue())

    def test_the_exclusion_survives_the_cache(self):
        d = Path(tempfile.mkdtemp(prefix="fold-cache-"))
        self.addCleanup(shutil.rmtree, d, True)
        cache = d / "c.json"
        scanner = lambda url, sha, work: fs.Found(defines_det_math=True)
        fs.scan([fs.Repo("R", "u")], cache, sha_of=lambda u: "a" * 40, scanner=scanner)
        res, *_ = fs.scan([fs.Repo("R", "u")], cache, sha_of=lambda u: "a" * 40, scanner=None)
        self.assertTrue(res["R"].defines_det_math)

    def test_a_workspace_member_counts_and_tests_do_not(self):
        repo(self.d / "w", {"Cargo.toml": '[workspace]\nmembers = ["inner"]\n', "inner/Cargo.toml": DEP,
                            "inner/src/lib.rs": "fn f() {}\n", "inner/tests/t.rs": TABLE, "examples/e.rs": USE})
        f = self.scan(self.d / "w")
        self.assertEqual((f.depends, f.breaks, f.moves), (True, [], []))

    def test_main_is_read_not_the_default_branch(self):
        d = self.d / "r"
        repo(d, {"Cargo.toml": DEP, "src/lib.rs": "fn f() {}\n"})
        git(d, "checkout", "-q", "-b", "wip")
        repo(d, {"src/lib.rs": TABLE})
        # HEAD (what a clone without --branch checks out) is wip, which breaks; main does not
        self.assertEqual(git(d, "symbolic-ref", "--short", "HEAD"), "wip")
        f = self.scan(d)
        self.assertEqual((f.breaks, f.moves), ([], []))

    def test_a_repository_without_main(self):
        repo(self.d / "x", {"Cargo.toml": DEP}, branch="trunk")
        self.assertIsNone(fs.main_sha(f"file://{self.d / 'x'}"))


class Cache(unittest.TestCase):
    def test_an_unchanged_main_is_not_fetched_again(self):
        d = Path(tempfile.mkdtemp(prefix="fold-cache-"))
        self.addCleanup(shutil.rmtree, d, True)
        shas = {"u1": "a" * 40, "u2": "b" * 40, "u3": None}
        calls: list[str] = []

        def scanner(url, sha, work):
            calls.append(url)
            return fs.Found(True, ["src/s.rs"] if url == "u1" else [], [])
        repos = [fs.Repo("R1", "u1"), fs.Repo("R2", "u2"), fs.Repo("R3", "u3")]
        cache = d / "c.json"
        res, n, fetched, no_main = fs.scan(repos, cache, sha_of=shas.get, scanner=scanner)
        self.assertEqual((n, fetched, no_main, sorted(calls)), (2, 2, ["R3"], ["u1", "u2"]))
        self.assertEqual(res["R1"].breaks, ["src/s.rs"])
        calls.clear()
        shas["u2"] = "c" * 40
        res, n, fetched, _ = fs.scan(repos, cache, sha_of=shas.get, scanner=scanner)
        self.assertEqual((n, fetched, calls), (2, 1, ["u2"]))
        self.assertEqual(res["R1"].breaks, ["src/s.rs"])   # from the cache


class Rules(unittest.TestCase):
    def test_a_change_of_the_scan_rules_rescans_an_unchanged_repository(self):
        d = Path(tempfile.mkdtemp(prefix="fold-rules-"))
        self.addCleanup(shutil.rmtree, d, True)
        calls: list[str] = []

        def scanner(url, sha, work):
            calls.append(url)
            return fs.Found(True, [], [])
        repos = [fs.Repo("R1", "u1")]
        cache = d / "c.json"
        fs.scan(repos, cache, sha_of=lambda u: "a" * 40, scanner=scanner)
        fs.scan(repos, cache, sha_of=lambda u: "a" * 40, scanner=scanner)
        self.assertEqual(calls, ["u1"])          # the second run reads the cache
        real = fs.RULES
        fs.RULES = "another rule set"
        self.addCleanup(setattr, fs, "RULES", real)
        fs.scan(repos, cache, sha_of=lambda u: "a" * 40, scanner=scanner)
        self.assertEqual(calls, ["u1", "u1"])    # the same main, new rules: scanned again

    def test_the_rules_are_the_scan_source(self):
        self.assertEqual(fs.RULES, fs.hashlib.sha256(Path(fs.__file__).read_bytes()).hexdigest())

    def test_private_repositories_are_excluded_whatever_the_token_lists(self):
        out = "ALICE-A\thttps://x/A\tpublic\nALICE-P\thttps://x/P\tprivate\nALICE-I\thttps://x/I\tinternal\n"
        real = fs.subprocess.run

        def fake(argv, **kw):
            if argv[:2] == ["gh", "api"]:
                return subprocess.CompletedProcess(argv, 0, out, "")
            return real(argv, **kw)
        fs.subprocess.run = fake
        self.addCleanup(setattr, fs.subprocess, "run", real)
        self.assertEqual([r.name for r in fs.list_repos("o")], ["ALICE-A"])

    def test_a_breaker_is_not_listed_as_a_mover_and_no_main_is_printed(self):
        results = {"B": fs.Found(True, ["s.rs"], ["l.rs"]), "M": fs.Found(True, [], ["l.rs"])}
        with contextlib.redirect_stdout(io.StringIO()) as o:
            breaks, moves = fs.report(results, 2, 0, "o", ["N"])
        self.assertEqual((breaks, moves), (["B"], ["M"]))
        self.assertIn("1 public repositories have no main branch, not scanned: N", o.getvalue())
        self.assertIn("private repositories of o are not scanned", o.getvalue())

    def test_nothing_scanned_fails_on_its_own(self):
        # repositories listed, none with a main: an input error, not an empty comparison
        real = (fs.list_repos, fs.scan)
        self.addCleanup(lambda: (setattr(fs, "list_repos", real[0]), setattr(fs, "scan", real[1])))
        fs.list_repos = lambda owner: [fs.Repo("A", "a")]
        fs.scan = lambda repos, cache: ({}, 0, 0, ["A"])
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as e:
            self.assertEqual(fs.main(["check"]), 2)
        self.assertIn("0 repositories scanned", e.getvalue())


class Compare(unittest.TestCase):
    def test_differences_both_ways_and_none(self):
        self.assertEqual(fs.compare(["A"], ["M"], ["A"], ["M"]), [])
        bad = fs.compare(["A", "B"], ["M"], ["A", "C"], [])
        self.assertIn(f"B breaks at its main but is not in {fs.CONSUMERS}", bad)
        self.assertIn(f"C is in {fs.CONSUMERS} but does not break at its main", bad)
        self.assertIn(f"M moves at its main but is not in {fs.MOVERS}", bad)
        self.assertEqual(len(bad), 3)

    def test_no_repository_listed_is_an_input_error(self):
        real = fs.list_repos
        fs.list_repos = lambda owner: []
        self.addCleanup(setattr, fs, "list_repos", real)
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as e:
            self.assertEqual(fs.main(["check"]), 2)
        self.assertIn("0 scanned", e.getvalue())

    def test_the_committed_lists_match_a_scan_with_the_same_results(self):
        # the lists in this repository against results built to match them, through main(): a
        # missing entry and an extra one each fail with exit 1
        root = Path(tempfile.mkdtemp(prefix="fold-lists-"))
        self.addCleanup(shutil.rmtree, root, True)
        (root / ".github").mkdir()
        (root / fs.CONSUMERS).write_text("ALICE-A\n", encoding="utf-8")
        (root / fs.MOVERS).write_text("# none\nALICE-M\n", encoding="utf-8")
        real = (fs.list_repos, fs.scan)
        self.addCleanup(lambda: (setattr(fs, "list_repos", real[0]), setattr(fs, "scan", real[1])))
        fs.list_repos = lambda owner: [fs.Repo("ALICE-A", "a"), fs.Repo("ALICE-M", "m")]
        for found, code in (({"ALICE-A": fs.Found(True, ["s.rs"], []), "ALICE-M": fs.Found(True, [], ["l.rs"])}, 0),
                            ({"ALICE-A": fs.Found(True, [], ["s.rs"]), "ALICE-M": fs.Found(True, [], ["l.rs"])}, 1),
                            ({"ALICE-A": fs.Found(True, ["s.rs"], []), "ALICE-M": fs.Found(False, [], [])}, 1)):
            fs.scan = lambda repos, cache, found=found: (found, len(found), 0, [])
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(fs.main(["check", "--root", str(root)]), code, found)


if __name__ == "__main__":
    unittest.main()
