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
   PES 2019 on the left bar before opening a save. quick_compile.bat below compiles for the
   PES picked there; quick_upgrade.bat upgrades for the PES of the savefile dropped onto it,
   whatever the left bar says. If the PES number on the left bar turns yellow or red, click
   the Settings gear at the bottom of the bar and set "PES folder" to the path of the
   folder that holds the PES exe.
4. VGL's organizer: read "For VGL's organizer" at the end before running either launcher.
   It starts with a backup of the game's download folder and the game's DpFileList.bin,
   changes one setting and sets the order of the work, and all of that comes before the
   sections in between.


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
3. The upgraded exports land in the exports folder. The window lists each export's
   warnings and errors and says how many were upgraded; for one that was not, it says why
   above that line. Info messages are left out of the window: the whole report is in
   upgrade_log.txt beside the launcher, until the next run replaces it.
4. Then move the upgraded old exports out of old_exports. An export already upgraded is not
   upgraded again: the next run lists it as not upgraded ("the output folder already
   exists") and leaves its folder in exports as it is.

The referee export (its name starts with refs, such as "refs Spring") takes no savefile, so
quick_upgrade.bat lists it as not upgraded. Upgrade it from a console in this folder (step
4 of "Compiling" below shows how to open one), with the export's own name in place of
refs Spring:

    UTW.exe export-upgrader upgrade "old_exports\refs Spring" -o exports


Compiling
---------

1. Put the exports to compile in the exports folder (the upgraded ones are already there).
2. Close PES, then double-click quick_compile.bat. It compiles every export in the exports
   folder and installs the result in the game.
3. Read the report, then press a key to close the window. It lists warnings and errors
   only: the whole report, Info messages included, is in compile_log.txt beside the
   launcher, until the next run replaces it.
4. The game needs the cup's official DpFileList.bin. If the report says dpfilelist_outdated
   or dpfilelist_not_official, yours is another list, and the report names the command that
   installs the official one. To run it, close PES, open the folder that holds
   UTW.exe, click the address bar at the top of the window, type cmd and press
   Enter. In the black window that opens, type

       UTW.exe team-compiler upgrade-dpfl

   and press Enter. It changes nothing yet: it lists what it would do (CPKs renamed to their
   official names, empty placeholders for the ones missing, the list replaced and the old one
   kept as DpFileList.bin.bak). Before going on, copy the game's download folder, found
   inside the folder that holds the PES exe, to a safe place: putting that copy back undoes
   everything this command and the compiles after it change there. If the list reads
   right, run the command again with --yes at the end:

       UTW.exe team-compiler upgrade-dpfl --yes

   This is needed once per game folder. If the report said dpfilelist_outdated, the
   compile stopped before compiling anything, so double-click quick_compile.bat again
   afterwards.


For VGL's organizer
-------------------

1. Before the first compile, do three things, in this order.

   a. Back up the game's download folder. Close PES, then copy the download folder, found
      inside the folder that holds the PES exe, to a safe place. The next step renames
      CPKs in it and replaces its DpFileList.bin, and every compile writes CPKs into it.
      To undo all of that, delete the download folder and put the copy back.

   b. Install the cup's official DpFileList.bin. Step 4 of "Compiling" above shows how:
      UTW.exe team-compiler upgrade-dpfl, read what it lists, then the same command with
      --yes. Do it now, not when a compile asks for it: the game loads only the CPKs its
      DpFileList.bin names, and a compile stops when the list lacks the CPKs it writes.
      This is needed once per game folder.

   c. Set the league's first DLC to compile into several CPKs. With the window closed,
      open data\settings.toml in Notepad, change the line

          multicpk_mode = false

      to

          multicpk_mode = true

      and save the file. quick_compile.bat then puts the teams into 4cc_41_teams.cpk and
      the CPKs after it, filling each one up to its size limit, and the files every team
      shares into 4cc_08_bins.cpk. Without this change, the first compile goes into
      4cc_99_test.cpk instead. The game loads that CPK last, so it would cover every team
      and midcup CPK compiled later.

2. The work runs in this order. The managers' tactical exports (the .ted files) go into
   the base save first: import them in the Save editor (the Team menu's "Import / export
   team file...") or in the game's own Edit mode (its import leaves the team names for you
   to rewrite). That makes the official save. In the Save editor, click Save on the
   toolbar, then close UTW.exe: the launchers change the save on disk, and an editor still
   open on it would overwrite their changes the next time you click Save. Then upgrade
   the old exports against the official save (drag it onto quick_upgrade.bat), compile
   the exports (quick_compile.bat), and apply the compile's patch to the official save.

   The upgrade finds each team in the save by its name. The compile finds its ID in
   data\teams_list.txt, which holds the league's 52 teams at their VGL 27 IDs, /@/ 781
   to /zzz/ 832, so each team of the official save needs the same ID. The compile
   refuses a team the list lacks, as team_name_unknown. If a team joins later, open a
   console in this folder as step 4 of "Compiling" shows, and run, with the save's own
   path in place of official\EDIT00000000:

       UTW.exe save-editor export-teams-list "official\EDIT00000000"

   It changes nothing yet: it lists the teams it would add, and each team whose ID in
   the save differs from the list's. If that reads right, run it again with --yes at
   the end.

3. The compile also writes a patch, aesthetics_patch.toml in the output folder beside
   UTW.exe, and applies it to a save: the one savefile_path names in data\settings.toml,
   which is the game's own save under Documents\KONAMI when the line is not set. The old
   file is kept beside it as .bak. When the official save is that save, this step is
   already done. For another save file, close PES, open a console in this folder as
   step 4 of "Compiling" shows, and run, with the save's own path in place of
   official\EDIT00000000:

       UTW.exe save-editor apply-patch output\aesthetics_patch.toml --base "official\EDIT00000000" --out "official\EDIT00000000"

   Keep the quotes around both paths: a path with a space in it fails without them. The
   file already there is kept as EDIT00000000.bak.

4. A later matchday is compiled into one CPK of its own. Change the line back to
   multicpk_mode = false, and change the line

       cpk_name = "4cc_99_test"

   to that matchday's CPK, one of 4cc_61_midcup to 4cc_75_midcup, such as

       cpk_name = "4cc_61_midcup"

   Before compiling it, move the exports compiled on earlier matchdays out of the exports
   folder, so that it holds only this matchday's: a team with two exports there (its first
   Full export and a new Midcup one) has both refused, as duplicate_aesthetics_export.

   A later matchday's export only adds to the team, so it has to be a Midcup one before
   it is upgraded: its old name must contain midcup or additions (rename "co day 5" to
   "co midcup day 5", for example). The upgrade makes any other name a Full export, and
   compiling a Full export removes every kit of the team it does not hold.

5. The patch is the last write to the official save. A .ted imported after it writes that
   team's players over their looks, so the patch has to be applied again.

6. A later matchday's compile writes a new patch, so what goes out is a new official
   save, not only the new CPKs: the players' looks live in the save.

7. Before the official save goes out, open it in the Save editor and check it against the
   league's rules: pick /vg/ League 27 in the dropdown on the toolbar, left of Live AATF,
   then run Live AATF on a team. For all of them at once, open Multi-team AATF in the
   Savefile menu. It starts on the save's first team alone, so pick the league's first
   team in its first Teams box and its last team in the second, then wait until
   "checking..." is gone.


If something goes wrong
-----------------------

Every message names a code, such as fpc_undershorts_break_hiding.
The help explains each one: click the ? button at the bottom of the left bar.

If the window crashes, please post the log file found in the data folder. If a launcher
fails, post its log file, found beside it: upgrade_log.txt for quick_upgrade.bat,
compile_log.txt for quick_compile.bat. If there is no such file, copy what the black window
says before pressing a key to close it, and post that.
