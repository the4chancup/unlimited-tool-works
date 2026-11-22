"""Remove the `target/` build folder of merged, clean sidekick worktrees (regenerable).

    python scripts/clear_targets.py NAME...     NAME = a folder under 4cc-studio-temp, e.g. sk_nocup

Each NAME is checked first: its worktree's HEAD is in main or pes14, its tree is clean, and
no running process has an exe under its target. Only `<temp>/<NAME>/target` is removed. A
NAME that fails a check is skipped with one line saying why (the exit code is then 1), and
the others are still cleared.
"""
import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

import psutil

REPO = Path(__file__).resolve().parents[1]
TEMP = REPO.parent / (REPO.name + "-temp")
NEVER = {"gate", "pes14", "release_003dev1"}


def git(*args, cwd):
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)


def merged(head):
    return any(
        git("merge-base", "--is-ancestor", head, branch, cwd=REPO).returncode == 0
        for branch in ("main", "pes14")
    )


def writable(func, path, _exc):
    os.chmod(path, stat.S_IWRITE)
    func(path)


names = sys.argv[1:]
assert names, "name at least one worktree"
targets = []
refused = []
for name in names:
    assert name not in NEVER and "/" not in name and "\\" not in name and name.startswith("sk_"), name
    tree = TEMP / name
    target = tree / "target"
    if not target.exists():
        print("already clear", target)
        continue
    assert target.is_dir(), f"{target} is no folder"
    assert target.resolve().parent == tree.resolve() and tree.resolve().parent == TEMP.resolve()
    head = git("rev-parse", "HEAD", cwd=tree).stdout.strip()
    if not (head and merged(head)):
        refused.append(f"{name}: HEAD {head} is in neither main nor pes14")
        continue
    dirty = git("status", "--short", cwd=tree).stdout.splitlines()
    if dirty:
        refused.append(f"{name}: {len(dirty)} uncommitted changes (git status --short)")
        continue
    targets.append(target)

for target in list(targets):
    prefix = str(target.resolve()).lower()
    running = [
        proc.info["pid"]
        for proc in psutil.process_iter(["exe", "pid"])
        if prefix in (proc.info["exe"] or "").lower()
    ]
    if running:
        refused.append(f"{target.parent.name}: processes {running} run from its target")
        targets.remove(target)

for target in targets:
    shutil.rmtree(target, onerror=writable)
    print("removed", target)
for reason in refused:
    print("skipped", reason)
sys.exit(1 if refused else 0)
