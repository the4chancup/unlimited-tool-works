# Fallback bins

The bases the Compiler builds a bin on when no installed CPK supplies one (a from-scratch
compile; the Compiler's plan, `pipeline.md` "Bins accumulation"). They are embedded in the Studio
binary.

| File | Used for | Source | SHA-256 (first 12) |
|---|---|---|---|
| `UniformParameter18.bin` | PES 18 | Red's `Engines/bins/`, commit `c94e673` (2024-05-17, "Update fallback bin files") | `1f11016d8377` |
| `UniformParameter19.bin` | PES 19-21 | same | `12edb570d886` |
| `TeamColor.bin` | every version | same | `47db01932b31` |
| `UniColor.bin` | every version | same | `82d8e10f48e6` |

The per-version choice is Red's (19 and later build on the 19 base; one `TeamColor.bin` and one
`UniColor.bin` for every version), so a from-scratch compile's bins are comparable with Red's
output.

## Layout of the two color bins (measured 2026-10-03)

Both hold one fixed-size record per team, in team ID order starting at team 100, so team `id`'s
record starts at `(id - 100) * record size`. Both bases hold 821 records, teams 100 to 920.

- `TeamColor.bin`, 16-byte records: `u16` team ID, `u16` color count (4 in every record), then
  four colors of three bytes each (R, G, B).
- `UniColor.bin`, 85-byte records: `u32` team ID, `u8` kit count, then ten 8-byte kit entries:
  kit number (player kits count from 0, goalkeeper kits from 0x10; 0xFF marks an unused entry,
  the rest of which is zero), menu icon number, and two colors of three bytes each.

The `UniColor.bin` base holds two kinds of record. Teams 701 to 771 (71 records) carry a past
cup's kits: a count of 3 to 10, entries in ascending kit number, unused entries after them.
The other 750 are the base game's placeholder for a team with no kit colors: a count of 2 over
ten identical entries, kit number 0, icon 0, both colors white (`00 00 FF FF FF FF FF FF`). The
Compiler reads a placeholder as holding no kit (the Compiler's plan, `pipeline.md` "Bins
accumulation"). Measured on 2026-10-04.

All integers are little-endian. The cup's own files in each install's `4cc_08_bins.cpk` have
the same layout (`scripts/provenance/fixtures/color_bins_compare.py`): PES 15, 16, 17, 19
and 21 hold 821 records each, and PES 18's and PES 20's CPKs hold neither file. PES 21's are
the last VGL's (its CPK is dated 2026-06-05). An older PES 21 cup's files, from before the
database took its current form, held 803 records (teams 100 to 902); that is not a layout the
compiler supports.

Some installed `TeamColor.bin` records are corrupt: their colors were written from the
record's first byte, over the ID and the count. The last VGL's file has three (teams 799, 829
and 831, nine color bytes each), the older PES 21 cup's had one (team 761, twelve bytes). What
wrote them is not known; Red writes a team's colors after the header. The Compiler sets
every record's header on every run (the Compiler's plan, `pipeline.md` "Bins accumulation"), so
a bin it writes has none.
