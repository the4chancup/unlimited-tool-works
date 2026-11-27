"""Tests of `driver.py`: the queue's job folders, the shard count, both
lanes taking a job's shards, resuming after a restart and waiting, a job's
sending to the VPS, this PC's side of a sent job, the VPS lane's coverage
map and its deploy, and diff jobs with the diff command that queues them
and waits (`mutants_diff.diff_run`).
Every test runs on a temporary `driver.DRIVER` and cost table, with
cargo-mutants, the tree's extraction, the mutant listing, the cost fold and
the build-slot lock stubbed. The VPS lane's systemd and sudo are stubbed too.
Every ssh call reaches a fake VPS: bash run in a temporary home. Only `git
rev-parse` and `git archive` reach the repository. Run by `just test`
(`python scripts/mutation/driver_test.py`)."""

import contextlib
import io
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import types
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import driver  # noqa: E402
import mutants  # noqa: E402
import mutants_diff  # noqa: E402

HEAD_TREE = subprocess.run(
    ["git", "rev-parse", "HEAD^{tree}"], cwd=SCRIPTS, capture_output=True, text=True, check=True
).stdout.strip()
HEAD_COMMIT = subprocess.run(
    ["git", "rev-parse", "HEAD"], cwd=SCRIPTS, capture_output=True, text=True, check=True
).stdout.strip()
# A tree object other than HEAD's, which `git archive` can still send: a diff
# job's working tree stands in for it.
SCRIPTS_TREE = subprocess.run(
    ["git", "rev-parse", "HEAD:scripts"], cwd=SCRIPTS, capture_output=True, text=True, check=True
).stdout.strip()
# A diff adding line 2 to a file of each of two real crates: `mutants.mutant_crate`
# reads the tree's own `Cargo.toml` files.
TWO_CRATES_DIFF = b"".join(
    b"diff --git a/crates/libs/%s/src/lib.rs b/crates/libs/%s/src/lib.rs\n"
    b"--- a/crates/libs/%s/src/lib.rs\n"
    b"+++ b/crates/libs/%s/src/lib.rs\n"
    b"@@ -1,1 +1,2 @@\n"
    b" context\n"
    b"+added\n" % ((crate,) * 4)
    for crate in (b"cpk", b"fpk")
)
# The real `run_lock`, before a test patches it: a test of the tree lock holds
# the real one on a file of its own.
REAL_RUN_LOCK = mutants.run_lock
# The fake VPS's shell. On Windows a bare `bash` may be WSL's, so Git Bash's.
if os.name == "nt":
    BASH = Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Git" / "bin" / "bash.exe"
    BASH = str(BASH) if BASH.exists() else None
else:
    BASH = shutil.which("bash")
VERSION = "cargo-mutants 27.1.0"


class Stop(BaseException):
    """Ends a driver loop from its patched sleep: an `Exception` could be
    caught by the loop, and the test would hang."""


def shard_of(args: list[str]) -> str:
    return args[args.index("--shard") + 1]


def option(args: list[str], name: str) -> str | None:
    return args[args.index(name) + 1] if name in args else None


def cargo_line(script: str) -> str:
    """The line of a VPS unit script that starts cargo-mutants."""
    return next(line for line in script.splitlines() if line.startswith(". ~/.cargo/env"))


