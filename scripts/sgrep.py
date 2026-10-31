"""Greps a file case-insensitively, skipping lines over a length: the drafts'
(`docs/prototypes/`) embedded data rows run to thousands of characters and drown a plain grep.

Run: python scripts/sgrep.py <file> <regex> [<max line length, default 400>]
"""
import re
import sys

path, pattern = sys.argv[1], sys.argv[2]
limit = int(sys.argv[3]) if len(sys.argv) > 3 else 400
rx = re.compile(pattern, re.IGNORECASE)
with open(path, encoding="utf-8") as f:
    for n, line in enumerate(f, 1):
        if len(line) <= limit and rx.search(line):
            print(f"{n}: {line.rstrip()}")
