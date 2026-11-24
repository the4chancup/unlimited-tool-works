"""Tests of scripts/pages/roadmap.py; `just test` runs them."""

import contextlib
import io
import os
import re
import tempfile
import unittest
from pathlib import Path

import roadmap
from roadmap import inline, parse, render

WORKLOG = """\
## Current status

### Now

One line per piece of work in flight.

- First thing in flight.
- Second thing, over
  two lines.

### Where the work stands

1. [ ] **TOP** A numbered item: its detail.
   - [ ] **a** [after b] open sub bullet: its tail
     runs on here.
     - [x] **aa** [after ab] done nested bullet `63ecaae8`.
     - [ ] **ab** [after TOP/b] open nested bullet.
   - [x] **b** done sub bullet `a1b2c3d`.
   - [ ] **c** [after b, MODE] open bullet: tail.
2. [x] **DONE** done item `e5f6a7b` 2026-11-22.
   - [x] **x** done child `12345678` 2026-11-21.
   - [ ] **y** [after x] open child.
3. [ ] **P2** second item: tail.
   - [x] **x** done child `23456789` 2026-11-18.

**Working mode.** How the work runs.
- [ ] **MODE** mode bullet: its tail.
- [ ] **NEXT** [after MODE] next bullet.

**Second block.** More prose.
- [ ] **b** top-level b.

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

TODAY = "2026-11-23"


def wl(body: str) -> str:
    """A minimal worklog whose "Where the work stands" section is `body`, which starts
    at line 9."""
    return ("## Current status\n\n### Now\n\n- n.\n\n### Where the work stands\n\n"
            + body + "\n---\n\n"
            "## Phases\n\n| Phase | Scope | Crates | Status |\n|---|---|---|---|\n"
            "| 1 | Boot | `s` | done |\n\n## Issues\n\n"
            "- open (2026-01-01) — one.\n\n## Log\n")


def circles(svg: str) -> dict[str, tuple[int, int]]:
    """Each row's circle by the path its `<title>` opens with: path -> (cx, cy)."""
    found = {}
    for match in re.finditer(
            r'<g><title>([^<]*)</title><circle cx="(\d+)" cy="(\d+)"', svg):
        path = match[1].split(" ")[0]
        found[path] = (int(match[2]), int(match[3]))
    return found


class Parse(unittest.TestCase):
    def test_phases_keeps_the_rows_not_done_in_the_tables_order(self) -> None:
        self.assertEqual(parse(WORKLOG).phases,
                         [("2", "Libs", "todo"),
                          ("3", "GUI after Phase 2", "in progress")])

    def test_open_issues_counts_only_the_open_lines(self) -> None:
        self.assertEqual(parse(WORKLOG).open_issues, 2)


