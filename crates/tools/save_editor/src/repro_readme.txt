Save editor

To list what changed between the save as opened and as written, player by player, use the
file names under inputs/ and outputs/:

    UTW save-editor diff inputs/0-EDIT00000000 outputs/0-EDIT00000000

Edits the member had not written yet are in the save under state/save-editor/: give its
path in place of the one under outputs/. A save under outputs/ is the last one written to
its path, so a member who never saved has none there.

diff names a field only when it changed. It compares only the players of cup teams (IDs 701
to 920): none of a team's own fields, so neither its tactics nor its roster, the players'
shirt numbers included. To see a team's fields and its players', including those that did
not change (an edit flag, say), write the team to a TOML file from any of the saves:

    UTW save-editor export-toml --team 701 -o before.toml inputs/0-EDIT00000000
