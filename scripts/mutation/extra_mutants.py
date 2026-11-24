"""`just extra-mutants <crate>`: run the mutants `cargo-mutants` lacks (worklog 5.0b).

`crates/dev/extra_mutants` lists the kinds cargo-mutants never makes: `guard` (an `if`
condition replaced with `false`), `literal` (an element removed from a literal list),
`normalize` (a `.trim()`-style call removed), `order` (a `sort()`-style statement
deleted), `call` (any other method-call statement deleted)
and `bool` (a `true`/`false` flipped). This script applies each listed patch
in one of `mutants.JOBS` copies of the working tree — that many mutants run
at once, a copy each, as cargo-mutants' `--jobs` run them — builds it
(`cargo nextest run --no-run`) and, when it built, lets `cargo nextest run
-p <crate>` decide whether the suite catches it (`build_outcome`,
`test_outcome`). An `[[exclude]]`
entry of `.cargo/extra_mutants.toml` (path, kind and whole description, never a
line number) drops the mutants it matches before they run: the file is the extra
kinds' list of what the runs no longer measure, each entry's `why` naming the
equivalence. `exclude_globs` in the copy's `.cargo/mutants.toml` drops them too,
the way cargo-mutants drops the files its entries match (`glob_matchers`).
An `[[exclude]]` entry that matches no listed mutant of its crate and kinds is
stale: printed, and written into the missed files as a `stale exclusion` line,
so `just mutants`' merged `missed_all.txt` shows it for triage. Outcomes land in
`extra_mutants.out/` (gitignored): `outcomes_<kinds>.tsv` has a row per mutant run (an
excluded one is not), written as each is decided, so a run stopped midway keeps them;
`missed_<kinds>.txt` the survivors as `crates/...:<line>: [<kind>] <what>` lines, and the
timeouts and untested mutants the same way, tagged `[timeout]` or `[untested]` after the
location. An untested mutant (`build_outcome`, `test_outcome`) fails the run.
`missed_<crate>.txt` holds the same lines for `just mutants`, which runs the converge
kinds (`mutants.EXTRA_KINDS`) itself and merges them into `missed_all.txt`. `--list`
only prints the mutants and their counts, without running anything. `--kind <name>`
(repeatable) restricts the run or the listing to those kinds.
"""

import concurrent.futures
import glob
import json
import os
import queue
import re
import signal
import subprocess
import sys
import threading
import time
import tomllib
from collections import Counter
from pathlib import Path

import mutants
import select_tests

ROOT = Path(__file__).resolve().parents[2]
OUT_DIR = ROOT / "extra_mutants.out"
LISTER = ["cargo", "run", "-q", "--release", "-p", "extra_mutants", "--", "list"]
# Every kind the lister can emit; `--kind` restricts a run or listing to a subset.
ALL_KINDS = ("bool", "call", "guard", "literal", "normalize", "order")


def build_outcome(code: int, out: str) -> str | None:
    """A mutant's outcome from its build phase, or None when it built and its
    tests decide: `unviable` or `untested` as `mutants.unviable_build` rules
    the failure — a build killed or starved for reasons outside the mutant, or
    failed another way, is untested, since unviable would say a compiling
    mutant does not compile."""
    if code == 0:
        return None
    return "unviable" if mutants.unviable_build(code, out) else "untested"


def test_outcome(code: int | None, out: str) -> str:
    """A mutant's outcome from its test phase's `cargo nextest run` exit code
    (None: timed out) and output: `timeout`, `missed`, `caught`, or `untested`.
    Nextest's code for a failed test run (`mutants.TEST_RUN_FAILED`), the one
    `mutants.caught_by_tests` reads for cargo-mutants' runs, is a catch even
    when a test binary was killed while its tests ran (SIGKILL, out of
    memory): the mutant made it blow up. A binary that never started
    (`mutants.LOST_CONSOLE`) ran nothing, so the mutant is untested."""
    if code is None:
        return "timeout"
    if code == 0:
        return "missed"
    if code == mutants.TEST_RUN_FAILED:
        return "untested" if mutants.LOST_CONSOLE in out.lower() else "caught"
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


