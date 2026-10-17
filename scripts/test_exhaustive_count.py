#!/usr/bin/env python3
"""Tests for scripts/exhaustive_count.py."""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import exhaustive_count as ec  # noqa: E402

A = "0123456789abcdef"
B = "fedcba9876543210"
PINS = {"sin": A, "cos": B}


def line(name: str, digest: str, n: int = 1 << 32) -> str:
    return f"EXHAUSTIVE {name} inputs swept {n} digest {digest}\n"


class Check(unittest.TestCase):
    def test_all_equal_passes(self):
        self.assertEqual(ec.check(line("sin", A) + "." + line("cos", B), PINS), [])

    def test_moved_digest_fails(self):
        # the full count is printed, the digest is not the pin
        self.assertTrue(ec.check(line("sin", B) + line("cos", B), PINS))

    def test_empty_log_fails(self):
        self.assertTrue(ec.check("test result: ok. 0 passed\n", PINS))

    def test_short_count_fails(self):
        self.assertTrue(ec.check(line("sin", A, (1 << 32) - 1) + line("cos", B), PINS))

    def test_missing_function_fails(self):
        self.assertTrue(ec.check(line("sin", A), PINS))

    def test_duplicate_fails(self):
        self.assertTrue(ec.check(line("sin", A) + line("sin", A) + line("cos", B), PINS))

    def test_unpinned_function_fails(self):
        self.assertTrue(ec.check(line("sin", A) + line("cos", B) + line("exp", A), PINS))


class Pins(unittest.TestCase):
    def test_reads_names_and_digests(self):
        self.assertEqual(ec.read_pins(f"# c\n\nsin {A}\ncos {B}\n"), (PINS, []))

    def test_empty_file_fails(self):
        self.assertTrue(ec.read_pins("# only a comment\n")[1])

    def test_malformed_line_fails(self):
        self.assertTrue(ec.read_pins(f"sin {A[:15]}\n")[1])

    def test_repeated_name_fails(self):
        self.assertTrue(ec.read_pins(f"sin {A}\nsin {B}\n")[1])


class Cli(unittest.TestCase):
    def test_exit_codes(self):
        with tempfile.TemporaryDirectory() as d:
            log, pins = Path(d) / "log", Path(d) / "pins"
            pins.write_text(f"sin {A}\n", encoding="utf-8")
            log.write_text(line("sin", A), encoding="utf-8")
            self.assertEqual(ec.main(["x", str(log), str(pins)]), 0)
            log.write_text(line("sin", B), encoding="utf-8")
            self.assertEqual(ec.main(["x", str(log), str(pins)]), 1)
            self.assertEqual(ec.main(["x", str(log)]), 2)

    def test_the_repository_digests_file_reads(self):
        root = Path(__file__).resolve().parent.parent
        pins, errors = ec.read_pins((root / "tests" / "exhaustive_f32.digests")
                                    .read_text(encoding="utf-8"))
        self.assertEqual(errors, [])
        self.assertEqual(len(pins), 12)


if __name__ == "__main__":
    unittest.main()
