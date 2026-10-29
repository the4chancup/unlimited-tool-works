# Unlimited Tool Works — Unified model format plan

> **Plan:** work not built yet (the open phase's sections may be partly built). What the suite does now is described in `docs/spec/`.

The unified model format is the Studio's own authoring format for PES player models: a **glTF model (`.glb`)
with two small PES extensions** plus a **`materials.toml`** sibling holding the PES material
properties. One model folder authored in this format compiles to either PES engine — Fox (`.fmdl`,
PES 18–21) or pre-Fox (`.model` + `.mtl`, PES 15–17) — through the `model_convert` lib's IR (see the
[Model conversion plan](model_conversion/README.md), which owns the IR, the importers/exporters, and the
Blender integration that writes this format). The [Team compiler plan](team_compiler/README.md) owns where
model folders sit inside an export and how models are categorized and merged.

**Design goal: user friendliness.** The format exists so that a member can author one model that
converts to `.model` and FMDL alike **without losing any customizability, in the simplest possible
way**. If it were harder to write than the native formats, most users would keep using those. Hence:
the common case needs almost nothing (a material entry can be as short as `[face]` plus a
`family`), every engine-specific knob remains reachable for the few who need it, and every
auto-generated file carries inline comments explaining its fields.

---

## Why glTF

Instead of parsing `.blend` files directly (Blender's binary format changes between versions, no
mature Rust parser) or using FBX (proprietary, immature Rust support), the format is **glTF with PES
extensions**:

- **Blender exports glTF natively** — no addon needed for base mesh/skeleton/UV data; it has been a
  stock exporter since Blender 2.80
- **Rust has a mature parser** — the `gltf` crate is production-quality
- **Other 3D tools support glTF** — Maya, 3ds Max, Godot, Sketchfab, Windows 3D Viewer, etc.
- **Extension mechanism** — PES-specific structural metadata (bones, mesh splitting) stored as
  custom JSON extensions; material properties live in TOML files alongside the model
- **Binary bulk data** — vertex/face data is binary, not text; the JSON scene description is ~5-10%
  of total size.

**Stock Blender is enough.** The plugin (`pes-models`, see the [Model conversion
plan](model_conversion/gltf.md)'s "Blender integration") adds material preview and toml editing inside
Blender, but a model exported from a plugin-less Blender with a hand-written `materials.toml` beside
it is a valid, compilable model. This is a stated guarantee, not an accident, and it constrains the
format: the PES extensions are optional with fallbacks (see "PES extension schema"), standard glTF
material data is ignored rather than rejected, and the file keeps the standard extension so every
glTF-aware tool opens it. A custom extension (`.pes`) was considered for friendliness and rejected:
it would make the plugin mandatory for any Blender work and tie the format's usability to the
plugin keeping pace with Blender releases.

**One file: `.glb`.** glTF has two containers for the same data. `.gltf` is a JSON text file that
references its binary vertex/index buffers by URI — conventionally a sibling `.bin` file, so a model
is always at least two files (the buffers *can* be inlined as base64 in the JSON, at ~33% size cost).
`.glb` packs the JSON and the binary buffer into one file. The Studio writes `.glb` (Blender's
default too) and the documentation talks about `.glb` models; `.gltf` + `.bin` is accepted as the
same format with no difference in handling. Textures are external files in both cases (see below),
so a model folder is: one `.glb` per model, the textures, and the material toml(s).

Materials are **never carried in the glTF**. The model's textures are sibling files anyway, JSON
has no comments, and tomls-as-siblings make sharing unambiguous: copy a `.glb` + its material toml +
textures into another player's folder and the materials travel with the model, touching nothing
already there — exactly like copying `.model` + `.mtl` files today.

---

## A model folder

What exists of this is in `spec/model_format.md` "A model folder".

```
15 - Snuffy/
├── face_high.glb             (the model: scene, skin, geometry; material objects carry only names)
├── face.png  face_nrm.png    (textures: any accepted image format, referenced by stem)
├── materials.toml            (PES material properties, keyed by material name; catch-all)
├── body.materials.toml       (optional name-matched material file — see "Material files")
├── hair.materials.toml.common (optional link to Common/hair.materials.toml — see "Link files")
└── hair.png.common           (optional link to Common/hair.png: a shared texture, packed once per team)
```

The compiler detects the model format by file extension (`.fmdl`, `.model`, `.glb`/`.gltf`) and
routes accordingly; glTF sources always go through the IR. A folder may hold both a native set and
glTF files ("dual-set folders"): `.mtl` serves `.model`, `.materials.toml` serves `.glb`/`.gltf` —
different extensions, different consumers, no ambiguity. Wherever this plan says "glTF" it means
either container; "the model's stem" is the file name without `.glb`/`.gltf`.

### Dependencies and images

What exists of this is in `spec/model_format.md` "Dependencies and images".

- **URI safety and locality.** External buffer/image URIs are percent-decoded and canonicalized
  into the shared `vtree::ScopePath` type and must remain inside the **source model folder**, not
  merely somewhere beneath the export root. Absolute paths, parent traversal, cross-folder
  references, network URLs, and non-`data:` URI schemes are rejected before materialization. This
  keeps folder tasks independently ownable and keeps `model_convert` reusable by the Balls compiler
  without depending on the aesthetics export format.
- **Embedded data.** Standard GLB buffer views and base64 `data:` URIs are accepted. Malformed or
  unsupported MIME types are validation errors; remote fetching is never performed.

---

## Kit-dependent assets (`kitN`)

What exists of this is in `spec/model_format.md` "Kit-dependent assets (`kitN`)".

- A material role set to `pants_kitN` means "the
  files `pants_kit1`, `pants_kit2`, … in this folder" (the folder of the material file that set the
  stem — so variants in `Common`, referenced from a `*.materials.toml.common` link or by a
  Common-linked model, are how one set of kit textures is shared by many players, which is what the
  old `Kit Textures` sharing did). Auto-detection gains a third candidate: for role `base` of
  material `pants`, the compiler tries `pants_bsm`, `pants`, then `pants_kitN` (present when
  a file of the set exists, `pants_kit1` or `pants_kitF`); other roles try `{m}{suffix}` then
  `{m}{suffix}_kitN`.
