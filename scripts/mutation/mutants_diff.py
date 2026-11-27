"""`just mutants-diff [base] [path...]`: mutation-test only the lines changed since `base`,
under `crates` or the paths given (a crate's folder).

The diff runs from `base` (default HEAD) to the working tree, untracked files included, so a
review measures the tests of the code under review rather than the whole workspace
(CONTRIBUTING.md "Testing and verification", "Mutation runs").

Two other modes share the same run. `--closure [crate ...]` (`just mutants-closure`) diffs
each crate — the named ones, or every workspace crate — since the commit
`.cargo/mutants_checked.toml` records as the one its last triaged whole-crate run was
launched from, instead of `base`; `--checked <crate> <commit>` (`just mutants-checked`)
writes that record once the run's survivors are triaged.

The mutation driver (`driver.py`) runs the mutants, so it must be running on this machine
(`just driver run`); the command refuses with exit 1 otherwise. The command queues one
driver job per crate a changed file under `crates/` belongs to, each carrying the diff: the
job's cargo-mutants shards test the diff's mutants alone (`--in-diff`), and its extra
shards the extra kinds (`mutants.EXTRA_KINDS`, `extra_mutants.py`) on the diff's changed
lines. Each crate gets a job of its own, not one for the whole diff, because cargo-mutants
times a mutant's tests out at a multiple of the baseline, and one run's baseline tests
every crate it mutates: a hung mutant of a crate whose tests take two seconds waited out
the slowest crate's suite (751 s at `mutants-diff 95f20eac`). A crate with neither kind of
mutant on the changed lines gets no job.

The command then waits for its jobs, holding no lock, and moves each finished one from the
driver's `done/` folder to `mutants.diff/<crate>/`. `mutants.diff/missed_all.txt` lists
every crate's survivors and, tagged `[timeout]`, its timeouts; the exit code combines the
jobs' (`mutants.run_code`). A command stopped while it waits leaves its jobs running, and
their results then stay in the driver's `done/` folder.
"""

import io
import json
import os
import subprocess
import sys
import tempfile
import time
import tomllib
from collections.abc import Iterable
from pathlib import Path

import extra_mutants
import mutants

ROOT = Path(__file__).resolve().parents[2]
# Each crate's driver job of the last diff run, and their survivors' `missed_all.txt`.
RESULTS = ROOT / "mutants.diff"
# The crates' commits of record for a `--closure` run's diffs, written by `--checked`.
CHECKED = ROOT / ".cargo" / "mutants_checked.toml"
# The rolling job's last whole-crate run of each crate, `<crate>/` as the driver
# files it, for the triage that ends with `--checked` recording the commit its
# `launched_from.txt` holds.
ROLLING = ROOT / "mutants.rolling"


def read_checked(path: Path = CHECKED) -> dict[str, str]:
    """The `[checked]` entries of `path` (`CHECKED`): each crate's commit its
    last triaged whole run was launched from. A missing file reads as none."""
    if not path.exists():
        return {}
    return dict(tomllib.loads(path.read_text(encoding="utf-8"))["checked"])


def write_checked(crate: str, commit: str, path: Path = CHECKED) -> None:
    """Move `crate`'s entry in `path` to `commit`, keeping the file's lines
    before its table and listing the entries by crate name. Written to a sibling temp
    file and moved over `path`: a write stopped midway never leaves a partial
    file standing as the record."""
    entries = {**read_checked(path), crate: commit}
    header = []
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines(keepends=True):
            if line.startswith("["):
                break
            header.append(line)
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(header)
            f.write("[checked]\n")
            for name in sorted(entries):
                f.write(f'{name} = "{entries[name]}"\n')
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def main(argv: list[str]) -> int:
    # A survivor's line quotes the source, any character of it, and a log file
    # opened in Windows' cp1252 refuses some. A test's StringIO takes them all.
    if isinstance(sys.stdout, io.TextIOWrapper):
        sys.stdout.reconfigure(encoding="utf-8")
    if len(argv) > 1 and argv[1] == "--checked":
        return mark_checked(argv[2:])
    if len(argv) > 1 and argv[1] == "--closure":
        return closure(argv[2:])
    base = argv[1] if len(argv) > 1 else "HEAD"
    # A crate's folder narrows the diff to it: since an old base, `crates` as a whole holds
    # every other crate's changes too (6690 mutants over 20 crates for one crate's 568).
    paths = argv[2:] or ["crates"]
    # Against the whole working tree as a tree object, not `git diff <base>`,
    # which leaves out untracked files: a new module's mutants would silently
    # go unmeasured unless the tree was staged first (found at 3.8).
    # Bytes, not text: the file must be LF on both sides (text mode writes
    # CRLF on Windows).
    tree = mutants.working_tree()
    diffed = subprocess.run(
        ["git", "diff", base, tree, "--", *paths],
        cwd=ROOT,
        capture_output=True,
    )
    if diffed.returncode != 0:
        print(f"mutants_diff: git diff failed:\n{diffed.stderr.decode(errors='replace')}")
        return 1
    return diff_run(diffed.stdout, tree, f"under {' '.join(paths)} since {base}")


