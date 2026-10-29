"""The library run against Red: every input an upgrade census upgraded clean (exit 0 in the
`--census` outcomes file; index `n` is its position among those rows) is upgraded again with
the frozen release binary (`--loose` unless a referee export), the upgraded export compiled
with `team-compiler compile --no-deploy` (`config.FOX_VERSION` for a Fox export,
`config.PREFOX_VERSION` for a pre-Fox one, the cups' versions), the old export compiled by
Red (`config.RED` at `config.RED_COMMIT`) from a fresh copy of its checkout for the same
version, and the two outputs compared by `red_parity` into `jobs/<n>/parity.tsv`. Everything
is written under `config.LIBRARY_DIR`.

Disk is bounded per export: `jobs/<n>/` keeps the two commands' logs, Red's run output and
logs (`red_logs/`), both CPK sets' hash manifests (`manifest_red.tsv`, `manifest_ours.tsv`:
name, bytes, sha256), `ids.tsv` and `parity.tsv`; the Red copy (`red/<n>/`), `up/` and `out/`
are removed once the row is written (`remove_scratch`: only those paths, created by this run).
No job starts with C: under 10 GB free.

Rows go to `rows_<tag>.tsv`, the pilot's columns: n, path, version, Red exit, Red seconds,
Red's CPKs, upgrade exit, compile outcome, compile seconds, parity classes as class=count
joined by ';'. `tally.py` reads them.

Usage: library.py <tag> --census <outcomes_<tag>.tsv> [--indexes N...] [--offset K] [--limit N]
                  [--workers W] [--jobs DIR] [--bin DIR [--frozen]] [--keep]
"""
import argparse
import hashlib
import os
import queue
import re
import shutil
import stat
import subprocess
import threading
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor

import config

HERE = config.LIBRARY_DIR
BUILT_EXE = os.path.join(config.REPO, "target", "release", "utw.exe")
BUILT_PARITY = os.path.join(config.REPO, "target", "release", "examples", "red_parity.exe")
BIN = os.path.join(HERE, "bin")
JOBS = os.path.join(HERE, "jobs")
RED_COPIES = os.path.join(HERE, "red")
CENSUS = None
RED = config.RED
RED_COMMIT = config.RED_COMMIT
PREFOX_VERSION = config.PREFOX_VERSION
FOX_VERSION = config.FOX_VERSION
FLOOR_BYTES = 10 * 10**9
LINE = re.compile(r"^- (?:(.*?): )?(Error|Warning|Info) (\S+)")
WEARER = re.compile(r"wearer_assumed at (Boots|Gloves)/([kg]\d+)[^(]*\(id=(\d+), slot=(\d+)\)")
TEAM = re.compile(r"export_identified \(team=[^,]*, id=(\d+)\)")


def inputs():
    rows = []
    for line in open(CENSUS, encoding="utf-8"):
        path, code = line.rstrip("\n").split("\t")[:2]
        if code == "0":
            rows.append(path)
    return rows


def is_referee(path):
    base = os.path.basename(path)
    if base.lower().endswith((".zip", ".7z")):
        base = os.path.splitext(base)[0]
    return base.split(" ")[0].lower() == "refs"


def check_red():
    def git(*args):
        return subprocess.run(["git", "-C", RED, *args], check=True, capture_output=True, text=True).stdout.strip()

    assert git("branch", "--show-current") == "main"
    assert git("rev-parse", "--short", "HEAD") == RED_COMMIT
    assert not git("status", "--porcelain", "--untracked-files=no")


def freeze_binaries():
    """Copies the built binaries under `bin/`, so a rebuild during the run changes nothing."""
    os.makedirs(BIN, exist_ok=True)
    for built in (BUILT_EXE, BUILT_PARITY):
        target = os.path.join(BIN, os.path.basename(built))
        assert not os.path.exists(target) or os.path.getmtime(target) >= os.path.getmtime(built), (
            f"{target} is older than the build: remove bin/ by hand for a new binary")
        if not os.path.exists(target):
            shutil.copy2(built, target)
    return os.path.join(BIN, "utw.exe"), os.path.join(BIN, "red_parity.exe")


