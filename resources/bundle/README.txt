Unlimited Tool Works 0.0.1
==========================

A test version, for the testers of DBC and VGL. If anything goes wrong or reads strangely,
tell us: that is what this version is for.

In the window, the Save editor is on the left bar. The Team compiler, the Export upgrader and
the Ruleset editor are greyed out there, because their windows come in a later version. In
this one, the Team compiler and the Export upgrader run through the two launchers below.


The first time
--------------

1. Unpack this archive into a folder of its own, anywhere you like.
2. Open UTW.exe. It keeps its settings in the data folder beside it, so
   everything it needs stays in this one folder.
3. On the left bar, pick your PES. The window starts on PES 2019, the game VGL is played on.
   DBC is played on PES 2017, so DBC's testers pick PES 2017 there before opening a save.
   If the PES box turns yellow or red, click the gear at the bottom of the bar (Settings) and
   set "PES folder" to the folder that holds the game's PES20xx.exe.
   Then close the window.


Checking a team for DBC
-----------------------

The Save editor checks teams against DBC7's rules from the start: every cup's current rules
are built in, so there is nothing to set up. To check against another cup's rules, pick them
in the dropdown on the Save editor's toolbar, left of Live AATF.


Upgrading old exports
---------------------

Exports made for Red need upgrading once before they compile here.

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

2. The league's first DLC is compiled into several CPKs. With the window closed, open
   data\settings.toml in Notepad, change the line

       multicpk_mode = false

   to

       multicpk_mode = true

   and save the file. quick_compile.bat then puts the teams into 4cc_41_teams.cpk and the CPKs
   after it, filling each one up to its size limit, and the files every team shares into
   4cc_08_bins.cpk.

3. A later matchday is compiled into one CPK of its own. Change the line back to
   multicpk_mode = false, and change the line

       cpk_name = "4cc_99_test"

   to that matchday's CPK, one of 4cc_61_midcup to 4cc_75_midcup, such as

       cpk_name = "4cc_61_midcup"

4. The Save editor checks teams against DBC7's rules. To check them against VGL's, pick
   /vg/ League 27 in the dropdown on its toolbar.


When something goes wrong
-------------------------

Every message names a code, such as export_disabled. The help explains each one: open
UTW.exe and click the ? at the bottom of the left bar.

If a problem stops you, compile with Red as usual, and tell us what happened.
