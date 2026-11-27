"""Tests of `driver.py`: the queue's job folders, the shard count, both
lanes taking a job's shards, resuming after a restart and waiting, a job's
sending to the VPS, this PC's side of a sent job, the VPS lane's coverage
map and its deploy.
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
    def run_lock(self):
        self.locked = True
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
            (out / "outcomes.json").write_text(
                json.dumps({"outcomes": outcomes}), encoding="utf-8"
            )
            (out / "missed.txt").write_text(f"missed in {crate} shard {k}\n", encoding="utf-8")
        return subprocess.CompletedProcess(args, self.codes.get((crate, k), 0))

    def queue(self, crate: str = "cpk", shards: int | None = 3, stamp: str | None = None) -> Path:
        if stamp is None:
            return driver.queue(crate, shards)
        moment = time.strptime(stamp, "%Y%m%dT%H%M%S")
        with mock.patch.object(driver.time, "gmtime", lambda: moment):
            return driver.queue(crate, shards)

    def done(self, job: Path) -> Path:
        return self.driver / "done" / job.name

    def result(self, job: Path) -> dict:
        return json.loads((self.done(job) / "result.json").read_text(encoding="utf-8"))

    def test_queue_writes_the_job_its_markers_and_its_source_and_no_temp(self):
        estimated = []

        def estimate(crates, share=1.0):
            estimated.append(crates)
            return mutants_diff.SHARD_SECONDS * 2.5

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

    def test_shard_count(self):
        threshold = mutants.SPLIT_THRESHOLD_SECONDS
        shard = mutants_diff.SHARD_SECONDS
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
            },
        )
        self.assertFalse((self.done(job) / "source").exists())
        self.assertEqual(
            [(self.done(job) / f"shard-{k}.done").read_text(encoding="utf-8") for k in range(3)],
            ["0", "2", "0"],
        )
        self.assertEqual(list(self.done(job).glob("shard-*.pc")), [])
        self.assertIn("cpk", self.out.getvalue().splitlines()[-1])

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

    def sent_job(self, shards: int = 2, stamp: str | None = None) -> tuple[Path, Path]:
        """A job queued and sent to the fake VPS: its folder here and the VPS's."""
        self.fake_vps()
        job = self.queue(shards=shards, stamp=stamp)
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

        with mock.patch.object(driver.time, "sleep", sleep), self.assertRaises(Stop):
            driver.run()
        self.assertEqual(len(sleeps), 2)
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
        here_missed: tuple[str, ...] = (WINDOWS_ONLY_HERE,),
    ) -> tuple[Path, list[tuple]]:
        """A sent job finished after shard 0 ended on the VPS with
        `vps_missed` and shard 1 here with `here_missed`, `on_windows` or
        not, the patched recheck catching `caught`. Returns the job and the
        recheck's calls: (lines, tree, out, whether under the build slot)."""
        for line, windows in [(self.WINDOWS_ONLY, True), (self.WINDOWS_ONLY_HERE, True),
                              (self.ANYWHERE, False)]:
            self.assertEqual(
                any(p.search(line) for p in mutants.WINDOWS_ONLY_TESTS), windows, line
            )
        job, remote = self.sent_job(shards=2)
        self.ended_shard(job, 0, vps_missed, 2, vps=True)
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


if __name__ == "__main__":
    unittest.main()
