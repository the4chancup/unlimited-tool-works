"""Tests of scripts/bundle_drafts.py; run as `python scripts/bundle_drafts_test.py`."""

import base64
import re
import tempfile
import unittest
from pathlib import Path

from bundle_drafts import CAPTURES, BundleError, bundle

ROOT = Path(__file__).resolve().parents[1]

PNG = b"\x89PNG fake bytes"
WEBP = b"RIFF\x00\x00\x00\x00WEBP fake"


class Bundle(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.drafts = Path(self.dir.name)
        (self.drafts / "tools").mkdir()
        (self.drafts / "tools" / "x.js").write_text(
            'const a = "assets/sub/b.webp";\n', encoding="utf-8")
        (self.drafts / "tools" / "y.js").write_text("const y = 1;\n", encoding="utf-8")
        (self.drafts / "assets" / "sub").mkdir(parents=True)
        (self.drafts / "assets" / "a.png").write_bytes(PNG)
        (self.drafts / "assets" / "sub" / "b.webp").write_bytes(WEBP)
        (self.drafts / "index.html").write_text(
            '<script src="tools/x.js"></script>\n<script src="tools/y.js"></script>\n'
            '<img src="assets/a.png">\n', encoding="utf-8")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def test_scripts_and_images_inline(self) -> None:
        text, scripts, images = bundle(self.drafts)
        self.assertEqual((scripts, images), (2, 2))
        self.assertNotIn('src="tools/', text)
        self.assertIn("const y = 1;", text)
        for mime, data in [("image/png", PNG), ("image/webp", WEBP)]:
            match = re.search(rf'"data:{mime};base64,([A-Za-z0-9+/=]+)"', text)
            self.assertIsNotNone(match, mime)
            self.assertEqual(base64.b64decode(match.group(1)), data)

    def test_a_ttf_asset_inlines_as_font_ttf(self) -> None:
        (self.drafts / "assets" / "sub").mkdir(parents=True, exist_ok=True)
        (self.drafts / "assets" / "sub" / "f.ttf").write_bytes(b"fakefont")
        (self.drafts / "tools" / "x.js").write_text('const f = "assets/sub/f.ttf";\n', encoding="utf-8")
        text, _, _ = bundle(self.drafts)
        match = re.search(r'"data:font/ttf;base64,([A-Za-z0-9+/=]+)"', text)
        self.assertIsNotNone(match)
        self.assertEqual(base64.b64decode(match.group(1)), b"fakefont")

    def test_a_single_quoted_asset_is_an_error(self) -> None:
        (self.drafts / "tools" / "x.js").write_text("const a = 'assets/a.png';\n", encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("assets/a.png", str(ctx.exception))

    def test_a_template_literal_asset_is_an_error(self) -> None:
        (self.drafts / "tools" / "x.js").write_text("const a = `assets/${x}.png`;\n", encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("assets/${x}.png", str(ctx.exception))

    def test_a_src_that_is_no_data_uri_is_an_error(self) -> None:
        (self.drafts / "index.html").write_text('<img src="other.png">\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("other.png", str(ctx.exception))

    def test_a_missing_asset_names_its_literal(self) -> None:
        (self.drafts / "tools" / "x.js").write_text('"assets/missing.png"\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn('"assets/missing.png"', str(ctx.exception))

    def test_an_unknown_extension_is_an_error(self) -> None:
        (self.drafts / "assets" / "a.bmp").write_bytes(b"x")
        (self.drafts / "tools" / "x.js").write_text('"assets/a.bmp"\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn('"assets/a.bmp"', str(ctx.exception))

    def test_a_script_holding_script_end_is_an_error(self) -> None:
        (self.drafts / "tools" / "x.js").write_text('const s = "</SCRIPT>";\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("tools/x.js", str(ctx.exception))

    def test_a_missing_script_names_its_path(self) -> None:
        (self.drafts / "index.html").write_text(
            '<script src="tools/zzz.js"></script>\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("tools/zzz.js", str(ctx.exception))

    def test_a_quoted_literal_holding_assets_is_an_error(self) -> None:
        (self.drafts / "tools" / "x.js").write_text(
            'const s = "label assets/a.png";\n', encoding="utf-8")
        with self.assertRaises(BundleError) as ctx:
            bundle(self.drafts)
        self.assertIn("assets/a.png", str(ctx.exception))

    def test_the_captures_folder_literal_is_kept(self) -> None:
        # The Overlay control builds a capture's file name at run time from the folder's
        # literal: the bundle keeps it, and the bundled overlay shows its missing-file line.
        (self.drafts / "tools" / "x.js").write_text(
            'const CAPTURES = "assets/captures/";\n', encoding="utf-8")
        text, _, images = bundle(self.drafts)
        self.assertIn('const CAPTURES = "assets/captures/";', text)
        self.assertEqual(images, 1)

    def test_the_real_drafts(self) -> None:
        drafts = ROOT / "docs" / "prototypes"
        text, scripts, images = bundle(drafts)
        self.assertEqual(scripts, len(list((drafts / "tools").glob("*.js"))))
        # Every asset a source names exists, and every file is named by a source.
        referenced = set()
        for src in [drafts / "index.html", *(drafts / "tools").glob("*.js")]:
            referenced |= set(re.findall(r'"(assets/[^"\n]+)"', src.read_text(encoding="utf-8")))
        referenced = {p[len("assets/"):] for p in referenced if p != CAPTURES}
        # The readme files are left out of the files side: the fonts' licence
        # note is documentation beside the assets, not an asset a page draws.
        # So are the built window's captures, which the overlay names at run time.
        files = {f.relative_to(drafts / "assets").as_posix()
                 for f in (drafts / "assets").rglob("*")
                 if f.is_file() and f.name != "README.md"}
        files = {f for f in files if not f.startswith(CAPTURES[len("assets/"):])}
        self.assertEqual(referenced, files)
        self.assertGreaterEqual(images, len(files))
        self.assertGreater(len(files), 0)
        self.assertNotIn('src="tools/', text)
        self.assertNotIn('"assets/', text.replace(f'"{CAPTURES}"', ""))


if __name__ == "__main__":
    unittest.main()
