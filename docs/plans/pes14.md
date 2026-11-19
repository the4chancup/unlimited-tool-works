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
- teams: name, roster, shirt numbers, team colours. The team kits (the game's Strip menu) are
  kept exactly as read: the editor has no kit design screen on any version;
- the tactics record, mapped whole so the editor reads and writes it exactly and the AATF
  knows the captain (below);
- the AATF check against the Akko VII ruleset, embedded as `crates/libs/aatf/rulesets/akko7.rhai`
  like the other events' rulesets. Its logo is the event's
  (`C:\Data\Pictures\4ccpics\Akko_League_VII_Logo_1.png`), scaled to 84 pixels high (137 × 84)
  as the other rulesets' logos are; its page is
  `https://implyingrigged.info/wiki/Akko_League_VII`. No game has a list of trick cards: a cup's
  organizer names the cards that hardly change play, which are then free or have their own
  limit. Akko VII's page sets the free tricks per medal but names no cards, so its ruleset's
  `card_groups.tricks` is the official 4cc list (`autumn26.rhai`'s), limited to the cards PES
  2014 has;
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

**Not in scope:** Team TOML export and import, patches, diff, the teams-list export and
Texport, unless one works on PES 2014 with no work beyond a test. The rest stay greyed out for
PES 2014, with a tooltip saying so.

## Re-derived, not copied

The fork is a hint for where things are, never the source of how they work. Its reported
failures on PES 2014 (Akko managers, 2026-11):

- the captain reads as the roster's first player, while the game keeps its own captain;
- the whole appearance page reads back as defaults, so physique, goal celebrations and the
  rest are not saved;
- the player's strip style reads back as a default;
- team colours read back as 0/0/0.

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

- **Edit flags are per section.** Bits 2 to 7 of byte 0x28 and bits 1 to 7 of byte 0x29 of a
  player record flag which parts of the player were edited; the game shows base data for every
  part whose flag is clear. The organizer's merge set only bit 0x29:1 and the game showed
  placeholder names and flat stats; setting the rest fixed it (checked in game by the
  organizer). Bit 0x29:0 is left alone: it may mark a base copy.
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
- Two more tables sit in the payload, unread by the fork: a second team table near 0x374C44
  (team ID at +0, name near +0x78) and a player table keyed by player ID right after the
  appearance table, at 0x2DABAC. The vanilla save leaves both empty.

## What is mapped in game

Each item ends with its offsets, widths and value orders written into "What is known about
the save" above.

- **Edit flags:** which bit of 0x28 and 0x29 belongs to which of the ten parts of the game's
  player menu, by editing one part per player and saving.
- **The tactics record:** the captain, and every run the codec writes (formations, man
  marking, presets), so a record reads and writes back unchanged.
- **The player record:** every field the fork decodes, checked against the game; Tenacity;
  goal celebrations; the playstyle list (18 styles) and the motion tops.
- **The cards:** the game's card list, slot by slot, with each card's name and description as
  the game shows them. The fork calls one card Precise Touch where later games have Cut Behind
  & Turn: the same card under an older name is a trick, a different card is not.
- **The appearance record:** face, hair, physique, boots, gloves, accessories and the strip
  style.
- **The team record:** team colours, in the bytes between +0x04 and +0x2B that vary between
  teams.

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
  `Scheme::Pes15` payload of 4 726 440 bytes is PES 2014. A PES 2014 `TEXPORTnn.bin` is then
  named PES 2014 too, and refused as a Texport with no layout.
- **Schema.** `schema/pes14.rs`, hand-written (the other versions' tables are generated from
  measurements PES 2014 does not have), with the five sections above. Each per-version table
  gains a PES 2014 arm: playstyles (18, measured), motion tops and face-type caps (measured),
  advanced instructions (none), Texport (none), the stat-name renames, the nations list
  (an eighth column), the save's folder and file name, and the pitch the formation screen
  draws (PES 2017's, the fallback for unmeasured versions).
- **Appearance records.** On PES 2014 only a player whose look was edited, or who was created
  in the save, has an appearance record: the vanilla save has none for its 11998 players, the
  Akko saves one for each of their 5060. A player without one reads no appearance fields and
  writes none. On PES 2015 and 2016 every player has one, and a missing record still refuses
  the save.
- **Tenacity** is a PES 2014 stat no other version has. It gets its own `Option` stat, its
  `PlayerField` and its AATF stat name. On another version's save its greyed box's tooltip
  reads "PES 2014 only", since the "PES NN+" wording would claim later versions store it.
- **Edit flags.** On PES 2014 the game shows base data for every part of a player whose flag is
  clear, so an edit the flags do not record is lost in game. Each player the editor changes on
  PES 2014 gets the flags of the parts it changed; which bit is which part is pinned in game
  (the game's player menu has ten parts: Player Name, Basic Settings, Position, Ability
  settings, Player Index, Face, Hairstyle, Physique, Accessories, Motion). On the other versions
  an ordinary edit sets no flag, as on main.
- **The Gameplan screen in Build 1.** The Gameplan segment is greyed with a "PES 2014: in the
  next build" tooltip; the import panel's Gameplan section and the Preset and Formation copy
  targets go with it. The captain is read for the AATF and kept as the game set it; it is
  shown and edited on the Gameplan screen, as on the other versions, from Build 2. A
  captain-only screen for Build 1 would be new design for one build's life.

## Releases

A build is a GitHub prerelease, tagged `pes14-v<version>`, where `<version>` is the branch's
workspace version: main's version with a `-pes14.<n>` pre-release part (`0.0.2-pes14.1`). Main's
update check reads only `v<semver>` tags, so it never offers a PES 2014 build to a member on
main. The branch's update check reads only `pes14-v<semver>` tags, so a PES 2014 build is never
offered main's next release, which cannot open PES 2014 saves. The branch's `release.yml` runs
on `pes14-v*` tags (GitHub reads the workflow file at the tagged commit, so main's file is
unchanged). A prerelease is never marked Latest, so the repository's front page keeps showing
main's release.

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
  appearance settings and colours. Team 857's record does not hold those two colours as plain
  bytes, so where PES 2014 keeps team colours is still to be found.
- **In game.** PES 2014 is installed at `E:\PES2014`, driven by `scripts/ingame/game.py 14`
  (a pad, OBS shots, no keyboard focus). The mapping saves are made there, and Build 1 ends
  with the edited `/4ccg/` looked at in Edit mode. Where the editable players are:
  - in the vanilla save, the leagues' squads are "Unregistered" dummy slots whose edits the
    game drops; created players (Edit > Players > Create Player, a name required) keep theirs;
  - in the Akko saves, the first league (English League) holds the 4cc teams, blank (23
    PLACEHOLDER players each). Akko is an invitational, so the teams playing it, `/4ccg/`
    among them, are in one of the lower leagues, under ids of their own: `/4ccg/` is 857
    there, while the save's team 784, the 4cc teams list's id for `/4ccg/`, is a blank 4cc team
    named "VGL 4".

## Process

Second-class work, lighter than main's: the implementation runs on SWE-2 High, with a SWE-2
Flash High twin on the same brief for the trial's tally; reviews are SWE-2 rounds only; no
mutation runs. The per-commit gates (`docs/CONTRIBUTING.md` "Testing and verification") run as
on main. Main is merged into the branch, never rebased onto it.

## Steps

1. The plan and `game.py`'s PES 2014 entry.
2. The mapping above, in game (the lead).
3. Open and save: `Pes14`, detection, the save's folder, the fixture, `schema/pes14.rs`; the
   fixture and the Akko saves written back byte for byte. No screen work.
4. The Save editor on PES 2014: Tenacity, the labels, playstyles, motions and nations, the
   edit flags, the Gameplan screen and the out-of-scope features greyed.
5. The Akko VII ruleset (`akko7.rhai`, its logo and page); `/4ccg/` in `.akko7v2` passes as it
   does in the fork's fixed build.
6. Releases: the `pes14-v` tag prefix in the update check and `release.yml`, the version
   `0.0.2-pes14.1`.
7. Build 1's check: `/4ccg/` edited in the build, saved, looked at in game in Edit mode; then
   the release.
8. Build 2: the Gameplan screen.
