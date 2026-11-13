"""Compares two census runs' outcomes path by path (outcomes_<a>.tsv, outcomes_<b>.tsv): how many
inputs kept their outcome, every change of outcome grouped by (before -> after) with up to five
paths each, and the inputs only one run holds. With reports_<tag>.txt, it also prints the lines a
changed input lost and gained. Both runs' files are under `config.CENSUS_DIR`.
An input that is no longer on disk is counted as gone, not as changed: the library is whatever
the maintainer's drives hold, and a folder removed to free space is no difference in the suite.
Usage: regress_check.py <a> <b>"""
import os
import sys
from collections import defaultdict

import config

HERE = config.CENSUS_DIR
a_tag, b_tag = sys.argv[1], sys.argv[2]


def outcomes(tag):
    rows = {}
    for line in open(os.path.join(HERE, f"outcomes_{tag}.tsv"), encoding="utf-8"):
        path, _code, outcome, *_ = line.rstrip("\n").split("\t")
        rows[path] = outcome
    return rows


def reports(tag):
    found, current = defaultdict(list), None
    if not os.path.exists(os.path.join(HERE, f"reports_{tag}.txt")):
        print(f"no reports_{tag}.txt: the lines lost and gained are not listed")
        return None
    for line in open(os.path.join(HERE, f"reports_{tag}.txt"), encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("=== "):
            current = line[4:]
        elif line.startswith("- ") and current is not None:
            found[current].append(line)
    return found


a, b = outcomes(a_tag), outcomes(b_tag)
ra, rb = reports(a_tag), reports(b_tag)
gone = sorted((path for path in set(a) & set(b) if not os.path.exists(path)), key=str.lower)
both = sorted(set(a) & set(b) - set(gone), key=str.lower)
same = [path for path in both if a[path] == b[path]]
changes = defaultdict(list)
for path in both:
    if a[path] != b[path]:
        changes[(a[path], b[path])].append(path)
print(f"{len(both)} inputs in both runs: {len(same)} same outcome, {len(both) - len(same)} changed")
print(f"only in {a_tag}: {len(set(a) - set(b))}; only in {b_tag}: {len(set(b) - set(a))}")
if gone:
    folders = sorted({os.path.dirname(path) for path in gone}, key=str.lower)
    print(f"{len(gone)} inputs are no longer on disk, under: {', '.join(folders[:10])}")
for (before, after), paths in sorted(changes.items(), key=lambda item: -len(item[1])):
    print(f"\n{len(paths):4}  {before}\n   -> {after}")
    for path in paths[:5]:
        if ra is None or rb is None:
            # Counted against a missing side, every line would read as lost or gained.
            print(f"        {path}")
            continue
        lost =[line for line in ra[path] if line not in rb[path]]
        gained = [line for line in rb[path] if line not in ra[path]]
        print(f"        {path}: {len(lost)} lines lost, {len(gained)} gained")
        for line in gained[:3]:
            print(f"            + {line[:200]}")
