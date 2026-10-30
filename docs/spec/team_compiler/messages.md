# Unlimited Tool Works — Team compiler plan: Message catalog

Part of the [Team compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Message catalog

The warning/error messaging is overhauled, not ported (see `README.md` "Relationship to the older compilers").
Red's ~150 messages (audited across `export_check.py`, `texture_check.py`, `xml_check.py`,
`team_id_get.py`, `portraits_move.py`, `bins_update.py`, `referee_tools.py`, `cpk_tools.py` and the
stage scripts) reduce to a structured catalog: every reportable condition gets a stable ID, and the
pipeline emits it as **data** — the GUI, CLI, and log files decide presentation.

### Message structure

```rust
use std::borrow::Cow;

pub struct MessageCode {
    pub tool_id: &'static str,             // "team-compiler"
    pub code: Cow<'static, str>,            // "player_number_duplicate"
}

pub struct Message {
    pub code: MessageCode,                  // cross-tool stable identity
    pub severity: Severity,                 // presentation and filtering
    pub disposition: Disposition,           // processing consequence, independent of severity
    pub scope: Scope,                       // what the message is about — drives grid cell mapping
    pub context: Vec<(String, String)>,     // structured fields: file name, expected/found values, ...
}

pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}
// Ordering: Info < Warning < Error < Fatal. "Errors" filter includes Fatal.
// Cell status uses the highest relevant severity after disposition policy.

pub enum Disposition {
    Keep,
    DropFile,
    DropSlot,                             // discard one normalized players.txt assignment
    DropFolder,
    DropExport,
    AbortRun,
}

pub enum Scope {
    Run,                                  // not tied to an export (output stage, settings)
    Export { export_id: ExportId },
    Folder { export_id: ExportId, path: vtree::ScopePath }, // player/shared/kit folder
    File   { export_id: ExportId, path: vtree::ScopePath },
    RosterEntry { export_id: ExportId, file: vtree::ScopePath, line: usize, slot: Option<u16> },
}
```

`ExportId` and canonical `vtree::ScopePath` values route events and logs; human-readable
export/folder/file names stay in message context. `ScopePath` is the platform-neutral canonical path
type: `aesthetics_export` uses/wraps it but does not own it, keeping `studio_core` independent of
team-format knowledge. Stable IDs avoid ambiguity when a folder export and an archive have the same
display stem.

The `MessageCode`/`Message`/`Severity`/`Disposition`/`Scope` types live in `studio_core` alongside
`PipelineEvent` (all tools emit them). Each tool owns its code strings and text catalog under its
stable tool ID; the catalog below belongs to `team-compiler`, while the stadium compiler and export
upgrader define their own codes on the same structure.

Principles:

- **One condition, one ID.** Red repeats near-identical messages per folder type ("Bad face folders"
  / "Bad boots folders" / "Bad gloves folders"); here the folder identity is in the `scope`, so one
  `fmdl_name_invalid` ID covers all model folder kinds.
- **Message text lives in one table** keyed by ID (template + remediation hint; until the help
  window and the GUI log need the text, in Phase 8, `messages.rs` holds each code's severity
  only, and the CLI prints the code with its context fields), not scattered
  through the pipeline. Red's remediation-hint style is kept ("Resize it so that both sizes are
  powers of 2").
- **Consequences are explicit**: `Disposition` records whether the finding keeps content, drops a
  file, normalized roster slot, folder, or export, or aborts the run; severity remains a
  presentation/filtering concern. File-scoped consequences still roll up according to Red's rules
  (model-file failures drop the folder; standalone texture/portrait failures may drop only the
  file). A line-local `RosterEntry` finding with `DropSlot` removes only that normalized assignment;
  file-wide encoding failure or roster ambiguity that prevents trustworthy normalization uses
  `DropExport` instead. Runtime failures derive their effective disposition from the owning scope:
  optional root file → `DropFile`, folder/task → `DropFolder`, required export metadata or unusable
  source → `DropExport`, output-writer/global invariant → `AbortRun`.
- **What makes a finding an Error.** A file that cannot be produced, or whose compiled result is
  unusable or visibly compromised in the game (a mesh that does not render, geometry that lags
  the match, a texture the model names and nobody supplies), is an Error and drops its folder. A
  file that compiles to something usable but may hide an issue is a Warning, and the folder
  compiles. The format crates' `check` severities follow the same rule, so the deep pass maps a
  format finding by its severity: Error is `DropFolder`, Warning and Info are `Keep`. A format
  Error that a census shows on models the game renders fine is a wrong severity in the format
  crate, fixed there, not an exception here.
- **`pass_through`** overrides only eligible content-level `DropFile`/`DropFolder` dispositions to
  keep-with-flag, as in Red, while retaining the original severity. Not eligible: findings whose
  content cannot exist (failed conversion/packing, unproducible logos), unused content
  (`shared_folder_orphaned`), contradictory directives that leave no value to keep
  (`fpc_conflict`, `kit_layout_conflict`, `shared_link_duplicate`), kit identities that cannot be kept (`kit_slot_duplicate`,
  `kit_folder_invalid`), ambiguous model-source or texture identity/conflicts, foundational
  unsafe-path/ambiguous-roster/required-metadata failures, geometry far from the origin
  (`vertex_too_far_from_origin`: the lag it causes persists for the whole matchday),
  `DropSlot`, `DropExport`, `AbortRun`, and environment-level dispositions. Written errored content
  receives a distinct `DoneWithErrors` outcome rather than an ordinary red error state.
- **No blocking console prompts.** Red's prompts are replaced, not kept: unknown team ID becomes a
  plain Error (`team_name_unknown`),
  the one standing consent (overwriting `dt00_x64.cpk` with the Fox referee marker) is gone with
  the write it guarded, and retry-on-locked-file becomes a plain Error. The CLI never prompts: all of these are hard errors there.
- **Progress chatter is not a message.** Red's "- Packing the face folders..." prints map to
  `Progress` events, not catalog entries.

### Catalog

Adapted to the Studio export format: Red's Note-file, Kit Configs/Kit Textures-folder,
dependency-check, and self-healing messages are gone; player-folder, link-file, settings.toml, and
savefile messages are new.

**Export level**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `export_extract_failed` | E | archive cannot be extracted/parsed, or its listing is refused (a path escaping the root, two names that fold to one; context: the path) | export skipped (`DropExport`) |
| `no_exports_found` | W | discovery finds no export folder, `.zip` or `.7z` in the exports folder (Run scope; context: the folder) — what a fresh install's created `exports/` reports until the user fills it | none; nothing is checked or compiled |
| `export_disabled` | I | root `NO_USE` / `NO_USE.txt` marker disables this source | export skipped; omitted from duplicate-ref detection |
| `export_identified` | I | the export's identity resolved (context: `team` and `id`, or `team` = `referees`) | none; how the CLI reports which team an export is |
| `source_read_failed` | E/F | a pinned source entry cannot be read | optional root file: `DropFile`; folder/task producer: `DropFolder`; required export metadata or unusable source: `DropExport`; output/global source invariant: `AbortRun` |
| `source_changed_during_run` | F | a file a task read is gone, or its size or modified time is no longer what the export's listing gave (an archive: the archive file's); on the export, context `path` | run aborted, the staging output discarded, nothing deployed (`AbortRun`) |
| `template_override_unreadable` | F | a template override file exists but cannot be read, or is one of the bins or `DpFileList.bin` and does not parse as it (embedded defaults cannot be missing; context: the file, the whole error chain) | run aborted before any export is read, whatever the resource (`AbortRun`; `pipeline.md` "Resolved decisions", "Templates and fallback bins") |
| `template_override_active` | I | a `templates/` override file is shadowing an embedded template/fallback-bin resource (context: the file) | override used; reported per file per run |
| `export_balls_skipped` | I | export name's first word is `balls` (balls exports belong to the Balls compiler) | export skipped (`DropExport`) |
| `multiple_ref_exports` | E | more than one non-disabled `refs`-prefixed export found at discovery | every conflicting export skipped (`DropExport` per row) |
| `duplicate_aesthetics_export` | E | multiple exports resolve to the same team ID; found once every export's identity is resolved, so `check` reports it too. An export validation dropped does not count, whether before its identity resolved or after (an exhausted ID pool): it is not compiled anyway (context: `id`, the team ID; `exports`, the conflicting sources' file names in export order) | all conflicting exports skipped (`DropExport`, on each) |
| `boots_id_pool_exhausted` | E | the shared boots folders taking a shared ID (linked plainly by two or more slots, or named by a `Midcup`'s installed record) need more of the block's 17 than are free; only a `Midcup` reaches it, a `Full` export having at most 11 such folders (context: `count`, the IDs needed) | export skipped (`DropExport`) |
| `gloves_id_pool_exhausted` | E | as `boots_id_pool_exhausted`, for the gloves folders | export skipped (`DropExport`) |
| `export_empty` | E | no usable content found at root | export skipped |
| `content_not_yet_compiled` | withdrawn | step 4.20 withdrew the Phase 3 subset gate that skipped an export holding content `compile` could not build yet (`spec/team_compiler/README.md` "Phase 3 scope"); every export now reaches processing, a file no package reads being `file_not_used` | — |
| `nested_folders_fixed` | W | content found nested one level down (exactly one usable root) | auto-fixed |
| `nested_root_ambiguous` | E | several nested child folders are usable export roots | export skipped (`DropExport`) |
| `nested_root_conflict` | E | loose root file collides with a flattened nested file at the same virtual path | export skipped (`DropExport`) |
| `export_tag_missing` | E | a team export's name has neither `Full` nor `Midcup` as its second word (context `name`: the export's name) | export skipped; the hint says to rename it `<team> Full …` or `<team> Midcup …` |
| `export_layout_old` | E | the export is in the old layout, which the compiler does not read: a root folder named `Kit Configs`, `Kit Textures` or `Other`, names the Studio format never has (its kits live in `Kits/`); context `folder`: the first such folder. Reported alone: the export's other structure findings are not computed, since every one of them would describe the old layout again | export skipped; the hint says to run the export through the Export upgrader once |
| `team_name_unknown` | E | canonical `team_name` not in `teams_list.txt` | export skipped |
| `team_id_out_of_range` | withdrawn | nothing produces it: `teams_list::TeamId` holds 701–920 only, and a list line whose ID is outside it is the list reader's own error, so no export resolves to an ID outside the range (converge 4.y-conv, 2026-10-09) | — |
| `team_colors_missing` | I | no root `colors.txt` (it is optional) | the team's colors in `TeamColor.bin` are left as they are |
| `color_entry_invalid` | W | a `colors.txt` line that does not parse as exactly one color (two colors on one line, the old Team Note kit entry, included), or a valid line past the file's color count (two for a kit, four for the team). Grammar: "Root files" (Colors) in `aesthetics_export/player_folders.md` | the line skipped |
| `root_file_unexpected` | W | unknown file or folder at the export root (a folder other than the content folders — a stale `wrapper/` beside a usable root included), or a file directly inside `Players`, `Kits`, `Faces`, `Boots` or `Gloves`, which hold only folders (`Players/players.txt`) | file or folder ignored |
| `portrait_conflict` | E | a slot with a portrait in its player folder and one in `Portraits/` whose bytes differ (the deep pass compares the two files; byte-identical files are one portrait and no finding) | export skipped: the compiler cannot tell which one the manager means |
| `notes_found` | I | non-empty valid root `notes.txt` present | collected into teamnotes.txt by `compile` |
| `notes_encoding_invalid` | E | root `notes.txt` is not valid UTF-8 after optional BOM handling | note dropped; export otherwise continues (`DropFile`, not pass-through-eligible) |

