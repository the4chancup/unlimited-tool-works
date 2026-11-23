// The build's help for the drafts' help window and log lines, never edited by hand:
// `just draft-help` writes it again; a test in `studio` fails while it differs from the build's.
const BUILD_HELP = {
  "chapters": [
    {
      "id": "setting-up",
      "messages": [],
      "title": "Setting up",
      "topics": [
        {
          "body": "Unlimited Tool Works ships as a `.7z` archive: unpack it anywhere and run\n`UTW.exe`. There is no installer. The folder you unpack it into is the\nprogram's home, so move it, copy it or delete it as one piece.\n\nYou reach each tool of the suite from the sidebar.\n",
          "slug": "install",
          "title": "Installing"
        }
      ]
    },
    {
      "id": "first-run",
      "messages": [],
      "title": "First run and data location",
      "topics": [
        {
          "body": "When a `data/` folder holding a `settings.toml` file already sits beside\n`UTW.exe`, as it does in a download that comes with its settings, the first\nrun asks nothing, and your settings and data stay in that folder. When there\nis no such folder but your user folder (below) already holds a\n`settings.toml`, from an earlier copy of the program for instance, the first\nrun asks nothing either and uses that one.\n\nOnly when neither place holds a `settings.toml` does the first run ask where\nyour settings and data should live:\n\n- `Next to the program` puts them in a `data/` folder beside `UTW.exe`, so the\n  program's folder holds the program and everything it keeps: it is fully\n  portable, and deleting it removes both;\n- `In my user folder` puts them in `%APPDATA%\\unlimited-tool-works` on Windows\n  or `~/.config/unlimited-tool-works` on Linux, so the data survives deleting\n  or replacing the program folder.\n",
          "slug": "data-location",
          "title": "Choosing the data location"
        }
      ]
    },
    {
      "id": "updates",
      "messages": [],
      "title": "Updates",
      "topics": [
        {
          "body": "Unlimited Tool Works looks for new versions by itself and can install one for\nyou, so you do not need to download a new release by hand.\n\n**Checking for a new version**\n\nThe program reads the list of releases on its GitHub page when it starts, and\nagain every 5 minutes while it stays open. You can turn this off with\n`Check for updates` in the settings. When a check fails, because you are\noffline for instance, nothing is shown and the program tries again later.\n\nThe launchers (the `.bat` files that come with the program on Windows, such\nas `quick_compile.bat`), and a tool's command typed in a terminal, look for a new\nversion too while `Check for updates` is on, and mention one in a line after\ntheir report. They install nothing: start the program and use the offer in the status\nbar.\n\n**The offer in the status bar**\n\nWhen a newer version is out, the status bar shows `Update available` and the\nversion's number in blue. Nothing opens by itself: click the text, or the\n`Details` button beside it, when you want to know more.\n\n**Update available**\n\nThe `Update available` window names the new version and the one you have, and shows the\nrelease notes, which say what the new version brings. It has four buttons:\n\n- `Update now` downloads the new version and installs it, as described below;\n- `Open release page` opens the version's page in your browser, where you can\n  download it yourself;\n- `Skip this version` removes the offer, and no later check offers this version\n  again, although a newer one will still be offered;\n- `Remind me later` closes the window and leaves the offer in the status bar.\n\n**What Update now does**\n\n1. It downloads the new version for your system, showing how many MB are done\n   above a progress bar. `Cancel`, under the bar, stops the download and brings\n   the offer back, with nothing changed.\n2. It checks the download against the checksums published with the release, so\n   a damaged download is never installed.\n3. It unpacks the new version into the program's folder. The program you are\n   running moves into the `old` folder inside it, and the new version's files\n   take the place of the old ones. Your settings and data are left as they are.\n\nWhile the download runs, clicking outside the window or pressing Escape does not\nclose it, so a stray click cannot hide or stop the update. Closing the program\nstops the download and waits until nothing is left half done.\n\n**When an update fails**\n\nIf something goes wrong, `Update failed` says what happened, and that nothing\nwas changed, so the version you have keeps working. The cause can be the\nconnection, a download that does not match its checksum, or a program folder\nyou are not allowed to write to, which is found before anything is downloaded.\nThe window has three buttons:\n\n- `Retry` starts the update again;\n- `Open release page` lets you download the new version by hand instead;\n- `Close` closes the window and leaves the offer in the status bar.\n\nIf the update stops while it replaces the program's files, it puts the old ones\nback. In the rare case where it cannot put them all back, the message does not\nsay that nothing was changed, and it names the folder holding them: move them\nback into the program's folder. No other update starts while that folder holds\nfiles.\n\n**Restarting**\n\nOnce the new version is in place, `Update ready` offers two buttons:\n\n- `Restart now` closes the program and starts the new version. If a tool first\n  asks about unsaved work, the program does not start again by itself, whatever\n  you answer: the new version runs the next time you start it;\n- `Later` closes the window. The status bar then reads `Update ready`, which\n  opens this window again, and the new version runs the next time you start\n  the program.\n\n**After the update**\n\nThe first time a new version starts, the status bar shows `Updated to` with its\nnumber and a `What's new` button. The button opens the What's new chapter of\nthis help, which lists what changed, version by version.\n",
          "slug": "updating",
          "title": "Updating the program"
        }
      ]
    },
    {
      "id": "reporting",
      "messages": [],
      "title": "Reporting a problem",
      "topics": [
        {
          "body": "When UTW does something wrong, **Report a problem** (the `⚠` while the\nsidebar is folded) saves a file to post with your report, so whoever looks\ninto it can repeat what you did. It sits in the sidebar just above\n**Help (WIP)** and opens a dialog. Nothing is sent: you post the file\nyourself, wherever you usually report problems.\n\nFor now only the Save editor records what you do in it; the other tools join\nas they are built. The file holds:\n\n- what you write in the dialog's box;\n- every step you took in the Save editor since the window opened: opening a\n  save, edits, imports and saves;\n- the files it opened, as they were before any change, the files it wrote,\n  and the open save as it is now, with your edits;\n- `utw.log`, UTW's version, your settings and the PES installs found.\n\nIn the box, write what you did, what you expected and what you got, and\nanything done outside UTW, in the game or in another tool. The words stay\nthere until the window closes, so you can close the dialog and finish them\nlater.\n\nUTW also keeps a record of each session, from the moment it opens until\nit closes, with the saves opened and written in it. The five sessions\nbefore the current one stay in UTW's data location; no setting turns\nthis off. When the problem began before UTW was last opened (it\ncrashed, or you tried a save in the game after closing UTW), tick\n**Include earlier sessions** under the list in the dialog: the file\nthen holds those sessions too.\n\n**Save…** asks where to put the file, which is named by the date and time\n(`utw-report-2026-11-15-1030.zip`). The file holds your whole save, and your\nuser folder is written as `~` in its paths.\n",
          "slug": "report",
          "title": "Report a problem"
        }
      ]
    },
    {
      "id": "whats-new",
      "messages": [],
      "title": "What's new",
      "topics": []
    },
    {
      "id": "compiler",
      "messages": [
        {
          "code": "export_extract_failed",
          "effect": "The export is not compiled. Repack the archive, or unpack it into a folder.",
          "severity": "Error",
          "when": "The archive (`.zip` or `.7z`) cannot be opened, or a path inside it is unusable: it leads outside the archive, or two names differ only in letter case or Unicode form."
        },
        {
          "code": "no_exports_found",
          "effect": "Nothing is checked or compiled.",
          "severity": "Warning",
          "when": "The exports folder holds no export folder, `.zip` or `.7z` (a fresh install's empty `exports/` reports this until you fill it)."
        },
        {
          "code": "export_disabled",
          "effect": "The export is not compiled, and it does not count toward the duplicate-referees check.",
          "severity": "Info",
          "when": "The export's root holds a `NO_USE` or `NO_USE.txt` marker file."
        },
        {
          "code": "export_upgrade_unfinished",
          "effect": "It is not compiled. Remove it, then upgrade the old export again.",
          "severity": "Warning",
          "when": "A folder whose name ends in `.upgrading` was left behind by an Export upgrader run that did not finish."
        },
        {
          "code": "export_identified",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The export was recognized as a team or the referees, and the line names which."
        },
        {
          "code": "export_balls_skipped",
          "effect": "The export is not compiled.",
          "severity": "Info",
          "when": "The export's name starts with `balls`, so it is a balls export, and those belong to the Balls compiler."
        },
        {
          "code": "multiple_ref_exports",
          "effect": "None of them is compiled. Disable or remove all but one.",
          "severity": "Error",
          "when": "More than one `refs` export is among the exports (one marked `NO_USE` does not count)."
        },
        {
          "code": "duplicate_aesthetics_export",
          "effect": "None of them is compiled, but the other teams still are. Remove the one that should not be used, or disable it with a `NO_USE` file.",
          "severity": "Error",
          "when": "Two or more exports resolve to the same team (the line names the team's ID and the exports). An export already left out does not count."
        },
        {
          "code": "color_entry_invalid",
          "effect": "The line is ignored, but the rest of the file is still read.",
          "severity": "Warning",
          "when": "A line of a `colors.txt` does not hold exactly one color (two colors on one line, the old Team Note kit entry among them), or a valid color sits past the most the file holds, which is two for a kit and four for the team."
        },
        {
          "code": "team_colors_missing",
          "effect": "The team keeps the colors it has installed.",
          "severity": "Info",
          "when": "The export's root has no `colors.txt` (it is optional)."
        },
        {
          "code": "kit_config_generated",
          "effect": "The template config, which carries the FPC values, is written for the kit. Nothing to do.",
          "severity": "Info",
          "when": "A kit has no `config.toml` and no installed one to keep. A `Full` export never keeps an installed config, and a `Midcup` export keeps the installed one wherever the slot has one it can read."
        },
        {
          "code": "kit_config_invalid",
          "effect": "The kit is left out, even with `pass_through` on.",
          "severity": "Error",
          "when": "A kit's `config.toml` is not UTF-8 text, or it cannot be read as a kit config: it is not TOML, a value has the wrong type or is out of range, or a key that must be a table holds something else (the line names the error)."
        },
        {
          "code": "kit_config_version_clamped",
          "effect": "The value is lowered to one the version can hold when the config is written.",
          "severity": "Warning",
          "when": "A kit config value does not fit the PES version compiled for (a name `y` over 33 before PES 2021). The value is in the kit's own `config.toml`, or in an installed config that a compile writes the FPC values or the collar into, for a kit the `Midcup` export does not hold."
        },
        {
          "code": "kit_config_option_ignored",
          "effect": "The config is built as written and the game ignores the option.",
          "severity": "Warning",
          "when": "A kit's `config.toml` asks for a sleeve or fit option its shirt model does not take (cut-out short sleeves, undershirt-only long sleeves, or the tight fit, on a model other than 144 or 160). When a player folder of the team carries `fpc_on`, the FPC values make the shirt model 176, which takes none of them."
        },
        {
          "code": "kit_config_fpc_adjusted",
          "effect": "The FPC values are written into the built config. Your file is not changed, and a compile never removes them.",
          "severity": "Info",
          "when": "A player folder carries `fpc_on`, and a kit config lacks the FPC values, whether it is one the export supplies (the goalkeeper kit's included) or the installed one of a kit a `Midcup` export does not hold."
        },
        {
          "code": "kit_config_fpc_unpatched",
          "effect": "The kit is left as it is. Send it in an export to give it the FPC values.",
          "severity": "Warning",
          "when": "A player folder carries `fpc_on`, and a kit the game lists for the team is not in the `Midcup` export and either has no installed config to patch or has one that cannot be read."
        },
        {
          "code": "kit_config_collar_unpatched",
          "effect": "The kit keeps its own collar. Send it in an export to give it the new one.",
          "severity": "Warning",
          "when": "The export brings a collar, and a kit the game lists for the team is not in the `Midcup` export and has no installed config to carry it, or one that cannot be read (`kit_config_fpc_unpatched` has not named it)."
        },
        {
          "code": "kit_placeholder",
          "effect": "The bundled checkerboard stands in as its main texture, with the template config unless one is supplied. Its menu colors come from its `colors.txt`, or are magenta and black without a usable one (`kit_colors_missing`).",
          "severity": "Info",
          "when": "A `Full` export's kit has no `kit` texture, its own or `all/`'s (an empty kit folder included). A `Midcup` kit without a `kit` texture keeps the installed textures."
        },
        {
          "code": "kit_texture_not_used",
          "effect": "The file is left out and the rest of the kit is built. One map is never converted into the other.",
          "severity": "Info",
          "when": "A kit holds a texture the version compiled for does not read: a `kit_mask` for PES 2018 to 2021, a `kit_srm` for PES 2015 to 2017, or a `kit` name outside the ones the compiler builds."
        },
        {
          "code": "kit_colors_derived",
          "effect": "The kit's menu colors are taken from its main texture. Nothing to do.",
          "severity": "Info",
          "when": "A `Full` export's kit has no `colors.txt`, or one with fewer than two valid colors. A `Midcup` kit without a usable `colors.txt` is not reported and keeps its installed colors."
        },
        {
          "code": "kit_colors_missing",
          "effect": "Magenta and black are written, so the gap shows in the game's menus. Give the kit a `colors.txt` or a `kit` texture.",
          "severity": "Warning",
          "when": "A `Full` export's kit has no usable `colors.txt` and no main texture to take the menu colors from, as is always the case for a placeholder kit without a `colors.txt`."
        },
        {
          "code": "kit_layout_converted",
          "effect": "The sock parts of `kit`, and of its `kit_mask` or `kit_srm`, are moved to the layout of the version compiled for, and the other textures are untouched. Nothing to do.",
          "severity": "Info",
          "when": "The kit's layout marker says its main texture was drawn for PES 2015 to 2017 (`pre-fox`) or PES 2018 to 2021 (`fox`), and the compile is for the other group."
        },
        {
          "code": "kit_collar_reserved",
          "effect": "The kit is left out, because its players would wear the referees' marker. Give it another collar.",
          "severity": "Error",
          "when": "A kit's collar or winter collar is 77, the referees' reserved collar, once the FPC and custom-collar rules have run."
        },
        {
          "code": "collar_id_invalid",
          "effect": "The file is left out.",
          "severity": "Error",
          "when": "A `Collars/` file's name is not `collar_` plus a number, or the number is not a stock collar a kit can wear on the version compiled for. The stock collars a kit can wear are 1-101 on PES 2015, 1-105 on PES 2016, 1-116 on PES 2017 and 2018, 1-124 on PES 2019, 1-127 on PES 2020 and 1-131 on PES 2021, and the 9xx collars are stock but no kit can wear one. `collar_105` and `collar_77` are `collar_id_conflict` instead."
        },
        {
          "code": "collar_id_conflict",
          "effect": "The file is left out and the team's kits keep their own collars.",
          "severity": "Error",
          "when": "The collar number the file names is already taken: by an export compiled before this one, by another collar file of this export (an export brings one collar, since its kit configs take one ID), or by the compiler, which keeps 105 (the FPC collar) and 77 (the referees' marker). The line names who holds it."
        },
        {
          "code": "logo_fit_applied",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A logo image was not square and was made square, and the line names the file and the way (`fit` when the name has no tag, else `crop` or `stretch`)."
        },
        {
          "code": "logo_upscaled",
          "effect": "It is built enlarged, and will look less sharp.",
          "severity": "Warning",
          "when": "A logo image is smaller than the size it is made into (512 pixels square for the main logo, 128 for `logo_small`), measured on its long side for `fit` and its short side for `crop` and `stretch`."
        },
        {
          "code": "boots_id_pool_exhausted",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "More shared `Boots/` folders need a shared ID than the team's block of 17 IDs has free (a `Midcup` export can run out when the installed CPKs already spend some, but a `Full` export cannot)."
        },
        {
          "code": "gloves_id_pool_exhausted",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "More shared `Gloves/` folders need a shared ID than the team's block of 17 IDs has free (a `Midcup` export can run out when the installed CPKs already spend some, but a `Full` export cannot)."
        },
        {
          "code": "shared_ids_unrecorded",
          "effect": "Its folders are numbered alphabetically, as a `Full` export's are, and may take IDs an installed CPK gives other folders.",
          "severity": "Warning",
          "when": "A `Midcup` export holds shared `Boots/` or `Gloves/` folders that take a shared ID, and no installed CPK records the team's shared IDs."
        },
        {
          "code": "portrait_conflict",
          "effect": "The export is not compiled, because the compiler cannot tell which portrait is meant. Remove one, or make them the same file.",
          "severity": "Error",
          "when": "A player has a portrait in his folder and a different one for his slot in `Portraits/` (two identical files count as one)."
        },
        {
          "code": "link_combined",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A player folder holds a link file and models of its own for the same part (face, boots or gloves), so the shared folder's models are built into the player's own set."
        },
        {
          "code": "settings_toml_invalid",
          "effect": "The file is ignored, even with `pass_through` on. The player's look (his `ingame_face.toml`'s keys included) and the boots and gloves IDs his settings would give are left out of the patch, so the save keeps the ones it has. His models still build, and an ID they or a link give is still written to the patch.",
          "severity": "Error",
          "when": "A player's `settings.toml` is not UTF-8 text or does not parse as the settings file (the line names the error). The in-game face's keys (`iris_color`, `[appearance.face]`, `[appearance.hair]`) are in `ingame_face.toml`, so one left here is an unknown key."
        },
        {
          "code": "ingame_face_toml_invalid",
          "effect": "Its keys are ignored, even with `pass_through` on. The player's look (his `settings.toml`'s keys included) and the boots and gloves IDs his settings would give are left out of the patch, so the save keeps the ones it has. His models still build, and an ID they or a link give is still written to the patch.",
          "severity": "Error",
          "when": "A player's `ingame_face.toml` is not UTF-8 text or does not parse as the in-game face file (the line names the error)."
        },
        {
          "code": "settings_model_id_conflict",
          "effect": "The compiler's ID wins and the key is ignored.",
          "severity": "Warning",
          "when": "In team exports, a `settings.toml` sets `boots_id` or `gloves_id` for a part whose ID the compiler assigns on its own (from the folder's own models or a link file)."
        },
        {
          "code": "settings_value_out_of_range",
          "effect": "The key's default is written in its place (a face type of 0, the template's motion, a custom skin written as 1).",
          "severity": "Warning",
          "when": "A `settings.toml` or `ingame_face.toml` value is inside its key's widest range but outside the range of the version compiled for (the line names the key, the value and the version's range)."
        },
        {
          "code": "settings_toml_name_shared",
          "effect": "The name, if it fits the version (`settings_name_invalid`), is written for every one of those players.",
          "severity": "Warning",
          "when": "A folder of an export marked `write_names` maps to more than one player, and its `settings.toml` gives `name` or `shirt_name`."
        },
        {
          "code": "settings_name_invalid",
          "effect": "It is not written, so the name already in the save is kept.",
          "severity": "Warning",
          "when": "A `name` or `shirt_name` in a `settings.toml` does not fit the version's field or character set (the line names the key)."
        },
        {
          "code": "fpc_strip_conflict",
          "effect": "The preset's value wins and the file's key is ignored.",
          "severity": "Warning",
          "when": "A `settings.toml` sets a key the folder's `fpc_on` or `fpc_off` preset also sets, to another value (for `skin_color`, a custom skin where the preset sets a preset one). `boots_id` and `gloves_id` are checked separately."
        },
        {
          "code": "fpc_inners_break_hiding",
          "effect": "It is written as authored. Take the inners off, or drop `fpc_on`, if the hiding should hold.",
          "severity": "Warning",
          "when": "A player whose folder carries `fpc_on` wears inners under a hidden body, which breaks the hiding."
        },
        {
          "code": "fpc_undershorts_break_hiding",
          "effect": "It is written as authored. Take the undershorts off, or drop `fpc_on`, if the hiding should hold.",
          "severity": "Warning",
          "when": "A player whose folder carries `fpc_on` wears undershorts under a hidden body, which breaks the hiding."
        },
        {
          "code": "fpc_taping_shows_pieces",
          "effect": "It is written as authored. Some custom setups do this on purpose. If yours does not, take the taping off.",
          "severity": "Info",
          "when": "A player whose folder carries `fpc_on` wears wrist or ankle taping, which shows pieces of the hidden body."
        },
        {
          "code": "fpc_gloves_checkbox_winter_gloves",
          "effect": "It is written as authored, so the player wears winter gloves in winter conditions.",
          "severity": "Warning",
          "when": "The gloves checkbox is on with gloves ID 0 (the ID `settings.toml` resolves to), on any player, whatever the FPC marker. A `Midcup` player keeping his installed row is not checked."
        },
        {
          "code": "fmdl_fcl_hair_fallback",
          "effect": "Nothing to do. Name a model as boots or gloves when it is one.",
          "severity": "Info",
          "when": "In PES 2018 to 2021, a model whose name says nothing about what it is (`torso.fmdl`) is taken for face content and merged into the face's `fcl_hair.fmdl`."
        },
        {
          "code": "fmdl_merged",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2018 to 2021, several models resolved to the same name and were merged into one, in file-name order."
        },
        {
          "code": "model_merged",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2015 to 2017, several `.model` files were merged into one `boots.model` and its `.mtl` (a shared `Boots/` folder holding several, an `ingame_face.toml` player's boots parts, or a combined boots link's model)."
        },
        {
          "code": "model_merge_flags_conflict",
          "effect": "That part of the folder (face, boots or gloves) is left out, as for `merge_material_conflict`. Drop all but one of the files being merged.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, `.model` files being merged carry different header `flags`, whose meaning is unknown, so there is no rule to combine them."
        },
        {
          "code": "model_hand_split",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A face model's vertices are weighted to the hand bones (`skh_`), so its hands were cut off at the wrist into the player's gloves (two more `face.xml` entries on PES 2015 to 2017)."
        },
        {
          "code": "merge_material_conflict",
          "effect": "That part of the folder (face, boots or gloves) is left out, but the rest still builds, so the player is built incomplete. Treat the folder as broken until the materials agree.",
          "severity": "Error",
          "when": "Models merged into one define a material of the same name differently."
        },
        {
          "code": "skl_merge_conflict",
          "effect": "That part of the folder is left out, as for `merge_material_conflict`.",
          "severity": "Error",
          "when": "Models merged into one disagree on their skeleton (a bone differs, or their `.skl` files do)."
        },
        {
          "code": "skl_no_slot",
          "effect": "The `.skl` is ignored, but the model is still built.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, a `.skl` pairs with a `face_high`, `hair_high` or `oral` model, and none of those has a skeleton slot (the `.skl` a `.common` link brings counts too)."
        },
        {
          "code": "face_file_not_used",
          "effect": "The file is not read.",
          "severity": "Info",
          "when": "A face file sits in a folder that builds no face. The face files are `face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo` on PES 2018 to 2021, and a member's `face.xml` under `ingame_face.toml` on PES 2015 to 2017. A folder builds no face when it is a player folder with no face model, no `Faces/` link bringing one and, on PES 2015 to 2017, no `face.xml` of its own; a shared `Faces/` folder with no face model and no such `face.xml`; or a shared `Boots/` or `Gloves/` folder."
        },
        {
          "code": "file_not_used",
          "effect": "The file is not read and left out of the compile.",
          "severity": "Warning",
          "when": "A file sits where the folder allows it, but nothing reads it: a `.skl` pairing no model, a `.common` link to a kind nothing takes, a `materials.toml`, an `.xml`, `.bin` or `.fclo` under no face name, a `.txt` note, a texture of a folder that builds no package (a player or referee folder holding `ingame_face.toml` and no model), a `Common/` file of a kind nothing reads, or a `refs` export's kit folder, logo, portrait or collar file."
        },
        {
          "code": "shared_folder_no_model",
          "effect": "It is not built and the player wears the game's own. Put a model of the folder's kind in it. If the player has his own model for that part, the folder is a texture source for it.",
          "severity": "Warning",
          "when": "A shared `Boots/` or `Gloves/` folder holds no model of its kind (textures only, or nothing, not even a `.common` link to one on PES 2018 to 2021), and a player links it without a model of his own for that part, so there is nothing to load."
        },
        {
          "code": "kit_variant_missing",
          "effect": "The lowest variant fills the gap. It is copied under the missing number's name on PES 2015 to 2017 (for a model set, the files the lowest variant's entries name), and written once as `pants_kitN` on PES 2018 to 2021.",
          "severity": "Warning",
          "when": "A texture variant set (`pants_kit1`, `pants_kit3`), or on PES 2015 to 2017 a model set a `face.xml` lists, has no variant for a kit number the export defines and no `kitF` fallback."
        },
        {
          "code": "kit_variant_model_left_out",
          "effect": "The set's `kitF` is used, or its lowest variant when it has none, and the other variants are left out. A variant made for the other group of games (PES 2015 to 2017 or PES 2018 to 2021) is converted.",
          "severity": "Warning",
          "when": "Per-kit model variants (`pants_kit1`, `pants_kit2`, and a `kitF` as the set's fallback) sit where nothing switches models with the kit: anywhere in PES 2018 to 2021, and in PES 2015 to 2017 in a shared `Boots/` or `Gloves/` folder or under `ingame_face.toml`, where no `face.xml` names the set."
        },
        {
          "code": "kit_variant_mtl_differs",
          "effect": "The model is packed, but the game finds no `.mtl` for that kit. Give the `.mtl` the name the entry implies.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017, a per-kit model variant's `.mtl` is not the one its set's `face.xml` entry implies for its kit number (the line names the model, the `.mtl` found and the one expected)."
        },
        {
          "code": "xml_face_neck_added",
          "effect": "The invisible stand-in model and its `.mtl` are added. Nothing to do.",
          "severity": "Info",
          "when": "A face's `face.xml` has no `face_neck` entry (a generated one or a member's own, but not a blank face's)."
        },
        {
          "code": "xml_uniform_pes15",
          "effect": "It is rewritten for PES 2015. Nothing to do.",
          "severity": "Info",
          "when": "When compiling for PES 2015, a face's `face.xml` (a generated one or a member's own) uses `type=\"uniform\"`, which PES 2015 names `uniform_sub`."
        },
        {
          "code": "face_diff_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `face_diff.bin`, or the decoded content of a `face_diff.xml`, is not a face diff, or the `face_diff.xml` is not base64 text or a `<dif>` holding it (the line names the file and the reason)."
        },
        {
          "code": "xml_dif_conflict",
          "effect": "The folder is left out. Keep one source of the face diff.",
          "severity": "Error",
          "when": "The folder gives its face diff twice. Its `face.xml` holds a `<dif>` beside a `face_diff.xml`, or it holds both `face_diff.bin` and `face_diff.xml` (a `<dif>` beside a `face_diff.bin` is no conflict, since each game reads its own)."
        },
        {
          "code": "xml_broken",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `face.xml` does not parse as XML (the finding's line names where in the file, by line and column)."
        },
        {
          "code": "mtl_texture_not_found",
          "effect": "The folder is still built, because the game still loads a face with a missing texture. Check the player in the game.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017 (and in PES 2018 to 2021 for the `.mtl` of a `.model` being converted), a material a mesh uses names a texture that nothing supplies: not in the folder, and for a Common path neither in `Common/` nor in a CPK listed before this compile's own."
        },
        {
          "code": "mtl_texture_unused_missing",
          "effect": "Nothing is left out. Nothing to do.",
          "severity": "Info",
          "when": "A `.mtl` names a texture nobody supplies, but only on materials that no mesh uses."
        },
        {
          "code": "mtl_common_path_invalid",
          "effect": "The `.mtl`'s folder is left out. For a `.mtl` in `Common/`, only the file is left out. Write the team's Common as `team_common_folder/`. An export upgraded before 0.0.2 needs upgrading again, from the original old export.",
          "severity": "Error",
          "when": "A `.mtl` texture path below `model/character/uniform/common/` names a folder there other than `team_folder`, the folder that stands for the team's Common: a team ID, `XXX` or another team's name. It is also reported for a folder after `common_folder/` (the game's own, where a reference names only a file directly below it), and for a path starting `common/`, the archive's own root folder, which no reference names."
        },
        {
          "code": "xml_root_tag_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A member's `face.xml` root element is not `<config>`."
        },
        {
          "code": "xml_model_type_missing",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `<model>` of a member's `face.xml` has no `type`."
        },
        {
          "code": "xml_model_path_missing",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `<model>` of a member's `face.xml` has no `path`."
        },
        {
          "code": "xml_model_not_found",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `path` or `material` of a member's `face.xml` names a file the export does not hold, through a `./` or a Common path."
        },
        {
          "code": "xml_common_path_invalid",
          "effect": "The folder is left out. Write the team's Common as `team_common_folder/`. An export upgraded before 0.0.2 needs upgrading again, from the original old export.",
          "severity": "Error",
          "when": "A `path` or `material` of a member's `face.xml` below `model/character/uniform/common/` names a folder there other than `team_folder`, the folder that stands for the team's Common: a team ID, `XXX` or another team's name. It is also reported for a folder after `common_folder/` (the game's own, where a reference names only a file directly below it), and for a value starting `common/`, the archive's own root folder, which no reference names."
        },
        {
          "code": "xml_oral_prefix_missing",
          "effect": "The folder is left out. Rename the model with one of those prefixes.",
          "severity": "Error",
          "when": "In PES 2016, a model a member's `face.xml` names has a file name that starts with none of `face_high_`, `hair_high_` or `oral_`, one of which the game needs to load it."
        },
        {
          "code": "xml_element_unknown",
          "effect": "Kept as written, but the compiler cannot vouch for it.",
          "severity": "Warning",
          "when": "A member's `face.xml` holds a child of `<config>` other than `<model>` or `<dif>`."
        },
        {
          "code": "xml_attribute_unknown",
          "effect": "Kept as written, but the compiler cannot vouch for it.",
          "severity": "Warning",
          "when": "A `<model>` of a member's `face.xml` carries an attribute other than `level`, `type`, `path`, `material` or `ratio`."
        },
        {
          "code": "xml_type_unknown",
          "effect": "Kept as written.",
          "severity": "Warning",
          "when": "A `type` of a member's `face.xml` is one the compiler never generates, so it has not been tested in the game."
        },
        {
          "code": "xml_level_lod",
          "effect": "Kept as written. Nothing to do.",
          "severity": "Info",
          "when": "A member's `face.xml` gives a `level` (the model's level of detail) other than 0. Higher levels are uncommon but known to work."
        },
        {
          "code": "xml_ratio_invalid",
          "effect": "Kept as written.",
          "severity": "Warning",
          "when": "A `ratio` of a member's `face.xml` is not a number."
        },
        {
          "code": "xml_path_unchecked",
          "effect": "Kept as written.",
          "severity": "Warning",
          "when": "A `path` or `material` of a member's `face.xml` is in a form the compiler cannot resolve (a game path such as `model/character/face/common/`), so the compiler does not check that it exists."
        },
        {
          "code": "xml_face_neck_multiple",
          "effect": "Kept as written.",
          "severity": "Warning",
          "when": "A member's `face.xml` has more than one `face_neck` entry."
        },
        {
          "code": "xml_model_unlisted",
          "effect": "The file is not packed and not checked. A `.mtl` that no entry names is skipped the same way.",
          "severity": "Warning",
          "when": "A `.model` sits in a folder whose own `face.xml` does not list it (a reference to a per-kit set names every variant of the set)."
        },
        {
          "code": "xml_ignored_fox",
          "effect": "The xml is ignored, and the folder's models compile as they would without it.",
          "severity": "Info",
          "when": "In PES 2018 to 2021, a player or shared folder holds its own `face.xml`, which these versions do not read."
        },
        {
          "code": "xml_ignored_shared",
          "effect": "The xml is ignored and not checked, and the folder's models are read as they would be without it.",
          "severity": "Info",
          "when": "In PES 2015 to 2017, a shared `Boots/` or `Gloves/` folder holds a `face.xml`, which its output (one model, one `glove.xml`) does not use."
        },
        {
          "code": "xml_shared_face_conflict",
          "effect": "The player folder is left out, even with `pass_through` on. Remove the player folder's `face.xml`, or its link to `Faces/`.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a player folder holding a `face.xml` links a `Faces/` folder, whether or not that folder holds one, because a face takes one xml."
        },
        {
          "code": "shared_texture_conflict",
          "effect": "The player folder is left out. Make the copies identical, which packs them once, or put the texture in the player's own folder, whose copy wins.",
          "severity": "Error",
          "when": "Two shared folders a player combines hold a texture of one name with different bytes. A texture `.common` link in one of them stands for a file of its linked name, but it always counts as different from a real file, even one with the same content. The player's own copy wins over either (`shared_texture_overridden`)."
        },
        {
          "code": "shared_texture_overridden",
          "effect": "The player's own is the one every model of his names, and the shared folder's copy is left out. Nothing to do.",
          "severity": "Info",
          "when": "A texture the player's own folder holds, as a file or a texture `.common` link, is also held by a shared folder he combines, and either the two differ or one of them is a link (two links naming the same `Common/` texture report nothing)."
        },
        {
          "code": "texture_too_small",
          "effect": "It is left out, and where it sits decides what goes with it: a texture in a model folder or a kit folder takes the folder with it, while a `Common/` or portrait file is left out alone.",
          "severity": "Error",
          "when": "A texture has a side under 4 pixels, smaller than one compression block."
        },
        {
          "code": "texture_not_pow2",
          "effect": "It is left out, as for `texture_too_small`.",
          "severity": "Error",
          "when": "A portrait has a side that is not a power of two (any version), or, on PES 2018 to 2021, a texture has a mip chain where neither side is a power of two. An image file always gets a chain, but a `.dds` or `.ftex` with a single level passes at any size. A kit's main texture on PES 2015 to 2017 is `kit_texture_too_big` instead."
        },
        {
          "code": "texture_not_div4",
          "effect": "It is left out, as for `texture_too_small`, even with `pass_through` on.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a texture the compiler writes block-compressed has a side that is not a multiple of 4, and those games cannot make a compressed texture at such a size. A `.dds` or `.ftex` stored uncompressed is written uncompressed, so it does not get this finding, unless it is a normal map (a name ending in `_nrm`, or in `_nrm` and a kit number), which is always compressed."
        },
        {
          "code": "kit_texture_too_big",
          "effect": "The kit is left out.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a kit's main `kit` texture, its own or `all/`'s, is wider or taller than 2048 pixels, or has a side that is not a power of two."
        },
        {
          "code": "texture_type_mismatch",
          "effect": "It is left out, as for `texture_too_small`, even with `pass_through` on.",
          "severity": "Error",
          "when": "A texture's bytes open with the signature of a format other than its extension's (a PNG renamed `skin.dds`, not resaved)."
        },
        {
          "code": "texture_unreadable",
          "effect": "It is left out, as for `texture_too_small`, even with `pass_through` on.",
          "severity": "Error",
          "when": "A texture's header cannot be read at all, neither as the format its name declares nor as any other (not a texture, or cut short before its header ends)."
        },
        {
          "code": "texture_codec_unsupported",
          "effect": "It is left out, as for `texture_too_small`, even with `pass_through` on. Resave it in a format the compiler reads.",
          "severity": "Error",
          "when": "A texture is in a codec the compiler does not read, or is a kind of `.dds` it does not read (a signed block format, a volume, an array, a paletted `.dds`, a bad header)."
        },
        {
          "code": "portrait_header_rewritten",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A `.dds` portrait under a DX10 extension header was written under the plain header of its format (the one the game reads), its pixels unchanged. A portrait with the DX10 header crashed PES 2019."
        },
        {
          "code": "material_texture_unused",
          "effect": "The role is left out of the built material. Nothing to do.",
          "severity": "Info",
          "when": "A material names a texture for a role that has no sampler in the version compiled for (`metalness` on PES 2015 to 2017, or a sampler of the source game's material table that the target does not carry)."
        },
        {
          "code": "bone_folded_for_version",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A converted model uses a bone the target version's skeleton lacks, so its weights were moved to the bone the fold table names, or the nearest one."
        },
        {
          "code": "skeleton_retargeted",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A converted model's pose was moved from its source version's skeleton onto the target's (bones moved by more than a small tolerance)."
        },
        {
          "code": "bone_matrix_unknown",
          "effect": "The model is built as it is. Check the player in the game.",
          "severity": "Warning",
          "when": "A converted model has a bone with no companion `.skl` and a name in none of the target version's skeleton tables, so its pose is the identity and the mesh it skins sits where the identity puts it."
        },
        {
          "code": "bone_slot_dropped",
          "effect": "The vertex now skins differently. Check the player in the game.",
          "severity": "Warning",
          "when": "A converted mesh's vertex was weighted to a bone slot past the mesh's bone group, so the slot was zeroed and the vertex's weights renormalized."
        },
        {
          "code": "bone_folded_by_position",
          "effect": "Check the player in the game.",
          "severity": "Warning",
          "when": "A converted model uses a bone the target version lacks, with no fold-table entry, so it was folded onto the nearest target body bone by rest position, and that is a guess."
        },
        {
          "code": "material_family_approximated",
          "effect": "The material may render differently. Check the player in the game.",
          "severity": "Warning",
          "when": "No shader rule named a converted material's shader, so its family, and with it the target shader, is the closest fit."
        },
        {
          "code": "material_split_by_flags",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "Meshes of one material carried different flags, so it was split into two materials in the converted model."
        },
        {
          "code": "material_parameter_dropped",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A parameter of the source game's material table that the target shader does not carry was dropped from a converted material."
        },
        {
          "code": "sampler_settings_defaulted",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A `.mtl` sampler name with no stored settings was written with the default settings in the converted model."
        },
        {
          "code": "vertex_bitangents_dropped",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A `.model` mesh's bitangents were dropped on conversion, because the `.fmdl` format has no place for them. The game derives them."
        },
        {
          "code": "dummy_texture_added",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A shaded or metal material with no normal or specular map got the game's dummy texture for the sampler an `.fmdl` requires."
        },
        {
          "code": "native_field_dropped",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A converted model had a non-default field that the target format has no place for, so the field was dropped. It holds nothing the game would show (the model's own local-space bone matrices, a `.skl` parent that the model's own parent wins over, a normal or tangent `w` a `.model` cannot carry)."
        },
        {
          "code": "mesh_flags_dropped",
          "effect": "Check the player in the game.",
          "severity": "Warning",
          "when": "An `.fmdl` mesh flag a `.mtl` cannot express (`no_shadow_cast`) was dropped on conversion to `.model`, so the mesh casts a shadow where the source did not. An `.fmdl` whose meshes are all hidden is left out with `model_hidden_dropped`."
        },
        {
          "code": "model_hidden_dropped",
          "effect": "The file is left out and nothing is built from it, but the folder's other models are still built.",
          "severity": "Info",
          "when": "Every mesh of an `.fmdl` read for PES 2015 to 2017 is hidden (`invisible`), so in PES 2018 to 2021 the game drew nothing of it (a blank head's `oral.fmdl`, a one-sided `Gloves/` folder's other side). A `.common` link naming it is reported the same way."
        },
        {
          "code": "static_bone_added",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "An unskinned mesh was weighted to the `static` bone an `.fmdl` needs."
        },
        {
          "code": "weight_clamped",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A vertex weight above 1 was clamped to the `.fmdl` weight's maximum."
        },
        {
          "code": "empty_mesh_bone_group_dropped",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A mesh with no vertices had its bone group written empty, because it skins nothing."
        },
        {
          "code": "model_gltf_unsupported",
          "effect": "The folder is left out, and a player folder linking it is left out with it (a `Collars/` or `Common/` file is left out alone), even with `pass_through` on. Convert the model to the game's own format.",
          "severity": "Error",
          "when": "A model is given as a `.glb` or `.gltf`, which the compiler does not read yet, with no `.fmdl` or `.model` of the version compiled for beside it (one of the other format is not used instead). It can sit in a player folder, a shared one (reported on each player linking it), `Collars/` or `Common/`. `compile` reports it, `check` does not."
        },
        {
          "code": "model_conversion_failed",
          "effect": "That part (face, boots or gloves) is left out, even with `pass_through` on, but the folder's other parts and textures still build. A collar file is left out alone, so the kits wear the game's own collar.",
          "severity": "Error",
          "when": "A model the compiler must convert cannot be converted or written, or its converted form triggers an Error in its format's checks (the line names the source file)."
        },
        {
          "code": "vertex_too_far_from_origin",
          "effect": "If the export's own file has it, the folder is left out (a `Common/` file is left out alone). If a converted model has it, that part is left out. Either happens even with `pass_through` on.",
          "severity": "Error",
          "when": "A model has a vertex more than 5000 units from the origin, which lags the game for the whole matchday (an export's file is checked as it is, a converted one in its target form)."
        },
        {
          "code": "model_broken",
          "effect": "The folder is left out (a `Common/` file is left out alone), even with `pass_through` on.",
          "severity": "Error",
          "when": "A model file (`.fmdl` or `.model`) cannot be read (the line names the error)."
        },
        {
          "code": "mtl_broken",
          "effect": "The folder is left out (a `Common/` file is left out alone), even with `pass_through` on.",
          "severity": "Error",
          "when": "A `.mtl` does not parse (the finding's line names where in the file, by line and column)."
        },
        {
          "code": "common_model_beaten_dropped",
          "effect": "The file is left out too, and the players linking it get `link_target_dropped`.",
          "severity": "Info",
          "when": "A `Common/` model was passed over for a file of the same name in the other format (a `.model` beside a `.fmdl` on PES 2018 to 2021, the reverse on PES 2015 to 2017), so it was never checked. When the chosen file is left out for a problem of its own, the passed-over one is left out with it, because it would otherwise be used unchecked in its place."
        },
        {
          "code": "fmdl_texture_not_found",
          "effect": "The part is kept, with one line per missing texture: a mesh naming it shows without it.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, one of the model's meshes names a texture that nothing supplies: not in the folder, not at the `./` path the mesh names, and for a Common path neither in `Common/` nor in a CPK the `DpFileList.bin` lists before this compile's own. The texture may still be in an installed CPK that could not be checked."
        },
        {
          "code": "fmdl_texture_kit_path",
          "effect": "Kept with the path unchanged. A part that should follow the kit uses a `team_kit` or `kitN` texture instead.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, a texture path a mesh names points into the game's kit texture folder (`/Assets/pes16/model/character/uniform/texture/`), where the modded exe swaps those files per match, so the model would not show the kit it names."
        },
        {
          "code": "fmdl_common_path_invalid",
          "effect": "The model's folder is left out (for a model in `Common/`, only the file is left out, and the players linking it get `link_target_dropped`). Write the team's Common as `team_common_folder/`, or `team_folder` in the long form. `export-upgrader upgrade` respells an `.fmdl` that names an old folder, in an old export (a Midcup export, for a model on its own).",
          "severity": "Error",
          "when": "An `.fmdl` names a texture in a folder below the game's common folder (`/Assets/pes16/model/character/common/`) other than `team_folder`, the folder that stands for the team's Common, or `sourceimages`, the game's own. Such a folder is `000` (the placeholder of the community's model templates), a team ID, `XXX` or a team's name. It is also reported for a folder the short spellings cannot write: one after `common_folder/` (`common_folder/x/`), or a directory starting `common/`."
        },
        {
          "code": "dummy_kit_name_old",
          "effect": "The model's or `.mtl`'s folder is left out (for a file in `Common/`, only the file is left out). Name the kit stand-in `team_kit` in the model (`team_gk_kit` for a keeper's); `export-upgrader upgrade` respells it in an old export (a Midcup export, for a model on its own).",
          "severity": "Error",
          "when": "A model names the kit stand-in by its old spelling, the game's `dummy_kit` or `dummy_gk_kit` (any letter case, and with any suffix after an `_`, such as `dummy_kit_srm`), in an `.fmdl` texture entry or a `.mtl` texture path, whatever folder the path names. The game's own `dummy_nrm`, `dummy_srm` and `dummy_bsm`, and any other `dummy_` name, are no stand-in."
        },
        {
          "code": "kit_token_old",
          "effect": "The player, Boots or Gloves folder is left out, whichever of its files holds the name or names the path. In `Common/` only that file or folder is left out, and the players linking it get `link_target_dropped`. Spell the token `kit1` to `kit9`, `kitN` or `kitF` (`gk_kit` and the same ending for a keeper's); `export-upgrader upgrade` respells it in an old export (a Midcup export, for a model on its own).",
          "severity": "Error",
          "when": "The name of a file or folder inside a player, Boots or Gloves folder or in `Common/` holds the old kit token: `u0`, three letters or digits, then `p` or `g` and a digit, with `u0`, `p` and `g` in lowercase as shown (`kit6_u0827p0.dds`, `Common/u0XXXp1/`, `u0714g1`). So does a path to a file of the export that a model or `.mtl` there, or a player's own `face.xml`, names. In a player, Boots or Gloves model, a path into the game's folders counts too, except the game's kit texture folder and the folders below its common folder, which use the game's own names. A folder so named is reported once, not each file in it."
        },
        {
          "code": "folder_pack_failed",
          "effect": "An Error leaves the folder out, while a Fatal stops the compile and nothing is written.",
          "severity": "Error or Fatal",
          "when": "A folder's or `Common/`'s files could not be packed for the output, or the run's output could not be written."
        },
        {
          "code": "dpfilelist_missing",
          "effect": "It is an Error when the run installs: the CPK is left in the output folder and the save is not touched. It is a Warning in `--mode test`, `--mode sideload` or with `--no-deploy`, which install nothing, so every bin is built on the bundled base.",
          "severity": "Error or Warning",
          "when": "The game's `download/` folder has no `DpFileList.bin`, so no installed CPK could be checked."
        },
        {
          "code": "installed_bin_unreadable",
          "effect": "The compile stops before any export is read, and the previous CPK is kept.",
          "severity": "Fatal",
          "when": "An installed `DpFileList.bin`, a CPK it lists, or a bin inside one cannot be read or parsed (the line names the file and the whole error)."
        },
        {
          "code": "bin_source",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The line names which installed CPK supplied a file the compile builds on, or `bundled` for the compiler's own copy."
        },
        {
          "code": "template_override_unreadable",
          "effect": "The compile stops before any export is read, and the previous CPK is kept. Fix or remove the file.",
          "severity": "Fatal",
          "when": "A file of the data folder's `templates/` exists but cannot be read, or is a bin that does not parse as its kind (the line names the file and the whole error)."
        },
        {
          "code": "template_override_active",
          "effect": "Nothing to do. The override is used.",
          "severity": "Info",
          "when": "A file of the data folder's `templates/` replaces the compiler's built-in file of its name for the run."
        },
        {
          "code": "bin_header_repaired",
          "effect": "The headers are rewritten and each entry's other bytes kept, so those teams' colors may be wrong until their exports are compiled again.",
          "severity": "Warning",
          "when": "A team color or kit color file the compile builds on held entries whose header was not their position's (a damaged installed bin, whatever wrote it). The line names the file and the teams."
        },
        {
          "code": "duplicate_path",
          "effect": "The `overrides/` file wins and the built one is left out.",
          "severity": "Warning",
          "when": "A file built for the CPK lands at the path of a file of the data folder's `overrides/` (the line names the path)."
        },
        {
          "code": "cpk_write_failed",
          "effect": "The compile stops. The partial file is discarded and the previous CPK stays as it was.",
          "severity": "Fatal",
          "when": "The CPK could not be written."
        },
        {
          "code": "output_commit_failed",
          "effect": "The compile stops, and the previous output stays as it was.",
          "severity": "Fatal",
          "when": "A finished CPK or output folder could not be moved into its final place."
        },
        {
          "code": "patch_written",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The run wrote `aesthetics_patch.toml` to the output folder (the line names the path and the teams it covers)."
        },
        {
          "code": "patch_write_failed",
          "effect": "The compile stops. A deployment is rolled back, putting the old CPKs back in `download/`, and nothing more is put in place. The files already written to `livecpk` stay in place.",
          "severity": "Fatal",
          "when": "The `aesthetics_patch.toml` could not be written (the line names the path and the error)."
        },
        {
          "code": "savefile_autodetected",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The `savefile_path` setting at `auto` found your save under `Documents\\KONAMI` (the line names it). When there are several account folders, the newest is used."
        },
        {
          "code": "savefile_missing",
          "effect": "The save step is skipped. Apply the patch by hand with `UTW save-editor apply-patch`.",
          "severity": "Warning",
          "when": "The run's patch holds entries (each compiled player's look and his boots and gloves IDs, and his name and shirt name for an export marked `write_names`) and no savefile is configured or found. A configured path naming a folder is `savefile_invalid` instead."
        },
        {
          "code": "savefile_skipped_pes_running",
          "effect": "The save step is skipped, because the game writes the save it holds over the file when you leave its Edit mode after changing anything there. Close the game's Edit mode, then compile again from a terminal with `--edit-mode-closed`: the save is then written, and opening Edit mode reads it in without restarting the game. The window's Compile button cannot add the flag, so from the window, close the game and compile again.",
          "severity": "Warning",
          "when": "Savefile changes are pending and the PES version the run compiled for is running, but `compile` was not given `--edit-mode-closed` (another version running does not count). The line names the game and the flag."
        },
        {
          "code": "savefile_write_failed",
          "effect": "It is Fatal after a deployment, which is rolled back and its CPKs are left in the output folder. In `--mode sideload` it is an Error, with the new `livecpk` files in place.",
          "severity": "Error or Fatal",
          "when": "The savefile could not be updated, because the patch could not be applied or the file could not be written back (the line names the path and the error)."
        },
        {
          "code": "savefile_invalid",
          "effect": "It is Fatal after a deployment, which is rolled back and its CPKs are left in the output folder. In `--mode sideload` it is an Error, with the new `livecpk` files in place. The save is not modified.",
          "severity": "Error or Fatal",
          "when": "The configured or found savefile cannot be read, or it is for a PES version other than the one compiled for."
        },
        {
          "code": "savefile_written",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The patch was applied to your save, which was replaced, with the previous file kept beside it as `.bak`."
        },
        {
          "code": "savefile_team_missing",
          "effect": "That team's entries are not applied, but the rest are. A save holding none of the patch's players is not rewritten.",
          "severity": "Error",
          "when": "A team the patch covers is not in the save."
        },
        {
          "code": "savefile_player_missing",
          "effect": "His entries are not applied, but the rest are.",
          "severity": "Error",
          "when": "A player the patch covers is not in the save, though his team is (the save editor's `apply-patch` reports the same skip)."
        },
        {
          "code": "deploy_skipped_by_flag",
          "effect": "Nothing to do. The save step is skipped and the run is clean.",
          "severity": "Info",
          "when": "`--no-deploy` was given, so the CPK was left in the output folder instead of being installed (one line per CPK with `multicpk_mode` on)."
        },
        {
          "code": "pes_folder_not_found",
          "effect": "Deployment is skipped, so the CPK is left in the output folder and the save is not touched. Set `PES folder` to the game's install folder.",
          "severity": "Error",
          "when": "The run installs, but `PES folder` does not point at an existing folder (no PES installed, or the setting is wrong)."
        },
        {
          "code": "pes_version_mismatch",
          "effect": "The CPK is still installed. Check the two settings.",
          "severity": "Warning",
          "when": "The `PES folder` holds no exe of the version compiled for (`PES2021.exe` for PES 2021), which suggests a wrong `PES version` or `PES folder` setting."
        },
        {
          "code": "dpfilelist_not_official",
          "effect": "A compile goes ahead anyway. Upgrade DpFileList, in the Compiler's toolbar, or `UTW compiler upgrade-dpfl` installs the official list.",
          "severity": "Warning",
          "when": "The game's `DpFileList.bin` differs from the cup's official list (it is missing entries, it has entries the official list does not, or the order is different), while every CPK of this compile is listed."
        },
        {
          "code": "dpfilelist_cpk_missing",
          "effect": "A compile goes ahead anyway, but the save step is skipped. Upgrade DpFileList, in the Compiler's toolbar, or `UTW compiler upgrade-dpfl` writes a placeholder for each official entry missing its file.",
          "severity": "Warning",
          "when": "The game's `DpFileList.bin` names CPKs, other than this compile's own, with no file in `download/`, so the game rejects the whole `download/` folder and loads none of its CPKs."
        },
        {
          "code": "dpfilelist_outdated",
          "effect": "A compile stops before compiling anything: nothing is installed or written, and the save is not touched. In the Compiler's window, Compile stays disabled until Upgrade DpFileList installs the official list; in a terminal, `UTW compiler upgrade-dpfl` installs it, then compile again.",
          "severity": "Fatal",
          "when": "The cup's official list names one or more CPKs of this compile that the game's older `DpFileList.bin` lacks (the line names them)."
        },
        {
          "code": "dpfilelist_cpk_renamed",
          "effect": "Nothing to do. The game still loads it under the official list.",
          "severity": "Info",
          "when": "The DpFileList upgrade renamed a CPK of an older DLC to the official name of its kind."
        },
        {
          "code": "dpfilelist_placeholder_written",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The DpFileList upgrade wrote the empty placeholder CPK for an official entry with no file in `download/`."
        },
        {
          "code": "dpfilelist_replaced",
          "effect": "The old list is kept beside it as `DpFileList.bin.bak`, replacing an older backup.",
          "severity": "Info",
          "when": "The DpFileList upgrade replaced the game's `DpFileList.bin` with the official one."
        },
        {
          "code": "dpfilelist_cpk_dropped",
          "effect": "The game no longer loads it. If its file is still in `download/`, it is yours to remove, and when a deletion you ticked failed, the line says why.",
          "severity": "Warning",
          "when": "The DpFileList upgrade could not rename a CPK of the old list that the official one does not have (no official name of its kind is left, its name is not of the `4cc_NN_kind.cpk` shape, or the official name it would take is in use). The line names its size when its file is in `download/`."
        },
        {
          "code": "dpfilelist_cpk_deleted",
          "effect": "Nothing to do: the game would no longer have loaded it.",
          "severity": "Info",
          "when": "Upgrade DpFileList deleted a CPK the official list does not name, because you ticked its box in the dialog."
        },
        {
          "code": "dpfilelist_up_to_date",
          "effect": "Nothing is written.",
          "severity": "Info",
          "when": "The DpFileList upgrade found that the game's `DpFileList.bin` already is the official one, and every CPK it names is in `download/`."
        },
        {
          "code": "dpfilelist_upgrade_planned",
          "effect": "Nothing is written. Run it again with `--yes` to do them.",
          "severity": "Info",
          "when": "`upgrade-dpfl` ran without `--yes`, so the lines before this one are what `--yes` would do."
        },
        {
          "code": "cpk_name_unlisted",
          "effect": "A compile stops before compiling anything: nothing is installed or written, and the save is not touched. Set `cpk_name` to a name the list has; then compile again.",
          "severity": "Fatal",
          "when": "One or more of this compile's CPK names are in neither the game's `DpFileList.bin` nor the cup's official list (the line names them)."
        },
        {
          "code": "cpk_slots_exhausted",
          "effect": "The compile stops and no CPK is written. Raise `cpk_part_max_size`, or ask for a list with more slots.",
          "severity": "Fatal",
          "when": "With `multicpk_mode` on, the teams do not fit the numbered CPK slots the official list has, at `cpk_part_max_size` each (the line names the first export left without room and how many slots there are)."
        },
        {
          "code": "cpk_team_exceeds_cap",
          "effect": "The compile stops and no CPK is written. Raise `cpk_part_max_size`.",
          "severity": "Fatal",
          "when": "With `multicpk_mode` on, one team's content alone is larger than `cpk_part_max_size`, and a team is never split."
        },
        {
          "code": "cpk_size_over_limit",
          "effect": "It is written whole and the game loads it, but the cup DLC's repository cannot hold a file that large.",
          "severity": "Warning",
          "when": "The single output CPK is larger than `cpk_part_max_size`."
        },
        {
          "code": "old_cpk_locked",
          "effect": "Deployment is skipped, so the CPK is left in the output folder and the save is not touched. Close PES and compile again.",
          "severity": "Error",
          "when": "The old CPK in `download/` could not be replaced, most likely because PES is running."
        },
        {
          "code": "deploy_target_unwritable",
          "effect": "Deployment is skipped, so the CPK is left in the output folder and the save is not touched. Run Unlimited Tool Works as administrator and compile again.",
          "severity": "Error",
          "when": "The `download/` folder, or the old CPK in it, cannot be written (usually a game installed under `Program Files`, which needs administrator rights)."
        },
        {
          "code": "overrides_active",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "The data folder's `overrides/` holds files, and each goes into the CPK at its path under the folder (into `livecpk\\` in `--mode sideload`)."
        },
        {
          "code": "teamnotes_write_failed",
          "effect": "The run's CPK stays in place, and the notes file is left as it was.",
          "severity": "Error",
          "when": "`teamnotes.txt` could not be written, or a previous one could not be removed, in the output folder."
        },
        {
          "code": "source_changed_during_run",
          "effect": "The compile stops, so the output it was building is discarded and nothing is installed. Run it again.",
          "severity": "Fatal",
          "when": "A file the run read is gone, or its size or modified time changed since the export was listed (for an archive, the archive file's)."
        },
        {
          "code": "fmdl_mesh_over_bone_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a mesh of an `.fmdl` is over a hard bone limit of the format, so the game cannot load it."
        },
        {
          "code": "fmdl_mesh_over_vertex_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a mesh of an `.fmdl` is over a hard vertex limit of the format, so the game cannot load it."
        },
        {
          "code": "fmdl_mesh_over_face_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a mesh of an `.fmdl` is over a hard face limit of the format, so the game cannot load it."
        },
        {
          "code": "fmdl_face_index_out_of_range",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a face of an `.fmdl` names a vertex that does not exist."
        },
        {
          "code": "fmdl_mesh_unassigned",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, no mesh group of an `.fmdl` lists one of its meshes, so the file would not contain it."
        },
        {
          "code": "fmdl_bone_slot_out_of_range",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, a vertex is weighted to a bone slot past the mesh's bone group, so the game ignores that weight (the game's own models do this)."
        },
        {
          "code": "fmdl_mesh_empty",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, a mesh of an `.fmdl` has no faces."
        },
        {
          "code": "fmdl_duplicate_bone_name",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2018 to 2021, an `.fmdl` has two bones of one name, which makes merges and `.skl` matching ambiguous."
        },
        {
          "code": "fmdl_weights_not_normalized",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2018 to 2021, a vertex's weights sum to neither 255 nor 0."
        },
        {
          "code": "fmdl_material_unused",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2018 to 2021, an `.fmdl` has a material that no mesh uses."
        },
        {
          "code": "model_mesh_over_bone_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a mesh of a `.model` is over a hard bone limit of the format, so the game cannot load it."
        },
        {
          "code": "model_mesh_over_vertex_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a mesh of a `.model` is over a hard vertex limit of the format, so the game cannot load it."
        },
        {
          "code": "model_mesh_over_face_limit",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a mesh of a `.model` is over a hard face limit of the format, so the game cannot load it."
        },
        {
          "code": "model_face_index_out_of_range",
          "effect": "The folder is left out (a `Common/` file is left out alone), but `pass_through` packs it as it is.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a face of a `.model`, at any detail level, names a vertex that does not exist."
        },
        {
          "code": "model_bone_slot_out_of_range",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017, a vertex is weighted to a bone slot past the mesh's bone group."
        },
        {
          "code": "model_mesh_empty",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017, a mesh of a `.model` has no faces."
        },
        {
          "code": "model_duplicate_bone_name",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017, a `.model` has two bones of one name."
        },
        {
          "code": "model_lod_record_mismatch",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "In PES 2015 to 2017, a `.model`'s level-of-detail record disagrees with its meshes' levels."
        },
        {
          "code": "model_weights_not_normalized",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2015 to 2017, a vertex's weights do not sum to 1."
        },
        {
          "code": "model_degenerate_face",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2015 to 2017, a face of a `.model` uses one vertex twice."
        },
        {
          "code": "model_material_unused",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "In PES 2015 to 2017, a `.model` has a material name that no mesh uses."
        },
        {
          "code": "mtl_material_duplicate",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `.mtl` lists one material twice."
        },
        {
          "code": "mtl_state_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `.mtl` state value is out of range (a `ztest` other than 1, or a `blendmode` or `alphablend` other than 0 or 1)."
        },
        {
          "code": "mtl_blendmode_nonzero",
          "effect": "Nothing to do.",
          "severity": "Warning",
          "when": "A `.mtl` sets `blendmode` to 1 (it works, but is not recommended)."
        },
        {
          "code": "mtl_state_missing",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A `.mtl` has no state names, so the defaults are used."
        },
        {
          "code": "mtl_state_nonrecommended",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "A `.mtl`'s `alphablend` and `zwrite` combination is not recommended."
        },
        {
          "code": "mtl_state_unknown",
          "effect": "Kept as written. Nothing to do.",
          "severity": "Info",
          "when": "A `.mtl` has a state name that is not one of the seven the compiler knows (`shadowcaster`, which some of the game's own files carry)."
        },
        {
          "code": "model_material_undefined",
          "effect": "The folder is left out. `pass_through` packs it as it is, but only when a `.mtl` was found.",
          "severity": "Error",
          "when": "A `.model` uses a material name its `.mtl` does not define (the mesh would render with the game's fallback), or no `.mtl` was found for it at all."
        },
        {
          "code": "nested_folders_fixed",
          "effect": "The export is read as if the extra level were not there. Repack the export without it.",
          "severity": "Warning",
          "when": "The export's content sits one level too deep, inside a single folder."
        },
        {
          "code": "nested_root_ambiguous",
          "effect": "The export is not compiled. Keep one root.",
          "severity": "Error",
          "when": "Several folders inside the export each look like its root."
        },
        {
          "code": "nested_root_conflict",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "A file at the export's root and one inside the nested folder resolve to the same place."
        },
        {
          "code": "players_txt_invalid",
          "effect": "The export is not compiled. An empty team roster stays valid for a kit-only export.",
          "severity": "Error",
          "when": "`players.txt` cannot be read as a roster (an encoding failure or a roster-wide ambiguity), or a referee roster is left with no valid assignment."
        },
        {
          "code": "refs_txt_ignored",
          "effect": "`players.txt` is used and `refs.txt` is ignored.",
          "severity": "Warning",
          "when": "A referee export holds both `players.txt` and the old `refs.txt` name."
        },
        {
          "code": "source_read_failed",
          "effect": "An optional root file that cannot be read is left out. When a folder's file cannot be read, the folder is left out, and when required metadata cannot be read or the source is unusable, the export is left out. A failure of the run's own output stops the compile.",
          "severity": "Error or Fatal",
          "when": "A file of the export could not be read when it was needed."
        },
        {
          "code": "export_empty",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "No usable content was found at the export's root."
        },
        {
          "code": "team_name_unknown",
          "effect": "The export is not compiled. Check the name against the `teams_list_path` file, or add the team to it.",
          "severity": "Error",
          "when": "The export's first word names no team in the `teams_list_path` file."
        },
        {
          "code": "export_tag_missing",
          "effect": "The export is not compiled. Rename it `<team> Full ...` or `<team> Midcup ...`.",
          "severity": "Error",
          "when": "A team export's name has neither `Full` nor `Midcup` as its second word."
        },
        {
          "code": "export_layout_old",
          "effect": "The export is not compiled, and nothing else is reported about it. Run it through the Export upgrader once.",
          "severity": "Error",
          "when": "The export is in the old layout, which the compiler does not read: a root folder named `Kit Configs`, `Kit Textures` or `Other` (kits live in `Kits/`)."
        },
        {
          "code": "players_txt_missing",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "A referee export has no `players.txt` (nor the old `refs.txt` name)."
        },
        {
          "code": "players_txt_line_invalid",
          "effect": "That line of `players.txt` is ignored.",
          "severity": "Error",
          "when": "A nonblank line of `players.txt` does not give a slot number and a folder name."
        },
        {
          "code": "players_txt_slot_invalid",
          "effect": "That line of `players.txt` is ignored.",
          "severity": "Error",
          "when": "A `players.txt` line's slot is outside 01-23 (teams) or 01-35 (refs)."
        },
        {
          "code": "players_txt_slot_duplicate",
          "effect": "The export is not compiled.",
          "severity": "Error",
          "when": "`players.txt` assigns the same slot more than once, so the roster is ambiguous."
        },
        {
          "code": "players_txt_target_missing",
          "effect": "That line of `players.txt` is ignored.",
          "severity": "Error",
          "when": "A `players.txt` line names a folder that does not exist."
        },
        {
          "code": "player_unlisted",
          "effect": "The folder is ignored.",
          "severity": "Warning",
          "when": "`players.txt` is present but does not list this player folder, whatever number its name carries."
        },
        {
          "code": "player_folder_number_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "Without `players.txt`, a player folder's number is not in 01-23."
        },
        {
          "code": "player_number_duplicate",
          "effect": "Both folders are left out.",
          "severity": "Error",
          "when": "Without `players.txt`, two player folder names claim the same slot."
        },
        {
          "code": "file_type_disallowed",
          "effect": "With `strict_file_type_check` on, this is an Error and the folder is left out (in `Collars/`, only the file is left out). With it off, it is an Info and nothing is left out.",
          "severity": "Error or Info",
          "when": "A file sits where its kind is not allowed, anywhere in a player or shared folder, in a kit folder, or in `Collars/`: its extension is disallowed, or, below a subfolder, it is a marker, a `settings.toml` or a per-player file."
        },
        {
          "code": "common_file_disallowed",
          "effect": "With `strict_file_type_check` on, this is an Error and the file is left out. With it off, it is an Info and nothing is left out.",
          "severity": "Error or Info",
          "when": "A file sits in `Common/` that is not model content (a model, `.mtl`, `.skl`, `.fclo`, `.xml`, `.bin`, texture or `.txt` note would be)."
        },
        {
          "code": "player_layout_proto",
          "effect": "The folder is left out, and nothing else is reported about it. Run the export through the Export upgrader once.",
          "severity": "Error",
          "when": "A referee export's player folder has a `face/`, `boots/`, `gloves/` or `common/` subfolder directly inside, the layout of older referee exports (in a team export those are ordinary subfolders)."
        },
        {
          "code": "shared_link_duplicate",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A player folder has more than one shared link for a part (face, boots or gloves), counted over the whole folder, subfolders included. Two links naming one folder count once."
        },
        {
          "code": "link_target_missing",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A link file names a shared folder that does not exist."
        },
        {
          "code": "common_link_missing",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "A `.common` link names a file missing from `Common/` (a `kitN` name or folder stands for its set). In PES 2018 to 2021, a texture link is satisfied when a CPK listed before this compile's own holds it at the team's Common path."
        },
        {
          "code": "link_target_dropped",
          "effect": "The folder is left out, and `pass_through` does not apply (it is not reported when the setting keeps the target).",
          "severity": "Error",
          "when": "A shared-folder or `.common` link names a target that was left out for its own finding (the line names the link, the target and the finding)."
        },
        {
          "code": "shared_folder_orphaned",
          "effect": "The folder is ignored.",
          "severity": "Warning",
          "when": "A shared `Faces/`, `Boots/` or `Gloves/` folder is linked by no player."
        },
        {
          "code": "fpc_conflict",
          "effect": "The folder is left out. Keep one marker.",
          "severity": "Error",
          "when": "A player folder holds both `fpc_on` and `fpc_off`."
        },
        {
          "code": "ingame_face_marker_renamed",
          "effect": "The file is ignored, and the folder compiles as if the file were not there (a blank face package unless the folder gives a face). Rename it `ingame_face.toml`: an empty one gives the game's default face.",
          "severity": "Error",
          "when": "A player folder holds `ingame_face` or `ingame_face.txt`, the in-game face marker's old name. The marker is now `ingame_face.toml`, which also holds the in-game face's keys."
        },
        {
          "code": "ingame_face_explicit_face_model",
          "effect": "The player folder is left out.",
          "severity": "Error",
          "when": "A player folder holds `ingame_face.toml` beside an explicit face model (`face_high`, `hair_high` or `oral`, in any format), a `.common` link to one, or a `Faces/` link, and any of them needs the face folder the marker suppresses."
        },
        {
          "code": "texture_stem_conflict",
          "effect": "The folder is left out (the model folder or the kit, or `all/` itself, so no kit inherits from it). In `Portraits/` and `Common/` the two files are left out.",
          "severity": "Error",
          "when": "Two image files share one stem in the same place, whatever their extensions (`hair.dds` beside `hair.png`, or a texture link `hair.png.common`; `hair.dds` beside `jessie/hair.dds` is no conflict). The place is a model folder, a kit folder, `Kits/all/`, a folder of `Common/`, or `Portraits/` (two files of one slot)."
        },
        {
          "code": "fmdl_name_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a model anywhere in a shared `Boots/` or `Gloves/` folder, or one a `.common` link there stands for, has a name that does not resolve to that category's allowed names."
        },
        {
          "code": "model_name_invalid",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a model anywhere in a shared `Boots/` or `Gloves/` folder lacks that category's suffix."
        },
        {
          "code": "edithair_unsupported",
          "effect": "The folder is left out.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a `face_edithair.xml` or `hair.xml` sits anywhere in a player or shared folder."
        },
        {
          "code": "kit_folder_invalid",
          "effect": "The kit is left out.",
          "severity": "Error",
          "when": "A `Kits/` subfolder's name is not `<slot>` or `<slot> - <label>` with the slot in `p1`-`p9`, `g1` or `all`."
        },
        {
          "code": "kit_slot_duplicate",
          "effect": "Both kits are left out; neither is picked.",
          "severity": "Error",
          "when": "Two `Kits/` subfolders resolve to the same slot (`p1/` and `p1 - Lakers/`)."
        },
        {
          "code": "kit_texture_name_invalid",
          "effect": "The file is left out.",
          "severity": "Error",
          "when": "A kit texture's name does not carry the `kit` prefix (`kit.dds`, `kit_mask.dds`)."
        },
        {
          "code": "kit_layout_conflict",
          "effect": "The kit is left out. Keep one marker.",
          "severity": "Error",
          "when": "A kit folder holds both `pre-fox` and `fox` markers."
        },
        {
          "code": "kit_icon_invalid",
          "effect": "The default icon (3) is used.",
          "severity": "Warning",
          "when": "A kit's `icon_<N>` marker is numbered above 23, or the kit holds two or more (each is reported)."
        },
        {
          "code": "kit_all_file_ignored",
          "effect": "The file is ignored.",
          "severity": "Warning",
          "when": "`Kits/all/` holds something other than kit textures (`config.toml`, `colors.txt`, an `icon_<N>` marker, anything else)."
        },
        {
          "code": "kit_all_unused",
          "effect": "Nothing to do. It is not used.",
          "severity": "Warning",
          "when": "`Kits/all/` is present but no kit folder inherits from it."
        },
        {
          "code": "kit_textures_inherited",
          "effect": "The textures are inherited from `all/`. Nothing to do.",
          "severity": "Info",
          "when": "A kit lacks texture names that `Kits/all/` provides (the line lists them)."
        },
        {
          "code": "portrait_name_invalid",
          "effect": "The file is left out.",
          "severity": "Error",
          "when": "A `Portraits/` file is not a texture named `NN` with NN in 01-23 (the `NN` alone, or followed by ` - ` and any text)."
        },
        {
          "code": "portrait_missing",
          "effect": "The player is compiled without a portrait. Add a `portrait` image to his folder, or one named by his number (`05`) to `Portraits/`.",
          "severity": "Warning",
          "when": "In PES 2017 to 2021, a player has no portrait in his folder or in `Portraits/`; in a `Midcup` export, the CPKs installed before the one being compiled hold none for him either."
        },
        {
          "code": "logo_missing",
          "effect": "The export is compiled without a logo. Add one at its root, or rename the export `<team> Midcup ...` if it only adds to what is installed.",
          "severity": "Error",
          "when": "A `Full` export has no logo: no file at its root whose name starts with `logo`."
        },
        {
          "code": "logo_file_invalid",
          "effect": "No logo is built for the team: the `logo*` and `logo_small*` files are left out together, because the game needs all three logo sizes.",
          "severity": "Error",
          "when": "A root `logo*` file has an unknown tag in its name, is in a format the compiler does not read, or does not decode."
        },
        {
          "code": "logo_role_duplicate",
          "effect": "No logo is built for the team.",
          "severity": "Error",
          "when": "Two files claim the same logo role (two `logo*`, or two `logo_small*`)."
        },
        {
          "code": "logo_small_without_main",
          "effect": "No logo is built, because the small image is not a source for the large sizes.",
          "severity": "Error",
          "when": "A `logo_small*` image is present with no main `logo*`."
        },
        {
          "code": "root_file_unexpected",
          "effect": "The file or folder is ignored.",
          "severity": "Warning",
          "when": "An unknown file or folder sits at the export's root (a stale `wrapper/` beside a usable root included), or a file sits directly inside `Players/`, `Kits/`, `Faces/`, `Boots/` or `Gloves/`, all of which hold only folders."
        },
        {
          "code": "notes_found",
          "effect": "`compile` gathers it into `teamnotes.txt` in the output folder.",
          "severity": "Info",
          "when": "The export's root holds a non-empty `notes.txt`."
        },
        {
          "code": "notes_encoding_invalid",
          "effect": "The note is left out; the export otherwise compiles.",
          "severity": "Error",
          "when": "The root `notes.txt` is not valid UTF-8."
        }
      ],
      "title": "The Compiler",
      "topics": [
        {
          "body": "The Compiler also runs without its window, from a terminal opened in the folder that holds\n`UTW.exe`:\n\n```text\nUTW compiler check [exports-root] [--export <path>]...\nUTW compiler compile [exports-root] [--mode normal|test|sideload] [--export <path>]... [--no-deploy] [--edit-mode-closed]\nUTW compiler upgrade-dpfl [--yes]\n```\n\n`check` reads your exports and reports what it finds, without writing anything. `compile`\nreports the same findings, plus the few only a build can find (a model that fails to convert,\nfor one), and builds the CPK `<cpk_name>.cpk`, named by the `cpk_name` setting, in the output\nfolder (`output/` beside `UTW.exe` unless you changed the `output_folder_path` setting).\n\nWhile either command runs in a terminal window, a line under the findings counts the exports\ndone, and, while `compile` builds, the parts built so far. When a compile ends, one line sums\nit up: the files written, the exports and folders left out, and the folders with errors.\n\nThe CPK is written in full before it replaces the previous one, so `<cpk_name>.cpk` is never\nhalf-written. A run that finds nothing to compile, with no file in the `overrides` folder\n(below), writes nothing and leaves the previous CPK as it was. If a file of an export changes\nwhile `compile` runs (someone saves over it, or replaces the archive), the run stops with the\nline `source_changed_during_run` naming the file. It writes nothing, leaves the previous CPK in\nplace and ends with exit code 3: run it again.\n\n**What `compile` builds**\n\n`compile` builds exports for PES 2015 to 2021: the models of the players in the roster, kept in\neach player's own folder (face models, boots and gloves), with their textures, their portraits\nand kits, and everything else this page describes.\n\nA model is boots when its name is `boots` or ends in `_boots`, such as `kit_boots.fmdl`\n(`redboots.fmdl` is not boots). It is gloves when its name is `glove_l`, `glove_r`, `handL` or\n`handR`, or ends in `_` and one of them. A name may also carry a kit number\n(`boots_kit1.fmdl`, `glove_l_kit2.fmdl`).\n\n**Portraits**\n\nA portrait is a `portrait` image in the player's folder, or an image named `NN` in the\n`Portraits` folder, numbered 01 to 23 as a player folder is (`05.dds`, or `05 - Bob.dds` with a\nname after ` - `). A file in `Portraits` that is not a texture named so, or any file in a\nsubfolder of it, is `portrait_name_invalid` and is left out. A player with both gets one\nportrait when the two files are identical. Two that differ are the line `portrait_conflict`,\nand the whole export is skipped, since the compiler cannot tell which one is meant.\n\nOn PES 2017 to 2021, a player with no portrait in either place is the warning\n`portrait_missing`, and he is compiled without one. In a `Midcup` export he is reported only\nwhen the CPKs installed before the one being compiled hold no portrait for him either.\n\n**Textures**\n\nA texture (a player's, a shared folder's, a Common one, a kit's or a portrait) can be a\n`.dds`, `.ftex`, `.png`, `.jpg`, `.bmp`, `.webp`, `.tga`, `.tif` or `.jxl` file (`.jpeg` and\n`.tiff` work too). Each one is converted to a format the chosen PES version reads: BC3, or BC1\nwhen the image has no transparency at all.\n\nA `.dds` or `.ftex` stored uncompressed stays uncompressed on every version, with two\nexceptions: a normal map (below), and a kit's main `kit` texture on PES 2015 to 2017, since\nthose versions write the season's logos into it in memory and crash on an uncompressed one.\nA `.dds` or `.ftex` already in a format the version reads keeps its compressed blocks, packed\ninto the version's own container (a `.dds` as an `.ftex` on PES\n2018 to 2021, a `.ftex` as a `.dds` on PES 2015 to 2017). Whatever its codec, a `.dds` or\n`.ftex` keeps the mip levels it brings (the smaller copies of itself the game needs), and an\nimage of any other kind has its mip levels generated for it.\n\nA portrait is the exception: the game reads it as a `.dds` on every version, so a `.dds`\nportrait is kept as it is. A `.dds` portrait under a DX10 header gets the header the game reads\nfor its format, its pixels unchanged, reported as `portrait_header_rewritten`, except an\nuncompressed one no other header can describe, which is converted. A portrait in any other\nformat becomes a BC3 `.dds` of the same size, its mip levels made anew. For PES 2015 to 2018,\nwhich cannot read BC4, BC5 or BC7, a `.dds` portrait in one of those formats, under either\nheader, is converted like any other image.\n\nA texture whose name ends in `_nrm` (`skin_nrm.png`), or in `_nrm` and a kit number\n(`pants_nrm_kit1.png`), is a normal map. It is always compressed, even when stored\nuncompressed: the game reads a normal map in the compressed layout.\n\n**One texture or model per kit**\n\nA texture named with `kit1` to `kit9` (`pants_kit1.dds`, `pants_kit2.dds`) is the one the game\nshows when that kit is picked before a match. The model that uses it names `pants_kitN`\ninstead, and the game puts the picked kit's number in place of the `N`. A texture named `kitF`\n(`pants_kitF.dds`) is the set's fallback, the file a kit number with no variant of its own\ngets. It is never used under its own name, so a model naming `pants_kitF` finds nothing.\n\nOld exports named a kit's textures with `u0`, three letters or digits, then `p` (or `g`, the\ngoalkeeper's) and a digit (`kit6_u0827p0.dds`, `u0714g1`). That spelling is refused with the\nerror `kit_token_old`: a player, Boots or Gloves folder holding it in a file or folder name,\nor in a path one of its models, `.mtl` files or its `face.xml` names, is left out, and in\n`Common/` only that file or folder is. Write `kit1` to `kit9`, `kitN` or `kitF` instead\n(`gk_kit` and the same ending for a goalkeeper's); `export-upgrader upgrade` respells it in\nan old export.\n\nWhen the team has a kit number (a `p1` to `p9` folder in `Kits`, and the goalkeeper's `g1` goes\nwith the number picked) with no texture of its own, it gets the set's fallback (`kitF`, else\nthe lowest variant). For PES 2018 to 2021 the fallback is written once as `pants_kitN`. For PES\n2015 to 2017 it is copied under the missing number's name, so the game never shows a missing\ntexture. The warning `kit_variant_missing` names the texture, the kit and the copied file, and\nfires only when the set has no `kitF`.\n\nPer-kit models (`pants_kit1.fmdl` beside `pants_kit2.fmdl`, or `.model` files, or a mix of the\ntwo) need a `face.xml` to name them. On PES 2018 to 2021, and on PES 2015 to 2017 in a shared\nboots or gloves folder and in a folder holding `ingame_face.toml`, the set's fallback is used\n(a model in the other game's format, converted, or a `.model` with its `.mtl`) and the other\nvariants are left out. The warning `kit_variant_model_left_out` says so.\n\nOn PES 2015 to 2017 a face's set is listed once in the `face.xml`, as `pants_kitN`. Every\nvariant is packed, and the game loads the one for the kit picked. Each variant's `.mtl` goes by\nthe same rule (`pants_kit1.mtl` and `pants_kit2.mtl`, or one `pants.mtl` for all), and the\nwarning `kit_variant_mtl_differs` names a variant whose `.mtl` would not be found that way. A\nkit number the team has with no model of the set gets the fallback's model copied for it, as a\ntexture does (its `.mtl` too when that is the variant's own, `pants_kit1.mtl`), so the game\nnever shows the player without that part. The `kitF` model and its `.mtl` are never packed\nunder their own names, and the warning `kit_variant_missing` names the set, the kit and the\ncopied model only when there is no `kitF`. The same holds for a set a player's own `face.xml`\nnames.\n\n**Textures that cannot be used**\n\nSix lines name a texture that cannot be used, each with the file:\n\n- `texture_too_small`: a side under 4 pixels.\n- `texture_not_div4`: for PES 2015 to 2017, a width or height that is not a multiple of 4\n  (1000 is one, 1002 is not). A texture that stays uncompressed (see **Textures**) does not\n  get this finding.\n- `texture_not_pow2`: a portrait whose width or height is not a power of two (256, 512,\n  1024, ...), or, for PES 2018 to 2021, a texture with mip levels when neither its width nor its\n  height is. A `.png` or other image always gets mip levels, and a `.dds` or `.ftex` with a\n  single level passes at any size.\n- `kit_texture_too_big`: for PES 2015 to 2017, a kit's main `kit` texture, its own or the one\n  from `all`, wider or taller than 2048 pixels or with a side that is not a power of two.\n- `texture_type_mismatch`: a file renamed to another format instead of resaved, such as a PNG\n  called `skin.dds`.\n- `texture_codec_unsupported`: a `.dds` or `.ftex` in a format the compiler does not read (BC6H\n  for one), or a 16-bit or interleaved `.tga`. Resave it.\n\nBoth commands report them from each file's header alone, so `check` finds them too, except a\ncodec that only the conversion finds unreadable (BC6H): `compile` reports that one. What is left\nout depends on where the texture is:\n\n- a player's or shared folder's texture leaves out the whole folder;\n- a kit's texture leaves out the whole kit (a texture in `all` leaves out every kit using it);\n- a `Common` texture leaves out only that file: the rest of `Common` is still built, and a\n  player linking that file is left out with the line `link_target_dropped`;\n- a portrait leaves out only that portrait.\n\nA `Common` texture that `compile` fails to convert, after those checks\n(`texture_codec_unsupported`, or `folder_pack_failed` for a file cut short past its header),\nleaves out every texture of its folder (`Common/` itself, or the subfolder it is in).\n\nThe `pass_through` setting keeps a folder or file with errors instead of leaving it out, though\nsome errors leave it out even with the setting on. With `pass_through` on, a texture with\n`texture_too_small`, `texture_not_pow2` or `kit_texture_too_big`, a portrait included, is kept\nand converted as it is. A renamed one is still left out, and so is one with `texture_not_div4`,\nwhich PES 2015 to 2017 cannot load at all, or with `texture_codec_unsupported`.\n\nA texture whose header cannot be read at all (not a texture, or cut short before its header\nends) is the line `texture_unreadable`. It is left out the way a texture with\n`texture_too_small` is, but `pass_through` does not keep it. A texture that cannot be packed for\nany other reason is reported as `folder_pack_failed`, naming the file, and its folder is left\nout.\n\n**The team's logo**\n\nThe team's logo is one image at the export's root named `logo` (`logo.png`), in any of the\nimage formats above and of any size: `compile` makes the game's three logo sizes (512, 256 and\n128 pixels square) from it. An image named `logo_small` beside it is used for the smallest size\ninstead, when you want a different picture there.\n\nEvery team needs its logo, so a `Full` export with no `logo*` file at its root is the error\n`logo_missing`, and the export is compiled without one.\n\nAn image that is not square is fitted inside a transparent square, unless its name ends in\n`_crop` (`logo_crop.png`: the middle square is kept and the sides are cut off) or `_stretch`\n(the whole image is stretched to a square). `_fit` spells the default out. The note\n`logo_fit_applied` says which was done. The warning `logo_upscaled` names an image the compile\nenlarges to the size it is made into, which makes it less sharp.\n\nA `logo` image at the export's root that is not a readable image, or whose name carries a tag\nthe compiler does not know (`logo_foo.png`, `logo2.png`), is the line `logo_file_invalid`, from\nboth commands, and the logo is left out even with `pass_through` on (a bad `logo_small` image\nleaves out all three sizes). A `logo_small` image needs a `logo` beside it\n(`logo_small_without_main`), and two images for one size are `logo_role_duplicate`: either way\nno logo is built.\n\n**Kit textures and configs by version**\n\nFor PES 2015 to 2017 a kit's `kit_mask` texture is built, and a `Full` export's kit without one\ngets the compiler's built-in mask. A `kit_srm` texture gets the line `kit_texture_not_used`:\nthese versions do not read it, so it is not built, and the rest of the kit is. For PES 2018 to\n2021 it is the other way round: the `kit_srm` is built, a kit without one gets none, and a\n`kit_mask` gets the line `kit_texture_not_used`. Kit configs go into the CPK as loose files, one\nper kit in the team's folder, and for PES 2015 to 2017 there is no `UniformParameter.bin`.\n\n**Boots, gloves and shared folders**\n\nA player's own boots and gloves get the ID reserved for his roster slot, the same number for\nboth. On PES 2018 to 2021 a face model whose vertices are weighted to the hand bones (`skh_`)\nhas its hands cut off at the wrist into the player's gloves at compile time, which the note\n`model_hand_split` names. A model named as boots or gloves is never cut.\n\nA shared `Boots` or `Gloves` folder that players point at with a link file (an empty\n`Crocs.boots` in the player's folder names `Boots/Crocs`) is built once, with its own textures,\nunder one of the 17 IDs the team keeps for shared boots folders (and 17 more for gloves\nfolders). The IDs are given out in the folders' name order (`Apple` before `Zebra`). A folder\nlinked plainly (by the link file alone, not combined with boots or gloves of the player's own,\nbelow) by just one player takes that player's own ID instead. Either way, a player linking it\nplainly gets no boots or gloves of his own.\n\nA player folder holding both a link file and boots (or gloves) of its own combines the two on\nPES 2018 to 2021, and on PES 2015 to 2017 when it also holds `ingame_face.toml`. The shared\nfolder's models join the player's own boots (or gloves) under the player's ID, its textures\ntravel with them, and the line `link_combined` says so. On PES 2018 to 2021 his own boots or\ngloves are any of these:\n\n- a model of his;\n- one a `.common` link brings in;\n- a boots or gloves model of a `Faces` folder he links;\n- for gloves, the hands cut off a face model of his or of a `Faces` folder he links.\n\nOn PES 2015 to 2017 without `ingame_face.toml` the link keeps its plain meaning, and the\nplayer's own boots and gloves models are parts of his face. The shared folder is still built on\nits own for any player linking it plainly.\n\nA link to a shared `Faces` folder (`Longhair.face` names `Faces/Longhair`) always combines: the\nshared folder's face models become part of the player's face, beside any face models of the\nplayer's own. Its face diff (`face_diff.bin` or `face_diff.xml`) is used only when the player's\nfolder has neither, and its `fcl_hair_sim.fclo` on PES 2018 to 2021 only when the player's\nfolder has none.\n\n**Links to the `Common` folder**\n\nA player can also load a model kept in the export's `Common` folder through an empty link file\nnamed after it plus `.common` (`legs.fmdl.common` directly in the player's folder names\n`Common/legs.fmdl`). On PES 2018 to 2021 the game cannot read a model from there, so the Common\nmodel is built into the player's own face, boots or gloves as if it were a model of the same\nname in the player's folder, and merged with the player's models of that name. Its name says\nwhich of the three it joins: `legs.fmdl.common` is face content, reported as\n`fmdl_fcl_hair_fallback` like a local `legs.fmdl`. A skeleton file named after it in `Common`\n(`Common/legs.skl`) comes with it.\n\nOn PES 2018 to 2021 a `.model` in `Common` works the same way (`legs.model.common`): it is\nconverted for each player linking it, with the `.mtl` a link finds (a `.mtl` named like it\nbeside the link, else one beside the model in `Common`, else one beside the link, else one in\nthe player's or shared folder holding the link). A `.fmdl` of the same name in `Common` is used\ninstead when there is one. A converted model's `.mtl` path into the team's PES 2015 to 2017\nCommon folder (`model/character/uniform/common/team_folder/skin.dds`, or\n`team_common_folder/skin.dds` written short) is pointed at a texture of that name in the\nmodel's own folders (a `skin.dds` of the player's). Otherwise it is pointed at the team's\nCommon output, whether `Common` holds that texture or not, and `fmdl_texture_not_found`\nreports one nobody supplies (below).\n\nThe textures in `Common` (image files directly in it, and on PES 2015 to 2017 the image files\nof a subfolder too) are built once for the whole team, whether a player uses them or not. A\nCommon model's textures of those names are read from there, so twenty players sharing one\nCommon model share one copy of its textures.\n\nA player's own models can use a texture in `Common` the same way: an empty file named after it\nplus `.common` (`hair.dds.common` directly in the player's folder, for `Common/hair.dds`) makes\nthe player's models use the team's one copy of `hair`, instead of a copy of the player's own.\nThe folder holding the link must not also hold a texture named `hair`, whatever its format\n(`texture_stem_conflict`: `hair.png` beside `hair.dds.common` is one).\n\nA shared `Faces`, `Boots` or `Gloves` folder can hold links too, in it or in any\nsubfolder, standing for the `Common` file at the same path: for PES 2018 to 2021 they\nwork as a player's do, a texture link, a model link (a `Boots` folder holding only\n`boots.fmdl.common` has boots) and a `.mtl` link alike. For PES 2015 to 2017 its links\nare not read yet: each is reported as `file_not_used`. The rest of its tree follows a player\nfolder's rules at any depth, except that a shared link, a marker, `settings.toml` or\n`ingame_face.toml` anywhere in it is `file_type_disallowed`. As in a player folder, a face file\n(`face.xml`, `face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo`) below a subfolder is\n`file_type_disallowed` too.\n\n**Merged models and conflicts**\n\nA model of the player's own and a Common model must not define a material of the same name\nover textures kept in different places (the player's folder and `Common`): that is a\n`merge_material_conflict`. When a texture in `Common` cannot be packed for a reason other than\nthe texture lines above, the line `folder_pack_failed` names it in `Common`. No texture of the\n`Common` folder or subfolder it is in is built, and the players linking Common models are\nstill built.\n\nSeveral boots models in one folder (`a_boots.fmdl` beside `kit_boots.fmdl`), several gloves\nmodels for one hand, or several face models under one name (`face_high.fmdl` beside\n`old_face_high.fmdl`) are, on PES 2018 to 2021, merged into one model in the alphabetical order\nof their file names, reported as `fmdl_merged`. On PES 2015 to 2017 only boots models merge,\nthose of a shared `Boots` folder or of a player whose folder holds `ingame_face.toml`, reported\nas `model_merged` (below).\n\nModels merged into one must agree with each other: a material they define differently is\nreported as `merge_material_conflict`, and skeletons that differ (one model with a skeleton\nfile and one without, two different files, or a bone placed differently) as\n`skl_merge_conflict`. Either leaves that folder's face, boots or gloves out of the CPK. The rest\nof the folder is still built, so the player shows in the game with that part missing (a player\nwith boots only and no face model of his own shows no head). Treat the folder as broken until\nthe error is fixed.\n\nA texture under one name in two of the places a player's models come from (the player's own\nfolder, a shared folder it combines) is packed once when the two files are the same. A texture\n`.common` link is the same only as another link to the same `Common` texture, never as a real\nfile, even one with the same content. When they differ, the player's own file wins, as a\ntexture link of his in that place does: every model of his, the shared folder's included, uses\nhis. The line `shared_texture_overridden` names the texture and the shared folder, so an\noverride you meant is visible and one you did not is caught. When two shared folders he\ncombines hold different files of one name, his whole folder is left out of the CPK, and the\nline `shared_texture_conflict` names the texture and every package holding it\n(`packages=face,boots`).\n\nAn export whose shared boots folders, or shared gloves folders, need more shared IDs than the\nteam's 17 of that kind have free is skipped by both commands with the error\n`boots_id_pool_exhausted` or `gloves_id_pool_exhausted`, naming the count.\n\n**Models named for no part**\n\nOn PES 2018 to 2021 a model whose name says nothing about what it is (`torso.fmdl`) is taken\nfor face content and merged into the face's `fcl_hair.fmdl`. The line `fmdl_fcl_hair_fallback`\nnames it, so boots and gloves models must say so in their names.\n\nIn a shared `Boots` or `Gloves` folder nothing can be face content. A model anywhere in it not\nnamed as boots or gloves (`hat.model` in `Boots/Mud`), and on PES 2018 to 2021 a `.common` link\nto one not named so (`glove_l.fmdl.common` in `Boots/Mud`), is the error `fmdl_name_invalid`\n(`model_name_invalid` for PES 2015 to 2017). The folder is left out with the players linking it\n(`link_target_dropped`). A shared boots or gloves folder holding no model of its kind (on PES\n2018 to 2021 a `.common` link to one counts) gets no ID. When a player links it with no model of\nhis own for that part, it is reported as the warning `shared_folder_no_model`, and the player\nwears the game's own.\n\n**Skeletons and face files**\n\nOn PES 2018 to 2021 a skeleton file named after a model (`kit_boots.skl` beside\n`kit_boots.fmdl`, `fcl_hair.skl` beside `fcl_hair.fmdl`, `torso.skl` beside `torso.fmdl`,\n`boots.skl` beside a shared folder's `boots.fmdl`) is packed as the part's skeleton under its\nslot's name (`boots.skl` for a boots model, `fcl_hair_sim.skl` for a face's `fcl_hair` part).\nOne named after a gloves model pairs with nothing and is `file_not_used`. Boots and hair\nwithout one get the standard body skeleton. On PES 2018 to 2021, a skeleton named after a\n`face_high`, `hair_high` or `oral` model has no place in the game's face package: it is left\nout and reported as `skl_no_slot`. On PES 2015 to 2017 no `.skl` is packed: the bones of one\nbeside an `.fmdl` go into the `.model` that `.fmdl` is converted to.\n\nA face folder with no face diff (`face_diff.bin` or `face_diff.xml`) gets the bundled default\nfile. On PES 2018 to 2021 an `fcl_hair` model without `fcl_hair_sim.fclo` beside it gets the\nbundled file of that name too.\n\nA face folder may give its `face_diff.bin` as text instead, in a `face_diff.xml`: the file in\nbase64, alone or inside a `<dif>` element, the way an older face folder's `face.xml` holds it.\nIt is decoded and packed as the face's diff (`face_diff.bin` on PES 2018 to 2021, the face's\n`<dif>` on PES 2015 to 2017), and counts as one. The player's own face diff, in either form, is\nused over a linked `Faces` folder's.\n\nBoth commands report a face diff that cannot be used as `face_diff_invalid`, naming the file\nand the reason: text that is not base64, a `<dif>` holding elements, bytes that are not a face\ndiff, or a file shorter than its own header says, which the game would read past the end of.\nThey also report a folder holding both `face_diff.bin` and `face_diff.xml` as\n`xml_dif_conflict`, so `check` finds both problems too. Either leaves the whole folder out of\nthe CPK, even with `pass_through` on, and a player linking a `Faces` folder left out this way\nis left out too, with the line `link_target_dropped`.\n\n**Subfolders in a player's folder**\n\nA player's folder may sort its files into subfolders of any name and depth. Each file there is\nwhat its name says, exactly as in the folder itself: `parts/boots.fmdl` is the boots. On PES\n2018 to 2021 a model in any subfolder goes into `fcl_hair.fmdl` when the model is not named as\nface, boots or gloves, reported as `fmdl_fcl_hair_fallback` with its subfolder in the name\n(`parts/torso.fmdl`). A skeleton file pairs with the model beside it in the same subfolder,\ntextures from any subfolder are packed with the player's other textures, and parts in a\nsubfolder are merged with the folder's own models and with a linked shared folder of the same\nkind.\n\nOnly the files directly in the player's folder count as `ingame_face.toml`, `fpc_on` or\n`fpc_off`, `settings.toml`, or a face file (`face.xml`, `face_diff.bin`, `face_diff.xml`,\n`fcl_hair_sim.fclo`): one of those inside a subfolder is reported as `file_type_disallowed`. A\nlink file in a subfolder works as it does in the folder itself, and a `.common` link there\nnames the `Common` file at the same path (`face/torso.fmdl.common` names\n`Common/face/torso.fmdl`).\n\nA `.txt` note (`notes.txt`) in a player or shared folder, or in `Common`, is allowed: nothing\nreads it, so it is reported as the warning `file_not_used`.\n\n**A player with no face model, and the in-game face**\n\nA player folder with no face model, and no `Faces` link or (on PES 2015 to 2017) `face.xml` of\nits own bringing one, still gets a face folder, an empty one, so the game shows no head for\nthat player. That is the usual case when a body model brings its own head.\n\nA file named `ingame_face.toml` in the player's folder gives him the in-game face instead: the\nhead the game builds from his face, hair and iris, which the file sets (`iris_color`, `[face]`,\n`[hair]`, the keys `settings.toml` held before). An empty file gives the game's default face.\nNo face folder is built, and a model that would have gone into the face (`torso.fmdl`,\n`fcl_hair.fmdl`) becomes part of the player's boots, with its skeleton file, and is not\nreported as `fmdl_fcl_hair_fallback`. The file's old names, `ingame_face` and\n`ingame_face.txt`, are the error `ingame_face_marker_renamed`: the file is ignored. A\n`face_high`, `hair_high` or `oral` model, a `.common` link to one, or a `Faces` link in a folder\nholding `ingame_face.toml` is the error `ingame_face_explicit_face_model`, and the player's\nfolder is left out.\n\nA face file in a folder that builds no face is not used, and both commands report it as\n`face_file_not_used`. The face files are `face_diff.bin` and `face_diff.xml`,\n`fcl_hair_sim.fclo` on PES 2018 to 2021, and on PES 2015 to 2017 a member's own `face.xml` in a\nfolder holding `ingame_face.toml`. A folder builds no face when it is:\n\n- a player folder with no face model, no `Faces` link bringing one and, on PES 2015 to 2017, no\n  `face.xml` of its own;\n- a shared `Faces` folder with no face model and no such `face.xml`;\n- a shared `Boots` or `Gloves` folder.\n\n**Faces on PES 2015 to 2017: the `face.xml`**\n\nFor PES 2015 to 2017 every `.model` of a player folder without `ingame_face.toml`, boots and\ngloves included, goes into the player's face, listed in a `face.xml` that `compile` writes with\neach model's type read from its name.\n\nA player folder may hold its own `face.xml` instead (directly in it, not in a subfolder), and\nboth commands check its content. What is known not to work leaves the folder out:\n\n- a file that does not parse;\n- a root other than `<config>`;\n- a `<model>` without `type` or `path`;\n- a `./` or Common reference to a file the export does not hold;\n- a Common path naming a folder other than `team_folder` or a folder after `common_folder/`\n  (the game's own folder, where a reference names only a file directly below it), or spelled\n  `common/` (the CPK's own root folder, which no reference names);\n- on PES 2016, a model name not starting with `face_high_`, `hair_high_` or `oral_`;\n- a `<dif>` beside a `face_diff.xml` (not beside a `face_diff.bin`: the `<dif>` is for PES 2015\n  to 2017, the bin for PES 2018 to 2021).\n\nWhat the compiler cannot vouch for is kept and warned about: an unknown element, attribute or\n`type`, a `ratio` that is not a number, a path it cannot check, and more than one `face_neck`\nentry. A `.model` of the folder the xml does not list is `xml_model_unlisted`, and it is not\npacked. A `level` other than 0 is noted.\n\n`compile` writes the xml back with these changes:\n\n- its `./` files each packed under the name its reference gives;\n- each Common model's path written with the team ID in place of `team_folder`;\n- the `face_neck` stand-in appended when no entry has that type (`xml_face_neck_added`);\n- for PES 2015, `uniform` written `uniform_sub` (`xml_uniform_pes15`).\n\nOnly the models and `.mtl` files the xml names are packed, and its own `<dif>`, else the\nfolder's face diff, is the face's. For PES 2018 to 2021 the xml is ignored, reported as\n`xml_ignored_fox`, and the folder's models compile as they would without it.\n\nOn PES 2015 to 2017 a `face.xml` in a shared face folder is checked the same way, in that\nfolder, and becomes the xml of every player's face linking it. Each of his own models it does\nnot name gets the entry a generated xml would give it, after the xml's own entries, and his own\nface diff, when he has one, wins over the xml's `<dif>`. A player folder linking a shared face\nmust not hold a `face.xml` of its own: holding one, whether or not the shared folder has one, is\n`xml_shared_face_conflict`, which leaves his folder out even with `pass_through` on. On PES 2015\nto 2017 a `face.xml` in a shared boots or gloves folder is ignored, reported as\n`xml_ignored_shared`. On PES 2018 to 2021 any folder's own `face.xml` is ignored as\n`xml_ignored_fox`.\n\nOn PES 2015 to 2017 a face model whose vertices are weighted to the hand bones (`skh_`) has its\nhands cut off at the wrist at compile time, which the note `model_hand_split` names. The rest\nkeeps the model's place, and the hands of `body.model` become two more models of the same face,\nlisted as its left and right gloves with the same `.mtl`. A model named as boots or gloves, or\none a `.common` link brings in, is never cut. Nor is any model of a face with a `face.xml` (its\nown or its linked shared face's), which lists what the face loads.\n\nWith `ingame_face.toml`, every `.model` but a glove (`shirt.model` and `torso.model` included)\nbecomes part of the player's own boots, written as one `boots.model` with one `boots.mtl` in his\nown boots folder. Several are merged into one, reported as `model_merged`, and a linked `Boots`\nfolder beside them is merged in too, reported as `link_combined`. Parts that define one material\ndifferently (`merge_material_conflict`), one bone differently (`skl_merge_conflict`) or carry\ndifferent header flags (`model_merge_flags_conflict`) leave the player's boots out, the error\ntelling which. His gloves (`glove_l`, `glove_r`, `handL`, `handR`) go to his own gloves folder,\neach written as a linked `Gloves` folder's models are (below), none merged. A linked `Gloves`\nfolder beside them adds its models, reported as `link_combined`, the player's own model or\n`.mtl` replacing the linked folder's file of the same name.\n\nWith `ingame_face.toml`, a `.common` link to a model (`kit_boots.model.common`,\n`glove_l.model.common`) makes the Common model one more part of his boots or gloves, copied into\nhis own folder with the `.mtl` it uses (found as for any link, below). A Common `.fmdl`\n(`legs.fmdl.common`) is converted into the part as his own are, and a link to a `.mtl` that one\nof his models uses is copied in the same way. The textures a Common `.mtl` names stay in the\nteam's Common folder.\n\nA face with models but none of type `face_neck` (a `face_high` model is one) gets an invisible\nstand-in, reported as `xml_face_neck_added`. For PES 2015 a model typed `uniform`\n(`body_uniform.model`) is listed as `uniform_sub` instead, reported as `xml_uniform_pes15`. A\nplayer or shared folder holding a `face_edithair.xml` or a `hair.xml`, anywhere in it, is the\nerror `edithair_unsupported` for PES 2015 to 2017, and the folder is left out. Both commands\nreport it.\n\nA linked `Faces` folder is copied into the face of each player linking it. A player who also has\nface models of his own gets the line `link_combined`, his own model or `.mtl` replacing the\nlinked folder's file of the same name. A linked `Boots` folder's `.model` is written once as\n`boots.model` with the `.mtl` it uses as `boots.mtl`, several merged into one and reported as\n`model_merged`. A linked `Gloves` folder's models are written once at their paths, and the\n`.mtl` files they use at the folder's root, each named by its path below the folder with `/`\nwritten `_` (`grip/glove_r.mtl` as `grip_glove_r.mtl`), all in lower case. The models are\nlisted in a `glove.xml` that `compile` writes. The textures of either sit beside the models.\n\n**Which `.mtl` a model uses**\n\nEach model uses the first `.mtl` found in its own folder, then in the player's or shared folder\nholding it: one whose name is the start or the end of the model's name (`body.mtl` for\n`body_high.model`; a model named as the game packs it is read without the `oral_` and `_win32`\naround its name, so `fbm.mtl` for `oral_fbm_u_win32.model`), then `materials.mtl`, then any.\n\nFor PES 2015 to 2017, and for PES 2018 to 2021 when no `.fmdl` of its name is beside it, a model\nwith no `.mtl` to use is the error `model_material_undefined`, and the folder holding it is left\nout, even with `pass_through` on. A model using a material its `.mtl` does not define is\n`model_material_undefined` too, naming the `.mtl` and the missing materials. The folder is left\nout unless `pass_through` is on, which keeps it as it is, and the game then draws those parts\nwith its fallback material. Both commands report either.\n\n**The `Common` folder on PES 2015 to 2017**\n\nFor PES 2015 to 2017 the models, `.mtl` files and textures in `Common` are written once into\nthe team's Common folder in the game, which loads them from there. A `.common` link to a model\n(`legs.model.common`) lists the Common model in the player's `face.xml`, using a `.mtl` named\nlike it beside the link, else the one found beside the model in `Common`, else one found as\nabove.\n\nA `.fmdl` in `Common` is converted once there, with the `.skl` of its name as its pose, and its\nmaterials are written as a `.mtl` of its name. A `.mtl` of that name in `Common` too is two\nfiles of one name, and the Common models are left out with `folder_pack_failed`. A link to the\n`.fmdl` (`legs.fmdl.common`) lists it with those materials, not a `.mtl` of the player's. A\n`.fmdl` beside a `.model` of the same name in `Common` is left out for the `.model`, which a link\nto the `.fmdl` then lists.\n\nA link to a `.mtl` (`body.mtl.common`) counts as that `.mtl` sitting beside the link, and a link\nto a texture points the player's `.mtl` files at the Common texture.\n\n**Kits with missing or unused files**\n\nA kit folder of a full export with no `kit` texture, its own or the one from `all` (an empty kit\nfolder included), is built with a magenta and black checkerboard in its place and reported as\n`kit_placeholder`, so a kit nobody drew shows as missing in the game. A kit texture whose name\nthe compiler does not build (`kit_spec.dds`) is left out and reported as `kit_texture_not_used`,\nas the `kit_mask` or `kit_srm` map the version does not read is (above). A kit's `config.toml`\nthat cannot be read (not UTF-8 text, a value of the wrong type or out of range) is reported by\nboth commands as\n`kit_config_invalid`, naming the error, and the kit is left out, even with `pass_through` on.\n\n**Model conversion**\n\nFor PES 2015 to 2017 `compile` builds:\n\n- a player folder's own `.model` files with their `.mtl` files, textures and face diff;\n- its `.common` links to a `.model`, an `.fmdl`, a `.mtl` or a texture;\n- the linked shared `Faces`, `Boots` and `Gloves` folders;\n- the `.model`, `.mtl`, `.fmdl` (converted, with its `.skl`) and texture files of the `Common`\n  folder;\n- the kits, the export's `.model` and `.fmdl` collars, and the portraits and the logo.\n\nA player's `.fmdl` models (with `ingame_face.toml`, the `.fmdl` parts of his own boots and\ngloves), and a linked shared `Faces`, `Boots` or `Gloves` folder's, are converted to `.model`\nfiles with their materials. A file in a model folder that no package reads (a skeleton pairing\nno model, a link to a file of a kind nothing reads) is left out and reported as the warning\n`file_not_used`. A `.fmdl` beside a `.model` of the same name is left out for the `.model`. A\n`.skl` of a converted model's name gives its pose, and `fcl_hair_sim.fclo` is not used.\n\nA converted model moved onto the target version's skeleton is noted as `skeleton_retargeted`,\nand one that cannot be converted leaves the face, boots or gloves it is part of out with\n`model_conversion_failed`. A player's own `.model` whose bones sit on another version's skeleton\nis moved onto the target version's too, its `.mtl` packed as written. A model a member's own\n`face.xml` lists is packed as it is, and so is a `.model` collar when it sits on the version's\nown body.\n\nA metal material (a Fox `fox3ddf_ggx` one) converted this way reflects the compiler's\nenvironment map, written beside the player's textures, or a shared `Boots` or `Gloves` folder's,\nas `env.dds`. An `env` texture of the folder's own (`env.dds`, `env.png`), or the Common one its\n`env.dds.common` link names, is used instead. A `Common` folder's `.fmdl` gets the map in the\nteam's Common output, or the `env` texture of the `Common` folder. A `templates/env.dds` in the\ndata folder (the folder holding the settings file) replaces the built-in map.\n\nFor PES 2018 to 2021 a player's own `.model` models, and a shared folder's, are converted to\n`.fmdl` with the `.mtl` they use. A `.model` beside a `.fmdl` of the same name is left out for\nthe `.fmdl`, with its `.mtl`. A converted model is checked again as the `.fmdl` it becomes. One\nwhose conversion fails, or that comes out with a vertex more than 5000 units from the origin, is\nreported as `model_conversion_failed` or `vertex_too_far_from_origin`, and its package (face,\nboots or gloves) is left out, even with `pass_through` on. A player's own `.fmdl` whose `.skl`\nposes it on a skeleton other than the target version's is moved onto the target version's\nskeleton, noted as `skeleton_retargeted`. One without an `.skl` is taken to sit on PES 2021's\nskeleton, since an `.fmdl` does not record its version and the other Fox versions' skeletons\ndiffer from it only slightly.\n\nOn every version, a converted model's Warnings and Infos name what the conversion changed: a\nbone folded onto another or its pose guessed, a material's shader guessed, a texture, a flag or\na field the target version has no place for.\n\n**`.glb` and `.gltf` models**\n\nA `.glb`/`.gltf` model in a player folder is not read yet. With no `.fmdl` (PES 2018 to 2021) or\n`.model` (PES 2015 to 2017) of its name beside it, `compile` leaves its folder out with the\nerror `model_gltf_unsupported`, naming it, and does not use the `.model` or `.fmdl` of the other\ngame beside it instead. Beside one of its name in the game's own format it is ignored.\n\nA `.glb` or `.gltf` model in `Common` (directly in it, and on PES 2015 to 2017 in a subfolder\ntoo) with none of its name in the game's format beside it is left out by itself, the same error\nnaming it. A folder whose `.common` link names a model of its name is left out with that error.\nOn PES 2018 to 2021, when that folder is a shared folder, the players linking it are left out\nwith it. One in a shared `Faces`, `Boots` or `Gloves` folder leaves out that folder and every\nplayer folder linking it, each player reported with `model_gltf_unsupported` naming the shared\nfile (`Boots/Crocs/boots.glb`): a player without the face, boots or gloves he linked would not\nlook as he should.\n\n**Custom collars**\n\nA `Collars` folder holds custom collar models, each replacing one of the game's stock collars:\nmodel files (`.fmdl`, `.model`, `.glb` or `.gltf`) directly in the folder, named `collar_` plus\nthe stock collar's number, with or without leading zeros (`collar_12.fmdl` and\n`collar_012.fmdl` both replace collar 12). Any other file there is `file_type_disallowed`.\n\nBoth commands check each collar file for the target version:\n\n- A name that is not `collar_<number>`, or a number that is not a stock collar a kit can wear,\n  is the error `collar_id_invalid`. The stock collars run from 1 to 101 on PES 2015, to 105 on\n  PES 2016, 116 on PES 2017 and 2018, 124 on PES 2019, 127 on PES 2020 and 131 on PES 2021.\n- Collars 105 (worn by every FPC kit) and 77 (the referees' marker, see below) are the\n  compiler's own, so a file named for either is the error `collar_id_conflict`, naming who\n  holds it.\n- A `.fmdl` or `.model` collar is then checked as any model is: one that cannot be read is\n  `model_broken`, and an error the checks on the model find (a vertex too far from the origin,\n  for instance) leaves it out too, while a warning keeps it.\n\nEach of these errors leaves the collar file out, and the rest of the export goes on.\n\nOn PES 2018 to 2021, `compile` writes a team's `Collars/collar_<number>.fmdl` as it is in place\nof that stock collar. It sets every kit of the team to wear it, as its collar and its winter\ncollar: the kits the export holds, and for a midcup export also the team's installed kits it\ndoes not send again. This comes after the FPC values, so an FPC team's custom collar wins over\nthe FPC one. Two exports cannot replace the same collar: the first in export order keeps it, and\nthe later one's file is the error `collar_id_conflict`, naming the export that holds it. That\nfile is left out and the later export's kits keep their own collars. An export holds one\ncollar, so a second collar file of the same export is that error too, naming the export itself.\n\nOn PES 2015 to 2017, `compile` writes a team's `Collars/collar_<number>.model` as it is in place\nof that stock collar. One posed on a body other than the version's, as PES 2017's own collars\nare on PES 2015, is first moved onto it, its material names kept. Every kit of the team wears it\nas its collar and winter collar, as above: the kits the export holds, and for a midcup export\nthe team's installed kits it does not send again, whose kit configs the CPK then carries\nchanged. The two rules on who keeps a collar are the same.\n\nA `.fmdl` collar compiled for PES 2015 to 2017 is converted to a `.model`, its materials named\nas the game's own collars' are: on PES 2017 `uni_collar` (the first) and `uni_shirts` (the\nrest), on PES 2015 and 2016 all `uni_shirts`, so the game dresses it with the team's kit as it\ndoes them. No `.mtl` is written for it. A `.model` collar compiled for PES 2018 to 2021 is\nconverted to a `.fmdl`, with PES 2017's own `uniform.mtl`, which the compiler carries, as its\nmaterial set. Its materials (`uni_collar` and `uni_shirts`, the only two a collar may name) take\nthe game's own collar materials of those names, so the game dresses it with the team's kit as it\ndoes them. One that cannot be converted, either way, is the error `model_conversion_failed`, and\nthe collar file is left out. A `.glb` or `.gltf` collar is not read yet:\n`compile` leaves it out with the error `model_gltf_unsupported`, and the rest of the export\ngoes on.\n\n**Referees**\n\nA `refs` export compiles like a team's player folders. Each referee folder its `players.txt`\nlists is built once and written for every slot it is listed under: slot NN as the face\n`referee0NN` with the boots `k99NN` and the gloves `g99NN`. The folder's textures go once into\n`common/999/<folder name>/`. A referee's link to a `Boots` or `Gloves` folder makes that folder\nthe boots or gloves of each of his slots. On PES 2018 to 2021 it is merged with any of his own;\non PES 2015 to 2017 it is written as his slot's folder alone, his own boots and gloves riding in\nhis face as a team player's do.\n\nA referee's face follows a player's (see \"A player with no face model, and the in-game face\"\nabove). A referee with no face of his own gets an empty face folder, so the game shows no head\nover a body that brings one. One who should keep the game's referee head holds\n`ingame_face.toml`.\n\nA `refs` export's kits, logo, portraits and collars have no referee to go to: each is left out\nand reported as the warning `file_not_used`.\n\nThe referees go into a CPK of their own, named by the `refs_cpk_name` setting\n(`4cc_18_referees.cpk`), never into the teams' CPKs, with `multicpk_mode` on or off. Beside the\nreferees, it holds the referee kits and the referee appearance file the game needs, the same in\nevery compile. A file of these can be replaced from the data folder's `templates/referees_fox`\nfolder, or `templates/referees_prefox` on PES 2015 to 2017 (see `templates` below). On PES 2018\nto 2021 each referee kit is also written, as it is in the referees CPK, as its entry in\n`UniformParameter.bin` (the team CPK's, or the bins CPK's with `multicpk_mode` on), which the\ngame reads the kits' values from. So a `refs` export compiled alone writes that CPK too.\n\nA `ref_marker.dds` at the root of the `refs` export, the cup's logo for instance, is shown on\nthe ground under every referee. On PES 2018 to 2021 it goes into the referees CPK with a flat\nsquare model written as collar 77, and the referee kits are set to wear that collar. On PES\n2015 to 2017 it replaces the texture of the prop the game itself shows under the referee\n(`parts/referee/incom_bsm.dds` in the referees CPK), and the referee kits keep their collar.\nCollar 77 stays the referees' on every version: a team's kit whose collar or winter collar is\n77 (which on PES 2018 to 2021 would wear the marker too) is left out with the error\n`kit_collar_reserved`, naming the field. A `ref_marker.dds` that cannot be converted is\nreported with its texture error and left out, and the referee kits keep their own collar.\n\nThe referees CPK is installed into the game together with the teams' CPKs, all of them or\nnone, or left in the output folder with them, after them. It is written only when the `refs`\nexport builds something. On PES 2015 to 2017 a compile in which only the `refs` export builds\nsomething, with no file in the `overrides` folder, writes it alone, and the installed team CPK\nstays as it was. Since the compile replaces it, the installed referees CPK is not one of the\nCPKs that the team colors, kit colors and other files shared by every team are taken from.\n\nA `refs_cpk_name` that names the team CPK (`cpk_name`), or, with `multicpk_mode` on, the\n`bins_cpk_name` CPK or one of the numbered `teams_cpk_name` CPKs, letter case aside, stops the\ncommand with exit code 2. `--mode test` and `--mode sideload` write the referees' files with the\nothers' and have no referees CPK. `--mode sideload` writes the referee kits and appearance file\nwith them, and `--mode test` does not, since they come from no export.\n\n**Installing the CPK**\n\n`compile` installs the CPK into the game. It copies the CPK into the `download` folder of the\ngame folder (the `pes_folder_path` setting) and then puts it in the place of the old one, so the\ngame never finds half of a CPK there, and no copy of it is left in the output folder. Before it\nreads any export, it checks that it can install. When it cannot, or when the old CPK cannot be\nreplaced at the end, it says why with an error (listed below, after `--no-deploy`), leaves the\nCPK in the output folder instead, and ends with exit code 1.\n\n**Notes for whoever compiles the cup**\n\nAn export may hold a `notes.txt` at its root, a note for whoever compiles the cup. `compile`\ngathers the notes of every export it compiles into one file, `teamnotes.txt` in the output\nfolder, one section per team headed by its name (`--- /co/ ---`, `--- /refs/ ---` for the\nreferees), in the order of the exports. The note of a skipped export is left out. A compile\nwith no notes to gather removes an older `teamnotes.txt`, so the file never shows notes of\nexports the last compile did not build. A compile that fails before its output is in place, or\nwrites nothing, leaves it as it was. When `teamnotes.txt` cannot be written, or a previous one\ncannot be removed, the error `teamnotes_write_failed` names it, and the CPK is still in place.\n\n**The `overrides` folder**\n\nA folder named `overrides` beside the settings file (the data folder) holds files that\n`compile` puts into the CPK as they are, each at its path below that folder (into `livecpk/`\nin `--mode sideload`): `overrides/common/etc/TeamColor.bin` becomes `common/etc/TeamColor.bin`\nin the CPK. Use it for a file no export provides, or a fixed version of one an export gets\nwrong. `compile` reports the note `overrides_active`, naming the folder and how many files it\nholds, and writes the CPK even when no export is compiled.\n\nA file in `overrides` wins over the same file from an export or from the compiler (the team\ncolor file, for one). The warning `duplicate_path` names each file replaced that way, and the\nrest of that export is still built. A file in `overrides` that cannot be read stops the CPK from\nbeing written, with the line `cpk_write_failed` naming it, and the previous CPK stays as it was.\n`check` does not read the folder.\n\n**What `check` prints**\n\n`check` prints one line per finding, giving:\n\n- the export it is about, when there is one;\n- how serious it is, and its code;\n- where in the export it is, when it is about a folder or file;\n- its details in parentheses, when it has any.\n\nThe line `Info export_identified (team=/co/, id=714)` tells you\nwhich team the export was recognized as: its name's first word, looked up in the teams list\n(`team=referees` for a `refs` export).\n\n**Full and midcup exports**\n\nThe second word of an export's name says what the export covers: `Full` or `Midcup`, in any\nletter case (`co Full Spring 2026`, `co midcup day 5`). Only the second word counts, so a\n`Full` later in the name changes nothing. A team export whose second word is neither is skipped\nwith the error `export_tag_missing`, naming the export. Rename it `<team> Full …` or\n`<team> Midcup …`. A referee export needs no tag.\n\nA full export is the team's whole set, everything it has for the cup, so it replaces the team's\nkits in the game's kit list. A kit of a past cup that the export does not hold is no longer\noffered, and on PES 2018 to 2021 the team's installed kit configs of kits the export does not\nhold are removed (on PES 2015 to 2017 each is a file of its own, which a compile cannot\nremove). Every team needs a player kit and a goalkeeper kit, so a full export with no player\nkit, or no goalkeeper kit, gets an empty one (`p1` or `g1`), which compiles as the placeholder\nkit (`kit_placeholder`). On PES 2018 to 2021 a full export also clears the boots or gloves of\neach of its compiled players whose folder has none.\n\nA midcup export brings additions to what is already installed. It adds its kits to those\ninstalled, and what a midcup kit folder does not hold stays as the game has it installed:\n\n- a folder without `config.toml` keeps the installed kit config (the template where there is\n  none, or one that cannot be read);\n- one holding no `kit` texture keeps the installed textures;\n- one holding no usable `colors.txt` keeps the kit's installed menu colors.\n\n**Exports in an old layout**\n\nAn export still in the old layout (a root folder named `Kit Configs`, `Kit Textures` or\n`Other`) is skipped with the one error `export_layout_old`, naming the folder, and nothing else\nis reported about it. Run it through the Export upgrader once. A referee export's player folder\nsorted into `face`, `boots`, `gloves` or `common` subfolders, the layout of older referee\nexports, is left out with the one error `player_layout_proto`, naming the subfolder, and\nnothing else is reported about it. Run the export through the Export upgrader once. In a team\nexport those names are ordinary subfolders.\n\n**Model and material checks**\n\nBoth commands also read every `.fmdl` and `.model` model and every `.mtl` material file of the\nexport that the compile uses, an archive's included. They report what is wrong or suspicious in\neach, one line per file and problem, naming the file and how many faces, vertices or bones are\nconcerned.\n\nA problem the game cannot live with (a mesh over a hard limit, a face naming a vertex that does\nnot exist) is an Error. The folder is left out (a file in `Common` is left out alone), unless\n`pass_through` is on, which keeps it as it is. A Warning or an Info leaves everything in, but an\n`.fmdl` whose every mesh is hidden is itself left out on PES 2015 to 2017\n(`model_hidden_dropped`).\n\nA model with a vertex more than 5000 units from the origin lags the game for the whole matchday.\nIt is reported as `vertex_too_far_from_origin`, and its folder is left out even when\n`pass_through` is on (for a model the compiler converts, the part it is made into). So is a\nfile that cannot be read as a model or a material file at all, reported as `model_broken` or\n`mtl_broken`. A player linking a shared folder or `Common` model left out this way is left out\ntoo, with the line `link_target_dropped`.\n\n**Player settings**\n\nBoth commands also read each player's `settings.toml`, and his `ingame_face.toml` when he has\none. One that cannot be read (not UTF-8 text, a key it does not know, a value of the wrong type\nor outside the range its key allows) is reported as `settings_toml_invalid` or\n`ingame_face_toml_invalid`, naming the error. The file is ignored, even with `pass_through` on,\nand the player's models are still built. A bad `ingame_face.toml` still marks the in-game face,\nso the player wears the game's default face. The in-game face's keys (`iris_color`, `[face]` and\n`[hair]`) are `ingame_face.toml`'s. A key left in `settings.toml` under its old name\n(`[appearance.face]`, `[appearance.hair]`) is an unknown key there.\n\n**Kits drawn for the other games**\n\nThe socks are laid out differently on a kit's main texture in PES 2015 to 2017 and in PES 2018\nto 2021, so a kit drawn for one shows its sock design shifted around the leg in the other. An\nempty file named `pre-fox` or `fox` in a kit folder says which games the kit's main texture was\ndrawn for: `pre-fox` for PES 2015 to 2017, `fox` for PES 2018 to 2021. With no such file the\ncompiler takes the kit as drawn for the game it compiles for. When the file names the other\ngames, `compile` moves the socks to the layout of the game it compiles for, on the kit's\n`kit_mask` (PES 2015 to 2017) or `kit_srm` (PES 2018 to 2021) too. It says so with the note\n`kit_layout_converted`, naming both layouts. The shirt, sleeves and shorts are laid out the\nsame in every version and are not touched.\n\nA number texture (`kit_back`, `kit_chest`, `kit_leg`) made for the other games, its ten digits\nin a column for PES 2015 to 2017 or in a row for PES 2018 to 2021, is re-arranged by `compile`\nfor the game it compiles for. It is told apart by its shape, whatever the `pre-fox` or `fox`\nfile says. The name texture (`kit_name`) is left as it is.\n\n**Team and kit colors**\n\nA kit folder may hold a `colors.txt` giving the kit's two menu colors, and the export's root a\n`colors.txt` giving the team's colors, up to four. Both files hold one color per line, written\n`#c11200` or as three numbers from 0 to 255 separated by spaces or commas (`211 74 79`,\n`211, 74, 79`), optionally after a label ending in a colon (`Shirt: #c11200`). A kit's file, for\nexample:\n\n```text\n211 74 79\n162 62 77\n```\n\nBoth commands read these files. Each line that does not give exactly one color (two colors on\none line, the way an old Team Note kit entry writes them, a number over 255, a hex color\nwithout its `#`), and each color past the second in a kit's file or the fourth in the root one,\nis reported as the warning `color_entry_invalid`, naming the line and the reason. The line is\nskipped and the rest of the file is still read.\n\n`compile` writes the team's colors into the game's team color file, which the menus and the\nscoreboard show. A color the file does not give keeps the one the team had. A team export with\nno root `colors.txt` keeps all the colors the team had, and `compile` says so with the note\n`team_colors_missing`.\n\n`compile` also writes each kit's two colors and its icon into the game's kit color file, which\nthe kit selection menu and the scoreboard show. The icon is the number of the kit's `icon_<N>`\nmarker, or icon 3 when the kit has none. A `Full` export's kit whose `colors.txt` is missing\nor gives fewer than two colors has its colors taken from its main texture, and `compile` says\nso with the note `kit_colors_derived`. A `Full` export's kit with no main texture to take\nthem from (an empty kit folder, or one holding only other textures) gets magenta and black, so\nthe gap shows in the game's menus, with the warning `kit_colors_missing`. A kit a midcup export\ndoes not hold keeps the colors and icon it had.\n\nWhen the team color file or the kit color file `compile` starts from has damaged entries (a\nteam's colors written over the start of its entry, so the game no longer finds that team),\n`compile` repairs them and reports the warning `bin_header_repaired`, naming the file and the\nteams. Those teams' colors may be wrong until their exports are compiled again.\n\n**Where the color and kit config files start from**\n\n`compile` starts the team color, kit color and kit config files from the ones the game loads:\nthose of the CPKs your `DpFileList.bin` lists before the CPK being compiled, the nearest first.\nIt says which CPK each file came from (`bin_source`). A file none of those CPKs holds starts\nfrom the copy built into the compiler, and so does every file when the PES folder has no\n`DpFileList.bin` (`dpfilelist_missing`) or the list does not name the CPK being compiled. When\nthe list itself, a listed CPK, or a file the compile reads in a listed CPK (a bin, a kit config\nor the record of the team's shared IDs) cannot be read or is not a valid file of its kind, the\ncompile stops before any export is read (`installed_bin_unreadable`), so the CPK you had is\nkept.\n\n**Textures a model names**\n\nOn PES 2018 to 2021, a texture a model names must be found in one of these places:\n\n- the model's folder, where the player's linked shared folders' textures count too (a `./` path\n  naming a subfolder, `./textures/skin.dds`, is looked for at that path alone);\n- for a path into the team's Common folder, the export's `Common` folder or a CPK your\n  `DpFileList.bin` lists before the CPK being compiled (a midcup export can use Common textures\n  compiled into an earlier midcup CPK). A CPK listed after it does not count, because removing\n  it must not break this one.\n\nOtherwise `compile` reports `fmdl_texture_not_found` as a warning, one line per missing texture\nnaming the model and the texture, and keeps the part: a mesh naming it shows without it. When\nthere is no PES folder or no `DpFileList.bin`, or the list does not name the CPK being compiled,\nthe texture may still be in an installed CPK that could not be checked, so the line is the same\nwarning.\n\nA model's texture directory names the team's Common as in a `.mtl`: `team_common_folder/`, then\na subfolder of `Common` if any (`team_common_folder/Boots/`). It names the game's own common\ntextures as `common_folder/`. The long forms work too: `team_folder` or the game's own\n`sourceimages` below the game's common folder (`/Assets/pes16/model/character/common/`), so\n`team_common_folder/Boots/` in full is\n`/Assets/pes16/model/character/common/team_folder/Boots/sourceimages/`. Anything else there,\n`000` (the placeholder the community's model templates carry), a team ID, `XXX`, a team's name\nor a folder after `common_folder/`, is refused as the error `fmdl_common_path_invalid`, which\nleaves the model's folder out. `export-upgrader upgrade` respells an old model's folder in an\nold export (a Midcup export, for a model on its own).\n\nA texture whose name starts with `dummy_` is never looked for: the game puts its own in its\nplace. The same goes for `team_kit` and its family (`team_kit_srm`, and `team_gk_kit` for a\nkeeper): the modded exe puts the team's kit texture there, and the compiler writes the name the\nexe looks for, `dummy_kit`. A model naming `dummy_kit` itself, the old spelling, is refused\n(`dummy_kit_name_old`), and `export-upgrader upgrade` respells it in an old export. `check`\ndoes not look for these textures, only `compile` does.\n\nA texture in the game's own kit texture folder\n(`/Assets/pes16/model/character/uniform/texture/`) is not looked for either. The modded exe\nswaps the files there for every match, so the model would show whichever kit is being worn. Its\npath is kept and `compile` reports the warning `fmdl_texture_kit_path`. A part that should\nfollow the kit uses the `team_kit` textures or a `kitN` texture of its own. A part that should\nnot uses a texture of the player's folder or of `Common`.\n\nA texture link (`hair.dds.common`) may also name a texture that an earlier CPK (one listed\nbefore the CPK being compiled) holds in the team's Common folder, when the export's `Common`\nfolder has no `hair.dds`. For these links `check` looks in those CPKs too.\n\nOn PES 2015 to 2017, `check` and `compile` both look for the textures a `.mtl` names, and report\ntheir findings the same way. A `./` path must name a texture of the player's folder (his linked\nshared folders' textures count too). A path into the team's Common folder\n(`team_common_folder/...` or `model/character/uniform/common/team_folder/...`) must name a\ntexture of `Common` or of a CPK listed before the CPK being compiled, as above. A missing\ntexture that a mesh uses is the warning `mtl_texture_not_found`. The folder is still compiled,\nsince the game shows some such faces as they were made, but look at the player in the game. One\nthat only a material no mesh uses names is an info line, `mtl_texture_unused_missing`. A\n`dummy_` name and a path into the game's own folders are not looked for. On PES 2018 to 2021 the\nsame goes for the `.mtl` of a `.model` the compiler converts (one with no `.fmdl` of its name\nbeside it). A `.mtl` no such model uses is not read at all.\n\n**The `templates` folder**\n\nA folder named `templates` in the data folder holds files that replace the compiler's built-in\ncopies of the same name, so a cup can swap one without a new version of the compiler. The\nnames, spelled exactly as here, are:\n\n- `TeamColor.bin` and `UniColor.bin`: the team color and kit color files a compile starts from\n  when no installed CPK has them;\n- `UniformParameter18.bin` and `UniformParameter19.bin`: the kit config file a compile starts\n  from when no installed CPK has it, for PES 2018 and for PES 2019 to 2021;\n- `placeholder_kit.dds`: the checkerboard texture of a placeholder kit;\n- `kit_mask.dds`: the mask a kit compiled for PES 2015 to 2017 gets when it has none;\n- `body.skl`: the skeleton a boots or hair model gets when its folder has none;\n- `face_diff.bin` and `fcl_hair_sim.fclo`: the face file a face gets when its folder has none,\n  and the hair simulation file a PES 2018 to 2021 face gets when its folder has none;\n- `dummy.model` and `dummy.mtl`: the invisible stand-in model and its material a PES 2015 to\n  2017 face gets when its `face.xml` lists no `face_neck`;\n- `DpFileList.bin`: the cup's official list, which a compile compares the game's with and\n  `upgrade-dpfl` installs;\n- `placeholder.cpk`: the empty CPK `upgrade-dpfl` writes for a CPK of the list that is not in\n  the `download` folder, and `compile` writes as every unused `teams` slot with `multicpk_mode`\n  on;\n- `env.dds`: the environment map a PES 2015 to 2017 compile gives a converted metal material\n  that names none;\n- `uniform.mtl`: the stock material set a `.model` collar is read with.\n\nIts folder `referees_fox` holds files that replace those of the referee kits and appearance file\na PES 2018 to 2021 referees CPK holds, and its folder `referees_prefox` those a PES 2015 to 2017\nreferees CPK holds. A file there at the path it has in the CPK, spelled exactly, replaces that\none file (for example\n`templates/referees_fox/common/character0/model/character/appearance/RefereeAppearance.bin`).\nAny other file in the folder is ignored.\n\n`compile` and `upgrade-dpfl` name each file they used with the note `template_override_active`.\nWhen they cannot read one of these files, or one of the four kit and color files or\n`DpFileList.bin` is not a valid file of its kind, the command stops before it does anything\nelse (`template_override_unreadable`), so the CPK you had is kept.\n\n**FPC teams**\n\nAn FPC player's body is hidden only when every kit config of the team, the goalkeeper kit's\nincluded, carries the FPC values (shirt model 176, shorts model 16, collar 105, winter collar\n105). So when any player folder of an export holds the marker `fpc_on`, `compile` builds every\nkit config the export supplies with these values, and says so with the note\n`kit_config_fpc_adjusted` for each `config.toml` that lacked them (your file is not changed). A\nkit without a `config.toml` gets them anyway. A midcup export with `fpc_on` also gives these\nvalues to the team's installed kit configs of the kits the export does not hold but the game\nlists for the team, and names each kit it changed with the same note. A kit the game lists\nwith no installed config to change, or one that cannot be read, is reported as the warning\n`kit_config_fpc_unpatched`, naming the kit. Send that kit in an export.\n\nA midcup export's collar is given to those kits in the same way, after the FPC values. A kit the\ngame lists with no installed config to wear it, or one that cannot be read, is reported as the\nwarning `kit_config_collar_unpatched`, naming the kit, unless `kit_config_fpc_unpatched` has\nnamed it already. That kit keeps its own collar until you send it in an export.\n\nThe compiler never removes FPC values from a kit config. An export without `fpc_on` has its\nconfigs built as they are, FPC values or not, and taking a team off FPC is an edit you make in\nthe configs yourself.\n\n**Kit config values the game ignores or cannot hold**\n\nA kit config value the chosen PES version cannot hold (a name position `y` over 33 before PES\n2021 or over 39 on PES 2021, or a shirt pattern of 12 or 13 on PES 2015) is lowered to one the\nversion can hold when the kit is built. Both commands report it as the warning\n`kit_config_version_clamped`, naming the field, the value and the maximum. `compile` also\nlowers such a value in an installed config it changes for a kit the export does not hold (the\nFPC values or the collar above), and reports it the same way, naming the kit. The same `y` puts\nthe name at the same height on every version, so a config needs no change to move between them.\n\nA kit config that asks for cut-out short sleeves, undershirt-only long sleeves or the tight fit\non a shirt model other than 144 or 160 is reported by both commands as the warning\n`kit_config_option_ignored`, naming the option and the model. The config is built as written,\nand the game ignores the option on that model. On a team whose kit-FPC status is on, the FPC\nvalues set the shirt model to 176, so a config written for model 144 or 160 with one of these\noptions is reported with model 176, the model it is built with.\n\n**Which exports a run reads**\n\nBoth commands read every export in the exports folder from the settings (`exports/` beside\n`UTW.exe` unless you changed it). To use another folder for one run, give its path as\n`exports-root`; the setting is not changed. The `exports/` folder beside `UTW.exe` is created\nfor you when it is missing, and so is any relative path in the `exports_folder_path` setting.\nAn absolute `exports_folder_path`, or an `exports-root`, must already exist, or the command\nstops before anything runs and tells you so. An exports folder with no export in it is\nreported as `no_exports_found`, naming the folder, and nothing is checked or compiled.\n\nAn export can be a folder, a `.zip` or a `.7z`, read where it is: nothing is extracted. An\narchive that cannot be read (damaged, password-protected, holding a file whose path leads\noutside it, or holding two files whose names differ only in case or Unicode form) is reported\nas `export_extract_failed` and left out of the run. A folder whose name ends in `.upgrading` is\nnot an export: an upgrade that did not finish left it there. Both commands report it as the\nwarning `export_upgrade_unfinished`, naming it. Remove it, then upgrade the old export again.\n\nWhen two exports of a run are for the same team (an old `co Full Spring` folder left\nbeside a new `co Full Summer.zip`), both commands skip each of them with the error\n`duplicate_aesthetics_export`, naming the team's ID and the exports, and the other teams are\nstill built. Remove the one that should not be used, or disable it with an empty file named\n`NO_USE` at its root.\n\n`--export <path>` limits the run to one export: a folder, a `.zip` or a `.7z`, which does not\nhave to be inside the exports folder. Repeat it to name several exports. A path that does not\nexist, or a file that is not a `.zip` or `.7z`, stops the command before anything runs.\n\nA relative path typed in the terminal is taken from the folder the terminal is in.\n\n**Building without installing, and install errors**\n\n`--no-deploy` builds the CPK into the output folder without installing it into the game, and\nsays where it is with the line `Info deploy_skipped_by_flag (path=...)`. It cannot be combined\nwith `--mode test` or `--mode sideload`.\n\nWithout `--no-deploy`, these lines say why the CPK could not be installed, and name the path in\nthe output folder where it was left instead (`output=...`), `pes_version_mismatch` aside:\n\n- `pes_folder_not_found`: `pes_folder_path` does not name a folder that exists. Set it to the\n  folder PES is installed in.\n- `pes_version_mismatch`, a warning: the game folder holds no `PES20NN.exe` of the version you\n  compile for. Check `pes_version` and `pes_folder_path`. The CPK is still installed.\n- `dpfilelist_outdated`: the game's `DpFileList.bin` is an older list that does not have the\n  CPK, so the game would not load it. The compile stops before compiling anything.\n  `UTW compiler upgrade-dpfl` installs the cup's official list, which has it; then\n  compile again.\n- `cpk_name_unlisted`: neither the game's `DpFileList.bin` nor the cup's official list has the\n  CPK's name, so the game would not load it. The compile stops before compiling anything.\n  Set `cpk_name` to a name the list has, then compile again.\n- `deploy_target_unwritable`: the `download` folder, or the old CPK in it, cannot be written,\n  usually because the game is installed under `Program Files`. Run Unlimited Tool Works as\n  administrator and compile again.\n- `old_cpk_locked`: the old CPK could not be replaced, most likely because PES is running. Close\n  PES and compile again.\n\nTwo warnings about the game's `DpFileList.bin` do not stop the install. `dpfilelist_not_official`\nsays the list is not the cup's official one, although it has every CPK of this run. The line\nnames the entries it is missing, the entries the official list does not have, or says that the\norder differs. `dpfilelist_cpk_missing` names CPKs the list has that are not in the `download`\nfolder, in which case the game loads none of the CPKs there. Both lines name\n`UTW compiler upgrade-dpfl`, which installs the official list.\n\nA game folder with no `DpFileList.bin` in its `download` folder is the error\n`dpfilelist_missing`, and the CPK is left in the output folder the same way.\n\n**`upgrade-dpfl`**\n\n`upgrade-dpfl` installs the cup's official `DpFileList.bin` in the `download` folder of the game\nfolder (the `pes_folder_path` setting). It asks no question. Without `--yes` it writes\nnothing and only prints, one line each, what `--yes` would do, then\n`dpfilelist_upgrade_planned`, which names the command to run with `--yes`. With `--yes` it does\nthose things in this order, printing each line once it is done:\n\n- `dpfilelist_cpk_renamed` (`from=...`, `to=...`): a CPK of an older DLC renamed to the official\n  name of its kind, so the game still loads it under the official list. CPKs of one kind keep\n  their order: `4cc_40_faces.cpk` and `4cc_45_uniform.cpk` become `4cc_41_teams.cpk` and\n  `4cc_42_teams.cpk`, `4cc_30_stadiums0.cpk` becomes `4cc_20_stadiums.cpk`. One already under an\n  official name moves too when a file before it in the list takes that name, and a CPK is never\n  renamed over a file that is already there.\n- `dpfilelist_placeholder_written` (`cpk=...`): the empty placeholder CPK written for a CPK of\n  the official list that is not in the folder, since the game loads none of the CPKs there when\n  one it lists is missing. A file that is there is never overwritten.\n- `dpfilelist_replaced` (`path=...`): the list replaced by the official one. Your old list is\n  kept beside it as `DpFileList.bin.bak` (`backup=...`), replacing an older backup. An old\n  list that cannot be read as a list is replaced the same way, the line saying why\n  (`unreadable=...`), and nothing is renamed. A folder with no list gets the official one, and\n  no backup.\n- `dpfilelist_cpk_dropped`, a warning (`cpk=...`, `size=...`): a CPK of your old list that the\n  official list does not have and that could not be renamed (no official name of its kind left\n  for it, a name not of the `4cc_NN_kind.cpk` shape, or its new name taken by another file). The\n  game no longer loads it. The command never deletes a CPK. The line gives the file's size (no\n  size when the file is not there), and removing it is up to you.\n\nWhen the list already is the official one and every CPK it names is in the folder, the only\nline is `dpfilelist_up_to_date`, and nothing is written.\n\n`upgrade-dpfl` refuses to run (exit code 2) when `pes_folder_path` or the `download` folder in\nit is not a folder. Set `pes_folder_path` to the folder PES is installed in. If it already is,\ncreate the `download` folder in it.\n\nWhen a file cannot be renamed or written (PES is running and holds the CPK, or the folder needs\nadministrator rights), `upgrade-dpfl` stops with exit code 3, naming the file. What it did\nbefore stays done, and the list is replaced last, so the game keeps your old list until the end.\nClose PES, or run Unlimited Tool Works as administrator, and run the command again to finish. A\nCPK it already renamed is not renamed again.\n\n**`--mode sideload`**\n\n`--mode sideload` is for trying a change in a running game. Instead of a CPK, `compile` writes\nwhat the CPK would hold as loose files in the `livecpk` folder of the game folder (the\n`pes_folder_path` setting), replacing everything that was there, for FoxDen (PES 2018 to 2021)\nor Sider (PES 2017) to load while the game runs. The folder is replaced only once the whole\ncompile is written. A compile that fails, or writes nothing, leaves it as it was. Nothing is\ninstalled into the game's `download` folder, so a missing `DpFileList.bin` is only a warning.\n`--mode sideload` is refused for PES 2015 and 2016, which nothing can sideload into, and when\n`pes_folder_path` does not name a folder.\n\n**`--mode test`**\n\n`--mode test` shows what the compiler made of each export. Instead of a CPK, `compile` writes\nwhat it made as loose files in the `test_output` folder of the output folder, one folder per\nexport, named as the export is (an archive's with its extension). Each file goes in the folder\nof the export it came from: a player's models unpacked from their packages beside his converted\ntextures, a kit's files in its kit folder. The bins go in `test_output/_bins`, at the paths they\nhave in a CPK. Nothing is installed into the game, and the `overrides` folder is not applied,\nsince its files are not an export's. The folder is replaced only once the whole compile is\nwritten. A compile that fails, or writes nothing, leaves it as it was.\n\n**`multicpk_mode`**\n\nThe `multicpk_mode` setting is for the cup maintainers building the whole cup's DLC. With it on,\n`compile` writes several CPKs instead of `<cpk_name>.cpk`. The teams go into the numbered CPKs\nthe cup's official `DpFileList.bin` has for the `teams_cpk_name` setting (`teams` gives\n`4cc_41_teams.cpk` to `4cc_45_teams.cpk`, filled in that order), whole teams in the order of the\nexports. A CPK takes teams until the next one would make it larger than `cpk_part_max_size`\nbytes (3 GiB unless you changed it), and that team starts the next CPK. A team is never split\nbetween two CPKs. Every numbered CPK the teams do not need is written as the empty placeholder\nCPK, so a CPK left from an earlier, bigger compile does not stay behind. The files the compiler\nbuilds for every team (team colors, kit colors, kit configs) and the files of the `overrides`\nfolder go into the CPK named by the `bins_cpk_name` setting (`4cc_08_bins.cpk`).\n\nEvery CPK of the compile is installed into the game's `download` folder, or none is. The game's\n`DpFileList.bin` must have all of them (a list without the `teams` CPKs is one\n`dpfilelist_outdated` line naming them). When one cannot be installed (PES holds it open, or the\nfolder cannot be written), the old CPKs stay as they were and every CPK of the compile is left\nin the output folder (`output=...` names the folder). With `--no-deploy` every CPK is left in\nthe output folder, with one `deploy_skipped_by_flag` line each, the bins CPK first.\n`--mode test` and `--mode sideload` write their loose files as they do without it.\n\nThese lines stop a multi-CPK compile with exit code 3. No CPK is written, and the ones in the\noutput folder stay as they were:\n\n- `cpk_slots_exhausted` (`export=...`, `stem=...`, `slots=...`, `cap=...`): the teams do not fit\n  in the numbered CPKs the official list has, at `cpk_part_max_size` each. The line names the\n  first export left without room and how many CPKs the list has. Raise `cpk_part_max_size`, or\n  ask for a list with more of them.\n- `cpk_team_exceeds_cap` (`export=...`, `size=...`, `cap=...`): one team alone is larger than\n  `cpk_part_max_size`, and a team is never split. Raise `cpk_part_max_size`.\n\nWithout `multicpk_mode`, the compile's one CPK is never split. When `<cpk_name>.cpk` is larger\nthan `cpk_part_max_size`, the warning `cpk_size_over_limit` (`cpk=...`, `size=...`, `cap=...`) says so. The game loads it\nas it is and the CPK is written whole, but the cup DLC's repository cannot hold a file that\nlarge.\n\n**Compressed `.dds` files on PES 2015 to 2017**\n\nFor PES 2015 to 2017, the `dds_compression` setting zlib-compresses every `.dds` file `compile`\nwrites, the way the game's own compressed files are. The game reads such a file as it reads a\nplain one, and the DLC gets smaller. A `.dds` of the export that is already compressed this way,\nin a format the version reads, is written as it is. The setting is `auto` unless you changed it:\n`auto` compresses when `multicpk_mode` is on and not otherwise, `true` always compresses and\n`false` never does. PES 2018 to 2021 ignore it, since their textures are `.ftex` files.\n\n**Exit codes**\n\nWhen a command ends, its exit code tells a script how it went:\n\n| Code | Meaning |\n|---|---|\n| 0 | Finished cleanly. Warnings and notes may have been reported. |\n| 1 | Finished, but with an error reported, for example because something of an export was left out, or kept because `pass_through` is on, or the CPK could not be installed into the game and was left in the output folder. |\n| 2 | The command line or a setting is wrong, for example a `cpk_name` that is not a valid file name. Nothing ran. |\n| 3 | The run stopped early, for example because the teams list cannot be read or the output folder cannot be written. |\n\n### The aesthetics patch and your save\n\nEvery `compile` that builds players also writes `aesthetics_patch.toml` in the output folder,\nwhatever the PES version. It holds each compiled player's look:\n\n- his appearance and motions, taken from his `settings.toml` and `ingame_face.toml` (a key\n  neither file sets gets the game's default);\n- the IDs of the boots and gloves he was compiled with;\n- only for an export whose root holds the `write_names` marker file, his name and shirt name.\n\nA `settings.toml` or `ingame_face.toml` the run cannot read leaves his look and his IDs out of\nthe patch, so the save keeps what it has for him. The cup's official save is built with that\nfile: the cup maintainer compiles the exports and hands over the CPKs and the patch, and the\nsave builder applies the patch with `UTW save-editor apply-patch`.\n\n`compile` also applies the patch to your own save, so what you just built shows in your\ngame, but only when the game will load what the run installed: after a CPK was installed\ninto the game's `download` folder, and in `--mode sideload`. It is not applied with\n`--no-deploy`, in `--mode test`, after an install that failed and left the CPK in the\noutput folder, or when `dpfilelist_cpk_missing` warns that the game will run without the\n`download` folder's CPKs at all.\n\nThe `savefile_path` setting says which save is yours. `auto`, the default, looks for it\nunder `Documents\\KONAMI` for the PES version you compile for, and picks the save written\nlast when the game has more than one account folder (the note `savefile_autodetected`\nnames it). Give a path yourself to use another save, a cup's template save for one.\n\nWhile that PES version is running, the game writes the save it holds over the file when you\nleave its Edit mode after changing anything there, so a save written while Edit mode is open\nis lost then unless Edit mode's Load reads it in first. `compile` therefore writes the save\nwhile the game runs only when you add `--edit-mode-closed`, which says you have closed Edit\nmode. Without it, the save step is skipped with the warning `savefile_skipped_pes_running`,\nwhich names the game and the flag: close Edit mode, then compile again with\n`--edit-mode-closed`. The save is then written, and opening Edit mode reads it in without\nrestarting the game. A PES of another version running does not count.\n\nWhen the save is written, the one you had is kept beside it with `.bak` added to its name\n(`EDIT00000000.bak`). A save that cannot be read, is for a PES version other than the one\ncompiled for, or cannot take the patch or be written (`savefile_invalid`,\n`savefile_write_failed`) stops the run with exit code 3. The new CPKs come out of the game\nagain and are left in the output folder, and the CPKs the game had go back in. In\n`--mode sideload` the same findings are errors instead: the exit code is 1 and the new\n`livecpk` files stay in place. When no save is found at all, that is only the warning\n`savefile_missing`, which names the patch file and the command that applies it by hand (a\n`savefile_path` naming a folder is `savefile_invalid` instead).\n",
          "slug": "command-line",
          "title": "Running from a terminal"
        }
      ]
    },
    {
      "id": "save-editor",
      "messages": [
        {
          "code": "team_exported",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "`export-toml` wrote one team's file."
        },
        {
          "code": "import_key_not_in_version",
          "effect": "The key is dropped from the import, and the rest of the import is written.",
          "severity": "Warning",
          "when": "The team file holds a key the file's version stores but the save's does not."
        },
        {
          "code": "import_value_not_encodable",
          "effect": "The value is written as off (a playing style as None), and the rest of the import is written.",
          "severity": "Warning",
          "when": "The team file holds a value the save's version cannot store: an instruction or a playing style it lacks."
        },
        {
          "code": "import_value_capped",
          "effect": "The value is lowered (the face type to 0, the motion to its last, the skin to 1), and the rest of the import is written.",
          "severity": "Warning",
          "when": "The team file holds a value above what the save's version stores: a face type above its cap, a motion above its last, or a custom skin it has no slot for."
        },
        {
          "code": "import_slot_empty",
          "effect": "The player is skipped, and the rest of the import is written.",
          "severity": "Warning",
          "when": "The team file holds a player for a roster slot the team leaves empty."
        },
        {
          "code": "import_text_cropped",
          "effect": "The text is cut to what the line shows, and the rest of the import is written.",
          "severity": "Warning",
          "when": "The team file holds a name or shirt name longer than the save's version has room for."
        },
        {
          "code": "patch_team_applied",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "`apply-patch` found the team in the save and wrote at least one of its patch players. A team whose patch players the save lacks entirely is not listed."
        },
        {
          "code": "patch_team_not_in_save",
          "effect": "The rest is written, and the command exits 1. When `--out` is the same file as `--base` and the save holds none of the patch's players, the save is left as it was.",
          "severity": "Error",
          "when": "`apply-patch` could not find a team with this id in the save, so its players were not looked at."
        },
        {
          "code": "patch_player_not_in_save",
          "effect": "The rest is written, and the command exits 1. When `--out` is the same file as `--base` and the save holds none of the patch's players, the save is left as it was.",
          "severity": "Error",
          "when": "`apply-patch` could not find a player with this id in the save."
        },
        {
          "code": "player_diff",
          "effect": "It is a report row. Differences are the output, so the command exits 0.",
          "severity": "Info",
          "when": "`diff` lists one field of a player who is in both saves, old value to new."
        },
        {
          "code": "player_only_in",
          "effect": "It is a report row, and there is nothing to do.",
          "severity": "Info",
          "when": "`diff` found a player who is in one save alone (the line says which)."
        },
        {
          "code": "teams_list_added",
          "effect": "The change is merged into the list, which is written only with `--yes`.",
          "severity": "Info",
          "when": "`export-teams-list` found a team in the save that the list lacked, and appended it or put it into a placeholder's slot."
        },
        {
          "code": "teams_list_overridden",
          "effect": "The change is merged into the list, which is written only with `--yes`.",
          "severity": "Info",
          "when": "`export-teams-list` found a name in both the save and the list, and gave it the save's id."
        },
        {
          "code": "teams_list_unresolved",
          "effect": "The other changes are written with `--yes`, and the command exits 1.",
          "severity": "Error",
          "when": "`export-teams-list` dropped a change because two teams would then share one id in the merged list."
        },
        {
          "code": "teams_list_name_duplicate",
          "effect": "The team is left out, and the command exits 1. Rename one of them in the save to keep both.",
          "severity": "Error",
          "when": "`export-teams-list` left a team of the save out because an earlier team has the same name, upper and lower case aside, and the list can hold a name once."
        },
        {
          "code": "teams_list_kept",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "`export-teams-list` reports the rows that ended unchanged (the line gives the count)."
        },
        {
          "code": "teams_list_written",
          "effect": "Nothing to do.",
          "severity": "Info",
          "when": "`export-teams-list` wrote the merged list."
        },
        {
          "code": "teams_list_not_written",
          "effect": "Run it again with `--yes` to write the list.",
          "severity": "Info",
          "when": "`export-teams-list` ran without `--yes`, so it printed the summary only and left the file untouched."
        },
        {
          "code": "teams_list_read_only",
          "effect": "Nothing is written, and the command exits 1.",
          "severity": "Error",
          "when": "`export-teams-list --yes` could not write the file into the data folder, because none is chosen yet or it cannot be written to (the line gives the reason)."
        },
        {
          "code": "aatf_violation",
          "effect": "An error-severity violation makes the command exit 1, while a warning alone leaves it at 0.",
          "severity": "Error or Warning",
          "when": "`check-aatf` found one rule the team breaks under the ruleset (the line names the team, the player for a player's violation, the rule and what is wrong)."
        },
        {
          "code": "aatf_check_failed",
          "effect": "The other teams are still checked, but the command exits 1.",
          "severity": "Error",
          "when": "`check-aatf` could not check the team, because the ruleset's check itself failed on it, so the team has no result (the line gives the error)."
        },
        {
          "code": "aatf_ruleset_gone",
          "effect": "The official ruleset built into Unlimited Tool Works, which the line names, is checked instead. Pick the cup's rules again in the dropdown on the Save editor's toolbar.",
          "severity": "Warning",
          "when": "`check-aatf` ran without `--rules`, and the ruleset picked last in the Save editor is gone from the data folder or no longer loads (the line names it)."
        },
        {
          "code": "fpc_inners_break_hiding",
          "effect": "Nothing is changed for you. Take the inners off, or the body will not hide.",
          "severity": "Error",
          "when": "The player wears inners under a hidden body, which breaks the hiding."
        },
        {
          "code": "fpc_undershorts_break_hiding",
          "effect": "Nothing is changed for you. Take the undershorts off, or the body will not hide.",
          "severity": "Error",
          "when": "The player wears undershorts under a hidden body, which breaks the hiding."
        },
        {
          "code": "fpc_taping_shows_pieces",
          "effect": "Some custom setups do this on purpose. If yours does not, take the taping off.",
          "severity": "Info",
          "when": "The player wears wrist or ankle taping under a hidden body, so the taped pieces still show."
        },
        {
          "code": "fpc_gloves_checkbox_winter_gloves",
          "effect": "Nothing is changed for you. Set Outfielder gloves to Off, or give the player a gloves ID, if that is not wanted.",
          "severity": "Warning",
          "when": "Outfielder gloves is Worn in winter with Gloves ID 0, so the player wears winter gloves in winter conditions."
        }
      ],
      "title": "Save editor",
      "topics": [
        {
          "body": "The Save editor's batch commands run from a terminal opened in the folder that holds `UTW.exe`.\nEvery command names the save it reads (the `EDIT00000000` file of your PES install's save\nfolder, or a copy of it), and no command picks one for you.\n\n```text\nUTW save-editor export-toml <save> --team <id> --out <file>\nUTW save-editor export-toml <save> --all-teams --out-dir <folder>\nUTW save-editor import-toml <team.toml> --base <save> --out <save> [--edit-mode-closed]\nUTW save-editor apply-patch <aesthetics_patch.toml> --base <save> --out <save> [--edit-mode-closed]\nUTW save-editor diff <first save> <second save> [--aesthetics | --gameplay] [--json]\nUTW save-editor check-aatf <save> [--teams <list>] [--rules <file|name>]\nUTW save-editor export-teams-list <save> [--yes]\nUTW save-editor unpack-session <session zip> <folder>\n```\n\n**Writing a save**\n\nA command that writes a save reads the one `--base` names and writes the one `--out` names,\nwhich may be the same file. It prints its lines only once `--out` is written, so a write that\nfails prints only its failure. A file already at `--out` is first copied to `<file>.bak`, and\nonly one copy is kept: the next write replaces it. No command creates a save from nothing.\n\n**While the game is running**\n\nWhile the PES version the save belongs to is running, the game holds its own copy of the save.\nWhen you leave Edit mode after changing anything there, the game writes that copy over the\nfile, so a save a command wrote while Edit mode was open is lost, unless Edit mode's Load\nread it in first. That is why, while the game runs, a command writes a save only when you add\n`--edit-mode-closed`, which tells it you have closed Edit mode.\n\nWithout it, the command writes nothing and names the running game: close Edit mode, then run\nthe command again with `--edit-mode-closed`. Opening Edit mode then reads the save in without\nrestarting the game. A PES of another version running does not count.\n\n**Exporting teams**\n\n`export-toml` writes a team of the save as a Team TOML file: the team's names, colors, manager\nand stadium, its tactics, and every rostered player with every field, the look included.\n\n- With `--team` one team goes to the file `--out` names, replacing it when it exists.\n- With `--all-teams` every cup team of the save (ids 701 to 920) goes to its own `<id>.toml`\n  in the `--out-dir` folder, which is created when missing and whose older files are\n  replaced.\n\nA team id the save does not hold, or an `--out` naming the save itself, is refused and nothing\nis written.\n\n**Importing a team**\n\n`import-toml` writes a Team TOML file onto the team its `[team] id` names. The file's keys\ndecide what it writes: a key, a player or a whole section missing from the file stays as the\nsave has it, so a file holding only `[team] id` and `[players.03] name = \"Snuffy\"` renames one\nplayer and nothing else.\n\nA file written from another PES version is converted on the way in, and each value that could\nnot be taken as written prints one line:\n\n- a key the save's version has no field for (`import_key_not_in_version`);\n- a value it cannot store (`import_value_not_encodable`);\n- a face type above the target's cap or a custom skin (7) on a version that has none\n  (`import_value_capped`, which writes the face type as 0 and the skin as 1);\n- a player table for an empty roster slot (`import_slot_empty`).\n\nThose lines do not change the exit code. A file with no `[team] id`, one naming a team the save\ndoes not hold, or one with a name longer than the save's version has room for is refused,\nnaming the team or the player, and nothing is written.\n\n**Applying an aesthetics patch**\n\n`apply-patch` applies the `aesthetics_patch.toml` the Compiler writes beside its CPKs: the\ncompiled players' motions and, for exports carrying the `write_names` marker, their names. The\npatch must be for the save's PES version and the suite's boots and gloves allocation scheme, or\nit is refused with nothing written.\n\nEach team of the patch prints one line, `patch_team_applied`, but only when at least one of its\nplayers was written: a team whose patch players the save lacks entirely is not listed. A team\nor a player the save does not hold prints the error line `patch_team_not_in_save` or\n`patch_player_not_in_save`. The rest of the patch is applied and written, and the command\nexits 1.\n\nWhen `--out` is the same file as `--base` and the save holds none of the patch's players, the\nsave is left as it was, so its `.bak` stays your previous backup. The lines then print once\nthe command finds that nothing needs writing.\n\n**Comparing two saves**\n\n`diff` lists what differs between two saves of the same version, player by player, paired by\nplayer id. It compares the players a cup team's roster holds in either save (ids 701 to 920)\nand leaves the game's own teams out:\n\n- one line per changed field, `player_diff`, with the field, the first save's value and the\n  second's;\n- a player who is in one save alone is `player_only_in`.\n\n`--aesthetics` keeps only the look's fields (appearance id, boots, gloves, base copy, physique,\nstrip, face, motions, star rating, playing attitude), and `--gameplay` keeps everything else\n(names, basics, abilities, positions, skills).\n\nWith `--json` the report is instead one JSON document on stdout, for scripts to read. It lists\nthe players with their changed fields, then the players only one save holds. Nothing else is\nprinted.\n\nDifferences are the output, not errors, so the exit code is 0. Two saves of different versions\nare refused.\n\n**Checking teams against AATF rules**\n\n`check-aatf` checks the save's cup teams against an AATF ruleset: every team whose id is in the\ncup range, 701 to 920, or the ones `--teams` names as a comma-separated list of ids and\ninclusive ranges (`701,705-710`).\n\n`--rules` names a rules file, read from where the command runs. When no file has\nthat name, it names a built-in ruleset by its short name (`--rules dbc7`, `--rules vgl27`). A\nvalue that is neither is refused, and the refusal lists the built-in names. Without `--rules`,\nthe ruleset is the one picked in the dropdown on the Save editor's toolbar (the `aatf_ruleset`\nsetting): a built-in ruleset or a copy in the data folder's `rules` folder. When that copy is\ngone, the warning line `aatf_ruleset_gone` says so and the official ruleset built into\nUnlimited Tool Works, 4cc Autumn 26, is used instead.\n\nEach violation prints as one `aatf_violation` line with the team, the player's id for a\nplayer's violation, the rule and what is wrong, and the teams print in id order. A team\nwhose check cannot run prints the error line `aatf_check_failed`, and the rest are still\nchecked.\nAn error-severity violation makes the command exit 1, while warnings alone leave it at 0.\n\nA rules file that does not load is refused, naming the line and column or the key path it\nfailed at, so the command also validates a rules file. A `--teams` id the save does not hold is\nrefused as well.\n\n**Updating the teams list**\n\n`export-teams-list` merges the save's teams into `teams_list.txt` in the data folder, the list\nthe Compiler looks export names up in; when that file does not exist yet, the merge starts\nfrom the list shipped with Unlimited Tool Works. It takes every team whose name is\nslash-wrapped (`/co/`) and whose id is in the cup range, 701 to 920.\n\nIt prints what the merge does, one line each:\n\n- a team the list lacked (`teams_list_added`);\n- a name already listed under another id, which takes the save's id instead\n  (`teams_list_overridden`);\n- a change dropped because two teams would share one id (`teams_list_unresolved`, an error\n  line);\n- a team left out because an earlier team of the save has the same name, upper and lower case\n  aside (`teams_list_name_duplicate`, an error line naming the team kept, so fix the save);\n- how many rows stay as they were (`teams_list_kept`).\n\nWithout `--yes` it stops there (`teams_list_not_written`); with `--yes` it writes the merged\nlist (`teams_list_written`), with the dropped changes left out. When the data folder cannot\ntake the file, or none is chosen yet, the command prints the error line `teams_list_read_only`\nand writes nothing.\nThe exit code is 1 when an error line was printed, and 0 otherwise.\n\n**Unpacking a report's earlier sessions**\n\nA problem report saved with Include earlier sessions ticked holds each earlier session as a\nzip in its `sessions/` folder, with its saves packed. `unpack-session` writes such a zip's\nfiles back as they were: each file the session read into `<folder>/inputs/` and each file it\nwrote into `<folder>/outputs/`, named with its number in the session (`0-EDIT00000000`). The\nfolder must be new or empty. A save that does not rebuild to the size and SHA-256 its session\nnoted stops the command before anything is written.\n\n**Exit codes**\n\nEach command ends with an exit code scripts can test:\n\n- 0 when it did its work;\n- 1 when it did its work but reported an error;\n- 2 when it was refused before writing anything (a bad argument, a file it cannot read, a team\n  it cannot find);\n- 3 when a write failed (what was written before it is kept).\n",
          "slug": "command-line",
          "title": "Running from a terminal"
        },
        {
          "body": "The Save editor's window opens a save (`EDIT00000000`, or a copy of it) and edits it.\n\n**Opening a save**\n\nWith no save open, the middle of the window offers **Open…**, the saves found in\nDocuments and the recent saves. The **File** menu lists, from the top:\n\n- **Open…** (Ctrl+O), which picks any file and starts in the folder the last save was opened\n  from when it exists, or in the newest found save's folder;\n- **Save** (Ctrl+S) and **Save as…** (Ctrl+Shift+S), which write the open save (see **Saving**\n  below) and stay greyed out until a save is open;\n- under **Open from Documents**, the saves found in your Documents folder for every PES version,\n  each named by its version, so your game's save opens in one click (hover an entry to see\n  its full path);\n- under **Recent files**, the last 10 saves you opened or saved, the most recent first,\n  remembered for the next time the program opens. Each is named by its version, folder and\n  file name, so two saves of the same name stand apart;\n- **Close save** (Ctrl+W), which closes the save and brings back the list of saves in the\n  middle of the window (greyed out while no save is open);\n- **Exit** (Ctrl+Q), which closes the window as its close button does.\n\nCtrl+5 to Ctrl+9 open the newest save found for PES 2015 to PES 2019, Ctrl+0 PES 2020's and\nCtrl+1 PES 2021's, or the file picker when none was found. They do nothing while you type in\na field or a dialog is open, and neither do Ctrl+O and Ctrl+W.\n\nOnce a save is open, the window's title bar shows its path, in square brackets after the\neditor's name, and the toolbar shows `• unsaved changes` while edits wait unsaved.\n\n**The save's PES version**\n\nThe editor reads a save's PES version from the file itself and edits it as that version, so\nthe sidebar's version box stays empty here. The players' portraits come from that version's\nPES folder.\n\n**Teams and players**\n\nThe dropdown above the list picks a team, or All teams for every cup team of the save.\nPicking a team selects its row, so the right side shows the team's fields and tactics;\nAll teams keeps what you had selected.\n\nThe radio buttons at the bottom of the open dropdown narrow the dropdown's teams and the\nlist to one competition's teams: 4cc, VGL or Invitational; All is every cup team again.\nWith one of them ticked, the dropdown's first entry reads its name in place of All, as in\nVGL teams. If the picked team is not in the category, the dropdown shows that entry. The\npick is remembered for next time.\n\nOnly the cup's teams are listed (ids 701 to 920), with the players on their rosters. A save\nmade from the game's own data also holds its clubs and national teams, and players on no cup\nteam: they stay in the save exactly as they are, and the editor never lists, checks or\nchanges them.\n\nThe list holds a row per team and, under each team, a row per player: the team's row shows\nits id, its two colors and its name, and a player's row his place in the roster, position\nand name as the game draws it, colors included. A `●` on a row marks\na player or the team itself with unsaved changes.\n\nThe filter box above the list finds a team or a player by part of a name or shirt name,\nor by the start of an id, so you can find someone without knowing their team first. It\nsearches every team the dropdown lists, even when one team is picked. A team whose name\nor id matches is listed with its whole squad. Double-click a team's row to show that\nteam alone: the dropdown picks it and the filter box empties.\n\nWhile no field has the keyboard and no menu or dialog is open, Up and Down move through\nthe list's rows, team rows included. Page Up and Page Down work from a field too: they\nmove to the previous or next team's row, or seven rows at a time when only one team is\nlisted.\n\n**Reordering a team**\n\nWith the filter box empty, drag a player row onto another row of his team to move him to\nthat slot; a drop on another team's row does nothing. The shirt number and the in-game face\nand hairstyle belong to the slot, so they stay where they were.\n\n**Editing**\n\nClick a player's row to select him: the right side shows his id and name, and under them his\nfields. The toggle left of the name switches between **Stats**, three columns of his fields,\nand **Appearance**, his look. Click a team's row, or **Gameplan** on the toggle, to see the\nteam's fields and its tactics.\n\nA player's Name and Shirt name labels count what the field holds (`(12/61)` reads as 12 used\nof 61). When it is longer than an older version can hold, the field takes an amber tint and\nits count turns red. A `\\x11c` plus 8 hex digits inside a name is a color code, and the field\nkeeps it as typed.\n\n**Undo and redo**\n\nEvery edit can be undone: the **Edit** menu's **Undo** (Ctrl+Z) and **Redo** (Ctrl+Y or\nCtrl+Shift+Z) step through your edits. A drag, or a run of typing in one field, is one undo\nstep.\n\n**Copy and paste**\n\nCtrl+C copies the selected row, and Ctrl+V pastes it onto another selected row of the same\nkind: a player's row copies his stats (the fields the **Player** menu's **Copy/swap stats…**\nmoves), and a team's row copies its Gameplan. Each paste is one undo step. The **Edit** menu's\n**Copy** and **Paste** do the same as the two keys.\n\nOn a team's **Gameplan** screen, a click on a **Preset** button (1, 2 or 3) or a formation\nbutton (**Kick-off**, **In-possession** or **Out-of-possession**) makes Ctrl+C copy that\npreset or formation, and Ctrl+V paste over the one shown. That lasts while the team's row\nstays selected: select another row, or click the team's row, and the keys copy and paste\nwhole rows again.\n\nThe copy is text on the system clipboard, so it also pastes into another Save editor window,\nor after another save is opened, even a save of another PES version. When the target's\nversion cannot hold a value as it was, the status bar says how many were left out or\nchanged.\n\nWhile you type in a text field, Ctrl+C and Ctrl+V copy and paste its text, and Ctrl+C with\ntext selected on screen copies that text.\n\n**Quick actions**\n\nUnder the list, one **Make \\<tier\\>** button per tier of the AATF rules in use gives the\nplayer that tier's values (stats, form, injury resistance, weak foot, name color) as one\nundoable edit. **Set stats** sets every ability stat to the number beside it (40 to 99).\nThese buttons act on the selected player, so they stay greyed out while a team row is\nselected.\n\n**AATF ruleset**\n\nThe dropdown on the toolbar, left of **Live AATF**, picks the AATF rules the quick actions,\nthe Cards used badge and Live AATF use. Every cup's current rules are built in, listed by\nname, and below them come the rules files in the data folder's `rules` folder, where\n**Import rules file…** puts them. The pick is remembered. The ruleset's event logo shows at\nthe window's top right, and when the ruleset names its event's page, a click on the logo\nopens it. When the file picked last is gone from the data folder, 4cc Autumn 26 is used\ninstead and a notice says so.\n\n**Import rules file…** at the bottom of the list adds a cup's `.4ccrules` file: it is copied\ninto the data folder and picked, while a file with errors is refused with its errors listed,\nso you can fix it and import it again. The **×** beside an imported ruleset removes it from\nthe list after asking, deleting only the data folder's copy, never the file you imported.\n\n**Comparator**\n\n**Comparator…**, in the toolbar's **Savefile** menu, opens a floating window comparing the\nopen save, edits included, against a second save: **Pick…** chooses it, and refuses a save of\nanother PES version, naming both versions. The **Show** row narrows the list to All, Gameplay or Aesthetics, and\nthe team dropdown to one team.\n\nRows group by team and by player: each row shows a field's name and its value in the picked\nsave and in the open save. Clicking a row selects that player in the editor. Only the players\non a cup team's roster, in either save, are compared: the game's own teams are left out.\n\nA comparison takes a moment on a big save, and the window says so until the rows land.\n**Export…** writes the differences the Show row picks to a text file, for every team,\nwhatever the team dropdown shows.\n\n**Live AATF**\n\nThe toolbar's **Live AATF** toggle checks the open team after every edit and lists, in a\npanel docked at the bottom of the window, the violations left: the header reads\n`N issues · <ruleset>`, and the rows are grouped per player, with the team's own under\n`Team-wide issues`. Once the team passes, the header reads `<team> passes <ruleset> with no\nissues`. Clicking a row selects that player. A Fielded position row, about a position he is\nfielded at, selects his team's row instead and shows the first formation of the named preset\non which his portrait is crossed out (see below).\n\nThe icon near the right end of the panel's header opens the list in a window of its own, which\nyou can move and resize to keep it beside what you are fixing. The window lists the players\none under the other, its icon beside the close button puts the list back in the panel, and\nclosing the window turns Live AATF off. The panel's own close button, beside its icon, turns\nLive AATF off too. The editor remembers where you left the list, so turning Live AATF on\nagain opens it there.\n\n**Copy to clipboard**, left of that icon in the panel and under the close button in the\nwindow, copies the list as it shows, as plain text you can paste into a message: a line\nnaming the ruleset, the team with its error and warning counts, then the same groups.\n\nWhile it is on, the editor itself is marked too:\n\n- each player of the team gets a mark at the end of his row in the list: a red `✖` when he\n  breaks a rule, otherwise a `✔`, amber when he has a warning, blue when he has only\n  suggestions and green when there is nothing to fix (hover over it to see how many of each\n  he has);\n- a field of the selected player that a row is about gets a mark just left of its label, or\n  after the panel's title when the row is about a whole panel (on Skills, after **Cards\n  used**): a red `✖` for an error, an amber `⚠` for a warning and a blue `ⓘ` for a\n  suggestion (hover over it to read its rows);\n- on the tactics pitch, a player fielded at a position he is not rated A at has his portrait\n  crossed out, on every formation the preset fields him at that position.\n\n**Show suggestions**, beside the toggle while it is on, also lists each player's unused\nallowances (cards he could still take, free tricks, weak foot) under `Suggestions`, after\nthe violations. It is off at first, because most players of a cup team leave some allowance\nunused. The toggle and the checkbox are remembered, so they are as you left them the next\ntime the editor is opened.\n\n**Saving**\n\nThe **File** menu's **Save**, or Ctrl+S, writes the edits to the file the save was opened\nfrom (a file it replaces is kept once as a `.bak` copy). **Save as…**, or Ctrl+Shift+S,\nwrites them to a file you pick instead, and the editor then carries on with that file, so\nthe next Save writes there too. A file it replaces is kept as a `.bak` copy as well.\n\nWhile the save's own PES version is running, saving first shows how to keep the save:\n\n- if the game's Edit mode is open, use its Data Management > Load after saving, before you\n  leave Edit mode. Leaving Edit mode after changing anything in it writes the game's own\n  copy over your save;\n- if Edit mode is closed, the changes show when you open it.\n\nWhile only a PES of another version is running, saving asks nothing.\n\n**Unsaved changes**\n\nClosing the save or the window, or opening another save, with unsaved edits asks first,\nover a table of each changed field with its value in the file and in the editor. Save\nwrites the edits, Discard loses them, and Cancel keeps everything as it was.\n\nEach row ends with an ×, which puts that one change back as the file has it and keeps\nyour other edits, so you can drop a change you did not mean and save the rest. The × is\nan undo step like any edit, so Undo brings the change back. Once no row is left there is\nnothing to save, so what you asked for goes ahead: the save or the window closes, or the\nother save or the file picker opens.\n\nA player you dragged to another slot in the list shows as one `Player order` row of his\nteam, which lists the slots he and the players he shifted held in the file beside the\nslots they hold now, one slot per line. Each of those players lists only what you\nedited on him, since the drag itself is that one row. On that row the × puts every\nplayer of the row back in his slot in the file, and each keeps what you edited on him.\n\nClicking `• unsaved changes` on the toolbar shows the same list, ×'s included. It stays\nopen a row shorter after each ×, and closes when no row is left.\n",
          "slug": "window",
          "title": "The window"
        },
        {
          "body": "Clicking a player's row opens the player view: his id and name on top, then the **Stats**\nscreen's three columns side by side. The toggle left of his name switches to **Appearance**.\n\n**The three columns**\n\nThe first column is his **General info**: name and shirt name, shirt number,\nage and nation. Nation is picked from the game's own nations for the save's\nversion, each shown after its ID; one that version lacks shows as its ID and a\nquestion mark. Under it, **Main stats** holds his captaincy, height and\nweight, injury resistance, form, weak foot usage and accuracy, stronger foot\nand hand, registered position and playing style. Then come his **Playable\npositions** with their C/B/A ratings. The second column is his **Skills** and\n**COM playing styles**. The third is his **Ability stats**.\n\n**Playable positions**\n\nEach position's chosen rating is filled, green for A, blue for B, grey for C,\nand his registered position's cell is outlined. A click on a position's name makes it\nhis registered position. The Pitch | Left | List toggle under the heading picks one of\nthree layouts: Pitch draws the positions as they stand on the pitch, Left starts every\nrow at one left edge, and List gives each position a row of its own. Your pick is\nremembered the next time you open the editor.\n\n**Tricks and Free cards**\n\nWhen the AATF rules in use set some cards apart, a second skills panel stands\nbetween Skills and COM playing styles. **Tricks** holds the ruleset's tricks, which\nits free trick allowance usually covers. A ruleset without tricks shows **Free\ncards** instead, the skills its free-card rules name, including those free only\nfor a tier, a position or the captain.\n\n**Counts**\n\nThe Name and Shirt name labels count what the field holds, as typed: `(12/61)` reads\nas 12 bytes used of the 61 the field holds (a color code counts as the ten\nbytes it stores). The shirt name counts letters, 18 on most versions and 16 on\nPES 2016. When a name or shirt name is longer than an older version can hold, the\nfield takes an amber tint and its count turns red, and hovering them says what an\nolder game would do with it.\n\n**Value ranges**\n\nThe numbers are the game's own, the same ones the game's Edit mode shows:\nabilities run 40 to 99, form 1 to 8, weak foot 1 to 4. A value the save holds\noutside a range is shown as stored, and it changes only when you edit it.\n\n**Greyed controls**\n\nWhen the save's PES version does not have a field, its name and its control are greyed\nout, and hovering either names the first version that has it (for example PES 19+). The\nsame fields stay in the same places on every version, so the page never rearranges.\n\n**Cards used**\n\n**Cards used** counts the non-free cards the player holds: the skills and COM\nplaying styles that count against his tier's limit under the AATF rules in use\n(the ruleset picked in the dropdown on the toolbar). His tier is read as the AATF\ncheck reads it, from his stats, registered position and height.\n\nWhen no tier's condition holds for him, the badge says \"No tier matches\". The\nnumber goes red when he holds more than the limit. When the rules cannot read\nthe player's tier, the badge's place reads \"Tier check failed: click to see\nwhy.\", and a click shows why.\n",
          "slug": "abilities",
          "title": "Abilities & Skills"
        },
        {
          "body": "The **Appearance** screen holds a player's look, in groups from top to bottom:\n**Full Player Customization** with his skin and iris **Colors** at its right,\nhis **Strip style**, his **Physique** with his **Motion** and the game's own\n**Edit flags** beside it, and last **Presentation**. The toggle beside the\nplayer's name picks one of three screens: **Stats** shows his fields,\n**Appearance** his look, and **Gameplan** his team's fields and tactics.\n\n**When it is locked**\n\nA player's look can belong to his team's export: the export's `settings.toml`\nand models decide it, and the Compiler writes it into the save through the\naesthetics patch. A hand edit here would be silently overwritten the next time\nthe team is recompiled or patched.\n\nThat is what the **Aesthetics from exports** setting, in the Save editor's\nSettings section, is for. A cup whose looks are compiled from exports turns it\non, which puts on the aesthetics lock: the screen turns read-only under a line\nsaying the team's export owns the aesthetics, and every other control that\nwrites the look, such as the Team menu's FPC items, is locked too. The setting\nis off by default, for the cups whose looks live in the save, so every control\nthen edits and neither that line nor the switch below is drawn.\n\n**Edit aesthetics anyway**\n\nThe **Edit aesthetics anyway** switch, under that line, unlocks the screen for\nthe rest of the session, for the cases the lock does not cover: a team with no\nexport, or a cup-night fix when recompiling is not an option. Opening another\nsave keeps it unlocked. Starting the tool again turns it back off.\n\nThe note under the switch warns that a later patch overwrites whatever you edit\nby hand.\n\n**Full Player Customization**\n\nThe **Full Player Customization** group, first on the screen, is where FPC\ninvisibility lives: a line reading from the save whether his default body is\nhidden, then the **Hide default body** and **Show default body** buttons.\n\nThe reading says \"The default body is currently hidden\" when his boots ID in the\nsave is the hide ID from Settings (55 unless changed) or his skin is Custom, and\n\"The default body is currently shown\" otherwise. A player hidden by his model's\nown boots ID, such as a compile-allocated one, reads \"The default body is\ncurrently shown\", because the save cannot tell his model hides him, which is why\nthese are two buttons and not a checkbox.\n\n- **Hide default body** writes long sleeves, a tucked shirt and short socks, and\n  a boots or gloves ID of 0 takes the hide ID from Settings (55 and 11 unless\n  changed, while a model's own ID is kept).\n- **Show default body** writes short sleeves, an untucked shirt and standard\n  socks, and a boots or gloves ID that holds the hide ID goes back to 0.\n\nOn PES 2015 to 2017 both also turn a Custom skin into Light. Changing the hide\nID in Settings rewrites no save, so a player hidden with the old ID then reads \"The\ndefault body is currently shown\", and Show default body leaves his ID alone (it\nis no longer the hide ID).\n\n**Settings that interfere with a hidden body**\n\nUnder the reading the group lists the strip settings that would interfere with a\nhidden body (inners or undershorts that break the hiding, taping that still\nshows) and **Outfielder gloves** set to Worn in winter with a gloves ID of 0,\nwhich puts winter gloves on the player in winter conditions. They are reported,\nnever changed for you. The inners, undershorts and\ntaping follow the save's own reading, so they are listed only for a player the\nsave calls hidden.\n\nFor a player hidden by his model's own boots ID (a compile-allocated one such\nas 627), whom the reading calls shown, the inners, undershorts and taping are\nnot listed, though the game hides his body all the same. The Compiler's check knows the player folder's\nFPC marker, so it reports them for a compiled team.\n\n**Randomize**\n\n**Randomize**, in the Motion group, draws new values for the hunching, arm\nmovement, kick and celebration motions, each within the range the game's Edit\nmode offers, and the dribbling motion is left as it is. One Undo restores them\nall.\n\n**Presentation**\n\nThe **Presentation** group, last on the screen, holds **Star rating**, which\nsets how likely the player is to show up as the team's featured player in the\npre-match screen, and **Playing attitude**, which only affects cutscenes.\nNeither changes how he plays, but both belong to his team's export with the\nrest of his look, so the lock covers them too.\n",
          "slug": "appearance",
          "title": "Appearance"
        },
        {
          "body": "Clicking a team's row in the list opens the team view: the team's id and name as\nits title, one row holding the name, three-letter short name and two color\nfields, the manager and stadium IDs and the **Kit slots…** button, and the\nTactics below. A field the save's version does not hold stays in its place,\ndisabled, with a note naming the first version that has it.\n\n**Names and colors**\n\nThe name and short name fields keep only what the save can hold: the name up to\nthe save's byte limit (an accented letter counts more than one), the short name\nup to three letters, shown upper case.\n\nA color opens the same picker as elsewhere. The save keeps 64 steps per\nchannel, so the color written is the nearest one the save can hold, and the hex\nbeside the field shows what the save actually stores.\n\n**Kit slots**\n\n**Kit slots…** opens the kit slot assignments in a dialog (PES 2017 only) and\ncloses on OK. A bound kit slot is shown as the team id it points at, and the\nextra data the game keeps alongside is left as it was.\n\n**The Team menu**\n\nThe **Team** menu works on the selected row's team, a player row's own or a team\nrow's, and writes every filled roster slot in one undoable step:\n\n- **Set edited flag** marks every player of the team as edited.\n- **Set boots and gloves IDs for everyone…** opens a small dialog with a box for\n  the boot IDs and one for the glove IDs.\n- **Set player names to positions** renames every player of the team to his\n  registered position's full name.\n- **Clear visual flags** (Ctrl+F) clears the face, hair, physique and strip edit\n  flags on every player of the team.\n- **Hide default body (Enable FPC)** writes long sleeves, a tucked shirt and\n  short socks on every player of the team, and a boots or gloves ID of 0 takes\n  the hide ID from Settings (55 and 11 unless changed, while a model's own ID\n  is kept, as on the Appearance screen).\n- **Show default body (Disable FPC)** writes short sleeves, an untucked shirt\n  and standard socks on them all, and sets a player's hide IDs back to 0.\n- **Import / export team file…** opens a panel beside the views.\n- **Save a picture of the formations…** saves a picture of the team's formations\n  (below).\n\nOn PES 2015 to 2017 both FPC items also turn a Custom skin into Light.\n\nThe boots and gloves item, Clear visual flags and the FPC items stay disabled\nwhile the aesthetics lock is on (see the Appearance screen), because they write\nwhat the team's export owns and a hand edit would be overwritten the next time\nthe team is recompiled or patched. Ctrl+F does nothing while the lock is on.\n\n**The boots and gloves dialog**\n\nThe boots can:\n\n- stay **Unchanged**;\n- count up from a start ID with **Set values starting from:** (slot 1 gets the\n  start, slot 2 the next, and so on; the grey box beside the start shows slot\n  23's ID and follows it);\n- take one ID with **Set all to the same ID:**.\n\nThe gloves can stay **Unchanged**, take each player's boot ID with **Set like\nboot IDs**, or take one ID for all.\n\n**Set** writes the IDs and closes the dialog, **Cancel** closes it without\nwriting. An ID the save cannot hold is refused with a line under the boxes, and\nnothing is written.\n\n**Exporting a team file**\n\nThe import / export panel's top row exports the team:\n\n- **Team TOML…** writes the team, its tactics and its players with every value,\n  the same file the command line's `export-toml` writes;\n- **Texport…** writes a file the game's own team import reads (PES 2017 to\n  2021). On PES 2018 to 2021 it is a `.ted` file. On PES 2017 it is named\n  `TEXPORT00000000`, `TEXPORT00000001` and so on, and the window suggests the\n  first free name in the save's folder, where the game's Import Team finds it.\n  The editor cannot write a PES 2015 or 2016 Texport, so the button stays\n  disabled there.\n\nNeither will write over the save that is open.\n\n**Importing a team file**\n\nBelow the panel's top row, **Pick…** opens a team file to import into the selected\nteam: a Team TOML (`.toml`), a squad file (`.4ccs`), a tactics file (`.4cct`) or a\nTexport. PES 2015 to 2017 name their Texports `TEXPORT00000000`, `TEXPORT00000001` and\nso on, with no extension, so on a save from those games the file window opens showing\nall files. A squad file lists which record goes to which player.\n\nWhen the picked file carries the ID of the team it was exported from and another team is\nselected, a button under **Import into** selects that team, so you can import back onto it\nwithout looking for it in the list.\n\nTick the sections to bring in:\n\n- **Stats** (abilities, skills, height and the like, and the team's name, short\n  name, manager and stadium);\n- **Aesthetics** (names, shirt numbers, nationality and age, the look and every\n  motion, and the team's colors and kits);\n- **Gameplan** (the lineup, the set-piece takers and captain, the players who\n  join the attack, the automatic settings' substitution, offside trap, preset\n  change and attack/defence levels, and the three presets).\n\nOnly the sections the file holds are offered, and Aesthetics stays off while the\naesthetics lock is on.\n\nA file from another PES version is converted, and the values that could not\ncarry over are listed as notes, in groups that open to show their lines. If a\nticked section holds a value the save cannot take, the reason stands under the\nchoices and Apply is disabled; untick that section to import the rest.\n\n**Apply** writes the ticked sections as one undoable step, unsaved until you\nsave. When the team already holds every value the ticked sections would write\n(its own file, or a file applied already), Apply stays disabled and the panel\nsays so.\n\n**Saving a picture of the formations**\n\n**Save a picture of the formations…** writes a PNG of the team's formations, a\nrow per preset and a column per formation, for checking a team's shapes or\nsharing them. Its dialog asks for:\n\n- the **Folder**, `formations` in UTW.exe's folder until you save a picture\n  elsewhere, then the last one used; **Browse…** picks another;\n- the **File name**, the team's name (`f.png` for /f/).\n\nThree checkboxes under them shape the picture:\n\n- **With each preset's settings beside its row** lists each preset's Formation\n  settings in a column beside its row, with its Advanced instructions on PES\n  2017 to 2021, and adds a line under the formations with the team's Automatic\n  settings.\n- **Portraits** draws the players' portraits. It starts matched to\n  **Portraits** on the formation bar, and changing it here does not change the\n  bar.\n- **Without the pitch's grain (smaller file)** paints the pitch plain.\n\nIf the file is already in the folder, **Save** asks before replacing it. The\nnotice that says the picture was written has an **Open folder** button.\n",
          "slug": "team",
          "title": "The team view and the Team menu"
        },
        {
          "body": "The **Player** menu works on the selected player:\n\n- **Set all stats…** gives every ability stat the same value, 40 to 99, and the\n  dialog's value starts at his Attacking Prowess (weak foot, form and injury are\n  not ability stats and stay as they are).\n- **Bump all stats…** (Ctrl+B) adds the same amount, -10 to +10, to every\n  ability stat, each kept within 40 to 99, and the dialog remembers the last\n  amount used in this window.\n- **Hide default body (Enable FPC)** or **Show default body (Disable FPC)**\n  (Ctrl+I): the entry's label names what the press will do, following the\n  Appearance screen's FPC reading. It shows the default body of a player\n  who reads \"The default body is currently hidden\" and hides any other's,\n  the same write as the screen's Show default body and Hide default body.\n- **Copy/swap stats…** opens a window that copies one player's stats onto another,\n  or trades them, and stays open for the next player.\n\nEvery operation is one undoable step. Set all stats and Bump all stats write the\nsame fields the player view's abilities and skills part edits, so they set no\nedit flag. Copy and Swap stats move the gameplay edit flags with the stats.\n\n**Hide and Show default body**\n\nA player hidden by his model's own boots ID reads \"The default body is currently\nshown\", so for him the entry hides him again; to show his default body, use\n**Show default body** on the Appearance screen instead. The entry stays\ndisabled while the aesthetics lock is on.\n\n**Copy and Swap stats**\n\nThe window has two rows, Source and Destination, each picked by team and then\nby player. Both start on the selected player, or on the team's first player\nwhen a team row is selected. While the window stays open, clicking a player row\nin the list sets the Destination to that player (or to a clicked team row's\nfirst player), so you can copy one player's stats onto several others with a\nclick on each and **Copy**. **Swap** trades the two players' stats instead.\n\nHeight, weight, every\nability, form, injury, weak foot, positions, playing style, skills and COM\nstyles, stronger foot and hand, and the gameplay edit\nflags move with the stats. Names, nationality, age, every motion, the star\nrating and playing attitude, and every appearance field stay as they are.\n\n**Generating a player folder**\n\nThe menu's **Generate settings.toml…** and **Generate in-game face…** write the selected\nplayer's look to a player folder of your choice, the files a team moving to\ncompile-time aesthetics is built from. `settings.toml` holds everything but the\nin-game face: the names, the stock boots and gloves IDs, skin color, physique,\nstrip, motions, and the star rating and playing attitude. `ingame_face.toml` holds\nthe in-game face itself: iris color, face and hair.\n\nThe save cannot say which players wear the in-game face, so you choose per\nplayer whether to generate it. Neither item writes the save.\n\nWhen the folder already holds a `settings.toml`, its comments stay while the\nvalues become the player's. `ingame_face.toml` is written whole.\n",
          "slug": "player",
          "title": "The Player menu"
        },
        {
          "body": "The **Savefile** menu works on the whole save:\n\n- **Comparator…** opens a window comparing the open save against a second save\n  (see **The window**).\n- **Apply aesthetics patch…** opens a panel for a patch the Compiler wrote.\n- **Output rosters to TSV…** writes the roster sheet, the file members paste\n  into a spreadsheet to check a cup's teams.\n- **Save formation pictures for a range of teams…** writes a picture of the\n  formations for each team of a range.\n- **Multi-team AATF…** checks a range of teams under the AATF ruleset picked on\n  the toolbar, for an organizer checking a cup's teams.\n- **Fix database…** is a maintenance repair behind a confirmation.\n\n**Applying an aesthetics patch**\n\nThe panel's **Open patch…** reads the patch; a patch made for another PES\nversion, or by a Compiler that numbers custom boots and gloves differently\nfrom the one in this program, is refused and the save is left alone. The panel\nthen lists every value the patch would change, player by player, with\nthe old value and the new one; click a row to select that player.\n\nA team the save does not hold is named and skipped, and the rest applies.\n**Apply** writes the patch as one undoable step, unsaved until you save. A patch\nwhose values the save already holds lists nothing, says so, and leaves Apply\ndisabled. It works while the aesthetics lock is on: a patch is how the teams'\nexports reach the save.\n\n**The roster sheet**\n\nThe sheet holds every cup team of the save: its name on a line of its own,\nthen one tab-separated row per player (shirt number, name, position,\nFinishing, a `(C)` on the captain, height, playing style, skills and COM\nstyles). A save dialog asks where to write it, with `EDIT.tsv` as the default\nname. Writing it changes nothing in the save.\n\n**Multi-team AATF**\n\nPick the first and the last team of the range (both start on the save's first cup\nteam). The last team's list starts at the first team, and moving the first past the last\nbrings the last along, so the range is never backwards. Every cup team whose id lies\nbetween them is checked and listed under its id and name, with its problems worded as\nLive AATF words them, or `All clean` when it has none. The check starts once both ends\nhave stayed put for half a second, so the mouse wheel can run through the teams without\nchecking each one on the way; until then the previous list stays, dimmed and marked\n`checking…`. Suggestions are left out, since an unused allowance is the manager's\nbusiness.\n\n**Copy to clipboard** copies the list as plain text for a cup's thread, each\nteam's line after a `>` so the thread shows it in green. The dialog only reads\nthe save.\n\n**Formation pictures for a range of teams**\n\n**Save formation pictures for a range of teams…** writes the Team menu's picture\nof the formations for every cup team between the two you pick, picked as for\nMulti-team AATF. The folder and the three boxes work as in the Team menu's\ndialog (see **Saving a picture of the formations** in **The team view and the\nTeam menu**). Each picture is named after its team, and one of that name already\nin the folder is replaced without asking. **Save pictures** writes them one team\nafter another, and **Stop** ends the run early, keeping the pictures already\nwritten. When the run ends, **Open folder** opens their folder.\n\n**Fix database**\n\n**Fix database…** does the following, as one undoable step:\n\n- on PES 2015 to 2018, it repairs base copies, clearing a stray copy id and a copy\n  flag that points at nothing (later games' records have no base-copy flag, so\n  their copy ids are kept);\n- it clears the face, hair, physique and strip edit flags on every cup player;\n- on PES 2017, it also clears every team's strip flag and resets the kit slots to\n  their defaults.\n\nIt repairs the cup's teams and their players alone, and leaves the game's own\nteams as they are.\n\nIt stays disabled while the aesthetics lock is on (see the Appearance screen),\nsince it clears what the teams' exports own.\n",
          "slug": "savefile",
          "title": "The Savefile menu"
        },
        {
          "body": "The bottom of the team view is the **Tactics** part, the team's game plan: which\nof the three presets is on screen, its formation settings, the advanced\ninstructions and the team's automatic settings. A field the save's version does\nnot hold stays in its place, disabled, with a note naming the first version that\nhas it.\n\n**The layout**\n\nThe part is a grid of three columns and three rows:\n\n- the preset bar level with the formation bar at the top;\n- **Formation settings**, the pitch and **Player assignments** in the middle;\n- **Advanced instructions** and **Automatic settings** beside the two player\n  lists at the bottom.\n\n**The pitch**\n\nThe **pitch** is where the formation takes shape, drawn like the game's own\nformation screen: each starter stands at his spot, shown as his portrait over a\nbadge with his position and his name below it. A name too long for the badge\nscrolls now and then, as in the game. Turning off **Scroll**, at the right of\nthe formation bar, holds the names still, and turning off **Portraits** beside\nit hides the portraits so the formation's shape is easier to see. Both settings\nare remembered.\n\nThe badge's right end shows his overall at the position he plays there, as the\ngame's does, and a gold, silver or bronze outline round the badge names his\nmedal tier under the active ruleset, where he has one.\n\nThe portraits come from the save's own game, out of the `download` folder\ninside the **PES folder** set for that version. A portrait that cannot be read\nfrom there shows as the game's blank one. When that folder is missing, or does\nnot hold that version's game, a **Set PES path** button stands on the pitch's\nbottom right for picking the right folder.\n\nThe formation's name in the pitch's bottom-left corner (such as 3-4-2-1) counts\nthe players in each line by their positions, back to front, as the game's\nformation screen does, so it changes when a player's position does but not when\nhe only moves.\n\n**Dragging a player**\n\nDragging a pitch player to a new place moves him there, snapped to the positions\nthe save can hold, and, unless the cup allows illegal formations (below), gives\nhim the position the game's own editor would there (which can depend on his\nratings: the spot offers up to two positions and he takes the one he rates\nhigher at). While you hold him, he moves one step at a time and the point you\ngrabbed stays under the pointer.\n\nThe pitch stays lit only where he is rated A or B, and his badge shows the\nposition he would take if you let go there, so it changes as he crosses from one\narea into another (unless the cup allows illegal formations, below). Nothing\nchanges until you let go: bring him back to where he started and he keeps his\nposition.\n\nUnless the cup allows illegal formations (below), dropping him on another\nstarter (the goalkeeper excepted) trades their places and positions in the\nformation on screen alone, as the game's own drag does: the preset's other two\nformations and the other presets keep theirs. While you hold him over the other\nstarter, that starter's marker gets corner brackets and the dragged one turns half\ntransparent, so a drag meant only to move him can still be steered aside onto\nthe grass.\n\nA bench player dropped on a pitch player swaps them, and a pitch player dropped\non a bench row does the same. The goalkeeper cannot be dragged and his position\nand spot stay fixed.\n\n**When the cup allows illegal formations**\n\nSome cups play a patched game that keeps any formation, and their ruleset says\nso (the Ruleset editor's **Illegal formations allowed (Whack Patch)** box; 4cc\nAutumn 26 and Divegrass Before Christmas 7 among the built-in rulesets). While\nsuch a ruleset is active, the pitch sets a player's spot and his position apart,\nand the rules above change:\n\n- a dragged player keeps his own position wherever you let go, and his badge\n  shows it all the while you hold him;\n- he can stand on any row, the goalkeeper's area included;\n- dropped on another starter he lands there, beside or on him, and nothing is\n  traded, since the game keeps two players on one spot; nothing is marked or\n  faded while you hold him there;\n- his details' position pick offers every outfield position, and moving him with\n  X and Y keeps his position.\n\nThe goalkeeper's spot, the lit areas and the swaps through the lists below\nwork as under any other ruleset.\n\n**A player's details**\n\nClicking a pitch player opens his details beside his marker (when the cup\nallows illegal formations, the first two work as described above):\n\n- a position pick (it offers only the positions his spot allows, as the game's\n  Select Position menu does);\n- the exact X and Y fields (moving him with them changes his position as a drag\n  there would);\n- **Make GK**, which puts him in goal and brings the goalkeeper out onto his\n  spot.\n\nThe details stay where they opened while X and Y move the player, so the fields'\narrows stay under the pointer, and a click anywhere outside closes them.\n\n**The three formations**\n\nThe **Kick-off**, **In-possession** and **Out-of-possession** buttons on the\nformation bar pick which of the preset's three formations the pitch shows. The game\nuses the second and third only while the preset's **Fluid formation** is on, so\nwhile it is off (and always on PES 2015) the buttons stay on Kick-off and cannot\nbe clicked. The formation bar's Copy/swap still works, so you can prepare the\nother two before switching it on. Switching formations changes nothing by\nitself.\n\n**Starters and bench**\n\nThe two lists under the pitch set who plays and in which order: the starting\neleven on the left, the bench on the right; the bench lists the players alone,\nnever an empty slot.\n\nDrag a row onto another to move or swap it:\n\n- a starter's row dropped on another starter's row, or on a pitch player, swaps\n  the two players everywhere, in every formation of every preset, as picking one\n  and then the other does in the game;\n- a pitch player dropped on a starter's row does the same;\n- a bench player dropped on a starter takes his place and every job he held (set\n  pieces, joining the attack, instruction targets).\n\nThe captain stays who he was.\n\n**Player assignments**\n\nThe **Player assignments** rows set who takes each set piece, the captain and\nwho joins the attack: each is a label over a chip naming its holder, or\n**Unassigned** where nobody holds it. **Join the attack** has three chips, one\nper joining player.\n\nClick a chip, then click the pitch player who takes the job. **Join the attack**\nstays open after a click, so up to three players can be picked or unpicked, and\nall three of its chips stay highlighted until you click a chip again to close it.\n\n**Presets**\n\nThe **Preset** buttons on the left of the preset bar pick which of the team's\nthree presets the panels below edit. Switching presets changes nothing by\nitself. The **Fluid formation** checkbox beside them (PES 2016 and later)\ndecides whether the game uses the second and third formations.\n\n**Formation settings**\n\n**Formation settings** pairs each attacking setting with its defensive\ncounterpart in six rows:\n\n- attacking styles beside defensive styles;\n- build up beside containment area;\n- attacking area beside pressuring;\n- positioning beside defensive line;\n- support range beside compactness;\n- numbers in attack beside numbers in defence.\n\nEach is a label over its control:\n\n- a slider with its value at the right;\n- three choices (Few, Medium, Many) for the numbers;\n- a two-choice switch named as the game names its two choices.\n\nThe defensive style's choices are shortened so the row fits; point at one to\nread its full name.\n\nFrom PES 2017 on, the game's own Game Plan no longer shows numbers in attack and\nnumbers in defence, and the game is said to use them only while the team's automatic\nattack and defence levels are on. On a PES 2017 to 2021 save they are therefore named\n**Auto attack level** and **Auto defence level**, and their choices are disabled, the\nstored one on a grey rectangle, unless both of these hold:\n\n- the cup allows illegal formations (see **When the cup allows illegal formations**\n  above);\n- **Attack/defence levels** is ticked under **Automatic settings**.\n\nPoint at a disabled choice to read why.\n\n**Advanced instructions**\n\n**Advanced instructions** (PES 2017 and later) sets the team's two attack and\ntwo defence instructions: the attack column lists the attacking choices and the\ndefence column the defending ones. When the cup allows illegal formations (see\nabove), its patched game also puts any instruction in any slot, so both columns\nlist every instruction. Tight Marking is never offered, because it\nmarks an opponent and is set during a match, not here. A save already holding it\nkeeps it, and the row still names it.\n\nUnder each instruction a chip names its target player, or reads \"Unassigned\".\nThe chip is enabled only for the instructions that take one of the team's own\nplayers (Counter Target and Defensive): click it, then click the pitch player,\nas the player assignments pick theirs. Under an instruction that takes no target\nthe chip stays disabled and reads \"No target\", or \"An opponent\" under a held\nTight Marking, whose target is one of theirs. Picking an instruction that takes\nno target clears the pick.\n\n**Automatic settings**\n\n**Automatic settings** holds:\n\n- the automatic substitution setting, a choice of the game's own names (Off,\n  Very late, Flexible or Very early, the same four on every version);\n- the offside trap;\n- the preset change;\n- the attack/defence levels (PES 2017 and later).\n\n**Copy and swap**\n\nEach bar's **Copy/swap** menu acts on the shown preset or formation: a copy to,\nthen a swap with, each of the other two (Copy to 2, Copy to 3, Swap with 2 and\nSwap with 3 from preset 1; Copy to In-possession and so on for a formation). A\npreset copy or swap moves the whole preset, man marking included, and a\nformation copy or swap moves its eleven slots; the starting eleven is never\ntouched. Every change, like every edit in the editor, is one undoable step\n(Ctrl+Z).\n",
          "slug": "tactics",
          "title": "The Tactics editor"
        }
      ]
    },
    {
      "id": "ruleset-editor",
      "messages": [
        {
          "code": "rules_finding",
          "effect": "The file is unchanged; the exit code is 1. Fix the file in a text editor and run it again.",
          "severity": "Error",
          "when": "`validate`: the rules file could not be loaded; the line names where it failed (a line and column when the text is broken, the entry that broke a rule) or, failing that, what went wrong."
        },
        {
          "code": "logic_updated",
          "effect": "The checking code is replaced with this build's; the old file is kept beside it as a .bak copy.",
          "severity": "Info",
          "when": "`update-logic`: the file's checking code was an older official version than this build's (the line gives it as `from`)."
        },
        {
          "code": "logic_already_current",
          "effect": "The file is left as it was; nothing is written.",
          "severity": "Info",
          "when": "`update-logic`: the file already carries this build's checking code."
        },
        {
          "code": "summary_written",
          "effect": "The file it replaced, if there was one, is kept beside it as a .bak copy.",
          "severity": "Info",
          "when": "`summary -o`: the file's readable rules text was written to the file the line names."
        }
      ],
      "title": "Ruleset editor",
      "topics": [
        {
          "body": "The Ruleset editor's commands run from a terminal opened in the folder that holds\n`UTW.exe`. A rules file (a `.4ccrules` file) is what a cup's teams are checked\nagainst: tiers, stat targets, heights, cards and special rules, plus the code that runs\nthe check.\n\n```text\nUTW ruleset-editor validate <file>\nUTW ruleset-editor update-logic <file>\nUTW ruleset-editor summary <file> [-o <out>]\n```\n\n`validate` checks that a rules file loads: the text parses, the rules hold together (no\nempty names or doubled ids, every id named exists, the tier counts fit the squad, the\nheight brackets ascend), and the checking code runs on a sample team. A file that passes\nprints nothing and exits 0. A file that fails prints one `rules_finding` line per\nproblem: where it failed when the text is broken (a line and column), or which entry\nbroke a rule (a path like `tiers[2].free.tricks`). The exit code is then 1. A file that\ncannot be read at all is refused with exit code 2. Nothing is written either way.\n\n`update-logic` refreshes a file's checking code when it is an older official version.\nThe file keeps working as it is until you run this, and when you do, the command copies the\nfile to `<file>.bak` and then writes this build's code in. A file already carrying this\nbuild's code is left alone (`logic_already_current`, exit 0). A file whose checking code\nwas edited by hand is refused: replacing it would discard someone's code. A file that\ndoes not load is refused as well; run `validate` on it to see why.\n\n`summary` prints the ruleset as readable rules text, the tiers, heights, card groups and\nspecial rules spelled out, ready to post as an invitational's rules. With `-o` the text is\nwritten to a file instead of printed, and the file it replaces is kept as `<out>.bak`. A\nfile that does not load is refused with exit code 2 before anything is written. To compare\ntwo rules files, run `summary` on each and diff the outputs.\n\nEach command ends with an exit code scripts can test: 0 when it did its work, 1 when it\ndid its work but reported an error, 2 when it was refused before writing anything, 3\nwhen a write failed.\n",
          "slug": "command-line",
          "title": "Running from a terminal"
        },
        {
          "body": "The Ruleset editor's window builds an AATF ruleset through forms and saves it as a\n`.4ccrules` rules file the Save editor and the Team creator can check against, like the\nofficial one.\n\n**Start page**\n\nWith no ruleset open, the window offers to start one:\n\n- **New from template**: the official rules and the VGL, VTL and DBC cup rulesets, then\n  **Blank: one tier, no height brackets, universal rules on**;\n- **Open file…**, to pick an existing rules file;\n- the recently used files, newest first.\n\nWhen a file cannot be opened, what is wrong with it is listed at the bottom of the page.\n\n**Editing**\n\nThe sections sit in a rail on the left. **Custom checks** appears there only when the file\ncarries hand-written checks, and a \"⚠\" marks a section holding a problem (the bar along the\nbottom names it). **Show preview** at the top right opens the rules file's data block, as\nthe editor writes it, with the lines the last edit changed highlighted.\n\n**What each section lets you do**\n\nThe window doesn't edit every section yet, which is why the tool's name ends in \"(WIP)\",\nwork in progress:\n\n- **General** is a form: the ruleset's name, its event's page and logo, and the stat\n  checklists. The logo can be any picture: a big one is made smaller to fit. It shows\n  at the Save editor's top right while the ruleset is active, and a click on it opens\n  the event's page. The rating stats pick the player's tier, and the \"may sit below\n  the target\" stats are the only ones allowed below their target.\n- **Tiers** is a form: the tiers table, highest first, each tier's per-stat targets\n  and free cards under its row, and the medal systems. A tier's conditions can't be\n  edited here yet.\n- **Universal rules** is a form: the rules every team plays under (ratings at the\n  registered position, B ratings, age, weight, the captain, goalkeepers, the skill-card\n  and COM-style caps) and whether the cup allows illegal formations. Its team settings\n  can't be edited here yet: **Summary** shows them, and saving leaves them unchanged.\n- **Custom checks** shows the file's hand-written checks as text, and saving keeps\n  them as they are.\n- **Summary** spells the ruleset out as readable rules text, with **Copy** putting it\n  on the clipboard to post.\n- **Heights**, **Cards**, **Specials** and **Suggestions** can't be edited here yet:\n  **Summary** shows what's in them, and saving leaves them unchanged.\n\n**Opening another ruleset**\n\nThe **File** menu's **New from template…** brings the start page back over the open\nruleset, and its **Open…** opens another rules file. When that file cannot be opened, a\nwindow lists what is wrong with it.\n\n**Undo and redo**\n\nThe **Edit** menu's **Undo** (Ctrl+Z) and **Redo** (Ctrl+Y or Ctrl+Shift+Z) step through\nthe edits.\n\n**Checking code changed by hand**\n\nA file whose checking code was changed by hand, or is an older official version, shows a\nbanner under the toolbar. Code changed by hand is kept as it is when saving, and an older\nofficial version offers **Update logic**.\n\n**Saving**\n\nThe **File** menu's **Save** (Ctrl+S), or **Save** on the bottom bar, writes the rules\nfile, and **Save as…** lets you choose where. An invalid ruleset is never written: the\neditor lists what blocks the write instead. With unsaved changes, closing the window,\nopening another file or picking a template asks whether to save them first.\n",
          "slug": "window",
          "title": "The window"
        }
      ]
    },
    {
      "id": "export-upgrader",
      "messages": [
        {
          "code": "no_old_export",
          "effect": "Nothing is written.",
          "severity": "Error",
          "when": "The input holds nothing an old export holds: no Team Note, none of the old format's folders, no `refs.txt` or `refs list.txt`."
        },
        {
          "code": "refs_txt_missing",
          "effect": "Nothing is written.",
          "severity": "Error",
          "when": "A `refs` export has neither `refs.txt` nor `refs list.txt`, so it holds no roster to migrate."
        },
        {
          "code": "export_extract_failed",
          "effect": "Nothing is written. Repack the archive, or unpack it into a folder first.",
          "severity": "Error",
          "when": "The archive (`.zip` or `.7z`) cannot be opened, or a path inside it is unusable: it leads outside the archive, or two names differ only in letter case or Unicode form."
        },
        {
          "code": "save_team_name_repeated",
          "effect": "The first team of that name in the save's order is used; the later one is left out.",
          "severity": "Warning",
          "when": "The save holds the export's team name on a later team too (the line names both teams' IDs)."
        },
        {
          "code": "unmigrated_content",
          "effect": "It is not migrated, and the run is not clean, so nothing is written. Fix it in the input and upgrade again.",
          "severity": "Error",
          "when": "Content of the input that reached the game, or was clearly meant for it, but that the migration does not carry into the output. Examples: two files or folders claiming one place (a second face folder of one slot, two portraits of one), a whole folder under a name the old format refuses, a Team Note entry whose colors do not read, a kit config that does not decode. The line names the cause, and, where another file or folder took the place, that one's path. \"Fixing unmigrated content\" in this tool's help lists the causes."
        },
        {
          "code": "migration_clean",
          "effect": "The upgraded export is written. The exit code is 0.",
          "severity": "Info",
          "when": "The report's last line, when there was no Error finding and no finding dropped anything but a leftover."
        },
        {
          "code": "migration_not_clean",
          "effect": "Nothing is written. The exit code is 1 after an Error finding, or after a finding that dropped more than a leftover. Fix what the findings name in the old export and upgrade again. If a finding names something the old export does not hold, post the report on the /aesco/ server or the divegrass thread.",
          "severity": "Info",
          "when": "The report's last line, otherwise."
        },
        {
          "code": "leftover_dropped",
          "effect": "It is not written, but the run stays clean.",
          "severity": "Warning",
          "when": "Content that never reached a player in game, wherever it sits (a refused portrait, a stray file, a second goalkeeper kit, a file directly in `Faces/`, `Boots/` or `Gloves/`). For a dropped part of a Team Note entry, the line names the entry's line in the Note."
        },
        {
          "code": "mtl_stray_text_dropped",
          "effect": "The text is dropped as the file is written back.",
          "severity": "Info",
          "when": "An old `.mtl` held text between its elements, which the compiler cannot read."
        },
        {
          "code": "roster_slot_empty",
          "effect": "It still becomes its player folder, with no `settings.toml`.",
          "severity": "Warning",
          "when": "A face folder's slot is empty in the save's roster (the line names the slot)."
        },
        {
          "code": "merge_kept_shared",
          "effect": "The boots or gloves folder is kept shared with a link instead.",
          "severity": "Info",
          "when": "A player folder and a boots or gloves folder that would merge into it each hold a file at the same path, with different bytes."
        },
        {
          "code": "wearer_assumed",
          "effect": "The pairing is reported so it can be checked. If it is wrong, rename the folder, or upgrade with `--savefile`.",
          "severity": "Info",
          "when": "`--loose` paired a boots or gloves folder with the player at its ID's position in the team's old block of 25 (the line names the folder's four-digit ID and the slot)."
        },
        {
          "code": "wearer_by_name",
          "effect": "The folder becomes his as if it were numbered with the save's ID, so his model is not lost. Check that this is the player the folder was meant for.",
          "severity": "Warning",
          "when": "A boots or gloves folder carries an ID no save record wears, but its name part is a rostered player's name, and in the save he wears a custom ID (above 100) that no folder of that kind carries (the line names the folder, the slot and the save's ID)."
        },
        {
          "code": "wearer_by_number",
          "effect": "The folder becomes the player's as if it were numbered with the save's ID, so his model is not lost. Check that this is the player the folder was meant for.",
          "severity": "Warning",
          "when": "A boots or gloves folder nobody wears has a same-numbered folder of the other kind, which was paired with a save ID by name, number or order. A player also wears that ID for this kind, with no folder carrying it, so the folder takes it too (the line names the folder, the slot, the save's ID and `from`, the folder it followed)."
        },
        {
          "code": "wearer_by_order",
          "effect": "The folder becomes the player's as if it were numbered with the save's ID, so his model is not lost. Check that this is the player the folder was meant for.",
          "severity": "Warning",
          "when": "The boots (or the gloves) had as many unworn folders as worn custom IDs no folder carried, so they were paired in order of ID and folder number (the line names the folder, the slot and the save's ID)."
        },
        {
          "code": "texture_duplicate_dropped",
          "effect": "The `.ftex` is dropped and the `.dds` kept, and the run stays clean. Check the kept one is the texture meant.",
          "severity": "Warning",
          "when": "A folder of the upgraded export held one texture stem twice, a `.dds` beside a `.ftex`, which the compiler would refuse (the line names the dropped file and `kept`, the `.dds`'s path)."
        },
        {
          "code": "kit_without_config",
          "effect": "The kit still forms. In a `Full` export it is a placeholder the compiler fills, and in a `Midcup` one it updates only the installed kit's colors.",
          "severity": "Warning",
          "when": "A Team Note kit entry gives a slot no `Kit Configs` config does."
        },
        {
          "code": "config_without_kit_entry",
          "effect": "Its `config.toml` is written all the same, so the kit is offered as the old export never offered it. Delete the kit's folder in the upgraded export if the kit should not be offered.",
          "severity": "Warning",
          "when": "A `Kit Configs` config gives a slot no Team Note kit entry does."
        },
        {
          "code": "kit_texture_copied",
          "effect": "The named texture was copied into the kit under the stem the line names.",
          "severity": "Info",
          "when": "A kit's config names another texture for a number or name (the line names the kit, the stem and where it came from)."
        },
        {
          "code": "kit_texture_hoisted",
          "effect": "It was moved to `Kits/all/`, and the kits compile the same without the copies.",
          "severity": "Info",
          "when": "A texture every kit folder held with identical bytes."
        },
        {
          "code": "portrait_conflict",
          "effect": "The run is not clean, so nothing is written. Keep one portrait in the old export and upgrade again.",
          "severity": "Error",
          "when": "A `Portraits/` file and its slot's face-folder portrait differ in bytes, which the compiler would refuse."
        },
        {
          "code": "logo_small_carried",
          "effect": "It is kept as `logo_small.png`.",
          "severity": "Info",
          "when": "The old `_r` small logo is a different drawing (the line gives how much it differs)."
        },
        {
          "code": "logo_small_dropped",
          "effect": "It is dropped, because the compiler regenerates it. Carry it by hand as `logo_small.png` if it was a variant.",
          "severity": "Info",
          "when": "The old `_r` small logo is a downscale of the large one."
        },
        {
          "code": "logo_fallback",
          "effect": "The largest usable image became `logo.png`, and no `logo_small.png` is written.",
          "severity": "Warning",
          "when": "The `Logo/` folder holds no usable `_r_ll` file."
        },
        {
          "code": "common_reference_kept",
          "effect": "It stays a path into the team's Common, in the new spelling `team_common_folder/`. An installed CPK may still supply the file.",
          "severity": "Warning",
          "when": "An `.mtl` or `.fmdl` reference into the team's `Common/` that names a file the input's `Common/` does not hold, or that a `.common` link cannot stand for (the model's folder already holds a texture or link of that stem, or a `.mtl` in a subfolder names a Common subfolder)."
        },
        {
          "code": "model_far_dropped",
          "effect": "It is not written (the compiler would refuse it), but the run stays clean. A boots or gloves folder left with no model hides the boots or hands.",
          "severity": "Warning",
          "when": "A model whose every vertex sits more than 5000 units from the origin, which is how old exports hid a template's default face, hair or glove, by moving the whole model out of view. The line names the file and its vertex count."
        },
        {
          "code": "antiblur_dropped",
          "effect": "The model is written without them, and its meshes no longer ask for them, so nothing adds them back. Nothing to do.",
          "severity": "Info",
          "when": "An `.fmdl` carrying anti-blur copies (duplicate meshes older models added to stay sharp under the game's blur, the line naming how many)."
        },
        {
          "code": "custom_model_missing",
          "effect": "The run is not clean, so nothing is written. Add the folder to the old export, or take the ID off the player in the save.",
          "severity": "Error",
          "when": "In the save, a rostered player of a `Full` output wears a custom boots or gloves ID (above 100) that no migrated folder claims, and the output gives him no folder."
        },
        {
          "code": "custom_model_not_worn",
          "effect": "His `settings.toml` gets the hide ID of the kind (55 for boots, 11 for gloves), which the game also shows as no model. Nothing to do.",
          "severity": "Info",
          "when": "In the save, a rostered player wears a custom boots or gloves ID that is the number of a folder of the other kind, the save's way of wearing no custom item of the kind."
        },
        {
          "code": "custom_model_unpaired",
          "effect": "The run is not clean, so nothing is written. Rename the folder meant for him to his ID, or take the ID off the player in the save.",
          "severity": "Error",
          "when": "In the save, a rostered player wears a custom boots or gloves ID (above 100) that no boots or gloves folder carries and no rule could pair, while an unworn folder of the kind is left."
        },
        {
          "code": "custom_model_hidden",
          "effect": "His `settings.toml` gets the hide ID of its kind (55 for boots, 11 for gloves), which the game shows as no model, the way a face folder carrying his whole body wants it. If a boots or gloves folder was meant, add it to the old export and upgrade again.",
          "severity": "Warning",
          "when": "In the save, a rostered player the output gives a folder wears a custom boots or gloves ID (above 100) that no migrated folder claims."
        },
        {
          "code": "settings_value_refused",
          "effect": "It is left out of the file the key belongs to.",
          "severity": "Warning",
          "when": "A stored value that no key of the player's `settings.toml` or `ingame_face.toml` can hold (the line names the slot, the player, the key and the value)."
        },
        {
          "code": "settings_not_generated",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A player's `settings.toml` could not be generated at all (the line names the error)."
        },
        {
          "code": "coverage_tag_chosen",
          "effect": "Check the tag is what the export should be. If it is not, upgrade again with `--coverage`, or, when the tag came from the name (`name`), fix the name's second word first.",
          "severity": "Info",
          "when": "The coverage tag the output's name took, and what decided it (the line names them): `flag` for `--coverage`, `name` for a tag word the name already carried, `midcup` or `additions` for text the name holds, `default` when it holds neither."
        },
        {
          "code": "staging_folder_removed",
          "effect": "It was removed before this run wrote its output.",
          "severity": "Info",
          "when": "An earlier run's incomplete output folder, the output's name with `.upgrading` appended, was still there when this run started (the line names it)."
        },
        {
          "code": "nested_folders_fixed",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Warning",
          "when": "The export's content sits one level too deep, inside a single folder."
        },
        {
          "code": "nested_root_ambiguous",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Several folders inside the export each look like its root."
        },
        {
          "code": "nested_root_conflict",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A file at the export's root and one inside the nested folder resolve to the same place."
        },
        {
          "code": "players_txt_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "`players.txt` cannot be read as a roster (an encoding failure or a roster-wide ambiguity), or a referee roster is left with no valid assignment."
        },
        {
          "code": "refs_txt_ignored",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Warning",
          "when": "A referee export holds both `players.txt` and the old `refs.txt` name."
        },
        {
          "code": "source_read_failed",
          "effect": "It is not migrated, and the run is not clean, so nothing is written. An unreadable Team Note (it decides the team) or a referee export's `refs.txt` or `refs list.txt` (it decides the roster) stops the run at once.",
          "severity": "Error",
          "when": "A file of the input could not be read after its listing did."
        },
        {
          "code": "export_empty",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "No usable content was found at the export's root."
        },
        {
          "code": "team_name_unknown",
          "effect": "Nothing is written. Fix the `Team:` line or rename the export so its first word is the team's name.",
          "severity": "Error",
          "when": "The Team Note's `Team:` line, or the export's name when there is no such line, is no team name: it is empty, or holds a space or an inner `/` (the line names the value)."
        },
        {
          "code": "export_tag_missing",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A team export's name has neither `Full` nor `Midcup` as its second word."
        },
        {
          "code": "export_layout_old",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "The export is in the old layout, which the compiler does not read: a root folder named `Kit Configs`, `Kit Textures` or `Other` (kits live in `Kits/`)."
        },
        {
          "code": "players_txt_missing",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A referee export has no `players.txt` (nor the old `refs.txt` name)."
        },
        {
          "code": "players_txt_line_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A nonblank line of `players.txt` does not give a slot number and a folder name."
        },
        {
          "code": "players_txt_slot_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `players.txt` line's slot is outside 01-23 (teams) or 01-35 (refs)."
        },
        {
          "code": "players_txt_slot_duplicate",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "`players.txt` assigns the same slot more than once, so the roster is ambiguous."
        },
        {
          "code": "players_txt_target_missing",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `players.txt` line names a folder that does not exist."
        },
        {
          "code": "player_unlisted",
          "effect": "Found in the upgraded export. The compiler would leave it out, so the run is not clean and nothing is written.",
          "severity": "Warning",
          "when": "`players.txt` is present but does not list this player folder, whatever number its name carries."
        },
        {
          "code": "player_folder_number_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Without `players.txt`, a player folder's number is not in 01-23."
        },
        {
          "code": "player_number_duplicate",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Without `players.txt`, two player folder names claim the same slot."
        },
        {
          "code": "file_type_disallowed",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A file sits where its kind is not allowed, anywhere in a player or shared folder, in a kit folder, or in `Collars/`: its extension is disallowed, or, below a subfolder, it is a marker, a `settings.toml` or a per-player file."
        },
        {
          "code": "common_file_disallowed",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A file sits in `Common/` that is not model content (a model, `.mtl`, `.skl`, `.fclo`, `.xml`, `.bin`, texture or `.txt` note would be)."
        },
        {
          "code": "player_layout_proto",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A referee export's player folder has a `face/`, `boots/`, `gloves/` or `common/` subfolder directly inside, the layout of older referee exports (in a team export those are ordinary subfolders)."
        },
        {
          "code": "shared_link_duplicate",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A player folder has more than one shared link for a part (face, boots or gloves), counted over the whole folder, subfolders included. Two links naming one folder count once."
        },
        {
          "code": "link_target_missing",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A link file names a shared folder that does not exist."
        },
        {
          "code": "common_link_missing",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `.common` link names a file missing from `Common/` (a `kitN` name or folder stands for its set). In PES 2018 to 2021, a texture link is satisfied when a CPK listed before the compile's own holds it at the team's Common path."
        },
        {
          "code": "link_target_dropped",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A shared-folder or `.common` link names a target that was left out for its own finding (the line names the link, the target and the finding)."
        },
        {
          "code": "shared_folder_orphaned",
          "effect": "Found in the upgraded export. The compiler would leave it out, so the run is not clean and nothing is written.",
          "severity": "Warning",
          "when": "A shared `Faces/`, `Boots/` or `Gloves/` folder is linked by no player."
        },
        {
          "code": "fpc_conflict",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A player folder holds both `fpc_on` and `fpc_off`."
        },
        {
          "code": "ingame_face_marker_renamed",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A player folder holds `ingame_face` or `ingame_face.txt`, the in-game face marker's old name. The marker is now `ingame_face.toml`, which also holds the in-game face's keys."
        },
        {
          "code": "ingame_face_explicit_face_model",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A player folder holds `ingame_face.toml` beside an explicit face model (`face_high`, `hair_high` or `oral`, in any format), a `.common` link to one, or a `Faces/` link, and any of them needs the face folder the marker suppresses."
        },
        {
          "code": "texture_stem_conflict",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Two image files share one stem in the same place, whatever their extensions (`hair.dds` beside `hair.png`, or a texture link `hair.png.common`; `hair.dds` beside `jessie/hair.dds` is no conflict). The place is a model folder, a kit folder, `Kits/all/`, a folder of `Common/`, or `Portraits/` (two files of one slot)."
        },
        {
          "code": "fmdl_name_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "In PES 2018 to 2021, a model anywhere in a shared `Boots/` or `Gloves/` folder, or one a `.common` link there stands for, has a name that does not resolve to that category's allowed names."
        },
        {
          "code": "model_name_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a model anywhere in a shared `Boots/` or `Gloves/` folder lacks that category's suffix."
        },
        {
          "code": "edithair_unsupported",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "In PES 2015 to 2017, a `face_edithair.xml` or `hair.xml` sits anywhere in a player or shared folder."
        },
        {
          "code": "kit_folder_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `Kits/` subfolder's name is not `<slot>` or `<slot> - <label>` with the slot in `p1`-`p9`, `g1` or `all`."
        },
        {
          "code": "kit_slot_duplicate",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Two `Kits/` subfolders resolve to the same slot (`p1/` and `p1 - Lakers/`)."
        },
        {
          "code": "kit_texture_name_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A kit texture's name does not carry the `kit` prefix (`kit.dds`, `kit_mask.dds`)."
        },
        {
          "code": "kit_layout_conflict",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A kit folder holds both `pre-fox` and `fox` markers."
        },
        {
          "code": "kit_icon_invalid",
          "effect": "Found in the upgraded export. The compiler would leave it out, so the run is not clean and nothing is written.",
          "severity": "Warning",
          "when": "A kit's `icon_<N>` marker is numbered above 23, or the kit holds two or more (each is reported)."
        },
        {
          "code": "kit_all_file_ignored",
          "effect": "Found in the upgraded export. The compiler would leave it out, so the run is not clean and nothing is written.",
          "severity": "Warning",
          "when": "`Kits/all/` holds something other than kit textures (`config.toml`, `colors.txt`, an `icon_<N>` marker, anything else)."
        },
        {
          "code": "kit_all_unused",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Warning",
          "when": "`Kits/all/` is present but no kit folder inherits from it."
        },
        {
          "code": "kit_textures_inherited",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Info",
          "when": "A kit lacks texture names that `Kits/all/` provides (the line lists them)."
        },
        {
          "code": "portrait_name_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `Portraits/` file is not a texture named `NN` with NN in 01-23 (the `NN` alone, or followed by ` - ` and any text)."
        },
        {
          "code": "portrait_missing",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Warning",
          "when": "In PES 2017 to 2021, a player of an export upgraded as `Full` has no portrait."
        },
        {
          "code": "logo_missing",
          "effect": "The run is not clean, so nothing is written. Put the logo in the old export's `Logo/` folder as `emblem_0XXX_r_ll.png`, or upgrade it with `--coverage midcup` if it only adds to what is installed.",
          "severity": "Error",
          "when": "An export upgraded as `Full` has no logo: the old export has no `Logo/` folder, or nothing in it can be taken as the logo."
        },
        {
          "code": "logo_file_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A root `logo*` file has an unknown tag in its name, is in a format the compiler does not read, or does not decode."
        },
        {
          "code": "logo_role_duplicate",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "Two files claim the same logo role (two `logo*`, or two `logo_small*`)."
        },
        {
          "code": "logo_small_without_main",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "A `logo_small*` image is present with no main `logo*`."
        },
        {
          "code": "root_file_unexpected",
          "effect": "Found in the upgraded export. The compiler would leave it out, so the run is not clean and nothing is written.",
          "severity": "Warning",
          "when": "An unknown file or folder sits at the export's root (a stale `wrapper/` beside a usable root included), or a file sits directly inside `Players/`, `Kits/`, `Faces/`, `Boots/` or `Gloves/`, all of which hold only folders."
        },
        {
          "code": "notes_found",
          "effect": "Found in the upgraded export, which is still written.",
          "severity": "Info",
          "when": "The export's root holds a non-empty `notes.txt`."
        },
        {
          "code": "notes_encoding_invalid",
          "effect": "The run is not clean, so nothing is written.",
          "severity": "Error",
          "when": "The root `notes.txt` is not valid UTF-8."
        }
      ],
      "title": "Export upgrader",
      "topics": [
        {
          "body": "The Export upgrader migrates an export in the old (AET) layout to the new\nformat the Compiler reads. Its commands run from a terminal opened in the\nfolder that holds `UTW.exe`.\n\n```text\nUTW export-upgrader upgrade [<export>...] [--all <DIR>] [--savefile <EDIT> | --loose] [--coverage full|midcup] [-o <dir>]\n```\n\n`upgrade` takes one or more old exports, `--all <DIR>`, or both. Each export\nis a folder, a `.zip` or a `.7z`. `--all <DIR>` adds every export in the\nfolder DIR after the ones named: its folders and its `.zip` and `.7z` files,\nin the order of their names. Any other file in DIR is passed over, and so is a\n`<name>.upgrading` folder an interrupted run left there.\n\nA team export takes `--savefile`, naming the save (`EDIT00000000`) that says\nwhich player wears which boots and gloves, or `--loose` to guess the wearer of\neach boots or gloves folder from its ID alone. A referee export (a name whose\nfirst word is `refs` in any letter case) takes neither.\n\nWith `--loose` each boots or gloves folder goes to the player at its ID's\nposition in the team's old block of 25 IDs, the block its kind's lowest ID\nfalls in (`k0128` is player 03 of the 126 block). Each pairing is reported as\n`wearer_assumed`, so it can be checked. An ID on the block's last two\npositions or past it belongs to no player and is reported as\n`unmigrated_content`. Without a save no look is written. A player folder with a name still\ngets a `settings.toml` whose only setting is `name = true`.\n\nThe output is a new folder beside the input, or inside the folder `-o` names\n(created when missing). Its name is the team's key with the coverage tag\ninserted: `co - Spring` becomes `co Full - Spring`. An export whose name holds\n`midcup` or `additions` in any letter case, and no tag as its second word,\ngets `Midcup` instead, and `--coverage full` or `--coverage midcup` sets the\ntag outright. A name whose second word is already a tag, in any letter case,\nkeeps it and refuses a flag naming the other. A referee export's name takes no\ntag and refuses `--coverage`.\n\nThe tag used is always reported (`coverage_tag_chosen`, naming what decided\nit), because an export that was only a mid-cup update gets `Full` when its\nname says neither, and a `Full` export's first compile builds placeholder\nkits over the team's installed ones. When the output's name is already the\ninput's own, letter case aside, ` upgraded` is appended so the original is\nnever written over. An output folder that already exists stops the run and is\nleft untouched.\n\nEvery team needs its logo, so an export upgraded as `Full` with no logo in its\n`Logo/` folder is the error `logo_missing`, and nothing is written. On PES\n2017 to 2021, a player with no portrait in an export upgraded as `Full` is the\nwarning `portrait_missing`, and the export is still written.\n\nA referee export is upgraded from its `refs.txt`, or from the older\n`Refs list.txt` when there is no `refs.txt`. The output gets a `players.txt`\nholding the same slots in the new spelling (two-digit slot numbers, one entry\nper line) and no `refs.txt`, and its `ref_marker.dds` is carried as it is. A\nroster file that is not UTF-8 is reported, and no `players.txt` is written. A\nreferee export with neither `refs.txt` nor `Refs list.txt` has no roster to\nmigrate, so it is reported (`refs_txt_missing`, or `no_old_export` when it\nholds nothing else the migration reads) and nothing is written.\n\nThe referee folders may also stand directly in the export, with no `Players/`\nfolder, and the migration is the same. The old `face/`, `boots/`, `gloves/`\nand `common/` subfolders of each referee's folder are folded into that folder,\nand a file held twice with the same bytes is kept once. The referee's folder is\nleft as it was, and the cause reported, when:\n\n- two files would land on one path with different bytes;\n- two textures from different places would land on one stem (a `hair.dds` and\n  a `hair.ftex`);\n- a subfolder of one of those four names is inside them.\n\nIn a referee folder that folds, a `common/` model no `face.xml` of the referee\nnames is left out and reported.\n\nThe original export is never modified. A file, a folder or a Team Note entry the migration\ncannot place yet is listed in the report as `unmigrated_content`, the line naming the cause\nand, where another file or folder already took the place, that one's path.\nContent that never reached a player in game (a stray file, a folder nobody\nwore) is dropped and listed as `leftover_dropped`, a warning that does not\nstop the migration.\n\nThe upgraded folder is checked against the same rules the Compiler\napplies. Every finding prints as one line, which names:\n\n- the export it is about, when there is one;\n- how serious it is;\n- its code;\n- where in the export it is, when it is about a file or folder;\n- its details in parentheses, when it has any.\n\nA run that gets as far as its verdict ends the report with `migration_clean`\nor `migration_not_clean`, and nothing is written in the second case. When a\nfinding stops the run at once, that finding is the report's last line instead.\n\nAn incomplete `<name>.upgrading` folder an earlier run left behind is removed\nand reported (`staging_folder_removed`) before this run writes its own. One\nthat cannot be removed aborts the run.\n\n**Several exports at once**\n\nWith several exports, the options given apply to all of them, and each export\nis upgraded as if it were run alone, so one that is refused or stops does not\nstop the others.\n\nThe run prints a line `Upgrading <name>` before each export's findings, with an\nempty line between exports. When an export is refused or stops, the reason is\nthe last line of its part of the report, starting with `error:`. The run ends\nwith the count of exports upgraded and not upgraded (`Upgraded: 48. Not\nupgraded: 2.`), and an export counts as upgraded when its migration is clean. A\nrun with `--all` prints these lines even when the folder holds one export.\n\nThe whole run is refused before any export is read when an argument is bad, the\nsavefile does not read as a save, or the `--all` folder is missing, is not a\nfolder or holds no export.\n\n**Exit codes**\n\n- 0 when the migration is clean;\n- 1 when the migration is not clean, after an error finding or after a finding\n  that drops anything but a leftover (a warning that drops a folder counts,\n  because the compile would lose it);\n- 2 when the command was refused before anything was written (a bad argument,\n  an input that is not there or not of the kind the command takes, a savefile\n  that does not read as a save or does not hold the team);\n- 3 when the run aborted, either before anything is written (the output folder\n  exists, a leftover `.upgrading` folder cannot be removed, or the output\n  cannot be created) or when the output cannot be written, moved into place or\n  removed and the incomplete output is left.\n\nWith several exports, the exit code is the highest any of them gave.\n",
          "slug": "command-line",
          "title": "Running from a terminal"
        },
        {
          "body": "An `unmigrated_content` line names content the old export was meant to show in\ngame, which the migration could not place. While any such line stands, the run\nwrites nothing: fix the cause in the old export and upgrade again.\n\nThe path after `at` is the file or folder the line is about, then `reason=`\nnames the cause, one of the values below. Where two files or folders claim one\nplace, `other=` names the one that has it. Some causes add `line=` for the\nTeam Note entry, `error=` for why a file does not read, or `name=` for a\nfolder.\n\n**Face folders and portraits**\n\n- `slot_invalid`: the face folder's number, the two digits after its name's\n  first three characters (`15` in `XXX15 - Snuffy`), is not a player number\n  from 01 to 23 (`XXX24`, `ab15x`). Rename the folder so it is.\n- `slot_taken`: a second face folder claims a player's number; `other=` names\n  the folder that has it. Merge their files into one, or rename the one meant\n  for a different player.\n- `portrait_taken`: two portraits claim one player, a second `Portraits/` file\n  of one number or two files ending `portrait.dds` inside one face folder.\n  Keep one of them.\n\n**Boots and gloves folders**\n\n- `id_invalid`: the folder's name is not a boots folder's `k` or a gloves\n  folder's `g` and four digits, then the name part (`K0430`, `k430`). Rename\n  it to that form, the letter lowercase, like `k0430 - Crocs`.\n- `id_taken`: a second boots or gloves folder carries one ID; `other=` names\n  the folder that has it. Keep one of the two, or renumber it.\n- `loose_unplaceable`: under `--loose` the folder's ID sits on a spare position\n  of its kind's block of 25 IDs, or past the block, so it is no player's.\n  Rename it to an ID of the block, or upgrade with `--savefile` so the save\n  says who wears it.\n\n**Kit configs, kit textures and the Team Note**\n\n- `config_name_invalid`: the file's name is no config name\n  (`XXX_DEF_1st_realUni.bin`, `_GK1st_` for the goalkeeper's). Rename it to\n  that form, or remove it if it is no kit config.\n- `config_too_deep`: the file sits deeper than the one `Kit Configs` subfolder\n  the old layout read. Move it up into `Kit Configs/` or its subfolder.\n- `config_taken`: a second config of one kit, different in content; `other=`\n  names the one kept. Keep one of the two.\n- `config_undecodable`: the file does not read as a kit config (`error=` says\n  why). Replace it or remove it; a kit left with no config still gets its\n  folder, a placeholder the compiler fills.\n- `kit_entry_past_ninth`: a player kit entry of the Team Note past the ninth\n  (`line=` the entry): there is no kit number for it. Remove it, or mark it\n  `gk:` when it was meant as the goalkeeper's.\n- `colors_unreadable`: a Team Note entry's colors do not read, a team-color\n  entry's or a kit entry's (`line=` the entry). Fix the line: a color is\n  `#RRGGBB` or three numbers (`211 74 79`), and a kit entry's two colors split\n  at ` - `.\n- `texture_undecodable`: an image file does not read, a kit `.ftex` that\n  fails its conversion or a PNG in `Logo/` (`error=` says why). Save it again\n  in a readable form, or replace it.\n- `texture_name_taken`: two kit textures would land on the same name in the\n  upgraded kit (`u0XXXp1.dds` beside `u0XXXp1.ftex`); `other=` names the one\n  kept. Keep one of the two.\n\n**The `Logo` folder**\n\n- `logo_member_invalid`: the file is not a logo image the old format reads:\n  it sits in a subfolder, or its name ends in none of `_r_ll.png`, `_r_l.png`,\n  `_r.png`. Rename it to the ending it was meant as, or move it out of\n  `Logo/`.\n- `logo_role_taken`: a second file claims one of the three endings, two\n  ending `_r.png` for example; `other=` names the one kept. Keep one file per\n  ending.\n\n**Models and other files**\n\n- `mtl_unparsable`: the `.mtl` cannot be read even after stray text inside it\n  is dropped (`error=` says why). Fix the file in a text editor, or remove it\n  from the folder.\n- `path_taken`: two files would land on the same path in the upgraded export\n  once renames apply (`A.dds` beside `a.dds`, or numbered copies of one kit\n  texture, `u0731p2.dds` beside `u0799p2.dds`); `other=` names the one kept.\n  Keep one of the two.\n  A copy spelled with letters (`u0XXXp2.dds`) beside numbered ones is no\n  conflict: it is the one the game loaded, so it is kept and each numbered\n  copy is dropped as a leftover.\n- `note_taken`: a second `.txt` at the export's root whose name holds `note`;\n  `other=` names the file read as the Team Note. Keep one of the two.\n- `unclaimed`: the file sits where nothing reads it: directly in `Players/` or\n  inside a folder the export does not use (a `Players/` folder in a team\n  export, `Kit Configs/` in a referee export). Move it into a folder the\n  export reads, or remove it.\n\n**Referee exports**\n\n- `model_unreferenced`: a model in the referee's `common/` that no `face.xml`\n  in his folder names. Remove it, or name it in a `face.xml`.\n- `name_taken`: two files from the referee's `face/`, `boots/`, `gloves/` or\n  `common/` subfolders would land on the same path in his folder, with\n  different content (`other=` the other file). Keep one of the two.\n- `stem_taken`: two textures from different subfolders would land on the same\n  stem in the referee's folder, a `hair.dds` and a `hair.ftex` (`other=` the\n  other file). Keep one of the two.\n- `nested_layout`: one of the referee's `face/`, `boots/`, `gloves/` or\n  `common/` subfolders holds another folder of the same four names inside it\n  (`face/face/`). Move the inner folder's files up into the outer one.\n- `roster_not_utf8`: `refs.txt` is not UTF-8, so no `players.txt` can be\n  written. Save it with UTF-8 encoding.\n",
          "slug": "unmigrated-content",
          "title": "Fixing unmigrated content"
        }
      ]
    }
  ],
  "messagesIntro": "Where a row says \"the line\", it means the line the finding prints in the log or the terminal."
};
