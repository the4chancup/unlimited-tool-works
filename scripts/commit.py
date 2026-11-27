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

The message, and the lines the commit adds, are searched for phrases saying where the
maintainer is, when they are free or that they are ill (`PERSONAL`), which no tracked file
holds: each match is printed as a warning, and the commit goes on, since a match may be about
something else.

A message file written under `FRESH_SECONDS` ago is refused too: only the command that
commits, or one sent beside it, could have written it, so it skipped the three readings every
message gets (`~/.claude/rules/shakespeare.md`) between being written and being committed.

With no paths and no `-a`, git commits the index alone; the script then names any tracked
file left changed and unstaged. With `-a`, which takes tracked files alone, it commits
nothing while a file is untracked and not ignored, and names it: a new file the commit
needed was once left out that way, and found only after the commit.

A commit that changes a file under `docs/prototypes/` first runs the drafts bundler's test
(`scripts/drafts/bundle_drafts_test.py`, under a second) and commits nothing when it fails:
a draft is pushed for the maintainer to open on the project's pages, often before a
checkpoint, and a template-literal image path once reached CI that way. A commit that
changes a `.md` or `.html` file runs the doc-pointer gate (`scripts/gates/doc_pointers.py`,
under a second) the same way: a broken pointer otherwise surfaces only at the next
checkpoint, which then fails on a docs commit long since made. A commit that changes a `.rs`,
`.toml` or `.py` file runs the comment gate (`scripts/gates/comment_refs.py`, a few seconds)
likewise: a test fixture's `# Worklog` line, read as a comment, once failed `just gates` on
main from a merge until a slice's review found it. A commit that changes a `.rs` file runs
rustfmt in check mode on the changed ones (`formatted`) and commits nothing when it would
change one: an import out of order once passed the tests and clippy, and failed only the
checkpoint's format gate after the commit. A commit (not a merge) that changes
`docs/WORKLOG.md` runs `scripts/pages/roadmap.py` (under a second, before the doc-pointer gate,
which then checks the page's links too), which refuses a worklog item breaking the
convention its header states (otherwise found only by the next checkpoint's test), and
writes the roadmap page, which goes into the same commit: rebuilt by hand, the page fell
behind the worklog. A commit that changes `docs/maintainer/questions.html`,
`proposals.html`, `tasks.html` or `ui_decisions.html` runs `scripts/pages/inbox.py` the
same way and puts the rebuilt inbox page in the commit. After the commit it prints the
lines the scripted pages changed apart from the other files' (`report_scripted`),
which git's summary counts together. A merge rebuilds nothing; the lead's worklog
commit after it does.

Lead, sidekick, reviewer or script, nobody runs `git commit` or `git merge` with a message of
their own any other way: a script that worded its own once made 49 commits no reader of the
log can use. `just gates-changed` checks every commit since its base, which catches one made
around this file before it is pushed.
"""

import os
import re
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

# The workspace's `edition` (`Cargo.toml`, `[workspace.package]`), which rustfmt run on a
# file alone cannot read: without it, rustfmt parses as Rust 2015.
EDITION = "2024"

# The roadmap page, and the same by a path from the repository's root whatever the directory
# run from.
ROADMAP_FILE = "docs/maintainer/roadmap.html"
ROADMAP_PAGE = f":/{ROADMAP_FILE}"

# The inbox page, likewise, and the card pages whose change rebuilds it.
INBOX_FILE = "docs/maintainer/inbox.html"
INBOX_PAGE = f":/{INBOX_FILE}"
CARD_PAGES = ("docs/maintainer/questions.html", "docs/maintainer/proposals.html",
              "docs/maintainer/tasks.html", "docs/maintainer/ui_decisions.html")

# Phrases saying where the maintainer is, when they are free or that they are ill, which no
# tracked file or message holds (docs/CONTRIBUTING.md "Commits"). Another wording slips past
# them and a match can be about something else, so they warn and the commit goes on.
PERSONAL = re.compile(
    r"\b(?:maintainer|user)(?:'s|'re| is| was| are| will be) (?:away|back|off|sick|ill"
    r"|unavailable|travel+ing|on (?:holiday|vacation|leave))\b"
    r"|\b(?:away from|back at) (?:this|the|their) (?:pc|computer|desk|machine)\b"
    r"|\baway (?:from \d{4}-\d{2}-\d{2} )?for (?:a|an|one|two|three|four|five|six|seven|\d+)"
    r" (?:days?|weeks?|months?)\b"
    r"|\b(?:on holiday|on vacation|out of office|out of town|sick leave|days off|time off)\b",
    re.IGNORECASE,
)


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
    warn_personal(message.read_text(encoding="utf-8"), "the message")
    if merge is not None:
        # A merge with nothing to bring in is a slip: run in the branch's own worktree (a `cd`
        # left behind), or the commit it follows failed. git only says "Already up to date".
        contained = ["git", "merge-base", "--is-ancestor", merge, "HEAD"]
        if subprocess.run(contained).returncode == 0:
            print(f"commit: nothing merged; {merge} is already in {Path.cwd()}'s HEAD: merge "
                  "from the tree that lacks it, after the branch's commit")
            return 1
        return subprocess.run(["git", "merge", "--no-ff", merge, "-F", str(message)]).returncode
    new = untracked([]) if "-a" in rest else []
    if new:
        print(f"commit: nothing committed; `-a` leaves out the untracked {', '.join(new)}: "
              "`git add` what the commit needs and delete the rest, then commit again")
        return 1
    add_new(rest)
    changed = changed_paths(rest)
    warn_personal(added_text(rest), "the changes")
    drafts = any(path.startswith("docs/prototypes/") for path in changed)
    if drafts and not passes("scripts/drafts/bundle_drafts_test.py"):
        print("commit: nothing committed; the drafts bundler's test failed (above)")
        return 1
    scripted = []
    if "docs/WORKLOG.md" in changed:
        if not passes("scripts/pages/roadmap.py"):
            print("commit: nothing committed; a worklog item breaks the worklog's convention (above)")
            return 1
        subprocess.run(["git", "add", "--", ROADMAP_PAGE], check=True)
        scripted.append(ROADMAP_FILE)
        if [arg for arg in rest if not arg.startswith("-")]:
            rest = [*rest, ROADMAP_PAGE]
    if any(path in CARD_PAGES for path in changed):
        if not passes("scripts/pages/inbox.py"):
            print("commit: nothing committed; the inbox page could not be rebuilt (above)")
            return 1
        subprocess.run(["git", "add", "--", INBOX_PAGE], check=True)
        scripted.append(INBOX_FILE)
        if [arg for arg in rest if not arg.startswith("-")]:
            rest = [*rest, INBOX_PAGE]
    docs = any(path.endswith((".md", ".html")) for path in changed)
    if docs and not passes("scripts/gates/doc_pointers.py"):
        print("commit: nothing committed; a doc pointer is broken (above)")
        return 1
    sources = any(path.endswith((".rs", ".toml", ".py")) for path in changed)
    if sources and not passes("scripts/gates/comment_refs.py"):
        print("commit: nothing committed; a comment points at the project's history (above)")
        return 1
    if not formatted([path for path in changed if path.endswith(".rs")]):
        print("commit: nothing committed; rustfmt would change a Rust file (above): run "
              "`just cargo fmt`, then commit")
        return 1
    code = subprocess.run(["git", "commit", "-F", str(message), *rest]).returncode
    if code == 0:
        report_left_out(rest)
        report_scripted(scripted)
    return code


def report_scripted(scripted: list[str]) -> None:
    """Prints the lines the files this command rebuilt changed in the commit just made, apart
    from the other files': git's summary counts them together, and the roadmap page alone can
    move hundreds of lines. Prints nothing when none of them changed."""
    if not scripted:
        return
    numstat = subprocess.run(["git", "show", "--numstat", "--format=", "HEAD"],
                             capture_output=True, text=True, encoding="utf-8", check=True).stdout
    rebuilt = []
    added = deleted = 0
    for row in numstat.splitlines():
        plus, minus, path = row.split("\t", 2)
        if plus == "-":  # a binary file, which has no line counts
            continue
        if path in scripted:
            rebuilt.append(f"{path} +{plus} -{minus}")
        else:
            added += int(plus)
            deleted += int(minus)
    if rebuilt:
        print(f"commit: rebuilt by script: {', '.join(rebuilt)}; the other files: "
              f"+{added} -{deleted}")


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
    new = untracked(paths)
    if new:
        subprocess.run(["git", "add", "--", *new], check=True)


def untracked(paths: list[str]) -> list[str]:
    """The files under `paths` (the whole tree when empty) git does not track and does not
    ignore."""
    return subprocess.run(
        ["git", "ls-files", "--others", "--exclude-standard", "--", *paths],
        capture_output=True, text=True, check=True,
    ).stdout.splitlines()


def changed_paths(rest: list[str]) -> list[str]:
    """The files the commit `rest` describes changes: the paths it names (or, with none,
    what is staged) against HEAD, `-a` taking every tracked change."""
    paths = [arg for arg in rest if not arg.startswith("-")]
    if paths or "-a" in rest:
        command = ["git", "diff", "--name-only", "HEAD", "--", *paths]
    else:
        command = ["git", "diff", "--cached", "--name-only"]
    return subprocess.run(command, capture_output=True, text=True, check=True).stdout.splitlines()


def added_text(rest: list[str]) -> str:
    """The lines the commit `rest` describes adds, over the same files as `changed_paths`."""
    paths = [arg for arg in rest if not arg.startswith("-")]
    if paths or "-a" in rest:
        command = ["git", "diff", "-U0", "HEAD", "--", *paths]
    else:
        command = ["git", "diff", "--cached", "-U0"]
    diff = subprocess.run(command, capture_output=True, text=True, encoding="utf-8",
                          errors="replace", check=True).stdout
    return "\n".join(line[1:] for line in diff.splitlines()
                     if line.startswith("+") and not line.startswith("+++"))


def personal_matches(text: str) -> list[str]:
    """Each `PERSONAL` match in `text`, with some words around it. The text is flattened
    first, so a sentence wrapped across lines still matches."""
    flat = " ".join(text.split())
    return [flat[max(0, m.start() - 40):m.end() + 40] for m in PERSONAL.finditer(flat)]


def warn_personal(text: str, where: str) -> None:
    for snippet in personal_matches(text):
        print(f"commit: warning: {where} may say something personal about the maintainer, "
              f"which no tracked file holds (docs/CONTRIBUTING.md \"Commits\"): ...{snippet}...")


def passes(script: str) -> bool:
    """Runs a check script of the tree being committed; whether it passed."""
    root = toplevel()
    # The tree being committed, which `--in` may make another worktree than this file's.
    # `roadmap.py` opens its input by a path relative to the root.
    command = [sys.executable, str(Path(root) / script)]
    return subprocess.run(command, cwd=root).returncode == 0


def formatted(paths: list[str]) -> bool:
    """Whether rustfmt leaves the Rust files `paths` (from the root, deleted ones skipped) as
    they are, under the repository's `rustfmt.toml`. Run from the root, so rustup takes the
    toolchain `rust-toolchain.toml` pins."""
    root = toplevel()
    present = [path for path in paths if (Path(root) / path).is_file()]
    if not present:
        return True
    command = ["rustfmt", "--check", "--edition", EDITION, "--", *present]
    return subprocess.run(command, cwd=root).returncode == 0


def toplevel() -> str:
    """The root of the tree being committed."""
    return subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True
    ).stdout.strip()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
