"""Gate 3: the workspace's tests, several test binaries at once.

`cargo test --workspace` runs its test binaries one after another, so a binary whose last
test is slow holds every other core idle until it ends: measured 2026-10-25, the 45 binaries
took 277 s that way and 176 s four at once (8 at once: 180 s, the machine's cores already full).
Each binary still runs whole, its tests on libtest's threads and its fixtures loaded once:
`cargo nextest` starts one process per test instead, which on Windows means a Python memory-cap
wrapper per test (`.cargo/config.toml`'s runner), and measured slower than `cargo test`
(391-500 s against 371-373 s).

Everything is built once with `--workspace`, so every binary has the features the workspace
unifies (egui's fonts arrive through eframe only then; a `-p` build measures text differently).

Usage: run_tests.py [--changed BASE]

With `--changed BASE` only the binaries of the packages the working tree changes since the
commit BASE run, with every workspace package depending on them; a changed file outside every
package, outside `docs/` (and the root's `*.md`), other than the mutation runs' exclusion
files and other than a script under `scripts/` no test run goes through runs them all. Those
alone run none.
The doc-tests run whenever any binary does.
"""

import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TIMES = ROOT / "target" / "run_tests_times.json"
JOBS = min(4, os.cpu_count() or 1)
# Files outside every package that no test reads: the mutation runs' exclusions, which change
# with most review rounds (`scripts/mutation/mutants_test.py`, run in both modes, checks them).
MUTATION_ONLY = {".cargo/mutants.toml", ".cargo/extra_mutants.toml"}
# The scripts every test binary runs through (this runner and the memory-cap wrapper
# `.cargo/config.toml` names): a change to one can change any result. Any other script under
# `scripts/` no Rust test reads (a generator's output is committed inside its crate), so its
# change cannot move a Rust result; the scripts' own `*_test.py` the gates list run regardless.
TEST_PATH_SCRIPTS = {"scripts/gates/run_tests.py", "scripts/gates/test_runner.cmd", "scripts/gates/test_runner.py"}


@dataclass(frozen=True)
class Package:
    name: str
    dir: Path
    deps: frozenset[str]


@dataclass(frozen=True)
class Binary:
    package: str
    target: str
    exe: str

    @property
    def key(self) -> str:
        return f"{self.package}/{self.target}"


def packages(metadata: dict) -> dict[str, Package]:
    """The workspace's packages by name, with their workspace dependencies (dev ones too)."""
    names = {p["name"] for p in metadata["packages"]}
    return {
        p["name"]: Package(
            p["name"],
            Path(p["manifest_path"]).parent,
            frozenset(d["name"] for d in p["dependencies"] if d["name"] in names),
        )
        for p in metadata["packages"]
    }


def owner(path: Path, pkgs: dict[str, Package]) -> str | None:
    """The package whose folder holds `path` (the deepest one), or None."""
    holders = [p for p in pkgs.values() if p.dir == path or p.dir in path.parents]
    return max(holders, key=lambda p: len(p.dir.parts)).name if holders else None


def dependents(names: set[str], pkgs: dict[str, Package]) -> set[str]:
    """`names` and every package depending on one of them, directly or not."""
    found = set(names)
    grew = True
    while grew:
        grew = False
        for p in pkgs.values():
            if p.name not in found and p.deps & found:
                found.add(p.name)
                grew = True
    return found


def select(changed: list[Path], pkgs: dict[str, Package]) -> set[str]:
    """The packages whose tests a change to the files `changed` (absolute paths) can affect."""
    owners = set()
    for path in changed:
        name = owner(path, pkgs)
        if name is not None:
            owners.add(name)
            continue
        rel = path.relative_to(ROOT)
        if rel.parts[0] == "docs" or (len(rel.parts) == 1 and rel.suffix == ".md"):
            continue
        if rel.as_posix() in MUTATION_ONLY:
            continue
        if rel.parts[0] == "scripts" and rel.as_posix() not in TEST_PATH_SCRIPTS:
            continue
        # Outside every package: the toolchain, Cargo.lock, `.cargo/`, `resources/`, a
        # script the test runs go through. Any of them can change every result.
        return set(pkgs)
    return dependents(owners, pkgs)


