"""Remove the stale build artifacts from a cargo `target/<profile>/deps` folder: for each
artifact stem (`libfoo`, `foo`), every hash but the newest. Cargo names an artifact
`<stem>-<16 hex hash>.<ext>`; a new hash appears when a crate's features, dependencies or
toolchain change, and the old hash's files stay behind forever. Removing them is safe: an
artifact cargo still wants is rebuilt, at the cost of that build's time.

    python scripts/stale_deps.py DEPS_DIR [--older-than-days N] [--apply]

Without `--apply` it only prints what it would remove (a dry run). A hash is stale when a
newer hash of the same stem exists and every file of the stale hash is older than N days
(default 1), so a build running now never loses what it just wrote. Refuses a folder not
named `deps`."""
import re
import sys
import time
from collections import defaultdict
from pathlib import Path

ARTIFACT = re.compile(r"^(?P<stem>.+)-(?P<hash>[0-9a-f]{16})(?P<ext>\..+)?$")


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    apply = "--apply" in sys.argv
    days = float(sys.argv[sys.argv.index("--older-than-days") + 1]) if "--older-than-days" in sys.argv else 1.0
    args = [a for a in args if a != str(days) and a != str(int(days))]
    deps = Path(args[0])
    assert deps.name == "deps" and deps.is_dir(), f"{deps}: not a cargo deps folder"
    cutoff = time.time() - days * 86400
    groups: dict[str, dict[str, list[Path]]] = defaultdict(lambda: defaultdict(list))
    for f in deps.iterdir():
        m = ARTIFACT.match(f.name)
        if m and f.is_file():
            groups[m["stem"]][m["hash"]].append(f)
    stale, total = [], 0
    for stem, hashes in groups.items():
        if len(hashes) < 2:
            continue
        newest = max(hashes, key=lambda h: max(f.stat().st_mtime for f in hashes[h]))
        for h, files in hashes.items():
            if h != newest and all(f.stat().st_mtime < cutoff for f in files):
                stale.extend(files)
                total += sum(f.stat().st_size for f in files)
    print(f"{len(stale)} stale files, {total / 2**30:.1f} GB, in {deps}")
    if not apply:
        biggest = sorted(stale, key=lambda f: f.stat().st_size, reverse=True)[:10]
        for f in biggest:
            print(f"  would remove {f.stat().st_size / 2**20:8.0f} MB  {f.name}")
        return
    for f in stale:
        f.unlink()
    print("removed")


if __name__ == "__main__":
    main()
