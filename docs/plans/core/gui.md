# 4cc Studio — Core plan: GUI design

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## GUI Design

### Shell layout

What exists of this is in `spec/core/gui.md` "Shell layout".

A shell built with egui panels: a left sidebar, a tool view, and a one-row status bar under both.
The tool view belongs to each tool; pipeline tools subdivide it into grid, toolbar, and log panel
(other tools, e.g. the save editor, use it differently). The status bar belongs to the shell.

```
┌──────────┬──────────────────────────────────────────────┐
│          │                                              │
│  [logo]  │   Tool view (fills remaining space)          │
│          │                                              │
│  PES 19▾ │   ┌────────────────────────────────────────┐ │
│          │   │ Team grid (scrollable)                 │ │
│  ──────  │   │  /aa/ [714]  [01][02][03][□][□]    [GK][K1]  │ │
│ ▌ TEAM   │   │  /bb/ [???]  [01][02][03][04][05]  [GK][K1]│ │
│   SAVE   │   │  /cc/ [809]  [01][□][□][□][□]      [GK][K1]    │ │
│   UPGR   │   └────────────────────────────────────────┘ │
│  (STAD)  │   [run strip]              [Compile ▾]    │
│  (⋯)     │   ┌────────────────────────────────────────┐ │
│          │   │ Log panel (scrolling, collapsible)     │ │
│  ──────  │   │ - Warning - Nested folders ...         │ │
│ [?][gear]│   │ - /cc/ processed                       │ │
│          │   └────────────────────────────────────────┘ │
├──────────┴──────────────────────────────────────────────┤
│ ⚠ data folder read-only          Compiled 48 · [Open output] │
└─────────────────────────────────────────────────────────┘
```

The sidebar lists the registered tools in registry order (today the Team compiler, the Save
editor, the Export upgrader), then the planned ones in parentheses here, grayed in the window
("Sidebar").

The window's opening size and fit, and its scaling under the design width, are built:
`spec/core/gui.md` "Shell layout", the paragraphs "The window's size" and "Scaling with the
width".

