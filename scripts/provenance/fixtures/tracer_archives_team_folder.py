"""Rebuild the Compiler's tracer archives (`tests/fixtures/sources/egg Midcup Tracer*`) from
their own entries, with two kinds of change:

- each entry in `REPLACED` takes the studio tracer's working copy: the respelled
  `fcl_hair.fmdl` (worklog 0.0.2-poc12: first its `dummy_kit` directory named
  `common/team_folder/`, no `000`; then the stand-in itself `team_kit`, `fmdl_team_kit.rs`).
  The archive's bytes must be the file's at HEAD (the pre-respell bytes) or already the
  working copy's;
- each entry in `ADDED` is appended when the archive lacks it (slice 0.0.2-logo a): the root
  `logo.png`, the `/4ccg/` logo of the Export upgrader's `co Spring` fixture, and the tracer
  player's own `portrait.dds`, so a test renaming an archive `Full` finds both. An entry
  already there must hold the same bytes.

Run it after a respelling of the studio tracer's file, or a change to `ADDED`, before it is
committed; it rebuilds the repository it sits in. Every other entry keeps its name, order,
timestamp, attributes, compression and bytes, and an added zip entry takes the archive's
first entry's timestamp, attributes and Deflate. The bad-notes zip's `notes.txt` keeps its
deliberately wrong CRC-32, patched back into its local header and central directory after the
write. Each archive is written to a sibling temp path, read back, then `os.replace`d.
"""
import os
import shutil
import struct
import subprocess
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "crates/tools/team_compiler/tests/fixtures"
SOURCES = FIXTURES / "sources"
STUDIO = "tracer/studio/egg Midcup Tracer"
PLAYER = "Players/05 - The Chad Stormworks Player"
REPLACED = [f"{PLAYER}/fcl_hair.fmdl"]
ADDED = {
    "logo.png": ROOT
    / "crates/tools/export_upgrader/tests/fixtures/co Spring/Logo/emblem_0XXX_r_ll.png",
    f"{PLAYER}/portrait.dds": FIXTURES / STUDIO / PLAYER / "portrait.dds",
}
SEVEN_ZIP = Path("C:/Program Files/7-Zip/7z.exe")
STAGE = ROOT / ".tmp/tracer_archive_stage"


def at_head(entry):
    """The studio tracer's `entry` as HEAD holds it."""
    return subprocess.run(
        ["git", "show", f"HEAD:crates/tools/team_compiler/tests/fixtures/{STUDIO}/{entry}"],
        cwd=ROOT, check=True, capture_output=True).stdout


replaced = {entry: (at_head(entry), (FIXTURES / STUDIO / entry).read_bytes())
            for entry in REPLACED}
added = {entry: path.read_bytes() for entry, path in ADDED.items()}


def rewritten(name, body):
    """The bytes the archive's entry `name`, holding `body`, is written back with."""
    if name in replaced:
        old, new = replaced[name]
        assert body in (old, new), f"{name} is neither HEAD's nor the working copy's"
        return new
    if name in added:
        assert body == added[name], f"{name} is already in the archive with other bytes"
    return body


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
            entries.append((info, rewritten(info.filename, body)))
    first = infos[0]
    for name, body in added.items():
        if name not in [info.filename for info in infos]:
            info = zipfile.ZipInfo(name, first.date_time)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = first.external_attr
            info.create_system = first.create_system
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
    first = (STAGE / REPLACED[0]).stat()
    for name in replaced:
        staged = STAGE / name
        stat = staged.stat()
        staged.write_bytes(rewritten(name, staged.read_bytes()))
        os.utime(staged, (stat.st_atime, stat.st_mtime))
    for name, body in added.items():
        staged = STAGE / name
        if staged.exists():
            rewritten(name, staged.read_bytes())
            continue
        staged.write_bytes(body)
        # An added file takes a neighbour's time, as the zips' added entries do.
        os.utime(staged, (first.st_atime, first.st_mtime))
    temp = path.with_name(path.name + ".tmp")
    assert not temp.exists()
    tops = sorted(entry.name for entry in STAGE.iterdir())
    subprocess.run([str(SEVEN_ZIP), "a", "-t7z", "-ms=on", "-m0=lzma2", str(temp), *tops],
                   cwd=STAGE, check=True, capture_output=True)
    listing = subprocess.run([str(SEVEN_ZIP), "l", "-slt", str(temp)], check=True,
                             capture_output=True, text=True).stdout
    assert "Solid = +" in listing
    before = subprocess.run([str(SEVEN_ZIP), "l", "-slt", str(path)], check=True,
                            capture_output=True, text=True).stdout
    paths = lambda text: {line for line in text.splitlines() if line.startswith("Path = ")
                          and not line.endswith(".7z") and ".7z.tmp" not in line}
    expected = paths(before) | {"Path = " + name.replace("/", os.sep) for name in added}
    assert paths(listing) == expected, (sorted(paths(listing)), sorted(expected))
    for name, body in [*((n, new) for n, (_, new) in replaced.items()), *added.items()]:
        read_back = subprocess.run([str(SEVEN_ZIP), "e", "-so", str(temp),
                                    name.replace("/", os.sep)],
                                   check=True, capture_output=True).stdout
        assert read_back == body, name
    os.replace(temp, path)
    shutil.rmtree(STAGE)
    print(path.name, path.stat().st_size, "bytes")


rebuild_zip(SOURCES / "egg Midcup Tracer.zip")
rebuild_zip(SOURCES / "egg Midcup Tracer bad notes.zip")
rebuild_7z(SOURCES / "egg Midcup Tracer.7z")
