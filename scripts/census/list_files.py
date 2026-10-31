"""A census's input list: every file the Everything index finds for a search (for example
`ext:model`), one absolute path per line, written to OUT. Refuses when the index returns fewer
rows than it counts (a truncated list would make the census look cleaner than the machine is).

Run: python scripts/census/list_files.py "<Everything search>" <out.txt>
(needs Everything's HTTP server on 127.0.0.1:80)
"""
import json
import os
import sys
import urllib.parse
import urllib.request

search, out = sys.argv[1], sys.argv[2]
url = (
    "http://127.0.0.1/?json=1&count=1000000&path_column=1&sort=path&ascending=1&search="
    + urllib.parse.quote(search)
)
with urllib.request.urlopen(url) as response:
    data = json.load(response)
paths = [
    os.path.join(entry["path"], entry["name"])
    for entry in data["results"]
    if entry["type"] == "file"
]
folders = sum(1 for entry in data["results"] if entry["type"] != "file")
assert len(paths) + folders == data["totalResults"], (len(paths), folders, data["totalResults"])
tmp = out + ".tmp"
with open(tmp, "w", encoding="utf-8") as handle:
    handle.write("\n".join(paths) + "\n")
os.replace(tmp, out)
print(len(paths), "files,", folders, "folders left out:", out)
