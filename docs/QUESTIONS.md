# Open questions for the maintainer

The one list of what only the maintainer can answer: in-game behavior, cup practice, a
preference between options the plan does not pick. The lead adds an entry when it parks a
question (`AGENTS.md` "When the plan has gaps") and removes it when answered, logging the
answer where it belongs (a plan edit and a `DECISIONS.md` entry, or a worklog note). Lead and
sidekick work in progress stays in `WORKLOG.md` "Issues"; this file holds nothing the lead can
settle itself. Entries are grouped by what an answer unblocks, newest last within a group; each
names where the question came from so the context can be read there.

## Before the release

- **How the update check shows itself** (worklog 0.0.1f1g9; the draft's proposal is uncommitted
  in `index.html`, renders `.tmp/vet/update/bar.html` and `dialog.html`, or tick "Update
  available" in the draft's controls).
  - **Where the offer appears.** The plans disagree: `plans/core/distribution.md` "Self-update"
    step 2 opens a dialog when a newer release is found; `plans/core/gui.md` "Status bar" makes
    it the `update_available` condition with an Install action. The lead's proposal: no dialog
    opens by itself; the status bar reads `Update available: v0.0.2` with a **Details** button,
    which opens a dialog with "Unlimited Tool Works v0.0.2 pre-alpha is out. You have v0.0.1
    pre-alpha.", the release notes as Markdown in a scrolling box, and **Open release page**,
    **Skip this version**, **Remind me later**. An update asks for no decision now, and a
    dialog at the start would land on whatever the member opens first.
  - **The draft's version label.** The draft's status bar reads `v0.1.0`, its About `v0.0.1
    pre-alpha` (worklog 8.26). The lead would make the bar `v0.0.1`.
  - **The lead's choices, to confirm or overrule:** the check reads GitHub's list of releases
    and takes the highest version, not `releases/latest`, which skips a release marked
    Pre-release; tags are `v<version>` (CI sets them); the window checks, the command line
    does not in 0.0.1 (a script never waits on the network); a failed check (offline,
    rate-limited) shows nothing and writes one log line; Skip this version stores the version
    in `skipped_version`, Remind me later closes until the next check; the two Settings rows
    show from 0.0.1 on (reversing DECISIONS 2026-10-25, "The update check's settings rows wait
    for the updater").
- **The comparator's head** (DECISIONS 2026-11-02, "Live AATF's heads in two tones"). The
  hyphens went into the draft's shared head function, so the draft's save comparator now reads
  `Player 71808 - 8 - PARTY HARD` too, while the app keeps the comparator's `Player 71808  8
  PARTY HARD` (it is not an AATF list). Hyphens in the comparator as well, or out of its draft?
- **When Live AATF's Copy to clipboard greys out.** The plan and the app: while the list shows
  no row. The draft: only when the team has no rows at all, hidden suggestions counted, so with
  suggestions off and nothing else listed the button copies a header line alone. The lead
  would bring the draft to the plan.
- **CI for GitHub releases** (worklog 0.0.1f1g7; it overrides DECISIONS 2026-11-01, "CI
  release builds right after 0.0.1's cut, Windows first"):
  - Does 0.0.1 itself go out through CI? The lead's proposal: the tag `v0.0.1` builds both
    bundles and attaches them to a draft GitHub release; your look is on the Windows `.7z`
    downloaded from it, then you publish. A local exe and CI's are never byte-identical (the
    linker's timestamps), so the checked binary is then the one members get, and the update
    check has a release to read.
  - Linux's archive: `.tar.gz` (keeps the binary's executable flag, what Linux users expect) or
    `.7z` as on Windows (often loses the flag on extraction)? The lead: `.tar.gz`.
  - The Linux look in WSL's Ubuntu with WSLg on this PC, in place of "a real machine"; built
    on Ubuntu 22.04 so the binary runs on older distributions.
  - The dry run: may the lead push a branch (`ci/release-dry-run`) and start the workflow by
    hand, so it uploads both bundles as workflow artifacts only (no tag, no release)?
- **The release profile** (worklog 0.0.1f1g6; `spec/core/distribution.md` "Size budget": "The
  release profile stays the default"). `codegen-units = 1` measured 38,724,608 bytes against
  41,491,456, 2.64 MiB (6.7%) smaller; one cold build took 206 s, its slowdown against the
  default not measured. Take it for 0.0.1i? Beside it, without a ruling needed unless you say
  otherwise: egui's `default_fonts` off, dropping its unused `Ubuntu-Light.ttf` (353 KiB).
- **cv39's fix in 0.0.1** (worklog 0.0.1f5; you, 2026-11-01: "it can wait"). Its test's race
  (`the_last_end_follows_the_first_...`, a press held in the Multi-team AATF dialog while its
  check lands) failed once in f1g's checks and passed alone three times; it can fail again in
  0.0.1i's gates and mutation runs. Pulling `99c9a36a` into f1g (one line of code and its test)
  stops that and puts the fix in 0.0.1 itself, so f5's separate exe for DBC's organizer
  before 2026-11-18 is no longer needed. Pull it in?
- **The final pass over the built app's UI** (worklog 0.0.1f1c). Any change it asks for is
  drafted first, looked at in the drafts, then built.
  - **The candidate:** 0.0.1i, staged once f1g's loop ends (worklog 0.0.1f1g); its folder and
    commit land here then. New since 0.0.1g: Tactics rows drag from their names (f1e), Up and
    Down walk the player list, the import's Stats, Aesthetics and Gameplan boxes, Direct3D 12
    on Windows; the dragged row's outline (f1f), two copies of the exe at once, Ctrl+C and
    Ctrl+V on team rows and the Tactics selectors, the VGL captain bonus's count, the Stronger
    foot colours, undo runs, Tab + Space; then f1g: the Fielded row's tooltip (P49), the
    shortcuts after Tab + Space (P50), Live AATF's heads and copy button, Invert value
    scrolling, PgUp/PgDn, and the update check if it lands.
  - **Yours in the same pass** (worklog 8.38, never checked in the built window): the dialog
    the Save editor shows when you save while PES 2017 runs; the AATF screen against its PES
    2017 sample; the list a click on the toolbar's `• unsaved changes` opens (8.31). Also the
    look outside the app's own windows (worklog 0.0.1f1c): the taskbar button, Alt+Tab and
    Explorer's icon on the exe.
  - **The notebook** (worklog 0.0.1f1e4): `run_gpu_check.bat`'s default start on 0.0.1i opens
    the window on the Intel HD 620, where every start of 0.0.1g crashed in the Vulkan driver.
  - **Already measured,** so not worth reporting: every screen the drafts and the build share
    matches the drafts within 1 point (2 where needed), in both themes. What still differs
    does so by ruling: the rail's order, a combo's arrow, the checkboxes' hit areas.
  - **Then:** the confirmation that freezes logic version 1's hash (0.0.1f2).

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
