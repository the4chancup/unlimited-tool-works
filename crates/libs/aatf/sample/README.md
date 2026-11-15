# The sample team

`team.toml` is the team load step 3 runs a rules file's `check_team` on (`spec/aatf_rules.md`
"Loading and validation"): what it reports does not matter, only that the logic and the custom
checks run on a real team without a script error.

It is a Team TOML document of team 714 of `2026 4chan Summer Cup Save V3.rar` (the save's
`EDIT.bin`, PES 15; the maintainer's `C:\Data\4cc\Saves`), written on 2026-10-12 with
`pes_savefile`'s `TeamToml::from_team` (rewritten 2026-10-13, when the model gained PES 15's auto
flags and every version's man marking; regenerated 2026-10-14 for the `playable` ratings'
scale and order, stored values unchanged) and then anonymized: the team's name and short name are
`Sample` and `SMP`, its id, manager, stadium, colors and kit slots are left out, and each player
is renamed `Player NN` (shirt name `PLAYER NN`, NN the roster slot) and carries no appearance,
edit flags, boots, gloves, base copy or nationality. None of those is read by a ruleset.

Checked when it was written: the document, parsed and applied onto bare records (one roster slot
per player key), gives every player's age, height, weight, stats, positions and skills, and the
team's tactics, equal to the save's, with no import notes.
