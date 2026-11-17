A problem report saved by Unlimited Tool Works (UTW) through its Report a problem button.

What each part holds:

- words.txt: what the member wrote in the dialog.
- journal.toml: UTW's version and the system, then every step that changed what a tool
  held since UTW's window opened, in order, and the original path, size and SHA-256 of
  each file below.
- inputs/: each file a tool opened, as it was before any change.
- outputs/: the last version of each file a tool wrote.
- state/<tool>/: what each tool held when the report was saved, written as its own file.
- utw.log: the window's log.
- settings.toml: UTW's settings.
- installs.txt: the PES installs UTW found.

Paths are written with the member's user folder as ~. Read the files with the UTW version
the member ran, named on journal.toml's first line.
