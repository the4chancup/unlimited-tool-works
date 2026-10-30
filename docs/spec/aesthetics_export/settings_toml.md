# Unlimited Tool Works — Aesthetics export plan: settings.toml

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Player settings in exports (settings.toml)

Aesthetic settings that used to live only in the savefile move into the export as TOML, and the
compile carries them to the game:

- **Fox (PES 18–21): in the database tables the output CPK carries.** The game takes a player's
  appearance from `PlayerAppearance.bin`, his boots from `BootsList.bin` and his gloves from
  `GloveList.bin` whenever his savefile appearance record's player id is -1 (`0xFFFFFFFF`), player
  by player; a record whose id is set wins over the tables (in-game Test 1, PES 2021: decision
  entry "aesthetics travel in the database tables"). The official save is therefore stripped once
  (every appearance id set to -1)
  and the compiler writes each compiled player's three rows from his two settings files; nothing
  of a player's appearance is written to the savefile. Gloves need a FoxDen change to keep the table's value (worklog
  "Issues"); until it does, a Fox gloves row has no effect in the 4cc setup.

**Everything in the files is optional** — each file itself included: a player folder with only
models is valid. **An absent key takes its default**, the value the templates below show for it,
on both engines; model-derived ID assignments and FPC
directives are separate inputs. `name` and `shirt_name` are the exceptions: absent means "not
written" (see the name rules). A compiled player's appearance is therefore his export (these
files, his folder's models and markers) and nothing else: no earlier compile, savefile or
installed table contributes to it, except a boots or gloves ID whose output failed (step 1
below) and,
in a `Midcup` export, a category the folder leaves to its default (step 3). A compiled player is
every player a kept player folder of a team export maps to, whatever his tasks' outcomes. A referee export's
`settings.toml` reaches no table or patch yet (worklog issue). A `settings.toml`'s and an
`ingame_face.toml`'s findings, a referee export's included, are reported by `check` and `compile`
alike.

**The files can express everything the savefile's player appearance record holds.** Their joined
schema is `pes_savefile`'s `PlayerSettings`: all of the decoded player appearance record —
physique, strip style, wrist-tape and spectacle colours, skin and iris colour, the motion block,
every setting of the ingame-face run (face sliders and feature types, eyebrow and facial-hair
colours, in-game hair), and stock boots/gloves IDs (below), except the compiler-owned edit flags
and base-copy ID.
About one cup player in twenty relies on in-game hair rather than a face model. Every bit of the
run is a key or measured unused ("The ingame-face run, decoded" in the
[Savefile plan](../pes_savefile/model.md), whose spec part holds the completeness test).
Completeness
matters here because these files are the **only** route by which a team's aesthetics reach the
game: the save editor's own appearance fields are read-only by default (see "Read-only
aesthetics" in the [Save editor plan](../../plans/save_editor.md)), and on Fox a stripped player's
appearance bytes are not read at all. The **templates** — `settings.toml`'s, which the Export
upgrader and the save editor generate, and `ingame_face.toml`'s, which the upgrader writes — are
what a new player folder starts from, and therefore list every key, one per line, each followed
on the same line by the comment that gives
its range: set ones with their value, unset ones as commented lines (the boots/gloves IDs as
`""`, see below), so a team can see what is
authorable without any other document. The blocks below are the complete key tables; `to_toml`
emits them in this order and with these comments.

**Boots/gloves IDs are authored only for stock models.** Custom boots and gloves are folder
content: local models and shared link files determine the compiler's player-exclusive or shared
assignments (see "Player folders"), and users never copy a custom model's numeric ID into this
file. A player who wears one of the game's own models has no folder to express that, so the two
top-level keys `boots_id` and `gloves_id` name a model of the **stock band**, IDs 0 to 100: the
cup's stock kit compacts Konami's boots and gloves into that range, and the compiler's per-team
blocks start at 101, so an ID above 100 is always some team's custom content and is rejected at
parse (`OutOfRange` naming the key). Besides an integer, both keys accept the empty string, which
means **default** and is the same as an absent key. Each category resolves in this order:

1. The folder's own models or a link file provide the category → the compiler-assigned ID; when
   that output fails, the player's installed ID stays (Fox: his row of the installed table;
   pre-Fox: the savefile's current ID), so no ID points at content the CPK lacks. A numeric key
   is ignored with `settings_model_id_conflict` (W), reported on Fox team exports until the
   pre-Fox patch and a referee's table rows exist, the only places the key reaches the game.
