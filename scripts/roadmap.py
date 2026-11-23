"""Writes `docs/maintainer/roadmap.html`: the maintainer's at-a-glance page of what is in
flight, what is next and what is later, made from `docs/WORKLOG.md` and the git log's recent
merges so it cannot drift from them. Run from the repository root (`just roadmap`) after the
worklog changes; the page is never written by hand. A sample for the maintainer's look for
now: wiring it into `scripts/commit.py` waits on that look."""
import html
import os
import re
import subprocess
from dataclasses import dataclass, field
from itertools import groupby
from operator import itemgetter

WORKLOG = "docs/WORKLOG.md"
OUT = "docs/maintainer/roadmap.html"

BULLET = re.compile(r"^\s*-\s+")
CHECKBOX = re.compile(r"^\s*- \[(?P<mark>[ x~!])\]\s*")
# A numbered item of "Where the work stands" (`6. The Compiler's view…`), always at column 0.
NUMBERED = re.compile(r"^\d+\.\s+")
# A run-in bold opening a paragraph (`**Working mode.** The lead plans…`).
RUNIN = re.compile(r"^\*\*(?P<bold>[^*]+?)\*\*")
OPEN_MARKS = " ~!"


@dataclass
class Roadmap:
    """What the page shows: `now` one line per thread in flight, `next` (group, text) per
    open checkbox of "Where the work stands", `later` (phase, scope, status) per unfinished
    row of the phases table, and `open_issues` the count of open lines under "Issues"."""
    now: list[str] = field(default_factory=list)
    next: list[tuple[str, str]] = field(default_factory=list)
    later: list[tuple[str, str, str]] = field(default_factory=list)
    open_issues: int = 0


def heading(lines: list[str], title: str) -> int:
    """The index of the line that is `title`."""
    return next(i for i, line in enumerate(lines) if line.strip() == title)


def joined(block: list[str], index: int, start: int) -> str:
    """The text of the item at `index` from `start`, joined with its continuation lines:
    the following lines indented deeper than its own, until a blank line or one at the same
    or a lower indentation."""
    indent = len(block[index]) - len(block[index].lstrip())
    parts = [block[index][start:].strip()]
    for line in block[index + 1:]:
        if not line.strip() or len(line) - len(line.lstrip()) <= indent:
            break
        parts.append(line.strip())
    return " ".join(parts)


def group(block: list[str], index: int) -> str:
    """The group the checkbox at `index` shows under: the nearest numbered item above it,
    or the nearest run-in bold paragraph when that one is nearer; "" when neither is."""
    for back in range(index - 1, -1, -1):
        line = block[back]
        numbered = NUMBERED.match(line)
        if numbered:
            return headline(joined(block, back, numbered.end()))
        runin = RUNIN.match(line)
        if runin:
            return runin["bold"].removesuffix(".").removesuffix(":")
    return ""


def parse(text: str) -> Roadmap:
    """The worklog's parts the page shows, as a `Roadmap`."""
    lines = text.splitlines()
    roadmap = Roadmap()

    start = heading(lines, "### Now")
    index = start + 1
    while index < len(lines) and not lines[index].startswith("#"):
        bullet = BULLET.match(lines[index])
        if bullet:
            roadmap.now.append(joined(lines, index, bullet.end()))
        index += 1

    start = heading(lines, "### Where the work stands")
    index = start + 1
    block = []
    while index < len(lines) and not lines[index].startswith(("## ", "---")):
        block.append(lines[index])
        index += 1
    for index, line in enumerate(block):
        checkbox = CHECKBOX.match(line)
        if checkbox and checkbox["mark"] in OPEN_MARKS:
            roadmap.next.append((group(block, index),
                                 headline(joined(block, index, checkbox.end()))))

    start = heading(lines, "## Phases")
    index = start + 1
    while index < len(lines) and not lines[index].startswith("## "):
        line = lines[index]
        if line.startswith("|"):
            cells = [cell.strip() for cell in line.split("|")[1:-1]]
            if len(cells) >= 4 and cells[0] not in ("Phase", "---") and cells[3] != "done":
                roadmap.later.append((cells[0], cells[1], cells[3]))
        index += 1

    start = heading(lines, "## Issues")
    index = start + 1
    while index < len(lines) and not lines[index].startswith("## "):
        if lines[index].startswith("- open"):
            roadmap.open_issues += 1
        index += 1

    return roadmap


