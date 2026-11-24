"""Tests of `select_tests.py`, the stand-in for cargo that adds a mutant's test filter:
which lines `changed_lines` reports changed, which outcomes make `test_filter` give no
filter, where `cargo_command` inserts `-E`, and that a missing variable exits 2 without
running anything. Run by `just test` (`python scripts/mutation/select_tests_test.py`)."""

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
import select_tests  # noqa: E402

SELECT_TESTS = SCRIPTS / "select_tests.py"


def write_files(root: Path, files: dict[str, bytes]) -> None:
    for name, data in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def make_trees(
    source_files: dict[str, bytes], copy_files: dict[str, bytes]
) -> tuple[tempfile.TemporaryDirectory, Path, Path]:
    """`source/` and `copy/` trees under one TemporaryDirectory: (tempdir, source, copy).
    The caller keeps the tempdir alive."""
    temp = tempfile.TemporaryDirectory()
    source = Path(temp.name) / "source"
    copy = Path(temp.name) / "copy"
    write_files(source, source_files)
    write_files(copy, copy_files)
    return temp, source, copy


def write_selection(path: Path, mutants: list[dict]) -> Path:
    path.write_text(json.dumps({"mutants": mutants}), encoding="utf-8")
    return path


class ChangedLines(unittest.TestCase):
    def test_equal(self):
        self.assertIsNone(select_tests.changed_lines(b"a\nb\n", b"a\nb\n"))

    def test_one_line(self):
        self.assertEqual(
            select_tests.changed_lines(b"1\n2\n3\n4\n5\n", b"1\n2\nX\n4\n5\n"), (3, 3)
        )

    def test_span_replaced_by_one_line(self):
        self.assertEqual(
            select_tests.changed_lines(b"1\n2\n3\n4\n5\n", b"1\nX\n5\n"), (2, 4)
        )

    def test_insertion(self):
        self.assertEqual(
            select_tests.changed_lines(b"1\n2\n3\n", b"1\n2\nX\n3\n"), (3, 3)
        )

    def test_last_line_without_trailing_newline(self):
        self.assertEqual(
            select_tests.changed_lines(b"1\n2\n3\n4\n5", b"1\n2\n3\n4\nX"), (5, 5)
        )


class TestFilter(unittest.TestCase):
    def run_filter(self, source_files, copy_files, selection):
        temp, source, copy = make_trees(source_files, copy_files)
        self.addCleanup(temp.cleanup)
        return select_tests.test_filter(selection, source, copy)

    def test_baseline(self):
        selection = [{"file": "f.rs", "start": 1, "end": 2, "filter": "t"}]
        expression, reason = self.run_filter(
            {"f.rs": b"a\nb\n"}, {"f.rs": b"a\nb\n"}, selection
        )
        self.assertIsNone(expression)
        self.assertEqual(reason, "no listed file differs from the source (the baseline)")

    def test_several_files_changed(self):
        selection = [
            {"file": "a.rs", "start": 1, "end": 1, "filter": "t"},
            {"file": "b.rs", "start": 1, "end": 1, "filter": "t"},
        ]
        expression, reason = self.run_filter(
            {"a.rs": b"1\n", "b.rs": b"1\n"}, {"a.rs": b"2\n", "b.rs": b"2\n"}, selection
        )
        self.assertIsNone(expression)
        self.assertIn("a.rs", reason)
        self.assertIn("b.rs", reason)

    def test_no_listed_mutant(self):
        selection = [{"file": "f.rs", "start": 9, "end": 10, "filter": "t"}]
        expression, reason = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertIsNone(expression)
        self.assertEqual(reason, "no listed mutant on f.rs:2-2")

    def test_null_filter(self):
        selection = [{"file": "f.rs", "start": 2, "end": 2, "filter": None}]
        expression, reason = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertIsNone(expression)
        self.assertEqual(reason, "no test covers f.rs:2-2")

    def test_filters_joined(self):
        selection = [
            {"file": "f.rs", "start": 2, "end": 2, "filter": "a"},
            {"file": "f.rs", "start": 1, "end": 3, "filter": "b"},
        ]
        expression, _ = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertEqual(expression, "a | b")

    def test_non_overlapping_entry_left_out(self):
        selection = [
            {"file": "f.rs", "start": 2, "end": 2, "filter": "a"},
            {"file": "f.rs", "start": 9, "end": 10, "filter": "b"},
        ]
        expression, _ = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertEqual(expression, "a")

    def test_filter_too_long(self):
        too_long = "x" * (select_tests.MAX_FILTER_CHARS + 1)
        selection = [{"file": "f.rs", "start": 2, "end": 2, "filter": too_long}]
        expression, reason = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertIsNone(expression)
        self.assertEqual(
            reason,
            f"the filter is too long ({select_tests.MAX_FILTER_CHARS + 1} characters)",
        )

    def test_filter_at_limit(self):
        at_limit = "x" * select_tests.MAX_FILTER_CHARS
        selection = [{"file": "f.rs", "start": 2, "end": 2, "filter": at_limit}]
        expression, _ = self.run_filter(
            {"f.rs": b"1\n2\n3\n"}, {"f.rs": b"1\nX\n3\n"}, selection
        )
        self.assertEqual(expression, at_limit)


