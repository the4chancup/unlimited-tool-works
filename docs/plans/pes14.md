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

**Not in scope:** patches, diff, the teams-list export, Texport,
and the player menu's Generate settings.toml and Generate in-game face, unless one works on PES
2014 with no work beyond a test. The rest stay greyed out for PES 2014, with a tooltip saying
so. The command line refuses their commands on a PES 2014 save in the same words
(`apply-patch`, `diff`, `export-teams-list`), so both routes keep
one rule. The two Generate items cannot work yet: PES 2014's face, hair and skin are not mapped, so
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
    bytes in presets 2 and 3 (0x6C and 0xD8 later), not in preset 1. Made with Preset Tactics
    2 or 3 shown, a swap exchanged the two positions' roles and spots in that preset alone,
    and left the lineup as it was (run `tactics5`, kept as `tactics5`: `/a/` in Preset Tactics
    2, `/aco/` in 3);
  - the captain byte counts roster slots, not lineup positions (run `tactics2`, kept as
    `tactics2`): on `/b/`, whose #12 plays #11's centre-forward spot, the captain could be put
    on #12, and on `/a/` the C mark followed #2 when he swapped places with #3. Whether the
    taker bytes count the same way is open.

  Saving also rewrote the competition data after the team records (from 0x3AB9E4, "Club
  International Cup") and the header byte 0x4A. The Edit Player cards are PES 2015's names:
  18 playing styles, 7 COM styles and 20 skills, Precise Touch among them (no Cut Behind &
  Turn: Precise Touch's description names the Cut Behind move).
- **Pinned in game, run `tactics3`** (2026-11-20, one Game Plan change on each of 24 teams of
  `.akko7v2`, kept as `tactics3`). Offsets are bytes of the tactics record; a preset's offsets
  are those of Preset Tactics 1, and Preset Tactics 2 and 3 sit 0x6C and 0xD8 later (Player Support in
  Preset Tactics 2 moved +0xC7):
  - the eight sliders at +0x5B to +0x62, one byte each in the panel's order: Player Support,
    Support Range, Position Switch, Front line pressure, Defensive Line, Defensive Compactness,
    Shifting to Attack, Attack/Defence Level. Each runs from 0 to 20. Support Range is stored as
    20 less the value shown: shown 20 is 0, and 19 is 1;
  - the Strategy at +0x5A is the dropdown's index: Custom 0, Long Ball 1, Quick Counter 2,
    Possession Game 3, All-out Defence 4, All-out Attack 5 (the middle three confirmed by run
    `formations1`). A pick sets some sliders as well: All-out Attack set
    Front line pressure, Defensive Line and Shifting to Attack to 15 and Attack/Defence Level to
    20;
  - the takers, one byte each holding a roster slot: Long FK +0x1C6, Short FK +0x1C7, FK Taker 2
    +0x1C8, Left CK +0x1C9, Right CK +0x1CA, PK +0x1CB (the captain follows at +0x1CC). When a
    new Long FK or Short FK taker is the FK Taker 2, the game moves FK Taker 2 to the next roster slot;
  - Players to Join Attack at +0x1CD to +0x1CF: roster slots, 0xFF in an empty one;
  - Tactical Assistance: Substitutions at +0x1D0 (OFF 0, "Very Early" 3: a list of timings, not
    a switch), then one byte each, OFF 0 and ON 1: Offside Trap +0x1D1, Change Preset Tactics
    +0x1D2, Combination Play +0x1D3;
  - Combination Play's three slots from +0x1D6, two bytes each: the area (1 to 11), then the
    strategy's id. The ids are not the picker's order: every team of `.akko7v2` holds Dummy Run
    and Cross (10) in area 2, Hold up the ball (1) in area 6 and Hold Up and Overlap (5) in area
    8, and Cross and Pull Back is 11;
  - One-Touch Settings (Data Management) is not in the team's record: switching it off changed
    no byte of `/f/`'s record, nor any outside the competition data every save rewrites. The
    Gameplan screen leaves it out.
- **Pinned in game, run `formations1`** (2026-11-20, one Game Plan change on each of teams 2 to
  12 and 40 to 59 of `.akko7v2`, kept as `formations1`). Offsets as in `tactics3`; no other
  team's record changed:
  - the formation list holds 11 entries, in this order: 4-4-2, 4-3-1-2, 4-2-3-1, 4-2-1-3,
    4-3-2-1, 4-1-2-3, 4-3-3, 3-2-3-2, 3-1-4-2, 3-2-2-3, 5-2-2-1. The Akko teams' 8-1-1 is none
    of them, so the dropdown opens on the first;
  - a formation is the ten outfield players' roles and places. The role of position k (k = 0
    to 9) is the byte at +0x06 + 4k, in the game's role numbers: CB 1, LB 2, RB 3, DMF 4,
    CMF 5, LMF 6, RMF 7, AMF 8, LWF 9, RWF 10, CF 12. Its place is two u16s from +0x32 + 4k:
    the depth from the own goal, then the width, larger on the right (4-4-2's centre backs
    stand at 10, 63 and 10, 41, its full-backs at 12, 89 and 12, 15). A pick wrote no slider
    or Strategy byte. It also set +0x2E from 4 to 3 on every team, whatever the formation,
    and left +0x30 (52 on every team) alone. The two are the keeper's place (inferred): the
    places run holds 11 pairs from +0x2E, the keeper's first. The width 52 is the keeper's on
    every version, the other versions store his depth as 3, and every team and preset of the
    vanilla save and of `.akko7v2` holds 4, 52 there. The keeper's role has no byte: a
    preset block's first six bytes hold the team id and zeros in preset 1, 0xFFFF and zeros
    in the others;
  - the Strategy's three middle values: Quick Counter 2, Possession Game 3, All-out Defence 4.
    Each pick sets sliders too, shown in the panel's order: Quick Counter 10, 10, 0, 15, 15,
    15, 15, 10; Possession Game 15, 10, 0, 10, 10, 10, 0, 10; All-out Defence changes
    Defensive Compactness alone, to 10;
  - the Combination Play strategy ids, 1 to 14: Hold up the ball 1, Diagonal Run 2, Cut
    Inside 3, Diagonal Dribble 4, Hold Up and Overlap 5, Straight Run 6, Hold Up and Switch 7,
    Overlap and Pull Back 8, Drop Deep and Overlap 9, Dummy Run and Cross 10, Cross and Pull
    Back 11, Cross and Dummy 12, Switch Flanks 13, Dummy and Lay Off 14. Each strategy takes
    only some areas, and the picker dims the rest (run `tactics4` reads each strategy's). One
    strategy may fill two slots;
  - Substitutions, in the list's order: OFF 0, Very Late 1, Flexible 2, Very Early 3;
  - Players to Join Attack takes three players; a fourth is refused with no message. The
    bytes hold roster slots counted from 0 (#3 is 2), in the order the players were added.
- **Pinned in game, run `tactics4`** (2026-11-20, one Game Plan change on each of teams 702 to
  708 of `.akko7v2`, kept as `tactics4`; no other team's record changed):
  - moving a slider keeps the Strategy: Quick Counter stays Quick Counter. A pick sets only some
    sliders (Long Ball set Defensive Compactness 5 and Shifting to Attack 15 and left the
    others as they were), and Custom sets none;
  - Combination Play's Clear stores area 0 and strategy 0. The slot stays in the list, reading
    "None Selected";
  - the areas each strategy takes, with the other two slots empty. The game's screen numbers
    the areas in rows from the top of its pitch: 1 and 2, 3 to 5, 6 to 8, 9 to 11. The picker
    lights a strategy's areas less those the other two slots hold, and opens on the first lit
    one, except Overlap and Pull Back and Drop Deep and Overlap, which open on 7:
    - the three crosses (Dummy Run and Cross, Cross and Pull Back, Cross and Dummy): 1, 2, 3
      and 5;
    - Switch Flanks: 6, 8, 9 and 11;
    - Dummy and Lay Off, Hold up the ball, Diagonal Run, Diagonal Dribble, Hold Up and Overlap
      and Straight Run: 3 to 8;
    - Cut Inside: 3 and 5;
    - Hold Up and Switch: 3, 5, 6 and 8;
    - Overlap and Pull Back and Drop Deep and Overlap: 4, 5, 7 and 10;
  - a formation picked in Preset Tactics 2 is written to that preset's block, 0x6C later, with
    its keeper's depth set to 3 there too.
- **Read in game, run `help1`** (2026-11-20, nothing changed or saved; the texts word for word
  in the worktree's `.tmp/pes14_ingame/help1/findings.md`). The Game Plan's help button opens
  one manual of 18 topics, the same wherever it is pressed; no panel, dropdown or picker shows
  a description of its own. The manual gives a text for each slider, each Strategy choice,
  each of the 11 formations, each set-piece taker and Players to Join Attack, each Tactical
  Assistance setting, and one for Combination Play as a whole; none for a Combination Play
  strategy (its picker plays an animation) or an area. Two of its topics bear on the screen:
  - "Pitch Display": LB and RB switch the pitch between "Strip Icon", "Form / Stamina" and
    "Position / Overall Rating", so the game shows overall ratings in one display;
  - "On Pitch Controls": "move the icon to an open part of the pitch and press A again. The
    areas shown in yellow during this step indicate the player's preferred positions. (Once
    you choose a position, the player's role will be set automatically to the one with the
    highest Overall Rating.)" Run `tactics1` found no such move, because a tap jumps the
    grabbed player to the nearest teammate.
- **Pinned in game, run `tactics6`** (2026-11-20, kept as `tactics6`). The help's move exists:
  holding a direction drags the grabbed player across the pitch, and A drops him on open
  grass. On `/a/`, #2 dropped in front of the centre-backs changed one byte, preset 1's slot 1
  depth (+0x32, 9 to 15); his width and role bytes stayed, and his marker still read CB. No
  yellow areas showed in the run's shots. The pitch has two displays, "Strip Icon" and
  "Position / Overall Rating" (each starter's overall over his position); the help's
  "Form / Stamina" is not among them.
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
    8 bits, kg less 30; goal celebrations 1 and 2 at 0x0E and 0x0F, 8 bits, 0 for OFF, up
    to 101 (the highest of the vanilla save's 11998 players, whose 97 values run up to it,
    and the fork's cap);
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
    0x4E:0, 3 bits, less 1;
  - the gloves id (the gloves model; 0 is bare hands), which the base game does not store:
    the Akko DLC reads it from the hair accessory's colour fields, 6 bits each from the lowest,
    red 0x13:0, blue 0x14:0,
    then green 0x50:3 (running into 0x51:0), an id of up to 18 bits. Source: the Akko7 fork's
    `pes14.cpp`, whose author also keeps the DLC. In `.akko7v2` 14 records hold one, all 11:
    Invitational 7's players (team 857, named `/4ccg/`) beside boots 55, the fork's pair that hides an FPC player's boots
    and gloves. Gloves 11 written by the editor shows no hands in Edit mode, as the DLC's own
    gloves-11 players do, where gloves 0 shows plain hands (run `step10`, 2026-11-21).

  Not told apart, since both passes changed them together: hairstyle, hair variation, hair
  colour, and forehead and wrinkles. Forehead or
  wrinkles fits the 3-bit run at 0x4E:3, but run `flags2`'s hairstyle edit set bit 0x4E:3
  alone. The hair accessory colour did not store: its row is greyed under the hairstyle the
  second pass chose. The face's build points, its sliders and its upper and lower face items were
  left alone. Across `.akko7v2`'s 5060 appearance records only the ids, the boots, the skin
  colour, byte 0x13 (the gloves id's low bits, above), byte 0x34 (the fork's iris colour, 0x34:4,
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
- The team record's edited-name flags: 0x0A:0 for the name and 0x0A:1 for the short name,
  as PES 2015's pair at 0x12:0 and 0x12:1. The game shows a team's own name only with its
  flag set: a rename without it still showed the old name in Edit mode (run `step10`), with
  it the new one (run `flags`, 2026-11-21). The fork also sets 0x0B:4 with 0x0A:0 and 0x0B:5
  with 0x0A:1, but 0x0B:4 is set on 221 of the Akko save's 222 teams, none of them renamed,
  so it is not an edit flag, and the editor keeps both as read.
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

## What the game's code says

Read statically from `PES2014.exe` in Ghidra (2026-11-21; the run's findings and tables are in
`4cc-studio-temp/pes14_re/out/findings.md` and `overall_tables14.txt`, with each function's
address). The code is observed; where a step had to be inferred, the item says so. Run
`gameplan2` (step 8) checks the inferred parts in game before they count as known.

- **The overall at a position** (`FUN_01c11f40`), the number the Game Plan shows for a player
  at his spot:
  1. Each ability the position weighs contributes `(value - 25) × weight` (the code subtracts 25
     only from a value over 24, which every stat is); the sum is rounded, a half up (the code
     cuts the sum plus 0.5 to an integer). Run `tactics6`'s `/3/`, whose players hold 77 in
     every ability, shows this: CB 78 (71.24 rounded, plus 7), CF 71, RB and LB 74 (73.84),
     CMF 72 (71.76), GK 67 (66.56), where rounding the sum plus 0.5 to the nearest would give
     one more at CB and CF.
  2. A bonus is added, by the position, in the order GK, CB, LB, RB, DMF, CMF, LMF, RMF, AMF,
     LWF, RWF, SS, CF:
     - at his registered position: 7, 7, 8, 8, 8, 7, 7, 7, 7, 9, 9, 7, 7;
     - else at a position he plays at A: 5, 5, 4, 4, 4, 5, 5, 5, 5, 4, 4, 5, 5;
     - else at B: 3, 2, 2, 2, 2, 3, 3, 3, 3, 2, 2, 3, 3; at C, none.
  3. The player's condition arrow for a match would add −14%, −7%, 0, +7% or +14% of that,
     rounded; the Game Plan passes the middle arrow, so nothing.
  4. The result is held to 40 to 109.

  The weights, per position (LB and RB share a row, as do LMF and RMF, and LWF and RWF):

  | Position | Weights |
  |---|---|
  | GK | Goalkeeping .52, Catching .52, Body control .12, Jump .12 |
  | CB | Defensive prowess .27, Ball winning .27, Body control .21, Jump .21, Heading .20, Speed .11, Stamina .10 |
  | LB, RB | Dribbling .15, Lofted pass .15, Defensive prowess .15, Speed .15, Explosive power .15, Ball winning .14, Stamina .13, Body control .12, Jump .12, Ball control .10, Attacking prowess .06 |
  | DMF | Lofted pass .20, Ball control .19, Low pass .19, Dribbling .16, Stamina .15, Body control .14, Swerve .13, Attacking prowess .07, Defensive prowess .07, Jump .05, Ball winning .03, Speed .03, Explosive power .03 |
  | CMF | Ball control .25, Dribbling .25, Low pass .25, Lofted pass .22, Stamina .18, Explosive power .06, Attacking prowess .05, Body control .05, Speed .04, Defensive prowess .03 |
  | LMF, RMF | Dribbling .26, Speed .26, Explosive power .23, Ball control .16, Stamina .14, Lofted pass .13, Attacking prowess .07, Low pass .07, Swerve .04 |
  | AMF | Ball control .23, Dribbling .23, Low pass .23, Finishing .18, Attacking prowess .15, Lofted pass .15, Explosive power .07, Speed .05, Body control .05, Stamina .03 |
  | LWF, RWF | Ball control .22, Dribbling .22, Attacking prowess .18, Speed .16, Explosive power .16, Finishing .12, Lofted pass .10, Body control .06, Stamina .06, Low pass .05, Kicking power .05 |
  | SS | Ball control .20, Dribbling .20, Explosive power .20, Attacking prowess .16, Finishing .15, Low pass .10, Lofted pass .10, Speed .10, Body control .07, Kicking power .06, Stamina .04 |
  | CF | Finishing .38, Attacking prowess .33, Ball control .25, Dribbling .15, Body control .10, Speed .05, Explosive power .05, Heading .03, Jump .03 |

  The game reads its abilities by field numbers that name nothing, so each field was named by
  its weights across the 13 positions, which match PES 2016's settled rows almost exactly.
  Tenacity and Place kicking are the two fields no position weighs. Run `gameplan2a` checked
  the names: two `/3/` players, registered CB with every grade C, were given 20 abilities that
  all differ, and the game showed 15 overalls, each the formula's: #2 read CB 53, RB 64, CMF
  67, LMF 72, AMF 71, LWF and RWF 75, CF 78; #11 read CB 88, RB 64, DMF 57, CMF 51, CF 42.
  Run `gameplan2b` checked the grades: #2, given SS A in the save (playable value 2), read SS
  77, the sum's 72 plus the A bonus. That a value of 1 reads as B is still inferred.
- **A free move** (`FUN_027d0550`, `FUN_027c3d30`, `FUN_027c77e0`, `FUN_027c3ed0`): a player
  dropped on open grass at a pitch point (width 0 to 104, depth 0 to 48, the units of the
  stored spots) gets his role by these rules:
  - each role has a zone (below); a point inside the dragged player's own role's zone keeps his
    role and moves his spot;
  - otherwise there are up to two candidates: GK when the point is in the keeper's zone, else the
    first of CB, CMF, SS whose zone holds it; and the first of LB, RB, DMF, LMF, RMF, AMF, LWF,
    RWF, CF whose zone holds it, a side role (LB, RB, LMF, RMF, LWF, RWF) only when no other
    starter holds it;
  - the player takes the candidate at which his overall is higher, the second on a tie;
  - the drop is refused when no zone holds the point (a width under 5 or over 99, a depth over
    47, or a depth under 4 outside the keeper's widths), and when the move would leave fewer than 2 defenders, 2 midfielders
    or 1 forward, or more than 5, 6 or 5 (a line the player already stood in is not counted
    against its maximum). The lines are defenders CB, LB, RB; midfielders DMF to AMF; forwards
    LWF to CF. Which of the two candidates the line check tests is inferred (the code tests
    the first).

  Run `gameplan2b` made 13 drops on `/3/` and `/a/`, and every role the save then held, every
  drop refused and every role kept is the rule's. Among them:
  - a CF at width 7 took SS over LWF, his overall there 49 against 47;
  - a DMF at width 14 took CMF, LMF being another starter's;
  - a RMF took RWF on a tie, 72 each;
  - a CB on the right stayed CB inside RB's zone;
  - the last two defenders, the lone forward and a sixth defender were refused;
  - so was a drop at depth 3 outside the keeper's widths.

  The zones, as depth and width ranges, both ends inside:

  | Role | Depth | Width |
  |---|---|---|
  | GK | 0–3 | 52–56 |
  | CB | 4–15 | 5–99 |
  | LB | 4–15 | 5–20 |
  | RB | 4–15 | 84–99 |
  | DMF | 16–25 | 21–83 |
  | CMF | 16–34 | 5–99 |
  | LMF | 16–34 | 5–20 |
  | RMF | 16–34 | 84–99 |
  | AMF | 26–34 | 21–83 |
  | LWF | 35–47 | 5–20 |
  | RWF | 35–47 | 84–99 |
  | SS | 35–47 | 5–99 |
  | CF | 40–47 | 21–83 |

  While a player is held over open grass, the game's bottom panel shows his role and overall,
  then the role a drop there would give him and his overall at it (run `gameplan2a`). It adds
  nothing where his role stays. The role it previews is not always what the drop does: it
  ignores the line limits, naming SS where the drop was then refused (run `gameplan2b`), and
  it named CB at the keeper's depth outside his widths, where the drop was refused too. The
  editor's held badge reads what a release there does, his own role where it is refused, not
  the game's chip: a badge that named a role the drop then refused would promise a move that
  does not happen.

  The zones bound where a drop finds a role, not where a player may stand: the game's own
  formations put wingers at widths 23 and 81 and 4-3-1-2's AMF at depth 35. The keeper is never
  set down on grass: a held keeper previews no role, and a drop sends him back to his spot (run
  `gameplan2b`). A drop inside the keeper's own zone was not made; the editor refuses it, as
  the game refuses the keeper's rows outside it.
- **The held player's areas** (`FUN_027c3060`): while a player is held, the game lights the
  zone of each position he plays at A or B; his registered position alone lights nothing, and
  a player rated A or B nowhere lights nothing and darkens nothing (run `gameplan2b`). A CB rated
  A at LB and SS and B at RB and CMF lit those four zones and left his own CB zone dark. The
  code reading suggested one zone to a band, the first in the code's order; the run lit LB and
  RB together, so the editor lights every zone he is rated at.

## PES 2014 in the code

`PesVersion` gains `Pes14`, whose engine is the pre-Fox one (PES 2014 is the first pre-Fox
game). It stays out of `PesVersion::ALL`, so the version pickers and the tests that loop over
every version are the same as on main, and main's merges into the branch stay small. Only the
Save editor reaches `Pes14`, by detecting a PES 2014 save; every other tool refuses it.
`PesVersion::from_number` keeps refusing 14, so a settings file, a ruleset's `versions`, a
`.4ccs` file or `PES2014.exe` never names PES 2014: the Save editor follows the opened save's
version, not the version selector. Its own check of the save's version's folder looks for that
version's exe by its exact name (`PES2014.exe`, in any letter case) before parsing names through
`from_number`, since the parse alone never finds PES 2014's, and the pitch's Set PES path
button would stay on every PES 2014 save, its tooltip saying the folder holds no PES 2014 exe.

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
  ruleset's stats, the Ruleset editor's columns) keeps its indices. It is a Team TOML key on
  PES 2014 alone (step 12). The AATF logic lists it among the ability
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
- **The tactics record in Build 2** maps the rest of what the Game Plan sets:
  - The formation goes through `FormationLayout`, which learns two things: how many
    formations a preset holds (one on PES 2014, read into the model's first; the other two
    stay at their defaults), and the first slot whose role is stored (1 on PES 2014, whose
    keeper has a place but no role byte). Each place is two u16s, read as their low bytes,
    since no place passes 104. A text that names a formation (an unsaved-changes row, an AATF
    line) names none on PES 2014, "Preset 1 · J. WALTERS · Position" and not "Preset 1 ·
    Kick-off · …": the game's screen has one formation a preset and gives it no name, so
    "Kick-off" would name a set PES 2014 does not have.
  - The lineup at +0x146 fills `starting_eleven` and `bench_order` (11 and 21 entries, the
    record's 32), each the low byte of its u32. The takers, the captain, the players to join
    the attack and the four switches are plain fields.
  - The Strategy and the eight sliders are a part of the preset that only PES 2014 has
    (`TacticsPreset::game_plan`, `None` on the other versions); Combination Play's three
    slots and its switch are likewise a part of the team's. Each holds the value the game shows, so Support Range is
    translated as the stats' offsets are. Their own part, not the shared sliders, because the
    ranges differ: the shared Support Range and Defensive Line run from 1 to 10, and every
    reader of them (the Team TOML, the legacy files, the diff, the Gameplan widgets) would
    take PES 2014's 0 to 20 for that.
- **The Gameplan screen in Build 1** was greyed: the Gameplan segment, with a "PES 2014: in
  the next build" tooltip, and a greyed line in place of a team row's tactics editor. Build 2
  opens it (below), and with it the captain, which Build 1 read for the AATF alone: a
  captain-only screen would have been new design for one build's life. The formation pictures
  stay greyed on PES 2014, since they draw the other versions' Gameplan, whose settings a PES
  2014 preset does not hold.
- **The Gameplan screen in Build 2** is drafted in `docs/prototypes/tools/save_editor.js`
  (`tacticsTab14`), on the draft's PES 14 sample: five Akko teams as run `formations1` kept
  them. It keeps the other versions' frame: the preset bar, the pitch, Player assignments, the
  starting eleven and the bench, and Automatic settings. What differs:
  - **Preset tactics** takes the place of the other versions' Formation settings panel. It
    holds the Formation dropdown, the Strategy dropdown and the eight sliders, 0 to 20, in the
    game's order. The dropdown lists the game's 11 formations; one the list does not hold
    is shown by its shape and listed first. The shape is marked "(custom)" when the list has
    an entry of that name with its players placed otherwise (a 4-4-2 with a player moved),
    and "(not in the game's list)" when it has none (the Akko teams' 8-1-1). The game itself
    names a moved 4-4-2 "4-4-2", so "not in the game's list" would be wrong for it. A pick
    writes the
    entry's roles and spots and puts the keeper at depth 3, as the game's pick does; each
    player keeps his place in the lineup. Picking a strategy sets only the sliders the
    game's pick sets, and Custom sets none; moving a slider afterwards keeps the strategy,
    as the game does (run `tactics4`).
  - **The pitch** draws each player as the game's pitch does, a white shirt with his number,
    since PES 2014 shows no portraits there (the game's "Strip Icon" display). Dropping a
    player on another swaps the two, the keeper included, even under a ruleset that allows
    illegal formations; while he is held, the teammate a drop would swap with is marked. A
    drop on open grass is the game's free move ("What the game's code says"): the player
    moves there and takes the role the game's rule gives him, or goes back to his spot where
    the game refuses the drop. A move that kept the old role could write a pair the game
    never writes. A drop of the keeper on grass, and any drop into the keeper's zone, are
    refused. A click opens no player box, since a role is not picked. While a player is held,
    the zones of the positions he plays at A or B light, and nothing darkens for a player rated
    A or B nowhere.
  - **A swap of two starters**, on the pitch or in the lineup list, moves them in the preset
    shown alone, writing what the game's swap writes:
    - with Preset Tactics 1 shown, the two exchange lineup entries, and presets 2 and 3
      exchange the two positions' roles and spots, so each player stands there where he stood
      (run `tactics1`);
    - with Preset Tactics 2 or 3 shown, that preset alone exchanges the two positions' roles
      and spots, and the lineup stays as it is (run `tactics5`).

    Both leave the same players in the same places. The editor writes the game's bytes, so a
    save it edits matches one the game edited.
  - **The pitch's badges** show each starter's overall at his role, by PES 2014's own formula
    ("What the game's code says"), as the game's "Position / Overall Rating" display does and
    as the other versions' badges show theirs. PES 2016's formula does not serve: it reads
    stats PES 2014 does not store, and its tables' values differ.
  - **The second free kick taker** moves to the next roster slot when he becomes the long or
    short free kick taker, as the game does.
  - **Combination play** is a panel of its own: three slots, each a strategy and an area, one
    strategy to an area, with a Clear button. The area list holds the strategy's areas that
    the other two slots leave free. A strategy whose areas they all hold (Cut Inside takes
    only 3 and 5) is greyed in the strategy list, since picking it would leave the slot with
    no area the game could store. An empty slot reads "None Selected", as in the game.
  - **Tooltips** are the game's own help texts, word for word (run `help1`), since most
    managers have never played PES 2014: on each slider's label, each set-piece label and
    Join the attack, each Automatic setting, and Combination play's title. The Formation and
    Strategy dropdowns carry the chosen entry's text, as the manual gives one per choice; a
    formation off the list has none. The area column's tooltip says how the game's area
    screen numbers the areas, in rows from its top: the game has no text for them, and its
    screen does not say which end is whose. The game's own wording is kept where it reads
    oddly ("automatically changes Change Preset Tactics").

    The game's texts are read from the member's install, never shipped with the suite: the
    English language archive's `string/all.str` (`dt21_eng.cpk` on PES 2014), one table
    in the code naming each tooltip's section and number. They are read once, off the UI
    thread, as the portraits are. With no PES folder set, or one that cannot be read, those
    tooltips are absent, and the pitch's button already asks for the folder. The area
    column's tooltip is the suite's own text and always shows. We read rather than bundle
    because the descriptions are Konami's prose, and the suite ships Konami material only
    to make files the games read, while a member editing a save can be expected to have the
    game installed.

    The file is read through the `pes_strings` lib crate, built on main and merged in
    before this slice; its plan on main (`plans/libs/pes_strings.md`) holds the format. Names
    and texts sit in paired sections, `…W` and `…T` (`F8W` and `F8T` for the Game Plan's),
    whose numbers do not always match ("Long Ball" is `F8W` 84, its text `F8T` 46), hence the
    table. The four Automatic settings share one text, `F3T` 93, a block per setting headed by
    its name in angle brackets (`<Offside Trap>`), so their table entries name the block as
    well, and each tooltip shows its own block. Two of the texts carry the game's inline codes:
    `F3T` 93 its emphasis marks (`i1` … `i0`), which a tooltip cannot draw and drops, and
    `F3T` 102, Combination play's, the button code `ll`, which the game draws as LT and the
    tooltip writes "LT". Line breaks are kept.
  - **Automatic settings** gain Combination Play's switch, and Substitution offers all four
    timings.

  Names are the other versions' where the setting is shared (Automatic settings, Preset
  change), and the game's where it is PES 2014's alone. The preset bar's Copy/swap menu copies
  and swaps presets as on the other versions. The Edit menu's Copy and Paste of a team's
  Gameplan, a preset or a formation stay refused, with "Not available on PES 2014 saves":
  they carry the settings as Team TOML, which PES 2014 saves do not offer before step 12;
  that step opens them.
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
- **Every team is shown.** On a PES 2014 save the editor lists every team the save holds, not
  only those with ids 701 to 920 (main's "Only the cup's teams are shown", `save_editor.md`):
  the 4cc ran on PES 2014 in 2014 at real clubs' ids (the autumn 2014 save,
  `EDIT.bin.autumn14`: /gd/ at 136, /hm/ at 1236, 478 teams), and the Akko organizer reads
  those teams. So on PES 2014 every place that reads the cup's teams reads them all: the list,
  All teams, the filter, Live and Multi-team AATF, Copy/swap stats, the comparator, the
  pictures and the roster sheet. The 4cc, VGL and Invitational categories keep their id
  ranges, so a team outside them is listed under All alone; its name is the save's own.
- **Team TOML on PES 2014** (step 12). The Akko organizer brings whole teams over from
  the autumn 2014 save into the Akko save: the players, their looks and the Gameplan, where
  copy and paste carries a player's stats alone. The route is Team TOML, extended, not a
  format of its own: the Save editor's export and import, the Edit menu's Gameplan copy and the
  command line's `export-toml` and `import-toml` already go through it, and its rules already
  cover a field that only some versions store (an optional key; when it is absent, the save's
  value stands).
  - What only PES 2014 stores becomes optional keys, read and written on PES 2014 alone:
    Tenacity, the Punt Kick motion, each preset's Strategy and eight sliders, and Combination
    Play. The playing style's key already takes The Instigator and Solid Player. The 1-to-8
    settings (Form, Injury Resistance, the weak foot) run 1 to 8 in a PES 2014 document.
  - A preset holds one formation on PES 2014, so the document carries each preset's first
    and leaves the other two out.
  - The look parts the editor does not map yet, the face and the hair, travel in the player's
    appearance record, whole: a PES 2014-only key holding its 88 bytes in hex. An import
    writes the record first and the mapped appearance keys after it, so a key edited in the
    document wins. A target player with no appearance
    record takes none of it, and the import names him, since the editor never creates a
    record.
  - A PES 2014 document imports only into a PES 2014 save, and a PES 2014 save takes only a
    PES 2014 document; the refusal names both versions. The conversion between versions has
    no map for these fields, and both saves the organizer uses are PES 2014.
  - The greyed menu items and the command line's refusals for Team TOML go. Patches, diff, the
    teams-list export and Texport stay refused.
- **Gloves 262143 is no gloves model.** A record the game creates on an in-game edit holds
  the hair accessory's colour at white, every bit set, which the Akko DLC reads as gloves
  262143 (the organizer's save: 90 of 91 new records). The game draws bare hands for it, as for
  0, not the DLC's hidden gloves 11 (run `hi3gloves`, 2026-11-22). So the editor reads 262143
  as 0 wherever a gloves ID means a model: the interference check, and the hide presets, where
  Hide default body replaces it with the hide ID as it replaces 0. The stored value is kept as
  it is until an edit writes the field. The ID travels at its full 18 bits, never through a
  `u16`.

## Releases

A build is a GitHub prerelease, tagged `pes14-v<version>`, where `<version>` is the branch's
workspace version: main's version with a `-pes14.<n>` pre-release part (`0.0.2-pes14.1`). Main's
update check reads only `v<semver>` tags, so it never offers a PES 2014 build to a member on
main. The branch's update check reads only `pes14-v<semver>` tags, so a PES 2014 build is never
offered main's next release, which cannot open PES 2014 saves. The branch's `release.yml` runs
on `pes14-v*` tags (GitHub reads the workflow file at the tagged commit, so main's file is
unchanged). A prerelease is never marked Latest, so the repository's front page keeps showing
main's release. Each cut opens `CHANGELOG.md` with the build's own section, above main's: the
app's What's new chapter shows the bundled changelog, which would otherwise present main's
last release as the build's news.

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
on main. Main is merged into the branch, never rebased onto it, and only when the branch
needs something from it: a fix that reaches a PES 2014 player, or code a step builds on. The
merge's message names what it brings, and a single small fix may be cherry-picked instead. We
do not merge main routinely, because each merge resolves again the code both branches change
(`ops/fpc.rs`, the Appearance page), while most of main's work never reaches a PES 2014
player, who uses only the Save editor.

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
   - the Team menu's boots and gloves dialog on PES 2014: the base game stores no gloves id,
     so the dialog sets the boots alone until step 10, and refuses when one of the team's
     players has no appearance record.
5. The Akko VII ruleset (`akko7.rhai`, its logo and page); `/4ccg/` in `.akko7v2` passes as it
   does in the fork's fixed build.
6. Releases: the `pes14-v` tag prefix in the update check and `release.yml`. The version
   `0.0.2-pes14.1` is set at the cut, in step 7.
7. Build 1's check: `/4ccg/` edited in the build, saved, looked at in game in Edit mode. The
   built window is captured where the branch changed a look: Ability stats with Tenacity's
   row, on PES 2014 and on a later version, where the row is greyed, and the Ruleset editor's
   stat grid with its Tenacity column. Then the release.
8. Build 2: the Gameplan screen. A position changes by picking one of the formation list's
   entries, by swapping two players, or by moving one to open grass, which sets his role by
   itself (runs `tactics1`, `tactics6`): a role is shown, never picked. So the screen is the
   game's own Game Plan, not the other versions' pitch editor.
   - Run `tactics3` (in game): done, pinned in "What is known about the save".
   - Run `formations1` (in game): done, pinned in "What is known about the save".
   - The draft (the lead): done, "The Gameplan screen in Build 2" above.
   - Run `tactics4` (in game): done, pinned in "What is known about the save".
   - Run `help1` (in game): done, read in "What is known about the save".
   - Run `tactics6` (in game): done, pinned in "What is known about the save".
   - Run `tactics5` (in game): done, pinned in "What is known about the save".
   - The game's code, read statically (Ghidra, as PES 2016's was): done (2026-11-21), in
     "What the game's code says".
   - Run `gameplan2` (in game): the inferred parts of "What the game's code says".
     - `gameplan2a`: the "Position / Overall Rating" display for players whose abilities
       differ, against the formula's numbers. Done (2026-11-21), each number the formula's.
     - `gameplan2b`: free moves saved and read back, and the held player's areas. Done
       (2026-11-21): the free move is the rule's at all 13 drops; the areas light every A or B
       zone, not one to a band.
   - Slice 1, the record's fields in the schema and the model: done.
   - Slice 2a, the screen opens with the Preset tactics panel: done.
   - Slice 2b, the pitch's PES 2014 rules (shirts, swaps): done.
   - Slice 2c: Combination play, Automatic settings and the tooltips; the changes rows name
     a Strategy as the dropdown does, and show Support Range as the game does. Done
     (2026-11-21).
     - 2c-1, Combination play, its switch and the changes rows: done (`64468241`), seen in
       the built window beside the draft.
     - 2c-2, the game's help texts as tooltips: done (`5fd1a8ef`); in the built window with
       `E:\PES2014` set, each tooltip hovered held the draft's words.
     - 2c-3, the review fixes of both: done (merge `44b1b8d5`, then `ad7ad4a3`).
       From 2c-1's review, a test that a wheel notch skips a greyed strategy, three rules
       made crate-private and the module's doc; from 2c-2's, more tooltip placements tested
       from the screen and a plainer closure; and three comments the comment check flags.
   - Slice 2d: free moves and the overall on the pitch, from "What the game's code says",
     each sub-slice reworked if run `gameplan2` contradicts it:
     - 2d-1, the overall on the pitch's badges: done; seen in the release build (2026-11-21),
       `/3/` picked to 4-4-2 on a copy of `tactics6`, every badge the draft's;
     - 2d-2, free moves and the held player's areas: done (`2ebe4fd4`, SWE-2 round S1f's test
       fixes `ed08437b`); seen in the release build (2026-11-21) on run `gameplan2b`'s test
       save: `/3/`'s #2 held over midfield read CMF 70 and lit what the game's own capture
       lights (CMF and SS above, the LB and RB strips, not his registered CB), and his release
       left him there as CMF;
     - 2d-3, the formation's name left out of the unsaved-changes rows on PES 2014 (seen in
       2d-1's look: "Preset 1 · Kick-off · …"): done (`9dbdd296`), its SWE-2 round clean.
       The AATF's illegal-formation line and the paste notice name a formation too, but never
       run on a PES 2014 save (the check skips PES 2014; the copy targets are greyed there).
   - The command line refuses the commands out of scope on a PES 2014 save ("Not in scope"):
     done (`422dead7`), each of the five commands with its test.
   - Later: the pitch's shirts in the team's kit colours and icon pattern, as PES 2015's
     Game Plan kit icons (patterns 0 to 23), read from the kit configs in the download
     folder.
9. Build 2's check, as Build 1's: `/4ccg/`'s Gameplan edited in the build (a formation picked
   from the list, two starters swapped, one moved to open grass, a starter swapped with a
   bench player, a slider, Combination play, an automatic setting and a set-piece taker),
   saved, and read on the game's own Game Plan in Edit mode: done (run `build2`, 2026-11-21,
   kept as `build2`). On the Akko save the game showed each of the eight edits as set:
   8-1-1 to 4-4-2, #66 and #8 swapped, #99 moved below the centre circle as CMF, bench #77
   starting at CF, Defensive Line 7, Combination play row 1 in Area 5, Offside Trap on, and
   #24 taking penalties. After the free move the game names the shape "4-4-2", which the
   build now marks "4-4-2 (custom)". Then step 10, then the release.
10. The gloves id on PES 2014 (the Akko DLC's, through the three colour fields) and the team
    name flags: done 2026-11-21 (merges `1637d105`, `0a53d5e9`). Checked in Edit mode with the
    DLC: a goalkeeper given gloves 11 in the build shows no hands (run `step10`), and a team
    renamed in the build shows its new name (run `flags`). Released as `0.0.2-pes14.2`
    (2026-11-21).
11. After the release, the skin colour and the iris colour on PES 2014, which the fork
    edits and the build greys: mapped at 0x0C:0 (3 bits, stored less 1) and 0x34:4 (4 bits) in the appearance
    record, so the Appearance page's two rows work as on the other versions. The game's
    own pickers cannot pin a stored value: step 10's run found the Skin Colour and Eyes
    palettes open on their first entry whatever is stored (skin 7, 3 and 0; iris 0, 2, 5
    and 1 all opened on entry 1). The part menu's model shows the player as stored, so the
    check is there: a standard-body player given each stored skin 0 to 7 in the build, his
    arms' colour on the model set beside the six swatches' players, and likewise the eyes
    for the iris. The FPC entries then write skin 7 on PES 2014 only if stored 6 or 7 turns
    out to be a custom skin. A player with no appearance record is refused, as for the
    boots.
12. Team TOML on PES 2014 (the bullet "Team TOML on PES 2014", above), in Build 3, whose
    release waits for it. Checked with the autumn 2014 save's /gd/ (team 136): exported,
    imported onto an Akko team, and exported again from the Akko save. The two documents are equal but for `[team] id`, and any other
    difference is listed and explained. Then the imported team is looked at in game in Edit
    mode: names, looks and the Game Plan.
    Slices, SWE-2, in this order:
    - a: PES 2014 documents: `pes_version = 14`, the 1-to-8 ranges, one formation a preset,
      and the refusal between PES 2014 and the other versions;
    - b: Tenacity and the Punt Kick motion;
    - c: each preset's Strategy and sliders, and Combination Play;
    - d: the appearance record;
    - e: the Save editor's and the command line's refusals lifted.
