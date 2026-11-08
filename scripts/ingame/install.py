"""Put a compiled CPK into a PES install's test slot for an in-game check, and take it out again.

  python scripts/ingame/install.py install GAME CPK   keep the slot's original aside, then put CPK
                                                      in its place (e.g. output/4cc_99_test.cpk)
  python scripts/ingame/install.py revert GAME        put the kept original back

The test slot is one CPK the game already loads last, so whatever it holds overrides every
other CPK; the compiled CPK replaces it whole. That one file is the only game file written: no
save, no DpFileList.bin, no other CPK. GAME is a key of `TEST_SLOTS` below (16, 17, 19 or 21); the
install folder comes from game.py's `GAMES`.

Every copy is checked by SHA-256. Both commands refuse while the game runs, and refuse to write
over a slot that holds neither the original (`original_sha256` below) nor the CPK this script
installed last (its hash recorded beside the kept original), so a slot changed by hand or by
another tool is never lost. A slot already holding CPK itself (a bundle's quick_compile.bat
deployed it) is accepted by `install`, which records it so `revert` can restore the original. The kept original lives in `.tmp/ingame/pes<GAME>/` of the
repository (gitignored scratch) and stays there after a revert, ready for the next install.
"""

import hashlib
import os
import shutil
import sys
from pathlib import Path

from game import GAMES, pids_of

# Per game: the test slot, relative to the install folder in game.py's GAMES, and the SHA-256 of
# the file the install holds there, the one revert restores. PES 17's slot is the last entry of
# its DpFileList (the highest priority); its own file is a leftover Fox face package of about
# 100 MB. A game gets an entry here once its slot and that file's hash are known.
TEST_SLOTS = {
    # PES 16's slot is the last entry of its DpFileList too, and holds the cup's 6 KB placeholder
    # CPK.
    "16": {
        "file": "download/4cc_90_test.cpk",
        "original_sha256": "9aba63a7cb351a858aae33e89caf16dc9b0e9bc6dc62e1441ef4dd5730d31eba",
    },
    "17": {
        "file": "download/4cc_90_test.cpk",
        "original_sha256": "b1eb59079817d8d3aedc7b0714b1ed8cadc6d786f1269bec9abc43e49bd93a2a",
    },
    # PES 19's and 21's slots are the last entries of their
    # DpFileLists too; each holds the cup's 6 KB placeholder CPK.
    "19": {
        "file": "download/4cc_69_midcup.cpk",
        "original_sha256": "9aba63a7cb351a858aae33e89caf16dc9b0e9bc6dc62e1441ef4dd5730d31eba",
    },
    "21": {
        "file": "download/4cc_99_test.cpk",
        "original_sha256": "9aba63a7cb351a858aae33e89caf16dc9b0e9bc6dc62e1441ef4dd5730d31eba",
    },
}
STATE_DIR = Path(__file__).resolve().parents[2] / ".tmp" / "ingame"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def copy_checked(source: Path, target: Path, expected: str) -> None:
    """Copies `source` over `target` through a sibling temporary file, replaced into place only
    once its hash is `expected`: a copy that fails halfway leaves `target` as it was, never a
    torn file that no later run would recognize."""
    partial = target.with_name(target.name + ".partial")
    shutil.copy2(source, partial)
    if sha256(partial) != expected:
        partial.unlink()
        raise SystemExit(f"the copy of {source} to {target} does not match: {target} left as it was")
    os.replace(partial, target)


def last_installed(record: Path) -> str | None:
    return record.read_text(encoding="utf-8").strip() if record.exists() else None


def slot_files(game_key: str) -> tuple[Path, str, Path, Path]:
    """The game's test slot, its original's hash, the kept original and the record of the hash
    installed last."""
    slot = Path(GAMES[game_key]["dir"]) / TEST_SLOTS[game_key]["file"]
    kept = STATE_DIR / f"pes{game_key}" / f"{slot.name}.original"
    return slot, TEST_SLOTS[game_key]["original_sha256"], kept, kept.parent / "installed.sha256"


def install(game_key: str, cpk: Path) -> None:
    slot, original, kept, record = slot_files(game_key)
    if not cpk.is_file() or cpk.stat().st_size == 0:
        raise SystemExit(f"{cpk} is not a CPK file")
    if not slot.is_file():
        raise SystemExit(f"{slot} does not exist: check GAMES in game.py and TEST_SLOTS here")
    live = sha256(slot)
    installed = sha256(cpk)
    # A slot already holding `cpk` itself (a launcher's compile deployed it there) loses
    # nothing to the copy, and recording it lets `revert` restore the original afterwards.
    if live not in (original, last_installed(record), installed):
        raise SystemExit(f"{slot} holds neither its original, the CPK installed last nor {cpk}: not touching it")
    if kept.exists():
        if sha256(kept) != original:
            raise SystemExit(f"{kept} is not the slot's original: not touching {slot}")
    else:
        if live != original:
            raise SystemExit(f"the original of {slot} is not kept in {kept.parent}: not touching it")
        kept.parent.mkdir(parents=True, exist_ok=True)
        copy_checked(slot, kept, original)
    copy_checked(cpk, slot, installed)
    record.write_text(installed + "\n", encoding="utf-8")
    print(f"installed {cpk} into {slot} ({cpk.stat().st_size} bytes); the original is kept as {kept}")


def revert(game_key: str) -> None:
    slot, original, kept, record = slot_files(game_key)
    if not kept.exists() or sha256(kept) != original:
        raise SystemExit(f"{kept} is missing or is not the slot's original: not touching {slot}")
    if not slot.is_file():
        raise SystemExit(f"{slot} does not exist: check GAMES in game.py and TEST_SLOTS here")
    live = sha256(slot)
    if live == original:
        print(f"{slot} already holds its original")
        return
    if live != last_installed(record):
        raise SystemExit(f"{slot} holds neither its original nor the CPK installed last: not touching it")
    copy_checked(kept, slot, original)
    print(f"reverted: {slot} holds its original again")


def main(args: list[str]) -> int:
    install_args = len(args) == 3 and args[0] == "install"
    revert_args = len(args) == 2 and args[0] == "revert"
    if not (install_args or revert_args) or args[1] not in TEST_SLOTS:
        print(__doc__, file=sys.stderr)
        return 2
    proc = GAMES[args[1]]["proc"]
    if pids_of(proc):
        raise SystemExit(f"{proc} is running: close the game first")
    if install_args:
        install(args[1], Path(args[2]))
    else:
        revert(args[1])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
