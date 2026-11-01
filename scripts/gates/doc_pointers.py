"""Section-pointer check: `doc_pointers.py` (in `just gates`).

A section pointer names a Markdown file and one or more places in it: `` `path.md` "Heading" ``,
with or without the backticks, further titles following after `, `, ` and ` or ` or `
(`` `object_model.md` "Core types", "Player folders" ``). A pointer the line end cuts continues
on the next line, in a `.rs` file on the next comment line. Every pointer in `AGENTS.md`, the
documents under `docs/` and the `.rs` and `.md` files under `crates/` must resolve: its file
exists and holds an anchor the title names. `DECISIONS.md` and `LOG.md` are not checked: they
are append-only, so their old pointers stay as they were written.

The documents name a place by a heading, a bold run-in title (`**Subfolders.**`, anywhere in a
line), the start of a paragraph or list item ("Rules:"), or a table row's first cell (a
glossary term), so each of these is an anchor, with its list marker, bold markers and
backticks removed, and a heading also without its number ("5. Writer" is also "Writer") or a
decision entry's date and scope ("2026-10-27 — core — A window …" is also "A window …"). A
title names an anchor equal to it or starting with it and then a space or `:`, `,`, `.`, so
"Testing" names "Testing and verification". The check is for a pointer whose text is gone
from its file (a section moved to the other tree, renamed or deleted); leniency on the exact
wording costs it nothing there.

A path resolves against, in order, the pointing file's own directory, `docs/`, the repository
root and `docs/plans/`; the first that names an existing file is the pointer's file. The last
base is how the worklog and the other working documents name a plan (`aatf_rules.md`); a
section of the spec is named with its tree (`spec/aatf_rules.md`), so a pointer never leaves
the reader to guess which tree it means. A path that resolves nowhere is a problem only when
the pointer is in `docs/` or `AGENTS.md`, or names `docs/`, `plans/` or `spec/`:
`` `README.md` "Usage" `` in a crate is about that crate.

A pointer never names a file in the scratch folder `.tmp/`, wherever it is, even one that
exists: `.tmp/` is gitignored, so its files exist on the machine that wrote them and in no
clone, and a pointer into one would pass there and fail in CI. A scratch file is named by its
path alone, its place in words without the quotes (`` `.tmp/release_0_0_1/0_0_1s/tac_rulings.md`, round A3 ``).

Code points at the spec alone: a pointer in `crates/` that resolves into `docs/plans/` is a
problem, since a plan describes only what code does not do yet. The one exception is the open
phase, whose sections stay in the plans until it closes although its code is being written:
`OPEN_PHASE_PLANS` lists their files, and the phase's close, which moves the sections, empties it.

A relative Markdown link to a `.md` file (`[core plan](core/README.md)`) must name a file that
exists beside the linking one; its `#anchor` is not checked.
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

TITLE = r'"[^"\n]{1,120}"'
PATH = r"(?<![\w/.])`?(?P<path>[\w./-]+\.md)`?[,;]?"
TITLES = rf"{TITLE}(?:,? (?:and |or )?{TITLE})*"
POINTER = re.compile(rf"{PATH} (?P<titles>{TITLES})")
# A pointer the line end cuts: its path alone, its titles so far followed by the `,`, `and` or
# `or` that joins the next one, or a title whose closing quote is on a later line. A complete
# pointer ending the line is not cut: joining a next line that opens with a quoted phrase would
# read that phrase as one more title.
CUT = re.compile(rf"{PATH}(?: {TITLES}(?:,|,? and|,? or))?(?: \"[^\"]*)?$")
COMMENT = re.compile(r"\s*//[/!]?\s*")
QUOTED = re.compile(TITLE)
HEADING = re.compile(r"^#+\s+(?P<title>.+?)\s*#*\s*$")
NUMBER = re.compile(r"^\d+\.\s+")
# A decision entry's heading opens with its date and scope; a pointer names it by its title.
DECISION = re.compile(r"^\d{4}-\d{2}-\d{2} — [^—]+ — ")
LIST_MARKER = re.compile(r"^\s*(?:[-*]|\d+\.)\s+")
BOLD = re.compile(r"\*\*(?P<title>[^*]+?)\*\*")
LINK = re.compile(r"\]\((?P<target>[^)\s#:]+\.md)(?:#[^)]*)?\)")
SKIPPED = {"docs/DECISIONS.md", "docs/LOG.md"}
# The plans code may still point at: each holds sections already built whose phase is still
# open, and they move to the spec when it closes.
OPEN_PHASE_PLANS = {
    # `libs/music_export` is built ahead of the Music player; its format section is here.
    "docs/plans/music_player.md",
    # `color_tools`' picker widget and the Save editor's `team_widgets`.
    "docs/plans/libs/color_tools.md",
    "docs/plans/save_editor.md",
    # The Ruleset editor.
    "docs/plans/ruleset_editor.md",
}


@dataclass(frozen=True)
class Pointer:
    source: str
    line: int
    path: str
    title: str


def continuation(line: str, code: bool) -> str | None:
    """The text a wrapped pointer continues with on `line`, or None when it cannot continue
    there: a blank line, or in a `.rs` file a line that is not a comment."""
    if code:
        comment = COMMENT.match(line)
        return line[comment.end() :].rstrip() if comment else None
    return line.strip() or None


def pointers(source: str, text: str) -> list[Pointer]:
    code = source.endswith(".rs")
    lines = text.splitlines()
    found = []
    for index, line in enumerate(lines):
        # A pointer may wrap onto the next lines; it is read whole, from the line it starts on.
        joined = line.rstrip()
        following = index + 1
        while following < len(lines) and CUT.search(joined):
            more = continuation(lines[following], code)
            if more is None:
                break
            joined += " " + more
            following += 1
        for match in POINTER.finditer(joined):
            if match.start() >= len(line):
                break
            for quoted in QUOTED.findall(match["titles"]):
                found.append(Pointer(source, index + 1, match["path"], quoted[1:-1]))
    return found


def clean(text: str) -> str:
    return text.replace("`", "").replace("**", "").strip()


def anchors(text: str) -> set[str]:
    found = set()
    in_fence = False
    previous = ""
    for line in text.splitlines():
        if line.startswith("```"):
            in_fence = not in_fence
        elif not in_fence:
            heading = HEADING.match(line)
            if heading:
                title = clean(heading["title"])
                found |= {title, NUMBER.sub("", title), DECISION.sub("", title)}
            elif line.startswith("|"):
                found.add(clean(line.split("|")[1]))
            elif LIST_MARKER.match(line) or not previous.strip() or HEADING.match(previous):
                found.add(clean(LIST_MARKER.sub("", line)))
            found |= {clean(bold["title"]) for bold in BOLD.finditer(line)}
        previous = line
    return found


def names_anchor(found: set[str], title: str) -> bool:
    title = clean(title)
    return title in found or any(
        anchor.startswith((title + " ", title + ":", title + ",", title + ".")) for anchor in found
    )


def resolve(root: Path, source: str, path: str) -> Path | None:
    bases = [(root / source).parent, root / "docs", root, root / "docs" / "plans"]
    for base in bases:
        candidate = (base / path).resolve()
        if candidate.is_file():
            return candidate
    return None


def problems(root: Path, found: list[Pointer]) -> list[str]:
    # `resolve` returns resolved paths, so the root must be resolved too: a Windows temp folder
    # can come back in its 8.3 short form (`RUNNER~1` on CI), which `relative_to` does not match.
    root = root.resolve()
    cache: dict[Path, set[str]] = {}
    out = []
    for pointer in found:
        if pointer.path.startswith(".tmp/"):
            out.append(f"{pointer.source}:{pointer.line}: `{pointer.path}` points into the scratch folder, which no clone has")
            continue
        target = resolve(root, pointer.source, pointer.path)
        where = f"{pointer.source}:{pointer.line}"
        if target is None:
            in_docs = pointer.source.startswith("docs/") or pointer.source == "AGENTS.md"
            if in_docs or pointer.path.startswith(("docs/", "plans/", "spec/")):
                out.append(f"{where}: `{pointer.path}` names no file")
            continue
        if target not in cache:
            cache[target] = anchors(target.read_text(encoding="utf-8"))
        shown = target.relative_to(root).as_posix()
        if not names_anchor(cache[target], pointer.title):
            out.append(f'{where}: {shown} has no "{pointer.title}"')
        elif (
            pointer.source.startswith("crates/")
            and shown.startswith("docs/plans/")
            and shown not in OPEN_PHASE_PLANS
        ):
            out.append(f'{where}: code points at the plan {shown} "{pointer.title}"')
    return out


def broken_links(root: Path, source: str, text: str) -> list[str]:
    out = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in LINK.finditer(line):
            if not ((root / source).parent / match["target"]).resolve().is_file():
                out.append(f"{source}:{number}: link to `{match['target']}` names no file")
    return out


def sources(root: Path) -> list[Path]:
    files = [root / "AGENTS.md"]
    files += sorted((root / "docs").rglob("*.md"))
    files += sorted((root / "crates").rglob("*.rs"))
    files += sorted((root / "crates").rglob("*.md"))
    return [f for f in files if f.relative_to(root).as_posix() not in SKIPPED and "target" not in f.parts]


def main() -> int:
    found = []
    links = []
    for file in sources(ROOT):
        source, text = file.relative_to(ROOT).as_posix(), file.read_text(encoding="utf-8")
        found += pointers(source, text)
        links += broken_links(ROOT, source, text)
    broken = problems(ROOT, found) + links
    for line in broken:
        print(line)
    print(f"{len(found)} section pointers and the links, {len(broken)} broken")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
