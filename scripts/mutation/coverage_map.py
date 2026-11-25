"""Builds a whole-crate run's selection file: which tests cover each mutant's lines.

`python scripts/mutation/coverage_map.py <crate> <selection.json> [--mutants
<mutants.json>] [--extra]`, at least one of the two options, run
from the root of the tree the mutants run on. `<mutants.json>` is `cargo mutants -p
<crate> --list --json`'s output; `--extra` appends the extra kinds' mutants
(`extra_mutants.list_mutants`, every kind), each row's byte range mapped to the
file's lines. `<selection.json>` is written in the shape
`select_tests.py` reads, one entry per mutant in the list's order — the
cargo-mutants entries first — its `filter` the
nextest expression of the tests covering the lines its `span` names, `null` when no
test does.

The crate's test binaries are built instrumented (`cargo llvm-cov`'s environment, into
`target/llvm-cov-target/` so the normal build is untouched), each listed test runs alone
with its own LLVM profile file, and `llvm-profdata` + `llvm-cov export` turn the profiles
into the covered lines of the crate's `src/`. The crate's own programs are read with each
test binary, so a test that starts one (a CLI test) is credited with the code it runs
there. A test whose own process wrote no profile (a timeout, a crash) or whose profile
cannot be read fails the whole map, and nothing is written: a mutant that test alone
catches would otherwise get a filter without it, and survive.

Needs on the machine: cargo-llvm-cov, cargo-nextest and rustup's `llvm-tools-preview`
component, which carries llvm-profdata and llvm-cov.
"""

import concurrent.futures
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import extra_mutants  # noqa: E402  (kill_tree)
import mutants  # noqa: E402  (below_normal)

# Every core: the map runs only in the remote's job, whose unit (`CPUWeight=idle`, under
# `nice`) already yields the CPUs to the host's own services, so a share held back stays idle.
JOBS = os.cpu_count() or 1
# Well past a single test's wait bound, so an unmutated test never reaches it.
TEST_TIMEOUT_SECONDS = 600

_LLVM_BIN: Path | None = None


def llvm_tool(name: str) -> str:
    """`<sysroot>/lib/rustlib/<host>/bin/<name>` of the toolchain rustc selects."""
    global _LLVM_BIN
    if _LLVM_BIN is None:
        sysroot = Path(
            subprocess.run(
                ["rustc", "--print", "sysroot"],
                check=True, capture_output=True, text=True,
            ).stdout.strip()
        )
        verbose = subprocess.run(
            ["rustc", "-vV"], check=True, capture_output=True, text=True
        ).stdout
        host = next(
            line.split(":", 1)[1].strip()
            for line in verbose.splitlines()
            if line.startswith("host:")
        )
        _LLVM_BIN = sysroot / "lib" / "rustlib" / host / "bin"
    tool = _LLVM_BIN / (name + ".exe" if os.name == "nt" else name)
    if not tool.exists():
        raise SystemExit(
            f"coverage_map: {tool} missing (rustup component llvm-tools-preview)"
        )
    return str(tool)