def glob_matchers(globs: list[str]) -> list[re.Pattern]:
    """`exclude_globs` entries compiled to match a path from the tree's root,
    by cargo-mutants' rules (`cargo mutants --help`): an entry holding a `/`
    matches the whole path; one without matches a file or folder of that name
    anywhere in the tree, contents included. `*` stays inside one path
    component and `**` crosses them. An entry with a `{a,b}` alternation,
    which cargo-mutants accepts and `glob.translate` does not, stops the run:
    spell its alternatives out as separate entries."""
    matchers = []
    for entry in globs:
        if "{" in entry:
            raise SystemExit(
                f"extra_mutants: exclude_globs entry {entry!r} uses {{a,b}}, which the extra "
                "kinds do not read: list each alternative as its own entry"
            )
        patterns = [entry] if "/" in entry else [f"**/{entry}", f"**/{entry}/**"]
        for pattern in patterns:
            matchers.append(
                re.compile(
                    glob.translate(pattern, recursive=True, include_hidden=True, seps="/")
                )
            )
    return matchers


def path_excluded(path: str, matchers: list[re.Pattern]) -> bool:
    """Whether `path`, a `/`-separated path from the tree's root, is covered by
    a compiled `exclude_globs` entry (`glob_matchers`)."""
    return any(matcher.fullmatch(path) for matcher in matchers)


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


def mutants_toml(cwd: Path) -> dict:
    """The run's `.cargo/mutants.toml` under `cwd` (its copy of the tree), parsed."""
    return tomllib.loads(
        (cwd / ".cargo" / "mutants.toml").read_text(encoding="utf-8")
    )


