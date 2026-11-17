Save editor

To list what changed between the save as opened and as written, player by player, use the
file names under inputs/ and outputs/:

    UTW save-editor diff inputs/0-EDIT00000000 outputs/0-EDIT00000000

diff names a field only when it changed, and compares players only. To see every stored
field of a team, a field that did not change included (an edit flag, say), and the team's
own fields, write it as a Team TOML from either save:

    UTW save-editor export-toml --team 701 -o before.toml inputs/0-EDIT00000000
