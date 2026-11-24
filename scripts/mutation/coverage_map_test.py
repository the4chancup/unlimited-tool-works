"""Tests of `coverage_map.py`: which tests `listed_tests` keeps from a nextest
listing, which lcov lines `covered_lines` counts, and how `selection` builds each
mutant's filter. Run by `just test`
(`python scripts/mutation/coverage_map_test.py`)."""

import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import coverage_map  # noqa: E402


class ListedTests(unittest.TestCase):
    LISTING = {
        "rust-suites": {
            "alpha": {
                "binary-id": "alpha",
                "binary-path": "/tree/target/debug/deps/alpha-111",
                "cwd": "/tree/crates/alpha",
                "testcases": {
                    "tests::kept": {"kind": "test", "ignored": False},
                    "tests::skipped": {"kind": "test", "ignored": True},
                },
            },
            "beta": {
                "binary-id": "beta",
                "binary-path": "/tree/target/debug/deps/beta-222",
                "cwd": "/tree/crates/beta",
                "testcases": {
                    "tests::other": {"kind": "test", "ignored": False},
                },
            },
        }
    }

    def test_non_ignored_kept_with_fields(self):
        tests = coverage_map.listed_tests(json.loads(json.dumps(self.LISTING)))
        self.assertEqual(
            tests,
            [
                {
                    "binary_id": "alpha",
                    "name": "tests::kept",
                    "binary": "/tree/target/debug/deps/alpha-111",
                    "cwd": "/tree/crates/alpha",
                },
                {
                    "binary_id": "beta",
                    "name": "tests::other",
                    "binary": "/tree/target/debug/deps/beta-222",
                    "cwd": "/tree/crates/beta",
                },
            ],
        )


