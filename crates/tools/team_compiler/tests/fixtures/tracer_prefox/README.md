# Pre-Fox tracer fixture

One real player with a face, boots and gloves, and one kit, as an old-layout export, its
Studio-layout twin, and Red's compiled output for the old one for PES 2017: the pre-Fox
counterpart of `../tracer/`, compared by `tests/parity_prefox.rs`. The scripts that made it
are in `scripts/provenance/tracer_prefox/`, but for the `red/` run, `scripts/provenance/red_main/`.

## `old/jp Tracer/`

Cut down by `make_old.py` from the /jp/ export "JP Aesthetic Export for Summer 18", a pre-Fox
(PES 2017) export whose files carry the team's old ID 728 (/jp/ is 731 in today's teams list):
the player "Fumos" (`Faces/72820 - Fumos/`, as `XXX20 - Fumos`), the one player whose
`face.xml` lists a face, boots and gloves, and kit `g1` (`Kit Configs/728/728_DEF_GK1st_realUni.bin`
as `Kit Configs/XXX_DEF_GK1st_realUni.bin`, `Kit Textures/u0728g1.dds` as `u0XXXg1.dds`), so
every name follows the `XXX` placeholder convention. Changes from the source:

- the player's hair set (`hair.mtl`, `hair_high_win32.model`, `hair_col.dds`,
  `hair_parts_col.dds`) is left out: his `face.xml` does not list it, so the game never loaded it;
- to keep the tree small, `face.dds` and `k2012_c.dds` (DXT1, 512×512) keep every fourth 4×4
  block in each direction (128×128), `k2012_sr.dds` (uncompressed 32-bit, 128×128) every second
  pixel (64×64), and the kit texture (DXT5, 2048×2048) every eighth block (256×256); headers
  are rewritten to match, the formats are unchanged;
- `jp note.txt` (the export's `JP Note.txt`) loses the five player kits' color lines, whose
  kits are not in the cut, and its player list keeps Fumos's line alone, as `XXX20`: one name
  in the list is not UTF-8, which Red cannot read; `ID: 728` is `ID: 731`.

Everything else is byte-identical to the source, quirks included: `boots.mtl` and six of the
DDS files are WESYS-wrapped, `k2012_sr.dds` is uncompressed, `face.mtl` names a
`face_edithair_specular_roughness.dds` the folder does not hold (Red: a Warning), and two of its
materials name no states (Red: an Info). The member's `face.xml` types the boots `boots`.

## `studio/jp Midcup Tracer/`

`old/` migrated to the Studio layout by hand (`make_studio.py`), for what `compile` handles:

- `Players/20 - Fumos/`: every file of the old face folder but `face.xml`, which the compiler
  generates; its `<dif>` decoded from base64 as `face_diff.bin`. The models renamed to the
  Studio's names, `face_high.model`, `boots.model`, `glove_l.model`, `glove_r.model` (each
  finds its `.mtl` by a name that starts or ends its stem: `face.mtl`, `boots.mtl`,
  `glove_l.mtl`, `glove_r.mtl`), their bytes unchanged.
- `colors.txt`: the note's team colors, `058 080 177` and `255 255 255`.
- `Kits/g1/`, since step 4.16a (until then a kit refused the whole export on PES 15-17, and
  the twin held none). `make_studio.py` writes it with `WITH_KIT = True`: `kit.dds` is
  `u0XXXg1.dds`, `config.toml` the GK config decoded as PES 17 by `kit_config`, `colors.txt`
  the note's `1st GK` entry, `#FFFFFF - #000000`.

## `upgraded/jp Midcup Tracer/`

What the Export upgrader makes of `old/`, as it wrote it: the link that lets a parity test
compile an upgraded export against `red/` without a tool crate depending on another. Made at
commit `0e52ddf` by `4cc-studio export-upgrader upgrade "old/jp Tracer" --loose -o <dir>`
(report: `migration_clean`, exit 0). It was regenerated with `--coverage midcup` when a
`Full` export with no logo became an Error, the tracer being a cut-down export: the files are
unchanged, only the folder's name and the report's `coverage_tag_chosen` line
(`tag=Midcup, source=flag`) differ. `export_upgrader`'s tests check that upgrading `old/`
still gives exactly this tree, so a change to the upgrader's output regenerates it the same
way. Against the hand-made twin in `studio/`:

