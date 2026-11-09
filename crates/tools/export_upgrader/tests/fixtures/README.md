# Export upgrader fixtures

Lead-written; tests copy them into their sandbox and never write here.

## `co Spring/`, `co Spring.zip`, `co Spring.7z`

One old (AET) export in the three input forms `upgrade` reads, each archive holding the folder
inside one main folder, as the AET page packs exports (EU-IN-01). Both archives were built from
the folder with 7-Zip's `7z a`, so the three hold the same files.

- `co Note.txt`: the real `/4ccg/` Note of `4CCG Aesthetics Export VGL26` (Lab), byte for byte
  (CRLF, no final newline) except its `Team:` line, which names `/co/` (714), the acceptance
  section's test team. It has four team colors, three player kits and one goalkeeper kit, and
  a two-line "Other Notes:" section.
- `Other/readme.txt`: one line, content no step migrates (step 10's unmigrated content).
- `Thumbs.db`: eight bytes of an OLE header, an OS artifact step 1 leaves out unreported. The
  repository's `.gitignore` ignores the name except below a `tests/fixtures/` folder, so tree
  copies that honor it (the mutation runs') keep this one.

## `jp Summer 18/`

`JP Note.txt` alone: the real Note of `JP Aesthetic Export for Summer 18` (Teams_Main), byte for
byte (CRLF). Its team and kit color lines are ASCII, its "Player IDs:" lines Shift-JIS (one,
`72802`'s, holds bytes that are not UTF-8), and its "Other Notes:" section is empty (EU-ROOT-05).

## `kit_configs/`

Real binary kit configs (120 bytes each, not WESYS-wrapped), byte for byte from two exports in
Lab, kept under their AET names. Kit textures are not kept: the upgrader renames and compares
them without decoding, so tests write a few bytes under each texture's name instead.

- `4CCG VGL26/`: the four configs of `4CCG Aesthetics Export VGL26`, the export `co Spring`'s
  Note comes from (`1st`, `2nd`, `3rd`, `GK1st`). Each names its own kit's textures (the GK's
  under another team ID, `u0839g1`, and a leg texture the export does not ship), and three
  carry set undecoded bits the template lacks (offset `0x17` = 4 in `1st` and `3rd`, `0x13` =
  1 in `2nd`), which `config.toml`'s `[unknown]` table keeps (EU-KIT-02).
- `MGGG VGL26/`: the five configs of `MGGG  Aesthetics Export VGL26` (`1st` to `4th`,
  `GK1st`). Every one names kit 1's back texture in its back and leg fields and kit 1's name
  texture in its name field (`u0811p1_back`, `u0811p1_name`), as makers who copy kit 1's config
  for the others leave them, and `4th` is a byte copy of `3rd`, so its main field names kit 3's
  texture (EU-KIT-05, -07).

## `logos/`

Real `Logo/` folders, byte for byte from two exports in Lab, kept under their AET names
(`emblem_0XXX_r_ll.png` 512², `_r_l` 256², `_r` 128²): the two pairs nearest the small logo's
40 % line on either side, of the 94 distinct pairs on the maintainer's machine measured with
`dds_convert::resize` (plan step 7).

- `OMG VGL26/`: from `OMG  Aesthetics Export VGL26`. Its `_r` is a downscale of its `_r_ll`,
  shifted a little, 4,919 of 16,384 pixels (30.02 %) differing: the worst-scoring downscale
  (EU-MDL-02).
- `VRG VGL26/`: from `VRG Aesthetics Export VGL26`. Its `_r` is a different drawing (the
  team's mascot, where `_r_ll` shows the name over a figure), 8,699 pixels (53.09 %) differing:
  the closest deliberate variant (EU-MDL-03).
