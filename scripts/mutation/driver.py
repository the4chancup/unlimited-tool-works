"""`just driver`: the mutation driver, one process per machine taking
cargo-mutants shards from a queue of jobs.

`just driver queue <crate> [--shards N]` adds a job: a whole-crate run of
`<crate>` at the committed `HEAD`, cut into shards of about `SHARD_SECONDS`.
A lane is a machine taking the queue's shards, named in the markers below
(`pc` for this one). `just driver run [--once]` starts this PC's lane, which
runs one shard at a time, oldest job first, and then waits for more; with
`--once` it stops when nothing is left. So queued work waits at most one
shard, and where several lanes share the queue, the faster one takes more.

Everything lives under `DRIVER`, one folder per machine shared by every tree:

    guard.lock               held by the running driver for its whole life
    hold-pc                  while it exists, this lane claims no new shard
    queue/<job>/             <job> is <UTC %Y%m%dT%H%M%S>-<crate>; name order is queue order
      job.json               {"crate", "tree", "args", "shards", "queued"}
      source/                the job's tree, extracted once
      shard-<k>.todo         an empty marker for each shard not taken yet
      shard-<k>.<lane>       the marker, renamed by the lane that took the shard
      shard-<k>/             the shard's cargo-mutants `--output` folder
      shard-<k>.done         the shard's exit code, as text, written last
      baseline-<lane>.json   {"timeout": seconds}, after the lane's first shard of the job
    done/<job>/              the job once every shard is done, with result.json

A lane takes a shard by renaming its marker, which only one lane can do.
A restarted driver goes on from the folders:

- each shard its lane had taken but not ended goes back to `.todo`, its
  partial results removed;
- each job whose shards all ended is finished.

An error ends the driver as a stop does, and the next start resumes the same way.
"""

import argparse
import json
import math
import os
import sys
import time
from pathlib import Path

import extra_mutants
import mutants
import mutants_diff

# Beside the cost table: this machine's, whichever tree queues or runs.
DRIVER = mutants.COST_CACHE.with_name("driver")
LANE = "pc"
SHARD_SECONDS = mutants_diff.SHARD_SECONDS
POLL_SECONDS = 60


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
    Returns the job's folder."""
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
    building.rename(job)
    print(f"driver: {crate} queued in {shards} shards, as {job.name}", flush=True)
    return job


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
    `.done` was written (the driver died before removing the marker) stays done."""
    for marker in (DRIVER / "queue").glob(f"*/shard-*.{LANE}"):
        if marker.with_suffix(".done").exists():
            marker.unlink()
            continue
        # Removed before the marker goes back, so a shard waiting as `.todo`
        # never has old results that cargo-mutants would keep beside the new.
        partial = marker.with_suffix("")
        if partial.exists():
            mutants.remove_tree(partial)
        marker.rename(marker.with_suffix(".todo"))


def claim() -> tuple[Path, int] | None:
    """Take the next shard: the oldest job's lowest `.todo` one, renamed to this
    lane's marker. Returns the job and the shard, or None when no shard is left."""
    for job in jobs():
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


def run_shard(job: Path, k: int) -> int:
    """Run shard `k` of `job` here, which this lane has claimed, and mark it done
    with its exit code, whatever the code: the job's result ranks it. The lane's
    first shard of a job runs the baseline, and its timeout is kept for the
    lane's next shards, which skip it. Returns the code."""
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
    mutants.update_cost_cache(out)
    mutants_diff.replace_text(job / f"shard-{k}.done", str(code))
    (job / f"shard-{k}.{LANE}").unlink()
    return code


def finish(job: Path) -> Path | None:
    """Once every shard of `job` is done: write its `result.json` (the combined
    code, the survivors in shard order, the untested mutants), remove its
    source and move it to `done/`. Returns the new folder, or None while a
    shard is not done."""
    spec = read_job(job)
    done = [job / f"shard-{k}.done" for k in range(spec["shards"])]
    if not all(path.exists() for path in done):
        return None
    codes = [int(path.read_text(encoding="utf-8")) for path in done]
    missed = mutants_diff.missed_lines(job)
    untested = mutants.killed_builds(job)
    # Untested mutants count as `run_code`'s 1, as a run outside the driver reports them.
    code = mutants.run_code([*codes, 1 if untested else 0])
    mutants_diff.replace_text(
        job / "result.json", json.dumps({"code": code, "missed": missed, "untested": untested})
    )
    if (job / "source").exists():
        mutants.remove_tree(job / "source")
    finished = DRIVER / "done" / job.name
    finished.parent.mkdir(parents=True, exist_ok=True)
    job.rename(finished)
    print(f"driver: {spec['crate']} done, exit {code}: {len(missed)} missed or timed out, "
          f"{len(untested)} untested, in {finished}", flush=True)
    return finished


def run(once: bool = False) -> int:
    """This PC's lane: take shards until stopped, or with `once` until none is
    left. Returns 1 at once when another driver runs on this machine."""
    DRIVER.mkdir(parents=True, exist_ok=True)
    with open(DRIVER / "guard.lock", "a+b") as guard:
        if not mutants.try_lock(guard):
            print(f"driver: another driver holds {DRIVER / 'guard.lock'}", flush=True)
            return 1
        try:
            return take_shards(once)
        finally:
            mutants.unlock(guard)


def take_shards(once: bool) -> int:
    recover()
    # A driver stopped between a job's last shard and its finish left it queued
    # with no shard to claim, so no later shard would finish it.
    for job in jobs():
        finish(job)
    hold = DRIVER / f"hold-{LANE}"
    held = False
    while True:
        if hold.exists():
            if not held:
                print(f"driver: held by {hold}: no new shard until it is deleted", flush=True)
                held = True
            time.sleep(POLL_SECONDS)
            continue
        held = False
        claimed = claim()
        if claimed is None:
            if once:
                return 0
            time.sleep(POLL_SECONDS)
            continue
        job, k = claimed
        run_shard(job, k)
        finish(job)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="driver", allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)
    queued = commands.add_parser("queue", help="add a whole-crate job", allow_abbrev=False)
    queued.add_argument("crate")
    queued.add_argument("--shards", type=int)
    running = commands.add_parser("run", help="take shards on this PC", allow_abbrev=False)
    running.add_argument("--once", action="store_true")
    options = parser.parse_args(argv[1:])
    if options.command == "queue":
        if options.shards is not None and options.shards < 1:
            parser.error("--shards must be at least 1")
        queue(options.crate, options.shards)
        return 0
    return run(options.once)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
