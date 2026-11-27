"""`just driver`: the mutation driver, one process per machine taking
cargo-mutants shards from a queue of jobs.

`just driver queue <crate> [--shards N]` adds a job: a whole-crate run of
`<crate>` at the committed `HEAD`, cut into shards of about `SHARD_SECONDS`.
A lane is a machine taking the queue's shards, named in the markers below:
`pc` for this one, `vps` for the host `STUDIO_MUTANTS_REMOTE` names.
`just driver run [--once]` starts this PC's lane, which runs one shard at a
time, oldest job first, and then waits for more; with `--once` it stops when
nothing is left. So queued work waits at most one shard, and where several
lanes share the queue, the faster one takes more.

A job is sent to the VPS as it is queued, unless no host is set or the crate
is in `mutants.LOCAL_ONLY_CRATES`. Its source, `job.json` and `.todo`
markers are copied into the VPS's queue, and this PC keeps the job with a
`sent.json` and no marker, since two machines cannot share one rename. The
VPS lane (`run --lane vps`, a service that `just driver deploy` installs
there) takes a sent job's shards, so they go on while this PC is off, and
keeps their results there; it finishes no job. Before its first shard of a
job it builds the job's coverage map, so each mutant of its shards runs only
the tests that cover the mutant's lines, while this PC's shards of the job
run the full suite. This PC's lane takes the
shards of every job, a sent job's by renaming its marker on the VPS. Between
its shards it syncs each sent job with the VPS (`sync`): a claim it lost goes
back to `.todo` there, and the shards the VPS ended are fetched. It finishes
each job once all its shards are done. For a sent job, the finish also reruns
here the VPS's survivors that only a Windows-only test catches, and removes
the VPS's copy.

Everything lives under `DRIVER`, one folder per machine shared by every tree
(`VPS_DRIVER` on the VPS):

    guard.lock               held by the running driver for its whole life
    hold-<lane>              while it exists, the lane claims no new shard
    driver.log               the VPS lane's output, appended by its service
    bin/                     the VPS lane's scripts, installed by `deploy`
    tmp/                     the VPS lane's TMPDIR, emptied before each shard
    queue/<job>/             <job> is <UTC %Y%m%dT%H%M%S>-<crate>; name order is queue order
      job.json               {"crate", "tree", "args", "shards", "queued"}
      sent.json              {"host": host}: on this PC, a job sent to the VPS
      source/                the job's tree, extracted once
      mutants-vps.toml       the VPS lane's cargo-mutants config for the job
      map.sh                 the VPS lane's script for the job's coverage map
      map.log                its output
      mutants.json           the job's mutants, as the map lists them
      selection.json         the tests covering each mutant's lines, the map's result;
                             missing when the map failed
      map-exit               the map's exit code, or `killed`, as text, written last
      shard-<k>.todo         an empty marker for each shard not taken yet
      shard-<k>.<lane>       the marker, renamed by the lane that took the shard; a
                             sent job's `.pc` is on the VPS, and here while this PC
                             runs the shard
      shard-<k>.sh           the VPS lane's script for the shard
      shard-<k>/             the shard's cargo-mutants `--output` folder; a VPS
                             shard's also holds its `log`, `exit` and memory figures,
                             on this PC once fetched
      shard-<k>.done         the shard's exit code, as text, written last
      shard-<k>.fetched      on the VPS: the shard's results are on this PC, and
                             removed there
      baseline-<lane>.json   {"timeout": seconds}, after the lane's first shard of the job
      recheck/               on this PC: a sent job's Windows-only recheck, run when
                             the job finishes
    done/<job>/              the job once every shard is done, with result.json

A lane takes a shard by renaming its marker, which only one lane can do.
A restarted driver goes on from the folders:

- each shard its lane had taken but not ended goes back to `.todo`, its
  partial results removed (for a sent job, the VPS's marker goes back at
  the next `sync`);
- each job whose shards all ended is finished.

An error ends the driver as a stop does, and the next start resumes the same way.
"""

import argparse
import getpass
import json
import math
import os
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path

import extra_mutants
import mutants
import mutants_diff

# Beside the cost table: this machine's, whichever tree queues or runs.
DRIVER = mutants.COST_CACHE.with_name("driver")
LANE = "pc"
# The VPS lane's folder, as the driver running there sees it, and the same
# folder in an ssh command, relative to the remote home.
VPS_DRIVER = Path.home() / "studio-driver"
REMOTE_DRIVER = "studio-driver"
SHARD_UNIT = mutants.SHARD_UNIT
SHARD_SECONDS = mutants_diff.SHARD_SECONDS
POLL_SECONDS = 60
ON_WINDOWS = os.name == "nt"
# The hosts whose last call failed, so an outage is printed once and not at
# every turn.
unreachable: set[str] = set()


def jobs() -> list[Path]:
    """The queued jobs, oldest first; a job still being written (`.tmp`) is not one."""
    folder = DRIVER / "queue"
    if not folder.exists():
        return []
    return sorted(p for p in folder.iterdir() if p.is_dir() and not p.name.endswith(".tmp"))