def changed_since(base: str) -> list[Path]:
    """The files the working tree changes since `base`, untracked ones included."""
    def lines(*args: str) -> list[str]:
        out = subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True,
                             text=True, encoding="utf-8").stdout
        return [line for line in out.splitlines() if line]
    names = lines("diff", "--name-only", base) + lines("ls-files", "--others", "--exclude-standard")
    return [ROOT / name for name in names]


def binaries(cargo_json: str, pkgs: dict[str, Package]) -> list[Binary]:
    """The test executables a `--no-run --message-format=json` build printed."""
    by_dir = {p.dir: p.name for p in pkgs.values()}
    found = []
    for line in cargo_json.splitlines():
        if not line.startswith("{"):
            continue
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg.get("executable") and msg["profile"]["test"]:
            package = by_dir[Path(msg["manifest_path"]).parent]
            found.append(Binary(package, msg["target"]["name"], msg["executable"]))
    return found


def runner() -> list[str]:
    """The wrapper `cargo test` puts every test binary in on this platform (`.cargo/config.toml`):
    on Windows the per-process memory cap, which the binaries keep here."""
    if os.name != "nt":
        return []
    import tomllib  # here: Python 3.11+, which only the Windows side has to have

    config = tomllib.loads((ROOT / ".cargo" / "config.toml").read_text(encoding="utf-8"))
    command = config["target"]["cfg(windows)"]["runner"]
    return [str(ROOT / command[0]), *command[1:]]


def run(binary: Binary, pkgs: dict[str, Package], wrapper: list[str]) -> tuple[int, float, str]:
    """One binary whole, as `cargo test` runs it: from its package's folder, with the package
    variables cargo sets."""
    pkg = pkgs[binary.package]
    env = dict(os.environ, CARGO_MANIFEST_DIR=str(pkg.dir), CARGO_PKG_NAME=pkg.name)
    start = time.monotonic()
    proc = subprocess.run([*wrapper, binary.exe], cwd=pkg.dir, env=env, capture_output=True,
                          text=True, encoding="utf-8", errors="replace")
    return proc.returncode, time.monotonic() - start, proc.stdout + proc.stderr


def main(argv: list[str]) -> int:
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT, check=True,
        capture_output=True, text=True, encoding="utf-8").stdout)
    pkgs = packages(meta)
    if argv[:1] == ["--changed"]:
        wanted = select(changed_since(argv[1]), pkgs)
        print(f"run_tests: changed since {argv[1]}: {', '.join(sorted(wanted)) or 'no package'}")
        if not wanted:
            return 0
    else:
        wanted = set(pkgs)

    # The build's diagnostics go to the terminal; its JSON lines name the executables.
    build = subprocess.run(
        ["cargo", "test", "--workspace", "--no-run", "--message-format=json-render-diagnostics"],
        cwd=ROOT, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    if build.returncode != 0:
        return build.returncode
    selected = [b for b in binaries(build.stdout, pkgs) if b.package in wanted]
    # Longest first, by the last run's times, so the slowest binary never starts last.
    times = json.loads(TIMES.read_text(encoding="utf-8")) if TIMES.exists() else {}
    selected.sort(key=lambda b: -times.get(b.key, 0.0))

    wrapper = runner()
    start = time.monotonic()
    failed = passed = 0
    with ThreadPoolExecutor(max_workers=JOBS) as pool:
        futures = {pool.submit(run, b, pkgs, wrapper): b for b in selected}
        for future in as_completed(futures):
            binary = futures[future]
            code, secs, output = future.result()
            times[binary.key] = round(secs, 1)
            summary = [line for line in output.splitlines() if line.startswith("test result:")]
            passed += sum(int(line.split()[3]) for line in summary)
            print(f"{binary.key} ({secs:.1f} s): {summary[-1] if summary else f'exit {code}'}",
                  flush=True)
            if code != 0:
                failed += 1
                print(f"---- {binary.key} failed (exit {code}) ----\n{output}", flush=True)
    wall = time.monotonic() - start
    TIMES.write_text(json.dumps(times, indent=1, sort_keys=True), encoding="utf-8")
    print(f"run_tests: {len(selected)} binaries, {passed} tests passed, {failed} binaries failed, "
          f"{wall:.0f} s, {JOBS} at once")
    if failed:
        return 1
    return subprocess.run(["cargo", "test", "--workspace", "--doc"], cwd=ROOT).returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
