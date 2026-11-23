"""Tests of scripts/roadmap.py; `just test` runs them."""

import unittest

from roadmap import group, headline, inline, parse, render

WORKLOG = """\
## Current status

### Now

One line per piece of work in flight.

- First thing in flight.
- Second thing, over
  two lines.

### Where the work stands

1. A numbered item: its detail.
   - [ ] open sub bullet: its tail.
   - [~] tilde sub bullet.
   - [!] bang sub bullet.
   - [x] done sub bullet.

**Working mode.** How the work runs.
- [ ] mode bullet: its tail.

---

## How to resume

- [ ] a bullet outside the stands section

## Phases

| Phase | Scope | Crates | Status |
|---|---|---|---|
| 1 | Bootstrap | `studio` | done |
| 2 | Libs | `wezlib` | todo |
| 3 | GUI after Phase 2 | `studio_core` | in progress |

## Steps

- [ ] a bullet outside the stands section

## Issues

- open (2026-11-23) — one.
- resolved (2026-11-23) — a settled one.
- open (2026-11-23) — two.

## Log
"""


class Parse(unittest.TestCase):
    def test_now_holds_each_bullet_with_its_continuation_joined(self) -> None:
        found = parse(WORKLOG)
        self.assertEqual(found.now, ["First thing in flight.",
                                     "Second thing, over two lines."])

    def test_next_keeps_each_open_mark(self) -> None:
        texts = [text for _, text in parse(WORKLOG).next]
        self.assertIn("open sub bullet", texts)
        self.assertIn("tilde sub bullet.", texts)
        self.assertIn("bang sub bullet.", texts)
        self.assertNotIn("done sub bullet.", texts)

    def test_next_names_each_bullets_group(self) -> None:
        found = parse(WORKLOG).next
        self.assertEqual(found[0], ("A numbered item", "open sub bullet"))
        self.assertEqual(found[-1], ("Working mode", "mode bullet"))

    def test_later_keeps_the_rows_not_done(self) -> None:
        self.assertEqual(parse(WORKLOG).later,
                         [("2", "Libs", "todo"),
                          ("3", "GUI after Phase 2", "in progress")])

    def test_open_issues_counts_only_the_open_lines(self) -> None:
        self.assertEqual(parse(WORKLOG).open_issues, 2)

    def test_a_run_in_group_ending_in_a_colon_loses_it(self) -> None:
        self.assertEqual(group(["**Done today:** prose.", "- [ ] task"], 1), "Done today")


class Headline(unittest.TestCase):
    def test_a_separator_inside_backticks_or_quotes_is_not_cut(self) -> None:
        self.assertEqual(headline("a `x: y` b: c"), "a `x: y` b")
        self.assertEqual(headline('a "x: y" b: c'), 'a "x: y" b')

    def test_the_first_separator_outside_is_cut(self) -> None:
        self.assertEqual(headline("head: tail"), "head")
        self.assertEqual(headline("head. tail"), "head")

    def test_a_text_without_a_separator_comes_back_whole(self) -> None:
        self.assertEqual(headline("whole text"), "whole text")

    def test_a_separator_inside_parentheses_is_not_cut(self) -> None:
        self.assertEqual(headline("a (b: c) d: e"), "a (b: c) d")


class Inline(unittest.TestCase):
    def test_html_is_escaped_and_marks_rendered(self) -> None:
        self.assertEqual(inline("<b>"), "&lt;b&gt;")
        self.assertEqual(inline("`a`"), "<code>a</code>")
        self.assertEqual(inline("**b**"), "<strong>b</strong>")


class Render(unittest.TestCase):
    def test_the_page_holds_each_card_and_head(self) -> None:
        page = render(parse(WORKLOG), [("2026-11-23", "merge(x): a merge")])
        for text in ("First thing in flight.", "Second thing, over two lines.",
                     "open sub bullet", "mode bullet", "Phase 2", "Phase 3",
                     "merge(x): a merge"):
            self.assertIn(text, page)
        for head in ("<h2>Now</h2>", "<h2>Next</h2>", "<h2>Later</h2>"):
            self.assertIn(head, page)
        self.assertIn("2 open issues in", page)
        self.assertNotIn('class="entry"', page)

    def test_one_open_issue_is_singular(self) -> None:
        roadmap = parse(WORKLOG)
        roadmap.open_issues = 1
        self.assertIn("1 open issue in", render(roadmap, []))

    def test_merges_fold_per_day_newest_first(self) -> None:
        merges = [("2026-11-23", "one"), ("2026-11-23", "two"), ("2026-11-22", "three")]
        page = render(parse(WORKLOG), merges)
        self.assertEqual(page.count("<details>"), 2)
        self.assertIn("2026-11-23: 2 merges", page)
        self.assertIn("2026-11-22: 1 merge", page)
        self.assertLess(page.index("2026-11-23"), page.index("2026-11-22"))

    def test_no_merges_is_a_line(self) -> None:
        self.assertIn("No merges in the last 7 days.", render(parse(WORKLOG), []))


if __name__ == "__main__":
    unittest.main()
