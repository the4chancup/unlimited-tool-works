# Unlimited Tool Works — Savefile plan: Operations

Part of the [Savefile plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Cross-version player conversion

Moving players from one version's save to another's is a Team TOML import whose document was
written from a save of another version: `TeamToml::from_team` on the source save, then
`TeamToml::apply` onto the target save, which converts when the document's `pes_version` differs
from the target's (the rules of `apply` are under "Team TOML", below).

The document carries canonical values (a playing style by name, a face or motion key as
`settings.toml` names it), so conversion is a property of the import rather than an operation of
its own. A second player-to-player converter would be a second copy of the same rules: the save
editor's cross-version import is `from_team` and `apply`, one team at a time.

### What the Rust module is

The converters were also the compiler of their day. Half of what they write (boots/gloves IDs
against the models present, the edit flags, taping and player gloves cleared) is compile policy,
not layout translation, and in this suite that policy belongs to the Team compiler's
`PlayerSettings`/`ops::fpc` pass.

A cross-version `apply` does the version-dependent half only: **every key the target can hold
is written, what it cannot is capped, dropped or left to the target, and each such case is
reported as an `ImportNote`**, never silently. What depends on the version pair alone and on no
key of the document is not a note: a skill the source version lacks leaves the target's value,
and a PES 15 or 16 target drops the instructions table.

The import writes **into a template**: the target is the player the roster slot names in the
target save, and its values stand for every key the document does not carry or the target cannot
hold.

This is also what settles the ingame-face run's two layouts (`FaceLayout`: PES 15's 46 bytes,
PES 16 to 21's 50). The run is written key by key through `settings_toml`'s shared writer, a
face or hair key the target's layout does not hold is dropped with a `NotInThisVersion` note,
and the target's own hair fields and unused bits stand. The nose is one type key on every layout
plus PES 15's angle key (`pes15_nose_angle`), which a PES 16 to 21 target drops with a note, the
type written alone.

No byte is copied by position between the layouts: PES 15's hair variation sits where 16 to 21
keep the hairstyle and cropped style (`spec/pes_savefile/model.md` "The ingame-face run"). No
byte is invented.

```rust
/// `interchange/team_toml/team.rs`. Converts when `self.pes_version` differs from `to`;
/// all or nothing: on `Err` the team and players are unchanged.
impl TeamToml {
    pub fn apply(&self, to: PesVersion, team: &mut TeamEntry, players: &mut [PlayerEntry])
        -> Result<Vec<ImportNote>, TeamTomlError>;
}
```

Key by key, on a document from another version:

- `name`/`shirt_name` longer than the target's field are cropped with a `TextCropped` note
  ("Portable text limits", below);
- any other player key the target version lacks (a `stats` key, an edit flag) is dropped with a
  `NotInThisVersion` note, every other one written;
- a set skill or COM style the target has no bit for is dropped with a `NotInThisVersion` note
  (`player_field_set`);
- the playing style goes through `schema::playstyle::encode` for the target, a style its list
  lacks written as `None` with a `NotEncodable` note;
- an appearance key the target lacks (a player field, or a face key of the other layout) is
  dropped with a `NotInThisVersion` note;
- each with a `Capped` note:
  - a motion past the target's top is written as the top (below);
  - a face type past `schema::limits::face_type_cap(to, field)` is reset to 0, not clamped;
  - skin 7 is reset to 1 when `fpc::custom_skin_available(to)` is false.

A 16 → 17 import therefore keeps a partial-hide FPC player's custom skin (both converters reset
unconditionally, but each of their directions has a no-custom-skin side; the reference editor's
import resets to 0, the converters to 1, and the converters are the verified path).

The motion ranges narrow from 20/21 to 19 and earlier (corner kick
1–10 vs 1–6, free kick 1–20 vs 1–16, penalty 1–7 vs 1–4, see the `settings.toml` key table).

What the game does with a stored motion past its range was measured on PES 18, from a patched
save:

- a hunching of stored 3 (the field is 2 bits; the editor offers 1 to 3, stored 0 to 2) shows
  "3";
- a corner kick of stored 7 (3 bits, 1 to 6) shows "6";
- a goal celebration of 150 shows "127".

Each shows the list's top, with no crash on load or in Edit mode. Saving the team with another
row of the same screen edited kept the hunching 3 and the celebration 150 but rewrote the corner
kick 7 to 0, the list's first entry, its own row untouched. PES 16 keeps a stored celebration of
183 the same way. Which values a save keeps is per field, so a value past the range is not
stable.

The import therefore caps a motion at the target's top instead of copying it through. The top is
what the game shows for it anyway, and a copied value can turn into the list's first entry on
the player's next in-game save, a change nobody asked for. The cap is a `Capped` note; each
version's tops are measured (`schema::limits::motion_top`, the table of
`aesthetics_export/settings_toml.md` "Player settings in exports").

```rust
/// `schema/limits.rs`: the per-version caps, next to the schemas. `None` for the fields that
/// are not face types (gloves, skin, iris).
pub fn face_type_cap(version: PesVersion, field: IngameFaceField) -> Option<u8>;
```

| Face type | PES 15 | PES 16 | PES 17–18 | PES 19 | PES 20–21 |
|---|---|---|---|---|---|
| cheek | 14 | 3 | 31 | 5 | 3 |
| forehead | 6 | 5 | 7 | 6 | 5 |
| facial hair | 29 | 12 | 31 | 31 | 19 |
| laughter lines | 6 | 4 | 7 | 7 | 4 |
| upper eyelid | 14 | 6 | 31 | 31 | 7 |
| lower eyelid | 14 | 2 | 15 | 15 | 6 |
| eyebrow | 17 | 5 | 31 | 31 | 7 |
| neck line | 5 | 2 | 7 | 7 | 3 |
| nose | 11 | 6 | 63 | 63 | 7 |
| upper lip | 7 | 3 | 7 | 7 | 4 |
| lower lip | 7 | 2 | 7 | 7 | 4 |

The PES 15 column joins the editor's range and the census of the game's faces. Neck line 4 and 5
and lips 7 have no editor entry, but the game draws them as distinct models.

No game measured PES 17's and 18's caps, nor PES 19's other than cheek and forehead. Each such
cap is the field's bit maximum (`INGAME_FACE_FIELDS`), not PES 16's, since real saves hold
values past 16's caps there (79 players of the PES 18 fixture save, `cheek_type` 4 or 5 among
them) and an assumed cap would reset a look the game shows. PES 19's cheek and forehead were
measured in its editor (26 cup players on the maintainer's save use the types past 16's caps).

PES 17 to 19 are therefore wider than 20 and 21 on every type, measured or by their bits: a 19
to 20 conversion resets a cheek 4 or 5, or a facial hair type past 19, like any face type past
the target's cap.

The `settings.toml` key table's widest range for each face key is derived from this table (the
maximum over versions), so the two cannot drift.

**Playing styles.** The stored value is a per-version index. The canonical `PlayStyle` enum
(`model/playstyle.rs`) has the 22 values of the PES 20/21 list (`None` plus 21 named styles);
`schema/playstyle.rs` holds one list per version group and `decode(version, u8) ->
Result<PlayStyle, CodecError>` / `encode(version, PlayStyle) -> Option<u8>` (`None` when the
version has no such style). The lists, from the reference editor's twelve conversion arrays and
checked against every fixture save (a census of style × registered position: goalkeepers sit at
17/18 on PES 15/16, 16/17 on 17/18, 20/21 on 19–21):

