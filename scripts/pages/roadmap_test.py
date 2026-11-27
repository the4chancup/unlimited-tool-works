"""Tests of scripts/pages/roadmap.py; `just test` runs them."""

import contextlib
import io
import json
import os
import re
import subprocess
import tempfile
import unittest
from html.parser import HTMLParser
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


def laid_out(svg: str) -> dict:
    """`layout()`'s output for `svg`: its input built from the svg's `data-*` attributes,
    run through `roadmap_layout.js` under node. Node not being on PATH fails the test."""
    def attrs(tag: str) -> dict[str, str]:
        return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))

    head = attrs(re.search(r"<svg [^>]*>", svg)[0])
    groups = [attrs(tag) for tag in re.findall(r"<g [^>]*>", svg)]
    lanes = [a for a in map(attrs, re.findall(r"<line [^>]*>", svg))
             if "data-track" in a]
    lane_x = {a["data-track"]: int(a["x1"]) for a in lanes}
    row_lane = {a["data-row"]: a["data-track"] for a in groups if "data-row" in a}
    layout_input = {
        "lineHeight": int(head["data-line-height"]),
        "rowPad": int(head["data-row-pad"]),
        "slots": [{"gap": True} if "data-gap" in a
                  else {"path": a["data-row"], "lines": int(a["data-lines"])}
                  for a in groups],
        "tracks": [{"key": a["data-track"], "first": a["data-first"],
                    "last": a["data-last"]}
                   for a in lanes],
        "edges": [{"from": a["data-from"], "to": a["data-to"],
                   "x1": lane_x[row_lane[a["data-from"]]],
                   "x2": lane_x[row_lane[a["data-to"]]]}
                  for a in map(attrs, re.findall(r"<path [^>]*>", svg))],
        "nowRows": [a["data-row"] for a in groups if "data-now" in a],
    }
    source = Path(__file__).with_name("roadmap_layout.js").read_text(encoding="utf-8")
    script = (source + "\nconsole.log(JSON.stringify(layout("
              + json.dumps(layout_input) + ")));")
    try:
        out = subprocess.run(["node", "-e", script], capture_output=True, text=True,
                             check=True)
    except FileNotFoundError:
        raise AssertionError("node is not on PATH") from None
    return json.loads(out.stdout)


def circles(svg: str) -> dict[str, tuple[int, int]]:
    """Each row's circle's place: path -> (cx, cy), the cx its lane's `<line>`'s x
    (the circle's own `cx` is the script's to set), the cy `layout` gives the row."""
    rows = laid_out(svg)["rows"]
    lane_x = dict(re.findall(r'<line data-track="([^"]+)"[^>]*x1="(\d+)"', svg))
    found = {}
    for match in re.finditer(r'<g data-row="([^"]+)"[^>]*data-track="([^"]+)"', svg):
        found[match[1]] = (int(lane_x[match[2]]), rows[match[1]])
    return found


class Elements(HTMLParser):
    """The page's elements as `{"tag", "attrs", "parent"}` dicts in document order,
    `parent` the index of the enclosing element or -1."""

    VOID = {"meta", "link", "br", "hr", "img", "input", "wbr"}

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.nodes = []
        self.stack = []

    def handle_starttag(self, tag: str, attrs: list) -> None:
        self.nodes.append({"tag": tag, "attrs": dict(attrs),
                           "parent": self.stack[-1] if self.stack else -1})
        if tag not in self.VOID:
            self.stack.append(len(self.nodes) - 1)

    def handle_startendtag(self, tag: str, attrs: list) -> None:
        self.nodes.append({"tag": tag, "attrs": dict(attrs),
                           "parent": self.stack[-1] if self.stack else -1})

    def handle_endtag(self, tag: str) -> None:
        for i in range(len(self.stack) - 1, -1, -1):
            if self.nodes[self.stack[i]]["tag"] == tag:
                del self.stack[i:]
                return


