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
- **What a Shift+click on the taskbar button does today** (worklog 0.0.2-inst, your ask of
  2026-11-03). The program sets no application ID on Windows and has no single-instance
  guard (two windows on one folder ran in Update now's look), so Windows' own Shift+click
  should already start a second window, and nothing in the code says why it would not. The
  lead's input goes only into the program's own window, so it cannot click the taskbar to
  see. What happens when you Shift+click the button (nothing, a second window, an error),
  and was the program started from the exe, a shortcut or a `.bat` when you saw it?

## 0.0.2's Update now on Linux

- **Where ST-UPD-04's Linux half runs** (worklog 0.0.2-upd; `plans/core/gui.md` ST-UPD-04).
  The scenario clicks Update now and Restart now in the Linux bundle, so it needs a Linux
  desktop and a pair of Linux bundles; Windows' half passed. Neither exists here yet:
  - WSL does not start on this PC (2026-11-12: `ERROR_NO_SYSTEM_RESOURCES` at `CreateVm`,
    three tries, the last with no build running; about 21 of the 32 GiB of RAM were in use
    by the open programs). It has a display (WSLg). Whether a restart of the PC brings it
    back is not known.
  - The VPS has no display, and running a window there means installing a virtual one
    (Xvfb, a software OpenGL, a tool that clicks) on the production host.
  - The bundles: `just release` on Linux needs `cargo-about`, which the VPS lacks (a
    `cargo install` in the build user's home, as `just` and cargo-mutants were), and a
    bundle built there links Debian 13's glibc 2.41, so it may not start on an older
    distribution; the release workflow builds on Ubuntu 22.04, but only from a pushed tag.

  Which of these do you want: WSL once it starts again (after a restart you do), the virtual
  display on the VPS, or running the scenario yourself on a Linux machine? And may
  `cargo-about` be installed on the VPS for the bundles?

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
- **Which kinds the Decompiler must recover from old DLCs** (`plans/decompiler.md` "Without
  one: the heuristic"). From a DLC compiled with recovery manifests every kind comes back
  exactly. A DLC compiled by the old tools has none, and the plan gives each kind a second
  path that works from the game's own files (a team by its ID, the balls from `Ball.bin`,
  the stadiums from their bins, the referees from their slots). That path is the larger half
  of the tool. For which kinds are there old DLCs whose exports are lost and wanted back:
  teams only, or the balls, the stadiums, the billboards and the referees too?
- **The 40 GB this project may hold on the VPS** (`RUNBOOK.md` "The scratch folder"; the
  decision "The VPS free-space floor for a mutation half is 26 GiB"). A mutation half's two
  build copies were measured at 23.6 GiB for `save_editor`, where the scripts assumed about 8.
  The folder may hold 30 GiB when a run starts (`REMOTE_DISK_BUDGET`), so it can reach about
  54 GiB while a half runs; in the measured run it peaked at 35.7. The free-space floor now
  keeps the disk from filling whatever the folder holds. Does the 40 GB limit still matter
  while that floor holds? If it does, the budget drops to about 13 GiB, under one cold build
  cache, and the clone's cache is removed before nearly every run that uses the host.

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
