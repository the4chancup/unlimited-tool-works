"""Guardrail 4: `fmdl` and `pes_model` stay PyO3-buildable (core plan, "Workspace guardrails").

Each guarded crate's normal (non-dev, non-build) dependency graph, over every target and with
every feature enabled, may not contain `studio_core`, `studio`, any tool crate, `model_convert`,
or any crate from the runtime denylist. A guarded crate that does not exist yet is skipped, so
the check runs from Phase 1; its first real exercise is worklog step 2.6 (`fmdl`), which plants a
denied dependency to prove the check goes red.
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOLS_DIR = ROOT / "crates" / "tools"

GUARDED = ["fmdl", "pes_model"]
DENIED = {
    "studio_core",
    "studio",
    "model_convert",
    "egui",
    "eframe",
    "wgpu",
    "tokio",
    "smol",
    "async-std",
    "pyo3",
}


def workspace_members() -> list[dict]:
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout
    return json.loads(out)["packages"]


def normal_dependencies(crate: str) -> set[str]:
    # Every target and every feature: a denied crate behind a cfg or an optional feature is
    # still a dependency the Blender build could pick up.
    out = subprocess.run(
        [
            "cargo", "tree", "-p", crate,
            "-e", "normal", "--target", "all", "--all-features",
            "--prefix", "none", "--format", "{p}",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout
    names = {line.split(" ", 1)[0] for line in out.splitlines() if line.strip()}
    names.discard(crate)
    return names


def main() -> int:
    members = workspace_members()
    names = {package["name"] for package in members}
    tool_crates = {
        package["name"]
        for package in members
        if TOOLS_DIR in Path(package["manifest_path"]).resolve().parents
    }
    denied = DENIED | tool_crates
    failed = False
    for crate in GUARDED:
        if crate not in names:
            print(f"deps_check: {crate} not in the workspace yet, skipped")
            continue
        offending = sorted(normal_dependencies(crate) & denied)
        if offending:
            failed = True
            print(f"deps_check: {crate} depends on denied crates: {', '.join(offending)}")
        else:
            print(f"deps_check: {crate} ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
