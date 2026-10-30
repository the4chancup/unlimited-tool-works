# Unlimited Tool Works — Core plan: Development plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Development Plan

What exists of this is in `spec/core/development_plan.md` "Development Plan".

**Releases.** The suite first ships as **0.1.0 at the end of Phase 8** ("Release 0.1.0" below):
the Team compiler with compile-time aesthetics (written to the savefile through the
aesthetics patch), the Save editor and the Export upgrader, on
Windows, with the CPU texture encoder and the native model formats only. Phase 8 is the line
because it is the first point where the two main tools exist with a GUI, and because its own
verification (community feedback) needs a distributed build; every later tool replaces a legacy
tool that keeps working until then. The phases therefore run in the order 1–6, 8, Release 0.1.0,
7, 9–16: glTF (Phase 7) moves after the release, since no current export carries glTF and it
depends on the Blender `pes-models` codec outside this workspace. Phase numbers are kept, since
every pointer names them. Releases stay `0.x` until Phase 16 closes with **1.0.0**; "first
release" in the plans means 1.0.0, so a "first-release" feature may land after 0.1.0.

**`0.x` releases are test releases.** They go to a restricted group of testers, who are told in
person of every change that reaches them, so everything can still change: no released surface
(export layout, `settings.toml`/`config.toml` keys, CLI flags and exit codes, finding codes) is
a compatibility contract before 1.0.0, and the maintenance-mode procedure that would hold them
to one is written for 1.0.0 (Phase 16). What a tester would notice still gets its
`CHANGELOG.md` line, which is what the personal notice is written from. The help chapters
beyond the per-view topics, the update check and the teams-list merge, which a wide audience
needs and a tester told in person does not, ship in Release 0.2.0 or later.

