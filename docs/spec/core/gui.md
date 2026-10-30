# Unlimited Tool Works — Core plan: GUI design

Part of the [Core plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## GUI Design

### Shell layout

In egui, the shell is built with `Panel` (sides) + `CentralPanel`, shown on the whole-app `Ui`
that `eframe::App::ui` provides (API as of egui 0.37; see "egui version discipline" below).
A fixed-width side panel (the sidebar, the help window's chapter tree, the Ruleset editor's
rail) takes `.resizable(false)` beside its `exact_size`: egui's `Panel::left` and
`Panel::right` start resizable and a pinned size does not turn that off, so the edge showed
the resize cursor while a drag did nothing.

**The window's size.** Studio opens at 1280 × 780 points, centered, not maximized: the size of
its client area, the title bar and borders outside it, as every window size here is.
Points are egui's logical units, so the system's display scaling applies: at 150% the window
covers 1920 × 1170 pixels and looks the same size it does at 100%. A fixed size, not a share of
the monitor, because a share makes the window as wide as an ultrawide monitor is. On a monitor
whose logical height cannot hold it with room for the taskbar and the title bar (780 points is
over 90% of its logical height: a 1366 × 768 laptop), only the height is fitted, to 90% of the
monitor's logical height rounded to whole points (1280 × 691 at 1366 × 768); the width stays
1280 wherever the window, its borders included, fits the monitor's width. A monitor narrower
than that (1280 × 720 logical: a 1080p one at 150%, rare today) gets the width that fits, and
the views draw scaled down (below). The monitor's size is known only once the window exists
(`ViewportInfo::monitor_size`, in points), so the window is created at 1280 × 780 and the shell
fits and centers it on the first frame that reports the monitor's size
(`ViewportCommand::InnerSize`, then `OuterPosition` centering the outer rectangle, title bar
included), and never again, so a member's own resize or move holds. A window reopening at a
remembered place (`plans/core/gui.md` "Window size classes") is not fitted. eframe's own clamp
at creation is not enough: it clamps to the whole monitor, under the taskbar. Where no frame
reports the monitor's size, the window stays as created.

**Scaling with the width.** A view is laid out for its window's design width: 1280 points for
the main tools. Narrower than that, the shell scales the whole UI down with the width (egui's
zoom factor set to the window's width over the design width, the width read in points at zoom
1, the frame's screen rectangle times the current zoom, so the zoom does not feed back into it
and display scaling still applies: 1440 pixels at 150% are 960 points, zoom 0.75), so every
row keeps the arrangement it has at 1280 and nothing clips or rewraps. egui applies a new zoom
only when the next pass begins, so the pass that finds the zoom changed is discarded and run
again at the new zoom (`Context::request_discard`): otherwise the frame after a jump (a snap
from 1920 points to 960) lays out once at 960 points, and the views draw a layout no other
frame shows. A frame may run three passes, not egui's default two (`theme::install`, which
the window and every view test call): a new `Grid` discards its first pass to size itself,
so a view that lays out from widths a pass measured (the Save editor's centred player
columns) needs a third to discard the second. No view therefore keeps a layout for a pane narrower than its design: we keep no
fallback no member reaches, because a fallback kept "just in case" was the state the tests ran
in instead of the design's (0.0.1m2). At or above the design width, and at any height, the
zoom is 1 and the views reflow: wider rows spread, a shorter window scrolls or stacks as each
view's height rules say. A frame that reports no window rectangle leaves the zoom as it was.
We scale on the width alone because the views' fixed-width rows (the Tactics pitch, a row of
colour swatches, the tab strip) are what clips below about 900 points (measured on the Save
editor, 2026-10-25), while height only ever needs a scroll. The zoom factor is the shell's
alone: egui's keyboard zoom (Ctrl with plus, minus or 0) is turned off, because the width rule
sets the zoom every frame and would undo a keyboard zoom one frame later; a member who wants
larger text has the system's display scaling, which points follow. A member's resize stops at
**960 × 600 points**: below 960 the scaled text drops under 75% of its size, past legible on a
100% display.