2. A numeric key → that stock ID, whatever FPC marker the folder carries.
3. Default (`""` or absent) → the folder's FPC marker decides: `fpc_on` writes the hide preset's
   nonexistent IDs (boots 55, gloves 11); `fpc_off` or no marker writes 0 for both (the default
   boots and a pair of normal hands). In a `Midcup` export with no marker the player keeps his
   installed row (Fox) or savefile ID (pre-Fox): only a full export resets what a team had
   ("Coverage tag" in `object_model.md`).

Gloves are not a goalkeeper-only model here: the cup's gloves system lets any player wear a gloves
model, which is why gloves ID 0 is a set of normal hands rather than "no gloves".

Default is what nearly every player wants: an FPC player (a face model carrying the full body,
no boots/gloves folders) needs the blank IDs, which the marker supplies without a key. Pre-Fox
local models embedded in face XML request no standalone output, so they fall through to steps 2
and 3. The template writes both keys uncommented
as `""`, the only keys shown with a value that sets nothing: every other key has a neutral value
to display, while any boots/gloves ID is a real model, and TOML has no bare `key =`. They sit at
the file's top level, next to `name`, rather than in
`[appearance]`: Team TOML embeds the `[appearance]` tables and has its own player-level
`boots_id`/`gloves_id` over the full stored range (save interchange), which two keys of the same
name inside `[appearance]` would collide with.

```toml
# settings.toml, inside a player folder. Every key is optional: an absent key takes the
# value shown here (name and shirt_name: not written). A commented key shows what can be
# set and its range. The file never references the export's models: what a player wears
# is decided by the folder contents (models present, link files pointing at shared
# folders); boots_id/gloves_id only name the game's stock models.
# The in-game face's keys (iris, face, hair) are in ingame_face.toml.

# true = derive from the folder name ("15 - Snuffy" gives "Snuffy"; the whole folder
# name for players.txt-mapped folders); "text" = write as is; absent = not written.
# name and shirt_name are used only when the export's root holds the `write_names` marker;
# otherwise they are ignored and the names in the save stand.
name = "Snuffy"
shirt_name = "SNUFFY"           # "text" = write as is; absent = not written
boots_id = ""                   # "" = default (fpc_on: hidden, fpc_off 0, no marker 0 or in a Midcup export the installed ID); 0 to 100, a stock boots model; ignored when the folder's boots models or a boots link get an ID (pre-Fox face-XML boots do not)
gloves_id = ""                  # "" = default (fpc_on: hidden, fpc_off 0, no marker 0 or in a Midcup export the installed ID); 0 to 100, a stock gloves model (0 = normal hands); ignored when the folder's gloves models or a gloves link get an ID (pre-Fox face-XML gloves do not)

[appearance]
skin_color = 1                  # 0 white, 1 light, 2 fair, 3 medium, 4 olive, 5 brown, 6 black, 7 custom (invisible body, PES 15 to 17 only)

[appearance.physique]
neck_length = 0                 # -7 to 7
neck_size = 0                   # -7 to 7
shoulder_height = 0             # -7 to 7
shoulder_width = 0              # -7 to 7
chest = 0                       # -7 to 7
waist = 0                       # -7 to 7
arm_size = 0                    # -7 to 7
arm_length = 0                  # -7 to 7
thigh = 0                       # -7 to 7
calf = 0                        # -7 to 7
leg_length = 0                  # -7 to 7
head_length = 0                 # -7 to 7
head_width = 0                  # -7 to 7
head_depth = 0                  # -7 to 7

[appearance.strip]
sleeves = "short"               # "seasonal", "short", "long"
inners = "off"                  # "off", "normal", "turtleneck"
socks = "standard"              # "standard", "long", "short"
undershorts = "off"             # "off", "short", "winter_long" (none in summer, long in winter), "short_winter_long" (short in summer, long in winter)
untucked = true                 # true, false
ankle_taping = false            # true, false
wrist_taping = "off"            # "off", "right", "left", "both"
wrist_tape_color_left = 0       # 0 to 7
wrist_tape_color_right = 0      # 0 to 7
spectacles = 0                  # 0 none, 1 rectangle rimless, 2 rectangle half frame, 3 rectangle full frame, 4 oval rimless, 5 oval half frame, 6 oval full frame, 7 round full frame
spectacles_color = 0            # 0 white, 1 black, 2 red, 3 blue, 4 yellow, 5 green, 6 pink, 7 turquoise
gloves = false                  # true, false (outfield player gloves)
gloves_color = 0                # 0 to 7

[appearance.motion]             # on Fox written by the aesthetics patch: the tables do not hold motions
hunching_dribbling = 1          # 1 to 3 (PES 20 and 21: 1 to 5)
hunching_running = 1            # 1 to 3 (PES 20 and 21: 1 to 5)
arm_movement_dribbling = 1      # 1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)
arm_movement_running = 1        # 1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)
corner_kick = 1                 # 1 to 6 (PES 20 and 21: 1 to 10)
free_kick = 1                   # 1 to 16 (PES 20 and 21: 1 to 20)
penalty_kick = 1                # 1 to 4 (PES 20 and 21: 1 to 7)
dribbling = 0                   # 0 to 3, PES 20 and 21 only
goal_celebration_1 = 0          # 0 none, then PES 15: 1 to 112, 16 and 17: 1 to 122, 18: 1 to 127, 19: 1 to 140, 20: 1 to 152, 21: 1 to 162
goal_celebration_2 = 0          # 0 none, then PES 15: 1 to 112, 16 and 17: 1 to 122, 18: 1 to 127, 19: 1 to 140, 20: 1 to 152, 21: 1 to 162
```

