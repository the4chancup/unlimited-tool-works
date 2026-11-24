"""Stop every cargo build and test running in one tree, and the scripts driving them.

    just stop                  the tree this justfile is in
    just stop sk_gloves14      a worktree beside the repository or in its -temp folder, by name
    just stop <path>           any tree, by path
    just stop <tree> --list    print what would be stopped, stopping nothing

A background run stopped from the agent harness (Claude Code's TaskStop) ends its outer shell
only: the script it ran goes on, and so does cargo, which holds the tree's build lock, and once
cargo ends the script starts its next stage. So this stops, in the tree:
- the builds: cargo, rustc, and anything run from the tree's `target/` (test binaries, build
  scripts);
- what launched them, walking up from each build while the parent is a launcher working in
  the tree: `just`, python (the recipes' `low_priority.py` wrapper, a script), or a shell
  running a script or a `-c` command. The walk stops at anything else: an agent's own process
  (a sidekick's session keeps running, only its command ends), an interactive terminal (a
  shell started with no script or command), or a process working elsewhere;
- everything those started (the linker, a test's children, a script's next command).
The command's own parents are left alone, since on Windows `just` runs this recipe through
`low_priority.py` too. Launchers are stopped before their children, so none starts another.
"""
import sys
from pathlib import Path

import psutil

REPO = Path(__file__).resolve().parents[2]
BUILD_TOOLS = {"cargo", "rustc"}
SHELLS = {"bash", "sh", "zsh", "pwsh", "powershell", "cmd"}
COMMAND_FLAGS = {"-c", "-command", "-file", "/c"}


def tree_of(arg: str) -> Path:
    """The tree `arg` names: a folder path, or a worktree's name beside the repository or in
    its `-temp` folder."""
    for candidate in (Path(arg), REPO.parent / arg, REPO.parent / (REPO.name + "-temp") / arg):
        if candidate.is_dir():
            return candidate.resolve()
    sys.exit(f"no tree named {arg}")


def inside(path: str, folder: Path) -> bool:
    return bool(path) and Path(path).resolve().is_relative_to(folder)


def stem(proc: psutil.Process) -> str:
    return proc.name().lower().removesuffix(".exe")


def working_in(proc: psutil.Process, tree: Path) -> bool:
    try:
        return inside(proc.cwd(), tree)
    except psutil.Error:
        return False


def is_build(proc: psutil.Process, tree: Path) -> bool:
    """Whether `proc` is one of `tree`'s builds or tests: run from its `target/`, or cargo or
    rustc working inside it."""
    try:
        if inside(proc.exe(), tree / "target"):
            return True
    except psutil.Error:
        pass
    return stem(proc) in BUILD_TOOLS and working_in(proc, tree)


def is_launcher(proc: psutil.Process, tree: Path) -> bool:
    """Whether `proc` is a launcher of `tree`'s builds: `just`, python, or a shell running a
    script or a command, working inside the tree. A shell with no arguments is a terminal."""
    name = stem(proc)
    if name == "just" or name.startswith("python"):
        return working_in(proc, tree)
    if name not in SHELLS:
        return False
    args = [arg.lower() for arg in proc.cmdline()[1:]]
    runs = (name in {"bash", "sh", "zsh"} and bool(args)) or any(a in COMMAND_FLAGS for a in args)
    return runs and working_in(proc, tree)


def main() -> None:
    args = [arg for arg in sys.argv[1:] if arg != "--list"]
    listing = "--list" in sys.argv[1:]
    tree = tree_of(args[0] if args else ".")
    me = psutil.Process()
    spared = {me.pid, *(parent.pid for parent in me.parents())}
    tops = {}
    for proc in psutil.process_iter():
        try:
            if proc.pid in spared or not is_build(proc, tree):
                continue
            top = proc
            parent = proc.parent()
            while parent and parent.pid not in spared and is_launcher(parent, tree):
                top, parent = parent, parent.parent()
            tops[top.pid] = top
        except psutil.Error:
            continue
    # Launchers first: a script stopped before its child cannot start the next stage.
    ordered = {}
    for top in tops.values():
        ordered.setdefault(top.pid, top)
        try:
            for child in top.children(recursive=True):
                ordered.setdefault(child.pid, child)
        except psutil.Error:
            pass
    procs = [proc for pid, proc in ordered.items() if pid not in spared]
    if not procs:
        print(f"no build or test runs in {tree}")
        return
    for proc in procs:
        try:
            print("would stop" if listing else "stopping", proc.pid, " ".join(proc.cmdline())[:120])
            if not listing:
                proc.terminate()
        except psutil.Error:
            pass
    if listing:
        return
    _, alive = psutil.wait_procs(procs, timeout=10)
    for proc in alive:
        try:
            proc.kill()
        except psutil.Error:
            pass
    print(f"stopped {len(procs)} processes in {tree}")


if __name__ == "__main__":
    main()
