"""`just mutants <crate>`: a whole-crate cargo-mutants run, optionally split in two.

With `STUDIO_MUTANTS_REMOTE` set to an ssh host (e.g. `bonfire`) the run is
split with it: a whole-crate run hands each half a complementary set of the
crate's source files, sized by the two halves' measured per-mutant costs
(`COST_CACHE`), through `examine_globs` in the local config and
`exclude_globs` in the remote's; a diff run, a crate with no measurement on a
side, and a file split that fails its exactness check keep the even
`--shard 0/2` / `1/2` round-robin. The remote half's results merge into
`mutants.out/remote/`. Unset (in the process
and, on Windows, in the user's registry environment) or empty, the plain local
command runs; it also runs for a crate in `LOCAL_ONLY_CRATES`, whose builds
outgrow the remote's memory cap.

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
waits for the remote half and fetches it. A new run refuses to start while a
remote half is running or finished but not yet collected, since the collection
summarizes the local `mutants.out` with it, and a run that uses the remote also
while `just gates-remote` runs there. A
running half is stopped by hand with `sudo systemctl stop studio-mutants` on
the host.

The remote needs: git, a C toolchain, rustup, cargo-mutants and `just` (for
`just gates-remote`) at the local versions (the pinned toolchain installs itself
on first use inside the tree), `sudo -n systemd-run` for the ssh user, `ionice`,
`tar`, `nproc`, `awk`, cgroup v2 (the unit's `memory.peak`) and `python3` (`just gates`'
scripts).
"""

import io
import json
import os
import re
import shutil
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

