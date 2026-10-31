"""Tests of scripts/doc_pointers.py; `just doc-pointers` runs them before the check."""

import tempfile
import unittest
from pathlib import Path

from doc_pointers import anchors, broken_links, names_anchor, pointers, problems

PLAN = """# Team compiler

## Pipeline

### 5. Writer (single-threaded)

**Subfolders.** A subfolder is a player folder.

Rules:

- **Collars** — a collar model.

| Term | Meaning |
|---|---|
| **ATF** | The task force. |

## 2026-10-27 — core — A window closed maximized reopens maximized

```
## Not a heading inside a fence
```
"""


class Pointers(unittest.TestCase):
    def test_one_title_with_or_without_backticks(self) -> None:
        found = pointers("docs/WORKLOG.md", 'see `a/b.md` "One" and c.md "Two"')
        self.assertEqual([(p.path, p.title) for p in found], [("a/b.md", "One"), ("c.md", "Two")])

    def test_titles_after_comma_and_or(self) -> None:
        found = pointers("x.md", '`b.md` "One", "Two" and "Three" or "Four"')
        self.assertEqual([p.title for p in found], ["One", "Two", "Three", "Four"])

    def test_a_quote_without_a_file_is_no_pointer(self) -> None:
        self.assertEqual(pointers("x.md", 'the word "One" alone'), [])

    def test_a_title_wrapped_in_a_comment_is_read_whole(self) -> None:
        text = '//! the bar (`spec/core/gui.md` "The status\n//! bar") stays\nfn main() {}'
        found = pointers("crates/x/src/lib.rs", text)
        self.assertEqual([(p.line, p.path, p.title) for p in found], [(1, "spec/core/gui.md", "The status bar")])

    def test_a_path_ending_a_line_takes_its_titles_from_the_next(self) -> None:
        text = '    /// see `a.md`\n    /// "One", "Two" and\n    /// "Three", then `b.md` "Four"'
        found = pointers("crates/x/src/lib.rs", text)
        self.assertEqual(
            [(p.line, p.path, p.title) for p in found],
            [(1, "a.md", "One"), (1, "a.md", "Two"), (1, "a.md", "Three"), (3, "b.md", "Four")],
        )

    def test_a_wrapped_paragraph_in_a_document(self) -> None:
        found = pointers("docs/WORKLOG.md", 'see `core/gui.md` "The status\n  bar", then')
        self.assertEqual([(p.line, p.title) for p in found], [(1, "The status bar")])

    def test_a_complete_pointer_does_not_take_the_next_line(self) -> None:
        found = pointers("docs/WORKLOG.md", 'see `a.md` "One"\n"Two" is a quote')
        self.assertEqual([p.title for p in found], ["One"])

    def test_no_wrap_across_a_blank_line_or_into_code(self) -> None:
        self.assertEqual(pointers("docs/WORKLOG.md", 'see `a.md` "One\n\ntwo" is a quote'), [])
        self.assertEqual(pointers("crates/x/src/lib.rs", '// see `a.md` "One\nlet s = "two";'), [])


class Anchors(unittest.TestCase):
    def test_every_anchor_form(self) -> None:
        found = anchors(PLAN)
        titles = ["Pipeline", "5. Writer", "Writer", "Subfolders", "Rules", "Collars", "ATF"]
        for title in [*titles, "A window closed maximized reopens maximized"]:
            self.assertTrue(names_anchor(found, title), title)

    def test_a_fenced_heading_is_no_anchor(self) -> None:
        self.assertFalse(names_anchor(anchors(PLAN), "Not a heading inside a fence"))

    def test_a_title_names_an_anchor_by_whole_words(self) -> None:
        found = {"Testing and verification", "Phase 5: Savefile integration"}
        self.assertTrue(names_anchor(found, "Testing"))
        self.assertTrue(names_anchor(found, "Phase 5"))
        self.assertFalse(names_anchor(found, "Test"))
        self.assertFalse(names_anchor(found, "Phase 50"))


