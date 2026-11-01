"""`just extra-mutants <crate>`: run the mutants `cargo-mutants` lacks (worklog 5.0b).

`crates/dev/extra_mutants` lists the kinds cargo-mutants never makes: `guard` (an `if`
condition replaced with `false`), `literal` (an element removed from a literal list),
`normalize` (a `.trim()`-style call removed), `order` (a `sort()`-style statement
deleted) and `bool` (a `true`/`false` flipped). This script applies each listed patch
in a copy of the working tree and lets
`cargo test -p <crate>` decide whether the suite catches it. An `[[exclude]]`
entry of `.cargo/extra_mutants.toml` (path, kind and whole description, never a
line number) drops the mutants it matches before they run: the file is the extra
kinds' list of what the runs no longer measure, each entry's `why` naming the
equivalence. An entry that matches no listed mutant of its crate and kinds is
stale: printed, and written into the missed files as a `stale exclusion` line,
so `just mutants`' merged `missed_all.txt` shows it for triage. Outcomes land in
`extra_mutants.out/` (gitignored): `outcomes_<kinds>.tsv` has a row per mutant run (an
excluded one is not), written as each is decided, so a run stopped midway keeps them;
`missed_<kinds>.txt` the survivors as `crates/...:<line>: [<kind>] <what>` lines, and the
timeouts and untested mutants the same way, tagged `[timeout]` or `[untested]` after the
location. An untested mutant (`outcome`) fails the run.
`missed_<crate>.txt` holds the same lines for `just mutants`, which runs the converge
kinds (`mutants.EXTRA_KINDS`) itself and merges them into `missed_all.txt`. `--list`
only prints the mutants and their counts, without running anything. `--kind <name>`
(repeatable) restricts the run or the listing to those kinds.
"""

import json
import os
import signal
import subprocess
import sys
import time
import tomllib
from collections import Counter
from pathlib import Path

import mutants

ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = ROOT / "extra_mutants.out"
LISTER = ["cargo", "run", "-q", "--release", "-p", "extra_mutants", "--", "list"]
# Every kind the lister can emit; `--kind` restricts a run or listing to a subset.
ALL_KINDS = ("bool", "guard", "literal", "normalize", "order")
# Each mutant's `cargo test` may take up to this multiple of the unmutated baseline.
TIMEOUT_FACTOR = 3
MIN_TIMEOUT_SECONDS = 120
# What `cargo test` prints, lowercase, when a test ran and failed: the only proof a mutant was
# caught. A failure without it (a build script that could not run, a full disk) tested nothing.
TEST_FAILED_SIGNS = ("test result: failed", "error: test failed", "error: doctest failed")


def outcome(code: int | None, out: str) -> str:
    """A mutant's outcome from its `cargo test` exit code (None: timed out) and output:
    `timeout`, `missed`, `unviable` (it does not compile), `caught` (a test failed), or
    `untested`. A build killed or starved for reasons outside the mutant
    (`mutants.KILLED_BUILD_SIGNS`, in a build or a test binary) or a failure with no failing
    test is untested: filed as caught, it hid a mutant nothing tested (4.y-conv lost 273
    builds' consoles at once), and filed as unviable, a mutant that does compile."""
    if code is None:
        return "timeout"
    if code == 0:
        return "missed"
    lowered = out.lower()
    if any(sign in lowered for sign in mutants.KILLED_BUILD_SIGNS):
        return "untested"
    if "could not compile" in lowered:
        return "unviable"
    if any(sign in lowered for sign in TEST_FAILED_SIGNS):
        return "caught"
    return "untested"


def crate_dir(name: str, root: Path) -> Path:
    """The named workspace crate's directory in the tree at `root`, from `cargo metadata`."""
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout
    for package in json.loads(out)["packages"]:
        if package["name"] == name:
            return Path(package["manifest_path"]).resolve().parent
    raise SystemExit(f"extra_mutants: no workspace crate named {name}")


