"""List corner radii written as numbers outside the theme.

    python scripts/gates/corner_radius.py

Every rectangle the suite paints takes its corner radius from `studio_core`'s theme, 2 points
(ST-CORNER-01), so a widget added later cannot bring its own. This script reads every tracked
Rust file under `crates/` and prints `path:line: text` for each `CornerRadius::same(` or
`.corner_radius(` whose first argument starts with a digit. Exit 1 when anything matches, 0
when nothing does.

What passes:
- `crates/studio_core/src/theme.rs`, the radius's home;
- a radius read from the style or a named constant (the captain's mark names its two);
- files of tests alone: a `tests.rs`, or any file under a folder named `tests`. A
  `#[cfg(test)]` block inside a source file is read, so a radius checked there reads the
  style like the code beside it.

It was a test of `studio_core` until a slice whose Verify ran only its own crate's tests let
a numbered radius through: a gate runs over every crate whichever crate changed.
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The radius's home.
THEME = "crates/studio_core/src/theme.rs"

# The calls whose first argument is a corner radius.
NEEDLES = ("CornerRadius::same(", ".corner_radius(")


def numbered_radius(line: str) -> bool:
    """`line` passes a number as a corner radius: a needle with a digit right after it."""
    for needle in NEEDLES:
        at = line.find(needle)
        while at != -1:
            after = line[at + len(needle) : at + len(needle) + 1]
            if after.isdigit():
                return True
            at = line.find(needle, at + 1)
    return False


def is_test_file(path: str) -> bool:
    """`path` (relative to the repository, with `/`) holds tests alone."""
    parts = path.split("/")
    return parts[-1] == "tests.rs" or "tests" in parts[:-1]


def product_files() -> list[str]:
    """The tracked Rust files under `crates/` the check reads."""
    listed = subprocess.run(
        ["git", "ls-files", "crates/*.rs", "crates/**/*.rs"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.split()
    return sorted(path for path in set(listed) if path != THEME and not is_test_file(path))


def main() -> int:
    found = []
    for path in product_files():
        text = (ROOT / path).read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), start=1):
            if numbered_radius(line):
                found.append(f"{path}:{number}: {line.strip()}")
    for hit in found:
        print(hit)
    if found:
        print("corner_radius: a number passed as a corner radius; take the theme's "
              "(ui.visuals().widgets.*.corner_radius, or a style value)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
