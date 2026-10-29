"""Runs every candidate old export through `export-upgrader upgrade` and tallies the outcomes.

Inputs: folders.tsv (find_folders.py) and the `export` rows of archives.tsv
(find_archives.py), under `config.CENSUS_DIR`, where this script writes too. A
team export runs with `--loose` (no savefile is known for it); a referee export
(a name whose first word is `refs`) with neither. Each run writes into
out/<n>/, which this script creates fresh and removes once the run's listing
(files, bytes) is recorded: the outputs together would not fit on the disk.

Outcome class: the exit code, then the sorted distinct Error and Warning codes.
Writes outcomes_<tag>.tsv (path, exit, class, files, bytes, seconds),
reports_<tag>.txt (every run's output) and census_<tag>.txt (tally per class
with up to three example paths, then every code's count of exports and lines).
Usage: census.py <utw exe> <tag>
"""
import os
import re
import shutil
import subprocess
import sys
import time
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor

import config

HERE = config.CENSUS_DIR
OUT = os.path.join(HERE, "out")
LINE = re.compile(r"^- (?:(.*?): )?(Error|Warning|Info) (\S+)")


def inputs():
    rows = [line.split("\t")[0] for line in open(os.path.join(HERE, "folders.tsv"), encoding="utf-8")]
    for line in open(os.path.join(HERE, "archives.tsv"), encoding="utf-8"):
        path, _size, kind, _detail = line.rstrip("\n").split("\t")
        if kind == "export":
            rows.append(path)
    return rows


def name_of(path):
    base = os.path.basename(path)
    lowered = base.lower()
    if lowered.endswith(".zip") or lowered.endswith(".7z"):
        base = os.path.splitext(base)[0]
    return base


def remove_output(target):
    """Removes out/<n>/, which this script created; refuses any other path."""
    resolved = os.path.realpath(target)
    assert os.path.dirname(resolved) == os.path.realpath(OUT), resolved
    assert os.path.basename(resolved).isdigit(), resolved
    shutil.rmtree(resolved)


def listing(target):
    files = total = 0
    for root, _dirs, names in os.walk(target):
        for name in names:
            files += 1
            total += os.path.getsize(os.path.join(root, name))
    return files, total


def run_one(job):
    index, exe, path = job
    target = os.path.join(OUT, str(index))
    assert not os.path.exists(target), target
    os.makedirs(target)
    command = [exe, "export-upgrader", "upgrade", path, "-o", target]
    if name_of(path).split(" ")[0].lower() != "refs":
        command.insert(4, "--loose")
    started = time.monotonic()
    try:
        run = subprocess.run(command, capture_output=True, timeout=1800)
        code, output = run.returncode, (run.stdout + run.stderr).decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        code, output = "timeout", ""
    seconds = round(time.monotonic() - started, 1)
    files, total = listing(target)
    remove_output(target)
    findings = []
    for line in output.splitlines():
        match = LINE.match(line)
        if match:
            findings.append((match.group(2), match.group(3)))
        elif line.strip() and not line.endswith("/"):
            findings.append(("Other", line.strip()[:120]))
    serious = sorted({code_ for severity, code_ in findings if severity in ("Error", "Warning", "Other")})
    outcome = f"exit {code}: " + (",".join(serious) if serious else "-")
    return path, code, outcome, files, total, seconds, findings, output


def main():
    exe, tag = sys.argv[1], sys.argv[2]
    os.makedirs(OUT, exist_ok=True)
    assert not os.listdir(OUT), "out/ is not empty"
    jobs = [(index, exe, path) for index, path in enumerate(inputs())]
    print(len(jobs), "inputs", flush=True)
    results = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        for count, result in enumerate(pool.map(run_one, jobs), 1):
            results.append(result)
            if count % 25 == 0:
                print(count, "done", flush=True)
            with open(os.path.join(HERE, f"reports_{tag}.txt"), "a", encoding="utf-8") as log:
                log.write(f"=== {result[0]}\n{result[7]}\n")
    rows = sorted(results, key=lambda result: result[0].lower())
    out = os.path.join(HERE, f"outcomes_{tag}.tsv")
    with open(out + ".tmp", "w", encoding="utf-8") as handle:
        for path, code, outcome, files, total, seconds, _f, _o in rows:
            handle.write(f"{path}\t{code}\t{outcome}\t{files}\t{total}\t{seconds}\n")
    os.replace(out + ".tmp", out)
    classes = defaultdict(list)
    exports_per_code, lines_per_code = Counter(), Counter()
    for path, _code, outcome, _files, _total, _seconds, findings, _o in rows:
        classes[outcome].append(path)
        for key in {(severity, code_) for severity, code_ in findings}:
            exports_per_code[key] += 1
        for key in findings:
            lines_per_code[key] += 1
    text = [f"{len(rows)} inputs, {len(classes)} classes", ""]
    for outcome, paths in sorted(classes.items(), key=lambda item: (-len(item[1]), item[0])):
        text.append(f"{len(paths):5}  {outcome}")
        text += [f"         {path}" for path in paths[:3]]
    text += ["", "code: exports, lines"]
    for key in sorted(exports_per_code, key=lambda key: (-exports_per_code[key], key)):
        text.append(f"{key[0]} {key[1]}: {exports_per_code[key]}, {lines_per_code[key]}")
    report = os.path.join(HERE, f"census_{tag}.txt")
    with open(report + ".tmp", "w", encoding="utf-8") as handle:
        handle.write("\n".join(text) + "\n")
    os.replace(report + ".tmp", report)
    print("\n".join(text[:60]))


if __name__ == "__main__":
    main()