- **Per-kit models.** Fox has no model-path indirection inside a face package, so there each kit
  number gets a face package of its own, its models merged from that number's variants
  (any file of the face may vary per kit: models, textures, the face's other files), and
  FoxDen loads the package of the kit picked: its kit swapper reads a package
  `face/real/<face id>p<K>/` beside the base `face/real/<face id>/` (likewise
  `boots/k<id>p<K>/` and `glove/g<id>p<K>/`), and loads the base for a kit with no package
  of its own. So the base package holds every set's fallback, and a number gets a package
  of its own only where some set has a variant for it, holding that number's variants and
  the other sets' fallbacks. The compiler builds those packages in a step of its own.

---

## glTF conventions

### Materials: only the name is read

The glTF material object's `name` is the key into the material tomls; that is the only material
field the compiler reads. The Studio and the `pes-models` codec write name-only materials:

```json
{
  "materials": [{ "name": "face_bsm" }]
}
```

Stock Blender writes standard PBR material data too (`pbrMetallicRoughness`, a base-color texture
reference, and — for `.glb` with the exporter's default image setting — the texture bytes embedded
in the file). All of it is **ignored, not rejected**: PES data has no representation in glTF
materials, and the tomls are the only source. Embedded images are never used as texture sources —
textures are the folder's files, referenced by stem — so an embedded copy only wastes space; the
compiler reports `gltf_embedded_image_ignored` (I) once per model to nudge the user towards the
exporter's "Images: None" option.

### PES extension schema

PES-specific structural metadata is stored in a `PES_` extension namespace in the glTF JSON:
`PES_bone` (skeleton metadata) and `PES_mesh` (mesh splitting metadata). Materials carry no
extension. **Both extensions are optional** — a model without them (a stock Blender export) compiles
with these fallbacks, which are the same ones native models without the data already get:

| Extension | Carries | When absent |
|---|---|---|
| `PES_bone` | `sklParent` (the SKL parent hierarchy, distinct from the render hierarchy) and `globalPosition` per bone | Looked up by bone name in the template skeleton; a bone name unknown to the template with no `PES_bone` is `gltf_bone_unknown` (E) — a custom skeleton must carry its metadata |
| `PES_mesh` | Mesh-splitting group identity, so a same-format round-trip re-splits meshes identically | The compiler splits from scratch at compile time (it must be able to anyway — hardware limits are checked on every model); only byte-stability of round-trips is lost |

The extensions are listed in `extensionsUsed`, never in `extensionsRequired`, so every glTF-aware
tool opens a Studio-written file (unknown extensions are skipped on import; a plugin-less re-export
drops them, which the fallbacks above absorb — the `pes-models` codec preserves them).

```json
{
  "nodes": [{
    "name": "sk_head",
    "extensions": {
      "PES_bone": {
        "matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0],
        "sklParent": "sk_neck",
        "globalPosition": [0, 1.64, 0.054, 1.0]
      }
    }
  }]
}
```

### Skeleton: native glTF skin, not a companion SKL

A glTF model's skeleton is stored **natively in the glTF**, not as a companion `.skl` file:

| SKL field | glTF equivalent |
|---|---|
| Bone name | `node.name` |
| Parent index | `node.children` hierarchy |
| Model-space 3×4 bind transform | Parent-local `node.matrix`/TRS plus matching `skin.inverseBindMatrices`, with coordinate and matrix-layout conversion |
| Bone count / order | `skin.joints` array order |

glTF has first-class skeleton support via `skins` + `nodes` with `matrix`/`TRS` — no custom
extension is needed for the skeleton itself. `PES_bone` carries the PES-specific `globalPosition`
and `sklParent` metadata that don't map to standard glTF fields (the SKL parent hierarchy is
preserved separately from the render hierarchy).

