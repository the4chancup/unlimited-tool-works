"""Builds the suite's symbols font from one SVG per symbol (plans/core/gui.md, the "Dark/light
theme" row): `resources/symbols/<codepoint>-<name>.svg` becomes the glyph at that codepoint of
`resources/fonts/symbols.ttf`, and the drafts get the same bytes in
`docs/prototypes/assets/fonts/symbols.ttf`. The symbols are the ones the views draw that the
body font lacks (→, ≡, ✔, ✖...): a string keeps its character, and egui draws the glyph from
this font, after the body font in the chain.

The font's units per em and vertical metrics are the body font's, read from its file: egui
places a glyph from a later font of the chain by centring that font's line height on the body
font's, so a font of other metrics would sit its symbols off the text's baseline (Phosphor's,
the icon font's, by 0.6 points at 12.5), where the browser aligns every baseline; with equal
metrics both draw a symbol where the drafts do. A new body font means a rebuild, nothing else.

Each SVG is drawn in 2048 units per em, y up from the baseline (a `scale(1 -1)` group flips it
into SVG's y-down), its view box's width the glyph's advance. Strokes are turned into outlines
and overlapping shapes merged (picosvg, skia-pathops), curves converted to TrueType's
quadratics; the timestamps are fixed, so a rebuild writes the same bytes.

Needs fontTools and picosvg (`pip install picosvg==0.23.0`, which brings skia-pathops); a
development tool, not a gate: run it after changing an SVG or the body font, then
`python scripts/fonts/make_symbols_test.py`, which checks the committed files are its output.

Run: python scripts/fonts/make_symbols.py
"""

import io
import os
import re
import sys
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from picosvg.svg import SVG
from picosvg.svg_pathops import skia_path, union

ROOT = Path(__file__).resolve().parents[2]
SYMBOLS = ROOT / "resources" / "symbols"
BODY_FONT = ROOT / "resources" / "fonts" / "RobotoFlex-Regular.ttf"
OUTPUTS = [ROOT / "resources" / "fonts" / "symbols.ttf",
           ROOT / "docs" / "prototypes" / "assets" / "fonts" / "symbols.ttf"]
DESIGN_UNITS_PER_EM = 2048
FAMILY = "4cc Symbols"
# 2026-10-27 00:00 UTC in the font format's seconds since 1904, for `head`'s two timestamps.
TIMESTAMP = 3_875_904_000
# Cu2Qu's tolerance, in font units: well under a pixel at any size the UI draws.
MAX_ERROR = 1.0
FILE_NAME = re.compile(r"([0-9a-f]{4,5})-[a-z0-9-]+\.svg")


class SymbolError(Exception):
    pass


def outline(svg_path):
    """The SVG's shapes as one merged outline, in SVG coordinates, and its advance."""
    svg = SVG.parse(str(svg_path))
    box = svg.view_box()
    if box is None or box.w <= 0:
        raise SymbolError(f"{svg_path.name}: no view box to take the advance from")
    shapes = list(svg.topicosvg().shapes())
    if not shapes:
        raise SymbolError(f"{svg_path.name}: no shape")
    merged = union([shape.as_cmd_seq() for shape in shapes], [shape.fill_rule for shape in shapes])
    return skia_path(merged, "nonzero"), box.w


def glyph(path, scale):
    """The TrueType glyph of a merged outline: flipped into font units' y-up, quadratic."""
    recording = DecomposingRecordingPen({})
    path.draw(TransformPen(recording, (scale, 0, 0, -scale, 0, 0)))
    pen = TTGlyphPen(None)
    # Skia winds a merged outer contour counter-clockwise in SVG's y-down; the flip makes it
    # clockwise, TrueType's direction, so the contours are not reversed here.
    recording.replay(Cu2QuPen(pen, MAX_ERROR))
    return pen.glyph()


def build():
    """The font's bytes, from the SVGs and the body font's metrics."""
    body = TTFont(BODY_FONT)
    units = body["head"].unitsPerEm
    scale = units / DESIGN_UNITS_PER_EM
    hhea, os2 = body["hhea"], body["OS/2"]

    cmap, glyphs, metrics = {}, {".notdef": TTGlyphPen(None).glyph()}, {".notdef": (units // 2, 0)}
    for svg_path in sorted(SYMBOLS.iterdir()):
        match = FILE_NAME.fullmatch(svg_path.name)
        if not match:
            raise SymbolError(f"{svg_path.name}: not <codepoint>-<name>.svg")
        codepoint = int(match.group(1), 16)
        name = f"uni{codepoint:04X}"
        path, advance = outline(svg_path)
        glyphs[name] = glyph(path, scale)
        glyphs[name].recalcBounds(None)
        cmap[codepoint] = name
        metrics[name] = (round(advance * scale), glyphs[name].xMin)
    if not cmap:
        raise SymbolError(f"no SVG in {SYMBOLS}")

    builder = FontBuilder(units, isTTF=True)
    builder.setupGlyphOrder(list(glyphs))
    builder.setupCharacterMap(cmap)
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics(metrics)
    builder.setupHorizontalHeader(ascent=hhea.ascent, descent=hhea.descent, lineGap=hhea.lineGap)
    builder.setupNameTable({"familyName": FAMILY, "styleName": "Regular"})
    builder.setupOS2(sTypoAscender=os2.sTypoAscender, sTypoDescender=os2.sTypoDescender,
                     sTypoLineGap=os2.sTypoLineGap, usWinAscent=os2.usWinAscent,
                     usWinDescent=os2.usWinDescent, fsSelection=os2.fsSelection,
                     sxHeight=os2.sxHeight, sCapHeight=os2.sCapHeight)
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