| Index | PES 15/16 | PES 17/18 | PES 19 | PES 20/21 |
|---|---|---|---|---|
| 0–3 | None, Goal Poacher, Dummy Runner, Fox in the Box | same | same | same |
| 4 | Prolific Winger | Prolific Winger | Target Man | Target Man |
| 5 | Classic No. 10 | Classic No. 10 | Creative Playmaker | Creative Playmaker |
| 6 | Hole Player | Hole Player | Prolific Winger | Prolific Winger |
| 7 | Box to Box | Box to Box | Roaming Flank | Roaming Flank |
| 8 | Anchor Man | Anchor Man | Crossing Specialist | Crossing Specialist |
| 9 | The Destroyer | The Destroyer | Classic No. 10 | Classic No. 10 |
| 10 | Extra Frontman | Extra Frontman | Hole Player | Hole Player |
| 11 | Offensive Fullback | Offensive Fullback | Box to Box | Box to Box |
| 12 | Defensive Fullback | Defensive Fullback | The Destroyer | The Destroyer |
| 13 | Target Man | Target Man | Orchestrator | Orchestrator |
| 14 | Creative Playmaker | Creative Playmaker | Anchor Man | Anchor Man |
| 15 | Build Up | Build Up | Build Up | Offensive Fullback |
| 16 | *(unused: no player of either fixture save carries it)* | Offensive Goalkeeper | Offensive Fullback | Fullback Finisher |
| 17 | Offensive Goalkeeper | Defensive Goalkeeper | Fullback Finisher | Defensive Fullback |
| 18 | Defensive Goalkeeper | — | Defensive Fullback | Build Up |
| 19 | — | — | Extra Frontman | Extra Frontman |
| 20 | — | — | Offensive Goalkeeper | Offensive Goalkeeper |
| 21 | — | — | Defensive Goalkeeper | Defensive Goalkeeper |

