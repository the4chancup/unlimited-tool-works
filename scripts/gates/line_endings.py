"""List tracked files whose working copy has other line endings than `.gitattributes` asks.

    python scripts/gates/line_endings.py

Every text file is LF (`.gitattributes`' `* text=auto eol=lf`), a few are CRLF by contract
(`eol=crlf`), and game files are stored as they are (`-text`). Git normalizes a text file
when it is committed, so a CRLF working copy of an LF file never reaches the history; it
still breaks what reads the working copy: `cargo fmt --check` (rustfmt's
`newline_style = "Unix"`), a test that hashes or compares a file's bytes (`aatf`'s
`logic.rhai` hash), `git diff` showing every line changed. On Windows, Python's text-mode
writes (`open(path, "w")`, `Path.write_text`) turn every `\\n` into `\\r\\n`, which is how
such working copies come about. This script reads `git ls-files --eol` and prints each file
whose working copy is CRLF or mixed where LF is asked, or LF or mixed where CRLF is asked.
Exit 1 when anything is listed, 0 when nothing is.

Untracked files are not seen: a new file's line endings show here once it is added.
"""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# What `git ls-files --eol` reports for a working copy that contradicts each asked ending.
WRONG = {"eol=lf": {"w/crlf", "w/mixed"}, "eol=crlf": {"w/lf", "w/mixed"}}


def wrong_endings(listing: str) -> list[tuple[str, str, str]]:
    """Each (path, working copy's ending, asked ending) of `git ls-files --eol` output whose
    working copy contradicts its attributes. A line reads `i/lf    w/crlf  attr/text=auto
    eol=lf <tab>path`; a `-text` file asks nothing."""
    found = []
    for line in listing.splitlines():
        if "\t" not in line:
            continue
        info, path = line.split("\t", 1)
        fields = info.split()
        work = next((field for field in fields if field.startswith("w/")), "")
        asked = next((field for field in fields if field in WRONG), None)
        if asked is not None and work in WRONG[asked]:
            found.append((path, work[len("w/") :], asked[len("eol=") :]))
    return found


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    listing = subprocess.run(
        ["git", "-c", "core.quotepath=off", "ls-files", "--eol"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    ).stdout
    found = wrong_endings(listing)
    for path, work, asked in found:
        print(f"{path}: the working copy is {work}, .gitattributes asks {asked}")
    print(f"{len(found)} working copies with other line endings than .gitattributes asks")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
