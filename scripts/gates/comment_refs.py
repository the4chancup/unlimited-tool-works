"""List code comments that point at the project's history instead of standing on their own.

    python scripts/gates/comment_refs.py [--count]

CONTRIBUTING.md "Code style", "A comment stands on its own": a comment never cites a plan,
DECISIONS.md, the worklog or the log, a step, a review round or concern, an agent, or who
decided and when. This
script reads the comments of every tracked Rust, TOML and Python file (Rust `//`, `///`,
`//!`; TOML and Python `#`) and prints `path:line: kind: text` for each one matching a
pattern below. `--count` prints the number of hits per top-level folder instead. Exit 1 when
anything matches, 0 when nothing does.

The patterns aim at what the rule names, not at every use of the words: "plan" is matched as
a word, so "planned" and "plane" pass; a step is a number, a dot, a number and a letter
(`5.6e`, `0.0.1m2`), so a version (`0.37.0`) or an exponent (`1.3e-5`) passes; a Markdown
file is matched unless its path starts at `spec/`.
"""

import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# Each kind of pointer the rule forbids, as one pattern.
PATTERNS = {
    "plan": re.compile(r"\bplans/|\b(?:core|the|tool's|its|this) plan\b|\bplan's\b|\bPhase \d", re.I),
    "decisions": re.compile(r"DECISIONS"),
    "worklog": re.compile(r"\bWORKLOG\b|\bLOG\.md\b|\bworklog\b", re.I),
    # A letter after the number (`5.6e`, `0.0.1m2`, `3.w`), not a signed exponent (`1.3e-5`); a
    # bare number only where the words make it a step ("step 4.18", "3.7's"), since "1.5 points"
    # cannot be told apart from "at 3.8" otherwise. A letter right after the dot or a possessive
    # needs a first number from 1, so versions pass (`0.y.z`, "egui 0.37's").
    "step": re.compile(
        r"\b\d+\.\d+(?:\.\d+)?(?!e[-+]\d)[a-z]\d*\b"
        r"|\b[1-9]\d*\.[a-z]\b"
        r"|\b[Ss]tep \d+\.\d+"
        r"|(?<![\w.])[1-9]\d*\.\d+'s\b"
    ),
    # A commit a review or a mutation run was made at.
    "history": re.compile(r"\bmutants-diff [0-9a-f]{7,40}\b"),
    # A Markdown file other than a spec file, the two tooling documents (what a tooling file's
    # comment may point at) or a fixture folder's README (its provenance): a plan, the worklog.
    "document": re.compile(
        r"(?<![\w./-])"
        r"(?!(?:docs/)?(?:spec/|CONTRIBUTING\.md|RUNBOOK\.md)|[\w./-]*fixtures/[\w./-]*README\.md)"
        r"[\w./-]*\.md\b"
    ),
    "review": re.compile(
        r"\b[SAR]\d+[ab]?-\d+\b|\bround [SA]\d+\b|\bAstra\b|\bSWE-2\b|\bsidekick\b"
        r"|\b(?:the|this|its|a) brief\b|\bbrief's\b",
        re.I,
    ),
    # Who decided and when: "maintainer" with a date or a deciding verb after it. Anchored on
    # the one word, since a wrapped comment leaves "the" on the line before; "the maintainer's
    # save" or "desk" passes, naming where a value came from rather than who ruled.
    "attribution": re.compile(
        r"\bmaintainer(?:'s)?,? (?:20\d\d-\d\d-\d\d\b|(?:asked|chose|decided|rejected|ruled|wanted)\b)",
        re.I,
    ),
}

# Files the rule does not apply to: this script and its test name the patterns themselves.
SKIPPED = {"scripts/gates/comment_refs.py", "scripts/gates/comment_refs_test.py"}


def comment_of(line: str, suffix: str) -> str | None:
    """The comment text of `line` (a file of type `suffix`), or None when it holds none.

    Only whole-line comments are read: a trailing comment after code is rare in this
    repository, and finding one safely would need each language's string rules."""
    stripped = line.strip()
    if suffix == ".rs":
        return stripped if stripped.startswith("//") else None
    return stripped if stripped.startswith("#") else None


def hits_in(path: str, text: str) -> list[tuple[int, str, str]]:
    """Every (line number, kind, comment) of `text` (the file at `path`) that a pattern matches;
    a comment matching two kinds is listed once, under the first."""
    suffix = Path(path).suffix
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        comment = comment_of(line, suffix)
        if comment is None:
            continue
        for kind, pattern in PATTERNS.items():
            if pattern.search(comment):
                found.append((number, kind, comment))
                break
    return found


def tracked_files() -> list[str]:
    """The tracked Rust, TOML and Python files, repository-relative with forward slashes."""
    listed = subprocess.run(
        ["git", "ls-files", "*.rs", "*.toml", "*.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split("\n")
    return [path for path in listed if path and path not in SKIPPED]


def main(argv: list[str]) -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    count = "--count" in argv
    per_folder: Counter[str] = Counter()
    total = 0
    for path in tracked_files():
        text = (ROOT / path).read_text(encoding="utf-8", errors="replace")
        for number, kind, comment in hits_in(path, text):
            total += 1
            parts = path.split("/")
            per_folder["/".join(parts[:3]) if parts[0] == "crates" else parts[0]] += 1
            if not count:
                print(f"{path}:{number}: {kind}: {comment}")
    if count:
        for folder, hits in per_folder.most_common():
            print(f"{hits:5} {folder}")
    print(f"{total} comments point at a plan, a decision, a step or a review")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
