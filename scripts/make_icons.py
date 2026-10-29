"""Builds the suite's icon font from one SVG per icon (plans/core/README.md, "GUI framework
choice: egui"): `resources/icons/<name>.svg` becomes the glyph at the codepoint `CODEPOINTS`
gives that name in `resources/fonts/icons.ttf`, and the drafts get the same bytes in
`docs/prototypes/assets/fonts/icons.ttf`. The icons are each tool's sidebar glyph and the
shell's general ones (help, settings, dismiss, warning, download); each keeps the Phosphor
codepoint its stand-in had, so every string that draws one keeps working.

Each SVG is one filled path on a 256 grid, y down. The font has 1024 units per em, ascent 960
and descent -64, every advance 1024, the metrics of the Phosphor font the icons replaced, so
an icon sits where its stand-in sat: font x = 4 * grid x, font y = 960 - 4 * grid y. The path
is read as it is, with no merging: absolute M, L, H, V, C, Q, T and Z only. A `T` after a `Q`
or a `T` continues one TrueType curve, its start point the one TrueType implies between two
off-curve points, so a glyph copied from a font keeps its points. Cubic curves are converted
to TrueType's quadratics; the timestamps are fixed, so a rebuild writes the same bytes.

Needs fontTools; a development tool, not a gate: run it after changing an SVG, then
`python scripts/make_icons_test.py`, which checks the committed files are its output.

Run: python scripts/make_icons.py
"""

import io
import os
import re
import sys
import xml.etree.ElementTree as ElementTree
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "resources" / "icons"
OUTPUTS = [ROOT / "resources" / "fonts" / "icons.ttf",
           ROOT / "docs" / "prototypes" / "assets" / "fonts" / "icons.ttf"]
FAMILY = "4cc Icons"
UNITS_PER_EM = 1024
ADVANCE = 1024
VIEW_BOX = "0 0 256 256"
# The 256 grid, y down, into font units, y up from the baseline.
GRID_TO_FONT = (4, 0, 0, -4, 0, 960)
HHEA = {"ascent": 960, "descent": -64, "lineGap": 0}
OS2 = {"sTypoAscender": 960, "sTypoDescender": -64, "sTypoLineGap": 64, "usWinAscent": 960,
       "usWinDescent": 64, "fsSelection": 64, "sxHeight": 960, "sCapHeight": 0}
# 2026-10-29 00:00 UTC in the font format's seconds since 1904, for `head`'s two timestamps.
TIMESTAMP = 3_876_076_800
# Cu2Qu's tolerance, in font units: well under a pixel at any size the UI draws.
MAX_ERROR = 1.0
CODEPOINTS = {
    "team-compiler": 0xE68E,
    "save-editor": 0xE34C,
    "stadium-compiler": 0xE102,
    "export-upgrader": 0xE52E,
    "music-player": 0xE33C,
    "music-export-editor": 0xE340,
    "match-tracker": 0xE1D6,
    "kit-config-editor": 0xE670,
    "refs-arranger": 0xE244,
    "balls-compiler": 0xE716,
    "player-aesthetics-editor": 0xE436,
    "team-creator": 0xE3D6,
    "ruleset-editor": 0xEB7A,
    "db-generator": 0xE1DE,
    "help": 0xE3E8,
    "settings": 0xE270,
    "dismiss": 0xE4F6,
    "warning": 0xE4E0,
    "download": 0xE20C,
}
SVG_PATH = "{http://www.w3.org/2000/svg}path"
PATH_DATA = re.compile(r"[MLHVCQTZ0-9.,\s-]*")
TOKEN = re.compile(r"[MLHVCQTZ]|-?(?:\d+\.?\d*|\.\d+)")
ARGUMENTS = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "Q": 4, "T": 2, "Z": 0}


class IconError(Exception):
    pass


def path_data(svg_path):
    """The `d` of the SVG's one path, after checking the SVG is the 256 grid's."""
    root = ElementTree.parse(svg_path).getroot()
    if root.get("viewBox") != VIEW_BOX:
        raise IconError(f"{svg_path.name}: the view box is not {VIEW_BOX}")
    if len(root) != 1 or root[0].tag != SVG_PATH:
        raise IconError(f"{svg_path.name}: not one path")
    data = root[0].get("d", "")
    if not PATH_DATA.fullmatch(data):
        raise IconError(f"{svg_path.name}: a command other than absolute M, L, H, V, C, Q, T, Z")
    return data