**`ingame_face.toml`.** The marker's file: a folder holding it wears the in-game face, which the
file sets (`player_folders.md` "`ingame_face` marker"). The template's `[appearance.face]` table,
`[appearance.hair]` table and `iris_color`, 90 of its keys, moved into it: in the file they are
the top-level `iris_color` and the tables `[face]` and `[hair]`, with the same keys, ranges,
defaults, per-version rules and comments. `skin_color`, the physique (head size included), the
strip, the motions and the boots/gloves IDs stay in `settings.toml`, since a player with a face
model wears them too. The keys moved because the game draws them on the in-game face alone (the
marker paragraph holds the measurement): a player wearing a face model sets them for nothing.

**The document.** `pes_savefile`'s `settings_toml` module reads and writes it beside
`settings.toml`, from the same key table: its keys sit at their `[appearance]` paths with the
root dropped (`iris_color`, `face.cheek_type`, `hair.length`), in the same ranges as
`settings.toml` (the widest per key). Team TOML keeps all of them under
`[players.NN.appearance]`, unchanged. The player's settings stay one value, joined from the two
files, so a row or the patch is built from one `PlayerSettings`:

```rust
/// A player's `ingame_face.toml`: the in-game face's keys, each `None` when absent.
pub struct IngameFaceSettings {
    pub iris_color: Option<u8>,
    pub face: FaceSettings,
    pub hair: HairSettings,
}
impl IngameFaceSettings {
    pub fn parse(text: &str) -> Result<Self, SettingsError>;
    /// The template: the header below, then the `iris_color` line, `[face]` and `[hair]`.
    pub fn to_toml(&self) -> Result<String, SettingsError>;
}
impl PlayerSettings {
    /// The in-game face's keys, as `ingame_face.toml` holds them.
    pub fn ingame_face(&self) -> IngameFaceSettings;
    /// The in-game face's keys set from `face`, an absent key unset.
    pub fn set_ingame_face(&mut self, face: IngameFaceSettings);
}
```

Its template's key lines are `settings.toml`'s (the `iris_color` line, then the
`[appearance.face]` and `[appearance.hair]` blocks under the headers `[face]` and `[hair]`),
verbatim but for those headers:

