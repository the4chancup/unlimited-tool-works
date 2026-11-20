# Unlimited Tool Works: Library crates plan: pes_strings

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/pes_strings`

The games' text file, `all.str`: every menu text, description and loading-screen tip a game
shows, one file per language (`eng/string/all.str` in the English archive). The crate reads it
into sections of numbered strings and looks a string up by its section's name and its number.
Its readers:

- the Save editor, which shows the game's own descriptions as tooltips (first the PES 2014
  Gameplan screen, on the `pes14` branch; its plan says which strings and how the install's
  file is found);
- later the [String editor](../string_editor.md) and the [Gametips editor](../gametips_editor.md),
  which also need the writer below.

The crate takes the file's bytes and returns its strings: finding the file in an install or an
archive is the caller's, through `cpk` where the file sits in one. It reads every version the
suite supports, PES 2014 included.

### The format

Observed in the games' own files of PES 2014, 2017, 2019 and 2021 and in the PES2021 STR
Editor's code (a lookup of 2026-11-20). All numbers are little-endian.

**The wrapper.** The file is a WESYS-wrapped zlib stream, which `wezlib` reads. Its first three
bytes differ by version (`00 01 01` on 2014, `00 10 d1` on 2017, `04 10 d1` on 2019, `ff 10 d1`
on 2021) and are ignored on read, as `wezlib` and the STR Editor both do. A file may also come
unwrapped: the gametips scripts work on the inflated payload, so the reader takes either.

**The payload.**

- `u32` section count, `u32` offset of the section table (always 8).
- The section table, 12 bytes a section: `u32` offset of the section's data, `u32` its length,
  `u32` offset of its name.
- The names: NUL-terminated ASCII, at most 8 characters seen, in sorted order, which is also the
  sections' order. On PES 2014 they sit after all the sections' data; on 2017 to 2021 right
  after the table. The reader follows the offsets, so it reads both.
- Each section, at an offset that is a multiple of 16:
  - `u32` string count;
  - `u32` group id: a number of its own per section (`Tip1C` 126, `Tip2C` 127), whose high 16
    bits are 0 in every file seen;
  - one 12-byte entry per string: `u32` string id, `u16` the text's length in bytes with its
    NUL, `u16` its length in UTF-16 code units without the NUL, `u32` the text's offset from
    the section's start;
  - the texts, NUL-terminated UTF-8, one after another;
  - zero padding to the next multiple of 16 (the file's end too).

The texts carry the game's inline codes as they are stored: `\x11` and two letters for a pad
button's glyph, private-use characters for other glyphs, `\n` for a line break. The reader keeps
them as they are; what a screen makes of them is the screen's.

The reader takes each text up to its NUL and ignores the two lengths, as the STR Editor does:
files the gametips scripts wrote carry a wrong second length (bytes minus one, right only for
ASCII), and those files reach the game.

### What it offers

- `read(bytes) -> Result<StrFile, Error>`: wrapped or not. A `StrFile` holds its sections in
  file order, each with its name, its group id (the whole `u32`, kept for the writer) and its
  strings in file order, each with its id (the whole `u32`) and its text.
- `StrFile::section(name)` and `Section::text(id)`: the lookup, `None` when absent.
- `Error`: the wrapper's (`wezlib::Error`), a table, entry or text past the payload's end,
  a text that is not UTF-8. Each names where it was met (the section's name, the string's id).

A caller that shows the strings reads the file once and keeps them, never per frame.

### The writer (built with the String editor)

Built when the first of the two editors is, not before: until then nothing writes the file.
It writes:

- the group ids and string ids as read;
- the lengths recomputed from each text: bytes plus the NUL, and UTF-16 code units, which is
  what the games' own files carry (the STR Editor writes the same);
- the names where the input had them (after the data on PES 2014, after the table on the
  others);
- each section and the file's end padded to 16.

So a file read and written unedited comes back byte for byte at the payload level, which is the
writer's test on each version's own file. The payload is then wrapped through `wezlib`, whose
prefix is `00 10 01`. The STR Editor writes the same prefix on its PES 2021 files; whether each
game takes it is checked in game when the writer is built.

### Tests

Two real files are committed as fixtures, one per name layout: PES 2017's `eng/string/all.str`
(278 KB, names after the table) and PES 2014's (350 KB, names after the data). Against each:

- the section count;
- the string counts of a few sections (`Tip1C` holds 343 on PES 2017);
- a handful of strings by section, id and text, ASCII and not, one with an inline code.

The expected values are read from the files by a separate script, not by the crate. The PES
2019 and 2021 files are read once from the installs when the crate is built, and the result
reported; they share PES 2017's layout, so they are not committed.
