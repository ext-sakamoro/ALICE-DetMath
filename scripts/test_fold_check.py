#!/usr/bin/env python3
"""Tests of scripts/fold_check.py, end to end on throwaway crates in throwaway git repositories
(built with cargo): a stand-in alice-det-math (the candidate and a published one), consumers that
fold its SEMANTICS_ID with a const-eval panic on a missing row, and the wave file's rules.

oracle: the expected result of each case is fixed by how the fixture is built (the row is or is
not in the consumer's table; the dependency does or does not reach det-math by a source the check
patches), never by running the check first."""
from __future__ import annotations

import contextlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fold_check as fc  # noqa: E402

CAND_ID = 0xC7   # the candidate's SEMANTICS_ID: [0xC7; 32]
OLD_ID = 0x61    # the published det-math's: [0x61; 32]
GIT_ENV = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t", "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}


def git(d: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=d, check=True, capture_output=True, text=True,
                          env={**os.environ, **GIT_ENV}).stdout.strip()


def repo(d: Path, files: dict[str, str]) -> str:
    d.mkdir(parents=True, exist_ok=True)
    if not (d / ".git").exists():
        git(d, "init", "-q", "-b", "main")
        git(d, "config", "uploadpack.allowAnySHA1InWant", "true")
    for k, v in files.items():
        (d / k).parent.mkdir(parents=True, exist_ok=True)
        (d / k).write_text(v, encoding="utf-8")
    git(d, "add", "-A")
    git(d, "commit", "-q", "-m", "c")
    return git(d, "rev-parse", "HEAD")


def det_math(version: str, byte: int) -> dict[str, str]:
    return {"Cargo.toml": f'[package]\nname = "alice-det-math"\nversion = "{version}"\nedition = "2021"\n[workspace]\n'
                          '[features]\ndefault = []\nstd = []\n',
            "src/lib.rs": f"pub const SEMANTICS_ID: [u8; 32] = [{byte:#04x}; 32];\n"}


def consumer_lib(rows: list[int]) -> str:
    table = ", ".join(f"([{r:#04x}; 32], [0x11; 32])" for r in rows)
    return ("pub const FAKE_SEMANTICS_BY_DET_MATH: &[([u8; 32], [u8; 32])] = &[" + table + "];\n"
            "const fn fold(id: &[u8; 32]) -> [u8; 32] {\n"
            "    let mut i = 0;\n"
            "    while i < FAKE_SEMANTICS_BY_DET_MATH.len() {\n"
            "        let mut j = 0; let mut same = true;\n"
            "        while j < 32 { if FAKE_SEMANTICS_BY_DET_MATH[i].0[j] != id[j] { same = false; } j += 1; }\n"
            "        if same { return FAKE_SEMANTICS_BY_DET_MATH[i].1; }\n"
            "        i += 1;\n"
            "    }\n"
            "    panic!(\"alice-det-math SEMANTICS_ID has no row in FAKE_SEMANTICS_BY_DET_MATH: add its fold\");\n"
            "}\n"
            "pub const FAKE_SEMANTICS_ID: [u8; 32] = fold(&alice_det_math::SEMANTICS_ID);\n")


