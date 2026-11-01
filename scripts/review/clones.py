"""Finds repeated code shapes in the workspace's Rust: CONTRIBUTING.md's "The third copy is a
function", measured. Each line is reduced to its shape (every identifier `I`, every string `S`,
every number `N`; keywords, punctuation and macros' `!` kept), lines that are only brackets,
`use`/`mod` lines and attributes dropped, and comments removed. A run of at least `WINDOW` shaped
lines found three or more times in one crate, or a run of at least `LONG` lines found twice,
is a cluster. Clusters are listed per crate, non-test code first, ranked by the lines a helper
would save ((copies - 1) x run length).

Usage: clones.py [--window N] [--long N] [<crate path prefix> ...] > report.txt
(prefixes relative to crates/, e.g. `libs/aatf`; none: every crate).
"""
import argparse
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATES = ROOT / "crates"

KEYWORDS = {
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while", "Some", "None", "Ok", "Err",
}
TOKEN = re.compile(
    r"""
    (?P<string>b?r(?P<hashes>\#*)".*?"(?P=hashes)|b?"(?:\\.|[^"\\])*")
  | (?P<char>b?'(?:\\.|[^'\\])')
  | (?P<lifetime>'[A-Za-z_][A-Za-z0-9_]*)
  | (?P<number>\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?[A-Za-z0-9_]*|0x[0-9A-Fa-f_]+[A-Za-z0-9_]*)
  | (?P<ident>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<punct>::|->|=>|==|!=|<=|>=|&&|\|\||\.\.=?|[^\sA-Za-z0-9_])
    """,
    re.VERBOSE,
)
BRACKETS_ONLY = re.compile(r"^[\s{}()\[\];,]*$")
ACTING = re.compile(r"\(|(?<![=!<>])=(?![=>])|\. I|\b(if|let|match|for|while|return|loop)\b|\?")


def strip_comments(text: str) -> str:
    """`text` with `//` and `/* */` comments blanked, strings left alone, line count kept."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if text.startswith("//", i):
            end = text.find("\n", i)
            i = n if end < 0 else end
        elif text.startswith("/*", i):
            depth = 0
            while i < n:
                if text.startswith("/*", i):
                    depth += 1
                    i += 2
                elif text.startswith("*/", i):
                    depth -= 1
                    i += 2
                    if depth == 0:
                        break
                else:
                    if text[i] == "\n":
                        out.append("\n")
                    i += 1
        elif ch == '"' or (ch in "rb" and re.match(r'b?r#*"|b"', text[i:i + 4])):
            match = TOKEN.match(text, i)
            if match and match.group("string"):
                out.append(match.group(0))
                i = match.end()
            else:
                out.append(ch)
                i += 1
        else:
            out.append(ch)
            i += 1
    return "".join(out)


def shape(line: str) -> str:
    """The line's shape: identifiers, strings and numbers replaced by one letter each."""
    parts = []
    for match in TOKEN.finditer(line):
        kind = match.lastgroup
        text = match.group(0)
        if kind == "string" or kind == "char":
            parts.append("S")
        elif kind == "number":
            parts.append("N")
        elif kind == "lifetime":
            parts.append("'L")
        elif kind == "ident":
            parts.append(text if text in KEYWORDS else "I")
        else:
            parts.append(text)
    return " ".join(parts)


CFG_TEST_MOD = re.compile(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([a-z]+\))?\s+)?mod\s+([a-z_0-9]+)\s*;")


def test_modules(crate: Path) -> set:
    """The files and folders of every module declared `#[cfg(test)] mod name;`: test code
    whatever its path (aatf's `src/fixtures/`)."""
    found = set()
    for path in crate.rglob("*.rs"):
        if "target" in path.parts:
            continue
        text = strip_comments(path.read_text(encoding="utf-8", errors="replace"))
        home = path.parent if path.name in ("lib.rs", "main.rs", "mod.rs") else path.with_suffix("")
        for name in CFG_TEST_MOD.findall(text):
            found.add(home / f"{name}.rs")
            found.add(home / name)
    return found


def is_test_path(path: Path, crate: Path, modules: set) -> bool:
    relative = path.relative_to(crate).parts
    if any(path == module or module in path.parents for module in modules):
        return True
    return relative[0] in ("tests", "benches", "examples") or path.name == "tests.rs" or (
        "tests" in relative[:-1]
    )


def shaped_lines(path: Path):
    """(line number, shape, is_test) of each significant line; in a source file, the lines from
    a `#[cfg(test)]` on count as test lines."""
    text = strip_comments(path.read_text(encoding="utf-8", errors="replace"))
    in_tests = False
    previous = None
    for number, line in enumerate(text.splitlines(), start=1):
        stripped = line.strip()
        if stripped.startswith("#[cfg(test)]"):
            in_tests = True
        if (
            not stripped
            or BRACKETS_ONLY.match(stripped)
            or stripped.startswith(("use ", "pub use ", "pub(crate) use ", "mod ", "pub mod "))
            or stripped.startswith(("#[", "#!["))
        ):
            continue
        line_shape = shape(stripped)
        # Consecutive lines of one shape are a table's rows (a list of records, a match's
        # arms), not copies: they count once.
        if line_shape == previous:
            continue
        previous = line_shape
        yield number, line_shape, in_tests


