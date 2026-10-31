"""Inflate WESYS-wrapped files (16-byte header: magic, stored size, inflated size, then zlib).

  python scripts/unwesys.py FILE...   writes FILE.raw beside each, prints its size and first bytes
"""

import struct
import sys
import zlib
from pathlib import Path

for name in sys.argv[1:]:
    data = Path(name).read_bytes()
    assert data[3:8] == b"WESYS", name
    stored, inflated = struct.unpack_from("<II", data, 8)
    raw = zlib.decompress(data[16 : 16 + stored])
    assert len(raw) == inflated, (name, len(raw), inflated)
    Path(name + ".raw").write_bytes(raw)
    print(name, inflated, raw[:32].hex(), raw[:32])
