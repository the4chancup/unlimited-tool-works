# Unlimited Tool Works — Unified model format plan

## A model folder

**The file name carries the part type.** A model's stem is `<anything>_<suffix>`, and the suffix —
always last — says what the model is: a Fox allowed name (`face_high`, `boots`, `glove_l`, …) or a
pre-Fox `face.xml` type (`gloveL`, `handL`, `uniform`, `eye`, …, or `model_type_<x>` for one the
table doesn't know). The compiler maps it to a Fox destination and a pre-Fox type through one table
("Model names: a free part plus a suffix" in the [Aesthetics export plan](aesthetics_export/README.md)); the type
is used only when targeting pre-Fox. Nothing about the part type lives inside the glTF or the
material toml, so the same `.glb` compiles typed on pre-Fox and merged into its Fox slot.

### Dependencies and images

- **Accepted image formats.** Any of these may be used as a texture source for any model format
  (FMDL, .model, glTF) — they are fully interchangeable:

  | Format | Decoder | Extensions |
  |--------|---------|------------|
  | DDS (BC7/BC5/BC3/BC1/…) | `dds_convert` (`block_compression`) | `.dds` |
  | FTEX (Fox) | `ftex` crate | `.ftex` |
  | PNG | `image` crate (pure Rust) | `.png` |
  | JPEG | `image` crate (pure Rust) | `.jpg`, `.jpeg` |
  | BMP | `image` crate (pure Rust) | `.bmp` |
  | WebP | `image` crate (pure Rust) | `.webp` |
  | TGA | `image` crate (pure Rust) | `.tga` |
  | TIFF | `image` crate (pure Rust) | `.tif`, `.tiff` |

  The `image` crate supports additional pure-Rust formats (QOI, PNM, Farbfeld, HDR, OpenEXR, GIF,
  ICO) that are not accepted because they serve no game-texture use case (animation, HDR, icons,
  obscure academic formats). AVIF remains outside the initial decoder set; its backend requirements
  depend on the selected library/version. QOI can be added if community demand arises. See the
  [library crates plan](libs/README.md) for the full format matrix and the PES texture boundary (which
  codecs each PES version gets).
- **Stem-based texture references.** Material references name textures by **stem** — filename
  without extension — as do Studio-format `.mtl` files and FMDL path tables (whose stored names
  can carry a decorative extension, `.tga` in Konami's files; it is ignored, the game appending
  `.ftex` either way). The compiler scans the model folder for image files, builds a stem→file map, and resolves
  each referenced stem to the actual file. Two image files with the same stem but different
  extensions (e.g. `face.dds` and `face.png`) is a conflict (`texture_stem_conflict`) — both would
  claim the same reference, and the compiler can't pick one. The rule applies within a single model
  folder; different folders may reuse the same stem independently. A reference may carry a path
  below the referencing file's folder, so textures can sit in subfolders beside the model: a
  `.mtl` path `./kitN/armor_bsm.dds`, or an FMDL texture whose directory starts with `./`
  (`./kitN/` and `armor_bsm`). It resolves at that path alone, to a texture or a link sitting
  there (a `kitN` directory or stem standing for its set, "Kit-dependent assets"), and one
  nothing sits at is not found; a bare stem (`./skin.dds`, an FMDL directory
  `./` or empty) resolves nearest first (`aesthetics_export/player_folders.md` "Subfolders").
  Any other directory keeps its meaning and the file name is looked up, as before: a game
  path (`/Assets/...` in an FMDL, `model/character/...` in a `.mtl`), which a converted
  `.model` carries into its FMDL as written, without a leading `/`, so the `./` is what
  marks a path below the folder, not a missing `/`.
- **PNG is the expected norm.** Blender cannot write DDS natively, so glTF-authored models naturally
  carry PNG textures; conversion to the target texture format happens at compile time (see the
  [library crates plan](libs/README.md)).

---

## Kit-dependent assets (`kitN`)

Some parts of a player model must change with the kit the user picks in the pre-match menu — a
shirt's cloth texture, undershorts, tape matching the socks. The community's modded exes (Fox and
pre-Fox alike) implement this with a **path magic**: a texture path — or, in a pre-Fox `face.xml`, a
model path — containing a reference token is rewritten at load time to the variant spelling for the
selected kit slot, and the game loads whichever variant file that names. The token may sit anywhere
in the path, including a folder name. The unified format keeps this mechanism and gives it a
readable spelling, looked for **in place of** the historical one: on Fox by FoxDen's kit
swapper (`scripts/02_kitswappers.lua`, which keeps a path whose variant for the kit picked does
not exist, so the path's own file is the fallback), on pre-Fox by an exe change made when the
cup next plays a pre-Fox version:

| Spelling (export and game files alike) | Meaning |
|---|---|
| `kitN` (literal `N`) | the *reference*: "the variant for the kit number selected in PES" |
| `kit1` … `kit9` | the variant used when kit 1 … 9 is selected |
| `kitF` | the *fallback*: what a kit number without a variant of its own gets; never written under its own name, so a path naming it finds nothing |

The reference is spelled `kitN`, not `kit0`, because `kit0` reads as "the texture with a 0 on it"
while `kitN` reads as the placeholder it is. The compiler writes the `kitN` and `kit1`…`kit9` spellings **verbatim**; there
is no output-side rewrite. The fallback is a file of its own, not kit 1's: filling every gap
from kit 1 puts the first kit's look on every other number (tape matching the home socks on
the away kit), and the author's only way out would be a hand-made copy per gap. A team's kit
folders keep their own fallback under its own name, `Kits/all/` (`player_folders.md`, the
paragraph starting "**Kits** live"), not `kitF`: it is a kit slot, a different mechanism from
a variant set, and one name for both would suggest a tie between them that does not exist.

**The historical `u0XXXp0` magic is legacy.** It is what the exes matched so far — `u0XXXp0`
rewritten to `u0XXXp1`, `u0XXXp2`, … — and the Studio compiler does **not** know it: the token is
just another stem fragment, so a leftover reference fails the ordinary texture-existence checks like
any other missing texture. There is no dedicated finding for it — a specific error would be partial
support, complexity for no gain, since every existing export goes through the Export upgrader before
Studio can use it. The planned Export upgrader is the only Studio component that reads the legacy
spelling, and it rewrites it to `kitN`/`kit1`…`kit9`, a `u0XXXp0` file or folder (the file
a kit without a variant loaded) to `kitF`. The legacy pattern is `u0` + **any three characters** + `p` +
digit: authors wrote the `XXX` placeholder literally *or* baked the real team ID in (`u0872p1.dds`),
and both forms mean the same thing, so both are detected and both are renamed.

There is one number. PES lets the user pick a single kit number per team, which selects the
outfield and goalkeeper kit textures together; with `kitN` the export supplies its *own* files and
only that number matters, so a goalkeeper's model uses `kitN` like anyone else's and its
`kit1`…`kit9` files are simply whatever that model should wear when that kit is picked. (The `p` in
the legacy magic was the exes' spelling, not an "outfielders only" restriction.)

Rules:

- The token is a stem fragment delimited by `_`, `-`, `.`, or the ends of the name — `pants_kitN`,
  `kitN_pants`, `kitN` — anywhere in a texture stem or a model file name. The conventional place is
  the suffix. `skitN` is not a token. `kitF` is a token as `kitN` is, spelled exactly so.
  A whole folder name may be the token instead, in any
  folder: `kit1/armor_bsm.dds`, `kit2/armor_bsm.dds` is the set `kitN/armor_bsm`, which a model
  names as `./kitN/armor_bsm`, one folder per kit holding a whole set of a kit's textures under
  their plain names; in `Common/`, a player's `kitN/` folder of links reaches it ("Link
  files").
- **A `kitN` reference never exists as a file** in the export (on PES 18-21 the output
  holds the set's fallback under it, below). Wherever a path is looked up (a `./` texture
  path, a `.common` link's target, a Common path), a `kitN` stem or directory names its set and
  is found when a file of the set is.
- **Every kit number the export defines needs a variant.** The team's `Kits/` folder lists its
  player kit slots (`p1`…`pN`). A set's *fallback* serves each of those numbers it has no
  variant for: its `kitF` file when it has one, the author's choice, with no finding;
  otherwise its **lowest existing variant**, reported as `kit_variant_missing` (W). On PES
  15-17 the compiler copies the fallback into each gap; on PES 18-21 it writes the fallback
  once under the reference's own spelling (`pants_kitN.ftex`, `kitN/armor_bsm.ftex`),
  whatever numbers the export defines, which FoxDen loads for any kit without a variant of
  its own. Pre-Fox keeps the copies because the
  exe change that will read `kitN` there is not written yet. Either way the model never
  shows a missing texture in game. A set of `kitF` alone is one fallback for every number, a
  set like any other. Variants for numbers the export does not define are emitted as they
  are (harmless).
- **Per-kit models.** Model files `pants_kit1.model` + `pants_kit2.model` (any source
  format) form one variant set: pre-Fox emits a single `face.xml` entry naming `pants_kitN` and the
  variant files beside it; each variant's materials resolve by the normal stem name matching
  (`pants.mtl`, or `pants.materials.toml` for glTF, matches both). The set is completed against the export's kit
  numbers as a texture set is on PES 15-17 (the fallback's model and material file copied
  under the names the listed entry takes for the missing number), since the game skips an entry whose respelled model is
  missing. The entry names the lowest variant's material file (`kitF`'s in a set of `kitF`
  alone), its
  kit token respelled `kitN` when it carries one (`pants_kit1.mtl` is written `pants_kitN.mtl`),
  since the game respells the whole entry for the kit picked; the other variants' material
  files must then go by the respelled names, and `kit_variant_mtl_differs` (W) names one that
  does not. On pre-Fox a set a `face.xml` lists is one directory's files (the packages no
  `face.xml` lists follow `team_compiler/pipeline.md` "Kit-dependent assets"): `jessie/pants_kit1.model` and a root
  `pants_kit2.model` are two sets, since the game respells the listed entry, its directory
  included, for the kit picked. On Fox the compiler builds the base package alone and reports
  `kit_variant_model_left_out` (W) when that leaves out a numbered variant.
- **Output naming.** The token (but `kitF`, which names no output file) passes through
  unchanged into emitted file names and into every path the compiler writes — FMDL path
  tables, `.mtl` sampler paths, `face.xml` model paths. Nothing in
  Studio, the Blender side included, ever produces `u0XXX`.

**What this replaces.** Two older ways of getting "the kit texture" onto a model existed:

- The pre-Fox `uniform` model type in `face.xml`, which makes the game ignore the material's diffuse
  path and use the active kit texture plus the kit config's pattern normal map. It is a native game
  feature and stays available on pre-Fox targets through the model-name suffix (a model whose
  name's suffix is `uniform` gets the type; the suffix table in the Aesthetics export plan's "Model
  names"); it has no Fox counterpart and is not part of the cross-engine format.
- `dummy_kit` (and `dummy_kit_back/_chest/_leg/_name/_nrm/_srm`), a reserved texture name the modded
  exes (on Fox, FoxDen's kit swapper) replace at load time with a kit texture of that role of the
  team (on PES 17 in Edit mode kit 2's, whatever kit the player wears; a file at the path is not
  read). It overlaps what `kitN` expresses with the export's own files, but is the only way for a
  model to wear the team's compiled kit texture itself, and gets **second-class support**: the compiler treats these stems as **game-provided** (like the Fox `dummy_nrm`/`dummy_srm`
  fallbacks), so a material may name them, the texture-existence check is skipped for them, and the
  path is emitted verbatim for the exe to substitute. Nothing is copied or generated for them: the
  substitution logic now lives in the exes and is stable, unlike the earlier Sider-script era that
  made Red's kit-1 fallback copies necessary. New exports should use `kitN` for their own files.

---

## Material files

### Link files (`.common`)

Like models, material files and textures can be loaded from the export's `Common` folder instead of
being copied into every player folder: an empty link file named after the Common file plus a
`.common` extension (a stray `.txt` suffix is accepted silently, as with every link file — Windows
hides known extensions by default). This is Red's existing `.mtl.common` convention (its
`find_mtl_file` resolves `.mtl.common` names) extended to the Studio format and to textures; Studio
supports all of these:

| Link file | Resolves to | Serves |
|---|---|---|
| `body.mtl.common` | `Common/body.mtl` | this folder's `body*`/`*body` `.model` files, via Red's MTL cascade |
| `hair.png.common` | `Common/hair.png` | the texture stem `hair` in this folder — for auto-detection, `""`, and explicit stems in any material file, and for native `.mtl`/FMDL references alike |
| `kitN/armor_bsm.dds.common` | `Common/kitN/armor_bsm.dds` | the texture `./kitN/armor_bsm` a model names, as `hair.png.common` serves `hair`; in a kit-token folder, so the set `Common/kit1/armor_bsm.dds`, `Common/kit2/armor_bsm.dds`, … |

There is no in-toml spelling for "this texture is in Common" (a `<common>/hair` tag was considered):
one link mechanism covers models, material files and textures; a texture link works with
auto-detection, so the common case — a shared normal map or hair texture — needs no toml editing at
all, only a file that can be copied between player folders; it works for native-format models too,
which a toml tag could not; and Explorer shows where a texture comes from. The cost is one empty
file per shared texture per player folder, which is what the other link kinds already cost.

Rules:

- **A link mirrors `Common/`'s tree.** A link stands for the `Common/` file at its own path
  below its player or shared folder (`jessie/body.fmdl.common` for `Common/jessie/body.fmdl`),
  so a model names the same path whether the file is local or linked, and Explorer shows
  where every link points: nothing is written inside a link.
- A link whose target does not exist in Common — model, material file or texture, a `kitN`
  directory or stem in it standing for its set ("Kit-dependent assets") — is
  `common_link_missing` (E, folder discarded), except, on PES 18-21, a texture link an
  installed CPK satisfies
  (`team_compiler/pipeline.md` "Resolved decisions"); the finding's context names the link kind
  and the Common path it looked for.
- A texture link counts as the texture being present in
  its folder under the linked name.
- **Where a texture resolves decides where it is packed**: resolved in the player folder, the
  player's own; resolved in `Common/`, the team's, packed once (one 2048² hair texture for twenty
  players is one file). The rule and its paths: "Texture relocation to common" in the [Team
  compiler pipeline](team_compiler/pipeline.md).
- Link files are folder-level references, not model content: the linked file is read from Common,
  never copied into the output, and the link file itself is not emitted. For pre-Fox `.mtl.common`
  the generated XML points the model at the Common MTL path (Red's behavior).

Pre-Fox `.model.common` links bring their own materials
through the `.mtl` search below, which reads a name-matched `.mtl` beside the link first
(a local override), then Common, then the link's folder, then the model folder.

**Pre-Fox: the `.mtl` a `.model` uses.** Each `.model` uses exactly one `.mtl`, the first this
search finds, and the generated `face.xml` names it. The search looks in a list of folders in
turn, and in each folder for three kinds of `.mtl` in turn: a **name-matched** one (its stem, case
folded, starts or ends the model's stem: `body.mtl` for `body_high.model`; a model stem carrying
both the `oral_` prefix and the `_win32` suffix is read without them, `fbm.mtl` for
`oral_fbm_u_win32.model`, since members name a part as the game packs it and its `.mtl` by the
part, and four VG exports' `Common/` folders hold such a pair beside another `.mtl`), then
`materials.mtl`, then **any** `.mtl`; when a folder holds several of one kind, the first in
case-folded name order wins (the order Explorer shows). A `.mtl.common` link counts as a `.mtl` of
its target's name in the folder holding the link. The folders, where the *model folder* is the
player or shared folder and the *file's folder* is the one directly holding the model or link
(the model folder or one of its subfolders, each a player folder of its own,
`aesthetics_export/player_folders.md` "Subfolders"):

- a `.model`: the file's folder, then the model folder;
- a `.model.common` link: the link's folder for a name-matched `.mtl` only (a local override of
  the shared model's materials), then the linked model's folder in `Common/` (where the model
  really is: `Common/jessie/` for `jessie/legs.model.common`, a link mirroring `Common/`'s
  tree), then the link's folder, then the model folder, each for all three kinds.

A model the search finds no `.mtl` for has every material undefined, which is
`model_material_undefined` (E). This is Red's `find_mtl_file` (`xml_editing.py`), with two
differences that change nothing for a real export: Red's second kind accepts any stem that
`materials` starts or ends with (`mat.mtl`), and it takes the folder listing's order; and when it
finds nothing it names a `materials.mtl` no folder holds, which the game would fail to load. Red
also reads `face_high_win32.model` as `face_neck`; the search keeps that stem as it is, which
the game's own `face.mtl` name-matches either way. Red searches only for a player's `face.xml`
and packs a `Common/` model as it is; the search also gives a `Common/` `.model` the set its
conversion pre-check reads.

### Comments are app-injected

All auto-generated tomls across the suite ship with predefined inline comments that serve as
simplified per-field documentation — the user never writes comments from scratch. `toml_edit`-based
edits preserve these comments and can inject comments for newly added keys. This principle applies to
`materials.toml`, `settings.toml` and `config.toml` alike (the AATF rules file is Rhai, not a
toml; the Ruleset editor writes its data block without per-field comments).

## `materials.toml` schema

### Layout and precedence

Values resolve for each target engine in this order, later winning:

1. **Family defaults** for that engine (table below).
2. **Engine-neutral keys** (`two_sided`, `transparent`, `antiblur`, `textures`, `parameters`),
   translated to the engine's representation by the fixed tables below.
3. **The engine table** (`[mat.fox]` / `[mat.prefox]`): native fields, applied verbatim. A raw
   `alpha_flags` here still yields to an explicit `two_sided`/`transparent` on the bits those
   booleans own; everything else in the engine table is final.

The engine tables exist for two reasons: lossless round-trips of native models (a converted FMDL
keeps its exact shader, flags, and parameters) and the rare custom setup the families don't cover.
Ordinary authoring never needs them.

### Engine-neutral keys

| Key | Type | Default | Meaning |
|---|---|---|---|
| `family` | string | `"shaded"` | Shader family; selects both engines' shader and defaults. One of `shaded`, `shadeless`, `metal`, `glass`. |
| `two_sided` | bool | `false` | Render both faces. Fox: alpha flag bit 32. Pre-Fox: state `twosided`. |
| `transparent` | bool | `false` | Alpha-blended (semi-transparent) rendering. Fox: alpha flag bit 128 on top of the family's default bits. Pre-Fox: the transparent state set (`zwrite 0`, `alphatest 0`, `alphablend 1`). Omitted = off (no texture inspection — deciding transparency from texture alpha, as the 19to16 converter does, is expensive and surprising). |
| `antiblur` | bool | family default | Fox: generate anti-blur duplicate meshes for this material's meshes (`Has-Antiblur-Meshes`). Pre-Fox: no equivalent (`.model` has no anti-blur). Listed as engine-neutral for discoverability — it describes the material, not a format quirk — even though only Fox implements it. |
| `textures` | table | see "Textures" | Canonical role → stem. |
| `parameters` | table | family default | Shader parameter name → `[x, y, z, w]` (four numbers). Passed to whichever engine defines the name (Fox material parameters, pre-Fox `<vector>` elements); names the target engine's shader doesn't know are dropped. |

### Shader families

The family is the format's main friendliness device: it captures what the converters' mapping tables
know about which Fox and pre-Fox shaders correspond, so the author picks one word and both engines
get a working material. `shaded` and `shadeless` cover the vast majority of models and are named as
the community knows them (not after either engine's shader — `blin`, `constant`, `Basic_C`, and
`Shadeless` are the native spellings the families map to). Native imports infer the family from the
shader (right-hand column). There is deliberately no family for the rare shaders (the pre-Fox
`Overlay` decal shader, Fox `3ddc`/`eyeocclusion`/`translucent`): in a decade of exports they have not
been used by members, so they stay reachable through the engine tables only (see the advanced
example), and a native import of one keeps its shader verbatim there with the family that best
approximates it for the other engine. That inference is always marked approximate
(`material_family_approximated`), even where a family substring also matches, as `3ddf` does in
`pes_3ddf_basic_color_translucent` and `lambert` does in `eyeocclusion_lambert`: shader names
containing `translucent`, `3ddc` or `eyeocclusion` are checked first.

| Family | Use | Fox shader / technique | Fox defaults | Pre-Fox shader | Pre-Fox defaults | Inferred from |
|---|---|---|---|---|---|---|
| `shaded` | Skin, hair, cloth, boots — the standard lit material (Fox "blin") | `fox3ddf_blin` / `fox3DDF_Blin` | alpha `128`, shadow `0`, `antiblur = false`; `MatParamIndex_0 = [0,0,0,0]`; `normal`/`specular` fall back to the game's `dummy_nrm`/`dummy_srm` | `Basic_C` + the letters of the roles that resolve (see the `Basic_*` ladder below) | opaque state set | Fox shaders containing `blin`/`3ddf`/`hair`; pre-Fox `Basic_*`, `Boots`, `Shirt_*`, `Pants_*` |
| `shadeless` | Unlit (eyes, decals baked into the texture, emissive parts) (Fox "constant") | `fox3dfw_constant_srgb_ndr_solid` / `fox3DFW_ConstantSRGB_NDR_Solid` | alpha `16`, shadow `5`, `antiblur = true`; no parameters | `Shadeless` — the community-made shader combining the advantages of the stock `Constant` and `Overlay`, which is why both of those infer this family | opaque state set | Fox shaders containing `constant`/`lambert` (or any other `3dfw` shader, with a warning); pre-Fox `Shadeless`, `Constant`, `Overlay` |
| `metal` | Metallic surfaces | `fox3ddf_ggx` / `fox3DDF_GGX` | as `shaded`; `metalness` role available | `Basic_CNSR` | opaque state set; `Reflection = [1,1,1,0]`, `Shininess = [0.9,0,0,1]`; `environment` falls back to the compiler's **template environment map** (a real cubemap shipped with the templates, taken from a working export's `env.dds`), so `Basic_CNSR` always has its `R` — a metal without a reflection isn't metal | Fox shaders containing `ggx` |
| `glass` | Visors, lenses | `pes3dfw_glass2` / `pes3DFW_Glass2` | alpha `16`, shadow `5`, `antiblur = false`; `MatParamIndex_0 = [54,0,0,0]`, `ReflectionIntensity = [1,0,0,0]`, `GlassRoughness = [0,0,0,0]`, `GlassFlatness = [0,0,0,0]`, `PCBoxCenter = [0,15,0,0]`, `PCBoxSize = [250,80,250,0]` | `Basic_C` | transparent state set | Fox shaders containing `glass` |

**The pre-Fox `Basic_*` ladder.** The `Basic_` shader's suffix letters spell the maps it samples:
`C`olor, `N`ormal, `S`pecular, `R`eflection (an `EnvironmentMap` cubemap plus the `Reflection` and
`Shininess` vectors — see the AUO lab export's `fbm*.mtl`, where every `Basic_CNSR` material with
an `env.dds` also carries those two vectors). For `shaded`, the compiler picks the shortest rung
covering the roles that resolve for the material: `Basic_C` → `Basic_CN` (normal) → `Basic_CNS`
(normal + specular) → `Basic_CNSR` (+ `environment`). Higher rungs tolerate missing lower maps (the
same export uses `Basic_CNSR` with only diffuse + specular), so an explicit `[prefox] shader` is
never wrong, only the automatic pick is minimal. Rungs other than these four (`Basic_CS`, …) are not
assumed to exist; verify against the game's shader list before adding any.

State sets (pre-Fox `[prefox.states]` defaults):

| Set | `ztest` | `zwrite` | `twosided` | `alphatest` | `alpharef` | `alphablend` | `blendmode` |
|---|---|---|---|---|---|---|---|
| opaque | 1 | 1 | 0 | 1 | 0 | 0 | 0 |
| transparent | 1 | 0 | 0 | 0 | 0 | 1 | 0 |

Fox `fuzzblock` shaders (`fox3ddf_blin_fuzzblock`, `…_uvscroll`) are anti-blur helper materials the
exporter generates; they never appear in a toml — importing an FMDL turns them back into
`antiblur = true` on the source material.

**Checked against the community's reference.** The cup wiki's Materials page
(`resources/Materials.wikitext`) differs from these tables in places; each difference was weighed
against every FMDL and `.mtl` on the maintainer's machine (2026-10-11: 7,315 FMDLs, 10,101 `.mtl`
files), not against either document. The tables stand where real files use our form: the
`shadeless` shader `fox3dfw_constant_srgb_ndr_solid` (6,870 materials, beside 813 of the wiki's
`fox3dfw_constant_srgb_ndr`; Konami's own files use neither); `.mtl` samplers with `srgb` and the
two filters only (14,543 samplers) and either state order (16,849 and 13,866 materials); the
opaque set's `alphatest` 1 (5,142 opaque materials, as the 19to16 converter writes it; Konami's
`Boots` files write 0); `Basic_CNSR` read as `shaded` (about two thirds of its 3,298 materials have no
`EnvironmentMap`); the alpha flag 128 on a forward shader (902 materials) and the add-on's
`invisible` bit over the family's shadow bits; and an anti-blur duplicate at the source mesh's
positions (9,973 of 9,994 paired duplicates, one smaller, where the wiki says "slightly smaller"). Shaders the wiki
documents and no family covers (Fresnel `fox3ddf_blin_incidence`, `fox3dfw_constant_srgb_plain`,
the UV scroll and step shaders, the `RoughnessMap` sampler) stay native-only, kept verbatim by an
import.

### Textures

`[mat.textures]` maps **canonical roles** to texture stems. The role vocabulary is engine-neutral;
a fixed table maps each role to the engine's sampler name, and roles an engine has no sampler for
are dropped for that engine (`material_texture_unused`, I):

| Role | Fox sampler | Pre-Fox sampler | Family default when omitted and not found |
|---|---|---|---|
| `base` | `Base_Tex_SRGB` (`Base_Tex_LIN` when `[fox] base_linear = true`) | `DiffuseMap` (`srgb="1"`) | required: `material_texture_not_found` |
| `normal` | `NormalMap_Tex_NRM` | `NormalMap` (`srgb="0"`) | Fox, `shaded` and `metal`: game `dummy_nrm`; pre-Fox: no sampler |
| `specular` | `SpecularMap_Tex_LIN` | `SpecularMap` (`srgb="1"`) | Fox, `shaded` and `metal`: game `dummy_srm`; pre-Fox: no sampler |
| `metalness` | `MetalnessMap_Tex_LIN` | — | none |
| `environment` | — | `EnvironmentMap` (`srgb="0"`, `minfilter="anisotropic"`, wrap addressing, `maxaniso="2"`) — the `R` in `Basic_CNSR` | `metal` family: the template environment map, emitted into the player's common textures like any other fallback texture; else none |
| `reflection` | `GlassReflection_Tex_SRGB` | — | none |
| `reflection_mask` | `GlassReflectionMask_Tex_LIN` | — | none |
| `detail_material` | — | `DetailMaterialMap` | none |
| `detail_normal` | — | `DetailNormalMap` | none |

The one set of **reserved stems** a toml may name
without a file behind them is the legacy `dummy_kit*` family, substituted by the modded exes (see
"Kit-dependent assets"); the existence check skips them.

### `[mat.fox]` — Fox (FMDL) table

| Key | Type | Meaning |
|---|---|---|
| `shader` | string | FMDL shader name (`fox3ddf_blin`, …). Default: family's. Setting it without `technique` keeps the family's technique. |
| `technique` | string | FMDL technique name (`fox3DDF_Blin`, …). Default: family's. |
| `alpha_flags` | integer 0–255 | Raw per-mesh alpha flags. Default: family's. Known bits: 32 two-sided, 128 transparent — owned by `two_sided`/`transparent` when those are set explicitly; the other bits are applied verbatim. |
| `shadow_flags` | integer 0–255 | Raw per-mesh shadow flags. Default: family's. Known bits: 1 no shadow cast, 2 invisible — owned by `cast_shadow`/`invisible` when those are set explicitly; the other bits are applied verbatim. |
| `cast_shadow` | bool | Shadow flag bit 1 *clear*. Default `true`. Fox-only and practically never changed, hence here rather than engine-neutral. |
| `invisible` | bool | Shadow flag bit 2: the mesh renders nothing but still casts a shadow if enabled. Default `false`. Same remark. |
| `base_linear` | bool | Bind the `base` role as `Base_Tex_LIN` instead of `Base_Tex_SRGB`. Default `false`. |
| `textures` | table | Native sampler name → stem, for samplers outside the canonical role table (custom shaders). Same value grammar as canonical roles except no auto-name suffix (absent = no sampler). A native name that *is* a canonical role's Fox sampler is `material_value_invalid` — use the role. |
| `parameters` | table | Fox-only parameters, name → `[x, y, z, w]`; merged over the engine-neutral `parameters`. |

FMDL stores alpha/shadow flags **per mesh**, not per material. Export writes the material's
resolved flags to every mesh using it. Import (`fmdl_to_ir`, and the Blender codec) splits a
material instance whose meshes carry differing flags into one material per distinct
`(alpha_flags, shadow_flags, antiblur)` combination, suffixed `_2`, `_3`, … — the same rule the
`pes-fmdl` addon applies when creating Blender materials.

### `[mat.prefox]` — pre-Fox (.model/.mtl) table

| Key | Type | Meaning |
|---|---|---|
| `shader` | string | `.mtl` shader name (`Basic_C`, `Basic_CN`, `Basic_CNSR`, `Shadeless`, `Constant`, `Overlay`, `Boots`, `Shirt_NB`, `Pants_NB`, …). Default: family's (texture-dependent for `shaded`). |
| `states` | table | `<state>` name → integer: `ztest`, `zwrite`, `twosided`, `alphatest`, `alpharef`, `alphablend`, `blendmode`. Default: family's state set, with `twosided`/`transparent` applied. Validated exactly like a `.mtl`: `mtl_state_invalid` (`ztest` ≠ 1, `blendmode`/`alphablend` ∉ {0,1}), `mtl_blendmode_nonzero`, `mtl_state_nonrecommended`. Unknown state names → `material_key_unknown`. |
| `samplers` | table of tables | Per-sampler `<sampler>` attributes, keyed by **native sampler name**: `srgb` (0/1), `minfilter`/`maxfilter`/`magfilter`/`mipfilter` (`linear`, `point`, `anisotropic`), `uaddr`/`vaddr`/`waddr` (`wrap`, `clamp`, `repeat`), `maxaniso` (integer). Defaults: `srgb` per the role table, `minfilter = magfilter = "linear"`, nothing else emitted. A `path` key here is `material_value_invalid` — paths come from `textures`. Extra native samplers (no canonical role) are declared here with a `stem` key. |
| `parameters` | table | Pre-Fox-only `<vector>` parameters, name → `[x, y, z, w]`; merged over the engine-neutral `parameters`. |

Output `.mtl` files written at compile time keep the `.dds` extension on texture paths, as the game
expects, and order states `ztest, zwrite, twosided, alphatest, alpharef, alphablend, blendmode`
(the 19to16 converter's order) for byte-reproducible output.

### Validation summary

| ID | Sev | Condition |
|---|---|---|
| `material_texture_unused` | I | a role the target engine has no sampler for |
| `common_link_missing` | E | a `.common` link (model, material file or texture) names a file missing from Common |
| `kit_variant_missing` | W | a `kitN` reference lacks the variant for a kit number the export defines and its set has no `kitF` (the lowest variant serves it) |
| `kit_variant_model_left_out` | W | per-kit model variants on a Fox target (the set's fallback used: `kitF`, else the lowest variant) |

Consequences follow the Team compiler's disposition rules (E → folder discarded). The full
catalog is in the [Team compiler plan](team_compiler/README.md).

---
