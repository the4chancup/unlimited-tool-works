"""`just mutants-diff [base] [path...]`: mutation-test only the lines changed since `base`,
under `crates` or the paths given (a crate's folder).

Runs `cargo mutants --in-diff` over the diff from `base` (default HEAD) to the working tree,
untracked files included, so a review measures the tests of the code under review rather than the whole
workspace (CONTRIBUTING.md "Testing and verification", "Mutation runs"). The diff goes through
a file because the justfile forbids pipes and redirection in recipe lines.

Two other modes share the same run. `--closure [crate ...]` (`just mutants-closure`) diffs
each crate — the named ones, or every workspace crate — since the commit
`.cargo/mutants_checked.toml` records as the one its last triaged whole-crate run was
launched from, instead of `base`; `--checked <crate> <commit>` (`just mutants-checked`)
writes that record once the run's survivors are triaged.

Each crate the diff mutates gets a cargo-mutants run of its own (`-p <crate>`), one after the
other, not one run for the whole diff: cargo-mutants times a mutant's tests out at a multiple of
the baseline, and one run's baseline tests every crate it mutates, so a hung mutant of a crate
whose tests take two seconds waited out the slowest crate's suite (751 s here at `mutants-diff
95f20eac`). A run costs one more baseline per crate, about a minute. Each crate's results move
to `mutants.diff/<crate>/` when its run ends, and `mutants.diff/missed_all.txt` lists every
crate's survivors and, tagged `[timeout]`, its timeouts.

After each crate's cargo-mutants run the extra kinds (`mutants.EXTRA_KINDS`,
`extra_mutants.py`) run too, on this machine and over the lines the diff changed in its
crate (`extra_mutants.diff_lines`): every crate a changed file belongs to gets its
pass, cargo-mutants mutants or not — a diff can hold `call` mutants where
cargo-mutants lists none. A pass's results move to `mutants.diff/<crate>/extra/` and
its survivors join `missed_all.txt` after the crate's cargo-mutants ones.

A crate run whose mutants would cost more than a few minutes locally splits with
`STUDIO_MUTANTS_REMOTE` (the split `mutants.py` describes, by files to the two
sides' measured speeds where it can, else in halves): the
estimate is the baseline plus the per-crate per-mutant costs this machine has
measured (`mutants.estimate_seconds`), so a pes_savefile diff splits while a
one-line `fpc` fix stays local. While the remote is busy — a whole-crate run
holds it for days — the crate runs here whole instead. A crate in
`mutants.LOCAL_ONLY_CRATES` never splits. With `mutants.REMOTE_ONLY` set every
cargo-mutants mutant runs on the remote, whatever the estimate; the extra
kinds still run here.
"""

import hashlib
import io
import json
import math
import os
import subprocess
import sys
import tempfile
import tomllib
from collections import Counter
from collections.abc import Iterable
from pathlib import Path

import extra_mutants
import mutants

ROOT = Path(__file__).resolve().parents[2]
# Each crate's `mutants.out` of the last diff run, and their survivors' `missed_all.txt`.
RESULTS = ROOT / "mutants.diff"
# The crates' commits of record for a `--closure` run's diffs, written by `--checked`.
CHECKED = ROOT / ".cargo" / "mutants_checked.toml"
# The longest a diff run's crates run here, in total, while another run's half is on the
# VPS. Not unbounded: this run holds the run lock throughout, that half's collect waits for
# it, and the VPS stays idle meanwhile.
BUSY_LOCAL_LIMIT_SECONDS = 3600
# A crate run here estimated past this runs as cargo-mutants shards of about this length
# (`sharded_run`), each kept as it ends: an interruption (a restart, a crash) then loses
# one shard, not the crate's run, and the same command rerun goes on from there.
SHARD_SECONDS = 1800
# The shards of a crate run under way, a folder per crate, until the run makes them its
# `mutants.out`.
SHARDS = ROOT / "mutants.shards"


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
    import rolling  # deferred: rolling imports this module

    # A rolling run's commit is read, never typed: `HEAD`, the natural thing to
    # type at triage, would record lines the run never measured.
    record = rolling.ROLLING / crate / "launched_from.txt"
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
        with mutants.run_lock():
            mutants.set_aside(RESULTS)
        return mutants.run_code([0, *codes])
    with tempfile.NamedTemporaryFile("wb", suffix=".diff", delete=False) as f:
        f.write(diff)
        diff_path = f.name
    try:
        with mutants.run_lock(), mutants.snapshot_source(tree) as source:
            return mutants.run_code([run(diff_path, tree, source, diff), *codes])
    finally:
        os.unlink(diff_path)