class Problems(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "docs" / "plans" / "core").mkdir(parents=True)
        (self.root / "docs" / "spec" / "core").mkdir(parents=True)
        (self.root / "docs" / "plans" / "core" / "gui.md").write_text("## Cancellation\n", encoding="utf-8")
        (self.root / "docs" / "spec" / "core" / "gui.md").write_text("## Grid\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def check(self, source: str, line: str) -> list[str]:
        return problems(self.root, pointers(source, line))

    def test_a_bare_plan_path_resolves_under_plans(self) -> None:
        self.assertEqual(self.check("docs/WORKLOG.md", '`core/gui.md` "Cancellation"'), [])

    def test_a_spec_section_needs_its_tree(self) -> None:
        self.assertEqual(self.check("docs/WORKLOG.md", '`spec/core/gui.md` "Grid"'), [])
        self.assertEqual(
            self.check("docs/WORKLOG.md", '`core/gui.md` "Grid"'),
            ['docs/WORKLOG.md:1: docs/plans/core/gui.md has no "Grid"'],
        )

    def test_a_path_resolves_beside_its_file_first(self) -> None:
        self.assertEqual(self.check("docs/spec/core/README.md", '`gui.md` "Grid"'), [])

    def test_a_missing_file_fails_in_docs_only(self) -> None:
        self.assertEqual(
            self.check("docs/WORKLOG.md", '`gui.md` "Grid"'), ["docs/WORKLOG.md:1: `gui.md` names no file"]
        )
        self.assertEqual(self.check("crates/x/README.md", '`USAGE.md` "Flags"'), [])

    def test_a_pointer_into_the_scratch_folder_fails_where_its_file_exists_too(self) -> None:
        (self.root / ".tmp").mkdir()
        (self.root / ".tmp" / "rulings.md").write_text("## Round A3\n", encoding="utf-8")
        scratch = "docs/WORKLOG.md:1: `.tmp/rulings.md` points into the scratch folder, which no clone has"
        self.assertEqual(self.check("docs/WORKLOG.md", '`.tmp/rulings.md` "Round A3"'), [scratch])
        self.assertEqual(
            self.check("crates/x/src/lib.rs", '// `.tmp/phase_08/8_29/rulings.md` "Round S6c4b"'),
            ["crates/x/src/lib.rs:1: `.tmp/phase_08/8_29/rulings.md` points into the scratch folder, which no clone has"],
        )
        self.assertEqual(self.check("docs/WORKLOG.md", "the rulings in `.tmp/rulings.md`, round A3"), [])

    def test_code_points_at_the_spec_alone(self) -> None:
        self.assertEqual(self.check("crates/x/src/lib.rs", '//! `spec/core/gui.md` "Grid"'), [])
        self.assertEqual(
            self.check("crates/x/src/lib.rs", '//! `docs/plans/core/gui.md` "Cancellation"'),
            ['crates/x/src/lib.rs:1: code points at the plan docs/plans/core/gui.md "Cancellation"'],
        )

    def test_a_wrapped_pointer_in_code_is_checked(self) -> None:
        source = "crates/x/src/lib.rs"
        self.assertEqual(self.check(source, '// the layout (`spec/core/gui.md`\n// "Grid")'), [])
        self.assertEqual(
            self.check(source, '// the layout (`spec/core/gui.md`\n// "Gridlines")'),
            ['crates/x/src/lib.rs:1: docs/spec/core/gui.md has no "Gridlines"'],
        )

    def test_a_link_names_a_file_beside_its_own(self) -> None:
        source = "docs/spec/core/README.md"
        self.assertEqual(broken_links(self.root, source, "[gui](gui.md#grid) [x](https://a.md)"), [])
        self.assertEqual(
            broken_links(self.root, source, "see [plan](../plans/core/missing.md)"),
            ["docs/spec/core/README.md:1: link to `../plans/core/missing.md` names no file"],
        )


if __name__ == "__main__":
    unittest.main()