class Steps(unittest.TestCase):
    def test_a_nested_items_path_mark_after_and_text(self) -> None:
        top, problems = roadmap.steps(WORKLOG)
        self.assertEqual(problems, [])
        self.assertEqual([s.id for s in top],
                         ["TOP", "DONE", "P2", "MODE", "NEXT", "b"])
        a = top[0].children[0]
        self.assertEqual((a.path, a.mark, a.after, a.line),
                         ("TOP/a", " ", ["b"], 14))
        ab = a.children[1]
        self.assertEqual((ab.path, ab.mark, ab.after, ab.text),
                         ("TOP/a/ab", " ", ["TOP/b"], "open nested bullet."))

    def test_an_items_text_joins_its_continuation_lines(self) -> None:
        top, _ = roadmap.steps(WORKLOG)
        a = top[0].children[0]
        self.assertTrue(a.text.startswith("open sub bullet: its tail runs on here."))

    def test_an_items_text_stops_at_its_first_child(self) -> None:
        top, _ = roadmap.steps(WORKLOG)
        self.assertEqual(top[0].children[0].text, "open sub bullet: its tail runs on here.")
        self.assertEqual(top[1].text, "done item `e5f6a7b` 2026-11-22.")

    def test_bullets_after_a_run_in_paragraph_are_top_level(self) -> None:
        top, _ = roadmap.steps(WORKLOG)
        self.assertEqual([s.id for s in top[3:]], ["MODE", "NEXT", "b"])
        self.assertIsNone(top[3].parent)

    def test_an_item_without_a_checkbox_and_id_is_a_problem_naming_its_line(self) -> None:
        _, problems = roadmap.steps(WORKLOG.replace("- [ ] **a**", "- **a**"))
        self.assertEqual(problems, ["14: no checkbox and ID"])
        self.assertEqual(roadmap.steps(WORKLOG)[1], [])

    def test_an_id_twice_under_one_parent_is_a_problem(self) -> None:
        _, problems = roadmap.steps(WORKLOG.replace(
            "**y** [after x] open child.", "**x** [after x] open child."))
        self.assertEqual(problems, ["22: x twice under DONE"])

    def test_the_same_id_under_two_parents_is_not_a_problem(self) -> None:
        self.assertEqual(roadmap.steps(WORKLOG)[1], [])

    def test_resolve_searches_siblings_then_upward_and_down_paths(self) -> None:
        top, _ = roadmap.steps(WORKLOG)
        c = top[0].children[2]
        ab = top[0].children[0].children[1]
        self.assertEqual(roadmap.resolve("b", c, top).path, "TOP/b")
        self.assertEqual(roadmap.resolve("MODE", c, top).path, "MODE")
        self.assertEqual(roadmap.resolve("TOP/b", ab, top).path, "TOP/b")
        self.assertIsNone(roadmap.resolve("nope", c, top))
        self.assertIsNone(roadmap.resolve("TOP/nope", ab, top))

    def test_an_unresolvable_after_name_is_a_problem(self) -> None:
        _, problems = roadmap.steps(WORKLOG.replace("[after TOP/b]", "[after TOP/nope]"))
        self.assertEqual(problems,
                         ["17: TOP/a/ab waits on TOP/nope, which names no step"])

    def test_a_wait_cycle_is_a_problem(self) -> None:
        _, problems = roadmap.steps(
            wl("- [ ] **x** [after y] one.\n- [ ] **y** [after x] two.\n"))
        self.assertEqual(problems, ["9: x waits on itself through y",
                                    "10: y waits on itself through x"])

    def test_the_real_worklog_breaks_no_convention(self) -> None:
        text = Path("docs/WORKLOG.md").read_text(encoding="utf-8")
        self.assertEqual(parse(text).problems, [])


class Wrap(unittest.TestCase):
    def test_lines_break_at_spaces_within_the_width(self) -> None:
        self.assertEqual(roadmap.wrap("aa bb cc dd", 5), ["aa bb", "cc dd"])
        self.assertEqual(roadmap.wrap("", 5), [""])

    def test_a_backticked_or_quoted_span_is_never_split(self) -> None:
        self.assertEqual(roadmap.wrap("x `a b c` y", 4), ["x", "`a b c`", "y"])
        self.assertEqual(roadmap.wrap('x "a b c" y', 4), ["x", '"a b c"', "y"])

    def test_a_path_stays_with_the_section_name_after_it(self) -> None:
        self.assertEqual(roadmap.wrap('see `p.md` "A b" now', 12),
                         ["see", '`p.md` "A b"', "now"])

    def test_a_word_longer_than_the_width_stands_alone(self) -> None:
        self.assertEqual(roadmap.wrap("a abcdefgh b", 4), ["a", "abcdefgh", "b"])

    def test_a_line_is_as_wide_as_the_measure_counts_it(self) -> None:
        # "ii ii" is narrow and "WW WW" wide: at one width the first fits on a line, the
        # second does not.
        width = roadmap.text_width("ii ii")
        self.assertEqual(roadmap.wrap("ii ii", width, roadmap.text_width), ["ii ii"])
        self.assertEqual(roadmap.wrap("WW WW", width, roadmap.text_width), ["WW", "WW"])

    def test_a_code_span_counts_the_monospace_width_and_not_its_backticks(self) -> None:
        self.assertEqual(roadmap.text_width("`ab`"), 2 * roadmap.LABEL_CHAR_WIDTH)
        self.assertLess(roadmap.text_width("i"), roadmap.text_width("W"))


