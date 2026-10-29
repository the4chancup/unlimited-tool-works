The Team compiler also runs without its window, from a terminal opened in the folder that holds
`UTW.exe`:

```text
UTW team-compiler check [exports-root] [--export <path>]...
UTW team-compiler compile [exports-root] [--mode normal|test|sideload] [--export <path>]... [--no-deploy] [--edit-mode-closed]
UTW team-compiler upgrade-dpfl [--yes]
```

`check` reads your exports and reports what it finds, without writing anything. `compile` reads
them, reports the same findings and the few only a build can find (a model that fails to
convert, for one), and builds the CPK named by the `cpk_name` setting in the output
folder (`output/` beside `UTW.exe` unless you changed the `output_folder_path`
setting), as
`<cpk_name>.cpk`. In a terminal window, a line under the findings counts the exports done
while `check` or `compile` runs, and, while `compile` builds, the parts built so far. When a
compile ends, one line sums it up: the files written, the exports and folders left out and the
folders with errors. The CPK is written in full before it replaces the previous one, so
`<cpk_name>.cpk` is never half-written. A run that finds nothing to compile, with no file in
the `overrides` folder (below), writes nothing and leaves the previous CPK as it was. If a file
of an export changes while `compile` runs (someone saves over it, or replaces the archive), the
run stops with the line `source_changed_during_run` naming the file, writes nothing, leaves the
previous CPK in place, and ends with exit code 3. Run it again. `compile` builds
exports for PES 2015 to 2021: the models of the players in the roster, kept in each player's
own folder (face models, boots and gloves), with their textures, their portraits and kits, and
everything else this chapter describes. Boots are a model whose name is `boots` or ends in
`_boots`, such as `kit_boots.fmdl` (`redboots.fmdl` is not), and gloves are a model whose name
is or ends the same way in `glove_l`, `glove_r`, `handL` or `handR`. A name may also
carry a kit number (`boots_kit1.fmdl`, `glove_l_kit2.fmdl`). A portrait
is a `portrait` image in the player's folder, or `NN` in the `Portraits` folder, numbered 01 to 23
as a player folder is (`05.dds`, or `05 - Bob.dds` with a name after ` - `). A file there that
is not a texture named so, or any file in a subfolder of it, is `portrait_name_invalid` and is
left out. A player
with both gets one portrait when the two files are identical, while two that differ are the
line `portrait_conflict` and the whole export is skipped, since the compiler cannot tell which
one is meant. A texture (a player's, a shared folder's, a Common one, a kit's or a
portrait) can be a `.dds`, `.ftex`, `.png`, `.jpg`, `.bmp`, `.webp`, `.tga` or `.tif` file
(`.jpeg` and `.tiff` work too). Each
is converted to what the chosen PES version reads: BC7 for PES 2019 to 2021, and BC3 for
PES 2015 to 2018 (BC1 when the image has no transparency at all). A `.dds` or `.ftex` already in a
format the version reads keeps its compressed blocks, packed into the version's own container
(a `.dds` as an `.ftex` on PES 2018 to 2021, a `.ftex` as a `.dds` on PES 2015 to 2017).
Whatever its codec, a `.dds` or `.ftex` keeps the mip levels it brings, and an image of any
other kind has the smaller copies of itself the game needs (its mip levels) generated for it. A portrait is the exception: the game
reads it as a `.dds` on
every version, so a `.dds` portrait is kept as it is. One under a DX10 header gets the header
the game reads for its format, its pixels unchanged, reported as `portrait_header_rewritten`,
except an uncompressed one no other header can describe, which is converted. A portrait in
any other format becomes a BC3 `.dds` of the same size, its mip levels made anew. For PES 2015 to
2018, which cannot read BC4, BC5 or BC7, a `.dds` portrait in one of those formats, under either
header, is converted like any other image. A texture whose name ends in `_nrm`
(`skin_nrm.png`), or in `_nrm` and a kit number (`pants_nrm_kit1.png`), is a normal map and
is encoded as one. A texture named with `kit1` to `kit9`
(`pants_kit1.dds`, `pants_kit2.dds`) is the one the game shows when that kit is picked before
a match. The model that uses it names `pants_kitN` instead, and the game puts the picked kit's number in
place of the `N`. A texture named `kitF` (`pants_kitF.dds`) is the set's fallback, the file
a kit number with no variant of its own gets. It is never used under its own name, so a model
naming `pants_kitF` finds nothing. When the team has a kit number (a `p1` to `p9` folder in `Kits`, and the
goalkeeper's `g1` goes with the number picked) with no texture of its own, it gets the set's
fallback (`kitF`, else the lowest variant). For PES 2018 to 2021 the fallback is written once
as `pants_kitN`; for PES 2015 to 2017 it is copied under the missing number's name, so the
game never shows a missing texture. The warning `kit_variant_missing`
names the texture, the kit and the copied file, and fires only when the set has no `kitF`.
Per-kit models (`pants_kit1.fmdl` beside
`pants_kit2.fmdl`, or `.model` files, or a mix of the two) need a `face.xml` to name them. On
PES 2018 to 2021, and on PES 2015 to 2017 in a shared boots or gloves folder and in a folder
holding `ingame_face.toml`, the set's fallback is used (a model in the other game's format,
converted, or a `.model` with its `.mtl`) and the other variants are left out. The warning
`kit_variant_model_left_out` says so.
On PES 2015 to 2017 a face's set is listed once in the
`face.xml`, as `pants_kitN`. Every variant is packed, and the game loads the one for the kit
picked. Each variant's `.mtl` goes by the same rule (`pants_kit1.mtl` and `pants_kit2.mtl`, or
one `pants.mtl` for all), and the warning `kit_variant_mtl_differs` names a variant whose `.mtl`
would not be found that way. A kit number the team has with no model of the set gets the
fallback's model copied for it, as a texture does (its `.mtl` too when that is the variant's own,
`pants_kit1.mtl`), so the game never shows the player without that part. The `kitF` model and
its `.mtl` are never packed under their own names, and the warning
`kit_variant_missing` names the set, the kit and the copied model only when there is no
`kitF`. The same holds for a set a player's own `face.xml` names. Six lines name a texture that cannot be
used, each with the file: `texture_too_small` (a side under 4 pixels), `texture_not_div4` (for
PES 2015 to 2017, a width or height that is not a multiple of 4: 1000 is one, 1002 is not),
`texture_not_pow2` (a portrait whose width or height is not a power of two, or, for PES 2018
to 2021, a texture with mip levels when neither its width nor its height is: 256, 512, 1024,
...; a `.png` or other image always gets mip levels, and a `.dds` or `.ftex` with a single level
passes at any size), `kit_texture_too_big` (for PES 2015 to 2017, a kit's main `kit` texture,
its own or the one from `all`, wider or taller than 2048 pixels or with a side that is not a
power of two), `texture_type_mismatch` (a file renamed to another
format instead of resaved, a PNG called `skin.dds`) and `texture_codec_unsupported` (a `.dds`
or `.ftex` in a format the compiler does not read, BC6H for one, or a 16-bit or interleaved
`.tga`: resave it). Both commands report them from each file's header alone, so `check`
finds them too, except a codec that only the conversion finds unreadable (BC6H): `compile`
reports that one. What is left out depends on where the texture is: a player's or shared
folder's texture leaves out the whole folder, a kit's texture the whole kit (a texture in `all`
leaves out every kit using it), a `Common` texture only that file (the rest of `Common` is still
built, and a player linking that file is left out with the line `link_target_dropped`), a
portrait only that portrait. A `Common` texture that `compile` fails to convert, after those
checks (`texture_codec_unsupported`, or `folder_pack_failed` for a file cut short past its header), leaves out
every texture of its folder (`Common/` itself, or the subfolder it is in). With `pass_through`
on, a texture with `texture_too_small`, `texture_not_pow2` or `kit_texture_too_big`, a portrait
included, is kept and converted as it is. A renamed one is still left out, and so is one
with `texture_not_div4`, which PES 2015 to 2017 cannot load at all, or with
`texture_codec_unsupported`. The
team's logo
is one image at the export's root named `logo` (`logo.png`), in any of the image formats
above and of any size: `compile` makes the game's three logo sizes (512, 256 and 128 pixels
square) from it. An image named `logo_small` beside it is used for the smallest size instead,
when you want a different picture there. An image that is not square is fitted inside a
transparent square, unless its name ends in `_crop` (`logo_crop.png`: the middle square is
kept and the sides are cut off) or `_stretch` (the whole image is stretched to a square);
`_fit` spells the default out. The
note `logo_fit_applied` says which was done. The warning `logo_upscaled` names an image
the compile enlarges to the size it is made into, which makes it less sharp. A `logo` image at
the export's root that is not a readable image, or whose name carries a tag the compiler does
not know (`logo_foo.png`, `logo2.png`), is the line `logo_file_invalid`, from both
commands, and the logo is left out even with `pass_through` on (a bad `logo_small` image
leaves out all three sizes). A `logo_small` image needs a `logo` beside it
(`logo_small_without_main`), and two images for one size are `logo_role_duplicate`: either
way no logo is built. A texture whose header cannot be read at all (not a texture, or
cut short before its header ends) is the line `texture_unreadable`. It is left out the way a
texture with `texture_too_small` is, but `pass_through` does not keep it. A texture that cannot be packed for any
other reason is reported as `folder_pack_failed`, naming the file, and its folder is left out. For PES 2015 to 2017 a
kit's `kit_mask` texture is built, a `Full` export's kit without one gets the compiler's
built-in mask, and a
`kit_srm` texture gets the line `kit_texture_not_used`: these versions do not read it, so it
is not built, and the rest of the kit is. For PES 2018 to 2021 it is the other way round: the
`kit_srm` is built, a kit without one gets none, and a `kit_mask` gets the line
`kit_texture_not_used`. Kit configs go into the CPK as loose files, one per kit in the team's
folder, and for PES 2015 to 2017 there is no `UniformParameter.bin`. A player's own boots and
gloves get the ID reserved for his roster
slot, the same number for both. On PES 2018 to 2021 a face model whose vertices are weighted to the hand bones
(`skh_`) has its hands cut off at the wrist into the player's gloves at compile time, which the
note `model_hand_split` names. A model named as boots or gloves is never cut. A shared `Boots`
or `Gloves` folder that players point at with a link file (an empty `Crocs.boots` in the
player's folder names `Boots/Crocs`) is built once, with its own textures, under one of the 17 IDs the team keeps for shared boots
folders (and 17 more for gloves folders), given out in the folders' name order (`Apple` before `Zebra`). A folder linked plainly by
just one player takes that player's own ID instead. Either way, a player linking it plainly gets no boots or
gloves of his own. A player folder holding both a link file and boots (or gloves) of its own
combines the two on PES 2018 to 2021, and on PES 2015 to 2017 when it also holds
`ingame_face.toml`: the shared folder's models join the player's own boots (or gloves) under
the player's ID, its textures travel with them, and the line `link_combined` says so. On PES
2018 to 2021 his own boots or gloves are a model of his, one a `.common` link brings in, a
boots or gloves model of a `Faces` folder he links, or for gloves the hands cut off a face
model of his or of a `Faces` folder he links. On PES 2015 to 2017 without `ingame_face.toml`
the link keeps its plain meaning, and the player's own boots and gloves models are parts of
his face. The
shared folder is still built on its own for any player linking it plainly. A link to a shared
`Faces` folder (`Longhair.face` names `Faces/Longhair`) always combines: the shared folder's
face models become part of the player's face, beside any face models of the player's own, and
its face diff (`face_diff.bin` or `face_diff.xml`) is used only when the player's folder has
neither, and its `fcl_hair_sim.fclo` on PES 2018 to 2021 only when the player's folder has none. A player can also load a model kept in the export's `Common` folder through an empty
link file named after it plus `.common` (`legs.fmdl.common` directly in the player's folder
names `Common/legs.fmdl`). On PES 2018 to 2021 the game cannot read a model
from there, so the Common model is built into the player's own face, boots or gloves as if it
were a model of the same name in the player's folder, and merged with the player's models of
that name. Its name says which of the three it joins: `legs.fmdl.common` is face content,
reported as `fmdl_fcl_hair_fallback` like a local `legs.fmdl`. A skeleton file named after it
in `Common` (`Common/legs.skl`) comes with it. On PES 2018 to 2021 a `.model` in `Common`
works the same way (`legs.model.common`): it is converted for each player linking it, with the `.mtl` a link
finds (a `.mtl` named like it beside the link, else one beside the model in `Common`, else one
beside the link,
else one in the player's or shared folder holding the link), and a `.fmdl` of the same name in
`Common` is used instead when there is one. A
converted model's `.mtl` path into the team's PES 2015 to 2017 Common folder
(`model/character/uniform/common/XXX/skin.dds`) is pointed at a texture of that name in the
model's own folders (a `skin.dds` of the player's), and otherwise at the team's Common output,
whether `Common` holds that texture or not, and
`fmdl_texture_not_found` reports one nobody supplies (below). The textures in `Common`
(image files directly in it, and on PES 2015 to 2017 the image files of a subfolder too) are built
once for the whole team, whether a player uses them or not, and a
Common model's textures of those names are read from there, so twenty players sharing one
Common model share one copy of its textures. A player's own models can use a texture in
`Common` the same way: an empty file named after it plus `.common` (`hair.dds.common` directly
in the player's folder, for `Common/hair.dds`) makes
the player's models use the team's one copy of `hair`, instead of a copy of the player's own.
The folder holding the link must not also hold a texture named `hair`, whatever its format
(`texture_stem_conflict`: `hair.png` beside `hair.dds.common` is one).

A shared `Faces`, `Boots` or `Gloves` folder can hold links too, in it or in any
subfolder, standing for the `Common` file at the same path: for PES 2018 to 2021 they
work as a player's do, a texture link, a model link (a `Boots` folder holding only
`boots.fmdl.common` has boots) and a `.mtl` link alike. For PES 2015 to 2017 its links
are not read yet: each is reported as `file_not_used`. The rest of its tree follows a player
folder's rules at any depth, except that a shared link, a marker, `settings.toml` or
`ingame_face.toml` anywhere in it is `file_type_disallowed`. As in a player folder, a face file
(`face.xml`, `face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo`) below a subfolder is
`file_type_disallowed` too.

A model of the player's own and a Common model
must not define a material of the same name over textures kept in different places (the
player's folder and `Common`): that is a `merge_material_conflict`. When a texture in `Common`
cannot be packed for a reason other than the texture lines above, the line
`folder_pack_failed` names it in `Common`, no texture of the `Common` folder or subfolder it is in is
built, and the players linking
Common models are still built. Several boots models in one
folder (`a_boots.fmdl` beside `kit_boots.fmdl`), several gloves models for one hand, or
several face models under one name (`face_high.fmdl` beside `old_face_high.fmdl`), are on PES
2018 to 2021 merged into one model in the alphabetical order of their file names, reported as
`fmdl_merged`; on PES 2015 to 2017 only boots models merge, those of a shared `Boots` folder
or of a player whose folder holds `ingame_face.toml`, reported as `model_merged` below.
Models merged into one must agree with each other: a material they
define differently is reported as `merge_material_conflict`, and skeletons that differ (one
model with a skeleton file and one without, two different files, or a bone placed differently)
as `skl_merge_conflict`. Either leaves that folder's face, boots or gloves out of the CPK. The
rest of the folder is still built, so the player shows in the game with that part missing (a
player with boots only and no face model of his own shows no head). Treat the folder as broken
until the error is fixed. A
texture under one name in two of the places a player's models come from (the player's own
folder, a shared folder it combines) is packed once when the two files are the same. A texture
`.common` link is the same only as another link to the same `Common` texture, never as a real
file, even one with the same content. When they
differ, the player's own file wins, as a texture link of his in that place does: every model
of his, the shared folder's included, uses his, and the line `shared_texture_overridden`
names the texture and the shared folder, so an override you meant is visible and one you did
not is caught. When two shared folders he combines hold different files of one name, his
whole folder is left out of the CPK, and the line `shared_texture_conflict` names the
texture and every package holding it (`packages=face,boots`). An export
whose shared boots folders, or shared gloves folders, need more shared IDs than the team's 17
of that kind have free is skipped by both commands with the error `boots_id_pool_exhausted` or `gloves_id_pool_exhausted`, naming
the count. On PES 2018 to 2021 a model whose name says nothing about what it is (`torso.fmdl`) is taken for face
content and merged into the face's `fcl_hair.fmdl`. The line `fmdl_fcl_hair_fallback`
names it, so boots and gloves models must say so in their names. In a shared `Boots` or
`Gloves` folder nothing can be face content: a model anywhere in it not named as boots or
gloves (`hat.model` in `Boots/Mud`), and on PES 2018 to 2021 a `.common` link to one not named
so (`glove_l.fmdl.common` in `Boots/Mud`), is the error `fmdl_name_invalid`
(`model_name_invalid` for PES 2015 to 2017), and the folder is left out with the players
linking it (`link_target_dropped`). A shared boots or gloves folder holding no model of its
kind (on PES 2018 to 2021 a `.common` link to one counts) gets no ID. When a player links it with no model of his own for that part, it is reported
as the warning `shared_folder_no_model`, and the player wears the game's own. On PES 2018 to 2021 a skeleton file named after a
model (`kit_boots.skl` beside `kit_boots.fmdl`, `fcl_hair.skl` beside `fcl_hair.fmdl`,
`torso.skl` beside `torso.fmdl`, `boots.skl` beside a shared folder's `boots.fmdl`) is packed
as the part's skeleton under its slot's name (`boots.skl` for a boots model,
`fcl_hair_sim.skl` for a face's `fcl_hair` part); one named after a gloves model pairs with
nothing and is `file_not_used`. Boots and hair without one get the standard body skeleton. On
PES 2015 to 2017 no `.skl` is packed: the bones of one beside an `.fmdl` go into the `.model`
that `.fmdl` is converted to. On PES 2018 to 2021, a skeleton named after a
`face_high`, `hair_high` or `oral` model has no place in the game's face package: it is left out
and reported as `skl_no_slot`. A face folder with no face diff (`face_diff.bin` or
`face_diff.xml`) gets the bundled default file. On PES 2018 to 2021 an `fcl_hair` model without
`fcl_hair_sim.fclo` beside it gets the bundled
file of that name too. A face folder may give its
`face_diff.bin` as text instead, in a `face_diff.xml`: the file in base64, alone or inside a
`<dif>` element, the way an older face folder's `face.xml` holds it. It is decoded and packed
as the face's diff (`face_diff.bin` on PES 2018 to 2021, the face's `<dif>` on PES 2015 to
2017), and counts as one. The player's own face diff, in either form, is used over
a linked `Faces` folder's. Both commands report a face diff that cannot be used as
`face_diff_invalid`, naming the file and the reason (text that is not base64, a `<dif>`
holding elements, bytes that are not a face diff, or a file shorter than its own header says, which the game would read past
the end of), and a folder holding both `face_diff.bin` and
`face_diff.xml` as `xml_dif_conflict`, so `check` finds them too. Either leaves the whole folder out of the CPK,
even with `pass_through` on, and a player linking a `Faces` folder left out this way is left
out too, with the line `link_target_dropped`. A player's folder may sort its
files into subfolders of any name and depth. Each file there is what its name says, exactly
as in the folder itself: `parts/boots.fmdl` is the boots, and on PES 2018 to 2021 a model in any
subfolder goes into `fcl_hair.fmdl` when the model is not named as face, boots or gloves, reported as
`fmdl_fcl_hair_fallback` with its subfolder in the name (`parts/torso.fmdl`). A skeleton file
pairs with the model beside it in the same subfolder, textures from any subfolder are packed
with the player's other textures, and parts in a subfolder are merged with the folder's own models and
with a linked shared folder of the same kind. Only the files directly in the player's folder
count as `ingame_face.toml`, `fpc_on` or `fpc_off`, `settings.toml`, or a face file (`face.xml`,
`face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo`): one of those inside a subfolder is reported as `file_type_disallowed`. A link file in a subfolder works as it does in
the folder itself, and a `.common` link there names the `Common` file at the same path
(`face/torso.fmdl.common` names `Common/face/torso.fmdl`). A `.txt` note (`notes.txt`) in a
player or shared folder, or in `Common`, is allowed: nothing reads it, so it is reported as the
warning `file_not_used`. A player folder with no face model, and
no `Faces` link or (on PES 2015 to 2017) `face.xml` of its own bringing one, still gets a face
folder, an empty one (a referee's folder gets none), so the game shows no head for that player. That is the
usual case when a body model brings its own head. A file named `ingame_face.toml` in the player's folder
gives him the in-game face instead, the head the game builds from his face, hair and iris,
which the file sets (`iris_color`, `[face]`, `[hair]`, the keys `settings.toml` held before).
An empty file gives the game's default face. No face folder is built, and a model
that would have gone into the face (`torso.fmdl`, `fcl_hair.fmdl`) becomes part of the
player's boots, with its skeleton file, and is not reported as
`fmdl_fcl_hair_fallback`. The marker's old names, `ingame_face` and `ingame_face.txt`, are
the error `ingame_face_marker_renamed`: the file is ignored. A `face_high`, `hair_high` or `oral` model, a `.common` link to one, or a `Faces` link in a folder holding
`ingame_face.toml` is the error `ingame_face_explicit_face_model`, and the player's folder is left out.
A face file in a folder that builds no face (a player folder with no face model, no `Faces`
link bringing one and, on PES 2015 to 2017, no `face.xml` of its own, a shared `Faces` folder
with no face model and no such `face.xml`, or a shared `Boots` or `Gloves` folder) is not used,
and both commands report it as `face_file_not_used`.
The face files are `face_diff.bin` and `face_diff.xml`, `fcl_hair_sim.fclo` on PES 2018 to 2021,
and on PES 2015 to 2017 a member's own `face.xml` in a folder holding `ingame_face.toml`. For PES 2015 to 2017 every `.model`
of a player folder without `ingame_face.toml`, boots and gloves included, goes into the player's
face, listed in a `face.xml` that `compile` writes with each model's type read from its name.
A player folder may hold its own `face.xml` instead (directly in it, not in a subfolder), and both
commands check its content. What is known not to work leaves the folder
out: a file that does not parse, a root other than `<config>`, a `<model>` without `type` or
`path`, a `./` or Common reference to a file the export does not hold, a Common path without
its 3-character folder, on PES 2016 a model name not starting with `face_high_`, `hair_high_`
or `oral_`, and a `<dif>` beside a `face_diff.xml` (not beside a `face_diff.bin`:
the `<dif>` is for PES 2015 to 2017, the bin for PES 2018 to 2021). What the
compiler cannot vouch for is kept and warned about: an unknown element, attribute or `type`, a
`ratio` that is not a number, a path it cannot check, and more than one `face_neck` entry. A
`.model` of the folder the xml
does not list is `xml_model_unlisted`, and it is not packed. A `level` other than 0 is noted. `compile` writes the xml back with its `./`
files each packed under the name its reference gives, each Common model's path written with the team ID
in place of its 3-character folder, the `face_neck`
stand-in appended when no entry has that type (`xml_face_neck_added`), and for PES 2015
`uniform` written `uniform_sub` (`xml_uniform_pes15`). Only the models and `.mtl` files the
xml names are packed, and its own `<dif>`, else the folder's face diff, is the face's. For
PES 2018 to 2021 the xml is ignored, reported as `xml_ignored_fox`, and the
folder's models compile as they would without it. On PES 2015 to 2017 a `face.xml` in a
shared face folder is checked the same way, in that folder, and becomes the xml of every
player's face linking it: each of his own models it does not name gets the entry a generated
xml would give it, after the xml's own entries, and his own face diff, when he has one, wins
over the xml's `<dif>`. A player folder linking a shared face must not hold a `face.xml` of its
own: holding one, whether or not the shared folder has one, is `xml_shared_face_conflict`, which
leaves his folder out even with `pass_through` on. On PES 2015 to 2017 a `face.xml` in a shared boots or
gloves folder is ignored, reported as `xml_ignored_shared`; on PES 2018 to 2021 any folder's
own `face.xml` is ignored as `xml_ignored_fox`.
On PES 2015 to 2017 a face model whose vertices are weighted to the hand bones (`skh_`) has its
hands cut off at the wrist at compile time, which the note `model_hand_split` names: the rest
keeps the model's place, and the hands of `body.model` become two more models of the same
face, listed as its left and right gloves with the same `.mtl`.
A model named as boots or gloves, or one a `.common` link brings in, is never cut, nor is any
model of a face with a `face.xml`, its own or its linked shared face's, which lists what the
face loads.
With `ingame_face.toml`, every `.model` but a glove (`shirt.model` and `torso.model` included)
becomes part of the player's own boots, written as one `boots.model` with one `boots.mtl` in
his own boots folder. Several are merged into one, reported as `model_merged`, and a linked
`Boots` folder beside them is merged in too, reported as `link_combined`. Parts that define
one material differently (`merge_material_conflict`), one bone differently
(`skl_merge_conflict`) or carry different header flags (`model_merge_flags_conflict`) leave
the player's boots out, the error telling which. His gloves (`glove_l`, `glove_r`, `handL`,
`handR`) go to his own gloves folder, each written as a linked `Gloves` folder's models are
(below), none merged. A linked `Gloves` folder beside them adds its models, reported as
`link_combined`, the player's own model or `.mtl` replacing the linked folder's file of the
same name. With `ingame_face.toml`, a `.common` link to a model (`kit_boots.model.common`,
`glove_l.model.common`) makes the Common model one more part of his boots or gloves, copied
into his own folder with the `.mtl` it uses (found as for any link, below). A Common `.fmdl`
(`legs.fmdl.common`) is converted into the part as his own are, and a link to a
`.mtl` that one of his models uses is copied in the same way. The textures a Common `.mtl` names
stay in the team's Common folder. A face with
models but none of type `face_neck` (a `face_high` model is one) gets an invisible stand-in,
reported as `xml_face_neck_added`. For PES 2015 a model typed `uniform` (`body_uniform.model`)
is listed as `uniform_sub` instead, reported as `xml_uniform_pes15`. A player or shared folder
holding a `face_edithair.xml` or a `hair.xml`, anywhere in it, is the error
`edithair_unsupported` for PES 2015 to 2017, and the folder is left out. Both commands report
it. A linked `Faces` folder is copied into the face of each
player linking it. A player who also has face models of his own gets the line `link_combined`,
his own model or `.mtl` replacing the linked folder's file of the same name. A linked `Boots` folder's `.model` is written once as
`boots.model` with the `.mtl` it uses as `boots.mtl`, several merged into one and reported as
`model_merged`. A linked `Gloves` folder's models
are written once at their paths and the `.mtl` files they use at the folder's root, each named
by its path below the folder with `/` written `_` (`grip/glove_r.mtl` as `grip_glove_r.mtl`),
all in lower case, and the models are listed in a `glove.xml` that `compile` writes. The
textures of either sit beside the models.
Each model uses the first `.mtl` found in its own folder, then in the player's or shared folder
holding it: one whose name is the start or the end of the model's name (`body.mtl` for
`body_high.model`; a model named as the game packs it is read without the `oral_` and `_win32`
around its name, so `fbm.mtl` for `oral_fbm_u_win32.model`), then `materials.mtl`, then any. For PES 2015 to
2017, and for PES 2018 to 2021 when no `.fmdl` of its name is beside it, a model with no `.mtl`
to use is the error `model_material_undefined`, and the folder holding it is left out, even with `pass_through`
on. A model using a material its `.mtl` does not define is
`model_material_undefined` too, naming the `.mtl` and the missing materials: the folder is
left out unless `pass_through` is on, which keeps it as it is, and the game then draws those
parts with its fallback material. Both commands report either. For PES 2015 to 2017 the
models, `.mtl` files and textures in `Common` are
written once into the team's Common folder in the game, which loads them from there: a
`.common` link to a model (`legs.model.common`) lists the Common model in the player's
`face.xml`, using a `.mtl` named like it beside the link, else the one found beside the model in
`Common`, else
one found as above. A `.fmdl` in `Common` is converted once there, with the `.skl` of its name
as its pose, its materials written as a `.mtl` of its name (a `.mtl` of that name in `Common`
too is two files of one name, and the Common models are left out with `folder_pack_failed`),
and a link to it (`legs.fmdl.common`) lists it with those materials, not a `.mtl` of the
player's. A `.fmdl` beside a `.model` of the same name in `Common` is left out for the `.model`,
which a link to the `.fmdl` then lists. A link to a `.mtl` (`body.mtl.common`) counts as that `.mtl` sitting beside
the link, and a link to a texture points the player's `.mtl` files at the Common texture. A
kit folder of a
full export with
no `kit` texture, its own or the one from `all` (an empty kit folder included), is built with a magenta and black checkerboard in its
place and reported as `kit_placeholder`, so a kit nobody drew shows as missing in the game. A
kit texture whose name the compiler does not build (`kit_spec.dds`) is left out and reported
as `kit_texture_not_used`, as the map the version does not read is. A
kit's `config.toml` that cannot be read (not UTF-8 text, a value of the wrong type or out of
range) is reported by both commands as `kit_config_invalid`, naming the error, and the kit is
left out, even with `pass_through` on. For PES
2015 to 2017 `compile` builds a player folder's own `.model` files with
their `.mtl` files, textures and face diff, its `.common` links to a `.model`, an `.fmdl`, a
`.mtl` or a texture, the linked shared `Faces`, `Boots` and `Gloves` folders, the `.model`,
`.mtl`, `.fmdl` (converted, with its `.skl`) and texture files of the `Common` folder, the
kits, the export's `.model` and `.fmdl` collars, and the
portraits and the logo. A player's `.fmdl` models (with `ingame_face.toml`, the `.fmdl` parts of
his own boots and gloves), and a linked shared `Faces`, `Boots` or `Gloves` folder's, are
converted to `.model` files with their materials. A file in a model folder that no package
reads (a skeleton pairing no model, a link to a file of a kind nothing reads) is left out and reported as the warning `file_not_used`. A `.fmdl` beside
a `.model`
of the same name is left out for the `.model`. A `.skl` of a converted model's name gives
its pose, and `fcl_hair_sim.fclo` is not used. A converted model moved onto the target version's skeleton is
noted as `skeleton_retargeted`, and one that cannot be converted leaves the face, boots or gloves
it is part of out with `model_conversion_failed`. A player's own `.model` whose bones sit on
another version's skeleton is moved onto the target version's too, its `.mtl` packed as written.
A model a member's own `face.xml` lists is packed as it is, and a `.model` collar is too when
it sits on the version's own body.
A metal material (a Fox `fox3ddf_ggx` one)
converted this way reflects the compiler's environment map, written beside the player's
textures, or a shared `Boots` or `Gloves` folder's, as `env.dds`. An `env` texture of the
folder's own (`env.dds`, `env.png`), or the Common one its `env.dds.common` link names, is
used instead. A `Common` folder's `.fmdl` gets the map in the team's Common output, or the
`env` texture of the `Common` folder, and `templates/env.dds` in the
data folder replaces the built-in one. On every version a `refs` export's kits, logo,
portraits and collars are left out with `file_not_used` (see the referees below), and every
other export compiles them. For PES 2018 to 2021 a player's own `.model` models, and a shared
folder's, are converted to `.fmdl` with the `.mtl` they use. A `.model` beside a
`.fmdl` of the same name is left out for the `.fmdl`, with its `.mtl`. A converted model is
checked again as the `.fmdl` it becomes: one whose conversion fails, or that comes out with
a vertex more than 5000 units from the origin, is reported as `model_conversion_failed` or
`vertex_too_far_from_origin` and its package (face, boots or gloves) is left out, even with
`pass_through` on. A player's own `.fmdl` whose `.skl` poses it on a skeleton other than the target
version's is moved onto the target version's skeleton, noted as `skeleton_retargeted`. One
without an `.skl` is taken to sit on PES 2021's skeleton, since an `.fmdl` does not record
its version and the other Fox versions' skeletons differ from it only slightly. On
every version, a converted model's Warnings and Infos name what the
conversion changed: a bone folded onto another or its pose guessed, a material's shader
guessed, a texture, a flag or a field the target version has no place for. A `.glb`/`.gltf` model in a player folder is not read yet: with no `.fmdl` (PES
2018 to 2021) or `.model` (PES 2015 to 2017) of its name beside it, `compile` leaves its
folder out with the error `model_gltf_unsupported`, naming it, and does not use the `.model`
or `.fmdl` of the other game beside it instead. Beside one of its name in the game's own format it
is ignored. One in `Common` (directly in it, and on PES 2015 to 2017 in a subfolder too) with
none of its name in the game's format beside it is left out by itself, the same error naming it,
and a folder whose `.common` link names a model of its name is left out with that error, on
PES 2018 to 2021 a shared folder with the players linking it. One in a shared `Faces`, `Boots` or `Gloves` folder leaves out that folder and every
player folder linking it, each player reported with `model_gltf_unsupported` naming the shared
file (`Boots/Crocs/boots.glb`): a player without the face, boots or gloves he linked would not
look as he should.

