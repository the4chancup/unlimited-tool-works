"""`just mutants-rolling`: whole-crate mutation runs on the VPS, one crate after
another, nobody waiting on them.

A phase's close measures only what changed (`just mutants-closure`, a diff since
each crate's last triaged whole run); this job keeps the whole runs themselves
going off that path, since only they catch what no diff shows, such as a test
weakened in a file whose source did not change. Each turn it picks the crate
checked longest ago (the later of its `.cargo/mutants_checked.toml` commit and
the commit its last rolling run was launched from,
`mutants.rolling/<crate>/launched_from.txt`; a crate with neither comes first)
and runs `mutants.whole_run` on that commit's tree alone, never the working
tree's uncommitted changes. The run's results stay under `mutants.rolling/<crate>/`
(`out/` the fetched `mutants.out`, `extra/` its extra-mutant pass, `exit.txt`
its code) for the triage that ends with `just mutants-checked <crate>`, which
reads the commit from `launched_from.txt`.

The lead starts it detached and leaves it running:

    python ~/.claude/tools/run_detached.py <log> just mutants-rolling

`--once` runs a single crate and exits with its code. `.tmp/rolling.hold` is the
hold file: while it stands the job waits between crates, the way to free the VPS
for the checkpoint's Linux gates before a push; delete it and it goes on. A run
the VPS refuses (busy) is retried after a wait, never counted, and so is one
that fails; a run the job lost (it was stopped, or its wait failed) is
collected and filed under its crate at the next turn.
"""

import io
import sys
import time
import traceback
from collections.abc import Callable, Iterable
from pathlib import Path

import extra_mutants
import mutants
import mutants_diff

ROOT = Path(__file__).resolve().parents[2]
# Each crate's kept whole-crate runs, `mutants.rolling/<crate>/`, until their
# survivors are triaged and `mutants-checked` records the launched commit.
ROLLING = ROOT / "mutants.rolling"
# Created, the job finishes its crate and waits between crates; deleted, it goes on.
HOLD = ROOT / ".tmp" / "rolling.hold"
# The crate and commit of the run under way, so a run the job lost (it was
# stopped, or its wait failed) is collected and filed at the next turn.
CURRENT = ROLLING / "current.txt"
# Both waits in minutes, like everything about a job whose runs take days: how
# often a hold is re-checked, and how long a refused run waits before picking again.
HOLD_POLL_MINUTES = 5
RETRY_MINUTES = 30


def pick(
    crates: Iterable[str], checked: dict[str, str],
    launched: dict[str, int], date: Callable[[str], int],
) -> str:
    """The crate to run next: the one whose last check is oldest. A crate's last
    check is the later of the commit `checked` records for it, dated by `date`
    (a commit's committer time), and the time its kept rolling run ended
    (`launched`): a run's launch commit can be days older than the run, and
    while HEAD stands still every crate's would tie. A crate with neither was
    never checked and comes first, and a tie breaks on the name."""

    def last_check(crate: str) -> int:
        dates = [date(checked[crate])] if crate in checked else []
        if crate in launched:
            dates.append(launched[crate])
        return max(dates) if dates else -1

    return min(crates, key=lambda crate: (last_check(crate), crate))


def launched_times() -> dict[str, int]:
    """When each crate's kept run ended: the time its
    `ROLLING/<crate>/launched_from.txt` was written, the run's last write. A
    run stopped midway writes none, so only finished runs count."""
    return {
        record.parent.name: int(record.stat().st_mtime)
        for record in ROLLING.glob("*/launched_from.txt")
    }


def commit_date(commit: str) -> int:
    """`commit`'s committer time, seconds since the epoch."""
    return int(
        mutants.git(
            "show", "-s", "--format=%ct", commit,
            check=True, capture_output=True, text=True,
        ).stdout
    )


def run_one(crate: str, host: str) -> int:
    """One crate's whole run: `mutants.whole_run` on HEAD's commit tree (the
    commit alone, never the working tree's uncommitted changes), the results
    kept under `ROLLING / crate` for the later triage. `launched_from.txt` is
    the last write (after `exit.txt`), so a run stopped midway leaves the crate
    undated and comes up for a fresh pick."""
    commit = mutants.git(
        "rev-parse", "HEAD", check=True, capture_output=True, text=True
    ).stdout.strip()
    ROLLING.mkdir(parents=True, exist_ok=True)
    CURRENT.write_text(f"{crate} {commit}\n", encoding="utf-8", newline="\n")
    code = mutants.whole_run(crate, host, f"{commit}^{{tree}}", commit)
    file_results(crate, commit, code)
    return code