### egui version discipline

egui still ships renames and deprecations most releases (0.31 moved window options into
`ViewportBuilder`; 0.32 rewrote popups/menus and renamed `Rounding` to `CornerRadius`; 0.34
deprecated `SidePanel`/`TopBottomPanel` for `Panel`, replaced `App::update(&Context)` with
`App::ui(&mut Ui)`, and renamed `Context::style` to `global_style`; 0.37 deprecated `Id::new`
for `Id::unique` and `IdSalt`, renamed `Ui::id` to `Ui::scope_id`, and replaced `WidgetType`
with `egui::Role`). LLM-assisted coding reproduces
whichever API the model saw most, so it will emit pre-0.34 code by default. Rules:

- `eframe` and `egui` are pinned exact (`=0.37.0`) once in `[workspace.dependencies]` (the
  lock-step rule under `architecture.md` "Workspace guardrails") and upgraded deliberately, one
  release at a time, following each release's migration notes in `CHANGELOG.md` / the GitHub
  release. Exact rather than caret, because a caret already excludes the next minor (the
  releases that rename) and what remains to exclude is a patch release arriving through a bare
  `cargo update`, unreviewed. `egui-wgpu` is not declared: it is `eframe`'s own dependency, and
  the crates built on egui (`egui-wgpu`, `egui-winit`, `egui_extras`) each require the `egui` of
  their own release, so the exact `egui` pin holds them to it without a pin of their own. What
  it does not hold is egui's own dependencies (`epaint`, `emath`, `ecolor`), which egui requires
  by caret; they carry no API this workspace calls directly.
- Treat deprecation warnings as errors in CI (`#![deny(deprecated)]` or `-D deprecated`) so stale
  API from generated code is caught at compile time rather than accumulating.
- The code sketches in these plans were written against 0.36 and earlier; they illustrate
  structure and must be re-checked against the pinned version when implemented, not pasted.

### Settings menu

- Settings persist to a single file: `[common]` holds `CommonSettings`, and each tool's values
  live in a table under its `id()` key (`[team-compiler]`; `common` is therefore not a valid tool
  id), with `default_settings()` merged in for missing values, recursively into nested tables
  (this replaces Red's settings-transfer mechanism for new versions). Missing common keys load as
  their defaults the same way; unknown keys in a tool's table are kept as read. The file's location — `data/` next to the exe or the user config dir — is the
  user's first-run choice (see `distribution.md` "Data location" under Distribution and updates).

### Help window

- **Chapters.** Every registered tool contributes one chapter through `StudioTool::help()`, a
  `HelpSection { title, topics: Vec<HelpTopic> }` whose topics are Markdown bodies embedded with
  `include_str!` from the crate's `help/` folder (numbered files fix the order: `01_overview.md`,
  `02_exports.md`, …). Chapters appear in sidebar order; an unregistered tool has no chapter, so
  the help can never describe a tool the build lacks.
- **Authoring rule.** Topics are written for members, not maintainers: what to put where, what a
  message means and what to do about it, never how the code works (that is `docs/`). A behavior
  change that a member can observe is not done until its topic says so — the help lives in the
  tool's crate precisely so the same diff carries both.

### Status bar

Three slots, each admitting one typed item kind from `studio_core::status`:

| Slot | Item | Admits |
|---|---|---|
| Left | `ShellCondition` | Persistent environment states the **shell** computes itself: `data_dir_read_only`, `update_available`. Not a PES folder that is missing or holds no exe or another version's: the version selector's coloring already owns that, in every tool's sidebar, so no condition repeats it |
| Middle | `ToolActivity` | Tools working in the background while a **different** tool's view is shown — `activity()` polled each frame for every registered tool except the active one, e.g. `Team compiler · 12/48 · 01:12` while the save editor is open. The active tool's own run strip is unchanged, so nothing shows twice |
| Right | `Notice` | The most recent one-off event, published by a tool through `ctx.notify(Notice)` or by the shell: `Switched to PES 2021 · [Fix PES path]`, `Compiled 48 teams · [Open output]`, `Teams list is read-only`. Text plus at most one action |

