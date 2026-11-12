# Runbook

How the lead runs the project's machinery: the order of the work and its checkpoints, mutation
runs and the VPS that shares them, the scratch
folder, the agents and their harness limits, how phases and releases open and close, briefing
and reviewing a sidekick, and the full rules of the review rounds. `AGENTS.md` keeps a summary
of each, and its rules stay in force. Read a section before starting what it covers.

## How the work runs

**The release path first.** The lead's handover opens with the next release's blocking path:
the open steps the release cannot go out without, in order, and who is on each. A sidekick
is on the first of them before anything else starts; review loops, small asks and anything
the maintainer marked low importance fill the time while those sidekicks run, and never
take their place. We do not pick by what is convenient to brief: a low-importance ask and
three review loops once ran before the release's largest open step was started.

**A checkpoint is one command.** `just checkpoint` runs `gates-changed` and `mutants-diff`
since the last gated commit and then moves that commit, the git ref
`refs/checkpoints/gated`, to the commit it started on (`scripts/gates/checkpoint.py`; a
clone seeds the ref once, as the script says). The gates read the working tree, so a commit
or an edit made while they run leaves the ref where it was, and the checkpoint is run again.
It runs at the end of a review loop, at a worklog step's close, after a merge, and before a
push is asked for. `just checkpoint-status` says how far HEAD is past the gate, what is not
pushed, and what CI made of the pushed head: the lead runs it after every push the
maintainer makes, and a red run is looked at before other work. The handover's sync stamps
the same lines into the handover and ends with an error once more than ten code commits are
past the gate. We do not keep the last gated commit as a typed note, nor the two runs as
separate steps to remember: three loops once closed with neither run, and a push went red on
a broken pointer.

**The checkpoint runs beside the work, not in front of it.** `just pipeline` runs the
checkpoint and then `just gates-remote` on the commit main's HEAD is on, from the gate
worktree (`../4cc-studio-temp/gate`, with its own `target/`), so main's tree stays free:
the lead commits, merges and briefs the next slice while it runs, and reads its log when it
ends (`scripts/gates/pipeline.py`; started detached and logged, one run at a time). A pass
moves the same ref. A failure is fixed by a commit on top and the next run gates it, as a
red CI run is handled. We do not run `just checkpoint` in main's own tree while there is
other work to do: it froze the tree for the gates' length after every slice, with the
Linux gates' wait behind it, and the next slice stood still for both. What still waits for
a green run is the sentence "main can be pushed": the last run must have gated the latest
commit that touches code, on both systems.

**One home per event.** What happened is written once:

| Event | Its one home |
|---|---|
| A reviewer's concern and its ruling | the rulings file (`.tmp/duck_rulings.md`); the worklog step gets the round's name and its accepted count, nothing more |
| A choice made where the plan was silent | the plan edit and its `DECISIONS.md` entry |
| A step's state | its worklog line |
| What a commit did and why | the commit message; `docs/LOG.md` is frozen at 2026-11-11, and the dated log since is `git log` |
| What the maintainer should read now | the turn's report (and `.tmp/report.md` in an autonomous run) |
| What the next session does first | the handover, which copies no rule and no state that has a home |

**A step collapses when it closes.** A closed worklog step is one line: its ID and title,
the date, the landing commit, one clause of what it delivered, with its `manual: checked`
lines under it. The story of how it got there (attempts, rounds, findings) is in git and in
the rulings file. An open step keeps what is left to do and what its doer needs. We do not
wait for the phase's close to collapse: with Phase 8 open for weeks the file reached 853 KB,
and "where we are" could only be searched.

**Looks owed are one list.** Every check that only the game or the built window can make,
and that is still owed, is a line of the worklog's "Pending looks" section, ordered by what a
wrong result would cost, each saying whether the in-game harness can make it (`scripts/ingame/`,
with OBS when the maintainer has not set it off-limits) or the maintainer must. A step or a
decision that says "not checked in game" adds its line there in the same edit. The lead
sends what the harness can do to a SWE-2 in-game run whenever the list holds a
game-facing write, and hands the maintainer the rest as one batch per game session.

**Sidekicks in parallel.** Two or three sidekicks at once is the normal state, on steps
that own disjoint files, the release path's first. The limits are the disk (a worktree that
builds grows a `target/` of about 40 GB; one that only edits documents or scripts builds
nothing and costs nothing) and SWE-2's hidden limit (two runs at once). When the Claude
weekly window nears its limit, the sidekick work goes back to SWE-2.

**The accounts' state has one home**: `~/.claude/skills/duck/accounts.md` (which login runs
the lead, each sidekick and each reviewer, the limits, floors and resets). This file, the
handover and the lead's memory point at it and copy nothing from it.

**Counts over the real exports come from the kept index.** `scripts/census/` builds an index
of the export library's textures and model references in `.census/` (gitignored, and never
emptied by a disk cleanup) and answers a question about them in seconds, each count beside a
control count (`scripts/census/README.md`). A count that a spec section, a decision or a
ruling cites is cited as its query, not as a scratch file: three such files were gone when
they were next needed.

## Mutation runs and the VPS

**Whole-crate mutation runs go to the VPS alone.** A whole-crate run (`just mutants
<crate>`, its extra kinds included) runs every mutant on bonfire, nothing on this machine:
it is medium priority, so a slower result (about 2.5 times the split's: `save_editor`'s
1990 mutants ran about 27 an hour here and 17 there) is fine, while a long local run blocks
in-game checks on the Fox versions and a restart of this machine for an update, and a power
failure here kills hours of a local half's progress, while the VPS rarely goes down.
The run refuses when `STUDIO_MUTANTS_REMOTE` names no host, and for a crate in
`LOCAL_ONLY_CRATES`; its extra kinds run on the VPS in the same service before
cargo-mutants and their results come home with the fetch. It holds the run lock only
while it launches and while it fetches and summarizes, never while it waits
(`just mutants-collect` waits the same way), so `just mutants-diff` runs while a whole
run is under way: it keeps the split below, a round's diff being minutes and its result
waited on, except while a mutation half is on the VPS (running, or ended and not collected)
or the VPS cannot be reached, when it runs entirely on this
machine instead of waiting days for its turn there. What follows
describes the split: a diff uses it, and the whole-crate file split it describes is kept
for when a whole run needs this machine again.

