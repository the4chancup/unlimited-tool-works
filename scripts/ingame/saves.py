"""Back up, put and restore a game's live save files, by SHA-256: before an in-game check that
changes the save, and after it to put the user's own back. Backups and kept saves go in
`.tmp/ingame_saves/` (ignored by git).

usage: python saves.py backup  <version>          copy the live EDIT/SYSTEM/GRAPHICS to
                                                   backup_<version>/ and write hashes.txt
       python saves.py put     <version> <file>   put <file> in place of the live EDIT
       python saves.py keep    <version> <name>   copy the live EDIT to saves_<version>/<name>
       python saves.py restore <version>          put every backed-up file back, verify hashes
       python saves.py status  <version>          live hashes against the backup's

Every write goes to a sibling temp file and is renamed over the target (`os.replace`), after
all assertions. `put` refuses unless the live EDIT's content is kept under this folder (the
backup or a saves_<version>/ copy), so nothing the game wrote is lost; `restore` and `put`
refuse while the game runs.
"""
import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parents[2] / ".tmp" / "ingame_saves"
KONAMI = Path(r"C:\Data\Documents\KONAMI")
LIVE = {
    "15": (KONAMI / "Pro Evolution Soccer 2015/save", "EDIT.bin", ["SYSTEM.bin"], "pes2015.exe"),
    "16": (KONAMI / "Pro Evolution Soccer 2016/save", "EDIT00000000", ["SYSTEM00000000"], "pes2016.exe"),
    "17": (KONAMI / "Pro Evolution Soccer 2017/save", "EDIT00000000", ["SYSTEM00000000"], "pes2017.exe"),
    "18": (KONAMI / "PRO EVOLUTION SOCCER 2018/save", "EDIT00000000", ["SYSTEM00000000"], "pes2018.exe"),
    "19": (KONAMI / "PRO EVOLUTION SOCCER 2019/292733975847239680/save", "EDIT00000000",
           ["SYSTEM00000000", "GRAPHICS000000"], "pes2019.exe"),
    "21": (KONAMI / "eFootball PES 2021 SEASON UPDATE/292733975847239680/save", "EDIT00000000",
           ["SYSTEM00000000", "GRAPHICS000000"], "pes2021.exe"),
}


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def running(proc: str) -> bool:
    out = subprocess.run(["tasklist", "/FI", f"IMAGENAME eq {proc}"], capture_output=True, text=True).stdout
    return proc in out.lower()


def replace(src: Path, dst: Path) -> None:
    tmp = dst.with_name(dst.name + ".gaps_tmp")
    assert not tmp.exists(), f"{tmp} exists"
    shutil.copyfile(src, tmp)
    assert sha(tmp) == sha(src), "the temp copy differs"
    os.replace(tmp, dst)
    assert sha(dst) == sha(src), f"{dst} differs after the rename"


def recorded(version: str) -> dict[str, str]:
    lines = (HERE / f"backup_{version}" / "hashes.txt").read_text(encoding="utf-8").splitlines()
    return dict(line.split("  ", 1)[::-1] for line in lines)


def main() -> None:
    action, version = sys.argv[1], sys.argv[2]
    folder, edit, others, proc = LIVE[version]
    backup = HERE / f"backup_{version}"
    names = [n for n in [edit, *others] if (folder / n).is_file()]
    if action == "backup":
        assert not backup.exists(), f"{backup} exists: a backup is never overwritten"
        backup.mkdir()
        lines = []
        for n in names:
            shutil.copy2(folder / n, backup / n)
            assert sha(backup / n) == sha(folder / n)
            lines.append(f"{sha(backup / n)}  {n}")
        (backup / "hashes.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
        print("\n".join(lines))
    elif action == "status":
        rec = recorded(version)
        for n in names:
            h = sha(folder / n)
            print(f"{n}: live {h[:16]} backup {rec.get(n, 'none')[:16]} {'SAME' if h == rec.get(n) else 'DIFFERS'}")
    elif action == "keep":
        dest = HERE / f"saves_{version}" / sys.argv[3]
        dest.parent.mkdir(exist_ok=True)
        assert not dest.exists(), f"{dest} exists"
        shutil.copy2(folder / edit, dest)
        print(f"kept {dest.name} {sha(dest)}")
    elif action == "put":
        assert not running(proc), "close the game first"
        src = Path(sys.argv[3])
        live = sha(folder / edit)
        kept = [p for p in (HERE / f"saves_{version}").glob("*") if p.is_file()] + [backup / edit]
        assert any(p.is_file() and sha(p) == live for p in kept), f"the live EDIT {live[:16]} is kept nowhere"
        replace(src, folder / edit)
        print(f"live EDIT {live[:16]} -> {sha(folder / edit)[:16]} ({src.name})")
    elif action == "restore":
        assert not running(proc), "close the game first"
        rec = recorded(version)
        for n, h in rec.items():
            assert sha(backup / n) == h, f"the backup of {n} changed"
        for n, h in rec.items():
            if (folder / n).is_file() and sha(folder / n) == h:
                print(f"{n}: already {h[:16]}")
                continue
            replace(backup / n, folder / n)
            assert sha(folder / n) == h
            print(f"{n}: restored, {h}")
    else:
        sys.exit(f"unknown action {action}")


if __name__ == "__main__":
    main()
