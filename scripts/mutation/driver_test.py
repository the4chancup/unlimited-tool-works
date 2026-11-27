"""Tests of `driver.py`: the queue's job folders, the shard count, and this
PC's lane taking a job's shards, resuming after a restart and holding. Every
test runs on a temporary `driver.DRIVER` and cost table, with cargo-mutants,
the tree's extraction, the mutant listing, the cost fold and the build-slot
lock stubbed; only `git rev-parse` reaches the repository. Run by `just test`
(`python scripts/mutation/driver_test.py`)."""

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import time
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


class Stop(BaseException):
    """Ends a driver loop from its patched sleep: an `Exception` could be
    caught by the loop, and the test would hang."""


def shard_of(args: list[str]) -> str:
    return args[args.index("--shard") + 1]


def option(args: list[str], name: str) -> str | None:
    return args[args.index(name) + 1] if name in args else None


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
        self.out = io.StringIO()
        patches = [
            mock.patch.object(driver, "DRIVER", self.driver),
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
        ]
        for patch in patches:
            patch.__enter__()
            self.addCleanup(patch.__exit__, None, None, None)
        self.assertTrue(driver.DRIVER.is_relative_to(self.root))
        self.assertTrue(mutants.COST_CACHE.is_relative_to(self.root))

    def extract_tree(self, tree: str, dest: Path) -> None:
        self.extracted.append(tree)
        (dest / ".cargo").mkdir(parents=True)
        (dest / ".cargo" / "mutants.toml").write_text(
            "timeout_multiplier = 5.0\nminimum_test_timeout = 150\n", encoding="utf-8"
        )

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


if __name__ == "__main__":
    unittest.main()
