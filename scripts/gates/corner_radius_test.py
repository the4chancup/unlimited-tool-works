"""Tests for corner_radius.py: what it lists and what it lets through."""

import unittest

from corner_radius import (THEME, is_test_file, numbered_radius, page_files, page_radius,
                           product_files)


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


class PageRadiusTest(unittest.TestCase):
    def test_a_radius_other_than_the_suite_s_is_listed(self):
        for line in [
            ".card { border-radius: 6px; }",
            "blockquote { border-radius: 0 4px 4px 0; }",
            '<rect x="1" y="2" width="3" height="4" rx="5"/>',
            "f'<rect rx=\"8\" />'",
            "a { border-radius: 2px; } b { border-radius: 3px; }",
        ]:
            self.assertTrue(page_radius(line), line)

    def test_the_suite_s_radius_a_square_and_a_dot_pass(self):
        for line in [
            ".card { border-radius: 2px; }",
            "blockquote { border-radius: 0 2px 2px 0; }",
            ".dot { border-radius: 50%; }",
            "td { border-radius: 0; }",
            '<rect x="1" rx="2"/>',
            '<rect x="1" rx="0"/>',
            "a { border: 1px solid #383838; }",
        ]:
            self.assertFalse(page_radius(line), line)

    def test_the_pages_and_their_generators_are_read(self):
        files = page_files()
        for path in ["docs/maintainer/maintainer.css", "docs/maintainer/tasks.html",
                     "docs/web/dev_cycle.html", "scripts/pages/roadmap.py",
                     "scripts/pages/dev_cycle_map.py", "scripts/pages/crate_map.py"]:
            self.assertIn(path, files)
        self.assertNotIn("docs/prototypes/index.html", files)


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
