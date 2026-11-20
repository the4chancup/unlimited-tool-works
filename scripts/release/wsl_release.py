"""`just release-wsl <version>`: the Linux bundle of the committed HEAD, packed in WSL's Ubuntu
22.04 and copied into the target directory's `dist/`, beside the Windows bundle `just release`
packs (RUNBOOK "A hand-given build").

Ubuntu 22.04's C library (glibc 2.35) is the release workflow's floor, so the bundle starts
wherever a released one does; the VPS's build asks for 2.39. `wsl/build.sh` does the work as the
build user (`wsl/setup.sh` makes it, once): it fetches the commit from this repository's git
folder into its own clone, runs `just release` there, then checks the archive's checksum, that
its binary starts and names the version, and that it asks for no newer glibc than the floor.

Refuses a dirty tree, since only the committed HEAD is built: a bundle stamped with a version
must not silently leave out the edits beside it.
"""

import json
import subprocess
import sys
from pathlib import Path, PureWindowsPath

DISTRO = "Ubuntu-22.04"
BUILD_USER = "utwbuild"
SETUP = f"wsl -d {DISTRO} -u root -- bash scripts/release/wsl/setup.sh"


def wsl_path(path: str) -> str:
    """The path WSL mounts a Windows path at: `C:\\Data\\x` is `/mnt/c/Data/x`."""
    windows = PureWindowsPath(path)
    if not windows.drive or not windows.drive.endswith(":"):
        raise ValueError(f"not a drive path: {path}")
    return "/".join(["/mnt", windows.drive[0].lower(), *windows.parts[1:]])


def git(*args: str) -> str:
    return subprocess.run(["git", *args], capture_output=True, text=True, check=True).stdout.strip()


def main(version: str) -> int:
    if git("status", "--porcelain"):
        print("error: the tree is dirty; only the committed HEAD is built", file=sys.stderr)
        return 1
    root = Path(git("rev-parse", "--show-toplevel"))
    git_dir = git("rev-parse", "--path-format=absolute", "--git-common-dir")
    commit = git("rev-parse", "HEAD")
    metadata = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                              capture_output=True, text=True, check=True, cwd=root).stdout
    dist = Path(json.loads(metadata)["target_directory"]) / "dist"
    command = ["wsl.exe", "-d", DISTRO, "-u", BUILD_USER, "--", "bash",
               wsl_path(str(root / "scripts/release/wsl/build.sh")),
               wsl_path(git_dir), commit, version, wsl_path(str(dist))]
    if subprocess.run(command).returncode:
        print(f"error: the WSL build failed, as said above (a first build needs the setup: {SETUP})",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: wsl_release.py <version>")
    sys.exit(main(sys.argv[1]))
