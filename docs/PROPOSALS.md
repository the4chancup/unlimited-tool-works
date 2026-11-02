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

## P40 — The Music export editor reorders its players

From the RigDJ audit (10.0h, 2026-10-27). The editor lists and writes the players in the
file's order, which is the order of Rigdio's buttons (`plans/music_export_editor.md` "What
the editor does", the slot list). 27 of the 50 RigDJ-written exports on the maintainer's
machine have their players out of sorted order, so managers reorder them by hand in a text
editor; RigDJ re-sorted them on every load.

Adds: up/down buttons (or a drag) on a player in the slot list, so a manager sets the button
order the streamer sees without leaving the editor. Costs: two buttons beside Rename and
Delete, and the drag the song rows already plan reused for the slot list; nothing new in the
format.

## P41 — The Kit config editor shows what a collar, shorts model or pattern looks like

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

## P42 — The color picker's hex field takes Kit Manager's spellings too

From the Kit Manager audit (12.0a, 2026-10-27). Kit makers paste colors from the tools they
used with Kit Manager, which accepted `0xRRGGBB` and bare `RRGGBB` (and nothing else, not
even `#RRGGBB`). The shared `color_tools` picker's hex field is not specified beyond the
TOML's `#RRGGBB`.

Adds: the picker's hex field accepts `#RRGGBB`, `0xRRGGBB` and `RRGGBB` alike in every tool
that uses it, so a pasted value from an old habit lands instead of being refused. Costs: one
parse with three prefixes in `color_tools`; the field still shows `#RRGGBB`.

## P49 — A Fielded position row's tooltip says where its click goes

From Astra's review of Live AATF (cv28-cv32, 2026-11-01). Every player's row in Live AATF
says `Click to switch to this player` on hover (SE-GUI-30), and a click opens that player,
except a `Fielded position` row's: its click opens his team's row, the preset it names shown
on a formation carrying the cross on his pitch marker, since the issue is where he stands in
the tactics, not in his own record.

Adds: that row says `Click to show him on the team's pitch` (or the like) instead, so the
jump to the team view is expected. Costs: a second tooltip constant in `team_widgets`
`violations.rs`, chosen by the row's rule, and its line in the draft and SE-GUI-30.

## P50 — Undo and the other shortcuts work while a button holds the keyboard

From the lead's built-app look at candidate 0.0.1h (2026-11-02). A member who reaches a
control with Tab and presses Space (a rating cell, a checkbox, a radio) leaves the keyboard on
it. Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z, Ctrl+F, Ctrl+I, Ctrl+B, Ctrl+O and the version keys then do
nothing until he presses Escape or clicks elsewhere. The toolbar holds them off while any
widget has the keyboard, for a text field's own undo. Ctrl+C and Ctrl+V are held off only by
a text field, so a focused button already lets them through, and Tab + Space settling (8.40)
makes this path more common: the Live AATF row appears, and Ctrl+Z does not take it back.

Adds: those shortcuts held off only while a text field (or a number being typed) holds the
keyboard, the rule Ctrl+C and Ctrl+V already follow. Costs: one condition in the save
editor's toolbar (`text_edit_focused` in place of `egui_wants_keyboard_input` for those
keys), and a test with a Tab-focused rating cell; nothing for a member to learn.
