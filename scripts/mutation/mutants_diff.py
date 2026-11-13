"""`just mutants-diff [base] [path...]`: mutation-test only the lines changed since `base`,
under `crates` or the paths given (a crate's folder).

Runs `cargo mutants --in-diff` over the diff from `base` (default HEAD) to the working tree,
untracked files included, so a review measures the tests of the code under review rather than the whole
workspace (CONTRIBUTING.md "Testing and verification", "Mutation runs"). The diff goes through
a file because the justfile forbids pipes and redirection in recipe lines.

Each crate the diff mutates gets a cargo-mutants run of its own (`-p <crate>`), one after the
other, not one run for the whole diff: cargo-mutants times a mutant's tests out at a multiple of
the baseline, and one run's baseline tests every crate it mutates, so a hung mutant of a crate
whose tests take two seconds waited out the slowest crate's suite (751 s here at `mutants-diff
95f20eac`). A run costs one more baseline per crate, about a minute. Each crate's results move
to `mutants.diff/<crate>/` when its run ends, and `mutants.diff/missed_all.txt` lists every
crate's survivors and, tagged `[timeout]`, its timeouts.

A crate run whose mutants would cost more than a few minutes locally splits with
`STUDIO_MUTANTS_REMOTE` (the split `mutants.py` describes, by files to the two
sides' measured speeds where it can, else in halves): the
estimate is the baseline plus the per-crate per-mutant costs this machine has
measured (`mutants.estimate_seconds`), so a pes_savefile diff splits while a
one-line `fpc` fix stays local. While the remote is busy — a whole-crate run
holds it for days — the crate runs here whole instead. A crate in
`mutants.LOCAL_ONLY_CRATES` never splits. With `mutants.REMOTE_ONLY` set every
mutant runs on the remote, whatever the estimate.
"""

import json
import os
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

import mutants

ROOT = Path(__file__).resolve().parents[2]
# Each crate's `mutants.out` of the last diff run, and their survivors' `missed_all.txt`.
RESULTS = ROOT / "mutants.diff"


def main(argv: list[str]) -> int:
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
    diff = diffed.stdout
    if not diff.strip():
        print(f"mutants_diff: no changes under {' '.join(paths)} since {base}")
        # Nothing to measure, and an older run's results never stand as this one's; moved in
        # this run's turn, since a run in progress writes them.
        with mutants.run_lock():
            mutants.set_aside(RESULTS)
        return 0
    with tempfile.NamedTemporaryFile("wb", suffix=".diff", delete=False) as f:
        f.write(diff)
        diff_path = f.name
    try:
        with mutants.run_lock(), mutants.snapshot_source(tree) as source:
            return run(diff_path, tree, source)
    finally:
        os.unlink(diff_path)


def run(diff_path: str, tree: str, source: Path) -> int:
    """Lists the diff's mutants and runs each crate's (`run_crate`), keeping each crate's
    results under `RESULTS`. `tree` is the working tree the diff was taken against, which the
    remote is sent, and `source` its copy (`mutants.snapshot_source`), which the listing and
    the local halves read."""
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
    if not found:
        return 0
    codes = {}
    for crate, count in sorted(counts.items()):
        print(f"mutants_diff: {crate}, {count} mutants", flush=True)
        try:
            codes[crate] = run_crate(diff_path, tree, source, crate, count)
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
    if not results.exists():
        print(f"mutants_diff: no crate's results landed (the last run's, if any: {results.name}.old)")
        return max(codes.values())
    survivors = [line for crate in sorted(codes) for line in missed_lines(results / crate)]
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
    return max(codes.values())


def run_crate(diff_path: str, tree: str, source: Path, crate: str, count: int) -> int:
    """Runs one crate's mutants of the diff: on the remote whole (`REMOTE_ONLY`),
    split, or here — here too while the remote is busy: a whole-crate run holds
    it for days."""
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
    print(f"mutants_diff: cargo mutants {' '.join(selection)}", flush=True)
    mutants.set_aside_last_run()
    code = mutants.local_mutants(selection, source).returncode
    mutants.update_cost_cache()
    return mutants.report_killed(code, local_half=True)


def missed_lines(out: Path) -> list[str]:
    """The survivors of one crate's finished run in `out` (its `mutants.out`): a split run's
    summary list, which merges both halves, their timeouts included, and drops what the
    Windows recheck caught; else a local run's own list and its timeouts
    (`mutants.timeout_lines`)."""
    summary = out / "missed_all.txt"
    if summary.exists():
        return mutants.read_lines(summary)
    return mutants.read_lines(out / "missed.txt") + mutants.timeout_lines(out)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
