# Unlimited Tool Works — Savefile plan: Data model

Part of the [Savefile plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Player/team/tactics model

One version-independent model (`model/player.rs`, `model/team.rs`, `model/tactics.rs`),
populated by whichever schema loaded the file. 4ccEditor's ~100-line manual `operator==` and
`PlayerExport()`/`PlayerImport()` copy methods are derived `PartialEq`/`Clone`; the model has
no serde derives, because every text form (Team TOML, `settings.toml`, Texport) is its own
format with its own writer in `interchange/` and `settings_toml/`.

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerEntry {
    pub id: u32,
    pub appearance_id: Option<u32>, // the appearance block's player id (PES 17+; `None` on
                                    // 15/16, where the appearance record's own `id` is the
                                    // join key): the player's own, or `STRIPPED_APPEARANCE_ID`
                                    // (the game's -1) on a stripped Fox save
    pub name: String,          // on disk: UTF-8, 46 B (15–19) / 61 B (20/21), NUL-terminated
                               // unless it fills the field (codec.md "Text fields");
                               // 4ccEditor's wchar[61] is only its in-memory buffer
    pub shirt_name: String,    // on disk: single-byte, 18 B (15, 17–19), 16 B (16) /
                               // 61 B (20/21); 4ccEditor's char[21]
    pub basic: PlayerBasics,        // nationality, age, height, weight
    pub stats: PlayerStats,         // the ability values, form, injury, weak foot, and the
                                    // version-gated ones: physical_contact (17+),
                                    // catching (15), clearing/reflexes/coverage (16+),
                                    // star (19+), tight_possession (20+)
    pub positions: PlayerPositions, // registered position, play_pos[13] (A/B/C), playstyle
    pub skills: PlayerSkills,       // play_skill[..41], com_style[7]
    pub motion: PlayerMotion,       // hunching, arm movement, kick motions, gc1/gc2
    pub edit_flags: PlayerEditFlags,// b_edit_face/hair/phys/strip/player/…, base copy
    pub appearance: PlayerAppearance, // boots/gloves IDs, physique, strip style,
                                      // colors, ingame face parameters
}
```

Version-gated fields are `Option<T>` (`None` = this save's version has no such field):

| Fields | Versions |
|--------|----------|
| `catching` ("Saving") | 15 only (replaced by clearing/reflex/coverage) |
| `clearing`, `reflex`, `coverage` | 16+ |
| `phys_cont` | 17+ |
| `star` | 19+ |
| `mo_drib`, `tight_pos`, `aggres`, `play_attit`, `strong_hand` | 20+ |
| available skills | 22 (15, mapped into 28 canonical slots), 28 (16–18), 39 (19), 41 (20/21) |

The model favors readable field names — e.g. `tight_possession` where the schema
tables quote 4ccEditor's source identifier `tight_pos` verbatim; the schema layer
maps between the two spellings.

The team side is a full model, not just names — the save editor needs all of it:

The save keeps a team in three records (team, roster, tactics) keyed by team id; the entry
merges all three:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamEntry {
    pub id: u32,
    pub name: String,            // on disk: UTF-8, 0x46 B (4ccEditor's wchar[0x46]
                                 // is its in-memory buffer)
    pub short_name: String,      // 3 chars, single-byte on disk
    pub manager_id: Option<u32>,          // 19+
    pub stadium_id: Option<u16>,          // 19+
    pub colors: Option<[TeamColor; 2]>,   // 17+, 6 bits per channel
    pub edit_flags: TeamEditFlags,        // name; short name (15), stadium (20/21), strip (17)
    pub kit_slots: Option<[KitSlot; 10]>, // 17 only: kit number ↔ bound team id
    pub roster: Vec<RosterSlot>, // one per roster array entry (32 or 40 by version):
                                 // player id (0 = empty) + shirt number (u16; 19+ stores
                                 // values above 99)
    pub tactics: TeamTactics,
}

pub struct TeamTactics {
    pub presets: [TacticsPreset; 3],
    pub starting_eleven: [u8; 11],        // roster slots
    pub bench_order: [u8; 21],            // roster slots
    pub set_pieces: SetPieceTakers,       // FK long/short/second, CK L/R, PK, captain;
                                          // roster slots, 0xFF none
    pub players_to_join_attack: [u8; 3],  // roster slots
    pub auto: TeamAutoFlags,              // auto sub, offside trap, preset change (15+);
                                          // attack/defence levels (17+)
}

pub struct TacticsPreset {
    pub style: TacticStyle,          // attacking/defensive style, zones, buildup,
                                     // positioning, containment, pressure, fluid (16+)
    pub sliders: TacticSliders,      // support range, defensive line, compactness (1–10),
                                     // numbers in attack/defence (1–3)
    pub formations: [Formation; 3],  // kick-off, in-possession, out-of-possession
    pub attack_instructions: Option<[AdvancedInstruction; 2]>,  // 17+
    pub defence_instructions: Option<[AdvancedInstruction; 2]>, // 17+
    pub man_marking: [u8; 11],       // own formation slot i -> the marked opponent's
                                     // formation slot (its starting-eleven index), 0xFF none
}

pub struct Formation {
    pub players: [FormationSlot; 11], // position (GK 0 .. CF 12), x, y
}
```

Version quirks to encode: advanced-instruction value ranges differ (PES 17:
0x00–0x0C; 18/19: 0x00–0x0F, different meanings — 4ccEditor resolves this through
a canonical 17-order enum: values 0x08–0x0C shift +2 for 18/19, and 18/19's
Defensive/False Winger/Wingback are 0x0D/0x0E/0x0F in canonical order, mapping
to 0x08/0x09/0x0F; 20/21 insert Anchoring at 0x0A, 18/19's 0x0A–0x0F moving to
0x0B–0x10, measured in-game on PES 21 — 4ccEditor's table is right for 17–19
only), and **playstyle enums are
version-specific** — 4ccEditor's `menu_lists.cpp` carries twelve conversion
arrays (16↔17/18, 16↔19, 16↔20/21, 17/18↔19, 17/18↔20/21, 19↔20/21). In Rust
these become one canonical `PlayStyle` enum (`model/playstyle.rs`) plus one list per version
group with `decode`/`encode` (`schema/playstyle.rs`), which is what makes cross-version team
import work; the lists and the rule for unlisted values are in `operations.md` "Cross-version
player conversion". `PlayerPositions.playing_style` stays the stored `u8` index.

---

## Player settings model (settings.toml)

The crate owns the `PlayerSettings` model behind the export format's
`settings.toml` files (see "Player settings in exports" in the
[Aesthetics export plan](../aesthetics_export/README.md)): parsing the TOML, applying it to a
`PlayerEntry`, and the reverse — generating `settings.toml` from an existing
savefile (used by the save editor's migration feature and the Export upgrader).
Keeping both directions next to the codec ensures the TOML schema and the
savefile fields cannot drift apart.

**Name colour codes.** Cup savefiles colour player names in-game with an inline
code: the byte `0x11`, the ASCII letter `c`, then **8 hex digits** (RRGGBB plus
an alpha byte, `ff` in every observed case) — `\x11ce5de00ffXIAO MEI MEI`. Codes
may appear anywhere in the name, several times, to colour segments differently
(`PEP\x11cffffffffSI \x11c004b93ffMAN` renders "PEPSI MAN" in three colours).
They are used freely — medal players are the common case, but not the only one.
Verified against ~30 decorated names across real PES 19 and 21 cup saves (see
Verification). No tool in the 4cc toolset writes them; they are hand-edited by
cup organizers and must be **preserved byte-for-byte** by every copy-through
and roundtrip path. This crate exposes the raw name and a
`display_name()` helper that strips the codes (fixed-length match: the colour is exactly the 8
bytes after `\x11c`, hex or not, so a name letter `a`–`f` following a code is never eaten; the
2-byte reset `\x11d` is stripped too; see `codec.md` "Whole-file API" for the measurement); the
save editor renders the colours from the raw name and allows editing them, while
folder-name derivation (Team compiler, Export upgrader) and search use the
stripped form. Any other control character in a name is not a known code and is
left alone.

`PlayerSettings` covers the player `name`, the `shirt_name`, **every decoded field of the
savefile's player appearance record** —
physique, strip style (taping, spectacles and their colour, sleeves, inners, socks, undershorts,
shirttail, player gloves and their colour), wrist-tape colours, skin and iris colour, the motion
block (hunching, arm movement, kick motions, gc1/gc2, dribbling motion on 20+; visual choices a
team makes, so aesthetics), every field of the ingame-face run — sliders, colours, hair, the
eleven feature types; the run is fully decoded ("The ingame-face run" below) — and
the boots/gloves model IDs
**within the stock band** (0 to 100; custom IDs are derived by the compiler from the models and
links present, see the Aesthetics export plan's "Player settings in exports") — with one exclusion,
**compiler-owned**: the edit flags, the base-copy ID and the appearance id (derived from what was
written). Export
`settings.toml` neither accepts the excluded keys nor emits them when generated from a save.

```rust
/// `settings_toml`
pub enum NameSetting { FromFolder, Explicit(String) }

/// One TOML key of the table; the enum is the table's row set and `spec()` its columns
/// (table, key name, kind, widest range, range comment, the default an absent key applies).
/// Physique keys convert between the game number and the stored value (+7); 1-based motion
/// keys between the game number and the stored value (-1); string-enum keys between the
/// listed labels and the stored index.
pub enum SettingKey { BootsId, GlovesId, SkinColor, IrisColor, NeckLength,
                      /* … every key of the block, in its order */ LowerLipType }

/// Every leaf is `Option`; the struct mirrors the TOML tables (top level, `appearance`,
/// `appearance.physique`, `.strip`, `.motion`, `.face`, `.hair`) with the model's
/// storage types (`u8`/`bool`): the string labels and the signed physique numbers exist
/// only at the parse/emit boundary. A `None` appearance key applies its `KeySpec` default
/// (the template's shown value), so a compiled player's look is his file and nothing else;
/// `name`, `shirt_name` and the two stock IDs are the exceptions: `None` is not written —
/// the IDs are the compiler's to resolve (the folder's models, the FPC marker). `boots_id`/
/// `gloves_id` are top-level keys (0 to 100, the stock band, or `""` = `None`), outside
/// `AppearanceSettings`, because Team TOML embeds `AppearanceSettings` beside its own
/// full-range player-level IDs. `to_toml` writes a `None` ID as `""`, uncommented.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerSettings { pub name: Option<NameSetting>, pub shirt_name: Option<String>,
                            pub boots_id: Option<u8>, pub gloves_id: Option<u8>,
                            pub appearance: AppearanceSettings }

impl PlayerSettings {
    /// Parses a file: unknown keys and tables, wrong types, out-of-range values and unknown
    /// labels are `SettingsError`s naming the key; the compiler-owned keys are unknown keys.
    /// A `name` or `shirt_name` containing a NUL is `WrongType` (reading a save field stops
    /// at the first NUL, so it would silently truncate on reload).
    pub fn parse(text: &str) -> Result<Self, SettingsError>;
    /// Every key `Some` from the player; `name` is `Explicit(raw name)`, colour codes included
    /// (the Export upgrader then writes `FromFolder` when that name equals the folder's name
    /// part; the function sees no folder).
    /// `boots_id`/`gloves_id` are the exception: `Some` only for a stored ID from 1 to 100 (0 is
    /// the game's default, an ID above 100 is custom content a folder owns), so they are never
    /// `OutOfRange`. Any other stored value the key's kind cannot represent (a 2-bit sleeves
    /// field holding 3) is `OutOfRange` naming the key.
    pub fn from_player(player: &PlayerEntry) -> Result<Self, SettingsError>;
    /// Sets a stored value; `OutOfRange` when the key's kind cannot represent it, so a
    /// `PlayerSettings` built through `set` is always one the emitters can write.
    pub fn set(&mut self, key: SettingKey, value: u8) -> Result<(), SettingsError>;
    /// Writes the fields onto the player, all or nothing: on `Err` the player is
    /// unchanged. `version` is the compile target: the names are checked against its text
    /// fields and a key its schema lacks (`dribbling` before PES 20) is skipped, set or
    /// not, so an export written for a newer version compiles for an older one as it is.
    /// `NameSetting::FromFolder` is not applied: the caller (the Team compiler)
    /// resolves it to `Explicit` first, since the folder rules are its.
    pub fn apply(&self, player: &mut PlayerEntry, version: PesVersion) -> Result<(), SettingsError>;
    /// The complete template: every key in table order, set ones as values, unset ones
    /// commented at their default, each with its range comment. `OutOfRange` for a value
    /// written directly into a public field that the key's kind cannot represent (the
    /// fields are plain data).
    pub fn to_toml(&self) -> Result<String, SettingsError>;
    /// Edits an existing document in place (`toml_edit`, comments preserved), setting the
    /// `Some` keys and leaving everything else as the user wrote it. `WrongType` when a
    /// table position holds a non-table (never a panic on a user's file).
    pub fn update_toml(&self, document: &mut toml_edit::DocumentMut) -> Result<(), SettingsError>;
}
```

A `PlayerSettings` (with its defaults) also produces the **Fox database row**:
`appearance_row(player_id, version) -> [u8; 60]`, the `PlayerAppearance.bin` row a Fox game
(PES 18–21) dresses a stripped player (appearance id -1) with — the player id, then the 56
appearance bytes his player record holds just after the appearance block's base-copy id
(record bytes 128 to 184 on PES 18/19, 252 to 308 on PES 20/21). The row is one layout on
every Fox version: the games' own tables hold the same bytes for most players from PES 17 to
21, and PES 17's, whose record offset in-game Test 4 confirmed, sits at the same place in the
same 72-byte block, whose fields lie at the same bits on PES 17 to 21 (measured 2026-10-12,
`.tmp/5_1/measure/`). The run's last 4 bytes (the block's last 4) are in no row, so their
measured-unused bits do not reach a stripped player. A pre-Fox version has no row
(`SettingsError::NoAppearanceRow`): PES 17 reads one, but its compile writes the savefile
instead.

The compile writes the rows `appearance_row` produces for a stripped player into its
`PlayerAppearance.bin`; boots and gloves go to the `BootsList.bin`/`GloveList.bin` pairs,
not to the row. For the Save editor's strip and seed a save player's row comes from his
record: `EditFile::appearance_row(player_id) -> Result<[u8; 60], SaveError>`, the player id
and the 56 bytes his record holds after the base-copy id, his entry as the save would write
it, so a value past a key's range is carried as stored, where the settings route
(`PlayerSettings::appearance_row`, still the route for a settings file) refuses it. The 56
bytes are all modeled fields (packed fields, then the ingame-face run kept verbatim; found
at 5.4c1a), so the entry is written over a zeroed record: a copy of the save's record
changed nothing (its mutation run's two survivors said so), and the test comparing rows
with the payload is what fails if a schema ever leaves a gap there. Tested on
each Fox fixture save: an unedited player's row is his record's bytes in the loaded
payload, and an edited field shows in it. A pre-Fox save has no row (an error naming the
version).

**The Fox player tables** live here too, as one type: `PlayerAppearance.bin` (60-byte rows,
the player id then the 56 appearance bytes), `BootsList.bin` and `GloveList.bin` ((player
id, item ID) `u32` pairs), each read whole (a length that is not a whole number of rows is
an error naming the table), edited by player id (set a row, remove a row) and written
sorted by player id. A table is read as a CPK stores it: WESYS-wrapped bytes are unwrapped
first, since every game, PES 15 to 21, ships its `BootsList.bin` and `GloveList.bin`
wrapped (its `PlayerAppearance.bin` and every cup copy plain), and wrapped bytes read as
rows refuse only by luck: PES 2020's wrapped boots list is a whole number of pairs and
would read as garbage (a stream that does not unwrap is an error naming the table).
The tables are written unwrapped, as the cup's copies are. Every table on the maintainer's
machine is, once unwrapped, sorted with each id once (103 distinct tables from 215 CPKs and
42 loose files, PES 15 to 21 installs, cup DLC and Studio's own output, censused 2026-10-16,
worklog 5.6c1). The Team compiler's bins step (its row-selection rules staying in the tool)
and the Save editor's strip and seed edit the same tables; a tool crate cannot call
another's code, and a third copy of a table codec is what this avoids.

Only the Save editor's strip operation writes the appearance id's -1 (worklog issue
"player id versus appearance id").

**The ingame-face run.** From its 22nd byte to its end, the appearance block (72 bytes on PES 16 to
21, 68 on PES 15; a separate record on 15/16, inside the player record from 17) is a run the crate
carries verbatim as bytes with a typed accessor per field: `PlayerAppearance.ingame_face:
IngameFace`, a `Vec<u8>` of the run (50 or 46 bytes; the schema says which). Every bit was measured
in the games (2026-10-13): in Edit mode each player of one team had one setting changed, then the
team was saved once, so the diff per player attributed each bit run to its setting; the rest was
measured from renders of patched saves, and from censuses of the games' own face tables (PES 17 to
21) and of every cup save (PES 15 to 21). PES 21 and PES 15 were measured whole; the renders were
checked on PES 17 and 19. So the run has no undecoded bit left: each bit is a field below, or
measured unused.

**Two layouts.** PES 16 to 21 share one layout of the 50-byte run (`FaceLayout::Pes16To21`). PES
15's 46-byte run (`FaceLayout::Pes15`) shares its face part, its colours and its accessory bits,
and differs at four places:

- byte 34 holds the hair variation, where 16 to 21 hold the hairstyle and cropped style;
- byte 38 bits 3 to 5 hold the hair type, where 16 to 21 hold the wave level;
- bytes 42 bit 4 to 45 are unused, where 16 to 21 hold the rest of the hairstyle;
- the nose field codes the nose angle with the nose type.

It also has no bytes 46 to 49. Every list field is as wide as PES 15 uses it, which is wider than
the ranges of 16 to 21 need. Each extra high bit is 0 in every face the games or the cups made on
16 to 21. Set, the game draws past the models it ships, and PES 21 crashes on a set high bit of
the cropped style. So the ranges of 16 to 21 keep these bits 0.

The fields, as run byte:bit and width in bits, in `settings.toml`'s `[appearance.face]` unless
said. Offsets are relative to the run's first byte:

- **Sliders.** 4 bits each; the key is -7 to 7, stored plus 7. They exist on every version:
  - face: `jawline` 0:4, `jaw_height` 1:0, `cheekbones` 2:0, `cheekbone_height` 2:4,
    `chin_width` 21:4, `chin_height` 22:0, `chin_depth` 22:4, `face_size` 29:0,
    `face_height` 29:4;
  - eyebrows: `brow_width` 4:4, `inner_eyebrow_height` 5:0, `eyebrow_depth` 5:4,
    `temple_width` 6:0, `outer_eyebrow_height` 6:4;
  - eyes: `pupil_size` 7:4, `eye_depth` 8:0, `inner_eye_position` 8:4, `inner_eye_height` 9:0,
    `outer_eye_position` 9:4, `eye_corner_height` 10:0, `upper_eyelid_width_inner` 10:4,
    `upper_eyelid_height_inner` 11:0, `upper_eyelid_width_outer` 11:4,
    `upper_eyelid_height_outer` 12:0, `bottom_eyelid_height` 13:0, `horizontal_eye_position`
    13:4, `eye_height` 14:0;
  - nose: `nose_height` 14:4, `nose_tip_depth` 15:0, `nostril_width` 15:4, `nose_depth` 16:4,
    `nose_width` 17:0;
  - mouth: `mouth_height` 18:0, `mouth_depth` 18:4, `lip_size` 19:0, `mouth_corner_height` 20:0,
    `lip_width` 20:4;
  - ears: `ear_length` 26:4, `ear_width` 27:0, `ear_angle` 27:4.

  The names are PES 21's labels. `cheekbone_height` is PES 15's slider; 16 to 21 have no
  slider for it but still draw it.
- **Sliders only PES 15 draws.** These are stored on every version, and the games' tables fill
  them for every face. The renders of 16 to 21 ignore them. Their editor writes the first three
  with their main slider's value and keeps the others:
  - `pes15_jaw_depth` 1:4;
  - `pes15_cheekbone_depth` 3:0;
  - `pes15_mouth_edge_position` 21:0;
  - `pes15_eyebrow_depth` 7:0;
  - `pes15_bottom_eyelid_position` 12:4;
  - `pes15_nasal_bridge_position` 16:0;
  - `pes15_nasal_root_height` 17:4.
- **Feature types.** These are the existing eleven keys, stored as is, with their fields
  widened as above:
  - `cheek_type` 23:3 w5;
  - `forehead_type` 24:0 w3;
  - `facial_hair_type` 24:3 w5;
  - `laughter_lines_type` 25:0 w3;
  - `upper_eyelid_type` 25:3 w5 (was 3);
  - `lower_eyelid_type` 26:0 w4 (was 3);
  - `eyebrow_type` 28:0 w5 (was 3);
  - `neck_line_type` 28:5 w3 (was 2);
  - `nose_type` 30:0 w6 (was 3);
  - `upper_lip_type` 31:0 w3;
  - `lower_lip_type` 31:3 w3.

  On PES 15 the nose field holds 3 × `nose_type` plus the code of `pes15_nose_angle`, a
  three-value slider: -1, 0 and 1 are coded 1, 0 and 2. On 16 to 21 it holds `nose_type` alone,
  and `pes15_nose_angle` has no field.
- **Colours and two-bit lists.** These exist on every version. Each colour channel is 6 bits,
  0 to 63:
  - `eyebrow_color_red` 35:0, `eyebrow_color_green` 36:0, `eyebrow_color_blue` 37:0;
  - `eyebrow_style` 35:6 w2 (0 normal, 1 thick, 2 thin);
  - `eyebrow_density` 36:6 w2 and `eyebrow_thickness` 37:6 w2 (0 to 3; their labels were not
    read);
  - `facial_hair_color_red` 39:4, `facial_hair_color_green` 40:2, `facial_hair_color_blue` 41:0;
  - `facial_hair_thickness` 41:6 w2.

  "Same as Hair Colour" in the editor copies the colour and stores no flag.
- **`pes15_eye_white` 38:6 w2.** 0 to 3: higher values make the eye whites brighter. It is
  stored on every version but drawn by PES 15 only, and PES 15's editor has no control for it.
- **`[appearance.hair]`, every version:**
  - `color_red` 31:6, `color_green` 32:4, `color_blue` 33:2, 6 bits each;
  - `accessory` 39:0 w1 (on PES 15 the game sets it with the variation);
  - `accessory_color` 39:1 w3 (the eight colours of the gloves row);
  - `color_preset` 43:4 w4.

  `color_preset` is 0 for the custom colour (the RGB above), or the editor's preset 1 to 8,
  stored through the game's internal ids: 1→5, 2→6, 3→7, 4→1, 5→3, 6→2, 7→8, 8→4. Choosing a
  preset keeps the stored RGB.
- **`[appearance.hair]`, PES 16 to 21:**
  - `style` 34:0 w4;
  - `cropped_style` 34:4 w4;
  - `length` 38:0 w3;
  - `wave_level` 38:3 w3 (stored plus 1);
  - `variation` 42:4 w8;
  - `front_style` 44:0 w3;
  - `parted` 44:3 w3;
  - `side_back_style` 44:6 w3;
  - `hairline` 45:1 w3;
  - `forehead_width` 45:4 w4.

  Labels and stored values were measured on PES 21. PES 16 to 20 are assumed equal: every cup
  save stays inside them.
- **`[appearance.hair]`, PES 15:**
  - `pes15_variation` 34:0 w8, stored as the editor's grid position minus 1;
  - `pes15_length` 38:0 w3;
  - `pes15_type` 38:3 w3.

  PES 15's "Default" hairstyle is the edited-hair flag clear, which is a compiler-owned
  `PlayerField`.
- **Unchanged:** `gloves` 0:0, `gloves_color` 0:1 w3, `skin_color` 23:0 w3, `iris_color` 42:0
  w4.
- **Measured unused**, with the value each is written as (`UNUSED_FACE_BITS`):
  - every version: 3:4, 4:0 and 19:4 (4 bits each, 7), and 30:6 w2 (1);
  - PES 16 to 21: bytes 46 to 49 (0);
  - PES 15: 42:4 w8 (0x77), byte 44 (0) and byte 45 (0x37).

  The written values are what the games' own editors and facial reset write. The games' tables
  spread the first three over 0 to 14 and 30:6 over 0 to 2, yet no render on any version moved
  with them. PES 15 shows a change of 1 in 255 over the skin for 30:6, which is invisible.

```rust
/// The two layouts of the run: PES 15's 46 bytes and PES 16 to 21's 50.
pub enum FaceLayout { Pes15, Pes16To21 }   // `FaceLayout::of(version)` picks one

/// A decoded bit run inside the ingame-face run (`schema/ingame_face.rs`, the one place
/// these offsets appear): one variant per field of the map above, each naming the layouts
/// it exists in — every version, PES 15 only, or PES 16 to 21 only (`IngameFaceField::ALL`,
/// the closed set of 92).
pub enum IngameFaceField { PlayerGloves, PlayerGlovesColor, Jawline, /* … */ HairForeheadWidth }

/// `schema/ingame_face.rs`: the fields as `FieldSpec<IngameFaceField>` rows, plus
/// `UNUSED_FACE_BITS`: the measured-unused bit runs with the value each is written as and
/// the layouts they exist in.
pub const INGAME_FACE_FIELDS: &[FieldSpec<IngameFaceField>];

/// Where the run sits in a record that carries it.
pub struct ByteRun { pub byte_offset: u32, pub len: u32 }
// `RecordSchema` gains `pub ingame_face: Option<ByteRun>`: `Some` on the PES 15/16 appearance
// record and the PES 17+ player record, `None` on every other record (15/16 player, team, roster).

/// `model/ingame_face.rs`
pub struct IngameFace(Vec<u8>);   // `Default` is empty: a fresh entry has no run until one is read
impl IngameFace {
    /// `CodecError::NoIngameFaceRun` when the run does not reach the field (an entry never
    /// read from a record), `FieldNotInLayout` for a field of the run's other layout: an
    /// error, not a silent 0, as for an unset gated field.
    pub fn get(&self, field: IngameFaceField) -> Result<u8, CodecError>;
    /// The errors of `get`; `ValueTooWide` when the value does not fit the field's bits.
    pub fn set(&mut self, field: IngameFaceField, value: u8) -> Result<(), CodecError>;
    /// The nose type and angle: PES 15 codes the angle into the nose field (3 × type + the
    /// angle's code), 16 to 21 store the type alone with angle 0. `set_nose` refuses an
    /// angle outside -1..=1 and a non-zero angle on 16 to 21.
    pub fn nose(&self) -> Result<(u8, i8), CodecError>;
    pub fn set_nose(&mut self, nose_type: u8, angle: i8) -> Result<(), CodecError>;
    pub fn bytes(&self) -> &[u8];
    /// The run with the player-gloves and skin bits zeroed: what the fingerprint hashes
    /// (the reference masks the same two, because they overlap fields it lists separately).
    pub fn normalized(&self) -> Vec<u8>;
    // crate-internal: `unused()` (the run with every field of its layout zeroed: the
    // measured-unused bits, the comparator's `FaceRun` row) and `write_unused()` (writes
    // `UNUSED_FACE_BITS` at the games' values, part of a settings.toml application).
}
```

The four fields the tables used to list as bit runs (`SkinColor`, `IrisColor`, `PlayerGloves`,
`PlayerGlovesColor`) leave the per-version tables and the `PlayerField` enum: they are reached
through `IngameFace::get`/`set`, so no byte has two owners and the schema overlap test keeps it
that way. `read_player_into` copies the run out of the record; `write_player` copies it back and
rejects a run of the wrong length (`CodecError::RunSize`). The generator emits the `ingame_face`
entry and omits the four fields; the round-trip tests over every real payload are unchanged and
still prove byte identity. Cross-version conversion (a Team TOML import from another version) is
where the 46-byte PES 15 run meets the 50-byte one: the run is written key by key, a key the
target's layout does not hold dropped with a note (`operations.md` "Cross-version player
conversion").

Every field is a `settings.toml` key (the key table, ranges and defaults in
`aesthetics_export/settings_toml.md`), so `[appearance.unknown]` is gone from `settings.toml`
and Team TOML. Applying a `settings.toml` writes the whole run — every key, absent ones at
their default, and every unused bit at its written value — so a compiled run holds nothing of
the save, as for every other key. A Team TOML import keeps its "absent = untouched" rule, so
the unused bits keep the save's values. Cross-version conversion converts the run field by
field between the layouts (`operations.md` "Cross-version player conversion"), and PES 15's
face caps are measured there.

**Completeness is a tested invariant, not a convention.** `settings_toml` classifies every
`PlayerField` and every `IngameFaceField` exhaustively (an `Ownership::{Settings, CompilerOwned,
Gameplay}` match with no wildcard arm, so a new field fails to compile until classified), and one
test asserts that the `Settings` set equals the set of fields the `SettingKey` table reaches —
a field added to a schema cannot be silently left out of the authorable set. A second test
asserts, per version, that every bit of the appearance block has exactly one carrier — a
`PlayerField` row of the block's record, an `INGAME_FACE_FIELDS` row, or an `UNUSED_FACE_BITS`
row — and that none of it is `Gameplay`, so no undecoded bit lies outside the run. The generated
template (the Export upgrader's and the save editor's `settings.toml` output, and the blank
template for a new player folder) lists **every** `SettingKey`, one per line, each followed by the
comment carrying its range: set ones with their value, unset ones as commented lines (unset
boots/gloves IDs as `""`), so the file
itself shows what can be authored and "absent = untouched" stays true for the unset ones. The key
table, its order, its comments and its ranges are the block in the
[Aesthetics export plan](../aesthetics_export/settings_toml.md) ("Player settings in exports"),
which the code reproduces literally. This matters because the patch workflow below makes
`settings.toml` the *only* route by which a team's aesthetics reach the official save: a field the
file cannot express is a field nobody can set.

`ops/fpc.rs` maps the `libs/fpc` presets onto a player and derives the interference inputs:

```rust
/// Writes one preset's appearance (sleeves, tuck, socks, boots/gloves IDs, skin), all or
/// nothing: every check (custom skin available, the face run present) runs before the first
/// write. The caller substitutes a custom model's own boots/gloves IDs, or an authored stock
/// ID, into `appearance` first (the precedence rules of "Player settings in exports"). `SkinColor::Custom` writes skin 7 on versions that have a
/// custom skin (`fpc::custom_skin_available`) and is `FpcError::CustomSkinUnavailable` elsewhere;
/// `SkinColor::Preset` resets a skin of 7 to 1 (light) and leaves any other skin alone.
pub fn apply(player: &mut PlayerEntry, appearance: &fpc::Appearance, version: PesVersion)
    -> Result<(), FpcError>;
/// The player's strip settings as the interference inputs (reads the face run, hence `Result`).
pub fn strip_style(player: &PlayerEntry) -> Result<fpc::StripStyle, FpcError>;
/// What the save shows, not what the team intends: a boots ID of `ids`'s (the hide ID the
/// settings give), or a custom skin. A hide preset applied with a substituted (real) boots ID
/// is indistinguishable from a dressed player by construction, so this is the save editor's
/// read-only classifier; the compiler, which knows the folder's marker, passes its own
/// `is_fpc_player` to `fpc::check`. Reads the skin through the ingame-face run, hence `Result`.
pub fn is_fpc_player(player: &PlayerEntry, ids: fpc::HideIds) -> Result<bool, FpcError>;
```

The authorable appearance tables are one schema shared by export `settings.toml` and Team TOML.
Full-fidelity Team TOML additionally preserves the save's boots/gloves IDs over their whole stored
range as player-record data; generating an aesthetic export copies them into `settings.toml` only
through `from_player`'s stock-band rule.

The crate also applies the **FPC enable/disable presets**, which
`libs/fpc` owns (Full Player Customization invisibility — nonexistent boots/gloves IDs plus strip
settings; their fields are listed in the [libs/fpc plan](../libs/fpc.md)). Both the save editor's FPC toggle and
the Team compiler's `fpc_on`/`fpc_off` marker files use the same preset values. The compiler
composes the boots/gloves ID fields with asset outcomes by the precedence rule in the
[Aesthetics export plan](../aesthetics_export/settings_toml.md)'s "Player settings in exports";
the other preset fields apply as they are.

---

## Unmodeled payload sections

The crate reads five sections of the payload (the player, appearance, team, roster and tactics
records of `container.md` "Payload layout per version") and their counts,
and writes every other byte back as read. A census of every cup save on the maintainer's
machine (PES 16: 28 distinct saves, 17: 73, 18: 9, 19: 23, 21: 15; 15 and 20 the fixture alone)
measured what those other bytes hold, so that modeling any of them starts here and not from a
new reading of the payload. The schema tables cover 12 to 27% of a payload's bits (PES 15
17.9%, 16 17.2%, 17 19.7%, 18 26.7%, 19 15.7%, 20 12.9%, 21 12.4%), and almost all of the rest
is empty capacity: each section is a fixed array of slots (25,000 player slots on 15/16,
21,000 on 17/18, 32,000 on 19, 30,000 on 20/21; 650 to 850 team slots; 715 to 990 roster and
tactics slots), and the
unused slots are zeros or the `0x0003FFFF` "no team" filler, identical in every save. The data
that varies between saves and no field covers is small, and every piece of it is named by the
community's edit-file wiki pages or by its content. Offsets, strides, counts and what varies
are measured; a name is the wiki's unless "measured" says an Edit-mode save moved it.

### Sections past the records

In payload order on every version, after the header, the count block, the player slots and
the team slots:

| Section | 15 | 16 | 17 | 18 | 19 | 20 | 21 | Varies across saves |
|---|---|---|---|---|---|---|---|---|
| Manager entries, 88 B (u32 id, u16 nationality, u16 picture, a byte, 79-B name) | in the team record | in the team record | 0x410118 × 850 | 0x40A35C × 850 | 0x5FECBC × 1200 | 0x94DDDC × 1500 | 0x958DA4 × 1300 | 17/18: every save; 19: 2 of 22; 21: 10 of 14 |
| Competitions | 0x4A947C, 92 B × 100 | 0x4C1B1C, 92 × 100 | 0x422548, 124 × 50 | 0x41C78C, 124 × 50 | 0x61893C, 760 × 65 | 0x96E17C (49,528 B, not 65 × 760: the stride is undetermined) | 0x974C84, 760 × 65 | 16: 21 saves; 17: 65; 18 holds the cup's competition names; 19/21: little |
| Stadium names (u32 id + name) | 0x4AB86C, 128 × 100 | 0x4C3F0C, 128 × 100 | 0x423D80, 128 × 40 | 0x41DFC4, 128 × 45 | 0x624A34, 128 × 65 | 0x97A2F4, 128 × 64 | 0x980D7C, 188 × 65 | 16: 21 saves; 17: 64; 18/19: identical |
| Kit (strip) entries, keyed by the u32 at +0 ("Kit slots and the strip table") | 0x4AEA6C, 88 × 2500 | 0x4C710C, 88 × 2500 | 0x425180, 132 × 2500 | 0x41F644, 132 × 2500 | 0x626AB4, 132 × 2500 | 0x97C2F4, 132 × 2500 | 0x983D38, 132 × 2500 | 16: 25 saves; 17: every save (155 entries, 33 teams); 18: empty; 19: 17 saves; 21: every save, all 2,500 populated |
| Competition-entry team-id lists (u32 each), after the unused roster slots | 0x506654 | 0x51EC74 | 0x48FAF8, 2,888 B | 0x487FB4, 3,008 B | 0x69DF4C, 3,392 B | 0xA00C0C, 4,656 B | 0xA08650, 4,656 B | 17: 65 saves; 18: 2; 19: 3; 21: 3 |
| Unused tactics slots to the payload's end | 397 KB | 372 KB | 146 KB | 130 KB | 56 KB | 333 KB | 333 KB | identical |

The 17 to 21 starts are the wiki's, each confirmed by the census (the section's stride starts
where the page says); the 15, 16 and 20 starts were located by content (the 92-byte and
128-byte strides, the manager and stadium names). The header's count block (0x3A to 0x4C on
15/16, 0x62 to 0x7C on 17 to 21) holds each table's u16 count: competitions, stadiums,
managers, team-player tables, game plans, and the kit entries (0x68 on 17, 0x6C on 18 to 21),
measured equal to the table's non-empty entries. On 17 to 21 the header's bytes 0x30 to 0x4F
differ in every save: the game rewrites them on every Edit-mode save (a digest or random,
inferred), and a save patched with them left as they were loads and plays, so leaving them as
read is safe. A PES 18 save also rewrote 0x0C to 0x0D and 0x54 to 0x55 (`06 29` to `76 27`,
both pairs), unexplained; PES 16 rewrites no header byte.

