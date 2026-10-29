"""`python scripts/symbolize_log.py <utw.log> <symbols folder>`: names the exe's
backtrace frames in a posted `utw.log` against the release's kept symbols, the
symbols archive `just release` leaves beside the bundle extracted into a folder (core
plan, "Diagnostic logging").

`utw.log`'s panic records write each frame as `<index>: 0x<base>+0x<offset>` (the
module's base and the offset into it), because the bundle ships no `.pdb`. Each
record's frames are a run of consecutive such lines starting at `0:`; frame 0 is
always inside the exe (the `backtrace` crate is linked into it), so its base is the
exe's. Every exe frame's line is followed by the function, file and line
`llvm-symbolizer` names for its offset; the other modules' frames stay as they are.
Exits 2 when the folder does not hold exactly one `.exe` and one `.pdb`, or
`llvm-symbolizer` is found neither on PATH nor in the MSVC toolset.
"""

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple

FRAME = re.compile(r"^\s*(\d+): 0x([0-9a-fA-F]+)(?:\+0x([0-9a-fA-F]+))?\s*$")
INDENT = "      "
MSVC_ROOT = Path(r"C:\Program Files\Microsoft Visual Studio")


class Refusal(Exception):
    """A reason the log cannot be symbolized; `main` prints it as `error:` and exits 2."""


class Frame(NamedTuple):
    """One `N: 0x<base>+0x<offset>` line: `line` is its 0-based number in the log and
    `base`/`offset` its parsed hex, both `None` on a bare `N: 0x<ip>` line."""

    line: int
    index: int
    base: int | None
    offset: int | None


def frame_runs(text: str) -> list[list[Frame]]:
    """The log's panic records' frame runs: each run is the consecutive frame lines of
    one record, starting at its `0:` line."""
    runs: list[list[Frame]] = []
    current: list[Frame] | None = None
    for lineno, line in enumerate(text.splitlines()):
        match = FRAME.match(line)
        if match is None:
            current = None
            continue
        index = int(match.group(1))
        if match.group(3) is None:
            frame = Frame(lineno, index, None, None)
        else:
            frame = Frame(lineno, index, int(match.group(2), 16), int(match.group(3), 16))
        if index == 0:
            current = [frame]
            runs.append(current)
        elif current is not None and index == current[-1].index + 1:
            current.append(frame)
        else:
            current = None
    return runs


def exe_frames(run: list[Frame]) -> list[Frame]:
    """The run's frames inside the exe: frame 0 is always one (the `backtrace` crate is
    linked into the exe), so the exe's module base is its base."""
    base = run[0].base
    if base is None:
        return []
    return [frame for frame in run if frame.base == base]


def blocks(output: str) -> list[list[str]]:
    """`llvm-symbolizer --pretty-print`'s output as one block of lines per address it
    was asked: `func at file:line:col` first, a ` (inlined by)` line per caller, and a
    blank line between addresses."""
    if not output.strip():
        return []
    return [block.splitlines() for block in re.split(r"\n\s*\n", output.strip("\n"))]


def resolve_offsets(symbolizer: str, exe: Path, offsets: list[int]) -> list[list[str]]:
    """One symbolizer run over `offsets`, the exe frames' offsets into their module."""
    out = subprocess.run(
        [
            symbolizer,
            f"--obj={exe}",
            "--relative-address",
            "--pretty-print",
            *[hex(offset) for offset in offsets],
        ],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return blocks(out)


def symbolized(text: str, resolve) -> str:
    """`text` with each exe frame's line followed by its names, indented; `resolve`
    maps a run's frame offsets to one block of `func at file:line` lines per offset."""
    lines = text.splitlines()
    insertions: dict[int, list[str]] = {}
    for run in frame_runs(text):
        frames = exe_frames(run)
        if not frames:
            continue
        # A frame's logged address is the call's return address; stepping one byte back
        # names the call's own line, as debuggers do.
        names = resolve([frame.offset - 1 for frame in frames])
        for frame, block in zip(frames, names):
            insertions[frame.line] = block
    out = []
    for lineno, line in enumerate(lines):
        out.append(line)
        out.extend(f"{INDENT}{name}" for name in insertions.get(lineno, ()))
    return "\n".join(out) + "\n"


def symbols_in(folder: Path) -> Path:
    """The symbols folder's one `.exe`; the folder must also hold exactly one `.pdb`
    beside it."""
    exes = list(folder.glob("*.exe"))
    pdbs = list(folder.glob("*.pdb"))
    if len(exes) != 1 or len(pdbs) != 1:
        raise Refusal(f"{folder} does not hold exactly one .exe and one .pdb")
    return exes[0]


def find_symbolizer() -> str | None:
    """`llvm-symbolizer` on PATH, else the newest MSVC toolset's; `None` if neither."""
    on_path = shutil.which("llvm-symbolizer")
    if on_path:
        return on_path
    candidates = list(
        MSVC_ROOT.glob("*/*/VC/Tools/MSVC/*/bin/HostX64/x64/llvm-symbolizer.exe")
    )
    if not candidates:
        return None
    return str(
        max(
            candidates,
            key=lambda path: [
                int(part) for part in path.parts[-5].split(".") if part.isdigit()
            ],
        )
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path, help="a utw.log holding panic records")
    parser.add_argument(
        "symbols", type=Path, help="the extracted symbols archive's folder"
    )
    args = parser.parse_args()
    try:
        exe = symbols_in(args.symbols)
        symbolizer = find_symbolizer()
        if symbolizer is None:
            raise Refusal(
                "llvm-symbolizer not found: it is not on PATH and no "
                f"{MSVC_ROOT}\\<edition>\\<year>\\VC\\Tools\\MSVC\\<version>\\bin\\"
                "HostX64\\x64\\llvm-symbolizer.exe exists"
            )
    except Refusal as refusal:
        print(f"error: {refusal}", file=sys.stderr)
        return 2
    text = args.log.read_text(encoding="utf-8", errors="replace")
    print(
        symbolized(text, lambda offsets: resolve_offsets(symbolizer, exe, offsets)),
        end="",
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
