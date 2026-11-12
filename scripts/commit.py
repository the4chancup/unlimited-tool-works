"""The way to commit in this repository: checks the message, then commits with it.

    python scripts/commit.py <message file> [git commit arguments...]
    python scripts/commit.py --merge <branch or commit> <message file>

The message is checked by `scripts/gates/commit_message.py` (the first line's shape, type and
length, the blank line before the body, no agent credit); a message that fails is printed with
what is wrong and nothing is committed. The first form then runs `git commit -F <message file>`
with the arguments given (paths, `-a`, `--amend`); the second `git merge --no-ff <ref> -F
<message file>`, so a merge commit's message is checked like any other.

Lead, sidekick, reviewer or script, nobody runs `git commit` or `git merge` with a message of
their own any other way: a script that worded its own once made 49 commits no reader of the
log can use. `just gates-changed` checks every commit since its base, which catches one made
around this file before it is pushed.
"""

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "gates"))
from commit_message import problems


def main(argv: list[str]) -> int:
    merge = None
    if argv[:1] == ["--merge"] and len(argv) == 3:
        merge, argv = argv[1], argv[2:]
    if not argv or argv[0].startswith("-"):
        print(__doc__)
        return 2
    message, rest = Path(argv[0]), argv[1:]
    found = problems(message.read_text(encoding="utf-8"))
    for problem in found:
        print(f"commit: {problem}")
    if found:
        print(f"commit: nothing committed; fix {message} (docs/CONTRIBUTING.md \"Commits\")")
        return 1
    if merge is not None:
        return subprocess.run(["git", "merge", "--no-ff", merge, "-F", str(message)]).returncode
    return subprocess.run(["git", "commit", "-F", str(message), *rest]).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
