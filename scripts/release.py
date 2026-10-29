"""`just release <version>`: pack the portable .7z bundle (`plans/core/distribution.md`
"Distribution: portable .7z bundle"). Windows only.

The recipe builds `studio` with `--locked`, reads the executable's name from the built exe's
`--help` usage line (the name lives in `studio_core`'s `EXE_NAME`, not in this script),
stages `unlimited-tool-works-<version>/` — the exe as `<name>.exe`, the launchers it ships
(`LAUNCHERS`, none in 0.0.1), the readme, the two licenses, the bundled `data/settings.toml`,
and the empty folders it ships (`EMPTY_FOLDERS`, none in 0.0.1) — under a temporary folder in
cargo's target directory (asked of `cargo metadata`, since a developer may have moved it),
packs it as `<target>/dist/unlimited-tool-works-<version>-windows.7z` with a
`sha256sum`-format `.sha256` beside it, then lists the archive and refuses unless the listing
equals the staged set exactly, the empty folders included. Beside it it packs `unlimited-tool-works-<version>-symbols.7z` (the exe and
its `.pdb`, the symbols `symbolize_log.py` names a posted `utw.log`'s backtrace with), never
in the bundle.

Every refusal prints one `error: <what>` line on stderr and exits 1: a platform that is not
Windows, a version that is not the workspace's, a readme whose first line names another
version, an archive that already exists (a release is never overwritten), a dirty tree, a
`--help` with no usage line, a launcher it ships (none in 0.0.1) that does not call the exe's
name, a 7-Zip that cannot be found, and a listing that differs from the staged set (that
archive is left for inspection). The pure helpers take their input as text so
`release_test.py` can call them without git, cargo or 7-Zip.
"""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEVEN_Z = Path(r"C:\Program Files\7-Zip\7z.exe")
# The exe records this name for its symbols and `llvm-symbolizer` finds the .pdb beside
# the exe by it, so the symbols archive keeps the build's name, not the bundle exe's.
PDB_NAME = "utw.pdb"
# The bundle's top folder and its archives' name. No built file states it, so it is the
# recipe's own; the exe's name, which does live in the code, comes from its usage line.
PACKAGE = "unlimited-tool-works"
# 0.0.1 is the Save editor's alone; the 0.0.2 preview ships `quick_compile.bat` and
# `quick_upgrade.bat` again, with the empty `exports/` and `old_exports/` they work in.
LAUNCHERS: tuple[str, ...] = ()
EMPTY_FOLDERS: tuple[str, ...] = ()


class Refusal(Exception):
    """A reason the release is not made; `main` prints it as the one `error:` line."""


def workspace_version(cargo_toml: str) -> str:
    return tomllib.loads(cargo_toml)["workspace"]["package"]["version"]


def exe_name(help_text: str) -> str | None:
    usage = re.search(r"^Usage: (\S+)", help_text, re.MULTILINE)
    return usage.group(1) if usage else None


def readme_names_version(readme: str, version: str) -> bool:
    # The version alone: the program's name lives in `studio_core`, never here.
    lines = readme.splitlines()
    return bool(lines) and lines[0].rsplit(" ", 1)[-1] == version


def launcher_calls(bat: str, name: str) -> bool:
    return f'"%~dp0{name}.exe"' in bat


def existing_glob(version: str) -> str:
    # The bundle's `<package>-<version>-windows.7z`, its `.sha256` and the symbols'
    # `<package>-<version>-symbols.7z`: a release is never overwritten, whatever its name.
    return f"*-{version}-*.7z*"


def symbols_entries(name: str) -> set[str]:
    # Flat, so an "Extract here" gives `symbolize_log.py` the exe and its .pdb side by side.
    return {f"{name}.exe", PDB_NAME}


def expected_entries(name: str, version: str) -> set[str]:
    top = f"{PACKAGE}-{version}"
    return {
        top,
        f"{top}/{name}.exe",
        f"{top}/README.txt",
        f"{top}/LICENSE-MIT",
        f"{top}/LICENSE-APACHE",
        f"{top}/data",
        f"{top}/data/settings.toml",
        *(f"{top}/{launcher}" for launcher in LAUNCHERS),
        *(f"{top}/{folder}" for folder in EMPTY_FOLDERS),
    }


def listing_paths(slt: str) -> set[str]:
    # The archive's own header block (its `Path = <archive>` included) ends at a `----------`
    # line; every `Path =` after it is one entry, `\`-separated on Windows.
    entries = set()
    in_entries = False
    for line in slt.splitlines():
        if line.startswith("----------"):
            in_entries = True
        elif in_entries and line.startswith("Path = "):
            entries.add(line.removeprefix("Path = ").replace("\\", "/"))
    return entries


def sha256_line(digest: str, archive_name: str) -> str:
    return f"{digest}  {archive_name}\n"


def target_dir() -> Path:
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return Path(json.loads(metadata.stdout)["target_directory"])


def find_7z() -> str | None:
    on_path = shutil.which("7z")
    if on_path:
        return on_path
    return str(SEVEN_Z) if SEVEN_Z.exists() else None


