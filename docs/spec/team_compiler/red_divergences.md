# Unlimited Tool Works: Compiler spec: Intended divergences from Red

Part of the [Compiler spec](README.md).

This is the one list of every place the compiler's output — or the Export upgrader's
migrated export, once compiled — knowingly differs from what Red produced from the same old
export. A difference is intended only once it has been checked against the real exports on
the maintainer's machine: which real exports reach the case, what Red's output does for each,
that ours is right for each. Each entry's **Status** says honestly whether that check exists:
**Checked** (real exports or an in-game check named), **No census needed** (the difference
never reaches a file the game reads or a member's result in game), **Not checked** (no such
check on record), or **Records disagree** (the repository states two different things).

## Packing layout and paths

### RD-1: No `*.fpk.xml` in the CPK

- **Red:** copied the source-side `face.fpk.xml`/`boots.fpk.xml`/`glove.fpk.xml` into the CPK
  beside each FPK it lists.
- **Ours:** no `*.fpk.xml` is produced; the compiler writes each FPK from the folder's own
  files.
- **Why:** the file is the source-side member list of an FPK, which no game reads; carried
  over it would only make a `file_not_used` Warning per model at every compile.
- **Evidence:** about 5,400 such files sit in the maintainer's export folders
  (`docs/spec/export_upgrader.md`, step 1); "a file no game reads".
- **Status:** Checked.
- **Stated in:** `parity.rs` `NotProduced` rows (tracer and referee tracer);
  `tests/fixtures/tracer/README.md`, `tracer_refs/README.md`;
  `spec/export_upgrader.md` step 1; `spec/team_compiler/testing.md` "Testing: parity against
  Red"; classes.md I1; DECISIONS 2026-10-18 (the `*.fpk.xml` item).

### RD-2: Boots and gloves pack under planned IDs, not the folder-name ID

- **Red:** packed a boots or gloves folder under the ID its name carried (`k2180`, `g2180`),
  inside the team's 25-ID block.
- **Ours:** the Studio format has no ID-named folders; the compiler packs the models under
  the IDs its 40-ID per-team block assigns (the parity tracer fixture's `k3745`/`g3745`).
- **Why:** the Studio format assigns boots/gloves IDs itself; the old folder-name ID is the
  migration's hint, not a contract. The VGL27 `--legacy-ids` mode exists because a manager's
  Red test CPK and the official save would disagree on every custom ID.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `parity.rs` `RelocatedContainer`/`Relocated` rows;
  `tests/fixtures/tracer/README.md`; `spec/team_compiler/testing.md` "Intentional
  differences"; `plans/team_compiler/pipeline.md` "Legacy boots and gloves IDs (VGL27)".

### RD-3: Player textures relocated to a per-player common subfolder

- **Red:** kept a team player's textures inside his face CPK beside the `.mtl` files that
  name them `./<name>` (pre-Fox), or packed them inside each model folder's structure (Fox);
  for a Pre-Studio `Players/` export it relocated them to `common/<team>/<name part>/`.
- **Ours:** every texture of a player folder goes to that player's one common subfolder
  (`common/<team>/<folder>/` on Fox, packed under `common/character1/` on pre-Fox), the
  model's texture paths rewritten to point there. A shared folder merged into several
  wearers leaves a copy per wearer where Red held one.
- **Why:** a single predictable location for every player's textures; textures shared
  between a player's packages are packed once. It generalizes Red's referee-only common
  preprocessing (`referee_tools.py`) to every player.
- **Evidence:** "verified extensively in-game for players as well as referees, across
  versions" (`pipeline.md`, the deviation note); Red's referee layout itself is called
  in-game-verified.
- **Status:** Checked.
- **Stated in:** `parity.rs` `Relocated(COMMON_SHIRT)` rows; `parity_prefox.rs` `Nested::Texture`
  rows and `RELOCATED`; `tests/fixtures/tracer_prefox/README.md`; `pipeline.md` "Per-model-folder
  parallel steps" item 6 ("Texture relocation to common" / "Intentional deviation from Red's
  packed layout for teams"); `testing.md` "Intentional differences"; classes.md I2.

### RD-4: `unlimited-tool-works/` records inside the CPK

- **Red:** wrote no such entries.
- **Ours:** the compiler may write its own records, e.g.
  `unlimited-tool-works/shared_ids/<team>.toml` (the team's shared-ID assignments a later
  `Midcup` — an additions-only export — reads back), at paths no game reads.
- **Why:** `Midcup` recompiles must keep the IDs the main CPK assigned.
- **Evidence:** the paths are not read by the game (`parity.rs` filters them out before
  comparing).
- **Status:** No census needed: record files at paths no game reads.
- **Stated in:** `parity.rs` and `parity_prefox.rs` (the `unlimited-tool-works/` filter and
  the `shared_ids` assertions); `spec/aesthetics_export/player_folders.md` "At compile time"
  (shared-ID records).

### RD-5: `kitN`/`kit1`…`kit9`/`kitF` spellings in packed names and paths

- **Red:** emitted the legacy `u0???p0`/`u0???p<d>`/`u0???g<d>` spellings (with the `XXX`
  placeholder or a baked-in team ID) in file names and in FMDL/MTL/XML paths; the modded
  exes' (the community's patched game executables) swap served them.
- **Ours:** emits the Studio spellings (`kitN` in paths, `kitF` in file names, `gk_kit<d>`
  for the goalkeeper token), and refuses the legacy token in an export (`kit_token_old`,
  `dummy_kit_name_old`).
- **Why:** two spellings of one thing are what a new member cannot tell apart; the modded
  exes (FoxDen) match the `kitN` spelling directly and can be modded to it at any time.
- **Evidence:** 71 real `u0???p0` files sit on the maintainer's machine; FoxDen's kit
  swapper keeps a `u0???p0` path whose variant is missing (the `kitF` role).
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Per-export non-model steps" (Kit-dependent assets);
  `testing.md` "Intentional differences"; `model_format.md` "Kit-dependent assets";
  classes.md I4; DECISIONS 2026-11-11 "kit_token_old checks only the files and paths the
  Export upgrader respells".

### RD-6: No `dummy_kit_replace` fallback copies

- **Red:** `dummy_kit_replace.py` copied the team's kit 1 textures over `dummy_kit*` files in
  Common, a fallback for the kit substitution.
- **Ours:** no such copies are emitted; the stand-in stems are written in the game's
  spelling for the exes to substitute.
- **Why:** the fallback dates from when the substitution lived in Sider scripts; it is in
  the exes now and stable.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Per-export non-model steps" (Kit-dependent assets, last
  paragraph).

### RD-7: Multi-CPK mode splits by size, not by content type

- **Red:** split multi-CPK output by content type: `4cc_40_faces` / `4cc_45_uniform` /
  `4cc_08_bins`.
- **Ours:** splits all team content into a sequence of size-capped `teams` CPKs
  (`cpk_part_max_size`, default 3 GiB) plus the bins CPK; unfilled DPFL slots get the empty
  placeholder CPK.
- **Why:** Git for Windows cannot handle objects of 4 GiB or more, and the cup DLC is
  versioned in Git; the faces/uniform distinction stops paying for itself.
- **Evidence:** a 32-team `4cc_40_faces.cpk` already sits at 3.77 GB, so a 48-team cup lands
  near 5.7 GB (measured, `pipeline.md` "Multi-CPK mode: teams parts").
- **Status:** Checked.
- **Stated in:** `pipeline.md` "5. Writer" item 6 "Multi-CPK mode: teams parts";
  `settings.md` `multicpk_mode` row.

### RD-8: Overrides always land in the team side

- **Red:** routed `sideload/` files into the refs CPK when only refs compiled, and ignored
  the folder in multi-CPK mode.
- **Ours:** `overrides/` entries always go into the team CPK (or the bins CPK in multi-CPK
  mode), whichever export commits.
- **Why:** Red's routing is an accident of its CPK naming; an override is an operator's file
  and always wins its path.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "5. Writer" items 1 and 6; DECISIONS 2026-10-11 "the Red
  behaviors Studio deliberately differs on".

### RD-9: `pass_through` keeps fewer file kinds

- **Red:** packed whatever it found under `pass_through`.
- **Ours:** `pass_through` keeps fewer kinds of file.
- **Why:** Red's broad keep is an accident of its folder walk.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-11 "the Red behaviors Studio deliberately differs on";
  `messages.md` `pass_through` rows.

### RD-10: A `Common/` file nothing references is not packed

- **Red:** packed it.
- **Ours:** a `Common/` file no model, link or reference names is not packed (`file_not_used`
  on PES 18-21 for a subfolder file no link reaches).
