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

- **Seasonal logos on an uncompressed kit.** On PES 2019 an uncompressed 4096 main kit
  (worklog 0.0.2b) loads and draws in Edit mode with no crash, but no seasonal logo showed
  (`.tmp/0_0_2/ingame/poc/s42_egg_body.png`). Does PES 2019 place the logos in Edit mode at
  all, or does this need a match to show whether Fox games write them into an uncompressed
  kit as PES 15–17 do? The player also draws see-through in that capture; the export's
  textures are a noise test pattern with alpha about 68, which may be all it is.
- **The lit area of a player rated nowhere.** While a starter's marker is held, the Tactics
  pitch lights the areas of the positions his playable list rates A or B and darkens the
  rest; a player whose list is all C (a blank team's placeholder, his registered position
  included) darkens nothing (`plans/save_editor.md` "Tactics editor"; `roster_slot` in
  `save_editor`'s `view/tactics.rs`). In the game's Edit Position screen, does such a
  player get his registered position's area lit, or nothing? If lit, the registered
  position counts as rated here too, as the overall already reads it.
- **The two numbers arriving another way.** On a PES 17 to 21 save under a ruleset without
  illegal formations the toggles of Numbers in attack and Numbers in defence are disabled
  (SE-GUI-71), but a preset pasted or copied from another, a Gameplan pasted from another
  team and an imported Team TOML still write the two bytes, and the AATF check says
  nothing of them. Should those keep the target's numbers, should the check flag a value
  the game's menu could not have set, or is the toggles' lock enough?
- **A team file imported onto another team brings its manager id.** The Save editor
  imports a Team TOML onto the open team, and writes the file's `manager_id` as it is:
  a file exported from team 876 and imported onto team 886 leaves both teams naming
  coach 876 (seen in a member's PES 2021 save, 2026-11-12; in that save every other team's
  manager id is its own team id). A player's base copy id is turned into the target's own
  on the same import; the manager id is not. Does the game mind two teams sharing one
  coach record, and should the import give the target its own manager id when the file's
  equals the file's team id?
- **An X on a row of the unsaved changes list, after a player was moved** (worklog
  0.0.2-undo). The list has one row per field that differs from the file, and each row
  holds the field's old value, so an X that writes that value back (one undo step, the list
  staying open) works for a stat, a name, a slider or a set-piece taker. A reorder is the
  hard case: moving a player up or down the list trades two players' whole records between
  their ids, so the list shows every field of both players as changed, and writing one of
  those rows back would give one player a single value of the other. Three rows of one
  formation spot (position, x, y) and the roster rows of a player moved between teams have
  the same problem on a smaller scale. Which of these is wanted?
  - Rows that only make sense together are undone together: an X on any row of a moved
    player puts both players back as the file has them, and hovering it marks every row it
    takes. Simple to build, but an edit made to either player after the move goes with it.
  - The list learns to show a move as one row ("Place 04 → 07") with the player's real
    edits beside it, each with its own X. This reads best, but the editor must then track
    which record went where through every drag, undo and import, and the close prompt's
    table changes with it.
  - The X is drawn only on rows that stand alone, and a moved player's rows have none (Undo
    is the way back for those).

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

## Sentences the plans and the spec leave open

From the readability pass of 2026-11-12: passages its readers could take two ways, which
neither the code nor another document settles (`.tmp/readable/findings.md`, with each one's
evidence in `fix_resolutions.md` beside it). Each waits only for its tool's phase.

- **The Refs arranger's label for PES 15/16** (`plans/refs_arranger.md` "CLI"). `chances`
  labels its PES 15/16 figures "probability", while "Storage box" says of the same figures
  "Exact for the supplied sample, not a claim about the game's true probabilities". What
  should the label say?
- **A manually started anthem and the kickoff fade** (`plans/music_player.md` "Autopilot",
  "Anthems start manually": "The existing kickoff fade-out may still stop a manually started
  anthem"). The only kickoff fade the plan defines is the goalhorn's, on the first clock start
  after a goal. Does that fade also stop an anthem, or is another fade meant?
- **A formation picture's panel colour** (`plans/save_editor.md` "Tactics editor", formation
  pictures, "The picture.": "The theme's panel colour under the team's name in its colour
  codes' colours, its id and the save's version"). Is the panel colour the whole picture's
  background, or only the heading's?
- **The file-name line for a range of teams** (same section: "the same checkbox, and the
  file-name line, which follows it"). Does the line come after the checkbox, or does it
  change with the checkbox's state?
- **The Comparator's man-marking row** (`plans/save_editor.md` "Comparator (gameplay +
  aesthetics)": "on the row of the own starter who marks (`Preset 1 · Man marking · <his
  name>`): the bare byte read as one of the team's own players."). Does the last clause say
  what the row shows, or what the bare byte must not be read as?
- **A click on an entry of the Export upgrader's report** (`plans/export_upgrader.md` "GUI":
  it "opens the system file browser at that entry (the folder, the file selected where the
  platform can; ...)"). Does a folder entry open that folder and a file entry its folder with
  the file selected, or does every entry open its containing folder with the entry selected?
- **Why the legacy FMDL writer failed on Konami files** (`spec/libs/format_crates.md`
  "`fmdl::model`", the bounding-box item: a group's box is "computed when the source had none,
  which is why the legacy writer failed on Konami files"). What did the legacy writer do with
  a group that had no box?
- **The Texport experiment's files** (`spec/pes_savefile/operations.md` "Texport (read +
  write)": "Four files were built from one game-made Texport of 772", the last bullet "and so
  did a file from the writer that copies the save's records"). Was the writer's file the
  fourth of the four, or a fifth?
- **The collar's alpha test on PES 17** (`spec/team_compiler/blue_port.md` "What does not
  carry over": the in-game test of 2026-10-09 found that a `.mtl` beside a nocloth `.model` is
  read and the collar model then stops drawing, "And a collar drawn with the kit's own
  material has no alpha test"). Did the same test show the second, or is it from elsewhere?