A `Collars` folder holds custom collar models, each replacing one of the game's stock collars:
model files (`.fmdl`, `.model`, `.glb` or `.gltf`) directly in the folder, named `collar_` plus the stock collar's
number, with or without leading zeros (`collar_12.fmdl` and
`collar_012.fmdl` both replace collar 12). Any other file there is `file_type_disallowed`.
Both commands check each collar file for the target version: a name that is not
`collar_<number>`, or a number that is not a stock collar a kit can wear (1 to 101 on PES 2015,
to 105 on PES 2016, 116 on PES 2017 and 2018, 124 on PES 2019, 127 on PES 2020, 131 on PES
2021), is the error `collar_id_invalid`. Collars 105 (worn by every FPC kit) and 77 (the
referees' marker, see below) are the compiler's own, so a file named for either is the error
`collar_id_conflict`, naming who holds it. A `.fmdl` or `.model` collar is then checked as any
model is: one that cannot be read is `model_broken`, and an error the checks on the model find (a
vertex too far from the origin, for instance) leaves it out too, while a warning keeps it.
Each of these errors leaves the collar file out, and the rest of the export goes on.
On PES 2018 to 2021, `compile` writes a team's `Collars/collar_<number>.fmdl` as it is in
place of that stock collar, and sets every kit of the team to wear it, as its collar and its
winter collar: the kits the export holds, and for a midcup export also the team's installed
kits it does not send again. This comes after the FPC values, so an FPC team's custom collar
wins over the FPC one. Two exports cannot replace the same collar: the first in export order keeps it, and the
later one's file is the error `collar_id_conflict`, naming the export that holds it, its file
left out and its kits keeping their own collars. An export holds one collar, so a second
collar file of the same export is that error too, naming the export itself. On PES 2015 to
2017, `compile` writes a team's `Collars/collar_<number>.model` as it is in place of that
stock collar (one posed on a body other than the version's, as PES 2017's own collars
are on PES 2015, is first moved onto it, its material names kept), and every kit of the
team wears it as its collar and winter collar, as above: the kits the export holds, and
for a midcup export the
team's installed kits it does not send again, whose kit configs the CPK then carries changed.
The two rules on who keeps a collar are the same. A `.fmdl` collar compiled for PES 2015 to
2017 is converted to a `.model`, its materials named as the game's own collars' are: on PES
2017 `uni_collar` (the first) and `uni_shirts` (the rest), on PES 2015 and 2016 all
`uni_shirts`, so the game dresses it with the team's kit as it does them. No `.mtl` is written
for it. A `.model` collar compiled for PES 2018 to 2021 is converted to a `.fmdl` with PES
2017's own `uniform.mtl`, which the compiler carries, as its material set, and its materials
(`uni_collar` and `uni_shirts`, the only two a collar may name) take the game's own collar
materials of those names, so the game dresses it with the team's kit as it does them. One
that cannot be converted, either way, is the error `model_conversion_failed`, and the collar
file is left out. A `.glb` or `.gltf` collar is not read yet:
`compile` leaves it out with the error `model_gltf_unsupported`, and the rest of the export
goes on.

A `refs` export compiles like a team's player folders: each referee folder its `players.txt`
lists is built once and written for every slot it is listed under, slot NN as the face
`referee0NN` with the boots `k99NN` and the gloves `g99NN`, and the folder's textures go once
into `common/999/<folder name>/`. A referee's link to a `Boots` or `Gloves` folder makes that
folder the boots or gloves of each of his slots: on PES 2018 to 2021 merged with any of his
own, on PES 2015 to 2017 written as his slot's folder alone, his own boots and gloves riding in
his face as a team player's do. A referee folder with no face model writes no face file, so the
game's own referee head stays, where a team player's folder with no face model shows no
head.
A `refs` export's kits, logo, portraits and collars have no referee to go to: each is left
out and reported as the warning `file_not_used`.
The referees go into a CPK of their own, named by the `refs_cpk_name` setting
(`4cc_18_referees.cpk`), never into the teams' CPKs, with `multicpk_mode` on or off. Beside
the referees it also holds the referee kits and the referee appearance file the game needs,
the same in every compile. A file of these can be replaced from the data folder's
`templates/referees_fox` folder, or `templates/referees_prefox` on PES 2015 to 2017 (see
`templates` below). On PES 2018 to 2021 each referee kit is also written, as it is in the referees CPK, as its
entry in `UniformParameter.bin` (the team CPK's, or the bins CPK's with `multicpk_mode` on),
which the game reads the kits' values from, so a `refs` export compiled alone writes that CPK
too. A
`ref_marker.dds` at the root of
the `refs` export, the cup's logo for instance, is shown on the ground under every referee: on
PES 2018 to 2021 it goes into the referees CPK with a flat square model written as collar 77,
and the referee kits are set to wear that collar. On PES 2015 to 2017 it replaces the texture
of the prop the game itself shows under the referee (`parts/referee/incom_bsm.dds` in the
referees CPK), and the referee kits keep their collar. Collar 77 stays the referees' on every
version: a team's kit whose collar or winter collar is 77 (which on PES 2018 to 2021 would wear
the marker too) is left out with the error `kit_collar_reserved`, naming the field. A
`ref_marker.dds` that cannot be converted is reported with its texture error and left out, and
the referee kits keep their own collar. The referees CPK is
installed into the game together with the teams' CPKs, all of them or none, or left in the
output folder with them, after them. It is written only when the `refs` export builds
something: on PES 2015 to 2017 a compile in which only the `refs` export builds something,
with no file in the `overrides` folder, writes it alone, and the installed team CPK stays as
it was. Since the
compile replaces it, the installed referees CPK is not one of the CPKs the team colors, kit
colors and other files shared by every team are taken from. A `refs_cpk_name` that names the team CPK (`cpk_name`), or with `multicpk_mode` on the
`bins_cpk_name` CPK or one of the numbered `teams_cpk_name` CPKs, letter case aside, stops the
command with exit code 2. `--mode test` and `--mode sideload` write the referees' files with
the others' and have no referees CPK; `--mode sideload` writes the referee kits and appearance
file with them, and `--mode test` does not, since they come from no export.

