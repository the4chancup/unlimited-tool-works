# Unlimited Tool Works: PES 2014 saves (branch `pes14`)

> **Plan:** work not built yet. This file lives only on the `pes14` branch, which never merges
> into main; it holds the branch's plan, its decisions and its steps.

The Save editor learns to open, edit and save PES 2014 `EDIT.bin` files, for Akko League VII,
the one cup that plays PES 2014. The builds are unofficial: no official cup plays PES 2014, so
on main this support would be dead code every change must keep building and testing. The branch
keeps it ready for the day one does (main's DECISIONS 2026-11-19 "PES 2014 support lives on its
own branch").

## Goal and scope

Akko's managers edit their squads with a PES 2014 build of 4ccEditor
(`C:\Data\4cc\Tools_4cc\4ccEditor-Akko7`, "the fork" below). It lets the cup run, but it loses data on every save (below). The goal is a
build that does the fork's job without those losses, used for the cup's mid-cup tactical
changes.

**Build 1, parity:** what the fork edits, read and written correctly, with the Gameplan screen
disabled:

- open and save `EDIT.bin`, found in `Documents\KONAMI\Pro Evolution Soccer 2014\save`;
- players: name, positions, stats, skills, motions, playstyle, goal celebrations, and the
  Appearance page: face, hair, physique, boots, gloves and the strip style (sleeves,
  undershorts, socks, shirt untucked), which on PES 2016 and 2017 hides the game's kit under a
  player's model;
- teams: name, roster, shirt numbers. PES 2014 stores no team colours (run `colors1`, below),
  so the team's colour cells stay greyed, as on any version without them. The team kits (the
  game's Strip menu) are kept exactly as read: the editor has no kit design screen on any
  version;
- the tactics record, mapped whole so the editor reads and writes it exactly and the AATF
  knows the captain (below);
- the AATF check against the Akko VII ruleset, embedded as `crates/libs/aatf/rulesets/akko7.rhai`
  like the other events' rulesets, first in the ruleset dropdown. The build's bundled
  settings pick it, as DBC's build picked DBC's, while the Save editor's own default stays
  the official ruleset. Its logo is the event's
  (`C:\Data\Pictures\4ccpics\Akko_League_VII_Logo_1.png`), scaled to 84 pixels high (137 × 84)
  as the other rulesets' logos are; its page is
  `https://implyingrigged.info/wiki/Akko_League_VII:_/fluff/_Bottomed_Girls`. No game has a list of trick cards: a cup's
  organizer names the cards that hardly change play, which are then free or have their own
  limit. Akko VII's page sets the free tricks per medal but names no cards, so its ruleset's
  `card_groups.tricks` is the official 4cc list (`autumn26.rhai`'s), limited to the cards PES
  2014 has: Scissors Feint, Flip Flap, Marseille Turn, Sombrero and Cut Behind & Turn (Precise
  Touch on PES 2014's screen). The
  page's numbers fit later games, where Injury Resistance stops at 3 and the weak foot at 4,
  while PES 2014 shows both on bars of 8. The ruleset reads each number the page gives as the
  value the game shows, so a weak foot capped at 2 is a 2 on PES 2014's bar. Where the page
  says "any amount":
  - a medal's Injury Resistance runs to 8;
  - a player of 175 cm or less may have a 4/4 weak foot, not 8/8, because both height tables
    give 4/4 and the fork checks 4/4.

  The page sets no age, weight, formation or team-settings rule, so the ruleset has none;
- the edit flags the game needs to show what was edited (below).

**Build 2, the Gameplan screen:** the tactics record's fields on screen and editable, each
pinned in game first. The organizer wants tactics in the tool.

**Why the tactics record is mapped in Build 1.** The captain lives in the tactics record, on
PES 2014 as on every version, and the AATF needs it: the captain's Captaincy card is free, and a
team with no captain is reported. The codec reads and writes a tactics record only through a
full layout: `VersionSchema.tactic` is mandatory, and the formation and man-marking runs are
always written. Skipping the record would leave the captain unknown, so the AATF would charge
the captain's Captaincy card and report a missing captain. Mapping the record first costs
Build 1 the largest piece of reverse engineering, and makes Build 2 mostly screen work.

**Not in scope:** Team TOML export and import, patches, diff, the teams-list export, Texport,
and the player menu's Generate settings.toml and Generate in-game face, unless one works on PES
2014 with no work beyond a test. The rest stay greyed out for PES 2014, with a tooltip saying
so. The two Generate items cannot work yet: PES 2014's face, hair and skin are not mapped, so
a generated file would carry zeros for them.

## Re-derived, not copied

The fork is a hint for where things are, never the source of how they work. Its reported
failures on PES 2014 (Akko managers, 2026-11):

- the captain reads as the roster's first player, while the game keeps its own captain;
- the whole appearance page reads back as defaults, so physique, goal celebrations and the
  rest are not saved;
- the player's strip style reads back as a default;
- team colours read back as 0/0/0, where the save stores none (run `colors1`).

The pattern: a field the fork does not decode is shown as a default and written back as that
default. Two rules follow.

1. **Every bit the editor does not map is written back exactly as read.** A record is
   decoded, its mapped fields changed, and the rest of its bytes left alone. A byte-identical
   round trip of the real Akko saves is a gate.
2. **Every field the editor edits is pinned.** A field's offset, width and value order come
   from saves where that one value was changed in game, a few steps apart, or from values the
   game shows for the Akko saves' players (every field a check reaches is written down here,
   not only the one it was for). A field that cannot be pinned is not editable: its bytes are
   preserved, and the editor shows it greyed.

## What is known about the save

From the fork's codec, checked against the vanilla save and the Akko saves (2026-11-18/19):

- **Container.** `EDIT.bin`, `TEXPORTnn.bin` and `SYSTEM` decrypt with the PES 2015 scheme
  (`container/pes15.rs`) unchanged, and carry the same `"Edit Data"` description. PES 2014 is
  told from PES 2015 by the payload's size: 4 726 440 bytes against 5 782 808.
- **Bit order.** Fields are little-endian bitfields: a field's bit `i` is the record's bit
  `start + i`, counted from the least significant bit of each byte. An MSB-first reading decodes
  garbage.
- **Sections**, at fixed offsets in a payload of constant size, each with a u16 record count in
  the payload's header (the Akko saves' count, then the vanilla save's):

  | Section | Offset | Record size | Count at | Counts |
  |---|---|---|---|---|
  | Players | 0x4C | 0x6C (108) | 0x34 | 5060, 11998 |
  | Appearance | 0x29332C | 0x58 (88) | 0x36 | 5060, 0 |
  | Teams | 0x36A0AC | 0x13C (316) | 0x38 | 222, 478 |
  | Rosters | 0x3E5CD4 | 0xA4 (164) | 0x42 or 0x46 | 222, 478 |
  | Tactics | 0x4081A0 | 0x1F6 (502) | 0x46 or 0x42 | 222, 478 |

  Which of 0x42 and 0x46 counts which is not known (both hold the team count in both saves),
  and it does not matter: the editor counts both sections by the team count at 0x38, as on
  PES 2015, since each holds one record per team. 0x3A (45 in both) and 0x3C (18 in the
  vanilla save, 0 in the Akko ones) count something else.

  A roster record and a tactics record each start with their team's id as a u32, in the team
  section's order (checked on every record of the vanilla save and `.akko7v2`). The fork's
  notes put both sections elsewhere. Its rosters started at 0x3E5D78, one record late, where
  the first record is the second team's. Its tactics started at 0x40858C with
  1004-byte records: two records read as one, which is why its keys skipped every other team.
  A team record's u32 at +0 is the team's id in its low 16 bits and 0xFFFF in its high 16.

- **Edit flags, as the organizer found them.** The organizer's merge set only bit 0x29:1 of
  each player record, and the game showed placeholder names and flat stats; setting bits 2 to 7
  of byte 0x28 and 1 to 7 of byte 0x29 as well fixed it (checked in game by the organizer). So
  those bits were read as one flag per edited part, the game showing base data for a part whose
  flag is clear. The runs below have not found one bit per part. Bit 0x29:0 is left alone: it
  may mark a base copy.
- **Pinned in game, run `flags1`** (2026-11-19, one edit on each of ten created players of the
  vanilla save; the saves `flags1_base` and `flags1` are kept in the worktree's
  `.tmp/ingame_saves/saves_14/`). Bit positions are a byte of the 108-byte player record and a
  bit counted from its least significant one; each edit changed the lowest bit of its value,
  so a field's width is still open:
  - Age 17 to 18: byte +0x10 went 0x11 to 0x12, the age itself;
  - Height 180 to 181: bit 0x0C:0;
  - Attacking Prowess 80 to 81: bit 0x11:4;
  - CF position grade C to B: bit 0x13:6;
  - Dribbling motion 1 to 2: bit 0x27:6 (one bit for one step, so stored from 0);
  - the Goal Poacher playing style on: bit 0x20:4;
  - Name Print set to `FLAG`: text at +0x5C;
  - face, hair and boots edits wrote no player-record bit: each created a record in the
    appearance table (its u16 count at 0x36 went up).

  A created player gets bits 0x28:2 and 0x29:1 when it is created, and no part edit moved a
  flag but Basic Settings', which set 0x28:6: a created player has no base data for the game to
  fall back to, so its part flags do not matter. They are mapped on the Akko saves' players
  instead, whose flags are clear (5036 of 5060 in `.akko7v2`).
- **Pinned in game, run `flags2`** (2026-11-20, the same ten edits on `/3/`'s players 1 to 10 in
  `.akko7v2`, whose flags were clear; kept as `flags2`). The game set no flag per part: every
  edit stored in the player record set bit 0x29:1 alone, and Basic Settings also set 0x28:6, as
  on a created player. The organizer's `.akko7v2_flags` sets 0x28 to 0xFC and 0x29 to 0xFE on
  every player instead. Run `flags3` loaded `flags2` again in a fresh game: every edit showed,
  the player-record ones with 0x29:1 alone and the face, hair and boots ones with no player
  flag. So the game's own flags keep its edits. The organizer's placeholder names are the
  Player Name, which 0x29:1 alone does not show (run `player1`). The values `flags2` moved
  agree with `flags1`'s offsets, and add:
  - Age is the low 6 bits of +0x10 (24 read 0x58, 25 0x59);
  - Height 180 cm is 0x30 at +0x0C, so the byte likely holds the height less 132;
  - the face, hair and boots edits wrote only the player's appearance record. A face edit
    cleared its u32 at +0x04, which in 5037 of `.akko7v2`'s 5060 records repeats the player's
    id; it set bits 0x0B:7, 0x0F:7 and 0x4E:3 and moved bytes at +0x20 and +0x23 (the build
    point). A hairstyle edit set bits 0x0F:7 and 0x4E:3 and nothing else. Boots to the next
    entry wrote 0x00 0xC0 to 0xFD 0xC3 at +0x08;
  - the Motion edit on list position 10 changed no record of player 10, but bit 0x27:6 of
    player 9's, the Dribbling bit `flags1` found: it landed on player 9, who shows it after
    the reload while player 10 keeps his base value (`flags3`).
- **Pinned in game, run `tactics1`** (2026-11-20, one Game Plan change on each of teams 2 to 7
  of `.akko7v2`'s English League, kept as `tactics1`). Offsets are bytes of the 502-byte
  tactics record:
  - the captain at +0x1CC, one byte: #1 to #2 wrote 0 to 1;
  - the Long FK taker at +0x1C6: #1 to #2 wrote 0 to 1, and the game moved the byte at
    +0x1C8 from 1 to 2 with it (the next taker in the list, as it avoids one player in two
    slots);
  - the lineup at +0x146, 32 u32s: entry k is the roster slot playing position k. Swapping two
    players exchanges their entries. Every team of `.akko7v2` has the lineup in roster order;
  - three preset blocks, 0x6C bytes apart (Preset Tactics 1 to 3). In Preset Tactics 1, the
    Strategy is at +0x5A (Custom 0, Long Ball 1, the dropdown's order), and picking Long Ball
    also set +0x60 to 5 and +0x61 to 15, the Defensive Compactness and Shifting to Attack the
    screen then showed. A formation change (8-1-1 to 4-3-1-2) rewrote one byte every 4 from
    +0x12 to +0x2E and u16 values from +0x32 to +0x58. A swap of two starters exchanged their
    bytes in presets 2 and 3 (0x6C and 0xD8 later), not in preset 1;
  - the captain byte counts roster slots, not lineup positions (run `tactics2`, kept as
    `tactics2`): on `/b/`, whose #12 plays #11's centre-forward spot, the captain could be put
    on #12, and on `/a/` the C mark followed #2 when he swapped places with #3. Whether the
    taker bytes count the same way is open.

  Saving also rewrote the competition data after the team records (from 0x3AB9E4, "Club
  International Cup") and the header byte 0x4A. The Edit Player cards are PES 2015's names:
  18 playing styles, 7 COM styles and 20 skills, Precise Touch among them (no Cut Behind &
  Turn: Precise Touch's description names the Cut Behind move).
- **Pinned in game, run `player1`** (2026-11-20, five `/4ccg/` players of `.akko7v2` read
  against the fork's decoding, nothing changed; findings in the worktree's
  `.tmp/pes14_ingame/player1/`). The game reads a player's record only when bit 0x29:1 is set
  (the fork's "edited player"). The 5036 placeholder records have it clear, and for them the
  game shows its own base data: a record holding 180 cm showed 160. With 0x29:1 alone every
  field below showed but the Player Name, which read "PLACEHOLDER" on all five while +0x2E holds
  their names; Name Print (+0x5C) showed. That is the organizer's "placeholder names". Run
  `player2` (one of them edited in game, each field set to its own value, saved and kept as
  `player2`) found the name's bit: a Player Name edit sets 0x28:2, and the game then shows
  +0x2E. It also told apart every stat, grade and motion below. Fields, as byte:bit, width,
  and how the value is stored:
  - nationality 0x0A, 16 bits (146 is BRAZIL); height 0x0C, 8 bits, cm less 132; weight 0x0D,
    8 bits, kg less 30; goal celebrations 1 and 2 at 0x0E and 0x0F, 8 bits, 0 for OFF;
  - age 0x10:0, 6 bits; Tenacity 0x10:6, 6 bits, less 40 (the fork reads it as Physical
    Contact);
  - the other stats, 6 bits each less 40, in the screen's order: Attacking Prowess 0x11:4,
    Ball Control 0x14:6, Dribbling 0x14:0, Low Pass 0x16:2, Lofted Pass 0x17:0, Finishing
    0x15:4, Place Kicking 0x18:6, Controlled Spin 0x19:4, Header 0x24:6, Defensive Prowess
    0x12:2, Ball Winning 0x18:0, Kicking Power 0x1C:0, Speed 0x1B:0, Explosive Power 0x1C:6,
    Body Balance 0x24:0, Jump 0x1D:4, Goalkeeping 0x13:0, Saving 0x1A:2, Stamina 0x1E:2;
  - the registered position 0x1F:0, 4 bits (GK 0, RMF 7, SS 11, CF 12); the position grades,
    2 bits each, C 0, B 1, A 2 (the game offers no fourth): CF 0x13:6, SS 0x17:6, LWF 0x1B:6,
    RWF 0x23:6, AMF 0x25:4, LMF 0x26:2, RMF 0x26:4, CMF 0x26:0, DMF 0x25:6, LB 0x27:0, RB
    0x27:2, CB 0x26:6, GK 0x27:4. The registered position's grade is edited like any other;
  - weak foot accuracy 0x1F:5, form 0x21:1, injury resistance 0x21:4 and weak foot usage
    0x21:7, 3 bits each, the shown 1 to 8 less 1; the stronger foot 0x2A:0 (set for left);
  - the playing style 0x20:4, 5 bits: 0 for none, then the game's 18 in screen order (Goal
    Poacher 1 to Defensive Goalkeeper 18). The fork's names for 15 to 17 are PES 2018's list
    and wrong here (its 17 shows in game as Offensive Goalkeeper);
  - the COM styles 0x2A:1 to 0x2A:7 and the 20 skills from 0x2B:0 to 0x2D:3, each in the
    game's screen order (card `i` at 0x2B + i/8, bit i%8). Precise Touch is card 5, its
    description naming the Cut Behind move: Cut Behind & Turn under its older name;
  - motions, each the shown value less 1, the game offering 1 to the top given: dribbling
    0x27:6 (2 bits, 4), free kick 0x20:0 (4 bits, 12), penalty kick 0x22:5 (3 bits, 4), punt
    kick 0x28:0 (2 bits, 4);
  - the flag bits a part edit sets: 0x29:1 for any player-record edit, 0x28:2 for the Player
    Name, 0x28:6 for Basic Settings; Name Print, Position, Ability settings, Player Index,
    Physique and Motion set no other.

  An in-game save also moved payload byte 0x4A (0x19 to 0x1D here, 0x1F in `flags2`), which
  may count the players with 0x29:1 set; not pinned.

  The save's leagues as the game lists them: English League holds the 4cc teams, Ligue 1 the
  Backup teams, Italian League the Invitational teams and `/4ccg/`, LIGA BBVA the /vg/ League
  teams, and Other the created players.
- **Pinned in game, run `appear1`** (2026-11-20, on `/4ccg/`'s player 85720 in `.akko7v2`:
  every look item set to its lowest and saved, kept as `appear1_min`, then to its highest,
  kept as `appear1_max`; findings in the worktree's `.tmp/pes14_ingame/appear1/`). Offsets are
  byte:bit of the 88-byte appearance record, and each list is stored in the screen's order
  from 0. The fork reads the same offsets for every item below that it reads:
  - boots 0x08:0, 14 bits, a boot code: the grid's first boot (Predator LZ TRX) is 1011, its
    last (WEPES SP2000) 83. Bits 0x09:6 and 0x09:7, above it, read 1 in all three saves;
  - the face's edited flag 0x0B:7 and the hairstyle's 0x0F:7, each set by its part's edit (a
    Default hairstyle sets none);
  - skin colour 0x0C:0, 3 bits, the swatch (1 to 6) less 1;
  - the 14 physique sliders, -7 to +7, 4 bits each holding the value plus 7, from 0x45:0 in
    this order: Neck Length, Neck Size, Shoulder Height, Shoulder Width, Chest, Waist, Arm
    Length, Arm Size (Arm Length first, unlike the screen), Thigh, Calf, Leg Length, Head
    Length, Head Width, Head Depth. The Face screen's head rows are the same three fields;
  - wristband 0x52:5, 2 bits (Off, Right, Left, Both); undershorts 0x52:7; sleeves 0x53:1, 2
    bits (Short, Long, Auto); sock length 0x53:3, 2 bits (Normal, Long, Short); spectacles
    0x54:6, 3 bits, the grid's cell less 1; player gloves 0x56:2 (Off, Auto); tape 0x56:3;
    shirttail 0x56:4 (Untucked, Tucked In);
  - inferred, each the one changed run its item fits: skin type 0x4C:5, 4 bits, less 1; neck
    0x4E:0, 3 bits, less 1.

  Not told apart, since both passes changed them together: hairstyle, hair variation, hair
  colour, and forehead and wrinkles. Forehead or
  wrinkles fits the 3-bit run at 0x4E:3, but run `flags2`'s hairstyle edit set bit 0x4E:3
  alone. The hair accessory colour did not store: its row is greyed under the hairstyle the
  second pass chose. The face's build points, its sliders and its upper and lower face items were
  left alone. Across `.akko7v2`'s 5060 appearance records only the ids, the boots, the skin
  colour, byte 0x13 (the fork's hair-accessory red), byte 0x34 (the fork's iris colour, 0x34:4,
  4 bits), the sliders and bytes 0x53 to 0x56 vary: the Akko players otherwise share one look.

  The save also changed the player record. It set bits 0x28:6 and 0x28:7, and it wrote
  "PLACEHOLDER", the name the game showed while 0x28:2 was clear, over the Player Name at
  +0x2E.
- **Pinned in game, run `appear2`** (2026-11-20, three `/4ccg/` players of `.akko7v2`, one
  look part each, one save kept as `appear2`; findings in `.tmp/pes14_ingame/appear2/`):
  - the wristband's colours, Right Side 0x4F:4 and Left Side 0x4F:1, and the spectacles'
    colour 0x50:0, 3 bits each, the swatch (1 to 8) less 1;
  - long-sleeved inners 0x53:5, 2 bits (Off, Normal, Turtle Neck);
  - a physique slider, a strip style item, or the wristband, its colours and the spectacles
    set no flag bit in either record, and leave 0x28 clear. So the face's flag 0x0B:7 is the
    face's alone, and 0x28:6 and 0x28:7 come from one of the edits `appear1` made beyond
    these: the skin colour, skin type, forehead, wrinkles or neck, the hairstyle, a head
    slider, the boots, the player gloves, the undershorts or the tape. Which one is not
    pinned;
  - the game greys the Wristband row while Sleeves is Long.
- **Pinned in game, run `colors1`** (2026-11-20, the shirt of `/4ccg/`'s first kit in
  `.akko7v2`, four saves `colors1` to `colors4`, each moving one of its two colours; findings
  in `.tmp/pes14_ingame/colors1/`): PES 2014 stores no team colours. Its team menu offers
  none (Game Plan, Squad Number, Team Name, Emblem, Strip, Home Ground), and no team record
  holds a colour: the bytes +0x04 to +0x2B, the only ones that vary between the 222 team
  records, hold none. The only colours are a kit's:
  - the kit table, from 0x3B0176, of 0xB0-byte records, each opening with a team ID (u16). A
    record is filled only once its team's kit is edited in game; until then it is empty
    (`ff ff`, then zeros) and the game draws a default white kit. `/4ccg/`'s was empty
    before the run;
  - in that record, the first kit's shirt colour: red 0x05:2, green 0x12:0, blue 0x12:6;
    its Design 1 colour: red 0x13:4, green 0x14:2, blue 0x15:0. Each channel is 6
    bits, the colour's 0 to 255 divided by 4;
  - a kit edit also sets bit 0x0A:4 of the team record, and byte 0x3E of the payload goes
    from 0 to 1.
- Two more tables sit in the payload, unread by the fork: a second team table at 0x37B2B4,
  right after the first (team ID at +0, name near +0x78), and a player table keyed by player
  ID right after the appearance table, at 0x2DABAC. The vanilla save leaves both empty, and
  so do the Akko saves and every save of the runs.

## What is mapped in game

Each item ends with its offsets, widths and value orders written into "What is known about
the save" above.

- **The tactics record:** the captain, and every run the codec writes (formations, man
  marking, presets), so a record reads and writes back unchanged.
- **The appearance record**, what runs `appear1` and `appear2` left (above): hairstyle, hair
  variation and hair colour; forehead and wrinkles; the hair-accessory colour; the skin type
  and the neck; the iris colour; the face's build points, sliders and upper and lower face
  items; the Copy from ID, which the fork reads at +0x04 (the u32 that repeats the player's
  id, and which a face edit cleared in run `flags2`); and which edit sets 0x28:6 and 0x28:7.
  Build 1 needs none of them: the Save editor
  edits these looks only through the in-game face run, which stays greyed on PES 2014, and the
  game keeps an edit without 0x28:6 and 0x28:7 (run `flags3`).

## PES 2014 in the code

`PesVersion` gains `Pes14`, whose engine is the pre-Fox one (PES 2014 is the first pre-Fox
game). It stays out of `PesVersion::ALL`, so the version pickers and the tests that loop over
every version are the same as on main, and main's merges into the branch stay small. Only the
Save editor reaches `Pes14`, by detecting a PES 2014 save; every other tool refuses it.
`PesVersion::from_number` keeps refusing 14, so a settings file, a ruleset's `versions`, a
`.4ccs` file or `PES2014.exe` never names PES 2014: the Save editor follows the opened save's
version, not the version selector.

- **Detection.** A PES 2014 save decrypts as PES 2015 (`Scheme::Pes15`), and
  `SaveContainer::version()` is the one place that would misname it. It checks the payload: a
  `Scheme::Pes15` payload of 4 726 440 bytes, an `EDIT.bin`'s, is PES 2014. A PES 2014
  `TEXPORTnn.bin` has a payload of its own size, 5 316 176 bytes (the Akko export's), so the
  check names it PES 2014 too, and it is refused as a Texport with no layout; read as PES 2015,
  it would be read at PES 2015's offsets. Neither size names a PES 2015 file: PES 2015's
  `EDIT.bin` payload is 5 782 808 bytes and its Texport's 5 577 964 (every Texport on this PC,
  measured).
- **Schema.** `schema/pes14.rs`, hand-written (the other versions' tables are generated from
  measurements PES 2014 does not have), with the five sections above. Each per-version table
  gains a PES 2014 arm: playstyles (18, measured), motion tops and face-type caps (measured),
  advanced instructions (none), Texport (none), the stat-name renames, the nations list
  (an eighth column: the 182 nations of PES 2014's own `Country.bin`, which are PES 2015's
  less seven, each shared ID named alike), the save's folder and file name, and the pitch the formation screen
  draws (PES 2017's, the fallback for unmeasured versions).
- **Appearance records.** On PES 2014 only a player whose look was edited, or who was created
  in the save, has an appearance record: the vanilla save has none for its 11998 players, the
  Akko saves one for each of their 5060. A player without one reads no appearance fields and
  writes none, and the player's look controls are greyed. The editor never creates a record:
  the game fills a new one with the player's whole look, taken from base data the save does
  not hold, so a record the editor made would reset every look part it did not set (inferred:
  a record holds every look field, and the save holds no other copy of them). On PES 2015 and
  2016 every player has one, and a missing record still refuses the save.
- **Tenacity** is a PES 2014 stat no other version has. It gets its own `Option` stat, its
  `PlayerField` and its AATF stat name. On another version's save its greyed box's tooltip
  reads "PES 2014 only", since the "PES NN+" wording would claim later versions store it. It is
  the last ability stat, after Aggression, so that every table indexed by ability stat (a
  ruleset's stats, the Ruleset editor's columns) keeps its indices. It is no Team TOML key,
  since Team TOML is not available on PES 2014. The AATF logic lists it among the ability
  stats, so the branch's logic is `ruleset-logic 2`: a released logic's hash is frozen, and a
  ruleset file saved with version 1 still reads as official. Main's next logic change will
  meet this one when main is merged into the branch.
- **Stored values.** PES 2014 stores each stat less 40, the height less 132 and the weight less
  30 (run `player1`). The codec adds those offsets when it reads a PES 2014 record and takes
  them off when it writes one, so the model, the screens and the AATF hold the values the game
  shows, as on every other version. A value below its offset is refused, not wrapped.
- **Playing styles.** PES 2014's list is PES 2015's with two other names: The Instigator at
  15, where PES 2015 has Build Up, and Solid Player at 16, the slot PES 2015 leaves empty. Each
  is a playing style of its own, not Build Up renamed, because nothing measured says they are
  the same style, and a ruleset naming Build Up would otherwise pass a PES 2014 Instigator. No
  other version stores either.
- **Motions.** PES 2014 has four: Dribbling (a field PES 2020 stores too), Free Kick, Penalty Kick and
  Punt Kick. Punt Kick is a PES 2014-only `Option`, like Tenacity. The corner kick, hunching
  and arm-movement motions are not stored.
- **Edit flags.** The game reads a player's record only when bit 0x29:1 is set (run
  `player1`), so the editor sets the flags the game sets, which keep every edit after a reload
  (run `flags3`): 0x29:1 on each player whose player record it changes, and 0x28:6 as well
  when the change is in the game's Basic Settings part, and 0x28:2 when it changes the Player
  Name, without which the game shows a rostered player as "PLACEHOLDER" (run `player2`). An
  appearance-only change sets no flag. The flags are set when the save is written, by
  comparing each player record with the bytes it replaces, so every route that changes a
  player (a screen, an AATF fix, an import) sets them alike. The organizer's
  0xFC/0xFE on every player is not copied, since the game never writes it; a save that
  carries it keeps it, since the editor sets these bits and never clears one. On the other
  versions an ordinary edit sets no flag, as on main.
- **The tactics record in Build 1.** PES 2015's formation and man-marking layouts do not fit
  PES 2014's record: one formation per preset, a lineup of roster slots and no player ids.
  So `TacticsSchema`'s `formations` and `man_marking_bit` become `Option`s, `None` on PES 2014,
  and the codec writes no formation or man-marking run there. PES 2014's table maps the team
  id and the captain, which the AATF needs, and every other byte of the record is kept as
  read. The takers, the lineup and the presets wait for Build 2.
- **The Gameplan screen in Build 1.** The Gameplan segment is greyed with a "PES 2014: in the
  next build" tooltip; the import panel's Gameplan section and the Preset and Formation copy
  targets go with it. The captain is read for the AATF and kept as the game set it; it is
  shown and edited on the Gameplan screen, as on the other versions, from Build 2. A
  captain-only screen for Build 1 would be new design for one build's life. A team row still
  opens the team's header (name and short name, the colour cells greyed), with the tactics
  editor below it replaced by a greyed line saying the same. The formation pictures and the
  Gameplan copy targets are greyed too, since they would picture or copy a record of zeros.
- **The recent-saves list** stores a PES 2014 save's version as 14, the one settings entry
  that names it: a recent save is a file the editor opened, not a version a member picks.
  Refusing 14 there would make the whole settings table unreadable after one PES 2014 save was
  opened, and every tool would quietly read its defaults.
- **The Appearance screen** follows the schema: a control whose field the version's records
  do not map is greyed, so PES 2014's controls come alive as their fields are mapped. PES 2014
  has no in-game face run, so the FPC buttons, the glove and skin cells and the colour palettes
  are greyed there, as are the hunching and arm-movement motions, which it does not store.
  Punt Kick takes the Corner kick cell, since PES 2014 stores no corner kick either: a row of
  its own would stand greyed on every other version, and the fork does the same. Randomize
  draws it with the other motions.

  Two look items are stored in another order than on the other versions: Sleeves (Short 0,
  Long 1, Auto 2) and Shirttail (Untucked 0, Tucked In 1, where the other versions store
  "untucked" as 1). The codec translates them on read and write, as it adds the stats'
  offsets, so the model holds the same meaning on every version. Where the game names a value
  otherwise, PES 2014's dropdown uses its words: Sleeves' Auto (the other versions' Seasonal),
  Undershorts' Off and On (one bit, where the other versions store four kinds), and the
  Spectacles grid's cell numbers (Off, then 2 to 8), whose glasses are not matched to the
  other versions' names.
- **The 1-to-8 settings.** On PES 2014 Form, Injury Resistance, Weak Foot Usage and Weak Foot
  Accuracy each run from 1 to 8 (run `player1`: each shows a bar of 8), where the other
  versions' boxes stop injury resistance at 3 and the weak foot at 4.

## Releases

A build is a GitHub prerelease, tagged `pes14-v<version>`, where `<version>` is the branch's
workspace version: main's version with a `-pes14.<n>` pre-release part (`0.0.2-pes14.1`). Main's
update check reads only `v<semver>` tags, so it never offers a PES 2014 build to a member on
main. The branch's update check reads only `pes14-v<semver>` tags, so a PES 2014 build is never
offered main's next release, which cannot open PES 2014 saves. The branch's `release.yml` runs
on `pes14-v*` tags (GitHub reads the workflow file at the tagged commit, so main's file is
unchanged). A prerelease is never marked Latest, so the repository's front page keeps showing
main's release. Each cut opens `CHANGELOG.md` with the build's own section, what it does on PES
2014 and what is still greyed there, above main's: the app's What's new chapter shows the
bundled changelog, which would otherwise present main's last release as the build's news.

## Verification

- **Fixture.** The vanilla PES 2014 `EDIT.bin`'s payload, committed compressed as
  `pes14_payload.bin.zz` beside the other versions' fixtures.
- **Real saves, local only.** The Akko saves (`EDIT.bin.akko`, `.akko7v2`, `.akko7v2_flags`,
  in the PES 2014 save folder) stay out of git. A local check opens each, saves it unchanged,
  and compares the payloads byte for byte; it opens `/4ccg/` (team 857) in `.akko7v2` and
  checks its 23 players by name.
- **`/4ccg/`'s VGL 26 export.** `/4ccg/` is a VGL team invited to Akko VII, and its Akko team
  was ported from its VGL 26 aesthetics export: a Fox-era export with kits, each player's
  settings and `colors.txt`, whose team colours are 48 146 82 and 221 0 0. A copy, upgraded to
  Studio's export format, is in the main tree's `.tmp/0_0_2/ingame/poc/run/exports/`. So the export is a second reading of
  what the Akko save should hold for those 23 players and their team, such as names, order,
  appearance settings. Its two colours have no place in the save, which stores no team
  colours (run `colors1`).
- **In game.** PES 2014 is installed at `E:\PES2014`, driven by `scripts/ingame/game.py 14`
  (a pad, OBS shots, no keyboard focus). The mapping saves are made there, and Build 1 ends
  with the edited `/4ccg/` looked at in Edit mode. Where the editable players are:
  - in the vanilla save, the leagues' squads are "Unregistered" dummy slots whose edits the
    game drops; created players (Edit > Players > Create Player, a name required) keep theirs;
  - in the Akko saves, the first league (English League) holds the 4cc teams, blank (23
    PLACEHOLDER players each). Akko is an invitational, so the teams playing it, `/4ccg/`
    among them, are in one of the lower leagues and in the save's Invitational team section
    (ids 851–920, `spec/libs/teams_list.md`): `/4ccg/` is 857 there, while 784, its id in the
    /vg/ League section, holds a blank team named "VGL 4".

## Process

Second-class work, lighter than main's: the implementation runs on SWE-2 High; reviews are
SWE-2 rounds only; no mutation runs. The per-commit gates (`docs/CONTRIBUTING.md` "Testing and verification") run as
on main. Main is merged into the branch, never rebased onto it.

## Steps

1. The plan and `game.py`'s PES 2014 entry.
2. The mapping above, in game (the lead).
3. Open and save: `Pes14`, detection, the save's folder, the fixture, `schema/pes14.rs`; the
   fixture and the Akko saves written back byte for byte. No screen work.
4. The Save editor on PES 2014: Tenacity, the labels, playstyles, motions and nations, the
   edit flags, the Gameplan screen and the out-of-scope features greyed. Step 3 left to it:
   - the Texport's payload size in the detection (above);
   - the save's discovery: the editor's list of saves loops over `PesVersion::ALL`, so it never
     lists a PES 2014 save;
   - the look controls greyed for a player who has no appearance record (above);
   - the face-type caps, which read each field's bit maximum until measured;
   - Tenacity among the ability stats (`PlayerStats::abilities()`, `ABILITY_STATS`) with its
     AATF name, which step 5 needs: slice 1 (the player record mapped) left it out;
   - no panic on a PES 2014 save. The screens assume what every other version stores: the
     Age, Height and Weight boxes take their field's maximum, and the Appearance segment, the
     player and team menus' FPC entries and a roster drag read the in-game face run, which
     PES 2014 does not map. Each such read follows the schema (a field it lacks is greyed), and
     a test opens the Akko save and draws every segment, menu and drag with a player and a team
     selected;
   - a box's range on PES 2014: `schema::player_max_value` gives the run's highest stored
     value (63 for a stat), while the model holds shown values, so a box's range adds the
     stored offset at both ends (a stat's run holds 40 to 103, the height's 132 to 387);
   - a field only PES 2014 stores (Tenacity): `first_version_with` searches
     `PesVersion::ALL`, so its greyed box's tooltip would find no version; it reads "PES 2014
     only" (above);
   - the overlong shirt name's tooltip "(PES 2016 shows 16)" names PES 2014 too;
   - skill card 4 reads "Precise Touch" on PES 2014, the game's older name for Cut Behind &
     Turn (`labels::card_name` takes no version today);
   - the Team menu's boots and gloves dialog on PES 2014: PES 2014 stores no gloves id, so the
     dialog sets the boots alone, and refuses when one of the team's players has no appearance
     record.
5. The Akko VII ruleset (`akko7.rhai`, its logo and page); `/4ccg/` in `.akko7v2` passes as it
   does in the fork's fixed build.
6. Releases: the `pes14-v` tag prefix in the update check and `release.yml`. The version
   `0.0.2-pes14.1` is set at the cut, in step 7.
7. Build 1's check: `/4ccg/` edited in the build, saved, looked at in game in Edit mode. The
   built window is captured where the branch changed a look: Ability stats with Tenacity's
   row, on PES 2014 and on a later version, where the row is greyed, and the Ruleset editor's
   stat grid with its Tenacity column. Then the release.
8. Build 2: the Gameplan screen. The game offers no free position move: a position changes
   only by picking one of the formation list's entries or swapping two players, and a
   player's role is shown, not chosen (run `tactics1`). So the screen is the game's own
   Game Plan, not the other versions' pitch editor.
   - Run `tactics3` (in game): every Game Plan setting not yet pinned, one change per team:
     the eight sliders one step each, then all eight at their maximum on one team (their
     widths); the Strategy's last entry; the six other set-piece settings; One-Touch
     Settings; the four Tactical Assistance switches; a Combination Play slot's strategy and
     area; one slider in Preset Tactics 2 (the 0x6C stride).
   - Run `formations1` (in game): each entry of the formation list set once, so the editor
     writes a formation as the game stores it.
   - The draft: the Gameplan screen on a PES 2014 save (the lead).
   - Slices: the record's fields in the schema and the model, then the screen.
