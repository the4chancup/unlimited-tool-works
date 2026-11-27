"""Tests for commit.py: `-a` and the files git does not track, the worklog check, and the
roadmap page each worklog commit carries."""

import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "pages"))
from roadmap_test import wl  # noqa: E402

SCRIPT = Path(__file__).with_name("commit.py")
ROADMAP = Path(__file__).parent / "pages" / "roadmap.py"
sys.path.insert(0, str(SCRIPT.parent))
from commit import personal_matches  # noqa: E402


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

    def commit_all(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), str(self.message), *(args or ("-a",))],
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

    def commit_worklog(self, item: str, *args: str) -> subprocess.CompletedProcess[str]:
        """Commits a worklog whose one step is `item`, beside the roadmap script that
        checks it and writes the page, with `args` (`-a` when none)."""
        (self.repo / "scripts" / "gates").mkdir(parents=True)
        (self.repo / "scripts" / "pages").mkdir(parents=True)
        shutil.copy(ROADMAP, self.repo / "scripts" / "pages" / "roadmap.py")
        shutil.copy(ROADMAP.with_name("roadmap_layout.js"),
                    self.repo / "scripts" / "pages" / "roadmap_layout.js")
        # Markdown and Python files also run two other gates, the scripts below: stubs that pass.
        for gate in ("doc_pointers.py", "comment_refs.py"):
            (self.repo / "scripts" / "gates" / gate).write_text("", encoding="utf-8")
        (self.repo / "docs" / "maintainer").mkdir(parents=True)
        (self.repo / "docs" / "WORKLOG.md").write_text(wl(item), encoding="utf-8")
        git(self.repo, "add", "-A")
        return self.commit_all(*args)

    def test_a_worklog_item_breaking_the_convention_stops_the_commit(self) -> None:
        head = git(self.repo, "rev-parse", "HEAD")
        run = self.commit_worklog("- a line with no checkbox and ID.\n")
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertIn("9: no checkbox and ID", run.stdout)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD"), head)

    def test_a_worklog_keeping_the_convention_is_committed(self) -> None:
        run = self.commit_worklog("- [ ] **A** a step.\n")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertIn("docs/WORKLOG.md", git(self.repo, "show", "--name-only", "--format=", "HEAD"))

    def assert_page_committed(self, run: subprocess.CompletedProcess[str]) -> None:
        """The commit holds the roadmap page made from its worklog, and leaves no change to it."""
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        page = git(self.repo, "show", "HEAD:docs/maintainer/roadmap.html")
        self.assertIn("a step only this worklog has", page)
        self.assertEqual(git(self.repo, "status", "--porcelain", "--", "docs"), "")

    def test_a_worklog_commit_carries_the_roadmap_made_from_it(self) -> None:
        self.assert_page_committed(self.commit_worklog("- [ ] **A** a step only this worklog has.\n"))

    def test_a_worklog_commit_naming_its_paths_carries_the_roadmap_too(self) -> None:
        run = self.commit_worklog("- [ ] **A** a step only this worklog has.\n",
                                  "docs/WORKLOG.md", "scripts")
        self.assert_page_committed(run)

    def test_a_worklog_commit_of_the_index_carries_the_roadmap_too(self) -> None:
        run = self.commit_worklog("- [ ] **A** a step only this worklog has.\n", "-q")
        self.assert_page_committed(run)

    def test_a_worklog_commit_reports_the_pages_lines_apart_from_the_others(self) -> None:
        run = self.commit_worklog("- [ ] **A** a step only this worklog has.\n")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        rows = [row.split("\t") for row in
                git(self.repo, "show", "--numstat", "--format=", "HEAD").splitlines()]
        page = next(row for row in rows if row[2] == "docs/maintainer/roadmap.html")
        others = [row for row in rows if row is not page]
        self.assertTrue(others, "premise: the commit has other files")
        added = sum(int(row[0]) for row in others)
        deleted = sum(int(row[1]) for row in others)
        self.assertIn(
            f"commit: rebuilt by script: docs/maintainer/roadmap.html +{page[0]} -{page[1]}; "
            f"the other files: +{added} -{deleted}\n", run.stdout)

    def test_a_binary_file_beside_the_worklog_counts_no_lines(self) -> None:
        (self.repo / "picture.png").write_bytes(b"\x89PNG\x00\x01\x02")
        run = self.commit_worklog("- [ ] **A** a step only this worklog has.\n")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        rows = [row.split("\t") for row in
                git(self.repo, "show", "--numstat", "--format=", "HEAD").splitlines()]
        self.assertIn(["-", "-", "picture.png"], rows, "premise: git counts no lines in it")
        others = [row for row in rows
                  if row[2] not in ("docs/maintainer/roadmap.html", "picture.png")]
        added = sum(int(row[0]) for row in others)
        deleted = sum(int(row[1]) for row in others)
        self.assertIn(f"; the other files: +{added} -{deleted}\n", run.stdout)

    def test_a_commit_rebuilding_nothing_reports_no_scripted_lines(self) -> None:
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertNotIn("rebuilt by script", run.stdout)

    def test_a_personal_line_warns_and_the_commit_goes_on(self) -> None:
        (self.repo / "notes.txt").write_text(
            "two\nThe move waits: the maintainer is\n  away from the PC.\n", encoding="utf-8")
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertIn("commit: warning: the changes may say something personal", run.stdout)
        self.assertIn("the maintainer is away from the PC", run.stdout)
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_a_personal_message_warns_too(self) -> None:
        self.message.write_text("chore(test): change the notes\n\nThe maintainer is on holiday.\n",
                                encoding="utf-8")
        old = time.time() - 60
        os.utime(self.message, (old, old))
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertIn("commit: warning: the message may say something personal", run.stdout)

    def commit_rust(self, text: str) -> subprocess.CompletedProcess[str]:
        """Commits a new `lib.rs` holding `text`, with the repository's toolchain pin, so
        rustup finds the toolchain on a machine with no default, and a stub comment gate."""
        shutil.copy(SCRIPT.parent.parent / "rust-toolchain.toml", self.repo)
        (self.repo / "scripts" / "gates").mkdir(parents=True)
        (self.repo / "scripts" / "gates" / "comment_refs.py").write_text("", encoding="utf-8")
        (self.repo / "lib.rs").write_text(text, encoding="utf-8")
        git(self.repo, "add", "-A")
        return self.commit_all()

    def test_a_rust_file_rustfmt_would_change_stops_the_commit(self) -> None:
        head = git(self.repo, "rev-parse", "HEAD")
        run = self.commit_rust("fn main(){}\n")
        self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
        self.assertIn("rustfmt would change a Rust file", run.stdout)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD"), head)

    def test_a_formatted_rust_file_is_committed(self) -> None:
        # `async` is a keyword from the 2018 edition on: parsed as Rust 2015, this fails.
        run = self.commit_rust("pub async fn run() {}\n")
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertIn("lib.rs", git(self.repo, "show", "--name-only", "--format=", "HEAD"))

    def test_a_deleted_rust_file_is_not_checked(self) -> None:
        self.assertEqual(self.commit_rust("pub fn run() {}\n").returncode, 0)
        (self.repo / "lib.rs").unlink()
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertEqual(git(self.repo, "status", "--porcelain"), "")

    def test_a_commit_saying_nothing_personal_warns_nothing(self) -> None:
        run = self.commit_all()
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        self.assertNotIn("warning", run.stdout)


class Personal(unittest.TestCase):
    def test_each_kind_of_phrase_matches(self) -> None:
        for text in ("postponed until the maintainer is back", "the user's away",
                     "the user was sick", "the maintainer will be on vacation",
                     "while they are away from this PC", "back at the desk",
                     "away from 2000-01-01 for two days", "away for 2 weeks",
                     "taking time off", "out of office"):
            self.assertTrue(personal_matches(text), text)

    def test_work_that_comes_back_is_no_match(self) -> None:
        for text in ("the button is back", "the 262K limit is back", "the maintainer's yes",
                     "waits until the maintainer starts it", "a folder kept away from the tree",
                     "the check is off by one", "four days of logs"):
            self.assertEqual(personal_matches(text), [], text)


if __name__ == "__main__":
    unittest.main()