def prepare_bin(exe, worker, version):
    """A portable copy of the binary for one worker and version; its settings name a PES folder
    and a savefile that do not exist, so nothing of the maintainer's is read or written."""
    root = os.path.join(BIN, f"{version}_{worker}")
    os.makedirs(os.path.join(root, "data"), exist_ok=True)
    target = os.path.join(root, "utw.exe")
    if not os.path.exists(target):
        shutil.copy2(exe, target)
    output = os.path.join(root, "output")
    settings = (
        "[common]\n"
        f"pes_version = {version}\n"
        f"pes_folder_path = '{os.path.join(root, 'nonexistent_pes')}'\n"
        "[team-compiler]\n"
        f"output_folder_path = '{output}'\n"
        f"savefile_path = '{os.path.join(root, 'save', 'EDIT00000000')}'\n"
    )
    with open(os.path.join(root, "data", "settings.toml"), "w", encoding="utf-8", newline="\n") as handle:
        handle.write(settings)
    return target, output


def run(command, log_path, cwd=None):
    started = time.monotonic()
    try:
        completed = subprocess.run(command, capture_output=True, timeout=3600, cwd=cwd)
        code = completed.returncode
        output = (completed.stdout + completed.stderr).decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        code, output = "timeout", ""
    seconds = round(time.monotonic() - started, 1)
    with open(log_path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(" ".join(command) + f"\nexit {code} in {seconds}s\n\n{output}")
    return code, output, seconds


def outcome_of(code, output):
    serious = set()
    for line in output.splitlines():
        match = LINE.match(line)
        if match and match.group(2) in ("Error", "Warning"):
            serious.add(match.group(3))
    return f"exit {code}: " + (",".join(sorted(serious)) or "-")


def has_model(folder):
    return any(name.lower().endswith(".model") for _root, _dirs, names in os.walk(folder) for name in names)


def manifest(folder, path):
    lines = []
    if os.path.isdir(folder):
        for name in sorted(os.listdir(folder)):
            file = os.path.join(folder, name)
            if name.lower().endswith(".cpk") and os.path.isfile(file):
                digest = hashlib.sha256()
                with open(file, "rb") as handle:
                    for chunk in iter(lambda: handle.read(1 << 20), b""):
                        digest.update(chunk)
                lines.append(f"{name}\t{os.path.getsize(file)}\t{digest.hexdigest()}")
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write("".join(line + "\n" for line in lines))
    return [line.split("\t")[0] for line in lines]


def setup_red(target, version, export_path):
    """A copy of Red configured for `version` with the old export staged, as
    `scripts/provenance/tracer_refs/setup_red.py` does."""
    assert not os.path.exists(target), target
    shutil.copytree(RED, target, ignore=shutil.ignore_patterns(
        ".git", "patches_output", "patches_contents", "exports_to_add", "extracted_teams",
        "extracted_referees", "temp", "*.log"))
    settings = os.path.join(target, "settings.ini")
    text = open(settings, encoding="utf-8").read()
    replacements = {
        "pes_version = 15": f"pes_version = {version}",
        "cpk_name = 4cc_90_test": "cpk_name = 4cc_90_tracer",
        "move_cpks = 1": "move_cpks = 0",
        r"pes_folder_path = C:\Program Files (x86)\Pro Evolution Soccer 20**": r"pes_folder_path = C:\nonexistent_pes_folder",
        "pause_allow = 1": "pause_allow = 0",
        "updates_check = 1": "updates_check = 0",
    }
    for old, new in replacements.items():
        assert text.count(old) == 1, old
        text = text.replace(old, new)
    with open(settings + ".new", "w", encoding="utf-8", newline="\r\n") as handle:
        handle.write(text)
    os.replace(settings + ".new", settings)
    exports = os.path.join(target, "exports_to_add")
    os.makedirs(exports)
    if os.path.isdir(export_path):
        shutil.copytree(export_path, os.path.join(exports, os.path.basename(export_path)))
    else:
        shutil.copy2(export_path, os.path.join(exports, os.path.basename(export_path)))


def ids_lines(upgrade_output, compile_output):
    """Red's boots and gloves folder IDs mapped to the exclusive IDs the compiler planned
    (`(team - 701) * 40 + 101 + slot - 1`), from the upgrade's `wearer_assumed` lines."""
    team = TEAM.search(compile_output)
    if team is None:
        return []
    team = int(team.group(1))
    lines = []
    for kind, folder, _old, slot in WEARER.findall(upgrade_output):
        letter = "k" if kind == "Boots" else "g"
        # Four digits, as the CPK paths spell an ID (`g0461`).
        lines.append(f"{folder}\t{letter}{(team - 701) * 40 + 101 + int(slot) - 1:04}")
    return lines


def clear_readonly(function, path, _error):
    assert os.path.realpath(path).startswith(os.path.realpath(HERE) + os.sep), path
    os.chmod(path, stat.S_IWRITE)
    function(path)


def remove_scratch(target):
    """Removes `red/<n>` or `jobs/<n>/up` or `jobs/<n>/out`, which this run created, and nothing
    else; retries while another process (an antivirus scan) holds a just-written file."""
    resolved = os.path.realpath(target)
    parent = os.path.dirname(resolved)
    if os.path.dirname(parent) == os.path.realpath(JOBS):
        assert os.path.basename(resolved) in ("up", "out"), resolved
        assert os.path.basename(parent).isdigit(), resolved
    else:
        assert parent == os.path.realpath(RED_COPIES), resolved
        assert os.path.basename(resolved).isdigit(), resolved
    for attempt in range(20):
        if not os.path.exists(resolved):
            return
        try:
            shutil.rmtree(resolved, onexc=clear_readonly)
            return
        except PermissionError:
            if attempt == 19:
                raise
            time.sleep(0.5)


def run_one(index, path, exe, parity_exe, worker):
    job = os.path.join(JOBS, str(index))
    assert not os.path.exists(job), f"{job} exists: an earlier run's"
    up = os.path.join(job, "up")
    out = os.path.join(job, "out")
    os.makedirs(up)
    upgrade_exe, _ = prepare_bin(exe, worker, FOX_VERSION)
    command = [upgrade_exe, "export-upgrader", "upgrade", path, "-o", up]
    if not is_referee(path):
        command.insert(4, "--loose")
    up_code, up_output, _ = run(command, os.path.join(job, "upgrade.txt"))
    written = os.listdir(up)
    if len(written) != 1:
        remove_scratch(up)
        return (index, path, "-", "-", 0.0, "", up_code, f"no single upgraded export: {written}", 0.0, "")
    upgraded = os.path.join(up, written[0])
    version = PREFOX_VERSION if has_model(upgraded) else FOX_VERSION
    bin_exe, output = prepare_bin(exe, worker, version)
    assert not os.path.exists(output), f"{output} exists: the previous compile's output was not moved"
    code, compile_output, seconds = run(
        [bin_exe, "team-compiler", "compile", "--no-deploy", "--export", upgraded], os.path.join(job, "compile.txt"))
    outcome = outcome_of(code, compile_output)
    if os.path.exists(output):
        os.replace(output, out)
    ours = manifest(out, os.path.join(job, "manifest_ours.tsv"))
    ids = ids_lines(up_output, compile_output)
    with open(os.path.join(job, "ids.tsv"), "w", encoding="utf-8", newline="\n") as handle:
        handle.write("".join(line + "\n" for line in ids))

    red_target = os.path.join(RED_COPIES, str(index))
    setup_red(red_target, version, path)
    command = [os.path.join(red_target, "Engines", "embed", "python.exe"),
               os.path.join(red_target, "Engines", "compiler_main.py"), "0", "--no-pause"]
    red_code, _, red_seconds = run(command, os.path.join(job, "red_run.txt"), cwd=red_target)
    logs = os.path.join(job, "red_logs")
    os.makedirs(logs)
    for name in os.listdir(red_target):
        if name.lower().endswith(".log"):
            shutil.copy2(os.path.join(red_target, name), os.path.join(logs, name))
    patches = os.path.join(red_target, "patches_output")
    red_cpks = manifest(patches, os.path.join(job, "manifest_red.tsv"))

    classes = Counter()
    if red_cpks and ours:
        report = os.path.join(job, "parity.tsv")
        parity = subprocess.run(
            [parity_exe, str(version), patches, out, report, os.path.join(job, "ids.tsv")],
            capture_output=True, text=True)
        if parity.returncode != 0:
            classes["parity_tool_failed"] += 1
            with open(os.path.join(job, "parity_error.txt"), "w", encoding="utf-8", newline="\n") as handle:
                handle.write(parity.stderr)
        else:
            for line in open(report, encoding="utf-8"):
                classes[line.rstrip("\n").split("\t")[3]] += 1
    summary = ";".join(f"{name}={count}" for name, count in sorted(classes.items()))
    if KEEP:
        # A diagnostic rerun keeps both outputs: Red's CPKs beside ours, `up/` and `out/`.
        if os.path.isdir(patches):
            os.replace(patches, os.path.join(job, "red_out"))
    remove_scratch(red_target)
    if not KEEP:
        remove_scratch(up)
        remove_scratch(out)
    return (index, path, version, red_code, red_seconds, ";".join(red_cpks), up_code, outcome, seconds, summary)


def main():
    global JOBS, KEEP, CENSUS, BIN
    parser = argparse.ArgumentParser()
    parser.add_argument("tag")
    parser.add_argument("--indexes", type=int, nargs="*", default=None)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--jobs", default="jobs", help="the jobs folder, under config.LIBRARY_DIR")
    parser.add_argument("--keep", action="store_true", help="keep up/, out/ and Red's CPKs (red_out/)")
    parser.add_argument("--frozen", action="store_true",
                        help="run the --bin folder's binaries as they are, even when older than the build "
                             "(to reproduce an earlier run)")
    parser.add_argument("--census", required=True, help="the upgrade census's outcomes file, whose exit-0 rows are the inputs")
    parser.add_argument("--bin", default="bin", help="the frozen binaries' folder, under config.LIBRARY_DIR (or an absolute path)")
    args = parser.parse_args()
    JOBS = os.path.join(HERE, args.jobs)
    KEEP = args.keep
    CENSUS = args.census
    BIN = os.path.join(HERE, args.bin)
    os.makedirs(HERE, exist_ok=True)
    check_red()
    if args.frozen:
        exe, parity_exe = os.path.join(BIN, "utw.exe"), os.path.join(BIN, "red_parity.exe")
        assert os.path.isfile(exe) and os.path.isfile(parity_exe), f"{BIN} holds no frozen binaries"
    else:
        exe, parity_exe = freeze_binaries()
    selection = inputs()
    indexes = args.indexes if args.indexes is not None else list(range(len(selection)))[args.offset:][:args.limit]
    print(len(selection), "clean upgrades,", len(indexes), "in this run", flush=True)
    for version in (PREFOX_VERSION, FOX_VERSION):
        for worker in range(args.workers):
            prepare_bin(exe, worker, version)
    free = queue.Queue()
    for worker in range(args.workers):
        free.put(worker)
    stopped = threading.Event()

    def with_worker(index):
        if stopped.is_set():
            return None
        if shutil.disk_usage(HERE).free < FLOOR_BYTES:
            stopped.set()
            print("stopping: under the 10 GB floor before input", index, flush=True)
            return None
        worker = free.get()
        try:
            return run_one(index, selection[index], exe, parity_exe, worker)
        finally:
            free.put(worker)

    rows = os.path.join(HERE, f"rows_{args.tag}.tsv")
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for row in pool.map(with_worker, indexes):
            if row is None:
                continue
            with open(rows, "a", encoding="utf-8", newline="\n") as handle:
                handle.write("\t".join(str(field) for field in row) + "\n")
            print(row[0], row[2], "red", row[3], f"{row[4]}s", "| ours", row[7][:100], "|", row[9][:200], flush=True)


if __name__ == "__main__":
    main()