def read_job(job: Path) -> dict:
    return json.loads((job / "job.json").read_text(encoding="utf-8"))


def shard_number(marker: Path) -> int:
    """`k` of a `shard-<k>.<anything>` marker."""
    return int(marker.name.split(".")[0][len("shard-"):])


def queue(crate: str, shards: int | None = None) -> Path:
    """Queue a whole-crate run of `crate` at `HEAD`'s tree, in `shards` shards, or
    as many as its estimate needs (`shard_count`). The job is written under a
    `.tmp` name and renamed when complete, so a driver never sees half a job.
    Before the rename it is sent to the VPS (`send`), unless no host is set or
    the crate is in `mutants.LOCAL_ONLY_CRATES`. Returns the job's folder."""
    tree = mutants.git(
        "rev-parse", "HEAD^{tree}", check=True, capture_output=True, text=True
    ).stdout.strip()
    moment = time.gmtime()
    job = DRIVER / "queue" / f"{time.strftime('%Y%m%dT%H%M%S', moment)}-{crate}"
    building = job.with_name(job.name + ".tmp")
    building.mkdir(parents=True)
    source = building / "source"
    mutants.extract_tree(tree, source)
    args = ["-p", crate]
    if shards is None:
        listed = mutants.list_mutants(args, mutants.local_config(source), source)
        shards = shard_count(mutants.estimate_seconds([crate] * len(listed)))
    record = {
        "crate": crate, "tree": tree, "args": args, "shards": shards,
        "queued": time.strftime("%Y-%m-%dT%H:%M:%SZ", moment),
    }
    mutants_diff.replace_text(building / "job.json", json.dumps(record))
    for k in range(shards):
        (building / f"shard-{k}.todo").touch()
    host = mutants.remote_host()
    sent = (
        host is not None and crate not in mutants.LOCAL_ONLY_CRATES
        and send(host, building, job.name)
    )
    if sent:
        # The VPS holds the job's markers alone. The source stays for the
        # rechecks this PC runs on the VPS's results.
        mutants_diff.replace_text(building / "sent.json", json.dumps({"host": host}))
        for k in range(shards):
            (building / f"shard-{k}.todo").unlink()
    building.rename(job)
    where = f", sent to {host}" if sent else ""
    print(f"driver: {crate} queued in {shards} shards, as {job.name}{where}", flush=True)
    return job


def send(host: str, building: Path, name: str) -> bool:
    """Copy the job being written in `building` into the VPS's queue as `name`:
    its source, `job.json` and `.todo` markers, under a `.tmp` name renamed
    when complete. Returns True when the job is on the VPS. Returns False,
    after printing why, when it stays this PC's:

    - the VPS's cargo-mutants version differs from this PC's, or asking for
      it failed;
    - a call of the copy failed, in which case the half-copied job is
      removed there."""
    local_version = mutants.local_mutants_version()
    try:
        probe = mutants.ssh(
            host, "~/.cargo/bin/cargo-mutants mutants --version", capture_output=True, text=True
        )
    except OSError as error:
        print(f"driver: {name} stays on this PC: cannot reach {host}: {error}", flush=True)
        return False
    if probe.returncode != 0:
        print(f"driver: {name} stays on this PC: cannot ask {host} for its cargo-mutants "
              f"version (exit {probe.returncode})", flush=True)
        return False
    remote_version = probe.stdout.strip()
    if remote_version != local_version:
        print(f"driver: {name} stays on this PC: cargo-mutants differs, {local_version} here, "
              f"{remote_version} on {host}", flush=True)
        return False
    remote = f"~/{REMOTE_DRIVER}/queue/{name}"
    try:
        upload(host, building, remote)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"driver: {name} stays on this PC: sending it to {host} failed: {error}",
              flush=True)
        remove_remote(host, f"{remote}.tmp")
        return False
    return True


def upload(host: str, building: Path, remote: str) -> None:
    """`send`'s copy into `<remote>.tmp` and its rename to `remote`; raises on
    any failed call."""
    spec = read_job(building)
    ssh_archive(
        host, [spec["tree"]], f"mkdir -p {remote}.tmp/source && tar -xf - -C {remote}.tmp/source"
    )
    # Bytes, not text: `text=True` would turn `\n` into `\r\n` on Windows.
    mutants.ssh(
        host, f"cat > {remote}.tmp/job.json", input=(building / "job.json").read_bytes(),
        check=True,
    )
    markers = " ".join(f"{remote}.tmp/shard-{k}.todo" for k in range(spec["shards"]))
    mutants.ssh(host, f"touch {markers} && mv {remote}.tmp {remote}", check=True)


def ssh_archive(host: str, archived: list[str], remote: str) -> None:
    """Run `remote` on `host` with `git archive --format=tar <archived>` as its
    stdin; raises on a failed call. The archive goes through a file under
    `mutants.LOCAL_TMPDIR` rather than a pipe, as `mutants.extract_tree`
    reads it."""
    mutants.LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    handle, archive = tempfile.mkstemp(dir=mutants.LOCAL_TMPDIR, suffix=".tar")
    os.close(handle)
    try:
        mutants.git("archive", "--format=tar", f"--output={archive}", *archived, check=True)
        with open(archive, "rb") as stream:
            mutants.ssh(host, remote, stdin=stream, check=True)
    finally:
        os.unlink(archive)