def crates(prefixes):
    for manifest in sorted(CRATES.rglob("Cargo.toml")):
        crate = manifest.parent
        if "target" in crate.parts or ".tmp" in crate.parts:
            continue
        name = str(crate.relative_to(CRATES)).replace("\\", "/")
        if prefixes and not any(name.startswith(prefix) for prefix in prefixes):
            continue
        yield name, crate


def clusters(sequences, window, long):
    """Maximal repeated runs over `sequences` ({file: [(line, shape)]}): each a tuple
    (run length in shaped lines, [(file, first line, last line)])."""
    hashes = {}
    occurrences = defaultdict(list)
    for file, lines in sequences.items():
        keys = []
        for start in range(len(lines) - window + 1):
            key = tuple(shape for _, shape in lines[start:start + window])
            keys.append(key)
            occurrences[key].append((file, start))
        hashes[file] = keys

    def occurrence_set(file, start):
        if start < 0 or start >= len(hashes[file]):
            return None
        return occurrences[hashes[file][start]]

    found = []
    seen = set()
    for key, places in occurrences.items():
        if len(places) < 2:
            continue
        shifted_back = sorted((f, s - 1) for f, s in places)
        previous = occurrence_set(*places[0][:1], places[0][1] - 1)
        if previous is not None and sorted(previous) == shifted_back:
            continue  # not the start of the run
        length = 0
        while True:
            following = occurrence_set(places[0][0], places[0][1] + length + 1)
            if following is None or sorted(following) != sorted(
                (f, s + length + 1) for f, s in places
            ):
                break
            length += 1
        run = window + length
        # A periodic run overlaps its own copies: keep the copies that do not overlap.
        kept = []
        for file, start in sorted(places):
            if kept and kept[-1][0] == file and start < kept[-1][1] + run:
                continue
            kept.append((file, start))
        places = kept
        if len(places) < 2 or (len(places) < 3 and run < long):
            continue
        # Declarations (an enum's variants, a struct's fields) repeat a shape without being
        # copied logic: a run counts only with two lines that act (a call, an assignment, a
        # field access, a control keyword).
        first_file, first_start = places[0]
        acting = sum(
            1
            for _, line_shape in sequences[first_file][first_start:first_start + run]
            if ACTING.search(line_shape)
        )
        if acting < 2:
            continue
        region = tuple(sorted(places))
        if region in seen:
            continue
        seen.add(region)
        spans = []
        for file, start in sorted(places):
            lines = sequences[file]
            spans.append((file, lines[start][0], lines[start + run - 1][0]))
        found.append((run, spans))
    found.sort(key=lambda item: (len(item[1]) - 1) * item[0], reverse=True)
    return found


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--window", type=int, default=4)
    parser.add_argument("--long", type=int, default=10)
    parser.add_argument(
        "--across",
        action="store_true",
        help="pool every crate's non-test code and list only the runs found in two crates or more",
    )
    parser.add_argument("prefixes", nargs="*")
    args = parser.parse_args()
    if args.across:
        pooled = {}
        for name, crate in crates(args.prefixes):
            modules = test_modules(crate)
            for path in sorted(crate.rglob("*.rs")):
                if "target" in path.parts or is_test_path(path, crate, modules):
                    continue
                relative = str(path.relative_to(ROOT)).replace("\\", "/")
                lines = [(n, s) for n, s, in_tests in shaped_lines(path) if not in_tests]
                if lines:
                    pooled[relative] = lines

        def crate_of(file):
            parts = file.split("/")
            return "/".join(parts[1:3]) if parts[1] in ("libs", "tools", "dev") else parts[1]

        found = [
            (run, spans)
            for run, spans in clusters(pooled, args.window, args.long)
            if len({crate_of(file) for file, _, _ in spans}) > 1
        ]
        print(f"== across crates: {len(found)} clusters")
        for run, spans in found:
            print(f"  {len(spans)} copies x {run} lines:")
            for file, first, last in spans:
                print(f"    {file}:{first}-{last}")
        return
    for name, crate in crates(args.prefixes):
        source = {}
        tests = {}
        modules = test_modules(crate)
        for path in sorted(crate.rglob("*.rs")):
            if "target" in path.parts:
                continue
            relative = str(path.relative_to(ROOT)).replace("\\", "/")
            test_file = is_test_path(path, crate, modules)
            for number, line_shape, in_tests in shaped_lines(path):
                kind = tests if (test_file or in_tests) else source
                kind.setdefault(relative, []).append((number, line_shape))
        for label, sequences in (("source", source), ("tests", tests)):
            found = clusters(sequences, args.window, args.long)
            if not found:
                continue
            saved = sum((len(spans) - 1) * run for run, spans in found)
            print(f"== {name} ({label}): {len(found)} clusters, about {saved} shaped lines repeated")
            for run, spans in found:
                print(f"  {len(spans)} copies x {run} lines:")
                for file, first, last in spans:
                    print(f"    {file}:{first}-{last}")


if __name__ == "__main__":
    main()
