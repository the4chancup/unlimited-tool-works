"""Tests for `changelog_tools.py`: python scripts/gates/temp_alias.py scripts/gates/changelog_tools_test.py"""

import unittest
from unittest import mock

import changelog_tools

CHANGELOG = """# Changelog

## [Unreleased]

### New

- **The Compiler and Ruleset editor**: a texture can be a JPEG XL file.
- **Save editor**: the Compiler is named in this entry's text, not its label.
- **Updates**: the dialog has an Update now button, which the
  **Export upgrader** does not own: a bold word inside an entry is no label.

### Minor fixes

- **export upgrader**: the progress line counts one upgrade as one export.

## [0.0.1] - 2026-11-03

- **The Compiler**: an entry a release already carries is history.
"""


@mock.patch.object(changelog_tools, "UNPRESENTED", ("The Compiler", "Export upgrader"))
class UnpresentedEntries(unittest.TestCase):
    def test_an_unreleased_entry_labelled_with_an_unpresented_tool_is_listed(self):
        self.assertEqual(
            changelog_tools.unpresented_entries(CHANGELOG),
            [(7, "The Compiler and Ruleset editor"), (14, "export upgrader")],
        )

    def test_a_changelog_without_such_entries_lists_nothing(self):
        clean = CHANGELOG.replace("The Compiler and Ruleset editor", "Ruleset editor").replace(
            "**export upgrader**", "**Help**"
        )
        self.assertEqual(changelog_tools.unpresented_entries(clean), [])


if __name__ == "__main__":
    unittest.main()
