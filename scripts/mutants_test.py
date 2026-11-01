"""Tests of `mutants.py`'s and `mutants_diff.py`'s readings of a finished run's files: which
mutants never ran, which builds were killed, when a split run's summary fails, and which list
holds a crate's survivors. Each builds a `mutants.out` in a
temporary directory the way cargo-mutants writes one; nothing reaches the host. Also which
source the local half, the listings and the config read (cargo itself stubbed), that a run
snapshots before it waits for the one in progress (`run_lock`), and that no run here starts
while a remote half is uncollected (`remote_pending`, ssh stubbed). Run by `just
test` (`python scripts/mutants_test.py`)."""
import contextlib
import io
import json
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
import extra_mutants  # noqa: E402
import mutants  # noqa: E402
import mutants_diff  # noqa: E402

HEAD_TREE = subprocess.run(
    ["git", "rev-parse", "HEAD^{tree}"], cwd=SCRIPTS, capture_output=True, text=True, check=True
).stdout.strip()
# Git's empty tree: a diff from it holds every file of the tree it is diffed with.
EMPTY_TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"


def mutant(name: str) -> dict:
    return {"Mutant": {"name": name}}


def write_run(out: Path, selected: int, outcomes: list[dict], logs: dict[str, str] = {}) -> None:
    """A `mutants.out` at `out`: a selection of `selected` mutants, the baseline and
    `outcomes`, and each `logs` entry as the log file of that name."""
    out.mkdir(parents=True, exist_ok=True)
    (out / "mutants.json").write_text(json.dumps([{}] * selected), encoding="utf-8")
    baseline = {"scenario": "Baseline", "summary": "Success", "phase_results": [{"duration": 1.0}]}
    (out / "outcomes.json").write_text(
        json.dumps({"outcomes": [baseline, *outcomes]}), encoding="utf-8"
    )
    for name, text in logs.items():
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


class KilledBuilds(unittest.TestCase):
    def test_every_spelling_of_a_killed_build_and_a_signal_ended_cargo_count(self):
        logs = {
            "log/oom": "error: could not compile (signal: 9, SIGKILL: kill)",
            "log/linker": "collect2: fatal error: ld terminated with signal 9 [Killed]",
            "log/llvm": "LLVM ERROR: out of memory",
            "log/windows": "memory allocation of 1048576 bytes failed",
            "log/console": "process didn't exit successfully (exit code: 0xc0000142, STATUS_DLL_INIT_FAILED)",
            "log/mutant": "error[E0308]: mismatched types",
        }
        outcomes = [
            {"scenario": mutant(name), "summary": "Unviable", "log_path": f"log/{name}"}
            for name in ("oom", "linker", "llvm", "windows", "console", "mutant")
        ]
        outcomes.append({"scenario": mutant("signal"), "summary": "Failure"})
        outcomes.append({"scenario": mutant("caught"), "summary": "CaughtMutant"})
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_run(out, len(outcomes), outcomes, logs)
            self.assertEqual(
                mutants.killed_builds(out),
                ["oom", "linker", "llvm", "windows", "console", "signal"],
            )


class Summarize(unittest.TestCase):
    def summarize(self, root: Path, remote_only: bool = False) -> int:
        # The maintainer's switch is set here, not read: the test holds in either mode.
        saved = mutants.ROOT, mutants.REMOTE_ONLY
        mutants.ROOT, mutants.REMOTE_ONLY = root, remote_only
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                return mutants.summarize(0, 0, root / "remote_console.txt", (None, None))
        finally:
            mutants.ROOT, mutants.REMOTE_ONLY = saved

    def test_a_split_run_whose_local_half_never_started_fails(self):
        caught = [{"scenario": mutant("a"), "summary": "CaughtMutant"}]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_run(root / "mutants.out" / "remote" / "mutants.out", 1, caught)
            self.assertEqual(self.summarize(root), 1)
            # A remote-only run has no local half to start.
            self.assertEqual(self.summarize(root, remote_only=True), 0)
            write_run(root / "mutants.out", 1, caught)
            self.assertEqual(self.summarize(root), 0)