Rules that keep it worth the row:

- **Typed, never free text.** Tools cannot draw into the bar; they return a `ToolActivity` and
  publish `Notice`s. Conditions are the shell's alone. A `set_status_text(String)` API is exactly
  how a status bar becomes clutter, and it does not exist.

### Event wiring

The tool crates emit `PipelineEventEnvelope` values via channels. eframe's `logic` pass ticks
every tool:

```rust
impl eframe::App for StudioApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Background pass: every tool ticks (event draining, timer deadlines),
        // then only the active tool's view renders
        for tool in &mut self.tools {
            tool.tick(&self.ctx);
        }
    }
}
```

### Cancellation

Rust's rayon scopes or spawned tasks support cancellation via a flag; the pipeline's lives in the
run's `MemoryBudget` (`cancel` wakes every waiter, `is_cancelled` checked before each task —
`spec/libs/pipeline.md`).

### Browser deployment: Studio Web

**Keeping the door open from Phase 1.** Studio Web is not a first-release gate (see `plans/core/development_plan.md` "Phase 18"),
but the codebase must not drift into unportable dependencies while it waits. Workspace guardrail 6
(`cargo check --target wasm32-unknown-unknown` for every lib crate in CI) enforces this from
the first commit; it costs minutes per PR and turns the later web build from an archaeology project
into UI wiring plus the pipeline adapters. The desktop binary remains the primary distribution.

### Architecture separation

```
lib crates (pes_savefile, cpk, fpk, fmdl, pes_model, ftex, dds_convert, fox2, vtree, archives, teams_list, aesthetics_export, pipeline, model_convert, aatf, music_export, audio_engine, match_feed, kit_config, ...)
    ↑ used by
tool crates (team_compiler, save_editor, stadium_compiler, export_upgrader, music_player, music_export_editor, match_tracker, kit_config_editor, refs_arranger, balls_compiler, player_aesthetics_editor, team_creator, ruleset_editor, db_generator) — each implements StudioTool
    ↑ registered by            ↖ trait + shell + common widgets defined in studio_core
studio (single binary: GUI when run without args, CLI subcommand dispatch with args;
        native binary + WASM web build)
```

The `pause()` pattern from Blue/Red disappears entirely. The pipeline returns warnings/errors as
data. The GUI or CLI layer decides how to present them (log panel, status-bar notices, dialogs for
decisions, console output).

### Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". `ST` is the shell's code; the scenarios not yet
moved here, with the scope and harness they share, are in `plans/core/gui.md` "Acceptance".

**Window**

```
ST-WIN-01   GIVEN a monitor of 1920 × 1080 points and the window created at 1280 × 780 inside an
                 outer rectangle 16 points wider and 39 taller (its borders and title bar)
            WHEN the GUI's first frame runs
            THEN it asks for a 1280 × 780 inner size and moves the window to (312, 130.5), its
                 outer rectangle centered on the monitor; GIVEN 1366 × 768 THEN 1280 × 691 at
                 (35, 19); GIVEN a monitor of 1280 × 720 points THEN 1264 × 648 at (0, 16.5);
                 and no later frame sends either; GIVEN the first frame reporting no monitor
                 size THEN neither command, and the first frame that reports one sends both;
                 the native window's options carry a 960 × 600 minimum inner size
ST-WIN-02   GIVEN a main tool's view and a window 960 points wide (native pixels at 100%)
            WHEN a frame runs
            THEN the zoom factor is 0.75 and the view lays out 1280 points wide; GIVEN 1280 or
                 1600 wide THEN the zoom is 1 and the view lays out at the window's width;
                 GIVEN 1440 native pixels at 150% display scaling THEN the zoom is 0.75;
                 GIVEN the height changed alone THEN the zoom is as before; GIVEN a frame at
                 1920 points wide, then one at 960 THEN no pass of the second frame that
                 reaches the screen lays the view out narrower than 1280 points
```

---