def file_results(crate: str, commit: str, code: int) -> None:
    """A finished run's local results moved under `ROLLING / crate` under the
    tree lock (its callers hold none), the crate's earlier run set aside, and
    `CURRENT` cleared. With no `mutants.out` to move — a same-tree run set it
    aside in the gap after the finish's lock — nothing is filed but the word:
    the crate is not recorded as run and comes up for a fresh pick."""
    with mutants.run_lock(mutants.tree_lock_path()):
        out = ROOT / "mutants.out"
        if not out.exists():
            print(
                f"rolling: {crate} left no mutants.out to file (exit {code}): "
                "the crate is unfiled and will be picked again",
                flush=True,
            )
            CURRENT.unlink(missing_ok=True)
            return
        folder = ROLLING / crate
        mutants.set_aside(folder)
        folder.mkdir(parents=True, exist_ok=True)
        out.rename(folder / "out")
        # A whole run's fetch lands the extra pass's results beside the remote
        # half's (`remote_extra_record`).
        extra = folder / "out" / "remote" / "extra_mutants.out"
        if extra.exists():
            extra.rename(folder / "extra")
        (folder / "exit.txt").write_text(f"{code}\n", encoding="utf-8", newline="\n")
        (folder / "launched_from.txt").write_text(
            f"{commit}\n", encoding="utf-8", newline="\n"
        )
        CURRENT.unlink(missing_ok=True)


def recover(host: str) -> None:
    """A run the job lost track of (it was stopped, or its wait failed) left
    `CURRENT`: when this tree's launch record is a whole run's (no local half),
    its half is collected (`mutants.collect` waits for one still running) and
    filed as `run_one` would have. Any other record is another run's, left to
    its owner, and `CURRENT` goes."""
    if not CURRENT.exists():
        return
    crate, commit = CURRENT.read_text(encoding="utf-8").split()
    record = mutants.pending_record() if mutants.remote_pending().exists() else None
    if record is not None and not record.get("local_half", True) and not record.get(
        "extra_only"
    ):
        print(f"rolling: collecting {crate}'s run, launched from {commit}", flush=True)
        file_results(crate, commit, mutants.collect(host))
    CURRENT.unlink(missing_ok=True)


def main(argv: list[str]) -> int:
    """`just mutants-rolling [--once]`: the loop the module docstring describes,
    ending only when it is stopped (or, with `--once`, after one crate)."""
    # A survivor's line quotes the source, any character of it, and a log file
    # opened in Windows' cp1252 refuses some. A test's StringIO takes them all.
    if isinstance(sys.stdout, io.TextIOWrapper):
        sys.stdout.reconfigure(encoding="utf-8")
    host = mutants.remote_host()
    if host is None:
        print(
            "rolling: STUDIO_MUTANTS_REMOTE names no host, and "
            "whole-crate runs go to the VPS alone",
            flush=True,
        )
        return 1
    once = "--once" in argv[1:]
    while True:
        if HOLD.exists():
            print(
                f"rolling: held by {HOLD.relative_to(ROOT).as_posix()}",
                flush=True,
            )
            while HOLD.exists():
                time.sleep(HOLD_POLL_MINUTES * 60)
            print("rolling: hold lifted", flush=True)
        recover(host)
        crates = sorted(
            set(extra_mutants.workspace_crates(ROOT)) - mutants.LOCAL_ONLY_CRATES
        )
        crate = pick(
            crates,
            mutants_diff.read_checked(mutants_diff.CHECKED),
            launched_times(),
            commit_date,
        )
        print(f"rolling: {crate}", flush=True)
        try:
            code = run_one(crate, host)
        except mutants.Refused as refusal:
            print(refusal, flush=True)
            if once:
                return 1
            time.sleep(RETRY_MINUTES * 60)
            continue
        except Exception:
            # A died half, an hour of failed polls, a dropped link: the run
            # ends, not the job, and the next turn's `recover` collects what
            # the remote still holds.
            traceback.print_exc(file=sys.stdout)
            sys.stdout.flush()
            if once:
                return 1
            time.sleep(RETRY_MINUTES * 60)
            continue
        if once:
            return code


if __name__ == "__main__":
    sys.exit(main(sys.argv))
