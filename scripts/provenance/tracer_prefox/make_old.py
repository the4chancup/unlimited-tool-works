"""Cut the /jp/ Summer 18 export down to the pre-Fox tracer's old-layout source: one player with
face, boots and gloves (72820 Fumos, as XXX20) and kit g1, the large textures reduced.

Writes .tmp/phase_04/4_16/tracer_prefox/old/jp Tracer/ (refuses to run when it exists).
"""

import shutil
import struct
from pathlib import Path

SOURCE = Path(r"C:\Data\4cc\Teams_Main\JP\Exports\JP Aesthetic Export for Summer 18")
TARGET = Path(r"C:\Data\4cc\Tools_Mine\4cc-studio\.tmp\4_16\tracer_prefox") / "old" / "jp Tracer"
FACE_SOURCE = SOURCE / "Faces" / "72820 - Fumos"
FACE_TARGET = TARGET / "Faces" / "XXX20 - Fumos"

# The hair set face.xml does not list (never loaded in game) is left out.
LEFT_OUT = {"hair.mtl", "hair_high_win32.model", "hair_col.dds", "hair_parts_col.dds"}
# DXT textures reduced by keeping every n-th 4x4 block in each direction.
REDUCED = {"face.dds": 4, "k2012_c.dds": 4}
# Uncompressed 32-bit textures reduced by keeping every n-th pixel in each direction.
REDUCED_RAW = {"k2012_sr.dds": 2}
KIT_REDUCTION = 8

DDSD_LINEARSIZE = 0x80000
DDSD_PITCH = 0x8


def header_fields(data: bytes):
    assert data[:4] == b"DDS ", data[:4]
    flags, height, width, pitch, _depth, mips = struct.unpack_from("<IIIIII", data, 8)
    return flags, height, width, pitch, mips


def reduced_dxt(data: bytes, factor: int) -> bytes:
    flags, height, width, _pitch, mips = header_fields(data)
    fourcc = data[84:88]
    block = {b"DXT1": 8, b"DXT3": 16, b"DXT5": 16}[fourcc]
    assert mips <= 1, mips
    blocks_x, blocks_y = width // 4, height // 4
    assert len(data) == 128 + blocks_x * blocks_y * block, (len(data), width, height)
    assert blocks_x % factor == 0 and blocks_y % factor == 0
    out = bytearray()
    for by in range(0, blocks_y, factor):
        for bx in range(0, blocks_x, factor):
            start = 128 + (by * blocks_x + bx) * block
            out += data[start : start + block]
    new_w, new_h = width // factor, height // factor
    header = bytearray(data[:128])
    struct.pack_into("<II", header, 12, new_h, new_w)
    assert flags & DDSD_LINEARSIZE, hex(flags)
    struct.pack_into("<I", header, 20, len(out))
    return bytes(header) + bytes(out)


def reduced_raw(data: bytes, factor: int) -> bytes:
    flags, height, width, _pitch, mips = header_fields(data)
    bits = struct.unpack_from("<I", data, 88)[0]
    assert data[84:88] == b"\0\0\0\0" and bits == 32, (data[84:88], bits)
    assert mips <= 1, mips
    assert len(data) == 128 + width * height * 4, (len(data), width, height)
    out = bytearray()
    for y in range(0, height, factor):
        for x in range(0, width, factor):
            start = 128 + (y * width + x) * 4
            out += data[start : start + 4]
    new_w, new_h = width // factor, height // factor
    header = bytearray(data[:128])
    struct.pack_into("<II", header, 12, new_h, new_w)
    if flags & DDSD_PITCH:
        struct.pack_into("<I", header, 20, new_w * 4)
    return bytes(header) + bytes(out)


assert not TARGET.exists(), f"{TARGET} exists; remove it by hand first"
assert FACE_SOURCE.is_dir(), FACE_SOURCE
names = {path.name for path in FACE_SOURCE.iterdir()}
assert LEFT_OUT <= names and set(REDUCED) <= names and set(REDUCED_RAW) <= names, names

FACE_TARGET.mkdir(parents=True)
for path in sorted(FACE_SOURCE.iterdir()):
    assert path.is_file(), path
    if path.name in LEFT_OUT:
        continue
    data = path.read_bytes()
    if path.name in REDUCED:
        data = reduced_dxt(data, REDUCED[path.name])
    elif path.name in REDUCED_RAW:
        data = reduced_raw(data, REDUCED_RAW[path.name])
    (FACE_TARGET / path.name).write_bytes(data)

(TARGET / "Kit Configs").mkdir()
shutil.copy2(
    SOURCE / "Kit Configs" / "728" / "728_DEF_GK1st_realUni.bin",
    TARGET / "Kit Configs" / "XXX_DEF_GK1st_realUni.bin",
)
(TARGET / "Kit Textures").mkdir()
kit = (SOURCE / "Kit Textures" / "u0728g1.dds").read_bytes()
(TARGET / "Kit Textures" / "u0XXXg1.dds").write_bytes(reduced_dxt(kit, KIT_REDUCTION))

note = (SOURCE / "JP Note.txt").read_bytes()
lines = note.split(b"\r\n")
# The player kits' color lines go with their kits, and the player list keeps Fumos alone: one
# name in it is not UTF-8, which Red cannot read.
kept = [
    line
    for line in lines
    if not (line.startswith(b"- ") and b"Player:" in line)
    and not (line.startswith(b"- 728") and line != b"- 72820 = Fumos")
]
assert len(lines) - len(kept) == 5 + 22, len(lines) - len(kept)
kept[kept.index(b"- 72820 = Fumos")] = b"- XXX20 = Fumos"
assert kept.count(b"ID: 728") == 1
kept[kept.index(b"ID: 728")] = b"ID: 731"
trimmed = b"\r\n".join(kept)
trimmed.decode("utf-8")  # raises if a line Red cannot read is left
assert b"1st GK" in trimmed and b"Player:" not in trimmed
(TARGET / "jp note.txt").write_bytes(trimmed)

total = 0
for path in sorted(TARGET.rglob("*")):
    if path.is_file():
        total += path.stat().st_size
        print(f"{path.stat().st_size:8d}  {path.relative_to(TARGET)}")
print("total", total)
