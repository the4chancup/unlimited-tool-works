# Unlimited Tool Works — Library crates plan: Format crates

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Recommended approach

Use `binrw` for typed, declarative binary readers and writers. Derives catch type errors, not
incorrect format layouts: wrong offsets, field widths, and endianness can compile successfully.
Verify those against real fixtures and independently decoded fields; a reader and writer can share
the same mistake and still pass a roundtrip.

```rust
#[derive(BinRead)]
#[br(little, magic = b"FMDL")]
struct FmdlHeader {
    version: u32,
    descriptors_offset: u64,
    section0_bitmap: u64,
    section1_bitmap: u64,
    section0_block_count: u32,
    section1_block_count: u32,
    section0_offset: u32,
    section0_length: u32,
    section1_offset: u32,
    section1_length: u32,
}
```

### Layout of the format crates: `format/` and `ops/`

The single-concern format crates (`cpk`, `fpk`, `ftex`, `fox2`, `uniparam`, `wezlib`, `kit_config`)
need no prescribed layout — one module tree, read and write, tests. `fmdl` and `pes_model` are
different: each carries **format-native algorithms** on top of the codec (mesh splitting, split
vertex encoding, anti-blur decode/encode, multi-model merging, path editing) and has a **second
consumer outside the workspace** (the Blender extension via `python_bindings`). Both crates use the
same two-level split so that boundary is visible in the tree:

```
crates/libs/fmdl/src/
├── lib.rs
├── format/             # the codec
│   ├── mod.rs          #   FmdlError
│   ├── file.rs         #   FmdlFile
│   ├── skl.rs          #   SklFile, the SKL codec
│   └── container.rs, records.rs, f16.rs, vertex.rs
├── model/              # the semantic layer: Model (mod.rs), from_file.rs, to_file.rs, validate.rs
├── ops/                # algorithms
│   ├── mod.rs          #   the op modules
│   ├── split/          #   mesh splitting (mod.rs, build.rs, combine.rs)
│   ├── vertex_enc.rs   #   split vertex encoding
│   ├── antiblur.rs
│   ├── merge.rs        #   multi-FMDL merging
│   └── paths.rs        #   path-table editing
└── check.rs            # deep validation → findings

crates/libs/pes_model/src/
├── lib.rs
├── format/             # the codec
│   ├── mod.rs          #   ModelError
│   ├── model/          #   PreFoxModel, the .model blocks
│   ├── mtl.rs          #   sibling .mtl XML
│   └── container.rs, datum.rs, records.rs, vertex.rs, write.rs, fixtures.rs
├── model/              # the semantic layer: Model (mod.rs), validate.rs
├── ops/                # algorithms
│   ├── mod.rs          #   the op modules
│   ├── split/          #   mesh splitting (mod.rs, build.rs, combine.rs)
│   ├── vertex_enc.rs
│   ├── merge.rs
│   └── paths.rs        #   texture stem rewriting
└── check.rs            # deep validation → findings
```

Each `model/`, `ops/split/` and (in `pes_model`) `format/model/` directory also holds its
`tests.rs`.

Rules:

- **`format/` is pure binrw**: structs, read, write, and nothing that changes a model's meaning. A
  round-trip through `format/` alone is byte-identical on the layout the game's own files use
  (tested on the Konami fixtures). Another writer's layout round-trips equal in records and
  buffers but not in bytes: `fmdl`'s writer emits Konami's layout, and the community add-on
  pads every section-0 block to 16 where Konami aligns only block 13. Nothing needs the
  add-on's bytes back (FMDL parity against Red is compared on the decoded model), so the
  writer does not carry a per-file padding mode.
- **`ops/` are format-native transformations** for lossless same-format work; they operate on the
  crate's own types, never on `model_convert`'s IR (the dependency points the other way).
  The algorithms both crates carry (mesh splitting, the split's combine, split vertex
  encoding, the `check` limits) are therefore written twice, and stay so, even when the
  algorithm itself changes: they differ where the formats do (byte against float weights, bone
  positions from the bone against the inverse bind matrix, each crate's mesh fields, error type
  and bone-group set), and one shared version would need a new crate and a trait carrying all
  of those, so a reader would follow one split through three files instead of one, and both
  crates would share a dependency their Python bindings now build without. Instead each
  mirrored file names its twin in its header, and a change to one goes to the other in the
  same commit; the duplication check's keep list holds each pair, so a diff that changes one
  copy alone shows in review.