def remove_remote(host: str, path: str) -> None:
    """Remove `path` on `host`, best effort: a failure is printed, not raised."""
    try:
        removed = mutants.ssh(host, f"rm -rf {path}").returncode == 0
    except OSError:
        removed = False
    if not removed:
        print(f"driver: could not remove {path} on {host}", flush=True)


def sent_host(job: Path) -> str | None:
    """The host `job` was sent to (its `sent.json`), or None for a job that stayed here."""
    sent = job / "sent.json"
    if not sent.exists():
        return None
    return json.loads(sent.read_text(encoding="utf-8"))["host"]


def remote_job(job: Path) -> str:
    """`job`'s folder on the VPS, in an ssh command."""
    return f"~/{REMOTE_DRIVER}/queue/{job.name}"


def call_remote(host: str, command: str, **kwargs) -> subprocess.CompletedProcess | None:
    """Run `command` on `host` through `mutants.ssh`. Returns None when the
    call fails. The first failure of an outage is printed; the next ones are
    not, until a call to `host` succeeds again."""
    try:
        called = mutants.ssh(host, command, **kwargs)
    except OSError as error:
        failure = str(error)
    else:
        if called.returncode == 0:
            unreachable.discard(host)
            return called
        failure = f"exit {called.returncode}"
    if host not in unreachable:
        unreachable.add(host)
        print(f"driver: a call to {host} failed ({failure}): its jobs wait until it answers",
              flush=True)
    return None


# Renames the job's lowest-numbered `.todo` to `.pc` and prints its number;
# prints nothing when none is left or the folder is gone. `mv` fails when the
# VPS lane renamed the marker first, and the next one is tried.
CLAIM_SCRIPT = (
    "cd {folder} 2>/dev/null || exit 0; "
    "for todo in $(ls shard-*.todo 2>/dev/null | sort -t- -k2 -n); do "
    'k=${{todo#shard-}}; k=${{k%.todo}}; '
    'if mv "$todo" "shard-$k.pc" 2>/dev/null; then echo "$k"; exit 0; fi; '
    "done"
)


def claim_remote(job: Path, host: str) -> int | None:
    """Take a shard of the sent `job` on `host`, where its markers live, and
    mark it here too. Returns its number, or None when none is left or the
    call failed."""
    called = call_remote(
        host, CLAIM_SCRIPT.format(folder=remote_job(job)), capture_output=True, text=True
    )
    if called is None or not called.stdout.strip():
        return None
    k = int(called.stdout.strip())
    (job / f"shard-{k}.{LANE}").touch()
    return k


def sync(job: Path) -> None:
    """Bring the sent `job`'s VPS folder in line with this PC's, between this
    lane's shards:

    - a `shard-<k>.pc` there is removed once this PC's `.done` exists, and
      otherwise goes back to `.todo`: a claim this PC lost, since no shard
      runs here while it syncs;
    - a `shard-<k>.done` there with no `shard-<k>.fetched` is fetched
      (`fetch_shard`) unless this PC already has its `.done`, and is then
      marked fetched there and its results removed.

    A failed call ends the sync for this turn. A job that stayed here, or
    whose VPS folder is gone, has nothing to sync."""
    host = sent_host(job)
    if host is None:
        return
    folder = remote_job(job)
    listed = call_remote(host, f"ls -1 {folder} 2>/dev/null || true",
                         capture_output=True, text=True)
    if listed is None:
        return
    names = set(listed.stdout.split())
    for name in sorted(names):
        marker = job / name
        if marker.suffix not in (".pc", ".done"):
            continue
        k = shard_number(marker)
        done_here = (job / f"shard-{k}.done").exists()
        if marker.suffix == ".pc":
            if done_here:
                command = f"rm -f {folder}/{name}"
            else:
                command = f"mv {folder}/{name} {folder}/shard-{k}.todo"
        elif f"shard-{k}.fetched" in names:
            continue
        else:
            if not done_here and not fetch_shard(job, k):
                return
            command = f"touch {folder}/shard-{k}.fetched && rm -rf {folder}/shard-{k}"
        if call_remote(host, command) is None:
            return


