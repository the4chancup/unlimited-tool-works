Unlimited Tool Works 0.0.1
==========================

A preview version for the DBC's managers and VGL's organizer.
If anything goes wrong or reads strangely, please let me know.

This version contains the Save editor. From the left bar, you can also check out the
work in progress preview of the Ruleset editor: it builds and edits a brand-new format of
cup rules file, called 4ccrules. Some of its sections are not editable yet, but you can click
on the link shown to open a demo webpage which shows how it'll look once it's finished.

The Team compiler and the Export upgrader are greyed out on the left bar, because their
windows come in a later version. In this one they run through the two launchers below.


Getting started
---------------

1. Unpack this archive into a folder of its own, anywhere you like.
2. Open UTW.exe (utw on Linux). It keeps its settings in the data folder beside it.
3. The window starts on PES 2017, the game the DBC is played on. VGL's organizer picks
   PES 2019 on the left bar before opening a save, and the two launchers below then work
   on PES 2019 too, since they read the PES picked there. If the PES number on the left bar
   turns yellow or red, click the Settings gear at the bottom of the bar and set
   "PES folder" to the path of the folder that holds the PES exe.


Checking a team for the DBC
---------------------------

The Save editor checks teams against the DBC7's rules from the start, so there is nothing to
set up. Just prepare your team then click the Live AATF button to check it.
If you need to check multiple teams in one go, you can use the Multi-team AATF button in the
Savefile menu.

The editor also comes with the rulesets from several recent cups built in. To check against
another cup's rules, pick them in the dropdown on the toolbar, to the left of the Live AATF
button.


Upgrading old exports
---------------------

Exports in the old format need upgrading once before they compile here.

1. Put the old exports in the old_exports folder: folders, .zip or .7z files.
2. Drag your PES savefile onto quick_upgrade.bat. The savefile is the file named
   EDIT00000000, in the game's save folder under Documents\KONAMI.
3. The upgraded exports land in the exports folder. The window says how many were upgraded;
   for one that was not, it says why above that line.
4. Then move the upgraded old exports out of old_exports. An export already upgraded is not
   upgraded again: the next run lists it as not upgraded ("the output folder already
   exists") and leaves its folder in exports as it is.


Compiling
---------

1. Put the exports to compile in the exports folder (the upgraded ones are already there).
2. Close PES, then double-click quick_compile.bat. It compiles every export in the exports
   folder and installs the result in the game.
3. Read the report, then press a key to close the window.
4. The game needs the cup's official DpFileList.bin. If the report says dpfilelist_outdated
   or dpfilelist_not_official, yours is another list, and the report names the command that
   installs the official one. To run it, close PES, open the folder that holds
   UTW.exe, click the address bar at the top of the window, type cmd and press
   Enter. In the black window that opens, type

       UTW.exe team-compiler upgrade-dpfl

   and press Enter. It changes nothing yet: it lists what it would do (CPKs renamed to their
   official names, empty placeholders for the ones missing, the list replaced and the old one
   kept as DpFileList.bin.bak). If that reads right, run it again with --yes at the end:

       UTW.exe team-compiler upgrade-dpfl --yes

   This is needed once per game folder.


For VGL's organizer
-------------------

1. The game needs the cup's official DpFileList.bin: step 4 of "Compiling" above installs it.

2. The work runs in this order. The managers' tactical exports (the .ted files) go into
   the base save first: import them in the Save editor (the Team menu's "Import / export
   team file...") or in the game's own Edit mode, whose import leaves the team names to
   rewrite. That makes the official save. Then upgrade the old exports against it (drag
   the official save onto quick_upgrade.bat), compile the exports (quick_compile.bat),
   and apply the compile's patch to the official save.

3. The compile also writes a patch, aesthetics_patch.toml in the output folder beside
   UTW.exe, and applies it to a save: the one savefile_path names in data\settings.toml,
   which is the game's own save under Documents\KONAMI when the line is not set. The old
   file is kept beside it as .bak. When the official save is that save, this step is
   already done. For another save file, close PES, open a console in this folder as
   step 4 of "Compiling" shows, and run, with the save's own path in place of
   official\EDIT00000000:

       UTW.exe save-editor apply-patch output\aesthetics_patch.toml --base official\EDIT00000000 --out official\EDIT00000000

   The file already there is kept as EDIT00000000.bak.

4. The league's first DLC is compiled into several CPKs. With the window closed, open
   data\settings.toml in Notepad, change the line

       multicpk_mode = false

   to

       multicpk_mode = true

   and save the file. quick_compile.bat then puts the teams into 4cc_41_teams.cpk and the CPKs
   after it, filling each one up to its size limit, and the files every team shares into
   4cc_08_bins.cpk.

5. A later matchday is compiled into one CPK of its own. Change the line back to
   multicpk_mode = false, and change the line

       cpk_name = "4cc_99_test"

   to that matchday's CPK, one of 4cc_61_midcup to 4cc_75_midcup, such as

       cpk_name = "4cc_61_midcup"

6. The patch is the last write to the official save. A .ted imported after it writes that
   team's players over their looks, so the patch has to be applied again.

7. A later matchday's compile writes a new patch, so what goes out is a new official
   save, not only the new CPKs: the players' looks live in the save.

8. Before the official save goes out, check it against the league's rules: pick
   /vg/ League 27 in the dropdown on the toolbar, left of Live AATF, then run Live AATF
   on a team, or Multi-team AATF in the Savefile menu for all of them at once.


If something goes wrong
-----------------------

Every message names a code, such as fpc_undershorts_break_hiding.
The help explains each one, just click the ? button at the bottom of the left bar.

If the tool crashes, please post the log file found in the data folder.