- **`check.rs` returns findings, not messages** — stable codes with context, mapped to catalog
  entries by the consuming tool, mirroring `aesthetics_export`'s rule.
- **Geometry far from the origin is an Error in both crates' `check`**
  (`fmdl_vertex_far_from_origin`, `model_vertex_far_from_origin`, one finding per mesh, `count` =
  the vertices past the limit): a vertex more than 5000 units from the origin (Euclidean
  distance of its position; exactly 5000 passes) makes the game lag persistently for the whole
  matchday. Every vertex counts, referenced by a face or loose; edges and faces need no test of
  their own, since every point of one lies within the reach of its farthest vertex. The Team
  compiler refuses such a model (`vertex_too_far_from_origin` in its catalog).
- **The `python_bindings` surface is `format/`'s entry points**, exposed as-is; if a function is awkward to
  expose, that is a hint it belongs in the Team compiler rather than here. Today the wheel
  carries each codec's `read`/`write` (`fmdl`'s `FmdlFile` and `SklFile`, `pes_model`'s
  `PreFoxModel` and `MaterialSet`) and nothing more; the `ops/` exposure is in
  `plans/libs/format_crates.md`. "As-is" means the
  entry points (file read/write, vertex and face decode/encode), not the codec's
  internals: `fmdl` keeps its raw container, vertex-attribute layout types, string lookup and
  split limits `pub(crate)` until a consumer names one.
- Core plan guardrail 4 (PyO3-buildable, dependency denylist) applies to the whole crate; the split
  does not relax it for `ops/`.

### `uniparam`: a canonical writer, parity with the reference writer

`UniformParameter` reads any container (WESYS-wrapped or not; every offset bounds-checked,
duplicate names refused), refuses at `insert` a name holding a NUL (the name pool is
NUL-terminated, so such a name could not read back), and writes one canonical layout: entries in byte-wise name order, the
name pool directly after the entry table, each content padded to 16. That layout is the
reference writer's (pes-file-tools, which Red uses to rebuild the container), reproduced byte for
byte on its sample, because Red's output is the compiler's parity standard. Konami's own PES 21
container differs from it in two measured ways, so a Konami file rewritten is equal in entries
but not in bytes: its table is in name order with `_` collating before the digits (`1_DEF…`
before `10_DEF…`), and its content pool starts 16-aligned (nine bytes of padding after the name
pool). Neither is reproduced: the game reads both layouts, and matching Red is what the parity
tests need. So the crate's round-trip standard is `read(write(x))` equal in entries, not the
byte identity a pure `format/` layer owes; the byte test is against the reference writer's
sample.

### `fmdl::model`: the semantic layer the ops work on

`format/` is records and buffers; the ops (splitting, anti-blur, merging, path editing) reason
about bones, materials, meshes and groups. `model.rs` is the layer between them: a `Model` built
from an `FmdlFile` and written back to a fresh one. It is not byte-identical (a fresh file is laid
out from scratch); it is *semantically* lossless: `Model::from_file(&FmdlFile::read(x)?)?.to_file()`
read back gives an equal `Model`, on Konami files as well as add-on-written ones (the legacy writer
could not rebuild Konami files at all; that is the bar this layer clears).

