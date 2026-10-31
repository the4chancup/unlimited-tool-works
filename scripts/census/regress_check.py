"""A census's regression check after a fix: every file whose outcome was `ok…` in the BEFORE run
must still be `ok…` in the AFTER run, then the table of every outcome change. Both are outcomes
TSVs (outcome, size, path). Exits 1 when a file regressed.

Run: python scripts/census/regress_check.py <before.tsv> <after.tsv>
"""
import collections
import sys


def outcomes(path):
    rows = {}
    for line in open(path, encoding="utf-8"):
        fields = line.rstrip("\n").split("\t")
        if len(fields) >= 3:
            rows[fields[2]] = fields[0]
    return rows


before, after = outcomes(sys.argv[1]), outcomes(sys.argv[2])
print(len(before), "before,", len(after), "after,", len(set(before) ^ set(after)),
      "paths in only one run")
regressed = [(path, was, after.get(path)) for path, was in before.items()
             if was.startswith("ok") and not (after.get(path) or "").startswith("ok")]
for path, was, now in regressed:
    print(f"REGRESSED: {was} -> {now}: {path}")
print(len(regressed), "files regressed from ok")
changes = collections.Counter((was, after.get(path)) for path, was in before.items()
                              if after.get(path) != was)
for (was, now), count in sorted(changes.items(), key=lambda item: -item[1]):
    print(f"{count:5} {was!r} -> {now!r}")
sys.exit(1 if regressed else 0)
