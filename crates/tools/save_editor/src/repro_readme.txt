Save editor

To list what changed between the save as opened and as written, player by player, use the
file names under inputs/ and outputs/:

    UTW save-editor diff inputs/0-EDIT00000000 outputs/0-EDIT00000000

Edits the member had not written yet are in the save under state/save-editor/, as it stood
when the report was saved: give its path in place of the one under outputs/, which holds
only the last save written, if any.

diff names a field only when it changed. It compares only the players of cup teams (IDs 701
to 920), and none of a team's own fields. To see every stored field of a team, including a
field that did not change (an edit flag, say) and the team's own fields, write it as a Team
TOML from either save:

    UTW save-editor export-toml --team 701 -o before.toml inputs/0-EDIT00000000