class World:
    """a directory of git repositories reached as file://<dir>/<name>, plus the candidate checkout"""

    def __init__(self, tc: unittest.TestCase):
        self.dir = Path(tempfile.mkdtemp(prefix="fold-test-"))
        tc.addCleanup(shutil.rmtree, self.dir, True)
        self.base = f"file://{self.dir}"
        self.published = repo(self.dir / "ALICE-DetMath", det_math("0.6.1", OLD_ID))
        self.dm_url = f"{self.base}/ALICE-DetMath"
        self.candidate = self.dir / "candidate"
        repo(self.candidate, det_math("0.7.0", CAND_ID))

    def dep_line(self, url: str | None = None, rev: str | None = None) -> str:
        return (f'alice-det-math = {{ git = "{url or self.dm_url}", rev = "{rev or self.published}", '
                'version = "0.6.1", default-features = false }\n')

    def consumer(self, name: str, rows: list[int], extra_deps: str = "", files: dict[str, str] | None = None) -> str:
        return repo(self.dir / name, {
            "Cargo.toml": f'[package]\nname = "{name.lower()}"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n'
                          f'[dependencies]\n{self.dep_line()}{extra_deps}',
            "src/lib.rs": consumer_lib(rows), **(files or {})})

    def check(self, name: str, entry: fc.Entry, runs=None) -> dict:
        work = Path(tempfile.mkdtemp(prefix="fold-work-", dir=self.dir))
        return fc.check_consumer(name, entry, self.candidate, work, self.base, "owner",
                                 check_runs=(lambda o, r, s: runs) if runs is not None else _no_gh)


def _no_gh(owner, repo, sha):
    raise AssertionError("CI is read only for a wave branch")


