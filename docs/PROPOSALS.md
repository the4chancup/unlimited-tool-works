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



## P45 — A ruleset carries its event's logo and page, drawn at the Save editor's top right

From the maintainer (2026-10-29, the placement and the link 2026-10-30). Other cups (DBC,
VGL, VTL) run their events on their own rules files, and 0.0.1 is DBC's build.

Adds: an organizer embeds a small logo in the `.4ccrules` file, and optionally the event's
page address, both in its data section (a base64 PNG and a URL under two keys), never in the
logic section, so the logic's hash and version are untouched. While the Save editor's active
ruleset carries a logo, the editor draws it in its own top-right corner, mirroring the
sidebar's clover: as far from the window's right edge and top as the clover's square is from
the left edge and top (12 and 6 points in the drafts), 32 points high, its width following
the picture's (at most 96 points, so a wide logo still reads). It is drawn over the empty
corner and moves nothing: no row is pushed down, no column narrowed. Measured in the drafts
at the default 1280-wide window, the toolbar at its widest (Live AATF on, Show suggestions,
the unsaved label, DBC7's description) ends 170 points short of the logo's square, and the
right column's title starts about 20 points under it. On a window too narrow for that room
(the toolbar reaching the logo's place) the logo is not drawn, rather than covering a
control. A ruleset with a page makes the logo a link: a click opens the page in the browser,
its tooltip names the event (the ruleset's name) and the address; without a page the
tooltip names the event alone. The Ruleset editor gets a Logo field (pick a PNG, preview it
at its drawn size, remove it) and a Page field; `check-aatf` ignores both. A member of that
cup sees his event's mark and reaches its page in one click; the sidebar's clover, the
suite's identity and About's target, never changes.

Costs: a texture where the rest of the UI draws font glyphs (the decoder is not new: `image`
is a workspace dependency, through `dds_convert`). The picture keeps one look on both themes;
logos are usually made for any background, so the preview shows it on the dark and the light
panel and nothing more. The field caps the file (64 KB, refused with a finding otherwise). A
text file holding a base64 blob is harder to read and to diff by hand. Two new keys in the
rules format, the reader, the writer, the Ruleset editor's form and its tests, the page
address checked to be `http` or `https` before the program opens it. After 0.0.1: it changes
the rules format.