```rust
pub struct Model {
    pub bones: Vec<Bone>,
    pub materials: Vec<MaterialInstance>,
    pub meshes: Vec<Mesh>,
    pub mesh_groups: Vec<MeshGroup>,
    /// The `X-FMDL-Extensions` header: which encodings the file declares.
    pub extensions: Extensions,
    /// Section-1 block 1, 64 bytes per bone in Konami files; carried as is (the add-on writes it empty).
    pub bone_matrices: Option<Vec<u8>>,
}
pub struct Bone { pub name: String, pub parent: Option<usize>, pub bounding_box: BoundingBox, pub local_position: [f32; 4], pub world_position: [f32; 4] }
pub struct BoundingBox { pub max: [f32; 4], pub min: [f32; 4] }
pub struct Texture { pub file_name: String, pub directory: String }
pub struct MaterialInstance {
    pub name: String, pub shader: String, pub technique: String,
    /// (sampler name, texture), in file order.
    pub textures: Vec<(String, Texture)>,
    /// (parameter name, four floats), in file order.
    pub parameters: Vec<(String, [f32; 4])>,
}
pub struct Mesh {
    pub vertices: MeshVertices,          // the codec's type; bone indices index `bone_group`
    pub faces: Vec<[u16; 3]>,
    pub bone_group: Vec<usize>,          // indices into `Model::bones`, at most 32
    pub material: usize,                 // index into `Model::materials`
    pub alpha_flags: u8,
    pub shadow_flags: u8,
    pub has_antiblur_meshes: bool,       // per-mesh extension headers
    pub is_antiblur_mesh: bool,
    pub custom_bounding_box: Option<BoundingBox>,
}
pub struct MeshGroup { pub name: String, pub parent: Option<usize>, pub meshes: Vec<usize>, pub bounding_box: Option<BoundingBox>, pub visible: bool, pub split_mesh_group: bool }
pub struct Extensions {
    pub mesh_splitting: bool, pub antiblur: bool, pub vertex_loop_preservation: bool,
    /// `X-FMDL-Extensions` values outside the three, kept verbatim for the rewrite.
    pub other: Vec<String>,
}
impl Model {
    pub fn from_file(file: &FmdlFile) -> Result<Model, FmdlError>;
    pub fn to_file(&self) -> Result<FmdlFile, FmdlError>;
    /// The invariants every op and `to_file` rely on; `from_file` output always passes.
    pub fn validate(&self) -> Result<(), FmdlError>;
}
```

**One list of invariants.** `Model`'s fields are public, so a caller (the IR exporter, a
merge, a test) can build a model whose indices dangle. `validate` is the one place that says
what a well-formed `Model` is: bone and mesh-group parents in range and acyclic, every mesh's
material and bone-group entries in range, every face index below the mesh's vertex count,
every attribute vector one value per vertex (at most four uv maps, one precision flag each,
bone weights and indices together), every group's mesh indices in range. It does not check the
format limits (32 bones, 65535 vertices, 21845 faces: `ops::split` exists to take meshes over
them) or a weighted bone slot past the mesh's bone group (Konami files carry those; `check.rs`
warns). `from_file` ends with it, `to_file` and the `Model`-level ops in `ops/` start with it
(`ops::paths`, which edits texture paths on the typed record layer `FmdlFile`, takes no
`Model`), and none of
them re-checks an index it covers, so an invalid model is an error, never a panic, and the
list cannot drift between five partial copies (before 2.20f `to_file`, `merge` and `split`
each checked a different subset and `antiblur` none). `to_file` adds one rule of its own,
the one `from_file` enforces on read: every mesh in exactly one group, so it never writes a
file its own reader refuses.

`from_file` resolves every index through the tables (strings, bounding boxes, bone groups,
materials, textures, parameter assignments, mesh-group assignments) and reads the extension
headers from the string table's tail (`X-FMDL-Extensions:` plus per-object headers such as
`Has-Antiblur-Meshes: 0,3` listing mesh indices, `Split-Mesh-Groups: 2` listing group indices,
`Custom-Bounding-Box-Meshes`), the `key: value, value` grammar the add-ons write. Every dangling
index is an error, never a panic, face indices included. Per-object headers other than those
four are dropped: their values are mesh or group indices that any op renumbers, and no known
writer emits another (unknown `X-FMDL-Extensions` flags carry no index and are kept in
`Extensions::other`).

