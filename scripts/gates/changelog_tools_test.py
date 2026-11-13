"""Tests for `changelog_tools.py`: python scripts/gates/temp_alias.py scripts/gates/changelog_tools_test.py"""

import unittest

import changelog_tools

CHANGELOG = """# Changelog

## [Unreleased]

### New

- **Team compiler and Ruleset editor**: a texture can be a JPEG XL file.
- **Save editor**: the Team compiler is named in this entry's text, not its label.
- **Updates**: the dialog has an Update now button, which the
  **Export upgrader** does not own: a bold word inside an entry is no label.

### Minor fixes

- **export upgrader**: the progress line counts one upgrade as one export.

## [0.0.1] - 2026-11-03

- **Team compiler**: an entry a release already carries is history.
"""


class UnpresentedEntries(unittest.TestCase):
    def test_an_unreleased_entry_labelled_with_an_unpresented_tool_is_listed(self):
        self.assertEqual(
            changelog_tools.unpresented_entries(CHANGELOG),
            [(7, "Team compiler and Ruleset editor"), (14, "export upgrader")],
        )

    def test_a_changelog_without_such_entries_lists_nothing(self):
        clean = CHANGELOG.replace("Team compiler and Ruleset editor", "Ruleset editor").replace(
            "**export upgrader**", "**Help**"
        )
        self.assertEqual(changelog_tools.unpresented_entries(clean), [])


if __name__ == "__main__":
    unittest.main()
