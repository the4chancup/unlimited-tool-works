# Unlimited Tool Works - Glossary

One line per term, with the plan section that owns the full definition. This file defines nothing
on its own: if a line and its plan section disagree, the plan is right and the line gets fixed.
Add a term when a session had to look it up or an agent misused it. Within a group, put related
terms next to each other rather than in alphabetical order; adjacency is what makes them stick.

## Project and process

| Term | Meaning | Owner |
|---|---|---|
| **4cc** | The 4chan Cup community: PES-based virtual football cups between board teams. Studio is its tool suite. | `spec/core/README.md` "Overview" |
| **Unlimited Tool Works (UTW)** | The suite's name, wherever a member sees it: the window, the help, the executable `UTW.exe` (`utw` on Linux), the `unlimited-tool-works` folders, `utw.log`. | `plans/core/development_plan.md` "The project's name: Unlimited Tool Works" |
| **Studio** | The project's codename, which the crates (`studio`, `studio_core`), the types (`StudioTool`) and the docs' and code's prose keep as the suite's shorthand; no member-facing text uses it. | `plans/core/development_plan.md` "The project's name: Unlimited Tool Works" |
| **Acceptance ID** | Stable identifier (`TC-KIT-03`) of one testable, user-observable behavior in a tool plan's "Acceptance" section; tests cite it. | `CONTRIBUTING.md` "Testing" |
| **ATF** | The Autism Task Force ("autism" in the meme sense, obsessively dedicated): the community members who checked every tactical export by hand before each cup. **AATF**, the Auto ATF, is the rules check that does that job now (`libs/aatf`). The **Aesthetics ATF** is the semi-manual check of a cup's aesthetics exports, aided by the maintainer's aes-atf-bot. | `spec/aatf_rules.md`; `plans/core/development_plan.md` "Unscheduled tools" |
| **Converge** | End-of-phase audit of code against the plan section and acceptance IDs; gaps become worklog steps. | `AGENTS.md` "Working documents" |
| **Finding** | A lib crate's structured report of a condition (stable code + context), free of user-facing text; tools map findings to messages. | `AGENTS.md` "Fundamental concepts" |
| **Gate** | One of the verification commands that must pass before a step is done (`fmt`, `clippy -D warnings`, `test`, `wasm32` check). | `CONTRIBUTING.md` "Testing and verification" |
| **Legacy tools** | Red, Blue, 4ccEditor, Midcupping, Rigdio/RigDJ, SEN:P-AI, the converters, the Blender addons: evidence of required behavior and formats, never source to translate. | `spec/core/README.md` "Relationship to legacy tools" |
| **Phase** | A numbered unit of the development plan (1–20), each with its own verification; the worklog tracks steps within it. | `plans/core/development_plan.md` "Development Plan" |
| **Plan** | The documents under `docs/plans/`: work not built yet, with its design and rationale, written in the present tense; each file opens with a banner saying so. | `plans/README.md` |
| **Spec** | The documents under `docs/spec/`: what the suite does now, and why. A phase's sections move there from the plans when it closes, under the same file and heading. | `spec/README.md` |

## Platform

