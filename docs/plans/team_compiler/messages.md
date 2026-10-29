# Unlimited Tool Works — Team compiler plan: Message catalog

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Message catalog

### Message structure

What exists of this is in `spec/team_compiler/messages.md` "Message structure".

- **Logs**: `issues.log` (Warning+) and `suggestions.log` (Info) are kept as disk artifacts,
  rendered from the same events. Display layers may group repeated (code, scope) pairs; the logs
  stay complete.
- **No blocking console prompts**: unknown team ID becomes the grid's inline-editable ID cell
  (including its reassignment confirmation — see the GUI section).
- **A failed name lookup names the nearest existing name**: `link_target_missing`,
  `players_txt_target_missing`, `common_link_missing`, `kit_folder_invalid`, and
  `file_type_disallowed` on a misspelled marker (`fpc-on`, `ingameface`), carry a `closest`
  context field: the nearest name of the right kind (a shared folder for a link, a kit name for
  a kit folder, a marker for a marker), by edit distance over case-folded names, under a small
  threshold, absent when none is near. The text can then say "no shared boots folder `Crocs`;
  `crocs new` exists". Nothing is resolved by the guess: the finding and its drop stay, since a
  wrong guess compiled silently would be worse than the miss. One distance function in
  `aesthetics_export`, never one per code.