def headline(text: str) -> str:
    """`text` up to its first `: ` or `. ` outside backticks, double quotes and parentheses,
    without the separator; the whole text when there is none. A cut inside one of those
    could leave half a section pointer (`` `plans/x.md` "Phase 8: GUI" ``), which the
    doc-pointer gate reads from the page and fails, and a cut inside parentheses leaves a
    card ending inside an open one."""
    in_code = in_quote = False
    depth = 0
    for index, char in enumerate(text):
        if char == "`" and not in_quote:
            in_code = not in_code
        elif char == '"' and not in_code:
            in_quote = not in_quote
        elif char == "(" and not in_code and not in_quote:
            depth += 1
        elif char == ")" and depth and not in_code and not in_quote:
            depth -= 1
        elif (char in ":." and not depth and not in_code and not in_quote
              and text[index + 1:index + 2] == " "):
            return text[:index]
    return text


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


STYLE = """\
.cols { display: flex; gap: 14px; align-items: flex-start; }
.col { flex: 1; min-width: 0; }
.card { border: 1px solid #383838; border-radius: 6px; padding: 10px 12px;
  margin: 10px 0; background: #222; }
.card p { margin: 0 0 6px; }
.card p:last-child { margin-bottom: 0; }
.group { color: #999; font-size: 13px; margin: 12px 0 -4px; }
"""

NAV = """\
<nav class="pages">
  <a class="here" href="roadmap.html">Roadmap</a>
  <a href="questions.html">Questions</a>
  <a href="proposals.html">Proposals</a>
  <a href="tasks.html">Tasks</a>
  <a href="ui_decisions.html">UI decisions</a>
</nav>"""


def render(roadmap: Roadmap, merges: list[tuple[str, str]]) -> str:
    """The page: three columns (Now, Next, Later), the week's merges in one `<details>`
    per day, the open-issue count."""
    now = "\n".join(f'<div class="card">{inline(item)}</div>' for item in roadmap.now)

    nexts = []
    last_group = None
    for item_group, text in roadmap.next:
        if item_group != last_group:
            last_group = item_group
            if last_group:
                nexts.append(f'<p class="group">{inline(last_group)}</p>')
        nexts.append(f'<div class="card">{inline(text)}</div>')
    nexts = "\n".join(nexts)

    later = "\n".join(
        f'<div class="card"><h3>Phase {inline(phase)}</h3>'
        f'<p>{inline(scope)}</p><p class="from">{inline(status)}</p></div>'
        for phase, scope, status in roadmap.later)

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
<style>
{STYLE}</style>
</head>
<body data-page="roadmap">
{NAV}

<h1>Roadmap</h1>
<p class="intro">Made from the worklog and git's merges; regenerated with
<code>just roadmap</code>.</p>

<div class="cols">
<div class="col">
<h2>Now</h2>
{now}
</div>
<div class="col">
<h2>Next</h2>
{nexts}
</div>
<div class="col">
<h2>Later</h2>
{later}
</div>
</div>

<h2>Merged in the last 7 days</h2>
{merged}
<p>{issues}</p>
</body>
</html>
"""


def main() -> None:
    with open(WORKLOG, encoding="utf-8") as f:
        roadmap = parse(f.read())
    merges = recent_merges()
    page = render(roadmap, merges)
    with open(OUT + ".new", "w", encoding="utf-8", newline="\n") as f:
        f.write(page)
    os.replace(OUT + ".new", OUT)
    print(f"now {len(roadmap.now)}, next {len(roadmap.next)}, later {len(roadmap.later)}, "
          f"open issues {roadmap.open_issues}, merges {len(merges)}")


if __name__ == "__main__":
    main()
