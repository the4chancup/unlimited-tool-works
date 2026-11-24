"""Empty the build folder `target/` of trees still in use (the main tree, the gate) before a pause
of days, so long mutation runs get the disk: the next build there starts from nothing.

    python scripts/builds/clear_for_pause.py [--apply] NAME...   NAME = main, or a folder under 4cc-studio-temp

Without --apply it prints what it would remove and its size, a dry run. `clear_targets.py` is
for finished worktrees and refuses these; `clear_caches.py` keeps their builds. Kept in each
target: `mutants.local.toml` (a running mutation run passes it to cargo-mutants as its config),
`run_tests_times.json` (measured test timings, which no build makes again), `CACHEDIR.TAG` and
`dist/` (packed archives). A target is refused, with one line saying why (the exit code is
then 1), while a running process has its exe, its working folder or an argument under it,
other than a path to a kept file.
"""
import os
import shutil
import stat
import sys
from pathlib import Path

import psutil

REPO = Path(__file__).resolve().parents[2]
TEMP = REPO.parent / (REPO.name + "-temp")
KEEP = {"mutants.local.toml", "run_tests_times.json", "CACHEDIR.TAG", "dist"}


def users(target: Path) -> list[str]:
    """The running processes using `target`, as "name pid"."""
    root = os.path.normcase(str(target.resolve()))
    found = []
    for proc in psutil.process_iter(["pid", "name", "exe", "cwd", "cmdline"]):
        try:
            places = [proc.info["exe"] or "", proc.info["cwd"] or "", *(proc.info["cmdline"] or [])]
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
        if any(os.path.normcase(p).startswith(root) and Path(p).name not in KEEP for p in places if p):
            found.append(f"{proc.info['name']} {proc.info['pid']}")
    return found


def size(path: Path) -> int:
    if path.is_file():
        return path.stat().st_size
    return sum(f.stat().st_size for f in path.rglob("*") if f.is_file())


def writable(func, path, _exc):
    os.chmod(path, stat.S_IWRITE)
    func(path)


args = sys.argv[1:]
apply = "--apply" in args
names = [a for a in args if a != "--apply"]
assert names, "name at least one tree: main, or a folder under 4cc-studio-temp"
status = 0
for name in names:
    assert "/" not in name and "\\" not in name, name
    target = (REPO if name == "main" else TEMP / name) / "target"
    if not target.is_dir():
        print(f"no target folder: {target}")
        continue
    busy = users(target)
    if busy:
        print(f"refused {name}: in use by {', '.join(busy[:5])}")
        status = 1
        continue
    total = 0
    for entry in sorted(target.iterdir()):
        if entry.name in KEEP:
            continue
        total += size(entry)
        if apply:
            if entry.is_dir():
                shutil.rmtree(entry, onerror=writable)
            else:
                entry.unlink()
    print(f"{name}: {'freed' if apply else 'would free'} {total / 1e9:.1f} GB in {target}")
print(f"C: {shutil.disk_usage(REPO.anchor).free / 1e9:.1f} GB free")
sys.exit(status)