| Term | Meaning | Owner |
|---|---|---|
| **Lib crate** | A crate under `crates/libs/` holding one format or one shared concern; depends only on other libs; returns findings, never messages. | `spec/core/architecture.md` "Crate structure" |
| **`PipelineEvent`** | The typed progress/result event tools emit (`ExportStarted`, `FolderStatus`, `Message`, `Progress`, `Complete`); GUI and CLI render it. | `spec/core/architecture.md` "Event system" |
| **`ScopePath`** | `vtree`'s canonical, platform-neutral, validated path inside an export; addresses events, findings and grid cells. | `spec/libs/README.md` "`vtree` path types" |
| **Stale-result envelope** | `PipelineEventEnvelope` carrying `run_id` and `export_revision`, so results from an outdated filesystem snapshot cannot overwrite newer state. | `spec/core/architecture.md` "Event system" |
| **`studio`** | The crate of the single binary, `utw` (`UTW.exe` in the Windows bundle): registers tools, launches the GUI or dispatches `UTW <tool-id> <command>`. | `spec/core/architecture.md` "Crate structure" |
| **`studio_core`** | The platform crate: shell, `StudioTool` trait, settings framework, common widgets, event types. Holds only what runs with zero tools installed; its module tree is closed. | `spec/core/architecture.md` "Architecture" |
| **`StudioTool` / `ToolContext`** | The plugin trait every tool implements (id, settings, `cli_run`, `gui_run`) and the platform services handed to it. | `spec/core/architecture.md` "Tool plugin interface" |
| **Studio Web** | Post-release, deliberately lite browser edition; the reason every lib crate stays `wasm32`-checkable. | `spec/core/gui.md` "Browser deployment" |
| **Run strip** | The one-line bar beside a pipeline tool's run button (stage, elapsed time, files written, memory): the tool's own, drawn in its view. Not "status strip" (the shell's bar is the status bar) and not "strip" (a kit). | `plans/team_compiler/gui.md` "Toolbar" |
| **Status bar** | The shell's one-row bar under every tool view, with three typed slots (condition, another tool's activity, the latest notice). | `spec/core/gui.md` "Status bar" |
| **Draft (GUI prototype)** | The web page under `docs/prototypes/` that every view and shell element is drawn as (the host page `index.html` for the shell, a script under `tools/` for each tool's view), limited to what the planned UI crates can render, refined with the maintainer before egui code is written, and kept as the view's specification. | `plans/core/gui.md` "Prototypes before implementation" |
| **Tool crate** | A crate under `crates/tools/`, one per tool, with the fixed skeleton `lib.rs` / `settings.rs` / `cli.rs` / `messages.rs` / `view/`. Never depends on another tool crate. | `spec/core/architecture.md` "Workspace guardrails" |
| **Tool id** | The hyphenated stable identifier of a tool (`team-compiler`), used in CLI subcommands and `MessageCode`. | `spec/core/architecture.md` "Tool plugin interface" |

## Exports and compilation

| Term | Meaning | Owner |
|---|---|---|
| **AET** | The Aesthetics Export Template, the layout of aesthetics exports before Studio (Red's input: `Faces/`, `Boots/`, `Gloves/` item folders with embedded IDs, `Kit Configs/`, `Kit Textures/`, `Other/`; the wiki page is `resources/AET.wikitext`), and the name of an export in it; the plans also say "old layout". The compiler does not read it; the Export upgrader migrates it once. | `spec/export_upgrader.md` |
| **Aesthetics export** | A team's folder or archive of player folders, kits, portraits and `settings.toml`; the Team compiler's unit of work and the project's primary motivation. | `plans/aesthetics_export/README.md` |
| **Anti-blur mesh** | A Fox model's duplicate of a `Constant`-material mesh, with a `Blin` material, hidden by the original: a `Blin` mesh behind a `Constant` one cancels the TAA blur in game, at the cost of the mesh's vertices and faces again; the `antiblur` FMDL extension flag marks the duplicates, which the Blender add-on drops on import and recreates on export. To retire once FoxDen forces TAA off. | `spec/libs/format_crates.md` "Anti-blur meshes" |
| **Autopilot team** | A cup team without a manager: its aesthetics export is made by a caretaker and its tactical export by the 4cc council, often before the destination team is known, so its players' names come from the names in the aesthetics export (the aesthetics patch). Not the music player's Autopilot mode. | `plans/pes_savefile/operations.md` "Aesthetics patch" |
| **Complete export** | A team's aesthetics export carrying its tactical half as `tactics.toml` (a Team TOML) and its music export as `Music/` at its root: the looks from the aesthetics half, names, gameplay and tactics from the tactical file, the match-day music from `Music/`. Each part optional; each alone still works. | `plans/aesthetics_export/player_folders.md` "Complete export" |
| **Roster order rule** | By 4cc rule a team's roster slot NN holds player `team id × 100 + NN`; it links an aesthetics export (by id) to a Team TOML (by slot). PES lets a roster be reordered, so the AATF check reports a break. | `plans/aatf_rules.md` "The host" |
| **Referee export** | An aesthetics export under the reserved `/refs/` team name; prepared by the Refs arranger, compiled by the Team compiler. Only one may be enabled. | `plans/refs_arranger.md`; `plans/team_compiler/README.md` |
| **Referee hook** | FoxDen's `03_refmod.lua`: a trampoline on the Fox games' referee slot-writer that forces or remaps the referee id for each of a match's five positions, so which referee appears is decided by a script rather than by the game's slot draw (PES 18–21). | `plans/refs_arranger.md` "The Fox referee hook" |
| **Referee list** | One line of `ref_lists.txt`: five distinct referee slot ids, one per match position, applied by the referee hook's script per match in file order. Written by the Refs arranger beside `players.txt`. | `plans/refs_arranger.md` "The lists file" |
| **Balls export** | The third export kind: a ball selection compiled by the Balls compiler into its own CPK. Its name's first word is `balls`. | `plans/balls_compiler.md` |
| **`BuildTask`** | One planned unit of compile work (a folder, a kit, a bin update) with its owning scope and planned model-ID assignments. | `spec/team_compiler/pipeline.md` "Per-export serial steps" |
| **Deep / shallow check** | Shallow: validate what is visible without extracting an archive (`PartialOk`/`PartialError`); deep: full validation (`FullOk`/`Error`). | `spec/core/architecture.md` "Event system" (`FolderStatus`) |
| **Export revision** | The pinned identity of an export's source contents for one run; a change mid-run is `source_changed_during_run` (`AbortRun`). | `spec/team_compiler/messages.md` "Message catalog" |
| **Export upgrader** | The tool that migrates AET exports (the old layout) to the Studio player-folder format once; the compiler does not read the AET layout. | `spec/export_upgrader.md` |
| **Coverage tag** | `Full` or `Midcup`, the second word of a team export's name: whether the export is everything the team has (the compiler rebuilds the team's records from it alone) or additions to what is installed (only what it holds is replaced). A name with neither is an error. | `spec/aesthetics_export/object_model.md` "Validation semantics" |
| **FPC** | Full Player Customization: the appearance + kit-config settings that hide a player's default body so the model supplies it. Toggled per player by `fpc_on` / `fpc_off` marker files. | `spec/aesthetics_export/fpc_toggle.md` "FPC toggle"; `plans/save_editor.md` |
| **Music export (`.4ccm`)** | A team's match-day audio package (anthem, goalhorns, chants, conditions) for the Music player. The `.4ccm` format stays canonical. | `plans/music_player.md` |
| **Help chapter** | A tool's part of the help window: the Markdown topics in its crate's `help/` folder (`StudioTool::help()`), plus a Messages topic generated from its `messages.rs` catalog. | `spec/core/gui.md` "Help window" |
| **Kit folder** | One kit's `config.toml`, `colors.txt` and textures under `Kits/`, named `<slot>[ - <label>]` (`p1 - Lakers`); `Kits/all/` holds textures every kit inherits per stem. | `spec/aesthetics_export/player_folders.md` "Kits" |
| **Kit layout** | Which engine's kit UV layout a main kit texture is drawn for (`KitLayout`: `PreFox` for PES 15–17, `Fox` for 18–21); the shirt, sleeves and shorts are identical, the sock islands differ. Declared by an empty `pre-fox` / `fox` marker file in the kit folder; absent = the compile target's. | `spec/aesthetics_export/player_folders.md` "Kit layout marker" |
| **Kit slot** | A kit's position (`KitSlot`): `p1`–`p9` player kits, `g1` goalkeeper; the slot part of a kit folder name and of the game's `u0{team}{slot}` texture names. | `spec/aesthetics_export/player_folders.md` "Kits" |
| **Link file** | An empty file whose name is the reference: `Crocs.boots`, `Longhair.face`, `Keeper.gloves` point a player folder at a shared folder of that category (at most one per category); `<file>.common` points at a file in `Common/` (see "Common link"). Every link file accepts a stray `.txt` suffix (`Crocs.boots.txt`), because Windows hides known extensions and Notepad adds one. | `spec/aesthetics_export/object_model.md` "Player folders" (shared models); `spec/model_format.md` "Link files" |
| **User-supplied `face.xml`** | A `face.xml` a member ships in a pre-Fox player folder instead of letting the compiler generate one: the authority on what the face loads (filename typing is off), checked for content, its references resolved and the file re-serialized; a `./` reference packs its file under the referenced name, a Common one is written as the Common output packs the model. Ignored on a Fox target. | `spec/team_compiler/messages.md` "User-supplied `face.xml`" |
| **`NO_USE` marker** | Root `NO_USE` / `NO_USE.txt` file that disables an export source; it is skipped and omitted from the grid. | `spec/team_compiler/messages.md` "Message catalog" |
| **Parity golden standard** | Red's CPK output on the fixture set: the Team compiler's output must match it byte for byte where the plan says so. | `AGENTS.md` "Fundamental concepts" |
| **Player folder** | One player's model, textures and per-player `settings.toml` inside an aesthetics export. | `spec/aesthetics_export/object_model.md` "Player folders" |
| **Player slot** | A roster position (`PlayerSlot`) in `players.txt`; `DropSlot` discards exactly one normalized assignment. | `spec/team_compiler/messages.md` "Message catalog" |
| **Roster file** | The Team creator's input: plain text, one player per line in shirt-number order, optional position / medal / `captain` tokens after two spaces or a tab, optional leading number. | `plans/team_creator.md` "2. Players" |
| **`settings.toml` / `config.toml` / `materials.toml`** | User-edited TOML inside exports (player aesthetics; kit config; model materials). Carry app-injected per-field comments; written only through `toml_edit`. | `spec/aesthetics_export/settings_toml.md`; `spec/kit_config_editor.md` "TOML form"; `spec/model_format.md` "`materials.toml` schema" |
| **`ingame_face.toml`** | A player folder's file holding the in-game face's keys, whose presence marks the folder as wearing it. | `spec/aesthetics_export/player_folders.md` "`ingame_face` marker" |
| **Static painting** | Weighting a model's vertices to a dummy vertex group (usually named `static`) that is no bone of the body skeleton, so no animation moves them: the model stays where it was modeled. How the referee marker stays on the ground while drawn as a collar. | `spec/team_compiler/blue_port.md` "Referee export processing" |
| **Stock band** | Boots/gloves IDs 0–100: the game's own models (the cup's stock kit compacts Konami's into it), below the compiler's per-team blocks. Gloves 0 is a set of normal hands: any player can wear a gloves model. The only IDs `settings.toml`'s `boots_id`/`gloves_id` accept; `""` there means default (FPC marker or 0). | `plans/aesthetics_export/settings_toml.md` "Player settings in exports" |
| **Starter head** | A ready player folder embedded in the Team creator (in-game head, cardhead, boxhead) that needs at most one image; the tool's word for the wiki's replace-a-texture model templates. Not a "preset": in the community that word means a tactics preset. | `plans/team_creator.md` "Starter heads" |
| **Team creator** | The new-team wizard: name + roster file → legal Team TOML (AATF-checked defaults) and a compilable aesthetics export built from starter heads. Owns no format, keeps no state, hands off to the compiler and editors. | `plans/team_creator.md` |
| **Ruleset editor** | The tool that builds AATF rulesets through forms (tiers, heights, cards, specials) and writes rules files; for invitational organizers. | `plans/ruleset_editor.md` |
| **`team_widgets`** | The egui widget lib over `pes_savefile`'s team model (tactics pitch and instructions, card pickers, AATF violations list) shared by the Save editor and the Team creator; takes `&mut` model, returns what changed, never sees a session or a file. | `plans/save_editor.md` "`libs/team_widgets`" |
| **Team name** | A 4cc team's `/xx/` board name (`TeamName`), the canonical identity of an export: derived from the first word of the export's folder or archive name, lowercased and wrapped in slashes; `/refs/` and `/balls/` are reserved. In a 4cc savefile the in-game team name field holds the same `/xx/` name; the save editor edits that field, and "Export teams list" writes it into `teams_list.txt`. | `spec/libs/teams_list.md` "`libs/teams_list`" (type, fold); `spec/team_compiler/pipeline.md` "Export display name and team name" (derivation) |
| **Team ID** | The numeric PES team identifier (`TeamId`), valid 701–920 for normal teams; resolved from the team name via `teams_list.txt`. | `spec/libs/teams_list.md` "`libs/teams_list`"; `spec/team_compiler/pipeline.md` "Export identity resolution" |
| **Templates override** | A `templates/` directory whose files shadow embedded CPK/bin templates; reported per file per run. | `spec/team_compiler/messages.md` "Message catalog" |
| **Overrides folder** | `overrides/` in the data directory (Red's `sideload/`): files injected into the normal team CPK that win over export entries at the same output path. Not related to sideload mode. | `spec/team_compiler/pipeline.md` "5. Writer" (override priority) |
| **Sideload mode** | The Team compiler output mode (`--mode sideload`) writing the unpacked PES folder structure to the game folder's `livecpk\`, which FoxDen (PES 18–21) or the user's Sider 3 (PES 17) serves to a running PES; refused on PES 15/16. | `spec/team_compiler/pipeline.md` "5. Writer" (output modes) |
| **Output sink** | `OutputSink`: where the writer puts each entry by its output-relative path, the CPK or the loose folder of test and sideload modes (also the harness the parity test drives); knows nothing of game paths, which the materialize step owns. | `spec/team_compiler/pipeline.md` "5. Writer" (output modes) |
| **Teams part** | One CPK of the size-split `teams` sequence multi-CPK mode fills with whole teams, first-fit in canonical order under `cpk_part_max_size`; its slots are the DpFileList entries `{prefix}_{NN}_{teams_cpk_name}`. | `spec/team_compiler/pipeline.md` "5. Writer" ("Multi-CPK mode: teams parts") |
| **Placeholder CPK** | The 6,272-byte CPK holding only an 11-byte file `placeholder` that the official DLC ships for unused slots (`4cc_68_midcup.cpk` is one); a multi-CPK run writes it into every teams slot it does not fill, so stale parts cannot survive. | `spec/team_compiler/pipeline.md` "5. Writer" ("Every slot is always written") |

## Messages

| Term | Meaning | Owner |
|---|---|---|
| **Disposition** | The processing consequence of a message, independent of severity: `Keep`, `DropFile`, `DropSlot`, `DropFolder`, `DropExport`, `AbortRun`. | `spec/team_compiler/messages.md` "Message structure" |
| **`DoneWithErrors`** | Folder outcome when content was written despite Error-level findings (`pass_through`, or `DropFile` with the rest processed). | `spec/core/architecture.md` "Event system" |
| **Message** | A tool's structured report to the user: `MessageCode` + severity + disposition + scope + context fields. Text is rendered from the tool's catalog, never baked in. | `spec/team_compiler/messages.md` "Message catalog" |
| **Message catalog** | A tool's table of message codes with template text and remediation hint; `messages.rs` in each tool crate. One condition, one ID. | `spec/team_compiler/messages.md` "Message catalog" |
| **`pass_through`** | Setting that turns eligible content-level `DropFile`/`DropFolder` dispositions into keep-with-flag, as in Red; never affects `DropSlot`/`DropExport`/`AbortRun`. | `spec/team_compiler/messages.md` "Message catalog" |
| **Scope** | What a message is about: `Run`, `Export`, `Folder`, `File`, `RosterEntry`; drives grid-cell mapping. | `spec/team_compiler/messages.md` "Message structure" |
| **Severity** | Presentation/filtering level: `Info` < `Warning` < `Error` < `Fatal`. Never decides processing consequence. | `spec/team_compiler/messages.md` "Message structure" |

## Formats and engines

| Term | Meaning | Owner |
|---|---|---|
| **CPK** | CRI archive format PES loads; the compiler's output container. May hold CRILAYLA-compressed entries. | `plans/libs/README.md` |
| **CRILAYLA** | CRI's bitstream LZ compression used inside CPKs. | `spec/libs/README.md` "Format references" |
| **DDS / FTEX** | DDS: the texture container for pre-Fox; FTEX: Fox Engine's texture container. `dds_convert` handles BCn encoding in-process (no texconv). | `plans/libs/README.md` |
| **FMDL / FPK** | Fox Engine model format and its package container (PES 18–21). | `plans/libs/README.md`; `plans/model_conversion/README.md` |
| **fox2** | Fox Engine entity files (stadiums), written from `fox2.xml` sources; strings are CityHash64-hashed. | `plans/stadium_compiler.md` |
| **Fox / pre-Fox** | The two PES engine generations: pre-Fox (PES 15–17, `.model` + `.mtl`, DDS) and Fox (PES 18–21, FMDL/FPK/FTEX). | `AGENTS.md` "Fundamental concepts" |
| **glTF authoring format** | Studio's single model authoring format: glTF plus optional `PES_bone` / `PES_mesh` extensions and `materials.toml`; converted to either engine at compile time. | `plans/model_format.md` |
| **Common link** | A link file named after a file in the export's `Common/` folder plus `.common` (`torso.glb.common`, `body.materials.toml.common`, `hair.png.common`), making that model, material file or texture resolve in the player folder as if it were local. A texture reached through Common is packed once per team and referenced in place. | `spec/model_format.md` "Link files" |
| **Global model file** | The user-facing name for a `.glb` authoring-format model, in the community's family of "fox model file" (`.fmdl`) and "pre-fox model file" (`.model`). A nickname, not an expansion of "glb"; plans and code say glTF. | `plans/model_conversion/gltf.md` "What users call it" |
| **IR** | `model_convert`'s intermediate representation; cross-format conversion goes through it, same-format work stays in the format crate's `ops/`. | `plans/model_conversion/README.md` |
| **Kit config** | The per-kit binary parameter block (collar, shorts, numbers…), reverse-engineered and editable as TOML `config.toml`. | `spec/kit_config_editor.md` "The format" |
| **`.model` / `.mtl`** | Pre-Fox mesh and material files. | `plans/libs/README.md`; `plans/model_conversion/README.md` |
| **uniparam** | The `UniformParameter` container in the team bins (kit configs, colors) the compiler accumulates into. | `plans/libs/README.md`; `spec/aesthetics_export/fpc_toggle.md` "FPC toggle" |
| **WESYS** | zlib wrapper with a custom header used by PES for some game files. | `spec/libs/README.md` "Format references" |

## Savefile

| Term | Meaning | Owner |
|---|---|---|
| **AATF rules** | The 4cc rule set constraining player stats and medals, the Auto ATF's (see "ATF"); checked and applied by `libs/aatf` from a rules file. | `spec/aatf_rules.md` |
| **Ruleset / rules file** | One AATF ruleset as a `.4ccrules` file (plain Rhai): the data (`RULESET`: tiers, heights, card economy, specials), the generic interpreter Studio writes into every file, and optional custom checks. The official one is embedded; invitationals make their own in the Ruleset editor. | `spec/aatf_rules.md` "The rules file" |
| **Tier** | A class of player in a ruleset (gold, silver, bronze, regular, …) with its own stat targets, form, limits and per-team count; a player's tier is recognized from their rating. "Medal" is the community's word for the non-regular tiers. | `spec/aatf_rules.md` "The ruleset schema" |
| **Special** | A conditional rule of a ruleset: *players matching these conditions get these effects* (a stat bonus, extra or free cards, a height allowance, a forbid…). | `spec/aatf_rules.md` "The ruleset schema" |
| **Suggestion** | An AATF finding of an allowance a player does not use up (fewer cards than allowed, weak foot below the maximum…); listed apart from errors, uncounted, shown on request. | `plans/aatf_rules.md` "The interpreter" |
| **Aesthetics patch** | `aesthetics_patch.toml`, written by the Team compiler to its output folder: the resolved savefile writes for the compiled players. On Fox only their motions and, for an export carrying the `write_names` marker, their resolved `name`/`shirt_name`, appearance travelling in the CPK's player appearance tables; on pre-Fox also settings, FPC values, boots/gloves IDs and edit flags. Applied whole to a save by the save editor; also how the compiler updates the local save. Lets the DLC builder and the savefile builder be different people. | `plans/pes_savefile/operations.md` "Aesthetics patch" |
| **Aesthetics fingerprint** | Decoded appearance data of a player used to diff saves (Midcupping's `compare-saves`). | `plans/pes_savefile/README.md` |
| **Aesthetics transplant** | Copying a donor save's team's looks into that team's export, by roster slot (after Release 0.1.0); Midcupping's `transplant-aesthetics` copied appearance blocks save to save, not ported. | `plans/save_editor.md` |
| **Comparator** | Save-to-save diff of gameplay and aesthetics (4ccEditor's comparator merged with Midcupping's). | `spec/save_editor.md` "Comparator" |
| **`EditFile`** | `pes_savefile`'s decoded, editable representation of an `EDIT00000000` save. | `plans/pes_savefile/README.md` |
| **Interchange formats** | Team TOML (Studio) and legacy `.4ccs` / `.4cct` files for moving team data between saves. | `plans/pes_savefile/README.md` |
| **`PlayerSettings`** | The per-player aesthetic settings resolved from `settings.toml` at compile time: on Fox into the CPK's `PlayerAppearance.bin`, `BootsList.bin` and `GloveList.bin` rows, on pre-Fox into the savefile. | `plans/pes_savefile/README.md` |
| **Savefile (`EDIT00000000`)** | PES's encrypted edit-data file holding players, teams, tactics; per-version crypto and schema in `pes_savefile`. | `plans/pes_savefile/README.md` |
| **Stripped save** | A Fox savefile whose players' appearance ids are all -1, so the game takes every player's appearance, boots and gloves from the database tables (`PlayerAppearance.bin`, `BootsList.bin`, `GloveList.bin`) the Team compiler writes. Prepared once per version, seeding everyone's current appearance into `4cc_08_bins.cpk`. | `plans/save_editor.md` "Stripped save" |
| **Game database (`pesdb`)** | PES's fixed-size-record tables under `common/etc/pesdb/` (Team, Player, Coach, CompetitionEntry, Ball, Stadium, …) declaring what exists; the 4cc DLC overrides them with the cup's own. Record layouts in `libs/pesdb`. | `plans/db_generator.md` "The `pesdb` lib crate" |
| **Base player** | The per-version placeholder `PlayerEntry` (name `PLACEHOLDER`, default abilities and appearance) every cup player starts as; the DB generator writes it into the database and, on 19+, into the fresh save. | `plans/db_generator.md`, `plans/pes_savefile/operations.md` "Player section population" |
| **Placeholder (PH-kun)** | The community's name for the generic blond face the game draws for a player with no face of his own (default face settings, no face model, no base copy), such as a stripped player whose face was base-copied. | `plans/save_editor.md` "Stripped save" |
| **Day-0 save** | The `EDIT00000000` a fresh install writes from the cup's database before any team is edited; on 19+ it lacks player records until populated. | `plans/db_generator.md` "Background" |

## Stream-side tools

| Term | Meaning | Owner |
|---|---|---|
| **Autopilot** | Music player mode where `match_feed` events (goals, cards, halves) drive the soundboard without operator input. | `plans/music_player.md` |
| **Anthem / victory anthem / goalhorn / chant** | The audio slots of a music export: team music; post-win music (with selectable `special` variants); per-player or default (`goal`) scoring music; crowd loop faded out on a timer. | `plans/music_player.md` |
| **Event clip** | A music export line carrying the `event` instruction: a one-shot clip for a player's card, sub or own goal, kept apart from the slots and fired only by the Match tracker. | `plans/music_player.md` "Model" |
| **`match_feed`** | In-process typed pub/sub of live match events read from the game's memory, with snapshot replay; feeds the tracker view and the Music player. | `plans/match_tracker/README.md` |
| **Match record** | `.match.json`: the tracker's saved per-match stats/events (replaces binary `.sen`). | `plans/match_tracker/README.md` |

## Legacy tools (evidence only)

| Term | Meaning | Owner |
|---|---|---|
| **4ccEditor** | C++ PES save editor; source for savefile schema and editing behavior. | `plans/save_editor.md` |
| **Blue / Red** | The two Python compilers of AET exports (see "AET"). Red is the CPK-output parity standard; Blue supplies parser variants. | `spec/team_compiler/README.md` "Relationship to the older compilers" |
| **Midcupping** | Python scripts for save crypto, transplant and aesthetics diff. | `plans/pes_savefile/README.md` |
| **pes-fmdl / pes-model** | Blender addons for FMDL and `.model`; their hot paths move into the `python_bindings` wheel. | `spec/model_conversion/gltf.md` "Blender integration" |
| **Rigdio / RigDJ** | Music player and music export editor being replaced. | `plans/music_player.md`; `plans/music_export_editor.md` |
| **SEN:P-AI** | Match tracker being replaced. | `plans/match_tracker/README.md` |
