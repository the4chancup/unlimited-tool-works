"""Writes `docs/maintainer/roadmap.html`: the maintainer's at-a-glance page, a timeline of the
phases not yet done, the open steps and what they wait on, the steps in progress on its "now"
line and the last week's done steps, made from `docs/WORKLOG.md` and the git log's recent
merges so it cannot drift from them.
It refuses a worklog whose "Where the work stands" items break the header's convention (no
checkbox and ID, an ID twice among siblings, an `[after …]` naming no step, a wait that
comes back to its step), so a wrong dependency fails when it is written, not at a converge.
Run from the repository root (`just roadmap`) after the worklog changes; the page is never
written by hand. A sample for the maintainer's look for now: wiring it into
`scripts/commit.py` waits on that look."""
import datetime
import heapq
import html
import os
import re
import subprocess
import sys
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from itertools import groupby
from operator import itemgetter

WORKLOG = "docs/WORKLOG.md"
OUT = "docs/maintainer/roadmap.html"

# An item of "Where the work stands" (`- ` or `6. `), with its leading spaces.
ITEM = re.compile(r"^( *)(-|\d+\.) ")
# What an item's rest must open with: checkbox, bold ID, the optional `[after …]` list.
HEAD = re.compile(r"^\[(?P<mark>[ x~!])\] \*\*(?P<id>[^*]+)\*\*"
                  r"(?: \[after (?P<after>[^\]]+)\])? ?")
# A done step's date: its first backticked short hash, else the first date in its text.
HASH = re.compile(r"`([0-9a-fA-F]{7,12})`")
DATE = re.compile(r"\d{4}-\d{2}-\d{2}")
OPEN_MARKS = " ~!"
# The timeline's room between two lanes, wide enough for an edge between them to be followed.
LANE_GAP = 32
# The phases' lane: grey, apart from the palette the work's lanes take.
PHASE_COLOUR = "#999999"
# The timeline is drawn at its own size, never scaled, so its text is FONT_SIZE pixels; its
# width is the roadmap page's (ROADMAP_WIDTH, the body's width there).
FONT_SIZE = 14
ROADMAP_WIDTH = 1400
# A description's lines stand LINE_HEIGHT apart; a row is its lines plus ROW_PAD.
LINE_HEIGHT = 22
ROW_PAD = 8
# The labels are monospace, so the descriptions' column is the longest label's length times
# one character's width: 0.6 em, Consolas's 0.55 em rounded up to the 0.602 em of DejaVu
# Sans Mono, the fallback where Consolas is missing.
LABEL_FONT = "Consolas, monospace"
LABEL_CHAR_WIDTH = 0.6 * FONT_SIZE
# The room between the longest label and the descriptions' column.
LABEL_GAP = 12
# Segoe UI's advance widths (regular, in 2048ths of an em), read from its font file, for the
# characters the steps' descriptions use: a description wraps at the width its text takes
# on the page. A character missing here counts a whole em, so a line still never overflows.
SEGOE_UI = dict(zip("".join(map(chr, range(32, 127))) + "—–’‘“”…×é→≤≥", (
    561, 582, 803, 1210, 1104, 1676, 1639, 471, 618, 618, 854, 1401, 444, 819, 444, 798,
    1104, 1104, 1104, 1104, 1104, 1104, 1104, 1104, 1104, 1104, 444, 444, 1401, 1401, 1401, 918,
    1956, 1321, 1174, 1268, 1436, 1036, 1000, 1405, 1454, 545, 731, 1188, 964, 1839, 1532, 1544,
    1147, 1544, 1225, 1088, 1073, 1407, 1272, 1913, 1208, 1132, 1168, 618, 776, 618, 1401, 850,
    549, 1042, 1204, 946, 1206, 1071, 641, 1206, 1159, 496, 496, 1018, 496, 1764, 1159, 1200,
    1204, 1206, 712, 869, 694, 1159, 981, 1480, 940, 991, 926, 618, 490, 618, 1401, 2048,
    1024, 469, 469, 772, 772, 1501, 1401, 1071, 1767, 1403, 1403,
), strict=True))
# Room kept at the right for the band's "now" label.
NOW_ROOM = 48
# One per top-level step, in the order they are listed: the Okabe-Ito palette, every one readable on
# the page's `#1e1e1e`.
PALETTE = ["#E69F00", "#56B4E9", "#009E73", "#F0E442",
           "#0072B2", "#D55E00", "#CC79A7", "#999999"]


