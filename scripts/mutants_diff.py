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
crate's survivors.

A crate run whose mutants would cost more than a few minutes locally splits with
`STUDIO_MUTANTS_REMOTE` like `just mutants` does (see `mutants.py`): the
estimate is the baseline plus the per-crate per-mutant costs this machine has
measured (`mutants.estimate_seconds`), so a pes_savefile diff splits while a
one-line `fpc` fix stays local. A crate in `mutants.LOCAL_ONLY_CRATES` never
splits. With `mutants.REMOTE_ONLY` set every mutant runs on the remote,
whatever the estimate.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

import mutants

ROOT = Path(__file__).resolve().parent.parent
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
        return 0
    with tempfile.NamedTemporaryFile("wb", suffix=".diff", delete=False) as f:
        f.write(diff)
        diff_path = f.name
    try:
        return run(diff_path, tree)
    finally:
        os.unlink(diff_path)


def run(diff_path: str, tree: str) -> int:
    """Lists the diff's mutants and runs each crate's (`run_crate`), keeping each crate's
    results under `RESULTS`. `tree` is the working tree the diff was taken against, which the
    remote is sent."""
    # The mutant list, for the printed line and the local-run estimate. Each
    # entry's `file` names the crate (`crates/<group>/<crate>/...`).
    listing = subprocess.run(
        ["cargo", "mutants", "--in-diff", diff_path, "--list", "--json"],
        cwd=ROOT,
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
    if not found:
        return 0
    results = RESULTS
    if results.exists():
        shutil.rmtree(results)
    results.mkdir()
    codes = {}
    for crate, count in sorted(counts.items()):
        print(f"mutants_diff: {crate}, {count} mutants", flush=True)
        codes[crate] = run_crate(diff_path, tree, crate, count)
        out = ROOT / "mutants.out"
        if out.exists():
            out.rename(results / crate)
    survivors = [line for crate in sorted(codes) for line in missed_lines(results / crate)]
    (results / "missed_all.txt").write_text(
        "".join(line + "\n" for line in survivors), encoding="utf-8"
    )
    print(f"mutants_diff: results in {results.name}/<crate>/")
    for crate, code in sorted(codes.items()):
        print(f"  {crate}: exit {code}")
    if survivors:
        print(f"missed ({len(survivors)}), in {results.name}/missed_all.txt:")
        for line in survivors:
            print(f"  {line}")
    return max(codes.values())


def run_crate(diff_path: str, tree: str, crate: str, count: int) -> int:
    """Runs one crate's mutants of the diff: on the remote whole, split, or here."""
    selection = ["--in-diff", diff_path, "-p", crate]
    estimated = mutants.estimate_seconds([crate] * count)
    print(f"mutants_diff: estimated {estimated:.0f} s on this machine")
    host = mutants.remote_host()
    if host is not None and mutants.REMOTE_ONLY:
        return mutants.split(selection, host, local=False, tree=tree)
    local_only = crate in mutants.LOCAL_ONLY_CRATES
    if host is not None and estimated >= mutants.SPLIT_THRESHOLD_SECONDS and not local_only:
        half = mutants.estimate_seconds([crate] * count, share=0.5)
        print(f"mutants_diff: split, about {half:.0f} s a half, plus the transfer and polling")
        return mutants.split(selection, host, tree=tree)
    if host is not None and local_only:
        print(f"mutants_diff: {crate} is local-only (LOCAL_ONLY_CRATES)")
    if host is not None and (refusal := mutants.remote_refusal(host, remote=False)):
        print(f"mutants_diff: {refusal}")
        return 1
    print("mutants_diff: running here", flush=True)
    print(f"mutants_diff: cargo mutants {' '.join(selection)}", flush=True)
    mutants.set_aside_last_run()
    code = mutants.local_mutants(selection).returncode
    mutants.update_cost_cache()
    return mutants.report_killed(code)


def missed_lines(out: Path) -> list[str]:
    """The survivors of one crate's finished run in `out` (its `mutants.out`): a split run's
    summary list, which merges both halves and drops what the Windows recheck caught, else a
    local run's own list."""
    for name in ("missed_all.txt", "missed.txt"):
        path = out / name
        if path.exists():
            return path.read_text(encoding="utf-8").splitlines()
    return []


if __name__ == "__main__":
    sys.exit(main(sys.argv))
