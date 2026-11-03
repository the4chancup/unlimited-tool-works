"""`just release <version>`: pack the portable bundle (`plans/core/distribution.md`
"Distribution: portable .7z bundle"), the .7z on Windows, the .tar.gz on Linux.

The recipe builds `studio` with `--locked`, the build machine's home folder, cargo and rustup
homes, repository and target directory remapped out of the binary's paths (`remap_flags`), reads the executable's name from the built binary's
`--help` usage line (the name lives in `studio_core`'s `EXE_NAME`, not in this script),
stages `unlimited-tool-works-<version>/` — the binary as `<name>.exe` (`<name>` on Linux), the launchers it ships
(`LAUNCHERS`, none in 0.0.1), the readme, the license files (`LICENSES`) and the generated
third-party notices in `licenses/`, the bundled `data/settings.toml`,
and the empty folders it ships (`EMPTY_FOLDERS`, none in 0.0.1) — under a temporary folder in
cargo's target directory (asked of `cargo metadata`, since a developer may have moved it),
packs it as `<target>/dist/unlimited-tool-works-<version>-windows.7z` with 7-Zip, or as
`-linux.tar.gz` with `tarfile` on Linux (a tar keeps the binary's executable bit), with a
`sha256sum`-format `.sha256` beside it, then lists the archive and refuses unless the listing
equals the staged set exactly, the empty folders included. On Windows it packs beside it `unlimited-tool-works-<version>-symbols.7z` (the exe and
its `.pdb`, the symbols `symbolize_log.py` names a posted `utw.log`'s backtrace with), never
in the bundle; on Linux the binary is stripped of its debug info (most of its size, the
standard library's: `objcopy` keeps the symbol table and a debuglink), which is packed flat
as `unlimited-tool-works-<version>-linux-symbols.tar.gz`.

Every refusal prints one `error: <what>` line on stderr and exits 1: a platform that is not
Windows or Linux, a version that is not the workspace's, a readme whose first line names another
version, an archive that already exists (a release is never overwritten), a dirty tree, a
`cargo build` that fails, a `--help` with no usage line, a binary that still holds one of the
remapped paths or, on Windows, names `VCRUNTIME140.dll` (the `--config` flags replaced `+crt-static`
instead of joining it), a launcher it ships (none in 0.0.1) that does not call the exe's
name, a 7-Zip that cannot be found or whose `a` fails (Windows only), an `objcopy` or a
`readelf` that cannot be found or fails (Linux only), a stripped binary that still carries
`.debug_` sections or lost its `.symtab` (Linux only), a `cargo-about` that cannot be found,
whose `generate` fails or that lists no third-party crate, a `.pdb` missing beside the built
exe (Windows only), a Linux archive whose binary member lost its executable bit, and a listing
that differs from the staged set (that archive is left for inspection).
The pure helpers take their input as text so
`release_test.py` can call them without git, cargo or 7-Zip.
"""

import argparse
import dataclasses
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
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
# The license files the bundle's `licenses/` folder ships, each staged under its file name:
# members open the exe and the readme, so the top folder keeps to those. The last two cover
# works the suite bundles outside a crate (the Roboto Flex font, the icon outlines taken
# from Phosphor), which cargo-about's notices cannot list.
LICENSES = (
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "resources/fonts/RobotoFlex-OFL.txt",
    "resources/icons/PHOSPHOR-LICENSE.txt",
)
# Beside them, the notices of the crates the exe links, egui's bundled fonts among them
# (`write_notices`).
NOTICES = "THIRD-PARTY-NOTICES.txt"


@dataclasses.dataclass(frozen=True)
class Platform:
    """The facts one platform decides: the rust target triple the build's rustflags key
    and cargo-about's dependency graph name, the file `cargo build` leaves under cargo's
    target directory, the suffix the staged binary's name takes, and the release
    archive's name ending."""
    triple: str
    built: str
    suffix: str
    archive: str


# The two platforms a release packs for, keyed by `platform_name`'s results.
PLATFORMS = {
    "windows": Platform(
        "x86_64-pc-windows-msvc", "release/utw.exe", ".exe", "windows.7z"
    ),
    "linux": Platform(
        "x86_64-unknown-linux-gnu", "release/utw", "", "linux.tar.gz"
    ),
}