At compile time the skin's bind data goes wherever the target keeps it: for Fox the compiler
reconstructs the `.skl` binary (only when the skeleton differs from the template — see "SKL
pairing" in the [Team compiler plan](team_compiler/README.md)); for pre-Fox it is written straight into the
`.model`'s inline per-bone matrix table, which is where that format has always stored its
skeleton. This keeps the glTF self-contained (one file to manage, link,
and pass through `.common` links) and matches the Blender workflow: armatures export as native glTF
skins automatically, with no separate SKL export step. The SKL binary layout and the
skeleton-reconstruction rules for FMDL sources are in the [Model conversion
plan](model_conversion/conversion.md) ("SKL binary format", "Skeleton reconstruction from FMDL").

### File size and compression

A `.glb` is comparable in size to the native formats:
- JSON scene description: 50-100KB (5-10% of total)
- binary vertex/face buffer: ~1.3MB for a 20K-vertex model
- Textures: same as current (1-10MB each)

**No format-level compression needed.** Exports may already be submitted as `.zip` or `.7z`
archives, which handle the submitted size. If size becomes a concern, enable
`KHR_mesh_quantization` in the Blender export (one checkbox — converts float32 to int16, ~50%
reduction with negligible quality loss). Do not use `KHR_draco_mesh_compression` — the decoding
complexity isn't worth it for models this size.

---

## Material files

### File pattern

Material files are identified by extension, not by name reservation:

| File | Role | Pre-Fox equivalent |
|------|------|-------------------|
| `materials.toml` | base layer — applies to every glTF in the folder, including those with name-matched files | `materials.mtl` |
| `body.materials.toml` | name-matched (example) — applies to glTFs whose stem startsWith or endsWith `body` | `body.mtl` |
| `body.materials.toml.common` | link to `Common/body.materials.toml`, participating as if it were local (see "Link files") | `body.mtl.common` |

The `.materials.toml` double extension is the structural marker: any file ending in
`.materials.toml` is a name-matched material file (stem = everything before `.materials.toml`);
`materials.toml` is the catch-all base layer; any other `.toml` file (`settings.toml`, etc.) is not
a material file. This avoids fragile name reservations and gets TOML syntax highlighting in every
text editor for free.

### Name matching

(Adapted from Red's `find_mtl_file` logic in `xml_editing.py`.) For each `.glb`/`.gltf` file in the
folder, find its material file(s):

1. **Base layer**: include `materials.toml` whenever present, whether or not any name-matched files
   exist.
2. **Name matches**: also include `*.materials.toml` files whose stem is a startsWith or endsWith
   of the glTF's stem (e.g. `body.materials.toml` matches `body_shirt.glb` and `body_main.glb`).
3. **No material file**: a glTF with neither the base file nor a name-matched toml is a hard error
   (`material_file_missing`) — glTF files at least carry material *names*, but the names alone can't
   render anything. There is no safe default: material settings include texture paths, and a default
   placeholder would produce an unusable model. Erroring at validation time is better than letting
   the user discover the problem after compiling and testing in PES.

### Layering

**Multiple matches layer, they don't compete.** A glTF may match several material files at once —
e.g. `body.materials.toml` providing generic definitions for every `body_*` model while
`body_andy.materials.toml` adds player-specific overrides for `body_andy_*` models. All matched files
apply, in a fixed order from least to most specific — deterministic, unlike Red's `os.listdir`-order
tiebreak. An informational `materials_file_ambiguous` diagnostic reports the layering when it
occurs; intentional layered setups are ordinary usage, not a problem.

**Resolution order** (deep merge per field). For material `face_bsm` in `body_shirt.glb`, the
matched files' `[face_bsm]` entries merge in this order:

1. **Base** `materials.toml` entry (if present) — always least specific.
2. **Name-matched** files' entries, ordered by ascending stem length (for `body_andy_shirt.glb`:
   `body.materials.toml`, then `body_andy.materials.toml`). For equal-length stems, a match at the
   beginning of the model filename wins over a suffix match: apply suffix-only matches first, then
   prefix matches. Thus `carlo_boots.glb` layers `materials.toml`, `boots.materials.toml`, then
   `carlo.materials.toml`, with the latter overriding conflicting fields.
3. **Same stem, linked and local**: a `.common` link and a local file with the same stem both
   apply; the linked (Common) file is the less specific layer (see "Link files").

Filesystem enumeration order never breaks ties.

Each layer is a field-level deep merge over the whole entry, sub-tables included — a more specific
entry that sets only `family` inherits the textures, flags, and engine tables from the less specific
layers; a more specific `[face.textures]` that sets only `normal` inherits `base` from below. This
lets you override one field per group without restating the whole material. A material name
referenced by the glTF with no definition in any matched toml is a hard error (`material_undefined`)
— the model can't render without texture paths, and defaults would only hide the problem.

Resolution fixtures cover base-field inheritance alongside name-matched overrides, longer-stem
precedence, the equal-length `carlo_boots` example under reversed file enumeration, and linked-vs-
local same-stem layering.

### Link files (`.common`)

What exists of this is in `spec/model_format.md` "Link files (`.common`)".

| Link file | Resolves to | Serves |
|---|---|---|
| `materials.toml.common` | `Common/materials.toml` | this folder's glTFs, as the catch-all layer |
| `body.materials.toml.common` | `Common/body.materials.toml` | this folder's `body*`/`*body` glTFs, name-matched by stem `body` |

- A link participates in name matching and layering **exactly as if the linked file were local
  with the same stem**. A folder with both `materials.toml` and `materials.toml.common` layers the
  Common file below the local one (field-level merge, local wins); `material_link_layered` (I)
  reports it. Common is the base, local is the override — the same relationship as a Common-linked
  model's own tomls versus the player folder's (below).
- **Texture stems resolve in the folder of the material file that set them.** A stem written in
  `Common/body.materials.toml` resolves against Common's images; a stem written in the player
  folder's `body.materials.toml` resolves against the player folder. Auto-detected roles (see
  "Textures") look in the model's own folder. This is what makes a Common material file reusable across
  players: it names the textures it ships with, and a player who wants a different texture overrides
  the stem in a local file, which then resolves locally.

