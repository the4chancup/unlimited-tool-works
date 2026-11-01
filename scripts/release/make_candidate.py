"""Stage a release candidate for the maintainer's check: the bundle `just release` packs, laid
out unpacked in a folder of its own outside the repository (`scripts/release/release.py` `release`,
lines "top = staging / ..." to the empty folders), from the release exe built at the
candidate's commit (`scripts/release/build_remapped.py`), and packed beside it as the .7z handed
to the maintainer. Writes nothing under `target/dist`: a version's release archive is written
once, by `just release`, after logic version 1's hash is frozen.

Refuses unless the tree is clean, HEAD is the commit named, the exe is newer than the last
commit that changed code or bundle resources, and the destination does not exist yet.

Run: python scripts/release/make_candidate.py <commit> [<destination folder name>]
(the folder goes in `../4cc-studio-temp/`; default `candidate-<workspace version>`; a
restaged candidate takes a new name, e.g. `candidate-0.0.1b`)
"""
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "release"))
import release  # noqa: E402  (its pure helpers: exe_name, launcher_calls, expected_entries)

VERSION = release.workspace_version((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
EXE = release.target_dir() / "release/utw.exe"
DEST = ROOT.parent / "4cc-studio-temp" / (sys.argv[2] if len(sys.argv) > 2 else f"candidate-{VERSION}")
TOP = DEST / f"{release.PACKAGE}-{VERSION}"

commit = sys.argv[1]


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True,
                          check=True).stdout.strip()


assert not git("status", "--porcelain"), "the tree is dirty"
head = git("rev-parse", "HEAD")
assert head.startswith(commit), (head, commit)
# The exe must postdate the last commit that changed what it is built from (a docs commit
# after the build changes nothing in it).
code_time = int(git("log", "-1", "--format=%ct", "--", "crates", "Cargo.toml", "Cargo.lock",
                    "resources"))
assert EXE.stat().st_mtime > code_time, "the release exe predates the last code commit"
assert not DEST.exists(), f"{DEST} exists: look at it first"

help_text = subprocess.run([str(EXE), "--help"], capture_output=True, text=True).stdout
name = release.exe_name(help_text)
assert name, "no usage line in --help"
for bat in release.LAUNCHERS:
    text = (ROOT / "resources/bundle" / bat).read_text(encoding="utf-8")
    assert release.launcher_calls(text, name), bat

TOP.mkdir(parents=True)
shutil.copyfile(EXE, TOP / f"{name}.exe")
for bundled in release.LAUNCHERS + ("README.txt",):
    shutil.copyfile(ROOT / "resources/bundle" / bundled, TOP / bundled)
(TOP / "data").mkdir()
shutil.copyfile(ROOT / "resources/bundle/data/settings.toml", TOP / "data/settings.toml")
(TOP / "licenses").mkdir()
for license_file in release.LICENSES:
    shutil.copyfile(ROOT / license_file, TOP / "licenses" / license_file)
release.write_notices(TOP / "licenses" / release.NOTICES)
for folder in release.EMPTY_FOLDERS:
    (TOP / folder).mkdir()
(DEST / "COMMIT.txt").write_text(f"{head}\n", encoding="utf-8", newline="\n")

staged = sorted(str(p.relative_to(DEST)).replace("\\", "/") for p in [TOP, *TOP.rglob("*")])
expected = sorted(release.expected_entries(name, VERSION))
print("staged:", staged)
print("release's set:", expected)
assert set(staged) == set(expected), "the staged set differs from the release's"

# The archive handed to the maintainer, packed as `just release` packs it; named after the
# destination, so a restaged candidate's archive is told apart from the last one's.
seven_z = release.find_7z()
assert seven_z, f"7-Zip not found: `7z` is not on PATH and {release.SEVEN_Z} does not exist"
ARCHIVE = DEST / f"{release.PACKAGE}-{DEST.name.removeprefix('candidate-')}-windows.7z"
packed = release.pack(seven_z, ARCHIVE, DEST, [TOP.name])
assert packed == set(expected), ("the archive's listing differs", packed ^ set(expected))
print("ok:", TOP)
print("ok:", ARCHIVE)
