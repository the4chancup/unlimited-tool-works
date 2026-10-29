"""Tests of scripts/make_icons.py; run as `python scripts/make_icons_test.py` (needs what the
script needs: fontTools)."""

import unittest
from unittest import mock

import make_icons


class Build(unittest.TestCase):
    def test_the_committed_fonts_are_the_build_of_the_committed_svgs(self) -> None:
        data = make_icons.build()
        for output in make_icons.OUTPUTS:
            with self.subTest(output=output.name, folder=output.parent.name):
                self.assertTrue(output.is_file() and output.read_bytes() == data,
                                f"{output} differs from a rebuild: run scripts/make_icons.py")


class Errors(unittest.TestCase):
    def test_two_icons_at_one_codepoint_are_refused(self) -> None:
        help_codepoint = make_icons.CODEPOINTS["help"]
        with mock.patch.dict(make_icons.CODEPOINTS, {"settings": help_codepoint}):
            with self.assertRaisesRegex(make_icons.IconError,
                                        "help.svg and settings.svg: both at U\\+E3E8"):
                make_icons.build()


if __name__ == "__main__":
    unittest.main()