```toml
# ingame_face.toml, inside a player folder: the in-game face, which the game draws when
# the folder holds no face model and no face link. The file marks the folder as wearing
# it. Every key is optional: an absent key takes the value shown here, and an empty file
# is the game's default face. A commented key shows what can be set and its range.

iris_color = 0                  # 0 black, 1 dark brown, 2 brown, 3 sable, 4 navy blue, 5 charcoal, 6 gray, 7 blue, 8 sienna, 9 green, 10 violet

[face]
jawline = 0                     # -7 to 7
jaw_height = 0                  # -7 to 7
cheekbones = 0                  # -7 to 7
cheekbone_height = 0            # -7 to 7 (PES 16 to 21: no editor slider)
chin_width = 0                  # -7 to 7
chin_height = 0                 # -7 to 7
chin_depth = 0                  # -7 to 7
face_size = 0                   # -7 to 7
face_height = 0                 # -7 to 7
brow_width = 0                  # -7 to 7
inner_eyebrow_height = 0        # -7 to 7
eyebrow_depth = 0               # -7 to 7
temple_width = 0                # -7 to 7
outer_eyebrow_height = 0        # -7 to 7
pupil_size = 0                  # -7 to 7
eye_depth = 0                   # -7 to 7
inner_eye_position = 0          # -7 to 7
inner_eye_height = 0            # -7 to 7
outer_eye_position = 0          # -7 to 7
eye_corner_height = 0           # -7 to 7
upper_eyelid_width_inner = 0    # -7 to 7
upper_eyelid_height_inner = 0   # -7 to 7
upper_eyelid_width_outer = 0    # -7 to 7
upper_eyelid_height_outer = 0   # -7 to 7
bottom_eyelid_height = 0        # -7 to 7
horizontal_eye_position = 0     # -7 to 7
eye_height = 0                  # -7 to 7
nose_height = 0                 # -7 to 7
nose_tip_depth = 0              # -7 to 7
nostril_width = 0               # -7 to 7
nose_depth = 0                  # -7 to 7
nose_width = 0                  # -7 to 7
mouth_height = 0                # -7 to 7
mouth_depth = 0                 # -7 to 7
lip_size = 0                    # -7 to 7
mouth_corner_height = 0         # -7 to 7
lip_width = 0                   # -7 to 7
ear_length = 0                  # -7 to 7
ear_width = 0                   # -7 to 7
ear_angle = 0                   # -7 to 7
pes15_jaw_depth = 0             # -7 to 7, drawn by PES 15 only
pes15_cheekbone_depth = 0       # -7 to 7, drawn by PES 15 only
pes15_mouth_edge_position = 0   # -7 to 7, drawn by PES 15 only
pes15_eyebrow_depth = 0         # -7 to 7, drawn by PES 15 only
pes15_bottom_eyelid_position = 0 # -7 to 7, drawn by PES 15 only
pes15_nasal_bridge_position = 0 # -7 to 7, drawn by PES 15 only
pes15_nasal_root_height = 0     # -7 to 7, drawn by PES 15 only
cheek_type = 0                  # 0 to 3 (PES 15: 0 to 14; PES 19: 0 to 5)
forehead_type = 0               # 0 to 5 (PES 15 and 19: 0 to 6)
facial_hair_type = 0            # 0 to 12 (PES 15: 0 to 29; PES 20 and 21: 0 to 19)
laughter_lines_type = 0         # 0 to 4 (PES 15: 0 to 6)
upper_eyelid_type = 0           # 0 to 6 (PES 15: 0 to 14; PES 20 and 21: 0 to 7)
lower_eyelid_type = 0           # 0 to 2 (PES 15: 0 to 14; PES 20 and 21: 0 to 6)
eyebrow_type = 0                # 0 to 5 (PES 15: 0 to 17; PES 20 and 21: 0 to 7)
neck_line_type = 0              # 0 to 2 (PES 15: 0 to 5; PES 20 and 21: 0 to 3)
nose_type = 0                   # 0 to 6 (PES 15: 0 to 11; PES 20 and 21: 0 to 7)
pes15_nose_angle = 0            # -1 to 1; PES 15 only
upper_lip_type = 0              # 0 to 3 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)
lower_lip_type = 0              # 0 to 2 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)
eyebrow_color_red = 14          # 0 to 63
eyebrow_color_green = 10        # 0 to 63
eyebrow_color_blue = 7          # 0 to 63
eyebrow_style = 0               # 0 normal, 1 thick, 2 thin
eyebrow_density = 3             # 0 to 3
eyebrow_thickness = 1           # 0 to 3
facial_hair_color_red = 14      # 0 to 63
facial_hair_color_green = 10    # 0 to 63
facial_hair_color_blue = 7      # 0 to 63
facial_hair_thickness = 3       # 0 to 3
pes15_eye_white = 0             # 0 to 3, brighter eye whites, drawn by PES 15 only

[hair]
color_red = 14                  # 0 to 63
color_green = 10                # 0 to 63
color_blue = 7                  # 0 to 63
color_preset = 0                # 0 custom (the colour above), 1 to 8 the editor's presets
accessory = false               # true, false (PES 15: the game sets it with the variation)
accessory_color = 0             # 0 white, 1 dark grey, 2 red, 3 blue, 4 yellow, 5 green, 6 magenta, 7 cyan
style = 1                       # 1 normal, 2 dry, 3 mohican, 4 afro, 5 dreadlocks, 6 braided, 7 special, 0 in some saves; PES 16 to 21
cropped_style = 1               # 1 to 6, 0 in some saves; PES 16 to 21
length = 2                      # 1 shaven, 2 very short, 3 short, 4 medium, 5 long, 0 in some saves; PES 16 to 21
wave_level = 0                  # 0 to 6, -1 in some saves; PES 16 to 21
variation = 0                   # 0 to 29; PES 16 to 21
front_style = 1                 # 1 up, 2 down, 3 combed back, 0 in some saves; PES 16 to 21
parted = 1                      # 1 off, 2 left 2, 3 left 1, 4 centre, 5 right 1, 6 right 2, 0 in some saves; PES 16 to 21
side_back_style = 4             # 1 normal, 2 reduce body, 3 shorter sides, 4 cropped, 0 in some saves; PES 16 to 21
hairline = 1                    # 1 to 3, 0 in some saves; PES 16 to 21
forehead_width = 2              # 1 narrow, 2 normal, 3 wide, 0 in some saves; PES 16 to 21
pes15_length = 2                # 1 skinhead, 2 very short, 3 short, 4 medium, 5 long, 6 special; PES 15 only
pes15_type = 1                  # 0 none (skinhead, special), 1 straight, 2 wavy, 3 permed, 4 dreadlocks; PES 15 only
pes15_variation = 1             # 1 to 82, the editor's grid position (each length and type has its own grid); PES 15 only
```