def coverage_env(root: Path) -> dict[str, str]:
    """os.environ plus the variables `cargo llvm-cov show-env` prints (the
    RUSTC_WRAPPER that instruments workspace crates), with the build directed to
    `root`'s `target/llvm-cov-target/`: show-env's CARGO_LLVM_COV_TARGET_DIR names
    the normal target and it sets no CARGO_TARGET_DIR, so without the override the
    instrumented build would overwrite the normal one."""
    cov_target = root / "target" / "llvm-cov-target"
    env = dict(os.environ)
    shown = subprocess.run(
        ["cargo", "llvm-cov", "show-env"],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout
    for line in shown.splitlines():
        key, eq, value = line.partition("=")
        if eq and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            env[key] = value.removeprefix("'").removesuffix("'")
    for key in (
        "CARGO_TARGET_DIR",
        "CARGO_LLVM_COV_TARGET_DIR",
        "CARGO_LLVM_COV_BUILD_DIR",
    ):
        env[key] = str(cov_target)
    # Profiles the instrumented build itself writes (build scripts, the listing)
    # stay inside the coverage target too.
    env["LLVM_PROFILE_FILE"] = str(cov_target / "profraws" / "%p-%16m.profraw")
    return env


def listed_tests(listing: dict) -> list[dict]:
    """Every non-ignored test in a `nextest list --message-format json` listing,
    as {binary_id, name, binary, cwd} — exactly the tests a mutation run runs."""
    tests = []
    for suite in listing["rust-suites"].values():
        for name, case in suite["testcases"].items():
            if not case["ignored"]:
                tests.append(
                    {
                        "binary_id": suite["binary-id"],
                        "name": name,
                        "binary": suite["binary-path"],
                        "cwd": suite["cwd"],
                    }
                )
    return tests


def nextest_list(crate: str, root: Path, env: dict) -> list[dict]:
    """`cargo nextest list -p <crate>` in `root` under `env` (it builds the
    instrumented test binaries), then the listing's non-ignored tests."""
    out = subprocess.run(
        ["cargo", "nextest", "list", "-p", crate, "--message-format", "json"],
        cwd=root, env=env, capture_output=True, text=True,
        **mutants.below_normal(),
    )
    if out.returncode != 0:
        raise SystemExit(f"coverage_map: nextest list failed:\n{out.stderr[-4000:]}")
    tests = listed_tests(json.loads(out.stdout))
    if not tests:
        raise SystemExit(f"coverage_map: nextest listed no tests for {crate}")
    return tests


def package_layout(crate: str, root: Path) -> tuple[str, list[str]]:
    """The crate's `src/` folder relative to `root`, slash-separated with the
    trailing slash, and the names of its programs (`bin` targets), from the
    package's manifest in `cargo metadata` (not the listed mutants' paths, which a
    crate with mutants in one subfolder would narrow too far)."""
    meta = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root, capture_output=True, text=True,
    )
    if meta.returncode != 0:
        raise SystemExit(f"coverage_map: cargo metadata failed:\n{meta.stderr[-2000:]}")
    for package in json.loads(meta.stdout)["packages"]:
        if package["name"] == crate:
            folder = Path(package["manifest_path"]).resolve().parent
            bins = [t["name"] for t in package["targets"] if "bin" in t["kind"]]
            return folder.relative_to(root).as_posix() + "/src/", bins
    raise SystemExit(f"coverage_map: no package named {crate} in this workspace")


def covered_lines(lcov: str, root: Path, src: str) -> set[tuple[str, int]]:
    """The (file, line) pairs `lcov` (an `llvm-cov export -format=lcov` text)
    reports covered in `src`: every `DA:<line>,<count>` with count > 0 whose
    `SF:` file, relative to `root` and slash-separated, starts with `src`."""
    covered = set()
    wanted = False
    for line in lcov.splitlines():
        if line.startswith("SF:"):
            try:
                rel = Path(line[3:]).resolve().relative_to(root).as_posix()
            except ValueError:
                rel = ""
            wanted = rel.startswith(src)
        elif wanted and line.startswith("DA:"):
            number, count = line[3:].split(",")[:2]
            if int(count) > 0:
                covered.add((rel, int(number)))
    return covered


