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
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.tree = Path(self.tmp.name).resolve() / "4cc-studio"

    def tearDown(self):
        self.tmp.cleanup()

    def test_a_path_under_the_tree_or_the_tree_itself_is_inside(self):
        self.assertTrue(clear_caches.inside(str(self.tree / "target" / "debug" / "t.exe"), self.tree))
        self.assertTrue(clear_caches.inside(str(self.tree), self.tree))

    def test_a_sibling_whose_name_extends_the_trees_is_not(self):
        sibling = self.tree.parent / "4cc-studio-temp" / "sk_a" / "cargo.exe"
        self.assertFalse(clear_caches.inside(str(sibling), self.tree))
        self.assertFalse(clear_caches.inside("", self.tree))


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


class Process:
    """A stand-in for a `psutil.Process`: what `busy` reads of one."""

    def __init__(self, name, exe="", cwd="", env=None):
        self.info = {"name": name, "exe": exe}
        self._cwd, self._env = cwd, env or {}

    def cwd(self):
        return self._cwd

    def environ(self):
        return self._env


class Busy(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.tree = Path(self.tmp.name).resolve() / "wt"
        self.elsewhere = str(Path(self.tmp.name).resolve() / "other")

    def tearDown(self):
        self.tmp.cleanup()

    def busy_with(self, *processes):
        with mock.patch.object(clear_caches.psutil, "process_iter", return_value=list(processes)):
            return clear_caches.busy(self.tree)

    def test_a_builder_working_in_the_tree_makes_it_busy(self):
        self.assertEqual(self.busy_with(Process("cargo.exe", cwd=str(self.tree / "crates"))), "cargo.exe")

    def test_a_builder_building_into_its_target_from_another_tree_makes_it_busy(self):
        staging = Process("rustc.exe", cwd=self.elsewhere, env={"CARGO_TARGET_DIR": str(self.tree / "target")})
        self.assertEqual(self.busy_with(staging), "rustc.exe")

    def test_a_test_binary_run_from_its_target_makes_it_busy(self):
        test = Process("aatf-0123.exe", exe=str(self.tree / "target" / "debug" / "deps" / "aatf-0123.exe"))
        self.assertEqual(self.busy_with(test), "aatf-0123.exe")

    def test_the_app_built_there_or_another_program_working_there_does_not(self):
        app = Process("utw.exe", exe=str(self.tree / "target" / "debug" / "utw.exe"))
        shell = Process("python.exe", exe=self.elsewhere, cwd=str(self.tree))
        builder_elsewhere = Process("cargo.exe", cwd=self.elsewhere, env={"CARGO_TARGET_DIR": self.elsewhere})
        self.assertIsNone(self.busy_with(app, shell, builder_elsewhere))

    def test_a_busy_tree_is_skipped_with_exit_code_1(self):
        write(self.tree / ".git")
        cache = write(self.tree / "target" / "tmp" / "x")
        out = io.StringIO()
        cargo = Process("cargo.exe", cwd=str(self.tree))
        with (
            mock.patch.object(clear_caches, "TEMP", self.tree.parent),
            mock.patch.object(clear_caches.psutil, "process_iter", return_value=[cargo]),
            redirect_stdout(out),
        ):
            code = clear_caches.main(["wt"])
        self.assertEqual(code, 1)
        self.assertTrue(cache.exists())
        self.assertIn("skipped wt: cargo.exe runs from it", out.getvalue())


class Arguments(unittest.TestCase):
    def test_an_unknown_option_stops_the_run_before_any_removal(self):
        with tempfile.TemporaryDirectory() as tmp:
            write(Path(tmp) / "wt" / ".git")
            cache = write(Path(tmp) / "wt" / "target" / "tmp" / "x")
            with (
                mock.patch.object(clear_caches, "TEMP", Path(tmp)),
                mock.patch.object(clear_caches.psutil, "process_iter", return_value=[]),
                redirect_stdout(io.StringIO()),
                mock.patch("sys.stderr", io.StringIO()),
                self.assertRaises(SystemExit),
            ):
                clear_caches.main(["--dryrun", "wt"])
            self.assertTrue(cache.exists())


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
