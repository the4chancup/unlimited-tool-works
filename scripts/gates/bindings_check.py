"""`just bindings`: build the python_bindings wheel with maturin and run its smoke test.

(core plan, "Phase 2", `python_bindings`). The wheel is unzipped onto the smoke test's
`sys.path` rather than pip-installed: Blender's bundled Python has no pip and the test
must run under it (`--python <interpreter>`).
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SMOKE = ROOT / "crates" / "libs" / "python_bindings" / "tests" / "smoke.py"


def wheel_path(output: str) -> Path | None:
    """The wheel maturin reports building ("Built wheel ... to <path>")."""
    for line in reversed(output.splitlines()):
        if ".whl" not in line:
            continue
        fragment = line.rsplit(" to ", 1)[-1].strip()
        match = re.search(r"(.+?\.whl)", fragment)
        if match and Path(match.group(1)).is_file():
            return Path(match.group(1))
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python", default=sys.executable,
                        help="the interpreter the smoke test runs under (e.g. Blender's)")
    args = parser.parse_args()

    if shutil.which("maturin") is None:
        print("bindings_check: maturin not found on PATH: pip install maturin")
        return 1

    build = ["maturin", "build", "--release", "--manifest-path",
             "crates/libs/python_bindings/Cargo.toml"]
    print("bindings_check: " + " ".join(build))
    result = subprocess.run(build, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    output = result.stdout + result.stderr
    if result.returncode != 0:
        print(output)
        return result.returncode
    wheel = wheel_path(output)
    if wheel is None:
        print(output)
        print("bindings_check: maturin reported no wheel path")
        return 1
    if "-cp311-abi3-" not in wheel.name:
        print(f"bindings_check: {wheel.name} is not a cp311-abi3 wheel "
              "(the plan's one-wheel-per-platform rule)")
        return 1
    print(f"bindings_check: wheel {wheel}")

    with tempfile.TemporaryDirectory(prefix="pes_models_native_") as temp:
        with zipfile.ZipFile(wheel) as archive:
            archive.extractall(temp)
        env = dict(os.environ)
        env["PYTHONPATH"] = temp + os.pathsep + env.get("PYTHONPATH", "")
        smoke = [args.python, str(SMOKE)]
        print("bindings_check: PYTHONPATH=" + temp)
        print("bindings_check: " + " ".join(smoke))
        return subprocess.run(smoke, cwd=ROOT, env=env).returncode


if __name__ == "__main__":
    sys.exit(main())
