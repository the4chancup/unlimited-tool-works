# Unlimited Tool Works — Export upgrader plan

The Export upgrader (`crates/tools/export_upgrader`) migrates exports from the old
format (the AET layout, `GLOSSARY.md` "AET") to the Studio player-folder format defined in the
[Team compiler plan](team_compiler/README.md) — the compiler only supports the Studio format,
so this is the one-time migration path for every existing export.
Platform context is in the [core plan](core/README.md).

---

## What it does

Migration builds a separate output draft and leaves the original source intact. References below
to moving, flattening, or removing files describe that draft, not destructive edits to the input.

1. **Reads the old export** (folders / .zip / .7z): Faces/Boots/Gloves item folders with embedded
   IDs, Kit Configs + Kit Textures folders, Common, Portraits, Collars, Logo, Other, Team Note txt; folder
   names are matched in any letter case, as Red matches them, while an item folder's `k` or `g`
   is lower case, as Red requires (a `K0430` folder is unmigrated content), and a top folder
   holding nothing but one subfolder of its own name, letter case aside (`Faces/Faces/`; Other
   and Common aside), is read as that subfolder, one level, as Red flattens it. A file the game stores zlib-wrapped (a WESYS `.mtl` or
   `face.xml`; 48 wrapped `face.xml` files on the maintainer's machine) is unwrapped for the
   rewrites below. A rewritten `.mtl` is written back wrapped, the compiler reading either. A
   `face.xml` is written unwrapped, rewritten or not: the compiler parses a member's `face.xml`
   as plain XML and refuses a wrapped one (`xml_broken`, the folder discarded), so a wrapped one
   carried as it was would cost the player his face at the first compile. OS artifacts (`Thumbs.db`, `desktop.ini`,
   `.DS_Store`, at any depth) are left out of the output and not reported: Red's copy drops
   `*.db` and `*.ini` files, the compiler ignores the three ("OS artifacts" in
   `spec/aesthetics_export/object_model.md`), and the maintainer's old exports hold 670 of them,
   which step 10 would otherwise list one by one. A Fox model folder's `*.fpk.xml` (`face.fpk.xml`,
   `boots.fpk.xml`, `glove.fpk.xml`, about 5,400 in the maintainer's export folders) is left out
   the same way, in both modes: it is the source-side list of an FPK's members, which the
   compiler writes from the folder's own files, so carried it would only make a `file_not_used`
   Warning per model at every compile, while Red packed it into the CPK as a file no game reads
   (the parity tracer's `boots.fpk.xml` rows, "Not produced" there). (A bare
   folder of item folders is the other accepted input — see "Model-folder mode".) An archive's
   name is its file's stem, the name of the folder inside it aside, as Red named an export
   (a member's renamed `day5.zip` holding `co Spring/` is `day5`). The export's root is the
   input's own, or, when that holds none of the files and folders this step and step 9 read
   but exactly one folder and no other file (OS artifacts aside), that folder: the AET page
   packs an export inside one main folder, and a folder input wrapped the same way reads the
   same, so the two are one rule. An input holding no Note,
   none of the folders above and none of a referee export's `refs.txt`, `Refs list.txt` and `Players/` (step
   9; the real `refs lan party` export holds `refs.txt` and `Players/` alone) is reported as no old export, nothing written, exit 1, as Red
   refused it ("No usable files found"): resolved from its name alone, it would become a Full
   export without kits, which compiles placeholder kits over the team's installed ones.
2. **Reads the team's savefile data** (via `pes_savefile`) to map old boots/gloves IDs to players —
   the old format encodes "which player wears boots k0915" only in the savefile, so the savefile is
   the input for a faithful migration; without one, loose mode (step 11) infers the wearers from the
   IDs instead. A stripped Fox save serves like any other: strip-and-seed sets the appearance
   ids to -1 and leaves the rest of the records, boots/gloves IDs included, as they were (step 8).
   A face folder whose slot the save's roster leaves empty (a roster of 22 with a `XXX23`
   folder) still becomes its player folder, with no `settings.toml`, and is reported (W,
   `roster_slot_empty`, the slot in its context): the compiler's validation then rules on the
   slot as for any export. The roster is read by player ID (`team_id * 100 + NN`, "Savefile
   identity" in `spec/aesthetics_export/player_folders.md`), never by position: a real save's
   roster need not be in slot order.
3. **Builds player folders**: each player's face folder contents move into `Players/NN - Name/` (NN
   from the face folder's `XXXxx` suffix: its first three characters are the `XXX` placeholder
   or a baked-in team ID, any three characters, and NN the two after them, as Red reads them;
   1,034 of the 3,641 face folders on the maintainer's machine carry an ID, `73921 - Dorifto`).
   NN must be two digits making 01 to 23, as Red requires; a face folder without them is
   unmigrated content as a whole. The name part is what follows, its leading spaces and hyphens trimmed (`XXX15 - Snuffy`,
   `XXX15 Snuffy` and `XXX15- Snuffy` all give `Snuffy`), and the same rule reads a boots or
   gloves folder's (`k2101 UNIFORM` → `UNIFORM`). Two face folders of one slot are one too many:
   the second by name is unmigrated content, as Red kept the first. A face folder holding
   the game's own path to its face (`Faces/81401/common/character0/model/character/face/real/81401/`,
   an extracted face CPK: FGOG VGL6 and minipatch, /co/'s fetus export, 2016-2018), every file
   of it below `common/character0/model/character/face/real/` and one folder there (letter
   case aside, OS artifacts aside), has that innermost folder's contents moved instead, the
   files directly in it as a face folder's are:
   the folder is the member's face for the slot, an extracted face CPK of their own. Red does
   not deliver it: its face CPK nests the tree below the player's own path
   (`face/real/79501.cpk` holding `.../real/79501/common/character0/.../real/81401/face.mtl`),
   and the `face.xml` it generates at that path, finding none at the folder's top, names models
   that are not beside it (FGOG VGL6's Red output; a CPK listing, not checked in game). We
   migrate the member's face rather than reproduce that output, because it is what the export
   was made to show. Moved as it is, the tree would sit below the player folder, where the
   compile refuses its `face.xml` (`file_type_disallowed`) and drops the player. Name comes, in order of preference, from the **face
   folder's own name part** (`XXX15 - Snuffy` → `Snuffy`; the author's curated name, and what the
   folder is called today), then from the **worn boots/gloves folder's name part** (a player with
   boots/gloves but no face folder, or whose face folder has no name part: one order for every
   player), then from the **savefile name** with its colour codes stripped
   (`pes_savefile`'s `display_name()` — see "Name colour codes" in `spec/pes_savefile/model.md`; the raw codes are
   control characters, invalid in folder names; any other character a folder name cannot hold,
   `/`, `?` or `:` among them, is dropped the same way, and a trailing `.` or space is trimmed,
   which a folder name cannot end in either: `T.O.M.` → `T.O.M`; a source left empty counts as
   absent), then NN alone. Red's Pre-Studio upgrader uses this
   same order. A face folder's portrait stays with the player folder, named `portrait.dds`: the
   AET allows a prefix (any file whose name ends in `portrait.dds`, letter case aside, as Red
   matches it), while the Studio format reads the exact stem `portrait`, so a prefixed one is
   renamed. A folder holding two such files is reported as unmigrated content and both are
   left in the player folder as named. A Fox hair's skeleton is renamed the same way: the old
   format names it after its game slot, `fcl_hair_sim.skl`, while the Studio format pairs a
   skeleton with its model by the model's stem ("SKL pairing" in
   `spec/aesthetics_export/player_folders.md`), so beside an `fcl_hair.fmdl` it becomes
   `fcl_hair.skl` (letter case aside, in both modes): kept as named, the compile reports it
   `file_not_used` and injects the template skeleton in its place, and 15 of the 84
   `fcl_hair_sim.skl` beside an FMDL on the maintainer's machine are custom ones the hair would
   lose. A folder holding `fcl_hair.skl` already keeps both as named, the compile's pairing
   taking its own. Old `ingame_face` and
   tolerated `ingame_face.txt` markers become `ingame_face.toml`, the file the marker is now
   (filled from the record at step 8); a player whose savefile record wears boots/gloves but who
   has **no face folder** also gets a player folder with an `ingame_face.toml`, since a stray
   boots/gloves folder is exactly the ingame-face case. An empty face folder is never that case: it is how a player with an FBM
   removes the placeholder face, as part of the FPC system (Red packs an empty face package for
   it, so the player's own face never shows). It migrates to nothing of its own: its player
   folder is written as any face folder's is, holding no face file and no marker (its
   `settings.toml` alone when the player wears nothing custom), and the compiler writes the
   blank face for a player folder with no face model (`spec/team_compiler/pipeline.md` step 1). The upgrader only has to see
   it, which a listing of files alone does not; the real PMMM, GIG VGL26 exports hold 18 and
   10 such folders. A boots or gloves
   folder holding no model (nothing, or a `boots.skl` alone) is how a member hid a player's
   boots or hands: Red packed an empty package under its ID, and any ID naming no model looks
   the same in game, as the FPC preset's IDs do. Each player wearing one gets the `fpc_on`
   marker, which gives him the preset's blank-model IDs (`spec/aesthetics_export/fpc_toggle.md`),
   rather than an empty package, and the folder's files are leftovers (step 10), so its name
   part names no player (`k2176 - EMPTY` would name its wearer `EMPTY`); the real EGG
   VGL26 export's `Boots/k2176 - EMPTY` and `Gloves/g2177 - EMPTY`, 9 exports in all. On a Fox export, model folders used by exactly **one** player are merged
   into that player's folder (a boots folder used only by player 15 becomes `boots.fmdl` +
   textures inside `15 - Name/`). A file path the two folders share is kept once when the
   bytes match; with different bytes the model folder is not merged but kept shared with a
   link, as in step 4, and reported (I, `merge_kept_shared`, the first clashing path in its
   context), since one path cannot hold both. A boots folder's name is `k` and four digits, a
   gloves folder's `g` and four digits (or three alone, below), then its name part, as Red
   requires; any other is unmigrated content as a whole. A model folder no save record names
   is a leftover (step 10): no player wore it in game. A wearer's player folder takes the boots folder's name part before the
   gloves folder's when the name falls to them. Two item folders
   carrying one ID (`k0430 - A`, `k0430 - B`) are one too many: the second by name is
   unmigrated content, as Red skipped it (`folder_error_num_repeated`). On a pre-Fox export they stay shared folders with a link, as
   in step 4: pre-Fox packs a player folder's every model into his face folder, where a
   member's own `face.xml`, which names only his face's parts, would drop the merged model
   (`spec/team_compiler/messages.md` "User-supplied `face.xml`"), while a link keeps loading
   the folder by its ID as the old export did.
4. **Keeps shared folders**: a model folder used by **two or more** players stays as a named shared
   folder (`Boots/Crocs/`), and each of those players' folders gets a link file (`Crocs.boots`).
   Naming: nearly all model folders in current exports already carry a descriptive name after their
   ID (`k0915 - Crocs`), so migration uses that name part when present (`Boots/Crocs/`) and falls
   back to the old ID (`Boots/k0915/`) for folders named by ID alone; either way users can rename
   them (and their link files) afterwards. Two shared folders of one kind whose names match,
   letter case aside (`k0430 - Crocs`, `k0431 - crocs`), would be one folder. When they hold
   the same files (the same paths, letter case aside, each byte for byte), they are one model
   under two IDs and become that one folder, under the lowest ID's spelling, every wearer of
   either linking it. Otherwise the lowest ID keeps the name and every other takes its ID
   (`Boots/k0431/`), the links following, so no wearer loads another's model. A gloves folder named by a three-digit ID alone
   is read as four digits (`g567` → `g0567`), as Red reads it (its `export_move.py`, gloves
   only).
