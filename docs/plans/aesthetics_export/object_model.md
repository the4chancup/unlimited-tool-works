# Unlimited Tool Works: Aesthetics export plan: Object model

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

## Object Model

### Validation semantics

What exists of this is in `spec/aesthetics_export/object_model.md` "Validation semantics".

A team export's `write_names` marker is also admitted at the root: the aesthetics patch writes
its players' names ("Player settings in exports" in
[`settings_toml.md`](settings_toml.md)).

A team export's `team.toml` is admitted at the root too (Release 0.1.0): the team's Team TOML,
the file the Save editor's export writes, from which the compile takes the tactics alone for
its tactical patch (`team_compiler/pipeline.md` "6. Post-processing"). Its other tables (the
team, its players) reach no output: the roster is the Save editor's to import. A `team.toml`
that is not a Team TOML is an Error naming the file, and the export compiles without it.

The cup's rules require every team's logo, so a `Full` team export with no logo is an Error,
`logo_missing`: one of `validate`'s export findings, beside `export_empty`. Only a `Full`
export is reported:
- a `Midcup` export replaces only what it holds, so the installed logo stays;
- a referee export's logo reaches no output.

"No logo" means no root file whose stem starts with `logo`. A logo file that is there but
refused reports its own Error (`logo_file_invalid`, `logo_role_duplicate`,
`logo_small_without_main`), and a second finding beside it would only repeat it. The finding is
on the export and drops nothing (`Keep`): the rest of the export compiles, without a logo.

It is a validation finding, not a Compiler-only one, so the Export upgrader, which runs the
same validation on what it writes, refuses a `Full` output that would have no logo (its
"Verification": a run with an Error writes nothing). A Compiler-only finding would let the
upgrader call clean an export whose first compile then reports an Error.

On PES 2017 to 2021, which show players' portraits, a player with no portrait is a Warning,
`portrait_missing`: a slot a player folder maps to that has neither a `portrait` texture in
the folder nor a `Portraits/NN` file. It is reported on the player folder, with the slot in
its context, and drops nothing (`Keep`). PES 2015 and 2016 show no portraits, so it is not
reported there. As with `logo_missing`, only a `Full` team export is reported: a `Midcup`
player keeps his installed portrait, and a referee export's portraits reach no output.

A file that maps to the slot counts as his portrait whatever a later check finds in it, since
that check's own finding names the file. A `Portraits/` file whose name maps to no slot
(`portrait_name_invalid`) is no one's portrait, so its player is reported too, which tells the
member whose portrait the misnamed file was meant to be.

### `aesthetics_export` crate layout

What exists of this is in `spec/aesthetics_export/object_model.md` "`aesthetics_export` crate layout".

The crate gains one file beyond the tree in the spec:

```
crates/libs/aesthetics_export/src/
└── players_txt.rs      # players.txt writer (Refs arranger) with atomic replacement
```

Placement rules:

- **No I/O.** `players_txt.rs` returns bytes and an atomic-replace *plan*; the caller writes.
- **Conventions are data.** Two rules that read like compiler behavior are conventions and live
  here, because a second consumer already depends on them:
  - the **model source selection rule** (target-native first, then glTF, then the convertible
    opposite native format; the Player aesthetics editor uses it to decide which set opens
    visible and why leftover natives must be removed after a glTF conversion);
  - the **structural file-kind classification** of a model folder's files (model, texture,
    `GltfBuffer`, `PotentialGltfImage`, link file, marker, disallowed, from names alone;
    `model_convert`'s glTF dependency contract takes this classification as its input and only
    *promotes* candidates during deep URI resolution).

### Structure pass types

What exists of this is in `spec/aesthetics_export/object_model.md` "Structure pass types".

