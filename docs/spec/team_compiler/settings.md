# Unlimited Tool Works — Team compiler plan: Settings

Part of the [Team compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Settings

Inherited from Red/Blue (`settings.ini`, `settings_default.ini`), reorganized into the suite's
two-level settings model: **common settings** live in `studio_core` (shared by all tools), **tool
settings** live in the Team compiler's own settings section.

### Common settings (studio_core)

| Setting | Old default | Notes |
|---|---|---|
| `pes_version` | 15 | Sidebar selector; 15–21. Unlimited Tool Works defaults to 19 as the version most relevant to the current community; derives `fox_mode` (≥18), per-version file extensions, game paths, bin names |
| `pes_folder_path` | `C:\Program Files (x86)\Pro Evolution Soccer 20**` | `**` replaced with the version; derives `download/` path and `PES20{version}.exe` path. Empty off Windows, where no default install folder exists |
| `pes_folders` | — | `[common.pes_folders]`, keyed by the version's number (`21 = '…'`); empty by default. A filled entry is that version's folder taken as written (no `**` expansion), overriding `pes_folder_path`; the settings menu's "A folder per version" rows |
| `exports_folder_path` | `exports/` | Watched by the GUI and scanned once by the CLI; belongs in `studio_core` so the Team, Refs, and Balls tools share one export source. Relative paths resolve **beside the executable** in both data-location modes (see "Path resolution"), so the folder sits next to `quick_compile.bat` like Red's `exports_to_add/` |
| `thread_count` | 0 (auto) | Was a Blue CLI arg; auto = logical cores − 1 |
| `memory_cap_percent` | 80.0 | Was a Blue CLI arg; memory budget cap, as a percent of the physical memory available at run start (`spec/libs/pipeline.md` "Memory cap"); `0 < percent <= 100` |
| `check_for_updates` | 1 | Was per-compiler (`updates_check` in Red's ini); becomes a suite-wide update check |
| `check_interval_hours` | 60 minutes (hardcoded) | `24`: hours between release checks. With `last_update_check` and `skipped_version` (both empty by default), the update state of `spec/core/distribution.md` "Update state". The updater that reads these three keys and `check_for_updates` is not built, so the settings menu draws no row for them |
| `last_update_check` | — | Update state (above): when the last release check ran. No row |
| `skipped_version` | — | Update state (above): a release the member chose to skip. No row |
| `theme` | — | `dark` or `light`, default `dark`: the GUI's color theme |
| `last_tool` | — | The tool active when the GUI last closed, which it reopens; empty on a first run. Written by the shell, no row |
| `window_main` | — | The main window's last place, `[x, y, width, height, scale, maximized]`, written when the window closes, the sixth number 1 when it was maximized (the first five then being the place its restore button gives back) and 0 when not; a record of five numbers reads as not maximized; empty until the first close. No row |
| `fpc_hide_boots_id` | — | `55`: the boots ID the FPC hide preset writes, one the game has no boots model for |
| `fpc_hide_gloves_id` | — | `11`: the gloves ID it writes (gloves 1 to 10 are the goalkeepers' real ones) |

### Team compiler settings

"Old default" is Red's value (— for settings Red did not have); "New default" is the Studio's. A
differing new default is a deliberate decision, explained in the Controls column.

| Setting | Old default | New default | Controls |
|---|---|---|---|
| `cpk_name` | `4cc_90_test` | `4cc_99_test` | Single-CPK output `CpkStem` (shared validation contract in the Writer section); the new default is the official DpFileList's test entry (`pipeline.md` "DpFileList upgrade") |
| `output_folder_path` | `patches_output/` (hardcoded) | `output/` | Root for promoted CPKs (degraded runs and `--no-deploy`), the `.staging/` folder, `test_output/`, and `teamnotes.txt`. Relative paths resolve **beside the executable** (Red's `patches_output/` in its new place; `%APPDATA%` is no place to look for a CPK) |
| `savefile_path` | — | auto | `EDIT00000000` to update. `auto` runs `pes_savefile`'s discovery under the user's **Documents\KONAMI** folder for the selected version (the savefile is never inside the PES install, so `pes_folder_path` plays no part — see "Savefile discovery" in the Savefile spec); newest account wins when several exist (`savefile_autodetected`); nothing found → `savefile_missing` (Red never touches the savefile) |
| `multicpk_mode` | 0 | same | Cup DLC mode: team content into size-split `teams` parts + the bins CPK (see "Multi-CPK mode: teams parts"); replaces Red's faces/uniform/bins content split |
| `teams_cpk_name` | `4cc_40_faces` + `4cc_45_uniform` | `teams` | Stem of the teams part slots; the slots themselves are every DpFileList entry matching `{prefix}_{NN}_{stem}` (`4cc_41_teams`, `4cc_42_teams`, …), ordered by number. A stem that is the bins CPK's own (`bins` for `4cc_08_bins`) is refused (exit code 2): the bins CPK would be a part too. Replaces `faces_cpk_name` and `uniform_cpk_name` |
| `cpk_part_max_size` | — | `3221225472` (3 GiB) | Cap per teams part (TOC included); the writer places whole teams, rolling to the next slot when the next team would not fit — teams are never split across parts. Chosen comfortably under Git for Windows' 4 GiB object ceiling; five slots give 15 GiB. Written as a whole number of bytes, not as text like `3 GB`: a count needs no unit parser and leaves no doubt between GB and GiB, and the findings show it in KiB, MiB or GiB. Single-CPK runs are not split, only warned (`cpk_size_over_limit`) |
| `bins_cpk_name` | `4cc_08_bins` | same | Multi-CPK bins name |
| `refs_cpk_name` | `4cc_35_referees` | `4cc_18_referees` | Referee CPK name, the new default the official DpFileList's. A name equal to `cpk_name`, or in multi-CPK mode to `bins_cpk_name` or one of the teams stem's slots (letter case aside), is refused as an invalid configuration (exit code 2): two writers on one file |
| `dds_compression` | 0 | **auto** | WESYS-zlib every emitted DDS. Tri-state `auto` / `1` / `0`; `auto` **follows `multicpk_mode`**, since multi-CPK is the cup DLC workflow and containing the full-cup DLC's size is the whole point of the setting — a manager compiling one team gets no compression and no cost, a cup maintainer gets it without remembering a second switch. **Pre-Fox only (PES ≤17)**: on Fox versions the setting is ignored whatever its value, because FTEX conversion already provides the size reduction there and the game does not expect zlibbed FTEX. Cheap in-process either way (see "DDS compression cost" below) |
| `strict_file_type_check` | 1 | same | Disallowed file types are errors vs info notes (`file_type_disallowed`) |
| `pass_through` | 0 | same | Keep folders with errors instead of discarding them (cup DLC workflow) |
| `teams_list_path` | `teams_list.txt` (hardcoded, beside the exe) | `teams_list.txt` | Location of the working teams list; relative paths resolve in the data directory (see "Path resolution"). Created from the embedded list on first run |

In the settings file the 0/1 settings are TOML booleans (`pass_through = false`). A setting enters
`TeamCompilerSettings` and `default_settings()` with the phase whose code reads it, not before,
so no key is written that nothing honors yet; the compiler reads `cpk_name`,
`output_folder_path`, `savefile_path`, `multicpk_mode`, `teams_cpk_name`, `cpk_part_max_size`,
`bins_cpk_name`, `refs_cpk_name`, `dds_compression`, `strict_file_type_check`, `pass_through`
and `teams_list_path`. A missing key
loads as its default; a key of the wrong type is a configuration error (exit code 2, naming the
key). A missing `teams_list.txt`, or no data directory yet, reads the embedded list without
writing it (only `compile` creates the file); one that exists but cannot be read or parsed stops
the run before any export is read (exit code 3, naming the path).

Output mode (normal / test / sideload) is not a persisted setting: it is the Compile button's dropdown
in the GUI and a flag on the CLI subcommand, as in Blue's `--mode` argument.

**DDS compression cost.** Red's `dds_compression` is a separate pass at the end of its
extraction stage: walk `extracted_teams/` (team exports only; referee content is never
compressed), read each DDS, Python `zlib.compress` at level 6, wrap in the 16-byte WESYS
header, write it back — I/O-bound and serial. In the Studio it is one more step in the texture task,
on bytes already in memory, running on rayon workers like everything else. Rough budget for a
200 MB pre-Fox export that is mostly DDS: `flate2` deflate at level 6 runs at roughly 30–60 MB/s
per core on block-compressed texture data (DXT blocks are high-entropy, so it is neither fast nor
very effective — expect 10–30 % size reduction), i.e. ~4–7 s of CPU spread over ~7 workers, well
under a second of wall time per export and far below the CPU BC3 encode of any raster-source
texture in the same task. **Measured at 4.16b** (36 real DDS files, 40.8 MB: a PES 17 kit pack,
a team's 2048² kit textures, a 5.6 MB Common body, five 1.4 MB model textures; `flate2` 1.1.10
with its default `miniz_oxide` backend, one thread, release build): level 1 gives 8.6 % of the
raw bytes in 47 ms, level 3 6.7 % in 161 ms, level 6 6.4 % in 302 ms, level 9 6.2 % in 713 ms.
DXT data compresses far better than the budget above feared (flat kit mains to 0.15 %,
photographic textures to 15 %), and the whole set takes a third of a second at Red's level.
**The level is 6**, Red's: level 3 would save 0.14 s per 40 MB and cost 5 % more bytes in every
CPK the game loads, level 1 0.25 s for 35 % more, and a compile of that export spends seconds
elsewhere. The `zlib-rs` backend is not adopted: it is a new dependency for a cost already
under a second, so it stays a maintainer's call. The Red parity harness compares WESYS-wrapped
entries **decompressed**, since deflate output is not byte-stable across implementations and
levels. Textures already WESYS-wrapped in the export are passed through, not re-compressed
(Red's `zlib_file` skip). With the `auto` default the cost is only ever paid on cup-DLC
compiles, where a few seconds against a 48-team run is irrelevant.

### Path resolution

| Artifact/input | Base location |
|---|---|
| Logs and persistent state | Selected data directory |
| Relative `exports_folder_path` | Executable directory in both data-location modes, so `exports/` sits beside `quick_compile.bat`; created when missing, before a run reads it, so a fresh install has a folder to put exports in. An exports folder named any other way (an absolute setting, the CLI's positional root) is never created: a missing one is refused before the run (`messages.md`, the paragraph after "Output stage and savefile") |
| Relative `output_folder_path` | Executable directory in both data-location modes (`output/` beside the exe, created on demand); holds `.staging/{run_id}/` during a run |
| `overrides/` input (Red's `sideload/`) | Selected data directory (an explicit setting may be added later) |
| Relative `teams_list_path` | Selected data directory — the file is user data once the grid writes ID assignments into it, so it follows the settings file (see `pipeline.md` "Resolved decisions", "Teams list"); nothing is bundled beside the binary |
| `templates/` override directory | Selected data directory; each file shadows the matching embedded template/fallback-bin resource (see `pipeline.md` "Resolved decisions") |
| `pes_folder_path` | Absolute PES installation path (after expanding its documented `**` version placeholder) |
| `savefile_path` | Absolute when explicitly set, a relative one beside the executable as `output_folder_path`; `auto` resolves under the shell's Documents folder → `KONAMI\{game folder}[\{account id}]\save\EDIT00000000` per the Savefile spec's discovery table — independent of `pes_folder_path` |
| Generated CPKs, `test_output/`, `teamnotes.txt` | Resolved `output_folder_path` |
| Installed CPKs | `{pes_folder_path}/download/` |
| Sideload-mode output | `{pes_folder_path}/livecpk/`, the root the game-side runtime serves; no setting (`pipeline.md` "5. Writer", output modes) |

The GUI and CLI use these same bases; a CLI exports-root override changes only that invocation's
resolved export source.

### Dropped settings

| Setting | Why it disappears |
|---|---|
| `cache_clear` | No disk cache: the pipeline is in-memory (each task's files read as owned bytes, `reader/source.rs` `ContentSource`), so there is no equivalent of Red's `patches_contents/` staging folder for the setting to clear |
| `move_cpks` | Deployment is always attempted; a run that cannot deploy degrades to `output/` by itself, and the deliberate "build but don't install" case is the CLI-only `--no-deploy` flag. See "Why no `move_cpks`" under `pipeline.md` "Post-processing" |
| `pause_allow` | The `pause()` pattern is gone; errors/warnings are events rendered by the GUI/CLI. Its unattended-run half (`pause_allow = 0`: finish without stopping) needs no key: a CLI run never stops for input, and the bundled `quick_compile.bat` pauses once, after the run, so its report stays on screen |
| `admin_mode` | Manifest-based UAC / GUI prompt (see core plan) |

### CLI

```text
UTW team-compiler compile [exports-root] [--mode normal|test|sideload] [--export <path>]... [--no-deploy] [--edit-mode-closed]
UTW team-compiler check [exports-root] [--export <path>]...
UTW team-compiler upgrade-dpfl [--yes]      # replace the installed DpFileList with the bundled official one
```

(Refreshing `teams_list.txt` from a savefile is the Save editor's `export-teams-list`; see the
[Save editor plan](../save_editor.md) "CLI".)

`upgrade-dpfl` prints what the override changes (entries that will no longer be loaded, with the
sizes of any matching `.cpk` files found in `download/`) and stops unless `--yes` is given; it never
deletes CPK files non-interactively — those are listed for the user to remove. See "DpFileList
upgrade" under "Post-processing". It prints as the other commands do, one finding per line
(`dpfilelist_cpk_renamed`, `dpfilelist_placeholder_written`, `dpfilelist_replaced`,
`dpfilelist_cpk_dropped`, then `dpfilelist_upgrade_planned` without `--yes`, or
`dpfilelist_up_to_date` alone), in the order `--yes` acts: renames, placeholders, the list.
It exits 0 when it is done or only listed; 2 when the PES folder or its `download/` is not a
folder; 3 when a file cannot be read or written, naming it, after which what was done before
stays done (the list is replaced last, so a failure leaves the old list in place, and running
the command again finishes the work: a file already renamed is no longer under its old name).

The optional positional argument is an exports-root override for that invocation; when omitted, both
commands use the common `exports_folder_path`. It is not a single-export path and is never persisted
back to settings. A relative path given on the command line (the root or an `--export`) resolves
against the current directory, as any command-line path does; only the settings' relative paths
resolve beside the executable or in the data directory ("Path resolution").

`--no-deploy` builds the CPKs into `output/` without touching the PES install or the savefile (see
`pipeline.md` "Post-processing"). It is CLI-only and has no GUI or settings counterpart: it is the cup maintainer's
DLC-production flag and the build-box flag, not something a regular user should find and leave on.
It is rejected together with `--mode test|sideload`, which have no deployment to skip.

`--edit-mode-closed` is the member's word that the compile target's game has its Edit mode
closed: with it the savefile step writes the save even while that PES runs, where without it
the step is skipped (`savefile_skipped_pes_running`; `pipeline.md` "6. Post-processing"). It
is `compile`'s alone (`check` writes no save) and has no settings counterpart: a standing "yes"
would answer a question about the game's state now before it is asked. A run that writes no
save (`--no-deploy`, `--mode test`) accepts it and changes nothing: the member's word about
Edit mode is not wrong there, only unused, and refusing it would break a launcher line that
always passes it.

`--export <path>` (repeatable) restricts the run to the named exports — a path to an export folder
or archive, anywhere on disk, not necessarily under the exports root. A path that does not exist,
or is a file other than a `.zip`/`.7z`, is an invalid invocation (exit code 2, naming the path),
and nothing runs. This is the contract for
**external callers driving a sideloading prototyping loop**, first of all the `pes-models` Blender
extension's "export model and compile for sideloading" button: it saves the model into the player folder
it was loaded from (it knows the export from the launch manifest — see the Player aesthetics editor
plan), then runs

```text
UTW team-compiler compile --mode sideload --export "D:\exports\aaa_export"
```

and PES, with its sideloading runtime serving `livecpk\` (FoxDen on 18–21, Sider 3 on 17), shows
the result on the next model load. `--mode sideload` with `pes_version` 15 or 16 is an invalid
configuration (exit code 2): no sideloading runtime exists for them; so is one whose
`pes_folder_path` is not a folder. Without `--export` the plugin
would recompile every export in the folder on each iteration. Console output follows the ordinary
`-` prefixed format, and the exit code gives a caller a one-line verdict: **0** clean (Warning and
Info findings allowed), **1** finished with an Error finding in some scope (something was dropped,
or kept by `pass_through`), **2** invalid invocation or configuration (clap's own code for the
arguments, and a setting that fails its validation such as a `cpk_name` that is no `CpkStem`;
nothing ran), **3** aborted (an `AbortRun` or Fatal finding, or an environment failure found
before the run, such as an output folder that cannot be written). `check` and `compile` share the
mapping, and deployment adds no code to it: a degraded run (a deployment Error finding, the CPKs
promoted to `output/`) exits 1 and an aborted deployment 3, as the mapping already says, so no
caller special-cases deployment. The CLI is a separate process with its own run
and does not require the GUI to be closed. In sideload mode the bins land at
their game paths inside `livecpk/` ("Output-mode artifact routing" under `pipeline.md` "Resolved
decisions"). A sideload-mode run with PES open skips the savefile step unless given
`--edit-mode-closed` (`savefile_skipped_pes_running`, see `pipeline.md` "Post-processing"): the
model iteration lands through sideloading, and the savefile catches up on a compile given the
flag or made with PES closed.

---