### Kit slots and the strip table

Every version's team record holds a block of ten u32 kit slots, each `(team id << 14) |
strip number`: the outfield 1st to 4th strips are 0 to 3 (the editor's Add menu offers those
and the goalkeeper's 1st; 4 to 8 occur on one team of one save), the goalkeeper's 1st is 0x80,
and an unused slot is 0xFFFFC000. The block is at +0x1C on 17 and 18 (measured on 18), +0x20
on 19 (measured) and +0x30 on 20/21 (measured on 21); on 15/16 bytes 24 to 63 are set and
uncovered, the same size, inferred to be the same block. The schema tables carry the
`KitSlotNumber`/`KitSlotTeam` rows for 17 alone ("The team
side" above), so the rows for the other versions are what modeling this adds. Measured on 18, 19 and
21: Edit > Teams > Strip > Add writes the new strip's key into the block's first unused slot
(so a block's order is the order of adding, not numeric: one team holds 0, 1, 0x80, 2, 3), adds
one entry with the same key to the strip table, which stays sorted by key (the entries after
it move down one slot), and adds 1 to the header's kit-entry count; nothing else changes.
The entry's body is the strip's design: on 20/21 a `FF 07 00 50` word and RGB-looking
triplets (`36 36 36` in a default entry), inferred to be the kit colours; the layout is
unmapped. The maintainer's PES 21 install refuses to edit a strip ("You cannot edit this
strip"), on a cup team's 1st and on one just added, so mapping the body by editing needs
another install or another route. A team whose slots name strips with no table entry (PES
19: 158 teams own strips 0, 1 and 0x80 against 15 entries) draws them from the kit configs
the CPK carries, inferred, not tested. In the live saves: PES 18, 89 teams with slots 0, 1 and
0x80 (11 with a 4th, one with all ten) and 96 with none, the table empty; PES 19, 158 teams
with 0, 1 and 0x80 and 52 with none; PES 21, every table entry populated, default teams
included.

### The team record

The tables cover 15 to 26% of the team record's bits. The uncovered bytes by version, with the
wiki's names (what a Save editor's team page would model):

- **15/16** (456 B): 4 to 17 emblem, home stadium, nationality, callname, coach nationality and
  image, turf, net, seats and sideline; 18 to 19 edit flags (the tables' `EditedName` at 0x12:0
  is the wiki's "Unknown A", and its "names edited" is 0x12:7); 24 to 63 the kit slots; 64 to
  75 three rival ids; 76 to 147 0xFF (145 to 147 set in about 600 records); 222 to 342 the
  custom stadium name; 343 to 406 four banner texts at a 16-byte stride; 407 to 455 the
  manager's name.
- **17/18** (480 B): 4 to 17 manager id, emblem, stadium, nationality, callname; 19 to 27
  stadium settings and edit flags (bits 215 and 216 set in about 10,400 of 15,221 records); 28
  to 67 the kit slots (uncovered on 18 alone); 68 to 79 rivals; 80 to 151 0xFF with
  banner-edited flags at 149 to 151; 226 to 346 the custom stadium name; 347 to 410 banners.
- **19** (416 B): 8 to 22; 25 to 31 the wiki's "Unknown B to E" (by 21's layout the goal-net
  colours, media backdrop, stadium settings, the 10-bit edited-settings mask and partner
  images; the census ran them to 51 and started the kit slots at 52, but the measured block
  starts at 32 and the rivals follow it at 72 as on every other Fox version, so the byte
  counts between 25 and 51 are not to be trusted); 32 to 71 the kit slots; 72 to 83 rivals; 85 to 87
  banner flags; 162 to 178 the stadium name; 283 to 344 banners.
- **20/21** (528/588 B): 8 to 22; 25 to 33 goal-net colours, media backdrop RGB, edited-stadium
  flags, goal, turf, sideline and seats, the 10-bit mask; 36 to 47 three partner-image ids
  (0xFFFFFFFF); 48 to 87 the kit slots; 88 to 99 rivals; 101 to 103 banner flags; 359 to 422
  banners; the 181-byte stadium name was set in no save.

Measured on PES 21: Rival Clubs slot 1 is the u32 at +0x58 (0x0003FFFF is Off), and setting it
also set bit 264, inferred to be the 10-bit mask's edited-rivals flag; the team colours are at
the tables' bits (`codec.md` "Reference readings the derivation flagged").

### The tactics record

The tables cover 4,120 of the record's 5,024 bits on 17 to 21 (4,088 of 4,128 on 15, 4,112
of 4,160 on 16, where no uncovered bit is set in any save). On 17 to 21 the uncovered bits
that real saves set are these. Bytes 516 to 523 on 19 to 21 are lineup slots 32 to 39
of the wiki's 40-byte lineup at 0x1E4 (the tables cover slots 0 to 31 as the starting eleven
and the bench); on PES 20, 218 of 220 records hold exactly 32 to 39 there. Byte 537 bit 0 is
Switch Preset Tactics (measured on 21: On to Off cleared it; set in 1,529 of 8,388 PES 19
records and 1,185 of 3,180 PES 21 ones) and byte 539 is Auto Lineup Select (measured on 21:
"Unedited" is 0 and the next entry right, "By Ability", is 2, on which the game re-picked the
lineup at once and swapped two lineup bytes; the other entries were not stepped through). On
17/18 that setting is byte 530 bit 1 (values 2 and 3 in 288 and 25 records). The wiki's PES 19
page lists its offsets after the lineup 3 bytes short; the tables and the data agree with each
other.

### The player record

- Every version: bytes 4 to 7 are the commentary-name player id (a 0x1xxxx value in most
  records, 0xFFFFFFFF in a few; a PES 16 Base Copy rewrites it, with the appearance block's
  base-copy id, to the copied player's); bytes 8 to 9 are 0xFFFF on 15 to 18.
- **PES 15** bits 149 to 155 (between goalkeeping and the free-kick motion) hold 99 in every
  record (on 16 those bits are dribbling); bits 344 to 345 are set in 13 of 20 records.
- **PES 16** bits 222 (0x1B:6, the wiki's "Unknown B") and 362 (0x2D:2, "Unknown D") are set
  together on the 23 players of one team (/gd/, 720) in 19 of the 28 saves and nowhere else.
  Measured: no Edit setting moves them (basic settings, motion, ability with injury resistance
  at its top, skills, COM styles and base copy were tried; positions, playing style,
  appearance, name and create player were not), nothing in the editor shows them, and the game
  keeps them through load, edit and save. Retained, meaning unknown, not editable.
- **PES 17** three one-record bits in byte 51 (noise).
- **PES 18** bits 252 to 254 (0x1F:4, 3 bits; on 17 the dribbling-arm motion sits there). No
  Edit setting moves them, and a dribbling-arm change moved the tables' own field at bit 295,
  so the tables' PES 18 motion offsets hold and these bits are not that motion. Konami's own
  players hold 1 to 7 on all 13,800 of the default save (1: 6,263, 2: 3,730, 3: 2,029, 4:
  1,057, 5: 444, 6: 195, 7: 82; the mean ability rises from 60.6 at 1 to 69.6 at 7, with no tie
  to the arm motions), the live cup save's rostered players 0, and one cup save's Invitational
  teams 2,827 nonzero. Kept through load, edit and save. Retained, meaning unknown, not
  editable.
- **PES 20/21** bytes 176 to 239 are the wiki's 64-byte "Print Name (National Team)" (varying
  at 176 to 191 on 21).
- The appearance block has no uncovered bit; the 20/21 roster record has 40 bytes set in no
  save.

An Edit-mode save also sets the four edited flags at +0x78 (bits 960 to 963: face, hair,
physique, strip) on a Fox player whose screens were visited, whether a value changed or not;
a player whose only visit was the hairstyle picker, confirmed on the entry it showed, stayed
byte-identical, so which screen sets them is not isolated.

### Open points

Not determined by the census: PES 20's competition stride (49,528 B is not 65 × 760); PES 15's
header 0x4A reading 1855 over an all-empty kit table; PES 16's competition-entry start (the
ids begin 128 B before the wiki's size implies); the strip entries' layout beyond the key and
the colour-looking bytes; the 32 header bytes; the edit flags' meanings (the wiki's words,
unmeasured). Not measured in Edit mode: the manager entries, competitions, stadium names, the
commentary-name id, the print name, and the team record's emblem, stadium and banner bytes.
