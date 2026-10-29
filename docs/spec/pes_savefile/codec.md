# Unlimited Tool Works — Savefile plan: Save codec

Part of the [Savefile plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Schema-driven save codec

4ccEditor's six per-version codec files are near-duplicated sequential bitfield reads:

```cpp
players.lowpass  = read_data(0, 7, current_byte, pDescriptorNew);
players.loftpass = read_data(7, 7, current_byte, pDescriptorNew);
players.finish   = read_data(6, 7, current_byte, pDescriptorNew);
```

In Rust this becomes a declarative schema — one field table per version, one generic codec engine.
Every bit run is LSB-first little-endian: field bit `i` is bit `(offset + i) % 8` of record byte
`(offset + i) / 8`, which is what the reference's byte-crossing reader computes and what the
hand-shifted PES 18 walk computes too.

```rust
/// One bit run of a record.
pub struct FieldSpec<F> {
    pub field: F,
    /// Bit position from the record's first byte.
    pub bit_offset: u32,
    pub bit_width: u32,
}

/// A regular run of indexed fields (`play_skill[0..41]`, roster slots, formation players):
/// element `i` is `make(i)` at `base_bit + i * stride_bits`.
pub struct ArraySpec<F> {
    pub make: fn(u8) -> F,
    pub count: u8,
    pub base_bit: u32,
    pub stride_bits: u32,
    pub bit_width: u32,
}

/// A byte string field: NUL-terminated when the text is shorter than the field, unterminated
/// when it fills the field exactly.
pub struct TextSpec<T> {
    pub text: T,
    pub byte_offset: u32,
    pub len: u32,
}

/// A byte run carried verbatim.
pub struct ByteRun { pub byte_offset: u32, pub len: u32 }

/// One record kind's layout for one version (player, appearance, team, roster).
pub struct RecordSchema<F, T> {
    pub size: usize,
    pub fields: &'static [FieldSpec<F>],
    pub arrays: &'static [ArraySpec<F>],
    pub texts: &'static [TextSpec<T>],
    /// The opaque ingame-face run (`model.md` "Player settings model"): `Some` on the record
    /// that carries the appearance block (the 15/16 appearance record, the 17+ player record).
    pub ingame_face: Option<ByteRun>,
}

/// A per-preset setting: one byte at `bit_offset + preset * preset_stride_bits`.
pub struct PresetSpec { pub field: PresetField, pub bit_offset: u32 }

/// The 3 x 3 formation blocks: player `slot`'s position byte at
/// `base + preset * preset_stride + formation * formation_stride + slot * slot_stride`, its
/// y then x bytes at `.. + y_offset + slot * pair_stride` and `.. + x_offset + ..`.
pub struct FormationLayout {
    pub base_bit: u32,
    pub formation_stride_bits: u32,
    pub slot_stride_bits: u32,
    pub y_offset_bits: u32,
    pub x_offset_bits: u32,
    pub pair_stride_bits: u32,
}

/// The advanced instructions (PES 17+): one byte at
/// `base + preset * preset_stride + side * side_stride + index * index_stride + part * part_stride`.
pub struct InstructionLayout {
    pub base_bit: u32,
    pub side_stride_bits: u32,
    pub index_stride_bits: u32,
    pub part_stride_bits: u32,
}

/// The tactics record: the scalar and array fields plus the three regular nested blocks,
/// which the derivation checks are regular before emitting them this way.
pub struct TacticsSchema {
    pub size: usize,
    pub fields: &'static [FieldSpec<TacticsField>],
    pub arrays: &'static [ArraySpec<TacticsField>],
    pub preset_stride_bits: u32,
    /// Where a preset's eleven man-marking bytes start within its block; measured, since the
    /// reference reads no man marking (the generator's `MEASURED_FIELDS`).
    pub man_marking_bit: u32,
    pub presets: &'static [PresetSpec],
    pub formations: FormationLayout,
    pub instructions: Option<InstructionLayout>,
}

/// Where a section of records sits in the payload.
pub struct SectionLayout {
    pub offset: usize,
    /// Byte offset of the little-endian u16 record count.
    pub count_offset: usize,
}

pub struct VersionSchema {
    pub version: PesVersion,
    pub players: SectionLayout,
    pub player: &'static RecordSchema<PlayerField, PlayerText>,
    /// PES 15/16: the separate appearance array (keyed by player id at +0); `None` when the
    /// appearance fields are inside the player record.
    pub appearance: Option<(SectionLayout, &'static RecordSchema<PlayerField, PlayerText>)>,
    pub teams: SectionLayout,
    pub team: &'static RecordSchema<TeamField, TeamText>,
    pub rosters: SectionLayout,
    pub roster: &'static RecordSchema<RosterField, TeamText>,
    pub tactics: SectionLayout,
    pub tactic: &'static TacticsSchema,
}

pub fn schema_for(version: PesVersion) -> &'static VersionSchema;
```

`F` and `T` are the field vocabularies in `schema/fields.rs` (generated with the tables):
`PlayerField` (scalar variants plus `PlayablePosition(u8)`, `ComStyle(u8)`, `Skill(u8)`),
`PlayerText` (`Name`, `ShirtName`), `TeamField` (identity, colours, edit flags,
`KitSlotNumber(u8)`/`KitSlotTeam(u8)`), `TeamText` (`Name`, `ShortName`), `RosterField` (`TeamId`,
`Player(u8)`, `Number(u8)`), `TacticsField` (`TeamId`, `Preset { preset, field: PresetField }`,
`Formation { preset, formation, slot, part: FormationPart }`, `Instruction { preset, side:
InstructionSide, index, part: InstructionPart }`, `Starting(u8)`, `Bench(u8)`,
`PlayerToJoinAttack(u8)`, `ManMarking { preset, slot }`, the set-piece takers, `Captain`, the
auto flags). Two kinds of field the reference never reads are measured instead and enter the
generator as its `MEASURED_FIELDS` table: the man-marking runs (every version) and PES 15's
three auto flags, so a regeneration keeps them. Variant names are the
readable model names (`TightPossession`, not `tight_pos`); the table module's doc comment says
which reference walk it was derived from, and this section is the evidence trail. The generator
is `scripts/derive_savefile_schema.py <reference source dir> <crate>/src/schema`; it is committed so
the tables can be regenerated, and the tables it wrote are committed as ordinary source.

**Text fields hold `len` bytes, not `len - 1`.** A text that fills its field is stored without a
NUL; a shorter one gets one NUL after it and the rest of the field keeps its old bytes. The
savefile census (2.20i, every save on the maintainer's machine) found real saves in both
shapes: PES 16 shirt names of exactly 16 bytes (`MEAT ON THE BONE`) and PES 17 names of exactly
46 bytes. Reading stops at the first NUL or the field end; writing refuses only a text longer
than the field, so every save we read writes back byte for byte, and a Team TOML carried from
such a save applies back. We do not cut a full text by one character to make room for a NUL,
which silently renames the player on the first save. The one place that still keeps a byte
free is `shirt_name_from` (`operations.md`), because it invents text rather than carrying it.
A name whose bytes are not UTF-8 (one PES 17 save carries a CP1252 `£`, byte `0xA3`) is refused
on read, not decoded lossily: that save does not load in 4ccEditor either, so no working save
carries one.

**Derivation, not transcription.** The tables are not typed in by hand: a lead script interprets
each reference read walk (`fill_player_entry17`, `fill_team_ids21`, …) over a symbolic byte
array, so `read_data(start, bits, …)`, `data[current_byte] >> 4`, `+= (data[current_byte] << 1)
& 127` and the string copies all resolve to which record bits feed which field bits, then checks
every field is one contiguous LSB-first run and emits the Rust tables. The derived tables were
checked against the real payloads (2026-09-14): every player of every save decodes to plausible
values (ages 15–50, abilities 40–99, positions 0–12, valid UTF-8 names), the PES 15/16 appearance
arrays resolve every player id, and the team id at +0 of the team, roster and tactics records
lists the same ids in the same order on every version (which pins all three record sizes).
Record sizes: player 112/112/188/188/188/312/312; appearance 68/72 (15/16); team
456/456/480/480/416/528/588; roster 164 (32 slots) on 15–18, 244 (40 slots) on 19, 284 (40) on
20/21; tactics 516/520/628/628/628/628/628, versions 15 to 21 in order. Counts: players u16 at
0x34/0x34/0x5C/0x60/0x60/0x60/0x60, teams at 0x38/0x38/0x60/0x64/0x64/0x64/0x64.

Reference readings the derivation flagged, kept as the reference has them and listed here so
nobody mistakes them for ours: `b_edit_stadium` on PES 19 is masked to zero by the reference
(`>> 6 & 64`), so the PES 19 table has no such field. The PES 21 team-colour bit positions were
suspect (they read zero for 203 of 220 teams of one 4cc save where PES 17/18 read the same
teams' colours) until measured in Edit mode on PES 21: the game shows and saves Team Colour 1
and 2 at the table's bits (changing colour 1 to a swatch moved `Color1Red`, `Color1Green` and
`Color1Blue` alone, to the RGB the face colour picker gives that swatch), and in the live cup
save the 70 cup teams 701 to 770 hold colours while the 140 placeholder teams 771 to 910 are
zero.
Two corrections are ours, applied by the generator, not the reference: PES 18's
dribbling-arm motion (`mo_armd`) is 3 bits wide where the reference reads 2 (`WIDTH_FIXES`; the
2.20i census found 2,808 records setting the third bit); and on PES 17+ the appearance block's
first u32, the appearance id the reference reads past, gets a row (`AppearanceId`), as the
PES 15/16 appearance record's `id` does.

~190KB of C++ collapses into static tables plus a ~100-line engine (the Rust equivalent of the
reference's `read_data`/`write_data`, which handle bit runs crossing byte boundaries). Roundtrip
tests (read → write → compare bytes, every record of every real payload) verify the tables tile
the bytes they claim; the plausibility census above is what verifies they name them right.

Schema tables needed (following 4ccEditor's actual sharing):

| Schema | Source | Notes |
|--------|--------|-------|
| 15 | `pes15.cpp` | Own player + separate appearance schema, own bit-read variant |
| 16 | `pes16.cpp` | Split player/appearance blocks |
| 17 | `pes17.cpp` | First unified block; adds `phys_cont` |
| 18 | `pes18.cpp` | New container header; unified block |
| 19 | `pes19.cpp` | Adds `star`; 39 skills |
| 20/21 | `pes20.cpp` (shared, with 21-specific team functions) | Adds `mo_drib`, `tight_pos`, `aggres`, `play_attit`, `strong_hand`; 41 skills |

The same schema mechanism covers the team sections (team IDs, rosters, tactics),
not just player blocks.

### Whole-file API

The consumer-facing entry point wraps the container crypto and the schema codec:
`EditFile::load` (version auto-detected by trying master keys / container shape;
decrypt, parse) and `EditFile::save` (serialize, re-encrypt, and preserve the original in a `.bak`
before replacement — the shared convention behind the save editor's save action and the Team
compiler's savefile update stage). An open `EditFile` keeps its detected version; changing the
suite selector cannot reinterpret it. Compiler updates require a save matching the compile target.
Retain unmodeled container sections, payload bytes, padding, and unknown bits, patching only the
selected known fields; rebuilding only the modeled fields cannot satisfy lossless-save tests.
A model value the record cannot hold is refused, never cut: a value wider than its field
(`ValueTooWide`), a text longer than its field, a roster longer than the record's slots
(`RosterTooLong`).

```rust
/// An open save: the retained container plus the decoded players and teams.
pub struct EditFile { /* container: SaveContainer, players: Vec<PlayerEntry>, teams: Vec<TeamEntry>,
                         plus the appearance-record index of every player (PES 15/16; the roster
                         and tactics records are their team's by position) */ }

impl EditFile {
    /// Decrypts and decodes; the version is the container's.
    pub fn from_bytes(bytes: &[u8]) -> Result<EditFile, SaveError>;
    /// `from_bytes` over a file (native only).
    pub fn load(path: &Path) -> Result<EditFile, SaveError>;
    pub fn version(&self) -> PesVersion;
    pub fn players(&self) -> &[PlayerEntry];
    pub fn players_mut(&mut self) -> &mut [PlayerEntry];
    pub fn player(&self, id: u32) -> Option<&PlayerEntry>;
    pub fn player_mut(&mut self, id: u32) -> Option<&mut PlayerEntry>;
    pub fn teams(&self) -> &[TeamEntry];         // team, roster and tactics joined by id
    pub fn teams_mut(&mut self) -> &mut [TeamEntry];
    pub fn team(&self, id: u32) -> Option<&TeamEntry>;
    pub fn team_mut(&mut self, id: u32) -> Option<&mut TeamEntry>;
    /// Every player and team written back into the retained payload, then re-encrypted under
    /// `salt`; the record order and count are the file's own (no player or team is added or
    /// removed through this API).
    pub fn to_bytes(&self, salt: &[u8; 320]) -> Result<Vec<u8>, SaveError>;
    /// `to_bytes` with a fresh salt, written next to `path` and renamed over it after the
    /// existing file was copied to `<path>.bak` (native only).
    pub fn save(&self, path: &Path) -> Result<(), SaveError>;
}
```

`SaveError` wraps `ContainerError`, `CodecError`, `std::io::Error`, plus `Layout` (a section's
count or a record runs past the payload) and `RecordId` (a roster or tactics record names a team
id the team section lacks, or a PES 15/16 player has no appearance record). Loading is strict:
the three team sections must list the same ids in the same order, which every real save does.

**Name colour codes, as measured** (2026-09-14, 346 decorated names across the PES 16/19/21
saves): a code is the byte `0x11`, the letter `c`, then **exactly eight bytes** taken as the
colour; the game does not check they are hex (nine names of one PES 21 team carry
`\x11ca000c8ON` and display without the `ON`), so neither does `display_name`. A second code
exists: `\x11d`, two bytes, which resets the colour (`WHEN THE \x11cb7bec5ffWORK RESULT\x11d WAS
GOOD`). `display_name` strips both; any other `0x11` sequence is left alone. `name` is the raw
form; the save editor renders the colours from it.

### Savefile discovery

The savefile does **not** live in the PES install folder — it is always under the user's
**Documents** folder, in a per-game KONAMI subfolder. Discovery therefore never looks at
`pes_folder_path`; it is a function of the PES version and the user profile, provided by this crate
(`discover_savefiles(version) -> Vec<SavefileCandidate>`) and used by the Team compiler's savefile
stage (`savefile_path = auto`) and the Save editor's Open dialog (initial folder / quick-open list).

Two layouts exist, verified on a live machine with PES 15, 16, 17, 18, 19 and 21 saves present
(2026-09-14) and a PES 20 save (2026-09-20):

| Versions | Path under `{Documents}\KONAMI\` | Notes |
|---|---|---|
| 15, 16, 17, 18 | `{game folder}\save\EDIT00000000` | one save per game, no account level. Game folders: `Pro Evolution Soccer 2015` (its save is named `EDIT.bin`, not `EDIT00000000`), `Pro Evolution Soccer 2016`, `Pro Evolution Soccer 2017`, `PRO EVOLUTION SOCCER 2018` (upper case; PES 18 was expected to use the account layout and does not) |
| 19, 20, 21 | `{game folder}\{account id}\save\EDIT00000000` | game folders: `PRO EVOLUTION SOCCER 2019`, `eFootball PES 2020`, `eFootball PES 2021 SEASON UPDATE`. `{account id}` is a numeric folder, one per account that has run the game on this profile: an 18-digit Konami ID, or the 17-digit Steam ID64 of a copy bought on Steam (the reference machine's PES 2020, `76561199320748656`); any all-digit name counts |

Rules:

- **`{Documents}` is the shell's known folder, never `%USERPROFILE%\Documents`.** Documents is
  frequently relocated (the reference machine has it at `C:\Data\Documents`) or redirected by
  OneDrive; resolve it via `directories::UserDirs::document_dir()` (which wraps
  `SHGetKnownFolderPath(FOLDERID_Documents)`). On Linux/Wine the same relative layout applies under
  the Wine prefix's Documents; the KONAMI folder name is the anchor either way.
- **Account folders**: with exactly one `{account id}` folder containing a `save\EDIT00000000`, that
  is the result. With several, all are returned as candidates ordered by the savefile's modification
  time, newest first. Folders without a savefile are ignored, not errors.
- Discovery never creates folders.

The per-version table is data (a `match` on `PesVersion` returning the game folder name and
whether an account folder sits between it and `save`), so a corrected folder name is a one-line
change.

```rust
pub struct SavefileCandidate {
    pub path: PathBuf,
    /// The account folder the save sits under (PES 19-21), else `None`.
    pub account: Option<String>,
    pub modified: SystemTime,
}

/// The game folder name and layout for `version`.
pub fn save_layout(version: PesVersion) -> SaveLayout;   // { game_folder: &'static str, account_folders: bool, file_name: &'static str }
/// The candidates under `documents/KONAMI/...` for `version`, newest first. Pure over the
/// given root so tests build a temp tree; the native caller passes `UserDirs::document_dir()`.
pub fn discover_savefiles_in(documents: &Path, version: PesVersion) -> Vec<SavefileCandidate>;
/// `discover_savefiles_in` under the shell's Documents folder (native only).
pub fn discover_savefiles(version: PesVersion) -> Vec<SavefileCandidate>;
```

The compiler's `savefile_path = auto` takes the newest candidate and reports which one it
chose (`savefile_autodetected`, Info, with the path and the candidate count) so a user with
two accounts can pin `savefile_path` if the guess is wrong. The version is verified after
discovery, not assumed from the folder: the candidate is loaded with the ordinary
`EditFile::load` auto-detection and must match the requested version, otherwise it is
refused (`savefile_invalid`, naming the save's version) rather than used — a stale save
left by a previous game version in a reused folder must not be edited as if it were
current. Nothing found is the consumer's own message (`savefile_missing` in the compiler).

Discovery returns paths and never opens the saves; verifying the version is the caller's
`EditFile::load`, the rule above.

---
