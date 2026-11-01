"""Cuts the body font, Roboto Flex, to the two static instances the app and the drafts bundle:
regular (wght 400) and semibold (wght 600), both at optical size 12.5, the body text's size,
which is the optical size the font trial's browser drew it at (CSS sets opsz to the font size
in px); every other axis at its default. No axis is tuned and nothing is scaled.

Run: python scripts/fonts/cut_flex.py <Roboto Flex's variable .ttf> [--write]
The variable file is Google Fonts' (`resources/fonts/README.md` names it and its SHA-256), its
OFL.txt beside it as `RobotoFlex-OFL.txt`. Without --write it only prints each instance's size,
glyph count and SHA-256. With it, the files go to resources/fonts/ and
docs/prototypes/assets/fonts/, each through a temporary file beside it, with the licence.
"""
import hashlib
import io
import shutil
import sys
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[2]
SRC = Path(sys.argv[1])
SRC_SHA = "9b523f7d82593df0107173849ebb8c817471a1df4b4fb2c3cbf40cfd810c8281"
LICENCE = SRC.parent / "RobotoFlex-OFL.txt"
OPSZ = 12.5
CUTS = {"RobotoFlex-Regular.ttf": 400.0, "RobotoFlex-Semibold.ttf": 600.0}
DESTS = [ROOT / "resources" / "fonts", ROOT / "docs" / "prototypes" / "assets" / "fonts"]

assert hashlib.sha256(SRC.read_bytes()).hexdigest() == SRC_SHA, "not the trial's Roboto Flex"
write = sys.argv[2:] == ["--write"]
assert sys.argv[2:] in ([], ["--write"]), __doc__
for dest in DESTS:
    assert dest.is_dir(), dest
for name, wght in CUTS.items():
    # The source's own timestamp is kept, so a rerun writes the same bytes.
    source = TTFont(SRC, recalcTimestamp=False)
    location = {axis.axisTag: axis.defaultValue for axis in source["fvar"].axes}
    location.update(opsz=OPSZ, wght=wght)
    static = instancer.instantiateVariableFont(source, location)
    assert "fvar" not in static
    buffer = io.BytesIO()
    static.save(buffer)
    data = buffer.getvalue()
    glyphs = len(static.getGlyphOrder())
    print(f"{name}: {len(data):,} bytes, {glyphs} glyphs, "
          f"SHA-256 {hashlib.sha256(data).hexdigest()}")
    if write:
        for dest in DESTS:
            tmp = dest / (name + ".partial")
            tmp.write_bytes(data)
            tmp.replace(dest / name)
if write:
    for dest in DESTS:
        shutil.copyfile(LICENCE, dest / "RobotoFlex-OFL.txt")
    print("written to", ", ".join(str(d) for d in DESTS))
