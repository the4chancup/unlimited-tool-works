"""A checkpoint of our own compile over the library, and the difference between two.

`write` upgrades and compiles every input an upgrade census upgraded clean, as the library run
does (`library.run_ours`), and keeps, per input, one line for every entry of every CPK
written: the CPK's name, the entry's path, its bytes and its SHA-256, a nested CPK's members
(a pre-Fox face package) listed below it as `<entry>.d/<member>`. Red is not run. The
checkpoint is `config.CHECKPOINT_DIR/<tag>/`: `rows.tsv` (index, input path, PES version,
upgrade exit, compile outcome), `<index>.tsv` per input, and `commit.txt`, the commit the
binary was built from, which is what a later diagnosis rebuilds. The run's logs stay under
`config.LIBRARY_DIR/jobs_chk_<tag>/`; the compiled output is removed once it is listed.

`diff` compares two checkpoints input by input, matched by path (an index moves when the
library gains an export): the inputs only one holds, those whose upgrade exit or compile
outcome changed, and every entry added, removed or changed. Each changed entry is then traced
to a commit between the two checkpoints, or is a defect.

The entries are listed by the `cpk` crate's `cpk_list` example, built from the current tree
whatever binary compiles: `cargo build --release -p cpk --example cpk_list`.

Usage: checkpoint.py write <tag> <commit> --census <outcomes_<tag>.tsv> [--bin DIR [--frozen]]
                           [--indexes N...] [--workers W]
       checkpoint.py diff <old tag> <new tag>
"""
import argparse
import hashlib
import os
import queue
import shutil
import subprocess
import threading
from concurrent.futures import ThreadPoolExecutor

import config
import library

CPK_LIST = os.path.join(config.REPO, "target", "release", "examples", "cpk_list.exe")


def entry_lines(out, scratch):
    """One line per entry of every CPK in `out`, read through an extraction into `scratch`."""
    lines = []
    for name in sorted(os.listdir(out)) if os.path.isdir(out) else []:
        if not name.lower().endswith(".cpk"):
            continue
        folder = os.path.join(scratch, name)
        subprocess.run([CPK_LIST, os.path.join(out, name), "", folder], check=True, capture_output=True)
        for root, _dirs, files in os.walk(folder):
            for file in files:
                full = os.path.join(root, file)
                with open(full, "rb") as handle:
                    digest = hashlib.sha256(handle.read()).hexdigest()
                entry = os.path.relpath(full, folder).replace(os.sep, "/")
                lines.append(f"{name}\t{entry}\t{os.path.getsize(full)}\t{digest}")
    return sorted(lines)


def write_one(index, path, exe, worker, target):
    up_code, version, outcome, _seconds, _cpks = library.run_ours(index, path, exe, worker)
    job = os.path.join(library.JOBS, str(index))
    out, scratch = os.path.join(job, "out"), os.path.join(job, "entries")
    lines = entry_lines(out, scratch) if version is not None else []
    with open(os.path.join(target, f"{index}.tsv"), "w", encoding="utf-8", newline="\n") as handle:
        handle.write("".join(line + "\n" for line in lines))
    for folder in (os.path.join(job, "up"), out, scratch):
        library.remove_scratch(folder)
    return (index, path, version or "-", up_code, outcome, len(lines))


def write(args):
    target = os.path.join(config.CHECKPOINT_DIR, args.tag)
    library.JOBS = os.path.join(config.LIBRARY_DIR, f"jobs_chk_{args.tag}")
    library.BIN = os.path.join(config.LIBRARY_DIR, args.bin)
    library.CENSUS = args.census
    assert os.path.isfile(CPK_LIST), f"{CPK_LIST} is not built"
    os.makedirs(target, exist_ok=True)
    with open(os.path.join(target, "commit.txt"), "w", encoding="utf-8", newline="\n") as handle:
        handle.write(args.commit + "\n")
    if args.frozen:
        exe = os.path.join(library.BIN, "utw.exe")
        assert os.path.isfile(exe), f"{library.BIN} holds no frozen binary"
    else:
        exe, _parity = library.freeze_binaries()
    selection = library.inputs()
    indexes = args.indexes if args.indexes is not None else list(range(len(selection)))
    print(len(selection), "clean upgrades,", len(indexes), "in this run", flush=True)
    for version in (config.PREFOX_VERSION, config.FOX_VERSION):
        for worker in range(args.workers):
            library.prepare_bin(exe, worker, version)
    free = queue.Queue()
    for worker in range(args.workers):
        free.put(worker)
    stopped = threading.Event()

    def with_worker(index):
        if stopped.is_set():
            return None
        if shutil.disk_usage(config.LIBRARY_DIR).free < library.FLOOR_BYTES:
            stopped.set()
            print("stopping: under the 10 GB floor before input", index, flush=True)
            return None
        worker = free.get()
        try:
            return write_one(index, selection[index], exe, worker, target)
        finally:
            free.put(worker)

    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for row in pool.map(with_worker, indexes):
            if row is None:
                continue
            with open(os.path.join(target, "rows.tsv"), "a", encoding="utf-8", newline="\n") as handle:
                handle.write("\t".join(str(field) for field in row) + "\n")
            print(row[0], row[2], row[4][:100], "|", row[5], "entries", flush=True)


