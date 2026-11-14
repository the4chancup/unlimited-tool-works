"""Write the Compiler's `face_diff.xml` fixtures (step 4.5d).

No loose `face_diff.xml` exists on the maintainer's machine (Everything index, 2026-10-03), so
the fixtures are two real `face_diff.bin` files from cup exports, encoded here in the two text
forms the compiler accepts, with Python's `base64` (an encoder independent of the Rust crate
the compiler decodes with):

- `dif.xml`: the `<dif>` form, written with ElementTree exactly as Red's `diff_from_xml`
  writes it (XML declaration, the base64 on one line between two line breaks);
- `plain.xml`: the base64 alone, wrapped at 76 columns with CRLF line breaks.

Each comes with the `.bin` it must decode to. The two bins also cover the two length classes
real files have: `dif.bin` is 944 bytes, the length its header gives; `plain.bin` is 960 bytes
with a header that gives 944 (326 of the 2,695 loose `face_diff.bin` on the machine).

Run from the repository root: `python scripts/provenance/fixtures/face_diff_fixtures.py`.
"""
import base64
import os
import struct
import xml.etree.ElementTree as ET
from pathlib import Path

SOURCES = {
    "dif": Path(r"C:/Data/4cc/Lab/Gud/SMBG  Aesthetics Export VGL26/Faces/XXX06- Miyamoto/face_diff.bin"),
    "plain": Path(
        r"C:/Data/4cc/Teams_Main/JP/Exports/JP 2026 Winter Aesthetic Additions for Day 2"
        r"/Faces/XXX14 - Lemontene/face_diff.bin"
    ),
}
OUT = Path("crates/tools/team_compiler/tests/fixtures/face_diff")
TEMPLATE = Path("resources/templates/face_diff.bin")


def header_length(data):
    """The length the header's two counts give."""
    count2, count3 = struct.unpack_from("<II", data, 0x48)
    return 0xF0 + count2 * 0x10 + count3 * 0x20


def write(path, data):
    """Write through a sibling temp file, so a failed run leaves no half-written fixture."""
    temp = path.with_name(path.name + ".tmp")
    temp.write_bytes(data)
    os.replace(temp, path)


def main():
    bins = {name: path.read_bytes() for name, path in SOURCES.items()}
    template = TEMPLATE.read_bytes()
    for name, data in bins.items():
        assert data[:4] == b"FACE", name
        # A fixture equal to the bundled template could not tell "decoded" from "injected".
        assert data != template, name
    assert (len(bins["dif"]), header_length(bins["dif"])) == (944, 944)
    assert (len(bins["plain"]), header_length(bins["plain"])) == (960, 944)

    dif = ET.Element("dif")
    dif.text = "\n%s\n" % base64.b64encode(bins["dif"]).decode("ascii")
    dif_path = OUT / "dif.xml"
    plain = base64.encodebytes(bins["plain"]).replace(b"\n", b"\r\n")

    OUT.mkdir(parents=True, exist_ok=True)
    temp = dif_path.with_name("dif.xml.tmp")
    ET.ElementTree(dif).write(temp, encoding="utf-8", xml_declaration=True)
    os.replace(temp, dif_path)
    write(OUT / "dif.bin", bins["dif"])
    write(OUT / "plain.xml", plain)
    write(OUT / "plain.bin", bins["plain"])

    # Read back with the decoder Red uses.
    assert base64.b64decode(ET.parse(dif_path).getroot().text.strip()) == bins["dif"]
    assert base64.b64decode((OUT / "plain.xml").read_bytes()) == bins["plain"]
    for path in sorted(OUT.iterdir()):
        print(path.name, path.stat().st_size)


main()
