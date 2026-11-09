"""The review's honesty and design sweeps (RUNBOOK.md "Lead and sidekick") over the added lines
of a diff file: every added line matching one of the patterns, with its file. Read-only; a hit is
a line to look at, not a finding.

Run: git diff <base> > .tmp/review.diff && python scripts/review/review_sweep.py .tmp/review.diff
"""
import re
import sys

HONESTY = [
    r"#\[allow", r"#\[expect", r"#\[ignore", r"\.ok\(\);", r"let _ =", r"drop\(",
    r"unwrap_or_default", r"unwrap_or\(",
]
DESIGN = [
    r"impl .* for", r"dyn ", r"_ =>", r"\.clone\(\)", r"\.to_vec\(\)", r" as [a-z]",
    r"== Engine::", r"!= Engine::",
    r"\bRed\b", r"\bBlue\b", r"reference", r"add-on", r"Python", r"texconv", r"always",
    r"^\+\s*pub ",
]

diff = open(sys.argv[1], encoding="utf-8").read()
current = ""
for line in diff.splitlines():
    if line.startswith("+++ "):
        current = line[6:]
        continue
    if not line.startswith("+"):
        continue
    for kind, patterns in (("honesty", HONESTY), ("design", DESIGN)):
        for pattern in patterns:
            if re.search(pattern, line):
                print(f"{kind} [{pattern}] {current}: {line[1:].strip()[:150]}")
