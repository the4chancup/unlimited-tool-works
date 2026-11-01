"""`just gates-remote`: `just gates` run on the mutants host (`STUDIO_MUTANTS_REMOTE`)
against the whole working tree, for while this machine should not carry the
load (2026-10-10: four MEMORY_MANAGEMENT bugchecks here, the last during a run;
`mutants.REMOTE_ONLY` moves the mutation runs the same way).

The tree goes over as a mutation run sends it (`mutants.snapshot`,
`mutants.transfer`), into the same remote clone, so this refuses to start while
a remote mutants half is running or uncollected, or other gates run there
(`mutants.remote_refusal`). Its `target/` is gitignored and survives the
checkout, so a rerun builds incrementally. The gates run as the transient
service `mutants.GATES_UNIT`, memory-capped (`mutants.remote_memory_max`) and
CPU-idle like a mutants half (the host's production Fluxer instance comes
first), synchronously:
`systemd-run --wait --pipe` streams their output here and returns their exit
code. A dropped connection ends the run here; the unit stops at its next write
to the closed pipe, so it may run on a while, and the next run's refusal waits
for it. Unlike a mutation run, a rerun loses nothing.

The host runs the justfile's own `gates` recipe, with `just` installed in
`~/.cargo/bin` at this machine's version (`cargo install --locked just@<version>`,
as cargo-mutants is), so the two never parse the justfile differently; a run
refuses when the versions differ.

`just remote <command>` runs one `cargo` or `just` command there instead of the
whole gates, the same way (`just remote cargo test -p save_editor`): the
sidekick's crate-scoped test and clippy runs while this machine should not
carry them (2026-10-25). Only those two programs: the clone is a build tree, and
anything else is a typo or belongs on this machine.
"""

import shlex
import subprocess
import sys

import mutants

GATES_JOB = f"{mutants.REMOTE_BASE}/gates.sh"


def main(command: list[str]) -> int:
    if command and command[0] not in ("cargo", "just"):
        print(f"runs a cargo or just command, not {command[0]!r}")
        return 1
    # Quoted for the host's bash: a test filter or `-- --nocapture` passes through as typed.
    run_line = shlex.join(command) if command else "just gates"
    host = mutants.remote_host()
    if host is None:
        print("STUDIO_MUTANTS_REMOTE is not set")
        return 1
    refusal = mutants.remote_refusal(host, remote=True)
    if refusal is not None:
        print(refusal)
        return 1
    local_just = subprocess.run(
        ["just", "--version"], check=True, capture_output=True, text=True
    ).stdout.strip()
    probe = mutants.ssh(host, "~/.cargo/bin/just --version", capture_output=True, text=True)
    if probe.returncode == 255:
        print(f"cannot reach {host}")
        return 1
    remote_just = probe.stdout.strip()
    if local_just != remote_just:
        print(
            f"just versions differ: local {local_just}, remote {remote_just or 'none'}; fix: "
            f"cargo install --locked just@{local_just.split()[-1]} on the remote"
        )
        return 1
    tree = mutants.snapshot()
    remote_head, remote_tree, _nproc, _version = mutants.remote_state(host)
    if remote_tree == tree:
        print("remote tree up to date", flush=True)
    else:
        mutants.transfer(host, remote_head)
    job = (
        "set -e\n"
        ". ~/.cargo/env\n"
        # On disk, not the tmpfs `/tmp`, which counts against the unit's memory
        # (`mutants.REMOTE_TMPDIR`); the CLI tests write a sandbox export each.
        f'export TMPDIR="{mutants.REMOTE_TMPDIR}" && mkdir -p "$TMPDIR"\n'
        f"nice -n 19 ionice -c3 {run_line}\n"
    )
    # A file, not bash's stdin, which a test reading stdin would eat lines of.
    mutants.ssh(
        host,
        f"cat > {GATES_JOB}",
        # Bytes, not text: a CRLF in the script would break bash on the host.
        input=job.encode("utf-8"),
        check=True,
    )
    memory_max = mutants.remote_memory_max(host)
    print(f"running `{run_line}` on {host}", flush=True)
    run = mutants.ssh(
        host,
        f"sudo -n systemd-run --unit={mutants.GATES_UNIT} --uid=debian --wait --pipe "
        f"--collect -p MemoryMax={memory_max} -p MemorySwapMax=0 -p CPUWeight=idle "
        f'--working-directory="$HOME/studio-mutants/tree" /bin/bash {GATES_JOB} '
        "< /dev/null",
    )
    print(f"`{run_line}` on {host}: exit {run.returncode}")
    return run.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
