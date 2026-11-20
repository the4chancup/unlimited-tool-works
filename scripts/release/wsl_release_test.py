"""Tests of `wsl_release.py`'s pure part: the path WSL mounts a Windows path at."""

import unittest

from wsl_release import wsl_path


class WslPath(unittest.TestCase):
    def test_a_drive_path_maps_under_mnt_with_the_drive_letter_lowered(self):
        self.assertEqual(wsl_path(r"C:\Data\4cc\target\dist"), "/mnt/c/Data/4cc/target/dist")

    def test_forward_slashes_and_spaces_map_alike(self):
        self.assertEqual(wsl_path("E:/Old Exports/x.git"), "/mnt/e/Old Exports/x.git")

    def test_a_path_with_no_drive_is_refused(self):
        with self.assertRaises(ValueError):
            wsl_path(r"\\server\share\x")


if __name__ == "__main__":
    unittest.main()
