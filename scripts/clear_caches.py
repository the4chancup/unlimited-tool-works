"""Remove the regenerable caches inside a worktree's `target/` while keeping its build: the
incremental caches (`<profile>/incremental`), cargo's `tmp/` and the Save editor tests' scratch
(`debug/save_editor_tests`). For the worktrees `clear_targets.py` never clears whole (the gate,
pes14, a release worktree), whose next build would otherwise start from nothing.

    python scripts/clear_caches.py [--dry-run] NAME...   NAME = a folder under 4cc-studio-temp

Each NAME is refused, with one line saying why (the exit code is then 1), while a cargo,
rustc or test process runs from its folder, since a build or test would lose files it is
writing. The next build of a cleared worktree recompiles the crates it changed in full rather
than incrementally; nothing else is lost.
"""
import os
import shutil
import stat
import sys
from pathlib import Path

import psutil

REPO = Path(__file__).resolve().parents[1]
TEMP = REPO.parent / (REPO.name + "-temp")
CACHES = ("debug/incremental", "release/incremental", "tmp", "debug/save_editor_tests")


def writable(func, path, _exc):
    os.chmod(path, stat.S_IWRITE)
    func(path)


def busy(tree):
    """A process whose executable or working folder is inside `tree`."""
    root = str(tree.resolve()).lower()
    for proc in psutil.process_iter(["exe", "cwd", "name"]):
        try:
            places = (proc.info["exe"] or "", proc.cwd() or "")
        except (psutil.AccessDenied, psutil.NoSuchProcess):
            continue
        if any(place.lower().startswith(root) for place in places):
            return proc.info["name"]
    return None


def size(folder):
    return sum(f.stat().st_size for f in folder.rglob("*") if f.is_file())


args = sys.argv[1:]
dry = "--dry-run" in args
names = [a for a in args if a != "--dry-run"]
assert names, "name at least one worktree"
refused = False
freed = 0
for name in names:
    assert "/" not in name and "\\" not in name, name
    tree = TEMP / name
    assert tree.resolve().parent == TEMP.resolve() and (tree / ".git").exists(), f"{tree} is no worktree"
    process = busy(tree)
    if process:
        print(f"skipped {name}: {process} runs from it")
        refused = True
        continue
    for cache in CACHES:
        folder = tree / "target" / cache
        if not folder.is_dir():
            continue
        bytes_ = size(folder)
        freed += bytes_
        print(f"{'would remove' if dry else 'removed'} {folder} ({bytes_ / 1e9:.1f} GB)")
        if not dry:
            shutil.rmtree(folder, onerror=writable)
print(f"{'would free' if dry else 'freed'} {freed / 1e9:.1f} GB")
sys.exit(1 if refused else 0)