class CoveredLines(unittest.TestCase):
    def test_counts_inside_src_only(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            inside = root / "crates" / "thing" / "src" / "lib.rs"
            outside = root / "crates" / "other" / "src" / "lib.rs"
            lcov = "\n".join(
                [
                    f"SF:{inside}",
                    "DA:10,0",
                    "DA:11,3",
                    "DA:12,1,checksum",
                    f"SF:{outside}",
                    "DA:20,5",
                ]
            )
            covered = coverage_map.covered_lines(lcov, root, "crates/thing/src/")
        self.assertEqual(
            covered,
            {
                ("crates/thing/src/lib.rs", 11),
                ("crates/thing/src/lib.rs", 12),
            },
        )


class Selection(unittest.TestCase):
    TESTS = [
        {"binary_id": "crate", "name": "tests::one", "binary": "b1", "cwd": "."},
        {"binary_id": "crate", "name": "tests::two", "binary": "b2", "cwd": "."},
        {"binary_id": "crate", "name": "tests::three", "binary": "b3", "cwd": "."},
    ]
    FILE = "crates/x/src/lib.rs"

    def mutant(self, start: int, end: int) -> dict:
        return {
            "file": self.FILE,
            "span": {"start": {"line": start}, "end": {"line": end}},
        }

    def test_covering_test_kept(self):
        coverage = {(self.FILE, 10): {0}}
        self.assertEqual(
            coverage_map.selection([self.mutant(10, 15)], coverage, self.TESTS),
            ["(binary_id(=crate) & test(=tests::one))"],
        )

    def test_uncovered_span_is_none(self):
        coverage = {(self.FILE, 10): {0}}
        self.assertEqual(
            coverage_map.selection([self.mutant(20, 22)], coverage, self.TESTS),
            [None],
        )

    def test_terms_in_test_index_order(self):
        # The span's last line and a middle line are covered, its first is not.
        coverage = {(self.FILE, 30): {2}, (self.FILE, 20): {0}}
        self.assertEqual(
            coverage_map.selection([self.mutant(5, 30)], coverage, self.TESTS),
            [
                "(binary_id(=crate) & test(=tests::one)) | "
                "(binary_id(=crate) & test(=tests::three))"
            ],
        )

    def test_output_keeps_the_list_order(self):
        coverage = {(self.FILE, 10): {0}}
        self.assertEqual(
            coverage_map.selection(
                [self.mutant(20, 22), self.mutant(10, 15)], coverage, self.TESTS
            ),
            [None, "(binary_id(=crate) & test(=tests::one))"],
        )


class Main(unittest.TestCase):
    TESTS = [
        {"binary_id": "c", "name": "tests::read", "binary": "b", "cwd": "."},
        {"binary_id": "c", "name": "tests::lost", "binary": "b", "cwd": "."},
    ]

    def run_main(
        self, notes: list, argv: list[str] | None = None,
        extra_rows: list[list[str]] | None = None,
    ) -> tuple[int, Path, str]:
        """`main` in a temporary root with the toolchain steps replaced: the two TESTS,
        each covering line 1 of `src/lib.rs`, with `notes[i]` as test i's tooling note.
        `argv` defaults to the `--mutants` form; `extra_rows` stubs the extra kinds'
        lister on a `crates/c` crate whose `src/lib.rs` is six two-byte lines, test 0
        also covering its line 4.
        The exit code, the selection file's path and what went to stderr."""
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        here = os.getcwd()
        os.chdir(root)
        self.addCleanup(os.chdir, here)
        listed = [{"file": "src/lib.rs", "span": {"start": {"line": 1}, "end": {"line": 1}}}]
        (root / "mutants.json").write_text(json.dumps(listed), encoding="utf-8")
        if extra_rows is not None:
            (root / "crates" / "c" / "src").mkdir(parents=True)
            (root / "crates" / "c" / "src" / "lib.rs").write_bytes(b"1\n2\n3\n4\n5\n6\n")

        def one(index, test, env, profiles, root, src, programs):
            lines = set() if notes[index] else {("src/lib.rs", 1)}
            if extra_rows is not None and index == 0:
                lines.add(("crates/c/src/lib.rs", 4))
            return {"index": index, "code": 0, "lines": lines, "note": notes[index]}

        err = io.StringIO()
        with (
            mock.patch.object(coverage_map, "coverage_env", return_value={}),
            mock.patch.object(coverage_map, "package_layout", return_value=("src/", [])),
            mock.patch.object(coverage_map, "nextest_list", return_value=self.TESTS),
            mock.patch.object(coverage_map, "one_coverage", side_effect=one),
            mock.patch.object(
                coverage_map.extra_mutants, "list_mutants",
                lambda directory: extra_rows or [],
            ),
            mock.patch.object(
                coverage_map.extra_mutants, "crate_dir",
                lambda crate, base: (base / "crates" / "c").resolve(),
            ),
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(err),
        ):
            code = coverage_map.main(
                argv or ["c", "selection.json", "--mutants", "mutants.json"]
            )
        return code, root / "selection.json", err.getvalue()

    def test_a_test_whose_coverage_cannot_be_read_fails_the_map(self):
        code, out, err = self.run_main([None, "no profraw"])
        self.assertEqual(code, 1)
        self.assertFalse(out.exists())
        self.assertIn("tests::lost: no profraw", err)

    def test_a_map_read_whole_is_written(self):
        code, out, _ = self.run_main([None, None])
        self.assertEqual(code, 0)
        written = json.loads(out.read_text(encoding="utf-8"))["mutants"]
        self.assertEqual(
            written,
            [{
                "file": "src/lib.rs", "start": 1, "end": 1,
                "filter": "(binary_id(=c) & test(=tests::read)) | (binary_id(=c) & test(=tests::lost))",
            }],
        )

    def test_an_extra_row_is_mapped_to_the_lines_its_bytes_span(self):
        """`--extra` alone lists the extra kinds' mutants and writes one entry
        per row: the crate-relative file, the 1-based lines its byte range
        spans (bytes 4-9 of the six-line file: lines 3 to 5), the filter of the
        tests covering any of them."""
        row = ["src/lib.rs", "4", "10", "call", "3", "", "drop f() | removed: f()"]
        code, out, _ = self.run_main(
            [None, None], argv=["c", "selection.json", "--extra"], extra_rows=[row]
        )
        self.assertEqual(code, 0)
        written = json.loads(out.read_text(encoding="utf-8"))["mutants"]
        self.assertEqual(
            written,
            [{
                "file": "crates/c/src/lib.rs", "start": 3, "end": 5,
                "filter": "(binary_id(=c) & test(=tests::read))",
            }],
        )

    def test_the_cargo_mutants_entries_come_first(self):
        row = ["src/lib.rs", "0", "2", "bool", "1", "false", "flip | removed: true"]
        code, out, _ = self.run_main(
            [None, None],
            argv=["c", "selection.json", "--mutants", "mutants.json", "--extra"],
            extra_rows=[row],
        )
        self.assertEqual(code, 0)
        written = json.loads(out.read_text(encoding="utf-8"))["mutants"]
        self.assertEqual(
            [entry["file"] for entry in written],
            ["src/lib.rs", "crates/c/src/lib.rs"],
        )

    def test_neither_option_is_an_error(self):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertEqual(coverage_map.main(["c", "selection.json"]), 2)
        self.assertIn("--extra", err.getvalue())


class OneCoverage(unittest.TestCase):
    TEST = {"binary_id": "c", "name": "tests::one", "binary": "b", "cwd": "."}

    def note(self, profile_pids: list[int]) -> str | None:
        """`one_coverage`'s note for test 3, whose process has id 100, when the
        profiles left hold one for each of `profile_pids`. The tools fail with "x", so
        a test past the profile check is noted `llvm-profdata: x`."""
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        profiles = Path(temp.name)

        def run_test(test, profile, env):
            for pid in profile_pids:
                (profiles / f"3-{pid}-0.profraw").write_bytes(b"")
            return 0, 100

        failed = subprocess.CompletedProcess([], 1, "", "x")
        with (
            mock.patch.object(coverage_map, "run_test", side_effect=run_test),
            mock.patch.object(coverage_map, "llvm_tool", return_value="tool"),
            mock.patch.object(coverage_map.subprocess, "run", return_value=failed),
        ):
            entry = coverage_map.one_coverage(3, self.TEST, {}, profiles, profiles, "src/", [])
        return entry["note"]

    def test_a_child_s_profile_alone_does_not_stand_for_the_test_s(self):
        self.assertEqual(self.note([200]), "the test's own process wrote no profile")

    def test_the_test_s_own_profile_is_read(self):
        self.assertEqual(self.note([100, 200]), "llvm-profdata: x")


if __name__ == "__main__":
    unittest.main()