- **Why:** an unreferenced Common file is never loaded by the game.
- **Evidence:** the class is defined by non-reference: nothing names the file, so it draws
  nothing.
- **Status:** Not checked: no census of whether the game loads such a file by a path the export
  never writes.
- **Stated in:** classes.md I28; `pipeline.md` "Per-model-folder parallel steps" (Common);
  `spec/aesthetics_export/player_folders.md` "Common model links and model merging".

### RD-11: A pre-Fox `Common/` model named `oral_x` is not double-prefixed

- **Red:** packed it as `oral_oral_x_win32.model` (the `oral_` prefix applied again).
- **Ours:** packed as `oral_x_win32.model` — affixes are added only when the stem lacks
  them.
- **Why:** doubling the affixes would type `oral` content wrongly; Red lowercases and skips
  affixes already present only because old exports carry them.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I35; `pipeline.md` "Per-model-folder parallel steps" step 4
  (packed names).

### RD-12: A `Common/` file the upgrade drops as a leftover is not packed

- **Red:** packed it.
- **Ours:** the upgrader drops it as a `leftover_dropped` Warning and the CPK holds nothing
  for it.
- **Why:** a leftover is content that never reached a player in game.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I40; `spec/export_upgrader.md` step 10 "Leftovers".

### RD-13: Files the game never reads are reported, not packed

- **Red:** packed files the game never reads (a `.txt` note, a `.png` beside a model) into
  the CPK.
- **Ours:** a `.txt` in a player or shared folder is admitted as a note and reported
  `file_not_used`; it is not packed.
- **Why:** a note beside a model is a comment; dead content in the CPK tells the member
  nothing.
- **Evidence:** none cited.
- **Status:** No census needed: the files are ones the game never opens.
- **Stated in:** classes.md I22; DECISIONS 2026-10-11 "a kit-texture path in an FMDL is a
  Warning; a `.txt` note in a model folder is not a disallowed file";
  `messages.md` `file_not_used`.

### RD-14: Pre-Fox gloves `.mtl` files are packed flattened at the root

- **Red:** not stated in the text (the reference's gloves `.mtl` placement is what the rows
  compare against).
- **Ours:** every `.mtl` a gloves output packs sits at the output's root, its name the path
  below the folder with each `/` written `_`, lowercased (`grip/materials.mtl` →
  `grip_materials.mtl`); `glove.xml` names it there.
- **Why:** the game does not load a `.mtl` texture path that climbs out of the `.mtl`'s
  folder.
- **Evidence:** in game on PES 17 Edit mode (2026-10-15), a `grip/` `.mtl` naming
  `../magenta.dds` drew white while `./magenta.dds` and `./sub/magenta.dds` loaded.
- **Status:** Not checked: what Red's output placed for a gloves `.mtl` is not on record.
- **Stated in:** `pipeline.md` "Per-model-folder parallel steps" step 7 (pre-Fox gloves
  output).

## Textures

### RD-15: `.ftex` zlib chunks are not byte-identical to Red's

- **Red:** wrote its chunk compression with its own zlib build's deflate output.
- **Ours:** writes the same pixel data in different chunk bytes; byte identity with zlib's
  output is declared a non-goal, so `.ftex` files compare by header fields and decoded DDS.
- **Why:** the chunk compression is a deflate encoder's choice; matching Red's bytes would
  mean reproducing one zlib build's deflate output.
- **Evidence:** the tracer's `shirt.ftex` and `u0792g1.ftex` differ from Red's only inside
  the zlib-chunk encoding and convert back to byte-identical DDS (DECISIONS 2026-10-03, the
  parity-tier decision).
- **Status:** Checked.
- **Stated in:** `parity.rs` `compare_leaf` (tier-2 `.ftex` comparison);
  `tests/fixtures/tracer_refs/README.md`; `testing.md` tier 2; DECISIONS 2026-10-03.

### RD-16: DDS and FTEX headers are written in one canonical form

- **Red:** copied the source DDS as it was — GIMP's reserved header bytes, no mip flags, a
  WESYS wrapper kept — and let a `.ftex` keep the source's header shape.
- **Ours:** writes the DDS header in one canonical form (mip count 1 with its flags), an
  FTEX as type 0x9/depth 1, and writes a WESYS-wrapped file unwrapped.
- **Why:** the header itself is not compared, its decoding is; a wrong mip count or pixel
  format still fails.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `parity_prefox.rs` `Row::Texture`/`Nested::Texture` doc comments;
  `tests/fixtures/tracer_prefox/README.md`; classes.md I7b.

### RD-17: An uncompressed texture may be re-encoded

- **Red:** copied an uncompressed DDS as it was (but refused an uncompressed kit main
  texture on PES 15-17).
- **Ours:** keeps an uncompressed source uncompressed (A8R8G8B8) except a normal-role
  source (encoded to DXT5nm) and a kit's main texture on PES 15-17 (block-compressed,
  because those versions soft-crash on an uncompressed one above a 2048×2048 DXT5's
  5.33 MB, which is why Red refuses it there).
- **Why:** a member saves a texture uncompressed on purpose; the two exceptions are cases
  the game cannot take as stored.
- **Evidence:** the PES 15-17 soft-crash is the cited mechanism. **The records disagree:**
  `tests/fixtures/tracer_prefox/README.md` says the compiler "encodes the uncompressed
  `k2012_sr.dds` to BC1", while `processing/texture.rs` (`keep_uncompressed = true` for
  player textures) and the parity row's codec comparison say a stored-uncompressed
  color-role source is written as it is.
- **Status:** Records disagree.
- **Stated in:** `spec/libs/dds_convert.md` "Passthrough"; `tests/fixtures/tracer_prefox/README.md`;
  classes.md I8; DECISIONS 2026-10-11 "a BC4, BC5 or BC7 portrait for PES 15-18 is encoded
  to BC3" (the codec rule it builds on); DECISIONS 2026-11-11 "texture_not_div4".

### RD-18: DX10-header textures pass through; normal maps go to DXT5nm

- **Red:** re-encoded every DX10-header texture with texconv's `-f DXT5 -srgb`, a lossy
  step that drops a BC5 normal map's X channel.
- **Ours:** rewrites a DX10-header BC1/BC3 with a legacy header rather than re-encoding,
  keeps BC5 on PES 19-21 (FTEX format 9), and encodes a normal-role source to DXT5nm
  (pre-Fox: the grey-Y layout).
- **Why:** the legacy compilers never produced the DXT5nm conversion; the Konami files are
  the standard.
- **Evidence:** Konami's own normal-map layout is the cited standard
  (`spec/libs/dds_convert.md`, the DXT5nm rows).
- **Status:** Not checked: no real export is named that reaches a DX10 header or a BC5 normal map.
- **Stated in:** `spec/libs/dds_convert.md` "Passthrough" and the DXT5nm conversion; classes.md
  I26.

### RD-19: The last two BC blocks of a re-encoded texture

- **Red:** its encoder's smallest two mips.
- **Ours:** the last two BC blocks (the smallest mips) differ.
- **Why:** encoder detail of the suite's own mip generation.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I15.

### RD-20: A PNG or other raster source becomes the `.dds` its material names

- **Red:** packed the source file.
- **Ours:** converts the raster source to the `.dds` (or `.ftex`) the model's material
  names.
- **Why:** material references name textures by stem and the compiler resolves each stem to
  the actual file, converting for the target.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I27; `pipeline.md` "Texture conversion" (step 5).

### RD-21: `dds_compression` reaches referee content

- **Red:** applied `dds_compression` to team exports only.
- **Ours:** applies the optional zlib DDS compression (PES ≤17) to referee content too, so
  the same textures compress where Red's stayed plain.
- **Why:** no reason given beyond uniform application ("each Red behavior is an accident
  of its folder walk or its CPK naming").
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-11 "the Red behaviors Studio deliberately differs on";
  `settings.md` `dds_compression`.

### RD-22: Logos are produced, not passed through; portraits re-encoded canonically

- **Red:** packed the member's own emblem files (the 512² `_r_ll`, 256² `_r_l` and 128² `_r`
  PNGs) and the portrait file as it was.
- **Ours:** produces the three PNGs — decodes `logo*` (and `logo_small*` when present),
  squares per the `_crop`/`_stretch`/`_fit` name tag, Lanczos3-resamples to 512²/256²/128²,
  PNG-encodes — under Red's destination names; a portrait is encoded in the canonical form
  of RD-16.
- **Why:** one source cannot get the sizes wrong; most old small logos are plain
  downscales the compiler regenerates, while a deliberate variant (`logo_small`) is carried
  when more than 40 % of pixels differ.
