# Unlimited Tool Works — Team compiler plan: Blue features to port

Part of the [Team compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Unimplemented Blue Features to Port

Blue is a prototype missing features that Red has. The Rust rewrite must implement these.

### Referee export processing (Red is the behavioral spec)

Blue explicitly skips referee exports (`coordinator.py:218`); Red's
`Engines/python/lib/referee_tools.py`, together with its referee-related path editing in
`fmdl_editing.py` and export routing in `export_move.py`, is the authoritative behavioral
specification.

**The referee export format changes only in name.** Red's current referee layout — `refs.txt` for
number→name mappings plus per-referee face/boots/gloves/common subfolders — was the prototype of the
player-folder format, tested only on referees. Referee exports adopt the **unified player-folder
format**: a `refs` export contains `Players/` folders exactly like a team's aesthetics export, with shared models
in named shared folders + link files. The per-referee subfolders of the AET layout are not read
by the compiler: a referee folder holding one is refused with `player_layout_proto`, naming the
Export upgrader, which flattens them into the Studio layout (see "Subfolders" in
[`player_folders.md`](../aesthetics_export/player_folders.md), and the upgrader plan's item 12);
`refs.txt` is accepted as a legacy alias of `players.txt` (see "Player numbering"), and the upgrader
renames it too. The flat layout lists a referee's contents more explicitly, and the compiler reading
the AET layout would be a second dialect to author
in.

**How a referee CPK is composed.** Every PES version has **35 referee slots**
(`referee001`–`referee035`), used randomly or pseudorandomly in every match. Filling all 35 with
distinct models is overkill, so in practice **about 8 models are repeated across the slots**, with
each model's repetition count determining how rare or frequent that referee is. Red drives this
through `refs.txt`, which may list the same folder name under multiple slot numbers. The unified
format keeps the mechanism as the required root **`players.txt`** slot mapping (see "Player
numbering" in the export format section): up to 35 slot entries, each naming a player folder, with
the same folder free to appear under any number of slots. The general multi-mapped player rule
performs shared preparation once per distinct source folder and slot instantiation once per listed
referee slot, rendering that slot's `refereeXXX`/`k99XX`/`g99XX` IDs and paths. Referee player
folders therefore carry no numbers in their names — the mapping lives in `players.txt`. In every
other respect a referee folder is a player folder, its `face.xml` included: on pre-Fox its local
boots and gloves are typed entries of that `face.xml` ("A link plus local models combines" in the
[Aesthetics export plan](../aesthetics_export/player_folders.md)), so no `k99XX`/`g99XX` folder is
written for them, and the game, modded to load `k99XX`/`g99XX` for slot XX, shows none there;
Fox writes them as the slot's `k99XX`/`g99XX` folders. A refs export may hold shared `Boots/` and
`Gloves/` folders like a team's, but a referee has no team block to give one an ID of its own: a
referee's link resolves to his slot's `k99XX`/`g99XX`, the shared folder written as that folder
for every slot that links it (on Fox the referee's local parts merged in, as for a player's own
folder). On pre-Fox the link is written there alone, never copied into his `face.xml` as
well: the game loads `k99XX` by slot, and a second copy in the face would dress him twice. A
referee folder with no face model gets no face folder, where a team player's gets a blank one
(the FPC rule, `pipeline.md` step 1): a referee has no FPC body to bring a head, so a blank
face would leave him headless, and the game's own referee head stays.

**The referee body (`refkit`, PES 15-17).** Referee teams are full FPC, so a head-only
referee shows no body unless his face lists one. The referee template tree ships one at
`common/999/refkit/` (Red's), and a refs export's `Common/refkit/` replaces its files of the
same name, the template's others staying (the cup customizes it: the Winter 26 to Summer 26
exports change 12 of its 16 files); every other file of the export's `Common/`, a subfolder's
included, lays over the template's `common/999/` the same way. A member's `face.xml` names
the body with Common paths (`model/character/uniform/common/999/refkit/oral_thigh_*.model`),
or, more simply, the referee folder holds `fpc_off` ("this player needs his body",
`aesthetics_export/fpc_toggle.md`): his face's xml, generated, his own or a linked shared
face's, then gets these entries appended, before the `face_neck` dummy and the `<dif>`, each
with `material` `model/character/uniform/common/999/refkit/refkit.mtl` (the list a cup
referee's own xml used in game, Winter 26's `tsuoffside`):

| `path` (`model/character/uniform/common/999/refkit/` +) | `type` |
|---|---|
| `oral_arm_*.model`, `oral_thigh_*.model`, `oral_refshirt_*.model`, `oral_pants_*.model`, `oral_pants_sub_*.model`, `oral_sleeve_*.model`, `oral_socks_*.model` | `parts` |
| `oral_hand_l_*.model` | `gloveL` |
| `oral_hand_r_*.model` | `gloveR` |
| `oral_boots_*.model` | `boots` |

An entry whose model the xml already names is left out, and so are the boots when he has
boots of his own (a model or a link) and the hands when he has gloves, so he is never dressed
twice; a partial body (his own shirt over the refkit's legs) is written by hand in his own
xml, without `fpc_off`. A referee folder with no face model gets no face, so `fpc_off` adds
nothing there. On PES 18-21 the body waits for a Fox refkit the maintainer will make (worklog
"Issues").

What carries over from Red as compiler-internal behavior (invisible in the format):
- `ExportIdentity::Referees`, rendered as fixed numeric ID 999 only inside game paths/templates (999
  is not a normal `TeamId`); per-referee IDs generated as `refereeXXX` / `k99XX` / `g99XX`
- Per-referee common structures and XML/MTL/FMDL path updates — no longer referee-specific: the
  standard texture relocation step moves **every** player's textures to a per-player common
  subfolder (see `pipeline.md` "Per-model-folder parallel steps"), which is released Red's preprocessing for referees
  generalized to every player (its unreleased development branch had begun the same for
  Pre-Studio `Players/` folders)
- Referee base template content (`refscpk` templates)

What does not carry over is how Red showed the referee marker (`ref_marker.dds`): pre-Fox it
replaced a texture of the referee template (`parts/referee/incom_bsm.dds` in the refs CPK), and
on Fox it wrote `cup_logo.ftex` into the system file `Data/dt00_x64.cpk` after an interactive
prompt. Fox shows the marker through a **reserved collar** instead; pre-Fox keeps Red's route,
the template prop's texture replaced (in game on PES 17, 2026-10-09: a `.mtl` beside a nocloth
`.model` is read, and the collar model then stops drawing, so a collar cannot carry a texture
of its own there; and a collar drawn with the kit's own material has no alpha test, so it could
not show a logo with transparency anyway). The Fox prop cannot be reached from the refs CPK:
`dt00_x64.cpk` holds the shaders and is loaded over every other CPK, `download/` ones included,
so a texture of its own is replaced only by writing `dt00` itself, which is what Red did. Either way nothing outside the refs CPK is ever written and no consent,
backup or rollback is needed:

- stock collar **77** is reserved for the referees, as 105 is for FPC (a stock collar of every
  target version, and one that none of the 4,635 kit configs on the maintainer's machine uses,
  Konami's and the cup's alike, surveyed 2026-10-03). A regular team may not use it: a `Collars/` file claiming it reports `collar_id_conflict`, and a team kit whose
  effective collar or winter collar (after FPC reconciliation and custom-collar rewriting) is
  the reserved one reports `kit_collar_reserved` and is dropped, since its players would wear
  the referees' marker;
- on Fox the refs CPK carries the marker model as that collar, at the collar's `nocloth` path
  ("Game paths reference" in `pipeline.md`): `collar_077.fmdl`, the file a Fox referee loads,
  bundled with the compiler like the other referee templates. On pre-Fox the refs CPK carries
  the marker texture as the template tree's `parts/referee/incom_bsm.dds`, the texture of the
  prop model the game draws by itself (the tree's `referee_prop.model`, a flat square bound to
  `judge_incom`), in place of the tree's file: no collar pair, and the template configs keep
  their collar 26. A pre-Fox referee draws `referee_collar_<ID>.model` and needs
  `collar_<ID>.model` to exist (in game, 2026-10-08), but a `.mtl` beside that model stops it
  drawing (in game, 2026-10-09, with the prop's material set and with the kit's), so a collar
  cannot carry the marker there. The 77 reservation holds on both engines all the same, one
  rule: on pre-Fox it costs a team one stock collar ID for nothing, cheaper than an engine case
  in two findings;
- the model's texture path names the marker texture in the referees' Common output, which is
  `ref_marker.dds` converted like any Common texture (the bundled Fox model's base texture,
  `common/000/sourceimages/cup_logo.dds`, is pointed at `common/999/sourceimages/ref_marker.dds`;
  its normal and specular maps stay the game's dummies);
- the referee template kit configs name that collar, as collar and winter collar, a
  replacement from the data directory's `templates/referees_fox/` included (nothing else in
  them changes). They are loose files under
  `common/character0/model/character/uniform/team/referee/` in the refs CPK on both engines,
  as the templates ship them (`referee_DEF_1.bin` and the rest). On Fox each is also written
  as the entry of its name, the file name with `.bin`, in the bins CPK's `UniformParameter.bin`,
  inserted by name like a team's: the bundled bases hold the `ACL` and `DEF` ones (PES 18's
  the `CL` ones too) at collar 105, which the entry written replaces, and no `LB` or `SDA`
  entry, which is added. The game needs the loose file but reads the values from the entry,
  as it does for a team's kit configs (in-game tests: a collar changed in an entry was worn).
  Loose files alone leave the referees on the entries' collar 105.

Without `ref_marker.dds` the collar model and the texture are not emitted, and the template
configs keep the collar they had. A marker that its texture checks drop or whose conversion
fails counts as absent, reported by its texture code: the collar and the texture stand or fall
together, so the referees never wear a collar that names a texture the CPK lacks.

The marker model is the one the legacy tools ship as a referee prop: a square about 1.5 m
wide lying on the ground, slightly tilted, under the referee. On Fox it is the 4cc's
`referee_prop.fmdl` (one mesh, 4 vertices, material `judge_watch`, base texture
`cup_logo.dds`), on pre-Fox the `referee_prop.model` of Red's referee template (material
`judge_incom`, its `.mtl` naming `./incom_bsm.dds`), which the pre-Fox template tree carries
under `parts/referee/` and the game draws by itself: the marker texture goes out as that
`incom_bsm.dds`, converted like a Common texture, and nothing else changes (the tree's own file
is WESYS-compressed, 907 KB for a 5.6 MB DDS; the compiler writes a plain DDS in its place, as
it writes every pre-Fox texture and as Red did there). Drawn as a Fox
collar it stays on the ground instead of following the neck by **static painting**, the cup
community's trick: its vertices are weighted to a dummy vertex group, usually named `static`,
that is no bone of the body skeleton. The marker shows on the ground under each official in every scene
(checked on PES 17 by the maintainer with the cup's referee CPK, in the form the compiler
writes, worklog 4.y-ingame2 (b)); why the lead's harness runs with a test texture showed none
is not known.

### Features that disappear in a compiled GUI app

These Red features are consequences of being a distributed Python script and are eliminated by the
Rust + GUI architecture:

| Feature | Red location | Why it disappears |
|---------|-------------|-------------------|
| Self-healing / dependency check | `dependency_check.py`, `file_management.py` | Single binary with embedded templates/fallback bins (see `pipeline.md` "Resolved decisions") — no missing files or packages |
| Admin privilege elevation | `admin_tools.py` | The shared `elevation` lib (manifest execution level + elevated relaunch) |
| Step-by-step run modes | `compiler_main.py:72-93` | CLI subcommands for automation |
| `pes_uniparam_edit` standalone tool | `pes_uniparam_edit.py` | Covered by `uniparam` format parser |

### Size deltas in shared modules

Red's shared modules are larger than Blue's due to referee-related edge cases and additional
validation:

| Module | Red | Blue | Delta |
|--------|-----|------|-------|
| `export_move.py` | 31KB | 23KB | 8KB (referee path logic) |
| `bins_update.py` | 18KB | 12KB | 6KB (more bin-packing logic) |
| `fmdl_editing.py` | 19KB | 12KB | 7KB (`fmdl_texture_paths_change` for referees) |
| `xml_editing.py` | 22.2KB | 14.8KB | 7.4KB (more template manipulation) |
| `export_check.py` | 44KB | 41KB | 3KB (minor) |

These deltas represent logic that must be accounted for in the Rust port. Use Red as the reference
for completeness.

---
