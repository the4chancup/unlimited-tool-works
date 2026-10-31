# Provenance scripts

One-shot scripts and one harness that produced the fixtures and measurements
cited from `docs/plans/`, `docs/DECISIONS.md` and the `tests/fixtures/README.md`
files. Each ran once on the writing machine and hardcodes that machine's paths;
most need the legacy tool sources (`4cc-aet-converter-19to16`,
`pes-stadium-compiler`) and PIL/numpy. Nothing here is called by a `just` recipe
or a gate. They are kept as evidence of method, so a number in a plan can be
traced back to how it was measured, not as maintained tooling.

- `fixtures/` — generated the committed test fixtures for `pes_savefile`,
  `archives`, `fox2` and `dds_convert`, cited by their fixture READMEs.
- `music_export/` — `list_4ccm.py` lists every `.4ccm` on the machine through the
  Everything HTTP API into `.tmp/phase_10/music_export/all_4ccm.txt`, the input of
  `music_export`'s census test (`tests/census.rs`, skipped when the list is absent).
- `pes_model/` — the `.model` layout census behind the section table in
  `docs/plans/libs/format_crates.md` and the count note in `docs/DECISIONS.md`.
- `fmdl_bone_matrix/` — the bind-pose noise census, gap and histogram scripts
  behind the `1e-4` merge threshold in `docs/DECISIONS.md` (`libs/format_crates.md` carries
  the numbers).
- `kit_uv/` — the kit UV island measurement and diff images behind the Fox to
  pre-Fox layout notes in `docs/plans/team_compiler/pipeline.md` and `docs/DECISIONS.md`;
  the sock table's measurement (`kit_uv_sock_angle.py`), the check that the shorts did not
  move (`kit_uv_pants_height.py`), the number atlases' arrangement (`kit_leg_atlas.py`), and
  `kit_layout_fixture.py`, which writes `team_compiler`'s `tests/fixtures/kit_layout/`.
- `tracer_prefox/` — cut the pre-Fox tracer export from a cup export on the writing machine
  (`make_old.py`), staged Red to compile it (`setup_red.py`) and migrated it to the Studio
  layout (`make_studio.py`): `team_compiler`'s `tests/fixtures/tracer_prefox/`, whose README
  gives the steps.
- `tracer_refs/` — cut the referee tracer export from a Fox referee export on the writing
  machine (`make_old.py`) and staged Red to compile it (`setup_red.py`): `team_compiler`'s
  `tests/fixtures/tracer_refs/`, whose README gives the steps.
- `red_main/` — `run_red.py` compiles a tracer fixture's `old/` export with released Red (its
  checkout on `main`) from a copy of Red's folder: the `red/` runs of `tracer/` and
  `tracer_prefox/` behind worklog step 6.11, whose READMEs give the result.
- `calib/` — a standalone Rust crate (own `[workspace]`, so it stays out of the
  root one) that measured kit color calibration for `docs/plans/libs/color_tools.md`.
  Written against the `dds_convert`/`kit_config` API of its day; it currently
  compiles (`cargo check` clean as of the move).