def fetch_shard(job: Path, k: int) -> bool:
    """Copy shard `k` of the sent `job` from the VPS into `job/shard-<k>/`,
    replacing a partial copy, and mark it done here. The copy holds:

    - its `mutants.out` but the logs of mutants their tests caught
      (`mutants.REMOTE_FETCH_LIST`);
    - its `log`, `exit`, `memory_peak` and `memory_events`, those present.

    A `memory_events` counting an `oom_kill` writes `oom_kills.txt` beside
    them. The `.done`, written last, holds the shard's code (`shard_code`).
    Returns False, with nothing marked done, when the call failed."""
    shard = job / f"shard-{k}"
    if shard.exists():
        mutants.remove_tree(shard)
    shard.mkdir()
    mutants.LOCAL_TMPDIR.mkdir(parents=True, exist_ok=True)
    # Through a file, not memory, as `mutants.fetch_remote` takes a run's results.
    with tempfile.TemporaryFile(dir=mutants.LOCAL_TMPDIR) as archive:
        fetched = call_remote(
            sent_host(job),
            f"cd {remote_job(job)}/shard-{k} && set -o pipefail && "
            "{ python3 -; for name in log exit memory_peak memory_events; do "
            'if [ -f "$name" ]; then echo "$name"; fi; done; } '
            "| tar -cf - --verbatim-files-from -T -",
            input=mutants.REMOTE_FETCH_LIST.encode(), stdout=archive,
        )
        if fetched is None:
            return False
        archive.seek(0)
        with tarfile.open(fileobj=archive, mode="r|") as tar:
            tar.extractall(shard, filter="data")
    warning = mutants.oom_warning(mutants.read_lines(shard / "memory_events"))
    if warning is not None:
        print(f"driver: {job.name} shard {k + 1}: {warning}", flush=True)
        mutants_diff.replace_text(shard / "oom_kills.txt", warning + "\n")
    mutants_diff.replace_text(job / f"shard-{k}.done", str(shard_code(job, k)))
    return True


def shard_count(estimate: float) -> int:
    """The shards a run estimated at `estimate` seconds here is cut into: one per
    `SHARD_SECONDS`, and at least two past `mutants.SPLIT_THRESHOLD_SECONDS`,
    so a run that long can go to two lanes at once."""
    count = math.ceil(estimate / SHARD_SECONDS)
    if estimate >= mutants.SPLIT_THRESHOLD_SECONDS:
        count = max(count, 2)
    return max(count, 1)


def recover() -> None:
    """Return this lane's claims to the queue: run at the start, under the guard,
    so only a dead driver of this lane can have left one. A claimed shard's
    partial results are removed, and its marker goes back to `.todo`; one whose
    `.done` was written (the driver died before removing the marker) stays done.
    A sent job's marker is removed with no `.todo` here, since the VPS holds
    its markers: `sync` puts the VPS's back. On the VPS lane a shard unit a
    dead driver left running is stopped first, so no shard or map still
    writes the results this removes. A coverage map it left half done (a
    `map.log` and no `map-exit`) loses its files, and the job's next shard
    builds it again."""
    if LANE == "vps":
        if unit_active(SHARD_UNIT) and sudo("systemctl", "stop", SHARD_UNIT).returncode != 0:
            raise RuntimeError(f"could not stop {SHARD_UNIT}, left running by an earlier driver")
        for log in (DRIVER / "queue").glob("*/map.log"):
            if not log.with_name("map-exit").exists():
                for name in ("map.log", "mutants.json", "selection.json"):
                    log.with_name(name).unlink(missing_ok=True)
    for marker in (DRIVER / "queue").glob(f"*/shard-*.{LANE}"):
        if marker.with_suffix(".done").exists():
            marker.unlink()
            continue
        # Removed before the marker goes back, so a shard waiting as `.todo`
        # never has old results that cargo-mutants would keep beside the new.
        partial = marker.with_suffix("")
        if partial.exists():
            mutants.remove_tree(partial)
        if sent_host(marker.parent) is None:
            marker.rename(marker.with_suffix(".todo"))
        else:
            marker.unlink()


def claim() -> tuple[Path, int] | None:
    """Take the next shard: the oldest job's lowest `.todo` one, renamed to this
    lane's marker, on the VPS for a sent job (`claim_remote`). Returns the job
    and the shard, or None when no shard is left."""
    for job in jobs():
        host = sent_host(job)
        if host is not None:
            k = claim_remote(job, host)
            if k is not None:
                return job, k
            continue
        for todo in sorted(job.glob("shard-*.todo"), key=shard_number):
            try:
                os.rename(todo, todo.with_suffix(f".{LANE}"))
            except FileNotFoundError:
                continue  # another lane took it first
            return job, shard_number(todo)
    return None


def baseline_test_seconds(out: Path) -> float | None:
    """The baseline's test phase seconds in `out` (a `mutants.out`), or None
    when the run recorded none, as a shard with no mutant to test may not."""
    path = out / "outcomes.json"
    outcomes = json.loads(path.read_text(encoding="utf-8"))["outcomes"] if path.exists() else []
    baseline = next((o for o in outcomes if o["scenario"] == "Baseline"), None)
    if baseline is None:
        return None
    return next(p["duration"] for p in baseline["phase_results"] if p["phase"] == "Test")


def unit_active(unit: str) -> bool:
    """Whether systemd runs `unit` on this host: the VPS lane's probe."""
    return subprocess.run(["systemctl", "is-active", "--quiet", unit]).returncode == 0


def sudo(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["sudo", "-n", *args])