`compile` installs the CPK into the game. It copies the CPK into the `download` folder of the game
folder (the `pes_folder_path` setting) and then puts it in the place of the old one, so the game
never finds half of a CPK there, and no copy of it is left in the output folder. Before it
reads any export, it checks that it can. When it cannot, or when the old CPK cannot be replaced
at the end, it says why with an error (listed below, after `--no-deploy`), leaves the CPK in the
output folder instead, and ends with exit code 1.

An export may hold a `notes.txt` at its root, a note for whoever compiles the cup. `compile`
gathers the notes of every export it compiles into one file, `teamnotes.txt` in the output
folder, one section per team headed by its name (`--- /co/ ---`, `--- /refs/ ---` for the
referees), in the order of the exports. The note of a skipped export is left out. A compile
with no notes to gather removes an older `teamnotes.txt`, so the file never shows notes of
exports the last compile did not build. A compile that fails before its output is in place, or
writes nothing, leaves it as it was. When `teamnotes.txt` cannot be written, or a previous one
cannot be removed, the error `teamnotes_write_failed` names it, and the CPK is still in place.

A folder named `overrides` beside the settings file (the data folder) holds files that
`compile` puts into the CPK as they are, each at its path below that folder (into `livecpk/`
in `--mode sideload`):
`overrides/common/etc/TeamColor.bin` becomes `common/etc/TeamColor.bin` in the CPK. Use it for
a file no export provides, or a fixed version of one an export gets wrong. `compile` reports the
note `overrides_active`, naming the folder and how many files it holds, and writes the CPK even
when no export is compiled. A file in `overrides` wins over the same file from an export or
from the compiler (the team color file, for one). The warning `duplicate_path` names each file
replaced that way, and the rest of that export is still built. A file in `overrides` that
cannot be read stops the CPK from being written, with the line `cpk_write_failed` naming it,
and the previous CPK stays as it was. `check` does not read the folder.