class Consumer(unittest.TestCase):
    def setUp(self):
        self.w = World(self)

    def test_a_consumer_with_the_row_passes(self):
        self.w.consumer("ALICE-Fake", [OLD_ID, CAND_ID])
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "ok", r)
        self.assertIn("1 resolved to this checkout", r["detail"])

    def test_a_consumer_without_the_row_is_red_and_named(self):
        self.w.consumer("ALICE-Fake", [OLD_ID])
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "missing-row", r)
        self.assertIn("FAKE_SEMANTICS_BY_DET_MATH has no row", r["detail"])

    def test_a_recorded_generation_that_does_not_match_is_red(self):
        # the other compile-time form: a const assert of the det-math identifier the crate recorded
        for recorded, want in ((CAND_ID, "ok"), (OLD_ID, "missing-row")):
            with self.subTest(recorded=hex(recorded)):
                name = f"ALICE-Gen{recorded:02x}"
                self.w.consumer(name, [OLD_ID, CAND_ID], files={"src/stamp.rs": "", "src/lib.rs": consumer_lib([OLD_ID, CAND_ID])
                    + f"const GENERATION: [u8; 32] = [{recorded:#04x}; 32];\n"
                    "const fn same(a: &[u8; 32], b: &[u8; 32]) -> bool { let mut i = 0; while i < 32 { if a[i] != b[i] { return false; } i += 1; } true }\n"
                    "const _: () = assert!(same(&alice_det_math::SEMANTICS_ID, &GENERATION),\n"
                    "    \"alice-det-math generation changed: re-record FAKE_SEMANTICS_ID and GENERATION\");\n"})
                r = self.w.check(name, fc.Entry("main", None))
                self.assertEqual(r["result"], want, r)
                if want != "ok":
                    self.assertIn("recorded det-math generation", r["detail"])

    def test_a_det_math_the_patch_does_not_reach_is_red(self):
        # the consumer reaches det-math also through a git dependency outside the fetched tree,
        # by another URL: that copy is not patched, the resolve holds the published det-math too,
        # and a build would compile against it (with its row) and pass
        other = repo(self.w.dir / "Other-DetMath", det_math("0.6.1", OLD_ID))
        helper = repo(self.w.dir / "helper", {
            "Cargo.toml": '[package]\nname = "helper"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n[dependencies]\n'
                          + self.w.dep_line(f"{self.w.base}/Other-DetMath", other),
            "src/lib.rs": "pub use alice_det_math::SEMANTICS_ID;\n"})
        self.w.consumer("ALICE-Fake", [OLD_ID, CAND_ID],
                        extra_deps=f'helper = {{ git = "{self.w.base}/helper", rev = "{helper}" }}\n')
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "patch-ignored", r)
        self.assertIn("Other-DetMath", r["detail"])
        # who pulls the other copy, with its requirement: the package to move in the wave
        self.assertIn("via helper 0.1.0 (alice-det-math ^0.6.1)", r["detail"])

    def test_siblings_pinned_by_the_consumer_are_fetched_beside_it(self):
        sib = repo(self.w.dir / "ALICE-Sib", {
            "Cargo.toml": '[package]\nname = "sib"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\n' + self.w.dep_line(),
            "src/lib.rs": "pub const S: [u8; 32] = alice_det_math::SEMANTICS_ID;\n"})
        self.w.consumer("ALICE-Fake", [CAND_ID], extra_deps='sib = { path = "../ALICE-Sib" }\n',
                        files={".github/alice-siblings": f"# pins\nALICE-Sib {sib}\n"})
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "ok", r)
        self.assertIn("2 requirement(s)", r["detail"])

    def test_a_wave_branch_at_its_pinned_tip_with_green_ci_is_built_instead_of_main(self):
        self.w.consumer("ALICE-Fake", [OLD_ID])            # main: no row
        d = self.w.dir / "ALICE-Fake"
        git(d, "checkout", "-q", "-b", "ci/wave")
        tip = repo(d, {"src/lib.rs": consumer_lib([OLD_ID, CAND_ID])})
        git(d, "checkout", "-q", "main")
        r = self.w.check("ALICE-Fake", fc.Entry("ci/wave", tip), runs=[("test", "completed", "success")])
        self.assertEqual((r["result"], r["sha"]), ("ok", tip), r)

    def test_a_stale_wave_sha_is_red(self):
        self.w.consumer("ALICE-Fake", [OLD_ID])
        d = self.w.dir / "ALICE-Fake"
        git(d, "checkout", "-q", "-b", "ci/wave")
        pinned = repo(d, {"src/lib.rs": consumer_lib([OLD_ID, CAND_ID])})
        repo(d, {"README.md": "moved on\n"})
        r = self.w.check("ALICE-Fake", fc.Entry("ci/wave", pinned), runs=[("test", "completed", "success")])
        self.assertEqual(r["result"], "stale-sha", r)

    def test_a_wave_branch_without_green_ci_is_red(self):
        self.w.consumer("ALICE-Fake", [OLD_ID])
        d = self.w.dir / "ALICE-Fake"
        git(d, "checkout", "-q", "-b", "ci/wave")
        tip = repo(d, {"src/lib.rs": consumer_lib([OLD_ID, CAND_ID])})
        for runs in ([], [("test", "completed", "failure")], [("test", "in_progress", "")],
                     [("a", "completed", "success"), ("b", "completed", "cancelled")]):
            with self.subTest(runs=runs):
                r = self.w.check("ALICE-Fake", fc.Entry("ci/wave", tip), runs=runs)
                self.assertEqual(r["result"], "ci-not-green", r)

    def test_a_requirement_that_resolves_to_nothing_is_red(self):
        # an optional dependency that no default feature enables: the requirement is rewritten, but
        # the resolve has no alice-det-math, so the build would compare nothing
        repo(self.w.dir / "ALICE-Fake", {
            "Cargo.toml": '[package]\nname = "f"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n'
                          '[features]\nlaw = ["dep:alice-det-math"]\n[dependencies]\n'
                          + self.w.dep_line().replace("default-features = false }", "default-features = false, optional = true }"),
            "src/lib.rs": ""})
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "nothing-resolved", r)

    def test_a_consumer_without_a_det_math_requirement_is_red(self):
        repo(self.w.dir / "ALICE-Fake", {
            "Cargo.toml": '[package]\nname = "f"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n',
            "src/lib.rs": ""})
        r = self.w.check("ALICE-Fake", fc.Entry("main", None))
        self.assertEqual(r["result"], "no-dependency", r)