The ingame-face run follows three rules:

- **One file shape on every version.** PES 15 stores its hair differently from PES 16 to 21
  (`spec/pes_savefile/model.md` "The ingame-face run"). So the keys marked "PES 16 to 21"
  are skipped on a PES 15 compile, and the `pes15_` hair keys and `pes15_nose_angle` are
  skipped on a 16 to 21 compile. Either way there is no finding, as for `dribbling`. A PES 16
  export compiled for PES 21 keeps its in-game hair; one compiled for PES 15 gets PES 15's hair
  keys, at their defaults if the file does not set them. The other `pes15_` keys exist on every
  version and only PES 15 draws them. A save of 16 to 21 holds them too, its editor keeps them,
  and the games' tables fill them for every face. So a generated file carries them and a
  compile writes them on every version.
- **The defaults are the game's default face.** The defaults are what PES 17's facial reset
  writes:
  - very short Normal hair;
  - dark brown hair, eyebrows and facial hair (14, 10, 7);
  - the template's values for the rest.

  The template keeps the sliders at 0 and the feature types at 0, as before. The reset itself
  also writes jaw height +2 and both lips 1, but a slider at 0 reads as an offset from the
  editor's centre. An all-zero run would be the editor's Preset look instead, every slider at
  -7, which is why an absent key is not stored as 0.
- **Unused bits are not keys.** The run's measured-unused bits are written at the values the
  games' editors write, on every compile, so a compiled run holds nothing of the save. They
  are dropped on generation.

The hair lists' ranges also hold the 0 that real saves store, outside the editors' 1-based
lists: in the fixture cup saves, PES 16, 17 and 18 players hold it in `style`, `front_style`
and `parted`, some in every list. Of the 1,651 PES 18 cup players, 1,609 hold 0 in those three
lists under `length` 1 (shaven), and 42 (the /asp/, /w/ and /wsg/ players) hold 0 in every
list, with `wave_level` -1. A generated file must carry a save's player, so each list takes 0,
as `variation` does, and `wave_level` (stored as the level plus 1) takes -1. The template's
defaults stay the editor's.

