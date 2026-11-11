"""Tests of scripts/gates/checkpoint.py: its commit classification, and which commit a run
marks gated; `just test` runs them."""

import contextlib
import io
import types
import unittest
from unittest import mock

import checkpoint
from checkpoint import docs_only


class DocsOnly(unittest.TestCase):
    def test_docs_and_root_markdown_are_docs_only(self):
        self.assertTrue(docs_only(["docs/WORKLOG.md", "docs/spec/core/gui.md", "README.md", "CHANGELOG.md"]))

    def test_any_other_path_makes_a_code_commit(self):
        for path in ["crates/libs/cpk/src/lib.rs", "scripts/gates/checkpoint.py", "justfile", "Cargo.lock",
                     "crates/tools/team_compiler/help/01_command_line.md", "resources/bundle/README.txt"]:
            self.assertFalse(docs_only(["docs/WORKLOG.md", path]), path)

    def test_a_commit_touching_nothing_is_not_docs_only(self):
        self.assertFalse(docs_only([]))


class Repo:
    """A stand-in for the repository and the two `just` runs: `during` maps a recipe name to
    what happens to the repository while it runs, `codes` to its exit code (0 unless named)."""

    def __init__(self, during=None, codes=None):
        self.head = "started"
        self.dirty = ""
        self.moved = []
        self.ran = []
        self.during = during or {}
        self.codes = codes or {}

    def git(self, *args):
        if args == ("status", "--porcelain"):
            return self.dirty
        if args == ("rev-parse", "HEAD"):
            return self.head
        if args[0] == "update-ref":
            self.moved.append(args[1:])
            return ""
        raise AssertionError(f"unexpected git call {args}")

    def just(self, command, cwd):
        recipe = command[1]
        # Both recipes run since the commit gated before: over another range the gates would
        # pass on commits they never read.
        if command != ["just", recipe, "gated-before"] or cwd != checkpoint.ROOT:
            raise AssertionError(f"unexpected run {command} in {cwd}")
        self.ran.append(recipe)
        self.during.get(recipe, lambda repo: None)(self)
        return types.SimpleNamespace(returncode=self.codes.get(recipe, 0))

    def run(self, mutants=True):
        with mock.patch.object(checkpoint, "git", self.git), \
                mock.patch.object(checkpoint, "gated", lambda: "gated-before"), \
                mock.patch.object(checkpoint, "status", lambda limit: 0), \
                mock.patch.object(checkpoint.subprocess, "run", self.just), \
                contextlib.redirect_stdout(io.StringIO()):
            return checkpoint.run(mutants)


def commit(repo):
    repo.head = "committed-meanwhile"


def edit(repo):
    repo.dirty = " M justfile"


class Run(unittest.TestCase):
    def test_gates_that_pass_move_the_ref_to_the_commit_they_started_on(self):
        repo = Repo()
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.ran, ["gates-changed", "mutants-diff"])
        self.assertEqual(repo.moved, [(checkpoint.REF, "started")])

    def test_gates_that_fail_leave_the_ref_and_skip_the_mutation_run(self):
        repo = Repo(codes={"gates-changed": 3})
        self.assertEqual(repo.run(), 3)
        self.assertEqual(repo.ran, ["gates-changed"])
        self.assertEqual(repo.moved, [])

    def test_a_tree_that_moved_under_the_gates_leaves_the_ref(self):
        for change in (commit, edit):
            repo = Repo(during={"gates-changed": change})
            self.assertEqual(repo.run(), 1, change.__name__)
            self.assertEqual(repo.ran, ["gates-changed"], change.__name__)
            self.assertEqual(repo.moved, [], change.__name__)

    def test_a_commit_during_the_mutation_run_is_not_the_one_gated(self):
        repo = Repo(during={"mutants-diff": commit})
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.moved, [(checkpoint.REF, "started")])

    def test_surviving_mutants_do_not_stop_the_ref(self):
        repo = Repo(codes={"mutants-diff": 1})
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.moved, [(checkpoint.REF, "started")])

    def test_without_mutants_only_the_gates_run(self):
        repo = Repo()
        self.assertEqual(repo.run(mutants=False), 0)
        self.assertEqual(repo.ran, ["gates-changed"])
        self.assertEqual(repo.moved, [(checkpoint.REF, "started")])

    def test_a_dirty_tree_is_refused_before_anything_runs(self):
        repo = Repo()
        repo.dirty = " M justfile"
        with self.assertRaises(SystemExit):
            repo.run()
        self.assertEqual(repo.ran, [])
        self.assertEqual(repo.moved, [])

    def test_a_head_already_gated_runs_nothing(self):
        repo = Repo()
        repo.head = "gated-before"
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.ran, [])
        self.assertEqual(repo.moved, [])


if __name__ == "__main__":
    unittest.main()
