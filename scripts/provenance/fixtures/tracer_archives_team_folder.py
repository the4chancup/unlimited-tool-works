"""Rebuild the Team compiler's tracer archives (`tests/fixtures/sources/egg Midcup Tracer*`) with
the respelled `fcl_hair.fmdl` (worklog 0.0.2-poc12: first its `dummy_kit` directory named
`common/team_folder/`, no `000`; then the stand-in itself `team_kit`, `fmdl_team_kit.rs`).
Run once after each respelling of the studio tracer's file, before it is committed.

Each archive's `fcl_hair.fmdl` must be byte-identical to the studio tracer's file at HEAD (the
pre-respell bytes) before it is replaced by the working copy's; every other entry keeps its
name, order, timestamp, attributes, compression and bytes. The bad-notes zip's `notes.txt`
keeps its deliberately wrong CRC-32, patched back into its local header and central directory
after the write. Each archive is written to a sibling temp path, read back, then `os.replace`d.
"""
import os
import shutil
import struct
import subprocess
import zipfile
from pathlib import Path

ROOT = Path("C:/Data/4cc/Tools_Mine/4cc-studio")
SOURCES = ROOT / "crates/tools/team_compiler/tests/fixtures/sources"
REL = "tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/fcl_hair.fmdl"
FIXTURE = ROOT / "crates/tools/team_compiler/tests/fixtures" / REL
ENTRY = "Players/05 - The Chad Stormworks Player/fcl_hair.fmdl"
SEVEN_ZIP = Path("C:/Program Files/7-Zip/7z.exe")
STAGE = ROOT / ".tmp/0_0_2/poc12/archive_stage"

old = subprocess.run(["git", "show", "HEAD:crates/tools/team_compiler/tests/fixtures/" + REL],
                     cwd=ROOT, check=True, capture_output=True).stdout
new = FIXTURE.read_bytes()
assert old != new, "the working copy's fcl_hair.fmdl is not respelled"


def raw_stored(path, info):
    """A stored entry's bytes read past zipfile's CRC check (the bad-notes entry)."""
    data = path.read_bytes()
    offset = info.header_offset
    assert data[offset:offset + 4] == b"PK\x03\x04"
    name_len, extra_len = struct.unpack_from("<HH", data, offset + 26)
    start = offset + 30 + name_len + extra_len
    return data[start:start + info.compress_size]


def rebuild_zip(path):
    with zipfile.ZipFile(path) as source:
        infos = source.infolist()
        entries = []
        bad = []
        for info in infos:
            try:
                body = source.read(info)
            except zipfile.BadZipFile:
                assert info.compress_type == zipfile.ZIP_STORED
                body = raw_stored(path, info)
                bad.append(info.filename)
            if info.filename == ENTRY:
                assert body == old, path.name
                body = new
            entries.append((info, body))
    original = path.read_bytes()
    temp = path.with_name(path.name + ".tmp")
    with zipfile.ZipFile(temp, "w") as target:
        for info, body in entries:
            copy = zipfile.ZipInfo(info.filename, info.date_time)
            copy.compress_type = info.compress_type
            copy.external_attr = info.external_attr
            copy.create_system = info.create_system
            target.writestr(copy, body)
    if bad:
        data = bytearray(temp.read_bytes())
        with zipfile.ZipFile(temp) as written:
            for name in bad:
                old_info = next(i for i, _ in entries if i.filename == name)
                local_crc = struct.unpack_from("<I", original, old_info.header_offset + 14)[0]
                new_info = written.getinfo(name)
                struct.pack_into("<I", data, new_info.header_offset + 14, local_crc)
                encoded = name.encode("utf-8")
                # This entry's central directory record, found by its name.
                position = 0
                while True:
                    position = data.index(b"PK\x01\x02", position)
                    name_len = struct.unpack_from("<H", data, position + 28)[0]
                    if bytes(data[position + 46:position + 46 + name_len]) == encoded:
                        break
                    position += 4
                struct.pack_into("<I", data, position + 16, old_info.CRC)
        temp.write_bytes(bytes(data))
    with zipfile.ZipFile(temp) as check:
        names = [i.filename for i in check.infolist()]
        assert names == [i.filename for i, _ in entries]
        for info, body in entries:
            if info.filename in bad:
                assert raw_stored(temp, check.getinfo(info.filename)) == body
                assert check.getinfo(info.filename).CRC == info.CRC
                try:
                    check.read(info.filename)
                    raise AssertionError("the bad CRC survived")
                except zipfile.BadZipFile:
                    pass
            else:
                assert check.read(info.filename) == body
                got = check.getinfo(info.filename)
                assert got.date_time == info.date_time and got.compress_type == info.compress_type
    os.replace(temp, path)
    print(path.name, path.stat().st_size, "bytes", "(bad CRC kept: %s)" % bad if bad else "")


def rebuild_7z(path):
    if STAGE.exists():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True)
    subprocess.run([str(SEVEN_ZIP), "x", "-y", str(path)], cwd=STAGE, check=True,
                   capture_output=True)
    staged = STAGE / ENTRY
    assert staged.read_bytes() == old
    stat = staged.stat()
    staged.write_bytes(new)
    os.utime(staged, (stat.st_atime, stat.st_mtime))
    temp = path.with_name(path.name + ".tmp")
    assert not temp.exists()
    subprocess.run([str(SEVEN_ZIP), "a", "-t7z", "-ms=on", "-m0=lzma2", str(temp), "Kits",
                    "Players"], cwd=STAGE, check=True, capture_output=True)
    listing = subprocess.run([str(SEVEN_ZIP), "l", "-slt", str(temp)], check=True,
                             capture_output=True, text=True).stdout
    assert "Solid = +" in listing
    before = subprocess.run([str(SEVEN_ZIP), "l", "-slt", str(path)], check=True,
                            capture_output=True, text=True).stdout
    paths = lambda text: sorted(line for line in text.splitlines() if line.startswith("Path = ")
                                and not line.endswith(".7z") and ".7z.tmp" not in line)
    assert paths(listing) == paths(before), (paths(listing), paths(before))
    read_back = subprocess.run([str(SEVEN_ZIP), "e", "-so", str(temp), ENTRY.replace("/", os.sep)],
                               check=True, capture_output=True).stdout
    assert read_back == new
    os.replace(temp, path)
    shutil.rmtree(STAGE)
    print(path.name, path.stat().st_size, "bytes")


rebuild_zip(SOURCES / "egg Midcup Tracer.zip")
rebuild_zip(SOURCES / "egg Midcup Tracer bad notes.zip")
rebuild_7z(SOURCES / "egg Midcup Tracer.7z")
