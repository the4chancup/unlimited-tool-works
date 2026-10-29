"""List colours written from literal values outside the theme's palette.

    python scripts/colour_literals.py

Every colour the suite draws comes from egui's `Visuals` or from `studio_core::theme::Palette`,
whose dark and light instances are one struct, so a colour cannot be given a dark value and
forgotten on the light theme. A colour written beside its use is how one came to have no light
value. This script reads every tracked Rust
file of the product code and prints `path:line: text` for each `Color32` written from literal
values: a named one (`Color32::WHITE`), or a `Color32::from_*` call whose arguments are all
literals or constants (`from_rgb(0x5a, 0x5a, 0x5a)`, `from_gray(GREY)`). Exit 1 when anything
matches, 0 when nothing does.

What passes:
- `crates/studio_core/src/theme.rs`, the palette's home;
- a colour computed from data (`from_rgb(r, g, b)`, a name's colour codes, the colour
  picker's value), since it has no fixed value to give a light twin;
- `Color32::PLACEHOLDER` and `Color32::TRANSPARENT`, which are no colour: egui's "the text's
  own colour" marker, and nothing drawn;
- test code: a file under a crate's `tests/`, `examples/` or `benches/`, a module declared
  under `#[cfg(test)]` (with its submodules' files), and any item written under
  `#[cfg(test)]`, since a test pinning an approved value with a literal checks the palette
  rather than drawing from it;
- a whole-line comment.

It reads `Color32` only: a colour kept as a bare `[u8; 3]` array is not seen, so the review
still asks where a new array of channel values is drawn.
"""

import bisect
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The palette's home, where every literal colour lives.
THEME = "crates/studio_core/src/theme.rs"

# Named `Color32` constants that are no colour.
NOT_COLOURS = {"PLACEHOLDER", "TRANSPARENT"}

# A `Color32::` path: a constant (capitals) or a constructor (`from_...`), with its position.
COLOUR = re.compile(r"\bColor32::(?:(?P<name>[A-Z][A-Z0-9_]*)\b|(?P<call>from_\w+)\s*\()")

# One argument that is a fixed value: a number (decimal, hex, float, with a type suffix), a
# string, or a constant's name (capitals, optionally a path ending in one).
FIXED_ARG = re.compile(
    r"\s*(?:"
    r"0x[0-9a-fA-F_]+(?:[ui]\d+)?"
    r"|\d[\d_]*(?:\.\d[\d_]*)?(?:[ui]\d+|f32|f64)?"
    r'|"[^"]*"'
    r"|(?:\w+::)*[A-Z][A-Z0-9_]*"
    r")\s*"
)

CFG_TEST = re.compile(r"^(\s*)#\[cfg\(test\)\]\s*$")
TEST_MOD = re.compile(r"^\s*(?:pub(?:\([\w:]+\))?\s+)?mod\s+(\w+)\s*;")


