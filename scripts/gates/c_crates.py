"""`just deps-check`: no shipped crate compiles C unless it is a listed exception
(`docs/CONTRIBUTING.md` "Dependencies": the shipped build is pure Rust by default).

A crate compiles C through one of the `BUILDERS`, taken as a build-dependency. So for each
release target the check asks cargo which packages of the workspace's graph depend on a
builder directly (`cargo tree -i`, over normal and build edges: what ships, not what only
the tests use) and refuses one `EXCEPTIONS` does not name. An exception no target needs any
more is refused as well, so the list cannot outlive its reason.

The pure helpers take cargo's output as text, so `c_crates_test.py` runs without cargo.
"""

import subprocess
import sys

# The crates through which a build script compiles C or C++.
BUILDERS = ("cc", "cmake")
# The targets a release is built for (`.github/workflows/release.yml`).
TARGETS = ("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu")
# The crates allowed to depend on a builder, each with the reason a pure-Rust choice falls
# short; a new one is the maintainer's ruling (`docs/DECISIONS.md`).
EXCEPTIONS = {
    "ring": (
        "the update check's TLS crypto (C and assembly): no pure-Rust `rustls` provider is "
        "fit for the update path yet (DECISIONS 2026-11-12)"
    ),
    "wayland-backend": (
        "compiles no C as built here: its build script calls `cc` only under its `log` "
        "feature, which is off; the dependency is declared all the same"
    ),
}


def dependents(tree: str, builder: str) -> list[str]:
    """The names of the packages `cargo tree -i <builder> --depth 1 --prefix none` lists
    under the builder: one `<name> v<version>` line each, the builder's own line first."""
    names = []
    for line in tree.splitlines():
        parts = line.split()
        if len(parts) >= 2 and parts[1].startswith("v") and parts[0] != builder:
            names.append(parts[0])
    return names


def unlisted(found: dict[str, set[str]]) -> list[str]:
    """One complaint per crate that depends on a builder with no exception, and per
    exception nothing depends through any more. `found` maps a crate to the targets it
    was found on."""
    complaints = [
        f"{name} compiles C through a build dependency on {', '.join(sorted(targets))} and "
        "is no listed exception (scripts/gates/c_crates.py)"
        for name, targets in sorted(found.items())
        if name not in EXCEPTIONS
    ]
    complaints += [
        f"{name} is a listed exception no release target needs: remove it"
        for name in sorted(EXCEPTIONS)
        if name not in found
    ]
    return complaints


def builder_dependents(builder: str, target: str) -> list[str]:
    run = subprocess.run(
        ["cargo", "tree", "--workspace", "-i", builder, "--target", target,
         "-e", "normal,build", "--depth", "1", "--prefix", "none"],
        capture_output=True, text=True,
    )
    # A builder no package of the lock file uses is cargo's error, not the check's.
    if run.returncode != 0 and "did not match any packages" in run.stderr:
        return []
    if run.returncode != 0:
        sys.exit(f"c_crates: cargo tree -i {builder} failed for {target}:\n{run.stderr}")
    return dependents(run.stdout, builder)


def main() -> int:
    found: dict[str, set[str]] = {}
    for target in TARGETS:
        for builder in BUILDERS:
            for name in builder_dependents(builder, target):
                found.setdefault(name, set()).add(target)
    complaints = unlisted(found)
    for complaint in complaints:
        print(f"c_crates: {complaint}")
    if not complaints:
        print(f"c_crates: ok, {len(found)} listed exceptions ({', '.join(sorted(found))})")
    return 1 if complaints else 0


if __name__ == "__main__":
    sys.exit(main())
