# Unlimited Tool Works: Save editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The save editor (`crates/tools/save_editor`) is the successor to **4ccEditor** and absorbs the
**Midcupping** scripts. It edits PES save files (player stats, appearance, teams, tactics),
enforces AATF rules, diffs saves (gameplay and aesthetics), handles team import/export (new
Team TOML format, legacy `.4ccs`/`.4cct` import, Texport read/write) and FPC invisibility.

It can also **generate `settings.toml` files from an existing savefile** (via `pes_savefile`'s
`PlayerSettings` model). This gives teams a migration path from the manual-editing workflow to
the [Team compiler](team_compiler/README.md)'s compile-time aesthetics (written to the savefile
through the aesthetics patch).

The savefile codec, the data model, and all interchange formats live in the `pes_savefile` lib
crate — see the [Savefile plan](pes_savefile/README.md). Platform context is in the
[core plan](core/README.md).

The porting stance: **full feature parity with 4ccEditor, improved wherever the
Win32 UI imposed friction** — but only with improvements that are actually
useful, never decoration. Concrete improvements are listed per section.

---

## Background: 4ccEditor

What exists of this is in `spec/save_editor.md` "Background: 4ccEditor".

| Component | Files | Size | Fate in the Rust port |
|-----------|-------|------|----------------------|
| Win32 UI | `main.cpp`, `window.cpp`, `resource.rc`, `menu_lists.cpp` | ~320KB | UI replaced by egui; retain the relevant domain tables in `menu_lists.cpp` as codec/conversion references |

---

## Feature inventory (the parity contract)

What exists of this is in `spec/save_editor.md` "Feature inventory (the parity contract)".

Everything 4ccEditor can do, with its fate in the port. This is the checklist the
implementation is measured against.

### File operations

| 4ccEditor | Fate |
|-----------|------|
| Load 15/16/17/18/19/20/21 EDIT file (7 menu items) | One **Open savefile** action; auto-detect the version by master keys / container shape. The loaded file's version governs its codec and editing widgets; warn on a global-selector mismatch, never reinterpret the open file. The Open menu lists the saves `pes_savefile`'s discovery finds under `Documents\KONAMI` for every version (labelled by version alone: the account folder's number tells a member nothing, and the entry's tooltip holds the path) as one-click entries, the first entry of each version with its 4ccEditor shortcut (Ctrl+5 to Ctrl+9 for PES 15 to 19, Ctrl+0 for 20, Ctrl+1 for 21), which opens that save from anywhere in the editor, or the file picker when discovery finds none for the version; Ctrl+O is the file picker's, with the files opened earlier in the session, above the ordinary file picker, and the picker opens in the folder of the selected version's discovered save when one is found (a folder with no EDIT file holds nothing to open, and from PES 19 the account folder is known only from a found save) — see "Savefile discovery" in the Savefile plan |
| Save EDIT file (encrypted; always asks for a path; the first save after a load keeps the file it replaces as `<file>.BACKUP`) | Kept; plus **Save as**, and the file each save replaces kept as `.bak` (one level), at every save rather than the first after a load only |
| Import Texport file (per-version submenu) | Kept, auto-detected version; plus **Texport export** (new — PES itself can import the result; see [Savefile plan](pes_savefile/README.md)) |

### Player operations

