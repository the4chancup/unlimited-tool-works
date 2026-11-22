"""Tests of scripts/drafts/draft_shots.py; run as `python scripts/drafts/draft_shots_test.py`."""

import os
import tempfile
import unittest
from datetime import datetime, timedelta
from pathlib import Path

from PIL import Image

from draft_shots import (changed_share, fragment_of, full_run_reminder, match_shots,
                         prune_runs, remove_shot_folder, safe_name, shot_rows)


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
        b.putpixel((3, 4), (10, 20, 33))
        self.assertEqual(changed_share(a, b), 1 / 20)

    def test_a_pixel_one_level_off_is_unchanged(self) -> None:
        a = Image.new("RGB", (4, 5), (10, 20, 30))
        b = a.copy()
        b.putpixel((3, 4), (10, 20, 31))
        self.assertEqual(changed_share(a, b), 0.0)

    def test_a_pixel_two_levels_off_counts(self) -> None:
        a = Image.new("RGB", (4, 5), (10, 20, 30))
        b = a.copy()
        b.putpixel((3, 4), (10, 20, 32))
        self.assertEqual(changed_share(a, b), 1 / 20)

    def test_a_different_size_is_all_changed(self) -> None:
        self.assertEqual(changed_share(Image.new("RGB", (4, 5)), Image.new("RGB", (5, 4))), 1.0)


class RemoveShotFolder(unittest.TestCase):
    def test_a_folder_outside_the_shots_folder_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name)
            (folder / "a.png").write_bytes(b"png")
            with self.assertRaises(ValueError):
                remove_shot_folder(folder, Path(name) / "shots")
            self.assertTrue((folder / "a.png").exists())


def write_shot(folder: Path, name: str, colour=(10, 20, 30)) -> None:
    """Writes `name` into `folder` as a small PNG of one `colour`."""
    folder.mkdir(exist_ok=True)
    Image.new("RGB", (4, 5), colour).save(folder / name)


class MatchShots(unittest.TestCase):
    VIEWS = [["save-editor", ["save_editor/player", "save_editor/team?selteam=718"]],
             ["compiler", ["compiler/main"]]]

    def test_only_links_holding_a_text_are_kept(self) -> None:
        kept, unmatched = match_shots(self.VIEWS, ["team"])
        self.assertEqual(kept, [["save-editor", ["save_editor/team?selteam=718"]]])
        self.assertEqual(unmatched, [])

    def test_no_text_keeps_every_link(self) -> None:
        kept, unmatched = match_shots(self.VIEWS, [])
        self.assertEqual(kept, self.VIEWS)
        self.assertEqual(unmatched, [])

    def test_a_text_no_link_holds_is_named(self) -> None:
        kept, unmatched = match_shots(self.VIEWS, ["team", "zzz"])
        self.assertEqual(unmatched, ["zzz"])