ROOT = Path(__file__).resolve().parent.parent
REMOTE_BASE = "~/studio-mutants"
REMOTE_REF = "refs/mutants/remote"
# Whether `just mutants` and `just mutants-diff` run every mutant on the remote
# (when STUDIO_MUTANTS_REMOTE names one), whatever the size or LOCAL_ONLY_CRATES,
# and nothing on this machine; `just gates-remote` is the gates' counterpart. A
# constant, not an environment variable: a shell started while a variable was set
# keeps the old value, which silently picked the wrong mode.
REMOTE_ONLY = False
# The extra_mutants kinds `just mutants <crate>` runs before the cargo-mutants
# run starts, so a whole-crate run stays one command and its missed_all.txt the
# one list to triage. A kind is listed when about half its survivors are real
# gaps, since each equivalent survivor costs a ruling every phase. `bool` joined
# once `.cargo/extra_mutants.toml` gave its ruled equivalents a home.
# `mutants-diff` does not run them: they run when a phase closes, only.
EXTRA_KINDS = ("bool", "literal", "normalize", "order")
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
# studio's log redaction matches the home folder in any letter case and with
# either slash on Windows alone, and its test is `#[cfg(windows)]`.
WINDOWS_ONLY_TESTS = [
    re.compile(
        r"crates/tools/team_compiler/src/output/deploy\.rs:\d+:\d+: .* in "
        r"(sweep_run|probe_download|promote_tree)$"
    ),
    re.compile(
        r"crates/studio_core/src/pes_process\.rs:\d+:\d+: "
        r"(replace (processes|image_path|probe) -> |.* in (processes|image_path|probe)$)"
    ),
    re.compile(r"crates/tools/export_upgrader/src/upgrade\.rs:\d+:\d+: .* in retry_while_locked$"),
    re.compile(
        r"crates/studio/src/gui_log\.rs:\d+:\d+: "
        r"(replace redact_any_spelling -> |.* in redact_(any_spelling|home)$)"
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
# Build jobs per remote cargo process (two run at once): fewer than the
# remote's half-the-CPUs test threads, because a build's memory grows with
# its parallel rustc processes and the GUI crates' dependencies (naga, wgpu,
# wayland) are the largest the workspace builds. At 2, a rerun peaked at the
# 9 GiB cap with three builds killed.
REMOTE_BUILD_JOBS = 1
# Crates whose mutants never go to the remote because their builds outgrow the
# cap even at one build job. A `mutants-diff` holding any of their mutants runs
# locally whole. Empty now: team_compiler was listed after two of its builds
# were killed at 9 GiB with REMOTE_BUILD_JOBS 1, until the cause turned out to
# be the host's tmpfs `/tmp`, where cargo-mutants put its two build copies
# (`REMOTE_TMPDIR` below). If a crate's builds are killed again with the
# copies on disk, list it here.
LOCAL_ONLY_CRATES: frozenset[str] = frozenset()
# Where the remote half's cargo-mutants makes its two copies of the tree. The
# host's `/tmp`, cargo-mutants' default, is a 7.8 GiB tmpfs (checked
# 2026-10-09), so each copy's `target/` (4 GiB with egui linked) lived in RAM
# and counted against the unit's MemoryMax: the team_compiler kills were per-mutant
# rebuilds of 1-2 GiB on top of two such trees, not the egui build itself.
REMOTE_TMPDIR = "$HOME/studio-mutants/tmp"
# The most of the host's disk `REMOTE_BASE` may hold when a run that uses the
# remote starts: the maintainer affords about 40 GB, and a running half adds its
# two build copies (about 4 GiB each) on top. On 2026-10-22 it held 50 GB (28.6 of
# them tests' leftovers in `REMOTE_TMPDIR`, 22 the clone's build cache), the host's
# disk was at 99%, and a half lost five mutants to it (RUNBOOK "The scratch folder").
REMOTE_DISK_BUDGET = 30 * 2**30
# The least free space the host's disk must have when such a run starts, whatever
# this project holds: a half's build copies and tests take about 10 GiB of it, and 2
# more stay for the host's own system (its logs, its other services, idle since
# 2026-10-29). At 10 a run could end on a full disk, which kills the half.
REMOTE_FREE_FLOOR = 12 * 2**30
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


def sized_config(
    cpus: int, build_jobs: int | None = None, extra: dict | None = None
) -> str:
    """`.cargo/mutants.toml`'s text with each of the two cargo processes (`--jobs
    2`) given half of `cpus` logical CPUs, for test threads and, unless
    `build_jobs` sets fewer, build jobs, so together they use every CPU
    without oversubscribing it. The scripts run cargo-mutants at a low
    priority instead of holding CPUs back (`below_normal`), so the machine
    stays usable. `extra` sets more keys — a file split's `examine_globs` or
    `exclude_globs`; a list value extends the base file's list of that key
    rather than replacing it."""
    config = tomllib.loads((ROOT / ".cargo/mutants.toml").read_text(encoding="utf-8"))
    half = max(1, cpus // 2)
    config["additional_cargo_args"] = ["--jobs", str(build_jobs or half)]
    config["additional_cargo_test_args"] = ["--", f"--test-threads={half}"]
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


def remote_config(host: str, nproc: int, extra: dict | None = None) -> None:
    """Write the remote's mutants.toml, sized for the remote's CPUs, its builds
    held to `REMOTE_BUILD_JOBS` for the memory cap; `extra` carries a file
    split's `exclude_globs`."""
    ssh(host, f"cat > {REMOTE_BASE}/mutants.remote.toml",
        input=sized_config(nproc, REMOTE_BUILD_JOBS, extra), text=True, check=True)


def local_config(extra: dict | None = None) -> Path:
    """Write `target/mutants.local.toml`, sized for this machine's CPUs, and
    return its path for `--config`; `extra` carries a file split's
    `examine_globs`."""
    path = ROOT / "target" / "mutants.local.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(".tmp")
    temp.write_text(sized_config(os.cpu_count() or 2, extra=extra), encoding="utf-8")
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


def local_mutants(args: list[str], config: Path | None = None) -> subprocess.CompletedProcess:
    """Runs `cargo mutants <args> --jobs 2` here, sized by `local_config` and at
    `below_normal` priority, its copies of the tree in `LOCAL_TMPDIR`: Rust's
    `temp_dir`, which cargo-mutants copies into, reads `TMP` then `TEMP` on
    Windows and `TMPDIR` elsewhere. `config` is a config a file split already
    wrote (its `examine_globs` must not be regenerated away)."""
    LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    folder = str(LOCAL_TMPDIR)
    if config is None:
        config = local_config()
    return subprocess.run(
        ["cargo", "mutants", *args, "--jobs", "2", "--config", str(config)],
        cwd=ROOT,
        env={**os.environ, "TMP": folder, "TEMP": folder, "TMPDIR": folder},
        **below_normal(),
    )


# What a build killed or starved for reasons outside the mutant leaves in its log, lowercase:
# rustc killed by the OOM killer, the linker killed by it (gcc's `collect2`), LLVM or rustc
# out of memory, a Windows process that lost its console.
KILLED_BUILD_SIGNS = (
    "sigkill", "signal 9 [killed]", "llvm error: out of memory", "memory allocation of",
    "0xc0000142",
)


def killed_builds(out_dir: Path) -> list[str]:
    """The mutants under `out_dir` (a `mutants.out`) that went untested: those
    cargo-mutants reported unviable because a build process was killed, not
    because the mutant does not compile, and those it filed as a `Failure` (its
    cargo process ended by a signal), which no list holds, so the counts would
    come up short without a word. A test killed for memory is a caught mutant,
    as intended: the mutant made the test blow up. The remote half's memory cap kills a rustc that
    outgrows it, and cargo-mutants files the failed build as unviable, so the
    mutant goes untested without a miss or an error: at 3.z, 18 of the remote
    half's 19 mutants, eframe's dependency tree outgrowing 6 GiB. Windows has
    no OOM killer: a rustc out of memory there aborts with "memory allocation
    of N bytes failed". A build that lost its console fails to start with
    0xC0000142 (4.y-conv: 273 of 274 mutants filed unviable that way)."""
    path = out_dir / "outcomes.json"
    if not path.exists():
        return []
    killed = []
    for outcome in json.loads(path.read_text(encoding="utf-8"))["outcomes"]:
        if outcome["summary"] == "Failure" and isinstance(outcome["scenario"], dict):
            killed.append(outcome["scenario"]["Mutant"]["name"])
            continue
        if outcome["summary"] != "Unviable" or not outcome.get("log_path"):
            continue
        log = (out_dir / outcome["log_path"]).read_text(encoding="utf-8", errors="replace")
        if any(sign in log.lower() for sign in KILLED_BUILD_SIGNS):
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


def remote_refusal(host: str, remote: bool) -> str | None:
    """Why a new run cannot start, or None. Any run refuses while a remote half is running
    or finished but uncollected, whose collection summarizes the local `mutants.out` with
    it, which a new local half would replace. A run that uses the remote (`remote`) also
    refuses while the host cannot be reached or `just gates-remote` runs there, since each
    rewrites the remote clone under the other, and past the disk guard (`disk_refusal`,
    after `remote_disk_use` has emptied the tests' leftovers); a local one starts when the
    host cannot be reached, which is no reason to stop it."""
    state = remote_run_state(host)
    if state is None:
        return f"cannot reach {host}" if remote else None
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
    if not remote:
        return None
    gates = ssh(host, f"systemctl is-active --quiet {GATES_UNIT}").returncode
    if gates == 255:
        return f"cannot reach {host}"
    if gates == 0:
        return f"`just gates-remote` is running on {host}: start the new run when it ends"
    return disk_refusal(*remote_disk_use(host))


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
    """A refusal when `REMOTE_BASE` holds more than `REMOTE_DISK_BUDGET` or its
    disk has less than `REMOTE_FREE_FLOOR` free (`used` and `free` in bytes,
    `remote_disk_use`), naming the cleanup; None, after printing both figures,
    otherwise."""
    figures = (
        f"{used / 2**30:.1f} GiB in {REMOTE_BASE} (budget {REMOTE_DISK_BUDGET / 2**30:.0f} "
        f"GiB), {free / 2**30:.1f} GiB free (floor {REMOTE_FREE_FLOOR / 2**30:.0f} GiB)"
    )
    if used > REMOTE_DISK_BUDGET:
        return (
            f"remote disk: {figures}: over the budget, run RUNBOOK \"The scratch folder\"'s "
            "remote cleanup, then start the run"
        )
    if free < REMOTE_FREE_FLOOR:
        return (
            f"remote disk: {figures}: below the floor, which this project's cleanup alone may "
            "not lift; ask the maintainer before starting the run"
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


def remote_selection(selection: list[str]) -> str:
    """The remote half's selection arguments: a `--in-diff` selection reads the
    uploaded copy of the diff (`run/in.diff`) and keeps what follows it, such as
    the `-p <crate>` of a per-crate diff run, which would otherwise test every
    crate's diff mutants on the remote."""
    if selection[0] == "--in-diff":
        return " ".join(['--in-diff "$HOME/studio-mutants/run/in.diff"', *selection[2:]])
    return " ".join(selection)


def launch_remote(host: str, selection: list[str], whole: bool = False) -> None:
    """Writes `run/job.sh` (and `run/in.diff` for a `--in-diff` selection,
    the same diff the local half reads) and starts it as the `REMOTE_UNIT`
    transient service (`sudo -n systemd-run`): shard 1/2 of the selection, or
    all of it when `whole` (`REMOTE_ONLY`, and a whole-crate file split, whose
    shares the remote's `exclude_globs` carry instead of a shard). The service is
    detached from this ssh session
    by construction: the unit is memory-capped (`MemoryMax`, no swap) and
    CPU-idle so Fluxer's production services on the host win, and
    `OOMPolicy=continue` keeps cargo-mutants alive past an in-unit OOM kill.
    `run/pid` is the service's MainPID and `run/memory_max` its cap
    (`remote_memory_max`); a failed `systemd-run` fails the launch (no
    fallback)."""
    remote_args = remote_selection(selection)
    shard = "" if whole else "--shard 1/2 --sharding round-robin "
    job = (
        "#!/bin/bash\n"
        "exec > ../run/log 2>&1\n"
        "start=$(date +%s)\n"
        # On disk, not the tmpfs `/tmp`: see REMOTE_TMPDIR.
        f'export TMPDIR="{REMOTE_TMPDIR}" && mkdir -p "$TMPDIR"\n'
        f". ~/.cargo/env && nice -n 19 ionice -c3 cargo mutants {remote_args} --jobs 2 "
        f"{shard}--config ../mutants.remote.toml\n"
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
        f"{REMOTE_RUN}/in.diff && "
        # The tree's `mutants.out` is gitignored, so the transfer keeps the last
        # run's; a half that fails before cargo-mutants starts must leave none,
        # or the fetch would count the last run's results as this one's.
        f"rm -rf {REMOTE_BASE}/tree/mutants.out && "
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


def run_split(
    selection: list[str], host: str, tree: str, remote_head: str | None,
    remote_tree: str | None, nproc: int, local: bool = True,
    local_files: set[str] | None = None,
) -> tuple[int, int, float | None, float]:
    """The local half starts at once, in the foreground with console output as
    today; a thread transfers the snapshot, writes the remote config and launches
    the remote half, so the transfer counts against the remote side only.
    `selection` is the mutant-selection arguments (`["-p", crate]` or
    `["--in-diff", path, "-p", crate]`); the remote reads an uploaded copy of a diff.
    Without `local` (`REMOTE_ONLY`) the remote runs the whole selection and
    nothing runs here. With `local_files` (a whole-crate file split) neither
    half takes a `--shard`: the local config examines those files and the
    remote's excludes them, so each tests a complementary set. Returns the
    (local, remote) codes and seconds: the local
    half's from the common start (None without one), the remote half's from its
    own launch."""
    start = time.monotonic()
    remote: dict = {}
    whole = not local
    remote_extra = (
        {"exclude_globs": sorted(local_files)} if local_files is not None else None
    )

    def remote_half() -> None:
        try:
            if remote_tree == tree:
                print("remote tree up to date", flush=True)
            else:
                transfer(host, remote_head)
            remote_config(host, nproc, remote_extra)
            # A file split's remote runs with no `--shard`: its config's
            # exclude_globs already name the local half's share.
            launch_remote(host, selection, whole=whole or local_files is not None)
            print("remote half launched", flush=True)
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
        local_run = local_mutants([*selection, "--shard", "0/2", "--sharding", "round-robin"])
    else:
        local_run = local_mutants(
            list(selection),
            config=local_config(extra={"examine_globs": sorted(local_files)}),
        )
    local_seconds = time.monotonic() - start
    thread.join()
    if "error" in remote:
        raise RuntimeError(
            "the remote half failed before running; the local half covered shard 0/2 "
            "only (rerun, or set STUDIO_MUTANTS_REMOTE empty for a local run)"
        ) from remote["error"]
    remote_code, remote_seconds = wait_remote(host)
    return local_run.returncode, remote_code, local_seconds, remote_seconds


def fetch_remote(host: str, partial: bool = False) -> None:
    """Copies the remote half's `mutants.out` and console log into
    `mutants.out/remote/`, emptied first so nothing of an earlier fetch is
    counted as this run's, then marks the run collected. A finished half with
    no `mutants.out` fails the fetch; a died one (`partial`) may have none."""
    out = ROOT / "mutants.out" / "remote"
    # Exit 3 is the half's own answer, no `mutants.out`; any other failure (ssh's 255, a
    # broken archive) is the transfer's, and the half stays uncollected for a retry.
    fetched = ssh(
        host,
        f"cd {REMOTE_BASE}/tree && {{ [ -d mutants.out ] || exit 3; }} && tar -cf - mutants.out",
        capture_output=True,
    )
    absent = fetched.returncode == 3
    if not absent and (fetched.returncode != 0 or not fetched.stdout):
        raise RuntimeError(
            f"fetching the remote half failed (exit {fetched.returncode}); it stays "
            "uncollected: retry with `just mutants-collect`"
        )
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    if not absent:
        with tarfile.open(fileobj=io.BytesIO(fetched.stdout), mode="r|") as tar:
            tar.extractall(out, filter="data")
    elif partial:
        print("warning: the remote half left no mutants.out", file=sys.stderr)
    else:
        raise RuntimeError(
            "the remote half finished but left no mutants.out: its log is "
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


def count_lines(path: Path) -> int:
    return len(path.read_text(encoding="utf-8").splitlines()) if path.exists() else 0


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


def recheck_windows(missed: list[str]) -> set[str]:
    """Rerun the remote half's `missed` survivors (missed.txt lines) in one
    cargo-mutants run on this machine, the tests that catch them being
    `#[cfg(windows)]` (WINDOWS_ONLY_TESTS) so the Linux remote cannot, and
    return the lines it caught. `--re` names each mutant escaped and anchored,
    `-p` the crates they live in, `--output` a folder of its own under
    `mutants.out/` so the local half's results stay. A run that cannot start or
    whose baseline fails (exit 4) is reported and returns empty, leaving every
    survivor in the list."""
    crates = sorted({mutant_crate(line.split(":", 1)[0]) for line in missed})
    out = ROOT / "mutants.out" / "recheck"
    pattern = "^(" + "|".join(re.escape(line) for line in missed) + ")$"
    args = [arg for crate in crates for arg in ("-p", crate)]
    args += ["--re", pattern, "--output", str(out)]
    print(f"rechecking {len(missed)} Windows-only mutants on this machine", flush=True)
    run = local_mutants(args)
    results = out / "mutants.out"
    if run.returncode == 4 or not (results / "outcomes.json").exists():
        print(
            f"the recheck did not finish (exit {run.returncode}); "
            f"the {len(missed)} mutants stay missed"
        )
        return set()
    caught = (
        (results / "caught.txt").read_text(encoding="utf-8").splitlines()
        if (results / "caught.txt").exists()
        else []
    )
    return set(caught) & set(missed)


def summarize(
    local_code: int, remote_code: int, log_path: Path,
    seconds: tuple[float | None, float | None],
) -> int:
    local = ROOT / "mutants.out"
    remote = local / "remote" / "mutants.out"
    names = ["caught", "missed", "timeout", "unviable"]

    # A survivor of the remote half that only a #[cfg(windows)] test catches can
    # never be caught on the Linux host; rerun it here and let a catch replace
    # the remote outcome. Under REMOTE_ONLY, or off Windows, it stays missed
    # with a word saying why.
    remote_missed_path = remote / "missed.txt"
    remote_missed = (
        remote_missed_path.read_text(encoding="utf-8").splitlines()
        if remote_missed_path.exists()
        else []
    )
    windows_missed = [
        line
        for line in remote_missed
        if any(pattern.search(line) for pattern in WINDOWS_ONLY_TESTS)
    ]
    caught_here: set[str] = set()
    if windows_missed:
        if os.name == "nt" and not REMOTE_ONLY:
            caught_here = recheck_windows(windows_missed)
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
    # `missed_<crate>.txt` beside it. A run without one (a diff, REMOTE_ONLY)
    # cleared the record before its cargo-mutants half started.
    extra_counts: dict | None = None
    extra_seconds = 0.0
    extra_missed = ""
    record_path = extra_run_record()
    if record_path.exists():
        record = json.loads(record_path.read_text(encoding="utf-8"))
        missed_path = record_path.parent / f"missed_{record['crate']}.txt"
        extra_counts = record["counts"]
        extra_seconds = record["seconds"]
        if missed_path.exists():
            extra_missed = missed_path.read_text(encoding="utf-8")

    print(f"{'':8}{'caught':>7}{'missed':>7}{'timeout':>8}{'unviable':>9}{'seconds':>9}")
    totals = dict.fromkeys(names, 0)
    for (label, base), elapsed in zip((("local", local), ("remote", remote)), seconds):
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
    missed = (local / "missed.txt").read_text(encoding="utf-8") if (local / "missed.txt").exists() else ""
    missed += "".join(
        line + "\n" for line in remote_missed if line not in caught_here
    )
    missed += extra_missed
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
    code = max(local_code, remote_code)
    # A survivor cargo-mutants does not know about (the extra kinds) is still a
    # miss: 2 is cargo-mutants' code for missed mutants. And a remote half whose
    # every miss the Windows recheck caught exits clean.
    if missed.strip():
        code = max(code, 2)
    elif code == 2:
        code = 0
    # A local half stopped before cargo-mutants wrote its selection leaves nothing to count
    # (`never_ran` has no selection to compare), and its zero row would read as clean: the
    # fetch creates `mutants.out` for the remote half's copy whether the local half ran or not.
    if not REMOTE_ONLY and not (local / "mutants.json").exists():
        print("the local half never started: none of its mutants ran")
        code = max(code, 1)
    for label, base in (("local", local), ("remote", remote)):
        short = never_ran(base)
        if short is not None:
            ran, selected = short
            print(f"the {label} half stopped early: {selected - ran} of its {selected} mutants never ran")
            code = max(code, 1)
    # A whole-crate split run records its whole listing's names
    # (split_run_record): whichever method shared them, the two halves'
    # merged outcomes must name each listed mutant exactly once — anything
    # else means a mutant went untested or got tested twice. A run with no
    # record (a diff, REMOTE_ONLY, a local run) skips the check.
    record_path = split_run_record()
    if record_path.exists():
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
    return report_killed(code)


def report_killed(code: int) -> int:
    """Lists the mutants whose build was killed (`killed_builds`), local half
    and remote half, and turns `code` into a failure when there are any: those
    mutants were never tested, and the counts above file them as unviable."""
    local = ROOT / "mutants.out"
    here = killed_builds(local)
    remote = killed_builds(local / "remote" / "mutants.out")
    if not here and not remote:
        return code
    print(
        f"{len(here) + len(remote)} mutants were NOT tested: a build process was killed "
        "for memory or lost its console, or cargo was ended by a signal, and cargo-mutants "
        "counted them as unviable or in no list."
    )
    if remote:
        print(
            "On the remote: rerun them on this machine (the same `just` command with "
            "STUDIO_MUTANTS_REMOTE set empty); if the remote half's peak reached its cap, "
            "free memory on the host or lower REMOTE_BUILD_JOBS "
            f"(now {REMOTE_BUILD_JOBS}) in scripts/mutants.py:"
        )
        for name in remote:
            print(f"  {name}")
    if here:
        print("On this machine: rerun them with less running here:")
        for name in here:
            print(f"  {name}")
    return max(code, 1)


def collect(host: str) -> int:
    """`just mutants-collect`: waits for a remote half this script launched but
    did not fetch (it was stopped, or gave up polling), fetches it and prints the
    summary against the local `mutants.out`, which no run since can have
    replaced while STUDIO_MUTANTS_REMOTE stayed set (`remote_refusal`). A died half's partial results are fetched too,
    whether it died before or during the wait, and the run exits 1."""
    state = remote_run_state(host)
    if state is None:
        print(f"cannot reach {host}")
        return 1
    if state == "none":
        print("no uncollected remote half")
        return 1
    try:
        if state == "died":
            raise RemoteDied("the remote half died")
        remote_code, remote_seconds = wait_remote(host)
    except RemoteDied:
        print("the remote half died before it finished; fetching its partial results")
        fetch_remote(host, partial=True)
        remote_code, remote_seconds = 1, None
    else:
        fetch_remote(host)
    update_cost_cache()
    update_cost_cache(
        ROOT / "mutants.out" / "remote" / "mutants.out", "remote_mutant"
    )
    # The local half's exit code went with the script that ran it; its files
    # say the same, in cargo-mutants' precedence: 4 = the baseline failed,
    # 3 = timeouts, 2 = missed mutants.
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
    return summarize(
        local_code, remote_code,
        ROOT / "mutants.out" / "remote" / "remote_console.txt",
        (None, remote_seconds),
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
    on this machine), `remote_mutant` (the same on the VPS half) and
    `baseline` (crate -> the baseline seconds of the last local run that
    mutated it); empty tables when no run has written them yet, as an older
    cache without `remote_mutant` reads. A cache in the
    older flat form (crate -> median seconds per mutant) seeds `mutant` with
    its values, minus the `src` key of the old crate naming: a median left
    out the timeouts and the cold builds, so it underestimates, but it is
    still nearer than the size fallback."""
    if not COST_CACHE.exists():
        return {"mutant": {}, "remote_mutant": {}, "baseline": {}}
    cache = json.loads(COST_CACHE.read_text(encoding="utf-8"))
    if isinstance(cache.get("mutant"), dict) and isinstance(cache.get("baseline"), dict):
        if not isinstance(cache.get("remote_mutant"), dict):
            cache["remote_mutant"] = {}
        return cache
    seeds = {crate: seconds for crate, seconds in cache.items() if crate != "src"}
    return {"mutant": seeds, "remote_mutant": {}, "baseline": {}}


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


def local_share(crate: str, cache: dict[str, dict[str, float]]) -> float | None:
    """The share of a whole-crate split run's mutants this machine should get:
    `remote / (local + remote)` over the two halves' measured mean seconds per
    mutant for `crate`, so the slower half tests less. None when either half
    has never measured the crate — the run then stays round-robin."""
    local = cache["mutant"].get(crate)
    remote = cache["remote_mutant"].get(crate)
    if not local or not remote:
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
    local `mutants.out` (the default) into `mutant`, the fetched remote
    half's into `remote_mutant`, which only a file split's share reads. Per
    crate, the mean of the summed phase durations over every mutant outcome,
    caught, missed, timed out or unviable alike, replacing the old value when
    the run had at least 5 of them (below that the mean is noise): each
    listed mutant is one of these, and the mean carries what a median
    dropped, a timeout's whole test timeout and each job's first, cold build
    (40-60 s at 3.z). The baseline's duration is recorded for every crate the
    run mutated — the local table only: it feeds this machine's estimates, so
    the remote's baseline, measured on other hardware, does not. A run with a
    killed build changes nothing: its durations time
    the failure, not the tests (4.y-conv: 273 builds lost their console in
    23 s), and would pull later estimates under the split threshold."""
    out_dir = out_dir or ROOT / "mutants.out"
    path = out_dir / "outcomes.json"
    if not path.exists() or killed_builds(out_dir):
        return
    durations: dict[str, list[float]] = {}
    baseline = None
    for outcome in json.loads(path.read_text(encoding="utf-8"))["outcomes"]:
        scenario = outcome["scenario"]
        seconds = sum(phase["duration"] for phase in outcome["phase_results"])
        if isinstance(scenario, dict):
            durations.setdefault(mutant_crate(scenario["Mutant"]["file"]), []).append(seconds)
        elif scenario == "Baseline":
            baseline = seconds
    cache = load_cost_cache()
    for crate, values in durations.items():
        if len(values) >= 5:
            cache[table][crate] = statistics.mean(values)
        if table == "mutant" and baseline is not None:
            cache["baseline"][crate] = baseline
    COST_CACHE.parent.mkdir(parents=True, exist_ok=True)
    temp = COST_CACHE.with_suffix(".tmp")
    temp.write_text(json.dumps(cache, indent=1) + "\n", encoding="utf-8")
    os.replace(temp, COST_CACHE)


def set_aside_last_run() -> None:
    """Move the last run's `mutants.out` to `mutants.out.old` before a run starts, as
    cargo-mutants itself does, so nothing of it is read as this run's: a cargo-mutants that
    fails before its own rotation (a config or diff error) would leave it in place, and the
    killed-build report and the cost cache would read it."""
    out = ROOT / "mutants.out"
    old = ROOT / "mutants.out.old"
    if out.exists():
        if old.exists():
            shutil.rmtree(old)
        out.rename(old)


def extra_run_record() -> Path:
    """`extra_mutants.out/run.json`: which crate's extra-mutant pass (EXTRA_KINDS)
    belongs to the run in `mutants.out`, with its wall seconds and outcome counts;
    the survivors sit beside it in `missed_<crate>.txt`. Read at summary time so
    `just mutants-collect`, a different process, finds the record of the run it
    summarizes; under `extra_mutants.out/` because cargo-mutants moves an existing
    `mutants.out` aside when a run starts. Resolved under `ROOT` at call time so
    mutants_test's patched `ROOT` never reads the real one."""
    return ROOT / "extra_mutants.out" / "run.json"


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


def list_mutants(crate: str, config: Path) -> list[dict]:
    """The mutants `cargo mutants -p crate --config config` would test, from
    `--list --json` on this machine: each entry's `file` is the source path
    (slash-separated, relative to the tree, what examine/exclude globs match
    whole) and `name` the string the run's outcomes and missed.txt carry."""
    listed = subprocess.run(
        ["cargo", "mutants", "-p", crate, "--list", "--json",
         "--config", str(config)],
        cwd=ROOT, capture_output=True, text=True,
    )
    if listed.returncode != 0:
        raise RuntimeError(f"listing {crate}'s mutants failed:\n{listed.stderr}")
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


def plan_file_split(crate: str, nproc: int) -> dict:
    """The split a whole-crate run of `crate` will use, as its `split.json`
    record: `files` when the halves' measured costs give a share a file
    partition can meet, `round-robin` otherwise — an unmeasured half, no
    file split nearer the target than halves, or per-half listings that do
    not recombine to the whole listing (`check_split` over `--list --json`
    with each half's config)."""
    whole = list_mutants(crate, local_config())
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
    counts = Counter(m["file"] for m in whole)
    local_files = partition(counts, share)
    if local_files is None:
        return record
    # The split launches only when exact: each half's own listing — the local
    # config's, and the remote config's text in a local file — must recombine
    # to the whole listing with no name missing or doubled.
    remote_text = sized_config(
        nproc, REMOTE_BUILD_JOBS, extra={"exclude_globs": sorted(local_files)}
    )
    remote_tmp = tempfile.NamedTemporaryFile(suffix=".toml", delete=False)
    try:
        remote_tmp.write(remote_text.encode("utf-8"))
        remote_tmp.close()
        local_names = [
            m["name"]
            for m in list_mutants(
                crate, local_config(extra={"examine_globs": sorted(local_files)})
            )
        ]
        remote_names = [m["name"] for m in list_mutants(crate, Path(remote_tmp.name))]
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


def split(
    selection: list[str], host: str, local: bool = True, tree: str | None = None
) -> int:
    """The whole split run for one selection: the busy-remote refusal, the
    snapshot, the version check, both halves, the fetch and the summary.
    Without `local` (`REMOTE_ONLY`) the remote runs every mutant. Either way the
    last run's local `mutants.out` is set aside first, as cargo-mutants itself
    does, so the summary counts this run's results alone. `tree` is the working
    tree a caller already wrote (`snapshot`)."""
    if local:
        print(f"splitting the run with {host}", flush=True)
    else:
        print(f"running every mutant on {host} (REMOTE_ONLY)", flush=True)
    started = time.monotonic()
    # The remote must be free before the transfer rewrites its tree.
    refusal = remote_refusal(host, remote=True)
    if refusal is not None:
        print(refusal)
        return 1

    # The snapshot and the one-call probe come first (a few seconds), so a
    # version mismatch stops before the local half starts.
    tree = snapshot(tree)
    remote_head, remote_tree, nproc, remote_version = remote_state(host)
    local_version = local_mutants_version()
    if local_version != remote_version:
        print(
            f"cargo-mutants versions differ: local {local_version}, "
            f"remote {remote_version}; fix: cargo install --locked "
            f"cargo-mutants@{local_version.split()[-1]} on the remote"
        )
        return 1

    set_aside_last_run()
    local_files: set[str] | None = None
    if selection[0] == "-p" and local:
        # The split is planned and recorded before the halves start, so a
        # later `just mutants-collect` checks this run's merged outcomes
        # against its own listing.
        record = plan_file_split(selection[1], nproc)
        record_split_run(record)
        if record["method"] == "files":
            local_files = set(record["local_files"])
        import extra_mutants  # deferred: extra_mutants imports this module

        _, extra_counts, extra_seconds = extra_mutants.run_crate(
            selection[1], list(EXTRA_KINDS), tree
        )
        record_extra_run(selection[1], extra_counts, extra_seconds)
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
        # A diff run has no extra pass: drop the last whole-crate run's records so
        # the summary never reads their contents as this run's.
        extra_run_record().unlink(missing_ok=True)
        split_run_record().unlink(missing_ok=True)
    local_code, remote_code, local_seconds, remote_seconds = run_split(
        selection, host, tree, remote_head, remote_tree, nproc, local, local_files
    )
    update_cost_cache()
    fetch_remote(host)
    update_cost_cache(
        ROOT / "mutants.out" / "remote" / "mutants.out", "remote_mutant"
    )
    code = summarize(
        local_code, remote_code,
        ROOT / "mutants.out" / "remote" / "remote_console.txt",
        (local_seconds, remote_seconds),
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
    crate = argv[1]
    if host is not None and REMOTE_ONLY:
        return split(["-p", crate], host, local=False)
    if host is None or crate in LOCAL_ONLY_CRATES:
        reason = (
            "STUDIO_MUTANTS_REMOTE is not set" if host is None
            else f"{crate} is local-only (LOCAL_ONLY_CRATES)"
        )
        if host is not None and (refusal := remote_refusal(host, remote=False)):
            print(refusal)
            return 1
        print(f"{reason}: running every mutant on this machine", flush=True)
        set_aside_last_run()
        # Not a split run: drop a previous split's record so its summary
        # never checks this run's outcomes against it.
        split_run_record().unlink(missing_ok=True)
        import extra_mutants  # deferred: extra_mutants imports this module

        _, extra_counts, extra_seconds = extra_mutants.run_crate(
            crate, list(EXTRA_KINDS)
        )
        record_extra_run(crate, extra_counts, extra_seconds)
        started = time.monotonic()
        code = local_mutants(["-p", crate]).returncode
        update_cost_cache()
        return summarize(
            code,
            0,
            ROOT / "mutants.out" / "remote" / "remote_console.txt",
            (time.monotonic() - started, None),
        )

    return split(["-p", crate], host)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