def release(version: str) -> None:
    if sys.platform != "win32":
        raise Refusal("releases are packed on Windows only")
    declared = workspace_version((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if version != declared:
        raise Refusal(f"version {version} is not the workspace's {declared}")
    readme = (ROOT / "resources/bundle/README.txt").read_text(encoding="utf-8")
    if not readme_names_version(readme, version):
        raise Refusal(
            f"resources/bundle/README.txt's first line does not end with the version {version}"
        )
    target = target_dir()
    dist = target / "dist"
    # Any name: a version is released once, whatever its archives are called.
    existing = sorted(dist.glob(existing_glob(version)))
    if existing:
        raise Refusal(
            f"{existing[0]} already exists; a release is never overwritten: remove it first"
        )
    status = subprocess.run(
        ["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True, check=True
    )
    if status.stdout.strip():
        raise Refusal("the working tree is dirty; a release builds from a clean checkout")
    build = subprocess.run(
        ["cargo", "build", "--release", "--locked", "-p", "studio"], cwd=ROOT
    )
    if build.returncode != 0:
        raise Refusal("cargo build --release --locked -p studio failed")
    exe = target / "release/utw.exe"
    help_text = subprocess.run(
        [str(exe), "--help"], cwd=ROOT, capture_output=True, text=True
    ).stdout
    name = exe_name(help_text)
    if name is None:
        raise Refusal(f"{exe} --help shows no usage line")
    for bat in LAUNCHERS:
        path = ROOT / "resources/bundle" / bat
        if not launcher_calls(path.read_text(encoding="utf-8"), name):
            raise Refusal(f'{path.relative_to(ROOT)} does not call "%~dp0{name}.exe"')
    archive = dist / f"{PACKAGE}-{version}-windows.7z"
    checksum = archive.with_name(archive.name + ".sha256")
    dist.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(dir=dist.parent))
    try:
        top = staging / f"{PACKAGE}-{version}"
        top.mkdir()
        shutil.copyfile(exe, top / f"{name}.exe")
        for bundled in LAUNCHERS + ("README.txt",):
            shutil.copyfile(ROOT / "resources/bundle" / bundled, top / bundled)
        (top / "data").mkdir()
        shutil.copyfile(
            ROOT / "resources/bundle/data/settings.toml", top / "data/settings.toml"
        )
        for license_file in ("LICENSE-MIT", "LICENSE-APACHE"):
            shutil.copyfile(ROOT / license_file, top / license_file)
        for folder in EMPTY_FOLDERS:
            (top / folder).mkdir()
        seven_z = find_7z()
        if seven_z is None:
            raise Refusal(
                f"7-Zip not found: `7z` is not on PATH and {SEVEN_Z} does not exist"
            )
        pack = subprocess.run(
            [seven_z, "a", "-t7z", "-mx=9", str(archive), top.name], cwd=staging
        )
        if pack.returncode != 0:
            raise Refusal("7z a failed")
        listing = subprocess.run(
            [seven_z, "l", "-slt", str(archive)],
            cwd=staging,
            capture_output=True,
            text=True,
            check=True,
        )
        found = listing_paths(listing.stdout)
        expected = expected_entries(name, version)
        if found != expected:
            raise Refusal(
                f"the archive's listing differs from the staged set "
                f"(missing {sorted(expected - found)}, extra {sorted(found - expected)}); "
                f"{archive} is left for inspection"
            )
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        checksum.write_bytes(sha256_line(digest, archive.name).encode("utf-8"))
        print(archive)
        print(checksum)
        print(f"{archive.stat().st_size} bytes")
        # The symbols archive: the exe and its .pdb, kept out of the bundle for
        # `symbolize_log.py` to name a posted log's frames against. The .pdb keeps
        # its build name, the one the exe records.
        pdb = target / "release" / PDB_NAME
        if not pdb.exists():
            raise Refusal(f"{pdb} does not exist; the release's symbols cannot be packed")
        symbols_stage = staging / "symbols"
        symbols_stage.mkdir()
        shutil.copyfile(top / f"{name}.exe", symbols_stage / f"{name}.exe")
        shutil.copyfile(pdb, symbols_stage / PDB_NAME)
        symbols = dist / f"{PACKAGE}-{version}-symbols.7z"
        pack = subprocess.run(
            [seven_z, "a", "-t7z", "-mx=9", str(symbols), f"{name}.exe", PDB_NAME],
            cwd=symbols_stage,
        )
        if pack.returncode != 0:
            raise Refusal("7z a failed (symbols)")
        listing = subprocess.run(
            [seven_z, "l", "-slt", str(symbols)],
            cwd=symbols_stage,
            capture_output=True,
            text=True,
            check=True,
        )
        found = listing_paths(listing.stdout)
        expected = symbols_entries(name)
        if found != expected:
            raise Refusal(
                f"the symbols archive's listing differs "
                f"(missing {sorted(expected - found)}, extra {sorted(found - expected)}); "
                f"{symbols} is left for inspection"
            )
        print(symbols)
    finally:
        shutil.rmtree(staging)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    args = parser.parse_args()
    try:
        release(args.version)
    except Refusal as refusal:
        print(f"error: {refusal}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
