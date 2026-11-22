# Unlimited Tool Works: Library crates plan: apk

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## `libs/apk`

The games' menu files, Konami's apk packs: each holds a menu screen's textures (atlases of
named sprites), its layouts and its shapes. The crate reads a pack's textures and its named
regions, and writes a pack with a texture replaced. Its users:

- the Save editor, which draws the Tactics screen's pitch from the install's own texture
  when the PES folder is set (`save_editor.md`, "Tactics editor");
- members replacing a texture in a DLC (a menu's pitch, an icon), through the PES file
  explorer that comes after this crate (worklog), which edits apk textures as the community's
  explorers do.

The crate takes a file's bytes and returns its parts: finding the file in an install or a CPK
is the caller's, through `cpk`. Layouts and shapes are kept as bytes, never decoded: nothing
the suite does needs them, and the writer carries them across unchanged.

### The format

Observed in every PES 2014 to 2021 install's `common/menu/general/gamePlan.bin`, the 4cc
patch's copies of it, and two more packs of the same format (`common/menu/parts/
pitchFigureGamePlan.bin`, `common/menu/general/gamePlanAnime.bin`); checked against the
maintainer's `pesafp_texturetool.py`, which reads and writes the format (2026-11-21).

**The wrapper.** A stock file is WESYS-wrapped zlib, which `wezlib` reads; its first three
bytes differ by version and are ignored on read. The 4cc patch's copies for PES 2018, 2019
and 2021 are stored unwrapped, and the game reads them. The reader takes either.

**The container.** Little-endian: `u32` 1, `u32` 8, the offset of the chunk area, its size,
and the offset of the pack's name (`afp_gamePlan.apk`). The name sits after the header on
PES 2015 to 2021 (chunk area at 0x30) and after the chunk area on PES 2014 (chunk area at
0x14). Every offset inside the chunk area is relative to its start.

**The texture pack.** The chunk area holds one `TXP2` chunk, whose fields are big-endian:

- 8 bytes of flags, the same in every file;
- the chunk's length, which the patch's rewritten files leave at the stock value: the reader
  bounds every read by the container's chunk size, never by this field;
- the header's length (0x6c) and a feature mask (`0x37fdf` in every file), whose bits say
  which sections follow, in bit order, each a count and an offset or an offset alone.

The sections the crate reads:

| Mask bit | Section | Each entry |
|---|---|---|
| `0x1` | the textures | name, record length, record offset (12 bytes) |
| `0x2` | the textures' name map | |
| `0x4` | (a flag) the textures are LZSS-compressed | |
| `0x8` | the regions | texture index, then left, top, right, bottom (10 bytes) |
| `0x10` | the regions' name map | |

The others (layouts `0x800`, shapes `0x2000`, a font record, per-layout blobs, and four
sections empty in every file) are kept as bytes. A file whose mask sets a bit the reader does
not know is refused, since the table's layout after it is then unknown.

**A name map** is a `NAMP` header (28 bytes: magic, three constants, the entry count, one
more field, the entries' offset), then 12-byte entries `{crc, index, name offset}` sorted by
`crc`. The `crc` is Konami's own hash of the name: a CRC-32 with the polynomial `0x4C11DB7`
that takes six bits of each byte. The reader finds a name by its text; the writer recomputes
the hash only for a name it adds, which no change below does.

**A region** stores its pixel edges doubled, plus one: left `2x + 1`, right `2x' - 1`. PES
2021's pitch, `gmPlanGameField`, stores (961, 1, 1919, 1145), the pixels (480, 0) to
(960, 573).

**A texture record** is `u32` decompressed length, `u32` stored length, then the stored bytes:
an LZSS stream when the mask sets `0x4`, the bytes themselves otherwise. The LZSS is
Okumura's: a 4,096-byte ring of zeros, written from 4,078; a flag byte per group, its bits
read from the lowest, 1 a literal, 0 a two-byte reference to 3 to 18 bytes of the ring. A
stream the patch's tools wrote may run one byte short of its stored length, so the decoder
stops at the decompressed length.

**The image** (`TXDT`, big-endian): width at +16, height at +18, the pixel format in the low
byte of +20, the pixels from +64. Two formats are in the games' files:

- `0x15`: ARGB8888, four bytes a pixel in that order;
- `0x1A`: BC3, its blocks stored as big-endian 16-bit words, so each word's two bytes are
  swapped before the blocks are decoded (through `block_compression`, as `dds_convert`
  decodes).

Another format is refused with its number. The image's own length field (+12) is stale on
the patch's files and is not read.

### What it offers

- `read(bytes) -> Result<Apk, Error>`: wrapped or not. An `Apk` holds its textures in file
  order (name, width, height, format, the image's bytes) and its regions (name, texture
  index, pixel rectangle), plus every other section's bytes for the writer.
- `Apk::region(name) -> Option<&Region>` and `Apk::texture(index)`.
- `Texture::to_rgba() -> Result<RgbaImage, Error>`: the whole atlas, decoded.
- `Error`: the wrapper's (`wezlib::Error`), a section or record past the chunk area, an
  unknown feature bit or pixel format, an LZSS stream that ends early, a name that is not
  ASCII. Each names where it was met (the section, the texture's or region's name).

A caller reads a file once and keeps what it needs, never per frame.

### The writer

A prototype rebuilt every file above (2026-11-21), and CPGE's apk reader read back the
three packs it wrote with PES 2021's pitch replaced.

**What it keeps.** The texture records are the chunk's last section in every file, so the
writer keeps every byte before the first record as it was read, except the sizes and offsets
that describe the records, and rewrites only the records. A stock pack written with nothing changed is the file it came
from, byte for byte, its wrapper's zlib stream aside; the 4cc patch's packs come out with two
quirks of the tool that wrote them gone, a stale `TXP2` length and records off the 4-byte
boundary, neither of which the game minds. A sprite's rectangle is edited in place, in
the regions table; adding or renaming a sprite would move every section after it, and no use
needs it. (The prototype also re-laid a whole chunk out from its parts and matched every stock
file byte for byte, so Konami's layout order is known if a use ever does.)

**The wrapper.** Konami's deflate stream is not zlib's at any level, memory level or
strategy, so the writer compresses with zlib and keeps the file's own three prefix bytes, or
writes the pack unwrapped, as the 4cc patch's files are, which the game reads. The caller
picks: the file's own form is the default.

**The LZSS encoder** is Okumura's, with its binary search tree over the ring: every position
is inserted as it is written, the 18 before 4,078 at the start. It reproduces all 20 stock
texture streams byte for byte, and 5 of the 8 the 4cc patch rewrote. The other three were
written by a weaker compressor, almost all literals and up to 7 times longer, and the game
reads them too: any valid stream loads.

**A texture replaced.** Its record becomes the new decompressed length, stored length and
stream, and its table entry takes the new record length (8 plus the stored length). The
records before it keep their bytes; every later record moves, and its entry's offset with it.
Each record starts on a 4-byte boundary, and the chunk ends on one. The chunk's new size goes
to the container and to the `TXP2` length (the true size, as stock files have it), with the
wrapper's sizes and, on PES 2014, whose pack name sits after the chunk area, the name's
offset. The texture's format may change: the 4cc patch's PES 2018 copy holds ARGB8888 where
the stock file has BC3.

**A region replaced** (a member's pitch): the writer decodes the texture, writes the new
pixels into the region, and in BC3 re-encodes only the blocks the region touches (through
`block_compression`, as `dds_convert` encodes), so the rest of the atlas keeps its bytes. A
block the region shares with its neighbours re-encodes their pixels too, a little lossy: under
PES 2021's pitch, rows 573 to 575 share blocks with row 572. In ARGB8888 nothing outside the
region changes.

**What it offers.** `Apk::replace_texture(index, image, format)`, `Apk::replace_region(name,
&RgbaImage)` (the image at the region's size, else an error naming both sizes) and
`write(&Apk, Wrapper) -> Vec<u8>`, `Wrapper` either the form the file was read in or none.

### Where each version keeps its pitch

The Save editor's Tactics screen draws the pitch region of the install's pack:

| Version | File | Region | Size |
|---|---|---|---|
| PES 2014 | `common/menu/parts/pitchFigureGamePlan.bin` | `pitchTexBaseGamePlan` | 326 x 418 |
| PES 2015 | `common/menu/parts/pitchFigureGamePlan.bin` | `gmPlanGameField` | 286 x 368 |
| PES 2016 | `common/menu/general/gamePlan.bin` | `gmPlanGameField` | 429 x 552 |
| PES 2017 to 2021 | `common/menu/general/gamePlan.bin` | `gmPlanGameField` | 480 x 573 |

On PES 2017 to 2021 the atlas holds three more rows of turf under the region, the rows of
the 480 x 576 texture the Save editor's plan draws at two thirds. Its two greens are within
1 of 255 of those the plan measured on screen. The file is in `Data/dt11*.cpk` (`dt11.cpk`
on PES 2014 and 2015, under `pes14_win_dat/` on PES 2014). The 4cc patch carries its own copy
in `download/4cc_01_db.cpk` on PES 2017, 2018, 2019 and 2021, with the same pitch pixels
(PES 2018's re-encoded, within 6 of 255); PES 2020's patch has none.

### Tests

PES 2021's `gamePlan.bin` (469 KB) and PES 2014's `pitchFigureGamePlan.bin` (57 KB) are committed as
fixtures, one per container layout. Against each: the texture and region counts, a few
regions by name and rectangle, and the pitch region's pixels against a PNG decoded by a
separate script (`.tmp/apk_re/apk_dump.py`'s output), not by the crate. Every other
version's file is read once from its install when the crate is built, and the result
reported.

The writer, against the same two fixtures: each written unchanged is its unwrapped payload
byte for byte; each texture's decoded bytes, encoded, are its stored stream byte for byte;
PES 2021's pitch replaced by a tinted copy reads back with the region tinted, every other
region's pixels unchanged outside the blocks it shares, and the second texture's record
unchanged. Then, in the game: PES 2021 shows the tinted pitch on the Game Plan screen from a
DLC holding the pack, once wrapped with our zlib stream and once unwrapped.
