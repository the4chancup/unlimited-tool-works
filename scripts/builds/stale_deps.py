"""Remove the stale build artifacts from a cargo `target/<profile>/deps` folder: for each unit
cargo built, every older build of it. Cargo names an artifact `<stem>-<16 hex hash>.<ext>`;
a new hash appears when a crate's features, dependencies or toolchain change, and the old
hash's files stay behind forever. An artifact cargo still wants is rebuilt, at the cost of
that build's time.

    python scripts/builds/stale_deps.py DEPS_DIR [--older-than-days N] [--apply]

Without `--apply` it only prints what it would remove (a dry run). The files of one hash are
one unit. Two units are builds of the same thing when they have the same root source file
(the first `.rs` file in the hash's `.d` dep-info file; the stem when there is none) and the
same kind of output (a test or binary `.exe`, a proc-macro `.dll`, an `.rlib`, or an
`.rmeta` alone, which `cargo clippy` and `cargo check` leave). Grouping by stem alone would
take units that are all in use for builds of one thing: every crate's integration test
named `cli`, two versions of one dependency, and a crate's clippy check beside its build.
A unit is stale when a newer one of the same group exists and every file of it is older
than N days (default 1), so a build running now never loses what it just wrote. Two feature
sets of one crate still share a group, so the older of them, if still in use, is rebuilt
once. Refuses a folder not named `deps`."""
import re
import sys
import time
from collections import defaultdict
from pathlib import Path

ARTIFACT = re.compile(r"^(?P<stem>.+)-(?P<hash>[0-9a-f]{16})(?P<ext>\..+)?$")
KINDS = (".exe", ".dll", ".rlib", ".rmeta")


def root_source(files: list[Path]) -> str | None:
    """The first `.rs` file a unit's dep-info names (clippy lists `clippy.toml` before it)."""
    for f in files:
        if f.suffix == ".d":
            first = f.read_text(encoding="utf-8", errors="replace").split("\n", 1)[0]
            for path in first.split(": ", 1)[-1].split(" "):
                if path.endswith(".rs"):
                    return path.lower()
    return None


def kind(files: list[Path]) -> str:
    suffixes = {f.suffix for f in files}
    return next((k for k in KINDS if k in suffixes), "other")


def stale(deps: Path, days: float) -> list[Path]:
    """The files of every unit a newer unit of its group replaced, older than `days`."""
    cutoff = time.time() - days * 86400
    units: dict[str, list[Path]] = defaultdict(list)
    for f in deps.iterdir():
        m = ARTIFACT.match(f.name)
        if m and f.is_file():
            units[m["hash"]].append(f)
    groups: dict[tuple[str, str], list[list[Path]]] = defaultdict(list)
    for files in units.values():
        stem = ARTIFACT.match(files[0].name)["stem"].removeprefix("lib")
        groups[(root_source(files) or stem, kind(files))].append(files)
    found = []
    for members in groups.values():
        newest = max(members, key=lambda files: max(f.stat().st_mtime for f in files))
        for files in members:
            if files is not newest and all(f.stat().st_mtime < cutoff for f in files):
                found.extend(files)
    return found


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    apply = "--apply" in sys.argv
    days = float(sys.argv[sys.argv.index("--older-than-days") + 1]) if "--older-than-days" in sys.argv else 1.0
    args = [a for a in args if a != str(days) and a != str(int(days))]
    deps = Path(args[0])
    assert deps.name == "deps" and deps.is_dir(), f"{deps}: not a cargo deps folder"
    found = stale(deps, days)
    total = sum(f.stat().st_size for f in found)
    print(f"{len(found)} stale files, {total / 2**30:.1f} GB, in {deps}")
    if not apply:
        biggest = sorted(found, key=lambda f: f.stat().st_size, reverse=True)[:10]
        for f in biggest:
            print(f"  would remove {f.stat().st_size / 2**20:8.0f} MB  {f.name}")
        return
    for f in found:
        f.unlink()
    print("removed")


if __name__ == "__main__":
    main()