| 4ccEditor | Fate |
|-----------|------|
| Set all ability stats (dialog: every ability stat to N, 40 to 99, prefilled with the player's first; weak foot, form and injury untouched) | Kept, same fields and range; no edit flag set (the dialog writes the ability stats, as typing them does, and typing sets none) |
| Bump all ability stats ±N (dialog, Ctrl+B: N from -10 to 10, remembered; each ability stat clamped to 40 to 99; weak foot, form and injury untouched) | Kept, same, no edit flag set; N is read whole (4ccEditor reads two characters, so -10 bumps by -1) |
| Copy player stats (source PID → dest PID: height, weight, every ability, form, injury, weak foot, registered and playable positions, playing style, skills, COM styles, stronger foot and hand, star, playing attitude, the free-kick motion only, and the edited-player and gameplay edit flags) | Kept, the same field set but the free-kick motion, a look that stays the target's (the maintainer, 2026-11-02: it is a motion like the others); the other player is picked by team, then by roster slot (two combo boxes: one list of a cup save's 4,600 players cannot be browsed), and copied onto the selected one |
| Swap player stats (two PIDs; the same field set as Copy) | Kept, the other player picked as for Copy |
| Move a player up or down the list (the spinner beside it: one step per click, within one team; the two players' records trade places between their ids, `base_copy_id` traded back to match each slot, shirt numbers and the captain left with the slots) | Kept as drag reorder, with the same effect ("Editing UI", the Player list bullet) |
| Toggle FPC settings (Ctrl+I) | Kept (see "Full Player Customization" below) |
| Quick actions: Make Gold / Make Silver / Make Bronze (from the Autumn 26 edition) / Make Regular / Set stats | Kept, **driven by the AATF rules file** instead of hardcoded rates — the buttons and the checker can never disagree |
| Hidden shortcuts: PgUp/PgDn (previous/next player from any field) | Kept, the list's selection moving from team row to team row, or 7 rows with one team listed ("Editing UI" below, SE-GUI-53) |
| Hidden shortcuts: Ctrl+Shift+P (a joke preset), Ctrl+Shift+B (a sound), Ctrl+Shift+H (show the dribbling-motion and hand controls) | Not kept: every control is always drawn |
| Hidden shortcuts: Ctrl+O (the Open dialog), Ctrl+5 to Ctrl+1 (the Open dialog for PES 15 to 21, reading the file as that version) | Kept as the Open menu's: Ctrl+5 to Ctrl+1 open that version's discovered save at once, since Open detects the version and the dialog would only add a step; Ctrl+O opens the file picker (File operations row above) |
| Hidden shortcut: Ctrl+Shift+T on the team selector (empties the team's slots past 23; its team index looks one off) | Dropped: cups no longer trim a roster to 23 this way |
| Is a copy? (Appearance; ticking it also sets the ability, COM, motion, playable-position, registered-position, skill and playing-style edit flags) | Kept, the same flags set |

### Team operations

What exists of this is in `spec/save_editor.md` "Team operations".

Each operation below acts on the selected row's team, a player's or a team row's own (under
ALL too, as 4ccEditor's do), on every filled roster slot, as one undoable edit. With no row
selected the Team menu is disabled.

4ccEditor's Set player names to positions alone renames every row its list shows, which under
ALL is every player of the save. That is not kept, so no team operation reaches past one team.

| 4ccEditor | Fate |
|-----------|------|
| Clear visual settings flags (all players on team) | Kept |
| Set edited/created flag (all players on team) | Kept |
| Set / remove FPC invisibility (whole team) | Kept |
| Set boot/glove IDs for everyone (boots: unchanged, incremental or same for all; gloves: unchanged, like the boots, or same for all; incremental gives slot `k` the start ID + `k` − 1, slots 1 to 23) | Kept, every filled slot of the roster numbered by its slot (4ccEditor stops at the first id missing from the team, a bug not kept), in 4ccEditor's own layout, the maintainer's design: a group per side (`Boot IDs`, `Glove IDs`), its options stacked (`Unchanged`, `Set incremental value:`, `Set all to the same ID:`; `Set like boot IDs`), each option's field in one column across both groups, the incremental range's last ID beside its start, read-only and following it (the start plus 22, slot 23's), drawn as a flat box a shade off the panel with its number in `#a0a0a0`, the draft's (`#5f5f5f` on the light theme; egui's weak text grey on that box is as faded as a disabled widget's text, barely readable), the one-ID boxes as wide as the range, each group as wide as its options and fields and the window as wide as the groups; `Set` applies and closes, `Close` closes; a value past the field's range is an error line under the groups |
| Set player names to positions (each name becomes its registered position's full name, "Goalkeeper", and the edited-player flag is set) | Kept, same, and written: 4ccEditor renames the players in memory but never marks the records changed, so its Save writes none of it back (measured, the slice 2 parity run) |
| Load squad (`.4ccs`, with stats/aesthetics/tactics checkboxes) | Kept as read-only legacy import, same section checkboxes; Team TOML import is the primary path. Cross-version import applies the playstyle/skill conversion maps |

### Tactics operations

| 4ccEditor | Fate |
|-----------|------|
| Export / import tactics (`.4cct` "nightly" files) | Import kept (read-only legacy); export replaced by Team TOML (tactics are a section of it) |
| Copy / swap presets (1↔2↔3) | Kept |
| Copy / swap formations (kick-off / in-possession / out-of-possession) | Kept |
| Full tactics editing (see "Tactics editor" below) | Kept at full parity + interaction improvements |
| (none) | New, after 0.0.1: Save a picture of the formations, bare or with each preset's settings ("Tactics editor", "Formation pictures") |

### Database operations

What exists of this is in `spec/save_editor.md` "Database operations".

| 4ccEditor | Fate |
|-----------|------|
| Output rosters to TSV (`EDIT.tsv` in the working folder, no dialog, UTF-8; every team, hidden ones included, in roster order; per player: number, name, registered position, Finishing as the "rating", captain mark, height, playing style, skills, COM styles) | Kept, the same columns and label spellings, UTF-8 with a byte-order mark (4ccEditor's `ccs=UTF-8` writes one, and a spreadsheet reads accented names right by it); written where a save dialog says, `EDIT.tsv` its default name, the open save refused as the destination as the team file panel's exports and the Comparator's report export refuse it |
| Fix database (clear all visual flags, reset kit slots, PES17 kit-ID repair; a player's base-copy id cleared when it is neither his own nor 0 and his base-copy flag is off, the flag cleared when the id is his own or 0; every team's strip-edited flag cleared) | Kept as a maintenance action behind a confirmation dialog, under the aesthetics lock (it clears what Clear visual flags clears), one undo step, but for two parts. The base-copy repair runs only where the record has the flag (PES 15 to 18): on 19 to 21 4ccEditor tests a flag it never reads (an uninitialized field), and clearing copy ids there would change players' looks. The PES 17 kit-ID repair is not kept: its 2,500 writes step 131 bytes (`write_dataOld` advances its cursor 3 bytes, then the loop adds 0x80) over a strip table whose entries are 132 bytes (`spec/pes_savefile/model.md` "Kit slots and the strip table"), so all but the first land inside entries' bodies, and each writes 0xC0 into the strip-number byte; a repair of that table waits for its layout to be measured. The kit slots reset where the model has them (PES 17, the one version 4ccEditor writes them on) |
| (none) | New, after 0.0.1: Save formation pictures for a range of teams ("Tactics editor", "Formation pictures") |

---

## Crate layout

What exists of this is in `spec/save_editor.md` "Crate layout".

Planned files that do not exist yet:

- `src/session/undo.rs` — field-level undo/redo stack over model edits
- `src/session/selection.rs` — team/player selection, cross-team filter query
- `src/session/recent.rs` — discovered saves (`pes_savefile::discovery`) + recent files list
- `src/view/` — the view, in the pre-move tree's shape:
  - `mod.rs` (master-detail layout, open/save actions, unsaved-changes prompt)
  - `player_list.rs` (drag-to-reorder, dirty dots, colour-coded names)
  - `info_strip.rs` (General info and Other stats)
  - `quick_actions.rs` (the Make buttons and Set stats under the list)
  - the player view's other panels (`abilities.rs`: the positions, skills, COM styles and
    ability stats; `appearance/`)
  - the team view (`team.rs`, `tactics.rs`)
  - `panels/` (`aatf.rs`, `comparator.rs`, `transplant.rs`, `interchange.rs` — Team TOML /
    `.4ccs` / `.4cct` / Texport import-export dialogs)
  - `widgets.rs` (editor-local reusable widgets: version-gated control, ID picker). A color is
    drawn by how the save stores it:
    - An RGB one (the team colors, the only RGB fields a view draws) is `color_tools`' picker,
      `libs/color_tools.md`, not a widget of this crate. The host converts between the
      picker's 8 bits and the save's 6 per channel: a picked channel `c` is stored as
      `c * 63 / 255` rounded, a stored `v` shown as `v * 255 / 63` rounded (the Team
      compiler's widening of 5:6:5 colors), so a save always holds the nearest color it can.
      4ccEditor stores `ceil(c / 4.05)` and shows `floor(4.05 v)`, so a value can read one
      higher here (`v` 32: 130, where 4ccEditor shows 129). The difference is kept on
      purpose, since the stored value is what reaches the game.
    - A palette index (skin, iris, wrist tape, spectacles, gloves, hair accessory, as
      `settings.toml`'s `skin_color` 0-7) is a choice of its palette's entries, never a picker
      whose free color no record can hold.

Placement rules for the planned parts:

- **The views' fields map 1:1 to 4ccEditor's tabs**, which is the parity contract ("Editing
  UI"); a panel is a file of its own, or a function in its group's file, never a section
  grown inside another panel's.
- **`view/panels/` are the Midcupping-derived and new features**; each is self-contained and can be
  developed in the build order below without touching the views.
- **The tactics editor, the card pickers and the violations list are `libs/team_widgets`** (see
  below); the view and panel files here are hosts: they hand the widget the session's team, apply
  the edits it reports to the undo stack, and add what only this tool has (preset copy/swap,
  jump-to-player).

---

## Editing UI

The tool view (the panel right of the suite sidebar) is a master-detail layout after
4ccEditor's own window, without its tabs and its dialog sprawl. A player row selected shows the
player view, every field of the player at once; a team row shows the team view, the team's
fields and its tactics.

Not tabs, because they split one player's fields over pages a member flicked between to check
a build, and every page but the one shown hid what an edit on it depended on (the skills' card
budget counts the positions' extra A ratings). At 1280 points the pane holds them all side by
side.

```
┌──────────────────────┬────────────────────────────────────────────────────────────────┐
│ [team selector ▾]    │ 70101 QUADS ONLY                                               │
│ [filter 🔍]          │ General info         Skills  Gold: 6 of 8   Ability stats      │
│ 701 ■■ /3/           │ Name (10/61)         ☐ Heading  ☐ Man Mar…  Attacking Prowess 77│
│  ≡ 1 GK /3/ QUADS ON…│ [QUADS ONLY       ]  ☐ Flip Flap ☐ …        Ball Control     77│
│  ≡ 2 CB /3/ UNCANN… ●│ Shirt (11/18)        ⋮                      ⋮                  │
│  ⋮ (drag to reorder) │ [PLACEHOLDER      ]  Tricks, COM playing styles                │
│ 718 ■■ /f/           │ Height [189] Number [1]   ☐ Trickster ☐ Long Ball Expert       │
│  ⋮                   │ ⋮  Other stats, Playable positions                             │
│ [Make Gold][Make Silver]                                                              │
│ [Set stats] [77]     │ ▸ Appearance                                                    │
└──────────────────────┴────────────────────────────────────────────────────────────────┘
```

- **Team selector** with an "ALL" entry (as in 4ccEditor), plus a **filter box
  that searches across all teams** (name, shirt name, ID) — finding a player no
  longer requires knowing their team first. Matching uses the colour-code-free
  `display_name()` (see the pes_savefile plan's "Name colour codes"), a team's own name
  included. A team whose name matches lists all its players under its row: a search for a
  team shows its squad, not only the players whose own names happen to match.

  A double click on a team row sets the selector to that team and clears the filter, so the
  list holds that team alone ("show me this team"). The row's tooltip says so (`Double-click to
  filter to this team`), and the help topic on the list names it.

  Picking a different team selects its team row, so the detail shows that team's view, its
  tactics with it: a manager who picks a team usually wants its tactics next. Picking ALL, or
  the team already shown, keeps the selection (that team's row heads the list, a click away).
- **Player list**, 255 points wide (its rows' frame and the scroll bar's strip; egui's panel
  margins and separator come on top) and not resizable. Under the design width the window
  scales instead of narrowing (`spec/core/gui.md` "Shell layout", the paragraph "Scaling with
  the width"), so the view always has the room the 1280-point layout gives it, and a list
  dragged wider would only take that room back.

  Its scroll bar stands outside the list's frame, egui's solid bar (4 points from the content,
  6 wide), so a selected or hovered row's fill reaches the frame's edge and the bar covers none
  of a row's marks (their hover and click), whereas egui's floating bar lies over the content.
  The strip stays while a filter leaves nothing to scroll, so the list keeps its width. The
  controls above and below it (the team combo, the filter, the quick actions) end at the
  frame's right edge, not at the bar's, so the column reads as one box; Set stats and its box
  still fit at that width.

  A **team row** heads each team's players, set apart from a player row by the faint fill
  under it and its name in the strong text colour (no bold: the app installs no bold face, and
  a second font file for one row costs more than it shows). It shows the team's id, its
  colours 1-2 as swatches and its name, then `[click for gameplan]` in the weak colour at the
  normal weight (nothing else says where the tactics are; on a narrow row it is elided before
  the name is), and a dot when the team's own fields (name, short name, colours, ids, kit
  slots, tactics) have unsaved changes.

  A click on a team row selects the team, and the detail shows the team view. A team row never
  drags and is no drop target. ALL lists every team's row and players. Under a filter a team's
  row stays while its own name or any of its players matches.

  The team row replaces the team bar above the old tabs: the team is one click away from its
  players, where a manager looks for it, rather than a strip that took a row from every
  player. A save opens on ALL with the list's first player row selected.

  **Up and Down** move the selection to the list's previous or next row as shown (team rows
  included, so the detail turns to the team view on one), the list scrolling to keep it in
  view, the first and last rows stopping it. They do so while no control holds the keyboard (a
  focused number steps with them, a text field moves its cursor) and no dialog or popup is
  open.

  **PgUp and PgDn**, from any field (4ccEditor's hidden shortcut, which stepped one player),
  move the selection from team row to team row while the list holds two team rows or more
  (ALL, or a filter several teams match), one per press, PgUp from a player to his own team's
  row first. With one team row (a team picked) they move it 7 rows at a time. The first and
  last rows, or team rows, stop them. The team rows counted are the list's as filtered,
  wherever it is scrolled, so a press does the same whatever the window's height. A page of
  rows, as a list box's keys move, crossed into the next team at a place that depended on the
  window's height.

  A player row shows his place in the roster (`01` to `23`, the slot `NN` of his id, always
  two digits, so no column is kept for right-aligning a one-digit number), position and name,
  with no team name under ALL or a filter: the team row above its players already names it.
  The row shows the place, not the shirt number, because the place is the `NN` of his export's
  `NN - Name` folder and of his id, and General info shows the shirt number.

  The rows' text, as every name the views draw (the lineup lists, the popover, the view's
  title), is the body text, 12.5 as the drafts', not egui's 14-point default font, at which
  DMF and CMF overflow the position's column. The three are columns, as the draft's `.prow`:
  the place in the weak colour, the position in a 28-point column, the name after it, elided
  when too long with the dot and the mark kept after it, each 5 points from the last. A row (a
  team row too) is 18 high, its text centred in it, the rows touching with no gap between
  them, as the draft's; a selectable's height plus egui's 3-point item spacing made them 21.
  The row's drag handle `≡` stands 4 points from its edge and 4 before the place, as in the
  Tactics view's lineup lists, so the name gets the room the column's narrow width leaves.

  The list's **drag-and-drop reordering** replaces the up/down spinner, and moves what the
  spinner moves: the players' records trade places between the ids, each id staying in its
  roster slot. `base_copy_id` is traded so each matches its new slot, and the in-game face
  run's face structure and hairstyle stay too, since the spinner's record, the `.4ccs` one,
  holds of that run only the skin, iris and player gloves. The shirt numbers, held by the
  roster's slots, stay where they are, as does the captain mark.

  The roster's own order is never changed: by 4cc rule slot `NN` holds player
  `team id × 100 + NN` (`plans/aatf_rules.md` "The host"), and reordering the roster itself
  would break it. A drop several rows away is the spinner pressed that many times, so the
  players in between shift one slot along.

  A drop works on a row of the dragged player's own team, in one team's list or under ALL,
  where each team's rows run unbroken under its team row. A drop on another team's row does
  nothing, and drag is off while a filter is typed, where neighbouring rows are not one team's
  neighbouring slots. While a drag is held over a row of his team other than his own, that
  row, the slot the drop moves him to, is outlined in the selection colour (the draft's
  `.dragover`), as the lineup lists outline theirs. A row the drop moves nothing onto (his
  own, another team's, an empty slot's) gets no outline.

  The dragged row is drawn under the pointer while it is held, the point first pressed kept
  under it, as a dragged lineup row is (SE-GUI-65), and its place in the list is drawn as the
  lineup draws a dragged row's place. A draggable row shows the pointing hand wherever the
  pointer is on it (egui leaves a row that also takes clicks on the arrow, so the row asks for
  it). The lineup rows keep the move cross, which Windows draws for egui's grab hand.

  A dot marks players with unsaved changes. Names render with their savefile colour codes
  applied as text colours (the in-game look; 4ccEditor shows the raw control characters), and
  the name edit field keeps the raw string so codes survive a roundtrip untouched. A code's
  first six bytes are the RGB hex; its alpha is not drawn, and a code whose six are not hex
  leaves the text the normal color (the game does not check them: `spec/pes_savefile/codec.md`
  "Name colour codes, as measured").
- **The quick actions** under the list, in two columns: on the left the medal tiers' Make
  buttons (the tiers whose ids are `gold`, `silver` and `bronze`, in the ruleset's order: Make
  Gold, Make Silver, Make Bronze under the official one), on the right the other tiers' (Make
  Goalkeeper, Make Regular), then Set stats with its number beside it ("Configurable AATF
  rules" below says what they do).

  Every button and the number's box are 20 points tall (their own height, the draft's: the
  theme's controls are 18), the Set stats row in line with the buttons above it. The grid spans
  the list's frame, from its left edge to its right, as the draft's `1fr 1fr`: two halves 8
  points apart, egui's item spacing (118.5 each in the 245-point frame), each button as wide
  as its half, rows 3 apart. A label longer than its half is elided, its tooltip naming the
  tier. Columns as wide as their widest cell instead ended 50 points short of the frame, which
  the column's controls reach ("Player list" above), and "Make Goalkeeper", the official
  ruleset's widest, fits its half (105 of 118.5).

  They act on the selected player, so a team row leaves them disabled, their tooltip "The make
  buttons act on a player: pick a player row". The Player menu's entries are disabled too,
  their tooltip "The Player menu acts on a player: pick a player row".
- **The event logo.** While a save is open and the active ruleset carries a logo
  (`plans/aatf_rules.md` "The ruleset schema", the event's logo and page), the view draws it
  at its top right. With no save open it is not drawn, because it heads an open save's view
  and there is no view to head.

  It mirrors the sidebar's clover: its box is 28 points high as the clover is, its right edge
  14 points from the window's and its top 8 from the view's, as the clover's ink is from the
  left and top. The box is as wide as the picture's shape asks, up to 84 points (3:1, so a
  wide logo still reads); a picture wider than 3:1 is scaled to the 84 points and centred in
  the box's height (a 4:1 one is 84 × 21, 3.5 below the box's top).

  The picture is resampled once with Lanczos3 to the pixels its box takes at the window's
  scale, and again when the scale changes (the window moved to a screen of another scaling),
  not shrunk by the GPU as it is drawn: egui's linear sampling, with no mipmaps, skips source
  pixels past a halving, and DBC7's 78 × 84 logo drawn 28 high read pixelated at 100% scaling.
  It is resampled with its alpha premultiplied, not straight: a transparent pixel's color is
  whatever the file stored (black in nearly all of DBC7's 1,294), and a straight resample
  blends it into the edge as a dark fringe. The Ruleset editor's logo field draws it the same
  way.

  It is drawn over the toolbar row's empty end and moves nothing: no row is pushed down, no
  column narrowed (the drafts' toolbar at its widest ends 136 points short of it at the
  1280-wide window, and the right column's title starts under it). Where the toolbar reaches
  within 8 points of it (a narrow window) it is not drawn, rather than covering a control.

  With a page it is a link: the pointing-hand cursor, a click opening the page in the browser,
  and its tooltip the event (the ruleset's name), then a blank line, then "Click here to open
  the event's page:" and the address on the next line. Without one its tooltip names the event
  alone. One look on both themes: an event's logo is made for any background. The picture is
  decoded once per ruleset, not per frame; one that fails to decode is not drawn (its bytes
  passed the load check, its pixels did not).
- **The toolbar**, in egui's `MenuBar` (0.37), its style left as the toolbar's so the
  buttons keep their frames. The bar's pieces (the bar, its separator, a menu's button) are
  `studio_core`'s widgets rather than this tool's, so every tool's toolbar takes them when it
  is built (this is the first tool with a finished UI). A menu item with its shortcut is
  egui's own `Button::shortcut_text`, which every tool already shares through egui, not a
  `studio_core` wrapper that would only forward to it.

  Its contents, in order:
  - `Open`, `Save`, `Save as`; a separator;
  - `Undo`, `Redo`; a separator;
  - the `Player`, `Team` and `Savefile` menus; a separator;
  - the ruleset dropdown and Live AATF (with `Show suggestions` while it is on);
  - then `• unsaved changes`, in the link blue (`Visuals::hyperlink_color`, the blue of Live
    AATF's suggestion marks), not the weak grey, which hid the one piece saying the save
    differs from its file among the toolbar's quieter text; the blue also tells it is
    clickable (the changes list).

  With one menu open, the pointer entering another menu's button opens that one instead, as a
  menu bar does (`MenuBar` switches on hover; a plain button such as `Save` opens nothing).

  Menus group by what an entry acts on, not by what it opens:
  - the `Savefile` menu holds the whole save's work: the Comparator and Apply aesthetics patch
    first (each reads another file against the save, a second save or a Team compiler's
    patch), a separator, Output rosters to TSV, Save formation pictures for a range of teams
    (after 0.0.1) and Multi-team AATF, a separator, Fix database;
  - the `Team` menu ends with the team's files, Import / export team file (the interchange
    panel) and Save a picture of the formations (after 0.0.1).

  There is no Panels menu: that the Comparator, the patch and the interchange open as floating
  panels says nothing a member looks for them by.

  A floating panel first opens where the drafts' `.panel` stands: 620 points wide, its top 90
  below the window's and its right edge 1 inside the window's. That puts it over the views'
  right columns rather than the player list a member picks from while it is open (egui's own
  first place put the Comparator at the bottom left, over the list). After that it opens where
  the member leaves it, while the program runs. One floating panel is open at a time, as in
  the drafts: opening one closes the other, which would otherwise open on top of it.

  A menu item's shortcut is drawn right of its label in the weak colour
  (`Button::shortcut_text`): Bump all stats `Ctrl+B`, Toggle FPC settings `Ctrl+I`, Clear
  visual flags `Ctrl+F`.

  Set all stats and Bump all stats ask for their number on the sentence's own row, the
  field at the dialog's right edge, in line with Cancel under it, at least 12 points from
  the sentence (the draft's `.num-row`): a number field alone on the row under a sentence
  read as a second step. The field is the drafts' 52-point number box, as wide as the
  quick actions' and the boots and gloves dialog's, not egui's 40-point least widget width.
- **No view draws the tool's name** (`core/gui.md` "Window title": the title bar carries it),
  **nor the open save's path**: the tool's `title_detail` gives it to the idle window title,
  in square brackets, and the toolbar keeps `• unsaved changes` alone, which never takes a
  second row.

  Each view's first line titles what it shows, `<id> <name>`: the player view's `70101 QUADS
  ONLY`, the team view's `701 /3/`, the id in the weak colour, as the list draws ids, and the
  name with its colours.

  The title's ink stands as far below the toolbar's buttons as the view's first row's ink is
  below its own (about 11 points each on the maintainer's screen), not close under the
  buttons. The gaps are even in what the eye sees, not in boxes. A line box holds more room
  under the baseline than over the capitals, so equal box gaps of 7 read as 9 above and 11
  below. The gaps are evened in ink instead: the drafts set the title 2 lower, and the app,
  whose title sat low instead, takes a 6-point margin over it and room under it (11 and 11
  measured).
- **The player view**: three columns side by side, then the Appearance section. The room the
  fixed columns leave is split into four equal gaps: after the player list, between the
  columns, and after Ability stats (30.4 points each at the 1280 × 780 window). The first
  counts from the list's frame, 22 points before the pane (the list's 10-point bar strip and
  the 12 between), and the last runs to the window's edge, past the pane's own 12-point right
  margin. The eye reads the gap from the list's last ink and to the window's edge, and a view
  centred in the pane alone left the gap after the list wider than the gaps between the
  columns (37 against 27).

  The view's title line and Appearance start at General info's title. With General info's
  6-point inset below, its labels stand 35 points from the list's frame, room for Live AATF's
  marks left of them (at 36-point gaps centred in the pane they were clipped at its edge).

  At the 1280 × 780 window it fits the pane with no scrolling (the draft's check), and the pane
  is never narrower: a narrower window scales the whole UI down (`spec/core/gui.md` "Scaling
  with the width") and the player list keeps its 255 points, so the columns sit side by side
  at every size, each panel in its design's columns, with no wrap to keep.

  Column by column, each panel's title over its content and the panels of one column 14
  points apart:
  1. **General info**, its column 350 points wide: the title at the column's left edge, the
     fields and their labels 6 in from it, as the ability stats stand in from theirs (their
     cells' padding), so a panel's title heads its body. Other stats stands the same way.

     The fields are 344 points wide, the draft's width. That is room for the longest pair
     label, "Injury resistance", beside its box on each half, and for the label-left rows,
     339 points in the body font. They are no wider, so the three columns stay on one row
     beside the 255-point list.

     They are laid out as 4ccEditor's General Info:
     - Name, then Shirt, each a row with its label on the left and its field on the right.
     - Then label-left pairs on two halves 28 points apart (the first as wide as the
       label-left rows' label column, their 12-point gap and a box, so its boxes start at the
       x those rows' fields start at): Height and Shirt number, Weight and Nation, Age and
       Captain. Each box is 54 points wide: on the first half at the half's right edge, on the
       second just after a label column as wide as that half's longest label ("Shirt
       number"), so a short label (Nation, Form) keeps its box close. Captain's radio is
       centred in its box's width, under the Nation box above it.
     - Then Stronger foot and Stronger hand (20+), each a two-choice toggle, Left | Right, its
       chosen side filled in the quiet grey of the positions' C (`#5a5a5a`), not the
       selection's blue: a foot or a hand is rarely what a member looks for, and the accent
       would pull the eye to it. The chosen side's word is in the strong text colour and the
       other side's in the normal text grey, so the chosen side reads white (the maintainer,
       2026-11-03).
     - Then Registered position, a combo whose entries name each position in full after its
       abbreviation (`AMF - Attacking Midfielder`, British spelling: Centre Back; the TSV
       output and Set player names to positions keep 4ccEditor's names, Center Back, as their
       parity requires), and Playing style.

     Name, Shirt and these four rows share one label column, as wide as their longest label
     ("Registered position"), so every one of those fields starts at one x, and each ends
     flush with the second half's boxes, not on the panel's right edge. Within a block (Name
     and Shirt; the pairs; Stronger foot to Playing style) rows stand 22 apart, the form
     grids' pitch (`core/gui.md` "Other UI components", Form grids), the toggles 18 tall like
     every control. The blocks stand 6 apart, 2 more than their rows, as the draft's `.gen`,
     so each block reads as one.

     Under it **Other stats**: Injury resistance and Form, then "Weak Foot: Usage" and
     "Accuracy" (the two sharing one label), on General info's halves so every box of both
     panels lines up.

     Under that **Playable positions**: the 13 positions with C/B/A ratings in five rows by
     line of the pitch, each left wing to right wing (LWF CF SS RWF / LMF AMF RMF / CMF DMF /
     LB CB RB / GK, the midfield in two, GK alone below the others). Each row is centred in
     the panel, and each position's label centred over its ratings (over the B).

     The registered position is outlined, its corners rounded as the widgets' are (every
     frame and field in the view is rounded; the theme's own radius, not a second one). The
     chosen rating is filled, so the three tell apart at a glance and an unrated position
     still shows its C:
     - A a deep green (`#129648`) under a white letter,
     - B the selection's blue,
     - C a grey square.

     A click on a position's label makes it the registered position, the same undoable edit
     as General info's combo, where the eye already is when choosing among the ratings.

     A field's label in these panels and in Ability stats is drawn in the normal text colour,
     as the checkboxes' labels are, not the weak one: the weak grey read too dark beside them.
     A label and its field are never closer than 9 points on a pair and 12 in the label-left
     rows and Ability stats; the room this takes comes off the margins either side of the
     centred view.
  2. **Skills**, the Cards used badge in its header, the version's skills (up to 41,
     version-gated) but the active ruleset's tricks or free cards, as checkboxes in two columns
     filled column by column.

     Under it **Tricks**, the skills the ruleset's `tricks` card group names, in the game's
     order, two columns filled the same way: a member weighs a skill against the card limit,
     while the ruleset's free trick allowance usually covers a trick, so the two read apart. A
     ruleset with no `tricks` group (VTL's) shows **Free cards** there instead: every skill one
     of its `free_cards` effects names, those free only for a tier, a position or the captain
     included, so a card does not move between panels when a player's tier changes. Neither
     panel shows when the ruleset names no such card: every skill then stays under Skills.

     Under them **COM playing styles**, the 7 in two columns whose second starts at the
     skills' second's x. Every cell is one line of one height, a version-gated card's disabled
     one as tall as an enabled one, so each row of the two columns shares one y (the draft's
     disabled cells were 2 points taller, and its second column ended 24 points lower).

     A skill's, COM style's or playing style's name, here and wherever one is named (the
     playing-style combo, the AATF rows, the Comparator, the changes lists, the team file
     dialog's notes and the Ruleset editor's summary), is its id's title case read as an
     English title, from one function in `pes_savefile`:
     - GK in capitals;
     - `&` for and;
     - in, on, the and to in lower case after the first word;
     - and `No. 10`.

     So the names read `GK Long Throw`, `Cut Behind & Turn`, `Fox in the Box` and
     `Classic No. 10` (the drafts' `cardName`), where the bare title case drew
     `Gk Long Throw` and `Classic No 10`.
  3. **Ability stats**, ~25 in one column in the game's order, each label beside its number.
     Each stat is named as the save's PES version names it, from one per-version table in
     `pes_savefile` that every surface naming a stat reads (this view, the AATF rows, the
     Comparator and the changes lists), so a member reads the names his game shows. PES
     2017's names are read off its Edit screen: the ids' title case but Header for `heading`
     (the drafts' `statLabel`), and Form, Injury Resistance, Weak Foot Usage and Weak Foot
     Accuracy in title case too, on every version. The other versions' names (PES 2021's
     Offensive Awareness, PES 2016's Unwavering Balance) are gathered from each game after
     0.0.1, the title case standing for them until then.

     The labels are left-aligned in one column starting at one x and the numbers in another,
     as General info's are. A short label such as Jump then stands far from its number, which
     the maintainer prefers to the ragged left edge right-aligned labels make.

  The three columns' titles share one line: Skills' row is as tall as its Cards used badge, so
  it starts 3 points above the columns' top, and the columns start 3 points lower than a bare
  title would. Every panel title of the view stands 6 points above the first thing drawn under
  it (a field's frame, a checkbox), the badge's lower half overlapping that gap: the spacing of
  the team view's titles, so the two views read alike.

  Then **Appearance**, a collapsing section closed by default, below the columns (its contents
  below). Opening it makes the view taller than the pane, and the pane's scroll bar then
  floats over the view's right margin (egui 0.37's default `ScrollStyle::floating`, which
  takes no room), so nothing shifts when it appears and the view reserves no strip for it:
  the margin the centring leaves is room enough.

  The pane is one scroll area whatever row is picked, so its offset outlives a pick: choosing
  another player leaves the view where it was (clamped where the new view is shorter), and a
  member editing one part of Appearance player after player never scrolls back to it. A scroll
  area keyed by the player would reset it on every pick.
- **The view fills its pane**: the list and the quick actions under it take the pane's whole
  height, and the detail ends at the pane's bottom, with no margin below that would give the
  pane a scroll bar of its own.
- **Name and Shirt.** Each label carries a count in brackets, `Name (28/46)`, that follows the
  field as typed: the name's bytes against the open save's field, the shirt name's letters
  against the 18 every version shows (PES 16's 16), so a longer name a PES 20/21 save holds
  reads `(20/18)`.

  The count says how near its limit the field is, the limit being the smaller of the shown
  maximum and the portable one below:
  - in the grey of a disabled card's label (`#5a5a5a`) under it,
  - in the warning orange on it,
  - in the error red past it.

  The field and its count carry one hover: past the portable limit below, its warning; else
  what counts (on PES 21 "Up to 61 bytes. A plain letter takes one byte, an accented letter
  takes two or more, and a colour code takes ten. A colour code is typed as \x11c and eight hex
  digits.", "Up to 18 letters. Western European accents (é, ü, ñ) are fine, but other letters
  and symbols are not.").

  The save is where a member writes the names a tactical export carries to a cup of any
  version, so its Name and Shirt fields follow `pes_savefile`'s portable limits
  (`spec/pes_savefile/operations.md` "Team TOML"):
  - **Name**: the open save's own field (61 bytes on PES 20/21, 46 before), so `(10/61)` on a PES
    21 save (a colour code counts ten, an accented letter two or more). A name past 46
    bytes, which only a PES 20/21 save holds, draws its field with an amber tint, its count red,
    and the hover "In PES 2019 and older the game cuts this name to «…»", the name as
    `crop_player_text` would cut it. A tint of the theme's warning colour, not a pale yellow
    fill: the theme is dark and its text light, which a pale fill would wash out.
  - **Shirt**: 18 letters on every version but PES 16 (its field holds 16), so `(11/18)`. A
    shirt name the save already holds past 18 (PES 20/21 allow more) is kept, drawn with the
    same tint and the hover "PES 2019 and older show 18 letters of a shirt name (PES 2016 shows
    16)"; an edit may shorten it but not lengthen it, so it can be fixed a keystroke at a time.
  - Nothing more: a warning, never a block on Save, so editing a save stays frictionless.
  - What 4ccEditor's fields do besides storing, kept:
    - a name or shirt name edit sets the player's edited-player flag;
    - a shirt name, and the team's short name, are upper-cased as typed or pasted, whatever
      Shift and Caps Lock say (the game shows both in capitals);
    - Captain is a radio that makes this player's slot the team's captain and is not unticked
      by a second click (the captain belongs to the slot).

    4ccEditor's "Edited/created player" checkbox is not kept: the name and shirt name edits set
    the flag, and the Team menu's Set edited flag covers a team, which is what a member sets it
    for.
- **Contents 1:1 with 4ccEditor's tabs**, which are the parity contract, but for stronger
  hand. 4ccEditor draws it on its Appearance tab; the player view draws it in General info,
  beside the gameplay values it is one of. Star and playing attitude stay with Appearance, in
  its Cutscenes group.
  - **The numbers** are the game's, in 4ccEditor's ranges: abilities 40 to 99, form 1 to 8,
    injury resistance 1 to 3, weak foot usage and accuracy 1 to 4 (these four stored one
    lower), and the Cutscenes group's star 0 to 7 and playing attitude 0 to 3, each inside the
    field's bit width on every version. A value the save holds outside its range shows as
    stored and changes only when the member edits it: drawing a player never edits him (an egui
    number control clamps the value it draws unless told not to, which would mark him unsaved on
    sight).
  - **The cards' labels** (the skills', the Tricks or Free cards', the COM styles') and the
    Cards used badge's words are in the normal text colour, as the field labels (`Stronger
    foot`, the stats' `Ball Winning`), not the checkboxes' light grey: a card reads as a
    player's field, not a setting. A hovered card's label still brightens.
  - **The skills' Cards used badge** is the active ruleset's, the toolbar's ruleset dropdown's
    ("Configurable AATF rules"). It reads the player's own tier alone: the tier his stats,
    registered position and height match under that ruleset, as `check-aatf` recognizes it.
    It is not one badge per tier, which a member had to match to the player by eye. It reads
    `Cards used for Gold: 2 of 8`, the tier, the paid cards and the limit in a chip (the
    draft's `.badge`: the widget fill, corners rounded 2, 4 points of margin across and none
    down), with no `cards` after the count, which the label before it already says. When no
    tier matches it reads `Cards used: No tier matches`, the reason after the colon.

    A special's choice that offers cards (VGL 27's captain: one card, or a COM style, or an A
    position) counts its cards in the limit while it is untaken or taken by a card, so a
    captain at 4 reads `of 5` before he uses it; once a COM style or an A position takes the
    choice, the limit is 4 again. The check resolves a choice only when an option is needed,
    so a limit without it showed the bonus only past the limit (a VGL 27 tester's report,
    2026-11-02).

    A tier check that fails on the player shows `Tier check failed, click for more
    information.` in the error colour in the badge's place, its click opening a dialog titled
    `The tier check failed` with the error. The AATF check never runs the two functions the
    badge asks (`player_tier`, then `card_limits`), so a member's edit that breaks only them
    would otherwise show nowhere.

    That error names a roster place as the player list numbers it (`place 04`). The quick
    actions' refusal `Cannot apply the tier` names a tier by its label (`Gold`), not by the
    ruleset's id. An id no tier of the ruleset carries is quoted as the id, the only name it
    has.

    The badge is computed on a worker thread, as the pitch's medal letters are, once per edit,
    player or ruleset, a new run starting only when none is in flight. Its two script calls
    each read the whole team under every candidate height system (a team no quota system
    takes runs every candidate's checks), and take 50 to 70 ms apiece on the maintainer's
    PES 2017 save in a release build. On the UI thread they made each wheel notch over a stat
    box cost over 100 ms, three times that under CPU load. Until the new badge lands the same
    player's kept one stays drawn, and another player's or ruleset's draws nothing.
  - **Appearance**: first a **Cutscenes** group, star (19+) and playing attitude (20+),
    outside the aesthetics lock: they change only cutscenes, so they are no aesthetic the
    export owns. Then the ownership line, the switch and the locked groups, each group's
    labels in their own aligned cells, the draft's pairs per row (physique two, the strip's
    ids, enums and checkboxes four, motion two):
    - edit flags (face, hair, physique, strip, motion, base copy + copy-from ID),
    - physique sliders (14 values),
    - strip style (boots ID, gloves ID — any player's, 0 = normal hands — taping,
      spectacles, sleeves, inners, socks, undershorts, shirttail, the player-gloves checkbox
      with its gloves colour, the wrist tape L/R and spectacles colours),
    - motion (hunching, arm movement, kick motions, gc1/gc2, randomize; dribbling motion
      20+),
    - colours (skin, iris),
    - then the Full Player Customization group.

    Editable, as in 4ccEditor, unless the "Aesthetics from exports" setting is on — see
    "Read-only aesthetics" below.

    The motion fields' ranges are the version's measured ones (`pes_savefile`'s `motion_top`:
    a 1-based motion runs from 1 to its top plus one, a goal celebration from 0, OFF, to its
    top). **Randomize** sets every motion but the dribbling motion to a value drawn from 1 to
    its shown top, as one undoable edit. 4ccEditor's button (`main.cpp`, `IDB_MOTI_RAND`)
    also draws from 1 and leaves the dribbling motion alone, but caps by two fixed sets
    (PES 15 to 19: hunching 3, arm movement 8, corner kick 6, free kick 16, penalty 4,
    celebrations 122; PES 20+: 5, 10, 10, 20, 7, 162). These give a PES 15 player arm
    movements and celebrations that game does not have (its tops are 5 and 112) and never
    reach PES 18's and 19's last celebrations (127, 140); the measured tops do neither.
- **The team view**: `<id> <name>` as its title (above), in the body text's strong weight
  rather than a heading's size. Then one row holding:
  - the team's name (`Name`, a 140-point box: the title already says it is the team's, as
    `Short` names its short name),
  - short name, its box taking three characters (the game shows a three-letter abbreviation;
    every version stores it in 4 bytes, so a longer one typed would not reach the game as
    typed),
  - colours 1-2 (RGB, each colour's picker and its hex, labelled `Col. 1` and `Col. 2`),
  - manager ID (17+) and stadium ID,
  - and a **Kit slots…** button opening the kit slot assignments in a dialog (rarely edited,
    so not a panel that pushes the tactics down; a version that stores none says so in it).

  The ids share the fields' row, not one of their own, because the Tactics editor below needs
  the height: with a heading-size title and the ids on a second row its last row ended about 5
  points past the 1280 x 800 window's pane.

  The row's units stand 8 points apart, egui's item spacing, as in every row of the drafts
  (the Name and Short labels are units of their own, 8 from their boxes). A colour's label,
  picker and hex (the hex in the weak grey, the picker's swatch carrying the colour), or an
  id's label and its box, stand a space apart (3 points). The ids' grid cells are padded 4
  points on each side (so 12 before Manager ID, 8 between the ids), and each id's box is 46 wide
  (the draft's `.info-grid` number box; egui's is 40).

  A stroke line under the row sets the team's fields apart from the Tactics (the draft's
  `.sep`, 8 points from the boxes around it). It is drawn 11 points under the row's controls
  and 10.5 over the Tactics' first, since the draft's id cells pad their boxes and its Tactics
  bar is taller than its buttons.

  Then the Tactics editor (below), whose bench list shows the players only (12 for a 4cc
  team), not the 21 bench-order entries with 9 `(empty)`.
- Version-gated widgets render disabled with a tooltip ("PES 19+") rather than
  disappearing, so the UI stays spatially stable across versions.
- **Undo/redo** across all edits — the single biggest safety upgrade over
  4ccEditor. An undo entry holds the before and after of each model entry it changed (a
  `PlayerEntry`, a `TeamEntry`): the model is plain data, an entry is a few hundred bytes, and
  one mechanism serves every widget, since a widget edits a copy and reports only that it
  changed (`libs/team_widgets`' contract), not which field.

  One entry per gesture: changes from one widget merge into the open entry while a pointer
  button is held or a widget has keyboard focus, so a drag or a typed value is one undo step,
  not one per frame. A gesture ends when its widget loses the keyboard (a click on another
  control, a Tab), so a typed value and the click that follows are two steps even when one
  widget call reports both. Only a widget that takes typing (a text field, a number being
  typed) holds its gesture open while it keeps the keyboard. A button, checkbox, radio, rating
  cell or list row pressed with Space ends its gesture on that frame, so each press is its own
  step and Live AATF checks it without waiting for a Tab.

  A run of steps to one value (wheel notches, arrow presses, a stepper's clicks) merges into
  one entry: ten notches taking a stat from 60 to 70 undo in one press. The run ends:
  - at an edit of another value,
  - at Undo or Redo,
  - after a second with no step (a wheel spun in one go stays one entry; a value nudged again
    a while later is a new one),
  - and when the keyboard moves to another control, so a typed value and the click after it
    stay two entries.

  A run's value is what an entry's merge already keys on, the widget call that made the step,
  on one player or team. The Tactics tab's settings panel reports as one call, so a run there
  spans its sliders and toggles, which a member turning two of them within a second undoes
  together. Edits that are one step by themselves (Copy stats, an import, a dialog's Apply)
  never join a run.
- **Unsaved changes** are the difference from the file as opened or last saved, not the undo
  stack's length: undoing back to the saved state clears the dot. A player's dot covers his
  entry and his roster slot's shirt number.
- Keyboard shortcuts preserved where they earn their keep (Ctrl+S save, Ctrl+F
  clear visual flags, Ctrl+I FPC toggle). Ctrl+I acts on a player, so it does nothing on a
  team row, as the Player menu's entries are disabled there; Ctrl+F is a team operation and
  acts on the selected row's team.

  These, Ctrl+B, Undo and Redo (Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z) and Open's keys (Ctrl+O,
  Ctrl+5 to Ctrl+1) are held off only while a text field holds the keyboard (a number being
  typed is one: its own undo is the field's) or a dialog is open, as Ctrl+C and Ctrl+V are.
  Ctrl+S is held off by a dialog alone. A button, checkbox or rating cell reached with Tab
  keeps the keyboard after Space, and holding the shortcuts off there too left Undo dead until
  Escape, the Space's edit already settled.
- **Ctrl+C and Ctrl+V** copy and paste what was clicked last of three kinds, while no text
  field holds the keyboard (a text field keeps its own copy and paste):
  - a player row copies his stats and pastes onto the selected player what Copy stats
    writes, with its rules;
  - a team row copies the team's Gameplan and pastes onto the selected team what a
    Gameplan-only import of the copied team's document writes, players by roster slot;
  - on the Tactics view, the preset selector copies the shown preset and the formation
    selector the shown formation, each pasting onto the one shown.

  A list row means the selected row, which Up and Down move without a click, so Ctrl+V after
  Down pastes onto the next row. A paste of another kind does nothing; each paste is one undo
  step.

  The clipboard is the editor's own, not the system's: a copy carries a save's records as they
  were when copied, which another program cannot take, and opening another save empties it. A
  copy still puts one line naming what it copied on the system clipboard ("Unlimited Tool
  Works: the stats of SURNAME"), because the window hears Ctrl+V only as a paste of the system
  clipboard's text, and an empty one sends nothing.
- In a number field being typed, **Up and Down** step the value by one, as a wheel notch
  does; egui steps a whole-number field by its drag speed (a quarter) and rounds the sum
  back, so a press changed nothing, and the drag's sensitivity stays as it is.
- Closing, or opening another save, with unsaved changes prompts: save, discard or cancel,
  over a table of what would be saved or lost, columns Item, Old value and New value. The
  table spans the prompt's width, the two value columns sharing its right part. The prompt is
  as wide as its widest row, up to nine tenths of the window, each row on one line, so a
  renamed player's long name reads in full without a sideways scroll; only a row wider than
  that scrolls.

  The table has one row per changed field against the file as opened or last saved (the dots'
  baseline, so an edit undone back is not listed), the players' rows first, then the teams'
  own fields. A row is worded as the Comparator panel's are, in the views' terms: the item is
  the player's display name (or the team's name) and the field's label as the view draws it,
  the values as the view shows them. The table is computed when the prompt opens, not per
  frame; players through `pes_savefile`'s `compare_players`, teams through a team twin of it
  over the team, roster and tactics schemas.
- The toolbar's `• unsaved changes` is a clickable label (Sense::click, the pointing-hand
  cursor, tooltip "Lists the changes since the save was opened or last saved"). A click opens
  a popup under it holding the close prompt's table (the same rows, computed when the popup
  opens, the table scrolling past 240 points) and none of the prompt's answers. The popup is
  460 to 640 points wide, its whole box.

  The popup closes on a click outside it, Esc, a second click on the label, or any edit, undo
  or redo (its rows would otherwise go stale under it); the toolbar's Undo is a click outside,
  so the label reopens the list one row shorter. A popup, not a window: the list is a glance
  at what Save would write, read and dismissed, where the panels hold results a member works
  against. Its left edge is the label's, slid left when it would pass the window's edge.

  In the table, the prompt's and the popup's alike, a colour's old and new values each draw a
  12-point swatch before the hex, so the rows carry the colour typed rather than as text.
- Save with the save's own PES running asks first ("Writing a save while its game runs",
  below); another version's game asks nothing.
- A save of another version than the sidebar's selector opens with a Warning notice in the
  status bar naming both (SE-GUI-28); the save keeps its own version. The warning lives from
  the save's opening until the member has seen it, which is the first of:
  - its action, **Switch to PES 20xx** (the save's version), which makes the same change the
    sidebar's selector makes, its blockers included (while one holds, such as a running
    compile, the action is disabled with the selector's reason and the warning stays up, as
    the selector stays mismatched);
  - its dismiss;
  - or another notice taking the bar from it while it is shown (a notice arriving while the
    versions match, such as a save's, ends nothing: no warning was there to see).

  While it lives it follows the selector: shown whenever the selector names another version
  than the save's, its text naming the selector's (a save opened under its own version too,
  once the selector moves away), gone while the two match, back when they part again. Once it
  has ended, nothing shows until another save is opened. Its text is the draft's: "Although
  the sidebar is set to PES 2019, this save is from PES 2021, so the editor will keep treating
  it as a PES 2021 save."

  It is the editor's standing notice rather than a published one
  (`StudioTool::standing_notice`, `plans/core/gui.md` "Status bar"), so it shows in the Save
  editor's view only, and the shell tells the editor when it ended. A notice, not a banner
  under the toolbar: the status bar is where the shell's environment warnings already sit, and
  a banner pushed every view down a row. Not a switch on open: the selector also sets what the
  other tools target (the Team compiler's game), and a member opening an old save to look at it
  must not retarget them.

Improvements policy: each of the above exists to remove a real friction point
(finding players, reordering, fear of misclicks). No animations-for-the-sake-of-it.

---

## Writing a save while its game runs

The command line's half is built: `spec/save_editor.md` "CLI", the bullet "Writing a save",
and SE-CLI-03. What follows is the window's.

A save can be written while its own version's PES runs. PES writes the save it holds over
the file when the member leaves Edit mode after changing anything there, even a change
undone again (with nothing changed, leaving Edit mode leaves the file alone). So:
- a write made while Edit mode is open is lost then, unless Edit mode's **Load** (Data
  Management > Load) reads it in first;
- a write made with Edit mode closed stands, and opening Edit mode reads it in without
  restarting the game (seen on PES 2017; on PES 2021 the check used Load after reopening Edit
  mode).

The tool therefore tells the member, before writing, how to keep the write rather than
refusing. We do not refuse, as the first rule did on the belief that the game writes its save
back when it exits: that made a member close the whole game for every edit, which Load makes
unnecessary. A PES of another version asks nothing, since a game only writes its own
version's save.

- **The window.** Save, Save as and the unsaved-changes prompt's Save, with the save's own PES
  running, first show a dialog naming the game:
  - if its Edit mode is open, use Data Management > Load after saving, before leaving Edit
    mode, since leaving it after changing anything there writes the game's own copy over the
    save;
  - if Edit mode is closed, the changes show when it is opened.

  Its buttons are Save, which writes, and Cancel, which writes nothing (and, under the
  unsaved-changes prompt, keeps that prompt open).
- **The command line**, which cannot ask, takes `--edit-mode-closed`, the member's word that
  Edit mode is closed. That is the one state that keeps the write with no step in game after
  it, a step a run from a script or a launcher cannot remind the member of
  (`spec/save_editor.md` "CLI", "Writing a save").
- The Team compiler's savefile step follows the same rule (`spec/team_compiler/pipeline.md`
  "Savefile update").
- Scenario: SE-GUI-21 (the Acceptance section).

---

## Tactics editor

Full parity with 4ccEditor's Tactics tab, drawn in the team view under the team's fields:
- preset selector (1/2/3; the wheel steps it, and the formation selector, as it steps the
  style toggles, wrapping round at their ends) with fluid checkbox;
- the three sliders (support range, defensive line, compactness) and the two numbers in
  attack and defence (stored 1 to 3, `spec/pes_savefile/operations.md`, each a three-choice
  toggle, Few | Medium | Many, rather than a three-step slider whose number a member had to
  hover to read);
- formation settings toggles (attacking style, zone, buildup, positioning, defensive style,
  containment, pressure);
- advanced instructions (2 attack + 2 defense, with target player where applicable);
- set-piece takers (FK long/short/2, CK L/R, PK), captain;
- auto flags (substitution, offside trap, atk/def levels, preset change);
- the three formations per preset, starting eleven and bench order.

The interaction layer is rebuilt around direct manipulation (this is where the
Win32 original hurt the most):

- **The pitch is the editor**: an egui painter canvas draws the formation;
  players are **dragged** to reposition (x/y), with position labels and snapping.
  4ccEditor's per-slot dropdown + coordinate fields remain as a detail popover on
  click, not the primary interface.

  The popover's header is the starter's name and `player <n>` (his roster slot counted from
  1). Under it stand the position dropdown, with no label (its abbreviations say what it
  holds), and Make GK beside it at its own width, the two filling the popover's width. The
  popover's top-right corner sits on the marker's bottom-left corner as it stood at the click,
  and there it stays while his X and Y change, so the fields' arrows stay under the pointer
  rather than moving off with the marker. A press outside the popover closes it, as Close
  does.

  X (0 to 104) and Y (4 to 48, the outfielders' rows: below the drop's bullet, "A pitch player
  dropped on another"; 0 to 48 under a ruleset that allows illegal formations, "Illegal
  formations" below) are steppers: the number with an arrow either side, `⏴` stepping it down
  by one and `⏵` up, each click one edit, clamped into the range (a stored spot past it steps
  back in), each arrow disabled at its end of the range and both for the keeper.

  The arrows take clicks, never the keyboard's focus, as a system spinner's do: Tab goes from
  X to Y, and a focused number steps with the arrow keys already. A nudge by one is a click
  there, where a drag of egui's `DragValue`, which has no arrows of its own, overshoots it.
  The number and its two arrows are joined into one box as wide as the number box was (52
  points: each arrow 12, the number 28, room for `104`), so the popover is no wider
  (`slotPopover`'s `num`, the draft's `.spin`).

  The position dropdown lists only the positions his spot offers, as the game's Select
  Position menu does: the two of the drop rule below ("A pitch player dropped on another"), his
  own first when the spot offers neither (an odd save's). A change of his X or Y applies the
  same rule, as one step of a drag does, so a Y typed into another band changes his position
  with it. Under a ruleset that allows illegal formations the dropdown lists every outfield
  position and an X or Y change keeps his position ("Illegal formations" below).

  The editor is **one grid** of three columns (the settings, 422 points; the pitch, 320; the
  assignments, the rest) and three rows, so every panel lines up with its neighbours rather
  than wrapping on its own:
  1. The **preset bar** (Preset 1/2/3, Fluid formation, then its Copy/swap menu) level with
     the **formation bar** (Kick-off / In-possession / Out-of-possession and theirs), which
     spans the pitch's column and the next: the formations and their menu need more than the
     pitch's width.

     While the shown preset's Fluid formation is off (PES 2015 has no switch, so always), the
     formation selector is disabled and on Kick-off, its hover `Formations 2 and 3 are used
     only while Fluid formation is on`. The game fields the other two only while it is on, so
     editing them there edits nothing a match uses, and switching Fluid formation off moves
     the view back to Kick-off. While it is enabled, its hover is `The game uses formations 2
     and 3 only while the preset's fluid formation is on`, so a member editing the other two
     learns the rule before switching it off locks them.

     It locks even when the other two differ from Kick-off: the view is the manager's and
     copies the game's, and an organizer looking for formations hidden there finds them in the
     formation pictures ("Formation pictures" below, after 0.0.1), a quicker check across
     teams than the view. Their Copy/swap menu stays, a copy into them being how a member
     prepares them before switching it on.
  2. **Formation settings**, the pitch and **Player assignments**, the two panels exactly as
     tall as the pitch (386 points with its border) and level with it. The pitch's border, the
     lineup lists' frames and the player list's rows frame round their corners by 2 points,
     as every framed panel does (egui's group frame, the drafts' `.group`), the turf clipped
     to the border's inner curve.

     Formation settings runs row by row, each offensive setting beside its defensive pair
     (Attacking style beside Defensive style, Buildup beside Containment area, Attacking zone
     beside Pressure, Positioning beside Defensive line, Support range beside Compactness,
     Numbers in attack beside Numbers in defence: the original's arrangement). Every row is
     the same height, the panel's free height shared among them. Each label stands over its
     control, every control in a box 24 points tall, so a toggle's centre is level with a
     slider's and every label stands the same distance above its control. The settings column
     is as narrow as the widest toggle choice allows ("Frontline Press.", two such choices a
     half).

     Player assignments lists the set-piece takers, the captain and **Join the attack** as
     three rows, one per joining player, all three one picker (below). Each label stands 4
     points under the button above it, which leaves 3 of the panel's height free at its
     bottom. Player assignments and Advanced instructions start 6 points under their titles,
     as Formation settings does.
  3. **Advanced instructions** over **Automatic settings** in the settings column, beside the
     **starting eleven and bench** lists, which span the other two columns, each list half of
     them (twelve 18-point rows, padded 4 points top and bottom: the eleven with a row to spare,
     which the bench uses for its twelfth player), the two sides' bottoms level.

     A row's position (the formation's for a starter, the registered one on the bench) is
     drawn in its group's colour, the pitch badge's (below, "A marker"), so a list reads as the
     pitch does at a glance. A name is never drawn in its position's colour, but in the text
     colour, or in its own colour codes' colours (`names`, "`libs/team_widgets`" below).

     Advanced instructions is one grid: the headers Attack and Defence over their columns,
     each instruction's number (1, 2) beside its two combos, each target under its combo.
     Automatic settings is one grid of two columns of 24-point rows (Substitution beside
     Preset change, Offside trap beside Attack/defence levels), its panel grown to the lists'
     bottom.

     Substitution is a combo naming the game's choices. Measured in game: Off 0, Very late 1,
     Flexible 2 on PES 15, 16 and 21, Very early 3 on PES 21. PES 17's screen lists Off, Very
     late, Flexible and Very early, PES 21's four in PES 21's order, so its stored values are
     read as PES 21's and it offers the four. PES 18 to 20, between the two, offer the four
     too. That is taken rather than looked at: a look at each game's Substitution choices is
     still to come. A stored value outside them is shown as its number, so the combo never
     misreports the save.

  The grid is laid out at the design width. A narrower window scales the whole UI with its
  width (`spec/core/gui.md` "Scaling with the width") rather than rearranging it, so the grid
  has no narrow fallback. A shorter window scrolls the team view to reach its last row rather
  than squeezing it in.
- **The pitch looks like the selected game's own formation screen** (Edit, Teams, a team,
  Game Plan, Team Sheet/Edit Position), each starter shown as the game shows him, by his
  portrait, read from the selected game's `download/` CPKs when that folder is found and holds
  one.

  Each version draws its own screen's pitch (the open save's version, the game the member
  checks the formation in, even where the sidebar names another), since the versions' pitches
  differ (PES 2017's has seven bands to PES 2021's eleven, darker turf, a coarser grain and
  smaller boxes and circle) and a member checks a formation against his own game. Measured so
  far: PES 2017 (0.0.1's version) and PES 2021. PES 2019 comes with 0.0.2 (VGL 27's version),
  the others after it, and a version not yet measured draws PES 2021's.

  The reference is each screen's own pitch texture and the screen, which draws that texture at
  two thirds of its size (mean luma difference 1.7 of 255 to a 1280 x 720 capture of PES
  2021's, 2.2 to the maintainer's screenshot of PES 2017's). The texture is the 480 x 576 pitch
  with faint lines in the atlas of `common/menu/general/gamePlan.bin`: PES 2021's in
  `Data/dt11_x64.cpk`, which the 4cc patch does not replace; PES 2017's in
  `Data/dt11_win.cpk`, whose copy in the 4cc patch's `download/4cc_01_db.cpk` has the same
  pitch pixels.

  The canvas is drawn at that scale, 320 x 384 on every version (PES 2017's screen shows 382
  rows of it, its last two being the texture's padding). It is painted from these measurements
  rather than from the texture, which is Konami's and which a member without that game does
  not have:
  - **Turf**: horizontal bands, dark at both ends, with the texture's grain (a fixed noise
    of the given luma deviation), painted at the texture's size, 480 x 576, and drawn at two
    thirds with linear sampling as the screen draws the texture: the scaling softens the
    grain as the game's is (PES 2017's on screen: 4.4 on the maintainer's screenshot, 4.3
    predicted from its texture), where the same noise painted per canvas pixel looks
    coarser:

    | | PES 2017 | PES 2021 |
    |---|---|---|
    | Bands | 7: the inner five 53.3 tall, the top one 58, the bottom one 59.3; the middle one light, centred on the halfway line | 11 equal |
    | Dark, light | `#1f4b17`, `#2c5d24` | `#2d571d`, `#35602a` |
    | Grain | 6.8 | 3.5 |

  - **Lines**: white at 20% opacity, 2 px wide (canvas y and lengths to the line's centre);
    no touchlines and no spots (the textures have none). The halfway line; the centre
    circle; each penalty box, centred; each goal area, centred; each penalty arc, about the
    penalty spot, drawn outside the box:

    | | PES 2017 | PES 2021 |
    |---|---|---|
    | Goal lines, halfway line | canvas y 0 and 382, 191 | 0 and 384, 192 |
    | Centre circle radius | 29 | 37 |
    | Penalty box, wide by deep | 130 by 52.3 | 160 by 66 |
    | Goal area, wide by deep | 62 by 15.7 | 74 by 22 |
    | Penalty arc: radius, spot from the goal line | 28.5, 38 | 36, 44 |

  - **A slot's spot** is its marker's badge centre, at (12.8 + 2.827 x, 363.8 - 6.793 y) on
    the canvas: x 0 to 104 across the width, the halfway line (canvas y 192) at y 25.3, the
    attack end up. This is the screen's own mapping, fitted on the ten outfield slots of /3/
    (the 4chan Cup save) within half a pixel, and PES 2017's (twenty badges of two
    screenshots, each within 0.06 of a whole stored y). A drop snaps through the same
    mapping.

    The keeper's marker is drawn where the screen draws him, badge centre at canvas
    (159.5, 358.5) on both (PES 2021's goal-area line, 7.5 above PES 2017's), not through the
    mapping, which would put his stored spot 15 px higher. That stored spot is (52, 3) in
    every one of the PES 2021 save's 1,890 formations, and the editor never moves it.
  - **A marker**, top to bottom: the portrait, the whole 128 x 128 texture drawn 50 x 50
    standing on the badge (the cup's portraits carry their own padding and any ribbon); the
    badge, 50 x 14 under the 50-point portrait, `#242b2d`, corners 2, the position's
    abbreviation at its left in its group's colour (forwards `#ff2c77`, midfielders
    `#57e42b`, defenders `#00d2ff`, keeper `#ffde00`) and his overall at its right in white,
    both at 11.5 pt and 2 in from their ends as the game's ink is (measured on a 720p
    capture), a medal's outline round it (both below, "A starter's badge"). At 11.5 the
    widest pair, a three-letter position and an overall of 100 or more (`CMF 109`), measures
    44 and leaves about 2 between them; where the two come close their colours tell them
    apart. The badge is not widened for it: a wider badge stood off the portrait's edges.

    The name is drawn in its colour codes' colours (white where it has none: the screen draws
    them as its lists do), 13 px below the badge's centre, centred, or from the badge's left
    edge and cut to the badge's width when wider. A wider name scrolls as the game's does
    (measured on four names over 30 s captures): every 13.5 s it scrolls left at 40 px/s
    through one loop, its width plus a 16 px gap after which its start follows, so it ends
    where it began, and holds still the rest of the period. A name that fits never moves.

    A **Scroll names** checkbox right of the formations' Copy/swap, on by default and
    remembered in the tool's settings as Live AATF is, holds every wider name still at its
    badge's left when off (a view setting, not an edit: no undo entry, no unsaved dot). Each
    name's period starts when the pitch first shows it, still first. The game staggers names by
    a rule not established (two names 2.1 s apart); the editor's start together when the team
    view opens, until the game's delays are measured (a worklog step after 0.0.2).

    The captain's mark, a white rounded box with a black "C", 12 x 11, outlined 1 px in the
    badge's colour (`#242b2d`, as the game's: on a light portrait the white box would
    otherwise have no edge), sits 5 px above the badge over the portrait, its outline 1 px
    inside the badge's left edge (measured on a 720p capture).

    A starter whose portrait the editor cannot show, for any reason, shows the game's own
    placeholder in its place, drawn as a portrait. The reasons are none in the CPKs, one that
    does not decode, a folder, list or CPK that cannot be read, no PES folder set, or one still
    being read, which would otherwise draw nothing and then the portrait. The placeholder is the
    selected game's `common/render/symbol/blank/player-dummy.png`, a 128 x 128 PNG in one of
    three versions (PES 2015; PES 2016-2018; PES 2019-2021). The three are bundled
    (`resources/portraits/`), not read from the install, so the pitch draws them with no PES
    folder set or reachable.

    Markers are drawn from the top of the canvas down, a lower marker covering the one above
    as the game's do (the keeper's portrait over a centre-back's badge), the dragged one last;
    a press or a drop hits the marker drawn on top.
  - **Portraits** come from the selected game (the sidebar's PES and the "PES folder"
    setting): the CPKs its `download/DpFileList.bin` lists, a later CPK's portrait winning
    as in game, at the path the Team compiler writes portraits to (`player_<id>.dds` up to
    PES 2018, `<id>.dds` from PES 2019; a WESYS-wrapped DDS unwrapped). The list and the
    CPKs' tables are read, and the shown team's portraits decoded, off the UI thread.

    A folder, list or CPK that cannot be read leaves every marker with the placeholder, the
    reason logged. A listed CPK that is missing counts as one that cannot be read: the game
    then loads none of `download/`'s CPKs (the Team compiler's `dpfilelist_cpk_missing`), so
    portraits from the others would show what the game does not.

    The DpFileList reader is `cpk`'s and the portrait path `pes_savefile`'s, shared with the
    Team compiler, so the editor finds a portrait wherever the compiler put it.
  - **The screen's selection marks** are reused for the editor's states: the open popover's
    marker gets the selected player's corner brackets (`#14fbff`), and every marker the armed
    assignment mode can take gets the candidates' (`#ff4e7f`), each a 50 x 50 box of corners
    around the portrait. A join member while join mode is armed has its badge filled
    `#005c80`.
  - **The formation's name** sits in the pitch's bottom-left corner as the screen writes it
    (`3-3-1-3`): the ten outfield slots counted by the row of the playable positions grid
    their position belongs to, back to front, an empty row left out: LB CB RB, then DMF CMF,
    then LMF AMF RMF, then LWF CF SS RWF. The keeper never counts and the spot plays no part.
    Read off PES 2021's screen on four teams' eight formations (an AMF given CMF at his own
    spot turns `3-3-1-3` into `3-4-3`; DMF DMF CMF LMF RMF SS CF reads `3-3-2-2`, the wide
    midfielders with the AMF, not the CMF). The grid's rows and the name are one table, so the
    two cannot disagree. It names the shown formation of the shown preset: a slot's position
    change updates it, a drop's included (the pitch-drop bullet below).

    The label is white, its digits 15 pt tall at the pitch's scale, its ink 7 pt in from the
    pitch's left edge and its bottom 6 pt up from the bottom edge (measured on the 720p
    capture), drawn after the markers so a low marker never covers it.

    The typeface need not be the game's, but the stroke must match the game's label, about 3 px
    on its 15 px digits. The label is in the body font's semibold
    (`resources/fonts/RobotoFlex-Semibold.ttf`, Roboto Flex cut at weight 600 and the
    regular's optical size, SIL OFL 1.1 as the regular). That is a font family of its own (the
    semibold, then the body chain's fallbacks), at 20.5 pt (digits 15.2 pt tall), the text
    placed 6.5 pt in for the face's 0.6 pt left bearing.

    We use Semibold, not Bold, because egui's dark-mode text alpha (`2c - c²`) thickens
    strokes: drawn so, Selawik Semibold's verticals measured 2.97 px against the game's 3.01
    and Bold's 3.41. Roboto Flex's semibold stem is 3% wider than Selawik's, so about 3.05
    drawn (inferred from the outlines, not measured on a render). It is the one text in the
    suite not in the regular weight. The formation pictures (worklog 8.33, after 0.0.1) carry
    it in every cell.
  - **Set PES path**: while the sidebar's PES selector is orange or red (`plans/core/gui.md`
    "Live exe check": no folder, no PES exe in it, or another version's exe), a `Set PES
    path` button stands in the pitch's bottom-right corner, in the room the bottom penalty
    box leaves: centred between the box's right line (its outer edge) and the pitch's right
    edge, its bottom as far from the pitch's bottom edge as its sides are from those two.

    It is outlined 1 point inside its edge in the text's grey (`#8c8c8c`; a plain button's
    fill alone reads as a patch of the grass), and its side padding is 2 points, not a
    button's 4, so it fits PES 2018-2021's corner: their box is 160 points wide, PES 2017's
    130, which leaves 79 points beside it against 94. The margins are about 9 points on PES
    2017 and 1.5 on PES 2018-2021 (the label 72 points at the body font).

    Its tooltip reads `The players' portraits are read from the PES folder` (the selector's own
    names what the check found). The portraits come from the PES folder, and a member who
    never notices the selector's colour, or reads the readme, would see only placeholders and
    take them for the editor's best.

    The button shows on red too: a folder holding another version's exe gives the wrong
    game's portraits or none. It reads the selector's own check, never one of its own, so the
    two cannot disagree, and it is gone as soon as the check passes.

    It opens `rfd`'s native folder picker and stores the pick as the running-PES notice's Fix
    PES path stores its folder (`plans/core/gui.md` "Settings menu", one function):
    `pes_folder_path` with the version's year written `20**` when the folder's own name holds
    it, or the selected version's own row once any version has one. The exe check runs again
    and the portraits are read from the new folder.

    It is a widget placed over the pitch after the markers, as the formation's name is drawn
    after them, not painted on the pitch, so the formation pictures never carry it.
- **Lineup by drag-and-drop**: the starting XI and bench are reorderable lists;
  **dragging a bench player onto a pitch player swaps them** (and vice versa). While a
  drag is held over a row, that row, the one the drop replaces, is outlined in the
  selection colour (the draft's `.dragover`), so a member sees whom the drop swaps out.
  The arrow-button ordering UI is gone.
- **Click-to-assign roles**: "players to join attack" (and other
  pick-N-players settings) are set by clicking players on the pitch in an
  assignment mode — the three dropdowns are gone.
  - Join the attack shows its three players as three rows (`Unassigned` where fewer join), and
    any row arms the one join mode, all three highlighted, in which pitch clicks add and
    remove up to three.
  - Set-piece takers get the same treatment with a role palette.
  - So does an advanced instruction's target: its chip under the instruction arms a mode in
    which one pitch click writes the clicked starter and disarms, the chip enabled only while
    the instruction takes one of ours.

  No note explains the armed mode: the highlighted chip and the candidates' corner brackets on
  the pitch say it.
- Copy/swap preset and copy/swap formation are two small menus, on the preset bar and the
  formation bar, each acting on the selected preset or formation: a copy to, then a swap
  with, each of the other two ("Copy to 2", "Copy to 3", "Swap with 2", "Swap with 3" from
  preset 1; "Copy to In-possession" and so on for a formation), four entries rather than the
  nine pairs of 4ccEditor's menu tree (its 18-item menu is not kept).
- **A starter's badge shows his overall and his medal tier**, as the game's shows the
  overall. The overall is his per-position overall at his slot's position (the formula the
  pitch drop's rule uses, below), so a position change on the popover or a swap redraws it;
  it is exact on PES 2017 and within 2 of the game's on PES 2021 (108 of 201 read there
  equal), the precision the drop's paragraph states.

  A player the active ruleset recognizes as a medal tier (the tier ids `gold`, `silver`,
  `bronze` every ruleset shares) gets a 1.5-point outline inside the badge's edge in the
  tier's colour:
  - gold `#cc9900` (the tier's name colour),
  - silver `#c0c0c0`,
  - bronze `#8c4a20` (far darker than the metal's `#cd7f32`, which a thin outline let pass for
    gold: on an outline this thin the tiers' colours must differ plainly).

  Any other tier, a ruleset that does not load, or none, gets no outline. An outline, not the
  badge filled in the metal: on the three metals the position colours read at a contrast of
  1.0 to 2.4 and white at 1.8 to 3.1. Join mode's fill (`#005c80`) replaces the dark one under
  the outline while armed.

  The medals are read off the drawing thread (23 script calls, about 95 ms on a PES 2021
  team), so the outlines appear a moment after a team or ruleset is picked. An edit re-reads
  them too, since a tier may test where a player is fielded (`fielded_only_at`), but the
  badges keep the team's last outlines until the new reading lands: dropping them all
  meanwhile made every outline blink at each drag on the pitch.
- **Formation pictures.** Built after 0.0.1, which ships without them.

  **One team.** The Team menu's "Save a picture of the formations…" writes one PNG of the
  selected row's team through a dialog of the tool's own, not rfd's save-file dialog, since a
  native dialog cannot hold the checkbox the range dialog has too. The dialog holds:
  - the folder last used (the tool's `formation_pictures_folder` setting, which the range
    dialog shares) with Browse for rfd's folder picker;
  - the file name `<id> <name>.png` filled in (the name less the characters a file name
    cannot hold, `718 f.png`; the id first so a folder sorts as the teams list does);
  - and the checkbox "With each preset's settings beside its row", which adds or removes
    " settings" before the extension, so a team's two pictures do not replace each other.

  Labels and fields sit on one grid, every field starting at one x. A name already in the
  folder asks "Replace it?" first; the picture is then written and a success notice names it,
  with an Open folder action.

  **The picture.** The theme's panel colour under the team's name in its colour codes'
  colours, its id and the save's version (a folder of pictures from several saves would
  otherwise not say which game each came from). Then one row per preset, labelled "Preset n",
  with "Fluid On" or "Fluid Off" under it in the text colour (PES 2016 on), with or without the
  settings. Each cell is the pitch as the view paints it at its own 320 x 384, the formation's
  name in its corner, with none of the view's states (no drag, corners or join fill, a wider
  name held still at the badge's left).

  The columns are the three formations under their names. A preset with fluid formation off
  draws its Kick-off formation alone, the room beside it empty, unless its other two are not
  exact copies of Kick-off. Then it draws all three, as a fluid preset does, since a team can
  hide formations there and switch Fluid formation on in a match, and the picture is the
  organizers' check (PES 2015, with no switch, draws Kick-off alone). The other two are drawn
  at 60% opacity over the background (darker on the dark theme, paler on the light, receding
  either way), so they read as unused, perhaps left there forgotten by the manager, rather
  than as a fluid set. A team whose every row draws one formation gets one column and no
  column heads (a lone "Kick-off" would name a switch the team never makes).

  With the settings, a 240 point column at the grid's right lists each preset's settings level
  with its row, in the Tactics editor's order: the Formation settings' offensive then
  defensive half, the Advanced instructions (PES 2017 on). Each target stands on a line of its
  own under its instruction, at the value's x after `↳` (the symbols font's), where on the
  instruction's line after an arrow a name had only the room the instruction left.

  A label is in the text colour and its value strong at 120 points, values named as the
  editor's controls name them, full length. The column holds the widest fixed value
  (Attacking Full-Backs) with a few points to spare, and a target's name longer than that is
  cut with an ellipsis, rather than every picture widened for the few long names (up to 282
  points among the sample saves' players) and left with empty room on the right.

  The team's Automatic settings, which no preset owns, are one line under the grid. The pitch
  painter is one function the view and the picture share, taking the formation it paints.

  **A range of teams.** The Savefile menu's "Save formation pictures for a range of teams…"
  writes one such picture per team whose id lies between two teams picked in combo boxes on
  one row, `Teams`, the first, `to`, the last. It picks them as Multi-team AATF picks its range:
  - the save's teams by name as the player list's selector lists them, the save's first and
    last by default, not typed ids, which a member would have to look up;
  - the right end lists only the teams from the left one on and follows the left one when it
    passes it, so the range is never reversed.

  Under the combo boxes stand a count line ("4 teams"), the folder row with Browse, the same
  checkbox, and the file-name line, which follows it. The dialog is as wide as its longer
  state, the file-name line sized for both of the checkbox's, so it keeps its width when the
  checkbox flips, and its labels and fields sit on one grid. An existing file is replaced, the
  file-name line saying so.

  The dialog then turns into the run's progress ("Writing 2 of 4: /f/ (718)", a ProgressBar,
  Stop, which keeps the pictures written so far) and then its result ("4 pictures written to
  …", or "Stopped after 2 of 4 pictures; they are in …", Open folder and Close): the run is the
  tool's own, so it reports in its view, not the status bar. The teams' portraits are read
  before each picture, as the view reads them, so a cell never shows the placeholder where the
  view shows a face.

  **Drawing the picture.** egui's Painter draws to the screen only, so the pitch painter draws
  through a small target that both the view's `Painter` and an `image::RgbaImage` implement,
  text through `ab_glyph` (already in the tree through epaint) from the view's fonts, the
  symbols font after the body font for `↳`, rather than a software renderer (a new dependency)
  or screenshot crops (the view would flicker through every formation).
- Formation edits participate in undo/redo like everything else.

What the controls write, where the draft and 4ccEditor's tab (`tab_four_dlg_proc`) could differ:

- **An advanced instruction's list** is the version's stored table in its stored order (the
  game's own list), split by side as the game's Game Plan splits it: an Attack combo lists the
  attacking instructions, a Defence combo the defending ones (Swarm the Box, Deep Defensive
  Line, Gegenpress, Tight Marking, Counter Target, and Wing-Back from PES 2018; the Rigged
  Wiki's Tactical pages for PES 2017, 2019 and 2021, Konami's PES 2019 manual counting 9 and
  6; every version's stored table lists the attacking ones first). One list for both would
  offer a slot what the game cannot put there. The side is `pes_savefile`'s (`Instruction`,
  beside `takes_player`), the one place the rule lives.

  Each list is less Tight Marking: it marks an opponent, which a member sets in a match, and
  Edit mode does not offer it (4ccEditor's lists leave it out for that reason). A record that
  holds it, or an instruction of the other side, shows it as the current choice.

  Under a ruleset that allows illegal formations ("Illegal formations" below) the side split
  goes: every combo, Attack and Defence alike, lists every instruction of the version's table
  in stored order, less Tight Marking, since the Whack Patch's Edit mode puts any instruction
  on any slot (an attacking one in a Defence slot, one instruction in all four). The same
  instruction twice is never refused, under any ruleset.

  The target-player pick is enabled only for the instructions that take one of ours, Counter
  Target, Defensive and Anchoring (`pes_savefile`'s `Instruction::takes_player`, the one place
  the rule lives). What a change of instruction does to the target:
  - Picking one of those three from an instruction that takes none sets the target to none
    (255), so a byte the game never read as a target does not show up as one.
  - Picking between two of them keeps the target.
  - Picking an instruction that takes none puts back the target byte the file held when opened
    or last saved (the unsaved-changes baseline), not 255. The game reads no target there and
    writes its own byte (0 under 2,303 of the 2,445 unassigned entries of the maintainer's PES
    2017 save, 255 under 123), so a fixed 255 left a change no row could show after a round
    trip back to the instruction as opened ("No target -> No target").

  Under an instruction that takes none of ours the disabled target shows no player of ours:
  "An opponent" under a held Tight Marking (its byte is the opposing formation's slot), "No
  target" under the rest (the game reads no target there).
- **Players to join the attack stay packed**: removing one moves the later ones up, as
  4ccEditor's three combos keep them (an empty first entry clears the rest), so the record
  never holds a gap.
- **The goalkeeper's slot is fixed**: its marker is not dragged and its position, X and Y are
  not edited, as 4ccEditor locks them (the game's Edit mode does not move the keeper). Make GK
  swaps the selected starter with the keeper in the starting eleven only; the formation's
  slots stay.
- **A preset copy or swap moves the whole preset**, man marking included (4ccEditor's preset
  holds no man marking, so its copy leaves the target's): the game's own copy preset is how
  members bring a match's man marking into a team (DECISIONS 2026-10-12), so the copy carries
  it. A formation copy or swap moves the eleven slots' position, X and Y within the shown
  preset; the starting eleven is the team's and stays.
- **A pitch player dropped on another** swaps the two on the pitch: their two slots exchange
  position, X and Y in the shown formation of the shown preset alone, each player staying in
  his slot, as the game's drag does in Team Sheet/Edit Position (a save written after one
  changed only those two slots of that one formation). The keeper is never a target, his
  slot's position and spot being fixed. The game's other swap, selecting one starter and then
  the other, exchanges the two players in the starting eleven, so it reaches every formation of
  every preset; the editor's is an XI row dropped on another row or on a pitch marker, or a
  pitch marker dropped on an XI row.

  One released on the grass moves to the snapped drop point, his position set by the rule
  below, and one released off the pitch stays where he was. The snap holds an outfielder at y 4
  or above: rows 0 to 3 are the keeper's area alone (no other position's rectangle, below,
  reaches them), and the game keeps a held outfielder at y 4 however low the pointer goes (PES
  2017, checked in game; PES 2016's code tries the spot's neighbour y + 1, then leaves him
  where he was), so a marker dragged below it stands, and lands, on y 4.

  The game also changes the position of a dragged player (an AMF dragged lower becomes a DMF),
  but not by fixed areas of the pitch. The same spot gives different positions depending on
  who is dragged and where his teammates stand (a dragged LMF reads AMF at a spot where a
  dragged CMF reads LMF; the LMF turns CMF about 11.5 short of the nearest CMF, wherever that
  CMF stands), and the area the screen lights while he is held is not where his position
  changes. A rule fitted to scans would guess, and its wrong guesses would set positions
  silently, so the rule is read from the game's code: PES 2016's executable, the last one that
  is not packed, then checked in PES 2021, where it reproduces the four scans and the Select
  Position menu.

  Each position has an area of the pitch, its rectangle in each of PES 2016's two tables (the
  second widens the first to the pitch's edges, x 0 and 104 and y 48), a point inside either
  counting; CF's starts at stored y 40, confirmed in PES 2021. Below, "rectangle" means that
  area. A dropped point offers two positions:
  - the band's default (the first of CB, CMF, SS whose rectangle holds the point);
  - and the first free one of LB, RB, DMF, LMF, RMF, AMF, LWF, RWF, CF, CB, SS holding it,
    where a central position is always free and a wide one (LB, RB, LMF, RMF, LWF, RWF) only
    while no other slot of the formation holds it.

  He keeps his current position when it is one of the two (or the point offers neither);
  otherwise he takes the one his own per-position overall rates higher, the default on a tie.
  So a point inside his position's rectangle does not always keep it: a second LMF dragged
  within LMF's rectangle is offered CMF alone, the other LMF holding the wide one. His
  teammates' spots never enter it. A slot whose roster row has no overalls (an empty slot)
  keeps his position: no overall, no pick.

  The overall is PES 2016's formula on every version, no other version's being read. PES
  2021's differs from it by up to 2 (93 of 201 points read in game differ, 12 of them by 2),
  so where the two candidates' overalls are within 2 the editor's pick can differ from the
  game's.

  The formula is `pes_savefile`'s, on its player model (a player's abilities and positions and
  a position in, the overall out): twenty abilities weighted per position, the position's
  bonus (registered, A, B; C none), and at the registered position, unless it is GK, a blend
  with the first nineteen abilities' mean through a lookup table, clamped to 40-109. The two
  keeper terms are each the mean of a few keeper abilities. PES 2015 stores no clearing,
  reflexes or coverage, so its means take the ones it stores rather than counting the missing
  ones as 0, which would sink every PES 2015 keeper (one keeper ability at 0 takes a 70 keeper
  to the floor of 40).

  Its weights, bonuses and lookup are PES 2016's tables, a measurement hand-written by the
  lead. Which ability each weight reads is inferred from its profile (the code reads them
  unnamed), and the two alternatives tried (kicking power and swerve swapped, physical contact
  for body control) change no overall read in PES 2017 or 2021. One function serves the
  drop's rule and the badge's number; the host hands `team_widgets` each starter's thirteen
  per-position overalls in his `RosterRow`, since the widgets read no player record.

  PES 2017, the game DBC7 plays, follows both exactly: ten rectangle edges crossed by six
  players dragged in its Lineup/Edit Position (CF's y 40 among them) fall where PES 2016's
  tables put them, and PES 2016's formula gives every one of 150 overalls read there (23
  players, nine positions, registered, A and C), so its drag is the rule as it stands.

  A dragged lineup row is drawn with the point first pressed under the pointer, as the browser
  draws the draft's (egui alone would centre it on the pointer; SE-GUI-65). A lineup row
  released on the grass does nothing, as in the draft: a row moves players between the lists,
  never on the pitch.

  While a dragged starter is held over another, that one carries the selected player's corner
  brackets (`#14fbff`), drawn over everything, and the dragged marker goes to 35% opacity over
  him, so a drag meant only to move a player shows the swap before the release and can be
  steered off it onto the grass. The dragged marker's own spot is never the target: the drop
  hits the marker drawn on top among the others, so the mark and the release always agree.
- **A held starter** moves a whole stored unit at a time (2.8 points across, 6.8 down),
  standing where a release would put him, and the point of his marker first pressed stays
  under the pointer: the press's offset from his badge centre is kept for the whole drag, so
  a press on his portrait or name does not jump his badge onto the pointer.

  At each step his badge shows the position a release there gives him and his overall at it:
  the drop rule above at his spot, from the position he had when picked up, never from the one
  shown a step earlier. So moving him back restores it (a CF dragged down to AMF and back up is
  a CF again, as in PES 2017; ruled from the step before, he passed SS's rows on the way back
  and stayed an SS). Over a teammate the badge shows the teammate's position, which a release
  there gives him (the game's badge does the same). The release writes the spot and that
  position, one undoable edit, and only a release commits one: a CF released as an AMF and
  dragged back up is a CF again where his overall rates CF over SS, while one released in
  SS's rows stays an SS in CF's area, SS being offered there.

  While he is held, the pitch outside the areas of the positions he is rated A or B at is
  darkened to 40% (the turf and its lines; the markers are drawn over it), as the game lights
  it. Each lit area is its position's rectangle in the first table, its edges half a unit out
  from its rows through the spot mapping, a row at the table's edge (x 5 or 99, y 47) reaching
  the canvas' edge. The darkening is painted as the cells the lit edges cut the canvas into,
  each filled once when no lit rectangle holds it, so overlapping rectangles (a band's and a
  smaller area inside it) never darken twice.

  These are the PES 2021 screen's edges within 1.5 points but for CF's lower one, which the
  screen draws at y 40.7 where the table's row starts at 40. The editor draws the table's, y
  39.5, where a CF stops being offered, since the screen's own geometry for that edge is not
  read. The lit area says where he is rated, not where his position changes: the rule decides
  that, from every position's area and his overalls.
- **Illegal formations**: a cup that plays a PES 2017 exe patched to keep any formation (the
  Whack Patch; 4cc Autumn 26 and DBC7) lets its members field what the game's own drag cannot
  make, a CB outside his position's area or an LMF up among the forwards. Its ruleset says so
  (`universal.illegal_formations`, `plans/aatf_rules.md`). Under the active ruleset's flag the
  pitch sets a starter's spot and his position apart, as 4ccEditor-1's Tactics tab does (a
  drag sets X and Y alone, `scale_static_proc`'s `WM_MOUSEMOVE` and `WM_LBUTTONUP`; its
  dropdown, `tab_four_dlg_proc`'s `IDC_TACT_PLPOS`, the position alone):
  - A held starter's badge shows his own position and his overall at it at every step on the
    grass, over a teammate too.
  - A release on the grass writes his spot alone, over a teammate as anywhere else: the
    patched game keeps two starters on one spot or a unit apart (worklog cv14g), so a drop on
    a teammate lands beside or on him, with no corner brackets or fading while held there.
    Only that marker-on-marker swap goes: two starters are still exchanged by dropping one XI
    row on the other, or a marker on an XI row, and a bench row dropped on a marker still
    swaps him in, as with the flag off.
  - The snap holds an outfielder anywhere in the stored rows, 0 to 48, the keeper's rows
    included, as 4ccEditor-1's fields allow.
  - The popover's dropdown lists every outfield position, whatever his spot, and its X and Y
    keep his position.

  The keeper's slot stays fixed and Make GK unchanged. The darkening while he is held stays,
  since where he is rated is still worth seeing when his position no longer follows it.

  With the flag off (every other ruleset) the pitch keeps the game's rule above: a member of a
  cup on the stock exe cannot make an illegal formation by accident, which is why the flag is
  the cup's, in its ruleset, and not a setting each member would have to find. Switching the
  ruleset changes nothing stored, only how the next edit behaves; the host hands
  `team_widgets` the flag with the tactics' other inputs, the widgets reading no ruleset.

  The patched game keeps an outfielder below y 4: a save holding CBs at y 0 and y 2, two CMFs
  on one spot and two SSs a unit apart showed them so in Edit mode's Team Sheet, and the save
  the game wrote afterwards held the same values.
- **A starter swapped with a bench player** hands the bench player every set-piece role, the
  joining of the attack and every preset's instruction targets the starter held; the captain
  stays who he was, as in 4ccEditor.

Version quirks (advanced-instruction ranges differ 17 vs 18+) are enforced by the
`pes_savefile` model, not the UI. A `.4cct` imports into any version `TeamToml::apply`'s
version rule admits (`spec/pes_savefile/operations.md` "Team TOML"), not only into 16-18 as
4ccEditor's did.

## `libs/team_widgets`

The tactics editor above, the skill/COM/playstyle pickers of the player view, and the AATF
violations list are not this tool's alone: the [Team creator](team_creator.md) shows the same
formation, cards and instructions to a new manager over a team that does not exist in any save
yet. Tool crates never depend on tool crates (core plan, guardrail 1), so the widgets live in a
lib — `libs/team_widgets`, an egui widget crate in the class the core plan already allows for
`color_tools` and `model_viewport`: egui-dependent, `wasm32`-clean, and nothing else but
`studio_core`'s shared widgets (the checkbox every view draws), so its controls look as the
host's do.

Contract, the same one the color picker has. A widget takes:
- `&mut` model (`pes_savefile`'s `TeamTactics` for the tactics and pitch widgets, a player's
  card flags or playing style for the card pickers);
- the host's `RosterRow`s where it names a roster slot (each slot's name, position, portrait,
  medal and overalls, read by the host so the widget never reads a player record);
- the version's schema for gating;
- and the AATF rules where it shows limits.

It draws, and it returns what changed. It never sees a session, an undo stack, a file, or a
savefile byte. The host decides what an edit means: this editor pushes it onto the undo stack,
the creator writes it into its draft.

Contents:

- `pitch` — the drag-and-drop formation canvas, position labels and snapping, the per-slot detail
  popover, lineup and bench lists with swap-by-drag.
- `tactics` — preset selector, sliders, style toggles, advanced instructions (with target-player
  pick), set-piece role palette, auto flags, click-to-assign modes.
- `cards` — skill, COM-style and playstyle pickers, version-gated, with one remaining-count badge:
  the player's own tier as the host's AATF rules file reads him (`player_tier`, then that tier's
  `card_limits`), or `No tier matches`; the host computes both and passes the one badge, so the
  widget never recognizes a tier itself. The playing styles are listed in the order the
  version stores them, which is the game's own Edit-mode list, not the canonical order the
  Team TOML names them in: a member compares the combo against the game's list.
- `violations` — the AATF results list grouped per player, emitting a "selected" event for the
  host's jump-to that names the row's player and its violation (the host lands a
  `FieldedNotA` one on the team row, its preset shown).

  The groups lead with the team's own rows, then the players' by slot (`check_team` returns
  the team's rows last), each group's errors before its warnings; a team header heads each
  team's rows. The heads read:
  - the team header `Team <id> <name>` while editing (a report's `<id> <name>`);
  - a group's head `Player <id> - <number> - <name>` (`Team-wide issues` for the team's own).

  `Player <id>` and the hyphens are in the weak colour. The number and name, the team header
  and `Team-wide issues` are in the strong text, every name in its colour codes as the player
  list draws it, the number in the body font: the grey id steps back, and the name reads as
  the list shows it. Suggestions are drawn only when the host asks, in their own group after
  the violations.

  A row is the rule's name in words and a sentence saying what is wrong, each stat, card,
  playing style and allowance in it named as the game names it (Attacking Prowess, never the
  rules file's `attacking_prowess`; `FieldedNotA` is a rules file's word, never the member's).
  The names and sentences are the widget's, one per `ViolationKind`, and `check-aatf`'s lines
  print the same ones, so the window and the command line word a violation alike: the
  rules-file spellings are for the program alone, since managers and rule makers both read
  the game's names. The exact names and sentences are drafted first, in the draft's
  `violationSentence` (`plans/aatf_rules.md` "The interpreter", its paragraph on violations in
  words).

  A `check-aatf` line is the `aatf_violation` finding with the context `team`, `player` (for a
  player's row), `rule` (the name in words) and `problem` (the sentence), where it printed the
  kind and each field. A height row's players (a bracket's, the pool's draws) are lines under
  it, wrapping in the plain text colour (`plans/aatf_rules.md` "The height findings name their
  players").
- `names` — a savefile name drawn with its colour codes applied as text colours, the in-game
  look, for the player list, the view titles and the lineup rows alike.
- `event_logo` — a ruleset's event logo decoded into a texture (through `dds_convert`'s PNG
  decoder, the one the portraits use) and its size, fitted proportions kept in the box 28
  high and up to 84 wide. The Save editor's top-right logo and the Ruleset editor's General
  field both draw it, so the two show it alike; each host decodes it once per logo, not per
  frame. Here, not in `studio_core`, which holds what runs with no tool installed, and not
  once per host, which would let the two sizes drift.

Stock-formation constants stay in the Team creator (its only consumer) until this editor wants
an "apply stock formation" action, at which point they move here.

---

## Full Player Customization (FPC)

FPC is the 4cc system for fielding **FBMs (Full Body Models)**, custom models that replace
the player's entire body; the system and its presets are specified with `libs/fpc`
([spec](../spec/libs/fpc.md)).

The editor's side of this, ported from `fpc.cpp`, is a per-player toggle and a team-wide
on/off that apply `libs/fpc`'s FPC enable/disable presets (their fields are listed in the
[libs/fpc spec](../spec/libs/fpc.md)). The strip settings that break or bend hiding (inners,
undershorts, taping, the gloves checkbox) are reported by its interference rules in the
Appearance section, not cleared. The same presets serve the
[Team compiler's](team_compiler/README.md) per-player-folder `fpc_on`/`fpc_off` marker
files, so the editor's toggle and the compiler's markers cannot drift apart.

- **The hide IDs are a common setting.** Any boots or gloves ID the game has no model for
  hides the body; 55 and 11 are the 4cc convention, not a game fact. So the two IDs are
  common settings (`fpc_hide_boots_id`, `fpc_hide_gloves_id`, defaults 55 and 11; core plan
  "Settings menu"), and `libs/fpc`'s hide preset takes them as an `fpc::HideIds` value rather
  than reading constants.

  The editor's Hide default body, Show default body, Ctrl+I, the team's Set and Remove FPC
  invisibility and the classifier (`is_fpc_player`) read the setting, and so do the
  compiler's `fpc_on` marker and its default-ID rule. One setting, not one per tool: a
  compiler writing 60 for a marker and an editor reading only 55 as hidden would show every
  compiled FPC player as "The default body is currently shown".

  A hide ID is valid in the stock band only, boots 1 to 100 and gloves 11 to 100: above 100
  the compiler allocates its per-team blocks (an ID there would be another player's real
  model), and gloves 1 to 10 are the goalkeepers' real gloves. Whether an ID in the band
  exists in a given game is the member's knowledge, as it is for 55 today.

  Changing the setting does not rewrite a save: a player hidden with the old ID reads "The
  default body is currently shown" afterwards, and Show default body leaves his ID alone (it is
  no longer the hide ID); the help says so.

- **A model's own IDs are kept.** The wiki's hide list reads "Boots 55 (or any other
  nonexistent boots ID), or your FBM's custom boots if it uses that", and the compiler keeps a
  `settings.toml`'s own `boots_id` over the marker's (TC-BIN-29). So the editor writes a
  preset's boots or gloves ID only where the record holds 0 or the hide preset's ID, and keeps
  any other: hiding turns 0 into 55 and keeps 627, showing turns 55 into 0 and keeps 627
  (55 standing for the configured hide ID, above).

  4ccEditor's `enable_fpc_invis_for_player` and `disable_fpc_invis_for_player` do the same.
  On PES 15 to 17 they also set the Custom skin when hiding. Hide default body does not,
  since the wiki's full hide sets no skin and the Custom skin is its partial hide's setting
  (both tools' Show default body give such a player the Light skin back, so the two
  converge).
- **Hide and Show, not a checkbox.** A player hidden by his model's own boots ID reads as not
  hidden to `is_fpc_player`, which sees only the save (the hide ID or a custom skin),
  so a checkbox drawn from it would stay unticked after a click and could never show him
  again. The Appearance section draws the classifier's reading ("The default body is currently
  hidden" or "The default body is currently shown") with two actions, Hide default body and
  Show default body. Ctrl+I shows a player the classifier reads as hidden and hides any other,
  and the Team menu's Set and Remove FPC invisibility are the same two actions over the team.
- **The findings are catalog codes.** `fpc::check`'s five codes are rows of the tool's message
  catalog (`fpc_inners_break_hiding`, `fpc_undershorts_break_hiding`, `fpc_taping_shows_pieces`,
  `fpc_gloves_checkbox_winter_gloves`, `fpc_custom_skin_on_non_fpc`), so the section's text and the
  help's Messages topic are one; they join the spec's Messages table when this phase closes.
  The last never fires here: the save's classifier counts a custom skin as hidden, so only the
  compiler, which classifies by the folder's marker, can meet it.

---

## Read-only aesthetics

A player's aesthetics are owned by the team's export: `settings.toml` and the models decide them.
On every engine the Team compiler resolves them into an **aesthetics patch** (format in
"Aesthetics patch" of the [Savefile plan](pes_savefile/operations.md)), and the save editor
applies the patch. Hand-editing the same fields here would create the two-sources problem —
the next patch silently overwrites the hand edit, or the hand edit silently diverges from the
DLC.

That holds for a cup whose looks this suite's Team compiler builds. A cup whose save is the
looks' home (DBC7: PES 17, compiled with the old tooling) edits them in the save as 4ccEditor
did. So the lock is a setting of the Save editor's section, **"Aesthetics from exports"**
(`aesthetics_from_exports`), **off by default**. Off, every aesthetics path below edits as
4ccEditor's did, with no ownership line and no switch drawn. On, the editor **does not edit
aesthetics** but through the switch:

- The Appearance section renders its values below the Cutscenes group disabled, with one
  line of explanation ("The team's export owns the aesthetics, so edit its `settings.toml`
  (the face, hair and iris: its `ingame_face.toml`) and recompile, then apply the patch", and
  a link to the apply action).
- The same lock covers every other aesthetics write path: the team operations that write
  appearance fields (set boots/gloves IDs for everyone, set/remove FPC for the team, clear visual
  flags, the FPC toggle), Fix database (it clears the same flags), the aesthetics section of
  Team TOML and legacy squad imports (the section checkbox is disabled, gameplay and tactics
  import as before).

  Copy and Swap stats need no lock: they never move a look, the free-kick motion included. A
  roster drag is not a look's edit: it moves a whole player to another slot, his looks with
  him, and the next patch writes each id's looks from its export.
- **Apply aesthetics patch** is the one aesthetics write open by default:
  1. open a patch;
  2. the version must match the save and the allocation scheme the suite's (refused
     otherwise, save untouched);
  3. the comparator's preview lists every player the patch will change with old → new;
  4. applied through the undo pipeline;
  5. saved only when the user saves.

  A team the save lacks is reported and skipped; the rest applies.

  The panel (the Savefile menu's entry, and the ownership line's link) holds `Open patch…`,
  then the file's name with the teams it writes and their players, and `Apply`. A refusal is
  the CLI's sentence (`<path>: the patch is for PES 2021, the save is PES 2017`) in the error
  colour; each skipped team or player is a warning line (`Team 714 /co/ is not in this save:
  its players are skipped.`).

  The preview is the open save as it stands against the patch applied to a copy of it, laid
  out as the unsaved-changes table: the columns Item, Old value and New value, the item the
  player's display name and the field's label in the views' terms (`TAKUMI · Free kick`), the
  values in monospace, the item's click selecting the player (its tooltip `Click to switch to
  this player`).

  The preview is recomputed when the save changes (an edit once it has settled, an undo,
  another save opened), not frozen at Open. A drag or a text edit in flight keeps the last
  preview drawn, since recomputing it every frame would slow the edit, and Apply recomputes a
  preview so held before it writes, never a list an edit since made stale. So after Apply the
  list is empty and `Applied. The changes stay unsaved until Save.` stands in its place until
  the next change. A patch whose values the save already holds lists nothing and says so, and
  Apply is disabled whenever the list is empty. An undo of the Apply brings the list back.

  Patches apply cleanly in sequence — a midcup patch carries only the teams it recompiled and
  only the fields their compile resolved.
- **Unlock: "Edit aesthetics anyway"**, a switch in the Appearance section, drawn only while the
  setting is on, off at every start of the tool. Turning it on re-enables all the paths above
  for the session, with the note that a later patch overwrites whatever is edited by hand.

  It exists for the cases the rule does not cover — a team with no export, a cup-night fix when
  recompiling is not an option — and it is a session switch rather than a second setting so
  that a machine set to "Aesthetics from exports" cannot quietly stay unlocked.

  The setting is off by default because the cup the suite serves first (DBC7) keeps its looks
  in the save. Its row in the settings menu carries the tooltip "On when the cup's looks are
  compiled from exports: the save's aesthetics are then read-only, behind Edit aesthetics
  anyway in the Appearance section."

Gameplay data (stats, skills, positions), team data and tactics are unaffected: they were never
the compiler's, and the savefile builder edits them as before.

---

## Comparator (gameplay + aesthetics)

What exists of this is in `spec/save_editor.md` "Comparator (gameplay + aesthetics)".

The panel over the spec's `diff` report:

- Load a second save (same version); the panel lists per-team, per-player
  differences.
- Filter toggles: All / Gameplay / Aesthetics; a team filter; export the diff as
  text.
- Clicking a diff row jumps to that player in the editor; its tooltip says so, `Click to
  switch to this player`.
- A player's rows stand under his head, `Player <id> - <number> - <name>` in the AATF lists'
  colours (the `violations` widget's paragraph): one head for a player wherever a list names
  him.
- A row is drawn in the views' terms: the field under the label its view draws, its value as
  that view shows it (a free-kick motion 15, where the save stores 14).

  A field no view shows is named in words from the report's name with its stored value (the
  draft's `fieldTerm` and `fieldWords`):
  - its words in sentence case;
  - Id as ID, Com as COM (the views' "COM playing styles") and Color as Colour;
  - a PES 2015-only field marked so;
  - the face's unnamed bits "Other face data".

  So `NoseHeight` is "Nose height", `Color1Red` "Colour 1 red" and `Pes15JawDepth` "Jaw depth
  (PES 2015)". The same words label the unsaved-changes prompt's, the toolbar's changes
  list's and the aesthetics patch preview's rows.

  The changes list and the prompt also list team and tactics rows, which the Comparator never
  shows (its rows are `diff`'s per-player differences), in the same words:
  - a roster slot as the player list numbers it (`Player(3)` is "Player 04", `Number(3)`
    "Player 04's number");
  - a man-marking byte, the marked opponent's formation slot, as `Opponent's starter 5`
    (numbered from 1; 255 reads `None`) on the row of the own starter who marks
    (`Preset 1 · Man marking · <his name>`): the bare byte read as one of the team's own
    players.

  A slot's number row the prompt lists under its player reads `Shirt number` instead, as the
  strip labels it, since the item already names him.

  Export writes the CLI's report unchanged, stored values included. That report's text names
  each field as the views label it (Header on PES 2017, where the save's schema says
  `Heading`), while `--json` keeps the schema's name, a key a script can match that no
  rewording moves. Not the report's values on screen, because a member reads a row against the
  view it jumps to, and a motion one lower than the view's reads as a wrong row. Not view
  values in the report either, because the report is what the compare scripts it replaces
  print.
- The picked save is the report's first and the open save its second, so a row reads picked,
  then open: what the member's edits changed against the save he compares with.

  A row stands in three columns, Item, Old value and New value, named by a head row above the
  list, as the unsaved changes list names its own (not one `old → new` text: values of every
  length then start at one place, and the eye runs down a column). Item is 190 points wide
  and Old value 200, each cut with an ellipsis where its text is longer, and New value takes
  the rest. A row that cuts a value shows the whole row's text on hover, above the hint of
  what its click does. The team and player heads are as they were.
- Export writes the report of the toggled scope over the whole save, the text `save-editor diff
  <picked> <open>` prints with `--gameplay` or `--aesthetics` (All: neither); the team filter
  narrows the screen only. Not the rows shown, because the command has no team filter, and an
  Export no command writes could not be checked against the CLI.
- The views' terms (a field's label, its stored-to-shown transform) are read from the views'
  own tables, not from a second table for the panel, which would drift from the views; the
  unsaved-changes prompt reads the same ones.
- The rows of the players both saves hold group per team:
  - A player stands under the first team in the open save's order whose roster holds him
    (`No team` when none does), not one heading per roster, which would repeat a player on a
    club and a national team.
  - There is one group per team, in the open save's team order (the team selector's), its
    players in the report's order, `No team` after the teams. Not a heading at each change
    along the report's order, which heads a team once per run of its players, and a real
    save's teams hold ids far apart.
  - The players only in one save come last, under `Only in the picked save` and `Only in the
    open save`, as the report lists them.

  The comparison runs on a worker thread, the held list marked `Comparing…` until it lands,
  and a frame draws only the rows in view: two real PES 2021 saves differ in 34,379 rows
  (about half a second for the command).
- Another save opened keeps the pick and compares it again (a member compares several saves
  against one reference), refused as any pick is when the versions differ.

---

## Aesthetics transplant

Built after Release 0.1.0 (0.2.0 or later); 0.0.1 and 0.1.0 ship without it.

A team's looks copied from a save into the team's export, the files the Team compiler compiles
from, never into a save: aesthetics live in the exports, and a look written into a save is one the
next compile overwrites.

Midcupping's save-to-save transplant is not kept, in the window or on the command line. Its one
remaining use, a team with no export, is served by generating the export from the save
("Interchange formats in the UI", `settings.toml` generation) and editing it there.

- Pick a donor save, then one of its teams by name (a combo that also takes typing).
- The team's roster as checkboxes, one per filled slot, `NN name` (the slot, 01-23, and the
  display name): the roster slot is what identifies a player, as it names his folder in the
  export (`NN - Name` or a root `players.txt`); shirt numbers are free text and identify nobody.
- The target is that team's own export in the exports folder, found the way the Team compiler
  finds a team's export; each checked slot's player folder in it. Cross-team copies are out of
  scope (a member copies the TOML files by hand). A missing export or a checked slot with no
  folder refuses the whole run before anything is written, naming what is missing.
- Each target folder's `settings.toml` and `ingame_face.toml` are updated in place through
  `toml_edit` (only the keys the donor's look carries are written; comments and every other key
  stay), created when the folder has none; the keys and values are the ones `settings.toml`
  generation writes for the same player.
- **Preview before writing**: every file listed with the keys it changes, old → new, so a wrong
  team or slot is visible before it does damage.
- Not under "Read-only aesthetics": writing the export is where a look is meant to change.

---

## Interchange formats in the UI

The formats themselves are specified in the [Savefile plan](pes_savefile/README.md);
the editor exposes them as:

- **Team TOML export/import** per team (the `.4ccs`/`.4cct` successor: complete
  team + tactics + players including full aesthetics and ingame-face data).
  Export writes the command line's `export-toml` text for the selected team, and, as the
  command line does, refuses a destination that is the open save itself (a Team TOML written
  over it would lose it); the same refusal guards the Texport export.

  Import goes onto the team selected in the editor and offers the same section choices as
  4ccEditor's squad load (gameplay / aesthetics / tactics), and applies cross-version
  conversion with a warning list when the file's version differs.

  The sections split the file as the squad load does, so a file that loads with 4ccEditor's
  choices loads the same here (`editor.h` `PlayerImport`, read 2026-10-21):
  - names, shirt names and shirt numbers go with aesthetics (its `PlayerImport` copies them
    under `importAes`), not with gameplay as the comparator's scopes have them, and so do
    nationality, age, every motion and the base-copy flag and id;
  - height, weight, star, playing attitude and stronger hand go with gameplay.

  One departure: the squad load copies the free-kick motion with the stats, and here it goes
  with aesthetics, with the other motions, since it is an `[appearance.motion]` key of the
  export's `settings.toml` like them (the aesthetics lock then guards it too). The file's
  `[team]` keys, which the squad file never carried, split: the name, short name, manager and
  stadium, with their edit flags, go with gameplay; the two team colours, the kit slots and
  the strip flag go with aesthetics, the team's look.

  A refusal that comes from a chosen section's keys (a team name too long, with gameplay; a
  shirt number past the field's range, with aesthetics) is drawn under the section choices
  with Apply disabled, the choices still drawn, since unticking that section is how the member
  imports the rest. Only a `.4ccs` with more records than the target's roster is refused
  whatever the choices, drawn alone. An import that would change nothing (the team's own file,
  or one applied already) draws Apply disabled and `The team already holds every value of the
  chosen sections.`, as the patch panel's empty list says so, rather than recording an edit
  that changes nothing.

  The section boxes read Stats, Aesthetics and Gameplan, the cup's own words for the gameplay
  and tactics sections (the comparator's Gameplay scope and the command line's `--gameplay`
  split the fields differently and keep their name).

  The warning list draws each note code's text from the catalog once (its when and its
  effect, the Messages topic's words too, written for any team file rather than for
  `import-toml`), the code's groups under it.

  A group is one key in words (its last part as the views name it: a stat at the save's
  version, a skill by its card name, a motion, an edit flag and a preset's instructions with
  what they belong to: `Manager ID`, `Tight Possession`, `Dribbling motion`,
  `Preset 1 attacking instructions`) with its number of players. It lists each player's place,
  name and what the note carries (an instruction's or a style's name, a lowered value's
  `19 → 0`). A team key's group lists nothing, its notes' details following its subject on
  its one line (`Preset 1 attacking instructions: Wing Back`), since a tactics key's note is
  the only place the instruction it writes off is named (the drafts' `ixWarningGroups`).

  The console's line, with its code and key path
  (`Warning import_key_not_in_version (path=players.03.stats.star)`), is the CLI's form, not
  the dialog's words.
- **Legacy import**: `.4ccs` and `.4cct` open through the same import dialog,
  read-only.
- **Picking the file**: the import's file dialog has two filters, "Team files" (`.toml`,
  `.4ccs`, `.4cct`, `.ted`) and "All files", and opens on the first. On a PES 2015-2017 save
  "All files" comes first, since those games name a Texport `TEXPORT0000000N`, with no
  extension, and "Team files" hid it until the member switched filters. On PES 2018-2021,
  whose Texports are `.ted`, "Team files" comes first.

  Not a pattern for files without an extension: Windows' dialog matches nothing with `*.` and
  shows a filter's patterns in its name (measured 2026-10-30), and a `TEXPORT*` pattern is not
  something `rfd`'s filters, extensions alone, can say. The save files beside a Texport then
  show too; one picked by mistake is refused with the panel's sentence (`<name>: the
  container's description is not "Team Export Data" (a save's is "Edit Data")`), nothing
  changed.
- **Texport import/export** (export is new; PES itself can import the file). The cup events
  take a team's tactics as a Texport until they accept Team TOML files, so both exports
  stay.

  Export writes PES 17-21 files from `Texport::new`'s templates and the open save
  (`plans/pes_savefile/operations.md` "Texport (read + write)"). On a PES 2015-2016 save it
  is disabled, its tooltip saying `PES 2015 and 2016 have no Texport template`.

  The Save dialog suggests `<short name>.ted` on 18-21. On 17 it suggests the name the game
  reads from the save's folder: `TEXPORT` and an eight-digit hexadecimal number in upper
  case, no extension (`TEXPORT0000000A` follows `TEXPORT00000009`; real cup folders hold
  `TEXPORT0000000D` and `TEXPORT00000013`), the first number from `00000000` that folder
  lacks, so the dialog opens on a name that writes over nothing. A decimal count would suggest
  `TEXPORT00000010` after `00000009`, a name the game reads as number 16.

  PES 2017's own import differs in two ways the panel keeps:
  - the game offers only the Texport's own team as the target (measured 2026-10-30), where
    the panel imports onto the selected team;
  - and the game writes into the save the team name a team bin in the DLC (a binary file the
    game loads from a CPK in its download folder) gives the target's id, whatever the file and
    the save named the team, where the panel writes the file's: `/gsg/` imported onto a team
    the save names `/fgog/` and the bin `VGL 17` leaves `VGL 17`.

  So organizers retype each name (`/gsg/`) after importing the cup's Texports in game; the
  other versions likely do the same, unconfirmed (worklog 8.9k5 k5g). Editing that bin, which
  would spare them, is future work with no plan yet.
- **Aesthetics patch apply**: see "Read-only aesthetics"; the patch is written by the Team
  compiler, never by the editor.
- **`settings.toml` generation**: per player folder, for migrating a team to the
  [Team compiler](team_compiler/README.md)'s compile-time aesthetics (written to the savefile
  through the aesthetics patch). Uses the shared authorable `PlayerSettings` subset;
  boots/gloves IDs are emitted only when they name a stock model (1 to 100), since custom IDs
  are compiler-assigned from models/links, not export settings. Full Team TOML remains a
  full-fidelity save interchange.

  Name handling: the editor generates without a folder, so it writes the save's strings
  (`name = true` is the Export upgrader's rule) — in particular a name carrying colour codes
  is always emitted as the explicit string, so the next compile preserves the codes.

  The face, hair and iris keys are not in it. A second action, **Generate in-game face**,
  writes them into the folder's `ingame_face.toml`, since the save cannot say which players
  wear the in-game face: the cup keeps every "modified face" flag off, the game choosing the
  face folder whenever one exists for the id (`spec/aesthetics_export/player_folders.md`
  "`ingame_face` marker"), so the member who knows the folder picks (SE-GUI-33, 34).

  Both are Player-menu items on the selected player, each a dialog with the folder (typed or
  picked, never guessed: the save does not know which export or folder name a player has), the
  file's text as it will be written, and Write. A `settings.toml` already in the folder is
  updated in place (`update_toml`: its comments and layout kept, the values the record's),
  since user-facing TOML is edited, never regenerated. `ingame_face.toml` is written whole,
  its values all the record's and its comments the template's, so there is nothing of the
  member's to keep. A `settings.toml` there that does not parse is shown as the error and
  nothing is written, rather than replaced by the template.

---

## Configurable AATF rules

What exists of this is in `spec/save_editor.md` "Configurable AATF rules".

What this editor adds on top of the lib:

- **`check-aatf --rules` takes an embedded ruleset too.** `--rules dbc7` checks against the
  embedded DBC7 ruleset, by the stem the `aatf_ruleset` setting already stores: a value naming
  an existing file is that file, else an embedded ruleset's stem is that ruleset, else the
  value is refused (exit 2) with the embedded stems listed. Without it a check against a
  built-in ruleset had to go through the settings file or a copy of the ruleset written out
  (DBC7's organizer, checking the DBC6 save, 2026-10-31).
- **Multi-team AATF's dialog is 900 points wide and its rows wrap.** At 640, the width every
  other dialog keeps, a row's sentence ran past the list and was cut off (a `BracketQuota` row
  of the GigaHeight system, and a height row's players). 900 is as wide as the window's
  960-point minimum leaves room for; the list is the dialog's content width (874), and every
  row, a sentence as a height row's players, wraps under its own column, the rule's name
  keeping its own.
- **Multi-team AATF counts the teams under each system.** One line above the list, and the
  same line first in its copied text, gives how many of the checked teams were read under
  each medal system and each height system of the ruleset, so an organizer sees a cup's mix
  without counting. The systems come in the ruleset's order, a system no team was read under
  left out, the two kinds after each other with a ` · ` between them (DBC6's 47 teams under
  DBC7: "Medal systems: GGG 4, GGSS 21, GSSSS 20, SSSSSS 1 · Height systems: Height 8,
  Manlet 14, Universal 11, GigaHeight 14"; team 868 matches no medal system and counts toward
  none).

  One line rather than one per kind: the list below is what the dialog is for, and the counts
  fit the dialog's width. The check reports the systems each team was read under (`aatf`'s
  team result), not the list's rows, which name a system only when something is off. A kind
  with one system or none is left out (the official ruleset, with no medal systems, shows
  its Green and Red counts alone), and with neither kind the line is gone. Draft:
  `save_editor.js` `multiAatfCounts`; scenario SE-GUI-51.
- **Multi-team AATF's range starts on one team, never reverses, and is checked once it
  rests**: built (`spec/save_editor.md` "Multi-team AATF"). The pictures' range below pairs
  its ends the same way, keeping the whole save as its default, since its run starts only on
  Save pictures.
- **The ruleset dropdown** (`spec/save_editor.md` "Configurable AATF rules" for what is
  built): the wheel over the closed combo (`core/gui.md` "The wheel over a value") steps the
  rulesets alone, never running `Import rules file…`, which is a command rather than a value.
- **Quick actions.** Under the player list, in two columns (the draft's `quickActions`):
  - one button per tier of the active ruleset, the medal tiers on the left and the others on
    the right, each in its order ("Editing UI": Make Gold / Silver / Bronze, then Goalkeeper /
    Regular under the official one), each `aatf::apply_tier` on the selected player as one
    undoable edit;
  - then Set stats with its number, the Player menu's Set all stats in one click (40 to 99).

  An `apply_tier` that fails (`CheckError`) leaves the player as he was and shows the error in
  the refusal dialog.
- **Live AATF.** A toolbar toggle, off by default and remembered in the tool's settings
  (`live_aatf`), for a member who has built the team and is now making it legal.

  On, a panel docked at the bottom of the view lists the violations of the team the view is
  editing (the selected row's team, else the selector's), errors first:
  - the team's own first under `Team-wide issues` (a formation or a tier count is the team's
    first concern, ahead of any one player's; every list of a team's issues, Multi-team AATF's
    and its copied text included, puts them first);
  - then grouped per player under a line naming him;
  - then, while **Show suggestions** is on, under a `Suggestions` heading, the team's unused
    allowances (`Unused` rows, `only 4 of 5 cards used`).

  A player's row tooltip is `Click to switch to this player`. A `Fielded position` row's is
  `Click to show him on the team's pitch` (its click opens his team's row on the preset it
  names, the cross on his marker: the issue is where he stands in the tactics, not his
  record). A team-wide row has none (it selects nothing, and its group's heading already says
  what it is).

  Show suggestions is a checkbox right of the toggle, drawn only while Live AATF is on, off by
  default and remembered (`live_suggestions`). On, the check asks for the suggestions, so a
  player whose list mark is blue has his reason in the dock. Off, no blue mark is drawn
  anywhere: a suggestion is no failure, and most players of a cup team have an allowance
  unused (in the PES 17 sample under the official rules, most of the two checked teams'
  players, for their free tricks), so unasked they would tint the whole list.

  In each of the two parts (the violations, then the suggestions) the players' groups stand in
  two columns, the starting eleven's on the left and the bench's on the right, each in
  roster-slot order, and the team's own rows span both columns above them: a player's rows are
  short and the dock spans the view's width, so one column left most of it empty. Multi-team
  AATF keeps one column: its report reads team by team, and its rows were just widened to read
  whole.

  A row under the pointer, here, in Multi-team AATF and in the comparator, is outlined in the
  hovered widget's stroke (1 point, gray 150 in the dark theme), never filled. A fill under a
  row's coloured words left an error's red rule name at 2.4:1 on egui's hover grey (1.9:1 on
  the selection blue the draft had), where the outline leaves it the 5:1 it has at rest.

  **Copy to clipboard**, Multi-team AATF's button, stands left of the dock's pop-out icon and,
  in the window, under its close button at the count line's right end. It writes the list as
  it shows, as Multi-team AATF's copy words a team: a line naming the ruleset as that copy
  does, the team and its error and warning counts, then the team's groups, then under
  `Suggestions` the suggestions while Show suggestions is on (a member who wants them out
  switches them off). A team that passes reads `All clean` there, as the list does.

  The button is never greyed out: before a list is held it copies the line the panel shows
  (`Checking <team>…`, or `No team is open.`), and after a failed check its message as the
  list words it. A notice `Copied the Live AATF list of <id> <name>.` says the copy was done
  (`Copied the Live AATF list.` with no team open).

  **Its own window**: the pop-out icon at the header's right end (the icon font's arrow out of
  a square, U+E5DE, Phosphor's arrow-square-out; tooltip
  `Open Live AATF in a window of its own`) moves the list into an `egui::Window` over the
  view, and the dock is gone until it comes back.

  Right of the icon, in the corner the window's close button holds in the window, the dock
  has a close button of its own, drawn as egui draws a window's (the same cross, size and
  stroke) and spaced from the icon as the window's is from its dock icon, with the same
  tooltip and effect: `Turn Live AATF off`, as the toolbar toggle does. It is one way to put
  the list away wherever it stands, where the dock alone had none but the toolbar's toggle, a
  long way from it. The dock's chevron only collapses it.

  The window's title is `Live AATF`, its title bar holding left of the close button the dock
  icon (the arrow into a square, U+E5DF, Phosphor's arrow-square-in; `Put Live AATF back in
  its dock`), which puts the list back, while the close button (`Turn Live AATF off`) turns
  Live AATF off as the toolbar toggle does. Under the title stand the dock's count line and
  checking mark, then the same rows in the same well, the players' groups in one column in
  roster order as Multi-team AATF lists them. The rule's column is 140 points (the longest
  rule name, `Weak Foot Accuracy`, draws about 114) where the dock's is 190: the window first
  opens 460 × 360 at the view's bottom right, where the dock stood and away from the floating
  panels' top-right spot, narrower than the dock, so two columns would wrap every sentence.

  It is moved by its title and resized from its edges, as the floating panels are, and
  remembered between sessions as in the window or in the dock (`live_window`, a tool setting
  beside `live_aatf`), so turning Live AATF on again opens it where it was. A member keeps it
  open beside the player and the tactics they are fixing. An embedded window, not one of the
  desktop's own (egui's viewports): it needs no second native window, moves with Studio's, and
  works wherever Studio does. A native window comes after 0.0.2 (worklog), without
  always-on-top, which is desktop-wide, so a member just moves it off the main window.

  The header reads `N issues · <ruleset>`, counting errors and warnings, or
  `<team> passes <ruleset> with no issues.` once neither is left, the list then reading
  `All clean` (Multi-team AATF's word) above any suggestions. Clicking a row selects that
  player (a `FieldedNotA` row selects his team's row instead, the named preset shown on the
  first of its formations that carries the cross, since the pitch its mark is on is the team
  view's).

  The team is re-checked after every edit to it, undo and redo included, and when another team
  is opened, so the list shrinks as the member fixes it. One team's check takes about 56 ms in
  a release build, nearly all of it the script's own work (`spec/aatf_rules.md` "The host"),
  more than a frame. So it runs on a worker, on the ruleset the window loaded, once the edit
  settles (a drag released, a field committed), and the list keeps the previous result, marked
  as checking, until the new one arrives. A result checked under another ruleset than the
  active one is dropped instead (the list reads as before any result), since its rows word
  findings the active ruleset never produced.

  The panel is the log panel widget's frame (its chevron collapses it to the header, its
  divider resizes it), its rows in the log's well (the extreme fill, the stroke, 6 × 2 points
  of margin), the body 150 points high when the panel first opens (the drafts'
  `LIVE.height`): lower than the log's 180, since the views above it need the height more
  than a compile's output does. At the bottom, not at a side: the player list holds the left
  and the views need their width (the player view's three columns), and a bottom strip is
  where the Team compiler's log already sits.

  The toggle is drawn as a button, framed, its label a play sign (`▶`, U+25B6, from the
  suite's symbols font the marks' glyphs come from) then `Live AATF` whether on or off, filled
  in the accent colour when on: the fill says on, and a check mark beside the label read as on
  even while it was off. egui's toggle value is a selectable label, which off reads as a label
  beside the menus, and the toggle is how a manager finds the check at all.

  While it is on, its result also marks the editor itself, so a manager sees what to fix
  where they fix it:
  - **The player list** marks each listed player of the checked team, at the row's right end
    after its unsaved-changes dot:
    - a cross (`✖`, U+2716) in the error colour on a player with an Error-severity row;
    - else a check mark (`✔`, U+2714; warnings and suggestions are not failures, as
      `check-aatf`'s exit code counts them), in the warning orange when one of his rows is a
      Warning, in the link blue when his rows are suggestions alone, in the success colour
      when he has none.

    Both glyphs are the suite's symbols font's. Its tooltip is the player's rows as the dock
    counts them, a zero count left out (`2 errors, 1 warning`, `1 suggestion`), and
    `No errors` when he has none. Players of other teams, with ALL picked, carry none: they
    are not checked. The team's own rows (no slot) mark the team's row the same way.
  - **The field a row is about** gets a mark when its player is selected (the pitch's, when
    his team's row is: the pitch is the team view's), for the highest severity among its
    rows:
    - a cross (`✖`) in the error colour for an Error,
    - a warning sign (`⚠`, U+26A0) in the warning orange for a Warning,
    - a circled i (`ⓘ`, U+24D8) in the link blue for suggestions alone.

    All three are the symbols font's (`ⓘ` drawn for it in the warning sign's stroke and
    height, so the two read as one family; a plain `!` and `i` were too slight to see). Not
    the list's check marks: on a field a check would read as "this field passes", which a
    field with a row does not. Every mark is drawn in an 11-point box, the widest glyph's (the
    warning sign, 10.6 at 12.5), its glyph centred, so a column's marks line up by their
    centres.

    The mark's box stands left of the field's label, 2 points from its text, drawn outside the
    label's box over the margin before it, so no label moves or wraps (after the label, the
    Registered position and `Weak Foot: Usage` labels wrapped to a second line) and the marks
    of a column of labels line up whatever the labels' lengths:
    - General info's and Other stats' labels;
    - an ability stat's at its label cell's left edge instead (over the cell's padding), where
      the column's left-aligned labels start, so the stats' marks share one column;
    - a panel's header for a row about the panel as a whole (the positions grid, the skills,
      the Tricks or Free cards panel, the COM styles).

    The Skills header's Cards used badge has no mark of its own: its rows mark the Skills
    header, the group holding the card counts, in one mark with the skills' own (the highest
    severity of both, both in its tooltip), since a mark in the row before the badge would
    shift the badge whenever it showed. The first column's marks stand in the gap after the
    list, which General info's inset widens enough ("The player view").

    The fields:
    - the Height box for `Height`;
    - the Age, Weight, Form, Injury resistance and weak foot fields for theirs;
    - the stat for `Stat`;
    - the name for `NameColor`;
    - the registered position for `RegisteredNotA` and `PositionRange`;
    - the positions grid for `BRating`, `GkSecondA` and `APositions`;
    - the player's spot on the named preset's Tactics pitch for `FieldedNotA`. It is drawn
      while the team view shows that preset, on each formation the preset fields that puts
      him at the violation's `position`: a fluid preset fields all three, and its others may
      field him legally; a preset not fluid fields its kick-off one alone, so its other two
      carry no cross. It is an X over his whole portrait, its two strokes in the severity's
      colour inside a thin black outline that keeps it seen over a red portrait, where a
      field's glyph beside the badge was barely seen, and painted after every marker, since a
      marker drawn later can cover his portrait with its own;
    - the playing style for `PlayingStyleRange`;
    - the skills for `SkillCardCap`;
    - the COM styles for `ComStyleCap`;
    - the skills or the COM styles for `RequiredCard` by its card's kind;
    - the Skills header for `Cards` and `MinCards` (the Cards used badge's), which no one
      field causes: `Cards`' budget counts every card kind and the extra A positions,
      `MinCards`' count the paid skills and the extra A positions;
    - and for a suggestion (`Unused`), the field of its allowance: the Skills header for the
      cards, the positions grid for the A positions, the `Weak Foot: Usage` box for the weak
      foot's usage and the `Accuracy` box for its accuracy (the rules file's `weak_foot` word
      reports the two as rows of their own), the Injury resistance field, the Height field for
      the height (a player below his `up_to` bracket's top could be taller), the COM styles
      for `com`, the Tricks or Free cards panel for the tricks and the free cards, and the
      skills for any other card group.

    A field several rows are about carries one mark, its tooltip their lines as the dock shows
    them, one per line. One function maps a kind to its field, a match with an arm per kind (a
    new kind does not compile past it). The kinds about no single field (`NoTier`,
    `StatTotal`, `Special`, `Custom`, `RosterOrder`) mark the player's row alone, and the
    team-level ones the team's. A field the view does not show (the Appearance section closed,
    another preset's pitch) carries its mark when shown.
  - While a check is in flight the marks keep the previous result, as the list does, and
    like it drop a result checked under another ruleset. A check that fails (a script error
    in the rules file) clears every mark: the dock shows the failure, and a mark from an older
    result would claim a check that no longer ran.

---

## Build order and verification

The build is split across three phases (see the
[core plan](core/development_plan.md#development-plan)):
- `pes_savefile` (codec, model, conversions, fingerprint ops, interchange formats, comparator,
  FPC invisibility) is built in Phase 2 as a standalone lib;
- the editor's non-UI substance a command drives — the tool crate's settings, CLI and the
  `session/` operations its commands run, plus the `aatf` rules engine — lands in Phase 5;
- the view lands in Phase 8 with the `session/` wiring only it drives (undo, selection, recent
  files, the FPC toggle, AATF quick actions), each built in the slice that uses it.

Within Phase 8, the draft comes first: `docs/prototypes/tools/save_editor.js` (`core/gui.md`
"Prototypes before implementation") shows the master-detail layout, every view and every
panel of the five slices below with sample data, and is refined with the maintainer. A slice
that changes what the draft shows changes the draft first.

The draft is the third arrangement (step 0.0.1s, "Editing UI"): no tabs, a team row per team
and the team view, the player's fields in three columns, Appearance a collapsed section. The
second's (the team bar above every tab, the Player / Team tab groups) was built in slices 1-3
and 8.9f's slice 4, and is ported to the third in step 0.0.1s3. The arrangements' panels and
dialogs are the same, function for function. The view builds up as:

1. Open/save + the player's fields (abilities and skills, appearance) — the card pickers as the
   first `team_widgets` content
2. The team's fields + batch operations + FPC
3. Tactics editor in `team_widgets` (pitch canvas last — everything else works without it)
4. The Comparator panel and Live AATF (the transplant after Release 0.1.0)
5. Interchange format dialogs (Team TOML, legacy import, Texport, settings.toml)

Verification:

- Field-level parity: the same save opened in 4ccEditor and in the new editor
  must display identical values for every field, all versions (spot-checked per
  panel; automated for the codec layer in `pes_savefile`'s roundtrip tests).
- Batch operations and Fix database compared against 4ccEditor's output on the
  same input save (byte-diff of the decrypted payload).
- AATF: the official ruleset's parity with `aatf.cpp` and the rest of the AATF rules plan's
  "Development phase and verification".
- Comparator output parity against the Midcupping scripts, its aesthetics scope against
  the transplant script's byte slice.
- Texport export verified by importing into the game (manual, per version).

---

## Acceptance

What exists of this is in `spec/save_editor.md` "Acceptance", whose rules (format, the fixture
saves, a team or player named by its id in them) these scenarios follow; IDs continue the
spec's numbering.

**Phase 8 scope.** Phase 8 delivers the view and the `session/` wiring only it drives (undo,
selection, recent files, the FPC toggle, AATF quick actions), in the five slices of "Build
order and verification".

What a scenario observes is:
- what a frame draws, through the headless harness (the frame's AccessKit nodes, a click or a
  drag as pointer frames, keyboard shortcuts as key events; a color, which no AccessKit node
  carries, is read back from the view state the frame paints from);
- the file written by Save;
- and the session's model.

`manual` marks only what needs a display, a native dialog or a running game. "The save" is
the fixture save of the version named, PES 21 unless the scenario says otherwise, copied under
the test's own folder; save discovery is pointed at a Documents folder under that folder,
never the developer's.

An operation's outcome in the file is its own specification's where one exists (a patch's,
`SE-PAT`; a tier's values, `spec/aatf_rules.md` `apply_tier`; a Team TOML's, the CLI's
export); these scenarios observe that the view reaches the same operation and shows its
result. The operations with no command, which the spec's Phase 5 scope left to the view (the
quick actions, the player and team operations, the tactics copies, the Texport and legacy
imports, generation), get theirs here.

```
SE-GUI-01  GIVEN the save
           WHEN it is opened from the view (Open, or its entry in the discovered-saves list)
           THEN the team selector lists its teams and ALL, the player list shows each player's
                place in the roster, position and name with the name's color codes rendered as colors under
                its team's row, and the player view's first line is the selected player's id
                and name, its color codes rendered as colors, over his General info fields
SE-GUI-02  GIVEN the PES 17 save open on team 701
           WHEN a name, shirt name or id of a player of another team is typed in the filter box
           THEN the list shows the matching players across all teams, matched on the
                color-code-free display name, and selecting one shows him
SE-GUI-46  GIVEN the PES 17 save open on team 701, ALL picked in the team selector
           WHEN part of team 718's name that none of its players' names, shirt names or ids
                holds is typed in the filter box
           THEN team 718's row heads every one of its players; WHEN that row is double-clicked
                THEN the selector shows team 718, the filter box is empty, the list holds team
                718 alone and its row is selected; a team row's tooltip reads `Double-click to
                filter to this team`
SE-GUI-03  GIVEN the save open
           WHEN a player's ability value is edited in the player view
           THEN his list entry carries the unsaved-changes dot, the file is unchanged until Save
                (Ctrl+S or the button), and after Save the reopened file holds the value with
                the dot gone
SE-GUI-04  GIVEN two edits made in order
           WHEN Undo is invoked, then Redo
           THEN the second edit's field returns to its earlier value and the first's stays,
                then the second is reapplied; the file changes only on Save
SE-GUI-05  GIVEN the save open with unsaved changes
           WHEN the window is closed, or another save opened
           THEN a prompt offers to save, discard or cancel over a table of the changes, one
                row per changed field (GIVEN a player's Height and Speed edited and a third
                field edited then undone THEN two rows, each naming him and the field with its
                old and new value as the view shows them); cancel leaves the session as it
                was; save writes the edits to the open file, then closes or opens the other
                save; discard closes or opens it and the open file is unchanged; GIVEN the
                Team compiler's view active when the window is closed THEN the close waits,
                the Save editor becomes the active view and shows the same prompt
SE-GUI-43  GIVEN the save open with a player's Height and Speed edited and team 701's colour 1
           edited
           WHEN the toolbar's `• unsaved changes` is clicked
           THEN a popup under it lists SE-GUI-05's table (three rows: the two fields named
                with the player, old and new values as the view shows them, and the colour
                whose old and new values each draw their swatch before the hex) and none of
                the prompt's save, discard and cancel; a click outside it, Esc, a second click
                on the label or another edit closes it; GIVEN the toolbar's Undo clicked
                THEN it closes and a click on the label reopens it two rows long; a row's
                item takes no click; GIVEN the pointer over the label THEN it is the pointing
                hand, the tooltip `Lists the changes since the save was opened or last
                saved`; GIVEN team 701's 23 players' heights edited THEN the popup's rows stop
                at 240 points and scroll; GIVEN a 52-letter name THEN the popup holds at 640
                points wide and its table scrolls sideways
SE-GUI-06  GIVEN a PES 17 save
           WHEN the player view is drawn, its Appearance section open
           THEN the stronger hand control (General info) and the star and playing attitude
                controls (Appearance's Cutscenes group) are disabled with a tooltip naming the
                first version that has them, in the same place as on a PES 21 save, where they
                are enabled; GIVEN a PES 21 save, the aesthetics editable THEN Copy from ID's
                Base copy flag is disabled in place, its tooltip `PES 15 to 18 only`
SE-GUI-07  GIVEN the save open with "Aesthetics from exports" on
           WHEN the Appearance section is drawn open
           THEN its values below the Cutscenes group are disabled with the line saying the
                team's export owns them and the patch step, the Cutscenes group's editable;
                WHEN "Edit aesthetics anyway" is switched on THEN they are editable, and the
                switch is off again at the next start of the tool
SE-GUI-08  GIVEN the save open on a team
           WHEN player 02 is dragged above player 01 in the list
           THEN the list shows the new order, and after Save slot 01 still holds player
                `team id × 100 + 01`, whose record (name, stats, appearance but the face and
                hair) is the one player 02 had, and the other way round; the roster's ids and shirt numbers are as they
                were, and the AATF check reports no `RosterOrder` error for the team; GIVEN
                ALL picked THEN the same drag has the same outcome, and WHEN player 02 is
                dropped on a row of the next team instead THEN nothing moves
SE-GUI-09  GIVEN the official ruleset active
           WHEN Make Gold is clicked on a player
           THEN his fields hold the Gold tier's values as `aatf::apply_tier` sets them
                (`spec/aatf_rules.md`), as one undoable edit, and check-aatf then reports no
                violation of the Gold tier's targets for him; GIVEN the VTL11 ruleset active
                (bundled, `aatf/rulesets/vtl11.rhai`) THEN the quick actions are Make Gold,
                Silver, Regular and Buffed in that order, no Bronze or Goalkeeper, and Make
                Silver sets VTL11's Silver values (rate 89, not the official 92); WHEN 85 is
                typed into Set stats' number and Set stats clicked THEN every ability stat the
                version has reads 85 and weak foot, form and injury resistance are unchanged,
                as one undoable edit, and a typed 120 settles at 99 (the number is 40 to 99)
SE-GUI-10  GIVEN the save open and a team's row clicked
           WHEN its name, a color or the manager id is edited in the team view, and saved
           THEN the reopened file holds the values; GIVEN #FF8000 picked as color 1 THEN the
                reopened team's color 1 is (63, 32, 0) and the team view shows it as #FF8200,
                its row's swatch that color
SE-GUI-11  GIVEN a player without FPC, his Appearance section open, and the aesthetics
                editable (the setting off, or on with "Edit aesthetics anyway" switched on)
           WHEN Ctrl+I, or Hide default body, is used
           THEN his record carries the hide preset's values and the Appearance section reads
                "The default body is currently hidden"; Ctrl+I again, or Show default body, gives him the show preset's; GIVEN a player
                whose boots ID is 627 THEN hiding keeps 627; Set FPC invisibility applies the
                hide preset to every player of the team; a strip setting that breaks hiding is
                listed in the Appearance section as an interference finding
SE-GUI-12  GIVEN the team view's Tactics on preset 1, and /vg/ League 27, which does not allow
                illegal formations, the active ruleset
           WHEN a pitch player is dragged to another spot
           THEN the slot's position is the drop point after snapping, undoable, and the saved
                file holds it; WHEN a bench player is dragged onto a pitch player THEN the two
                swap; WHEN a pitch player is dropped on another on Kick-off THEN the two slots'
                positions and spots are exchanged in preset 1's Kick-off alone, the starting
                eleven, the other two formations and presets 2 and 3 unchanged; WHEN an XI row
                is dropped on another THEN the two players are exchanged in the starting
                eleven, every formation unchanged; WHEN a set-piece role is assigned from the palette THEN the taker field
                holds that player; WHEN "players to join attack" is in its assignment mode and
                two pitch players are clicked THEN those two are the setting's players
SE-GUI-42  GIVEN the PES 21 save's team 718 (/f/) on preset 1's Kick-off, that formation and
                its LMF PARTY HARD's overalls and ratings (LMF A and LWF A) set from the
                draft's PES 21 sample save (the fixture's teams carry a fresh save's
                defaults), PARTY HARD at (20, 26), and /vg/ League 27, which does not allow
                illegal formations, the active ruleset (for the PES 17 save's team below
                too)
           WHEN he is pressed on his portrait and held
           THEN the pitch outside x up to 20.5 from y 15.5 up is darkened, and moving the pointer
                moves his marker a whole unit at a time, the pressed point of his portrait
                under the pointer within a unit; WHEN he is held at (30, 26) THEN his badge
                reads AMF; WHEN he is then released at (19, 20) THEN his slot holds (19, 20)
                and LMF, one undoable edit; WHEN his popover is opened at (20, 26) THEN its
                Position combo lists CMF and LMF alone, and WHEN its Y is set to 36 THEN he is
                an LWF; GIVEN the PES 17 save's team 702 (/a/), its CF COWBOY BEBOP (70223,
                rated SS A and CF A) at (40, 45) WHEN he is held at (42, 30), clear of his
                teammates' markers, THEN his badge reads AMF, and WHEN he is held back at
                (40, 45) without a release THEN it reads CF; WHEN he is released at (42, 30)
                THEN he is an AMF, and WHEN he is then dragged to (40, 45) THEN he is a CF;
                WHEN he is instead released at (42, 30), then at (42, 37) (an SS), then
                dragged to (40, 45) THEN he stays an SS; WHEN an outfielder is held with the
                pointer below the pitch THEN his badge stands on y 4, and WHEN he is released
                on the grass there THEN his slot holds y 4; WHEN his popover is opened at y 4
                THEN its Y's ⏴ is disabled
SE-GUI-45  GIVEN the PES 17 save's team 702 (/a/) on preset 1's Kick-off, its CF COWBOY BEBOP
                (70223) at (40, 45), and DBC7, which allows illegal formations, the active
                ruleset
           WHEN he is held at (42, 30), clear of his teammates' markers
           THEN his badge reads CF, and WHEN he is released there THEN his slot holds (42, 30)
                and CF, one undoable edit; WHEN he is held with the pointer below the pitch
                THEN his badge stands on y 0, and WHEN he is released on the grass there THEN
                his slot holds y 0; WHEN his popover is then opened THEN its Position combo
                lists the twelve outfield positions, CB to CF, and its Y's ⏴ is disabled, and
                WHEN its Y's ⏵ is clicked THEN his slot holds y 1; WHEN LB is picked THEN his
                slot holds LB, its X and y 1 unchanged, and WHEN its Y is then set to 36 and
                its X to 60 THEN his slot holds LB at (60, 36); WHEN he is
                held over the SS's marker at (64, 42) THEN his badge reads LB at full opacity
                and the SS carries no corner brackets, and WHEN he is released there THEN his slot
                holds LB at (64, 42) and the SS's slot is unchanged; WHEN the active ruleset is then switched to
                /vg/ League 27, which does not allow them, and he is held at (42, 30) THEN
                his badge reads AMF, as in SE-GUI-42, and his slot is as he left it
SE-GUI-13  GIVEN the save open and a second save of the same version picked in the comparator
           WHEN the panel is drawn
           THEN it lists, per team and per player, the differences `diff <picked> <open>`
                reports, each row under the label its field's view draws it with and with
                its values as that view shows them (a field no view shows is named in words
                from the report's name, its stored values kept); the All / Gameplay / Aesthetics toggles and the team filter
                narrow them; Export writes the text `diff <picked> <open>` prints for the
                toggled scope, whatever the team filter; clicking a row of a player the open
                save rosters selects that player (one only in the picked save, or on no
                roster, is listed and not clickable); each player's rows stand under the
                head `Player <id> - <number> - <name>`, the id and hyphens in the weak colour;
                each row draws its label, the picked save's value and the open save's in
                three columns, at 4, 202 and 410 points from the row's left, under one head
                row `Item`, `Old value`, `New value`, and no arrow;
           GIVEN a save of another version picked THEN it is refused naming both versions;
           GIVEN team 734 picked in the team filter WHEN another save is opened THEN the
                filter reads ALL; WHEN the same path is picked again after the file changed on
                disk THEN the panel lists the new differences; WHEN Export writes diff.txt
                THEN the panel reads `Wrote <path>`, and WHEN it names a folder that does not
                exist THEN a dialog titled `Cannot export the differences` names the path
SE-GUI-14  GIVEN (after Release 0.1.0) the transplant panel, a donor save, one of its teams and
                that team's export in the exports folder, slots 03 and 07 checked
           WHEN Preview, then Apply
           THEN the preview lists the two folders' settings.toml and ingame_face.toml with each
                changing key old → new, and Apply writes exactly those keys, the files' other
                keys and comments untouched; GIVEN a checked slot with no folder in the export
                THEN the run is refused naming the slot, and nothing is written
SE-GUI-15  GIVEN the PES 17 save open and no ruleset imported
           WHEN the toolbar's ruleset dropdown is opened
           THEN it stands left of the Live AATF toggle, whether Live AATF is on or off, and
                lists the built-in rulesets by their descriptions, then `Import rules file…`
                under a separator, and no toolbar menu holds an entry that opens an AATF panel
                (the Savefile menu's Multi-team AATF…, SE-GUI-41, opens a report dialog over a
                range of teams); WHEN another
                ruleset is picked THEN the quick actions and the Cards used badge are that
                ruleset's on the next frame, Live AATF's list drops the old ruleset's rows on
                it and shows the new ruleset's when its check returns (SE-GUI-32 for the setting it
                writes); WHEN the wheel turns over the closed dropdown THEN it
                steps between the rulesets and never runs Import rules file…; WHEN Import rules
                file… picks a file the loader refuses THEN a dialog lists its findings, each
                with its line and column or key path, nothing is copied and the list is as it
                was; WHEN it picks a clean file named like no built-in THEN the file is copied
                into the data folder's `rules/`, listed under a separator with an × and picked;
                WHEN a file of the same name is imported again THEN a confirmation names the
                import it replaces, and once accepted one row remains; WHEN a file named like a
                built-in is imported THEN it is refused naming the built-in; WHEN the imported
                row's × is clicked and the confirmation accepted THEN its copy is deleted, its
                row gone, and the ruleset is the first built-in
SE-GUI-41  GIVEN the PES 17 save open under the official ruleset
           WHEN the Savefile menu's Multi-team AATF… is used
           THEN a dialog titled `Multi-team AATF (<ruleset>)` holds on one row the range's two
                combo boxes, the save's first team picked in both, and lists, under `N teams
                checked, M with issues`, every team under its id and name, its players' rows
                as Live AATF's dock words them and no suggestion, `All clean` under a team
                with none; WHEN the range is narrowed to one team THEN that team alone is
                listed; WHEN Copy to clipboard is clicked THEN the clipboard holds the list as
                plain text, its first line the ruleset, the range and the counts, then the
                team's id and name after a `>`, its `Team-wide issues` line first, then each
                player's line (or `All clean`), indented two spaces, and each row's severity,
                rule and sentence four; WHEN the first
                end is moved past the last THEN the last end follows it, and the last end's
                list starts at the first end's team; WHEN an end changes THEN the list is
                checked again only once the ends have rested 500 ms, dimmed and marked
                checking until then; WHEN another ruleset is picked THEN the old one's rows
                and summary are not drawn and Copy to clipboard is disabled until the new
                one's check returns; GIVEN a PES 21 save whose team 703's record comes before
                702's THEN the list and the range's combos run in id order, 702 above 703
SE-GUI-51  GIVEN DBC6's save (`EDIT00000000_DBC6`) open under DBC7, Multi-team AATF open
           WHEN the range is set to 851 to 897
           THEN the line above the list reads `Medal systems: GGG 4, GGSS 21, GSSSS 20,
                SSSSSS 1 · Height systems: Height 8, Manlet 14, Universal 11, GigaHeight 14`,
                and the copied text's first line is the same; WHEN the range is narrowed to
                857 THEN it reads `Medal systems: GGSS 1 · Height systems: Manlet 1`; WHEN the
                ruleset is the official one THEN the line holds the height systems alone
SE-GUI-52  GIVEN the PES 17 save open on ALL, team 701's (/3/) last player row selected and no
                control holding the keyboard
           WHEN Down is pressed
           THEN team 702's (/a/) row is selected and the detail shows its team view; WHEN Up
                is pressed THEN 701's last player is selected again; GIVEN 701's team row, the
                list's first, selected WHEN Up is pressed THEN nothing moves; GIVEN a player's
                Height field holding the keyboard WHEN Down is pressed THEN the selection
                stays; WHEN Down is pressed on a row below the list's view THEN the list
                scrolls so the selected row shows, and no widget holds the keyboard after a
                press that moved the selection; GIVEN ALL's last row selected WHEN Down is
                pressed THEN nothing moves; GIVEN the Player menu's Copy stats from dialog,
                the unsaved-changes list or team 701's slot popover open, or a pointer button
                held on a row, WHEN Down is pressed THEN the selection stays and the dialog,
                list or popover stays open; WHEN the button is released, or PgDn selects a
                player row from the slot popover, THEN Down walks one row again
SE-GUI-53  GIVEN the PES 17 save open on ALL with team 718's player 05 selected
           WHEN PgDn is pressed
           THEN the next team's row is selected, and each PgDn after it the next one's,
                the last team row stopping it; WHEN PgUp is pressed on a player THEN his own
                team's row is selected, and on a team row the one above it; GIVEN a filter
                several teams match THEN the same; GIVEN team 718 picked and its player 01
                selected WHEN PgDn is pressed four times THEN 08, 15, 22 and 23 are selected,
                the last row stopping it, and PgUp twice then selects 16 and 09
SE-GUI-54  GIVEN a player selected
           WHEN "abc" is typed into his shirt name, and into his team's short name
           THEN the records hold "ABC"
SE-GUI-55  GIVEN a stat at 60 and the Live AATF check on
           WHEN ten wheel notches take it to 70 in one go
           THEN one Undo puts back 60; GIVEN a notch on another stat between two notches
                THEN they are two entries; GIVEN a rating cell focused by Tab WHEN Space is
                pressed THEN the next frame's check reads the new rating with no other key;
                GIVEN a digit typed into the Height field THEN no check starts while the
                field holds the keyboard, and once Escape lets it go the next frame's does
SE-GUI-56  GIVEN player A's row clicked, Ctrl+C pressed, then player B's row clicked
           WHEN Ctrl+V is pressed
           THEN B's record holds what Copy stats from A writes, and one Undo restores B;
                GIVEN team 701's row copied and team 702's row selected WHEN Ctrl+V is
                pressed THEN 702 holds what a Gameplan-only import of 701's document
                writes; GIVEN preset 1 copied from its selector and preset 2 shown WHEN
                Ctrl+V is pressed THEN preset 2 equals preset 1, and likewise a formation;
                GIVEN a preset copied WHEN a player row is selected and Ctrl+V pressed
                THEN nothing changes; each Ctrl+C puts on the system clipboard a line naming
                what it copied (`Unlimited Tool Works: the stats of <name>`, `… preset 2 of
                <team>`, `… the Kick-off formation of <team>`), the last control pressed
                choosing it, the locked formation selector included, a pointer moved with no
                press not; GIVEN team 701's row copied and pasted onto itself THEN no undo
                entry is pushed and the save stays clean; GIVEN another save opened after
                Ctrl+C WHEN Ctrl+V is pressed THEN nothing changes and no undo entry is pushed;
                GIVEN the list's filter box holding the keyboard THEN Ctrl+V and Ctrl+C are
                the field's own: no record is written and the clipboard keeps A's stats
SE-GUI-57  GIVEN a player's Height field being typed in
           WHEN Up is pressed once
           THEN the value is one more; a drag over the field moves it as before
SE-GUI-58  GIVEN a team's Tactics view
           WHEN the wheel turns one notch over the preset selector on 3, or over the
                formation selector on Out-of-possession
           THEN it shows 1, or Kick-off: one step, round past the end
SE-GUI-59  GIVEN a team's lineup list and a row dragged
           WHEN the drag is held over another starter's row
           THEN that row alone is outlined in the selection colour; released off the
                list, no row is
SE-GUI-60  GIVEN a PES 19 save under the VGL 27 ruleset and a captain whose cards are under
                his tier's limit, using no COM style or A position past it (the tester's
                VGL27 Friendlies save reproduces it by hand)
           WHEN his Skills panel shows
           THEN the Cards used badge's limit counts the captain's choice card; WHEN a COM
                style past the tier's takes the choice THEN the limit drops by one
SE-GUI-61  GIVEN a player's Stronger foot toggle
           WHEN it draws
           THEN the chosen side's word is in the strong text colour and the other
                side's in the normal text grey
SE-GUI-63  GIVEN the PES 17 save open on team 734 under the official rules, Live AATF on, the
                check returned
           THEN the list opens with `Team 734 <name>`, each player's group's head reads
                `Player <id> - <number> - <name>`, `Player <id>` and the hyphens in the weak
                colour and the number and name in the strong text, a coded name in its codes' colours, and
                `Team-wide issues` is in the strong text; WHEN the dock's Copy to clipboard is
                pressed THEN the system clipboard holds the list as text, its first line naming
                the ruleset, team 734 and its error and warning counts, and a notice says so;
                WHEN Show suggestions is on and it is pressed again THEN the text ends with
                `Suggestions` and their groups, and WHEN it is turned off before the next
                check returns THEN the copied text holds no suggestion; GIVEN Live AATF in its
                window THEN the button stands at the count line's right end; GIVEN a team that
                passes THEN the list reads `All clean`, the button is not greyed out and the
                copied text is the header line and `All clean`; GIVEN Show suggestions off
                THEN no suggestion is drawn, and on THEN the list ends with `Suggestions`,
                then each player's head and his `Unused` rows (73410's `Unused: 4 of 5 paid
                cards used` first), their rule name in the normal text colour; GIVEN no
                check returned yet THEN the button is not greyed out and the copied text is
                `Checking /lit/…`, and with ALL picked and nothing selected `No team is
                open.`, the notice `Copied the Live AATF list.`; GIVEN a ruleset whose check fails on team 743 THEN the
                list holds the failure line and no `All clean`
SE-GUI-64  GIVEN the save open on a team, player 01's row dragged
           WHEN the drag is held over player 03's row
           THEN that row alone is outlined in the selection colour; held over his own row,
                or under ALL over a row of the next team, no row is; released off the list,
                no row is; GIVEN the pointer over a player row's name, no drag held THEN the
                cursor is the pointing hand; GIVEN a filter typed THEN it is the arrow
SE-GUI-65  GIVEN the Tactics view, a Starting eleven row pressed 20 points right of its centre
           WHEN it is dragged
           THEN the dragged row is drawn with that point under the pointer, not its centre
SE-GUI-69  GIVEN a team's player list, row 03 pressed 20 points right of its name's start
           WHEN it is dragged 30 points right and 40 down
           THEN the row's name is drawn 30 points right and 40 below where it stood, on top
                of the list; GIVEN a filter typed THEN a press and the same move draw nothing
                there
SE-GUI-66  GIVEN the PES 17 save open on a player whose own tier, Regular, allows 3 paid
               cards, 3 of them ticked
           WHEN Abilities shows
           THEN the Skills header reads `Cards used for` and the chip `Regular: 3 of 3`;
               `Cards used for` and the skills' labels (`Heel Trick`) paint the normal
               text colour, `Stronger foot`'s; WHEN a fourth paid card is ticked THEN the
               chip reads `Regular: 4 of 3` in the error colour
SE-GUI-62  GIVEN the PES 17 save open on a player, the keyboard moved by Tab to a rating cell
                of his Playable positions and Space pressed on it (the rating stepped)
           WHEN Ctrl+Z is pressed, the cell still holding the keyboard
           THEN the rating is back as it was and Undo is disabled; WHEN Ctrl+Y THEN the
                step is back; GIVEN instead his Name field holding the keyboard after a
                letter typed THEN Ctrl+Z leaves the editor's undo history as it was
SE-GUI-16  GIVEN the save open on a team
           WHEN Team TOML export, then import on another team with only the Stats section
                chosen, runs from the interchange dialog
           THEN the file written equals the CLI's export, and the import changes only the
                gameplay fields (the team's colours, kits and every motion staying the
                target's), with a warning list when the file's version differs; GIVEN a PES 19
                save on team 701 (/3/) and team 714's file with a 200-letter team name THEN
                the lib's refusal is drawn under the choices Stats, Aesthetics and Gameplan,
                in the error colour, Apply disabled, until Stats is unticked
SE-GUI-67  GIVEN a PES 19 save open on team 701 (/3/) and team 714's (/co/) Team TOML picked in
                the interchange dialog
           WHEN team 713 (/cm/) is selected before Apply
           THEN the dialog reads `Import into /cm/ (713)` and Apply writes 713, 701 untouched;
                WHEN instead Aesthetics is unticked and a 701 player renamed THEN Apply writes
                the file's stats and the new name stays; WHEN instead Aesthetics is unticked
                and another save opened THEN the dialog stays open with the file picked, and
                Apply writes it onto the new save; WHEN instead "Aesthetics from exports" is
                turned on THEN Apply writes no aesthetics, and once "Edit aesthetics anyway"
                is on, Apply writes them; WHEN 713's file is picked after the first was
                applied THEN the first's `Applied` line goes, and Apply writes 713's file
                onto 701 and reads `Applied cm.toml. The changes stay unsaved until Save.`
SE-GUI-68  GIVEN a PES 17 save open on team 714 (/co/)
           WHEN team 714's Team TOML from a PES 21 save, its preset 1 attacking instructions
                holding Wing Back, which PES 17 lacks, is picked; then the PES 19 squad file
                `pes19_squad.4ccs`; and, on a PES 21 save's team 701, whose players fill places
                01 to 23, a file naming players 24 and 25
           THEN the first's notes hold the line `Preset 1 attacking instructions: Wing Back`;
                the second's group `Goal celebration 1 motion · 23 players`, opened, lists
                each player's place, name and value change (`01`, his name, `127 → 122`); the
                third's notes are one group, `Empty roster place · 2 players`
SE-GUI-17  GIVEN a `.4ccs` or `.4cct` file, or a Texport
           WHEN it is opened through the import dialog
           THEN its sections import as the Team TOML import does, read-only for the legacy
                formats; WHEN Texport export runs on a team of a PES 17-21 save THEN the file
                written is a Texport of the save's version (on PES 17 under the first
                `TEXPORT0000000N` the save's folder lacks) that the import dialog reads back
                to the same team values; GIVEN a PES 15 or 16 save THEN the export is
                disabled, its tooltip naming the missing template; GIVEN `pes19_squad.4ccs`
                (23 player records) picked on a PES 19 team with 22 rostered players THEN it
                is refused alone, `pes19_squad.4ccs: 23 player records; the target team has
                22 rostered players`, with no section choice and no Apply
SE-GUI-18  GIVEN an aesthetics patch of the save's version
           WHEN Apply aesthetics patch is used
           THEN the preview lists every player it changes with old and new values, Apply is
                one undoable edit, and a patch of another version is refused with the save
                untouched, its sentence (`<path>: the patch is for PES 2021, the save is PES
                2017`) in the error colour; GIVEN the patch also naming team 999 (/zz/), which
                the save lacks, THEN `Team 999 /zz/ is not in this save: its players are
                skipped.` stands in the warning colour above the rest's preview; GIVEN a file
                that cannot be read, or is not TOML, THEN its path and the reader's or the
                parser's error are drawn, no Apply, the save untouched; WHEN a row's item is
                clicked THEN that row's player is selected (71405's row: 71405, not 71403)
                and the selector moves to his team; WHEN another save is opened, or an edit
                gives a player the patch's value, THEN the preview follows it; WHEN an edit
                follows Apply THEN the `Applied` line gives way to the preview (`The save
                already holds every value this patch writes.`); GIVEN an edit still in flight,
                the pointer held, THEN the preview drawn stays, and Apply recomputes it first,
                the edit kept and the patch's values on it
SE-GUI-19  GIVEN a player with name color codes
           WHEN settings.toml generation runs on his folder from the view
           THEN the file holds the explicit name string with its codes (never `name = true`,
                the Export upgrader's folder-matching rule)
SE-GUI-33  GIVEN a player whose record has hair length 5 and iris 7
           WHEN settings.toml generation, then Generate in-game face, run on his folder from
                the view
           THEN settings.toml holds no face, hair or iris key, and ingame_face.toml holds
                iris_color = 7, [hair] length = 5 and every other key the save's version holds
SE-GUI-34  manual  GIVEN a PES 21 player whose record has a non-default in-game face (long red
                         hair, blue iris, a slider moved), his settings.toml and
                         ingame_face.toml the view generated from it, then his record changed
                         to another non-default look (short blond hair, green iris), the
                         folder compiled into the test slot and its patch applied to the save
                   WHEN he is opened in PES 21's Edit mode
                   THEN he has the generated look (long red hair, blue iris), neither the
                        changed record's nor the default face
SE-GUI-20  withdrawn: no save is stripped, so no Strip again window opens (step 8.36b)
SE-GUI-21  manual  GIVEN the save's game running
                   WHEN Save is clicked
                   THEN a dialog naming the game says to use Edit mode's Data Management >
                        Load before leaving it if it is open, and that a closed Edit mode shows
                        the changes when opened; Cancel leaves the file unchanged; Save writes
                        it, and the edit then shows in game without a restart, by opening Edit
                        mode, or by Load inside it
SE-GUI-22  GIVEN the save open with "Aesthetics from exports" on and "Edit aesthetics
                anyway" off (every start of the tool)
           WHEN the Appearance section (open), the team operations menu, the Savefile menu
                and the import dialog are drawn
           THEN the FPC toggle and the team-wide FPC switch, the boots and gloves assignment,
                Clear visual flags, Fix database and the imports' aesthetics section
                are disabled, each with the ownership line as its tooltip, while Apply
                aesthetics patch is enabled; WHEN the ownership line's link is clicked, on a
                PES 17 save too, THEN the patch panel opens
SE-GUI-23  GIVEN the save open on a team and the aesthetics editable (the setting off, or on
                with "Edit aesthetics anyway" switched on)
           WHEN a team operation runs from its menu (Set edited flag, Set boots and gloves for
                everyone in incremental mode, Set player names to positions, Clear visual flags,
                the last also by Ctrl+F)
           THEN every player of the team carries the operation's values, as one undoable edit,
                and the saved file holds them
SE-GUI-35  GIVEN the save open with "Aesthetics from exports" off (the default)
           WHEN the Appearance section (open), the team operations menu, the Savefile menu
                and the import dialog are drawn
           THEN no ownership line and no "Edit aesthetics anyway" switch are drawn, every
                path SE-GUI-22 names is enabled, and an Appearance edit lands in the record
                as an ability edit does; GIVEN the setting switched on in the settings menu
                THEN the next frame draws the switch, off, and the paths disabled
SE-GUI-24  GIVEN the save open on a player
           WHEN Set all stats to N, Bump all stats by N, Copy stats from another player or Swap
                stats with him runs from the player operations
           THEN his ability values are the operation's result (Set and Bump clamped to 40 to
                99, weak foot, form and injury untouched; Copy and Swap over the parity row's
                field set, positions, skills and COM styles included), as one undoable edit,
                and the saved file holds them; WHEN Bump all stats is opened again THEN its
                field holds the last N used
SE-GUI-25  GIVEN the team view's Tactics on preset 1
           WHEN the preset bar's Copy/swap, which lists Copy to 2, Copy to 3, Swap with 2 and
                Swap with 3, runs Copy to 2, then the formation bar's, on Kick-off, runs Swap
                with In-possession
           THEN preset 2 equals preset 1 before the swap, the two formations are exchanged, each
                as one undoable edit
SE-GUI-26  GIVEN two PES 21 saves and one PES 19 save under the test's Documents folder, the
                 selector at PES 21, and one of the three opened earlier in the session
           WHEN the Open menu is drawn
           THEN it lists the three discovered saves, each labelled by its version alone, the
                first PES 21 one with Ctrl+1 beside it and the PES 19 one with Ctrl+9, and the
                recent file above the file picker; WHEN the menu is closed and Ctrl+9 is pressed
                THEN the PES 19 save opens, and WHEN Ctrl+7 is pressed (no PES 17 save found)
                THEN the file picker opens; WHEN Ctrl+Shift+9 is pressed, or Ctrl+9 while the
                Name field holds the keyboard, THEN nothing opens and no prompt shows; WHEN
                the menu's Choose file is used THEN the picker starts in the newest PES 21
                save's folder, and with the selector at PES 18, which has no save there, in
                no folder; WHEN Save as writes a new
                path THEN the file replaced, if any, is kept once as its .bak
SE-GUI-27  GIVEN the save open
           WHEN Output rosters to TSV runs
           THEN the file written holds, for every team, its name on a line of its own, then one
                tab-separated row per player (number, name, registered position, Finishing as
                the rating, captain mark, height, playing style, skills, COM styles), as
                4ccEditor's output does; WHEN
                Fix database is chosen THEN a
                confirmation dialog precedes it, its question naming only the repairs the
                save's version has, and cancelling changes nothing
SE-GUI-28  GIVEN the selector at PES 19
           WHEN a PES 21 save is opened
           THEN the status bar shows a Warning notice naming both versions, the save keeps its
                own version (its codec and its widgets' gating are PES 21's), and the selector's
                setting is unchanged; WHEN the notice's "Switch to PES 2021" is clicked THEN the
                selector's setting is PES 21 and the notice is gone, and the selector set back
                to PES 19 by hand shows nothing; WHEN instead the selector is set to PES 17 by
                hand THEN the notice names PES 2017 and PES 2021, set to PES 21 by hand it is
                gone, and set back to PES 19 it shows again; WHEN instead its dismiss is
                clicked THEN the notice is gone and the setting unchanged, and no selector
                change brings it back until another save is opened; GIVEN the selector at
                PES 21 WHEN the PES 21 save is opened, saved (its save notice taking the bar)
                and the selector then set to PES 19 THEN the warning shows; GIVEN another
                registered tool reporting a version-change blocker THEN "Switch to PES 2021" is
                drawn disabled, its hover the blocking tool's "<label>: <reason>", and the
                setting stays PES 19 and the notice stays
SE-GUI-29  manual  GIVEN a Texport written by SE-GUI-17's export from a PES 17 save, and (after
           0.0.1) from a PES 18, 19, 20 and 21 save
           WHEN it is imported through the game's own team import, in that version
           THEN the game accepts it and the imported team shows the exported squad and its
                players' names
SE-GUI-30  GIVEN the PES 17 save open on a team with violations under the active ruleset
           WHEN Live AATF is switched on
           THEN the docked panel reads `Checking <team>…` until the first check returns, then
                lists the team's violations under `N issues · <ruleset>`, its `Team-wide
                issues` group above every player's, a player's row's tooltip `Click to switch
                to this player`, a `FieldedNotA` row's `Click to show him on the team's pitch` and a
                team-wide row's none, and clicking a row
                selects that player (a `FieldedNotA` row his team's row, the named preset
                shown on a formation carrying the cross), a click on a team-wide row
                selecting nothing; WHEN an edit fixes one of them THEN the list stays,
                marked `checking…`, until the check returns, its row then gone and N one less,
                and an undo brings it back; WHEN another team is opened, or another ruleset
                picked in the toolbar's dropdown, THEN the list and N are that team's, or that
                ruleset's; the panel checks
                the selected row's team (a player's or a team row's own, another team's
                when the filter lists it), else the selector's (with ALL picked and nothing
                selected: `No team is open.`);
                GIVEN the PES 15 save with a team that passes THEN the header reads `<team>
                passes <ruleset> with no issues.`, and WHEN an edit breaks a rule and the last one is
                fixed THEN it reads so again; WHEN the tool is reopened THEN Live AATF is
                still on
