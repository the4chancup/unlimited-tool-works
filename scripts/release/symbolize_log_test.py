"""Tests of scripts/release/symbolize_log.py; `just doc-pointers` runs them with the others."""

import tempfile
import unittest
from pathlib import Path

from symbolize_log import (
    Refusal,
    blocks,
    exe_frames,
    frame_runs,
    symbolized,
    symbols_in,
)

# Two panic records (each a run of frame lines starting at `0:`), a frame of another
# module (the `0x7ffe...` base), a frame no module claims (the bare `0x<ip>`) and a
# third record whose frame 0 has no base.
LOG = """Unlimited Tool Works v0.0.1 pre-alpha
INFO  session: opened
ERROR panic: thread 'main' panicked at crates/studio/src/x.rs:3: first boom
  0: 0x7ff6a1230000+0x59c4
  1: 0x7ff6a1230000+0x1200
  2: 0x7ffe12340000+0x77aa
INFO  session: more
ERROR panic: thread 'main' panicked at crates/studio/src/x.rs:9: second boom
  0: 0x7ff6a1230000+0x59c4
  1: 0x1234567890ab
  2: 0x7ff6a1230000+0x2222
ERROR panic: third boom
  0: 0xabcdef0123
  1: 0x7ff6a1230000+0x1111
"""


class Runs(unittest.TestCase):
    def test_every_panic_record_is_a_run_and_the_exe_base_is_frame_0s(self) -> None:
        runs = frame_runs(LOG)
        self.assertEqual(
            [[frame.index for frame in run] for run in runs],
            [[0, 1, 2], [0, 1, 2], [0, 1]],
        )
        self.assertEqual(runs[0][0].base, 0x7FF6A1230000)
        # Only frames sharing frame 0's base are the exe's: another module's frame and
        # a baseless one stay out.
        self.assertEqual([frame.index for frame in exe_frames(runs[0])], [0, 1])
        self.assertEqual([frame.index for frame in exe_frames(runs[1])], [0, 2])
        # Frame 0 without a base leaves the run nothing to tell the exe's module by.
        self.assertEqual(exe_frames(runs[2]), [])


class Merge(unittest.TestCase):
    def test_each_exe_frames_line_is_followed_by_its_names(self) -> None:
        asked: list[list[int]] = []

        def resolve(offsets: list[int]) -> list[list[str]]:
            asked.append(list(offsets))
            return [[f"fn_{offset:x}", f"file.rs:{offset:x}"] for offset in offsets]

        out = symbolized(LOG, resolve)
        # One symbolizer call per run, on the exe frames' offsets minus 1.
        self.assertEqual(
            asked, [[0x59C4 - 1, 0x1200 - 1], [0x59C4 - 1, 0x2222 - 1]]
        )
        lines = out.splitlines()
        first = lines.index("  0: 0x7ff6a1230000+0x59c4")
        self.assertEqual(
            lines[first + 1 : first + 3],
            ["      fn_59c3", "      file.rs:59c3"],
        )
        second = lines.index("  2: 0x7ff6a1230000+0x2222")
        self.assertEqual(
            lines[second + 1 : second + 3],
            ["      fn_2221", "      file.rs:2221"],
        )
        # Other modules' frames and baseless ones are left as they are.
        dll = lines.index("  2: 0x7ffe12340000+0x77aa")
        self.assertEqual(lines[dll + 1], "INFO  session: more")
        bare = lines.index("  1: 0x1234567890ab")
        self.assertEqual(lines[bare + 1], "  2: 0x7ff6a1230000+0x2222")


class Blocks(unittest.TestCase):
    def test_pretty_print_output_splits_into_one_block_per_address(self) -> None:
        out = (
            "fn_a at a.rs:1:0\n (inlined by) fn_b at b.rs:2:0\n\n"
            "fn_c at c.rs:3:0\n\n"
        )
        self.assertEqual(
            blocks(out),
            [["fn_a at a.rs:1:0", " (inlined by) fn_b at b.rs:2:0"], ["fn_c at c.rs:3:0"]],
        )
        self.assertEqual(blocks(""), [])


class Folder(unittest.TestCase):
    def test_a_folder_without_exactly_one_exe_and_one_pdb_refuses(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            with self.assertRaises(Refusal):
                symbols_in(folder)
            (folder / "a.exe").write_bytes(b"x")
            with self.assertRaises(Refusal):
                symbols_in(folder)
            (folder / "b.pdb").write_bytes(b"x")
            self.assertEqual(symbols_in(folder), folder / "a.exe")
            (folder / "c.pdb").write_bytes(b"x")
            with self.assertRaises(Refusal):
                symbols_in(folder)


if __name__ == "__main__":
    unittest.main()
