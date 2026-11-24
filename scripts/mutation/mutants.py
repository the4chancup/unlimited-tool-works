"""`just mutants <crate>`: a whole-crate cargo-mutants run — on the VPS alone.

A whole-crate run measures for days, so it runs every mutant on the host
`STUDIO_MUTANTS_REMOTE` names (e.g. `bonfire`), none on this machine: a local
half would block in-game checks and restarts here, and a power cut would lose
it. The run refuses without a host, and for a crate in `LOCAL_ONLY_CRATES`
(its builds outgrow the remote's memory cap). The extra kinds (`EXTRA_KINDS`)
run there too, in the same service before cargo-mutants (`--remote-extra`),
their results fetched with the rest. `just extra-mutants-remote <crate>`
(`--extra-only`) runs the extra kinds alone the same way — a kind's backfill
over the crates measures for days of its own — its results into
`extra_mutants.out/remote/`. The run waits for its remote half holding
no run lock — the launch, and the fetch and summary, each take it — so
`just mutants-diff` runs meanwhile; it runs on this machine whole while the
remote is busy (`mutants_diff.py`).

A `mutants-diff` crate run still splits with the host when the split pays, by
the file split below (`plan_file_split`, `partition`, `check_split`,
`split.json`): it hands each half a complementary set of the selection's
source files, sized by the two halves' measured seconds per tested mutant
(`COST_CACHE`, `local_share`), through `examine_globs` in the local config and
`exclude_globs` in the remote's, so the slower half tests fewer mutants and
the two end together. Where no file split can be planned (a half not measured
yet, shares within `SHARE_MIN_GAIN` of halves, mutants in too few files) the
remote gets the even `--shard 1/2` round-robin share. A whole run that needs
this machine again splits the same way, with the local extra pass in `split`.
The remote half's results merge into `mutants.out/remote/`.

The remote half runs as a transient systemd service on the host (`sudo -n
systemd-run --unit=studio-mutants`), detached from the ssh session by
construction, memory-capped and CPU-idle: the host also runs the production
Fluxer instance, an allocating mutant filled its RAM and swap at 2.20i, and
`nice` alone cannot keep the run off Fluxer's CPU (cgroup v2 weights ignore
it). Its state lives in `~/studio-mutants/run/` (`job.sh`, `in.diff`, `pid`,
`memory_max`, `log`, `exit`, `memory_peak`, `collected`), and this machine only
polls it with short ssh calls, so a dropped connection costs one poll, not the run:
cargo-mutants cannot resume, and a remote half tied to one long ssh session
died with it (2.20i). If this script itself stops, `just mutants-collect`
waits for the remote half and fetches it. A run that uses the remote refuses
to start while a remote half is running or finished but not yet collected, or
while `just gates-remote` runs there; a run on this machine alone refuses only
for this tree's own uncollected half that had a local half (`remote_pending`),
whose collection summarizes the local `mutants.out`. A
running half is stopped by hand with `sudo systemctl stop studio-mutants` on
the host.

The remote needs: git, a C toolchain, rustup, cargo-mutants, cargo-nextest,
cargo-llvm-cov and `just` (for `just gates-remote`) at the local versions,
installed with `cargo install` run inside `~/studio-mutants/tree` where the
pinned toolchain applies (the host has no default toolchain; the toolchain
itself installs on first use there), rustup's `llvm-tools-preview` component
added inside that tree so it lands on the pinned toolchain too (the whole
run's coverage map needs both it and cargo-llvm-cov), `sudo -n systemd-run`
for the ssh user, `ionice`,
`tar`, `nproc`, `awk`, cgroup v2 (the unit's `memory.peak`) and `python3` 3.13 or later (`just gates`'
scripts; the extra kinds' `glob.translate` came in 3.13).
"""

import contextlib
import io
import json
import os
import re
import shutil
import stat
import statistics
import subprocess
import sys
import tarfile
import tempfile
import threading
import time
import tomllib
from collections import Counter
from collections.abc import Iterable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REMOTE_BASE = "~/studio-mutants"
REMOTE_REF = "refs/mutants/remote"
# Whether `just mutants-diff` runs every mutant on the remote (when
# STUDIO_MUTANTS_REMOTE names one) and nothing on this machine, whatever the
# estimate or LOCAL_ONLY_CRATES; `just gates-remote` is the gates' counterpart.
# `just mutants <crate>` no longer reads it: a whole run is remote-only always.
# A constant, not an environment variable: a shell started while a variable was
# set keeps the old value, which silently picked the wrong mode.
REMOTE_ONLY = False
# The extra_mutants kinds `just mutants <crate>` runs before the cargo-mutants
# run starts, so a whole-crate run stays one command and its missed_all.txt the
# one list to triage. A kind is listed when about half its survivors are real
# gaps, since each equivalent survivor costs a ruling every phase. `bool` joined
# once `.cargo/extra_mutants.toml` gave its ruled equivalents a home.
# `mutants-diff` does not run them: they run when a phase closes, only.
EXTRA_KINDS = ("bool", "call", "literal", "normalize", "order")
# missed.txt lines whose only catching tests are #[cfg(windows)]: they survive on
# the Linux remote half, so summarize() reruns them on this machine when it is
# Windows and REMOTE_ONLY is off. The seed is deploy.rs's match guards
# (deploy.rs:180, :409 and :565 survived on bonfire and were caught on rerun
# here): sweep_run's lock arm is caught by
# a_staging_whose_lock_cannot_be_opened_is_left (a lock path that is a folder
# cannot be opened on Windows; Linux opens and locks folders), probe_download's
# access-denied arm by the held-open CLI tests
# an_old_cpk_held_open_is_old_cpk_locked_and_the_cpk_is_promoted and
# a_cpk_held_open_stops_the_upgrade_naming_it_and_the_list_stays (a share-mode
# violation is not ErrorKind::PermissionDenied, and nothing holds a file open on
# Linux), promote_tree's removal arm by
# a_previous_tree_that_cannot_be_removed_fails_the_promotion_and_is_not_overlaid
# (FILE_SHARE_READ denies the delete on Windows). Matched by file and function,
# never by line number, which drifts. studio_core's running-PES probe has a
# `#[cfg(windows)]` arm (processes, image_path, probe) the Linux half never
# compiles, so its mutants all came back missed there; `probe` also names
# the Linux and other-platform arms, which a rerun here leaves missed, as they are.
# studio_core's log redaction matches the home folder in any letter case and with
# either slash on Windows alone, and its test is `#[cfg(windows)]`.
# studio_core's update temp-folder removal is caught only by
# the_succeeded_mark_stays_beside_a_backup_that_cannot_be_removed, which is
# `#[cfg(windows)]`: on Linux every test's backup deletes whole, so
# remove_temp's mark write and remove_temp_folder's not-found arm have no
# other catcher.
# taskbar's set_relaunch and set_relaunch_values are `#[cfg(windows)]`, and so
# is every test that reaches them: the Linux half compiles neither, so their
# mutants all come back missed there.
WINDOWS_ONLY_TESTS = [
    re.compile(
        r"crates/tools/team_compiler/src/output/deploy\.rs:\d+:\d+: .* in "
        r"(sweep_run|probe_download|promote_tree)$"
    ),
    re.compile(
        r"crates/studio_core/src/pes_process\.rs:\d+:\d+: "
        r"(replace (processes|image_path|probe) -> |.* in (processes|image_path|probe)$)"
    ),
    re.compile(r"crates/tools/export_upgrader/src/staging\.rs:\d+:\d+: .* in retry_while_locked$"),
    re.compile(
        r"crates/studio_core/src/updater/install\.rs:\d+:\d+: "
        r"(replace (remove_temp|remove_temp_folder) -> "
        r"|.* in (remove_temp|remove_temp_folder)$)"
    ),
    re.compile(
        r"crates/studio_core/src/redact\.rs:\d+:\d+: "
        r"(replace redact_any_spelling -> |.* in redact_(any_spelling|home)$)"
    ),
    re.compile(
        r"crates/studio_core/src/shell/taskbar\.rs:\d+:\d+: "
        r"(replace set_relaunch with \(\)|replace set_relaunch_values -> "
        r"|.* in (set_relaunch|set_relaunch_values)$)"
    ),
]
# The extra kinds' missed lines (`path:line: [kind] in <fn>: ...`) in
# Windows-only code get the same recheck as the cargo-mutants lines above:
# matched in the extra pass's line shape, where a [timeout]/[untested] tag
# before the kind does not match (only missed lines are rechecked).
# elevation's windows.rs is #[cfg(windows)] whole; the other entries give
# each WINDOWS_ONLY_TESTS entry the same file and functions. An entry added
# to either list gets its twin in the other.
_EXTRA_KIND = "(?:" + "|".join(EXTRA_KINDS) + ")"
WINDOWS_ONLY_EXTRA = [
    re.compile(rf"crates/libs/elevation/src/windows\.rs:\d+: \[{_EXTRA_KIND}\] "),
    re.compile(
        rf"crates/tools/team_compiler/src/output/deploy\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in (sweep_run|probe_download|promote_tree):"
    ),
    re.compile(
        rf"crates/studio_core/src/pes_process\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in (processes|image_path|probe):"
    ),
    re.compile(
        rf"crates/tools/export_upgrader/src/staging\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in retry_while_locked:"
    ),
    re.compile(
        rf"crates/studio_core/src/updater/install\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in (remove_temp|remove_temp_folder):"
    ),
    re.compile(
        rf"crates/studio_core/src/redact\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in (redact_any_spelling|redact_home):"
    ),
    re.compile(
        rf"crates/studio_core/src/shell/taskbar\.rs:\d+: "
        rf"\[{_EXTRA_KIND}\] in (set_relaunch|set_relaunch_values):"
    ),
]
# The remote half's systemd unit: a transient service so its cgroup is
# memory-capped and CPU-idle: an allocating mutant once OOM-killed the host,
# and Fluxer's production services have priority.
REMOTE_UNIT = "studio-mutants"
# `just gates-remote`'s unit (`remote_gates.py`), which a mutants run waits for:
# both rewrite the remote clone.
GATES_UNIT = "studio-gates"
# What a unit's MemoryMax (with MemorySwapMax=0) leaves the host: the cap is the
# host's available memory when the unit starts, less this, so at the cap the
# kernel OOM-kills inside the unit only, never in Fluxer's. Measured at launch,
# not fixed: the host's free memory changes as its services do, and a fixed cap
# either wasted what was free or squeezed the host. A service that grows during
# a run eats into this margin, so it is not cut finer.
REMOTE_MEMORY_RESERVE = 2**30
# The remote half's `--jobserver-tasks`: the build cap, since under
# cargo-mutants 27.1.0's jobserver cargo ignores `-j`/`--build-jobs`. `None` is
# the host's CPU count (`nproc`, already probed for the config), the default
# every remote run built under; a number lowers it when builds are killed at
# the unit's memory cap.
REMOTE_JOBSERVER_TASKS: int | None = None
# Crates whose mutants never go to the remote because their builds outgrow the
# cap even at one jobserver task. A `mutants-diff` holding any of their mutants
# runs locally whole. Empty now: team_compiler was listed after two of its
# builds were killed at 9 GiB, until the cause turned out to be the host's
# tmpfs `/tmp`, where cargo-mutants put its two build copies
# (`REMOTE_TMPDIR` below). If a crate's builds are killed again with the
# copies on disk, list it here.
LOCAL_ONLY_CRATES: frozenset[str] = frozenset()
# Where the remote half's cargo-mutants makes its two copies of the tree. The
# host's `/tmp`, cargo-mutants' default, is a 7.8 GiB tmpfs (checked
# 2026-10-09), so each copy's `target/` (4 GiB with egui linked) lived in RAM
# and counted against the unit's MemoryMax: the team_compiler kills were per-mutant
# rebuilds of 1-2 GiB on top of two such trees, not the egui build itself.
REMOTE_TMPDIR = "$HOME/studio-mutants/tmp"
# The least free space the host's disk must have when a run that uses the remote
# starts, whatever this project holds. A `save_editor` half takes more than 33 GiB of
# it: one that started with 35.4 GiB free filled the disk on 2026-11-22 and lost a
# mutant, where on 2026-11-12 one had peaked at 23.6 GiB in `REMOTE_TMPDIR` (its
# tests' leftovers grow with each caught mutant; no other crate's is measured). So
# the floor is that failure's start plus a third; 2 of it stay for the host's own
# system (its logs, its other services, idle since 2026-10-29). Each earlier floor
# failed the same way: at 12, a half that started with 19.5 GiB free; at 26, set from
# the 23.6 measurement, the 2026-11-22 one. Nothing bounds what `REMOTE_BASE` itself holds:
# the host can spare its whole disk but those 2, so only the free space is guarded.
# On 2026-10-22 the folder held 50 GB (28.6 of them tests' leftovers in
# `REMOTE_TMPDIR`, 22 the clone's build cache), the disk was at 99%, and a half
# lost five mutants to it (RUNBOOK "The scratch folder").
REMOTE_FREE_FLOOR = 48 * 2**30
# Where the local half's cargo-mutants makes its copies of the tree (and its
# builds and tests their temporary files): one fixed folder, so a single
# antivirus exclusion covers every run's randomly named copy, whose tests write
# a sandbox export per CLI test.
LOCAL_TMPDIR = Path(tempfile.gettempdir()) / "studio-mutants"
# The detached remote half's files: `job.sh`, `in.diff` (a diff run's diff),
# `pid` (the service's MainPID), `memory_max` (its cap, `remote_memory_max`),
# `log`, `exit` ("<code> <seconds>", written when cargo-mutants returns),
# `memory_peak` (the unit cgroup's peak memory) and `collected` (written by
# the fetch).
REMOTE_RUN = f"{REMOTE_BASE}/run"
# What a run costs on this machine, measured per crate from the local half of
# every run: the mean seconds of one mutant and the unmutated baseline's
# seconds. Kept per user, not in a tree's `target/`: the numbers are this
# machine's, so the main tree and the mutants worktree share one table instead
# of drifting apart, and deleting a `target/` to free disk loses none of it.
COST_CACHE = (
    Path(os.environ.get("LOCALAPPDATA") or Path.home() / ".cache") / "4cc-studio" / "mutants-cost.json"
)
# One mutation run at a time for this user on this machine (`run_lock`), whichever worktree
# launches it: every run shares the machine's build slots and the remote. A run launched
# meanwhile polls for it.
RUN_LOCK = COST_CACHE.with_name("mutants.lock")
LOCK_POLL_SECONDS = 30
# Mutants a run tests at once, each in its own copy of the tree and given
# `cpus // JOBS` test threads (`sized_config` sizes cargo-mutants' jobs,
# `extra_mutants` its own): a mutant's incremental build uses few cores, so a
# second job keeps the rest busy.
JOBS = 2
# The baseline (a cold build and test of the mutated packages in a fresh copy
# of the tree) of a crate no local run has measured yet: one measured run's
# 56 s + 8 s for team_compiler, studio_core and studio, rounded.
DEFAULT_BASELINE_SECONDS = 60.0
# Polling starts when the local half ends, so this is the wait after a remote
# half that finishes last (about half the runs measured), a mean of half
# the interval; each poll is one short ssh call.
POLL_SECONDS = 15
# Below this estimated local wall time a diff run stays local: the split's
# fixed overhead (about a minute plus up to POLL_SECONDS of polling) eats
# the halving.
SPLIT_THRESHOLD_SECONDS = 240
# How many tested mutants of history a crate's seconds-per-tested-mutant mean
# keeps (`update_cost_cache`): a new run's mutants are weighed against at most
# this many earlier ones, so the mean follows a suite as it grows instead of
# averaging in what it cost months ago, and one small run cannot swing it.
TESTED_HISTORY = 100
# A half's mean is used for a share once it stands on this many tested mutants.
SHARE_MIN_TESTED = 10
# A share nearer to halves than this stays round-robin: the halves' means are
# not measured more finely than that, and round-robin mixes each half's files.
SHARE_MIN_GAIN = 0.05
# Consecutive failed polls before giving up (an hour at 15 s); the remote half
# keeps running and `just mutants-collect` picks it up later.
MAX_POLL_FAILURES = 240
# One poll's ssh call is a status line; past this it counts as a failed poll.
POLL_CALL_TIMEOUT_SECONDS = 120
# Keepalives let a dead link fail a call in about a minute instead of hanging it.
SSH_OPTIONS = [
    "-o", "BatchMode=yes",
    "-o", "ConnectTimeout=20",
    "-o", "ServerAliveInterval=15",
    "-o", "ServerAliveCountMax=4",
]