- `Players/20 - Fumos/` keeps the member's `face.xml` byte for byte, so the models keep the
  names it lists (`face_high_win32.model`, `oral_boots_win32.model`, `oral_glove_l_win32.model`,
  `oral_glove_r_win32.model`) and no `face_diff.bin` is split out;
- the folder holds a generated `settings.toml`, `name = true` and every other key at its
  default;
- `colors.txt` and `Kits/g1/colors.txt` carry the note's colors in the note's own forms
  (`058 080 177`, `#FFFFFF`), where the twin writes lowercase hex;
- `Kits/g1/config.toml` has no `[source_texture_names]` table.

## `red/`

The entries of the CPK Red compiled from `old/`, extracted as stored with the `cpk` crate. Red
4.4.0, commit `1a48fa5` of `4cc-aet-compiler-red`, branch `main`, run from a copy of its folder
(`scripts/provenance/red_main/run_red.py tracer_prefox 17`) with `settings.ini` at its defaults
except `pes_version = 17`, `cpk_name = 4cc_90_tracer`, `move_cpks = 0`, `pause_allow = 0`,
`updates_check = 0`, and `pes_folder_path` pointing at a folder that does not exist, so every
bin was built on Red's fallback base. Command:
`Engines\embed\python.exe Engines\compiler_main.py 0 --no-pause` with `exports_to_add\jp Tracer\`.
Red reported the missing texture above as a Warning and the materials without states as Info.

The tree was first made with commit `e12aa01`, on the unreleased `dev/pre-studio-format`
branch (staged by `setup_red.py`). Released Red's run (2026-10-19) gives the same six files,
five of them byte for byte; the sixth, the face CPK, holds the same 20 entries byte for byte,
its own bytes differing in its `ETOC` table alone, the entries' modification times (Red stamps
the files it unwraps, `boots.mtl` here, with the run's time, so two runs of the same commit
differ there too). The committed face CPK is released Red's.

What the comparison found when it was written (the test's tables carry each row):

- the four models are byte-identical; the face's packed name gains the compiler's `oral_`;
- Red keeps a team player's textures in his face CPK, beside `.mtl` paths `./<name>`; the
  compiler relocates them to `common/character1/model/character/uniform/common/731/20 - Fumos/`
  as on Fox (`pipeline.md` step 6), with the same pixels: it writes a WESYS-wrapped file
  unwrapped and every header in one form (mip count 1 with its flags), and keeps the
  uncompressed `k2012_sr.dds` uncompressed, as Red does;
- the `.mtl` files define the same materials once the relocated paths are normalized; Red
  unwraps a WESYS `.mtl` too;
- `face.xml`: the same `<dif>` bytes; the generated entries type the boots `parts`, as Red's own
  generator does (the member's file, which Red keeps, says `boots`);
- `TeamColor.bin` is byte-identical;
- since 4.16a, with the twin's `Kits/g1/`: the mask is byte-identical (both write Red's
  template, which the compiler bundles); `u0731g1.dds` has the same pixels, Red copying the
  source file as it is (GIMP's reserved header bytes, no mip flags) and the compiler writing
  its canonical header; Red's kit config keeps the names of the four number textures the cut
  does not ship, where the compiler's encoder leaves an absent texture's name empty, and the
  rest of the 120 bytes is the same; `UniColor.bin` holds the same `g1` entry for team 731,
  and the rest of the compiler's file is the bundled base's, the team's past cup's record
  included, where Red's fallback base held an empty record for the team.

The reference tree is about 660 KB, small enough to commit whole rather than as a hash
manifest (`testing.md` "Testing: parity against Red"), so the parity test runs anywhere, CI
included, without Red. The three trees are about 1.7 MB on disk and about 840 KB as git
stores them: the models are the same blobs in `old/` and `studio/`.
