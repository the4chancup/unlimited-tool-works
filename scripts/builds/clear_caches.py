"""Remove the regenerable parts of a worktree's `target/` while keeping its current build: the
incremental caches (`<profile>/incremental`), cargo's `tmp/`, the tests' scratch
(`debug/*_tests`), and in each `<profile>/deps` the older builds `stale_deps.stale` finds.
For the trees `clear_targets.py` never clears whole (main, the gate, pes14, a release
worktree), whose next build would otherwise start from nothing.

    python scripts/builds/clear_caches.py [--dry-run] [--over GB] NAME...
    NAME = `main` (this repository) or a folder under 4cc-studio-temp

`--over GB` leaves a tree whose `target/` holds less than GB, counted on disk (`just
pipeline` runs this for main and the gate with `--over`, so a long-lived tree's `target/`
stops growing without a cold build). Each NAME is refused, with one
line saying why (the exit code is then 1), while a build or test runs in its folder, since
it would lose files it is writing; for that reason the older builds go whatever their age. The next build of a cleared tree recompiles the crates it changed in full rather
than incrementally, and an older build still in use (another feature set of a crate) once.
"""
import os
import shutil
import stat
import sys
from pathlib import Path

import psutil

from stale_deps import stale

REPO = Path(__file__).resolve().parents[2]
TEMP = REPO.parent / (REPO.name + "-temp")
CACHES = ("debug/incremental", "release/incremental", "tmp")
SCRATCH_SUFFIX = "_tests"
PROFILES = ("debug", "release")
BUILDERS = {"cargo", "rustc", "clippy-driver", "cargo-nextest", "cargo-mutants", "link", "rust-lld"}


def writable(func, path, _exc):
    os.chmod(path, stat.S_IWRITE)
    func(path)


def busy(tree: Path) -> str | None:
    """A build or test in `tree`: a program run from inside it (a test binary), or one of
    `BUILDERS` working in it. Any process working in main would not do: the agent's own
    shell and the pipeline that runs this work there."""
    for proc in psutil.process_iter(["exe", "name"]):
        try:
            exe = proc.info["exe"] or ""
            name = (proc.info["name"] or "").lower().removesuffix(".exe")
            cwd = (proc.cwd() or "") if name in BUILDERS else ""
        except (psutil.AccessDenied, psutil.NoSuchProcess):
            continue
        if inside(exe, tree) or inside(cwd, tree):
            return proc.info["name"]
    return None


def inside(path: str, tree: Path) -> bool:
    # The separator keeps main's path from matching `4cc-studio-temp`'s, and `sk_a` `sk_ab`.
    root = str(tree.resolve()).lower() + os.sep
    return (path.lower() + os.sep).startswith(root)


def size(folder: Path) -> int:
    return sum(f.stat().st_size for f in folder.rglob("*") if f.is_file())


def tree_of(name: str) -> Path:
    if name == "main":
        return REPO
    assert "/" not in name and "\\" not in name, name
    tree = TEMP / name
    assert tree.resolve().parent == TEMP.resolve() and (tree / ".git").exists(), f"{tree} is no worktree"
    return tree


def clear(target: Path, dry: bool) -> int:
    """Removes the caches, the scratch and the older builds under `target`; returns the bytes."""
    folders = [target / cache for cache in CACHES]
    folders += [f for f in (target / "debug").glob("*" + SCRATCH_SUFFIX) if f.is_dir()]
    freed = 0
    for folder in folders:
        if not folder.is_dir():
            continue
        bytes_ = size(folder)
        freed += bytes_
        print(f"{'would remove' if dry else 'removed'} {folder} ({bytes_ / 1e9:.1f} GB)")
        if not dry:
            shutil.rmtree(folder, onerror=writable)
    for profile in PROFILES:
        deps = target / profile / "deps"
        if not deps.is_dir():
            continue
        files = stale(deps, 0)
        bytes_ = sum(f.stat().st_size for f in files)
        freed += bytes_
        print(f"{'would remove' if dry else 'removed'} {len(files)} older builds in {deps} ({bytes_ / 1e9:.1f} GB)")
        if not dry:
            for f in files:
                f.unlink()
    return freed


def main(argv: list[str]) -> int:
    dry = "--dry-run" in argv
    over = float(argv[argv.index("--over") + 1]) if "--over" in argv else 0.0
    names = [a for i, a in enumerate(argv) if not a.startswith("--") and (i == 0 or argv[i - 1] != "--over")]
    assert names, "name at least one worktree"
    refused = False
    freed = 0
    for name in names:
        tree = tree_of(name)
        target = tree / "target"
        held = size(target) if target.is_dir() else 0
        if held < over * 1e9:
            print(f"left {name}: its target/ holds {held / 1e9:.1f} GB, under {over:g}")
            continue
        process = busy(tree)
        if process:
            print(f"skipped {name}: {process} runs from it")
            refused = True
            continue
        freed += clear(target, dry)
    print(f"{'would free' if dry else 'freed'} {freed / 1e9:.1f} GB")
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
