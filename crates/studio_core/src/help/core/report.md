When UTW does something wrong, **Report a problem** (the `⚠` while the
sidebar is folded) saves a file to post with your report, so whoever looks
into it can repeat what you did. It sits in the sidebar just above
**Help (WIP)** and opens a dialog. Nothing is sent: you post the file
yourself, wherever you usually report problems.

For now only the Save editor records what you do in it; the other tools join
as they are built. The file holds:

- what you write in the dialog's box;
- every step you took in the Save editor since the window opened: opening a
  save, edits, imports and saves;
- the files it opened, as they were before any change, the files it wrote,
  and the open save as it is now, with your edits;
- `utw.log`, UTW's version, your settings and the PES installs found.

In the box, write what you did, what you expected and what you got, and
anything done outside UTW, in the game or in another tool. The words stay
there until the window closes, so you can close the dialog and finish them
later.

UTW also keeps a record of each session, from the moment it opens until
it closes, with the saves opened and written in it. The five sessions
before the current one stay in UTW's data location; no setting turns
this off. When the problem began before UTW was last opened (it
crashed, or you tried a save in the game after closing UTW), tick
**Include earlier sessions** under the list in the dialog: the file
then holds those sessions too.

**Save…** asks where to put the file, which is named by the date and time
(`utw-report-2026-11-15-1030.zip`). The file holds your whole save, and your
user folder is written as `~` in its paths.
