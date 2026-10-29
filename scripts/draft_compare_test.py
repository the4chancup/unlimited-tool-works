"""Tests of `draft_compare.py`'s pure parts: the bands, the dump's parsing, the matching."""

import unittest

import draft_compare


class Band(unittest.TestCase):
    def band(self, delta):
        return draft_compare.band("text", "Text", delta)

    def test_an_offset_of_one_or_less_is_within_1(self):
        self.assertEqual(self.band([0.0, -1.0, 0.4, 1.0]), "within 1 px")

    def test_an_offset_over_one_up_to_two_is_1_to_2(self):
        self.assertEqual(self.band([1.1, 0.0, 0.0, 0.0]), "1 to 2 px")
        self.assertEqual(self.band([0.0, 0.0, -2.0, 0.0]), "1 to 2 px")

    def test_an_offset_over_two_is_over_2(self):
        self.assertEqual(self.band([0.0, 2.1, 0.0, 0.0]), "over 2 px")
        self.assertEqual(self.band([0.0, 0.0, 0.0, -14.0]), "over 2 px")

    def test_the_largest_of_the_four_differences_decides(self):
        self.assertEqual(self.band([0.5, -0.5, 0.5, 2.5]), "over 2 px")

    def test_a_texts_ink_against_a_widget_is_listed_apart_whatever_the_offset(self):
        # The Appearance header: the draft's word against egui's button, arrow included.
        self.assertEqual(draft_compare.band("text", "Button", [-14.4, -0.5, 17.5, 0.0]),
                         "box kinds differ")
        self.assertEqual(draft_compare.band("text", "Button", [0.0, 0.0, 0.0, 0.0]),
                         "box kinds differ")

    def test_a_checkboxs_width_is_not_compared(self):
        # Malicia: the draft's label spans its 158-point cell, egui's checkbox its content.
        self.assertEqual(draft_compare.band("check", "CheckBox", [-0.8, -0.5, -100.0, 0.0]),
                         "within 1 px")
        self.assertEqual(draft_compare.band("check", "CheckBox", [-0.8, -2.5, -100.0, 0.0]),
                         "over 2 px")

    def test_two_widgets_are_banded(self):
        self.assertEqual(draft_compare.band("button", "Button", [0.0, -2.0, 0.0, 0.0]), "1 to 2 px")


class ParseDump(unittest.TestCase):
    def test_rows_move_into_client_coordinates_and_keep_a_names_first_line(self):
        dump = "\n".join([
            "ControlType.Button | Save | 400,150 34x18",
            "ControlType.ComboBox | 4cc Autumn 26",
            "The ruleset the checks use | 900,150 190x18",
            "ControlType.Text |  | 10,10 5x5",
            "ControlType.Text | broken | 10,x 5x5",
        ])
        rows = draft_compare.parse_dump(dump, (336, 142))
        self.assertEqual(rows, [
            ["Button", "Save", 64, 8, 34, 18],
            ["ComboBox", "4cc Autumn 26", 564, 8, 190, 18],
        ])

    def test_a_window_left_of_the_main_monitor_has_negative_coordinates(self):
        rows = draft_compare.parse_dump("ControlType.Button | Save | -1792,336 34x18", (-1856, 328))
        self.assertEqual(rows, [["Button", "Save", 64, 8, 34, 18]])


class Pair(unittest.TestCase):
    def test_a_text_met_on_both_sides_pairs_in_reading_order(self):
        draft = [["text", "Left", 466.0, 214.0, 108.0, 18.0], ["text", "Left", 466.0, 192.0, 108.0, 18.0]]
        app = [["Text", "Left", 465.0, 192.0, 108.0, 18.0], ["Text", "Left", 465.0, 215.0, 108.0, 18.0]]
        pairs, unmatched = draft_compare.pair(draft, app)
        self.assertEqual(unmatched, [])
        self.assertEqual([p[3] for p in pairs], [[-1.0, 0.0, 0.0, 0.0], [-1.0, 1.0, 0.0, 0.0]])

    def test_a_text_with_different_counts_is_unmatched(self):
        draft = [["text", "Few", 338.0, 463.0, 64.0, 20.0], ["text", "Few", 550.0, 463.0, 64.0, 20.0]]
        app = [["Text", "Few", 338.0, 463.0, 64.0, 18.0]]
        pairs, unmatched = draft_compare.pair(draft, app)
        self.assertEqual(pairs, [])
        self.assertEqual(unmatched, [("Few", 2, 1)])


if __name__ == "__main__":
    unittest.main()
