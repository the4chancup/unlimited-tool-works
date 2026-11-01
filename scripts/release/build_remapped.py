"""The release exe as `just release` builds it (scripts/release/release.py's remap flags joined with
the config's `+crt-static`), then its checks: no remapped prefix left in the exe and no
VCRUNTIME140.dll. For a release candidate and the built-window checks, so target/release keeps
one set of flags (a change of rustflags rebuilds every crate).

Run from anywhere: python scripts/release/build_remapped.py   (the lead starts it at below-normal
priority, like every build)
"""
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "release"))
import release  # noqa: E402

target = release.target_dir()
home = str(Path.home())
cargo_home = os.environ.get("CARGO_HOME")
rustup_home = os.environ.get("RUSTUP_HOME")
root = str(ROOT)
flags = release.remap_flags(home, cargo_home, rustup_home, root, str(target))
print("flags:", *flags, sep="\n  ")
build = subprocess.run(
    ["cargo", "build", "--release", "--locked", "-p", "studio", "--config",
     release.rustflags_config(flags)],
    cwd=ROOT,
)
if build.returncode != 0:
    sys.exit(f"build failed ({build.returncode})")
exe = target / "release/utw.exe"
built = exe.read_bytes()
leaked = release.leaked_prefixes(
    built, [p for p in (home, cargo_home, rustup_home, root, str(target)) if p]
)
print("leaked prefixes:", leaked or "none")
print("links the C runtime dynamically:", release.links_crt_dynamically(built))
print("the user's name anywhere in the exe:", built.lower().count(Path.home().name.lower().encode()))
sys.exit(1 if leaked or release.links_crt_dynamically(built) else 0)