- **Evidence:** census of the 94 distinct real `Logo/` pairs on the maintainer's machine
  (77 downscales score 0–30 %, 17 deliberate variants 53–97 %; the two real pairs nearest
  the line are the fixtures).
- **Status:** Checked.
- **Stated in:** `export_upgrader.md` step 7 "Logo"; `pipeline.md` "Per-export non-model
  steps" (Logo); DECISIONS 2026-09-11 "logo is one root image (+ optional `_small`), sizes
  generated at compile time"; classes.md I9.

### RD-23: A portrait in a texture format is converted properly

- **Red:** **the records disagree.** `spec/export_upgrader.md` says Red renamed a
  `player_???NN` portrait of another texture format to the game's `.dds` name with its
  bytes unchanged (a jpg named `.dds`); the DECISIONS entry of 2026-10-19 (correcting the
  earlier reading) says Red converted every portrait to DDS before placing it.
- **Ours:** migrates the file keeping its format and the compiler's `Portraits/` pass
  converts it to a real DDS.
- **Why:** the portrait its author meant, readable by the game.
- **Evidence:** BAG VGL18.5's `player_XXX08.jpg` sits in Red's own output as `78808.dds`
  (6.12c, 2026-10-19); the maintainer asked whether Red really converted those.
- **Status:** Records disagree.
- **Stated in:** `export_upgrader.md` step 7 "Portraits"; DECISIONS 2026-10-19 "a portrait
  Red converted or read by its short name migrates" and the correction entry under it.

## Models

### RD-24: Anti-blur duplicate meshes are stripped

- **Red:** packed a model's anti-blur duplicate meshes (`Is-Antiblur-Meshes` copies and each
  mesh's `Has-Antiblur-Meshes` request) as authored.
- **Ours:** the upgrader writes the FMDL without the copies (`fmdl::ops::antiblur::decode`,
  reported `antiblur_dropped`, I); the compiled model carries none.
- **Why:** FoxDen's TAA removal keeps models on Forward shaders sharp without them, so the
  copies are bloat that draws every mesh twice.
- **Evidence:** census: 3,566 of the 6,374 distinct readable FMDLs on the maintainer's
  machine carried them; a fuzzblock shader without the flag occurs in 6 files.
- **Status:** Checked.
- **Stated in:** `parity.rs` `decoded_model` (anti-blur normalized away on both sides);
  `spec/export_upgrader.md` step 7 "Anti-blur duplicates dropped"; `spec/libs/format_crates.md`
  (`fmdl::model`); classes.md I36.

### RD-25: A model hidden by distance is dropped by the upgrade

- **Red:** packed the far model (the pre-Blender trick for hiding the default face or hair:
  every vertex more than 5000 units from the origin).
- **Ours:** the upgrader drops such a model before planning (`model_far_dropped`, W); the
  face folder it leaves with no model compiles to the blank face, which hides the default
  face the same way, and a gloves folder becomes a hidden hand.
- **Why:** far geometry lags the match all day (the compiler's
  `vertex_too_far_from_origin` stays an Error), so carrying the file would only drop the
  player folder at compile time.
- **Evidence:** census 2026-11-05: 49,122 `.fmdl`/`.model` files (14,688 distinct), 23 far
  in every vertex (VGL26 faces/hairs, three old gloves files, two stadium models); in VGL26
  they are 89 files (9 distinct) in 47 folders across 7 exports.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 7 "Two cleanups"; classes.md I37.

### RD-26: The hand auto-split moves hand-weighted meshes into the gloves

- **Red:** packed the face model as authored.
- **Ours:** a hand-weighted face part is split: its hand-bone (`skh_*`) weighted vertices
  become `glove_l`/`glove_r` parts of the gloves packages, merged with authored gloves;
  reported `model_hand_split` (I).
- **Why:** the split gloves are the parts the cup's full-body models were authored around;
  detection precedes planning because the gloves IDs and tasks are planned from file names.
- **Evidence:** measured through the round trip on Red's `00007 - BLANK` `fcl_hair.fmdl`
  (the model's own bones are kept); the residue rule was measured in game on HDG VGL26's
  `EAT` (6.16c): stray 1/255 hand weights move with their grow ring.
- **Status:** Checked.
- **Stated in:** classes.md I23; `pipeline.md` "Fox mode fixups" item 3;
  `spec/model_conversion/hand_split.md`; DECISIONS 2026-10-20 "a hand split takes stray hand
  weights as they are".

### RD-27: A model may be re-bound through the IR (`skeleton_retargeted`)

- **Red:** packed the model with its source binding.
- **Ours:** a model whose skeleton must be retargeted across versions is re-bound through
  `model_convert`'s IR (the conversion intermediate representation); its bytes differ where
  Red's kept the source binding.
- **Why:** cross-version skeleton retargeting is an IR operation the plan names.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I14, I14? (an FMDL re-bound or paired, checked by hand).

### RD-28: A model with every mesh hidden is dropped

- **Red:** packed it (26 blank-head `oral.fmdl` files in MARISA's roster are the BLANK-face
  trick).
- **Ours:** an FMDL whose every mesh is hidden is left out with `model_hidden_dropped` (I);
  a hidden mesh inside a model stays.
- **Why:** leaving the model out is what Fox shows; an error would drop a package for a
  file that draws nothing.
- **Evidence:** census 2026-10-10: 61 hidden meshes of 35,251 in 7,315 files, 38 files with
  every mesh hidden (26 blank-head `oral.fmdl`, 8 one-sided glove placeholders); the
  maintainer's rule ("no mesh used in the 4cc is ever invisible", with the scan that proved
  the flag is used on purpose).
- **Status:** Checked.
- **Stated in:** DECISIONS 2026-10-10 (the hidden-mesh decision); `messages.md`
  `model_hidden_dropped`; `spec/model_conversion/ir.md` "A hidden Fox mesh".

### RD-29: An `.mtl` is re-serialized, not copied

- **Red:** copied each `.mtl` file as it was (unwrapping a WESYS one).
- **Ours:** writes it back through `MaterialSet` — the same materials, possibly a different
  text layout.
- **Why:** the compiler's own writer emits the parsed set; the comparison is on materials,
  not bytes.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `parity_prefox.rs` `Nested::Mtl` doc comment;
  `tests/fixtures/tracer_prefox/README.md`.

### RD-30: FMDL directory strings are appended, not rewritten in place

- **Red:** rewrote the old texture-directory strings in the FMDL string table in place.
- **Ours:** appends the new directory strings and leaves the old ones unreferenced — the
  model bytes differ in the string table alone, the decoded model identical.
- **Why:** the writer's canonical layout; the parity test compares decoded models.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `tests/fixtures/tracer_refs/README.md` ("What the comparison found").

### RD-31: A mesh group without a bounding box gets one computed