def call_args(text: str, open_at: int) -> str | None:
    """The text between the parenthesis at `open_at` and its match, or None when it never
    closes (a file cut short)."""
    depth = 0
    for index in range(open_at, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return text[open_at + 1 : index]
    return None


def is_literal_call(args: str) -> bool:
    """Whether every comma-separated argument of a call is a fixed value."""
    parts = args.split(",")
    if parts and not parts[-1].strip():
        parts = parts[:-1]  # a trailing comma, as rustfmt writes a call split across lines
    return bool(parts) and all(FIXED_ARG.fullmatch(part) for part in parts)


def test_item_lines(lines: list[str]) -> set[int]:
    """The 0-based line numbers of every item written under `#[cfg(test)]`.

    rustfmt lays an item out one way, which this reads: a block item (a module, a function)
    opens with a line ending in `{` and closes on the first later line at the attribute's
    indentation starting with `}`; any other item ends at its first line ending in `;`."""
    covered: set[int] = set()
    number = 0
    while number < len(lines):
        found = CFG_TEST.match(lines[number])
        if not found:
            number += 1
            continue
        indent = found.group(1)
        end = number + 1
        # Other attributes and doc comments may sit between the attribute and its item.
        while end < len(lines) and lines[end].strip().startswith(("#[", "///", "//")):
            end += 1
        block = False
        while end < len(lines):
            stripped = lines[end].rstrip()
            if block:
                if lines[end].startswith(indent + "}"):
                    break
            elif stripped.endswith("{"):
                block = True
            elif stripped.endswith(";"):
                break
            end += 1
        covered.update(range(number, end + 1))
        number = end + 1
    return covered


def hits_in(text: str) -> list[tuple[int, str]]:
    """Every (line number, line) of `text`, a Rust file's, where a colour is written from
    literal values outside test code."""
    lines = text.splitlines()
    exempt = test_item_lines(lines)
    # The same text with whole-line comments blanked, so a match keeps its line number.
    code_lines = ["" if line.strip().startswith("//") else line for line in lines]
    code = "\n".join(code_lines)
    starts = [0]
    for line in code_lines:
        starts.append(starts[-1] + len(line) + 1)
    found = []
    for match in COLOUR.finditer(code):
        number = bisect.bisect_right(starts, match.start()) - 1
        if number in exempt:
            continue
        if match.group("name"):
            literal = match.group("name") not in NOT_COLOURS
        else:
            args = call_args(code, match.end() - 1)
            literal = args is not None and is_literal_call(args)
        if literal:
            found.append((number + 1, lines[number].strip()))
    return found


def crate_root(path: str, roots: set[str]) -> str:
    """The folder of the crate `path` belongs to: the deepest folder holding a manifest."""
    parts = path.split("/")
    for cut in range(len(parts) - 1, 0, -1):
        folder = "/".join(parts[:cut])
        if folder in roots:
            return folder
    return ""


def test_module_folders(path: str, text: str) -> list[str]:
    """The files and folders of the modules `path` declares under `#[cfg(test)]`, as path
    prefixes: `mod tests;` in `view/mod.rs` names `view/tests.rs` and `view/tests/`."""
    folder, name = path.rsplit("/", 1)
    if name not in ("mod.rs", "lib.rs", "main.rs"):
        folder = f"{folder}/{name[: -len('.rs')]}"
    lines = text.splitlines()
    prefixes = []
    for number in sorted(test_item_lines(lines)):
        declared = TEST_MOD.match(lines[number])
        if declared:
            module = declared.group(1)
            prefixes += [f"{folder}/{module}.rs", f"{folder}/{module}/"]
    return prefixes


def product_files(paths: list[str], manifests: list[str], read) -> list[str]:
    """The paths among `paths` (tracked Rust files) that are product code: not the theme, not
    a crate's `tests/`, `examples/` or `benches/`, not a module declared under `#[cfg(test)]`.
    `read(path)` returns a file's text."""
    roots = {manifest.rsplit("/", 1)[0] for manifest in manifests}
    exempt: list[str] = []
    for path in paths:
        exempt += test_module_folders(path, read(path))
    kept = []
    for path in paths:
        root = crate_root(path, roots)
        inside = path[len(root) + 1 :] if root else path
        if path == THEME or inside.startswith(("tests/", "examples/", "benches/")):
            continue
        if any(path == prefix or (prefix.endswith("/") and path.startswith(prefix)) for prefix in exempt):
            continue
        kept.append(path)
    return kept


def tracked(pattern: str) -> list[str]:
    """The tracked files under `crates/` matching `pattern`, repository-relative."""
    listed = subprocess.run(
        ["git", "ls-files", f"crates/{pattern}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split("\n")
    return [path for path in listed if path]


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")

    def read(path: str) -> str:
        return (ROOT / path).read_text(encoding="utf-8", errors="replace")

    total = 0
    for path in product_files(tracked("*.rs"), tracked("*Cargo.toml"), read):
        for number, line in hits_in(read(path)):
            total += 1
            print(f"{path}:{number}: {line}")
    print(f"{total} colours written from literal values outside the palette")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
