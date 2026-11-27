"""`just gates-remote`: `just gates` run on the mutants host (`STUDIO_MUTANTS_REMOTE`)
against the whole working tree, for while this machine should not carry the
load (the gates' build once crashed it under memory pressure;
`mutants.REMOTE_ONLY` moves the diff mutation runs the same way).

The tree goes over as a mutation run sends it (`mutants.snapshot`,
`mutants.transfer`), into the same remote clone, so this refuses to start while
a remote mutants half is running or uncollected, other gates run there, or the
mutation driver's VPS lane runs a shard there (`mutants.SHARD_UNIT`) —
the refusal's check through the unit's whole run (the `--wait --pipe` call)
and the `target/` removal under the remote lock
(`mutants.REMOTE_LOCK`, `mutants.remote_refusal`), so a run launched meanwhile
waits rather than racing the clone. The whole gates' build cache, the clone's
`target/`, is removed when they end (`remote_clean_target.sh`), since it held
the disk under the mutation runs' floor; `just remote` keeps it, so a crate's
reruns build incrementally. The gates run as the transient
service `mutants.GATES_UNIT`, memory-capped (`mutants.remote_memory_max`) and
CPU-idle like a mutants half (the host's production Fluxer instance comes
first), synchronously:
`systemd-run --wait --pipe` streams their output here and returns their exit
code. A dropped connection or a Ctrl+C ends the run here, not there — Rust
binaries ignore SIGPIPE, so the closed pipe never tells the unit: the run
below stops it once, best effort, and the next run's refusal waits for
whatever survives. Unlike a mutation run, a rerun loses nothing.

The host runs the justfile's own `gates` recipe, with `just` installed in
`~/.cargo/bin` at this machine's version (`cargo install --locked just@<version>`,
as cargo-mutants is), so the two never parse the justfile differently; a run
refuses when the versions differ.

`just remote <command>` runs one `cargo` or `just` command there instead of the
whole gates, the same way (`just remote cargo test -p save_editor`), for
crate-scoped runs while this machine should not carry them. Only those two
programs: the clone is a build tree, and anything else is a typo or belongs on
this machine.
"""

import shlex
import subprocess
import sys
from pathlib import Path

import mutants

GATES_JOB = f"{mutants.REMOTE_BASE}/gates.sh"
CLEAN_TARGET = Path(__file__).with_name("remote_clean_target.sh")
# The recipes `just remote just <recipe>` refuses: each reads the diff or the
# messages since a base, which the remote's HEAD — the snapshot commit —
# empties or misreads (`commit-messages` reads the snapshot's own message).
CHANGED_RECIPES = ("gates-changed", "test-changed", "commit-messages")


def main(command: list[str]) -> int:
    if command and command[0] not in ("cargo", "just"):
        print(f"runs a cargo or just command, not {command[0]!r}")
        return 1
    if command[:1] == ["just"] and len(command) > 1 and command[1] in CHANGED_RECIPES:
        print(
            f"`just {command[1]}` reads the commits since a base, which the remote's "
            "snapshot commit makes meaningless: `just gates-remote` runs the whole "
            "gates there, `just remote cargo test -p <crate>` one crate's tests"
        )
        return 1
    # Quoted for the host's bash: a test filter or `-- --nocapture` passes through as typed.
    run_line = shlex.join(command) if command else "just gates"
    host = mutants.remote_host()
    if host is None:
        print("STUDIO_MUTANTS_REMOTE is not set")
        return 1
    # Before the lock's wait, as every entry point does: the run measures the
    # tree as it stood when its command started, whatever is edited meanwhile.
    tree = mutants.working_tree()
    with mutants.run_lock(mutants.REMOTE_LOCK):
        refusal = mutants.remote_refusal(host)
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
        # The pre-wait tree, committed to `REMOTE_REF` here under the lock.
        mutants.snapshot(tree)
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
        # The `--wait --pipe` call is the unit's whole run: the remote lock
        # stays held through it, so a mutants launch or collect waits rather
        # than racing the clone mid-gates.
        interrupted = False
        try:
            run = mutants.ssh(
                host,
                f"sudo -n systemd-run --unit={mutants.GATES_UNIT} --uid=debian --wait --pipe "
                f"--collect -p MemoryMax={memory_max} -p MemorySwapMax=0 -p CPUWeight=idle "
                f'--working-directory="$HOME/studio-mutants/tree" /bin/bash {GATES_JOB} '
                "< /dev/null",
            )
            code = run.returncode
        except KeyboardInterrupt:
            interrupted, code = True, 130
        if interrupted or code == 255:
            # The unit may still run: its Rust binaries ignore SIGPIPE, so the
            # closed pipe never reached it. One stop call, best effort — on a
            # dropped link it may not get through either.
            print(
                f"{'interrupted' if interrupted else 'the link dropped'}; "
                f"stopping {mutants.GATES_UNIT} on {host}",
                flush=True,
            )
            try:
                mutants.ssh(
                    host,
                    f"sudo -n systemctl stop {mutants.GATES_UNIT}",
                    timeout=mutants.POLL_CALL_TIMEOUT_SECONDS,
                )
            except subprocess.TimeoutExpired:
                pass
        print(f"`{run_line}` on {host}: exit {code}")
        if not command:
            # The whole gates leave a 30 GB build cache, which put the disk under the mutation
            # runs' free floor, so the next pipeline was refused every time: a
            # cold build costs those gates a few minutes, a refusal a whole run. The removal
            # itself stays under the remote lock: it deletes `target/` under the clone a
            # launch's builds would write back into.
            mutants.ssh(host, "bash -s -- --delete", input=CLEAN_TARGET.read_bytes())
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
