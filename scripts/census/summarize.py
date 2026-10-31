"""A census run's tally: an outcomes TSV (outcome, size in bytes, path; one file a row) counted
per outcome class, the numbers after "at" collapsed (so every "truncated at 1234" is one class),
largest class first, with the four smallest files of each: the ones to diagnose first.

Run: python scripts/census/summarize.py <outcomes.tsv>
"""
import collections
import re
import sys

by_class = collections.defaultdict(list)
with open(sys.argv[1], encoding="utf-8") as handle:
    for line in handle:
        outcome, size, file = line.rstrip("\n").split("\t")
        by_class[re.sub(r"at -?\d+", "at N", outcome)].append((int(size), file))
total = sum(len(files) for files in by_class.values())
print(total, "files")
for outcome, files in sorted(by_class.items(), key=lambda item: -len(item[1])):
    files.sort()
    print(len(files), outcome)
    for size, file in files[:4]:
        print("   ", size, file)