- **A finding carries the context its text needs.** The texts Phase 8 writes must say the fix,
  and some codes' context leaves the member unable to act. Each gains what its text names:
  `players_txt_invalid` the cause, of two, both roster-wide and so with no line (the file is
  not UTF-8; a referee roster has no valid line), while `notes_encoding_invalid`, whose one
  cause is the file's encoding, gains nothing (both encoding texts say how to re-save as
  UTF-8 in Notepad); `player_folder_number_invalid` gains no field, its text naming the
  rule, which never varies: the folder's name starts with a number from 1 to 23 (`05` and `5`
  alike), then optionally a `-` and a name (`05 - Bob`, `05-Bob`), or the team uses a
  `players.txt` (a form alone such as `NN - Name` would not do: `24 - Alice` has it and is
  refused); `player_number_duplicate` the other folders claiming the slot (one, unless three
  or more claim it); `kit_folder_invalid` gains no field, its text naming the allowed
  names (`p1`-`p9`, `g1`, `all`), which never vary;
  `file_type_disallowed` which case it is (an unknown extension, a hidden `.txt`, a known file
  in the wrong place); `skl_merge_conflict`, `merge_material_conflict` and
  `model_merge_flags_conflict` the two parts that disagree; `model_conversion_failed` which stage
  failed: reading or converting the source, or writing the converted model (both naming the
  cause, the error chain as today: a member's file can be at fault, such as a material path
  holding a character XML cannot carry), or the converted model breaking a check of its
  target form (the conversion's fault, not the member's file); `texture_type_mismatch` the format the
  bytes are; `export_tag_missing` the two exact names to rename to. The same holds for the
  other tools' catalogs: the Export upgrader's `custom_model_missing` and
  `settings_not_generated` gain no field, their text alone naming the ways to a clean upgrade,
  and `check-aatf`'s `aatf_violation`
  (`spec/save_editor.md`'s catalog) the player's name and shirt number beside his id in its
  `player` context field; the rule's own fields stay as the rules file spells them.
  `RosterOrder`, whose rule field is itself named `player` and holds the same id, prints it
  once, as that context field with the name and number (today the context field is skipped
  for it as the duplicate), and its `expected` gains the expected player's name and number
  the same way: it is the one violation whose fix is finding two players.

### Catalog

What exists of this is in `spec/team_compiler/messages.md` "Catalog".

**Export level**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `teams_list_read_only` | W | a teams-list write (ID cell, updater merge) failed because the data directory is not writable | write dropped; the in-memory list is unchanged (no elevation — see `pipeline.md` "Resolved decisions", "Teams list") |
| `export_disabled` | I | as the spec's row | GUI: a greyed row with **Enable** (`gui.md` "Row actions") |
| `export_tag_missing` | E | as the spec's row | GUI: the row offers the two tags as buttons |
| `team_name_unknown` | E | as the spec's row | GUI: ID cell becomes editable for inline assignment |
| `kits_folder_missing` | W | a `Full` team export with no `Kits/` folder at all, which compiles to placeholder `p1` and `g1` kits and so resets the team's kits in game (`spec/aesthetics_export/object_model.md` "Coverage tag"); not a full-body team, which writes `Kits/p1/` itself. The text names the two ways out: add the kits, or rename the export `Midcup` | kept: the placeholders are written as today (`kit_placeholder`) |

**Player folders and shared model folders**

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `players_txt_missing` | E | as the spec's row | `ParsedAestheticsExport` remains available to the Refs arranger for repair |
| `materials_toml_invalid` | E | a `materials.toml` or `*.materials.toml` file fails to parse | folder discarded |
| `material_file_missing` | E | a glTF file has no matched material file (no name-matched `*.materials.toml`, no catch-all `materials.toml`, no `.common` link to either) | folder discarded |
| `material_undefined` | E | a material name referenced by a glTF file has no definition in any matched material toml | folder discarded |
| `material_key_unknown` | W | a material toml entry has a key outside the schema (typo or newer-Studio field) | key ignored |
| `material_value_invalid` | E | a material toml value has the wrong type, is out of range, names an unknown family/role/state, or uses a native sampler name reserved by a canonical role | folder discarded |
| `material_texture_not_found` | E | a mesh-used glTF material's `base`, `""`-marked, or explicit texture stem cannot be resolved | folder discarded |
| `material_parameter_unused` | I | a shader parameter the target engine's shader does not define | parameter dropped for that engine |
| `material_entry_unused` | I | a material toml entry matches no material in the folder's glTF models | none |
| `materials_file_ambiguous` | I | multiple name-matched `*.materials.toml` files match one glTF; definitions layer least → most specific (intentional layered setups are ordinary usage) | none |
| `material_link_layered` | I | a `.common` material link and a local material file share a stem; the Common file layers below the local one | none |
| `gltf_embedded_image_ignored` | I | a glTF embeds texture bytes (stock Blender's default); textures are only ever read from the folder's files | embedded images ignored (once per model) |
| `gltf_bone_unknown` | E | a glTF skin joint has neither `PES_bone` metadata nor a name in the template skeleton | folder discarded |
| `model_name_type_misplaced` | W | a model name with no recognized suffix (routed as face content, `spec/aesthetics_export/player_folders.md` "Model names"; on Fox merged into `fcl_hair.fmdl`) whose name holds a type word of the suffix table elsewhere than at its end (`boots2.fmdl`, `boots - Copy.fmdl`, `glove_l (1).fmdl`): the game shows such a model skinned as hair. Context: the file and the intended rename (`boots2.fmdl` → `new_boots.fmdl`, a name the suffix table routes) | kept: the routing is unchanged (`fmdl_fcl_hair_fallback` still reports it) |
| `model_source_ambiguous` | E | duplicate model sources within the selected representation (target-native > glTF > convertible opposite); its input is two glTF files of one stem (`boots.glb` beside `boots.gltf`), so it is first reachable in Phase 7 | folder discarded (`DropFolder`) |

**XML/MTL content checks** (pre-Fox, plus `face_diff.xml` in Fox)

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `xml_texture_path_missing` | E | sampler with no texture path and not auto-fillable (sampler not in the suffix mapping table) | folder discarded |

The three `mtl_state_*` checks other than `mtl_state_missing` also run on a glTF material's
`[prefox.states]` table when the target is pre-Fox (the format plan's schema mirrors the `.mtl`
states one-to-one; a glTF material with states absent just gets its family's state set, so
`mtl_state_missing` does not apply).

**Output stage and savefile** (Run scope)

| ID | Sev | Condition | Consequence |
|---|---|---|---|
| `dpfilelist_not_official` | W | as the spec's row | the GUI offers **Upgrade DpFileList**, which asks before replacing the list |
| `dpfilelist_outdated` | E | as the spec's row | GUI offers Upgrade DpFileList |
| `old_cpk_locked` | E | as the spec's row | GUI offers Retry (deployment only) and Open output folder |
| `deploy_target_unwritable` | E | as the spec's row | GUI offers Relaunch as administrator. Normally pre-empted by the live writability preflight (see `pipeline.md` "6. Post-processing") |
