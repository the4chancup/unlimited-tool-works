"""Checks a commit message against `CONTRIBUTING.md` "Commits": what a reader of `git log`
depends on and a machine can tell.

    commit_message.py <message file>     the message about to be committed (`scripts/commit.py`)
    commit_message.py --range <base>     every commit of <base>..HEAD (`just gates-changed`)

The first line is `type(scope): summary`, its type one of TYPES, at most 120 characters, with no
full stop at its end; a body starts after one blank line; no line credits an agent. Whether the
summary says what changed, and in the right mood, stays the writer's job.

A commit made around `scripts/commit.py` (a script's own `git commit`, a merge) is still caught
by the range check before it is pushed: 49 commits of one night once went out as `docs: b07
pass 2 of ...`, no scope and a scratch name for a subject, because the script that made them
worded its own.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TYPES = ("feat", "fix", "docs", "test", "refactor", "perf", "chore", "merge")
MAX_SUBJECT = 120
SUBJECT = re.compile(r"^(?P<type>[a-z]+)\((?P<scope>[^()]+)\): (?P<summary>\S.*)$")
CREDIT = re.compile(r"^\W*(co-authored-by:|generated with )", re.IGNORECASE)


def problems(message: str) -> list[str]:
    """What is wrong with `message`, one line each; empty when it passes."""
    lines = message.splitlines()
    # Git drops the blank lines around a message read from a file; so does the check.
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    if not lines:
        return ["the message is empty"]
    subject, found = lines[0], []
    shape = SUBJECT.match(subject)
    if shape is None:
        found.append("the first line is not `type(scope): summary`")
    elif shape["type"] not in TYPES:
        found.append(f"the type `{shape['type']}` is not one of {', '.join(TYPES)}")
    if len(subject) > MAX_SUBJECT:
        found.append(f"the first line is {len(subject)} characters, over {MAX_SUBJECT}: "
                     "what does not fit belongs in the body")
    if subject.rstrip() != subject or subject.endswith("."):
        found.append("the first line ends with a full stop or a space")
    if len(lines) > 1 and lines[1].strip():
        found.append("the second line is not blank: the body starts after one empty line")
    found += [f"the line `{line.strip()}` credits an agent" for line in lines if CREDIT.match(line.strip())]
    return found


def range_problems(base: str) -> list[str]:
    """The problems of every commit in `base`..HEAD, each line led by the commit's short id."""
    out = subprocess.run(["git", "log", "--format=%h%x1f%B%x1e", f"{base}..HEAD"], cwd=ROOT,
                         check=True, capture_output=True, text=True, encoding="utf-8").stdout
    found = []
    for record in out.split("\x1e"):
        if not record.strip():
            continue
        commit, message = record.strip("\n").split("\x1f")
        first = message.splitlines()[0] if message.strip() else ""
        found += [f"{commit} ({first[:60]}): {problem}" for problem in problems(message)]
    return found


def main(argv: list[str]) -> int:
    if len(argv) == 2 and argv[0] == "--range":
        found = range_problems(argv[1])
        subject = f"commits since {argv[1]}"
    elif len(argv) == 1 and not argv[0].startswith("-"):
        found = problems(Path(argv[0]).read_text(encoding="utf-8"))
        subject = argv[0]
    else:
        print(__doc__)
        return 2
    for problem in found:
        print(f"commit_message: {problem}")
    if found:
        print(f"commit_message: {subject}: {len(found)} problem(s) (docs/CONTRIBUTING.md \"Commits\")")
        return 1
    print(f"commit_message: {subject}: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