What a 0 draws was measured on PES 18, from Edit-mode renders of patched saves. A 0 in a list
the hairstyle uses (`length` and `hairline` were measured) makes the game draw a fallback hair
that depends on the face, not a look of 0's own: short black spiky hair on one face, short
brown hair on another, and the all-zero players draw that same fallback. A 0 in a list the
length greys out (`style`, `front_style`, `parted`, `variation` and `side_back_style` under
shaven) changes nothing, so the 1,609 draw their shaven look. The editor shows a 0 as its
first entry (`length` "Shaven", `hairline` "Type 1", `forehead_width` "Narrow",
`cropped_style` "Type 1", `wave_level` "+0"); changing the length writes 1 into every list at
0, and confirming a picker on the entry it shows writes 1 into a list at 0 and nothing into a
list already holding its value. Not measured: `wave_level`, `forehead_width`,
`cropped_style` or `side_back_style` at 0 with the other lists set, and any length but
shaven. So the keys keep accepting 0, and a file that carries one compiles to what the save
drew: the game's fallback.

Where the ranges come from, so a wrong one can be traced: the boots/gloves range is the stock
band below the compiler's first team block (`spec/aesthetics_export/player_folders.md` "Assigns IDs automatically"); the colour, spectacle, sleeve, inner,
sock, undershort, shirttail and wrist-taping lists are the reference save editor's combo boxes
(stored value = list position; wrist taping 1 = right, 2 = left, 3 = both, matching the two taping
bits); the physique numbers are the editor's spin ranges (stored value = number + 7); the motion
ranges are its spin ranges per version (1-based ones are stored minus 1, celebrations as is); the
face-type maxima are the caps the two legacy converters apply when writing a PES 16 and a PES 21
save. The 17 and 18 face caps are unmeasured; PES 19's editor offers cheek types 0 to 5 and
forehead types 0 to 6, wider than 16's, its other caps unmeasured. An unmeasured face cap is
the field's bits, for the parser's widest range and the compiler's narrowing alike, the
comments keeping the measured ranges (`spec/pes_savefile/operations.md` "Cross-version
player conversion" holds the table). PES 15's face caps
were measured: the editor's range joined with the game's own faces. The run's other keys were
measured in the games (`spec/pes_savefile/model.md` "The ingame-face run"). The in-game hair
ranges were measured on PES 21 and are assumed for 16 to 20. The parser checks a value against the **widest** range of a key (a PES
16 file may legitimately be compiled for PES 21); the per-version narrowing is the compiler's
finding at compile time, when the target version is known (`settings_value_out_of_range`). The
value is then written as the key's default (a face type 0, a motion the template's value), as
a cross-version player conversion resets a face type (`spec/pes_savefile/operations.md`
"Cross-version player conversion"), and a custom skin (7) the target has none of as 1. Not as
authored: a motion beyond the target's range may not fit its field's bits (PES 18's two for
`hunching_dribbling`), and one that fits is shown as the list's top and may be rewritten to the
list's first entry by the player's next in-game save (`spec/pes_savefile/operations.md`
"Cross-version player conversion"). A key the target has
no field for (`dribbling` before PES 20) is not written, with no finding: an export written for a
newer version compiles for an older one as it is. Strings are matched exactly (lower case, as
listed). Height and weight are not keys: they are the player record's own fields, not the
appearance block's, and height affects gameplay, so both belong to the manager's tactical side
(the save editor), and weight leaves with height for consistency.

The motion ranges were measured in every version's editor (2026-10-13, every row stepped past
its top): each 1-based key starts at 1 and wraps, stored one below what the editor shows, and
only the goal celebrations have an OFF entry (stored 0) below 1, stored as shown. The reference
editor's ranges, which the template first copied, were wrong on PES 15's arm movements and on
the celebrations of PES 15, 18, 19 and 20. The tops, as the editor shows them:

| Key | PES 15 | PES 16 | PES 17 | PES 18 | PES 19 | PES 20 | PES 21 |
|---|---|---|---|---|---|---|---|
| `hunching_dribbling`, `hunching_running` | 3 | 3 | 3 | 3 | 3 | 5 | 5 |
| `arm_movement_dribbling`, `arm_movement_running` | 5 | 8 | 8 | 8 | 8 | 10 | 10 |
| `corner_kick` | 6 | 6 | 6 | 6 | 6 | 10 | 10 |
| `free_kick` | 16 | 16 | 16 | 16 | 16 | 20 | 20 |
| `penalty_kick` | 4 | 4 | 4 | 4 | 4 | 7 | 7 |
| `goal_celebration_1`, `goal_celebration_2` | 112 | 122 | 122 | 127 | 140 | 152 | 162 |