class Inline(unittest.TestCase):
    def test_html_is_escaped_and_marks_rendered(self) -> None:
        self.assertEqual(inline("<b>"), "&lt;b&gt;")
        self.assertEqual(inline("`a`"), "<code>a</code>")
        self.assertEqual(inline("**b**"), "<strong>b</strong>")


class Render(unittest.TestCase):
    def test_the_page_holds_each_card_and_head(self) -> None:
        page = render(parse(WORKLOG), [("2026-11-23", "merge(x): a merge")],
                      {}, TODAY)
        for text in ("open nested bullet", "mode bullet", "Phase 2", "Phase 3",
                     "merge(x): a merge", "<h2>Timeline</h2>"):
            self.assertIn(text, page)
        for head in ("<h2>Now</h2>", "<h2>Next</h2>", "<h2>Later</h2>"):
            self.assertNotIn(head, page)
        self.assertNotIn("First thing in flight.", page)
        self.assertLess(page.index("<h2>Timeline</h2>"),
                        page.index("<h2>Merged in the last 7 days</h2>"))
        self.assertIn("2 open issues in", page)
        self.assertNotIn('class="entry"', page)

    def test_a_button_brings_the_now_line_to_the_middle(self) -> None:
        page = render(parse(WORKLOG), [], {}, TODAY)
        self.assertLess(page.index('<button id="to-now" type="button">Go to now</button>'),
                        page.index("<h2>Timeline</h2>"))
        self.assertEqual(page.count('id="now"'), 1)
        self.assertIn('document.getElementById("now").scrollIntoView'
                      '({block: "center"', page)

    def test_a_steps_codename_says_what_its_groups_are_for(self) -> None:
        page = render(parse(WORKLOG), [], {}, TODAY)
        # TOP/a/ab sits two groups down; its codename names both, the outer first.
        self.assertIn('font-weight="600"><title>TOP: A numbered item: its detail.\n'
                      "TOP/a: open sub bullet: its tail runs on here.</title>TOP/a/ab</text>",
                      page)
        # A done group's text shows too: DONE is done, with y open under it.
        self.assertIn('font-weight="600"><title>DONE: done item e5f6a7b 2026-11-22.</title>'
                      "DONE/y</text>", page)

    def test_a_top_level_steps_codename_has_no_tooltip_of_its_own(self) -> None:
        page = render(parse(WORKLOG), [], {}, TODAY)
        self.assertIn('font-weight="600">MODE</text>', page)

    def test_one_open_issue_is_singular(self) -> None:
        found = parse(WORKLOG)
        found.open_issues = 1
        self.assertIn("1 open issue in", render(found, [], {}, TODAY))

    def test_merges_fold_per_day_newest_first(self) -> None:
        merges = [("2026-11-23", "one"), ("2026-11-23", "two"), ("2026-11-22", "three")]
        page = render(parse(WORKLOG), merges, {}, TODAY)
        self.assertEqual(page.count("<details>"), 2)
        self.assertIn("2026-11-23: 2 merges", page)
        self.assertIn("2026-11-22: 1 merge", page)
        self.assertLess(page.index("2026-11-23: 2 merges"),
                        page.index("2026-11-22: 1 merge"))

    def test_no_merges_is_a_line(self) -> None:
        self.assertIn("No merges in the last 7 days.",
                      render(parse(WORKLOG), [], {}, TODAY))