@dataclass
class Step:
    """One item of "Where the work stands": `id` (its bold code), `path` (the IDs from the
    top-level item down, joined by `/`), `mark` (the checkbox), `text` (what follows the ID
    and the `[after …]`, continuation lines joined), `after` (the names as written),
    `children`, `parent` and `line` (its 1-based line in the worklog)."""
    id: str
    path: str
    mark: str
    text: str
    after: list[str]
    children: list["Step"]
    parent: "Step | None" = field(repr=False, compare=False, default=None)
    line: int = 0


@dataclass
class Roadmap:
    """What the page shows: `phases` (phase, scope, status) per row of the phases table not
    done, in the table's order, which is the order the phases run; `open_issues` the count
    of open lines under "Issues", `steps` the section's tree and `problems` the convention
    problems found in it."""
    phases: list[tuple[str, str, str]] = field(default_factory=list)
    open_issues: int = 0
    steps: list[Step] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


def heading(lines: list[str], title: str) -> int:
    """The index of the line that is `title`."""
    return next(i for i, line in enumerate(lines) if line.strip() == title)


def joined(block: list[str], index: int, start: int, items: bool = True) -> str:
    """The text of the item at `index` from `start`, joined with its continuation lines:
    the following lines indented deeper than its own, until a blank line or one at the same
    or a lower indentation, or, with `items` false, the first line that is an item of its
    own (a child step, whose text is its own)."""
    indent = len(block[index]) - len(block[index].lstrip())
    parts = [block[index][start:].strip()]
    for line in block[index + 1:]:
        if not line.strip() or len(line) - len(line.lstrip()) <= indent:
            break
        if not items and ITEM.match(line):
            break
        parts.append(line.strip())
    return " ".join(parts)


def walk(top: list[Step]) -> Iterator[Step]:
    """Every step in `top`, in worklog order: each step before its children."""
    for step in top:
        yield step
        yield from walk(step.children)


def resolve(name: str, step: Step, top: list[Step]) -> Step | None:
    """The step `name` in `[after …]` names, by the header's rule: looked up among the
    step's siblings first, then among its parent's, and so on up to the top-level items;
    a path such as `GRID/1b` names a step inside another, its first name looked up the
    same way."""
    first, *rest = name.split("/")
    node = step.parent
    while True:
        scope = node.children if node else top
        hit = next((c for c in scope if c.id == first), None)
        if hit:
            for part in rest:
                hit = next((c for c in hit.children if c.id == part), None)
                if hit is None:
                    return None
            return hit
        if node is None:
            return None
        node = node.parent


def reaches(source: Step, wanted: Step, top: list[Step]) -> bool:
    """`wanted` is reachable from `source` through resolved `[after …]` waits."""
    seen, queue = set(), [source]
    while queue:
        step = queue.pop()
        if step is wanted:
            return True
        if id(step) in seen:
            continue
        seen.add(id(step))
        queue += [t for name in step.after
                  if (t := resolve(name, step, top)) is not None]
    return False


