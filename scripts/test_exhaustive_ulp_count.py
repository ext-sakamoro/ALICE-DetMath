#!/usr/bin/env python3
"""Tests of scripts/exhaustive_ulp_count.py: each refusal it promises, and a passing log."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import exhaustive_ulp_count as c  # noqa: E402

N = c.INPUTS
OK = f"EXHAUSTIVE_ULP asin inputs {N} max 1 flagged 6\nEXHAUSTIVE_ULP acos inputs {N} max 0 flagged 6\n"


class T(unittest.TestCase):
    def test_passes(self):
        self.assertEqual(c.check(OK), [])

    def test_empty_log_fails(self):
        self.assertTrue(c.check("running 2 tests\n"))

    def test_missing_function_fails(self):
        self.assertTrue(c.check(f"EXHAUSTIVE_ULP asin inputs {N} max 0 flagged 0\n"))

    def test_wrong_count_fails(self):
        self.assertTrue(c.check(OK.replace(f"asin inputs {N}", f"asin inputs {N - 1}")))

    def test_over_bound_fails(self):
        self.assertTrue(c.check(OK.replace("acos inputs", "acos inputs").replace("max 0", "max 2")))

    def test_repeat_fails(self):
        self.assertTrue(c.check(OK + f"EXHAUSTIVE_ULP asin inputs {N} max 0 flagged 0\n"))

    def test_unknown_function_fails(self):
        self.assertTrue(c.check(OK + f"EXHAUSTIVE_ULP atan inputs {N} max 0 flagged 0\n"))


if __name__ == "__main__":
    unittest.main()
