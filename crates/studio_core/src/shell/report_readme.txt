A problem report saved by Unlimited Tool Works (UTW) through its Report a problem button.

What each part holds:

- words.txt: what the member wrote in the dialog.
- journal.toml: UTW's version and the system, then every step a tool recorded since UTW's
  window opened, in order, and the original path, size and SHA-256 of each file under
  inputs/ and outputs/.
- inputs/: each file a tool read, as it was before any change.
- outputs/: the last version of each file a tool wrote.
- state/<tool>/: what each tool held when the report was saved, unsaved edits included,
  each in its usual file format.
- sessions/: when the member ticked Include earlier sessions, one zip for each earlier
  time UTW was opened, laid out as this report's journal.toml, inputs/ and outputs/. A
  save in them is packed (its name ends in .packed): UTW save-editor unpack-session
  <zip> <folder> writes a zip's files back as they were.
- utw.log: the window's log.
- settings.toml: UTW's settings.
- installs.txt: each PES version's folder in the settings, and what UTW found in it.

The files under inputs/ and outputs/ are exact copies, byte for byte. In every other file,
paths show the member's user folder as ~. Read the files with the UTW version the member
ran, named on journal.toml's first line.
