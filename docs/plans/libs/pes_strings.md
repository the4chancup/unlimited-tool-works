# Unlimited Tool Works: Library crates plan: pes_strings

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/pes_strings`

The games' text file, `all.str`: every menu text, description and loading-screen tip a game
shows, one file per language (`eng/string/all.str` in the English archive). The crate reads it
into sections of numbered strings and looks a string up by its section's name and its id.
Its readers:

- the Save editor, which shows the game's own descriptions as tooltips (first the PES 2014
  Gameplan screen, on the `pes14` branch; its plan says which strings and how the install's
  file is found);
- later the [String editor](../string_editor.md) and the [Gametips editor](../gametips_editor.md),
  which also need the writer below.

The crate takes the file's bytes and returns its strings: finding the file in an install or an
archive is the caller's, through `cpk` where the file sits in one. It is meant for every version
the suite supports, PES 2014 included ("Tests" says how the versions not seen yet are checked).

### The format

Observed in the games' own files of PES 2014 to 2021 and in the PES2021 STR Editor's code (a
lookup of 2026-11-20). All numbers are little-endian.

**The wrapper.** The file is a WESYS-wrapped zlib stream, which `wezlib` reads. Its first three
bytes differ by version (`00 01 01` on 2014, `00 10 d1` on 2015 to 2017, `04 10 d1` on 2018 to
2020, `ff 10 d1` on 2021) and are ignored on read, as `wezlib` and the STR Editor both do. A
file may also come unwrapped, and the games read it so: the cup's gametips CPKs for PES 2015,
2018, 2019 and 2021 carry the bare payload. The reader takes either.

**The payload.**

- `u32` section count, `u32` offset of the section table (always 8).
- The section table, 12 bytes a section: `u32` offset of the section's data, `u32` its length
  (its bytes up to its padding: 8, 12 a string, then its texts), `u32` offset of its name.
- The names: NUL-terminated ASCII, up to 13 characters seen, in sorted order, which is also
  the sections' order. Where they sit is one of the two layouts below.
- Each section, at the offset its record gives:
  - `u32` string count;
  - `u32` group id: a number of its own per section (`Tip1C` 126, `Tip2C` 127), whose high 16
    bits are 0 in every file seen;
  - one 12-byte entry per string: `u32` string id, `u16` the text's length in bytes with its
    NUL, `u16` its length in UTF-16 code units without the NUL, `u32` the text's offset from
    the section's start;
  - the texts, NUL-terminated UTF-8, one after another;
  - zero padding, as its layout says.

**Two layouts.** The versions differ only in where the names sit and what pads to what:

| | PES 2014 | PES 2017, 2019, 2021 |
|---|---|---|
| The names | after the sections' data, each padded to 4 | after the table, back to back |
| Each section starts at | a multiple of 4 | a multiple of 16 |
| The file's end | padded to 4 | padded to 16 |

The reader follows the offsets, so it reads both alike.

The texts carry the game's inline codes as they are stored: `\x11` and two letters for a pad
button's glyph, private-use characters for other glyphs, `\n` for a line break. The reader keeps
them as they are; what a screen makes of them is the screen's.

The reader takes each text up to its NUL and ignores the two lengths, as the STR Editor does:
files the gametips scripts wrote carry a wrong second length (the first minus one, right only
for ASCII), and those files reach the game.

### What it offers

- `read(bytes) -> Result<StrFile, Error>`: wrapped or not. A `StrFile` holds its sections in
  file order, each with its name, its group id (the whole `u32`, kept for the writer) and its
  strings in file order, each with its id (the whole `u32`) and its text.
- `StrFile::section(name)` and `Section::text(id)`: the lookup, `None` when absent.
- `Error`: the wrapper's (`wezlib::Error`), a table, name, entry or text past the payload's
  end, a name or text that is not UTF-8. Each names where it was met (the section's name, the
  string's id).

A caller that shows the strings reads the file once and keeps them, never per frame.

### The writer (built with the String editor)

Built when the first of the two editors is, not before: until then nothing writes the file.
It writes:

- the group ids and string ids as read;
- the lengths recomputed: each text's bytes plus the NUL and its UTF-16 code units, which is
  what the games' own files carry (the STR Editor writes the same), and each section's length
  up to its padding;
- the names and the padding in the input's layout, which the `StrFile` then keeps from the
  read (the reader alone does not need it).

So a file read and written unedited comes back byte for byte at the payload level, which is the
writer's test on each version's own file. The payload is then wrapped through `wezlib`, whose
prefix is `00 10 01`. The STR Editor writes the same prefix on its PES 2021 files; whether each
game takes it is checked in game when the writer is built.

### Tests

Two real files are committed as fixtures, one per layout: PES 2017's `eng/string/all.str`
(278 KB, names after the table) and PES 2014's (350 KB, names after the data). Against each:

- the section count;
- the string counts of a few sections (`Tip1C` holds 343 on PES 2017);
- a handful of strings by section, id and text, ASCII and not, one with an inline code.

The expected values are read from the files by a separate script, not by the crate. Every other
version's file (PES 2015, 2016 and 2018 to 2021) is read once from its install when the crate
is built, and the result reported; a file in neither layout stops the work for a plan change.
Those in PES 2017's layout are not committed. On 2026-11-20 every one of them read in PES
2017's layout, as did the cup's gametips files for PES 2015, 2016, 2018, 2019 and 2021.
