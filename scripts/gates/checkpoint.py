"""`just checkpoint` and `just checkpoint-status`: the one command a checkpoint is.

A checkpoint (CONTRIBUTING.md "Testing and verification": the end of a review loop, a worklog
step's close, a merge, before a push is asked for) used to be two commands and a hand-typed
note of the last commit that passed, and it was skipped: three review loops closed on
2026-11-11 with neither run, and a push went red on CI. Here it is one command, and the last
gated commit is a git ref the command moves itself, `refs/checkpoints/gated`, so nobody types
it and anything can ask how far HEAD has run past it.

`run` needs a clean tree (the ref names a commit, so what is gated must be one): it runs
`just gates-changed <gated>`, then `just mutants-diff <gated>` (skipped by `--no-mutants`),
and moves the ref to HEAD when the gates pass. Surviving mutants do not stop the ref: they
are assertions nobody wrote, listed in `mutants.diff/missed_all.txt` for triage.

`status` prints the gated commit, how many commits HEAD is past it (docs-only ones apart),
how many commits are not pushed, and what CI made of the pushed head. With
`--max-ungated N` it exits 1 when more than N commits that touch code are past the gate.
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REF = "refs/checkpoints/gated"


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def docs_only(paths: list[str]) -> bool:
    """Whether a commit touching `paths` changes no code: every path is under `docs/` or is a
    root-level `.md` file. A commit touching nothing (a merge) is not docs-only: what it
    brings in is its parents' business, counted with them."""
    return bool(paths) and all(p.startswith("docs/") or ("/" not in p and p.endswith(".md")) for p in paths)


def gated() -> str:
    found = subprocess.run(["git", "rev-parse", "--verify", "--quiet", REF], cwd=ROOT, capture_output=True, text=True)
    if found.returncode != 0:
        sys.exit(f"no {REF}: seed it with the last commit the gates passed at, `git update-ref {REF} <sha>`")
    return found.stdout.strip()


def commits_past(base: str) -> tuple[int, int]:
    """The commits of `base..HEAD`: how many touch code, how many are docs-only."""
    code = docs = 0
    for sha in git("rev-list", "--no-merges", f"{base}..HEAD").split():
        paths = git("show", "--format=", "--name-only", sha).splitlines()
        if docs_only(paths):
            docs += 1
        else:
            code += 1
    return code, docs


def ci_of(sha: str) -> str:
    """What the CI workflow made of `sha`, in a few words; never an error: this is a status line."""
    try:
        listed = subprocess.run(
            ["gh", "run", "list", "--branch", "main", "--limit", "20", "--json", "headSha,status,conclusion,workflowName"],
            cwd=ROOT, capture_output=True, text=True, timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"unknown ({error})"
    if listed.returncode != 0:
        return f"unknown (gh: {listed.stderr.strip()[:80]})"
    for run in json.loads(listed.stdout):
        if run["workflowName"] == "CI" and run["headSha"] == sha:
            return run["conclusion"] or run["status"]
    return "no run found"


def status(max_ungated: int | None) -> int:
    base = gated()
    code, docs = commits_past(base)
    print(f"gated: {git('log', '-1', '--format=%h %s', '--abbrev=8', base)}")
    print(f"past the gate: {code} commits touching code, {docs} docs-only")
    pushed = subprocess.run(["git", "rev-parse", "--verify", "--quiet", "origin/main"], cwd=ROOT, capture_output=True, text=True)
    if pushed.returncode == 0:
        sha = pushed.stdout.strip()
        print(f"not pushed: {git('rev-list', '--count', f'{sha}..HEAD')} commits; CI on the pushed {sha[:8]}: {ci_of(sha)}")
    if max_ungated is not None and code > max_ungated:
        print(f"CHECKPOINT DUE: {code} code commits past the gate (more than {max_ungated}); run `just checkpoint`")
        return 1
    return 0


def run(mutants: bool) -> int:
    if git("status", "--porcelain"):
        sys.exit("the tree is not clean: commit first, a checkpoint gates a commit")
    base = gated()
    # The commit the gates read, taken now: a commit made while they run is not gated by them.
    head = git("rev-parse", "HEAD")
    if base == head:
        print("HEAD is the gated commit already")
        return status(None)
    gates = subprocess.run(["just", "gates-changed", base], cwd=ROOT)
    if gates.returncode != 0:
        print(f"the gates failed: {REF} stays at {base[:8]}")
        return gates.returncode
    print(f"the gates passed on {head[:8]}", flush=True)
    if mutants:
        diff = subprocess.run(["just", "mutants-diff", base], cwd=ROOT)
        print(f"mutants-diff ended with exit code {diff.returncode}; its survivors, if any, are in mutants.diff/missed_all.txt")
    git("update-ref", REF, head)
    return status(None)


def main(argv: list[str]) -> int:
    match argv[1:]:
        case ["status"]:
            return status(None)
        case ["status", "--max-ungated", limit] if limit.isdigit():
            return status(int(limit))
        case ["run"]:
            return run(mutants=True)
        case ["run", "--no-mutants"]:
            return run(mutants=False)
    sys.exit("usage: checkpoint.py status [--max-ungated N] | run [--no-mutants]")


if __name__ == "__main__":
    sys.exit(main(sys.argv))