def _cargo(
    command: list[str], cwd: Path, timeout: float | None
) -> tuple[int | None, str]:
    """Start `command` in `cwd`, output merged and the temporary files in
    `LOCAL_TMPDIR`, and return (exit code, output); the code is None when
    `timeout` killed the process tree."""
    if os.name == "nt":
        kwargs = mutants.below_normal()
    else:
        # No `below_normal()` here: its `preexec_fn` can deadlock a child forked
        # from a worker thread of the pool, and the remote job already runs the
        # pass under `nice`, which the children inherit. A session of their own,
        # so kill_tree's group kill hits the cargo tree only, never this script's.
        kwargs = {"start_new_session": True}
    proc = subprocess.Popen(
        command,
        cwd=cwd,
        env=mutants.tmpdir_env(),
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


def test_threads() -> int:
    """One job's `--test-threads`: the logical CPUs split `mutants.JOBS` ways,
    never under 1 — the share `sized_config` gives each cargo-mutants job."""
    return max(1, (os.cpu_count() or 1) // mutants.JOBS)


def build_jobs() -> int:
    """One job's `--build-jobs`: the run's CPUs split `mutants.JOBS` ways, never
    under 1 — the machine's count, or `mutants.REMOTE_JOBSERVER_TASKS` when the
    build cap is set, the same budget the remote's cargo-mutants half runs on.
    There is no shared jobserver between the jobs' cargo processes, so each
    gets its own share."""
    return max(
        1, (mutants.REMOTE_JOBSERVER_TASKS or os.cpu_count() or 1) // mutants.JOBS
    )


def build(crate: str, cwd: Path) -> tuple[int, str]:
    """`mutants.test_command`'s `cargo nextest run` of `crate` in `cwd` plus
    `--build-jobs` (this job's share of the run's CPUs, `build_jobs`) and
    `--no-run` (the test binaries built, nothing run): a mutant's build phase,
    split from its tests as cargo-mutants splits them. No timeout —
    cargo-mutants sets none on builds either."""
    command = mutants.test_command(crate, mutants_toml(cwd), test_threads())
    code, out = _cargo(
        [*command, f"--build-jobs={build_jobs()}", "--no-run"], cwd, timeout=None
    )
    assert code is not None  # only a timeout yields None, and a build sets none
    return code, out


def run_tests(
    crate: str, cwd: Path, timeout: float | None, expression: str | None = None
) -> tuple[int | None, str]:
    """`mutants.test_command`'s `cargo nextest run` of `crate` in `cwd` (the run's copy
    of the tree, whose `.cargo/mutants.toml` it reads): a mutant's test phase,
    run only after `build` succeeded; `--test-threads` is `test_threads()`'s
    `cpus // JOBS` share of this machine's logical CPUs, the same
    `sized_config` gives each cargo-mutants job. `expression` (a nextest
    filterset, the tests covering the mutant's lines) is added with `-E`; None
    runs the full suite. Returns (exit code, output);
    the code is None when the timeout killed the process tree."""
    command = mutants.test_command(crate, mutants_toml(cwd), test_threads())
    if expression is not None:
        command = select_tests.with_filter(command, expression)
    return _cargo(command, cwd, timeout)


def check_timeout_config(config: dict) -> None:
    """Stops the run, naming the key, when `config` (a parsed
    `.cargo/mutants.toml`) lacks one a mutant's timeout needs — there is no
    default."""
    for key in ("timeout_multiplier", "minimum_test_timeout"):
        if key not in config:
            raise SystemExit(f"extra_mutants: .cargo/mutants.toml lacks {key}")


def timeout_seconds(config: dict, baseline: float) -> float:
    """A mutant's test-phase timeout from the run's parsed `.cargo/mutants.toml`:
    `max(minimum_test_timeout, timeout_multiplier x baseline)`, the same rule
    cargo-mutants reads from the file. `check_timeout_config` stops the run on
    a missing key."""
    check_timeout_config(config)
    return max(config["minimum_test_timeout"], config["timeout_multiplier"] * baseline)


def baseline_seconds(crate: str, cwd: Path) -> float:
    """The unmutated test phase's wall time in `cwd`, the timeout's yardstick:
    the build runs first, timed apart, since `timeout_multiplier` is a
    multiple of the tests alone. A failing phase ends the run."""
    code, out = build(crate, cwd)
    if code == 0:
        start = time.monotonic()
        code, out = run_tests(crate, cwd, timeout=None)
    if code != 0:
        print("extra_mutants: the unmutated baseline fails; last lines:")
        print("\n".join(out.splitlines()[-30:]))
        raise SystemExit(1)
    return time.monotonic() - start


def byte_lines(data: bytes, start: int, end: int) -> tuple[int, int]:
    """The 1-based, inclusive lines of `data` holding a mutant's byte range
    `start:end` (end exclusive, as `apply_patch` reads it): the line of the
    first byte and the line of the last."""
    return (
        data.count(b"\n", 0, start) + 1,
        data.count(b"\n", 0, max(start, end - 1)) + 1,
    )


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


def missed_line(crate_rel: Path, row: list[str], decided: str = "missed") -> str:
    """`row`'s line in `run_crate`'s missed files, `path:line: [kind]
    description` — cargo-mutants' missed.txt shape plus the kind's tag; a
    `decided` other than missed is also tagged after the location
    (`path:line: [timeout] [kind] ...`), as `mutants.timeout_lines` tags
    cargo-mutants'."""
    tag = "" if decided == "missed" else f"[{decided}] "
    return f"{(crate_rel / row[0]).as_posix()}:{row[4]}: {tag}[{row[3]}] {row[6]}"


def run_crate(
    crate: str, kinds: list[str], tree: str | None = None,
    only: set[str] | None = None, selection: Path | None = None,
) -> tuple[list[str], Counter, float]:
    """Run every `kinds` mutant the lister finds in `crate`, `mutants.JOBS` of
    them at once as cargo-mutants' jobs run: git `tree` (the working tree when
    None) is extracted that many times, and each mutant is applied in one of
    the copies, built (`build`) and, when it built, decided by `cargo nextest
    run -p <crate>` (`run_tests`), the baseline — always the full suite —
    measured in the first copy before any mutant. An outcome is appended to
    `outcomes_<kinds>.tsv` and
    printed as it is decided — the main thread does both — while the missed
    files keep the listing's order whatever order the mutants finish in. With
    `only`, only the mutants whose missed line it
    holds run (exclusions still apply first). `selection` is the coverage
    map's file (`coverage_map.py --extra` writes the extra kinds' entries too):
    each mutant's test phase runs the filter of the entries covering its
    lines, and every gap — a missing file (one line printed), a line no entry
    covers, a `null` filter, a filter too long — falls back to the full suite,
    so a gap costs time and never hides a mutant; a file that cannot be read
    is a wiring fault and stops the run. Writes `outcomes_<kinds>.tsv`
    and `missed_<kinds>.txt` under `extra_mutants.out/`, plus
    `missed_<crate>.txt`, the same survivors named for `just mutants`, which
    calls this for its EXTRA_KINDS and records the run in `run.json` beside
    it for `just mutants-collect` to summarize. Returns (the missed lines,
    the per-outcome counts, the wall seconds)."""
    entries: list[dict] | None = None
    if selection is not None:
        if not selection.exists():
            print(
                f"extra_mutants: no selection file at {selection} (the "
                "coverage map failed): every mutant runs the full suite",
                flush=True,
            )
        else:
            try:
                entries = json.loads(
                    selection.read_text(encoding="utf-8")
                )["mutants"]
            except (OSError, ValueError, KeyError, TypeError) as error:
                raise SystemExit(
                    f"extra_mutants: the selection file at {selection} cannot "
                    f"be read ({error})"
                )
    tree = tree or mutants.working_tree()
    copies = [
        mutants.LOCAL_TMPDIR / f"extra-mutants-{os.getpid()}-{index}"
        for index in range(mutants.JOBS)
    ]
    for copy_dir in copies:
        if copy_dir.exists():
            # The copy of an interrupted run whose pid this one reuses: removed rather than
            # tested against. Other interrupted runs' copies stay until
            # `scripts/mutation/rm_mutant_copies.py` removes them.
            mutants.remove_tree(copy_dir)
    wall_start = time.monotonic()
    try:
        for copy_dir in copies:
            mutants.extract_tree(tree, copy_dir)
        # The crate's folder as the snapshot holds it, not the live tree: a queued run
        # reaches its turn long after its snapshot.
        crate_rel = crate_dir(crate, copies[0]).relative_to(copies[0].resolve())
        config = mutants_toml(copies[0])
        rows = [row for row in list_mutants(copies[0] / crate_rel) if row[3] in kinds]
        excluded, stale = apply_exclusions(
            rows, crate_rel, exclusions(copies[0]), kinds
        )
        # The config's `exclude_globs` drop listed mutants the way cargo-mutants
        # drops the files they match (`glob_matchers` holds its rules).
        matchers = glob_matchers(config.get("exclude_globs", []))
        excluded = [
            drop or path_excluded((crate_rel / row[0]).as_posix(), matchers)
            for row, drop in zip(rows, excluded)
        ]
        rows = [row for row, drop in zip(rows, excluded) if not drop]
        if only is not None:
            rows = [row for row in rows if missed_line(crate_rel, row) in only]
        # Reported before the baseline's minutes: a stale entry means the code
        # changed under a ruling, worth knowing before the run spends its time.
        for line in stale:
            print(line, flush=True)
        check_timeout_config(config)
        baseline = baseline_seconds(crate, copies[0])
        timeout = timeout_seconds(config, baseline)
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
        free = queue.Queue()
        for copy_dir in copies:
            free.put(copy_dir)
        stop = threading.Event()

        def decide(row: list[str]) -> tuple[str, bool | None] | None:
            """One mutant's outcome and whether its test phase ran on its
            filter (None for both when `stop` ended the run before it started,
            for the flag alone when its build decided it — no test phase ran):
            patched, built and tested in the copy this job holds
            alone, the file restored either way."""
            copy_dir = free.get()
            try:
                if stop.is_set():
                    return None
                rel, start, end, kind, line, replacement, description = row
                # A mutant path must stay inside the copy: an absolute or
                # escaping one would patch the real working tree.
                rel_path = Path(rel)
                if rel_path.is_absolute() or ".." in rel_path.parts:
                    raise RuntimeError(
                        f"extra_mutants: mutant path escapes the copy: {rel}"
                    )
                removed = description.rsplit(" | removed: ", 1)[-1]
                target = copy_dir / crate_rel / rel_path
                original = target.read_bytes()
                # The lines are the unpatched file's: the map's entries name
                # the mutants by where they sit before any patch is applied.
                expression = None
                if entries is not None:
                    first, last = byte_lines(original, int(start), int(end))
                    expression, _ = select_tests.filter_for(
                        entries, (crate_rel / rel_path).as_posix(), first, last
                    )
                try:
                    apply_patch(
                        target, int(start), int(end), replacement, removed
                    )
                    decided = build_outcome(*build(crate, copy_dir))
                    ran_selected = None
                    if decided is None:
                        ran_selected = expression is not None
                        decided = test_outcome(
                            *run_tests(crate, copy_dir, timeout, expression)
                        )
                finally:
                    target.write_bytes(original)
                return decided, ran_selected
            finally:
                free.put(copy_dir)

        # Indexed by the row's place in the listing: the missed files list in
        # that order whatever order the jobs finish in.
        outcomes: list[tuple[str, list[str]] | None] = [None] * len(rows)
        failed: BaseException | None = None
        selected_count = 0
        full_count = 0
        with concurrent.futures.ThreadPoolExecutor(max_workers=mutants.JOBS) as pool:
            futures = {
                pool.submit(decide, row): index for index, row in enumerate(rows)
            }
            decided_count = 0
            for future in concurrent.futures.as_completed(futures):
                index = futures[future]
                try:
                    decided = future.result()
                except BaseException as error:
                    # One job's failure stops the run: no mutant not yet
                    # started starts, the ones running finish.
                    stop.set()
                    if failed is None:
                        failed = error
                    continue
                if decided is None:
                    continue
                outcome, ran_selected = decided
                if ran_selected is True:
                    selected_count += 1
                elif ran_selected is False:
                    full_count += 1
                decided_count += 1
                outcomes[index] = (outcome, rows[index])
                with open(tsv, "a", encoding="utf-8", newline="\n") as rows_file:
                    rows_file.write(outcome + "\t" + "\t".join(rows[index]) + "\n")
                print(
                    f"[{decided_count}/{len(rows)}] {outcome}: "
                    f"{rows[index][0]}:{rows[index][4]} {rows[index][6]}",
                    flush=True,
                )
        if failed is not None:
            raise failed
        decided_outcomes = [outcome for outcome in outcomes if outcome is not None]
        missed = [
            missed_line(crate_rel, row, decided)
            for decided, row in decided_outcomes
            if decided in ("missed", "timeout", "untested")
        ]
        missed_text = "".join(text + "\n" for text in missed + stale)
        write_out(OUT_DIR / f"missed_{kinds_key}.txt", missed_text)
        write_out(OUT_DIR / f"missed_{crate}.txt", missed_text)
        per_outcome = Counter(decided for decided, _ in decided_outcomes)
        per_kind = Counter(row[3] for _, row in decided_outcomes)
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
        if selection is not None:
            print(
                f"extra_mutants: {selected_count} mutants ran selected tests, "
                f"{full_count} the full suite",
                flush=True,
            )
        if per_outcome["untested"]:
            print(
                f"{per_outcome['untested']} mutants were NOT tested: a build or a test binary "
                "was killed or lost its console, or cargo failed before any test ran. Rerun "
                "them with less running here (the [untested] lines above)."
            )
        return missed + stale, per_outcome, wall
    finally:
        for copy_dir in copies:
            if copy_dir.exists():
                try:
                    mutants.remove_tree(copy_dir)
                except OSError as error:
                    print(f"extra_mutants: could not remove {copy_dir}: {error}")


def main(argv: list[str]) -> int:
    # A description quotes the source, whose characters a Windows console's code page lacks
    # (a `▸` in a UI string stopped a listing).
    sys.stdout.reconfigure(encoding="utf-8")
    crate, kinds, list_only = parse_args(argv)
    if list_only:
        directory = crate_dir(crate, ROOT)
        crate_rel = directory.relative_to(ROOT)
        rows = [row for row in list_mutants(directory) if row[3] in kinds]
        excluded, stale = apply_exclusions(rows, crate_rel, exclusions(ROOT), kinds)
        matchers = glob_matchers(
            mutants_toml(ROOT).get("exclude_globs", [])
        )
        excluded = [
            drop or path_excluded((crate_rel / row[0]).as_posix(), matchers)
            for row, drop in zip(rows, excluded)
        ]
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
        # An uncollected remote half with a local half's collection merges this
        # crate's `missed_<crate>.txt`.
        if refusal := mutants.local_refusal():
            raise SystemExit(f"extra_mutants: {refusal}")
        _, counts, _ = run_crate(crate, kinds, tree)
    return 1 if counts["untested"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
