"""Walks the census roots and writes a kept SQLite index of their files: every file's size,
modification time and folder names, every `.dds`/`.ftex` texture's header, and every `.fmdl`'s
texture references. `query.py` answers questions from it in seconds; README.md says what the
index holds and what it does not.

Usage: index.py [--roots FILE] [--index FILE] [--keep-repositories]
Roots come from `.census/roots.txt` (one path per line, `#` comments), written with `C:/Data/4cc`
when missing. A rerun re-reads only files whose size or modification time changed and drops
the rows of files that are gone. `--keep-repositories` walks into folders holding a `.git`
entry too (the comparison with walks that skip only folders by name).
Tests: `python scripts/census/census_test.py`.
"""
import argparse
import json
import os
import shutil
import sqlite3
import struct
import time
import zlib
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CENSUS = ROOT / ".census"
DEFAULT_ROOTS = "C:/Data/4cc"
SKIPPED_NAMES = ("target", ".git", "node_modules")
KINDS = {".dds": "dds", ".ftex": "ftex", ".fmdl": "fmdl", ".model": "model", ".mtl": "mtl", ".xml": "xml"}

# Block-compressed layouts: the legacy FourCCs and the DXGI ranges of BC1 to BC7, typeless,
# UNORM, sRGB and signed variants alike. Any other layout stores pixels.
BLOCK_FOURCCS = {b"DXT1", b"DXT2", b"DXT3", b"DXT4", b"DXT5", b"ATI1", b"ATI2", b"BC4U", b"BC4S", b"BC5U", b"BC5S"}
BLOCK_DXGI = set(range(70, 85)) | set(range(94, 100))
BLOCK_FTEX_FORMATS = {2, 3, 4, 8, 9, 10, 11}

SCHEMA = """
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS skips (rule TEXT PRIMARY KEY, folders INTEGER);
CREATE TABLE IF NOT EXISTS folders (path TEXT PRIMARY KEY, mtime REAL);
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY, path TEXT UNIQUE, root TEXT, size INTEGER, mtime REAL,
    ext TEXT, kind TEXT, folders TEXT);
CREATE TABLE IF NOT EXISTS textures (
    file_id INTEGER PRIMARY KEY REFERENCES files ON DELETE CASCADE,
    wesys INTEGER, width INTEGER, height INTEGER, levels INTEGER, header TEXT, fourcc TEXT,
    dxgi INTEGER, ftex_format INTEGER, bit_count INTEGER, uncompressed INTEGER, error TEXT);
CREATE TABLE IF NOT EXISTS fmdls (
    file_id INTEGER PRIMARY KEY REFERENCES files ON DELETE CASCADE, textures INTEGER, error TEXT);
CREATE TABLE IF NOT EXISTS fmdl_textures (
    file_id INTEGER REFERENCES files ON DELETE CASCADE, directory TEXT, name TEXT);
CREATE INDEX IF NOT EXISTS fmdl_textures_file ON fmdl_textures (file_id);
"""


def read_roots(path):
    """The roots file's paths; a missing file is written with the default root first."""
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        write_text(path, "# The folders the census index walks, one per line.\n" + DEFAULT_ROOTS + "\n")
        print(f"no roots file: wrote {path} holding {DEFAULT_ROOTS}")
    lines = path.read_text(encoding="utf-8").splitlines()
    return [line.strip().replace("\\", "/") for line in lines if line.strip() and not line.strip().startswith("#")]


def write_text(path, text):
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)
    os.replace(tmp, path)


def walk(root, keep_repositories, skips, folders):
    """Yields (path, size, mtime, folder names) for every file under `root`, lowercased folder
    names relative to the root written `/a/b/`. Records each walked folder's mtime in `folders`
    and counts each skip rule's dropped folders in `skips`. The root itself is walked whatever
    it holds: naming it is asking for it."""
    stack = [(root, "/")]
    while stack:
        folder, names = stack.pop()
        # The folder's own stat, not its parent's listing: NTFS updates a folder's time in the
        # parent's listing lazily. Taken before the listing, so a change during the walk
        # reads as stale rather than as current.
        try:
            mtime = os.stat(folder).st_mtime
            entries = list(os.scandir(folder))
        except OSError:
            skips["unreadable"] += 1
            continue
        if folder != root and not keep_repositories and any(entry.name == ".git" for entry in entries):
            skips["repository"] += 1
            continue
        folders[folder.replace("\\", "/")] = mtime
        for entry in entries:
            # A junction is no symlink to Python: followed, it lists its target's files a second
            # time under its own path, and one pointing at a folder above it never ends.
            if entry.is_symlink() or entry.is_junction():
                skips["links"] += 1
                continue
            if entry.is_dir():
                if entry.name in SKIPPED_NAMES:
                    skips[entry.name] += 1
                else:
                    stack.append((entry.path, names + entry.name.lower() + "/"))
            elif entry.is_file():
                # Gone since the listing: its folder's time changed with it, so the index reads as stale.
                try:
                    stat = entry.stat()
                except OSError:
                    continue
                yield entry.path.replace("\\", "/"), stat.st_size, stat.st_mtime, names