class CargoCommand(unittest.TestCase):
    REAL = "real-cargo"

    def env(self, source: Path, selection: Path) -> dict[str, str]:
        return {
            "UTW_REAL_CARGO": self.REAL,
            "UTW_MUTANTS_SOURCE": str(source),
            "UTW_MUTANTS_SELECTION": str(selection),
        }

    def test_build_phase_passes_through(self):
        args = ["nextest", "run", "--no-run", "--workspace"]
        env = {"UTW_REAL_CARGO": self.REAL, "UTW_MUTANTS_SOURCE": "", "UTW_MUTANTS_SELECTION": ""}
        command, line = select_tests.cargo_command(args, env, Path("copy"))
        self.assertEqual(command, [self.REAL, *args])
        self.assertIsNone(line)

    def test_other_call_passes_through(self):
        args = ["metadata", "--format-version", "1"]
        env = {"UTW_REAL_CARGO": self.REAL, "UTW_MUTANTS_SOURCE": "", "UTW_MUTANTS_SELECTION": ""}
        command, line = select_tests.cargo_command(args, env, Path("copy"))
        self.assertEqual(command, [self.REAL, *args])
        self.assertIsNone(line)

    def run_test_phase(self, mutated: bytes, args: list[str]) -> tuple[list[str], str | None]:
        """`cargo_command` on `args` in a copy whose `f.rs` reads `mutated`, against a
        source `f.rs` of `1\\n2\\n` and one listed mutant on line 2 with the filter `t`."""
        temp, source, copy = make_trees({"f.rs": b"1\n2\n"}, {"f.rs": mutated})
        self.addCleanup(temp.cleanup)
        selection = write_selection(
            Path(temp.name) / "selection.json",
            [{"file": "f.rs", "start": 2, "end": 2, "filter": "t"}],
        )
        return select_tests.cargo_command(args, self.env(source, selection), copy)

    def test_test_phase_appends_filter(self):
        args = ["nextest", "run", "-p", "crate"]
        command, line = self.run_test_phase(b"1\nX\n", args)
        self.assertEqual(command, [self.REAL, *args, "-E", "t"])
        self.assertEqual(line, "select_tests: 1 mutants' filters on f.rs:2-2")

    def test_test_phase_inserts_before_separator(self):
        command, _ = self.run_test_phase(b"1\nX\n", ["nextest", "run", "--", "--skip", "slow"])
        self.assertEqual(
            command,
            [self.REAL, "nextest", "run", "-E", "t", "--", "--skip", "slow"],
        )

    def test_test_phase_baseline(self):
        args = ["nextest", "run"]
        command, line = self.run_test_phase(b"1\n2\n", args)
        self.assertEqual(command, [self.REAL, *args])
        self.assertEqual(
            line, "select_tests: no listed file differs from the source (the baseline)"
        )


class Main(unittest.TestCase):
    def test_missing_variable(self):
        err = io.StringIO()
        with (
            mock.patch.dict(os.environ, {}, clear=True),
            mock.patch("subprocess.run") as run,
            mock.patch("os.execv") as execv,
            contextlib.redirect_stderr(err),
        ):
            code = select_tests.main(["nextest", "run"])
        self.assertEqual(code, 2)
        self.assertIn("select_tests: UTW_REAL_CARGO is not set", err.getvalue())
        run.assert_not_called()
        execv.assert_not_called()

    def test_an_unreadable_selection_file_exits_2_and_runs_nothing(self):
        err = io.StringIO()
        with tempfile.TemporaryDirectory() as temp:
            variables = {
                "UTW_REAL_CARGO": "real-cargo",
                "UTW_MUTANTS_SOURCE": temp,
                "UTW_MUTANTS_SELECTION": str(Path(temp) / "missing.json"),
            }
            with (
                mock.patch.dict(os.environ, variables, clear=True),
                mock.patch("subprocess.run") as run,
                mock.patch("os.execv") as execv,
                contextlib.redirect_stderr(err),
            ):
                code = select_tests.main(["nextest", "run"])
        self.assertEqual(code, 2)
        self.assertIn("select_tests: the selection file cannot be read", err.getvalue())
        run.assert_not_called()
        execv.assert_not_called()

    @unittest.skipUnless(os.name == "posix", "os.execv runs the real cargo on POSIX only")
    def test_run_as_program(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            write_files(root / "source", {"f.rs": b"1\n2\n"})
            write_files(root / "copy", {"f.rs": b"1\nX\n"})
            selection = write_selection(
                root / "selection.json",
                [{"file": "f.rs", "start": 2, "end": 2, "filter": "test(=covers)"}],
            )
            out = root / "args.txt"
            cargo = root / "fake-cargo"
            cargo.write_text(
                '#!/bin/sh\nprintf "%s\n" "$@" > "$FAKE_CARGO_OUT"\nexit 7\n'
            )
            cargo.chmod(0o755)
            env = {
                **os.environ,
                "UTW_REAL_CARGO": str(cargo),
                "UTW_MUTANTS_SOURCE": str(root / "source"),
                "UTW_MUTANTS_SELECTION": str(selection),
                "FAKE_CARGO_OUT": str(out),
            }
            result = subprocess.run(
                [sys.executable, str(SELECT_TESTS), "nextest", "run"],
                cwd=root / "copy",
                env=env,
            )
            self.assertEqual(result.returncode, 7)
            self.assertEqual(
                out.read_text().splitlines(),
                ["nextest", "run", "-E", "test(=covers)"],
            )


if __name__ == "__main__":
    unittest.main()
