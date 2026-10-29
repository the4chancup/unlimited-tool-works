# Unlimited Tool Works — Team compiler plan

The Team compiler (`crates/tools/team_compiler`) is the flagship tool of the suite:
the successor to the AET Compiler (Red/Blue). It compiles aesthetics exports into CPK
archives, with compile-time model format conversion (see the
[Model conversion plan](../model_conversion/README.md)) and compile-time aesthetics (on Fox the
output CPK's database tables and the savefile patch; the pre-Fox savefile writing is still
planned — see the [Savefile plan](../pes_savefile/README.md)). Platform context (crate structure, tool
plugin trait, GUI shell, parallelism) is in the [core plan](../core/README.md).

### Relationship to the older compilers

The older compilers are stopgaps, not templates. The original batch-file compiler
was rudimentary; **Red** is a direct translation of it into Python — feature-complete
and battle-tested, but slow at its core (it works by moving files around on
storage); **Blue** is an LLM-made prototype demonstrating that an in-memory,
parallel pipeline is possible. None of them is followed to the letter:

- **The contract is output parity with Red** (still the golden standard): given the
  same inputs, the game-facing CPK contents must match Red's — exact bytes for
  unaffected artifacts and normalized semantic parity for intentional ID/path/layout
  changes (see `testing.md` "Testing: parity against Red"). *How* that output
  is produced is free to change.
- **The implementation is built around the object model** (typed structs
  representing each element of an export — see the [Aesthetics export plan](../aesthetics_export/README.md)), not around Red's
  disk-shuffling stages or Blue's path-string processing functions. References to
  Red/Blue modules in this document identify *where the behavior to reproduce is
  specified*, not code to translate.
- **Warning/error messaging is overhauled**, not ported: the pipeline emits
  structured `PipelineEvent`s (see the core plan) carrying export/folder identity,
  and the GUI/CLI decide presentation. Red's console-text messages serve only as a
  checklist of conditions worth reporting.

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Pipeline](pipeline.md) | Per-export pipeline walkthrough |
| [Message catalog](messages.md) | Message catalog |
| [Settings](settings.md) | Settings |
| [Blue features to port](blue_port.md) | Unimplemented Blue Features to Port |
| [GUI grid](../../plans/team_compiler/gui.md) | GUI: the live-validating team grid; Live validation (no Check or Refresh buttons); Cell states; Toolbar |
| [Testing](testing.md) | Testing: parity against Red (adapted from Blue) |

## Data flow

```
User exports (folders / .zip / .7z)
        │
        ▼
┌──────────────────────────────────────────────┐
│  team_compiler: Reader                       │
│  ─ Scan exports/                             │
│  ─ List each export's structure              │
│    (CanonicalListing) and read its team_name │
│  ─ Route NO_USE, /balls/, duplicate /refs/   │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│  team_compiler: Coordinator                  │
│  ─ Structural validation                    │
│  ─ Team ID resolution                       │
│  ┐ Run-level manifest planning:              │
│  │  ─ Player split + dependency resolution   │
│  └  ─ IDs, targets, collisions, ordering     │
│  ┐ Per task (rayon parallel):                │
│  │  ─ Materialize content under budget       │
│  │  ─ model_convert via IR (if needed)       │
│  │  ─ XML/FMDL/model/name editing            │
│  │  ─ Texture conversion and relocation      │
│  │  ─ contents_packing (CPK/FPK)             │
│  └ ─ Emit packed entries to writer           │
│  ─ bins_update (team colors, uniparam; Fox:  │
│    player rows from resolved settings.toml)  │
│  ┐ Referee processing (spec: Red)            │
│  └ ─ Slot-mapped packing (players.txt)       │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│  team_compiler: Writer (single-threaded)     │
│  ─ Incremental CPK writing                   │
│  ─ Per-task atomicity: staged entries of     │
│    failed tasks are discarded                │
│  ─ Overrides folder injection                │
│  ─ Duplicate invariant check                 │
│  ─ Deploy CPKs to PES folder (or output/)    │
└──────────────────┬───────────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────────┐
│  pes_savefile: Aesthetics patch + savefile   │
│  ─ Resolve the patch from committed outputs  │
│  ─ Write aesthetics_patch.toml to output/    │
│    (Fox: motions, names; the rest in bins/)  │
│  ─ Local save configured: decrypt, apply the │
│    patch, re-encrypt, save (.bak backup)     │
└──────────────────────────────────────────────┘
```

---

## Crate layout

One crate, organized by **pipeline stage**, so a reader who knows the walkthrough below knows where
the code is. The layout is fixed here because the crate is large (~88k lines) and largely
LLM-written: a predefined tree is what keeps generated code from scattering into `utils.rs` piles
and re-inventing placement per session.

```
crates/tools/team_compiler/
├── Cargo.toml
├── help/               # the tool's help chapter (numbered Markdown topics)
│   └── 01_command_line.md
├── src/
│   ├── lib.rs            # `Tool`: the StudioTool impl — wiring only, no logic
│   ├── settings.rs       # TeamCompilerSettings, defaults
│   ├── cli.rs            # clap Command; the parsed command's dispatch (compile, check, upgrade-dpfl)
│   ├── compile.rs        # the `compile` command: deploy preflight, validation, planning, the
│   │                     #   worker pool of tasks, the writer, deployment or promotion
│   ├── check.rs          # the `check` command (the lib validates; this runs it)
│   ├── upgrade.rs        # the `upgrade-dpfl` command
│   ├── gui_run.rs        # the window's compile: a thread of its own, events drained to the log
│   ├── events.rs         # the run's PipelineEvent stream; the worst severity decides the exit code
│   ├── messages.rs       # the message catalog: codes, severity, disposition
│   ├── paths.rs          # per-version CPK paths for both engines, the bins' paths, referee constants
│   ├── templates.rs      # resources bundled from resources/ (include_bytes!) + templates/ overrides
│   ├── reader/           # stage 1: where exports come from
│   │   ├── mod.rs        #   discovery: exports scan, NO_USE, team_name, /refs/ and /balls/ routing
│   │   └── source.rs     #   one source: its listing and revision, lazy file reads
│   │                     #   (ContentSource), memory permit acquisition
│   ├── validation.rs     # the run's validation: structure pass, deep pass, identity, duplicates
│   ├── deep/             # the deep pass: the checks only a file's contents answer
│   │   ├── mod.rs        #   drives the per-export and per-folder checks
│   │   ├── model.rs      #   the format crates' model checks; hand weights
│   │   ├── texture.rs    #   texture checks from the header alone
│   │   ├── materials.rs  #   a .mtl's texture lookup; which materials are mesh-used
│   │   ├── pairings.rs   #   each model paired with the .mtl it binds
│   │   ├── documents.rs  #   face_diff, config.toml, settings.toml, colors.txt
│   │   ├── portrait.rs   #   portrait size; a slot's two portraits compared
│   │   └── collar.rs     #   collar_<ID> names for the target version
│   ├── face_diff.rs      # face_diff.bin / face_diff.xml decode and check
│   ├── face_xml.rs       # the generated pre-Fox face.xml and glove.xml
│   ├── user_face_xml.rs  # a member's own face.xml: parse, resolve, check
│   ├── mtl_search.rs     # the .mtl a pre-Fox .model uses
│   ├── texture_lookup.rs # a texture name resolved nearest first
│   ├── kit_variants.rs   # kitN-dependent assets: kit tokens, lookup keys
│   ├── plan/             # stage 2, the coordinator: ResolvedAestheticsExport → BuildManifest
│   │   ├── mod.rs        #   plan_run → PlanReport; BuildManifest, BuildTask kinds, ModelFolder
│   │   ├── ids.rs        #   PlannedModelIds: 40-ID team blocks, boots/gloves assignment
│   │   ├── roles.rs      #   each file's role in the target engine's output
│   │   ├── collars.rs    #   which export of the run keeps which collar
│   │   ├── item_rows.rs  #   the compiled players' BootsList / GloveList / PlayerAppearance rows
│   │   ├── patch.rs      #   the aesthetics patch's resolved motions and `write_names` names
│   │   └── overrides.rs  #   overrides/ tree precedence
│   ├── processing/       # stages 3–4: parallel per-task work, process_task(BuildTask) → TaskBatch
│   │   ├── mod.rs        #   dispatch by task kind; memory permits; message collection
│   │   ├── model.rs      #   a Fox model package: conversion, names, texture paths, merging, .skl pairing
│   │   ├── conversion.rs #   a model of the other engine's format, via model_convert's IR
│   │   ├── prefox_face.rs    # a player's pre-Fox face: models, .mtl files, face.xml, the face diff
│   │   ├── prefox_shared.rs  # pre-Fox boots/gloves outputs under shared and exclusive IDs
│   │   ├── prefox_common.rs  # the pre-Fox Common output's models and .mtl files
│   │   ├── prefox_split.rs   # the pre-Fox hand auto-split through model_convert
│   │   ├── texture.rs    #   DDS/FTEX conversion, WESYS zlib, relocation to common
│   │   ├── kit.rs        #   kit folders: config generation, colors, textures, FPC patching
│   │   ├── kit_layout.rs #   kit texture and number atlases re-laid between the engines' layouts
│   │   ├── team_assets.rs#   the team's logo: the game's three square PNGs
│   │   ├── referee_marker.rs # the referees' marker: prop texture pre-Fox, reserved collar on Fox
│   │   └── materialize.rs#   THE seam: a task's output becomes the writer's entries (game paths,
│   │                     #   Fox FPK packing; unpacked under test mode)
│   ├── bins/             # the working bins: UniColor / TeamColor and the kit configs
│   │                     #   (UniformParameter entries, or loose files pre-Fox); on Fox also
│   │                     #   BootsList / GloveList / PlayerAppearance
│   │   ├── mod.rs        #   the color bins' records (UniColorBin, TeamColorBin) and WorkingBins
│   │   ├── installed.rs  #   working-bin lookup via the DpFileList walk
│   │   ├── kit_configs.rs#   kit config edits: FPC values and collars
│   │   ├── player_tables.rs # BootsList/GloveList/PlayerAppearance rows
│   │   └── dpfl.rs       #   DpFileList.bin read
│   ├── output/           # stages 5–6: the writer and everything after it
│   │   ├── mod.rs        #   the writer, what happens to its CPK, the notes file
│   │   ├── writer.rs     #   canonical-order incremental writing: overrides, task batches, bins
│   │   ├── parts.rs      #   multi-CPK mode's teams parts and placeholders
│   │   ├── sink.rs       #   OutputSink: CPK (normal) vs loose folder (test/sideload)
│   │   ├── deploy.rs     #   staging, .partial + rename, promotion, writability preflight,
│   │   │                 #   the .cpk.old transaction (commit once the savefile step ends)
│   │   ├── patch.rs      #   aesthetics_patch.toml written to output/ once outputs are in place
│   │   ├── savefile.rs   #   locates the local save, applies the patch to it (via pes_savefile)
│   │   └── teamnotes.rs  #   output/teamnotes.txt
│   ├── view/             # egui — renders state, owns no pipeline logic
│   │   └── mod.rs        #   tool view (settings read-only, Compile button, run log)
│   └── testing.rs        # what the crate's unit tests share (cfg(test))
└── tests/
    ├── cli/              # the command line run in-process, one file per area
    ├── common/mod.rs     #   the sandbox the CLI tests run in
    ├── fixture_save.rs   #   the fixture-save smoke test (pes_savefile's test-support feature)
    ├── parity.rs         # compile + compare against the Fox reference tree, entry by entry
    ├── parity_prefox.rs  # the same, pre-Fox
    └── fixtures/         # small committed exports and parts (tracer, tracer_prefox, sources, …)
```

Placement rules:

- **If it validates an export, it is not here** — it belongs in `libs/aesthetics_export`.
- **If two tools need it, it is not here** — it moves to a lib (`pipeline` for scaffolding such as
  `MemoryBudget` and `CpkStem`; `aesthetics_export` for format knowledge). The same rule
  in reverse: `bins/dpfl.rs` stays here until a second consumer appears (the Balls compiler only
  *checks* the DPFL, via a read helper that can move to `pipeline` when needed).
- **`processing/materialize.rs` is the single output-mode seam.** No task's output asks which
  output mode is active; the mode decides only *where bytes go* — `compile` picks the
  `OutputSink`, the promotion target and whether `overrides/` applies, `output/` writes through
  them — never *what a task produces*.
- **`reader/` is the only module that touches export sources.** `check`/`compile` ask it for the
  current export set; `plan/` receives the resolved exports and `processing/` the files a task
  read through `ContentSource`; nothing else opens a folder or archive.
- **`view/` is render-only.** It reads tool state built from `PipelineEvent`s and emits the user
  intent (compile); it never calls into `processing/` or `output/` directly.
- **Module names follow the walkthrough's stage names** (`reader`, `check`, `plan`, `processing`,
  `bins`, `output`) so the plan and the tree use one vocabulary. `pipeline` is deliberately *not*
  used for a tool-local module: it is the shared lib's name, and `pipeline::MemoryBudget` next to a
  `crate::pipeline::…` would be a permanent source of confusion.
- **Files, not folders, until a module exceeds roughly a thousand lines** — then it becomes a
  folder with a `mod.rs` and the same name, so paths in this plan stay valid.
- No `utils.rs`, `helpers.rs`, or `misc.rs`. A helper belongs to the stage that uses it; one used by
  several stages belongs to `pipeline`.

---

## Object model and export format

The export object model, its validation progression (`ParsedAestheticsExport` → `ValidatedAestheticsExport` →
`ResolvedAestheticsExport`), the `aesthetics_export` crate layout, the player-folder format, `settings.toml` in
exports, and the FPC marker files are specified in the [Aesthetics export plan](../aesthetics_export/README.md). The
compiler consumes them; only the compile-time behavior — the loaded processing types
(`ModelFolder` and friends, with their conversion and packing methods), the `BuildManifest`,
planning, processing, packing, bins, GUI, and CLI — lives in this crate and this plan.

---

## Development phases

- **Phase 3 — skeleton and shared export model (done 2026-10-02):** `libs/aesthetics_export`'s
  structure parsing and format-level validation (on the `ExportIdentity` boundary, the sanitized
  validated-versus-eligible projection and the roster-entry semantics of "Validation semantics"
  in the [Aesthetics export plan](../aesthetics_export/object_model.md)), the Team compiler
  settings/CLI surface, the reader/coordinator/writer over `libs/pipeline` compiling the subset
  in "Phase 3 scope" below, and the shell slice's view.
- **Phase 4 — processing logic (done 2026-10-12):** the deep pass, model and non-model
  processing on both engines (cross-format conversion through the Phase 2 `model_convert`),
  packing, bins, referees, the
  output modes and deployment, on synthetic Studio-format fixtures, with planning fixing every
  output name and every model task's planned IDs before processing; what `compile` builds is
  "Phase 4 scope" below. Its parity with Red covers the tracer export (Fox) and the pre-Fox
  reference (`testing.md`); the full Red parity gate over upgraded old exports depends on
  Phase 6's Export upgrader (and on Phase 5's savefile and ID integration), so it runs once
  those phases land.
- **Phase 5 — savefile integration (done 2026-10-16):** on Fox, every compiled player's
  `PlayerAppearance.bin`, `BootsList.bin` and `GloveList.bin` rows from his resolved
  `settings.toml`, the default-ID rows, the Midcup shared-IDs record, and the aesthetics patch,
  written beside the output and applied to the local save in one transaction with the CPKs;
  what `compile` builds is "Phase 5 scope" below. The pre-Fox patch is deferred with
  pre-Fox's other gaps.

This documents the dependency without changing the phase order in the core plan.

---

## Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". IDs are never reused; later phases append.
In every phase's scenarios, a team export is a `Full` export, its name carrying the tag,
unless the scenario says `Midcup` (the Aesthetics export plan's "Coverage tag"), and a
scenario that names no PES version runs for PES 21. A GIVEN names the files that decide its
outcome; a folder or export it names also holds the least ordinary content its THEN needs
to be reachable (a face model in a player folder whose face compiles, a model in a shared
folder, a `kit.dds` in a kit whose texture is used, a kit in an export whose bin entries
are observed), as the cited test sets up.

**Phase 3 scope.** Phase 3 delivered discovery and export sources, `aesthetics_export`'s
structure pass with its dispositions ("Validation semantics" in the Aesthetics export plan),
export identity, the settings and CLI surface, the pipeline scaffolding compiling the tracer
bullet's content (Fox player folders with face models, and kits) into a CPK, and the shell
slice. Until step 4.20 a subset gate skipped, with the Error `content_not_yet_compiled`, every
export holding anything `compile` could not build yet, so that no CPK differing from the
finished compiler's was written; the gate is withdrawn with its finding (TC-OUT-06), and every
export of either engine reaches processing whatever it holds. What becomes of a file is its
role's: a file no package reads is not read and is reported `file_not_used` (`pipeline.md`
"2. Per-export serial steps", the face-file paragraph), the one refusal left being a model
folder whose selected representation is glTF (`model_gltf_unsupported`, Phase 7). A refs export
plans no colors record (its `colors.txt` is not read), no `team_colors_missing`, no kits and no
`BootsList.bin`/`GloveList.bin` rows: the game's referee hook loads slot NN's `k99NN`/`g99NN`
by number, and the referees' kits are the template tree's (`blue_port.md` "Referee export
processing"), so a refs export's kits, logo, portraits and collars are `file_not_used`.
Findings are observed
in `check`'s and `compile`'s console output, one line per finding naming its code and scope; the
compiled CPK's content is the parity test's (`testing.md`), not a scenario's. Of the events,
Phase 3 emits `ExportStarted`, `Message` and `ExportProcessed` only: `FolderStatus`, `Progress`
and `Complete` wait for their consumers, the progress grid and the run strip (Phase 8), which
is also when `Complete`'s placeholder fields are restructured ("Event system" in
`core/architecture.md`), so a cell's outcome (`DoneWithErrors`, "Cell states" in `gui.md`) is
not one of these scenarios.

**Sources**

```
TC-SRC-01  GIVEN an exports root holding the same export, with a players.txt and an empty kit
           folder p2/, as a folder, a .zip and a solid .7z
           WHEN the root is checked, then each source is compiled alone
           THEN three exports are reported, each with the same findings, the .7z's roster read;
                the three compiled CPKs hold the same entries with the same contents, each with
                p2's placeholder texture and config, and the same UniColor entry for p2 (TC-BIN-02)
TC-SRC-02  GIVEN an export folder "co - Spring" and an archive "co - Spring.zip"
           WHEN the root is checked
           THEN they are two exports, and every finding names the source it is about
TC-SRC-03  GIVEN an export whose root holds NO_USE or NO_USE.txt
           WHEN the root is checked or compiled
           THEN export_disabled is reported and nothing else about that export, and it is not compiled
TC-SRC-04  GIVEN an export whose name's first word is "balls"
           WHEN the root is checked or compiled
           THEN export_balls_skipped is reported and the export is neither validated nor compiled
TC-SRC-05  GIVEN two non-disabled exports whose first word is "refs", and a third one disabled
           WHEN the root is checked
           THEN multiple_ref_exports is reported for each of the two, neither is validated, and
                the disabled one reports only export_disabled
TC-SRC-06  GIVEN a corrupt .zip beside a valid export
           WHEN the root is checked or compiled
           THEN export_extract_failed is reported for the .zip and the valid export is checked
                or compiled
TC-SRC-07  GIVEN a .zip holding two entries whose names differ only in case, or an entry "../x"
           WHEN the root is checked
           THEN export_extract_failed names the path, and the export is skipped
TC-SRC-08  GIVEN --export given twice, naming an export outside the exports root and one inside
           it, beside a third export in the root
           WHEN check runs, then compile
           THEN each reports only the two named exports, and compile compiles only those two
TC-SRC-09  GIVEN any export, one of them with its content nested one folder down
           WHEN it is checked and then compiled
           THEN every file of every source is byte-identical to before, and no file was added
TC-SRC-10  GIVEN exports "co Midcup One", "dbg Midcup Two.7z" and "egg Midcup Three"
           WHEN the root is checked
           THEN each export's findings are printed together, the exports in the order of their
                source names whatever their kind
```

**Structure**