def read_rows(tag):
    """A checkpoint's rows by input path: (index, version, upgrade exit, compile outcome)."""
    rows = {}
    for line in open(os.path.join(config.CHECKPOINT_DIR, tag, "rows.tsv"), encoding="utf-8"):
        index, path, version, up_code, outcome, _count = line.rstrip("\n").split("\t")
        assert path not in rows, f"{path} twice in {tag}"
        rows[path] = (index, version, up_code, outcome)
    return rows


def read_entries(tag, index):
    """One input's entries: (CPK name, entry path) to (bytes, SHA-256)."""
    entries = {}
    for line in open(os.path.join(config.CHECKPOINT_DIR, tag, f"{index}.tsv"), encoding="utf-8"):
        cpk, entry, size, digest = line.rstrip("\n").split("\t")
        entries[(cpk, entry)] = (size, digest)
    return entries


def entry_changes(old, new):
    """The lines saying how `new`'s entries differ from `old`'s: `-` removed, `+` added, `~`
    changed (with both sizes), in the entries' order."""
    lines = []
    for key in sorted(old.keys() | new.keys()):
        name = "\t".join(key)
        if key not in new:
            lines.append(f"- {name}")
        elif key not in old:
            lines.append(f"+ {name}")
        elif old[key] != new[key]:
            lines.append(f"~ {name}\t{old[key][0]} -> {new[key][0]} bytes")
    return lines


def diff(args):
    old_rows, new_rows = read_rows(args.old), read_rows(args.new)
    report = []
    for label, only in (("only in " + args.old, old_rows.keys() - new_rows.keys()),
                        ("only in " + args.new, new_rows.keys() - old_rows.keys())):
        report += [f"{label}: {path}" for path in sorted(only)]
    changed_inputs = changed_entries = 0
    for path in sorted(old_rows.keys() & new_rows.keys()):
        old, new = old_rows[path], new_rows[path]
        lines = [f"  {what}: {before} -> {after}"
                 for what, before, after in zip(("version", "upgrade exit", "compile"), old[1:], new[1:])
                 if before != after]
        lines += ["  " + line for line in entry_changes(read_entries(args.old, old[0]), read_entries(args.new, new[0]))]
        if lines:
            changed_inputs += 1
            changed_entries += sum(line[2] in "+-~" for line in lines)
            report += [f"{path} ({args.old} {old[0]}, {args.new} {new[0]})"] + lines
    summary = (f"{len(old_rows.keys() & new_rows.keys())} inputs in both, {changed_inputs} differ, "
               f"{changed_entries} entries added, removed or changed")
    target = os.path.join(config.CHECKPOINT_DIR, f"diff_{args.old}_{args.new}.txt")
    with open(target, "w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join([summary] + report) + "\n")
    print(summary)
    print(target)


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    write_args = commands.add_parser("write")
    write_args.add_argument("tag")
    write_args.add_argument("commit", help="the commit the compiling binary was built from")
    write_args.add_argument("--census", required=True, help="the upgrade census's outcomes file, whose exit-0 rows are the inputs")
    write_args.add_argument("--bin", default="bin", help="the frozen binaries' folder, under config.LIBRARY_DIR")
    write_args.add_argument("--frozen", action="store_true", help="run the --bin folder's binary as it is, even when older than the build")
    write_args.add_argument("--indexes", type=int, nargs="*", default=None)
    write_args.add_argument("--workers", type=int, default=3)
    diff_args = commands.add_parser("diff")
    diff_args.add_argument("old")
    diff_args.add_argument("new")
    args = parser.parse_args()
    {"write": write, "diff": diff}[args.command](args)


if __name__ == "__main__":
    main()
