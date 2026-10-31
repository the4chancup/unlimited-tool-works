"""Is the working tree's change under a directory a pure move against a commit?

Compares the multiset of code lines (stripped, blank lines dropped) over every
.rs file under DIR at BASE and in the working tree. `use`/`mod` lines and
visibility-only differences are normalized away and reported separately;
everything else must be identical as a multiset.

    python scripts/pure_move_check.py <base-commit> <dir>
"""

import collections
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VIS = re.compile(r"^pub(\((crate|super|in [\w:]+)\))? ")


def lines_of(text: str) -> tuple[collections.Counter, collections.Counter]:
    code, imports = collections.Counter(), collections.Counter()
    in_use = False
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        if in_use or re.match(r"^(pub(\([^)]*\))? )?(use|mod) ", line):
            imports[line] += 1
            in_use = not line.endswith(";")
            continue
        code[VIS.sub("", line)] += 1
    return code, imports


def at_base(base: str, directory: str) -> tuple[collections.Counter, collections.Counter]:
    files = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", base, directory],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.split()
    code, imports = collections.Counter(), collections.Counter()
    for f in files:
        if f.endswith(".rs"):
            text = subprocess.run(
                ["git", "show", f"{base}:{f}"], cwd=ROOT, check=True,
                capture_output=True, text=True, encoding="utf-8",
            ).stdout
            c, i = lines_of(text)
            code += c
            imports += i
    return code, imports


def in_tree(directory: str) -> tuple[collections.Counter, collections.Counter]:
    code, imports = collections.Counter(), collections.Counter()
    for path in (ROOT / directory).rglob("*.rs"):
        c, i = lines_of(path.read_text(encoding="utf-8"))
        code += c
        imports += i
    return code, imports


def main() -> int:
    base, directory = sys.argv[1], sys.argv[2]
    before, before_imports = at_base(base, directory)
    after, after_imports = in_tree(directory)
    removed, added = before - after, after - before
    print(f"code lines: base {sum(before.values())}, tree {sum(after.values())}")
    print(f"import/mod lines changed: -{sum((before_imports - after_imports).values())} "
          f"+{sum((after_imports - before_imports).values())} (not compared)")
    for label, diff in (("REMOVED", removed), ("ADDED", added)):
        for line, n in sorted(diff.items()):
            print(f"{label} x{n}: {line}")
    if removed or added:
        print("NOT a pure move")
        return 1
    print("pure move: code lines identical as a multiset (visibility ignored)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