def mark_checked(args: list[str]) -> int:
    """`--checked <crate> [<commit>]`: record `commit` as the one `crate`'s
    last triaged whole run was launched from, moving its `--closure` diff past
    it. Without `commit`, the crate's kept rolling run's; with one, it must be
    that run's when the crate has one."""
    if len(args) not in (1, 2):
        print("usage: mutants_diff.py --checked <crate> [<commit>]")
        return 1
    crate = args[0]
    if crate not in extra_mutants.workspace_crates(ROOT):
        print(f"mutants_diff: no workspace crate named {crate}")
        return 1
    # A rolling run's commit is read, never typed: `HEAD`, the natural thing to
    # type at triage, would record lines the run never measured.
    record = ROLLING / crate / "launched_from.txt"
    launched = record.read_text(encoding="utf-8").strip() if record.exists() else None
    if len(args) == 1 and launched is None:
        print(
            f"mutants_diff: no rolling run of {crate} on record ({record}): name "
            "the commit its whole run's first line gives"
        )
        return 1
    commit = args[1] if len(args) == 2 else launched
    resolved = subprocess.run(
        ["git", "rev-parse", "--verify", f"{commit}^{{commit}}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if resolved.returncode != 0:
        print(f"mutants_diff: {commit} does not resolve to a commit")
        return 1
    sha = resolved.stdout.strip()
    if launched is not None and sha != launched:
        print(
            f"mutants_diff: {crate}'s kept rolling run was launched from {launched}, "
            f"not {sha}: triage that run, or move `{record.parent}` aside to record a "
            "run started by hand"
        )
        return 1
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", sha, "HEAD"],
        cwd=ROOT,
        capture_output=True,
    )
    if ancestor.returncode != 0:
        print(f"mutants_diff: {sha} is not an ancestor of HEAD")
        return 1
    write_checked(crate, sha, CHECKED)
    print(f"mutants_diff: {crate} checked at {sha}")
    return 0


def closure(crates: list[str]) -> int:
    """`--closure [crate ...]`: the same diff run over what each named crate —
    every workspace crate when none is named — changed since the commit its
    last triaged whole run is recorded at in `CHECKED`. A crate with no record
    is named and joined into the run's code as untested: nothing it changed was
    ever covered by a full check."""
    dirs = extra_mutants.workspace_crates(ROOT)
    unknown = [name for name in crates if name not in dirs]
    if unknown:
        print(f"mutants_diff: no workspace crate named {', '.join(unknown)}")
        return 1
    names = crates or sorted(dirs)
    checked = read_checked(CHECKED)
    missing = [name for name in names if name not in checked]
    if missing:
        print(
            f"mutants_diff: no triaged whole run on record for "
            f"{', '.join(missing)}: `just mutants <crate>` checks them"
        )
    # The whole working tree as a tree object, as the plain mode takes it.
    tree = mutants.working_tree()
    parts = []
    for name in names:
        if name not in checked:
            continue
        folder = dirs[name].relative_to(ROOT).as_posix()
        diffed = subprocess.run(
            ["git", "diff", checked[name], tree, "--", folder],
            cwd=ROOT,
            capture_output=True,
        )
        if diffed.returncode != 0:
            print(f"mutants_diff: git diff failed:\n{diffed.stderr.decode(errors='replace')}")
            return 1
        if diffed.stdout.strip():
            parts.append(diffed.stdout)
    return diff_run(
        b"".join(parts), tree, "since the crates' recorded commits",
        [1] if missing else [],
    )


def diff_run(diff: bytes, tree: str, where: str, codes: Iterable[int] = ()) -> int:
    """The tail both diff modes share: `diff` the bytes the run measures,
    `where` their description for the no-changes message, `codes` a code the
    mode owes on top of the run's (`--closure`'s 1 for a crate with no record:
    its mutants went untested)."""
    if not diff.strip():
        print(f"mutants_diff: no changes {where}")
        # Nothing to measure, and an older run's results never stand as this one's; moved in
        # this run's turn, since a run in progress writes them.
        with mutants.run_lock(mutants.tree_lock_path()):
            mutants.set_aside(RESULTS)
        return mutants.run_code([0, *codes])
    return mutants.run_code([run_jobs(diff, tree), *codes])


def run_jobs(diff: bytes, tree: str) -> int:
    """Queue `diff`'s driver jobs, on `tree` (the working tree it was taken
    against), one per crate a changed file under `crates/` belongs to, and
    wait for them: each finished job moves to `RESULTS/<crate>/`, and their
    survivors go into `RESULTS/missed_all.txt`. Returns the jobs' combined
    code; 0 when no crate has a mutant on the diff's lines; 1, queuing
    nothing, when no driver runs on this machine."""
    # Imported here, not at the top: `driver` imports this module.
    import driver

    if not driver.running():
        print("mutants_diff: no mutation driver runs on this machine: start `just driver "
              "run`, then run this command again")
        return 1
    # Every crate a cargo-mutants mutant of the diff sits in has a changed
    # file, and so does every crate with an extra mutant on its lines.
    crates = sorted({
        mutants.mutant_crate(path)
        for path in extra_mutants.diff_lines(diff)
        if path.startswith("crates/")
    })
    with mutants.run_lock(mutants.tree_lock_path()):
        # Even with no job to queue: an older run's results never stand as this one's.
        mutants.set_aside(RESULTS)
    jobs = {}
    for crate in crates:
        job = driver.queue(crate, extra=None, diff=diff, tree=tree)
        if job is not None:
            jobs[crate] = job.name
    if not jobs:
        print(f"mutants_diff: no crate's results landed (the last run's, if any: {RESULTS.name}.old)")
        return 0
    results = wait_for(jobs, driver.DRIVER / "done", driver.POLL_SECONDS)
    survivors = [line for crate in sorted(results) for line in results[crate]["missed"]]
    with mutants.run_lock(mutants.tree_lock_path()):
        (RESULTS / "missed_all.txt").write_text(
            "".join(line + "\n" for line in survivors), encoding="utf-8"
        )
    print(f"mutants_diff: results in {RESULTS.name}/<crate>/")
    for crate in sorted(results):
        print(f"  {crate}: exit {results[crate]['code']}")
    if survivors:
        print(f"missed or timed out ({len(survivors)}), in {RESULTS.name}/missed_all.txt:")
        for line in survivors:
            print(f"  {line}")
    return mutants.run_code([result["code"] for result in results.values()])


def wait_for(jobs: dict[str, str], done: Path, poll_seconds: float) -> dict[str, dict]:
    """Wait for each crate's job in `jobs` (crate -> job name) to finish in the
    driver's `done` folder, polling every `poll_seconds`, and move each to
    `RESULTS/<crate>/` as it does. Returns each crate's `result.json`.
    No lock is held while it waits: this machine's driver takes the tree lock
    of the tree it runs from to file a rolling job, so a diff run holding it
    in that tree would stop the very driver it waits for."""
    print(f"mutants_diff: waiting for the driver's jobs: {', '.join(jobs.values())}", flush=True)
    results = {}
    while True:
        for crate, name in jobs.items():
            finished = done / name
            if crate in results or not (finished / "result.json").exists():
                continue
            result = json.loads((finished / "result.json").read_text(encoding="utf-8"))
            with mutants.run_lock(mutants.tree_lock_path()):
                RESULTS.mkdir(exist_ok=True)
                mutants.move_results(finished, RESULTS / crate)
            results[crate] = result
        if len(results) == len(jobs):
            return results
        time.sleep(poll_seconds)


def replace_text(path: Path, text: str) -> None:
    """Write `path` through a sibling temp file: an interruption mid-write
    leaves the old file or none, never a torn one a rerun cannot read."""
    temp = path.with_name(path.name + ".tmp")
    temp.write_text(text, encoding="utf-8", newline="\n")
    os.replace(temp, path)


def missed_lines(out: Path) -> list[str]:
    """The survivors of one crate's finished run in `out` (its `mutants.out`): a split run's
    summary list, which merges both halves, their timeouts included, and drops what the
    Windows recheck caught; a driver job's shards' lists, in shard order; else a local
    run's own list and its timeouts (`mutants.timeout_lines`)."""
    if shards := mutants.shard_outputs(out):
        return [line for shard in shards for line in missed_lines(shard)]
    summary = out / "missed_all.txt"
    if summary.exists():
        return mutants.read_lines(summary)
    return mutants.read_lines(out / "missed.txt") + mutants.timeout_lines(out)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
