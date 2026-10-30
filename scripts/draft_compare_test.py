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


class Within(unittest.TestCase):
    def test_only_the_rows_inside_the_menu_named_by_it_are_kept(self):
        # The Player menu's entries, and the view's own Save button under the toolbar's.
        draft = [["button", "Make Gold", 348.0, 26.0, 160.0, 18.0],
                 ["button", "Save", 348.0, 47.0, 160.0, 18.0]]
        app = [["Button", "Make Gold", 349.0, 26.0, 160.0, 18.0],
               ["Button", "Save", 349.0, 47.0, 160.0, 18.0],
               ["Button", "Save", 128.0, 8.0, 34.0, 18.0],
               ["Button", "Make Gold", 64.0, 684.0, 118.0, 20.0],
               ["Text", "General info", 339.0, 57.0, 67.0, 15.0]]
        self.assertEqual(draft_compare.within(draft, app), app[:2])

    def test_a_row_reaching_past_the_margin_is_dropped(self):
        draft = [["button", "Save", 100.0, 100.0, 50.0, 18.0]]
        app = [["Button", "Save", 69.0, 100.0, 50.0, 18.0], ["Button", "Save", 70.0, 100.0, 50.0, 18.0]]
        self.assertEqual(draft_compare.within(draft, app), app[1:])


def text_image(colour, stem_width, x=10):
    """A 60 x 20 image on the dark panel's grey (27) holding one 'glyph': a vertical stem of
    `stem_width` columns in `colour`, with a half-strength edge column on each side."""
    from PIL import Image

    image = Image.new("RGB", (60, 20), (27, 27, 27))
    half = tuple((c + 27) // 2 for c in colour)
    for y in range(4, 16):
        image.putpixel((x - 1, y), half)
        for dx in range(stem_width):
            image.putpixel((x + dx, y), colour)
        image.putpixel((x + stem_width, y), half)
    return image


class Ink(unittest.TestCase):
    def test_the_core_colour_is_the_stems_not_the_edges(self):
        colour, _ = draft_compare.ink(text_image((220, 220, 220), 2), (0, 0, 60, 20))
        self.assertEqual(colour, (220, 220, 220))

    def test_a_blank_box_has_no_ink(self):
        self.assertIsNone(draft_compare.ink(text_image((220, 220, 220), 2), (30, 0, 20, 20)))

    def test_a_thicker_stem_carries_more_ink(self):
        _, thin = draft_compare.ink(text_image((140, 140, 140), 1), (0, 0, 60, 20))
        _, thick = draft_compare.ink(text_image((140, 140, 140), 3), (0, 0, 60, 20))
        self.assertGreater(thick, 1.5 * thin)


class InkFindings(unittest.TestCase):
    def pair(self, name, x, app_kind="Text"):
        row = ["text", name, x, 0.0, 20.0, 20.0]
        return (name, row, [app_kind, name, x, 0.0, 20.0, 20.0], [0.0] * 4)

    def test_a_colour_step_and_a_weight_against_the_screens_median(self):
        from PIL import Image

        draft = Image.new("RGB", (120, 20), (27, 27, 27))
        built = Image.new("RGB", (120, 20), (27, 27, 27))
        # Three texts: two alike on both sides, one a brighter title in the build.
        for image, colours in ((draft, (140, 140, 220)), (built, (140, 140, 255))):
            for slot, colour in enumerate(colours):
                image.paste(text_image((colour,) * 3, 2, x=5).crop((0, 0, 20, 20)), (slot * 40, 0))
        pairs = [self.pair("Height", 0.0), self.pair("Weight", 40.0), self.pair("General info", 80.0)]
        found = draft_compare.ink_findings(pairs, draft, built)
        self.assertEqual(found[0][2], 0)
        self.assertEqual(found[2][:3], ((220, 220, 220), (255, 255, 255), 35))
        self.assertAlmostEqual(found[0][3], 1.0)
        self.assertGreater(found[2][3], 1.0)

    def test_a_text_held_by_a_widget_is_measured_in_the_drafts_box(self):
        draft = text_image((220, 220, 220), 2, x=30)
        built = text_image((160, 160, 160), 2, x=30)
        # The app's box is the header's whole button, wider, from x 0; the text lines up.
        name, d, _, delta = self.pair("Appearance", 25.0)
        found = draft_compare.ink_findings([(name, d, ["Button", name, 0.0, 0.0, 60.0, 20.0], delta)],
                                           draft, built)
        self.assertEqual(found[0][:3], ((220, 220, 220), (160, 160, 160), 60))


def button_image(fill, text=(220, 220, 220)):
    """A 60 x 20 image on the dark panel's grey (27) holding a 40 x 20 button from x 10 in
    `fill`, one stem of its label in `text`."""
    from PIL import Image

    image = Image.new("RGB", (60, 20), (27, 27, 27))
    image.paste(fill, (10, 0, 50, 20))
    for y in range(4, 16):
        for x in (28, 29):
            image.putpixel((x, y), text)
    return image


class FillFindings(unittest.TestCase):
    def pair(self, draft_kind="button", app_kind="Button"):
        return ("OK", [draft_kind, "OK", 10.0, 0.0, 40.0, 20.0], [app_kind, "OK", 10.0, 0.0, 40.0, 20.0],
                [0.0] * 4)

    def test_a_widgets_fill_and_text_on_both_sides(self):
        found = draft_compare.fill_findings([self.pair()], button_image((60, 60, 60)),
                                            button_image((90, 90, 90), (255, 255, 255)))
        self.assertEqual(found[0], ((60, 60, 60), (90, 90, 90), 30,
                                    (220, 220, 220), (255, 255, 255), 35))

    def test_alike_widgets_step_zero(self):
        image = button_image((60, 60, 60))
        found = draft_compare.fill_findings([self.pair("sel", "ListItem")], image, image)
        self.assertEqual((found[0][2], found[0][5]), (0, 0))

    def test_a_text_pair_is_left_to_the_ink(self):
        image = button_image((60, 60, 60))
        self.assertEqual(draft_compare.fill_findings([self.pair("text", "Text")], image, image), {})
        self.assertEqual(draft_compare.fill_findings([self.pair("text", "Button")], image, image), {})

    def test_a_blank_widget_has_a_fill_and_no_text(self):
        draft = button_image((60, 60, 60), (60, 60, 60))
        found = draft_compare.fill_findings([self.pair()], draft, button_image((60, 60, 60)))
        self.assertEqual(found[0], ((60, 60, 60), (60, 60, 60), 0, None, None, None))

    def test_the_shifts_move_each_box_into_its_capture(self):
        # Rows relative to a dialog's title: unshifted, both boxes would read the panel's grey.
        found = draft_compare.fill_findings([("OK", ["button", "OK", 0.0, 0.0, 10.0, 20.0],
                                              ["Button", "OK", 2.0, 0.0, 10.0, 20.0], [0.0] * 4)],
                                            button_image((60, 60, 60)), button_image((60, 60, 60)),
                                            (12, 0), (10, 0))
        self.assertEqual(found[0][:3], ((60, 60, 60), (60, 60, 60), 0))


if __name__ == "__main__":
    unittest.main()