def run(diff_path: str, tree: str, source: Path, diff: bytes) -> int:
    """Lists the diff's mutants and runs each crate's (`run_crate`), keeping each crate's
    results under `RESULTS`. `tree` is the working tree the diff was taken against, which the
    remote is sent, `source` its copy (`mutants.snapshot_source`), which the listing and
    the local halves read, and `diff` the diff's bytes, whose changed lines
    (`extra_mutants.diff_lines`) the extra kinds' passes run on."""
    # The mutant list, for the printed line and the local-run estimate. Each
    # entry's `file` names the crate (`crates/<group>/<crate>/...`).
    listing = subprocess.run(
        ["cargo", "mutants", "--in-diff", diff_path, "--list", "--json"],
        cwd=source,
        capture_output=True,
    )
    if listing.returncode != 0:
        print(f"mutants_diff: listing failed:\n{listing.stderr.decode(errors='replace')}")
        return 1
    listed = listing.stdout
    # A diff with no mutable lines prints nothing at all (an "INFO" line on
    # stderr), not an empty array.
    found = json.loads(listed) if listed.strip() else []
    counts = Counter(mutants.mutant_crate(m["file"]) for m in found)
    names = ", ".join(f"{crate}: {count}" for crate, count in sorted(counts.items()))
    print(f"mutants_diff: {len(found)} mutants ({names}), one run per crate")
    results = RESULTS
    # Even with no mutant to run: an older run's results never stand as this one's.
    mutants.set_aside(results)
    # The extra kinds run for every crate a changed file under `crates/` belongs
    # to, cargo-mutants mutants or not: a diff can hold `call` mutants where it
    # lists none. A standalone extra run's `extra_mutants.out` is not this run's,
    # so it is set aside like the rest.
    lines = extra_mutants.diff_lines(diff)
    # Before anything is set aside: an uncollected split's local half keeps its
    # record in `extra_mutants.out`, which its collection reads.
    if refusal := mutants.local_refusal():
        print(f"mutants_diff: {refusal}")
        return 1
    crates = {
        mutants.mutant_crate(path) for path in lines if path.startswith("crates/")
    }
    mutants.set_aside(extra_mutants.OUT_DIR)
    if not found and not crates:
        return 0
    codes = {}
    busy = Counter()
    extra_missed: dict[str, list[str]] = {}
    for crate in sorted(counts.keys() | crates):
        count = counts.get(crate, 0)
        print(f"mutants_diff: {crate}, {count} mutants", flush=True)
        if count:
            try:
                codes[crate] = run_crate(diff_path, tree, source, crate, count, busy=busy)
            except mutants.Refused as refusal:
                # Refused before touching `mutants.out`: what is there is no result of this crate.
                print(f"mutants_diff: {refusal}")
                codes[crate] = 1
                continue
            out = ROOT / "mutants.out"
            if out.exists():
                # Made with the first results that land, so a run that lands none leaves no empty
                # folder to be set aside over the last results.
                results.mkdir(exist_ok=True)
                out.rename(results / crate)
        try:
            missed, extra_counts, _ = extra_mutants.run_crate(
                crate, list(mutants.EXTRA_KINDS), tree, lines=lines
            )
        except SystemExit as stop:
            # A failing baseline or a broken config ends this crate's pass, as
            # cargo-mutants' exit 4 ends its run, and the crates after it run.
            print(f"mutants_diff: {crate}'s extra pass stopped: {stop}")
            missed, extra_counts = [], Counter(untested=1)
        extra_missed[crate] = missed
        # The extra pass's code joins the crate codes `run_code` combines: 1
        # when a mutant went untested, 2 when one was missed, else 0.
        codes[crate] = mutants.run_code(
            [
                codes.get(crate, 0),
                1 if extra_counts["untested"] else 2 if missed else 0,
            ]
        )
        extra_out = extra_mutants.OUT_DIR
        if extra_out.exists():
            results.mkdir(exist_ok=True)
            (results / crate).mkdir(exist_ok=True)
            extra_out.rename(results / crate / "extra")
    survivors = [
        line
        for crate in sorted(codes)
        for line in missed_lines(results / crate) + extra_missed.get(crate, [])
    ]
    # An extra pass that kept no mutant writes no folder, yet its stale
    # exclusions are survivors all the same.
    if not results.exists() and not survivors:
        print(f"mutants_diff: no crate's results landed (the last run's, if any: {results.name}.old)")
        return mutants.run_code(codes.values())
    results.mkdir(exist_ok=True)
    (results / "missed_all.txt").write_text(
        "".join(line + "\n" for line in survivors), encoding="utf-8"
    )
    print(f"mutants_diff: results in {results.name}/<crate>/")
    for crate, code in sorted(codes.items()):
        print(f"  {crate}: exit {code}")
    if survivors:
        print(f"missed or timed out ({len(survivors)}), in {results.name}/missed_all.txt:")
        for line in survivors:
            print(f"  {line}")
    return mutants.run_code(codes.values())


