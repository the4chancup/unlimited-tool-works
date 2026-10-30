# Unlimited Tool Works - Coding rules

Read this in full before writing or changing code. `AGENTS.md` covers orientation, the working
documents, and conduct; this file covers how the code is structured, written, and verified. Every
rule here is enforced in review.

## Architecture rules

Workspace guardrails (details: `spec/core/architecture.md` "Workspace guardrails"):

1. Tool crates never depend on tool crates. Shared logic moves down into a lib crate.
2. Every shared dependency is declared once in the root `Cargo.toml` `[workspace.dependencies]`;
   crates inherit with `dep.workspace = true`. egui must be in lock-step workspace-wide.
3. Don't over-split. A lib crate earns its existence through multiple consumers or by being a
   conceptually standalone format. No per-tool companion libs; the Team compiler stays one crate.
4. `fmdl` and `pes_model` stay PyO3-buildable: no dependency on `studio_core`, `studio`, any tool
   crate, `model_convert`, `egui`, `eframe`, `wgpu`, `tokio`, `smol`, `async-std`, `pyo3`.
5. `cargo clippy` is clean with `-D warnings` from the first commit. Lints are declared once in
   `[workspace.lints.clippy]` (defaults plus `let_underscore_must_use = "deny"`); a per-crate
   `#[allow]` is an exception to call out, not a default. Prefer `#[expect(lint, reason = "…")]`
   over `#[allow]`: it errors when the lint no longer fires, so suppressions cannot go stale.
6. Every lib crate and `studio_core` must pass `cargo check --target wasm32-unknown-unknown`.
   Native-only parts go behind `#[cfg(not(target_arch = "wasm32"))]` or a cargo feature.

Placement:

- Every tool crate has the same skeleton: `lib.rs` (the `StudioTool` impl, wiring only),
  `settings.rs`, `cli.rs`, `messages.rs`, `view/` (egui, render-only: reads state, emits intents),
  and a `help/` folder of numbered Markdown topics beside `src/`: the tool's chapter of the help
  window, written for members (what to put where, what a message means), never about the code.
  A user-observable behavior change lands with its topic in the same diff. Logic goes in modules
  next to these.