class Timeline(unittest.TestCase):
    DATES = {"TOP/a/aa": "2026-11-20", "TOP/b": "2026-11-23",
             "DONE/x": "2026-11-17", "P2/x": "2026-11-16"}

    def test_an_open_step_sits_above_the_open_step_it_waits_on(self) -> None:
        cy = {p: y for p, (_, y) in
              circles(roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)).items()}
        self.assertLess(cy["NEXT"], cy["MODE"])     # NEXT is listed after MODE
        self.assertLess(cy["TOP/c"], cy["MODE"])    # c waits on MODE too

    def test_done_rows_are_newest_first_and_within_the_week(self) -> None:
        cy = {p: y for p, (_, y) in
              circles(roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)).items()}
        self.assertLess(cy["TOP/b"], cy["TOP/a/aa"])
        self.assertLess(cy["TOP/a/aa"], cy["DONE/x"])
        self.assertNotIn("P2/x", cy)    # 7 days before today is out
        self.assertIn("DONE/x", cy)     # 6 days before is in

    def test_done_rows_of_one_day_follow_their_commits_times(self) -> None:
        # TOP/b is listed after TOP/a/aa: their times, not the listing's order, place them.
        dates = {**self.DATES, "TOP/a/aa": "2026-11-23T09:00:00+01:00",
                 "TOP/b": "2026-11-23T08:00:00+01:00"}
        cy = {p: y for p, (_, y) in
              circles(roadmap.timeline(parse(WORKLOG), dates, TODAY)).items()}
        self.assertLess(cy["TOP/a/aa"], cy["TOP/b"])
        dates.update({"TOP/a/aa": "2026-11-23T08:00:00+01:00",
                      "TOP/b": "2026-11-23T09:00:00+01:00"})
        cy = {p: y for p, (_, y) in
              circles(roadmap.timeline(parse(WORKLOG), dates, TODAY)).items()}
        self.assertLess(cy["TOP/b"], cy["TOP/a/aa"])

    def test_done_steps_without_a_date_are_counted(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)
        self.assertNotIn("done steps have no date", svg)
        dates = {k: v for k, v in self.DATES.items() if k != "P2/x"}
        svg = roadmap.timeline(parse(WORKLOG), dates, TODAY)
        self.assertIn("1 done step has no date", svg)
        dates = {k: v for k, v in dates.items() if k != "DONE/x"}
        svg = roadmap.timeline(parse(WORKLOG), dates, TODAY)
        self.assertIn("2 done steps have no date", svg)

    def test_a_row_is_its_monospace_label_and_its_description(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)
        self.assertRegex(svg, r'<text x="\d+" y="(\d+)" [^>]*font-family="Consolas, '
                              r'monospace" font-weight="600"><title>[^<]*</title>TOP/c'
                              r'</text><text [^>]*>'
                              r'<tspan x="\d+" y="\1">open bullet: tail\.</tspan></text>')
        self.assertIn('<title>TOP/c open bullet: tail. — waits on b, MODE</title>', svg)

    def test_the_phases_not_done_stand_above_the_open_steps_the_next_lowest(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)
        cy = {p: y for p, (_, y) in circles(svg).items()}
        self.assertNotIn("Phase 1", svg)    # done
        rows = re.findall(r'<g><title>(Phase \d)', svg)
        self.assertEqual(rows, ["Phase 3", "Phase 2"])     # 3 runs after 2: higher
        top_step = min(cy[p] for p in ("TOP/a/ab", "TOP/c", "MODE", "NEXT", "b"))
        phase_2 = re.search(r'<g><title>Phase 2[^<]*</title><circle cx="\d+" cy="(\d+)"',
                            svg)
        self.assertLess(int(phase_2[1]), top_step)
        self.assertIn('<title>Phase 3 GUI after Phase 2 (in progress)</title>', svg)

    def test_every_description_starts_at_one_column_past_the_longest_label(self) -> None:
        text = WORKLOG.replace("| 2 | Libs |", "| 0.1.0 | Libs |")
        svg = roadmap.timeline(parse(text), self.DATES, TODAY)
        rows = re.findall(r'<text x="(\d+)" [^>]*font-weight="600">([^<]*)</text>'
                          r'<text [^>]*><tspan x="(\d+)" ', svg)
        self.assertGreater(len(rows), 5)
        self.assertEqual(len({label_x for label_x, _, _ in rows}), 1)
        self.assertEqual(len({desc_x for _, _, desc_x in rows}), 1)
        label_x, desc_x = int(rows[0][0]), int(rows[0][2])
        longest = max(len(label) for _, label, _ in rows)
        self.assertEqual(longest, len("Release 0.1.0"))
        self.assertEqual(desc_x - label_x, round(0.6 * 14 * longest) + 12)

    def test_a_long_description_wraps_under_its_first_line_and_pushes_the_rows_below(self) -> None:
        long = " ".join(["word"] * 60)
        svg = roadmap.timeline(parse(wl(f"- [ ] **A** {long}\n- [ ] **B** short.\n")), {}, TODAY)
        spans = re.findall(r'<tspan x="(\d+)" y="(\d+)">(word[^<]*)</tspan>', svg)
        self.assertGreater(len(spans), 1)
        self.assertEqual(len({x for x, _, _ in spans}), 1)
        ys = [int(y) for _, y, _ in spans]
        self.assertEqual(ys, [ys[0] + 22 * i for i in range(len(ys))])
        self.assertEqual(" ".join(words for _, _, words in spans), long)
        cy = {p: y for p, (_, y) in circles(svg).items()}
        self.assertEqual(cy["A"], ys[0])                          # the code on the first line
        self.assertEqual(cy["B"], ys[-1] + 22 + 8)                 # B below A's last line
        self.assertIn(f'font-weight="600">A</text>', svg)
        self.assertRegex(svg, rf'<text x="\d+" y="{ys[0]}" [^>]*font-weight="600">A</text>')

    def test_a_wrapped_line_fills_its_column_before_it_breaks(self) -> None:
        long = " ".join(["word"] * 120)
        svg = roadmap.timeline(parse(wl(f"- [ ] **A** {long}\n")), {}, TODAY)
        spans = re.findall(r'<tspan x="(\d+)" y="\d+">(word[^<]*)</tspan>', svg)
        room = roadmap.ROADMAP_WIDTH - roadmap.NOW_ROOM - int(spans[0][0])
        lines = [text for _, text in spans]
        self.assertGreater(len(lines), 2)
        for line in lines:
            self.assertLessEqual(roadmap.text_width(line), room, line)
        for line in lines[:-1]:
            self.assertGreater(roadmap.text_width(line + " word"), room, line)

    def test_a_release_row_is_named_release_and_the_phases_lane_is_grey(self) -> None:
        text = WORKLOG.replace("| 2 | Libs |", "| 0.1.0 | Libs |")
        svg = roadmap.timeline(parse(text), self.DATES, TODAY)
        self.assertIn("<title>Release 0.1.0 Libs (todo)</title>", svg)
        text = WORKLOG.replace("| 2 | Libs |", "| 0.2.0 | Release 0.2.0, a later one |")
        self.assertIn("<title>Release 0.2.0 a later one (todo)</title>",
                      roadmap.timeline(parse(text), self.DATES, TODAY))
        self.assertRegex(svg, r'<title>Release 0\.1\.0[^<]*</title><circle cx="\d+" '
                              r'cy="\d+" r="5" fill="#1e1e1e" stroke="#999999"')

    NOW = wl("""\
1. [ ] **A** item.
   - [ ] **a1** [after a2] open.
   - [~] **a2** [after a3] in progress.
   - [x] **a3** done `abc1234`.
2. [~] **B** in progress too.
""")

    def test_steps_in_progress_sit_on_the_now_band_between_open_and_done(self) -> None:
        svg = roadmap.timeline(parse(self.NOW), {"A/a3": TODAY}, TODAY)
        cy = {p: y for p, (_, y) in circles(svg).items()}
        band = re.search(r'<rect id="now" x="0" y="(\d+)" width="\d+" height="(\d+)" rx="2"', svg)
        top, bottom = int(band[1]), int(band[1]) + int(band[2])
        self.assertLess(cy["A/a1"], top)
        for path in ("A/a2", "B"):
            self.assertTrue(top < cy[path] < bottom, (path, cy[path], top, bottom))
        self.assertGreater(cy["A/a3"], bottom)
        self.assertIn('dominant-baseline="central">now</text>', svg)
        self.assertNotIn("today", svg)

    def test_without_steps_in_progress_a_line_labelled_now_parts_open_from_done(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)
        self.assertNotIn("<rect", svg)
        line = re.search(r'<line id="now" x1="0" y1="(\d+)" x2="\d+" y2="\d+" stroke="#555"/>', svg)
        cy = {p: y for p, (_, y) in circles(svg).items()}
        self.assertTrue(cy["NEXT"] < int(line[1]) < cy["TOP/b"])
        self.assertIn('dominant-baseline="central">now</text>', svg)


