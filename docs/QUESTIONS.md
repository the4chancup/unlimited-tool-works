# Open questions for the maintainer

The one list of what only the maintainer can answer: in-game behavior, cup practice, a
preference between options the plan does not pick. The lead adds an entry when it parks a
question (`AGENTS.md` "When the plan has gaps") and removes it when answered, logging the
answer where it belongs (a plan edit and a `DECISIONS.md` entry, or a worklog note). Lead and
sidekick work in progress stays in `WORKLOG.md` "Issues"; this file holds nothing the lead can
settle itself. Entries are grouped by what an answer unblocks, newest last within a group; each
names where the question came from so the context can be read there.

## Cup practice

The Music player's three below come from reading Rigdio v2.2.0's controls (worklog 10.0i,
`plans/music_player.md` "Playback semantics"):

- **Chaoshorn and the score.** Rigdio's Chaoshorn presses every goalhorn button, so it scores
  one goal for each player and one for the Standard Goalhorn: the scoreboard jumps by the
  roster size plus one, and each player's horn is chosen as for that goal. Should Studio's
  Chaoshorn score those goals, or play each button's horn as if for a goal and leave the score
  as it was? (Is Chaoshorn pressed while the score still matters?)
- **A button after its song ends.** When a horn with `end stop`, or a victory anthem, plays to
  its end, Rigdio's button stays pressed, and the next press only releases it (no goal, no
  sound); the press after scores and plays. Do streamers rely on that extra press, so that a
  button releasing itself at the song's end would turn their habitual double press into two
  goals? Or should the button release itself when the song ends, so the next press is a goal?
- **title.log and chants.** Rigdio writes title.log for anthems, victory anthems and
  goalhorns, never for a chant. Should the OBS overlay show a chant's title too, or stay on
  horns and anthems as it always has?

The Refs arranger's three below come from its readiness audit (worklog 13.0a,
`.tmp/phase_13/13_0a/refs_audit.md`); none blocks the tool's first slice:

- **FoxDen without a lists file.** FoxDen's referee module, as shipped, forces five distinct
  referees drawn uniformly from all 35 slots at boot and after every match
  (`Tools_4cc/FoxDen/scripts/03_refmod.lua`, `randomize`), so a FoxDen player never sees the
  game's own draw. Is FoxDen expected on every viewer's install, or on the streamers' only?
  And with no `ref_lists.txt`, should the reading script keep that uniform draw, or turn
  forcing off so the game draws over the arranger's weighted fill (`plans/refs_arranger.md`
  "Who gets the file")? The second is a change the hook's author would make.
- **Which game mode do Fox cup matches run in?** FoxDen's reading of PES 2018 has Exhibition
  modes draw five referees uniformly from slots 1–10, 16–25 and 31–35, and competition, league
  and Master League modes from league-driven pools (`plans/refs_arranger.md` "Background").
  Also: which version(s) were the 125 matches of the PES 17–21 table logged on, and what is
  slot 24's real count (marked uncertain)?
- **Which PES version was the 15/16 pattern table measured on?** And is there a log from the
  other one, to say whether the two share it?

## After the release

- **Substitution's choices in PES 2018, 2019 and 2020** (worklog 0.0.1y7c; you: "we'll check
  in detail in the future"). The Tactics editor's Automatic settings offers Off, Very late,
  Flexible and Very early on those three, taken from PES 2017's and 2021's screens (both list
  the four) rather than looked at (`plans/save_editor.md` "Tactics editor", the sentence on
  Substitution). A look at each game's Game Plan > Automatic settings > Substitution confirms
  or corrects it; the three are on G:.
- **Playing attitude on PES 2019** (you, 2026-11-01: not editable on 19). The Save editor shows
  it from PES 2020 only (`plans/save_editor.md` "Editing UI", the Cutscenes group: "star (19+)
  and playing attitude (20+)"): PES 19's schema has no field for it, and the census found no
  bit of the PES 19 player record left uncovered that any save sets (`spec/pes_savefile/model.md`
  "The player record"). Does PES 2019's Edit mode offer a playing attitude on a player? If it
  does, it is a field still to find (changed in game, the save diffed); if not, the plan holds.

## The proof of concept (0.0.2-poc)

All 53 VGL26 exports were upgraded against `EDIT00000000_VGL26` and compiled for PES 2019 on
copies (2026-11-05, `.tmp/0_0_2/poc/all_run.py`, the release binary at `46105675`; tallies
`tally.py`, `missing_ids.py`). With the two rules answered that day (a custom ID no folder
holds becomes the hide ID; a texture nobody supplies is a Warning; worklog 0.0.2-poc1 and
poc2), three Error classes are left (worklog 0.0.2-poc3):

- **/omg/'s boots numbers.** Six players wear boots 2552, 2554, 2556, 2561, 2562 and 2564,
  and the export's boots folders named after them are `k2952 - BANISHED`, `k2954 - BROOL`,
  ... `k2964 - 81514- LET ME DEFEND`: each 400 above the save's number. Under the new rule
  those six upgrade clean with the hide ID 55 and their boots folders (full bodies:
  `body.dds`, `belly.dds`) dropped as unworn leftovers, two Warnings each. Is the VGL26 save
  older than the export here (the cup's own save wearing 2952), and should the upgrader
  refuse instead when the export holds an unworn folder of the same kind as the missing ID,
  since that is likelier a mismatch than a full body in the face folder?
- **/akg/'s `Common/dummy_srm`.** `Common/` holds both `dummy_srm.dds` and `dummy_srm.ftex`,
  so the upgrade's validation refuses it (`texture_stem_conflict`), as a compile would. The
  compiler never looks up a `dummy_` stem (the game substitutes it). Does the game ever read
  a team's own `dummy_` file from the CPK, or can the upgrader drop both as leftovers?
- **`vertex_too_far_from_origin` (89 Errors in 7 exports).** Rerun with both rules (`4e69334c`,
  `.tmp/0_0_2/poc/far_faces.py`), the compile's only Error class left drops 47 players' folders.
  They are mostly two community templates, byte for byte: one `face_high.fmdl` with 739
  vertices past 5000 units and its `hair_high.fmdl` with 16 sit in 29 folders (/domg/, /ink/,
  /vn/'s "BLANK" players...), another face with 1,478 in 13 (/aceg/, /hgg2d/, /lcg/). The
  rule is there for the lag such geometry causes all matchday, and these teams played VGL26
  with them. Are those far vertices the templates' way of hiding parts (and harmless), so
  the rule should be a Warning, or did the cup see the lag, so the Error stands?
