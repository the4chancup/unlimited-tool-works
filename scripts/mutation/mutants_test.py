"""Tests of `mutants.py`'s and `mutants_diff.py`'s readings of a finished run's files: which
mutants never ran, which builds were killed, when a split run's summary fails, and which list
holds a crate's survivors. Each builds a `mutants.out` in a
temporary directory the way cargo-mutants writes one; nothing reaches the host. Also which
source the local half, the listings and the config read (cargo itself stubbed), that a run
snapshots before it waits for the one in progress (`run_lock`), and that no run here starts
while this tree's own remote half with a local one is uncollected (`remote_pending`,
ssh stubbed). Run by `just test` (`python scripts/mutation/mutants_test.py`)."""
import contextlib
import io
import json
import os
import stat
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import tomllib
import unittest
from collections import Counter
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import extra_mutants  # noqa: E402
import mutants  # noqa: E402
import mutants_diff  # noqa: E402
import rm_mutant_copies  # noqa: E402

HEAD_TREE = subprocess.run(
    ["git", "rev-parse", "HEAD^{tree}"], cwd=SCRIPTS, capture_output=True, text=True, check=True
).stdout.strip()
# Git's empty tree: a diff from it holds every file of the tree it is diffed with.
EMPTY_TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"

# Where runs keep their results and this machine's cost table, pointed at a scratch folder for
# the whole module: a test that reached `mutants.diff/` unpatched set a running run's results
# aside. The scripts' `ROOT` stays the repository, where git runs and whose objects the
# snapshot tests extract; a test that writes under it patches it itself.
SCRATCH = tempfile.TemporaryDirectory()
REPOSITORY = mutants.ROOT
REAL_REMOTE_PENDING = mutants.remote_pending
ISOLATION = [
    mock.patch.object(mutants_diff, "RESULTS", Path(SCRATCH.name) / "mutants.diff"),
    mock.patch.object(extra_mutants, "OUT_DIR", Path(SCRATCH.name) / "extra_mutants.out"),
    mock.patch.object(mutants, "COST_CACHE", Path(SCRATCH.name) / "mutants-cost.json"),
    mock.patch.object(mutants, "LOCAL_TMPDIR", Path(SCRATCH.name) / "copies"),
    # A run's refusals ask the real host and read this tree's pending record otherwise; a
    # test that needs a host patches its own. The record follows a test's own `ROOT`, so a
    # launch a test makes leaves no record behind for the tests after it.
    mock.patch.object(mutants, "remote_host", lambda: None),
    mock.patch.object(
        mutants, "remote_pending",
        lambda: Path(SCRATCH.name) / "remote-pending" if mutants.ROOT == REPOSITORY
        else REAL_REMOTE_PENDING(),
    ),
]


def setUpModule():
    for patcher in ISOLATION:
        patcher.start()


def tearDownModule():
    for patcher in reversed(ISOLATION):
        patcher.stop()
    SCRATCH.cleanup()


class Isolation(unittest.TestCase):
    def test_no_test_reaches_the_real_results_or_cost_table(self):
        # The real copies' folder is under the system's temporary folder too.
        scratch = Path(SCRATCH.name)
        for path in (
            mutants_diff.RESULTS, extra_mutants.OUT_DIR, mutants.COST_CACHE,
            mutants.LOCAL_TMPDIR, mutants.remote_pending(),
        ):
            self.assertTrue(path.is_relative_to(scratch), path)
        self.assertIsNone(mutants.remote_host())


def mutant(name: str) -> dict:
    return {"Mutant": {"name": name}}


def write_run(
    out: Path, selected: int, outcomes: list[dict], logs: dict[str, str] | None = None
) -> None:
    """A `mutants.out` at `out`: a selection of `selected` mutants, the baseline and
    `outcomes`, and each `logs` entry as the log file of that name. A phase result an
    outcome leaves without its name and exit status gets them as cargo-mutants writes
    a test phase: nextest's 100 for a catch, success otherwise."""
    out.mkdir(parents=True, exist_ok=True)
    (out / "mutants.json").write_text(json.dumps([{}] * selected), encoding="utf-8")
    baseline = {"scenario": "Baseline", "summary": "Success", "phase_results": [{"duration": 1.0}]}
    written = []
    for outcome in [baseline, *outcomes]:
        status = {"Failure": 100} if outcome["summary"] == "CaughtMutant" else "Success"
        phases = [
            {"phase": "Test", "process_status": status, **phase}
            for phase in outcome.get("phase_results", [{"duration": 1.0}])
        ]
        written.append({**outcome, "phase_results": phases})
    (out / "outcomes.json").write_text(json.dumps({"outcomes": written}), encoding="utf-8")
    for name, text in (logs or {}).items():
        (out / name).parent.mkdir(parents=True, exist_ok=True)
        (out / name).write_text(text, encoding="utf-8")


class NeverRan(unittest.TestCase):
    def test_a_half_stopped_in_its_baseline_ran_none_of_its_selection(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "mutants.json").write_text(json.dumps([{}, {}, {}]), encoding="utf-8")
            self.assertEqual(mutants.never_ran(out), (0, 3))

    def test_a_half_stopped_midway_ran_part_of_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_run(out, 3, [{"scenario": mutant("a"), "summary": "CaughtMutant"}])
            self.assertEqual(mutants.never_ran(out), (1, 3))

    def test_a_complete_half_and_one_with_no_selection_report_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_run(out, 1, [{"scenario": mutant("a"), "summary": "CaughtMutant"}])
            self.assertIsNone(mutants.never_ran(out))
            (out / "mutants.json").unlink()
            self.assertIsNone(mutants.never_ran(out))


def caught_outcome(name: str, status: dict) -> dict:
    """Mutant `name`'s outcomes.json entry as cargo-mutants writes a catch, its test
    phase ended with `status`, its log at `log/<name>.log`."""
    test = {"phase": "Test", "duration": 1.0, "process_status": status}
    return {
        "scenario": mutant(name), "summary": "CaughtMutant",
        "log_path": f"log/{name}.log", "phase_results": [test],
    }


def pend(
    host: str = "bonfire", local_half: bool = True, extra: bool = False,
    extra_only: bool = False,
) -> Path:
    """A `remote_pending` launch record under the patched ROOT as `launch_remote`
    writes one (`extra_only` present only when the run is extras-only), returned."""
    pending = mutants.remote_pending()
    pending.parent.mkdir(parents=True, exist_ok=True)
    record = {"host": host, "local_half": local_half, "extra": extra}
    if extra_only:
        record["extra_only"] = True
    pending.write_text(json.dumps(record) + "\n", encoding="utf-8")
    return pending


@contextlib.contextmanager
def lock_probes(root: Path):
    """Patches `mutants.run_lock` to take `root/mutants.lock` and yields
    `probe`: `probe(tag)` appends (tag, whether that lock is held) to
    `probe.calls`, asserted after the run — a whole run's wait holds no lock."""
    calls = []

    def probe(tag):
        with open(root / "mutants.lock", "a+b") as handle:
            calls.append((tag, mutants.try_lock(handle)))

    probe.calls = calls
    real_lock = mutants.run_lock
    with mock.patch.object(
        mutants, "run_lock", lambda *a, **k: real_lock(root / "mutants.lock")
    ):
        yield probe


class KilledBuilds(unittest.TestCase):
    def test_every_spelling_of_a_killed_build_and_a_signal_ended_cargo_count(self):
        logs = {
            "log/oom": "error: could not compile (signal: 9, SIGKILL: kill)",
            "log/linker": "collect2: fatal error: ld terminated with signal 9 [Killed]",
            "log/llvm": "LLVM ERROR: out of memory",
            "log/windows": "memory allocation of 1048576 bytes failed",
            "log/console": "process didn't exit successfully (exit code: 0xc0000142, STATUS_DLL_INIT_FAILED)",
            # A Windows SDK update had removed the system libraries mid-run (2026-11-01).
            "log/link": "error: linking with `rust-lld` failed: exit code: 1\n  = note: rust-lld: "
                        "error: could not open 'kernel32.lib': no such file or directory",
            "log/mutant": "error[E0308]: mismatched types",
            # An ordinary failing test's log: the catch stands.
            "log/caught.log": "        FAIL [   0.289s] pes_strings "
                              "tests::an_absent_section_or_string_is_none\n"
                              "error: test run failed",
            # A test binary that lost its console ran nothing, whatever
            # nextest's 100 says: the mutant is untested, not caught.
            "log/lost_console.log": "        ABORT [   0.003s] pes_strings "
                                    "tests::an_absent_section_or_string_is_none\n"
                                    "           - with code 0xc0000142: The application "
                                    "failed to initialize properly.",
        }
        outcomes = [
            {"scenario": mutant(name), "summary": "Unviable", "log_path": f"log/{name}"}
            for name in ("oom", "linker", "llvm", "windows", "console", "link", "mutant")
        ]
        outcomes.append({"scenario": mutant("signal"), "summary": "Failure"})
        outcomes.append(caught_outcome("caught", {"Failure": 100}))
        # Nextest's exit when it cannot list the tests: no test ran, so nothing caught it.
        outcomes.append(caught_outcome("no_tests_ran", {"Failure": 104}))
        outcomes.append(caught_outcome("lost_console", {"Failure": 100}))
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_run(out, len(outcomes), outcomes, logs)
            self.assertEqual(
                mutants.killed_builds(out),
                ["oom", "linker", "llvm", "windows", "console", "link", "signal",
                 "no_tests_ran", "lost_console"],
            )


class Summarize(unittest.TestCase):
    def summarize(
        self,
        root: Path,
        remote_only: bool = False,
        local_half: bool = True,
        extra_run: Path | None = None,
    ) -> tuple[int, str]:
        # The maintainer's switch is set here, not read: the test holds in either mode.
        saved = mutants.ROOT, mutants.REMOTE_ONLY
        mutants.ROOT, mutants.REMOTE_ONLY = root, remote_only
        out = io.StringIO()
        try:
            with contextlib.redirect_stdout(out):
                code = mutants.summarize(
                    0, 0, root / "remote_console.txt", (None, None),
                    local_half=local_half, extra_run=extra_run,
                )
            return code, out.getvalue()
        finally:
            mutants.ROOT, mutants.REMOTE_ONLY = saved

    def test_a_split_run_whose_local_half_never_started_fails(self):
        caught = [{"scenario": mutant("a"), "summary": "CaughtMutant"}]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_run(root / "mutants.out" / "remote" / "mutants.out", 1, caught)
            self.assertEqual(self.summarize(root)[0], 1)
            # A remote-only run has no local half to start.
            self.assertEqual(self.summarize(root, remote_only=True)[0], 0)
            write_run(root / "mutants.out", 1, caught)
            self.assertEqual(self.summarize(root)[0], 0)

    def test_a_run_with_no_local_half_reads_only_the_remote_half(self):
        """A whole run has no local half: a stale local `mutants.out` beside the
        fetched one — a missed line, a killed build, no `mutants.json` — reaches
        neither the merged list nor the exit code."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            local = root / "mutants.out"
            (local / "log").mkdir(parents=True)
            (local / "missed.txt").write_text(
                "crates/libs/cpk/src/lib.rs:1:1: a stale local survivor\n", encoding="utf-8"
            )
            (local / "outcomes.json").write_text(
                json.dumps({"outcomes": [
                    {"scenario": mutant("killed"), "summary": "Unviable",
                     "log_path": "log/k"},
                ]}),
                encoding="utf-8",
            )
            (local / "log" / "k").write_text(
                "error: could not compile (signal: 9, SIGKILL: kill)", encoding="utf-8"
            )
            remote = local / "remote" / "mutants.out"
            write_run(remote, 1, [{"scenario": mutant("r"), "summary": "MissedMutant"}])
            (remote / "missed.txt").write_text(
                "crates/libs/cpk/src/read.rs:2:2: a remote survivor\n", encoding="utf-8"
            )
            code, out = self.summarize(root, local_half=False)
            missed_all = (local / "missed_all.txt").read_text(encoding="utf-8").splitlines()
        self.assertEqual(code, 2)
        self.assertEqual(missed_all, ["crates/libs/cpk/src/read.rs:2:2: a remote survivor"])
        self.assertNotIn("never started", out)
        self.assertNotIn("NOT tested", out)
        self.assertFalse(any(line.startswith("local") for line in out.splitlines()))


class SplitCoverage(unittest.TestCase):
    def summarize(self, root: Path) -> tuple[int, str]:
        saved = mutants.ROOT
        mutants.ROOT = root
        try:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = mutants.summarize(
                    0, 0, root / "remote_console.txt", (None, None),
                    local_half=True, extra_run=None,
                )
            return code, out.getvalue()
        finally:
            mutants.ROOT = saved

    def write_split_run(
        self, root: Path, local: list[str], remote: list[str], names: list[str]
    ) -> None:
        """A finished whole-crate split run under `root`: the split record
        listing `names`, and each half's `mutants.out` testing its names."""
        record = root / "extra_mutants.out" / "split.json"
        record.parent.mkdir(parents=True, exist_ok=True)
        record.write_text(
            json.dumps(
                {
                    "crate": "cpk",
                    "method": "files",
                    "target": 0.6,
                    "achieved": 0.6,
                    "local_files": ["crates/libs/cpk/src/lib.rs"],
                    "names": names,
                }
            ),
            encoding="utf-8",
        )
        for base, found in (
            (root / "mutants.out", local),
            (root / "mutants.out" / "remote" / "mutants.out", remote),
        ):
            write_run(
                base,
                len(found),
                [{"scenario": mutant(name), "summary": "CaughtMutant"} for name in found],
            )

    def test_the_merged_halves_naming_each_listed_mutant_once_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_split_run(
                root, ["m_local_a", "m_local_b"], ["m_remote_c"],
                ["m_local_a", "m_local_b", "m_remote_c"],
            )
            code, _ = self.summarize(root)
            self.assertEqual(code, 0)

    def test_a_listed_mutant_neither_half_tested_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_split_run(
                root, ["m_local_a", "m_local_b"], [],
                ["m_local_a", "m_local_b", "m_remote_c"],
            )
            code, out = self.summarize(root)
            self.assertEqual(code, 5)
            self.assertIn("never tested", out)
            self.assertIn("m_remote_c", out)

    def test_a_mutant_both_halves_tested_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_split_run(
                root, ["m_local_a", "m_local_b"], ["m_local_b"],
                ["m_local_a", "m_local_b"],
            )
            code, out = self.summarize(root)
            self.assertEqual(code, 5)
            self.assertIn("tested twice", out)
            self.assertIn("m_local_b", out)


class SizedConfig(unittest.TestCase):
    def test_the_snapshots_config_with_extra_keys_writes_toml_tomllib_reads_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp)
            (source / ".cargo").mkdir()
            # Keys the live tree's file does not hold: only the snapshot's file has them.
            (source / ".cargo" / "mutants.toml").write_text(
                'test_tool = "nextest"\n'
                'exclude_globs = ["crates/libs/cpk/src/snapshot_only.rs"]\n'
                "timeout_multiplier = 7.0\n",
                encoding="utf-8",
            )
            text = mutants.sized_config(
                16,
                extra={
                    "examine_globs": ["crates/libs/cpk/src/read.rs"],
                    "exclude_globs": ["crates/libs/cpk/src/write.rs"],
                },
                source=source,
            )
        parsed = tomllib.loads(text)
        self.assertEqual(parsed["examine_globs"], ["crates/libs/cpk/src/read.rs"])
        self.assertEqual(
            parsed["exclude_globs"],
            ["crates/libs/cpk/src/snapshot_only.rs", "crates/libs/cpk/src/write.rs"],
        )
        self.assertEqual(parsed["timeout_multiplier"], 7.0)
        self.assertIsInstance(parsed["timeout_multiplier"], float)
        # The remote's jobserver cap goes on cargo-mutants' own command line;
        # `additional_cargo_args` would reach `cargo nextest run`.
        self.assertNotIn("additional_cargo_args", parsed)
        self.assertEqual(parsed["additional_cargo_test_args"], ["--test-threads=8"])