SE-GUI-47  GIVEN the PES 17 save open on team 718 under DBC7, Live AATF on, one bench player's
                Speed raised one past his Regular tier's target, the check returned
           THEN the starting eleven's players with rows stand in the dock's left column and the
                bench player's in its right, each under the line naming him, both columns from
                the same top (718 has no rows of its own under DBC7: where a team has some, they
                stand above both columns at the full width, which `team_widgets`' tests pin)
SE-GUI-48  GIVEN SE-GUI-47's save, team, edit and check, the list in the dock
           WHEN the pop-out icon at the dock header's right end is clicked
           THEN the dock is gone and a window titled `Live AATF` holds the dock's count line
                and the same rows, the players' groups in one column in roster order (the
                bench player 71801's, slot 0, above the starter 71808's, slot 7),
                its dock icon left of its close button, where it stays the frames after a
                touch on the window; WHEN a row of 71801's group is clicked in the window THEN
                71801 (718's slot 0) is selected; WHEN the dock icon is clicked THEN the
                window is gone and the dock is back with its two columns; WHEN the list is
                popped out again and the window's close button clicked THEN Live AATF is off
                (the toolbar's button unfilled, no list, no marks), and WHEN it is turned on
                again THEN the list opens in the window, `live_window` read back from the
                settings
SE-GUI-49  GIVEN SE-GUI-48's save, team, edit and check, the list in the dock
           WHEN the close button right of the dock header's pop-out icon is clicked
           THEN Live AATF is off (the toolbar's button unfilled, no list, no marks), and WHEN
                it is turned on again THEN the list opens in the dock, `live_window` still off;
                the dock's close button is the window's in size and stroke, and stands right of
                the pop-out icon by the window's spacing between its dock icon and close button
SE-GUI-50  GIVEN the PES 17 save open on team 718 in the team view, the sidebar on PES 2017,
                and the sidebar's exe check reporting that the PES folder does not exist
           WHEN the view is drawn
           THEN a `Set PES path` button stands in the pitch's bottom-right corner, its
                left edge as far from the bottom penalty box's right line as its right edge is
                from the pitch's right edge and its bottom from the pitch's bottom, within half
                a point, and every starter's portrait is the placeholder; WHEN the check reports the folder holding `PES2017.exe`
                THEN the button is gone; WHEN it reports `PES2019.exe` instead THEN it is
                back; WHEN it is clicked and the folder picker returns
                `<test folder>/Pro Evolution Soccer 2017` THEN the common settings'
                `pes_folder_path` reads `<test folder>/Pro Evolution Soccer 20**`. Manual: the
                click opens the native folder picker. (The check itself, a folder's listing
                on a change and on the window's focus, is the sidebar's, ST-SIDE-03.)
SE-GUI-36  GIVEN SE-GUI-30's save and team, Live AATF on and its first check returned, one
                 player of the team breaking the Height rule alone and another breaking none
           WHEN the view is drawn
           THEN the toolbar's Live AATF is a button whose label holds the play sign, the list
                shows a cross beside the first player and a check mark beside the second; WHEN
                the first is selected THEN a cross stands beside the Height box, its tooltip
                the dock's line for that violation, and no other field carries one; WHEN an edit fixes the
                height and the check returns THEN both marks are a check mark and the cross
                beside the box is gone; WHEN Live AATF is switched off THEN no mark is drawn;
                WHEN another ruleset is picked THEN the old ruleset's result marks no row and
                crosses no field, and once the new one's check returns its marks are drawn
SE-GUI-37  GIVEN the PES 17 save open on team 701
           WHEN the list is drawn
           THEN a team row heads the team's players, naming 701 and the team with its colours
                1-2 as swatches; WHEN it is clicked THEN the detail shows the team view (its
                name, short name, colours, manager and stadium ids, the Kit slots… button, the
                Tactics), and the quick actions and the Player menu's entries are disabled, the
                Make buttons' tooltip "The make buttons act on a player: pick a player row";
                WHEN the team's short name is edited THEN the team row carries the unsaved dot
                and none of its players does; WHEN the team row is dragged onto a player row
                THEN nothing moves; WHEN a player row is clicked THEN the player view shows,
                its first line his id then his name, under it General info, Skills and
                Ability stats side by side left to right at the 1280 × 780 window, their tops
                level, and Appearance closed below them; and no view draws "Save editor"
SE-GUI-38  GIVEN the save, player 70101's name set to the colour code `\x11cff0000ff` then `AB`,
                and his shirt name to 20 letters, before it is opened
           WHEN he is selected
           THEN the Name label reads "Name (12/61)", its count in the theme's quiet count grey (`#5a5a5a`, a
                disabled card label's, on the dark theme; `#9f9f9f` on the light one), and the Shirt
                label "Shirt (20/18)", the shirt field tinted with the warning colour, its count
                in the error colour, and its hover the "PES 2019
                and older show 18 letters" sentence; WHEN a letter is deleted from the shirt
                name THEN it reads "Shirt (19/18)", still tinted, and a letter typed after it is
                refused; WHEN another is deleted THEN "(18/18)" is in the warning colour and the
                field untinted, and at "(17/18)" the count is in that grey; WHEN 47 bytes are typed as his name THEN "Name (47/61)", tinted, its hover
                the cut PES 2019 makes; GIVEN the PES 16 save THEN a player's Shirt label's
                limit is 16 and its Name label's 46
SE-GUI-39  GIVEN the PES 21 save with team 718's preset 1, team 722's three presets and team
                701's Kick-off set to the formations of the draft's PES 21 sample save (the
                fixture's teams carry a fresh save's default formations), open on team 718
                (/f/), its Tactics on preset 1, fluid formation on
           WHEN Kick-off, In possession and Out of possession are shown in turn
           THEN the pitch's bottom-left corner reads `3-3-2-2`, `3-3-4` and `3-4-2-1`, as the
                game's formation screen does; WHEN team 722 (/gd/) is shown THEN its presets 1, 2
                and 3 read `4-2-1-3`, `4-3-1-2` and `4-3-2-1`; WHEN team 701 (/3/) is shown THEN
                it reads `3-3-1-3`, and WHEN its AMF's position is changed to CMF THEN `3-4-3`,
                and WHEN he is dragged to another spot instead THEN `3-3-1-3` still
SE-GUI-40  GIVEN the PES 21 save open on a player whose registered position is not SS
           WHEN the SS label of his Playable positions is clicked
           THEN his registered position is SS (General info's combo reads it, the SS cell is
                outlined) as one undoable edit, his ratings unchanged; WHEN Undo is invoked
                THEN the earlier position is back
SE-GUI-31  GIVEN the common settings' FPC hide IDs set to 60 (boots) and 12 (gloves), "Edit
                aesthetics anyway" switched on, and the player's Appearance section open
           WHEN Hide default body is used on a player whose boots and gloves IDs are 0
           THEN his record holds boots 60 and gloves 12 and Appearance reads "The default body is currently hidden";
                WHEN Show default body is used THEN both are 0 again; GIVEN a player whose boots ID is
                55 THEN Appearance reads "The default body is currently shown"; WHEN Ctrl+I, the Player menu's Toggle
                FPC settings or the Team menu's Set and Remove FPC invisibility run THEN they
                write and clear 60 and 12 the same, and a player holding 55 and 11 keeps them
SE-GUI-32  GIVEN the save open and VTL11 picked in the toolbar's ruleset dropdown (SE-GUI-15)
           WHEN the tool is closed and reopened
           THEN the `aatf_ruleset` setting reads `vtl11`, the tool's other keys unchanged, the
                dropdown shows VTL11's description and the quick actions are VTL11's
                (SE-GUI-09), the settings menu's Save editor section holds no rules file row,
                and `check-aatf` run without `--rules` checks against VTL11; GIVEN the setting
                naming an imported file no longer in the data folder's `rules/` THEN at start
                the ruleset is the first built-in one and a warning notice says so, and no
                quick action, badge or list shows a load error; GIVEN it naming an imported
                file still there (Cup Rules) THEN at start the dropdown shows Cup Rules with
                no notice, and opened it lists the built-ins, then Cup Rules and its remove
                row, then Import rules file…
SE-GUI-44  GIVEN a save open and DBC7, which carries a logo and a page, the active ruleset
           WHEN the view is drawn at the 1280-wide window
           THEN the logo stands 28 points high, its right edge 14 points from the window's
                and its top 8 from the view's, and every other widget stands where it does
                under the same ruleset without a logo; its tooltip reads DBC7's name, a blank line, "Click
                here to open the event's page:" and its page, each on its line, and a click
                opens the page; GIVEN a ruleset without a logo THEN nothing is drawn there;
                GIVEN a window narrow enough for the toolbar to reach it THEN it is not
                drawn
SE-AATF-08 GIVEN SE-AATF-03's save and the command started from a folder holding no file
                named `vgl27`
           WHEN check-aatf is run with --rules vgl27, then with --rules nosuch, then with
                --rules naming by its path a file called `vgl27` that is SE-AATF-07's
                official ruleset
           THEN the first prints SE-AATF-03's VGL27 violation; the second checks nothing,
                exits 2 and its error names `nosuch` and every embedded ruleset's stem; the
                third checks against the file: the VGL27 violation is not printed
SE-REP-01  GIVEN a repro bundle (`plans/core/architecture.md` "Repro bundles") whose journal
                opens the save, sets player 70101's Height from the value the save holds to
                another and writes the save, the save so written its kept output
           WHEN `save-editor replay` is run on it
           THEN it prints that the write matches and exits 0
SE-REP-02  GIVEN SE-REP-01's bundle with the Height step taken out of its journal
           WHEN `save-editor replay` is run on it
           THEN it prints that the write differs, naming player 70101's Height with the
                replayed value and the written one, and exits 1
SE-REP-03  GIVEN SE-REP-01's bundle with the Height step's old value one above the value the
                save holds
           WHEN `save-editor replay` is run on it
           THEN it stops at that step, printing its line, the value the save holds and the
                one the step expected; it compares no write and exits 1
SE-REP-04  GIVEN SE-REP-01's bundle whose journal writes the save to the same path a second
                time after the Height step, the second write its kept output
           WHEN `save-editor replay` is run on it
           THEN it prints the first write as replaced by a later one and not compared, the
                second as matching, and exits 0
SE-REP-05  GIVEN SE-REP-01's bundle recorded by another Studio version
           WHEN `save-editor replay` is run on it
           THEN it prints both versions, replays the bundle as in SE-REP-01 and exits 0
SE-REP-06  GIVEN a file that is not a zip, then SE-REP-01's bundle with one byte of its kept
                input changed
           WHEN `save-editor replay` is run on each
           THEN each prints why it is not a bundle it can replay (not a zip; the input's
                bytes do not match the journal's SHA-256), replays nothing and exits 2
```

The full-look patch's `apply-patch` scenarios (SE-PAT-07 to 11) moved to
`spec/save_editor.md` "Acceptance" (worklog 8.36c).