A stored value outside the list (PES 15/16's 16 included) decodes to
`CodecError::UnknownPlayingStyle { version, value }`, not to `None`: no real save carries one,
and a silent 0 is the mistake `Missing` exists to avoid.

The reference editor fills its PES 15/16 combo box with the 17/18 names (mislabelling the two
goalkeeper styles by one). The arrays, not the combo box, are the evidence, and the census
confirms them.

The model keeps `PlayerPositions.playing_style: u8` (the stored index), because the codec does
not know the version and only the interchange formats need the canonical value.

---

## Save-to-save operations

Ported from Midcupping's compare script and the reference editor's comparator, exposed in
the [save editor](../save_editor.md). Midcupping's other script, the aesthetics transplant
(`transplant-aesthetics-*.py`, a donor save's appearance blocks copied into a target save), is
not ported: aesthetics live in the exports, so a look copied into a save is overwritten by the
next compile (`spec/save_editor.md` "Background: Midcupping"). Its byte rule still defines the
appearance block the comparator's aesthetics scope is checked against (below).

- **Aesthetics fingerprint** (`compare-saves-*.py`): a hash of the ingame-face block
  **normalized** by masking the player-gloves and skin-color bits (they overlap fields the
  script lists separately): the first four bytes of SHA-256, printed as eight hex characters
  as the script does. It is how the comparator says "the face changed" without decoding every
  facial parameter, and how a fingerprint printed by the script can be checked against ours.
- **Comparator** (the reference editor's `comparator.cpp` plus `compare-saves-*.py`): every
  stored field of two same-version players, old → new, each row tagged gameplay or aesthetics.

Both are thin operations over the model: they live here so the save editor and any batch
CLI use identical logic. Per-entry functions do the work; the save-level entry point pairs
players by id and checks the version pair, and takes `EditFile`s (the decoded save; `ops/`
still never touches bytes).

### What the reference scripts do

The transplant script rewrites one byte slice of the target's appearance block from the donor's:
bytes 4 to 68 of the 72-byte block (`output[4:68] = playerFace[4:68]`; on PES 15 the block is 68
bytes and the slice is everything but the id; on 17+ the block sits inside the player record and
the same slice applies). That slice is the four appearance edit flags (face, hair, physique,
strip), boots/gloves IDs, the base-copy id, the fourteen physique measures, the strip settings
and the first 46 bytes of the ingame-face run.

It does *not* copy the id, and on 16/17 it leaves the run's last four bytes (a PES 15 slice never
adjusted). Measured: those four bytes are zero on every one of the 24 656 players of the
16/17/18/19/21 fixtures, so copying the whole block is byte-identical to the script on real saves
and one rule instead of two (decision entry, 2.17g). The donor's base-copy id is copied as it
stands (the donor's own id when it has its own face).

The tests keep this rule as `test_support::copy_appearance` (the whole block but the id), the
yardstick of the comparator's aesthetics scope.

The compare script decodes, per player, boots/gloves/face IDs (face = base-copy id, or 0 when it
equals the player's own id), the two wrist tapings, spectacles style, sleeves, inners, socks,
undershorts, shirttail, ankle taping, player gloves and colour, skin, and the normalized run's
hash.

The reference editor's comparator walks each team's roster slots and lists names, id, age,
height, weight, every ability, style, registered position, stronger foot, form, injury, weak
foot, playable positions, COM styles and skills.

Neither compares motion, edit flags, nationality or the physique measures. Ours compares every
stored field, so the two references are each a subset of the rows.

```rust
/// `ops/fingerprint.rs`. The first four bytes of SHA-256 over `IngameFace::normalized()`;
/// `Display` is the eight lower-case hex characters the reference prints.
pub struct FaceHash(pub [u8; 4]);
pub fn face_hash(face: &IngameFace) -> FaceHash;
```

```rust
/// `ops/compare.rs`. Every difference between two players of the same version: the schema's
/// stored fields (the player record's, plus the 15/16 appearance record's) through the model,
/// the two texts, the known ingame-face bits, and the run's measured-unused bits as one row.
/// `Err` only when a gated field the schema stores is `None` on either side (an entry not
/// read from a save of this version).
pub fn compare_players(
    a: &PlayerEntry, b: &PlayerEntry, schema: &VersionSchema,
) -> Result<Vec<PlayerDiff>, CodecError>;

pub enum PlayerDiff {
    /// A bit-run field of the record.
    Field { field: PlayerField, old: u32, new: u32 },
    /// The name or shirt name.
    Text { text: PlayerText, old: String, new: String },
    /// A known bit run of the ingame-face block.
    Face { field: IngameFaceField, old: u8, new: u8 },
    /// Bits of the run no `IngameFaceField` names differ; the hashes are the reference's
    /// fingerprints of the two runs.
    FaceRun { old: FaceHash, new: FaceHash },
}

impl PlayerDiff {
    /// Aesthetics is the appearance block (the scripts' slice above): its fields (the
    /// appearance id, the four appearance edit flags, boots/gloves/base-copy IDs, physique,
    /// strip), every `Face` and `FaceRun` row. Everything else (motion and the other edit
    /// flags included) is gameplay.
    pub fn scope(&self) -> DiffScope;
}
pub enum DiffScope { Gameplay, Aesthetics }

/// Two saves, players paired by id (the compare script's pairing; the reference editor pairs
/// by roster slot, which reports a reshuffle as edits).
pub fn compare(a: &EditFile, b: &EditFile) -> Result<SaveDiff, CompareError>;

pub struct SaveDiff {
    /// Players in both saves with at least one difference, in `a`'s record order.
    pub players: Vec<(u32, Vec<PlayerDiff>)>,
    pub only_in_a: Vec<u32>,
    pub only_in_b: Vec<u32>,
}
pub enum CompareError {
    VersionMismatch { a: PesVersion, b: PesVersion },
    Codec(#[from] CodecError),
}
```

A face-type change is reported once, as its `Face` row: `FaceRun` fires only when bits outside
every known field differ, so the row means "something in the face we cannot name changed". The
hash it carries still masks only gloves and skin (`normalized()`, unchanged), so it equals the
script's printed fingerprint.

Teams compare here too, for the Save editor's unsaved-changes prompt: the team record's runs in
schema order, the two texts (name, then short name), the roster record's runs, then the tactics
record's runs, each through the model.

```rust
/// `ops/compare.rs`. Every difference between two teams of the same version. `Err` only when a
/// gated field the schema stores is `None` on either side (an entry not read from a save of
/// this version), or with `Index` on a roster or tactics entry shorter than the schema.
pub fn compare_teams(
    a: &TeamEntry, b: &TeamEntry, schema: &VersionSchema,
) -> Result<Vec<TeamDiff>, CodecError>;

pub enum TeamDiff {
    /// A bit-run field of the team record.
    Field { field: TeamField, old: u32, new: u32 },
    /// The team name or short name.
    Text { text: TeamText, old: String, new: String },
    /// A field of the roster record.
    Roster { field: RosterField, old: u32, new: u32 },
    /// A field of the tactics record.
    Tactics { field: TacticsField, old: u32, new: u32 },
}
```

`compare` stays player-level (`SaveDiff`): a caller comparing teams pairs the two entries itself
and calls `compare_teams`.

Parity:

- the appearance block is checked against the transplant script's slice rule on every player of
  every fixture (target record after `copy_appearance` + `write_player` equals the record with
  bytes 4..68 replaced from the donor record, the id kept), and each field is aesthetics exactly
  when that copy moves it, the appearance id aside (aesthetics, though the copy keeps the
  target's own);
- the fingerprint and each Midcupping field against the script's bit reads, transcribed as
  literal offsets in the golden test, over every fixture player;
- the comparator against itself (a player compared with its own copy has no rows, a twin
  carrying the next player's appearance block differs from him in aesthetics rows only).

---

## Interchange formats

All player/team interchange formats live in this crate, next to the model they
serialize (the save editor is just UI over them):

### Team TOML (new, replaces `.4ccs`/`.4cct`)

A human-readable full-fidelity dump of one team: team entry (names, colors,
manager/stadium IDs, kit slots), tactics (all 3 presets, formations, instructions,
lineup), and every player with **all** known fields — gameplay stats, skills,
positions, motion, edit flags, and the complete aesthetics including the
ingame-face parameters (which the legacy formats omit). Goals:

- Readable and editable in a text editor without opening Unlimited Tool Works.
- The per-player authorable appearance tables use the exact `PlayerSettings` schema from
  `settings.toml`. Full player records additionally retain boots/gloves IDs over their whole
  stored range for save interchange, at player level; `settings.toml`'s own top-level
  `boots_id`/`gloves_id` accept only the stock band (0 to 100).
- Version-gated fields serialize as optional keys; importing into an older
  version applies the cross-version conversion (caps + playstyle maps) with
  warnings.
- **Absent = untouched on import**, at every level (key, player, team section) — the rule the
  aesthetics patch follows for players and teams.

  An export writes every field, so a round trip is full-fidelity, the bench's last nine
  excepted: the document carries the twelve and an import refills the rest as the game writes
  them. A file written by hand or by the Team creator carries only what its author decided, and
  the save's values stand in for the rest. A partial file is therefore a patch, not a template
  with holes.
- `shirt_name_from(name, version)` lives here: the shirt name derived from a player name under
  the version's field length and character set. Used by the Team creator's defaults and offered
  by the Save editor as a one-click fill; the lib owns the limits, so no tool carries a copy.

**The file.** Every table is addressable by name so that "absent = untouched" holds at every
level (a `[[presets]]` array could not leave preset 2 alone while setting preset 1).

Players are keyed by **roster slot**, never by id: slot `NN` of the file is roster slot `NN` of
the team it is applied to (Texport uses the same rule), so a team's file imports onto any team,
and a file never carries a player id.

Gameplay values are the **stored** numbers (the record's, ranges from the bit widths), because
this is a dump, and the manager-facing reinterpreted numbers belong to `settings.toml`. What the
model already names canonically is written as a label: positions, playable ratings, the playing
style, skills, COM styles, the advanced instructions, the style switches, the stronger
foot/hand.

The `[players.NN.appearance]` table is `settings.toml`'s `[appearance]` table verbatim (same
keys, same kinds, same ranges, parsed and emitted by `settings_toml`'s own parser and emitter),
so a player's aesthetics can be pasted between the two files. There is one difference of policy,
not of keys: Team TOML reads and writes the *stored* value of every field (real saves hold face
types and celebrations past the editor's ranges, and a dump must carry them), where
`settings.toml` keeps the editor ranges (decision entry, 2.17h-3).

```toml
# team.toml — one team of one save. Absent = untouched on import, at every level.
pes_version = 19                # the PES version of the save this was exported from; importing into another version converts the values

[team]
id = 841                        # the team this was exported from; the command line imports onto it, the Save editor onto the open team
name = "/98hu/"                 # colour codes as in the save
short_name = "I1"
manager_id = 1000               # PES 17 and later
stadium_id = 12
color_1 = [31, 58, 15]          # PES 17 and later; red, green, blue: 0 to 63 each
color_2 = [63, 63, 63]
kit_slots = [{ number = 0, binding = 0 }, ...]   # PES 17 only; ten entries, values as stored

[team.edit_flags]
name = true
short_name = false              # PES 15 only
stadium = false                 # PES 20 and later
strip = false                   # PES 17 only

[tactics]
starting_eleven = [0, 10, 3, 1, 5, 4, 9, 2, 6, 7, 8]  # roster slots, counting from 0
bench_order = [11, 12, ...]     # the bench: 12 roster slots, counting from 0
players_to_join_attack = [255, 255, 255]             # roster slots, counting from 0; 255 = nobody

[tactics.set_pieces]            # roster slots, counting from 0; 255 = nobody
free_kick_long = 3
free_kick_short = 3
free_kick_second = 4
corner_left = 5
corner_right = 5
penalty = 9
captain = 0

[tactics.auto]
substitution = 0                # as the game stores it
offside_trap = false
preset_change = false
attack_defence_levels = false   # PES 17 and later

[tactics.preset_1]              # preset_1 .. preset_3
man_marking = [255, 255, 255, 255, 10, 255, 255, 2, 4, 255, 5]   # for each formation slot, the opponent's formation slot he marks; 255 = nobody
[tactics.preset_1.style]
attacking_style = "possession"  # "counter_attack" | "possession"
attacking_zone = "wide"         # "centre" | "wide"
buildup = "short_pass"          # "long_pass" | "short_pass"
positioning = "flexible"        # "maintain" | "flexible"
defensive_style = "frontline_pressure"   # "frontline_pressure" | "all_out_defence"
containment_area = "middle"     # "middle" | "wide"
pressure = "aggressive"         # "aggressive" | "conservative"
fluid = false                   # PES 16 and later
[tactics.preset_1.sliders]
support_range = 4               # 1 to 10
defensive_line = 3              # 1 to 10
compactness = 7                 # 1 to 10
numbers_in_attack = 3           # 1 few, 2 medium, 3 many
numbers_in_defence = 3          # 1 few, 2 medium, 3 many
# players: in the order of starting_eleven (its first entry is the first player here); roster slots count from 0, so slot 0 is [players.01]
[tactics.preset_1.formation_1]  # formation_1 kick-off, formation_2 in possession, formation_3 out of possession
players = [{ position = "GK", x = 52, y = 2 }, ...]  # eleven; position "GK" .. "CF"; x, y as stored
[tactics.preset_1.instructions] # PES 17 and later; instruction names, converted to the target version's on import
attack = [{ instruction = "hug_the_touchline", player = 3 }, { instruction = "off", player = 0 }]
defence = [{ instruction = "off", player = 0 }, { instruction = "off", player = 0 }]

[players.01]                    # roster slot 01 to 40 (01 to 32 on PES 15 to 18)
name = "YOUR SKILL: A FAILURE"
shirt_name = "SCORE"
number = 1                      # shirt number
nationality = 231
age = 25
height = 180                    # cm
weight = 75                     # kg
boots_id = 3601
gloves_id = 0
base_copy_id = 84101            # the player whose look this one copies; his own id (the team id times 100 plus the slot) means none, and an import turns it into the target player's own id

[players.01.stats]              # stored values; the comments give the game's editor range, the parser accepts the field's stored range
attacking_prowess = 77
ball_control = 77
dribbling = 77
tight_possession = 77           # PES 20 and later
low_pass = 77
lofted_pass = 77
finishing = 77
heading = 77
place_kicking = 77
swerve = 77
defensive_prowess = 77
ball_winning = 77
aggression = 77                 # PES 20 and later
kicking_power = 77
speed = 77
explosive_power = 77
body_control = 77
physical_contact = 77           # PES 17 and later
jump = 77
stamina = 77
goalkeeping = 40
catching = 40
clearing = 40                   # PES 16 and later
reflexes = 40                   # PES 16 and later
coverage = 40                   # PES 16 and later
weak_foot_usage = 1             # 0 to 3
weak_foot_accuracy = 1          # 0 to 3
form = 3                        # 0 to 7
injury_resistance = 1           # 0 to 2
star = 2                        # PES 19 and later, 0 to 7
playing_attitude = 0            # PES 20 and later

[players.01.positions]
registered = "CF"               # "GK" "CB" "LB" "RB" "DMF" "CMF" "LMF" "RMF" "AMF" "LWF" "RWF" "SS" "CF"
playable = { CF = "A", SS = "B", GK = "C", ... }   # thirteen keys; "C" | "B" | "A" (stored 0-2; a stored 3, which no save holds, reads "3")
playing_style = "goal_poacher"  # the playing style's name, or "none"
stronger_foot = "right"         # "right" | "left"
stronger_hand = "right"         # PES 20 and later

[players.01.skills]
skills = ["scissors_feint", "heading"]      # the player's skills (the names are listed below)
com_styles = ["trickster"]                  # "trickster" "mazing_run" "speeding_bullet" "incisive_run" "long_ball_expert" "early_cross" "long_ranger"

[players.01.edit_flags]         # the parts the game marked as edited, true or false
player = true
basic_settings = true
registered_position = false
playable_positions = false
abilities = true
skills = false
playing_style = false
com_styles = false
motion = false
base_copy = false
face = true
hair = true
physique = false
strip = false

[players.01.appearance]         # settings.toml's [appearance] table, verbatim
skin_color = 1
# ... [players.01.appearance.physique] / .strip / .motion / .face / .hair, as in settings.toml
```

Skill names, in the model's slot order 0-40: `scissors_feint flip_flap marseille_turn sombrero
cut_behind_and_turn scotch_move heading long_range_drive knuckle_shot acrobatic_finishing
heel_trick first_time_shot one_touch_pass weighted_pass pinpoint_crossing outside_curler rabona
low_lofted_pass low_punt_trajectory long_throw gk_long_throw malicia man_marking track_back
acrobatic_clear captaincy super_sub fighting_spirit double_touch crossover_turn step_on_skill
chip_shot dipping_shots rising_shots no_look_pass gk_high_punt penalty_specialist
gk_penalty_specialist interception long_range_shooting through_passing`.

Advanced instruction names, in the canonical 17-based order 0-16: `off hug_the_touchline
false_no_9 false_full_backs attacking_full_backs wing_rotation tiki_taka centering_targets
swarm_the_box deep_defensive_line gegenpress tight_marking counter_target defensive false_winger
wing_back anchoring`. The per-version encodings are `schema/instruction.rs` (below).

The reference editor's table ends at `wing_back` and is right for PES 17-19 only. On 20/21
Anchoring is stored as 0x0A, 18/19's 0x0A-0x0F moving to 0x0B-0x10 (measured in-game on PES 21);
it is canonical 0x10 here and absent before PES 20.

Three instructions take one of the team's own players as their target, its roster slot in the
entry's `player`: `counter_target`, `defensive` and `anchoring` (`Instruction::takes_player`).
Under every other instruction the saves hold 0 or 0xFF there, and nothing reads it.

```rust
/// `model/instruction.rs`. The canonical advanced instruction, in the reference editor's
/// 17-based order: 0x00-0x0C are PES 17's own values, 0x0D-0x0F the three PES 18 added,
/// 0x10 the one PES 20 added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instruction { Off, HugTheTouchline, FalseNo9, FalseFullBacks, AttackingFullBacks,
    WingRotation, TikiTaka, CenteringTargets, SwarmTheBox, DeepDefensiveLine, Gegenpress,
    TightMarking, CounterTarget, Defensive, FalseWinger, WingBack, Anchoring }

/// `schema/instruction.rs`. PES 17 stores the canonical value; PES 18/19 store 0x01-0x07 as is,
/// Defensive/FalseWinger at 0x08/0x09, SwarmTheBox..CounterTarget at 0x0A-0x0E, WingBack at
/// 0x0F; PES 20/21 insert Anchoring at 0x0A, SwarmTheBox..CounterTarget moving to 0x0B-0x0F and
/// WingBack to 0x10. PES 15/16 have no instructions (`None` for every
/// value). A stored value outside the version's list is
/// `CodecError::UnknownInstruction { version, value }`.
pub fn decode(version: PesVersion, stored: u8) -> Result<Option<Instruction>, CodecError>;
/// `None` when the version has no such instruction (a PES 17 target for the three PES 18 ones,
/// a 19-or-earlier target for Anchoring).
pub fn encode(version: PesVersion, instruction: Instruction) -> Option<u8>;
```

```rust
/// `interchange/team_toml/`. The document: every leaf `Option`, `None` = absent.
pub struct TeamToml {
    pub pes_version: Option<PesVersion>,
    pub team: TeamSection,          // the [team] keys and [team.edit_flags]
    pub tactics: TacticsSection,    // [tactics], .set_pieces, .auto, preset_1..3 with their sub-tables
    pub players: BTreeMap<u8, PlayerSection>,   // roster slot (1-based, as in the key) -> the player's tables
}
/// The sections mirror the tables, field names = key names, every leaf `Option`; what the TOML
/// writes as a label is the model's value here (positions `u8`, switches `bool`, `PlayStyle`,
/// `Instruction`), the label tables being the parser's and emitter's alone.
pub struct TeamSection { pub id: Option<u32>, pub name: Option<String>, pub short_name: Option<String>,
    pub manager_id: Option<u32>, pub stadium_id: Option<u16>, pub color_1: Option<TeamColor>,
    pub color_2: Option<TeamColor>, pub kit_slots: Option<[KitSlot; 10]>, pub edit_flags: TeamEditFlagsSection }
pub struct TeamEditFlagsSection { pub name: Option<bool>, pub short_name: Option<bool>, pub stadium: Option<bool>, pub strip: Option<bool> }
pub struct TacticsSection { pub starting_eleven: Option<[u8; 11]>, pub bench_order: Option<[u8; 12]>,
    pub players_to_join_attack: Option<[u8; 3]>, pub set_pieces: SetPiecesSection, pub auto: AutoSection,
    pub presets: [PresetSection; 3] }
pub struct SetPiecesSection { /* the seven takers, `Option<u8>` each, names as in `SetPieceTakers` */ }
pub struct AutoSection { pub substitution: Option<u8>, pub offside_trap: Option<bool>, pub preset_change: Option<bool>, pub attack_defence_levels: Option<bool> }
pub struct PresetSection { pub man_marking: Option<[u8; 11]>, pub style: StyleSection, pub sliders: SlidersSection,
    pub formations: [Option<[FormationSlot; 11]>; 3], pub instructions: Option<InstructionsSection> }
pub struct StyleSection { /* the seven switches + `fluid`, `Option<bool>` each, names as in `TacticStyle` */ }
pub struct SlidersSection { /* the five sliders, `Option<u8>` each, names as in `TacticSliders` */ }
pub struct InstructionsSection { pub attack: [InstructionEntry; 2], pub defence: [InstructionEntry; 2] }
pub struct InstructionEntry { pub instruction: Instruction, pub player: u8 }
/// One `[players.NN]`.
pub struct PlayerSection { pub name: Option<String>, pub shirt_name: Option<String>, pub number: Option<u16>,
    pub nationality: Option<u16>, pub age: Option<u8>, pub height: Option<u8>, pub weight: Option<u8>,
    pub boots_id: Option<u32>, pub gloves_id: Option<u32>,
    pub base_copy_id: Option<u32>,
    pub stats: StatsSection,
    pub positions: PositionsSection, pub skills: SkillsSection, pub edit_flags: EditFlagsSection,
    pub appearance: AppearanceSettings }
pub struct StatsSection { /* one `Option<u8>` per `[players.NN.stats]` key, names as in the block */ }
pub struct PositionsSection { pub registered: Option<u8>, pub playable: Option<[u8; 13]>,
    pub playing_style: Option<PlayStyle>, pub stronger_foot: Option<u8>, pub stronger_hand: Option<u8> }
pub struct SkillsSection { pub skills: Option<[bool; 41]>, pub com_styles: Option<[bool; 7]> }
pub struct EditFlagsSection { /* the fourteen flags, `Option<bool>` each, names as in `PlayerEditFlags` */ }

/// `interchange/legacy.rs`: the read-only formats, each into a `TeamToml` (see below).
/// `.4ccs` records carry no player id, so they are read against the team they import into.
pub fn read_squad(bytes: &[u8], team: &TeamEntry, players: &[PlayerEntry]) -> Result<TeamToml, LegacyError>;
pub fn read_tactics(bytes: &[u8]) -> Result<TeamToml, LegacyError>;   // .4cct
pub enum LegacyError { BadTag { expected: &'static str, found: String }, BadVersion(String), BadTeamId(String),
    Truncated { needed: usize, len: usize }, UnknownPlayingStyle { version: PesVersion, value: u8 },
    BadInstruction(u8), TooManyPlayers { count: usize }, MoreRecordsThanRoster { records: usize, rostered: usize },
    Value(TeamTomlError), Text(String) }

/// What an import did not apply as written. Version-pair-wide facts are not notes (the 2.17f rule).
pub enum ImportNote {
    /// A key the target version has no field for (`tight_possession` into PES 19); dropped.
    NotInThisVersion { path: String },
    /// A label the target version cannot encode (a PES 18 instruction into 17, a style the
    /// target's list lacks); written as `off`/the list's rule.
    NotEncodable { path: String, label: String },
    /// A face type above the target's cap (reset to 0, the 2.17f rule) or skin 7 into a
    /// no-custom-skin version (reset to 1); `to` is what was written.
    Capped { path: String, from: u8, to: u8 },
    /// A `[players.NN]` whose target roster slot is empty; skipped.
    EmptySlot { slot: u8 },
    /// A player's `name`/`shirt_name` longer than the target's field, cut to it at whole
    /// characters and colour codes; `cropped` is the text as stored.
    TextCropped { path: String, cropped: String },
}

impl TeamToml {
    /// Every field of the team and its roster's players, `Some`. `version` is the save's. An
    /// empty `players` gives a team/tactics-only document; a non-empty one missing a rostered
    /// id is `PlayerMissing` (a partial pool is a caller bug, a silent drop is data loss).
    pub fn from_team(version: PesVersion, team: &TeamEntry, players: &[&PlayerEntry]) -> Result<Self, TeamTomlError>;
    pub fn parse(text: &str) -> Result<Self, TeamTomlError>;
    /// The plan block's layout: one key per line, tables in the block's order, the range comments.
    pub fn to_toml(&self) -> Result<String, TeamTomlError>;
    /// Applies the `Some` fields onto `team` (id kept) and onto the players its roster slots
    /// name, all or nothing; `to` is the save's version, conversion happens when it differs from
    /// `pes_version`. Team TOML never adds players: an empty slot is a note.
    pub fn apply(&self, to: PesVersion, team: &mut TeamEntry, players: &mut [PlayerEntry]) -> Result<Vec<ImportNote>, TeamTomlError>;
}
/// `shirt_name_from`: the name upper-cased, colour codes stripped, cut to the version's shirt-name
/// field (the schema's text length minus the terminator) by characters. Unlike every other text
/// rule (`codec.md` "Text fields hold `len` bytes"), a generated shirt name keeps its NUL: it is
/// new text, and no in-game check has yet shown a full-field shirt name displaying whole.
pub fn shirt_name_from(name: &str, version: PesVersion) -> String;
```

Rules of `apply`:

- `pes_version` absent means "the target's version" (no conversion).
- A `bench_order` writes its twelve and refills the record's other nine as the game writes them
  (the roster slots from 0 that neither the eleven nor the twelve name, ascending, the eleven
  being the file's `starting_eleven` when it has one, the record's else).
- The `appearance` table's keys alone, each as its stored value (a key the table lacks keeps its
  value: "absent = untouched", where `settings.toml` and the patch write every field), go
  through `settings_toml`'s shared writer, the run's measured-unused bits keeping the save's
  values.
- An import never sets a player's appearance id.
- `base_copy_id` equal to the file's own player id (the file's team id × 100 + slot) becomes the
  target player's id.
- A `stats` key the target version lacks is `NotInThisVersion` when the versions differ and
  `TeamTomlError::NotInThisVersion` when they are equal (a same-version file with a foreign key
  is a mistake, a converted one is expected to shed keys).
- A skill or COM style the version cannot store, in a file whose `pes_version` equals the
  target's or names none (`double_touch` on PES 17), is `TeamTomlError::NotInThisVersion` the
  same way, since writing it would mean the member's skill silently never lands. A converted
  file keeps the pair-wide rule instead: a skill the *source* version lacks is not written (the
  target's stands, no note) and a skill the target lacks is dropped with a note.
- The team's texts must fit the target version's text fields or `apply` is `TextTooLong` before
  any write, while a player's `name`/`shirt_name` longer than the target's field is cropped to
  it with a `TextCropped` note ("Portable text limits", below).
- The playing style goes through `schema::playstyle::encode` for the target (2.17f's lists);
  instructions through `schema::instruction::encode`.
- PES 15/16 targets drop the instructions table silently (the version has none: a pair-wide
  fact).

What `shirt_name_from` keeps of the character set beyond upper-casing is unmeasured (the game's
own edit screen restricts it); the function is the one place to tighten when it is.

**Portable text limits.** A Team TOML is meant to apply to a save of any version, so
`pes_savefile` exposes what every version can hold, computed from the schema tables rather than
typed in (`portable_text_max(PlayerText)`):

- a player's name is the shortest name field of PES 15 to 21 (46 bytes; 20 and 21 hold 61);
- a shirt name the shortest of PES 15 and 17 to 21 (18 letters; PES 16's field, 16, is the one
  outlier left out).

A name is not held to 46 bytes where it is written: a member may want a long name on the
versions that show it.

`apply` onto a version whose field is shorter than a name or shirt name crops it to that field,
with a `TextCropped` note naming the player's key and the text as cut, never a refusal. The crop
is `crop_player_text(version, PlayerText, &str) -> Cow<str>`, the longest prefix of whole spans
(`name_spans`: a character or a colour code) that fits, so a crop never splits a character or
leaves half a colour code drawn as text. The cases real files reach:

- a PES 20/21 name past 46 bytes applied to PES 15 to 19;
- a shirt name past 16 letters applied to PES 16.

Export from a save writes every name as the save holds it (the dump stays full-fidelity), with
a note on the line of each name past 46 bytes or shirt name past 18 letters, naming the
versions that crop it. A crop is never silent: real cup saves show names cut at 45 bytes when a
team moved from PES 21 to PES 17 (2026-10-21: 12 of 4,830 names in a PES 21 save were past 46
bytes, the longest 59), and a cut nobody was told about renames the player.

### Aesthetics patch (new)

The file the Team compiler writes to its output folder with its CPKs and the save editor applies: the resolved
savefile writes for the compiled players, so that the people who build the DLC and the people who
build the official savefile can be different people. The DLC builder compiles the cup's exports
without ever seeing the savefile builder's data (the teams' custom tactics, which they send to the
savefile builder alone), hands over the CPK and the patch, and the savefile builder applies the
patch to the official save.

`interchange/aesthetics_patch` holds the format; what is still planned is in
[`plans/pes_savefile/operations.md`](../../plans/pes_savefile/operations.md) "Aesthetics patch
(new)".

The Team compiler writes the file to the output folder with the CPKs a compile publishes, its
header naming the CPKs (empty for a sideload tree, which publishes loose files).

A compile with nothing to put in it (no player holding a key once the IDs of tasks that did not
commit are dropped) writes none and moves an earlier patch to `aesthetics_patch.toml.bak`, one
backup replaced each time. The write happens once the run's outputs are in place, so a run that
publishes nothing leaves the previous CPK and its patch alone.

The compiler's own savefile stage is "apply the patch just produced to the local save" (see
"Savefile update" in the [Team compiler plan](../team_compiler/pipeline.md)) — one path, so a
patch applied a week later by someone else yields the same bytes the DLC builder's machine
wrote.

**The patch is the one route by which a compiled player's look reaches the game, on every
engine.** The compiler writes no `PlayerAppearance.bin`, `BootsList.bin` or `GloveList.bin`
rows and no save is stripped. A patch's `players.NN` is player id `team id × 100 + NN`, the
export's savefile identity (`spec/aesthetics_export/player_folders.md` "Savefile identity"), whatever roster slot he
holds, so the patch lands on the player whose models the CPK carries. Per compiled player an
entry holds:

- the **`[appearance]` tables**, the `PlayerSettings` schema as his `settings.toml` and
  `ingame_face.toml` give it together (the face, hair and iris from the latter), every key the
  version holds written, with the defaults for keys his files lack, so his look is his
  export's whatever the save held. The motion keys join the others there.
- **`boots_id` and `gloves_id` over the field's whole stored range** (14 bits on every
  version, 0 to 16383), not `settings.toml`'s stock band. The resolved ID is the one the
  order in the [Aesthetics export spec](../aesthetics_export/settings_toml.md)'s "Player
  settings in exports" reaches (a committed custom output's assigned ID, else the authored
  stock ID 0 to 100, else the FPC marker's default). A category whose requested output
  failed, or a `Midcup` export's markerless default, carries no key, so the save keeps its
  current ID.

  The compiler plans the patch before its tasks run and fills these two keys once each task's
  outcome is known, then writes the file once: the file never holds an ID conditioned on a
  task.
- **`base_copy_id`**, the id of the player whose face and hair he copies; `apply` writes it
  as his base copy with the flag left as the save holds it, where an `[appearance]` entry
  without one gets its copy cleared to his own id ("Applying a look", below). The
  player-folder link that fills it is the plan's.
- **the `name` and `shirt_name` of the players of an export with the `write_names` marker**
  at its root (typically an autopilot team, whose tactical export carries placeholder
  names). It is applied whole, with no team picker: a managed team's export has no marker,
  so its `name` and `shirt_name` are not used and its players keep the names their manager
  set.

**No look for a player whose `settings.toml` or `ingame_face.toml` was invalid**: no
`[appearance]` and no ID key his settings would resolve, neither an authored stock ID nor
the default a marker or a `Full` export gives, so the save keeps his record and his IDs. An
ID his folder's models or links get still lands. A player left with nothing to write gets no
entry at all, so `apply`'s summary lists only players it wrote.

Both files' keys go in the one `[appearance]`, so one invalid file holds back the other's, not
a half look built from defaults. An invalid file is an Error a member reads and fixes, and a
partial table would need an `apply` that writes some keys and leaves others, a second write path
for a case the next compile corrects.

The file stays readable as the export's own files are: the same key names, tables and
values as `settings.toml` and `ingame_face.toml` (`sleeves = "short"`, a motion as the game
shows it), the face, hair and iris under the player's `appearance` as the joined schema
holds them, so a reader compares a patch with a player's files key by key. What differs is
that every value is resolved (no `""` default, no `name = true`, the FPC preset applied) and
that there are no per-key range comments: the templates carry those.

An `[appearance]` is a whole look or absent: reading a patch refuses one missing a key the
version holds, since a missing key means a broken patch whose apply would reset the player's
other keys.

```toml
# aesthetics_patch.toml, written by the Team compiler to its output folder
pes_version = 21
tool_version = "1.4.0"
allocation_scheme_version = 1
cpks = ["4cc_69.cpk"]          # the CPKs the run published; none for a sideload tree
compiled = 2026-09-12T14:03:00Z

[[teams]]
id = 702
name = "/a/"                 # for the reader; the id is what is applied

[teams.players.03]           # slot 03 → player 70203
name = "Snuffy"              # a write_names export's only; absent = not written
shirt_name = "SNUFFY"        # likewise
boots_id = 143               # the resolved ID; absent = the save keeps his
gloves_id = 0
base_copy_id = 101334        # a <id>.copy link's id; absent, an [appearance] clears his copy

[teams.players.03.appearance]
skin_color = 2
# … every key the version holds: the physique, strip, motion, face, hair and iris tables
```

**Applying a look also makes the record his.** For a player whose entry holds
`[appearance]`, `apply`:

- writes it through `PlayerSettings::apply` (every key, the unused bits at the games' values);
- sets the appearance block's player id to his own id, as real saves and `populate_players`
  hold it;
- and clears his base copy: the base-copy id set to his own id (the record's "no copy") and the
  base-copy flag, on the versions that hold one, cleared; unless the entry holds
  `base_copy_id`, which `apply` writes as his base copy, the flag left as the save holds it.

The id, because a save still at -1 from the old stripped route (the maintainer's test saves)
would otherwise ignore the written look.

The base copy, because a record whose base-copy id names another player shows that player's
face and hair over his own. In PES 21's Edit mode (2026-10-15) a cup save's "MESSI" copying
player 7511 and a teammate copying 114624 showed the copied faces. The game shows a copied face
with the face edit flag clear as with it set (the same save, that flag alone cleared), and over
the player's own `face.fpk` too (two players with one each, flag set and clear, given a base
copy). A census of 125 Fox saves on the maintainer's machine (2026-10-14,
`.tmp/phase_05/5_4/base_copy_census.txt`) found such an id on 0 to 26 players of each real
PES 21 cup save, up to 117 on PES 19 and 988 on PES 18; counted are ids neither 0 nor the
player's own. A compiled player whose export describes no face of his own (no face model, no
`ingame_face.toml`) then shows the Placeholder face, as his export says, where the copy
showed someone else's.

A written name or shirt name sets the edited-player flag, as the Save editor's own name edit
does: on PES 2019 a name edit shows in game only with that flag set, and alone it suffices (in
game, 2026-11-08; `DECISIONS.md` "a written name sets the edited-player flag"). The other edit
flags (the record's `Edited*` bits) but the base-copy flag are left as the save holds them.

Rules:

- **Everything is resolved.** `name = true` is the derived string; `fpc_on`/`fpc_off` are the
  preset's concrete field values; the IDs are the resolved ones, already filtered by the compiler's
  "only for content that was actually packed" rule. The patch answers "what will the save contain",
  never "what did the export say" — the export is not needed to apply it.
- **Applying is `pes_savefile`'s operation** (`AestheticsPatch::apply`): for each player, apply
  what the file holds, through the same `apply_appearance`
  `settings_toml` shares — one path, so a patch applied by someone else yields the same bytes
  the DLC builder's machine wrote.
- `pes_version` must equal the save's (`VersionMismatch`); there is no cross-version application —
  the compile targeted a version and its values are that version's. `allocation_scheme_version`
  must equal the suite's `ALLOCATION_SCHEME_VERSION` (`SchemeMismatch`), so a patch from another
  scheme is refused. A team or player ID absent from the save is skipped and reported in
  `PatchSummary` while the rest applies; referees are team 999 like anywhere else.
- **A patch is refused whole or applied whole, absences aside.** Every check that can fail
  (the two versions, a name longer than the version's field, an appearance key's value outside
  its range or its field's width, a key the version does not hold) runs before the first
  write, and a failure leaves the save untouched: the patch is the compiler's output, so a bad
  value means a broken patch, and a half-applied one would leave a save matching no
  compile. Only a team or player absent from the save is skipped per item, as above.

The types (`interchange/aesthetics_patch/mod.rs`):

```rust
/// The suite's boots and gloves ID allocation scheme (`team_compiler/pipeline.md`); a patch
/// from another scheme is refused.
pub const ALLOCATION_SCHEME_VERSION: u32 = 1;

/// `aesthetics_patch.toml`: the compiler's resolved savefile writes for its compiled players.
pub struct AestheticsPatch {
    pub pes_version: PesVersion,
    pub tool_version: String,
    pub allocation_scheme_version: u32,
    /// The CPKs the run published; empty for a sideload tree.
    pub cpks: Vec<String>,
    pub compiled: toml_edit::Datetime,
    pub teams: Vec<PatchTeam>,
}

pub struct PatchTeam {
    pub id: u32,
    /// For the reader; the id is what is applied.
    pub name: String,
    /// By `NN` (`players.NN`, player id `id × 100 + NN`).
    pub players: BTreeMap<u8, PatchPlayer>,
}

pub struct PatchPlayer {
    /// A `write_names` export's only; `None` = not written.
    pub name: Option<String>,
    pub shirt_name: Option<String>,
    /// The resolved boots ID over the field's whole stored range (0 to 16383); `None` =
    /// the save keeps his.
    pub boots_id: Option<u16>,
    /// Like `boots_id`.
    pub gloves_id: Option<u16>,
    /// A `<id>.copy` link's id, written as his base copy; `None` = no link.
    pub base_copy_id: Option<u32>,
    /// His whole look, every key the version holds; `None` = the save keeps his look.
    pub appearance: Option<AppearanceSettings>,
}

impl AestheticsPatch {
    pub fn parse(text: &str) -> Result<Self, PatchError>;
    /// `Err` for a value `parse` would refuse back (a hand-built one).
    pub fn to_toml(&self) -> Result<String, PatchError>;
    /// Every refusal before any write (above); then each team and player present is written.
    pub fn apply(&self, save: &mut EditFile) -> Result<PatchSummary, PatchError>;
}

pub struct PatchSummary {
    /// The ids of the patch's teams the save holds, in file order.
    pub applied_teams: Vec<u32>,
    /// The ids of the players written, in file order, a team's by slot. Empty means the
    /// save is unchanged.
    pub applied_players: Vec<u32>,
    pub skipped: Vec<PatchSkip>,
}

pub enum PatchSkip {
    TeamNotInSave { team: u32 },
    PlayerNotInSave { player: u32 },
}

/// The refusals, each naming what failed; a key-level one (unknown, wrong type, out of
/// range, a name too long) in `settings.toml`'s vocabulary, at the key's dotted path.
pub enum PatchError {
    Toml(String),
    MissingKey { key: String },
    VersionMismatch { patch: PesVersion, save: PesVersion },
    SchemeMismatch { patch: u32, suite: u32 },
    Settings(SettingsError),
}
```

### Legacy 4ccEditor formats (read-only)

Both are raw dumps of the reference editor's in-memory structs; both parse into a `TeamToml`
(`interchange/legacy.rs`), so the import path is Team TOML's and there is one. Never written.

- **`.4ccs` squad files**, in order:
  - `"21a"` (3 ASCII bytes; the real files on this machine, written by an earlier build, carry
    `"20a"` with the same record — both tags are accepted, decision entry 2.17h);
  - 2 ASCII digits of the exporting PES version;
  - one **356-byte `player_export` record per rostered player**, in the order the source save's
    player records hold them, not roster order;
  - `[u16; 40]` shirt numbers in roster order (80 bytes);
  - optionally the 405-byte tactics block below (present when the file is 405 bytes longer than
    `5 + n × 356 + 80`; the editor writes it when tactics are enabled).

  The record is the MSVC layout of `player_export` (`editor.h`): little-endian, 4-byte
  alignment, `bool` one byte, `wchar_t[61]` UTF-16 name, `char[21]` shirt name, one padding
  byte at 271. The offsets are `scripts/provenance/fixtures/interchange_fixtures.py`'s ctypes
  table, and the golden test holds them as literals. Fields the exporting version lacks are
  ignored whatever the bytes (the writer fills `phys_cont` from `body_ctrl` on 15 and 16, and
  `clearing`/`reflex`/`cover` on 15 only). The playing style is the exporting version's index.
  The record has no id and no ingame-face run: the face and hair keys are absent,
  skin/iris/player-gloves go to the `appearance` keys.

  With no id, a record's player is only its position, so `read_squad` takes the target team and
  its save's players, as the reference's import does. Record *k* goes to the roster slot of the
  *k*-th rostered player in the target's record order, and number *i* to slot *i* when that slot
  received a record (the reference copies all 40 numbers, empty slots included; a `TeamToml`
  section needs a rostered player). The reference also counts the roster only up to its first
  empty slot; no save on the machine has a player after an empty slot (2.20k, 30714 rostered
  teams), so `read_squad` simply takes every rostered slot.

  We do not map record *k* to slot *k*, because the two orders differ on real teams (at 2.20k,
  in 22 of the machine's 27 distinct PES 16 saves, up to 27 cup teams each, and in 438 of 508
  teams of two PES 18 saves; never on 17 or 19-21), and there that puts every stat and
  appearance on the wrong player.

  A file with more records than the target has rostered players is `MoreRecordsThanRoster`
  (the reference stops reading and then takes its shirt numbers from the wrong offset). A
  shorter one fills the first rostered players in record order and leaves the rest untouched.

  Importable into any version through `TeamToml::apply`. The reference converts styles by
  version pair and clamps 19+ shirt numbers to 231 on older saves; `apply` instead refuses a
  number wider than the target's roster field before any write, like a too-long text, so a
  squad never applies and then fails to save.
- **`.4cct` "nightly" tactics files**: `"001"` + 2-char PES version + 8-char team id (ASCII,
  NUL-padded) + the **405-byte tactics block**:
  - per preset, 3 × (11 position bytes, then 11 y/x pairs), 7 style bytes (attacking style,
    buildup, attacking zone, positioning, defensive style, containment area, pressure), 2 + 2
    instructions each as canonical byte + player byte, support range, numbers in attack,
    defensive line, compactness, numbers in defence, fluid;
  - then starting XI (11), bench (21), 6 set-piece takers (FK long, FK short, FK 2, CK left, CK
    right, PK), 3 join-attack, 4 auto flags (substitution, offside trap, preset change,
    attack/defence levels) — **no captain**.

  The reference imports only into 16-18 and only from the same version (its x/y "conversion
  factor" is 1 for every pair). Here the block parses into `TeamToml.tactics`, `bench_order`
  taking the first 12 of its 21 and `apply` refilling the nine as a Team TOML import does, and
  `apply`'s version rule decides, the header's version being `pes_version`.

  No real `.4cct` exists on this machine. The fixture is transcribed from the reference's write
  walk over the PES 19 fixture save's team 713 (provenance in the fixtures README), and the
  test's evidence is that the Rust reader's `TeamTactics` equals the schema codec's for that
  team.

### Texport (read + write)

Texport files are **PES's own in-game team export/import format**. Measured on real files
(one PES 15, 12 PES 16, 34 PES 17, four 18, eleven 19, one 20, five 21): the file is a
concatenation of the **save's own records, in the save's own layouts**, so the schema codec
reads and writes every record and `interchange/texport.rs` adds only the crypto and the layout
table (`schema/texport.rs`). Two eras:

- **15-17** (`TEXPORT00000000`, ~5.6 MB): the version's savefile container (`SaveContainer`
  decrypts it unchanged; the description reads `Team Export Data NN` + the team name where a
  save's reads `Edit Data`, the check `NotATexport` applies).

  The body lays its records out as an 18-21 one does, each the save schema's size:
  - on 16, the team record (456 bytes) at `0x1002C`, then the roster record (164) at
    `0x101F4`, then the tactics record at `0x10298`;
  - on 17, the team record (480) at `0x10054`, the 88-byte coach block at `0x10234`, the roster
    record at `0x1028C`, the tactics record at `0x10330`.

  So the roster sits at the tactics offset minus the roster record's size and the team record
  ends where the roster starts (17: where the coach block starts). In all 46 files the three
  records' ids agree (`IdsDisagree` when they do not) and the roster holds the 23 player ids and
  their shirt numbers.

  Player records are at `0x51065C` / `0x510660` / `0x510840`, each filled roster slot's record
  at that slot's position (`players_at` + slot × the record stride, as on 18-21), its id checked
  against the slot's (`RosterMismatch`, as for 18-21). On 15/16 a player record then its
  appearance record per player. That order is the roster's, not always the ids': one real
  PES 16 file (team 804) lists 80401, 80402, 80423, 80404, ..., which a consecutive-id scan
  reads as two players and loses the other 21. No real roster has an empty slot before a filled
  one (55 texports, 2026-10-15), so slot and packed positions agree on every file measured; a
  gapped roster laid out packed would be refused by the id check, never misread.

  PES 15 lays out as 16 does (the same tactics offset and record sizes): its one real file (team
  748 `/sci/`, read 2026-10-31) opens, and written back unedited reproduces its payload and
  description byte for byte.

  The records are read rather than keeping the target save's team and roster because the file
  carries them, and the consecutive-id rule they replaced was ours, never measured: it is what
  lost the 21 players.
- **18-21** (`.ted`, 8128 / 9648 / 14760 / 14820 bytes for 18 / 19 / 20 / 21): plaintext header
  `0x00..0x30`, a 32-byte key at `0x30`, then the body XOR'd with the key starting at key index
  `0x12` / `0x13` / `0x14` / `0x15` (18 / 19 / 20 / 21) and wrapping at 32. PES 20's size is
  not the `0x39E4` the reference editor assumes for 20 and 21: its team record is 60 bytes
  shorter than 21's.

  The body is `team record | 88-byte coach block |
  roster record | tactics record | 660-byte block | player records × roster width | 12-byte tail`,
  every size the save schema's (18: 480/164/628/188 × 32; 19: 416/244/628/188 × 40; 20:
  528/284/628/312 × 40; 21: 588/284/628/312 × 40), so tactics land at `0x32C` / `0x33C` /
  `0x3D4` / `0x410` and players at `0x834` / `0x844` / `0x8DC` / `0x918`.

  The header is constant per version (its u32 at `0x08` is the body length) except the u32 at
  `0x0C` (two values seen on 18, one each on 19 and 21, independent of content) and the u32s at
  `0x10`/`0x14` (0 or 4). No content checksum was found (byte sums, CRC-32 and Adler-32 of
  plaintext and ciphertext, with and without the key, match nothing).

  The coach block is the save's coach record for the team (below). The 660-byte block is zero
  or one to five 125-188-byte runs (an edited coach, presumably; the writer writes it zero), and
  the tail is a per-version constant.

**A written file's records are the save's.** The game writes a Texport by copying the save's
records whole: every record of two real PES 2017 Texports is in the save they came from, the
coach block byte for byte. So `Texport::new` (17-21) copies the open save's own record bytes
for the team, roster, tactics and each rostered player, and the coach record, and writes the
codec's fields over the copies; a byte no field models keeps the save's value.

Encoding onto zeros instead loses fields the game reads. A PES 2017 file written that way
imports and lists its players in Edit, but the kick-off screen's "Starting Lineup" shows nobody
(a tester's report, 2026-11-11), or players with blank names and stats.

The field the game needs is the manager id. Of four files built from one game-made Texport of
772, the zero-encoded one and that one with every player's id copy restored showed blank names
and stats, while that one with only the team record's +4 restored worked like the game's own
file, and so did a file from the writer that copies the save's records (PES 2017, in game,
2026-11-11). The team record's u32 at +4 is the team's **manager id** (on PES 17 equal to the
team id on 218 of 220 teams; on 19 the game's own coach ids), a field of the codec from PES 17
on. Among the bytes no field models: tactics values, and each player record's second copy of the
player id and the `ff ff` after it.

The coach block is the save's **coach record** that manager id names: a section of 88-byte
records sorted by coach id (the u32 at +0) and closed by an all-`ff` record, its count a u16 in
the payload header after the team count (`VersionSchema::coaches`). Measured on the reference
saves 2026-11-11:

- `0x410118`, count at `0x62` (17);
- `0x40a35c`, `0x66` (18);
- `0x5fecbc`, `0x66` (19, not in team order);
- `0x94dddc`, `0x66` (20);
- `0x958da4`, `0x66` (21, 231 coaches for 210 teams);
- `None` on 15 and 16, not measured.

A manager id naming no coach record is `NoCoach`, not a template: the game's file always
carries one. The measured templates remain for what is not a record (the header, the tail, the
zero 660-byte block).

```rust
/// `file.rs`: a team's own record bytes in the open save, as loaded (slices of the retained
/// payload: an unsaved edit is not in them, the writer writes the entry over them).
pub struct TeamRecords<'a> {
    pub team: &'a [u8],
    pub coach: Option<&'a [u8]>,   // the coach record its manager id names; None: no section or no record
    pub roster: &'a [u8],
    pub tactics: &'a [u8],
}
impl EditFile {
    pub fn team_records(&self, id: u32) -> Option<TeamRecords<'_>>;   // None: no team `id`
    pub fn player_record(&self, id: u32) -> Option<&[u8]>;          // None: no player `id`
}
```

```rust
/// `schema/texport.rs`: the 18-21 layout, offsets derived from the record sizes.
pub struct TexportLayout {
    pub key_index: usize,       // first key byte used
    pub size: usize,            // whole file
    pub header: [u8; 0x30],     // the fixture's header, the writer's template
    pub tail: [u8; 12],
}
pub fn texport_layout(version: PesVersion) -> Option<&'static TexportLayout>;   // None on 15-17

/// `interchange/texport.rs`.
pub struct Texport { /* version, the plaintext (18-21) or the SaveContainer (15-17), team, players */ }
pub enum TexportError { UnknownVersion, WrongSize { version, expected, actual }, Container(ContainerError), NotATexport, Codec(CodecError), RosterMismatch { slot, roster: u32, player: u32 }, NoTemplate(PesVersion), IdsDisagree { team, roster, tactics }, NoCoach { team: u32, manager: u32 }, RecordSize { record: &'static str, expected: usize, actual: usize } }
// `NotATexport`: a 15-17 savefile container that is a save, not a texport — the description is
// the file's own label, checked before any record is read, so a save handed to `from_bytes` is
// refused rather than opening with a garbage team and no players. `IdsDisagree`: the team,
// roster and tactics records' ids disagree — the invariant every measured 15-17 texport holds.

impl Texport {
    /// `version = None` detects: by size for 18-21 (every size is distinct once 20's is derived;
    /// the candidate whose team, roster and tactics record ids agree wins, `UnknownVersion` when
    /// none or several do), then the container's version for 15-17. `Some(version)` on 15-17
    /// defers to the container's own version (no variant expresses the mismatch).
    pub fn from_bytes(bytes: &[u8], version: Option<PesVersion>) -> Result<Texport, TexportError>;
    /// A fresh file written onto the save's own records (`EditFile::team_records`, and each
    /// player paired with his `EditFile::player_record`, in roster order): the copies take the
    /// entries' fields, ids checked against the roster and the record (`RosterMismatch`); the
    /// coach block is `records.coach` as it is (`NoCoach` when `None`); a record of the wrong
    /// size is `RecordSize`. Around the records, 18-21 the layout's header, a zero 660-byte
    /// block, the tail and a key from `salt[..32]`; 17 a PES 2017 container around `PES17`'s
    /// payload template, carrying `serial`, the writer's serial (the open save's,
    /// `EditFile::serial`; unused on 18-21). 15 and 16 are `NoTemplate`: write by reading a real
    /// file and replacing its team and players (`team_mut`/`players_mut`).
    pub fn new(version: PesVersion, team: &TeamEntry, records: &TeamRecords<'_>, players: &[(&PlayerEntry, &[u8])], salt: &[u8; 320], serial: &[u8]) -> Result<Texport, TexportError>;
    pub fn version(&self) -> PesVersion;
    pub fn team(&self) -> &TeamEntry;            // team + roster + tactics, the file's records on
                                                 //   every version
    pub fn players(&self) -> &[PlayerEntry];     // roster order, empty slots skipped
    pub fn team_mut(&mut self) -> &mut TeamEntry;
    pub fn players_mut(&mut self) -> &mut [PlayerEntry];
    /// The records re-encoded into the plaintext (unmodeled bytes as read), then encrypted:
    /// XOR with the file's own key on 18-21, the container with `salt` on 15-17, whose
    /// description takes the team's name at 0x80 (the name the game lists), cleared past it.
    pub fn to_bytes(&self, salt: &[u8; 320]) -> Result<Vec<u8>, TexportError>;
    /// The import document: `TeamToml::from_team` over the texport; on every version the
    /// `[team]` section and the roster's shirt numbers are the file's.
    pub fn to_team_toml(&self) -> Result<TeamToml, TeamTomlError>;
}
```

Import never trusts the file's player IDs: players map onto the target team's roster by slot
order, which is `texport.to_team_toml()` then `TeamToml::apply`. That is one import path for
every format, conversion included (the reference zeroes 15/16 skills 28-41 and sets tight
possession/aggression/physical contact to 77: the target's values stand in for those instead,
per the 2.17f rule).

Because the game itself consumes these, full compatibility matters in both directions: read
(the reference already does) **and write** (new — the reference is import-only), so the editor
can produce a file PES can import. Every player record's appearance id holds the player's own id
in each export the game wrote (the 18-21 fixtures), so the writer writes it so, whatever the
save's entry holds.

The round trip `from_bytes` → `to_bytes` is byte-identical on every fixture (the codec and the
carried bytes cover the whole file). Whether the game accepts a `new` file, and a file whose
records were edited, is the manual per-version check in [Verification](verification.md).

---
