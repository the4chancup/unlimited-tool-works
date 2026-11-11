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

## The 0.0.2 Save editor report

- **The tester's files.** The PES 2019 in-game run (worklog 0.0.2-se19) reproduced the
  tester's placeholders (a name edit on an unflagged player draws PLACEHOLDER) but not their
  default stats: typed stats showed with every flag clear. Can the tester send the save before
  and after their edit, with what they changed? A byte diff of the two says what the write
  did, which no reconstruction of their steps can.
- **Seasonal logos on an uncompressed kit.** On PES 2019 an uncompressed 4096 main kit
  (worklog 0.0.2b) loads and draws in Edit mode with no crash, but no seasonal logo showed
  (`.tmp/0_0_2/ingame/poc/s42_egg_body.png`). Does PES 2019 place the logos in Edit mode at
  all, or does this need a match to show whether Fox games write them into an uncompressed
  kit as PES 15–17 do? The player also draws see-through in that capture; the export's
  textures are a noise test pattern with alpha about 68, which may be all it is.

## The PES 2017 Texport's empty starting lineup

- **Which lost field empties the lineup.** A tester's PES 2017 Texport from the save editor
  (0.0.2-poc.1) imports and shows its players in Edit, but the kick-off screen's "Starting
  Lineup" lists nobody (worklog issue of 2026-11-11). Four files in
  `.tmp/report17/proof/<variant>/TEXPORT00000004`, each built from the game's own
  `TEXPORT00000002` (/4cccc/, team 772): `A_current_writer` (rebuilt by the editor's writer),
  `B_manager_id` (A with the team's manager id restored), `C_player_id_copy` (A with every
  player's second id copy restored), `D_game_file_resalted` (the game's file, control). For
  each: import it onto 772, pick /4cccc/ for a match, and say whether the kick-off screen lists
  the starting eleven. The fix copies every field anyway; the answer says whether A reproduces
  the report and which field the game needs.

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
