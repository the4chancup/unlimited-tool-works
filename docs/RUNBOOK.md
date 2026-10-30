# Runbook

How the lead runs the project's machinery: mutation runs and the VPS that shares them, the scratch
folder, the agents and their harness limits, and the full rules of the review rounds. Moved here
from `AGENTS.md` on 2026-10-14, verbatim, because the Codex CLI truncates a project doc past
32 KB and its reviewers never saw `AGENTS.md`'s last quarter; `AGENTS.md` keeps a summary of each
and its rules stay in force. Read a section before starting what it covers.

## Mutation runs and the VPS

**Whole-crate mutation runs are split with the maintainer's VPS.** On the maintainer's PC the
user environment variable `STUDIO_MUTANTS_REMOTE=bonfire` (an alias in `~/.ssh/config`) makes
`just mutants <crate>` run part of the mutants there (`scripts/mutants.py`: the working tree is
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
("splitting the run with bonfire" or "running every mutant on this machine"). At 2.20i an
agent shell without the variable ran all 806 `pes_savefile` mutants locally, over three
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
first build and every timeout: until 3.z the estimate left all three out, and a 38-mutant diff
estimated at 80 s took 270 s (now 266 s). That estimate is for the whole run here; a split
run prints its own, the baseline plus half the rest (each half builds a baseline, so a split
does not halve the estimate), and ends with its wall time, which adds the transfer, polling
and fetch. The local half runs at below-normal priority with
each of its two cargo processes given half the logical CPUs (`sized_config`, the remote half's
sizing), not with two CPUs held back: the machine stays usable by priority, and the run gets
what the user leaves idle (measured level with the old cap on the 3.z diff). A starved test can
then time out, so a timeout that no infinite loop explains is rerun before it counts as caught.
A build killed for memory (the remote half's cap) is filed by
cargo-mutants as unviable, which hides an untested mutant: at 3.z eframe's dependency tree
outgrew the 6 GiB cap and 12 remote mutants went untested that way, so the scripts now list
every killed build and fail the run. Below
that the split's fixed overhead, about a minute plus up to 15 s of polling, eats the gain. At
2.20k a 32-mutant `pes_savefile` diff took 11 min 57 s locally; split, the local half took
417 s and the remote half 502 s from its launch (the VPS ran about a fifth slower; the total
wall time was not recorded). Manual ssh from
Git Bash uses `/c/Windows/System32/OpenSSH/ssh.exe bonfire`: Git's own `ssh` cannot reach the
Windows agent that holds the key. **A whole-crate run copies the working tree when it starts**,
so a chain of runs queued behind a sidekick's edits snapshots a half-edited crate: at 4.y-conv
the `team_compiler` run started while 4.y-fix1 was being written and had to be stopped; nothing
edits a crate while its run is queued or going. The lead launches a long chain detached, with
PowerShell `Start-Process bash.exe <script> -WindowStyle Hidden` from a script under `.tmp/`,
never with the Bash tool's background mode: its two-hour cap kills the chain, and stopping it
kills the console the run's children need, after which every build fails with `0xC0000142`
and cargo-mutants files the mutant as unviable (273 of 274 in 23 s at 4.y-conv's first attempt).

**When the maintainer's PC is suspect, the heavy runs go to the VPS.** Four
`MEMORY_MANAGEMENT` bugchecks there (2026-10-07 to 10-10, the last during a run, all four in
`rustc.exe` at one kernel site) led the maintainer to move them until third-party drivers
were removed (2026-10-11), when the split came back; the switch stays for the next time. The
maintainer moved every mutation run to the VPS again on 2026-10-25 (the switch on), and the
lead's gates with them (`just gates-remote`); tests and gates came back the same evening, and
mutation runs split again from 2026-10-26 (the switch off).
`REMOTE_ONLY = True` in `scripts/mutants.py` makes
`just mutants` and `just mutants-diff` run every mutant on the VPS as one whole run (no
local half, no split threshold; the last local `mutants.out` set aside as
`mutants.out.old`), and `just gates-remote` runs `just gates` there against a snapshot of
the working tree (`scripts/remote_gates.py`: the same transfer and clone, so it refuses
while a mutants half is running or uncollected; the transient service `studio-gates`,
memory-capped and CPU-idle, run synchronously). The VPS's `just` is installed in
`~/.cargo/bin` at this machine's version, as cargo-mutants is. The lead uses
`just gates-remote` for its review gates while the switch is on, and since 2026-10-25 (the
maintainer: "move all the tests that can be moved") a sidekick's test and clippy runs go
there too, one command at a time, `just remote cargo test -p <crate>` (`cargo` or `just`
only; a brief says so, and that the run refuses while a mutation half or other gates use the
clone, to wait and retry); only `cargo fmt` and `cargo check` stay local. Measured on 4.y-sub (b3b4a): gates 4 min 11 s cold (peak
8.7 GiB), 1 min 23 s incremental; a 15-mutant diff 330 s wall. The switch is a constant,
not an environment variable as it first was: a shell started while the variable was set
kept it after it was deleted, and would have kept running every mutant remotely.

**The VPS's production Fluxer instance comes first.** The remote half runs as the transient
system service `studio-mutants` (`sudo -n systemd-run`) with no swap and a `MemoryMax` of the
host's available memory at launch less 1 GiB (`REMOTE_MEMORY_RESERVE`; printed as `remote
memory cap`, and `just gates-remote`'s unit gets the same). Until 2026-10-11 the cap was a
constant raised by hand each time the maintainer freed memory on the host (6G to 11G); a
measured one takes what is free without asking and leaves the host its margin. Each remote
cargo process also builds with few jobs (`REMOTE_BUILD_JOBS`, 1 since 4.14b's run hit the cap).
So a runaway mutant is OOM-killed inside the unit and counts as caught; `OOMPolicy=continue`, so that
kill does not stop cargo-mutants; `CPUWeight=idle`, because `nice` cannot keep a `user.slice`
process off Fluxer's CPU under cgroup v2. On 2026-09-29 the old uncapped half filled the host's
RAM and swap (`display_name` with `i *= 1`, a loop that grows a `Vec` forever), and the global
OOM killer, which could as well have picked Postgres, took the test. The run prints its peak
(`remote memory peak`) against its cap. Every split run since the tmpfs fix peaks at the
cap, which is the cgroup's page cache being reclaimed, not a kill, so a peak near the cap
changes nothing by itself. When a build is killed at the cap, the run fails and lists the
untested mutants (`report_killed`), and the fix is then memory freed on the host (the
maintainer's) or a lower `REMOTE_BUILD_JOBS`; past one job, the crate joins
`LOCAL_ONLY_CRATES` in `scripts/mutants.py`, whose mutants never go to the VPS (a whole-crate run
and any diff holding one of them run locally whole). `team_compiler` was listed from 4.14d, whose
run lost two builds at the cap with one job, until the Phase 4 converge audit (2026-10-09) found
the cause: the host's `/tmp` is a 7.8 GiB tmpfs, cargo-mutants' default place for its two copies
of the tree, so each copy's `target/` (4 GiB with egui linked) lived in the unit's memory and the
per-mutant rebuilds of 1-2 GiB tipped it over the cap; the script now builds the remote half under
`~/studio-mutants/tmp` on disk (`REMOTE_TMPDIR`). The build copies' size, not the build's memory,
is what to look at first when a killed build shows up again. A running half is stopped with
`sudo systemctl stop studio-mutants` on the host.

**The remote half runs detached** from any ssh session (`~/studio-mutants/run/`: `job.sh`,
`pid`, `log`, `exit`, `memory_peak`), and the script polls it every 15 s with short ssh calls (30 s until 2026-10-10). A dropped link
costs a poll, not the run: at 2.20i the remote half still hung off one long ssh session and
died with it. If `just mutants` itself stops, `just mutants-collect` waits for the remote half
and fetches it, including a died half's partial results. A new run refuses to start while a
remote half is running or uncollected (a local one too, since the collection summarizes the
local `mutants.out` with the remote half), and a run that uses the VPS also while `just
gates-remote` runs there: both rewrite the remote clone. **A failed half is reported, never restarted:**
`cargo mutants` does not resume, so a rerun starts from zero, hours of work (at 2.20i the
sidekick restarted the VPS half and waited on it although the local half had already tested
every mutant). A brief that starts a whole-crate run says so.

## The scratch folder

**The session scratch `.tmp/` holds one folder per subphase** (`4_17/` for every 4.17 slice's
briefs, reports, diffs, logs and probes; `lead/` for the lead's live notes; `reference/` for
cup material; `review_queue_S/` for the S1-S6 review rounds), sorted that way on 2026-10-08
after a flat top level had reached about two thousand files: a new scratch file goes in its
step's folder. **A temporary folder outside the repository** (a git worktree for a review, a
side-lead, a release or in-game build) goes under `../4cc-studio-temp/<name>`, never loose beside
the repository: the maintainer browses that folder, and seventeen `4cc-studio-*` siblings had
buried the repository by 2026-10-26, when they moved there. **An archived run keeps its summary, not its logs.** A run kept aside (in
`.tmp/<subphase>/mutants_<step>_<what>/`, before
the next run overwrites `mutants.out/`) copies only `*.txt` and `outcomes.json`, never the
per-mutant `log/` or `remote/` trees: those are what reproduce a survivor, and a survivor is
reproduced by re-running its mutant, not by reading a week-old log. Whole-folder copies had
grown `.tmp/` to 17 GB by 3.8. Each local run builds in a
`%TEMP%\studio-mutants\cargo-mutants-4cc-studio-*.tmp` copy of the tree (one fixed parent,
`LOCAL_TMPDIR`, so one antivirus exclusion covers every run; until 2026-10-10 directly in
`%TEMP%`), deleted when the run ends normally; a killed run leaves its copy behind
(some with a multi-GB `target/`), so after stopping one, delete its copy. `.cargo/mutants.toml`
sets `gitignore = true` so the copies leave out `.tmp/` and `target/`: before 3.8 each copy
carried all of `.tmp/`, and two 3.7 runs died of a full disk. The logs alone are large: a
`pes_savefile` whole-crate run leaves about 20 GiB in `mutants.out/` (5.6b: 8.9 GiB local,
11 GiB fetched from the remote half), which the next run moves to `mutants.out.old`; a chain
script that copied `remote/` whole into its archive filled C: and stopped the two runs queued behind it.
When C: fills, a finished tree's `target/` can go whole, and the main tree's `target/debug`
once nothing builds there (254 GiB on 2026-10-26, mostly stale `deps`): cargo rebuilds them,
each run rewrites its generated config, and the cost table lives outside every `target/`.

**On the VPS the project may hold about 40 GB** (the maintainer's figure), on a disk the host's
production services share. Every run that uses the host passes a guard first (`remote_refusal`):
it empties `~/studio-mutants/tmp` (`REMOTE_TMPDIR`, the remote tests' temp dir), where a test
that removes its folder at its end leaves it on every caught mutant and a killed half leaves its
tree copies, then refuses when `~/studio-mutants` still holds over 30 GiB
(`REMOTE_DISK_BUDGET`, leaving room for a half's two build copies) or the disk has under
12 GiB free (`REMOTE_FREE_FLOOR`: a half's own copies and tests take about 10, and 2 stay for
the host's system; when this project's cleanup cannot lift it, the maintainer is asked). On 2026-10-22, before the guard, the folder held 50 GB (28.6 of them test
leftovers, 27 the Save editor's pid-named copies of a fixture save), the disk was at 99% and a
half lost five mutants. **At each phase's close**, after its last mutation run, the lead
removes the clone's build cache, `~/studio-mutants/tree/target` (22 GB after two phases; only
`just gates-remote` builds there, which then starts cold) with `scripts/remote_clean_target.sh`
(its header has the two ssh lines: a dry run first, then `--delete`); it refuses while a half,
the gates or a cargo process runs. The same cleanup frees the folder whenever the guard refuses
for the budget or the floor between closes (2026-10-29: 35 GB, the disk from 13 to 47 GB
free). Claude Code's auto-mode classifier refuses the two ssh lines unless the project's
`.claude/settings.local.json` allows them, which it does on the maintainer's machine, each
exactly as the header writes it.

## Agents and harness

Three model families, three roles. Lead: Claude (Fable) in Devin CLI's Fusion mode. Sidekick:
SWE-2 (Kimi lineage), reached through the `sidekick` tool; a cold probe on a spec-in-hand crate
showed it follows `CONTRIBUTING.md` literally and reports plan gaps instead of deciding them, so it
is trusted with implementation, not with judgment. Reviewer subagent profiles (global,
`%APPDATA%\Devin\agents\`): `gpt-astra-high` (GPT, latest Astra) when the lead is a Claude, **or
when you do not know what model you are**; `fable-medium` (Claude) only when you know you are a
GPT. The reviewer reviews; it is never asked to edit. Cost is not the constraint on any of these;
time is, hence the batching rule above.

Two harness limits, both measured. **`run_subagent` profiles have no `skill` tool**, and the
reviewer profiles (whose files list `allowed-tools`) have `read`, `grep` and `find_file_by_name`
only. A brief that says "invoke the `karpathy-guidelines` skill" sends a subagent globbing the
repo for a file that is not there; give it the file to read instead,
`%APPDATA%\devin\skills\karpathy-guidelines\SKILL.md` (outside the repo, and its `read` tool
reaches it). The `sidekick` tool has the `skill` tool, so its briefs keep the invoke wording.
**The sidekick's role can be played by a `swe-2-high` subagent** (same model, profile file with
no `allowed-tools`, so it has the shell, `edit` and `write`; measured 2026-10-05): run it in the
foreground (a background subagent has unapproved tools denied), resume the same one per task
(`resume: <agent_id>` keeps its memory, as the sidekick's handoffs do). SWE-2 has a 1M-token
context (a post-trained Kimi K3): the two S5.14 review runs (2026-10-05) that ended with an
empty final message at about 260K hit a 262K limit the ACP layer imposed, since lifted. A review over a wide surface is still split
into two briefs run in parallel, because one round over a whole module runs well over an hour. The
`sidekick` tool's single sidekick cannot be reset. **Compaction does not fire while a sidekick handoff or a background subagent is in
flight**, so a run started "to save time" ahead of the `[[/compact]]` sentinel costs the
compaction instead (2.20b's mutation runs were started that way and the sentinel was ignored).
Compact first, then start the run in the resumed turn.

The Devin IDE compacts the sidekick's context by itself when it reaches 500K tokens, so a
full sidekick context needs no action from the lead; a brief still stands alone (file under
`.tmp/`, read in full), because a compaction may fall between two handoffs.

**A Devin CLI run is watched for stalls.** A Claude lead runs SWE-2 through
`devin_sidekick.py` (implementation) and `duck.py --reviewer swe` (reviews) as background
commands, and the harness tells the lead only when a command exits; a run whose session goes
quiet mid-turn never exits. On 2026-10-24 a rework's resumed session went idle at 01:39 in the
middle of a red run, its hand perturbation still applied, and only the lead's check at 01:50
found it; the background command's 2-hour cap would have ended it at 03:22. So every such run
gets a second background command beside it, `python ~/.claude/skills/duck/devin_watch.py
<brief>` (timeout 7200000): it exits quietly when the run ends, and with `STALLED` (exit 3) when
for 20 minutes neither the session's `last_activity_at` in Devin's `sessions.db` moved nor the
run's process tree burned CPU (a build or test burns it without any step). The watcher never
stops the run: what to do is the lead's call, after reading `git diff`, since a stalled run can
leave a half-done edit or a perturbation in the tree. A
session that stalled after its one allowed resume is not resumed again; the lead finishes the
brief's remaining items itself or briefs a fresh session.

**Sidekicks in parallel worktrees never share a `target/`.** Each worktree's builds go to its
own `CARGO_TARGET_DIR` (`../4cc-studio-temp/wt-<x>/target`), at `CARGO_BUILD_JOBS=4`. Cargo names a workspace crate's build output by its path relative
to the workspace root, so every worktree writes the same file name for the same crate, and a
build in one worktree takes another's output as up to date: on 2026-10-28 one `save_editor`
build linked another worktree's `aatf` (a constant reported missing in a tree that has it), and
a doc-test linked a `studio_core` without the function its own tree had just added. Cargo's
lock on a shared folder serializes builds but does not stop this, so a shared folder makes
every sidekick's test results suspect. Each folder costs a full build and its disk.

**At most two cargo builds at once, on the whole machine** (the maintainer, 2026-10-29): a
sidekick's, a mutation run's local half, the lead's gates or package tests and a release
build each count as one. Two already fill the CPU almost entirely, so a third makes nothing
faster and only risks exhausting the memory and crashing the system. Before starting a
build, count the ones running; a third waits for one to end.

**Every build and test job runs at below-normal priority** (the maintainer, 2026-10-30): at
normal priority two jobs left the maintainer's machine barely usable, while a below-normal one
yields every processor to their own programs and slows only while they use it. The priority is
inherited, so it is set where a job starts: `scripts/mutants.py` starts cargo-mutants that way,
the lead's sidekick launcher starts its `claude` process that way (everything the sidekick runs
inherits it), and the lead starts its own gates, package tests and release builds through a
wrapper that does the same (`.tmp/lead/below_normal.py <command>`). A job found at normal
priority is lowered with its whole process tree (`.tmp/lead/lower_tree.ps1 -Match <text>`).

**A headless `claude -p` sidekick runs its builds and tests in the foreground.** Its session
ends with its turn, so a run it started in the background and a notification it waits for die
with it: on 2026-10-29 an Opus sidekick ended on "Waiting for the test run's notification",
with no report and its test run killed, and a resume had to finish it. The launch prompt says
so ("every cargo command in the foreground, a timeout up to 600000 ms; split the packages over
several calls if one would take longer").

## Review rounds

**Second opinion.** A model reviewing its own work is bounded by its own training: same data, same
blind spots. The sidekick's code is already reviewed cross-family by the lead; what is *not* is
everything the lead writes itself: plans, acceptance sections, briefs, converge audits, decision
entries. At these checkpoints, and only these, get a critique from the reviewer subagent of a
*different model family* than the lead (profiles in "Environment"), invoked with `run_subagent`:
(a) after writing an Acceptance section; (b) after a worklog step that added a new `pub` interface
or touched more than one crate, after the implementation *and* its tests are written and before
the tests run, so one critique covers both the code and the assertions; steps inside one crate behind
an existing interface are covered by the lead's diff review and by converge; (c) as the second half
of every converge audit, after the lead's own, the primary use: the one check of code against
*plan* (not against the brief, which carries the lead's assumptions) by a model that wrote
neither; (d) reactively, when
two attempts on the same premise have failed, the sidekick's included. **Batch, don't stream:**
renames, doc edits, one-file fixes with no new behavior are never reviewed on their own; their
diffs ride along with the next (b) review, or with converge. Never more than one *scheduled*
critique per step; the reactive one (d) is an escalation and is exempt from the ceiling. Hand the
reviewer *pointers* (plan sections, file paths, a diff written to `.tmp/lead/review.diff`, the
acceptance IDs), never your own summary, which carries the assumptions it is there to catch. It
returns every concern it found, ranked, each marked verified or suspected. Answer every one in the
turn report: accepted → what changed; rejected → one line why. A concern silently dropped is the
failure mode this exists to prevent. **There is no cap on concerns; rounds are bounded by the
accept rate, not by the lead's sense of importance.** A cap made reviewers curate instead of
report: 3.1's first round had a seven-concern cap, and its reasoning trace set out to pick "seven
issues that a parent will likely accept, ideally five or more", so whatever it judged unlikely to
be accepted went unreported, and a later round need not find it again. The ranking by
reachability (below) is what keeps impossible edge cases at the bottom of the list. So a critique of one surface runs another round only when at
least five of the last round's concerns were accepted (the signal was real); the next round gets
the rework diff and the prior rulings so it does not repeat them, and the loop stops at a round
under five accepted. There is no round ceiling: 2.20d's first three rounds each returned seven
(then the cap), all accepted, most of them parity and memory defects rather than padding, so a ceiling of three
stopped the loop where its signal was strongest. The stop therefore rests on the lead's
rulings: a concern is rejected, with its one-line reason, when its input cannot reach the code,
the plan or a decision entry already settles it, or it restates an accepted concern's class
without a new defect. **Reviewer briefs rank concerns by reachability.** First comes what a
real file, the crate's own readers or its own operations produce. A hand-built value that none
of them produce is low value when the crate refuses it with an error, and ranks high only
when it panics, hangs, or writes a wrong file without an error. From 2.20f's second round on,
the top of the list went to models no reader or operation produces. Round 4's brief carried this line, and
none of its four concerns needed a hand-built model. **Reviewer briefs also ask for
over-engineering**, and the lead's rulings watch for it, above all on plans: a reviewer asked
only for gaps reports only gaps, so each round adds a sentence per edge case it can name, and
the fix for one concern becomes the next round's concern. The brief asks it to flag what could
be cut (a special case a general rule already covers, machinery no scenario needs); the lead
prefers one general rule over a growing list of cases, and rejects a narrow addition that
distinguishes no behavior a real export reaches. At 3.1 (maintainer's warning), the rulings on
the sidekick's round S2 replaced a growing list of "not compilable yet" cases with one short
rule, what the tracer compiles, and rejected three narrow additions (a `NO_USE` inside a
wrapper, folders inside `Common/`, a player folder named `Players`); from GPT round 6 on, both
reviewers' briefs asked for over-engineering. Weighing was not enough: at 6.16a the
lead, with this paragraph and the `duck` skill's "technically correct is not enough" both loaded,
accepted 14 of 17, 10 of 11 and 11 of 12 concerns on whether each was true, each fix's
clause the next round's concern, and two rounds added a rule no real export reaches (an
`.ftex` kit texture in a pre-Fox export: none of the 58 on the maintainer's machine). So every
acceptance now names its reach (`AGENTS.md` "Second opinion": a real export's outcome, a
contradiction, an unobservable THEN, a wrong fact), and one that names none is rejected; a
nit fixed in passing counts toward no round. Re-ruled that way, the third round accepted 4.
Past the fifth round on one surface the lead reports the counts and the
classes found to the user and continues unless told otherwise: a check-in, not a stop. Each
round's reviewer is a fresh `run_subagent`, never a resumed one (`resume`), even though resuming
saves the re-exploration: 2.20d's fourth round ran both on the same surface, and the reviewer
resumed from round three returned four concerns, all among the fresh reviewer's seven, which
found three more, all accepted; a resumed reviewer keeps its earlier reading and its blind spots
with it. The
count *returned* is not a stop signal: a reviewer told to keep the list short withholds
(2.20c's first round returned five with its reasoning trace saying it aimed for "3-5 strong
concerns" because the brief said "do not pad"; the second round, run as an experiment, returned
three verified concerns, all accepted, one of them contradicting a lead ruling). So the brief
never says "do not pad" or names any number of concerns, as a cap or a target; it says to report
every concern found, ranked by reachability, and that a round with fewer than five accepted ends
the loop, so a verified concern is never withheld to keep the list short; padding is caught by
the accept rate, which is what that rule is for. At converge the surface is one crate (or one
coupled pair), never the phase. The user can request a critique at any time with `/duck`, and
since 2026-10-09 the lead runs the `duck` skill itself for every scheduled critique. Astra's
five-hour quota holds five or six reviews (the weekly one about forty). The accounts' order of
preference never makes a review wait: when the preferred ones are spent, the review runs now on
any account that can take it. Only a review no account can take goes on the worklog's
"Handover" queue: the lead retries once an hour, and a working call runs the queue until the
next stop; the work in between continues.

**A review is sized before it is briefed.** Every plan's quota (Devin's, Codex's, Claude's) is
a fixed amount of API-priced dollars shown as a percentage, so a review costs what its tokens
cost. Each request resends the whole context, so a review costs about its requests times its
average context, and GPT-6 Astra charges the whole of any request whose prompt passes 272K
tokens at twice its input rates (output 1.5x). Measured over the 34 Devin-route Astra exports
to 2026-10-16 (`.tmp/astra_cost/long_context.py`): a typical review peaks at 150-220K over about
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
