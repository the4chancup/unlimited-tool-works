"""Delete the tree copies a killed mutation run left behind: cargo-mutants builds each local run
in a `cargo-mutants-4cc-studio-*.tmp` copy under `mutants.py`'s `LOCAL_TMPDIR`, which a run
that ends normally removes and a killed one leaves, some with a multi-GB `target/`. Refuses
anything that is not such a copy. Run only while no mutation run is going: a live run builds in
one of these.

Run: python scripts/rm_mutant_copies.py
"""
import os
import shutil
import stat
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from mutants import LOCAL_TMPDIR  # noqa: E402

copies = sorted(LOCAL_TMPDIR.glob("cargo-mutants-4cc-studio-*.tmp"))
for copy in copies:
    assert copy.is_dir(), copy
    assert copy.parent == LOCAL_TMPDIR, copy
    assert (copy / "Cargo.toml").exists(), f"{copy} does not look like a workspace copy"


def writable_then_retry(function, path, _error):
    # A test leaves a file read-only (deploy_unwritable); clear the flag and retry.
    os.chmod(path, stat.S_IWRITE)
    function(path)


for copy in copies:
    shutil.rmtree(copy, onexc=writable_then_retry)
    print("removed", copy)
print(len(copies), "copies removed from", LOCAL_TMPDIR)
