# Library parity

The measurement every phase close that changes the Team compiler or the Export upgrader reruns
(`docs/spec/team_compiler/testing.md`, RUNBOOK.md "Closing a phase"): every old (AET) export on
the maintainer's machine is upgraded (the **upgrade census**), and each one that upgrades clean
is compiled, its CPKs' entries listed by name, size and hash and compared with the last run's
list (a **checkpoint**), every changed entry traced to a commit of the phase or fixed. It is
not a test: the inputs are members' exports on one machine, and judging each change is the
point of the run.

Red is no longer part of a rerun. The **library run** against it (each clean input also
compiled by Red, the two outputs compared entry by entry, every difference attributed to a
class of `classes.md`) is kept for a question about what the old tool did: "A run against
Red" below.

Machine paths (Red's checkout and commit, 7-Zip, Everything, the excluded trees) are in
`config.py`; every output goes under `.tmp/library_parity/` (`config.WORK`). Python 3.12 or
later, with `PYTHONUTF8=1` set (export paths hold characters the Windows console's code page
cannot print); Windows, for Red. Each script's docstring has its exact usage.

## A rerun, in order

1. **Build** the release binaries: `cargo build --release -p studio`,
   `cargo build --release -p team_compiler --example red_parity` and
   `cargo build --release -p cpk --example cpk_list`.
2. **Find the inputs** (only when the library changed; the lists are reused otherwise):
   `find_folders.py` (Everything, seconds) writes `census/folders.tsv`; `find_archives.py`
   lists every archive Everything knows with 7-Zip into `census/archives.tsv`.
3. **Upgrade census**: `census.py target/release/utw.exe <tag>` writes
   `census/outcomes_<tag>.tsv`, `reports_<tag>.txt` and the tally `census_<tag>.txt`.
   `regress_check.py <previous tag> <tag>` lists every input whose outcome changed and the
   finding lines it lost and gained; each change is diagnosed (`analyze.py <tag> <code>` groups
   one code's findings by kind). An exit-1 run must have written nothing.
4. **Checkpoint**: `checkpoint.py write <tag> <commit> --census
   .tmp/library_parity/census/outcomes_<tag>.tsv`, `<commit>` being the one the binary was
   built from, copies the binaries into `library/bin/` (remove that folder by hand for a new
   build) and writes `checkpoints/<tag>/`: a row per clean input in `rows.tsv` and its CPK
   entries in `<n>.tsv`. The logs stay in `library/jobs_chk_<tag>/`.
5. **Compare**: `checkpoint.py diff <previous tag> <tag>` writes
   `checkpoints/diff_<previous tag>_<tag>.txt`: the inputs only one checkpoint holds, those
   whose upgrade or compile outcome changed, and every entry added (`+`), removed (`-`) or
   changed (`~`). Each is traced to a commit between the two checkpoints' `commit.txt` that
   meant it, or is a defect and becomes a worklog step. To look into one, build the previous
   checkpoint's commit in a worktree and compile that input with both builds (`library.py`'s
   `--indexes`, `--bin` and `--keep` do this with Red beside it). The summary of the
   differences goes into the commit that closes the phase.

The first checkpoint, `p11`, is the output of the last run against Red (commit `3c512628`),
written with that run's frozen binaries (`--bin bin_p11 --frozen`).

## A run against Red

Only for a question about what the old tool did. Steps 1 to 3 as above, then:

4. **Library run**: Red's checkout must be at `config.RED_COMMIT` on `main` with a clean tree
   (`library.py` checks). `library.py <tag> --census .tmp/library_parity/census/outcomes_<tag>.tsv`
   copies the binaries into `library/bin/` (remove that folder by hand for a new build), and for
   each clean input writes `library/jobs/<n>/` and a row of `library/rows_<tag>.tsv`. About 20
   minutes of Red and 17 of ours over 250 exports, three workers. `--indexes` reruns a subset,
   `--jobs`/`--bin` keep a rerun apart from an earlier one, `--keep` keeps both outputs.
5. **Attribute**: `attribute.py .tmp/library_parity/library/rows_<tag>.tsv` (`LIBRARY_JOBS` naming the jobs folder
   when it is not `jobs`, `ATTR_TAG` a suffix for its outputs) writes `attribution.txt` and
   `unattributed.tsv`. `tally.py` gives the raw `red_parity` classes. Every unattributed line,
   `I14?` and `warning-slot` line, and every export under a D class, is diagnosed: a defect
   becomes a worklog step, an intentional difference a new row of `classes.md` and a rule in
   `attribute.py`.