```
TC-STR-01  GIVEN an export whose root holds only notes.txt and a folder wrapper/ holding Players/
           WHEN it is compiled
           THEN nested_folders_fixed is reported, it compiles as if wrapper/'s content were at the
                root, and notes.txt is kept as the export's root notes (notes_found); an export
                holding Players/Players/03 - A/ likewise reports nested_folders_fixed and compiles
                A as player 03
TC-STR-02  GIVEN an export with two nested child folders that each hold usable content
           WHEN it is checked
           THEN nested_root_ambiguous is reported and the export is skipped
TC-STR-03  GIVEN a loose root file at the same path as a file of the one nested root
           WHEN it is checked
           THEN nested_root_conflict is reported and the export is skipped
TC-STR-04  GIVEN an export with no usable content
           WHEN it is checked
           THEN export_empty is reported and the export is skipped
TC-STR-05  GIVEN a player folder with the link file Crocs.boots, or Crocs.boots.txt, and Boots/Crocs/
           WHEN it is checked
           THEN the link resolves and no finding is reported for it
TC-STR-06  GIVEN a player folder linking a shared folder that does not exist
           WHEN it is checked
           THEN link_target_missing is reported and that player folder is dropped
TC-STR-07  GIVEN a player folder with two .boots links
           WHEN it is checked
           THEN shared_link_duplicate is reported and that player folder is dropped
TC-STR-08  GIVEN a shared folder no link file names, and one linked only by a player folder that
           players.txt does not list
           WHEN it is checked
           THEN shared_folder_orphaned is reported for both, and neither is compiled
TC-STR-09  GIVEN a player folder holding a file outside the allowlist
           WHEN it is checked with strict_file_type_check on, then off
           THEN file_type_disallowed is an Error dropping the folder, then an Info keeping it
TC-STR-10  withdrawn: restated as TC-STR-20 (`spec/team_compiler/README.md`), the
           marker ingame_face.toml since step 0.0.1h and its old spellings reported
TC-STR-11  GIVEN an ingame_face.toml player folder that also holds face_high.fmdl, or a .face link
           WHEN it is checked
           THEN ingame_face_explicit_face_model is reported and that player folder is dropped
TC-STR-12  GIVEN a player folder holding both fpc_on and fpc_off
           WHEN it is checked
           THEN fpc_conflict is reported and that player folder is dropped
TC-STR-13  GIVEN a player folder holding hair.dds and hair.png
           WHEN it is checked
           THEN texture_stem_conflict is reported and the player folder is dropped (a subfolder's
                hair.dds is no second hair of the root's: a subfolder is a folder of its own,
                TC-MOD-66)
TC-STR-14  GIVEN ingame_face.toml player folders holding, in turn, parts/boots.fmdl with
           parts/glove_l.fmdl and parts/skin.dds; parts/hair_high.fmdl; and common/body.fmdl
           with common/extra/readme.txt
           WHEN they are checked
           THEN the first reports no finding (each model takes its role from its name, wherever
                it sits); the second reports ingame_face_explicit_face_model and is dropped; the
                third reports file_type_disallowed for readme.txt alone and is dropped
TC-STR-15  GIVEN player folders linking Common/torso.fmdl as torso.fmdl.common and as
           torso.fmdl.common.txt, and a third linking missing.fmdl.common
           WHEN the export is checked
           THEN the first two links resolve with no finding; the third reports common_link_missing
                naming the Common path, and only that player folder is dropped
TC-STR-16  GIVEN strict_file_type_check on, Faces/Base/ holding a disallowed file, and a player
           folder linking it with Base.face
           WHEN the export is checked, then with pass_through on
           THEN file_type_disallowed drops Base and link_target_dropped drops the player folder;
                with pass_through both are kept and only file_type_disallowed is reported; with
                pass_through and Base holding hair.dds and hair.png instead, texture_stem_conflict
                and link_target_dropped still drop both
TC-STR-17  GIVEN a Fox target, Boots/Crocs/ holding torso.fmdl and Boots/Mud/ holding kit_boots.fmdl,
           each linked by a player, and a player folder holding torso.fmdl
           WHEN the export is checked
           THEN fmdl_name_invalid is reported for Crocs, which is dropped with its linking player
                (link_target_dropped); Mud, its player and the player holding torso.fmdl (face
                content by the name table) report no Warning or Error
TC-STR-18  GIVEN /co/ slot 05 holding an ingame_face.toml that does not parse, slot 06 one
                 holding an unknown key, each beside a settings.toml setting skin_color = 4,
                 and no model
           WHEN the export is checked, then compiled for PES 21
           THEN both runs report ingame_face_toml_invalid for each, naming the file and the
                error; the CPK holds no face/real/71405/ or 71406/ path, and both players'
                PlayerAppearance rows hold skin colour 4 and the template's defaults in every
                face, hair and iris field
TC-STR-19  GIVEN /co/ slot 05 holding a settings.toml with [appearance.hair] and
                 skin_color = 4, and an ingame_face.toml setting iris_color = 7, and no model
           WHEN the export is checked, then compiled for PES 21
           THEN both runs report settings_toml_invalid naming appearance.hair as an unknown
                key, and 71405's row holds iris 7 and the template's skin colour, 1
TC-STR-20  GIVEN /co/ slots 05, 06 and 07 holding, in turn, ingame_face.toml, a bare
                 ingame_face and ingame_face.txt, and no model
           WHEN the export is checked, then compiled for PES 21
           THEN both runs report ingame_face_marker_renamed for 06's and 07's files and
                neither reports a disallowed file nor a finding on 05; the CPK holds no
                face/real/71405/ path and a blank face package for 71406 and 71407, as for
                a folder with no marker
```

**Roster**

```
TC-ROS-01  GIVEN no players.txt and player folders "03 - A" and "15 - B"
           WHEN the export is compiled
           THEN A is compiled as player 03 and B as player 15
TC-ROS-02  GIVEN no players.txt and player folders "24 - C" and "Snuffy"
           WHEN the export is checked
           THEN player_folder_number_invalid is reported for each and both are dropped
TC-ROS-03  GIVEN no players.txt and player folders "05 - A" and "05 - B"
           WHEN the export is checked
           THEN player_number_duplicate is reported for each and both are dropped
TC-ROS-04  GIVEN players.txt "03 snuffy", a folder "Snuffy" and a folder "15 - B"
           WHEN the export is compiled
           THEN Snuffy is player 03, and B reports player_unlisted and is not compiled
TC-ROS-05  GIVEN players.txt listing one folder under slots 03 and 07
           WHEN the export is compiled
           THEN that folder is compiled for both slots
TC-ROS-06  GIVEN players.txt lines "x3 A", "24 B", "05 Nobody" and "07 C", and folders A, B and C
           WHEN the export is compiled
           THEN the first three lines report players_txt_line_invalid, players_txt_slot_invalid
                and players_txt_target_missing, each naming its line; A also reports
                player_unlisted, B does not; C alone is compiled, as player 07
TC-ROS-07  GIVEN players.txt lines "03 OldName" and "03 NewName", with only a folder NewName
           WHEN the export is checked
           THEN players_txt_slot_duplicate is reported on the second line and
                players_txt_target_missing on the first, and the export is skipped
TC-ROS-08  GIVEN players.txt that is not UTF-8, and another with a BOM and CRLF line ends
           WHEN each export is checked
           THEN the first reports players_txt_invalid and is skipped; the second reads normally
TC-ROS-09  GIVEN a normal team with an empty players.txt, a player folder and a kit
           WHEN the export is compiled
           THEN the kit is compiled, and the player folder reports player_unlisted
TC-ROS-10  GIVEN a refs export with no players.txt and no refs.txt, its folders numbered
           WHEN it is checked
           THEN players_txt_missing is reported and the export is skipped
TC-ROS-11  GIVEN a refs export with refs.txt only, and another with refs.txt and players.txt
           WHEN each is checked
           THEN the first reads refs.txt as its roster; the second reports refs_txt_ignored and
                reads players.txt
TC-ROS-12  GIVEN a refs export whose players.txt maps one folder to slots 01, 20 and 35, and one
           whose every line is dropped
           WHEN each is checked
           THEN the first is valid with three slots; the second reports players_txt_invalid and
                is skipped
```

**Identity**

```
TC-ID-01   GIVEN "co Full Spring 2026.zip" and a teams list with the row 714 /co/
           WHEN it is compiled
           THEN the export is reported as /co/ (714)
TC-ID-02   GIVEN an export whose first word is not in the teams list, beside a valid export
           WHEN the root is compiled
           THEN team_name_unknown is reported, that export is skipped, the other is compiled,
                and the exit code is 1
TC-ID-03   GIVEN a refs export and a teams list with no /refs/ row
           WHEN it is checked
           THEN no team_name_unknown is reported and the export is reported as referees
TC-ID-04   GIVEN an export named "--- .zip"
           WHEN it is checked
           THEN team_name_unknown is reported with an empty name and the export is skipped
TC-ID-05   GIVEN exports "co Spring 2026" and "a Full Spring 2026"
           WHEN the root is checked, then compiled
           THEN both commands report export_tag_missing for the first, naming it; it is skipped,
                the other is compiled, and the exit code is 1
TC-ID-06   GIVEN exports "co midcup day 5", "a FULL v2" and "b Spring Full"
           WHEN the root is checked
           THEN the first two report no export_tag_missing (letter case does not matter) and
                the third does (only the second word counts)
TC-ID-07   GIVEN a refs export named "refs Spring 2026"
           WHEN it is checked
           THEN no export_tag_missing is reported
```

**Kits and root files**

```
TC-KIT-01  GIVEN kit folders "p1 - Lakers" and "g1"
           WHEN the export is compiled
           THEN they are compiled as kits p1 and g1
TC-KIT-02  GIVEN kit folders "p1" and "p1 - Lakers"
           WHEN the export is checked
           THEN kit_slot_duplicate is reported and both are dropped
TC-KIT-03  GIVEN kit folders "p10", "x1" and "home"
           WHEN the export is checked
           THEN kit_folder_invalid is reported for each and each is dropped
TC-KIT-04  GIVEN all/ holding kit_back.dds and kit_name.dds, and p2/ holding its own kit_name.dds
           WHEN the export is checked
           THEN kit_textures_inherited lists only back for p2
TC-KIT-05  GIVEN all/ holding config.toml, and in another export all/ with no kit folder
           WHEN each is checked
           THEN the first reports kit_all_file_ignored, the second kit_all_unused
TC-KIT-06  GIVEN a kit folder holding both the pre-fox and fox markers
           WHEN the export is checked
           THEN kit_layout_conflict is reported and the kit is dropped
TC-KIT-07  GIVEN a kit folder holding back.dds
           WHEN the export is checked
           THEN kit_texture_name_invalid is reported and only that file is dropped
TC-KIT-08  GIVEN a kit folder holding an icon_25 marker
           WHEN the export is checked
           THEN kit_icon_invalid is reported and the kit is kept
TC-KIT-09  GIVEN p1/ holding kit.png and kit.dds; then all/ holding kit_back.png and kit_back.dds
           beside p2/; then all/kit_name.dds and p3/kit_name.png
           WHEN each export is checked
           THEN the first reports texture_stem_conflict and drops p1; the second reports it and
                drops all/, p2 inheriting nothing; the third reports no finding at all (no
                texture_stem_conflict, no kit_textures_inherited for p3)
TC-ROOT-01 GIVEN root files extra.bin and readme.TXT, a root folder wrapper/ beside Players/, a
           file Players/players.txt, and a refs export's ref_lists.txt
           WHEN each export is checked
           THEN root_file_unexpected is reported for extra.bin, wrapper/ and Players/players.txt
                only, each ignored
TC-ROOT-02 GIVEN a root notes.txt that is not UTF-8, and pass_through on
           WHEN the export is compiled
           THEN notes_encoding_invalid is reported, the note is dropped, and the export compiles
TC-ROOT-03 GIVEN root logo_zoom.png; then logo.png and logo.dds; then logo_small.png alone; then
           logo.png and logo_small.png
           WHEN each export is checked
           THEN the first three report logo_file_invalid, logo_role_duplicate and
                logo_small_without_main, each export otherwise kept; the fourth reports none
TC-ROOT-04 GIVEN Portraits/24.dds
           WHEN the export is checked
           THEN portrait_name_invalid is reported and only that file is dropped
TC-ROOT-05 GIVEN the tracer bullet's export with a root notes.txt that cannot be read
           WHEN it is compiled
           THEN source_read_failed is reported for notes.txt only, the rest compiles, and the
                exit code is 1
```

**Dispositions**

```
TC-DSP-01  GIVEN an export with one valid player folder and one dropped by link_target_missing
           WHEN it is compiled
           THEN the CPK holds the valid player's content only, and the exit code is 1
TC-DSP-02  GIVEN the same export with pass_through on
           WHEN it is compiled
           THEN both players are compiled, link_target_missing is still reported as an Error,
                and the exit code is 1
TC-DSP-03  GIVEN pass_through on and exports with players_txt_slot_duplicate, players_txt_slot_invalid,
           shared_folder_orphaned, fpc_conflict, kit_layout_conflict, shared_link_duplicate,
           kit_slot_duplicate, kit_folder_invalid, face_diff_invalid, xml_dif_conflict or
           kit_config_invalid
           WHEN they are compiled
           THEN each finding keeps its drop: the export, the assignment, the shared folder, the
                player folder, the kit (both kits, for kit_slot_duplicate)
TC-DSP-04  GIVEN an export skipped by players_txt_slot_duplicate beside a valid export
           WHEN the root is compiled
           THEN the valid export is compiled as it is when compiled alone
```

**CLI and output**

```
TC-CLI-01  GIVEN an exports root, an existing settings file and teams list
           WHEN check runs
           THEN nothing is written to the output folder or into any export, and the settings
                file and teams list are byte-identical to before (logs may be written)
TC-CLI-02  GIVEN exports with, in turn, only Info and Warning findings, an Error finding, and none
           WHEN each is checked
           THEN the exit codes are 0, 1 and 0
TC-CLI-03  GIVEN compile --no-deploy --mode test
           WHEN it runs
           THEN it is refused as an invalid invocation naming --no-deploy and --mode as
                incompatible, with exit code 2, and nothing runs
TC-CLI-04  GIVEN compile with a positional exports root
           WHEN it runs
           THEN that root is compiled and the settings file's exports_folder_path is unchanged
TC-CLI-05  GIVEN an invalid setting value: a cpk_name or bins_cpk_name that is not a valid
           CpkStem (empty, "con", "a/b"), or a value of the wrong type or out of range
           WHEN check or compile starts
           THEN it is refused before any export is read, naming the setting, with exit code 2
TC-CLI-06  GIVEN an output_folder_path that cannot be created or written
           WHEN compile starts
           THEN it is refused before any export is read, naming the path, with exit code 3
TC-CLI-07  GIVEN --export naming a path that does not exist
           WHEN check or compile runs
           THEN it is refused as an invalid invocation naming the path, with exit code 2
TC-CLI-08  GIVEN an absolute exports_folder_path naming a folder that does not exist
           WHEN check or compile runs
           THEN it is refused before any export is read with the message messages.md gives,
                naming the path and the settings file, no OS error text, and exit code 2
TC-CLI-09  GIVEN the default relative exports_folder_path and no exports folder beside the
           executable
           WHEN check or compile runs
           THEN the folder is created, no_exports_found is reported naming it, nothing is
                written to the output folder, and the exit code is 0
TC-CLI-10  GIVEN pes_folder_path '<dir>/PES20**', pes_version 21, and the install at <dir>/PES2021
           WHEN compile runs
           THEN the installed bins are read from <dir>/PES2021: ** stands for the version's two
                digits
TC-CLI-11  GIVEN a root holding an export with a kit whose task fails, and a disabled export
           WHEN check or compile runs
           THEN each export's events are ExportStarted, its findings, then ExportProcessed, sent
                after its last task's finding, or right after planning for an export that plans
                no task
TC-CLI-12  GIVEN no teams_list.txt in the data directory, or one that does not parse
           WHEN check or compile runs
           THEN a missing list is written from the bundled one by compile (check writes nothing);
                one that does not parse is refused naming teams_list.txt with exit code 3
TC-OUT-01  GIVEN the tracer bullet's export
           WHEN compile --no-deploy runs
           THEN <output_folder_path>/<cpk_name>.cpk is written, the PES folder and the savefile
                are untouched, and the exit code is 0
TC-OUT-02  GIVEN a root whose every export is skipped
           WHEN it is compiled
           THEN no CPK is written, deployed or promoted, and a previous <cpk_name>.cpk in output/
                or in download/ is left as it was
TC-OUT-03  GIVEN a previous <cpk_name>.cpk (in test or sideload mode, test_output/ or livecpk/)
           and a run whose CPK writing fails
           WHEN it is compiled
           THEN cpk_write_failed is reported, the exit code is 3, no partial CPK remains, and the
                previous output is byte-identical to before
TC-OUT-04  GIVEN a previous <cpk_name>.cpk and one export whose only player folder reports
           link_target_missing, pass_through off
           WHEN it is compiled
           THEN no CPK is written, the previous CPK is byte-identical to before, and the exit
                code is 1
TC-OUT-05  GIVEN a previous <cpk_name>.cpk that another process holds open without delete sharing
           WHEN a run writes its new CPK completely and then replaces the previous one
           THEN output_commit_failed is reported, the exit code is 3, the previous CPK is
                byte-identical to before, and no partial output is left beside it
TC-OUT-06  withdrawn: the subset gate it tested went at step 4.20; a gloves model naming no hand
           is TC-MOD-53's
```

**Shell slice**

```
TC-GUI-01  manual  GIVEN the studio binary
                   WHEN it is launched with no arguments
                   THEN a window titled Unlimited Tool Works opens, whose sidebar lists the registered tools
TC-GUI-02  GIVEN the tracer bullet's export in the exports folder
           WHEN Compile is clicked
           THEN the run is the CLI's `compile` on the effective settings (what it writes and
                where is TC-DEP's and TC-OUT's), and the log shows the run's findings as the
                console prints them (the export's export_identified line among them), then the
                run's exit code
TC-GUI-03  GIVEN the shell with registered tools
           WHEN a tool's sidebar entry is clicked
           THEN that entry is marked active and its view is the one drawn; the Team compiler's
                view shows its settings section, collapsed, opening on the effective settings
                as `key = value` lines, a Compile button enabled while no run is in flight and
                disabled while one is, and the run log, which follows its newest line
```