def unwrap_wesys(data, limit=None):
    """`data` without PES's WESYS zlib wrapper (16-byte header, `WESYS` at offset 3), inflated
    up to `limit` bytes; unwrapped data is returned as it is. Second value: whether it was wrapped."""
    if data[3:8] != b"WESYS":
        return data, False
    stream = data[16:]
    return (zlib.decompress(stream) if limit is None else zlib.decompressobj().decompress(stream, limit)), True


def read_texture(path, kind):
    """The textures row's columns for a `.dds` or `.ftex` file, read from its header alone."""
    with open(path, "rb") as handle:
        data, wesys = unwrap_wesys(handle.read(4096), 148)
    row = {"wesys": int(wesys)}
    if kind == "ftex":
        if len(data) < 17 or data[:4] != b"FTEX":
            raise ValueError("no FTEX header")
        fmt, width, height, _depth, levels = struct.unpack_from("<4HB", data, 8)
        return row | {"width": width, "height": height, "levels": levels, "header": "ftex",
                      "ftex_format": fmt, "uncompressed": int(fmt not in BLOCK_FTEX_FORMATS)}
    if len(data) < 128 or data[:4] != b"DDS ":
        raise ValueError("no DDS header")
    height, width = struct.unpack_from("<2I", data, 12)
    levels = max(1, struct.unpack_from("<I", data, 28)[0])
    flags, fourcc, bit_count = struct.unpack_from("<I4sI", data, 80)
    row |= {"width": width, "height": height, "levels": levels}
    if not flags & 0x4:
        return row | {"header": "masks", "bit_count": bit_count, "uncompressed": 1}
    if fourcc != b"DX10":
        return row | {"header": "fourcc", "fourcc": fourcc.decode("latin-1"),
                      "uncompressed": int(fourcc not in BLOCK_FOURCCS)}
    if len(data) < 148:
        raise ValueError("DX10 header cut short")
    dxgi = struct.unpack_from("<I", data, 128)[0]
    return row | {"header": "dx10", "dxgi": dxgi, "uncompressed": int(dxgi not in BLOCK_DXGI)}


def read_fmdl_textures(path):
    """Every texture reference of an FMDL as (directory, file name), read through its container:
    section-0 block 6 holds (file name, directory) string ids, block 12 each string's section-1
    block, length and offset."""
    data, _ = unwrap_wesys(Path(path).read_bytes())
    if len(data) < 64 or data[:4] != b"FMDL":
        raise ValueError("no FMDL header")
    descriptors, count0, count1, offset0, _length0, offset1 = struct.unpack_from("<Q16x5I", data, 8)
    section0 = {}
    for index in range(count0):
        block, entries, offset = struct.unpack_from("<2HI", data, descriptors + 8 * index)
        section0[block] = (entries, offset0 + offset)
    section1 = {}
    for index in range(count1):
        block, offset, length = struct.unpack_from("<3I", data, descriptors + 8 * count0 + 12 * index)
        start = offset1 + offset
        # The string block runs to the file end whatever its stored length says.
        section1[block] = data[start:] if block == 3 else data[start:start + length]
    if 6 not in section0:
        return []
    if 12 not in section0:
        raise ValueError("texture references but no string table")
    strings_count, strings_at = section0[12]

    def string(index):
        if index >= strings_count:
            raise ValueError(f"string {index} past the table")
        block, length, offset = struct.unpack_from("<2HI", data, strings_at + 8 * index)
        if block not in section1 or offset + length > len(section1[block]):
            raise ValueError(f"string {index} outside its block")
        return section1[block][offset:offset + length].decode("utf-8")

    entries, at = section0[6]
    references = []
    for index in range(entries):
        name_id, directory_id = struct.unpack_from("<2H", data, at + 4 * index)
        references.append((string(directory_id), string(name_id)))
    return references