**The split with the maintainer's VPS.** On the maintainer's PC the
user environment variable `STUDIO_MUTANTS_REMOTE=bonfire` (an alias in `~/.ssh/config`) makes
`just mutants-diff` run part of a larger crate's mutants there (`scripts/mutation/mutants.py`: the working tree is
sent as a git bundle, the remote half runs at low priority, its results land in
`mutants.out/remote/` and the combined survivors in `mutants.out/missed_all.txt`). A whole-crate
run hands each side a complementary set of the crate's source files rather than `--shard`
halves: the local config's `examine_globs` name this machine's files and the remote's
`exclude_globs` add them, the local share being `remote / (local + remote)` of the two sides'
measured mean seconds per mutant (the cost table, below; the remote side's means fold into
its `remote_mutant` table at each fetch), so a slower side tests less. A diff run, a crate one
side has not measured yet, a file assignment landing further from the target than halves, and per-side
listings that do not recombine to the whole listing exactly all keep the even `--shard 0/2` /
`1/2`, and `extra_mutants.out/split.json` records which a run used so the summary can check the
merged outcomes name every listed mutant exactly once. Benchmarked
on 2026-09-27 on one commit, the VPS is about as fast as the PC (`fox2` whole crate 167 s
against 256 s, one eighth of `fmdl` 460 s against 433 s) with identical results, so a split
roughly halves a whole-crate run whose sides rate the same, and it moves part of its energy off
the home machine either way.
Agent and IDE shells started before the variable was set do not see it, so the script also
reads it from the user's registry environment; the run's first line says which mode it took
("splitting the run with bonfire", "running every mutant on bonfire" or, from `mutants-diff`,
"running here"). An agent
shell without the variable once ran all 806 `pes_savefile` mutants locally, over three
hours. An empty value in the process opts out. `mutants-diff` runs one cargo-mutants per crate
the diff mutates, one after the other, each with its own baseline and so its own timeout, not
one run for the diff: one run's baseline tests every crate it mutates, and at `mutants-diff
95f20eac` (`music_export`, `pes_savefile`, `save_editor`) a `music_export` mutant that hung a
loop waited 751 s here and about 1100 s on the VPS where its tests take 1 to 2 s; nine timeouts
cost about 2.5 h of job time, more than the two machines' speed difference. Each crate's
results move to `mutants.diff/<crate>/` and their survivors are listed together in
`mutants.diff/missed_all.txt`; a crate killed mid-run leaves its results in `mutants.out/` for
`just mutants-collect`, and the crates after it did not run. Each crate's run splits the same
way only when its estimated local time is at least 4 minutes (`SPLIT_THRESHOLD_SECONDS`): the estimate is the
unmutated baseline plus each mutant's crate's measured mean seconds per mutant over the two
jobs (the cost table, `%LOCALAPPDATA%/4cc-studio/mutants-cost.json`, `~/.cache/4cc-studio/` off
Windows, refreshed by every local run; one per user rather than per tree, so the main tree and
the mutants worktree share it and deleting a `target/` loses none of it; a never-run crate falls back to
its size, a never-measured baseline to 60 s), because cost follows a crate's test suite, not
the mutant count (0.7-1.4 s for most crates, 33-37 s for `pes_savefile`). The baseline is a
cold build in a fresh copy of the tree, about a minute, and the mean counts each job's cold
first build and every timeout: an estimate that left all three out put a 38-mutant diff at
80 s, and it took 270 s (the estimate now says 266 s). That estimate is for the whole run
here; a split run prints its own, the baseline plus half the rest (each half builds a
baseline, so a split does not halve the estimate), and ends with its wall time, which adds
the transfer, polling and fetch. The local half runs at below-normal priority with each of
its two cargo processes given half the logical CPUs (`sized_config`, the remote half's
sizing), not with two CPUs held back: the machine stays usable by priority, and the run gets
what the user leaves idle (measured level with the old cap). A starved test can then time
out, so a timeout that no infinite loop explains is rerun before it counts as caught, unless
its log (`mutants.diff/<crate>/log/`) already shows tests reported `FAILED`: the mutant is
then caught, and the run only outlasted its limit because several failing tests each waited
out a bounded wait (the update dialog's tests wait up to 10 s for a frame, and a mutant of
`updater::install` fails a dozen of them under a 20 s limit). A build
killed for memory (the remote half's cap) is filed by cargo-mutants as unviable, which hides
an untested mutant: eframe's dependency tree once outgrew a 6 GiB cap and 12 remote mutants
went untested that way, so the scripts list every killed build and fail the run. A failed
link counts the same way: a Windows SDK update once removed the system libraries mid-run, and
every local mutant of a diff run was filed unviable. Below
that the split's fixed overhead, about a minute plus up to 15 s of polling, eats the gain. At
2.20k a 32-mutant `pes_savefile` diff took 11 min 57 s locally; split, the local half took
417 s and the remote half 502 s from its launch (the VPS ran about a fifth slower; the total
wall time was not recorded). Manual ssh from
Git Bash uses `/c/Windows/System32/OpenSSH/ssh.exe bonfire`: Git's own `ssh` cannot reach the
Windows agent that holds the key. **A run snapshots the working tree when its command starts**
(a git tree object, `mutants.working_tree`): the remote half runs on a transfer of it, and the
listing, the config and the local half read a copy of it extracted under the temp folder
(`mutants.snapshot_source`), never the working tree, so
edits and merges are safe once the run has started, every crate of one `mutants-diff`
included. **One run at a time, each queued with its snapshot**: `just mutants`,
`mutants-diff` and `extra-mutants` snapshot first, then take the user's lock (`mutants.lock`
beside the cost table, so a worktree's run queues too) for the run's whole length
(`mutants.run_lock`; a whole run and `mutants-collect` let go of it while they wait on the
VPS, above), so a run launched while another is under way waits
its turn with the tree it was launched on, and edits and merges are safe while it waits. Runs
are launched as separate commands, never chained: a chain's later run snapshots only when
the one before ends, and a run queued that way once snapshotted a half-edited crate and had
to be stopped. A run sets the last one's
results aside (`mutants.out.old`, `mutants.diff.old`) rather than deleting them. The lead
launches a long run detached, with
PowerShell `Start-Process bash.exe <script> -WindowStyle Hidden` from a script under `.tmp/`,
never with the Bash tool's background mode: its two-hour cap kills the run, and stopping it
kills the console the run's children need, after which every build fails with `0xC0000142`
and cargo-mutants files the mutant as unviable (once 273 of 274 in 23 s).

**When the maintainer's PC is suspect, the heavy runs go to the VPS.** Four
`MEMORY_MANAGEMENT` bugchecks there (the last during a run, all four in `rustc.exe` at one
kernel site) once moved them until third-party drivers were removed; the switch stays for the
next time, and is off now. While the switch is on, the lead's checks run through `just remote`
and `just gates-remote`, and a sidekick's `cargo check`, test and clippy runs all go through
`just remote`; only `cargo fmt` stays local.
`REMOTE_ONLY = True` in `scripts/mutation/mutants.py` makes
`just mutants-diff` run every mutant on the VPS as one whole run, as `just mutants` always does (no
local half, no split threshold; the last local `mutants.out` set aside as
`mutants.out.old`), and `just gates-remote` runs `just gates` there against a snapshot of
the working tree (`scripts/mutation/remote_gates.py`: the same transfer and clone, so it refuses
while a mutants half is running or uncollected; the transient service `studio-gates`,
memory-capped and CPU-idle, run synchronously). The VPS's `just` is installed in
`~/.cargo/bin` at this machine's version, as cargo-mutants is, and so is `cargo-about` 0.9.2
(the release workflow's version, `--features cli`), so `just remote just release <version>`
packs a Linux bundle there, into the clone's `target/dist`. A bundle built on the host
asks for a newer C library than the release workflow's (glibc 2.39 where the workflow
builds on Ubuntu 22.04), so it serves a test, never a release. Its Python has Pillow from
Debian's `python3-pil` package (11.1.0 on Debian 13, where CI installs the pinned 12.3.0
through pip): `just test` runs the drafts tooling's tests, which read images through it, and
without it the recipe stopped there with the tests after it unrun. The lead uses
`just gates-remote` for its review gates while the switch is on, and a sidekick's runs go
there one command at a time, `just remote cargo test -p <crate>` (`cargo` or `just` only; a
brief says so, and that the run refuses while a mutation half or other gates use the clone,
to wait and retry). Measured on 4.y-sub (b3b4a): gates 4 min 11 s cold (peak 8.7 GiB),
1 min 23 s incremental; a 15-mutant diff 330 s wall. The switch is a constant, not an
environment variable: a shell started while the variable was set kept it after it was
deleted, and would have kept running every mutant remotely.

**The VPS's production Fluxer instance comes first.** The remote half runs as the transient
system service `studio-mutants` (`sudo -n systemd-run`) with no swap and a `MemoryMax` of the
host's available memory at launch less 1 GiB (`REMOTE_MEMORY_RESERVE`; printed as `remote
memory cap`, and `just gates-remote`'s unit gets the same), not a constant raised by hand each
time the maintainer frees memory on the host: a measured cap takes what is free without asking
and leaves the host its margin. Each remote cargo process also builds with few jobs
(`REMOTE_BUILD_JOBS`, 1 since a run with more hit the cap). So a runaway mutant is OOM-killed
inside the unit and counts as caught; `OOMPolicy=continue`, so that kill does not stop
cargo-mutants; `CPUWeight=idle`, because `nice` cannot keep a `user.slice` process off
Fluxer's CPU under cgroup v2. An uncapped half once filled the host's RAM and swap
(`display_name` with `i *= 1`, a loop that grows a `Vec` forever), and the global OOM killer,
which could as well have picked Postgres, took the test. The run prints its peak (`remote
memory peak`) against its cap. Every split run peaks at the cap, which is the cgroup's page
cache being reclaimed, not a kill, so a peak near the cap changes nothing by itself. When a
build is killed at the cap, the run fails and lists the untested mutants (`report_killed`),
and the fix is then memory freed on the host (the maintainer's) or a lower
`REMOTE_BUILD_JOBS`; past one job, the crate joins `LOCAL_ONLY_CRATES` in
`scripts/mutation/mutants.py`, whose mutants never go to the VPS (a whole-crate run of one
refuses, and a diff's mutants of such a crate run here). The remote half builds
under `~/studio-mutants/tmp` on disk (`REMOTE_TMPDIR`), not in the host's `/tmp`: that is a
7.8 GiB tmpfs, cargo-mutants' default place for its two copies of the tree, so each copy's
`target/` (4 GiB with egui linked) lived in the unit's memory and the per-mutant rebuilds of
1-2 GiB tipped it over the cap. The build copies' size, not the build's memory,
is what to look at first when a killed build shows up again. A running half is stopped with
`sudo systemctl stop studio-mutants` on the host. The lead stops one with exactly
`/c/Windows/System32/OpenSSH/ssh.exe bonfire sudo -n systemctl stop studio-mutants`, nothing
chained (the only form the permission classifier allows; if it refuses, ask, never work around
it), then `just mutants-collect`. The VPS itself is never stopped: it hosts a production
service.

**The remote half runs detached** from any ssh session (`~/studio-mutants/run/`: `job.sh`,
`pid`, `log`, `exit`, `memory_peak`), and the script polls it every 15 s with short ssh calls.
A dropped link costs a poll, not the run: a remote half that hung off one long ssh session
once died with it. If `just mutants` itself stops, `just mutants-collect` waits for the remote half
and fetches it, including a died half's partial results. It is run from the tree that
launched the half, whose `.tmp/mutants-remote-pending` says what the run was: from another
tree it summarizes the remote half alone and says so. A run that uses the VPS refuses to start while a
remote half is running or uncollected, or while `just gates-remote` runs there: both rewrite
the remote clone. A run on this machine alone refuses only while this tree's launch record
(`.tmp/mutants-remote-pending`, kept until the fetch) names a half whose run also had a local
half, since that collection summarizes the local `mutants.out`; a whole run's has none, and
the record is read with no call to the host, so `STUDIO_MUTANTS_REMOTE` set empty changes
nothing. **A failed half is reported, never restarted:**
`cargo mutants` does not resume, so a rerun starts from zero, hours of work (a sidekick once
restarted the VPS half and waited on it although the local half had already tested every
mutant). A brief that starts a whole-crate run says so.

**An agent starts the local half detached too:** `python ~/.claude/tools/run_detached.py <log>
just mutants-diff ...`, one command per run, never as the harness's background command. The
harness ends a background command at two hours, and the processes it started with it: a
`mutants-diff` chained behind a long one under one background command once died at that cap
with its local half 25 minutes in, and the run started again from zero. A detached run
outlives the harness and ends its log with `exit N`; the lead waits on that line.

## The scratch folder

**The session scratch `.tmp/` holds one folder per subphase, inside its phase's folder**
(`phase_04/4_17/` for every 4.17 slice's briefs, reports, diffs, logs and probes; a release's
steps under `release_0_0_1/`, `release_0_0_2/`; a named folder of no one step in its phase's
folder when it has one, else in `topics/`), because a flat top level once reached about two
thousand files and some 250 step folders made the IDE's folder listing hang: a new scratch
file goes in its step's folder, and a new step's folder in its phase's. The top level keeps
only what a path names from outside: `lead/` (the lead's live notes and handover),
`sidelead/` (the sidekick launcher's), `reference/` (cup material), the folders scripts
default to (`ingame/`, `library_parity/`, `draft_shots/`,
`draft_compare/`, `0_0_1s4t_v2/app`, the built-window copy), the two files the lead's tools
append to (`report.md`, `duck_rulings.md`),
and a folder a running job is writing until the job ends. **`.old/` holds what is done
with**: a closed phase's folder whole, a topic untouched for weeks; it is the first thing
deleted when the disk needs room. A pointer into `.tmp/` in a document is a historical
note, never kept current: what matters past its step is in the documents, and scratch is
scratch. A folder's name being unique, a stale path still finds it (`.tmp/4_17/` is
`.tmp/.old/phase_04/4_17/` now) through the Everything index. **A temporary folder outside the repository** (a git worktree for a review, a
side-lead, a release or in-game build) goes under `../4cc-studio-temp/<name>`, never loose beside
the repository: the maintainer browses that folder, and seventeen `4cc-studio-*` siblings once
buried the repository. **A slice's worktree and branch go once it is merged and its review
loop has ended**, its `.tmp/` first copied to main's `.tmp/worktrees/<folder>/`: never pruned,
they once reached 168 worktrees and 171 branches, every `git worktree list` and branch picker
a page of finished work. Left alone
by every agent: `review`, `review3`, `review5`, `review6`, `release`, `wt-m`, `ingame`, and
`wt-cand-e`, the release staging worktree (its `CARGO_TARGET_DIR` is main's target). The
history was reworded and force-pushed on 2026-11-04 (old refs in `refs/archive/pre-reword/`):
a clone or worktree made before holds the old history, so it refetches and never merges
it back. **A script likely to be run again
goes in `scripts/`, not `.tmp/`**: a one-off probe stays in
its step's folder, but a tool the next check will reach for (the built-window harness,
`scripts/built_window/`, had served every check for weeks from three scratch folders) gets a
folder or file under `scripts/` with its usage in the docstring, its paths from the repository
root or an environment variable rather than a scratch path, and a line here or in the folder's
`README.md`. The agents' own tooling is not the project's and stays out of the repository, in
`~/.claude/tools/` on the maintainer's machine: the below-normal build wrapper, the safe amend,
the sidekick launcher and its waiter, the detached runner, the context and disk watchers, the
handover's stamp and the review-cost readers; each takes the repository from the directory it
runs in. A release candidate takes two: `scripts/release/build_remapped.py` builds the release exe
with `just release`'s flags and checks it leaks no local path, and `scripts/release/make_candidate.py
<commit>` stages the unpacked bundle from it in `../4cc-studio-temp/candidate-<version>`;
`scripts/drafts/sgrep.py <file> <regex>` greps a draft past its embedded data rows. A review has
`review_sweep.py <diff>` (the honesty and design sweeps' patterns over the added lines),
`pure_move_check.py <base> <dir>` (a move changed no code line), `clones.py` (repeated code
shapes, for the design-health pass) and `acceptance_ids.py <ID or prefix>...` (a scenario's
definition, citations and manual checks); a release, `win7_imports.py <exe>` (imports Windows 7
lacks) and `bloat_compare.py <old> <new>` (two `cargo bloat --crates` tables); the bundled fonts,
`cut_flex.py` and `symbols_used.py` (`resources/fonts/README.md`); a save or a binary,
`unwesys.py` (inflate WESYS-wrapped files) and `strings.py`; a capture, `crop.py`; a console
check, `conpty_run.py` (runs a command under a real terminal). Each says its usage in its
docstring. `.tmp/` is ignored by git: nothing there is reviewed, versioned or found by the
next session except through a note that names it. **An archived run keeps its summary, not its logs.** A run kept aside (in
`.tmp/<subphase>/mutants_<step>_<what>/`, before
the next run overwrites `mutants.out/`) copies only `*.txt` and `outcomes.json`, the local
half's from `mutants.out/` and a split run's remote half's from
`mutants.out/remote/mutants.out/` (`mutants.out/remote/` itself holds only the console log: a
chain that copied from there kept no remote summary), never the per-mutant `log/` trees: those
are what reproduce a survivor, and a survivor is reproduced by re-running its mutant, not by
reading a week-old log. Whole-folder copies once grew `.tmp/` to 17 GB. Each local run
extracts its snapshot to `%TEMP%\studio-mutants\source-*` and builds in
`cargo-mutants-source-*.tmp` copies of it (one fixed parent, `LOCAL_TMPDIR`, so one antivirus
exclusion covers every run), the extra pass in `extra-mutants-*`, all deleted when the run
ends normally; a killed run leaves its copies behind (some with a multi-GB `target/`), so
after stopping one, delete them (`scripts/mutation/rm_mutant_copies.py`, which refuses
anything else). `.cargo/mutants.toml` sets `gitignore = true` so the copies leave out `.tmp/`
and `target/`: without it each copy carried all of `.tmp/`, and two runs died of a full disk.
The logs alone are large: a `pes_savefile` whole-crate run leaves about 20 GiB in
`mutants.out/` (5.6b: 8.9 GiB local, 11 GiB fetched from the remote half), which the next run
moves to `mutants.out.old`.
When C: fills, a finished tree's `target/` can go whole, and the main tree's `target/debug`
once nothing builds there (254 GiB on 2026-10-26, mostly stale `deps`): cargo rebuilds them,
each run rewrites its generated config, and the cost table lives outside every `target/`.

**On the VPS the project leaves the host 2 GiB of free disk** and may hold the rest: the
host's production service runs but is used by nobody, so the 40 GB the project's folder was
once held to no longer applies, and nothing bounds the folder itself. Every run that uses the
host passes a guard first (`remote_refusal`):
it empties `~/studio-mutants/tmp` (`REMOTE_TMPDIR`, the remote tests' temp dir), where a test
that removes its folder at its end leaves it on every caught mutant and a killed half leaves its
tree copies, then refuses when the disk has under
26 GiB free (`REMOTE_FREE_FLOOR`: a half's two build copies take 24, measured on a
`save_editor` half, whose temp folder peaked at 23.6 GiB, and 2 stay for
the host's system). The refusal prints what `~/studio-mutants` holds: the cleanup below frees
the clone's build cache, usually most of it (a finished run's results stay until the next
launch), and when that cannot lift the floor the maintainer is asked.
Before the guard, the folder once held 50 GB (28.6 of them test leftovers, 27 the Save
editor's pid-named copies of a fixture save), the disk was at 99% and a half lost five
mutants. **At each phase's close**, after its last mutation run, the lead
removes the clone's build cache, `~/studio-mutants/tree/target` (22 GB after two phases; only
`just gates-remote` builds there, which then starts cold) with `scripts/mutation/remote_clean_target.sh`
(its header has the two ssh lines: a dry run first, then `--delete`); it refuses while a half,
the gates or a cargo process runs. The same cleanup frees the folder whenever the guard refuses
between closes (2026-10-29: 35 GB, the disk from 13 to 47 GB
free). Claude Code's auto-mode classifier refuses the two ssh lines unless the project's
`.claude/settings.local.json` allows them, which it does on the maintainer's machine, each
exactly as the header writes it.

## Agents and harness

Three model families, three roles. Lead: Claude, in Claude Code or in the Devin CLI.
Sidekicks: Claude Opus through headless Claude Code for a real slice, SWE-2 (Kimi lineage)
through the Devin CLI for a small contained one and for in-game runs (the `sidekick` skill
launches both); a cold probe on a spec-in-hand crate showed SWE-2 follows `CONTRIBUTING.md`
literally and reports plan gaps instead of deciding them, so it is trusted with
implementation, not with judgment. Reviewers: SWE-2, Gemini 4 Carbon and GPT-6 Astra, run
through the `duck` skill ("Review rounds"). A Devin lead's reviewer subagent profiles (global,
`%APPDATA%\Devin\agents\`): `gpt-astra-high` (GPT, latest Astra) when the lead is a Claude, **or
when you do not know what model you are**; `fable-medium` (Claude) only when you know you are a
GPT. The reviewer reviews; it is never asked to edit.

Two limits of the Devin harness, both measured. **`run_subagent` profiles have no `skill` tool**, and the
reviewer profiles (whose files list `allowed-tools`) have `read`, `grep` and `find_file_by_name`
only. A brief that says "invoke the `karpathy-guidelines` skill" sends a subagent globbing the
repo for a file that is not there; give it the file to read instead,
`%APPDATA%\devin\skills\karpathy-guidelines\SKILL.md` (outside the repo, and its `read` tool
reaches it). The `sidekick` tool has the `skill` tool, so its briefs keep the invoke wording.
**The sidekick's role can be played by a `swe-2-high` subagent** (same model, profile file with
no `allowed-tools`, so it has the shell, `edit` and `write`; measured 2026-10-05): run it in the
foreground (a background subagent has unapproved tools denied), resume the same one per task
(`resume: <agent_id>` keeps its memory, as the sidekick's handoffs do). SWE-2 has a 1M-token
context (a post-trained Kimi K3). A review over a wide surface is still split into two briefs
run in parallel, because one round over a whole module runs well over an hour. The `sidekick`
tool's single sidekick cannot be reset. **Compaction does not fire while a sidekick handoff or
a background subagent is in flight**, so a run started "to save time" ahead of the
`[[/compact]]` sentinel costs the compaction instead. Compact first, then start the run in the
resumed turn.

The Devin IDE compacts the sidekick's context by itself when it reaches 500K tokens, so a
full sidekick context needs no action from the lead; a brief still stands alone (file under
`.tmp/`, read in full), because a compaction may fall between two handoffs.

**A Devin CLI run is watched for stalls.** A Claude lead runs SWE-2 through
`devin_sidekick.py` (implementation) and `duck.py --reviewer swe` (reviews) as background
commands, and the harness tells the lead only when a command exits; a run whose session goes
quiet mid-turn never exits. A rework's resumed session once went idle in the middle of a red
run, its hand perturbation still applied, and only a check by the lead found it; the
background command's 2-hour cap would have ended it nearly two hours later. So every such run
gets a second background command beside it, `python ~/.claude/skills/duck/devin_watch.py
<brief>` (timeout 7200000): it exits quietly when the run ends, and with `STALLED` (exit 3) when
for 20 minutes neither the session's `last_activity_at` in Devin's `sessions.db` moved nor the
run's process tree burned CPU (a build or test burns it without any step). The watcher never
stops the run: what to do is the lead's call, after reading `git diff`, since a stalled run can
leave a half-done edit or a perturbation in the tree. A
session that stalled after its one allowed resume is not resumed again; the lead finishes the
brief's remaining items itself or briefs a fresh session.

**A Devin CLI run is started detached**: `python
~/.claude/tools/run_detached.py <brief>.run.log "git add -A && python
~/.claude/skills/sidekick/devin_sidekick.py ..."` (or `duck.py`), with a background waiter beside
it (`until grep -q "^exit " <log>; do sleep 60; done; tail -40 <log>`) as the notification, and
the stall watcher as before. A background command dies with the lead's session and at the
2-hour cap; a
detached run outlives both, and only its waiter needs re-arming. It runs at below-normal
priority, as `run_detached.py` starts everything. Stopping one is a `taskkill` of its process
tree, the lead's call as for a stall.

