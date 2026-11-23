"""Tests for commit.py: `-a` and the files git does not track."""

import os
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("commit.py")


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, check=True
    ).stdout


class CommitAll(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.repo = Path(self.dir.name)
        git(self.repo, "init", "-q")
        git(self.repo, "config", "user.name", "test")
        git(self.repo, "config", "user.email", "test@example.invalid")
        (self.repo / ".gitignore").write_text("scratch/\n", encoding="utf-8")
        (self.repo / "notes.txt").write_text("one\n", encoding="utf-8")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "chore(test): start")
        self.message = self.repo / "scratch" / "MSG"
        self.message.parent.mkdir()
        self.message.write_text("chore(test): change the notes\n", encoding="utf-8")
        # Older than commit.py's bar for a message written by the command committing it.
        old = time.time() - 60
        os.utime(self.message, (old, old))
        (self.repo / "notes.txt").write_text("two\n", encoding="utf-8")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def commit_all(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), str(self.message), "-a"],
            cwd=self.repo, capture_output=True, text=True,
        )

    def test_an_untracked_file_stops_the_commit_and_is_named(self) -> None:
        (self.repo / "design.txt").write_text("new\n", encoding="utf-8")
        head = git(self.repo, "rev-parse", "HEAD")
        run = self.commit_all()
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertIn("design.txt", run.stdout)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD"), head)

    def test_an_ignored_file_lets_the_commit_through(self) -> None:
        (self.repo / "scratch" / "notes.log").write_text("log\n", encoding="utf-8")
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_a_staged_new_file_goes_into_the_commit(self) -> None:
        (self.repo / "design.txt").write_text("new\n", encoding="utf-8")
        git(self.repo, "add", "design.txt")
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        files = git(self.repo, "show", "--name-only", "--format=", "HEAD").split()
        self.assertEqual(sorted(files), ["design.txt", "notes.txt"])


if __name__ == "__main__":
    unittest.main()