class SplitCoverage(unittest.TestCase):
    def summarize(self, root: Path) -> tuple[int, str]:
        saved = mutants.ROOT
        mutants.ROOT = root
        try:
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = mutants.summarize(0, 0, root / "remote_console.txt", (None, None))
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
        self.assertEqual(parsed["additional_cargo_args"], ["--jobs", "8"])


class Partition(unittest.TestCase):
    def test_each_file_goes_to_the_side_further_below_its_share_largest_first(self):
        files = {"a.rs": 40, "b.rs": 30, "c.rs": 20, "d.rs": 10}
        self.assertEqual(mutants.partition(files, 0.6), {"a.rs", "c.rs"})

    def test_a_split_further_from_the_share_than_halves_falls_back(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertIsNone(mutants.partition({"a.rs": 90, "b.rs": 10}, 0.55))


class LocalShare(unittest.TestCase):
    def test_the_slower_half_gets_the_smaller_share(self):
        cache = {
            "mutant": {"cpk": 10.0},
            "remote_mutant": {"cpk": 15.0},
            "baseline": {},
        }
        self.assertAlmostEqual(mutants.local_share("cpk", cache), 0.6)

    def test_a_half_without_a_measurement_gives_no_share(self):
        cache = {"mutant": {"cpk": 10.0}, "remote_mutant": {}, "baseline": {}}
        self.assertIsNone(mutants.local_share("cpk", cache))
        cache = {"mutant": {}, "remote_mutant": {"cpk": 15.0}, "baseline": {}}
        self.assertIsNone(mutants.local_share("cpk", cache))


class CostCache(unittest.TestCase):
    def update(self, root: Path) -> bool:
        """Whether `update_cost_cache` over `root`'s `mutants.out` wrote the cache."""
        saved = mutants.ROOT, mutants.COST_CACHE
        mutants.ROOT, mutants.COST_CACHE = root, root / "cost.json"
        try:
            mutants.update_cost_cache()
            return mutants.COST_CACHE.exists()
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
            out = Path(tmp) / "mutants.out"
            killed = "process didn't exit successfully (exit code: 0xc0000142)"
            write_run(out, 5, outcomes("Unviable"), {"log/m": killed})
            self.assertFalse(self.update(Path(tmp)))
            write_run(out, 5, outcomes("CaughtMutant"), {"log/m": "test failed"})
            self.assertTrue(self.update(Path(tmp)))

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
            self.assertNotIn("cpk", cache["mutant"])
            self.assertNotIn("cpk", cache["baseline"])


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
    """A remote half launched from this tree and not yet fetched refuses every run on this
    machine alone, whatever STUDIO_MUTANTS_REMOTE says now: its collection summarizes the
    local `mutants.out` that a new run would set aside."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        patcher = mock.patch.object(mutants, "ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def pend(self, host: str = "bonfire") -> Path:
        pending = mutants.remote_pending()
        pending.parent.mkdir(parents=True, exist_ok=True)
        pending.write_text(host + "\n", encoding="utf-8")
        return pending

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
            mutants.launch_remote("bonfire", ["-p", "fpc"])
        self.assertEqual(recorded_at_start, [True])
        self.assertEqual(mutants.remote_pending().read_text(encoding="utf-8"), "bonfire\n")

    def test_the_fetch_clears_the_record(self):
        pending = self.pend()

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
        self.pend()
        last = self.root / "mutants.out"
        last.mkdir()
        out = io.StringIO()
        with (
            mock.patch.object(mutants, "remote_host", lambda: None),
            contextlib.redirect_stdout(out),
        ):
            self.assertEqual(mutants.whole_run("fpc", None, HEAD_TREE, self.root), 1)
            self.assertEqual(
                mutants_diff.run_crate("in.diff", HEAD_TREE, self.root, "fpc", 3), 1
            )
        self.assertTrue(last.exists(), "the uncollected half's local results stay")
        self.assertEqual(out.getvalue().count("mutants-collect"), 2, out.getvalue())
        self.assertIn("bonfire", out.getvalue())

    def test_with_no_record_the_host_decides(self):
        with mock.patch.object(mutants, "remote_refusal", lambda host, remote: f"{host} {remote}"):
            self.assertIsNone(mutants.local_refusal(None))
            self.assertEqual(mutants.local_refusal("bonfire"), "bonfire False")

    def test_a_collect_finding_no_half_clears_a_stale_record(self):
        pending = self.pend()
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
            "with mutants.run_lock(Path(sys.argv[2])):\n"
            "    Path(sys.argv[3]).touch()\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            lock = Path(tmp) / "mutants.lock"
            entered = Path(tmp) / "entered"
            argv = [sys.executable, "-c", child, str(SCRIPTS), str(lock), str(entered)]
            with mutants.run_lock(lock):
                second = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                try:
                    with contextlib.suppress(subprocess.TimeoutExpired):
                        second.wait(timeout=2)
                    self.assertFalse(entered.exists(), "the second run started inside the first")
                except BaseException:
                    second.kill()
                    raise
            out, _ = second.communicate(timeout=30)
            self.assertTrue(entered.exists(), out.decode(errors="replace"))
            self.assertIn(b"waits", out)

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
                mutants_diff.main(["mutants_diff.py", EMPTY_TREE, "scripts/mutants.py"])

        self.assertEqual(
            self.order(call), ["snapshot", "lock", "copy", "run", "uncopy", "unlock"]
        )

    def test_a_whole_crate_run_snapshots_before_it_waits(self):
        def call(events):
            with mock.patch.object(mutants, "whole_run", lambda *_: events.append("run") or 0):
                mutants.main(["mutants.py", "fpc"])

        self.assertEqual(
            self.order(call), ["snapshot", "lock", "copy", "run", "uncopy", "unlock"]
        )

    def test_a_collect_holds_the_lock(self):
        """A run queued behind a collect would replace the `mutants.out` it summarizes."""

        def call(events):
            with mock.patch.object(mutants, "remote_host", lambda: "bonfire"), \
                    mock.patch.object(mutants, "collect", lambda *_: events.append("run") or 0):
                mutants.main(["mutants.py", "--collect"])

        self.assertEqual(self.order(call), ["lock", "run", "unlock"])

    def test_an_extra_mutants_run_snapshots_before_it_waits(self):
        def call(events):
            with mock.patch.object(extra_mutants, "run_crate", lambda *_: events.append("run")):
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


class DiskRefusal(unittest.TestCase):
    """The disk guard at its two bounds, with 2026-10-22's figures as the failing case."""

    def quiet(self, used: int, free: int) -> str | None:
        with contextlib.redirect_stdout(io.StringIO()):
            return mutants.disk_refusal(used, free)

    def test_the_budget_and_the_floor_themselves_pass(self):
        self.assertIsNone(self.quiet(mutants.REMOTE_DISK_BUDGET, mutants.REMOTE_FREE_FLOOR))

    def test_one_byte_over_the_budget_refuses_naming_the_cleanup(self):
        refusal = self.quiet(mutants.REMOTE_DISK_BUDGET + 1, 60 * 2**30)
        self.assertIn("over the budget", refusal)
        self.assertIn("The scratch folder", refusal)

    def test_one_byte_below_the_floor_refuses_naming_the_maintainer(self):
        refusal = self.quiet(2**30, mutants.REMOTE_FREE_FLOOR - 1)
        self.assertIn("below the floor", refusal)
        self.assertIn("maintainer", refusal)

    def test_the_full_disk_of_2026_10_22_refuses(self):
        # 50 GB of this project's on a disk with 2.2 GB free.
        self.assertIn("over the budget", self.quiet(50 * 2**30, 2 * 2**30))


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
