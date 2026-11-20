# Unlimited Tool Works: String editor plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The String editor (`crates/tools/string_editor`, tool id `string-editor`, sidebar label
"String editor") edits the games' text file, `all.str`, which holds the menu texts and
loading-screen tips each game shows. It replaces the PES2021 STR Editor, a closed .NET
program the community uses today. It is low priority: this plan is written ahead of any
phase that builds it.

It opens the file, shows its sections and their numbered strings, and saves the edited file.
Reading and writing the format belong to the [`pes_strings`](libs/pes_strings.md) lib crate,
which the Save editor also uses to show the game's own texts.

The cup's loading-screen tips are not edited here: the [Gametips editor](gametips_editor.md)
does that, from the members' submissions. The String editor has one tie to it, the Gametips
editor's button that opens its result here.

The rest of this plan (the editor's view and operations, the format details it relies on) is
written from a lookup of the PES2021 STR Editor's code (`4cc-studio-temp/str_re/out/findings.md`).
