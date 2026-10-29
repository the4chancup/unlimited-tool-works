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


## P44 — The help window's text column capped at a reading width

From the maintainer's look at the help (2026-10-29: paragraphs read as one extremely long
line). The window opens at 90% of the program's window, so a topic's lines run about 200
characters across its pane at 1280 points; print and web typography keep a line to roughly
60-90 characters, past which the eye loses the next line's start.

Adds: a topic's text, lists and code blocks wrap at about 720 points (some 110 characters at
the body size), left-aligned in the pane, the rest of the pane empty; the Messages table keeps
the pane's full width. Costs: one `ui.set_max_width` in the topic's column and the drafts'
`.help-topic` `max-width`; wide windows show unused room right of the text.