def list_mutants(directory: Path) -> list[list[str]]:
    """The lister's rows (`path`, `start`, `end`, `kind`, `line`, `replacement`,
    `description`), run on the crate's sources under `directory`."""
    out = subprocess.run(
        [*LISTER, str(directory)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if out.returncode != 0:
        raise SystemExit(f"extra_mutants: the lister failed:\n{out.stderr}")
    rows = [line.split("\t") for line in out.stdout.splitlines() if line]
    for row in rows:
        if len(row) != 7:
            raise SystemExit(f"extra_mutants: malformed lister row: {row}")
    return rows


def exclusions(tree_root: Path) -> list[dict]:
    """The `[[exclude]]` entries of `.cargo/extra_mutants.toml` under `tree_root`
    (the run's copy of the tree, so what is excluded is the file the tree under
    test carries). Every entry is exactly `path`, `kind`, `description` and
    `why`, each a string; a malformed file or entry stops the run, an entry
    named by its index."""
    path = tree_root / ".cargo" / "extra_mutants.toml"
    if not path.exists():
        return []
    try:
        entries = tomllib.loads(path.read_text(encoding="utf-8")).get("exclude", [])
    except tomllib.TOMLDecodeError as error:
        raise SystemExit(f"extra_mutants: {path}: not TOML: {error}")
    for index, entry in enumerate(entries, 1):
        if (
            not isinstance(entry, dict)
            or set(entry) != {"path", "kind", "description", "why"}
            or not all(isinstance(value, str) for value in entry.values())
        ):
            raise SystemExit(
                f"extra_mutants: {path}: entry {index} is not exactly "
                "path, kind, description and why, each a string"
            )
        # A misspelt kind matches no mutant, and its staleness is never reported either.
        if entry["kind"] not in ALL_KINDS:
            raise SystemExit(
                f"extra_mutants: {path}: entry {index} has kind {entry['kind']!r}, "
                f"none of {', '.join(ALL_KINDS)}"
            )
    return entries


def apply_exclusions(
    rows: list[list[str]], crate_rel: Path, entries: list[dict], kinds: list[str]
) -> tuple[list[bool], list[str]]:
    """Per `rows` row, whether an entry claims it (path, kind and whole
    description equal, exact text; an entry drops every mutant it matches), and
    the stale-entry lines: an entry of this crate whose kind the run includes
    but which matched no listed mutant, shaped as a missed line for triage."""
    matched = [False] * len(entries)
    excluded = []
    for row in rows:
        hit = False
        for index, entry in enumerate(entries):
            if (
                (crate_rel / row[0]).as_posix() == entry["path"]
                and row[3] == entry["kind"]
                and row[6] == entry["description"]
            ):
                matched[index] = True
                hit = True
        excluded.append(hit)
    prefix = crate_rel.as_posix() + "/"
    stale = [
        f"{entry['path']}: [{entry['kind']}] stale exclusion: {entry['description']}"
        for entry, was_matched in zip(entries, matched)
        if not was_matched
        and entry["path"].startswith(prefix)
        and entry["kind"] in kinds
    ]
    return excluded, stale


def kill_tree(proc: subprocess.Popen) -> None:
    """Kill `proc` and every process it started (cargo's rustc and test children)."""
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/F", "/T", "/PID", str(proc.pid)], capture_output=True
        )
    else:
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except ProcessLookupError:
            pass
    proc.wait()


def cargo_test(crate: str, cwd: Path, timeout: float | None) -> tuple[int | None, str]:
    """`cargo test -p <crate>` in `cwd` at below-normal priority, output merged.
    Returns (exit code, output); exit code is None when the timeout killed the
    process tree."""
    kwargs = mutants.below_normal()
    if os.name != "nt":
        kwargs["start_new_session"] = True
    proc = subprocess.Popen(
        ["cargo", "test", "-p", crate],
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        **kwargs,
    )
    try:
        out, _ = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        kill_tree(proc)
        return None, ""
    return proc.returncode, out


def baseline_seconds(crate: str, cwd: Path) -> float:
    """The unmutated `cargo test` wall time in `cwd`; a failing baseline ends the run."""
    start = time.monotonic()
    code, out = cargo_test(crate, cwd, timeout=None)
    if code != 0:
        print("extra_mutants: the unmutated baseline fails; last lines:")
        print("\n".join(out.splitlines()[-30:]))
        raise SystemExit(1)
    return time.monotonic() - start


def apply_patch(path: Path, start: int, end: int, replacement: str, removed: str) -> None:
    """Replace `data[start:end]` with `replacement`, checking the range holds the text
    the lister described (so a drifted file cannot silently patch somewhere else)."""
    data = path.read_bytes()
    if not (0 <= start <= end <= len(data)):
        raise AssertionError(f"{path}: mutant byte range {start}..{end} out of bounds")
    actual = " ".join(data[start:end].decode("utf-8").split())
    if actual != removed:
        raise AssertionError(
            f"{path}: the byte range holds {actual!r}, the lister described {removed!r}"
        )
    patched = data[:start] + replacement.encode("utf-8") + data[end:]
    if patched == data:
        raise AssertionError(f"{path}: mutant at {start}..{end} changes nothing")
    path.write_bytes(patched)


def write_out(path: Path, text: str) -> None:
    temp = path.with_name(path.name + ".tmp")
    temp.write_text(text, encoding="utf-8", newline="\n")
    os.replace(temp, path)


def parse_args(argv: list[str]) -> tuple[str, list[str], bool]:
    """(crate, kinds, list_only)."""
    args = argv[1:]
    list_only = "--list" in args
    args = [arg for arg in args if arg != "--list"]
    kinds = []
    positional = []
    i = 0
    while i < len(args):
        if args[i] == "--kind":
            i += 1
            if i == len(args):
                raise SystemExit("extra_mutants: --kind needs a name")
            kinds.append(args[i])
        else:
            positional.append(args[i])
        i += 1
    unknown = [kind for kind in kinds if kind not in ALL_KINDS]
    if len(positional) != 1 or unknown:
        raise SystemExit("usage: extra_mutants.py <crate> [--list] [--kind <name>]...")
    return positional[0], sorted(set(kinds)) or list(ALL_KINDS), list_only


def run_crate(
    crate: str, kinds: list[str], tree: str | None = None
) -> tuple[list[str], Counter, float]:
    """Run every `kinds` mutant the lister finds in `crate`, each applied in a
    copy of git `tree` (the working tree when None) and decided by one `cargo
    test -p <crate>`. Writes `outcomes_<kinds>.tsv` and `missed_<kinds>.txt`
    under `extra_mutants.out/`, plus `missed_<crate>.txt`, the same survivors
    named for `just mutants`, which calls this for its EXTRA_KINDS and records
    the run in `run.json` beside it for `just mutants-collect` to summarize.
    Returns (the missed lines, the per-outcome counts, the wall seconds)."""
    tree = tree or mutants.working_tree()
    copy_dir = mutants.LOCAL_TMPDIR / f"extra-mutants-{os.getpid()}"
    if copy_dir.exists():
        # The copy of an interrupted run whose pid this one reuses: removed rather than
        # tested against. Other interrupted runs' copies stay until
        # `scripts/mutation/rm_mutant_copies.py` removes them.
        mutants.remove_tree(copy_dir)
    wall_start = time.monotonic()
    try:
        mutants.extract_tree(tree, copy_dir)
        # The crate's folder as the snapshot holds it, not the live tree: a queued run
        # reaches its turn long after its snapshot.
        crate_rel = crate_dir(crate, copy_dir).relative_to(copy_dir.resolve())
        rows = [row for row in list_mutants(copy_dir / crate_rel) if row[3] in kinds]
        excluded, stale = apply_exclusions(
            rows, crate_rel, exclusions(copy_dir), kinds
        )
        rows = [row for row, drop in zip(rows, excluded) if not drop]
        # Reported before the baseline's minutes: a stale entry means the code
        # changed under a ruling, worth knowing before the run spends its time.
        for line in stale:
            print(line, flush=True)
        baseline = baseline_seconds(crate, copy_dir)
        timeout = max(MIN_TIMEOUT_SECONDS, TIMEOUT_FACTOR * baseline)
        print(
            f"extra_mutants: baseline {baseline:.0f} s, timeout {timeout:.0f} s, "
            f"{len(rows)} mutants, {sum(excluded)} excluded",
            flush=True,
        )
        OUT_DIR.mkdir(exist_ok=True)
        kinds_key = "_".join(kinds)
        # A row per mutant as it is decided, so a run stopped hours in keeps what it measured.
        tsv = OUT_DIR / f"outcomes_{kinds_key}.tsv"
        write_out(tsv, "")
        outcomes: list[tuple[str, list[str]]] = []
        for i, row in enumerate(rows, 1):
            rel, start, end, kind, line, replacement, description = row
            # A mutant path must stay inside the copy: an absolute or escaping one would
            # patch the real working tree.
            rel_path = Path(rel)
            if rel_path.is_absolute() or ".." in rel_path.parts:
                raise RuntimeError(f"extra_mutants: mutant path escapes the copy: {rel}")
            removed = description.rsplit(" | removed: ", 1)[-1]
            target = copy_dir / crate_rel / rel_path
            original = target.read_bytes()
            try:
                apply_patch(
                    target, int(start), int(end), replacement, removed
                )
                code, out = cargo_test(crate, copy_dir, timeout)
            finally:
                target.write_bytes(original)
            decided = outcome(code, out)
            outcomes.append((decided, row))
            with open(tsv, "a", encoding="utf-8", newline="\n") as rows_file:
                rows_file.write(decided + "\t" + "\t".join(row) + "\n")
            print(f"[{i}/{len(rows)}] {decided}: {rel}:{line} {description}", flush=True)
        # cargo-mutants' missed.txt shape (workspace-relative path, line, then the
        # description), tagged with the kind cargo-mutants' lines cannot carry; a timeout
        # or an untested mutant also with its outcome after the location, as
        # `mutants.timeout_lines` tags cargo-mutants' timeouts.
        missed = []
        for decided, row in outcomes:
            if decided not in ("missed", "timeout", "untested"):
                continue
            tag = "" if decided == "missed" else f"[{decided}] "
            missed.append(f"{(crate_rel / row[0]).as_posix()}:{row[4]}: {tag}[{row[3]}] {row[6]}")
        missed_text = "".join(text + "\n" for text in missed + stale)
        write_out(OUT_DIR / f"missed_{kinds_key}.txt", missed_text)
        write_out(OUT_DIR / f"missed_{crate}.txt", missed_text)
        per_outcome = Counter(decided for decided, _ in outcomes)
        per_kind = Counter(row[3] for _, row in outcomes)
        print(
            "outcomes: "
            + (", ".join(f"{name} {n}" for name, n in sorted(per_outcome.items())) or "none")
        )
        print(
            "per kind: "
            + (", ".join(f"{name} {n}" for name, n in sorted(per_kind.items())) or "none")
        )
        wall = time.monotonic() - wall_start
        print(f"wall time: {wall:.0f} s")
        if per_outcome["untested"]:
            print(
                f"{per_outcome['untested']} mutants were NOT tested: a build or a test binary "
                "was killed or lost its console, or cargo failed before any test ran. Rerun "
                "them with less running here (the [untested] lines above)."
            )
        return missed + stale, per_outcome, wall
    finally:
        if copy_dir.exists():
            try:
                mutants.remove_tree(copy_dir)
            except OSError as error:
                print(f"extra_mutants: could not remove {copy_dir}: {error}")


def main(argv: list[str]) -> int:
    crate, kinds, list_only = parse_args(argv)
    if list_only:
        directory = crate_dir(crate, ROOT)
        crate_rel = directory.relative_to(ROOT)
        rows = [row for row in list_mutants(directory) if row[3] in kinds]
        excluded, stale = apply_exclusions(rows, crate_rel, exclusions(ROOT), kinds)
        for row, is_excluded in zip(rows, excluded):
            print("\t".join(row) + ("\t(excluded)" if is_excluded else ""))
        per_kind = Counter(row[3] for row in rows)
        counts = ", ".join(f"{kind} {n}" for kind, n in sorted(per_kind.items()))
        print(f"{len(rows)} mutants ({counts}), {sum(excluded)} excluded")
        for line in stale:
            print(line)
        return 0
    tree = mutants.working_tree()
    with mutants.run_lock():
        # An uncollected remote half's collection merges this crate's `missed_<crate>.txt`.
        if refusal := mutants.local_refusal(mutants.remote_host()):
            raise SystemExit(f"extra_mutants: {refusal}")
        _, counts, _ = run_crate(crate, kinds, tree)
    return 1 if counts["untested"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