def index_file(db, path, root, size, mtime, names):
    ext = os.path.splitext(path)[1].lower()
    kind = KINDS.get(ext, "other")
    file_id = db.execute(
        "INSERT INTO files (path, root, size, mtime, ext, kind, folders) VALUES (?, ?, ?, ?, ?, ?, ?)",
        (path, root, size, mtime, ext, kind, names),
    ).lastrowid
    if kind in ("dds", "ftex"):
        try:
            row = read_texture(path, kind)
        except (OSError, ValueError, struct.error, zlib.error) as error:
            row = {"error": f"{type(error).__name__}: {error}"}
        columns = ", ".join(["file_id", *row])
        db.execute(f"INSERT INTO textures ({columns}) VALUES ({', '.join('?' * (len(row) + 1))})",
                   (file_id, *row.values()))
    elif kind == "fmdl":
        try:
            references = read_fmdl_textures(path)
            error = None
        except (OSError, ValueError, struct.error, zlib.error) as caught:
            references, error = [], f"{type(caught).__name__}: {caught}"
        db.execute("INSERT INTO fmdls VALUES (?, ?, ?)", (file_id, len(references), error))
        db.executemany("INSERT INTO fmdl_textures VALUES (?, ?, ?)",
                       [(file_id, directory, name) for directory, name in references])


def build(roots_path, index_path, keep_repositories):
    """Brings the index at `index_path` up to date with the roots' files; returns a summary."""
    roots = read_roots(roots_path)
    for root in roots:
        if not os.path.isdir(root):
            raise SystemExit(f"{roots_path} names {root}, which is no folder")
    started = time.time()
    tmp = index_path.with_name(index_path.name + ".tmp")
    index_path.parent.mkdir(parents=True, exist_ok=True)
    # The update runs on a copy, so an interrupted run leaves the last index whole.
    if index_path.exists():
        shutil.copyfile(index_path, tmp)
    elif tmp.exists():
        tmp.unlink()
    db = sqlite3.connect(tmp)
    db.execute("PRAGMA foreign_keys = ON")
    db.executescript(SCHEMA)
    known = {path: (file_id, size, mtime, root)
             for file_id, path, size, mtime, root in db.execute("SELECT id, path, size, mtime, root FROM files")}
    skips, folders, seen = Counter(), {}, set()
    reread = 0
    for root in roots:
        for path, size, mtime, names in walk(root, keep_repositories, skips, folders):
            # A root inside another root lists its files twice; the first root keeps them.
            if path in seen:
                continue
            seen.add(path)
            old = known.get(path)
            if old and old[1:] == (size, mtime, root):
                continue
            if old:
                db.execute("DELETE FROM files WHERE id = ?", (old[0],))
            index_file(db, path, root, size, mtime, names)
            reread += 1
    gone = [(known[path][0],) for path in known.keys() - seen]
    db.executemany("DELETE FROM files WHERE id = ?", gone)
    db.execute("DELETE FROM folders")
    db.executemany("INSERT INTO folders VALUES (?, ?)", folders.items())
    db.execute("DELETE FROM skips")
    db.executemany("INSERT INTO skips VALUES (?, ?)",
                   [(rule, skips[rule]) for rule in (*SKIPPED_NAMES, "repository", "unreadable", "links")])
    meta = {"built": started, "seconds": round(time.time() - started, 1), "roots": json.dumps(roots),
            "roots_file": str(roots_path), "keep_repositories": int(keep_repositories)}
    db.execute("DELETE FROM meta")
    db.executemany("INSERT INTO meta VALUES (?, ?)", meta.items())
    db.commit()
    db.close()
    os.replace(tmp, index_path)
    return {"files": len(seen), "reread": reread, "dropped": len(gone), "seconds": meta["seconds"],
            "skips": dict(skips)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--roots", type=Path, default=CENSUS / "roots.txt")
    parser.add_argument("--index", type=Path, default=CENSUS / "index.sqlite")
    parser.add_argument("--keep-repositories", action="store_true",
                        help="walk into folders holding a .git entry too")
    args = parser.parse_args()
    summary = build(args.roots, args.index, args.keep_repositories)
    print(f"{args.index}: {summary['files']:,} files, {summary['reread']:,} read, "
          f"{summary['dropped']:,} dropped as gone, in {summary['seconds']} s; "
          f"skipped folders {summary['skips']}")


if __name__ == "__main__":
    main()
