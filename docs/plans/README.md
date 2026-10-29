# Unlimited Tool Works — Plan index

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The work not built yet, one document per concern; what is built is described under the same
file names in the [spec](../spec/README.md). Start with the [core plan](core/README.md) (the GUI
shell, distribution, the development plan's remaining phases); the tools and the major library
crates have their own documents. A document below whose concern is mostly built (the Team
compiler, the libraries, the savefile) holds only its unbuilt parts here.

| Plan | Covers |
|------|--------|
| [Core](core/README.md) | Platform architecture and cross-cutting decisions |
| [Team compiler](team_compiler/README.md) | The flagship tool: pipeline, planning, processing, packing, deployment, GUI grid, referee support |
| [Aesthetics export](aesthetics_export/README.md) | The `aesthetics_export` lib and the Studio aesthetics export format: object model + validation progression, player folders, `settings.toml`, FPC markers — shared by the compiler, upgrader, kit config editor, refs arranger, and Team creator |
| [Save editor](save_editor.md) | 4ccEditor + Midcupping successor: editing UI, tactics editor, AATF checks, comparator, transplant; also specs the `team_widgets` lib shared with the Team creator |
| [AATF rules](aatf_rules.md) | The `aatf` lib: the ruleset schema (tiers, heights, cards, conditional specials), the generic Rhai interpreter carried in every rules file, suggestions, the host API, the official Autumn 26 and VGL26 rulesets — shared by the Save editor, Team creator and Ruleset editor |
| [Ruleset editor](ruleset_editor.md) | Forms over the ruleset schema for invitational organizers: tiers, heights, cards, specials, suggestions; rules file open/save with logic identity, rules summary |
| [Stadium compiler](stadium_compiler.md) | Stadium export compilation, fox2, billboards |
| [Export upgrader](export_upgrader.md) | Old-format → Studio-format export migration: the optional glTF pass (Phase 7) and the tool's view (Phase 8) |
| [Music player](music_player.md) | Rigdio successor: match-day soundboard — .4ccm exports, conditional goalhorns, chants, loudness normalization (also specs the `music_export`/`audio_engine` libs) |
| [Music export editor](music_export_editor.md) | RigDJ successor: .4ccm editing UI, condition forms, validation, audition playback |
| [Match tracker](match_tracker/README.md) | SEN:P-AI successor: live match stats/events read from the game's memory, match records, and the `match_feed` lib driving the Music player's autopilot |
| [Kit config editor](kit_config_editor.md) | Kit Manager successor: kit config format reference (recovered by disassembly), TOML configs in exports, editing UI with preview and kit menu colors, and the `kit_config` lib |
| [Refs arranger](refs_arranger.md) | Referee allocation aid: per-match referee lists for the Fox referee hook (PES 18–21) with a weighted fallback fill, per-version slot appearance-rate tables and drag-and-drop slot assignment (PES 15–17), players.txt / ref_lists.txt writing, handoff to the Team compiler |
| [Balls compiler](balls_compiler.md) | Ball selection compiler: the balls export format, ball list editing with thumbnails, Ball.bin reference, own CPK output |
| [DB generator](db_generator.md) | pes-db-generator successor: the cup's game database (`pesdb` tables for teams, placeholder players, managers, competition entries) from the teams list, the Konami tables copied from the install, the 19+ populated EDIT; also specs the `pesdb` lib shared with the Balls and Stadium compilers |
| [Team creator](team_creator.md) | New-team wizard for new managers and fake-team invitationals: roster file → legal Team TOML (AATF-checked defaults) + aesthetics export built from embedded starter heads; owns no format, hands off to the compiler, editors and Save editor |
| [Player aesthetics editor](player_aesthetics_editor.md) | Player-folder convenience editor: browse/launch into Blender via the `pes-models` extension's loader (manifest contract), settings.toml editing, in-place glTF conversion, base-model extraction |
| [Unified model format](model_format.md) | The Studio's authoring format: glTF + `PES_bone`/`PES_mesh` extensions, material files (`materials.toml`, name matching, layering, `.common` links) and the complete material schema (shader families, canonical texture roles, per-engine tables) |
| [Model conversion](model_conversion/README.md) | `model_convert` lib: IR, glTF (import + export), Blender integration, mesh splitting |
| [Savefile](pes_savefile/README.md) | `pes_savefile` lib: crypto, schema-driven codec, player/team/tactics model, interchange formats |
| [Library crates](libs/README.md) | The remaining lib crates: format parsers, DDS conversion, color tools (kit-color extraction + picker widget), archives |

Coding rules live in [`../CONTRIBUTING.md`](../CONTRIBUTING.md), domain terms in
[`../GLOSSARY.md`](../GLOSSARY.md), implementation status in [`../WORKLOG.md`](../WORKLOG.md), and
choices made where these plans were silent in [`../DECISIONS.md`](../DECISIONS.md).

## How these documents evolve

Two trees with one layout: `docs/spec/` describes what the suite does now, `docs/plans/` the
work not built yet. A file and a heading keep their path and text in both, so a section that is
partly built appears in each tree under the same heading, the plan's part pointing at the spec
section it builds on. Plans are written in the present tense, like the spec: the folder says the
work is future, and a section moves without a rewrite of its verbs. Because an excerpt read
without its path would pass for fact, every plan file opens with a banner saying it is a plan,
and briefs and reviewer prompts quote plan text with its path.

- **Acceptance sections.** Before the phase that delivers a tool, that tool's plan gains an
  "Acceptance" section: stable-ID GIVEN/WHEN/THEN scenarios for the user-observable behavior the
  phase adds (format in `CONTRIBUTING.md` "Testing"; procedure in `AGENTS.md` "Working
  documents"). They are written just-in-time, one phase at a time, not for the whole suite up
  front. Lib crate plans do not get them: for a format crate the fixtures and the parity standard
  are the specification.
- **The move at phase close.** When a phase closes, after the converge audit has confirmed the
  code meets its sections and acceptance IDs, what it delivered moves to the same file and
  heading in `docs/spec/`, corrected where the code was built differently. The rationale ("X not
  Y because Z") moves with it; the acceptance IDs move as the behavior contract. Moving per phase
  rather than in one pass at the end keeps the spec true while the work is fresh. Text written
  in the future tense before 2026-10-12 is converted when its section is next edited or moves,
  never in bulk.

Three rules about where the text lives:

- **The specification is not moved into the crates.** The documents do not map one-to-one onto
  crates (`libs` covers fifteen, `team_compiler` specs three, the format documents are shared by
  several tools and the Blender add-on, `core` has no crate). A crate that wants to name its spec
  points at the section in `docs/spec/` (`//!` in `lib.rs`, a fixture README); code never points
  at a plan, which describes only what code does not do yet.
- **A pointer names its tree.** A working document names a plan section by its path below
  `plans/` (or the bare path, the older form) and a spec section by `spec/<path>`; `just
  doc-pointers` (a gate) fails on a pointer whose file or section is gone, so a move lists every
  pointer it broke. `DECISIONS.md` and `LOG.md` stay append-only: their pointers keep the tree
  they were written for.
- **A document that crosses roughly a thousand lines becomes a folder**, `<name>/`, with a
  `README.md` (the original preamble, an index of the parts, and the small cross-cutting
  sections: crate layout, phases, key decisions) and one file per major section; the same
  threshold and shape the code uses for a module (`CONTRIBUTING.md` "Architecture rules").
  Section headings keep their text across the split, so a pointer only changes its file part,
  and every part states which document it belongs to.
