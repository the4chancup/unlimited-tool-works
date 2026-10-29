# Unlimited Tool Works

One suite for every tool the 4chan Cup community uses to run its PES (2015-2021) cups: a
Team compiler (aesthetics exports to CPK archives), a save editor, and the smaller tools around
them (kit configs, referees, balls, music, match tracking, model conversion), in a single binary
that is both a GUI and a CLI.

**Status: in development, not released yet.** Six of twenty phases are done:

- **Phases 1 and 2**, the foundations: readers and writers for the game's files (CPK, FPK,
  FTEX, DDS, WESYS, uniparam, zip/7z), both model families (FMDL for PES 18-21, `.model`/`.mtl`
  for PES 15-17) with conversion between versions, kit configs, the teams list, and the
  savefile (real saves decrypted, edited and rewritten byte for byte).
- **Phases 3 and 4**, the Team compiler: a cup's aesthetics exports checked and compiled into
  CPK archives for PES 2015 to 2021 (faces, boots, gloves, kits, portraits, referees), compared
  with the community's current compiler on its library of real exports.
- **Phase 5**, the savefile side: the settings an export carries written into a save, the AATF
  rules checked on a save's teams (the official rulesets and the invitationals' own), and the
  save editor's command line (team exports and imports, comparisons).
- **Phase 6**, the Export upgrader: an export in the old layout migrated once to the new one.

Phase 8, the window, is under way: the shell (sidebar, help, log panel), the color picker, the
AATF card pickers, and the save editor's first screens (the player list, the info strip, the
Abilities & Skills tab). The first preview, planned for the Divegrass Before Christmas cup, is
to ship the save editor and the ruleset editor. The `.4ccm` music format (Rigdio's) is built ahead of its
phase.

4,261 tests, all gates green. Every finished tool also runs from the command line
(`UTW <tool> <command>`), the way to use those the window does not show yet.

The architectural plan lives in `docs/plans/` (start at `docs/plans/README.md`). `AGENTS.md` is
the short orientation for anyone working on the code; `docs/CONTRIBUTING.md` holds the coding
rules and verification gates; `docs/WORKLOG.md` says where the work currently stands.

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE` or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license (`LICENSE-MIT` or <http://opensource.org/licenses/MIT>)

at your option. Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any additional terms or
conditions.

**Not covered by these licenses:** data extracted from the Pro Evolution Soccer games and kept in
this repository or embedded in the binary for interoperability: the player skeletons under
`resources/skeletons/`, `DpFileList.bin` and the other `.bin` templates, the kit-icon reference
sheet, and similar game-derived files. Those remain the property of Konami Digital Entertainment.
They are included so the tools can produce files the games read, and nothing here grants any
right to them beyond that use.