The parser accepts a key's widest range, the table's top over every version; the per-version
narrowing at compile time and the cross-version conversion's cap read the version's column. A
stored value past a version's top is shown as the top: PES 16's editor shows "122" for a cup
player who stores 183 in `goal_celebration_2`, and keeps the 183 when another row of the motion
screen is edited and the team saved (what a match draws for 183 is unmeasured). Reading such a
player into a `settings.toml` refuses the value, as any value past the widest range.

The four `undershorts` labels name the summer/winter pair the game stores (the reference editor
shows them as "S: Off / W: Off", "S: Short / W: Short", "S: Off / W: Long", "S: Short / W: Long").

Name-writing rules, in full — **not writing is the default**; writing is opt-in:

1. `name` absent → no name is written, ever. The savefile name stays whatever the save editor set.
   This is also what makes multi-mapped `players.txt` folders work as **generic model folders**:
   several players can share one folder's models without being renamed.
2. `name = true` → the name is derived from the folder (the name part of a numbered folder name, or
   the whole folder name under `players.txt`) and written.
3. `name = "..."` → the string is written as-is. This is also the only way to write a name carrying
   the savefile's **name colour codes** (`\x11c` + 8 hex digits, see the pes_savefile plan's "Name
   colour codes") — control characters cannot appear in a folder name, so a derived name (`true`)
   is always plain. Cup organizers colour names freely (not only medal players'), so tools that
   generate `settings.toml` from a savefile (the Export upgrader, the save editor) must emit the
   explicit string whenever the savefile name carries codes, or the next compile would strip them.
4. In a `write_names` export, with `name = true` or a string in a folder mapped to multiple players
   via `players.txt`, the same name is applied to all of them, with a warning
   (`settings_toml_name_shared`).

`shirt_name` is a string or absent: absent writes nothing, a string is written as is (checked
against the version's shirt-name field and character set, as a `name` string is, a misfit
being `settings_name_invalid` and not written; it may fill the field, as real saves'
shirt names do: only `shirt_name_from` keeps a byte free, for text it invents). There is
no `true`: a shirt name rarely repeats the player name, and a derived default would be one more
thing to read. In a `write_names` export a shared folder applies it to every mapped
player, like `name`.

On Fox the motions and the names are the parts of this file that reach the savefile, and only
through the **aesthetics patch**. As for names, an export carrying the `write_names` marker at
its root has its names written by the patch. The motivating case is an autopilot team (an
aesthetics export by a caretaker, a tactical export by the council, often written before the
destination team was known, with names like "Gold striker"), whose names must be written over
the tactical export's; an autopilot team whose tactical export already holds the right names
leaves the marker out. The marker is named for what it does, not for the team's status, because
the two do not coincide. The patch is applied whole, with no team
picker: without the marker an export's `name` and `shirt_name` are not used, so a managed team's
players keep the names their manager set in the save editor, and a generated file's names do no
harm. No finding reports them unused: every generated file carries names, so it would fire on
nearly every export. Without the marker the patch is still written: it carries the players'
motions.

Compile-time flow: resolve boots/gloves IDs from the models and link files present (a broken link is
caught as `link_target_missing` — a check impossible in the current split-tool workflow), falling
back per category to the authored stock `boots_id`/`gloves_id` → parse accepted TOML → compile →
resolve the settings and the IDs of content that was actually written, then per engine:

- **Fox:** each compiled player's `PlayerAppearance.bin`, `BootsList.bin` and `GloveList.bin` rows
  are built from the resolved settings and written into the output CPK with every other player's
  installed row (see "Bins accumulation" in the [Team compiler plan](../team_compiler/pipeline.md));
  the resolved motions, and a `write_names` export's names, go into the **aesthetics patch** written
  with it: motions are player-record fields outside the appearance block a table row holds.

If a local savefile is configured, the compiler applies that patch to it through `pes_savefile`
(decrypt, apply, re-encrypt, `.bak`); a Fox patch's write leaves the player's appearance id as it
is, so a stripped player stays stripped. The patch is what a DLC builder hands to the savefile
builder, who applies it in the save editor; the compiler has no other savefile write path (see
"Post-processing" in the [Team compiler plan](../team_compiler/pipeline.md)).

---