def run_crate(
    diff_path: str, tree: str, source: Path, crate: str, count: int,
    busy: Counter | None = None,
) -> int:
    """Runs one crate's mutants of the diff: on the remote whole (`REMOTE_ONLY`),
    split, or here — here too while the remote is busy: a whole-crate run holds
    it for days. `busy["seconds"]` totals the estimates this run has run here
    while another run's half was on the remote, across its crates: past
    `BUSY_LOCAL_LIMIT_SECONDS` a crate is left untested."""
    busy = Counter() if busy is None else busy
    selection = ["--in-diff", diff_path, "-p", crate]
    estimated = mutants.estimate_seconds([crate] * count)
    print(f"mutants_diff: estimated {estimated:.0f} s on this machine")
    host = mutants.remote_host()
    if host is not None and mutants.REMOTE_ONLY:
        return mutants.split(selection, host, tree, source, local=False)
    local_only = crate in mutants.LOCAL_ONLY_CRATES
    if host is not None and estimated >= mutants.SPLIT_THRESHOLD_SECONDS and not local_only:
        state = mutants.remote_run_state(host)
        if state == "none":
            half = mutants.estimate_seconds([crate] * count, share=0.5)
            print(f"mutants_diff: split, about {half:.0f} s a side, plus the transfer and polling")
            return mutants.split(selection, host, tree, source)
        if state is not None:
            if busy["seconds"] + estimated > BUSY_LOCAL_LIMIT_SECONDS:
                raise mutants.Refused(
                    f"{host} is busy (a remote half {state}) and {crate} would bring this "
                    f"run's time here past {BUSY_LOCAL_LIMIT_SECONDS // 60} min: left untested. "
                    "That half's collect waits for this run's lock, so the VPS would sit idle "
                    "as long; rerun the crate once the VPS is free"
                )
            busy["seconds"] += estimated
        print(
            f"mutants_diff: {host} is busy (a remote half "
            f"{state or 'unreachable'}): running here",
            flush=True,
        )
    if host is not None and local_only:
        print(f"mutants_diff: {crate} is local-only (LOCAL_ONLY_CRATES)")
    if refusal := mutants.local_refusal():
        raise mutants.Refused(refusal)
    print("mutants_diff: running here", flush=True)
    if estimated > SHARD_SECONDS:
        return sharded_run(
            diff_path, tree, source, crate, selection, math.ceil(estimated / SHARD_SECONDS)
        )
    print(f"mutants_diff: cargo mutants {' '.join(selection)}", flush=True)
    mutants.set_aside_last_run()
    code = mutants.local_mutants(selection, source).returncode
    mutants.update_cost_cache()
    return mutants.report_killed(code, local_half=True)


