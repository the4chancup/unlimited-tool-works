"""`python scripts/perturb_check.py <list.py> [label...]`: show that each test fails when the
code it guards is perturbed, for code whose tests are Python's own (the scripts).

`<list.py>` defines `ROOT` (the tree to perturb, a path), `COMMAND` (the test command as a
list, run from `ROOT`, the perturbation's test name appended) and `PERTURBATIONS`, tuples of
(label, file relative to `ROOT`, old text, new text, test name). Each perturbation replaces
`old`, which must occur exactly once, runs the command, then puts the file's original bytes
back (asserted) whatever happened. Labels on the command line run those alone, printing the
test's output. Prints RED or GREEN per perturbation; exits 1 when any stayed green.

The tests run with bytecode off: Python reuses a `.pyc` whose source has the same size and
the same whole-second mtime, so a perturbation the size of the one before, written within the
same second, ran the one before's code, and a perturbation looked caught or missed for it."""

import os
import runpy
import subprocess
import sys
from pathlib import Path


def write(path: Path, data: bytes) -> None:
    temp = path.with_name(path.name + ".perturb")
    temp.write_bytes(data)
    os.replace(temp, path)


def main(argv: list[str]) -> int:
    spec = runpy.run_path(argv[1])
    root = Path(spec["ROOT"])
    only = argv[2:]
    perturbations = [p for p in spec["PERTURBATIONS"] if not only or p[0] in only]
    if only and len(perturbations) != len(only):
        raise SystemExit(f"perturb_check: unknown labels among {only}")
    green = 0
    for label, rel, old, new, test in perturbations:
        path = root / rel
        original = path.read_bytes()
        text = original.decode("utf-8")
        if text.count(old) != 1:
            raise SystemExit(f"perturb_check: {label}: the old text occurs {text.count(old)} times")
        write(path, text.replace(old, new).encode("utf-8"))
        try:
            run = subprocess.run(
                [*spec["COMMAND"], test], cwd=root, capture_output=True, text=True,
                env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
            )
        finally:
            write(path, original)
        assert path.read_bytes() == original, f"{rel} was not restored"
        if only:
            print(run.stdout[-3000:], run.stderr[-3000:])
        caught = run.returncode != 0
        green += not caught
        errors = [line for line in run.stderr.splitlines() if "Error" in line]
        print(f"{'RED  ' if caught else 'GREEN'} {label}: {(errors or ['(no error line)'])[-1][:140]}",
              flush=True)
    print("every perturbation red" if not green else f"{green} perturbations stayed green")
    return 1 if green else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