**Common-linked models bring their own materials.** A `.common` model link (`torso.glb.common`)
loads a model whose real folder is Common, so its material files resolve there first: Common's
`materials.toml` and name-matched `*.materials.toml` (matched against the model's stem inside
Common) form the base layers, and the player folder's matched material files layer on top as
overrides. No material link is needed for this case — the link is for the reverse situation, a
*local* model using *shared* material definitions.

### Emission

`ir_to_gltf` (the [Player aesthetics editor](player_aesthetics_editor.md)'s convert-to-glTF and the
[Export upgrader](export_upgrader.md)'s optional glTF pass — the Team compiler never emits glTF)
writes **one `materials.toml` per folder** (the catch-all), with app-injected comments — most users
edit one file and have changes reflected across all models. The modular `*.materials.toml` layout and
`.common` links are an advanced opt-in that users create manually; the exporter doesn't generate
them. A native source's material is written with its `family` inferred from the native shader (see
"Shader families") **and** its native fields verbatim in the matching engine table, so the same-format
round-trip is lossless and the other engine gets the family's defaults. Existing textures are
referenced by stem, never re-encoded or embedded.

---

## `materials.toml` schema

One table per material, keyed by the glTF material name. Every key is optional; the shortest valid
entry is an empty table (`[face]`), which means "the standard face shader, textures named after the
material". Keys not in this schema are `material_key_unknown` (W) and ignored — a typo never
silently changes a material, and a toml written for a newer Studio still loads on an older one.
Wrong types or out-of-range values are `material_value_invalid` (E, folder discarded).

