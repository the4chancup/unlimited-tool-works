"""Named queries over the census index `index.py` writes. Each prints its count, a control count
that shows the index holds comparable files, the index's age and what its walk skipped or could
not read, and up to N examples; a count is never printed alone. Regular expressions match
case-insensitively (`re.search`, Python syntax). README.md lists the queries with examples.

Usage: query.py <query> [arguments] [--index FILE] [--examples N]
  fmdl-dirs REGEX [--except REGEX] [--in FOLDER]   FMDLs naming a texture directory that matches
  fmdl-names REGEX [--except REGEX] [--in FOLDER]  the same over texture file names
  textures [predicates] [--kind dds|ftex] [--name REGEX] [--except-name REGEX] [--in FOLDER]
  sql "SELECT ..."                                 anything else, rows as they come
Tests: `python scripts/census/census_test.py`.
"""
import argparse
import datetime
import json
import os
import re
import sqlite3
import time
from collections import Counter
from pathlib import Path

import index as census


def scope(within, kind=None):
    """The WHERE clause and parameters limiting `files f` to a kind and to files inside a
    folder named `within` (any case, at any depth below the root)."""
    clauses, params = ["1"], []
    if within:
        clauses.append("instr(f.folders, ?) > 0")
        params.append("/" + within.lower() + "/")
    if kind:
        clauses.append("f.kind = ?")
        params.append(kind)
    return " AND ".join(clauses), params


def changed_folders(db):
    """Indexed folders whose modification time differs from the walk's, or that are gone: a
    file added, removed or renamed in them since. A file rewritten in place changes no folder."""
    changed = []
    for path, mtime in db.execute("SELECT path, mtime FROM folders"):
        try:
            if os.stat(path).st_mtime != mtime:
                changed.append(path)
        except OSError:
            changed.append(path)
    return changed


def rerun_command(meta, index_path):
    command = "python scripts/census/index.py"
    if Path(meta["roots_file"]).resolve() != (census.CENSUS / "roots.txt").resolve():
        command += f" --roots {meta['roots_file']}"
    if index_path.resolve() != (census.CENSUS / "index.sqlite").resolve():
        command += f" --index {index_path}"
    if meta["keep_repositories"] == "1":
        command += " --keep-repositories"
    return command


def describe(db, index_path):
    """Prints the index's age, roots, skips, unopened archives and unreadable files, then
    whether the roots changed since."""
    meta = dict(db.execute("SELECT key, value FROM meta"))
    built = float(meta["built"])
    roots = json.loads(meta["roots"])
    hours = (time.time() - built) / 3600
    print(f"index {index_path}: built {datetime.datetime.fromtimestamp(built):%Y-%m-%d %H:%M} "
          f"({hours:.1f} h ago) in {meta['seconds']} s over {', '.join(roots)}")
    skips = dict(db.execute("SELECT rule, folders FROM skips"))
    archives = dict(db.execute("SELECT ext, count(*) FROM files WHERE ext IN ('.zip', '.7z') GROUP BY ext"))
    bad_textures = db.execute("SELECT count(error) FROM textures").fetchone()[0]
    bad_fmdls = db.execute("SELECT count(error) FROM fmdls").fetchone()[0]
    repositories = "walked" if meta["keep_repositories"] == "1" else f"{skips['repository']:,} skipped"
    print(f"skipped folders: target {skips['target']:,}, .git {skips['.git']:,}, "
          f"node_modules {skips['node_modules']:,}, unreadable {skips['unreadable']:,}; repositories {repositories}; "
          f"archives not opened: {archives.get('.zip', 0):,} .zip, {archives.get('.7z', 0):,} .7z; "
          f"unreadable files: {bad_textures:,} textures, {bad_fmdls:,} FMDLs")
    current_roots = census.read_roots(Path(meta["roots_file"])) if Path(meta["roots_file"]).exists() else None
    changed = changed_folders(db)
    if current_roots == roots and not changed:
        print("current: no indexed folder changed since the walk")
        return
    reason = "the roots file changed" if current_roots != roots else \
        f"{len(changed):,} folders changed since the walk (first: {changed[0]})"
    print(f"STALE: {reason}; rerun: {rerun_command(meta, index_path)}")