- `fmdl` and `pes_model` are split into `format/` (pure `binrw`, byte-identical round-trip),
  `ops/` (format-native algorithms on the crate's own types) and `check.rs` (findings).
- The two tempting misplacements into `studio_core`: format knowledge (→ `aesthetics_export`) and a
  widget used by one tool (→ that tool's `view/`). `studio_core`'s module tree is closed.
- `docs/crate_map.html` draws every crate and the workspace crates each one uses, in layers from
  the binary down to the libs that use nothing (hover a crate to see its arrows). A commit that
  adds a crate or changes the workspace dependencies runs `just crate-map` to redraw it; no gate
  checks it, so a missed run only leaves the map stale until the next.
- Directory name = package name, with underscores (`pes_savefile`). Hyphens only in CLI subcommands
  and tool ids (`team-compiler`).
- No `utils.rs`, `helpers.rs`, or `misc.rs`. A helper belongs to the module that uses it; one used
  by several modules belongs to the lib that owns the concept. Files, not folders, until a module
  passes roughly a thousand lines; then a folder with a `mod.rs` of the same name.

## Code style

**Intent.** This is boring Rust, for two reasons. First, the code is written largely by agents and
reviewed by a maintainer who is learning Rust as the project goes: code the reviewer cannot follow
is code nobody has verified. Second, most of the rules below close off the paths an agent takes
when fighting the compiler rather than the problem: a trait with one implementor, `Box<dyn Trait>`
for a closed set, an `Rc<RefCell<_>>` graph, a `macro_rules!` for repetitive impls, an `unwrap()`
to make an error go away. Nothing in this domain (binary formats, file I/O, a `rayon` pipeline, an
egui shell) needs anything cleverer than what these rules allow. So: idiomatic at the statement
level (clippy enforces that), boring at the type and architecture level. If a reader has to
understand a trait hierarchy, a lifetime dance or a macro to follow the data, the code is too
clever for this project.

**Rules: defaults that yield to measured performance.** Where a simplification would noticeably
slow the program, keep the fast version and leave a one-line comment saying why it is shaped that
way. Performance-motivated complexity cites a measurement or the plan section that mandates it;
speculative optimization is just as unwelcome as speculative abstraction. Where no measurement can
exist yet (every Phase 2 crate), the plan's data-shape rules stand in for one.

- Plain structs, enums and free functions. A trait needs at least two real implementors (a test
  double or a backend the plan names counts); `enum` + `match` over `Box<dyn Trait>` so the
  compiler catches the missing case; no `Box<dyn Any>`.
- Newtypes for validated identities at crate boundaries (`TeamName`, `TeamId`, `PlayerSlot`,
  `BootsId`, `CpkStem`, …). A raw `String` or `u16` that has been validated should not leave the
  function that validated it.
- Closed sets are enums. A string that gets compared against literals, or matched with a `_ =>`
  arm, is an `enum`: `serde(rename_all)` for the file format, `Display`/`FromStr` for the CLI. A
  `_ =>` arm on one of our own enums is a smell; it is where the compiler stops catching the
  missing case. The same holds for `pes_version::Engine`, which may grow a third engine (PES 12,
  worklog 4.0a): it is branched on with an exhaustive `match`, never tested against one variant
  with `==`/`!=` (comparing two engines is fine), and never derived from an ordering of
  `PesVersion` (`>= Pes18`), so a new engine is a compile error at every site that must decide,
  not a silent misroute.
- Borrow parameters (`&str`, `&Path`, `&[u8]`); own what you store and return (`String`,
  `PathBuf`, `Vec<T>`). Generics over std traits (`impl Read + Seek`, `impl Write`,
  `impl Iterator<Item = T>`) are fine; generics over our own traits, and struct lifetimes, only
  where the plan asks. Bulk data (file bytes, textures, decoded images, mesh buffers) is never
  cloned across an API: `Arc<[u8]>`/`Arc<T>` as the plan specifies, or borrowed. Small values
  clone freely.
- `pub(crate)` by default. A `pub` item is crate API: something another crate, the CLI or the
  Python bindings is meant to call. Tool crates expose almost nothing beyond the `StudioTool` impl.
- Derive macros (`binrw`, `serde`, `thiserror`, `clap`) are the point; use them. No `macro_rules!`
  for anything a function can do.
- Iterator chains are fine as long as they read as one sentence; when one stops doing so, name the
  intermediates. A readability guideline, not a `for`-loop rule: never rewrite a `rayon` `par_iter`
  chain into a sequential loop.
- Early returns over nesting. Small functions with one job. Full-word names; the only abbreviations
  are domain terms (`cpk`, `fmdl`, `fpk`, `ftex`, `mtl`), conventional math/loop names (`i`, `n`,
  `x`/`y`/`z`, `u`/`v`) and the ones std itself uses (`len`, `buf`, `iter`, `ptr`).
- The third copy is a function. Before writing a function, read the one above it; two functions
  that differ in a name and a branch are one function with a parameter. Copy-paste is how a
  module doubles in size without gaining a concept.
- A module is a noun, not a layer. If saying what a module is *about* needs an "and", it is two
  modules. A `commands.rs`, `handlers.rs` or `state.rs` that grows by accretion is `utils.rs` with
  a better name.
- Shared mutable state: one lock per concept, never per field, and a fact lives in one place.
  Prefer an immutable `Arc<Snapshot>` swapped whole over a `Mutex<T>` mutated in place; never
  mirror fields of one struct into separate locks so another thread can read them. A `Mutex<()>`
  used as a semaphore is an ask.
- Errors: `thiserror` enums in lib crates, `anyhow` in tools and the binary, `?` everywhere.
  `unwrap`/`expect` only in tests, or on a true invariant with a message that says why it holds.
  `.lock().unwrap()` on a `std::sync::Mutex` is the one bare `unwrap` allowed: it asserts "no
  thread panicked while holding this lock", which needs no message.
- A discarded `Result` is a decision, and `clippy::let_underscore_must_use` (denied workspace-wide)
  makes `let _ = fallible()` a compile error. The ways out, in order of preference: handle it
  (`if let Err(e) = … { log::debug!(…) }` is the usual answer for a failure nobody needs to act on);
  route it through one place (a `send` to a receiver that may be gone belongs in one `emit` method
  that documents the disconnected case once, not in `let _` at every call site); or
  `#[expect(clippy::let_underscore_must_use, reason = "…")]` on the statement for the truly
  ignorable; `expect`, not `allow`, so a suppression that stops firing is itself an error.
  Discarding through `.ok();` is the same thing without the reason and is forbidden.
- Diagnostics go through `log` (`debug!`/`trace!` in libs, `info!` for stage milestones in tools);
  user-facing findings go through `Message`. Never `println!`/`eprintln!` outside `studio`'s CLI
  result output and `main`. Libs never `error!`: return the error, let the caller decide. No
  `[module]` prefixes; the target carries the module path. Full rules: `spec/core/architecture.md`
  "Diagnostic logging".
- `unsafe` only where an OS API forces it (`match_feed`'s memory source, `elevation`,
  `studio_core`'s running-PES poll and its title bar without an icon, `pipeline`'s available-memory read on Windows) and in `python_bindings`, each block preceded by a `// SAFETY:`
  comment. Anywhere else it is an ask. Review checks for the comment specifically: agents do not
  write it unprompted.
- `///` doc comment on every `pub` item, written for the PES-literate non-Rustacean: what it is in
  format or pipeline terms, and any invariant the signature does not show. A doc that restates the
  name (`/// Returns the name.`) is worse than none. Inline `//` comments only for non-obvious
  *why* (format quirks, perf rationale, SAFETY), never to narrate what the next line does.
- **Code does not refer to the legacy tools or to the project's history.** No tool the suite
  replaces or mirrors (Red, Blue, 4ccEditor, pes-file-tools, the Blender add-ons, the converters,
  texconv) by name or by allusion ("the reference", "the add-on", "the 4cc compilers", "as the
  scripts did", "has always produced"), and no Python function names, in doc comments, comments
  or identifiers: the code describes the format or the behavior as it is ("the layout PES
  accepts", "one frame per mip"), and the plans hold why and which tool showed it. Naming the
  game, Konami's own files or a format's specification owner (DirectX for DDS, CRI for CPK) is
  describing the format, not history. Exceptions: a parity test, whose purpose is comparing with
  a legacy tool's output, may name that tool; a literal an existing file carries (a `Tvers`
  string a parity test must reproduce); and the provenance line of a committed fixture in its
  `README.md`.
- **A comment stands on its own.** It is plain sentences a maintainer understands on one
  reading without opening another file: what the item is, or why the code is shaped this way.
  It never cites a plan (`plans/…`, "the core plan", "Phase 8"), `DECISIONS.md` or a decision's
  date, the worklog or the log, a step (`0.0.1m2`, `5.6e`), a review round or concern (`S4a-2`,
  "round A1") or an agent (the sidekick, the reviewer): plans change and move, steps and rounds
  are history, and a chain of pointers in parentheses is what makes a comment unreadable. Write
  short sentences, one idea each, and name things the way the code and the UI do. When the why
  takes more than a few lines, the comment gives the short version and points at the one
  `docs/spec/` section that holds the rest, by file and heading (`spec/core/gui.md` "Scaling
  with the width"), never at a plan; a tooling file (`Cargo.toml`, `.cargo/`, the justfile,
  `scripts/`) may point at this file or `RUNBOOK.md`, which specify the tooling, and a test at
  its fixture folder's `README.md`. A view's code may name the element of its draft it
  reproduces (`the draft's .lrow`): the draft is the view's specification. A test's
  acceptance-ID line is not a comment in this sense (below, "Testing"). `just comment-refs`
  lists every comment that breaks the pointer half of this rule.
- No child→parent back-references, anywhere: pass resolved identity/context down as parameters; no
  `Rc<RefCell<_>>` graphs. `Arc` for sharing immutable leaf data across threads (textures,
  snapshots, wake callbacks) is not a parent reference and is what the plan intends.
- **Text a member reads is written for a layman**: the help chapters, the catalog rows' When and
  Effect, UI strings and the injected `settings.toml` comments say, in plain words, what happened
  and what the member can do about it, as easy to follow as the fact allows. None of the code's
  or the docs' own vocabulary reaches them: no disposition names (`DropExport`), no pointers into
  `docs/`, no internal field or type names, no legacy tool names; a file, folder or setting the
  member handles is named as they see it. A value the program spells for itself (a rules file's
  `attacking_prowess` or `a_positions`, a violation kind's `FieldedNotA`, a `{:?}` print) is
  shown as the game names it (Attacking Prowess), on the command line too: those spellings are
  for the program, and rule makers read the game's names as members do. A member who cannot follow a message asks in the cup's
  chat instead of fixing the export, which is the cost the rule exists to avoid.
- **That text flows naturally**, as a person would say it: whole sentences in ordinary word order,
  a cause joined to its effect with *because*, *so* or *although* rather than facts stacked behind
  semicolons and colons, and no shorthand from the code ("gating", "widgets", "targets"). Correct
  but dry: "This save is PES 17; the selector targets PES 2019. The save keeps its own version:
  its widgets and gating stay PES 17's." Natural: "Although the sidebar is set to PES 2019, this
  save is from PES 2017, so the editor will keep treating it as a PES 2017 save." A member reads the
  dry version twice and still wonders what to do; the review of a change that adds such text
  reads each new line aloud.
- **A help topic is read in pieces, one subject each**: a paragraph holds one subject in a
  few sentences, a topic with several subjects gives each its own subheading (a line of bold
  text alone, `**Teams and players**`, which the window and the drafts draw alike), and
  parallel items (a menu's entries, a dialog's options, the steps of a procedure) are a list,
  one item per line. A paragraph that runs through five facts one after the other reads as one
  long line in the help window's wide pane, and a member scanning for one fact finds none
  (the maintainer, of the Save editor's chapters).
- **No em dashes in text a user reads**: the README, the help chapters, message catalog text,
  UI strings, the comments injected into `settings.toml`/`config.toml`/`materials.toml`, the Team
  creator's `README.txt`. Use a comma, a colon, parentheses or a new sentence. The character has
  become a tell for machine-written prose and readers react to it, not to the sentence; the
  internal documents (plans, worklog, decisions) are not held to this, but new text there should
  not add to the count either.

## Testing and verification

The gates, run before claiming anything is done:

```
just gates
```

The root `justfile` is the one definition of the gates (`cargo fmt --all --check`, `cargo clippy
--workspace --all-targets -- -D warnings`, the tests `cargo test --workspace` builds, run four
binaries at once by `scripts/run_tests.py` and then the doc-tests, `cargo check --target
wasm32-unknown-unknown` for `studio_core` and every lib crate, and `just acceptance`, the
acceptance-ID scanner in its `report` mode, and `just doc-pointers`, which fails on a
pointer to a document's section, its path and then the quoted title, whose file or section is
gone, or whose file is in the gitignored `.tmp/`, which no clone has, and `just comment-refs`, which fails on a code comment citing a plan, a decision, a step,
a review or who decided, and `just colour-literals`, which fails on a colour written from literal
values outside `studio_core`'s `theme.rs`, whose palette gives each colour its light value), and
CI runs the same recipe, so
the list cannot drift between this document, the local run and CI (it had, before the justfile
existed: two spellings of the fmt gate in two documents). The test gate runs each binary whole,
its tests on libtest's threads, several binaries at once: `cargo test` alone runs them one after
another, leaving cores idle behind each binary's slowest test (277 s against 176 s, 2026-10-25),
and `cargo nextest`'s process per test was slower still on Windows, where every test binary
starts through the memory-cap wrapper. `just gates-changed [base]` runs the same gates with the
test gate over the packages the working tree changes since `base` (default `HEAD`) and every
package depending on them; a changed file outside every package, `docs/`, the root's `*.md`,
the mutation runs' exclusion files and the scripts no test run goes through (every script but
the runner and its memory-cap wrapper: no Rust test reads one, and the scripts' own
`*_test.py` the gates list run whatever changed) runs every package's tests, since the toolchain, `Cargo.lock`,
`.cargo/` and `resources/` reach
them all.

How often each runs. A commit that touches only `docs/` or the root's `*.md` runs `just
doc-pointers` and `just acceptance`. A code commit reruns, in the real workspace, the changed
packages' own tests (`cargo test -p <each>`), `cargo clippy -p <each> --all-targets -- -D
warnings` and `cargo fmt --all --check`: the sidekick's pasted runs are a claim, and these are
the checks a slice's own change fails first. `just gates-changed <the last commit it passed
at>` runs once over every commit since, at a checkpoint: the end of a surface's review loop,
right before its mutation run ("Mutation runs" below), which then starts from a tree known
green; a worklog step's close; a merge of another branch. The full `just gates` runs before a
release build, and CI runs it on every push. A failure a checkpoint finds in a dependent
package is fixed by a commit of its own (or an amend of the newest unpushed one) naming the
commit that caused it: the history stays local until the push, so a later find costs a short
search over a few small commits, while running every dependent's tests on every commit cost
minutes of the whole machine each time once the suite passed 5000 tests (2026-10-27: one
commit to `studio_core` ran all 32 packages' tests). The other recipes are the project's
repeatable sequences, each of which the plan names somewhere: `just deps-check` (guardrail 4,
the `fmdl`/`pes_model` denylist, plus the `cargo deny` license allowlist; see "License" in the
core plan; run it right after any change that adds a package to `Cargo.lock`, a transitive one
included, because `just gates` does not run it: 3.z's `eframe` brought two `BSL-1.0` crates that
only CI saw), `just acceptance strict` (the scanner's converge mode, under "Requirements"), `just parity`
(the two Red-parity tests alone, `tests/parity.rs` and `tests/parity_prefox.rs`, which `just test`
also runs), `just bindings` (the `maturin` build of `python_bindings`), `just release <version>`
(arrives with release 0.1.0, `core/development_plan.md` "Release 0.1.0"), `just
mutants <crate>`, `just mutants-collect` and `just mutants-diff [base]` (the mutation runs,
below), `just citation-audit` (the citation audit, below), `just gpu-tests` (`dds_convert`'s GPU
encoder tests: the one sanctioned `#[ignore]`, each with the reason `"needs a Vulkan or Metal
GPU"`, since no CI runner has one; whoever changes `dds_convert`'s `gpu` module runs them on a
machine that has one before it lands, and the review pastes their result). `just --list` shows
them with a one-line description each.

On Windows every binary `cargo test`, `run` or `bench` launches goes through
`scripts/test_runner.cmd` (`.cargo/config.toml` `runner`), which caps each process at 6 GB with
a Job Object. Windows has no OOM killer: an unbounded allocation (a mutant, or the same bug
written by mistake) would otherwise page the machine down until some unrelated program's
allocation failed. At the cap the test's own allocation fails, it aborts (`0xC0000409`), and the
runner prints `test_runner: <exe> hit the 6G per-process memory cap`. The largest normal test
binary peaks under 0.5 GB (`pes_savefile`, 2026-09-29). rustc is not capped, so the cap cannot
turn a mutant unviable. The figure matches the VPS mutation unit's `MemoryMax=6G` only in
number: that one caps the whole run (builds, both workers, page cache), this one each test
binary alone. Linux relies on the kernel's OOM killer.

Rules for the justfile, so it stays a command list and not a second build system:

- A recipe is a list of commands, never logic. Anything with a branch or a loop is a Python
  script under `scripts/`, and the recipe calls it. Reading the justfile top to bottom must tell a
  maintainer exactly which commands run, in which order.
- Recipes run under `sh` on Linux and CI and under PowerShell on Windows (`set windows-shell`;
  Git for Windows' `sh` is not on PATH from a plain PowerShell, and putting its `usr/bin` there
  shadows `find`, `sort` and `link.exe`). Two shells means recipe lines are bare commands: no
  pipes, no redirection, no quoted arguments; an argument that needs quoting goes into the script
  the recipe calls. The one exception is a recipe parameter, which the caller supplies and the
  script cannot hold: it goes through just's `quote()` (`{{quote(interpreter)}}`), a
  single-quoted literal both shells read as one argument, so a path with a space stays whole.
  Its escape for an apostrophe is the POSIX `'\''`, which PowerShell does not read, so on
  Windows a parameter containing `'` still breaks the recipe. That is also what makes the PowerShell false-failure trap below not apply to
  the gates: `just` stops at the first non-zero exit.
- Nothing hardcodes `target/`: a developer may have moved the target directory (`build.target-dir`
  in a global cargo config), so a recipe that needs a built artifact asks cargo where it is
  (`cargo metadata` → `target_directory`, or `cargo build --artifact-dir`).
- `just` is a developer tool, installed like rustup (`cargo install just`; CI installs it too); it
  is not a workspace dependency and does not appear in "External Dependencies". The cargo commands
  keep working without it; the justfile is the definition, not a requirement to build. `maturin`
  (`pip install maturin`) is the same kind of tool for `just bindings`; `scripts/bindings_check.py`
  runs it and reads the wheel path from its output, so the recipe never hardcodes `target/wheels`.
  Pillow (`pip install pillow`) is the same for `just test`: the drafts tooling's tests
  (`scripts/draft_compare_test.py`) measure text ink in images through it.

Requirements:

- Every `format/` module has a round-trip test on real file fixtures: parse → write → parse must be
  equal, and for `format/`-only paths the bytes must be identical. A `binrw` derive that compiles
  proves nothing about offsets, widths or endianness; fixtures and independently decoded fields do.
- Fixtures live in each crate's own `tests/fixtures/`. Real PES-derived files may be committed,
  kept minimal so cloning stays fast: the smallest file that exercises the code path, one per
  format variant rather than one per team, an FPK or FMDL rather than the CPK that held it, reuse
  before adding. Copy a small sample needed by several crates; ask before duplicating a large one.
- New behavior comes with a test that fails before the change and passes after. A regression test
  that passes on the old code has not demonstrated the bug. A test of a write first asserts its
  premises: that the target does not already hold the value it expects (`assert_ne!` on the
  value before the write), and that its setup changed what it meant to change. Where a
  fixture's value coincides, a skipped write passes the test. Three review rounds
  at step 5.1d found such tests, among them one that compared a name against a different player
  and one that swapped two roster slots without moving the player it patched.
- **A test fails, it never hangs.** A loop that waits for a condition (frames until a state
  shows, Ctrl+Z until the undo stack is empty) has a bound and panics past it, naming what
  never happened. Broken code then fails in seconds; unbounded, it spins until the mutation
  run's timeout: at 2026-10-21's `mutants-diff 12ffa8c3` one such helper hung ten mutants for
  17 minutes each.
- **A worker's in-between state is read on a frame no reply can reach.** A view that starts a
  check on a worker thread and marks the list as checking until it replies cannot be tested
  by drawing the frame that starts it: egui runs a window's or panel's first frame in two
  passes, and the worker can reply between them, so the frame draws the result instead (8.9j3:
  one full-suite run in eight). The mark is read on a frame where no check can start (the
  pointer held over an unsettled edit); the test never widens its assertion to pass both ways.
- **A test never reaches the developer's own files.** A setting whose default finds something on
  the machine (`savefile_path = auto` walks Documents\KONAMI, `pes_folder_path` names the real
  install) is set to a path under the test's own folder by every test context that runs the
  tool: the crate's CLI sandbox and its in-process `tool_context` alike, inserted before the
  tool's defaults are merged in (a default already merged is not replaced). At 5.3b2 a context
  that missed this let three GUI tests apply their patch to the maintainer's PES 21 save.
  Machine state is held to the same rule: a test gets the running PES games (the process list)
  through the tool's probe, which it sets, and never reads them directly.
- **Acceptance IDs.** Each tool plan has an "Acceptance" section listing that tool's user-observable
  behavior as scenarios with stable IDs, written before the phase that delivers them (procedure in
  `AGENTS.md` "Working documents"). Format, one entry per behavior:

  ```
  TC-KIT-03  GIVEN a kit folder without colors.txt
             WHEN the export is compiled
             THEN kit colors are derived from kit.dds AND finding kit_colors_derived is emitted
  ```

  `TC` is the tool's short code (`TC` team compiler, `SE` save editor, `EU` export upgrader, `SC`
  stadium compiler, `MP` music player, `ME` music export editor, `MT` match tracker, `KC` kit config
  editor, `RA` refs arranger, `BC` balls compiler, `PA` player aesthetics editor, `RE` ruleset
  editor; `ST` the shell,
  which is no tool: its scenarios are in `plans/core/gui.md` "Acceptance"), `KIT` an area
  within the tool, the number never reused once assigned; a withdrawn scenario keeps its ID with
  the text `withdrawn: <reason>`. Scenarios describe observable outcomes (files written, findings
  emitted, savefile fields set, CLI output), never internal structure; if the implementation could
  change without the scenario changing, the scenario is fine. Behavior defined by a fixture or the
  parity standard (byte-identical CPK, format round-trips) is not restated as a scenario.
- The test that proves a scenario carries the exact ID in a `//` comment on the line above the
  `#[test]` (`// TC-KIT-03`), so `rg TC-KIT-03` finds both the requirement and its proof; the
  function name is free to be descriptive. One test may prove several IDs (`// TC-KIT-03,
  TC-KIT-04`, or one ID per comment line, all directly above the `#[test]`); one ID may need
  several tests. A scenario without a citing test is what the phase-closing converge audit looks
  for.
- Every test of a tool's CLI suite (`crates/tools/<tool>/tests/cli/`) cites at least one
  scenario. The suite runs the tool end to end and observes what a member sees (findings, the
  files written, the exit code), which is what scenarios describe, so a CLI test citing none
  is behavior the Acceptance section does not state: add the scenario, or widen the one the
  test varies (another engine, file kind or not-found case); a test pinning a past bug cites
  the scenario of the behavior it protects. Unit tests cite none: they check internals, which
  scenarios never describe, and a citation is read as proof of the scenario it names. The
  scenarios then read as the tool's whole behavior, without the plan files or the code. A
  GUI scenario's test is not a unit test in this sense, though it sits in the crate's own
  tests: it drives the view or the shell headless as the member does (a frame, a click, the
  setting the sidebar writes) and observes what the window shows, so it cites its scenario.
- A scenario no automated test can prove (GUI interaction, a native dialog, a running PES) is
  marked `manual` after its ID in the Acceptance section. Its proof is a line in the worklog's
  converge step (`TC-GUI-04 manual: checked 2026-11-02, <what was done>`), and the converge check
  treats that line as the citation; collapsing the phase's step list keeps these lines. Prefer
  an automated proof whenever the UI toolkit allows one; `manual` is for what it does not.
- `scripts/acceptance.py` checks both halves. Every mode fails on a citation that proves nothing:
  one naming an ID no scenario defines or a withdrawn one, an ID-only `//` comment not directly
  above a `#[test]`, a manual check of a scenario not marked `manual`, an ID defined twice, or
  an Acceptance-section line that starts with an ID but is not a scenario. `report` (gate 5)
  lists unproven scenarios without failing, since an open phase's scenarios precede their
  tests; `strict`, run at converge, also fails on each of them. Strict checks every scenario in
  the plans: written just in time, they all belong to the closing phase or an earlier one, and
  an earlier phase's proof that disappeared is a regression.
- **Citation audit.** `acceptance.py` proves a citation names a live scenario, not that the
  test proves it: a test checking a case its scenario does not state passes every gate, and
  the scenarios stop reading as the tool's whole behaviour. `just citation-audit` asks SubQ
  Snap (a decision model with a 384K-token window, hosted by Subquadratic) of every (scenario,
  citing test) pair whether the test proves the scenario, given the scenario's text, the test
  and the functions of its own file it calls, and lists the pairs under P 0.3
  (`snap.out/citations.md`, gitignored). Measured on 2026-10-17 (`DECISIONS.md`): the 732
  citations then in the code against the same tests paired with a neighbouring scenario they
  do not cite, AUC 0.995; 32 flagged, the three read in full all real (a refusal test citing a
  filter scenario, a deadlock regression citing the determinism scenario, a gloves-ID test
  citing a boots-ID scenario), and both citations the project had removed as wrong scored
  under 0.3. Every cited scenario had a test above 0.5, so a flag says a test checks an
  unstated case, never that a scenario is unproven. It runs at a phase's close (the
  design-health pass), never as a gate: it needs the network and a key, read from
  `SUBQ_API_KEY` or the file `STUDIO_SNAP_KEY_FILE` names, outside the repository (one key
  a line, spent in order: a key the service refuses is left for the next). The lead
  runs it and rules each flag like a reviewer concern, its row (the report prints it) going
  into `scripts/citation_rulings.md`: accepted, the scenario is widened, a scenario is added,
  or the test's citation moves to the scenario it proves; rejected, with a one-line reason.
  Answers are cached by state (`snap.out/cache.json`), so a second pass pays for changed pairs
  only; a full pass sends about 1.5M tokens ($0.08).
- Code must build on Windows and Linux. No external converters or runtimes (texconv, 7z, ffmpeg,
  libmpv are all replaced in-process); the only subprocesses are the ones the plan names (Blender,
  PES, the elevated self-relaunch).
- **Mutation runs.** The gates prove the tests pass; they do not prove the tests would fail on a
  wrong implementation. `cargo-mutants` (installed like `just`: `cargo install cargo-mutants`)
  measures that directly by rewriting one function or operator at a time and rerunning the
  tests; a mutant the suite does not catch ("missed") is an assertion nobody wrote. It runs at two
  points, never as a gate (a whole-workspace run is an hour today and grows with the code):
  `just mutants-diff [base]` over the lines a surface's review loop changed, once at the loop's
  end (its base the parent of the loop's first reviewed commit, its run right after that
  checkpoint's `just gates-changed`), its survivors going into one rework brief, whose own
  tests the next checkpoint's run measures; not once per rework round, since each run freezes
  the crates it mutates for its whole length and its baseline repeats the checkpoint's tests;
  and `just mutants <crate>` over each of a phase's crates at converge (when
  `STUDIO_MUTANTS_REMOTE` names an ssh host, it runs half the mutants there; prerequisites in
  `scripts/mutants.py`). Every survivor is
  triaged into one of three: a missing test (write it, or a worklog step), an equivalent mutant
  (the mutated code computes the same value; its pattern goes into `.cargo/mutants.toml` with
  the equivalence named, so that file is the list of what the runs no longer measure), or
  unreachable code (a design finding). A survivor with no bucket is a review finding, not a
  number to accept. Results land in `mutants.out/` (gitignored); a diff run's in
  `mutants.diff/<crate>/`, one cargo-mutants run per crate so each times its mutants out
  against its own baseline, every crate's survivors in `mutants.diff/missed_all.txt`. What a clean run cannot
  vouch for: `cargo-mutants` rewrites operators, a `!`, match arms and guards, a field, and a
  function's whole return value; it never edits a literal or removes a method call. So `just
  mutants <crate>` first runs four such kinds (`EXTRA_KINDS` in `scripts/mutants.py`, listed by
  `crates/dev/extra_mutants`, run locally in a copy of the tree, a few minutes per crate and
  about twenty for `team_compiler`), their survivors in the same `missed_all.txt` tagged with
  their kind and triaged the same way: `literal` removes one alternative of a `matches!` or
  one element of a literal list a `split`, `contains` or similar method decides by;
  `normalize` removes a `.trim()`-style or case-changing call, or makes `eq_ignore_ascii_case`
  an `==`; `order` drops a statement that is a `sort*`, `dedup*` or `reverse` call; `bool`
  flips a `true`/`false` argument or struct field, the flags that land in a written file and
  that cargo-mutants never touches. An equivalent extra mutant goes into
  `.cargo/extra_mutants.toml` with its reason, matched by path, kind and whole description (no
  line number: a description names its function and its site, the field, or the call and its
  receiver); an entry that matches nothing in its crate is reported as a stale exclusion
  among the survivors, since the code under the ruling changed. The tool's other kind
  (`guard`) is not run (`just extra-mutants <crate> --kind guard` runs it by hand): too few
  of its survivors were real gaps (worklog 5.0b). A survivor of the remote half whose only
  catching test is `#[cfg(windows)]` is rerun on this machine when the run is collected, and
  the outcome here replaces the remote one; such a test's mutants are named in
  `WINDOWS_ONLY_TESTS` in `scripts/mutants.py`, by file and function. So a guard that
  refuses an input the standard library would accept (a leading `+`, which `u8::from_str`
  takes) still gets a test of its own when written: no run will ask for it (the color grammar
  had none, with all its mutants caught; S10's review, 2026-10-09).
- **Clef scan (retired 2026-10-17).** A decision model asked whether each 60-line window of
  production code holds a bug (`scripts/clef_scan.py`, `just clef <crate>` and `just
  clef-diff [base]`, Cloudflare's Clef, flags at P >= 0.7, rulings in
  `scripts/clef_rulings.md`) ran at every phase close from 2026-10-06 and is no longer part of
  any review. Over 88 ruled flags two were real (a version-gated flag, 4.c-fix1; a memory
  charge released early, 4.y), both scored below every flag at 0.80 or above, and Phase 5's
  close pass raised 22, all false; the cost was the lead's reading of every flag. Its
  successor candidate was measured before anything replaced it (2026-10-17, outside the
  repository, in the maintainer's `SystemOne/jev-eval/round3` folder, its results' Round 4):
  SubQ Snap, a decision model with a 384K-token window, catches one-line slips of the
  mutation-run kind far better (AUC 0.92 against 0.84 over 732 pairs), but on 111 real
  defects from the project's fix commits
  it scores the code before each fix no higher than the code after (AUC 0.50 for the window,
  the whole file, and the file with the spec sections it cites), and its highest windows on
  real code were less often ones a later fix changed than windows overall. The slips it does
  catch are the kind a test catches, which the mutation runs already measure. A scan comes
  back only with a model that separates real pre-fix code from fixed code on that history set
  (`build_history.py` there rebuilds it from any list of fix commits).

## Dependencies

Only crates listed in `spec/core/README.md` "External Dependencies" are pre-approved. Anything else,
including "just a small helper crate", is a question for the user, with the reason and the
alternative considered. Prefer versions published at least a week ago; no floating ranges.

## Commits

The first line is `type(scope): summary` (Conventional Commits), scope being the crate or area
(`fix(vtree): ...`, `feat(studio_core): ...`, `docs(plans): ...`, `chore(workspace): ...`; a
multi-crate step uses its phase, `feat(phase2): ...`). Types: `feat`, `fix`, `docs`, `test`,
`refactor`, `perf` (a change made for speed or memory that changes no behavior), `chore`
(workspace, dependencies, CI). The first line is at most 120 characters: it is what `git log
--oneline` and every tool's history view show, and a subject that lists every change of the
commit is a body written on the wrong line (subjects had reached 368 characters). A worklog step tag (`4.y-sub b3b4c`) goes in the body, not the first
line: the first line is for what changed. A summary naming something done puts every verb in
the imperative (`docs(worklog): audit the open 0.0.1 boxes; close s, x and y4 as done`), never
in the past participle (`the open 0.0.1 boxes audited; s, x and y4 closed as done`), which reads
like verse; a summary stating what the code does from now on keeps the present tense (`the
drafts name the suite Unlimited Tool Works where the preview does`). The body says why, not what; the diff says
what. This is for maintainers reading history (`git log --grep '^fix('` when bisecting), not for
members: `CHANGELOG.md` stays hand-written per the core plan's "Changelog and version display" and
is never generated from commit messages. From the first release on, a commit that changes
something a member notices against the last release (a fix for a bug a release shipped, a new or
changed behavior, a speed-up worth telling) adds its line under `[Unreleased]` in `CHANGELOG.md`
itself; a fix for something no release has shipped adds none. There is one `fix` type for both:
`git log --grep '^fix(' -- CHANGELOG.md` lists the fixes a release note owes its readers. Write
the message to a file and `git commit -F` it;
PowerShell has no heredocs.
Do not include agent co-authoring lines.

## Toolchain notes

- The toolchain is pinned by the root `rust-toolchain.toml` (exact stable version plus the
  `wasm32-unknown-unknown` target); rustup picks it up on checkout. Bumping the version is its own
  commit: run the gates on the new toolchain, fix what the new clippy lints flag, then commit. Never
  let a `rustup update` be the reason the gate is red.
- **Builds leave two logical processors free.** `.cargo/config.toml` sets `build.jobs = -2`
  (relative to the CPU count, so CI is capped the same way); rustc's codegen threads share that
  jobserver budget. Pass `--jobs N` to override for one run.
- **`cfg`-gated code is compiled on every CI OS.** A module built `#[cfg(any(windows, test))]`
  exists on Linux only for its tests, so every item in it needs a portable test or the Linux
  gate fails with `dead_code` while the Windows gate, where the real caller exists, stays green.
- **PowerShell false failures with cargo.** cargo writes progress to stderr; redirecting (`2>&1`)
  or piping turns that into a `NativeCommandError` and a nonzero exit code on success. Judge by the
  output (`Finished`, `test result: ok`), or run the command bare and echo `$LASTEXITCODE` after.
- **Line endings are LF everywhere** (`.editorconfig`, `.gitattributes`, `.vscode/settings.json`
  all say so). The trap is Python on Windows: text-mode `open()` writes `\r\n` on save and append,
  silently converting a whole file and making every line show as changed. Scripts that touch repo
  files open them with `newline="\n"`, or read and write bytes. rustfmt emits LF by default.
  Binary fixtures are exempt: `.gitattributes` marks `crates/**/tests/fixtures/**` as `-text`, so
  a game file that happens to contain CRLF is stored as is (the first `cpk` fixture commit
  normalized one and shrank it by 97 bytes before this rule existed).