**What the version number means.** The suite's version is semver-shaped, because Cargo
requires that shape, the update check compares the release tag with `CARGO_PKG_VERSION` as
semver (`distribution.md`), and the milestones above are written in it. The meaning is an
application's, not semver's API contract: nothing compiles against the suite, so there is no
API for a major number to protect. The major marks an era: 1.0.0 is the feature-complete
release whose surfaces are contracts, and a later major is for a change of that weight (support
for a PES generation dropped), so it stays 1 for a long time. The minor is a feature release,
the patch fixes only. Compatibility is promised per surface instead, each with its own rule:
the export layout stays readable by every later release (the format the cups rely on is never
broken; a new layout would arrive through the Export upgrader), the AATF logic carries its own
version (a change of verdicts is a new logic version, never the same one rewritten), the
aesthetics patch its allocation scheme version, the Team TOML only ever gains keys (an absent
key leaves the team's value untouched, so an older file still applies), and the
settings keys, CLI flags and exit codes, finding codes and `.4ccm` follow the maintenance
mode's rule from 1.0.0 (a migration or a clear refusal, never a silent reinterpretation). A
release's number never stands in for one of these: a member reads a surface's own version,
not the app's, to know whether a file still works. Calendar versions (2026.11) would match the
cup seasons but drop the milestones the plans are written in.

**Divegrass Before Christmas (DBC).** The invitational (exports due 2026-11-18) is the first
test ground for the Save editor, which members should have as early as the usual rigor allows,
to get used to it before the deadline. What matters most there is a working editor whose AATF
check is rock solid (`libs/aatf`), so after Phase 6 closes the work runs: the DBC ruleset's
coverage (worklog 20.0d), Phase 8's usability pass (8.4) and its shell, the Save editor's draft
and view first among Phase 8's tool views, then Phase 8's other views and Release 0.1.0 as
above. Members get the Save editor as a **preview build, 0.0.1**, cut once its view is done:
the shell with that tool active, and the Team compiler, the Export upgrader and the Ruleset
editor, whose views come later, greyed out on the sidebar as the planned tools are. The exe
embeds every cup's current rulesets (`spec/aatf_rules.md` "Embedded rulesets"), DBC7 and VGL27
among them, and the bundle's settings pick DBC7 as the Save editor's `aatf_ruleset` and PES
2017, DBC's game, as the sidebar's `pes_version`: 0.0.1 is DBC's alone, its testers DBC's
managers, whose first open shows no version warning (the app's own default stays PES 2019).
No rules file rides beside the exe: one in the data folder's `rules/` would list as an
import, the same ruleset twice in the dropdown, its copy one × from deleted. The settings file
lives in the bundle's `data/` folder, `data/settings.toml`, which makes the bundle portable
(`spec/core/distribution.md` "Data location: asked on first run"), so 0.0.1's first start asks
nothing: the location choice is what a preview's testers lose, against a check that is DBC's
from the first open. The settings file also carries the Team compiler's
`multicpk_mode` and `cpk_name` at their defaults, the two lines VGL's organizer changes in
0.0.2 (below), so that readme can say which line to edit rather than what to add. It needs from Release 0.1.0's list the Windows
bundle (`just release`, the `.7z` and its checksum), and from Phase 8's the GUI-subsystem exe
(`architecture.md` "Windows subsystem": no console window beside the window, ST-RUN-04) with
the window's diagnostic log (`architecture.md` "Diagnostic logging"), since without a console
a panic would leave nothing a member could post; nothing else: no updater, since 0.1.0
replaces it by hand, and no `CHANGELOG.md`, which starts with 0.1.0 (0.0.1's few testers are
told in person what it holds). We do not shorten the Save editor's phase to meet the date: a
tool members learn on is one whose behavior they will keep relying on, and a rushed rule check
that passes a team the organizers' check refuses costs the invitational more than a later tool.
The Ruleset editor (Phase 20) ships in 0.0.1 as far as it is built, its sidebar label and
window title "Ruleset editor (WIP)": opening and saving, the start page and templates, the
preview, General, Tiers, Custom checks and Summary. Heights, Cards, Universal rules, Specials
and Suggestions draw their heading and one line saying they cannot be edited yet, that
Summary shows what they hold and that saving leaves them unchanged; a save keeps them as the
file held them. Under that line, a link reading "Check out a demo of the planned section
online" opens the section in the drafts (`docs/prototypes/`, published by GitHub Pages:
`https://the4chancup.github.io/unlimited-tool-works/prototypes/#ruleset_editor/<section>?demo=1&start=0`,
`<section>` the drafts' id: `heights`, `cards`, `universal`, `specials`, `suggestions`), in
the drafts' demo mode: no draft controls, the window at its default size on a page it can be
moved and resized over (a page smaller than that is the monitor the window is fitted to, as
the app fits its window to a small one). The drafts are not held to these addresses: a section
renamed there leaves the link opening the tool's first section. A demo link, not nothing, because the drafts of these sections are
finished but for polish, and seeing one beats reading the Summary; only a section whose draft
is that far along gets one (the other tools' drafts are not yet). The github.io address, not
a custom domain, until one is live: the old address redirects to it, and a 0.0.1 build keeps
its links. Its other forms (the condition and effects editors among them), Randomize
and its close run after Release 0.1.0. Not the whole tool before the cut: those forms are
the hardest left, and building them would split the time the check needs. Not left out
either: what is built has been reviewed, and it lets an organizer change a ruleset's tiers
and General settings in a window rather than in the rules file's text.

**The /vg/ League 27 (VGL): preview 0.0.2.** The league starts in late November 2026 on PES
2019 with FoxDen, and its organizer tests the Team compiler there: managers send their exports
in the old format, as for any cup; the maintainer and the organizer upgrade them with the
Export upgrader, compile them, and fall back to Red if a major issue surfaces. So a second
preview, 0.0.2, carries those two tools through their CLI. 0.0.1 goes out without them: the
Save editor reaches DBC's managers before the two tools are tested for the league, and a
launcher nobody tested for the release has no place in its bundle. Its bundle therefore has
neither launcher, nor the `exports/` and `old_exports/` folders, nor the readme's VGL section;
0.0.2 adds them back, as they are written now, with a readme that has the organizer pick PES
2019. The tools' CLI is in 0.0.1's exe all the same, as every tool's is. They need no view
for it, and the window stays as above, the two greyed out. The bundle's launchers run the CLI
in their console window, as Red's own bat runs its CLI, and wait for a key at the end so the
report can be read:

- `quick_compile.bat` runs `team-compiler compile` over the bundle's `exports/` (through
  `UTW.exe`, "The project's name" below).
  It takes its `--gui` form (`architecture.md` "Launch modes") when the Team compiler's view
  lands.
- `quick_upgrade.bat` upgrades every export in the bundle's `old_exports/` (folders, `.zip`,
  `.7z`) into `exports/` against the savefile dropped onto it, then prints how many were
  upgraded and how many were not. A team export needs `--savefile` or `--loose` (`spec/export_upgrader.md` "CLI"),
  and loose mode is an assumption the user opts into, so the bat asks for the save rather than
  falling back to loose; with no file dropped it says how to drop one and stops. An export
  already upgraded is refused on a second run, its folder in `exports/` existing (replacing
  it would lose edits made since), and counted as not upgraded; the bat cannot tell that
  refusal from another, so the readme tells the member to move upgraded old exports out of
  `old_exports/`, which an organizer receiving exports over several days needs. The bat goes
  when the Export upgrader's view lands.

The organizer's PES 2019 install carries the official DpFileList (an install with another
list takes it with `team-compiler upgrade-dpfl`, run once from a console in the bundle's
folder; no launcher runs it, so the readme gives the line), and the league's initial DLC compiles in multi-CPK mode
(`multicpk_mode` set in the settings file, as the readme shows): the team content fills
`4cc_41_teams` and the slots after it by size, the bins go to `4cc_08_bins`, and a later
matchday compiles single-CPK into its `midcup` slot (`spec/team_compiler/pipeline.md`
"Multi-CPK mode: teams parts"). The defaults stay as they are: the official list is what every
install is meant to carry, and `dpfilelist_outdated` already names the upgrade.

The console's progress line and run summary (`architecture.md`, ST-CLI-02) come with them,
pulled forward from Phase 8's progress-grid step: a whole league compiled with nothing on the
screen for minutes reads as a hung program. Two checks join the release's own:

- an interrupted run (Ctrl+C in the console) leaves no half-written file where the game reads
  it;
- a rehearsal on PES 2019: a past VGL's old exports upgraded and compiled from the extracted
  bundle through the two launchers, then checked in game with FoxDen.

Both tools stay what their phases delivered; nothing is shortened for the league's date
either.