class Refusal(Exception):
    """A reason the release is not made; `main` prints it as the one `error:` line."""


def platform_name(sys_platform: str) -> str:
    """`windows` on win32, `linux` on linux: the values `PLATFORMS` is keyed by."""
    if sys_platform == "win32":
        return "windows"
    if sys_platform == "linux":
        return "linux"
    raise Refusal("releases are packed on Windows and Linux only")


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
    # Both platforms' archives, their `.sha256`s and the two symbols archives: a release
    # is never overwritten, whatever its archives are called.
    return f"*-{version}-*"


def symbols_entries(name: str) -> set[str]:
    # Flat, so an "Extract here" gives `symbolize_log.py` the exe and its .pdb side by side.
    return {f"{name}.exe", PDB_NAME}


def linux_symbols_entries(binary: str) -> set[str]:
    # Flat like the Windows symbols': the debug info `objcopy` split out of the binary.
    return {f"{binary}.debug"}


def expected_entries(name: str, version: str, platform: str) -> set[str]:
    top = f"{PACKAGE}-{version}"
    return {
        top,
        f"{top}/{name}{PLATFORMS[platform].suffix}",
        f"{top}/README.txt",
        f"{top}/licenses",
        *(f"{top}/licenses/{Path(license_file).name}" for license_file in (*LICENSES, NOTICES)),
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
    """Pack `names` (relative to `cwd`) into the .7z `archive`, the Windows bundle's and
    the symbols' format (the Linux archive is `pack_tar`'s). Returns the listing's entries."""
    if subprocess.run([seven_z, "a", "-t7z", "-mx=9", str(archive), *names], cwd=cwd).returncode:
        raise Refusal(f"7z a failed ({archive.name})")
    listing = subprocess.run(
        [seven_z, "l", "-slt", str(archive)], cwd=cwd, capture_output=True, text=True, check=True
    )
    return listing_paths(listing.stdout)


def pack_tar(archive: Path, cwd: Path, top_name: str, binary: str) -> set[str]:
    """Pack `cwd`/`top_name` recursively into the .tar.gz `archive`; returns its member
    names. The binary and the directories are `0755`, every other file `0644`, and every
    member is owned by uid and gid 0 with no names, so the archive names nobody."""

    def owned_by_nobody(member: tarfile.TarInfo) -> tarfile.TarInfo:
        member.uid = member.gid = 0
        member.uname = member.gname = ""
        member.mode = (
            0o755 if member.isdir() or member.name == f"{top_name}/{binary}" else 0o644
        )
        return member

    with tarfile.open(archive, "w:gz") as tar:
        tar.add(cwd / top_name, arcname=top_name, filter=owned_by_nobody)
    with tarfile.open(archive) as tar:
        return {member.name for member in tar.getmembers()}


def pack_tar_flat(archive: Path, cwd: Path, name: str) -> set[str]:
    """Pack the one file `cwd`/`name` into the .tar.gz `archive` under its own name, the
    Linux symbols archive's shape; returns the member names. `0644`, and owned by uid
    and gid 0 with no names, as `pack_tar`'s files are."""

    def owned_by_nobody(member: tarfile.TarInfo) -> tarfile.TarInfo:
        member.uid = member.gid = 0
        member.uname = member.gname = ""
        member.mode = 0o644
        return member

    with tarfile.open(archive, "w:gz") as tar:
        tar.add(cwd / name, arcname=name, filter=owned_by_nobody)
    with tarfile.open(archive) as tar:
        return {member.name for member in tar.getmembers()}


def elf_sections(readelf_text: str) -> set[str]:
    """The section names in `readelf --section-headers --wide`'s table: the token after
    each `[Nr]` row label when it starts with `.`, as every named section's does. The
    unnamed first section's `NULL`, its type, is skipped by the same rule."""
    return {
        match.group(1)
        for line in readelf_text.splitlines()
        if (match := re.match(r"\s*\[\s*\d+\]\s+(\.\S+)", line))
    }


def leftover_debug(sections: set[str]) -> list[str]:
    """The debug-info sections `objcopy --strip-debug` should have removed, sorted. Not
    `.debug_gdb_scripts`: rustc puts it in the loaded image (a few bytes pointing gdb at
    Rust's pretty-printers), so a strip keeps it, and it holds no debug info."""
    return sorted(s for s in sections if s.startswith(".debug_") and s != ".debug_gdb_scripts")


def about_config(deny_toml: str, triple: str) -> str:
    """`cargo-about`'s config: it accepts what `cargo deny` allows, so one list rules both, and
    reads the shipped binary's dependency graph for `triple`, since the two platforms link
    different crates."""
    allowed = tomllib.loads(deny_toml)["licenses"]["allow"]
    return (
        f"accepted = [{', '.join(json.dumps(license_id) for license_id in allowed)}]\n"
        f'targets = ["{triple}"]\n'
        "ignore-build-dependencies = true\n"
        "ignore-dev-dependencies = true\n"
    )


def copyright_lines(text: str, shared: set[str]) -> list[str]:
    """A license copy's own notices: lines opening with "Copyright", "(c) <year>" or "©"
    that are not in every copy of the license (`shared`). "(c)" counts only with a year
    after it: Apache-2.0's and MPL-2.0's bodies open lettered clauses with "(c)", which a
    copy missing the clause would leave unshared and printed as a crate's notice. A
    license's body has such lines too, the font license's "Copyright Holder(s)" definition
    among them, and they are in every copy; placeholders are license text as well (the
    Apache appendix's, an MIT template left unfilled)."""
    return [
        line.strip() for line in text.splitlines()
        if re.match(r"\s*(Copyright\b|\([cC]\)\s*\d|©)", line) and line.strip() not in shared
        and not any(marker in line for marker in ("[yyyy]", "{yyyy}", "<year>"))
    ]


def third_party_notices(about: dict, workspace: Path) -> str:
    """`licenses/THIRD-PARTY-NOTICES.txt` from `cargo about generate --format json`'s output:
    each license once, its text without any one author's copyright line, then every crate under
    it with the copyright lines its own copy states, which are what the licenses ask to keep.
    cargo-about lists a license once per distinct text, and copies of MIT differ in their
    copyright line alone (117 entries for MIT in 0.0.1's graph). The workspace's own crates
    are left out: the bundle's two license files cover them. A graph with no third-party
    crate at all is refused: the header alone would ship as the notices file."""
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
        sections.append((len(crates), common["name"], sorted(set(crates)), text))
    sections.sort(key=lambda section: -section[0])
    if not sections:
        raise Refusal("cargo-about listed no third-party crate")
    out = [
        "Third-party notices\n\nThis program contains the open-source software below, grouped "
        "by license: each license's\npackages with their copyright notices, then the license's "
        "text.\n"
    ]
    for _, name, crates, text in sections:
        out.append(f"\n{'=' * 80}\n{name}\n\n" + "\n".join(crates) + f"\n\n{text}\n")
    return "".join(out)


def write_notices(path: Path, platform: str) -> None:
    """Writes `third_party_notices` for the platform binary's dependency graph at
    `Cargo.lock`'s versions."""
    if shutil.which("cargo-about") is None:
        raise Refusal(
            "cargo-about not found: `cargo install cargo-about --version 0.9.2 --locked "
            "--features cli`"
        )
    with tempfile.TemporaryDirectory() as work:
        config = Path(work) / "about.toml"
        config.write_text(about_config((ROOT / "deny.toml").read_text(encoding="utf-8"),
                                       PLATFORMS[platform].triple),
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


def rustflags_config(flags: list[str], triple: str) -> str:
    # `--config` joins this array with the one `.cargo/config.toml` sets on the same key
    # (`+crt-static`); `RUSTFLAGS` would replace it. JSON's string escapes are TOML's.
    return f"target.{triple}.rustflags=[{', '.join(map(json.dumps, flags))}]"


def build_args(flags: list[str], triple: str) -> list[str]:
    """The `cargo build` argv `release()` and `build_remapped.py` share, so a flag cannot
    change in one and leave a candidate built unlike the release."""
    return [
        "cargo", "build", "--release", "--locked", "-p", "studio",
        "--config", rustflags_config(flags, triple),
    ]


def leaked_prefixes(exe: bytes, prefixes: list[str]) -> list[str]:
    # Windows paths match whatever their case and either separator.
    folded = exe.lower().replace(b"/", b"\\")
    return [
        prefix
        for prefix in prefixes
        if prefix.lower().replace("/", "\\").encode("utf-8") in folded
    ]


def leaking_strings(exe: bytes, prefixes: list[str], limit: int = 20) -> list[str]:
    """The printable strings of `exe` holding one of `prefixes` (folded as
    `leaked_prefixes` folds them), each once, at most `limit`: what a refusal shows so a
    leak is diagnosed from a CI log without the binary."""
    folded = [prefix.lower().replace("/", "\\") for prefix in prefixes]
    found: list[str] = []
    for run in re.finditer(rb"[\x20-\x7e]{4,}", exe):
        text = run.group().decode("ascii")
        if text not in found and any(p in text.lower().replace("/", "\\") for p in folded):
            found.append(text)
            if len(found) == limit:
                break
    return found


def links_crt_dynamically(exe: bytes) -> bool:
    return b"vcruntime140" in exe.lower()


def machine_prefixes(target: Path) -> list[str]:
    """The build machine's paths the shipped exe must not hold: the home folder,
    `CARGO_HOME` and `RUSTUP_HOME` when set, the repository and cargo's target directory,
    in `remap_flags`' order."""
    prefixes = [
        str(Path.home()),
        os.environ.get("CARGO_HOME"),
        os.environ.get("RUSTUP_HOME"),
        str(ROOT),
        str(target),
    ]
    return [prefix for prefix in prefixes if prefix]


def check_exe(exe: Path, prefixes: list[str], platform: str) -> None:
    """Refuses the built `exe` when a path the remap should have taken out is still in it,
    or when it links the C runtime dynamically (the `--config` flags replaced `+crt-static`
    instead of joining it). The C-runtime check is Windows' alone: a Linux binary links
    the system's libc. `release()`, `build_remapped.py` and `make_candidate.py` all run
    it: a candidate ships only what a release would."""
    built = exe.read_bytes()
    leaked = leaked_prefixes(built, prefixes)
    if leaked:
        strings = "\n  ".join(leaking_strings(built, leaked))
        raise Refusal(
            f"{exe} still holds the build machine's paths: {', '.join(leaked)}; in\n  {strings}"
        )
    if platform == "windows" and links_crt_dynamically(built):
        raise Refusal(f"{exe} names VCRUNTIME140.dll: the C runtime is not linked statically")


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
    platform = platform_name(sys.platform)
    plat = PLATFORMS[platform]
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
    build = subprocess.run(build_args(flags, plat.triple), cwd=ROOT)
    if build.returncode != 0:
        raise Refusal("cargo build --release --locked -p studio failed")
    exe = target / plat.built
    check_exe(exe, machine_prefixes(target), platform)
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
    archive = dist / f"{PACKAGE}-{version}-{plat.archive}"
    checksum = archive.with_name(archive.name + ".sha256")
    dist.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(dir=dist.parent))
    try:
        top = staging / f"{PACKAGE}-{version}"
        top.mkdir()
        binary = f"{name}{plat.suffix}"
        if platform == "windows":
            shutil.copyfile(exe, top / binary)
        else:
            # `line-tables-only` still links the standard library's debug info, most of
            # the binary: it is moved out beside the bundle as the .pdb is on Windows,
            # the shipped binary stripped of it but keeping its symbol table and a
            # debuglink to `utw.debug`.
            symbols_stage = staging / "symbols"
            symbols_stage.mkdir()
            debug = symbols_stage / f"{binary}.debug"
            objcopy = shutil.which("objcopy")
            if objcopy is None:
                raise Refusal("objcopy not found: `objcopy` is not on PATH (binutils provides it)")
            if subprocess.run(
                [objcopy, "--only-keep-debug", str(exe), str(debug)]
            ).returncode:
                raise Refusal(f"objcopy --only-keep-debug failed ({exe.name})")
            if subprocess.run(
                [objcopy, "--strip-debug", f"--add-gnu-debuglink={debug}",
                 str(exe), str(top / binary)]
            ).returncode:
                raise Refusal(f"objcopy --strip-debug failed ({binary})")
            readelf = shutil.which("readelf")
            if readelf is None:
                raise Refusal("readelf not found: `readelf` is not on PATH (binutils provides it)")
            headers = subprocess.run(
                [readelf, "--section-headers", "--wide", str(top / binary)],
                capture_output=True,
                text=True,
            )
            if headers.returncode:
                raise Refusal(f"readelf --section-headers failed ({binary})")
            sections = elf_sections(headers.stdout)
            debug_sections = leftover_debug(sections)
            if debug_sections:
                raise Refusal(f"{binary} still carries debug sections: {', '.join(debug_sections)}")
            if ".symtab" not in sections:
                raise Refusal(f"{binary} lost its symbol table with its debug info")
        for bundled in LAUNCHERS + ("README.txt",):
            shutil.copyfile(ROOT / "resources/bundle" / bundled, top / bundled)
        (top / "data").mkdir()
        shutil.copyfile(
            ROOT / "resources/bundle/data/settings.toml", top / "data/settings.toml"
        )
        (top / "licenses").mkdir()
        for license_file in LICENSES:
            shutil.copyfile(ROOT / license_file, top / "licenses" / Path(license_file).name)
        write_notices(top / "licenses" / NOTICES, platform)
        for folder in EMPTY_FOLDERS:
            (top / folder).mkdir()
        if platform == "windows":
            seven_z = find_7z()
            if seven_z is None:
                raise Refusal(
                    f"7-Zip not found: `7z` is not on PATH and {SEVEN_Z} does not exist"
                )
            found = pack(seven_z, archive, staging, [top.name])
        else:
            found = pack_tar(archive, staging, top.name, binary)
        expected = expected_entries(name, version, platform)
        if found != expected:
            raise Refusal(
                f"the archive's listing differs from the staged set "
                f"(missing {sorted(expected - found)}, extra {sorted(found - expected)}); "
                f"{archive} is left for inspection"
            )
        if platform == "linux":
            # The tar ships the binary's mode with it, so it is checked like the listing:
            # a member that lost its executable bit would not start once unpacked.
            with tarfile.open(archive) as tar:
                binary_mode = tar.getmember(f"{top.name}/{binary}").mode
            if not binary_mode & stat.S_IXUSR:
                raise Refusal(f"{archive}: {top.name}/{binary} is not executable")
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        checksum.write_bytes(sha256_line(digest, archive.name).encode("utf-8"))
        print(archive)
        print(checksum)
        print(f"{archive.stat().st_size} bytes")
        if platform == "windows":
            # The symbols archive: the exe and its .pdb, kept out of the bundle for
            # `symbolize_log.py` to name a posted log's frames against. The .pdb keeps
            # its build name, the one the exe records.
            pdb = target / "release" / PDB_NAME
            if not pdb.exists():
                raise Refusal(f"{pdb} does not exist; the release's symbols cannot be packed")
            symbols_stage = staging / "symbols"
            symbols_stage.mkdir()
            shutil.copyfile(top / binary, symbols_stage / binary)
            shutil.copyfile(pdb, symbols_stage / PDB_NAME)
            symbols = dist / f"{PACKAGE}-{version}-symbols.7z"
            found = pack(seven_z, symbols, symbols_stage, [binary, PDB_NAME])
            expected = symbols_entries(name)
            if found != expected:
                raise Refusal(
                    f"the symbols archive's listing differs "
                    f"(missing {sorted(expected - found)}, extra {sorted(found - expected)}); "
                    f"{symbols} is left for inspection"
                )
            print(symbols)
        else:
            # Linux's symbols archive is the debug info `objcopy` split out of the
            # staged binary, packed flat: a crash's backtrace names its functions, and
            # `utw.debug` adds the lines (`addr2line`).
            symbols = dist / f"{PACKAGE}-{version}-linux-symbols.tar.gz"
            found = pack_tar_flat(symbols, symbols_stage, debug.name)
            expected = linux_symbols_entries(binary)
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
