"""Tests of `c_crates.py`'s pure helpers, on cargo's output as it printed it on 2026-11-12."""

import unittest

import c_crates

# `cargo tree --workspace -i cc --target x86_64-unknown-linux-gnu -e normal,build --depth 1
# --prefix none`.
LINUX_CC = "cc v1.5.1\nring v0.17.14\nwayland-backend v0.3.17\n"
# The same for a target where nothing depends on the builder: cargo prints a warning on
# stderr and nothing on stdout.
NOTHING = ""


class Dependents(unittest.TestCase):
    def test_the_builders_own_line_is_left_out(self):
        self.assertEqual(c_crates.dependents(LINUX_CC, "cc"), ["ring", "wayland-backend"])

    def test_no_output_is_no_dependent(self):
        self.assertEqual(c_crates.dependents(NOTHING, "cc"), [])

    def test_a_crate_named_like_another_builder_is_still_a_dependent(self):
        # Only the queried builder's own line is dropped.
        self.assertEqual(c_crates.dependents("cmake v0.1.54\ncc v1.5.1\n", "cmake"), ["cc"])


class Unlisted(unittest.TestCase):
    EVERY = {name: {"x86_64-unknown-linux-gnu"} for name in c_crates.EXCEPTIONS}

    def test_the_listed_exceptions_pass(self):
        self.assertEqual(c_crates.unlisted(dict(self.EVERY)), [])

    def test_an_unlisted_crate_is_refused_with_its_targets(self):
        found = dict(self.EVERY)
        found["libz-sys"] = {"x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"}
        (complaint,) = c_crates.unlisted(found)
        self.assertIn("libz-sys compiles C", complaint)
        self.assertIn("x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu", complaint)

    def test_an_exception_nothing_needs_is_refused(self):
        found = dict(self.EVERY)
        del found["ring"]
        self.assertEqual(
            c_crates.unlisted(found),
            ["ring is a listed exception no release target needs: remove it"],
        )


if __name__ == "__main__":
    unittest.main()
