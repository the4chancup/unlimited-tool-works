"""Tests for corner_radius.py: what it lists and what it lets through."""

import unittest

from corner_radius import THEME, is_test_file, numbered_radius, product_files


class NumberedRadiusTest(unittest.TestCase):
    def test_a_number_as_a_radius_is_listed(self):
        for line in [
            "let rounding = egui::CornerRadius::same(2);",
            "frame.corner_radius(4.0)",
            "shape.corner_radius(0)",
            "let a = x.corner_radius(radius); let b = CornerRadius::same(6);",
        ]:
            self.assertTrue(numbered_radius(line), line)

    def test_a_radius_from_the_style_or_a_constant_passes(self):
        for line in [
            "egui::CornerRadius::same(RADIUS)",
            "frame.corner_radius(ui.visuals().widgets.inactive.corner_radius)",
            ".corner_radius(theme::CORNER)",
            "let corner_radius = 2;",
        ]:
            self.assertFalse(numbered_radius(line), line)


class FilesTest(unittest.TestCase):
    def test_files_of_tests_alone_are_skipped(self):
        self.assertTrue(is_test_file("crates/tools/team_compiler/src/view/tests.rs"))
        self.assertTrue(is_test_file("crates/tools/team_compiler/src/view/tests/grid.rs"))
        self.assertTrue(is_test_file("crates/tools/team_compiler/tests/cli/textures.rs"))
        self.assertFalse(is_test_file("crates/tools/team_compiler/src/view/toolbar.rs"))
        self.assertFalse(is_test_file("crates/libs/export_grid/src/contests.rs"))

    def test_the_product_files_are_read_and_the_theme_is_not(self):
        files = product_files()
        self.assertIn("crates/tools/team_compiler/src/view/upgrade_dpfl.rs", files)
        self.assertIn("crates/studio_core/src/widgets/mod.rs", files)
        self.assertNotIn(THEME, files)
        self.assertFalse([path for path in files if is_test_file(path)])


if __name__ == "__main__":
    unittest.main()