5. **Restructures kits**: Kit Configs + Kit Textures are reorganized into `Kits/` subfolders, one
   per kit slot (a config named `???_DEF_<ordinal>_realUni.bin`, any three ID characters and
   any letter case, as Red matches it: `_1st_` → `p1/`, … `_9th_` → `p9/`, `_GK1st_` → `g1/`); the binary
   config is converted to `config.toml` via `libs/kit_config` (texture-name fields dropped — the
   compiler re-derives them from the textures present; undecoded bits preserved in the `[unknown]`
   table — see the [Kit config editor plan](kit_config_editor.md)); kit textures are renamed from
   the old `u0XXXp1_…` to `u0XXXp9_…` and `u0XXXg1_…` patterns (into `p1/` to `p9/` and `g1/`; a
   `u0XXXg2` and up, no Studio slot, is a leftover as below) to the generic `kit` prefix (`kit.dds`, `kit_mask.dds`,
   `kit_chest.dds`, …) that the compiler re-expands at compile time; kit colors are extracted from
   the Team Note txt into per-kit `colors.txt` files, one color per line (the Note's kit entry
   holds both on one line, `211 74 79 - 162 62 77 - 3` or `#D34A4F - #A23E4D - 3`; its trailing icon number, when present,
   from 0 to 23 and not the default 3, becomes that kit's icon marker (one past 23, which Red
   replaced by the default, writes none), `icon_<N>`: "Marker names" in the
   Aesthetics export plan). The Note's bytes are read as UTF-8, a byte sequence that is not
   UTF-8 reading as the replacement character: every token the grammar below reads is ASCII,
   and real Notes carry Shift-JIS in their other lines (`JP`'s Summer 18, `poland`, `nijiura`),
   on which Red's strict read (`encoding="utf8"`, no error handler) raises. The Note is read as Red reads it (`bins_update.py`): a line whose
   first word is `team` and whose second starts with `co` (`Team Colours:`, any letter case)
   opens the team colors, one whose first word is `kit` likewise opens the kit colors, a line
   starting with `player`, `other` or `notes` ends the reading, and an entry is a line whose
   first non-blank character is `-`, its colors what follows its last `:`, as `#RRGGBB` or three decimal components,
   each written to `colors.txt` as the Note spells it, which is the "Colors" grammar
   (`spec/aesthetics_export/player_folders.md`); a
   kit entry holding `gk:` in any letter case is a goalkeeper's (the AET template spells
   `- 1st GK:`). A kit entry's colors split at each `-` into its two colors, each read as a
   team color is, and an optional third part, the icon: a whole number from 0 to 23 other than
   3 gives `icon_<N>`, any other third part none, as Red read it (`bins_update.py` ignores
   what is no such number); an entry of one part or of more than three does not read. An entry whose colors do not read is reported
   (step 10) and keeps its slot, which gets no `colors.txt`, the later entries keeping theirs: Red dropped it and shifted
   every later kit's colors one slot down, onto another kit's config. The Note's kit-entry order maps
   entries to slots (player entries → `p1`…, `gk:` entries → `g1`…), matching how Red numbered them
   in `UniColor.bin`. An entry whose slot has no config still becomes its kit folder, holding
   its `colors.txt` alone, which the compiler builds as a placeholder kit in a `Full` export
   and, in a `Midcup` one, as an update of the kit's colors alone, its installed textures and
   config staying (Red wrote the colors to `UniColor.bin` and packed nothing else), because a Note's
   kit lines are what told Red how many kits the team has (the AET page: "The amount of lines
   will let PES understand how many kits you have"); it is reported (`kit_without_config`, W,
   naming the slot), since the page also tells authors to delete lines of kits they lack. Both
   this Warning and the next keep what they name (the finding on the Note's file, or on the
   config's, context `slot`): neither loses anything, so a run whose only findings they are is
   clean. The
   other way round, a config whose slot no Note entry gives (a `XXX_DEF_4th_realUni.bin`
   beside three player entries) still becomes its kit folder, reported
   (`config_without_kit_entry`, W, naming the slot), as Red warned on the mismatch: Red packed
   the config but, counting the Note's entries, never offered the kit, which the upgraded
   export offers; the author deletes the folder if the kit was dropped. A second or later
   goalkeeper kit (a second `gk:` entry, a `u0XXXg2` texture, a `XXX_DEF_GK2nd_realUni.bin`
   config) has no Studio slot and is a leftover (step 10): no version the cup plays shows a
   goalkeeper kit past the first (Red wrote the entry to `UniColor.bin` as kit 0x11). The slot
   names leave room for one: a version that shows a `g2` gets a new slot, not a new format.
   Player entries past the ninth have no Studio slot either (Red wrote them as kits past 9) and
   are unmigrated content (step 10). A Kit Textures file inside a subfolder, which Red did not
   read (step 1's same-named subfolder aside), is a leftover, and so, in a `Full` export, is a
   file naming a slot (`p1`-`p9`, `g1`; a `u0XXXg2` and up is a leftover in either kind) that
   no config and no Note entry of the export gives (a `u0XXXp5.dds` beside configs and Note
   entries for kits 1 to 3): Red packed it, but no kit row named it. In
   a `Midcup` export such a file is the kit's texture update and becomes its kit folder with no
   `config.toml`, which keeps the slot's installed config (`spec/team_compiler/pipeline.md`
   "Bins accumulation"); so does a slot whose Note entry has no config. A `Kit Configs` subfolder
   (the AET page's old `Kit Configs/<team id>/` layout, 953 configs on the maintainer's machine)
   is read as if its files sat at the folder's root, as Red flattens it (`export_check.py`); a
   name found at both places is one config when the bytes match and unmigrated content
   otherwise, and the same holds for any second config of one slot (`714_DEF_1st_realUni.bin`
   beside `XXX_DEF_1st_realUni.bin`): one config with the kit's when the bytes match,
   unmigrated content otherwise, the kit's config being the root's before a subfolder's, then
   the first by name: two configs of one slot cannot both reach the game. A file deeper than one
   subfolder, which Red's flatten left a folder and refused, and a config that does not decode
   (its error named in the report), are unmigrated content too. A Kit Textures file is read by
   position, as Red reads it: `u`, four characters, `p` or `g` with the kit's digit, then the
   rest of the stem, empty or starting with `_`, which the generic name keeps
   (`u0XXXp1_back.dds` → `p1/kit_back.dds`), and the extension `.dds`, in any letter case; a
   file of that shape with the extension `.ftex` is converted to the DDS it holds
   (`ftex::ftex_to_dds`) and migrated as that `.dds`, since Red's Fox mode packed a member's
   `.ftex` kit texture as it was (13 exports on the maintainer's machine ship them, /fgog/'s from
   VGL15 to VGL18 among them), and one that does not convert is unmigrated content, its error
   named. A converted texture stays its own kit's file: the copy between kits and the hoist to
   `all/` below read the copied textures alone, which no real export misses (the 13 ship base
   textures and `_srm` ones, no copy field's stem, but for Red's own `VP Test` folder); any
   other file directly in the folder is a leftover (step 10), in any export kind, since none
   reached the game: Red's check deleted a name not starting `u` with `p` or `g` sixth
   (`back.dds`), its move packed only `.dds` and `.ftex` files (`u0XXXp1.png` stayed behind),
   and a rest neither empty nor `_`-led names a texture no kit row names (`u0XXXp2 -
   Copy.dds`). The
   second by name of two that would land on one name, letter case aside, is unmigrated content
   (`u0XXXp1.dds` before `u0XXXp1.ftex`, the DDS kept as Red's Fox mode converted the DDS over
   the FTEX of its stem). Kit folders are written as bare slots (`p1/`) — the old format has no kit
   names to carry. A kit whose binary config names **another kit's texture** in its back,
   chest, leg or name field gets a copy of that texture in its own folder, under the generic
   name, reported (`kit_texture_copied`, I, naming the kit, the stem and the kit it came from):
   makers copy kit 1's config for the others, so kits 2-4 (and often the GK) wear kit 1's number
   and name fonts without shipping their own (59 of the 1041 distinct old configs on the
   maintainer's machine, in 23 exports, nearly all VGL26). Red rewrites only the team ID in those
   fields and keeps the kit the maker wrote, while the compiler re-derives the fields from the
   kit's own folder, so without the copy those kits would compile without numbers or names.
   The same holds for a field naming a texture of **another stem** than its own, the kit's own
   included (MGGG VGL26's leg fields name `u0811p1_back`, `JP Aesthetics Export World`'s chest
   field its own `u0728p1_front`, a stem the compiler does not use): the copy goes under the
   naming field's generic name (`kit_leg.dds` from `p1/kit_back.dds`), so one rule covers
   every field that names something other than its own kit's own texture (162 fields of the
   1,038 distinct configs, the named texture shipped for 43). A field is read as Red read it:
   by position, `u`, four characters, the kit (`p0` meaning the config's own), the rest
   naming the stem; the source is the named kit's migrated texture of that rest, and no copy
   is made when it is not in the output, or when the kit holds its own file of the field's
   name: the maker shipped that one (MGGG's goalkeeper holds its own back and name textures
   while its config, copied from kit 1, names kit 1's). The finding is on the config's file,
   its context the kit, the field's stem and the source (`kit=p2`, `stem=kit_back`,
   `from=p1/kit_back`). Then the shared textures are **hoisted into `Kits/all/`** (a copied texture moves too when
   every kit now holds it): a stem present in
   *every* kit folder with byte-identical contents (two or more kits) moves to `all/` and the copies
   are deleted (`kit_texture_hoisted`, I, naming the stem). Every kit counts, the empty `p1` or
   `g1` a Full team export is given when it has no player or no goalkeeper kit included (it
   inherits `all/`, `spec/aesthetics_export/player_folders.md` "Kits"), so an export without a
   goalkeeper config hoists nothing. Only the all-kits-identical case
   qualifies: hoisting a stem that some kit lacks would hand that kit a texture it never had and
   flip its config's texture-name field — a game-facing change the upgrader must not make. So the
   upgraded export compiles byte-for-byte as before, just without nine copies of the same number
   font. A survey of the maintainer's 129 multi-kit old exports found `_back`/`_leg`/`_name`
   byte-identical across every kit in roughly a quarter of them; the rest share partially or were
   re-saved with different bytes, and stay as they are — the author can hoist by hand once the
   `all/` folder exists. No kit layout marker (`pre-fox` / `fox`, see "Kit layout marker" in the
   [Aesthetics export plan](aesthetics_export/README.md)) is ever written: old kits have passed through
   several export converters unchanged, so nothing in the files says which engine they were drawn
   for, and a wrong guess would silently displace a correct kit's socks. Authors add the
   marker when they compile across engines.
6. **Replaces the Team Note txt**: team colors are extracted into the root `colors.txt` and the
   "Other Notes" section into the root `notes.txt` (both optional — only generated when the Note has
   the corresponding content). The section is every line after the first line starting with
   `Other Notes:`, in any letter case, as the color sections' headers are read (step 5); Red
   took `Other Notes:` in that case alone, and a real Note's `Other notes:` would lose its text.
   A team-color entry is one color; one holding two, its colors split at its one `-` into two
   parts that each read as one color (a kit entry's spelling, `- 1st: 048 146 082 - 255 255
   255`), keeps the first, written as the Note spells it, and its second is a leftover (step
   10) reported on the Note with the line, Red having kept the first; any other entry that
   does not read as one color (no color, three parts, a second part that is no color) is
   unmigrated content (EU-ROOT-06). The rest of the Note is dropped. Team identity for the migration is
   resolved the way Red's compiler does, since old exports carry the authoritative name in the Note:
   the Note txt's `Team:` line first (the Note txt being a `.txt` at the flattened export root
   whose name holds `note` in any letter case, as Red finds it; with several, the first by name;
   the line is the first holding `Team:` in that letter case, as Red reads it), the
   canonical `/xx/` first-word key derived from the export folder name as the fallback when no
   Note, no `Team:` line or an empty one exists — an export named after something else than its team (e.g.
   "Numbers" for `/#/`) must still resolve. A `Team:` line naming no team of the list is not
   skipped for the folder name: it is `team_name_unknown`, naming the line, as Red stops on it
   unless its user adds the team at its prompt (its `team_id_get.py`), because a Note that names a team the list lacks says the export may
   be another team's than its folder name suggests. The
   resolved key is used for the `teams_list.txt` lookup and as the output's first word ("CLI"); the
   list is the data directory's `teams_list.txt`, the file the Team compiler's default
   `teams_list_path` names, else the embedded list, and one that cannot be read or parsed aborts
   (exit 3), as the compiler's does: the upgrader has no setting of its own for it, since an
   export it resolves against one list and the compiler against another would compile as
   another team; the
   complete folder name remains the display/source name, and the Studio output carries no Note (the format has no `Team:` line —
   see the Team compiler plan's "Root files").
7. **Carries over** Portraits, Collars, and Common, and **converts the Logo folder**. An old
   portrait is named `player_???NN.dds` (the AET page's `player_XXXNN.dds`, the `???` the
   placeholder or a baked-in team ID as with the kit magic below: 2,248 of the 2,271 old
   portraits on the maintainer's machine; Red read NN as the two characters before `.dds`
   whatever stands between, so `player_736917.dds`, in four real `/o/` exports, is slot 17's); it is renamed `NN.dds`, a name the compiler
   accepts (`portrait_name_invalid`: `NN`, alone or followed by ` - ` and any text, as a
   player folder is named). Red also packed a portrait of another texture format, renaming it
   to the game's `.dds` name with its bytes unchanged (BAG VGL18.5's `player_XXX08.jpg`, packed
   as `78808.dds`); a `player_???NN` portrait in a format the compiler converts (`png`, `jpg`,
   ...) migrates as well, renamed `NN` and keeping its format, which the compiler's
   `Portraits/` converts as it converts any texture: the portrait its author meant, now
   readable by the game whether or not Red's rename was. Red's check refused, and discarded, a name not starting
   `player_` (in that letter case) or whose two characters before a three-letter extension
   are not a slot 01 to 23 (it reads `file_name[-6:-4]`; `XXX17.jpg`, `Vanilla vitamins.png`,
   `player_73906-.dds`, `Player_XXX05.dds`, `player_XXX05.jpeg`): such a portrait never
   reached the game and is a leftover (step 10; Red's own output for BAG VGL18.5 holds no
   `78817.dds`), and so is every other file under `Portraits/`, a subfolder's included, which
   Red's check failed too. The upgrader reads the name as Red's check did, so what migrates
   is what the game showed. A slot
   with a portrait here and a different one in its face folder is kept both ways, and the
   upgrader compares the two files' bytes and reports `portrait_conflict` (E, exit 1; on the
   `Portraits/` file, the slot in the context, `slot`; both files stay where they are), as Red
   refused the export: the compiler's comparison belongs to its deep pass
   (`spec/aesthetics_export/object_model.md`), which the structural validation the upgrader
   runs does not include, so without its own the upgrade would report clean an export no
   compile accepts. The Logo
   folder becomes the Studio root files. Its members are the files directly in it whose names end,
   in any case, in `_r_ll.png`, `_r_l.png` or `_r.png` (the AET page's `emblem_0XXX_r_ll.png`; 148
   of the 149 such folders on the maintainer's machine hold exactly those three, the other a
   `Thumbs.db` beside them, an OS artifact the listing skips); any other file, and a second member of one role, is unmigrated
   content (step 10). A member is usable when it decodes as a PNG. The old `_r_ll` (512²)
   image becomes root `logo.png`, its bytes as they are; the old `_r` (128²) image becomes root `logo_small.png` **only
   when it is clearly a different picture** — most old small logos are plain downscales, which the
   compiler now regenerates, but some are deliberate menu variants (simplified, joke), and the
   upgrader must tell the two apart rather than always carry or always drop. The test: downscale
   `_r_ll` to 128² with the suite's one resampler, the compiler's (`dds_convert::resize`:
   Lanczos3, straight alpha), resample the old `_r` to 128² too if it is not already, and compare
   the two pixel by pixel over RGBA. A pixel *differs*
   when any channel differs by more than 32 (of 255); two pixels that are both fully transparent
   are equal whatever their color channels hold (authors leave garbage behind alpha 0). The small
   image is a distinct variant, and is carried, when **more than 40 % of pixels differ**
   (`LOGO_SMALL_DISTINCT_SHARE`); at or below that it is a downscale and is dropped. The number comes
   from measuring the 94 distinct `Logo/` pairs on the maintainer's machine with that resampler
   (`.tmp/6_4/logo_share/`): the 77 downscales score 0–30 % — far above codec noise, because authors downscaled with
   nearest-neighbor or sharpening kernels, shifted or cut alpha edges differently, which the channel
   tolerance does not absorb — and the 17 deliberate variants score 53–97 %: a different
   drawing, a recolored crest (`/pmmm/` VGL26), a crop (`/tf2g/` VGL26). 40 % sits in the empty
   band, so neither mistake has a real pair behind it, and the one nearer (a redundant file the
   user deletes) is the harmless one, not a lost drawing. Both outcomes are reported on the
   `_r` file with the measured share, rounded to a whole percent (`share`, `30%`)
   (`logo_small_carried`, `logo_small_dropped`; both I, and neither drops anything the compile
   would miss), so a wrong call is one look at the report
   away from being fixed by hand: the old file is still in the source export. The `_r_l` (256²)
   file is dropped beside a usable `_r_ll` — the compiler regenerates it. A folder without
   a usable `_r_ll` falls back to the largest usable member, `_r_l` before `_r`, its bytes
   becoming `logo.png`, reported on that file (`logo_fallback`, W), and then has nothing to
   compare the small image against, so no `logo_small.png` is written and the `_r` beside it is
   dropped. A member that does not decode is unmigrated content, its error in the context. `Common/` is carried over, its
   files as they are save the kit magic below. An old `.mtl`'s texture references mostly stay
   as written: the Studio format ignores a reference's extension, so a name in the model's own
   folder (`./face_srm.dds`) already resolves nearest first, and a game directory other than
   the team's Common (`model/character/boots/k0002/ek0062_n.dds`, in the `/o/` 2024 Spring
   export's `Dorifto/realkit/apc.mtl`, with no local copy) keeps its meaning ("Stem-based
   texture references" in `spec/model_format.md`). We do not strip the `.dds`, because the
   compiler puts it back only on a reference it resolves to a file of the export: a game path
   written without it would reach the game without it. A reference that pointed into the team's Common folder by game path
   (`model/character/uniform/common/???/hair.dds` in an `.mtl`, the Fox common directory in an FMDL
   path table; `???` is the `XXX` placeholder or a baked-in team ID, as with the kit magic below;
   in a pre-Fox `.mtl` or `face.xml` it is any folder name, letter case aside: the team's name
   without its slashes (`common/o/env.dds`, `common/vg/oral_shirt_*.model`, in /o/'s 2017-2024
   and VG's 2017-2018 faces, since the 2016 batch compiler wrote a pre-Fox export's `Common/` to
   `model/character/uniform/common/<team name>/` and those faces loaded the files there) or
   another team's (`common/refs/refkit/texture.dds`, a referee kit's `.mtl` copied into ASP's
   4CCC face), because a Studio export names no Common but its own and the compiler refuses a
   segment that is not three characters, so any other reading lets the upgrade end clean and
   the compile fail; a segment of another length that stays a game path is written as the
   team's ID, `XXX` in model-folder mode. The Fox common directory keeps a three-character
   `???`: the batch compiler wrote a Fox export's Common to `common/XXX/`)
   cannot simply become the bare name `./hair.dds` — that would resolve in the player folder and fail — so the
   upgrader writes the bare name **and creates the texture link** `hair.dds.common` (the
   `Common/` file's own name, whose extension the link names) at the root of the
   model's player folder, which is the Studio spelling of the same reference. A shared
   `Boots/` or `Gloves/` folder's `.mtl`, and a `.mtl` in `Common/` (343 such references in
   the maintainer's 1,489 `Common/` `.mtl` files, the compiler emitting a member's `.mtl` as
   written), keeps its Common references as written, save the `???` below, with no finding when the input's
   `Common/` holds the texture (one it lacks is reported as below): the pre-Fox compile does not read a shared folder's links
   (they wait until the cup next plays a pre-Fox version) and `.mtl` is pre-Fox alone, while
   the game path is supplied from the carried `Common/` (a link
   stands for the `Common/` file at its own path below that root, so one beside a subfolder's
   `.mtl` would name a file in a `Common/` subfolder; a bare name from a subfolder finds the
   root's link, nearest first) (see "Link files" in the [Unified model format
   plan](model_format.md)). Every old `.mtl` is read with `pes_model`'s
   `read_dropping_stray_text`: text between its elements (a `\` typed after a `<sampler/>` in
   G Autumn 25's `72117 - Lisa/apc.mtl`, which Red's parser ignored) is dropped and the file
   reported (I, `mtl_stray_text_dropped`), since the compiler's reader refuses it. One that
   names nothing to change and had no such text is copied as it is; one that does not parse
   even so is unmigrated content (its parse error in the context): the compile would drop its
   folder as `mtl_broken`, so the output would not be clean. A rewritten one is written in the
   reader's layout, wrapped again when it was (step 1). A reference into a Common subfolder becomes a path below the
   model's folder and a link at that path (`common/???/u0???p0/sourceimages/armor_bsm` becomes
   the FMDL texture `./kitN/` `armor_bsm` and the link `kitN/armor_bsm.dds.common` in the
   model's folder, the kit magic below respelled in the folder name as in a file name; a kit
   reference counts as present when a variant folder holds the file). The game path's
   `sourceimages/` level is the compiler's, not the export's: Red inserts it when it places a
   `Common/` subfolder's files, so that reference names the old file `Common/u0XXXp1/armor_bsm.dds`,
   which becomes `Common/kit1/armor_bsm.dds`, where the link resolves. A Common-path
   reference whose texture is not in the input's `Common/` (a texture of its stem in the
   directory it names; two there conflict in `Common/` and are both dropped, so they count as
   none), or which no link can spell — the
   model's folder already holding a texture or link of that stem on the reference's
   nearest-first way up (the link would conflict with it, `texture_stem_conflict`, or lose to
   it; the kept game path loses to it too, the compiler looking a game path's file name up in
   the folder first, so the warning's text says the folder's texture of that stem is the one
   the model will wear), or a Common subfolder named from a `.mtl` in a subfolder (the path below that `.mtl`
   and the link mirroring `Common/` cannot agree) — is reported (`common_reference_kept`, W,
   on the `.mtl`, the reference as written in the context, `reference`: an installed CPK may
   hold it, so it is no unmigrated content) and left as it was, save
   its `???`, which becomes the resolved team's ID as Red's `path_id_change` respelled it
   (here and for the goalkeeper token below, a referee export's ID is 999, the one Red compiled
   referees under; model-folder mode, having no team, leaves it): the
   compiler then finds the file in an installed CPK's Common when a midcup export ships without
   it (`fmdl_texture_not_found`'s lookup), which the `XXX` placeholder never names. An
   FMDL texture whose stem is **not in the model's folder but is in the input's `Common/`** gets
   the same stem and link, whatever directory the FMDL named: Red's referee pass points such a
   texture at the team's Common folder, and its team pass pointed it at the model's folder,
   where the file was missing; the link widens the referee rule to teams, giving the model the
   texture its author shipped. A directory below the Fox common directory
   `/Assets/pes16/model/character/common/` that is not the team's (`common/sourceimages/`,
   where the game's shared textures such as `zero.dds` sit, used by the real `CIV4XG` VGL26
   `k2071`) is kept as written, as Red keeps that whole prefix and as the compiler leaves a game
   directory unlooked-for. Every other FMDL texture directory becomes the model's own folder
   (the bare stem), as Red repoints it
   (`fmdl_model_folder_paths_fix`: a model copied between folders, a boots FMDL in a face
   folder, still names its old folder's directory); unlike a `.mtl`'s game directory, an FMDL
   one is not kept. A stem that resolves nowhere stays that bare stem, as Red wrote it: the
   compiler's `fmdl_texture_not_found` then leaves the package out where Red packed the model
   without its texture ("A texture a model names must exist" in
   `spec/team_compiler/pipeline.md`), and keeping the old directory would only hide the same
   missing file (19 of the 5,921 FMDLs on the maintainer's machine name such a texture). The
   FMDL rules in full, per texture entry, a census of the 6,053 real FMDLs in export folders
   behind them (`.tmp/6_4/fmdl_dir_census.txt`): a `dummy_` stem is left as written, as the
   compiler never looks one up; a directory starting `./` is the Studio spelling already, its
   kit magic alone respelled, as a `.mtl`'s; the game's kit texture folder
   `/Assets/pes16/model/character/uniform/texture/` is kept as written (25 real FMDLs), as the
   compiler keeps it (`fmdl_texture_kit_path`): repointing it at the model's folder would lose
   the kit texture to `fmdl_texture_not_found`; the team's Common is
   `/Assets/pes16/model/character/common/` + three characters + `/`, an optional subfolder path,
   then `sourceimages/` (`common/000/Boots/sourceimages/` names `Common/Boots/`, 117 real
   boots FMDLs), linked as a `.mtl`'s Common reference is; any other directory below the Fox
   common directory is kept; every other directory becomes `./`, the stem linked to the
   `Common/` root's texture when the model's folder, nearest first from the FMDL's own folder,
   holds none of it. A shared folder's FMDL is linked as a player folder's, the Fox compile
   reading a shared folder's links; a `Common/` FMDL gets the shared `.mtl` rule (its Common
   references kept, `???` respelled, reported when `Common/` lacks the texture, its other
   directories kept). An FMDL that does not read is copied as it is. This applies to structural migration, not only optional glTF conversion.
   In the same pass, the legacy kit-dependent path magic is rewritten to the Studio spelling:
   `u0???p1`…`u0???p9` → `kit1`…`kit9`, in file and folder names and in `.mtl`/`face.xml` paths
   and FMDL path tables alike, and `u0???p0` → `kitN` in those paths but → `kitF` in a file or
   folder name (`u0875p0.dds` → `kitF.dds`, `Common/u0XXXp0/` → `Common/kitF/`), see
   "Kit-dependent assets" in the [Unified model format plan](model_format.md). A `u0???p0` path
   is the reference, while a `u0???p0` file is what a kit without a variant of its own loads,
   FoxDen's kit swapper keeping a path whose variant is missing: that is `kitF`'s role, and a
   file named `kitN` would be no file of the set, the compiler filling the gaps from the lowest
   variant instead (71 real `u0???p0` files, `MARISA`'s `k4465` holding `u0875p0` to `p4`). The `???` matches any three characters: authors wrote the
   `XXX` placeholder literally or baked the real team ID in (`u0872p1.dds`), and both forms mean the
   same thing. The token is matched as Red's `path_id_change` matches it: `u0`, three ASCII
   letters or digits, then `p` and a digit (or `g`), in that letter case, anywhere in a name or
   path and every occurrence (`skin_u0771p3.dds`, `Common/ltd/u0XXXp5/`). The names renamed
   are those below a player folder, below a shared `Boots/`/`Gloves/` folder and below
   `Common/` (the kit folders have their own naming, `Kit Textures/` step 6's); an output path,
   renamed or not, that another output path already takes, letter case aside, is the second
   input's, not written, and reported `unmigrated_content` (a face folder's `u0XXXp1.dds` beside
   the same name in a boots folder merged into its player folder; `A.dds` beside `a.dds`), where a
   second write would silently replace the first. Inputs of one folder whose paths differ only in
   their tokens' three characters (`u0XXXp2.dds` beside `u0731p2.dds` in a /jp/ export) are the
   exception: they were one file to the old compile, whose rename (`filenames_id_replace`) gave
   each the team's ID and replaced the file already under the new name, in the folder's listing
   order, Windows's case-folded name order. The one it wrote last is what the game loaded, and is
   kept: the last renamed one in that order, an input already spelled with the team's ID being
   replaced by any renamed one. The others are leftovers (step 10), since no compile shipped them.
   We do not refuse these, because the old compile's pick was neither a refusal nor arbitrary:
   the real JP Genso Cup exports' `k0860` holds an older `u0731p2.dds` beside the newer
   `u0XXXp2.dds` the cup showed, and GBPEN VGL26's `g2336` copies spelled `u0799` (another
   team's ID) and `u0XXX`. Model-folder mode knows no team ID, so its collisions stay
   `unmigrated_content`.
   The kit folder token is a texture set's alone (`spec/model_format.md` "Kit-dependent
   assets"): a `face.xml` naming a model in a `u0???p0/` folder is respelled `./kitN/…` as
   written, which the compile does not resolve; no real export holds one.
   The paths respelled are those the export resolves, a model folder's own (`./`) and the
   team's Common; a game directory's are kept as written, the file there being the game's own
   (`model/character/uniform/texture/u0814p1.dds` is team 814's compiled kit texture; no real
   export names one, 176 references in one model-test folder do). A Common-path reference
   whose respelled name is a `kitN` reference (`common/XXX/u0XXXp0.dds`, `Common/` holding
   `u0XXXp1.dds` to `p3`: /vg/'s Autumn 24 export) links like any other, the link named after
   the reference's respelled name (`kitN.dds.common`) and present when `Common/`'s directory
   holds a member of its set, as the shared validation resolves such a link. A name segment
   holding a legacy `p` token and also a Studio token of the author's (`kit1_u0827p0.dds`, five
   VGL26 exports) has that token's `k` written `K` (`Kit1_kitN.dds`, `Common/Kit1_kit1.dds`), in
   names and references alike: a stem's first token is its kit token, so `kit1_kitN` would read
   as variant 1 and its set would never link. In a
   `face.xml`, only the `path` and `material` attributes' values change, the rest of the file
   byte for byte as written, `<dif>` included; their `???` stays, the compiler respelling any
   three characters there. The upgrader is the **only** Studio component that reads the legacy spelling — the
   compiler has no notion of it. `dummy_kit*` references are left as they are — the compiler
   keeps them as reserved game-substituted names; a `.mtl`'s `dummy_` stem in a Common path
   (`model/character/uniform/common/XXX/dummy_kit.dds`, the real /jp/ Summer 25 export's
   `XXX15 - Sister`) is looked up nowhere and reported never, its `???` alone respelled, since
   the compiler emits a member's `.mtl` as written. The goalkeeper token **`u0???g`**
   (`u0XXXg1.dds` in a `Common/`, which a keeper's model names) is no variant set in Studio,
   which has one kit number for every player ("Kit-dependent assets"): in a whole-export upgrade
   its three ID characters become the resolved team's ID, in file and folder names and in
   `.mtl`/`face.xml` paths and FMDL path tables alike, as Red respells it, so a model and a file
   that spelled the ID differently (`XXX` in one, the ID in the other) match again. Model-folder
   mode has no team and leaves the token as written. Pre-Fox **model-type prefixes become suffixes**:
   Red typed `face.xml` entries by a leading type (`gloveL_foo.model`, matched case-insensitively
   with underscores ignored, or a `model_type_<x>` token anywhere); the Studio format puts the type
   last (`foo_gloveL.model`, `foo_model_type_<x>.model` — see "Model names: a free part plus a
   suffix" in the [Aesthetics export plan](aesthetics_export/README.md)). The swap applies **only to folders
   without a `face.xml`** — filename typing exists to generate the xml, so a folder that ships its
   own xml carries the types there and its files are left exactly as named. In xml-less folders (a
   player folder with no `face.xml` of its own, its subfolders included, and a shared boots or
   gloves folder, which validation refuses a model of when its name does not end in its type:
   `oral_boots_win32.model` is `model_name_invalid`, `boots.model` is not) and in `Common/`, the
   upgrader moves a recognized leading type to the end of the stem: one of Red's types
   (`face_neck`, `handL`, `handR`, `gloveL`, `gloveR`, `uniform`, `shirt`, `pants_nocloth`, `eye`,
   `mouth`, tried in that order) followed by `_` and more of the name (`gloveL_foo` →
   `foo_gloveL`, `glove_l_foo` → `foo_glove_l`, the type kept as written). A type glued to the
   next word is no type here (`eyebrow_x` stays, where Red typed it `eye`): none of the 856
   models in the xml-less face folders of the maintainer's exports is named so, and moving
   letters off a word makes a name nobody wrote. A name holding a `model_type_` token is left as
   written: Red and the compiler both read everything after the token as the type, so it is in
   place wherever it stands, and moving it would change the type. A name with no leading type is
   untouched, one already ending in a type included, and so is a name that is a type alone
   (`gloveL.model`). It also drops the `oral_` prefix
   and `_win32` suffix from a model named with both (`oral_parts_body_win32.model` →
   `parts_body.model`), in `Common/` too, a link of that name following its target
   (`oral_x_win32.model.common` → `x.model.common`): the compiler adds them back to the name it packs, and Red matched a
   `.mtl` against the name without them (its `xml_editing.py`), which the compiler's search,
   matching the literal stem, would not (`/jp/`'s Winter 2025 `XXX17 - Dekinai` holds that
   model beside `parts_body.mtl` and `materials.mtl`). Each renamed model keeps the `.mtl`
   Red chose for it: in the type swap, a `.mtl` in the model's folder named after the old stem
   alone (the stem the swap starts from, compared case-folded) is renamed with
   it (`gloveL_foo.mtl` → `foo_gloveL.mtl`), since the search matches a stem that starts or
   ends the model's ("Pre-Fox: the `.mtl` a `.model` uses" in `spec/model_format.md`); the
   wrapper drop renames no `.mtl`, Red having matched against the unwrapped name already. A
   `.mtl.common` link is not renamed with a model of its own folder: it follows its target in
   `Common/`, keeping its name unless that `.mtl` is renamed with a `Common/` model, since a link
   to a name that moved would name a file that is not there. In
   either rename, a `.model.common` link follows its target's new name (`gloveL_foo.model.common` →
   `foo_gloveL.model.common` with `Common/gloveL_foo.model` → `foo_gloveL.model`). A renamed
   output that an earlier output already takes (compared case-folded) is left out and its input
   reported `unmigrated_content`, as for the kit magic's renames. A `Common/`
   file that a member `face.xml` names (a `path` or `material` value below
   `model/character/uniform/common/<segment>/`, the file name's `*` read as `win32`, as the
   compiler reads it) keeps its name through both renames, and so does one that a link in a
   folder with a `face.xml` names, that link being left as written: the xml's `path` is
   packed as written, and on a PES 16 target one without the `oral_` prefix is refused
   (`xml_oral_prefix_missing`, `spec/team_compiler/messages.md`), so following the rename would
   lose the face folder where keeping the name loses nothing (the real `/o/` Winter 18 export's
   `73602 - Don't panic/face.xml` names `Common/oral_hand_l_win32.model`).
8. **Generates `settings.toml`** for each player folder steps 3 and 4 write (a rostered player
   with no face folder and no custom boots or gloves gets no folder, and his look stays where
   the game reads it now, his record or his installed row), populated from the savefile's aesthetic data
   (physique, strip style, skin/iris color, etc.) — the same TOML-generation logic the save editor
   uses. A marked folder's `ingame_face.toml` is written beside it, filled the same way: the face
   run, the hair and the iris, every key the save's version holds (`settings_value_refused` and
   `settings_from_stripped_record` cover both files); a folder without the marker gets a
   `settings.toml` alone, without those keys, and without a record to generate from (a failed
   `settings_not_generated`) the file is written empty, so the marker's effect survives.
   Custom boots/gloves IDs (above 100) are omitted: old IDs help resolve the wearers during
   migration, but the migrated models/links determine Studio's automatic assignments. A stock ID
   (1 to 100: a game model, which no folder carries) is kept as `boots_id`/`gloves_id`, or the
   player would lose it. A stored 0 is left unset, its default, except in a Midcup output, where
   it is written (`boots_id = 0`): an unset key there keeps the installed ID
   (`spec/aesthetics_export/settings_toml.md`), which may not be the default his record holds;
   a Full output keeps it unset, since an explicit 0 would also override the hide preset an
   author who switches to the `fpc_on` marker expects. A player step 3 marks `fpc_on` (a
   modelless boots or gloves folder) is the exception: the category no folder of his decides
   keeps a stored 0 as an explicit `= 0`, since under the marker an unset key gives the hide
   preset's blank-model ID and he would lose the stock boots or the hands he wore (EU-PLY-17). An ID a migrated boots or gloves folder
   claims (his worn ID, merged into his folder or linked from it) is left out whatever its
   value, a stock-band one included: the folder decides that category, and a number beside it
   would be ignored at every compile (`settings_model_id_conflict`). A custom ID worn by a player that no model folder of the export claims is
   reported (step 10): the content is missing, and writing the number would point at whatever
   Studio's block scheme later assigns there. It is `custom_model_missing` (E, the slot, the
   player's name, the key and the ID), checked for every rostered player of a Full output but
   in a Midcup output only for one the output gives a folder: a Midcup compile leaves a player
   without a folder as installed, his content included, while a Full one rebuilds the team from
   the export alone. Since name writing is opt-in, the upgrader emits `name = true` when the
   player name the compiler derives from the output folder's name (`aesthetics_export`'s rule,
   the name part of `NN - Name`) equals the savefile name, or the explicit string otherwise — which is the case whenever the
   folder was named from the face folder rather than the savefile, and whenever the savefile name
   can't be expressed in a folder name (filesystem-invalid characters, **including name colour
   codes**, which the explicit string preserves byte-for-byte so a compile never strips them).
   A stored value is written as stored, since the player must look the same; real saves hold
   values past the ranges assumed for versions no game measured (79 players of the PES 18
   fixture save, `cheek_type` 4 or 5 among them), and those ranges become the fields' bits
   ("Player settings in exports" in the [Aesthetics export plan](aesthetics_export/settings_toml.md)).
   A value past even a key's widest range cannot be written, the parser refusing it (a PES 16
   cup player stores `goal_celebration_2` 183, `spec/aesthetics_export/settings_toml.md`): that
   key is left out of his file and reported (W, `settings_value_refused`: the slot, the
   player's name, the dotted key and the stored value). A record whose file cannot be generated
   at all (a field read that fails) leaves his folder without one, reported
   `settings_not_generated` (E, the slot, the name and the error). On a stripped Fox save
   (`spec/save_editor.md` "Stripped save") the save editor's generation skips a player at -1,
   his record not being his look; the upgrader generates from the record all the same and names
   him in the report (I, `settings_from_stripped_record`, the slot and the name): strip-and-seed seeded his rows from those bytes, and the compiler,
   which does not read the AET layout, has compiled nothing of his team since, so the record
   still is his look. That holds until the team's first Studio compile, which the upgrader
   cannot see: from then on his look is the compiled export's `settings.toml` (an edit of
   his record in the save does not change it, `spec/save_editor.md`), so the
   report's line says the file is generated from a stripped record and that a team compiled
   by Studio since keeps its own file instead. No `fpc_on` marker is written from the record
   (step 3's modelless item folder writes one from the folder): an FPC player's record holds the hide strip
   and either the stock hide IDs, kept as any stock ID is, or, as the cup's FPC teams mostly do,
   a blank boots ID of his own whose folder migrates like any other (the 2023 Winter Cup save's
   `/a/`: boots and gloves 126 to 148, one per player), so the raw settings reproduce him, while
   matching them against `libs/fpc`'s enable preset would miss those players and mark a dressed
   player with long sleeves, tucked shirt and short socks. The author switches to the marker by
   hand.
9. **Migrates referee exports**, those whose name's first word is `refs` (in any letter case;
   `refs.txt` and the referee subfolders below are matched so too, as real exports spell them
   `Refs.txt`, `Face/`; a `refs` export without a `refs.txt` holds no roster to migrate and is
   reported, nothing written, exit 1: `refs_txt_missing`, E, on the export; the roster file's
   older spelling `Refs list.txt`, the refs compiler's, is read when there is no `refs.txt`, and
   that layout's referee folders stand directly at the export root instead of under `Players/`:
   when the export has no `Players/`, every root folder step 1 does not know is a referee's,
   since four of the eleven real referee exports on the maintainer's machine, Winter to Summer
   25, are that layout, with the same `Face/` and `Common/` per referee), as the compiler tells
   them apart ("Identity boundary" in `spec/aesthetics_export/object_model.md`): the old referee layout (`refs.txt` + per-referee
   face/boots/gloves/common subfolders — the prototype of the player-folder format) is refused by
   the compiler per referee folder (`player_layout_proto`, naming this tool; only the `refs.txt`
   alias is accepted), so it is upgraded to the fully unified layout: `refs.txt` becomes a root `players.txt`
   carrying the slot mapping (including repeated entries — the same model backing multiple of the 35
   referee slots for rarity control) in `aesthetics_export`'s canonical UTF-8/LF roster grammar shared with
   the Refs arranger (every nonblank line, a line that parses written as its slot in two digits,
   one space and the folder name, one that does not kept as written, so the shared validation
   reports it on the output rather than the upgrader deciding what it meant; a `refs.txt` that is
   not UTF-8 is unmigrated content and no `players.txt` is written, the validation then dropping
   the output for the missing roster), and the `refs.txt` is removed; a root `ref_marker.dds` is carried as it is
   (the format admits it, `spec/aesthetics_export/object_model.md`); each referee's subfolders are **flattened** into
   the player folder root (step 12). Either old `ingame_face` spelling becomes `ingame_face.toml`
   exactly as for team player folders — empty, a referee export having no record to fill it from.
   No savefile needed — referee IDs are compiler-generated. A Note txt in a
   referee export is a leftover (step 10): step 6 reads a team's colors, which referees have none
   of, so nothing compiled it, and the Warning names it for the author.