class Lanes(unittest.TestCase):
    LANES = wl("""\
1. [ ] **A** item.
   - [ ] **a1** s.
   - [ ] **a2** s.
2. [ ] **B** item.
   - [ ] **b1** [after A/a2] s.
3. [ ] **C** item.
   - [ ] **c1** s.
""")

    def test_overlapping_tracks_take_different_columns_a_later_one_reuses(self) -> None:
        svg = roadmap.timeline(parse(self.LANES), {}, TODAY)
        xs = re.findall(r'<line x1="(\d+)" y1="\d+" x2="\d+" y2="\d+" '
                        r'stroke="#[0-9A-F]+" stroke-width="2"/>', svg)
        self.assertEqual([int(x) for x in xs], [10, 42, 10])

    def test_an_open_rows_circle_is_hollow_and_a_done_rows_filled(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), Timeline.DATES, TODAY)
        cx, cy = circles(svg)["TOP/c"]
        self.assertIn(f'<circle cx="{cx}" cy="{cy}" r="5" fill="#1e1e1e" '
                      f'stroke="#E69F00" stroke-width="2"/>', svg)
        cx, cy = circles(svg)["TOP/b"]
        self.assertIn(f'<circle cx="{cx}" cy="{cy}" r="5" fill="#E69F00"/>', svg)


