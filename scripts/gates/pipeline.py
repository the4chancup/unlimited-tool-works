"""`just pipeline`: the checkpoint and the Linux gates on main's latest commit, run in a
worktree of their own, so main's tree stays free to work in while they run.

`just checkpoint` reads the working tree it is started in, which froze main for as long as
the gates ran (no commit, no merge, no edit), and `just gates-remote` after it made the wait
longer: work stood still after every slice. Here both run from the gate worktree
(`../4cc-studio-temp/gate`, or the folder `STUDIO_GATE_WORKTREE` names), detached at the
commit main's HEAD was on when this started, with that worktree's own `target/`. The ref
`refs/checkpoints/gated` is the repository's, shared by every worktree, so a pass moves it
as a checkpoint run in main does, and `just checkpoint-status` in main reads it.

What it gates is a commit: work not committed in main is not part of the run, and commits
made while it runs wait for the next one. A failure is fixed by a commit on top, as one CI
finds is. One run at a time: a second would move the worktree under the first, so a marker
file under the worktree's `.tmp/` refuses it. Start it detached and logged, as any long
command (`RUNBOOK.md` "How the work runs").
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GATE = Path(os.environ.get("STUDIO_GATE_WORKTREE") or ROOT.parent / "4cc-studio-temp" / "gate")
MARKER = GATE / ".tmp" / "pipeline.running"


def git(where: Path, *args: str) -> str:
    return subprocess.run(["git", *args], cwd=where, capture_output=True, text=True, check=True).stdout.strip()


def main(argv: list[str]) -> int:
    if argv[1:] not in ([], ["--no-mutants"]):
        sys.exit("usage: pipeline.py [--no-mutants]")
    if not (GATE / ".git").exists():
        sys.exit(f"no gate worktree at {GATE}: create it once with `git worktree add --detach {GATE} HEAD`")
    if MARKER.exists():
        sys.exit(f"a pipeline run is going ({MARKER.read_text().strip()}); if none is, delete {MARKER}")
    if git(GATE, "status", "--porcelain"):
        sys.exit(f"the gate worktree is not clean: {GATE}")
    head = git(ROOT, "rev-parse", "HEAD")
    MARKER.parent.mkdir(exist_ok=True)
    MARKER.write_text(f"{head[:8]}, pid {os.getpid()}\n", encoding="utf-8")
    try:
        git(GATE, "checkout", "-q", "--detach", head)
        print(f"gating {head[:8]} in {GATE}", flush=True)
        for step in (["just", "checkpoint", *argv[1:]], ["just", "gates-remote"]):
            code = subprocess.run(step, cwd=GATE).returncode
            if code != 0:
                print(f"`{' '.join(step)}` failed on {head[:8]} (exit code {code})")
                return code
        print(f"{head[:8]} passed the checkpoint and the Linux gates")
        return 0
    finally:
        MARKER.unlink()


if __name__ == "__main__":
    sys.exit(main(sys.argv))