def blocked() -> str | None:
    """Why this lane claims no new shard now, or None. Either lane waits while
    its hold exists. The VPS lane also waits:

    - while a `just mutants` remote half (`mutants.REMOTE_UNIT`) or `just
      gates-remote` (`mutants.GATES_UNIT`) runs on the same host, since both
      would build there at once;
    - while the disk holding `DRIVER` has less than
      `mutants.REMOTE_FREE_FLOOR` free."""
    hold = DRIVER / f"hold-{LANE}"
    if hold.exists():
        return f"held by {hold}: no new shard until it is deleted"
    if LANE != "vps":
        return None
    for unit in (mutants.REMOTE_UNIT, mutants.GATES_UNIT):
        if unit_active(unit):
            return f"{unit} is running on this host: no new shard until it ends"
    if shutil.disk_usage(DRIVER).free < mutants.REMOTE_FREE_FLOOR:
        return (f"less than {mutants.REMOTE_FREE_FLOOR / 2**30:.0f} GiB free on the disk "
                f"holding {DRIVER}: no new shard until there is more")
    return None


def memory_max(meminfo: Path = Path("/proc/meminfo")) -> int:
    """The MemoryMax, in bytes, of a shard unit started now: this host's
    available memory less `mutants.REMOTE_MEMORY_RESERVE`, as
    `mutants.remote_memory_max` reckons it. Raises when nothing is left, which
    ends the driver; its service starts it again."""
    available = 1024 * next(
        int(line.split()[1]) for line in meminfo.read_text(encoding="utf-8").splitlines()
        if line.startswith("MemAvailable:")
    )
    cap = available - mutants.REMOTE_MEMORY_RESERVE
    if cap <= 0:
        raise RuntimeError(f"only {available / 2**30:.2f} GiB available: no memory cap to start under")
    return cap


def empty_tmp() -> Path:
    """`DRIVER/tmp`, emptied: a shard's tests leave their scratch folders
    there when a caught mutant's panic skips their cleanup."""
    tmp = DRIVER / "tmp"
    tmp.mkdir(parents=True, exist_ok=True)
    for entry in tmp.iterdir():
        if entry.is_dir() and not entry.is_symlink():
            mutants.remove_tree(entry)
        else:
            entry.unlink()
    return tmp


def unit_environment(tmp: Path) -> list[str]:
    """The lines that set the environment of both VPS unit scripts, the
    shard's and the map's: their copies of the tree in `tmp`, each test
    binary capped at `mutants.REMOTE_TEST_MEMORY`."""
    return [
        f"export TMPDIR={shlex.quote(str(tmp))}",
        # A target runner wraps only the test binaries, so each test caps at
        # the per-test figure while rustc and cargo keep the unit's.
        'export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='
        f'"prlimit --data={mutants.REMOTE_TEST_MEMORY} --"',
    ]


def map_script(job: Path, tmp: Path) -> bytes:
    """The bash script that builds `job`'s coverage map in the VPS lane's
    unit, from the job's source (the unit's working directory):

    - its output to `map.log`, its environment as a shard's (`unit_environment`);
    - the job's mutants listed into `mutants.json`, with the shards'
      arguments and config but no `--shard`, so the map covers every shard;
    - `coverage_map.py` writing `selection.json` from that list, at the
      lowest priority;
    - the map's instrumented build removed;
    - last, `map-exit`: the listing's code when it failed, else the map's.

    LF line ends, as bash needs."""
    spec = read_job(job)
    listed = job / "mutants.json"
    listing = shlex.join([
        "cargo-mutants", "mutants", *spec["args"], "--dir", str(job / "source"),
        "--list", "--json", "--config", str(job / "mutants-vps.toml"),
    ])
    coverage = shlex.join([
        "python3", "scripts/mutation/coverage_map.py", spec["crate"],
        str(job / "selection.json"), "--mutants", str(listed),
    ])
    lines = [
        "#!/bin/bash",
        f"exec > {shlex.quote(str(job / 'map.log'))} 2>&1",
        *unit_environment(tmp),
        f". ~/.cargo/env && {listing} > {shlex.quote(str(listed))}",
        "code=$?",
        'if [ "$code" = 0 ]; then',
        f"nice -n 19 ionice -c3 {coverage}",
        "code=$?",
        "fi",
        # Read by nothing after the map, the instrumented build would fill
        # the disk beside the shards' copies of the tree.
        "rm -rf target/llvm-cov-target",
        f'echo "$code" > {shlex.quote(str(job / "map-exit"))}',
    ]
    return ("\n".join(lines) + "\n").encode("utf-8")


def build_map(job: Path, tmp: Path) -> None:
    """Build `job`'s coverage map through `run_unit` (`map_script`). A failed
    map is printed, and the job's shards run the full suite. A killed one (no
    `map-exit` after the unit) is a failed map whose `map-exit` is then
    written as `killed`, so no later shard retries it. Raises when the unit
    never ran the script."""
    script = job / "map.sh"
    script.write_bytes(map_script(job, tmp))
    run_unit(script, memory_max())
    if not (job / "map.log").exists():
        raise RuntimeError(f"{SHARD_UNIT} did not start {script}")
    exit_file = job / "map-exit"
    if exit_file.exists():
        code = exit_file.read_text(encoding="utf-8").strip()
        reason = f"exit {code}"
    else:
        code = reason = "killed"
        mutants_diff.replace_text(exit_file, code)
    selection = job / "selection.json"
    if code != "0" or not selection.exists():
        # A map killed after it wrote its selection is still a failed map:
        # its shards run the full suite, as the print says.
        selection.unlink(missing_ok=True)
        print(f"driver: {job.name} coverage map failed ({reason}): its shards run the full "
              f"suite, log {job / 'map.log'}", flush=True)


