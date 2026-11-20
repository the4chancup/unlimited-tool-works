# Unlimited Tool Works: Gametips editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The Gametips editor (`crates/tools/gametips_editor`, tool id `gametips-editor`, sidebar label
"Gametips editor") puts the cup's own loading-screen tips into the game, in place of Konami's.
It replaces Noteperson's gametips scripts (`writeTips.py` and the converter beside it). It is
low priority: this plan is written ahead of any phase that builds it.

Its input is the raw file the tips submission form produces: one entry per submission, written
as `timestamp%sep%title%sep%body%sep%`, where a body may run over several lines. The file holds
what the form received, so it also holds test entries and the same tip sent twice. The organizer
curates it in the tool (drops entries, fixes text, picks which tips to use), and the tool writes
the chosen tips into the game's tip strings in `all.str`, through the same string-file lib crate
the [String editor](string_editor.md) uses.

**Two tools, not one with two modes.** The String editor starts from the game's file and shows
its sections; this tool starts from the submissions and treats the game's file as where they
go. Their users differ too: any member may edit a string now and then, while the tips are
set once a cup by its organizer. In one tool, the tips workflow would sit behind a mode built
for the other job. What they share, the file format, is in the lib crate. A button here
opens the written file in the String editor, for a look at the result or an edit the curation
did not cover.

The rest of this plan (curation, how tips fill the game's slots, the output and how it reaches
the cup's files) is written from a lookup of the scripts (`4cc-studio-temp/str_re/out/findings.md`).