**Anti-blur meshes.** Fox PES's temporal anti-aliasing (TAA) blurs meshes that use the
`Constant` material. The community found that a `Blin` mesh placed behind a `Constant` one
cancels the blur, so the fix is a duplicate of such a mesh with a `Blin` material, hidden by
the original; it costs that mesh's vertices and faces again. As the add-on builds it, and
`ops/antiblur.rs` likewise, the duplicate copies the mesh's vertices and faces unchanged and
differs only in its material (`fox3DDF_Blin_Fuzzblock`, or the `_UVScroll` variant), alpha
flags 128 plus the source's two-sided bit 32 (so 160 for a two-sided source), and shadow flags
1. How the game then draws it behind the original is not known.
The `antiblur` flag with its `Has-Antiblur-Meshes` header marks those duplicates for the
Blender FMDL add-on (`Tools_4cc/pes-fmdl`): it discards them on import and recreates them on
export, so an author never edits the duplicate by hand. `ops/antiblur.rs`
is that encode step. FoxDen is to force TAA off in PES, after which the duplicates and the flag
are no longer needed; until then a written model keeps them as the add-on would.

**Custom bounding boxes.** The format has no per-mesh box; `Custom-Bounding-Box-Meshes` only
marks a mesh whose box the author set by hand, and the add-on folds that box into the mesh's
*group* box. We do the same both ways: `from_file` gives a marked mesh `custom_bounding_box =
Some(its group's box)`; `to_file` emits the header for every mesh with `Some`, and when it
computes a group box (the group had none) it uses a mesh's custom box in place of that mesh's
vertex extent. A group box the model already carries is written as is. An anti-blur duplicate
keeps its source's custom box (the add-on copies the source's headers onto the duplicate), so
duplication never grows a hand-set group box back to the geometry.

**Mesh-group assignments.** A group whose meshes are not consecutive (a split replaces a mesh
by a child group, leaving its siblings apart) is written as one assignment record per run, all
naming the group's one bounding box. `from_file` accepts repeated assignments to a group when
they name the same bounding-box id and refuses a second, different id, which is the add-on
reader's rule. `to_file` lays a file out the way the add-on writer does, which
years of add-on-written models prove PES accepts: positions in buffer 0 (stride 12), the other
attributes interleaved in buffer 1 in the order normal, tangent, color, bone weights, bone indices,
uv maps (a uv map identical to an earlier one shares its offset), faces in buffer 2; strings
de-duplicated; a bounding box per bone and per mesh group that has meshes (computed when the
source had none, which is why the legacy writer failed on Konami files: over its meshes and its
child groups' boxes, as the add-on computes it; a group without meshes or box gets no
assignment record, Konami's own layout); one level-of-detail record;
the fixed blocks 18 and 20 as the add-on writes them; the extension headers re-emitted after the
last string. The codec's per-mesh vertex kinds (normal, tangent, color, bone mapping, uv count and
precision) are exactly what `MeshVertices` carries, so no separate "vertex fields" record exists.

### `pes_model::format`: the `.model` container and its sections

A `.model` is a pointer graph, not a block table: eleven sections, each a *record array* (a
12-byte table of contents `u32 first_record_offset, u32 record_count, u32 record_size`, an
optional header of `first_record_offset - 12` bytes, then the fixed-size records) followed by the
data its records point at, every offset relative to the array's own start. Layout measured on all
2610 Konami `.model` files of a PES 2017 install plus the two add-on-written card heads (the
fixtures in `crates/libs/pes_model/tests/fixtures/` are the representatives, one per variant):

- 8-byte magic `MODEL\0\0\0`, then `u32 table_offset` (16 always), `u16 unknown` (0 always),
  `u16 version` (19; one shadow model is 17), `u32 unknown` (9 always), `u32 flags` (0; 4 in two
  face-montage models, meaning unknown, carried verbatim). The section table is a record array of
  eleven `u32` offsets at file offset 24, relative to 24; the sections follow back to back from
  offset 80, each 4-aligned, the file ending with the last one. Konami writes them in the order
  0 1 2 3 4 5 6 8 9 10 7 without exception, the add-on in the order 7 0 5 6 3 8 9 10 1 2 4, so
  table order and file order differ and both are kept for byte identity.
