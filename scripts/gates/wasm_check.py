"""Gate 4: `cargo check --target wasm32-unknown-unknown` over studio_core and every lib crate.

The crate list is derived from the workspace (core plan, "Workspace guardrails" rule 6), so a
new lib crate is checked without anyone remembering to list it. Crates that cannot be checked
for wasm32 by design are named in EXCLUDED with the reason.
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LIBS_DIR = ROOT / "crates" / "libs"

# Lib crates exempt from the wasm32 check, each with the reason it is exempt.
EXCLUDED = {
    "python_bindings": "PyO3 cdylib for Blender; native only (core plan, guardrail 4)",
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


def main() -> int:
    crates = []
    for package in workspace_members():
        manifest = Path(package["manifest_path"]).resolve()
        name = package["name"]
        is_lib = LIBS_DIR in manifest.parents
        if name == "studio_core" or (is_lib and name not in EXCLUDED):
            crates.append(name)
    crates.sort()
    if not crates:
        print("wasm_check: no crates to check")
        return 0
    print("wasm_check: " + " ".join(crates))
    for name, reason in EXCLUDED.items():
        print(f"wasm_check: skipping {name} ({reason})")
    command = ["cargo", "check", "--target", "wasm32-unknown-unknown"]
    for name in crates:
        command += ["-p", name]
    return subprocess.run(command, cwd=ROOT).returncode


if __name__ == "__main__":
    sys.exit(main())
