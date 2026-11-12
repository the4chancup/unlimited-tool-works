"""The temp folder the gates' tests get: one that is not its own real path.

A CI runner's temp folder is named by something other than its real path: GitHub's Windows
runner by an 8.3 short name (`RUNNER~1` in the middle of it), macOS by a symlink (`/tmp`). A
test that compares a temp path as it was handed out against one the code resolved passes on a
developer's machine, whose temp folder is its own real path, and fails there. So every test the
gates run gets a temp folder reached through a link, and that defect fails at the first local
run instead of after a push.

`run_tests.py` gives the test binaries `env()`; a script's tests run as
`temp_alias.py <script> [args...]` (the justfile's `script_test`).
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

# Python's `tempfile` reads all three; Rust's `temp_dir` reads `TMPDIR` on Unix, `TMP` then
# `TEMP` on Windows.
VARIABLES = ("TMPDIR", "TEMP", "TMP")


def alias() -> Path:
    """The machine's temp folder if it is already not its own real path (a CI runner's, a run
    started by one that has the link), else a link to a folder inside it, made on first use."""
    temp = Path(tempfile.gettempdir())
    if temp.resolve() != temp:
        return temp
    real = temp / "utw_gates_temp"
    link = temp / "utw_gates_temp_link"
    real.mkdir(exist_ok=True)
    if not link.exists():
        try:
            if os.name == "nt":
                # A junction: a symlink needs a privilege a developer's account lacks.
                subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(real)],
                               check=True, capture_output=True)
            else:
                os.symlink(real, link)
        except (OSError, subprocess.CalledProcessError):
            # Two runs at once (the gates of two worktrees): the other one made it first.
            if not link.exists():
                raise
    return link


def env() -> dict[str, str]:
    """This process's environment, its temp folder variables naming `alias()`."""
    return dict(os.environ, **dict.fromkeys(VARIABLES, str(alias())))


if __name__ == "__main__":
    sys.exit(subprocess.run([sys.executable, *sys.argv[1:]], env=env()).returncode)