def commands(data, name):
    """The path's commands, each with its numbers; the pairs after an M's first are lines, as
    SVG reads them."""
    tokens = TOKEN.findall(data)
    i, command = 0, None
    while i < len(tokens):
        if tokens[i].isalpha():
            command = tokens[i]
            i += 1
        elif command in (None, "Z"):
            raise IconError(f"{name}: a number with no command")
        count = ARGUMENTS[command]
        numbers = tokens[i:i + count]
        if len(numbers) < count or any(token.isalpha() for token in numbers):
            raise IconError(f"{name}: {command} needs {count} numbers")
        yield command, [float(number) for number in numbers]
        i += count
        if command == "M":
            command = "L"


def draw(data, pen, name):
    """Draws an SVG path on `pen`, a Q and the T's after it as one quadratic curve."""
    start = current = None
    # The off-curve points of the quadratic curve not drawn yet, which ends at `current`.
    controls = []
    open_contour = False
    for command, values in commands(data, name):
        if controls and command != "T":
            pen.qCurveTo(*controls, current)
            controls = []
        if command == "M":
            if open_contour:
                pen.closePath()
            start = current = (values[0], values[1])
            pen.moveTo(current)
            open_contour = True
        elif not open_contour:
            raise IconError(f"{name}: {command} before an M")
        elif command == "Z":
            pen.closePath()
            open_contour, current = False, start
        elif command == "L":
            current = (values[0], values[1])
            pen.lineTo(current)
        elif command == "H":
            current = (values[0], current[1])
            pen.lineTo(current)
        elif command == "V":
            current = (current[0], values[0])
            pen.lineTo(current)
        elif command == "C":
            pen.curveTo((values[0], values[1]), (values[2], values[3]), (values[4], values[5]))
            current = (values[4], values[5])
        elif command == "Q":
            controls = [(values[0], values[1])]
            current = (values[2], values[3])
        elif not controls:
            raise IconError(f"{name}: T not after a Q or a T")
        else:
            # The control point is the last one mirrored through the current point, so the
            # current point is their midpoint, the point TrueType implies between them.
            last = controls[-1]
            controls.append((2 * current[0] - last[0], 2 * current[1] - last[1]))
            current = (values[0], values[1])
    if controls:
        pen.qCurveTo(*controls, current)
    if open_contour:
        pen.closePath()


def glyph(svg_path):
    """The TrueType glyph of one icon SVG, in font units."""
    pen = TTGlyphPen(None)
    draw(path_data(svg_path), TransformPen(Cu2QuPen(pen, MAX_ERROR), GRID_TO_FONT), svg_path.name)
    result = pen.glyph()
    result.recalcBounds(None)
    if result.numberOfContours == 0:
        raise IconError(f"{svg_path.name}: no shape")
    return result


def build():
    """The font's bytes, from the SVGs."""
    names = {svg_path.stem for svg_path in ICONS.glob("*.svg")}
    unlisted, missing = sorted(names - CODEPOINTS.keys()), sorted(CODEPOINTS.keys() - names)
    if unlisted:
        raise IconError(f"{unlisted[0]}.svg: no codepoint in CODEPOINTS")
    if missing:
        raise IconError(f"{missing[0]}.svg: missing from {ICONS}")
    first_at = {}
    for name in sorted(CODEPOINTS):
        first = first_at.setdefault(CODEPOINTS[name], name)
        if first != name:
            raise IconError(f"{first}.svg and {name}.svg: both at U+{CODEPOINTS[name]:04X}")

    cmap, glyphs, metrics = {}, {".notdef": TTGlyphPen(None).glyph()}, {".notdef": (ADVANCE, 0)}
    for name in sorted(CODEPOINTS):
        codepoint = CODEPOINTS[name]
        glyph_name = f"uni{codepoint:04X}"
        glyphs[glyph_name] = glyph(ICONS / f"{name}.svg")
        cmap[codepoint] = glyph_name
        metrics[glyph_name] = (ADVANCE, glyphs[glyph_name].xMin)

    builder = FontBuilder(UNITS_PER_EM, isTTF=True)
    builder.setupGlyphOrder(list(glyphs))
    builder.setupCharacterMap(cmap)
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics(metrics)
    builder.setupHorizontalHeader(**HHEA)
    builder.setupNameTable({"familyName": FAMILY, "styleName": "Regular"})
    builder.setupOS2(**OS2)
    builder.setupPost()
    font = builder.font
    font["head"].created = font["head"].modified = TIMESTAMP
    font.recalcTimestamp = False
    out = io.BytesIO()
    font.save(out)
    return out.getvalue()


def main():
    data = build()
    for target in OUTPUTS:
        temporary = target.with_name(target.name + ".tmp")
        temporary.write_bytes(data)
        os.replace(temporary, target)
        print(f"{target.relative_to(ROOT)}: {len(data):,} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