def shard_script(job: Path, k: int, args: list[str], tmp: Path) -> bytes:
    """The bash script the VPS lane's shard unit runs: cargo-mutants on the
    job's source at the lowest priority, its output to `shard-<k>/log`, its
    environment from `unit_environment`. When the job's `selection.json`
    exists, `select_tests.py` stands in for cargo, so each mutant runs the
    tests covering its lines. Then the unit's memory figures, and last
    `shard-<k>/exit`, "<code> <seconds>". LF line ends, as bash needs."""
    shard = job / f"shard-{k}"
    source = job / "source"
    tasks = mutants.REMOTE_JOBSERVER_TASKS or os.cpu_count() or 2
    # `cargo mutants` would set `CARGO` for the subcommand, overriding the stand-in.
    command = shlex.join([
        "cargo-mutants", "mutants", *args, "--dir", str(source),
        "--jobs", str(mutants.JOBS), "--jobserver-tasks", str(tasks),
        "--config", str(job / "mutants-vps.toml"),
    ])
    selection = job / "selection.json"
    stand_in = ""
    if selection.exists():
        stand_in = (
            f"CARGO={shlex.quote(str(source / 'scripts' / 'mutation' / 'select_tests.py'))} "
            "UTW_REAL_CARGO=$(command -v cargo) "
            f"UTW_MUTANTS_SOURCE={shlex.quote(str(source))} "
            f"UTW_MUTANTS_SELECTION={shlex.quote(str(selection))} "
        )
    quoted = shlex.quote(str(shard))
    lines = [
        "#!/bin/bash",
        f"exec > {shlex.quote(str(shard / 'log'))} 2>&1",
        "start=$(date +%s)",
        *unit_environment(tmp),
        f". ~/.cargo/env && {stand_in}nice -n 19 ionice -c3 {command}",
        "code=$?",
        'cgroup="/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)"',
        f'[ -f "$cgroup/memory.peak" ] && cp "$cgroup/memory.peak" {quoted}',
        f'[ -f "$cgroup/memory.events" ] && cp "$cgroup/memory.events" {quoted}',
        # Last: a shard with no `exit` was killed before it ended.
        f'echo "$code $(( $(date +%s) - start ))" > {shlex.quote(str(shard / "exit"))}',
    ]
    return ("\n".join(lines) + "\n").encode("utf-8")


def run_unit(script: Path, memory_max: int) -> None:
    """Run `script` as the transient `SHARD_UNIT` service and wait for it to
    end: memory-capped with no swap, so the kernel kills inside the unit
    alone, and CPU-idle, so the host's own services come first. Its exit code
    is not read: the script's `exit` file holds cargo-mutants'."""
    sudo(
        "systemd-run", "--wait", "--collect", "--quiet", f"--unit={SHARD_UNIT}",
        f"--uid={getpass.getuser()}", "-p", f"MemoryMax={memory_max}", "-p", "MemorySwapMax=0",
        "-p", "OOMPolicy=continue", "-p", "CPUWeight=idle",
        f"--working-directory={script.parent / 'source'}", "/bin/bash", str(script),
    )


def shard_code(job: Path, k: int) -> int:
    """The code of shard `k` of `job` run on the VPS: the first number of its
    `exit`, or 1 when there is none (the unit, or its bash, was killed), so
    the job's result ranks it as untested."""
    exit_file = job / f"shard-{k}" / "exit"
    if not exit_file.exists():
        print(f"driver: {job.name} shard {k + 1} was killed before it ended: untested",
              flush=True)
        return 1
    return int(exit_file.read_text(encoding="utf-8").split()[0])


def run_vps_shard(job: Path, k: int, args: list[str]) -> int:
    """Run shard `k` of `job` through `run_unit`, and return its code
    (`shard_code`). The job's coverage map is built first (`build_map`)
    unless the job has a `map-exit`, so the map runs once per job."""
    config = job / "mutants-vps.toml"
    if not config.exists():
        mutants_diff.replace_text(
            config, mutants.sized_config(os.cpu_count() or 2, source=job / "source")
        )
    tmp = empty_tmp()
    if not (job / "map-exit").exists():
        build_map(job, tmp)
    shard = job / f"shard-{k}"
    shard.mkdir()
    script = job / f"shard-{k}.sh"
    script.write_bytes(shard_script(job, k, args, tmp))
    run_unit(script, memory_max())
    if not (shard / "log").exists():
        # The script never ran (sudo refused, the unit could not start): marking
        # the shard done would mark every next shard done the same way.
        raise RuntimeError(f"{SHARD_UNIT} did not start {script}")
    return shard_code(job, k)


