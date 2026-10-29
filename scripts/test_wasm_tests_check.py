#!/usr/bin/env python3
"""Tests of wasm_tests_check.py on synthetic cargo logs and metadata."""
from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import wasm_tests_check as w  # noqa: E402

META = {"packages": [{"manifest_path": "/p/Cargo.toml", "targets": [
    {"name": "alice_det_math", "kind": ["lib"], "src_path": "/p/src/lib.rs"},
    {"name": "golden", "kind": ["test"], "src_path": "/p/tests/golden.rs"},
    {"name": "law_bits", "kind": ["test"], "src_path": "/p/tests/law_bits.rs"},
    {"name": "bench_x", "kind": ["bench"], "src_path": "/p/benches/x.rs"},
]}]}


def log(*blocks: str) -> str:
    return "\n".join(blocks) + "\n"


def run(name: str, body: str, passed: int = 3, status: str = "ok", ignored: int = 0) -> str:
    kind = "unittests " if name == "src/lib.rs" else ""
    return (f"\x1b[1m\x1b[92m     Running\x1b[0m {kind}{name} (target/wasm32-wasip1/debug/deps/x-0.wasm)\n"
            f"{body}test result: {status}. {passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out\n")


SKIP = "test refuses_a_nan - should panic ... ignored\n"
ALL_OK = log(run("src/lib.rs", ""), run("tests/golden.rs", ""), run("tests/law_bits.rs", SKIP, ignored=1))
PANIC_LISTED = '[should_panic]\n"law_bits::refuses_a_nan" = "panic=abort on wasm"\n'


class Check(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.d = Path(self._tmp.name)
        (self.d / "meta.json").write_text(json.dumps(META), encoding="utf-8")

    def check(self, text: str, excl: str = "") -> tuple[int, str]:
        (self.d / "log.txt").write_text(text, encoding="utf-8")
        (self.d / "ex.toml").write_text(excl, encoding="utf-8")
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(out):
            rc = w.main([str(self.d / "log.txt"), "--metadata", str(self.d / "meta.json"),
                         "--exclusions", str(self.d / "ex.toml")])
        return rc, out.getvalue()

    def test_every_target_ran_and_the_skip_is_listed(self):
        rc, out = self.check(ALL_OK, PANIC_LISTED)
        self.assertEqual(rc, 0, out)
        self.assertIn("3 targets, 3 ran on wasm, 0 excluded, 1 should_panic skipped (1 listed), 0 finding(s)", out)

    def test_a_target_that_did_not_run_fails(self):
        rc, out = self.check(log(run("src/lib.rs", ""), run("tests/law_bits.rs", SKIP)), PANIC_LISTED)
        self.assertEqual(rc, 1, out)
        self.assertIn("golden (tests/golden.rs) did not run on wasm and is not excluded", out)

    def test_an_excluded_target_needs_no_run_but_a_reason(self):
        excl = PANIC_LISTED + '[targets]\ngolden = "reads the host filesystem"\n'
        rc, out = self.check(log(run("src/lib.rs", ""), run("tests/law_bits.rs", SKIP)), excl)
        self.assertEqual(rc, 0, out)
        rc, out = self.check(log(run("src/lib.rs", ""), run("tests/law_bits.rs", SKIP)),
                             PANIC_LISTED + '[targets]\ngolden = " "\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("exclusion 'golden' has no reason", out)

    def test_an_excluded_target_that_ran_fails(self):
        rc, out = self.check(ALL_OK, PANIC_LISTED + '[targets]\ngolden = "x"\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("golden is excluded but ran", out)

    def test_an_exclusion_naming_no_target_fails(self):
        rc, out = self.check(ALL_OK, PANIC_LISTED + '[targets]\nnope = "x"\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("[targets] 'nope' names no test target", out)

    def test_an_unlisted_should_panic_skip_fails(self):
        rc, out = self.check(ALL_OK, "")
        self.assertEqual(rc, 1, out)
        self.assertIn("law_bits::refuses_a_nan was skipped as should_panic", out)

    def test_a_listed_skip_that_did_not_happen_fails(self):
        rc, out = self.check(ALL_OK, PANIC_LISTED + '"golden::gone" = "panic=abort on wasm"\n')
        self.assertEqual(rc, 1, out)
        self.assertIn("[should_panic] 'golden::gone' was not skipped in this run", out)

    def test_a_failed_or_empty_target_fails(self):
        rc, out = self.check(log(run("src/lib.rs", ""), run("tests/golden.rs", "", status="FAILED"),
                                 run("tests/law_bits.rs", SKIP)), PANIC_LISTED)
        self.assertEqual(rc, 1, out)
        self.assertIn("golden (tests/golden.rs): test result FAILED", out)
        rc, out = self.check(log(run("src/lib.rs", ""), run("tests/golden.rs", "", passed=0),
                                 run("tests/law_bits.rs", SKIP)), PANIC_LISTED)
        self.assertEqual(rc, 1, out)
        self.assertIn("golden (tests/golden.rs): 0 tests passed", out)

    def test_nothing_ran_is_exit_2(self):
        rc, out = self.check("no cargo output here\n", "")
        self.assertEqual(rc, 2, out)
        self.assertIn("no target ran", out)

    def test_bench_targets_are_not_required(self):
        self.assertEqual(sorted(w.targets_from_metadata(META).values()), ["alice_det_math", "golden", "law_bits"])

    def test_the_committed_exclusions_read(self):
        t = w.tomllib.loads(w.EXCLUSIONS.read_text(encoding="utf-8"))
        for table in ("targets", "should_panic"):
            for k, v in t.get(table, {}).items():
                self.assertTrue(isinstance(v, str) and v.strip(), k)


if __name__ == "__main__":
    unittest.main()