def sharded_run(
    diff_path: str, tree: str, source: Path, crate: str, selection: list[str], count: int
) -> int:
    """Runs `crate`'s mutants here as `count` cargo-mutants shards, each into
    `SHARDS/<crate>/shard-<k>` and marked done (its `exit`) as it ends; the
    folder then becomes the root's `mutants.out`. A folder an interrupted run
    left for the same tree and diff is resumed from its first shard not done,
    at its own shard count, since the estimate moves as shards fold in their
    costs; one for another tree or diff is set aside. The first shard runs the
    baseline, and the others skip it with the timeout its test phase sets. A
    shard ending other than 0, 2 (missed) or 3 (timed out) stops the crate
    without its mark, so a rerun tries it again."""
    # Before anything runs, as the whole-crate path does: an older run's
    # `mutants.out` left here would be moved into this crate's results.
    mutants.set_aside_last_run()
    folder = SHARDS / crate
    record = folder / "run.json"
    key = {"tree": tree, "diff": hashlib.sha256(Path(diff_path).read_bytes()).hexdigest()}
    kept = json.loads(record.read_text(encoding="utf-8")) if record.exists() else {}
    if {name: kept.get(name) for name in key} == key:
        count = kept["shards"]
        print(f"mutants_diff: resuming {crate}'s {count} shards", flush=True)
    else:
        mutants.set_aside(folder)
        folder.mkdir(parents=True)
        record.write_text(json.dumps({**key, "shards": count}), encoding="utf-8")
    codes = []
    timeout = 0.0
    for k in range(count):
        shard = folder / f"shard-{k}"
        done = shard / "exit"
        if not done.exists():
            args = [
                *selection, "--shard", f"{k}/{count}", "--sharding", "round-robin",
                "--output", str(shard),
            ]
            if k:
                args += ["--baseline", "skip", "--timeout", f"{timeout:.0f}"]
            print(f"mutants_diff: shard {k + 1} of {count}: cargo mutants {' '.join(args)}",
                  flush=True)
            shard.mkdir(exist_ok=True)
            code = mutants.local_mutants(args, source).returncode
            if code not in (0, 2, 3):
                print(f"mutants_diff: shard {k + 1} exited {code}: {crate} stops here, "
                      f"its shards kept in {folder}")
                return code
            mutants.update_cost_cache(shard / "mutants.out")
            done.write_text(str(code), encoding="utf-8")
        codes.append(int(done.read_text(encoding="utf-8")))
        if not k:
            outcomes = json.loads(
                (shard / "mutants.out" / "outcomes.json").read_text(encoding="utf-8")
            )["outcomes"]
            baseline = next(o for o in outcomes if o["scenario"] == "Baseline")
            tests = next(p["duration"] for p in baseline["phase_results"] if p["phase"] == "Test")
            timeout = extra_mutants.timeout_seconds(extra_mutants.mutants_toml(source), tests)
    folder.rename(mutants.ROOT / "mutants.out")
    return mutants.report_killed(mutants.run_code(codes), local_half=True)


def missed_lines(out: Path) -> list[str]:
    """The survivors of one crate's finished run in `out` (its `mutants.out`): a split run's
    summary list, which merges both halves, their timeouts included, and drops what the
    Windows recheck caught; a sharded run's shards' lists, in shard order; else a local
    run's own list and its timeouts (`mutants.timeout_lines`)."""
    if shards := mutants.shard_outputs(out):
        return [line for shard in shards for line in missed_lines(shard)]
    summary = out / "missed_all.txt"
    if summary.exists():
        return mutants.read_lines(summary)
    return mutants.read_lines(out / "missed.txt") + mutants.timeout_lines(out)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
