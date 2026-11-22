"""The way to commit in this repository: checks the message, then commits with it.

    python scripts/commit.py <message file> [git commit arguments...]
    python scripts/commit.py --merge <branch or commit> <message file>
    python scripts/commit.py --in <worktree> ...    either form, run in that worktree

The message is checked by `scripts/gates/commit_message.py` (the first line's shape, type and
length, the blank line before the body, no agent credit); a message that fails is printed with
what is wrong and nothing is committed. The first form then runs `git commit -F <message file>`
with the arguments given (paths, `-a`, `--amend`); the second `git merge --no-ff <ref> -F
<message file>`, so a merge commit's message is checked like any other. `--in` runs either
in another worktree (a sidekick's under `4cc-studio-temp/`): a `cd` chained before the
command makes the agent harness ask the maintainer for permission.

With no paths and no `-a`, git commits the index alone; the script then names any tracked
file left changed and unstaged.

A commit that changes a file under `docs/prototypes/` first runs the drafts bundler's test
(`scripts/drafts/bundle_drafts_test.py`, under a second) and commits nothing when it fails:
a draft is pushed for the maintainer to open on the project's pages, often before a
checkpoint, and a template-literal image path once reached CI that way.

Lead, sidekick, reviewer or script, nobody runs `git commit` or `git merge` with a message of
their own any other way: a script that worded its own once made 49 commits no reader of the
log can use. `just gates-changed` checks every commit since its base, which catches one made
around this file before it is pushed.
"""

import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "gates"))
from commit_message import problems


def main(argv: list[str]) -> int:
    if argv[:1] == ["--in"] and len(argv) > 2:
        os.chdir(argv[1])
        argv = argv[2:]
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
    if touches_drafts(rest) and not drafts_bundle():
        print("commit: nothing committed; the drafts bundler's test failed (above)")
        return 1
    code = subprocess.run(["git", "commit", "-F", str(message), *rest]).returncode
    if code == 0:
        report_left_out(rest)
    return code


def report_left_out(rest: list[str]) -> None:
    """After a commit of the index alone (no paths, no `-a`), names the tracked files still
    changed and unstaged. Such a commit is sometimes meant, so this only reports; but an edit
    made after a `git add -A` was once merged without, the commit having taken the index."""
    if [arg for arg in rest if not arg.startswith("-")] or "-a" in rest:
        return
    left = subprocess.run(
        ["git", "diff", "--name-only"], capture_output=True, text=True, check=True
    ).stdout.splitlines()
    if left:
        print(f"commit: left out, changed but not staged: {', '.join(left)}")


def touches_drafts(rest: list[str]) -> bool:
    """Whether the commit `rest` describes changes a draft: the paths it names (or, with
    none, what is staged) against HEAD, `-a` taking every tracked change."""
    paths = [arg for arg in rest if not arg.startswith("-")]
    if paths or "-a" in rest:
        command = ["git", "diff", "--name-only", "HEAD", "--", *paths]
    else:
        command = ["git", "diff", "--cached", "--name-only"]
    changed = subprocess.run(command, capture_output=True, text=True, check=True).stdout
    return any(line.startswith("docs/prototypes/") for line in changed.splitlines())


def drafts_bundle() -> bool:
    """Runs the drafts bundler's test; whether it passed."""
    root = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()
    # The tree being committed, which `--in` may make another worktree than this file's.
    test = Path(root) / "scripts" / "drafts" / "bundle_drafts_test.py"
    return subprocess.run([sys.executable, str(test)]).returncode == 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
