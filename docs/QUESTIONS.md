# Open questions for the maintainer

The one list of what only the maintainer can answer: in-game behavior, cup practice, a
preference between options the plan does not pick. The lead adds an entry when it parks a
question (`AGENTS.md` "When the plan has gaps") and removes it when answered, logging the
answer where it belongs (a plan edit and a `DECISIONS.md` entry, or a worklog note). Lead and
sidekick work in progress stays in `WORKLOG.md` "Issues"; this file holds nothing the lead can
settle itself. Entries are grouped by what an answer unblocks, newest last within a group; each
names where the question came from so the context can be read there.

## Before the release

- **The final pass over the built app's UI** (worklog 0.0.1f1c). Any change it asks for is
  drafted first, looked at in the drafts, then built.
  - **The candidate:** branch `release/0.0.1-candidate` at `bddd0027`, unpacked at
    `C:\Data\4cc\Tools_Mine\4cc-studio-temp\candidate-0.0.1c\unlimited-tool-works-0.0.1\`
    (`COMMIT.txt` beside the folder names its commit). New since 0.0.1b, each checked by the
    lead in this build: the team file panel (import and export, a PES 2017 Texport without
    an extension), the list under "• unsaved changes", the player list's and the pitch's
    rounded corners, the light theme's greys and warning orange, the sidebar's own icons,
    the running-game dialog's text, the import notes in words (worklog 8.9k5, 8.31, 8.9m,
    8.14, 8.29d).
  - **Already measured,** so not worth reporting: every screen the drafts and the build share
    matches the drafts within 1 point (2 where needed), in both themes. What still differs
    does so by ruling: the rail's order, a combo's arrow, the checkboxes' hit areas.
  - **Then:** the confirmation that freezes logic version 1's hash (0.0.1f2).
- **PES 2017's pitch in the drafts** (worklog cv9): the Save editor's team view with the
  sidebar on PES 2017 (`#save_editor/team?save=pes17&pes=2017&selteam=702`) now paints PES
  2017's own formation screen (seven bands, its colours and grain, its smaller boxes, arcs
  and circle), and a dragged outfielder stops at y 4. A look before the app follows it;
  side by side with the texture and the screenshot: `.tmp/0_0_1cv/pitch17/draft_pitch.png`.

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
`.tmp/13_0a/refs_audit.md`); none blocks the tool's first slice:

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
