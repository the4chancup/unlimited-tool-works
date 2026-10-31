"""python scripts/strings.py FILE [PATTERN]: print the file's printable ASCII runs of 4+ bytes, those matching the
case-insensitive regex PATTERN if given."""
import re, sys

data = open(sys.argv[1], "rb").read()
pattern = re.compile(sys.argv[2], re.I) if len(sys.argv) > 2 else None
for run in re.findall(rb"[\x20-\x7e]{4,}", data):
    text = run.decode()
    if pattern is None or pattern.search(text):
        print(text)