class Rewrite(unittest.TestCase):
    """the requirement forms rewrite_versions handles, the ones it refuses, and the sources it patches"""

    def setUp(self):
        self.d = Path(tempfile.mkdtemp(prefix="fold-rw-"))
        self.addCleanup(shutil.rmtree, self.d, True)

    def manifest(self, deps: str, rel: str = "Cargo.toml") -> Path:
        p = self.d / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text('[package]\nname = "x"\nversion = "0.1.0"\n' + deps, encoding="utf-8")
        return p

    def test_each_requirement_form_is_set_to_the_candidate(self):
        self.manifest('[dependencies]\nalice-det-math = "0.5.1"\n')
        self.manifest('[dependencies]\nalice-det-math = { git = "https://e/x", rev = "abc", version = "0.5.2", default-features = false }\n', "a/Cargo.toml")
        self.manifest('[dependencies]\nalice-det-math = { git = "https://e/y", rev = "abc" }\n', "b/Cargo.toml")
        self.manifest('[dependencies]\nalice-det-math = { workspace = true }\n', "c/Cargo.toml")
        n, sources = fc.rewrite_versions(self.d, "0.7.0")
        self.assertEqual(n, 3)   # the workspace = true entry is not a requirement of its own
        self.assertEqual(sources, {"crates-io", "https://e/x", "https://e/y"})
        self.assertIn('alice-det-math = "=0.7.0"', (self.d / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn('version = "=0.7.0"', (self.d / "a/Cargo.toml").read_text(encoding="utf-8"))

    def test_a_path_dependency_and_an_unhandled_form_are_input_errors(self):
        self.manifest('[dependencies]\nalice-det-math = { path = "../d", version = "0.6" }\n')
        with self.assertRaisesRegex(fc.InputError, "by path"):
            fc.rewrite_versions(self.d, "0.7.0")
        self.manifest('[dependencies.alice-det-math]\nversion = "0.6"\n')
        with self.assertRaisesRegex(fc.InputError, "does not handle"):
            fc.rewrite_versions(self.d, "0.7.0")

    def test_crates_io_is_always_patched_with_each_git_source(self):
        cfg = fc.patch_config({"https://e/x"}, Path("/c"))
        keys = [c.split(".alice-det-math.path=")[0] for c in cfg if c != "--config"]
        self.assertEqual(sorted(keys), ['patch."https://e/x"', "patch.crates-io"])
        self.assertEqual(len([c for c in fc.patch_config(set(), Path("/c")) if c != "--config"]), 1)


class Inputs(unittest.TestCase):
    def setUp(self):
        self.d = Path(tempfile.mkdtemp(prefix="fold-in-"))
        self.addCleanup(shutil.rmtree, self.d, True)

    def write(self, name: str, text: str) -> Path:
        p = self.d / name
        p.write_text(text, encoding="utf-8")
        return p

    def test_an_empty_list_is_an_error(self):
        with self.assertRaisesRegex(fc.InputError, "0 consumers"):
            fc.read_consumers(self.write("c.txt", "# none\n\n"))

    def test_a_listed_crate_without_an_entry_and_an_unlisted_entry_are_errors(self):
        w = self.write("w.toml", '[consumers]\nALICE-A = "main"\nALICE-C = "main"\n')
        with self.assertRaises(fc.InputError) as cm:
            fc.read_wave(w, ["ALICE-A", "ALICE-B"])
        self.assertIn("no entry for ALICE-B", str(cm.exception))
        self.assertIn("not in .github/fold-consumers.txt: ALICE-C", str(cm.exception))
        # each rule on its own: a missing entry with no extra one, an extra one with none missing
        with self.assertRaisesRegex(fc.InputError, "no entry for ALICE-B"):
            fc.read_wave(self.write("w.toml", '[consumers]\nALICE-A = "main"\n'), ["ALICE-A", "ALICE-B"])
        with self.assertRaisesRegex(fc.InputError, "not in .github/fold-consumers.txt: ALICE-C"):
            fc.read_wave(self.write("w.toml", '[consumers]\nALICE-A = "main"\nALICE-C = "main"\n'), ["ALICE-A"])

    def test_a_wave_entry_needs_a_ci_branch_and_a_full_sha(self):
        for bad in ('{ branch = "ci/x", sha = "abc123" }', '{ branch = "feature/x", sha = "' + "a" * 40 + '" }',
                    '{ branch = "ci/x" }', '"ci/x"'):
            with self.subTest(entry=bad):
                w = self.write("w.toml", f"[consumers]\nALICE-A = {bad}\n")
                with self.assertRaises(fc.InputError):
                    fc.read_wave(w, ["ALICE-A"])
        w = self.write("w.toml", '[consumers]\nALICE-A = { branch = "ci/x", sha = "' + "a" * 40 + '" }\n')
        self.assertEqual(fc.read_wave(w, ["ALICE-A"])["ALICE-A"], fc.Entry("ci/x", "a" * 40))

    def tally(self, consumers: str, results: dict[str, str]) -> int:
        root = self.d / "root"
        (root / ".github").mkdir(parents=True, exist_ok=True)
        (root / fc.CONSUMERS).write_text(consumers, encoding="utf-8")
        res = self.d / "results"
        shutil.rmtree(res, True)
        res.mkdir()
        for n, r in results.items():
            (res / f"{n}.json").write_text(json.dumps({"name": n, "result": r}), encoding="utf-8")
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            return fc.main(["--candidate", str(root), "tally", "--results", str(res)])

    def test_tally_needs_one_passing_result_per_listed_crate(self):
        self.assertEqual(self.tally("ALICE-A\nALICE-B\n", {"ALICE-A": "ok", "ALICE-B": "ok"}), 0)
        self.assertEqual(self.tally("ALICE-A\nALICE-B\n", {"ALICE-A": "ok"}), 1)
        self.assertEqual(self.tally("ALICE-A\nALICE-B\n", {"ALICE-A": "ok", "ALICE-B": "missing-row"}), 1)
        self.assertEqual(self.tally("ALICE-A\n", {}), 1)
        self.assertEqual(self.tally("# none\n", {}), 2)   # 0 consumers: an input error


class Ids(unittest.TestCase):
    def test_moved_and_unmoved_identifiers_are_read_and_written(self):
        d = Path(tempfile.mkdtemp(prefix="fold-ids-test-"))
        self.addCleanup(shutil.rmtree, d, True)
        first = repo(d / "dm", det_math("0.6.1", OLD_ID))
        repo(d / "dm", {"README.md": "docs\n"})
        out = d / "out"
        with contextlib.redirect_stdout(io.StringIO()) as s:
            self.assertEqual(fc.main(["--candidate", str(d / "dm"), "ids", "--base", first, "--github-output", str(out)]), 0)
        self.assertIn("2 identifiers read, moved: no", s.getvalue())
        repo(d / "dm", det_math("0.7.0", CAND_ID))
        with contextlib.redirect_stdout(io.StringIO()) as s:
            self.assertEqual(fc.main(["--candidate", str(d / "dm"), "ids", "--base", first, "--github-output", str(out)]), 0)
        self.assertIn(f"SEMANTICS_ID at {first}: {'61' * 32}", s.getvalue())
        self.assertIn(f"SEMANTICS_ID here: {'c7' * 32}", s.getvalue())
        self.assertIn("moved: yes", s.getvalue())
        self.assertEqual(out.read_text(encoding="utf-8").splitlines(),
                         ["moved=false", f"id={'61' * 32}", "moved=true", f"id={'c7' * 32}"])

    def test_an_unreadable_identifier_is_an_input_error(self):
        d = Path(tempfile.mkdtemp(prefix="fold-ids-test-"))
        self.addCleanup(shutil.rmtree, d, True)
        first = repo(d / "dm", {"Cargo.toml": '[package]\nname = "alice-det-math"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n',
                                "src/lib.rs": "// no SEMANTICS_ID\n"})
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as e:
            self.assertEqual(fc.main(["--candidate", str(d / "dm"), "ids", "--base", first]), 2)
        self.assertIn("could not be read", e.getvalue())


if __name__ == "__main__":
    unittest.main()
