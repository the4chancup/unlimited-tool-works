"""Tests of scripts/fonts/make_symbols.py; run as `python scripts/fonts/make_symbols_test.py` (needs what the
script needs: fontTools and picosvg)."""

import io
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from fontTools.pens.areaPen import AreaPen
from fontTools.ttLib import TTFont

import make_symbols


class Build(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.data = make_symbols.build()
        cls.font = TTFont(io.BytesIO(cls.data))
        cls.body = TTFont(make_symbols.BODY_FONT)

    def test_the_committed_fonts_are_the_build_of_the_committed_svgs(self) -> None:
        for output in make_symbols.OUTPUTS:
            with self.subTest(output=output.name, folder=output.parent.name):
                self.assertTrue(output.read_bytes() == self.data,
                                f"{output} differs from a rebuild: run scripts/fonts/make_symbols.py")

    def test_each_svg_is_the_glyph_at_its_codepoint_with_its_view_boxs_width(self) -> None:
        cmap = self.font.getBestCmap()
        svgs = sorted(make_symbols.SYMBOLS.glob("*.svg"))
        self.assertEqual(len(cmap), len(svgs))
        for svg in svgs:
            codepoint = int(svg.name.split("-")[0], 16)
            width = float(re.search(r'viewBox="0 -?\d+ ([\d.]+) ', svg.read_text(encoding="utf-8"))[1])
            with self.subTest(svg=svg.name):
                self.assertIn(codepoint, cmap)
                advance = self.font["hmtx"][cmap[codepoint]][0]
                self.assertEqual(advance, round(width * self.body["head"].unitsPerEm
                                                / make_symbols.DESIGN_UNITS_PER_EM))
                self.assertGreater(self.font["glyf"][cmap[codepoint]].numberOfContours, 0)

    def test_the_vertical_metrics_are_the_body_fonts(self) -> None:
        self.assertEqual(self.font["head"].unitsPerEm, self.body["head"].unitsPerEm)
        for table, fields in [("hhea", ["ascent", "descent", "lineGap"]),
                              ("OS/2", ["sTypoAscender", "sTypoDescender", "sTypoLineGap",
                                        "usWinAscent", "usWinDescent", "fsSelection"])]:
            for field in fields:
                with self.subTest(table=table, field=field):
                    self.assertEqual(getattr(self.font[table], field),
                                     getattr(self.body[table], field))

    def test_every_outline_winds_clockwise_as_truetype_draws_it(self) -> None:
        # A counter-clockwise outer contour still renders in most rasterizers, so only its
        # area's sign shows the flip went the wrong way.
        glyphs = self.font.getGlyphSet()
        for name in self.font.getBestCmap().values():
            pen = AreaPen(glyphs)
            glyphs[name].draw(pen)
            with self.subTest(glyph=name):
                self.assertLess(pen.value, 0)


class Errors(unittest.TestCase):
    def build_from(self, files: dict[str, str]) -> bytes:
        with tempfile.TemporaryDirectory() as folder:
            for name, text in files.items():
                (Path(folder) / name).write_text(text, encoding="utf-8")
            with mock.patch.object(make_symbols, "SYMBOLS", Path(folder)):
                return make_symbols.build()

    def test_a_file_named_without_its_codepoint_is_refused(self) -> None:
        svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="5" height="5"/></svg>'
        with self.assertRaisesRegex(make_symbols.SymbolError, "minus.svg: not <codepoint>-<name>.svg"):
            self.build_from({"minus.svg": svg})

    def test_an_svg_with_no_shape_is_refused(self) -> None:
        svg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"></svg>'
        with self.assertRaisesRegex(make_symbols.SymbolError, "2212-minus-sign.svg: no shape"):
            self.build_from({"2212-minus-sign.svg": svg})


if __name__ == "__main__":
    unittest.main()
