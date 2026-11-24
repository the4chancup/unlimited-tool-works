#!/usr/bin/env python3
"""The program cargo-mutants runs in cargo's place when a whole-crate run selects tests.

cargo-mutants starts whatever `$CARGO` names for each cargo call of its run — the builds, the
test listing and each mutant's test phase — in its own copy of the tree with the mutant
applied. This script passes every call to the real cargo. In a test phase it first finds the
lines the mutant changed (the copy against the run's source), looks up the tests covering
them in the selection file and adds a nextest filter (`-E`) for them.

Three variables wire it up; a missing one exits 2 without running anything:
- `UTW_REAL_CARGO`: the real cargo's path;
- `UTW_MUTANTS_SOURCE`: the unmutated tree the run copies;
- `UTW_MUTANTS_SELECTION`: a JSON file giving each listed mutant the filter of its tests.

Whenever the answer is not one clear filter — no file changed (the baseline), several changed
files, no listed mutant on the changed lines, a mutant no test covers, a filter too long for
one command line — the real cargo runs with the arguments unchanged, so a gap costs time
and never hides a mutant. A selection file that cannot be read is a wiring fault, like a
missing variable, and exits 2 too. That fails the run's baseline and stops the run at once,
where running the full suite would only make the whole run slow, unnoticed.
"""

import json
import os
import subprocess
import sys
from collections.abc import Mapping
from pathlib import Path

# Windows' command line holds 32,767 characters, and one rule for both machines is simpler.
MAX_FILTER_CHARS = 30_000
REQUIRED_VARS = ("UTW_REAL_CARGO", "UTW_MUTANTS_SOURCE", "UTW_MUTANTS_SELECTION")


def changed_lines(original: bytes, mutated: bytes) -> tuple[int, int] | None:
    """The 1-based, inclusive range of `original`'s lines that `mutated` changed, or `None`
    when the two are equal. A change that only inserts lines maps to the original line the
    insertion sits before (the last line when it sits at the end)."""
    if original == mutated:
        return None
    before = original.splitlines(keepends=True)
    after = mutated.splitlines(keepends=True)
    prefix = 0
    while prefix < len(before) and prefix < len(after) and before[prefix] == after[prefix]:
        prefix += 1
    suffix = 0
    while (
        prefix + suffix < len(before)
        and prefix + suffix < len(after)
        and before[len(before) - 1 - suffix] == after[len(after) - 1 - suffix]
    ):
        suffix += 1
    if len(before) - suffix == prefix:
        line = min(prefix + 1, len(before))
        return line, line
    return prefix + 1, len(before) - suffix


def _read(path: Path) -> bytes | None:
    """A file's bytes, `None` when it cannot be read: a missing file counts as changed."""
    try:
        return path.read_bytes()
    except OSError:
        return None


def test_filter(selection: list[dict], source: Path, copy: Path) -> tuple[str | None, str]:
    """The nextest expression a mutant's test phase adds, and a one-line reason; `None` runs
    the full suite. `selection` is the selection file's `mutants` list, `source` the run's
    unmutated tree and `copy` cargo-mutants' copy of it with the mutant applied."""
    files = list(dict.fromkeys(entry["file"] for entry in selection))
    changed = [file for file in files if _read(source / file) != _read(copy / file)]
    if not changed:
        return None, "no file differs from the source (the baseline)"
    if len(changed) > 1:
        return None, "several files differ from the source: " + ", ".join(changed)
    file = changed[0]
    start, end = changed_lines(_read(source / file) or b"", _read(copy / file) or b"") or (
        1,
        1,
    )
    where = f"{file}:{start}-{end}"
    mutants = [
        entry
        for entry in selection
        if entry["file"] == file and entry["start"] <= end and start <= entry["end"]
    ]
    if not mutants:
        return None, f"no listed mutant on {where}"
    if any(entry["filter"] is None for entry in mutants):
        return None, f"no test covers {where}"
    filters = list(dict.fromkeys(entry["filter"] for entry in mutants))
    expression = " | ".join(filters)
    if len(expression) > MAX_FILTER_CHARS:
        return None, f"the filter is too long ({len(expression)} characters)"
    return expression, f"{len(mutants)} mutants' filters on {where}"


def cargo_command(
    args: list[str], env: Mapping[str, str], copy: Path
) -> tuple[list[str], str | None]:
    """The command to run in place of `args` on the real cargo, and the line to print to
    stderr (`None` prints nothing). Only a test phase — `nextest run` without `--no-run` —
    can gain a filter; every other call passes through unchanged."""
    cargo = env["UTW_REAL_CARGO"]
    if args[:2] != ["nextest", "run"] or "--no-run" in args:
        return [cargo, *args], None
    selection = json.loads(
        Path(env["UTW_MUTANTS_SELECTION"]).read_text(encoding="utf-8")
    )["mutants"]
    expression, reason = test_filter(
        selection, Path(env["UTW_MUTANTS_SOURCE"]), copy
    )
    if expression is not None:
        insert = args.index("--") if "--" in args else len(args)
        args = [*args[:insert], "-E", expression, *args[insert:]]
    return [cargo, *args], f"select_tests: {reason}"


def main(argv: list[str]) -> int:
    for name in REQUIRED_VARS:
        if not os.environ.get(name):
            print(f"select_tests: {name} is not set", file=sys.stderr)
            return 2
    try:
        command, line = cargo_command(argv, os.environ, Path.cwd())
    except (OSError, ValueError, KeyError) as error:
        print(f"select_tests: the selection file cannot be read ({error!r})", file=sys.stderr)
        return 2
    if line is not None:
        print(line, file=sys.stderr)
    if os.name == "posix":
        # exec replaces this process, so a timeout's kill reaches cargo with no
        # process between; Windows only ever starts the script from its tests.
        os.execv(command[0], command)
    return subprocess.run(command).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