`check` prints one line per finding: the export it is about, when there is one, how serious it
is, its code, where in the export it is, when it is about a folder or file, and its details in
parentheses, when it has any.
The line `Info export_identified (team=/co/,
id=714)` tells you which team the export was recognized as: its name's first word, looked up in
the teams list (`team=referees` for a `refs` export).

The second word of an export's name says what the export covers: `Full` or `Midcup`, in any
letter case (`co Full Spring 2026`, `co midcup day 5`). A full export is the team's whole set,
everything it has for the cup. A midcup export brings additions to what is already installed. So
a full export replaces the team's kits in the game's kit list. A kit of a past cup that the
export does not hold is no longer offered, and on PES 2018 to 2021 the team's installed kit
configs of kits the export does not hold are removed (on PES 2015 to 2017 each is a file of
its own, which a compile cannot remove). Every team needs a player kit and a goalkeeper kit,
so a full export with no player kit, or no goalkeeper kit, gets an empty one (`p1` or `g1`),
which compiles as the placeholder kit (`kit_placeholder`). On PES 2018 to 2021 a full export
also clears the boots or gloves of each of its compiled players whose folder has none. A midcup export adds its
kits to those installed, and what a midcup kit folder does not hold stays as the game has it
installed: a folder without `config.toml` keeps the installed kit config (the template where
there is none, or one that cannot be read), one holding no `kit` texture keeps the installed
textures, and one holding no usable `colors.txt` keeps the kit's installed menu colors. Only
the second word counts, so a `Full` later in the name changes
nothing. A team export whose second word is neither is skipped with the error
`export_tag_missing`, naming the export. Rename it `<team> Full …` or `<team> Midcup …`. A
referee export needs no tag. An export still in the old layout (a root folder named `Kit
Configs`, `Kit Textures` or `Other`) is skipped with the one error `export_layout_old`,
naming the folder, and nothing else is reported about it. Run it through the Export
upgrader once. A referee export's player folder sorted into `face`, `boots`, `gloves` or
`common` subfolders, the layout of older referee exports, is left out with the one error
`player_layout_proto`, naming the subfolder, and nothing else is reported about it. Run the
export through the Export upgrader once. In a team export those names are ordinary
subfolders.