**In-game checks are run by a SWE-2 sidekick, not by the lead.** Driving a game is dozens of
screenshot-and-input turns, each one cheap for SWE-2 and costly for the lead, so the lead
does none of them. It builds the binary and compiles the CPK, then briefs the check like any
other step: the scenario or question, the game and its test slot, the folder the screenshots
go to (`.tmp/<step>/ingame/`), and, for each thing to look at,
what a pass and a failure would look like. The sidekick runs `scripts/ingame/` (its README is
the brief's reading): the pad and OBS only, never keys or the screen, which take the
maintainer's keyboard; `saves.py backup` before anything that changes a save and `restore`
after; the game closed and `install.py revert` run at the end, pass or fail. Its report gives,
per thing looked at, the verdict, the screenshot file that shows it, and what in the image
shows it. The lead then reads only those key images, and only where a verdict matters to
what it lands or is uncertain; the rest of the run stays unread. While the game runs, a
PES 18-21 check holds every cargo command on the machine (the lead's builds and gates
included); the brief says which game, so the lead knows when to wait.

**A `git commit` failing with "unable to write new_index file" is transient** on this machine:
check `git log -1` and `git status` (it may have landed), then run the same command again.

**Sidekicks in parallel worktrees never share a `target/`.** Each worktree's builds go to its
own `CARGO_TARGET_DIR` (`../4cc-studio-temp/wt-<x>/target`), at `CARGO_BUILD_JOBS=4`. Cargo names a workspace crate's build output by its path relative
to the workspace root, so every worktree writes the same file name for the same crate, and a
build in one worktree takes another's output as up to date: one `save_editor` build once
linked another worktree's `aatf` (a constant reported missing in a tree that has it). Cargo's
lock on a shared folder serializes builds but does not stop this, so a shared folder makes
every sidekick's test results suspect. Each folder costs a full build and its disk. The one
exception is the staging worktree (`wt-cand-e`), which builds a candidate into the main tree's
`target/` to spare a full build; afterwards the main tree's next build takes the candidate's
crates as up to date (main's `gates-changed` once failed to compile against the candidate's
`pes_savefile`). So after staging, touch every file `git diff --name-only
<candidate> HEAD -- crates Cargo.toml Cargo.lock` lists in the main tree before its next build
(`.tmp/lead/touch_diverged.py <candidate>`); the staging worktree also needs an empty
`target/` of its own (`WORKLOG.md` Issues).

**At most three cargo builds at once, on the whole machine**: a sidekick's, a mutation run's
local half, a lead's gates or package tests and a release build each count as one. The main
lead's side (its sidekicks, its mutation run, its own gates) holds two; a side-lead in its own
worktree holds the third, its sidekick's builds and its own together, so the two never need
to sync. With the Optane's 64 GiB swap on `F:` a third build does not risk exhausting memory,
though two already fill the CPU, so a third shares it rather than adding speed. Before
starting a build, count your side's; with no side-lead running, the main lead's side may use
the third for a program needed as soon as possible (a quick tool, a build for a manual check).

**Every build and test job runs at below-normal priority**: at normal priority two jobs left
the maintainer's machine barely usable, while a below-normal one yields every processor to
their own programs and slows only while they use it. The priority is inherited, so it is set
where a job starts: `scripts/mutation/mutants.py` starts cargo-mutants that way, the lead's
sidekick launcher starts its `claude` process that way (everything the sidekick runs
inherits it), and the lead starts its own gates, package tests and release builds through a
wrapper that does the same (`~/.claude/tools/below_normal.py <command>`). A job found at normal
priority is lowered with its whole process tree (`~/.claude/tools/lower_tree.ps1 -Match <text>`).
Priority alone proved too little: a test gate that filled all sixteen logical processors at
below-normal priority still slowed the desktop (2026-11-12). So the wrapper and the detached
launcher also keep their job off two logical processors, one core, which the job's
processes inherit as they inherit the priority (`~/.claude/tools/leave_cores.py`;
`lower_tree.ps1 -Match <text> -Free 2` for a job already running).

**A headless `claude -p` sidekick runs its builds and tests in the foreground.** Its session
ends with its turn, so a run it started in the background and a notification it waits for die
with it: an Opus sidekick once ended on "Waiting for the test run's notification",
with no report and its test run killed, and a resume had to finish it. The launch prompt says
so ("every cargo command in the foreground, a timeout up to 600000 ms; split the packages over
several calls if one would take longer").

## Phases and releases

**Before a tool phase** (any phase whose deliverable is a tool crate, or savefile writing): the
tool's plan gets an "Acceptance" section: stable-ID, GIVEN/WHEN/THEN scenarios for the
user-observable behavior that phase delivers (format in `CONTRIBUTING.md` "Testing"). Written
just-in-time for that phase only, never for the whole project up front: requirements written far
ahead of the code that meets them go stale. Lib phases do not get one; for a format crate the
fixture and the parity standard *are* the spec. A scenario for something a legacy tool already
does (a control Studio keeps or replaces) states the effect read from that tool's source, citing
the function, never one inferred from the control's name: SE-GUI-08 was written from the plan's
"drag-and-drop reordering replaces the up/down spinner" and reordered the roster, where
4ccEditor's spinner trades the two players' records between their ids, and the draft built on it
carried the error two more days, every review checking the draft against the scenario.

**Closing a phase**, in this order: (1) **converge**: audit the code against the phase's plan
sections and acceptance IDs, the lead's own audit first (every plan code block compared line by
line with the type it specifies, every step's verify criterion re-run), its gaps fixed, and only
then the cross-family reviewer ("Review rounds"), so the reviewer's attention goes to what the lead could
not see rather than to what it did not look for; every requirement that is missing, partial or
untested becomes a new worklog step, and the phase stays open until those are done. A requirement the plan *explicitly
defers* to a later phase (a "before implementing X, extend Y" note, an open question naming its
phase) is not a gap and stays in the plan. The lead's audit ends with a **design-health pass**
over the phase's crates, separate from requirements coverage: the design sweep ("Lead and sidekick") run
over the whole phase's code rather than one diff, `just mutants <crate>` (the extra mutant
kinds included) run over each crate with every survivor triaged (`CONTRIBUTING.md` "Mutation
runs"), `just citation-audit` with every flag ruled (`CONTRIBUTING.md` "Citation audit"), and
every `pub` item listed with the consumer that justifies it (a crate, the CLI, the bindings,
or a plan section naming one). A crate that reads a file format also gets a **census**: every file of that format on the maintainer's
machine (found through the Everything index, archives opened) run through read, the semantic
round trip and `check`, tallied by outcome, each failing class diagnosed before it is fixed or
recorded as a refusal, and re-run after every fix with a regression check (`scripts/census/`: the file list, the tally
and the regression check; its README says how a run fits them). A phase that changes the Team compiler or the Export upgrader also reruns the
upgrade census and our compile of the library, compared entry by entry with the last checkpoint
(`scripts/library_parity/`, its README's rerun order; `spec/team_compiler/testing.md`
"Testing: parity against Red (adapted from Blue)"), each changed entry traced to a commit of
the phase or fixed; Red is not run. Fixtures and mutation runs only test the files someone thought to add; the
first census found about 1900 community `.model` files the reader refused. A census of a
bit-packed format also tallies the bits real records set that no schema field covers: a round
trip keeps them, so only this tally shows a field modeled too narrow (one found PES 18's
dribbling-arm motion read as 2 bits of 3, misread for 2,808 players). Coverage asks
"is everything the plan wants there?"; this pass asks "did the phase make the code harder to
change?", which no test or lint measures and which compounds silently across phases. The
whole-crate run happens twice per crate: in the lead's audit, where its survivors seed the first
rework brief, and once at close, after the reviewer loop ends. The rework rounds in between are
measured together, one `just mutants-diff` at each review loop's end over the loop's commits
(`CONTRIBUTING.md` "Mutation runs"; 2.20f: 45-53 minutes per whole-crate run, 3 for a round's
diff). Neither run holds the work up: the next phase starts while a run is under way, and
the closing phase keeps its row open until the run's survivors are triaged and their tests
land, which can be days on the VPS alone; avoiding that downtime is worth a phase row open a
little longer; (2) **move** what the phase delivered from
`docs/plans/` to the same file and heading in `docs/spec/` (acceptance IDs stay, as the behavior
contract), correcting it where the code was built differently and converting any future-tense
wording; deferred parts stay in the plan, and a section partly delivered keeps its heading in both
trees, the plan's part pointing at the spec section it builds on; `just doc-pointers` then lists
every pointer left naming the old tree, and code points at the spec alone; `GLOSSARY.md`'s
rows for the moved terms are repointed by hand, since a pointer into a plan file that still
exists (as a stub naming its spec twin) resolves and the check passes it, after which the
gates run, comment-only edits included: a comment can sit inside text a test pins (one
repointed in `aatf`'s hashed `logic.rhai` left the crate's close mutation run's baseline
failing); (3) collapse the
worklog's step list to the phase row, keeping its `manual: checked`
lines, which are the manual scenarios' proofs (`CONTRIBUTING.md` "Testing"); (4) bring the
root `README.md` to the closed phase: what the suite does now, the phases done and the one
next, the test count from the gates' last run, in words for someone who has never opened
`docs/` (it is the first page a visitor reads, and it still said Phase 3 was under way after
four more phases had closed). Moving per phase, not at the
end of the project, is what keeps the spec true while the memory of the work is fresh.

**Cutting a release** whose phases are still open (0.0.1, a preview cut from Phase 8) runs
step (1) of "Closing a phase", the converge and its design-health pass, over what the
release ships, before the cut; the release's cut step in the worklog names it as a
prerequisite, so the cut cannot be checked off without it. The per-slice review loops do not
stand in for it: each reads one surface against its own sections, and none asks whether the
shipped whole meets its plan. A rule tied to a phase's close alone left 0.0.1's candidate
with the maintainer, every slice reviewed and the Save editor never converged.
One part of the pass never holds a 0.x.x cut: the whole-crate mutation runs of its crates.
Every slice a release ships had its `mutants-diff`, which tests the mutants of the lines it
changed, while `save_editor`'s whole-crate run alone measured over 30 hours. During 0.x.x
the project releases, then patches what the runs find afterwards: a hobby project's
previews can afford the concession, and its members get fixes sooner than a release held
for days would give them. The runs come after the cut, and their survivors' tests go into
the next release, or a patch when a survivor hides a real bug.

## Lead and sidekick

**Lead and sidekick.** Sessions run as a lead model that plans, briefs, reviews and talks to the
user, and a sidekick model (a different family) that implements and runs the gates. The general
method of briefing and of reviewing a delegate's work is the user-level `helium` skill: the lead
invokes it before writing a brief or reviewing a delegate's diff, the sidekick when a brief
arrives; what follows here is this project's own part, and wins where the two differ. The lead
hand-writes only what is *correctness-critical*: things whose wrong version passes instead of
failing: workspace lints, CI, `rust-toolchain.toml`, the acceptance-ID scanner, any fixture or
golden file, any measurement. Everything with a plan section to implement against is briefed to
the sidekick. A brief names the plan section(s) and `CONTRIBUTING.md` as reading to do *before*
coding (the sidekick does not inherit the lead's context), tells the sidekick to invoke the
`karpathy-guidelines` skill first (skills the lead has loaded are not active in the sidekick;
an uninstructed sidekick never invokes one), plus the acceptance IDs, the `→ verify:`
criterion, and the rule that a plan gap is *reported*, never silently decided. The same rule
covers the brief itself: an item of the brief that turns out unused or wrong (a dependency the
code never calls, a helper that does not exist) is reported in the closing section, not
implemented verbatim; the sidekick sees the code, the lead wrote the brief from a reading of it
(a brief once listed a `log` dependency the crate never used, and it landed until review). The
brief's verification list names the tests of every crate that *consumes* the one being changed, not
only the crate's own: a change to `fmdl`'s reader that every game `body.skl` tripped passed
`cargo test -p fmdl` and was caught only by `model_convert`'s tests at the lead's gate run.
`fmdl`'s and `pes_model`'s consumers include `python_bindings`, whose test is `just bindings`,
outside `just gates`: fixtures once added to both crates left the smoke test's fixture counts
stale, and only CI's `bindings` job saw it. A brief that adds tests says to insert them with
the editor tool inside the file's existing `#[cfg(test)]` module, never to append them with
shell redirection, which lands them after the module's closing brace. The lead
reads the whole diff before it lands, not the report about it; the report is a claim, the diff is
the evidence. Two failed sidekick attempts on one brief means the brief is suspect before the
sidekick is. **A brief for a change a member sees or does names the draft that already does
it** (the file and function or class under `docs/prototypes/`, in its reading list), and one
the lead cannot point there is not sent: the lead drafts it first, renders or drives it, and
the plan, DECISIONS and the draft land together. A plan sentence alone is not a draft: one
item was once briefed from one and built before the draft knew the behaviour. **A brief is
sized for one review.** Past about 500 lines of new code, a diff is read but not resteered: a
wrong shape in the first module is already copied into the third by the time the lead sees it
(`model_convert` took a 666-line rework commit after a single review of its first four
steps). A step expected to exceed that is briefed as ordered slices, each
landing as its own commit and reviewed before the next slice is briefed (`format/` first, then
`ops/`, then `check.rs`, for a format crate); generated tables and fixtures do not count toward
the size.

**The lead sees the report, not the run.** The harness shows the lead only the sidekick's final
report, never its compiler runs, so an error fixed by a workaround (a suppression, a loosened
assertion, an `#[ignore]`, a discarded `Result`) is invisible unless declared. Every brief
therefore requires a closing section, "Errors hit and how each was resolved": one line per
compiler, clippy or test failure met during the work and the fix applied, plus an explicit list
of any suppression, ignored test, removed or weakened assertion (or "none"), and, for every test
added for new behavior, the failing assertion text from its red run before the change ("red
first" is otherwise a claim the lead never sees; a test with no meaningful "before", such as a
new format crate's fixture round-trip, says so). Omitting the section or an item is a brief
violation, not an oversight. On the lead's side, every review before a commit includes two
sweeps of the diff and re-runs the changed packages' tests, clippy and fmt in the real
workspace (the dependents' tests run at the next checkpoint, `CONTRIBUTING.md` "Testing and
verification"); the sidekick's pasted gate tails are a claim. The **honesty sweep** looks for a hidden failure: `#[allow`, `#[expect`,
`#[ignore`, `.ok();`, `let _ =`, `drop(` on a `Result`, `unwrap_or_default`, `unwrap_or(`, and
a `while` or `loop` in a test with no bound (`CONTRIBUTING.md`: a test fails, it never hangs). The **design sweep** looks
for code that passes and is worse, the thing neither the gates nor a model's training penalize,
over the diff's tests as over its production code (`CONTRIBUTING.md`: tests are code), the
first item there a test that repeats another's scene or checks instead of sharing a helper:
`impl .* for` (a new trait: does it have two real implementors?), `dyn `, `_ =>` on one of our
enums, `== Engine::`/`!= Engine::` (a test against one engine, which a third one would pass
silently), `.clone()`/`.to_vec()` on bulk data, `as ` casts, two functions differing in a name and a
branch, `pub` items with no caller outside the crate, a module past roughly a thousand lines
that the diff grows (a test module too: crossing the line once is not the only flag, or a file
already past it grows unchecked; `aesthetics_export`'s `validate/tests.rs` reached 4,838), a comment naming or alluding to a legacy tool outside a parity test (`Red`,
`Blue`, `reference`, `add-on`, `Python`, `texconv`, `always`), a comment that does not stand on
its own (it cites a plan, a decision, a step or a review: `just comment-refs`; or it is a chain
of parentheses a maintainer has to read twice). The **mutation run** (`just
mutants-diff <the loop's first commit's parent>`, once per review loop, over every diff the loop
landed) is the third check and the only one that is a measurement rather than a reading: each
survivor is an assertion the sweeps cannot see because it was never written (the `kit_config`
probe found twenty such gaps in a crate that had passed both sweeps, converge and the
cross-family reviewer), and every survivor is triaged per `CONTRIBUTING.md` "Mutation runs"
before the surface closes, the missing tests going into one rework brief. Each sweep item is a
`CONTRIBUTING.md` rule; the sweeps exist because a rule nobody greps for is a rule the review
applies only when it happens to notice. When a review finds slop
neither list names, the fix is a new entry in this section or in `CONTRIBUTING.md`, not a longer review:
the rules are the project's whole substitute for taste, and any quality they do not express is
quality nobody is checking.

**Plan-defined shapes are copied, not recalled.** Where the plan gives a code block for a type,
a trait or a signature, the implementation starts from that block, pasted, and any deviation is a
decision entry, never a silent rewrite. Phase 1's first converge lost three of its seven reviewer
slots to shapes the lead had written from memory, all three of them written down in the plan.

**Working in parallel.** Lead and sidekick share one working tree, so when both write code at
once (the lead on a correctness-critical crate, the sidekick on a briefed one) they work in
disjoint crates, and each verifies with crate-scoped commands (`cargo test -p vtree`,
`cargo clippy -p vtree --all-targets -- -D warnings`) while the other's crate may not compile
yet. The lead never leaves the workspace unbuildable longer than one edit (declare a module only
once its file exists), and runs `just gates-changed` once, after both are done. The brief says
which crate the sidekick owns and that the rest of the tree is in motion.

## Review rounds

**Second opinion.** A model reviewing its own work is bounded by its own training: same data, same
blind spots. The sidekick's code is already reviewed cross-family by the lead; what is *not* is
everything the lead writes itself: plans, acceptance sections, briefs, converge audits, decision
entries. At these checkpoints, and only these, get a critique from the reviewer subagent of a
*different model family* than the lead (the reviewers of "Agents and harness"):
(a) after writing an Acceptance section; (b) after a worklog step that added a new `pub` interface
or touched more than one crate, after the implementation *and* its tests are written and before
the tests run, so one critique covers both the code and the assertions; steps inside one crate behind
an existing interface are covered by the lead's diff review and by converge; (c) as the second half
of every converge audit, after the lead's own, the primary use: the one check of code against
*plan* (not against the brief, which carries the lead's assumptions) by a model that wrote
neither; (d) reactively, when
two attempts on the same premise have failed, the sidekick's included; (e) after a change to a
**vital script**: one whose defect silently loses or misreports a measurement, which no gate
catches because the gates run through it. The list: the mutation runs' `mutants.py`,
`mutants_diff.py` and `extra_mutants.py`; the test runner `run_tests.py` and `test_runner.py`;
`remote_gates.py`; `scripts/census/` and `scripts/library_parity/`; the release build's
`release.py`, `build_remapped.py` and `make_candidate.py`. Their review is SWE-2 rounds at
least (the same loop, until a round accepts fewer than five), Astra when a change reshapes how
a run measures; the first review of each covers the whole script, later ones the changed
functions and their callers. Unreviewed, `mutants_diff.py` once deleted the previous run's
results whenever two runs followed each other. A test a
change adds to one is shown to fail by `scripts/review/perturb_check.py` (a list of perturbations,
each with its test, applied and undone one at a time).
**Three reviewer loops, cheapest first, one after the other.** SWE-2 runs its loop first; then
the loop of Gemini 4 Carbon on High thinking (`duck.py --reviewer carbon`, model
`gemini-4-carbon-high`: paid, much cheaper than Astra, better than SWE-2 and not as good as
Astra); then Astra's loop. Each loops
by the same rule (another round only when five or more were accepted). Every Carbon or Astra
round sits between two SWE-2 subloops, each looping by that rule: one before it, so the dearer
reviewer sees code SWE-2 has nothing left to say about, and one over its fixes (which is the
next round's "before"). A paid loop never reruns inside another: an Astra round's fixes get
the SWE-2 subloop, then the next Astra round, never a Carbon one. Example, accepted counts in
brackets: SWE(7) SWE(3) | Carbon(6) SWE(2) Carbon(3) SWE(1) | Astra(8) SWE(4) Astra(2) SWE(0).
Which login runs each reviewer changes with the accounts' plans: "How the work runs" names
the one file that holds them. A Devin login on the Free plan runs nothing. Carbon spends a Devin plan's
usage far faster than SWE-2's hidden limit does; Astra runs on Codex (`--reviewer astra`).
A reviewer with no usable login waits for one: its loop is queued in the worklog, and the
next loop in the order runs meanwhile.
**How deep a surface is reviewed follows what a defect in it would cost.** *Tier 1*, a
surface that writes what the game or a member's save reads (a CPK's content, a converted
model or texture, the savefile codec and its patch, an interchange file, the Export
upgrader's output, the updater's swap): the three loops above. *Tier 2*, other shipped code
(a check and its messages, a view, the command line's surface): SWE-2's
loop, then one round of one paid reviewer, which runs again only by the five-accepted rule.
*Tier 3*, scripts, CI, documents and drafts: one SWE-2 round, a second only by that rule (a
vital script, (e) above, keeps its own: SWE-2 rounds at least, Astra when a change reshapes
how a run measures). A step's worklog line names its tier when it lands. A loop not started
within seven days of the landing is not run on its own: the surface's next converge audit,
(c) above, covers it, and the step's line says so. We do not give every slice the full
stack: ten loops were once queued, some over code a week old, while it was the first rounds
that found the real defects and the one second round after a rework accepted none of its
seven concerns.
**The citation audit before every review round over code.** The lead runs `just
citation-audit` (`CONTRIBUTING.md` "Citation audit": cached, so a run pays for changed pairs
only) and rules its flags before writing the round's brief, so SWE-2 and Astra do not spend
concerns on a test that does not prove the scenario it cites. It sees only cited pairs: a rule
the spec states with no scenario of its own, or a test that passes because another rule gives
the same result on its fixture, is still the reviewers' and the mutation run's to find.
**Batch, don't stream:**
renames, doc edits, one-file fixes with no new behavior are never reviewed on their own; their
diffs ride along with the next (b) review, or with converge. Never more than one *scheduled*
critique per step; the reactive one (d) is an escalation and is exempt from the ceiling. Hand the
reviewer *pointers* (plan sections, file paths, a diff written to `.tmp/lead/review.diff`, the
acceptance IDs), never your own summary, which carries the assumptions it is there to catch. It
returns every concern it found, ranked, each marked verified or suspected. Answer every one in the
turn report: accepted → what changed; rejected → one line why. A concern silently dropped is the
failure mode this exists to prevent. **There is no cap on concerns; rounds are bounded by the
accept rate, not by the lead's sense of importance.** A cap made reviewers curate instead of
report: a round with a seven-concern cap once had its reasoning trace set out to pick "seven
issues that a parent will likely accept, ideally five or more", so whatever it judged unlikely to
be accepted went unreported, and a later round need not find it again. The ranking by
reachability (below) is what keeps impossible edge cases at the bottom of the list. So a critique of one surface runs another round only when at
least five of the last round's concerns were accepted (the signal was real); the next round gets
the rework diff and the prior rulings so it does not repeat them, and the loop stops at a round
under five accepted. There is no round ceiling: one loop's first three rounds each returned
seven, all accepted, most of them parity and memory defects rather than padding, so a ceiling
of three stopped the loop where its signal was strongest. The stop therefore rests on the lead's
rulings: a concern is rejected, with its one-line reason, when its input cannot reach the code,
the plan or a decision entry already settles it, or it restates an accepted concern's class
without a new defect. **Reviewer briefs rank concerns by reachability.** First comes what a
real file, the crate's own readers or its own operations produce. A hand-built value that none
of them produce is low value when the crate refuses it with an error, and ranks high only
when it panics, hangs, or writes a wrong file without an error. Without this line, the top of
one loop's list went to models no reader or operation produces. **Reviewer briefs also ask for
over-engineering**, and the lead's rulings watch for it, above all on plans: a reviewer asked
only for gaps reports only gaps, so each round adds a sentence per edge case it can name, and
the fix for one concern becomes the next round's concern. The brief asks it to flag what could
be cut (a special case a general rule already covers, machinery no scenario needs); the lead
prefers one general rule over a growing list of cases, and rejects a narrow addition that
distinguishes no behavior a real export reaches. Weighing was not enough: a lead with this
paragraph and the `duck` skill's "technically correct is not enough" both loaded once accepted
14 of 17, 10 of 11 and 11 of 12 concerns on whether each was true, each fix's clause the next
round's concern, and two rounds added a rule no real export reaches. So every acceptance names
its reach (a real export's outcome, a contradiction, an unobservable THEN, a wrong fact), and
one that names none is rejected; a nit fixed in passing counts toward no round.
Past the fifth round on one surface the lead reports the counts and the
classes found to the user and continues unless told otherwise: a check-in, not a stop. A review stopped for quota is queued in the lead's handover. Each
round's reviewer is a fresh run, never a resumed one, even though resuming
saves the re-exploration: when both once ran on one surface, the reviewer resumed from the
round before returned four concerns, all among the fresh reviewer's seven, which found three
more, all accepted; a resumed reviewer keeps its earlier reading and its blind spots with it. The
count *returned* is not a stop signal: a reviewer told to keep the list short withholds
(a round once returned five with its reasoning trace saying it aimed for "3-5 strong
concerns" because the brief said "do not pad"). So the brief
never says "do not pad" or names any number of concerns, as a cap or a target; it says to report
every concern found, ranked by reachability, and that a round with fewer than five accepted ends
the loop, so a verified concern is never withheld to keep the list short; padding is caught by
the accept rate, which is what that rule is for. At converge the surface is one crate (or one
coupled pair), never the phase. The user can request a critique at any time with `/duck`, and
the lead runs the `duck` skill itself for every scheduled critique. The accounts' order of
preference never makes a review wait: when the preferred ones are spent, the review runs now on
any account that can take it. Only a review no account can take goes on the handover's queue:
the lead retries once an hour, and a working call runs the queue until the next stop; the work
in between continues.

**A review is sized before it is briefed.** Every plan's quota (Devin's, Codex's, Claude's) is
a fixed amount of API-priced dollars shown as a percentage, so a review costs what its tokens
cost. Each request resends the whole context, so a review costs about its requests times its
average context, and GPT-6 Astra charges the whole of any request whose prompt passes 272K
tokens at twice its input rates (output 1.5x). Measured over the 34 Devin-route Astra exports
to 2026-10-16 (`~/.claude/tools/long_context.py`): a typical review peaks at 150-220K over about
55 requests and costs $7-11 at API prices; only four crossed 272K, adding 5% to the total; one
of them (`CP_a1`) reached 426K in 19 requests by reading large files whole. So the surcharge
is not what to design around; the context's size is. The lead estimates a surface as the bytes
it reads divided by four, plus about 30K for the brief, the rules and the guidelines, and:

- over about 200K, splits it along the code's own boundaries (one crate; a format crate's
  `format/`, `ops/` and `check.rs`; each with its plan sections), one brief per part, never
  through the middle of a module: what a split loses is the concern that needs both parts in
  view (two modules disagreeing, one rule applied two ways, a plan section the other part
  contradicts), and a boundary the code already draws keeps nearly all of those;
- for a converge audit split by crate, adds a short third brief over what crosses the parts
  (the shared types, the plan's cross-references), given as pointers;
- in every brief, has the reviewer read large files (fixtures, generated tables, a spec file
  past about 1,000 lines) by range or search, not whole;
- under about 200K does not split: two parts repeat the fixed reading and save nothing.
