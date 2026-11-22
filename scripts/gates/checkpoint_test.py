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
    what happens to the repository while it runs, `codes` to its exit code (0 unless named).
    `mutated` is where `refs/checkpoints/mutated` stands, the gated commit unless named."""

    def __init__(self, during=None, codes=None, mutated="gated-before"):
        self.head = "started"
        self.mutated = mutated
        self.dirty = ""
        self.moved = []
        self.ran = []
        self.output = ""
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
        # The mutation run goes around `just`, whose Windows shell flattens its exit code
        # to 1 (`checkpoint.MUTANTS_DIFF`); the gates still go through it.
        if command[:-1] == checkpoint.MUTANTS_DIFF:
            recipe, expected = "mutants-diff", [*checkpoint.MUTANTS_DIFF, self.mutated]
        else:
            recipe, expected = "gates-changed", ["just", "gates-changed", "gated-before"]
        # The gates run since the commit gated before, the mutants since the last commit
        # whose mutants ran: over another range either would pass on commits it never read.
        if command != expected or cwd != checkpoint.ROOT:
            raise AssertionError(f"unexpected run {command} in {cwd}")
        self.ran.append(recipe)
        self.during.get(recipe, lambda repo: None)(self)
        return types.SimpleNamespace(returncode=self.codes.get(recipe, 0))

    def run(self, mutants=True):
        out = io.StringIO()
        with mock.patch.object(checkpoint, "git", self.git), \
                mock.patch.object(checkpoint, "gated", lambda: "gated-before"), \
                mock.patch.object(checkpoint, "mutated", lambda: self.mutated), \
                mock.patch.object(checkpoint, "status", lambda limit: 0), \
                mock.patch.object(checkpoint.subprocess, "run", self.just), \
                contextlib.redirect_stdout(out):
            code = checkpoint.run(mutants)
        self.output = out.getvalue()
        return code


def commit(repo):
    repo.head = "committed-meanwhile"


def edit(repo):
    repo.dirty = " M justfile"


class Run(unittest.TestCase):
    def test_gates_that_pass_move_the_ref_to_the_commit_they_started_on(self):
        repo = Repo()
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.ran, ["gates-changed", "mutants-diff"])
        self.assertEqual(repo.moved, [(checkpoint.REF, "started"), (checkpoint.MUTATED, "started")])

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
        self.assertEqual(repo.moved, [(checkpoint.REF, "started"), (checkpoint.MUTATED, "started")])

    def test_surviving_mutants_do_not_stop_the_ref(self):
        for code in (2, 3):  # missed mutants, timeouts: every mutant ran; survivors are listed
            repo = Repo(codes={"mutants-diff": code})
            self.assertEqual(repo.run(), 0, code)
            self.assertEqual(repo.moved, [(checkpoint.REF, "started"), (checkpoint.MUTATED, "started")], code)
            self.assertIn("missed_all.txt", repo.output, code)

    def test_an_untested_run_keeps_the_ref_and_fails(self):
        for code in (1, 4, 5):  # untested mutants, a failed baseline, a coverage gap
            repo = Repo(codes={"mutants-diff": code})
            self.assertEqual(repo.run(), code, code)
            self.assertEqual(repo.moved, [], code)
            # A run that landed nothing wrote no missed_all.txt to point at.
            self.assertNotIn("missed_all.txt", repo.output, code)

    def test_without_mutants_only_the_gates_run_and_only_the_gated_ref_moves(self):
        repo = Repo()
        self.assertEqual(repo.run(mutants=False), 0)
        self.assertEqual(repo.ran, ["gates-changed"])
        self.assertEqual(repo.moved, [(checkpoint.REF, "started")])

    def test_the_mutants_run_from_the_last_commit_whose_mutants_ran(self):
        # A checkpoint without mutants left the mutated ref behind the gated one.
        repo = Repo(mutated="mutated-before")
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.ran, ["gates-changed", "mutants-diff"])
        self.assertEqual(repo.moved, [(checkpoint.REF, "started"), (checkpoint.MUTATED, "started")])

    def test_a_head_gated_but_not_mutated_runs_the_mutants_alone(self):
        repo = Repo(mutated="mutated-before")
        repo.head = "gated-before"
        self.assertEqual(repo.run(), 0)
        self.assertEqual(repo.ran, ["mutants-diff"])
        self.assertEqual(repo.moved, [(checkpoint.MUTATED, "gated-before")])

    def test_a_head_gated_but_not_mutated_runs_nothing_without_mutants(self):
        repo = Repo(mutated="mutated-before")
        repo.head = "gated-before"
        self.assertEqual(repo.run(mutants=False), 0)
        self.assertEqual(repo.ran, [])
        self.assertEqual(repo.moved, [])

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