def run_shard(job: Path, k: int) -> int:
    """Run shard `k` of `job` here, which this lane has claimed, and mark it done
    with its exit code, whatever the code: the job's result ranks it. The lane's
    first shard of a job runs the baseline, and its timeout is kept for the
    lane's next shards, which skip it. This PC's lane builds under the build
    slot and folds the shard's timings into the cost table; the VPS lane runs
    it through `run_vps_shard`, and its timings, another machine's, are not
    folded. A sent job's marker on the VPS is removed once the shard is done
    here, best effort: `sync` retries. Returns the code."""
    spec = read_job(job)
    shard = job / f"shard-{k}"
    args = [
        *spec["args"], "--shard", f"{k}/{spec['shards']}", "--sharding", "round-robin",
        "--output", str(shard),
    ]
    baseline = job / f"baseline-{LANE}.json"
    if baseline.exists():
        timeout = json.loads(baseline.read_text(encoding="utf-8"))["timeout"]
        args += ["--baseline", "skip", "--timeout", f"{timeout:.0f}"]
    print(f"driver: {job.name} shard {k + 1} of {spec['shards']}: cargo mutants {' '.join(args)}",
          flush=True)
    if LANE == "vps":
        code = run_vps_shard(job, k, args)
    else:
        # Under the build slot a diff run started outside the driver takes too.
        with mutants.run_lock():
            shard.mkdir()
            code = mutants.local_mutants(args, job / "source").returncode
    out = shard / "mutants.out"
    # Only a passed baseline sets the timeout: a failed one (exit 4), or a
    # shard that ran none, leaves the next shard to run the baseline again.
    if not baseline.exists() and code in (0, 2, 3):
        tests = baseline_test_seconds(out)
        if tests is not None:
            config = extra_mutants.mutants_toml(job / "source")
            timeout = extra_mutants.timeout_seconds(config, tests)
            mutants_diff.replace_text(baseline, json.dumps({"timeout": timeout}))
    if LANE == "pc":
        mutants.update_cost_cache(out)
    mutants_diff.replace_text(job / f"shard-{k}.done", str(code))
    (job / f"shard-{k}.{LANE}").unlink()
    host = sent_host(job)
    if host is not None:
        remove_remote(host, f"{remote_job(job)}/shard-{k}.{LANE}")
    return code


def recheck(job: Path, spec: dict) -> list[str]:
    """Rerun here, under the build slot and into `job/recheck/`, the survivors
    of `job`'s VPS shards that only a Windows-only test catches
    (`mutants.WINDOWS_ONLY_TESTS`). A VPS shard's folder holds its `log`,
    which this PC's shards never write; its `exit` is missing when the shard
    was killed. Returns the ones it caught. Off Windows they are printed, and
    none is rerun."""
    lines = [
        line
        for k in range(spec["shards"])
        if (job / f"shard-{k}" / "log").exists()
        for line in mutants.read_lines(job / f"shard-{k}" / "mutants.out" / "missed.txt")
        if any(pattern.search(line) for pattern in mutants.WINDOWS_ONLY_TESTS)
    ]
    if not lines:
        return []
    if not ON_WINDOWS:
        print("Windows-only tests, not rechecked:", flush=True)
        for line in lines:
            print(f"  {line}", flush=True)
        return []
    with mutants.run_lock():
        caught = mutants.recheck_windows(lines, spec["tree"], out=job / "recheck")
    print(f"rechecked on this machine (Windows-only tests): {len(caught)} caught, "
          f"{len(lines) - len(caught)} still missed", flush=True)
    return [line for line in lines if line in caught]


def finish(job: Path) -> Path | None:
    """Once every shard of `job` is done: write its `result.json` (the combined
    code, the survivors in shard order, the untested mutants, the survivors
    `recheck` caught), remove its source and move it to `done/`, and for a
    sent job remove its VPS folder. Returns the new folder, or None while a
    shard is not done."""
    spec = read_job(job)
    done = [job / f"shard-{k}.done" for k in range(spec["shards"])]
    if not all(path.exists() for path in done):
        return None
    codes = [int(path.read_text(encoding="utf-8")) for path in done]
    missed = mutants_diff.missed_lines(job)
    rechecked = recheck(job, spec)
    missed = [line for line in missed if line not in rechecked]
    untested = mutants.killed_builds(job)
    # Untested mutants count as `run_code`'s 1, as a run outside the driver reports them.
    code = mutants.run_code([*codes, 1 if untested else 0])
    # cargo-mutants' 2 says mutants were missed: once the recheck caught them all, none is.
    if rechecked and not missed and code == 2:
        code = 0
    mutants_diff.replace_text(
        job / "result.json",
        json.dumps({"code": code, "missed": missed, "untested": untested, "rechecked": rechecked}),
    )
    if (job / "source").exists():
        mutants.remove_tree(job / "source")
    finished = DRIVER / "done" / job.name
    finished.parent.mkdir(parents=True, exist_ok=True)
    job.rename(finished)
    host = sent_host(finished)
    if host is not None:
        remove_remote(host, remote_job(finished))
    print(f"driver: {spec['crate']} done, exit {code}: {len(missed)} missed or timed out, "
          f"{len(untested)} untested, in {finished}", flush=True)
    return finished


