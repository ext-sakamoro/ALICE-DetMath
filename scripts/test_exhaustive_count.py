#!/usr/bin/env python3
"""Tests for scripts/exhaustive_count.py."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import exhaustive_count as ec  # noqa: E402

D = "0123456789abcdef"


def line(name: str, n: int = 1 << 32) -> str:
    return f"EXHAUSTIVE {name} inputs swept {n} digest {D}\n"


class Check(unittest.TestCase):
    def test_all_present_passes(self):
        self.assertEqual(ec.check(line("sin") + "." + line("cos"), ["sin", "cos"]), [])

    def test_empty_log_fails(self):
        self.assertTrue(ec.check("test result: ok. 0 passed\n", ["sin"]))

    def test_short_count_fails(self):
        self.assertTrue(ec.check(line("sin", (1 << 32) - 1), ["sin"]))

    def test_missing_function_fails(self):
        self.assertTrue(ec.check(line("sin"), ["sin", "cos"]))

    def test_duplicate_fails(self):
        self.assertTrue(ec.check(line("sin") + line("sin"), ["sin"]))

    def test_unexpected_function_fails(self):
        self.assertTrue(ec.check(line("sin") + line("exp"), ["sin"]))

    def test_cli_exit_codes(self):
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "log"
            p.write_text(line("sin"), encoding="utf-8")
            self.assertEqual(ec.main(["x", str(p), "sin"]), 0)
            self.assertEqual(ec.main(["x", str(p), "sin,cos"]), 1)
            self.assertEqual(ec.main(["x", str(p), ""]), 2)


if __name__ == "__main__":
    unittest.main()