**Window size classes.** The main tools share one window size: switching between them never
resizes the window. A special tool, one whose use calls for another size, declares its own
(`StudioTool::window_size`, `None` for the main tools): today only the Music player, compact so
a streamer places it beside the PES window, its design size set with its view's draft. The
shell remembers each class's last size and position, across runs as most desktop programs do:
switching from a tool of one class to a tool of the other restores the other's (its declared
size, centered, the first time), so a streamer's compact window comes back where they left
it, and the window opens in the class of the tool it opens on (the last active one,
ST-RUN-02) at that class's remembered place. A remembered size larger than every monitor
reopens at the largest monitor's size: eframe's creation clamp
(`clamp_size_to_monitor_size`, on by default) is kept, so a window stretched over two
monitors reopens no wider than the largest monitor once one is unplugged (it may still stand
partly past that monitor's edge; the title-bar check below keeps it within reach). A class never used
opens at its design size
(1280 × 780 for the main class, a special tool's declared size), fitted and centered as a
first start is (`spec/core/gui.md` "The window's size"). A remembered place whose title bar lies on no monitor now (a second monitor
unplugged) is dropped for the fit, so the window is never opened out of reach. `monitor_size`
names only the window's own monitor, without its origin, and eframe 0.37 reaches winit's list
of monitors only once the window exists (`DECISIONS.md`, "eframe 0.37 gives the shell the
monitor list only once the window exists"), through the frame's window
(`Frame::winit_window`, then `available_monitors`), so the check runs on the first frame: the
window is created at the
remembered place unchecked, and the first frame tests it against every monitor's rectangle, in
the screen's pixels. A window created off every monitor is out of sight, so its move to the fit
is not seen as a jump; the exception is a desk whose scales changed with the unplugging (a
place on a 200% monitor, a 100% primary left), where winit's conversion of the creation
position with the primary's scale lands the window on a monitor for that one frame, the same
one-frame correction a mixed-scale desk sees below. The check is one pure function the first frame calls with the window's
list and the tests call with a list of their own (a function pointer on the shell, the one seam,
since a test's frame has no window): the title bar, the window's width over the top 32 pixels
at 100%, scaled by each monitor's own scale (the window's size in pixels once it sits there),
overlaps that monitor's rectangle; an empty list, a frame with no window to ask, trusts the
place. Both places are shell state in the
settings file, `window_main` and `window_special`, each a list of six numbers
`[x, y, width, height, scale, maximized]`: the outer position in the screen's physical pixels, because a
point is a different number of pixels on each monitor and Windows and X11 place windows in
pixels; the inner size in points at zoom 1 (its pixels over the monitor's scale), the size a
window keeps when the OS moves it to a monitor of another scale, never in the shell's zoomed
points, which would reopen a window closed under the design width wider by the zoom; and the
monitor's scale factor when the place was recorded; and whether the window was maximized
(below). The shell reads the place every frame (the
last normal one: not minimized, maximized or fullscreen, the rects known, and not the frame
where a queued zoom lands, which reports its rects at the zoom before it: its inner rect and
`screen_rect` differ there by a pixel or more, against a rounding error on every other frame),
writes it when the class is left and when the window closes (`App::on_exit`), and opens the
window at it: created there (`with_position` with the pixels over the record's scale,
`with_inner_size`, not centered), and the first frame, the place on a monitor, sends the outer
position again in its own points, the pixels over the frame's pixels per point, which is a
no-op where the creation landed right and the correction on a mixed-scale desk, where winit
converts the creation position with the primary monitor's scale. Not a move from a centered
creation: eframe shows the window after the first frame's paint and applies that frame's
viewport commands after the show, so the member would see the jump. Not eframe's own window
persistence, whose `persistence` feature writes a file of its own outside the data location
(a portable install keeps everything beside the exe), knows one window size, not two, and
stores the size in zoomed points. A window closed maximized reopens maximized, as most desktop
programs do, and its restore button gives back the normal place it had before: the record
gains a sixth number, 1 when the window was maximized on its last frame that was not minimized
(a member who minimizes a maximized window and closes it from the taskbar closed it
maximized), else 0, while its first five stay the last normal frame's place; the window is
created at that place and maximized (`ViewportBuilder::with_maximized`), so the OS keeps the
place as the size to restore to; and the first frame, after it sends the position again
(above), sends the maximize again, on every platform: winit moves a maximized window by
restoring it first (on Windows `set_outer_position` clears the maximized flag, a
`ShowWindow(SW_RESTORE)`), and an X11 window manager ignores the maximize asked of the window
eframe keeps hidden until its first paint (egui-winit's own persistence maximizes after the
first show for that reason). For a maximized place the position is sent again only when the
creation can have landed wrong: when the record's scale differs from the primary monitor's
(the scale winit converts the creation position with) or from the frame's pixels per point
(the window sits on a monitor of another scale than the record's). Elsewhere the resend moves
nothing and only restores the shown window and maximizes it again, a faint but visible
flicker; the maximize is still sent, a no-op on Windows for a
window already maximized and the needed one on X11. Not the resend skipped for every maximized
place: on a mixed-scale desk the restore place, and the monitor the window maximizes on, would
stay wrong; there the window may be seen restoring and maximizing again once. A record of five numbers, written before the sixth existed,
reads as not maximized. A place the first frame finds on no monitor ends fitted and not
maximized: the window was created maximized before that check could run, so the first frame
sends `Maximized(false)` ahead of the fit, a visible jump we accept for a case as rare as an
unplugged monitor (kept maximized, its restore button would put the window back off every
monitor). On Wayland, where winit reports
no window position and ignores a set one, nothing is recorded and the window always opens
fitted. A special tool's design width is its declared width, and its minimum its
own.

The shell draws only what is its own. Panels claim space in call order, so the status bar is
added first to span the full width, the sidebar next, and the central panel, whatever is left,
is handed to the active tool's `view()`:

```rust
impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Status bar: full width, under the sidebar too (added first)
        egui::Panel::bottom("status_bar").exact_size(24.0)
            .show(ui, |ui| self.render_status_bar(ui));

        // Left sidebar: the icon rail; the full sidebar opens over the view on hover
        egui::Panel::left("sidebar").exact_size(RAIL_WIDTH)
            .show(ui, |ui| self.render_sidebar(ui));

        // Central panel: the active tool's view, which lays itself out inside it
        egui::CentralPanel::default().show(ui, |ui| {
            self.tools[self.active].view(ui, &self.ctx);
        });
    }
}
```

A pipeline tool subdivides the `Ui` it is given with panels of its own, in the same claim
order: the log panel at the bottom, the toolbar above it, the grid in what remains. Since 0.34 a
panel's `show` takes any `Ui`, the whole-app one or a tool's (the older `show_inside` is
deprecated, and deprecations are errors here, `spec/core/gui.md` "egui version discipline"):

```rust
// team_compiler::view, inside `view()`; the log's `BottomDock` claims its own
// `Panel::bottom` (resizable, 220 by default), so the dock's frame is one widget
self.log_panel.show(ui, &self.run.lines);
egui::Panel::bottom("toolbar").exact_size(32.0)
    .show(ui, |ui| toolbar::show(ui, &self.run));
egui::CentralPanel::default().show(ui, |ui| grid::show(ui, &self.rows));
```

The shell never draws a tool's log or toolbar: an envelope names no tool (`spec/core/architecture.md`
"Event system"), so a shell-owned log would have to guess whose run a line belongs to, and the
Save editor has no run to show. The widgets are `studio_core`'s (the progress grid, the log panel
and the run strip under `widgets/`, `architecture.md` "Crate structure"); where they are placed
is each tool's.

### Sidebar

Top to bottom:

1. **4cc logo** — doubles as the About click target. About is a small popup holding the
   *detailed* version block — `<app> v1.4.0` (the display name, "Window title"), build kind (release, or `debug` from
   `cfg!(debug_assertions)`), target triple, data location, selected PES version — with a
   **Copy** button that puts exactly that block on the clipboard for a bug report, and links to
   the release page, the repository and the help. No version is shown beside the logo itself: the
   status bar already shows it 300 px away, and the icon rail has no room for it, so it would be
   the one element that came and went with the hover.
2. **PES version selector** — a *global* setting, not per-tool: the Team compiler targets it, the
   save editor uses it as a selection/default hint (an open save keeps its detected schema), and
   integrated conversion uses it as the destination format. Compact dropdown. Changing it is blocked with a tooltip while any tool reports a
   `version_change_blocker` (e.g. a running compilation, or the match tracker holding live memory
   addresses).
   - **Live exe check**: the selector is colored against the exe actually present in the resolved
     `pes_folder_path` — **red** when the folder exists but holds a `PES20XX.exe` of a *different*
     version (wrong selection or wrong path), **orange** when the folder is missing or holds no PES
     exe at all (likely invalid path); the tooltip names what was found. Re-checked on version/path
     changes and on window focus — a cheap single directory listing. This is the GUI rendering of
     the same condition the team compiler's `pes_version_mismatch` warning reports at compile time
     (the warning remains for the CLI).
   - **Auto-switch to the running PES** (desktop, wherever the running-PES probe reads processes:
     Windows, and Linux under Wine or Proton): a lightweight background poll watches for
     running `PES20XX.exe` processes. **Once per program life**, when a running PES's version
     differs from the selection, the selector switches to it automatically — primarily so the match
     tracker attaches to the right game without ceremony — announced via a status-bar notice.
     Once-per-life means a user who then switches away is never fought; a manual version change also
     consumes the auto-switch. If a `version_change_blocker` is active at detection time the switch
     is deferred until it clears (not consumed). If several PES versions are running, the first
     detected wins. The switch writes the ordinary persisted setting — the selector always reflects
     the setting, no shadow session state. The poll also knows the running exe's *path*: when it
     differs from the resolved `pes_folder_path` (e.g. the selector sits red/orange while the real
     install runs from elsewhere), the same notice offers a **"Fix PES path"** action that rewrites
     the PES folder from the running process — strictly opt-in via the notice's button, never
     applied automatically: the main row, `pes_folder_path`, with the `**` placeholder put back
     where the folder's own name holds the running version's year (Browse's rule, so the fix
     keeps working across versions), or, once any version has a folder of its own, the running
     version's own row, the folder stored as it is (an override row is taken as written;
     "Settings menu"). The action is offered only
     when the exe's folder is a folder on this system: under Wine or Proton the probe reports the
     game's Windows path, which a Linux setting cannot use.
3. **Tool list** — vertical nav, one entry per tool: its icon at **24 points**, its label in the
   body text beside it once the sidebar opens (item 4), populated from the registered
   `StudioTool` list. The rail shows the icons alone, so they are drawn at about twice the body
   text (12.5) for the rail by itself to tell the tools apart, and at the same size and place in
   the opened sidebar, so nothing moves when it opens. 24 rather than larger because the rail
   is 56 points wide and every entry is in it: at 24 the fourteen entries take about 470 points
   of the 776-point rail at the default window, leaving the logo, the version dropdown, the help
   and the gear their room; at 32 they would take about 600 with 12-point side margins, and a
   rail that overflows scrolls its entries out of sight. The active tool is highlighted with an
   accent-colored left border bar plus a filled background. The suite's planned tools not in
   this build (the registry's, `architecture.md` "Tool plugin interface") are listed after the
   registered ones, grayed and not clickable, each tooltip saying the tool is not in this build:
   the list communicates the suite roadmap. A registered tool the registry marks **CLI-only**
   (in the 0.0.1 preview the Team compiler and the Export upgrader, whose views come later;
   `development_plan.md` "Divegrass Before Christmas") is drawn as its planned entry, grayed
   and not clickable, its tooltip saying it has no window in this version and runs from the
   command line, and it is never the active tool (a `last_tool` naming it opens the first
   window tool). It stays registered for everything else: its CLI commands, its help chapter
   and its settings section, which the command line reads. The mark is the registry's, in
   `studio`, not a `StudioTool` method: which tools a build shows is the build's choice, and
   the preview's list goes when the views land.
4. **A rail that opens on hover** — the sidebar is always an icon-only rail (~56 px) beside the
   tool's view, which takes the rest of the window. The pointer resting on the rail (a short
   delay, ~150 ms, so a pointer crossing it on its way elsewhere opens nothing) opens the full
   sidebar (the labels, the version dropdown; as wide as the longest label needs on one line,
   "Player aesthetics editor", since its width costs the view nothing) **over** the tool's left
   edge, as a
   floating layer with a shadow, never pushing or resizing the view under it; it closes when
   the pointer leaves it, except while a popup it opened (the version dropdown, the About
   block) is open. A click on a rail entry switches tools without waiting for the hover, so the
   rail alone is enough to work with, and keyboard users get the same overlay on focus. There
   is no collapse button and no expanded state to persist. We overlay rather than reserve the
   expanded width, because the expanded sidebar is needed only to read a label or pick a
   version, and reserving ~104 more points cost every tool that width all the time (at the
   default window a tab's area grows from 904 to about 1008 points).
5. **Help `?` and settings gear** pinned at the bottom, at the tool icons' 24 points — open the
   help window and the settings menu. F1 opens the help too.

### Settings menu

What exists of this is in `spec/core/gui.md` "Settings menu".

The settings menu is assembled from injected sections:

- **Common settings** (defined by `studio_core`): PES version, PES folder path, exports folder path,
  thread count, memory cap, theme, the update check and its interval, and the FPC hide boots and
  gloves IDs (`plans/save_editor.md` "Full Player Customization (FPC)": number rows bounded to
  the stock band, boots 1 to 100 and gloves 11 to 100). The update check's two rows are drawn
  from the release whose updater reads them (`core/distribution.md`, step 1, Release 0.2.0),
  not before: until then a row a member switches off changes nothing, and reads as a check
  the build never runs. The two keys stay in the file meanwhile. The state the shell writes
  itself (`last_tool`, `last_update_check`, `skipped_version`) has no row. The PES version row
  is the sidebar selector's twin: the same blocker, and a pick spends the running-PES
  auto-switch as the selector's does.
- **One collapsible section per registered tool that has settings**, provided by that tool's
  `settings_view(ui, ctx)` (each tool defines its own settings struct and UI, and writes it
  through `ctx.set_tool_settings`). A tool has settings when its `default_settings()` table is
  not empty: the table lists every key the tool reads, so an empty one is a tool with nothing
  to show (the Export upgrader), and a header over nothing would be a dead end. A tool the
  registry marks CLI-only has no section either, whatever its table holds: its window is not
  built, so its section would draw a placeholder or nothing (its table holding only state the
  tool writes itself), and its keys are edited in the settings file, as the readme says.
- Opening the settings menu shows the **currently selected tool's section expanded** and all other
  sections collapsed.
- **No Save or Apply**: a change applies as it is made and saves at the end of the frame. A text
  row (a path, a name) commits when it loses focus or Enter is pressed, not per keystroke: a
  half-typed folder path would otherwise reach every tool reading it. A path row (the common
  folders, a tool's file) is that text edit and a Browse button opening `rfd`'s native picker
  of its kind (a folder, or a file filtered to the type the tool reads), a pick committing at
  once; `studio_core` owns the row, so every section's path rows commit the same way. A path's
  edit is wide enough to show a usual install path whole: Steam's `C:\Program Files
  (x86)\Steam\steamapps\common\Pro Evolution Soccer 2019`, 71 characters, at the body font,
  so a member reads which folder is set without scrolling inside the box. In egui the edit is 472 points
  with its 8-point margins (a test measures the path's galley inside the drawn edit at the
  body font; the draft's box is 440, its 660-point body leaving no room for 472 beside the
  label and Browse), and the menu is 660 points: the common grid's widest row (its label column, the edit and Browse) needs
  656. A tool section's row sits indented under its header with that tool's own label
  column, so a section whose labels are long widens the menu while it is open, rather than
  narrowing its path box below the path.
- **The PES folder** is one main row for every version, `pes_folder_path`, whose `**` stands
  for the selected version's two digits (the default `…\Pro Evolution Soccer 20**`). Its
  Browse stores a picked folder whose own name holds the selected version's year with that
  year written `20**` (`D:\Games\Pro Evolution Soccer 2019` becomes `D:\Games\Pro Evolution
  Soccer 20**`), so the selector keeps switching folders as it does for the default; any other
  folder is stored as picked, and the row's tooltip says which. It is the rule the
  running-PES notice's Fix PES path already applies (the year anywhere in the folder's own
  name), one function both call, so the two ways to set the folder store the same text. The
  rule is a pure function, tested alone: the native picker cannot be driven headlessly.
  Under it, an **A folder per version** toggle shows one path row per version (PES 2015 to
  2021), for a member whose installs do not share a pattern: a filled row is that version's
  folder, taken as written, overriding the main row, and an empty one falls back to it. The
  overrides are a `[common.pes_folders]` table keyed by the version's number as
  `pes_version` stores it (`21 = '…'`), an entry only for a filled row;
  `CommonSettings::pes_folder`, which every reader calls, resolves the selected version's
  override first. The toggle is the view's alone, not a setting: it is on while any override
  exists and cannot be switched off until the rows are emptied, since a hidden override would
  still apply and hide why a folder is used. Emptying the last row leaves the toggle on, now
  free to switch off, rather than switching it off with the rows: the rows would vanish under
  the edit that emptied them. Fix PES path ("Sidebar") writes the running
  version's own row whenever any override exists (a member who keeps folders per version,
  whose other versions a rewritten main row would move), the folder as it is, else the main
  row as before.
- Saving is **best-effort**: if the write fails (typically a portable install unpacked somewhere
  unwritable), the in-memory settings keep working for the session and the shell raises the
  `data_dir_read_only` condition in the status bar — one condition covering settings and the
  teams list, since both live there. No elevation, no retry prompt. A save edits the file in
  place (`toml_edit`), changing only the values that changed: until the settings menu exists
  members edit the file by hand, and a tool switch saving `last_tool` must not drop their
  comments, blank lines or key order. Where the edit would not read back as the settings (a
  section written inline turned into a table header) or the file is not TOML, the save writes
  it fresh: a file the next start could not open costs more than the comments.

### Help window

What exists of this is in `spec/core/gui.md` "Help window".

The help window is the user documentation — it replaces both of Red's readmes (the 114-line
README and the 913-line `readme_advanced.md`), and the bundled readme shrinks to a pointer at it.
Like the settings menu it is **assembled from the tools**, not written centrally:

- **Chapters.** `studio_core` contributes the core chapters (setting up, first run and data
  location, updates, and "What's new" — the repository's `CHANGELOG.md` embedded at build time,
  see `distribution.md` "Changelog and version display"), which come first in the chapter
  tree; the tools' chapters follow in sidebar order. Red's `readme_advanced.md` maps onto
  this almost one-to-one: its refs, format, texture-handling and special-files headings become
  Team compiler topics, its updater and first-run headings core topics.
- **The Messages topic is generated.** Each chapter ends with a "Messages" topic rendered from the
  tool's `messages.rs` catalog — ID, severity, condition, consequence — so it is always complete
  and never hand-maintained; `help()` carries prose only. This is Red's "Message Types" chapter
  done once, for every tool. The two text cells, *When* and *Effect*, sit in the catalog's row
  beside the code and its severity, written for members; they start as the spec's catalog
  table's cells, and that table stays the design record (dispositions, context fields, the
  why), so the two diverge only by a wording pass. Not a Markdown table in the tool's `help/`
  folder: the row in `messages.rs` is the one the tests already hold to the codes the tool
  emits, so a text beside it cannot miss a code or outlive one. A tool lists the shared
  structure pass's codes it reports (the Export upgrader's) with its own severity and text.
- **Search.** One text box above the chapter tree, case-insensitive substring over every topic's
  paragraphs and every catalog row; results list as *Chapter › Topic › matching line*, click to
  jump with the hit highlighted. No index and no fuzzy matching: the whole corpus is tens of
  kilobytes, and a linear scan per keystroke costs nothing. Its first job is the one Red's users
  did by hand — paste a message ID from the log and find out what it means.
- **Opening.** The sidebar's `?` button or F1 opens the window on the **active tool's chapter**
  (as the settings menu opens on the active tool's section). A **message ID in the log panel is a
  link**: clicking it opens the help at that catalog row (`HelpTarget { tool, topic, anchor }`,
  the same target type search results and in-doc links resolve to). In-doc links use the form
  `[text](tool-id#topic-slug)` so a topic can point at another tool's topic without knowing where
  it renders.
- **Window.** An in-app egui window — non-modal and resizable, chapter tree left, rendered topic
  right, the search box above both. It first opens at 90% of Studio's window (its whole frame,
  title bar included, in whole points), centered in it, so it fits whatever size that window
  has (`spec/core/gui.md` "Shell layout", "The window's size"), rather than at a fixed size a small window cuts off. Its position and size hold while the program runs (egui
  keeps a window's rectangle by its id, across a close and a reopen), not across restarts: a
  settings field written at every drag's end is a settings save per move for a reader opened
  now and then. Opening it closes a shell modal (the settings menu, the About popup), which
  would otherwise cover it. Not a second OS window: egui's multi-viewport support
  exists, but it adds a class of platform bugs that a text reader does not need (re-checked
  2026-10-21 at egui 0.37: about 130 open issues mention viewports, among them child windows
  lost on a macOS Space switch, deferred ones not redrawing on Windows, and focus across
  windows on Wayland), and no headless test reaches it, since a test draws the embedded
  fallback. Rendering is
  `egui_commonmark` (CommonMark subset with tables, code blocks and links); a hand-rolled
  renderer was considered and rejected as a permanent maintenance item for a solved problem.
- **CLI equivalence.** `4cc-studio help [tool-id] [topic]` prints the Markdown to stdout, and
  `4cc-studio help --export <dir>` writes every chapter out as `.md` files — the wiki and the release
  page get the same text with no second source, and the `--export` output is what the release
  process publishes. A chapter prints as `# <title>`, then each topic as `## <title>` and its
  body, the Messages topic last as one table, so a topic file carries no title of its own and
  its subheadings start at `###`; `help` alone lists the chapters (id, title, topic
  slugs), `help <id> messages` prints the Messages topic alone, and the export writes
  `<id>.md` per chapter. Core chapters have ids of their own (`setting-up`, `first-run`), in
  the tools' namespace, so `help` is no tool's id. An unknown id or slug is refused with exit
  code 2, naming the ones that exist.

### Log panel

- Fixed-height region below the toolbar, monospace, colored lines (info/warning/error mapped from
  `PipelineEvent`); its chevron collapses it to its header row and back, and dragging its
  divider resizes it, the height kept across frames. That frame (header row, chevron, divider)
  is the widget's own part, apart from the lines it holds: the Save editor's Live AATF list
  (`plans/save_editor.md` "Configurable AATF rules") docks in it too.
- **Capped display**: only the last few hundred lines are rendered (egui text rendering is not
  optimized for thousands of lines). Full logs are written to disk (`issues.log`, `suggestions.log`)
  as before.
- **Auto-scroll with interruption**: sticks to the bottom until the user scrolls up, which pauses
  auto-scroll and shows a "↓ N new" jump-back indicator. While paused, the panel draws the
  lines it held when the pause began, and the arrivals are counted, not drawn, until the jump
  back or a scroll to the bottom: past the cap, a window sliding with each arrival would move
  the text under the reader at an unchanged scroll offset.
- **No wrapping**: a line longer than the panel scrolls sideways, as the draft's `white-space:
  pre` does; a wrapped line breaks the one-line-per-finding reading.
- **Filter toggles**: All; Warnings (the warning, error and fatal lines); Errors (the error and
  fatal lines). A filter hides lines, never drops them: All shows every line again.
- **Cross-linking**: clicking a warning/error line highlights the corresponding grid cell (they
  share the export/folder identity from the event payload); clicking the line's **message ID**
  opens the help window at that message's catalog row (see "Help window").

### Status bar

What exists of this is in `spec/core/gui.md` "Status bar".

One row, 24px, full width under sidebar and tool view, present in every tool. It is the home for
everything that is *about the environment or about another tool* — the things that previously
had no home and were described as "toasts" in three places. It is not a second log: a message
about the current tool's own work (an export, a scope, a run) goes to that tool's Log panel or
run strip, never here.

Each item kind stays in the bar for its lifetime:

| Item | Lifetime |
|---|---|
| `ShellCondition` | While the condition holds; disappears by itself |
| `ToolActivity` | While the tool reports it |
| `Notice` | A few seconds; with an action, until clicked or dismissed. Older notices are not kept — an event worth keeping is also a finding in a log |

`update_available` carries an Install action; each condition shows a tooltip and at most one action.

More rules that keep it worth the row:

- **A notice is seen, not found.** The notice sits in the middle of the bar and the
  `ToolActivity` of another tool on its right (the spec's table had them the other way round):
  the eye goes to the centre of a row, and the activity is the rarer item, shown only while
  another tool's view is up. Each notice carries a kind, `NoticeKind::{Success, Info,
  Warning}`, set by its publisher, and is drawn in that kind's bright colour: success bright
  green (`theme::SUCCESS_FG`, a file written, a compile done), information the link blue
  (`Switched to PES 2021`, `Updated to v1.4.0`), a warning the theme's warning orange (`Teams
  list is read-only`, a switch whose `Fix PES path` action points at a wrong exe). One plain
  colour for every notice was how a member missed a "Wrote settings.toml" line at the bar's
  edge; a single green for every notice would paint a warning as a success. What needs a
  decision now is a dialog, never a notice (next rule).
- **The middle takes what the sides leave.** The left slot (the version and any held conditions) and
  the right one (another tool's activity) are as wide as what they hold, and the notice's middle
  slot gets the rest, its text centred in it and cut with an ellipsis only past that, the whole
  text then its tooltip. Not three equal columns: a third of a 1280-point bar cut the Save
  editor's version notice (about 100 characters) while the bar had room for all of it.
- **Never modal, never a question.** Anything needing a decision (updater consent, the teams-list
  merge review) is a dialog. The bar announces; it does not ask.
- **Empty state is static, not blank**: `v1.2.0` in small text — the one always-visible version
  number, the line a user pastes into a bug report (see `distribution.md` "Changelog and version display"),
  then a frameless **Reset window** button in the same small weak text, and nothing else.
  Reset window puts the window back to its class's design size (1280 × 780 for the main
  class, a special tool's declared size), fitted to its monitor and centered on it as a first
  start is, a maximized window restored first: one click back from a window dragged half off
  a monitor or stretched over two, where finding the edges to drag is the chore. It changes
  nothing else, and the remembered place follows as after any resize. The button is drawn
  only while the window is maximized or its size differs from the active tool's class's
  design size fitted to the monitor: at that size the click would change nothing a member
  set out to change, so the bar shows the version alone. Its position does not count, as a
  window moved at its default size is one the member placed. The data location is not shown: it is a choice made once and on purpose, its one
  failure mode already has the `data_dir_read_only` condition, and the About block carries it for
  the rare report that needs it. The row is always present at a fixed height, so a notice never
  shifts the tool view; auto-hiding was rejected for that reason.
- **After an update**, the first start of a new version publishes the shell notice `Updated to
  v1.4.0 · [What's new]`, whose action opens the help window at the "What's new" chapter. No
  popup: a notice can be ignored and dismissed; a what's-new dialog is read by nobody who did
  not ask for it.
- **CLI equivalence:** conditions print once at startup on stderr, notices print on stderr as
  they occur (stdout is the findings'); the same items, rendered as lines. The CLI writes no
  settings, so its `data_dir_read_only` comes from a probe at startup: the settings file
  opened for writing, nothing written, which fails where the file is read-only, as it is in a
  portable copy on read-only media or in a folder the user may not write. The GUI raises the
  condition from a save that fails and drops it at the next save that succeeds; it does not
  probe, since its first switch of tool saves anyway.

### Window title

The title bar is the one surface a user sees **while the window is not in front** — the taskbar
button, alt-tab, a second monitor — so it carries what matters then: whether the Studio is
still working, and what it just finished. Red did this with the console title (`1 - …`, `2 - …`,
`Done - …`); the Studio does the same with more detail, from the same typed items the status bar
uses, so it costs no new API:

`<app>` is the suite's display name, Unlimited Tool Works in every release before 1.0.0
(`development_plan.md` "The preview's codename"), the same here and in ST-TTL:

| State | Title |
|---|---|
| Idle | `Save editor - <app>` (the active tool's name) |
| Idle, the active tool names its open file | `Save editor [C:\…\EDIT00000000] - <app>` |
| A tool reports a `ToolActivity` | `Team compiler · 12/48 · 01:12 - <app>` |
| Idle, window unfocused, a `Notice` arrived since focus was lost | `Compiled 48 teams - <app>` |

Rules:

- **The idle title names the active tool, and no view draws its own name.** A heading repeating
  the tool's name took a row from every view and told the member what the highlighted sidebar
  icon already did; the title bar is where a window says what it shows. A view still titles
  what it shows inside it (the Save editor's selected player or team).
- **The open file's path rides the idle title, in square brackets after the tool's name.** A
  tool that edits one file says which through `StudioTool::title_detail(&self) ->
  Option<String>` (default `None`); the Save editor returns its save's path while one is open.
  Not a line in the view: the path beside the toolbar's `• unsaved changes` took a second row
  once both were long, which moved the whole view down. The busy and notice titles keep their
  own text: a path there would push the progress past the taskbar's cut.

- **A plain hyphen before the name.** The title is a UI string, so it carries no em dash
  (`CONTRIBUTING.md` "Code style"); ` - ` is also what most desktop apps put before their name.

- **Progress first, name last.** Taskbar buttons truncate from the right; the part worth
  reading is the number, not the app name.
- **Every tool's activity counts, the active one included.** The status bar's middle slot hides
  the active tool's activity because its run strip already shows it; the title has no such
  duplicate, so the shell polls `activity()` for all registered tools. Several at once: the
  active tool's first, then sidebar order, joined with ` · ` — in practice one, since almost
  every activity is a compile.
- **The "Done" state is the last `Notice`, held while unfocused.** The shell keeps the latest
  notice's text in the title from the moment it arrives until the window next gains focus, then
  returns to idle. That is Red's `Done -` with the result in it, and it falls out of the notice
  the tool already publishes (`Compiled 48 teams · [Open output]` — the action is not in the
  title). While focused the notice is in the status bar and the title stays idle.
- **Never the version, never a condition.** The version lives in the status bar (see "Changelog
  and version display"); a `ShellCondition` is an environment fact that outlasts any glance at
  the taskbar and would read as a stuck process.
- Set through `ViewportCommand::Title` only when the string changes, once per frame at most.
  The Windows taskbar progress bar (`ITaskbarList3`) is **not** used: neither winit nor eframe
  exposes it, and a text title covers the need without platform code.

### Event wiring

What exists of this is in `spec/core/gui.md` "Event wiring".

Each tool owns the receiving end of its own runs, as the spec's shell slice built it: a run gets
its sink through `ctx.with_events(sender)`, the tool drains the receiver in `tick`, and `view()`
renders the resulting state and reports the user's intent (a Compile click starts the next
run). The block extends the spec's `logic` pass, which ticks every tool:

```rust
// Pipeline workers (on the run's thread or the rayon pool) send stable-ID events through the
// stale-result envelope, to the sender the run was started with:
ctx.emit(PipelineEventEnvelope {
    run_id,
    export_id: Some(export_id),
    export_revision: Some(export_revision),
    event: PipelineEvent::FolderStatus { export_id, path: folder_path.clone(), status },
});
// Messages are sent as separate enveloped events; the tool associates them by scope.
for msg in messages {
    ctx.emit(PipelineEventEnvelope {
        run_id,
        export_id: msg.scope.export_id(),
        export_revision: revision_for(&msg.scope),
        event: PipelineEvent::Message(msg),
    });
}

// The tool drains its receivers once per frame, in `tick`, which the shell calls for every
// registered tool before `ui` (and also while the window is hidden, when a repaint was
// requested), so a compile behind the Save editor's view keeps its grid current:
impl StudioTool for team_compiler::Tool {
    fn tick(&mut self, _ctx: &ToolContext) {
        // One receiver per live source: the compile in flight, the live checks.
        while let Ok(envelope) = self.run.events.try_recv() {
            // Reject what an older run or an older export revision computed.
            if self.rows.is_current(&envelope) {
                self.rows.apply(envelope.event);
            }
        }
    }

    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        // Render: grid, toolbar and log read the tool's state (layout as in "Shell layout").
    }
}
```

egui repaints only on input, so a run's events wake the GUI themselves: the shell installs a
wake callback in the `ToolContext` it hands out (`Arc<dyn Fn(Duration) + Send + Sync>`
calling `egui::Context::request_repaint_after`: GUI-neutral like the match feed's `wake` in
`architecture.md` "Tool plugin interface", but taking a delay; the feed's own stays `Fn()`,
since a publish always wants the next frame), and `ToolContext::emit` calls it with no delay
after each send, on every sink `with_events` makes. The next frame's `tick` then drains the
event whichever view is drawn, or none while the window is hidden, so a compile behind the Save
editor's view still ends its activity and updates the window title (ST-TTL-03). A tool's own
deadlines (the music player's chant and VA timers) go through the same callback, as
`ToolContext::request_repaint_after(delay)` called from `tick`, which is what keeps them firing
while another tool is in view. We wake from the context, not from the tool's view
(`request_repaint_after` while a run is in flight, as the Phase 3 slice does), because a view
not drawn requests nothing: the run's last events, or a hidden tool's next deadline, would wait
for unrelated input. A run's end wakes it the same way: its verdict (the exit code, or a
refusal that emitted no event at all) reaches the view as the last message on the run's own
channel, sent before the wake, not read from the thread's `JoinHandle`, whose `is_finished`
can still be false when the frame the last event woke runs (TC-GUI-22). In the CLI the
callback does nothing. The match feed wakes the GUI the same
way, on publish.

The grid, log panel, and run strip are all views over the same tool state, updated from the
same receivers. The stadium compiler view reuses the same grid rendering code with rows =
stadiums and cells = components.

### Other UI components

| Component | Implementation |
|-----------|---------------|
| Common and per-tool settings forms | egui widgets (`TextEdit`, `CheckBox`, `ComboBox`), saved to TOML |
| Export tree browser | `egui::CollapsingHeader` tree, shows object model |
| Menu buttons | egui's `menu_button`, its label ending in ` ⏷` (the symbols font's) as the drafts draw every menu button (`menu()` in `docs/prototypes/index.html`): egui draws no arrow on a menu button outside a menu, and the arrow is what tells a menu from a button |
| Modals | Every modal (the settings menu, the About, the first-run prompt, each tool's dialogs and prompts) draws through one `studio_core` helper, the drafts' `.window` (`docs/prototypes/index.html`): an `egui::Modal` whose frame has 12 points of margin across and 8 down, at least 360 wide with its margins (`min-width`); its title (`theme::window_title`) then 8 points before the body (`.window .title`'s margin, where a label leaves egui's 3); its buttons in one row 10 under the body, right-aligned, in the order the drafts give them (`window_()`'s row). One helper rather than each modal's own frame, because the build's modals each took egui's default (about 6 points of margin, the buttons wherever the body ended) and drifted 6 to 9 points from every draft at once |
| Form grids | A form that draws a drafts' `table.grid` (a settings or field form: label, control, label, control) spaces its rows 22 apart, the drafts' 18-point control with its cell's 2 points above and below: `studio_core`'s one row-spacing constant, 4, is an `egui::Grid`'s row spacing (its column gap stays each form's own) and the `item_spacing.y` of a section whose rows are stacked in a vertical layout (General info's pairs); egui spaces both by `item_spacing.y`, 3, so each row of a long form lands a point higher than its draft's (25 points by the Ability stats' last). The theme's `item_spacing` stays 3: it spaces every other layout (the lists' rows, the toolbars) the drafts approved at 3 |
| File/folder pickers | `rfd` for native dialogs and supported web file picking; web directory access needs a separate File System Access API adapter (`rfd` has no WASM folder picker) |
| Dark/light theme | `studio_core`'s `theme.rs`: the curated dark `Visuals` preset, the default; light, second-class, egui's light `Visuals` with the same accent, rounding and status colors; chosen by the common `theme` setting and applied live (`Context::set_theme`, no restart; the setting, never the system's theme, decides, so a member on a light desktop still gets the dark default); icon glyphs from the suite's own icon font (`README.md` "GUI framework choice: egui"); body text in Roboto Flex, untuned and unscaled, bundled as static instances cut from its variable file (`resources/fonts/RobotoFlex-Regular.ttf`, and `RobotoFlex-Semibold.ttf` for the Tactics pitch's formation name; SIL Open Font License) at weight 400 (600) and optical size 12.5, the body text's size, every other axis at its default: the font as the trial drew it, since a browser sets the optical size to the text's size, where at the file's default of 14 it is 1.5% narrower and tighter than what was approved. One static cut per weight, not the variable file, which egui could draw at each size's own optical size: 96 KB against 1.8 MB, and the few sizes but 12.5 (the 15-point heading, 11-point small text) differ from the trial's optical size by a hair; it is the maintainer's pick, from the drafts drawn in Selawik (Segoe UI's metric twin, whose look did not please), Segoe UI and ten open fonts. It is not scaled or tuned to Segoe UI's width: the width constants the drafts were approved with are re-measured in it instead, each widened just enough that every row keeps the number of lines it had; monospace stays egui's Hack. Every glyph the UI draws comes from a bundled file, in this order: proportional Roboto Flex, the suite's symbols font, its icon font, `egui-icons`; monospace Hack, Roboto Flex, the symbols font, the icon font, `egui-icons`. The symbols the views draw that the body font lacks (→ ↓ ↳ ↻ ≡ ▸ ▾ ⏴ ⏵ ⏷ ● ▶ ⚠ ⓘ ✔ ✕ ✖ ⠇ − ≠ ≤ ∉) are the symbols font's (`resources/fonts/symbols.ttf`): each drawn as an SVG in `resources/symbols/` and built at its own Unicode codepoint by `scripts/make_symbols.py`, so a string keeps its character, the drafts load the same file, and one hand draws every symbol at the text's weight, not three fallback fonts' mismatched shapes (egui's `NotoEmoji-Regular` and `emoji-icon-font`, and Hack's arrows, were the alternative). Each keeps the advance and ink box of the glyph the drafts were reviewed with (Segoe UI's or Segoe UI Symbol's, measured), so no approved width moves; `ⓘ`, first drawn for Live AATF's suggestion mark, has no such glyph and takes the warning sign's stroke and height; `↳`, first drawn for a formation picture's instruction target, is `→` whole with a stem from its tail to the body font's cap height, since Segoe UI Symbol's, half as wide, turns too short to read at 12.5; `⏴` and `⏵`, first drawn for the Tactics popover's steppers, keep Segoe UI Symbol's advance and ink size with the ink centred in the advance, where Segoe's sit a point off-centre in opposite directions, since a stepper centres each in its own arrow button. The symbols are a file of their own, not glyphs of the icon font, because the file takes the body font's units and vertical metrics: egui places a glyph from a later font of the chain by centring that font's line height on the body font's, so a symbol in the icon font's metrics would sit 0.6 point below the text's baseline at 12.5, where the browser aligns every baseline; a body font changed later means a rebuild, not a redrawing. eframe's system font fallback stays for what a member types (a name in another script), never for the suite's own symbols: a symbol drawn from it is a different glyph per machine and a missing one in every test context, which has no system fallback (measured 2026-10-27: → ↓ ≡ ▸ ▾ ● ▶ ⚠ ✔ ✕ ✖ ⠇ all came from it, Ubuntu Regular and the icon font lacking them); every test context that draws a frame installs the same fonts and style as the window (`theme::install`), so a width or height a test measures is the one a member sees, whichever crates a build unifies. Until the preset is curated it is egui's own dark `Visuals` with 0.36's widget states (below), the colors the drafts are drawn in (the host page's stylesheet, `docs/prototypes/index.html`), so a curation pass changes the drafts' stylesheet first. The spacing, corner radii and widget colors are the drafts', which were reviewed at egui 0.36's: `theme.rs` pins 0.36's `button_padding` (4 × 1, where 0.37 has 8 × 2; the drafts' stylesheet pads a button 4 × 1 too, where it had padded 6 a side, so every button was 4 points wider in the drafts than in the app), its `interact_size` (40 × 18, where 0.37 has 40 × 20: the drafts' buttons, combos, number boxes and list rows all measure 18) and both themes' widget states whole (corner radii 2 to 3 points, where 0.37 has 4 to 6; the dark buttons grey 60 with text at 180, the stylesheet's `--widget` and button text, where 0.37 has 50 and 215), so an egui upgrade does not move the approved look. The quick actions keep their own 20-point height (`save_editor.md` "The quick actions"). The text sizes are the drafts' too: `theme.rs` pins `Small` 11, `Body` and `Button` 12.5, `Monospace` 12 and `Heading` 15 (the stylesheet's `.small`, body, `.mono` and `h2`), where egui 0.36 and 0.37 default to 9, 13, 13, 13 and 18, so every approved width and row holds: left at egui's defaults, the app's text was 4% wider than every draft's and its rows 16 points against their 15. A window's title, 14 in the drafts (`.window .title`, and the help window's and a floating panel's `.title`), has a text style of its own, proportional 14 in the strong colour: `theme::window_title` draws it as a modal's first line, and `theme::window_title_text` is an `egui::Window`'s title, which its title bar would otherwise draw in `Heading`, centred in the room the close button leaves (the drafts' help window and floating panels centre theirs the same way, since a left-aligned title would mean drawing egui's title bar, its drag, collapse and close, by hand); every modal's title takes it, the ones the drafts lack included (the Ruleset editor's unsaved prompt), since a title's place, not its draft, makes it a window's; a page's heading (the drafts' `h2`: a help topic's, a ruleset section's) stays `Heading`. A checked checkbox and a selected radio button are filled in the selection blue, the tick (2 points wide) or dot in the selection's text colour, hovered and pressed alike, so a choice made stands out (`studio_core::widgets`' `checkbox` and `radio`, every view's, painting over egui's own widget, whose look hook needs egui's `experimental` feature; `clippy.toml` disallows egui's own checkbox and radio everywhere else); egui's own draw both on the widget grey with the tick in the label's colour, where a checked box hardly stands apart from an unchecked one (the drafts' stylesheet, the CheckBox rules). A slider is egui's own (an 8-point rail, an 11 × 14 rectangle handle, its value in a drag value beside it), which the drafts copy rather than the browser's range input, with its handle and its rail up to the handle's centre in the selection blue: the rail by egui's trailing fill, the handle painted over in that blue, since egui fills it with the rail's own grey. Every colour the suite draws comes from egui's `Visuals` or from the palette, `studio_core::theme::Palette`: one named field per colour beyond the `Visuals` (the success green, the boots range's readout, a count's grey, the ratings' fills, the medals, the pitch's turf, lines and position colours...), with two instances, `Palette::DARK` and `Palette::LIGHT`, and `theme::palette(&visuals)` picking by `visuals.dark_mode`. No crate writes a colour from literals anywhere else: `scripts/colour_literals.py`, in the gates, fails on a `Color32` built from literal values (or a named one such as `Color32::WHITE`) outside `theme.rs`, where a constant written beside its use was how a colour came to have no light value; a colour computed from data (a name's colour codes, the colour picker's value) is not a literal and passes. So do `Color32::PLACEHOLDER`, egui's mark for the text's own colour, and `Color32::TRANSPARENT`, which are no colour (a layout made only to measure text takes `PLACEHOLDER`, not a white nobody sees); and test code, where a literal pins an approved value and so checks the palette rather than drawing from it. The colour picker's markers (a white ring and bar edged in black, on the colour field itself) are palette fields like any other, the same in both instances: `color_tools` depends on `studio_core`, as the wheel needs it to. The two instances are one struct, so a colour added to one and forgotten in the other does not compile. A light value keeps the dark one's contrast ratio against its background (egui's light panel is grey 248, a faint box about 253): light being second-class means fewer looks reviewed, not text left unreadable. A colour drawn on a surface both themes share (the pitch's turf and badges, a fill under its own text colour as the A rating's white on green) has the same value in both. The drafts are dark only, so a new colour's plan text gives its light value beside the dark one |

### The wheel over a value

The mouse wheel over a closed combo box or a drag value steps its value, as Windows' own
controls do, which members know from 4ccEditor: wheel up takes the item above or one more,
wheel down the item below or one less, clamped at the ends (no wrap), one step per notch, by 1
in the units the box shows. A notch is one line (or page) of a mouse wheel's events, and 50
points of a touchpad's, which come as a stream of small deltas added up, so a touchpad steps
once per notch's worth: counting a mouse line in points would not step until the second notch,
egui's line being 40 points on the desktop. One step a frame at most, the rest of a fast turn
stepping on the next frames, so each notch is its own undo step in an editor, which seals its
entry between frames. A notch first takes the keyboard from whatever widget holds it (a text
field being typed in, a drag value in typing mode, the stepped control itself included) and steps
two frames later, as a press on another control ends the focused one's gesture in an editor:
an editor seals only once no widget holds the keyboard, so a notch stepped while a field kept it
would join the field's undo entry, and every later notch with it. Two frames, not one: a field
that commits its value on losing the keyboard (the Ruleset editor's tier id, whose typing would
otherwise carry through another tier's id; egui's drag value commits each keystroke) learns of a
loss in the middle of a frame only on the next, so its commit lands there, sealed at that
frame's end, and a step on the same frame would join it. A disabled control ignores the wheel,
and an open combo's list scrolls as a list. The step goes through the control's own change path
(a combo's selection, a drag value's commit), so it is the edit a click or a typed value makes:
the same validation, one undo step per notch in an editor. The step takes the wheel's delta, so
the scroll area around the control does not scroll too. A wheel that began by scrolling a pane
keeps scrolling it while the pointer sweeps over such controls, until the wheel has rested 500
ms: a control sees the wheel only by being hovered, so without the rest rule scrolling the
player view would edit every box that passed under the pointer. One function in `studio_core`'s
widgets applies it to a control's response, and every combo box and drag value of the suite
calls it, the shared libs' included (`team_widgets` and `color_tools` depend on `studio_core`):
a wrapper per widget kind would leave out every control built with egui's own constructor. The
drafts do the same for every `select` and number input (`docs/prototypes/index.html`, the
wheel listener), so a draft can be tried as the app behaves (ST-WHEEL-01 to ST-WHEEL-04).

### No two widgets overlap

A headless test draws every screen of the Save editor at the default pane, on a PES 21 and a
PES 17 save: the player view with Appearance closed and open, the team view, and every dialog,
prompt and window the views open (each opened as its own tests open it). It fails when two
widgets' rectangles intersect. It reads the frame's AccessKit nodes of the widget and label
roles (those the pane-fit test reads), each at egui's own rectangle for it clipped to what its
scroll area shows, not at AccessKit's bounds: those are the whole rectangle, so a list row
laid out past its list's edge, which nobody sees, would collide with the buttons under the
list. It compares every pair on one egui layer of which neither contains the other: a dialog,
a window or a popup covers the view by design, and the view under it is still checked on its
own layer; a node drawn inside another (a lineup row's labels, a marker on the pitch) is part
of it, not a collision. An intersection counts once it is more than one point deep in both directions, the
draft tolerance (two rows touching share an edge). An overlap a screen means (a marker's badge
over the pitch) is listed in the test by role and label, with why, never by position: a
position would excuse whatever moved there next. Why a test and not the overlay alone: the
overlay shows a mismatch where a capture of the screen exists and someone looks at it, while a
collision a later change brings (a longer label, a widened field) shows on no capture until the
next look, and egui draws a row too wide for its room over its neighbour rather than refusing
it. Another tool's views join the test when they are built.

### Prototypes before implementation

Every tool's GUI, and every new shell element, is drafted first as an interactive web page and
refined with the maintainer (and shown to the members who asked to help shape it) before any
egui code is written: a page is quicker to change than a view, and a member can open it without
building anything. Before a draft is refined with the maintainer, the lead reviews it for
usability (a step a member repeats, a choice the view could make for them, a dead end after a
run), each idea a `PROPOSALS.md` entry ruled beside the page: a flow problem shows on a
page, not in a spec, and moved after implementation it costs a view and its GUI scenarios
instead of a page edit. The draft then becomes the view's specification, so it uses only the CSS
and HTML elements the UI crates we plan to use can represent: `egui`/`eframe` and the UI crates
of the dependency table in `README.md` "External Dependencies" (today `egui_commonmark`,
`egui_animation`, `egui_extras`, `rfd`), and the shell's icon font. Anything else would promise a screen the
implementation cannot deliver. An element only an unlisted crate can draw needs that crate added
to the table first (a new dependency, asked for first):

- **Representable, for example:** egui's widgets (label, button, checkbox, radio button,
  selectable label, combo box, slider, drag value, single- and multi-line text edit, hyperlink,
  image, progress bar, spinner, separator, colour button), its containers (collapsing header,
  grid, scroll area, horizontal and vertical layouts, top, bottom, side and central panels,
  window, modal), tooltips, context and bar menus, drag and drop between widgets and of files
  dropped on the window from outside, native file dialogs (`rfd`), Markdown text
  (`egui_commonmark`), the icon font's single-colour glyphs, colour or position transitions with easing
  (egui's `animate_*`, `egui_animation`), cards and rows that change on hover (a `Frame` over
  a container that senses hover, `UiBuilder::sense`), mixed inline formatting in one line
  (colour, size, background highlight, underline, strikethrough: egui's `LayoutJob`), and
  tables with sortable or resizable columns and a sticky header (`egui_extras`' `TableBuilder`;
  the sorting is ours).
- **Not representable, for example:** free CSS layout (flexbox or grid placement egui's layouts
  and `egui_extras`' `StripBuilder` cannot express), CSS effects beyond fill, stroke, rounding
  and shadow (gradients, blur, filters), and per-character text effects.
- **Look:** one shared stylesheet in Studio's dark theme, so every draft looks like Studio and
  the drafts look like one another, its body text at regular weight (400), not the light weight
  of egui's default font, which reads too thin. Drafts are not reviewed in the light theme,
  which is second-class. A control the browser would draw itself is drawn as egui draws it,
  measured in the built window, not left to the browser, whose look is the system's: the
  checkbox and radio button (a 14-point box or circle on the widget fill, the tick a
  polyline), the slider, a combo's open list (egui's popup under its button, opened by the
  draft's own code over the `select`), and a scroll bar where the app reserves a band for its
  floating one (the browser's hidden, the band kept clear).
- **Behaviour:** what a control does beyond its look (the wheel stepping a value, a drag, a
  key) is written in the plan with an acceptance scenario, not left to the draft alone: a
  comparison of the built window with the draft sees pixels, so a behaviour only the draft
  shows is lost without anyone noticing.
- **Fonts:** a draft draws text only in the app's own font files, loaded by `@font-face` from
  `docs/prototypes/assets/fonts/` in the app's fallback order (the Dark/light theme row in
  "Other UI components"), naming no system font and no generic family, canvas text included; no bold (egui has no bold face),
  but for the one text that imitates a game label in a bundled heavier face, the Tactics pitch's
  formation name (`--font-label`, `FONT_LABEL`: `plans/save_editor.md`, "The formation's name");
  every line at egui's row height, 1.2 times the font size, set explicitly, since egui rounds
  the font's `hhea` height (1.17 times the size for Roboto Flex) to whole pixels, 15 at 12.5
  and 18 at 15, where a browser keeps the fraction (and with Selawik took the Windows metrics,
  16 at 12.5); and egui's floor under a widget's height, `interact_size.y` (18: a button,
  combo, drag value, checkbox row or list row), stated as a `min-height`, which the browser's
  16-point line plus the padding used to supply by accident. A font named but not installed falls back silently: the drafts
  named Ubuntu, which no machine they were opened on had, and every layout was reviewed and
  calibrated in Segoe UI. `scripts/draft_fonts.py` (Chromium's `CSS.getPlatformFontsForNode`
  over every text node of every page) lists the fonts the drafts really draw in and fails on
  any other; it runs after every draft edit. Two things are no glyph of the app's: a member's
  own text (the sample saves' names, ♂ and fullwidth letters), which eframe's system fallback
  draws in the app too and which the script knows by its characters, and the simulated
  window's title bar, the OS's in the app, whose maximize box is drawn rather than typed.
- **Size:** the UI is drawn inside a simulated window of the window's default size ("Shell
  layout": 1280 × 780, fitted to 90% of the browser's `screen` size as Studio fits the
  monitor; CSS pixels are logical pixels, as egui's points are; every size is the client
  area's, as the app's are, the drawn title bar and border outside it), on a desktop-colored page,
  with the draft controls in one column on its left, in sections: the window (a size choice:
  the default, the smallest default of 1264 × 648, the minimum of 960 × 600, the whole
  monitor), the shell's controls grouped by surface, then the active tool's. A column rather
  than strips above the window, so the window starts at the page's top at every tool and a
  tool with many controls does not push it down the page. Narrower
  than the tool's design width the simulated window scales its content down with the width,
  as Studio does (`spec/core/gui.md` "Shell layout", "Scaling with the width"), so a narrow size shows what a
  member sees rather than a layout Studio never draws. Like Studio Web's window ("Browser deployment: Studio Web") it resizes
  from its bottom-right corner and maximizes to the page from its title bar (a button, or a
  double-click on the bar), and the column's window section reads its size live, in pixels and as a
  share of the monitor, which is how the default size is tuned. A full-page draft in a maximized browser shows room the real window
  does not have. The frame is the harness, not the UI, like the draft controls; its CSS
  transform makes the fixed-position menus and dialogs place themselves inside it, as egui's
  do in the window.

The drafts are one host page and a page per tool, in `docs/prototypes/`. The host,
`index.html`, holds what every tool shares, once: the simulated window above (size choice,
resize, maximize, the size readout), the shell's own elements (the sidebar listing every tool,
the status bar, the settings menu, the help window, the first-run dialog), the shared
stylesheet and the drawing helpers. Its sidebar switches the tool view, which draws the picked
tool's page: `tools/<tool>.js` (`save_editor.js`, `ruleset_editor.js`, `team_compiler.js`,
`export_upgrader.js`), a script that registers the tool with the host and draws only its view,
loaded by a `<script>` tag; its images are files under `assets/`. A script, not an HTML page
the host fetches or frames: a page opened from disk cannot fetch another file, and an iframe
would hold a tool's menus and dialogs inside the tool area, where egui's span the whole window.
A tool with a script is listed among the registered tools, as it will be once built; the
others keep their stub or their grayed name. The view uses the shell as the built one will: the
sidebar's PES selector is the one it reads, and what it reports in the status bar is a notice,
with a notice's lifetime. The host opens on the tool its address names (`index.html#save_editor`, the tool script's name),
so a pointer to one draft is one link.
One host is what keeps the drafts easy to change: the window's size, the sidebar or the theme
is edited once, not in every draft. Behavior is faked with sample data, never wired to the
real libs. A draft that must travel as one attachment is bundled by `scripts/bundle_drafts.py`
(the host, every tool script and its images inlined into one HTML file); easy prototyping comes
first, so the bundle is a convenience, not the format. Each tool page is the first step of its view's build: Phase 8's
list in `development_plan.md` names it ahead of every view, and each tool plan says where its
draft sits in that tool's build order. A draft is drawn whole, every screen the tool's plan
describes with sample data, before the first slice of the view is coded; a slice that changes
what the draft shows changes the draft first, so the draft stays the specification the view
is checked against.

**Layout edit.** The host's draft controls carry a "Layout edit" toggle, so the maintainer
tunes an arrangement by hand instead of describing it. While it is on, every panel a tool
script names (`data-panel="<name>"`: group boxes, columns, sidebars, bars; never a single
control) shows an outline, a handle that moves it and a corner that resizes it, and the
panels' own controls take no input. Each panel is tuned on its own. A moved panel is pinned
where it was dropped, in window points: no later resize of another panel moves it, and its
own slot in the layout stays as it was, so the move pushes nothing either. (Not an offset
from where the layout puts it: a neighbour's resize then dragged the moved panel along,
2026-10-26.) A resize sets the panel's width and height, and the panels around it that are not
pinned reflow, as they do in the window; a panel laid out in columns (the stats, the
positions, a grid of fields) can be narrowed past its columns' width, and then reflows into
fewer columns rather than stopping at its widest row (a draft aid alone: the built views never
narrow, `spec/core/gui.md` "Scaling with the width"). "Copy layout" puts one line per changed panel on the clipboard (the
tool, its screen or tab, the simulated window's size and the browser zoom, and each panel's
name with its rectangle in window points before and after); the tweaks survive a reload, and
"Reset" clears them. The string is evidence for the lead, not a layout file: the views lay out
by width rules (rows picked by width, caps, fixed cells), so the
lead turns a tweak into width constants and rules in the plan and the code, and reports one no
rule expresses (an overlap, a panel placed freely) rather than placing it by coordinates.
Panel names are the readable, stable half of the string: a draft renames a panel only with
its plan section.

**The host's other review aids**, each built once in the host for every tool's draft:

- **Remarks.** A "Remark" toggle beside "Layout edit": a click on a panel or a control opens a
  note field, and "Copy remarks" copies one line per note, `tool/screen › panel › control:
  text`, with its link (below). Remarks pinned to an element, not prose ("the box under the
  pitch"), which the lead maps to an element and sometimes wrongly. Notes persist like the
  layout tweaks, until "Reset".
- **Links.** The host keeps the current tool, screen and draft state (the tab, an open dialog,
  the team picked, the draft controls) in the URL's fragment
  (`index.html#save_editor/tactics?team=3&dialog=preset`), read and written through one host
  helper each tool script calls for its few keys; opening a link draws that state. Every
  remark and layout line carries its link, so the lead opens the exact state in Playwright
  rather than repeating clicks from a description.
- **Longest values.** A draft control that fills every text from the longest real values
  (player and team names from the fixture saves and `teams_list.txt`, the longest option of
  each list, the widest numbers), extracted once by a script into `assets/longest.js` (a script
  the host loads with a `<script>` tag, not a JSON file: a page opened from disk cannot fetch
  one); each
  tool script draws its texts through a host helper that picks the sample or the longest
  value. Short sample names hide clipping: a real 26-character name ("3DSMAX HAS STOPPED
  WORKING") overflowed the Tactics instruction combos at the default window in the built view,
  where the draft had shown no overflow.
- **The app's font.** The host loads the font files the app bundles by `@font-face`, at egui's
  sizes, so a text's width in the draft predicts the app's to within hinting; with the
  browser's own font a row that fits in the draft wraps in the app, and a width constant
  cannot be settled from the draft.
- **Overlay.** A draft control that lays a capture of the built window over the draft at half
  opacity, or beside it: captures taken at the simulated window's size by the lead's
  PrintWindow script into `assets/captures/<tool>_<screen>.png`, refreshed when a view's slice
  lands, each dated in its corner so a stale one shows. A shifted panel or a different gap
  shows at once, to the maintainer as to the lead, where two separate captures compared by eye
  miss it. A difference of one pixel between the draft and the app is accepted, two where
  matching closer would complicate the implementation (a browser and egui round text and
  borders apart, and chasing the last pixel costs code a maintainer then owns); a larger one
  is a mismatch, fixed or ruled. A layout test pinning a draft measure may allow the same.
- **Draft shots.** `scripts/draft_shots.py` captures every screen of every tool (the list from
  the tool scripts' registrations, reached through their links) at the default and the
  smallest default size with Playwright, and lists those whose pixels changed since its last
  run (a gitignored baseline folder). One edit to the host changes every tool's draft at once;
  the list shows the side effects on the tools nobody was looking at before the edit lands.

### Cancellation

What exists of this is in `spec/core/gui.md` "Cancellation".

During a run the Compile button becomes Cancel. A texture task checks the flag between its
textures, since it converts them one after another and a folder of large BC7 encodes can be one
of a run's longest tasks. It aborts cleanly: the run's staging is discarded and the previous
outputs stay as they were. In the grid, the cells not yet processed return to their check-phase
state (the run wrote nothing of theirs, so nothing is "done"), and the log's last line says the
run was cancelled, so a later reader of the log does not take a short run for a finished one.

**Closing the window** asks every tool, not only the one in view, and so does every close the
shell performs itself (the Team compiler's quick-compile close on success included): a tool
holding unsaved work (the Save editor's edits) reports it through a `StudioTool` hook beside
`version_change_blocker`, `fn hold_close(&mut self) -> bool` (default `false`): the shell calls
it in sidebar order until a tool returns `true`, which also opens that tool's prompt in its
view, and the shell then cancels the close (`ViewportCommand::CancelClose`) and makes that tool
the active view. The tools after it are not asked, so only one prompt shows at a time. Save or
discard sends a close request back to the shell (`ToolContext::request_close`, a
`ShellRequest`, as a tool switch is), which closes the window through the same path, asking
every tool again, so a second tool with unsaved work gets its own prompt in turn; cancel sends
nothing, which keeps the window and ends the close. A
close the shell decided alone would drop the edits of a tool whose view was not drawn. A tool
with a run in flight (a compile, an upgrade) reports it through the same hook, and its prompt
offers to close once the run has ended, cancelling it first where the tool has a Cancel (the
Team compiler's, above; an upgrade has none and runs to its end), the close request going back
when the run ends, or to keep the window: an exit cannot discard the run's staging as a cancel
does, and a run killed while writing leaves a torn output (an upgrade's half-written folder
then blocks the next run, EU-CLI-05) (ST-RUN-05, ST-RUN-06, TC-GUI-23, EU-GUI-07).

### Browser deployment: Studio Web

What exists of this is in `spec/core/gui.md` "Browser deployment: Studio Web".

The same `studio` binary crate compiles to WebAssembly (the eframe `wasm32` target), producing
**Studio Web**: a browser-served, deliberately *lite* edition of the suite. Its purpose is
convenience, not parity — a regular user on a PC other than their usual one opens a bookmark and
compiles their own team or edits its settings, without hunting for the tool's download link. It is
never meant to compile a full cup's worth of DLC; that stays the desktop binary's job. The name
"Studio Web" is user-facing, precisely so nobody expects the desktop feature set from it.

The user opens the URL, the WASM application loads, and from that point it works with local files
via the browser's File System Access API (`showDirectoryPicker`).

**Tool eligibility.** Tools split into three tiers by how much of the desktop platform they lean on:

| Tier | Tools | Web cost |
|---|---|---|
| Web-eligible, cheap | Save editor, Kit config editor, Music export editor, Export upgrader checks — tools that read a few user-picked files and write results back | Only the async file-access adapter; the lib crates and the egui shell compile unchanged |
| Web-eligible, pipeline-bound | Team compiler (single team), Balls compiler, Stadium compiler | The pipeline adapters below; single-export scale only |
| Desktop-only by construction | Match tracker (Win32 memory), Player aesthetics editor's Blender launch, self-update, elevation, live folder watching | Not offered; the WASM build hides them (saved-match viewing stays available — see the Match tracker plan) |

Differences from the desktop build:
- **A window on the page, not the page.** The browser is usually maximized, and the views are
  laid out for the desktop window's size ("Shell layout"), so stretched over the whole page
  they turn sparse. Studio Web draws the same shell (sidebar, tool view, status bar) inside one
  `egui::Window` titled with the display name (`<app>`, "Window title") over a plain backdrop, at the desktop window's default
  size, centered, movable and resizable, with a maximize button in its title bar that pins it
  to the whole page (`fixed_rect`) and restores the earlier rectangle. One shell function draws
  into either host: the desktop's root `Ui`, or this window's (`Panel::show` takes any `Ui`
  since egui 0.34). Not a window manager of one window per tool: on the desktop the system's
  window manager already does that job and Studio shows one tool at a time, and the page
  keeps the same one-tool shell. The floating windows (the shell's help and settings, the
  Save editor's panels) are the egui context's, not Studio's window's, so they float over the
  whole page rather than inside it, as a second desktop window would; a modal dims the whole
  page.
- **No folder watching**: browsers don't support `notify`. The user picks their exports folder
  manually; a "Rescan" button replaces the automatic watcher.
- **File access is permission-gated**: the user must grant access to each folder (exports, output,
  PES install, savefile). This is a browser sandbox constraint, not an egui limitation.
  `showDirectoryPicker` requires a secure context (HTTPS) and is Chromium-only at the time of
  writing; Firefox/Safari users get single-file picking, which covers the cheap tier but not
  folder-based exports.
- **Pipeline adapters are the real cost**: filesystem access is asynchronous and handle-based;
  blocking I/O, ordinary threads, and the `Condvar`-based `MemoryBudget` backpressure are not
  drop-in browser APIs. The pure format/domain logic is reused as-is; `libs/pipeline`'s scheduling
  and I/O get browser variants. Parallel WASM (`wasm-bindgen-rayon`) needs `SharedArrayBuffer`,
  which requires the server to send COOP/COEP headers (`Cross-Origin-Opener-Policy: same-origin`,
  `Cross-Origin-Embedder-Policy: require-corp`). Once built, every pipeline change carries a
  dual-target tax (`#[cfg(target_arch = "wasm32")]` forks in the scaffolding), which is why this
  tier ships after the desktop pipeline has settled, not alongside it.
- **Memory ceiling**: `wasm32` addresses at most 4 GB, and browsers typically grant less. Exports
  average ~200 MB and texture conversion expands them, so a single team compiles comfortably while a
  multi-team run does not. Studio Web enforces a small in-flight limit (one or two exports) rather
  than exposing the desktop thread/memory settings.
- **Embedded resources**: the ~6 MB of templates and fallback bins embedded in the desktop exe (see
  "Resolved decisions" in the Team compiler plan) are demand-paged on desktop but fully downloaded
  and resident in a `.wasm`. The template accessor (embedded → shadowed by the `templates/` override
  directory) is the single seam: the web build may swap the embedded source for lazy fetches behind
  it if the download size matters.
- **Transfer size:** a target, not a limit. The first load should stay at
  a few MB as the browser receives it (brotli or gzip compressed), so a member on an ordinary
  home connection waits seconds, not minutes; Phase 18 measures it. Past that, the embedded
  resources are the first thing to fetch lazily (the bullet above).
- **Native-only features stay unavailable**: live process tracking, Blender/PES launch, elevation,
  and binary self-update. Confirm archive and audio support on the chosen browser target before
  promising desktop parity.

**Hosting.** The intended host is the community's own server (the `implyingrigged.info` wiki
machine: nginx on Debian, admin access available), which can set the COOP/COEP headers that
GitHub Pages cannot; the `coi-serviceworker` workaround stays as a fallback for header-less hosts.
Deployment is a static directory (`.wasm`, JS glue, `index.html`) under an HTTPS location; the
build runs `trunk`/`wasm-bindgen` in CI and the release job copies the output over. Suitability is
evaluated when the web tier is scheduled — the requirements are just static files, HTTPS, and two
response headers.

### Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". `ST` is the shell's code (`studio_core`'s shell
and widgets, and the `studio` binary's launch modes); IDs are never reused; later phases append.
The shell is not a tool, so its scenarios live here, beside the sections they prove.

**Phase 8 scope.** Phase 8 completes the shell the Phase 3 slice started: the sidebar, the
settings menu, the status bar, the window title, the help window and `4cc-studio help`, the
log panel widget, the first-run data-location dialog, the `last_tool` reopening, the
`--gui` autorun and the window's opening size. What a scenario observes is what a frame draws (through the headless harness
of `studio_core/src/shell/mod.rs`: the frame's AccessKit nodes, a click as press and release
frames, the viewport and output commands a frame emits, a title or a clipboard copy; a color,
which no AccessKit node carries, is read
back from the shell state the frame paints from), the settings file, and what the binary
prints; `manual` marks only what needs a display, a native dialog or a running game. The tools in a
scenario are stub tools registered with the test's shell unless a real one is named; "the
settings file" is one under the test's own folder (`AppPaths` pointed there), never the
developer's. The Team compiler's own toolbar, grid and cancellation are `TC-GUI` scenarios
(`plans/team_compiler/README.md` "Acceptance"), the Save editor's view `SE-GUI` and the Export
upgrader's `EU-GUI`.

**Window** (ST-WIN-01 and ST-WIN-02 are built: `spec/core/gui.md` "Acceptance")

```
ST-WIN-03   GIVEN two main stub tools and a special stub tool declaring a 480 × 640 size
            WHEN the active tool goes from the first main tool to the second
            THEN no size or position command is sent; WHEN it goes to the special tool THEN the
                 window is asked for 480 × 640, centered; WHEN the member resizes it and moves it,
                 and the active tool goes back to a main tool THEN the main class's last size and
                 position are restored; WHEN it goes to the special tool again THEN its own last
                 size and position
ST-WIN-04   GIVEN a main tool active and the window moved to (100, 50) and resized to 1100 × 700
                 on a 100% display
            WHEN the window closes and the GUI starts again
            THEN the settings file holds window_main as [100, 50, 1100, 700, 1, 0], the
                 native window's options carry the position (100, 50) and the inner size
                 1100 × 700 and are not centered or maximized, and the first frame sends the
                 outer position (100, 50) and no size, not the fit's; GIVEN the same on a 200%
                 display, the window 2200 × 1400 pixels THEN window_main is
                 [100, 50, 1100, 700, 2, 0], the options' position (50, 25) and inner size
                 1100 × 700, and the first frame's (50, 25); GIVEN the window maximized when
                 it closes, or minimized from maximized THEN window_main is the place it had
                 before the maximize with 1 last, the options carry that place and are
                 maximized, and the first frame, the primary monitor's scale and its own the
                 record's, sends the maximize alone; GIVEN the same with the primary
                 monitor's scale, or the frame's, another than the record's THEN the first
                 frame sends the outer position, then the maximize;
                 GIVEN a window_main of five numbers THEN the options are not
                 maximized; GIVEN a remembered place whose title bar
                 lies on none of the window's monitors THEN the window is created at the place
                 and its first frame sends ST-WIN-01's fit instead of the position, preceded
                 by an unmaximize when the record says maximized; GIVEN no
                 monitor list (a frame with no window) THEN the first frame sends the place's
                 position; GIVEN a last tool of the special class THEN it opens at
                 window_special's place
```

**Sidebar**

```
ST-SIDE-01  GIVEN three registered tools and a settings file whose last_tool names the second
            WHEN the GUI opens
            THEN the sidebar's rail lists the three in registration order, the second is
                 active and its view is the one drawn, and the opened sidebar shows their full
                 labels
ST-SIDE-02  GIVEN the GUI open on a tool
            WHEN the pointer rests on the sidebar's rail, then leaves it
            THEN the full sidebar with its labels is drawn over the tool's left edge while the
                 pointer is on it, the tool's view keeping its position and width, and only the
                 rail is drawn once the pointer has left; a click on a rail entry switches to
                 that tool without the hover
ST-SIDE-03  GIVEN the selector at PES 21 and pes_folder_path resolving to a folder holding
                 PES2019.exe
            WHEN the sidebar is drawn
            THEN the selector is red and its tooltip names the exe found; GIVEN the folder
                 holding no PES exe, or missing, THEN the selector is orange and the tooltip
                 says what was found; GIVEN it holding PES2021.exe THEN neither
ST-SIDE-04  GIVEN a tool reporting a version_change_blocker
            WHEN the sidebar is drawn
            THEN the selector is disabled with the tool's reason as its tooltip, and the setting keeps its
                 value; GIVEN no tool blocking WHEN another version is picked THEN the settings
                 file holds it and every tool reads it at its next frame
ST-SIDE-05  manual  GIVEN the selector at PES 21 and PES2019.exe running from a folder other than
                         the resolved pes_folder_path
                    WHEN the poll next runs
                    THEN the selector switches to PES 19 once, the status bar shows the notice
                         naming the switch with a "Fix PES path" action, the action rewrites
                         pes_folder_path from the running process (the ** placeholder put back
                         where the folder's own name holds the version's year), and a later
                         manual change back to PES 21 is not switched again while the program
                         lives
ST-SIDE-06  GIVEN the GUI open
            WHEN the logo is clicked
            THEN the About popup shows the version block (version, build kind, target triple,
                 data location, selected PES version), links to the release page, the repository
                 and the help, and its Copy button puts exactly that block on the clipboard
ST-SIDE-07  GIVEN a build registering two of the suite's tools
            WHEN the sidebar is drawn
            THEN the suite's other planned tools are listed after them, grayed and not
                 clickable, each tooltip saying the tool is not in this build
ST-SIDE-08  GIVEN a build registering two window tools and one marked CLI-only, and a settings
                  file whose last_tool names the CLI-only one
            WHEN the GUI opens
            THEN the CLI-only tool is listed as a planned entry, grayed and not clickable, its
                 tooltip saying it runs from the command line in this version; the first window
                 tool is active; the CLI-only tool's help chapter is in the help window, the
                 settings menu has no section for it, even with a settings table; and its CLI
                 commands still run
```

**Settings menu**

```
ST-SET-01   GIVEN two registered tools with settings sections and the second active
            WHEN the settings gear is clicked
            THEN the menu shows the common section and one collapsible section per tool, the
                 second tool's expanded and the first's collapsed
ST-SET-02   GIVEN the menu open
            WHEN a common setting (`thread_count`) is changed
            THEN the settings file holds the new value under [common], every tool table as it
                 was, and ctx.common() returns the new value on the next frame
ST-SET-03   GIVEN a data directory that cannot be written
            WHEN a setting is changed
            THEN the change applies for the session, the status bar's left slot shows the
                 data_dir_read_only condition, and no dialog or prompt opens
ST-SET-04   GIVEN the selector at PES 2019 and the menu open
            WHEN `D:\Games\Pro Evolution Soccer 2019` is stored as a pick on the PES folder
                 row (the pick's rule called with that folder: the native picker is manual)
            THEN pes_folder_path holds `D:\Games\Pro Evolution Soccer 20**`, and with the
                 selector at PES 2021 the resolved folder is `…Soccer 2021`; GIVEN `E:\PES`
                 THEN it is stored as given; WHEN A folder per version is switched on and PES
                 2021's row typed `F:\PES2021` and committed THEN [common.pes_folders] holds
                 21 alone, PES 2021 resolves there and PES 2019 to the main row's, and the
                 toggle cannot be switched off; WHEN Fix PES path is applied for a running PES
                 2019 at `G:\PES 2019` (the notice's action applied with that folder: the
                 running process is manual) THEN [common.pes_folders] gains 19 and the main row is
                 unchanged; WHEN both rows are emptied THEN the entries are gone and the toggle
                 can be switched off
```

**Status bar and window title**

```
ST-BAR-01   GIVEN no condition, no background activity and no notice, the window maximized
            WHEN the status bar is drawn
            THEN it shows the version number in small text and the Reset window button and
                 nothing else, at its fixed height
ST-BAR-06   GIVEN a main tool active, the window at 1280 × 780 on a 1920 × 1080 monitor
            WHEN the status bar is drawn
            THEN it draws no Reset window button; GIVEN the window resized to 1100 × 780,
                 or maximized, THEN the button is drawn; GIVEN a special tool declaring
                 480 × 640 active, its window at 480 × 640 and moved, THEN no button
ST-BAR-05   GIVEN a main tool active, the window maximized on a 1920 × 1080 monitor
            WHEN Reset window is clicked
            THEN the window is restored and asked for 1280 × 780, centered on that monitor;
                 GIVEN a special tool declaring 480 × 640 active and its window moved and
                 resized THEN it is asked for 480 × 640, centered
ST-BAR-02   GIVEN the first tool active and the second reporting a ToolActivity
            WHEN the status bar is drawn
            THEN its right slot shows the second tool's activity; GIVEN the second tool made
                 active THEN the slot is empty (its own run strip shows it)
ST-BAR-03   GIVEN a tool publishing a Notice with an action
            WHEN the status bar is drawn
            THEN its middle slot shows the text, the one action and the dismiss X, centred in
                 the space the left and right slots leave, as one group, until the action is clicked or the notice dismissed; GIVEN a notice without an
                 action THEN it leaves the slot by itself after a few seconds, and a later
                 notice replaces an earlier one; GIVEN a 1280-point window, the version on the
                 left, and a notice of 100 characters with an action THEN its whole text is
                 drawn; GIVEN one longer than the bar holds THEN its text ends in an ellipsis
                 inside the middle slot and its tooltip is the whole text
ST-BAR-04   GIVEN three notices published in turn, one of each kind (Success, Info, Warning)
            WHEN the status bar is drawn after each
            THEN each notice's text is drawn in its kind's colour: SUCCESS_FG, the link blue,
                 the warning orange
ST-TTL-01   GIVEN a tool reporting a ToolActivity (label "Team compiler", 12 of 48 done, 72 s
                 elapsed)
            WHEN a frame is drawn
            THEN the viewport title command reads "Team compiler · 12/48 · 01:12 - <app>",
                 sent once and not again while the string is unchanged; GIVEN no activity and
                 a stub tool "Stub A" active THEN "Stub A - <app>"; GIVEN "Stub A" naming its
                 open file `C:\saves\EDIT00000000` (`title_detail`) THEN
                 "Stub A [C:\saves\EDIT00000000] - <app>", and while the activity runs the
                 busy title above, no path in it
ST-TTL-02   GIVEN the window unfocused and no tool reporting an activity
            WHEN a notice "Compiled 48 teams" arrives
            THEN the title reads "Compiled 48 teams - <app>" until the window gains focus,
                 then the active tool's idle title ("<tool> - <app>"); GIVEN the window
                 focused when it arrives THEN the title stays the idle one
ST-TTL-03   GIVEN the window focused, two stub tools, the second's view active, and the first's
                 run on a thread of its own
            WHEN the run emits its last event through its ToolContext and no input follows
            THEN the emit requests a repaint, and the next frame's tick drains the event into
                 the first tool's state (its activity ended, the title the second's idle one) without its
                 view being drawn
```

**Help**

```
ST-HELP-01  GIVEN the Team compiler active
            WHEN F1 is pressed, or the sidebar's ? clicked
            THEN the help window opens on the Team compiler's chapter, the chapter tree lists
                 one chapter per registered tool in sidebar order after the core chapters, and
                 the chapter's last topic, Messages, lists every code of the tool's catalog with
                 its severity
ST-HELP-02  GIVEN the help window open
            WHEN a message code is typed in the search box
            THEN the results list its catalog row as "Chapter › Topic › line", and clicking the
                 result shows that topic with the line highlighted; WHEN a word of a topic's
                 body is typed, in another letter case, THEN that topic's line is listed the same
                 way
ST-HELP-03  GIVEN a tool's log panel showing a finding line
            WHEN its message code is clicked
            THEN the help window opens at that code's catalog row; GIVEN a topic holding an
                 in-doc link `[text](tool-id#topic-slug)` to another tool's topic WHEN the link
                 is clicked THEN that topic is shown
ST-HELP-04  GIVEN the built binary
            WHEN `4cc-studio help team-compiler` runs
            THEN stdout is the chapter's Markdown; WHEN `4cc-studio help --export <dir>` runs
                 THEN <dir> holds one .md file per chapter, each tool's ending with its Messages
                 topic
ST-HELP-05  GIVEN Studio's window at 1037 × 648 points
            WHEN the help window opens for the first time
            THEN its rectangle is 933 × 583 points (90% of the window's),
                 centered in the window; GIVEN the window at 1280 × 800 THEN 1152 × 720
```

**Log panel widget**

```
ST-LOG-01   GIVEN a log holding more lines than the display cap
            WHEN the panel is drawn
            THEN only the last lines up to the cap are drawn, the newest at the bottom
ST-LOG-02   GIVEN the panel scrolled up by the user
            WHEN new lines arrive
            THEN the view does not move and a "↓ N new" indicator counts them; clicking it
                 returns to the bottom and auto-scroll resumes
ST-LOG-03   GIVEN a log holding info, warning, error and fatal lines
            WHEN the Warnings filter is chosen
            THEN the warning, error and fatal lines are drawn and the info lines are not; Errors,
                 the error and fatal lines alone; All, every line again
ST-LOG-04   GIVEN the log panel open at its default height
            WHEN its chevron is clicked
            THEN the panel collapses to its header row and the view above it takes the space,
                 and a second click restores it at its height; WHEN its divider is dragged THEN the panel takes the dragged height, kept across
                 frames
```

**Launch and close**

```
ST-RUN-01   GIVEN neither settings file exists
            WHEN the GUI starts
            THEN a dialog asks for the data location (portable data/ or the user config
                 directory) before any tool view, and the chosen location's settings.toml exists
                 afterwards; GIVEN one exists THEN no dialog and that location is used
ST-RUN-02   GIVEN the GUI closed with the second tool active
            WHEN `4cc-studio` starts again
            THEN the second tool is active
ST-RUN-03   GIVEN a tool whose gui_run does not implement the command
            WHEN `4cc-studio --gui <tool-id> <command>` runs
            THEN it exits with a message naming the tool, and no window opens;
                 GIVEN a tool implementing it THEN the GUI opens with that tool active and the
                 command runs as if its button had been pressed
ST-RUN-04   manual  GIVEN the Windows bundle
                    WHEN the exe is double-clicked
                    THEN no console window opens beside the GUI; WHEN `4cc-studio team-compiler
                         check` runs from a terminal THEN its output reaches that terminal
ST-RUN-05   GIVEN two stub tools each reporting unsaved work, a third tool's view active
            WHEN the window's close is requested
            THEN the window stays and the first stub's view is active with its prompt; WHEN
                 it discards THEN the second stub's view is active with its prompt; WHEN that
                 one discards THEN the window closes; GIVEN either prompt cancelled instead
                 THEN the window stays and no further prompt shows
ST-RUN-06   GIVEN a stub tool reporting a run in flight, another tool's view active
            WHEN the window's close is requested
            THEN the window stays and the stub's view is active with its prompt; WHEN it
                 chooses to close THEN the window closes once the run has ended, not before;
                 WHEN it chooses to keep the window THEN the window stays and the run goes on
ST-RUN-07   GIVEN a data location holding `studio.log` and `studio.old.log` from earlier runs
            WHEN the GUI starts
            THEN `studio.old.log` holds what `studio.log` held, the new `studio.log`'s first
                 line names the version, and a line logged with the home folder in it reads
                 `~` there; GIVEN no data location yet (a first run) THEN no log is written
ST-RUN-08   manual  GIVEN a build whose window panics on a click (a test build, the panic
                    added by hand and reverted after)
                    WHEN the click is made
                    THEN a dialog names `studio.log`'s path and repeats the panic's message,
                         the process exits with 101 once it is closed, and the log ends with
                         the panic's message, its location and a backtrace
ST-RUN-09   GIVEN a data location whose `studio.log` ends without the closing line (a session
                  that crashed or was ended)
            WHEN the GUI starts
            THEN before the window opens a dialog says the last session did not close
                 normally and names `studio.old.log`'s path; GIVEN it ends with the closing
                 line of a normal close or of a crash already reported THEN no dialog; GIVEN
                 no earlier log THEN no dialog
ST-RUN-10   GIVEN the built exe started over a test data location with `--crash-test stack`
                 (handled once the monitor runs, before any window: no display needed)
            WHEN the trigger overflows the stack
            THEN `studio.dmp` is written beside `studio.log`, reads as a minidump holding a
                 thread list and the exception stream of a stack overflow (read in the test
                 by the `minidump` crate, a dev dependency), and the log's last line is the
                 monitor's naming the dump; the next start's dialog text (ST-RUN-09's, built from the old log and
                 the dump) names the dump's path beside the log's; GIVEN the monitor's start
                 given a program that is not an exe THEN it returns the failure and the log
                 holds the warning
ST-RUN-11   manual  GIVEN ST-RUN-10's dump from a release build
                    WHEN `minidump-stackwalk` reads it against that build's kept `.pdb`
                         (through `dump_syms`' symbol files)
                    THEN the crashed thread's stack names the trigger's function
ST-CLI-01   GIVEN a data directory that cannot be written
            WHEN a CLI command runs
            THEN stderr starts with the data_dir_read_only condition as one line, and a notice a
                 tool publishes during the command prints as one line when it occurs
ST-CLI-02   manual  GIVEN a compile run from a terminal
                    WHEN it runs
                    THEN one progress line is drawn on stderr and redrawn in place, cleared while
                         a finding line goes to stdout, and one summary line remains when the run
                         ends; GIVEN the same command with stderr piped THEN neither is drawn and
                         stdout holds exactly the lines it holds in a terminal
```

**The wheel over a value** ("The wheel over a value")

```
ST-WHEEL-01 GIVEN a scroll area taller than its view holding a drag value at 180 (range 0 to 255)
                  and a combo box on the second of five items, the wheel at rest
            WHEN the pointer is over the drag value and the wheel turns one notch up
            THEN the value is 181 and the scroll area's offset is unchanged; WHEN two notches down
                 THEN 179; WHEN the pointer is over the combo and the wheel turns two notches up
                 THEN the first item is selected (one step, then clamped, no wrap)
ST-WHEEL-02 GIVEN ST-WHEEL-01's scroll area
            WHEN the wheel turns over the area's empty space and, under 500 ms later, again with
                 the pointer over the drag value
            THEN the value is unchanged and the area's offset moved by both turns; WHEN the
                 wheel then rests 500 ms and turns one notch up over the drag value THEN the value
                 is 181
ST-WHEEL-03 GIVEN a disabled drag value at 180, and a combo box with its list open
            WHEN the wheel turns one notch over each
            THEN the drag value is 180 and the combo's selection is unchanged
ST-WHEEL-04 GIVEN ST-WHEEL-01's scroll area and a text field holding the keyboard, the wheel at rest
            WHEN the pointer is over the drag value and the wheel turns one notch up
            THEN on that frame the field no longer holds the keyboard and the value is 180; on the
                 next frame it is still 180, and on the one after it is 181
```
