# Unlimited Tool Works — Aesthetics export plan: Player folders

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Player folders (the Studio export format)

**This is a primary motivation for the project.** The export format itself is new: it is called the
**Studio export format** and was designed for Unlimited Tool Works — the Team compiler is its only consumer,
and nothing writes it yet. A "player folder" holds all of a player's models —
face, boots, and gloves — in a single folder, without the old split into separate Faces/Boots/Gloves
item folders. (Red's referee export layout was the prototype of this idea, tested only on referees;
its per-referee face/boots/gloves/common subfolders are not kept, the Export upgrader flattens
them ("Subfolders" below), and the unified format is used by team and referee exports alike — see
"Referee export processing" in the [Team compiler plan](../team_compiler/README.md).)

```
Players/
├── 15 - Snuffy/
│   ├── face_high.fmdl        (or .model, or .glb — see the Unified model format plan)
│   ├── hair_high.fmdl
│   ├── oral.fmdl
│   ├── boots.fmdl            (player-exclusive boots)
│   ├── Crocs.boots           (link file: references Boots/Crocs/, shared with other players)
│   ├── glove_l.fmdl
│   ├── glove_r.fmdl
│   ├── *.dds / *.ftex / *.png / *.jpg / *.bmp / *.webp / *.tga / *.tiff
│   │                         (textures: any image format, referenced by stem; interchangeable)
│   ├── materials.toml        (glTF models only: PES material properties keyed by material name;
│   │                          the catch-all applies to all glTFs in the folder. Optional
│   │                          name-matched *.materials.toml files (e.g. body.materials.toml)
│   │                          override per-model, mirroring the .model/.mtl system; a
│   │                          *.materials.toml.common or *.mtl.common link pulls the file of
│   │                          that name from Common/ — see the Unified model format plan)
│   ├── ingame_face.toml      (the in-game face; marks the folder)
│   ├── fpc_on                (optional empty marker: apply the FPC preset at compile time; see `fpc_toggle.md` "FPC toggle")
│   ├── settings.toml         (the player's settings: on Fox they compile into the CPK's database tables, on pre-Fox into the savefile; see below)
│   ├── jessie/  shorts/  ...
│   │                         (optional subfolders of any name and depth: each is a player folder
│   │                          of its own, merged into this one; see "Subfolders")
│   └── ...
└── 16 - Another Player/
    └── ...
```

**Player numbering** — a normal team player folder gets its number(s) in one of two ways:

1. **In the folder name**: `NN - Name` with NN in 01–23, as above.
2. **Via a root `players.txt`**: lines of `NN <folder name>`; the listed folders then carry no
   number in their names. Sparse lists are allowed, and the same folder may be listed under multiple
   team slots. When `players.txt` is present it is authoritative: every player folder must be listed
   in it, and folder-name numbers are not used.

**Savefile identity.** Normal 4cc team rosters and player IDs are aligned: slot `NN` belongs to
player ID `team_id * 100 + NN`. Resolve that record by ID, not by name or physical file offset;
model/portrait destinations and savefile edits use the same identity. For team 701, slot 03 targets
70103 even if its savefile name is generic. A folder mapped to several slots applies to each slot's
corresponding ID. Names may be written through the opt-in `name` setting, but never select the
record; a missing record is not replaced by a name-based match.

**Multi-mapped processing.** For team and referee identities alike, shared preparation (parse,
convert, and merge) runs once per distinct source player folder; the results are then rendered and
packed per mapped slot (IDs, paths, portraits, savefile entries), while the folder's relocated
textures are emitted once into their name-keyed common subfolder shared by all mapped slots (see
"Texture relocation to common" in the [Team compiler plan](../team_compiler/README.md)). Both stages commit as one atomic unit, so a partial set of slot
instances is never written. Referees use this same mechanism with their `refereeXXX`/`k99XX`/`g99XX`
rendering.

**Referee exception:** a refs export requires root `players.txt`; numbered referee folder names are
not a supported substitute. Its valid slots are 01–35, and repeating a folder under several slots is
how the export controls referee rarity. The same filename serves both identities, and the export
name's first word being `refs` is the referee discriminant. For legacy Red-era referee exports
only, a root **`refs.txt`** is accepted as a read-only alias of `players.txt` (identical grammar);
when both are present `players.txt` is authoritative and the alias is ignored with `refs_txt_ignored`.
Studio never writes the alias: the Refs arranger and the Export upgrader emit `players.txt` and
remove a `refs.txt` they read from. A refs export may also carry a root **`ref_lists.txt`** — the
referee lists the Fox referee hook applies per match (grammar and purpose in the [Refs arranger
plan](../../plans/refs_arranger.md), "The lists file"). It is the arranger's file: the export model lists it
as a known root file so validation does not warn about it, and the compiler neither reads nor
packs it — it travels to the hook's folder beside the compiled CPK by hand.

**`players.txt` grammar.** Input is strict UTF-8 with an optional UTF-8 BOM; LF and CRLF are
accepted, blank lines are ignored, and comments are not supported. Every nonblank line starts with a
decimal slot followed by whitespace; the trimmed remainder of that line is the complete folder name.
Folder lookup is case-insensitive, but case/Unicode-colliding player folder names are rejected
rather than selected arbitrarily. Writers emit two-digit slots as canonical UTF-8/LF with one
trailing newline. The Refs arranger and Export upgrader read and write this same grammar. A referee
roster must retain at least one valid normalized assignment; an empty file or a file whose entries
all drop reports `players_txt_invalid` and drops the export. An empty authoritative normal-team
roster is allowed for a kit-only export; any present player folders are then unlisted and follow
`player_unlisted`.

**`ingame_face` marker.** `ingame_face.toml` is the marker: its presence does all the marker did,
its content sets the in-game face, and an empty file is the game's default face
(`settings_toml.md` "The defaults are the game's default face"), the old bare marker's successor.
The old spellings, bare `ingame_face` and the tolerated Notepad form `ingame_face.txt`, are
recognized only to be reported `ingame_face_marker_renamed` (E, the file): they mark nothing and
the folder compiles as one without the file; `ingame_face.toml.txt` is not recognized, as
`settings.toml.txt` is not. The file is a settings-stage input like `settings.toml`: held as
`PlayerFolder.ingame_face` (`Option<FileDescriptor>`), read by the deep pass, its findings
reported by `check` and `compile` alike, and never reported as a disallowed file.

It holds the in-game face's keys because the game draws them on the in-game face alone: measured
on 2026-10-24 in PES 21 and PES 17 Edit mode (a face model without a hair model, the same with
one, and the marker, each with long red hair and a blue or violet iris set), beside a face model
the game draws neither the in-game hair nor the record's iris, the model's own eyes standing
(1,186 of the machine's 1,472 Fox face models carry no eye material of their own; the PES 17
sample paints its eyes into its texture); the marker's player draws both. The face sliders and
feature types shape the in-game face's own mesh and textures. So 19 players in 20, who wear a
face model, carried 90 keys that change nothing, in the file a member reads first; a key that
changes nothing reads as a bug when it is set and nothing happens. A player without the file gets
the defaults for them: on Fox his row's face, hair and iris fields hold the game's default face
(not drawn, his face model standing), as an absent key does; on pre-Fox the settings reach
nothing yet, the patch's pre-Fox half being the plan's (`plans/pes_savefile/operations.md`
"Aesthetics patch (new)").

