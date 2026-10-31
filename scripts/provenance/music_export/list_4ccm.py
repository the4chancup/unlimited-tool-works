"""List every .4ccm on this machine through the Everything HTTP API, one absolute path per line.

Usage: python -I scripts/provenance/music_export/list_4ccm.py .tmp/phase_10/music_export/all_4ccm.txt

The output is the input of `music_export`'s census test (`crates/libs/music_export/tests/census.rs`),
which is skipped when the file is absent. Needs Everything 1.5 with its HTTP server on port 80.
"""

import json
import sys
import urllib.request

URL = "http://127.0.0.1/?json=1&count=5000&path_column=1&search=ext%3A4ccm"


def main() -> None:
    out = sys.argv[1]
    data = json.load(urllib.request.urlopen(URL))
    rows = sorted(r["path"] + "\\" + r["name"] for r in data["results"])
    assert len(rows) == data["totalResults"], "the count exceeds the request's page"
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        for r in rows:
            f.write(r + "\n")
    print(f"{len(rows)} files listed in {out}")


if __name__ == "__main__":
    main()
