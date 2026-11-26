"""Tests of `stale_deps.stale`: which units of a deps folder are older builds of another."""
import os
import tempfile
import time
import unittest
from pathlib import Path

from stale_deps import stale

DAY = 86400


class Stale(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.deps = Path(self.tmp.name) / "deps"
        self.deps.mkdir()

    def tearDown(self):
        self.tmp.cleanup()

    def unit(self, stem, hash_, suffixes, root=None, age_days=3.0, clippy=False):
        """The files of one unit, `age_days` old; a `.d` naming `root` when given."""
        when = time.time() - age_days * DAY
        files = []
        for suffix in suffixes:
            path = self.deps / f"{stem}-{hash_}{suffix}"
            path.write_bytes(b"x")
            files.append(path)
        if root is not None:
            depinfo = self.deps / f"{stem.removeprefix('lib')}-{hash_}.d"
            sources = (["C:\\repo\\clippy.toml"] if clippy else []) + [root]
            depinfo.write_text(f"{depinfo}: {' '.join(sources)}\n", encoding="utf-8")
            files.append(depinfo)
        for path in files:
            os.utime(path, (when, when))
        return files

    def names(self, files):
        return sorted(f.name for f in files)

    def test_an_older_build_of_the_same_unit_is_stale(self):
        old = self.unit("libfoo", "a" * 16, [".rlib", ".rmeta"], "crates\\foo\\src\\lib.rs", 5)
        self.unit("libfoo", "b" * 16, [".rlib", ".rmeta"], "crates\\foo\\src\\lib.rs", 3)
        self.assertEqual(self.names(stale(self.deps, 1)), self.names(old))

    def test_two_crates_tests_of_one_name_are_both_kept(self):
        self.unit("cli", "a" * 16, [".exe", ".pdb"], "crates\\x\\tests\\cli\\main.rs", 5)
        self.unit("cli", "b" * 16, [".exe", ".pdb"], "crates\\y\\tests\\cli\\main.rs", 3)
        self.assertEqual(stale(self.deps, 1), [])

    def test_two_versions_of_a_dependency_are_both_kept(self):
        self.unit("libwindows_sys", "a" * 16, [".rlib"], "registry\\windows-sys-0.52.0\\src\\lib.rs", 5)
        self.unit("libwindows_sys", "b" * 16, [".rlib"], "registry\\windows-sys-0.60.2\\src\\lib.rs", 3)
        self.assertEqual(stale(self.deps, 1), [])

    def test_a_check_beside_a_build_of_one_crate_is_kept(self):
        self.unit("libfoo", "a" * 16, [".rmeta"], "crates\\foo\\src\\lib.rs", 5, clippy=True)
        self.unit("libfoo", "b" * 16, [".rlib", ".rmeta"], "crates\\foo\\src\\lib.rs", 3)
        self.assertEqual(stale(self.deps, 1), [])

    def test_clippy_toml_before_the_root_does_not_merge_two_checks(self):
        self.unit("libfoo", "a" * 16, [".rmeta"], "crates\\foo\\src\\lib.rs", 5, clippy=True)
        self.unit("libbar", "b" * 16, [".rmeta"], "crates\\bar\\src\\lib.rs", 3, clippy=True)
        self.assertEqual(stale(self.deps, 1), [])

    def test_an_older_build_newer_than_the_floor_is_kept(self):
        self.unit("libfoo", "a" * 16, [".rlib"], "crates\\foo\\src\\lib.rs", 0.5)
        self.unit("libfoo", "b" * 16, [".rlib"], "crates\\foo\\src\\lib.rs", 0.1)
        self.assertEqual(stale(self.deps, 1), [])

    def test_units_with_no_dep_info_are_grouped_by_stem(self):
        old = self.unit("libfoo", "a" * 16, [".rlib"], None, 5)
        self.unit("libfoo", "b" * 16, [".rlib"], None, 3)
        self.unit("libbar", "c" * 16, [".rlib"], None, 5)
        self.assertEqual(self.names(stale(self.deps, 1)), self.names(old))


if __name__ == "__main__":
    unittest.main()
