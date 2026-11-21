"""Print `CHANGELOG.md`'s section for a version, its heading left out: the notes the release
workflow gives a release's draft, above which the maintainer writes the introduction.

    python scripts/release/release_notes.py <version>

Refuses, with one `error:` line and exit 1, a version the changelog has no section for, or
one whose section is empty.
"""
import sys

from release import ROOT, Refusal, changelog_section


def main() -> int:
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    try:
        sys.stdout.write(changelog_section(changelog, sys.argv[1]))
    except Refusal as refusal:
        print(f"error: {refusal}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
