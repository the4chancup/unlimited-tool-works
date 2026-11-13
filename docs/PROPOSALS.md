# Proposals for the maintainer

Ideas nobody asked for that would make the suite easier to use: a button, a shortcut, a
default, a message that saves a member a step. Any agent adds one when it thinks of it while
working (`AGENTS.md` "When the plan has gaps"); the maintainer vetoes or accepts entries as
they come across them. A proposal is not built while it is unruled: plan text that already
describes one says so and points here, and the phase that would build it lists the entry in
its turn report. A vetoed entry leaves with a one-line `DECISIONS.md` entry, so it is not
proposed again; an accepted one leaves into its plan section and a `DECISIONS.md` entry.

Each entry: a heading, where it came from, what it adds and for whom, and what it costs (code,
surface, things a member must learn). Newest last.

The first idea of this kind, the Export upgrader's "Move to exports and compile" button, was
asked in `QUESTIONS.md` before this file existed and approved.

P16 to P38 were ruled on 2026-10-26 (`DECISIONS.md`'s entries "Usability proposals P17-P24
and P32-P38 accepted", "A folder per PES version", "Red's settings are not imported on
first run" and "Live AATF stays off by default"). P39 (a window closed maximized reopens
maximized) was accepted on 2026-10-27 (`DECISIONS.md` "A window closed maximized reopens
maximized"). P43 (stats named as each PES version names them) was accepted on 2026-10-29
(`DECISIONS.md` "Stats are named as the save's PES version names them").
P44 (the help's text column capped at a reading width) was accepted on 2026-10-29
(`DECISIONS.md` "The help's text wraps at 720 points").
P45 (a ruleset's event logo and page, at the Save editor's top right) was accepted for
0.0.1 on 2026-10-30 (`DECISIONS.md` "A ruleset carries its event's logo and page").
P46 (a check flagging an illegal formation under a cup that forbids them) was accepted for
0.0.2 on 2026-10-31 (`DECISIONS.md` "The illegal formation check is the stock game's
refusal, in the host").
P47 (a height finding names its players, and the nearest quota system) was accepted on
2026-10-31 (`DECISIONS.md` "A height finding names its players, and the nearest quota system
(P47)").
P48 (Multi-team AATF counts the teams under each medal and height system) was accepted
for 0.0.1's next candidate on 2026-11-01 (`DECISIONS.md` "Multi-team AATF counts the
teams under each system (P48)").
P49 (a Fielded position row's tooltip says where its click goes) and P50 (Undo and the
other shortcuts work while a button holds the keyboard) were accepted for 0.0.1 on
2026-11-02 (`DECISIONS.md` "A Fielded position row says where its click goes (P49)" and
"Only a text field holds the shortcuts off (P50)").

## P40: The Music export editor reorders its players

From the RigDJ audit (10.0h, 2026-10-27). The editor lists and writes the players in the
file's order, which is the order of Rigdio's buttons (`plans/music_export_editor.md` "What
the editor does", the slot list). 27 of the 50 RigDJ-written exports on the maintainer's
machine have their players out of sorted order, so managers reorder them by hand in a text
editor; RigDJ re-sorted them on every load.

Adds: up/down buttons (or a drag) on a player in the slot list, so a manager sets the button
order the streamer sees without leaving the editor. Costs: two buttons beside Rename and
Delete, and the drag the song rows already plan reused for the slot list; nothing new in the
format.

## P41: The Kit config editor shows what a collar, shorts model or pattern looks like

From the Kit Manager audit (12.0a, 2026-10-27). Kit Manager's preview was a catalogue: one
prerendered picture per collar (1–106), shorts model (0–17), shirt pattern (0–13), sleeve
type and shirt model, shown as the field changed. The planned preview draws the kit's
colors, positions and real textures (`plans/kit_config_editor.md` "Preview panel"), so it
drops the one thing Kit Manager's showed: what collar 37 looks like before picking it.

Adds: a thumbnail beside the Collar, Winter collar, Shorts model and Shirt pattern fields,
or a gallery to pick from, for the kit maker who knows the look and not the number. Costs:
the pictures themselves, which Kit Manager's bitmaps cannot supply (PES 2016's models, 51 MB,
not ours to ship): renders of each version's stock collars and shorts, made once per version
(up to 131 collars on PES 2021), plus a gallery widget.

## P42: The color picker's hex field takes Kit Manager's spellings too

From the Kit Manager audit (12.0a, 2026-10-27). Kit makers paste colors from the tools they
used with Kit Manager, which accepted `0xRRGGBB` and bare `RRGGBB` (and nothing else, not
even `#RRGGBB`). The shared `color_tools` picker's hex field is not specified beyond the
TOML's `#RRGGBB`.

Adds: the picker's hex field accepts `#RRGGBB`, `0xRRGGBB` and `RRGGBB` alike in every tool
that uses it, so a pasted value from an old habit lands instead of being refused. Costs: one
parse with three prefixes in `color_tools`; the field still shows `#RRGGBB`.

## P51: A button that goes back to the version an update replaced

From the first plan (2026-09-13, `plans/core/distribution.md` "Rollback"), written after Red,
which keeps the whole old folder when it updates. The maintainer never asked for it, and on
2026-11-12, with its slice next in line for 0.0.2, did not remember doing so; it waits here
for a ruling instead.

What is built: Update now leaves the program it replaced in the bundle's `old/` folder, one
version deep. A member can go back by hand: close the program and copy the program file from
`old/` over the one in the bundle's folder.

Adds: a settings menu button, "Roll back to previous version", that swaps the two programs
and restarts, for a member whose new version is broken. No member can reach it in 0.0.2:
0.0.1 cannot update itself, so the first `old/` folder appears when 0.0.2 updates to its
successor, and the button that then matters is the successor's. Only a 0.0.2 proof-of-concept
build updated to the final 0.0.2 would show it.

Costs: the plan's three lines leave most of it open, each a thing to build and a member to
learn. A confirmation, since the program restarts. What the button reads after a rollback,
when `old/` holds the newer program: naming the version needs a version file written beside
the program in `old/`. The state after the swap when a tool's close prompt cancels the
restart: a reminder like `Update ready`, and no update offered meanwhile. Only the program
file goes back: the launchers and the readme stay the newer version's, and a launcher that
passes an option the older program lacks fails. A cheaper cover for the same case is one
sentence in the help's Updates chapter giving the by-hand steps above.

## P52: The window checks for updates at every start

From reading the check's schedule for the command line's notice (0.0.2-cli, 2026-11-12). The
window checks when `last_update_check` is older than the interval (120 minutes by default),
at startup as while it stays open, and the offer it finds lives for that session only. So a
member who sees `Update available`, closes the program and opens it again within the interval
finds no offer until the interval has passed: the plan's "the next check offers it again" is
then up to two hours away. The command line's notice makes the case more likely, since its
line sends the member to the window.

Adds: the window checks once at every start, when `check_for_updates` is on, and the interval
governs only the checks that follow while it stays open. An offer seen once is there again at
the next start. Costs: one request to GitHub per start instead of at most one per interval
(GitHub allows 60 an hour per address without a login); `last_update_check` stops deciding
the first check, and the plan's step 1 and its tests change with it.

## P53: Every version-gated field's label is dimmed with its control

From the ask of 2026-11-03 that the name of an ability stat the save's version lacks be
dimmed (0.0.2-statlbl, which does that for the Ability stats column alone). The Save editor
has other fields a version lacks, each drawn disabled in place with its `PES NN+` tooltip
while its label keeps the normal text colour: Stronger hand in General info before PES 2020,
the Cutscenes group's star before PES 2019 and playing attitude before PES 2020, and on the
team view of a PES 2016 save the colours, the Manager ID and the kit slots.

Adds: one rule for the whole editor, "a field the save's version lacks has its label dimmed
with its control", so a member reads every such row the same way. Costs: the label's draw
call at each of those places goes through the disabled-widget helper, and each gets its
line in a test; a member who reads a dimmed label as "locked by the aesthetics switch" (the
Appearance section dims for that reason too) has one more thing dimmed there.

## P55: The Team compiler's help as two topics

From the text rereading of 2026-11-13 (worklog 0.0.2-text, `sh_tc.full.md`). The Team
compiler's help is one topic, "Running from a terminal", of about 1,200 lines. Two thirds of
it is what goes in an export and what each file becomes, which a member using the window
needs as much as one using a terminal and would not look for under that title; the
run-level text a member opens the page for (the exports folder, `--export`, `--no-deploy`,
the modes, the exit codes) starts after 750 lines of per-file rules.

Adds: two topics, "What goes in an export" (from "What `compile` builds" through "Kit
config values the game ignores or cannot hold") and "Running from a terminal" (the commands,
which exports a run reads, install errors, `upgrade-dpfl`, the modes, `multicpk_mode`, exit
codes, the aesthetics patch), each findable by its title. Costs: the text moves between two
files, the topic list in the tool's `lib.rs` and the help tests gain an entry, and a link
or search hit a member saved to the old topic's second half lands on the new one.

## P56: The window can say the game's Edit mode is closed

From the same rereading (`sh_ui_tc.proposals.md` P4). While the PES the save belongs to is
running, a compile skips the save step unless it is run with `--edit-mode-closed`. The
window's Compile button cannot add the flag, so from the window the only way to get the save
written is to close the game, and the `savefile_skipped_pes_running` message now says so.

Adds: when a compile from the window skips the save step for that reason, the finding (or
a dialog) offers "Edit mode is closed, write the save", which runs the save step alone with
the flag's meaning. Costs: one more confirmation whose wrong answer loses the save's changes
when the member leaves Edit mode (the reason the flag exists), a save-only run path from the
window, and its tests.
