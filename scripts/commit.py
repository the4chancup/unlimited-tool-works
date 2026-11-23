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
command makes the agent harness ask the maintainer for permission. The message file is read
from where the script was run, while paths name the worktree's files.

A message file written under `FRESH_SECONDS` ago is refused too: only the command that
commits, or one sent beside it, could have written it, so it skipped the three readings every
message gets (`~/.claude/rules/shakespeare.md`) between being written and being committed.

With no paths and no `-a`, git commits the index alone; the script then names any tracked
file left changed and unstaged.

A commit that changes a file under `docs/prototypes/` first runs the drafts bundler's test
(`scripts/drafts/bundle_drafts_test.py`, under a second) and commits nothing when it fails:
a draft is pushed for the maintainer to open on the project's pages, often before a
checkpoint, and a template-literal image path once reached CI that way. A commit that
changes a `.md` or `.html` file runs the doc-pointer gate (`scripts/gates/doc_pointers.py`,
under a second) the same way: a broken pointer otherwise surfaces only at the next
checkpoint, which then fails on a docs commit long since made. A commit that changes a `.rs`,
`.toml` or `.py` file runs the comment gate (`scripts/gates/comment_refs.py`, a few seconds)
likewise: a test fixture's `# Worklog` line, read as a comment, once failed `just gates` on
main from a merge until a slice's review found it.

Lead, sidekick, reviewer or script, nobody runs `git commit` or `git merge` with a message of
their own any other way: a script that worded its own once made 49 commits no reader of the
log can use. `just gates-changed` checks every commit since its base, which catches one made
around this file before it is pushed.
"""

import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "gates"))
from commit_message import problems

# A message file younger than this was written by the command that commits it, or by one
# sent beside it; one written in the call before, and read, was measured at about 3 s, so
# a 5 s bar refused ordinary commits too.
FRESH_SECONDS = 2


def main(argv: list[str]) -> int:
    start = Path.cwd()
    if argv[:1] == ["--in"] and len(argv) > 2:
        os.chdir(argv[1])
        argv = argv[2:]
    merge = None
    if argv[:1] == ["--merge"] and len(argv) == 3:
        merge, argv = argv[1], argv[2:]
    if not argv or argv[0].startswith("-"):
        print(__doc__)
        return 2
    message, rest = start / argv[0], argv[1:]
    # The Shakespeare readings come between writing a text and committing it, so a message
    # written by this command, or one beside it, skipped them.
    age = time.time() - message.stat().st_mtime
    if age < FRESH_SECONDS:
        print(f"commit: nothing committed; {message} was written {age:.1f} s ago, by this "
              "command or one sent with it: read it three times (the Shakespeare rule), then "
              "commit")
        return 1
    found = problems(message.read_text(encoding="utf-8"))
    for problem in found:
        print(f"commit: {problem}")
    if found:
        print(f"commit: nothing committed; fix {message} (docs/CONTRIBUTING.md \"Commits\")")
        return 1
    if merge is not None:
        # A merge with nothing to bring in is a slip: run in the branch's own worktree (a `cd`
        # left behind), or the commit it follows failed. git only says "Already up to date".
        contained = ["git", "merge-base", "--is-ancestor", merge, "HEAD"]
        if subprocess.run(contained).returncode == 0:
            print(f"commit: nothing merged; {merge} is already in {Path.cwd()}'s HEAD: merge "
                  "from the tree that lacks it, after the branch's commit")
            return 1
        return subprocess.run(["git", "merge", "--no-ff", merge, "-F", str(message)]).returncode
    add_new(rest)
    changed = changed_paths(rest)
    drafts = any(path.startswith("docs/prototypes/") for path in changed)
    if drafts and not passes("scripts/drafts/bundle_drafts_test.py"):
        print("commit: nothing committed; the drafts bundler's test failed (above)")
        return 1
    docs = any(path.endswith((".md", ".html")) for path in changed)
    if docs and not passes("scripts/gates/doc_pointers.py"):
        print("commit: nothing committed; a doc pointer is broken (above)")
        return 1
    sources = any(path.endswith((".rs", ".toml", ".py")) for path in changed)
    if sources and not passes("scripts/gates/comment_refs.py"):
        print("commit: nothing committed; a comment points at the project's history (above)")
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


def add_new(rest: list[str]) -> None:
    """Stages the untracked files among the paths `rest` names: `git commit <paths>` refuses
    a path git does not know ("did not match any file"), so a new file named beside changed
    ones failed the whole commit. An ignored file is not listed as untracked, and stays
    refused."""
    paths = [arg for arg in rest if not arg.startswith("-")]
    if not paths:
        return
    new = subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "--", *paths],
        capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    if new:
        subprocess.run(["git", "add", "--", *new], check=True)


def changed_paths(rest: list[str]) -> list[str]:
    """The files the commit `rest` describes changes: the paths it names (or, with none,
    what is staged) against HEAD, `-a` taking every tracked change."""
    paths = [arg for arg in rest if not arg.startswith("-")]
    if paths or "-a" in rest:
        command = ["git", "diff", "--name-only", "HEAD", "--", *paths]
    else:
        command = ["git", "diff", "--cached", "--name-only"]
    return subprocess.run(command, capture_output=True, text=True, check=True).stdout.splitlines()


def passes(script: str) -> bool:
    """Runs a check script of the tree being committed; whether it passed."""
    root = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()
    # The tree being committed, which `--in` may make another worktree than this file's.
    return subprocess.run([sys.executable, str(Path(root) / script)]).returncode == 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