# A DOM for the page's script under node: the attribute reads and writes and the
# queries `placeTimeline` and the "Go to now" handler make, nothing more.
DOM = """\
const NODES = __NODES__;
function desc(root) {
  return NODES.map((n, i) => i).filter(i => {
    for (let p = NODES[i].parent; p !== -1; p = NODES[p].parent)
      if (p === root) return true;
    return false;
  });
}
function select(sel, root) {
  const child = sel.match(/^:scope > (\\w+)$/);
  if (child)
    return NODES.map((n, i) => i)
      .filter(i => NODES[i].parent === root && NODES[i].tag === child[1]);
  const m = sel.match(/^(\\w+)\\[([\\w-]+)\\]$/);
  if (m) {
    const scope = root === -1 ? NODES.map((n, i) => i) : desc(root);
    return scope.filter(i => NODES[i].tag === m[1] && m[2] in NODES[i].attrs);
  }
  throw new Error("unsupported selector " + sel);
}
class El {
  constructor(i) { this._i = i; }
  get dataset() {
    const d = {};
    for (const [k, v] of Object.entries(NODES[this._i].attrs))
      if (k.startsWith("data-"))
        d[k.slice(5).replace(/-([a-z])/g, (m, c) => c.toUpperCase())] = v;
    return d;
  }
  getAttribute(n) { return NODES[this._i].attrs[n] ?? null; }
  setAttribute(n, v) { NODES[this._i].attrs[n] = String(v); }
  addEventListener() {}
  querySelector(sel) { const m = select(sel, this._i); return m.length ? new El(m[0]) : null; }
  querySelectorAll(sel) { return select(sel, this._i).map(i => new El(i)); }
}
const document = {
  querySelector: sel => { const m = select(sel, -1); return m.length ? new El(m[0]) : null; },
  querySelectorAll: sel => select(sel, -1).map(i => new El(i)),
  getElementById: id => { const i = NODES.findIndex(n => n.attrs.id === id);
                          return i === -1 ? null : new El(i); },
};
"""


def placed(page: str) -> dict:
    """The attributes the page's own `<script>` leaves set, the script run under node on
    a stand-in DOM of the page's elements: `svg`, `now` and `now-label` keys, `rows` by
    `data-row`, `tracks` by `data-track`, `edges` by (`data-from`, `data-to`), `circles`
    by the `data-row` of the `<g>` the circle sits in."""
    script = re.search(r"<script>\n(.*)\n</script>", page, re.S)[1]
    parser = Elements()
    parser.feed(page)
    program = (DOM.replace("__NODES__", json.dumps(parser.nodes)) + script
               + "\nconsole.log(JSON.stringify(NODES));")
    try:
        out = subprocess.run(["node", "-e", program], capture_output=True, text=True,
                             check=True)
    except FileNotFoundError:
        raise AssertionError("node is not on PATH") from None
    nodes = json.loads(out.stdout)
    found = {"rows": {}, "tracks": {}, "edges": {}, "circles": {}}
    for node in nodes:
        tag, attrs = node["tag"], node["attrs"]
        if "data-row" in attrs:
            found["rows"][attrs["data-row"]] = attrs
        elif "data-track" in attrs:
            found["tracks"][attrs["data-track"]] = attrs
        elif "data-from" in attrs:
            found["edges"][(attrs["data-from"], attrs["data-to"])] = attrs
        elif attrs.get("id") == "now":
            found["now"] = attrs
        elif attrs.get("id") == "now-label":
            found["now-label"] = attrs
        elif tag == "svg":
            found["svg"] = attrs
    for node in nodes:
        row = (nodes[node["parent"]]["attrs"].get("data-row")
               if node["tag"] == "circle" and node["parent"] != -1 else None)
        if row:
            found["circles"][row] = node["attrs"]
    return found