Both commands also read every `.fmdl` and `.model` model and every `.mtl` material file of the
export that the compile uses, an archive's included, and report what is wrong or suspicious in
each, one line per file and problem, naming the file and how many faces, vertices or bones are
concerned. A
problem the game cannot live with (a mesh over a hard limit, a face naming a vertex that does not
exist) is an Error. The folder is left out (a file in `Common` is left out alone), unless
`pass_through` is on, which keeps it as it is. A Warning or an Info leaves everything in, but an
`.fmdl` whose every mesh is hidden is itself left out on PES 2015 to 2017
(`model_hidden_dropped`). A model
with a vertex more than 5000 units from the origin lags the game for the whole matchday. It is
reported as `vertex_too_far_from_origin` and its folder is left out even when `pass_through` is
on (for a model the compiler converts, the part it is made into). So is a file that cannot be
read as a model or a material file at all, reported as `model_broken` or `mtl_broken`.
A player linking a shared folder or `Common` model left out this
way is left out too, with the line `link_target_dropped`.

Both commands also read each player's `settings.toml`, and his `ingame_face.toml` when he
has one. One that cannot be read (not UTF-8 text, a key it does not know, a value of the
wrong type or outside the range its key allows) is reported as `settings_toml_invalid` or
`ingame_face_toml_invalid`, naming the error. The file is ignored, even with `pass_through`
on (a bad `ingame_face.toml` still marks the in-game face, so the player wears the game's
default face), and the player's models are still built. The in-game face's keys (`iris_color`,
`[face]` and `[hair]`) are `ingame_face.toml`'s. One left in `settings.toml` under its old name
(`[appearance.face]`, `[appearance.hair]`) is an unknown key there.

