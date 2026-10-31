"""Compare two `cargo bloat --release --crates` tables (KiB per crate): what grew and shrank
between two builds of the exe.

Run: python scripts/bloat_compare.py <old table> <new table>
"""
import re, sys
def table(path):
    rows = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"\s*[\d.]+%\s+[\d.]+%\s+([\d.]+)(KiB|MiB|B)\s+(\S+)$", line)
        if m:
            v = float(m.group(1)) * {"KiB": 1, "MiB": 1024, "B": 1 / 1024}[m.group(2)]
            rows[m.group(3)] = v
        if "file size is" in line:
            print(path, line.strip())
    return rows
old, new = table(sys.argv[1]), table(sys.argv[2])
diffs = sorted(((new.get(k, 0) - old.get(k, 0), k) for k in set(old) | set(new)), reverse=True)
for d, k in diffs[:25]:
    print(f"{d:+9.1f} KiB {k:24} old {old.get(k, 0):8.1f} new {new.get(k, 0):8.1f}")
print("negative:")
for d, k in diffs[-8:]:
    print(f"{d:+9.1f} KiB {k:24} old {old.get(k, 0):8.1f} new {new.get(k, 0):8.1f}")