def run_test(test: dict, profile: Path, env: dict) -> tuple[int | None, int]:
    """The test's binary with `--exact <name>` in its suite's cwd, its coverage
    profile going to `profile` (LLVM's %p/%16m pattern). The exit code (None when
    TEST_TIMEOUT_SECONDS killed the process tree) and the test process's id."""
    if os.name == "nt":
        kwargs = mutants.below_normal()
    else:
        # No `below_normal()` here: its `preexec_fn` can deadlock a child forked from
        # a worker thread, and the remote job already runs under `nice`, which the
        # tests inherit. A session of their own, so kill_tree's group kill hits the
        # test's tree only, never this script's.
        kwargs = {"start_new_session": True}
    proc = subprocess.Popen(
        [test["binary"], "--exact", test["name"]],
        cwd=test["cwd"],
        env={**env, "LLVM_PROFILE_FILE": str(profile)},
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        **kwargs,
    )
    try:
        proc.wait(timeout=TEST_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        extra_mutants.kill_tree(proc)
        return None, proc.pid
    return proc.returncode, proc.pid


def one_coverage(
    index: int, test: dict, env: dict, profiles: Path, root: Path, src: str,
    programs: list[str],
) -> dict:
    """One test's covered (file, line) pairs, run alone with its own profile
    files (deleted once read), read against its binary and `programs` (the
    crate's own programs it may start). A failing test still gives its coverage;
    a test whose own process wrote no profile, or a tool failing, lands in `note`."""
    profraw = profiles / f"{index}-%p-%16m.profraw"
    profdata = profiles / f"{index}.profdata"
    lines: set[tuple[str, int]] = set()
    note = None
    try:
        code, pid = run_test(test, profraw, env)
        raws = sorted(profiles.glob(f"{index}-*.profraw"))
        # A child's profile does not stand for the test's own: a test killed or
        # crashed after its child exited would leave only the child's.
        if not any(raw.name.startswith(f"{index}-{pid}-") for raw in raws):
            note = "the test's own process wrote no profile"
        else:
            merged = subprocess.run(
                [
                    llvm_tool("llvm-profdata"), "merge", "-sparse",
                    *(str(raw) for raw in raws), "-o", str(profdata),
                ],
                capture_output=True, text=True,
            )
            if merged.returncode != 0:
                note = f"llvm-profdata: {merged.stderr.strip()[-200:]}"
            else:
                exported = subprocess.run(
                    [
                        llvm_tool("llvm-cov"), "export", "-format=lcov",
                        "-instr-profile", str(profdata), test["binary"],
                        *(arg for program in programs for arg in ("-object", program)),
                    ],
                    capture_output=True, text=True,
                    encoding="utf-8", errors="replace",
                )
                if exported.returncode != 0:
                    note = f"llvm-cov: {exported.stderr.strip()[-200:]}"
                else:
                    lines = covered_lines(exported.stdout, root, src)
    finally:
        for stale in profiles.glob(f"{index}-*"):
            stale.unlink(missing_ok=True)
        profdata.unlink(missing_ok=True)
    return {"index": index, "code": code, "lines": lines, "note": note}


def selection(
    listed: list[dict], coverage: dict[tuple[str, int], set[int]], tests: list[dict]
) -> list[str | None]:
    """Per mutant, in the list's order: the nextest filterset of the tests
    covering any line of its span — one `(binary_id(=B) & test(=T))` term each,
    sorted by the tests' index and joined with ` | ` — or None when no test
    covers the span. Test names and binary ids hold only identifier characters,
    safe unquoted in nextest's DSL."""
    filters = []
    for mutant in listed:
        span = mutant["span"]
        indexes = set()
        for line in range(span["start"]["line"], span["end"]["line"] + 1):
            indexes.update(coverage.get((mutant["file"], line), ()))
        terms = [
            f"(binary_id(={tests[i]['binary_id']}) & test(={tests[i]['name']}))"
            for i in sorted(indexes)
        ]
        filters.append(" | ".join(terms) if terms else None)
    return filters


def main(argv: list[str]) -> int:
    extra = "--extra" in argv
    mutants_path = None
    positional = []
    i = 0
    while i < len(argv):
        if argv[i] == "--extra":
            i += 1
        elif argv[i] == "--mutants" and i + 1 < len(argv):
            mutants_path = argv[i + 1]
            i += 2
        else:
            positional.append(argv[i])
            i += 1
    if len(positional) != 2 or (mutants_path is None and not extra):
        print(
            "usage: python scripts/mutation/coverage_map.py "
            "<crate> <selection.json> [--mutants <mutants.json>] [--extra]",
            file=sys.stderr,
        )
        return 2
    crate, out_path = positional
    root = Path.cwd().resolve()
    listed: list[dict] = []
    if mutants_path is not None:
        try:
            listed.extend(
                json.loads(Path(mutants_path).read_text(encoding="utf-8"))
            )
        except (OSError, ValueError) as error:
            raise SystemExit(f"coverage_map: cannot read {mutants_path} ({error})")
    if extra:
        directory = extra_mutants.crate_dir(crate, root)
        crate_rel = directory.relative_to(root)
        contents: dict[str, bytes] = {}
        for row in extra_mutants.list_mutants(directory):
            if row[0] not in contents:
                contents[row[0]] = (directory / row[0]).read_bytes()
            start, end = extra_mutants.byte_lines(
                contents[row[0]], int(row[1]), int(row[2])
            )
            # The `span` shape `selection` reads the cargo-mutants entries by.
            listed.append({
                "file": (crate_rel / row[0]).as_posix(),
                "span": {"start": {"line": start}, "end": {"line": end}},
            })
    env = coverage_env(root)
    src, bins = package_layout(crate, root)
    tests = nextest_list(crate, root, env)
    built = root / "target" / "llvm-cov-target" / "debug"
    suffix = ".exe" if os.name == "nt" else ""
    # Only the programs the build made: a crate's tests that start none build none.
    programs = [str(built / (name + suffix)) for name in bins]
    programs = [program for program in programs if Path(program).exists()]
    profiles = root / "target" / "llvm-cov-target" / "test-profiles"
    profiles.mkdir(parents=True, exist_ok=True)
    # Profiles an interrupted earlier run left would merge into this run's.
    for stale in profiles.iterdir():
        stale.unlink()
    started = time.monotonic()
    print(f"coverage_map: {len(tests)} tests for {crate}", flush=True)
    coverage: dict[tuple[str, int], set[int]] = {}
    failed = 0
    unread: list[str] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=JOBS) as pool:
        futures = [
            pool.submit(one_coverage, index, test, env, profiles, root, src, programs)
            for index, test in enumerate(tests)
        ]
        done = 0
        for future in concurrent.futures.as_completed(futures):
            entry = future.result()
            done += 1
            if done % 100 == 0:
                print(f"coverage_map: {done}/{len(tests)} tests", flush=True)
            if entry["code"] != 0:
                failed += 1
            if entry["note"] is not None:
                unread.append(f"{tests[entry['index']]['name']}: {entry['note']}")
            for key in entry["lines"]:
                coverage.setdefault(key, set()).add(entry["index"])
    if unread:
        print("\n".join(f"coverage_map: {line}" for line in unread), file=sys.stderr)
        print(f"coverage_map: the coverage of {len(unread)} tests could not be read; "
              "no selection file written", file=sys.stderr)
        return 1
    filters = selection(listed, coverage, tests)
    entries = [
        {
            "file": mutant["file"],
            "start": mutant["span"]["start"]["line"],
            "end": mutant["span"]["end"]["line"],
            "filter": filter_,
        }
        for mutant, filter_ in zip(listed, filters)
    ]
    out = Path(out_path)
    temp = out.with_suffix(".tmp")
    temp.write_text(
        json.dumps({"mutants": entries}, indent=1) + "\n",
        encoding="utf-8", newline="\n",
    )
    os.replace(temp, out)
    minutes = (time.monotonic() - started) / 60
    with_filter = sum(filter_ is not None for filter_ in filters)
    print(
        f"coverage_map: {len(tests)} tests, {len(coverage)} covered lines, "
        f"{with_filter} mutants with a filter, {len(filters) - with_filter} "
        f"without, {failed} failed, {minutes:.1f} min",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
