"""Tests of scripts/gates/checkpoint.py's commit classification; `just test` runs them."""

import unittest

from checkpoint import docs_only


class DocsOnly(unittest.TestCase):
    def test_docs_and_root_markdown_are_docs_only(self):
        self.assertTrue(docs_only(["docs/WORKLOG.md", "docs/spec/core/gui.md", "README.md", "CHANGELOG.md"]))

    def test_any_other_path_makes_a_code_commit(self):
        for path in ["crates/libs/cpk/src/lib.rs", "scripts/gates/checkpoint.py", "justfile", "Cargo.lock",
                     "crates/tools/team_compiler/help/01_command_line.md", "resources/bundle/README.txt"]:
            self.assertFalse(docs_only(["docs/WORKLOG.md", path]), path)

    def test_a_commit_touching_nothing_is_not_docs_only(self):
        self.assertFalse(docs_only([]))


if __name__ == "__main__":
    unittest.main()