### Layout and precedence

What exists of this is in `spec/model_format.md` "Layout and precedence".

```toml
[face]                       # engine-neutral: what the material *is*
family = "shaded"
two_sided = false
transparent = false

[face.textures]              # engine-neutral: canonical roles → texture stems
base = "face"
normal = "face_nrm"

[face.parameters]            # engine-neutral: shader parameters, name → [x, y, z, w]
MatParamIndex_0 = [0, 0, 0, 0]

[face.fox]                   # Fox-only knobs (FMDL); override the family's Fox defaults
shader = "fox3ddf_blin"
technique = "fox3DDF_Blin"

[face.prefox]                # pre-Fox-only knobs (.model/.mtl); override the family's pre-Fox defaults
shader = "Basic_CN"
```

### Engine-neutral keys

What exists of this is in `spec/model_format.md` "Engine-neutral keys".

An unknown `family` is `material_value_invalid`, and a `parameters` name the target engine's
shader doesn't know is reported with `material_parameter_unused` (I).

### Shader families

What exists of this is in `spec/model_format.md` "Shader families".

**Before glTF sources (Phase 7) make the `metal` and `glass` defaults reachable on Fox**
(today only a Fox source reaches those families, and it keeps its own samplers), settle three
against the same files: `glass` binds `Base_Tex_LIN` in 39 of 42 real glass materials (all 28
of Konami's `fox_3dfw_glass`, 11 of 14 `pes3dfw_glass2`) and every `pes3dfw_glass2` carries a
`NormalMap_Tex_NRM` (12 of 14 `dummy_nrm`), where the family binds `Base_Tex_SRGB` and no normal
dummy; `metal` (`fox3ddf_ggx`) carries its own `SpecularMap_Tex_LIN` and `MetalnessMap_Tex_LIN`
in 176-178 of 193 real materials, and the wiki calls both required (defaults `#FF6C3A` and
`#FFFFFF`), where the family falls back to `dummy_srm` and no metalness map; and a `shaded`
material whose `environment` texture lifts it to `Basic_CNSR` gets no `Reflection`/`Shininess`
(only `metal` defaults them), against the ladder's `R`.

### Textures

What exists of this is in `spec/model_format.md` "Textures".

Each role's auto-name suffix, a column added to the spec's role table:

| Role | Auto-name suffix |
|---|---|
| `base` | `_bsm`, falling back to the bare material name |
| `normal` | `_nrm` |
| `specular` | `_srm` |
| `metalness` | `_mtl` |
| `environment` | `_env` |
| `reflection` | `_cbm` |
| `reflection_mask` | `_rfm` |
| `detail_material` | `_mtm` |
| `detail_normal` | `_dnrm` |

Each role's value is one of:

| Value | Meaning |
|---|---|
| *(key absent)* | **Auto-detect**: use the folder texture named `{material}{suffix}` if it exists; otherwise the family default for that engine (a game dummy texture or no sampler at all). A `[face]` entry with no `textures` table and `face.png` + `face_nrm.png` beside it therefore gets both maps on both engines with nothing written. |
| `""` | **Auto-name, required**: the folder texture `{material}{suffix}` must exist (`material_texture_not_found` otherwise). Use it to be told when a texture is missing instead of silently getting the dummy. |
| `"stem"` | **Explicit**: that folder texture (filename without extension). |
| `"none"` | **Disabled**: no sampler for this role even if an auto-named texture exists (game dummy on Fox where the shader needs one). |

The suffix table is fixed because Fox and pre-Fox use different sampler names for the same role, so
the suffix can't be parsed from the sampler name consistently. `base` tries two candidates in order:
`{material}_bsm` (the convention the `pes-fmdl` presets and Red's templates follow) and then the
bare material name (`face` → `face_bsm`, else `face`); both present is not a conflict, `_bsm` wins.
Auto-fill
and auto-detection happen during deep material parsing (not the structure pass), so a missing texture
reports `material_texture_not_found` at that stage, not during shallow validation. Game-provided
textures (the Fox `dummy_nrm`/`dummy_srm` in `/Assets/pes16/model/character/common/sourceimages/`)
are only ever reached through family defaults — a toml never names a game path, so exports stay
self-contained and portable across PES versions.

### Examples

Minimal — a face with `face.png` and `face_nrm.png` beside it:

```toml
[face]
```

Typical — as `ir_to_gltf` writes it for a plain FMDL face, comments included:

```toml
[face]
family = "shaded"            # shaded (lit, standard) | shadeless (unlit) | metal | glass
two_sided = false            # render both sides of each face
transparent = false          # alpha-blended (semi-transparent) rendering

[face.textures]              # role = "stem" (filename without extension). "" = must exist,
base = "face"                # omitted = use "<material><suffix>" if present, "none" = off
normal = "face_nrm"          # suffix _nrm; falls back to the game's flat normal map on Fox
specular = "face_srm"        # suffix _srm; falls back to the game's flat specular map on Fox

[face.fox]                   # written from the source FMDL so the same-format round-trip is exact
shader = "fox3ddf_blin"
technique = "fox3DDF_Blin"
alpha_flags = 128
shadow_flags = 0
```

Advanced — a two-sided hair card with tweaked pre-Fox states and a Fox-only parameter, and an eye
occlusion decal using a shader no family covers (the AUO lab export's `eyeocclusion_lambert`):

```toml
[hair]
family = "shaded"
two_sided = true
transparent = true
antiblur = true

[hair.textures]
base = "hair"
normal = "none"

[hair.prefox.states]
alpharef = 128
[hair.prefox.samplers.DiffuseMap]
uaddr = "clamp"
vaddr = "clamp"

[hair.fox.parameters]
MatParamIndex_0 = [3, 0, 0, 0]

[eye_occlusion]
family = "shadeless"         # what Fox gets; pre-Fox is overridden below
transparent = true

[eye_occlusion.prefox]
shader = "Overlay"
[eye_occlusion.prefox.states]
alphatest = 1
[eye_occlusion.prefox.parameters]
DepthBias = [0.005, 0, 0, 0]
```

### Validation summary

What exists of this is in `spec/model_format.md` "Validation summary".

| ID | Sev | Condition |
|---|---|---|
| `materials_toml_invalid` | E | a material file fails to parse as TOML |
| `material_file_missing` | E | a glTF matches no material file |
| `material_undefined` | E | a glTF material name has no entry in any matched file |
| `material_key_unknown` | W | unknown key in an entry (ignored) |
| `material_value_invalid` | E | wrong type / out of range / unknown family or role / reserved native name |
| `material_texture_not_found` | E | a mesh-used material's `base`, `""`-marked, or explicit stem resolves to no image |
| `material_parameter_unused` | I | a parameter the target engine's shader doesn't define |
| `material_entry_unused` | I | an entry matching no material in the folder's glTFs |
| `materials_file_ambiguous` | I | several name-matched files apply to one glTF (layering reported) |
| `material_link_layered` | I | a `.common` material link and a local file share a stem (Common below local) |
| `gltf_embedded_image_ignored` | I | the glTF embeds texture bytes; only the folder's image files are texture sources |
| `gltf_bone_unknown` | E | a skin joint has no `PES_bone` metadata and no name in the template skeleton |
| `mtl_state_invalid` / `mtl_blendmode_nonzero` / `mtl_state_nonrecommended` | E / W / I | `[prefox.states]` checks, shared with `.mtl` validation |

## One schema, one canonical converter

The `PES_bone`/`PES_mesh` extension schema and the `materials.toml` schema are the single contracts:
the `pes-models` Blender extension's PES glTF codec writes them (and reads them back into Blender
custom properties — see "Blender integration" in the [Model conversion plan](model_conversion/gltf.md)),
`gltf_to_ir` validates them, and any glTF + tomls the extension exports must import cleanly through
`gltf_to_ir`. Blender round-trips are for *authoring*; migrating a player folder between formats is
the Studio's IR conversion (deterministic, tested, superset-merging — see the [Player aesthetics
editor plan](player_aesthetics_editor.md)), so the two paths cannot drift apart on what a converted
folder looks like.