The socks are laid out differently on a kit's main texture in PES 2015 to 2017 and in PES 2018
to 2021, so a kit drawn for one shows its sock design shifted around the leg in the other. An
empty file named `pre-fox` or `fox` in a kit folder says which games the kit's main texture was
drawn for: `pre-fox` for PES 2015 to 2017, `fox` for PES 2018 to 2021. With no such file the
compiler takes the kit as drawn for the game it compiles for. When the file names the other
games, `compile` moves the socks to the layout of the game it compiles for, on the kit's
`kit_mask` (PES 2015 to 2017) or `kit_srm` (PES 2018 to 2021) too. It says so with the note
`kit_layout_converted`, naming both layouts. The shirt, sleeves and shorts are laid out the
same in every version and are not touched. A
number texture (`kit_back`, `kit_chest`, `kit_leg`) made for the other games, its ten digits in
a column for PES 2015 to 2017 or in a row for PES 2018 to 2021, is re-arranged by `compile` for
the game it compiles for, told apart by its shape, whatever the marker says. The name texture
(`kit_name`) is left as it is.

A kit folder may hold a `colors.txt` giving the kit's two menu colors, and the export's root a
`colors.txt` giving the team's colors, up to four. Both files hold one color per line, written
`#c11200` or as three numbers from 0 to 255 separated by spaces or commas (`211 74 79`,
`211, 74, 79`), optionally after a label ending in a colon (`Shirt: #c11200`). A kit's file:

```text
211 74 79
162 62 77
```

Both commands read these files. Each line that does not give exactly one color (two
colors on one line, the way an old Team Note kit entry writes them, a number over 255, a hex
color without its `#`), and each color past the second in a kit's file or the fourth in the
root one, is reported as the warning `color_entry_invalid`, naming the line and the reason.
The line is skipped and the rest of the file is still read. `compile` writes the team's
colors into the game's team color file, which the menus and the scoreboard show. A color the
file does not give keeps the one the team had. A team export with no root `colors.txt` keeps
all the colors the team had, and `compile` says so with the note `team_colors_missing`.

`compile` also writes each kit's two colors and its icon into the game's kit color file, which
the kit selection menu and the scoreboard show. The icon is the number of the kit's `icon_<N>`
marker, or icon 3 when the kit has none. A `Full` export's kit whose `colors.txt` is missing
or gives fewer than two colors has its colors taken from its main texture, and `compile` says
so with the note `kit_colors_derived`. A `Full` export's kit with no main texture to take
them from (an empty kit folder, or one holding only other textures) gets magenta and black, so
the gap shows in the game's menus, with the warning `kit_colors_missing`. A kit a midcup export
does not hold keeps the colors and icon it had.

