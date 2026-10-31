"""Writes resources/templates/DpFileList.bin and resources/templates/placeholder.cpk (step 4.25).

DpFileList.bin is the official list the `upgrade-dpfl` command installs and every compile
compares the installed list with: the entries of resources/templates/DpFileList.txt, in order,
in the layout the maintainer's 53-entry in-game test loaded on PES 2015 and PES 2021
(.tmp/phase_04/4_7/dpfl_pes21_53.py; docs/plans/team_compiler/pipeline.md "DpFileList upgrade"): a 16-byte
header (u32 0, u32 entry count, 8 zero bytes), one 48-byte NUL-padded record per CPK file name,
then the PES 2021 list's 1204-byte zero tail.

placeholder.cpk is the empty placeholder CPK the official DLC ships for unused slots, a copy of
the cpk crate's fixture cpkmc136_placeholder.cpk (6,272 bytes, one 11-byte file `placeholder`),
byte-identical to 4cc_68_midcup.cpk of the maintainer's PES 2021 install and to the PES 2020
install's placeholders.
"""
import hashlib
import os
import struct
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TEMPLATES = REPO / "resources/templates"
SOURCE = TEMPLATES / "DpFileList.txt"
PLACEHOLDER = REPO / "crates/libs/cpk/tests/fixtures/cpkmc136_placeholder.cpk"
RECORD = 48
PES21_TAIL = 1204
PLACEHOLDER_SHA256 = "9aba63a7cb351a85"


def entries() -> list[str]:
    names = []
    for line in SOURCE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            names.append(line)
    return names


def record(name: str) -> bytes:
    raw = name.encode("ascii")
    assert len(raw) < RECORD, name
    return raw + bytes(RECORD - len(raw))


def read_back(data: bytes) -> list[str]:
    zero, count = struct.unpack_from("<II", data, 0)
    assert zero == 0 and data[8:16] == bytes(8)
    out = []
    for i in range(count):
        rec = data[16 + i * RECORD:16 + (i + 1) * RECORD]
        out.append(rec[:rec.index(0)].decode("ascii"))
    assert not any(data[16 + count * RECORD:]), "the tail after the records is zero"
    return out


def write(path: Path, data: bytes) -> None:
    temp = path.with_name(path.name + ".tmp")
    temp.write_bytes(data)
    os.replace(temp, path)


def main() -> None:
    names = entries()
    assert len(names) == 53, len(names)
    assert len(set(names)) == len(names), "an entry is listed twice"
    assert all(name.startswith("4cc_") and name.endswith(".cpk") for name in names)
    assert names[0] == "4cc_01_db.cpk" and names[-1] == "4cc_99_test.cpk"

    data = struct.pack("<II8x", 0, len(names))
    data += b"".join(record(name) for name in names) + bytes(PES21_TAIL)
    assert read_back(data) == names
    assert len(data) == 16 + 53 * RECORD + PES21_TAIL == 3764

    placeholder = PLACEHOLDER.read_bytes()
    assert len(placeholder) == 6272
    assert hashlib.sha256(placeholder).hexdigest().startswith(PLACEHOLDER_SHA256)

    write(TEMPLATES / "DpFileList.bin", data)
    write(TEMPLATES / "placeholder.cpk", placeholder)
    for name in ("DpFileList.bin", "placeholder.cpk"):
        path = TEMPLATES / name
        print(name, path.stat().st_size, hashlib.sha256(path.read_bytes()).hexdigest()[:16])


if __name__ == "__main__":
    main()