- Section roles: **0** bone data (entry 0: one `float32Matrix34` record per bone, the inverse
  bind matrix, in bone-name order; every further entry: one bone group, a `u16` bone-index list;
  a boneless model still has the empty entry 0), **1** geometry (per mesh: one vertex set holding
  one vertex-field descriptor array, one face descriptor, and an extras array of bounds /
  material-combination flags `1,0,0,0` / an order word 0), **2** annotation strings, **3**
  annotation records (28 bytes, always `0 0 0 2 2 2 0`, one per annotation of each mesh), **4**
  meshes (signed offsets from section 4's start into sections 0, 1, 2, 3 and 6, which is what
  makes the graph cross-section), **5** bone names, **6** material names, **7** model bounds and
  the LOD record, **8** cloth, **9** material combinations, **10** locator geometry.
- Version 17 layout (the shadow model): 20-byte mesh records without the trailing editor-data
  pointer, two geometry extras (no order word), 12-byte section-10 records. Every other file:
  24-byte mesh records whose editor-data array is present and empty, three extras, 16-byte
  section-10 records with a 4-byte zero header.
- LOD: 532 of the 2610 files (the `modD_*collar*` shirt parts) carry 4 to 7 LOD levels: the face
  descriptor names a table of `(start, end)` face-vertex ranges that partition the face stream in
  order of decreasing detail, level 0 first, and section 7's LOD record is
  `u32 level_count, f32 0.0625, f32 4.0, f32 0.3` (`0, 0.0625, 4.0, 0.0` without LODs). With no
  LODs Konami's `lod_table_offset` points at the section end; the add-on writes 0.
- Mesh annotations: `(string, section-3 record, 7, type)`. Konami types: **1** the part name
  (`prt`, `glasses_02`, `head_color`), always first; **10** the same string again (hair, glasses);
  **2** `DSpecularS`; **7** a normal-map name (`DNormalS`, `HeadNormal`, `head_normal_default`,
  `<hair>_face_normal`). The add-on defines **128** mesh name and **129** extension header, with
  no section-3 record. The reference parser reads only 128 and 129, so it drops Konami's.