def page_lines_diff(before: str, after: str) -> tuple[set[str], set[str]]:
    """(removed, added): the page lines the `after` worklog's render drops and gains
    against `before`'s, a line that only moved in the page counting as unchanged."""
    old = set(render(parse(before), [], {}, TODAY).splitlines())
    new = set(render(parse(after), [], {}, TODAY).splitlines())
    return old - new, new - old


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

    def test_a_new_row_changes_only_its_own_lines(self) -> None:
        before = wl("""\
1. [ ] **A** item.
   - [ ] **a1** s.
   - [ ] **g** group.
     - [ ] **g1** s.
   - [ ] **h** group.
     - [ ] **h1** s.
""")
        # A second-level step inserted second opens a new track before g's and h's.
        after = before.replace("   - [ ] **g** group.",
                               "   - [ ] **n** group.\n     - [ ] **n1** s.\n"
                               "   - [ ] **g** group.")
        removed, added = page_lines_diff(before, after)
        self.assertEqual(removed, set())
        self.assertTrue(added)
        for line in added:
            self.assertIn("A/n", line)

    def test_a_lane_taking_a_new_column_changes_only_its_own_line(self) -> None:
        # The A/n lane's arrival reorders the rows so A/g's lane leaves its column
        # for A's: the page writes a lane's x on its <line> alone, so A/g's moved
        # lane is the only page line that changes.
        before = wl("""\
1. [ ] **A** item.
   - [ ] **a1** s.
   - [ ] **a2** s.
   - [ ] **g** group.
     - [ ] **g1** s.
     - [ ] **g2** s.
   - [ ] **h** group.
     - [ ] **h1** s.
     - [ ] **h2** [after a1] s.
""")
        after = before.replace("   - [ ] **h** group.",
                               "   - [ ] **n** group.\n"
                               "     - [ ] **n1** s.\n"
                               "     - [ ] **n2** [after a2] s.\n"
                               "   - [ ] **h** group.")
        lane = r'<line data-track="A/g"[^>]*x1="(\d+)"'
        xs = [re.search(lane, render(parse(text), [], {}, TODAY))[1]
              for text in (before, after)]
        self.assertNotEqual(xs[0], xs[1])
        removed, _ = page_lines_diff(before, after)
        self.assertEqual(
            removed,
            {'<line data-track="A/g" data-first="A/g/g1" '
             'data-last="A/g/g2" x1="42" x2="42" stroke="#E69F00" '
             'stroke-width="2"/>'})

    def test_place_timeline_lays_out_the_rows_the_line_its_label_and_the_height(self) -> None:
        # No step in progress and no done step: the bare "now" line in the gap slot.
        page = render(parse(wl("- [ ] **A** open.\n- [ ] **B** open too.\n")), [], {}, TODAY)
        put = placed(page)
        self.assertEqual(put["rows"]["A"]["transform"], "translate(0,15)")
        self.assertEqual(put["rows"]["B"]["transform"], "translate(0,45)")
        self.assertEqual(put["circles"]["A"]["cx"], "10")   # A's lane's column
        self.assertEqual((put["now"]["y1"], put["now"]["y2"]), ("71", "71"))
        self.assertEqual(put["now-label"]["y"], "64")
        self.assertEqual((put["svg"]["height"], put["svg"]["viewBox"]),
                         ("82", "0 0 1400 82"))

    def test_place_timeline_lays_out_the_band_its_label_the_rows_and_an_edge(self) -> None:
        # Timeline.NOW: the band covers the two in-progress rows; its label is its middle.
        page = render(parse(Timeline.NOW), [], {"A/a3": TODAY}, TODAY)
        put = placed(page)
        self.assertEqual((put["now"]["y"], put["now"]["height"]), ("30", "60"))
        self.assertEqual(put["now-label"]["y"], "60")
        self.assertEqual(
            {p: put["rows"][p]["transform"] for p in ("A/a1", "A/a2", "B", "A/a3")},
            {"A/a1": "translate(0,15)", "A/a2": "translate(0,45)",
             "B": "translate(0,75)", "A/a3": "translate(0,105)"})
        # NOW's waits are inside one track, so no edge shows; Edges.EDGES has two.
        put = placed(render(parse(Edges.EDGES), [], {}, TODAY))
        self.assertEqual(put["edges"][("A/g/g1", "A/a1")]["d"],
                         "M 42 45 C 42 60, 10 60, 10 75")
        self.assertEqual(put["edges"][("B/b1", "A/a2")]["d"],
                         "M 10 105 C 10 60, 10 60, 10 15")
        # A/g's lane sits one column in, B's shares A's column.
        self.assertEqual(put["circles"]["A/g/g1"]["cx"], "42")
        self.assertEqual(put["circles"]["B/b1"]["cx"], "10")
        # A's track runs from a2's row to a1's.
        self.assertEqual((put["tracks"]["A"]["y1"], put["tracks"]["A"]["y2"]), ("15", "75"))


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
        rows = re.findall(r'<g data-row="(Phase \d)"', svg)
        self.assertEqual(rows, ["Phase 3", "Phase 2"])     # 3 runs after 2: higher
        top_step = min(cy[p] for p in ("TOP/a/ab", "TOP/c", "MODE", "NEXT", "b"))
        self.assertLess(cy["Phase 2"], top_step)
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
        self.assertEqual(ys[0], 0)                               # first line on the row's circle
        self.assertEqual(cy["B"], cy["A"] + ys[-1] + 22 + 8)     # B below A's last line
        self.assertIn(f'font-weight="600">A</text>', svg)
        self.assertRegex(svg, r'<text x="\d+" y="0" [^>]*font-weight="600">A</text>')

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
        self.assertRegex(svg, r'<g data-row="Release 0\.1\.0"[^>]*data-track="phases">'
                              r'<title>Release 0\.1\.0[^<]*</title><circle cx="0" '
                              r'cy="0" r="5" fill="#1e1e1e" stroke="#999999"')

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
        self.assertIn('<rect id="now" x="0" width="1400" rx="2"', svg)
        now = laid_out(svg)["now"]
        top, height = now["band"]
        self.assertEqual(now["labelY"], top + height / 2)
        bottom = top + height
        self.assertLess(cy["A/a1"], top)
        for path in ("A/a2", "B"):
            self.assertTrue(top < cy[path] < bottom, (path, cy[path], top, bottom))
        self.assertGreater(cy["A/a3"], bottom)
        self.assertIn('dominant-baseline="central">now</text>', svg)
        self.assertNotIn("today", svg)

    def test_without_steps_in_progress_a_line_labelled_now_parts_open_from_done(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), self.DATES, TODAY)
        self.assertNotIn("<rect", svg)
        self.assertIn('<line id="now" x1="0" x2="1400" stroke="#555"/>', svg)
        line_y = laid_out(svg)["now"]["line"]
        cy = {p: y for p, (_, y) in circles(svg).items()}
        self.assertTrue(cy["NEXT"] < line_y < cy["TOP/b"])
        self.assertIn('dominant-baseline="central">now</text>', svg)

    def test_with_only_open_steps_the_now_line_sits_below_them(self) -> None:
        svg = roadmap.timeline(parse(wl("- [ ] **A** open.\n- [ ] **B** open too.\n")), {},
                               TODAY)
        out = laid_out(svg)
        # Two rows of one line (22 + 8 each), then the line's empty slot of 22.
        self.assertEqual(out["now"], {"line": 71, "labelY": 64})
        self.assertEqual(out["height"], 82)


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
        xs = re.findall(r'<line data-track="[^"]+"[^>]*x1="(\d+)"', svg)
        self.assertEqual([int(x) for x in xs], [10, 42, 10])

    def test_each_tracks_ends_are_its_first_and_last_member_rows_ys(self) -> None:
        svg = roadmap.timeline(parse(self.LANES), {}, TODAY)
        cy = {p: y for p, (_, y) in circles(svg).items()}
        out = laid_out(svg)
        tracks = re.findall(r'<line data-track="([^"]+)" data-first="([^"]+)" '
                            r'data-last="([^"]+)"', svg)
        self.assertEqual([key for key, _, _ in tracks], ["A", "B", "C"])
        for key, first, last in tracks:
            self.assertEqual(out["tracks"][key], [cy[first], cy[last]], key)

    def test_an_open_rows_circle_is_hollow_and_a_done_rows_filled(self) -> None:
        svg = roadmap.timeline(parse(WORKLOG), Timeline.DATES, TODAY)
        self.assertRegex(svg, r'<g data-row="TOP/c"[^>]*data-track="TOP">'
                              r'<title>[^<]*</title><circle cx="0" cy="0" r="5" '
                              r'fill="#1e1e1e" stroke="#E69F00" stroke-width="2"/>')
        self.assertRegex(svg, r'<g data-row="TOP/b"[^>]*data-track="TOP">'
                              r'<title>[^<]*</title><circle cx="0" cy="0" r="5" '
                              r'fill="#E69F00"/>')


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
        edges = laid_out(svg)["edges"]
        self.assertIn('M 10 105 C 10 60, 10 60, 10 15', edges)   # b1 on A, ends at a2
        self.assertIn('M 42 45 C 42 60, 10 60, 10 75', edges)   # g1 on a1


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