**Player folders and shared model folders**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `player_folder_number_invalid` | E | without `players.txt`, a team player folder's number is not in 01–23 | folder discarded (`DropFolder`) |
| `players_txt_slot_invalid` | E | a `players.txt` line's decimal slot is outside 01–23 (teams) / 01–35 (refs) | that `RosterEntry` assignment skipped (`DropSlot`) |
| `player_number_duplicate` | E | without `players.txt`, two team folder names claim the same player slot | both conflicting folders discarded (`DropFolder` on each folder) |
| `players_txt_missing` | E | referee export has no required root `players.txt` (nor a legacy `refs.txt` alias) | export skipped (`DropExport`) |
| `refs_txt_ignored` | W | referee export has both `players.txt` and the legacy `refs.txt` alias | `players.txt` used, alias ignored |
| `players_txt_slot_duplicate` | E | authoritative `players.txt` assigns the same slot more than once, making roster identity ambiguous | export skipped (`DropExport`) |
| `player_unlisted` | W | authoritative `players.txt` is present but does not list this player folder, regardless of any number in its name | folder ignored |
| `player_layout_proto` | E | a referee export's player folder has a direct `face/`, `boots/`, `gloves/` or `common/` subfolder: the AET referee layout, Red's prototype of the player folder, whose subfolders type their files by the subfolder's name where the Studio rule types a subfolder's files by their names (`spec/aesthetics_export/player_folders.md` "Subfolders"); in a team export those names are plain subfolders (context `folder`: the first such subfolder) | folder discarded (`DropFolder`); the hint says to run the export through the Export upgrader once |
| `players_txt_line_invalid` | E | one nonblank `players.txt` line cannot be parsed into decimal slot + folder name | that `RosterEntry` skipped (`DropSlot`) |
| `players_txt_invalid` | E | file-level encoding failure or roster-wide ambiguity prevents trustworthy normalization, or a referee roster retains zero valid assignments | export skipped (`DropExport`; foundational). An empty normal-team roster remains valid for a kit-only export |
| `players_txt_target_missing` | E | `players.txt` entry names a nonexistent folder | `RosterEntry` scoped assignment skipped (`DropSlot`) |
| `link_target_missing` | E | link file references a nonexistent shared folder | folder discarded |
| `link_target_dropped` | E | a shared-folder or `.common` link names a target the structure pass or the deep pass dropped, by its effective disposition (context: the link, the target and its finding) | folder discarded (`DropFolder`); not reported when `pass_through` keeps the target; never itself pass-through-eligible |
| `common_model_beaten_dropped` | I | a `Common/` model another representation of its stem beats (`boots.model` beside `boots.fmdl` on PES 18-21, the reverse on PES 15-17), which the deep pass does not read, when the pass drops the winner: without this the winner's drop would leave the unread file as the stem's selected model (context: `file`, the beaten file's export path; `winner`, the dropped file's) | file discarded (`DropFile`), so planning never selects a file the pass did not check; the players linking the stem follow `link_target_dropped`; not reported when `pass_through` keeps the winner; never itself pass-through-eligible |
| `shared_link_duplicate` | E | player folder has more than one shared link for any category (face, boots, or gloves), counted over its whole tree, two links naming one shared folder counting once | folder discarded (`DropFolder`) |
| `link_combined` | I | link file plus local models for the same category; the shared models become parts of the player's own set (Fox: mesh-merged, own ID) | none |
| `shared_folder_orphaned` | W | shared folder referenced by no player | folder skipped |
| `settings_toml_invalid` | E | optional `settings.toml` is not UTF-8 text or fails to parse, found by the deep pass (context: the file, the error); the in-game face's keys (`iris_color`, `[appearance.face]`, `[appearance.hair]`) are `ingame_face.toml`'s, one left here an unknown key | file ignored, even under `pass_through`; the player compiles with the default settings in its keys' place (`DropFile`, not pass-through-eligible); his `ingame_face.toml` still applies and his models still build |
| `ingame_face_toml_invalid` | E | a player's `ingame_face.toml` is not UTF-8 text or fails to parse as the in-game face file, found by the deep pass (context: the file, the error) | its keys ignored, even under `pass_through`: the file still marks the in-game face, which the player keeps at the game's default; his `settings.toml` still applies |
| `settings_model_id_conflict` | W | `settings.toml` sets `boots_id` or `gloves_id` for a category whose ID the compiler assigns, from the folder's standalone models or a link file (pre-Fox face-XML models assign none; context: the category). Reported on Fox team exports only: on pre-Fox and on a referee export the key reaches nothing yet (the pre-Fox patch, a referee's table rows: worklog issues), and a warning there would single out one ignored key of many | the compiler-assigned ID wins; key ignored |
| `settings_value_out_of_range` | W | a `settings.toml` or `ingame_face.toml` value inside its key's widest range but outside the compile target's (`settings_toml.md`, the templates' ranges; context: the key, the value, the target's range), found by the deep pass on the player folder | the key's default written (`settings_toml.md`: a face type 0, a motion the template's value; a custom skin 7 as 1) |
| `settings_toml_name_shared` | W | `name` or `shirt_name` given in a `write_names` export's folder mapped to multiple players (Fox; pre-Fox with the pre-Fox patch) | the name, when its check passes, goes to all of them |
| `settings_name_invalid` | W | a `name` or `shirt_name` string that does not fit the target version's field or character set (Fox; pre-Fox with the pre-Fox patch; context: the key) | not written; the save's name stands |
| `fpc_strip_conflict` | W | `settings.toml` sets a key the folder's FPC preset also sets, to another value (for `skin_color`, a custom skin, 7, where the preset sets a preset skin; boots and gloves IDs aside: `settings_toml.md` resolves them) | FPC preset wins; keys ignored |
| `fpc_inners_break_hiding` | W | `libs/fpc`'s interference rule on a player whose folder carries `fpc_on` (`libs/fpc.md`; context: the player; the marker is the lib's `is_fpc_player`, so its fifth rule, `fpc_custom_skin_on_non_fpc`, is not reported: a custom skin with no marker is the partial-hide setup, a custom body in a stock jersey, which no marker names); the lib's Error, a Warning here because a Studio Error would drop the folder | written as authored |
| `fpc_undershorts_break_hiding` | W | likewise, the lib's Error too | written as authored |
| `fpc_taping_shows_pieces` | I | likewise | written as authored |
| `fpc_gloves_checkbox_winter_gloves` | W | `libs/fpc`'s rule on any player: the gloves checkbox on with gloves ID 0, whatever the FPC marker (context: the player); the ID is the one `settings_toml.md` resolves, and a `Midcup` player whose gloves keep his installed row is not checked, `check` reading no installed table | written as authored |
| `material_texture_unused` | I | a texture role the target engine has no sampler for (e.g. `metalness` on pre-Fox); from a conversion, also a native sampler of the source engine's material table the target does not carry (context: `model`; `material`, the index; `texture`, the role or the sampler name) | role dropped for that engine |
| `bone_folded_for_version` | I | a used bone the target PES version's skeleton lacks had its weights transferred to the fold table's (or nearest) bone — see "Skeleton retargeting" in the Model conversion plan; on the folder (context: `model`, the source as `model_conversion_failed` names it; `bone`, `<bone> -> <target>`) | none |
| `skeleton_retargeted` | I | the model's bind pose was re-bound from its source version's skeleton to the target's (bones moved more than tolerance); on the folder (context: `model`, the source as `model_conversion_failed` names it; `bones`, how many moved) | none |
| `bone_matrix_unknown` | W | a converted model's bone has no companion `.skl` and a name in none of the target version's template tables, so its bind matrix is the identity: the mesh it skins sits where the identity puts it (context: `model`; `bone`, the index; `name`, the bone's) | none |
| `bone_slot_dropped` | W | a mesh's vertex weighted a bone slot past the mesh's bone group; the slot was zeroed and the vertex's weights renormalized, so it skins differently (context: `model`; `mesh`, the index; `count`, the slots dropped) | none |
| `bone_folded_by_position` | W | a used bone the target version lacks with no fold-table entry was folded onto the nearest target body bone by rest position, a guess (context: `model`; `bone`, `<bone> -> <target>`) | none |
| `material_family_approximated` | W | no shader rule named the material's shader, so its family is the closest fit and the target shader is that family's: the material may render differently (context: `model`; `material`, the index; `name`, the material's) | none |
| `material_split_by_flags` | I | meshes of one FMDL material instance carried different flags, so the instance split into two materials of the target (context: `model`; `material`, the index; `name`, the new material's) | none |
| `material_parameter_dropped` | I | a native parameter of the source engine's material table the target shader does not carry, the conversion's twin of `material_parameter_unused` (context: `model`; `material`, the index; `parameter`, the name) | parameter dropped |
| `sampler_settings_defaulted` | I | a stored `.mtl` sampler name with no stored settings table exported with the default settings (context: `model`; `material`, the index; `sampler`, the name) | none |
| `vertex_bitangents_dropped` | I | FMDL has no bitangent attribute; the game derives them (context: `model`; `mesh`, the index) | none |
| `dummy_texture_added` | I | a shaded or metal material with no normal or specular map got the game's dummy texture for the sampler Fox requires (context: `model`; `material`, the index; `sampler`, the name) | none |
| `native_field_dropped` | I | a field the IR or the target format has no home for was non-default and was dropped, with nothing the game would show: the FMDL's redundant local-space `bone_matrices`, an SKL parent the FMDL's wins over (`skl_parent`), a `normal_w`/`tangent_w` a `.model` cannot carry, a `.model` field (context: `model`; `field`, the name; and the subject's `mesh`, `material` or `bone` index when it has one) | none |
| `mesh_flags_dropped` | W | the converter's `native_field_dropped` for the Fox mesh flag a `.mtl` cannot express, `no_shadow_cast`: the mesh casts a shadow where the source did not (a hidden mesh, `invisible`, is left out of the `.model` with no finding, `model_conversion/ir.md`), which a Warning says and an Info would bury under the row above (context: `model`; `field`, the flag; and the subject's index) | none |
| `model_hidden_dropped` | I | every mesh of an FMDL read for PES 15-17 is hidden (`invisible`), so the game drew nothing of it on Fox: the deep pass drops the file, and a `.common` link naming it with the same Info on the link, so no task converts or names it (no `.model`, no `.mtl`, no xml entry, no part of a merge), the folder's other models standing (`spec/model_conversion/ir.md` "A hidden Fox mesh"; a blank head's `oral.fmdl`, a one-sided gloves folder's other side). Reported by both commands, on the file, not pass-through-eligible (context: `file`, the file as its folder's per-file findings name it; on a link also `model`, the Common file's export path) | none |
| `static_bone_added` | I | an unskinned mesh was weighted to the `static` bone an FMDL needs (context: `model`; `mesh`, the index) | none |
| `weight_clamped` | I | a weight lane above 1 was clamped to the FMDL weight's maximum (context: `model`; `mesh`, the index; `count`, the vertices clamped) | none |
| `empty_mesh_bone_group_dropped` | I | a vertexless mesh's bone group was written empty, it skins nothing (context: `model`; `mesh`, the index; `count`, the group's size) | none |
| `kit_variant_missing` | W | a folder's texture variant set (`pants_kit1`, `pants_kit3`), or on PES 15-17 a model set its face's `face.xml` lists, has no variant for a kit number the export defines and no `kitF` fallback; on the folder, once per set and number (context: `texture`, or `model` for a model set, the set's reference as `pants_kitN`, after its directory below the folder for a subfolder's set (`jessie/pants_kitN`); `kit`, the number; `copied`, the lowest variant's stem, after the same directory) | the lowest existing variant serves the gap: on PES 15-17 copied into it (for a model set, the files its entries name), on PES 18-21 written once under the reference's spelling, which FoxDen loads for a kit without a variant |
| `kit_variant_model_left_out` | W | per-kit model variants (`*_kit1`, `*_kit2`, …; a `*_kitF` joins the set as its fallback) where no `face.xml` names the set, the set's fallback (`kitF`, else the lowest variant) used and a file left out: on a Fox target, which has no model-path indirection, and on PES 15-17 in a shared `Boots/` or `Gloves/` folder's own output (one `boots.model`, one `glove.xml` listing every glove) and under `ingame_face.toml` (his boots and gloves hold parts, not entries); on the folder, once per set, a shared folder's only while a mapped player links it (planning's glTF drop removes the shared folders it leaves with no linking player, with no finding of their own: the drop's names the cause, and such a folder gets no id and no task) (context: `model`, the set's reference with the fallback's extension, `pants_kitN.fmdl` or `pants_kitN.model`; `used`, the fallback's file name, the target's format when both formats hold it) | fallback used, others ignored; a variant in the other engine's format converts, a `.model` with its `.mtl` |
| `kit_variant_mtl_differs` | W | on PES 15-17 a per-kit model variant whose `.mtl` (the one its search finds) is not the one its set's `face.xml` entry implies for its kit number (the entry's `material` respelled for it, directory included); on the folder (context: `model`, the variant's file name; `mtl`, the `.mtl` its search finds as the `face.xml` would write it, `./materials.mtl`; `expected`, the one the entry implies, `./pants_kit2.mtl`) | model packed; the game finds no `.mtl` for that kit |
| `common_link_missing` | E | a `.common` link — model, material file or texture — names a file missing from the Common folder (a `kitN` stem or directory in it standing for its set, `spec/model_format.md` "Kit-dependent assets") (context: the link file, whose name shows its kind, and the Common path looked for); on PES 18-21 a texture link is satisfied by an installed CPK holding the texture at the team's Common path (`pipeline.md` "Resolved decisions", "A texture a model names must exist") | folder discarded |
| `fpc_conflict` | E | both `fpc_on` and `fpc_off` present in a player folder | folder discarded |
| `fmdl_name_invalid` | E | Fox: a model file of any format anywhere in a boots/gloves shared folder, or one a `.common` link there stands for (`spec/aesthetics_export/player_folders.md`), not resolving to that category's allowed names | folder discarded |
| `shared_folder_no_model` | W | a shared boots or gloves folder a mapped player links plainly holds no model of its kind (textures only, or nothing; on PES 18-21 a `.common` link to a model of its kind counts as one, and a selected glTF counts as a model, the folder being dropped for it with `model_gltf_unsupported`), so it has nothing to load: it takes no ID and gets no task, and that player wears the game's own; linked beside his own model it is a texture source as any combined folder is, and a `Faces/` folder with no model is one too (context: `player`, the first folder linking it plainly, in roster order) | folder emits nothing (`Keep`) |
| `shared_ids_unrecorded` | W | a `Midcup` export whose shared boots or gloves folders take a shared ID (a folder under its one wearer's own ID takes none), and no installed CPK below its output records the team's shared IDs (`spec/aesthetics_export/player_folders.md` "A `Midcup` export keeps the shared IDs the team already has"; context: the team) | its folders numbered alphabetically, as a `Full` export's; they may take IDs the main CPK gives other folders |
| `fmdl_fcl_hair_fallback` | I | Fox: arbitrary-named FMDL treated as a face model, routed into the `fcl_hair.fmdl` merge (Red's single-file fallback, generalized) | none |
| `fmdl_merged` | I | Fox: multiple models resolve to the same allowed name; meshes merged into one FMDL (alphabetical source order) | none |
| `model_merged` | I | pre-Fox: several boots models merged into one `boots.model` and its `.mtl` (a shared boots folder holding several, or an `ingame_face.toml` player's boots parts, a combined boots link's model included), in the order `fmdl_merged`'s merge takes; the pre-Fox twin of `fmdl_merged` | none |
| `model_merge_flags_conflict` | E | pre-Fox: merged parts' `.model` headers carry different `flags`, a field of unknown meaning (0 in every file but two Konami face-montage models), so the merge has no rule to combine them | that package left out (`DropFolder`), as for `merge_material_conflict` |
| `model_hand_split` | I | a face model's vertices carry hand-skeleton (`skh_`) weights, so its hands were cut off at the wrist: on Fox into the player's gloves, on pre-Fox into two more `face.xml` entries, `<stem>_glove_l` and `<stem>_glove_r` (`pipeline.md` step 6 of "Per-export serial steps", `spec/model_conversion/hand_split.md` "Pipeline integration"; context: `model`, the model file, and `gloves`, the parts made: `glove_l`, `glove_r` or both). Reported by `compile`, where the model is compiled | none |
| `merge_material_conflict` | E | merged parts define the same material name differently (Fox merge or pre-Fox `ingame_face.toml` merge) | that package (face, boots or gloves) left out (`DropFolder`); the folder's other packages and its textures are still packed, so the player is built incomplete (beside a blank face, no head over the stock body): the Error on the player's folder is what tells the member the folder is not usable until the conflict is fixed, and the help says so |
| `skl_merge_conflict` | E | merged parts have incompatible skeletons (Fox compares effective SKL content; the pre-Fox merge compares `.model` bone matrices within `1e-4` per component, "`pes_model::ops::merge`" in the Libraries plan) | that package left out (`DropFolder`), as for `merge_material_conflict` |
| `skl_no_slot` | W | Fox: a member's `.skl` paired with a model resolving to `face_high`/`hair_high`/`oral` (no skeleton slot exists for those; context: `file`, the `.skl`, or the `.common` link whose Common model it pairs with). The skeleton a conversion generates for a model of those roles or for a glove part is dropped with no finding (`pipeline.md` step 3 "Format conversion"): the member authored no file, and every converted face gets one | SKL ignored (no-op file) |
| `face_file_not_used` | I | a face file (`face_diff.bin`, `face_diff.xml`, and on PES 18-21 `fcl_hair_sim.fclo`, which on PES 15-17 is the other engine's companion and reported by nothing; on PES 15-17 also a member's `face.xml` under `ingame_face.toml`) in a player folder with no face model (a link to a `Faces/` folder holding none gives none; on PES 15-17 a `face.xml` of its own gives it one), under `ingame_face.toml` or not, in a shared `Faces/` folder in the same case, or in a shared `Boots/` or `Gloves/` folder, which gives no face; a blank face folder always takes the bundled face diff (context: the file) | file not read |
| `file_not_used` | W | a file a model folder admits that no package reads, planning giving it no role (`pipeline.md` "2. Per-export serial steps", the face-file paragraph): a `.skl` pairing no model, a `.common` link to a kind no role takes (on PES 15-17 a per-kit variant model's too, and any link in a shared folder, deferred until the cup next plays a pre-Fox version), a `materials.toml`, a `.xml`, `.bin` or `.fclo` under no face name, a `.txt` note; a texture of a folder that plans no package (a referee folder or an `ingame_face.toml` player holding textures and no model: no textures task is planned, `pipeline.md` step 3); a `Common/` file of a kind no task reads (`.fclo`, `.xml`, `.bin`, `materials.toml`, a `.txt` note); a refs export's kit folder, logo, portrait or collar file, which have no referee to go to. Not reported: a model another representation of its stem beats and the other engine's companions (`pipeline.md` step 3 "Format conversion"), a left-out kit variant (`kit_variant_model_left_out`), a Fox `face.xml` (`xml_ignored_fox`), a Common model, `.mtl` or `.skl` directly in `Common/` no link or conversion takes (`Common/` is a library), and a file the structure pass already named (`file_type_disallowed`). Over mapped player folders, every shared folder, `Common/` and the refs export's root content, by `check` too (context: `file`, the file's path below the folder, or its export path on the export) | file not read (`Keep`) |
| `ingame_face_explicit_face_model` | E | `ingame_face.toml` beside explicit face models (`face_high`/`hair_high`/`oral`, in any supported source format), a `.common` link to one, or a shared face link, which require the face folder the marker suppresses | player folder discarded (`DropFolder`) |
| `ingame_face_marker_renamed` | E | a player folder holds `ingame_face` or `ingame_face.txt`, the in-game face marker's old name (context: the file) | file ignored (`DropFile`, not pass-through-eligible): it marks nothing and the folder compiles as one without it (a blank face package unless the folder gives a face); renamed to `ingame_face.toml`, an empty file gives the game's default face |
| `shared_texture_conflict` | E | two shared folders a player combines, of different packages, hold one texture path with different bytes, a texture `.common` link standing there counting as differing from a file (the player's own copy wins over either, `shared_texture_overridden`) | the player folder left out (`DropFolder`); identical bytes deduplicate |
| `shared_texture_overridden` | I | a texture path the player's own folder holds, as a texture or as a texture `.common` link standing there, that a shared folder he combines also holds, the two copies differing or one of them a link (two links naming the one `Common/` texture, nothing is reported): his is the one every model of his names, local files winning over imported ones (`pipeline.md` step 6; context: `texture`, the path below the texture home without its extension; `folder`, the shared folder's export path) | the shared folder's copy left out |
| `merged_texture_conflict` | withdrawn | nothing produces it: its one case, the player's own folder against the face folder he combines, is `shared_texture_overridden` since 4.y-sub (b3b3), and a player combines one folder of each kind (2026-10-10) | — |
| `model_gltf_unsupported` | E | a model stem's selected representation is glTF (no target-native model of its stem beside it, in a player folder or a shared one), which the compiler does not read until Phase 7; never compiled from the opposite native format instead, which the glTF beats (`pipeline.md` step 3 "Format conversion"); a glTF a target-native model of its stem beats is ignored with no finding; a `Collars/` glTF too, on both engines (`pipeline.md` "Collars"), and a glTF directly in `Common/` with no model of the target's format of its stem beside it, dropped the file alone (`file`, its export path; a model of the other engine's format of that stem is beaten by it, as in a player folder, and a mapped player whose `.common` link names that beaten model is dropped, `file` the glTF's export path, as a player linking a shared folder's glTF is); a shared folder's glTF is reported on each player folder linking it, `file` the shared file's export path, the shared folder removed from the export (`pipeline.md` step 3 "Format conversion"); reported by `compile` at planning, where the folder's tasks are made, so `check` does not report it (TC-MOD-28; context: `file`, the file below the folder, or a linked shared folder's file by its export path) | folder discarded (`DropFolder`), a linking player folder with it; a collar file discarded (`DropFile`), its export compiled without it; not pass-through-eligible |
| `model_conversion_failed` | E | selected model cannot be converted to the target format: the converter returns an error, or the target format's `check` fires an Error other than the far vertex on the converted form (context: `model`, the source file below the task's folder, or by its export path when a player's task converts a shared folder's file, the finding being on the player's folder; every `model` context a model folder's task reports is named the same way, a split's and a texture's included; a collar task's, whose scope is the file itself, is the file name); an FMDL collar converted for PES 15-17 too, and a `.model` collar converted for PES 18-21 with the templates' `uniform.mtl` (`pipeline.md` "Collars") | that package left out (`DropFolder`), as for `merge_material_conflict`: a task's failure is its package's, and the folder's other packages and its textures stand (TC-MOD-29); a collar file discarded (`DropFile`), its export compiled without it, the team's kits wearing the game's own collar of the ID planning gave them before the task ran; not pass-through-eligible, the source is in the other engine's format |
| `vertex_too_far_from_origin` | E | a model has a vertex more than 5000 units from the origin (the format crate's `fmdl_vertex_far_from_origin` / `model_vertex_far_from_origin`, run on every model the compiler processes: native sources as loaded, converted and glTF sources in their target-format form, before any merge; context: model file, offending vertex count). Text: "Vertex too far away, it will cause persistent lag for the whole matchday" | from the deep pass, which reads the native sources, the folder is discarded (`DropFolder`, TC-MOD-30); from the task that converts a source, on its target form, that package is left out, as every task failure (`model_conversion_failed`); not pass-through-eligible |
| `folder_pack_failed` | E/F | a task cannot build its packed batch | folder/task: `DropFolder`; output-writer/global: `AbortRun` |
| `model_name_invalid` | E | pre-Fox: a model anywhere in a shared boots or gloves folder without its category's suffix, the pre-Fox twin of `fmdl_name_invalid` | folder discarded |
| `xml_broken` | E | XML fails to parse (with line/column) | folder discarded |
| `mtl_broken` | E | MTL fails to parse (with line/column) | folder discarded |
| `edithair_unsupported` | E | pre-Fox: `face_edithair.xml` / `hair.xml` present (anywhere in a player or shared folder; reported by the structure pass, so `check` reports it) | folder discarded |
| `file_type_disallowed` | E/I | a file in a player or shared folder's tree, in a kit folder or in `Collars/` that its position's allowlist row does not admit (`spec/aesthetics_export/object_model.md` "File-type allowlist": a disallowed extension, or a marker, `settings.toml`/`ingame_face.toml` or per-player singleton below a subfolder; E if `strict_file_type_check`, else I) | folder discarded / kept |
| `fmdl_texture_not_found` | E/W | Fox: a texture one of the model's meshes uses is supplied by nobody: its stem resolves to no file of the folder, or its path names one below the model's folder (`./textures/skin`) where none sits, or its path names the team's Common output, where neither the export's `Common/` nor an installed CPK holds it (`pipeline.md` "Resolved decisions", "A texture a model names must exist"; context: `model`, the model file, and `texture`, the texture path the model now names). W when no install could be read to look, since the texture may be there. Reported by `compile`, where the model is compiled | that package left out (`DropFolder`), as for `merge_material_conflict`, naming the first missing texture; kept when W, one per missing texture |
| `fmdl_texture_kit_path` | W | Fox: a texture path one of the model's meshes names in the game's kit texture folder (`/Assets/pes16/model/character/uniform/texture/`): the modded exe swaps the kit textures per match, so a fixed file there is not the kit the player wears; the model's texture should be the `dummy_kit` family or a `kitN` texture of its own (`spec/model_format.md` "Kit-dependent assets"). Context: `model`, `texture`, the path. Reported by `compile`, once per model and path (a hand-split model's face and gloves packages each), instead of `fmdl_texture_not_found` | kept, the path unchanged |

**Model checks** (the format crates' `check`, run by the deep pass on every native model as it
is loaded, whatever the target, and from the conversion steps on, on each converted model in its
target form. A finding keeps the format crate's code and its severity, which "What makes a
finding an Error" sets; the one renamed is the far vertex, which both formats report as
`vertex_too_far_from_origin`, above. An Error discards the folder and is
pass-through-eligible, since the file can still be packed as it is; a Warning or an Info changes
nothing. On a converted form only an Error acts (`model_conversion_failed`, the far vertex,
both the package's); its Warnings and Infos are not reported: they describe the converter's
output (every converted FMDL trips `fmdl_weights_not_normalized`), and what the member can
change is in the source, which the deep pass checks with every rule. One finding per file and
code, however many meshes trip the rule. Context: the model
file, and the number of items that tripped it over the whole file: faces, vertices or bone
slots for a rule about a mesh's contents, and the meshes, materials or bones themselves for a
rule about one of them (two empty meshes count 2). A finding on a
`Common/` model acts on that file, and the players linking it follow `link_target_dropped`)

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `model_broken` | E | a model file (`.fmdl` or `.model`) does not parse (context: the file, the reader's error) | folder discarded; not pass-through-eligible (it cannot be processed) |
| `fmdl_mesh_over_bone_limit`, `fmdl_mesh_over_vertex_limit`, `fmdl_mesh_over_face_limit` | E | Fox: a mesh exceeds a hard FMDL limit; the game cannot load it | folder discarded |
| `fmdl_face_index_out_of_range` | E | Fox: a face names a vertex that does not exist | folder discarded |
| `fmdl_mesh_unassigned` | E | Fox: no mesh group lists the mesh, so the written file would not contain it | folder discarded |
| `fmdl_bone_slot_out_of_range` | W | Fox: a weighted bone index points past the mesh's bone group; the game ignores that weight (Konami's own models do this) | none |
| `fmdl_mesh_empty` | W | Fox: a mesh with no faces | none |
| `fmdl_duplicate_bone_name` | W | Fox: two bones of one name; merges and `.skl` matching get ambiguous | none |
| `fmdl_weights_not_normalized` | I | Fox: a vertex's weights sum to neither 255 nor 0 | none |
| `fmdl_material_unused` | I | Fox: a material instance no mesh uses | none |
| `model_mesh_over_bone_limit`, `model_mesh_over_vertex_limit`, `model_mesh_over_face_limit` | E | pre-Fox: a mesh exceeds a hard `.model` limit; the game cannot load it | folder discarded |
| `model_face_index_out_of_range` | E | pre-Fox: a face of any LOD level names a vertex that does not exist | folder discarded |
| `model_bone_slot_out_of_range` | W | pre-Fox: a weighted bone index points past the mesh's bone group | none |
| `model_mesh_empty` | W | pre-Fox: a mesh with no faces | none |
| `model_duplicate_bone_name` | W | pre-Fox: two bones of one name | none |
| `model_lod_record_mismatch` | W | pre-Fox: the LOD record's level count disagrees with the meshes' LOD levels | none |
| `model_weights_not_normalized` | I | pre-Fox: a vertex's weights do not sum to 1 | none |
| `model_degenerate_face` | I | pre-Fox: a face uses one vertex twice | none |
| `model_material_unused` | I | pre-Fox: a material name no mesh uses | none |

The `.mtl` checks (`mtl_material_duplicate`, `mtl_state_*`) are in "XML/MTL content checks"
below; the deep pass runs them on every `.mtl` it reads: on PES 15-17 every one, on PES 18-21
the one a selected `.model` or a Common `.model` link pairs with, in a model folder or in
`Common/` (the `mtl_texture_not_found` row). It also reports `model_material_undefined` for
each pre-Fox `.model` (or `.common` link to one) of a model folder, pairing it with the `.mtl`
the face step's search finds (`spec/model_format.md` "Pre-Fox: the `.mtl` a `.model` uses"), so
`check` reports it too.

**Textures** (file-scoped; in model folders the folder fails, and so does a kit, whose config
names its textures; elsewhere (`Common/`, portraits) the file is dropped; the logo sources are the
exception — main and `_small` fail as one atomic producer of the game's three sizes. A file is
`texture_type_mismatch` when its bytes open with the signature of another accepted format than
its extension's; TGA has no signature, so a `.tga` is a mismatch only when it opens with
another's. An uncompressed texture is no finding, a kit's included: it is encoded like any
raster source (`spec/libs/dds_convert.md` "Passthrough"), which is what PES 15–17's crash on
uncompressed kits needed)

The deep pass reports `texture_type_mismatch`, `texture_too_small`, `texture_not_pow2`,
`texture_not_div4` and `kit_texture_too_big` from each texture's header (`probe` in
`libs/dds_convert.md`), without decoding it, so `check` reports them and the folder is dropped
before any ID is planned for it. A mismatched file gets that one finding: its header is another
format's, so its size is not read. `texture_type_mismatch` is not pass-through-eligible, because
the file cannot be converted as the format its name declares; nor is `texture_not_div4`, because
every pre-Fox texture is written block-compressed (BC1 or BC3), and the pre-Fox games' Direct3D 9
renderer cannot create a block-compressed texture whose sides are not multiples of 4. The other
three size findings are eligible: the texture converts, and what the game makes of it is the
member's risk. A texture whose header the
probe refuses is a finding here too, at the same place as a mismatched file's:
`texture_unreadable` when the header cannot be read at all (bytes that are no texture, a header
cut short), `texture_codec_unsupported` when the probe refuses its codec or its kind of DDS, the
two arms conversion itself calls a codec refusal (`processing::texture::conversion_failure`), so
that the file never reaches a task that would fail whole with it (a Common texture's task packs
every Common texture). A cube map skips the probe: the 2D reader refuses every cube map, and the
compile never decodes one (it goes out as it is, or through the FTEX container conversion), so
a cube map the Fox route refuses still fails its task. Pixel data cut short, which no header
shows, still fails the task (`folder_pack_failed`).

A portrait (a `Portraits/` file or a player folder's `portrait.*`) is checked the same way and
dropped alone, its player folder keeping everything else. Its sides must be powers of two
whatever its level count and the target. A logo source is the one texture the deep pass
decodes in full (an export has at most two, and nothing else can tell that the game's three
sizes can be made from it): one that does not decode is `logo_file_invalid`, not
pass-through-eligible, and the logo goes as one unit.

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `texture_too_small` | E | a side under 4 pixels, one block | discarded |
| `texture_not_pow2` | E | a side that is not a power of two on a portrait (any target), or neither side a power of two on a Fox target on a texture that carries a mip chain: a raster source always (its chain is generated), a DDS or FTEX source with more than one level (a single-level one passes at any size). PES 21 draws a mipped texture with one such side (512x461, 797x64) and a 4096 kit. Not reported on a kit's main texture on PES 15-17, where `kit_texture_too_big` covers it | discarded |
| `texture_not_div4` | E | pre-Fox: a side that is not a multiple of 4 (every pre-Fox texture is written block-compressed); not pass-through-eligible | discarded |
| `kit_texture_too_big` | E | a kit's main texture (`kit`, its own or the one inherited from `all/`) wider or taller than 2048 pixels, or with a side that is not a power of two; PES 15-17 only | discarded |
| `texture_type_mismatch` | E | header doesn't match extension (renamed, not resaved) | discarded |
| `texture_unreadable` | E | the header cannot be read as the format the name declares and matches no other accepted format's signature (not a texture, or cut short before its header ends); reported by the deep pass from the header at the place's disposition, as `texture_type_mismatch` is (the file alone in `Common/`, so its other textures are kept; the folder for a model folder's or a kit's); not pass-through-eligible | discarded |
| `texture_codec_unsupported` | E | codec not convertible in-process (`dds_convert` refuses the codec, or the DDS reader refuses the file as a DDS: a signed block format, a volume, an array, a paletted DDS, a header of the wrong size, a zero side, an impossible level count, premultiplied alpha, an unknown four-character code); reported by the deep pass from the header at the place's disposition, as `texture_unreadable` is, and by the Fox cube-map route when `dds_to_ftex` refuses one; not pass-through-eligible | discarded |
| `portrait_header_rewritten` | I | a DDS portrait under a DX10 extension header went out under the legacy header of its format, an sRGB DXGI id as its UNORM twin, its pixel data unchanged (`spec/aesthetics_export/player_folders.md` "Portraits": such a file crashed PES 19; context: `file`; `dxgi`, the id the source carried) | none |
| `texture_stem_conflict` | E | two image files with the same stem in one lookup namespace, whatever their extensions: one folder of a model folder's tree, the root or one subfolder (`hair.dds` beside `hair.png`, or a texture link `hair.png.common`, which counts as a file of its linked name; `hair.dds` beside `jessie/hair.dds` is no conflict, a model's texture name resolving nearest first, `spec/aesthetics_export/player_folders.md` "Subfolders"), `Common/` (`hair.dds` beside `hair.png`; each directory of `Common/` a namespace of its own), a kit folder, or `Kits/all/` (`kit.png` beside `kit.dds`; a kit's own file overriding an `all/` file of its stem is not a conflict), or `Portraits/` (`05.dds` beside `05 - Bob.png`, two files of one slot) | folder discarded (the model folder, or the kit; for `all/`, `all/` itself, so no kit inherits from it); in `Portraits/` and `Common/`, both files (a player linking a dropped Common file follows `link_target_dropped`) |

**XML/MTL content checks** (pre-Fox, plus `face_diff.xml` in Fox)

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `mtl_texture_not_found` | W | texture (by stem) used by one of the model's meshes supplied by nobody: resolving nowhere from the `.mtl`'s folder, not at the path below the `.mtl`'s folder a `./textures/skin.dds` names, and for a Common path neither in the export's `Common/` nor in an installed CPK (the lookup of `fmdl_texture_not_found`); a Warning where Fox's is an Error, because the game plays such a face (the paragraph below). On Fox the same check runs, in the deep pass too, on a `.mtl` a selected `.model` pairs with (`pipeline.md` step 3 "Format conversion"): the file checked is the member's source, the same whatever the target, so the finding is the same; a `.mtl` no selected model pairs with is read by nothing there | folder kept |
| `mtl_texture_unused_missing` | I | texture referenced only by materials no mesh uses | none |
| `xml_root_tag_invalid` | E | root tag is not `<config>` | folder discarded |
| `xml_model_type_missing` | E | `<model>` without `type` | folder discarded |
| `xml_model_path_missing` | E | `<model>` without `path` | folder discarded |
| `xml_model_not_found` | E | `path` (or `material`) is a `./` or Common reference whose file is missing from the export | folder discarded |
| `xml_common_path_invalid` | E | Common reference without 3-char subfolder | folder discarded |
| `mtl_common_path_invalid` | E | on a pre-Fox target, `xml_common_path_invalid`'s rule for a `.mtl`: a sampler `path` below `model/character/uniform/common/` whose subfolder is not three characters long (`common/o/env.dds`) — the team's Common is written under its three-digit ID, so such a path reaches nothing (on Fox the conversion rewrites the paths, so the rule is meaningless there) | folder discarded (the `.mtl`'s folder; a `Common/` `.mtl`'s, the file alone) |
| `xml_oral_prefix_missing` | E | PES16 target: model file name starting with none of `face_high_`/`hair_high_`/`oral_` | folder discarded |
| `xml_dif_conflict` | E | two sources for one datum in one folder: the xml carries a `<dif>` and the folder also has `face_diff.xml`, or, on either engine, the folder has both `face_diff.bin` and `face_diff.xml`. A `<dif>` beside a `face_diff.bin` is no conflict: it is the dual-engine layout of the current referee exports, the xml's `<dif>` going out on PES 15-17 (the `face_diff.bin` ignored there with no finding, as a file only the other engine reads) and the `face_diff.bin` on PES 18-21 (where the xml is `xml_ignored_fox`) | folder discarded |
| `xml_element_unknown` | W | a child of `<config>` other than `<model>`/`<dif>` | kept verbatim |
| `xml_attribute_unknown` | W | a `<model>` attribute other than `level`/`type`/`path`/`material`/`ratio` | kept verbatim |
| `xml_type_unknown` | W | `type` not in the generated vocabulary (the type table plus `uniform_sub`) | kept verbatim |
| `xml_level_lod` | I | `level` other than `0` — the model's LoD level; higher levels are uncommon but known to work | kept verbatim |
| `xml_ratio_invalid` | W | `ratio` that is not a number | kept verbatim |
| `xml_path_unchecked` | W | `path`/`material` in a form the compiler cannot resolve (`model/character/face/common/…` or any other game path): not verified to exist | kept verbatim |
| `xml_face_neck_multiple` | W | more than one `face_neck` entry | kept verbatim |
| `xml_model_unlisted` | W | a model file in the folder that the xml does not reference (a reference to a per-kit set names every variant of the set) | file not emitted, nor read by the deep pass; a `.mtl` the xml does not name is unread the same way, with no finding |
| `xml_face_neck_added` | I | no `face_neck` entry among a face's models; the dummy entry was appended (Red's rule); not on a blank face | dummy model + mtl emitted |
| `xml_uniform_pes15` | I | PES15 target: `type="uniform"` rewritten to `uniform_sub` (Red's rule) | rewritten |
| `xml_ignored_fox` | I | a user `face.xml` in a folder compiled for a Fox target | xml ignored; models compile by the normal route |
| `xml_ignored_shared` | I | PES 15-17: a `face.xml` in a shared `Boots/` or `Gloves/` folder, whose output (one model, a `glove.xml`) no face xml drives (context: `file`) | xml ignored and not checked; the deep pass pairs every model of the folder as without an xml |
| `xml_shared_face_conflict` | E | PES 15-17: a player folder holding a `face.xml` links a `Faces/` folder, whether or not that folder holds one: a face takes one xml (context: `file`, the player's xml; `link`, the shared folder's export path) | folder discarded (`DropFolder`, not pass-through-eligible) |
| `face_diff_invalid` | E | `face_diff.xml` is not base64 text or a `<dif>` holding it, or its decoded bytes, or a `face_diff.bin`, are not a face diff (`spec/aesthetics_export/player_folders.md` "`face_diff.xml`"; context: the file, the reason) | folder discarded |
| `mtl_material_duplicate` | E | material listed twice | folder discarded |
| `mtl_state_invalid` | E | `ztest` ≠ 1 / `blendmode` ∉ {0,1} / `alphablend` ∉ {0,1} | folder discarded |
| `mtl_blendmode_nonzero` | W | `blendmode` = 1 (works, not recommended) | none |
| `mtl_state_missing` | I | state names absent, defaults used | none |
| `mtl_state_nonrecommended` | I | `alphablend`/`zwrite` combination not recommended | none |
| `mtl_state_unknown` | I | a state name outside the seven the schema knows (`shadowcaster` in 18 of Konami's files) | kept verbatim |
| `model_material_undefined` | E | a material name a `.model` uses has no entry in the `.mtl` it is paired with; the mesh would render with the game's fallback (context: the model, the `.mtl`, the undefined names in the model's order). Also a `.model` the search pairs with no `.mtl` (context: the model) | folder discarded; pass-through-eligible only when a `.mtl` was found (the file packs as it is), never when none was (the `face.xml` must name one) |

Red's "no models from Common on PES16" check is intentionally **not** ported: the plan treats the
PES16 exe as patched to load models from Common (the patch is made when the cup next plays PES
16), so the compiler must not reject such references.

**User-supplied `face.xml`.** A pre-Fox model folder may ship its own `face.xml` instead of having
the compiler generate one. This is **second-class, accepted on purpose**: the generated xml covers
everything the format knows how to express, and a hand-written one exists to let a user try what the
pre-Fox engines can do that nobody has mapped yet (a `type` no table lists, a `level` other than 0,
an element the generator never writes). Because a malformed xml can crash the game, the compiler
checks it carefully, and the severity line is drawn by evidence, not by taste:

- **Error** — what is known not to work, or cannot work: unparsable XML, a root other than
  `<config>`, a `<model>` without `type` or `path`, a `./` or Common reference to a file the export
  does not contain (the one thing the compiler can verify outright), a Common path without its
  3-character subfolder, the PES16 `oral_`/`face_high_`/`hair_high_` name rule, and two sources for
  the face diff. These are Red's checks (`xml_check.py`), each added after real breakage, plus the
  two structural impossibilities Red happened not to test (a missing `path`, a doubled diff).
- **Warning** — everything outside the vocabulary the compiler's own generator emits, because
  that vocabulary is the in-game-verified set and nothing outside it is: an unknown element or
  attribute, a `type` not in the type table, a non-numeric `ratio`, a path in a form the
  compiler cannot resolve (`model/character/face/common/…` and any other game path; the
  compiler says it did not check, while Red let only `model/character/face/common/…` through
  silently and reported any other form it could not resolve as an Error for a model or `.mtl`,
  a Warning for a texture: a pre-Fox gap, worklog "Issues"). The value is **kept verbatim**: a
  warning is the compiler saying "I can't vouch for this", never "I changed this". A model file in
  the folder that the xml does not list is also a warning — it will not be in the game, which is
  almost always a mistake and occasionally the point.
- **Info** — the compiler's own rewrites, which it performs on a user xml exactly as it would on a
  generated one and reports so the user knows the emitted file differs from the source: the
  `face_neck` dummy appended when no entry has that type; `uniform` → `uniform_sub` on PES15.
  What a user writes for the compiler to fill in is filled in without a finding, as in a
  generated xml: the team ID in place of a Common path's 3-character subfolder, and the `<dif>`
  block from `face_diff.xml` when the xml has none; `kitN` tokens pass through. On a Fox target
  the xml is ignored with an info line and the folder's models compile by the normal route (Fox
  has no `face.xml`). Also info, though not a rewrite: a `level` other than 0 — it is the model's
  LoD level, and higher levels are merely uncommon, not unverified (a value that has already made
  the move from warning to known).

Rules that follow from "the xml is the authority in its folder": filename typing is off — the
suffix table is for generating an xml, and this folder has one; `.common` model links are not needed
(the xml names Common paths itself) and an unlisted one is just an unlisted file; only what the xml
references is emitted, plus the MTLs and textures those models bind, which go through the ordinary
MTL and texture checks unchanged. The compiler always re-serializes the xml it emits (it has to, for
the ID substitution and the appended entries), so comments, whitespace and the encoding declaration
of the source never reach the game — the checks above are about content, not formatting. New
knowledge moves a row: when an experiment shows a value works, it joins the generated vocabulary and
stops warning; when one shows a value crashes, it becomes an error with the crash as its citation.

How a `path` or `material` is resolved and written, Red's rules (`xml_check.py` `listed_file_check`,
`update_xml_for_renamed_common_models`), since the one real member xml in the fixtures (the pre-Fox
tracer's Fumos folder: `./face_high_*.model` beside `face_high_win32.model`) was written for them:

- A `./<name>` reference names a file of the face (the player's own files, directly in the folder,
  then a linked shared face's), and `./<path>/<name>` (`./jessie/body/x.model`) a file of the
  player's subfolder at that path, relative to the referencing file
  (`spec/aesthetics_export/player_folders.md` "Subfolders"), compared case-folded with the reference's `*`
  read as `win32`; the referenced file is **packed under the name the reference gives** (`*` as
  `win32`, spelled as the xml spells it) and the reference is written as it is. We do not
  rename the file to its `oral_<stem>_win32` packed name and respell the reference, because the
  xml is the member's statement of what the game should load, and Red kept both as written.
- A `model/character/uniform/common/<3 chars>/<name>` reference has the 3-character segment
  replaced by the team ID (any other length is `xml_common_path_invalid`), and `<name>` (`*` as
  `win32`) names a file directly in the export's `Common/`, case-folded, else `xml_model_not_found`;
  `<subfolder>/<name>` (`.../common/999/refkit/oral_arm_*.model`, `<subfolder>` at any depth)
  names a file in that
  subfolder of `Common/`, which the team's Common output packs at its own path (`pipeline.md`
  "Common"), and in a refs export a file of the referee template's `common/999/` tree counts
  too when the export does not hold one of that path (`blue_port.md` "Referee export
  processing").
  A `.model` is written as the team's Common output packs it, `oral_<stem>_*.model` (the Common
  output renames every model, as Red's `model_names_fix` does, and Red then respells the xml);
  a `.mtl` keeps its name there.
- A kit token in a referenced name (`pants_kitN.model`, `pants_kit2.mtl`) passes through: the
  reference exists when a file of its set is among the files searched (`kit_variants`), and a
  `./` reference to a `.model` set packs every variant under its own file name, since the game
  respells the entry for the kit picked, and completes the set as a generated face's is
  (`pipeline.md` "Kit-dependent assets", `kit_variant_missing`).
- A `material` is resolved the same way; an entry without one is written without one.
- Any other path form is `xml_path_unchecked` and written as it is.

What is emitted from such a folder: each referenced `.model` under its name, each referenced
`.mtl` under its own name with its texture paths pointed as a generated face's are, the
folder's textures as usual, and the xml. The `.mtl` the deep pass compares a listed model's
material names with (`model_material_undefined`) is the entry's `material`, not the search's
(`mtl_search`), which the xml overrides; an unlisted `.model` is `xml_model_unlisted` and is
neither emitted nor compared. The xml's own `<dif>` is decoded as a `face_diff.xml`'s is and
checked the same way (`face_diff_invalid`); beside a `face_diff.xml` it is `xml_dif_conflict`, while
beside a `face_diff.bin` it is the dual-engine layout, each engine taking its own (the catalog
row); with none of the three the bundled face diff is written. A `<model>`'s
attributes are written in source order, verbatim but for `type` (the version rewrite) and the
two references above; an unknown element is written verbatim with its attributes, text and
children. The written order is the children in source order, then the `face_neck` dummy when
appended, then the `<dif>` last, as the game's own files and Red's output place it. A file two
references name (a `.model` listed twice, a `.mtl` under two spellings) is packed once, under
the first. A `kitN` reference to a `.mtl` set packs every variant as a `.model` set's does: a
`pants_kitN.mtl` packed under that name is a file the game never asks for. An xml naming the
dummy's own names (`./dummy.mtl`, `./oral_dummy_*.model`) without a `face_neck` entry fails the
task when the dummy is appended, since two files cannot share a name; no export does this. `xml_uniform_pes15` on a
user xml names the entry by its `path` value, since an entry need not resolve to a file. The hand
auto-split does not apply to such a folder: the xml says what the face loads, and a split would
add glove entries the member did not write. A folder holding a `face.xml` has a face, whatever
models it holds (an xml naming only Common models is one), so its face files are used.

A `Faces/` folder's `face.xml` is the xml of every face linking the folder. The deep pass
checks it in the shared folder's own pass as a member's own (its `./` references naming the
shared folder's files, a model of the folder it does not list `xml_model_unlisted`). A
linking player's face is the shared folder's copy with his own files on top ("A link plus
local models combines" in `aesthetics_export/player_folders.md`), and its xml is the shared
one: a reference resolves over the face's files as above, his own file first, and each of
his models the xml does not name gets the entry a generated xml would give it, appended
after the xml's children and before the `face_neck` dummy and the `<dif>`. His own
`face_diff.xml` or `face_diff.bin` is the face's diff over the shared xml's `<dif>`, as any
of his files wins over the shared folder's. We append rather than generate a second list,
because the shared xml is the author's statement of what the base face loads and the
player's parts layer over it as his files do over the shared ones; the face has an xml, so
the hand auto-split does not apply to it. A player folder linking a `Faces/` folder holds no
`face.xml` of its own: two xmls for one face leave no rule for which one speaks, so his is
`xml_shared_face_conflict` and the folder is discarded, whether or not the shared folder
has one. With neither, the face's xml is generated as usual. A `Boots/` or `Gloves/`
folder's `face.xml` is ignored on PES 15-17 as on Fox (`xml_ignored_shared`): its output is
one model or a `glove.xml`, which no face xml drives.

Texture existence is checked **deep**, not shallow: `pes_model` parses the `.model` files and knows
which MTL material each mesh actually binds, so a missing texture on a mesh-used material is a hard
error, while a miss on a material no mesh uses can never break the model and is only reported as
info. Red only reads the MTL and cannot tell used materials from unused ones, so it has to report
every miss as a warning.

On pre-Fox the check runs in the deep pass, which already pairs each `.model` with its `.mtl`
(the search, or the entry of the member's own `face.xml`) and has read both, so `check` reports
it as `compile` does; Fox's `fmdl_texture_not_found` runs in its package's task instead, where the
merged model's pointed paths are known. A `.mtl` material is **mesh-used** when a model paired
with that `.mtl` binds its name. Every `.mtl` of a folder is checked, so one no model pairs with
(an unlisted `.mtl` beside a member's xml) reports its misses as `mtl_texture_unused_missing`.
A sampler path, whatever directory it spells (but a `./<dir>/` one, which resolves at that path
alone, `spec/model_format.md` "Stem-based texture references"), is supplied when its stem (or a
variant of its
set, for a `pants_kitN` reference) is one the folder's lookup resolves for it, nearest first
from the `.mtl`'s folder (`spec/aesthetics_export/player_folders.md` "Subfolders"): a texture among the folder's own
files or its linked shared face's (for a referee, whose face packs his combined boots' and
gloves' textures too, those folders' as well), or the linked stem of a texture `.common` link;
this is the stem rule the face task points `.mtl` paths by, so the check and the pointing
agree. A shared face folder's `.mtl` files are checked in that folder's own pass against its
own textures: a shared face is complete by itself, and a texture a player holds of a stem it
names does not count. Past the folder, a `./` or bare path is missing (the face packs no
texture the folder does not hold); a path of the Common form
`model/character/uniform/common/<3 chars>/<path>` (`<path>` a name, or a `Common/` subfolder's
`jessie/skin.dds`, at any depth; a `kitN/` directory stands for its set, a file of the set supplying it, as in
`pipeline.md`) is supplied when `Common/` holds the stem at that path, or an installed CPK
holds `common/character1/model/character/uniform/common/<team>/<path>` as a `.dds`
(the walk of `fmdl_texture_not_found`; a lookup that cannot be made, or an export with no team
ID, holds nothing); any other path names the game's own files and is not looked up, nor is a
`dummy_*` stem. One finding per `.mtl` and distinct path, naming the file, the path as written
and the materials naming it; a folder an xml Error drops gets none. **The finding is a Warning
that keeps the folder**, where Fox's is an Error: the pre-Fox tracer's Fumos face names
`./face_edithair_specular_roughness.dds` on a mesh-used material, holds no such file, and played
a cup as Red compiled it with a warning. Which samplers a pre-Fox shader reads is not known, so
the compiler cannot tell that harmless miss from one that shows, and refusing a face the game
plays would fail the parity standard; the Warning tells the member where to look. A `Common/`
`.mtl` is packed once for the team, so its mesh-used names are those every kept `Common/`
`.model` binds, and its paths resolve among the `Common/` textures and the installed CPKs. A
Common-form path the check calls supplied is still packed with its segment as written (the
face task points only stems the folder holds): whether Red respells it is a question for the
pre-Fox parity step. A pre-Fox texture `.common` link
whose target is not in the export stays `common_link_missing`, not yet satisfied by an installed
CPK: the face task points a link by its target's name, so lifting that needs the task to point
a targetless link by the link's own stem, an improvement deferred until the cup next plays a
pre-Fox version (worklog step 4.y-sub, slice (b3b4e)).

**Kits**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `kit_folder_invalid` | E | subfolder name doesn't parse as `<slot>[ - <label>]` with the slot in `p1`–`p9`, `g1`, `all` | kit discarded |
| `kit_slot_duplicate` | E | two subfolders resolve to the same slot (`p1/` and `p1 - Lakers/`) | both discarded — never a pick |
| `kit_textures_inherited` | I | a kit lacks stems that `all/` provides; lists them (`p3: back, leg, name from all/`) | textures inherited |
| `kit_all_file_ignored` | W | `all/` holds something other than kit textures (`config.toml`, `colors.txt`, an `icon_<N>` marker, anything else) | file ignored |
| `kit_all_unused` | W | `all/` present but no kit folder to inherit from it | — |
| `kit_config_generated` | I | a `Full` export's kit with no `config.toml`; generated from the template, which carries the FPC values whatever the team's FPC status. In a `Midcup` export the kit keeps its slot's installed config instead, and the template is written (and this reported) only where nothing is installed for the slot (`pipeline.md` "Bins accumulation") | auto-fixed |
| `kit_config_invalid` | E | `config.toml` is not UTF-8 text or `kit_config` refuses it: not TOML, a key's value of the wrong type or outside its field's encoding, a key that must be a table holding something else; found by the deep pass (context: the file, the error). `kit_config`'s other findings (its two Infos, `kit_collar_zero`) are not reported yet (`pipeline.md` "Kit configs") | kit discarded (`DropFolder`, not pass-through-eligible) |
| `kit_config_version_clamped` | W | a supplied config's field doesn't fit the target PES version's encoding (e.g. a Name Y over 33 before PES 21), found by the deep pass (context: the field, the value, the version's maximum); or an installed config of a kit slot a `Midcup` export does not hold, encoded again for the FPC values or the export's collar, holds such a value (the team export's; context: the slot, the field, the value, the version's maximum; `pipeline.md` "Bins accumulation") | value clamped when the config is emitted |
| `kit_config_option_ignored` | W | a supplied config sets a sleeve or fit option its shirt model does not take (`kit_config`'s `kit_cut_out_requires_model_144_or_160`, `kit_undershirt_only_requires_model_144_or_160`, `kit_tight_requires_model_144_or_160`), found by the deep pass on the config as it is emitted, the FPC values applied when the team's kit-FPC status is On (context: the file, the option, the shirt model as emitted) | kept; the game ignores the option |
| `kit_config_fpc_adjusted` | I | team kit-FPC status is On but a config lacks the FPC values — supplied configs and unexported slots' base entries (pre-Fox: their loose configs) alike, GK kit included; an unexported slot's is the team export's (context: the slot), the slots being the kits its `UniColor.bin` record holds (`pipeline.md` "Bins accumulation") | auto-fixed (values written; FPC values are never auto-reverted) |
| `kit_config_fpc_unpatched` | W | team kit-FPC status is On but an unexported kit slot has no base entry or config to patch, or its entry does not decode as a kit config (the team export's; context: the slot) | slot left alone; the team needs a kit export |
| `kit_config_collar_unpatched` | W | the export has a collar but an unexported kit slot (one its `UniColor.bin` record holds) has no base entry or config to wear it, or its entry does not decode as a kit config, and `kit_config_fpc_unpatched` has not named the slot (the team export's; context: the slot; `pipeline.md` "Collars") | slot left alone, wearing its own collar; the team needs a kit export for it |
| `kit_placeholder` | I | a `Full` export's kit whose effective textures lack `kit.dds` (an empty folder included); the bundled checkerboard stands in. A `Midcup` kit without `kit.dds` keeps the installed textures instead, emitting what it holds | placeholder kit emitted: checkerboard texture, template config unless supplied, UniColor entry per the colors fallback |
| `kit_colors_derived` | I | a `Full` export's kit `colors.txt` missing, or present but yielding fewer than two valid colors; menu colors extracted from the kit's main texture, the `kit` texture of its effective set (an inherited `all/kit.dds` counts). A `Midcup` kit without a usable `colors.txt` reports nothing and keeps its installed `UniColor.bin` entry (`pipeline.md` step 2, **Kits**) | auto-fixed |
| `kit_colors_missing` | W | a `Full` export's kit with no usable `colors.txt` and no main texture in the kit's effective set to derive from (placeholder kits without a `colors.txt` always; a `Midcup` kit with neither reports nothing and keeps its installed `UniColor.bin` entry) | the magenta/black "no colors chosen" pair is written, so the gap shows in the game menus |
| `kit_icon_invalid` | W | a kit's `icon_<N>` marker is numbered above 23, or the kit holds two or more (each is reported) | default icon (3) used |
| `kit_texture_name_invalid` | E | texture doesn't carry the `kit` prefix (`kit.dds`, `kit_mask.dds`, …) | file discarded |
| `kit_texture_not_used` | I | a kit texture of the kit's effective set that the target does not emit: a `kit_mask` on a Fox target or a `kit_srm` on a pre-Fox target (the other engine's map), or a `kit_*` stem outside the seven the compiler builds (`kit_spec`; the five the config names, `kit_mask`, `kit_srm`) (context: the file) | file dropped from the set at planning, so neither read nor emitted (a map is never converted into the other) |
| `kit_layout_conflict` | E | both `pre-fox` and `fox` markers in one kit folder | kit discarded |
| `kit_layout_converted` | I | the kit's layout marker names the other engine than the target; names the direction (`p1: pre-fox → fox`) | `kit` and its mask/srm re-laid out per `KIT_LAYOUT_REMAP`; other textures untouched |

**Portraits / Logo / Collars / Common**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `portrait_name_invalid` | E | a `Portraits/` file that is not a texture named `NN` with NN in 01–23 (alone or followed by ` - ` and any text, as a player folder is numbered) | file discarded |
| `logo_file_invalid` | E | a root `logo*` file has an unknown tag in its stem, a disallowed format, or fails to decode in the deep pass | no logo emitted for the team (both files dropped as one unit — the game needs all three sizes) |
| `logo_role_duplicate` | E | two files claim the same role (two `logo*`, or two `logo_small*`) | no logo emitted for the team |
| `logo_small_without_main` | E | `logo_small*` present with no main `logo*` | no logo emitted (the small image is not a source for the large sizes) |
| `logo_fit_applied` | I | a non-square source was made square; names the file and the mode (`fit` by default, or the file's tag) (context: `file`, `mode`) | — |
| `logo_upscaled` | W | a source is smaller than its largest target (512² for main, 128² for small): its long side for `fit`, its short side for `crop` and `stretch` (context: `file`, `size` as `300x300`, `target`) | emitted upscaled |
| `collar_id_invalid` | E | collar filename doesn't parse as `collar_<ID>` (zero padding optional), or the ID is not a stock collar of the target version that a kit config can name (each version's set counted in its install's base data CPK, `dt35`'s `nocloth` collars: PES 15: 1-101 and 901-904; PES 16: 1-105 and 901-904; PES 17 and 18: 1-116 and 901-916; PES 19: 1-124 and 901-916; PES 20: 1-127 and 901-913; PES 21: 1-131 and 901-913; the 9xx collars are stock but a kit config holds a collar in one byte, so no team's kit could wear one: invalid too). The suite's reserved IDs are checked first (`collar_id_conflict`), so `collar_105` is a conflict on PES 15 too | collar file discarded |
| `collar_id_conflict` | E | the stock collar ID is already claimed: by another export earlier in this run (canonical export order), or by the suite itself, which reserves 105 (FPC) and 77 (the referees' marker, `blue_port.md` "Referee export processing") (context: the claimant, an export's name or `FPC` / `referees`) | the collar is discarded and its team's configs are not rewritten |
| `common_file_disallowed` | E/I | as `file_type_disallowed`, Common scope | files discarded / kept |

**Referees** — referee exports use the same player-folder format (see `blue_port.md` "Referee export processing"),
so all player-folder, texture, and XML/MTL messages above apply as-is. The referee marker needs
no code of its own on a refs export: on both engines it is a model on the referees' reserved
collar and a texture in their Common output, both inside the refs CPK (`blue_port.md` "Referee
export processing"), so its failures are the texture codes above, on `ref_marker.dds`. Red's
injection into the system `dt00_x64.cpk`, and with it `ref_marker_needs_consent` and
`dt00_write_failed`, is gone. The one addition guards the reserved collar on regular teams:

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `kit_collar_reserved` | E | a regular team's kit whose effective collar or winter collar, after FPC reconciliation and custom-collar rewriting, is 77, the referees' reserved collar (context: the field, `collar` or `winter_collar`; the kit folder is the message's scope) | kit discarded: its players would wear the referee marker |

**Output stage and savefile** (Run scope)

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `duplicate_path` | W/E | a task's entry, or a bin, at the path of an `overrides/` file (Run scope; context: the path). The folder and export forms (two tasks claiming one path) have no trigger today and are not built (`pipeline.md` "3. Per-model-folder parallel steps") | override: the `overrides/` file wins, the entry is left out (`Keep`, W); folder collision: losing folder blocked (`DropFolder`, E); whole-export collision: losing export blocked (`DropExport`, E) |
| `cpk_write_failed` | F | incremental CPK writing fails | run aborted and partial CPK discarded (`AbortRun`) |
| `output_commit_failed` | F | a completed CPK/tree cannot be atomically committed to its final output path | run aborted; prior published outputs remain untouched (`AbortRun`) |
| `patch_written` | I | the aesthetics patch was written to the output folder (names the path and the teams it covers) | none |
| `patch_write_failed` | F | the aesthetics patch could not be written (context: the path, the error) | run aborted (`AbortRun`): a deployment is rolled back, the old CPKs back in `download/`, and nothing more is published; a sideload tree stays in place |
| `savefile_autodetected` | I | `savefile_path = auto` resolved a savefile under Documents\KONAMI (names the path; noted especially when several account folders existed and the newest was chosen) | none |
| `savefile_missing` | W | the patch holds entries (Fox: motions and resolved names; pre-Fox: aesthetics) but no savefile is configured/found; the patch is the run's only savefile output | savefile step skipped; the message names the patch and the save editor's apply action (context: `patch`, `path` when configured, `command`). A configured path naming a folder is not missing: it is `savefile_invalid` |
| `savefile_skipped_pes_running` | W | savefile changes pending, the compile target's PES running, and `compile` not given `--edit-mode-closed` (a PES of another version does not count; any output mode; the motivating case is sideload-mode iteration); context `path`, `version`, `flag` | savefile step skipped; applied by the next compile given the flag or made with PES closed |
| `savefile_write_failed` | E/F | savefile could not be updated: the patch's apply refused it, or the write failed (context: the path, the error) | Fatal (`AbortRun`) after a deployment, which is rolled back and its CPKs promoted to `output/`; an Error in sideload mode, the new tree in place |
| `savefile_invalid` | E/F | the configured or found savefile cannot be read, or its version is not the compile target's (context: the path, the version) | Fatal (`AbortRun`) after a deployment, which is rolled back and its CPKs promoted to `output/`; an Error in sideload mode, the new tree in place; the save is not modified |
| `savefile_written` | I | the patch was applied to the local savefile and the savefile replaced, the previous file kept as `.bak` (context: the path, the team ids applied) | none |
| `savefile_team_missing` | E | a team of the patch is not in the savefile (context: the team) | that team's entries are not applied; the rest is; a save holding none of the patch's players (as `savefile_team_missing` or `savefile_player_missing`) is not rewritten, so its `.bak` stays the user's previous backup |
| `savefile_player_missing` | E | a player of the patch is not in the savefile, his team being there (context: the player id); the save editor's `apply-patch` reports the same skip | that player's entries are not applied; the rest is |
| `savefile_player_not_stripped` | W | Fox: a compiled player's record in the local save has its appearance id set (a save never stripped, or an id the game's Edit mode restored), so the game shows his record, not the compiled rows (context: the players) | reported; nothing to do until the compiler writes the look into the aesthetics patch (worklog 8.36c), the stripped route being gone |
| `teamnotes_write_failed` | E | `teamnotes.txt` cannot be written, or a previous one cannot be removed, in the output folder (context: `path`, `error`) | the run's CPK stays in place; the notes file is left as it was |
| `uniparam_compile_failed` | withdrawn | an installed `UniformParameter.bin` that does not parse is `installed_bin_unreadable` since 4.21c, and any failure writing the bin is `cpk_write_failed`; nothing between the two can fail (converge 4.y-conv, 2026-10-09) | — |
| `pes_folder_not_found` | E | `pes_folder_path` invalid at deployment time (includes the no-PES-install machine; context: `path`, `output` the path the CPK is promoted to) | deployment skipped; staged CPKs promoted to `output/`; savefile step skipped |
| `pes_version_mismatch` | W | `pes_folder_path` exists but holds no `PES20{pes_version}.exe` — a different-version exe suggests a wrong `pes_version` setting (Red's suppressible console notice becomes a plain per-run warning, dropping `state/ver_mismatch_warned.txt`; the GUI also surfaces this live via the version selector's red/yellow state — see the core plan's Sidebar; context: `path` the PES folder, `exe` the file looked for) | none; deployment proceeds |
| `dpfilelist_missing` | E/W | no `DpFileList.bin` in download folder: an Error in a compile that deploys, a Warning in sideload and test mode and with `--no-deploy` (a clean run, "Post-processing"), which deploy nothing, so the user knows no bin from the download folder was used | E: deployment skipped; staged CPKs promoted to `output/`; savefile step skipped. W: none; the bins are built on the bundled bases (`bin_source` `bundled`) |
| `dpfilelist_not_official` | W | the installed DpFileList's entries differ from the bundled official list's (missing entries, entries the official list lacks, or their order), while every target of this run is listed (context: `path` the installed list; `missing` the official entries it lacks, in the official order; `unofficial` its entries the official list lacks, in its order; `order` = `differs` when the entries both lists hold are in another order; each of these three only when it applies, lists joined by `, `; `command` the subcommand, `UTW team-compiler upgrade-dpfl`) | none: the run compiles and deploys; the CLI names `upgrade-dpfl`, which asks before replacing the list |
| `dpfilelist_cpk_missing` | W | the installed DpFileList names CPKs with no file in the download folder, other than this run's own target (context: `path` the download folder, `files` the file names in list order, `command` as `dpfilelist_not_official`'s). The game then rejects the whole download folder and runs vanilla | none: the run compiles and deploys; the savefile step is skipped; `upgrade-dpfl` writes a placeholder for each |
| `dpfilelist_outdated` | E | installed DpFileList lacks a target of this run that the bundled official DPFL lists (old layout, e.g. no `teams` slots; context: `cpk` every CPK of the run the installed list lacks and the official one names, joined by `, ` in the run's order (the bins CPK, then the parts by slot number), `path` the installed list, `output` the CPK's path in the output folder, or the output folder when the run writes several, `command` as `dpfilelist_not_official`'s) | deployment skipped; staged CPKs promoted to `output/`; CLI names `upgrade-dpfl`; savefile step skipped |
| `dpfilelist_cpk_renamed` | I | `upgrade-dpfl`: an old DLC's CPK renamed by stem to an official name (context: `from`, `to`, the file names) | none |
| `dpfilelist_placeholder_written` | I | `upgrade-dpfl`: the placeholder CPK written for an official entry with no file in `download/` (context: `cpk`) | none |
| `dpfilelist_replaced` | I | `upgrade-dpfl`: the installed list replaced by the official one (context: `path`; `backup` the `DpFileList.bin.bak` written, absent when there was no list; `unreadable` the error, only when the old list could not be read as a list, so nothing was renamed) | the old list kept as `DpFileList.bin.bak`, replacing an older backup |
| `dpfilelist_cpk_dropped` | W | `upgrade-dpfl`: an installed entry the official list lacks and that is not renamed (no official name of its stem left, no stem, or the name held by another file) (context: `cpk`; `size` of its file in `download/`, absent when there is none) | the game no longer loads it; the file is kept for the user to remove |
| `dpfilelist_up_to_date` | I | `upgrade-dpfl`: the installed list is the official one byte for byte and every CPK it names is in `download/` (context: `path`) | nothing is written |
| `dpfilelist_upgrade_planned` | I | `upgrade-dpfl` without `--yes`: the lines before it are what `--yes` would do (context: `command`, the subcommand with `--yes`) | nothing is written |
| `cpk_name_unlisted` | E | CPK name in neither the installed nor the bundled DpFileList (a genuinely unknown name; context: `cpk` every CPK of the run neither list names, joined as `dpfilelist_outdated`'s, `path` the installed list, `output` as `dpfilelist_outdated`'s) | deployment skipped; staged CPKs promoted to `output/`; savefile step skipped |
| `cpk_slots_exhausted` | F | multi-CPK content does not fit the available slots at `cpk_part_max_size` (context: `export` the first export no slot is left for, `stem` the slots' stem, `slots` how many the official list has, `cap` the cap as a size) | run aborted; the DPFL author adds slots or the cap is raised |
| `cpk_team_exceeds_cap` | F | a single team's compiled content is larger than `cpk_part_max_size` (impossible in practice at 3 GiB; context: `export`, `size`, `cap`) | run aborted; teams are never split across parts |
| `cpk_size_over_limit` | W | a single-CPK output exceeds `cpk_part_max_size` (valid for PES; the DLC repository will reject it; context: `cpk`, `size`, `cap`) | none |
| `old_cpk_locked` | E | old CPK cannot be replaced (PES running): the rename of the copied `.partial` over it fails, or, when the run writes several CPKs, the move of an old CPK aside to `.cpk.old` or of a `.partial` into its place (context: `path` the old CPK, `error`, `output` as `dpfilelist_outdated`'s) | deployment skipped; staged CPKs promoted to `output/`; savefile step skipped |
| `deploy_target_unwritable` | E | destination folder denies writes (typically elevation needed under `Program Files`): the probe before any export is read, or the copy to the `.partial` (context: `path`, `error`, `output`) | deployment skipped; staged CPKs promoted to `output/`; savefile step skipped |
| `deploy_skipped_by_flag` | I | `--no-deploy` given (CLI) (context: `path`, the CPK promoted; one per CPK in multi-CPK mode, the bins CPK first, then the parts by slot number) | staged CPKs promoted to `output/`; savefile step skipped; run is clean |
| `overrides_active` | I | the data directory's `overrides/` folder holds at least one file, each injected at its relative path into the CPK, or into `livecpk\` in sideload mode (`compile` in normal and sideload mode, the modes that apply overrides; Run scope; context: the folder, the file count) | none |
| `installed_bin_unreadable` | F | the working-bin walk (`pipeline.md` "Bins accumulation") meets an installed `DpFileList.bin`, a listed CPK or a bin inside it that cannot be read, a bin that does not parse as its format included (context: the file, the whole error chain) | run aborted before any export is read; the previous CPK is kept (`AbortRun`) |
| `bin_source` | I | which installed CPK supplied a working bin in the DpFileList walk, or `bundled` when the embedded base was used (context: the bin, the CPK or `bundled`) | none |
| `player_table_missing` | W | the run has `PlayerAppearance.bin`, `BootsList.bin` or `GloveList.bin` rows to write (Fox) and no CPK of the working-bin walk holds the table, which has no bundled base (context: the table, the number of rows) | the table is not written and the rows are left out: a table of the run's rows alone would hide every other player's (`pipeline.md` "Bins accumulation") |
| `bin_header_repaired` | W | a working `TeamColor.bin` or `UniColor.bin` held records whose header was not their position's: the team ID, or in `TeamColor.bin` the color count. One finding per bin (context: the bin, the teams). It marks a corrupt installed bin, whatever wrote it, so the cause can be looked for | the headers are rewritten; each record's other bytes are kept, so those teams' colors may be wrong until their exports are compiled again |

A missing savefile
is the explicitly permitted warning-only case. A present savefile that cannot be read or whose
version differs from the compile target is an error, not `savefile_missing`; do not modify it or
publish a partially deployed run.

Deployment-stage errors above mean no partial installation: preflight failures leave installed
outputs untouched; failures after replacement starts roll back the deployment as a whole.

The `cpk_write_failed` and `output_commit_failed` consequences are required guarantees: partial
generated output is discarded and prior published output remains untouched. The corresponding open
questions choose the `.partial`/rename/rollback implementation, not whether these guarantees apply.

Fatal settings/environment problems (invalid PES version, unwritable output folder, a missing
exports folder) are surfaced by the settings UI and CLI argument validation before a run starts,
not as pipeline messages. Their text is a sentence written for the user, naming the path and what
to do, never a bare OS error (`The system cannot find the path specified. (os error 3)` is what a
missing exports folder printed in Phase 3's shell slice). A missing exports folder reads
`the exports folder <path> does not exist. Create it and put your exports inside, or set
exports_folder_path in <settings file> to the folder that holds them`; when the path came from the
command line, `the exports folder <path> given on the command line does not exist`. It is a
configuration error (exit code 2). The relative default is created instead (`settings.md` "Path
resolution"); when it cannot be, the run is refused naming the path, as an unwritable output folder
is (exit code 3).

---
