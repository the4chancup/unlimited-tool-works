"""Tests of `checkpoint.py`'s comparison: `python scripts/library_parity/checkpoint_test.py`."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import checkpoint  # noqa: E402

FACE = ("4cc_90_test.cpk", "common/character0/model/character/face/real/73905.cpk")
KIT = ("4cc_90_test.cpk", "common/character0/model/character/uniform/texture/u0714p1.dds")


class EntryChanges(unittest.TestCase):
    def test_equal_checkpoints_give_no_line(self):
        entries = {FACE: ("100", "aa"), KIT: ("200", "bb")}
        self.assertEqual(checkpoint.entry_changes(entries, dict(entries)), [])

    def test_each_kind_of_change_is_named(self):
        member = (FACE[0], FACE[1] + ".d/face.xml")
        old = {FACE: ("100", "aa"), KIT: ("200", "bb")}
        new = {FACE: ("120", "cc"), member: ("30", "dd")}
        self.assertEqual(
            checkpoint.entry_changes(old, new),
            [
                "~ " + "\t".join(FACE) + "\t100 -> 120 bytes",
                "+ " + "\t".join(member),
                "- " + "\t".join(KIT),
            ],
        )

    def test_same_size_and_other_hash_is_a_change(self):
        self.assertEqual(
            checkpoint.entry_changes({KIT: ("200", "bb")}, {KIT: ("200", "ee")}),
            ["~ " + "\t".join(KIT) + "\t200 -> 200 bytes"],
        )


if __name__ == "__main__":
    unittest.main()
