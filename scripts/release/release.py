"""`just release <version>`: pack the portable .7z bundle (`plans/core/distribution.md`
"Distribution: portable .7z bundle"). Windows only.

The recipe builds `studio` with `--locked`, the build machine's home folder, cargo and rustup
homes, repository and target directory remapped out of the exe's paths (`remap_flags`), reads the executable's name from the built exe's
`--help` usage line (the name lives in `studio_core`'s `EXE_NAME`, not in this script),
stages `unlimited-tool-works-<version>/` — the exe as `<name>.exe`, the launchers it ships
(`LAUNCHERS`, none in 0.0.1), the readme, the two licenses in `licenses/`, the bundled `data/settings.toml`,
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
`--help` with no usage line, an exe that still holds one of the remapped paths or names
`VCRUNTIME140.dll` (the `--config` flags replaced `+crt-static` instead of joining it), a launcher it ships (none in 0.0.1) that does not call the exe's
name, a 7-Zip that cannot be found, and a listing that differs from the staged set (that
archive is left for inspection). The pure helpers take their input as text so
`release_test.py` can call them without git, cargo or 7-Zip.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
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
# The repository's license files, shipped in the bundle's `licenses/` folder: members open the
# exe and the readme, so the top folder keeps to those.
LICENSES = ("LICENSE-MIT", "LICENSE-APACHE")
# Beside them, the notices of the crates and fonts the exe contains (`write_notices`).
NOTICES = "THIRD-PARTY-NOTICES.txt"


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
        f"{top}/licenses",
        *(f"{top}/licenses/{license_file}" for license_file in (*LICENSES, NOTICES)),
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


def pack(seven_z: str, archive: Path, cwd: Path, names: list[str]) -> set[str]:
    """Pack `names` (relative to `cwd`) into the .7z `archive`; returns its listing's entries."""
    if subprocess.run([seven_z, "a", "-t7z", "-mx=9", str(archive), *names], cwd=cwd).returncode:
        raise Refusal(f"7z a failed ({archive.name})")
    listing = subprocess.run(
        [seven_z, "l", "-slt", str(archive)], cwd=cwd, capture_output=True, text=True, check=True
    )
    return listing_paths(listing.stdout)


def about_config(deny_toml: str) -> str:
    """`cargo-about`'s config: it accepts what `cargo deny` allows, so one list rules both, and
    reads the Windows exe's dependency graph, the only one a bundle ships."""
    allowed = tomllib.loads(deny_toml)["licenses"]["allow"]
    return (
        f"accepted = [{', '.join(json.dumps(license_id) for license_id in allowed)}]\n"
        'targets = ["x86_64-pc-windows-msvc"]\n'
        "ignore-build-dependencies = true\n"
        "ignore-dev-dependencies = true\n"
    )


def copyright_lines(text: str, shared: set[str]) -> list[str]:
    """A license copy's own notices: lines opening with "Copyright", "(c)" or "©" that are
    not in every copy of the license (`shared`). A license's body has such lines too, the
    font license's "Copyright Holder(s)" definition among them, and they are in every copy;
    placeholders are license text as well (the Apache appendix's, an MIT template left
    unfilled)."""
    return [
        line.strip() for line in text.splitlines()
        if re.match(r"\s*(Copyright\b|\([cC]\)|©)", line) and line.strip() not in shared
        and "[yyyy]" not in line and "<year>" not in line
    ]


def third_party_notices(about: dict, workspace: Path) -> str:
    """`licenses/THIRD-PARTY-NOTICES.txt` from `cargo about generate --format json`'s output:
    each license once, its text without any one author's copyright line, then every crate under
    it with the copyright lines its own copy states, which are what the licenses ask to keep.
    cargo-about lists a license once per distinct text, and copies of MIT differ in their
    copyright line alone (117 entries for MIT in 0.0.1's graph). The workspace's own crates
    are left out: the bundle's two license files cover them."""
    by_id: dict[str, list[dict]] = {}
    for variant in about["licenses"]:
        by_id.setdefault(variant["id"], []).append(variant)
    sections = []
    for variants in by_id.values():
        shared = set.intersection(
            *({line.strip() for line in variant["text"].splitlines()} for variant in variants)
        )
        crates = []
        for variant in variants:
            notices = copyright_lines(variant["text"], shared)
            for use in variant["used_by"]:
                crate = use["crate"]
                if Path(crate["manifest_path"]).is_relative_to(workspace):
                    continue
                line = f"  {crate['name']} {crate['version']}"
                crates.append(line + (f": {'; '.join(notices)}" if notices else ""))
        if not crates:
            continue
        common = max(variants, key=lambda variant: len(variant["used_by"]))
        notices = set(copyright_lines(common["text"], shared))
        text = "\n".join(
            line for line in common["text"].splitlines() if line.strip() not in notices
        )
        text = re.sub(r"\n\s*\n(\s*\n)+", "\n\n", text).strip()
        sections.append((len(crates), common["name"], sorted(crates), text))
    sections.sort(key=lambda section: -section[0])
    out = [
        "Third-party notices\n\nThis program contains the open-source software below, grouped "
        "by license: each license's\npackages with their copyright notices, then the license's "
        "text.\n"
    ]
    for _, name, crates, text in sections:
        out.append(f"\n{'=' * 80}\n{name}\n\n" + "\n".join(crates) + f"\n\n{text}\n")
    return "".join(out)