def tally(title, values, limit):
    """Prints the `limit` most frequent values of a Counter, with their counts."""
    if values:
        print(f"{title}:")
        for value, count in values.most_common(limit):
            print(f"  {count:>7,}  {value}")


def fmdl_query(db, column, args):
    """FMDLs naming a texture `column` (directory or file name) that matches `args.pattern`;
    a value matching `args.exception` too is not counted, and the FMDLs naming such a value are
    the control."""
    noun = "directory" if column == "directory" else "file name"
    pattern = re.compile(args.pattern, re.I)
    exception = re.compile(args.exception, re.I) if args.exception else None
    where, params = scope(args.within)
    read, unreadable, without = db.execute(
        f"SELECT count(*) - count(m.error), count(m.error), sum(m.error IS NULL AND m.textures = 0) "
        f"FROM fmdls m JOIN files f ON f.id = m.file_id WHERE {where}", params).fetchone()
    counted, removed = {}, {}
    for path, value in db.execute(
            f"SELECT f.path, t.{column} FROM fmdl_textures t JOIN files f ON f.id = t.file_id WHERE {where}", params):
        if pattern.search(value):
            target = removed if exception and exception.search(value) else counted
            target.setdefault(path, set()).add(value)
    limit = f" inside a folder named {args.within}" if args.within else ""
    excepting = f" (except {args.exception!r})" if exception else ""
    print(f"COUNT: {len(counted):,} FMDLs{limit} name a texture {noun} matching {args.pattern!r}{excepting}")
    if exception:
        print(f"control: {len(removed):,} FMDLs{limit} name a matching {noun} the exception removed")
    print(f"scope: {read:,} FMDLs{limit} read, {without or 0:,} of them naming no texture; {unreadable:,} unreadable")
    tally("counted values (FMDLs naming each)", Counter(v for values in counted.values() for v in values), args.examples)
    tally("removed values (FMDLs naming each)", Counter(v for values in removed.values() for v in values), args.examples)
    examples("examples", sorted(counted), args.examples)


def layout(header, fourcc, dxgi, ftex_format, bit_count):
    return {"masks": f"masks {bit_count}-bit", "fourcc": f"FourCC {fourcc}", "dx10": f"DX10 DXGI {dxgi}",
            "ftex": f"FTEX format {ftex_format}"}[header]


def texture_predicates(args):
    """The (description, test on (width, height, header, uncompressed)) pairs the arguments ask."""
    predicates = []
    if args.uncompressed:
        predicates.append(("uncompressed", lambda w, h, header, flat: flat == 1))
    if args.compressed:
        predicates.append(("block-compressed", lambda w, h, header, flat: flat == 0))
    if args.odd_side:
        predicates.append(("a side no multiple of 4", lambda w, h, header, flat: w % 4 or h % 4))
    if args.not_pow2:
        predicates.append(("a side no power of two", lambda w, h, header, flat: w & (w - 1) or h & (h - 1)))
    if args.side_over is not None:
        predicates.append((f"a side over {args.side_over}", lambda w, h, header, flat: max(w, h) > args.side_over))
    if args.header:
        predicates.append((f"a {args.header} header", lambda w, h, header, flat: header == args.header))
    return predicates


