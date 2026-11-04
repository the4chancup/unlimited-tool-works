"""Tests of `clones.py`: `python scripts/review/clones_test.py`."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import clones  # noqa: E402

# Four acting lines, the block each test below copies.
BLOCK = [
    "let mut window = Headless::new();",
    "let nodes = window.frame(vec![], draw);",
    "let bounds = entry(&nodes, \"Save\").bounds();",
    "assert!(bounds.x0 > 10.0, \"the button sits right\");",
]


def sequence(lines, first=1):
    """`lines` as `clusters`' input: (line number, shape) per line, numbered from `first`."""
    return [(first + index, clones.shape(line)) for index, line in enumerate(lines)]


class Shape(unittest.TestCase):
    def test_names_strings_and_numbers_reduce_to_one_letter_each(self):
        self.assertEqual(
            clones.shape('let width = text_width(ui, "Save") + 2.5;'),
            "let I = I ( I , S ) + N ;",
        )

    def test_comments_are_blanked_and_the_line_count_kept(self):
        text = 'let a = "// kept"; // gone\n/* one\ntwo */ let b = 1;\n'
        stripped = clones.strip_comments(text)
        self.assertEqual(stripped.count("\n"), text.count("\n"))
        self.assertIn('"// kept"', stripped)
        self.assertNotIn("gone", stripped)
        self.assertNotIn("two", stripped)


class Clusters(unittest.TestCase):
    def test_three_copies_of_a_window_are_one_cluster(self):
        sequences = {
            "a.rs": sequence(BLOCK + ["let other = 1;"] + BLOCK),
            "b.rs": sequence(BLOCK, first=40),
        }
        found = clones.clusters(sequences, window=4, long=10)
        self.assertEqual(
            found, [(4, [("a.rs", 1, 4), ("a.rs", 6, 9), ("b.rs", 40, 43)])]
        )

    def test_two_copies_shorter_than_long_are_not_a_cluster(self):
        sequences = {"a.rs": sequence(BLOCK), "b.rs": sequence(BLOCK)}
        self.assertEqual(clones.clusters(sequences, window=4, long=10), [])
        self.assertEqual(len(clones.clusters(sequences, window=4, long=4)), 1)

    def test_a_run_of_declarations_is_not_copied_logic(self):
        fields = ["pub width: f32,", "pub height: f32,", "pub name: String,", "pub id: u32,"]
        sequences = {name: sequence(fields) for name in ("a.rs", "b.rs", "c.rs")}
        self.assertEqual(clones.clusters(sequences, window=4, long=10), [])


class FileRanking(unittest.TestCase):
    def test_files_rank_by_their_lines_inside_a_run_counted_once(self):
        # Listed least repeated first: only the ranking's sort puts a.rs on top.
        sequences = {
            "c.rs": sequence(["let alone = 2;"]),
            "b.rs": sequence(BLOCK, first=40),
            "a.rs": sequence(BLOCK + ["let other = 1;"] + BLOCK),
        }
        found = clones.clusters(sequences, window=4, long=10)
        # A second cluster over a.rs's first copy covers no new line.
        found.append((2, [("a.rs", 1, 2), ("b.rs", 40, 41)]))
        self.assertEqual(
            clones.file_ranking(sequences, found),
            [("a.rs", 9, 8), ("b.rs", 4, 4), ("c.rs", 1, 0)],
        )


if __name__ == "__main__":
    unittest.main()