def steps(text: str) -> tuple[list[Step], list[str]]:
    """The top-level steps of "Where the work stands" and the problems found: an item
    without its checkbox and ID, an ID twice among siblings, an `[after …]` name that
    resolves to nothing, a wait that comes back to its step. Nesting is by indentation;
    a column-0 line that is not an item ends every open list."""
    lines = text.splitlines()
    start = heading(lines, "### Where the work stands")
    block, index = [], start + 1
    while index < len(lines) and not lines[index].startswith(("## ", "---")):
        block.append(lines[index])
        index += 1

    top, stack, problems = [], [], []
    for i, line in enumerate(block):
        item = ITEM.match(line)
        if not item:
            if line and not line.startswith(" "):
                stack = []
            continue
        head = HEAD.match(line[item.end():])
        if not head:
            problems.append(f"{start + 2 + i}: no checkbox and ID")
            continue
        indent = len(item.group(1))
        while stack and stack[-1][0] >= indent:
            stack.pop()
        parent = stack[-1][1] if stack else None
        step = Step(id=head["id"],
                    path=f"{parent.path}/{head['id']}" if parent else head["id"],
                    mark=head["mark"],
                    text=joined(block, i, item.end() + head.end(), items=False),
                    after=[a.strip() for a in head["after"].split(",")]
                          if head["after"] else [],
                    children=[], parent=parent, line=start + 2 + i)
        (parent.children if parent else top).append(step)
        stack.append((indent, step))

    all_steps = list(walk(top))
    for parent in [None] + all_steps:
        children = top if parent is None else parent.children
        seen = set()
        for child in children:
            if child.id in seen:
                where = parent.path if parent else "the top"
                problems.append(f"{child.line}: {child.id} twice under {where}")
            else:
                seen.add(child.id)
    for step in all_steps:
        for name in step.after:
            target = resolve(name, step, top)
            if target is None:
                problems.append(f"{step.line}: {step.path} waits on {name}, "
                                "which names no step")
            elif reaches(target, step, top):
                problems.append(f"{step.line}: {step.path} waits on itself "
                                f"through {name}")
    return top, problems


def parse(text: str) -> Roadmap:
    """The worklog's parts the page shows, as a `Roadmap`."""
    lines = text.splitlines()
    roadmap = Roadmap()
    roadmap.steps, roadmap.problems = steps(text)

    start = heading(lines, "## Phases")
    index = start + 1
    while index < len(lines) and not lines[index].startswith("## "):
        line = lines[index]
        if line.startswith("|"):
            cells = [cell.strip() for cell in line.split("|")[1:-1]]
            if len(cells) >= 4 and cells[0] not in ("Phase", "---") and cells[3] != "done":
                roadmap.phases.append((cells[0], cells[1], cells[3]))
        index += 1

    start = heading(lines, "## Issues")
    index = start + 1
    while index < len(lines) and not lines[index].startswith("## "):
        if lines[index].startswith("- open"):
            roadmap.open_issues += 1
        index += 1

    return roadmap