When the team color file or the kit color file `compile` starts from has damaged entries (a
team's colors written over the start of its entry, so the game no longer finds that team),
`compile` repairs them and reports the warning `bin_header_repaired`, naming the file and the
teams. Those teams' colors may be wrong until their exports are compiled again.

`compile` starts the team color, kit color and kit config files from the ones the game loads:
those of the CPKs your `DpFileList.bin` lists before the CPK being compiled, the nearest first.
It says which CPK each file came from (`bin_source`). A file none of those CPKs holds starts
from the copy built into the compiler, and so does every file when the PES folder has no
`DpFileList.bin` (`dpfilelist_missing`) or the list does not name the CPK being compiled. When
the list itself, a listed CPK, or a file the compile reads in a listed CPK (a bin, a kit
config or the record of the team's shared IDs) cannot be read or is not a valid file
of its kind, the compile stops before any export is read (`installed_bin_unreadable`), so the
CPK you had is kept.

On PES 2018 to 2021 the boots list, the gloves list and the player appearance table also come
from those CPKs (with `multicpk_mode` on, from the installed `bins_cpk_name` CPK itself
first), and `compile` points each compiled player whose boots or gloves it built at
them. The compiler has no copy of these files. A list none of them has is not
written, so those players' new boots, gloves or appearance do not show in the game, and
`compile` says so
with the warning `player_table_missing`, naming the list and how many players were left out. A
list of only this compile's players would take every other player's rows away.

On PES 2018 to 2021, a texture a model names must be in the folder (his linked shared
folders' textures count too; a `./` path naming a subfolder, `./textures/skin.dds`, at that
path alone), or for a path into the team's Common folder in the export's `Common` folder or in a
CPK your `DpFileList.bin` lists before the CPK being compiled
(a midcup export can use Common textures compiled into an earlier midcup CPK). A CPK listed
after it does not count, because removing it must not break this one. Otherwise that model's face,
boots or gloves is left out of the CPK, and `compile` reports `fmdl_texture_not_found` with the
model and the texture. When there is no PES folder or no `DpFileList.bin`, or the list does not
name the CPK being compiled, the texture may still be there, so the model is kept and the line
is a warning. A texture whose name starts with `dummy_` is never looked for. The game puts its
own in its place. `check` does not look for these textures, only `compile` does. A texture in
the game's own kit texture folder (`/Assets/pes16/model/character/uniform/texture/`) is not
looked for either. The modded exe swaps the files there for every match, so the model would show
whichever kit is being worn. Its path is kept and `compile` reports the warning
`fmdl_texture_kit_path`. A part that should follow the kit uses the `dummy_kit` textures or a
`kitN` texture of its own. A part that should not uses a texture of the player's folder or of
`Common`. A texture
link (`hair.dds.common`) may also name a texture that such an earlier CPK holds in the team's
Common folder, when the export's `Common` folder has no `hair.dds`. For these links `check`
looks in those CPKs too.

On PES 2015 to 2017, `check` and `compile` both look for the textures a `.mtl` names, and
report their findings the same way. A `./` path must name a texture of the player's folder (his
linked shared folders' textures count too), and a path into the team's Common folder
(`model/character/uniform/common/XXX/...`) a texture of `Common` or of a CPK listed before the
CPK being compiled, as above. A missing texture that a mesh uses is the warning
`mtl_texture_not_found`. The folder is still compiled, since the game shows some such faces as they were made, but look
at the player in the game. One that only a material no mesh uses names is an info line,
`mtl_texture_unused_missing`. A `dummy_` name and a path into the game's own folders are not
looked for. On PES
2018 to 2021 the same goes for the `.mtl` of a `.model` the compiler converts (one with no
`.fmdl` of its name beside it). A `.mtl` no such model uses is not read at all.

A folder named `templates` in the data folder holds files that replace the compiler's built-in
copies of the same name, so a cup can swap one without a new version of the compiler. The names,
spelled exactly as here, are `TeamColor.bin` and `UniColor.bin` (the team color and kit color
files a compile starts from when no installed CPK has them), `UniformParameter18.bin` and
`UniformParameter19.bin` (the kit config file a compile starts from when no installed CPK has
it, for PES 2018 and for PES 2019 to 2021), `placeholder_kit.dds` (the checkerboard texture of
a placeholder kit), `kit_mask.dds` (the mask a kit compiled for PES 2015 to 2017 gets when it
has none), `body.skl` (the skeleton a boots or hair model gets when its folder has
none), `face_diff.bin` and `fcl_hair_sim.fclo` (the face file a face gets when its folder has
none, and the hair simulation file a PES 2018 to 2021 face gets when its folder has none), `dummy.model` and `dummy.mtl` (the invisible stand-in
model and its material a PES 2015 to 2017 face gets when its `face.xml` lists no `face_neck`),
`DpFileList.bin` (the cup's official list, which a compile compares the game's with and
`upgrade-dpfl` installs), `placeholder.cpk` (the empty CPK `upgrade-dpfl` writes for a CPK of
the list that is not in the `download` folder, and `compile` writes as every unused `teams`
slot with `multicpk_mode` on), `env.dds` (the environment map a PES 2015 to 2017 compile gives
a converted metal material that names none), and `uniform.mtl` (the stock material set a
`.model` collar is read with).
Its folder `referees_fox` holds files that replace those of the referee kits and
appearance file a PES 2018 to 2021 referees CPK holds, and its folder `referees_prefox` those a
PES 2015 to 2017 referees CPK holds. A file there at the path it has in the CPK, spelled
exactly, replaces that one file (for example
`templates/referees_fox/common/character0/model/character/appearance/RefereeAppearance.bin`).
Any other file in the folder is ignored. `compile` and `upgrade-dpfl` name each file they used
with the note `template_override_active`. When they cannot read one of these files, or one of
the four kit and color files or `DpFileList.bin` is not a valid file of its kind, the command
stops before it does anything else (`template_override_unreadable`), so the CPK you had is kept.

An FPC player's body is hidden only when every kit config of the team, the goalkeeper kit's
included, carries the FPC values (shirt model 176, shorts model 16, collar 105, winter collar
105). So when any player folder of an export holds the marker `fpc_on`, `compile` builds every
kit config the export supplies with these values, and says so with the note
`kit_config_fpc_adjusted` for each `config.toml` that lacked them (your file is not changed). A
kit without a `config.toml` gets them anyway. A midcup export with `fpc_on` also gives these
values to the team's installed kit configs of the kits the export does not hold but the game
lists for the team, and names each kit it changed with the same note. A kit the game lists
with no installed config to change, or one that cannot be read, is reported as the warning
`kit_config_fpc_unpatched`, naming the kit. Send that kit in an export. A midcup export's
collar is given to those kits in the same way, after the FPC values, and a kit the game lists
with no installed config to wear
it, or one that cannot be read, is reported as the warning `kit_config_collar_unpatched`, naming the kit, unless
`kit_config_fpc_unpatched` has named it already. That kit keeps its own collar until you send
it in an export. The compiler never removes FPC values from a kit config. An export without `fpc_on`
has its configs built as they are, FPC values or not, and taking a team off FPC is an edit you
make in the configs yourself.

A kit config value the chosen PES version cannot hold (a name position `y` over 33 before PES
2021 or over 39 on PES 2021, or a shirt pattern of 12 or 13 on PES 2015) is lowered to one the
version can hold when the kit is built. Both commands report it as the warning
`kit_config_version_clamped`, naming the field, the value and the maximum. `compile` also
lowers such a value in an installed config it changes for a kit the export does not hold (the
FPC values or the collar above), and reports it the same way, naming the kit. The same `y` puts
the name at the same height on every version, so a config needs no change to move between them.

A kit config that asks for cut-out short sleeves, undershirt-only long sleeves or the tight fit
on a shirt model other than 144 or 160 is reported by both commands as the warning
`kit_config_option_ignored`, naming the option and the model. The config is built as written,
and the game ignores the option on that model. On a team whose kit-FPC status is on, the FPC
values set the shirt model to 176, so a config written for model 144 or 160 with one of these
options is reported with model 176, the model it is built with.

Both commands read every export in the exports folder from the settings (`exports/` beside
`UTW.exe` unless you changed it). To use another folder for one run, give its path
as `exports-root`. The setting is not changed. The `exports/` folder beside
`UTW.exe` is created for you when it is missing, and so is any relative path in
the `exports_folder_path` setting. An absolute `exports_folder_path`, or an `exports-root`,
must already exist, or the command stops before anything runs and tells you so. An exports
folder with no export in it is reported as `no_exports_found`, naming the folder,
and nothing is checked or compiled.

An export can be a folder, a `.zip` or a `.7z`, read where it is. Nothing is extracted. An archive
that cannot be read (damaged, password-protected, holding a file whose path leads outside it,
or holding two files whose names differ only in case or Unicode form) is reported as
`export_extract_failed` and left out of the run. A folder whose name ends in
`.upgrading` is not an export. An upgrade that did not finish left it there, and both commands
report it as the warning `export_upgrade_unfinished`, naming it. Remove it, then upgrade the old
export again.

When two exports of a run are for the same team (an old `co Full Spring` folder left
beside a new `co Full Summer.zip`), both commands skip each of them with the error
`duplicate_aesthetics_export`, naming the team's ID and the exports, and the other teams are
still built. Remove the one that should not be used, or disable it with an empty file named
`NO_USE` at its root.

`--export <path>` limits the run to one export: a folder, a `.zip` or a `.7z`, which does not
have to be inside the exports folder. Repeat it to name several exports. A path that does not
exist, or a file that is not a `.zip` or `.7z`, stops the command before anything runs.

`--no-deploy` builds the CPK into the output folder without installing it into the game, and
says where it is with the line `Info deploy_skipped_by_flag (path=...)`. It cannot be combined
with `--mode test` or `--mode sideload`.

Without `--no-deploy`, these lines say why the CPK could not be installed, and name the path in
the output folder where it was left instead (`output=...`), `pes_version_mismatch` aside:

- `pes_folder_not_found`: `pes_folder_path` does not name a folder that exists. Set it to the
  folder PES is installed in.
- `pes_version_mismatch`, a warning: the game folder holds no `PES20NN.exe` of the version you
  compile for. Check `pes_version` and `pes_folder_path`. The CPK is still installed.
- `dpfilelist_outdated`: the game's `DpFileList.bin` is an older list that does not have the
  CPK, so the game would not load it. `UTW team-compiler upgrade-dpfl`
  installs the cup's official list, which has it.
- `cpk_name_unlisted`: neither the game's `DpFileList.bin` nor the cup's official list has the
  CPK's name, so the game would not load it. Set `cpk_name` to a name the list has.
- `deploy_target_unwritable`: the `download` folder, or the old CPK in it, cannot be written,
  usually because the game
  is installed under `Program Files`. Run Unlimited Tool Works as administrator and compile
  again.
- `old_cpk_locked`: the old CPK could not be replaced, most likely because PES is running. Close
  PES and compile again.

Two warnings about the game's `DpFileList.bin` do not stop the install. `dpfilelist_not_official`
says the list is not the cup's official one, while it has every CPK of this run. The line names
the entries it is missing, the entries the official list does not have, or says that the order
differs.
`dpfilelist_cpk_missing` names CPKs the list has that are not in the `download` folder. The game
then loads none of the CPKs there. Both lines name `UTW team-compiler
upgrade-dpfl`, which installs the official list.

A game folder with no `DpFileList.bin` in its `download` folder is the error
`dpfilelist_missing`, and the CPK is left in the output folder the same way.

`upgrade-dpfl` installs the cup's official `DpFileList.bin` in the `download` folder of the game
folder (the `pes_folder_path` setting). It asks no question. Without `--yes` it writes
nothing and only prints, one line each, what `--yes` would do, then
`dpfilelist_upgrade_planned`, which names the command to run with `--yes`. With `--yes` it does
those things in this order, printing each line once it is done:

- `dpfilelist_cpk_renamed` (`from=...`, `to=...`): a CPK of an older DLC renamed to the official
  name of its kind, so the game still loads it under the official list. CPKs of one kind keep
  their order: `4cc_40_faces.cpk` and `4cc_45_uniform.cpk` become `4cc_41_teams.cpk` and
  `4cc_42_teams.cpk`, `4cc_30_stadiums0.cpk` becomes `4cc_20_stadiums.cpk`. One already under an
  official name moves too when a file before it in the list takes that name, and a CPK is never
  renamed over a file that is already there.
- `dpfilelist_placeholder_written` (`cpk=...`): the empty placeholder CPK written for a CPK of
  the official list that is not in the folder, since the game loads none of the CPKs there when
  one it lists is missing. A file that is there is never overwritten.
- `dpfilelist_replaced` (`path=...`): the list replaced by the official one. Your old list is
  kept beside it as `DpFileList.bin.bak` (`backup=...`), replacing an older backup. An old
  list that cannot be read as a list is replaced the same way, the line saying why
  (`unreadable=...`), and nothing is renamed. A folder with no list gets the official one, and
  no backup.
- `dpfilelist_cpk_dropped`, a warning (`cpk=...`, `size=...`): a CPK of your old list that the
  official list does not have and that could not be renamed (no official name of its kind left
  for it, a name not of the `4cc_NN_kind.cpk` shape, or its new name taken by another file). The
  game no longer loads it. The command never deletes a CPK. The line gives the file's size (no
  size when the file is not there), and removing it is up to you.

When the list already is the official one and every CPK it names is in the folder, the only
line is `dpfilelist_up_to_date`, and nothing is written.

`upgrade-dpfl` refuses to run (exit code 2) when `pes_folder_path` or the `download` folder in
it is not a folder. Set `pes_folder_path` to the folder PES is installed in. If it already is,
create the `download` folder in it. When a file cannot be
renamed or written (PES is running and holds the CPK, or the folder needs administrator
rights), it stops with exit code 3, naming the file. What it did before stays done, and the list
is replaced last, so the game keeps your old list until the end. Close PES, or run Unlimited
Tool Works as administrator, and run the command again to finish. A CPK it already renamed is not renamed
again.

`--mode sideload` is for trying a change in a running game. Instead of a CPK, `compile` writes
what the CPK would hold as loose files in the `livecpk` folder of the game folder (the
`pes_folder_path` setting), replacing everything that was there, for FoxDen (PES 2018 to 2021)
or Sider (PES 2017) to load while the game runs. The folder is replaced only once the whole
compile is written. A compile that fails, or writes nothing, leaves it as it was. Nothing is
installed into the game's `download` folder, so a missing `DpFileList.bin` is only a warning.
`--mode sideload` is refused for PES 2015 and 2016, which nothing can sideload into, and when
`pes_folder_path` does not name a folder.

`--mode test` shows what the compiler made of each export. Instead of a CPK, `compile` writes it
as loose files in the `test_output` folder of the output folder, one folder per export named as
the export is (an archive's with its extension), each file in the folder of the export it came
from: a player's models unpacked from their packages beside his converted textures, a kit's
files in its kit folder. The bins go in `test_output/_bins`, at the paths they have in a CPK.
Nothing is installed into the game, and the `overrides` folder is not applied, since its files
are not an export's. The folder is replaced only once the whole compile is written. A compile
that fails, or writes nothing, leaves it as it was.

The `multicpk_mode` setting is for the cup maintainers building the whole cup's DLC. With it on,
`compile` writes several CPKs instead of `<cpk_name>.cpk`. The teams go into the numbered CPKs
the cup's official `DpFileList.bin` has for the `teams_cpk_name` setting (`teams` gives
`4cc_41_teams.cpk` to `4cc_45_teams.cpk`, filled in that order), whole teams in the order of the
exports. A CPK takes teams until the next one would make it larger than `cpk_part_max_size`
bytes (3 GiB unless you changed it), and that team starts the next CPK. A team is never split
between two CPKs. Every one of these CPKs the teams do not need is written as the empty
placeholder CPK, so a CPK left from an earlier, bigger compile does not stay behind. The files
the compiler builds for every team (team colors, kit colors, kit configs, the boots and gloves
lists and the player appearance table) and the files of the `overrides` folder go into the CPK
named by the `bins_cpk_name` setting (`4cc_08_bins.cpk`). Every one of these CPKs is installed
into the game's `download` folder, or none is. The game's `DpFileList.bin` must have all of
them (a list without the
`teams` CPKs is one `dpfilelist_outdated` line naming them). When one cannot be installed
(PES holds it open, or the folder cannot be written), the old CPKs stay as they were and every
CPK of the compile is left in the output folder (`output=...` names the folder). With
`--no-deploy` every CPK is left in the output folder, with one `deploy_skipped_by_flag` line
each, the bins CPK first. `--mode test` and `--mode sideload` write their loose files as they
do without it.

These lines stop a multi-CPK compile with exit code 3. No CPK is written, and the ones in the
output folder stay as they were:

- `cpk_slots_exhausted` (`export=...`, `stem=...`, `slots=...`, `cap=...`): the teams do not fit
  in the numbered CPKs the official list has, at `cpk_part_max_size` each. The line names the
  first export left without room and how many CPKs the list has. Raise `cpk_part_max_size`, or
  ask for a list with more of them.
- `cpk_team_exceeds_cap` (`export=...`, `size=...`, `cap=...`): one team alone is larger than
  `cpk_part_max_size`, and a team is never split. Raise `cpk_part_max_size`.

A compile of one CPK is never split. When `<cpk_name>.cpk` is larger than `cpk_part_max_size`,
the warning `cpk_size_over_limit` (`cpk=...`, `size=...`, `cap=...`) says so. The game loads it
as it is and the CPK is written whole, but the cup DLC's repository cannot hold a file that
large.

For PES 2015 to 2017, the `dds_compression` setting zlib-compresses every `.dds` file `compile`
writes, the way the game's own compressed files are, which the game reads as it reads a plain
one and which makes the DLC smaller. A `.dds` of the export that is already compressed this
way, in a format the version reads, is written as it is. It is `auto` unless you changed it.
`auto` compresses when `multicpk_mode` is on and not otherwise, `true` always compresses and
`false` never does. PES 2018 to 2021 ignore it, since their textures are `.ftex` files.

A relative path typed in the terminal is taken from the folder the terminal is in.

When a command ends, its exit code tells a script how it went:

| Code | Meaning |
|---|---|
| 0 | Finished cleanly. Warnings and notes may have been reported. |
| 1 | Finished, but with an error reported, for example because something of an export was left out, or kept because `pass_through` is on, or the CPK could not be installed into the game and was left in the output folder. |
| 2 | The command line or a setting is wrong, for example a `cpk_name` that is not a valid file name. Nothing ran. |
| 3 | The run stopped early, for example because the teams list cannot be read or the output folder cannot be written. |

### The aesthetics patch and your save

On PES 2018 to 2021, every `compile` that builds players also writes `aesthetics_patch.toml`
in the output folder. It holds each compiled player's motions, taken from his
`settings.toml` (a motion the file does not set gets the game's default), and, only for an
export whose root holds the `write_names` marker file, his name and shirt name. The cup's
official save is built with that file. The cup maintainer compiles the exports and hands
over the CPKs and the patch, and the save builder applies the patch with `UTW
save-editor apply-patch`.

`compile` also applies the patch to your own save, so what you just built shows in your
game, but only when the game will load what the run installed: after a CPK was installed
into the game's `download` folder, and in `--mode sideload`. It is not applied with
`--no-deploy`, in `--mode test`, after an install that failed and left the CPK in the
output folder, or when `dpfilelist_cpk_missing` warns that the game will run without the
`download` folder's CPKs at all.

The `savefile_path` setting says which save is yours. `auto`, the default, looks for it
under `Documents\KONAMI` for the PES version you compile for, and picks the save written
last when the game has more than one account folder (the note `savefile_autodetected`
names it). Give a path yourself to use another save, a cup's template save for one.

While that PES version is running, the game writes the save it holds over the file when you
leave its Edit mode, so a save written while Edit mode is open is lost then. `compile`
therefore writes the save while the game runs only when you add `--edit-mode-closed`, which
says you have closed Edit mode. Without it, the save step is skipped with the warning
`savefile_skipped_pes_running`, which names the game and the flag: close Edit mode, then
compile again with `--edit-mode-closed`. The save is then written, and Edit mode's Load reads
it in without restarting the game. A PES of another version running does not count. When the
save is written, the one you had is kept beside it with `.bak` added to its name
(`EDIT00000000.bak`). A save that cannot be read, is for a PES version other than the one
compiled for, or cannot take the patch or be written (`savefile_invalid`,
`savefile_write_failed`) stops the
run with exit code 3. The new CPKs come out of the game again and are left in the output folder,
and the CPKs the game had go back in. In `--mode sideload` the same findings are errors
instead. The exit code is 1 and the new `livecpk` files stay in place. The warning
`savefile_player_not_stripped` names a compiled player in your save who still has an appearance
of his own (the save was never stripped, or the game's Edit mode restored it). The game shows
his save data, not the compiled one. `UTW save-editor strip-and-seed` strips a
save never stripped, and `UTW save-editor check-stripped` with `--strip-again`
and `--out` clears the appearances Edit mode brought back. When no save is found at all, that is
only the warning `savefile_missing`, which names the patch file and the command that applies it
by hand (a `savefile_path` naming a folder is `savefile_invalid` instead).