Only the first needs a display (`run_gui` opens the window); the rest of the Phase 3 slice is
proven headless (`egui::Context::run_ui`, the frame's AccessKit nodes).

**Phase 4 scope.** `compile` builds everything a validated export holds, for every target version
(PES 15-21) and for referee exports, into the normal CPK, the test tree or the sideload tree, and
deploys the CPKs into the PES install, degrading to `output/` when it cannot; `check` runs the
structure pass and the deep pass on every source kind. The run plans IDs (the 40-ID blocks;
`duplicate_aesthetics_export` is the validation pass's, `duplicate_path` the writer's), reads the
working bins from the installed CPKs and writes `UniColor.bin` and `TeamColor.bin` and, on Fox,
`UniformParameter.bin` and the `BootsList.bin` and `GloveList.bin` rows for the compiled players whose custom boots or gloves
committed, a failed output keeping the installed row, a compiled player of a `Full`
export whose folder holds no boots or no gloves losing that installed row, every other row kept,
`PlayerAppearance.bin` passing through unchanged until Phase 5
builds its rows from `settings.toml`. `content_not_yet_compiled` is withdrawn (step 4.20). What
`compile` still refuses: a model folder whose selected representation is glTF, dropped with
`model_gltf_unsupported` until Phase 7, never compiled from a native source beside it. Not in
Phase 4, so not here: the savefile step and the aesthetics patch (Phase 5; `compile` emits no
`savefile_*` or `patch_written` finding and a configured savefile is left byte-identical),
`run_pes`, `FolderStatus`/`Progress`/`Complete`, live validation and the GUI's degraded-run
actions (Phase 8), the in-game effect of a `GloveList.bin` row (FoxDen's gloves patch, worklog
"Issues") and of a sideload tree (FoxDen's LiveCPK gaps). The areas `PRT`, `MOD`, `TEX`, `CHK`,
`XML`, `CMN`, `BIN`, `PLN`, `REF` and `DEP` are new; `KIT`, `ROOT` and `OUT` continue their
numbering. Scenarios observe the console findings, the exit code, the files of the output tree or
of the CPK (read back with the `cpk` crate); byte parity with Red is the parity test's. Test
data: `/co/` is team 714, so its 40-ID block is 621-660, slot 05 is player 71405 with exclusive
boots/gloves ID 625 and the first shared ID is 644; `/egg/` is 792 (the tracer fixture); `/a/` is
702 and `/b/` 707.

**Portraits**

```
TC-PRT-01  GIVEN a /co/ export whose slot 05 folder holds portrait.dds, and
           Portraits/07.png or .dds
           WHEN it is compiled for PES 21, then for PES 18
           THEN the CPK holds common/render/symbol/player/71405.dds and 71407.dds, then
                player_71405.dds and player_71407.dds, each a DDS, each .dds source's
                byte-identical to it
TC-PRT-02  GIVEN slot 05's folder holding portrait.dds and Portraits/05.dds with different
           contents, and another export where the two files are byte-identical
           WHEN each is compiled
           THEN the first reports portrait_conflict and is skipped; the second compiles with one
                71405.dds and no portrait finding
TC-PRT-03  GIVEN Portraits/05.dds whose side is 300 pixels
           WHEN the export is checked or compiled
           THEN texture_not_pow2 is reported and only that file is dropped (kept, with
                pass_through on)
TC-PRT-04  GIVEN slot 05's folder holding portrait.dds, a 128x128 single-level BC3 DDS under a
           DX10 header with DXGI format 78 (BC3 sRGB), the shape that crashed PES 19
           WHEN the export is compiled for PES 21, then for PES 17
           THEN 71405.dds, then player_71405.dds, holds the source's pixel data under the legacy
                DXT5 header (ftex::dds::header_bytes(Bc3, 128, 128, 1)), portrait_header_rewritten
                is reported as an Info naming the file with dxgi=78, and nothing is dropped
TC-PRT-05  GIVEN slot 05's folder holding portrait.dds, a 128x128 BC7 DDS under a DX10 header
           WHEN the export is compiled for PES 18, then for PES 21
           THEN player_71405.dds is a 128x128 DXT5 DDS under the legacy header with the full mip
                chain and no portrait_header_rewritten; then 71405.dds holds the source's BC7
                blocks re-headered, as TC-PRT-04's do
```

**Models**

```
TC-MOD-01  GIVEN a /co/ export whose slot 05 folder holds kit_boots.fmdl, glove_l.fmdl and
           glove_r.fmdl beside its face models and textures
           WHEN it is compiled for PES 21
           THEN the CPK holds Asset/model/character/boots/k0625/#Win/boots.fpk and .fpkd and
                glove/g0625/#Win/glove.fpk, the boots package holding boots.fmdl and boots.skl, the
                gloves package glove_l.fmdl and glove_r.fmdl, and every texture of the folder once
                under common/714/05 - <name>/sourceimages/
TC-MOD-02  GIVEN slot 05 holding kit_boots.fmdl and kit_boots.skl (or linking
           kit_boots.fmdl.common, Common/ holding both), and another export whose slot 05
           holds kit_boots.fmdl alone
           WHEN each is compiled for PES 21, the version the model is posed for
           THEN boots.skl in the boots package is byte-identical to kit_boots.skl; without the
                .skl it is byte-identical to the bundled PES 21 body.skl
TC-MOD-03  GIVEN a players.txt mapping one folder with boots.fmdl and a texture to slots 03 and 07
           WHEN the export is compiled
           THEN k0623 and k0627 both hold the boots package, and the folder's textures appear once,
                under the folder's name
TC-MOD-04  GIVEN slot 05 holding a face model, skin.dds and kit_boots.fmdl, and linking Crocs.boots,
           whose folder holds boots.fmdl, sole.dds and skin.dds: once with the same bytes, once
           with different ones
           WHEN each is compiled
           THEN the first packs skin once with no texture finding; the second reports
                shared_texture_overridden naming skin and Boots/Crocs, keeps the face, k0625
                and sole.ftex, and the skin.ftex packed is the player's
TC-MOD-05  GIVEN Boots/Crocs/ and Boots/Mud/ each holding boots.fmdl and a texture, Crocs linked by
           slots 03 and 07, Mud by slot 11
           WHEN the export is compiled for PES 21
           THEN k0644, the block's first shared ID, holds Crocs, and k0631, slot 11's own ID
                (Mud's one wearer), holds Mud, each once with its texture under its own
                k06NN/#windx11/ and its model's path naming it there, and no k0623 or k0627
                folder exists
TC-MOD-06  GIVEN an installed CPK below the output whose /co/ record names seventeen boots
                folders, 644 to 660, and a Midcup /co/ export holding Boots/Birk, which the
                record does not name, linked by slots 05 and 06
           WHEN the export is checked
           THEN boots_id_pool_exhausted is reported with the count 18 and the export is
                skipped (a Full export cannot exhaust the pool: with 23 slots at most 11 folders
                have two wearers)
TC-MOD-94  GIVEN a /co/ export whose slots 01 to 20 each link their own shared boots folder,
                Boots/B01 to Boots/B20, each holding boots.model and boots.mtl; slots 01 to 15
                holding ingame_face.toml and nothing else, slots 16 to 20 face_high.model and
                face_high.mtl
           WHEN it is compiled for PES 17
           THEN no boots_id_pool_exhausted is reported, boots/k0621/ to k0640/ each hold the
                boots model of the folder slot NN links (k0620+NN), no boots/k0644/ folder
                exists, no face CPK is written for slots 01 to 15 (71401 to 71415), and slots
                16 to 20 each get theirs
TC-MOD-07  GIVEN Boots/Crocs/ holding boots.fmdl and sole.dds, linked by slot 05, which also holds
           kit_boots.fmdl; then slot 07 also linking Crocs.boots plainly
           WHEN the export is compiled for PES 21 each time
           THEN link_combined and fmdl_merged are reported for slot 05, k0625 holds one boots.fmdl
                whose mesh count is Crocs's plus the local model's, sole.ftex sits under slot
                05's texture folder, and no folder holds Crocs alone in the first run, while in
                the second k0627, slot 07's own ID (Crocs's one plain wearer), holds Crocs alone,
                with its own copy of sole.ftex, and no k0644 folder exists
TC-MOD-08  GIVEN Faces/Longhair/ holding hair_high.fmdl, linked by slot 05, which holds face_high.fmdl
           WHEN the export is compiled for PES 21
           THEN slot 05's face package holds face_high.fmdl and hair_high.fmdl, and no output
                exists for Longhair on its own
TC-MOD-09  GIVEN a folder holding a texture and two parts merged into one boots.fmdl, one paired
           with a custom .skl and the other with none; and another folder holding a texture and
           two parts that define material "skin" differently
           WHEN each export is compiled
           THEN the first reports skl_merge_conflict and the second merge_material_conflict, and
                each folder's boots are left out of the CPK, its blank face and its textures
                still packed
TC-MOD-10  GIVEN slot 05 holding torso.fmdl, legs.fmdl.common and Common/legs.fmdl
           WHEN the export is compiled for PES 21
           THEN fmdl_fcl_hair_fallback is reported for torso.fmdl and the linked legs.fmdl,
                fmdl_merged for the folder, the
                face package holds one fcl_hair.fmdl whose mesh count is the sum, and no
                legs.fmdl.common or Common model path is in the CPK
TC-MOD-11  GIVEN Common/legs.fmdl referencing cloth.dds, which sits in Common/
           WHEN TC-MOD-10's export is compiled
           THEN cloth.ftex sits once at Asset/model/character/common/714/sourceimages/#windx11/
                and the merged FMDL's path table names
                /Assets/pes16/model/character/common/714/sourceimages/, not the player's subfolder
TC-MOD-12  GIVEN slot 05 holding fcl_hair.fmdl only (no face_diff.bin, no fcl_hair_sim.fclo,
           no .skl)
           WHEN the export is compiled for PES 21
           THEN the face package holds fcl_hair.fmdl, face_diff.bin, fcl_hair_sim.fclo and
                fcl_hair_sim.skl, the last three byte-identical to the bundled templates (the
                face_diff.bin: or to the data directory's templates/face_diff.bin)
TC-MOD-13  GIVEN slot 05 holding face_high.fmdl and face_high.skl
           WHEN the export is checked, then compiled, for PES 21
           THEN skl_no_slot is reported and the folder is kept, the .skl not emitted; checked
                for PES 17, neither skl_no_slot nor fmdl_fcl_hair_fallback is reported
TC-MOD-14  GIVEN slot 05 holding ingame_face.toml, parts/boots.fmdl and parts/skin.dds
           WHEN the export is compiled for PES 21
           THEN k0625 holds boots.fmdl (the model typed by its name, wherever it sits)
TC-MOD-15  GIVEN slot 05 holding a face model and face_diff.xml, and another holding a face model
           and a face_diff.xml whose base64 payload is corrupt
           WHEN each export is compiled for PES 21
           THEN the first's face package holds a face_diff.bin decoded from the xml; the second
                reports face_diff_invalid and the folder is dropped
TC-MOD-16  GIVEN slot 05 holding ingame_face.toml, torso.fmdl posed for PES 21, torso.skl and
           glove_l.fmdl
           WHEN the export is compiled for PES 21
           THEN the CPK holds no face/real/71405/ path, k0625's package holds boots.fmdl and a
                boots.skl byte-identical to torso.skl, and g0625's holds glove_l.fmdl
TC-MOD-17  GIVEN slot 05 holding ingame_face.toml, kit_boots.fmdl and Crocs.boots, and slot 07
           linking Crocs.boots plainly
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, k0625 holds the merged boots, no face/real/71405/ path
                is in the CPK, and Crocs keeps its own output for slot 07, who alone links it
                plainly, under his own k0627
TC-MOD-18  GIVEN slot 05 holding ingame_face.toml and an empty face/ subfolder, with boots.fmdl
           WHEN the export is compiled
           THEN no finding is reported for face/, no face package is emitted, and k0625 holds the
                boots
TC-MOD-19  GIVEN slot 05 holding boots.fmdl only, without ingame_face.toml; and slot 07 holding an
           empty face/ and nothing else
           WHEN the export is compiled for PES 21
           THEN a face package is emitted for 71405 and for 71407 (the blank face folder) and k0625
                holds the boots
TC-MOD-20  GIVEN a /co/ slot 05 folder holding face_high.model, face_high.mtl, skin.dds and
           face_diff.bin
           WHEN it is compiled for PES 17
           THEN the CPK holds common/character0/model/character/face/real/71405.cpk, whose face.xml
                lists one face_neck entry naming the emitted model, whose .mtl names skin.dds under
                model/character/uniform/common/714/05 - <name>/, whose <dif> carries face_diff.bin,
                and no Asset/ path exists
TC-MOD-21  GIVEN slot 05 holding face_high.model, kit_boots.model, hat_parts.model,
           arm_gloveL.model, cape_model_type_cape.model and visor_ratio_2_parts.model
           WHEN it is compiled for PES 17
           THEN face.xml lists six entries, in any order: one face_neck, three parts (visor's
                with ratio="2"), one gloveL and one cape, in the face CPK alone: no boots/ or
                glove/ folder is written for the player
TC-MOD-22  GIVEN slot 05 holding face_high.model, face_high.mtl, kit_boots.model, kit_boots.mtl and
           Crocs.boots, Boots/Crocs/ holding boots.model and boots.mtl (or a boots model of
           another name with its .mtl, and a .mtl no model uses)
           WHEN it is compiled for PES 17
           THEN common/character0/model/character/boots/k0625/ (slot 05's own ID: his kit_boots
                is face content, so his link is plain and he is Crocs's one wearer) holds
                Crocs's files as loose entries,
                its boots models merged into one boots.model with their boots.mtl (a .mtl no
                model uses left out), slot 05's face.xml lists kit_boots's emitted model as a
                parts entry and none of Crocs's, and no link_combined is reported
TC-MOD-23  GIVEN slot 05 holding body_uniform.model (or, for PES 15, his own face.xml typing a
           model uniform)
           WHEN it is compiled for PES 15, then PES 16, then PES 17
           THEN its entry type is uniform_sub with xml_uniform_pes15 on 15, uniform on 16 and 17,
                and on 16 its emitted model name carries the oral_ prefix
TC-MOD-24  GIVEN slot 05 holding legs.model.common and Common/legs.model with Common/legs.mtl;
           then slot 05 also holding a legs.mtl of its own naming another texture
           WHEN it is compiled for PES 17 each time
           THEN face.xml points the entry at the Common path with 714 substituted, the Common
                output holds oral_legs_win32.model and legs.mtl once, and the face CPK does
                not hold them; in the
                second run the entry's material is slot 05's own legs.mtl, in the face CPK
TC-MOD-25  GIVEN slot 05 holding face_edithair.xml, and another export holding hat.model in
           Boots/Mud/ (a shared boots folder), linked by slot 07
           WHEN each export is checked for PES 17
           THEN edithair_unsupported drops the first, model_name_invalid drops Mud and its
                linking player (link_target_dropped)
TC-MOD-26  GIVEN slot 05 holding boots.fmdl posed for PES 21 and boots.model posed for PES 17
           WHEN it is compiled for PES 21, then for PES 17
           THEN the FMDL is used on 21 and the .model on 17, with no conversion finding either
                way and no finding about the source left unread (on 21, a .model without its
                .mtl)
TC-MOD-27  GIVEN the tracer export (a Fox face: fcl_hair.fmdl)
           WHEN it is compiled for PES 17
           THEN the face CPK holds a .model and .mtl pair the pes_model reader accepts with the
                FMDL's mesh count less its anti-blur meshes (folded back into their source
                material), the .mtl naming the folder's textures as DDS under the player's
                texture home and its dummy_kit texture under the team's Common directory,
                with no sampler for the Fox dummy normal and specular maps; on PES 15,
                skeleton_retargeted names each model whose bones sit elsewhere in PES 15's
                skeleton
TC-MOD-28  GIVEN slot 05 holding boots.glb, boots.model and skin.png (or linking Boots/Crocs/
           holding boots.glb), and another folder holding boots.glb beside boots.fmdl
           WHEN each export is compiled for PES 21 (the link: or PES 17), then checked
           THEN the first reports model_gltf_unsupported naming boots.glb (Boots/Crocs/boots.glb,
                on slot 05) and slot 05 is dropped, boots.model not converted instead; the
                second compiles the FMDL with no model_gltf_unsupported; check reports it for
                neither
TC-MOD-29  GIVEN slot 05 holding a boots .model one of whose bones stores a singular matrix, which
           the pes_model reader accepts and the conversion to FMDL cannot invert (or a face FMDL
           whose conversion for PES 17 fails)
           WHEN the export is compiled for PES 21, then again with pass_through on (the FMDL:
                compiled for PES 17)
           THEN model_conversion_failed is reported naming the file and that model's package is
                left out (the face FMDL's: the face CPK); for the boots, the folder's blank face
                and its textures stand (a task's failure is its package's, as every task
                failure), and with pass_through on the boots are still left out
TC-MOD-30  GIVEN slot 05 holding a .model with a vertex 6000 units from the origin
           WHEN the export is compiled for PES 21
           THEN vertex_too_far_from_origin is reported naming the .model and the folder dropped
TC-MOD-31  GIVEN slot 05 holding body.fmdl whose vertices carry skh_*_l and skh_*_r weights
           WHEN the export is compiled for PES 21
           THEN g0625 holds glove_l.fmdl and glove_r.fmdl, the face's fcl_hair.fmdl holds the
                rest, every face of the source is in exactly one of the three, and
                model_hand_split names body.fmdl
TC-MOD-32  GIVEN slot 05 holding boots.fmdl and a face_diff.bin that is not the bundled one,
           without ingame_face.toml; and slot 07 holding ingame_face.toml, boots.fmdl and
                face_diff.bin
           WHEN the export is compiled for PES 21
           THEN face_file_not_used names face_diff.bin on each folder, 71405's face package
                holds the bundled face_diff.bin alone, and no face package is emitted for 71407
TC-MOD-33  GIVEN slot 05 holding face_high.fmdl and skin.dds, and linking Faces/Round, whose
           folder holds hair_high.fmdl and a skin.dds of other bytes
           WHEN the export is compiled for PES 21
           THEN shared_texture_overridden names skin and Faces/Round, 71405's face package is
                in the CPK, and slot 05's texture folder holds the skin.ftex converted from the
                player's skin.dds
TC-MOD-34  GIVEN slot 05 holding boots.model and boots.mtl naming skin, and skin.dds
           WHEN the export is compiled for PES 21
           THEN k0625 holds a boots.fmdl the fmdl reader accepts with the source's mesh count
                plus the anti-blur mesh the FMDL export regenerates (the card head's one mesh
                becomes two), its texture table naming skin.dds under the player's texture
                home, and skin.ftex sits in the player's common subfolder
TC-MOD-35  GIVEN slot 05 holding ingame_face.toml, kit_boots.model, kit_boots.mtl and Crocs.boots,
           Boots/Crocs/ holding boots.model and boots.mtl, linked by no other player; and
           another export whose slot 05 holds ingame_face.toml, Crocs.boots, and parts/boots.model
           with parts/boots.mtl naming skin.dds and parts/skin.dds, Boots/Crocs/ holding
           boots.model, its boots.mtl naming skin beside its skin.png; and a third whose slot
           05 holds ingame_face.toml, Studs.boots, and boots.model with a boots.mtl naming
           studs.dds, which Boots/Studs/ alone holds, beside its boots.model and boots.mtl
           WHEN each is compiled for PES 17
           THEN link_combined is reported, no 71405 face CPK is written,
                common/character0/model/character/boots/k0625/ holds one boots model whose mesh
                count is Crocs's plus the local model's, its textures, Crocs's among them, in his
                texture home, and no boots/k0644/ folder exists; the second's merged boots.mtl
                names, Crocs's part first, skin.dds at his texture home's root and his part's
                parts/skin.dds, both written there; the third's names studs.dds in his texture
                home for both parts, which the CPK holds
TC-MOD-36  GIVEN slot 05 holding boots.fmdl whose material uses fox3ddf_ggx and names no
           environment texture
           WHEN the export is compiled for PES 17
           THEN the emitted .mtl gives that material the Basic_CNSR shader and an EnvironmentMap
                sampler naming a texture the CPK holds, which is the template environment
                map
TC-MOD-37  GIVEN slot 05 holding face_high.model, face_high.mtl.common, and Common/ holding
           face_high.mtl naming skin and skin.dds
           WHEN the export is compiled for PES 17
           THEN the face.xml entry's material names the Common path with 714 substituted, and
                the Common output holds face_high.mtl and skin.dds once, the face CPK neither
TC-MOD-38  GIVEN Gloves/Keeper/ holding glove_l.model and glove_r.model with their .mtl files,
           and a materials.mtl no model uses, linked by slot 05
           WHEN the export is compiled for PES 17
           THEN the gloves output of ID 625 (slot 05's own, Keeper's one wearer) holds a
                glove.xml listing the two emitted
                models typed gloveL and gloveR, and of the .mtl files only those they use
TC-MOD-39  GIVEN slot 07 holding an empty face/ and a face_diff.bin that is not the bundled one
           WHEN the export is compiled for PES 17
           THEN face_file_not_used names face_diff.bin, and face/real/71407.cpk holds a face.xml
                with one face_neck entry naming ./oral_dummy_*.model and ./dummy.mtl and a
                <dif> from the bundled face_diff.bin, oral_dummy_win32.model byte-identical to the
                template dummy.model, and a dummy.mtl holding an empty <materialset>
TC-MOD-40  GIVEN Faces/Longhair/ holding hair_high.model and hair_high.mtl, linked by slot 05,
           which holds face_high.model and face_high.mtl
           WHEN the export is compiled for PES 17
           THEN 71405.cpk holds both models, its face.xml lists both, and no output exists for
                Longhair on its own; a file of slot 05's own replaces Longhair's of its name
TC-MOD-41  GIVEN slot 05 holding ingame_face.toml, shirt.model and socks.model with .mtl files
           defining material "skin" differently; and another export whose slot 05 holds
           ingame_face.toml, shirt.model and socks.model naming one bone with different
           transforms
           WHEN each export is compiled for PES 17
           THEN the first reports merge_material_conflict and the second skl_merge_conflict,
                and neither CPK holds common/character0/model/character/boots/k0625/
TC-MOD-42  GIVEN slot 05 holding a face model, face_diff.bin and face_diff.xml; and another export
           whose slot 05 holds a face model and a face_diff.bin shorter than its header gives
           WHEN each export is checked, then compiled
           THEN the first reports xml_dif_conflict naming face_diff.xml and the second
                face_diff_invalid naming face_diff.bin, both times, and each slot 05 folder is
                left out of its CPK
TC-MOD-43  GIVEN slot 05 holding body.model, whose vertices carry skh_*_l and skh_*_r weights,
           with body.mtl defining its material; a second export whose slot 05 holds that
           model as body_kit1.model and body_kit2.model with one body.mtl, beside Kits/p1, p2
           and p3; and a third whose slot 05 holds body.model and body.mtl.common, beside
           Common/body.mtl
           WHEN each is compiled for PES 17
           THEN 71405.cpk holds oral_body_win32.model, oral_body_glove_l_win32.model and
                oral_body_glove_r_win32.model, its face.xml listing them typed parts, gloveL
                and gloveR, each naming ./body.mtl, every face of the source is in exactly one
                of the three, model_hand_split names body.model, and no glove/g0625/ folder;
                the second's holds those three for each kit number, kit 3's copied from kit 1
                (kit_variant_missing), its face.xml listing each part once, by
                ./oral_body_kitN_*.model and its kitN hands; the third's holds the three split
                models and no body.mtl, their entries naming
                model/character/uniform/common/<team>/body.mtl
TC-MOD-44  GIVEN slot 05 holding ingame_face.toml, socks.model with socks.mtl naming skin.dds,
           and kit_boots.model.common; Common/ holding kit_boots.model, kit_boots.mtl naming
           studs and studs.dds
           WHEN the export is compiled for PES 17
           THEN no 71405 face CPK is written, boots/k0625/boots.model's mesh count is the Common
                model's plus socks's, and its boots.mtl names studs.dds in the team's Common
                output and skin.dds in his common folder
TC-MOD-45  GIVEN slot 05 holding ingame_face.toml and gloves models of his own or linked from
           Common (glove_l.model.common, glove_r.model and glove_r.mtl.common); Common/ holding
           glove_l.model, glove_l.mtl and glove_r.mtl
           WHEN the export is compiled for PES 17
           THEN glove/g0625/ holds each gloves model under its name with its .mtl (here
                glove_l.model byte-identical to Common's), and its glove.xml lists each, typed
                gloveL or gloveR, naming its .mtl; each gloves model is written unmerged, a
                combined gloves folder's beside his own, his own winning a name
TC-MOD-46  GIVEN slot 05 holding the tracer's boots.fmdl and a boots.skl that is PES 21's body
           skeleton with sk_hand_r raised 5 cm; and another export the same with dsk_ear_t_l
           raised instead, a bone the boots do not use
           WHEN both are compiled for PES 21
           THEN the first's boots.fmdl is re-bound (its meshes differ from the source's),
                skeleton_retargeted is reported and its boots.skl is the bundled body skeleton;
                the second's boots.fmdl is packed as its source bytes with no finding, its
                boots.skl the member's
TC-MOD-47  GIVEN slot 05 holding the hand-split body as fcl_hair.fmdl and a fcl_hair.skl that is
           PES 21's body skeleton with sk_hand_l raised 5 cm
           WHEN the export is compiled for PES 21
           THEN the face reports skeleton_retargeted, and glove_l.fmdl's meshes differ from
                those compiled with the unmoved skeleton while glove_r.fmdl's are the same
TC-MOD-48  GIVEN slot 05 holding the pre-Fox tracer's face_high.model with its face.mtl, textures
           and face_diff.bin
           WHEN the export is compiled for PES 21
           THEN the face package holds face_high.fmdl and face_diff.bin and no .skl, and
                skl_no_slot is not reported
TC-MOD-49  GIVEN slot 05 holding the stock PES 17 cap model as face_high.model with its .mtl, a
           model whose bones PES 15's skeleton does not all hold
           WHEN the export is compiled for PES 15, then PES 16
           THEN on PES 15 bone_folded_for_version is reported for the two bones PES 15 lacks
                (folded into dsk_upperarm_l; the bones it keeps sit on PES 15's pose, so nothing
                is re-bound and skeleton_retargeted is not reported) and the face CPK's model
                differs from the source while its face_high.mtl is the member's set, pointed; on
                PES 16 the model is packed byte-identical to the source
TC-MOD-50  GIVEN the cap as a shared folder's model with its .mtl (Boots/Cap/boots.model,
           Gloves/Cap/glove_l.model) linked by slot 05, or a PES 17 collar as
           Collars/collar_12.model
           WHEN the export is compiled for PES 15, then PES 16 (the collar: PES 17)
           THEN on PES 15 the output's model differs from the source and the fold lines are
                reported on that folder or file, naming the model (and the collar's
                skeleton_retargeted); on PES 16 (17) it is byte-identical to the source
TC-MOD-51  GIVEN Common/ holding the cap as cap.model with cap.mtl, and slot 05 holding
           cap.model.common
           WHEN the export is compiled for PES 15, then PES 16
           THEN on PES 15 the Common output's model differs from the source and the fold
                lines are reported on Common, naming cap.model; on PES 16 it is byte-identical;
                with no Common/cap.mtl, the model is packed as written with no fold line
TC-MOD-52  GIVEN slot 05 holding the cap as face_high.model and face_high.mtl.common, Common/
           holding face_high.mtl, the cap's set
           WHEN the export is compiled for PES 15
           THEN the fold lines are reported on the player and the face CPK's model differs from
                the source
TC-MOD-53  GIVEN slot 05 holding face_high.fmdl and gloves/keeper.fmdl, a model named for no part
           WHEN the export is compiled for PES 21
           THEN fmdl_fcl_hair_fallback names gloves/keeper.fmdl, the face package holds its meshes
                (the subfolder's name forces nothing), no gloves package is written and the exit
                code is 0
TC-MOD-54  GIVEN slot 05 holding body.model and body.mtl.common, Common/ holding body.mtl, the
           model's set
           WHEN the export is compiled for PES 21
           THEN the face FPK holds the converted body FMDL with the set's materials, its skin.dds
                paths naming the team's Common output (when the folder holds a skin.dds too:
                the set's stems resolve where the set is), and no finding names
                body.mtl.common
TC-MOD-55  GIVEN Faces/Round holding fcl_hair.fmdl and boots.fmdl, slot 05 linking it as his face
           WHEN the export is compiled for PES 21
           THEN the boots folder of slot 05's exclusive id holds boots.fmdl and his BootsList row
                names it
TC-MOD-56  GIVEN Boots/Studs holding boots_kit1.model and boots_kit2.model with their .mtl (or the
           two as FMDLs), slot 05 linking it plainly
           WHEN the export is compiled for PES 17 (PES 21 for the FMDLs)
           THEN kit_variant_model_left_out is reported on the folder and the shared boots model
                (boots.model; on PES 21 the boots FPK) holds boots_kit1's meshes alone; on PES
                17 a variant of another number in the linking player's own folder is no variant
                of the folder's
TC-MOD-57  GIVEN Boots/Studs holding studs.dds and no model (or only models whose every mesh is
           hidden), Boots/Zebra holding boots.model and its .mtl, slots 05 and 07 linking Studs
           plainly and slots 06 and 08 Zebra
           WHEN the export is compiled for PES 17
           THEN shared_folder_no_model is reported on Boots/Studs as a Warning, no boots folder is
                written for it, Zebra takes the block's first shared id, and the exit code is 0
TC-MOD-61  GIVEN slot 05 holding legs.model.common, a local legs.mtl (the Common one's materials,
           naming ./skin.dds) and his own skin.dds, Common/ holding legs.model, legs.mtl and
           skin.dds
           WHEN the export is compiled for PES 21
           THEN the converted legs in his face FPK name skin.dds at his texture home, where his
                skin.ftex is written, Common's skin.ftex is in the team's Common output, and no
                finding names a texture
TC-MOD-62  GIVEN slot 05 holding face.fmdl, boots_kit1.fmdl and a boots_kit2.fmdl that does
           not parse (or, holding ingame_face.toml, boots_kit1.model and a boots_kit2.model that
           does not parse)
           WHEN the export is compiled for PES 21 (PES 17 for the .model files)
           THEN no finding names the unparsable variant but kit_variant_model_left_out, and slot
                05's boots (its boots FPK; on PES 17 boots.model) hold boots_kit1's model
TC-MOD-63  withdrawn: a subfolder's model is a face part by its name (4.y-sub), so a broken
           common/parts_body.model is model_broken like any face model's; the premise, a
           common/ file no task reads, is gone
TC-MOD-64  GIVEN slot 05 holding face_high.fmdl with two meshes, the second flagged invisible
           WHEN the export is compiled for PES 17
           THEN the face's .model holds the first mesh alone and no mesh_flags_dropped names
                invisible
TC-MOD-65  GIVEN slot 05 holding face_high.fmdl, shorts.dds, and jessie/hair_high.fmdl whose
           material names shorts.dds, with jessie/shorts.dds of other bytes
           WHEN the export is compiled for PES 21
           THEN the face package holds both models, both textures sit under the player's
                texture home at their paths (shorts.ftex and jessie/shorts.ftex, each in its
                own #windx11 folder), hair_high.fmdl's path names the nearer one at jessie/,
                and no texture_stem_conflict is reported
TC-MOD-66  GIVEN slot 05 holding face_high.model with face_high.mtl naming skin.dds, skin.dds,
           and jessie/body/x.model with jessie/body/x.mtl naming skin.dds, with jessie/skin.dds;
           and another export whose slot 05 holds face_high.fmdl, jessie/hair_high.model with
           the root's hair_high.mtl as its .mtl, naming skin.dds, skin.dds and jessie/skin.dds
           WHEN the first is compiled for PES 17, the second for PES 21
           THEN the face CPK holds jessie/body/oral_x_win32.model and jessie/body/x.mtl at those
                paths, face.xml lists ./jessie/body/oral_x_*.model with ./jessie/body/x.mtl,
                x.mtl's skin path names jessie/skin.dds under the texture home and face_high.mtl's
                the root's skin.dds, and no texture_stem_conflict is reported; the second's
                converted hair_high.fmdl names skin in the texture home's own folder, where its
                .mtl is, and no texture_not_found is reported
TC-MOD-67  GIVEN slot 05 holding face_high.fmdl, jessie/ingame_face.toml and
           jessie/settings.toml
           WHEN the export is checked, then compiled for PES 21 with
                strict_file_type_check = false
           THEN the check reports file_type_disallowed for each of the two and drops the
                folder; the lenient compile keeps it with the two as Infos, neither
                counting (no marker, no settings), and the player keeps his face
TC-MOD-68  GIVEN Boots/Crocs holding boots.fmdl and extra/x_boots.fmdl, slot 05 linking it
           WHEN the export is compiled for PES 21
           THEN no file_type_disallowed is reported and the boots package holds
                boots.fmdl's and x_boots.fmdl's meshes, merged
TC-MOD-71  GIVEN slot 05 holding face_high.fmdl naming ./textures/skin, and textures/skin.dds
           WHEN the export is compiled for PES 21
           THEN the face package's model names skin in the directory
                .../common/<team>/05 - A/textures/sourceimages/, the CPK holds that
                directory's #windx11/skin.ftex, and no finding names skin; with skin.dds only
                elsewhere, fmdl_texture_not_found names ./textures/skin.dds and drops the folder
TC-MOD-72  GIVEN Kits/p1, Common/kit1/armor_bsm.dds, Boots/Crocs holding boots.fmdl naming
           ./kitN/armor_bsm and kitN/armor_bsm.dds.common, and slot 05 linking Crocs.boots
           WHEN the export is compiled for PES 21
           THEN the boots model names armor_bsm in
                /Assets/pes16/model/character/common/<team>/kitN/sourceimages/, and neither
                file_type_disallowed nor file_not_used is reported
TC-MOD-73  GIVEN slot 05 holding face_high.fmdl and skin.dds, linking Faces/Round, which holds
           jessie/hair_high.fmdl naming skin and jessie/skin.dds of other bytes
           WHEN the export is compiled for PES 21
           THEN the face package holds both models' meshes, hair_high's path names skin in
                .../common/<team>/05 - A/jessie/sourceimages/, the player's texture home holds
                both skin.ftex at their paths, and neither texture_stem_conflict nor
                shared_texture_overridden is reported
TC-MOD-74  withdrawn: restated as TC-MOD-93 (`spec/team_compiler/README.md`), a gloves output
           packing every .mtl at its root since PES 17 loads no ../ texture path
TC-MOD-75  GIVEN Common/jessie/armor_bsm.dds, Boots/Crocs holding boots.fmdl naming
           ./jessie/armor_bsm and jessie/armor_bsm.dds.common, and slot 05 linking Crocs.boots
           WHEN the export is compiled for PES 21
           THEN the boots model names armor_bsm in
                /Assets/pes16/model/character/common/<team>/jessie/sourceimages/, the CPK holds
                that directory's #windx11/armor_bsm.ftex, and no file_type_disallowed,
                file_not_used or fmdl_texture_not_found is reported
TC-MOD-78  GIVEN Common/hair.dds, and slot 05 holding face_high.fmdl and linking Faces/Round,
           which holds hair_high.fmdl naming hair and hair.dds.common
           WHEN the export is compiled for PES 21
           THEN hair_high in 71405's face package names hair in
                /Assets/pes16/model/character/common/714/sourceimages/, and no finding names hair
TC-MOD-76  GIVEN Faces/Round holding fcl_hair.fmdl and hair.dds.common, no hair.dds in Common/,
           and slot 05 holding face_high.fmdl and linking Round.face
           WHEN the export is checked for PES 21
           THEN common_link_missing is reported on Faces/Round, and slot 05 reports
                link_target_dropped naming it and is dropped
TC-MOD-77  GIVEN a .common link PES 15-17 does not read: a shared folder's (Boots/Crocs's
           armor.dds.common, linked by slot 05; a face folder's hair_high.mtl.common), or slot
           05's link to a per-kit Common model
           WHEN the export is compiled for PES 17
           THEN file_not_used names the link on its folder, which compiles as if it were absent
TC-MOD-79  GIVEN Common/boots.fmdl, Boots/Crocs holding boots.fmdl.common alone, and slot 05
           holding face_high.fmdl and linking Crocs.boots
           WHEN the export is compiled for PES 21
           THEN Crocs takes slot 05's own boots ID, 625 (he is its one wearer), whose boots
                package holds Common's boots.fmdl's meshes, slot 05's BootsList row names 625, and
                neither shared_folder_no_model nor file_not_used is reported
TC-MOD-80  GIVEN Common/jessie/body.fmdl naming hair and Common/jessie/hair.dds, and slot 05
           holding face_high.fmdl and linking Faces/Round, which holds fcl_hair.fmdl and
           jessie/body.fmdl.common
           WHEN the export is compiled for PES 21
           THEN 71405's face package holds face_high's, fcl_hair's and body.fmdl's meshes, the
                model built from body.fmdl names hair in
                /Assets/pes16/model/character/common/714/jessie/sourceimages/, the CPK holds
                that directory's #windx11/hair.ftex, and no finding names body or hair but
                fmdl_fcl_hair_fallback on the link (body is no face part's name)
TC-MOD-81  GIVEN Common/boots.mtl naming ./skin.dds and Common/skin.dds, Boots/Crocs holding
           boots.model and boots.mtl.common, and slot 05 holding face_high.fmdl and linking
           Crocs.boots
           WHEN the export is compiled for PES 21
           THEN Crocs' boots package holds boots.model converted with Common's boots.mtl, its
                model names skin in /Assets/pes16/model/character/common/714/sourceimages/,
                and neither model_material_undefined nor file_not_used is reported
TC-MOD-82  GIVEN Common/boots.mtl naming ./skin.dds and Common/skin.dds, and Faces/Round holding
           fcl_hair.fmdl, boots.model, materials.mtl and boots.mtl.common, slot 05 linking it
           as his face
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, and the boots folder of slot 05's exclusive id holds
                boots.model converted with Common's boots.mtl, not materials.mtl, naming skin
                in /Assets/pes16/model/character/common/714/sourceimages/, and no
                file_not_used is reported
TC-MOD-83  GIVEN Common/glove_l.fmdl, Boots/Crocs holding boots.fmdl and glove_l.fmdl.common,
           and slot 05 linking Crocs.boots
           WHEN the export is compiled for PES 21
           THEN fmdl_name_invalid names glove_l.fmdl.common on Boots/Crocs, and Boots/Crocs and
                slot 05 (link_target_dropped) are left out
TC-MOD-69  GIVEN slot 05 holding face_high.model with its .mtl, and oral.fmdl whose only mesh
           is flagged invisible; and another export whose slot 05 holds the face model,
           aa.fmdl.common linking a Common/aa.fmdl so flagged, and zz.fmdl so flagged
           WHEN the first is compiled for PES 17, the second checked for PES 17
           THEN the face package holds the face model, its .mtl and face.xml alone, the xml
                names the face model alone, and model_hidden_dropped names oral.fmdl; the
                second reports model_hidden_dropped for aa.fmdl.common (naming Common/aa.fmdl)
                and zz.fmdl in slot 05's file order, the link at its own place, then for
                Common/aa.fmdl
TC-MOD-70  GIVEN Boots/Crocs/boots.fmdl, Boots/Tabi/boots.fmdl, and slot 05 holding
           face_high.fmdl, Crocs.boots and jessie/Crocs.boots
           WHEN the export is compiled for PES 21, then checked with jessie/Tabi.boots in
                place of jessie/Crocs.boots
           THEN the compile reports no shared_link_duplicate and his boots are Crocs';
                the check reports shared_link_duplicate and drops the folder
TC-MOD-58  GIVEN Faces/Round holding fcl_hair.fmdl, boots.model and boots.mtl, slot 05 linking it
           as his face
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, the boots folder of slot 05's exclusive id holds the
                boots converted with Round's .mtl, his BootsList row names it, and the exit code
                is 0
TC-MOD-59  GIVEN Faces/Round holding fcl_hair.fmdl and boots.fmdl, Boots/Crocs holding boots.fmdl,
           slot 05 holding no model and linking both (Round.face, Crocs.boots)
           WHEN the export is compiled for PES 21
           THEN link_combined is reported for both links, the boots folder of slot 05's exclusive
                id holds Round's and Crocs's boots merged, his BootsList row names it, and no
                shared boots folder is written for Crocs
TC-MOD-60  GIVEN slot 05 holding a hand-weighted face model and Crocs.gloves, Gloves/Crocs holding
           glove_l.fmdl and glove_r.fmdl
           WHEN the export is compiled for PES 21
           THEN link_combined is reported, the gloves folder of slot 05's exclusive id holds the
                split hands and Crocs's gloves, his GloveList row names it, and no shared gloves
                folder is written for Crocs
TC-MOD-84  GIVEN slot 05 holding Face_High.fmdl and face_high.mtl, which a PES 17 output packs
           under one name, letter case aside
           WHEN the export is compiled for PES 17
           THEN folder_pack_failed names the folder (Common, for files of Common/) and the name,
                and that folder's package is left out (for Common/, its models and .mtl files,
                its textures standing) while the rest of the export compiles
TC-MOD-85  GIVEN an FMDL that a PES 15-17 output reads (the tracer's fcl_hair.fmdl with its .skl),
           in a subfolder of slot 05, a shared folder he links, Common/ through a .common
           link, or among an ingame_face.toml player's boots or gloves parts
           WHEN the export is compiled for PES 17
           THEN it is converted into a .model and .mtl written where a .model of its name there
                would be, the .mtl naming its textures where that output's textures sit, its
                conversion findings reported at the folder whose output it joins (the player,
                for a linked face folder's or an ingame_face.toml player's Common part), and no
                .fmdl or .skl in the CPK
TC-MOD-86  GIVEN slot 05 holding a face model, Face_Diff.bin, kit_boots.fmdl naming shirt,
           Shirt.dds and Kit_Boots.skl
           WHEN the export is compiled for PES 21
           THEN each file plays the role its name gives, letter case aside: the face's
                face_diff.bin is Face_Diff.bin, the boots name Shirt.dds's texture, and
                boots.skl is Kit_Boots.skl
TC-MOD-87  GIVEN slot 05's face parts defining one material over textures in two places:
           torso.fmdl beside his shirt.dds, and torso2.fmdl.common linking the same model
           in Common/ beside Common/shirt.dds
           WHEN the export is compiled for PES 21
           THEN merge_material_conflict names the material and nothing of slot 05 is in
                the CPK
TC-MOD-88  GIVEN slot 05 holding boots.model with a boots.mtl naming
           model/character/uniform/common/714/shirt.dds
           WHEN the export is compiled for PES 21
           THEN the converted boots name shirt in
                /Assets/pes16/model/character/common/714/sourceimages/, found there when
                Common/ holds shirt.dds; else, with no PES install, mtl_texture_not_found
                and fmdl_texture_not_found are Warnings and the folder is kept
TC-MOD-89  GIVEN slot 05 holding boots.model with its .mtl, one of whose bones no game
           skeleton holds
           WHEN the export is compiled for PES 21
           THEN the boots package's boots.skl holds that bone; beside a boots.skl of his
                that lacks it, skl_merge_conflict leaves the boots out
TC-MOD-90  GIVEN TC-MOD-36's boots, or a Common/legs.fmdl with such a material, beside an
           env.dds of its own (slot 05's env.dds, his env.dds.common link, or
           Common/env.dds), or templates/env.dds in the data directory
           WHEN the export is compiled for PES 17
           THEN the EnvironmentMap names that env.dds and the CPK's env.dds is that texture,
                not the bundled template's; the data directory's reports
                template_override_active
TC-MOD-91  GIVEN slot 05 holding an FMDL one of whose textures has the directory
           /Assets/pes16/model/character/uniform/texture/ (the game's kit textures)
           WHEN the export is compiled for PES 21
           THEN fmdl_texture_kit_path names the path, the model is packed with that path
                unchanged, and fmdl_texture_not_found is not reported for it
TC-MOD-92  GIVEN slot 05 holding a face model and notes.txt
           WHEN the export is compiled with strict_file_type_check on
           THEN file_not_used names notes.txt, file_type_disallowed is not reported, and the
                folder compiles as if it were absent
```

**Textures**

```
TC-TEX-01  GIVEN slot 05 holding skin.png (1024x1024 with alpha) referenced by its face model;
           and another export whose Common/ holds hair.png, named by the Common/legs.fmdl slot
           05 links, and whose Boots/Crocs/, linked by slot 05, holds sole.png beside its
           boots.fmdl
           WHEN the first is compiled for PES 21, then PES 18, and the second for PES 21
           THEN the emitted skin.ftex decodes as BC7 with 11 mip levels, then as BC3 with 11;
                the second CPK's Common hair.ftex and k0625's sole.ftex (Crocs under slot 05's
                own ID) are BC7 with a full mip chain
TC-TEX-02  GIVEN slot 05 holding skin.dds already BC7 with a full mip chain
           WHEN the export is compiled for PES 21, then PES 18
           THEN on 21 the FTEX's mip data converts back to the source DDS unchanged; on 18 it is BC3
TC-TEX-03  GIVEN four player folders: skin.png of 3x3 pixels, skin.png of 300x300 pixels on a
           Fox face, skin.dds whose header is a PNG's (or a kit's texture so refused), and
           skin.dds of 512x461 with a mip chain on a Fox face
           WHEN the export is checked or compiled for PES 21
           THEN texture_too_small, texture_not_pow2 and texture_type_mismatch are reported, each
                dropping its folder (a kit's: that kit); with pass_through on, texture_too_small
                and texture_not_pow2 keep their folders, their textures compiled as they are;
                the fourth folder compiles with no texture finding
TC-TEX-04  GIVEN a .dds in a codec dds_convert cannot decode
           WHEN the export is compiled
           THEN texture_codec_unsupported is reported naming the file and the folder is dropped
TC-TEX-05  GIVEN slot 05's face_high.fmdl naming hair in the team's Common output, an export with
           no Common/hair.*, and a PES folder whose DpFileList lists 4cc_61_midcup.cpk before
           the CPK compiled (cpk_name 4cc_62_midcup) and 4cc_63_midcup.cpk after it
           WHEN the export is compiled for PES 21 with 4cc_61_midcup.cpk holding
           Asset/model/character/common/714/sourceimages/#windx11/hair.ftex, then with only
           4cc_63_midcup.cpk holding it, then with no PES folder; and again with the model naming
           hair in the Common output's jessie/ folder, slot 05 holding jessie/hair.dds.common
           and the export Common/jessie/hair.dds
           THEN the first compiles with no texture finding; the second reports fmdl_texture_not_found as
                an Error and drops the folder; the third reports it as a Warning and keeps it;
                the last compiles with no texture finding, the model naming hair in
                /Assets/pes16/model/character/common/714/jessie/sourceimages/ and the CPK
                holding that folder's hair.ftex once
TC-TEX-06  GIVEN p1/kit.png, p1/kit_back.tga and Portraits/05.webp
           WHEN the export is compiled for PES 21
           THEN u0714p1.ftex and u0714p1_back.ftex are emitted and 71405.dds is a DDS
TC-TEX-07  GIVEN slot 05 holding skin.png of 1002x1002 pixels
           WHEN the export is compiled for PES 17
           THEN texture_not_div4 is reported and the folder is dropped
TC-TEX-08  GIVEN a /co/ export whose slot 05 holds a face model naming skin and hair, skin.dds,
           and hair.dds already WESYS-wrapped; and dds_compression = true, then false, then
           auto with multicpk_mode off, then on
           WHEN the export is compiled for PES 17 each time, then for PES 21 with true and
           with false
           THEN every .dds entry of the first run but hair.dds is WESYS-wrapped and decompresses
                to the second run's bytes; the third run's .dds entries equal the second's and
                the fourth's are wrapped as the first's; hair.dds is emitted as it is in every
                PES 17 run; the two PES 21 runs'
                CPKs are byte-identical
TC-TEX-09  GIVEN slot 05 holding a face model naming the textures hair and skin, skin.dds,
           hair.dds.common, and Common/hair.dds
           WHEN the export is compiled for PES 21 (or PES 17)
           THEN on PES 21 hair.ftex sits once at Asset/model/character/common/714/sourceimages/
                #windx11/, the packed model's hair path names /Assets/pes16/model/character/
                common/714/sourceimages/, its skin path the player's own texture folder, and no
                hair.ftex and no link file is in the player's folder; on PES 17 the .mtl names
                hair in model/character/uniform/common/714/, packed once there
TC-TEX-10  GIVEN slot 05's face model naming skin and skin_nrm, skin.png (1024x1024, fully
           opaque) and skin_nrm.png (or a per-kit pants_nrm_kit1.png in his folder)
           WHEN the export is compiled for PES 18 (the normal map: or PES 21)
           THEN skin.ftex decodes as BC1, and skin_nrm.ftex (pants_nrm_kit1.ftex) as BC3, on
                PES 18 holding X in alpha and Y in green
TC-TEX-11  GIVEN slot 06 holding a face model naming hair and hair.dds.common, no Common/hair.*,
           and TC-TEX-05's PES folder
           WHEN the export is compiled for PES 21 with cpk_name 4cc_62_midcup and
           4cc_61_midcup.cpk holding the texture at the team's Common path, then with only
           4cc_63_midcup.cpk holding it; and with 4cc_61_midcup.cpk holding it again, the model
           and the link moved into slot 06's jessie/
           THEN the first reports no texture or link finding and the model's hair path names
                /Assets/pes16/model/character/common/714/sourceimages/; the second reports
                common_link_missing and drops the folder; so does the third, the link naming
                Common/jessie/hair.dds, which no installed CPK holds
TC-TEX-12  GIVEN slot 05's face model naming hair, with hair.png.common, and Common/hair.png of
           3x3 pixels; and another export whose slot 05 links hair.dds.common to a
           Common/hair.dds whose header reads but whose pixel data is cut short (or a BC6H
           one, which only conversion refuses, beside a sound Common/skin.dds)
           WHEN each export is compiled for PES 21
           THEN the first reports texture_too_small and link_target_dropped and slot 05 is left
                out; the second reports folder_pack_failed (texture_codec_unsupported, for the
                BC6H) at Common, no texture of Common/ is in the CPK, and slot 05's face
                package is; a player folder's own texture failing so reports
                folder_pack_failed on that folder, which is left out
TC-TEX-16  GIVEN slot 05 holding face_high.fmdl naming hair, hair.dds.common and Common/hair.dds,
           and linking Faces/Round, whose folder holds hair_high.fmdl naming hair and hair.dds
           WHEN the export is compiled for PES 21
           THEN both models in 71405's face package name hair in
                /Assets/pes16/model/character/common/714/sourceimages/, no hair.ftex is in slot
                05's texture folder, and shared_texture_overridden names hair and Faces/Round
TC-TEX-17  GIVEN slot 05 holding face_high.fmdl naming hair and hair.dds, Common/hair.dds, and
           slot 05 linking Faces/Round, whose folder holds hair_high.fmdl naming hair and
           hair.dds.common; and another export where slot 05 holds hair.dds.common in place
           of hair.dds
           WHEN each is compiled for PES 21
           THEN both models in 71405's face package name hair in
                /Assets/pes16/model/character/common/714/05 - A/sourceimages/, the CPK holds
                that directory's #windx11/hair.ftex, and shared_texture_overridden names hair
                and Faces/Round; in the second both name it in
                /Assets/pes16/model/character/common/714/sourceimages/, and no finding names
                the path
TC-TEX-18  GIVEN Common/skin.dds, and slot 05 holding face_high.fmdl and boots.fmdl (so his
           boots combine) and linking Faces/Round, holding hair_high.fmdl naming skin and
           skin.dds.common, and Boots/Crocs, holding boots.fmdl naming skin and skin.dds; and
           another export where Boots/Crocs holds skin.dds.common in place of skin.dds
           WHEN each is compiled for PES 21
           THEN shared_texture_conflict names skin on slot 05, and slot 05 is left out of the
                output; the second reports no conflict and keeps both packages, hair_high.fmdl
                and k0625's boots.fmdl naming skin in
                /Assets/pes16/model/character/common/714/sourceimages/, the CPK holding that
                skin.ftex and none in his texture home
TC-TEX-13  GIVEN slot 05 holding no model and an env.dds that is a 128x128 DXT5 cube map with
           eight mip levels (the bundled template's bytes); the textures task reads no model,
           and a Fox model converted for PES 17 would add its own findings to the run
           WHEN the export is compiled for PES 17, then for PES 21
           THEN PES 17 writes slot 05's env.dds in his texture home byte for byte as the source,
                PES 21 writes env.ftex there equal to ftex::dds_to_ftex of the source (an FTEX
                cube map, type 0xD), and no finding names the file in either run
TC-TEX-14  GIVEN Common/ holding hair.dds, a DXT1, and bumps.dds, a DX10 BC5_SNORM DDS (a signed
           block format no target keeps), or any other texture the deep pass refuses
           WHEN the export is compiled for PES 21
           THEN texture_codec_unsupported (or that refusal's finding) is reported naming
                bumps.dds and dropping that file alone, hair.ftex is in the team's Common
                output, and no folder_pack_failed is reported
TC-TEX-15  GIVEN slot 05 holding portrait.ftex, a 128x128 single-level FTEX (ftex::dds_to_ftex of
           a single-level BC3 DDS)
           WHEN the export is compiled for PES 21
           THEN his portrait DDS is BC3 at the source's size with the full mip chain down to 1x1
TC-TEX-19  GIVEN a model of slot 05's or of a folder he links naming hair in the team's Common
           directory (or the game's team-000 one), and Common/hair.dds
           WHEN the export is compiled for PES 21
           THEN hair is found in Common/: fmdl_texture_not_found is not reported, and hair.ftex
                is written in the team's Common output
TC-TEX-20  GIVEN slot 05's model holding a texture entry unused.dds in the team's Common
           directory (or the game's team-000 one) that no material uses and nothing supplies
           WHEN the export is compiled for PES 21
           THEN fmdl_texture_not_found is not reported for it, and the packed model's entry
                names the compiling team's Common directory
```

**Deep checks**

```
TC-CHK-01  GIVEN slot 05's boots.fmdl (or a shared folder's he links) holding a vertex 6000 units
           from the origin
           WHEN the export is checked, then compiled with pass_through on
           THEN check reports vertex_too_far_from_origin naming the file and exits 1 (a shared
                folder's: on the folder, his own reporting link_target_dropped); compile
                leaves the folder out of the CPK and reports it again
TC-CHK-02  GIVEN a solid .7z export whose model has the same vertex
           WHEN it is checked
           THEN vertex_too_far_from_origin is reported, as for the folder export
TC-CHK-03  GIVEN slot 05 holding a settings.toml with a key of the wrong type
           WHEN the export is compiled
           THEN settings_toml_invalid is reported, the folder's models compile, and the exit code
                is 1
TC-CHK-04  GIVEN p1/kit.dds of 4096x4096 and p2/kit.dds uncompressed (RGBA8)
           WHEN the export is compiled for PES 17, then for PES 21
           THEN on 17 kit_texture_too_big is reported for p1 and the kit dropped; on 21 p1
                compiles with no texture finding; p2 gets none on either, and its
                u0714p2.ftex on 21 is BC7
TC-CHK-05  GIVEN a root logo.png whose bytes are not a decodable image
           WHEN the export is checked
           THEN logo_file_invalid is reported and no logo is emitted when compiled
TC-CHK-06  GIVEN slot 05's boots.fmdl holding a mesh of 21846 faces, one over the hard limit
           WHEN the export is checked and compiled, then again with pass_through on
           THEN both report fmdl_mesh_over_face_limit as an Error naming the file; without
                pass_through the folder is left out of the CPK, with it the folder is compiled
TC-CHK-07  GIVEN slot 05's boots.fmdl holding bytes that are not a model
           WHEN the export is checked, then compiled with pass_through on
           THEN model_broken is reported naming the file and the reader's error, and the folder
                is left out of the CPK
TC-CHK-08  GIVEN slot 05's face_high.model holding a mesh over a hard .model limit (more
           vertices than the format can index)
           WHEN the export is checked, then compiled, for PES 17
           THEN model_mesh_over_vertex_limit is reported both times naming the file, and the
                folder is left out of the CPK
TC-CHK-09  GIVEN Common/ holding legs.model and a legs.mtl with ztest 0, slot 05 holding
           legs.model.common, and pass_through on
           WHEN the export is compiled for PES 17
           THEN mtl_state_invalid is reported as passed through, no model_material_undefined
                is reported, and slot 05's face CPK is written
```

**Pre-Fox XML and MTL checks**

```
TC-XML-01  GIVEN slot 05 holding face_high.model, hat_parts.model and its own face.xml with three
           entries: ./face_high.model typed face_neck; ./hat_parts.model typed "cape", with a
           <model> attribute "glow" and level="1"; and
           model/character/uniform/common/XXX/legs.model typed parts, which Common/ holds; a
           second export whose slot 05 face.xml names ./jessie/body/oral_x_*.model with
           ./jessie/body/x.mtl, both of which jessie/body/ holds, x.mtl naming skin.dds, and
           ./face_high.model typed face_neck with face_high.mtl, beside the root's skin.dds;
           and a third whose slot 05 holds the Fumos face.xml (the tracer_prefox fixture's
           Faces/XXX20 - Fumos/) with the four models and four .mtl files it names and
           skin.dds
           WHEN each is compiled for PES 17
           THEN the emitted xml keeps cape, glow and level="1" verbatim, with xml_type_unknown,
                xml_attribute_unknown (Warnings) and xml_level_lod (Info); the ./ references are
                written as they are and the face CPK holds face_high.model and hat_parts.model
                under those names; the Common path is written
                model/character/uniform/common/714/oral_legs_*.model, the name the Common output
                packs legs.model under; an entry with no material is written without one; the
                second's face CPK holds jessie/body/oral_x_win32.model and x.mtl at those paths,
                its xml naming them as written and x.mtl's skin the root's skin.dds; the third's
                holds the eight files, the four models as they are, its xml keeping the four
                entries in their
                order and its <dif> as written, xml_type_unknown naming boots
TC-XML-02  GIVEN the same folder with the face.xml removed
           WHEN it is compiled for PES 17
           THEN the generated face.xml lists face_high.model's emitted model as face_neck and
                hat_parts.model's as parts
TC-XML-03  GIVEN a face.xml with a <model> lacking path, and another whose ./hat.model names an
           absent file
           WHEN each export is checked for PES 17
           THEN xml_model_path_missing and xml_model_not_found each drop their folder
TC-XML-04  GIVEN a face.xml carrying a <dif> beside a face_diff.xml, and another with no face_neck
           entry (or a folder with no face.xml and no face_neck model)
           WHEN each is compiled for PES 17
           THEN the first reports xml_dif_conflict and is dropped; the second gains the dummy
                face_neck entry with xml_face_neck_added, its dummy model and mtl emitted
TC-XML-05  GIVEN a face.xml not listing hat.model, which sits in the folder
           WHEN the export is compiled for PES 17
           THEN xml_model_unlisted is reported and no model emitted from hat.model is in the face
                CPK
TC-XML-06  GIVEN a user face.xml in a folder compiled for PES 21
           WHEN it is compiled
           THEN xml_ignored_fox is reported and the models compile by the normal route
TC-XML-07  GIVEN a .mtl whose mesh-used material names a missing texture, and another whose
           missing texture is on a material no mesh uses
           WHEN each export is checked for PES 17 (the first, a .model's, also compiled for
                PES 21)
           THEN mtl_texture_not_found is a Warning on the first and mtl_texture_unused_missing
                an Info on the second, both folders kept (on PES 21 the converted model
                written)
TC-XML-08  GIVEN slot 05 holding face_high.model binding material "skin" and no readable .mtl
           defining that name (a Common one the check drops counts as none)
           WHEN the export is checked or compiled for PES 17
           THEN model_material_undefined is reported (naming skin, when a .mtl was read) and the
                folder is dropped
TC-XML-09  GIVEN slot 05 holding body_uniform.model and its own face.xml naming
           ./body_uniform.model
           WHEN the export is compiled for PES 16
           THEN xml_oral_prefix_missing is reported and the folder is dropped
TC-XML-10  withdrawn: a shared face folder's face.xml is no longer ignored (TC-XML-14)
TC-XML-11  GIVEN slot 05 holding face_high.model, face_high.mtl, a face.xml carrying a <dif> and
           a face_diff.bin
           WHEN the export is compiled for PES 17, then for PES 21
           THEN neither run reports xml_dif_conflict; the PES 17 face CPK's face.xml carries the
                xml's <dif> and no finding names face_diff.bin; the PES 21 face.fpk holds the
                folder's face_diff.bin byte for byte and xml_ignored_fox is reported
TC-XML-12  GIVEN slot 05 holding face_high.model, face_high.mtl, a legs.mtl defining none of
           Common/legs.model's materials, and a face.xml listing face_high and
           model/character/uniform/common/714/legs.model with material ./legs.mtl
           WHEN the export is compiled for PES 17
           THEN model_material_undefined is reported naming Common/legs.model, legs.mtl and
                the undefined names, and the folder is left out of the CPK
TC-XML-13  GIVEN slot 05 holding face_high.model, face_high.mtl, a face.xml carrying a <dif>
           and a face_diff.bin shorter than its counts
           WHEN the export is compiled for PES 17
           THEN no face_diff_invalid is reported and the face CPK's face.xml carries the
                xml's <dif>
TC-XML-14  GIVEN Faces/Round holding face_high.model, face_high.mtl and a face.xml naming
           face_high, slot 05 linking the folder and holding hair.model and hair.mtl
           WHEN the export is compiled for PES 17
           THEN no xml_ignored_shared is reported, and slot 05's face CPK holds face_high.model,
                hair.model and a face.xml whose entries are the shared xml's face_high
                entry, then the entry a generated xml gives hair.model, then the face_neck
                dummy; a file of his at a path a shared entry names (his face_diff for the <dif>)
                replaces the shared folder's under that entry
TC-XML-15  GIVEN Faces/Round holding face_high.model and face_high.mtl, slot 05 linking it and
           holding a face.xml naming ./face_high.model
           WHEN the export is compiled for PES 17, then again with pass_through on
           THEN both times xml_shared_face_conflict is reported on slot 05's folder, which is
                left out of the CPK
TC-XML-16  GIVEN slot 05's own face.xml (or a Faces/ folder's he links) naming
           ./pants_kitN.model with ./pants_kitN.mtl, the set's variants in his folder or the
           linked one, and Kits/p1 to p3
           WHEN the export is compiled for PES 17
           THEN the face.xml keeps that entry as the member wrote it, every numbered variant
                model and .mtl is packed under its own name, his own among a linked set's, and
                the set is completed against the export's kits as a generated entry's is
                (TC-CMN-17, TC-CMN-27), a single .mtl the entry names packed once, not copied
                per kit
TC-XML-17  GIVEN slot 05 holding jessie/body/x.model with jessie/body/x.mtl naming ./skin.dds, which
           jessie/ holds, and face_high.mtl naming ./shorts.dds, which only jessie/ holds
           WHEN the export is checked or compiled for PES 17
           THEN x.mtl names jessie/skin.dds, and mtl_texture_not_found (W) names ./shorts.dds, the
                folder kept: a model's .mtl and a .mtl's texture are found nearest first, from their
                own folder up, never in a subfolder the path does not name; a path into the team's
                Common directory is found in Common/ at that path or in an installed CPK listed
                before the compiled one
```

**Kits**

```
TC-KIT-10  GIVEN p1/ holding kit.dds and kit_mask.dds, compiled for PES 21
           WHEN it is compiled
           THEN kit_texture_not_used is reported once for p1 and no u0714p1_mask or _srm entry
                exists
TC-KIT-11  GIVEN p1/colors.txt holding two valid entries, p2/ holding kit.dds and no colors.txt,
           and a p3/ with no kit.dds (empty, or holding kit_back.dds alone)
           WHEN the export is compiled for PES 21
           THEN UniColor.bin carries p1's two colors at team 714's p1 entry, p2's entry holds the
                pair the dominant-color extraction gives for kit.dds with kit_colors_derived reported, and p3's
                the magenta/black pair with kit_placeholder and kit_colors_missing
TC-KIT-12  GIVEN p1/kit.dds and p1/colors.txt holding one valid entry and one unparsable line
           WHEN the export is compiled
           THEN color_entry_invalid is reported for the line, and the kit's colors are derived from
                its texture (kit_colors_derived)
TC-KIT-13  GIVEN a p1/icon_7 marker, and p2 without an icon marker
           WHEN the export is compiled
           THEN p1's UniColor entry carries icon 7 and p2's icon 3
TC-KIT-14  GIVEN a Midcup /co/ export with kits p1 and p2, p2's task failing (its kit.dds holds
           a whole header over pixel data cut short, which the deep pass passes)
           WHEN the export is compiled
           THEN p2 has no UniColor or UniformParameter entry change, and p1's entries are
                written
TC-KIT-15  GIVEN fpc_on in slot 05's folder, p1/config.toml without the FPC values and p2/ without
           a config
           WHEN the export is compiled for PES 21
           THEN both emitted configs carry the FPC values (shirt model, shorts model, collar,
                winter collar), kit_config_fpc_adjusted is reported for p1 and
                kit_config_generated for p2, and the export is compiled (Phase 3 refused fpc_on)
TC-KIT-16  GIVEN no fpc_on anywhere and p1/config.toml carrying the FPC values
           WHEN the export is compiled
           THEN the config is emitted unchanged and no kit_config_fpc_adjusted is reported
TC-KIT-17  GIVEN p1/config.toml whose name.y is 36
           WHEN the export is checked, then compiled, for PES 18
           THEN kit_config_version_clamped is reported both times with the maximum 33, and the
                emitted config holds 33
TC-KIT-18  GIVEN p1/ holding a 1024x1024 BC1 kit.dds drawn for the pre-Fox layout and the marker
           pre-fox
           WHEN the export is compiled for PES 21
           THEN kit_layout_converted is reported for p1 naming pre-fox to fox, the decoded top mip
                equals the no-marker compile's outside the sock rectangles, and each sock stripe's
                centre lands within 6 px (of 2048) of where the games' uniform models put it
TC-KIT-19  GIVEN the same kit compiled for PES 17, and a kit marked fox compiled for PES 17
           WHEN each is compiled
           THEN the first equals the no-marker compile, as a fox-marked kit compiled for PES 21
                does; the second is re-laid with the inverse table and reports
                kit_layout_converted naming fox to pre-fox
TC-KIT-20  GIVEN an empty p3/ marked pre-fox, and all/ holding a file named fox; and another
           export whose p1/, marked fox, holds kit_back.dds and kit_mask.dds and no kit.dds
           WHEN the first is compiled for PES 21, the second for PES 17
           THEN p3 compiles as without a marker (the placeholder is never re-laid), and
                kit_all_file_ignored is reported for all/fox; the second reports
                kit_layout_converted once for p1, whose mask alone is re-laid, its placeholder
                and kit_back as without the marker
TC-KIT-21  GIVEN p1/ holding kit.dds without kit_mask.dds
           WHEN the export is compiled for PES 17
           THEN u0714p1_mask.dds is emitted byte-identical to the bundled mask template, with no
                finding about the mask
TC-KIT-22  GIVEN p1/ holding kit.dds, kit_mask.dds and kit_srm.dds
           WHEN the export is compiled for PES 17, then PES 21
           THEN on 17 the mask is emitted, not re-laid, and the srm is not; on 21 the srm is
                emitted, not re-laid, and the mask is not, each time with kit_texture_not_used
                naming the map left out
TC-KIT-23  GIVEN p1/ and g1/ with configs
           WHEN the export is compiled for PES 17
           THEN the CPK holds the two 120-byte configs as loose files under
                common/character0/model/character/uniform/team/714/ and no UniformParameter.bin
TC-KIT-24  GIVEN p1/config.toml whose name.y is 30
           WHEN the export is compiled for PES 18, then for PES 21
           THEN neither run reports kit_config_version_clamped and the two emitted configs hold
                the same bytes at 0x1C and 0x1D, which decode to Name Y 30
TC-KIT-25  GIVEN Kits/all/kit.dds and an empty p2/
           WHEN the export is compiled for PES 21
           THEN p2's UniColor entry holds the pair the dominant-color extraction gives for
                all/kit.dds, with kit_colors_derived reported for p2; an all/kit.dds the texture
                checks refuse (a PNG named kit.dds) is reported at, and drops, p2 alone
TC-KIT-26  GIVEN p1/ holding kit.dds and a kit_back.dds column atlas (128x2048, ten flat-colored
           digit cells), and p2/ holding kit.dds and a kit_back.dds row atlas (2048x256)
           WHEN the export is compiled for PES 21, then for PES 17
           THEN on 21 p1's back atlas is a row of the ten colors in digit order and p2's is as
                given; on 17 p1's is as given and p2's is a column in digit order
TC-KIT-27  GIVEN p1/ holding kit.dds and a config.toml holding shirt = 144 where a [shirt] table
           belongs, and p2/ holding kit.dds; and another export whose p1/ holds kit.dds and a
           config.toml that is not valid TOML (shirt = [ unclosed)
           WHEN the first is checked, then compiled, and the second checked
           THEN kit_config_invalid is reported for p1 both times naming config.toml, p1 is left
                out and u0714p2 is compiled; the second's console reports kit_config_invalid as
                an Error, every console line a finding line (no bare line of the parser's
                message)
TC-KIT-28  GIVEN p1/ holding kit.dds, a kit_srm.dds whose sock islands differ from the rest, and
           the marker pre-fox
           WHEN the export is compiled for PES 21
           THEN the srm's decoded top mip equals the no-marker compile's outside the sock
                rectangles and differs inside them
TC-KIT-29  GIVEN p1/ holding kit.dds, a kit_mask.dds whose sock islands differ from the rest, and
           the marker fox
           WHEN the export is compiled for PES 17
           THEN the mask's decoded top mip equals the no-marker compile's outside the sock
                rectangles and differs inside them
TC-KIT-30  GIVEN p1/ holding kit.dds and kit_spec.dds
           WHEN the export is compiled for PES 21
           THEN kit_texture_not_used is reported naming kit_spec.dds and the CPK holds no texture
                of that stem
TC-KIT-31  GIVEN p1/ holding kit.dds and a kit_mask.dds whose header is a PNG's
           WHEN the export is compiled for PES 21
           THEN no texture finding names kit_mask.dds, kit_texture_not_used is reported
                naming it, and u0714p1.ftex is in the CPK
TC-KIT-32  GIVEN p1/config.toml holding shirt model 176 with short_sleeves = "cut-out", p2's
           model 176 with long_sleeves = "undershirt-only", p3's model 176 with tight = true,
           and p4's model 144 with all three
           WHEN the export is checked, then compiled, for PES 21
           THEN kit_config_option_ignored is reported both times for p1, p2 and p3, once each,
                naming the option (shirt.short_sleeves, shirt.long_sleeves, shirt.tight) and
                the model 176, nothing names p4 (with fpc_on, which makes its shirt model 176,
                its three are reported as theirs are), and p1's emitted config still holds the
                cut-out sleeves
TC-KIT-33  GIVEN a Midcup /co/ export whose p1/ holds kit.dds and colors.txt and no config.toml,
           p2/ holds colors.txt alone, and p3/ holds config.toml alone, the installed game
           holding team 714's configs and textures for p1 to p3
           WHEN the export is compiled for PES 21
           THEN neither kit_config_generated nor kit_placeholder is reported, the output holds
                p1's kit texture and p3's config and no other kit file, UniformParameter.bin's
                p1 and p2 entries are the installed ones, and UniColor.bin carries p1's and p2's
                colors at their entries and p3's installed one
TC-KIT-34  GIVEN a Midcup /co/ export whose p4/ holds kit.dds alone, the installed game holding
           team 714's configs for p1 to p3 and none for p4
           WHEN the export is compiled for PES 21
           THEN kit_config_generated is reported for p4 and the output holds p4's template
                config
TC-KIT-35  GIVEN the installed game holding team 714's p1 textures u0714p1, u0714p1_back and
           u0714p1_name, and a Midcup /co/ export whose p1/ holds kit.dds and config.toml
           WHEN the export is compiled for PES 21
           THEN the emitted p1 config names u0714p1, u0714p1_back and u0714p1_name, its chest
                and leg fields empty
TC-KIT-36  GIVEN a Midcup /co/ export whose p1/ holds kit.dds and config.toml and no
           colors.txt, and p2/ holds kit.dds and a colors.txt of one unparsable line, the
           installed game holding team 714's UniColor.bin entries for p1 and p2
           WHEN the export is compiled for PES 21
           THEN kit_colors_derived is not reported and UniColor.bin's p1 and p2 entries are the
                installed ones
```

**Root files, Common and collars**

```
TC-ROOT-06 GIVEN a root logo.png of 1000x600 pixels
           WHEN the export is compiled for PES 21
           THEN the CPK holds e_000714_r_ll.png, e_000714_r_l.png and e_000714_r.png under
                common/render/symbol/flag/, square at 512, 256 and 128 pixels, letterboxed with
                transparent borders, and logo_fit_applied names fit
TC-ROOT-07 GIVEN logo_crop.png of 1000x600, logo_stretch.png of 1000x600 in another export, and a
           300x300 logo.png in a third
           WHEN each is compiled
           THEN the first is center-cropped and the second resampled to square, each with
                logo_fit_applied naming its mode; the third reports logo_upscaled
TC-ROOT-08 GIVEN logo.png beside logo_small.png, and an export whose logo_small.png is undecodable
           WHEN each is compiled
           THEN the first's 128 PNG comes from logo_small and the larger two from logo; the second
                emits no logo at all and reports logo_file_invalid
TC-ROOT-09 GIVEN exports /co/ with notes.txt "Hello", /a/ with notes.txt, and /b/ skipped by
           players_txt_slot_duplicate with a notes.txt
           WHEN the root is compiled
           THEN output/teamnotes.txt holds /a/'s then /co/'s notes under team headers (canonical
                order), LF line ends, and nothing of /b/; a later run with no notes removes it
TC-ROOT-10 GIVEN a root colors.txt with valid entries, and another export without one
           WHEN each is compiled
           THEN TeamColor.bin carries the first's colors at team 714's entry; the second reports
                team_colors_missing (Info) and its entry is the installed or base value
TC-ROOT-11 GIVEN /co/ with notes.txt, and output/teamnotes.txt a folder
           WHEN the root is compiled
           THEN teamnotes_write_failed is reported, the run's CPK is in place and the folder is
                left as it was
TC-ROOT-12 GIVEN output/teamnotes.txt from an earlier run, and a root whose only export
           players_txt_slot_duplicate skips, with a notes.txt
           WHEN the root is compiled
           THEN no CPK is written and teamnotes.txt is left byte for byte as it was
TC-ROOT-13 GIVEN a root logo.png
           WHEN the export is compiled for PES 19
           THEN the CPK holds emblem_0714_r_ll.png, emblem_0714_r_l.png and emblem_0714_r.png
                under common/render/symbol/flag/ and no e_000714_* name
TC-ROOT-14 GIVEN an export in the old layout: root folders Faces, Boots, Kit Configs, Kit Textures
           and Portraits, no Players folder, named without a Full or Midcup tag
           WHEN it is checked, then compiled for PES 21
           THEN each command reports export_layout_old naming Kit Configs and nothing else about
                the export, it is skipped, and no CPK entry of its team is written
TC-CMN-01  GIVEN Collars/collar_12.fmdl, p1/config.toml and fpc_on in slot 05's folder (or, in a
           Midcup export, the collar alone)
           WHEN the export is compiled for PES 21
           THEN Asset/model/character/uniform/nocloth/#Win/collar_012.fmdl is in the CPK and every
                emitted kit config's collar and winter collar read 12, its other FPC values
                applied (the Midcup's: each config of a kit the team's record lists, nothing
                else changed)
TC-CMN-02  GIVEN Collars/collar_105.fmdl, Collars/neck.fmdl and Collars/collar_9999.fmdl
           WHEN the export is checked for PES 21
           THEN collar_id_invalid is reported for neck.fmdl and collar_9999.fmdl,
                collar_id_conflict for collar_105.fmdl (reserved for FPC), and each file is dropped
TC-CMN-03  GIVEN exports /a/ and /co/ each holding Collars/collar_12.fmdl, /co/ also p1/config.toml
           WHEN the root is compiled
           THEN /a/ keeps its collar (canonical order) and /co/ reports collar_id_conflict, its
                collar dropped and its configs not rewritten
TC-CMN-04  GIVEN slot 05's face model referencing pants_kitN, with pants_kit1.dds and pants_kit3.dds
           in the folder, and Kits/ holding p1, p2 and p3
           WHEN the export is compiled for PES 21
           THEN kit_variant_missing is reported for kit 2, the CPK holds pants_kit1.ftex,
                pants_kit3.ftex and pants_kitN.ftex (pants_kit1's conversion) and no
                pants_kit2.ftex, and the path table keeps the literal pants_kitN
TC-CMN-05  GIVEN a kit set's kit 1 and kit 2 models (pants_kit1.fmdl and pants_kit2.fmdl, or
           .model files, or boots_kit*) anywhere in slot 05's folder or the folders he combines
           WHEN the export is compiled for PES 21
           THEN kit_variant_model_left_out is reported and only kit 1's model is compiled
TC-CMN-06  GIVEN slot 05's face model referencing dummy_kit and dummy_kit_srm, or (PES 17) a .mtl
           naming a dummy_kit stem or a game path, neither present
           WHEN the export is checked, then compiled for PES 21 (the .mtl: checked for PES 17)
           THEN no texture-existence finding is reported either time, and the packed model's
                paths name dummy_kit and dummy_kit_srm verbatim
TC-CMN-07  GIVEN slot 05 holding, or linking a face folder holding, pants_kit1.model and
           pants_kit2.model with their .mtl files (or one .mtl they share)
           WHEN the export is compiled for PES 17
           THEN the face.xml holds one entry naming ./oral_pants_kitN_*.model with material
                pants_kitN.mtl (a shared .mtl, as it is), and beside it the variant models are
                packed as oral_pants_kit1_win32.model and oral_pants_kit2_win32.model, their
                .mtl files under their own names
TC-CMN-08  GIVEN Collars/collar_12.model, and another export holding Collars/collar_117.model
           WHEN each is compiled for PES 17
           THEN the first CPK holds collar_012.model under
                common/character0/model/character/uniform/nocloth/; the second reports
                collar_id_invalid (117 is not a PES 17 collar) and its file is dropped
TC-CMN-09  GIVEN Collars/collar_12.fmdl
           WHEN the export is compiled for PES 17
           THEN the CPK holds collar_012.model under
                common/character0/model/character/uniform/nocloth/ and no .fmdl or .mtl,
                its materials named uni_collar (the FMDL's first) and uni_shirts (the rest)
TC-CMN-10  GIVEN Collars/collar_12.fmdl and Collars/collar_12.dds
           WHEN the export is checked
           THEN file_type_disallowed is reported for collar_12.dds, which is dropped, and
                collar_12.fmdl is kept
TC-CMN-11  GIVEN Collars/collar_12.model beside a compiling player
           WHEN the export is compiled for PES 21
           THEN the CPK holds collar_012.fmdl under the Fox nocloth path, read with the
                templates' uniform.mtl and its materials the stock collars' Fox set by name
                (uni_collar pes_3ddf_collar, uni_shirts pes_3ddf_shirt_nb, each binding
                Pattern_Tex_LIN to the game's uni_pattern.dds), the kits' configs wear
                collar 12 and the player compiles
TC-CMN-12  GIVEN Common/x.glb (or Collars/collar_12.glb) beside a compiling player
           WHEN the export is compiled for PES 17 (the collar: or PES 21)
           THEN model_gltf_unsupported is reported on the file, which is dropped, and the
                player compiles (for the collar, the kit configs keep their own collars)
TC-CMN-13  GIVEN Common/sub/x.dds, which no link at any depth names
           WHEN the export is compiled for PES 21
           THEN file_not_used is reported on Common/sub/x.dds and the team's Common output
                holds no texture of stem x
TC-CMN-14  GIVEN Common/ holding boots.fmdl and a boots.model that does not parse, slot 05
           holding boots.model.common
           WHEN the export is compiled for PES 21
           THEN no finding names boots.model, no link_target_dropped is reported, and slot
                05's boots are compiled from Common/boots.fmdl; a Common/boots.fmdl the deep
                pass drops takes the .model it beats with it (common_model_beaten_dropped), and
                every player linking either is link_target_dropped
TC-CMN-15  GIVEN Common/sub/legs.model that does not parse, with or without a legs.fmdl it beats
           WHEN the export is compiled for PES 17
           THEN model_broken is reported on Common/sub/legs.model, which is dropped, and the
                team's Common output holds no model of stem legs, the beaten FMDL dropped with it
TC-CMN-18  GIVEN Common/refkit/oral_thigh_win32.model and Common/refkit/refkit.mtl (in a refs
           export, the referee template tree's), and slot 05's own face.xml naming
           model/character/uniform/common/XXX/refkit/oral_thigh_*.model with material
           .../common/XXX/refkit/refkit.mtl
           WHEN the export is compiled for PES 17
           THEN no xml_path_unchecked is reported, the team's Common output (the referee CPK)
                holds refkit/oral_thigh_win32.model and refkit/refkit.mtl, and the entry is
                written with the team's ID in place of XXX
TC-CMN-20  GIVEN Common/jessie/body.fmdl, and slot 05 holding face_high.fmdl and
           jessie/body.fmdl.common
           WHEN the export is compiled for PES 21, then for PES 17
           THEN on PES 21 the face package holds body.fmdl's meshes beside face_high.fmdl's
                and no file_not_used names Common/jessie/body.fmdl; on PES 17 the Common
                output holds jessie/oral_body_win32.model and face.xml lists it by that path
TC-CMN-21  GIVEN Kits/p1, p2 and p3, Common/kit1/armor_bsm.dds and Common/kit2/armor_bsm.dds,
           and slot 05 holding face_high.fmdl naming ./kitN/armor_bsm and
           kitN/armor_bsm.dds.common
           WHEN the export is compiled for PES 21
           THEN the face package's model names armor_bsm in
                /Assets/pes16/model/character/common/<team>/kitN/sourceimages/, the CPK
                holds Asset/model/character/common/<team>/kit1/, kit2/ and kitN/
                sourceimages/#windx11/armor_bsm.ftex, kitN's a copy of kit1's, and no
                kit3/ one, and kit_variant_missing names kit 3
TC-CMN-22  GIVEN Common/jessie/hair.dds, and slot 05 holding face_high.fmdl, jessie/hair_high.fmdl
           naming hair, and jessie/hair.dds.common
           WHEN the export is compiled for PES 21, then for PES 17
           THEN on PES 21 the hair model names hair in
                /Assets/pes16/model/character/common/<team>/jessie/sourceimages/, the CPK holds
                Asset/model/character/common/<team>/jessie/sourceimages/#windx11/hair.ftex, and
                neither file_type_disallowed nor file_not_used is reported; on PES 17 the
                converted model's .mtl names model/character/uniform/common/<team>/jessie/hair.dds,
                which the Common output holds
TC-CMN-23  GIVEN Common/jessie/hair_high.model with Common/jessie/hair_high.mtl naming
           ./sub/hair.dds, Common/jessie/sub/hair.dds and Common/jessie/hair.dds of other
           bytes, and slot 05 holding face_high.model and jessie/hair_high.model.common
           WHEN the export is compiled for PES 17
           THEN the .mtl face.xml names for the hair model points hair at
                model/character/uniform/common/<team>/jessie/sub/hair.dds, which the Common
                output holds, and no mtl_texture_not_found is reported
TC-CMN-24  GIVEN slot 05 holding ingame_face.toml, studs.dds and jessie/kit_boots.model.common,
           and Common/jessie/kit_boots.model with Common/jessie/kit_boots.mtl naming
           ./studs.dds and Common/jessie/studs.dds
           WHEN the export is compiled for PES 17
           THEN the boots.mtl in k0625's package names
                model/character/uniform/common/<team>/jessie/studs.dds, the Common output
                holds it, and no mtl_texture_not_found is reported
TC-CMN-25  GIVEN Common/jessie/body.fmdl naming hair, Common/jessie/hair.dds and Common/hair.dds
           of other bytes, and slot 05 holding face_high.fmdl and jessie/body.fmdl.common; a
           second export whose slot 05 holds face_high.model, a face_high.mtl naming skin by
           the pre-Fox Common path model/character/uniform/common/000/jessie/skin.dds, and
           jessie/skin.dds.common, beside Common/jessie/skin.dds; and a third whose slot 05
           holds his own face_high.model with a face_high.mtl naming ./skin.dds, beside
           Common/skin.dds
           WHEN each is compiled for PES 21
           THEN the first's face package's model built from body.fmdl names hair in
                /Assets/pes16/model/character/common/<team>/jessie/sourceimages/, the CPK
                holds Asset/model/character/common/<team>/jessie/sourceimages/#windx11/hair.ftex
                converted from Common/jessie/hair.dds, and no file_not_used names it; the
                second's face_high.fmdl names skin in that same folder; the third reports
                mtl_texture_not_found for ./skin.dds (his own model's texture comes from
                Common/ only through a link or a Common path, as in the second) and exits 0
TC-CMN-19  GIVEN Collars/collar_12.fmdl with two materials
           WHEN the export is compiled for PES 16
           THEN the CPK's collar_012.model names uni_shirts alone
TC-CMN-16  GIVEN Collars/sub/collar_12.model and strict_file_type_check off
           WHEN the export is compiled for PES 17
           THEN file_type_disallowed is reported as Info naming it, the CPK holds no
                collar_012.model, and the team's kit configs keep their collars
TC-CMN-17  GIVEN Kits/p1, p2 and p3, and slot 05 holding pants_kit1.model with pants_kit1.mtl
           and pants_kit2.model with pants_kit2.mtl; and another export with Kits/p1 and p2,
           whose slot 05 holds pants_kit1.model, pants_kit2.model, socks_kit1.model and
           socks_kit3.model, with pants.mtl and socks.mtl
           WHEN each is compiled for PES 17
           THEN kit_variant_missing is reported on slot 05's folder for model pants_kitN, kit
                3, copied pants_kit1, and the face CPK holds the kit 3 spellings of
                pants_kit1's packed model and .mtl with their bytes, beside the one
                pants_kitN entry; the second reports kit_variant_missing for socks_kitN alone,
                kit 2, copied socks_kit1 (each set completed against its own kit numbers),
                and its face CPK holds pants kits 1 and 2 and socks kits 1, 2 and 3
TC-CMN-26  GIVEN slot 05's face model referencing pants_kitN, with pants_kit1.dds and
           pants_kitF.dds of other bytes in the folder, and Kits/ holding p1, p2 and p3
           WHEN the export is compiled for PES 21
           THEN no kit_variant_missing is reported, the CPK holds pants_kit1.ftex and
                pants_kitN.ftex, pants_kitF's conversion, and no pants_kit2, pants_kit3 or
                pants_kitF file
TC-CMN-27  GIVEN Kits/p1, p2 and p3, and slot 05 holding pants_kit1.model with pants_kit1.mtl
           and pants_kitF.model with pants_kitF.mtl
           WHEN the export is compiled for PES 17
           THEN no kit_variant_missing is reported, the face CPK holds the kit 2 and kit 3
                spellings of pants_kitF's model and .mtl with their bytes beside the one
                pants_kitN entry, and no pants_kitF file
TC-CMN-28  GIVEN slot 05 holding pants_kit1.fmdl and pants_kitF.fmdl
           WHEN the export is compiled for PES 21
           THEN kit_variant_model_left_out is reported with used pants_kitF.fmdl, and the face
                package's model is built from pants_kitF alone
TC-CMN-29  GIVEN Kits/p1 and p2, and slot 05 holding kit1/armor_bsm.dds, kitF/armor_bsm.dds of
           other bytes, and face_high.fmdl naming ./kitN/armor_bsm
           WHEN the export is compiled for PES 21
           THEN no finding names armor_bsm, the face package's model names armor_bsm in the
                folder's kitN/sourceimages/ directory, and the CPK holds the folder's kit1/
                and kitN/ armor_bsm.ftex, kitN's kitF's conversion, and no kitF/ one
TC-CMN-30  GIVEN slot 05 holding kitF/armor_bsm.dds and face_high.fmdl naming ./kitF/armor_bsm
           WHEN the export is compiled for PES 21
           THEN the face model's texture armor_bsm is reported not found
TC-CMN-31  GIVEN Kits/p1 and p2, Common/kit1/armor_bsm.png of 3x3 pixels and
           Common/kit2/armor_bsm.dds, and slot 05 holding face_high.fmdl naming
           ./kitN/armor_bsm and kitN/armor_bsm.dds.common
           WHEN the export is compiled for PES 21
           THEN texture_too_small names Common/kit1/armor_bsm.png, no link_target_dropped is
                reported, and slot 05's face package is in the CPK
TC-CMN-32  GIVEN slot 05 holding pants_kit1.model with pants_kit1.mtl, and pants_kit2.model whose
           .mtl search finds materials.mtl
           WHEN the export is compiled for PES 17
           THEN the face.xml lists the set once, by its lowest variant wherever it sorts, naming
                that variant's .mtl (a kit token in it respelled kitN), and
                kit_variant_mtl_differs (W) names pants_kit2.model with the .mtl found and the
                one the entry gives it
TC-CMN-33  GIVEN Collars/collar_12.fmdl whose conversion to .model fails (a material name
           a .mtl cannot hold)
           WHEN the export is compiled for PES 17
           THEN model_conversion_failed names the collar, the CPK holds no collar, and the
                team's kit configs still wear collar 12
TC-CMN-34  GIVEN a Common/boots.mtl that does not parse, linked by slot 05's
           boots.mtl.common beside his boots.fmdl, or beside his boots.model
           WHEN the export is checked or compiled for PES 21
           THEN a Common .mtl is read only when a .model's material search reaches it:
                beside boots.fmdl nothing is reported; beside boots.model mtl_broken drops
                it and model_material_undefined drops slot 05
TC-CMN-35  GIVEN Common/spare.fmdl and Common/spare.skl, which no link names
           WHEN the export is compiled for PES 21
           THEN nothing is built from them and no finding reports them unused
```

**Bins**

```
TC-BIN-01  GIVEN a /co/ export with kits p1 and p2, compiled with no PES install configured
           WHEN it is compiled for PES 21
           THEN the CPK holds UniColor.bin, TeamColor.bin and UniformParameter.bin built on the
                bundled bases, with only team 714's entries changed
TC-BIN-02  GIVEN TC-SRC-01's export (empty kit p2/)
           WHEN it is compiled
           THEN UniColor.bin carries p2's entry with the magenta/black pair and icon 3
TC-BIN-03  withdrawn: TC-KIT-14 proves the same behavior on the same input
TC-BIN-04  GIVEN a /co/ export with p1, a root colors.txt, and Boots/Crocs/ (boots.model and
           boots.mtl) linked by slot 05
           WHEN it is compiled for PES 17
           THEN UniColor.bin and TeamColor.bin are written with team 714's entries set, and no
                UniformParameter.bin, PlayerAppearance.bin, BootsList.bin or GloveList.bin is
                written
TC-BIN-05  GIVEN a PES folder whose download/DpFileList.bin lists 4cc_08_bins, 4cc_61_midcup and
           4cc_99_test, 4cc_08_bins.cpk holding a UniColor.bin with team 714's p1 entry set to A and
           4cc_61_midcup.cpk one with it set to B
           WHEN a Midcup /co/ export with only p2/ is compiled with cpk_name 4cc_99_test, then with cpk_name
           4cc_61_midcup
           THEN the first run's UniColor.bin carries B at p1 (the highest-priority CPK below the
                output's) and the second run's carries A (the midcup CPK and everything above it
                skipped, as is an installed refs CPK the run also writes), both with p2 set;
                bin_source names the supplying CPK for each bin
TC-BIN-06  GIVEN an installed UniColor.bin whose team 714 record holds kits p1, p2 and p3, an
           installed UniformParameter.bin holding team 714's p1 entry without the FPC
           values (shirt model 144) and no p3 entry, and a Midcup
           /co/ export with fpc_on and only p2/
           WHEN it is compiled for PES 21
           THEN the emitted UniformParameter.bin's p1 entry carries the FPC values with
                kit_config_fpc_adjusted reported for slot p1, and p3, which has no entry, reports
                kit_config_fpc_unpatched
TC-BIN-07  GIVEN a templates/UniColor.bin (or a file at a referee template tree's path, with a
           refs export) in the data directory, then also a templates/TeamColor.bin that cannot
           be read
           WHEN a from-scratch compile of a /co/ export (of the refs export: for PES 21 or
                PES 17) runs each time
           THEN the first reports template_override_active naming the file, and its CPK's
                UniColor.bin is the template with team 714's entries set (the referee CPK's file
                at that path, its bytes); the second reports template_override_unreadable naming
                TeamColor.bin and aborts with exit 3
TC-BIN-08  GIVEN an installed CPK whose UniColor.bin is WESYS-wrapped
           WHEN a compile reads it
           THEN the emitted UniColor.bin is plain and carries the installed entries
TC-BIN-09  GIVEN a PES folder with no DpFileList.bin
           WHEN a compile runs
           THEN the bins are built on the bundled bases, dpfilelist_missing is reported, and the
                CPK is promoted to output/
TC-BIN-21  GIVEN TC-BIN-05's install with 4cc_61_midcup.cpk replaced by bytes that are not a CPK,
           or by one whose UniColor.bin does not parse
           WHEN a /co/ export is compiled with cpk_name 4cc_99_test
           THEN installed_bin_unreadable is reported naming 4cc_61_midcup.cpk, no export is
                read, no CPK is written, and the exit code is 3
TC-BIN-10  withdrawn: restated as TC-BIN-25 (`spec/team_compiler/README.md`), which also
           gives the compiled player's gloves and appearance rows
TC-BIN-11  GIVEN slot 05 holding boots.fmdl whose task fails, and an installed pair (71405, 7)
           WHEN the export is compiled for PES 21
           THEN BootsList.bin keeps (71405, 7)
TC-BIN-12  GIVEN slot 05 holding glove_l.fmdl and glove_r.fmdl, slots 07 and 08 linking
                Gloves/Keeper/, and slot 09 alone linking Gloves/Grip/
           WHEN the export is compiled for PES 21
           THEN GloveList.bin holds (71405, 625), (71407, 644), (71408, 644) and (71409, 629)
TC-BIN-13  GIVEN an installed TeamColor.bin whose record for team 799 starts with color bytes in
           place of its team ID and color count
           WHEN a /co/ export is compiled
           THEN the emitted TeamColor.bin's record for team 799 starts with team ID 799 and
                count 4, every other installed record is unchanged, and bin_header_repaired
                is reported once, naming TeamColor.bin and team 799
TC-BIN-14  GIVEN a Full /co/ export with kits p1, p2, p3 and g1, p3's task failing (its kit.dds
           holds a whole header over pixel data cut short), and no PES install configured (the bundled
           UniColor.bin's record for team 714 holds a past cup's eight kits: 0 to 6 and 0x10)
           WHEN it is compiled for PES 21
           THEN team 714's UniColor.bin record has a kit count of 4 and holds p1's, p2's and
                g1's new entries and p3's base entry, every other entry unused
TC-BIN-15  GIVEN a Midcup /co/ export with kit p1 and a kit number the bundled base's record for
           team 714 does not hold, and no PES install configured
           WHEN it is compiled for PES 21
           THEN team 714's record keeps the base's other entries, p1's is replaced, the new
                kit's is added, and the count is one more than the base's
TC-BIN-16  GIVEN an installed UniformParameter.bin holding team 714's p1, p2 and p3 configs and
           team 702's p1, and a Full /co/ export with p1 only
           WHEN it is compiled for PES 21
           THEN the emitted UniformParameter.bin holds team 714's p1 config and neither its p2
                nor its p3, and team 702's p1 unchanged
TC-BIN-17  withdrawn: restated as TC-BIN-26 (`spec/team_compiler/README.md`), a Full export's
           markerless player now getting the default ID rather than losing his row
TC-BIN-18  GIVEN an installed CPK holding team 714's loose p1 kit config without the FPC values
           (shirt model 144), and a Midcup /co/
           export with fpc_on and only p2/
           WHEN it is compiled for PES 17
           THEN the CPK holds p1's config re-emitted with the FPC values, with
                kit_config_fpc_adjusted reported for slot p1, and p3, which has no installed
                config, reports kit_config_fpc_unpatched; a p1 config already carrying the FPC
                values in the nearest installed CPK is left as it is, and a Full export
                re-emits no config of a kit it does not hold
TC-BIN-19  GIVEN a Full /co/ export with players and no Kits/ folder, and no PES install
           configured
           WHEN it is compiled for PES 21
           THEN it compiles as with empty p1/ and g1/: kit_placeholder is reported for both,
                the CPK holds their placeholder kits and template configs, and team 714's
                UniColor.bin record has a kit count of 2, holding p1's and g1's entries
TC-BIN-20  GIVEN a Full /co/ export with kits p1 and p2 and no g1/, and a Midcup /a/ export with
           p1 only
           WHEN they are compiled for PES 21
           THEN /co/ gets g1 as an empty g1/ (kit_placeholder for g1; its UniColor.bin record
                has a kit count of 3) and /a/ gets no placeholder kit
TC-BIN-22  withdrawn: restated as TC-BIN-27 (`spec/team_compiler/README.md`), which names
           each of the three tables
TC-BIN-23  GIVEN an installed UniColor.bin whose team 714 record holds kits p1, p2 and p3, an
           installed UniformParameter.bin holding team 714's p1 entry and no p3 entry, and
           a Midcup /co/ export with Collars/collar_12.fmdl (or .model), only p2/ and no fpc_on
           WHEN it is compiled for PES 21, or for PES 17 over an installed loose p1 config
           THEN p1's emitted config (its UniformParameter.bin entry, or on PES 17 its loose
                file) wears collar 12 as its collar and winter collar, and p3, which has no
                entry, reports kit_config_collar_unpatched
TC-BIN-24  GIVEN an installed CPK holding team 714's loose p1 kit config without the FPC values
           (shirt model 144) and with Name Y 40, and a Midcup /co/ export with fpc_on and
           only p2/
           WHEN it is compiled for PES 17
           THEN the CPK holds p1's config re-emitted with the FPC values and Name Y 33,
                and kit_config_version_clamped is reported on the export naming slot p1,
                the field, 40 and 33
```

**Planning**

```
TC-PLN-01  GIVEN /co/ with slot 05 holding boots.fmdl and slot 23 holding glove_l.fmdl
           WHEN the export is compiled twice
           THEN both runs write k0625 and g0643 and the two CPKs are byte-identical
TC-PLN-02  GIVEN Boots/Zebra/ and Boots/Apple/, each linked by two players
           WHEN the export is compiled, then with a third folder Boots/Mango/ added
           THEN Apple is k0644 and Zebra k0645 the first time; the second time Apple k0644, Mango
                k0645, Zebra k0646
TC-PLN-03  GIVEN exports "co Full A" (a folder) and "co Full B.zip", both resolving to 714, beside a
           valid /a/ export
           WHEN the root is compiled
           THEN duplicate_aesthetics_export is reported for each of the two, neither is compiled,
                /a/ is, and the exit code is 1
TC-PLN-04  GIVEN overrides/common/etc/TeamColor.bin in the data directory, and
           overrides/Asset/model/character/boots/k0625/#Win/boots.fpk
           WHEN a /co/ export whose slot 05 has boots is compiled (or a root holding no export)
           THEN overrides_active is reported, the CPK's two entries hold the override bytes,
                duplicate_path is a Warning naming each path the run also compiled, and the
                export's own boots task is not dropped (with no export, the CPK holds the
                overrides and the bins alone)
TC-PLN-05  GIVEN exports /a/ and /co/
           WHEN the root is checked or compiled with one thread_count or memory_cap_percent,
                then another
           THEN the two runs' findings are identical and their CPKs byte-identical
TC-PLN-06  GIVEN an export whose boots.fmdl is replaced after planning and before its task reads it
           WHEN it is compiled
           THEN source_changed_during_run is reported, the run aborts with exit 3, no CPK is
                written and a previous one is byte-identical
TC-PLN-07  GIVEN a root with /co/ skipped by duplicate_aesthetics_export and /a/ valid
           WHEN it is compiled
           THEN /a/'s CPK content equals a compile of /a/ alone
TC-PLN-08  GIVEN the tracer export as a .7z archive, and a memory_cap_percent whose cap is
           smaller than the archive's decompressed size
           WHEN it is compiled for PES 21
           THEN the run ends with exit code 0 and no finding of the cap, and the CPK holds
                player 79205 and kit u0792g1
```

**Referees**

```
TC-REF-01  GIVEN a refs export whose players.txt maps "Ref A" (face_high.fmdl, boots.fmdl, skin.dds)
           to slots 01, 20 and 35
           WHEN the root is compiled for PES 21 with refs_cpk_name 4cc_18_referees
           THEN 4cc_18_referees.cpk holds the referee face package of slots 01, 20 and 35, boots
                folders k9901, k9920 and k9935, one skin.ftex under common/999/Ref A/sourceimages/,
                and the team CPK holds no 999 path
TC-REF-02  GIVEN the same refs export and a /co/ export in one root
           WHEN the root is compiled
           THEN two CPKs are written, 4cc_99_test.cpk without referee content and
                4cc_18_referees.cpk without /co/'s, and both are deployed or promoted together
TC-REF-03  GIVEN a refs export with Faces/Base/ linked by Ref A and Ref B
           WHEN it is compiled for PES 21
           THEN each referee's face package holds the merged face and Base has no output of its
                own
TC-REF-04  GIVEN a refs export with ref_marker.dds
           WHEN it is compiled for PES 17
           THEN the referee CPK holds the marker texture, converted, as the template tree's
                common/character1/model/character/parts/referee/incom_bsm.dds in place of the
                tree's file and nowhere else (no copy under common/999/), no referee_collar_077
                pair and no collar_077.model, every other tree file as the template's, and loose
                referee kit configs under common/character0/model/character/uniform/team/referee/
                equal to the template's; no team CPK is written
TC-REF-05  GIVEN a refs export whose players.txt maps Ref A to slots 01 and 20, compiled with
           --mode test
           WHEN it runs
           THEN output/test_output/<refs source key>/Players/Ref A/ holds the processed files once,
                not per slot
TC-REF-06  GIVEN a refs export with ref_marker.dds, and a PES folder with dt00_x64.cpk
           WHEN it is compiled for PES 21
           THEN the referee CPK holds the marker model as collar_077.fmdl, the marker's FTEX
                under common/999/sourceimages/, and TC-REF-04's loose referee kit configs whose
                collar is 77;
                dt00_x64.cpk is byte-identical and the exit code is 0
TC-REF-07  GIVEN a /co/ export holding Collars/collar_77.fmdl, p1/config.toml with collar 77 and
           p2/config.toml with winter collar 77
           WHEN it is compiled for PES 21
           THEN collar_id_conflict is reported and the file dropped, and kit_collar_reserved is
                reported for p1 and for p2, each kit dropped
TC-REF-08  GIVEN a refs export with no ref_marker.dds, or one texture_codec_unsupported drops
           WHEN it is compiled for PES 21 or PES 17
           THEN the referee CPK holds no marker model (on PES 17, none of TC-REF-04's) and no
                marker texture (on PES 17, the template's prop texture at its path), its kit
                configs keep the template's collar, and on PES 21 the team CPK's
                UniformParameter.bin entries of their names equal them
TC-REF-09  GIVEN TC-REF-01's refs export
           WHEN the root is compiled for PES 17
           THEN the referee CPK holds common/character0/model/character/face/real/referee001.cpk,
                referee020.cpk and referee035.cpk, each face.xml holding Ref A's boots model as a
                parts entry, no boots folder k99NN (the template tree's k0062 alone under
                common/character0/model/character/boots/), and one skin.dds under
                common/character1/model/character/uniform/common/999/Ref A/
TC-REF-10  GIVEN a refs export whose players.txt maps Ref A to slots 01 and 20, Ref A holding
           only a link to the shared folder Boots/Studs, which holds boots.fmdl
           WHEN the root is compiled for PES 21
           THEN the referee CPK holds boots folders k9901 and k9920, each with the shared
                boots model, no other boots folder, and no face/real/referee001 or
                referee020 folder
TC-REF-11  GIVEN a refs export holding Kits/p1 with kit.dds beside a mapped referee folder holding
           a face model
           WHEN the root is compiled for PES 21 (or PES 17)
           THEN file_not_used is reported naming Kits/p1 as a Warning, the referee CPK holds no
                kit, its kits give no kit number to complete a referee's texture set against,
                and the referee face compiles; file_not_used likewise names a refs export's
                logo, portraits and collars, and a referee folder's textures or texture links
                when it holds no model
TC-REF-12  GIVEN a refs export with ref_marker.dds
           WHEN it is compiled for PES 21 with --no-deploy
           THEN the team CPK (with multicpk_mode on, the bins CPK) is written holding
                UniformParameter.bin, whose entries named as the 20 loose referee kit configs
                (referee_ACL_1.bin to referee_SDA_3.bin) equal the referee CPK's loose files
                byte for byte, each with collar 77, and whose other entries equal the installed
                or bundled bin's; the team CPK holds no 999 path and no loose referee config;
                compiled for PES 17, no team CPK is written
TC-REF-13  GIVEN TC-REF-10's refs export (Ref A in slots 01 and 20, holding only a link to the
           shared folder Boots/Studs, which holds boots.fmdl)
           WHEN the root is compiled for PES 17
           THEN the referee CPK holds boots folders k9901 and k9920, each with the shared boots
                model converted, and no referee001.cpk or referee020.cpk: the link is not
                copied into a face, and a referee folder with no face model gets no face
                folder, the game's referee head staying (a team player's gets a blank one)
TC-REF-14  GIVEN a refs export whose root colors.txt holds a line that does not parse
           WHEN the export is checked
           THEN no color_entry_invalid is reported
TC-REF-15  GIVEN a refs export whose Common/refkit/ holds a texture.dds other than the template's
           WHEN it is compiled for PES 17
           THEN the referee CPK's common/999/refkit/texture.dds is the export's, and the
                template's other refkit files are beside it
TC-REF-16  GIVEN a refs export whose Ref A holds face_high.model, face_high.mtl and fpc_off
           WHEN it is compiled for PES 17
           THEN his face.xml lists his own entries (face_high, or his own face.xml's, as written),
                then the ten refkit body entries in the table's order but any already listed,
                with material model/character/uniform/common/999/refkit/refkit.mtl, then nothing
                else but the <dif>; a team player holding the same gets no refkit entry
TC-REF-17  GIVEN TC-REF-16's Ref A also holding boots or a glove of his own (a model named for
           them anywhere in his folder, or a link)
           WHEN it is compiled for PES 17
           THEN the refkit entries leave out oral_boots_*.model for his boots, both hands for a
                glove, and his own stand in their place
TC-REF-18  GIVEN a refs export whose Ref A holds face_high.model and boots/boots.model, and a
           team export whose slot 05 holds the same two files
           WHEN both are checked
           THEN player_layout_proto is reported on Ref A naming boots (context folder) with the
                hint naming the Export upgrader and Ref A is dropped, while slot 05 is valid
                with boots/ a subfolder of his own
```

**Output modes, deployment, multi-CPK**

```
TC-OUT-07  GIVEN the tracer export; an export whose slot 05 holds face_high.fmdl naming hair,
           hair.dds, and jessie/hair_high.fmdl naming hair with jessie/hair.dds; and an export
           with Kits/p1,
           p2 and p3, whose slot 05's face_high.fmdl names armor_bsm.dds at ./kitN/ and links
           it with kitN/armor_bsm.dds.common, and whose Common/ holds kit1/armor_bsm.dds and
           kit2/armor_bsm.dds
           WHEN compile --mode test runs on each, the tracer's with multicpk_mode on and off
           THEN output/test_output/ is replaced whole, and its egg Midcup Tracer/Players/05 - The
                Chad Stormworks Player/ (an archive source's folder named with its extension)
                holds fcl_hair.fmdl with its texture path rewritten, shirt.ftex, face_diff.bin,
                fcl_hair_sim.fclo and fcl_hair_sim.skl, no .fpk exists under test_output/, the
                bins are written under test_output/_bins/ at their game-relative paths, and the
                PES folder is untouched, output/ holding test_output/ and aesthetics_patch.toml
                alone, the same files either way multicpk_mode is set; the other two exports'
                textures land at their own folders below their export folders (Players/05 -
                A/hair.ftex and jessie/hair.ftex; Common/kit1/ and kit2/armor_bsm.ftex, and
                Common/kitN/armor_bsm.ftex holding kit1's, the lowest kit's)
TC-OUT-08  withdrawn: two sources of one team are duplicate_aesthetics_export before any output
           (TC-PLN-03), so their test_output folders cannot collide
TC-OUT-09  GIVEN a PES folder with download/DpFileList.bin and livecpk/old.txt,
           overrides/common/etc/TeamColor.bin in the data directory, the tracer export and a
           refs export
           WHEN compile --mode sideload runs for PES 21
           THEN livecpk/ holds exactly the files the normal-mode CPKs (the team's and the refs')
                hold, at the same relative paths with the same bytes
                (livecpk/common/etc/TeamColor.bin the override's, with overrides_active and
                duplicate_path reported), old.txt is gone, download/ is untouched, and the exit
                code is 0
TC-OUT-10  GIVEN pes_version 15, then 16, then 17, or a pes_folder_path naming no folder
           WHEN compile --mode sideload runs
           THEN 15, 16 and the missing folder are refused as an invalid configuration naming the
                version or the folder with exit code 2 and nothing runs; 17 writes livecpk/
TC-OUT-11  GIVEN compile --mode test with a kit and a root notes.txt
           WHEN it runs
           THEN output/teamnotes.txt is written and the kit's UniColor entry is in the UniColor.bin
                under test_output/_bins/
TC-OUT-12  GIVEN multicpk_mode on, exports /a/, /b/ and /co/, cpk_part_max_size just above the
           compiled size of /a/ and /b/ together, and a DPFL reserving 4cc_41_teams to 4cc_45_teams
           WHEN the root is compiled
           THEN 4cc_41_teams.cpk holds /a/ and /b/ whole, 4cc_42_teams.cpk holds /co/,
                4cc_43_teams.cpk to 4cc_45_teams.cpk are each the 6,272-byte placeholder, and
                4cc_08_bins.cpk holds the bins only
TC-OUT-13  GIVEN TC-OUT-12's setup with cpk_part_max_size below one team's size
           WHEN the root is compiled
           THEN cpk_team_exceeds_cap is reported, the run aborts with exit 3 and no part is written
TC-OUT-14  GIVEN TC-OUT-12's setup with a DPFL reserving one teams slot
           WHEN the root is compiled
           THEN cpk_slots_exhausted names the shortfall, the run aborts with exit 3 and no part is
                written
TC-OUT-15  GIVEN multicpk_mode off and cpk_part_max_size set below the single CPK's size
           WHEN the root is compiled
           THEN cpk_size_over_limit is a Warning, the CPK is written whole and the exit code is 0
TC-OUT-16  GIVEN teams_cpk_name teams2 and a DPFL reserving 4cc_51_teams2 and 4cc_41_teams
           WHEN a multi-CPK compile runs
           THEN of the teams parts only 4cc_51_teams2.cpk is written, and 4cc_41_teams is not
                touched
TC-OUT-17  GIVEN overrides/common/etc/TeamColor.bin and the tracer export
           WHEN compile --mode test runs
           THEN test_output/_bins/common/etc/TeamColor.bin is the compiled one, not the
                override's bytes, and overrides_active is not reported
TC-OUT-18  GIVEN a PES folder with no DpFileList.bin and the tracer export
           WHEN compile --mode test runs, then compile --mode sideload
           THEN each reports dpfilelist_missing as a Warning and bin_source naming bundled for
                each bin, and exits with code 0
TC-DEP-01  GIVEN a PES folder with PES2021.exe, download/DpFileList.bin listing 4cc_99_test (with
           multicpk_mode on, every CPK the run writes) and an old download/4cc_99_test.cpk
           WHEN the tracer is compiled for PES 21
           THEN download/4cc_99_test.cpk holds the run's CPK (with multicpk_mode on, download/
                holds each CPK of the run), no .cpk.partial remains, nothing else is added to
                download/, output/ holds no CPK and no .staging/, and the exit code is 0
TC-DEP-02  GIVEN the same with the old CPK held open without delete sharing
           WHEN it is compiled
           THEN old_cpk_locked is reported, output/4cc_99_test.cpk holds the run's CPK, the old
                download CPK is byte-identical, and the exit code is 1
TC-DEP-03  GIVEN pes_folder_path naming a folder that does not exist
           WHEN it is compiled
           THEN pes_folder_not_found is reported, the CPK is promoted to output/, the bins come from
                the bundled bases, and the exit code is 1
TC-DEP-04  GIVEN a PES folder whose download/ denies writes
           WHEN compile starts
           THEN deploy_target_unwritable is reported before any export is read, the run still
                compiles and promotes to output/, and the exit code is 1
TC-DEP-05  GIVEN a DPFL that lists neither cpk_name nor any bundled name for it
           WHEN it is compiled
           THEN cpk_name_unlisted is reported and the CPK is promoted
TC-DEP-06  GIVEN a PES folder holding PES2019.exe while pes_version is 21
           WHEN it is compiled
           THEN pes_version_mismatch is a Warning and deployment proceeds
TC-DEP-07  GIVEN output/.staging/<a dead run's folder> left from a killed process
           WHEN compile starts
           THEN that folder is removed and the run's own staging is created and removed as usual
TC-DEP-08  GIVEN an installed DPFL with no 4cc_NN_teams entry (the pre-teams layout) and
           multicpk_mode on
           WHEN the root is compiled
           THEN dpfilelist_outdated is reported naming upgrade-dpfl, every CPK is promoted to
                output/, and the exit code is 1; so is it for any list lacking a CPK of the run
                that the official list names (no entry for cpk_name)
TC-DEP-09  GIVEN the same install with a 1 KiB download/4cc_40_faces.cpk
           WHEN upgrade-dpfl runs without --yes
           THEN 4cc_40_faces is printed as renamed to 4cc_41_teams, DpFileList.bin is
                byte-identical, no file is renamed, and the exit code is 0
TC-DEP-10  GIVEN the same, or a DpFileList.bin that is missing or does not parse
           WHEN upgrade-dpfl --yes runs
           THEN DpFileList.bin equals the bundled official list, DpFileList.bin.bak equals the old
                file, and 4cc_40_faces.cpk is now 4cc_41_teams.cpk with its bytes unchanged
                (when the list was missing or did not parse, nothing is renamed, and nothing is
                backed up when there was none)
TC-DEP-11  GIVEN multicpk_mode on and a refs export beside two teams
           WHEN the root is compiled with an old 4cc_42_teams.cpk locked, or the copy of one CPK
                failing
           THEN old_cpk_locked (deploy_target_unwritable, for a failed copy) is reported, every
                CPK of the run (parts, placeholders, bins, refs) is promoted to output/ and none
                replaces an installed one
TC-DEP-12  GIVEN an installed DPFL holding the official list's entries plus 4cc_80_mine, and
           another holding exactly the official list's
           WHEN /co/ is compiled on each with the default cpk_name
           THEN the first reports dpfilelist_not_official naming 4cc_80_mine and upgrade-dpfl,
                download/4cc_99_test.cpk holds the run's CPK and the exit code is 0; the second
                reports neither dpfilelist_not_official nor dpfilelist_outdated
TC-DEP-13  GIVEN an installed DPFL listing 4cc_38_balls, 4cc_40_faces, 4cc_45_uniform,
           4cc_60_midcup, 4cc_61_midcup and 4cc_86_mine, each a 1 KiB file of its own bytes in
           download/
           WHEN upgrade-dpfl --yes runs
           THEN the files are named 4cc_16_balls, 4cc_41_teams, 4cc_42_teams, 4cc_61_midcup (the
                old 4cc_60's bytes) and 4cc_62_midcup (the old 4cc_61's), each byte for byte,
                4cc_86_mine.cpk is still present and reported as no longer loaded, and every
                other official entry is the 6,272-byte placeholder
TC-DEP-14  GIVEN an installed DPFL holding the official list's entries, and download/ holding
           every listed CPK but 4cc_62_midcup.cpk
           WHEN /co/ is compiled with the default cpk_name
           THEN dpfilelist_cpk_missing is reported naming 4cc_62_midcup.cpk, the run's CPK is
                deployed, and the exit code is 0
TC-DEP-15  GIVEN an installed DPFL holding the official list's entries, or those plus one of its own
           WHEN upgrade-dpfl --yes runs
           THEN the official list is not rewritten: dpfilelist_up_to_date when download/ holds
                every listed CPK, else a placeholder is written for each missing one; a list with
                an entry of its own is replaced, dpfilelist_cpk_dropped naming that entry's CPK,
                which stays
TC-DEP-16  GIVEN no PES folder or no download/, a DpFileList.bin that cannot be read, or a CPK to
           rename held open without delete sharing
           WHEN upgrade-dpfl --yes runs
           THEN it is refused naming the missing folder and pes_folder_path (exit code 2) or the
                file (exit code 3), and nothing in download/ is created, renamed or written
```

**Phase 5 scope.** On Fox, `compile` builds every compiled player's `PlayerAppearance.bin` row
from his `settings.toml` and folder markers, writes the boots and gloves rows of every compiled
player (the default-ID rows included), and writes the aesthetics patch (motions, and a `write_names`
export's names) to the output folder, applying it to the local savefile. A compiled player is
every player a kept player folder of a team export maps to ("Player settings in exports" in the Aesthetics export
plan). Phase 5 withdraws TC-BIN-10, TC-BIN-17 and TC-BIN-22, whose player tables it changes,
and TC-BIN-25 to TC-BIN-27 restate them. Not in Phase 5, so not here: the pre-Fox patch and its
savefile writes (deferred with pre-Fox's other gaps: a compile for PES 15-17 writes no patch and
leaves the savefile byte-identical) and a referee export's settings (worklog issue). PES 18
and 19 appearance rows are built like PES 20 and 21's (`spec/pes_savefile/model.md`, "Fox
database row"). Test data as in Phase 4. A scenario naming no coverage tag compiles a `Full`
export (a `Midcup` one keeps a player's installed row where a `Full` one writes the default,
`aesthetics_export/settings_toml.md`). The patch halves of TC-BIN-31 to TC-BIN-33 are proven
with the patch (step 5.3b), their table halves before. Unless a
scenario says otherwise, the PES folder is a deployable fixture install whose DpFileList lists
`4cc_08_bins` (holding the three player tables) and the output CPK, and `savefile_path` names
a stripped fixture save of the compile's version carrying team 714 (a test never uses `auto`,
which would find the developer's own save).

**Player tables**

```
TC-BIN-25  GIVEN an installed BootsList.bin of ten pairs, a GloveList.bin and a
           PlayerAppearance.bin, none of them for a team 714 player, and a /co/ export whose
           slot 05 holds boots.fmdl and a settings.toml
           WHEN it is compiled for PES 21
           THEN the CPK's BootsList.bin holds eleven pairs sorted by player id with (71405, 625)
                among them and the ten unchanged; GloveList.bin gains (71405, 0) and
                PlayerAppearance.bin a row for 71405, every installed row unchanged
TC-BIN-26  GIVEN an installed BootsList.bin holding (71405, 7), (71406, 9) and (71407, 55), and a
           /co/ export whose slot 05 holds boots.fmdl, slot 06 no boots, and slot 07 no boots
           and the fpc_off marker
           WHEN it is compiled for PES 21 as a Full export, then as a Midcup export
           THEN the Full run's BootsList.bin holds (71405, 625), (71406, 0) and (71407, 0), and
                the Midcup run's (71405, 625), (71406, 9) and (71407, 0)
TC-BIN-27  GIVEN no PES install configured, and a /co/ export whose slot 05 holds boots.fmdl
           WHEN it is compiled for PES 21
           THEN the CPK holds k0625's boots and no BootsList.bin, GloveList.bin or
                PlayerAppearance.bin, and player_table_missing names each of the three with 1 row
TC-BIN-28  GIVEN an installed PlayerAppearance.bin with a row for 71405 and one for 71406, a /co/
                export whose slot 05 settings.toml sets one appearance key, and whose players.txt
                maps slot 05's folder to 05 and another folder, holding a face model and no
                settings.toml, to 06 and 08
           WHEN it is compiled for PES 21
           THEN 71405's row is the template's defaults with that key's value, 71406's and
                71408's are the template's defaults, and nothing of either installed row remains
TC-BIN-29  GIVEN a /co/ export whose slot 05 carries fpc_on and its settings.toml sets
                boots_id = 12, slot 06 carries fpc_on and no boots or gloves, and slot 07 holds
                boots.fmdl and sets boots_id = 12
           WHEN it is compiled for PES 21
           THEN BootsList.bin holds (71405, 12), (71406, 55) and (71407, 627), GloveList.bin
                (71405, 11), (71406, 11) and (71407, 0), and settings_model_id_conflict names
                slot 07's boots
TC-BIN-30  GIVEN a Full /co/ export whose slot 05 carries fpc_on, and whose settings.toml sets a key
                the hide preset sets, to another value, the inners on, undershorts and taping;
                and whose slot 06, with no marker, sets the gloves checkbox on and no gloves;
                and two more /co/ exports whose slot 05, with no marker, sets the gloves
                checkbox on and holds no gloves: a Full one linking Round.face, the export's
                Faces/Round/ holding a face model, and a Midcup one holding a face model
           WHEN each is checked and compiled for PES 21
           THEN both runs of the first report fpc_strip_conflict, fpc_inners_break_hiding,
                fpc_undershorts_break_hiding and fpc_taping_shows_pieces for slot 05 and
                fpc_gloves_checkbox_winter_gloves for slot 06; 71405's row carries the preset's
                value for the key and the inners, undershorts and taping as authored; the
                Full one reports fpc_gloves_checkbox_winter_gloves for slot 05 (a face link
                is no gloves ID), and the Midcup one reports no gloves finding
TC-BIN-31  withdrawn: restated as TC-BIN-49 (`spec/team_compiler/README.md`), the
           face and hair keys read from ingame_face.toml since step 0.0.1h
TC-BIN-32  GIVEN an installed pair (71405, 7), and a /co/ export whose slot 05 holds a
           boots.fmdl whose task fails and a settings.toml setting an appearance key and a
           motion key, and whose slot 06 holds a face model that compiles
           WHEN it is compiled for PES 21
           THEN BootsList.bin keeps (71405, 7), 71405's appearance row carries the key, and
                the patch holds his motion keys
TC-BIN-33  GIVEN an installed PlayerAppearance.bin row for 71405 with no value at its default,
                and a /co/ export whose slot 05 holds a face model and a settings.toml with a key
                of the wrong type
           WHEN it is compiled for PES 21
           THEN settings_toml_invalid is reported, 71405's row is the template's defaults, and
                the patch holds his motions at their defaults
TC-BIN-37  GIVEN multi-CPK mode, an installed 4cc_08_bins.cpk (the bins CPK) holding a
                PlayerAppearance.bin row for 71423 and the pair (71423, 7), and a /co/ export
                with no slot 23 folder whose slot 05 holds boots.fmdl
           WHEN it is compiled for PES 21
           THEN the output bins CPK keeps 71423's row and pair as installed, and holds 71405's
                rows from the export
TC-BIN-38  withdrawn: restated as TC-BIN-48 (`spec/team_compiler/README.md`), the
           facial hair type read from ingame_face.toml since step 0.0.1h
TC-BIN-47  GIVEN /egg/ slot 05 holding face_high.fmdl and a settings.toml, and slot 06
                 holding an ingame_face.toml setting iris_color = 7 and [hair] length = 5
           WHEN the export is compiled for PES 21
           THEN 79205's PlayerAppearance row holds the template's defaults for every face,
                hair and iris field, and 79206's holds iris 7 and hair length 5, its other
                face and hair fields the defaults
TC-BIN-48  GIVEN a Midcup /co/ export whose only player folder, slot 05, holds an
                 ingame_face.toml setting a facial hair type and a settings.toml setting
                 boots_id = 12, and no model
           WHEN it is compiled for PES 21
           THEN the output CPK is written and holds the three player tables, with (71405, 12)
                in BootsList.bin and 71405's appearance row from his two files, the facial
                hair type the ingame_face.toml's
TC-BIN-49  GIVEN a /co/ export whose slot 05 ingame_face.toml holds facial_hair_type = 20
                 (PES 18 unmeasured: the field's 0 to 31; PES 21: 0 to 19) and in-game hair
                 (style = 4, color_red = 40), and whose settings.toml holds
                 hunching_dribbling = 5 (PES 18: 1 to 3) and dribbling = 2 (PES 20 and 21
                 only)
           WHEN it is compiled for PES 18, and for PES 21
           THEN the PES 18 run reports settings_value_out_of_range for hunching_dribbling
                alone, 71405's row has facial hair type 20, its patch holds
                hunching_dribbling 1 and no dribbling key, no finding names dribbling; the
                PES 21 run reports it for facial_hair_type alone and its row has facial hair
                type 0; both runs' rows carry that hair; both exit 0
```

**Aesthetics patch and savefile**

```
TC-SAV-01  GIVEN a /co/ export whose slot 05 settings.toml sets one motion key and a name, and
                a savefile whose team 714 holds 71405 in roster slot 06 and 71406 in slot 05
           WHEN it is compiled for PES 21
           THEN <output_folder_path>/aesthetics_patch.toml names the CPK and holds team 714's
                slot 05 with every motion key (the defaults for those absent) and no name;
                patch_written names it; in the savefile 71405's motion fields are the patch's
                and 71406's unchanged,
                his name and appearance id unchanged, every other byte unchanged, and the
                previous file is the .bak
TC-SAV-02  GIVEN an export with the `write_names` marker whose slot 05 settings.toml has name = true
           and shirt_name = "SNUFFY", in the folder whose name part is Snuffy, and whose slot 06
           folder, mapped to two players by players.txt, sets a name
           WHEN it is compiled for PES 21
           THEN the patch and the savefile carry 71405's name Snuffy and shirt name SNUFFY, both
                players of slot 06's folder carry its name, and settings_toml_name_shared is
                reported
TC-SAV-03  GIVEN a slot 05 settings.toml whose name is longer than PES 21's name field, in a
           `write_names` export
           WHEN it is compiled
           THEN settings_name_invalid is reported, the patch has no name key for 71405, and
                the savefile keeps his name
TC-SAV-04  GIVEN savefile_path naming a missing file
           WHEN the export is compiled
           THEN the patch is written, savefile_missing is reported, and the exit code is 0
TC-SAV-05  GIVEN a PES 19 savefile for a PES 21 compile, and a root holding the /co/ and /egg/
           exports with a savefile holding team 792 and not 714
           WHEN each is compiled
           THEN the first reports savefile_invalid, leaves the savefile byte-identical and the
                PES folder's previous CPK in place; the second reports savefile_team_missing
                naming 714 and applies 792's entries; the first exits 3 (its deployment rolled
                back), the second 1
TC-SAV-06  GIVEN an output folder holding an aesthetics_patch.toml from an earlier run
           WHEN an export with kits and no player folder is compiled
           THEN no patch is written and the earlier one is now aesthetics_patch.toml.bak
TC-SAV-07  GIVEN a savefile whose 71405 appearance id is set
           WHEN a /co/ export with slot 05 is compiled for PES 21
           THEN savefile_player_not_stripped names 71405, and his appearance id stays set
TC-SAV-08  GIVEN a savefile that cannot be replaced (its .bak path taken by a folder)
           WHEN the export is compiled
           THEN savefile_write_failed is reported, the PES folder holds the CPK it held
                before the run, and the exit code is 3
TC-SAV-09  GIVEN an output folder in which aesthetics_patch.toml cannot be written (the path
           taken by a folder)
           WHEN the export is compiled
           THEN patch_write_failed is reported, the PES folder holds the CPK it held before the
                run, the savefile is byte-identical, and the exit code is 3
TC-SAV-10  withdrawn: restated as TC-SAV-17, the savefile written while the game runs
           when compile is given --edit-mode-closed (step 8.9l5b)
TC-SAV-11  GIVEN the installed copy of the output CPK held open without delete sharing
                (TC-DEP-02's degraded run)
           WHEN the export is compiled
           THEN the CPK and the patch are promoted to <output_folder_path>, the savefile is
                byte-identical, and the exit code is 1
TC-SAV-12  GIVEN a /co/ export whose slot 05 settings.toml sets a motion key, and savefile_path
                naming a PES 17 fixture save
           WHEN it is compiled for PES 17
           THEN no aesthetics_patch.toml is written and the savefile is byte-identical
TC-SAV-13  GIVEN a write_names export with a player whose name is too long for the field and
                two players sharing one settings.toml name
           WHEN it is checked, then compiled with --no-deploy
           THEN check reports the same settings_name_invalid and settings_toml_name_shared
                lines as compile
TC-SAV-14  GIVEN a team export and a referee export (/refs/) compiled together for PES 21
           WHEN they are compiled with --no-deploy
           THEN the patch holds one [[teams]] entry, the team export's, and the exit code is 0
TC-SAV-15  GIVEN a folder at <output_folder_path>/aesthetics_patch.toml and an export with a
                notes.txt
           WHEN it is compiled with --no-deploy
           THEN patch_write_failed is reported (Fatal), no teamnotes.txt is written, and the exit
                code is 3
TC-SAV-16  GIVEN a PES 21 savefile present and an export whose settings.toml sets a motion key
           WHEN it is compiled with --no-deploy, and again in --mode test
           THEN no savefile_ finding is reported, the savefile is byte-identical, and each exit
                code is 0
TC-SAV-17  GIVEN a running PES of the compile target's version (the tests' running-PES probe)
                 with a savefile present
           WHEN the export is compiled in sideload mode
           THEN without --edit-mode-closed the patch is written naming no CPK,
                savefile_skipped_pes_running is reported naming the game and the flag, the
                savefile is byte-identical, and the exit code is 0; with it, the savefile is
                written (savefile_written); with no PES of the target's version running (none at
                all, or only another version's), it is written without the flag
```

**Shared IDs across a Midcup**

The /co/ block's shared IDs are 644 to 660 (`spec/aesthetics_export/player_folders.md` "Assigns
IDs automatically").

```
TC-BIN-34  GIVEN a Full /co/ export holding Boots/Crocs, Boots/adidas and Boots/Birk, slots 05
                and 07 linking Crocs, slots 06 and 08 adidas, and slot 09 alone Birk (a shared
                folder no player links takes no ID)
           WHEN it is compiled for PES 21
           THEN the CPK holds unlimited-tool-works/shared_ids/714.toml whose [boots] names 644 adidas
                and 645 Crocs and not Birk, and BootsList.bin holds (71405, 645), (71406, 644),
                (71407, 645), (71408, 644) and (71409, 629), Birk under slot 09's own ID
TC-BIN-35  GIVEN an installed CPK below the output holding TC-BIN-34's record, and a Midcup
                /co/ export holding Boots/Birk and Boots/crocs, slots 05 and 07 linking Birk and
                slot 06 alone linking crocs
           WHEN it is compiled for PES 21
           THEN Birk takes 646 and crocs keeps 645, the record naming it though one player
                wears it: BootsList.bin holds (71405, 646), (71406, 645) and (71407, 646), the
                CPK's record names 644 adidas, 645 crocs and 646 Birk, and no
                shared_ids_unrecorded is reported
TC-BIN-36  GIVEN no installed CPK holding a /co/ record, and a Midcup /co/ export holding
                Boots/Birk, slots 05 and 06 linking it; and, apart, the same Midcup with slot 05
                alone linking Birk
           WHEN each is compiled for PES 21
           THEN the first's Birk takes 644, BootsList.bin holds (71405, 644) and (71406, 644),
                shared_ids_unrecorded names team 714, and the CPK's record names 644 Birk; the
                second's Birk takes 625, slot 05's own ID, and the CPK holds no record and
                reports no shared_ids_unrecorded
TC-BIN-39  GIVEN no installed CPK holding a /co/ record, and a Midcup /co/ export whose only
                player folder, slot 05, holds a settings.toml
           WHEN it is compiled for PES 21
           THEN the CPK holds no unlimited-tool-works/shared_ids/714.toml, and no shared_ids_unrecorded
                is reported
TC-BIN-40  GIVEN an installed CPK below the output holding an empty /co/ record, and TC-BIN-39's
                Midcup export; and, apart, a Full /co/ export whose only player folder, slot
                05, holds a settings.toml
           WHEN each is compiled for PES 21
           THEN each CPK holds unlimited-tool-works/shared_ids/714.toml with no entry, and no
                shared_ids_unrecorded is reported
TC-BIN-41  GIVEN four /co/ exports whose slot 05 settings.toml sets the stock ID 3 (gloves_id
                in the first two, boots_id in the others): the first's slot 05 holding
                Crocs.gloves, the export's Gloves/Crocs/ holding glove_l.fmdl; the second's a
                body.fmdl whose vertices carry both hands' weights; the third's Round.face, the
                export's Faces/Round/
                holding face_high.fmdl and boots.fmdl; the fourth's Crocs.boots, the export's
                Boots/Crocs/ holding a texture and no model
           WHEN each is checked and compiled for PES 21
           THEN 71405's GloveList.bin row is 625, his exclusive ID, in the first (Crocs, which
                he alone wears) and in the second (the hands split off his body), and his
                BootsList.bin row 625 in the third (Round's boots.fmdl a part of his boots),
                each run reporting settings_model_id_conflict for the authored key; the
                fourth's row is the authored 3, shared_folder_no_model names Boots/Crocs, and
                no settings_model_id_conflict is reported
TC-BIN-42  GIVEN the installed game holding team 714's TeamColor.bin and UniColor.bin entries
           for p1 to p3, and two Midcup /co/ exports, the first holding a root colors.txt and
           an empty Kits/p2/ (a root file alone is export_empty), the second only
           Kits/p2/colors.txt
           WHEN each is compiled for PES 21
           THEN each writes the output CPK: the first's TeamColor.bin holds the root file's
                colors for 714, the second's UniColor.bin p2's colors with p1 and p3 as
                installed
```

**Shared gloves on PES 15-17**

Every `.mtl` of the output moves to its root (`pipeline.md` "3. Per-model-folder parallel
steps (rayon)"), so TC-MOD-93 restates TC-MOD-74, whose packing it changes; TC-MOD-74 is
marked withdrawn in the commit that rewires its test.

```
TC-MOD-93  GIVEN Gloves/Keeper holding glove_l.model with glove_l.mtl, skin.dds, and
                grip/glove_r.model with grip/glove_r.mtl naming grip.dds and skin.dds, and
                grip/grip.dds, slot 05 linking it
           WHEN the export is compiled for PES 17
           THEN the gloves output holds grip/glove_r.model, grip_glove_r.mtl, grip/grip.dds
                and skin.dds, and no grip/glove_r.mtl; glove.xml lists ./glove_l.model and
                ./grip/glove_r.model with material ./grip_glove_r.mtl; grip_glove_r.mtl's
                paths name ./grip/grip.dds and ./skin.dds
```

**Referees**

```
TC-REF-19  GIVEN a refs export whose Ref A holds a face model and a settings.toml setting the
                gloves checkbox on and no gloves; and a refs export whose Ref A, with the same
                settings.toml, links Grip.gloves, the export's Gloves/Grip/ holding glove_l.fmdl
           WHEN each is checked, and compiled for PES 21
           THEN both runs of the first report fpc_gloves_checkbox_winter_gloves for Ref A; the
                CPK holds no PlayerAppearance.bin row for him, and no patch is written; the
                second reports no gloves finding (the linked gloves are his own package)
```

---