class Driver(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.driver = self.root / "driver"
        # (crate, shard) -> exit code, 0 when absent.
        self.codes: dict[tuple[str, int], int] = {}
        # Shards whose run records no outcome, or writes no outcomes file at
        # all, as one with no mutant to test may.
        self.empty: set[tuple[str, int]] = set()
        self.unwritten: set[tuple[str, int]] = set()
        # Shards whose run files a mutant ended by a signal: it went untested.
        self.untested: set[tuple[str, int]] = set()
        # Shards whose run files a catch by a test nextest stopped at its time limit.
        self.starved: set[tuple[str, int]] = set()
        self.calls: list[list[str]] = []
        self.locked_calls: list[bool] = []
        self.costed: list[Path] = []
        self.extracted: list[str] = []
        self.locked = False
        self.vps_down = False
        self.out = io.StringIO()
        self.enter([
            # `main` sets both for the VPS lane: the patches put them back.
            mock.patch.object(driver, "DRIVER", self.driver),
            mock.patch.object(driver, "LANE", "pc"),
            mock.patch.object(driver, "unreachable", set()),
            mock.patch.object(mutants, "remote_host", lambda: None),
            mock.patch.object(mutants, "LOCAL_TMPDIR", self.root / "local-tmp"),
            mock.patch.object(mutants, "COST_CACHE", self.root / "mutants-cost.json"),
            mock.patch.object(mutants_diff, "ROLLING", self.root / "mutants.rolling"),
            mock.patch.object(mutants_diff, "CHECKED", self.root / "mutants_checked.toml"),
            # The rolling job's crate list runs cargo: a test reaching it fails.
            mock.patch.object(
                driver.extra_mutants, "workspace_crates",
                lambda root: self.fail("a test reached cargo metadata"),
            ),
            mock.patch.object(mutants, "extract_tree", self.extract_tree),
            mock.patch.object(mutants, "local_mutants", self.local_mutants),
            mock.patch.object(mutants, "update_cost_cache", self.costed.append),
            mock.patch.object(mutants, "run_lock", self.run_lock),
            mock.patch.object(mutants, "local_config", lambda source: self.root / "local.toml"),
            mock.patch.object(
                mutants, "list_mutants",
                lambda selection, config, source: [{"name": str(i)} for i in range(9)],
            ),
            contextlib.redirect_stdout(self.out),
        ])
        self.assertTrue(driver.DRIVER.is_relative_to(self.root))
        self.assertTrue(mutants.COST_CACHE.is_relative_to(self.root))
        self.assertTrue(mutants_diff.ROLLING.is_relative_to(self.root))
        self.assertTrue(mutants_diff.CHECKED.is_relative_to(self.root))

    def enter(self, patches) -> None:
        for patch in patches:
            patch.__enter__()
            self.addCleanup(patch.__exit__, None, None, None)

    def extract_tree(self, tree: str, dest: Path) -> None:
        self.extracted.append(tree)
        (dest / ".cargo").mkdir(parents=True)
        (dest / ".cargo" / "mutants.toml").write_text(
            'test_tool = "nextest"\ntimeout_multiplier = 5.0\nminimum_test_timeout = 150\n',
            encoding="utf-8",
        )

    def vps_lane(self) -> None:
        """Make this the VPS lane, on the same temporary `DRIVER`: see `vps_fakes`."""
        self.vps_fakes()
        self.enter([mock.patch.object(driver, "LANE", "vps")])

    def vps_fakes(self) -> None:
        """The VPS lane's host stubbed: systemd answers that the units in
        `self.active` run, the disk has `self.free` bytes free, `sudo` is
        recorded in `self.sudo_calls`, and `run_unit` is `self.run_unit`. A
        wait raises `Stop` unless the test patches the sleep again, so a lane
        that waits where it should not fails the test rather than hanging it."""
        self.active: set[str] = set()
        self.free = 100 * 2**30
        self.sudo_calls: list[tuple[str, ...]] = []
        # (script name, its text, what DRIVER/tmp held when the unit started)
        self.units: list[tuple[str, str, list[str]]] = []
        # Shards whose unit is killed (no `exit`), or never starts (no `log`).
        self.killed: set[int] = set()
        self.unstarted: set[int] = set()
        # How a map unit ends: "built", "failed" (exit 1, no selection),
        # "killed" (no `map-exit`) or "unstarted" (no `map.log`).
        self.map_outcome = "built"
        self.enter([
            mock.patch.object(driver, "unit_active", lambda unit: unit in self.active),
            mock.patch.object(
                driver.shutil, "disk_usage", lambda path: types.SimpleNamespace(free=self.free)
            ),
            mock.patch.object(driver, "sudo", self.sudo),
            mock.patch.object(driver, "run_unit", self.run_unit),
            mock.patch.object(driver, "memory_max", lambda: 8 * 2**30),
            mock.patch.object(driver.time, "sleep", self.unexpected_sleep),
        ])

    def unexpected_sleep(self, seconds):
        raise Stop(f"the lane waited: {self.out.getvalue().splitlines()[-1:]}")

    def sudo(self, *args):
        self.sudo_calls.append(args)
        if args[:2] == ("systemctl", "stop"):
            self.active.discard(args[2])
        return subprocess.CompletedProcess(args, 0)

    def run_unit(self, script: Path, memory_max: int) -> None:
        """A map unit (`map.sh`) writes what its script would for
        `self.map_outcome`. A shard unit runs the script's cargo-mutants
        arguments through `local_mutants`, then writes the `log` and `exit`
        the script would."""
        text = script.read_text(encoding="utf-8")
        self.units.append((script.name, text, sorted(os.listdir(driver.DRIVER / "tmp"))))
        if script.name == "map.sh":
            self.map_unit(script.parent)
            return
        k = int(script.stem[len("shard-"):])
        if k in self.unstarted:
            return
        shard = script.parent / f"shard-{k}"
        (shard / "log").write_text("", encoding="utf-8")
        command = shlex.split(cargo_line(text))
        if "--remote-extra" in command:
            # An extra shard's unit: the pass's `extra_mutants.out` moved into
            # the shard's folder, with a survivor naming its crate and shard.
            crate = command[command.index("--remote-extra") + 1]
            out = shard / "extra_mutants.out"
            out.mkdir()
            (out / f"missed_{crate}.txt").write_text(
                f"extra missed in {crate} shard {k}\n", encoding="utf-8"
            )
            code = self.codes.get((crate, k), 0)
        else:
            args = command[command.index("mutants") + 1:]
            code = self.local_mutants(args, script.parent / "source").returncode
        if k not in self.killed:
            (shard / "exit").write_text(f"{code} 12\n", encoding="utf-8")

    def map_unit(self, job: Path) -> None:
        """`job`'s map unit ending as `self.map_outcome` says. A killed one
        dies after its selection is written, while it removes its build."""
        if self.map_outcome == "unstarted":
            return
        (job / "map.log").write_text("", encoding="utf-8")
        (job / "mutants.json").write_text("[]\n", encoding="utf-8")
        if self.map_outcome == "failed":
            (job / "map-exit").write_text("1\n", encoding="utf-8")
            return
        (job / "selection.json").write_text('{"mutants": []}\n', encoding="utf-8")
        if self.map_outcome == "built":
            (job / "map-exit").write_text("0\n", encoding="utf-8")

    def fake_vps(self, version: str = VERSION, fail: str | None = None) -> None:
        """`mutants.ssh` replaced by bash run in `self.home`, a temporary remote
        home whose cargo-mutants answers `version`. A command holding `fail`
        runs and then fails; while `self.vps_down` is set, every call fails
        with ssh's 255 and runs nothing. In front of each command, `sudo` logs
        its arguments to `$HOME/sudo.log` (and a `tee`'s input to
        `$HOME/tee.in`), `systemctl` answers 3, and `python3` is this Python,
        its printed paths with `/` and no `\\r`, as the VPS's would print them."""
        if BASH is None:
            self.skipTest("no bash for the fake VPS (Git Bash on Windows)")
        self.home = self.root / "vps-home"
        (self.home / ".cargo" / "bin").mkdir(parents=True)
        cargo_mutants = self.home / ".cargo" / "bin" / "cargo-mutants"
        cargo_mutants.write_bytes(f'#!/bin/bash\necho "{version}"\n'.encode())
        cargo_mutants.chmod(0o755)
        self.ssh_calls: list[str] = []
        python = shlex.quote(Path(sys.executable).as_posix())
        functions = (
            'sudo() { echo "$*" >> "$HOME/sudo.log"; '
            'if [ "$2" = tee ]; then cat > "$HOME/tee.in"; fi; }; '
            "systemctl() { return 3; }; "
            # `\134` is a backslash: a `\\` would reach Git Bash as one.
            f"python3() {{ {python} \"$@\" | tr -d '\\r' | tr '\\134' /; }}; "
        )

        def ssh(host, remote, **kwargs):
            self.ssh_calls.append(remote)
            if self.vps_down:
                nothing = "" if kwargs.get("text") else b""
                return subprocess.CompletedProcess(remote, 255, nothing, nothing)
            if fail is not None and fail in remote:
                remote = f"{remote} && false"
            if "input" not in kwargs:
                kwargs.setdefault("stdin", subprocess.DEVNULL)
            return subprocess.run(
                [BASH, "-c", functions + remote], cwd=self.home,
                env={**os.environ, "HOME": self.home.as_posix()}, **kwargs,
            )

        self.enter([
            mock.patch.object(mutants, "ssh", ssh),
            mock.patch.object(mutants, "remote_host", lambda: "bonfire"),
            mock.patch.object(mutants, "local_mutants_version", lambda: VERSION),
        ])

    def remote_queue(self) -> Path:
        return self.home / "studio-driver" / "queue"

    @contextlib.contextmanager
    def run_lock(self, path=None):
        """`mutants.run_lock`: `self.locked` is True under the build slot, or
        the path of any other lock, while it is held."""
        self.locked = True if path is None else path
        try:
            yield
        finally:
            self.locked = False

    def local_mutants(self, args, source, config=None):
        """A shard's cargo-mutants run: its outcomes (a 40-second baseline
        test phase when it runs the baseline), one survivor naming its crate
        and shard, and the exit code the test chose."""
        self.calls.append(args)
        self.locked_calls.append(self.locked)
        crate = option(args, "-p")
        k = int(shard_of(args).split("/")[0])
        out = Path(option(args, "--output")) / "mutants.out"
        out.mkdir(parents=True)
        if (crate, k) not in self.unwritten:
            outcomes = []
            if "--baseline" not in args and (crate, k) not in self.empty:
                outcomes.append({
                    "scenario": "Baseline", "summary": "Success",
                    "phase_results": [
                        {"phase": "Build", "duration": 30.0},
                        {"phase": "Test", "duration": 40.0},
                    ],
                })
            if (crate, k) in self.untested:
                outcomes.append({
                    "scenario": {"Mutant": {"name": f"killed in {crate} shard {k}"}},
                    "summary": "Failure", "phase_results": [],
                })
            if (crate, k) in self.starved:
                (out / "log").mkdir()
                (out / "log" / "starved.log").write_text(
                    "     TIMEOUT [ 180.035s] (1/1) cpk tests::starved\n", encoding="utf-8"
                )
                outcomes.append({
                    "scenario": {"Mutant": {"name": f"starved in {crate} shard {k}"}},
                    "summary": "CaughtMutant", "log_path": "log/starved.log",
                    "phase_results": [
                        {"phase": "Test", "duration": 1.0, "process_status": {"Failure": 100}}
                    ],
                })
            (out / "outcomes.json").write_text(
                json.dumps({"outcomes": outcomes}), encoding="utf-8"
            )
            (out / "missed.txt").write_text(f"missed in {crate} shard {k}\n", encoding="utf-8")
        return subprocess.CompletedProcess(args, self.codes.get((crate, k), 0))

    def queue(
        self, crate: str = "cpk", shards: int | None = 3, stamp: str | None = None,
        extra: int | None = 0,
    ) -> Path:
        if stamp is None:
            return driver.queue(crate, shards, extra)
        moment = time.strptime(stamp, "%Y%m%dT%H%M%S")
        with mock.patch.object(driver.time, "gmtime", lambda: moment):
            return driver.queue(crate, shards, extra)

    def done(self, job: Path) -> Path:
        return self.driver / "done" / job.name

    def result(self, job: Path) -> dict:
        return json.loads((self.done(job) / "result.json").read_text(encoding="utf-8"))

    def test_queue_writes_the_job_its_markers_and_its_source_and_no_temp(self):
        estimated = []

        def estimate(crates, share=1.0):
            estimated.append(crates)
            return driver.SHARD_SECONDS * 2.5

        with mock.patch.object(mutants, "estimate_seconds", estimate):
            job = driver.queue("cpk")
        self.assertEqual(estimated, [["cpk"] * 9], "the estimate covers every listed mutant")
        self.assertEqual([p.name for p in (self.driver / "queue").iterdir()], [job.name])
        self.assertRegex(job.name, r"^\d{8}T\d{6}-cpk$")
        self.assertEqual(
            sorted(p.name for p in job.iterdir()),
            ["job.json", "shard-0.todo", "shard-1.todo", "shard-2.todo", "source"],
        )
        self.assertTrue((job / "source" / ".cargo" / "mutants.toml").exists())
        spec = json.loads((job / "job.json").read_text(encoding="utf-8"))
        self.assertEqual(
            {key: spec[key] for key in ("crate", "tree", "args", "shards")},
            {"crate": "cpk", "tree": HEAD_TREE, "args": ["-p", "cpk"], "shards": 3},
        )
        self.assertEqual(self.extracted, [HEAD_TREE])
        self.assertRegex(spec["queued"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")

    def test_queue_records_head_s_commit_and_takes_the_tree_from_it(self):
        """One resolution of `HEAD`: the tree is the recorded commit's, so a
        commit landing between two reads cannot pair one with the other."""
        asked = []
        git = mutants.git

        def recording(*args, **kwargs):
            asked.append(args)
            return git(*args, **kwargs)

        with mock.patch.object(mutants, "git", recording):
            job = driver.queue("cpk", 1)
        spec = driver.read_job(job)
        self.assertEqual((spec["commit"], spec["tree"]), (HEAD_COMMIT, HEAD_TREE))
        self.assertEqual(
            asked, [("rev-parse", "HEAD"), ("rev-parse", f"{HEAD_COMMIT}^{{tree}}")]
        )
        self.assertNotIn("rolling", spec, "a job queued by hand is no rolling job")

    def test_a_diff_job_keeps_its_diff_lists_its_mutants_and_sends_the_diff(self):
        """`in.diff` holds the diff's bytes; the job runs on the given tree, its
        commit still HEAD's; the listing names `in.diff` where the job is being
        written, and the VPS gets `in.diff` beside `job.json`."""
        self.fake_vps()
        listings = []

        def list_mutants(selection, config, source):
            listings.append(selection)
            return [{"name": "a"}]

        with mock.patch.object(mutants, "list_mutants", list_mutants):
            job = driver.queue("cpk", diff=TWO_CRATES_DIFF, tree=SCRIPTS_TREE)
        self.assertEqual((job / "in.diff").read_bytes(), TWO_CRATES_DIFF)
        spec = driver.read_job(job)
        self.assertEqual(
            (spec["diff"], spec["tree"], spec["commit"], spec["args"]),
            (True, SCRIPTS_TREE, HEAD_COMMIT, ["-p", "cpk"]),
        )
        self.assertEqual(self.extracted, [SCRIPTS_TREE])
        building = job.with_name(job.name + ".tmp")
        self.assertEqual(listings, [["-p", "cpk", "--in-diff", str(building / "in.diff")]])
        self.assertEqual(
            (self.remote_queue() / job.name / "in.diff").read_bytes(), TWO_CRATES_DIFF
        )

    def test_a_sent_rolling_job_is_marked_rolling(self):
        self.fake_vps()
        job = driver.queue("cpk", 2, 0, rolling=True)
        self.assertIs(driver.read_job(job)["rolling"], True)
        self.assertTrue((job / "sent.json").exists())
        self.assertIs(
            json.loads((self.remote_queue() / job.name / "job.json").read_text(encoding="utf-8"))
            ["rolling"],
            True,
        )

    def test_a_rolling_job_that_is_not_sent_is_not_queued(self):
        """With no host, or when the send fails, the job is dropped whole:
        a rolling job runs its extra pass, which only the VPS lane runs."""
        self.assertIsNone(driver.queue("cpk", 2, 0, rolling=True))
        self.assertEqual(list((self.driver / "queue").iterdir()), [], "no job and no .tmp")
        self.assertEqual(self.out.getvalue().count("cpk not queued"), 1)
        self.fake_vps(fail="tar -xf")
        self.assertIsNone(driver.queue("cpk", 2, 0, rolling=True))
        self.assertEqual(list((self.driver / "queue").iterdir()), [], "no job and no .tmp")
        self.assertEqual(list(self.remote_queue().iterdir()), [])
        self.assertEqual(self.out.getvalue().count("cpk not queued"), 2)

    def test_shard_count(self):
        threshold = mutants.SPLIT_THRESHOLD_SECONDS
        shard = driver.SHARD_SECONDS
        for estimate, expected in [
            (0.0, 1), (threshold - 1, 1), (threshold, 2),
            (2 * shard - 1, 2), (2 * shard, 2), (2 * shard + 1, 3),
        ]:
            self.assertEqual(driver.shard_count(estimate), expected, estimate)

    def test_a_job_s_first_shard_runs_the_baseline_and_the_job_ends_in_done(self):
        job = self.queue()
        self.codes[("cpk", 1)] = 2
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/3", "1/3", "2/3"])
        for k, args in enumerate(self.calls):
            self.assertEqual(args[:2], ["-p", "cpk"])
            self.assertEqual(option(args, "--sharding"), "round-robin")
            self.assertEqual(option(args, "--output"), str(job / f"shard-{k}"))
        self.assertEqual(self.locked_calls, [True, True, True], "every shard builds under the lock")
        self.assertNotIn("--baseline", self.calls[0])
        for args in self.calls[1:]:
            self.assertEqual(option(args, "--baseline"), "skip")
            # 5 x the first shard's 40 s test phase, above the 150 s floor.
            self.assertEqual(option(args, "--timeout"), "200")
        self.assertEqual(
            self.costed, [job / f"shard-{k}" / "mutants.out" for k in range(3)],
            "each shard's costs are folded in as it ends",
        )
        self.assertFalse(job.exists())
        self.assertEqual(
            self.result(job),
            {
                "code": 2,
                "missed": ["missed in cpk shard 0", "missed in cpk shard 1", "missed in cpk shard 2"],
                "untested": [],
                "rechecked": [],
                "starved": [],
            },
        )
        self.assertFalse((self.done(job) / "source").exists())
        self.assertEqual(
            [(self.done(job) / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(3)],
            ["0", "2", "0"],
        )
        self.assertEqual(list(self.done(job).glob("shard-*.pc")), [])
        self.assertIn("cpk", self.out.getvalue().splitlines()[-1])

    def test_a_catch_by_a_test_stopped_at_the_time_limit_is_listed_and_said(self):
        """It stays caught: the job's code and lists are a run's without it."""
        job = self.queue(shards=2)
        self.starved.add(("cpk", 1))
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(
            self.result(job),
            {
                "code": 2,
                "missed": ["missed in cpk shard 0", "missed in cpk shard 1"],
                "untested": [],
                "rechecked": [],
                "starved": ["starved in cpk shard 1"],
            },
        )
        self.assertEqual(
            self.out.getvalue().count(
                f"driver: {job.name}: 1 caught only by a test stopped at the time limit "
                "(starved in result.json)"
            ),
            1,
        )

    def test_a_shard_another_lane_claimed_is_skipped_and_waited_for(self):
        job = self.queue()
        (job / "shard-1.todo").rename(job / "shard-1.vps")
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/3", "2/3"])
        self.assertTrue(job.exists())
        self.assertIsNone(driver.finish(job))
        self.assertFalse((job / "result.json").exists())
        # The other lane ends its shard: the next start finishes the job.
        (job / "shard-1" / "mutants.out").mkdir(parents=True)
        (job / "shard-1" / "mutants.out" / "missed.txt").write_text(
            "missed in cpk shard 1\n", encoding="utf-8"
        )
        (job / "shard-1.done").write_text("2", encoding="utf-8")
        (job / "shard-1.vps").unlink()
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(len(self.calls), 2)
        self.assertTrue(self.done(job).exists(), "the job finishes once its last shard is done")
        self.assertEqual(
            self.result(job)["missed"],
            ["missed in cpk shard 0", "missed in cpk shard 1", "missed in cpk shard 2"],
        )
        self.assertEqual(self.result(job)["code"], 2)

    def test_a_job_whose_finish_was_cut_short_is_finished_at_the_next_start(self):
        job = self.queue(shards=2)
        while claimed := driver.claim():
            driver.run_shard(*claimed)
        # The driver stopped inside `finish`, after it removed the source.
        mutants.remove_tree(job / "source")
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(len(self.calls), 2)
        self.assertTrue(self.done(job).exists(), "the next start finishes the job")
        self.assertEqual(self.result(job)["missed"],
                         ["missed in cpk shard 0", "missed in cpk shard 1"])

    def test_recover_returns_this_lane_s_claims_and_keeps_its_done_shards(self):
        job = self.queue(shards=4)
        (job / "shard-0.todo").rename(job / "shard-0.pc")
        (job / "shard-0" / "mutants.out").mkdir(parents=True)
        (job / "shard-0" / "mutants.out" / "partial").write_text("", encoding="utf-8")
        (job / "shard-1.todo").rename(job / "shard-1.pc")
        (job / "shard-1" / "mutants.out").mkdir(parents=True)
        (job / "shard-1.done").write_text("0", encoding="utf-8")
        (job / "shard-2.todo").rename(job / "shard-2.vps")
        # Claimed, and the driver stopped before the shard's folder was made.
        (job / "shard-3.todo").rename(job / "shard-3.pc")
        driver.recover()
        self.assertEqual(
            sorted(p.name for p in job.iterdir()),
            ["job.json", "shard-0.todo", "shard-1", "shard-1.done", "shard-2.vps",
             "shard-3.todo", "source"],
        )

    def test_a_restart_reruns_the_shard_its_lane_had_claimed(self):
        job = self.queue(shards=2)
        (job / "shard-0.todo").rename(job / "shard-0.pc")
        (job / "shard-0" / "mutants.out").mkdir(parents=True)
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        self.assertTrue(self.done(job).exists())

    def test_a_resumed_job_with_its_baseline_skips_it_on_its_first_shard_here(self):
        job = self.queue()
        (job / "baseline-pc.json").write_text(json.dumps({"timeout": 321.0}), encoding="utf-8")
        driver.run(once=True)
        for args in self.calls:
            self.assertEqual(option(args, "--baseline"), "skip")
            self.assertEqual(option(args, "--timeout"), "321")

    def test_a_failed_shard_is_done_and_the_job_s_code_ranks_it(self):
        job = self.queue()
        self.codes[("cpk", 0)] = 1
        self.codes[("cpk", 1)] = 3
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(len(self.calls), 3)
        self.assertFalse(job.exists())
        # `run_code`'s 1 outranks a timeout's 3, where the highest code would be 3.
        self.assertEqual(self.result(job)["code"], 1)

    def test_untested_mutants_make_the_job_s_code_1(self):
        job = self.queue()
        self.untested.add(("cpk", 2))
        driver.run(once=True)
        self.assertEqual(self.result(job)["untested"], ["killed in cpk shard 2"])
        self.assertEqual(self.result(job)["code"], 1)

    def test_the_older_job_s_shards_run_first(self):
        newer = self.queue("cpk", 2, stamp="20261127T120000")
        older = self.queue("fpk", 2, stamp="20261127T110000")
        driver.run(once=True)
        self.assertEqual(
            [(option(args, "-p"), shard_of(args)) for args in self.calls],
            [("fpk", "0/2"), ("fpk", "1/2"), ("cpk", "0/2"), ("cpk", "1/2")],
        )
        self.assertTrue(self.done(older).exists())
        self.assertTrue(self.done(newer).exists())

    def test_a_queued_job_s_shards_come_before_an_older_rolling_job_s_on_both_lanes(self):
        for lane, (rolling, plain) in {
            "pc": ("cpk", "fpk"), "vps": ("fmdl", "ftex"),
        }.items():
            older = self.queue(rolling, 1, stamp="20261127T110000")
            spec = driver.read_job(older)
            spec["rolling"] = True
            (older / "job.json").write_text(json.dumps(spec), encoding="utf-8")
            newer = self.queue(plain, 1, stamp="20261127T120000")
            with mock.patch.object(driver, "LANE", lane):
                self.assertEqual(driver.claim(), (newer, 0), lane)
                self.assertEqual(driver.claim(), (older, 0), lane)
                self.assertIsNone(driver.claim(), lane)

    def test_a_job_still_being_queued_is_not_claimed(self):
        job = self.queue()
        job.rename(job.with_name(job.name + ".tmp"))
        self.assertIsNone(driver.claim())

    def test_shards_are_claimed_in_number_order_past_nine(self):
        job = self.queue(shards=11)
        claimed = []
        while taken := driver.claim():
            claimed.append(taken)
        self.assertEqual(claimed, [(job, k) for k in range(11)])

    def test_a_shard_another_lane_renamed_first_is_passed_over(self):
        job = self.queue()
        rename = driver.os.rename

        def lost_race(source, target):
            if Path(source).name == "shard-0.todo":
                rename(source, job / "shard-0.vps")
                raise FileNotFoundError(source)
            rename(source, target)

        with mock.patch.object(driver.os, "rename", lost_race):
            self.assertEqual(driver.claim(), (job, 1))
        self.assertTrue((job / "shard-1.pc").exists())

    def test_a_hold_claims_nothing(self):
        job = self.queue()
        (self.driver / "hold-pc").write_text("", encoding="utf-8")
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            if len(sleeps) == 2:
                raise Stop

        with mock.patch.object(driver.time, "sleep", sleep), self.assertRaises(Stop):
            driver.run(once=True)
        self.assertEqual(sleeps, [driver.POLL_SECONDS] * 2)
        self.assertEqual(self.calls, [])
        self.assertEqual(sorted(p.name for p in job.glob("shard-*")),
                         ["shard-0.todo", "shard-1.todo", "shard-2.todo"])
        self.assertEqual(self.out.getvalue().count("hold-pc"), 1, "the hold is announced once")

    def test_without_once_the_lane_waits_for_work_and_announces_each_hold(self):
        hold = self.driver / "hold-pc"
        self.driver.mkdir(parents=True)
        hold.write_text("", encoding="utf-8")
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            if len(sleeps) == 1:
                hold.unlink()  # lifted, with nothing queued: the lane waits
            elif len(sleeps) == 2:
                hold.write_text("", encoding="utf-8")
            else:
                raise Stop

        with mock.patch.object(driver.time, "sleep", sleep), self.assertRaises(Stop):
            driver.run()
        self.assertEqual(sleeps, [driver.POLL_SECONDS] * 3)
        self.assertEqual(self.out.getvalue().count("hold-pc"), 2)

    def rolling_turns(self, queue=lambda: None) -> list[float]:
        """`driver.queue_rolling` replaced by `queue` for the test; each call's
        `time.monotonic()` is appended to the list returned."""
        turns = []

        def queue_rolling():
            turns.append(driver.time.monotonic())
            return queue()

        self.enter([mock.patch.object(driver, "queue_rolling", queue_rolling)])
        return turns

    def test_this_pc_s_lane_queues_the_rolling_job_on_an_empty_queue_and_claims_it(self):
        queued = []

        def queue():
            if queued:
                raise Stop
            queued.append(driver.queue("cpk", 2, 0))
            return queued[0]

        turns = self.rolling_turns(queue)
        with (
            mock.patch.object(driver.time, "sleep", self.unexpected_sleep),
            self.assertRaises(Stop),
        ):
            driver.run()
        self.assertEqual(len(turns), 2, "queued, then the next empty pass stopped")
        # Claimed at once: a wait would have raised Stop before either shard.
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        self.assertTrue(self.done(queued[0]).exists())

    def test_with_once_the_lane_never_queues_the_rolling_job(self):
        turns = self.rolling_turns()
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(turns, [])

    def test_the_vps_lane_never_queues_the_rolling_job(self):
        self.vps_lane()
        turns = self.rolling_turns()
        with self.assertRaises(Stop):
            driver.run()
        self.assertEqual(turns, [])

    def test_a_job_in_the_queue_keeps_the_rolling_job_out(self):
        """A sent job whose every shard runs on the VPS leaves this lane
        nothing to claim, and still no rolling job is queued beside it."""
        job, remote = self.sent_job(shards=1)
        (remote / "shard-0.todo").rename(remote / "shard-0.vps")
        turns = self.rolling_turns()
        with (
            mock.patch.object(driver.time, "sleep", self.unexpected_sleep),
            self.assertRaises(Stop),
        ):
            driver.run()
        self.assertEqual(turns, [])
        self.assertTrue(job.exists())

    def test_after_queuing_nothing_the_lane_waits_the_retry_time_before_trying_again(self):
        # Each wait moves the clock by 2/5 of the retry time: the third pass,
        # at 4/5, is inside it, and the fourth, at 6/5, past it.
        step = driver.ROLLING_RETRY_SECONDS * 2 // 5
        clock = [0]

        def sleep(seconds):
            self.assertEqual(seconds, driver.POLL_SECONDS)
            if clock[0] == 6 * step:
                raise Stop
            clock[0] += step

        turns = self.rolling_turns()
        with (
            mock.patch.object(driver.time, "monotonic", lambda: clock[0]),
            mock.patch.object(driver.time, "sleep", sleep),
            self.assertRaises(Stop),
        ):
            driver.run()
        self.assertEqual(turns, [0, 3 * step, 6 * step])

    CRATES = ("cpk", "fpc", "fpk")

    def test_pick_prefers_the_crate_checked_longest_ago(self):
        # `checked` holds commits, dated by `date`; `launched` holds the times
        # the kept runs ended.
        dates = {"cpk-checked": 300, "fpc-checked": 500}
        checked = {"cpk": "cpk-checked", "fpc": "fpc-checked"}
        launched = {"fpc": 100}
        date = dates.__getitem__
        # fpk has neither a checked commit nor a kept run, so it comes first.
        self.assertEqual(
            driver.pick(list(self.CRATES), checked, launched, date), "fpk"
        )
        # Its last check is then its kept run's: 200 < 300 < 500.
        launched = {**launched, "fpk": 200}
        self.assertEqual(
            driver.pick(list(self.CRATES), checked, launched, date), "fpk"
        )
        # A tie on the last check breaks on the name.
        same = {"cpk-checked": 400, "fpc-checked": 400}
        self.assertEqual(
            driver.pick(
                list(self.CRATES), checked, {"fpc": 400, "fpk": 400}, same.__getitem__
            ),
            "cpk",
        )

    def test_a_run_that_just_ended_goes_last_while_head_stands_still(self):
        """Every crate recorded at one commit, and cpk's run just ended,
        launched from that same commit: cpk goes to the back, not to the
        front again by its launch commit's old date and its name."""
        checked = dict.fromkeys(self.CRATES, "c" * 40)
        date = {"c" * 40: 100}.__getitem__
        self.assertEqual(
            driver.pick(list(self.CRATES), checked, {"cpk": 900}, date), "fpc"
        )
        # The kept run's time is when it ended: its file's.
        kept = mutants_diff.ROLLING / "cpk" / "launched_from.txt"
        kept.parent.mkdir(parents=True)
        kept.write_text("c" * 40 + "\n", encoding="utf-8")
        os.utime(kept, (900, 900))
        self.assertEqual(driver.launched_times(), {"cpk": 900})

    def test_queue_rolling_queues_the_crate_pick_names_with_its_extra_pass(self):
        """Each of the three readings decides: with the checked commits left
        out cpk would come first, with the kept runs' times left out fmdl, and
        fpc, never checked, is a local-only crate."""
        commits = {"cpk": "c" * 40, "fmdl": "d" * 40, "fpk": "e" * 40}
        dates = {"c" * 40: 700, "d" * 40: 300, "e" * 40: 650}
        for crate, commit in commits.items():
            mutants_diff.write_checked(crate, commit, mutants_diff.CHECKED)
        for crate, ended in {"cpk": 400, "fmdl": 800, "fpk": 650}.items():
            kept = mutants_diff.ROLLING / crate / "launched_from.txt"
            kept.parent.mkdir(parents=True)
            kept.write_text(commits[crate] + "\n", encoding="utf-8")
            os.utime(kept, (ended, ended))
        queued = []
        job = self.root / "a job"
        with (
            mock.patch.object(mutants, "remote_host", lambda: "bonfire"),
            mock.patch.object(
                driver.extra_mutants, "workspace_crates",
                lambda root: {name: root / name for name in ("cpk", "fmdl", "fpc", "fpk")},
            ),
            mock.patch.object(mutants, "LOCAL_ONLY_CRATES", frozenset({"fpc"})),
            mock.patch.object(driver, "commit_date", dates.__getitem__),
            mock.patch.object(
                driver, "queue", lambda *args, **kwargs: queued.append((args, kwargs)) or job
            ),
        ):
            self.assertEqual(driver.queue_rolling(), job)
        self.assertEqual(queued, [(("fpk",), {"extra": None, "rolling": True})])

    def test_queue_rolling_without_a_host_queues_nothing(self):
        with mock.patch.object(driver, "queue", lambda *a, **k: self.fail("a job was queued")):
            self.assertIsNone(driver.queue_rolling())
        self.assertIn("STUDIO_MUTANTS_REMOTE", self.out.getvalue())

    def test_a_second_driver_leaves_the_queue_alone(self):
        job = self.queue()
        # A claim a running driver would recover at its start.
        (job / "shard-0.todo").rename(job / "shard-0.pc")
        before = sorted(p.name for p in job.iterdir())
        with open(self.driver / "guard.lock", "a+b") as held:
            self.assertTrue(mutants.try_lock(held))
            try:
                self.assertEqual(driver.run(once=True), 1)
            finally:
                mutants.unlock(held)
        self.assertEqual(sorted(p.name for p in job.iterdir()), before)
        self.assertEqual(self.calls, [])

    def test_running_is_whether_another_handle_holds_the_guard(self):
        self.assertFalse(driver.running(), "no guard file")
        guard = self.driver / "guard.lock"
        self.driver.mkdir(parents=True)
        guard.touch()
        self.assertFalse(driver.running(), "a guard nobody holds")
        with open(guard, "a+b") as held:
            self.assertTrue(mutants.try_lock(held))
            try:
                self.assertTrue(driver.running())
            finally:
                mutants.unlock(held)
        self.assertFalse(driver.running(), "released by its holder")

    def diff_run_setup(self) -> Path:
        """`mutants_diff.diff_run` on this temporary driver: a driver runs,
        `RESULTS` and the tree lock are under the test's folder, the tree lock
        is the real one (other locks keep `self.run_lock`), each crate's
        estimate is one shard's and its extra pass lists no mutant. Returns
        the tree lock's path."""
        tree_lock = self.root / ".tmp" / "mutants.tree.lock"
        fake = mutants.run_lock

        def run_lock(path=None):
            if path == tree_lock:
                return REAL_RUN_LOCK(path)
            return fake() if path is None else fake(path)

        self.enter([
            mock.patch.object(driver, "running", lambda: True),
            mock.patch.object(mutants_diff, "RESULTS", self.root / "mutants.diff"),
            mock.patch.object(mutants, "tree_lock_path", lambda: tree_lock),
            mock.patch.object(mutants, "run_lock", run_lock),
            mock.patch.object(mutants, "estimate_seconds", lambda crates, share=1.0: 0.0),
            mock.patch.object(
                driver.extra_mutants, "listed",
                lambda crate, root, kinds: (Path(f"crates/libs/{crate}"), [], [], 0),
            ),
        ])
        return tree_lock

    def test_a_diff_run_queues_a_job_per_crate_and_files_their_results(self):
        """The two crates' jobs run on this PC's lane while the command waits;
        each lands in `mutants.diff/<crate>/`, their survivors crate by crate
        in `missed_all.txt`, and the code combines the jobs' (2 and 3)."""
        self.diff_run_setup()
        self.codes[("fpk", 0)] = 3
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            if len(sleeps) > 1:
                raise Stop("waited again after the jobs ran")
            driver.run(once=True)

        with mock.patch.object(mutants_diff.time, "sleep", sleep):
            code = mutants_diff.diff_run(TWO_CRATES_DIFF, HEAD_TREE, "a diff")
        self.assertEqual(code, 3)
        self.assertEqual(sleeps, [driver.POLL_SECONDS])
        self.assertEqual([option(args, "-p") for args in self.calls], ["cpk", "fpk"])
        results = mutants_diff.RESULTS
        for crate, crate_code in (("cpk", 2), ("fpk", 3)):
            self.assertEqual(
                json.loads((results / crate / "result.json").read_text(encoding="utf-8"))["code"],
                crate_code,
            )
        self.assertEqual(
            (results / "missed_all.txt").read_text(encoding="utf-8"),
            "missed in cpk shard 0\nmissed in fpk shard 0\n",
        )
        self.assertEqual(list((self.driver / "done").iterdir()), [])
        self.assertEqual(self.out.getvalue().count("waiting for the driver's jobs"), 1)

    def test_a_diff_run_with_no_driver_refuses_and_queues_nothing(self):
        self.diff_run_setup()
        results = mutants_diff.RESULTS
        results.mkdir()
        (results / "missed_all.txt").write_text("the last run's survivor\n", encoding="utf-8")
        with (
            mock.patch.object(driver, "running", lambda: False),
            mock.patch.object(driver, "queue", lambda *a, **k: self.fail("a job was queued")),
        ):
            self.assertEqual(mutants_diff.diff_run(TWO_CRATES_DIFF, HEAD_TREE, "a diff"), 1)
        self.assertIn("just driver run", self.out.getvalue())
        self.assertEqual(
            (results / "missed_all.txt").read_text(encoding="utf-8"), "the last run's survivor\n"
        )
        self.assertFalse(results.with_name("mutants.diff.old").exists())

    def test_a_diff_run_holds_no_tree_lock_while_it_waits(self):
        """This PC's driver takes the tree lock of the tree it runs from to
        file a rolling job: a diff run waiting with it held would stop it."""
        tree_lock = self.diff_run_setup()
        free = []

        def sleep(seconds):
            with open(tree_lock, "a+b") as handle:
                taken = mutants.try_lock(handle)
                if taken:
                    mutants.unlock(handle)
            free.append(taken)
            driver.run(once=True)

        with mock.patch.object(mutants_diff.time, "sleep", sleep):
            mutants_diff.diff_run(TWO_CRATES_DIFF, HEAD_TREE, "a diff")
        self.assertEqual(free, [True])

    def test_a_failed_baseline_leaves_the_next_shard_to_run_it(self):
        job = self.queue()
        self.codes[("cpk", 0)] = 4
        driver.run(once=True)
        self.assertNotIn("--baseline", self.calls[0])
        self.assertNotIn("--baseline", self.calls[1], "shard 0's baseline failed: shard 1 runs it")
        self.assertEqual(option(self.calls[2], "--timeout"), "200")
        self.assertEqual(self.result(job)["code"], 4)

    def test_a_shard_with_no_baseline_outcome_leaves_the_next_shard_to_run_it(self):
        self.queue(shards=4)
        self.unwritten.add(("cpk", 0))
        self.empty.add(("cpk", 1))
        driver.run(once=True)
        self.assertNotIn("--baseline", self.calls[1], "shard 0 wrote no outcomes")
        self.assertNotIn("--baseline", self.calls[2], "shard 1 recorded no baseline")
        self.assertEqual(option(self.calls[3], "--timeout"), "200")

    def test_main_queues_and_runs(self):
        self.assertEqual(driver.main(["driver.py", "queue", "cpk", "--shards", "2"]), 0)
        self.assertEqual(driver.main(["driver.py", "run", "--once"]), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            driver.main(["driver.py", "queue", "cpk", "--shards", "0"])
        self.assertEqual(list((self.driver / "queue").iterdir()), [])

    def assert_local_job(self, job: Path, shards: int) -> None:
        """`job` stayed this PC's: every `.todo` here, no `sent.json`."""
        self.assertEqual(
            sorted(p.name for p in job.iterdir()),
            ["job.json", *(f"shard-{k}.todo" for k in range(shards)), "source"],
        )

    def test_a_queued_job_is_sent_to_the_vps_with_its_markers_there_alone(self):
        self.fake_vps()
        job = self.queue(shards=2)
        remote = self.remote_queue() / job.name
        self.assertEqual([p.name for p in self.remote_queue().iterdir()], [job.name], "no .tmp")
        self.assertEqual(
            sorted(p.name for p in remote.iterdir()),
            ["job.json", "shard-0.todo", "shard-1.todo", "source"],
        )
        self.assertTrue((remote / "source" / "scripts" / "mutation" / "driver.py").is_file())
        self.assertEqual((remote / "job.json").read_bytes(), (job / "job.json").read_bytes())
        self.assertEqual(
            sorted(p.name for p in job.iterdir()), ["job.json", "sent.json", "source"]
        )
        self.assertEqual(
            json.loads((job / "sent.json").read_text(encoding="utf-8")), {"host": "bonfire"}
        )
        self.assertIn("sent to bonfire", self.out.getvalue())

    def test_a_job_stays_here_when_the_vps_s_cargo_mutants_differs(self):
        self.fake_vps(version="cargo-mutants 26.0.0")
        job = self.queue(shards=2)
        self.assert_local_job(job, 2)
        self.assertFalse((self.home / "studio-driver").exists())
        self.assertIn("cargo-mutants differs, cargo-mutants 27.1.0 here, cargo-mutants 26.0.0 "
                      "on bonfire", self.out.getvalue())

    def test_a_job_stays_here_when_the_vps_cannot_be_asked_its_version(self):
        self.fake_vps(fail="--version")
        job = self.queue(shards=2)
        self.assert_local_job(job, 2)
        self.assertFalse((self.home / "studio-driver").exists())
        self.assertIn("cannot ask bonfire for its cargo-mutants version", self.out.getvalue())

    def test_a_send_failing_midway_leaves_the_job_here_and_nothing_on_the_vps(self):
        self.fake_vps(fail="tar -xf")
        job = self.queue(shards=2)
        self.assert_local_job(job, 2)
        self.assertEqual(list(self.remote_queue().iterdir()), [], "the half-sent job is removed")
        self.assertIn("sending it to bonfire failed", self.out.getvalue())

    def test_a_local_only_crate_or_no_host_sends_nothing(self):
        def no_call(*args, **kwargs):
            self.fail("the VPS was asked")

        with (
            mock.patch.object(mutants, "ssh", no_call),
            mock.patch.object(mutants, "local_mutants_version", no_call),
        ):
            with (
                mock.patch.object(mutants, "remote_host", lambda: "bonfire"),
                mock.patch.object(mutants, "LOCAL_ONLY_CRATES", frozenset({"cpk"})),
            ):
                local_only = self.queue("cpk", 2)
            no_host = self.queue("fpk", 2)
        self.assert_local_job(local_only, 2)
        self.assert_local_job(no_host, 2)

    def test_a_job_queued_with_extra_sends_every_shard_s_marker_to_the_vps(self):
        self.fake_vps()
        job = self.queue(shards=2, extra=2)
        remote = self.remote_queue() / job.name
        self.assertEqual(
            sorted(p.name for p in remote.glob("shard-*.todo")),
            [f"shard-{k}.todo" for k in range(4)],
        )
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (4, 2))
        self.assertEqual(
            sorted(p.name for p in job.iterdir()), ["job.json", "sent.json", "source"]
        )
        self.assertIn("queued in 2 shards and 2 extra shards", self.out.getvalue())

    def test_a_job_not_sent_drops_its_extra_shards_and_says_the_pass_needs_the_vps(self):
        job = self.queue(shards=2, extra=2)
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (2, 0))
        self.assert_local_job(job, 2)
        self.assertIn("extra pass needs the VPS", self.out.getvalue())

    def test_a_job_whose_extra_pass_was_dropped_finishes_untested(self):
        """Every shard exits 0 with no survivor: the job dropped at `queue`
        ends 1, as untested mutants do, and the same job with none dropped 0."""
        dropped = self.queue("cpk", shards=1, extra=2)
        kept = self.queue("fpk", shards=1)
        self.assertEqual(driver.read_job(dropped)["extra_dropped"], 2)
        self.assertNotIn("extra_dropped", driver.read_job(kept))
        self.unwritten.update({("cpk", 0), ("fpk", 0)})
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(self.result(dropped)["code"], 1)
        self.assertEqual(self.result(kept)["code"], 0)
        self.assertIn("cpk done, exit 1: 0 missed or timed out, 0 untested, its extra pass "
                      "not run", self.out.getvalue())

    def test_extra_none_sizes_the_pass_from_the_extracted_source_s_listing(self):
        calls = []
        rows = [
            ["src/lib.rs", str(5 * i), str(5 * i + 4), "bool", "1", "false",
             f"flip {i} | removed: true"]
            for i in range(5)
        ]

        def listed(crate, root, kinds):
            calls.append((crate, root.name, kinds))
            return Path("crates/libs/cpk"), rows, [], 0

        estimates = []
        self.fake_vps()
        with (
            mock.patch.object(driver.extra_mutants, "listed", listed),
            mock.patch.object(
                mutants, "estimate_seconds",
                lambda crates, share=1.0: estimates.append(crates)
                or driver.SHARD_SECONDS * 2.5,
            ),
        ):
            job = self.queue(shards=2, extra=None)
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (5, 3))
        self.assertEqual(calls, [("cpk", "source", list(mutants.EXTRA_KINDS))])
        self.assertEqual(estimates, [["cpk"] * 5])
        remote = self.remote_queue() / job.name
        self.assertEqual(
            sorted(p.name for p in remote.glob("shard-*.todo")),
            [f"shard-{k}.todo" for k in range(5)],
        )

    def queue_listing(self, rows: list[list[str]], estimate: float = 0.0) -> Path:
        """cpk queued in 2 shards and sent, with `extra=None`: the extra pass
        lists `rows`, and its estimate is `estimate` seconds."""
        self.fake_vps()
        with (
            mock.patch.object(
                driver.extra_mutants, "listed",
                lambda crate, root, kinds: (Path("crates/libs/cpk"), rows, [], 0),
            ),
            mock.patch.object(mutants, "estimate_seconds", lambda crates, share=1.0: estimate),
        ):
            return self.queue(shards=2, extra=None)

    def test_a_crate_with_no_extra_mutant_gets_no_extra_shard(self):
        job = self.queue_listing([])
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (2, 0))
        self.assertEqual(sorted(p.name for p in (self.remote_queue() / job.name).glob("shard-*.todo")),
                         ["shard-0.todo", "shard-1.todo"])

    def test_a_crate_gets_no_more_extra_shards_than_extra_mutants(self):
        """An estimate past the split threshold asks for two shards; one
        mutant fills one."""
        self.assertEqual(driver.shard_count(mutants.SPLIT_THRESHOLD_SECONDS + 1), 2)
        row = ["src/lib.rs", "0", "4", "bool", "1", "false", "flip 0 | removed: true"]
        job = self.queue_listing([row], mutants.SPLIT_THRESHOLD_SECONDS + 1)
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (3, 1))
        self.assertEqual(
            sorted(p.name for p in (self.remote_queue() / job.name).glob("shard-*.todo")),
            ["shard-0.todo", "shard-1.todo", "shard-2.todo"],
        )

    def queue_diff_listing(
        self, listed: int, rows: list[list[str]], estimate: float = 0.0
    ) -> Path | None:
        """cpk queued as a sent diff job of `TWO_CRATES_DIFF` (which adds
        cpk's line 2), with `extra=None`: cargo-mutants lists `listed`
        mutants, the extra pass lists `rows` in `src/lib.rs`, which reads
        "a\\nbb\\nccc\\n", and each estimate is `estimate` seconds. Records
        each estimate's crate list in `self.estimates`."""
        self.fake_vps()
        self.estimates = []

        def extract_tree(tree, dest):
            self.extract_tree(tree, dest)
            (dest / "crates" / "libs" / "cpk" / "src").mkdir(parents=True)
            (dest / "crates" / "libs" / "cpk" / "src" / "lib.rs").write_bytes(b"a\nbb\nccc\n")

        with (
            mock.patch.object(mutants, "extract_tree", extract_tree),
            mock.patch.object(
                mutants, "list_mutants",
                lambda selection, config, source: [{"name": str(i)} for i in range(listed)],
            ),
            mock.patch.object(
                driver.extra_mutants, "listed",
                lambda crate, root, kinds: (Path("crates/libs/cpk"), rows, [], 0),
            ),
            mock.patch.object(
                mutants, "estimate_seconds",
                lambda crates, share=1.0: self.estimates.append(crates) or estimate,
            ),
        ):
            return driver.queue("cpk", extra=None, diff=TWO_CRATES_DIFF, tree=HEAD_TREE)

    # Extra mutants of cpk's `src/lib.rs`: two on line 2, which the diff
    # changes, one on line 3, which it does not.
    ON_LINE_2 = [["src/lib.rs", "2", "4", "bool", "2", "false", f"on line 2, {i}"]
                 for i in range(2)]
    ON_LINE_3 = ["src/lib.rs", "5", "8", "bool", "3", "false", "on line 3"]

    def test_a_job_gets_no_more_shards_than_listed_mutants(self):
        job = self.queue_diff_listing(1, [], estimate=driver.SHARD_SECONDS * 2.5)
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (1, 0))
        self.assertEqual(self.estimates, [["cpk"]], "an estimate worth three shards")

    def test_a_diff_job_s_extra_shards_count_the_mutants_on_its_lines_alone(self):
        job = self.queue_diff_listing(0, [*self.ON_LINE_2, self.ON_LINE_3])
        spec = driver.read_job(job)
        self.assertEqual((spec["shards"], spec["extra"]), (1, 1))
        self.assertTrue((job / "sent.json").exists())
        self.assertEqual(self.estimates, [[], ["cpk"] * 2], "line 3's mutant is not counted")
        self.assertIn("queued in 0 shards and 1 extra shards", self.out.getvalue())

    def test_a_diff_job_with_no_mutant_on_its_lines_is_not_queued(self):
        self.assertIsNone(self.queue_diff_listing(0, [self.ON_LINE_3]))
        self.assertEqual(list((self.driver / "queue").iterdir()), [], "no job and no .tmp")
        self.assertFalse((self.home / "studio-driver").exists(), "nothing sent")
        self.assertEqual(self.out.getvalue().count("cpk not queued: no mutant in the diff"), 1)

    def sent_job(
        self, shards: int = 2, stamp: str | None = None, extra: int | None = 0
    ) -> tuple[Path, Path]:
        """A job queued and sent to the fake VPS: its folder here and the VPS's."""
        self.fake_vps()
        job = self.queue(shards=shards, stamp=stamp, extra=extra)
        return job, self.remote_queue() / job.name

    def vps_shard(self, remote: Path, k: int, files: dict[str, str]) -> None:
        """Shard `k` of the VPS's job `remote` ended there: `files` (paths
        relative to its folder) written, its marker gone, its `.done` last."""
        shard = remote / f"shard-{k}"
        for name, text in files.items():
            (shard / name).parent.mkdir(parents=True, exist_ok=True)
            (shard / name).write_text(text, encoding="utf-8")
        for marker in ("todo", "vps"):
            (remote / f"shard-{k}.{marker}").unlink(missing_ok=True)
        (remote / f"shard-{k}.done").write_text("0", encoding="utf-8")

    def ended_shard(self, job: Path, k: int, missed: list[str], code: int, vps: bool) -> None:
        """Shard `k` of `job` ended with `missed` and `code`: fetched from the
        VPS when `vps`, else run here. A fetched shard has its `log` beside
        and here no `exit`, as a VPS shard killed before its end has: only the
        `log` tells it from this PC's."""
        out = job / f"shard-{k}" / "mutants.out"
        out.mkdir(parents=True)
        (out / "missed.txt").write_text("".join(f"{line}\n" for line in missed), encoding="utf-8")
        if vps:
            (job / f"shard-{k}" / "log").write_text("console\n", encoding="utf-8")
        (job / f"shard-{k}.done").write_text(str(code), encoding="utf-8")

    def ended_extra(self, job: Path, k: int, missed: list[str], code: int) -> None:
        """Extra shard `k` of `job` ended, as `ended_shard` ends a
        cargo-mutants one: its `extra_mutants.out` holds `missed_<crate>.txt`,
        its `.done` last."""
        crate = driver.read_job(job)["crate"]
        out = job / f"shard-{k}" / "extra_mutants.out"
        out.mkdir(parents=True)
        (out / f"missed_{crate}.txt").write_text(
            "".join(f"{line}\n" for line in missed), encoding="utf-8"
        )
        (job / f"shard-{k}.done").write_text(str(code), encoding="utf-8")

    def mark_extra(self, job: Path, extra: int) -> None:
        """Give `job`'s job.json `extra` extra shards (its last `extra` shard
        numbers): a local job's queue never writes it — the extra pass needs
        the VPS — so a test of one writes it by hand."""
        spec = driver.read_job(job)
        spec["extra"] = extra
        (job / "job.json").write_text(json.dumps(spec), encoding="utf-8")

    def test_a_sent_job_s_shards_run_on_both_lanes_and_finish_here_as_one_run(self):
        job, remote = self.sent_job(shards=2)
        self.codes[("cpk", 0)] = 3
        self.vps_fakes()
        vps_claims = iter([driver.claim])
        with (
            mock.patch.object(driver, "DRIVER", self.home / "studio-driver"),
            mock.patch.object(driver, "LANE", "vps"),
            # One shard, then none: shard 1 is left for this PC.
            mock.patch.object(driver, "claim", lambda: next(vps_claims, lambda: None)()),
        ):
            self.assertEqual(driver.run(once=True), 0)
        self.assertTrue((remote / "shard-0.done").exists())
        self.assertTrue((remote / "shard-1.todo").exists())
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        self.assertFalse(remote.exists(), "the VPS's copy goes with the finish")
        self.assertEqual(
            self.result(job),
            {
                "code": 3,
                "missed": ["missed in cpk shard 0", "missed in cpk shard 1"],
                "untested": [],
                "rechecked": [],
                "starved": [],
            },
        )
        fetched = self.done(job) / "shard-0"
        self.assertTrue((fetched / "mutants.out" / "outcomes.json").is_file())
        self.assertEqual((fetched / "exit").read_text(encoding="utf-8"), "3 12\n")
        self.assertEqual((self.done(job) / "shard-0.done").read_text(encoding="utf-8"), "3")

    def test_a_fetch_leaves_a_tested_caught_mutant_s_log_there_and_copies_the_rest(self):
        job, remote = self.sent_job()
        tested = [{"phase": "Test", "duration": 1.0, "process_status": {"Failure": 100}}]
        outcomes = {"outcomes": [
            {"scenario": {"Mutant": {"name": "caught"}}, "summary": "CaughtMutant",
             "log_path": "log/caught.log", "phase_results": tested},
            {"scenario": {"Mutant": {"name": "missed"}}, "summary": "MissedMutant",
             "log_path": "log/missed.log",
             "phase_results": [{"phase": "Test", "duration": 1.0, "process_status": "Success"}]},
        ]}
        self.vps_shard(remote, 0, {
            "mutants.out/outcomes.json": json.dumps(outcomes),
            "mutants.out/missed.txt": "missed\n",
            "mutants.out/log/caught.log": "caught\n",
            "mutants.out/log/missed.log": "missed\n",
            "log": "console\n",
            "exit": "2 12\n",
            "memory_peak": "123\n",
            "memory_events": "oom 0\noom_kill 0\n",
        })
        # A partial fetch from before: removed first.
        (job / "shard-0").mkdir()
        (job / "shard-0" / "stale").write_text("", encoding="utf-8")
        self.assertTrue(driver.fetch_shard(job, 0))
        shard = job / "shard-0"
        self.assertEqual(
            sorted(p.relative_to(shard).as_posix() for p in shard.rglob("*") if p.is_file()),
            ["exit", "log", "memory_events", "memory_peak", "mutants.out/log/missed.log",
             "mutants.out/missed.txt", "mutants.out/outcomes.json"],
        )
        self.assertEqual((shard / "mutants.out" / "outcomes.json").read_text(encoding="utf-8"),
                         json.dumps(outcomes))
        self.assertEqual((job / "shard-0.done").read_text(encoding="utf-8"), "2")
        self.assertTrue((remote / "shard-0" / "mutants.out" / "log" / "caught.log").exists())

    def test_an_extra_shard_s_fetch_copies_its_extra_mutants_out_log_and_exit(self):
        job, remote = self.sent_job(shards=1, extra=1)
        self.vps_shard(remote, 1, {
            "extra_mutants.out/missed_cpk.txt": "extra missed\n",
            "extra_mutants.out/run.json": "{}\n",
            "extra_mutants.out/outcomes_bool.tsv": "missed\trow\n",
            "log": "console\n",
            "exit": "2 9\n",
        })
        self.assertTrue(driver.fetch_shard(job, 1))
        shard = job / "shard-1"
        self.assertEqual(
            sorted(p.relative_to(shard).as_posix() for p in shard.rglob("*") if p.is_file()),
            ["exit", "extra_mutants.out/missed_cpk.txt",
             "extra_mutants.out/outcomes_bool.tsv", "extra_mutants.out/run.json", "log"],
        )
        self.assertEqual((job / "shard-1.done").read_text(encoding="utf-8"), "2")

    def test_a_vps_shard_with_no_exit_is_done_with_code_1_and_an_oom_kill_is_kept(self):
        job, remote = self.sent_job()
        self.vps_shard(remote, 0, {"log": "", "memory_events": "oom 3\noom_kill 2\n"})
        self.vps_shard(remote, 1, {"log": "", "exit": "0 5\n", "memory_events": "oom_kill 0\n"})
        self.assertTrue(driver.fetch_shard(job, 0))
        self.assertTrue(driver.fetch_shard(job, 1))
        self.assertEqual(
            [(job / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(2)], ["1", "0"]
        )
        warning = ("the kernel killed processes in the remote run (oom_kill 2), so its caught "
                   "counts may include mutants a kill ended")
        self.assertEqual((job / "shard-0" / "oom_kills.txt").read_text(encoding="utf-8"),
                         warning + "\n")
        self.assertFalse((job / "shard-1" / "oom_kills.txt").exists())
        out = self.out.getvalue()
        self.assertIn(warning, out)
        self.assertIn("shard 1 was killed before it ended: untested", out)

    def test_a_sent_job_s_claim_takes_the_vps_s_lowest_shard_and_skips_it_while_unreachable(self):
        job, remote = self.sent_job(shards=11, stamp="20261127T110000")
        for k in range(9):
            (remote / f"shard-{k}.todo").unlink()
        with mock.patch.object(mutants, "remote_host", lambda: None):
            local = self.queue("fpk", 2, stamp="20261127T120000")
        self.assertEqual(driver.claim(), (job, 9), "9 before 10")
        self.assertTrue((job / "shard-9.pc").exists())
        self.assertTrue((remote / "shard-9.pc").exists())
        driver.run_shard(job, 9)
        self.assertEqual(sorted(p.name for p in job.glob("shard-9*")), ["shard-9", "shard-9.done"])
        self.assertFalse((remote / "shard-9.pc").exists(), "the VPS's marker goes with the run")
        self.vps_down = True
        self.assertEqual(driver.claim(), (local, 0))
        self.assertEqual(driver.claim(), (local, 1))
        outage = "a call to bonfire failed"
        self.assertEqual(self.out.getvalue().count(outage), 1, "printed once over two turns")
        self.vps_down = False
        self.assertEqual(driver.claim(), (job, 10))
        self.assertTrue((remote / "shard-10.pc").exists())
        self.assertIsNone(driver.claim(), "none left anywhere")
        self.vps_down = True
        self.assertIsNone(driver.claim())
        self.assertEqual(self.out.getvalue().count(outage), 2, "a new outage is printed again")

    def test_this_pc_s_claim_script_takes_only_a_sent_job_s_cargo_mutants_shards(self):
        job, remote = self.sent_job(shards=2, extra=2)
        self.assertEqual(driver.claim(), (job, 0))
        self.assertEqual(driver.claim(), (job, 1))
        self.assertIsNone(driver.claim(), "the extra shards are the VPS lane's")
        self.assertEqual(
            sorted(p.name for p in remote.glob("shard-*")),
            ["shard-0.pc", "shard-1.pc", "shard-2.todo", "shard-3.todo"],
        )

    def test_this_pc_s_lane_passes_over_a_local_job_s_extra_shards(self):
        job = self.queue(shards=3)
        self.mark_extra(job, 1)
        self.assertEqual(driver.claim(), (job, 0))
        self.assertEqual(driver.claim(), (job, 1))
        self.assertIsNone(driver.claim())
        self.assertTrue((job / "shard-2.todo").exists())

    def test_the_vps_lane_claims_a_job_s_extra_shards_too(self):
        job, remote = self.sent_job(shards=2, extra=2)
        self.vps_fakes()
        claimed = []
        with (
            mock.patch.object(driver, "DRIVER", self.home / "studio-driver"),
            mock.patch.object(driver, "LANE", "vps"),
        ):
            while taken := driver.claim():
                claimed.append(taken)
        self.assertEqual(claimed, [(remote, k) for k in range(4)])

    def test_a_job_without_an_extra_key_claims_runs_and_finishes(self):
        """A job queued before `extra` existed has none: every shard is a
        cargo-mutants shard."""
        job = self.queue(shards=2)
        spec = driver.read_job(job)
        del spec["extra"]
        (job / "job.json").write_text(json.dumps(spec), encoding="utf-8")
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        self.assertTrue(self.done(job).exists())

    def test_sync_puts_back_a_lost_claim_and_marks_a_fetched_shard_without_a_second_fetch(self):
        job, remote = self.sent_job(shards=3)
        # A claim this PC lost.
        (remote / "shard-0.todo").rename(remote / "shard-0.pc")
        # Run here, and the VPS's marker not removed.
        (remote / "shard-1.todo").rename(remote / "shard-1.pc")
        (job / "shard-1.done").write_text("0", encoding="utf-8")
        # Fetched before a crash, and not yet marked fetched there.
        self.vps_shard(remote, 2, {"log": "", "exit": "0 5\n"})
        self.ended_shard(job, 2, [], 0, vps=True)
        before = sorted(p.name for p in remote.iterdir())
        self.vps_down = True
        driver.sync(job)
        self.assertEqual(sorted(p.name for p in remote.iterdir()), before)
        self.vps_down = False

        def fetch_shard(job, k):
            self.fail(f"shard {k} fetched again")

        with mock.patch.object(driver, "fetch_shard", fetch_shard):
            driver.sync(job)
        self.assertEqual(sorted(p.name for p in remote.glob("shard-*")),
                         ["shard-0.todo", "shard-2.done", "shard-2.fetched"])

    def test_a_sent_job_whose_last_shard_ends_on_the_vps_is_finished_by_the_waiting_lane(self):
        job, remote = self.sent_job(shards=1)
        (remote / "shard-0.todo").rename(remote / "shard-0.vps")
        sleeps = []

        def sleep(seconds):
            sleeps.append(seconds)
            if len(sleeps) == 1:
                self.vps_shard(remote, 0, {"log": "", "exit": "0 5\n"})
            else:
                raise Stop

        # The finish empties the queue, so the lane's next pass turns to the rolling job.
        turns = self.rolling_turns()
        with mock.patch.object(driver.time, "sleep", sleep), self.assertRaises(Stop):
            driver.run()
        self.assertEqual(len(sleeps), 2)
        self.assertEqual(len(turns), 1)
        self.assertTrue(self.done(job).exists(), "finished between the two waits")
        self.assertFalse(remote.exists())

    def test_recover_leaves_a_sent_job_s_claims_to_the_vps(self):
        job, _ = self.sent_job(shards=2)
        (job / "shard-0.pc").touch()
        (job / "shard-0" / "mutants.out").mkdir(parents=True)
        (job / "shard-1.pc").touch()
        (job / "shard-1.done").write_text("0", encoding="utf-8")
        driver.recover()
        self.assertEqual(sorted(p.name for p in job.iterdir()),
                         ["job.json", "sent.json", "shard-1.done", "source"])

    WINDOWS_ONLY = ("crates/studio_core/src/redact.rs:10:5: replace redact_any_spelling -> "
                    "String with String::new()")
    WINDOWS_ONLY_HERE = "crates/studio_core/src/pes_process.rs:20:9: replace probe -> bool with true"
    ANYWHERE = "crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0"

    def finish_with_recheck(
        self, vps_missed: list[str], on_windows: bool, caught: set[str],
        here_missed: tuple[str, ...] = (WINDOWS_ONLY_HERE,), vps_code: int = 2,
    ) -> tuple[Path, list[tuple]]:
        """A sent job finished after shard 0 ended on the VPS with
        `vps_missed` and `vps_code`, and shard 1 here with `here_missed`,
        `on_windows` or not, the patched recheck catching `caught`. Returns the job and the
        recheck's calls: (lines, tree, out, whether under the build slot)."""
        for line, windows in [(self.WINDOWS_ONLY, True), (self.WINDOWS_ONLY_HERE, True),
                              (self.ANYWHERE, False)]:
            self.assertEqual(
                any(p.search(line) for p in mutants.WINDOWS_ONLY_TESTS), windows, line
            )
        job, remote = self.sent_job(shards=2)
        self.ended_shard(job, 0, vps_missed, vps_code, vps=True)
        self.ended_shard(job, 1, list(here_missed), 2 if here_missed else 0, vps=False)
        calls = []

        def recheck(missed, tree, out=None):
            calls.append((missed, tree, out, self.locked))
            return caught

        with (
            mock.patch.object(mutants, "recheck_windows", recheck),
            mock.patch.object(driver, "ON_WINDOWS", on_windows),
        ):
            self.assertEqual(driver.finish(job), self.done(job))
        self.assertFalse(remote.exists())
        return job, calls

    def test_finish_rechecks_here_the_vps_shards_windows_only_survivors(self):
        job, calls = self.finish_with_recheck(
            [self.WINDOWS_ONLY, self.ANYWHERE], True, {self.WINDOWS_ONLY}
        )
        self.assertEqual(calls, [([self.WINDOWS_ONLY], HEAD_TREE, job / "recheck", True)])
        self.assertEqual(self.result(job)["missed"], [self.ANYWHERE, self.WINDOWS_ONLY_HERE])
        self.assertEqual(self.result(job)["rechecked"], [self.WINDOWS_ONLY])
        self.assertEqual(self.result(job)["code"], 2)

    def test_off_windows_the_vps_shards_windows_only_survivors_are_printed_not_rechecked(self):
        job, calls = self.finish_with_recheck([self.WINDOWS_ONLY, self.ANYWHERE], False, set())
        self.assertEqual(calls, [])
        self.assertEqual(self.result(job)["missed"],
                         [self.WINDOWS_ONLY, self.ANYWHERE, self.WINDOWS_ONLY_HERE])
        self.assertEqual(self.result(job)["rechecked"], [])
        self.assertIn(f"Windows-only tests, not rechecked:\n  {self.WINDOWS_ONLY}\n",
                      self.out.getvalue())

    def test_a_job_whose_every_survivor_the_recheck_caught_exits_clean(self):
        job, calls = self.finish_with_recheck(
            [self.WINDOWS_ONLY], True, {self.WINDOWS_ONLY}, here_missed=()
        )
        self.assertEqual(len(calls), 1)
        self.assertEqual(self.result(job)["missed"], [])
        self.assertEqual(self.result(job)["code"], 0, "cargo-mutants' 2 no longer holds")

    def test_a_recheck_that_caught_every_survivor_clears_only_a_2(self):
        """A shard that timed out (3) beside its survivor keeps the job's 3
        once the recheck caught the survivor: only cargo-mutants' 2 says
        mutants were missed."""
        job, _calls = self.finish_with_recheck(
            [self.WINDOWS_ONLY], True, {self.WINDOWS_ONLY}, here_missed=(), vps_code=3
        )
        self.assertEqual(self.result(job)["rechecked"], [self.WINDOWS_ONLY])
        self.assertEqual(self.result(job)["code"], 3)

    def test_a_code_2_with_no_survivor_and_nothing_rechecked_stays_2(self):
        """Only a recheck that caught every survivor clears cargo-mutants' 2:
        a shard ending 2 with empty survivor lists keeps it."""
        job = self.queue(shards=2)
        self.ended_shard(job, 0, [], 2, vps=False)
        self.ended_shard(job, 1, [], 0, vps=False)
        self.assertEqual(driver.finish(job), self.done(job))
        self.assertEqual(
            self.result(job),
            {"code": 2, "missed": [], "untested": [], "rechecked": [], "starved": []},
        )

    WINDOWS_EXTRA = "crates/libs/elevation/src/windows.rs:10: [bool] flip to false"

    def test_finish_lists_the_extra_survivors_after_the_cargo_mutants_ones(self):
        """The extra shards' `missed_<crate>.txt` files join the job's
        survivors after the cargo-mutants ones, in shard order; nothing else
        in an extra shard's folder is read (its decoy mutants.out-shaped
        files included)."""
        self.assertTrue(
            any(p.search(self.WINDOWS_EXTRA) for p in mutants.WINDOWS_ONLY_EXTRA)
        )
        # Three shards, the last an extra one.
        job = self.queue(shards=3)
        self.mark_extra(job, 1)
        self.ended_shard(job, 0, ["cargo missed 0"], 2, vps=False)
        self.ended_shard(job, 1, ["cargo missed 1"], 2, vps=False)
        self.ended_extra(job, 2, ["extra missed a"], 2)
        # A decoy: a reader confusing the extra shard for a cargo-mutants one
        # (`missed_lines`, `killed_builds` read `shard-<k>/mutants.out`) would
        # pick it up.
        (job / "shard-2" / "extra_mutants.out" / "missed.txt").write_text(
            "decoy\n", encoding="utf-8"
        )
        calls = []

        def recheck_extra(lines, tree):
            calls.append((lines, tree, self.locked))
            return set(), 0

        with mock.patch.object(mutants, "recheck_remote_extra", recheck_extra):
            self.assertEqual(driver.finish(job), self.done(job))
        self.assertEqual(
            calls,
            [(["extra missed a"], HEAD_TREE, True)],
            "the extra survivors are rechecked under the build slot",
        )
        self.assertEqual(
            self.result(job)["missed"],
            ["cargo missed 0", "cargo missed 1", "extra missed a"],
        )
        self.assertEqual(self.result(job)["untested"], [])

    def test_an_extra_survivor_alone_makes_the_job_s_code_2(self):
        # An extra shard exits 0 with survivors (the pass's 1 is for untested
        # mutants alone), so the survivor itself must make the code 2.
        job = self.queue(shards=2)
        self.mark_extra(job, 1)
        self.ended_shard(job, 0, [], 0, vps=False)
        self.ended_extra(job, 1, ["extra missed a"], 0)
        with mock.patch.object(mutants, "recheck_remote_extra", lambda lines, tree: (set(), 0)):
            driver.finish(job)
        self.assertEqual(self.result(job)["missed"], ["extra missed a"])
        self.assertEqual(self.result(job)["code"], 2)

    def test_finish_moves_a_windows_only_extra_survivor_the_recheck_caught(self):
        job = self.queue(shards=2)
        self.mark_extra(job, 1)
        self.ended_shard(job, 0, [], 0, vps=False)
        self.ended_extra(job, 1, [self.WINDOWS_EXTRA, "extra missed elsewhere"], 2)
        calls = []

        def recheck_extra(lines, tree):
            calls.append((lines, tree, self.locked))
            return {self.WINDOWS_EXTRA}, 0

        with mock.patch.object(mutants, "recheck_remote_extra", recheck_extra):
            self.assertEqual(driver.finish(job), self.done(job))
        self.assertEqual(calls[0][:2], ([self.WINDOWS_EXTRA, "extra missed elsewhere"],
                                        HEAD_TREE))
        self.assertTrue(calls[0][2], "rechecked under the build slot")
        self.assertEqual(self.result(job)["missed"], ["extra missed elsewhere"])
        self.assertEqual(self.result(job)["rechecked"], [self.WINDOWS_EXTRA])

    def test_an_untested_extra_recheck_mutant_makes_the_job_s_code_1(self):
        job = self.queue(shards=2)
        self.mark_extra(job, 1)
        self.ended_shard(job, 0, [], 0, vps=False)
        self.ended_extra(job, 1, ["extra missed"], 2)
        with mock.patch.object(
            mutants, "recheck_remote_extra", lambda lines, tree: (set(), 2)
        ):
            driver.finish(job)
        self.assertEqual(self.result(job)["code"], 1)
        self.assertEqual(self.result(job)["missed"], ["extra missed"])

    def test_a_rolling_job_s_finish_files_it_under_its_crate(self):
        """A rolling job ends in `mutants.rolling/<crate>/`, where the triage
        and `mutants-checked` read it: the crate's earlier one set aside as
        `<crate>.old`, and `launched_from.txt` holding the job's commit. A
        job queued by hand ends in `done/`
        (`test_a_job_s_first_shard_runs_the_baseline_and_the_job_ends_in_done`)."""
        filed = mutants_diff.ROLLING / "cpk"
        filed.mkdir(parents=True)
        (filed / "result.json").write_text("earlier\n", encoding="utf-8")
        (filed / "launched_from.txt").write_text("c" * 40 + "\n", encoding="utf-8")
        self.fake_vps()
        job = driver.queue("cpk", 2, 0, rolling=True)
        remote = self.remote_queue() / job.name
        self.ended_shard(job, 0, ["missed in cpk shard 0"], 2, vps=True)
        self.ended_shard(job, 1, [], 0, vps=False)
        moved_under = []
        move_results = mutants.move_results

        def recording(folder, target):
            moved_under.append(self.locked)
            move_results(folder, target)

        with mock.patch.object(mutants, "move_results", recording):
            self.assertEqual(driver.finish(job), filed)
        self.assertEqual(
            moved_under, [mutants.tree_lock_path()] * 2,
            "the earlier folder's move aside and the job's, both under the tree lock",
        )
        self.assertFalse(job.exists())
        self.assertEqual(list(self.driver.glob("done/*")), [])
        self.assertFalse(remote.exists(), "the VPS's copy goes with the finish")
        self.assertEqual(
            sorted(p.name for p in filed.iterdir()),
            ["job.json", "launched_from.txt", "result.json", "sent.json",
             "shard-0", "shard-0.done", "shard-1", "shard-1.done"],
        )
        self.assertEqual(
            json.loads((filed / "result.json").read_text(encoding="utf-8")),
            {
                "code": 2, "missed": ["missed in cpk shard 0"], "untested": [], "rechecked": [],
                "starved": [],
            },
        )
        self.assertEqual(
            (filed / "shard-0" / "mutants.out" / "missed.txt").read_text(encoding="utf-8"),
            "missed in cpk shard 0\n",
        )
        self.assertEqual(
            (filed / "launched_from.txt").read_text(encoding="utf-8"), HEAD_COMMIT + "\n"
        )
        old = mutants_diff.ROLLING / "cpk.old"
        self.assertEqual((old / "result.json").read_text(encoding="utf-8"), "earlier\n")
        self.assertEqual((old / "launched_from.txt").read_text(encoding="utf-8"), "c" * 40 + "\n")
        self.assertIn(str(filed), self.out.getvalue().splitlines()[-1])

    def test_the_vps_lane_runs_a_job_s_shards_and_finishes_nothing(self):
        job = self.queue(shards=2)
        self.codes[("cpk", 1)] = 2
        self.vps_lane()
        sized = []
        sized_config = mutants.sized_config

        def sizing(cpus, extra=None, *, source):
            sized.append(source)
            return sized_config(cpus, extra, source=source)

        with mock.patch.object(mutants, "sized_config", sizing):
            self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([unit[0] for unit in self.units], ["map.sh", "shard-0.sh", "shard-1.sh"])
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        self.assertNotIn("--baseline", self.calls[0])
        self.assertEqual(option(self.calls[1], "--baseline"), "skip")
        self.assertEqual(option(self.calls[1], "--timeout"), "200")
        self.assertTrue((job / "baseline-vps.json").exists())
        self.assertEqual(
            [(job / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(2)], ["0", "2"]
        )
        self.assertEqual(list(job.glob("shard-*.vps")), [])
        self.assertTrue(job.exists(), "the job stays queued: this PC finishes it")
        self.assertFalse((self.driver / "done").exists())
        self.assertEqual(sized, [job / "source"], "the job's config is written once")
        self.assertEqual(
            (job / "mutants-vps.toml").read_text(encoding="utf-8"),
            sized_config(os.cpu_count() or 2, source=job / "source"),
        )
        self.assertEqual(self.costed, [], "another machine's timings are not folded in")

    def test_the_vps_lane_runs_a_job_s_extra_shards_through_their_own_script(self):
        job, remote = self.sent_job(shards=2, extra=2)
        self.vps_fakes()
        with (
            mock.patch.object(driver, "DRIVER", self.home / "studio-driver"),
            mock.patch.object(driver, "LANE", "vps"),
        ):
            self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(
            [unit[0] for unit in self.units],
            ["map.sh", "shard-0.sh", "shard-1.sh", "shard-2.sh", "shard-3.sh"],
        )
        # cargo-mutants' `--shard` divides by the cargo-mutants count alone.
        self.assertEqual([shard_of(args) for args in self.calls], ["0/2", "1/2"])
        out = self.out.getvalue()
        self.assertIn("shard 1 of 2: cargo mutants", out)
        self.assertIn("extra shard 1 of 2", out)
        for k in range(4):
            self.assertEqual((remote / f"shard-{k}.done").read_text(encoding="utf-8"), "0")
        for k in (2, 3):
            shard = remote / f"shard-{k}"
            self.assertTrue((shard / "extra_mutants.out" / "missed_cpk.txt").is_file())
            self.assertFalse((shard / "mutants.out").exists())
        self.assertTrue(job.exists(), "this PC finishes the job")

    def test_a_vps_extra_shard_records_no_baseline(self):
        """The lane's baseline comes from a cargo-mutants shard's
        `mutants.out/outcomes.json`, which an extra shard never writes, so
        its run records none."""
        job, remote = self.sent_job(shards=1, extra=1)
        self.vps_fakes()
        with (
            mock.patch.object(driver, "DRIVER", self.home / "studio-driver"),
            mock.patch.object(driver, "LANE", "vps"),
        ):
            self.assertEqual(driver.claim(), (remote, 0))
            self.assertEqual(driver.claim(), (remote, 1))
            driver.run_shard(remote, 1)
        self.assertTrue((remote / "shard-1.done").exists())
        self.assertTrue((remote / "shard-1" / "extra_mutants.out" /
                         "missed_cpk.txt").is_file())
        self.assertFalse((remote / "baseline-vps.json").exists())

    def test_a_job_finished_on_the_vps_lane_s_start_is_left_for_this_pc(self):
        job = self.queue(shards=1)
        (job / "shard-0.todo").unlink()
        (job / "shard-0.done").write_text("0", encoding="utf-8")
        self.vps_lane()
        self.assertEqual(driver.run(once=True), 0)
        self.assertTrue(job.exists())

    def stand_in(self, job: Path) -> str:
        """The words a VPS shard script puts before `nice` when `job` has a selection."""
        source = job / "source"
        return (
            f"CARGO={shlex.quote(str(source / 'scripts' / 'mutation' / 'select_tests.py'))} "
            "UTW_REAL_CARGO=$(command -v cargo) "
            f"UTW_MUTANTS_SOURCE={shlex.quote(str(source))} "
            f"UTW_MUTANTS_SELECTION={shlex.quote(str(job / 'selection.json'))} "
        )

    def assert_full_suite(self, job: Path, reason: str) -> None:
        """`job`'s map ran once, before its two shards, and failed for
        `reason`, printed once: both shards run with no stand-in for cargo."""
        self.assertEqual([unit[0] for unit in self.units], ["map.sh", "shard-0.sh", "shard-1.sh"])
        for name, text, _ in self.units[1:]:
            self.assertNotIn("CARGO=", text, name)
            self.assertNotIn("UTW_", text, name)
        self.assertFalse((job / "selection.json").exists())
        out = self.out.getvalue()
        self.assertEqual(out.count("coverage map failed"), 1, out)
        self.assertIn(f"driver: {job.name} coverage map failed ({reason}): its shards run the "
                      f"full suite, log {job / 'map.log'}\n", out)
        self.assertEqual(
            [(job / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(2)], ["0", "0"]
        )

    def test_the_vps_lane_builds_a_job_s_map_once_and_its_shards_select_their_tests(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([unit[0] for unit in self.units], ["map.sh", "shard-0.sh", "shard-1.sh"])
        for name, text, _ in self.units[1:]:
            self.assertTrue(
                cargo_line(text).startswith(
                    f". ~/.cargo/env && {self.stand_in(job)}"
                    "nice -n 19 ionice -c3 cargo-mutants mutants "
                ),
                cargo_line(text),
            )
        script = (job / "map.sh").read_bytes()
        self.assertNotIn(b"\r", script)
        lines = script.decode("utf-8").splitlines()
        self.assertEqual(lines[1], f"exec > {shlex.quote(str(job / 'map.log'))} 2>&1")
        self.assertIn(f"export TMPDIR={shlex.quote(str(self.driver / 'tmp'))}", lines)
        self.assertIn(f"--data={mutants.REMOTE_TEST_MEMORY} --", script.decode("utf-8"))
        listing = shlex.split(cargo_line(script.decode("utf-8")))
        self.assertEqual(listing[:5], [".", "~/.cargo/env", "&&", "cargo-mutants", "mutants"])
        self.assertEqual(option(listing, "-p"), "cpk")
        self.assertNotIn("--shard", listing)
        self.assertEqual(option(listing, "--config"), str(job / "mutants-vps.toml"))
        self.assertEqual(option(listing, "--dir"), str(job / "source"))
        self.assertIn("--list", listing)
        self.assertIn("--json", listing)
        self.assertEqual(listing[-2:], [">", str(job / "mutants.json")])
        (mapped,) = [i for i, line in enumerate(lines) if "coverage_map.py" in line]
        self.assertEqual(
            shlex.split(lines[mapped]),
            ["nice", "-n", "19", "ionice", "-c3", "python3", "scripts/mutation/coverage_map.py",
             "cpk", str(job / "selection.json"), "--mutants", str(job / "mutants.json")],
        )
        self.assertGreater(lines.index("rm -rf target/llvm-cov-target"), mapped)
        self.assertEqual(lines[-1], f'echo "$code" > {shlex.quote(str(job / "map-exit"))}')

    def test_the_map_script_covers_the_extra_kinds_for_a_job_with_extra_shards(self):
        """`--extra` makes the map list the extra kinds' mutants too, so the
        job's extra shards get the selection like its cargo-mutants ones."""
        self.fake_vps()
        job = self.queue(shards=1, extra=1)
        script = driver.map_script(job, self.driver / "tmp").decode("utf-8")
        mapped = next(line for line in script.splitlines() if "coverage_map.py" in line)
        self.assertIn("--extra", shlex.split(mapped))
        plain = self.queue("fpk", shards=1)
        self.assertNotIn(
            "--extra",
            driver.map_script(plain, self.driver / "tmp").decode("utf-8"),
        )

    def run_map_script(
        self, crate: str, listing_code: int, map_code: int
    ) -> tuple[str, list[str]]:
        """Run the `map.sh` of a job of `crate` in bash, with cargo-mutants exiting
        `listing_code`, `coverage_map.py` `map_code`, and `nice` and `ionice`
        passing their command through. Returns its `map-exit` and the
        arguments `python3` was called with; checks the build is removed."""
        if BASH is None:
            self.skipTest("no bash to run the map script (Git Bash on Windows)")
        job = self.queue(crate, shards=1)
        home = self.root / f"map-home-{crate}"
        stubs = home / "bin"
        stubs.mkdir(parents=True)
        (home / ".cargo").mkdir()
        (home / ".cargo" / "env").write_bytes(f'export PATH="{stubs.as_posix()}:$PATH"\n'.encode())
        for name, body in [
            ("cargo-mutants", f"echo '[]'; exit {listing_code}"),
            ("python3", f'echo "$*" >> "$HOME/python3.calls"; exit {map_code}'),
            ("nice", 'shift 2; exec "$@"'),
            ("ionice", 'shift; exec "$@"'),
        ]:
            (stubs / name).write_bytes(f"#!/bin/bash\n{body}\n".encode())
            (stubs / name).chmod(0o755)
        (job / "source" / "target" / "llvm-cov-target").mkdir(parents=True)
        (job / "map.sh").write_bytes(driver.map_script(job, self.root / "map-tmp"))
        subprocess.run(
            [BASH, str(job / "map.sh")], cwd=job / "source",
            env={**os.environ, "HOME": home.as_posix()}, check=True,
        )
        self.assertFalse((job / "source" / "target" / "llvm-cov-target").exists())
        calls = home / "python3.calls"
        called = calls.read_text(encoding="utf-8").splitlines() if calls.exists() else []
        return (job / "map-exit").read_text(encoding="utf-8").strip(), called

    def test_the_map_script_s_exit_is_the_listing_s_when_it_failed_else_the_map_s(self):
        self.assertEqual(self.run_map_script("cpk", 0, 0)[0], "0")
        exit_code, called = self.run_map_script("fpk", 0, 1)
        self.assertEqual(exit_code, "1")
        self.assertEqual(len(called), 1)
        self.assertTrue(called[0].startswith("scripts/mutation/coverage_map.py fpk "), called)
        self.assertEqual(
            self.run_map_script("ftex", 3, 1), ("3", []), "no map after a failed listing"
        )

    def test_a_failed_coverage_map_leaves_its_shards_the_full_suite(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.map_outcome = "failed"
        self.assertEqual(driver.run(once=True), 0)
        self.assert_full_suite(job, "exit 1")
        self.assertEqual((job / "map-exit").read_text(encoding="utf-8"), "1\n")

    def test_a_killed_coverage_map_is_marked_killed_and_leaves_its_shards_the_full_suite(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.map_outcome = "killed"
        self.assertEqual(driver.run(once=True), 0)
        self.assert_full_suite(job, "killed")
        self.assertEqual((job / "map-exit").read_text(encoding="utf-8"), "killed")

    def test_a_map_unit_that_never_started_ends_the_driver_and_marks_nothing_done(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.map_outcome = "unstarted"
        with self.assertRaisesRegex(RuntimeError, "did not start .*map.sh"):
            driver.run(once=True)
        self.assertEqual([unit[0] for unit in self.units], ["map.sh"])
        self.assertEqual(list(job.glob("shard-*.done")), [])
        self.assertFalse((job / "map-exit").exists(), "the next start builds the map again")
        self.assertTrue((job / "shard-0.vps").exists(), "the next start puts it back")

    def test_the_vps_lane_s_recover_removes_a_half_built_map_and_keeps_a_finished_one(self):
        half = self.queue("cpk", shards=1)
        built = self.queue("fpk", shards=1)
        self.vps_lane()
        files = ["map.log", "mutants.json", "selection.json"]
        for job in (half, built):
            for name in files:
                (job / name).write_text("", encoding="utf-8")
        (built / "map-exit").write_text("0\n", encoding="utf-8")
        driver.recover()
        self.assertEqual([name for name in [*files, "map-exit"] if (half / name).exists()], [])
        self.assertEqual([name for name in [*files, "map-exit"] if (built / name).exists()],
                         [*files, "map-exit"])
        self.assertTrue((half / "shard-0.todo").exists())

    def test_this_pc_s_lane_runs_no_coverage_map_for_a_sent_job(self):
        job, _ = self.sent_job(shards=1)
        self.vps_fakes()
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(self.units, [], "no unit started here")
        self.assertEqual(
            self.calls,
            [["-p", "cpk", "--shard", "0/1", "--sharding", "round-robin",
              "--output", str(job / "shard-0")]],
        )
        self.assertEqual(list(self.done(job).glob("map*")), [])
        self.assertFalse((self.done(job) / "selection.json").exists())

    def assert_memory_copies(self, script: str, shard: Path) -> None:
        """`script` copies its unit cgroup's memory figures into `shard` under
        the names `fetch_shard` takes and a whole run keeps."""
        folder = shlex.quote(str(shard))
        self.assertIn(f'cp "$cgroup/memory.peak" {folder}/memory_peak\n', script)
        self.assertIn(f'cp "$cgroup/memory.events" {folder}/memory_events\n', script)

    def test_the_vps_lane_s_shard_script(self):
        job = self.queue(shards=1)
        self.vps_lane()
        driver.run(once=True)
        script = (job / "shard-0.sh").read_bytes()
        self.assertNotIn(b"\r", script)
        lines = script.decode("utf-8").splitlines()
        command = shlex.split(cargo_line(script.decode("utf-8")))
        self.assertEqual(option(command, "--output"), str(job / "shard-0"))
        self.assertEqual(option(command, "--config"), str(job / "mutants-vps.toml"))
        self.assertEqual(option(command, "--dir"), str(job / "source"))
        self.assertEqual(option(command, "--jobs"), str(mutants.JOBS))
        self.assertEqual(
            option(command, "--jobserver-tasks"),
            str(mutants.REMOTE_JOBSERVER_TASKS or os.cpu_count() or 2),
        )
        cargo = command.index("cargo-mutants")
        self.assertEqual(command[cargo - 5:cargo + 2],
                         ["nice", "-n", "19", "ionice", "-c3", "cargo-mutants", "mutants"])
        self.assertIn(f"export TMPDIR={shlex.quote(str(self.driver / 'tmp'))}", lines)
        self.assertIn(f"exec > {shlex.quote(str(job / 'shard-0' / 'log'))} 2>&1", lines)
        self.assertIn(f"--data={mutants.REMOTE_TEST_MEMORY} --", script.decode("utf-8"))
        self.assert_memory_copies(script.decode("utf-8"), job / "shard-0")
        self.assertEqual(
            lines[-1],
            f'echo "$code $(( $(date +%s) - start ))" > {shlex.quote(str(job / "shard-0" / "exit"))}',
        )
        # The stand-in is set when the script is written with a selection, and only then.
        args = ["-p", "cpk", "--shard", "0/1"]
        with_selection = driver.shard_script(job, 0, args, self.driver / "tmp").decode("utf-8")
        (job / "selection.json").unlink()
        without = driver.shard_script(job, 0, args, self.driver / "tmp").decode("utf-8")
        self.assertNotIn("CARGO=", without)
        self.assertNotIn("UTW_", without)
        self.assertTrue(cargo_line(without).startswith(
            ". ~/.cargo/env && nice -n 19 ionice -c3 cargo-mutants mutants -p cpk "
        ))
        self.assertEqual(
            with_selection,
            without.replace(". ~/.cargo/env && ", f". ~/.cargo/env && {self.stand_in(job)}"),
        )

    def test_the_vps_lane_s_extra_script(self):
        """An extra shard's script runs `--remote-extra` on the job's source
        with the shard's own (j, m) of the extra count: `rm` of the pass's
        output before it, its move into `shard-<k>/` after, `exit` last;
        `--selection` names `selection.json` only when the map left one."""
        job, _ = self.sent_job(shards=2, extra=2)
        shard = job / "shard-3"
        script = driver.extra_script(job, 3, self.driver / "tmp")
        self.assertNotIn(b"\r", script)
        lines = script.decode("utf-8").splitlines()
        self.assertEqual(lines[1], f"exec > {shlex.quote(str(shard / 'log'))} 2>&1")
        rm = lines.index("rm -rf extra_mutants.out")
        run = next(i for i, line in enumerate(lines) if "--remote-extra" in line)
        self.assertLess(rm, run)
        command = shlex.split(lines[run])
        self.assertEqual(option(command, "--shard"), "1/2")
        self.assertEqual(option(command, "--source"), str(job / "source"))
        self.assertIsNone(option(command, "--selection"))
        self.assertIn(f"export TMPDIR={shlex.quote(str(self.driver / 'tmp'))}", lines)
        self.assertIn(f"--data={mutants.REMOTE_TEST_MEMORY} --", script.decode("utf-8"))
        moved = next(
            i for i, line in enumerate(lines) if line.startswith("mv extra_mutants.out")
        )
        self.assertGreater(moved, run)
        self.assert_memory_copies(script.decode("utf-8"), shard)
        self.assertEqual(
            lines[-1],
            f'echo "$code $(( $(date +%s) - start ))" > '
            f'{shlex.quote(str(shard / "exit"))}',
        )
        (job / "selection.json").write_text('{"mutants": []}\n', encoding="utf-8")
        lines = driver.extra_script(job, 3, self.driver / "tmp").decode("utf-8").splitlines()
        command = shlex.split(next(
            line for line in lines if "--remote-extra" in line
        ))
        self.assertEqual(option(command, "--selection"), str(job / "selection.json"))

    def test_a_diff_job_s_diff_reaches_its_shards_its_map_and_its_extra_pass(self):
        """`--in-diff <job>/in.diff` on this PC's shard and in the map's
        listing, `--diff <job>/in.diff` on the extra pass, each for the diff
        job alone: the whole-crate job beside it gets neither."""
        diffed = driver.queue("cpk", 1, 0, diff=TWO_CRATES_DIFF, tree=HEAD_TREE)
        whole = self.queue("fpk", 1)
        tmp = self.driver / "tmp"
        for job, expected in ((diffed, str(diffed / "in.diff")), (whole, None)):
            listing = shlex.split(cargo_line(driver.map_script(job, tmp).decode("utf-8")))
            self.assertEqual(option(listing, "--in-diff"), expected, job.name)
            extra = shlex.split(next(
                line for line in driver.extra_script(job, 1, tmp).decode("utf-8").splitlines()
                if "--remote-extra" in line
            ))
            self.assertEqual(option(extra, "--diff"), expected, job.name)
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(
            {option(args, "-p"): option(args, "--in-diff") for args in self.calls},
            {"cpk": str(diffed / "in.diff"), "fpk": None},
        )

    def test_a_shard_with_no_exit_file_is_done_with_code_1(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.killed.add(0)
        driver.run(once=True)
        self.assertEqual(
            [(job / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(2)], ["1", "0"]
        )
        self.assertIn("shard 1 was killed before it ended: untested", self.out.getvalue())

    def test_a_shard_unit_that_never_started_ends_the_driver_and_marks_nothing_done(self):
        job = self.queue(shards=2)
        self.vps_lane()
        self.unstarted.add(0)
        with self.assertRaisesRegex(RuntimeError, "did not start"):
            driver.run(once=True)
        self.assertEqual(list(job.glob("shard-*.done")), [])
        self.assertTrue((job / "shard-0.vps").exists(), "the next start puts it back")

    def assert_waits(self, job: Path, sleep=None) -> None:
        """Run the lane until its second sleep, and check it claimed nothing."""
        sleeps = []

        def stop_at_second(seconds):
            sleeps.append(seconds)
            if sleep is not None:
                sleep(len(sleeps))
            if len(sleeps) == 2:
                raise Stop

        with mock.patch.object(driver.time, "sleep", stop_at_second), self.assertRaises(Stop):
            driver.run(once=True)
        self.assertEqual(self.units, [])
        self.assertEqual(sorted(p.name for p in job.glob("shard-*")), ["shard-0.todo"])

    def test_the_vps_lane_waits_under_its_hold(self):
        job = self.queue(shards=1)
        self.vps_lane()
        (self.driver / "hold-vps").write_text("", encoding="utf-8")
        self.assert_waits(job)
        self.assertEqual(self.out.getvalue().count("hold-vps"), 1)

    def test_the_vps_lane_waits_for_the_old_commands_units_and_says_each(self):
        job = self.queue(shards=1)
        self.vps_lane()
        self.active = {mutants.REMOTE_UNIT}

        def switch(count):
            if count == 1:
                self.active = {mutants.GATES_UNIT}

        self.assert_waits(job, switch)
        out = self.out.getvalue()
        self.assertEqual(out.count(f"{mutants.REMOTE_UNIT} is running"), 1)
        self.assertEqual(out.count(f"{mutants.GATES_UNIT} is running"), 1)

    def test_the_vps_lane_waits_while_its_disk_is_below_the_floor(self):
        job = self.queue(shards=1)
        self.vps_lane()
        self.free = mutants.REMOTE_FREE_FLOOR - 1
        self.assert_waits(job)
        self.assertEqual(self.out.getvalue().count("free on the disk"), 1)

    def test_the_vps_lane_claims_with_nothing_in_its_way(self):
        job = self.queue(shards=1)
        self.vps_lane()
        self.active = {"fluxer"}
        self.free = mutants.REMOTE_FREE_FLOOR
        self.assertEqual(driver.run(once=True), 0)
        self.assertEqual([unit[0] for unit in self.units], ["map.sh", "shard-0.sh"])
        self.assertTrue((job / "shard-0.done").exists())

    def test_this_pc_s_lane_asks_neither_systemd_nor_the_disk(self):
        self.queue(shards=1)
        self.vps_fakes()

        def asked(*args):
            self.fail("this PC's lane asked the VPS lane's question")

        with (
            mock.patch.object(driver, "unit_active", asked),
            mock.patch.object(driver.shutil, "disk_usage", asked),
        ):
            self.assertEqual(driver.run(once=True), 0)
        self.assertEqual(len(self.calls), 1)
        self.assertEqual(self.units, [])

    def test_the_vps_lane_s_recover_stops_a_running_shard_unit_first(self):
        job = self.queue(shards=2)
        self.vps_lane()
        (job / "shard-0.todo").rename(job / "shard-0.vps")
        (job / "shard-0" / "mutants.out").mkdir(parents=True)
        self.active = {driver.SHARD_UNIT}
        partial_at_stop = []
        sudo = self.sudo

        def recorded(*args):
            partial_at_stop.append((job / "shard-0").exists())
            return sudo(*args)

        with mock.patch.object(driver, "sudo", recorded):
            driver.recover()
        self.assertEqual(self.sudo_calls, [("systemctl", "stop", driver.SHARD_UNIT)])
        self.assertEqual(partial_at_stop, [True], "stopped before its results are removed")
        self.assertEqual(sorted(p.name for p in job.glob("shard-*")),
                         ["shard-0.todo", "shard-1.todo"])
        driver.recover()
        self.assertEqual(len(self.sudo_calls), 1, "no unit running: no stop")

    def test_the_vps_lane_empties_its_tmp_before_a_shard(self):
        self.queue(shards=1)
        self.vps_lane()
        (self.driver / "tmp" / "left" / "deep").mkdir(parents=True)
        (self.driver / "tmp" / "left" / "deep" / "file").write_text("", encoding="utf-8")
        (self.driver / "tmp" / "stray.txt").write_text("", encoding="utf-8")
        driver.run(once=True)
        self.assertEqual(
            [unit[2] for unit in self.units], [[], []], "the map's unit, then the shard's"
        )

    def test_run_unit_starts_the_shard_unit_capped_and_waits(self):
        self.sudo_calls = []
        self.active = set()
        script = self.root / "job" / "shard-3.sh"
        with mock.patch.object(driver, "sudo", self.sudo):
            driver.run_unit(script, 123)
        (args,) = self.sudo_calls
        self.assertEqual(args[:5], ("systemd-run", "--wait", "--collect", "--quiet",
                                    f"--unit={driver.SHARD_UNIT}"))
        for setting in ("MemoryMax=123", "MemorySwapMax=0", "OOMPolicy=continue",
                        "CPUWeight=idle"):
            self.assertEqual(args[args.index(setting) - 1], "-p")
        self.assertIn(f"--working-directory={self.root / 'job' / 'source'}", args)
        self.assertEqual(args[-2:], ("/bin/bash", str(script)))

    def test_memory_max_is_the_available_memory_less_the_reserve(self):
        meminfo = self.root / "meminfo"
        meminfo.write_text("MemTotal: 16777216 kB\nMemAvailable:    8388608 kB\n",
                           encoding="utf-8")
        self.assertEqual(driver.memory_max(meminfo), 8 * 2**30 - mutants.REMOTE_MEMORY_RESERVE)
        meminfo.write_text(f"MemAvailable: {mutants.REMOTE_MEMORY_RESERVE // 1024} kB\n",
                           encoding="utf-8")
        with self.assertRaisesRegex(RuntimeError, "no memory cap"):
            driver.memory_max(meminfo)

    def test_deploy_installs_the_lane_from_head_and_restarts_it(self):
        self.fake_vps()
        (self.home / "studio-driver" / "bin" / "old").mkdir(parents=True)
        driver.deploy("bonfire")
        installed = self.home / "studio-driver" / "bin"
        self.assertTrue((installed / "scripts" / "mutation" / "driver.py").is_file())
        self.assertFalse((installed / "old").exists(), "the old install is replaced")
        self.assertFalse((self.home / "studio-driver" / "bin.new").exists())
        self.assertEqual(
            (self.home / "sudo.log").read_text(encoding="utf-8").splitlines(),
            [
                "-n tee /etc/systemd/system/studio-driver.service",
                "-n systemctl daemon-reload",
                "-n systemctl enable studio-driver",
                "-n systemctl restart studio-driver",
            ],
        )
        user, home = mutants.ssh(
            "bonfire", 'id -un; echo "$HOME"', capture_output=True, text=True
        ).stdout.splitlines()
        self.assertEqual(
            (self.home / "tee.in").read_bytes(), driver.service_unit(user, home).encode("utf-8")
        )

    def test_the_service_unit(self):
        unit = driver.service_unit("debian", "/home/debian").splitlines()
        for line in (
            "User=debian",
            "ExecStart=/usr/bin/python3 -u "
            "/home/debian/studio-driver/bin/scripts/mutation/driver.py run --lane vps",
            "Restart=always",
            "RestartSec=60",
            "StandardOutput=append:/home/debian/studio-driver/driver.log",
            "StandardError=append:/home/debian/studio-driver/driver.log",
            "Nice=10",
            "CPUWeight=idle",
            "WantedBy=multi-user.target",
        ):
            self.assertIn(line, unit)

    def test_main_runs_the_vps_lane_on_its_folder(self):
        vps = self.root / "vps"
        with mock.patch.object(driver, "DRIVER", vps):
            job = self.queue(shards=1)
        self.vps_fakes()
        with mock.patch.object(driver, "VPS_DRIVER", vps):
            self.assertEqual(driver.main(["driver.py", "run", "--lane", "vps", "--once"]), 0)
        self.assertEqual([unit[0] for unit in self.units], ["map.sh", "shard-0.sh"])
        self.assertTrue((job / "shard-0.done").exists())
        self.assertTrue((job / "baseline-vps.json").exists())
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            driver.main(["driver.py", "run", "--lane", "wsl", "--once"])

    def test_the_vps_lane_s_hold_is_the_one_gates_remote_makes(self):
        """`mutants.VPS_HOLD` is the path an ssh command makes and deletes; the
        VPS lane, run from `VPS_DRIVER`, must look for that same file."""
        self.vps_fakes()
        looked = []
        with (
            mock.patch.object(driver, "DRIVER", driver.VPS_DRIVER),
            mock.patch.object(driver, "LANE", "vps"),
            mock.patch.object(Path, "exists", lambda path: looked.append(path) or True),
        ):
            self.assertIsNotNone(driver.blocked())
        self.assertEqual(looked, [Path.home() / mutants.VPS_HOLD.removeprefix("~/")])


if __name__ == "__main__":
    unittest.main()
