# Format census kit

A format crate's census (RUNBOOK.md "Closing a phase"): every file of that format on the
maintainer's machine run through the crate's read, its semantic round trip and `check`, tallied
by outcome, each failing class diagnosed, and the census re-run after every fix with a
regression check. These three scripts are the parts that do not depend on the format:

| Script | Does |
|---|---|
| `list_files.py "<Everything search>" <out.txt>` | the input list, from the Everything index (archives are opened by the run, not listed here) |
| `summarize.py <outcomes.tsv>` | the tally: files per outcome class, the smallest of each first |
| `regress_check.py <before.tsv> <after.tsv>` | after a fix: no file that read `ok` before reads otherwise now, and every outcome change |

The run itself is the format's own: a test binary or CLI command that reads each listed path and
writes one `outcome<TAB>size<TAB>path` row per file to an outcomes TSV, kept in the census's
scratch folder (`.tmp/<step>/…_census/`) with its probes. The first census, PES 2017's `.model`
files at 2.20g, ran this way; `scripts/library_parity/` is the export upgrader's own census, with
its classes and archives.
