"""Tests of `clear_caches`: what it removes from a `target/`, what it keeps, and when it leaves
a tree alone."""
import io
import os
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

import clear_caches

DAY = 86400


def write(path: Path, age_days: float = 0.0) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * 10)
    when = time.time() - age_days * DAY
    os.utime(path, (when, when))
    return path


class Inside(unittest.TestCase):
    def test_a_path_under_the_tree_or_the_tree_itself_is_inside(self):
        tree = Path("C:/x/4cc-studio")
        self.assertTrue(clear_caches.inside("C:\\x\\4cc-studio\\target\\debug\\t.exe", tree))
        self.assertTrue(clear_caches.inside("C:\\x\\4cc-studio", tree))

    def test_a_sibling_whose_name_extends_the_trees_is_not(self):
        tree = Path("C:/x/4cc-studio")
        self.assertFalse(clear_caches.inside("C:\\x\\4cc-studio-temp\\sk_a\\cargo.exe", tree))
        self.assertFalse(clear_caches.inside("", tree))


class Clear(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.target = Path(self.tmp.name) / "target"
        deps = self.target / "debug" / "deps"
        self.caches = [
            write(self.target / "debug" / "incremental" / "a" / "x"),
            write(self.target / "tmp" / "sandbox" / "y"),
            write(self.target / "debug" / "save_editor_tests" / "t" / "z"),
        ]
        self.old = write(deps / f"libfoo-{'a' * 16}.rlib", 5)
        self.current = [write(deps / f"libfoo-{'b' * 16}.rlib", 3), write(self.target / "debug" / "utw.exe")]

    def tearDown(self):
        self.tmp.cleanup()

    def run_clear(self, dry):
        with redirect_stdout(io.StringIO()):
            return clear_caches.clear(self.target, dry)

    def test_a_dry_run_removes_nothing_and_counts_what_it_would(self):
        self.assertEqual(self.run_clear(dry=True), 40)
        for path in [*self.caches, self.old, *self.current]:
            self.assertTrue(path.exists(), path)

    def test_the_caches_scratch_and_older_builds_go_and_the_current_build_stays(self):
        self.assertEqual(self.run_clear(dry=False), 40)
        for path in [*self.caches, self.old]:
            self.assertFalse(path.exists(), path)
        for path in self.current:
            self.assertTrue(path.exists(), path)


class Over(unittest.TestCase):
    def test_a_tree_under_the_floor_is_left(self):
        with tempfile.TemporaryDirectory() as tmp:
            temp = Path(tmp)
            tree = temp / "wt"
            write(tree / ".git")
            cache = write(tree / "target" / "tmp" / "x")
            out = io.StringIO()
            with mock.patch.object(clear_caches, "TEMP", temp), redirect_stdout(out):
                code = clear_caches.main(["--over", "1", "wt"])
            self.assertEqual(code, 0)
            self.assertTrue(cache.exists())
            self.assertIn("left wt", out.getvalue())


if __name__ == "__main__":
    unittest.main()
