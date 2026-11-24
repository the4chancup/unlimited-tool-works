"""Derive the Save editor's nation lists from each PES version's own database.

Usage: python scripts/derive/derive_nations.py <games dir>

<games dir> holds one install per version, `PES2015` .. `PES2021`, each with its `Data/dt10*.cpk`
(the PES installs on G:, `G:/Games`). Build the CPK lister first:
`just cargo build -p cpk --example cpk_list`.

Each version's nations are in `common/etc/pesdb/Country.bin` inside its `dt10` CPK: a WESYS
container (16-byte header: magic, stored size, inflated size, then zlib) of fixed-size records,
1,348 bytes up to PES 2018 and 1,420 from PES 2019, ordered by ID. A record's nation ID is bits
10 to 18 of its first four bytes (little-endian), the 9-bit nationality the save stores; its
names follow from offset 78 in 70-byte language slots, the fourth (offset 288) English, in the
game's capitals.

Writes `crates/tools/save_editor/data/nations.tsv`: a header row, then one row per ID found in
any version, its English name in each version's column, empty where that version lacks it.
"""
import os
import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates" / "tools" / "save_editor" / "data" / "nations.tsv"
LISTER = ROOT / "target" / "debug" / "examples" / ("cpk_list.exe" if os.name == "nt" else "cpk_list")
VERSIONS = (2015, 2016, 2017, 2018, 2019, 2020, 2021)
RECORD = {2015: 1348, 2016: 1348, 2017: 1348, 2018: 1348, 2019: 1420, 2020: 1420, 2021: 1420}
ENGLISH = slice(288, 288 + 70)


def country_bin(games: Path, version: int, scratch: Path) -> bytes:
    """The version's Country.bin, inflated, taken out of its dt10 CPK."""
    cpks = sorted((games / f"PES{version}" / "Data").glob("dt10*.cpk"))
    assert len(cpks) == 1, (version, cpks)
    folder = scratch / str(version)
    subprocess.run([str(LISTER), str(cpks[0]), "pesdb/country.bin", str(folder)],
                   check=True, capture_output=True)
    data = (folder / "common" / "etc" / "pesdb" / "Country.bin").read_bytes()
    assert data[3:8] == b"WESYS", (version, data[:16])
    stored, inflated = struct.unpack_from("<II", data, 8)
    raw = zlib.decompress(data[16:16 + stored])
    assert len(raw) == inflated, (version, len(raw), inflated)
    return raw


def nations(raw: bytes, size: int) -> dict[int, str]:
    """ID to English name, one per record; IDs ascending and unique."""
    assert len(raw) % size == 0, (len(raw), size)
    found = {}
    for at in range(0, len(raw), size):
        record = raw[at:at + size]
        nation_id = (int.from_bytes(record[:4], "little") >> 10) & 0x1FF
        name = record[ENGLISH].split(b"\0")[0].decode("utf-8")
        assert nation_id not in found and name and "\t" not in name, (nation_id, name)
        assert not found or nation_id > max(found), nation_id
        found[nation_id] = name
    return found


def main() -> None:
    games = Path(sys.argv[1])
    assert LISTER.exists(), f"{LISTER} missing: just cargo build -p cpk --example cpk_list"
    with tempfile.TemporaryDirectory() as scratch:
        lists = {v: nations(country_bin(games, v, Path(scratch)), RECORD[v]) for v in VERSIONS}
    ids = sorted(set().union(*lists.values()))
    rows = ["id\t" + "\t".join(f"PES {v}" for v in VERSIONS)]
    rows += [f"{i}\t" + "\t".join(lists[v].get(i, "") for v in VERSIONS) for i in ids]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    tmp = OUT.with_suffix(".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(rows) + "\n")
    os.replace(tmp, OUT)
    print(OUT, len(ids), "ids;", {v: len(lists[v]) for v in VERSIONS})


if __name__ == "__main__":
    main()
