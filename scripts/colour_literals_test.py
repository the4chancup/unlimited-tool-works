"""Tests for colour_literals.py: what it lists and what it lets through."""

import unittest

from colour_literals import hits_in, product_files


class HitsTest(unittest.TestCase):
    def lines(self, text: str) -> list[int]:
        return [number for number, _ in hits_in(text)]

    def test_literal_colours_are_listed(self):
        text = "\n".join(
            [
                "const GREY: egui::Color32 = egui::Color32::from_rgb(0x5a, 0x5a, 0x5a);",
                "const LIGHT: Color32 = Color32::from_gray(0x9f);",
                "const DIM: egui::Color32 = egui::Color32::from_black_alpha(153);",
                "const LINES: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 51);",
                "painter.galley(at, galley, egui::Color32::WHITE);",
                "let fill = Color32::from_gray(GREY_LEVEL);",
                "let fill = Color32::from_rgb(Self::R, 0, 0);",
                "let fill = Color32::from_rgba_unmultiplied(1.0f32, 2_0, 3u8, 4);",
            ]
        )
        self.assertEqual(self.lines(text), [1, 2, 3, 4, 5, 6, 7, 8])

    def test_a_call_split_across_lines_is_read_whole(self):
        text = "\n".join(
            [
                "let fill = egui::Color32::from_rgb(",
                "    0x12,",
                "    0x96,",
                "    0x48,",
                ");",
                "pixels.push(egui::Color32::from_rgb(",
                "    channel(base[0]),",
                "    channel(base[1]),",
                "    channel(base[2]),",
                "));",
            ]
        )
        self.assertEqual(self.lines(text), [1])

    def test_computed_colours_and_non_colours_pass(self):
        text = "\n".join(
            [
                "Color32::from_rgb(r, g, b)",
                "egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])",
                "egui::Color32::from_gray(level)",
                "egui::TextFormat::simple(font, egui::Color32::PLACEHOLDER)",
                ".fill(egui::Color32::TRANSPARENT)",
                "fn tint(visuals: &egui::Visuals) -> egui::Color32 {",
                "use egui::Color32;",
                "// a comment naming Color32::WHITE",
                "    /// a doc line naming `Color32::from_rgb(1, 2, 3)`",
            ]
        )
        self.assertEqual(self.lines(text), [])

    def test_a_listed_line_keeps_its_number_after_comments(self):
        text = "\n".join(["// one", "/// two", "fn f() {", "    let c = Color32::BLACK;", "}"])
        self.assertEqual(hits_in(text), [(4, "let c = Color32::BLACK;")])

    def test_items_under_cfg_test_pass_and_the_code_after_them_does_not(self):
        text = "\n".join(
            [
                "#[cfg(test)]",
                "pub(crate) const PITCH: Color32 = Color32::WHITE;",
                "const A: Color32 = Color32::RED;",
                "    #[cfg(test)]",
                "    /// The fill a test reads.",
                "    fn probe(",
                "        x: u8,",
                "    ) -> Color32 {",
                "        Color32::from_gray(1)",
                "    }",
                "    const B: Color32 = Color32::BLUE;",
                "#[cfg(test)]",
                "const C: Color32 =",
                "    Color32::GREEN;",
                "const D: Color32 = Color32::YELLOW;",
                "#[cfg(test)]",
                "mod tests {",
                "    const E: Color32 = Color32::from_rgb(0, 92, 128);",
                "    fn f() {",
                "    }",
                "}",
                "const F: Color32 = Color32::GOLD;",
            ]
        )
        self.assertEqual(self.lines(text), [3, 11, 15, 22])


class ProductFilesTest(unittest.TestCase):
    FILES = {
        "crates/studio_core/src/theme.rs": "",
        "crates/studio_core/src/lib.rs": "",
        "crates/libs/color_tools/src/picker.rs": "",
        "crates/libs/team_widgets/src/lib.rs": "/// Shared.\n#[cfg(test)]\nmod test_support;\npub mod pitch;\n",
        "crates/libs/team_widgets/src/test_support.rs": "",
        "crates/libs/team_widgets/src/pitch/mod.rs": "#[cfg(test)]\nmod tests;\nmod lineup;\n",
        "crates/libs/team_widgets/src/pitch/tests.rs": "mod paint;\n",
        "crates/libs/team_widgets/src/pitch/tests/paint.rs": "",
        "crates/libs/team_widgets/src/pitch/lineup.rs": "",
        "crates/tools/save_editor/src/view/menus.rs": "#[cfg(test)]\nmod tests;\n",
        "crates/tools/save_editor/src/view/menus/tests.rs": "",
        "crates/tools/save_editor/tests/cli/main.rs": "",
        "crates/libs/cpk/examples/cpk_list.rs": "",
    }
    MANIFESTS = [
        "crates/studio_core/Cargo.toml",
        "crates/libs/color_tools/Cargo.toml",
        "crates/libs/team_widgets/Cargo.toml",
        "crates/tools/save_editor/Cargo.toml",
        "crates/libs/cpk/Cargo.toml",
    ]

    def test_only_product_code_is_read(self):
        kept = product_files(list(self.FILES), self.MANIFESTS, self.FILES.__getitem__)
        self.assertEqual(
            kept,
            [
                "crates/studio_core/src/lib.rs",
                "crates/libs/color_tools/src/picker.rs",
                "crates/libs/team_widgets/src/lib.rs",
                "crates/libs/team_widgets/src/pitch/mod.rs",
                "crates/libs/team_widgets/src/pitch/lineup.rs",
                "crates/tools/save_editor/src/view/menus.rs",
            ],
        )


if __name__ == "__main__":
    unittest.main()
