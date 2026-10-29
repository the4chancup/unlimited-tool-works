"""Tests of scripts/draft_shots.py; run as `python scripts/draft_shots_test.py`."""

import tempfile
import unittest
from pathlib import Path

from PIL import Image

from draft_shots import changed_share, fragment_of, remove_shot_folder, safe_name


class SafeName(unittest.TestCase):
    def test_link_characters_are_mapped(self) -> None:
        name = safe_name("save_editor/player?save=pes21&player=701.1&x=a:b")
        for character in "?&=/:":
            self.assertNotIn(character, name)
        self.assertEqual(name, "save_editor~2Fplayer~3Fsave~3Dpes21~26player~3D701.1~26x~3Da~3Ab")

    def test_two_fragments_never_share_a_name(self) -> None:
        fragments = ["a/b", "a_b", "a~2Fb", "a%2Fb", "a?b", "a=b", "a&b", "a:b", "a~b", "a@b"]
        self.assertEqual(len({safe_name(f) for f in fragments}), len(fragments))
        for fragment in fragments:
            self.assertEqual(fragment_of(safe_name(fragment)), fragment)


class ChangedShare(unittest.TestCase):
    def test_identical_images_are_unchanged(self) -> None:
        a = Image.new("RGB", (4, 5), (10, 20, 30))
        self.assertEqual(changed_share(a, a.copy()), 0.0)

    def test_one_changed_pixel_is_its_share(self) -> None:
        a = Image.new("RGB", (4, 5), (10, 20, 30))
        b = a.copy()
        b.putpixel((3, 4), (10, 20, 31))
        self.assertEqual(changed_share(a, b), 1 / 20)

    def test_a_different_size_is_all_changed(self) -> None:
        self.assertEqual(changed_share(Image.new("RGB", (4, 5)), Image.new("RGB", (5, 4))), 1.0)


class RemoveShotFolder(unittest.TestCase):
    def test_a_folder_outside_the_shots_folder_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name)
            (folder / "a.png").write_bytes(b"png")
            with self.assertRaises(ValueError):
                remove_shot_folder(folder)
            self.assertTrue((folder / "a.png").exists())


if __name__ == "__main__":
    unittest.main()