10. **Reports** anything that couldn't be migrated automatically (e.g. contents of the Other folder,
    unassigned model folders) for manual attention — the `UnmigratedContentFound` pipeline event;
    the tool defines its own message catalog on the shared `Message` structure (see the Team
    compiler plan's "Message catalog"). Content is reported entry by entry: where Red discarded a
    whole folder for one bad entry (every `Kit Configs/` config when one name breaks the
    grammar, as the real `CIV4XG` VGL26 export's `XXX_DEF_GK2nd_realUni.bin` does; `Kit
    Textures/` for one failing texture; `Logo/` when it holds other than three emblems), the
    other entries migrate and the bad one is reported, since the author can then fix one file
    rather than lose the team's kits silently, as Red's compile did. An entry is a file, or a
    folder no step migrates as a whole (a second face folder of one slot, one without a slot),
    reported once by its path; a file a step leaves in the output as named (a second portrait)
    is reported all the same.
    Two kinds of entry, since only one keeps the run from being clean ("Verification"): a
    **leftover**, content that never reached a player in game, is dropped with a Warning naming
    it (`leftover_dropped`). It is content outside the old format's layout that nothing
    compiled (a top-level folder the AET does not define, such as the real KFG VGL26 export's
    `FMDL Boots/` of `.model` copies; a file at the root other than the Note; `Other/`'s files,
    which the AET keeps for the author alone), or content inside it that Red never read or that
    the game never loaded: a file directly in `Faces/`, `Boots/` or `Gloves/`, where Red's walks
    list subfolders only (`Faces/73606 - Civic cool.7z`, `Boots/boots.fmdl`: archives made to
    share model folders, removed before a compile); a `Full` export's kit texture whose slot no
    config and no Note entry gives, and in any export a Kit Textures file that is no kit
    texture of a given slot (step 5: `u0XXXp2 - Copy.dds`, `back.dds`, `u0XXXp1.png`); a
    portrait Red's check refused (step 7); a file the old compile's ID rename replaced (step 7:
    `u0731p2.dds` beside `u0XXXp2.dds` in one folder); the second color of a team-color entry holding two
    (Red kept the first); an item folder no save record names (VG Summer 20's misspelled
    second set `k2501`-`k2523`); a second or later goalkeeper kit (step 5: no version the cup
    plays shows one); and two kinds of stray file, which the compile would otherwise refuse a
    folder for or misread: a file the shared `classify` reads as no known kind and that is no
    `.txt` note, wherever it sits in the old format's folders (a model folder, `Common/`,
    `Collars/`, `Kit Configs/`, `Logo/`: `face.mtl.backup`, `materials.mtl.bak`, M Winter 18 and
    two other exports; `file_type_disallowed` or `common_file_disallowed`, Errors under the
    default `strict_file_type_check`; Red's `common_check` deleted such a file and went on, its
    other checks discarded the folder), and in a Fox boots or gloves folder an FMDL under no
    name the game loads, Red's names being `boots`, `glove_l` and `glove_r`, each alone or after a
    prefix and `_` (`boots - Copy.fmdl` beside `boots.fmdl`), which in the wearer's player
    folder would be merged into his hair (`fmdl_fcl_hair_fallback`). Red discards the whole
    folder over such a file ("is not allowed", "is not a valid fmdl name for fox mode"); we
    drop the file and keep the folder, the member's content, which the stray copy never
    belonged to. A file of a known kind that sits where the format refuses it (a `face.xml`
    in a subfolder, EU-REP-02) is no stray: it is left for the output's strict check to name,
    since it is the member's content in the wrong place, not a copy. An entry inside the format's folders that reached the game but that no step migrates
    (a texture that does not decode, a folder loose mode cannot place) is
    `unmigrated_content`, an Error. So is content Red kept out that the author evidently meant
    for the game, where a leftover would lose it over a slip: two candidates for one place, of
    which only the author knows the meant one (a second face folder of one slot, a second item
    folder of one ID, two kit textures landing on one name, two portraits in one face folder,
    two old `Portraits/` files of one slot), Red's pick being a refusal or arbitrary, and a
    whole model folder under a name Red refused (a face folder without a valid slot, a `K0430`
    folder), which would drop a player's face over a typo. A single file under a name Red's
    per-file name checks refused (`portraits_check`, `kittextures_check`) is a leftover; a
    model folder's file of a known kind under a name the format refuses (a `face_edithair.xml`,
    a model's name) is not, the author's content in the wrong form, which the output's check or
    the compile names. A team color's second color is no such choice, Red always keeping the
    first. We do not refuse a run over a leftover,
    because the game never showed it, so the Studio export is complete without it and the
    Warning tells the author what was left behind; we do refuse one over an unmigrated entry,
    because the compile would miss what the old export compiled.
    Each `unmigrated_content` names its cause in a `reason`, one value of a closed set per
    cause (`slot_taken`, `id_invalid`, `texture_undecodable`, `loose_unplaceable`, ...: the
    crate's `UnmigratedReason`, about thirty), and, where two candidates claim one place, the
    other's input path in `other`: one code covers every cause, and a manager whose upgrade is
    not clean, the one outcome where the input must be edited, could not otherwise tell from
    the line what to fix. A closed set rather than free text, because the message text
    (Phase 8) branches on it, a sentence per cause. Every site builds the finding through one
    constructor that requires the reason, so a new site cannot leave it out.
11. **Loose mode** (no savefile available): step 2 needs the savefile only to learn which player
    wears which old boots/gloves ID. When no savefile is given, the upgrader offers loose mode,
    which infers the wearer from the **ID's position inside the team's Red block** instead —
    exports being upgraded were made for the cup's per-team ranges of **25** IDs (`101 + 25 × n`,
    the "reserved set of ID slots" the AET page links to), not Studio's 40-ID blocks, so the block is inferred from the lowest
    folder ID of each kind on its own, the boots folders' from the lowest `k` ID and the gloves
    folders' from the lowest `g` ID (`range_start = 101 + 25 × ⌊(min_id − 101) / 25⌋`; a lowest
    folder of 0128 still infers the 126 block): the two kinds are numbered apart, and one
    kind's numbering cannot move the other's pairings. `k0126` → player 01, `k0128` → player 03. IDs on
    the block's spare positions (24–25), and IDs past the block, belong to nobody: such a folder
    is unmigrated content as a whole (step 10), unlike one no save record names (step 3, a
    leftover): without a save the pairing is a guess, so a folder it cannot place may be
    anyone's. It is reported rather than paired by its position within its own block (the ID modulo 25): an
    ID in another block is in another team's range, so its folder may be anyone's, and the author
    places it by hand or upgrades with the save (the real GBFG VGL26 export numbers two gloves
    folders `g2522` and `g2523` beside `g2301` to `g2320`, the same players' boots being `k2322`
    and `k2323`: loose mode reports the two). An ID on a slot
    without a face folder still yields a player folder with an `ingame_face.toml`, as in step 3.
    Every other ID maps to exactly one slot, so its folder is always merged in on a Fox export
    (unless a byte clash keeps it shared, `merge_kept_shared`),
    and kept shared with a link on a pre-Fox one (step 3). Loose mode is a documented
    assumption, never a cross-check: it is offered only when the savefile is absent, the report
    states each assumed pairing (I, `wearer_assumed`, on the item folder, its four-digit ID and
    the slot in its context: `id=0128, slot=03`), and no `settings.toml` aesthetic data can be generated (step 8 is
    skipped, apart from `name` handling, which needs no savefile: a folder with a name part gets a
    `settings.toml` holding `name = true` alone, one named by its number alone none), every
    `ingame_face.toml` written empty, the bare marker's effect. Red's Pre-Studio upgrader
    implements this same mode (its `export_upgrade.py`).
12. **Flattens the AET referee layout's subfolders**: `face/`, `boots/`, `gloves/` and `common/`
    directly inside a referee's player folder (the layout the compiler refuses with
    `player_layout_proto`, `spec/aesthetics_export/player_folders.md` "Subfolders") are folded into the player folder
    root; no file is renamed (the census of 2026-10-10 found every such `boots/` holding files
    named as the root would name them: `boots.fmdl`, `boots.model`, `oral_boots_win32.model`).
    Each folder's subfolders keep their place below the root (`face/kit/x` becomes `kit/x`),
    save one that would land as a `face/`, `boots/`, `gloves/` or `common/` subfolder of the root,
    the refused layout again (the real `refs for Autumn Q 25 Day 1` referee `galosengen` holds
    `Face/face/`): it is reported as unmigrated content and the referee folder left as it was.
    A file name present in two locations with identical bytes is deduplicated; with different
    bytes it is reported as unmigrated content (step 10) and the folder is left as it was, for
    the author to resolve, and so is one stem two files share with different extensions
    (`texture_stem_conflict` once folded) when the two come from different locations, the
    folder's root counting as one: that conflict is the fold's making, while one already inside
    a location is the compile's to report as for any folder. A folder left as it was is copied
    at its old paths, and step 7's renames and rewrites skip it: it is the author's to repair by
    hand, and names moved under him would hide what the report named. A model of `common/` that no reference of the referee
    names (a `face.xml` of his, wherever it stands in his folder, whose `path` value's file
    name, its `*` read as `win32`, is the model's, letter case aside) is reported as unmigrated content and left out: Red kept `common/` in the referee's
    own common directory, where a model loads only when a reference names it, while at the root
    it would become a part of the face (the real `galosengen` holds such a
    `Common/parts_body.model`). References follow the files: in a referee's `.mtl`, `face.xml` or
    FMDL, a `./x` or `common/???/x` reference whose file the flattened folder holds at that
    path (from its `common/`, or from any other of its subfolders: the compiler looks a folder's
    own file up before a game path either way, so one rule covers both) names it
    where the flatten puts it (the bare stem, or `./<sub>/x` for a file in a subfolder of
    `common/`), with no link and no finding (from a `.mtl` or FMDL in a subfolder of his, a
    reference into a `common/` subfolder follows step 7's subfolder rule instead: `./<sub>`
    from there would name a path below that subfolder), and a `./x` whose file is in neither the referee's subfolders nor its `common/`
    but in the export's `Common/` gets step 7's stem and link when it is an `.mtl`'s or an
    FMDL's texture (a `face.xml`'s `./x` is left as written: the compiler resolves a `path`
    among the folder's own files, no link stands in for a model there, and it reports the miss
    itself, `xml_model_not_found`; every `./` value of the 69 real referee `face.xml` files on
    the maintainer's machine names a file of its own face folder, and their 64 Common-path
    values name the export's `Common/refkit/`, which step 7 keeps as written); the referee's `common/` is checked
    before the export's `Common/`, as Red's referee pass checks them (`referee_tools.py`
    `update_file_paths`, which pointed them at the referee's own common directory), and step 7's
    other rules apply to the rest. The 15 referee exports on the maintainer's machine hold 693
    `common/???/` references into a referee's own `common/`, which step 7's Common rule alone
    would report and break. Any other subfolder of a player folder, in a team export or a referee
    export, is a player folder of its own under the Studio rule and is carried over unchanged.
13. **Optional glTF conversion** (`--convert-to-gltf`): not built (Phase 7); its design is
    `plans/export_upgrader.md` "What it does", step 13.

## Model-folder mode

The second input shape, alongside whole exports: **a folder of old-format model folders** — the
item folders that used to live under `Faces/`, `Boots/`, `Gloves/` (`XXX15 - Snuffy`,
`k0915 - Crocs`, …), placed loose in one directory, optionally with a `Common/` beside them. This
is what the community's export converters accept in their "player folders" mode, and it exists
for the same reason here: once a team's export has been upgraded, new models keep arriving in the
old spelling (a face made in an old workflow, a boots folder from a previous cup), and the
kit-dependent magic in particular (`u0???pN` → `kitN`) is not something an author should hand-edit
inside FMDL path tables. The user upgrades the folder and drops the results into their Studio
export.

Each top-level folder is migrated as **one model folder** with the per-folder rules of step 7 —
`kitN` rewriting in names and path tables, `.mtl` stem normalization, pre-Fox type prefix → suffix
and the `oral_`/`_win32` drop with their `.mtl` rule (xml-less folders only), `ingame_face`/`ingame_face.txt` → `ingame_face.toml` normalization, `.mtl.common` → kept as is (a `Common/` in the
input is upgraded with the Common rules and carried over, so the links keep resolving). Output
naming follows steps 3–4 so the result is directly droppable: a face folder becomes a player folder
`NN - Name/` (NN from its `XXX` suffix, Name from its name part; without a suffix, the name part
alone); a boots/gloves folder becomes a shared-style folder named by its name part (`Crocs/`), or
by its old ID when it has no name — the user drops it into `Boots/`/`Gloves/` and adds the link
file, or moves its contents into a player folder (renaming the model to `boots.fmdl` /
`glove_*.fmdl` as the Studio format expects). The report lists, per output folder, where it goes
(`model_folder_destination`, I, on the output folder, `into` = `Players`, `Boots`, `Gloves`, or
`root` for `Common/`).
The shared validation ("Verification") runs on the output as it would sit once dropped in,
not on the published layout, which no export reads as player or shared folders: the upgrader
validates a listing that places each player folder under `Players/`, each boots or gloves
folder under `Boots/` or `Gloves/` and the upgraded `Common/` beside them, as a Midcup export
(which needs no kits), and reports the findings against the published folders. The findings
about where a folder will sit rather than what it holds are left out, since the mode has no
destination export: the identity findings (the listing is named `models Midcup`, so none
arises), the roster's numbering (`player_folder_number_invalid`, `player_number_duplicate`: the
folder's number, and its siblings', are the destination's, and a `players.txt` there may name a
folder without one) and `shared_folder_orphaned` (the link naming a boots or gloves folder is
the user's to add). We do not keep them, because every boots or gloves folder would then drop
(`shared_folder_orphaned` is a `DropFolder`) and no run holding one could exit 0. The listing's
version is "Verification"'s: PES 16 when the output holds a `.model` file, PES 21 otherwise.

The input is read as it is: a folder (no archive), its top level the model folders, so step 1's
root rules (the one wrapper folder, the same-named subfolder) do not apply: they would read a
folder holding one model folder as that model folder. A top-level folder's kind is its name's:
`Common` in any letter case; a boots or gloves folder by steps 3-4's item grammar (`k` or `g`,
lower case, four digits, then the name part; `g` and three digits); any other name a face
folder. A face folder's name is step 3's: a slot (characters 4-5 making 01 to 23) gives `NN -
Name`, or `NN` alone with no name part; without one the folder keeps its own name (`Snuffy`).
Every name is cleaned as step 3 cleans a candidate. In a face folder only the `ingame_face`
spellings are normalized (either becomes `ingame_face.toml`); its other files keep their names,
the portrait included (no `Portraits/` to compare it with). Two folders whose output names match, letter case aside (`XXX15 - A` and `71415 - A`,
or a face folder `Crocs` beside `k0430 - Crocs`, both published at the output's top level), keep
the first by input name; the later is unmigrated content as a whole folder (`DropFolder`), as
step 3's second face folder of a slot. A file directly in the input folder is unmigrated content
(step 10).

Not in this mode: kits, portraits, logo, `settings.toml`, the savefile, loose mode, the roster. No
savefile is read — there are no wearers to resolve. A top-level folder that holds no file directly
in it (OS artifacts aside; `Common/` included, an empty one having nothing to carry) is reported
as unmigrated content (step 10) and left out; the converters'
two-level `players/<player>/<model folders>/` layout is therefore *not* accepted — one level, one
folder per model, so that "what is a model folder" never has to be guessed. We do not ask for a
model file directly in it, because a real Fox face folder may carry no model of its own (5 of the
23 face folders of the real /jp/ 2024 Winter Cup export hold `face_diff.bin` and a portrait
only), and `upgrade` carries such a folder: the two modes agree on what a face folder is.

Verification: fixtures with one face folder carrying `u0XXXp1` textures and path-table references
(asserting `kit1` everywhere, folder renamed `NN - Name`), one boots folder with a prefix-typed
pre-Fox model (asserting the suffix form and the `Crocs/` output name), and a `Common/` with an
`.mtl` whose references stay as written (step 7: a `Common/` `.mtl` keeps them, extension
included) while the player folder's `.mtl.common` link still
resolves against the migrated Common through `aesthetics_export`'s validation; plus one player
`.mtl` referencing `model/character/uniform/common/XXX/hair.dds` (asserting `./hair.dds` in the
migrated `.mtl`, a `hair.dds.common` link beside it, and the stem resolving into Common through
validation) and one such reference whose file is absent from `Common/` (asserting it is reported
and left unchanged).

## CLI

```text
UTW export-upgrader upgrade <export> [--savefile <EDIT> | --loose] [--coverage full|midcup] [-o <dir>]
UTW export-upgrader upgrade-models <folder> [-o <dir>]
```

`upgrade` takes one old export (folder, `.zip` or `.7z`); the output is a folder beside the
input, or inside the directory `-o` names (created when missing), as `upgrade-models` writes
into its `-o`. Its name,
wherever it is written, is the one the compiler reads a team export by: the resolved team's key
(step 6), then the coverage tag (`Midcup` when the old name holds `midcup` or `additions` in any
letter case, the two ways old midcup exports were named; `Full` otherwise; the Aesthetics export
plan's "Coverage tag"), then the rest of the old name (`co - Spring` → `co Full - Spring`). An old
name whose first word is not the key keeps that word in the rest (`Numbers` resolved from its
Note to `/co/` → `co Full Numbers`), because the compiler takes the team from the first word
alone; one whose second word is already `Full` or `Midcup` keeps it rather than doubling it; a
referee export takes no tag. A name equal to the export's own (a folder's name, an archive's
stem), letter case aside, gets ` upgraded` appended (`refs Spring` → `refs Spring upgraded`), so
the input is never written into: Windows names ignore case, so `CO Full Spring`'s output
`co Full Spring` would be the input folder itself, and an archive's output takes the same name its folder form
would. Every `upgrade` of a team export reports the tag it used, an Info
`coverage_tag_chosen` with the `tag` and its `source`, the first of these that holds: `name`
when the old name's second word is already `Full` or `Midcup` in any letter case; `flag`;
`midcup` or `additions`, the text the name was found to hold (a substring, the first in the
name if both); `default` when it holds neither. `--coverage full|midcup` sets the tag in
place of the name's guess, which stays the default: a manager upgrading an old mid-cup export
named neither way ("co day 5 fix") would get `Full`, whose first compile rebuilds the team from
the export alone, everything it does not carry becoming the default or a placeholder, and the
Info says so before that compile. A flag naming the other tag than a second word the name
already carries is refused (exit 2): the member wrote that word, and renaming it under them
would surprise more than the refusal. A referee export takes no tag and refuses the flag
(exit 2). An output folder
that already exists is refused, naming it: replacing it would lose edits made since an earlier
upgrade. A team export takes `--savefile` or `--loose`, exactly one: loose mode is an assumption
the user opts into (step 11), never a silent fallback; a referee export takes neither, and is refused with either. A
savefile that does not hold the resolved team is refused naming the team; an export whose team
resolves neither from its Note nor from its name is reported (`team_name_unknown`, the
compiler's code) and nothing is written. `upgrade-models` is model-folder mode; its output is one
folder per migrated input folder under the output directory, by default a sibling folder named
after the input with ` upgraded` appended, or the directory `-o` names. That directory is the
output folder, refused like `upgrade`'s when it exists: we do not write into an existing one,
because an earlier run's folders, edited since, would sit beside this run's with nothing telling
them apart. Both print step 10's report, one line per finding
naming its code and, for a finding about a file or folder, its path (the input's, or the
output's for a check run on the output), and exit with the Team compiler's mapping
(`spec/team_compiler/settings.md` "CLI"): **0** clean (Warning and Info findings allowed when
they drop nothing but a leftover, step 10), **1** not clean (unmigrated content, an Error
finding, a finding that drops something else, the unknown team, or an archive that does not
extract, reported `export_extract_failed` as the compiler does; nothing is written,
"Verification"), **2** invalid invocation (a missing input path, an
`upgrade` input that is a file other than a `.zip` or `.7z`, an `upgrade-models` input that is
not a folder, the flags above, a savefile that
does not read as a save or lacks the team; nothing is written), **3** aborted: before anything
is written (the output folder exists, or it or its staging folder below cannot be created; the
data directory's `teams_list.txt` exists but does not read or parse, step 6), or when a file
of the output cannot be written, the error naming it and the incomplete staging folder. Both
commands write into a **staging folder**, the output's name with `.upgrading` appended, beside
where the output goes; the shared validation reads it there, and the verdict then renames it
to the output when the run is clean or removes it when not ("Verification": nothing is
written). A staging folder an earlier run left (a write that failed, a rename or removal the
verdict could not finish) is removed by the next run of the same output, reported as an Info
`staging_folder_removed` naming it, rather than aborting: a staging folder is never a finished
output, and the member who meets it never made it. A removal that fails (on Windows a file of
it held open, by another run writing it at that moment) aborts as above (exit 3), naming the
folder; a second upgrade of one export started while the first writes, the one way to meet a
live staging folder, then loses at most that run, never a finished output. We
stage rather than validate an in-memory listing because the validation reads files from disk,
as the compile does, and a rename is one step: no reader ever sees a half-written output under
the output's name. The rename and the removal retry for about two seconds while Windows
refuses them with "access denied", then abort: a folder whose files were just written can stay
open for a moment in another process (an antivirus scan), which the crate's test suite met in
about one run in three, and a member whose long run ended clean should not lose it
to a lock that clears by itself. `--convert-to-gltf` is step 13's, not built (Phase 7): it is
an unknown argument (exit 2). The view over both commands is not built either (Phase 8,
`plans/export_upgrader.md` "GUI").

## GUI

Not built (Phase 8): the view over both commands is `plans/export_upgrader.md` "GUI".

## Message catalog

Tool id `export-upgrader`. Each step names the codes it reports where it describes them; this
table holds the ones no single step owns. The report names two exports: the input, by its name
(step 1), for what is found in it, and the output, by its folder name, for the shared
validation's findings on what was written.

| Code | Sev | When | Context |
|---|---|---|---|
| `no_old_export` | E | step 1: the input holds nothing an old export holds; nothing is written | — |
| `export_extract_failed` | E | the archive does not open or its listing is refused (the compiler's code); nothing is written | `path`, `error` |
| `source_read_failed` | E | a file of the input does not read after its listing did (the compiler's code); it is not migrated and the run reports on and ends not clean, so nothing is written; an unreadable Note (it decides the team) or a referee export's `refs.txt` (it decides the roster) stops the run at once | `path`, `error` |
| `team_name_unknown` | E | the team resolves to no row of the list (the compiler's code), the `Team:` line's file or the export named; nothing is written | `name` |
| `unmigrated_content` | E | an entry inside the old format's folders that reached the game and that no step migrates, or that step 10's boundary keeps an Error (two candidates for one place, a misnamed whole folder), entry by entry; a Note entry whose colors do not read is one, its line in the context, and so is a kit config that does not decode (step 5), its error in the context | `reason` (the cause), `other` (the other candidate's input path, where two claim one place), then `line` (a Note entry) or `error` (a kit config, a texture, an `.mtl`) |
| `migration_clean` | I | last line: no Error finding, and no finding drops anything but a leftover ("Verification"'s clean migrated export); exit 0 | — |
| `migration_not_clean` | I | last line otherwise: nothing is written ("Verification"); exit 1 after any finding that drops anything but a leftover | — |
| `leftover_dropped` | W | step 10: content that never reached a player in game, outside the old format's layout or inside it (a refused portrait, a stray file, a second goalkeeper kit, ...), not written; a Note entry's dropped part names its line; the run stays clean | `line` (a Note entry) |
| `mtl_stray_text_dropped` | I | step 7: an old `.mtl` held text between its elements, dropped as the file is written back | — |
| `coverage_tag_chosen` | I | `upgrade` of a team export: the coverage tag the output's name took ("CLI") | `tag`, `source` |
| `staging_folder_removed` | I | an earlier run's staging folder for this output removed before the write ("CLI") | `path` |

The shared validation's codes (`aesthetics_export::ISSUE_CODES`) keep the Team compiler's
severity (`spec/team_compiler/messages.md` "Catalog"), each run-dependent one taken as for a
compile that deploys with pass-through and the strict file-type check on (the output's check
runs with it, "Verification", so `file_type_disallowed` and `common_file_disallowed` are
Errors), and an `E/F` one as
an Error, since the run it would abort is the upgrade of this one export, already written: the
output is judged as its first compile will judge it.

## Dependencies

Uses `aesthetics_export` (the Studio format's object model, conventions, and validation — the upgrader builds
and verifies its output with the same crate the compiler consumes it with), `pes_savefile` (savefile
reading, TOML generation), `archives` (old export loading), `vtree`, `kit_config` (binary → TOML kit
config conversion), `dds_convert` (the logo comparison's decoding and resampling, step 7, the
compiler's own resampler so the test judges what the compile would regenerate), the format
lib crates for content inspection, and `roxmltree` (already the workspace's XML reader, the
compiler's for a member `face.xml`) for the `face.xml` attribute values step 7 respells. The
format lib crates it inspects content with are `fmdl` and `pes_model` (the models step 7
renames and whose texture paths it rewrites; `pes_model`'s tolerant `.mtl` read is the
upgrader's alone), `ftex` (a kit texture's FTEX decoded, step 5) and `wezlib` (a wrapped
`face.xml` read, step 7); `teams_list` resolves the team (step 6), and `studio_core`,
`pes_version`, `clap`, `anyhow`, `toml` (the settings table) and `egui` are the tool-crate
skeleton's (`core/architecture.md` "Crate structure"). Old-format knowledge stays in
this tool. The structural migration (steps 1–12) does no model-format conversion, but does rewrite
text metadata, kit configs, and legacy MTL texture references. `model_convert` is not a
dependency: only step 13, the glTF pass, needs it (`plans/export_upgrader.md`).

## Verification

For each migration the upgrader lists the staging folder it wrote ("CLI") into canonical
`vtree::ScopePath` entries plus the small metadata files read from it, parses that listing back through
`aesthetics_export::parse_listing` as `ParsedAestheticsExport`, runs the shared validation, and reports every
scoped issue. The validation is for the savefile's version (in loose mode, PES 21, or PES 16 when
the export holds a `.model` file outside its leftovers, step 10: old exports are single-engine,
and the version decides only which model names are allowed; a leftover does not decide it,
because the real KFG VGL26 export, a Fox one, keeps `.model` copies of its boots in a side
folder `FMDL Boots/`, and counted they made it migrate as pre-Fox, where item folders never
merge, its 23 single-wearer boots folders kept shared past the block's 17 shared IDs) in a
fixed context, not the compiler's settings: no
pass-through, the strict file-type check on, as the compiler's default has it, no installed
Common textures. The strict check is what holds the clean-output promise for a file type no
step expected: off, a disallowed file was an Info and the output written, and the compile then
refused the player's folder (6.12c: the nested `face.xml` of step 3's game-path face folders);
on, the run is not clean and the file is named. Shared validation may produce a sanitized `ValidatedAestheticsExport` with ineligible
files/folders omitted, but the upgrader applies a stricter publication rule: **it writes its
output only when the migration is clean**, no Error finding and no finding that drops anything
but a leftover (step 10's `leftover_dropped` Warnings, content that never reached a player in
game).
Otherwise nothing is written and the report is the run's whole result: either the input is not
a clean old export and is fixed first, or the upgrader failed to migrate a clean one and is
fixed. We do not write a draft for manual repair, because every Studio export is upgraded once,
from a clean Red export, and a half-migrated one handed on would be compiled as if it were whole,
the losses the report named forgotten. Fixtures cover both old
`ingame_face` spellings and assert one canonical `ingame_face.toml` in the migrated player
folder; savefile names with colour codes (see the pes_savefile plan) and assert a plain folder name plus an
explicit `name = "..."` preserving the codes; a Red-era referee export, asserting that its
face/boots/gloves/common subfolders are flattened, `refs.txt` becomes `players.txt` and is removed,
and the result compiles to Red's output for the unflattened original, each difference
tabulated (the referee tracer, `team_compiler/testing.md`); a flattening collision (same file
name, different bytes, in two subfolders) left in place and reported; loose mode on an export
whose Red block starts mid-range (lowest folder 0128 → block 126, 0128 → player 03, 0149 → spare,
reported); and team identity — an export whose Note `Team:` line differs from its folder name
resolves from the Note (e.g. a folder "Numbers" whose Note names `/co/`), one without any Note falls back to the first-word
key; and the logo comparison — the two real pairs closest to the 40 % line from the survey behind
step 7: the worst-scoring downscale (`/omg/` VGL26, a shifted `_r` at 30 %) asserting no
`logo_small.png` and `logo_small_dropped`, and the closest deliberate variant (`/vrg/` VGL26, a
different drawing at 53 %) asserting `logo_small.png` and `logo_small_carried`. Real files, not synthesized
ones, so the threshold is checked against what authors actually shipped; the asserted shares are
the measured ones, so a change to the resampler or the tolerance shows up as a failing number.

The upgrader is exercised end-to-end by the parity tests (`team_compiler/testing.md`): each
tracer's old export is upgraded and compiled against Red's committed output, and the
maintainer's whole library of old exports is upgraded, compiled and compared against Red's
output by the library parity, rerun at each phase close that changes the upgrader or the
compiler (`scripts/library_parity/`).

## Acceptance

Format and rules: `CONTRIBUTING.md` "Testing". IDs are never reused; later phases append.
In every phase's scenarios, an old export is an AET export folder of a Fox team, upgraded with
`upgrade --savefile` naming a PES 21 save that holds its team, its team resolving from its
Note's `Team:` line, unless the scenario says otherwise (one naming pre-Fox files, `.model`
and `.mtl`, is a pre-Fox export upgraded with a PES 16 save); its folders and files are named as the
AET page names them (`XXX15 - Snuffy`, `k0430 - Crocs`, `g0432 - Mitts`,
`XXX_DEF_1st_realUni.bin`, `u0XXXp1.dds`). A GIVEN names the files that decide its outcome.
"The output" is the folder `upgrade` or `upgrade-models` writes; "the report" is step 10's,
printed on stdout, one line per finding naming its code and, for a finding about a file or
folder, its path.

**Phase 6 scope.** Phase 6 delivered the structural migration (steps 1 to 12) and model-folder
mode, through the CLI: the GUI's two drop targets are Phase 8's, and step 13, the glTF pass, is
Phase 7's, their scenarios with them in `plans/export_upgrader.md`. Observed are the output's files, the report and the exit code.
That a migrated export compiles to Red's output for the old one is the parity test's (Team
compiler plan, `testing.md`), not a scenario's, and so is a converted kit config's content beyond
the keys named here (`kit_config`'s fixtures). Test data: old exports are fixtures cut from real
AET exports of the maintainer's library; the team is `/co/` (714, Red block 426 to 450) unless a
scenario names another, and its save is `pes_savefile`'s PES 21 fixture save with that team's
records set as the scenario needs; the logo pairs are the two real ones "Verification" names.

**Input**

```
EU-IN-01   GIVEN the same old export as a folder, and as a .zip and a .7z holding it inside one
                main folder, as the AET page has them packed
           WHEN each is upgraded with -o into a directory of its own
           THEN the three outputs hold the same files with the same contents, the three reports
                the same findings naming the same paths inside the export, and every input file
                is unchanged
EU-IN-02   GIVEN day5.zip holding the folder "co Spring", whose Note's Team: line names /co/
           WHEN it is upgraded
           THEN the output is the sibling folder "co Full day5"
EU-IN-03   GIVEN the folder "co Spring" holding only a readme.pdf
           WHEN it is upgraded
           THEN it is reported as no old export, nothing is written, and the exit code is 1
EU-IN-04   GIVEN a Fox export whose face folder "XXX05 - Bob" holds face.fpk.xml and whose
                boots folder "k2180 - Bob", worn by slot 05, holds boots.fpk.xml
           WHEN it is upgraded, and when the folder holding those two folders is upgraded with
                upgrade-models
           THEN neither output holds a .fpk.xml file and neither report names one
```

**Team identity and root files**

```
EU-ROOT-01 GIVEN an old export folder "Numbers" whose Note's Team: line names /co/
           WHEN it is upgraded
           THEN the output is the sibling folder "co Full Numbers", its players' settings.toml
                files generated from /co/'s records in the save
EU-ROOT-02 GIVEN an old export "co Spring 26" without any Note txt
           WHEN it is upgraded
           THEN the team resolves from the first word, the output is "co Full Spring 26", and it
                holds no colors.txt, notes.txt or Note txt
EU-ROOT-03 GIVEN an old export "co Spring 26" whose Note's Team: line names no team of the
                teams list, and one "Numbers" without any Note
           WHEN each is upgraded
           THEN team_name_unknown is reported naming the Team: line for the first and the
                folder name for the second, no output is written, and the exit code is 1
EU-ROOT-04 GIVEN a Note with team colors and an "Other Notes" section
           WHEN the export is upgraded
           THEN the output's root colors.txt holds the Note's team colors, one per line in the
                Note's order, its notes.txt the section's text, and the output holds no Note txt
EU-ROOT-05 GIVEN a Note whose team and kit color lines are ASCII and whose "Player IDs:" lines are
                Shift-JIS, as JP's Summer 18 Note
           WHEN the export is upgraded
           THEN the root colors.txt and each kit's colors.txt hold the Note's colors, and the
                exit code is 0
EU-ROOT-06 GIVEN a Note whose team colors hold an entry that reads as no color: "- 2nd: blue",
                or an unfilled "- 2nd:", beside "- 1st: 048 146 082"
           WHEN the export is upgraded
           THEN unmigrated_content is reported at the Note naming that line, nothing is
                written, and the exit code is 1
EU-ROOT-07 GIVEN a Note whose team colors hold "- 1st: 048 146 082 - 255 255 255" and
                "- 2nd: #FFFFFF"
           WHEN the export is upgraded
           THEN the root colors.txt holds 048 146 082 and #FFFFFF, one per line,
                leftover_dropped is reported at the Note naming the first line, and the exit
                code is 0
```

**Player folders**

```
EU-PLY-01  GIVEN the face folder "XXX15 - Snuffy" holding a face model and portrait.dds, and the
                face folders "71416 Rex" and "XXX17- Ann"
           WHEN the export is upgraded
           THEN Players/15 - Snuffy/ holds the face folder's files, portrait.dds among them, and
                the others are Players/16 - Rex/ and Players/17 - Ann/
EU-PLY-07  GIVEN the face folder "XXX15 - Snuffy" holding Snuffy_Portrait.dds
           WHEN the export is upgraded
           THEN Players/15 - Snuffy/ holds portrait.dds with that file's bytes, and the exit
                code is 0
EU-PLY-14  GIVEN the face folder "XXX16 - Rex" holding portrait.dds and rex_portrait.dds
           WHEN the export is upgraded
           THEN the two files are reported as unmigrated content, nothing is written, and the
                exit code is 1
EU-PLY-08  GIVEN a pre-Fox export whose face folder "XXX05 - Bob" ships a face.xml, and the
                boots folder "k0430 - Crocs", worn by slot 05 alone
           WHEN the export is upgraded with a PES 16 save
           THEN Boots/Crocs/ holds the boots model, Players/05 - Bob/ holds Crocs.boots, and no
                player folder holds the model
EU-PLY-09  GIVEN slot 05's face folder and his alone-worn boots folder "k0430 - Crocs" both
                holding skin.dds, with different bytes, and slot 06's face folder and his
                alone-worn "k0431" both holding a byte-identical skin.dds
           WHEN the export is upgraded
           THEN Boots/Crocs/ holds the boots model, slot 05's folder holds Crocs.boots and his
                own skin.dds, the kept link is reported; slot 06's folder holds boots.fmdl and
                one skin.dds
EU-PLY-10  GIVEN the face folders "XXX23 - Bahsaka" and "XXX23 - Smug Redman", and the boots
                folders "k0430 - A" and "k0430 - B", 430 worn by slot 23 alone; and the face
                folder "XXX24 - Ann" and the boots folder "K0431 - Rex"
           WHEN the export is upgraded
           THEN "XXX23 - Smug Redman" and "k0430 - B", the second of each by name, and
                "XXX24 - Ann" and "K0431 - Rex", whose names Red refused, are reported as
                unmigrated content, nothing is written, and the exit code is 1
EU-PLY-02  GIVEN no face folder for slot 05, whose save record wears custom boots 430, and the
                boots folder "k0430 - Crocs" no other player wears
           WHEN the export is upgraded
           THEN Players/05 - Crocs/ holds boots.fmdl with the boots folder's textures and an
                ingame_face.toml, and no Boots/ folder holds the model
EU-PLY-03  GIVEN the face folder "XXX09" without a name part, and slot 09's save name carrying
                name colour codes
           WHEN the export is upgraded
           THEN the player folder is "09 - " followed by that name with the codes stripped
EU-PLY-04  withdrawn: restated as EU-PLY-22 (`spec/export_upgrader.md`), the
           marker written as ingame_face.toml, filled from the record, since step 0.0.1h
EU-PLY-05  GIVEN boots folders "k0430 - Crocs", worn by slots 03 and 05, and "k0431", worn by 04
                and 06, and the gloves folder "g0432 - Mitts", worn by 03 and 04
           WHEN the export is upgraded
           THEN Boots/Crocs/, Boots/k0431/ and Gloves/Mitts/ hold the models; slot 03's folder
                holds Crocs.boots and Mitts.gloves, 05's Crocs.boots, 04's k0431.boots and
                Mitts.gloves, 06's k0431.boots; and no player folder holds a copy of a model
EU-PLY-06  GIVEN a gloves folder no save record wears
           WHEN the export is upgraded
           THEN leftover_dropped is reported for each of its files, the output holds no copy of
                them, and the exit code is 0
EU-PLY-11  GIVEN the face folder "XXX23 - Pinto" and a save whose /co/ roster has 22 players
           WHEN the export is upgraded
           THEN Players/23 - Pinto/ holds the face and no settings.toml, the empty slot is
                reported naming 23, and the exit code is 0
EU-PLY-12  GIVEN boots folders "k0430 - Crocs", worn by slots 03 and 05, and "k0431 - Crocs",
                worn by 04 and 06, holding the same files byte for byte, and "k0432 - crocs",
                worn by 07 and 08, whose boots.fmdl differs
           WHEN the export is upgraded
           THEN Boots/Crocs/ holds k0430's files, Boots/k0432/ holds k0432's, no Boots/k0431/
                exists, the folders of 03, 04, 05 and 06 hold Crocs.boots and those of 07 and
                08 k0432.boots
EU-PLY-13  GIVEN a Fox face folder "XXX05 - Bob" holding fcl_hair.fmdl and fcl_hair_sim.skl,
                and one "XXX06 - Ann" holding fcl_hair_sim.skl and no fcl_hair.fmdl
           WHEN the export is upgraded, and when the folder holding the two face folders is
                upgraded with upgrade-models
           THEN Bob's output folder holds fcl_hair.skl, its bytes fcl_hair_sim.skl's, and no
                fcl_hair_sim.skl, and Ann's keeps fcl_hair_sim.skl as named
EU-PLY-15  GIVEN the files Faces/71406 - Civic cool.7z, Boots/boots.fmdl and
                Gloves/glove_l.fmdl, directly in those folders
           WHEN the export is upgraded
           THEN leftover_dropped is reported for each, the output holds none, and the exit
                code is 0
EU-PLY-16  GIVEN the empty face folder "XXX05 - Bob", slot 05 wearing custom boots 430 from
                "k0430 - FBM", which no other player wears, and the empty face folder
                "XXX06 - Ann", slot 06 wearing stock boots and gloves
           WHEN the export is upgraded
           THEN Players/05 - Bob/ holds boots.fmdl with the boots folder's textures and
                settings.toml, Players/06 - Ann/ holds settings.toml alone, neither holds an
                ingame_face.toml, and the exit code is 0
EU-PLY-17  GIVEN the boots folder "k0430 - EMPTY" holding boots.skl alone, worn by slots 03 and
                04, and the empty gloves folder "g0432 - EMPTY", worn by slot 03; slot 03 with
                the face folder "XXX03 - Ann", slot 04 with none and storing gloves 0
           WHEN the export is upgraded
           THEN Players/03 - Ann/ holds an fpc_on marker, slot 04's folder, named from his save
                name, holds fpc_on and an ingame_face.toml and a settings.toml with
                gloves_id = 0, no folder holds boots.skl or a link,
                the output holds no Boots/ or Gloves/ folder, leftover_dropped is reported for
                boots.skl, and the exit code is 0
EU-PLY-18  GIVEN a pre-Fox face folder "XXX05 - Bob" holding face.mtl and face.mtl.backup, and
                the boots folder "k0430 - Crocs", worn by slots 03 and 04, holding boots.model,
                boots.mtl and materials.mtl.bak
           WHEN the export is upgraded with a PES 16 save
           THEN Players/05 - Bob/ holds face.mtl and no face.mtl.backup, Boots/Crocs/ holds
                boots.model and boots.mtl and no materials.mtl.bak, leftover_dropped is reported
                for the two stray files, and the exit code is 0
EU-PLY-19  GIVEN the boots folder "k0430 - IKA-CHAN" holding boots.fmdl and "boots - Copy.fmdl",
                and the gloves folder "g0432" holding glove_l.fmdl, glove_r.fmdl and
                "glove_l - Copy.fmdl", both worn by slot 05 alone
           WHEN the export is upgraded
           THEN slot 05's folder holds boots.fmdl, glove_l.fmdl and glove_r.fmdl and neither
                copy, leftover_dropped is reported for each copy, and the exit code is 0
EU-PLY-20  GIVEN a pre-Fox face folder "71401" whose files, face.xml, face_high_win32.model and
                face.mtl, all sit below common/character0/model/character/face/real/71401/
                (FGOG VGL6's shape)
           WHEN the export is upgraded with a PES 16 save
           THEN slot 01's folder holds the three files directly and no common/ subfolder, and
                the exit code is 0
EU-PLY-21  GIVEN a face folder for slot 03 and no face folder for slot 05, whose record wears
                 custom boots
           WHEN the export is upgraded with a save
           THEN Players/03's settings.toml holds no face, hair or iris key and no
                ingame_face.toml exists there; Players/05 holds ingame_face.toml with his
                record's face, hair and iris
EU-PLY-22  GIVEN one player folder holding ingame_face and another ingame_face.txt, in a team
                 export and in a referee export
           WHEN each export is upgraded, the team one with a save, the referee one with
                neither flag
           THEN each of those player folders holds ingame_face.toml and no ingame_face or
                ingame_face.txt; the team export's two files hold their records' face, hair
                and iris, every key the save's version holds, and their settings.toml none of
                those keys; the referee export's two are empty
```

**Kits**

```
EU-KIT-01  GIVEN Kit Configs XXX_DEF_1st_realUni.bin, 714_def_2nd_realuni.bin and
                XXX_DEF_GK1st_realUni.bin, and Kit Textures u0XXXp1.dds, u0XXXp1_back.dds,
                u0714p2.dds and u0XXXg1.dds, the three main textures of different bytes
           WHEN the export is upgraded
           THEN Kits/p1/, Kits/p2/ and Kits/g1/ each hold a config.toml, p1 kit.dds and
                kit_back.dds, p2 kit.dds, g1 kit.dds; the output holds no binary config, no Kit
                Configs or Kit Textures folder, and no pre-fox or fox marker in any kit folder
EU-KIT-02  GIVEN a binary kit config with undecoded bits set
           WHEN the export is upgraded
           THEN its config.toml has no texture-name key and its [unknown] table holds those bits
EU-KIT-03  GIVEN the configs of EU-KIT-01 and a Note whose kit entries are "211 74 79 - 162 62 77 - 3", "30 30 30 - 250 250
                250 - 5" and "- 1st GK: 30 30 30 - 60 60 60", the template's spelling, without an icon
                number
           WHEN the export is upgraded
           THEN p1/colors.txt holds 211 74 79 and 162 62 77, one per line, and p1 holds no icon
                marker; p2/colors.txt holds its two colors and p2 holds icon_5; g1/colors.txt
                holds the gk: entry's two colors
EU-KIT-04  GIVEN a Note with two gk: entries and ten player kit entries
           WHEN the export is upgraded
           THEN leftover_dropped is reported for the second gk: entry and unmigrated content
                for the tenth player entry, each naming its line, nothing is written, and the
                exit code is 1
EU-KIT-05  GIVEN kits 1 and 2 whose binary configs both name kit 1's back and name textures,
                kit 2 shipping neither, and a kit 3 whose config names its own, which it ships
                with different bytes
           WHEN the export is upgraded
           THEN p2 holds kit_back.dds and kit_name.dds, copies of p1's, kit_texture_copied is
                reported for each naming p2, the stem and p1, and p3 keeps its own
EU-KIT-06  GIVEN player kits 1 and 2 and the goalkeeper kit holding byte-identical kit_back
                textures, and kit_name textures byte-identical in kits 1 and 2 and absent from
                the goalkeeper's
           WHEN the export is upgraded
           THEN Kits/all/ holds kit_back.dds, no kit folder holds one, kit_texture_hoisted is
                reported naming kit_back, and kit_name.dds stays in p1 and p2
EU-KIT-07  GIVEN player kits 1 and 2 and the goalkeeper kit, all three binary configs naming
                kit 1's back texture, which kit 1 alone ships
           WHEN the export is upgraded
           THEN Kits/all/ holds kit_back.dds, no kit folder holds one, and both
                kit_texture_copied and kit_texture_hoisted are reported
EU-KIT-08  GIVEN a Full export's Kit Configs XXX_DEF_1st_realUni.bin, XXX_DEF_2nd_realUni.bin
                and XXX_DEF_3rd_realUni.bin, a Note with three player kit entries, and Kit
                Textures holding u0XXXp5.dds, "u0XXXp2 - Copy.dds", back.dds and u0XXXp3.png
           WHEN the export is upgraded
           THEN leftover_dropped is reported for each of the four, the output holds no
                Kits/p5/ and no copy of them, and the exit code is 0
EU-KIT-09  GIVEN Kit Configs XXX_DEF_1st_realUni.bin and XXX_DEF_2nd_realUni.bin, and a Note
                with three player kit entries
           WHEN the export is upgraded
           THEN Kits/p3/ holds the third entry's colors.txt alone, kit_without_config is
                reported naming p3, and the exit code is 0
EU-KIT-10  GIVEN Kit Configs/714/ holding XXX_DEF_1st_realUni.bin and XXX_DEF_2nd_realUni.bin
           WHEN the export is upgraded
           THEN Kits/p1/ and Kits/p2/ each hold a config.toml converted from those configs, and
                the exit code is 0
EU-KIT-16  GIVEN Kit Configs XXX_DEF_1st_realUni.bin and Kit Textures/714/ holding
                u0XXXp1.dds
           WHEN the export is upgraded
           THEN leftover_dropped is reported for Kit Textures/714/u0XXXp1.dds, Kits/p1/ holds
                config.toml and no texture, and the exit code is 0
EU-KIT-11  GIVEN the configs of EU-KIT-08 and a Note whose "Kit Colours:" section holds
                "- 1st player: #D34A4F - #A23E4D" and "- 2nd player: 30 30 30 - 250 250 250",
                followed by "Other Notes:" and "- 3rd player: 1 2 3 - 4 5 6"
           WHEN the export is upgraded
           THEN p1/colors.txt holds #D34A4F and #A23E4D, p2/colors.txt holds 30 30 30 and 250
                250 250, no kit folder carries the line after "Other Notes:" (p3 holds no
                colors.txt), and the exit code is 0
EU-KIT-17  GIVEN the configs of EU-KIT-08 and a Note whose "Kit Colours:" section holds the
                template's "- 2nd player: #RRGGBB - #RRGGBB"
           WHEN the export is upgraded
           THEN the 2nd player entry is reported as unmigrated content, nothing is written, and
                the exit code is 1
EU-KIT-12  GIVEN Kit Configs XXX_DEF_1st_realUni.bin, XXX_DEF_GK1st_realUni.bin and
                XXX_DEF_GK2nd_realUni.bin, Kit Textures u0XXXg1.dds and u0XXXg2.dds, and a Note
                with one player kit entry and one gk: entry
           WHEN the export is upgraded
           THEN leftover_dropped is reported for XXX_DEF_GK2nd_realUni.bin and u0XXXg2.dds,
                Kits/g1/ holds config.toml and kit.dds, the output holds no other goalkeeper
                kit folder, and the exit code is 0
EU-KIT-13  GIVEN Kit Configs XXX_DEF_1st_realUni.bin to XXX_DEF_4th_realUni.bin, and a Note
                with three player kit entries
           WHEN the export is upgraded
           THEN Kits/p4/ holds a config.toml and no colors.txt, config_without_kit_entry is
                reported naming p4, and the exit code is 0
EU-KIT-14  GIVEN player kits 1 and 2 holding byte-identical kit_back textures, and no
                goalkeeper config
           WHEN the export is upgraded
           THEN p1 and p2 each keep kit_back.dds, the output holds no Kits/all/, and
                kit_texture_hoisted is not reported
EU-KIT-15  GIVEN Kit Textures holding u0XXXp1.ftex and u0XXXp1_back.ftex, the FTEX of two DDS
                textures, beside the config XXX_DEF_1st_realUni.bin
           WHEN the export is upgraded
           THEN Kits/p1/ holds kit.dds and kit_back.dds, each the DDS its FTEX converts back
                to, and the exit code is 0
EU-KIT-18  GIVEN Kit Textures holding u0XXXp2.dds and u0XXXp2.ftex beside
                XXX_DEF_2nd_realUni.bin
           WHEN the export is upgraded
           THEN u0XXXp2.ftex is reported as unmigrated content, nothing is written, and the exit
                code is 1
EU-KIT-19  GIVEN the old export "co Midcup Day 5" with Kit Configs XXX_DEF_1st_realUni.bin, a
                Note with one player kit entry, and Kit Textures u0XXXp1.dds, u0XXXp3.dds,
                u0XXXp3_back.dds, u0XXXg2.dds and back.dds, the two main textures of
                different bytes
           WHEN it is upgraded
           THEN Kits/p3/ holds kit.dds and kit_back.dds and neither config.toml nor colors.txt,
                no finding names p3 or its textures, leftover_dropped is reported for
                u0XXXg2.dds and back.dds, the output holds no Kits/g2/, and the exit code is 0
```

**Portraits, logo, models and Common**

```
EU-MDL-01  GIVEN Portraits/player_XXX05.dds, player_71407.dds and player_09.dds, and a Collars
                folder
           WHEN the export is upgraded
           THEN Portraits/ holds 05.dds, 07.dds and 09.dds, and Collars/ holds the old folder's
                files
EU-MDL-19  GIVEN Portraits/player_XXX08.jpg and XXX17.png (BAG VGL18.5's shapes),
                player_XXX24.dds, "Vanilla vitamins.png", Player_XXX05.dds,
                player_XXX06.jpeg and sub/player_XXX07.dds
           WHEN the export is upgraded
           THEN Portraits/ holds 08.jpg with that file's bytes and nothing else,
                leftover_dropped is reported for each of the other six, and the exit code is 0
EU-MDL-20  GIVEN Portraits/player_71405.dds and player_XXX05.dds, of different bytes
           WHEN the export is upgraded
           THEN player_XXX05.dds, the second by name, is the only file reported as
                unmigrated content, nothing is written, and the exit code is 1
EU-MDL-02  GIVEN the Logo folder of /omg/'s VGL26 export, whose _r is a downscale of its _r_ll
                with 30 % of pixels differing
           WHEN the export is upgraded
           THEN the root logo.png is its _r_ll image, the output holds no logo_small.png and no
                _r_l image, and logo_small_dropped is reported with the share 30 %
EU-MDL-03  GIVEN the Logo folder of /vrg/'s VGL26 export, 53 % of pixels differing
           WHEN the export is upgraded
           THEN the root holds logo.png and logo_small.png, its _r image, and logo_small_carried
                is reported with the share 53 %
EU-MDL-04  GIVEN a Logo folder holding _r_l and _r images and no _r_ll
           WHEN the export is upgraded
           THEN logo.png is the _r_l image, logo_fallback is reported naming it, no
                logo_small.png is written, and the exit code is 0
EU-MDL-05  GIVEN a pre-Fox face folder whose .mtl references face_srm.dds, a file of the
                folder, model/character/uniform/common/XXX/hair.dds, with Common/hair.dds, and
                model/character/boots/k0002/ek0062_n.dds, and whose subfolder realkit/ holds a
                .mtl referencing that Common hair too
           WHEN the export is upgraded
           THEN the migrated .mtl references ./face_srm.dds and
                model/character/boots/k0002/ek0062_n.dds as written and ./hair.dds, realkit's
                references ./hair.dds, the player folder's root holds the texture link
                hair.dds.common, and realkit/ holds none
EU-MDL-06  GIVEN a .mtl reference into model/character/uniform/common/XXX/ whose file Common/
                lacks
           WHEN the export is upgraded
           THEN it is reported naming the reference, the .mtl keeps it as written, its XXX
                now 714, and the exit code is 0
EU-MDL-22  GIVEN a pre-Fox /vg/ export whose face folder's .mtl references
                model/character/uniform/common/vg/env.dds and
                model/character/uniform/common/VG/fbm_s.dds, and Common/ holding env.dds alone
           WHEN the export is upgraded
           THEN the .mtl references ./env.dds beside the link env.dds.common, the fbm_s
                reference is reported and kept as written, its VG now 758, and the exit code
                is 0
EU-MDL-07  GIVEN a Fox face folder whose FMDL path table names hair in the Fox common directory,
                and skin under another directory, hair and skin being in Common/ and not in the
                folder
           WHEN the export is upgraded
           THEN the FMDL names hair and skin as textures of its own folder, which holds
                hair.dds.common and skin.dds.common
EU-MDL-08  GIVEN an FMDL texture common/XXX/u0XXXp0/sourceimages/armor_bsm, and
                Common/u0XXXp1/armor_bsm.dds
           WHEN the export is upgraded
           THEN the FMDL texture is ./kitN/ armor_bsm, the model's folder holds the link
                kitN/armor_bsm.dds.common, Common/ holds kit1/armor_bsm.dds, and the shared
                validation finds the link resolving
EU-MDL-09  GIVEN a model folder holding u0XXXp1.dds, u0714p2.dds, a folder u0XXXp0/ and
                jacket_u0XXXp1.model, the textures named by its .mtl and its FMDL path table,
                which also name dummy_kit.dds, and the model by its zlib-wrapped face.xml as
                ./jacket_u0XXXp0.model
           WHEN the export is upgraded
           THEN the folder holds kit1.dds, kit2.dds, kitF/ and jacket_kit1.model, the references
                to the two files name kit1 and kit2 and those to the folder kitN/, the face.xml
                names ./jacket_kitN.model and is written unwrapped, and the dummy_kit reference
                is unchanged
EU-MDL-10  GIVEN Common/u0XXXg1.dds and a keeper's face folder whose FMDL names u0714g1
           WHEN the export is upgraded
           THEN Common/ holds u0714g1.dds and the keeper's FMDL texture and link both name
                u0714g1
EU-MDL-21  GIVEN a /co/ export whose face folder "XXX07 - Ann" holds u0714p2.dds and
                u0XXXp2.dds, and u0715p3.dds and u0XXXp3.dds, the four of different bytes
           WHEN the export is upgraded
           THEN Players/07 - Ann/ holds kit2.dds with u0XXXp2.dds's bytes and kit3.dds with
                u0XXXp3.dds's, leftover_dropped is reported for u0714p2.dds and u0715p3.dds, and
                the exit code is 0
EU-MDL-11  GIVEN a pre-Fox folder without a face.xml holding gloveL_foo.model, bar_gloveR.model
                and plain.model, and one with a face.xml holding gloveL_baz.model
           WHEN the export is upgraded
           THEN the first holds foo_gloveL.model, bar_gloveR.model and plain.model, and the
                second gloveL_baz.model, its name unchanged
EU-MDL-12  GIVEN a pre-Fox face folder without a face.xml holding oral_parts_body_win32.model,
                parts_body.mtl and materials.mtl, gloveL_foo.model with gloveL_foo.mtl, and the
                link oral_cap_win32.model.common, with Common/oral_cap_win32.model
           WHEN the export is upgraded
           THEN it holds parts_body.model, parts_body.mtl, materials.mtl, foo_gloveL.model,
                foo_gloveL.mtl and cap.model.common, Common/ holds cap.model, and the shared
                validation finds the link resolving
EU-MDL-13  GIVEN a Fox face folder whose FMDL names skin under
                /Assets/pes16/model/character/boots/k0915/sourceimages/, with skin.dds in the
                folder, and zero under /Assets/pes16/model/character/common/sourceimages/
           WHEN the export is upgraded
           THEN the FMDL names skin as a texture of its own folder and zero under its directory
                as written, and neither is reported
EU-MDL-14  GIVEN a pre-Fox face folder whose face.xml names
                model/character/uniform/common/XXX/oral_hand_l_*.model, with
                Common/oral_hand_l_win32.model
           WHEN the export is upgraded
           THEN Common/ keeps oral_hand_l_win32.model, and the face.xml names it as before
EU-MDL-15  GIVEN Portraits/player_XXX07.dds, and slot 07's face folder holding a portrait.dds
                of different bytes
           WHEN the export is upgraded
           THEN portrait_conflict is reported naming slot 07, nothing is written, and the exit
                code is 1
EU-MDL-16  GIVEN a pre-Fox face folder whose .mtl names
                model/character/uniform/common/XXX/u0XXXp0.dds, with Common/u0XXXp1.dds and
                Common/u0XXXp2.dds
           WHEN the export is upgraded
           THEN Common/ holds kit1.dds and kit2.dds, the .mtl names ./kitN.dds, the player
                folder holds the link kitN.dds.common, and the shared validation finds the
                link resolving
EU-MDL-17  GIVEN a pre-Fox face folder whose apc.mtl holds a `\` after a <sampler/>, as G
                Autumn 25's "72117 - Lisa" does
           WHEN the export is upgraded
           THEN the output's apc.mtl holds the same materials without the `\`, pes_model's
                MaterialSet::read reading it, mtl_stray_text_dropped is reported naming it, and
                the exit code is 0
EU-MDL-18  GIVEN a pre-Fox face folder whose .mtl is not XML (an unclosed <material>)
           WHEN the export is upgraded
           THEN it is reported as unmigrated content with its parse error, nothing is written,
                and the exit code is 1
```

**Player settings**

```
EU-SET-01  withdrawn: restated as EU-SET-10 (`spec/export_upgrader.md`), the
           face, hair and iris keys out of settings.toml since step 0.0.1h
EU-SET-02  GIVEN two players with face folders, one wearing stock boots 12 and one custom
                boots 430 from a migrated folder, both wearing the default gloves 0
           WHEN the export is upgraded
           THEN the first's settings.toml has boots_id = 12, none has a gloves_id, and the custom
                wearer's has no boots_id; upgraded from the folder "co Midcup Spring", both files
                also have gloves_id = 0
EU-SET-03  GIVEN a player whose record wears custom boots 433, which no boots folder carries
           WHEN the export is upgraded
           THEN it is reported naming the player and 433, nothing is written, and the exit code
                is 1
EU-SET-04  GIVEN one player whose folder's name part equals his save name, a face folder named
                otherwise than its player's save name, and a save name carrying colour codes
           WHEN the export is upgraded
           THEN their settings.toml files hold name = true, name = the save name, and name = the
                save name with its codes byte for byte, in a folder named without them
EU-SET-05  withdrawn: restated as EU-SET-11 (`spec/export_upgrader.md`), his
           cheek_type in ingame_face.toml since step 0.0.1h
EU-SET-06  GIVEN the PES 21 fixture save after strip-and-seed
           WHEN an export of one of its teams is upgraded with it
           THEN the output equals the one from the unstripped save, and the report also names
                each stripped player, at Info
EU-SET-07  GIVEN a player whose record holds the hide strip and boots and gloves IDs of his own,
                with their folders, each holding its model
           WHEN the export is upgraded
           THEN his folder holds no fpc_on marker, his settings.toml carries the strip, and his
                boots and gloves folders are merged into his player folder
EU-SET-08  GIVEN a rostered player with no face folder, wearing stock boots and gloves
           WHEN the export is upgraded
           THEN the output holds no player folder for his slot
EU-SET-09  GIVEN a PES 16 save holding a player with a face folder who stores 183 in
                goal_celebration_2
           WHEN his team's pre-Fox export is upgraded with it
           THEN his settings.toml has no goal_celebration_2 key, a warning names him, the key
                and 183, and the exit code is 0
EU-SET-10  GIVEN a player with a face folder, and his save record
           WHEN the export is upgraded
           THEN his settings.toml sets every key the save's version holds but the face, hair
                and iris keys to his record's value, shirt_name to his save string, and has no
                height or weight key, and his folder holds no ingame_face.toml
EU-SET-11  GIVEN the PES 18 fixture save, holding a player with cheek_type 5 whose slot the
                 export gives no face folder and custom boots
           WHEN his team's export is upgraded with it
           THEN his ingame_face.toml has cheek_type = 5, and nothing is reported about it
```

**Referee exports**

```
EU-REF-01  GIVEN an old referee export whose root holds only refs.txt, Players/, ref_marker.dds
                and a Note txt, its refs.txt mapping 35 slots, one folder backing two
           WHEN it is upgraded without --savefile or --loose
           THEN the output's players.txt holds the same mapping, the repeated folder included, in
                the canonical roster grammar, the output holds no refs.txt and the same
                ref_marker.dds at its root, leftover_dropped is reported for the Note, and the
                exit code is 0
EU-REF-02  GIVEN a referee folder with face/, boots/, gloves/ and common/ subfolders, one file
                name in two of them with identical bytes
           WHEN the export is upgraded
           THEN the referee folder holds every file at its root under its own name, that one
                once, and none of those subfolders
EU-REF-03  GIVEN a referee folder whose boots/ and gloves/ hold one file name with different
                bytes
           WHEN the export is upgraded
           THEN it is reported as unmigrated content naming both files, nothing is written, and
                the exit code is 1
EU-REF-04  GIVEN a team export's face folder and a referee folder each holding a subfolder alt/
                with a face model
           WHEN each export is upgraded
           THEN alt/ is carried unchanged in both player folders
EU-REF-05  GIVEN the old referee export folder "refs Spring"
           WHEN it is upgraded with --savefile, with --loose, then with neither flag
           THEN the first two are refused naming the flag, with exit code 2 and nothing
                written; the third writes the sibling folder "refs Spring upgraded"
EU-REF-06  GIVEN a referee folder whose face/face.mtl references
                model/character/uniform/common/XXX/hair.dds, ./skin.dds and
                model/character/uniform/common/XXX/kit/sock.dds, files of its common/ (the
                last in common/kit/), and ./whistle.dds, a file of the export's Common/ alone
           WHEN the export is upgraded
           THEN the .mtl references hair, skin, ./kit/sock and whistle, the referee folder holds
                hair.dds, skin.dds, kit/sock.dds and the link whistle.dds.common, and the shared
                validation finds the link resolving
EU-REF-07  GIVEN the old export folder "refs Summer" holding Players/ and no refs.txt
           WHEN it is upgraded
           THEN it is reported as holding no refs.txt, nothing is written, and the exit code
                is 1
EU-REF-08  GIVEN a referee folder whose face/ holds kit/kit.dds
           WHEN the export is upgraded
           THEN the referee folder holds kit/kit.dds and no face/ subfolder, and the exit code
                is 0
EU-REF-11  GIVEN a referee folder whose face/ holds face/face.fmdl
           WHEN the export is upgraded
           THEN face/face/ is reported as unmigrated content, nothing is written, and the exit
                code is 1
EU-REF-09  GIVEN a referee folder without a face.xml whose common/ holds parts_body.model, which
                no reference of the referee names
           WHEN the export is upgraded
           THEN parts_body.model is reported as unmigrated content, nothing is written, and the
                exit code is 1
EU-REF-10  GIVEN an old referee export in the refs compiler's layout (as "refs for Winter 25
                Day 2"): a root Refs list.txt and Common/, the referee folders at the root, each
                with a Face/ and some with a Common/ of their own, no Players/
           WHEN it is upgraded without --savefile or --loose
           THEN the output's players.txt holds Refs list.txt's mapping, each referee folder is
                under Players/ with its subfolders folded as EU-REF-02's, Common/ is carried, and
                the exit code is 0
EU-REF-12  GIVEN an old referee export in EU-REF-10's layout whose root referee folder
                "boston" has a Face/ holding face.model, face.mtl and hair.xml
           WHEN it is upgraded
           THEN the export is read as pre-Fox (a .model inside a root referee folder is inside
                the old layout, unlike a leftover's in EU-REP-04): edithair_unsupported is
                reported for Players/boston, nothing is written, and the exit code is 1
```

**Loose mode**

```
EU-LOOSE-01 GIVEN an old /a/ export (Red block 126 to 150) with the face folder "XXX03 - Ann",
                 boots folders "k0128 - Rex" and "k0130", and no face folder for slot 05
            WHEN it is upgraded with --loose
            THEN k0128 is merged into Players/03 - Ann/, k0130 into Players/05/, which holds an
                 ingame_face.toml, the report states the pairings 0128 → 03 and 0130 → 05,
                 03 - Ann/settings.toml holds name = true alone, 05/ holds no settings.toml, and
                 the exit code is 0
EU-LOOSE-02 GIVEN EU-LOOSE-01's export with the boots folders "k0149" and "k0155" too
            WHEN it is upgraded with --loose
            THEN k0149 and k0155 are reported as IDs naming no player, nothing is written, and
                 the exit code is 1
EU-LOOSE-03 GIVEN a /a/ export holding the empty face folder "71405" alone
            WHEN it is upgraded with --loose
            THEN the output holds an empty Players/05/, no ingame_face.toml in it, and the
                 exit code is 0
```

**Report**

```
EU-REP-01  GIVEN an old export whose Other/ folder holds two files
           WHEN it is upgraded
           THEN leftover_dropped is reported for each, the output is written without them, the
                report says the migration is clean, and the exit code is 0
EU-REP-02  GIVEN the face folder "XXX05 - Bob" whose subfolder alt/ holds a face.xml, which the
                Studio format admits directly in a player folder alone
           WHEN the export is upgraded
           THEN the shared validation's file_type_disallowed is reported for Players/05 - Bob
                naming alt/face.xml, the report says the migration is not clean, nothing is
                written, and the exit code is 1
EU-REP-04  GIVEN a Fox export (as the real KFG VGL26 one) whose boots folder "k0128 - Rex",
                worn by slot 03, holds boots.fmdl, beside a root folder "FMDL Boots/k0128 - Rex/"
                holding boots.model and a root readme.pdf
           WHEN it is upgraded with --loose
           THEN leftover_dropped is reported for FMDL Boots/k0128 - Rex/boots.model and for
                readme.pdf, Players/03 - Rex/ holds boots.fmdl (the migration is Fox's: the
                leftover's .model decides nothing), the output holds no Boots/ folder, and the
                exit code is 0
EU-REP-05  GIVEN EU-PLY-10's export: face folders "XXX23 - Bahsaka" and "XXX23 - Smug
                 Redman", boots folders "k0430 - A" and "k0430 - B", and the misnamed
                 "XXX24 - Ann" and "K0431 - Rex"
           WHEN it is upgraded with its save
           THEN each unmigrated_content line carries its reason: the second face folder
                slot_taken with `other` naming "Faces/XXX23 - Bahsaka", the second boots folder
                id_taken with `other` "Boots/k0430 - A", the two misnamed slot_invalid and
                id_invalid with no `other`
EU-REP-03  GIVEN an old export whose migration needs no manual attention
           WHEN it is upgraded
           THEN the report holds no finding above Info and says the migration is clean, and the
                exit code is 0
```

**Model-folder mode**

```
EU-MF-01   GIVEN a folder holding a Fox face folder "XXX15 - Snuffy" whose file names and FMDL
                path table name u0XXXp1, a pre-Fox gloves folder "g0430 - Crocs" with
                gloveL_crocs.model, the boots folder "k0431", a face folder "Snuffy" holding a
                hair model and ingame_face.txt, and Common/
           WHEN upgrade-models runs
           THEN the output holds 15 - Snuffy/ with kit1 in every file name and reference,
                Crocs/ with crocs_gloveL.model, k0431/, Snuffy/ with a bare ingame_face.toml,
                and an upgraded Common/, and the report names each output folder with where it goes
EU-MF-02   GIVEN a player folder's .mtl referencing model/character/uniform/common/XXX/hair.dds,
                another reference whose file Common/ lacks, a .mtl.common link, and Common/
                holding hair.dds and a .mtl referencing face.dds
           WHEN upgrade-models runs
           THEN the player's .mtl references ./hair.dds beside the link hair.dds.common, the
                other reference is reported and left as written, Common's .mtl keeps its
                reference to face.dds as written, the .mtl.common link is kept, the shared
                validation finds every link resolving, and the exit code is 0
EU-MF-03   GIVEN a top-level folder holding no file directly in it (a players/ folder of model
                folders, or a folder holding only Thumbs.db)
           WHEN upgrade-models runs
           THEN it is reported as unmigrated content, nothing is written, and the exit code is 1
EU-MF-04   GIVEN a model folder whose FMDL names u0XXXg1
           WHEN upgrade-models runs
           THEN the migrated FMDL names u0XXXg1 as written
EU-MF-05   GIVEN two face folders of one slot ("XXX15 - A", "XXX15 - B"), a face folder without
                a number and a boots folder no player folder links
           WHEN upgrade-models runs
           THEN the report holds no finding on a folder's number or a missing link, the
                destination export's to make, and the exit code is 0
EU-MF-06   GIVEN a Fox face folder "XXX01 - Dekinai" holding face_diff.bin and portrait.dds and
                no model
           WHEN upgrade-models runs
           THEN the output holds 01 - Dekinai/ with both files, reported as going into Players
EU-MF-07   GIVEN a boots folder "k0431" holding boots.fmdl and "boots - Copy.fmdl"
           WHEN upgrade-models runs
           THEN k0431/ holds boots.fmdl alone, leftover_dropped is reported for the copy, and
                the exit code is 0
```

**CLI**

```
EU-CLI-01  GIVEN old exports "co - Spring", "co Day 8 Additions", "co midcup Day 5" and
                "co Full Spring"
           WHEN each is upgraded
           THEN the outputs are the sibling folders "co Full - Spring", "co Midcup Day 8
                Additions", "co midcup Day 5 upgraded" and "co Full Spring upgraded"
EU-CLI-02  GIVEN the old export "co Spring" upgraded with -o naming a directory D
           WHEN it runs
           THEN the output is D/co Full Spring, and nothing is written beside the input
EU-CLI-03  GIVEN a team export upgraded with both --savefile and --loose, or with neither
           WHEN it runs
           THEN it is refused naming the two flags, with exit code 2, and nothing is written
EU-CLI-04  GIVEN in turn an export path that does not exist, an export path naming a .txt file,
                a --savefile that is not a save, and a savefile that does not hold the resolved
                team, and an upgrade-models folder that does not exist or names a file, and an
                upgrade-models -o whose path names no folder (it ends in ..)
           WHEN upgrade, or upgrade-models for the last two, runs
           THEN it is refused naming the path, the file or the team, with exit code 2, and
                nothing is written
EU-CLI-07  GIVEN a corrupt .zip export
           WHEN upgrade runs
           THEN export_extract_failed is reported naming it, nothing is written, and the exit
                code is 1
EU-CLI-05  GIVEN an output folder that already exists, or one that cannot be created
           WHEN upgrade or upgrade-models runs
           THEN it is refused naming the folder, with exit code 3, nothing is written, and an
                existing folder is unchanged
EU-CLI-06  GIVEN a folder "new models" of old model folders
           WHEN upgrade-models runs without -o
           THEN the output is the sibling folder "new models upgraded", one folder per input
                folder
           WHEN it runs with -o out/done, out/ missing
           THEN out/done/ holds one folder per input folder directly, and no "new models
                upgraded" folder exists
EU-CLI-08  GIVEN the old export "co Spring" named on the command line as "co Spring/." and as
                "co Spring/Other/.."
           WHEN each is upgraded
           THEN the output is the sibling folder "co Full Spring", beside "co Spring", as for
                the plain name
EU-CLI-09  GIVEN EU-CLI-01's exports and EU-REF-01's referee export
           WHEN each is upgraded
           THEN each team export's report holds coverage_tag_chosen with its tag and source
                (`co - Spring`: Full, default; `co Day 8 Additions`: Midcup, additions; `co
                midcup Day 5`: Midcup, name; `co Full Spring`: Full, name) and the referee
                export's none; WHEN `co - Spring` is upgraded with --coverage midcup THEN the
                output is `co Midcup - Spring`, source flag; WHEN `co Full Spring` is upgraded
                with --coverage midcup THEN exit 2 and nothing written; GIVEN the referee
                export and --coverage, or an unknown value, THEN exit 2
EU-CLI-10  GIVEN `co - Spring` and its output's staging folder `co Full - Spring.upgrading`
                 holding a file, left by an earlier run
           WHEN it is upgraded
           THEN the staging folder is removed, staging_folder_removed names it, and the run is
                clean with its output written; GIVEN (Windows, where an open file blocks its
                removal) a file of the staging folder held open THEN exit 3 naming the
                folder, nothing written; GIVEN EU-CLI-06's "new models" and its output's
                staging folder `new models upgraded.upgrading` left by an earlier run, WHEN
                `upgrade-models` runs, THEN it is removed and named the same way
```