class TestCommand(unittest.TestCase):
    """`mutants.test_command` is the command both runners test a mutant with:
    `sized_config` hands cargo-mutants the same nextest args the extra kinds'
    `run_tests` starts, so how mutants are tested lives in one place."""

    def test_the_nextest_command_runs_the_crate_with_the_sized_threads(self):
        config = {
            "test_tool": "nextest",
            "additional_cargo_args": ["--frozen"],
            "additional_cargo_test_args": ["--test-threads=7"],
        }
        self.assertEqual(
            mutants.test_command("cpk", config, 13),
            ["cargo", "nextest", "run", "-p", "cpk", "--frozen", "--test-threads=13"],
        )

    def test_another_test_tool_fails_naming_it(self):
        for config in ({"test_tool": "cargo"}, {}):
            with self.assertRaisesRegex(ValueError, "'cargo'"):
                mutants.test_command("cpk", config, 8)

    def test_a_key_only_cargo_mutants_would_honor_stops_both_runners(self):
        config = {"test_tool": "nextest", "features": ["gpu"]}
        with self.assertRaisesRegex(ValueError, "features"):
            mutants.test_command("cpk", config, 8)
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp)
            (source / ".cargo").mkdir()
            (source / ".cargo" / "mutants.toml").write_text(
                'test_tool = "nextest"\nfeatures = ["gpu"]\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "features"):
                mutants.sized_config(16, source=source)

    def test_the_repositorys_config_passes_the_check(self):
        text = (REPOSITORY / ".cargo" / "mutants.toml").read_text(encoding="utf-8")
        mutants.check_test_config(tomllib.loads(text))

    def started(self, call) -> tuple:
        """`call(cwd)` — a `run_tests` or `build` call — with `subprocess.Popen`
        mocked, `cwd` a temporary directory holding the frozen-nextest
        `.cargo/mutants.toml`. Returns (the Popen mock, the proc it made,
        `cwd`, `call`'s result)."""
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        cwd = Path(tmp.name)
        (cwd / ".cargo").mkdir()
        (cwd / ".cargo" / "mutants.toml").write_text(
            'test_tool = "nextest"\nadditional_cargo_args = ["--frozen"]\n',
            encoding="utf-8",
        )
        proc = mock.Mock()
        proc.communicate.return_value = ("out", None)
        proc.returncode = 0
        with mock.patch("subprocess.Popen", return_value=proc) as popen:
            result = call(cwd)
        return popen, proc, cwd, result

    def test_run_tests_starts_the_commands_list(self):
        """The extra kinds' `run_tests` builds `test_command`'s list from the run's
        copy of `.cargo/mutants.toml`, so the two runners cannot part without this
        failing."""
        popen, proc, cwd, (code, out) = self.started(
            lambda cwd: extra_mutants.run_tests("cpk", cwd, 1.0)
        )
        config = {"test_tool": "nextest", "additional_cargo_args": ["--frozen"]}
        self.assertEqual(
            popen.call_args.args[0],
            mutants.test_command(
                "cpk", config, max(1, (os.cpu_count() or 1) // mutants.JOBS)
            ),
        )
        # In the run's copy, where the mutant is applied, not the live tree.
        self.assertEqual(popen.call_args.kwargs["cwd"], cwd)
        proc.communicate.assert_called_once_with(timeout=1.0)
        self.assertEqual((code, out), (0, "out"))

    def test_build_starts_the_commands_list_with_no_run_and_no_timeout(self):
        """The extra kinds' `build` is `run_tests`' command plus `--no-run`, and
        waits without a timeout, as cargo-mutants does on its build phase."""
        popen, proc, cwd, (code, out) = self.started(
            lambda cwd: extra_mutants.build("cpk", cwd)
        )
        config = {"test_tool": "nextest", "additional_cargo_args": ["--frozen"]}
        self.assertEqual(
            popen.call_args.args[0],
            mutants.test_command(
                "cpk", config, max(1, (os.cpu_count() or 1) // mutants.JOBS)
            ) + ["--no-run"],
        )
        self.assertEqual(popen.call_args.kwargs["cwd"], cwd)
        proc.communicate.assert_called_once_with(timeout=None)
        self.assertEqual((code, out), (0, "out"))

    def test_build_and_run_tests_split_the_cpus_among_the_jobs(self):
        """`run_crate`'s `JOBS` jobs share the machine as cargo-mutants' do:
        each gets `cpus // JOBS` test threads, never under 1."""
        for cpus, threads in ((8, 4), (1, 1)):
            with mock.patch("os.cpu_count", return_value=cpus):
                popen, *_ = self.started(
                    lambda cwd: extra_mutants.run_tests("cpk", cwd, 1.0)
                )
                self.assertIn(
                    f"--test-threads={threads}", popen.call_args.args[0]
                )
                popen, *_ = self.started(lambda cwd: extra_mutants.build("cpk", cwd))
                self.assertIn(
                    f"--test-threads={threads}", popen.call_args.args[0]
                )

    def test_baseline_times_the_test_phase_alone(self):
        """`baseline_seconds` is the test phase's wall time: the build before
        it is timed apart, so a cold build does not stretch the mutants'
        timeouts."""
        now = [0.0]

        def build(crate, cwd):
            now[0] += 100.0
            return 0, ""

        def run_tests(crate, cwd, timeout):
            now[0] += 10.0
            return 0, ""

        clock = mock.Mock(monotonic=lambda: now[0])
        with mock.patch.multiple(
            extra_mutants, build=build, run_tests=run_tests, time=clock
        ):
            self.assertEqual(extra_mutants.baseline_seconds("cpk", Path(".")), 10.0)

    def test_baseline_a_failed_build_stops_it_before_any_test(self):
        def build(crate, cwd):
            return 101, "error: could not compile `cpk`"

        def run_tests(crate, cwd, timeout):
            raise AssertionError("the baseline tested a crate that did not build")

        with (
            mock.patch.multiple(extra_mutants, build=build, run_tests=run_tests),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaises(SystemExit),
        ):
            extra_mutants.baseline_seconds("cpk", Path("."))


class Partition(unittest.TestCase):
    def test_each_file_goes_to_the_side_further_below_its_share_largest_first(self):
        files = {"a.rs": 40, "b.rs": 30, "c.rs": 20, "d.rs": 10}
        self.assertEqual(mutants.partition(files, 0.6), {"a.rs", "c.rs"})

    def test_a_split_further_from_the_share_than_halves_falls_back(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertIsNone(mutants.partition({"a.rs": 90, "b.rs": 10}, 0.55))


class LocalShare(unittest.TestCase):
    def test_the_slower_half_gets_the_smaller_share(self):
        # The every-outcome means say the opposite (a half that drew more unviable mutants
        # reads cheaper): the share is the tested means' alone.
        cache = {
            "mutant": {"cpk": 15.0},
            "remote_mutant": {"cpk": 10.0},
            "tested": {"cpk": [10.0, 10]},
            "remote_tested": {"cpk": [15.0, 10]},
        }
        self.assertAlmostEqual(mutants.local_share("cpk", cache), 0.6)

    def test_a_half_measured_on_too_few_tested_mutants_gives_no_share(self):
        measured = {"cpk": [10.0, mutants.SHARE_MIN_TESTED]}
        few = {"cpk": [15.0, mutants.SHARE_MIN_TESTED - 1]}
        for local, remote in ((measured, {}), ({}, measured), (measured, few), (few, measured)):
            cache = {"tested": local, "remote_tested": remote}
            self.assertIsNone(mutants.local_share("cpk", cache), cache)
        self.assertIsNotNone(
            mutants.local_share("cpk", {"tested": measured, "remote_tested": measured})
        )


class CostCache(unittest.TestCase):
    def update(self, root: Path) -> dict | None:
        """The cache `update_cost_cache` over `root`'s `mutants.out` wrote, or None."""
        saved = mutants.ROOT, mutants.COST_CACHE
        mutants.ROOT, mutants.COST_CACHE = root, root / "cost.json"
        try:
            mutants.update_cost_cache()
            if not mutants.COST_CACHE.exists():
                return None
            return json.loads(mutants.COST_CACHE.read_text(encoding="utf-8"))
        finally:
            mutants.ROOT, mutants.COST_CACHE = saved

    def test_a_run_with_a_killed_build_leaves_the_measured_costs(self):
        def outcomes(summary: str) -> list[dict]:
            return [
                {
                    "scenario": {"Mutant": {"name": f"m{i}", "file": "crates/libs/cpk/src/lib.rs"}},
                    "summary": summary,
                    "log_path": "log/m",
                    "phase_results": [{"duration": 1.0}],
                }
                for i in range(5)
            ]

        with tempfile.TemporaryDirectory() as tmp:
            # `mutant_crate` resolves the crate by the nearest Cargo.toml.
            (Path(tmp) / "crates" / "libs" / "cpk").mkdir(parents=True)
            (Path(tmp) / "crates" / "libs" / "cpk" / "Cargo.toml").write_text("")
            out = Path(tmp) / "mutants.out"
            killed = "process didn't exit successfully (exit code: 0xc0000142)"
            write_run(out, 5, outcomes("Unviable"), {"log/m": killed})
            self.assertIsNone(self.update(Path(tmp)))
            write_run(out, 5, outcomes("CaughtMutant"), {"log/m": "test failed"})
            self.assertEqual(self.update(Path(tmp))["mutant"], {"cpk": 1.0})

    def test_the_tested_mean_leaves_out_unviable_mutants_and_weighs_history(self):
        def outcomes(seconds: dict[str, list[float]]) -> list[dict]:
            return [
                {
                    "scenario": {"Mutant": {"name": f"m{i}", "file": "crates/libs/cpk/src/lib.rs"}},
                    "summary": summary,
                    "log_path": "log/m",
                    "phase_results": [{"duration": value}],
                }
                for i, (summary, value) in enumerate(
                    (summary, value) for summary, values in seconds.items() for value in values
                )
            ]

        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "crates" / "libs" / "cpk").mkdir(parents=True)
            (Path(tmp) / "crates" / "libs" / "cpk" / "Cargo.toml").write_text("")
            out = Path(tmp) / "mutants.out"
            # Two tested at 100 s and 300 s, four unviable at 20 s, one timeout at 900 s:
            # the tested mean is 200 over two, where every outcome's is 197.
            run = {
                "CaughtMutant": [100.0], "MissedMutant": [300.0],
                "Unviable": [20.0] * 4, "Timeout": [900.0],
            }
            write_run(out, 7, outcomes(run), {"log/m": "test failed"})
            cache = self.update(Path(tmp))
            self.assertEqual(cache["tested"], {"cpk": [200.0, 2]})
            self.assertAlmostEqual(cache["mutant"]["cpk"], 1380.0 / 7)
            self.assertEqual(cache["remote_tested"], {})
            # A second run of two at 400 s: weighed two against two.
            write_run(out, 2, outcomes({"CaughtMutant": [400.0] * 2}), {"log/m": "test failed"})
            self.assertEqual(self.update(Path(tmp))["tested"], {"cpk": [300.0, 4]})
            # History counts for `TESTED_HISTORY` mutants at most: a mean standing on that
            # many, then a run as large, lands halfway and stays at the cap.
            cache_path = Path(tmp) / "cost.json"
            cache = json.loads(cache_path.read_text(encoding="utf-8"))
            cache["tested"]["cpk"] = [100.0, mutants.TESTED_HISTORY]
            cache_path.write_text(json.dumps(cache), encoding="utf-8")
            many = {"CaughtMutant": [300.0] * mutants.TESTED_HISTORY}
            write_run(out, mutants.TESTED_HISTORY, outcomes(many), {"log/m": "test failed"})
            self.assertEqual(
                self.update(Path(tmp))["tested"], {"cpk": [200.0, mutants.TESTED_HISTORY]}
            )

    def test_a_cache_without_remote_mutant_loads_an_empty_table(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache_path = Path(tmp) / "cost.json"
            cache_path.write_text(
                json.dumps({"mutant": {"cpk": 1.5}, "baseline": {"cpk": 60.0}}),
                encoding="utf-8",
            )
            saved = mutants.COST_CACHE
            mutants.COST_CACHE = cache_path
            try:
                cache = mutants.load_cost_cache()
            finally:
                mutants.COST_CACHE = saved
            self.assertEqual(cache["remote_mutant"], {})
            self.assertEqual(cache["tested"], {})
            self.assertEqual(cache["remote_tested"], {})
            self.assertEqual(cache["mutant"], {"cpk": 1.5})
            self.assertEqual(cache["baseline"], {"cpk": 60.0})

    def test_remote_outcomes_fold_into_remote_mutant_not_baseline(self):
        def outcomes() -> list[dict]:
            return [
                {
                    "scenario": {
                        "Mutant": {
                            "name": f"m{i}",
                            "file": "crates/libs/cpk/src/lib.rs",
                        }
                    },
                    "summary": "CaughtMutant",
                    "phase_results": [{"duration": 2.0}],
                }
                for i in range(5)
            ]

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # `mutant_crate` resolves the crate by the nearest Cargo.toml.
            (root / "crates" / "libs" / "cpk").mkdir(parents=True)
            (root / "crates" / "libs" / "cpk" / "Cargo.toml").write_text("")
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(remote, 5, outcomes())
            saved = mutants.ROOT, mutants.COST_CACHE
            mutants.ROOT, mutants.COST_CACHE = root, root / "cost.json"
            try:
                mutants.update_cost_cache(remote, "remote_mutant")
                cache = json.loads(mutants.COST_CACHE.read_text(encoding="utf-8"))
            finally:
                mutants.ROOT, mutants.COST_CACHE = saved
            self.assertEqual(cache["remote_mutant"], {"cpk": 2.0})
            self.assertEqual(cache["remote_tested"], {"cpk": [2.0, 5]})
            self.assertNotIn("cpk", cache["mutant"])
            self.assertNotIn("cpk", cache["tested"])
            self.assertNotIn("cpk", cache["baseline"])


class PlanFileSplit(unittest.TestCase):
    """The plan of a split: a diff's selection is listed as a whole crate's is, and the
    record says `files` only when a file split is planned."""

    DIFF = ["--in-diff", "in.diff", "-p", "cpk"]
    # Ten mutants over four files: 4, 3, 2 and 1.
    FILES = {"a.rs": 4, "b.rs": 3, "c.rs": 2, "d.rs": 1}

    def plan(self, tested: dict, remote_tested: dict, drop: str | None = None):
        """`plan_file_split(DIFF)` with the listing stubbed: the whole selection's with the
        plain config, a half's own files with its config (`drop`: a name the local half's
        listing loses). Returns the record, the selections listed and the printed text."""
        listed = []
        configs = {}

        def local_config(source, extra=None):
            path = Path(f"local{len(configs)}.toml")
            configs[str(path)] = extra
            return path

        def sized_config(nproc, extra, source):
            return json.dumps(extra)

        def list_mutants(selection, config, source):
            listed.append(list(selection))
            if str(config) in configs:
                extra = configs[str(config)]
                keep = None if extra is None else set(extra["examine_globs"])
            else:
                excluded = set(json.loads(Path(config).read_text(encoding="utf-8"))["exclude_globs"])
                keep = set(self.FILES) - excluded
            return [
                {"file": file, "name": f"{file}:{i}"}
                for file, count in self.FILES.items()
                for i in range(count)
                if (keep is None or file in keep) and not (keep and f"{file}:{i}" == drop)
            ]

        cache = {"tested": tested, "remote_tested": remote_tested}
        out = io.StringIO()
        with (
            mock.patch.multiple(
                mutants,
                local_config=local_config,
                sized_config=sized_config,
                list_mutants=list_mutants,
                load_cost_cache=lambda: cache,
            ),
            contextlib.redirect_stdout(out),
        ):
            record = mutants.plan_file_split(self.DIFF, 8, Path("source"))
        return record, listed, out.getvalue()

    def test_a_diffs_selection_is_listed_and_split_by_files_to_the_measured_share(self):
        # 150 s there against 100 s here: this machine takes 60%, files a.rs and c.rs.
        record, listed, out = self.plan({"cpk": [100.0, 20]}, {"cpk": [150.0, 20]})
        self.assertEqual(listed, [self.DIFF] * 3)
        self.assertEqual(record["crate"], "cpk")
        self.assertEqual(record["method"], "files")
        self.assertEqual(record["local_files"], ["a.rs", "c.rs"])
        self.assertAlmostEqual(record["target"], 0.6)
        self.assertAlmostEqual(record["achieved"], 0.6)
        self.assertEqual(len(record["names"]), 10)
        self.assertIn("file split: this machine tests 60% of cpk's 10 mutants", out)

    def test_an_unmeasured_half_or_a_share_near_halves_stays_round_robin(self):
        record, listed, out = self.plan({"cpk": [100.0, 20]}, {})
        self.assertEqual((record["method"], record["local_files"]), ("round-robin", []))
        self.assertEqual(listed, [self.DIFF], "only the whole listing, for the record's names")
        self.assertEqual(len(record["names"]), 10)
        self.assertIn("no per-mutant cost measured on both halves", out)
        # 100 s against 115 s is 53%: inside SHARE_MIN_GAIN of halves.
        record, listed, out = self.plan({"cpk": [100.0, 20]}, {"cpk": [115.0, 20]})
        self.assertEqual((record["method"], record["local_files"]), ("round-robin", []))
        self.assertEqual(listed, [self.DIFF])
        self.assertIn("near enough to halves", out)
        # At the line itself the split is by files.
        edge = 100.0 * (0.5 + mutants.SHARE_MIN_GAIN) / (0.5 - mutants.SHARE_MIN_GAIN)
        record, _, _ = self.plan({"cpk": [100.0, 20]}, {"cpk": [edge + 1.0, 20]})
        self.assertEqual(record["method"], "files")

    def test_halves_that_do_not_recombine_to_the_listing_stay_round_robin(self):
        record, _, out = self.plan({"cpk": [100.0, 20]}, {"cpk": [150.0, 20]}, drop="a.rs:0")
        self.assertEqual((record["method"], record["local_files"]), ("round-robin", []))
        self.assertIn("a file split is inexact (1 missing, first: a.rs:0)", out)


class RemoteSelection(unittest.TestCase):
    def test_a_per_crate_diff_run_keeps_its_crate_on_the_remote(self):
        self.assertEqual(
            mutants.remote_selection(["--in-diff", "C:/tmp/x.diff", "-p", "music_export"]),
            '--in-diff "$HOME/studio-mutants/run/in.diff" -p music_export',
        )
        self.assertEqual(mutants.remote_selection(["-p", "cpk"]), "-p cpk")


class LocalSource(unittest.TestCase):
    def test_the_local_half_runs_on_its_snapshot_not_the_working_tree(self):
        def git_out(args: list[str], text: str) -> str:
            # Bytes: text mode on Windows writes `\n` as CRLF, and `mktree` would read
            # the carriage return into the file's name.
            return mutants.git(
                *args, input=text.encode(), check=True, capture_output=True
            ).stdout.decode().strip()

        # A tree no working tree holds: only an extracted copy of it has this file.
        blob = git_out(["hash-object", "-w", "--stdin"], "the snapshot's line\n")
        tree = git_out(["mktree"], f"100644 blob {blob}\tsnapshot_only.rs\n")
        calls = []
        real_run = subprocess.run

        def run(args, **kwargs):
            if args[0] != "cargo":
                return real_run(args, **kwargs)
            source = Path(args[args.index("--dir") + 1])
            calls.append({
                "source": source,
                "cwd": kwargs["cwd"],
                "file": (source / "snapshot_only.rs").read_text(encoding="utf-8"),
                "outputs": [args[i + 1] for i, arg in enumerate(args) if arg == "--output"],
            })
            return subprocess.CompletedProcess(args, 0)

        with (
            tempfile.TemporaryDirectory() as tmp,
            mock.patch.object(mutants, "LOCAL_TMPDIR", Path(tmp)),
            mock.patch.object(mutants.subprocess, "run", run),
        ):
            config = Path(tmp) / "mutants.toml"
            with mutants.snapshot_source(tree) as source:
                mutants.local_mutants(["-p", "cpk"], source, config=config)
                mutants.local_mutants(
                    ["-p", "cpk", "--output", "elsewhere"], source, config=config
                )
        first, second = calls
        self.assertEqual(first["file"], "the snapshot's line\n")
        self.assertEqual(first["cwd"], first["source"], "cargo starts in the copy")
        self.assertEqual(first["source"].parent, Path(tmp), "the copy is under LOCAL_TMPDIR")
        self.assertEqual(first["outputs"], [str(mutants.ROOT)], "results in the root's mutants.out")
        self.assertEqual(second["outputs"], ["elsewhere"], "a caller's --output kept, alone")
        self.assertFalse(first["source"].exists(), "the copy removed once the run ends")

    def test_two_copies_open_at_once_are_two_folders(self):
        """The Windows recheck opens a copy of the remote half's tree inside the run's own: one
        name per process would extract both into one folder and delete it under the run."""
        with (
            tempfile.TemporaryDirectory() as tmp,
            mock.patch.object(mutants, "LOCAL_TMPDIR", Path(tmp)),
        ):
            with mutants.snapshot_source(EMPTY_TREE) as outer:
                with mutants.snapshot_source(EMPTY_TREE) as inner:
                    self.assertNotEqual(inner, outer)
                self.assertFalse(inner.exists())
                self.assertTrue(outer.exists(), "the inner copy's removal took the run's")
            self.assertFalse(outer.exists())

    def test_a_copy_that_cannot_be_removed_warns_and_the_run_keeps_its_result(self):
        def stuck(path):
            raise PermissionError(13, "in use by another process", str(path))

        def run() -> int:
            with mutants.snapshot_source(EMPTY_TREE):
                return 2

        err = io.StringIO()
        with (
            tempfile.TemporaryDirectory() as tmp,
            mock.patch.object(mutants, "LOCAL_TMPDIR", Path(tmp)),
            mock.patch.object(mutants, "remove_tree", stuck),
            contextlib.redirect_stderr(err),
        ):
            self.assertEqual(run(), 2)
        self.assertIn("could not remove", err.getvalue())

    def test_the_listings_and_the_extra_pass_read_the_snapshot(self):
        """A queued run reaches its turn long after its snapshot: its listings and the extra
        pass's crate folder come from the snapshot's copy, not from the working tree."""
        cwds = {}
        real_run = subprocess.run

        def run(args, **kwargs):
            if args[:2] == ["cargo", "mutants"]:
                cwds["diff listing" if "--in-diff" in args else "crate listing"] = kwargs["cwd"]
                return subprocess.CompletedProcess(args, 0, stdout="" if kwargs.get("text") else b"")
            if args[:2] == ["cargo", "metadata"]:
                cwds["metadata"] = kwargs["cwd"]
                manifest = Path(kwargs["cwd"]) / "crates" / "libs" / "fpc" / "Cargo.toml"
                packages = [{"name": "fpc", "manifest_path": str(manifest)}]
                return subprocess.CompletedProcess(args, 0, stdout=json.dumps({"packages": packages}))
            return real_run(args, **kwargs)

        with (
            tempfile.TemporaryDirectory() as tmp,
            mock.patch.object(subprocess, "run", run),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            source = Path(tmp)
            self.assertEqual(mutants.list_mutants("fpc", source / "mutants.toml", source), [])
            self.assertEqual(mutants_diff.run(str(source / "in.diff"), HEAD_TREE, source), 0)
            self.assertEqual(
                extra_mutants.crate_dir("fpc", source),
                (source / "crates" / "libs" / "fpc").resolve(),
            )
        self.assertEqual(
            cwds, {"crate listing": source, "diff listing": source, "metadata": source}
        )


class PendingRemote(unittest.TestCase):
    """A remote half launched from this tree, not yet fetched, and recorded with a
    local half refuses every run on this machine alone, whatever
    STUDIO_MUTANTS_REMOTE says now: its collection summarizes the local
    `mutants.out` that a new run would set aside. A half with none, and one
    launched from another tree, refuse nothing here."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(mutants, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_a_launch_records_the_half_before_its_unit_starts(self):
        recorded_at_start = []

        def ssh(host, remote, **kwargs):
            if "systemd-run" in remote:
                recorded_at_start.append(mutants.remote_pending().exists())
            return subprocess.CompletedProcess([host, remote], 0)

        with (
            mock.patch.object(mutants, "ssh", ssh),
            mock.patch.object(mutants, "remote_memory_max", lambda host: 2**30),
        ):
            mutants.launch_remote("bonfire", ["-p", "fpc"], nproc=8)
        self.assertEqual(recorded_at_start, [True])
        self.assertEqual(
            json.loads(mutants.remote_pending().read_text(encoding="utf-8")),
            {"host": "bonfire", "local_half": True, "extra": False},
        )

    def test_an_extras_only_launch_marks_its_record(self):
        """`collect` reads `extra_only` to take the extras-only finish; a run
        that is not extras-only leaves the key off the record."""

        def ssh(host, remote, **kwargs):
            return subprocess.CompletedProcess([host, remote], 0)

        with (
            mock.patch.object(mutants, "ssh", ssh),
            mock.patch.object(mutants, "remote_memory_max", lambda host: 2**30),
        ):
            mutants.launch_remote(
                "bonfire", ["-p", "cpk"], whole=True, nproc=8,
                local_half=False, extra=False, extra_only=True, kinds=["call"],
            )
            self.assertEqual(
                json.loads(mutants.remote_pending().read_text(encoding="utf-8")),
                {"host": "bonfire", "local_half": False, "extra": True,
                 "extra_only": True},
            )
            mutants.remote_pending().unlink()
            mutants.launch_remote(
                "bonfire", ["-p", "cpk"], whole=True, nproc=8,
                local_half=False, extra=True,
            )
            self.assertNotIn(
                "extra_only",
                json.loads(mutants.remote_pending().read_text(encoding="utf-8")),
            )

    def test_the_fetch_clears_the_record(self):
        pending = pend()

        def ssh(host, remote, **kwargs):
            if "tar -cf" in remote:
                return subprocess.CompletedProcess([host, remote], 3, stdout=b"")
            return subprocess.CompletedProcess(
                [host, remote], 0, stdout="\n\n" if kwargs.get("text") else b""
            )

        with mock.patch.object(mutants, "ssh", ssh), contextlib.redirect_stderr(io.StringIO()):
            mutants.fetch_remote("bonfire", partial=True)
        self.assertFalse(pending.exists())

    def test_a_run_here_refuses_and_keeps_the_last_results_with_no_host_set(self):
        pend()
        last = self.root / "mutants.out"
        last.mkdir()
        with (
            mock.patch.object(mutants, "remote_host", lambda: None),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            # A whole run refuses for the VPS rule before the pending record matters.
            with self.assertRaisesRegex(mutants.Refused, "VPS"):
                mutants.whole_run("fpc", None, HEAD_TREE)
            with self.assertRaisesRegex(mutants.Refused, "bonfire.*mutants-collect"):
                mutants_diff.run_crate("in.diff", HEAD_TREE, self.root, "fpc", 3)
        # A standalone extra pass would replace the `missed_<crate>.txt` the collection merges.
        extra_runs = []
        with (
            mock.patch.object(mutants, "remote_host", lambda: None),
            mock.patch.object(mutants, "working_tree", lambda: HEAD_TREE),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(extra_mutants, "run_crate", lambda *args: extra_runs.append(args)),
        ):
            with self.assertRaisesRegex(SystemExit, "bonfire.*mutants-collect"):
                extra_mutants.main(["extra_mutants.py", "fpc"])
        self.assertEqual(extra_runs, [])
        self.assertTrue(last.exists(), "the uncollected half's local results stay")

    def test_with_no_record_the_host_is_not_asked(self):
        """Only this tree's own uncollected local half stops a run here: a half
        launched from another tree is collected there, so a busy host alone is
        no reason to refuse."""
        with (
            mock.patch.object(
                mutants, "remote_run_state", lambda host: self.fail("the host was asked")
            ),
            mock.patch.object(
                mutants, "remote_refusal", lambda host: self.fail("the host was asked")
            ),
        ):
            self.assertIsNone(mutants.local_refusal())

    def test_a_record_with_no_local_half_lets_a_run_here_start(self):
        """A whole run's remote half reads nothing of this tree's `mutants.out`:
        its collection never merges the local results a new run would replace."""
        pend(local_half=False)
        self.assertIsNone(mutants.local_refusal())

    def test_a_record_with_a_local_half_refuses_as_today(self):
        pend(local_half=True)
        refusal = mutants.local_refusal()
        self.assertIn("bonfire", refusal)
        self.assertIn("mutants-collect", refusal)

    def test_a_record_this_version_cannot_read_refuses_as_a_local_halfs(self):
        """An earlier version's record held the host's name alone: whether its
        run had a local half is unknown, so a run here must not replace one."""
        pending = mutants.remote_pending()
        pending.parent.mkdir(parents=True, exist_ok=True)
        pending.write_text("bonfire\n", encoding="utf-8")
        refusal = mutants.local_refusal()
        self.assertIn("launched on bonfire", refusal)
        self.assertIn("mutants-collect", refusal)

    def test_a_collect_finding_no_half_clears_a_stale_record(self):
        pending = pend()
        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: "none"),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.collect("bonfire"), 1)
        self.assertFalse(pending.exists())


class RunQueue(unittest.TestCase):
    """A run snapshots the tree when its command starts and only then waits for the run in
    progress, so nothing launched while another runs has to be held back."""

    def test_a_second_run_waits_for_the_first_to_end(self):
        child = (
            "import sys\n"
            "from pathlib import Path\n"
            "sys.path.insert(0, sys.argv[1])\n"
            "import mutants\n"
            "mutants.LOCK_POLL_SECONDS = 0.05\n"
            "Path(sys.argv[4]).touch()\n"
            "with mutants.run_lock(Path(sys.argv[2])):\n"
            "    Path(sys.argv[3]).touch()\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            lock = Path(tmp) / "mutants.lock"
            entered, ready = Path(tmp) / "entered", Path(tmp) / "ready"
            argv = [sys.executable, "-c", child, str(SCRIPTS), str(lock), str(entered), str(ready)]
            with mutants.run_lock(lock):
                second = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                try:
                    # The child must reach the lock while it is held: one that starts after the
                    # release enters freely, and the test would pass with the lock broken.
                    deadline = time.monotonic() + 30
                    while not ready.exists() and second.poll() is None:
                        self.assertLess(time.monotonic(), deadline, "the second run never started")
                        time.sleep(0.05)
                    self.assertTrue(ready.exists(), "the second run ended before the lock")
                    with contextlib.suppress(subprocess.TimeoutExpired):
                        second.wait(timeout=1)
                    self.assertFalse(entered.exists(), "the second run started inside the first")
                except BaseException:
                    second.kill()
                    raise
            out, _ = second.communicate(timeout=30)
            self.assertTrue(entered.exists(), out.decode(errors="replace"))

    def test_the_lock_is_the_users_not_one_worktrees(self):
        self.assertEqual(mutants.RUN_LOCK.name, "mutants.lock")
        self.assertFalse(mutants.RUN_LOCK.is_relative_to(mutants.ROOT))

    def order(self, call) -> list[str]:
        """The events of `call(events)`, with the snapshot, the lock and the snapshot's
        extracted copy recorded."""
        events = []

        @contextlib.contextmanager
        def lock(*_):
            events.append("lock")
            yield
            events.append("unlock")

        @contextlib.contextmanager
        def source(tree):
            events.append("copy")
            yield Path(tree)
            events.append("uncopy")

        def snapshot():
            events.append("snapshot")
            return HEAD_TREE

        with mock.patch.object(mutants, "working_tree", snapshot), \
                mock.patch.object(mutants, "run_lock", lock), \
                mock.patch.object(mutants, "snapshot_source", source):
            call(events)
        return events

    def test_a_diff_run_snapshots_before_it_waits(self):
        def call(events):
            with mock.patch.object(mutants_diff, "run", lambda *_: events.append("run") or 0):
                mutants_diff.main(["mutants_diff.py", EMPTY_TREE, "scripts/mutation/mutants.py"])

        self.assertEqual(
            self.order(call), ["snapshot", "lock", "copy", "run", "uncopy", "unlock"]
        )

    def test_a_whole_crate_run_snapshots_before_it_waits(self):
        """The lock and the snapshot's copy moved into `whole_run`, which must be
        free to drop the lock while the remote half runs: what `main` still
        guarantees is the snapshot at the command's start."""
        def call(events):
            with mock.patch.object(mutants, "whole_run", lambda *_: events.append("run") or 0):
                mutants.main(["mutants.py", "fpc"])

        self.assertEqual(self.order(call), ["snapshot", "run"])

    def test_a_refused_whole_crate_run_exits_1_saying_why(self):
        out = io.StringIO()

        def refuse(*_):
            raise mutants.Refused("`just gates-remote` is running on bonfire")

        def call(events):
            with mock.patch.object(mutants, "whole_run", refuse), contextlib.redirect_stdout(out):
                self.assertEqual(mutants.main(["mutants.py", "fpc"]), 1)

        self.order(call)
        self.assertIn("`just gates-remote` is running on bonfire", out.getvalue())

    def test_a_collect_is_not_wrapped_in_the_lock(self):
        """A collect's wait on the remote half lasts as long as the half does, so
        `main` holds no lock over it: the fetch and summary take it inside
        `collect` (`CollectRemote`)."""

        def call(events):
            with mock.patch.object(mutants, "remote_host", lambda: "bonfire"), \
                    mock.patch.object(mutants, "collect", lambda *_: events.append("run") or 0):
                mutants.main(["mutants.py", "--collect"])

        self.assertEqual(self.order(call), ["run"])

    def test_an_empty_diff_sets_the_last_results_aside_in_its_turn(self):
        """A rerun after fixes that changed nothing under its paths still leaves no results
        of an older run standing as its own, and moves them only when no run uses them."""

        def call(events):
            with mock.patch.object(mutants, "set_aside", lambda *_: events.append("set aside")), \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(
                    mutants_diff.main(["mutants_diff.py", HEAD_TREE, "scripts/mutation/mutants.py"]), 0
                )

        self.assertEqual(self.order(call), ["snapshot", "lock", "set aside", "unlock"])

    def test_an_extra_mutants_run_snapshots_before_it_waits(self):
        def call(events):
            def run_crate(*_):
                events.append("run")
                return [], Counter(), 0.0

            with mock.patch.object(extra_mutants, "run_crate", run_crate):
                extra_mutants.main(["extra_mutants.py", "fpc"])

        self.assertEqual(self.order(call), ["snapshot", "lock", "run", "unlock"])

    def test_the_last_runs_results_are_set_aside_not_deleted(self):
        with tempfile.TemporaryDirectory() as tmp:
            results = Path(tmp) / "mutants.diff"
            results.mkdir()
            (results / "missed_all.txt").write_text("a survivor\n", encoding="utf-8")
            old = Path(tmp) / "mutants.diff.old"
            old.mkdir()
            (old / "older").write_text("", encoding="utf-8")
            mutants.set_aside(results)
            self.assertFalse(results.exists())
            self.assertEqual((old / "missed_all.txt").read_text(encoding="utf-8"), "a survivor\n")
            self.assertFalse((old / "older").exists())


class WholeRun(unittest.TestCase):
    """`just mutants <crate>` runs every mutant on the VPS alone (RUNBOOK
    "Mutation runs and the VPS"): the launch under the run lock, the days-long
    wait without it, and nothing cargo-mutants or extra on this machine."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(mutants, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def launch(self, remote_only: bool = False) -> tuple[int, dict, str]:
        """`whole_run` with everything past the refusals stubbed: the recorded
        `upload_and_launch` and `finish_remote_run` arguments, and the output."""
        launched, finished = {}, {}
        out = io.StringIO()

        def upload(*args, **kwargs):
            launched["args"] = args
            launched.update(kwargs)

        def finish(*args, **kwargs):
            finished["args"] = args
            finished.update(kwargs)
            return 0

        with (
            mock.patch.object(mutants, "REMOTE_ONLY", remote_only),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(
                mutants, "snapshot_source", lambda tree: contextlib.nullcontext(self.root)
            ),
            mock.patch.object(mutants, "remote_start", lambda host, tree: (None, None, 8)),
            mock.patch.object(mutants, "upload_and_launch", upload),
            mock.patch.object(mutants, "wait_remote", lambda host: (0, 9.0)),
            mock.patch.object(mutants, "finish_remote_run", finish),
            mock.patch.object(
                mutants, "local_mutants",
                lambda *a, **k: self.fail("cargo-mutants ran here"),
            ),
            mock.patch.object(
                mutants, "plan_file_split",
                lambda *a, **k: self.fail("a file split was planned"),
            ),
            mock.patch.object(
                extra_mutants, "run_crate",
                lambda *a, **k: self.fail("the extra pass ran here"),
            ),
            contextlib.redirect_stdout(out),
        ):
            code = mutants.whole_run("cpk", "bonfire", HEAD_TREE)
        return code, {"launch": launched, "finish": finished}, out.getvalue()

    def test_without_a_host_or_for_a_local_only_crate_a_whole_run_refuses(self):
        with (
            mock.patch.object(
                mutants, "set_aside_last_run", lambda: self.fail("set aside")
            ),
            mock.patch.object(
                mutants, "run_lock", lambda *a, **k: self.fail("the run lock was taken")
            ),
            mock.patch.object(
                mutants, "ssh", lambda *a, **k: self.fail("ssh was called")
            ),
            mock.patch.object(
                mutants.subprocess, "run", lambda *a, **k: self.fail("cargo ran")
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaisesRegex(mutants.Refused, "VPS.*STUDIO_MUTANTS_REMOTE"):
                mutants.whole_run("cpk", None, HEAD_TREE)
            with mock.patch.object(mutants, "LOCAL_ONLY_CRATES", frozenset({"cpk"})):
                with self.assertRaisesRegex(mutants.Refused, "cpk.*LOCAL_ONLY_CRATES"):
                    mutants.whole_run("cpk", "bonfire", HEAD_TREE)

    def test_every_mutant_runs_remote_whatever_remote_only_says(self):
        for remote_only in (False, True):
            code, calls, out = self.launch(remote_only=remote_only)
            self.assertEqual(code, 0)
            self.assertEqual(
                calls["launch"],
                {
                    # The crate as a selection, the snapshot, its copy, and what
                    # `remote_start` answered.
                    "args": ("bonfire", ["-p", "cpk"], HEAD_TREE, self.root, None, None, 8),
                    "whole": True, "local_half": False, "extra": True,
                },
            )
            self.assertEqual(calls["finish"]["local_half"], False)
            self.assertEqual(
                calls["finish"]["extra_run"],
                self.root / "mutants.out" / "remote" / "extra_mutants.out" / "run.json",
            )
            self.assertEqual(calls["finish"]["args"], ("bonfire", 0, 0, (None, 9.0)))
        self.assertTrue(
            out.startswith("running every mutant on bonfire (a whole-crate run)"),
            out,
        )

    def test_the_wait_holds_no_run_lock(self):
        """A whole run waits days on its remote half; the run lock covers only
        the launch and the fetch-and-summary, so a diff run can go ahead."""
        with (
            lock_probes(self.root) as probe,
            mock.patch.object(
                mutants, "snapshot_source",
                lambda tree: contextlib.nullcontext(self.root),
            ),
            mock.patch.object(
                mutants, "remote_start",
                lambda host, tree: probe("start") or (None, None, 8),
            ),
            mock.patch.object(
                mutants, "upload_and_launch", lambda *a, **k: probe("launch")
            ),
            mock.patch.object(
                mutants, "wait_remote", lambda host: probe("wait") or (0, 1.0)
            ),
            mock.patch.object(
                mutants, "finish_remote_run", lambda *a, **k: probe("finish") or 0
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.whole_run("cpk", "bonfire", HEAD_TREE), 0)
        self.assertEqual(
            probe.calls,
            [("start", False), ("launch", False), ("wait", True), ("finish", False)],
        )

    def test_remote_extra_records_the_pass_and_fails_on_an_untested_mutant(self):
        """The remote job's `--remote-extra` runs the extra kinds on the clone's
        own tree and records them for the fetch, failing on an untested mutant."""
        runs = []
        with (
            mock.patch.object(
                extra_mutants, "run_crate",
                lambda *a: runs.append(a) or ([], Counter(caught=2), 3.5),
            ),
            mock.patch.object(
                mutants, "run_lock", lambda *a, **k: self.fail("a lock was taken")
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.main(["mutants.py", "--remote-extra", "cpk"]), 0)
        self.assertEqual(runs, [("cpk", list(mutants.EXTRA_KINDS))])
        record = json.loads(
            (self.root / "extra_mutants.out" / "run.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            record, {"crate": "cpk", "counts": {"caught": 2}, "seconds": 3.5}
        )
        with (
            mock.patch.object(
                extra_mutants, "run_crate", lambda *a: ([], Counter(untested=1), 1.0)
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.main(["mutants.py", "--remote-extra", "cpk"]), 1)

    def test_remote_extra_with_kinds_runs_those_alone(self):
        """An extras-only run's job narrows the pass by naming kinds after the
        crate; none named runs `EXTRA_KINDS`, as the test above holds."""
        runs = []
        with (
            mock.patch.object(
                extra_mutants, "run_crate",
                lambda *a: runs.append(a) or ([], Counter(caught=1), 1.0),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(
                mutants.main(["mutants.py", "--remote-extra", "cpk", "call"]), 0
            )
            self.assertEqual(
                mutants.main(
                    ["mutants.py", "--remote-extra", "cpk", "call", "bool"]
                ),
                0,
            )
        self.assertEqual(runs, [("cpk", ["call"]), ("cpk", ["call", "bool"])])


class ExtraOnlyRun(unittest.TestCase):
    """`just extra-mutants-remote` (`mutants.py --extra-only`): the extra kinds
    alone on the VPS — a kind's backfill over the crates measures for days of
    its own, so it goes where a whole run goes, without the cargo-mutants
    part. The refusals are a whole run's, the launch, wait and finish shaped
    the same; `mutants.out` and the cost table go untouched."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(mutants, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_the_refusals_of_a_whole_run_hold(self):
        with (
            mock.patch.object(
                mutants, "run_lock", lambda *a, **k: self.fail("the run lock was taken")
            ),
            mock.patch.object(
                mutants, "snapshot_source",
                lambda tree: self.fail("the snapshot was copied"),
            ),
            mock.patch.object(
                mutants, "ssh", lambda *a, **k: self.fail("ssh was called")
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaisesRegex(mutants.Refused, "unknown.*nonsense"):
                mutants.extra_only_run("cpk", ["nonsense"], "bonfire", HEAD_TREE)
            with self.assertRaisesRegex(mutants.Refused, "VPS.*STUDIO_MUTANTS_REMOTE"):
                mutants.extra_only_run("cpk", ["call"], None, HEAD_TREE)
            # `guard` is a real kind (`ALL_KINDS`): it passes the kind check and
            # meets the no-host refusal like the rest.
            with self.assertRaisesRegex(mutants.Refused, "VPS.*STUDIO_MUTANTS_REMOTE"):
                mutants.extra_only_run("cpk", ["guard"], None, HEAD_TREE)
            with mock.patch.object(mutants, "LOCAL_ONLY_CRATES", frozenset({"cpk"})):
                with self.assertRaisesRegex(mutants.Refused, "cpk.*LOCAL_ONLY_CRATES"):
                    mutants.extra_only_run("cpk", ["call"], "bonfire", HEAD_TREE)
        with (
            mock.patch.object(mutants, "remote_refusal", lambda host: "a busy host"),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(
                mutants, "snapshot_source",
                lambda tree: contextlib.nullcontext(self.root),
            ),
            mock.patch.object(
                mutants, "upload_and_launch", lambda *a, **k: self.fail("launched")
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaisesRegex(mutants.Refused, "busy"):
                mutants.extra_only_run("cpk", ["call"], "bonfire", HEAD_TREE)

    def test_the_launch_waits_and_finishes_as_a_whole_run_does(self):
        """The launch and the finish hold the run lock, the wait none, and
        nothing of `mutants.out` is set aside or folded into the cost table."""
        launched, finished = {}, {}
        with (
            lock_probes(self.root) as probe,
            mock.patch.object(
                mutants, "snapshot_source",
                lambda tree: contextlib.nullcontext(self.root),
            ),
            mock.patch.object(
                mutants, "remote_start",
                lambda host, tree: probe("start") or (None, None, 8),
            ),
            mock.patch.object(
                mutants, "upload_and_launch",
                lambda *a, **k: probe("launch") or launched.update(args=a, kwargs=k),
            ),
            mock.patch.object(
                mutants, "wait_remote", lambda host: probe("wait") or (7, 1.0)
            ),
            mock.patch.object(
                mutants, "finish_extra_run",
                lambda *a, **k: probe("finish") or finished.update(args=a, kwargs=k) or 1,
            ),
            mock.patch.object(
                mutants, "set_aside_last_run", lambda: self.fail("set aside")
            ),
            mock.patch.object(
                mutants, "update_cost_cache", lambda *a, **k: self.fail("the cost table")
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(
                mutants.extra_only_run("cpk", ["call"], "bonfire", HEAD_TREE), 1
            )
        self.assertEqual(
            probe.calls,
            [("start", False), ("launch", False), ("wait", True), ("finish", False)],
        )
        self.assertEqual(launched["args"][1], ["-p", "cpk"])
        self.assertEqual(
            launched["kwargs"],
            {"whole": True, "local_half": False, "extra": False,
             "extra_only": True, "kinds": ["call"]},
        )
        self.assertEqual(finished["args"], ("bonfire", 7))

    def test_main_extra_only_parses_kind_names(self):
        calls = []
        with (
            mock.patch.object(mutants, "working_tree", lambda: HEAD_TREE),
            mock.patch.object(
                mutants, "extra_only_run",
                lambda crate, kinds, host, tree: calls.append((crate, kinds, host)) or 0,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(
                mutants.main(["mutants.py", "--extra-only", "cpk", "--kind", "call"]), 0
            )
            self.assertEqual(
                mutants.main(
                    ["mutants.py", "--extra-only", "cpk",
                     "--kind", "call", "--kind", "bool"]
                ),
                0,
            )
            self.assertEqual(mutants.main(["mutants.py", "--extra-only", "cpk"]), 0)
            self.assertEqual(mutants.main(["mutants.py", "--extra-only"]), 1)
            self.assertEqual(
                mutants.main(["mutants.py", "--extra-only", "cpk", "call"]), 1
            )
            self.assertEqual(
                mutants.main(["mutants.py", "--extra-only", "cpk", "--kind"]), 1
            )
        self.assertEqual(
            calls,
            [("cpk", ["call"], None), ("cpk", ["call", "bool"], None),
             ("cpk", [], None)],
        )


class SplitShares(unittest.TestCase):
    """Each half of a split tests its own share: halves given the same shard would halve a
    diff run, which has no coverage check, with every count looking right."""

    def run_split(self, local_files: set[str] | None) -> dict:
        calls = {}

        def local_mutants(args, source, config=None):
            calls["local"] = (args, config)
            return subprocess.CompletedProcess(args, 0)

        def launch_remote(host, selection, whole=False, **kwargs):
            calls["whole"] = whole
            calls["launch"] = kwargs

        with (
            mock.patch.multiple(
                mutants,
                local_mutants=local_mutants,
                launch_remote=launch_remote,
                transfer=lambda *_: None,
                remote_config=lambda *_: None,
                local_config=lambda source, extra=None: Path("local.toml"),
                wait_remote=lambda host: (0, 1.0),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            mutants.run_split(
                ["-p", "cpk"], "bonfire", HEAD_TREE, Path("source"), None, None, 8,
                local_files=local_files,
            )
        return calls

    def job(self, whole: bool, nproc: int = 8, **kwargs) -> str:
        jobs = []
        remotes = []

        def ssh(host, remote, **kwargs):
            remotes.append(remote)
            if "/job.sh" in remote and "input" in kwargs:
                jobs.append(kwargs["input"].decode())
            return subprocess.CompletedProcess([host, remote], 0)

        with (
            tempfile.TemporaryDirectory() as tmp,
            mock.patch.object(mutants, "ROOT", Path(tmp)),
            mock.patch.object(mutants, "ssh", ssh),
            mock.patch.object(mutants, "remote_memory_max", lambda host: 2**30),
        ):
            mutants.launch_remote(
                "bonfire", ["-p", "cpk"], whole=whole, nproc=nproc, **kwargs
            )
            self.record = json.loads(mutants.remote_pending().read_text(encoding="utf-8"))
        self.assertFalse(mutants.remote_pending().exists(), "the launch's record left behind")
        (job,) = jobs
        self.remotes = remotes
        return job

    def test_round_robin_halves_take_shards_0_and_1(self):
        calls = self.run_split(None)
        self.assertEqual(
            calls["local"], (["-p", "cpk", "--shard", "0/2", "--sharding", "round-robin"], None)
        )
        self.assertFalse(calls["whole"])
        self.assertEqual(
            calls["launch"],
            {"nproc": 8, "local_half": True, "extra": False,
             "extra_only": False, "kinds": ()},
        )
        job = self.job(whole=False)
        self.assertIn("--shard 1/2 --sharding round-robin ", job)
        # The remote's build cap: cargo-mutants' own option on its command
        # line (`additional_cargo_args` would reach `cargo nextest run`).
        self.assertIn("--jobserver-tasks 8 ", job)
        # `REMOTE_JOBSERVER_TASKS` is the knob to lower it with.
        with mock.patch.object(mutants, "REMOTE_JOBSERVER_TASKS", 2):
            self.assertIn("--jobserver-tasks 2 ", self.job(whole=False))

    def test_a_split_refuses_before_it_touches_any_result(self):
        """A refusal the caller counted as a run filed the last run's `mutants.out` as its
        own (`mutants_diff.run`): a split refuses by raising, before setting anything aside."""
        with (
            mock.patch.object(mutants, "remote_refusal", lambda host: "busy"),
            mock.patch.object(mutants, "set_aside_last_run", lambda: self.fail("set aside")),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaisesRegex(mutants.Refused, "busy"):
                mutants.split(["-p", "cpk"], "bonfire", HEAD_TREE, Path("source"))
        with (
            mock.patch.multiple(
                mutants,
                remote_refusal=lambda host: None,
                snapshot=lambda tree: tree,
                remote_state=lambda host: (None, None, 8, "cargo-mutants 25.3.1"),
                local_mutants_version=lambda: "cargo-mutants 25.2.0",
                set_aside_last_run=lambda: self.fail("set aside"),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            with self.assertRaisesRegex(mutants.Refused, "versions differ"):
                mutants.split(["-p", "cpk"], "bonfire", HEAD_TREE, Path("source"))

    def test_a_diff_split_is_planned_recorded_and_run_by_its_plan(self):
        """A diff run's split goes through `plan_file_split` with its own selection, its
        record is kept for the summary's coverage check, and a `files` plan reaches the
        halves; a run with no local half plans nothing and drops the last record."""
        diff = ["--in-diff", "in.diff", "-p", "cpk"]
        for method, files in (("files", ["a.rs"]), ("round-robin", [])):
            planned = []
            ran = {}

            def plan(selection, nproc, source):
                planned.append(selection)
                return {"method": method, "local_files": files, "names": ["m"]}

            def run_split(selection, host, tree, source, head, remote_tree, nproc, local, local_files):
                ran["local_files"] = local_files
                return 0, 0, 1.0, 1.0

            with (
                tempfile.TemporaryDirectory() as tmp,
                mock.patch.object(mutants, "ROOT", Path(tmp)),
                mock.patch.multiple(
                    mutants,
                    remote_start=lambda host, tree: (None, None, 8),
                    set_aside_last_run=lambda: None,
                    plan_file_split=plan,
                    run_split=run_split,
                    finish_remote_run=lambda *a, **k: 0,
                ),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                mutants.record_extra_run("cpk", {}, 1.0)
                mutants.split(diff, "bonfire", HEAD_TREE, Path("source"))
                self.assertEqual(planned, [diff])
                self.assertEqual(ran["local_files"], set(files) if files else None)
                record = json.loads(mutants.split_run_record().read_text(encoding="utf-8"))
                self.assertEqual(record["method"], method)
                self.assertFalse(mutants.extra_run_record().exists(), "a diff has no extra pass")
                mutants.split(diff, "bonfire", HEAD_TREE, Path("source"), local=False)
                self.assertEqual(planned, [diff], "a run with no local half plans nothing")
                self.assertFalse(mutants.split_run_record().exists())

    def test_a_file_split_takes_no_shard_on_either_side(self):
        calls = self.run_split({"crates/libs/cpk/src/read.rs"})
        self.assertEqual(calls["local"], (["-p", "cpk"], Path("local.toml")))
        self.assertTrue(calls["whole"])
        self.assertNotIn("--shard", self.job(whole=True))

    def test_a_whole_runs_job_runs_the_extra_pass_first_and_a_diffs_never(self):
        """A whole run's remote job runs the extra kinds before cargo-mutants,
        logging the pass's code but leaving `run/exit` cargo-mutants'; the
        cleanup clears the clone's
        `extra_mutants.out` with its `mutants.out`, so a failed pass leaves no
        last run's record to fetch."""
        job = self.job(whole=True, local_half=False, extra=True)
        # What `collect` reads when the launching command is gone by the fetch.
        self.assertEqual(
            self.record, {"host": "bonfire", "local_half": False, "extra": True}
        )
        self.assertIn(
            "nice -n 19 ionice -c3 python3 scripts/mutation/"
            "mutants.py --remote-extra cpk\n",
            job,
        )
        self.assertNotIn("CARGO_BUILD_JOBS", job)
        self.assertLess(
            job.index("--remote-extra"), job.index("cargo-mutants mutants -p cpk --jobs")
        )
        self.assertIn('echo "extra pass exit $?"', job)
        self.assertLess(
            job.index("extra pass exit"), job.index("cargo-mutants mutants -p cpk --jobs")
        )
        self.assertLess(job.index("cargo-mutants mutants -p cpk --jobs"), job.index("code=$?"))
        self.assertTrue(
            any("extra_mutants.out" in remote for remote in self.remotes),
            "the launch's cleanup never removed the clone's extra_mutants.out",
        )
        diff_job = self.job(whole=False)
        self.assertNotIn("remote-extra", diff_job)
        self.assertNotIn("extra pass exit", diff_job)

    def test_an_extras_only_job_is_the_extra_pass_alone(self):
        """An extras-only run's remote job is the extra pass alone (`extra_only`
        suffices for it and its record mark): no `cargo mutants` line, and
        `run/exit` written from the pass's own code. A whole run's job is
        pinned byte for byte below."""
        job = self.job(whole=True, extra=False, extra_only=True, kinds=["call"])
        self.assertNotIn("cargo mutants", job)
        self.assertNotIn("extra pass exit", job)
        self.assertIn(
            ". ~/.cargo/env && nice -n 19 ionice -c3 python3 "
            "scripts/mutation/mutants.py --remote-extra cpk call\ncode=$?\n",
            job,
        )
        self.assertNotIn("CARGO_BUILD_JOBS", job)
        job = self.job(
            whole=True, extra=False, extra_only=True, kinds=["bool", "call"]
        )
        self.assertIn("--remote-extra cpk bool call\n", job)
        whole = """\
#!/bin/bash
exec > ../run/log 2>&1
start=$(date +%s)
export TMPDIR="$HOME/studio-mutants/tmp" && mkdir -p "$TMPDIR"
. ~/.cargo/env && nice -n 19 ionice -c3 python3 scripts/mutation/mutants.py --remote-extra cpk
echo "extra pass exit $?"
cargo-mutants mutants -p cpk --list --json --config ../mutants.remote.toml > ../run/mutants.json
nice -n 19 ionice -c3 python3 scripts/mutation/coverage_map.py cpk ../run/mutants.json ../run/selection.json
echo "coverage map exit $?"
if [ -f ../run/selection.json ]; then
CARGO=$PWD/scripts/mutation/select_tests.py UTW_REAL_CARGO=$(command -v cargo) UTW_MUTANTS_SOURCE=$PWD UTW_MUTANTS_SELECTION=$PWD/../run/selection.json nice -n 19 ionice -c3 cargo-mutants mutants -p cpk --jobs 2 --jobserver-tasks 8 --config ../mutants.remote.toml
else
nice -n 19 ionice -c3 cargo-mutants mutants -p cpk --jobs 2 --jobserver-tasks 8 --config ../mutants.remote.toml
fi
code=$?
peak="/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)/memory.peak"
[ -f "$peak" ] && cp "$peak" ../run/memory_peak
echo "$code $(( $(date +%s) - start ))" > ../run/exit
"""
        self.assertEqual(self.job(whole=True, local_half=False, extra=True), whole)

    def test_a_whole_runs_job_selects_each_mutants_tests(self):
        """A whole run's remote job lists the crate's mutants, builds the
        coverage map from them (its code logged but not `run/exit`'s) and
        starts cargo-mutants directly — `cargo
        mutants` would overwrite `CARGO` — with the stand-in's four variables
        on the selection-file branch only."""
        job = self.job(whole=True, local_half=False, extra=True)
        self.assertIn(
            "cargo-mutants mutants -p cpk --list --json "
            "--config ../mutants.remote.toml > ../run/mutants.json\n",
            job,
        )
        self.assertIn(
            "nice -n 19 ionice -c3 python3 scripts/mutation/"
            "coverage_map.py cpk ../run/mutants.json ../run/selection.json\n",
            job,
        )
        self.assertNotIn("CARGO_BUILD_JOBS", job)
        self.assertIn('echo "coverage map exit $?"', job)
        self.assertNotIn("cargo mutants", job)
        self.assertIn("cargo-mutants mutants -p cpk --jobs 2 ", job)
        # Order: extra pass, listing, map, cargo-mutants, `code=$?`.
        self.assertLess(job.index("--remote-extra"), job.index("--list --json"))
        self.assertLess(job.index("--list --json"), job.index("coverage_map.py"))
        self.assertLess(job.index("coverage_map.py"), job.index('echo "coverage map exit'))
        self.assertLess(
            job.index('echo "coverage map exit'),
            job.index("cargo-mutants mutants -p cpk --jobs"),
        )
        self.assertLess(
            job.index("cargo-mutants mutants -p cpk --jobs"), job.index("code=$?")
        )
        # The four variables appear once each, on the selection branch.
        self.assertIn("if [ -f ../run/selection.json ]; then\n", job)
        args = "-p cpk --jobs 2 --jobserver-tasks 8 --config ../mutants.remote.toml"
        self.assertIn(
            "CARGO=$PWD/scripts/mutation/select_tests.py "
            "UTW_REAL_CARGO=$(command -v cargo) UTW_MUTANTS_SOURCE=$PWD "
            "UTW_MUTANTS_SELECTION=$PWD/../run/selection.json "
            f"nice -n 19 ionice -c3 cargo-mutants mutants {args}\n"
            "else\n"
            f"nice -n 19 ionice -c3 cargo-mutants mutants {args}\n"
            "fi\n",
            job,
        )
        for var in ("UTW_REAL_CARGO", "UTW_MUTANTS_SOURCE", "UTW_MUTANTS_SELECTION"):
            self.assertEqual(job.count(var), 1, var)
        # The branch's `CARGO=` starts its line (`UTW_REAL_CARGO` ends in
        # "CARGO=" too, so a bare count of it proves nothing).
        self.assertEqual(job.count("\nCARGO="), 1)

    def test_only_a_whole_runs_job_selects(self):
        """A diff run's job and an extras-only one carry nothing of the
        selection: no listing, no coverage map, no stand-in, no `CARGO=`."""
        for job in (
            self.job(whole=False),
            self.job(whole=True),
            self.job(whole=True, extra=False, extra_only=True, kinds=["call"]),
        ):
            for text in (
                "coverage_map", "select_tests", "CARGO=", "--list --json",
                "CARGO_BUILD_JOBS",
            ):
                self.assertNotIn(text, job)

    def test_the_launch_cleanup_removes_the_last_runs_selection_files(self):
        """`run/mutants.json` and `run/selection.json` are removed with the
        rest at launch: a failed listing must not pair an old run's map."""
        self.job(whole=True, local_half=False, extra=True)
        cleanup = next(
            remote for remote in self.remotes if "rm -f" in remote
        )
        self.assertIn("run/mutants.json", cleanup)
        self.assertIn("run/selection.json", cleanup)


class SummaryLists(unittest.TestCase):
    """What a finished split run's summary writes into `missed_all.txt`, the file triage
    reads, and when it fails the run."""

    def summarize(
        self,
        root: Path,
        local_half: bool = True,
        extra_run: Path | None = None,
        codes: tuple[int, int] = (0, 0),
        remote_only: bool = False,
    ) -> tuple[int, str]:
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "ROOT", root),
            mock.patch.object(mutants, "REMOTE_ONLY", remote_only),
            contextlib.redirect_stdout(out),
        ):
            code = mutants.summarize(
                *codes, root / "remote_console.txt", (None, None),
                local_half=local_half, extra_run=extra_run,
            )
        return code, out.getvalue()

    def test_survivors_and_timeouts_of_both_halves_and_the_extra_pass_merge_line_by_line(self):
        def outcomes(*names: str) -> list[dict]:
            return [{"scenario": mutant(name), "summary": "MissedMutant"} for name in names]

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            local = root / "mutants.out"
            remote = local / "remote" / "mutants.out"
            write_run(local, 2, outcomes("a", "b"))
            write_run(remote, 2, outcomes("c", "d"))
            # No final newline: the remote half's first line must still start a line.
            (local / "missed.txt").write_text(
                "crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0", encoding="utf-8"
            )
            (local / "timeout.txt").write_text(
                "crates/libs/cpk/src/lib.rs:20:9: replace < with <= in pad\n", encoding="utf-8"
            )
            (remote / "missed.txt").write_text(
                "crates/libs/cpk/src/read.rs:30:5: replace read -> u8 with 0\n", encoding="utf-8"
            )
            (remote / "timeout.txt").write_text(
                "crates/libs/cpk/src/read.rs:40:13: delete ! in read\n", encoding="utf-8"
            )
            with mock.patch.object(mutants, "ROOT", root):
                mutants.record_extra_run("cpk", {"missed": 1}, 5.0)
            (root / "extra_mutants.out" / "missed_cpk.txt").write_text(
                "crates/libs/cpk/src/lib.rs:50: [literal] remove `b\"CPK \"` from the magic list\n",
                encoding="utf-8",
            )
            code, _ = self.summarize(
                root, extra_run=root / "extra_mutants.out" / "run.json"
            )
            self.assertEqual(code, 2)
            self.assertEqual(
                (local / "missed_all.txt").read_text(encoding="utf-8").splitlines(),
                [
                    "crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0",
                    "crates/libs/cpk/src/lib.rs:20:9: [timeout] replace < with <= in pad",
                    "crates/libs/cpk/src/read.rs:30:5: replace read -> u8 with 0",
                    "crates/libs/cpk/src/read.rs:40:13: [timeout] delete ! in read",
                    "crates/libs/cpk/src/lib.rs:50: [literal] remove `b\"CPK \"` from the magic list",
                ],
            )

    def test_a_killed_build_fails_a_clean_run_and_names_its_mutant(self):
        outcomes = [
            {"scenario": mutant("replace pad -> usize with 0"), "summary": "Unviable",
             "log_path": "log/pad"},
        ]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_run(
                root / "mutants.out", 1, outcomes,
                {"log/pad": "error: could not compile (signal: 9, SIGKILL: kill)"},
            )
            code, out = self.summarize(root)
        self.assertEqual(code, 1)
        self.assertIn("1 mutants were NOT tested", out)
        self.assertIn("  replace pad -> usize with 0", out)

    def test_an_untested_mutant_outranks_a_survivor(self):
        """A run with a missed mutant (cargo-mutants' 2) and a catch whose tests
        never failed (this script's untested 1) exits 1: the survivor is in
        `missed_all.txt`, the untested mutant is in no list."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            local = root / "mutants.out"
            write_run(
                local, 2,
                [
                    {"scenario": mutant("a"), "summary": "MissedMutant"},
                    caught_outcome("b", {"Failure": 104}),
                ],
            )
            (local / "missed.txt").write_text(
                "crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0\n",
                encoding="utf-8",
            )
            code, out = self.summarize(root)
        self.assertEqual(code, 1)
        self.assertIn("NOT tested", out)

    def test_a_halfs_own_failure_outranks_a_survivor(self):
        """A half that died or errored (its own 1) beside another half's missed
        mutant (2) exits 1: the survivor is listed, the untested mutants are
        in no list."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            local = root / "mutants.out"
            write_run(local, 1, [{"scenario": mutant("l"), "summary": "MissedMutant"}])
            (local / "missed.txt").write_text(
                "crates/libs/cpk/src/lib.rs:2:2: a local survivor\n", encoding="utf-8"
            )
            # The remote half wrote nothing: it died before its selection. Its
            # code (1) makes the summary tail this log.
            (root / "remote_console.txt").write_text("died\n", encoding="utf-8")
            code, _ = self.summarize(root, codes=(2, 1))
        self.assertEqual(code, 1)

    def test_an_extra_passs_untested_mutants_fail_the_run(self):
        """`counts["untested"]` in the extra pass's record means mutants went
        untested outside the halves' lists: the run exits 1."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}])
            record = root / "extra_mutants.out" / "run.json"
            record.parent.mkdir(parents=True)
            record.write_text(
                json.dumps(
                    {"crate": "cpk", "seconds": 4.0, "counts": {"untested": 1}}
                ),
                encoding="utf-8",
            )
            code, _ = self.summarize(root, local_half=False, extra_run=record)
        self.assertEqual(code, 1)

    def test_a_half_stopped_early_outranks_a_survivor(self):
        """`never_ran` sets `untested` with no killed build to list: beside a
        survivor the run exits exactly 1, not the survivor's 2."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(remote, 3, [{"scenario": mutant("r"), "summary": "MissedMutant"}])
            (remote / "missed.txt").write_text(
                "crates/libs/cpk/src/read.rs:2:2: a remote survivor\n", encoding="utf-8"
            )
            code, out = self.summarize(root, local_half=False)
        self.assertEqual(code, 1)
        self.assertIn("never ran", out)

    def test_the_fetched_extra_record_merges_and_a_missing_one_fails(self):
        """A whole run's extra pass record comes home with the fetch: its counts
        join the table and its `missed_<crate>.txt` the merged list; a run that
        should have the record and has none lost the pass's outcomes, so the
        summary fails instead of filing them clean."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}])
            extra = root / "mutants.out" / "remote" / "extra_mutants.out"
            extra.mkdir(parents=True)
            record = extra / "run.json"
            record.write_text(
                json.dumps(
                    {"crate": "cpk", "seconds": 4.0,
                     "counts": {"missed": 1, "caught": 2}}
                ),
                encoding="utf-8",
            )
            (extra / "missed_cpk.txt").write_text(
                "crates/libs/cpk/src/lib.rs:5: [bool] flip it\n", encoding="utf-8"
            )
            code, out = self.summarize(
                root, local_half=False, extra_run=record
            )
            self.assertEqual(code, 2)
            self.assertEqual(
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines(),
                ["crates/libs/cpk/src/lib.rs:5: [bool] flip it"],
            )
            self.assertIn("extra", out)
            # A survivor beside the missing record: the rerun's code really is 2.
            write_run(remote, 2, [
                {"scenario": mutant("r"), "summary": "CaughtMutant"},
                {"scenario": mutant("m"), "summary": "MissedMutant"},
            ])
            (remote / "missed.txt").write_text(
                "crates/libs/cpk/src/read.rs:7:7: a remote survivor\n", encoding="utf-8"
            )
            record.unlink()
            seen = []
            real = mutants.report_killed

            def spy(code, **kwargs):
                seen.append(code)
                return real(code, **kwargs)

            with mock.patch.object(mutants, "report_killed", spy):
                code, out = self.summarize(
                    root, local_half=False, extra_run=record
                )
        # A missing record over a survivor is still 1: untested outranks missed,
        # and the survivor really reached `report_killed` as a 2.
        self.assertEqual(seen, [2])
        self.assertEqual(code, 1)
        self.assertIn("the extra pass left no record: see the remote log", out)

    ELEVATION_SURVIVOR = (
        "crates/libs/elevation/src/windows.rs:64: [call] in relaunch_elevated: "
        "the push() statement is deleted | removed: parameters.push(0);"
    )
    TEAMS_SURVIVOR = (
        "crates/libs/teams_list/src/reconcile.rs:74: [call] in appended_cells: "
        "the resize() statement is deleted | removed: "
        "cells.resize(width.max(cells.len()), String::new());"
    )

    def extra_record(
        self, root: Path, lines: list[str], counts: dict, *, remote: bool = True
    ) -> Path:
        """The extra pass's `run.json` (crate `elevation`) and its
        `missed_elevation.txt` holding `lines`: under `mutants.out/remote/`
        when `remote` (the record a whole run's fetch lands), under
        `extra_mutants.out/` else (a local pass's)."""
        extra = (
            root / "mutants.out" / "remote" / "extra_mutants.out"
            if remote
            else root / "extra_mutants.out"
        )
        extra.mkdir(parents=True, exist_ok=True)
        (extra / "missed_elevation.txt").write_text(
            "".join(line + "\n" for line in lines), encoding="utf-8"
        )
        record = extra / "run.json"
        record.write_text(
            json.dumps({"crate": "elevation", "seconds": 4.0, "counts": counts}),
            encoding="utf-8",
        )
        return record

    @contextlib.contextmanager
    def windows_recheck(self, run_crate):
        """The Windows recheck's surroundings mocked: `git rev-parse` answers
        a tree, `mutant_crate` maps `crates/{libs,tools}/<crate>` and
        `crates/<crate>` paths, this machine is Windows, and
        `extra_mutants.run_crate` is `run_crate`."""
        def git(*args, **kwargs):
            return subprocess.CompletedProcess(args, 0, stdout="sometree")

        def mutant_crate(file: str) -> str:
            parts = file.split("/")
            return parts[2] if parts[1] in ("libs", "tools") else parts[1]

        with (
            mock.patch.object(mutants, "git", git),
            mock.patch.object(mutants, "mutant_crate", mutant_crate),
            mock.patch.object(mutants.os, "name", "nt"),
            mock.patch.object(extra_mutants, "run_crate", run_crate),
        ):
            yield

    def test_a_fetched_extra_records_windows_only_survivors_are_rechecked(self):
        """The remote pass's survivors in Windows-only code get the
        cargo-mutants recheck's counterpart: `run_crate` reruns them alone on
        this machine, a catch leaving `missed_all.txt` and moving the extra
        row's counts (missed down, caught up) as the remote row's do."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(
                remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}]
            )
            record = self.extra_record(
                root, [self.ELEVATION_SURVIVOR, self.TEAMS_SURVIVOR],
                {"missed": 2, "caught": 1},
            )
            calls = []

            def run_crate(crate, kinds, tree=None, only=None):
                calls.append((crate, tree, only))
                return [], Counter(caught=1), 0.5

            with self.windows_recheck(run_crate):
                code, out = self.summarize(
                    root, local_half=False, extra_run=record
                )
            self.assertEqual(code, 2)
            self.assertEqual(
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines(),
                [self.TEAMS_SURVIVOR],
            )
        # "sometree": the remote run's tree, as `windows_recheck`'s git answers it.
        self.assertEqual(calls, [("elevation", "sometree", {self.ELEVATION_SURVIVOR})])
        extra_row = next(
            line for line in out.splitlines() if line.startswith("extra")
        )
        self.assertEqual(extra_row.split()[1:5], ["2", "1", "0", "0"])
        self.assertIn(
            "rechecked on this machine (Windows-only extra mutants): "
            "1 caught, 0 still missed",
            out,
        )

    def test_windows_only_extra_survivors_are_listed_when_not_rechecked(self):
        """Under REMOTE_ONLY this machine runs nothing: the survivors stay
        missed, named under a heading like the cargo-mutants ones, and
        `run_crate` is never called."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(
                remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}]
            )
            record = self.extra_record(
                root, [self.ELEVATION_SURVIVOR, self.TEAMS_SURVIVOR],
                {"missed": 2},
            )
            calls = []

            def run_crate(crate, kinds, tree=None, only=None):
                calls.append(crate)
                return [], Counter(), 0.0

            with self.windows_recheck(run_crate):
                code, out = self.summarize(
                    root, local_half=False, extra_run=record, remote_only=True
                )
            self.assertEqual(code, 2)
            self.assertEqual(calls, [])
            self.assertEqual(
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines(),
                [self.ELEVATION_SURVIVOR, self.TEAMS_SURVIVOR],
            )
        self.assertIn("Windows-only extra mutants, not rechecked:", out)
        self.assertIn(f"  {self.ELEVATION_SURVIVOR}", out)
        self.assertNotIn(f"  {self.TEAMS_SURVIVOR}", out)

    def test_a_local_extra_records_survivors_are_never_rechecked(self):
        """`extra_run_record()` is a local pass's record — the pass already ran
        on this machine, so its survivors really were missed: nothing
        rechecks them, whatever the machine."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(
                remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}]
            )
            record = self.extra_record(
                root, [self.ELEVATION_SURVIVOR], {"missed": 1}, remote=False
            )
            calls = []

            def run_crate(crate, kinds, tree=None, only=None):
                calls.append(crate)
                return [], Counter(), 0.0

            with self.windows_recheck(run_crate):
                code, out = self.summarize(
                    root, local_half=False, extra_run=record
                )
            self.assertEqual(code, 2)
            self.assertEqual(calls, [])
            self.assertEqual(
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines(),
                [self.ELEVATION_SURVIVOR],
            )
        self.assertNotIn("not rechecked", out)

    def test_an_untested_mutant_of_the_extra_recheck_fails_the_run(self):
        """As the extra pass's own `untested` count does: a `[untested]`
        return is not a catch (the line stays missed) and the run exits 1."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(
                remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}]
            )
            record = self.extra_record(root, [self.ELEVATION_SURVIVOR], {"missed": 1})
            untested_line = self.ELEVATION_SURVIVOR.replace(
                ": [call]", ": [untested] [call]"
            )

            def run_crate(crate, kinds, tree=None, only=None):
                return [untested_line], Counter(untested=1), 0.5

            with self.windows_recheck(run_crate):
                code, _ = self.summarize(root, local_half=False, extra_run=record)
            self.assertEqual(code, 1)
            self.assertEqual(
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines(),
                [self.ELEVATION_SURVIVOR],
            )

    def recheck_elevation(self, run_crate) -> tuple[int, str, list[str]]:
        """A fetched record whose one survivor is elevation's, rechecked through
        `run_crate`: the summary's code, its output and `missed_all.txt`."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(
                remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}]
            )
            record = self.extra_record(root, [self.ELEVATION_SURVIVOR], {"missed": 1})
            with self.windows_recheck(run_crate):
                code, out = self.summarize(root, local_half=False, extra_run=record)
            missed = (
                (root / "mutants.out" / "missed_all.txt")
                .read_text(encoding="utf-8")
                .splitlines()
            )
        return code, out, missed

    def test_a_survivor_the_extra_recheck_never_ran_is_not_caught(self):
        """A line no listed mutant matches runs nothing and is in no list the
        recheck returns: it stays missed and the run exits 1, not caught."""
        code, out, missed = self.recheck_elevation(
            lambda crate, kinds, tree=None, only=None: ([], Counter(), 0.5)
        )
        self.assertEqual(code, 1)
        self.assertEqual(missed, [self.ELEVATION_SURVIVOR])
        self.assertIn("the recheck ran 0 of elevation's 1 survivors", out)

    def test_an_unviable_extra_recheck_is_not_a_catch(self):
        """A rerun that does not compile was caught by no test: the line stays
        missed (the run exits 2, as for any survivor)."""
        code, out, missed = self.recheck_elevation(
            lambda crate, kinds, tree=None, only=None: ([], Counter(unviable=1), 0.5)
        )
        self.assertEqual(code, 2)
        self.assertEqual(missed, [self.ELEVATION_SURVIVOR])
        self.assertIn("found 1 of elevation's survivors unviable", out)

    def test_a_timed_out_extra_recheck_stays_missed_and_tested(self):
        """A rerun that times out is not a catch, nor an untested mutant: the
        line stays missed and the run exits 2, not 1."""
        timed_out = self.ELEVATION_SURVIVOR.replace(": [call]", ": [timeout] [call]")
        code, _, missed = self.recheck_elevation(
            lambda crate, kinds, tree=None, only=None: (
                [timed_out], Counter(timeout=1), 0.5
            )
        )
        self.assertEqual(code, 2)
        self.assertEqual(missed, [self.ELEVATION_SURVIVOR])

    def test_a_failed_extra_recheck_leaves_the_summary_whole(self):
        """A rerun that cannot start (its baseline fails, the lister does not
        build: both `SystemExit`) is reported, the line stays missed and the
        summary still writes `missed_all.txt`: the run is already collected."""
        def run_crate(crate, kinds, tree=None, only=None):
            raise SystemExit("extra_mutants: the baseline failed")

        code, out, missed = self.recheck_elevation(run_crate)
        self.assertEqual(code, 2)
        self.assertEqual(missed, [self.ELEVATION_SURVIVOR])
        self.assertIn("the recheck of elevation did not finish", out)

    def test_the_summary_says_whether_the_run_selected_tests(self):
        """A whole run's summary reports the coverage map's logged exit: "on"
        at 0, "off" (the full suite ran) at any other code; a console without
        the line (a diff run's) prints nothing."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            remote = root / "mutants.out" / "remote" / "mutants.out"
            write_run(remote, 1, [{"scenario": mutant("r"), "summary": "CaughtMutant"}])
            console = root / "remote_console.txt"
            console.write_text("coverage map exit 0\n", encoding="utf-8")
            _, out = self.summarize(root, local_half=False)
            self.assertIn("test selection: on", out)
            console.write_text("coverage map exit 1\n", encoding="utf-8")
            _, out = self.summarize(root, local_half=False)
            self.assertIn(
                "test selection: off (the coverage map failed, exit 1; "
                "every mutant ran the full suite)",
                out,
            )
            self.assertNotIn("test selection: on", out)
            console.write_text("extra pass exit 0\n", encoding="utf-8")
            _, out = self.summarize(root, local_half=False)
            self.assertNotIn("test selection", out)


class RecheckWindows(unittest.TestCase):
    """The Windows recheck of a remote half's missed mutants: a mutant counts as
    caught here under the same rule as everywhere else — its nextest run failed
    tests, not merely exited nonzero (cargo-mutants files any nonzero exit in
    `caught.txt`)."""

    def test_a_catch_whose_tests_never_failed_leaves_the_mutant_missed(self):
        missed = [
            "crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0",
            "crates/libs/cpk/src/read.rs:20:9: replace read -> u8 with 0",
        ]

        def local_mutants(args, source, config=None):
            out = Path(args[args.index("--output") + 1])
            write_run(
                out / "mutants.out", len(missed),
                [
                    caught_outcome(missed[0], {"Failure": 100}),
                    # Nextest's exit when it cannot list the tests: none ran.
                    caught_outcome(missed[1], {"Failure": 104}),
                ],
            )
            return subprocess.CompletedProcess(args, 0)

        def git(*args, **kwargs):
            return subprocess.CompletedProcess(args, 0, stdout="sometree")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = io.StringIO()
            with (
                mock.patch.multiple(
                    mutants,
                    ROOT=root,
                    git=git,
                    local_mutants=local_mutants,
                    mutant_crate=lambda file: "cpk",
                    snapshot_source=lambda tree: contextlib.nullcontext(root),
                ),
                contextlib.redirect_stdout(out),
            ):
                caught = mutants.recheck_windows(missed)
                code = mutants.report_killed(0, local_half=False, rechecked=True)
        self.assertEqual(caught, {missed[0]})
        self.assertEqual(code, 1)
        self.assertIn("1 mutants were NOT tested", out.getvalue())
        self.assertIn(missed[1], out.getvalue())

    def test_a_stale_recheck_folder_counts_nothing_when_no_recheck_ran(self):
        """An earlier run's `mutants.out/recheck` is not this run's: a summary
        that rechecked nothing (a collect's remote-only run, an off-Windows
        one) leaves a clean code alone even when the stale folder holds a
        killed build."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_run(
                root / "mutants.out" / "recheck" / "mutants.out", 1,
                [caught_outcome("stale", {"Failure": 104})],
            )
            out = io.StringIO()
            with (
                mock.patch.object(mutants, "ROOT", root),
                contextlib.redirect_stdout(out),
            ):
                code = mutants.report_killed(0, local_half=False)
        self.assertEqual(code, 0)
        self.assertNotIn("NOT tested", out.getvalue())

    def test_a_failed_recheck_leaves_no_stale_results(self):
        """A recheck that dies before cargo-mutants writes its own results must
        not read an earlier recheck's: `recheck_windows` clears the folder
        first, so the failed run catches nothing and lists no killed build."""
        missed = ["crates/libs/cpk/src/lib.rs:10:5: replace pad -> usize with 0"]

        def local_mutants(args, source, config=None):
            return subprocess.CompletedProcess(args, 1)

        def git(*args, **kwargs):
            return subprocess.CompletedProcess(args, 0, stdout="sometree")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # An earlier recheck's folder: a catch of this survivor and a
            # killed build, which this run must not inherit.
            write_run(
                root / "mutants.out" / "recheck" / "mutants.out", 2,
                [
                    caught_outcome(missed[0], {"Failure": 100}),
                    caught_outcome("killed", {"Failure": 104}),
                ],
            )
            out = io.StringIO()
            with (
                mock.patch.multiple(
                    mutants,
                    ROOT=root,
                    git=git,
                    local_mutants=local_mutants,
                    mutant_crate=lambda file: "cpk",
                    snapshot_source=lambda tree: contextlib.nullcontext(root),
                ),
                contextlib.redirect_stdout(out),
            ):
                caught = mutants.recheck_windows(missed)
                code = mutants.report_killed(0, local_half=False, rechecked=True)
        self.assertEqual(caught, set())
        self.assertEqual(code, 0)

    def test_windows_only_extra_matches_its_files_untagged_lines_alone(self):
        """Each `WINDOWS_ONLY_EXTRA` entry matches a missed line of its file
        and functions (`path:line: [kind] in <fn>:`) — not the same line
        tagged `[timeout]` or `[untested]` (only missed lines are rechecked),
        a kind the pass does not run (`guard`), another file's line, or a
        function the entry does not name; elevation's `windows.rs`, which is
        `#[cfg(windows)]` whole, matching under any place at all. Each file
        still defines its functions, and cargo-mutants' line for each is in
        `WINDOWS_ONLY_TESTS` (elevation's aside): both lists once named two
        files their functions had left, so they matched nothing."""
        cases = [
            # (file, functions, whether any place in the file matches)
            ("crates/libs/elevation/src/windows.rs",
             ["relaunch_elevated", "is_elevated", "wide_parameters"], True),
            ("crates/tools/team_compiler/src/output/deploy.rs",
             ["sweep_run", "probe_download", "promote_tree"], False),
            ("crates/studio_core/src/pes_process.rs",
             ["processes", "image_path", "probe"], False),
            ("crates/tools/export_upgrader/src/staging.rs",
             ["retry_while_locked"], False),
            ("crates/studio_core/src/updater/install.rs",
             ["remove_temp", "remove_temp_folder"], False),
            ("crates/studio_core/src/redact.rs",
             ["redact_any_spelling", "redact_home"], False),
            ("crates/studio_core/src/shell/taskbar.rs",
             ["set_relaunch", "set_relaunch_values"], False),
        ]
        self.assertEqual(len(mutants.WINDOWS_ONLY_EXTRA), len(cases))
        # The other direction: a `WINDOWS_ONLY_TESTS` entry with no case here
        # would have no extra-kind twin.
        for pattern in mutants.WINDOWS_ONLY_TESTS:
            self.assertTrue(
                any(
                    pattern.search(f"{file}:9:5: replace x -> bool with true in {function}")
                    for file, functions, whole in cases
                    if not whole
                    for function in functions
                ),
                pattern.pattern,
            )
        for pattern, (file, functions, whole) in zip(
            mutants.WINDOWS_ONLY_EXTRA, cases
        ):
            source = (REPOSITORY / file).read_text(encoding="utf-8")
            for function in functions:
                self.assertIn(f"fn {function}(", source, file)
                if not whole:
                    cargo_line = f"{file}:9:5: replace x -> bool with true in {function}"
                    self.assertTrue(
                        any(p.search(cargo_line) for p in mutants.WINDOWS_ONLY_TESTS),
                        cargo_line,
                    )
                line = (
                    f"{file}:9: [call] in {function}: the push() statement is "
                    "deleted | removed: x.push(0);"
                )
                self.assertTrue(pattern.search(line), line)
                for tag in ("timeout", "untested"):
                    tagged = line.replace(": [call]", f": [{tag}] [call]")
                    self.assertFalse(pattern.search(tagged), tagged)
                guarded = line.replace("[call]", "[guard]")
                self.assertFalse(pattern.search(guarded), guarded)
            other = (
                f"crates/libs/cpk/src/lib.rs:9: [call] in {functions[0]}: "
                "x | removed: x();"
            )
            self.assertFalse(pattern.search(other), other)
            unnamed = (
                f"{file}:9: [call] at top level: x | removed: x();"
                if whole
                else f"{file}:9: [call] in unrelated_function: x | removed: x();"
            )
            self.assertEqual(bool(pattern.search(unnamed)), whole, unnamed)


class DiffRun(unittest.TestCase):
    """`mutants_diff.run`'s filing of each crate's results, `run_crate` stubbed."""

    LISTING = json.dumps([
        {"file": "crates/libs/cpk/src/lib.rs", "name": "a"},
        {"file": "crates/libs/fpc/src/lib.rs", "name": "b"},
    ]).encode()

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.results = self.root / "mutants.diff"
        for patcher in (
            mock.patch.object(mutants_diff, "ROOT", self.root),
            mock.patch.object(mutants_diff, "RESULTS", self.results),
            mock.patch.object(mutants, "mutant_crate", lambda file: file.split("/")[2]),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)

    def run_diff(self, run_crate, listing: bytes | None = None) -> tuple[int, str]:
        listing = self.LISTING if listing is None else listing
        real_run = subprocess.run

        def run(args, **kwargs):
            if args[:2] == ["cargo", "mutants"]:
                return subprocess.CompletedProcess(args, 0, stdout=listing)
            return real_run(args, **kwargs)

        out = io.StringIO()
        with (
            mock.patch.object(subprocess, "run", run),
            mock.patch.object(mutants_diff, "run_crate", run_crate),
            contextlib.redirect_stdout(out),
        ):
            code = mutants_diff.run("in.diff", HEAD_TREE, self.root)
        return code, out.getvalue()

    def keep(self, folder: Path, text: str) -> None:
        folder.mkdir()
        (folder / "missed_all.txt").write_text(text, encoding="utf-8")

    def test_a_refused_crate_files_nothing_and_a_run_ones_survivors_and_timeouts_merge(self):
        # The local results of an uncollected remote half, which no crate of this run owns.
        self.keep(self.root / "mutants.out", "an uncollected half's survivor\n")

        def run_crate(diff_path, tree, source, crate, count):
            if crate == "cpk":
                raise mutants.Refused("the remote half launched on bonfire is not collected")
            mutants.set_aside(self.root / "mutants.out")
            out = self.root / "mutants.out"
            out.mkdir()
            (out / "missed.txt").write_text(
                "crates/libs/fpc/src/lib.rs:1:5: replace len -> usize with 1\n", encoding="utf-8"
            )
            (out / "timeout.txt").write_text(
                "crates/libs/fpc/src/lib.rs:2:9: replace + with * in len\n", encoding="utf-8"
            )
            return 3

        code, out = self.run_diff(run_crate)
        # A refused crate's mutants went untested: its 1 outranks the other's 3.
        self.assertEqual(code, 1)
        self.assertIn("mutants_diff: the remote half launched on bonfire is not collected", out)
        self.assertFalse((self.results / "cpk").exists(), "a refused crate files no results")
        self.assertEqual(
            (self.results / "missed_all.txt").read_text(encoding="utf-8").splitlines(),
            [
                "crates/libs/fpc/src/lib.rs:1:5: replace len -> usize with 1",
                "crates/libs/fpc/src/lib.rs:2:9: [timeout] replace + with * in len",
            ],
        )

    def test_a_run_that_lands_no_crate_keeps_the_last_results_through_the_next_one(self):
        self.keep(self.results, "the last run's survivor\n")

        def refused(*_):
            raise mutants.Refused("`just gates-remote` is running on bonfire")

        for _ in range(2):
            code, _ = self.run_diff(refused)
            self.assertEqual(code, 1)
            self.assertFalse(self.results.exists())
            self.assertEqual(
                (self.root / "mutants.diff.old" / "missed_all.txt").read_text(encoding="utf-8"),
                "the last run's survivor\n",
            )

    def test_a_diff_with_no_mutants_sets_the_last_results_aside(self):
        self.keep(self.results, "the last run's survivor\n")
        code, _ = self.run_diff(lambda *_: self.fail("no crate to run"), listing=b"")
        self.assertEqual(code, 0)
        self.assertFalse(self.results.exists())
        self.assertTrue((self.root / "mutants.diff.old" / "missed_all.txt").exists())

    def test_an_empty_folder_is_never_kept_over_the_last_results(self):
        self.results.mkdir()
        self.keep(self.root / "mutants.diff.old", "kept\n")
        mutants.set_aside(self.results)
        self.assertFalse(self.results.exists())
        self.assertEqual(
            (self.root / "mutants.diff.old" / "missed_all.txt").read_text(encoding="utf-8"),
            "kept\n",
        )


class DiffBusy(unittest.TestCase):
    """A diff crate run asks the host before splitting: busy or unreachable
    (a whole run holds it for days) it runs here whole; `REMOTE_ONLY` is
    unchanged."""

    def run_crate(self, state: str | None, remote_only: bool = False):
        calls = {}
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "estimate_seconds", lambda crates, share=1.0: 999.0),
            mock.patch.object(mutants, "remote_host", lambda: "bonfire"),
            mock.patch.object(mutants, "REMOTE_ONLY", remote_only),
            mock.patch.object(
                mutants, "remote_run_state",
                lambda host: calls.__setitem__("state", host) or state,
            ),
            mock.patch.object(
                mutants, "split",
                lambda *a, **k: calls.__setitem__("split", (a, k)) or 0,
            ),
            mock.patch.object(
                mutants, "local_refusal",
                lambda: calls.__setitem__("refusal", True) or None,
            ),
            mock.patch.object(mutants, "set_aside_last_run", lambda: None),
            mock.patch.object(
                mutants, "local_mutants",
                lambda *a, **k: calls.__setitem__("local", a)
                or subprocess.CompletedProcess(a, 0),
            ),
            mock.patch.object(mutants, "update_cost_cache", lambda *a, **k: None),
            mock.patch.object(mutants, "report_killed", lambda code, **k: code),
            contextlib.redirect_stdout(out),
        ):
            code = mutants_diff.run_crate("in.diff", HEAD_TREE, Path("src"), "cpk", 3)
        return code, calls, out.getvalue()

    def test_a_busy_remote_runs_the_crate_here_whole(self):
        for state, word in (
            ("running", "running"), ("finished", "finished"),
            ("died", "died"), (None, "unreachable"),
        ):
            code, calls, out = self.run_crate(state)
            self.assertEqual(code, 0)
            self.assertNotIn("split", calls)
            self.assertIn("local", calls)
            self.assertIn("refusal", calls, "local_refusal still runs first")
            self.assertIn(
                f"bonfire is busy (a remote half {word}): running here", out
            )

    def test_a_free_remote_splits_as_today(self):
        code, calls, out = self.run_crate("none")
        self.assertEqual(code, 0)
        self.assertIn("split", calls)
        self.assertNotIn("local", calls)
        self.assertIn("split, about", out)

    def test_remote_only_still_sends_every_mutant_remote(self):
        code, calls, out = self.run_crate("running", remote_only=True)
        self.assertEqual(code, 0)
        self.assertNotIn("state", calls, "REMOTE_ONLY never asks the host's state")
        self.assertEqual(calls["split"][1], {"local": False})


class Fetch(unittest.TestCase):
    """`fetch_remote` and `finish_extra_run` with ssh stubbed: the archive
    arrives on the stdout the stub's tar call is given."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.fetched = self.root / "mutants.out" / "remote"
        for patcher in (
            mock.patch.object(mutants, "ROOT", self.root),
            mock.patch.object(mutants, "LOCAL_TMPDIR", self.root / "tmp"),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)
        self.calls = []

    def ssh_stub(
        self, code: int, members: dict[str, str] | None, extra_present: bool = False
    ):
        """An ssh fake whose tar call writes `members` as a tar archive to the
        stdout it is given and exits `code`, whose `[ -d extra_mutants.out ]`
        probe answers `extra_present`, and whose other calls answer a log or
        two blank lines."""
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode="w") as tar:
            for name, text in (members or {}).items():
                data = text.encode()
                info = tarfile.TarInfo(name)
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))

        def ssh(host, remote, **kwargs):
            self.calls.append(remote)
            if "tar -cf" in remote:
                self.listed_by = kwargs.get("input")
                if code == 0:
                    kwargs["stdout"].write(archive.getvalue())
                return subprocess.CompletedProcess([host, remote], code, stderr=b"")
            if "extra_mutants.out" in remote:
                return subprocess.CompletedProcess(
                    [host, remote], 0 if extra_present else 1
                )
            text = kwargs.get("text")
            return subprocess.CompletedProcess(
                [host, remote], 0, stdout="\n\n" if text else b"the remote log"
            )

        return ssh

    def fetch(
        self, code: int, members: dict[str, str] | None = None,
        partial: bool = False, extra_present: bool = False,
    ):
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "ssh", self.ssh_stub(code, members, extra_present)),
            contextlib.redirect_stdout(out),
        ):
            mutants.fetch_remote("bonfire", partial=partial)
        self.printed = out.getvalue()

    def finish(
        self, code: int, members: dict[str, str] | None = None, pass_code: int = 0,
        partial: bool = False,
    ) -> int:
        """`finish_extra_run` through the same stub; `code` is the tar call's
        exit, `pass_code` the remote pass's own."""
        out, err = io.StringIO(), io.StringIO()
        with (
            mock.patch.object(mutants, "ssh", self.ssh_stub(code, members)),
            contextlib.redirect_stdout(out),
            contextlib.redirect_stderr(err),
        ):
            result = mutants.finish_extra_run("bonfire", pass_code, partial=partial)
        self.printed = out.getvalue()
        self.printed_err = err.getvalue()
        return result

    def stale(self, name: str = "mutants.out") -> Path:
        """A leftover file inside the last fetch of `name` (`mutants.out` or
        `extra_mutants.out`), which no fetch of this run writes: kept, it
        would read as this run's results."""
        stale = self.root / name / "remote" / name / "stale.txt"
        stale.parent.mkdir(parents=True)
        stale.write_text("an earlier fetch's line\n", encoding="utf-8")
        # A test's scratch output can leave a read-only file, which plain rmtree refuses.
        os.chmod(stale, stat.S_IREAD)
        return stale

    def test_a_fetch_replaces_the_last_fetch_whole(self):
        stale = self.stale()
        self.fetch(0, {"mutants.out/missed.txt": "crates/libs/cpk/src/lib.rs:1:1: x\n"})
        self.assertFalse(stale.exists())
        self.assertEqual(
            (self.fetched / "mutants.out" / "missed.txt").read_text(encoding="utf-8"),
            "crates/libs/cpk/src/lib.rs:1:1: x\n",
        )
        self.assertEqual((self.fetched / "remote_console.txt").read_bytes(), b"the remote log")
        self.assertTrue(any("collected" in call for call in self.calls))
        # The host's tar packs what the list program prints, and a failing program fails it.
        self.assertEqual(self.listed_by, mutants.REMOTE_FETCH_LIST.encode())
        self.assertTrue(any("pipefail" in call and "-T -" in call for call in self.calls))

    def listed(self, outcomes: str | None) -> set[str]:
        """What `REMOTE_FETCH_LIST` prints over a `mutants.out` holding `outcomes` as its
        outcomes.json (None: no such file), three mutant logs and a summary list."""
        clone = self.root / "clone"
        logs = clone / "mutants.out" / "log"
        logs.mkdir(parents=True, exist_ok=True)
        for name in ("caught.log", "unviable.log", "missed.log", "untested.log"):
            (logs / name).write_text("log\n", encoding="utf-8")
        (clone / "mutants.out" / "caught.txt").write_text("x\n", encoding="utf-8")
        if outcomes is None:
            (clone / "mutants.out" / "outcomes.json").unlink(missing_ok=True)
        else:
            (clone / "mutants.out" / "outcomes.json").write_text(outcomes, encoding="utf-8")
        printed = subprocess.run(
            [sys.executable, "-"], input=mutants.REMOTE_FETCH_LIST, cwd=clone,
            capture_output=True, text=True, check=True,
        ).stdout
        return {Path(line).as_posix() for line in printed.splitlines()}

    def test_the_fetch_list_leaves_out_only_the_caught_mutants_logs(self):
        # Nextest's 104: it could not list the tests, so none failed and the log is kept.
        outcomes = {"outcomes": [
            caught_outcome("caught", {"Failure": 100}),
            caught_outcome("untested", {"Failure": 104}),
            {"summary": "Unviable", "log_path": "log/unviable.log"},
            {"summary": "MissedMutant", "log_path": "log/missed.log"},
        ]}
        self.assertEqual(self.listed(json.dumps(outcomes)), {
            "mutants.out/caught.txt", "mutants.out/outcomes.json",
            "mutants.out/log/unviable.log", "mutants.out/log/missed.log",
            "mutants.out/log/untested.log",
        })

    def test_the_fetch_list_brings_a_whole_runs_extra_pass_home(self):
        """A whole run's extra pass leaves its record under the clone's
        `extra_mutants.out/` — `run.json`, `missed_<crate>.txt`, the outcomes
        tsvs — all small, all fetched, while the caught-log filter still holds."""
        outcomes = {"outcomes": [caught_outcome("caught", {"Failure": 100})]}
        extra = self.root / "clone" / "extra_mutants.out"
        extra.mkdir(parents=True)
        (extra / "run.json").write_text("{}\n", encoding="utf-8")
        (extra / "missed_cpk.txt").write_text("x\n", encoding="utf-8")
        (extra / "outcomes_bool.tsv").write_text("caught\tx\n", encoding="utf-8")
        listed = self.listed(json.dumps(outcomes))
        self.assertIn("extra_mutants.out/run.json", listed)
        self.assertIn("extra_mutants.out/missed_cpk.txt", listed)
        self.assertIn("extra_mutants.out/outcomes_bool.tsv", listed)
        self.assertNotIn("mutants.out/log/caught.log", listed)

    def test_an_outcomes_file_it_cannot_read_keeps_every_log(self):
        logs = {
            "mutants.out/caught.txt", "mutants.out/log/caught.log",
            "mutants.out/log/unviable.log", "mutants.out/log/missed.log",
            "mutants.out/log/untested.log",
        }
        self.assertEqual(self.listed(None), logs)
        self.assertEqual(self.listed('{"outcomes": [{"summ'), logs | {"mutants.out/outcomes.json"})
        # Another shape, the first outcome's log already taken as caught before the second
        # fails: the list still keeps it.
        other_shape = json.dumps({"outcomes": [
            caught_outcome("caught", {"Failure": 100}), {"log_path": "x"},
        ]})
        self.assertEqual(self.listed(other_shape), logs | {"mutants.out/outcomes.json"})

    def test_a_finished_half_without_results_fails_and_stays_uncollected(self):
        with self.assertRaisesRegex(RuntimeError, "left no mutants.out"):
            self.fetch(3)
        self.assertFalse(any("collected" in call for call in self.calls))

    def test_an_extras_half_collected_from_the_wrong_tree_is_named(self):
        """The cargo-mutants fetch's "left no mutants.out" tells the user to
        `rm -r run` on the host — which would delete an extras-only half's
        results when a pending record is missing and this tree's collect takes
        the cargo path. The probe for the pass's folder names the run first."""
        with self.assertRaisesRegex(
            RuntimeError, "extras-only run.*collect it from the tree that launched it"
        ) as raised:
            self.fetch(3, extra_present=True)
        self.assertNotIn("rm -r", str(raised.exception))
        self.assertFalse(any("collected" in call for call in self.calls))

    def test_a_failed_transfer_keeps_the_last_fetch_and_the_half_uncollected(self):
        stale = self.stale()
        with self.assertRaisesRegex(RuntimeError, "stays uncollected"):
            self.fetch(255)
        self.assertTrue(stale.exists())
        self.assertFalse(any("collected" in call for call in self.calls))

    EXTRA_RECORD = json.dumps(
        {"crate": "cpk", "seconds": 2.0, "counts": {"caught": 3, "missed": 1}}
    )

    def test_the_finish_fetches_the_pass_counts_and_survivors(self):
        """`finish_extra_run`, an extras-only run's finish: the remote tree's
        `extra_mutants.out` into `extra_mutants.out/remote/` on this machine,
        emptied first, the pass's counts printed from its `run.json` beside its
        survivors file, and `mutants.out` never named."""
        extra = self.root / "extra_mutants.out" / "remote"
        stale = self.stale("extra_mutants.out")
        pending = pend(local_half=False, extra=True, extra_only=True)
        code = self.finish(
            0,
            {
                "extra_mutants.out/run.json": self.EXTRA_RECORD + "\n",
                "extra_mutants.out/missed_cpk.txt": "crates/x:1: [call] drop it\n",
            },
        )
        self.assertEqual(code, 0)
        self.assertFalse(stale.exists(), "the last fetch was not emptied first")
        record = extra / "extra_mutants.out" / "run.json"
        self.assertTrue(record.exists())
        self.assertIn("caught 3", self.printed)
        self.assertIn("missed 1", self.printed)
        self.assertIn(str(record.parent / "missed_cpk.txt"), self.printed)
        self.assertEqual((extra / "remote_console.txt").read_bytes(), b"the remote log")
        (tar_call,) = [call for call in self.calls if "tar -cf" in call]
        self.assertIn("[ -d extra_mutants.out ]", tar_call)
        # Every `mutants.out` the command names is the extra pass's, never the
        # cargo-mutants results an extras-only run does not write.
        self.assertNotIn("mutants.out", tar_call.replace("extra_mutants.out", ""))
        self.assertTrue(any("collected" in call for call in self.calls))
        self.assertFalse(pending.exists())

    def finish_rechecking(self, recheck: tuple[set[str], int]) -> tuple[int, Path]:
        """An extras-only finish on Windows whose survivors file holds
        elevation's `windows.rs` line and a `cpk` one, the recheck answering
        `recheck` (lines caught, untested count): the finish's code and the
        fetched survivors file."""
        pend(local_half=False, extra=True, extra_only=True)
        with (
            mock.patch.object(mutants.os, "name", "nt"),
            mock.patch.object(mutants, "REMOTE_ONLY", False),
            mock.patch.object(
                mutants, "recheck_windows_extra", lambda lines: recheck
            ),
        ):
            code = self.finish(
                0,
                {
                    "extra_mutants.out/run.json": self.EXTRA_RECORD + "\n",
                    "extra_mutants.out/missed_cpk.txt": (
                        SummaryLists.ELEVATION_SURVIVOR + "\ncrates/x:1: [call] drop it\n"
                    ),
                },
            )
        survivors = (
            self.root / "extra_mutants.out" / "remote" / "extra_mutants.out" / "missed_cpk.txt"
        )
        return code, survivors

    def test_the_finish_reruns_windows_only_survivors_here(self):
        """An extras-only run (a backfill's) gets the whole run's Windows
        recheck: a catch leaves the survivors file and moves the counts."""
        code, survivors = self.finish_rechecking(({SummaryLists.ELEVATION_SURVIVOR}, 0))
        self.assertEqual(code, 0)
        self.assertEqual(
            survivors.read_text(encoding="utf-8"), "crates/x:1: [call] drop it\n"
        )
        self.assertIn("caught 4", self.printed)
        self.assertIn("missed 0", self.printed)

    def test_an_untested_rerun_fails_the_extras_only_finish(self):
        code, survivors = self.finish_rechecking((set(), 1))
        self.assertEqual(code, 1)
        self.assertIn(SummaryLists.ELEVATION_SURVIVOR, survivors.read_text(encoding="utf-8"))

    def test_a_failed_fetch_leaves_the_half_uncollected_and_the_record(self):
        stale = self.stale("extra_mutants.out")
        pending = pend(local_half=False, extra=True, extra_only=True)
        with self.assertRaisesRegex(RuntimeError, "stays uncollected"):
            self.finish(255)
        self.assertTrue(stale.exists())
        self.assertTrue(pending.exists())
        self.assertFalse(any("collected" in call for call in self.calls))

    def test_a_finished_half_without_the_folder_fails_and_stays_uncollected(self):
        pending = pend(local_half=False, extra=True, extra_only=True)
        with self.assertRaisesRegex(RuntimeError, "left no extra_mutants.out"):
            self.finish(3)
        self.assertTrue(pending.exists())
        self.assertFalse(any("collected" in call for call in self.calls))

    def test_a_died_extras_half_fetches_partial_and_fails_for_the_record(self):
        """A died extras-only half may leave no `extra_mutants.out`: the
        partial fetch warns rather than fails, the half is collected, and the
        finish reports the missing record."""
        code = self.finish(3, partial=True)
        self.assertEqual(code, 1)
        self.assertIn("left no extra_mutants.out", self.printed_err)
        self.assertIn("left no record", self.printed)
        self.assertTrue(any("collected" in call for call in self.calls))

    def test_a_fetched_pass_without_its_record_fails(self):
        code = self.finish(0, {"extra_mutants.out/missed_cpk.txt": "x\n"})
        self.assertEqual(code, 1)
        self.assertIn("left no record", self.printed)

    def test_a_failed_pass_fails_the_finish(self):
        code = self.finish(
            0, {"extra_mutants.out/run.json": self.EXTRA_RECORD}, pass_code=1
        )
        self.assertEqual(code, 1)


LOCAL_SURVIVOR = "crates/libs/cpk/src/lib.rs:1:1: a stale local survivor"
REMOTE_SURVIVOR = "crates/libs/cpk/src/read.rs:2:2: a remote survivor"
REMOTE_EXTRA_SURVIVOR = "crates/libs/cpk/src/lib.rs:5: [bool] flip it"


class CollectRemote(unittest.TestCase):
    """`just mutants-collect`: the wait on the remote half holds no run lock,
    the fetch and summary take it, and the launch record decides whether the
    run had a local half and ran the extra pass."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(mutants, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_the_wait_holds_no_run_lock(self):
        with (
            lock_probes(self.root) as probe,
            mock.patch.object(mutants, "remote_run_state", lambda host: "finished"),
            mock.patch.object(
                mutants, "wait_remote", lambda host: probe("wait") or (0, 1.0)
            ),
            mock.patch.object(
                mutants, "fetch_remote", lambda *a, **k: probe("fetch")
            ),
            mock.patch.object(mutants, "update_cost_cache", lambda *a, **k: None),
            mock.patch.object(mutants, "summarize", lambda *a, **k: 0),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.collect("bonfire"), 0)
        self.assertEqual(probe.calls, [("wait", True), ("fetch", False)])

    def test_a_record_with_no_local_half_and_the_extra_pass_summarizes_as_a_whole_run(self):
        """A whole run's record says there was no local half and the remote ran
        the extra pass: the summary reads the fetched `mutants.out` and the
        fetched `run.json` alone — a stale local `mutants.out` and the local
        cost table are left untouched."""
        pend(local_half=False, extra=True)
        self.stale_local_run()
        code, missed, costs = self.collect_fetching(extra=True)
        self.assertEqual(code, 2)
        self.assertEqual(missed, [REMOTE_SURVIVOR, REMOTE_EXTRA_SURVIVOR])
        # The local `mutant` and `baseline` cost entries stay as they were: only
        # the fetched half folds into `remote_mutant`.
        self.assertEqual(
            costs, [(self.root / "mutants.out" / "remote" / "mutants.out", "remote_mutant")]
        )

    def test_an_extra_only_record_takes_the_extra_finish(self):
        """A collect of an extras-only half fetches the extra pass's results
        and prints its counts, not the cargo-mutants summary of a `mutants.out`
        the run never wrote. The pass's code is the run's."""
        pend(local_half=False, extra=True, extra_only=True)
        finished = {}
        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: "finished"),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(mutants, "wait_remote", lambda host: (2, 9.0)),
            mock.patch.object(
                mutants, "finish_remote_run",
                lambda *a, **k: self.fail("the cargo-mutants finish ran"),
            ),
            mock.patch.object(
                mutants, "finish_extra_run",
                lambda *a, **k: finished.update(args=a, kwargs=k) or 1,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.collect("bonfire"), 1)
        self.assertEqual(finished["args"], ("bonfire", 2))
        self.assertEqual(finished["kwargs"], {"partial": False})

    def test_an_extra_only_died_half_collects_partial(self):
        """A died extras-only half's collect passes `partial` down to the
        extras-only finish, which fetches what the pass left."""
        pend(local_half=False, extra=True, extra_only=True)
        finished = {}
        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: "died"),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(
                mutants, "wait_remote", lambda host: self.fail("no wait on a died half")
            ),
            mock.patch.object(
                mutants, "finish_remote_run",
                lambda *a, **k: self.fail("the cargo-mutants finish ran"),
            ),
            mock.patch.object(
                mutants, "finish_extra_run",
                lambda *a, **k: finished.update(args=a, kwargs=k) or 1,
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            self.assertEqual(mutants.collect("bonfire"), 1)
        self.assertEqual(finished["args"], ("bonfire", 1))
        self.assertEqual(finished["kwargs"], {"partial": True})

    def test_with_no_record_the_remote_half_is_summarized_alone(self):
        """A half launched from another tree: this tree's `mutants.out`, extra
        record and split record are some earlier run's, and the fetched extra
        record is the only sign the half ran the extra pass."""
        self.stale_local_run()
        code, missed, costs = self.collect_fetching(extra=True)
        self.assertEqual(code, 2)
        self.assertEqual(missed, [REMOTE_SURVIVOR, REMOTE_EXTRA_SURVIVOR])
        self.assertEqual(
            costs, [(self.root / "mutants.out" / "remote" / "mutants.out", "remote_mutant")]
        )

    def test_a_half_that_selected_its_tests_leaves_the_remote_costs(self):
        """A diff run's split reads the remote costs, and its halves run the
        full suite: a whole run's selected tests would make the remote look
        faster than it is. A failed map ran the full suite, and is folded."""
        remote = (self.root / "mutants.out" / "remote" / "mutants.out", "remote_mutant")
        _, _, costs = self.collect_fetching(extra=False, console="coverage map exit 0\n")
        self.assertEqual(costs, [])
        _, _, costs = self.collect_fetching(extra=False, console="coverage map exit 1\n")
        self.assertEqual(costs, [remote])

    def test_with_no_record_a_half_that_ran_no_extra_pass_is_not_failed_for_it(self):
        """A diff run's half launched from another tree brings no extra record:
        no record here says it should have."""
        code, missed, _ = self.collect_fetching(extra=False)
        self.assertEqual(code, 2)
        self.assertEqual(missed, [REMOTE_SURVIVOR])
        self.assertNotIn("left no record", self.printed)

    def test_a_record_this_version_cannot_read_keeps_the_local_half(self):
        """The record an earlier version wrote held the host's name alone: the
        run it stands for was a split launched from this tree."""
        pending = mutants.remote_pending()
        pending.parent.mkdir(parents=True, exist_ok=True)
        pending.write_text("bonfire\n", encoding="utf-8")
        local = self.root / "mutants.out"
        write_run(local, 1, [{"scenario": mutant("l"), "summary": "MissedMutant"}])
        (local / "missed.txt").write_text(LOCAL_SURVIVOR + "\n", encoding="utf-8")
        _, missed, _ = self.collect_fetching(extra=False)
        self.assertEqual(missed, [LOCAL_SURVIVOR, REMOTE_SURVIVOR])

    def stale_local_run(self) -> None:
        """A local `mutants.out` with a survivor, a local extra record with
        another and a split record naming a mutant no half ran: what a run this
        tree made earlier leaves behind."""
        local = self.root / "mutants.out"
        write_run(local, 1, [{"scenario": mutant("l"), "summary": "MissedMutant"}])
        (local / "missed.txt").write_text(LOCAL_SURVIVOR + "\n", encoding="utf-8")
        extra = self.root / "extra_mutants.out"
        extra.mkdir()
        (extra / "run.json").write_text(
            json.dumps({"crate": "cpk", "seconds": 2.0, "counts": {"missed": 1}}),
            encoding="utf-8",
        )
        (extra / "missed_cpk.txt").write_text(
            "crates/libs/cpk/src/lib.rs:9: [bool] a stale extra survivor\n", encoding="utf-8"
        )
        (extra / "split.json").write_text(
            json.dumps({"names": ["a mutant of an earlier split"]}), encoding="utf-8"
        )

    def collect_fetching(
        self, *, extra: bool, console: str = "log\n"
    ) -> tuple[int, list[str], list]:
        """Runs `collect` over a finished half whose fetch brings one survivor,
        the job log `console` and, with `extra`, the extra pass's record with
        one more; returns the exit code, `missed_all.txt`'s lines and the
        cost-table updates, and keeps what it printed in `self.printed`."""
        remote = self.root / "mutants.out" / "remote"
        fetched = remote / "mutants.out"

        def fetch(host, partial=False):
            write_run(fetched, 1, [{"scenario": mutant("r"), "summary": "MissedMutant"}])
            (fetched / "missed.txt").write_text(REMOTE_SURVIVOR + "\n", encoding="utf-8")
            if extra:
                folder = remote / "extra_mutants.out"
                folder.mkdir(parents=True)
                (folder / "run.json").write_text(
                    json.dumps({"crate": "cpk", "seconds": 2.0, "counts": {"missed": 1}}),
                    encoding="utf-8",
                )
                (folder / "missed_cpk.txt").write_text(
                    REMOTE_EXTRA_SURVIVOR + "\n", encoding="utf-8"
                )
            (remote / "remote_console.txt").write_text(console, encoding="utf-8")

        costs = []
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: "finished"),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(mutants, "wait_remote", lambda host: (2, 9.0)),
            mock.patch.object(mutants, "fetch_remote", fetch),
            mock.patch.object(
                mutants, "update_cost_cache", lambda *a, **k: costs.append(a or k)
            ),
            contextlib.redirect_stdout(out),
        ):
            code = mutants.collect("bonfire")
        self.printed = out.getvalue()
        missed = (self.root / "mutants.out" / "missed_all.txt").read_text(encoding="utf-8")
        return code, missed.splitlines(), costs

    def test_a_collect_reports_a_half_already_collected_while_it_waited(self):
        """Its own run fetched the half before this collect's turn at the lock:
        a second fetch would summarize a record that is gone."""
        states = iter(["finished", "none"])
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: next(states)),
            mock.patch.object(mutants, "wait_remote", lambda host: (0, 1.0)),
            mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
            mock.patch.object(
                mutants, "fetch_remote", lambda *a, **k: self.fail("a second fetch")
            ),
            contextlib.redirect_stdout(out),
        ):
            self.assertEqual(mutants.collect("bonfire"), 1)
        self.assertIn("already collected", out.getvalue())


class RemoteRefusal(unittest.TestCase):
    """`remote_refusal` through the host's states, for a run that uses the remote:
    every caller does (`local_refusal` asks this tree's record alone)."""

    def refusal(
        self, state: str | None, gates: int = 3, free: int = 100 * 2**30
    ) -> str | None:
        def ssh(host, command, **kwargs):
            return subprocess.CompletedProcess([host, command], gates)

        with (
            mock.patch.object(mutants, "remote_run_state", lambda host: state),
            mock.patch.object(mutants, "ssh", ssh),
            mock.patch.object(mutants, "remote_disk_use", lambda host: (2**30, free)),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            return mutants.remote_refusal("bonfire")

    def test_an_unreachable_host_stops_the_run(self):
        self.assertEqual(self.refusal(None), "cannot reach bonfire")

    def test_a_half_not_collected_stops_the_run(self):
        for state, words in (
            ("running", "is still running"),
            ("finished", "has finished but is not collected"),
            ("died", "stopped before it finished"),
        ):
            refusal = self.refusal(state)
            self.assertIn(words, refusal)
            self.assertIn("mutants-collect", refusal)

    def test_a_free_host_stops_the_run_only_for_the_gates_the_link_or_its_disk(self):
        self.assertIn("gates-remote", self.refusal("none", gates=0))
        self.assertEqual(self.refusal("none", gates=255), "cannot reach bonfire")
        self.assertIsNone(self.refusal("none", gates=3))
        self.assertIn(
            "below the floor",
            self.refusal("none", gates=3, free=mutants.REMOTE_FREE_FLOOR - 1),
        )


class ExtraRun(unittest.TestCase):
    """`extra_mutants.run_crate` with the lister, the copy and `cargo nextest run` stubbed:
    how each mutant's outcome is decided, and what lands in its files."""

    SOURCE = b"true true true true true true true true true true\n"
    # Per mutant, in turn: the build phase's (exit code, output), the test
    # phase's (None when the build decided it — no test runs), and the outcome
    # the pair must give.
    TESTS = [
        ((0, ""), (None, ""), "timeout"),
        ((0, ""), (0, "        PASS [   0.309s] (13/14) pes_strings "
                    "tests::an_absent_section_or_string_is_none\n"
                    "     Summary [   0.313s] 14 tests run: 14 passed, 0 skipped"),
         "missed"),
        ((101, "error[E0308]: `?` operator has incompatible types\n"
               "error: could not compile `pes_strings` (lib) due to 1 previous error"),
         None, "unviable"),
        ((0, ""), (100, "        FAIL [   0.289s] (11/14) pes_strings "
                       "tests::an_absent_section_or_string_is_none\n"
                       "     Summary [   0.411s] 14 tests run: 13 passed, 1 failed, "
                       "0 skipped\n"
                       "error: test run failed"), "caught"),
        # A test binary killed while its tests ran is caught, not untested: the
        # mutant made the test blow up (as cargo-mutants counts it).
        ((0, ""), (100, "     SIGKILL [   4.120s] ( 9/14) pes_strings "
                       "tests::an_absent_section_or_string_is_none\n"
                       "     Summary [   9.405s] 14 tests run: 13 passed, 1 failed, "
                       "0 skipped"), "caught"),
        ((101, "error: could not compile `cpk` (lib)\nCaused by:\n  process didn't exit "
               "successfully: `rustc --crate-name cpk` (signal: 9, SIGKILL: kill)"),
         None, "untested"),
        # A test binary that lost its console never ran its tests, whatever nextest says.
        ((0, ""), (100, "        ABORT [   0.003s] ( 1/14) pes_strings "
                       "tests::an_absent_section_or_string_is_none\n"
                       "           - with code 0xc0000142: The application failed to "
                       "initialize properly."), "untested"),
        ((101, "error: failed to run custom build command for `cpk v0.1.0`\nCaused by:\n  "
               "No space left on device (os error 28)"), None, "untested"),
        # A test binary that ran out of memory is a catch, as a SIGKILLed one is.
        ((0, ""), (100, "        FAIL [   0.289s] (11/14) pes_strings "
                       "tests::an_absent_section_or_string_is_none\n"
                       "memory allocation of 25769803776 bytes failed\n"
                       "error: test run failed"), "caught"),
        # A test phase that broke before or around the tests is no catch either.
        ((0, ""), (101, "error: failed to run custom build command for `cpk v0.1.0`\n"
                       "Caused by:\n  No space left on device (os error 28)"), "untested"),
    ]

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.out_dir = self.tmp / "extra_mutants.out"
        self.rows = [
            ["src/lib.rs", str(5 * i), str(5 * i + 4), "bool", "1", "false",
             f"flip {i + 1} | removed: true"]
            for i in range(len(self.TESTS))
        ]

        def extract_tree(tree, dest):
            (dest / "crates/libs/cpk/src").mkdir(parents=True)
            (dest / "crates/libs/cpk/src/lib.rs").write_bytes(self.SOURCE)
            (dest / ".cargo").mkdir()
            (dest / ".cargo" / "mutants.toml").write_text(
                "timeout_multiplier = 5.0\nminimum_test_timeout = 150\n",
                encoding="utf-8",
            )

        stale = {"path": "crates/libs/cpk/src/read.rs", "kind": "bool",
                 "description": "flip 11 | removed: true", "why": "an equivalent flip"}
        for patcher in (
            mock.patch.object(mutants, "LOCAL_TMPDIR", self.tmp / "copies"),
            mock.patch.object(mutants, "extract_tree", extract_tree),
            mock.patch.object(extra_mutants, "OUT_DIR", self.out_dir),
            mock.patch.object(extra_mutants, "crate_dir",
                              lambda name, root: (root / "crates/libs/cpk").resolve()),
            mock.patch.object(extra_mutants, "list_mutants", lambda directory: self.rows),
            mock.patch.object(extra_mutants, "exclusions", lambda root: [stale]),
            mock.patch.object(extra_mutants, "baseline_seconds", lambda crate, cwd: 1.0),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)

    def patched_index(self, cwd: Path) -> int:
        """The `self.rows` index of the mutant patched into the copy at `cwd`:
        the stubs learn which mutant a call is for from the patched file
        itself, two jobs calling them at once in no fixed order."""
        data = (cwd / "crates/libs/cpk/src/lib.rs").read_bytes()
        changed = [
            index for index, row in enumerate(self.rows)
            if data[int(row[1]):int(row[1]) + len(row[5])] == row[5].encode()
        ]
        assert len(changed) == 1, changed
        return changed[0]

    def run_crate(self, results, only=None):
        """`results`: per row of `self.rows`, the mutant's (build result, test
        result) pair — the test result None when the build decides it — or an
        exception for `build` to raise; the test result may be a callable
        taking `(crate, cwd, timeout)`."""
        self.timeouts = []
        self.build_cwds = []

        def build(crate, cwd):
            self.build_cwds.append(cwd)
            item = results[self.patched_index(cwd)]
            if isinstance(item, BaseException):
                raise item
            return item[0]

        def run_tests(crate, cwd, timeout):
            self.timeouts.append(timeout)
            test_result = results[self.patched_index(cwd)][1]
            if callable(test_result):
                test_result = test_result(crate, cwd, timeout)
            if test_result is None:
                raise AssertionError("a test ran on a mutant whose build decided it")
            return test_result

        with (
            mock.patch.object(extra_mutants, "build", build),
            mock.patch.object(extra_mutants, "run_tests", run_tests),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            return extra_mutants.run_crate("cpk", ["bool"], HEAD_TREE, only=only)

    def test_each_outcome_and_the_lines_triage_reads(self):
        listed, counts, _ = self.run_crate(
            [(build_result, test_result) for build_result, test_result, _ in self.TESTS]
        )
        self.assertEqual(
            counts, {"timeout": 1, "missed": 1, "unviable": 1, "caught": 3, "untested": 4}
        )
        expected = [
            "crates/libs/cpk/src/lib.rs:1: [timeout] [bool] flip 1 | removed: true",
            "crates/libs/cpk/src/lib.rs:1: [bool] flip 2 | removed: true",
            "crates/libs/cpk/src/lib.rs:1: [untested] [bool] flip 6 | removed: true",
            "crates/libs/cpk/src/lib.rs:1: [untested] [bool] flip 7 | removed: true",
            "crates/libs/cpk/src/lib.rs:1: [untested] [bool] flip 8 | removed: true",
            "crates/libs/cpk/src/lib.rs:1: [untested] [bool] flip 10 | removed: true",
            "crates/libs/cpk/src/read.rs: [bool] stale exclusion: flip 11 | removed: true",
        ]
        self.assertEqual(listed, expected)
        for name in ("missed_bool.txt", "missed_cpk.txt"):
            self.assertEqual(
                (self.out_dir / name).read_text(encoding="utf-8").splitlines(), expected
            )
        # The tsv lists each outcome as it is decided, so its order is the
        # two jobs' finish order, not the listing's.
        self.assertCountEqual(
            (self.out_dir / "outcomes_bool.tsv").read_text(encoding="utf-8").splitlines(),
            [
                outcome + "\t" + "\t".join(row)
                for row, (_, _, outcome) in zip(self.rows, self.TESTS)
            ],
        )

    def test_only_runs_the_mutants_whose_missed_line_it_holds(self):
        """`only` names the missed lines of the mutants to run (the Windows
        recheck's survivors): the rest are never patched or tested."""
        listed, counts, _ = self.run_crate(
            [((0, ""), (0, "test result: ok. 12 passed; 0 failed"))] * len(self.TESTS),
            only={"crates/libs/cpk/src/lib.rs:1: [bool] flip 2 | removed: true"},
        )
        self.assertEqual(
            listed,
            [
                "crates/libs/cpk/src/lib.rs:1: [bool] flip 2 | removed: true",
                "crates/libs/cpk/src/read.rs: [bool] stale exclusion: "
                "flip 11 | removed: true",
            ],
        )
        self.assertEqual(counts, {"missed": 1})

    def test_a_run_stopped_midway_keeps_the_outcomes_it_decided(self):
        tsv = self.out_dir / "outcomes_bool.tsv"

        def second_test(crate, cwd, timeout):
            # Finishes only once the first mutant's outcome was written, so
            # the tsv's order is fixed whatever order the jobs picked them up.
            for _ in range(1200):
                if tsv.exists() and tsv.read_text(encoding="utf-8").strip():
                    return 0, "test result: ok. 12 passed; 0 failed"
                time.sleep(0.1)
            raise AssertionError("the first mutant's outcome was never written")

        only = {
            f"crates/libs/cpk/src/lib.rs:1: [bool] flip {i} | removed: true"
            for i in (1, 2, 3)
        }
        results = [
            ((0, ""), (None, "")),
            ((0, ""), second_test),
            RuntimeError("the PC went down"),
        ]
        with self.assertRaisesRegex(RuntimeError, "the PC went down"):
            self.run_crate(results, only=only)
        self.assertEqual(
            tsv.read_text(encoding="utf-8").splitlines(),
            ["timeout\t" + "\t".join(self.rows[0]), "missed\t" + "\t".join(self.rows[1])],
        )

    def test_the_missed_files_keep_the_listings_order(self):
        """The second mutant's outcome is written to the tsv before the
        first's is decided; the missed files still list them in the
        listing's order."""
        tsv = self.out_dir / "outcomes_bool.tsv"

        def first_test(crate, cwd, timeout):
            for _ in range(1200):
                if tsv.exists() and tsv.read_text(encoding="utf-8").strip():
                    return 0, "test result: ok. 12 passed; 0 failed"
                time.sleep(0.1)
            raise AssertionError("the second mutant's outcome was never written")

        only = {
            f"crates/libs/cpk/src/lib.rs:1: [bool] flip {i} | removed: true"
            for i in (1, 2)
        }
        listed, counts, _ = self.run_crate(
            [((0, ""), first_test), ((0, ""), (0, ""))], only=only
        )
        self.assertEqual(counts, {"missed": 2})
        self.assertEqual(
            listed,
            [
                "crates/libs/cpk/src/lib.rs:1: [bool] flip 1 | removed: true",
                "crates/libs/cpk/src/lib.rs:1: [bool] flip 2 | removed: true",
                "crates/libs/cpk/src/read.rs: [bool] stale exclusion: "
                "flip 11 | removed: true",
            ],
        )

    def test_the_two_jobs_patch_and_build_in_two_copies(self):
        """Each job holds one copy of the tree: the two mutants' `build`s run
        in two different folders, and while both test phases run, a copy's
        patched file holds its own mutant's change only."""
        barrier = threading.Barrier(2)

        def check_isolation(crate, cwd, timeout):
            index = self.patched_index(cwd)
            barrier.wait(timeout=120)
            others = [path for path in cwd.parent.iterdir() if path != cwd]
            self.assertEqual(len(others), 1)
            data = (others[0] / "crates/libs/cpk/src/lib.rs").read_bytes()
            patched = {
                at
                for at, row in enumerate(self.rows)
                if data[int(row[1]):int(row[1]) + len(row[5])] == row[5].encode()
            }
            self.assertEqual(patched, {1 - index})
            return 0, "test result: ok. 12 passed; 0 failed"

        only = {
            f"crates/libs/cpk/src/lib.rs:1: [bool] flip {i} | removed: true"
            for i in (1, 2)
        }
        self.run_crate([((0, ""), check_isolation)] * 2, only=only)
        self.assertEqual(len(set(self.build_cwds)), 2)

    def test_the_mutant_timeout_is_the_configs_on_both_sides_of_the_minimum(self):
        """`run_tests` gets `max(minimum_test_timeout, timeout_multiplier x
        baseline)`, both from the run's copy of `.cargo/mutants.toml`."""
        one = {"crates/libs/cpk/src/lib.rs:1: [bool] flip 1 | removed: true"}
        for baseline, expected in ((40.0, 200.0), (10.0, 150.0)):
            with mock.patch.object(
                extra_mutants, "baseline_seconds", lambda crate, cwd: baseline
            ):
                self.run_crate([((0, ""), (0, ""))], only=one)
            self.assertEqual(self.timeouts, [expected])

    def test_a_config_lacking_a_timeout_key_stops_the_run_naming_it(self):
        with (
            mock.patch.object(
                extra_mutants, "mutants_toml",
                lambda cwd: {"minimum_test_timeout": 150},
            ),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(SystemExit, "timeout_multiplier"),
        ):
            extra_mutants.run_crate("cpk", ["bool"], HEAD_TREE)

    def test_a_standalone_run_with_an_untested_mutant_fails(self):
        for counts, code in ((Counter(caught=3, missed=1), 0), (Counter(caught=3, untested=1), 1)):
            with (
                mock.patch.object(mutants, "working_tree", lambda: HEAD_TREE),
                mock.patch.object(mutants, "run_lock", contextlib.nullcontext),
                mock.patch.object(extra_mutants, "run_crate", lambda *_: ([], counts, 1.0)),
            ):
                self.assertEqual(extra_mutants.main(["extra_mutants.py", "cpk"]), code)


class LeftCopies(unittest.TestCase):
    def test_every_copy_a_killed_run_leaves_is_found_and_nothing_else(self):
        """cargo-mutants names its copy after the `--dir` folder, the run's `source-*` copy
        of the snapshot: a pattern naming the repository's folder found none of them."""
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            left = [
                "cargo-mutants-source-47472-TFiBQg.tmp", "source-47472",
                "extra-mutants-9120-0", "extra-mutants-9120-1",
            ]
            for name in left:
                (folder / name).mkdir()
                (folder / name / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            (folder / "keep").mkdir()
            (folder / "keep" / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            self.assertEqual(
                sorted(path.name for path in rm_mutant_copies.copies(folder)), sorted(left)
            )


class ExtraExclusions(unittest.TestCase):
    def test_an_exclusion_of_an_unknown_kind_stops_the_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / ".cargo").mkdir()
            (Path(tmp) / ".cargo" / "extra_mutants.toml").write_text(
                '[[exclude]]\npath = "crates/libs/cpk/src/lib.rs"\nkind = "booll"\n'
                'description = "flip 1 | removed: true"\nwhy = "an equivalent flip"\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(SystemExit, "entry 1.*booll"):
                extra_mutants.exclusions(Path(tmp))


class RemoteDiskUse(unittest.TestCase):
    def test_the_folders_size_comes_first_and_the_free_space_second(self):
        # Swapped, the guard would read the folder's size as the disk's free space.
        def ssh(host, command, **kwargs):
            self.assertLess(command.index("du -sk"), command.index("df -Pk"))
            return subprocess.CompletedProcess([host, command], 0, stdout="13631488\n34603008\n")

        with mock.patch.object(mutants, "ssh", ssh):
            self.assertEqual(
                mutants.remote_disk_use("bonfire"), (13 * 2**30, 33 * 2**30)
            )


class DiskRefusal(unittest.TestCase):
    """The disk guard at its bound, with 2026-10-22's figures as the failing case."""

    def quiet(self, used: int, free: int) -> str | None:
        with contextlib.redirect_stdout(io.StringIO()):
            return mutants.disk_refusal(used, free)

    def test_the_floor_itself_passes(self):
        self.assertIsNone(self.quiet(2**30, mutants.REMOTE_FREE_FLOOR))

    def test_what_the_folder_holds_refuses_nothing_while_the_floor_is_free(self):
        self.assertIsNone(self.quiet(50 * 2**30, mutants.REMOTE_FREE_FLOOR))

    def test_one_byte_below_the_floor_refuses_naming_the_cleanup_and_the_maintainer(self):
        refusal = self.quiet(13 * 2**30, mutants.REMOTE_FREE_FLOOR - 1)
        self.assertIn("below the floor", refusal)
        self.assertIn("13.0 GiB in", refusal)
        self.assertIn("The scratch folder", refusal)
        self.assertIn("maintainer", refusal)

    def test_the_full_disk_of_2026_10_22_refuses(self):
        # 50 GB of this project's on a disk with 2.2 GB free.
        self.assertIn("below the floor", self.quiet(50 * 2**30, 2 * 2**30))


class DiffSurvivors(unittest.TestCase):
    def test_a_split_crate_run_gives_its_merged_list_and_a_local_one_its_own(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertEqual(mutants_diff.missed_lines(out), [])
            (out / "missed.txt").write_text("local only\n", encoding="utf-8")
            self.assertEqual(mutants_diff.missed_lines(out), ["local only"])
            (out / "missed_all.txt").write_text("local\nremote\n", encoding="utf-8")
            self.assertEqual(mutants_diff.missed_lines(out), ["local", "remote"])


if __name__ == "__main__":
    unittest.main()
