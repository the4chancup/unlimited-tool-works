"""Tests of scripts/drafts/vet_drafts.py; run as `python scripts/drafts/vet_drafts_test.py`."""

import re
import tempfile
import unittest
from pathlib import Path

from PIL import Image

from vet_drafts import crop_path, default_out, page_html, remove_export

WHY = ("The drafts' dialog sentences kept the browser's own 12.5-point paragraph margins, "
       "which no rule chose; the app's dialogs space their rows 8 under the title and 10 "
       "before the buttons, so the drafts' sentences now stand 8 from what is around them.")


def crops(page: Path) -> list[tuple[str, Path, Image.Image]]:
    return [(label, crop_path(page, label), Image.new("RGB", (300 + i, 40 + i)))
            for i, label in enumerate(("before", "after"))]


class PageHtml(unittest.TestCase):
    def test_the_caption_heads_the_page_and_the_reason_follows_it(self) -> None:
        page = page_html("caption", WHY, crops(Path("x.html")))
        self.assertIn("<h1>caption</h1>", page)
        self.assertIn('<p class="why">Why: The drafts&#x27; dialog sentences', page)
        self.assertLess(page.index("<h1>"), page.index('class="why"'))

    def test_each_crop_stands_under_its_label_at_its_own_size(self) -> None:
        page = page_html("caption", "short", crops(Path("d/x.html")))
        figures = re.findall(r'<figcaption>(\w+)</figcaption><img src="([^"]+)" '
                             r'width="(\d+)" height="(\d+)"', page)
        self.assertEqual(figures, [("before", "x_before.png", "300", "40"),
                                   ("after", "x_after.png", "301", "41")])

    def test_every_text_is_escaped(self) -> None:
        page = page_html("Make <tier> & co", "a < b", crops(Path("x.html")))
        self.assertIn("<h1>Make &lt;tier&gt; &amp; co</h1>", page)
        self.assertIn("Why: a &lt; b", page)
        self.assertNotIn("<tier>", page)

    def test_the_text_is_larger_than_the_drafts_body_text(self) -> None:
        # The drafts' body text is 12.5 points; the old sheet's PIL font was smaller still.
        page = page_html("caption", "short", crops(Path("x.html")))
        sizes = [int(size) for size in re.findall(r"font(?:-size)?: (\d+)px", page)]
        self.assertTrue(sizes, "the page sets its type sizes")
        self.assertGreaterEqual(min(sizes), 17)


class RemoveExport(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        self.exports = self.root / "proto"
        (self.exports / "work" / "docs").mkdir(parents=True)
        (self.root / "elsewhere").mkdir()

    def tearDown(self) -> None:
        self.dir.cleanup()

    def test_a_folder_under_the_exports_is_removed(self) -> None:
        remove_export(self.exports / "work", self.exports)
        self.assertFalse((self.exports / "work").exists())

    def test_a_folder_outside_the_exports_is_refused(self) -> None:
        for path in [self.root / "elsewhere", self.exports / ".." / "elsewhere", self.exports]:
            with self.assertRaises(ValueError, msg=str(path)):
                remove_export(path, self.exports)
        self.assertTrue((self.root / "elsewhere").is_dir())
        self.assertTrue((self.exports / "work").is_dir())


class DefaultOut(unittest.TestCase):
    def test_a_revision_pair_names_a_safe_page(self) -> None:
        out = default_out("HEAD~1", "work")
        self.assertEqual(out.name, "HEAD_1_work.html")
        self.assertEqual(out.parent.name, "vet")
        self.assertEqual(default_out("1f9fb937^", "1f9fb937").name, "1f9fb937__1f9fb937.html")

    def test_the_crops_lie_beside_the_page(self) -> None:
        page = Path("vet") / "a_b.html"
        self.assertEqual(crop_path(page, "before"), Path("vet") / "a_b_before.png")
        self.assertEqual(crop_path(page, "after"), Path("vet") / "a_b_after.png")


if __name__ == "__main__":
    unittest.main()