`ExportSlot` (a team or referee slot, `split_player`'s input) and `BootsId`/`GlovesId` belong to
the Compiler's planning, not to this crate. `players_txt.rs` gets its shape with its
consumer, the Refs arranger.

### Model files

Files within model folders are typed. Files that the processing code needs to look inside get rich
structs; files that are just passed through stay opaque.

```rust
pub enum ModelFile {
    // Ancillary files only: every FMDL/.model/glTF source and sibling MTL is owned
    // exactly once by a ModelPart below.
    Texture(TextureFile),    // Has format, mipmaps, conversion methods
    Xml(XmlFile),            // Parsed DOM, has editing methods
    Skl(SklFile),            // Paired with a same-basename .fmdl; content-hashed for merge checks
    Fclo(FcloFile),          // Opaque or parsed depending on needs
    RawBin(RawFile),         // Opaque game/template bin
    Other(RawFile),          // Opaque
}

pub struct RawFile {
    pub relative_path: String, // canonical within the folder, by `vtree`'s path rules
    data: Arc<[u8]>,
}
```

### Model folders

```rust
pub struct FaceModelFolder {
    pub model_id: String,
    pub folder_name: String,
    pub files: Vec<ModelFile>, // ancillary textures/XML/SKL/FCLO/raw bins only
    pub parts: Vec<ModelPart>, // owns every selected model source exactly once
}

pub struct ModelPart {
    pub logical_output: String,             // resolved allowed output name / XML entry
    pub source: ModelSource,
    pub material_source: Option<MaterialSource>, // optional folder/glTF material overrides
    pub provenance: ModelProvenance,        // local, shared-folder, or Common link
}

pub enum ModelSource {
    Fox(FmdlFile),
    PreFox { model: PreFoxModel, mtl: MtlFile }, // one atomic native bundle
    Gltf(GltfModel),
    Intermediate(CanonicalModel),  // after conversion, before re-serialization
}

impl FaceModelFolder {
    pub fn fmdls(&self) -> impl Iterator<Item = &FmdlFile>;
    pub fn textures(&self) -> impl Iterator<Item = &TextureFile>;
    pub fn validate(&self, context: &ValidationContext) -> Vec<ValidationIssue>;
    pub fn convert_textures(&mut self, settings: &Settings) -> Result<()>;
    pub fn convert_to(&mut self, target: ModelFormat) -> Result<()>;
    pub fn pack(&self, settings: &Settings) -> Vec<PackedEntry>;
}
```

`convert_to` converts every selected part to the target-native format before parts are grouped by
`logical_output` for Fox merging or pre-Fox XML generation. A pre-Fox part always owns its `.model`
and sibling `.mtl` as one native bundle. Conversion consumes or produces both atomically and commits
the replacement only after both succeed.

Material-definition files pair with their models here for both formats by the same stem
name-matching, each with its own resolution rule:

- pre-Fox `.mtl` via Red's cascade (exactly one per model);
- glTF `*.materials.toml` via the layered least→most-specific merge (see the [Unified model
  format plan](../model_format.md)'s "Material files").

For both, a `.common` link file (`body.mtl.common`, `body.materials.toml.common`) stands in for
the Common file of that name, and a `.common`-linked *model* resolves its material files in
Common first with the player folder's matches layered on top (format plan, "Link files"). This
represents mixed local/shared/Common assemblies without forcing a model folder into one singular
source format.

`BootsModelFolder` and `GlovesModelFolder` use the same `parts`/ancillary-`files` structure and
processing contract as `FaceModelFolder`.

### Player folders

What exists of this is in `spec/aesthetics_export/object_model.md` "Player folders".

```rust
// tools/team_compiler free functions operate on a materialized folder rather
// than adding inherent compile methods to aesthetics_export::PlayerFolder.
pub fn categorize_player(folder: &LoadedPlayerFolder) -> CategorizedModels;

/// Split into standard folders using boots/gloves IDs frozen by plan_run
/// (processing never allocates IDs). `slots` comes from ValidatedAestheticsExport.roster
/// (one entry for ordinary team players, possibly several for referee folders).
pub fn split_player(
    folder: LoadedPlayerFolder,
    identity: &ExportIdentity,
    slots: &[ExportSlot],
    planned_ids: &PlannedModelIds,
) -> (Option<FaceModelFolder>, Option<BootsModelFolder>, Option<GlovesModelFolder>);
// Team and referee folders alike may contain only boots, gloves, portraits,
// or settings — any category may be absent.
```

### Design constraints

What exists of this is in `spec/aesthetics_export/object_model.md` "Design constraints".

The strong-types list there additionally names `ExportSlot` (distinguishing team vs referee
slots), `BootsId` and `GlovesId`, none built yet: the Compiler's planning keys its slots
`PlayerSlot`/`RefSlot` and its boots/gloves IDs are plain `u16` (`plan/ids.rs`).
