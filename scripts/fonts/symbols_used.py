"""Every symbol character (U+2000 and up, outside the private use area) the code and the
drafts use, literally or as an escape, with counts per tree and the files that hold it: what
`resources/symbols/` and the body font must cover (`resources/fonts/README.md`).

Run: python scripts/fonts/symbols_used.py
"""
import collections
import pathlib
import re
import sys

ESCAPE = re.compile(r"\\u\{([0-9A-Fa-f]{4,5})\}|\\u([0-9A-Fa-f]{4})|([^\x00-\x7e])")
ROOT = pathlib.Path(__file__).resolve().parents[2]

sys.stdout.reconfigure(encoding="utf-8")
found = {}
for tree in ("crates", "docs/prototypes"):
    for p in (ROOT / tree).rglob("*"):
        if p.suffix not in (".rs", ".js", ".html") or "target" in p.parts:
            continue
        text = p.read_text(encoding="utf-8", errors="replace")
        for m in ESCAPE.finditer(text):
            hexa = m.group(1) or m.group(2)
            c = chr(int(hexa, 16)) if hexa else m.group(3)
            o = ord(c)
            if (0x2000 <= o < 0xE000) or o > 0xF8FF:
                entry = found.setdefault((o, c), collections.Counter())
                entry[str(p.relative_to(ROOT)).replace("\\", "/")] += 1

for (o, c), files in sorted(found.items()):
    trees = collections.Counter(f.split("/")[0] for f in files.elements())
    print(f"U+{o:04X} {c}  {dict(trees)}  {', '.join(sorted(files))[:150]}")