- **Red:** its writer wrote `None` for a box it never read (a group without meshes).
- **Ours:** computes a box over the group's meshes and its child groups' boxes, as the
  community add-on computes it; a group without meshes or box gets no assignment record
  (Konami's own layout).
- **Why:** Konami files hold such groups, and Red's writer stopped on them.
- **Evidence:** Konami files hold such groups (real game files observed).
- **Status:** Not checked: the groups were seen in Konami's files, not in a real export.
- **Stated in:** `spec/libs/format_crates.md` (`fmdl`, the writer's bounding-box rule).

### RD-32: An arbitrary-named model joins the face merge on both engines

- **Red:** renamed a single arbitrary-named face FMDL to `fcl_hair.fmdl` (the single-file
  fallback).
- **Ours:** a model with no recognized suffix is face content on both engines, merged into
  the player's packages (`fmdl_fcl_hair_fallback`, I, per routed file).
- **Why:** the fallback generalized — arbitrary names have nowhere else to go on Fox.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/aesthetics_export/player_folders.md` "Model names" (last table row);
  `pipeline.md` "Fox mode fixups"; `messages.md` `fmdl_fcl_hair_fallback`.

### RD-33: An FMDL texture found only in `Common/` is linked there, not repointed at the folder

- **Red:** its team pass pointed such a texture at the model's own folder, where the file
  was missing; its referee pass pointed it at the team's Common.
- **Ours:** the upgrader writes the stem and a `.common` link to the team's Common — the
  referee rule widened to teams — giving the model the texture its author shipped.
- **Why:** Red's team-pass repoint left the model without a texture its export held.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 7 "FMDL texture directories"; DECISIONS
  2026-10-17 item (13).

### RD-34: A `Common/` model on Fox is baked into its players, not packed in Common

- **Red:** packed it in the team's Common output.
- **Ours:** a linked Common model's meshes are merged into each linking player's output
  FMDL; an unlinked one is ignored (no finding).
- **Why:** Fox engines cannot load models from Common at all, so a link resolves only by
  baking.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I39; `spec/aesthetics_export/player_folders.md` "Common model links
  and model merging"; `pipeline.md` step 3 "Fox mode fixups".

### RD-35: An FMDL kit-texture path is kept with a Warning, not silently repointed

- **Red:** released Red reports nothing and repoints a `/Assets/pes16/model/character/
  uniform/texture/` directory at the model's own folder — the file is not there, so the
  model loses the texture silently.
- **Ours:** the path is kept unchanged and reported `fmdl_texture_kit_path` (W).
- **Why:** the modded exe swaps kit textures per match so the path cannot work; a member is
  told why the texture will look wrong while the outcome stays Red's.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-11 (the kit-texture path decision) and 2026-10-18
  premise (1); `messages.md` `fmdl_texture_kit_path`.

### RD-36: Texture-existence findings differ from Red's

- **Red:** never checked a Fox model's textures at all (its `fmdl_no_texture_ids` warning
  has no equivalent) and warned for every pre-Fox miss it found.
- **Ours:** checks both engines — `fmdl_texture_not_found`/`mtl_texture_not_found` is a
  Warning that keeps the package on a mesh-used material and an Info on a material no mesh
  uses, with an installed-CPK lookup before a miss is reported.
- **Why:** real models carry leftover hidden meshes naming textures nobody shipped; Red
  packed them and the cup played them, while dropping the package would swap in the default
  boots or face.
- **Evidence:** maintainer ruling 2026-11-05; "Red packed them and the cup played them"
  (/dbg/ player 10's hidden mesh naming `medic_bsm`); the pre-Fox tracer's Fumos face
  played a cup as Red compiled it with a warning.
- **Status:** Checked.
- **Stated in:** `pipeline.md` "Resolved decisions" ("A texture a model names must exist");
  `messages.md` "Texture existence"; DECISIONS 2026-11-05 "a texture nobody supplies is a
  Warning on Fox too".

### RD-37: No PES 16 "no models from Common" refusal

- **Red:** refused Common-model references on PES 16.
- **Ours:** the check is intentionally not ported; the PES16 exe is treated as patched to
  load models from Common.
- **Why:** the patch is made when the cup next plays PES 16.
- **Evidence:** none cited; the exe patch is planned, not yet made.
- **Status:** Not checked.
- **Stated in:** `messages.md` under "XML/MTL checks".

### RD-38: The pre-Fox `.mtl` search is exact `materials.mtl` in a defined order

- **Red:** its default `.mtl` kind accepted any stem `materials` starts or ends with
  (`mat.mtl` counts) and took the listing's order; finding nothing, it named a missing
  `materials.mtl` in the xml — a file the game cannot load.
- **Ours:** takes an exact `materials.mtl`, then any `.mtl`, in case-folded name order;
  finding nothing is `model_material_undefined` (Error) instead of a broken reference.
- **Why:** an exact name and a defined order give the same answer on every real export and
  one a member can predict; a name the game cannot load is an Error's subject.
- **Evidence:** "the same answer on every real export" (asserted, not a counted census).
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-07 "the `.mtl` a pre-Fox `.model` uses, restated from
  Red's code"; `model_format.md` "Material files".

### RD-39: FMDL texture names keep their letter case

- **Red:** normalized the case of texture names it rewrote.
- **Ours:** keeps the source's letter case in the written texture names.
- **Why:** not stated beyond the class's own line.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I24.

### RD-40: Other-engine files: ignored or converted, never a reason to drop the folder

- **Red:** per-engine allowlists discarded the folder holding a file of the other engine's
  format (a `.fmdl` in a pre-Fox export, a `.model` on Fox).
- **Ours:** the allowlist is the same on every target: a face folder's other-engine file is
  ignored, an item folder's is converted (`pipeline.md` step 3's conversion path).
- **Why:** Red's per-engine lists predate conversion, which makes every model format a
  source for either engine.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I25, I38; `spec/aesthetics_export/object_model.md`
  "File-type allowlist"; DECISIONS
  2026-10-01 "aesthetics_export — 3.6 (b): allowlist, OS artifacts, scopes, eligibility".

## Faces

### RD-41: The blank face for a folder with no face model

- **Red:** released Red generates no face folder for a player folder without a face model
  (its unreleased development branch packed one); for a referee folder Red packed the
  referee's `face/` holding its own `face_diff.bin` alone.
- **Ours:** every roster-mapped folder — a team player's or a referee's (since commit
  `1bccacd9`) — without `ingame_face` and without a face model gets the blank face on both
  engines. On Fox it is a `face.fpk` holding only the bundled template `face_diff.bin`
  plus an empty `face.fpkd`; on pre-Fox a face CPK with a dummy model, `dummy.mtl` and a
  generated `face.xml` carrying the template diff. The folder's own `face_diff.bin` is
  reported `face_file_not_used` (I).
- **Why:** the absence of the default PES face is the final element of FPC (the cup's
  full-body-model mode), so the folder must exist — the bundled diff is the blank face's
  whole content.
- **Evidence:** maintainer ruling that a blank face always takes the bundled diff
  (2026-10-04); the VGL organizer compiled the VGL27 refs export on PES 19 and saw the
  game's referee heads floating over their bodies (the case the rule exists for);
  census: in every real refs export the 15 referees with no face model hold a boots model,
  the other 147 a face model; 467 VGL26 face folders hold a `face_diff.bin` (345 an
  outdated FPC diff).
- **Status:** Checked.
- **Stated in:** `parity.rs` `Row::BlankFace` rows and the `face_file_not_used` assertion;
  `pipeline.md` step 4 (the blank folder); `referees.md` "Referee export processing";
  `testing.md` (the referee tracer's difference list); DECISIONS 2026-10-04, 2026-10-18
  premise (4), 2026-11-20 "A referee folder with no face model gets the blank face";
  classes.md I12, I18.

### RD-42: An empty old face folder migrates to the blank face, not an empty package

- **Red:** packed an empty face package for an empty face folder — how a player with a
  full-body model removes the placeholder face, as part of the FPC system.
- **Ours:** the folder migrates to nothing of its own: the player folder holds no face
  file and no marker, and the compiler writes the blank face of RD-41.
- **Why:** the maintainer's answer — the empty folder's whole meaning is the
  full-body-model-hides-face case, which the blank face reproduces.
- **Evidence:** census: PMMM VGL26 holds 18 such folders, GIG VGL26 10, the 4.2.2 test
  export `3` 2; maintainer's answer quoted in DECISIONS 2026-10-19.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 3; DECISIONS 2026-10-19 "an empty face
  folder means no `ingame_face` marker".

### RD-43: A face folder holding the game's own path migrates its innermost folder

- **Red:** its face CPK for such a folder nests the member's tree below the player's own
  path, and the `face.xml` Red generates at the player's path names models that are not
  beside it — under Red the face did not load.
- **Ours:** the innermost folder's contents become the face folder, so the member's face
  (and his own `face.xml`) is what compiles.
- **Why:** the folder is the member's face for the slot, an extracted face CPK of their
  own; reproducing Red's output would ship a face that does not load.
- **Evidence:** real Red output inspected: FGOG VGL6's `face/real/79501.cpk` holding the
  nested `.../real/81401/` tree and a generated `face.xml` pointing at models not beside
  it (a CPK listing, not checked in game); FGOG minipatch and /co/'s fetus export are the
  same case (2016-2018 exports).
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 3; DECISIONS 2026-10-19 (the rule) and
  2026-10-20 "correction: Red does not deliver a game-path face folder"; classes.md I30.

### RD-44: A generated `face.xml`'s `ratio` attribute stops at the next `_`

- **Red:** took the whole rest of the stem after `ratio_` (`visor_ratio_2_parts` →
  `ratio="2_parts"`).
- **Ours:** takes the token up to the next `_` (`ratio="2"`).
- **Why:** `2_parts` is a value the game cannot read as a ratio.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` step 4 (packed names); DECISIONS 2026-10-07 (the generated
  `face.xml` decision).

### RD-45: `face_neck` is typed for any `face_high` model, not only the exact name

- **Red:** gave the `face_neck` type only to a file named exactly `face_high_win32.model`.
- **Ours:** gives it to any model whose suffix is `face_high`.
- **Why:** the game's `face.mtl` name-matches either stem, so the wider typing is safe.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/aesthetics_export/player_folders.md` "Model names" table; DECISIONS
  2026-10-19 (the `.mtl` search entry, noting Red's reading is not copied).

### RD-46: A model-type prefix glued to the next word is not a type

- **Red:** typed `eyebrow_x` as `eye` in generated xml (it matched types with no delimiter).
- **Ours:** recognizes a type only followed by `_` and more of the name, so `eyebrow_x`
  keeps no type in the generated `face.xml`.
- **Why:** moving letters off a word makes a name nobody wrote.
- **Evidence:** census: none of the 856 models in the xml-less face folders of the
  maintainer's exports is named so.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 7 "Model-type prefixes"; DECISIONS
  2026-10-18 (the model-type decision).

### RD-47: Unlisted models and unused `.mtl` files are not packed

- **Red:** packed them.
- **Ours:** a `.model` the member's `face.xml` does not list is `xml_model_unlisted`,
  neither emitted nor compared; an `.mtl` no packed model uses is not packed either.
- **Why:** the xml is the member's statement of what the game should load; an unlisted
  model is almost always a mistake, occasionally the point — a Warning says so.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I10, I10b; `messages.md` "User-supplied `face.xml`"
  (`xml_model_unlisted`).

### RD-48: Packed model names and a generated `face.xml`'s types

- **Red:** its own packed names (`face_high_win32.model` kept as listed).
- **Ours:** packed names differ where the compiler generates the xml — `oral_`/`_win32`
  affixes and lowercasing are applied, the types and materials the same — and a generated
  `face.xml` lists `hair_high` as `parts`.
- **Why:** the compiler's generated xml follows its own name rules, the same rules Red's
  own generator uses where they overlap.
- **Evidence:** the `parts` typing for boots matches "the reference's own generator"
  (parity_prefox.rs); the `hair_high` case has none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I19, I19b; `parity_prefox.rs` `FACE_XML` table;
  `spec/aesthetics_export/player_folders.md` "Model names".

### RD-49: A WESYS-wrapped member `face.xml` is refused

- **Red:** unwrapped and compiled it.
- **Ours:** `xml_broken` (E), the folder discarded — the upgrader writes every member
  `face.xml` unwrapped, so migrated exports never hit the refusal.
- **Why:** the compiler parses a member's `face.xml` as plain XML only.
- **Evidence:** census: 48 such files on the maintainer's machine, each of which would
  cost a player his face at the first compile of the upgraded export.
- **Status:** Checked.
- **Stated in:** DECISIONS 2026-10-18 "a WESYS-wrapped `face.xml` is written unwrapped";
  `messages.md` `xml_broken`.

### RD-50: `face_diff` validation and two-source conflicts

- **Red:** let the `.bin` win silently when both `face_diff.bin` and `face_diff.xml` stand,
  and checked the decoded length for equality only for the xml.
- **Ours:** both files in one folder is `xml_dif_conflict`; a diff in either form must have
  the `FACE` magic and at least the length its header gives — a longer file passes.
- **Why:** a shorter file makes the game read past its end; equality would refuse the text
  form of files the game loads.
- **Evidence:** census: 326 of 2,695 loose `face_diff.bin` on the maintainer's machine are
  16 bytes longer than their header gives, all in cups' CPKs.
- **Status:** Checked.
- **Stated in:** DECISIONS 2026-10-03 "what a valid face diff is, and `face_diff.xml`'s
  two forms"; `messages.md` `face_diff_invalid`, `xml_dif_conflict`.

### RD-51: An unresolvable path in a member's `face.xml` is a Warning kept verbatim

- **Red:** let only `model/character/face/common/…` through silently and reported any other
  unresolvable path form as an Error for a model or `.mtl`, a Warning for a texture.
- **Ours:** `xml_path_unchecked` (W) for any path form the compiler cannot resolve, the
  value kept verbatim.
- **Why:** a warning is "I can't vouch for this", never "I changed this"; it is recorded as
  one of the compiler's pre-Fox gaps against Red.
- **Evidence:** none cited; the gap is on the worklog.
- **Status:** Not checked.
- **Stated in:** `messages.md` "User-supplied `face.xml`"; DECISIONS 2026-09-12 (the
  severity boundary).

## Kits and bins

### RD-52: The kit config's texture-name fields are re-derived, not kept

- **Red:** copied the member's binary config, rewriting only the team ID in the texture-name
  fields, so a kit keeps the names the maker wrote — including names for textures the kit
  does not ship (a copied-from-kit-1 config naming kit 1's fonts).
- **Ours:** `config.toml` drops the name fields and the compiler re-derives them from the
  kit's effective texture set (for a `Midcup` kit, from the set plus the installed CPKs):
  an absent texture's name field is left empty.
- **Why:** without the derived fields the copied configs would compile kits with no
  numbers or names; a name for a texture the game cannot find says nothing.
- **Evidence:** census: 59 of the 1,041 distinct old configs on the maintainer's machine
  (in 23 exports) borrow another kit's fonts; 162 fields of the 1,038 configs name a
  texture of another stem, 43 with the named texture shipped; the real JP 2024 Spring
  Day 8 Midcup resends configs naming installed textures.
- **Status:** Checked.
- **Stated in:** `parity_prefox.rs` `Row::KitConfig`/`compare_kit_config`;
  `tests/fixtures/tracer_prefox/README.md`; `export_upgrader.md` step 5 "Copied textures";
  `pipeline.md` "Bins accumulation"; DECISIONS 2026-10-19 "a Midcup kit's config names the
  number textures the game already holds"; classes.md I6.

### RD-53: Mask and srm are engine-specific

- **Red:** converted a `kit_mask` to FTEX and shipped it on Fox targets (a file the game
  never opens), and wrote its flat template mask `u0NNNg1_mask.dds` for a kit with no mask.
- **Ours:** emits `kit_mask` only for PES 15-17 targets and `kit_srm` only for 18-21; the
  other engine's map is not emitted (`kit_texture_not_used`, I). A pre-Fox target without
  a mask gets the bundled template (as Red's `kit_masks_check` filled it); a Fox target
  without an srm gets nothing (Red never made one). A `Midcup` kit's mask is not
  re-emitted: the installed one stands.
- **Why:** each map is read by name convention only and their channels mean different
  things; neither is ever converted into the other.
- **Evidence:** census: all 788 PES 17 and 1,359 PES 21 stock kit configs checked name
  neither map; 0 of 365 kits in the maintainer's library ship an `_srm`, 14 ship a mask;
  70 of 641 surveyed ship no mask. **The records disagree:** `tracer_prefox/README.md`
  says "the mask is byte-identical (both write Red's template)" while `parity_prefox.rs`'s
  `NotProduced` row asserts a Midcup compile emits none.
- **Status:** Records disagree.
- **Stated in:** `parity_prefox.rs` `OUTER` mask row; `pipeline.md` "Per-export non-model
  steps" (Kits); DECISIONS 2026-09-12 "kit `_mask` (pre-Fox) and `_srm` (Fox)";
  `tests/fixtures/tracer_prefox/README.md`; classes.md I13, I13b.

### RD-54: `UniColor.bin` is built on the bundled base, not Red's fallback base

- **Red:** built the bin on its fallback base, whose record for the tracer's team was
  empty before the kit's entry.
- **Ours:** builds it on the bundled base (`resources/bins/UniColor.bin`), keeping the
  base's other kits in the team's record.
- **Why:** the bundled bases serve from-scratch compiles the same role Red's
  `Engines/bins/` did, but they are current files, not Red's.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `parity_prefox.rs` `Row::UniColor`/`compare_uni_color`;
  `tests/fixtures/tracer_prefox/README.md`.

### RD-55: A Full export rebuilds the team's bins from the export alone

- **Red:** rebuilt the team's `UniColor.bin` record from the Note's kit list (every kit of
  the team) and left placeholder records.
- **Ours:** a `Full` export's record holds its committed kits and nothing else (empty `p1`/
  `g1` slots get placeholder kits: checkerboard `kit.dds`, generated `config.toml`,
  magenta/black `kit_colors_missing` pair); a `Midcup` kit's absent pieces keep the
  installed entry.
- **Why:** the record's count is what makes the game offer a kit, so a full rebuild must
  drop what the export does not carry; the placeholder mechanism is Red's silent
  `kit_mask.dds` fill generalized.
- **Evidence:** 750 of the bundled base's 821 records are the game's own placeholder
  records (ten white entries numbered 0); the checkerboard is 64² DXT1 like Red's mask
  template (~3 KB).
- **Status:** Not checked: the evidence is about the bundled base, not what Red's output held
  for a real Full export.
- **Stated in:** `pipeline.md` "Bins accumulation"; classes.md I5; DECISIONS 2026-09-12
  "empty kit folders are placeholder kits".

### RD-56: A `Midcup` kit's absent config may be generated from the template

- **Red:** the run installs nothing for a kit it does not resend.
- **Ours:** a `Midcup` kit without `config.toml` emits no config — the installed entry is
  edited in place — and where nothing is installed or the installed config does not
  decode, the template is written (`kit_config_generated`).
- **Why:** the slot needs a config the game can read.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Bins accumulation"; classes.md I29.

### RD-57: Team colors are capped at the record's four

- **Red:** wrote team colors with no cap, so a fifth would overwrite the next team's ID.
- **Ours:** uses the first four valid colors (`TeamColor.bin`'s record capacity); later
  lines report `color_entry_invalid`.
- **Why:** the record holds four.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/aesthetics_export/player_folders.md` "Root files" (Colors);
  `pipeline.md` "Bins accumulation".

### RD-58: Custom collars rewrite the team's kit configs

- **Red:** passed collar files through without changing the configs.
- **Ours:** derives the replaced stock collar's ID from the `collar_<ID>` filename and sets
  it as the collar and winter collar in all of the team's kit configs — the "intentional
  deviation from Red" the spec names — with 105 (FPC) and 77 (referee marker) reserved and
  `collar_id_conflict` for a second claimant.
- **Why:** it puts the custom model on every player at once, a quick alternative to
  per-player models.
- **Evidence:** none cited for the rewrite itself (the collar material choice is backed by
  an in-game check on PES 21: two stock PES 17 collars converted with the converter's
  materials drew nothing, 4.y-ingame2, 2026-10-10).
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Per-export non-model steps" (Collars, the deviation note);
  `testing.md` "Intentional differences"; `tests/cli/collars.rs`.

### RD-59: The working-bin walk follows the installed `DpFileList.bin` bottom-up

- **Red:** approximated the same precedence by sorting the DPFL's CPK names in reverse
  alphabetical order, and searched only CPKs whose names hold `midcup`, `uniform`, `faces`
  or `bins`.
- **Ours:** walks the actual `DpFileList.bin` from the bottom up over every listed CPK,
  starting just below the run's own output CPK, for both the working bins and the Common
  texture lookup.
- **Why:** the bottom of the list has the highest PES loading priority; a bins CPK not
  named `midcup` is still read by the game, so the name filter misses real sources.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Bins accumulation" and "Resolved decisions" (working-bin
  lookup; installed texture lookup); DECISIONS 2026-10-03 "a model's missing texture is an
  Error; installed CPKs are searched", 2026-10-11 "the Red behaviors Studio deliberately
  differs on".

### RD-60: `bins_updating` cannot be turned off

- **Red:** had a `bins_updating` setting.
- **Ours:** bins updating is unconditional.
- **Why:** it only existed before "fetch the latest installed bins and update those" was a
  thing; skipping it is never useful now that recompiles patch the current cup state.
- **Evidence:** none cited.
- **Status:** No census needed: a dropped setting; the output is produced either way.
- **Stated in:** `pipeline.md` "Bins accumulation" and "Resolved decisions".

### RD-61: The `_back`/`_chest`/`_leg` number atlases are re-arranged for the target

- **Red:** shipped the member's atlas as it was (Red never converted across engines).
- **Ours:** re-arranges an atlas in the other engine's arrangement into the target's
  (`NUMBER_ATLAS_SLOTS`: PES 15-17's column of ten equal cells ↔ PES 18-21's row of fixed
  slots), glyphs moved by a uniform scale.
- **Why:** the two arrangements are not the same cells turned; neither engine reads the
  other's.
- **Evidence:** measured on the games' stock files — every stock PES 17 `_back`/`_chest`/
  `_leg` (797 of each) is a column, every stock PES 21 one (1,357 of each) a row;
  `kit_leg_atlas.py`.
- **Status:** Checked.
- **Stated in:** `pipeline.md` "Layout conversion" (the atlas paragraphs); classes.md I7.

### RD-62: A kit drawn for the other layout has its sock islands re-laid

- **Red:** shipped the kit unchanged (no cross-engine compile exists in Red; the community
  converters that shipped cross-engine kits never edited the layout, so every one had
  displaced socks).
- **Ours:** when the layout marker names the other engine, the kit's `kit` (and mask/srm
  where the target takes one) is decoded, re-laid by `KIT_LAYOUT_REMAP` (two bands per
  sock island, mirrored on the right) and re-encoded (`kit_layout_converted`, I).
- **Why:** only the two sock islands differ between the engines' layouts; the map's slope
  varies along the leg, so one scale cannot fit.
- **Evidence:** measured from the games' own uniform models (the sock islands span 432→364
  px; the table sits within 6 px rms of the angle pairs and 10 of the arc pairs over three
  sock variants); PES Master's two kit creators agree within 15 px — a cross-check, never
  a source.
- **Status:** Checked.
- **Stated in:** `pipeline.md` "Layout conversion"; `tests/fixtures/kit_layout/README.md`;
  DECISIONS 2026-09-12 "kit layout marker and cross-engine re-layout".

### RD-63: A `Full` export's orphan kit texture is a leftover

- **Red:** packed a kit texture naming a slot that no config and no Note entry gives (no
  `UniColor.bin` entry named it, but the file shipped).
- **Ours:** it is a `leftover_dropped` Warning and the CPK holds nothing for it — in a
  `Midcup` export the same file is the slot's texture update and becomes its kit folder.
- **Why:** no `UniColor.bin` entry names it, so the game never offered it.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 5 "Files that do not migrate as kits" and
  step 10's leftover list; classes.md I31.

## Referees

### RD-64: A refs-only run writes a team-side CPK for the bins

- **Red:** writes no team CPK for a referee export.
- **Ours:** on PES 18-21 writes the team side holding only `UniColor.bin`,
  `UniformParameter.bin` (the bundled base plus every referee kit config under its file
  name — each config is also its `UniformParameter.bin` entry) and `TeamColor.bin`; on
  PES 15-17, whose referee configs are loose files alone, the refs CPK stands alone.
- **Why:** on PES 18-21 each referee kit config is also its entry in
  `UniformParameter.bin`.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `parity.rs` `REFS_TEAM_SIDE` and the referee test's team-CPK assertions;
  `pipeline.md` "5. Writer" (Output modes and targets);
  `tests/fixtures/tracer_refs/README.md`; `testing.md` difference list; classes.md I21
  (part).

### RD-65: A referee's modelless item folder packs nothing

- **Red:** packed an empty package for a referee's modelless boots or gloves folder.
- **Ours:** writes nothing for it (referees are full FPC; compare RD-79's team-player
  `fpc_on` marker).
- **Why:** an empty package and no package look the same in game.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I20; `spec/export_upgrader.md` step 3 (the modelless-folder
  rule the referee export shares).

### RD-66: The referee marker is a reserved collar, not a system-file write

- **Red:** on Fox wrote `cup_logo.ftex` into the system file `Data/dt00_x64.cpk` after an
  interactive prompt (pre-Fox: replaced the referee template prop's texture in the refs
  CPK); `ref_marker_needs_consent` and `dt00_write_failed` went with it.
- **Ours:** shows the marker through reserved collar 77 — a prop model (`referee_prop.fmdl`
  on Fox, the template's `referee_prop.model` on pre-Fox) and a texture inside the refs
  CPK; nothing outside the refs CPK is written and no consent/backup is needed.
- **Why:** `dt00_x64.cpk` is loaded over every other CPK, so its own texture is replaced
  only by writing the system file itself — which is what Red did.
- **Evidence:** in-game on PES 17 (2026-10-09): a `.mtl` beside a nocloth `.model` is read
  and stops the collar drawing, so the pre-Fox route keeps Red's template prop; the marker
  shows under each official checked on PES 17 with the cup's referee CPK in the form the
  compiler writes (4.y-ingame2(b)); collar 77 is used by none of the 4,635 kit configs on
  the maintainer's machine (surveyed 2026-10-03).
- **Status:** Checked.
- **Stated in:** `referees.md` "Referee export processing" (the marker paragraphs);
  `messages.md` "Referees".

### RD-67: The AET referee layout is refused, not compiled

- **Red:** compiled the AET referee layout (`face/`, `boots/`, `gloves/`, `common/`
  subfolders inside each referee's folder) as it stood.
- **Ours:** refuses such a folder `player_layout_proto` (E), naming the Export upgrader,
  which flattens the subfolders into the player folder root.
- **Why:** the AET layout types a subfolder's files by the subfolder's name where the
  Studio rule types them by the file's name — compiling it as subfolders would give a
  different output than Red's without a finding.
- **Evidence:** maintainer's ruling 2026-10-10; the referee tracer proves the flattened
  upgrade compiles to Red's output for the unflattened original.
- **Status:** Checked.
- **Stated in:** `spec/aesthetics_export/player_folders.md` "Subfolders";
  `spec/export_upgrader.md` step 12; DECISIONS 2026-10-10 (the reserved-subfolders
  decision); `messages.md` `player_layout_proto`.

### RD-68: A referee `common/` model no reference names is left out

- **Red:** kept it packed in the referee's own common directory, where a model loads only
  when a reference names it.
- **Ours:** reports it as unmigrated content and leaves it out — folded to the player
  folder's root it would become a part of the referee's face.
- **Why:** at the root it would silently become a face part; in the referee's own common
  directory it could only sit unreferenced.
- **Evidence:** real example: `refs for Autumn Q 25 Day 1` referee `galosengen`'s
  `Common/parts_body.model`.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 12; DECISIONS 2026-10-17 item (17).

### RD-69: `refs.txt` + `players.txt` together: `players.txt` wins

- **Red:** its development branch errored and skipped the export; released Red never opens
  a referee export's `players.txt` at all.
- **Ours:** uses `players.txt` and reports `refs_txt_ignored` (I).
- **Why:** the Studio roster grammar is `players.txt`'s.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-11 list and 2026-10-18 premise (2);
  `spec/aesthetics_export/player_folders.md` "Referee exception".

### RD-70: `refs.txt` is read with the strict roster grammar

- **Red:** released Red skips a malformed line, lets the last of a repeated slot win, and
  bounds nothing.
- **Ours:** a malformed line, a repeated slot or a slot outside 01-35 is an Error.
- **Why:** a silently skipped line, or a slot no game loads, is what an Error exists to
  show.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-18 premise (5); `spec/aesthetics_export/player_folders.md`
  "Referee exception".

### RD-71: A referee's `hair_parts` textures move to Common with no copy left behind

- **Red:** left a copy of a referee's `hair_parts` textures beside the face
  (`referee_tools.py:222-229`, a special case for its pre-Fox move of models to Common).
- **Ours:** moves them to the referee's Common output with the rest of his textures; no
  copy remains beside the face.
- **Why:** Red's copy only "preserved originals"; the hair model and `hair.mtl` stay in
  the face folder, so the `.mtl` reads the textures in Common fine.
- **Evidence:** maintainer ruling 2026-10-11, no export check cited.
- **Status:** Not checked.
- **Stated in:** DECISIONS 2026-10-11 "a referee's `hair_parts` textures move to Common
  like any other"; `pipeline.md` step 6; the Red inventory's G1.

### RD-72: The pre-Fox referee layout class

- **Red:** the reference for the whole class.
- **Ours:** class I11 of the library parity attribution: remaining pre-Fox referee-layout
  differences beyond RD-41, RD-64, RD-65, RD-66 and RD-30 (the string-table item).
- **Why:** a class this wide hides its differences: split into one entry per difference.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I11; `referees.md` "Referee export processing".

### RD-73: A refs export's template output and no referee portrait

- **Red:** its referee template output (the referee bins and marker its template wrote)
  took a different shape in the runs the class covers.
- **Ours:** writes the bundled referee template tree (bins, marker) into the refs CPK when
  the refs export commits, and emits no referee portrait.
- **Why:** the template tree is the bundled base for the referee content the export does
  not ship.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** classes.md I21; `referees.md` "Referee export processing";
  `pipeline.md` "5. Writer".

## Upgrade choices (what the Export upgrader migrates where Red's compile did otherwise)

### RD-74: A modelless boots/gloves folder marks the wearer `fpc_on`

- **Red:** packed an empty package under the folder's ID — how a member hid a player's
  boots or hands.
- **Ours:** each player wearing one gets the `fpc_on` marker, which gives him the preset's
  blank-model IDs; no empty package is written, and the folder's files are leftovers.
- **Why:** the maintainer — "Giving a player a boots/gloves ID for an inexistent folder
  will work as fine"; the upgrade can mark the player instead of packing an empty package.
- **Evidence:** maintainer's answer quoted in DECISIONS 2026-10-19; real EGG VGL26
  `Boots/k2176 - EMPTY` and `Gloves/g2177 - EMPTY`, 9 exports in all.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 3; DECISIONS 2026-10-19 "a modelless boots
  or gloves folder marks its wearer `fpc_on`"; classes.md I32.

### RD-75: An unreadable Note kit entry keeps its slot — Red shifted the rest down

- **Red:** dropped the entry and shifted every later kit's colors one slot down, onto
  another kit's config.
- **Ours:** the entry is reported (unmigrated content) and keeps its slot, which gets no
  `colors.txt`; the later entries keep their own slots.
- **Why:** a mistyped color is not repaired — a Warning that let the rest upgrade would
  mostly go unread, so the refusal makes the manager fix the Note.
- **Evidence:** none cited (the companion refusal cites DECISIONS 2026-11-09).
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 5 "Reading the Note".

### RD-76: A config whose slot no Note entry gives is offered anyway

- **Red:** packed the config but, counting the Note's entries, never offered the kit.
- **Ours:** it still becomes its kit folder, reported `config_without_kit_entry` (W) — the
  upgraded export offers the kit.
- **Why:** the author deletes the folder if the kit was dropped; a Warning keeps what it
  names.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 5 "Entries and configs"; DECISIONS
  2026-10-17 item (15).

### RD-77: No slot for a second goalkeeper kit or a player entry past nine

- **Red:** wrote a second/later goalkeeper kit to `UniColor.bin` as kit 0x11 and player
  entries past the ninth as kits past 9.
- **Ours:** they have no Studio slot and are unmigrated content.
- **Why:** no version the cup plays shows a goalkeeper kit past the first.
- **Evidence:** maintainer's answer — "none of the PES versions the cup uses supports
  goalkeeper kits past g1 (negative but keep the road open for it)".
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 5; DECISIONS 2026-10-19 "no second
  goalkeeper kit slot"; classes.md I13 (part).

### RD-78: The Note is read with replacement characters where Red's read raised

- **Red:** read the Note with a strict UTF-8 decoder, which raises on the Shift-JIS bytes
  real Notes carry.
- **Ours:** reads the bytes as UTF-8 with replacement characters; every token the grammar
  reads is ASCII, so the migration proceeds.
- **Why:** real Notes carry Shift-JIS in the lines the grammar does not read.
- **Evidence:** real Notes: JP's Summer 18, `poland`, `nijiura` — the pre-Fox tracer had to
  cut a non-UTF-8 name "which Red cannot read".
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 5 "Reading the Note";
  `tests/fixtures/tracer_prefox/README.md`.

### RD-79: Token-only-differing kit texture copies keep the old compile's pick

- **Red:** `filenames_id_replace` renamed every token to the team's ID in listing order,
  deleting the file under the new name — its pick depended on the team's ID and the
  Windows case-folded name order.
- **Ours:** keeps the one the old compile's rename wrote last (a placeholder-spelled copy
  is renamed whatever the team and lists last, so it wins); with a save the pick is
  reproduced, and in loose or model-folder mode the numbered-only case is
  `unmigrated_content` — which copy shipped depended on the team ID the upgrader then
  cannot read.
- **Why:** the old compile's pick was deterministic, not arbitrary.
- **Evidence:** real inputs: JP Genso Cup `k0860` (an older `u0731p2.dds` beside the newer
  `u0XXXp2.dds` the cup showed) and GBPEN VGL26 `g2336`; the 6.16 census rerun found four
  real inputs refused for this alone; no export holds the numbered-only case.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 7 "Copies differing only in the token";
  DECISIONS 2026-10-20 "a texture the old compile's ID rename replaced is a leftover".

### RD-80: A stray file drops alone — Red discarded the whole folder

- **Red:** discarded a whole folder over one stray or bad file: every `Kit Configs/` config
  when one name breaks the grammar, `Kit Textures/` for one failing texture, `Logo/` for
  more than three emblems, a model folder for `face.mtl.backup` or an FMDL under no name
  the game loads.
- **Ours:** drops the file (`leftover_dropped`, W) and keeps the folder — the member's
  content, which the stray copy never belonged to.
- **Why:** the author can then fix one file rather than lose the team's kits silently.
- **Evidence:** the real CIV4XG VGL26 export lost every kit config to one
  `XXX_DEF_GK2nd_realUni.bin`; 6.12c D6: M Winter 18 and two other exports
  (`face.mtl.backup`, `materials.mtl.bak`); D9: the 4.2.3 test export `C-blin`'s
  `k0326 - IKA-CHAN`.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 10; DECISIONS 2026-10-17 item (12) and
  2026-10-19 "a stray file a folder is refused for is a leftover"; classes.md I33.

### RD-81: A `.dds`+`.ftex` pair of one stem keeps the `.dds` alone

- **Red:** packed both files of the pair.
- **Ours:** drops the `.ftex`, keeps the `.dds` (`texture_duplicate_dropped`, W); when no
  model of the export uses the stem, both are leftovers.
- **Why:** the shared validation refuses the pair (`texture_stem_conflict`), and the
  `.dds` is the one Red's Fox mode itself preferred.
- **Evidence:** real example: /akg/ VGL26's `Common/dummy_srm.dds` + `dummy_srm.ftex`, its
  one refusal.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 10 "A `.dds` and a `.ftex` of one stem";
  DECISIONS 2026-10-19 "a hair skeleton renamed to pair, `.fpk.xml` left out, an `.ftex`
  kit texture converted".

### RD-82: An `unmigrated_content` Error where Red picked silently

- **Red:** kept the first or made an order-dependent pick and compiled the rest — two
  folders of one slot or ID (Red kept the first in directory order), a whole folder under
  a name it refused, a config deeper than one subfolder.
- **Ours:** the entry is `unmigrated_content` (E): two candidates for one place, a
  misnamed whole folder — nothing is written until the author decides.
- **Why:** of two candidates only the author knows the meant one; Red's pick was a refusal
  or arbitrary.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 10 "Unmigrated content"; DECISIONS
  2026-10-19 "where 'never reached the game' stops being a leftover";
  `player_number_duplicate` in DECISIONS 2026-10-11's list.

### RD-83: Stray text inside an old `.mtl` is cleaned

- **Red:** packed the `.mtl` as it was — its parser ignored text between elements.
- **Ours:** the upgrader reads it with `read_dropping_stray_text` (`mtl_stray_text_dropped`,
  I) and the compile emits the cleaned file.
- **Why:** the compiler's reader refuses the stray text (`mtl_broken` would drop the
  folder); every Studio export passes through the upgrader once, so cleaning there keeps
  the compiler strict.
- **Evidence:** real example: G Autumn 25's `72117 - Lisa/apc.mtl` (a `\` after a
  `<sampler/>`), found by the Red parity run; census: 3 of all 12,755 `.mtl` files on the
  machine hold such text.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 7; `spec/libs/format_crates.md` (the tolerant
  reader); DECISIONS 2026-10-19 "the upgrader cleans stray text out of an old `.mtl`".

### RD-84: A custom ID no folder claims becomes the FPC hide preset's ID

- **Red:** the stored custom ID stood in the record and the game drew no model for it
  (members set such IDs on purpose for a player whose face folder carries his whole body).
- **Ours:** writes the FPC hide preset's ID of the kind (`boots_id = 55`,
  `gloves_id = 11`) into `settings.toml`, reported `custom_model_hidden` (W). The preset ID
  shows the same no-model the member's unused number meant.
- **Why:** the member's number would point at whatever Studio's block scheme assigns
  there, not the nothing it meant.
- **Evidence:** VGL26: /skg/ and five more exports set such IDs on purpose.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 8 "Custom IDs no folder claims".

### RD-85: `Other notes:` in any letter case is carried

- **Red:** read `Other Notes:` in that letter case alone, so a real Note's
  `Other notes:` text was lost.
- **Ours:** reads the header in any letter case, as the color sections' headers are read,
  and the section reaches `notes.txt`.
- **Why:** the section is every line after the first `Other Notes:`-spelled line.
- **Evidence:** "a real Note's `Other notes:`" — no export named.
- **Status:** Not checked.
- **Stated in:** `spec/export_upgrader.md` step 6; DECISIONS 2026-10-17 "generation,
  portraits and the CLI settled before the acceptance section" (item 14).

### RD-86: A face whose Common path names the team by name is migrated

- **Red:** discarded the face folder (its check refuses a Common path whose subfolder is
  not the three-character team folder).
- **Ours:** the upgrader keeps the member's `.mtl` and respells the path to
  `team_common_folder/` — the team names (`common/o/`, `common/vg/`, another team's) are
  respelled as the team's Common.
- **Why:** a Studio export names no Common but its own, and the compiler refuses any
  folder after `common/` but `team_folder`.
- **Evidence:** 6.12c D4: /o/ and VG name their team in such paths; 8 exports' faces were
  affected.
- **Status:** Checked.
- **Stated in:** `spec/export_upgrader.md` step 7 (the `.mtl` Common rule); classes.md I34;
  DECISIONS 2026-10-19 "a `.mtl` Common path needs the three-character subfolder too".

## Output, interface and settings

### RD-87: An ambiguous nested export root is refused

- **Red:** `team_id_get.py` took the first matching child of a nested export.
- **Ours:** several usable candidates are ambiguous and reject the export
  (`nested_root_ambiguous`); a loose root file colliding with a flattened file at the same
  virtual path rejects it too (`nested_root_conflict`).
- **Why:** collisions are rejected, never silently resolved — the suite-wide rule.
- **Evidence:** none cited.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` step 1 and "Resolved decisions" (root normalization).

### RD-88: An unresolvable team is a hard error, not a prompt

- **Red:** prompted interactively for an unresolvable team.
- **Ours:** `team_name_unknown` on the CLI; the GUI's team picker is the prompt's
  successor.
- **Why:** no blocking console prompts.
- **Evidence:** none cited.
- **Status:** No census needed: an interactive prompt replaced by a GUI picker or a CLI
  error — interface behavior, not a compiled byte.
- **Stated in:** `pipeline.md` step 3 "Export identity resolution"; `plans/team_compiler/
  gui.md` (the teams-list picker).

### RD-89: No `these_cpks_were_moved_to_download_folder.txt`

- **Red:** wrote a record of the installed CPKs
  (`patches_output/these_cpks_were_moved_to_download_folder.txt`).
- **Ours:** keeps no record of the installed CPKs.
- **Why:** the file has no reader.
- **Evidence:** none cited.
- **Status:** No census needed: a record file no game reads.
- **Stated in:** `pipeline.md` "6. Post-processing" (Deploy CPKs).

### RD-90: The promoted-CPK folder is `output/`, not `patches_output/`

- **Red:** wrote undeployed CPKs to `patches_output/`.
- **Ours:** degrades or promotes staged CPKs to `output/` (reached by degradation or
  `--no-deploy`).
- **Why:** `patches_output/`'s role in its new place.
- **Evidence:** none cited.
- **Status:** No census needed: an output folder's name.
- **Stated in:** `settings.md` `output_folder_path`; `pipeline.md` "Degraded run".

### RD-91: The savefile is written — Red never touched it

- **Red:** never touched the savefile; members edited the records by hand.
- **Ours:** a compile writes `aesthetics_patch.toml` to its output folder with the CPKs it
  publishes (each compiled player's look and IDs, and names on an export marked to write
  names) and applies it to the configured local save.
- **Why:** the aesthetics settings that used to be edited in the savefile by hand now live
  in the export and reach the game at compile time; the DLC builder and the savefile
  builder are different roles, so the patch is a file that can be carried to another
  machine.
- **Evidence:** none cited in the sources read.
- **Status:** Not checked.
- **Stated in:** `pipeline.md` "Resolved decisions" ("The aesthetics patch is the
  compiler's only savefile write path"); `settings.md` `savefile_path`; DECISIONS
  2026-10-11 "Phase 5 savefile writing".

### RD-92: `file_critical_check` cannot happen

- **Red:** had a `file_critical_check` class of missing/mismatched-template errors.
- **Ours:** templates and fallback bins are embedded in the binary (with a `templates/`
  override directory), so the class disappears.
- **Why:** no loose files exist to be missing.
- **Evidence:** none cited.
- **Status:** No census needed: a finding class whose condition cannot occur.
- **Stated in:** `pipeline.md` "Resolved decisions" (templates and fallback bins).

### RD-93: `teamnotes.txt` headers are `--- /xx/ ---`

- **Red:** its header was console decoration (`. `, `- `, `-- {team}'s note file:`).
- **Ours:** one header line `--- /xx/ ---` (the export's team name), then the note with LF
  line ends.
- **Why:** a header that names the team as every message does is enough.
- **Evidence:** none cited.
- **Status:** No census needed: a member-facing text file's decoration, not a game file.
- **Stated in:** `pipeline.md` step 5 "Notes collection"; DECISIONS 2026-10-04
  "`teamnotes.txt`: its layout, when it is written, and its failure".

### RD-94: The whole message surface is new

- **Red:** ~150 console-text messages (near-identical per folder type), progress chatter
  and blocking interactive prompts.
- **Ours:** structured `PipelineEvent` findings — one condition, one code, severities by
  rule — and no console prompts; Red's message texts serve only as a checklist of
  conditions worth reporting.
- **Why:** the messaging is overhauled, not ported; GUI/CLI decide presentation.
- **Evidence:** none cited.
- **Status:** No census needed: the finding surface itself, not a compiled byte.
- **Stated in:** `team_compiler/README.md` (the contract section); `messages.md` head.