def ssh_binary() -> str:
    # Windows' own OpenSSH client shares the Windows ssh-agent that holds the
    # maintainer's key; Git Bash's /usr/bin/ssh comes first on PATH under bash
    # and fails with "Permission denied (publickey)".
    if os.name == "nt":
        candidate = Path(os.environ["SystemRoot"]) / "System32" / "OpenSSH" / "ssh.exe"
        if candidate.exists():
            return str(candidate)
    return "ssh"


def ssh(host: str, remote: str, **kwargs) -> subprocess.CompletedProcess:
    # A call that sends nothing gets an empty stdin (what `ssh -n` does). Without
    # it, two calls of a detached run once hung for good with the link up, the likely
    # cause being Windows' ssh.exe waiting on the inherited hidden console.
    if "input" not in kwargs:
        kwargs.setdefault("stdin", subprocess.DEVNULL)
    return subprocess.run([ssh_binary(), *SSH_OPTIONS, host, remote], **kwargs)


def git(*args: str, **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=ROOT, **kwargs)


def working_tree() -> str:
    """Write the whole working tree (tracked edits and untracked non-ignored
    files: a run measures the tree under review) as a git tree object, through
    a private index so the real one is not touched: one per process, since a
    `mutants-diff` and a `gates-remote` may snapshot at once. Returns the
    tree's id."""
    index = git(
        "rev-parse", "--git-path", f"mutants-remote-{os.getpid()}.index",
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    index_path = Path(index)
    if not index_path.is_absolute():
        index_path = ROOT / index_path
    env = dict(os.environ, GIT_INDEX_FILE=str(index_path))
    try:
        # Seeded from HEAD, so a tracked file keeps the mode git records: where
        # `core.fileMode` is off (Windows), a fresh index adds every file as
        # 100644, and the remote could not run the executable stand-in for
        # cargo (`select_tests.py`).
        seeded = git("read-tree", "HEAD", env=env, capture_output=True, text=True)
        if seeded.returncode != 0:
            raise RuntimeError(f"git read-tree HEAD failed:\n{seeded.stderr}")
        # A fresh index re-adds every file, flooding stderr with CRLF warnings;
        # silence it on success only.
        added = subprocess.run(
            ["git", "add", "-A"], cwd=ROOT, env=env, capture_output=True, text=True
        )
        if added.returncode != 0:
            raise RuntimeError(f"git add -A failed:\n{added.stderr}")
        written = git("write-tree", env=env, capture_output=True, text=True)
        if written.returncode != 0:
            raise RuntimeError(f"git write-tree failed:\n{written.stderr}")
        return written.stdout.strip()
    finally:
        index_path.unlink(missing_ok=True)


def extract_tree(tree: str, dest: Path) -> None:
    """Extract git tree `tree` into `dest`, an empty folder (created if absent)."""
    dest.mkdir(parents=True, exist_ok=True)
    # The whole archive is read before extracting, as `fetch_remote` reads the remote
    # half's results: streamed through a pipe, `git archive` twice sat unexited after
    # writing every member (once for 35 minutes, not reproduced since).
    archived = subprocess.run(
        ["git", "archive", "--format=tar", tree],
        cwd=ROOT,
        capture_output=True,
    )
    if archived.returncode != 0:
        raise RuntimeError(
            f"git archive {tree} failed: {archived.stderr.decode(errors='replace')}"
        )
    with tarfile.open(fileobj=io.BytesIO(archived.stdout), mode="r|") as archive:
        archive.extractall(dest, filter="data")


def remove_tree(path: Path) -> None:
    """`shutil.rmtree`, clearing the read-only bit of a file it cannot delete: a test's
    scratch output can hold one (`team_compiler`'s unwritable-deploy test), which left a
    whole copy, `target/` included, behind on Windows."""

    def make_writable(function, failed, _error):
        os.chmod(failed, stat.S_IWRITE)
        function(failed)

    shutil.rmtree(path, onexc=make_writable)


@contextlib.contextmanager
def snapshot_source(tree: str):
    """Extract git `tree`, the run's snapshot, to a fresh folder under `LOCAL_TMPDIR` for the
    run's length and yield its path. Everything the run reads from the tree comes from there:
    the mutant listing, `.cargo/mutants.toml` and the local half's source. A queued run reaches
    its turn long after its snapshot, and so can a crate's turn in a diff run; an edit or a
    merge in between left the diff naming lines the live tree no longer held, and the half
    refused to start (or a whole-crate half measured code the remote half never saw). A fresh
    name, not one per process: the Windows recheck opens a second copy inside a run's. The
    folder is removed at the end; a removal that fails is a warning, never an error that would
    replace the finished run's result."""
    LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    source = Path(tempfile.mkdtemp(dir=LOCAL_TMPDIR, prefix="source-"))
    try:
        extract_tree(tree, source)
        yield source
    finally:
        try:
            remove_tree(source)
        except OSError as error:
            print(f"warning: could not remove the run's copy {source}: {error}",
                  file=sys.stderr, flush=True)


def try_lock(handle) -> bool:
    """Take the OS lock on `handle`'s first byte without waiting; False when another
    process holds it."""
    handle.seek(0)
    try:
        if os.name == "nt":
            import msvcrt

            msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl

            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        return False
    return True


def unlock(handle) -> None:
    handle.seek(0)
    if os.name == "nt":
        import msvcrt

        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
    else:
        import fcntl

        fcntl.flock(handle, fcntl.LOCK_UN)


@contextlib.contextmanager
def run_lock(path: Path = RUN_LOCK):
    """Hold the mutation runs' lock, for a run's whole length except a whole run's wait on the
    remote (`whole_run`, `collect`): a run launched while another
    is under way waits for it rather than racing it for `mutants.out`, `mutants.diff/`, the
    remote and the build slots. Every entry point snapshots the tree first, so a waiting run
    measures the tree as it stood when its command started, whatever is edited or merged
    while it waits; a chain of commands would snapshot each run only when the one before
    ends. The lock is the OS's, released with the process, so a killed run leaves none
    behind."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a+b") as handle:
        if not try_lock(handle):
            print(f"another mutation run holds {path.name}: this one waits, its snapshot taken",
                  flush=True)
            while not try_lock(handle):
                time.sleep(LOCK_POLL_SECONDS)
        try:
            yield
        finally:
            unlock(handle)


def snapshot(tree: str | None = None) -> str:
    """Commit `tree`, else the whole working tree (`working_tree`), to
    `refs/mutants/remote`. Returns the tree. A caller that already wrote the
    tree passes it, so what the remote runs is what it measured, not a
    second read of a tree the maintainer may have edited since."""
    tree = tree or working_tree()
    commit = git(
        "commit-tree", tree, "-p", "HEAD", "-m", "mutants remote snapshot",
        check=True, capture_output=True, text=True,
    ).stdout.strip()
    git("update-ref", REMOTE_REF, commit, check=True)
    return tree


def remote_state(host: str) -> tuple[str | None, str | None, int, str]:
    """The remote clone's HEAD and tree (None without a clone), its CPU count and
    its cargo-mutants version, in one call. The binary is run directly (as
    `cargo-mutants mutants`), since the host has no default toolchain for `cargo`."""
    out = ssh(
        host,
        f"git -C {REMOTE_BASE}/tree rev-parse HEAD 'HEAD^{{tree}}' 2>/dev/null || true; "
        "nproc; ~/.cargo/bin/cargo-mutants mutants --version",
        capture_output=True, text=True,
    )
    if out.returncode != 0:
        raise RuntimeError(f"probing {host} failed:\n{out.stderr}")
    out = out.stdout.splitlines()
    head, tree = (out[0], out[1]) if len(out) == 4 else (None, None)
    return head, tree, int(out[-2]), out[-1].strip()


def transfer(host: str, remote_head: str | None) -> None:
    """Send the snapshot as a bundle and check it out remotely."""
    incremental = remote_head is not None and git(
        "cat-file", "-e", f"{remote_head}^{{commit}}"
    ).returncode == 0
    bundle = tempfile.NamedTemporaryFile(suffix=".bundle", delete=False)
    bundle.close()
    try:
        if incremental:
            git("bundle", "create", bundle.name, REMOTE_REF, f"^{remote_head}",
                check=True)
            print(f"sending incremental bundle (past remote HEAD {remote_head[:12]})",
                  flush=True)
        else:
            git("bundle", "create", bundle.name, REMOTE_REF, check=True)
            print("sending full bundle", flush=True)
        remote = (
            f"mkdir -p {REMOTE_BASE} && cd {REMOTE_BASE} && "
            "(test -d tree/.git || git init -q tree) && cat > snapshot.bundle && "
            "cd tree && git fetch -q ../snapshot.bundle refs/mutants/remote && "
            "git checkout -q --force --detach FETCH_HEAD && git clean -fdq"
        )
        with open(bundle.name, "rb") as data:
            ssh(host, remote, stdin=data, check=True)
    finally:
        Path(bundle.name).unlink(missing_ok=True)


def test_threads_arg(test_threads: int) -> str:
    """The `--test-threads` argument nextest takes directly, without libtest's `--`."""
    return f"--test-threads={test_threads}"


# The `.cargo/mutants.toml` keys `check_test_config` accepts: those that pick which
# mutants run, their timeouts, and the test command's parts `test_command` builds.
# cargo-mutants honors all of them; the extra kinds' pass reads `exclude_globs`
# (which files its mutants land in), the timeout keys and the test command's parts.
# `exclude_re` it ignores without diverging — its mutants' names never match a
# cargo-mutants spelling, and `.cargo/extra_mutants.toml` holds its own exclusions —
# and `gitignore` it cannot see, its snapshot copies holding no gitignored file.
# `examine_globs` is not accepted: it would narrow cargo-mutants' run alone, so a
# file split sets it through `sized_config`'s `extra`, past this check.
TEST_CONFIG_KEYS = {
    "test_tool", "additional_cargo_args", "additional_cargo_test_args", "timeout_multiplier",
    "minimum_test_timeout", "gitignore", "exclude_globs", "exclude_re",
}
# Nextest's exit codes the runs read: tests ran and one failed, or the build failed.
TEST_RUN_FAILED = 100
BUILD_FAILED = 101


def check_test_config(config: dict) -> None:
    """Raises `ValueError` for a parsed `.cargo/mutants.toml` the two runners would not
    test alike: a `test_tool` other than nextest; a key outside `TEST_CONFIG_KEYS`
    (`features`, `profile`, `examine_globs` — a split's `sized_config` `extra`
    carries that one, past this check); or an `additional_cargo_test_args` entry
    other than `--test-threads=`, which `sized_config` overwrites and
    `test_command` drops. Both `sized_config` and `test_command` call it, so such
    an edit stops both runners until the key is handled in both."""
    tool = config.get("test_tool", "cargo")
    if tool != "nextest":
        raise ValueError(f"mutants.toml: unsupported test_tool {tool!r}")
    unknown = sorted(set(config) - TEST_CONFIG_KEYS)
    if unknown:
        raise ValueError(f"mutants.toml: keys the extra kinds' pass does not honor: {unknown}")
    dropped = [
        arg for arg in config.get("additional_cargo_test_args", [])
        if not arg.startswith("--test-threads=")
    ]
    if dropped:
        raise ValueError(
            "mutants.toml: additional_cargo_test_args entries the runs would "
            f"drop (only --test-threads= is kept): {dropped}"
        )


def test_command(crate: str, config: dict, test_threads: int) -> list[str]:
    """The command that tests `crate` the way a mutation run does: `cargo nextest run -p
    <crate>` plus `config`'s `additional_cargo_args` and `test_threads` threads,
    `config` being a parsed `.cargo/mutants.toml` that `check_test_config` accepts. The
    file's own `additional_cargo_test_args` is dropped, the same swap `sized_config`
    makes for cargo-mutants."""
    check_test_config(config)
    return [
        "cargo", "nextest", "run", "-p", crate,
        *config.get("additional_cargo_args", []),
        test_threads_arg(test_threads),
    ]


def sized_config(
    cpus: int, extra: dict | None = None, *, source: Path
) -> str:
    """`source`'s `.cargo/mutants.toml` (the run's copy, `snapshot_source`) as text,
    with each of the `JOBS` cargo processes (`--jobs`) given `cpus // JOBS`
    logical CPUs for test threads, so together they
    use every CPU without oversubscribing it. The scripts run cargo-mutants at a low
    priority instead of holding CPUs back (`below_normal`), so the machine
    stays usable. The remote's build cap (`--jobserver-tasks`) is
    cargo-mutants' own option, so `launch_remote` puts it on the command line:
    in `additional_cargo_args` it would reach `cargo nextest run`. `extra`
    sets more keys — a file split's `examine_globs` or `exclude_globs`; a list
    value extends the base file's list of that key rather than replacing it."""
    config = tomllib.loads((source / ".cargo/mutants.toml").read_text(encoding="utf-8"))
    check_test_config(config)
    half = max(1, cpus // JOBS)
    # The file's `test_tool` is nextest, which takes `--test-threads`
    # directly, without libtest's `--`.
    config["additional_cargo_test_args"] = [test_threads_arg(half)]
    for key, value in (extra or {}).items():
        base = config.get(key)
        config[key] = (
            base + value if isinstance(base, list) and isinstance(value, list) else value
        )
    lines = []
    for key, value in config.items():
        # A float (`timeout_multiplier`) dumps as a TOML float: `3.0`, never `3`.
        serializable = isinstance(value, (str, int, float, bool)) or (
            isinstance(value, list) and all(isinstance(item, str) for item in value)
        )
        if not serializable:
            raise TypeError(f"cannot write mutants.toml key {key!r}: {value!r}")
        # JSON escapes are valid TOML basic-string escapes.
        lines.append(f"{key} = {json.dumps(value)}")
    return "\n".join(lines) + "\n"


def remote_config(host: str, nproc: int, source: Path, extra: dict | None = None) -> None:
    """Write the remote's mutants.toml from `source`'s, sized for the remote's CPUs;
    `extra` carries a file split's `exclude_globs`."""
    ssh(host, f"cat > {REMOTE_BASE}/mutants.remote.toml",
        input=sized_config(nproc, extra, source=source),
        text=True, check=True)


def local_config(source: Path, extra: dict | None = None) -> Path:
    """Write `target/mutants.local.toml` from `source`'s, sized for this machine's CPUs,
    and return its path for `--config`; `extra` carries a file split's
    `examine_globs`."""
    path = ROOT / "target" / "mutants.local.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(".tmp")
    temp.write_text(
        sized_config(os.cpu_count() or 2, extra=extra, source=source), encoding="utf-8"
    )
    os.replace(temp, path)
    return path


def below_normal() -> dict:
    """`subprocess` keyword arguments that start cargo-mutants at a below-normal
    priority, which every process it starts inherits (cargo, rustc, the
    linker, the test binaries): the run takes every CPU cycle the user's own
    work leaves idle, and yields the rest. On Windows the class is inherited
    by children by definition; on Linux the nice value is."""
    if os.name == "nt":
        return {"creationflags": subprocess.BELOW_NORMAL_PRIORITY_CLASS}
    return {"preexec_fn": lambda: os.nice(10)}


def tmpdir_env() -> dict:
    """`os.environ` with `TMP`, `TEMP` and `TMPDIR` pointed at `LOCAL_TMPDIR`:
    Rust's `temp_dir` reads `TMP` then `TEMP` on Windows and `TMPDIR`
    elsewhere, so a run's copies of the tree and every cargo child's
    temporary files share the one folder the antivirus exclusion covers."""
    folder = str(LOCAL_TMPDIR)
    return {**os.environ, "TMP": folder, "TEMP": folder, "TMPDIR": folder}


def local_mutants(
    args: list[str], source: Path, config: Path | None = None
) -> subprocess.CompletedProcess:
    """Runs `cargo mutants <args> --jobs <JOBS>` here on `source`, the run's copy of
    its snapshot (`snapshot_source`), sized by `local_config` and at `below_normal`
    priority, its copies of the tree in `LOCAL_TMPDIR` (`tmpdir_env`).
    Its results land in the root's `mutants.out` unless `args`
    names an `--output`. `config` is a config a file split already wrote (its
    `examine_globs` must not be regenerated away)."""
    LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    if config is None:
        config = local_config(source)
    output = [] if "--output" in args else ["--output", str(ROOT)]
    return subprocess.run(
        [
            "cargo", "mutants", *args, "--dir", str(source), *output,
            "--jobs", str(JOBS), "--config", str(config),
        ],
        cwd=source,
        env=tmpdir_env(),
        **below_normal(),
    )


# The Windows status of a process that lost its console: it never starts.
# A killed-build sign (`KILLED_BUILD_SIGNS`), and in a test phase's output the
# mark of a test binary that ran nothing.
LOST_CONSOLE = "0xc0000142"
# What a build killed or starved for reasons outside the mutant leaves in its log, lowercase:
# rustc killed by the OOM killer, the linker killed by it (gcc's `collect2`), LLVM or rustc
# out of memory, a Windows process that lost its console, a link that failed (a mutant
# changes Rust code, which compiled; a Windows SDK update had removed the system libraries),
# and a full disk on Linux or Windows.
KILLED_BUILD_SIGNS = (
    "sigkill", "signal 9 [killed]", "llvm error: out of memory", "memory allocation of",
    LOST_CONSOLE, "error: linking with",
    "no space left on device", "there is not enough space on the disk",
)


def unviable_build(code: int | None, out: str) -> bool:
    """Whether a failed build is the mutant's own: `True` only when cargo reported
    a crate that could not compile — exit `BUILD_FAILED` with "could not compile"
    in the output — and the log shows no sign the build was killed or starved for
    reasons outside the mutant (`KILLED_BUILD_SIGNS`: a killed rustc, a full
    disk). Anything else — another code, a build script's failure, cargo itself
    dying — is `False`, meaning the mutant went untested."""
    lowered = out.lower()
    return (
        code == BUILD_FAILED
        and "could not compile" in lowered
        and not any(sign in lowered for sign in KILLED_BUILD_SIGNS)
    )


def caught_by_tests(outcome: dict, out_dir: Path) -> bool:
    """Whether a mutant's `CaughtMutant` outcome is a catch: its test phase ended
    as nextest ends a run in which tests failed (exit code 100,
    `{"Failure": 100}` in outcomes.json), and its log, when it is here, shows no
    test binary that lost its console (`LOST_CONSOLE`) — one ran nothing, so it
    caught nothing. `out_dir` is the `mutants.out` the outcome's `log_path` is
    read under; a caught mutant's log is not always there (`REMOTE_FETCH_LIST`
    leaves them behind), and a missing log cannot clear the catch."""
    if not any(
        phase["phase"] == "Test"
        and phase.get("process_status") == {"Failure": TEST_RUN_FAILED}
        for phase in outcome.get("phase_results", [])
    ):
        return False
    log_path = outcome.get("log_path")
    return not (
        log_path
        and (out_dir / log_path).exists()
        and LOST_CONSOLE
        in (out_dir / log_path).read_text(encoding="utf-8", errors="replace").lower()
    )


def killed_builds(out_dir: Path) -> list[str]:
    """The mutants under `out_dir` (a `mutants.out`) that went untested: those
    cargo-mutants reported unviable although their build failed outside the
    mutant's own compilation — killed or starved for reasons outside it, or
    failed another way (`unviable_build` rules it) — and those it filed as a
    `Failure` (its cargo process ended by a signal), which no list holds, so
    the counts would come up short without a word. A test killed for memory is
    a caught mutant, as intended: the mutant made the test blow up. The remote
    half's memory cap kills a rustc that outgrows it, and cargo-mutants files
    the failed build as unviable, so the mutant goes untested without a miss or
    an error: at 3.z, 18 of the remote half's 19 mutants, eframe's dependency
    tree outgrowing 6 GiB. Windows has no OOM killer: a rustc out of memory
    there aborts with "memory allocation of N bytes failed". A build that lost
    its console fails to start with 0xC0000142 (4.y-conv: 273 of 274 mutants
    filed unviable that way), and a full disk's write fails either OS.

    Also those it filed as caught although nothing proves the tests caught it
    (`caught_by_tests` reads the outcome and the log the same way here):
    cargo-mutants counts any nonzero exit of the test phase as a catch, and
    nextest exits 100 only when tests failed; any other code means its run
    broke before or around the tests, as a process that lost its console
    would. A catch by that code is untested too when the log shows a test
    binary that never started (`LOST_CONSOLE`): it ran nothing, so it caught
    nothing."""
    path = out_dir / "outcomes.json"
    if not path.exists():
        return []
    killed = []
    for outcome in json.loads(path.read_text(encoding="utf-8"))["outcomes"]:
        if outcome["summary"] == "Failure" and isinstance(outcome["scenario"], dict):
            killed.append(outcome["scenario"]["Mutant"]["name"])
            continue
        if outcome["summary"] == "CaughtMutant":
            if not caught_by_tests(outcome, out_dir):
                killed.append(outcome["scenario"]["Mutant"]["name"])
            continue
        if outcome["summary"] != "Unviable" or not outcome.get("log_path"):
            continue
        log = (out_dir / outcome["log_path"]).read_text(encoding="utf-8", errors="replace")
        build = next(
            (
                phase.get("process_status")
                for phase in outcome.get("phase_results", [])
                if phase["phase"] == "Build"
            ),
            None,
        )
        code = build.get("Failure") if isinstance(build, dict) else None
        if not unviable_build(code, log):
            killed.append(outcome["scenario"]["Mutant"]["name"])
    return killed


def local_mutants_version() -> str:
    return subprocess.run(
        ["cargo", "mutants", "--version"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.strip()


def remote_run_state(host: str) -> str | None:
    """`none` (no run, or the last one collected), `running`, `finished` (not yet
    collected) or `died` (a pid, no exit file and the unit gone: killed, or the
    host rebooted); None when the host could not be reached. `running` asks
    systemd for the unit, not `kill -0` for the pid: after a reboot the pid may
    belong to another process, which would read as a half running forever."""
    # A timeout as well as the keepalives: one poll once hung for five
    # hours with the link up, so the keepalives never fired, and the finished
    # half sat uncollected; the timeout turns such a hang into one failed poll.
    try:
        probe = ssh(
            host,
            f"cd {REMOTE_RUN} 2>/dev/null || {{ echo none; exit 0; }}; "
            "if [ -f collected ]; then echo none; "
            "elif [ -f exit ]; then echo finished; "
            f"elif systemctl is-active --quiet {REMOTE_UNIT}; then echo running; "
            # Again: a half that writes `exit` and ends between the first test and
            # systemd's answer finished.
            "elif [ -f exit ]; then echo finished; "
            "elif [ -f pid ]; then echo died; "
            "else echo none; fi",
            capture_output=True, text=True, timeout=POLL_CALL_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return None
    state = probe.stdout.strip()
    if probe.returncode != 0 or state not in ("none", "running", "finished", "died"):
        return None
    return state


def remote_refusal(host: str) -> str | None:
    """Why a new run that uses the remote cannot start, or None — every caller
    does (`local_refusal` asks this tree's launch record alone). It refuses
    while a remote half is running or finished but uncollected, whose
    collection summarizes the run's `mutants.out` with it; while the host
    cannot be reached or `just gates-remote` runs there, since each rewrites
    the remote clone under the other; and below the disk's free floor
    (`disk_refusal`, after `remote_disk_use` has emptied the tests' leftovers)."""
    state = remote_run_state(host)
    if state is None:
        return f"cannot reach {host}"
    if state != "none":
        status = {
            "running": "is still running",
            "finished": "has finished but is not collected",
            "died": "stopped before it finished",
        }[state]
        return (
            f"the remote half of an earlier run {status}: `just mutants-collect` "
            "fetches it, then start the new run"
        )
    gates = ssh(host, f"systemctl is-active --quiet {GATES_UNIT}").returncode
    if gates == 255:
        return f"cannot reach {host}"
    if gates == 0:
        return f"`just gates-remote` is running on {host}: start the new run when it ends"
    return disk_refusal(*remote_disk_use(host))


def remote_pending() -> Path:
    """`.tmp/mutants-remote-pending`, holding a JSON record of a remote half
    launched from this tree — the host, whether the run has a local half,
    whether the remote ran the extra pass and whether the run is extras-only —
    removed when it is fetched.
    Resolved under `ROOT` at call time, as `extra_run_record` is."""
    return ROOT / ".tmp" / "mutants-remote-pending"


def pending_record() -> dict | None:
    """The `remote_pending` record as a dict, or None when there is none or it
    is not one this version wrote: before the record carried fields it held the
    host's name alone, which `local_refusal` and `collect` treat as a split's
    with a local half."""
    pending = remote_pending()
    if not pending.exists():
        return None
    try:
        record = json.loads(pending.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    return record if isinstance(record, dict) else None


def local_refusal() -> str | None:
    """Why a run on this machine alone cannot start, or None. Only this tree's
    own record of an uncollected remote half that has a local half stops it
    (`remote_pending`): that half's collection summarizes the local
    `mutants.out` the new run would set aside. A half with none (a whole run, a
    `REMOTE_ONLY` diff) reads nothing of it, and a half launched from another
    tree is collected there."""
    pending = remote_pending()
    if not pending.exists():
        return None
    record = pending_record()
    if record is None:
        # Whether the run had a local half is unknown, so it refuses as one's.
        local_half, launched_on = True, pending.read_text(encoding="utf-8").strip()
    else:
        local_half = record.get("local_half", True)
        launched_on = record.get("host", "?")
    if not local_half:
        return None
    return (
        f"the remote half launched on {launched_on} is not collected: `just "
        f"mutants-collect` (with STUDIO_MUTANTS_REMOTE={launched_on}) fetches it, "
        f"then start the new run (the record: {pending})"
    )


def remote_disk_use(host: str) -> tuple[int, int]:
    """Empties `REMOTE_TMPDIR`, then returns the bytes `REMOTE_BASE` holds and the
    bytes free on its disk. Called only once `remote_refusal` knows no half and no
    gates run there, so all the folder holds is leftovers: the scratch folders of
    tests that remove theirs at their end, which a caught mutant's panic skips
    (16,786 of them on 2026-10-22), and the tree copies of a killed half."""
    out = ssh(
        host,
        f'd="{REMOTE_TMPDIR}"; mkdir -p "$d" && '
        'find "$d" -mindepth 1 -maxdepth 1 -exec rm -rf -- {} + && '
        f"du -sk {REMOTE_BASE} | cut -f1 && "
        f"df -Pk {REMOTE_BASE} | awk 'NR == 2 {{print $4}}'",
        capture_output=True, text=True, check=True,
    ).stdout
    used, free = out.split()
    return int(used) * 1024, int(free) * 1024


def disk_refusal(used: int, free: int) -> str | None:
    """A refusal when `REMOTE_BASE`'s disk has less than `REMOTE_FREE_FLOOR` free
    (`used`, what the folder holds, and `free` in bytes, `remote_disk_use`), naming
    the cleanup and who to ask when it is not enough; None, after printing both
    figures, otherwise."""
    figures = (
        f"{used / 2**30:.1f} GiB in {REMOTE_BASE}, {free / 2**30:.1f} GiB free "
        f"(floor {REMOTE_FREE_FLOOR / 2**30:.0f} GiB)"
    )
    if free < REMOTE_FREE_FLOOR:
        return (
            f"remote disk: {figures}: below the floor. RUNBOOK \"The scratch folder\"'s remote "
            "cleanup frees the clone's build cache; if that cannot lift the floor, ask the "
            "maintainer before starting the run"
        )
    print(f"remote disk: {figures}", flush=True)
    return None


def remote_memory_max(host: str) -> int:
    """The MemoryMax, in bytes, of a unit started on `host` now: its available
    memory less `REMOTE_MEMORY_RESERVE`. Printed, since it is the figure a
    build killed at the cap is read against."""
    meminfo = ssh(
        host, "awk '/^MemAvailable:/ {print $2}' /proc/meminfo",
        capture_output=True, text=True, check=True,
    ).stdout
    available = int(meminfo) * 1024
    cap = available - REMOTE_MEMORY_RESERVE
    if cap <= 0:
        raise RuntimeError(
            f"only {available / 2**30:.2f} GiB available on {host}: no memory cap to start under"
        )
    print(
        f"remote memory cap: {cap / 2**30:.2f} GiB ({available / 2**30:.2f} GiB available "
        f"on {host}, less {REMOTE_MEMORY_RESERVE / 2**30:.0f} GiB for the host)",
        flush=True,
    )
    return cap


class RemoteDied(RuntimeError):
    """The remote half stopped without an exit code."""


class Refused(RuntimeError):
    """A run that cannot start (`remote_refusal`, `local_refusal`, a cargo-mutants version
    mismatch), raised before it touches any result: a caller that counted the refusal as a
    finished run filed the last run's `mutants.out` as this one's."""


def remote_selection(selection: list[str]) -> str:
    """The remote half's selection arguments: a `--in-diff` selection reads the
    uploaded copy of the diff (`run/in.diff`) and keeps what follows it, such as
    the `-p <crate>` of a per-crate diff run, which would otherwise test every
    crate's diff mutants on the remote."""
    if selection[0] == "--in-diff":
        return " ".join(['--in-diff "$HOME/studio-mutants/run/in.diff"', *selection[2:]])
    return " ".join(selection)


def launch_remote(
    host: str, selection: list[str], whole: bool = False, *,
    nproc: int, local_half: bool = True, extra: bool = False,
    extra_only: bool = False, kinds: Iterable[str] = (),
) -> None:
    """Writes `run/job.sh` (and `run/in.diff` for a `--in-diff` selection,
    the same diff the local half reads) and starts it as the `REMOTE_UNIT`
    transient service (`sudo -n systemd-run`): shard 1/2 of the selection, or
    all of it when `whole` (a whole-crate run, `REMOTE_ONLY`, and a whole-crate
    file split, whose shares the remote's `exclude_globs` carry instead of a
    shard). Its cargo-mutants line caps the jobserver at
    `REMOTE_JOBSERVER_TASKS` (`nproc` while unset), the build cap the config
    cannot carry (`sized_config`). With `extra` (a whole run) the job first
    lists the crate's mutants and builds the coverage map from them —
    which tests cover each listed mutant's lines, the extra kinds' included
    (`coverage_map.py --mutants --extra`, logging `coverage map exit
    <code>`) — then runs the extra kinds on it (`--remote-extra --selection`,
    logging `extra pass exit <code>`), then cargo-mutants whatever the
    codes: `run/exit` stays cargo-mutants'. cargo-mutants is started as
    `cargo-mutants mutants` — `cargo mutants` would overwrite `CARGO` for the
    subcommand — with the stand-in `select_tests.py` as `CARGO` when the map's
    `run/selection.json` exists, so each mutant runs the tests covering its
    lines; without the file the full suite runs, a failed map costing time,
    never a wrong outcome. An extras-only run (`extra_only`) writes the map
    (`--extra` alone, no cargo-mutants listing) and
    the pass, `kinds` naming the kinds after the crate — `run/exit` is
    the pass's own code, and the record carries `extra_only` for `collect`.
    The service is
    detached from this ssh session
    by construction: the unit is memory-capped (`MemoryMax`, no swap) and
    CPU-idle so Fluxer's production services on the host win, and
    `OOMPolicy=continue` keeps cargo-mutants alive past an in-unit OOM kill.
    `run/pid` is the service's MainPID and `run/memory_max` its cap
    (`remote_memory_max`); a failed `systemd-run` fails the launch (no
    fallback). The launch records the half in `remote_pending` before the unit
    starts: `local_refusal` and `collect` read the record."""
    remote_args = remote_selection(selection)
    shard = "" if whole else "--shard 1/2 --sharding round-robin "
    selection_map = ""
    if extra or extra_only:
        # The map is built from the listing the run tests: same -p, same
        # --config; `--extra` adds the extra kinds' mutants, whose pass then
        # reads it through --selection. Its failure leaves no selection.json,
        # so the stand-in stays off and every mutant runs the full suite.
        listing = ""
        if not extra_only:
            listing = (
                f"cargo-mutants mutants {remote_args} --list --json "
                "--config ../mutants.remote.toml > ../run/mutants.json\n"
            )
        selection_map = (
            listing
            + "nice -n 19 ionice -c3 "
            + f"python3 scripts/mutation/coverage_map.py {selection[1]} "
            + "../run/selection.json"
            + ("" if extra_only else " --mutants ../run/mutants.json")
            + " --extra\n"
            'echo "coverage map exit $?"\n'
        )
    extra_pass = ""
    if extra or extra_only:
        # The selection is ["-p", crate].
        named = "".join(f" {kind}" for kind in kinds)
        extra_pass = (
            "nice -n 19 ionice -c3 "
            f"python3 scripts/mutation/mutants.py --remote-extra "
            f"{selection[1]}{named} --selection ../run/selection.json\n"
        )
        if not extra_only:
            extra_pass += 'echo "extra pass exit $?"\n'
    mutants_line = ""
    if not extra_only:
        cargo_args = (
            f"{remote_args} --jobs {JOBS} "
            f"--jobserver-tasks {REMOTE_JOBSERVER_TASKS or nproc} "
            f"{shard}--config ../mutants.remote.toml"
        )
        if extra:
            # `cargo mutants` would overwrite CARGO for the subcommand, so the
            # binary is started directly; CARGO is set only with a selection.
            mutants_line = (
                "if [ -f ../run/selection.json ]; then\n"
                "CARGO=$PWD/scripts/mutation/select_tests.py "
                "UTW_REAL_CARGO=$(command -v cargo) UTW_MUTANTS_SOURCE=$PWD "
                "UTW_MUTANTS_SELECTION=$PWD/../run/selection.json "
                f"nice -n 19 ionice -c3 cargo-mutants mutants {cargo_args}\n"
                "else\n"
                f"nice -n 19 ionice -c3 cargo-mutants mutants {cargo_args}\n"
                "fi\n"
            )
        else:
            mutants_line = (
                f"nice -n 19 ionice -c3 cargo mutants {cargo_args}\n"
            )
    job = (
        "#!/bin/bash\n"
        "exec > ../run/log 2>&1\n"
        "start=$(date +%s)\n"
        # On disk, not the tmpfs `/tmp`: see REMOTE_TMPDIR.
        f'export TMPDIR="{REMOTE_TMPDIR}" && mkdir -p "$TMPDIR"\n'
        f". ~/.cargo/env && {selection_map}{extra_pass}{mutants_line}"
        "code=$?\n"
        # `memory.peak` of this unit's cgroup tells whether the cap is tight;
        # `exit` stays the last file written: pollers treat it as finished.
        'peak="/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)/memory.peak"\n'
        '[ -f "$peak" ] && cp "$peak" ../run/memory_peak\n'
        'echo "$code $(( $(date +%s) - start ))" > ../run/exit\n'
    )
    ssh(
        host,
        f"mkdir -p {REMOTE_RUN} && "
        f"rm -f {REMOTE_RUN}/pid {REMOTE_RUN}/log {REMOTE_RUN}/exit "
        f"{REMOTE_RUN}/collected {REMOTE_RUN}/memory_peak {REMOTE_RUN}/memory_max "
        # A whole run's mutant listing and its selection file are this run's:
        # a failed listing must leave none, or an old map would pair with it.
        f"{REMOTE_RUN}/in.diff {REMOTE_RUN}/mutants.json "
        f"{REMOTE_RUN}/selection.json && "
        # The tree's `mutants.out` and `extra_mutants.out` are gitignored, so
        # the transfer keeps the last run's; a half that fails before
        # cargo-mutants starts must leave none, or the fetch would count the
        # last run's results as this one's.
        f"rm -rf {REMOTE_BASE}/tree/mutants.out {REMOTE_BASE}/tree/extra_mutants.out && "
        f"cat > {REMOTE_RUN}/job.sh",
        # Bytes, not text: `text=True` translates \n to \r\n on Windows and a
        # CRLF in the script makes bash and cargo-mutants fail.
        input=job.encode("utf-8"),
        check=True,
    )
    if selection[0] == "--in-diff":
        # Bytes again: the diff must stay LF for the remote cargo-mutants.
        ssh(
            host,
            f"cat > {REMOTE_RUN}/in.diff",
            input=Path(selection[1]).read_bytes(),
            check=True,
        )
    memory_max = remote_memory_max(host)
    # Before the unit starts: a launch call that fails after starting it still leaves a half
    # to collect. `collect` removes a marker no half answers.
    pending = remote_pending()
    pending.parent.mkdir(parents=True, exist_ok=True)
    record = {"host": host, "local_half": local_half, "extra": extra or extra_only}
    if extra_only:
        record["extra_only"] = True
    pending.write_text(json.dumps(record) + "\n", encoding="utf-8")
    ssh(
        host,
        f"echo {memory_max} > {REMOTE_RUN}/memory_max && "
        f"sudo -n systemd-run --unit={REMOTE_UNIT} --collect --uid=debian "
        f"-p MemoryMax={memory_max} -p MemorySwapMax=0 -p OOMPolicy=continue "
        '-p CPUWeight=idle --working-directory="$HOME/studio-mutants/tree" '
        f'/bin/bash "$HOME/studio-mutants/run/job.sh" && '
        f"systemctl show -p MainPID --value {REMOTE_UNIT} > {REMOTE_RUN}/pid",
        check=True,
    )


def wait_remote(host: str) -> tuple[int, float]:
    """Polls until the remote half finishes; returns its exit code and its own
    seconds. A failed poll is retried; only MAX_POLL_FAILURES in a row give up,
    and the remote half is still collectable after that."""
    failures = 0
    announced = False
    while True:
        state = remote_run_state(host)
        if state == "finished":
            exit_line = ssh(host, f"cat {REMOTE_RUN}/exit", capture_output=True, text=True)
            if exit_line.returncode == 0:
                code, seconds = exit_line.stdout.split()
                return int(code), float(seconds)
            state = None  # the link dropped between the two calls: one failed poll
        if state is None:
            failures += 1
            print(f"remote poll failed ({failures} in a row), retrying", flush=True)
            if failures >= MAX_POLL_FAILURES:
                raise RuntimeError(
                    f"could not reach {host} for {failures} polls; the remote half keeps "
                    "running there: collect it with `just mutants-collect`"
                )
        elif state == "died":
            raise RemoteDied(
                "the remote half stopped without an exit code (killed, or the host "
                "rebooted): `just mutants-collect` fetches its partial results"
            )
        elif state == "none":
            raise RuntimeError("no remote half is recorded on the host (none was started, "
                               "or its run folder was removed)")
        else:
            failures = 0
            if not announced:
                print("waiting for the remote half", flush=True)
                announced = True
        time.sleep(POLL_SECONDS)


def upload_and_launch(
    host: str, selection: list[str], tree: str, source: Path,
    remote_head: str | None, remote_tree: str | None, nproc: int,
    config_extra: dict | None = None, *,
    whole: bool = False, local_half: bool, extra: bool,
    extra_only: bool = False, kinds: Iterable[str] = (),
) -> None:
    """The remote half's launch, shared by a split (`run_split`, in the local
    half's shadow thread), a whole run and an extras-only one: the transfer
    when the clone's tree differs from the run's snapshot, the remote config
    (`config_extra`: a file split's `exclude_globs`) and the unit's start
    (`launch_remote`, `extra_only` and `kinds` as it takes them)."""
    if remote_tree == tree:
        print("remote tree up to date", flush=True)
    else:
        transfer(host, remote_head)
    remote_config(host, nproc, source, config_extra)
    launch_remote(
        host, selection, whole=whole, nproc=nproc, local_half=local_half,
        extra=extra, extra_only=extra_only, kinds=kinds,
    )
    print("remote half launched", flush=True)


def run_split(
    selection: list[str], host: str, tree: str, source: Path, remote_head: str | None,
    remote_tree: str | None, nproc: int, local: bool = True,
    local_files: set[str] | None = None,
) -> tuple[int, int, float | None, float]:
    """The local half starts at once on `source`, the copy of the snapshot `tree`, in the
    foreground with console output as
    today; a thread transfers the snapshot, writes the remote config and launches
    the remote half, so the transfer counts against the remote side only.
    `selection` is the mutant-selection arguments (`["-p", crate]` or
    `["--in-diff", path, "-p", crate]`); the remote reads an uploaded copy of a diff.
    Without `local` (`REMOTE_ONLY`, a diff run) the remote runs the whole
    selection and nothing runs here. With `local_files` (a whole-crate file
    split) neither
    half takes a `--shard`: the local config examines those files and the
    remote's excludes them, so each tests a complementary set. Returns the
    (local, remote) codes and seconds: the local
    half's from the common start (None without one), the remote half's from its
    own launch."""
    start = time.monotonic()
    remote: dict = {}
    whole = not local
    config_extra = (
        {"exclude_globs": sorted(local_files)} if local_files is not None else None
    )

    def remote_half() -> None:
        try:
            # A file split's remote runs with no `--shard`: its config's
            # exclude_globs already name the local half's share.
            upload_and_launch(
                host, selection, tree, source, remote_head, remote_tree, nproc,
                config_extra,
                whole=whole or local_files is not None,
                local_half=local, extra=False,
            )
        except Exception as error:
            # Re-raised in the main thread once the local half is done: an
            # uncaught one would end the thread silently and leave the main
            # thread waiting for a half that was never launched.
            remote["error"] = error

    if not local:
        remote_half()
        if "error" in remote:
            raise RuntimeError("the remote run failed before running") from remote["error"]
        remote_code, remote_seconds = wait_remote(host)
        return 0, remote_code, None, remote_seconds
    thread = threading.Thread(target=remote_half)
    thread.start()
    if local_files is None:
        local_run = local_mutants(
            [*selection, "--shard", "0/2", "--sharding", "round-robin"], source
        )
    else:
        local_run = local_mutants(
            list(selection),
            source,
            config=local_config(source, extra={"examine_globs": sorted(local_files)}),
        )
    local_seconds = time.monotonic() - start
    thread.join()
    if "error" in remote:
        share = "shard 0/2" if local_files is None else "its share of the files"
        raise RuntimeError(
            f"the remote half failed to launch; the local half covered {share} only. The "
            "launch can fail after starting the half: if one runs, `just mutants-collect` "
            "fetches it; otherwise rerun"
        ) from remote["error"]
    remote_code, remote_seconds = wait_remote(host)
    return local_run.returncode, remote_code, local_seconds, remote_seconds


# Run by the host's python3 in its clone, its stdout the list `fetch_remote`'s tar packs:
# every file of `mutants.out` but a caught mutant's log, and every file of a
# whole run's `extra_mutants.out` (the extra pass's `run.json`, `missed_<crate>.txt`
# and outcome tsvs are all small). Those logs are never read here
# (`killed_builds` reads the unviable ones', the triage the survivors'), and a test that
# fails by printing big values fills them: pes_savefile's 900 caught logs came to 6.1 GB
# of its remote half's 6.1, which the link fetched at 41 KB/s. An outcomes.json this cannot
# read (missing, cut short by a half that died mid-write, another cargo-mutants' shape)
# keeps every log: a program that failed would fail every retry of the fetch, and the
# uncollected half would refuse every later run.
REMOTE_FETCH_LIST = """\
import json, os
caught = set()
try:
    with open("mutants.out/outcomes.json", encoding="utf-8") as f:
        for outcome in json.load(f)["outcomes"]:
            # caught_by_tests' exit-code half: a catch whose tests never
            # failed keeps its log (its lost-console half is Windows-only).
            tested = any(
                phase["phase"] == "Test" and phase.get("process_status") == {"Failure": 100}
                for phase in outcome.get("phase_results", [])
            )
            if outcome["summary"] == "CaughtMutant" and outcome.get("log_path") and tested:
                caught.add(os.path.normpath(os.path.join("mutants.out", outcome["log_path"])))
except Exception:
    caught = set()
for base in ("mutants.out", "extra_mutants.out"):
    for folder, _, files in os.walk(base):
        for name in sorted(files):
            path = os.path.normpath(os.path.join(folder, name))
            if path not in caught:
                print(path)
"""


def fetch_remote(host: str, partial: bool = False, *, extra_only: bool = False) -> None:
    """Copies the remote half's `mutants.out` (all but the caught mutants' logs:
    `REMOTE_FETCH_LIST`) and, for a whole run, its `extra_mutants.out`, plus the
    console log into `mutants.out/remote/`, emptied
    first so nothing of an earlier fetch is counted as this run's, then marks
    the run collected. A finished half with
    no `mutants.out` fails the fetch; a died one (`partial`) may have none.
    An extras-only run's fetch (`extra_only`) is the pass's `extra_mutants.out`
    alone, tarred whole into `extra_mutants.out/remote/`: the pass keeps no big
    logs to filter out, and the shared fetch's `[ -d mutants.out ]` guard would
    read an extras-only run's absent `mutants.out` as a half with no results."""
    name = "extra_mutants.out" if extra_only else "mutants.out"
    out = ROOT / name / "remote"
    LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    # The archive goes to a file, not memory: a fetch measured 11 GiB, and capturing it held
    # it twice (the captured bytes, then the reader over them).
    with tempfile.TemporaryFile(dir=LOCAL_TMPDIR) as archive:
        # Exit 3 is the half's own answer, no results folder; any other failure (ssh's 255, a
        # broken archive) is the transfer's, and the half stays uncollected for a retry.
        if extra_only:
            members = f"tar -cf - {name}"
            fed = {}
        else:
            members = "python3 - | tar -cf - --verbatim-files-from -T -"
            fed = {"input": REMOTE_FETCH_LIST.encode()}
        fetched = ssh(
            host,
            f"cd {REMOTE_BASE}/tree && {{ [ -d {name} ] || exit 3; }} && "
            f"set -o pipefail && {members}",
            **fed, stdout=archive, stderr=subprocess.PIPE,
        )
        absent = fetched.returncode == 3
        size = archive.seek(0, os.SEEK_END)
        if not absent and (fetched.returncode != 0 or not size):
            raise RuntimeError(
                f"fetching the remote half failed (exit {fetched.returncode}); it stays "
                "uncollected: retry with `just mutants-collect`"
            )
        if out.exists():
            remove_tree(out)
        out.mkdir(parents=True)
        if not absent:
            archive.seek(0)
            with tarfile.open(fileobj=archive, mode="r|") as tar:
                tar.extractall(out, filter="data")
        elif partial:
            print(f"warning: the remote half left no {name}", file=sys.stderr)
        else:
            # A collect on a tree whose pending record is missing or says
            # otherwise takes this path for an extras-only half too: the `rm
            # -r run` hint below would delete that half's results, so the
            # pass's folder is probed first.
            if not extra_only and ssh(
                host, f"cd {REMOTE_BASE}/tree && [ -d extra_mutants.out ]"
            ).returncode == 0:
                # A whole run whose cargo-mutants never started leaves the
                # same folders, and only its log tells the two apart.
                raise RuntimeError(
                    "the remote half left extra_mutants.out and no mutants.out: if "
                    "it is an extras-only run, collect it from the tree that "
                    "launched it; if its log "
                    f"({REMOTE_RUN}/log on {host}) ends with cargo-mutants' error, "
                    "it is a whole run whose cargo-mutants never started: its "
                    f"extra pass is in {REMOTE_BASE}/tree/extra_mutants.out, and "
                    f"`rm -r {REMOTE_RUN}` there frees the host"
                )
            raise RuntimeError(
                f"the remote half finished but left no {name}: its log is "
                f"{REMOTE_RUN}/log on {host}; once read, `rm -r {REMOTE_RUN}` there frees "
                "the host for the next run"
            )
    log = ssh(host, f"cat {REMOTE_RUN}/log 2>/dev/null || true", capture_output=True, check=True)
    (out / "remote_console.txt").write_bytes(log.stdout)
    peak, cap = ssh(
        host,
        f'for f in memory_peak memory_max; do echo "$(cat {REMOTE_RUN}/$f 2>/dev/null)"; done',
        capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    if peak and cap:
        print(
            f"remote memory peak: {int(peak) / 2**30:.2f} GiB "
            f"(cap {int(cap) / 2**30:.2f} GiB)"
        )
    ssh(host, f"touch {REMOTE_RUN}/collected", check=True)
    remote_pending().unlink(missing_ok=True)


def count_lines(path: Path) -> int:
    return len(read_lines(path))


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8").splitlines() if path.exists() else []


def timeout_lines(out_dir: Path) -> list[str]:
    """The mutants under `out_dir` (a `mutants.out`) whose tests timed out, as `timeout.txt`'s
    lines tagged `[timeout]` after the location: listed with the survivors in `missed_all.txt`,
    the file triage reads, since an unexplained timeout is rerun before it counts as caught."""
    tagged = []
    for line in read_lines(out_dir / "timeout.txt"):
        location, _, what = line.partition(": ")
        tagged.append(f"{location}: [timeout] {what}")
    return tagged


def never_ran(out_dir: Path) -> tuple[int, int] | None:
    """The (ran, selected) mutant counts of the run under `out_dir` (a `mutants.out`) when it
    stopped before testing its whole selection, else None: cargo-mutants writes the
    selection (`mutants.json`) before testing and its outcomes as it goes, so a half stopped
    midway leaves lists that look like a complete, shorter run."""
    selection = out_dir / "mutants.json"
    outcomes = out_dir / "outcomes.json"
    if not selection.exists():
        return None
    selected = len(json.loads(selection.read_text(encoding="utf-8")))
    # A half stopped during its baseline has written its selection and no outcome yet.
    ran = sum(
        isinstance(outcome["scenario"], dict)
        for outcome in json.loads(outcomes.read_text(encoding="utf-8"))["outcomes"]
    ) if outcomes.exists() else 0
    return (ran, selected) if ran < selected else None


def run_code(codes: Iterable[int]) -> int:
    """The exit code a run's results combine to: this script's 1 (mutants went
    untested) outranks cargo-mutants' 2 (missed) and 3 (timed out) — a survivor
    is in `missed_all.txt`, an untested mutant is in no list. A baseline's 4
    and a split coverage gap's 5 stay on top."""
    top = max(codes)
    return 1 if 1 in codes and top < 4 else top


def recheck_windows(missed: list[str]) -> set[str]:
    """Rerun the remote half's `missed` survivors (missed.txt lines) in one
    cargo-mutants run on this machine, the tests that catch them being
    `#[cfg(windows)]` (WINDOWS_ONLY_TESTS) so the Linux remote cannot, and
    return the lines it caught: a `CaughtMutant` outcome counts only as
    `caught_by_tests` proves it (failed tests, and no lost-console log),
    cargo-mutants filing any nonzero test exit in `caught.txt`. `--re` names
    each mutant escaped and anchored,
    `-p` the crates they live in, `--output` a folder of its own under
    `mutants.out/` so the local half's results stay. It runs on the tree the
    remote half ran (`REMOTE_REF`), whose lines the survivors name. A run that
    cannot start or whose baseline fails (exit 4) is reported and returns empty,
    leaving every survivor in the list."""
    tree = git(
        "rev-parse", f"{REMOTE_REF}^{{tree}}", check=True, capture_output=True, text=True
    ).stdout.strip()
    crates = sorted({mutant_crate(line.split(":", 1)[0]) for line in missed})
    out = ROOT / "mutants.out" / "recheck"
    # An earlier recheck's results are not this one's: a run that fails before
    # cargo-mutants rotates them away would read them otherwise.
    if out.exists():
        remove_tree(out)
    pattern = "^(" + "|".join(re.escape(line) for line in missed) + ")$"
    args = [arg for crate in crates for arg in ("-p", crate)]
    args += ["--re", pattern, "--output", str(out)]
    print(f"rechecking {len(missed)} Windows-only mutants on this machine", flush=True)
    with snapshot_source(tree) as source:
        run = local_mutants(args, source)
    results = out / "mutants.out"
    outcomes_path = results / "outcomes.json"
    if run.returncode == 4 or not outcomes_path.exists():
        print(
            f"the recheck did not finish (exit {run.returncode}); "
            f"the {len(missed)} mutants stay missed"
        )
        return set()
    caught = {
        outcome["scenario"]["Mutant"]["name"]
        for outcome in json.loads(outcomes_path.read_text(encoding="utf-8"))["outcomes"]
        if outcome["summary"] == "CaughtMutant" and caught_by_tests(outcome, results)
    }
    return caught & set(missed)


def recheck_windows_extra(missed: list[str]) -> tuple[set[str], int]:
    """`recheck_windows` for the extra kinds' survivors (`WINDOWS_ONLY_EXTRA`):
    one `extra_mutants.run_crate` per crate the `missed` lines name, `only`
    restricting it to just them, on the tree the remote pass ran
    (`REMOTE_REF`), whose lines they name. Returns (the lines caught — a line
    the run still lists, missed or tagged `[timeout]`/`[untested]`, is not
    one — and how many mutants went untested, which fails the run as the
    extra pass's own do; a line the run never reached counts there too). A
    crate whose rerun fails, or finds a mutant unviable, catches nothing."""
    tree = git(
        "rev-parse", f"{REMOTE_REF}^{{tree}}", check=True, capture_output=True, text=True
    ).stdout.strip()
    crates: dict[str, set[str]] = {}
    for line in missed:
        crates.setdefault(mutant_crate(line.split(":", 1)[0]), set()).add(line)
    import extra_mutants  # deferred: extra_mutants imports this module

    untested = 0
    returned: set[str] = set()
    for crate in sorted(crates):
        lines = crates[crate]
        try:
            run_missed, counts, _ = extra_mutants.run_crate(
                crate, list(EXTRA_KINDS), tree, only=lines
            )
        except (SystemExit, Exception) as error:
            # The run is already marked collected: a traceback here would lose
            # its summary for good. As in `recheck_windows`, the lines stay missed.
            print(
                f"the recheck of {crate} did not finish ({error}); "
                f"its {len(lines)} survivors stay missed"
            )
            returned |= lines
            continue
        untested += counts["untested"]
        returned |= set(run_missed)
        # A line no listed mutant matched never ran, and an unviable rerun was
        # caught by no test: absent from the survivors, either would read as caught.
        never_ran = len(lines) - sum(counts.values())
        if never_ran > 0:
            print(
                f"the recheck ran {len(lines) - never_ran} of {crate}'s "
                f"{len(lines)} survivors; none counts as caught"
            )
            untested += never_ran
            returned |= lines
        elif counts["unviable"]:
            print(
                f"the recheck found {counts['unviable']} of {crate}'s survivors "
                "unviable; none counts as caught"
            )
            returned |= lines
    caught = set()
    for line in missed:
        location, _, rest = line.partition(": ")
        back = {line} | {
            f"{location}: [{tag}] {rest}" for tag in ("timeout", "untested")
        }
        if not back & returned:
            caught.add(line)
    return caught, untested


def recheck_remote_extra(missed: list[str]) -> tuple[set[str], int]:
    """A remote extra pass's survivors (`missed`) in Windows-only code
    (`WINDOWS_ONLY_EXTRA`), rerun here by `recheck_windows_extra` on Windows
    outside REMOTE_ONLY, else listed as not rechecked. Returns its (lines
    caught, untested count), empty when nothing reran."""
    windows_extra = [
        line
        for line in missed
        if any(pattern.search(line) for pattern in WINDOWS_ONLY_EXTRA)
    ]
    if not windows_extra:
        return set(), 0
    if os.name != "nt" or REMOTE_ONLY:
        print("Windows-only extra mutants, not rechecked:")
        for line in windows_extra:
            print(f"  {line}")
        return set(), 0
    caught, untested = recheck_windows_extra(windows_extra)
    print(
        f"rechecked on this machine (Windows-only extra mutants): "
        f"{len(caught)} caught, {len(windows_extra) - len(caught)} still missed"
    )
    return caught, untested


def print_selection(log_path: Path) -> None:
    """The summary's "test selection" line, from the coverage map's exit code in
    the job log at `log_path`; nothing for a run that built no map."""
    map_exit = coverage_map_exit(log_path)
    if map_exit is None:
        return
    if map_exit == "0":
        print("test selection: on")
    else:
        print(
            "test selection: off (the coverage map failed, exit "
            f"{map_exit}; every mutant ran the full suite)"
        )


def coverage_map_exit(log_path: Path) -> str | None:
    """The coverage map's exit code as a whole run's job log records it, or
    None when the log has no such line (a diff run's, or no log): the run's
    mutants — cargo-mutants' and the extra pass's alike — selected their tests
    only when the map exited 0."""
    console = (
        log_path.read_text(encoding="utf-8", errors="replace")
        if log_path.exists()
        else ""
    )
    found = re.search(r"coverage map exit (\d+)", console)
    return found.group(1) if found else None


def summarize(
    local_code: int, remote_code: int, log_path: Path,
    seconds: tuple[float | None, float | None], *,
    local_half: bool, extra_run: Path | None,
) -> int:
    local = ROOT / "mutants.out"
    remote = local / "remote" / "mutants.out"
    names = ["caught", "missed", "timeout", "unviable"]
    # A run with no local half (a whole run, a REMOTE_ONLY diff) reads nothing
    # of the local `mutants.out` — a previous run's leftovers are not this run's.
    halves = ([("local", local, seconds[0])] if local_half else []) + [
        ("remote", remote, seconds[1])
    ]

    # A survivor of the remote half that only a #[cfg(windows)] test catches can
    # never be caught on the Linux host; rerun it here and let a catch replace
    # the remote outcome. Under REMOTE_ONLY, or off Windows, it stays missed
    # with a word saying why.
    remote_missed = read_lines(remote / "missed.txt")
    windows_missed = [
        line
        for line in remote_missed
        if any(pattern.search(line) for pattern in WINDOWS_ONLY_TESTS)
    ]
    caught_here: set[str] = set()
    rechecked = False
    if windows_missed:
        if os.name == "nt" and not REMOTE_ONLY:
            caught_here = recheck_windows(windows_missed)
            rechecked = True
            print(
                f"rechecked on this machine (Windows-only tests): "
                f"{len(caught_here)} caught, "
                f"{len(windows_missed) - len(caught_here)} still missed"
            )
        else:
            print("Windows-only tests, not rechecked:")
            for line in windows_missed:
                print(f"  {line}")

    # The extra-mutant pass a `just mutants <crate>` ran (EXTRA_KINDS) records
    # itself in run.json: which crate, its counts and seconds, the survivors in
    # `missed_<crate>.txt` beside it. `extra_run` names the record of the run's
    # own pass — a whole run's is the fetched `remote_extra_record`, a local
    # split's the local `extra_run_record()`; a run with no pass (a diff)
    # passes None. A pass that should have left its record and did not lost the
    # outcomes: say so and fail the run, rather than file them clean.
    extra_counts: dict | None = None
    extra_seconds = 0.0
    extra_missed = ""
    extra_missing = extra_run is not None and not extra_run.exists()
    if extra_missing:
        print("the extra pass left no record: see the remote log")
    elif extra_run is not None:
        record = json.loads(extra_run.read_text(encoding="utf-8"))
        missed_path = extra_run.parent / f"missed_{record['crate']}.txt"
        extra_counts = record["counts"]
        extra_seconds = record["seconds"]
        if missed_path.exists():
            extra_missed = missed_path.read_text(encoding="utf-8")

    # The extra kinds' survivors in Windows-only code get the same recheck as
    # the cargo-mutants ones — a fetched record's only: a local pass already
    # ran on this machine, so its survivors really were missed.
    extra_caught: set[str] = set()
    extra_untested = 0
    if extra_run == remote_extra_record():
        extra_caught, extra_untested = recheck_remote_extra(extra_missed.splitlines())

    print_selection(log_path)

    print(f"{'':8}{'caught':>7}{'missed':>7}{'timeout':>8}{'unviable':>9}{'seconds':>9}")
    totals = dict.fromkeys(names, 0)
    for label, base, elapsed in halves:
        counts = {name: count_lines(base / f"{name}.txt") for name in names}
        if label == "remote" and caught_here:
            counts["caught"] += len(caught_here)
            counts["missed"] -= len(caught_here)
        for name in names:
            totals[name] += counts[name]
        elapsed_text = "-" if elapsed is None else f"{elapsed:.0f}"
        print(
            f"{label:8}{counts['caught']:>7}{counts['missed']:>7}"
            f"{counts['timeout']:>8}{counts['unviable']:>9}{elapsed_text:>9}"
        )
    if extra_counts is not None:
        # The recheck's catches move out of missed, as the remote row's do.
        extra_counts["missed"] = extra_counts.get("missed", 0) - len(extra_caught)
        extra_counts["caught"] = extra_counts.get("caught", 0) + len(extra_caught)
        for name in names:
            totals[name] += extra_counts.get(name, 0)
        print(
            f"{'extra':8}{extra_counts.get('caught', 0):>7}"
            f"{extra_counts.get('missed', 0):>7}{extra_counts.get('timeout', 0):>8}"
            f"{extra_counts.get('unviable', 0):>9}{extra_seconds:>9.0f}"
        )
    print(
        f"{'total':8}{totals['caught']:>7}{totals['missed']:>7}"
        f"{totals['timeout']:>8}{totals['unviable']:>9}"
    )
    # Line by line, never text joined to text: a list without its final newline merged its last
    # line with the next list's first.
    local_missed = (
        read_lines(local / "missed.txt") + timeout_lines(local) if local_half else []
    )
    missed = "".join(
        line + "\n"
        for line in [
            *local_missed,
            *(line for line in remote_missed if line not in caught_here),
            *timeout_lines(remote),
            *(line for line in extra_missed.splitlines() if line not in extra_caught),
        ]
    )
    # A half that died before cargo-mutants started may leave no mutants.out at
    # all; the summary must still write its (empty or partial) list.
    local.mkdir(exist_ok=True)
    (local / "missed_all.txt").write_text(missed, encoding="utf-8")
    if missed:
        print(missed, end="")
    if remote_code not in (0, 2, 3):
        # cargo-mutants: 2 = missed mutants, 3 = timeouts, 4 = baseline failed.
        # This script adds 1 for its own findings (a half that never ran,
        # stopped early, or lost builds) and 5 for a split run whose merged
        # halves do not cover the recorded listing exactly once (below).
        print("--- remote log tail ---")
        print("\n".join(log_path.read_text(encoding="utf-8", errors="replace").splitlines()[-20:]))
    code = run_code([remote_code] + ([local_code] if local_half else []))
    # A survivor cargo-mutants does not know about (the extra kinds) is still a
    # miss: 2 is cargo-mutants' code for missed mutants. And a remote half whose
    # every miss the Windows recheck caught exits clean.
    if missed.strip():
        code = run_code([code, 2])
    elif code == 2:
        code = 0
    # The script's own 1 is not folded with max(): it must outrank a 2 or 3
    # (`run_code` does that in `report_killed`). The extra pass's `untested`
    # count (and its Windows recheck's) means the same, outside the halves' lists.
    untested = (
        extra_missing
        or bool(extra_counts and extra_counts.get("untested"))
        or bool(extra_untested)
    )
    # A local half stopped before cargo-mutants wrote its selection leaves nothing to count
    # (`never_ran` has no selection to compare), and its zero row would read as clean: the
    # fetch creates `mutants.out` for the remote half's copy whether the local half ran or not.
    if local_half and not REMOTE_ONLY and not (local / "mutants.json").exists():
        print("the local half never started: none of its mutants ran")
        untested = True
    for label, base, _ in halves:
        short = never_ran(base)
        if short is not None:
            ran, selected = short
            print(f"the {label} half stopped early: {selected - ran} of its {selected} mutants never ran")
            untested = True
    # A whole-crate split run records its whole listing's names
    # (split_run_record): whichever method shared them, the two halves'
    # merged outcomes must name each listed mutant exactly once — anything
    # else means a mutant went untested or got tested twice. A run with no
    # record (a diff, REMOTE_ONLY, a local run) skips the check, and so does
    # one with no local half: a record this tree holds is an earlier run's.
    record_path = split_run_record()
    if local_half and record_path.exists():
        record = json.loads(record_path.read_text(encoding="utf-8"))
        tested: Counter = Counter()
        for base in (local, remote):
            outcomes = base / "outcomes.json"
            if not outcomes.exists():
                continue
            for outcome in json.loads(outcomes.read_text(encoding="utf-8"))["outcomes"]:
                scenario = outcome["scenario"]
                if isinstance(scenario, dict):
                    tested[scenario["Mutant"]["name"]] += 1
        missing, doubled = check_split(record["names"], tested)
        for kind, names in (("never tested", missing), ("tested twice or unlisted", doubled)):
            if names:
                print(
                    f"split coverage: {sum(names.values())} mutants {kind}: "
                    + ", ".join(sorted(names)[:5])
                )
                code = max(code, 5)
    return report_killed(
        code, local_half=local_half, untested=untested, rechecked=rechecked
    )


def report_killed(
    code: int, *, local_half: bool, untested: bool = False, rechecked: bool = False
) -> int:
    """Lists the mutants that went untested (`killed_builds`: a killed build, a
    signal-ended cargo, a nextest run that broke before failing a test) under
    the local half's `mutants.out` when the run had one, the remote half's and,
    when the run rechecked (`rechecked`), the Windows recheck's — an earlier
    run's recheck folder is not this run's. Turns `code` into a failure when
    `untested` or a list is nonempty (`run_code` ranks it): those mutants were
    never tested, and the counts file them as unviable, caught or in no list."""
    local = ROOT / "mutants.out"
    here = killed_builds(local) if local_half else []
    # The recheck ran on this machine too: its untested mutants rerun the same way.
    if rechecked:
        here += killed_builds(local / "recheck" / "mutants.out")
    remote = killed_builds(local / "remote" / "mutants.out")
    if here or remote:
        print(
            f"{len(here) + len(remote)} mutants were NOT tested: a build process was killed "
            "for memory or lost its console, a build failed outside the compilation, "
            "a link failed (their logs say which), cargo was "
            "ended by a signal, or nextest failed without a failing test, and cargo-mutants "
            "counted them as unviable, caught or in no list."
        )
        untested = True
        if remote:
            tasks = REMOTE_JOBSERVER_TASKS or "the host's CPU count"
            print(
                "On the remote: a diff run reruns them on this machine (the same `just` "
                "command with STUDIO_MUTANTS_REMOTE set empty); a whole run has no local "
                "half, so rerun it there. If the remote half's peak reached its cap, "
                "free memory on the host or lower REMOTE_JOBSERVER_TASKS "
                f"(now {tasks}) in scripts/mutation/mutants.py:"
            )
            for name in remote:
                print(f"  {name}")
        if here:
            print("On this machine: rerun them with less running here:")
            for name in here:
                print(f"  {name}")
    return run_code([code, 1] if untested else [code])


def collect(host: str) -> int:
    """`just mutants-collect`: waits for a remote half this script launched but
    did not fetch (it was stopped, or gave up polling), fetches it and prints the
    summary. The wait holds no run lock — a whole run's lasts days; the fetch
    and summary take it. The half's launch record (`remote_pending`), read
    before the fetch removes it, tells whether its run had a local half (whose
    summary reads the local `mutants.out`, which no run since can have
    replaced: `local_refusal`) and ran the extra pass. With no record (a half
    launched from another tree) it summarizes the remote half alone, the extra
    pass counted when the fetch brings its record, and says so: nothing this
    tree holds is that run's. A record an earlier version wrote summarizes as a
    split, and one that says `extra_only` takes the extras-only finish
    (`finish_extra_run`): the pass's results and counts, no `mutants.out`. A
    died half's partial results
    are fetched too, whether it died before or during the wait, and the run
    exits 1."""
    state = remote_run_state(host)
    if state is None:
        print(f"cannot reach {host}")
        return 1
    if state == "none":
        print("no uncollected remote half")
        if remote_pending().exists():
            remote_pending().unlink()
            print(
                "the launch record is removed: its half is not on the host (it never "
                "started, or another tree collected it, and then a local half's "
                "results in this tree's mutants.out are in no summary)"
            )
        return 1
    try:
        if state == "died":
            raise RemoteDied("the remote half died")
        remote_code, remote_seconds = wait_remote(host)
    except RemoteDied:
        print("the remote half died before it finished; fetching its partial results")
        remote_code, remote_seconds = 1, None
        partial = True
    else:
        partial = False
    with run_lock():
        record = pending_record()
        if remote_run_state(host) == "none":
            # The half's own run fetched it while this collect waited.
            print("the remote half is already collected")
            return 1
        if record is not None and record.get("extra_only"):
            return finish_extra_run(host, remote_code, partial=partial)
        extra_if_fetched = False
        if not remote_pending().exists():
            # Launched from another tree: this tree's `mutants.out` and extra
            # record are some earlier run's, never this half's.
            print(
                "no launch record in this tree: summarizing the remote half alone "
                "(a local half's results are in the tree that launched it)"
            )
            local_half = False
            extra_run = None
            extra_if_fetched = True
        elif record is None:
            print(
                "the launch record is not one this version wrote: summarizing the "
                "run as a split with a local half"
            )
            local_half = True
            extra_run = extra_run_record() if extra_run_record().exists() else None
        else:
            local_half = record.get("local_half", True)
            extra_run = (
                remote_extra_record()
                if record.get("extra")
                else extra_run_record() if extra_run_record().exists() else None
            )
        # The local half's exit code went with the script that ran it; its files
        # say the same, in cargo-mutants' precedence: 4 = the baseline failed,
        # 3 = timeouts, 2 = missed mutants.
        local_code = 0
        if local_half:
            local = ROOT / "mutants.out"
            outcomes = local / "outcomes.json"
            baseline_failed = outcomes.exists() and any(
                outcome["scenario"] == "Baseline" and outcome["summary"] != "Success"
                for outcome in json.loads(outcomes.read_text(encoding="utf-8"))["outcomes"]
            )
            local_code = (
                4 if baseline_failed
                else 3 if count_lines(local / "timeout.txt")
                else 2 if count_lines(local / "missed.txt")
                else 0
            )
        return finish_remote_run(
            host, local_code, remote_code, (None, remote_seconds),
            local_half=local_half, extra_run=extra_run, partial=partial,
            extra_if_fetched=extra_if_fetched,
        )


def mutant_crate(file: str) -> str:
    """The crate a mutant lives in: the folder of the nearest `Cargo.toml` above
    its `file` field (`crates/libs/pes_savefile/src/...` -> `pes_savefile`,
    `crates/studio_core/src/...` -> `studio_core`). Counting path components
    instead named `studio` and `studio_core`, which sit directly under
    `crates/`, both `src`."""
    folder = (ROOT / file).parent
    while not (folder / "Cargo.toml").exists() and folder.parent != folder:
        folder = folder.parent
    return folder.name


def load_cost_cache() -> dict[str, dict[str, float]]:
    """`COST_CACHE`: `mutant` (crate -> mean seconds per mutant
    on this machine), `remote_mutant` (the same on the VPS half),
    `baseline` (crate -> the baseline seconds of the last local run that
    mutated it), and `tested` and `remote_tested` (crate -> [mean seconds per
    tested mutant, the tested mutants it stands on], which `local_share`
    reads); empty tables when no run has written them yet, as an older
    cache without `remote_mutant` or the tested tables reads. A cache in the
    older flat form (crate -> median seconds per mutant) seeds `mutant` with
    its values, minus the `src` key of the old crate naming: a median left
    out the timeouts and the cold builds, so it underestimates, but it is
    still nearer than the size fallback."""
    empty = {"mutant": {}, "remote_mutant": {}, "baseline": {}, "tested": {}, "remote_tested": {}}
    if not COST_CACHE.exists():
        return empty
    cache = json.loads(COST_CACHE.read_text(encoding="utf-8"))
    if isinstance(cache.get("mutant"), dict) and isinstance(cache.get("baseline"), dict):
        for table in ("remote_mutant", "tested", "remote_tested"):
            if not isinstance(cache.get(table), dict):
                cache[table] = {}
        return cache
    seeds = {crate: seconds for crate, seconds in cache.items() if crate != "src"}
    return {**empty, "mutant": seeds}


def cost_per_mutant(crate: str, cache: dict[str, dict[str, float]]) -> float:
    """A crate's measured mean seconds per mutant, or the size fallback when
    it has never run here: `1 + nonblank_lines / 1000` over
    `crates/**/<crate>/src/**/*.rs` (overestimates the mid-size crates, which
    errs toward splitting)."""
    if crate in cache["mutant"]:
        return cache["mutant"][crate]
    nonblank = 0
    for src in ROOT.glob(f"crates/**/{crate}/src/**/*.rs"):
        nonblank += sum(
            1
            for line in src.read_text(encoding="utf-8", errors="replace").splitlines()
            if line.strip()
        )
    return 1 + nonblank / 1000


def local_share(crate: str, cache: dict) -> float | None:
    """The share of a split run's mutants this machine should get:
    `remote / (local + remote)` over the two halves' mean seconds per tested
    mutant of `crate` (`tested`, `remote_tested`), so the slower half tests
    less and the two end together. Not the `mutant` tables' means, which
    count unviable mutants: those cost a build's seconds, and how many a half
    draws is chance, so one run's mean said 52% for a crate the VPS tests at
    two thirds of this machine's speed. None when either half's mean stands on
    fewer than `SHARE_MIN_TESTED` mutants — the run then stays round-robin."""
    local, local_weight = cache["tested"].get(crate, (0.0, 0))
    remote, remote_weight = cache["remote_tested"].get(crate, (0.0, 0))
    if min(local_weight, remote_weight) < SHARE_MIN_TESTED or not local or not remote:
        return None
    return remote / (local + remote)


def estimate_seconds(crates: list[str], share: float = 1.0) -> float:
    """The estimated wall seconds of running these mutants (one crate name per
    mutant) locally only: the unmutated baseline, which runs alone first,
    then every mutant's cost over the two jobs. The baseline is the largest
    measured for any of the crates: a cold build of the mutated packages in a
    fresh copy of the tree, a minute where a mutant takes seconds, so it
    dominates a small diff (3.z: 64 s of a 270 s run, while counting it as
    one mutant estimated the run at 80 s). `share` 0.5 gives one half of a
    split run: each half builds its own baseline, so a split takes the
    baseline plus half the rest, not half the estimate (over 29 split diffs to
    4.y-sub b3a, the local half came within a median 2% of this)."""
    cache = load_cost_cache()
    costs = [cost_per_mutant(crate, cache) for crate in crates]
    baseline = max(
        (cache["baseline"].get(crate, DEFAULT_BASELINE_SECONDS) for crate in set(crates)),
        default=0.0,
    )
    return baseline + sum(costs) / 2 * share


def update_cost_cache(out_dir: Path | None = None, table: str = "mutant") -> None:
    """Fold a finished half's measured costs into `COST_CACHE`'s `table` — the
    local `mutants.out` (the default) into `mutant` and `tested`, the fetched
    remote half's into `remote_mutant` and `remote_tested`. Per
    crate, `mutant` is the mean of the summed phase durations over every mutant outcome,
    caught, missed, timed out or unviable alike, replacing the old value when
    the run had at least 5 of them (below that the mean is noise): each
    listed mutant is one of these, and the mean carries what a median
    dropped, a timeout's whole test timeout and each job's first, cold build
    (40-60 s at 3.z). The baseline's duration is recorded for every crate the
    run mutated — the local table only: it feeds this machine's estimates, so
    the remote's baseline, measured on other hardware, does not. `tested` is
    the mean over the caught and missed mutants alone (an unviable one never
    reaches the tests, a timeout waits out a multiple of the baseline), folded
    into the mean so far as its mutants against at most `TESTED_HISTORY`
    earlier ones: what a file split's share reads (`local_share`). A run with a
    killed build changes nothing: its durations time
    the failure, not the tests (4.y-conv: 273 builds lost their console in
    23 s), and would pull later estimates under the split threshold."""
    out_dir = out_dir or ROOT / "mutants.out"
    path = out_dir / "outcomes.json"
    if not path.exists() or killed_builds(out_dir):
        return
    durations: dict[str, list[float]] = {}
    tested: dict[str, list[float]] = {}
    baseline = None
    for outcome in json.loads(path.read_text(encoding="utf-8"))["outcomes"]:
        scenario = outcome["scenario"]
        seconds = sum(phase["duration"] for phase in outcome["phase_results"])
        if isinstance(scenario, dict):
            crate = mutant_crate(scenario["Mutant"]["file"])
            durations.setdefault(crate, []).append(seconds)
            if outcome.get("summary") in ("CaughtMutant", "MissedMutant"):
                tested.setdefault(crate, []).append(seconds)
        elif scenario == "Baseline":
            baseline = seconds
    cache = load_cost_cache()
    for crate, values in durations.items():
        if len(values) >= 5:
            cache[table][crate] = statistics.mean(values)
        if table == "mutant" and baseline is not None:
            cache["baseline"][crate] = baseline
    tested_table = "tested" if table == "mutant" else "remote_tested"
    for crate, values in tested.items():
        mean, weight = cache[tested_table].get(crate, (0.0, 0))
        cache[tested_table][crate] = [
            (mean * weight + sum(values)) / (weight + len(values)),
            min(weight + len(values), TESTED_HISTORY),
        ]
    COST_CACHE.parent.mkdir(parents=True, exist_ok=True)
    temp = COST_CACHE.with_suffix(".tmp")
    temp.write_text(json.dumps(cache, indent=1) + "\n", encoding="utf-8")
    os.replace(temp, COST_CACHE)


def set_aside_last_run() -> None:
    """Move the last run's `mutants.out` to `mutants.out.old` before a run starts, as
    cargo-mutants itself does, so nothing of it is read as this run's: a cargo-mutants that
    fails before its own rotation (a config or diff error) would leave it in place, and the
    killed-build report and the cost cache would read it."""
    set_aside(ROOT / "mutants.out")


def set_aside(results: Path) -> None:
    """Move a results folder to `<name>.old`, replacing the older one, so a run's results
    outlive the next run's start. An empty folder, a run that landed nothing, is removed
    instead: kept, it would replace the results before it at the next run's start."""
    if not results.exists():
        return
    if not any(results.iterdir()):
        results.rmdir()
        return
    old = results.with_name(results.name + ".old")
    if old.exists():
        remove_tree(old)
    results.rename(old)


def extra_run_record() -> Path:
    """`extra_mutants.out/run.json`: which crate's extra-mutant pass (EXTRA_KINDS)
    belongs to the run in `mutants.out`, with its wall seconds and outcome counts;
    the survivors sit beside it in `missed_<crate>.txt`. Read at summary time so
    `just mutants-collect`, a different process, finds the record of the run it
    summarizes; under `extra_mutants.out/` because cargo-mutants moves an existing
    `mutants.out` aside when a run starts. Resolved under `ROOT` at call time so
    mutants_test's patched `ROOT` never reads the real one."""
    return ROOT / "extra_mutants.out" / "run.json"


def remote_extra_record() -> Path:
    """`mutants.out/remote/extra_mutants.out/run.json`: the extra pass's record
    as a whole run's fetch lands it — the remote ran the pass in its own tree
    (`--remote-extra`). `extra_run_record` is a local pass's."""
    return ROOT / "mutants.out" / "remote" / "extra_mutants.out" / "run.json"


def record_extra_run(crate: str, counts: dict, seconds: float) -> None:
    path = extra_run_record()
    path.parent.mkdir(exist_ok=True)
    temp = path.with_name(path.name + ".tmp")
    temp.write_text(
        json.dumps({"crate": crate, "seconds": seconds, "counts": dict(counts)})
        + "\n",
        encoding="utf-8",
    )
    os.replace(temp, path)


def split_run_record() -> Path:
    """`extra_mutants.out/split.json`: how the run in `mutants.out` shared a
    whole-crate split's mutants — the crate, the method (`files` or
    `round-robin`), the target and achieved local shares, the local files and
    the whole listing's mutant names, which the summary checks the merged
    outcomes against. Beside `run.json` for the same reason
    (`extra_run_record`): `just mutants-collect` is another process, and
    cargo-mutants moves an existing `mutants.out` aside when a run starts."""
    return ROOT / "extra_mutants.out" / "split.json"


def record_split_run(record: dict) -> None:
    path = split_run_record()
    path.parent.mkdir(exist_ok=True)
    temp = path.with_name(path.name + ".tmp")
    temp.write_text(json.dumps(record) + "\n", encoding="utf-8")
    os.replace(temp, path)


def list_mutants(selection: list[str], config: Path, source: Path) -> list[dict]:
    """The mutants `cargo mutants <selection> --config config` would test in `source`
    (the run's copy, `snapshot_source`; `selection` is `["-p", crate]` or a diff
    run's `["--in-diff", path, "-p", crate]`), from
    `--list --json` on this machine: each entry's `file` is the source path
    (slash-separated, relative to the tree, what examine/exclude globs match
    whole) and `name` the string the run's outcomes and missed.txt carry."""
    listed = subprocess.run(
        ["cargo", "mutants", *selection, "--list", "--json",
         "--config", str(config)],
        # UTF-8, not the Windows locale's code page: each mutant's `diff` quotes the source,
        # and characters such as `●` and `⏵` hold bytes cp1252 has no character for, which
        # killed the reader thread and left the listing empty.
        cwd=source, capture_output=True, text=True, encoding="utf-8",
    )
    if listed.returncode != 0:
        raise RuntimeError(f"listing {' '.join(selection)} failed:\n{listed.stderr}")
    return json.loads(listed.stdout) if listed.stdout.strip() else []


def partition(files: dict[str, int], share: float) -> set[str] | None:
    """The local half's source files of a whole-crate file split: `files`
    maps each file to its mutant count and `share` is the local target
    (`local_share`). Greedy, largest count first (ties by path): each file
    goes to the half further below its target, a deficit tie to local. None
    when the result lands further from `share` than an even split does — a
    crate whose mutants sit in one big file cannot be balanced by files, and
    stays round-robin."""
    total = sum(files.values())
    if not total:
        return None
    target = share * total
    local: set[str] = set()
    local_count = remote_count = 0
    for path, count in sorted(files.items(), key=lambda item: (-item[1], item[0])):
        if target - local_count >= total - target - remote_count:
            local.add(path)
            local_count += count
        else:
            remote_count += count
    achieved = local_count / total
    if abs(achieved - share) > abs(0.5 - share):
        print(
            f"a file split lands {achieved:.0%} of the mutants on this machine, "
            f"further from the {share:.0%} target than halves: "
            "splitting round-robin",
            flush=True,
        )
        return None
    return local


def check_split(
    expected: Iterable[str], actual: Iterable[str]
) -> tuple[Counter, Counter]:
    """Two multisets of mutant names: of `expected` (a run's whole listing),
    those `actual` (the halves' selections or merged outcomes) never covers,
    and of `actual`, those beyond `expected` — a mutant both halves took, or
    one the listing did not hold."""
    return Counter(expected) - Counter(actual), Counter(actual) - Counter(expected)


def plan_file_split(selection: list[str], nproc: int, source: Path) -> dict:
    """The split a run of `selection` (a whole crate's `["-p", crate]`, or a diff
    run's `["--in-diff", path, "-p", crate]`) in `source` will use, as its `split.json`
    record: `files` when the halves' measured costs give a share a file
    partition can meet, `round-robin` otherwise — an unmeasured half, a share
    within `SHARE_MIN_GAIN` of halves, no
    file split nearer the target than halves, or per-half listings that do
    not recombine to the whole listing (`check_split` over `--list --json`
    with each half's config)."""
    crate = selection[selection.index("-p") + 1]
    whole = list_mutants(selection, local_config(source), source)
    record = {
        "crate": crate,
        "method": "round-robin",
        "target": 0.5,
        "achieved": 0.5,
        "local_files": [],
        "names": [m["name"] for m in whole],
    }
    share = local_share(crate, load_cost_cache())
    if share is None:
        print(
            f"{crate}: no per-mutant cost measured on both halves yet; "
            "splitting round-robin",
            flush=True,
        )
        return record
    record["target"] = share
    if abs(share - 0.5) < SHARE_MIN_GAIN:
        print(
            f"{crate}: the measured costs give this machine {share:.0%} of the mutants, "
            "near enough to halves: splitting round-robin",
            flush=True,
        )
        return record
    counts = Counter(m["file"] for m in whole)
    local_files = partition(counts, share)
    if local_files is None:
        return record
    # The split launches only when exact: each half's own listing — the local
    # config's, and the remote config's text in a local file — must recombine
    # to the whole listing with no name missing or doubled.
    remote_text = sized_config(
        nproc, extra={"exclude_globs": sorted(local_files)}, source=source
    )
    remote_tmp = tempfile.NamedTemporaryFile(suffix=".toml", delete=False)
    try:
        remote_tmp.write(remote_text.encode("utf-8"))
        remote_tmp.close()
        local_names = [
            m["name"]
            for m in list_mutants(
                selection,
                local_config(source, extra={"examine_globs": sorted(local_files)}),
                source,
            )
        ]
        remote_names = [
            m["name"] for m in list_mutants(selection, Path(remote_tmp.name), source)
        ]
    finally:
        Path(remote_tmp.name).unlink(missing_ok=True)
    missing, doubled = check_split(record["names"], local_names + remote_names)
    if missing or doubled:
        names, kind = (missing, "missing") if missing else (doubled, "doubled")
        print(
            f"a file split is inexact ({sum(names.values())} {kind}, first: "
            f"{sorted(names)[0]}): splitting round-robin",
            flush=True,
        )
        return record
    achieved = sum(counts[path] for path in local_files) / len(record["names"])
    record.update(
        method="files", achieved=achieved, local_files=sorted(local_files)
    )
    print(
        f"file split: this machine tests {achieved:.0%} of {crate}'s "
        f"{len(record['names'])} mutants ({len(local_files)} files, target "
        f"{share:.0%}), the remote the rest",
        flush=True,
    )
    return record


def remote_start(host: str, tree: str) -> tuple[str | None, str | None, int]:
    """The first of a remote run's launch (the caller holds the run lock): the
    busy-host refusal (`Refused`), the snapshot's commit to `REMOTE_REF`, and
    the one-call probe with the cargo-mutants version check — a mismatch stops
    before anything runs. Returns the remote clone's head, tree and CPU
    count."""
    # The remote must be free before the transfer rewrites its tree.
    refusal = remote_refusal(host)
    if refusal is not None:
        raise Refused(refusal)
    snapshot(tree)
    remote_head, remote_tree, nproc, remote_version = remote_state(host)
    local_version = local_mutants_version()
    if local_version != remote_version:
        raise Refused(
            f"cargo-mutants versions differ: local {local_version}, "
            f"remote {remote_version}; fix: cargo install --locked "
            f"cargo-mutants@{local_version.split()[-1]} on the remote"
        )
    return remote_head, remote_tree, nproc


def finish_remote_run(
    host: str, local_code: int, remote_code: int,
    seconds: tuple[float | None, float | None], *,
    local_half: bool, extra_run: Path | None, partial: bool = False,
    extra_if_fetched: bool = False,
) -> int:
    """A remote run's end (the caller holds the run lock): fold the halves'
    measured costs into the cost table — the local table only when the run had
    a local half, the remote one only when its mutants ran the full suite —
    fetch the remote half's results (`partial`: it died, so it may have left
    none) and summarize. With `extra_if_fetched` (a collect with no launch
    record) the extra pass is the fetched record when the fetch brought one:
    every launch empties the host's, so one there is this half's."""
    if local_half:
        update_cost_cache()
    fetch_remote(host, partial=partial)
    if extra_if_fetched and remote_extra_record().exists():
        extra_run = remote_extra_record()
    console = ROOT / "mutants.out" / "remote" / "remote_console.txt"
    # The remote table sizes a diff run's split, whose halves run the full
    # suite: a whole run's selected tests would make the remote look faster.
    if coverage_map_exit(console) != "0":
        update_cost_cache(
            ROOT / "mutants.out" / "remote" / "mutants.out", "remote_mutant"
        )
    return summarize(
        local_code, remote_code, console,
        seconds, local_half=local_half, extra_run=extra_run,
    )


def finish_extra_run(host: str, code: int, *, partial: bool = False) -> int:
    """An extras-only run's end (the caller holds the run lock): fetch the
    pass's `extra_mutants.out`, print its `run.json`'s counts and the path of
    the survivors file beside it. The pass's own code is the run's — no
    cargo-mutants ran to take precedence over it (an untested mutant is the
    pass's 1; its missed mutants, like its caught, count but do not fail).
    Survivors in Windows-only code are rerun here first
    (`recheck_remote_extra`): a catch leaves the survivors file and moves the
    counts, and an untested rerun fails the run."""
    fetch_remote(host, partial=partial, extra_only=True)
    record_path = (
        ROOT / "extra_mutants.out" / "remote" / "extra_mutants.out" / "run.json"
    )
    if not record_path.exists():
        print("the extra pass left no record: see the remote log")
        return 1
    record = json.loads(record_path.read_text(encoding="utf-8"))
    crate = record["crate"]
    counts = record["counts"]
    survivors = record_path.parent / f"missed_{crate}.txt"
    lines = read_lines(survivors)
    caught, untested = recheck_remote_extra(lines)
    if caught:
        counts["missed"] = counts.get("missed", 0) - len(caught)
        counts["caught"] = counts.get("caught", 0) + len(caught)
        kept = "".join(line + "\n" for line in lines if line not in caught)
        temporary = survivors.with_name(survivors.name + ".tmp")
        temporary.write_text(kept, encoding="utf-8", newline="\n")
        os.replace(temporary, survivors)
    print(
        f"extra pass ({crate}, {record['seconds']:.0f} s): "
        + (", ".join(f"{name} {counts[name]}" for name in sorted(counts)) or "none")
    )
    print_selection(ROOT / "extra_mutants.out" / "remote" / "remote_console.txt")
    print(f"survivors: {survivors}")
    return 0 if code == 0 and not untested else 1


def split(
    selection: list[str], host: str, tree: str, source: Path, local: bool = True
) -> int:
    """The whole split run for one selection (the caller holds the run lock
    throughout: a diff run is minutes): the remote launch, both halves, the
    fetch and the summary. Without `local` (`REMOTE_ONLY`, a diff run) the
    remote runs every mutant. A split is planned first (`plan_file_split`) and
    recorded, a diff's as a whole crate's, so the summary checks the halves'
    merged outcomes against the run's own listing. The `["-p", crate]` path's
    local extra pass is kept for a whole run that needs this machine
    again; `whole_run` does not reach it. Either way the
    last run's local `mutants.out` is set aside first, as cargo-mutants itself
    does, so the summary counts this run's results alone. `tree` is the run's
    snapshot (`working_tree`) and `source` its copy (`snapshot_source`)."""
    if local:
        print(f"splitting the run with {host}", flush=True)
    else:
        print(f"running every mutant on {host} (REMOTE_ONLY)", flush=True)
    started = time.monotonic()
    remote_head, remote_tree, nproc = remote_start(host, tree)

    set_aside_last_run()
    local_files: set[str] | None = None
    extra_run: Path | None = None
    if local:
        # The split is planned and recorded before the halves start, so a
        # later `just mutants-collect` checks this run's merged outcomes
        # against its own listing.
        record = plan_file_split(selection, nproc, source)
        record_split_run(record)
        if record["method"] == "files":
            local_files = set(record["local_files"])
    if selection[0] == "-p" and local:
        import extra_mutants  # deferred: extra_mutants imports this module

        _, extra_counts, extra_seconds = extra_mutants.run_crate(
            selection[1], list(EXTRA_KINDS), tree
        )
        record_extra_run(selection[1], extra_counts, extra_seconds)
        extra_run = extra_run_record()
    elif selection[0] == "-p":
        kinds = "".join(f" --kind {kind}" for kind in EXTRA_KINDS)
        print(
            f"extra mutants skipped (REMOTE_ONLY runs nothing on this machine): "
            f"run `just extra-mutants {selection[1]}{kinds}` when it does",
            flush=True,
        )
        extra_run_record().unlink(missing_ok=True)
        split_run_record().unlink(missing_ok=True)
    else:
        # A diff run has no extra pass: drop the last whole-crate run's record so
        # the summary never reads its contents as this run's, and with no local
        # half the last split's too.
        extra_run_record().unlink(missing_ok=True)
        if not local:
            split_run_record().unlink(missing_ok=True)
    local_code, remote_code, local_seconds, remote_seconds = run_split(
        selection, host, tree, source, remote_head, remote_tree, nproc, local, local_files
    )
    code = finish_remote_run(
        host, local_code, remote_code, (local_seconds, remote_seconds),
        local_half=local, extra_run=extra_run,
    )
    # The halves' seconds leave out the transfer, the polling and the fetch;
    # this is what the user waited.
    print(f"{'split' if local else 'remote'} run: {time.monotonic() - started:.0f} s wall")
    return code


def remote_host() -> str | None:
    """`STUDIO_MUTANTS_REMOTE` from the process, else (Windows) from the user's
    registry environment. A process started before the variable was set, such as
    an IDE's or an agent's shell, does not see it, and would silently run every
    mutant locally. An empty value in the process opts out."""
    host = os.environ.get("STUDIO_MUTANTS_REMOTE")
    if host is None and os.name == "nt":
        import winreg

        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                host = winreg.QueryValueEx(key, "STUDIO_MUTANTS_REMOTE")[0]
        except OSError:
            host = None
    return host or None


def main(argv: list[str]) -> int:
    host = remote_host()
    if argv[1] == "--collect":
        if host is None:
            print("STUDIO_MUTANTS_REMOTE is not set")
            return 1
        return collect(host)
    if argv[1] == "--remote-extra":
        # The remote job runs this inside the unit: a whole run's job before
        # cargo-mutants, an extras-only run's after its map (`launch_remote`), the
        # kinds after the crate (EXTRA_KINDS when none) and `--selection`
        # naming the coverage map the job built first, each mutant's test
        # phase running the tests covering its lines. The pass runs in a
        # copy of the clone's own tree, its record where the fetch picks it up.
        # No lock, no refusal: the unit is alone on the clone.
        import extra_mutants  # deferred: extra_mutants imports this module

        kinds = argv[3:]
        selection = None
        if "--selection" in kinds:
            at = kinds.index("--selection")
            if at + 1 == len(kinds):
                print(
                    "usage: mutants.py --remote-extra <crate> [kinds...] "
                    "[--selection <path>]"
                )
                return 1
            selection = Path(kinds[at + 1])
            del kinds[at:at + 2]
        _, counts, seconds = extra_mutants.run_crate(
            argv[2], kinds or list(EXTRA_KINDS), selection=selection
        )
        record_extra_run(argv[2], counts, seconds)
        return 1 if counts["untested"] else 0
    if argv[1] == "--extra-only":
        # `just extra-mutants-remote <crate> [--kind <name>]...`: the extra
        # kinds alone on the VPS — a backfill (the `call` kind's over every
        # crate) measures for days of its own, so it goes where a whole run
        # goes, without the cargo-mutants part.
        if len(argv) < 3:
            print("usage: mutants.py --extra-only <crate> [--kind <name>]...")
            return 1
        kinds = []
        i = 3
        while i < len(argv):
            if argv[i] != "--kind" or i + 1 == len(argv):
                print("usage: mutants.py --extra-only <crate> [--kind <name>]...")
                return 1
            kinds.append(argv[i + 1])
            i += 2
        try:
            return extra_only_run(argv[2], kinds, host, working_tree())
        except Refused as refusal:
            print(refusal)
            return 1
    crate = argv[1]
    tree = working_tree()
    try:
        return whole_run(crate, host, tree)
    except Refused as refusal:
        print(refusal)
        return 1


def whole_run(crate: str, host: str | None, tree: str) -> int:
    """`just mutants <crate>` on the snapshot `tree`: every mutant on the
    remote, the coverage map and the extra kinds before cargo-mutants in the
    same service — a whole-crate run
    measures for days, and a local half would block this machine and die with
    it. The file split stays in `split` for when a whole run needs this machine
    again. The launch takes the run lock; the wait and `collect`'s take none,
    so a diff run goes ahead meanwhile."""
    if host is None:
        raise Refused(
            'whole-crate runs go to the VPS alone (RUNBOOK "Mutation runs and '
            'the VPS"): set STUDIO_MUTANTS_REMOTE'
        )
    if crate in LOCAL_ONLY_CRATES:
        raise Refused(
            f"{crate} is in LOCAL_ONLY_CRATES: its builds outgrow the remote's "
            "memory cap"
        )
    started = time.monotonic()
    with run_lock(), snapshot_source(tree) as source:
        print(f"running every mutant on {host} (a whole-crate run)", flush=True)
        remote_head, remote_tree, nproc = remote_start(host, tree)
        set_aside_last_run()
        # This run's extra pass runs on the remote (`--remote-extra`): a
        # previous run's local records are not its.
        extra_run_record().unlink(missing_ok=True)
        split_run_record().unlink(missing_ok=True)
        upload_and_launch(
            host, ["-p", crate], tree, source, remote_head, remote_tree, nproc,
            whole=True, local_half=False, extra=True,
        )
    remote_code, remote_seconds = wait_remote(host)
    with run_lock():
        code = finish_remote_run(
            host, 0, remote_code, (None, remote_seconds),
            local_half=False, extra_run=remote_extra_record(),
        )
    print(f"remote run: {time.monotonic() - started:.0f} s wall")
    return code


def extra_only_run(crate: str, kinds: list[str], host: str | None, tree: str) -> int:
    """`just extra-mutants-remote <crate>` (`--extra-only`) on the snapshot
    `tree`: the extra kinds `kinds` alone on the remote (`EXTRA_KINDS` when
    empty) — a kind's backfill over the crates measures for days of its own, so
    it goes where a whole run goes, the coverage map built first the same way,
    without the cargo-mutants part. The
    refusals are `whole_run`'s and the launch, wait and finish are shaped the
    same; nothing of `mutants.out` is set aside, summarized or folded into the
    cost table — the finish reads the pass's own files."""
    import extra_mutants  # deferred: extra_mutants imports this module

    unknown = [kind for kind in kinds if kind not in extra_mutants.ALL_KINDS]
    if unknown:
        raise Refused(
            f"unknown extra mutant kinds: {', '.join(sorted(set(unknown)))} "
            f"(the kinds are {', '.join(extra_mutants.ALL_KINDS)})"
        )
    kinds = sorted(set(kinds)) or list(EXTRA_KINDS)
    if host is None:
        raise Refused(
            'extra-mutant runs go to the VPS alone as whole-crate runs do '
            '(RUNBOOK "Mutation runs and the VPS"): set STUDIO_MUTANTS_REMOTE'
        )
    if crate in LOCAL_ONLY_CRATES:
        raise Refused(
            f"{crate} is in LOCAL_ONLY_CRATES: its builds outgrow the remote's "
            "memory cap"
        )
    started = time.monotonic()
    with run_lock(), snapshot_source(tree) as source:
        print(
            f"running the extra mutant kinds ({', '.join(kinds)}) on {host} "
            "(an extras-only run)",
            flush=True,
        )
        remote_head, remote_tree, nproc = remote_start(host, tree)
        upload_and_launch(
            host, ["-p", crate], tree, source, remote_head, remote_tree, nproc,
            whole=True, local_half=False, extra=False, extra_only=True, kinds=kinds,
        )
    remote_code, _ = wait_remote(host)
    with run_lock():
        code = finish_extra_run(host, remote_code)
    print(f"remote run: {time.monotonic() - started:.0f} s wall")
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv))
