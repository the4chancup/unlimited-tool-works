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
| `docs/RUNBOOK.md` | How the lead runs the machinery: mutation runs and the VPS, the scratch folder, the agents and their harness limits, phases and releases, briefing and reviewing a sidekick, the review rounds' full rules |
| `docs/GLOSSARY.md` | One line per domain term, pointing at the spec or plan section that owns it |
| `docs/WORKLOG.md` | *Where we are*: current status, phase/step checklist, current-state gotchas, open issues (the lead's and the sidekick's) |
| `docs/LOG.md` | *What happened* up to 2026-11-11, frozen: since then the dated log is `git log` |
| `docs/DECISIONS.md` | Choices made where the plan was silent (append-only) |
| `docs/QUESTIONS.md` | The questions only the maintainer can answer (in-game behavior, cup practice, a preference), grouped by what an answer unblocks; an entry leaves when answered and the answer lands in the plan and `DECISIONS.md` |
| `docs/PROPOSALS.md` | Usability ideas nobody asked for (a button, a default, a shortcut), which the maintainer vetoes or accepts; an unruled one is not built |

**Before a step:** read the worklog's current status, then the plan section the step points at,
from the document, not from memory. **After a step:** run the step's checkpoint gates
(`CONTRIBUTING.md` "Testing and verification": what runs per commit, what at a checkpoint), collapse the
closed step to one line (ID, date, commit, what it delivered), update "Current status",
commit (`python scripts/commit.py <message file>`, the only way to commit or merge: it checks
the message first). Once a release exists, a change a
member would notice gets its `CHANGELOG.md` `[Unreleased]` line (`CONTRIBUTING.md` "Commits").

**Phases, releases and sidekicks** follow `docs/RUNBOOK.md`, read before doing what each
covers: "Phases and releases" (a tool phase opens with its Acceptance section; a phase closes
by converge, the spec move, the worklog collapse and the README; a release cut with phases
open runs the converge first), "Lead and sidekick" (what the lead hand-writes, a brief's
shape and its "Errors hit" section, the diff review's sweeps and mutation run) and "Review
rounds" (the cross-family critique and when it stops).

**This file stays under 16,000 bytes**: Devin injects 16 KB of it, Codex 32 KB, and a rule cut
off is followed by nobody. New procedure goes to `RUNBOOK.md` or `CONTRIBUTING.md`, with a line
here only when every agent needs it.

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
  structure. Red's CPK output is the parity golden standard; a phase's compile of the library is
  measured against our own last checkpoint, not Red (`RUNBOOK.md` "Phases and releases"). Marco's reverse engineering of the
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
how the work runs (the release path first, `just checkpoint`, one home per event);
mutation runs and the VPS (`just mutants`, `mutants-diff`, `gates-remote`, and the memory caps
that keep the host's production service first); the `.tmp/` scratch layout and what an
archived run keeps; the agents and their harness limits; phases and releases; lead and
sidekick; and the full review-round rules.

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
it adds and what it costs, and waits for the maintainer's ruling there. An answer is open to discussion, an earlier one included: when the lead sees a
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
