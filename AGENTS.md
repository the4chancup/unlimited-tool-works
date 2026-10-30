# Unlimited Tool Works - Agent briefing

Unlimited Tool Works is a Rust suite of every tool the 4cc community uses to run its PES (2015–2021) cups:
a Team compiler (aesthetics exports → CPK archives), a save editor, and a dozen smaller tools, all
in one `egui` binary that also works as a CLI. What the suite does is specified in `docs/spec/`
(index: `docs/spec/README.md`), what comes next is planned in `docs/plans/` (index:
`docs/plans/README.md`). This file is the short version: how the project is organized, how
the codebase works, and what to do when the plan runs out. Before writing or changing code, read
`docs/CONTRIBUTING.md` in full: architecture rules, code style, verification gates.

## Working documents

Ten documents, ten jobs. Do not let content leak between them; two homes means they drift.

| Document | Holds |
|---|---|
| `docs/spec/*.md` | What the suite does now, and why: architecture, formats, rationale, resolved decisions, and each delivered tool's "Acceptance" section. A phase's sections move here from the plans when it closes |
| `docs/plans/*.md` | Work not built yet, with the same layout and headings as the spec: the design of what comes next, its rationale, and the open phase's "Acceptance" sections. Written in the present tense (the folder says it is future), each file opening with a banner that says so; a section partly built keeps its heading in both trees, the plan's part pointing at the spec section it builds on |
| `docs/CONTRIBUTING.md` | How code is structured, written and verified |
| `docs/RUNBOOK.md` | How the lead runs the machinery: mutation runs and the VPS, the scratch folder, the agents and their harness limits, the review rounds' full rules |
| `docs/GLOSSARY.md` | One line per domain term, pointing at the plan section that owns it |
| `docs/WORKLOG.md` | *Where we are*: current status, phase/step checklist, current-state gotchas, open issues (the lead's and the sidekick's) |
| `docs/LOG.md` | *What happened*: the dated log, three lines at most per entry, newest last (append-only) |
| `docs/DECISIONS.md` | Choices made where the plan was silent (append-only) |
| `docs/QUESTIONS.md` | The questions only the maintainer can answer (in-game behavior, cup practice, a preference), grouped by what an answer unblocks; an entry leaves when answered and the answer lands in the plan and `DECISIONS.md`. Before this file, they were scattered across the worklog's steps and issues and nobody could see the list |
| `docs/PROPOSALS.md` | Usability ideas nobody asked for (a button, a default, a shortcut), which the maintainer vetoes or accepts; an unruled one is not built |