class Edges(unittest.TestCase):
    EDGES = wl("""\
1. [ ] **A** item.
   - [ ] **a1** s.
   - [ ] **a2** [after a1] s.
   - [ ] **g** group.
     - [ ] **g1** [after a1] s.
2. [ ] **B** item.
   - [ ] **b1** [after A, OLD] s.
3. [x] **OLD** old done, undated.
""")

    def test_each_shown_wait_across_tracks_is_one_edge(self) -> None:
        svg = roadmap.timeline(parse(self.EDGES), {}, TODAY)
        self.assertEqual(svg.count("<path"), 2)

    def test_a_wait_on_an_unshown_parent_ends_at_its_topmost_descendant(self) -> None:
        svg = roadmap.timeline(parse(self.EDGES), {}, TODAY)
        self.assertIn('d="M 10 105 C 10 60, 10 60, 10 15"', svg)   # b1 on A, ends at a2
        self.assertIn('d="M 42 45 C 42 60, 10 60, 10 75"', svg)   # g1 on a1


class Main(unittest.TestCase):
    def run_main(self, text: str, argv: tuple[str, ...] = ()) -> tuple[int, str, "str | None"]:
        """`main(argv)` with WORKLOG and OUT pointed at temp files: (code, output, page)."""
        with tempfile.TemporaryDirectory() as folder:
            worklog = os.path.join(folder, "WORKLOG.md")
            out = os.path.join(folder, "roadmap.html")
            Path(worklog).write_text(text, encoding="utf-8", newline="\n")
            keep = roadmap.WORKLOG, roadmap.OUT
            roadmap.WORKLOG, roadmap.OUT = worklog, out
            shown = io.StringIO()
            try:
                code = 0
                with contextlib.redirect_stdout(shown):
                    try:
                        roadmap.main(list(argv))
                    except SystemExit as error:
                        code = error.code or 0
                page = (Path(out).read_text(encoding="utf-8")
                        if os.path.exists(out) else None)
            finally:
                roadmap.WORKLOG, roadmap.OUT = keep
            return code, shown.getvalue(), page

    def test_main_refuses_a_worklog_with_a_problem(self) -> None:
        code, out, page = self.run_main(wl("- a line with no checkbox and ID.\n"))
        self.assertEqual(code, 1)
        self.assertIsNone(page)
        self.assertIn("9: no checkbox and ID", out)

    def test_main_writes_the_page_on_a_clean_worklog(self) -> None:
        code, _, page = self.run_main(WORKLOG)
        self.assertEqual(code, 0)
        self.assertIn("<h2>Timeline</h2>", page)

    def test_check_passes_a_clean_worklog_and_writes_no_page(self) -> None:
        code, out, page = self.run_main(WORKLOG, ("--check",))
        self.assertEqual((code, out), (0, ""))
        self.assertIsNone(page)

    def test_check_refuses_a_worklog_with_a_problem(self) -> None:
        code, out, page = self.run_main(wl("- a line with no checkbox and ID.\n"), ("--check",))
        self.assertEqual(code, 1)
        self.assertIsNone(page)
        self.assertIn("9: no checkbox and ID", out)


if __name__ == "__main__":
    unittest.main()