**Findings.** A file that is not UTF-8 or fails to parse, an unknown key included, is
`ingame_face_toml_invalid` (E, the file and the error): its keys are ignored, as
`settings_toml_invalid` ignores its file's keys, and the player keeps the in-game face at the
defaults, the file itself still marking it. A value out of the target's range is
`settings_value_out_of_range`, as in `settings.toml`. A moved key left in `settings.toml` is an
unknown key there, so `settings_toml_invalid`, its message naming the key; the help says where
the face keys went.

At categorization the marker removes face-classified parts and face-specific ancillary files
(each reported as `face_file_not_used`), and **no face folder is emitted** (on either engine — a
face folder containing only boots/gloves would override the ingame face, and an empty one would
blank it: the cup keeps every player's "modified face" flag off, so the game loads a face folder
whenever one exists for the player's ID, whatever the save says).

The face-classified parts split into two groups with different fates:

- **Explicitly-named face models** (`face_high`, `hair_high`, `oral`) have no skeleton slot and
  belong only in a face folder — there is none under `ingame_face.toml`, and `fcl_hair` cannot
  absorb them (it is a single body-skeleton model, not a container for the typed face set). Their
  presence combined with `ingame_face.toml` is a hard error that drops the whole player folder
  (`ingame_face_explicit_face_model`).
- **Arbitrarily-named models** (those that would route to `fcl_hair`, which — like `boots` —
  supports the full body skeleton) **reroute to the boots folder** instead of being dropped:
  renamed to `boots`, or merged into one `boots` if several candidates are present (Fox: `fmdl`
  mesh merging; pre-Fox: `pes_model`'s native merge over the `.model` + `.mtl` pair), and carrying their paired SKL into
  the `boots.skl` slot (see "SKL pairing"). This generalizes the existing `ingame_face.toml` boots
  relocation to the body-skeleton face content that would otherwise be lost.

Remaining gloves parts are relocated to **player-specific gloves folders** (`glove_l`, `glove_r`),
and the rerouted arbitrary-named models plus any explicit boots models form the **player-specific
boots folder** — the one case where pre-Fox produces player-exclusive boots/gloves folders with IDs
from the per-team block scheme and savefile writes. Without `ingame_face.toml`, a player with
boots/gloves but no face models keeps a blank face folder (the FPC requirement — see the pipeline
walkthrough).

**`ingame_face` with shared links.** A boots/gloves link with no local parts of that category
resolves as usual: on either engine the player simply wears the shared folder's ID, since nothing
needs a face folder. A link **combined with local parts** of the same category is where pre-Fox
must deviate from its no-merge rule: normally the shared folder loads by ID while the local parts
ride in the face XML, but under `ingame_face.toml` there is no face XML, and relocating the local
parts to a player-exclusive folder would leave the player with two candidate IDs (the shared
folder's and the exclusive one) for a single savefile slot. Pre-Fox therefore behaves like Fox's `link_combined`
here: the linked shared model is one more input of the player-exclusive merge (`pes_model`'s
native merge, like the multiple-boots case above), the exclusive ID wins and is written to
the savefile, and the shared folder is left untouched for players who link it plainly. Gloves
combine without a merge on pre-Fox: the exclusive gloves folder is written as a shared gloves
folder is (each model under its own name with its `glove.xml` entry), the linked folder's models
joining the player's as more entries and a local model whose output name a linked one shares
replacing it. `glove.xml` lists any number of entries, so the game loads every part as it is,
while a merge would add conflicts to fix for nothing and could not combine a `handL` part with a
`gloveL` one, whose types differ. A `.model.common`, `.fmdl.common` or `.mtl.common` link under
`ingame_face.toml` follows the same rule: with no `face.xml` to name the Common path, the Common
model is one more part of his boots or one more model of his gloves, copied in under its own
name as a combined folder's would be (a Common FMDL converted into the part, as his own FMDL
parts are), and a Common `.mtl` a part of his uses is copied in with it (its textures stay in
the team's Common output, which the copy names). A **face** link under
`ingame_face.toml` is contradictory (the marker suppresses the face folder the link would fill)
and drops the player folder with `ingame_face_explicit_face_model`, exactly like explicitly-named
local face models.

**Subfolders.** A subfolder of a player folder, of any name and at any depth, is a player folder
of its own whose output is merged into the root player's: its files take their roles from their
names exactly as the root's files do (`jessie/hair_high.fmdl` is a face model, `jessie/body/boots.model`
a boots model, `shorts/shorts.dds` a texture), and its face, boots and gloves parts merge into the
root player's packages as a combined shared folder's do (`link_combined`'s rule, "A link plus
local models combines"). The AET-era exports keep a member's parts in folders of the author's own
choosing (one VTL9 export holds 23 players with 688 files nested three deep), and Red compiles
them in place; a rule that admitted only a flat folder, or only a few reserved names, would drop
that content as `file_type_disallowed`. One rule, "a subfolder is a player folder", covers every
such tree with nothing to name.

- **Paths are kept.** On PES 15-17 the face CPK and the player's texture home hold each file at
  its path below the player folder, the model under its packed name (`jessie/body/x.model` is
  packed as `jessie/body/oral_x_win32.model`, `jessie/body/x.mtl` and `jessie/skin.dds` as they
  are), as Red packs the tree (its `model_names_fix` renames in every subfolder, and its xml
  names `./jessie/body/oral_x_*.model`). On PES 18-21 a subfolder's models merge into the
  player's packages and its textures go under the texture home at their directory, then
  `sourceimages/#windx11/` (`jessie/sourceimages/#windx11/skin.ftex`, the FMDL naming
  `jessie/sourceimages/skin`), the layout Red gives a `Common/` subfolder and a linked
  `Common/` subfolder file has here too (`team_compiler/pipeline.md` step 6); PES 21 draws a
  player's subfolder texture there, checked in game). A `face.xml` or
  `.mtl` reference that carries a path (`./jessie/body/oral_x_*.model`, `./shorts/y.dds`),
  or an FMDL texture whose directory starts with `./` (`./shorts/` and `y`),
  resolves as written, relative to the referencing file: `./sub/name` is a local reference,
  checked like `./name` (`spec/team_compiler/messages.md` "User-supplied `face.xml`"), not
  `xml_path_unchecked`; `./name` in a `face.xml` names a file of the xml's own folder alone (a `.mtl`'s `./name`
  is a bare stem, nearest first). The generated
  `face.xml` lists a subfolder's models by their path, and names each one's `.mtl` where the
  search found it (`./jessie/body/x.mtl` beside the model, `./x.mtl` in the root).
- **The per-player singletons are the root's.** A `face.xml`, `face_diff.bin`, `face_diff.xml`
  or `fcl_hair_sim.fclo` counts directly in the player folder alone, as the markers do: a player
  has one of each, and two from different folders would have no merge. Inside a subfolder each
  is `file_type_disallowed`.
- **A texture name resolves nearest first.** A texture named with no path resolves in the model's
  own folder first, then in each parent up to the player's root, nearest first, never down;
  one named with a path below the model's folder (`./textures/skin`) resolves at that path,
  which is how a member sorts textures into subfolders beside a model above them. Two models in
  different subfolders may each have a `skin.dds` of their own: the lookup namespace of
  `texture_stem_conflict` is one folder, not the player folder with its subfolders, and a
  collision is solved by keeping a texture beside the models that use it.
- **Only the root's markers and settings count.** `ingame_face.toml`, `fpc_on`, `fpc_off` and
  `settings.toml` are read directly in the player folder alone; inside a subfolder each is
  `file_type_disallowed`, an old-spelling marker the same.
- **Shared links count at any depth.** A shared link (`.face`, `.boots`, `.gloves`) below a
  subfolder combines with the tree as a root link combines with local models, and the
  one-per-category rule (`shared_link_duplicate`) counts the whole tree, two links naming one
  shared folder counting once.
- **A `.common` link mirrors `Common/`'s tree.** A `.common` link stands for the `Common/`
  file at its own path below the folder: `kitN/armor_bsm.dds.common` for
  `Common/kitN/armor_bsm.dds`, `jessie/body.fmdl.common` for `Common/jessie/body.fmdl`, a root
  link for a direct file. A model names the same path whether its texture is local or linked
  (`./kitN/armor_bsm`), so sharing a texture is replacing it by a link of its name in its
  place, and a player's `kitN/` folder of links reaches a per-kit set in `Common/`. This is
  also the xml-less way to what a pre-Fox `face.xml` does by naming
  `.../common/<team>/jessie/oral_body_*.model`, and on PES 18-21, which has no xml, the only
  way; a `Common/` subfolder file no link names and no linked model or `.mtl` brings is
  `file_not_used` there
  (`team_compiler/pipeline.md` step 6). A player folder moved into another keeps its files'
  roles but not its links' targets (they then name `Common/<its name>/...`): such composition
  is not a goal the link rule serves.
- **A shared folder's tree is a player folder's.** A shared folder (`Faces/`, `Boots/`,
  `Gloves/`) may hold subfolders under the same rules: their models are parts of its package,
  named as its direct ones are (a boots or gloves folder's say so by suffix at any depth,
  `extra/x_boots.fmdl`), its textures keep their paths and resolve nearest first, and its
  `.common` links count at any depth. It holds no shared link, marker or settings, as before,
  and its singletons (`face.xml`, the face diffs, `fcl_hair_sim.fclo`) count directly in it
  alone, `file_type_disallowed` below a subfolder.
- **No reserved names.** The names `face`, `boots`, `gloves` and `common` are plain subfolder names
  in a team export. They exist only in the AET referee layout, Red's prototype of the player
  folder (one subfolder per category inside each referee's folder), which the Export upgrader
  flattens (its item 12); a referee export in the Studio layout never has them. A referee export's
  player folder with a direct `face/`, `boots/`, `gloves/` or `common/` subfolder is therefore
  that layout, and the compiler refuses the folder with `player_layout_proto`, naming the
  upgrader, rather than compile the tree as subfolders and give the referee a different output
  than Red's (the AET layout types a subfolder's files by the subfolder's name, this rule by the
  file's name). In a team export the same names stay plain subfolders: the census found an
  AET-era team folder whose `face.xml` names its `boots/` as an ordinary part folder.
- Root normalization never treats a subfolder as a nested export root, and `Players/` detection
  is unaffected.

**Shared models** live in Faces/Boots/Gloves folders identified by **name only** (no embedded IDs —
player folders are the main reference point for each player). A player folder references a shared
folder with an empty link file named after it (`Crocs.boots`, `Longhair.face`, `Keeper
gloves.gloves`). A stray `.txt` suffix (`Crocs.boots.txt`) is accepted silently — Windows hides
known extensions by default, so users creating link files with Notepad often produce one. At most
one shared link per category (face, boots, and gloves) may appear in a player folder; multiple links
of any same kind are rejected. Multiple shared-face bases are ambiguous pre-Fox because copied trees
can collide, and unnecessary on Fox because one shared base can already merge with all local parts.
Fox merging therefore combines that one shared base with local parts, never several same-kind shared
links:

```
Boots/
└── Crocs/                    (shared boots, referenced by two+ players via link files)
    ├── boots.fmdl
    └── *.dds
```

**A link plus local models combines** — for every category, so there is no link-versus-model
conflict anywhere. The shared folder acts as a **reusable base** (a body, a head, a boot shape) and
the player's local models are parts layered over it. How that is realized differs by engine, because
pre-Fox is far more permissive:

- **Pre-Fox**: everything in a player folder that isn't a boots or gloves link file becomes part of
  that player's **face folder**. The `face.xml` has a per-entry model *type* property, so boots and
  gloves models load into their own skeletons from the face folder — meaning per-player boots/gloves
  folders are never needed, and nothing is ever merged. A shared face folder is copied per player
  and receives the local files on top (a local file replaces the shared folder's file that
  packs under the same name, model or `.mtl`, as a copy would; `link_combined` reports the
  link, as on Fox); a boots/gloves link keeps loading its shared folder by ID
  while the player's local parts load from the face folder alongside it.
- **Fox**: there is no model type property, so per-player boots and gloves folders *are* required.
  Local boots/gloves models form a **new player-exclusive folder** with its own ID from the team's
  block, and a shared model referenced by a link becomes just another component of it, merged in
  (see below). A face link brings the shared folder's files in under his own roles, so a
  `Faces/` folder's boots- or glove-named model is a part of his own boots or gloves package, as
  his own would be. The shared folder is left untouched for the players that link it plainly. A shared
  **face** link with no local face parts is the same mechanism one part deep: the shared face model
  becomes the player's face FMDL outright (a merge of one), since shared face folders take no ID and
  have no independent output to be left untouched. The face's other files (`face_diff.bin`,
  `fcl_hair_sim.fclo`) are not parts to merge: one the player folder holds wins over the shared
  folder's, the player's files being layered over the base.

Combining is reported as `link_combined`. Consequences for Fox ID assignment: a player who combines
gets their deterministic player-exclusive ID rather than the shared folder's, and a shared
boots/gloves folder that every referencing player combines is only a source of parts — it needs no
folder of its own in the output and consumes no ID from the scarce shared pool (it is still
"referenced", so not `shared_folder_orphaned`). Pre-Fox only ever assigns IDs to shared boots/gloves
folders, since player-exclusive ones don't exist there — except under `ingame_face.toml`, where
the relocated gloves/boots parts form player-exclusive folders with their own IDs (see "ingame_face
marker").

**Common model links and model merging** — a model can be loaded from the export's `Common` folder
instead of shipping in the player folder: an empty link file named after the Common model plus a
`.common` extension (a stray `.txt` is accepted, as with shared-folder links) — Red's pre-Fox
`.model.common` link convention, extended to the Studio format's model file names. On pre-Fox targets
the link resolves to a real runtime reference: the generated XML points at the Common path (on PES16
via the patched exe — see the note under the XML/MTL checks). Fox engines cannot load models from
Common at all, so there the compiler **bakes the link away: the Common model's meshes are merged
into the player's output FMDL** at compile time via the `fmdl` crate.

The same link convention applies to **material definition files**: `body.mtl.common` and
`body.materials.toml.common` pull `Common/body.mtl` / `Common/body.materials.toml` into the player
folder's material resolution as if they were local files of that stem (Red's `find_mtl_file`
already resolves `.mtl.common` names). A Common-linked model needs no such link — its material files
resolve in Common first, with the player folder's matching files layered on top as overrides. And
to **textures**: `hair.png.common` stands in for `Common/hair.png` under the stem `hair`, for
auto-detection and explicit stems in material files and for native `.mtl`/FMDL references alike —
the way to share one texture among many players. A texture that resolves into Common is packed once
in the team's Common output and referenced there; one that resolves in the player folder is the
player's own and travels with it. Rules, same-stem layering, texture-stem resolution and the
packing rule are in the [Unified model format plan](../model_format.md)'s "Link files"; the pre-Fox
XML points at the Common MTL path when the MTL was found in or linked from Common, as Red does.

Merging is driven by name resolution. Red's prefix convention (a prefixed allowed name is renamed to
it: `kit_boots.fmdl` → `boots.fmdl`) extends from pure renaming to multi-part assembly: **all models
— local files, shared-folder parts, `.common` links, or a mix — that resolve to the same allowed
output name are merged into that one model**, in alphabetical source-name order (deterministic
recompiles). `torso_fcl_hair.fmdl` + `legs_fcl_hair.fmdl.common` → one `fcl_hair.fmdl`; a `.common`
link with no local counterpart bakes the shared model in unchanged — the Fox substitute for pre-Fox
common-model sharing. Parts may be in any supported source format (converted to FMDL first, then
merged via the `fmdl` crate's mesh merging).

**Merging is Fox-only.** Fox model sets are closed — a face is a fixed list of FMDL names, and boots
and gloves are separate ID'd folders with one model each (`boots`, `glove_l`, `glove_r`) — so every
extra part has to be merged into one of those. Pre-Fox needs none of it: the typed `face.xml`
absorbs any number of entries of any type, and `.common` links stay runtime references. **Exception
— `ingame_face.toml`**: with no face folder emitted, non-face parts are relocated to
player-specific folders; multiple boots models are merged into one (Fox: `fmdl` mesh merging;
pre-Fox: `pes_model::ops::merge`, the native counterpart with the same rules over the `.model` +
`.mtl` pair — see "`pes_model::ops::merge`" in the Libraries plan — the one pre-Fox merge case),
and gloves go to `glove_l`/`glove_r` folders on Fox and to one player-exclusive gloves folder,
unmerged, on pre-Fox (see "ingame_face marker").

**Model names: a free part plus a suffix.** A model file name is `<anything>_<suffix>` (or just
`<suffix>`), and the **suffix is always at the end** — one rule for every source format and both
engines. The suffix says what the model is; the compiler derives the Fox destination and the
pre-Fox `face.xml` type from it through one table:

| Suffix | Fox destination | Pre-Fox `face.xml` type |
|---|---|---|
| `face_high`, `hair_high`, `oral`, `fcl_hair` | that FMDL (merge if several) | `face_neck` for `face_high` (wider than Red, which gives it only to a file named exactly `face_high_win32.model`); `parts` otherwise |
| `boots` | the `boots` folder (`boots.fmdl`, merge if several) | `parts` (boots use the body skeleton; today's typing) |
| `glove_l` / `gloveL` | the gloves folder, `glove_l.fmdl` | `gloveL` |
| `glove_r` / `gloveR` | the gloves folder, `glove_r.fmdl` | `gloveR` |
| `handL` / `handR` | the gloves folder (`glove_l`/`glove_r` — hand-skeleton models have nowhere else to go on Fox) | `handL` / `handR` |
| `uniform`, `shirt`, `pants_nocloth`, `eye`, `mouth`, `face_neck`, `parts` | face: the `fcl_hair.fmdl` merge | as named (`uniform` → `uniform_sub` on PES15, Red's rule) |
| `model_type_<x>` | face: the `fcl_hair.fmdl` merge | `<x>` verbatim — the escape hatch for a type this table does not know |
| *(none)* | face: the `fcl_hair.fmdl` merge (`fmdl_fcl_hair_fallback` reports each routed file) | `parts` (Red's default) |

The first column's Fox allowed names and the pre-Fox type names are both native vocabularies, so
both are accepted; `glove_l`/`gloveL` and `glove_r`/`gloveR` are aliases of each other. Matching is
case-insensitive and ignores underscores inside the suffix, as Red's typing did. A `_ratio_<n>` token
anywhere in the free part still sets the `face.xml` entry's `ratio` attribute (Red's convention).
Pre-Fox entries additionally get the `oral_`/`_win32` affixes on the emitted file name (a PES 16
loading requirement). The type column applies **only when targeting pre-Fox and only when the
compiler generates the `face.xml`**: a folder that ships its own xml carries the types there, and its
file names are not interpreted for typing. A user-written `face.xml` is accepted as a second-class
path — the way to experiment with what the pre-Fox engines can do beyond this table — and is checked
by the compiler with errors for what is known to break and warnings for everything outside the
vocabulary it generates itself ("User-supplied `face.xml`" in the [Team compiler
plan](../team_compiler/README.md)). Fox has nothing resembling designable model types, so on
Fox a type maps to a destination and nothing more. glTF
models follow the same table — the type lives in the file name, not in the glTF or the material
toml, so a `.glb` authored once compiles typed on pre-Fox and merged on Fox.

**This inverts Red's typing, which matched types as prefixes** (`gloveL_foo.model`); the Studio
format matches them as suffixes (`foo_gloveL.model`), the same position the Fox allowed names already
occupied (`kit_boots.fmdl`), so there is one place to look. The Export upgrader swaps them (see its
plan). "Arbitrary model names are face content" is the last row: the generalization of Red's
fcl_hair fallback (which renamed a single arbitrary-named face FMDL to `fcl_hair.fmdl`) — a model
with no recognized suffix is a face part on both engines, which is what makes the plain-named case
work with no naming ceremony: `torso.fmdl` + `legs.fmdl.common` → one `fcl_hair.fmdl` on Fox, two
`parts` entries pre-Fox. Boots and gloves models must therefore *say so* by suffix — in a player
folder because an unsuffixed model would be taken for face content, and in a shared boots/gloves
folder, where nothing can be a face, unsuffixed names are `fmdl_name_invalid` errors
(`model_name_invalid` on a pre-Fox target, the same rule under the `.model` catalog's code).
On PES 18-21, where a shared folder's `.common` links are read, a model link there is named
by the same rule (`glove_l.fmdl.common` in `Boots/` is `fmdl_name_invalid`): the link stands
for the model at its name, so one rule covers both, rather than a link the folder's package
cannot use being left out silently.

**SKL pairing** — a model may carry a custom skeleton. Each source format keeps it differently: an
`.fmdl` in a companion `.skl` file named after the model's source basename (`commander.skl` for
`commander.fmdl`, `kit_boots.skl` for `kit_boots.fmdl`); a glTF in its native `skin` (see
"Skeleton: native glTF skin" in the [Unified model format plan](../model_format.md)); a `.model`
**inline**, in its per-bone matrix table — pre-Fox has no sidecar to pair (see "Skeleton in
`.model`" in the [Model conversion plan](../model_conversion/README.md)). The `.skl` pairing is recognized in
**Common folders, shared (Faces/Boots/Gloves) folders, and player folders** alike. When a model is
pulled in — via a `.common` link, a shared link, or used locally — its skeleton travels with it into
the destination output: on Fox targets as the destination's `.skl` (pass-through bytes for `.fmdl`
inputs the retargeting leaves unchanged; generated from the IR for glTF and `.model` inputs that carry bones outside the target's
skeleton tables), on pre-Fox targets inside the written `.model`'s bone table, for every source
format. This covers the rare case where a model has a custom pose that depends on a custom skeleton;
the common case needs no SKL and the compiler's template skeletons suffice. Independently of custom
skeletons, every model is **retargeted to the target version's body skeleton** at compile time — the
games' `body.skl` files differ per version in bone set and rest pose (PES15 markedly), and the
compiler ships all of them; see "Skeleton retargeting and bone conformance" in the [Model conversion
plan](../model_conversion/README.md). This absorbs the standalone `4cc-model-simplifier-15` step.

Only two output destinations have SKL slots: **`fcl_hair`** (the `fcl_hair_sim.skl` slot) and
**`boots`** (the `boots.skl` slot) — both are full-body-skeleton models. `face_high`, `hair_high`,
and `oral` have no skeleton slot. A custom SKL arriving at a destination replaces the template
injection for that slot: if any part merged into the destination brings a custom SKL, the template
`boots.skl`/`fcl_hair_sim.skl` is not injected and the custom one is renamed to the slot's canonical
name. If no part brings a custom SKL, today's template injection stands. A custom SKL paired with a
model that resolves to `face_high`/`hair_high`/`oral` has no slot to land in and is reported as a
warning (`skl_no_slot`) — a no-op file the user likely authored by mistake.

**What the injected files are.** Both slot templates are the **body skeleton** under the slot's
name — Red ships PES21's `body.skl` and writes it as `boots.skl` / `fcl_hair_sim.skl`. The name
`boots.skl` is misleading: the game's own `boots.skl` (`resources/skeletons/pes*/boots.skl`) has
four bones (`sk_foot_*`, `dsk_toe_*`) and is never what a 4cc export ships. The community
convention this encodes is that the boots folder is the place for a **full-body model** whenever the
`fcl_hair` slot is not used — a body model saved as `boots.fmdl` needs the whole body skeleton, so
members used to add a renamed `body.skl` themselves; the compiler's template made the manual copy
unnecessary. Real boots models use a subset of the same bones, so the body skeleton serves them too.
PES21's file is the template because it has the largest bone set of the Fox versions, so any bone a
model may reference has a bind pose. **Once the retargeting pass folds bones the target lacks, that
superset rationale disappears** and injecting the *target version's* `body.skl` becomes the
consistent choice (its rest pose is what the game's animations drive). Whether the game behaves
differently with a same-version skeleton than with PES21's is untested in-game; the plan keeps PES21
as the injected file until a compile-and-play comparison on PES18/19 settles it (open point).

**`face_diff.xml`.** A face folder may give its `face_diff.bin` as text instead, the form the
pre-Fox `face.xml` carries it in: the binary file base64-encoded (standard alphabet, padded;
line breaks and other whitespace anywhere), either alone in the file or as the text of a `<dif>`
root element (a file whose first non-blank character is `<` is read as XML; a UTF-8 BOM is
skipped). On a Fox target the compiler decodes it and packs it as `face_diff.bin`. A `<dif>`
holding child elements instead of text (Konami's structured form) is not supported. The
decoded bytes, and a `face_diff.bin` supplied directly, must be a face diff: the magic `FACE`
and at least the length its header gives, `0xF0 + 0x10 × count(0x48) + 0x20 × count(0x4C)`
(both counts `u32` little-endian), otherwise `face_diff_invalid` drops the folder, since the
game would read past the file's end. A longer file passes: of the 2,695 loose `face_diff.bin`
on the maintainer's machine (2026-10-03), 326 are 960 bytes long with counts that give 944,
and they are in cups' CPKs. A folder holding both `face_diff.bin` and `face_diff.xml` gives
one datum twice and is discarded (`xml_dif_conflict`); a player folder's own file over a
combined shared face's is the ordinary layering, whichever form each has. Both findings are
the Team compiler's deep pass's: `check` reports them, the folder holding the file is dropped
before any ID is planned for it, and neither is pass-through-eligible, as there is no usable
diff to keep. The pass checks every file planning gives a face diff's role, which is the root's alone
("Subfolders": a subfolder's is `file_type_disallowed`); a file with no role (in a folder
with no face model and no face link) is not read. The roles are the Fox output's, so a
folder whose only face models are `.model` files has its face diff unchecked until the pre-Fox
face steps give it a role. No loose
`face_diff.xml` exists on that machine, so the test fixtures are encoded from real
`face_diff.bin` files. A diff is passed through for whatever target is compiled, although the
two engines use it differently to shape the face skeleton: converting one engine's diff into
the other's is uninvestigated (worklog "Issues").

**Merge constraint** — parts merged into one output FMDL must reference the same skeleton. A part
with a custom SKL and a part using the default template skeleton reference different skeletons, as
do two parts with different custom SKLs. "Same" is decided by **content hash** for two `.skl`
files (identical bytes under different filenames are one skeleton), a skeleton the `.model`
conversion writes included: the conversion writes one only for a model keeping a bone outside
the game's tables, two skeletons of one output name meet only through a combined boots or
gloves link, and a mismatch fails loudly (`skl_merge_conflict`), so a tolerance comparison,
which would carry each skeleton's origin through the merge, buys nothing an export reaches
(`DECISIONS.md` 2026-10-09). The pre-Fox native merge compares `.model` bone matrices within a measured `1e-4` per
component (Libraries plan, "`pes_model::ops::merge`"). A skeleton mismatch between merge
parts is a hard error (`skl_merge_conflict`) that leaves that package out (`messages.md`).

**Kits** live in a `Kits/` folder with one subfolder per kit (this per-kit granularity also drives
the GUI's per-kit grid cells). A kit folder is named by its **slot**, optionally followed by a
label in the same `<slot> - <label>` shape as player folders: `p1`, `p1 - Lakers`, `g1 - Goalie`.
The slot part (`p1`–`p9`, `g1`, or `all`, case-insensitive) is all the compiler reads; the label is
for the human — the GUI kit cell's tooltip and the Kit config editor's tab show it, nothing is
derived from it. Two folders resolving to the same slot (`p1/` and `p1 - Lakers/`) are an error
(`kit_slot_duplicate`), not a pick. The special folder **`all/`** holds textures shared by every
kit: for each kit, the effective texture set is the kit folder's own files plus every `all/` file
whose stem the kit does not have itself — own files win, per stem. Most teams use the same
`_back`/`_leg`/`_name` number and name textures on all their kits, and copying them into nine
folders is what made old exports heavy and what let one kit silently fall behind when the set was
updated. The rule is per stem, so a kit can inherit `kit_back` and still override `kit_name`; it
covers the main `kit.dds` too, for the rare team whose kits differ only by config. The five
texture-name fields of each kit's config are derived from the *effective* set — an inherited
`kit_back.dds` makes the kit's `back` field non-empty exactly as an own copy would. In a `Midcup`
export a field is also named when the game already has that texture of the slot installed: the
export does not resend a number font it gave in its `Full` one, and the game finds it installed. `all/` is not
a kit: it has no cell, no `config.toml`, `colors.txt` or icon marker (such files there are
reported and ignored), and an `all/` no kit inherits is reported as unused (a `Full` export's
mandatory empty `p1`/`g1` still count as kits). A shared base
config was considered and rejected: a kit folder without `config.toml` means "the template" (in a
`Full` export; a `Midcup` one keeps its slot's installed config), so
the folder describes its own look and can be copied between exports unchanged. With an inherited
config, the same folder would compile differently in each export, and a collar or shorts-number
change is invisible until the game shows it. An inherited *texture* changes a copied kit too, but
visibly, as itself — and often intentionally, since the kit takes on the number font of the team
it joins.

A kit folder may be **completely empty** — a bare `p1/` with nothing in it, and no `all/` to
inherit from. It still declares that the slot exists, and the compiler fills it in: a bundled
placeholder main texture (the magenta/black "missing texture" checkerboard), the template
`config.toml`, and a `UniColor` entry as for any kit (colors from the kit's `colors.txt`, else the
loud magenta/black "no colors chosen" pair — see "Kit colors fallback" in the [Team compiler
plan](../team_compiler/README.md)). Teams whose whole roster is
full-body models never render a kit texture (except a pre-Fox player given the `uniform` model
type), yet the game expects every kit slot the team uses to have a texture and a config; the empty
folder says "pretend a kit exists here" without making the author draw one. The same fill applies
to any kit whose effective texture set lacks `kit.dds`, so a folder holding only a `kit_back.dds`
is a placeholder kit with a custom number font, not an error. None of this fill applies in a
`Midcup` export: its kit folder gives only what it holds, and the slot keeps the installed rest
("Bins accumulation" in the [Team compiler pipeline](../team_compiler/pipeline.md)). Kit textures follow the export-wide
texture rule: matched by **stem**, in **any accepted image format** — `kit.png` is as good as
`kit.dds` (the compiler converts; the diagrams say `.dds` only by habit). Online kit designers
hand out PNGs, and a new manager should not need a DDS tool to use one.

**Kit layout marker.** The kit UV layout changed between the two engines: the shirt, sleeves,
collar strip and shorts are laid out the same in PES 15–17 and 18–21, but the **sock islands are
68 px narrower** in Fox (u 8–372 instead of 8–440 on the left, mirrored on the right; height
unchanged), and not by one scale: the pre-Fox sock gives the part of the leg near one seam more
texels than the rest, the Fox one spreads them evenly. A kit drawn for one engine and compiled
for the other therefore shows its sock design displaced around the leg, by up to about 70 px of 2048. An
optional empty **marker file** in the kit folder, named
`pre-fox` or `fox` (case-insensitive; the Notepad forms `pre-fox.txt` / `fox.txt` are tolerated,
as for `ingame_face`), declares which layout the kit's `kit.dds` and its `kit_mask.dds` /
`kit_srm.dds` are drawn for. When the marker names the other engine than the compile target, the
compiler re-lays those textures out (see "Kits" under "Processing" in the [Team compiler
plan](../team_compiler/README.md)); when it names the target's engine, or is absent, nothing is edited —
**no marker means "drawn for whatever you compile for"**, which is today's behavior. Both markers
in one folder is an error (`kit_layout_conflict`, kit discarded), like `fpc_conflict`. The marker
belongs in kit folders only: in `all/` it is `kit_all_file_ignored` like any non-texture, and a
kit that inherits `all/kit.dds` states the layout of that inherited texture with its own marker.
It is a marker file, not a `config.toml` key, because `config.toml` holds only data that reaches
the game's kit config; the layout describes the texture and stays beside it. The Export upgrader
never writes one: kits have passed through several converters unchanged, so their true origin is
unknowable from the files, and a wrong guess would silently displace a correct kit. The only
authored kit config is the TOML file `config.toml` — the
binary game format is never part of an export and exists only as compile output; old exports' binary
configs are converted by the [Export upgrader](../export_upgrader.md) (schema and binary layout in the
[Kit config editor plan](../kit_config_editor.md)); each kit folder also has a `colors.txt` with the
kit's two menu colors (the grammar is under "Root files", "Colors"), and
optionally an empty `icon_<N>` marker whose name carries the kit's menu icon number (0–23, zero
padding optional; the old Note txt kit entries' trailing number — selects the two-color kit icon pattern shown next to the formations in the
prematch gameplan screens, which only PES 15/16 display; absent = default 3):

```
Kits/
├── all/                      (optional: textures every kit inherits unless it has its own file of that stem)
│   ├── kit_back.dds
│   ├── kit_leg.dds
│   └── kit_name.dds
├── p1 - Lakers/              (slot, optionally ` - ` and a free label)
│   ├── config.toml           (kit config — TOML; compiled to the game binary at compile time)
│   ├── colors.txt            (the two menu colors; grammar under "Root files", "Colors")
│   ├── icon_7                (optional empty marker: menu icon number, 0-23; absent = default 3)
│   ├── pre-fox               (optional empty marker, `pre-fox` or `fox`: which engine's kit layout the
│   │                          main texture and mask are drawn for; absent = the compile target's)
│   ├── kit.dds               (main kit texture)
│   ├── kit_mask.dds          (kit textures: generic "kit" prefix — no more u0XXXp1 naming;
│   ├── kit_srm.dds            the prefix is replaced with the kit's ID, e.g. u0701p2,
│   ├── kit_chest.dds          at compile time; `_mask` is the pre-Fox material map and `_srm`
│   └── kit_*.dds              the Fox one — each is emitted only for its engine, never converted)
├── p2/
│   ├── kit_name.dds          (overrides all/kit_name.dds for this kit; kit_back/kit_leg are inherited)
│   └── ...
├── p3/                       (empty: a placeholder kit — checkerboard texture, template config, menu
│                              colors from a colors.txt here or the loud "none" pair; for full-body teams)
└── g1/
    └── ...
```

**Portraits** — a player's portrait normally lives in their player folder (stem `portrait`, any
accepted image format). A DDS source under the legacy 128-byte header passes through unchanged.
One under a DX10 extension header goes out re-headered, its pixel data untouched: the legacy
FourCC header for BC1, BC2 and BC3 (an sRGB DXGI id as its UNORM twin) and the masks header for
BGRA8, a DX10 header with the UNORM id for a format the legacy header cannot express (BC7); an
uncompressed DX10 layout the legacy header cannot express (RGBA8) is encoded like a raster
source. On PES 15-18, which read BC1-BC3 only (`spec/libs/dds_convert.md` "Passthrough"), a BC4,
BC5 or BC7 portrait, under either header, is encoded like a raster source too. Re-headered
rather than re-encoded, because the blocks are fine and a re-encode is lossy (the legacy
compilers re-encoded every DX10 header to DXT5). The game's own DDS reader is the reason: a 128x128 single-level BC3 portrait under a DX10 header with the sRGB id
(DXGI 78) crashed PES 19 when its slot was hovered (VGL, 2026-10-09), and the same blocks under
a `DXT5` header did not; of the cup corpus's 1218 portraits, 520 are single-level legacy
ones and 12 DX10 BC7 ones, none under a DX10 sRGB id (`QUESTIONS.md` asks which header
property the game cannot take). The rewrite is reported as `portrait_header_rewritten`. Any
other accepted image format is encoded to BC3 (DXT5) at its own size with a full mip chain.
The game is not fussy: the 1534
portraits in the installed PES 2021 cup CPKs are all DDS passed through as their makers made them,
in size classes from 64x64 DXT5 to 512x512 DXT5 (uncompressed BGRA8 included), and 128x128 DXT5
with a full mip chain is the commonest class, so it is the one non-DDS sources are encoded to. The
standalone `Portraits/` folder (`NN.*` files, named by slot as a player folder is) is deliberately kept alongside: some managers
make custom portrait sets — while keeping the players' models unchanged — depending on the opponent
team they are about to face, and the standalone folder supports those model-less portrait exports.
A player with a portrait in both locations is an error when the two files differ
(`portrait_conflict`, which skips the export); two byte-identical files are one portrait.

**Root files** — the old Team Note txt is dismissed entirely. In its place:

```
<export root>/
├── colors.txt                (optional: the team's colors, up to four; same grammar as a kit folder's, see "Colors")
├── notes.txt                 (optional: strict UTF-8 free-form notes, collected into teamnotes.txt at compile time)
├── README.txt                (optional: for humans only — known to the model, ignored by the compiler;
│                              the Team creator writes one with next steps)
├── players.txt               (optional for teams, required for refs: slot → player folder mapping)
├── logo.png                  (optional: the team logo, one image in any accepted format, any size;
│                              a `_crop` / `_stretch` / `_fit` tag chooses how a non-square one
│                              becomes square — see "Logo")
├── logo_small.png            (optional: a different image for the game's smallest, 128² logo —
│                              menus use it; same tags; absent = downscaled from logo)
├── Players/ ...
├── Kits/ ...
└── ...
```

- **Team identity**: the canonical `team_name` is always derived from the first word of
  `export_display_name`; the complete stem remains presentation/source identity only. There is no
  `Team:` line anywhere.
- **Colors**: one grammar for the optional root `colors.txt` (feeding `TeamColor.bin`; absent: the
  team's existing bin colors are left untouched) and each kit folder's `colors.txt` (feeding
  `UniColor.bin`, with the kit's icon marker). The file is UTF-8 with an optional BOM, LF or
  CRLF. Each non-blank line holds **one color**, optionally after a label ending in `:` (a
  leading `- ` is tolerated): `#RRGGBB`, or three decimal components 0-255 separated by spaces
  or commas. A kit's two colors are two lines:

  ```
  211 74 79
  162 62 77
  ```

  A line that does not parse as exactly one color reports `color_entry_invalid` and is skipped.
  That includes two colors on one line, the old Team Note kit entry (`211 74 79 - 162 62 77`,
  with or without its trailing icon number): one color per line leaves no separator to get
  wrong, and the Export upgrader splits a Note's kit entry into two lines and an icon marker
  when it migrates it. A kit's file uses its first two valid colors; the root file its first
  four, the record's capacity (a `TeamColor.bin` record is `u16 team_id`, `u16 4`, then four
  3-byte RGB colors, and Red writes them with no cap, so a fifth would overwrite the next team's
  id). Valid lines past those report `color_entry_invalid` ("more than N colors"). Team color
  slots the file does not fill keep the working bin's bytes. The color is what follows the
  line's last `:`; bytes that are not UTF-8 make their own line invalid, not the file. The
  reader is `aesthetics_export::colors_txt` (the Kit config editor reads the same files); the
  Team compiler's deep pass reports its refused lines, so `check` shows them.
- **Marker names**: a marker is an empty file whose name says something about its folder, and
  every marker with a value is written `<name>_<value>`, like `collar_12` and `player_05`:
  `fpc_on` and `fpc_off` in a player folder, `icon_<N>` (0-23, zero padding optional) in a kit
  folder for its menu icon. A dot would make the value a file extension, which file browsers
  hide, sort by and warn about on rename. The valueless markers
  keep their names: the kit layout's `pre-fox` and `fox`. Every marker
  tolerates a `.txt` after its name, what Notepad's save dialog adds. A kit with two icon
  markers, or one whose number is not 0-23, reports `kit_icon_invalid` and uses the default
  icon.
- **Notes**: optional strict-UTF-8 root `notes.txt`, replacing the old "Other Notes" section. An
  optional BOM is stripped, newlines normalize to LF, invalid encoding drops the note with
  `notes_encoding_invalid`, and empty/whitespace-only content produces no `teamnotes.txt` entry.
- **Player numbers**: optional root `players.txt` for normal teams as an alternative to numbered
  folder names; required for referee exports, whose authoritative slot mapping and repetition cannot
  be expressed through numbered folder names.
- **Logo**: optional, at most **two root image files**, any accepted raster format (the texture
  allowlist), any size. Stem grammar: `logo` `[_small]` `[_<fit>]`, tags in that order, matched
  case-insensitively. `logo*` (no `_small`) is the **main** logo: the compiler always produces the
  game's 512² and 256² PNGs from it, and the 128² one too unless `logo_small*` exists.
  `logo_small*` is an optional **different image for the 128² logo only** — the game's menus show
  that one, and teams sometimes want a simplified or joke version there; it never influences the
  two larger sizes. `<fit>` is one of `crop` (center-crop the long side), `stretch` (resample to
  square, distorting), `fit` (letterbox with a transparent border), each file carrying its own; a
  square source needs no tag; a non-square source without one is treated as `fit`, the only mode
  that loses nothing and distorts nothing — the tag exists to *opt into* loss. A `logo_small*`
  without a main `logo*` is an error: the game needs all three sizes and the small one is not a
  source for the large ones. There is no `Logo/` folder: the old format asked authors for a
  hand-made triplet with a placeholder team ID baked into three filenames, and one in eleven
  shipped exports got the sizes wrong; files the compiler resizes cannot. More than one file per
  role (two `logo*`, or two `logo_small*`) is an error, not a pick.

At compile time, the pipeline:

0. **Hand auto-split** — in-process in Rust via `model_convert`, select each hand's vertices with
   positive `skh_*_l`/`skh_*_r` weights, grow the selection once along the mesh topology, and separate
   it into `glove_l`/`glove_r`, leaving the body. This is equivalent to Blender's select → `Ctrl +`
   → `P` workflow, not an invocation of Blender (see "Hand auto-split" in the
   [Model conversion plan](../model_conversion/README.md)). Models without such weights pass through unchanged.
   Only models step 1 makes face content are split: the body stays face content and the split
   parts join the player's gloves, while a model named as boots or gloves, or in a shared
   `Boots/`/`Gloves/` folder, is never split.
1. **Categorizes** each model file as face/boots/gloves — by filename convention or by
   face.xml-style metadata (the same categorization problem the 16→21 converter already solves via
   `parseFaceXml` and filename matching). Anything the conventions don't claim is **face content by
   default**, so there is no "uncategorized model" failure. Pre-Fox the category only picks the
   model's *type* attribute in the generated `face.xml` — everything ships in the one face folder;
   Fox splits the categories into separate folders and merges each category's parts (see "Common
   model links and model merging")
2. **Assigns IDs automatically** (Fox needs both pools; pre-Fox only the shared one, having no
   player-exclusive boots/gloves folders — except under `ingame_face.toml`, see "ingame_face
   marker") —
   from a **hardcoded per-team block of 40 IDs** in the 4-digit boots/gloves ID space: IDs 0 to 100
   are the **stock band** (the cup's stock kit compacts Konami's boots and gloves into it; a player
   wearing one names it with `settings.toml`'s `boots_id`/`gloves_id`, see "Player settings in
   exports"), then each team gets a fixed block starting from team ID
   701 (`block_start = 101 + (team_id - 701) × 40`; team 701 gets 101–140). The block splits into
   the player-exclusive part (deterministic: `block_start + player_number - 1` for slots 01–23,
   extending the 16→21 converter's `bootsId = base_id + player_number - 1` scheme) and the **17
   shared-folder IDs** (the remainder, assigned in alphabetical folder-name order, case-insensitive
   as the export's names are (`apple` before `Mango` before `Zebra`), so the order is the one a
   member sees in Explorer rather than one where every capital sorts first — deterministic, so
   recompiling an unchanged export yields the same IDs). Only a folder that two or more roster
   slots link plainly takes a shared ID: a folder exactly one slot links plainly takes that
   slot's exclusive ID instead, still packed as a boots or gloves output of its own and never
   through his face folder (an `ingame_face.toml` player gets no face folder at all, since any
   face folder overrides his in-game face). His exclusive ID is free for it, because a player wears
   one boots and one gloves ID, and a link beside his own model of its kind already merges into
   his own output. So with 23 slots at most 11 folders of a kind need a shared ID, and a `Full`
   export never exhausts the 17; we do not lend the unused exclusive IDs to the shared pool
   instead, because which ones are unused depends on the whole roster, which a `Midcup`
   compiling some players cannot see. Link files resolve to the assigned IDs.
   A shared boots or gloves folder holding no model of its kind takes no ID: it has nothing to
   load, so a player linking it plainly wears the game's own and is told
   (`shared_folder_no_model`, `team_compiler/messages.md`); linked beside his own model it is a
   texture source, as any combined folder is.
   Boots and gloves are **disjoint game namespaces** (separate `boots/{id}/` and `glove/{id}/`
   folders, `k`/`g` prefixes), so the identical block layout applies independently in each — no
   boots-versus-gloves split of the block is needed. Sizing rationale: at 220 teams (IDs 701–920),
   the 40-ID blocks end at 8900, leaving 8901–9899 unallocated and keeping clear of the `99XX` band
   that referee `k99XX`/`g99XX` IDs occupy (the hard ceiling would be 44 per team, ending at 9780;
   45 would cross into the referee band and overflow into 5 digits). The block size is fixed for the
   lifetime of a scheme version — IDs are baked into distributed outputs. The ABI is future-upgradeable: a later FoxDen
   change might allow 5-digit IDs, letting each player's boots/gloves ID equal their player ID; any
   such change is a new scheme version. On Fox the IDs live in the CPK's `BootsList`/`GloveList`
   rows, so a scheme change needs no savefile remake; on pre-Fox they are baked into distributed
   savefiles, so a scheme change ships with a from-scratch savefile remake. Either way bumping the
   version is a sanctioned path rather than a compatibility break
3. **Splits and packs** everything into the correct game structures (face folder with player ID,
   boots/gloves folders with assigned IDs), rewriting FMDL/MTL/XML texture paths to match

A team export's root also admits the valueless `write_names` marker: the aesthetics patch writes
its players' names ("Player settings in exports" in [`settings_toml.md`](settings_toml.md)).

The block size is fixed for the lifetime of a scheme version — IDs are baked into distributed
outputs — so the aesthetics patch records `allocation_scheme_version = 1` in its header, and applying a
patch of another scheme is refused.

**A `Midcup` export keeps the shared IDs the team already has** (step 2 above, "Assigns IDs
automatically"). A cup's Midcup export carries only the shared boots and gloves folders that
changed, and its CPK loads above the cup's main one, whose rows stay for every player it does
not compile; numbering its own folders alphabetically would give a new folder B the ID an
absent folder A holds below, and A's wearers would wear B. So every compiled CPK records, per
team, which shared folder took which ID, in `unlimited-tool-works/shared_ids/<team_id>.toml` (a path no
game reads; on both engines):

```toml
# Written by Unlimited Tool Works; do not edit.
[boots]
1690 = "Crocs"
[gloves]
1684 = "Grip test"
```

A `Full` export numbers its folders taking a shared ID alphabetically and records them; a
folder under its one wearer's exclusive ID is not recorded, the ID being his. A `Midcup` export
first reads the team's record from the installed CPKs below its output (the topmost holding
one, the walk "Bins accumulation" makes for the working bins; `check` reads it too, so the
two report the pool and `shared_ids_unrecorded` alike): a folder named there, compared
case-folded as folder names are, keeps its ID, even when one slot of the Midcup links it (the
main CPK's other wearers still wear that ID); a folder not named takes its one wearer's
exclusive ID as in a `Full` export, or, linked by two or more, the lowest of the
team's 17 shared IDs of its kind neither the record nor an earlier new folder holds,
alphabetically; and the CPK records the installed entries with the new shared ones, each folder
the Midcup holds under its own spelling. When no installed
CPK holds a record (the main CPK built by Red, or before this rule), the Midcup numbers its
folders taking a shared ID alphabetically and reports `shared_ids_unrecorded` (a warning naming
the team): its IDs may be ones the main CPK gives other folders. Such a Midcup with no shared
folder taking a shared ID writes no record: an empty one would read, to the next Midcup above it, as the team having
no shared folders, and that Midcup's new folders would take the main CPK's IDs without the
warning. A Midcup whose new folders do not all fit among the
free IDs is refused as a Full export with more than 17 is (`boots_id_pool_exhausted`,
`gloves_id_pool_exhausted`). We record in the CPK rather than in the
export or a Studio setting because the CPK is what every member installs and compiles over:
the record travels with the IDs it describes, and a Midcup compiled on another machine reads
the same one. We do not require a Midcup to carry every shared folder, which would make
members copy folders they did not change.

**The old export format is not supported by the compiler.** Old exports are migrated once with the
Export upgrader tool (see the [Export upgrader plan](../export_upgrader.md)), which also resolves old
embedded IDs to the new name-based structure.

---