- Konami's empty section 8 has table-of-contents offset 0 (not 12) in every file; section 9's is
  12. Konami aligns some pointed-at data to 8 or 16 with zero padding in no fixed pattern; no file
  carries an unreferenced non-zero byte. Section 8 (cloth), 9 and 10 are empty in every one of
  the 2610 files, and no mesh references section 10; the vertex-field header's cloth word and the
  per-mesh editor-data array are always zero and empty. Two third-party trophy props (not
  Konami's) have sections at odd offsets and annotation word 8, which the container reads fine.
- Editor data (the 24-byte mesh record's last pointer): an array of `u32` offsets, relative to
  the array's own start, each to an item `u32 kind, u32 unknown, value`, the value a
  NUL-terminated string padded to 4 when `kind == 1` and a `u32` otherwise (stadium and prop
  models carry editor tool names and node types this way; no player part does). This item
  layout comes from the reference parser's notes, not from a file: none of the 2610 has a
  non-empty array, so the typed layer's decode of it is specified but unverified against a real
  sample, and says so in its doc comment.

Community files break two of those regularities, and the census of every `.model` on the
maintainer's machine (2026-09-28: 6575 distinct files, 4145 read before the fixes below; the
fixtures README names the representatives) is what the reader is held to beyond the Konami set:

- **Offsets reach past their section.** An offset is relative to its array's own start, but
  nothing confines the data it names to the section's run: in 1640 files a geometry's
  vertex-field and face offsets land after the last section in file order (section 1's own run
  sometimes holding a stale, unreferenced copy). That is how the game and the reference parser
  read them, as a stream over the whole file from the section's start. So the typed layer
  resolves every section's offsets against the unwrapped file from that section's start to the
  file's end. The container's runs (a section's bytes up to the next section's start) stay what
  byte identity is measured on; a rewrite through the typed layer moves such data back into its
  section.
- **Annotations that point nowhere.** One writer puts the constant pointers `-124`/`-84` (word
  8, kind 1) in every annotation of every file (228 files, stadium parts, balls, faces and boots
  among them), so they land wherever those offsets happen to fall. An annotation whose string
  pointer does not land on a section-2 record, or whose non-zero record pointer does not land on
  a section-3 record, is dropped on read, as the reference parser drops it; the reference notes
  PES reads neither section, and these files are in cup use. The strings no kept annotation
  references become model-level headers, as for any unreferenced string.
- **Zero-length data anywhere.** Empty geometries (0 vertices, 0 face indices) point their field
  and face offsets at or past the end of the file; a zero-length read succeeds wherever it
  points, and only a read of at least one byte past the end is `Truncated`.
- **An empty array's header.** In four files, section 8 (cloth, always empty) has a garbage
  first-record offset: the previous section's string runs over the table's location. An array
  with no records whose declared header would run past the end of the buffer reads as having
  no header, as Konami's own section 8 with offset 0 does. An array with records keeps every
  check.
- **A field type listed twice.** A few files carry a second descriptor for a type already
  listed (a bone-weights descriptor covering half the vertices, inside the first one's data).
  The first descriptor of each type is the field; a later one of the same type is ignored by
  decoding (and left untouched by `encode_vertices`), as the reference parser does, before the
  vertex-count and duplicate rules apply to the rest.

So the crate has two format layers, like `fmdl`: **`ModelContainer`** (header fields, the eleven
sections as opaque byte runs in file order) is the byte-identical one (`write(read(x)) == x` on
every fixture, the WESYS-wrapped ones compared unwrapped), and **`PreFoxModel`** is the typed
layer over it: flags and the version the file declared, bones (name + matrix), bone groups, material names,
annotation strings and records, geometries with their vertex-field descriptors, raw field data,
face stream and LOD ranges, meshes with every cross-section pointer resolved to an index, model
bounds. Its `write` lays a fresh file out the add-on's way (section order, 4-padding, no
alignment gaps, the version-19 record sizes) and always writes version 19 in the header: that
word with that layout is the one combination years of add-on-written models prove PES 16 and 17
accept, and a version-17 word over version-19 record sizes has never been seen by a game. So a
version-17 file rewritten becomes a version-19 file, with the version-19 layout throughout (a
geometry without an order word gets order 0, so three extras), and `read(write(m)) == m` holds
on every fixture once `version` is set to 19 and a missing order word to 0; otherwise the layer is semantically lossless, not
byte-identical, because reproducing Konami's padding would mean carrying every offset. Everything Konami writes and the add-on drops is kept: annotation types 1, 2, 7 and 10
with their section-3 records, LOD tables and the LOD record. Sections 8, 9 and 10 are written
empty; a model whose geometry or meshes reference them (cloth, material combinations, locators,
none of which any Konami player part uses) is a read error naming the feature, not a warning,
since a rewrite could not preserve it.

### `pes_model::format::mtl`: the sibling material set

Every `.model` ships with a `.mtl`: a small XML file, WESYS-wrapped like the model, whose shape
was measured on all 945 of a PES 2017 install: a `<materialset>` root holding `<material
name shader>` elements, each holding `<sampler>`, `<state>` and `<vector>` children in any order
(states before or after samplers, both occur). No XML declaration, no BOM, no text content.
`<sampler>` attributes always appear in the order `name, path, srgb, minfilter, magfilter,
mipfilter, uaddr, vaddr, waddr, maxaniso` with trailing ones omitted (four subsets seen); values:
`srgb` 0/1, filters `linear`/`anisotropic` (the plan's material schema also allows `point`),
addresses `clamp`/`wrap`/`repeat`, `maxaniso` 2; paths end in `.dds` and are either `./name.dds`
or `model/...` from the data root. `<state name value>`: the seven names the material schema
validates plus `shadowcaster`, values 0, 1, 64, 254, so the name stays a string and the value an
integer. `<vector name x y z w>` (two files carry `x` alone), component text `0`, `0.0`, `0.035`,
`500.0`. 917 files are CRLF with four-space indents (eight for children), 15 use tabs, the rest
mix; 943 end with a newline.

`MaterialSet` is the typed reader (`roxmltree`) and hand writer. The writer emits one canonical
layout under a per-file `MtlStyle` (newline, indent unit, final newline) detected on read and
defaulting to Konami's majority (CRLF, four spaces, final newline); vector components are written
as the shortest `f32` text (`0`, `0.035`). Measured before coding: that writer reproduces 909 of
the 945 files byte for byte; the 36 others mix indentation, align attributes with extra spaces,
carry blank lines or trailing spaces, or write `0.0`, and are re-formatted on a rewrite with no
semantic change. Byte parity is tested on the regular fixtures (card head, `headHi`, `hair`,
`shadow`), the semantic round trip on all of them. An element or attribute the grammar does not
name is a read error, never dropped.

The census of every `.mtl` on the maintainer's machine (2026-09-28: 1509 distinct PES material
sets read, 405 Wavefront OBJ `.mtl` files correctly refused) added three rules:
- **`maxfilter`** is a sampler attribute: 3 community exports write it between `minfilter` and
  `magfilter`. It is read and written back in that position with the filter values, and is
  absent in every Konami file.
- **Content after the root element is ignored.** In 8 community files the real
  `</materialset>` is followed by a second closing tag or binary junk. The reader parses up
  to the end of the first root element; a rewrite drops the tail.
- **A repeated state name takes its first value**, in `check` as in `model_convert`'s import.
  No file on the machine repeats one, so this only makes the three readers agree.

Stray text between elements stays a read error of `MaterialSet::read`, the compiler's reader: a
Studio export is hand-made or upgraded, and the Export upgrader cleans it. The 2026-10-19
census of all 12,755 `.mtl` files on the machine found three that carry any, all typos (a `\`
or a backtick after an element, an edit's leftover `dr="clamp"/>`), one of them in a cup
export, which Red's parser (ElementTree) ignored. `MaterialSet::read_dropping_stray_text` is
the same parse skipping such text, and says whether it skipped any; the upgrader reads an old
export's `.mtl` with it and writes the set back, which drops the text.

### `pes_model::model`: the semantic layer the ops work on

As for `fmdl`, the ops reason about bones, materials and meshes, not records: `Model` is built
from a `PreFoxModel` and written back to a fresh one, semantically lossless
(`from_file(to_file(m)) == m` on every fixture). Compared with `PreFoxModel` it decodes the
vertices (`MeshVertices`), splits the face stream into level-0 faces and lower LOD levels, gives
each mesh its own bone group as indices into `bones`, and sorts the annotations by meaning: kind
128 is the mesh name and kind 129 a per-mesh extension header (the add-on's), every other kind is
a Konami tag kept as `(kind, text)`; annotation strings no mesh uses are the model's extension
headers (`Skeleton-Type: Simplified` on the card heads). Bounds are carried as read and
recomputed by the ops that move vertices; the section-7 LOD record is carried verbatim, with a
constructor for new models (`level_count`, `0.0625, 4.0`, `0.3` with LODs and `0.0` without).
`to_file` writes one section-3 record per Konami tag, a bone-group entry per mesh, and a present
but empty editor-data array unless the mesh carries items; a face index past the mesh's vertex
count is an error, never a panic. Degenerate faces are kept (the reference importer drops them;
a model layer that alters faces on read cannot claim to be lossless). One repair is the
exception, because its input has no faithful reading at all: an old exporter counted loose
vertices it did not write, so every face index past such a vertex is too high by the number
of skipped ones and some point past the vertex table (4 census files, cup exports among them).
When a mesh has a face index at or past its vertex count and the distinct indices over all its
LOD levels number exactly its vertex count, `from_file` maps each index to its rank among those
distinct indices, the one order-preserving reading that uses every vertex (the reference
importer's repair, which modders saw in Blender when they imported the file). Any other
out-of-range index is `BadReference`, as before.

**One list of invariants**, as in `fmdl::model`: `Model::validate(&self) -> Result<(),
ModelError>` is the one place that says what a well-formed `Model` is: every mesh's `material`
and `bone_group` entries in range; every attribute vector one value per vertex (normals,
tangents, bitangents, colors, each uv map, bone indices, bone weights), at most four uv maps,
bone weights only together with bone indices (indices alone are the referee-card case) and
`bone_weight_width` 2, 3 or 4 when weights are present; every face index of every LOD level
below the vertex count. It does not check the format limits (64 bones, 65535 vertices, 21845
faces: `ops::split` takes meshes over them; `check`'s `model_mesh_over_face_limit` stays an
Error at 21845 although 193 community files, made with an old exporter, go past it: the
current modelling add-on enforces that limit, and a few hundred missing faces are too few to
settle it in game, maintainer 2026-09-28) or a weighted bone slot past the bone group
(`check.rs` warns). `from_file` ends with it; `to_file`, `merge` (on each part) and
`split::encode`/`decode` start with it; `vertex_enc`'s per-mesh `encode` and `decode` start
with its mesh-local half (`Mesh::validate`, crate-private), and `decode`/`decode_model` return
`Result` for that. None of them re-checks an index it covers, so an invalid model is an error,
never a panic. `split::encode` also checks the hierarchy its caller supplies: one entry per
bone, every parent in range (cycles are cut, as before).

### `pes_model::ops::merge`: several parts into one `.model` + `.mtl`

The pre-Fox counterpart of `fmdl::ops::merge`, native over `Model` and `MaterialSet`, so both
engines merge through the same kind of code and a same-format operation never round-trips
through `model_convert`'s IR. (The plan's first answer routed the pre-Fox `ingame_face` merges
through an IR-level `merge_ir_parts`; dropped because it made them the one exception to "same
format skips the IR" for no gain: the `.mtl` merge is a material-name merge either way, and the
native op is what the Blender bindings can call.) A part is the pair the game loads:

```rust
pub fn merge(parts: &[(&Model, &MaterialSet)]) -> Result<(Model, MaterialSet), MergeError>;

pub enum MergeError {
    MaterialConflict { name: String },  // two parts define one material name differently
    SkeletonConflict { name: String },  // two parts carry one bone name with different matrices
    FlagsConflict,                      // header flags differ (meaning unknown, so no merging rule)
    Other(ModelError),                  // a part's index out of range (never expected)
}
```

`parts` is in caller order (canonical, preserved), so the output is deterministic. Rules,
`fmdl`'s unless stated:

- **Bones** unioned by name; a bone in several parts must carry the same twelve matrix floats
  within `1e-4` per component (absolute), else `SkeletonConflict`; the first part's matrix is the
  merged one. The tolerance is measured, not chosen: over the 2606 Konami files, a bone name
  shared between files differs from its first occurrence by at most `3.6e-5` per component where
  the files share a skeleton (8268 bone pairs of float noise, kit parts included: `modD_cap`
  against a collar differs by up to `1e-4`-ish on four shoulder bones) and by at least `2.0e-4`,
  up to `0.5`, where the bind pose really differs (the gloves' forearm and hand bones, the shadow
  model, the special hair types' face bones), so `1e-4` sits in the gap. Exact comparison, the
  plan's first answer and `fmdl`'s rule, cannot merge two Konami kit parts. Each mesh's
  `bone_group` is remapped to the union; per-vertex bone indices index the group and do not
  change.
- **Materials** by name: `Model::materials` unioned in first-seen order and mesh `material`
  remapped; the `.mtl` materials unioned by name the same way, two definitions of one name equal
  (`Material` equality, entry order included) or `MaterialConflict`. A `.mtl` definition no model
  names is kept (Konami shares one `.mtl` between models); a model naming a material its `.mtl`
  lacks is `check_bundle`'s finding, not the merge's. The merged `.mtl` takes the first part's
  `MtlStyle`.
- **Meshes** concatenated in part order with only `material` and `bone_group` rewritten, and
  split components renumbered: each part numbers its `Split-Mesh` groups from 1, so the merge
  renumbers them in output order from 1 across all parts (a part's components keep their
  grouping, and two parts' groups never meet in `split::decode`).
- **Model level**: `extension_headers` unioned, deduplicated, first-seen order; `flags` must
  agree; `bounds` is the union of the parts' boxes; `lod` is `LodRecord::for_levels(n)` with `n`
  the most levels any mesh carries (`1 + lower_lods.len()`, 0 without lower levels), which
  reproduces every Konami record.
- `merge` of one part is the identity, tested on every fixture bundle.