def inline(text: str) -> str:
    """`text` HTML-escaped, with `` `x` `` as `<code>` and `**x**` as `<strong>`."""
    out = []
    for index, part in enumerate(re.split(r"`([^`]*)`", html.escape(text))):
        if index % 2:
            out.append(f"<code>{part}</code>")
        else:
            out.append(re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", part))
    return "".join(out)


def recent_merges(days: int = 7) -> list[tuple[str, str]]:
    """(date, subject) per merge of the last `days` days, newest first."""
    log = subprocess.run(["git", "log", "--merges", f"--since={days}.days",
                          "--format=%as%x09%s"],
                         capture_output=True, text=True, check=True).stdout
    return [tuple(line.split("\t", 1)) for line in log.splitlines()]


def order_open(shown: list[Step], top: list[Step]) -> list[Step]:
    """`shown`, worklog order, re-sorted so a step sits above every shown step it waits
    on: a stable topological order whose ties keep the worklog's. A wait that forms a
    cycle is a `steps` problem; a step still in one sinks to the end here."""
    place = {id(s): i for i, s in enumerate(shown)}
    targets = {id(s): [t for name in s.after
                       if (t := resolve(name, s, top)) is not None]
               for s in shown}
    waiting = [0] * len(shown)    # how many shown steps wait on each
    for s in shown:
        for t in targets[id(s)]:
            if id(t) in place:
                waiting[place[id(t)]] += 1
    ready = [i for i in range(len(shown)) if not waiting[i]]
    heapq.heapify(ready)
    order, emitted = [], set()
    while ready:
        i = heapq.heappop(ready)
        order.append(shown[i])
        emitted.add(i)
        for t in targets[id(shown[i])]:
            if id(t) not in place:
                continue
            j = place[id(t)]
            waiting[j] -= 1
            if not waiting[j]:
                heapq.heappush(ready, j)
    order += [s for i, s in enumerate(shown) if i not in emitted]
    return order


def lanes(rows: list[Step], slot: dict[int, int], top: list[Step],
          phases: list[Step]):
    """The timeline's tracks, `[top slot, bottom slot, colour, column]` each, and every
    shown step's track index. The phases not done are one grey track; besides it, a track
    is the shown rows of one top-level step and its direct leaf children, or of one
    second-level step's whole subtree. Columns are greedy: tracks sorted by top row, each
    takes the lowest column no track already in it overlaps (spans sharing a row
    overlap)."""
    shown = {id(s) for s in rows}
    tracks, member = [], {}
    if phases:
        member.update((id(s), 0) for s in phases)
        tracks.append([min(slot[id(s)] for s in phases),
                       max(slot[id(s)] for s in phases), PHASE_COLOUR])
    for i, t in enumerate(top):
        colour = PALETTE[i % len(PALETTE)]

        def add(members: list[Step]) -> None:
            ids = [id(m) for m in members if id(m) in shown]
            if ids:
                member.update((m, len(tracks)) for m in ids)
                tracks.append([min(slot[m] for m in ids),
                               max(slot[m] for m in ids), colour])

        if not t.children:
            add([t])
            continue
        add([t] + [c for c in t.children if not c.children])
        for c in t.children:
            if c.children:
                add(list(walk([c])))

    columns: list[list[tuple[int, int]]] = []
    for k in sorted(range(len(tracks)), key=lambda k: tracks[k][0]):
        lo, hi = tracks[k][0], tracks[k][1]
        for c, spans in enumerate(columns):
            if all(hi < a or lo > b for a, b in spans):
                tracks[k].append(c)
                spans.append((lo, hi))
                break
        else:
            tracks[k].append(len(columns))
            columns.append([(lo, hi)])
    return tracks, member, len(columns)


def text_width(text: str) -> float:
    """`text`'s width in pixels as a description draws it: Segoe UI at FONT_SIZE, each
    backticked span in the labels' monospace font and its backticks dropped (`mono`)."""
    width, code = 0.0, False
    for char in text:
        if char == "`":
            code = not code
        elif code:
            width += LABEL_CHAR_WIDTH
        else:
            width += SEGOE_UI.get(char, 2048) * FONT_SIZE / 2048
    return width


def wrap(text: str, width: float, measure: Callable[[str], float] = len) -> list[str]:
    """`text` in lines no wider than `width` as `measure` counts it (characters by default),
    broken at spaces, but never inside a backticked or double-quoted span, nor between a
    backticked path and the quoted section name after it: the doc-pointer gate
    (`scripts/gates/doc_pointers.py`) reads the page, and a pointer split over two lines is
    one it cannot read. A word wider than `width` stands on a line of its own."""
    words: list[str] = []
    in_code = in_quote = False
    for word in text.split():
        glued = words and (in_code or in_quote
                           or (words[-1].endswith("`") and word.startswith('"')))
        if glued:
            words[-1] += " " + word
        else:
            words.append(word)
        for char in word:
            if char == "`" and not in_quote:
                in_code = not in_code
            elif char == '"' and not in_code:
                in_quote = not in_quote
    lines: list[str] = []
    for word in words:
        if lines and measure(lines[-1] + " " + word) <= width:
            lines[-1] += " " + word
        else:
            lines.append(word)
    return lines or [""]


def mono(text: str) -> str:
    """`text` HTML-escaped, each backticked span a monospace `<tspan>` with the
    backticks dropped."""
    out = []
    for i, part in enumerate(html.escape(text).split("`")):
        if i % 2:
            out.append(f'<tspan font-family="Consolas, monospace">{part}</tspan>')
        else:
            out.append(part)
    return "".join(out)


def timeline(roadmap: Roadmap, dates: dict[str, str], today: str) -> str:
    """The steps as a vertical `git log --graph`: on top the phases not done, the last to
    run highest, then the open steps, each above every open step it waits on, then the
    steps in progress (`[~]`) on a band labelled `now` (a bare line when there are none),
    then the last week's done steps, newest first; a lane per track and an edge to what
    each step waits on. `dates` maps a step's path to its commit's ISO time or a bare
    `YYYY-MM-DD`, which sorts before the times of its day."""
    top = roadmap.steps
    phase_rows = []
    for phase, scope, status in reversed(roadmap.phases):
        name = f"Release {phase}" if "." in phase else f"Phase {phase}"
        # A release's scope opens with its own name, which the row's bold label already says.
        if scope.startswith(name):
            scope = scope[len(name):].lstrip(", ")
        phase_rows.append(Step(id=name, path=name, mark=" ", text=f"{scope} ({status})",
                               after=[], children=[]))
    leaves = order_open([s for s in walk(top)
                         if s.mark in OPEN_MARKS
                         and not any(c.mark in OPEN_MARKS for c in s.children)],
                        top)
    open_rows = phase_rows + [s for s in leaves if s.mark != "~"]
    now_rows = [s for s in leaves if s.mark == "~"]
    end = datetime.date.fromisoformat(today)
    since = end - datetime.timedelta(days=6)
    done, undated = [], 0
    for s in walk(top):
        if s.mark in OPEN_MARKS or s.children:
            continue
        day = dates.get(s.path)
        if day is None:
            undated += 1
        elif since <= datetime.date.fromisoformat(day[:10]) <= end:
            done.append((day, s))
    done.sort(key=lambda d: (d[0], d[1].line), reverse=True)
    done_rows = [s for _, s in done]
    rows = open_rows + now_rows + done_rows

    # With steps in progress, their rows are the band; with none, an empty row is the line.
    first_done = len(open_rows) + (len(now_rows) or 1)
    slot = {id(s): i for i, s in enumerate(open_rows + now_rows)}
    slot.update({id(s): first_done + j for j, s in enumerate(done_rows)})

    tracks, member, ncols = lanes(rows, slot, top, phase_rows)

    def col_x(col: int) -> int:
        return 10 + LANE_GAP * col

    text_x = col_x(ncols - 1) + 18 if ncols else 24
    desc_x = text_x + round(LABEL_CHAR_WIDTH * max((len(s.path) for s in rows),
                                                   default=0)) + LABEL_GAP
    width = ROADMAP_WIDTH
    room = width - NOW_ROOM - desc_x
    lines = {id(s): wrap(s.text.rstrip(";,"), room, text_width) for s in rows}

    # Each slot's top: a row is as tall as its lines; the empty slot of the bare line one line.
    tops = [0]
    by_slot = {slot[id(s)]: s for s in rows}
    for i in range(max([*slot.values(), first_done - 1]) + 1):
        s = by_slot.get(i)
        tops.append(tops[-1] + (LINE_HEIGHT * len(lines[id(s)]) + ROW_PAD if s
                                else LINE_HEIGHT))

    def slot_y(i: int) -> int:
        """The middle of slot `i`'s first line."""
        return tops[i] + (ROW_PAD + LINE_HEIGHT) // 2 if i in by_slot else \
            (tops[i] + tops[i + 1]) // 2

    def cy(s: Step) -> int:
        return slot_y(slot[id(s)])

    band_top = tops[len(open_rows)]
    band_bottom = tops[first_done]
    height = tops[-1]

    parts = [f'<svg viewBox="0 0 {width} {height}" width="{width}" height="{height}" '
             'xmlns="http://www.w3.org/2000/svg">']
    if now_rows:
        parts.append(f'<rect id="now" x="0" y="{band_top}" width="{width}" '
                     f'height="{band_bottom - band_top}" rx="2" fill="#2b2b2b" '
                     'stroke="#555"/>')
        label_y = (band_top + band_bottom) / 2
    else:
        sep_y = (band_top + band_bottom) / 2
        parts.append(f'<line id="now" x1="0" y1="{sep_y:g}" x2="{width}" y2="{sep_y:g}" '
                     'stroke="#555"/>')
        label_y = sep_y - 7
    parts.append(f'<text x="{width - 8}" y="{label_y:g}" fill="#999" '
                 f'font-size="{FONT_SIZE}" '
                 'text-anchor="end" dominant-baseline="central">now</text>')
    for lo, hi, colour, col in tracks:
        parts.append(f'<line x1="{col_x(col)}" y1="{slot_y(lo)}" '
                     f'x2="{col_x(col)}" y2="{slot_y(hi)}" '
                     f'stroke="{colour}" stroke-width="2"/>')
    for s in rows:
        for name in s.after:
            t = resolve(name, s, top)
            if t is None:
                continue
            if id(t) not in member:
                inside = [d for d in walk([t]) if id(d) in member]
                if not inside:
                    continue
                t = min(inside, key=lambda d: slot[id(d)])
            if member[id(t)] == member[id(s)]:
                continue
            x1 = col_x(tracks[member[id(s)]][3])
            x2 = col_x(tracks[member[id(t)]][3])
            y1, y2, ym = cy(s), cy(t), (cy(s) + cy(t)) / 2
            parts.append(f'<path d="M {x1} {y1} C {x1} {ym:g}, {x2} {ym:g}, {x2} {y2}" '
                         f'stroke="{tracks[member[id(s)]][2]}" stroke-width="1.5" '
                         'opacity="0.6" fill="none"/>')
    for s in rows:
        ti = member[id(s)]
        colour, x, y = tracks[ti][2], col_x(tracks[ti][3]), cy(s)
        open_ = s.mark in OPEN_MARKS
        circle = (f'<circle cx="{x}" cy="{y}" r="5" fill="#1e1e1e" '
                  f'stroke="{colour}" stroke-width="2"/>' if open_ else
                  f'<circle cx="{x}" cy="{y}" r="5" fill="{colour}"/>')
        text = s.text.rstrip(";,")
        title = (f"{s.path} {text}"
                 + (f" — waits on {', '.join(s.after)}" if s.after else ""))
        attrs = (f'fill="{"#fff" if open_ else "#ddd"}" font-size="{FONT_SIZE}" '
                 'dominant-baseline="central"')
        spans = "".join(f'<tspan x="{desc_x}" y="{y + LINE_HEIGHT * i}">{mono(part)}</tspan>'
                        for i, part in enumerate(lines[id(s)]))
        parts.append(f'<g><title>{html.escape(title)}</title>{circle}'
                     f'<text x="{text_x}" y="{y}" {attrs} font-family="{LABEL_FONT}" '
                     'font-weight="600">'
                     f'{html.escape(s.path)}</text>'
                     f'<text {attrs}>{spans}</text></g>')
    parts.append("</svg>")
    if undated:
        parts.append(f'<p class="from">{undated} done step'
                     f'{" has" if undated == 1 else "s have"} no date</p>')
    return "\n".join(parts)


def step_dates(top: list[Step]) -> dict[str, str]:
    """Each done step's date: its first backticked short hash's commit time (ISO), else the
    first `YYYY-MM-DD` in its text, else none. One `git show` resolves every hash."""
    wanted = {}
    for s in walk(top):
        if s.mark not in OPEN_MARKS:
            hit = HASH.search(s.text)
            if hit:
                wanted[s.path] = hit.group(1)
    dates = {}
    if wanted:
        out = subprocess.run(["git", "show", "-s", "--format=%h%x09%aI",
                              *wanted.values()],
                             capture_output=True, text=True, check=False).stdout
        shown = dict(line.split("\t", 1) for line in out.splitlines()
                     if "\t" in line)
        for path, short in wanted.items():
            for full, day in shown.items():
                if full.startswith(short) or short.startswith(full):
                    dates[path] = day
                    break
    for s in walk(top):
        if s.mark in OPEN_MARKS or s.path in dates:
            continue
        hit = DATE.search(s.text)
        if hit:
            dates[s.path] = hit.group(0)
    return dates


NAV = """\
<nav class="pages">
  <a class="here" href="roadmap.html">Roadmap</a>
  <a href="questions.html">Questions</a>
  <a href="proposals.html">Proposals</a>
  <a href="tasks.html">Tasks</a>
  <a href="ui_decisions.html">UI decisions</a>
</nav>"""


def render(roadmap: Roadmap, merges: list[tuple[str, str]],
           dates: dict[str, str], today: str) -> str:
    """The page: a button that brings the timeline's "now" line to the middle of the
    window, the timeline, the week's merges in one `<details>` per day, the open-issue
    count."""
    days = [(day, [subject for _, subject in same_day])
            for day, same_day in groupby(merges, key=itemgetter(0))]
    if days:
        merged = "\n".join(
            f"<details><summary>{inline(day)}: {len(subjects)} merge"
            f"{'s' if len(subjects) != 1 else ''}</summary><ul>"
            + "".join(f"<li>{inline(subject)}</li>" for subject in subjects)
            + "</ul></details>"
            for day, subjects in days)
    else:
        merged = "<p>No merges in the last 7 days.</p>"
    issues = (f"{roadmap.open_issues} open issue"
              + ("s" if roadmap.open_issues != 1 else "")
              + " in the worklog's Issues section.")

    return f"""<!doctype html>
<!-- Made by scripts/roadmap.py from docs/WORKLOG.md and git's merges: do not edit this file
     by hand, since the next `just roadmap` overwrites it. -->
<html lang="en">
<head>
<meta charset="utf-8">
<title>Unlimited Tool Works: roadmap</title>
<link rel="stylesheet" href="maintainer.css">
<style>body {{ max-width: {ROADMAP_WIDTH}px; }}</style>
</head>
<body data-page="roadmap">
{NAV}

<h1>Roadmap</h1>
<p class="intro">Made from the worklog and git's merges; regenerated with
<code>just roadmap</code>.</p>
<p><button id="to-now" type="button">Go to now</button></p>

<h2>Timeline</h2>
{timeline(roadmap, dates, today)}

<h2>Merged in the last 7 days</h2>
{merged}
<p>{issues}</p>
<script>
document.getElementById("to-now").addEventListener("click", () =>
  document.getElementById("now").scrollIntoView({{block: "center", behavior: "smooth"}}));
</script>
</body>
</html>
"""


def main() -> None:
    with open(WORKLOG, encoding="utf-8") as f:
        roadmap = parse(f.read())
    if roadmap.problems:
        print("\n".join(roadmap.problems))
        sys.exit(1)
    merges = recent_merges()
    page = render(roadmap, merges, step_dates(roadmap.steps),
                  datetime.date.today().isoformat())
    with open(OUT + ".new", "w", encoding="utf-8", newline="\n") as f:
        f.write(page)
    os.replace(OUT + ".new", OUT)
    print(f"phases {len(roadmap.phases)}, steps {len(list(walk(roadmap.steps)))}, "
          f"open issues {roadmap.open_issues}, merges {len(merges)}")


if __name__ == "__main__":
    main()