def run(once: bool = False) -> int:
    """This machine's lane (`LANE`): take shards until stopped, or with `once`
    until none is left. Returns 1 at once when another driver runs on this
    machine."""
    DRIVER.mkdir(parents=True, exist_ok=True)
    with open(DRIVER / "guard.lock", "a+b") as guard:
        if not mutants.try_lock(guard):
            print(f"driver: another driver holds {DRIVER / 'guard.lock'}", flush=True)
            return 1
        try:
            return take_shards(once)
        finally:
            mutants.unlock(guard)


def settle() -> None:
    """This PC's lane between its shards: each sent job synced with the VPS
    (`sync`), then each job whose shards are all done finished. So a job
    whose last shard ended on the VPS, or before a driver stopped, is
    finished though no shard of it is left to run here."""
    for job in jobs():
        sync(job)
        finish(job)


def take_shards(once: bool) -> int:
    """The lane's loop. Only this PC's lane finishes jobs: the VPS lane's are
    this PC's to finish once it has their results."""
    recover()
    if LANE == "pc":
        settle()
    waiting = None
    while True:
        reason = blocked()
        if reason is not None:
            if reason != waiting:
                print(f"driver: {reason}", flush=True)
                waiting = reason
            time.sleep(POLL_SECONDS)
            continue
        waiting = None
        if LANE == "pc":
            settle()
        claimed = claim()
        if claimed is None:
            if once:
                return 0
            time.sleep(POLL_SECONDS)
            continue
        job, k = claimed
        run_shard(job, k)
        if LANE == "pc":
            finish(job)


def service_unit(user: str, home: str) -> str:
    """The VPS lane's systemd unit, for `user` whose home is `home` there."""
    folder = f"{home}/{REMOTE_DRIVER}"
    return (
        "[Unit]\n"
        "Description=The mutation driver's VPS lane\n"
        "\n"
        "[Service]\n"
        f"User={user}\n"
        f"ExecStart=/usr/bin/python3 -u {folder}/bin/scripts/mutation/driver.py run --lane vps\n"
        "Restart=always\n"
        "RestartSec=60\n"
        f"StandardOutput=append:{folder}/driver.log\n"
        f"StandardError=append:{folder}/driver.log\n"
        "Nice=10\n"
        "CPUWeight=idle\n"
        "\n"
        "[Install]\n"
        "WantedBy=multi-user.target\n"
    )


def deploy(host: str) -> None:
    """Install the VPS lane on `host` from `HEAD`'s `scripts/mutation/` and
    restart it. A restart stops a running shard, and the restarted lane runs
    it again from the start."""
    folder = f"~/{REMOTE_DRIVER}"
    ssh_archive(
        host, ["HEAD", "scripts/mutation"],
        f"rm -rf {folder}/bin.new && mkdir -p {folder}/bin.new && "
        f"tar -xf - -C {folder}/bin.new && rm -rf {folder}/bin && mv {folder}/bin.new {folder}/bin",
    )
    user, home = mutants.ssh(
        host, 'id -un; echo "$HOME"', capture_output=True, text=True, check=True
    ).stdout.splitlines()
    # Bytes, not text: `text=True` would turn `\n` into `\r\n` on Windows.
    mutants.ssh(
        host, "sudo -n tee /etc/systemd/system/studio-driver.service > /dev/null",
        input=service_unit(user, home).encode("utf-8"), check=True,
    )
    mutants.ssh(
        host,
        "sudo -n systemctl daemon-reload && sudo -n systemctl enable studio-driver && "
        "sudo -n systemctl restart studio-driver",
        check=True,
    )
    print(f"driver: the VPS lane installed on {host} and restarted", flush=True)


def main(argv: list[str]) -> int:
    global DRIVER, LANE
    parser = argparse.ArgumentParser(prog="driver", allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)
    queued = commands.add_parser("queue", help="add a whole-crate job", allow_abbrev=False)
    queued.add_argument("crate")
    queued.add_argument("--shards", type=int)
    running = commands.add_parser("run", help="take shards on this machine", allow_abbrev=False)
    running.add_argument("--once", action="store_true")
    running.add_argument("--lane", choices=["pc", "vps"], default="pc")
    commands.add_parser(
        "deploy", help="install the VPS lane from HEAD and restart it", allow_abbrev=False
    )
    options = parser.parse_args(argv[1:])
    if options.command == "queue":
        if options.shards is not None and options.shards < 1:
            parser.error("--shards must be at least 1")
        queue(options.crate, options.shards)
        return 0
    if options.command == "deploy":
        host = mutants.remote_host()
        if host is None:
            parser.error("no VPS: STUDIO_MUTANTS_REMOTE names none")
        deploy(host)
        return 0
    if options.lane == "vps":
        DRIVER, LANE = VPS_DRIVER, "vps"
    return run(options.once)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