def write_notices(path: Path) -> None:
    """Writes `third_party_notices` for the exe's dependency graph at `Cargo.lock`'s versions."""
    if shutil.which("cargo-about") is None:
        raise Refusal(
            "cargo-about not found: `cargo install cargo-about --version 0.9.2 --locked "
            "--features cli`"
        )
    with tempfile.TemporaryDirectory() as work:
        config = Path(work) / "about.toml"
        config.write_text(about_config((ROOT / "deny.toml").read_text(encoding="utf-8")),
                          encoding="utf-8")
        about = Path(work) / "about.json"
        run = subprocess.run([
            "cargo", "about", "generate", "--locked", "--fail", "--format", "json",
            "-m", str(ROOT / "crates/studio/Cargo.toml"), "-c", str(config), "-o", str(about),
        ], cwd=ROOT)
        if run.returncode:
            raise Refusal("cargo about generate failed")
        notices = third_party_notices(json.loads(about.read_text(encoding="utf-8")), ROOT)
    path.write_text(notices, encoding="utf-8", newline="\n")


def sha256_line(digest: str, archive_name: str) -> str:
    return f"{digest}  {archive_name}\n"


def remap_flags(
    home: str, cargo_home: str | None, rustup_home: str | None, root: str, target: str
) -> list[str]:
    # rustc applies the last prefix that matches, so the home folder, which may hold all the
    # others, comes first.
    pairs = [(home, "~")]
    if cargo_home:
        pairs.append((cargo_home, "~cargo"))
    if rustup_home:
        pairs.append((rustup_home, "~rustup"))
    pairs.append((root, "."))
    if not Path(target).is_relative_to(root):
        pairs.append((target, "target"))
    return [f"--remap-path-prefix={source}={to}" for source, to in pairs]


def rustflags_config(flags: list[str]) -> str:
    # `--config` joins this array with the one `.cargo/config.toml` sets on the same key
    # (`+crt-static`); `RUSTFLAGS` would replace it. JSON's string escapes are TOML's.
    return f"target.x86_64-pc-windows-msvc.rustflags=[{', '.join(map(json.dumps, flags))}]"


def leaked_prefixes(exe: bytes, prefixes: list[str]) -> list[str]:
    # Windows paths match whatever their case and either separator.
    folded = exe.lower().replace(b"/", b"\\")
    return [
        prefix
        for prefix in prefixes
        if prefix.lower().replace("/", "\\").encode("utf-8") in folded
    ]


def links_crt_dynamically(exe: bytes) -> bool:
    return b"vcruntime140" in exe.lower()


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
    home = str(Path.home())
    cargo_home = os.environ.get("CARGO_HOME")
    rustup_home = os.environ.get("RUSTUP_HOME")
    flags = remap_flags(home, cargo_home, rustup_home, str(ROOT), str(target))
    build = subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--locked",
            "-p",
            "studio",
            "--config",
            rustflags_config(flags),
        ],
        cwd=ROOT,
    )
    if build.returncode != 0:
        raise Refusal("cargo build --release --locked -p studio failed")
    exe = target / "release/utw.exe"
    built = exe.read_bytes()
    leaked = leaked_prefixes(
        built, [p for p in (home, cargo_home, rustup_home, str(ROOT), str(target)) if p]
    )
    if leaked:
        raise Refusal(f"{exe} still holds the build machine's paths: {', '.join(leaked)}")
    if links_crt_dynamically(built):
        raise Refusal(f"{exe} names VCRUNTIME140.dll: the C runtime is not linked statically")
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
        (top / "licenses").mkdir()
        for license_file in LICENSES:
            shutil.copyfile(ROOT / license_file, top / "licenses" / license_file)
        write_notices(top / "licenses" / NOTICES)
        for folder in EMPTY_FOLDERS:
            (top / folder).mkdir()
        seven_z = find_7z()
        if seven_z is None:
            raise Refusal(
                f"7-Zip not found: `7z` is not on PATH and {SEVEN_Z} does not exist"
            )
        found = pack(seven_z, archive, staging, [top.name])
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
        found = pack(seven_z, symbols, symbols_stage, [f"{name}.exe", PDB_NAME])
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