class ShotRows(unittest.TestCase):
    def test_a_shot_only_an_older_run_has_is_found(self) -> None:
        # run2 holds no `a@d.png`: run3's copy is measured against run1's, not read as new.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png", (10, 20, 30))
            (root / "20250102-000000-x").mkdir()
            run = root / "20250103-000000"
            write_shot(run, "a@d.png", (10, 20, 33))
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("changed", "a@d")])

    def test_the_newest_earlier_copy_wins(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png", (10, 20, 30))
            write_shot(root / "20250102-000000", "a@d.png", (90, 90, 90))
            run = root / "20250103-000000"
            write_shot(run, "a@d.png", (90, 90, 90))
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("unchanged", "a@d")])

    def test_a_shot_no_earlier_run_has_is_new(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png")
            run = root / "20250102-000000"
            write_shot(run, "a@d.png")
            write_shot(run, "b@d.png")
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("unchanged", "a@d"), ("new", "b@d")])

    def test_baseline_is_the_oldest_run(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "baseline", "a@d.png", (10, 20, 30))
            write_shot(root / "20250101-000000", "a@d.png", (90, 90, 90))
            run = root / "20250102-000000"
            write_shot(run, "a@d.png", (90, 90, 90))
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("unchanged", "a@d")])

    def test_missing_only_on_a_full_run(self) -> None:
        # `d` is missing: the newest earlier full run had it. `c` is not: only the filtered
        # run had it, and that run is not a full one.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png")
            write_shot(root / "20250101-000000", "d@d.png")
            write_shot(root / "20250102-000000-x", "a@d.png")
            write_shot(root / "20250102-000000-x", "c@d.png")
            run = root / "20250103-000000"
            write_shot(run, "a@d.png")
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("unchanged", "a@d"), ("missing", "d@d")])
            filtered = root / "20250103-000001-x"
            write_shot(filtered, "a@d.png")
            self.assertEqual([row[:2] for row in shot_rows(root, filtered)],
                             [("unchanged", "a@d")])

    def test_missing_is_measured_from_the_newest_full_run(self) -> None:
        # `b` is not missing: only the oldest earlier full run had it, and "missing" measures
        # from the newest one.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png")
            write_shot(root / "20250101-000000", "b@d.png")
            write_shot(root / "20250102-000000", "a@d.png")
            run = root / "20250103-000000"
            write_shot(run, "a@d.png")
            self.assertEqual([row[:2] for row in shot_rows(root, run)],
                             [("unchanged", "a@d")])

    def test_a_changed_shots_detail_is_its_pixel_share(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png", (10, 20, 30))
            run = root / "20250102-000000"
            run.mkdir()
            image = Image.new("RGB", (4, 5), (10, 20, 30))
            image.putpixel((3, 4), (10, 20, 33))
            image.save(run / "a@d.png")
            self.assertEqual(shot_rows(root, run),
                             [("changed", "a@d", " (5.0000% of its pixels)")])


class PruneRuns(unittest.TestCase):
    def test_a_run_whose_shots_all_have_newer_copies_is_deleted(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "baseline", "a@d.png")
            write_shot(root / "20250101-000000", "a@d.png")
            write_shot(root / "20250102-000000", "a@d.png")
            write_shot(root / "20250103-000000", "a@d.png")
            prune_runs(root)
            self.assertEqual([f.name for f in root.iterdir()], ["20250103-000000"])

    def test_a_run_holding_a_shots_newest_copy_is_kept(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png")
            write_shot(root / "20250101-000000", "b@d.png")
            write_shot(root / "20250102-000000", "a@d.png")
            prune_runs(root)
            self.assertEqual(sorted(f.name for f in root.iterdir()),
                             ["20250101-000000", "20250102-000000"])

    def test_the_newest_full_run_is_never_deleted(self) -> None:
        # The two filtered runs cover the full run's shots between them, but the full run
        # stays: "missing" rows and the full-run reminder measure from it.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / "20250101-000000", "a@d.png")
            write_shot(root / "20250101-000000", "b@d.png")
            write_shot(root / "20250102-000000-x", "a@d.png")
            write_shot(root / "20250103-000000-x", "b@d.png")
            prune_runs(root)
            self.assertEqual(sorted(f.name for f in root.iterdir()),
                             ["20250101-000000", "20250102-000000-x",
                              "20250103-000000-x"])


class FullRunReminder(unittest.TestCase):
    def test_no_full_run_says_so(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            run = root / "20250103-000000-x"
            write_shot(run, "a@d.png")
            self.assertEqual(full_run_reminder(root, run),
                             "no full run yet; run once without a filter")

    def test_a_full_run_over_a_day_old_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            stamp = datetime.now() - timedelta(days=2)
            write_shot(root / stamp.strftime("%Y%m%d-%H%M%S"), "a@d.png")
            run = root / (datetime.now().strftime("%Y%m%d-%H%M%S") + "-x")
            write_shot(run, "a@d.png")
            self.assertEqual(full_run_reminder(root, run),
                             f"the last full run was {stamp:%Y-%m-%d %H:%M}; "
                             "run once without a filter")

    def test_a_full_run_an_hour_old_reminds_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / (datetime.now() - timedelta(hours=1)).strftime("%Y%m%d-%H%M%S"),
                       "a@d.png")
            run = root / (datetime.now().strftime("%Y%m%d-%H%M%S") + "-x")
            write_shot(run, "a@d.png")
            self.assertIsNone(full_run_reminder(root, run))

    def test_the_reminder_measures_the_newest_full_run(self) -> None:
        # The older full run is over a day old but the newer is an hour old: no reminder.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            write_shot(root / (datetime.now() - timedelta(days=2)).strftime("%Y%m%d-%H%M%S"),
                       "a@d.png")
            write_shot(root / (datetime.now() - timedelta(hours=1)).strftime("%Y%m%d-%H%M%S"),
                       "a@d.png")
            run = root / (datetime.now().strftime("%Y%m%d-%H%M%S") + "-x")
            write_shot(run, "a@d.png")
            self.assertIsNone(full_run_reminder(root, run))

    def test_a_full_run_reminds_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            run = root / "20250103-000000"
            write_shot(run, "a@d.png")
            self.assertIsNone(full_run_reminder(root, run))

    def test_baseline_reads_its_modification_time(self) -> None:
        # `baseline/`'s name holds no time; its folder's modification time stands in.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            stamp = datetime.now() - timedelta(days=2)
            write_shot(root / "baseline", "a@d.png")
            when = stamp.timestamp()
            os.utime(root / "baseline", (when, when))
            run = root / (datetime.now().strftime("%Y%m%d-%H%M%S") + "-x")
            write_shot(run, "a@d.png")
            self.assertEqual(full_run_reminder(root, run),
                             f"the last full run was {stamp:%Y-%m-%d %H:%M}; "
                             "run once without a filter")


if __name__ == "__main__":
    unittest.main()