**Before a step:** read the worklog's current status, then the plan section the step points at,
from the document, not from memory. **After a step:** run the step's checkpoint gates
(`CONTRIBUTING.md` "Testing and verification": what runs per commit, what at a checkpoint), mark the
step done with a one-line summary and the files touched, update "Current status", add a line to `docs/LOG.md`,
commit (`git commit -F <file>`; PowerShell has no heredocs). Once a release exists, the review
before that commit also asks whether a member would notice the change against the last release,
and if so checks the diff for its `CHANGELOG.md` `[Unreleased]` line (`CONTRIBUTING.md`
"Commits").

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
then the cross-family reviewer (below), so the reviewer's attention goes to what the lead could
not see rather than to what it did not look for; every requirement that is missing, partial or
untested becomes a new worklog step, and the phase stays open until those are done. A requirement the plan *explicitly
defers* to a later phase (a "before implementing X, extend Y" note, an open question naming its
phase) is not a gap and stays in the plan. The lead's audit ends with a **design-health pass**
over the phase's crates, separate from requirements coverage: the design-tell sweep (below) run
over the whole phase's code rather than one diff, `just mutants <crate>` (the extra mutant
kinds included) run over each crate with every survivor triaged (`CONTRIBUTING.md` "Mutation
runs"), `just citation-audit` with every flag ruled (`CONTRIBUTING.md` "Citation audit"), and
every `pub` item listed with the consumer that justifies it (a crate, the CLI, the bindings,
or a plan section naming one). A crate that reads a file format also gets a **census**: every file of that format on the maintainer's
machine (found through the Everything index, archives opened) run through read, the semantic
round trip and `check`, tallied by outcome, each failing class diagnosed before it is fixed or
recorded as a refusal, and re-run after every fix with a regression check (`.tmp/2_20/model_census/`
is the template). A phase that changes the Team compiler or the Export upgrader also reruns the
library parity against Red (`scripts/library_parity/`, `plans/team_compiler/testing.md`), each
new difference class diagnosed. Fixtures and mutation runs only test the files someone thought to add; the
first census, at 2.20g, found about 1900 community `.model` files the reader refused. A census of
a bit-packed format also tallies the bits real records set that no schema field covers: a round
trip keeps them, so only this tally shows a field modeled too narrow (at 2.20i it found PES 18's
dribbling-arm motion read as 2 bits of 3, misread for 2,808 players). Coverage asks
"is everything the plan wants there?"; this pass asks "did the phase make the code harder to
change?", which no test or lint measures and which compounds silently across phases. The
whole-crate run happens twice per crate: in the lead's audit, where its survivors seed the first
rework brief, and once at close, after the reviewer loop ends. The rework rounds in between are
measured together, one `just mutants-diff` at each review loop's end over the loop's commits
(`CONTRIBUTING.md` "Mutation runs"; 2.20f: 45-53 minutes per whole-crate run, 3 for a round's
diff); (2) **move** what the phase delivered from
`docs/plans/` to the same file and heading in `docs/spec/` (acceptance IDs stay, as the behavior
contract), correcting it where the code was built differently and converting any future-tense
wording; deferred parts stay in the plan, and a section partly delivered keeps its heading in both
trees, the plan's part pointing at the spec section it builds on; `just doc-pointers` then lists
every pointer left naming the old tree, and code points at the spec alone; `GLOSSARY.md`'s
rows for the moved terms are repointed by hand, since a pointer into a plan file that still
exists (as a stub naming its spec twin) resolves and the check passes it (5.6's close left
three, fixed at 6.0b), after which the
gates run, comment-only edits included: a comment can sit inside text a test pins (5.6e
repointed one in `aatf`'s hashed `logic.rhai`, and the crate's close mutation run found its
baseline failing); (3) collapse the
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
with the maintainer, every slice reviewed and the Save editor never converged (2026-10-30).

**Lead and sidekick.** Sessions run as a lead model that plans, briefs, reviews and talks to the
user, and a sidekick model (a different family) that implements and runs the gates. The general
method of briefing and of reviewing a delegate's work is the user-level `helium` skill: the lead
invokes it before writing a brief or reviewing a delegate's diff, the sidekick when a brief
arrives; what follows here is this project's own part, and wins where the two differ. The lead
hand-writes only what is *correctness-critical*: things whose wrong version passes instead of
failing: workspace lints, CI, `rust-toolchain.toml`, the acceptance-ID scanner, any fixture or
golden file, any measurement. Everything with a plan section to implement against is briefed to
the sidekick. A brief names the plan section(s) and `CONTRIBUTING.md` as reading to do *before*
coding (the sidekick does not inherit this file's context), tells the sidekick to invoke the
`karpathy-guidelines` skill first (skills the lead has loaded are not active in the sidekick;
an uninstructed sidekick never invokes one), plus the acceptance IDs, the `→ verify:`
criterion, and the rule that a plan gap is *reported*, never silently decided. The same rule
covers the brief itself: an item of the brief that turns out unused or wrong (a dependency the
code never calls, a helper that does not exist) is reported in the closing section, not
implemented verbatim; the sidekick sees the code, the lead wrote the brief from a reading of it
(2.18's brief listed a `log` dependency the crate never used, and it landed until review). The brief's
verification list names the tests of every crate that *consumes* the one being changed, not
only the crate's own: a change to `fmdl`'s reader that every game `body.skl` tripped passed
`cargo test -p fmdl` and was caught only by `model_convert`'s tests at the lead's gate run.
`fmdl`'s and `pes_model`'s consumers include `python_bindings`, whose test is `just bindings`,
outside `just gates`: 2.20g-h added fixtures to both crates, the smoke test's fixture counts
went stale, and only CI's `bindings` job saw it (run 24, 2026-09-29). A brief that adds tests says to insert them with the editor tool inside the file's existing
`#[cfg(test)]` module, never to append them with shell redirection, which lands them after the
module's closing brace (three times in 2.20f). The lead
reads the whole diff before it lands, not the report about it; the report is a claim, the diff is
the evidence. Two failed sidekick attempts on one brief means the brief is suspect before the
sidekick is. **A brief is sized for one review.** Past about 500 lines of new code, a diff is
read but not resteered: a wrong shape in the first module is already copied into the third by
the time the lead sees it (`model_convert` took a 666-line rework commit after a single review
of its first four steps). A step expected to exceed that is briefed as ordered slices, each
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
for code that passes and is worse, the thing neither the gates nor a model's training penalize:
`impl .* for` (a new trait: does it have two real implementors?), `dyn `, `_ =>` on one of our
enums, `== Engine::`/`!= Engine::` (a test against one engine, which a third one would pass
silently), `.clone()`/`.to_vec()` on bulk data, `as ` casts, two functions differing in a name and a
branch, `pub` items with no caller outside the crate, a module that crossed roughly a thousand
lines in this diff, a comment naming or alluding to a legacy tool outside a parity test (`Red`,
`Blue`, `reference`, `add-on`, `Python`, `texconv`, `always`), a comment that does not stand on
its own (it cites a plan, a decision, a step or a review: `just comment-refs`; or it is a chain
of parentheses a maintainer has to read twice). The **mutation run** (`just
mutants-diff <the loop's first commit's parent>`, once per review loop, over every diff the loop
landed) is the third check and the only one that is a measurement rather than a reading: each
survivor is an assertion the sweeps cannot see because it was never written (the `kit_config`
probe found twenty such gaps in a crate that had passed both sweeps, converge and the
cross-family reviewer), and every survivor is triaged per `CONTRIBUTING.md` "Mutation runs"
before the surface closes, the missing tests going into one rework brief. A decision model's bug scan of
each 60-line window (the Clef scan) is retired: `CONTRIBUTING.md` "Clef scan" says why and
what a successor must show first. Each sweep item is a
`CONTRIBUTING.md` rule; the sweeps exist because a rule nobody greps for is a rule the review
applies only when it happens to notice. When a review finds slop
neither list names, the fix is a new entry here or in `CONTRIBUTING.md`, not a longer review:
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

**Second opinion.** A model reviewing its own work shares its own blind spots. The sidekick's
code is reviewed cross-family by the lead; what the lead writes itself (plans, acceptance
sections, briefs, converge audits, decision entries) gets a critique from a reviewer of a
*different model family* (the `duck` skill: Astra, with SWE-2 rounds around it) at these
checkpoints only: (a) after writing an Acceptance section; (b) after a worklog step that added a
new `pub` interface or touched more than one crate, once its code and tests are written; (c) as
the second half of every converge audit, after the lead's own, the one check of code against the
plan by a model that wrote neither; (d) reactively, when two attempts on one premise have failed.
Renames, doc edits and one-file fixes ride along with the next (b) review or with converge; at
most one scheduled critique per step. Hand the reviewer pointers (plan sections, paths, a diff
file, acceptance IDs), never a summary of your own. Its brief asks for every concern, ranked by
reachability (what real files and the crate's own operations reach first; a hand-built input
ranks low unless it panics, hangs or writes a wrong file without an error), and for
over-engineering, and never names a number of concerns. The lead answers every concern in its
turn report: accepted, with what changed, or rejected, with one line why. **True is not enough
to accept**: each acceptance names its reach, (a) a real export whose outcome it changes, (b) a
contradiction with the code, another rule or a scenario, (c) a THEN that cannot be observed, or
(d) a wrong fact; a concern with none is rejected however true it is, and a nit fixed in passing
does not count toward the five. Another round runs only
when at least five of the last round's concerns were accepted; each round is a fresh reviewer,
never a resumed one; past the fifth round on one surface the lead reports the counts and classes
to the maintainer and continues. A review stopped for quota is queued in the worklog's "Handover
queue". The full rules, with the measurements behind each, are in `docs/RUNBOOK.md` "Review
rounds".

## Read order by task

You do not need to read `core/README.md` end to end. Read this file and the worklog's
"Current status", then the rows below. Each document they name may exist in both trees,
`docs/spec/` for what is built and `docs/plans/` for what is not; read both copies when both
exist.

| Working on | Read |
|---|---|
| Any code change | `docs/CONTRIBUTING.md`, then the row below that matches; `docs/GLOSSARY.md` whenever a term is unfamiliar; do not infer domain terms from their English meaning |
| A format or leaf lib crate (`cpk`, `fpk`, `fmdl`, `pes_model`, `ftex`, `dds_convert`, `fox2`, `uniparam`, `wezlib`, `vtree`, `archives`, `fpc`, `teams_list`, `color_tools`, `elevation`) | `libs/README.md`; for `fmdl`/`pes_model` also "Blender integration" in `model_conversion/gltf.md` |
| `kit_config` | `libs/README.md` for the crate, `kit_config_editor.md` for the format reference |
| `python_bindings` | "Blender integration" in `model_conversion/gltf.md`; guardrail 4 in `core/architecture.md` |
| A tool crate | That tool's plan, plus "Tool plugin interface", "Event system", and the tool-crate skeleton under "Crate structure" in `core/architecture.md` |
| Anything that reads or writes exports | `aesthetics_export/README.md` (object model, folder conventions, validation) |
| `studio_core` or `studio` | `core/architecture.md` and `core/gui.md` |
| Model conversion or glTF | `model_conversion/README.md`, `model_format.md` |
| Savefile | `pes_savefile/README.md`, then `save_editor.md` |

Several documents carry "Resolved decisions" / "Open questions" sections. Check them before asking:
your question may already be listed, with the phase in which it gets resolved.

## Fundamental concepts

- **Speed is the primary goal.** The suite exists because the Python tools are slow. Everything
  else is second, or tied for first.
- **Platform + plugins.** `studio_core` holds only what runs with zero tools installed (shell,
  `StudioTool` trait, settings framework, common widgets, `PipelineEvent` types). Every tool is its
  own crate under `crates/tools/`; every piece of shared functionality is its own lib crate under
  `crates/libs/`; `studio` is the crate of the single binary, `UTW`, that registers tools
  and launches the GUI or dispatches `UTW <tool-id> <command>`.
- **Two engines, one IR.** PES 15–17 use pre-Fox formats (`.model` + `.mtl`); PES 18–21 use Fox
  formats (FMDL, FPK, FTEX). Cross-format model conversion goes through `model_convert`'s IR at
  compile time; same-format work stays in the format crate's `ops/` and skips the IR unless the plan
  names an IR operation (hand auto-split, cross-version skeleton retargeting). Authoring format is glTF
  with optional `PES_bone`/`PES_mesh` extensions plus `materials.toml`.
- **Exports are the unit of work.** Three kinds: aesthetics (player folders with `settings.toml`,
  kits, portraits: the project's primary motivation; referee exports are aesthetics exports with
  the `/refs/` team name), music (`.4ccm`), balls. Aesthetic settings that used to be edited in the
  savefile by hand live in TOML inside the export and reach the game at compile time, written
  to the savefile through the aesthetics patch on every engine. The old export
  layout is not supported by the compiler; the Export upgrader migrates it
  once.
- **The upgrader absorbs the old format's slack; the compiler stays strict.** An upgrade
  starts from a clean Red export and ends in a clean Studio export, or writes nothing: a run
  with an Error, or a finding that drops what the old export compiled, leaves only its report,
  and the fix is to the input (not a clean old export) or to the upgrader (it failed to
  migrate one). Never a half-migrated output. A leftover, content that never reached a player
  in game (a side folder of copies, a stray root file, a portrait or kit texture Red's
  pipeline never packed, a backup file in a face folder), is dropped with a Warning naming it,
  and the run stays clean. A quirk Red tolerated in old files (stray text in an `.mtl`) is cleaned
  by the upgrader, through a tolerant read only it uses, rather than tolerated by the compiler
  or a format crate's default reader: every Studio export passes through the upgrader once, so
  the quirk never reaches the compiler, which refuses it loudly when a member types it by hand.
- **User-facing TOML is edited, never regenerated.** `settings.toml`, `config.toml`,
  `materials.toml` carry app-injected per-field comments that *are* the user documentation. Write them through `toml_edit`, which preserves them; `toml` (serde) is for
  read-only parsing.
- **Legacy tools are evidence, not source.** Red, Blue, 4ccEditor, Midcupping, Rigdio, SEN:P-AI,
  the converters and the Blender addons define *what* must be produced (formats, observable
  behavior), never *how* the Rust code is organized: no line-by-line translation, no copied module
  structure. Red's CPK output is the parity golden standard. Marco's reverse engineering of the
  PES 2021 executable (`C:/Data/4cc/Tools_Marco/GodotFootball/docs/re/`, index `README.md`;
  save records, kits, faces, formats, each claim tagged observed or inferred) is evidence of
  the same kind, for PES 2021 only.
- **Findings, not messages, in libs.** Lib crates return stable finding codes with context; each
  tool's `messages.rs` maps them to user-facing text, severity and disposition. Tools report
  progress through `PipelineEvent`s addressed by `vtree::ScopePath`.
- **Parallelism is `rayon` + `crossbeam-channel`.** No async runtime anywhere in the workspace.
- **Inputs are trusted, not hostile.** Files come from cup members'
  exports and the game's own install, a fairly trustworthy environment. Malformed input still
  errors instead of panicking or writing a wrong file, but hardening against deliberately
  crafted files (allocation amplification, pathological sizes, characters no real file
  carries) is not a goal: a review concern that needs a crafted file to trigger is rejected on
  that ground, and reviewer briefs say so.

## Environment

The maintainer develops on Windows with PowerShell; Linux is a first-class target. Prefer Python
scripts over PowerShell one-liners for anything with quotes; never inline regex in PowerShell.
**Scripts never write a repo file in place**: `Path.write_text` truncates the file before it
validates its own arguments (one bad `newline=` emptied a 600-line source file that git had no
copy of). Write to a sibling temp path and `os.replace` it, and run every assertion before the
write. Small edits use the editor tool, not a script. Before editing a tree the sidekick left,
`git add -A` first, so every file, untracked ones included, has a blob in the index to restore
from. **No `git checkout -- <path>`, `git restore`, `git reset --hard` or `git stash` over
uncommitted work**: each discards whatever the index does not hold, and nothing in the reflog
gets it back (a checkout meant to undo a one-line perturbation also erased the unstaged golden it
sat in). A deliberate perturbation is undone with the editor tool, by hand; a git restore is
allowed only after `git add -A` has given every file a blob to restore from. Never redirect to
`nul` from bash: on Windows that creates a real file named `nul`, which git cannot index
(`git add -A` fails outright) and which only `Remove-Item -LiteralPath "\\?\<full path>"` can
delete; use `/dev/null` in bash, `$null` in PowerShell.

**Operations** are in `docs/RUNBOOK.md`, read before starting what each section covers:
mutation runs and the VPS that takes half of them (`just mutants`, `mutants-diff`,
`gates-remote`, and the memory caps that keep the host's production service first); the `.tmp/`
scratch layout and what an archived run keeps; the agents and their harness limits; and the full
review-round rules. Three model families play three roles: the lead (Claude) plans, briefs,
reviews and talks to the maintainer; the sidekick (SWE-2) implements; the reviewer (GPT Astra,
through the `duck` skill) critiques what the lead writes. Two operations rules every agent needs:
nothing edits a crate while its mutation run is queued or going (the run copies the tree when it
starts), and a failed remote half is reported, never restarted (`cargo mutants` does not resume).

## When the plan has gaps

The plan is thorough but not complete, and it will be wrong in places. When you find a gap, an
oddity, or think of an improvement, do not ignore it and do not work around it silently.

**Act, then log**, when *all* of these hold:

- it is implementation-level: no change to user-visible behavior, CLI/GUI surface, file formats,
  or anything the game reads;
- it stays inside one crate and adds no dependency, no crate, no `unsafe`, no `#[allow]`;
- the plan is *silent* on the point, not contradicted by your choice.

**Stop and ask** when *any* of these hold:

- the plan says X and you believe X is wrong (a contradiction is not a gap); never quietly "fix"
  the plan;
- it touches game-facing output, export text formats, the savefile, the UI, or the CLI;
- it needs a new dependency, a new crate, a crate-boundary move, `unsafe`, or a clippy opt-out;
- there are two reasonable options and no plan text picks one (user preference is involved).

**A recommendation is applied, not asked.** When the lead already has
a clear recommended choice for one of the cases above, it applies that choice and logs the
decision (plan edit, then decision entry), and the turn report names it so the maintainer can
reverse it. It stops to ask only when the choice is truly ambiguous (no option is clearly
better on the evidence) or needs information only the maintainer has (in-game behavior, cup
practice, a preference); such a question is written to `docs/QUESTIONS.md` (never with a
"Recommended:" line: a question the lead can recommend an answer to is one it applies), with
the worklog issue or step that waits on it pointing there, and the work goes on with whatever
does not depend on it. The rule covers choices inside the work asked for. **Something nobody
asked for** (a usability idea: a button, a shortcut, a default, a handoff between tools) is
not applied, however clear the recommendation: it is written to `docs/PROPOSALS.md`, with what
it adds and what it costs, and waits for the maintainer's ruling there (side-lead 8.0 rightly
asked about the upgrader's "Move to exports and compile" button, approved before this file
existed). An answer is open to discussion, an earlier one included: when the lead sees a
reason against it (a case it breaks, a cost it carries, a simpler rule), it says so with the
evidence before landing it, and the maintainer decides; an answer it agrees with needs no
argument. New dependencies and `unsafe` still need a yes first; a dependency the
plan already names (`rayon`, `crossbeam-channel`, ...) is accounted for, not new.

Logging a decision means, in this order: edit the relevant plan section so the plan stays the
source of truth (keep its "we do X, not Y, because Y causes Z" style), then append an entry to
`docs/DECISIONS.md` (format at the top of that file) immediately, not at the end of the turn.
Who decided and when is the entry's alone: no `(maintainer, <date>)` note in a plan, here or in
`CONTRIBUTING.md`.

## Conduct

Invoke the `karpathy-guidelines` skill before non-trivial work if it is available.

**The ownership standard.** Everything you produce must be maintainable by the maintainer without
an assistant present: code, plan text and decision entries alike. Code the reviewer cannot follow
and a plan section only the writing session understood are not done, whatever the gates say. When
an outcome fails this test, say so and simplify before reporting. (The code half is
`CONTRIBUTING.md`'s "boring Rust" intent; this is the standard it is accepted on.)

End every turn that changed something with a short report: what changed, which gates ran and their
result, what was not verified, second-opinion concerns and how each was answered, worklog steps
updated, decisions logged in `docs/DECISIONS.md`, and open questions.