def texture_query(db, args):
    """Textures passing every predicate; a file name matching `args.except_name` is not counted,
    and those it removed are the control."""
    predicates = texture_predicates(args)
    name = re.compile(args.name, re.I) if args.name else None
    exception = re.compile(args.except_name, re.I) if args.except_name else None
    where, params = scope(args.within, args.kind)
    read = unreadable = 0
    counted, removed, layouts = [], [], Counter()
    for path, width, height, header, fourcc, dxgi, ftex_format, bit_count, flat, error in db.execute(
            "SELECT f.path, t.width, t.height, t.header, t.fourcc, t.dxgi, t.ftex_format, t.bit_count, "
            f"t.uncompressed, t.error FROM textures t JOIN files f ON f.id = t.file_id WHERE {where}", params):
        file_name = path.rsplit("/", 1)[-1]
        if name and not name.search(file_name):
            continue
        if error:
            unreadable += 1
            continue
        read += 1
        if not all(test(width, height, header, flat) for _, test in predicates):
            continue
        if exception and exception.search(file_name):
            removed.append(path)
            continue
        described = layout(header, fourcc, dxgi, ftex_format, bit_count)
        counted.append(f"{width}x{height} {described}  {path}")
        layouts[described] += 1
    kind = f".{args.kind}" if args.kind else ".dds/.ftex"
    limit = "".join([f" named {args.name!r}" if name else "",
                     f" inside a folder named {args.within}" if args.within else ""])
    asked = ", ".join(description for description, _ in predicates) or "any"
    excepting = f" (except names {args.except_name!r})" if exception else ""
    print(f"COUNT: {len(counted):,} {kind} textures{limit} with {asked}{excepting}")
    if exception:
        print(f"control: {len(removed):,} more with {asked} that the name exception removed")
    print(f"scope: {read:,} {kind} textures{limit} read; {unreadable:,} unreadable")
    tally("layouts counted", layouts, args.examples)
    examples("examples", counted, args.examples)


def sql_query(db, args):
    cursor = db.execute(args.statement)
    rows = cursor.fetchall()
    print(f"COUNT: {len(rows):,} rows")
    sizes = {table: db.execute(f"SELECT count(*) FROM {table}").fetchone()[0]
             for table in ("files", "textures", "fmdls", "fmdl_textures")}
    print("control: the index holds " + ", ".join(f"{count:,} {table}" for table, count in sizes.items()))
    print("columns: " + " | ".join(column[0] for column in cursor.description))
    examples("rows", [" | ".join(str(value) for value in row) for row in rows], args.examples)


def examples(title, lines, limit):
    if lines:
        print(f"{title} ({min(limit, len(lines))} of {len(lines):,}):")
        for line in lines[:limit]:
            print(f"  {line}")


def parser():
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--index", type=Path, default=census.CENSUS / "index.sqlite")
    common.add_argument("--examples", type=int, default=10, help="examples and tally lines shown")
    top = argparse.ArgumentParser(description="Named queries over the census index.")
    queries = top.add_subparsers(dest="query", required=True)
    for query, noun in (("fmdl-dirs", "directory"), ("fmdl-names", "file name")):
        sub = queries.add_parser(query, parents=[common], help=f"FMDLs naming a texture {noun} that matches")
        sub.add_argument("pattern")
        sub.add_argument("--except", dest="exception", help=f"a matching {noun} matching this too is not counted")
        sub.add_argument("--in", dest="within", metavar="FOLDER", help="only FMDLs inside a folder of this name")
    textures = queries.add_parser("textures", parents=[common], help="textures by predicate")
    flat = textures.add_mutually_exclusive_group()
    flat.add_argument("--uncompressed", action="store_true")
    flat.add_argument("--compressed", action="store_true")
    textures.add_argument("--odd-side", action="store_true", help="a side no multiple of 4")
    textures.add_argument("--not-pow2", action="store_true", help="a side no power of two")
    textures.add_argument("--side-over", type=int, metavar="PIXELS")
    textures.add_argument("--header", choices=("masks", "fourcc", "dx10", "ftex"))
    textures.add_argument("--kind", choices=("dds", "ftex"))
    textures.add_argument("--name", help="only file names matching this")
    textures.add_argument("--except-name", help="file names matching this are not counted")
    textures.add_argument("--in", dest="within", metavar="FOLDER")
    sql = queries.add_parser("sql", parents=[common], help="a free SELECT over the index's tables")
    sql.add_argument("statement")
    return top


def main(argv=None):
    args = parser().parse_args(argv)
    if not args.index.exists():
        raise SystemExit(f"no index at {args.index}: run python scripts/census/index.py first")
    db = sqlite3.connect(args.index.resolve().as_uri() + "?mode=ro", uri=True)
    describe(db, args.index)
    if args.query == "textures":
        texture_query(db, args)
    elif args.query == "sql":
        sql_query(db, args)
    else:
        fmdl_query(db, "directory" if args.query == "fmdl-dirs" else "name", args)
    db.close()


if __name__ == "__main__":
    main()
