"""Provenance of `resources/bins/TeamColor.bin` and `UniColor.bin` (step 4.8) and of the layout
`resources/bins/README.md` gives for them: checks that the two bundled bases are Red's
(`Engines/bins/`, commit `c94e673`), measures their records, and compares them with the files
in each PES install's `4cc_08_bins.cpk`.

Read-only on Red's checkout and on the installs; the installs' files are extracted into
`.tmp/4_7/color_bins/` with the scratch CPK tool `.tmp/4_0/apptest` (`apptest extract <cpk> <inner
path> <out file>`). Run from the repository root with the installs' drive connected.
"""

import collections
import hashlib
import struct
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BUNDLED = ROOT / "resources/bins"
RED = Path("C:/Data/4cc/4cc aet compiler/4cc-aet-compiler-red/Engines/bins")
APPTEST = ROOT / ".tmp/4_0/apptest/target/release/apptest.exe"
OUT = ROOT / ".tmp/4_7/color_bins"
INSTALLS = Path("G:/Games")
VERSIONS = ("2015", "2016", "2017", "2018", "2019", "2020", "2021")
# name -> (path inside a CPK, record size, the ID field's format)
BINS = {
    "TeamColor.bin": ("common/etc/TeamColor.bin", 16, "<H"),
    "UniColor.bin": ("common/character0/model/character/uniform/team/UniColor.bin", 85, "<I"),
}


def describe(label: str, name: str, data: bytes) -> None:
    """One line on `data`: its record count, ID range, and the records out of sequence."""
    _, size, id_format = BINS[name]
    records = len(data) // size
    ids = [struct.unpack_from(id_format, data, index * size)[0] for index in range(records)]
    odd = [(index + 100, found) for index, found in enumerate(ids) if found != index + 100]
    stray = len(data) % size
    print(
        f"{label:9} {name:14} {len(data):6} bytes, {records} records, {stray} stray bytes, "
        f"records whose ID is not 100 + index: {odd}, sha256 {hashlib.sha256(data).hexdigest()[:12]}"
    )


def team_color_fields(data: bytes) -> None:
    """The color count field's values over every record."""
    counts = collections.Counter(
        struct.unpack_from("<H", data, index * 16 + 2)[0] for index in range(len(data) // 16)
    )
    print("  TeamColor color counts:", dict(counts))


def uni_color_fields(data: bytes) -> None:
    """The kit count, kit number and icon values, and whether an unused entry is all zero."""
    kit_counts, kit_numbers, icons = (collections.Counter() for _ in range(3))
    unused_nonzero = 0
    for index in range(len(data) // 85):
        record = data[index * 85 : index * 85 + 85]
        kit_counts[record[4]] += 1
        for kit in range(10):
            entry = record[5 + kit * 8 : 13 + kit * 8]
            kit_numbers[entry[0]] += 1
            if entry[0] == 0xFF:
                unused_nonzero += entry[1:] != bytes(7)
            else:
                icons[entry[1]] += 1
    print("  UniColor kit counts:", dict(sorted(kit_counts.items())))
    print("  UniColor kit numbers:", dict(sorted(kit_numbers.items())))
    print("  UniColor icons:", dict(sorted(icons.items())))
    print("  UniColor unused entries with a nonzero rest:", unused_nonzero)


def main() -> None:
    for name in BINS:
        bundled = (BUNDLED / name).read_bytes()
        assert bundled == (RED / name).read_bytes(), f"{name} differs from Red's"
        describe("bundled", name, bundled)
    team_color_fields((BUNDLED / "TeamColor.bin").read_bytes())
    uni_color_fields((BUNDLED / "UniColor.bin").read_bytes())

    OUT.mkdir(parents=True, exist_ok=True)
    for version in VERSIONS:
        cpk = INSTALLS / f"PES{version}/download/4cc_08_bins.cpk"
        for name, (inner, _, _) in BINS.items():
            target = OUT / f"pes{version}_{name}"
            if target.exists():
                target.unlink()
            subprocess.run(
                [str(APPTEST), "extract", str(cpk), inner, str(target)],
                capture_output=True,
                text=True,
                check=False,
            )
            if target.exists():
                describe(f"PES {version}", name, target.read_bytes())
            else:
                print(f"PES {version}  {name:14} not in {cpk.name}")


if __name__ == "__main__":
    main()
