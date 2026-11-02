"""Tests for line_endings.py: which `git ls-files --eol` lines it lists."""

import unittest

from line_endings import wrong_endings


class WrongEndingsTest(unittest.TestCase):
    def test_a_crlf_or_mixed_copy_of_an_lf_file_is_listed(self):
        listing = "\n".join(
            [
                "i/lf    w/crlf  attr/text=auto eol=lf \tscripts/built_window/gui.py",
                "i/lf    w/mixed attr/text=auto eol=lf \tcrates/studio/src/gui_log.rs",
            ]
        )
        self.assertEqual(
            wrong_endings(listing),
            [
                ("scripts/built_window/gui.py", "crlf", "lf"),
                ("crates/studio/src/gui_log.rs", "mixed", "lf"),
            ],
        )

    def test_an_lf_copy_of_a_crlf_file_is_listed(self):
        listing = "i/lf    w/lf    attr/text eol=crlf    \tresources/bundle/quick_compile.bat"
        self.assertEqual(
            wrong_endings(listing), [("resources/bundle/quick_compile.bat", "lf", "crlf")]
        )

    def test_copies_that_match_their_attributes_pass(self):
        listing = "\n".join(
            [
                "i/lf    w/lf    attr/text=auto eol=lf \tcrates/studio/src/main.rs",
                "i/lf    w/crlf  attr/text eol=crlf    \tresources/bundle/quick_upgrade.bat",
                "i/none  w/none  attr/text=auto eol=lf \tcrates/libs/cpk/tests/empty.txt",
            ]
        )
        self.assertEqual(wrong_endings(listing), [])

    def test_a_binary_file_asks_nothing(self):
        listing = "\n".join(
            [
                "i/crlf  w/crlf  attr/-text            \tcrates/libs/teams_list/data/teams_list.txt",
                "i/-text w/-text attr/-text            \tcrates/libs/cpk/tests/fixtures/a.cpk",
            ]
        )
        self.assertEqual(wrong_endings(listing), [])

    def test_a_path_with_spaces_is_kept_whole(self):
        listing = "i/lf    w/crlf  attr/text=auto eol=lf \tcrates/a b/c d.txt"
        self.assertEqual(wrong_endings(listing), [("crates/a b/c d.txt", "crlf", "lf")])


if __name__ == "__main__":
    unittest.main()
