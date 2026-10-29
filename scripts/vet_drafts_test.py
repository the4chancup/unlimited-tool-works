"""Tests of scripts/vet_drafts.py; run as `python scripts/vet_drafts_test.py`."""

import tempfile
import unittest
from pathlib import Path

from PIL import Image

from vet_drafts import BACKGROUND, compose, default_out, remove_export, why_lines

WHY = ("The drafts' dialog sentences kept the browser's own 12.5-point paragraph margins, "
       "which no rule chose; the app's dialogs space their rows 8 under the title and 10 "
       "before the buttons, so the drafts' sentences now stand 8 from what is around them.")


def crops(width: int, height: int) -> list[tuple[str, Image.Image]]:
    return [(label, Image.new("RGB", (width, height), (255, 255, 255)))
            for label in ("before", "after")]


class Compose(unittest.TestCase):
    def test_wide_crops_stack(self) -> None:
        # Side by side: 1248 wide, 115 tall, more than four times as wide as tall.
        sheet = compose("caption", "short", crops(600, 40))
        self.assertEqual(sheet.size, (632, 43 + 2 * (16 + 40 + 16)))

    def test_square_crops_lie_side_by_side(self) -> None:
        sheet = compose("caption", "short", crops(300, 300))
        self.assertEqual(sheet.size, (300 + 300 + 3 * 16, 43 + 16 + 300 + 16))

    def test_the_reason_starts_with_why_and_wraps_to_the_sheet(self) -> None:
        for size in [(600, 40), (300, 300)]:
            width = compose("caption", WHY, crops(*size)).width
            lines = why_lines(WHY, crops(*size))
            self.assertGreater(len(lines), 1, size)
            self.assertTrue(lines[0].startswith("Why: "), lines[0])
            self.assertEqual(" ".join(lines), "Why: " + WHY)
            self.assertLessEqual(max(len(line) for line in lines), (width - 2 * 16) // 6, size)

    def test_the_drawn_reason_ends_inside_the_sheet(self) -> None:
        for size in [(600, 40), (300, 300)]:
            sheet = compose("caption", WHY, crops(*size))
            lines = why_lines(WHY, crops(*size))
            pixels = sheet.load()
            inked = [x for y in range(24, 24 + 13 * len(lines)) for x in range(sheet.width)
                     if pixels[x, y] != BACKGROUND]
            self.assertGreater(len(inked), 0, size)
            self.assertLessEqual(max(inked), sheet.width - 16, size)


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
    def test_a_revision_pair_names_a_safe_file(self) -> None:
        out = default_out("HEAD~1", "work")
        self.assertEqual(out.name, "HEAD_1_work.png")
        self.assertEqual(out.parent.name, "vet")
        self.assertEqual(default_out("1f9fb937^", "1f9fb937").name, "1f9fb937__1f9fb937.png")


if __name__ == "__main__":
    unittest.main()
