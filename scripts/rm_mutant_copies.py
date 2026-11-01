"""Delete the tree copies a killed mutation run left behind under `mutants.py`'s `LOCAL_TMPDIR`,
which a run that ends normally removes and a killed one leaves, some with a multi-GB `target/`:
the run's extracted snapshot (`source-*`), cargo-mutants' build copies of it (named after it,
`cargo-mutants-source-*.tmp`) and the extra pass's copy (`extra-mutants-*`). Refuses anything
that is not such a copy. Run only while no mutation run is going: a live run builds in one of
these.

Run: python scripts/rm_mutant_copies.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import mutants  # noqa: E402

PATTERNS = ("source-*", "cargo-mutants-*.tmp", "extra-mutants-*")


def copies(folder: Path) -> list[Path]:
    """The run copies under `folder`, each checked to be a workspace copy."""
    found = sorted({path for pattern in PATTERNS for path in folder.glob(pattern)})
    for copy in found:
        assert copy.is_dir(), copy
        assert (copy / "Cargo.toml").exists(), f"{copy} does not look like a workspace copy"
    return found


def main() -> int:
    found = copies(mutants.LOCAL_TMPDIR)
    for copy in found:
        mutants.remove_tree(copy)
        print("removed", copy)
    print(len(found), "copies removed from", mutants.LOCAL_TMPDIR)
    return 0


if __name__ == "__main__":
    sys.exit(main())