**The project's name: Unlimited Tool Works.** The suite is Unlimited Tool Works, UTW for
short, wherever a user sees it: the window title, the About box, the first-run prompt, the
CLI's `--help` header and its usage examples, the help pages (and their `help --export`
files), the readme, the log, the executable and the bundle. The name began as the 0.0.1
preview's codename, meant to give way to "4cc Studio" at 1.0.0, and is the project's own from
0.0.1 on: the first release fixes the executable's name, the settings folder and the bundle's
name, which a later rename would have to migrate on every member's machine. The reference
(Fate/stay night's reality marble, where every sword once seen is traced) is the community's
own and fits a suite that traces every tool its cups have used. Its spellings:

- **`Unlimited Tool Works`**, the display name: the window title, the About box, the log's
  first line, the exe's version details, and a written CPK's `Tvers` field
  (`Unlimited Tool Works <version>`, the tool that wrote it).
- **`UTW.exe`** on Windows, **`utw`** on Linux, the executable: cargo builds the bin `utw`,
  and the release recipe names it `UTW.exe` in the bundle. The command line is second-class,
  so its users can type the capitals (Windows ignores their case anyway), and the CLI's usage
  lines and `--version` (`UTW v0.0.1 pre-alpha`, the executable's name before the version,
  as a CLI's own line reads) print the platform's spelling.
- **`unlimited-tool-works`** in paths and packages: the settings folder under the user's
  config directory (`%APPDATA%\unlimited-tool-works`, `~/.config/unlimited-tool-works`), the
  bundle's top folder and its archives (`unlimited-tool-works-<version>-windows.7z`, the
  symbols' beside it), the folder of a compiled CPK's shared-IDs records
  (`unlimited-tool-works/shared_ids/`), and the window's app id, which a Linux desktop entry
  matches.
- **`utw`** in the short file names a member handles and posts: the log `utw.log`, with
  `utw.old.log` and `utw.dmp` beside it.

Nothing is read under the old names (`4cc-studio`, `4cc_studio/shared_ids/`,
`studio_version`): no release wrote them, 0.0.1 being the first, and the maintainer's test
installs are compiled again. The code reads the display name and the executable's from two
constants in `studio_core`. The help pages, the launchers, the readme and `studio`'s build
script spell them as text, since a Markdown page, a batch file and a build script cannot read
a Rust constant; a test over the `help --export` files and the CLI's `--help` output fails on
any "4cc Studio", `4cc-studio`, `UnlimitedToolWorks` or bare "Studio" left. Text a file
format keeps names no product at all (a ruleset file's header and its logic banner, the
shared-IDs file's comment: "written by the Ruleset editor", "by the Team compiler"; the
aesthetics patch's `tool_version` key, `studio_version` until the rename): the logic banner is
hashed into the ruleset's identity, and a product's name in a format ties every file to it.

What users never see keeps the working name, Studio, as the project's codename for good:
the crates `studio_core` and `studio`, their types (`StudioApp`, `StudioTool`) and the
developers' environment variables (`STUDIO_*`); the local checkout's folder (`4cc-studio`)
keeps it until 1.0.0. A rename would buy a member nothing (the exe, the data folder, the
logs and the bundle already carry the product's names) and would touch the crate names'
268 lines in 129 files, colliding with every slice in flight. The repository is
`the4chancup/unlimited-tool-works` (GitHub redirects the old `4cc-studio` address), and
`Cargo.toml`'s `repository` names it. The docs spell the product's names where they
name what a member sees (the titles, the CLI's examples, the folders, the log); in their prose
and the code's comments "Studio", the working name, stays the suite's shorthand, as the crates
carry it, and the glossary says so. Rewriting the prose's ~250 "Studio" (the Studio format, a
Studio export) would buy a member nothing and set the docs apart from the code they describe.

**The icon** goes with the name: Archer, the reality marble's caster, from the
maintainer's picture, on a transparent background: the whole picture at 32 pixels and up, and
at 24 and 16 a tighter square on the head, above the outfit, so the smallest sizes show the
face alone. Three files under `resources/icon/` carry it:
- `icon.ico`, the six sizes (16, 24 tight; 32, 48, 64, 256 whole), the executable's file icon
  (Explorer, a shortcut, the bundle's exe before it runs). Windows draws only what an exe's
  resources hold there, so `studio`'s build script compiles it in on Windows with
  `winresource`, a build dependency, which calls the Windows SDK's `rc.exe`. Its resource
  also carries the exe's version details, whose file description is the name Task Manager
  lists the running program under and whose product name Explorer's Details tab shows: both
  say Unlimited Tool Works, not `winresource`'s default, the package's name `studio`. The
  build script spells the name as text, like the launchers, since it runs before any
  crate's constant exists.
- `small.png`, the tight crop at 64 pixels: the window's icon on Windows
  (`eframe::icon_data::from_png_bytes`). eframe makes both of the window's icons from this
  one image, resized: the small one (`ICON_SMALL`: the title bar, 16 pixels at 100% scaling,
  24 at 150%) and the big one (`ICON_BIG`, 32 and 48: the taskbar's). The title bar needs the
  tight crop; whether the taskbar's button, about 24 pixels at 100%, is better served by it or
  by the whole picture is checked on the built window. Giving the two icons different
  pictures would mean setting them past eframe, through winit's own calls on the first frame
  (a direct `winit` dependency, since eframe does not re-export it), which waits for that
  check to show the need. The title bar itself draws no icon: `studio_core` turns it off as
  the window opens, with `SetWindowThemeAttribute`'s `WTNCA_NODRAWICON` (one `unsafe` call
  through the `windows` crate, the window's handle from eframe's creation context), which
  leaves the icon to the taskbar and Alt+Tab. Windows 10 keeps the title where it stood,
  after the icon's slot, now blank (Windows 11 is not measured yet). Adding `WTNCA_NOSYSMENU` would move the title to the bar's
  edge but takes the window menu away (Alt+Space no longer opens it, measured on Windows
  10), so the slot stays blank and the menu stays. Not a title bar of our own (eframe's
  decorations off, egui drawing the title, the drag area and the caption buttons), which
  needs no `unsafe` but is far more code and loses the system's snapping and caption
  behaviour.
- `window.png`, the whole picture at 256: the window's icon elsewhere, where one image serves
  every size. On Linux that holds under X11; a Wayland compositor ignores a window's own icon
  and shows the one its desktop entry names (matched by the app id), which a Linux package
  installs. The preview's archive installs none, so on Wayland its window shows the
  compositor's default until a package does.

`scripts/make_icon.py` makes the three once from the picture, checking each of the ICO's
sizes holds its own crop, and they are committed, so a build needs no image tool.

### Phase 7: glTF support

Runs after Release 0.1.0 (see "Releases" above). Besides the codec, it lifts the Team compiler's
interim refusal of glTF sources (Phase 4) and enables the Export upgrader's `--convert-to-gltf`
pass (`export_upgrader.md` step 13).

- Add the `gltf` crate dependency to `model_convert` (Phase 2 built its native-format half)
- Implement `gltf_to_ir` and `ir_to_gltf` with a shared PES_bone/PES_mesh and materials.toml contract
- First settle the `metal` and `glass` Fox defaults and the `shaded` `Basic_CNSR` vectors that
  glTF sources make reachable (`spec/model_format.md` "Checked against the community's reference")
- Coordinate with the Blender project's `pes-models` PES glTF codec (its implementation lives there)
- Test with real Blender-exported models

**Verification:** glTF → FMDL → import in Blender → re-export → compare.

### Phase 8: GUI

The shell slice from Phase 3 (window, sidebar, one tool's view, a plain event log) exists by now;
this phase completes it. Its scenarios are written: the shell's under `gui.md` "Acceptance"
(`ST`), each tool's view under its plan's "Acceptance" (`TC-GUI`, `SE-GUI`, `EU-GUI`).

- Before any view or shell element is coded, its draft (`gui.md` "Prototypes before
  implementation"): the shell in the host page, `docs/prototypes/index.html`, first, then one
  script per tool as its view comes up (`tools/team_compiler.js`, `tools/save_editor.js`,
  `tools/export_upgrader.js`), each reviewed for
  usability by the lead first (ideas to `PROPOSALS.md`), then refined with the maintainer and
  kept as the view's specification; each tool plan says where its draft sits in that tool's
  build order
- Before the first tool's draft, one usability pass over what members already touch, built in
  Phases 1-6 (the export format and folder conventions, the message catalogs' texts and what
  each asks of the member, defaults, the handoffs from upgrade to compile to install), its
  ideas `PROPOSALS.md` entries ruled before the drafts are drawn around them
- Complete the `studio_core` app shell (sidebar, tool registry rendering, settings menu with per-tool
  collapsible sections, status bar with its three slots and static empty state, window title
  from activities and the held notice, help window with chapter tree, generated Messages topics,
  search, log-panel message links, and `UTW help`), and the first-run data-location dialog
  (`spec/core/distribution.md` "Data location: asked on first run")
- Build the common widgets: progress grid, log panel (auto-scroll, filters, grid cross-linking),
  run strip; and the console printer's progress line and run summary for CLI mode, drawn on
  stderr in a terminal only (`architecture.md` "Event system")
- Build `color_tools`' Phase 8 widgets (`plans/libs/color_tools.md`): the kit menu icon
  rendering (the grid's kit-cell miniatures, pattern-only mode) and the color-picker popup (the
  Save editor's team color fields; the Kit config editor and the Team creator later)
- Implement each tool's `view()`: Team compiler grid (live validation), save editor
  (player/team/tactics editing, AATF checks, comparator — build order in the [Save
  editor plan](../save_editor.md); the `session/` wiring only the view drives, undo,
  selection, recent files, the FPC toggle and AATF quick actions, lands with it; the tactics editor, card pickers and violations list are built as
  `libs/team_widgets`, which the Team creator later reuses), export upgrader
- Implement the exports folder watcher (`notify` + debouncing) and the check cache in
  `libs/pipeline` (`spec/libs/pipeline.md`; the design is `plans/team_compiler/gui.md` "Live
  validation"), and the Team compiler's `check/` scheduling its two-tier check over them
- Each view over the receivers its tool's `tick` drains (`gui.md` "Event wiring"): the grid,
  the log panel and the run strip read the tool's state, never a shell-owned channel
- Implement cancellation (`spec/core/gui.md` "Cancellation"): the Compile button's Cancel, the pipeline
  checking it between tasks and between a texture task's textures, the cancelled run's staging
  discarded
- The Team compiler's `run_pes` and sideload mode's Launch PES button as one launcher of
  `PES20{version}.exe` (`spec/team_compiler/pipeline.md` "Post-processing", Run PES): both start the
  same exe the same way, and this phase holds its only manual check
- Test on Windows (Linux is a Phase 16 item: no Linux machine is available, and 0.1.0 is
  Windows-only)

**Verification:** the acceptance scenarios above, the automated ones through the headless
harness (a frame's AccessKit nodes, clicks as press and release frames: `studio_core/src/shell/mod.rs`'s
tests are the model), the `manual` ones checked on Windows and recorded in the worklog; user
feedback from the modding community arrives through Release 0.1.0, which follows.

### Release 0.1.0

Cut once Phase 8 closes, as an early test release ("Releases" above: testers told of every
change in person, nothing a compatibility contract yet). It is a milestone rather than a
numbered phase: its deliverables are the slice of Phase 16 an early release cannot do without,
pulled forward.

**Ships:** the Team compiler (Phases 3–5: native FMDL/`.model` sources, CPU texture encoding,
compile-time aesthetics, written to the savefile through the aesthetics patch),
the Save editor (Phase 5 logic and the full Phase 8 view), and the
Export upgrader (Phase 6, structural migration; its glTF pass waits for Phase 7), as the Windows
portable bundle. Old-format exports reach the compiler through the upgrader; Red's Pre-Studio
preview layout gets no dedicated input path, since few members are expected to use it; an AET
referee export's per-referee `face/`, `boots/`, `gloves/` and `common/` subfolders reach it
through the upgrader too (`player_layout_proto`).

**Does not ship:** glTF (Phase 7), desktop GPU BC7, the Linux bundle, and the updater's
download/swap/rollback (all Phase 16), the tools of Phases 9–15, and what Release 0.2.0 brings
(the help chapters beyond the per-view topics, the update check, the teams-list merge).

- Release bundling as the `just release <version>` recipe, Windows bundle only (`distribution.md`
  "Distribution: portable .7z bundle", "Changelog and version display"): the `.7z` with a SHA256
  checksum asset, the release body copied from the version's `CHANGELOG.md` section
- `CHANGELOG.md` started, a line for every change a tester would notice. The help a tool
  carries is the topics its view's steps write (the Save editor's, the shell's core chapters)
- Phases 7 and 9–15 keep the phase procedure; each may ship as a `0.x` release

**Verification:** `just release 0.1.0` produces the bundle and checksum; a recorded manual check
on a clean Windows machine: the bundle extracted to a fresh folder, the first-run dialog, and a
fixture export compiled through `quick_compile.bat`. The release closes with converge and
rewrite over its own sections.

### Release 0.2.0

A later test release (or a later `0.x`; "Releases" above), with what a wide audience needs and a
tester told in person does not:

- Help chapters for the shipped tools beyond their views' topics (Red's two readmes migrated
  into the Team compiler and core chapters, "Updates" and "What's new"); `UTW help
  --export` wired into the release process
- The updater's check and notice (`distribution.md` "Self-update: in-place binary swap", step 1,
  and step 2 reduced to a notice): the release notes and a link to the release page, with *Skip
  this version* / *Remind me later*; the CLI one-line notice. Plus step 5, the teams-list
  reconciliation on the first start of a new version: it runs on a new binary's first start
  however the binary arrived, so a manual update needs it as much as the swap does. Until it
  ships, a new build does not replace a tester's working `data/teams_list.txt`, so a team added
  upstream is missing until the tester takes the new list by hand, which the release's notice
  says

**Verification:** the update notice shown against a test release with a higher tag, and the
teams-list merge's summary on a new binary's first start over a working list with a local row.

### Phase 9: Stadium compiler (`tools/stadium_compiler`)

- Implement the fox2 parser (+ CityHash64) as `libs/fox2`
- Implement the stadium DB bin generation
- Build the stadium pipeline on the shared reader/coordinator/writer scaffolding
- Implement components, billboards, packing, ID remapping
- Implement the tool's view (reuses the progress grid: rows = stadiums, cells = components)

**Verification:** fox2 roundtrip tests (byte-identical, matching FoxTool); output comparison against
the Python stadium compiler on real stadium exports.

### Phase 10: Music tools (`tools/music_player`, `tools/music_export_editor`)

The music tools are independent of phases 2–9 — they touch none of the game-file lib crates and need
only `studio_core` and the app shell, so this phase can run in parallel with (or before) the others
once the shell exists.

- Implement .4ccm parsing/writing + the condition/instruction model as `libs/music_export`
- Build `libs/audio_engine` (kira + symphonia playback, Opus decoder, ebur128 loudness, limiter)
- Music player tool: selection logic, soundboard view, normalization, chants, match state
- Music export editor tool: editing view, condition forms, live preview, validation, audition
  playback

**Verification:** .4ccm round-trip on the community export library; selection-logic scenario tests;
loudness within tolerance of the ffmpeg reference; manual streamer/manager testing.

### Phase 11: Match tracker (`tools/match_tracker`)

Needs only the shell for the tracker itself, plus Phase 10's music player for the autopilot half.

- Transcribe the PES17 memory signatures (team data table, stats table, stat catalog) as per-version
  data; implement the Win32 memory source and the acquisition ladder behind the memory-source trait
- Build the match model: stat diffing, event derivation, clock reconstruction
- Implement `libs/match_feed` (typed pub/sub + snapshot replay) and the tracker view (stats table,
  event log, match tabs)
- Match file save/load (JSON) + autosave; `watch` CLI
- Music player autopilot: feed subscription, auto team loading, feed-driven scoring, event clips

**Verification:** replay tests on recorded table snapshots (including ET/penalty matches);
clock-reconstruction unit tests; scripted-feed autopilot tests; a live simulated stream with
autopilot on.

### Phase 12: Kit config editor (`tools/kit_config_editor`)

Small: the format logic already exists in `libs/kit_config` from Phase 2/4 — this phase adds the GUI
form, the preview, and the container-editing mode.

- Editing form with constraint gating; export-folder tabs and loose-file mode
- Schematic + texture-composite preview (via `dds_convert`)
- UniformParameter container editing (via `uniparam`); `convert`/`check` CLI

**Verification:** bit-identical roundtrips over the bundled UniformParameter contents; byte-parity
edits against Kit Manager on the same file.

### Phase 13: Refs arranger (`tools/refs_arranger`)

Small: the export knowledge already lives in `libs/aesthetics_export` — this phase adds
the slot-allocation editor and the players.txt writer. Needs Phase 3 (`aesthetics_export`)
and Phase 8 (the shell).

- Slot appearance-rate tables as per-version constants (flat rates for PES 17–21, pattern table for
  PES 15/16)
- Slot-list editor with drag-and-drop from the referee storage box; read-only pattern preview with
  hover cross-linking (15/16 mode)
- Lists mode for PES 18–21 (the Fox referee hook decides who appears): one home slot per referee,
  ordered five-position lists built by hand or randomized, the weighted fallback fill of the
  leftover slots, `ref_lists.txt` writer
- players.txt writing (overwrite confirmation) + Team compiler handoff (`ToolContext` tool switch);
  `chances` and `randomize` CLI

**Verification:** players.txt round-trips through `aesthetics_export`'s roster reader and
writer, ref_lists.txt through the arranger's own; chance-math
unit tests against the measured tables; randomizer and fallback-fill invariants;
interrupted/observed atomic replacement tests (including Windows); a refs export arranged, saved,
and compiled by the Team compiler.

### Phase 14: Balls compiler (`tools/balls_compiler`)

Small: the heavy machinery (FPK/CPK packing, DDS↔FTEX, FMDL path rewriting, IR
conversion) exists as lib crates by this point. Needs Phase 2 (format libs and `model_convert`
for native conversion), Phase 7 (glTF support), and Phase 8 (the shell).

- Balls export parsing + validation (in-tool; the format has one consumer)
- `Ball.bin` writer + embedded `BallCondition.bin`/`ball.fpkd` templates
- Per-ball pipeline: model selection/conversion, texture conversion, FPK packing, thumbnails, CPK
  output
- List editor view (storage box + ordered list, thumbnails, utility buttons); `import-legacy` CLI

**Verification:** output comparison against 2.03 on the same library (accounting for the intentional
drops: no `name.txt`/`ball.fpk.xml` in the CPK, no texture the model does not name, and nothing a
stale 2.03 `patches_contents/` folder carried over from an earlier run); `Ball.bin` byte-parity for
identical lists, the expected file from a fresh 2.03 run (the `Ball.bin` shipped in its `Bin Files/`
is an older four-ball run's output, not a template);
interrupted/observed atomic `balls.txt` replacement tests (including Windows); shared `CpkStem`
boundary tests; in-game menu check.

### Phase 15: Player aesthetics editor (`tools/player_aesthetics_editor`)

Small and non-essential (a convenience tool — schedule freely once its
dependencies exist): Phase 2 (format libs, `model_convert`, `pes_savefile` settings and FPC
presets), Phase 3 (`aesthetics_export`), Phase 7 (glTF support), and Phase 8 (the shell). The dual-set superset merge lands here if not already built. The Blender-side
counterpart (the `pes-models` extension's loader module) is the Blender
project's deliverable, not this workspace's; the launch manifest is the
contract between them.

- Plain-folder player browser/launcher + manifest writing + Blender launch; archives require
  explicit extraction before launch, editing, or conversion
- settings.toml and ingame_face.toml editing panel (comment-preserving via `toml_edit`) + `fpc_on`/`fpc_off` marker and the in-game face toggle
  toggles; missing settings stay unset until the user edits them
- In-place glTF conversion (superset merge, app-data backup + restore); `launch`/`convert` CLI
- Base-model extraction from installed CPKs + bundled glTF stand-ins

**Verification:** golden-manifest tests; superset-merge field-provenance tests; transactional
conversion publication and backup/restore; comment preservation; compiled-output comparisons on both
engine targets against the chosen geometry and preserved compatible metadata. A dual-set merge
cannot retain both original geometries when they differ; verify that deliberate choice separately
from unintended conversion loss.

### Phase 16: Polish and distribution

Closes with the **1.0.0** release. Release 0.1.0 already delivered the Windows `just release`
recipe and `CHANGELOG.md`, Release 0.2.0 the first help chapters and the update check and notice
with the teams-list merge; this phase completes them and writes the maintenance-mode section.

- Release bundling: the Linux bundle joins the `just release <version>` recipe (see
  `distribution.md` "Distribution and updates", `distribution.md` "Changelog and version
  display"); Linux tested on a real machine
- Self-updater, the rest of it: the *Update now* option, download/verify/swap, `UTW
  update`, rollback button (see `distribution.md` "Distribution and updates")
- Desktop GPU BC7 on Phase 4's texture step (`libs/README.md` "First-release desktop GPU BC7"):
  cold pipeline creation and upload/readback measured first, then bounded batches, the Auto/CPU
  setting and the fallback report
- Documentation: every shipped tool's `help/` chapter written for members (a tool shipping in a
  `0.x` release from 0.2.0 on carries its chapter from that release; 0.1.0's carry their views'
  topics); the bundled readme stays a pointer (the
  window itself is Phase 8)
- Web guide: after the pre-release testing and before 1.0.0, a guide to the whole suite in web
  format, with as many visual elements as can be made (screenshots, annotated folder trees,
  diagrams), above all on preparing a Studio aesthetics export (player folders, subfolders,
  link files, shared folders, `Common/`, kit sets). The `help/` chapters stay the in-app
  reference; the guide is where a member learns the layout before opening the app, because
  the export's flexibility is what a new manager gets wrong first
- Maintenance mode: the `AGENTS.md` section on how the project changes once released is written
  here, for 1.0.0, the first release whose surfaces are compatibility contracts (the `0.x`
  releases go to testers told of every change in person; "Releases"), and not earlier: a
  procedure written far ahead of its use goes stale. Agreed outline: the plans stay the spec, edited in place, and `DECISIONS.md` stays the
  decision (ADR) log, split per year past about 4-5k lines, with no per-file ADRs or ADR
  tooling. A fix is a failing regression test first, a plan edit only if behavior changes, and
  `mutants-diff`. A feature runs the phase procedure at small scale (acceptance scenarios,
  converge, present-tense rewrite), as Phases 17-18 already do. Released surfaces become
  compatibility contracts (`settings.toml`/`config.toml` keys, CLI flags and exit codes, the
  export layout, finding codes, `.4ccm`): breaking one needs an Export upgrader migration or a
  clear refusal, never a silent reinterpretation. Converge runs per release over the commits
  since the last tag: the plan sections they touched, and `mutants-diff <last tag>`

### Phase 17: Team creator (`tools/team_creator`, post-release)

Not a first-release gate: its users are the *next* cup's new managers, and it depends on the
widest set of finished pieces — `libs/team_widgets` and `libs/aatf` with `apply_tier` (Phases 5
and 8), the compiler's kit placeholder (Phase 3, its UniColor entry Phase 4) and layout-marker rules (Phase 4), `aesthetics_export`
writing, and the Player aesthetics editor to hand off to (Phase 15). Plan: [Team
creator](../team_creator.md).

- The wizard (team, players, roles, cards, looks, kits, finish) and its help chapter written
  from the wiki's newcomer pages
- Roster file grammar; `legal_defaults` (positions from a stock formation, heights to bracket
  quota, weights, captain, set pieces) over `aatf::apply_tier`; stock formation constants
- The four embedded starter heads in Studio format (in-game head, two cardheads, boxhead)
- Export + Team TOML + README writer (temp sibling, renamed into place, never over an existing
  folder); `create` CLI; hand-offs to the four tools

**Verification:** `legal_defaults` output passes `libs/aatf` for every stock formation and both
height systems; a created export compiles on both engines with only the expected placeholders; the
Team TOML imports into a fixture save and passes the Save editor's AATF check; each starter head
verified in-game on PES 17 and PES 21; wizard and CLI produce identical files for identical inputs.

### Phase 18: Studio Web (post-release)

Not a desktop release gate. Ships once the desktop pipeline has settled, in two steps that match
the eligibility tiers in `spec/core/gui.md` "Browser deployment":

1. **Cheap tier**: the eframe `wasm32` build of the shell plus the file-picking tools (Save editor,
   Kit config editor, Music export editor, Export upgrader checks; the Ruleset editor once Phase 20
   has delivered it). Needs the async file-access
   adapter (`rfd` file picking + a File System Access API adapter for folders), browser-storage
   settings persistence, and hiding of the desktop-only tools. This step alone is the "open a
   bookmark and edit your team" demo.
2. **Pipeline tier**: single-team compilation. Needs the `libs/pipeline` browser variants (async
   reader/writer, async `MemoryBudget`, `wasm-bindgen-rayon` workers), the small in-flight limit, and
   COOP/COEP headers on the host.

Hosting on the community server (nginx, HTTPS, two headers) is evaluated at step 1; CI has been
`cargo check`ing the lib crates for `wasm32` since Phase 1 (guardrail 6), so this phase is UI
wiring and adapters, not porting.

**Verification:** the cheap-tier tools' file round-trips in Chromium and Firefox; a single-team
compile in Chromium producing the same CPK as the desktop CPU path on the same export; embedded
resource download size measured and, if it matters, lazy-fetched behind the template accessor.

### Phase 19: DB generator (`tools/db_generator`, `libs/pesdb`)

Small, and scheduled by need rather than by number: it depends only on Phase 2 (`teams_list`,
`cpk`, `pes_savefile`) and Phase 8 (the shell), and a cup start is when it is wanted. Until then
the Python scripts work. Plan: [DB generator](../db_generator.md).

- `libs/pesdb`: the six database tables the generator writes as typed per-version records
  (`Team`, `Player`, `PlayerAppearance`, `PlayerAssignment`, `Coach`, `CompetitionEntry`), the
  blank-table and Konami-table lists per version; `Ball`/`BallCondition` and `Stadium` join when
  Phases 14 and 9 need them (the Ball.bin writer planned inside the Balls compiler moves here)
- `pes_savefile::ops::populate`: the 19+ placeholder player section written through the codec
  (the scripts' hex-editor procedure), with the base player per version as a fixture
- The tool: teams list → tables (Backup/VGL/Invitational placeholder rows classified into their
  competitions), Konami tables copied from the configured install's data CPK, output as tree or
  CPK, the populated EDIT; `generate` CLI; the view

**Verification:** byte parity with the scripts' output on a three-team list per version (their
trees committed as fixtures); `pesdb` round trips on real Konami tables; populating the PES 20
fixture's stripped player section reproduces it byte for byte; in-game start with a generated
database and EDIT (manual, per version).

### Phase 20: Ruleset editor (`tools/ruleset_editor`, post-release)

Not a first-release gate: its users are invitational organizers, who use 4ccEditor forks until
then. Its remaining forms run after Release 0.1.0; 0.0.1 ships what it built while it was
pulled forward (the writer, the CLI, the window's frame and its Tiers section, worklog 20.3
to 20.6a2a) as "Ruleset editor (WIP)" ("Divegrass Before Christmas" above). Depends on `libs/aatf` with the schema and the
interpreter (Phase 5) and the shell (Phase 8).
Plan: [Ruleset editor](../ruleset_editor.md).

- `libs/aatf`'s writer (`Ruleset` → `RULESET` block, deterministic) and the schema-migration hook
- The tool: start page and templates, the section forms (tiers, heights, cards, universal rules,
  specials with the condition and effects editors, suggestions), opening and saving with logic
  identity, the rules summary, Randomize (Sensible / Weird / Crazy, seeded, every result with a
  legal witness team); `validate`, `update-logic`, `summary` and `random` CLI; the help chapter

**Verification:** as in the Ruleset editor plan's "Development phase": writer round trips on both
embedded rulesets, VGL26 rebuilt from Blank through the forms (manual), logic update and refusal,
no invalid ruleset ever written, every random ruleset's witnesses legal over 1,000 seeds per
preset.

### Unscheduled tools

The suite takes in every tool the community uses, those released after it as well, so an
outside tool that would talk to a Studio tool is planned into the suite and calls it
in-process; we do not give Studio tools inbound APIs for outside programs, because each one
is a server and a wire contract to keep for a consumer the suite is going to absorb anyway.
Tools the suite will take in, with no phase or plan yet:

- **Aesthetics ATF bot** (the maintainer's `Tools_Marco/4cc-aes-atf-bot`, Python; the Aesthetics
  ATF is the semi-manual check of a cup's aesthetics exports, `GLOSSARY.md` "ATF"): drives a
  running PES's Edit mode with a virtual Xbox 360 pad and records every player of each team of
  a teams list through OBS's WebSocket (a still, a dribbling clip, a corner-kick clip), so a
  cup's aesthetics are checked without playing through the rosters by hand. Its pad and OBS
  capture already drive the in-game check harness (`scripts/ingame/game.py` `pad` and
  `obs-shot`).
- **rigbot**: lets managers control their own team's tactics remotely during a match, and
  drives the Rigdio fork's local API (`music_player.md` "Autopilot": team loads, the anthems,
  the victory anthem, the chant, goals), passing SEN:P-AI `--no-rigdio` because it drives
  Rigdio itself. Not published: its author wants to work on it further before publishing the
  source (2026-10-18), so it waits; nothing scheduled depends on it, and until then what it does
  is known only from the forks' READMEs.

Not a tool of the suite but a way to run it, also with no phase yet:

- **A cup's submission server**, on the community's server (the wiki's host or `bonfire`): a
  script that runs the export checks on every export a manager submits and rejects it, with
  the findings, when they hold an Error; when the deadline hits, it compiles the DLC and the
  savefile and gives the organizer one download link per group of outputs (the DLC, the
  savefile, the pack of the teams' music exports, "Complete export" in
  `plans/aesthetics_export/player_folders.md`), some of which the organizer passes on to the
  streamer. It needs nothing the suite does not already aim for: the CLI runs every check and
  compile with no display, Linux is a first-class target, and findings carry stable codes
  that a script can sort. Keeping those three true is what keeps the server cheap when it
  comes; a design that only works with the GUI, or only on Windows, is a cost to it.

  After the compile it can also check the result in game: install the DLC and the savefile into
  the cup's PES version, run the Aesthetics ATF bot over every team (its stills and clips) and
  publish the captures on a page per team for managers and organizers. That half cannot run on
  the community server itself, which has no graphics card (`bonfire`: 8 virtual cores, 15 GB,
  a virtual display adapter): PES needs a real GPU on Windows, the bot's virtual pad a Windows
  driver (ViGEmBus) and its recording OBS. So it runs on a Windows machine with a GPU and that
  PES version installed, which takes the job from the server, plays it and uploads the
  captures; one machine plays one version at a time, and a whole cup's pass takes hours (the
  bot records each player's clips in real time). The pieces exist: the bot (PES 15 and 21 so
  far) and the in-game harness that already launches a version, puts a compiled CPK in its
  test slot and takes it out again (PES 17, 19 and 21), drives the pad and captures through OBS
  (`scripts/ingame/`); installing the savefile beside it, with the player's own save kept
  aside, is the step still missing.

---
