"""List `CHANGELOG.md` `[Unreleased]` entries about a tool no release has presented.

    python scripts/gates/changelog_tools.py

A tool whose code ships but which no release's notes and not the `README.md` have presented
gets no changelog entries: nobody has been told it exists, so its changes are noise in the
help's What's new chapter, the release notes and the update dialog, and the release that
first presents it describes it whole in one bullet (CONTRIBUTING.md "Commits"; DECISIONS
2026-11-06 "a tool no release has presented gets no entries"). Nothing else stops such an
entry: the rule every agent reads says "a change a member would notice gets its line", and
between 2026-11-08 and 2026-11-13 eleven entries for the Team compiler and the Export
upgrader landed that way.

`UNPRESENTED` names those tools as an entry's bold label spells them. An entry is a bullet
that opens with a bold label (`- **Team compiler**: ...`, `- **Team compiler and Ruleset
editor**: ...`); one whose label names an unpresented tool is listed, with its line. A tool
leaves the list in the commit whose release notes present it. Exit 1 when anything is
listed, 0 when nothing is.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The tools no release has presented, as a changelog label names them.
UNPRESENTED = ("Team compiler", "Export upgrader")

LABEL = re.compile(r"^- \*\*([^*]+)\*\*")


def unpresented_entries(changelog: str) -> list[tuple[int, str]]:
    """Each (line number, label) of an `[Unreleased]` bullet whose bold label names a tool
    of `UNPRESENTED`. The section runs from the `## [Unreleased]` heading to the next `## `
    heading; a released section's entries are history and are not read."""
    found = []
    inside = False
    for number, line in enumerate(changelog.splitlines(), start=1):
        if line.startswith("## "):
            inside = line.startswith("## [Unreleased]")
            continue
        label = LABEL.match(line) if inside else None
        if label and any(tool.lower() in label.group(1).lower() for tool in UNPRESENTED):
            found.append((number, label.group(1)))
    return found


def main() -> int:
    found = unpresented_entries((ROOT / "CHANGELOG.md").read_text(encoding="utf-8"))
    for number, label in found:
        print(f"CHANGELOG.md:{number}: an entry for {label}, which no release has presented")
    if found:
        print(
            f"{len(found)} entries for an unpresented tool (CONTRIBUTING.md \"Commits\"): "
            "remove them, or take the tool out of UNPRESENTED in the commit that presents it"
        )
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
