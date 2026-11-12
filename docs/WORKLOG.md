# 4cc Studio — Worklog

Where the implementation is and what comes next. The plan (`docs/plans/`) holds the *why*;
`DECISIONS.md` holds choices made where the plan was silent; this file holds *where we are*.
Never duplicate rationale here — link to the plan section instead. Procedure for using this file
is in `AGENTS.md` ("Working documents").

---

## Current status

**Now: Release 0.0.2, the VGL preview** (`plans/core/development_plan.md` "The /vg/ League 27
(VGL)"). The cut holds every open 0.0.2 step (DECISIONS 2026-11-11 "0.0.2 holds every open
0.0.2 step"), worked in this order:
1. Update now (0.0.2-upd, its slices below; the command line's notice, 0.0.2-cli, is in),
   then the TLS provider (0.0.2-tls).
2. The illegal formation check (0.0.2c): its six clauses are set; the game's own formations
   and the cup saves are still to run under them.
3. The Save editor asks of 2026-11-03 still open (0.0.2-undo, -statlbl, -inst, -drift,
   -clip).
4. The measurements: the compile profile against Red (0.0.2d), `codegen-units` (0.0.2-cgu).
5. The library checkpoints (0.0.2-chk), then the cut.

**The PoC for VGL's organizer:** 0.0.2-poc.5 is built (`4cc-studio-temp/poc/target/dist`,
archive SHA-256 `cac0447a…`, branch `vgl27` at `61d224c4`: main of 2026-11-12 at
`cfd15a27`, so with the launchers' logs, the readme's backup and list steps, Update now's
Restart and the Save editor's three-column comparator), beside poc.4 and the older ones.
Both launchers were run from the unpacked archive under a pseudo-console, over a fake PES 21
install and the tests' fixture exports. Its in-game check and its hand-over are the
maintainer's.
**Blocked on:** nothing.
**Behind 0.0.2:** Phase 8's other views and Release 0.1.0, then the phase order 1–6, 8,
0.1.0, 7, 9–16 (`plans/core/development_plan.md` "Releases"; the first-class target is the
Fox version the cup moves to around April 2027, "Target versions"). Phases 1 to 6 are
closed; 0.0.1 was released 2026-11-03 (tag `v0.0.1` on `c97ce914`). Built ahead of their
phases: 10.0a-g (`music_export`), 16.w (the platform floor), 16.x a (`dds_convert`'s GPU
BC7 half, unwired), 20.0 to 20.6a2a (the Ruleset editor, resuming after 0.1.0).
**Planned for after 0.0.2** (the maintainer, 2026-11-12): the Team decompiler is now the
Decompiler, for every export kind (`plans/decompiler.md`, a stub: team, referee, balls,
stadium and billboards exports from a DLC), and each kind's compile packs a recovery manifest
(`plans/team_compiler/pipeline.md`, `plans/balls_compiler.md` and
`plans/stadium_compiler.md`, "Recovery manifest"). The one Compiler those kinds are compiled
by was already decided (DECISIONS 2026-11-09, `plans/core/README.md` "One Compiler"; its
respelling of the plans is the issue of 2026-11-09 below). Reviewed in one SWE-2 round (tier
3): twelve concerns accepted and in the plan.

**Working mode.** `RUNBOOK.md` "How the work runs": the lead plans, briefs, reviews and
writes what is correctness-critical; sidekicks implement, two or three at once; a review's
depth follows its surface's tier. A review that cannot run is queued in the lead's handover,
and no phase closes with one outstanding.

---

## How to resume

| Thing | Where |
|---|---|
| Plan index | `docs/plans/README.md` → `docs/plans/core/README.md` |
| Development phases | `docs/plans/core/development_plan.md` "Development Plan" |
| Legacy tools (format evidence) | `docs/spec/core/README.md` "Project context" (paths are per-machine) |
| Skeleton data | `resources/skeletons/` (see its README) |
| FPC source text (for the `fpc` crate) | `resources/FPC.wikitext` |
| `.model` format as measured (for the plugin author; the crate's notes are in `libs/README.md`) | `resources/prefox_model_format.md` |
| Coding rules and verification gates | `docs/CONTRIBUTING.md` |
| Domain terms | `docs/GLOSSARY.md` |

### Current-state gotchas

Things that are true *right now* and would cost the next session time to rediscover (a broken
fixture, a flaky test, a crate that doesn't build on one platform). Remove an entry when it stops
being true. Permanent toolchain facts go in `docs/CONTRIBUTING.md` "Toolchain notes" instead.

- **Until 0.0.1's cut, this machine's CPU goes to 0.0.1** (the maintainer, 2026-10-27; DECISIONS
  "Until the 0.0.1 cut, no test run reaches past what 0.0.1 needs"): work and test runs on
  anything 0.0.1 does not strictly need are queued (Phase 20's 20.6a2b and 20.7, the "after
  0.0.1" steps, mutation runs of crates outside 0.0.1). 0.0.1's own verification keeps this
  machine at full speed: a step or a merge is gated locally with `just gates-changed <its
  start>` (the changed packages and every package depending on them), and 0.0.1's mutation
  runs stay split. Only what no 0.0.1 change reaches moves off it: a full-workspace `just
  gates-remote` on bonfire when bonfire is free, blocking nothing. Remove this entry at the cut.
- `just` needs PowerShell as its Windows shell (`set windows-shell` in the justfile): Git for
  Windows' `sh` is not on PATH from a plain PowerShell. Recipes still stop at the first failure.
- `cargo install just cargo-deny` is required once per machine (both take a few minutes to build);
  the pinned toolchain and the wasm32 target install themselves on the first `cargo` call.
- `dds_convert`'s CPU BC1 core (`block_compression`, PCA plus one refinement) trails DirectXTex
  by about 2.5 mean error on an 8x4 grey-ramp mip while matching it within 1.0 at the top mip;
  the encode tests compare the chain-wide mean for that reason. Revisit with representative kit
  and face textures when the Team compiler can produce them (plan: "tune against representative
  textures").
- `just mutants-diff` reuses `mutants.out/`: it wipes a whole-crate run's results. Keep the
  whole-crate run's summary aside (to `.tmp/mutants_<step>_whole/`, only `*.txt` and
  `outcomes.json`: `RUNBOOK.md` "The scratch folder") before the first diff run.
- `block_compression` 0.10's BC3/BC4/BC5 decoder truncates the alpha-ramp interpolation where
  DirectX rounds; `dds_convert::dds::fix_interpolated_channels` re-derives those channels. Drop
  it if a later release rounds (the exact-match test will say).
- `studio_core`'s `pes_process.rs` is split by `cfg` per OS, so a mutation run reports the other
  OS's mutants as missed: a Windows-only function's mutant tested on the VPS's Linux half (and a
  Linux one on this machine) compiles out. Triage those by OS, not as test gaps. Windows gates
  never compile a `target_os = "linux"` branch either (5.4a6's did not build): `cfg`-split code
  gets `just gates-remote` before it lands.
- A compile killed between moving a CPK aside and renaming its `.partial` in leaves that CPK
  missing, and one killed between the install and the savefile write leaves the new CPKs with
  the old save; the next compile repairs both (0.0.1c, `.tmp/release_0_0_1/0_0_1/interrupt/`).
- The VGL27 Friendlies set (39 teams, the organizer's save) is sorted in main's
  `.tmp/vgl27f/sorted/` (`sort_copy_c.py`): `F:/VGL27 Friendlies sorted` was found emptied.

---

## Phases

Status: `todo` · `in progress` · `done` · `blocked`. A phase is done only after its last two
steps, which every phase has: **converge** (the lead's own audit first, then the cross-family
reviewer's, both against the phase's plan sections and acceptance IDs; each gap becomes a new
step above it, and the phase waits for them) and **rewrite**
(what the phase delivered moved from `docs/plans/` to `docs/spec/`, corrected where it was built
differently; `RUNBOOK.md` "Phases and releases"). Then collapse its step list
below to this one row, keeping its `manual: checked` lines (the step-level detail stays in git
history), and bring the root `README.md` to the closed phase. Tool phases (3–6, 8–15, 17, 19, 20) also
open with an **acceptance** step: the tool plan's "Acceptance" section for that phase, written
before any code (GUI scenarios that no automated test can prove are marked `manual` and proven by
a recorded check at converge — `CONTRIBUTING.md` "Testing"). Procedure: `RUNBOOK.md` "Phases and
releases".

| Phase | Scope | Crates | Status |
|---|---|---|---|
| 1 | Workspace bootstrap + core skeleton | workspace, CI, non-GUI `studio_core`, `vtree`, `pes_version` | done |
| 2 | Library crates (standalone-verifiable) | `wezlib` `cpk` `fpk` `ftex` `dds_convert` `fmdl` `pes_model` `uniparam` `fox2` `archives` `fpc` `teams_list` `kit_config` `color_tools` `elevation` `model_convert` (native) `pes_savefile` `python_bindings` | done |
| 3 | Team compiler skeleton | `team_compiler`, `aesthetics_export`, `pipeline` | done |
| 4 | Processing logic | `team_compiler` (`plan/` `processing/` `bins/` `output/`), `aesthetics_export` deep validation | done |
| 5 | Savefile integration | `save_editor` logic, `aatf`, `team_compiler` `output/savefile.rs` | done |
| 6 | Export upgrader | `export_upgrader` | done |
| 7 | glTF support (runs after Release 0.1.0) | `model_convert` (glTF half) | todo |
| 8 | GUI | `studio_core` shell, `studio`, tool views, `color_tools` widget | todo |
| 0.0.1 | Preview build: the Save editor for DBC alone, with DBC7's rules file and PES 2017 picked (`core/development_plan.md` "Divegrass Before Christmas") | `just release` | todo |
| 0.0.2 | Preview build: 0.0.1 plus the Team compiler and Export upgrader through their CLI for VGL 27, with the launchers (`core/development_plan.md` "The /vg/ League 27 (VGL)") | `just release`, the launchers (the console's progress line done in 0.0.1a) | todo |
| 0.1.0 | Release 0.1.0, after Phase 8, a test release (`core/development_plan.md` "Release 0.1.0") | `just release`, `CHANGELOG.md` | todo |
| 0.2.0 | Release 0.2.0 or a later `0.x` (`core/development_plan.md` "Release 0.2.0") | help chapters, update check + notice, teams-list merge | todo |
| 9 | Stadium compiler | `stadium_compiler` | todo |
| 10 | Music tools | `music_player`, `music_export_editor`, `music_export`, `audio_engine` | todo |
| 11 | Match tracker | `match_tracker`, `match_feed` | todo |
| 12 | Kit config editor | `kit_config_editor` | todo |
| 13 | Refs arranger | `refs_arranger` | todo |
| 14 | Balls compiler | `balls_compiler` | todo |
| 15 | Player aesthetics editor | `player_aesthetics_editor` | todo |
| 16 | Polish and distribution | — | todo |
| 17 | Team creator (post-release) | `team_creator` | todo |
| 18 | Studio Web (post-release) | — | todo |
| 19 | DB generator (scheduled by need, after 2 and 8) | `db_generator`, `pesdb`, `pes_savefile` `ops/populate.rs` | todo |
| 20 | Ruleset editor (post-release) | `ruleset_editor`, `aatf` writer | todo |

---

## Steps

`[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked / needs a decision

A step is done when its own `→ verify:` check has actually been run and passed, on top of the
gates. Every step carries one when itemized; if a step is listed without one, the agent writes it
before starting the step (a specific check, not "test it") and puts it in the step text. Mark a
done step with a one-line summary and the files or crates touched. One `[~]` per agent at a time.

### Phase 1 — Workspace bootstrap + core skeleton

Done 2026-09-13 (spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 1"). Step detail
in git history up to commit `2f158a9`. CI proof: green run on `5e6ffc6`, deliberately red run on
`eccf0f3` (both `gates` jobs failed at `just gates`, `deps-check` unaffected), reverted in
`2f158a9`.

### Phase 2 — Library crates

Done 2026-09-30 (spec now describes what exists: `docs/spec/core/development_plan.md`
"Phase 2", `libs/README.md`, `model_conversion/README.md`, `pes_savefile/README.md`). Step
detail (2.1-2.21: each crate's build, review rounds A-C, the 2.20a-k converge with its
mutation runs, censuses and reviewer rulings) in git history up to commit `7de61f7`. Carried
forward: 2.5b GPU BC7 is step 16.x, the `dds_convert` cache bound is step 4.y, and the
phase's open questions are under "Issues".

### Phase 3 — Team compiler skeleton

Done 2026-10-02, its cross-family reviews still queued (the maintainer's directive; "Handover"
lists them, 3.7's and 3.8's among them; what they find becomes steps under this heading). Spec
now describes what exists: `docs/spec/core/development_plan.md` "Phase 3",
`team_compiler/README.md` ("Acceptance": 77 scenarios, 75 cited by tests, 2 manual),
`spec/team_compiler/pipeline.md` "Run driver shapes", `aesthetics_export/object_model.md`,
`libs/pipeline.md`. Step detail (3.1-3.9f, the 3.w `.cargo/mutants.toml` audit, the 3.y
converge with its whole-crate mutation runs, the 3.z shell slice, 3.x this rewrite) in git
history up to commit `32261dd`. Carried forward: the phase's open issues under "Issues".

TC-GUI-01 manual: checked 2026-10-02 by the maintainer on Windows, the window, the sidebar's
Team compiler entry, its Settings, Compile button and log.
TC-GUI-02 was checked by hand the same day (the /egg/ tracer export compiled from the window,
the log showing `egg Tracer: Info export_identified (team=/egg/, id=792)` then `Run finished:
exit code 0`); since 8.0 it and TC-GUI-03 are automated, cited by the headless view and shell
tests, and TC-GUI-01 is narrowed to the window opening. Linux: not yet checked, no machine
with a display.

### Phase 4 — Processing logic

Done 2026-10-12. Spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 4",
`team_compiler/README.md` ("Phase 4 scope"; "Acceptance": 363 scenarios, 2 manual, 5
withdrawn, all proven, every CLI test citing one), `team_compiler/pipeline.md` (the walkthrough
and "Run driver shapes"), `messages.md`, `settings.md`, `aesthetics_export/object_model.md`.
Step detail (4.0-4.34; the 4.c Clef passes; the 4.y converge: the lead's audit and the export
census, 4.y-fix1 to 4.y-fix23, 4.y-sub, 4.y-7z, 4.y-kitwarn, 4.y-mattable, 4.y-simp,
4.y-scen, the S7-S20 review rounds, the in-game harness and checks; 4.z this rewrite) in git
history up to commit `c93f025`. The converge's closing whole-crate mutation runs, all on bonfire
over `6e13fae`: `aesthetics_export` 519 caught, 0 missed; `pipeline` 54 caught, 6 missed
(`available_memory`'s arms for other platforms, accepted in `.cargo/mutants.toml`), 8
timeouts (`MemoryBudget::acquire` waits a mutant makes endless, caught); `team_compiler`
2205 caught, 10 missed, 3 timeouts, 384 unviable: the five sites filing a texture as the folder's own or a combined folder's got tests (`c93f025`), `own_package`'s
`ingame_face` guard is excluded as read by no consumer, the three `deploy.rs` error-kind
guards (tests Windows-only; the run was Linux) and the timeouts (two endless loops, one
64-fold texture) were caught when rerun on Windows, `settings_view` is the placeholder
label. Carried forward: the Fox cube map check (4.y-ingame2 (d), waiting on a hand-made FMDL; its
`QUESTIONS.md` entry), the phase's open issues under "Issues" (the pre-Fox
ones deferred until the cup next plays a pre-Fox version), the in-game questions in
`docs/QUESTIONS.md`.

The phase's in-game checks (TC-REF-04's at 4.19d; 4.y-ingame; 4.y-ingame2):

4.19d:
manual: checked 2026-10-09 by the lead on PES 17 through the harness (`.tmp/phase_04/4_19/ingame/`, results in `.tmp/phase_04/4_0/apptest/results.txt`): the referee present with the pair installed, nothing drawn; a `.mtl` beside a nocloth `.model` is read and stops the model drawing (runs E and F against 2026-10-08's run I), so the pre-Fox marker goes by Red's route instead: step 4.19f, DECISIONS 2026-10-09; the scene where the pre-Fox marker shows is a maintainer question

4.y-ingame:
manual: checked 2026-10-10 (a): PES 21's stock `collar_107.fmdl` as `Collars/collar_039.fmdl`
of an `/a/` Midcup export, converted for PES 17 (`uni_collar` then `uni_shirts`, two
meshes), drawn on the keeper WIDEFACE in Edit mode as the laced polo it is, in the GK
kit's black (`q03_neck.png`); the baseline, PES 17's own `collar_039.model` packed as
it is, draws a plain stand collar (`r02_neck.png`).
manual: checked 2026-10-10 (b): the tracer's `fcl_hair.fmdl` converted for slot 01 and
02, its body material naming `common/702/dummy_kit.dds`, draws kit 2's texture on
both (the keeper wears the GK kit, slot 02 kit 1), and a green checker shipped at that
path is not read (`p20_appearance.png`, `q04_slot02.png`): the plan's "active kit"
corrected, DECISIONS 2026-10-10; no compiler change.

4.y-ingame2:
manual: checked 2026-10-10, the portrait header variants on PES 21: the four files
of `C:/Data/4cc/Tools_Mine/temp/variants/` as the portraits of `/out/` bench players
(`.tmp/phase_04/4_y/ingame/build_portraits_cpk.py`), each hovered alone in Kick Off's team
sheet after its neighbour (the maintainer's method): A (DX10 header, BC3, sRGB id,
one level) and B (the same with the UNORM id) crash the game on hover; C (the
legacy `DXT5` header) and D (BC7 under DX10, full chain) draw
(`v21_portraits_sheet.png`). So the DX10 header itself, not the colour-space id,
crashes the Fox games' BC3 portrait reader, and the compiler's re-headering (4.y-fix6)
is the right fix; BC7 stays under DX10, which draws. The cup's eight BC7 portraits (ESG
XXX07-11, FGOG XXX13/21/23) are 128x128 with eight levels under DXGI 98, D's shape, so
nothing of the question is left (its QUESTIONS entry removed).
manual: checked 2026-10-10, collars beyond the stock set on PES 21: Test 3's CPK
(`.tmp/phase_04/4_0/apptest/out_test3/pes21/`, team `/out/`: kit 1's collar 200, a copy of the
game's `collar_107.fmdl` under that ID; the GK kit's collar 201, no model anywhere)
in the test slot, Edit mode's Appearance view: the outfielder draws a folded collar
(`p21_19_collar_front.png`), the keeper a plain neckline, no crash
(`p21_22_gk_front.png`): an ID beyond the stock set loads when its model exists.
manual: checked 2026-10-10, the sock table on PES 21, in part: sock design A
(`socks_design_prefox.png`, converted) on `/out/`'s kit 1: the thick white stripe at
the front, the thin red one and the yellow disc on the outer side, both socks alike,
the stripes straight, no seam at the visible part (`p21_legs_sheet.png`); the
diagonal and the hoop sit under the team's gaiter boots, and the PES 17 reference
view is still to take (open item (a)).
manual: checked 2026-10-10, the sock table's PES 17 reference and the lower sock:
Test 3's kit (design A, laid out for PES 15-17, so no conversion) compiled for
`/out/` on PES 17 with the cup's own heads, whose PES 17 players wear the game's
boots: the thick white stripe at the front, the thin red one at the back, the
yellow disc on the outer side, the green diagonal band on the lower calf and the
orange hoop at the ankle, as drawn (`s17_34_out_legs.png`, `s17_35_out_legs_side.png`,
`s17_36_out_legs_back.png`); PES 21's converted upper part matches it feature for
feature. The lower part on PES 21 still waits for a player without the gaiters: a
`settings.toml` with `boots_id = 0` writes no table before Phase 5, and a blank
`face.fpk` in the test slot did not replace `/out/`'s cup heads in PES 21's Edit
mode (`w21_24_roster.png`), an observation for Phase 5's own checks. Method:
`/a/`'s save hides its players' standard bodies (billboard models), so any face put
on an `/a/` slot draws with no body on PES 17; the Face view draws the face package
alone, and the whole body shows in the Appearance view zoomed out or in Strip Style
(`scripts/ingame/README.md`).
manual: checked 2026-10-10, the stock collar set: the template config's collar and
the version's last stock ID (116 on PES 17, 131 on PES 21) each draw a collar on
`/out/`'s standard bodies, a crew neck with the kit's own V at the back, 116 a
V-neck under the long-sleeve toggle (`s17_38_collar_front.png`,
`s17_42_c116_collar.png`, `s17_47_sleeves_long.png`, `w21_17_row5.png`); the
QUESTIONS entry closed.
manual: checked 2026-10-10, `kit_config::validate`'s two Infos: shirt model 150
(outside the documented 144, 160 and 176) draws a sleeveless shirt on PES 17 and
PES 21, no crash (`s17_40_model150.png`, `w21_17_row5.png`), a model the game has,
so `kit_shirt_model_unknown` names a documentation gap, not a bad value;
`long_sleeves = 3`, an undocumented raw value, leaves the short-sleeve view as it
is and draws the long-sleeve variant without sleeves on PES 17
(`s17_47_sleeves_long.png`), no crash. Both stay unreported by the compiler
(DECISIONS 2026-10-10); open item (e) closed, (a) closed but for PES 21's lower
sock.
manual: checked 2026-10-10, the 192x512 mipped texture (c, first half): galosengen's
`Common/scroll.dds` (192x512, DXT5, ten levels) compiles for PES 17 with no finding
(the pre-Fox rule has been `texture_not_div4` alone since 2026-10-07; the question's
"both engines" was stale) and, as the `face.dds` of the Fumos head on eight `/out/`
players, draws its content (black with brick-red squares, `scroll_source.png`) on
their heads in Edit mode's roster preview (`t17_rows_sheet2.png`, rows 16-23, against
the fixture's red texture on rows 1-15): PES 17 reads a mipped texture whose sides are
not powers of two, so `texture_not_pow2` stays Fox's. Export built by
`.tmp/phase_04/4_y/ingame/build_tagtex_export.py`.
manual: checked 2026-10-10, the `.model` mesh tag (c, second half): the stock
`konami_modD_cap.model` is PES 17's captain armband (its one mesh tagged
`DCaptainmark`, material `modD_cap_phone` with no texture), put as `cap.model` on
`/out/` players with a blank face: the tagged one and a copy with the tag renamed
`DZaptainmark` both draw the white band on the left arm in Edit mode's Appearance view
(`t17c_face_01.png`, `t17c_face_08.png`; none on the control, `t17c_face_16.png`), and
in an exhibition match (/acccc/ against /wsg/, the tagged band on /wsg/'s slots 01-11
with blank faces, the untagged one on 12-23 under the Fumos head) a headless
non-captain wears the band in the replay's ground camera (`t17m_free_left.png`; the
captain is Ricardo Milos, red-headed in `t17m_rt_22.png`). The game draws a tagged mesh
as an untagged one, so the IR keeps dropping tags (`ir.md`, DECISIONS 2026-10-10); the
QUESTIONS entry removed. (c) closed.
manual: checked 2026-10-10, a `.model` collar converted for PES 21 (4.y-fix12's open
question): PES 17's stock `collar_001.model` and `collar_050.model` (the probe copies in
`.tmp/phase_04/4_y/uniform_mtl/`) each as `Collars/collar_012.model` of an `/out/` export with
Test 3's kit 1 (`.tmp/phase_04/4_y/ingame/build_collar21_export.py [050]`, the release build of
`104d8c8`; 200 is `collar_id_invalid`), the CPK's `collar_012.fmdl` in the test slot: row
19's standard body draws the same plain neckline for both (`c21_14_row19_neck.png`,
`c21_24_row19_neck.png`), the one a collar ID with no model draws (`p21_22_gk_front.png`),
where the stock `collar_107.fmdl` copy drew its folded collar (`p21_19_collar_front.png`).
The converted FMDL (the converter's `fox3ddf_blin`, every sampler dropped) is not drawn:
step 4.y-fix18 gives it the stock collars' Fox material set. The PES 21 Edit path drifts
when the title needs one `a` instead of two (Kick Off's submenu opens: `b` backs out).
manual: checked 2026-10-10, the same `collar_050.model` under ID 12 with 4.y-fix18's
release build (the stock collars' Fox material set): row 19's standard body draws a
folded collar in the kit's colour at the neck (`f18_05_row19_neck.png`, against
`c21_24_row19_neck.png`'s plain neckline), so the material set was the whole cause;
the missing vertex colour stream and the four unweighted bones the converted FMDL lacks
(4.y-fix18's comparison) change nothing visible. Compiled with the CLI from the test
folder (`team-compiler compile --no-deploy --export`; the version is the settings').
(d) answered by the maintainer 2026-10-10, no run needed: a Fox referee export's `env.dds`
is a pre-Fox leftover, and no Fox material samples a cube map
(`resources/Materials.wikitext`), so the 0xD form is drawn by nothing and stays Red's
harmless output (`pipeline.md` step 3); the QUESTIONS entry removed. The earlier note:
manual: not run 2026-10-10, the Fox cube map of type 0xD (d): no material of the refs
export (robocopclassic's `apc.mtl`, `face.mtl`) and no bundled `.mtl` names `env.dds`,
and the 4cc Fox CPKs hold no cube map (4.y-fix3's recon), so a Fox material pointed at a
converted `env.ftex` exists in no export: the check needs a hand-made FMDL, which no
reachability rule puts ahead of the queue. The QUESTIONS entry stands as written.
manual: checked 2026-10-10, the pre-Fox referee marker (b), no answer yet: run D (the
checker as `parts/referee/incom_bsm.dds`) and run G (the template tree's
`referee_prop.model` and `.mtl` with it, what the compiler writes) in PES 17's test
slot, an exhibition match of `/4cccc/` against `/wsg/` driven by the pad: no magenta
and yellow square anywhere in the walkout, the kickoff's wide camera or the replay's
top-down and free cameras at 0:00 (`r17_walk_*`, `r17g_kick_*`, `r17g_top_03.png`),
and no referee figure at all in any of them (a player leads the walkout; the only
figures on the pitch wear the two kits). The cup's own `4cc_35_referees.cpk` holds the
same prop pair and its clover `incom_bsm.dds` (5.6 MB), 35 referee face CPKs and the
referee kit textures, and Sider's LiveCPK root (`C:/Data/4cc/0Livecpk`) holds no referee
file, so the install's referee is drawn by nothing the lead can see: whether the
referee is hidden on purpose on this install, and which scene draws the prop, stays
the maintainer's (QUESTIONS, narrowed). The replay's free camera (mode 3 of the A
button) is tethered to the target player and the pad holds one input at a time, so
no close view of an official was reachable; the pad's `hold:left` rewinds there.
manual: checked 2026-10-10, a Fox subfolder texture before `sourceimages` (f): /@/'s
`hair_high.fmdl` (a whole head naming `face_bsm`) on /out/'s slots 01-03 in PES 21's
test slot (`.tmp/phase_04/4_y/ingame_f/build_subtex_export.py`, compiled by the release build of
`565c802`): slot 01, the head and its own texture in the root, draws as /@/'s player;
slot 02, the head and a magenta `face_bsm.png` in `jessie/`, (b1)'s layout as compiled
(`.../02 - Old/sourceimages/jessie/#windx11/`), draws magenta; slot 03, the same in
green with the CPK patched to the chosen layout (`.../03 - New/jessie/sourceimages/
#windx11/`, the FMDL directory moved alike, `patch_new_layout.py`), draws green
(`s21_rows.png`, Edit Player's roster preview). The game reads a texture wherever the
FMDL's directory says, `#windx11/` inserted before the name, so slice (b3)'s layout move
is safe; (f) closed, its QUESTIONS entry removed.
manual: checked 2026-10-10 by the maintainer, the pre-Fox referee marker (b): PES 17 with
the cup's current `4cc_35_referees.cpk` (the template's prop pair and clover, what the
compiler writes, step 4.19f) draws the clover on the ground under each of the three
officials in every scene (a walkout in the black Void stadium, the maintainer's
screenshot). That CPK's clover is a plain 5.6 MB DDS, the form the compiler writes there
(`blue_port.md`), so the plain-DDS half is answered too. Why the lead's runs D and G (a
plain checker through the test slot) showed no marker is not known: the marker does not
scale with a small referee (the maintainer), so a missed official does not explain it.
(b) closed, its QUESTIONS entry removed.
manual: checked 2026-10-12, the first PES 20 `.ted` (g): team /3/ exported from
`F:/Games/PES2020` (the cup's base DLC; `game.py`'s new `20` entry, no Sider; the file
name typed by posting `WM_CHAR` to the window, so no focus taken; frames
`.tmp/phase_04/4_y/ingame/frames/p20/`) is 14760 bytes (`0x39A8`, the derived size) with key
index `0x14`, the only one that decodes the coach block's `MANAGER` and the team id.
The writer's PES 20 header template was PES 21's, whose u32 at `0x08` (the body length)
is 21's: replaced by the file's header, and the file is the PES 20 fixture of
`pes_savefile`'s Texport tests (`pes20_texport.ted`). Its 660-byte block holds five
125-byte runs where the other fixtures' is zero (`operations.md` Texport). (g) closed.
manual: checked 2026-10-12, the lower sock on PES 21 (a's rest): Test 3's kit (design A,
`pre-fox`, converted) compiled for `/3/` (701, `build_socks_export.py socks team=3
runner=21 noplayers`, the release build of `5a28e37`), whose second row is a standard
body in the game's boots: the green diagonal band on the lower calf crossing the white
stripe, each leg's in the PES 17 reference's direction, and the orange hoop at the
ankle, above the boot (`.tmp/phase_04/4_y/ingame/frames/p21sock/s06_front.png`,
`s07_side.png`, `s08_back.png`, against `s17_34_out_legs.png`); with the upper part
checked 2026-10-10, the converted sock matches PES 17 feature for feature. (a) closed.
manual: checked 2026-10-12, QUESTIONS "Non-power-of-two mipped textures and kits over 2048
on Fox": Red (`e12aa01`, a scratch copy under `.tmp/phase_05/5_0e/red_np2/`, `setup.py` and
`retarget.py`) compiled EGG VGL26's `Faces/XXX14` and AKG VGL26's kits as `/3/` (the
save's /vg/ teams are placeholders that draw no custom kit, `s06_vgl5.png`); its FTEX keep
the chains (`screen` 512x461 and `text` 797x64 ten levels, `u0701p1` 4096 thirteen). In
Edit mode row 14 (70114) draws the Steam page and the caption as the sources show
(`.tmp/phase_04/4_y/ingame/frames/p21np2/s09_row14_face.png` against `screen_source.png`), and row
2 wears AKG's kit (`s07_kit4096.png`). Red's rules adopted (DECISIONS, step 5.0f), the
entry removed. The gloves-subfolder entry (PES 17) cannot run yet: a shared gloves folder
compiles to the team's first shared ID, drawn only on a player the save points at it, which
is the deferred pre-Fox patch's write; the entry says so.

### Phase 5 — Savefile integration

Done 2026-10-17. Spec now describes what exists: `docs/spec/core/development_plan.md` "Phase 5",
`pes_savefile/README.md` (and `operations.md`, `model.md`), `aatf_rules.md`, `save_editor.md`
(its CLI and 25-code message catalog), `team_compiler/README.md` ("Acceptance": the SE and TC
scenarios, 442 proven under `just acceptance strict`), `aesthetics_export/settings_toml.md`.
Step detail (5.0-5.0m4: the 4ccEditor fork survey, the Red inventory and its Fox gaps, the
plans/spec split; 5.1a-d the schema widening to PES 18-19; 5.2a-i `aatf`: schema, interpreter,
host, the eight rulesets, their review loops; 5.3a-b the Team compiler's rows, patch and
savefile transaction; 5.4a-c the Save editor's CLI; 5.5 the in-game check; 5.y-gloves; the
5.6a-f converge: the lead's audit, the design-health pass, the `pes_savefile` census, the
cross-family reviews, the citation audit, 5.6e this rewrite) in git history up to commit
`27a94bb`. The converge's closing whole-crate mutation runs (extra kinds included, split with
bonfire, triage in `.tmp/phase_05/5_6/close/mutants_<crate>/triage.md`): `pes_savefile` 1830 caught, 2
missed (both tested, `27a94bb`), 2 timeouts (endless loops, caught); `aatf` 242 caught, 0 missed, 1 timeout (an engine with no operation limit hangs the endless-loop
test, caught);
`save_editor` 128 caught, 3 missed (the placeholder `view` and `settings_view`, and a
`NotADirectory` arm covered on Linux); `studio_core` 76 caught, 5 missed (`pes_process.rs` arms
the half's OS compiles out); `aesthetics_export` 603 caught, 0 missed; `team_compiler` 2474
caught, 1 missed (the placeholder `settings_view`), 3 timeouts (caught). Carried forward: the
gloves half of the in-game check (issue "FoxDen's PES 2021 gloves trampoline"), the pre-Fox
patch and the Red inventory's pre-Fox gaps (deferred until the cup next plays a pre-Fox
version), the placeholder views (issue of 2026-10-16), the phase's other open issues under
"Issues", the in-game questions in `docs/QUESTIONS.md`.

The phase's manual checks:

TC-SAV-10 (withdrawn at 8.9l5b for TC-SAV-17, which tests prove), checked by hand 2026-10-15, against a stripped copy of the maintainer's save
and a sandbox PES folder (`.tmp/phase_05/5_5/studio_sl/`): with PES 21 running, sideload compile
exit 0, the patch's `cpks = []`, `savefile_skipped_pes_running`, the copy byte-identical;
with only PES 17 running, `savefile_written` and the copy's `.bak` kept

SE-CLI-02 (withdrawn at 8.9l5 for SE-CLI-03, which tests prove), checked by hand 2026-10-15, with PES 21 running: `import-toml`, `apply-patch`,
`transplant`, `strip-and-seed` and `check-stripped --strip-again` each refused with exit 2
naming PES 2021, no `--out` written; with only PES 17 running each wrote its `--out`
(`.tmp/phase_05/5_5/cli02/`)

5.4c2 (`strip_base_copied_face`):
manual: checked 2026-10-15, PES 21 Edit mode (`.tmp/ingame/face_check/`): a stripped
base-copied player shows Placeholder; the copied face shows with the face flag clear too

5.5 (the in-game check; runs, shots and saves in `.tmp/phase_05/5_5/`, by a305229's release exe; the
maintainer's save swapped with their approval and restored, SHA-256 checked):
manual: checked 2026-10-15, PES 21 Edit mode, the maintainer's cup save strip-and-seeded
(`4cc_08_bins.cpk` seeded into the test slot; /d/ 0 of 23 players stripped before, 23 after):
/d/ rows 1-3 and VGL 22 (team 792) rows 5-6 look the same as on the unstripped save
(`pair_d_0N.png`, `pair_vgl_0N.png`), a sample, not every player
manual: checked 2026-10-15, PES 21 Edit mode, stripped save, the /egg/ tracer export compiled
into the test slot: 792's slot 05 shows the compiled full-body model with its boots
(`s3_05_body.png`); the CPK holds `4cc_studio/shared_ids/792.toml` and loads as any other
manual: checked 2026-10-15, same session: slot 06, an `ingame_face` folder whose
`settings.toml` sets only a motion, shows the default face (short dark-brown hair, Face and
Hairstyle editable; `pair4_vgl_06.png`). Without the marker the folder gets the blank face
and no head, as designed (DECISIONS 2026-10-04; `s3_06_body.png`)

5.y-gloves (every `.mtl` of a pre-Fox shared gloves output packed at its root):
manual: checked 2026-10-15, PES 17 Edit mode, /out/ slots 01-03 on shared gloves 1684-1686
(`.tmp/ingame/gloves_check/`, `r3_slot0N.png`, `r4_slot03.png`): `glove.xml`'s
`./grip/glove_r.model` loads; a `.mtl` texture path `./x.dds` and `./sub/x.dds` loads,
`../x.dds` and the game-rooted `model/character/glove/g1684/x.dds` do not (white glove);
a `grip/` model whose `.mtl` sits at the root naming `./x.dds` loads (`r5_slot01.png`)
manual: checked 2026-10-15, PES 17 Edit mode, end to end: run 2's export compiled by
this step's exe (`.tmp/ingame/gloves_check/studio6/`), unpatched: slots 01 and 02 draw their
right gloves magenta (`r6_slot01.png`, `r6_slot02.png`)

### Phase 6 — Export upgrader

Done 2026-10-20. Spec now describes what exists: `docs/spec/export_upgrader.md` (steps 1 to 12,
model-folder mode, the CLI, the message catalog, the Phase 6 acceptance),
`docs/spec/core/development_plan.md` "Phase 6", `team_compiler/testing.md` (the tracers' upgraded cases and the
library parity), `aesthetics_export/object_model.md` (Portraits named `NN`); the plan keeps
step 13 (Phase 7) and the view (Phase 8). Step detail (6.0a-c the open items, the acceptance
section and the Red inventory on `main`; 6.1-6.8 the build, step by step of the plan; 6.9-6.11
the tracers' goldens and parity cases, Red's references recompiled with released Red; 6.12 the
library parity, its census, Red runs and attribution, kept as `scripts/library_parity/`;
6.13-6.16 the defects and the maintainer's rulings it raised: an upgrade writes a clean export
or nothing, leftovers dropped with a Warning, one-wearer item folders' exclusive IDs, Midcup
kits keeping what they do not resend, a pre-Fox Common segment of any name; the converge: the
lead's audit, the design-health passes, Astra and SWE-2 rounds, the citation audits) in git
history up to commit `cf78a8e1`. The close (2026-10-20): the census of every old export on the
maintainer's machine (639 inputs, 279 clean, every unclean run writing nothing, none lost
against 6.16's run), the library parity over the 279 against released Red (every line
attributed but two `red_parity` pairing ties, `.tmp/phase_06/6_close2/`), the design sweep over the code
since the first close run (`upgrade.rs` past a thousand lines and the staging claim written
twice: `staging.rs`), the citation audit (one flag, EU-REF-12), and the closing whole-crate
mutation runs (extra kinds included, split with bonfire, `.tmp/mutants_6_close2_<crate>/`):
`export_upgrader` 981 caught, 6 missed (the two placeholder views; three equivalents whose
`exclude_re` pins had drifted, re-pinned; a game path's letter case, a test added in
`players.rs`), `team_compiler` 2532 caught, 3 missed (the placeholder settings view;
`match_stem`'s empty-inner guard, a test added; the shared folders' packing sort, an equivalent
pinned in `.cargo/extra_mutants.toml`, since the CPK sorts its entries and the tables their
rows). Carried forward: the placeholder views (8.3), the face transformation for a face made for
the other engine (issue "a face diff is engine-specific", after the DBC editors), the gloves
half of the in-game checks (issue "FoxDen's PES 2021 gloves trampoline"), the `save_editor`
tests a running PES game fails (issue), the in-game questions in `docs/QUESTIONS.md`.

The phase's manual checks:

6.16c (a one-sided hand split, HDG VGL26 slot 22 `EAT`):
    manual: checked 2026-10-19, PES 21 Edit mode (`.tmp/phase_06/6_16/ingame/`, the export upgraded
    `--loose` and compiled by dc30c45; /hdg/ is the maintainer's save's VGL 33, team 803; save
    backed up and restored): the compile's `GloveList.bin` rows never reach an outfield player
    on the cup's exe (issue "FoxDen's PES 2021 gloves trampoline"; a stripped save, then the
    stock exe, showed stock hands, slot 06's two-sided crossbow gloves included). With the ids
    in the save records (80306 gloves 4186, 80322 gloves 4202, the route the cup uses today):
    slot 06 wears its crossbow gloves (`w01.png`); EAT shows no stock hands, and the split's
    patch floats as a speck beside the model (`w02.png`, `w02_speck.png`)

### Phase 8 — GUI

The phase's steps are itemized when it opens. Before that, its plans get the fixes a survey
found (2026-10-15; the prototype rule and the launcher were fixed that day, DECISIONS):

- [x] 8.0 plan fixes before itemizing (side-lead C, branch `side/8.0`). Done 2026-10-19: the
  sketches matched to the spec, Phase 8's acceptance sections written (`ST`, TC-GUI, SE-GUI,
  EU-GUI), `eframe` and `egui` pinned `=0.36.2` (DECISIONS 2026-10-19)
- [ ] 8.1 egui's bundled fonts, once the theme settles its icon font (the shell's own, 8.14;
  egui 0.37 also drops `default_fonts` from egui's default features, 8.11): the
  four default fonts are 1.37 MiB of the release exe's `.rdata` (2026-10-16,
  `.tmp/phase_16/16w/rdata_parts.py`), the two emoji fonts (`NotoEmoji-Regular` 409 KiB,
  `emoji-icon-font` 317 KiB) about 0.7 MiB of it. If the GUI shows no emoji, egui's
  `default_fonts` feature goes off and `Ubuntu-Light` and `Hack` are embedded through
  `FontDefinitions` → verify: every view drawn with the emoji fonts removed shows no missing
  glyph (egui's own widgets draw a few symbols from them), and the exe shrinks by about 0.7 MiB
  (`spec/core/distribution.md` "Size budget"). That font setup is the shell's first context
  setup: `surrender_focus_on = Presses`, set today by the Save editor's view every frame
  (`save_editor` `view::set_surrender_on_press`, 8.9e rework 5: a press elsewhere ends a
  field's keyboard gesture, which the one-entry-per-gesture undo needs), moves there, set once
  for every view, the Save editor's test harness setting it the same way
- [ ] 8.3 carried into the itemizing (2026-10-19): the placeholder `view` and `settings_view`
  of `export_upgrader` and `save_editor` (`lib.rs`, one label each) are the mutants their
  whole-crate runs left missed, deferred because no headless test exists for a label; each
  tool's view brings its own tests (EU-GUI, SE-GUI) and the placeholders go, so the next
  whole-crate run of each crate measures them
- [x] 8.4 the usability pass before the tools' drafts. Done 2026-10-20: `PROPOSALS.md` P1-P22
- [x] 8.5 the Save editor's draft, `docs/prototypes/save_editor.html`, in five slices (8.5a-e).
  Done 2026-10-20 (SWE-2): every SE-GUI scenario but the manual ones walked in a headless-Edge
  harness on real save data
- Phase 8 opens (itemized 2026-10-20). The shell is Phase 3's slice (a label list and the
  active view, `studio_core/src/shell/mod.rs`); no `ST` or `SE-GUI` scenario is proven, and
  neither `libs/team_widgets` nor `color_tools`' widgets exist. The steps run in DBC's order
  (`plans/core/development_plan.md` "Divegrass Before Christmas"): what the 0.0.1 preview needs
  first, the Team compiler's and the Export upgrader's views after it. Each step builds against
  its draft (the host page `docs/prototypes/index.html` and its `tools/*.js`) and its plan, and cites its scenarios in tests
  through the shell's headless harness (`shell/mod.rs`'s tests). The drafts wait for the
  maintainer's remarks, so the steps that draw least of what a remark would change come first
  - [ ] 8.6 the shell (`plans/core/gui.md`, `spec/core/gui.md`; draft `index.html`), sliced
    - [x] 8.6a the sidebar: the planned-tool registry, the About block, the PES selector, the
      icon rail, `last_tool`. Done 2026-10-20 (SWE-2)
    - [x] 8.6b the status bar, the window title and the tools' wake callback (ST-BAR-01 to 03,
      ST-TTL-01 to 03, ST-SET-03, ST-CLI-01, ST-SIDE-05). Done 2026-10-20 (SWE-2):
      `Settings::save` edits the file in place (`toml_edit`)
    - [x] 8.6c the settings menu (8.6c1), the first-run dialog and the close hook (8.6c2).
      Done 2026-10-20 (SWE-2)
    - [ ] 8.6d the help window (and the About popup's help link, completing ST-SIDE-06), its
      generated Messages topics and search, `4cc-studio help`
      (ST-HELP-01 to 04), and the log panel widget (ST-LOG-01 to 04); a notice's `OpenHelp`
      action opens the window at its target (8.6b leaves it a dismiss)
      - [x] 8.6d1 the catalogs' member texts, the core chapters, `help` and `--export`
        (ST-HELP-04). Done 2026-10-20 (SWE-2)
      - [x] 8.6d2 the help window (`egui_commonmark` `=0.25.0`; ST-HELP-01, 02, ST-SIDE-06).
        Done 2026-10-21 (SWE-2)
      - [x] 8.6d3 the log panel widget (ST-LOG-01 to 04) and its code links (ST-HELP-03).
        Done (SWE-2): `widgets/dock.rs` `BottomDock`, `widgets/log_panel.rs`; the grid
        cross-link waits for the grid (8.10)
      - [ ] when the first topic gains an in-doc link `[text](<id>#<slug>)`, `--export`
        rewrites it to `<id>.md#<slug>` and writes an anchor per topic slug (review A3-7: no
        shipped topic holds one yet, so the exported files have nothing to break)
      - [x] 8.6d4 a member wording pass over the three catalogs' texts (8.6d4a the compiler's
        234 rows, 8.6d4b the upgrader's 69 and the Save editor's 23). Done 2026-10-21 (SWE-2):
        `studio`'s `every_catalog_row_is_written_for_a_member` guards them
    - [x] 8.6e the window opens at 1440 × 900 points, centered, fitted to a smaller monitor;
      the help window at 90% of it (ST-WIN-01, ST-HELP-05). Done 2026-10-23 (SWE-2)
  - [ ] 8.7 `color_tools`' colour-picker popup (`plans/libs/color_tools.md` Phase 8), the
    Save editor's team colours; the kit menu icons wait for the Team compiler's grid
    - [x] 8.7a the picker popup drawn in the Save editor's draft. Done 2026-10-21 (SWE-2)
    - [x] 8.7b `color_tools`' picker widget against the draft, tested headless. Done
      2026-10-21 (SWE-2, seven reworks over the S1-S3 and A1-A3 rounds): `picker::color_field`
    - [x] 8.7c a click on a suggestion or recent swatch commits and closes the popup (P23).
      Code done 2026-10-26 (SWE-2)
  - [ ] 8.8 `libs/team_widgets` (`plans/save_editor.md` "`libs/team_widgets`"): the card
    pickers and the violations list first, the tactics editor and its pitch with the view's
    slice 3
    - [x] 8.8a the Save editor's draft follows DBC7 as 20.0e left it. Done 2026-10-21 (SWE-2)
    - [x] 8.8b the crate and its `cards` module, and `paid` in `aatf`'s `card_limits`. Done
      2026-10-21 (SWE-2)
    - [x] 8.8c `team_widgets::violations`: the AATF rows grouped per player. Done 2026-10-21
      (SWE-2)
    - [x] 8.8 review: Astra over 8.8b-c and the SWE-2 loops, ended. Done 2026-10-21
  - [ ] 8.9 the Save editor's view in the five slices of `plans/save_editor.md` "Build order
    and verification", with the `session/` wiring each needs (undo, selection, recent files,
    the FPC toggle, AATF quick actions) and the placeholders of 8.3 gone; each slice's SE-GUI
    scenarios cited
    - [x] 8.9a slice 1's skeleton: Open, the team selector and list, the info strip, undo
      (SE-GUI-01, 02, 04). Done 2026-10-21 (SWE-2); reviewed (S1, A1)
    - [x] 8.9b1 the Abilities & Skills tab, with the 8.9a review's fixes. Done 2026-10-21
      (SWE-2); review closed 2026-10-22
    - [x] 8.9b2 the Appearance tab, slice 1's last part (SE-GUI-07, 20, 22's Appearance part).
      Done 2026-10-22 (SWE-2)
    - [x] the portable name limits (`plans/pes_savefile/operations.md` "Team TOML", Portable
      text limits; DECISIONS 2026-10-21): `portable_text_max` and `crop_player_text`
    - [x] the roster-order rule (`plans/aatf_rules.md` "The host"; DECISIONS 2026-10-21) and
      drag reorder (SE-GUI-08), in two slices (8.9c1, 8.9c2). Done 2026-10-22 (SWE-2)
    - [x] 8.9d slice 2 (Team tab, batch operations, FPC; 8.9d1-d4). Done 2026-10-22 (SWE-2):
      parity with 4ccEditor ruled (DECISIONS 2026-10-22 "Slice 2's parity")
    - [x] the FPC hide IDs (55, 11) as a suite-common setting the editor and the Team compiler
      both read (SE-GUI-31, TC-BIN-43). Done 2026-10-22 (SWE-2): `fpc::HideIds`
    - [x] 8.9e slice 3, the Tactics tab (8.9e1 settings, 8.9e2 lists, 8.9e3 pitch; SE-GUI-12,
      25). Done 2026-10-23 (SWE-2), reviewed
    - [ ] 8.9f the draft's second arrangement (the maintainer's request of 2026-10-23: several
      usability issues, mostly badly arranged elements; their own remarks to follow). A new
      page, `docs/prototypes/save_editor_v2.html`, beside the first until the maintainer picks,
      arranged on the lead's judgment, weighing a collaborator's remarks on the first draft
      (not yet ruled by the maintainer): the team's name and abbreviation always in view, not
      on a tab (invitational members edit them often); fonts too thin (seen on the draft only:
      check the built view too); the card count derivable from stats, registered position and
      height rather than listed; star rating and playing attitude toward Appearance (they act
      only in cutscenes); the stats and the playable positions aligned, the positions in the
      grid CF SS LWF RWF / AMF LMF RMF CMF DMF / CB LB RB GK. Each remark adopted or not, with
      why, written in the page beside its draft controls. Slice 4 waits for the maintainer's
      ruling; what the ruling changes in slices 1-3's built tabs becomes a step of its own →
      verify: the harness walks the v2 page as it walks the first, every SE-GUI scenario
      the first draft shows drawn in v2 too
      - [x] slice 1, the frame: body text at weight 400, the team bar above every tab
      - [x] slice 2, Abilities: Profile, the positions grid, one cards badge
      - [x] slice 3: star and playing attitude to Appearance, Tactics in three columns; the v2
        harness 53 of 53
      - [ ] slice 4: v2 is the base (the maintainer, 2026-10-24; DECISIONS "The second draft
        arrangement is the base"): the built tabs of slices 1-3 ported to `save_editor_v2.html`'s
        arrangement, after 8.9j4 (the comparator, panels the same in both drafts) and before
        8.9j5; the maintainer's fine tweaks follow on v2
        - [x] 4a the frame: the team bar, the tab strip in groups, the player header
          (SE-GUI-01, 10, 32). Done 2026-10-24 (SWE-2), reviewed
        - [x] 4b Abilities: Profile, the positions grid, stats in four columns (SE-GUI-06, 07).
          Done 2026-10-24 (SWE-2), reviewed
        - [x] 4c Appearance's groups in aligned cells, Tactics in three columns. Done
          2026-10-25 (SWE-2), reviewed (`.tmp/phase_08/8_9f/duck_rulings_4c.md`); reworks r3
          and r4 landed 2026-10-25 (r4: the view harness draws inside a `CentralPanel`, the
          default-pane tests at `DEFAULT_PANE` 1224 x 776; post-Astra round S2 over r2-r4 ended
          the 4c loop)
          - [ ] rework r2 with 8.15 (2026-10-25, SWE-2; the lead finished it): the window at
            1280 x 800 (ST-WIN-01, ST-HELP-05), the lists at 200, the instruction combos
            capped, the survivors' tests (DECISIONS "The Tactics settings column scrolls with
            the tab at 1280 × 800"). Open: the post-Astra SWE-2 round over r2-r4
        - [x] 4d the one cards badge, the player's own tier, through `Rules::player_tier`. Done
          2026-10-25 (SWE-2), reviewed
        - [x] 4d's post-Astra SWE-2 round S2: a failed tier check shows its error in the
          badge's place (DECISIONS)
        - [x] 4e the regular-weight body font, `resources/fonts/Ubuntu-Regular.ttf` (DECISIONS
          "The body font is Ubuntu Regular, bundled; every view test draws with the app's
          fonts"). Done 2026-10-25 (SWE-2), reviewed
    - [ ] 8.9j slice 4 (`plans/save_editor.md` "Configurable AATF rules", "Comparator",
      "Aesthetics transplant"), the AATF parts first (the maintainer's direction of 2026-10-24),
      each part its own commit and review:
      - [x] 8.9j1 the rules file's settings row and the quick actions row (SE-GUI-09, 32).
        Done 2026-10-24 (SWE-2), reviewed; Browse checked by hand in the built window
      - [x] 8.9j2 the AATF panel (SE-GUI-15). Done 2026-10-24 (SWE-2), reviewed
      - [x] 8.9j3 Live AATF (SE-GUI-30). Done 2026-10-24 (SWE-2), reviewed
      - [x] 8.9j4 the comparator (SE-GUI-13; 8.9j4a the tabs' terms, 8.9j4b the panel). Done
        2026-10-24 (SWE-2), reviewed
      - [ ] 8.9j5 the transplant (SE-GUI-14), redirected 2026-10-25 (the maintainer; DECISIONS
        "The aesthetics transplant writes the team's export, after 0.1.0; the save-to-save
        transplant goes"): the export-writing panel is built after 0.1.0 (its plan section);
        8.9j5c removed the save-writing transplant (8.9j5a's panel, the CLI command,
        `pes_savefile`'s operation, their codes, help, spec), keeping the comparator's r2 fixes
        (the one-line pick row, the focus given up); 8.9j5b is dropped:
        - [x] 8.9j5a the save-to-save transplant panel. Done 2026-10-25 (SWE-2); removed by
          8.9j5c
        - [-] 8.9j5b the two-pane browser: dropped with the save-to-save transplant
        - [x] 8.9j5c remove the save-writing transplant; SE-TRN-01 to 03 withdrawn. Done
          2026-10-25 (SWE-2)
    - [x] 8.9h the editor's tabs fit a narrow window: ruled a 960 × 600 window minimum
      (DECISIONS "The window has a 960 × 600 minimum"). Done 2026-10-25 (SWE-2)
    - [x] 8.9i a starter whose portrait the pitch cannot show draws the game's placeholder
      portrait (`resources/portraits/`). Done 2026-10-23 (SWE-2)
    - [x] 8.9g the Tactics pitch as PES 2021's own: the reference and the draft, slice 1 the
      look, slice 2 the portraits from `download/` CPKs. Done 2026-10-23, seen in the built
      window on PES 2021
    - [x] before slice 4, the draft draws Live AATF (SE-GUI-30). Done 2026-10-22 (lead)
    - [x] before slice 4, one team's AATF check measured: `rhai` built with `sync`,
      `check-aatf` in parallel (DECISIONS 2026-10-22). Done 2026-10-22 (lead)
    - [x] 8.9m the script's own work: `logic.rhai`'s `read_player` rewritten to copy less,
      logic 1's hash `ea75b5fb…` (DECISIONS 2026-10-25). Done 2026-10-25 (SWE-2)
    - [ ] 8.9k slice 5, the interchange dialogs (`plans/save_editor.md` "Build order and
      verification" item 5): Team TOML export and import with its sections (SE-GUI-16), the
      import dialog for `.4ccs`, `.4cct` and Texport (SE-GUI-17, with SE-GUI-22's and 35's
      aesthetics section under both settings), Apply aesthetics patch with its preview
      (SE-GUI-18), the Texport export checked in game per version (SE-GUI-29, manual). The
      libs hold every operation (`pes_savefile::interchange`); the draft has every panel
      (`docs/prototypes/tools/save_editor.js`). Slices, each one review (draft check against
      the app's theme first, brief, sidekick, review, merge, release-build check):
      - [x] k5a 2026-10-29 Apply aesthetics patch, the Savefile menu's floating panel
        (SE-GUI-18): `view/patch.rs`; release build checked on the real PES 21 save
      - [ ] k5b the interchange panel's import half (SE-GUI-16's import, SE-GUI-17's import,
        SE-GUI-22's and SE-GUI-35's import-dialog parts): the file picked decides the format,
        the sections the file holds offered, the legacy formats read-only, the version
        mismatch's warning list, the aesthetics section under the ownership setting. The
        lib applies a Team TOML whole (no section choice), so two sub-slices:
        - [x] k5b1 2026-10-29 the section filter: `session/import_sections.rs`
        - [ ] k5b2 the panel's import half on it (`legacy::read_squad`/`read_tactics`,
          `Texport::from_bytes` + `to_team_toml`, the notes list, the `.4ccs` pairing), the
          file pick row as the Comparator's, the floating panels' open flags folded into one.
          Built and merged 2026-10-29 (`ffd74ec9`): `view/interchange.rs`; release build
          checked on the real PES 21 save. The notes' scroll area (rulings L1) landed with
          k5b2r
      - [x] k5c its export half (SE-GUI-16's export, SE-GUI-17's export). Merged 2026-10-29
        (`ae9fd2a2`); release build checked 2026-10-30
      - [x] k5b2r the rework of SWE-2 round 1 on k5b2 and round 2 on k5a, and of k5c's round 1.
        Merged 2026-10-29 (`e194f974`, tests `17cfd6d8`)
      - [ ] k5d after 0.0.1 (the maintainer, 2026-10-30: 0.0.1 targets PES 2017 only):
        SE-GUI-29 manual, each Texport the panel exports imported by the game's own team import
        in PES 18, 19, 20 and 21 (the installs on G:; saves backed up first). Look: Pending looks
      - [x] k5e PES 2015-2017 Texports in the panel (no extension; on PES 2015-2017 the dialog
        opens on All files). Landed `81daabdb` (Opus sidekick); checked in PES 2017 2026-10-30
      - [x] k5f Texport export on PES 2017, from the open save (DECISIONS "PES 2017 Texports
        written from templates and the open save"). Landed `740388f4` (Opus sidekick);
        SE-GUI-29's PES 17 case checked in game 2026-10-30; the zero emblem's effect unseen
        (look: Pending looks)
      - [ ] k5g after 0.0.1 (the maintainer, 2026-10-30: "the behavior applies to all
        versions, but we'll have to confirm it"): on PES 2015, 2016 and 2018-2021, import a
        team file in game and check whether the import writes the team name the DLC's team
        bin gives the target's id into the save, as on PES 2017 (`plans/save_editor.md` "Interchange
        formats in the UI", the Texport bullet); the bin's editing is future work, no plan yet.
        Look: Pending looks
      - [x] k5rev the cross-family review loop over k5a-k5c, ended 2026-10-30 (rulings
        `.tmp/phase_08/8_9k5/rulings_k5.md`; fixes merged `83fea5f2`)
      - [x] k5mut the loop's `mutants-diff` over k5a-k5c, ran 2026-10-30: 9 missed, all test
        gaps, tested in cv3 and the brief after it
    - [x] 8.9m the maintainer's pass over the 0.0.1 candidate (`30b14680`), 2026-10-29 (m1-m5:
      the medals off the UI thread, rounded corners, the light theme's greys, the running-game
      texts; m2 `draft_compare.py` compares fills and text colours). Done 2026-10-30; m4's
      pitch warning cross unseen (look: Pending looks)
    - [x] 8.9l 2026-10-29 the open/save scenarios no slice built: the unsaved-changes prompt
      (SE-GUI-05), the Open menu's discovered saves and Save as (SE-GUI-26), the version
      warning as a standing notice (SE-GUI-28), SE-GUI-21 in the built window, and
      `--edit-mode-closed` for the CLI and the compiler (SE-CLI-03, TC-SAV-17); with it the
      drafts take egui's spacing (l3d, DECISIONS 2026-10-29 "The drafts take egui's spacing")
      SE-GUI-21 manual: checked 2026-10-29 (lead, release build of `7be7ecb5`, PES 2021 from
        `E:/PES2021` driven by the pad, the maintainer's live save backed up first and restored
        by hash after, `.tmp/release_0_0_1/se21b/`): with the game running, Save on a renamed player (/3/'s
        QUADS ONLY to QUADS ONLYX) opened the dialog naming PES 2021 and asking to close Edit
        mode; Cancel left the live EDIT00000000's hash unchanged; with Edit mode closed in game,
        the dialog's Save wrote it (hash changed); back in Edit mode, Load ("Rigging Edit Data
        will erase current data") then Edit Player showed QUADS ONLYX, no restart
        (`.tmp/release_0_0_1/se21b/shots/12_dialog.png`, `11_after_load_squad.png`)
  - [ ] 8.10 after 0.0.1: the progress grid (with the log panel's cross-link: a warning or
    error line's click highlights its grid cell, `plans/core/gui.md` "Log panel", which 8.6d3 leaves for
    the grid), the run strip, the exports folder watcher and check cache, cancellation, the launcher (`quick_compile.bat` to its `--gui` form, `quick_upgrade.bat` dropped once the upgrader's view lands), the Team
    compiler's and the Export upgrader's views (their drafts first); the notice action
    `OpenFolder` opens the system file browser for the compile's "Open output" (8.6b leaves it
    a dismiss; eframe's `open_url` opens a browser even for a local path, so not through it),
    and the Team compiler's view stops requesting repaints itself (`ToolContext::emit` wakes
    the GUI since 8.6b); the first tool that publishes a notice during a CLI command gets a
    binary test of ST-CLI-01's notice half (8.6b proves it through the printer's loop only);
    the grid's row actions (`plans/team_compiler/gui.md` "Row actions": Open folder, one-click
    fixes, Upgrade on an old export, the empty grid's drop, Compile only this export and the
    hidden compile column, disabled exports greyed with Enable and Disable on a clash, the save
    findings opening the Save editor; accepted 2026-10-25 as P3-P5, P7-P9, P15's Part A)
  - [ ] 8.23 after 0.0.1: edits made in PES come back to the export
    (`plans/aesthetics_export/settings_toml.md` "Edits made in PES come back to the export";
    DECISIONS of 2026-10-25; the maintainer's "even if Studio was not running" first):
    - [ ] 8.23a the in-game check first (PES 21, a save with a compiled team's patch applied,
      a backup before; after 8.36c): a motion and a face edit in Edit mode, which keys the
      record then holds → verify: `manual:` lines with captures; the plan's key list corrected
      to what it found. Look: Pending looks
    - [ ] 8.23b the compile record beside the patch (its format, written by every compile with
      a save step, run or skipped) → verify: a compile test reading it back
    - [ ] 8.23c the comparison and the notice (at start, on the views opening, before a
      compile, on the save's change), Bring into the exports, the conflict choice, the
      compile's Warning over an unbrought edit → verify: tests over a fixture save edited as
      PES would (a motion on one player, a face on another), with Studio "not running"
      between the compile and the edit (a fresh shell over the record)
    - [ ] 8.23d Apply to the save once PES is closed (a pending patch) → verify:
      the running-PES probe faked; nothing written while it reports PES running
  - [ ] 8.24 after 8.23, research: previewers outside PES for the in-game face and the motions
    (the maintainer wants it looked into, 2026-10-25), in Blender or in Studio's planned
    `libs/model_viewport` (`plans/core/README.md`'s "Player aesthetics viewing" row): first
    what the game's files and Marco's RE notes give (the base head and the sliders' shape
    rules; the motion archives and what selects a clip), then a plan section, not code
  - [x] 8.25 the sidebar as a rail that opens on hover over the view (DECISIONS "The sidebar
    is a rail that opens over the view on hover"; ST-SIDE-02): the drafts' half merged
    `7b5ed0ef`, the code half 2026-10-25 (SWE-2); seen in the built window at 1280 × 800
  - [ ] 8.22 after 0.0.1: the accepted usability items outside the grid (2026-10-25, P1, P2,
    P6, P10-P14; DECISIONS "Usability proposals P1-P14 accepted"; 2026-10-26, P18-P20, P22,
    P32-P36, h to j), each its own commit:
    - [ ] 8.22a the finding line's disposition phrase (`plans/core/architecture.md` "Every
      finding line says what was lost") → verify: a CLI test per phrase, the pinned lines
      updated
    - [ ] 8.22b the `closest` context field on five codes (`plans/team_compiler/messages.md`
      "A failed name lookup names the nearest existing name") → verify: a test per code with a
      real misspelling (`fpc-on`, a shared folder one letter off), and one with nothing near
    - [ ] 8.22c the deep check runs every check whose inputs exist before a compile
      (`plans/team_compiler/gui.md` "Deep check": a gap, not a proposal): first an audit of
      every code the catalog marks "reported by `compile`", each kept with its reason or
      moved; then `fmdl_texture_not_found`'s lookup shared by both callers → verify: the
      audit's list; a deep check reporting a missing texture with no compile, a test pair
      keeping the two callers in agreement
    - [ ] 8.22d `model_name_type_misplaced` and `kits_folder_missing`
      (`plans/team_compiler/messages.md`) → verify: a test each, and the full-body team not
      warned
    - [ ] 8.22e an invalid `settings.toml` keeps the installed look
      (`plans/aesthetics_export/settings_toml.md`) → verify: a Fox compile over an installed
      row keeps it; the defaults with none installed; the pre-Fox patch holds no entry
    - [ ] 8.22f the savefile in the preflight (`plans/team_compiler/pipeline.md` "6.
      Post-processing") → verify: another version's save reported before any export is read
    - [ ] 8.22g the first run finds the PES version and folder (`plans/core/distribution.md`
      "Data location") → verify: a dialog test over a fake Documents and install tree (no real
      install reached); nothing found keeps the defaults
    - [ ] 8.22h the context the findings' texts need (P22, accepted 2026-10-26;
      `plans/team_compiler/messages.md` "A finding carries the context its text needs"),
      before the texts that read it → verify: a test per code reading the new field
    - [x] 8.22i 2026-11-12 (`88c91de1`): `upgrade <export>...` (P20), built as 0.0.2-upgall
    - [ ] 8.22j the upgrader's view items (P18's pair, P19's dialog, P32-P36;
      `plans/export_upgrader.md` "GUI"), with the view (8.10), drafted first (8.17b) → verify:
      EU-GUI scenarios for each, written with the view's Acceptance before it is built
  - [x] 8.11 egui 0.36.2 to 0.37 (`spec/core/gui.md` "egui version discipline"). Done
    2026-10-23 (SWE-2, with 8.13): `egui-phosphor` dropped for
    `resources/fonts/phosphor-subset.ttf` (since replaced by 8.14's `icons.ttf`)
  - [ ] 8.12 after DBC: the Texport coverage audit, the measurement that decides whether a
    Team TOML can stand for a tactical export (`plans/aesthetics_export/player_folders.md`
    "Complete export"; agreed 2026-10-21). Every byte of the real texports of each version
    (`spec/pes_savefile/operations.md` "Texport") classified: carried by Team TOML, worth
    adding to the half that owns it (the candidates, set in game: the team record's emblem, stadium settings, rivals,
    callname, banners, goal-net colours, manager name, PES 18's kit slots; the tactics'
    lineup slots 32 to 39 on PES 19-21, Switch Preset Tactics, Auto Lineup Select; a
    player's commentary name and PES 20/21's print name), or not a tactic (the 660-byte block,
    the tail); the meanings checked in game, and for PES 2021 against Marco's reverse
    engineering of the game's executable (`C:/Data/4cc/Tools_Marco/GodotFootball/docs/re/`;
    the repository is in active evolution, so before the audit starts its new commits are
    pulled, `git pull --ff-only` in that clone, a pull that cannot fast-forward reported to
    the maintainer rather than forced;
    index `README.md`, every claim tagged observed or inferred: `pes-re-matchlogic.md` for the
    player and team records, `pes-kit-assembly.md` §8.1 for the EDIT save's layout); each
    "worth adding" a Team TOML key; then a test
    per version taking a real texport to Team TOML and back, byte for byte apart from the
    bytes classified as not a tactic
  - [x] 8.13 the release stage after the version: `v0.0.1 pre-alpha` from
    `shell::version_label`. Done 2026-10-23 (with 8.11)
  - [x] 8.14 the shell's own icon font: `scripts/fonts/make_icons.py` builds
    `resources/fonts/icons.ttf` from the 13 tool SVGs and the Phosphor glyphs (DECISIONS "The
    final icons approved; …"). Done (`581aef24`, merged `a87ae977`; Opus sidekick); the rail
    seen in the built window 2026-10-30
  - [ ] 8.15 (code done 2026-10-25 with 8.9f4c's rework r2; the built window's check open) the window's default to 1280 × 800 (the maintainer, 2026-10-25; `spec/core/gui.md`
    "The window's size", ST-WIN-01, ST-HELP-05; DECISIONS "Studio opens at 1280 × 800
    points"): `studio_core`'s `DEFAULT_SIZE` and its tests, the help window's; the Save
    editor's Tactics lists at 200 points, and its tests named for 1440 × 900 → verify: the
    scenarios' tests; the built window at 1280 × 800 on the FHD monitor, the Tactics tab in
    three columns. Look: Pending looks
  - [x] 8.16 the drafts as one host and a script per tool (DECISIONS "The drafts are one host
    page and a script per tool"): `docs/prototypes/index.html`, `tools/*.js`,
    `scripts/drafts/bundle_drafts.py`. Done 2026-10-25 (side-lead A, SWE-2)
  - [ ] 8.17 the Export upgrader's draft, `tools/export_upgrader.js` (`plans/export_upgrader.md`;
    `core/gui.md` "Prototypes before implementation"; side-lead track A, after 8.16): every
    screen the plan describes, with sample data; the lead's usability review first, ideas to
    `PROPOSALS.md`; then the maintainer's remarks
    Drawn 2026-10-25 (side-lead A, SWE-2): every screen on real CLI runs' samples
    (`.tmp/8_17/runs.py`); DECISIONS "The view's arguments and report, where the plan was
    silent"; the usability review's ideas `PROPOSALS.md` P32-P36. Waits for the maintainer's
    remarks
    - [ ] 8.17b the accepted items drawn (P18's Full/Midcup pair, P19's dialog, P32-P36; ruled
      2026-10-26, `plans/export_upgrader.md` "GUI"), the samples' rows grouped and the
      `needs <team>` hint from the real runs → verify: the bundled drafts build; each item
      reached in the draft
  - [x] 8.18 the host's layout edit toggle (DECISIONS "The drafts' host gets a layout edit
    toggle"). Done 2026-10-25 (side-lead A, merged `139355d9`)
  - [ ] 8.19 the host's remarks, links and longest values (`core/gui.md` "The host's other
    review aids"; DECISIONS "The drafts' host gets remarks, links, …"; after 8.18, with it on
    track A or the top lead's): the URL fragment helper and every tool script's state keys,
    the Remark toggle and "Copy remarks", the extraction script and `assets/longest.js`
    (real values only) → verify: Playwright opens a link to a dialog's state and the capture
    shows it; a remark's copied line names its panel and control and carries a link that
    reopens the state; with Longest values on, each tool's longest text is drawn (a capture per
    tool, looked at)
    - [x] 8.19a links (side-lead A, merged `144fedf1`): `#<tool>/<screen>?keys`
    - [x] 8.19b remarks (Opus, 2026-10-27): the Layout group's "Remark" toggle, "Copy remarks"
    - [ ] 8.19c longest values, briefed: `.tmp/phase_08/8_19/brief_8_19c.md`
  - [ ] 8.20 the host's overlay and draft shots (`core/gui.md` "The host's other review aids";
    after 8.19, whose links it walks; the overlay moved into 0.0.1 as 0.0.1ov): the Overlay control and the capture naming, the
    Save editor's first captures from the built window; `scripts/drafts/draft_shots.py` → verify: the
    overlay of the built Tactics tab on its draft at 1280 × 800, looked at; a one-line host
    stylesheet edit lists every tool's changed screens, reverted by hand, then lists none.
    Briefed: `.tmp/phase_08/8_20/brief_8_20.md` (the overlay needs built-window captures, which take
    the UI automation until `--gui` opens a save)
  - [ ] 8.26 the drafts' text drift side-lead A found (2026-10-25): the drafts show `v0.1.0`
    where the build shows `v0.0.1 pre-alpha`; the host's help chapters still say `4cc-studio`
    where the help files say `UnlimitedToolWorks`; with the preview pane on at 1280, the
    Ruleset editor's form clips its stat grid (the Ruleset editor waits for 0.1.0: fix with
    20.6) → verify: a capture of each, looked at
  - [x] 8.27 the notice in the bar's middle, in its kind's colour (DECISIONS "A notice sits in
    the bar's middle, in its kind's bright colour"; ST-BAR-02 to 04). Done 2026-10-25 (SWE-2)
  - [x] 8.28 the aesthetics lock behind the "Aesthetics from exports" setting, off by default
    (DECISIONS "The aesthetics lock is a setting, …"; SE-GUI-35). Done 2026-10-25 (SWE-2)
  - [x] 8.30 WITHDRAWN (the maintainer, 2026-10-28: "keep the Nation input as it is, with no
    label"); its draft stays on `side/drafts2`, unmerged
  - [x] 8.31 a click on the toolbar's `• unsaved changes` opens the list of the changes in a
    popup (DECISIONS "The changes list is a popup under the unsaved-changes label"; SE-GUI-43).
    Done (`832caa83`, merged `0bf68fc4`); seen in the built window 2026-10-30
  - [x] 8.32 the pitch draws the formation's name in its corner: built as 0.0.1x, approved
    2026-10-28
  - [ ] 8.33 after 0.0.1 (the maintainer, 2026-10-27): a menu entry that saves a picture of
    the team's formations to a folder: a grid, one row per preset, one column per formation.
    Drafted and approved 2026-10-28 (DECISIONS 2026-10-27 "Formation pictures, the
    formation's name and the nation names: the maintainer's answers", "The drafts' looks of
    2026-10-28"; revised 2026-10-29, DECISIONS "Formation pictures draw hidden formations,
    label Fluid formation, and put a target on its own line"); the plan written
    (`plans/save_editor.md` "Tactics editor", "Formation pictures"); left: the code, with
    8.34, its scenarios written with it
  - [ ] 8.34 after 8.33 (the maintainer, 2026-10-27): the same for a range of teams, one
    picture per team, in one go. Drafted and approved with 8.33 (above); left: the code
  - [x] 8.35 a ruleset carries its event's logo and page (P45; DECISIONS "A ruleset carries its
    event's logo and page"; RE-FILE-09, SE-GUI-44, RE-GUI-19): a `aatf`'s keys, b the Save
    editor's logo and the Ruleset editor's fields, c every ruleset file's logo and page. Done
    2026-10-30 (Opus sidekick, merged `d4cdcdf4`, `a61ee964`, `eb699752`)
  - [ ] 8.36 every engine's looks through the savefile patch, no stripped saves (the
    maintainer, 2026-10-30; DECISIONS "Every engine carries a player's looks in the savefile
    patch; no stripped saves")
    - [x] a the plans: stripped saves out, the full-look patch designed
      (`plans/pes_savefile/operations.md` "Aesthetics patch (new)"; DECISIONS "The full-look
      patch writes the player's own appearance id…")
    - [x] b before 0.0.1, the Save editor: `strip-and-seed`, `check-stripped` and every
      stripped mark removed, `diff` gains the appearance id (SE-DIF-05), SE-GUI-20 withdrawn.
      Done 2026-10-30 (Opus sidekick, branch `s36b`)
    - [ ] c after 0.0.1 and before 0.0.2 (VGL 27 plays on PES 2019, a Fox version, with no
      stripped save; `plans/core/development_plan.md` "The /vg/ League 27"), the full-look
      patch and the Team compiler: the aesthetics patch on
      every engine (it is Fox-only today) gains the appearance block, the boots and gloves IDs
      over the whole allocation, filled after their tasks commit, and clears a compiled
      player's base copy, or writes the one his `<id>.copy` link names (the link's validation
      in the compiler, its generation in the Export upgrader's step 8:
      `plans/aesthetics_export/player_folders.md`, `plans/export_upgrader.md` "What it does");
      `apply-patch` writes them; the Fox player-table writer and its
      messages go; the spec with it (the inventory's S22-S54, S56-S57's table parts, S61,
      S64-S66, S68, and GLOSSARY's D1 and D2), its Acceptance written first → verify: the
      gates; TC-SAV-12 (a PES 17 compile writes no patch) rewritten; the library parity's
      difference classes re-diagnosed
      - [x] c0 the Acceptance (2026-11-01): `plans/team_compiler/README.md` "8.36c scope",
        SE-PAT-07 to 11, EU-SET-12, 13; four DECISIONS entries; reviewed
        (`.tmp/0_0_2/s36c/rulings_c0.md`)
      - [x] c1 the patch's look: `pes_savefile` and `apply-patch` (`[appearance]`, the two IDs
        over 14 bits, `base_copy_id`). Done 2026-11-01 (SWE-2, merged `81301b9e`)
      - [x] c2a the compiler's IDs filled once the tasks' outcomes are known, the patch on
        pre-Fox (TC-SAV-19, TC-BIN-50 to 59, TC-MOD-95 to 99). Done 2026-11-05 (SWE-2)
      - [x] c2b the player tables go (about 2,400 lines; TC-BIN-37 and TC-SAV-07 removed).
        Done 2026-11-05 (SWE-2)
      - [ ] c3 the `<id>.copy` link: the compiler's three checks and the patch's
        `base_copy_id`; the upgrader's step 8 → verify: TC-STR-21, TC-SAV-20, EU-SET-12, 13.
        Settle first (review S-36c r1, A1): an entry holding `base_copy_id` alone (an
        invalid-settings player whose folder keeps its link) leaves a record at appearance id
        -1 reading its look from the game's table, since `apply` writes the own id only with
        an `[appearance]`; the spec is silent on whether a link alone makes the record his
      - [ ] c4 the spec moved (the inventory's S22-S54 …), help and catalog texts (among them
        `messages/catalog.rs`'s "motions and names on PES 2018 to 2021, aesthetics on PES 2015
        to 2017", wrong since c1), the drafts' message rows, the library parity re-run and its
        classes re-diagnosed; then 8.36c's review: Astra over its code (checkpoint (b)) and
        one `mutants-diff` over c1-c4
        - the texts done 2026-11-05 (two SWE-2 sidekicks): the built scenarios moved to the
          spec, TC-SAV-20 and TC-STR-21 staying for c3; the 8.36d checks stay in the plan
        - the library parity done 2026-11-05 at `46105675` (tag `c4` against `close6`): one
          new intentional class, I35
        - the review done 2026-11-06 (the duck rulings' "S-36c r1" to "r3", "A-36c part 1"
          and "part 2")
        - `mutants-diff 8bcbf720` over 8.36c and 0.0.2-poc1 to poc11, run 2026-11-08 and
          2026-11-09: 1 missed, equivalent, no missing test
          (`.tmp/0_0_2/poc11/mutants_diff_8bcbf720/triage.md`)
        - the library parity p11 (release build of `ee80b0ac`): every change traced, five new
          intentional classes I36 to I40 (`.tmp/library_parity/library/diagnosis_p11.md`)
    - [ ] d in game, before 0.0.2 (PES 2019 first, VGL 27's game and the likeliest for the
      4cc's spring cup, then PES 2021 and PES 2017; the maintainer, 2026-11-04): a
      patch applied to an
      unstripped save shows the compiled look in Edit mode, boots and gloves included, on a
      player whose base copy named another and on one whose edit flags were clear
      (`scripts/ingame/`, saves backed up); with it, the FoxDen gloves issue rechecked, and
      SE-GUI-34 proved again in its patch form (a `SE-GUI-34 manual: checked` line). Look:
      Pending looks
  - [x] 8.37 a dialog's title centred over the dialog (DECISIONS "A dialog's title is centred,
    as the windows' titles are"), with 8.37b Set all and Bump all stats' number on the
    sentence's row. Built (Opus sidekick, merged `72df1947`); seen in the built window
    2026-11-01
  - [x] 8.38 (closed by 0.0.1's cut: the maintainer, 2026-11-05) the built-window check of
    2026-10-30 (`.tmp/phase_08/8_30c/findings.md`): a the number dialogs' 52-point box (merged
    `bd940680`), b a disabled widget's look, `studio_core::widgets::add_enabled` (merged
    `6cb1a234`). The Fox ownership line's Apply link went unchecked (look: Pending looks)
  - [ ] 8.39 after 0.0.1 (the maintainer, 2026-10-30): rename `dds_convert` to `image_convert`.
    The crate decodes every picture the suite reads (PNG, DDS, FTEX), resizes, writes PNGs and
    encodes the games' textures; the Save editor, the Ruleset editor and `team_widgets` use it
    for logos and portraits that are never a DDS, so the name tells a reader the wrong scope.
    `image_convert`, not a split into a raster crate and a texture crate: the decode is shared
    by both halves (CONTRIBUTING guardrail 3, no over-splitting), and the name pairs with
    `model_convert`. Mechanical: the package and folder, every `dds_convert::` path and manifest
    (76 files), `plans/libs/dds_convert.md` and `spec/libs/dds_convert.md` renamed with their
    pointers, GLOSSARY, AGENTS.md's read-order row, `just crate-map`; DECISIONS and LOG keep
    the old name. Only with no branch open and no mutation run queued (it touches every tool)
    → verify: `git grep dds_convert` lists only DECISIONS and LOG; `just gates`; `just
    doc-pointers`
  - [x] 8.40 a keyboard edit settles without leaving the control: a focus held by a widget
    with no text cursor counts as settled. Done 2026-11-02 as `4f37cab7` (with 8.42), on `f1e`
  - [x] 8.41 the shirt name and the team's short name take upper case only. Done 2026-11-02
    as `091c725b`, on `f1e`
  - [x] 8.42 the undo history merges a run of steps to one value within a second. Done
    2026-11-02 as `4f37cab7`, on `f1e`
  - [x] 8.43 PgUp/PgDn step a page of the rows the list shows in full. Done 2026-11-02 as
    `091c725b`, on `f1e`
  - [x] 8.44 the wheel steps the Tactics preset and formation selectors, wrapping round
    (DECISIONS "The wheel steps the preset and formation selectors, wrapping round"). Done
    2026-11-02 as `cbbb727e`, on `f1e`
  - [x] 8.45 Ctrl+C and Ctrl+V on the player list copy a player's stats (DECISIONS "Ctrl+C and
    Ctrl+V on the player list do what Copy stats does"). Done 2026-11-02 as `a4545968`, on
    `f1e`
  - [x] 8.46 one Up or Down press in an edited number field steps it one (egui 0.37's
    `DragValue` steps by its drag speed, 0.25). Done 2026-11-02 as `091c725b`, on `f1e`
  - [ ] 8.29 the text pass: every line a member reads reread against CONTRIBUTING's "That text
    flows naturally" (the maintainer, 2026-10-25; DECISIONS "Text a member reads flows
    naturally, and every existing line is swept once"), rewritten where it reads dry, its meaning
    kept; the tests that pin a string follow it. One slice per surface, the 0.0.1 tools first;
    each slice's diff read line by line by the lead before it lands (a rewrite that changes what
    a line says is a finding) → verify: the gates; every rewritten line read in its place (the
    draft or a headless frame)
    - [ ] a the Save editor, in three slices:
      - [x] a1 the window's strings, tooltips and notices under `view/`: 34 lines rewritten
      - [x] a2 the message catalog and the CLI's output and `--help`: 76 lines rewritten; the
        spec's Messages table keeps its developers' wording
      - [x] a3 the help chapters and the draft `docs/prototypes/tools/save_editor.js`: 93 lines
    - [x] b the shell (`studio_core`) and the drafts host `docs/prototypes/index.html`: 35 lines
      rewritten (Opus, worktree `wt-829b`)
    - [ ] c the Team compiler and the Export upgrader: their message catalogs, help chapters,
      CLI output and the comments injected into `settings.toml`/`config.toml`/`materials.toml`
      (side-lead U, branch `side/text`), in six slices, the last outside its locks. Left by
      c4-c5 for whoever holds the files: `libs/fpc`'s hide-ID refusals keep a "key = value:
      must be" shape; the Save editor's `view/portraits.rs` keeps "not a CPK the reader
      accepts"; `spec/aesthetics_export/player_folders.md` (about line 720) says a shared
      folder with no model of its kind takes no ID without the `.common` link case; the drafts
      host's mock catalog (`docs/prototypes/index.html`) and `tools/export_upgrader.js` quote
      the upgrader's old texts:
      - [x] c1 the Team compiler's catalog, rows 1-80: 89 texts rewritten (SWE-2 S1-S3)
      - [x] c2 the catalog's rows 81-161: 115 texts in all (SWE-2 S1-S5)
      - [x] c3 the catalog's format-check and structure-pass rows (162-237): 58 texts, the
        upgrader's 13 copies following
      - [x] c4 the Team compiler's CLI output, `--help`, window strings and help chapter
        (c4a-c4d), and c4e the fact audit of both tools' help chapters against the catalog and
        the spec (c4e1-c4e3: 60 facts made true)
      - [x] c5 the Export upgrader: c5a its catalog (45 texts), c5b its `cli.rs` and help
      - [ ] c6 the injected comments: they live in `pes_savefile` (`settings_toml/`, the
        `settings.toml` and `ingame_face.toml` headers and key comments) and `kit_config`
        (`toml_form.rs`, `config.toml`), outside side-lead U's locks: for whoever holds those
        crates (no code writes comments into `materials.toml` today)
    - [ ] d `libs/aatf`'s violation texts and the other tools' catalogs and help
      - [x] d1 before 0.0.1: no GUI or CLI text prints an internal identifier (inventory
        `.tmp/phase_08/8_29d/inventory.md`): d1a the AATF rows in words (DECISIONS "Each
        violation kind's name in words, …"), d1b below. Done 2026-10-30
        - [x] d1b the rest of the inventory: d1b1 the fields no view draws (merged `5aaf3b12`),
          d1b2 below
          - [x] d1b2 2026-10-30 the team file notes, the badge's and quick actions' refusals,
            the card and playing-style labels, the man-marking row (DECISIONS "card names, the
            team file notes, man marking and the tier refusals in words"). Merged `5da2edf0`
      - [ ] d3 the stats named as each version names them (P43, accepted 2026-10-29; DECISIONS
        "Stats are named as the save's PES version names them", "Before 0.0.1, only PES
        2017's stat names are checked"):
        - [x] d3a PES 2017's stat names (`pes_savefile` `labels::stat_name`). Done 2026-10-30
          (Opus sidekick, merged `34639d07`)
        - [ ] d3b after 0.0.1: every other version's names, gathered from each game (the
          player.rs doc comments name no versions; Marco's RE reports for PES 2021, the
          games on G: for the rest) → verify: a test per version against the gathered names.
          Look: Pending looks
      - [ ] d2 after 0.1.0, with Phase 20 (the maintainer, 2026-10-29): the Ruleset editor
        shows the game's names (Attacking Prowess) and writes the rules-file ids
        (`attacking_prowess`)
    - [ ] e the help chapters' shape (the maintainer, 2026-10-29: "each paragraph is a single
      extremely long line, which explains multiple facts one after the other"; DECISIONS "Help
      topics in short paragraphs under subheadings, lists for parallel items", CONTRIBUTING's
      bullet): every topic restructured, its facts kept, each change read by the lead line by
      line (a fact lost or changed is a finding)
      - [x] e1 before 0.0.1: the Save editor's eight chapters and the two core ones. Done
        2026-10-30 (Opus sidekick, merged `db1b149d`); e1r the search's labels as plain text
        (merged `1c1fb403`). The read in the built window was left open (look: Pending looks)
      - [ ] e2 after 0.0.1: the Team compiler's, the Export upgrader's and the other tools'
        chapters
  - [ ] 8.21 the app's font in the drafts (`core/gui.md` "The host's other review aids"; after
    8.9f4e picks the bundled font): `@font-face` rules at egui's sizes → verify: one width
    constant's row (`TOP_ROW_ONE_LINE`) measured in the draft and in the built window agree
    within a few points
- [x] 8.2 the shell's draft, `docs/prototypes/shell.html` (2026-10-19, side-lead E, SWE-2 in
  seven slices): every `core/gui.md` shell screen, walked by a 29-check headless harness

### Release 0.0.1 — the preview build (DBC's editors, VGL 27's compiler and upgrader)

`plans/core/development_plan.md` "Divegrass Before Christmas" and "The /vg/ League 27 (VGL)"
(the maintainer's choice of 2026-10-23: the CLI half joins the build already planned). Steps
a-e run once 8.9e and the second draft (8.9f) are done, while the maintainer rules on that
draft, and before Phase 20, so the rehearsal's findings have the most time before the league;
f is the cut, once Phase 20 closes. The usual rigor holds throughout:
briefs, review loops, gates and mutation runs as for any step.

- [x] 0.0.1a the console's progress line and run summary (`studio` `console.rs`, `indicatif`;
  ST-CLI-02's piped half by a CLI test). Done; the terminal half checked by hand 2026-10-23
- [x] 0.0.1b the launchers: `resources/bundle/quick_{compile,upgrade}.bat`, tested through
  `cmd /c` in `crates/studio/tests/launchers.rs`. Done
- [x] 0.0.1c an interrupted run: Ctrl+C at several points of a compile and an upgrade, 65
  killed runs tallied (`.tmp/release_0_0_1/0_0_1/interrupt/tally.md`). Done 2026-10-23
  (SWE-2): `upgrade-dpfl` writes through a temporary, the compiler skips a killed upgrade's
  `<output>.upgrading` (`export_upgrade_unfinished`, TC-SRC-11)
- [x] 0.0.1d `just release <version>` (`scripts/release/release.py`) under the codename, the
  portable `.7z` with its SHA256 file. Done (0.0.1d1 the codename, 0.0.1d2 below)
  - [x] 0.0.1d2 `just release`, the readme, a run from a clean worktree at 51e32df0. Done: the
    exe 35.3 MiB, past the size budget's "about 30 MB"
- [x] 0.0.1e the rehearsal on PES 2019 with FoxDen: VGL26's 53 old exports through both
  launchers from the extracted bundle, then checked in game in Edit mode. Done 2026-10-23
- [x] 0.0.1g the Team compiler and the Export upgrader greyed on the sidebar while their CLI
  stays (ST-SIDE-08; `studio`'s `CLI_ONLY`). Done
- [x] 0.0.1h `ingame_face.toml`: the marker becomes a file holding the in-game face, and
  `settings.toml` loses the face, hair and iris keys (DECISIONS 2026-10-24; TC-STR-18 to 20,
  TC-BIN-47 to 49, EU-PLY-21, 22, EU-SET-10, 11, SE-GUI-33, 34). Done 2026-10-25 (SWE-2, three
  slices), SE-GUI-34's earlier form checked in game on PES 21 (re-proved in its patch form at
  8.36d)
- [x] 0.0.1i `libs/aatf`'s audit for 0.0.1: the converge audit, the design pass and a census
  of 424 saves under DBC7's rules, `Rules::player_tier` for 8.9f4d (DECISIONS 2026-10-25).
  Done 2026-10-25 (side-lead B, SWE-2): the census unchanged after the fixes, the close run's
  mutation run none missed (`.tmp/release_0_0_1/0_0_1i/`)
- [x] 0.0.1k the window's diagnostic log, `studio.log` (DECISIONS 2026-10-26): k1 the sink
  and the panic dialog (ST-RUN-07), k2 backtraces as base and offset with
  `scripts/release/symbolize_log.py` (ST-RUN-09), k3 the check in the built window, k4 the
  crash dump (`crates/studio/src/crash_dump.rs`, one `unsafe` block, ST-RUN-10). Done
  2026-10-28
  - ST-RUN-08 manual: checked 2026-10-28 by the lead on a release build of `825f1885` in
    the `ingame` worktree with About's Copy panicking by hand (reverted, rebuilt, the
    bundle's exe free of it), its own data folder (`.tmp/release_0_0_1/0_0_1k3/`, `k3.py`): the click
    shows a dialog repeating "k3 test panic: About's Copy clicked" and naming `studio.log`;
    once closed the process exits 101; the log holds the message, `about.rs:63:17` and 59
    frames, which `scripts/release/symbolize_log.py` names against the kept exe and `.pdb`
    (`about::show` at `about.rs:63`), and ends "the crash above was shown to the member"
    (kept, symbolized, as `.tmp/release_0_0_1/0_0_1k3/symbolized.log`; the later starts rotated it out).
    ST-RUN-09 the same day: no dialog on the first start (no earlier log) nor after the
    reported crash; after the process was killed, a dialog before the window naming
    `studio.old.log`; a WM_CLOSE ends the log "the window closed normally", exit 0
  - ST-RUN-11 manual: checked 2026-10-27 (k4b's release dump, its exe and `.pdb` kept in
    `4cc-studio-temp/st_run_11/`; `dump_syms` then `minidump-stackwalk`: the crashed main
    thread's frames 2-3 `4cc_studio::crash_dump::overflow_the_stack()`, `crash_dump.rs`
    254 and 263, under `__chkstk` and `memset`; the plan's sentence now names `dump_syms`).
    manual: checked 2026-10-27, the built window's normal close leaves no monitor (the same
    release exe, portable over a fresh `data/`; `st_run_11/close_check.py`): two processes
    open, the window and its `--crash-monitor`, none within 15 s of `WM_CLOSE`, exit 0,
    the closing line last, no dump.
    manual: checked 2026-10-28, the window opening when the monitor cannot start: a release
    build of `825f1885` whose `main.rs` handed `watch_crashes` the exe's path with a `.txt`
    extension (reverted by hand, rebuilt: 12 bytes from the clean build, the PE and debug
    timestamps and the PDB signature), over a fresh `data/` (`.tmp/release_0_0_1/0_0_1k4c/`): the window
    opened (`window.png`), one process and no monitor, the log's second line "the crash
    monitor did not start, so this session writes no crash dump: cannot start the crash
    monitor …4cc-studio.txt: The system cannot find the file…", WM_CLOSE's closing line last.
    Linux: `tests/crash_dump.rs` passed in `just gates-remote` on bonfire (Debian,
    2026-10-28; expects SIGSEGV, 11); CI's own job ran red then for another cause, the
    section-pointer check reading the gitignored `.tmp/` (fixed the same day). Reviews:
    SWE-2 S1 7 of 18, S2 1 of 5, Astra A1 (checkpoint b) 0 of 1 (`.tmp/release_0_0_1/0_0_1k4b/rulings.md`)
- [ ] 0.0.1l no console window in the preview (`plans/core/architecture.md` "Windows
  subsystem", DECISIONS 2026-10-25, `AttachConsole` approved): `#![windows_subsystem =
  "windows"]` on every Windows build (the plan's paragraph after it says why debug too),
  `AttachConsole(ATTACH_PARENT_PROCESS)` behind one safe function when command arguments are
  present, the window mode's startup failures in a message dialog, after 0.0.1k (a panic
  needs the log first) → verify: the gates' piped CLI tests (`help_cli`, `launchers`,
  `team_compiler_cli`) on the GUI-subsystem binary; ST-RUN-04. Left: the two launchers'
  check (each prints, its `pause` waits) on 0.0.2's bundle, which has them. Look: Pending
  looks
  - [x] l1 the subsystem on the exe, the attach in `main`, `tests/subsystem.rs`
  - [x] l2 ST-RUN-04 in the bundle: a double-click opens no console; `team-compiler check`
    from a terminal prints there
    - ST-RUN-04 manual: checked 2026-11-04, the published `unlimited-tool-works-0.0.1-windows.7z`
      (its SHA256SUMS hash), extracted to `.tmp/after_cut/l2/`: the exe's PE subsystem is 2;
      launched through Explorer it opened its window and its `--crash-monitor` child, no console
      window appeared (`double_click.log`); from `cmd` in a pseudo-console (`run.bat` through
      `scripts/conpty_run.py`) `UTW --help`, an unknown command's error and `UTW team-compiler
      check` (`Warning no_exports_found`, exit 0) all printed there. Run with no shell
      between, under `conpty_run.py` alone, nothing prints: the console's owner is then the
      script's helper process, not a shell, a case no member meets
- [x] 0.0.1m the window's fit and its scaling with the width (ST-WIN-01, 02): m1 the fit and
  the zoom, m2 and m2b no narrow-pane fallback and no frame laid out under the design width
  (DECISIONS "No narrow-pane fallback, …", "A frame may run three passes"). Done 2026-10-27;
  the size classes (ST-WIN-03) land with the Music player's view (Phase 10)
- [x] 0.0.1n the drafts' Layout edit and scaling. Done 2026-10-26 (SWE-2)
- [x] 0.0.1o Live AATF seen where the fix is (SE-GUI-36; DECISIONS "Live AATF stays off by
  default", "Live AATF's marks: where they sit and their glyphs", "Live AATF's pitch cross:
  left of the badge, over every marker"): o1 the draft, o2 the code (o2a the toggle, the
  list's marks and field crosses; o2b the pitch's cross for `FieldedNotA`). Done 2026-10-27;
  the marks seen in the built window 2026-10-28
- [x] 0.0.1p the upgrader's accepted CLI items for VGL 27's managers (P17, P18, P21): p1
  `unmigrated_content`'s `reason` and `other` (EU-REP-05), p2 `--coverage` (EU-CLI-09), p3
  `staging_folder_removed` (EU-CLI-10). Done 2026-10-26 (SWE-2)
- [x] 0.0.1q the window reopens where it was closed, maximized if closed so (P24, P39;
  ST-WIN-04; DECISIONS "eframe 0.37 gives the shell the monitor list only once the window
  exists", "A maximized reopen resends its position only where the creation can have landed
  wrong"). Done 2026-10-27 (q1-q5), checked in the built window on both monitors
- [x] 0.0.1r a folder per PES version (P25; DECISIONS "A folder per PES version"; ST-SET-04):
  `[common.pes_folders]`; r2 the path edits 472 points, the menu 660. Done; Browse checked in
  the built window 2026-10-28
- [x] 0.0.1s the Save editor's main panel after 4ccEditor's layout, no tabs: s1 the draft and
  the maintainer's looks, s2 `plans/save_editor.md` "Editing UI" rewritten (SE-GUI-37, 38),
  s3 the code (s3a-s3k), s4 the player view's last look, s4t the window title's path
  (`StudioTool::title_detail`), s5 the Tactics editor's grid (DECISIONS "The Tactics editor
  is one grid"), s6 and s7 the maintainer's requests of 2026-10-27. Done 2026-10-27, its loop's
  `mutants-diff 3d39aff3` clean; seen in the built window at 1280 x 800
- [x] 0.0.1t a rules file a member handles ends in `.4ccrules` (DECISIONS "A rules file a
  member handles ends in `.4ccrules`"): `aatf::RULES_FILE_EXTENSION`. Done 2026-10-26 (SWE-2)
- [x] 0.0.1u the theme pins egui 0.36's spacing, corner radii, `interact_size` and widget
  colours (DECISIONS "The theme pins egui 0.36's spacing and corner radii", "The player list
  reserves its scroll bar's band"): `theme::install`. Done 2026-10-27 (SWE-2)
- [x] 0.0.1v the preview's icon (DECISIONS "The preview's icon and its build dependency"):
  v1 the assets (`scripts/fonts/make_icon.py`, `resources/icon/`), v2 the window icon and the
  exe's resources (`winresource`). Done 2026-10-27; the taskbar, Alt+Tab and Explorer icons
  checked by the maintainer 2026-10-30
- [x] 0.0.1w the drafts and the app draw the same fonts: the body font Roboto Flex
  (DECISIONS "The body font is Roboto Flex at its default axes, unscaled"), the symbols font
  `resources/fonts/symbols.ttf` (`scripts/make_symbols.py`), the drafts' native controls as
  egui draws them, a modal's title at 14 points, the Tricks or Free cards panel (w6; DECISIONS
  "A Tricks panel under Skills", "A Free cards panel for a ruleset without tricks"). Done
  2026-10-28, seen in the built window
- [x] 0.0.1x the formation's name on the pitch (8.32; SE-GUI-39; DECISIONS 2026-10-27
  "Formation pictures, the formation's name and the nation names: the maintainer's
  answers"): the rule from PES 2021 captures, the plan, the draft (approved 2026-10-28), x4
  below. Done; seen in the built window 2026-10-28
  - [x] x4 the code: `team_widgets::pitch::POSITION_ROWS` and `formation_name`, the name in
    the `label` family (Opus, with y2)
- [ ] 0.0.1y the Tactics pitch's two swaps as the game's (the maintainer, 2026-10-27: drag
  swapping should behave as the game's, and the select-one-then-the-other way be offered "if
  easy enough"; DECISIONS 2026-10-27 "A pitch player dropped on another swaps the two slots in
  the shown formation, as the game's drag"; the save diffs in `.tmp/release_0_0_1/0_0_1x/captures.md`)
  - [x] y1 the draft: a marker dropped on another exchanges the two slots' position, X and Y
  - [x] y2 the code (with x4): `formations[..].players.swap`, the keeper no target; the
    maintainer's look: "Looks fine" (2026-10-28)
  - [x] y3 the position areas: no fixed areas to map, measured in PES 2021 (DECISIONS
    2026-10-27 "A pitch drop keeps the position: the game's rule is not a map of areas";
    `.tmp/release_0_0_1/0_0_1y3/findings.md`)
  - [x] y4 the dragged player's position rule read from PES 2016's executable (DECISIONS "The
    dragged player's position rule is read from PES 2016's executable", "The drag rule's
    overall is PES 2016's formula until a version's own is measured"): y4e1
    `PlayerEntry::overall_at`, y4e2 `team_widgets` `pitch/drop_rule.rs`. Done 2026-10-28,
    checked in game on PES 2017 and in the built window. The PES 2017 check left untested the
    kicking power/swerve and body control/physical contact mapping and the B bonus (look:
    Pending looks)
  - [x] y5 the slot popover's position row: no label, the dropdown and Make GK spanning it.
    Done 2026-10-28 (SWE-2)
  - [x] y6 the badge's overall and medal outline (`RosterRow::overalls`). Done 2026-10-28
    (SWE-2), seen in the built window
  - [ ] y7 Substitution's Very early on PES 2018 to 2020 too (the maintainer, 2026-10-28:
    taken from PES 2017 and 2021 having it, the games between them)
    - [x] y7a the draft and `plans/save_editor.md` "Tactics editor"
    - [x] y7b the code: `team_widgets`' `substitution_choices` offers Very early from PES 2017
    - [ ] y7c later (the maintainer: "we'll check in detail in the future"): a look at PES
      2018, 2019 and 2020's Game Plan, Automatic settings, Substitution; the plan's "taken
      rather than looked at" goes with it (`QUESTIONS.md` "After the release"). Look: Pending
      looks
- [x] 0.0.1z the maintainer's look at the built app (2026-10-27): the app's sliders kept, the
  drafts made to match (z1, z4); z2 the code (the slider's fill, Reset window, ST-BAR-05,
  06, SE-GUI-40; z2c Reset window under zoom), z3 the active-tool bar's left corners. Done
  2026-10-29, seen in the built window
- [x] 0.0.1j the default-settings audit before the cut (j1 the update rows hidden until
  0.2.0, j2 the readme says to pick the version first, j3 Live AATF off, j4 no settings
  section for a CLI-only tool, j5 the bundle ships VGL 27's rules file). Done 2026-10-27
- [x] 0.0.1cm comments that stand on their own (DECISIONS "A code comment stands on its
  own"): the 1044 comments `just comment-refs` listed rewritten (cm1-cm4), `comment-refs` in
  `just gates` (cm5), `doc_pointers.py` reading a wrapped pointer (cm6). Done 2026-10-27
- [ ] la the library crates audited again (the maintainer, 2026-10-27: "the library crates need
  to be as solid foundations as they can be"): four read-only Fable audits of every lib crate
  but `team_widgets` (brief `.tmp/topics/lib_audit/brief.md`), 55 findings, ruled by the lead
  (`.tmp/topics/lib_audit/rulings.md`): 43 accepted (some conditionally, one after the cut), 12
  rejected with a line each. The audit of `music_export` stopped at its context limit after the format core
  (`select.rs`, `eval.rs`, `resolve.rs`, `state.rs`, `write.rs` unread; Phase 10). Rework in five slices
  under `.tmp/topics/lib_audit/rework_common.md`, Opus sidekicks, each reviewed and gated before it
  lands, `mutants-diff` over the loop at its end
  - [x] A1 `pes_savefile`'s behavior and spec fixes (`slice_a1.md`)
  - [x] C the model stack (`slice_c.md`)
  - [x] B `aatf`, `pes_version`, `kit_config`, `color_tools`, `pipeline`, `vtree` (`slice_b.md`)
  - [x] D `dds_convert` and the containers (`slice_d.md`): the Converter rewraps blocks a
    target keeps without decoding them
  - [x] A2 `pes_savefile`'s design items and `aatf`'s bare record (`slice_a2.md`)
  - [x] the loop's `mutants-diff ba7a4b6d` (2026-10-28): 3 survivors, all equivalent
    (`883eb9ad`); A2's known gaps tested
  - [ ] after the cut: `aatf`'s nine fixture modules' repeated tests as helpers (b-3)
- [x] 0.0.1ov every built window laid over its draft before the final pass: ov1 the
  comparison (`.tmp/release_0_0_1/0_0_1ov/rect_compare.py`), ov2 the fixes (ov2a-ov2z: the
  1280 × 780 client area, blue checkboxes, the modal helper, the form grids' 22-point rows, the
  wheel over a value, ST-WHEEL-01 to 04), ov3 the overlap test, ov4 the drafts' Overlay control
  and `scripts/drafts/draft_shots.py` (8.20's), ov5 `scripts/drafts/vet_drafts.py`, ov8-ov14.
  Done 2026-10-29. Built-window looks it left open: look: Pending looks
  - [x] ov7 Live AATF replaces the AATF panel (DECISIONS "Live AATF replaces the AATF panel;
    the final pass's fourth rulings"): the toolbar's ruleset dropdown, every ruleset built
    in, Multi-team AATF (SE-GUI-41)
  - [x] ov11 the light theme (DECISIONS "Every colour comes from a switchable palette, none
    written in a view"), ov11a below; the light rulings built, the opened sidebar below modals
    - [x] ov11a the palette: `studio_core::theme::Palette`, `DARK` and `LIGHT`; `just
      colour-literals` in the gates. Merge `f145b688`
- [x] 0.0.1rn the project renamed Unlimited Tool Works (DECISIONS "The project's name in every
  spelling"): rn1 the code and the files a user receives (`UTW.exe`, merge `21190bba`), rn2
  the drafts, rn3 the docs, rn4 the repository `the4chancup/unlimited-tool-works`. Done
  2026-10-29
- [x] 0.0.1g the bundle for DBC alone (DECISIONS "0.0.1 is the Save editor for DBC alone;
  VGL's tools wait for 0.0.2"): no launcher, `exports/` or `old_exports/`, PES 2017 picked.
  Done (`3d7fdc33`, merged `7dc14f79`); 0.0.2 restores the launchers, the folders and the
  readme's VGL section from git, rewriting `release_test.py`'s
  `test_the_bundle_ships_no_launcher_and_no_export_folder`
- [ ] 0.0.1cv the Save editor's converge review before the cut (the maintainer asked,
  2026-10-30, whether a review loop had covered the whole UI: none had; every loop ran per
  slice or surface, the crate audits covered `aatf` (0.0.1i) and the libs but `team_widgets`
  (la), and 0.0.1ov and 8.38 compared the look, not the code). `RUNBOOK.md` "Closing a phase"
  step (1) over 0.0.1's surface, the Save editor on PES 2017 (`save_editor`, `team_widgets`,
  `studio_core`'s shell, about 56,000 lines with 38,000 of tests): the lead's audit first
  (every `plans/save_editor.md` section 0.0.1 ships compared with its code, every SE-GUI
  scenario on PES 2017 with a test or a `manual: checked` line, the design-tell sweep over
  the whole surface), its gaps briefed; then Astra over the surface in parts, a SWE-2 loop
  after each round; the whole-crate mutation runs of `save_editor` and `team_widgets`,
  after the cut (DECISIONS 2026-10-31, "0.0.1's whole-crate mutation runs come after its
  cut"). Runs beside the maintainer's final pass (0.0.1f1c); the cut waits on it (`RUNBOOK.md`
  "Cutting a release")
  - [x] cv1 the lead's audit (2026-10-30, at `5d0c105d`): 63 findings ruled
    (`.tmp/release_0_0_1/0_0_1cv/rulings_audit.md`); no finding writes a wrong value to a save
  - [x] cv2 the plan and scenario edits the audit ruled (17 rulings)
  - [x] cv3 the view's findings (`save_editor`, `team_widgets`): merged `7e219402`; its
    `mutants-diff 5da2edf0^1` left partly to cv6's whole-crate runs
  - [x] cv3b merged `ac53806e`: the Texport name suggestion counts in hexadecimal (DECISIONS
    2026-10-30 "PES 2017's import names the team from the DLC's bin, and Texport numbers are
    hexadecimal")
  - [x] cv4 the shell's findings (`studio_core`, `studio`): merged `ba379914`; Reset window
    centres on the window's own monitor
  - [x] cv5 by hand, in the release build of `a8e01940` (portable copy, captures
    `.tmp/release_0_0_1/0_0_1cv/cv5/`)
    manual: checked 2026-10-30 by the lead: Output rosters to TSV asks with a save dialog
    named `EDIT.tsv` and writes UTF-8 with a byte-order mark, 210 teams of 23 rows from the
    maintainer's PES 2021 save, accented and symbol names intact (SE-GUI-27)
    ST-SIDE-05 manual: checked 2026-10-30 by the lead, PES 2017 running from
    `E:/PES2017`, the selector at 21 and `pes_folder_path = 'E:/PES2021'`: the poll switched
    to 17 with "Switched to PES 2017" and Fix PES path, which wrote `E:\PES20**`; set back
    to 21 by hand, two polls later still 21 with the game running; the PES 2017 save folder
    unchanged by hash
    RE-FILE-08 manual: checked 2026-10-30 by the lead: Open and Save as start in the folder
    of the last file, also after a restart, and in the folder `last_folder` names when it
    was pointed elsewhere while closed (so the setting, not Windows' own dialog memory, picks
    it)
    manual: checked 2026-10-30 by the lead: the Save editor and the Ruleset editor together:
    each keeps its document and unsaved mark across switches; closing with both unsaved
    shows the Save editor's prompt first, Cancel keeps the window with no further prompt,
    Discard brings the Ruleset editor into view with its own prompt, whose Discard closes
    the window; the real save untouched (ST-RUN-05)
    manual: checked 2026-10-30 by the lead: the ownership line at 1280 wide: the sentence
    fills a row and Apply aesthetics patch starts the next one whole
  - [x] cv8d the Ruleset editor's Save as name, the event-logo note, the summary's header, the
    unbuilt sections' demo links. Merged `e1f7cf41`
  - [x] cv8e the drafts' demo mode, `demo=1` (`docs/prototypes/index.html`). Done, checked
    on the published page 2026-10-30
  - [x] cv8f the General section's "Event page (optional)" and "Event logo (optional)"
    labels. Merged `6c7fcd18`
  - [x] cv9 the Tactics pitch as PES 2017 has it (DECISIONS 2026-10-31): cv9a an outfielder
    stops at y 4, cv9b each version draws its own screen's pitch (`PitchLook`). Merged
    `d93c48d6`, checked in the built window 2026-10-31; PES 2019's look in 0.0.2
  - [x] cv10 the Open menu labels a save by version alone, Ctrl+5 to Ctrl+1 and Ctrl+O
    (DECISIONS 2026-10-31, SE-GUI-26). Merged `f6bba2af`
  - [x] cv11 the pitch badges' medal outlines no longer blink on every edit. Merged
    `f6bba2af`
  - [x] cv12 the release exe holds no path of the machine that built it (DECISIONS
    2026-10-31; `scripts/release/release.py` `remap_flags`). Done 2026-10-31; the toolchain
    stays 1.98.1
  - [ ] cv13 after the cut (the maintainer, 2026-11-01: "It can wait"): the window opens
    slowly (the maintainer, 2026-10-31, on the 03:08 release exe):
    the exe itself starts in about 170 ms (`--help`, three runs), but the CPU was at 100%
    (a mutation run and a sidekick), and the log shows wgpu probing four adapters on three
    backends (Vulkan, DX12, GL). Measure, on an idle machine, the time to the first frame as
    is and with eframe's wgpu instance limited to one backend
    (`studio_core` `shell/window_size/mod.rs` `native_options`); the maintainer wants the UI
    on a single backend if the cons are not noticeable. To weigh: on Windows DX12 runs on
    every Windows 10 machine and falls back to its software adapter (WARP), where Vulkan needs
    a driver that ships it; Linux has no DX12 (Vulkan, GL for old GPUs and VMs); the GPU
    encoding study (16.x c) creates its own instance. Needs the window on screen: once no
    sidekick builds.
    - measured 2026-10-31 (`.tmp/release_0_0_1/0_0_1cv/cv13_first_frame.py`, `cv13_times.txt`): idle,
      the first frame 528 ms as is, 419 with DX12 alone, 449 Vulkan, 592 GL; probing the
      extra backends costs ~120-140 ms; under load 0.8-1.3 s median with outliers near 6 s
    - loads apart 2026-11-01 (`.tmp/0_0_2/cv13/cv13_load.py`, `cv13_load.txt`): CPU
      contention stalls it (2.5 s as is, 1.8 s with DX12 or Vulkan alone), disk alone barely
      does; the wait looks to be outside the app's own threads
    The maintainer's target: the first frame under one second, two at most; met at rest,
    missed under load. A stall that is disk contention alone is accepted (the maintainer,
    2026-10-31: members rarely run heavy builds while working on cup files); one that is CPU
    contention is not. Next, on a quiet machine: more rounds of `cv13_load.py cpu` and
    `cv13_who.py`, to find the process the start waits on; a WPR trace needs an elevated
    prompt, so it is the maintainer's if it comes to that. One backend at rest (~130 ms) is
    decided after that
  - [x] cv14 a ruleset may allow illegal formations (the Whack Patch; plan "Illegal
    formations", SE-GUI-45, RE-SUM-01, DECISIONS): cv14a the `aatf` field, cv14c the Universal
    rules page (20.6a2c, RE-GUI-20), cv14b the Save editor's pitch under the flag, cv14g the
    in-game look on `E:\PES2017`. Done 2026-10-31
  - [x] cv15 `check-aatf --rules` takes an embedded ruleset's stem (`--rules dbc7`)
  - [x] cv16 the height findings name their players (P47; cv16a the logic, cv16b the detail
    lines). Merged `80425dda`, `6993763e`; seen in the built window 2026-10-31
  - [x] cv17 Multi-team AATF's dialog 900 points wide, its rows wrapping. Done with cv16b
  - [x] cv18 the filter box also lists the players of a team whose name matches (SE-GUI-46).
    Merged `dcd441ac`
  - [x] cv19 a double click on a team row sets the team combo to that team. Done with cv18
  - [x] cv20 the wheel steps the sliders, their value boxes and the toggles (ST-WHEEL-05).
    Merged `2b21b3e9`
  - [x] cv21 Live AATF lists the players' rows in two columns, the starting eleven left
    (SE-GUI-47). Merged `5b00c831`
  - [x] cv22 bug: an advanced instruction picked back to Unassigned kept the unsaved mark (a
    target-less pick wrote 255); the pick puts back the target byte as opened. Merged
    `7b8d1675`
  - [ ] cv23 the wheel on Ability stats' boxes lags behind the notches, badly under CPU load
    (the maintainer, 2026-10-31, Live AATF off). By design a notch shows a frame after the one
    that reads it, a fast turn steps one notch a frame, and a box holding the keyboard waits two
    frames more (`studio_core` `widgets/wheel.rs`), so the lag is a few frames times the frame's
    cost → measure first: a Save editor frame's time on the maintainer's PES 2017 save at Ability
    stats, idle and on a step's frame, headless, release build; then cut where the time goes;
    with it, what one Multi-team AATF check over 220 teams costs the UI thread (cv26's
    rest cuts how many checks start, not what one costs), and what the card badge's
    `player_tier` and `card_limits` cost on an edit of a team read under the nearest system
    (since cv33 each runs the candidates' checks; cached per key, once per edit).
    Measured 2026-11-01 (`.tmp/0_0_2/cv23/run1.log`, `run2.log`): the frame after a notch
    114 ms, the Cards left badge's `player_tier` and `card_limits`; one Multi-team check over
    220 teams 3.7 s wall on all cores (about 160 ms a team), a dialog frame listing them 52 ms
    - [x] cv23a the badge on a worker (DECISIONS 2026-11-01). Merged `56882356`: the frame
      after a notch 0.72 ms; seen in the built window 2026-11-01
    - [ ] cv23b the Multi-team dialog's 52 ms frames: find where a listing frame's time goes
      (220 teams' rows laid out each frame), then cut it. Measured on a quiet machine
      (`.tmp/0_0_2/cv23/run4.log`, `run4_b.txt`, the merge `56882356`): a listing frame 23 ms
      median (the 52 was under load), of which `violations_list` 21.3, `listed_players` 0.26,
      the team list 0.02. Each row words its sentence and detail and makes three text layouts
      every frame, for every team's rows, visible or not. Usable as it is (a report read
      once, ~40 frames a second); after the cut: lay out only the rows in view, their heights
      kept per check key and width, or keep the worded rows per check key
    - [ ] cv23c the logic's cost: a team check about 160 ms of CPU, the team reading redone
      by each host call (`height_system` in `player_tier`, `card_limits`, `tier_values` and
      `check_team`); after the cut unless cv23b shows it is what the dialog waits on
  - [x] cv26 the wheel over Multi-team AATF's range combos froze the window: both ends open on
    the first team, the check after 500 ms of rest (SE-GUI-41, DECISIONS 2026-10-31). Merged
    `5d7fb22b`
  - [x] cv27 a hovered AATF row outlined, not filled (DECISIONS 2026-11-01). Merged
    `56882356`
  - [x] cv28 Live AATF in a window of its own, embedded (SE-GUI-48), the dock's grip
    (ST-LOG-04). Merged `6bd86314`; seen in the built window 2026-11-01
  - [x] cv29 the Live AATF dock's close button (SE-GUI-49). Merged `149e99db`
  - [x] cv30 team-wide AATF rows first, the rows' tooltips, egui's monochrome emoji fonts.
    Merged `7b778cf4`
  - [x] cv31 the Ruleset editor's Blank template's label (DECISIONS 2026-11-01)
  - [x] cv32 cv28-cv30's review rework. Merged `0ff2d1f6`; the loops ended
  - [x] cv33 a team no quota system takes is read under the nearest system, fewest errors
    deciding (DECISIONS 2026-11-01). Merged `e1d6cddf`, `c27e945a`
  - [x] cv34 P48: Multi-team AATF counts the teams under each system (SE-GUI-51). Merged
    `6c4147ca`
  - [x] cv35 Set PES path over the team view's pitch while the selector is orange or red
    (SE-GUI-50, DECISIONS 2026-11-01). Merged `29cf30ee`
  - [x] cv36 the cv28-32 mutation runs' survivors tested. Merged `86475f0b`
  - [x] cv37 the AATF and comparator rows at the draft's inset and gap. Merged `b6d49de6`
  - [x] cv38 the wheel over a Tactics toggle wraps round (ST-WHEEL-05, DECISIONS 2026-11-01).
    Merged `f2c35716`
  - [x] cv39 a press held in the Multi-team AATF dialog while its check lands no longer
    clicks Close (`skip_ahead_auto_ids`). Merged `b5f8bd25`, after the cut
  - [x] cv40 three view tests raced a worker's reply under load: `workers_replied`. Done
    (lead, test code only)
  - [x] cv23d the idle Ability stats view asks for no repaint. Merged `cb6df900`
  - [x] cv24 event logos resampled to the box's pixels (Lanczos3). Merged `da2ce96d`
  - [x] cv25 the colour picker's popup as wide as its widest row. Merged `da2ce96d`
  - [ ] cvA the Argon trial (Gemini 4 Argon High as a third reviewer family; the maintainer,
    2026-10-31, on Devin `work`'s last weekly usage before its reset): a read-only sweep of
    every crate in 87 parts, libraries first (`.tmp/argon/sweep.py`, worktree
    `../4cc-studio-temp/wt-argon` at `2c5fe5a9`, outputs `.tmp/argon/parts/`, rulings
    `.tmp/argon/rulings.md`). Stopped 06:04 by the maintainer (Devin `work` at 93%, the rest
    kept for Astra): 7 of 87 parts run (pes_version, pes_savefile 1-6), the other 80 not
    started; no new Argon review without the maintainer's word. The accepted concerns become
    one step per crate (cvA1: the fixes first, the mutation run after, an order the
    30-hour chain set)
    - [ ] cvA1 `pes_savefile`: Argon's accepted concerns (`.tmp/argon/rulings.md` A1-A6, A19:
      26 accepted, 8 of them in part; 10 marked "run"); the fixes landed (cvA1r, cvA1r2).
      Left: `just mutants pes_savefile` (its last whole-crate run is 2026-10-16) over the
      fixed code, after the 0.0.1 cut: each "run" concern it confirms as a survivor gets its
      test, the rest drop (A4-6, A6-3, A6-4 unreached by the diff)
      - [x] cvA1r the rework: `mutants-diff 2337e4f6` triaged, SWE-2 round 1's items. Merged
        `06a262c6`
      - [x] cvA1r2 a Team TOML refusal names the value as written, in the file's units.
        Merged `a881b621`
  - [ ] cv6 Astra over the surface in parts, a SWE-2 loop after each; the whole-crate
    mutation runs. Left: the whole-crate runs of the 0.0.1 crates, after the cut;
    `save_editor`'s was stopped 2026-10-31 (extra kinds done, 40 survivors triaged in
    `.tmp/release_0_0_1/0_0_1cv/triage_se_extra.md`; main run 119 of 1052 local mutants), rerun from
    scratch then
    - [x] cv6a and cv6b merged `468b49e0`: Astra's six parts ruled, SWE-2 S1-S3, every loop
      ended
    - [x] `mutants-diff bd75e682` over cv6a and cv6b (73 mutants)
    - [x] cv6t the triage: the missed mutants tested, the TIMEOUTs classified. Merged
      `064a8eec`
  - [x] cv8 the Ruleset editor in 0.0.1 as "Ruleset editor (WIP)" (DECISIONS 2026-10-30):
    cv8a off `CLI_ONLY`, cv8b `squad_size` out of `libs/aatf`, cv8c below, the converge (A7,
    `just mutants ruleset_editor`). Done
    - [x] cv8c merged `cc3527e1`: A7's rework (a typed tier id counts as unsaved; the summary
      states the logic's rules, DECISIONS 2026-10-30)
  - [ ] cv7 after 0.0.1: split `view/mod.rs` (the toolbar), `view/menus.rs` (the dialogs),
    `view/interchange.rs` (the export half), `team_widgets` `pitch/mod.rs` (the paint) and
    `tactics/mod.rs` (the changes' terms), each past a thousand lines
- [ ] 0.0.1f the cut, once the Save editor's view and cv are done (the Ruleset editor ships as
  far as it is built, labelled (WIP), `plans/core/development_plan.md` "Development Plan"):
  the bundled settings picking DBC7, built into the exe with the frozen logic version 1, as the Save editor's `aatf_ruleset`
  (ov7: no rules file beside the exe); the clean-machine check (`plans/core/development_plan.md`
  "Release 0.1.0"'s verification; 0.0.1's bundle has no launcher, 0.0.2's adds both back, "The
  /vg/ League 27 (VGL): preview 0.0.2"), the exe's size measured crate by
  crate with `cargo bloat` (35.3 MiB at 0.0.1d2, past the budget's "about 30 MB";
  `spec/core/distribution.md` "Distribution: portable .7z bundle" gets the new measurement),
  logic version 1's hash
  frozen in `LOGIC_VERSIONS` only after the maintainer's final confirmation, given once the UI
  is polished (the maintainer, 2026-10-25: never before it) (`plans/aatf_rules.md` "Loading and validation": a later logic
  change is version 2), `just release 0.0.1`, handed to DBC's testers (VGL's organizer gets 0.0.2, its launchers and readme section).
  Prerequisite: 0.0.1cv, the converge review (`RUNBOOK.md` "Cutting a release")
  - [x] f1 the preparation (side-lead U, 2026-10-27; DECISIONS "The bundle's `data/` holds its
    settings and DBC7's rules file"): `release.py` `stage_data`, the readme
  - [x] f1c (closed by the cut, as 8.38) the maintainer's final pass over the built app's UI;
    the candidate `release/0.0.1-candidate`, staged by `make_candidate.py`
  - [x] f1d the maintainer's changes after DBC7's commissioner approved 0.0.1f (f1d1 Team
    TOML's `bench_order` of 12, f1d2 both instruction sides under illegal formations, f1d3).
    Done 2026-11-01, candidate 0.0.1g staged
  - [ ] f1e the maintainer's f1c pass on 0.0.1g (2026-11-01): everything as listed, except the
    Tactics "Starting eleven" and "Bench" rows: a press on a row's text selects the text
    instead of dragging the player (an I-beam over the position and the name; only the gaps
    drag). On branch `f1e` from `40b274e5` (worktree `wt-f1d`), merged into the candidate and
    main, candidate 0.0.1h staged for the maintainer's look, then gold (f2). The row's labels
    stop being selectable (`team_widgets` `pitch/lineup.rs` `lineup_row`; brief
    `wt-f1d/.tmp/brief_f1e.md`) → verify: a headless test reads the grab cursor over each
    row's handle, position and name (red: the text cursor); the maintainer drags from a name
    in the built window (look: Pending looks). The press is taken 2 points outside the
    labels, where egui hands it to a close click-sensing widget (the maintainer: "four areas
    of glitch per player"). Reviewed with f1e2-4 (`wt-f1d/.tmp/f1d_rulings.md`; `mutants-diff
    f1bc0502` none missed)
    - [ ] f1e2 (the maintainer, 2026-11-01, same branch): Up and Down walk the player list
      (`plans/save_editor.md` "Editing UI", the player list's paragraph; DECISIONS "Up and
      Down walk the player list"; drafted in `save_editor.js` `onKey`) → verify: headless,
      Down from a team's last player selects the next team's row and Up returns; the ends
      stop; a focused number field keeps its arrows; the selected row scrolled into view
    - [ ] f1e3 (the maintainer, 2026-11-02, same branch): the import's section boxes read
      Stats, Aesthetics and Gameplan (the cup's words); the free-kick motion imports with
      Aesthetics, and Copy and Swap stats stop moving it; `[team]` splits, its colours, kit
      slots and strip flag going with Aesthetics (`plans/save_editor.md` "Interchange formats in the UI" and
      the inventory's Copy stats row; DECISIONS "The import's sections are Stats, Aesthetics
      and Gameplan"; drafted in `save_editor.js` `ixWrite`) → verify: headless, the boxes'
      labels; a Stats-only import leaves the motions, colours and kits as the target's and an
      Aesthetics-only import writes them; Copy stats leaves the free-kick motion
    - [x] f1e4 (closed by the cut, as 8.38) on Windows the window draws through Direct3D 12
      and OpenGL, not Vulkan (the notebook's Intel driver crashed in Vulkan; DECISIONS "On
      Windows the window draws through Direct3D 12")
  - [ ] f1f (the maintainer, 2026-11-02: the cut moves a day; DECISIONS "0.0.1 takes a second
    batch of small changes before its cut"), each item plan sentence and draft first where it
    changes what a member sees, on a branch from `f1e` once 0.0.1h's items land. Reviewed
    (`.tmp/lead/f1f_rulings.md`), the candidate fast-forwarded to `f22a76ae`:
    - [x] f1f1 the lineup list outlines the row a dragged row would replace. Done 2026-11-02
      as `cbbb727e`, on `f1e`
    - [x] f1f2 two copies of UTW.exe run at once, each on its own save. Done 2026-11-02 as
      `d0ac4845`, on `f1e`
    - [x] f1f3 Ctrl+C/Ctrl+V on team rows, presets and formations. Done 2026-11-02 as
      `a4545968`, on `f1e`
    - [x] f1f4 a VGL 27 tester's captain bonus not shown under the limit: the untaken
      choice's card counted in the badge's limit. Done 2026-11-02 as `12512060`, on `f1e`
    - [x] f1f5 the Stronger foot/hand toggle's two colours swapped. Done 2026-11-02 as
      `091c725b`, on `f1e`
  - [ ] f1g (the maintainer, 2026-11-02, after the lead's look at 0.0.1h: a polish batch
    while their look waits; one candidate, 0.0.1i, then their look), each item plan and
    draft first where a member sees it, on a branch from the candidate's `f22a76ae`:
    - [x] f1g1 Live AATF's heads in two tones (SE-GUI-63). Done 2026-11-02 as `6dbd8be6`
    - [x] f1g2 P49: a `Fielded position` row's tooltip. Done 2026-11-02 as `4bccd760`
    - [x] f1g3 P50: the toolbar's shortcuts held off only by a text field or a dialog
      (SE-GUI-62). Done 2026-11-02 as `4bccd760`
    - [x] f1g4 PgUp/PgDn by team rows (SE-GUI-53). Done 2026-11-02 as `7fece457`
    - [x] f1g5 an Invert value scrolling setting (ST-WHEEL-06). Done 2026-11-02 as `4e28469b`
    - [x] f1g6 `cargo bloat` on the release build: 41,491,456 B; closed 2026-11-02 with no
      change taken (DECISIONS "egui's default fonts stay")
    - [x] f1g7 CI for GitHub releases, Windows and Linux (DECISIONS "0.0.1 is built by CI",
      "How CI's release workflow builds, checks and keeps the bundles"):
      `.github/workflows/release.yml`, the Linux `.tar.gz`, the test tag `v0.0.1-rc.1`. Done
      2026-11-03
    - [x] f1g8 whether 0.0.2-poc fits in 0.0.1: cancelled by the maintainer 2026-11-02
    - [x] f1g9 the update check (DECISIONS "The update check ships in 0.0.1"): slices 1-2
      done 2026-11-02 (`79497010`, `ce963700`); f1g9c Update now leaves the dialog
      (`b05de149`); slice 3 postponed to 0.0.2 (0.0.2-upd)
    - [x] f1g10 the comparator's heads with the hyphens, Live AATF's `All clean` line. Done
      2026-11-02 as `6ab65df9`
    - [x] f1g11 the player list's drag outlines the row it would land on (SE-GUI-64). Done
      2026-11-02 as `5593c356`
    - [x] f1g12 the team row's hint reads `[click for gameplan]`. Done 2026-11-02
      (`3e1a9d85`)
    - [x] f1g13 four fixes from the look at 0.0.1i-pre2 (the update interval in minutes,
      SE-GUI-64, 65, 61). Done 2026-11-03 (`4b68d2d7`, `b9954f8a`)
    - [x] f1g14 three fixes before gold (ST-COMBO-01, SE-GUI-66). Done 2026-11-03 as
      `699a77ab`
    - [x] f1g15 Stronger foot/hand's chosen side white (SE-GUI-61). Done 2026-11-03 as
      `ed74a327`
  - [x] f2 logic version 1's hash frozen (`f6055ebe`), `just release 0.0.1` by CI, the
    clean-machine check, the hand-over. Done 2026-11-03: tag `v0.0.1` on `c97ce914`, CI run
    41564671137, published by the maintainer as "Unlimited Tool Works v0.0.1 pre-alpha"
  - [x] f3 done 2026-11-04 (scripts `.tmp/lead/rewrite/f3_*.py`): 171 branches to `main` alone,
    worktrees pruned to the kept 8, the unmerged branches kept as `refs/archive/<branch>`, 35
    subjects reworded with `git filter-repo` (the old history `refs/archive/pre-reword/`)
  - [x] f4 superseded 2026-11-02 by f1g7 (DECISIONS "0.0.1 is built by CI"). Was: CI release
    builds after the cut, sliced:
    - [ ] f4a the design in `plans/core/distribution.md`: the tag trigger, the draft release's
      assets, how the Windows asset is checked against the locally checked exe (byte-identical,
      or the release checks plus the maintainer's look when CI's build cannot match), what
      the Linux bundle holds (`utw`, `quick_compile.sh`, its archive)
    - [ ] f4b the Windows bundle built by CI: `scripts/release/release.py`'s checks (no remapped
      prefix, static C runtime, the user's name absent) passing on CI's exe; the workflow
      written by the lead (CI is correctness-critical) → verify: a test tag produces a draft
      release whose `.7z` passes the checks
    - [ ] f4c the Linux bundle in `release.py` and the workflow, its file list and tests; the
      GUI run on a real Linux machine (the maintainer's, or a VM) before it is attached
  - [x] f5 superseded 2026-11-02 (DECISIONS "cv39's fix ships in 0.0.1": onto f1g)
  - [ ] f6 after the cut (the maintainer, 2026-11-02), moved out of the cut. Done: (1) f1e3's
    SWE-2 round and its rework f1e3r (2026-11-04); (2) the exe's size crate by crate, 41.6
    MiB, in `spec/core/distribution.md` "Size budget" (2026-11-04); (3) l2's terminal half
    (2026-11-04); (4) the GPU encoder's crash guard planned, built with the wiring after
    0.1.0; (6) PES 15's fluid formation switch, closed without code (no byte in the save);
    (7) f1g14's and f1g15's mutation runs (`.tmp/after_cut/f1g14m/`); (8) the whole-crate runs
    of `team_widgets`, `pes_savefile` and `save_editor`, every survivor triaged and its tests
    landed (`.tmp/after_cut/mutants_whole*/triage.md`); (9) the `review2` worktree removed,
    nothing to merge; (10) Astra A1 over f1e3r, cv8f, cv10 and cv11, its fix cv11r. Left:
    - (5) the Ruleset editor's undo merges a run of steps to one value as 8.42 does the Save
      editor's (`plans/ruleset_editor.md` the undo paragraph; DECISIONS "A run of undo steps
      is keyed by the widget call") → verify: ten wheel notches on a tier's count, one Undo
    - (8)'s two design findings, open: `team_widgets` `pitch/popover.rs` `slot_popover`'s
      outside-press focus surrender is unobservable under egui 0.37 (`Focus::end_pass` drops
      an undrawn widget's focus; the DragValue commits as typed), so the block and its "cannot
      end on its own" comment look removable: check against an egui upgrade first; and
      `save_editor` `abilities.rs` `playable_positions`' `column_width` measured 350 against
      an available 350 over the whole suite (9,008 frames), so its `max` and `+ CELL_PADDING`
      decide nothing where the tests look; whether a wider window or font ever makes them
      bind is unmeasured, its `+` -> `*` mutant a survivor until that is settled
  - [ ] f7 a design-health pass over the tests (the maintainer, 2026-11-04: tests are code,
    `CONTRIBUTING.md` "Tests are code"), after f6's tw2 and cv11r land, before 0.0.2 starts.
    Not a full read of the repo: ~390K Rust lines would be ~600 SWE-2 parts, every concern
    ruled by the lead. (a) Measure: a script (`scripts/review/`, with its test) hashing
    normalized ~8-line windows across every crate's `.rs` files lists the duplicated blocks
    by size and file, and the modules past about a thousand lines; (b) SWE-2 sidekick briefs
    for the top 10 to 15 files it names (GUI test modules expected first), each naming its
    blocks, the shared helpers extracted, the tests still passing and still failing under a
    perturbation; (c) then, to evaluate (the maintainer's idea, 2026-11-04), the rest of the
    tests swept by a SWE-2 side-lead the maintainer launches in a Devin chat (it compacts
    itself, where a long sidekick session hangs), running SWE-2 subagents, judging their
    findings and applying fixes, then reporting every decision. The lead writes its handoff
    (`.tmp/after_cut/f7/handoff.md`) with: test code only, no assertion changed (a production
    finding reported, not applied); per change the tests passing, the test count changed only
    by merged tests, each touched test failing under a hand perturbation, the per-file assert
    count down only by listed merged duplicates; one commit per test module on its own branch
    and worktree, each decision naming its commit (keep or `git revert`); the lead's
    acceptance bar (reach, not truth: a copied scene, repeated checks, a case for an existing
    table, never a taste-level reshuffle); one cargo build at a time at `CARGO_BUILD_JOBS=4`
    while a mutation run holds the other, no `rm`, no push; (a)'s ranking worst first, stopping
    under a block-size threshold, (a) rerun at the end for the before and after; otherwise
    each phase's close design-health pass covers the rest
    - [x] (a) 2026-11-04: `clones.py --files --window 8`, the per-file ranking of test code
      (`.tmp/after_cut/f7/files_w8.txt`, the window-4 ranking beside it)
    - [ ] (b) also splits each of those files past a thousand lines by concern
      (`CONTRIBUTING.md` "Architecture rules", test modules), as a pure move checked by
      `pure_move_check.py`, its own commit before that file's helper extraction. Learned at
      f7b2: the window-8 ranking counts a file's scenario idiom (one call over a listing, then
      one assert, each a named behaviour), which is not copied setup; the briefs target the
      runs of ten shaped lines or more
      - [x] f7b1 `aesthetics_export` `validate/tests.rs` split into eleven files (SWE-2)
      - [x] f7b2 its repeated runs into tables and helpers: 15 of 29 clusters (SWE-2)
    - [ ] (d) every other test module past a thousand lines split the same way, in due time
      (the maintainer, 2026-11-04: all of them eventually, not at once): 117 files at
      `69687460`, the (a) ranking's `*` rows; meanwhile any diff that grows one splits it
      first (RUNBOOK.md design sweep)
- [ ] 0.0.2 the VGL preview (`plans/core/development_plan.md` "The /vg/ League 27 (VGL)").
  Prerequisites: 8.36c and 8.36d (the full-look aesthetics patch, checked in game on PES
  2019), before the PES 2019 rehearsal; the readme's VGL section gains the patch's place in
  the organizer's order and the AATF check of the official save (the plan's paragraph)
  - [ ] 0.0.2-cgu `codegen-units = 1` timed on the AATF check of a 220-team save, both
    profiles (the maintainer: taken only if the app runs faster; DECISIONS "Test pre-releases
    for Update now")
  - [ ] 0.0.2-upd Update now (`plans/core/distribution.md` "Self-update" steps 3, 4 and 6;
    DECISIONS "Update now waits for 0.0.2", "Update now's dialog: what the plan left open"),
    postponed from 0.0.1's f1g9 → verify: ST-UPD-03, 05 and 06 headless, ST-UPD-04 by hand
    - [x] slice 1, 2026-11-11 (`b6954512`, Opus): the updater's core, `updater::install`
      (download, SHA-256, unpack, the swap with its undo)
    - [x] slice 2a, 2026-11-11 (`7e37bb5c`, Opus): the dialog's stages, Update now, the
      download's progress and Cancel, `Update failed`, `Update ready` (ST-UPD-03)
    - [x] slice 2b, 2026-11-12 (`62b92662`, Opus): `Update ready` on the status bar, Restart
      now, `UTW_RELEASES_URL` (ST-UPD-05); the tests for slice 1's five mutation gaps
    - [x] the dialogs and a real restart looked at in the built exe, 2026-11-12, against a
      releases list served from this PC over plain http (`UTW_RELEASES_URL`): every stage
      drawn (the offer with a release's real notes, the download, `Update failed` on a
      checksum that does not match and on a program another window replaced, Cancel,
      `Update ready`, Later and the next start, What's new and the help's Updates chapter).
      The restarted program takes log slot 2 (`utw.2.log`) while the old one is still
      closing, and slot 1 again at the next start alone; the swap leaves no temp folder
      and keeps `old/UTW.exe`. Found and fixed (`a19cc7b5`): the offer back after a Cancel
      or a failure was wrapped to the download's narrower dialog. Captures:
      `4cc-studio-temp/st-upd-04{,b,c}/shot_*.png`; the scripts: `.tmp/0_0_2/upd/rc/`
    - [x] slice 3, 2026-11-12 (`d617bf3a`, Opus): the `Updated to` notice
      (`last_run_version`), the help's Updates and What's new chapters (ST-UPD-06), the
      offer's tooltip naming the app and the version
    - [x] review round 1 (tier 1, SWE-2), 2026-11-12 (`caf88b0f`, Opus): 23 of 43 concerns
      accepted: Update now refused when another window replaced the program, Cancel read
      once more before the swap and disabled once clicked, a check interval of 0 read as a
      minute, the name lookup's timeout, and the endings and dialog moves no test reached
    - [x] review round 2 (SWE-2), 2026-11-12 (`314e2551`, Opus): 7 of 12 concerns accepted:
      the program's stamp compared again before the swap (a second window clicking Update
      now during the first one's download), each field of the stamp and the "old files not
      put back" ending tested, a member's file inside a folder the bundle ships
    - [x] review round 3 (SWE-2), 2026-11-12: one concern, rejected, which ends SWE-2's loop
    - [x] Astra's round, 2026-11-12 (`deafb774`, the lead), ahead of Carbon's loop: 3 of 4
      concerns accepted, which ends Astra's loop: a Cancel over a stalled connection brings
      the offer back when the wait times out, the help's Restart now says the restart is
      dropped whenever a tool asks about unsaved work, and a test reads the command line's
      notice after the command's own lines (ST-CLI-05)
    - [x] SWE-2's round over Astra's fixes, 2026-11-12: 2 of 4 concerns accepted, both in
      the new tests (the notice test read the machine's own settings; no test kept a
      failure a failure after Cancel), which ends the subloop
    - [ ] the review loops' rest: Carbon's loop (its login resets on 2026-11-15)
    - the settings menu's rollback button is not a step: nobody asked for it, and it waits
      for a ruling as `PROPOSALS.md` P51
    - [ ] ST-UPD-04 by hand, on Windows and on Linux. Windows 2026-11-12: the bundle of
      `0.0.2-rc.1` updated itself to `0.0.2-rc.2` and reopened on `Updated to v0.0.2-rc.2
      pre-alpha`, the settings, a file inside `exports/` and a root log file as they were
      (both bundles built by `just release` on the throwaway branch `test/upd-rc`, worktree
      `4cc-studio-temp/rc`, never merged or pushed). Linux: open
  - [ ] 0.0.2-tls the update check's TLS without C (DECISIONS "The shipped build is pure
    Rust by default"): `ring`, `rustls`' crypto provider since f1g9, compiles C and assembly through `cc`,
    the shipped graph's one crate that does. Candidates, each measured before one is proposed
    (maturity and audits, maintenance, licence against `deny.toml`, exe growth, the Windows and
    Linux targets): a pure-Rust `rustls` provider (`rustls-rustcrypto`; `rustls-graviola`, Rust
    with inline assembly); the system's own TLS where it needs no compiled C; keeping `ring` as
    a ruled exception. Writing our own crypto is not a candidate. With it, whether the rule
    allows exceptions and on what terms (the maintainer's ruling per crate, a reason a
    pure-Rust choice falls short, listed where the gate reads them). Then `CONTRIBUTING.md`
    states the rule and a gate refuses a shipped-target crate that builds C and is not a
    listed exception (`cc` in its build graph, `cargo tree -i cc` per release target)
    → verify: the gate red on an unlisted C crate; the update check's tests; a CI dry run's
    Linux binary passes the path check
  - [x] 0.0.2-cmp 2026-11-12 (`5412b447`): the comparator's field rows in three columns,
    Item, Old value and New value, under one head row (SE-GUI-13)
  - [x] 0.0.2-drag 2026-11-12 (`5412b447`): the player list's dragged row floats under the
    pointer, its grab point kept, its place left blank (SE-GUI-69)
  - [ ] 0.0.2-undo an X on each row of the unsaved changes list undoing that change alone,
    if a design for changes spanning many fields holds (the maintainer, 2026-11-03)
  - [ ] 0.0.2-inst a Shift+click on the app's taskbar button opens a new instance, as Windows
    does for most apps (the maintainer, 2026-11-03)
  - [ ] 0.0.2-drift the risk of the settings drifting between two instances open at once
    checked: each saves its own copy, so the last to save may undo the other's change; find
    out what happens today and rule a fix (the maintainer, 2026-11-03)
  - [ ] 0.0.2-clip Ctrl+C/Ctrl+V across instances, if possible: players, formations,
    presets, gameplans copied in one instance and pasted in another (the maintainer,
    2026-11-03)
  - [ ] 0.0.2-statlbl the label of an ability stat the save's version does not have (Tight
    Possession, Aggression, ...) in the same darker colour as a greyed card's label (the
    maintainer, 2026-11-03); drafted first
  - [x] 0.0.2-log 2026-11-12 (`d61750cb`, Opus; tier 2, reviewed with 0.0.2-upgall
    below): `--log-file <FILE>` appends a run's whole output to the file and leaves Info
    findings out of stdout, and the two launchers pass it (ST-CLI-04); the upgrade launcher
    run under a pseudo-console showed the warnings, a refusal's `error:` line and the
    progress line in the window and every line in the log. The bundle's readme names the
    logs, and the organizer's backup of `download/` and the list's upgrade as steps
  - [x] 0.0.2-upgall 2026-11-12 (`88c91de1`, Opus; tier 2): `upgrade`
    takes several exports or `--all <DIR>` in one run (8.22i), each as if alone under its
    naming line, with the tally at the end, and `quick_upgrade.bat` starts the exe once, so
    the progress line counts every export (EU-CLI-11, 13 to 15). The launcher run under a
    pseudo-console over two exports, one refused, drew `[ 0/2 exports ]` first and showed
    both blocks, the `error:` line and `Upgraded: 1. Not upgraded: 1.`
    - [x] review round 1 of both (SWE-2), 2026-11-12 (`be23aec1`, Opus): 12 of 19 concerns
      accepted: a crash's text in the log file, the launcher's last report kept when no run
      follows, the progress line's parts counter per export, and the cases of a run of
      several exports no test reached
    - [x] review round 2 of both (SWE-2), 2026-11-12: 3 concerns, two comments corrected,
      which ends SWE-2's loop
    - [ ] one paid round (Carbon, from 2026-11-15)
  - [x] 0.0.2-cli 2026-11-12 (`6fed2e3e`, Opus; tier 1, its review loops owed): with
    `check_for_updates` on, a tool command starts the check on its own thread and, when a
    newer release has answered by the run's end, prints one line last on stderr and in the
    log file; it never waits and stores nothing (ST-CLI-05,
    `plans/core/distribution.md` "The command line's notice"). Not run: a command against a
    served releases list printing a real offer (the tests cover the line in process)
  - [x] 0.0.2-se19 dropped 2026-11-11 (the maintainer: the tester's save is lost; DECISIONS
    "0.0.2 holds every open 0.0.2 step"): the writer was found sound, and what the edit
    flags gate in game is in `spec/pes_savefile/model.md` "What the player's edit flags gate
    in game"
  - [ ] 0.0.2-poc the first work after 0.0.1's cut (the maintainer, 2026-10-31): 8.36c and
    8.36d, then a proof-of-concept exe for VGL's organizer that
    upgrades and compiles a couple of example teams on PES 2019. 8.36c3 (the `<id>.copy`
    link) waits until after it: no player of the two example teams uses a copy id (the
    maintainer, 2026-11-04). A bundle is fine (the maintainer, 2026-11-04), else the exe
    with the three commands; the gates and the in-game check suffice, 8.36c4's Astra
    review and `mutants-diff` running beside it, their fixes in 0.0.2 → verify: the example teams'
    players show their compiled faces, boots and gloves in Edit mode on PES 2019. The example
    teams (the maintainer): /dbg/ and /4ccg/, the old exports `DBG Aesthetics Export VGL26`
    and `4CCG Aesthetics Export VGL26` in `C:/Data/4cc/Lab/VGL26/`, upgraded
    against the save `C:/Data/4cc/Saves/EDIT00000000_VGL26`. Dry runs 2026-11-05 over the 53
    VGL26 exports (`.tmp/0_0_2/poc/dry_run.py`, `all_run.py`, `tally.py`): their refusals
    and Errors answered by poc1 to poc7
    - [x] 0.0.2-poc1 a custom ID no folder claims becomes the hide ID, with a Warning
      (EU-SET-14, `custom_model_hidden`)
    - [x] 0.0.2-poc2 a texture nobody supplies is a Warning on Fox too (TC-MOD-100,
      TC-TEX-21)
    - [x] 0.0.2-poc3 the 53 exports again with poc1 and poc2's binary: every Error left
      diagnosed (2026-11-05, `4e69334c`, `.tmp/0_0_2/poc/all/`)
    - [x] 0.0.2-poc4 the maintainer's two cleanups: a worn ID's folder found by name, a `.dds`
      beside a `.ftex` resolved (EU-PLY-23, EU-MDL-23)
    - [x] 0.0.2-poc6 what no name pairs: `wearer_by_number`, `wearer_by_order`,
      `custom_model_unpaired` (EU-PLY-24 to 26)
    - [x] 0.0.2-poc5 a model hidden by distance dropped (`model_far_dropped`; EU-MDL-24)
    - [x] 0.0.2-poc7 anti-blur duplicates dropped (EU-MDL-25; TC-MOD-34 withdrawn, restated as
      TC-MOD-101): the 53 VGL26 exports' compile ends clean (2026-11-06)
    - [ ] 0.0.2-poc8 the bundle for VGL's organizer (found 2026-11-08: no step had it): the
      two launchers, `exports/` and `old_exports/` back in `release.py`'s `LAUNCHERS` and
      `EMPTY_FOLDERS`, the readme's VGL section and "pick your PES" step
      restored from `3d7fdc33^`'s `resources/bundle/README.txt`, the section gaining the
      plan's order (`core/development_plan.md` "The /vg/ League 27 (VGL)": the patch the last
      write, the official save's AATF check); `release_test.py`'s
      `test_the_bundle_ships_no_launcher_and_no_export_folder` rewritten, not deleted → verify:
      the staged bundle's file list; the PES 2019 rehearsal (/dbg/ and /4ccg/ through the
      launchers from the extracted bundle, checked in Edit mode). Landed:
      - main's half 2026-11-08 (SWE-2, brief `.tmp/0_0_2/poc/brief_poc8.md`): the two tuples,
        the readme, the CHANGELOG's `### New`; the version 0.0.2-poc.1 (DECISIONS "the PoC
        bundle is 0.0.2-poc.1"); Astra A-poc8 r1 and r2 over the readme, the loop ended
      - branch `vgl27` (named `poc/0.0.2-poc.1` until 2026-11-11), built 2026-11-08 and
        rebuilt with each later fix (earlier builds in `target/dist_prev_*`)
      - the rehearsal on PES 2019 2026-11-09 (`.tmp/0_0_2/ingame/rehearsal/findings.md`): both
        teams upgrade, the compile deploys and patches the save, 8 players show their own
        faces, bodies, boots and gloves
      - the official save's write path (A-savepath r1 and r2, S-savepath r1 and r2, loops
        ended): a written name sets the edited-player flag (SE-PAT-13); an in-place
        `apply-patch` that applies no player writes nothing (SE-PAT-12)
      - the VGL27 Friendlies through the bundle 2026-11-09 (39 teams): 37 upgrade and compile
        clean for PES 2019; /wuwa/'s note colour `##8C96C5` refuses its upgrade, which stays
        (DECISIONS 2026-11-09)
      Open: the hand-over to VGL's organizer (the maintainer's); the Linux tarball now also
      carries the two `.bat` files (`expected_entries` has no platform gate), to settle before
      0.0.2
  - [x] 0.0.2-poc9 a legacy kit folder's file names lose their Studio tokens (DECISIONS
    2026-11-09, two entries; EU-MDL-28): an author's token written `kit_tex` (`975e68a7`)
  - [x] 0.0.2-poc10 the embedded teams list carries VGL27's allocation (DECISIONS "the
    embedded list carries VGL27's allocation"; done 2026-11-09, `6268280c`): 781–834
  - [x] 0.0.2-poc11 the upgrader writes no team ID and needs no teams list (DECISIONS
    2026-11-09 "the Studio format names no team ID"; done 2026-11-09): the save's team found
    by name, `gk_kitN` for a keeper's `u0XXXgN`
  - [ ] 0.0.2-poc12 no ambiguous Common spelling (the maintainer, 2026-11-09, ruling on
    poc11's *lead* choices after Astra's round). Rulings in DECISIONS 2026-11-09 "no ambiguous
    Common spelling" and "the kit stand-in is written `team_kit`", 2026-11-11 for slice 4: the
    team's Common is `team_common_folder/…` (long form
    `model/character/uniform/common/team_folder/…`), a game file below the uniform Common
    `common_folder/<name>`, a reference starting `common/` refused; an FMDL texture directory
    takes the same two short forms; the kit stand-in is `team_kit`/`team_gk_kit` (the
    compiler writes `dummy_kit…` in every output); every legacy spelling is refused by the
    compiler and respelled only by the upgrader
    - [x] spec: DECISIONS 2026-11-09 "no ambiguous Common spelling"; TC-XML-21, 22,
      TC-MOD-103, EU-MDL-34 to 39, EU-REF-14, EU-MF-10
    - [x] fixtures (lead, `cf80cf48`): the studio tracer's four FMDLs, `deep/boots_far.fmdl`
      and the three archives built from them
    - [x] the code (slice 1): `team_common_folder/`, SWE-2, `.tmp/0_0_2/poc12/brief.md` and
      `rework1.md`
    - [x] slice 2: the FMDL short forms (TC-MOD-104, 105, EU-MDL-40), `brief2.md`
    - [x] slice 3: `team_kit` (TC-MOD-106, TC-CMN-36, EU-MDL-41, 42), `brief3.md`, `brief3b.md`
    - [x] slice 3b: FoxDen's `02_kitswappers.lua` gets back the `dummy_kit`/`dummy_gk_kit`
      swapper; done, uncommitted in FoxDen's repository (the official scripts are the
      maintainer's)
    - [x] the PoC rebuilt 2026-11-09 (`f815f1b1`, again `19a7fb3f`): 36 of 39 Friendlies
      upgrade, no old spelling left
    - [x] slice 4: the legacy kit token refused in an export's file names and paths, naming
      the upgrader (`kit_token_old`; TC-STR-22, TC-MOD-107; `9382b068`, merged `abe8e0ff`)
      - [ ] review rounds. Astra A-poc12s4: 2 accepted (names below a player or shared folder
        and in `Common/` only; a bare `.mtl` name is the folder's own; DECISIONS 2026-11-11),
        its loop ended. SWE-2 S-poc12s4 r1 (two parts): part a 2 nits (`aa38922c`); part b 5
        accepted, the check wider than the upgrader's respelling (`Faces/`, collars,
        game-supplied names; the catalog row's text): rework by Opus sidekick `sk_kto`
        (`4cded58e`, merge `46fc9622`; DECISIONS 2026-11-11), then SWE-2 r2: 0 accepted (the
        paths it raised are in none of the 15,199 FMDLs under `C:/Data/4cc`; a doc nit
        fixed), its loop ended; Carbon after 11-15
  - [ ] 0.0.2-poc13 VGL27 keeps the managers' boots and gloves IDs (the maintainer,
    2026-11-09, after the PoC): managers keep the old format and Red for their own tests, so
    an official DLC whose boots/gloves IDs differ from what they sent (Studio's slot-based
    40-per-team block, 3745 for team 792's slot 05, against the old exports' own `k`/`g`
    IDs in a 25-per-team block) breaks their test CPKs against the official save.
    - [x] poc13a spec (2026-11-10): `plans/team_compiler/pipeline.md` "Legacy boots and
      gloves IDs (VGL27)", the block table `crates/tools/team_compiler/data/legacy_boots_blocks.txt`
    - [x] poc13b the upgrader's `legacy_ids.toml` (2026-11-10, SWE-2, landed first time)
    - [x] poc13a2 re-spec (2026-11-10 evening; DECISIONS 2026-11-10): legacy mode is the
      upgrader's `--legacy-ids`, the compiler reads the file's presence
    - [x] poc13c the upgrader's `--legacy-ids` (2026-11-10, SWE-2, landed first time;
      EU-SET-15 to 17)
    - [x] poc13d the compiler's legacy packing (2026-11-10, SWE-2, landed first time;
      TC-LEG-01/02)
    - [x] poc13e the block check, brought into the PoC (2026-11-11, SWE-2, landed first time;
      TC-LEG-03/04): /utg/'s wrong IDs refused (DECISIONS 2026-11-11)
    - [x] poc13f a legacy merge that clashes keeps the folder shared, packed under its
      old ID (2026-11-11, SWE-2, landed first time; TC-LEG-05, EU-SET-17)
    - [x] poc13g an out-of-block old ID reported once, on the folder that would pack it
      (2026-11-11, SWE-2, landed first time; DECISIONS 2026-11-11)
    - [ ] the maintainer's asks for the PoC if they fit (2026-11-10), each checked first.
      Bats keep their `quick_` names (the maintainer, 11-10)
      - [x] why /nepgen/'s `SWITCH EXCLUSIVE` became a shared gloves folder with one wearer
        (answered 2026-11-11, no change: its textures differ from the boots folder's)
      - [x] poc14 (SWE-2, two reworks): `dpfilelist_outdated` and `cpk_name_unlisted` Fatal
        at the preflight (TC-DEP-05, 08); the bundle's commented `data/settings.toml`
      - [x] 0.0.2-poc.3 built (2026-11-11; branch `1a731a94`, archive SHA-256 `991cc670…`,
        the exe `v0.0.2-poc.3`) in `4cc-studio-temp/poc/target/dist`. Look: Pending looks
    - [x] the embedded `dpfl`'s `4cc_99_test` renamed `4cc_90_test` on the PoC branch alone
      (`301c8b46`); main keeps `4cc_99_test`
    - [x] 0.0.2-poc.2 rebuilt with poc13e/f (2026-11-11; branch `9eafbfad`, archive SHA-256
      `1cabaa13…`): the Friendlies with `--legacy-ids` upgrade 36 of 39 and compile with only
      the expected Errors. Not checked in game (look: Pending looks)
  - [x] 0.0.2-ux1 the CLI's progress line and finding times (the maintainer, 2026-11-10;
    DECISIONS 2026-11-10): ux1a and ux1b below
    - [x] ux1a the upgrader's line (EU-CLI-12): `PipelineEvent::ExportNamed` names the output
      half, the parts are the files written into staging (no test for a failed read's part)
    - [x] ux1b `--timestamps` on `check` and `compile` (TC-CLI-13): the time on the event
      envelope (`PipelineEventEnvelope.made_at`)
    - [ ] review rounds over ux1a + ux1b (`03ce3612..2ea6e27c`): the Carbon loop waits for
      `home`'s 2026-11-15 reset. Astra A-ux1: 2 accepted (a test of `write_batches`' finish
      times, the plan's `duplicate_path` timing), its loop ended. SWE-2 S-ux1 r1 (three
      parts): 1 accepted (TC-CLI-13's `check` test asserts no stderr), its loop ended
  - [ ] 0.0.2-chk library parity against checkpoints, not Red (the maintainer, 2026-11-09:
    parity with Red was proven once the core was built; divergence is expected since, and
    accounting for it at every run grows costlier). *Lead*, the shape (veto any):
    - the p11 run (`3c512628`, every line attributed) is the last measured against Red; its
      commit's outputs become checkpoint 1: per job, the CPK entries' names and hashes, kept
      under `.tmp/library_parity/checkpoints/<tag>/` (machine-local, as the inputs are)
    - a phase close reruns the upgrade census (`regress_check.py` already compares against the
      previous tag) and our compile alone, diffed entry by entry against the last checkpoint;
      each changed entry is traced to a commit of the phase (intended: the checkpoint is
      re-pinned, the diff's summary in the commit) or is a defect (a worklog step). Red is not
      run, and `classes.md`'s thirty-odd Red classes are not re-attributed
    - an entry is diagnosed by rebuilding the checkpoint's commit in a worktree and rerunning
      that job, so the checkpoint keeps hashes, not GBs of output
    - Red comes back only for a question about what the old tool did (a member's report, a
      newly ported area's first build), at a checkpoint of its own
    - the committed tracer cases (`tests/parity.rs` against `red/`) stay as they are: small,
      run in CI, and `parity.rs` changed in 4 commits since 2026-10-20 (22 in all); they become checkpoint
      goldens of our own output the first time keeping one costs more than re-pinning
    Lands in `spec/team_compiler/testing.md` (the library paragraph's rationale),
    `scripts/library_parity/` (a `checkpoint.py` writing and diffing manifests, the README's
    rerun order), RUNBOOK.md "Closing a phase" and AGENTS.md's one sentence. After poc12 and
    the PoC; until then no Red run is due (the next is a phase close's)
  - [x] 0.0.2-acc withdrawn scenarios out of the way (the maintainer, 2026-11-09): whole
    withdrawn areas in `docs/spec/withdrawn_ids.md`, runs of withdrawn IDs as range lines
  - [x] 0.0.2a JPEG XL sources (DECISIONS 2026-10-30; TC-TEX-22): `jxl` =0.8.0 decoded
    directly, built without AVX-512 (DECISIONS 2026-11-11); Carbon after 11-15
  - [x] 0.0.2-ids the team's manager id (+4, u32) and home stadium id (+10, u16) on PES 17
    and 18 (DECISIONS 2026-11-11; `8cb837d6`, merge `395e734c`); Carbon after 11-15
  - [x] 0.0.2-st15 the home stadium id on PES 2015 and 2016 (DECISIONS 2026-11-11;
    `9975abe6`, merge `9ae8e390`): the u16 at +6, shown live. Not checked in game (look:
    Pending looks); Carbon after 11-15
  - [ ] 0.0.2b no BC7 encoding until the GPU encoder (`spec/libs/dds_convert.md` "Target codecs
    and CPU encoder"; DECISIONS 2026-10-30): PES 19–21 encode as
    PES 15–18, BC7 sources kept; the Team compiler's tests and goldens that expect BC7
    follow, and the library parity against Red re-run (Red ships BC3). Pulled into the
    0.0.2-poc bundle (the maintainer, 2026-11-06). With it, uncompressed sources stay
    uncompressed (`spec/libs/dds_convert.md` "In-process DDS conversion (replaces texconv)";
    DECISIONS 2026-11-06), a pre-Fox main kit texture excepted. → verify also: the
    maintainer's PES 21 test compile (`.tmp/release_0_0_1/0_0_1/manual_run2`, about 1:30
    with BC7) re-timed; its 45 DXT5 sources pass through either way, and its three
    uncompressed 4096 × 4096 sources (`kit.dds` twice, one `shirt.dds`) were the only BC7
    encodes and now encode nothing, so a run still slow after this step sends the profiling
    of 0.0.2d elsewhere. Landed:
    - the code (SWE-2, the commit "no BC7 encoding; uncompressed sources stay uncompressed"),
      the spec moved (the deferred GPU part stays in the plan); `just gpu-tests` not run
    - re-timed 2026-11-06 (`.tmp/0_0_2/poc/retime.py`): 0.8 s wall where BC7 took about 1:30
    - in game on PES 2019 2026-11-07 (`.tmp/0_0_2/ingame/poc/findings.md`, `s42`-`s44`): the
      4096 `kit.dds` kept uncompressed loads and Edit mode draws the shirt; the seasonal
      logos not seen (QUESTIONS "Seasonal logos on an uncompressed kit"; look: Pending looks)
    - Astra A-nobc7 r1 (3 accepted) and its rework 2026-11-08; SWE-2 S-nobc7 r1 2026-11-11
      (3 accepted, among them `texture_not_div4` skipping a pre-Fox texture written
      uncompressed, DECISIONS 2026-11-11; look: Pending looks), its loop ended, Carbon after
      11-15
    - the same run's PoC look passed: /dbg/ and /4ccg/ upgrade and compile clean (about 2 s
      for both), and the 8 players checked show their own faces and bodies
  - [ ] 0.0.2c the illegal formation check (`plans/aatf_rules.md` "The illegal formation
    check (0.0.2)"; DECISIONS 2026-10-31, P46 accepted; 2026-11-12 "An illegal formation is
    one only a tool outside PES can make"): what the stock screens can produce, read from
    PES 2017's exe, then looked at in game on `PES2017.exe.old5`; then the plan's rule, its
    Acceptance, and the slice
    - [x] the first reading, 2026-11-12 (Opus, `.tmp/0_0_2/formation/research/report.md`):
      the Whack Patch's 25 runs read; no area refusal among them, a match-time rewrite of
      the line counts and the drag's re-pick switched off
    - [x] the second reading, 2026-11-12 (Opus, `.tmp/0_0_2/formation/research/report2.md`):
      the drop's offers per spot (two area tables, PES 2017's first equal to
      `team_widgets`'), the menu, the swap, the per-set positions; the test's four clauses,
      no line counts; a formation breaking each clause
    - [x] six defenders and seven midfielders tried in the stock screen (the maintainer,
      2026-11-12, two captures): the drop is refused with a no-entry sign, so the screens
      do count lines and the second reading's "no line count" is wrong
    - [x] the limits, from the maintainer's looks, 2026-11-12: forwards 1 to 5, midfielders
      1 to 6, defenders 2 to 5, each a refused drop; where the exe decides it is unread
      and not needed for the check
    - [x] the sixth clause (the maintainer, 2026-11-12: overlapping starters are refused,
      at the distance the editor's pitch already swaps at): two outfield starters no
      more than 8 units apart across and 2 up or down, each inside the other's marker box
    - [ ] the game's own formations under the six clauses: each team's in PES 2017's data
      and the preset list, then the cup saves, tallied by clause
    - [ ] each clause looked at in game under the stock exe (Pending looks)
  - [ ] 0.0.2d profile a compile against Red, after 0.0.2b (the maintainer, 2026-11-06: one
    PES 2021 export, `egg Midcup Big` in `.tmp/release_0_0_1/0_0_1/manual_run2/`, took about
    1:30 in a console where Red takes under half that, and its progress line showed no parts
    for its first 40 seconds) → measure on an idle machine (no mutation run, no cargo) with a
    release build: Red's and ours on the same export, wall time, three runs, the median; our
    time split by stage (validation, planning, each task kind, the CPK write) with a
    sampling profiler; each stage that costs more than Red's whole run gets a worklog step
- [ ] after 0.0.2: Live AATF in a native window of its own (the maintainer, 2026-11-01; the
  embedded one is cv28): an egui viewport (`show_viewport_immediate`, the embedded window as
  its fallback where the platform has no second window), not always-on-top (desktop-wide, so
  it would cover other programs; a member moves it off Studio's window instead) → first
  measure what a second viewport costs per frame on the maintainer's machine, and what focus
  and the keyboard shortcuts do across the two windows; then the plan's paragraph, its
  scenario and a slice
- [ ] after 0.0.2: the pitch's name scroll delays as the game's (the maintainer, 2026-11-01;
  `plans/save_editor.md` "Tactics editor", the marker bullet's scrolling name: the editor
  starts every name together, the game staggers them by a rule not established) → first measure in PES
  2017 on captures of a whole lineup whose names overflow, over several 13.5 s periods: each
  name's delay from the team screen opening to its first scroll, against its slot, its
  position and its width; PES 2019 the same if 2017's rule is unclear; then the plan's
  sentence, a scenario and a slice
- [ ] after 0.0.2: the pitch's portraits from the team's aesthetics export first (the
  maintainer, 2026-11-01): before the PES folder's `download/` CPKs (`plans/save_editor.md`
  "Tactics editor", the Portraits bullet), the Save editor looks in the exports folder
  (`exports_folder_path`) for the shown team's aesthetics export and takes every portrait it
  holds from there; `download/` stays the fallback for each player the export has none for,
  and the placeholder after both → first read how the Team compiler finds an export's team
  (`aesthetics_export` `resolve_identity` over the teams list) and which player a portrait
  belongs to (`spec/aesthetics_export/player_folders.md` "Savefile identity" and
  "Portraits": a player folder's `portrait.*` or the standalone `Portraits/NN.*`, any
  accepted format, a conflict between the two), and reuse both rather than a second reading.
  While any shown portrait came from the export, a Refresh button, a symbol button (the
  suite's icon set, a tooltip naming it), stands at the pitch's top right and reads the
  export's portraits again, so a member iterating on a portrait sees each change without
  reopening the save (the maintainer, 2026-11-01). Then the draft (the button's place
  against the Set PES path button's bottom right), the plan's paragraph (what a malformed
  or ambiguous export does, off the UI thread as the CPK read is), its scenarios, and a slice

### Phase 10 — Music tools

The phase's steps are itemized when Phase 9 closes; its format crate was built ahead of it:

- [x] 10.0 `libs/music_export`: the `.4ccm` format crate (`music_player.md` "The `.4ccm` format")
  (10.0a-10.0e: model, parser, writer, evaluation, selection, resolution; `just mutants
  music_export` 0 missed; DECISIONS 2026-10-18)
- [x] 10.0f the legacy-behavior audit against Rigdio v2.2.0 (`RUNBOOK.md` "Before a tool
  phase"; `.tmp/phase_10/10_0f/rigdio_audit.md`): done 2026-10-21 (DECISIONS 2026-10-21, "A
  `.4ccm` reads as Rigdio reads it")
- [x] 10.0g the crate to the audit's rulings: in-order conditions, `once` spent when reached,
  `TeamLists` with one shared `goal` fallback per team. Done 2026-10-21 (`dff1548f`,
  `5cf02863`)
- [x] 10.0h the Music export editor's legacy-behavior audit against RigDJ v1.16.0
  (`.tmp/phase_10/10_0h/rigdj_audit.md`): 42 rows ruled into the plan and four DECISIONS
  entries; PROPOSALS P40
- [x] 10.0i 10.0f's part 2, Rigdio's player controls against `plans/music_player.md`. Done
  2026-10-27 (DECISIONS "The Music player's controls, ruled against Rigdio v2.2.0")

### Phase 12 — Kit config editor

The phase's steps are itemized when it opens; prepared ahead:

- [x] 12.0a the tool's GUI plan against Kit Manager's recovered behavior. Done 2026-10-27
  (`.tmp/phase_12/12_0a/kitmanager_audit.md`; DECISIONS "The Kit config editor saves on Save,
  and the rest of Kit Manager's audit"; PROPOSALS P41-P42)
- [x] 12.0b in game (PES 2016): a long-sleeve badge on undershirt-only long sleeves (0x02 =
  0xBB) draws no badge on the forearm; stopped 2026-10-27, the editor grays the four fields

### Phase 13 — Refs arranger

The phase's steps are itemized when it opens; prepared ahead:

- [x] 13.0a the plan's readiness audit: `plans/refs_arranger.md` against what it relies on
  (`.tmp/phase_13/13_0a/refs_audit.md`; DECISIONS "The Refs arranger's plan checked against
  FoxDen and the export library")
- [ ] 13.0b in game, when the phase opens (PES 2021 with FoxDen copied in for the test and
  removed after, pad and OBS): which of positions 0..4 is the main referee, the linesmen and
  the other two (`plans/refs_arranger.md` "Open questions"); FoxDen's files are not in any
  game folder today

### Phase 14 — Balls compiler

The phase's steps are itemized when it opens; prepared ahead:

- [x] 14.0a the legacy-behavior audit of the 4cc balls compiler 2.03
  (`.tmp/phase_14/14_0a/balls_audit.md`): 49 rows ruled into the plan, five DECISIONS entries

### Phase 16 — Polish and distribution

Steps are itemized when Phase 15 closes; three are fixed already:

- [x] 16.w the release exe's platform floor and size (maintainer, 2026-10-15; DECISIONS
  2026-10-15): Windows 10 x64 and Linux, `+crt-static`, `rhai` `only_i64` (16.w1-16.w3)

- [ ] 16.x `dds_convert` GPU BC7 (deferred from 2.5b to Phase 4, decision entry 2026-09-21, then
  past Release 0.1.0, decision entry 2026-09-28; spec `libs/README.md` "First-release desktop GPU
  BC7"): `block_compression`'s wgpu backend on a Vulkan/Metal device, CPU fallback with the
  fallback reason reported, cold pipeline creation and upload/readback measured first, then the
  texture step's bounded batches → verify: GPU and CPU outputs decode within the same tolerance
  on the `dds_convert` fixtures; the fallback path exercised by forcing no adapter
  - [x] 16.x a `dds_convert`'s half, built ahead and unwired (2026-10-19, DECISIONS "the GPU
    BC7 encoder is built ahead of 0.1.0"): `gpu::GpuEncoder`, `just gpu-tests`
  - [ ] 16.x b the Team compiler's texture step on the encoder, after 0.1.0: the CPU starts at
    once and the GPU joins when its device is ready, started when the run has BC7 work
    (`plans/libs/README.md` "First-release desktop GPU BC7"); start it after the window's
    renderer is up, a failed start this run's CPU fallback with its reason in the report
    (`.tmp/phase_16/16x/findings.md`, the transient adapter failure seen under load). The system log
    (checked 2026-10-21) has no driver reset at any of those failures (04:07, 04:15 and later
    on 2026-10-19); that day's one reset, 03:16:41, which Adrenalin reported twice (two
    `LiveKernelEvent` reports), was the probe's single-submission DX12 run of an 8192 chain.
    So the adapter blip is not a timeout: most likely the Vulkan loader with several processes
    opening it at once, unexplained
  - [ ] 16.x c the BC7 speed probe, after all of DBC's work, the GPU encoding study with it once
    0.0.1 is out (the maintainer, 2026-10-30, after a 2048² face's CPU BC7 chain took 31.7 s of
    CPU against 128 ms for DXT5, `.tmp/release_0_0_2/jxl_bench/findings.md`; VGL 27's PES 2019 compiles are
    BC7 throughout) (the maintainer, 2026-10-21;
    `plans/libs/README.md` "The workload the speed is judged on"): on the RX 9060 XT and its
    CPU, real kits (4096² and 8192²) and faces, time and PSNR for `block_compression`'s BC7
    presets (`ultra_fast` to `basic`) on GPU and CPU, `intel_tex_2`'s on CPU, BC3 as the
    quality floor; a team's ~30 chains in flight on the GPU at once against one at a time; the
    share of flat 4×4 blocks in real kits and faces, and the time a direct write of them saves
    → verify: a table per encoder and preset, and a recommendation: the preset and CPU encoder
    to ship, or the BC3 fallback, with a decision entry; our own encoder only if every lever
    falls short

- [ ] 16.y a settings panel where a member customizes the palette (the maintainer,
  2026-10-29: after Release 0.2.0), on ov11a's `studio_core::theme::Palette` (`plans/core/gui.md`,
  the Dark/light theme row): each named colour editable with the suite's colour picker,
  saved in the settings, reset per colour and whole; `theme::palette` then returns the
  member's edited copy, the built-in dark and light instances its defaults. To plan first:
  whether egui's own `Visuals` colours (text, widget fills, the accent) are editable too,
  or the palette grows fields that set them; a draft, then the plan section

### Phase 20 — Ruleset editor (post-release)

Opened 2026-10-23, pulled forward for DBC (`plans/core/development_plan.md` "Divegrass Before
Christmas"); itemized as 20.3 onward below. Its GUI was drafted ahead of it, at the maintainer's
request (2026-10-14), to show the members who offered to help shape the ruleset format:

- [x] 20.0 The Ruleset editor's GUI draft (`docs/prototypes/ruleset_editor.html`), with the
  coverage tables of Clouds' proposal and the DBC7 rules in `aatf_rules.md` (20.0a-20.0e;
  DECISIONS 2026-10-20, 2026-10-21)
  - [x] 20.0c the draft brought up to date (maintainer's request, 2026-10-15): the five other
    embedded rulesets as working templates and a sixth, "Clouds' proposal, every item (from
    VGL26)". Done 2026-10-15 (SWE-2)
- [x] 20.1 Autumn 26 against the official 4ccEditor's Autumn 26 edition: six differences
  ruled against the wiki's `Appendix:Rules` (DECISIONS "Autumn 26 follows the wiki's written
  rules…"). Done 2026-10-21
- [x] 20.2 Autumn 26 takes the 4cc organizer's answers (DECISIONS 2026-10-22). Done
  2026-10-22
- [x] 20.3 the Acceptance section (lead, 2026-10-23): `plans/ruleset_editor.md` "Acceptance",
  logic version 1 frozen at the 0.0.1 cut (DECISIONS 2026-10-23). Done 2026-10-23
- [x] 20.4 `libs/aatf`'s writer (`src/writer/`, `Ruleset::to_data_block`): every ruleset
  reads back equal from what it writes. Done 2026-10-23
- [x] 20.5 the tool crate's egui-free core (`plans/ruleset_editor.md` "Crate layout"):
  `document.rs`, `summary.rs`, the CLI's `validate`, `update-logic`, `summary`. Done
  2026-10-23
- Set aside on 2026-10-24 (the maintainer: not in 0.0.1, after Release 0.1.0;
  `plans/core/development_plan.md` "Phase 20"): 20.6a2a, in flight then, is reviewed and
  landed, and the steps below wait until after 0.1.0.
- [ ] 20.6 the view (`plans/ruleset_editor.md` "The editor", the draft
  `docs/prototypes/tools/ruleset_editor.js`), as slices:
  - 20.6a the window's frame and the plain forms, as subslices:
    - [x] 20.6a1 `settings.rs`, the templates and Blank, undo, the start page, the toolbar,
      the rail, the preview, General, Custom checks and Summary (RE-GUI-01, 02, 13 to 15,
      RE-SUM-01, RE-FILE-01 to 03, 05, 07). Done 2026-10-23, its review reworks to 2026-10-24
    - 20.6a2 Tiers with the medal systems, Cards, Universal rules with the team settings,
      Suggestions (RE-GUI-03 to 06, 08, RE-FILE-04, 06), as two subslices (one is past a
      review's size):
      - [x] 20.6a2a Tiers and the medal systems, the tier expander's per-stat targets and free
        cards (RE-GUI-03, RE-GUI-04's tiers half, 05, 06, RE-FILE-04, 06). Done 2026-10-24,
        reviewed (`74dcf473`)
      - [ ] 20.6a2b Cards, the team settings, Suggestions (RE-GUI-08)
      - [x] 20.6a2c Universal rules without the team settings (RE-GUI-20), for the illegal
        formations checkbox (cv14; the maintainer, 2026-10-31: "a very simple page", the
        team settings may wait): `sections/universal.rs`, after cv14a lands
  - 20.6b the condition editor, the effects editor, Heights and Specials (RE-GUI-07, 09 to 12,
    and RE-GUI-04's height-systems half, whose test joins the tiers half's)
- [ ] 20.7 Randomize (`plans/ruleset_editor.md` "Randomize"): `random/`, the start page's and
  toolbar's entries, the CLI's `random` (RE-RND, RE-CLI-04) → verify: 1,000 seeds per preset,
  and the share of seeds that needed a redraw measured and reported
- [ ] 20.8 the help chapter, and the Ruleset editor active on the shell's sidebar
- [ ] 20.9 converge (RUNBOOK.md "Closing a phase"), RE-GUI-16 checked by hand

---

## Pending looks

Checks only the game or the built window can make, one line each: what to look at, the PES
version, the step or decision it belongs to, and who can make it (`harness`: the lead's in-game
or window-capture harness; `maintainer`). Ordered by the cost of a wrong result, game-facing
writes first. A look leaves this list when its finding or `manual:` line lands in its step.

- The aesthetics patch on an unstripped save, the compiled look in Edit mode (boots and gloves,
  a base-copied player, one with clear flags), the FoxDen gloves issue, SE-GUI-34 in patch
  form; PES 2019, then 2021 and 2017; 8.36d; harness
- The Friendlies compiled with `--legacy-ids` (`4cc_90_test.cpk` and the patched save), boots
  and gloves under their old IDs in Edit mode; PES 2019; 0.0.2-poc13's poc.2; harness
- The 0.0.2-poc.4 bundle from a fresh extract: its launchers' upgrade and compile, the teams in
  Edit mode; PES 2019; 0.0.2-poc; maintainer (theirs, 2026-11-11)
- The illegal formation test's clauses under PES 2017's stock exe (`PES2017.exe.old5`), a
  4-4-2 changed in one starter each time by a tool (the base formation is in section 6 of
  `.tmp/0_0_2/formation/research/report2.md`): the keeper at (52, 1); the LB at (2, 10); a CF at (52, 10); a second LMF at
  (14, 30): does Team Sheet show each as stored, move the starter or change his position,
  and what does the save the game then writes hold; PES 2017; 0.0.2c; harness
- A team's home stadium id (the u16 at +6) edited by the Save editor shows as that stadium in
  game; PES 2015 and 2016; 0.0.2-st15 (DECISIONS 2026-11-11); harness
- A pre-Fox texture written uncompressed with a side no multiple of 4 (a 1030 × 513 hair part)
  loads and draws; PES 2017; 0.0.2b (DECISIONS 2026-11-11 "texture_not_div4 holds only a
  pre-Fox texture the compile block-compresses"); harness
- Substitution's "Very early" in the Game Plan's Automatic settings; PES 2018, 2019 and 2020;
  0.0.1y's y7c (`QUESTIONS.md` "After the release"); harness
- A motion and a face edit in Edit mode on a save with a compiled team's patch: which keys the
  record then holds; PES 2021; 8.23a; harness
- SE-GUI-29: each Texport the panel exports, imported by the game's team import; PES 2018 to
  2021; 8.9k's k5d; harness
- Whether a team-file import writes the DLC team bin's name into the save, as on PES 2017; PES
  2015, 2016, 2018 to 2021; 8.9k's k5g; harness
- The drag rule's mapping of kicking power/swerve and body control/physical contact, and the B
  bonus in the overall; PES 2017; 0.0.1y's y4; maintainer (a save where those stats differ)
- The seasonal logos on an uncompressed 4096 main kit; PES 2019; 0.0.2b (`QUESTIONS.md`
  "Seasonal logos on an uncompressed kit"); harness
- Each version's stat names, read off its Edit screen; PES 2015, 2016, 2018 to 2021; 8.29d's
  d3b; harness
- What a zero emblem in a Texport the panel wrote does in Select Team; PES 2017; 8.9k's k5f;
  harness
- Each launcher prints in its console and its `pause` waits, on 0.0.2's bundle; any PES;
  0.0.1l; harness
- The built window at 1280 × 800 on the FHD monitor, the Tactics tab in three columns; any PES;
  8.15; harness
- A lineup row dragged from its name in the built window; any PES; 0.0.1f's f1e; maintainer
- The Fox ownership line's Apply link in the built window; PES 2018 to 2021; 8.38; harness
- The Save editor's eight help chapters and the two core ones read in the built window; any
  PES; 8.29e's e1; harness
- The Live AATF pitch's `#FF8F00` warning cross, drawn for a Warning-severity position mark (no
  real save under 4cc Autumn 26 raises one); any PES; 8.9m's m4; harness
- ov7b's Import and × dialogs, ov10's pitch drag and ov12's popover steppers in the built
  window; any PES; 0.0.1ov; harness
- The players' name colour codes in the light theme; any PES; 0.0.1ov's ov11b; maintainer

---

## Issues

Bugs, unexpected behavior, things to revisit. `open` / `resolved (date)`. Resolved issues are
pruned when their phase closes; they stay in git history. A question only the maintainer
can answer is not an issue: it goes to `docs/QUESTIONS.md`, and an issue that waits on one
points there.

- open 2026-11-11 (the worklog's collapse) — What the collapse left for the lead, from its
  report (`.tmp/sidelead/sk_wl.report.md`): (1) the closed issues' in-game results, Tests 2,
  4, 4b, 4c and 5 (the BootsList and GloveList rows, the referee collars on PES 17 and 21),
  are in git history only (`git show 85d6d966:docs/WORKLOG.md`): check the spec holds each,
  and move in what it lacks; (2) five rulings recorded only in the worklog get their
  DECISIONS entries (the PoC branch's `4cc_90_test`, 0.0.2-chk's checkpoints, 0.0.2b in the
  PoC bundle, the bats' `quick_` names, 0.0.2-acc's archive rule); (3) states to settle:
  open children under closed parents (ux1, poc12 slice 4, 4c's r2, f4a-c), items open though
  recorded done (k5b, k5b2, 8.36c4, f1e to f1g, poc13's asks), closed items still naming
  something open (8.36b, 20.6a1), the tags 8.9m and 0.0.1g each used twice.
- open 2026-11-12 (`just mutants-diff 33fe81f0`, `.tmp/process/mutants_1112h.log`) — the
  VPS's disk filled during `save_editor`'s remote half ("No space left on device"), which
  stopped with one of its 16 mutants unrun, although the guard had passed: 26.8 GiB in the
  project's folder (budget 30) and 19.5 GiB free (floor 12). So a `save_editor` half needs
  more than the 10 GiB `REMOTE_FREE_FLOOR` allows for (`scripts/mutation/mutants.py`), or
  the host's own use grew meanwhile. The clone's build cache was then 28 GB of the folder;
  `remote_clean_target.sh` removed it (47 GB free, the disk at 70%), so the next
  `just remote` builds cold. To do: measure a `save_editor` half's peak in
  `~/studio-mutants/tmp` and set the floor from it (a vital script: its SWE-2 rounds). The
  unrun mutant (`comparator.rs`, `column_offsets -> [0.0; 3]`) was applied by hand here:
  two comparator tests fail on it.
- open 2026-11-12 (the readability findings): `spec/aatf_rules.md`'s sentence that a VGL25
  goalkeeper rated A at GK is still told of an unused A position comes from reading
  `crates/libs/aatf/logic/logic.rhai` (`a_positions` makes no goalkeeper exception): no
  fixture asserts that suggestion. A test with the ruleset's next change.
- open 2026-11-12 (the readability findings, item 9) — the comments of a kit's generated
  `config.toml` (`crates/libs/kit_config/src/toml_form.rs`, `to_toml`) give the old editor's
  ranges, which the game's fields can exceed (`x  # 0-14` in a field of 0-15). The two that
  stock configs do exceed are corrected (the collar's, per PES version, and the chest
  number's `y`); whether each of the others follows the old editor or its field is settled
  with the Kit config editor's plan (Phase 12), from what the game draws past the editor's
  limit.
- open 2026-11-12 (0.0.2-log, `.tmp/0_0_2/poc5/redirect_probe.bat`) — started from `cmd` or a
  bat, the exe's output can be neither redirected nor piped: `UTW.exe --version > file`,
  `| findstr` and `2>&1` each received 0 bytes, where Git Bash and a Rust `Command`, which
  hand the handles over themselves, receive every line. So `save-editor diff ... > report.txt`
  typed at a `cmd` prompt writes an empty file. `plans/core/architecture.md` "Windows
  subsystem" names only the interleaved prompt as the arrangement's cost. Unmeasured:
  PowerShell. `--log-file` covers a tool's findings; a `diff` or `--json` to a file from
  `cmd` has no route yet (a console-subsystem shim beside the exe is the usual one).
- open 2026-11-12 (0.0.2-drag) — the player list draws only the rows in view, so a row
  dragged while the wheel scrolls its place out of view stops floating until the release
  (the drop still lands). Harmless on a team's list, which fits the window; a fix would
  paint the float from outside the rows.
- open 2026-11-12 (0.0.2-upd, the look in the built exe) — a dialog whose button row is
  wider than its body draws the buttons out past the body's left edge
  (`widgets::modal_buttons` lays them out from the right in the body's width). Seen on the
  update offer with one-line notes and versions long enough to wrap, where the notes' box
  also hugs its text instead of spanning the dialog as the draft's does; a release's real
  notes fill the width and show neither. A fix would measure the row and widen the body.
  `Update ready`'s sentence now wraps with two words on its second line (the dialog is as
  wide as a fresh one, about 600 points): a look for the maintainer.
- open 2026-11-11 (the census index, `.tmp/sidelead/sk_census.report.md`) — (1) 603 upgraded
  or Studio FMDLs still carry an old Common path in their strings that no texture reference
  uses (a raw-bytes search finds 8,783 files, the references 8,180): which tool leaves the
  string, and whether the game reads it, is untraced. (2) `spec/export_upgrader.md`'s "6,053
  FMDLs in export folders" names no folders, so no query reproduces it (the index reads
  4,793 with repositories skipped): restate it from the index. (3) The team library holds no
  `Collars` FMDL at all: the 146 counted for S-poc12s4 were copies under this repository's
  `.tmp/`, so the issue below stays unmeasured on real exports.
- open 2026-11-11 (S-poc12s4 r1; `export_upgrader` `kit_magic.rs` `respell_outputs`) — The
  Export upgrader respells the old kit token only below `Players/`, `Boots/`, `Gloves/` and
  in `Common/`, and `kit_token_old` follows it there. Whether Red's kit magic also rewrote
  tokens in collar models (`Collars/`) is unchecked: if it did, an old export's collar
  naming `u0???pN` compiled right under Red and ships the token verbatim now, and the
  upgrader should respell collars too (and `kit_token_old` follow). Check Red's
  `path_id_change` callers on `main`.

- open (2026-11-11) — `scripts/derive_savefile_schema.py` no longer reproduces `schema/fields.rs`
  (from `Tools_4cc/4ccEditor-1` or `DBC7-Editor`): the file was edited by hand after it was
  generated (the serde derives, `ABILITY_FIELDS`, two doc comments). The version tables
  `pes15.rs`..`pes20.rs` still regenerate byte for byte (checked 2026-11-11, with `coaches`).
  Either the generator emits those additions, or `spec/pes_savefile/codec.md` stops naming
  `fields.rs` among its outputs.

- open (2026-11-11) — `studio_core` settings' `read_back == desired` guards (`save`'s in-place
  edit and `fresh_from_template`) survive mutation (`mutants-diff ee80b0ac`, line 224): the
  shape their tests use, a section written inline under a comment, makes an edit that does
  not parse at all, so only the `Err` arm is tested, and none of ten other shapes tried
  (dotted keys, an array of tables, inline, nested and dotted tool tables) parses to other
  values. The guards stay, as the net against writing a file `load` refuses; a test lands
  when a shape that reaches them is found.

- open (2026-11-09) — One Compiler for every export kind (DECISIONS 2026-11-09): before Phase
  9's acceptance is written, the plans naming the Balls and Stadium compilers (about 30 files)
  are respelled for the Compiler and the Balls arranger, and the crate layout is chosen.
  The layout waits on whether guardrail 3 (`spec/core/architecture.md` "Workspace
  guardrails") is amended for test cost, which the maintainer wants only if worth it:
  measured mutation cost per mutant (build + tests) is about 217 s for `save_editor`, 91 s
  for `pes_savefile`, 50 s for `team_widgets`, 23 s for `team_compiler`. First,
  `.tmp/lead/test_times.py <crate>` (nextest per-test times, the libtest wall time with and
  without each binary's slowest tests) over those crates, on an idle machine, to see whether
  a few tests dominate; a split pays only where slow suites are many tests, not a few. After
  0.0.2-poc ships (the maintainer: the PoC first).

- open (2026-11-09) — A member's `Collars/` FMDL is packed as it is on PES 18-21
  (`processing/mod.rs` `TaskKind::Collar`, `Engine::Fox if is_fmdl => Ok(bytes)`): its texture
  table is never read, so a `team_folder` directory keeps the placeholder instead of the team's
  ID, and since poc12 slice 2 so do `team_common_folder/` and `common_folder/` (the upgrader
  keeps a collar's paths as written, so it writes neither). Found by slice 2's sidekick; not
  measured how many real collars name the Fox common directory. The fix reads a collar like a
  package part (the short forms, then the ID replacement), or refuses the Common directories
  there.
- open (2026-11-09) — `compile::tests::a_7z_export_s_source_keeps_its_decompressed_content_into_the_planned_run`
  depends on the machine's free memory: the run's budget cap comes from the physical memory
  available, and with 232 MB free (a poc12 sidekick's run, 2026-11-09)
  `keeps_archive` declines the archive and `source.content.is_some()` fails; it passed on a
  rerun with 2.2 GB free. The test should pin the cap (the budget it plans with) rather than
  read the host's; a flaky gate trains everyone to rerun until green.
- open (2026-11-07) — The Team compiler's recovery manifest (`plans/team_compiler/pipeline.md`
  "Recovery manifest", the maintainer's request): early, since only DLCs compiled after it
  ships carry it. One slice in the writer (each team and referee export's manifest and the
  files it packs beside it, the round trip read back from a compiled CPK); no in-game check
  (every version ignores unread files). The balls, stadium and billboards manifests come with
  their kinds, and the Decompiler (`plans/decompiler.md`, a stub) waits for its own phase.
- open (2026-11-07) — Repro bundles in every tool (`plans/core/architecture.md` "Repro
  bundles", the maintainer's request after the 0.0.2-se19 report). To slice before briefing:
  (1) the button and dialog drafted in `docs/prototypes/` and looked at by the maintainer
  (done 2026-11-07: `index.html` `drawReport`, the sidebar tail's "Report a problem",
  looked at by the maintainer, its wording theirs; the icon is the warning glyph until
  one of its own is drawn);
  (2) `studio_core`'s journal type and bundle writer, with the Save editor's journal taken at
  `Session::changed` and its file operations, its inputs kept, and the test that every model
  change goes through the journal (done 2026-11-07, SWE-2; reviewed to 2026-11-08: SWE-2 r1-r11,
  Astra X1, Argon A1-A3, reworks to `eef1a9e1`; the loop's `mutants-diff 188cf158^..` done
  2026-11-08: 60 mutants, 3 missed, all Write's `NotFound` guard, `generate.rs:333`, tested in se_kills_2a); (3) `utw save-editor replay` (the plan's "Replay reads a typed
  change", DECISIONS 2026-11-08), in three slices: (3a) the player side end to end below
  the command: the step's `op` in `studio_core`, written and read back from a bundle, the
  kept files' indexes, the open's and the save write's ops and every player row's (serde on
  `PlayerDiff`, the face run's unused bits, `pes_savefile`'s apply and a file's salt), and
  replay run over a bundle a `Session` recorded (done 2026-11-08, SWE-2, one rework;
  Astra first, `.tmp/repro/rulings_3a.md`, round A-repro3a r1: 4 test gaps accepted, their rework
  landed; open: the SWE-2 loop over 3a and it, then Argon);
  (3b) the `replay` command, its messages and SE-REP-01 to 06; (3c) the team, roster and
  tactics rows, `Session::apply_patch`'s field lines, and a test per `Session` mutator that
  replaying its journal rebuilds the model. 3b and 3c wait until a member's bundle needs
  them (the maintainer, 2026-11-08, DECISIONS): until then a bundle is read with
  `save-editor diff` of its kept input against its written save beside the journal; (4) the
  "Report a problem" button and dialog in the shell (the draft's `drawReport`), each tool's
  current state through the tool interface, and replay comparing that state last: slice 2
  built the writer and nothing calls it outside tests yet, so no member can write a bundle
  until this lands, and it lands after (3) so the first bundles members post replay; (5) the
  panic hook's bundle; (6) the Team compiler and the Export upgrader; the rest as each tool
  is built. Each slice gets SWE-2, Argon and Astra rounds (a new `pub` interface across
  crates).
- open (2026-11-06) — Whole-crate mutation runs on the VPS alone (`RUNBOOK.md` "Mutation
  runs and the VPS", the maintainer's directive). Slice 1 done 2026-11-09 (SWE-2 sidekick,
  `.tmp/mutation_vps/brief_vps_only.md`; `mutants_test.py` 79, 9 perturbations red): a whole
  run refuses without a host, runs every mutant there, its extra kinds first in the same
  service (`--remote-extra`) and fetched with the rest; the launch record says whether a
  local half exists, which the summary, `collect` and `local_refusal` read; the wait holds
  no run lock, and `mutants-diff` runs here while the VPS is busy. Open: its SWE-2 loop (a
  vital script), Astra after it (it reshapes how a run measures). Then, as a second slice: a stopped run
  resumes where it stopped (the VPS needing an update mid-run is the case), its tested
  mutants kept, never retested and never lost from the summary; first check whether
  cargo-mutants' `--iterate` (skips mutants an earlier run caught) does this for a half
  stopped mid-run, before writing our own. A vital script: SWE-2 rounds over the changed
  functions
- open (2026-11-06) — files past a thousand lines that recent diffs grew. `team_compiler`,
  in 8.36c's review fixes (review S-36c r3): `src/output/writer.rs`'s inline test module
  (~2,100 lines) and `tests/cli/models.rs` (~3,150). `export_upgrader`, in 0.0.2-poc6 and
  poc5/7 (reviews S-poc6c r1, S-poc57u r1): `src/players.rs` (2,993, its inline test module
  ~1,730), `tests/cli/players.rs` (1,248), `tests/cli/carried.rs` (1,178) and
  `src/materials.rs` (2,321). `save_editor`, in se_kills_2a: `src/view/tests.rs` (the
  helpers hub, 1,688), whose new helpers had no smaller home; `view/tests/live_aatf.rs`
  (2,682) and `view/tests/strip.rs` (1,807) grew only a `mod` line, their new tests in
  child files. Split each by
  concern as pure moves (the `fmdl` and `aesthetics_patch` splits' way: a parent keeping
  the shared helpers, one child file per concern, a helper used by one child moved with
  it), with no cargo-mutants run holding the crate.
- resolved (2026-11-06) — `zip` 9.0.5 fixed Dependabot's alerts. `cargo deny check advisories`
  runs in no gate (fails on `pulldown-cmark` 0.13.4, five unmaintained crates): Dependabot watches

- open (2026-11-01, after 0.0.1) — the Ruleset editor's tier handle
  (`ruleset_editor/src/view/sections/tiers.rs` around line 140) is a `dnd_drag_source` whose
  content is a selectable `ui.label("⠇")`: the defect f1e fixed in the Tactics lineup rows.
  egui 0.37's hit test hands a press within `interact_radius` of a click-sensing widget inside
  a drag area to that widget, so a press just beside the handle's glyph starts a text
  selection instead of the drag. The fix is the same (`.selectable(false)`), with a test
  pressing 2 points outside the glyph
- open (2026-11-01, after the cut) — `scripts/gates/run_tests.py` writes its timings to
  `<repo>/target/run_tests_times.json` (`TIMES`), not to cargo's target folder: a worktree that
  builds into another tree's target through `CARGO_TARGET_DIR` (the staging worktree
  `wt-cand-e`) and has no `target/` of its own fails `just test` after every test has passed,
  before the doc tests and the rest of the gates (0.0.1g's first full run). Worked around by an
  empty `target/` in the worktree; the fix reads the target folder as `release.target_dir()`
  does
- open (2026-11-01, after the cut) — `scripts/release/make_candidate.py` puts the candidate in
  `<repo>/../4cc-studio-temp/` (`DEST`): run from a worktree under `4cc-studio-temp` (as
  `wt-cand-e` is), that is `4cc-studio-temp/4cc-studio-temp/`; 0.0.1g was moved up by hand.
  The fix resolves the folder from the main tree (`git rev-parse --git-common-dir`)
- resolved (2026-11-01, cv40) — three `save_editor` view tests raced a worker's reply under load;
  `a_range_turned_by_the_wheel_is_checked_once_it_has_rested`'s cause stayed unconfirmed
- open 2026-11-01 — the vital scripts' first reviews (`RUNBOOK.md` "Review rounds", checkpoint
  (e); DECISIONS 2026-11-01): each reviewed whole by SWE-2 rounds once, in this order:
  - [x] the mutation runs' `mutants.py`, `mutants_diff.py`, `extra_mutants.py` (and
    `mutants_test.py`): SWE-2 rounds 1-3 (S-scripts), the loop ended (`bde2eafb`); then
    `scripts/`' root sorted by job (`6f37bf18`)
  - [ ] `run_tests.py`, `test_runner.py`, `remote_gates.py`
  - [x] `release.py`, `build_remapped.py`, `make_candidate.py`: SWE-2 rounds 1-2 (S-release),
    the loop ended (`82a5ca93`)
  - [ ] `scripts/census/`, `scripts/library_parity/`
- open 2026-11-01, 0.0.2 — two more resamples run on straight alpha (found by the cv18-cv26
  rework, `aeb8c917`): `export_upgrader` `carried.rs` `at_128` and `team_compiler`
  `kit_layout.rs` call `dds_convert::resize`, which fringes an edge against transparency the
  way the event logo did. Check first whether either input carries partial transparency on
  real exports, and what Red's output is there (library parity), before moving either to
  `resize_straight_alpha`
- open 2026-10-30, Phase 20 — a tier id's rename carries its key in the medal systems alone
  (`plans/ruleset_editor.md` RE-GUI-17), so renaming a tier that a special's `when.tiers`, a
  per-tier effect map or another tier's `counts_as` names leaves an "unknown tier id"
  problem the bottom bar lists until the tier is renamed back (found at cv8c: renaming
  Autumn 26's Gold makes Save refuse four specials). In 0.0.1 the specials cannot be edited,
  so renaming back is the only way out. Revisit with 20.6a2b/20.6b: carry the rename through
  every reference, or keep the problem as the member's cue.

- open 2026-10-30 — two member-facing errors still number a roster slot from 0, where the
  player list numbers it from 01 (found reviewing d1b2, whose rule covered the badge's failed
  check): `pes_savefile` `texport.rs`'s `roster slot {slot} holds player {roster}, the
  record is {player}` (a Texport import's refusal) and the Save editor's
  `session/player_ops.rs` `team {team_id}'s roster slot {slot} is empty`. Each takes the
  list's `place NN` the way `aatf`'s `MissingPlayer` now does; ride the next Save editor
  text slice

- open 2026-10-29 — `studio`'s `tests/crash_dump.rs`
  `a_stack_overflow_leaves_a_dump_and_the_monitors_line` failed in k5a's checkpoint
  (`gates-changed 4e311a05`, four test binaries at once beside `save_editor`'s): the crashed
  run took 51.9 s and left no `data/utw.dmp` (`Minidump::read_path` → `FileNotFound`); alone
  it passes in 0.16 s. So the crash monitor's dump fails or times out under load, which a
  member's machine running PES may also be. Not k5a's (it touches no crash path). Diagnose
  before trusting a dump on a loaded machine: rerun under the gates' load, read the
  monitor's own wait and the log it leaves; do not loosen the test

- open 2026-10-28 — `studio_core`'s first-run tests depend on the portable hint's width
  (8.29b): a hint of about 100 characters made three of them fail as if a click were lost
  (`the_first_run_dialog_writes_the_settings_file_to_the_user_folder`,
  `the_portable_choice_writes_next_to_the_program`,
  `a_failed_write_keeps_the_dialog_open_with_the_error`), with the old hint back they passed;
  78 characters pass. The dialog's layout moves with the text and the tests' two-frame settle
  no longer finds the button. Fix when next touched: settle until the layout stops moving
  (bounded), or click the button by its id, so a rewording cannot pass for a broken click
- open — Studio Web (phase 18): `theme::install_fonts` is native-only, so on wasm the named
  families the views draw in, `icons` (the sidebar) and `label` (the Tactics pitch's formation
  name, 0.0.1x4), are unbound, and epaint warns and draws no glyph for them (read in epaint
  0.37's `font_definitions.rs`, not run). The web frontend installs the same fonts when it
  starts (found by x4's sidekick, 2026-10-27)
- open — `plans/core/distribution.md` "Changelog and version display" was never moved to the
  spec although the version label it specifies is built (status bar, About, `--version`, the
  log's first line): `version_label`'s doc comment pointed at the spec heading that does not
  exist, across a line break the pointer check did not read; 0.0.1k's rewrap exposed it and
  the pointer went (2026-10-26). The built part moves to `spec/core/distribution.md` under
  the same heading, the changelog's unbuilt rest staying in the plan
- open — a save whose one player name is not UTF-8 is refused whole (0.0.1i's census,
  2026-10-25): `pes_savefile`'s player codec errs (`codec/player.rs`, "player name is not
  UTF-8"), so `check-aatf` and every other command refuse
  `Documents\KONAMI\Pro Evolution Soccer 2017\save_old\EDIT00000000 testteams` (a test save,
  the census's copy `.tmp/release_0_0_1/0_0_1i/census/saves/5e3d82ea3c9469ee` on side-lead B's worktree).
  Not diagnosed: which player and which bytes (a name cut inside a multi-byte character by an
  editor is the guess, unverified). Whether the game itself writes such names decides between
  a tolerant read and the refusal; one save of 424 holds one
- open (the Ruleset editor's half fixed 2026-10-23, 20.6a1's rework 2; the Save editor's
  waits) — a test can open a real file dialog on the developer's desktop (the maintainer saw
  four "Save As" windows, `.rhai` suggested, on 2026-10-23): `mutants-diff 8aeb0193`'s mutant
  `menu_item -> true` clicked every menu item each frame, and the views the tests build with
  `RulesetView::new()` carry the real `rfd` pickers, so Save as… opened a native dialog and
  the test hung until the run's timeout killed it (TIMEOUT, not caught). CONTRIBUTING: a test
  fails, it never hangs, and never reaches the developer's own environment. Fix (20.6a1's
  review rework, after 8.11): under `cfg(test)` the default pickers panic ("a test reached a
  real file dialog: stub `open_dialog`/`save_as_dialog`"), so such a mutant fails at once
  with no window. The Save editor calls `rfd::FileDialog` straight from its click handlers
  (`view/mod.rs`, `view/menus.rs`): the same exposure under a mutant, and the same fix
  there (pickers behind fields, panicking under `cfg(test)`), in its next rework
- open (SWE-2 S3 on 20.6a2a, 2026-10-24; for the Ruleset editor's resumption after 0.1.0) — a
  Ctrl+Z pressed while a tier's drag handle is held restores the tiers under the drag, whose
  payload is the row index: the release moves whichever tier now holds that index, and panics
  when the undo left a shorter list (`sections/tiers.rs`, the `dnd_drag_source` payload). A
  two-handed gesture, rejected for 20.6a2a as an edge case; the fix to weigh then is the
  tier's id as the payload, resolved at release
- open — tests built on the default settings may reach the real install (Astra A1 on 8.9j1,
  2026-10-24: a Save editor FPC test's `ToolContext` from `studio_core::Settings::default()`
  kept the default `pes_folder_path`, and `Tool::view` with a save open started the portrait
  worker against it; the Save editor's tests fixed in 8.9j1's rework 2). The other crates'
  tests that build a `Settings::default()` (`team_compiler`, `export_upgrader`,
  `ruleset_editor`, `studio_core`'s shell harness) are not audited yet: each one that reaches
  a path setting gets a scratch path, before the 0.0.1 cut
- open — repetition in the code (the maintainer, 2026-10-23, after the writer's review let
  `aatf`'s `writer/mod.rs` keep one six-line shape over 40 times): CONTRIBUTING's "The third
  copy is a function" had no check, so neither the sidekick nor the lead's sweeps applied it.
  A shape detector (`.tmp/topics/dup_audit/clones.py`: identifiers, strings and numbers blanked,
  declarations and table rows set aside; a run of 4 shaped lines found 3 times in a crate, or
  of 10 found twice, is a cluster; `--across` for runs in two crates) measured HEAD
  `8aeb0193`: about 260 non-test clusters (pes_savefile 70, aatf 30, team_compiler 31,
  save_editor 23), 81 across crates (`fmdl`'s and `pes_model`'s `ops/split/build.rs` share
  about 180 lines), and some 65,000 repeated shaped lines in tests. The work: (1) SWE-2 rules
  every non-test cluster, five groups (`.tmp/topics/dup_audit/swe_dup_<a-e>.md`, findings beside
  them), the lead rules the findings; (2) one rework brief per crate for the accepted ones,
  between the slices in flight, each gated and measured by `mutants-diff` like any rework,
  `writer/mod.rs` first after 20.6a1; (3) the test clusters, the same way; (4) the detector
  moves to `scripts/` with a `just clones` recipe, the thresholds the audit's rulings settle,
  and a committed keep list (each kept cluster's shape and its one-line reason, which the
  detector then skips), and runs at two points (the maintainer's question of 2026-10-23):
  in `just gates`, failing on any cluster the keep list does not hold, so a diff that adds a
  copy is fixed or ruled before it lands, with no reliance on a reviewer noticing; and at
  every phase's close, whole-crate, in the design-health pass (`RUNBOOK.md` "Closing a
  phase", beside `just mutants` and `just citation-audit`), where the keep list's entries for
  the phase's crates are read again, since a kept pair that grew a third copy is no longer
  a pair. `AGENTS.md` and CONTRIBUTING "Code style" say both. State: (1) done, the rulings in
  `.tmp/topics/dup_audit/rulings.md` (group IDs `A-F<n>` to `E-F<n>`, the lead's own `C-L<n>`; keep
  that file until (2) ends); (2) the first brief, `aatf` and the Ruleset editor's summary,
  `.tmp/topics/dup_audit/brief_rework_aatf.md`, done 2026-10-23 (SWE-2; `aatf`'s source clusters 30
  to 21, the summary's 7 to 5; one `push`, `check_empty`, `check_unique`, `id_known` and
  `check_id` in the checker, `push_set` and `push_some` in the writer, the new `pub`
  `PlayerValue::key` spelling a `ranges` key once for the writer, the reader and the
  summary; the writer's bytes and the summary goldens unchanged), reviewed 2026-10-24 (SWE-2
  S1 3 of 6, Astra A1 2 of 2, S2 none; the lead's fixes `ff88177d`: `id_counts`,
  `number_list`, `check_color_code`, the summary's `group_label`; `mutants-diff b8ed279f`:
  `aatf` 84 caught, none missed; over the fixes, `mutants-diff 86d79c92`: none missed), then
  the other crates' briefs; reworks touching `fmdl` or `pes_model` also run `just bindings`, and the first of them adds each mirrored file's header
  naming its twin (`spec/libs/format_crates.md`, "`ops/` are format-native transformations":
  the two `ops/split/build.rs`, `ops/split/combine.rs`, `ops/vertex_enc.rs`, `check.rs`, and
  `model/validate.rs`); (4)'s check also flags a keep-list entry that no longer matches, so a
  change to one copy of a kept pair stops the gates until both copies agree or the entry is
  re-ruled
- open 2026-10-23 — a command's `Report` (`events.rs`: its findings sent as events, the worst
  severity deciding the exit code) is copied in `save_editor` and `ruleset_editor` (20.5a),
  about 40 lines each, the Save editor's with one more method (`output`). Two copies are not
  yet a pattern; when a third tool needs it, it moves to `studio_core` and the copies go
- open 2026-10-21 — the 4ccEditor source audit's rows that need a measurement or a check of
  built code (the rest are in `plans/save_editor.md`'s parity table and "Editing UI", and in
  8.9b1's rework): (1) a custom skin of 7 on PES 18+ becomes 0 in 4ccEditor (`main.cpp`
  `show_player_info` and the squad load), where `spec/pes_savefile/model.md` `SkinColor::Preset`
  and the cross-version conversion reset it to 1: which one the game shows as the intended
  skin, checked in Edit mode; (2) 4ccEditor clamps a shirt number to 231 below PES 19 (its
  number field and the squad load), where the info strip takes the field's bit width: why 231,
  checked in game; (3) 4ccEditor's ALL hides players on no team and its team list hides a team
  whose first slot is empty (`pes20.cpp`), lists both in record order: compare with 8.9a's
  selector and list, and rule. (4), the motion randomizer's ranges, is in the plan
  (2026-10-22: from 1 to the measured tops, never the dribbling motion; DECISIONS)

- open 2026-10-19 — the maintainer saw AMD driver timeouts during the BC7 encoding tests
  (`dds_convert`'s `gpu.rs` tests, 16.x a). Logged so far: one Windows GPU reset (System log,
  event 4101 "Display driver amduw23g ... stopped responding and has successfully recovered")
  at 03:16:41, while 16.x's probe (`.tmp/phase_16/16x/gpu_probe/`, edited 02:50-03:09) dispatched whole
  mips in one submission, before `gpu.rs`'s bounded slabs (`SUBMISSION_PIXELS`, c68ef99 at
  04:36); none logged since. Still to rule out: the tests' own load, each GPU test building its
  own device and `cargo test` running them in parallel, several worktrees' gates at once, a
  `dds_convert` mutation run's parallel jobs. Parked (the maintainer, 2026-10-19): no
  investigation now; during every later BC7 test (16.x b, a `dds_convert` mutation run) check
  the System log for event 4101 (`Get-WinEvent -FilterHashtable @{LogName='System'; Id=4101}`)
  against the run's times, and if one recurs, reproduce (the GPU tests looped, alone and in
  parallel processes) and serialize the tests' device use or lower the budget, whichever the
  measurement points at.

- open (2026-10-15, 5.6c census) — PES 2014 files read as PES 15 and then fail with a
  misleading error: `SaveContainer::decrypt` labels a PES 2014 `EDIT.bin` or `TEXPORT0N.bin`
  `Pes15`, and the decode fails on a player name (not UTF-8) or a tactics byte. Loud, never a
  wrong write, but it should be refused as PES 2014 (unsupported): measure what separates the
  two versions' containers (PES 2014's EDIT is 4,799,651 bytes, its texport 5,389,806; no real
  PES 15 texport is on the machine) and refuse it by name. `.tmp/phase_05/5_6/audit_notes.md`.
- open (2026-10-16, 5.6b mutation runs) — three GUI views are placeholder lines no test reads:
  `save_editor`'s `view` and `settings_view` and `team_compiler`'s `settings_view`
  (`lib.rs`), each a survivor when replaced with `()`. Their tests come with the views they
  stand in for (the Save editor's window, the Team compiler's settings editor, Phase 8), with
  `team_compiler`'s headless `view/mod.rs` harness.
- open (2026-10-15, Astra review B.A1 of 5.3b) — nothing refuses an `output_folder_path` that
  resolves to the PES folder's `download/` (or inside the PES install): a degraded or
  `--no-deploy` run then installs into the game past every deployment check, and a savefile
  failure's rollback promotes the new CPK over the old one it just restored. Fix when the
  settings get their validation pass (Phase 8's settings view at the latest): refuse such a
  path on the command line and in the settings view, comparing the canonical paths.
- open (2026-10-11, Phase 5 Acceptance) — a referee export's `settings.toml` reaches no table or
  patch: a referee's look is the template's `RefereeAppearance.bin` (`blue_port.md`), whose rows
  nothing writes, and `blue_port.md` has `fpc_off` apply its settings preset to a referee on
  PES 18-21. Settle with a Fox refkit (the issue on a referee's body under `fpc_off`): which
  route a referee's settings take (the aesthetics patch, as team 999, since 8.36 writes no
  player tables, or the template's table), and TC-REF-19's outcome then; with it, lift
  `settings_id_messages`' team-only gate (`validation.rs`).
- open (2026-10-11, 5.0c; deferred until the cup next plays a pre-Fox version) — what Red does
  on PES 15-17 that Studio does not (`.tmp/phase_05/5_0c/xml.md`, `checks.md`), most noticed first:
  - a member's `face.xml` that does not list `hair_high_win32.model` (1,248 of 1,263 such
    folders on the maintainer's machine): Red packs the file and the game loads it by its fixed
    name (inferred), while Studio reports `xml_model_unlisted` and leaves it out, so the head
    compiles bald
  - a generated `face.xml` lists `hair_high` as a `parts` entry, while Red leaves
    `hair_high_win32.model` out of the xml and packs it under its own name; the in-game effect
    is unknown
  - a `.mtl` Common path that only an installed CPK supplies keeps its `XXX`, while Red's
    `txt_id_change` writes the team ID in every `.mtl` (the answer to the question
    `messages.md` leaves for the pre-Fox parity step)
  - a Common model or `.mtl` that a member's xml names and only an installed CPK holds: Red
    accepts it, Studio reports `xml_model_not_found`
  - a WESYS-wrapped `face.xml` (48 on the machine): Red unwraps it, Studio reports `xml_broken`
  - a `.mtl` Common path whose team folder is not 3 characters: Red reports an Error, Studio
    nothing
  - a `<model>` with no `material`: Red reports an Error, Studio writes it (TC-XML-01)
  - `mtl_state_nonrecommended` misses Red's `alphablend 0` with `zwrite` other than 1 (Info)
  - two models packing to one name (`hat.model` beside `oral_hat_win32.model`)
  - the dummy team-ID respelling of file names (`Common/u0XXXg1.dds` named as
    `.../common/XXX/u0XXXg1.dds`, 9 exports): the file and the path are respelled together or
    not at all
  - DECISIONS 2026-10-09 sends the xml's `<dif>` on PES 15-17 beside a `face_diff.bin`, while Red
    lets the `.bin` replace it; the two differ in 36 of 53 such folders
  - `boots_edit.model` and `glove_edit.model`, the Edit-mode models (2 found), and a member's
    own `glove.xml`
  - a member's `face.xml` path in a game-path form other than `model/character/face/common/…`:
    Red reports an Error for a model or `.mtl` (a Warning for a texture), Studio keeps it with
    `xml_path_unchecked` (W); DECISIONS 2026-09-12's "Red waved unresolvable game paths through
    silently" holds for that one form only (`messages.md`, the `xml_path_unchecked` paragraph)
- open (2026-10-11, 4.y-mattable row 27; deferred until the cup next plays a pre-Fox version) —
  the cup wiki says a `.mtl` splits a path at spaces, and 212 of 60,027 texture names in real
  FMDLs hold one (`Kit 1_u0XXXp0.dds`), so a Fox model converted for PES 15-17 may write a
  `.mtl` path the game cuts short. Unverified in game.
- open (2026-10-11, 4.y-fix23 (1); deferred until the cup next plays a pre-Fox version) — a
  PES 17 referee whose only model is a `.model.common` link gets no face (`roles.rs`
  `is_package_model` does not count `PreFoxCommonModel`), while the converted Common model is
  still written into the refs CPK with nothing naming it; the link should count as his face
  part, as a team player's does.
- open (2026-10-11, S19.1-B3) — which material files a glTF in a player or shared folder's
  subfolder reads (`model_format.md` "Name matching" is written for one flat folder; the
  `.mtl` search's two folders, the file's then the model folder's, are the likely answer):
  for Phase 7's plan pass, when glTF sources are planned.
- open (2026-10-09) — a Fox referee's body under `fpc_off` waits on a Fox refkit, an FMDL
  body the maintainer will make ("We'll make one later"; `team_compiler/blue_port.md` "The referee
  body"): then the Fox referee template ships it and a step merges it into an `fpc_off`
  referee's face package. Until then `fpc_off` applies its settings preset alone on Fox.
- open (2026-10-09) — per-kit face packages on Fox wait on FoxDen (`model_format.md`
  "Kit-dependent assets", DECISIONS 2026-10-09): each kit number gets a face package of its
  own, merged from that number's variants of any file kind, once FoxDen loads one per kit
  picked; the layout it reads is decided with FoxDen's work, then a step in the phase open
  at the time builds it and retires `kit_variant_model_fox`. FoxDen is a work in progress.
- open (4.y-fix17's review, 2026-10-10) — a PES 17 `.model` collar compiled for PES 15 or 16
  keeps its `uni_collar` material, packed as is when the pre-check does not flag it (PES 16)
  or moved onto the skeleton when it does (PES 15), while those versions' `uniform.mtl`
  defines no `uni_collar` (the FMDL conversion names `uni_shirts` alone for them, DECISIONS
  2026-10-09). What the game draws for a collar material its `uniform.mtl` lacks is unknown:
  an in-game check on PES 16 (the F: install), and if it draws nothing, the same-engine
  collar gets the version's names as the converted one does.
- open (converge census, 2026-10-09) — a pre-Fox face of several `.model` parts converted
  for PES 18-21 fails to merge: each part's conversion generates its own skeleton (the bones it
  uses) and the face merge refuses the differing skeletons (`skl_merge_conflict`,
  `skeleton=differs` or a `bone=`), nine referee folders in seven of the cup's current referee
  exports (FAA, germanbro, coolguy, geodude, donte, ...). Red never converted a `.model` for
  Fox, so these referees were never on PES 21; the fix is the converted parts of one folder
  sharing one generated skeleton (the union of their bones, or the target's full table), a
  `model_convert` and `fmdl` merge question for Phase 7, when the conversion work resumes.
- open (converge census, 2026-10-09) — on PES 18-21 a member's own `face.xml` is ignored
  (`xml_ignored_fox`), so a pre-Fox referee folder with Red-packed names
  (`oral_fixed_kit_win32.model`, five `.mtl` files) converted for Fox pairs each model with
  the `.mtl` search's choice (`apc.mtl`, the first by name) where the xml names `fixed.mtl`, and
  `model_material_undefined` drops the folder (ANF, four cup exports). The xml's `material`
  attributes could drive the conversion's pairing on Fox; a plan ruling first. Low priority:
  these referees are pre-Fox material.
- open (converge census, 2026-10-09; diagnosed as the plan's rules, no change) — the census's
  other refusals of the cup's referee exports: `common_file_disallowed` on `Common/refkit/`
  (a maintainer question, `docs/QUESTIONS.md`); `file_type_disallowed` on files two levels
  down (`Face/kit/`, `Face/face/`, a `Common_shared/` subfolder, a `.model` in a player's
  `common/`), which the plan's reserved-subfolder rule names and the Export upgrader flattens;
  `texture_not_pow2` on a 192x512 mipped texture (a maintainer question); ANF's
  `face_high_win32.model` naming `head_phong`, which the `face.mtl` his xml names lacks (a true
  defect of the export; `pass_through` compiles it as Red did). Three referee exports compile
  clean on both engines (Summer 26, `refs numbered`, Autumn 25 Final Boss on PES 17).
- open (found at 4.14b's review) — pre-Fox faces: two files of one face packing under one
  name (`hat.model` beside `face/hat.model`, two `.mtl` of one name in the folder and
  `face/`, or a member's own `dummy.mtl` when the dummy is added) fail the face task at
  `compile` (`folder_pack_failed`, folder dropped) while `check` reports nothing. No finding
  code exists for it; a planning check naming both files would make `check` agree. No real
  export is known to do it.
- open (found at 4.14b's review) — the pre-Fox gate names an `ingame_face` marker as
  `<folder>/ingame_face` (without a `.txt` it may carry), since `PlayerFolder` keeps no
  marker file's name; the message is still findable. Gone when 4.14e compiles it (links
  compile since 4.14c1).
- open (found at 4.14c1's review) — pre-Fox combined face: the player's own texture of a
  stem the linked `Faces/` folder also holds, with other bytes, is `merged_texture_conflict`
  and drops the folder, as on Fox, while his own model or `.mtl` of a shared file's packed
  name replaces it (the plan's copy with local files on top). On pre-Fox no merged model
  makes the texture ambiguous, so the player's could win too; the plan's step 6 rule is
  engine-wide today. Safe as it is (an Error, nothing wrong written).
- open (found at 4.14c2's review) — pre-Fox Common: a texture link whose target only an
  installed CPK holds is still `common_link_missing` on PES 15-17
  (`installed_common_textures` is empty there; the Fox walk's stems are `.ftex` paths). A
  later step can read the installed CPKs' `common/character1/.../common/{team}/` `.dds`
  stems the same way.
- open (found at 4.14c2's review) — pre-Fox Common: with `pass_through` on, a `Common/` `.mtl`
  kept despite an eligible Error is left out of the deep pass's `.mtl` search (which cannot
  see the setting), so a model whose only `.mtl` it is gets `model_material_undefined` where
  `compile` could have built it. Conservative (drops, never panics); rare.
- open (found at 4.14c2's review) — pre-Fox: two `.model.common` links to one Common model in
  one folder (`legs.model.common` and `boots/legs.model.common`) give one `face.xml` entry,
  the first in file order, though their positions could type them differently. Fox merges
  them as one part too. No real export is known to do it.
- open (found at 4.14c1's review) — pre-Fox: a file a task does not use is still read and
  charged (a shared face's model or `.mtl` the player's file replaces, a shared boots
  folder's `.mtl` its model does not use): `TaskKind::files` lists every role. Harmless.
- open (found at 4.14b's review) — pre-Fox faces: every `.mtl` of a player folder is packed
  into the face, one no model uses included (a blank face too), as Red packs the whole
  folder. Harmless to the game; revisit if a census shows stray `.mtl` files are common.
- open (2026-10-08) — the PC blue-screened three times during a local mutation run (4.14c2's
  half, 4.14e1's and 4.14e2's; MEMORY_MANAGEMENT at the first), never otherwise: the two
  cargo-mutants jobs each build a copy of the tree, the heaviest load the machine gets now that
  `team_compiler` runs locally whole. The maintainer memtested the RAM (clean), turned XMP off
  after the second, and after the third set the paging file to 32 GB, the size it had before
  the crashes began. A killed run leaves its two `%TEMP%\cargo-mutants-4cc-studio-*.tmp`
  copies (3.6 GB each; `.tmp/lead/rm_mutant_copies.py` removes them, clearing the read-only flag
  a test leaves on one file). If a run crashes again at 32 GB, the next lever is one job
  (`--jobs 1` in `scripts/mutation/mutants.py`'s local config).
- open (2026-10-08, 4.14e3's plan) — pre-Fox: a hand-weighted model behind a `.model.common`
  link is listed whole by its Common path, not split (`spec/model_conversion/hand_split.md`
  "Pipeline integration"): the game then loads a body with hands on the body skeleton. Splitting it
  means packing a copy into the face. Build it when an export has one. Likewise (4.14e3,
  decision entry) a hand-weighted model whose `.mtl` is a Common file fails its folder with
  `model_conversion_failed` instead of reading that file for the split.
- open (found at 4.14e2's review) — pre-Fox combined folders: when a player's `.mtl` replaces
  a linked folder's of the same name and a linked model he does not replace used it, that
  model is listed with his `.mtl`, which the deep pass never compared with its materials
  (it checked the linked `.mtl`): a material his lacks is undefined in game with no finding.
  The combined face behaves the same way. Rare (two `.mtl` files of one name, the linked
  model kept); a fix checks the material names against the `.mtl` that is packed.
- open (found at 4.18's review) — hand auto-split: a split glove beside an authored glove
  of the same hand may not merge. The split prunes the bones no vertex of a glove uses, so
  a split `sk_hand_l` can lose its parent `sk_forearm_l` where an authored glove keeps it,
  and `fmdl`'s merge reports `skl_merge_conflict`, leaving the gloves out (seen with the
  tracer's glove beside the lead's fixture, whose bones are all roots; unmeasured on a real
  full-body model). A player holding both is unlikely; revisit if one appears (the fix
  would be the split keeping a used bone's ancestors, a `model_convert` change).
- open (found at 4.18's review) — hand auto-split: a player folder with a plain `Gloves/`
  link and a hand-split face part keeps the link plain (links combine by file names), so
  the shared folder still compiles under its shared ID while the player's gloves ID goes to
  the split gloves, its own task. An authored glove beside a link combines
  instead. The plan is silent; no real export is known to do it.
- open (found at 4.18's review) — hand auto-split: a face model whose hand-weighted
  vertices sit in no face gives `model_hand_split` an empty `gloves` value and an empty
  gloves package. No real file is known to reach it.

- open (found at 4.26c's review) — when an all-or-none install's undo cannot move an old
  CPK back from `.cpk.old` (`deploy.rs` `undo`), the failure is only logged: `download/`
  then lacks a CPK its list names, so the game loads none of the folder, and the member
  sees only the `old_cpk_locked` or `deploy_target_unwritable` that started the undo. The
  move back renames a file the run itself just moved, so nothing real is known to fail it;
  the plan names no finding for it. Revisit if a member meets it.

- open (found at 4.26b's review) — a multi-CPK run promotes its CPKs to `output/` one
  rename at a time, so a rename failing part-way leaves `output/` with some new CPKs and
  some of the previous run's. The plan settles all-or-none for the installation only
  (`spec/team_compiler/pipeline.md` "Deploy CPKs"); the output folder is the maintainers' build folder, and the
  next run rewrites every CPK. Revisit if a maintainer meets it.

- open — the `livecpk/` replacement is not atomic (4.23a): the previous tree is removed before
  the staged one is renamed in, so a copy that fails partway (the output folder on another
  drive) or a removal that fails partway (a file the runtime holds open, unverified for FoxDen)
  leaves `livecpk/` partial or empty, reported as `output_commit_failed`. A swap through a
  sibling folder would narrow it; decide with the first in-game sideload test.
- open — the compiler reports only two of `kit_config::validate`'s findings (the two a
  version's encoding clamps, as `kit_config_version_clamped`, step 4.9a). The maintainer's
  rulings (2026-10-10): the three "requires model 144 or 160" warnings are reported as one
  Warning, `kit_config_option_ignored` (step 4.y-kitwarn); the two Infos
  (`kit_shirt_model_unknown`, `kit_unknown_sleeve_value`) stay unreported (step 4.y-ingame2
  (e): the game draws both values, no crash; DECISIONS 2026-10-10);
  `kit_collar_zero` (the lib's one Error; `messages.md` says `kit_config_invalid` covers a
  config that "fails to validate", which the deep pass does not do) waits for a collar
  census: before a kit is dropped for a zero collar, count how many real configs carry one
  (the AET exports' `Kit Configs/` and the installs' CPKs), eventually.

- open (4.y-fix11's review, 2026-10-10) — read in the Fox-to-pre-Fox direction,
  `KIT_LAYOUT_REMAP` puts two right-sock stripes (pre-Fox 2008-2040 and 1656-1688)
  exactly 6.00 units off their range's centre, TC-KIT-19's tolerance: the test passes
  at its edge, and one texel more would fail it. Nothing in game shows it (the
  maintainer's PES 21 view of design A sits as drawn, 4.y-ingame2). Check the right
  sock's column of the table against the models at the design-health pass before
  widening the tolerance.

- open — the headless egui test harness (a frame with AccessKit on, a node by label, a click as
  press and release frames) exists twice, in `studio_core/src/shell/mod.rs` and
  `team_compiler/src/view/mod.rs` (3.y design sweep). The next tool view in Phase 8 would be
  the third copy: extract it then into `studio_core` behind a test-support feature, rather
  than a third copy.
- open — for Phase 8's settings menu, the first code that saves `settings.toml`: the `[common]`
  table is read into `CommonSettings`, so a key it does not know (a newer version's) is lost on
  save, while a tool's table keeps unknown keys (`core/gui.md`). Settle whether `[common]` keeps
  them too before that save path lands (review S2.1).
- open, waits on FoxDen — Fox per-kit models: the maintainer answered (`DECISIONS.md` "on Fox
  each kit number gets a face package of its own, once FoxDen loads one"); the package layout is
  decided with FoxDen when it gains the support, and `kit_variant_model_fox` and
  `spec/model_format.md` "Kit-dependent assets" change then.
- open — an unwritable `livecpk\`: sideload mode writes it "with the same access path as
  deployment to `download/`, elevation included" (`spec/team_compiler/pipeline.md` "Output modes"), but no
  plan says what a sideload run does when it cannot (a tree has no `output/` to degrade
  to). Settle with the sideload step (review S6.1b item 21).
- open — the deep pass reads everything `compile` reads again (4.7): each checked file is
  read once by the pass and once by its task, and a solid `.7z` is decompressed twice (its
  check, then its tasks; three times before 4.7f for an export holding a metadata file).
  For a folder the second read comes from the system's file cache; for a `.7z` it is a whole
  extra decompression per export, about 2 s for a 0.6 to 1 GB export (4.7f's timing): about
  2 s of a 5.2 to 6.8 s compile. The buffer is not kept from the check to the tasks because
  planning needs every export validated first, and holding every `.7z` until then is the
  residency the budget forbids (`spec/libs/pipeline.md` "What a solid `.7z` is charged"). The
  maintainer's answer (2026-10-10): a small archive is kept, step 4.y-7z, done the same
  day: the `.7z` half of this issue is closed; the folder half (the file cache's second
  read) stands.
- open — a face diff is engine-specific (maintainer, 2026-10-03): a diff authored for one
  engine misplaces the face on the other; the compiler passes a diff through for whatever
  target it compiles, so a diff is the author's responsibility, as with Red. Investigated
  offline 2026-10-15 (`.tmp/topics/face_diff/findings.md`): one format on both engines, Konami's
  diffs carried unchanged from PES 17 to 18; the real split is version 4 (32 entries) against
  5 (33), with a mechanical conversion. The maintainer's example (2026-10-17, a member's
  anime face, `.tmp/topics/face_diff/lapwing/findings.md`): the Fox fix is two eye floats (head 0x2C
  and 0x30, 35 to 45), which PES 21's Edit mode does not show even at 0 and 90. Settled
  2026-10-19 by a second example and its author's explanation (`Sopdog`, the engines animate
  eyes differently, so model and diff are made per engine and do not convert). Answered
  2026-10-20 (DECISIONS "a face made for the other engine is transformed"): a `fox`/`pre-fox`
  face marker and the Sopdog pair's transformation applied when the engine changes, attempted
  after the DBC editors (plan `model_conversion/conversion.md` "A face made for the other
  engine"): derive the rule from the pair, check it in game on both engines, then build it.
- open — u16 face indices cap a reassembled split mesh (found at 2.20f review): `fmdl::Mesh`
  and the IR (`spec/model_conversion/ir.md` "IR struct") store faces as `[u16; 3]`, so `fmdl::ops::split::decode`
  refuses (loud `VertexMismatch`) an add-on file whose components together reference more
  than 65536 vertices, and vertex-limit splitting of an IR mesh can only move loose vertices.
  The 2.20h conversion census found 114 real files over it. Since 2.20h such a group stays
  split rather than erroring (`conversion.md` split rules), so nothing refuses them now. The
  face type is still Phase 7's decision (glTF brings u32 indices).
- open — texture names and case on Fox (found at 4.4b2): stems now compare case-folded when a
  player's sources share one, but `processing/model.rs` still points a model's texture path at the
  player's folder only when the referenced name matches a stem exactly (since 4.3), so a model
  naming `Shirt.dds` beside `shirt.dds` keeps its original directory; the written `.ftex` keeps
  one source's spelling while each model keeps its own. Whether the game's CPK lookup is
  case-insensitive is unmeasured; measure it (or fold both sides) before 4.20 opens every export.
  Also: per-stem conflict decisions are independent, so with two conflicts in one folder a
  package can be dropped over a stem whose winner is a package already dropped for another stem
  (rare; needs two differing stems across three packages).
- open — `model_convert` converge questions (2.20): (1) Fox decal shaders (`translucent`,
  `3ddc`, `eyeocclusion`) infer `Shaded` *exact* through the `3ddf` rule, so the highneck
  fixture converts to an opaque `Basic_C`, where the 19to16 converter wrote `Overlay` with alpha
  blending; the format plan deliberately gives decals no family — decide whether the rule should
  at least mark them approximate. (2) Hand split: a face inside both hands' selections goes to
  both gloves; unused materials/textures are not pruned from the split parts. (3) `retarget`
  returns `ConvertError::Validation` after mutating the IR (no rollback). (4) `.model` tangent `w`
  is 1.0 on import (plan bullet); the bitangent-derived handedness is untested in game.
  (5) A `Metal` material without an `Environment` texture exports `Basic_CNSR` with no
  `EnvironmentMap` sampler; the format plan assigns the template-cubemap fallback to the
  compiler's texture step (Phase 4), which must add the sampler, not just the file. (6) Konami
  FMDLs carry a 64-byte-per-bone `bone_matrices` block the IR does not; the add-on writes it
  empty and its files work in game, so the export drops it with a finding; what the block holds
  and whether it is derivable from `Bone.matrix` is unmeasured.
- open — `pes_savefile` the kit-slot rows: the ten-slot kit block is measured on 18 (+0x1C),
  19 (+0x20) and 21 (+0x30), with what Strip > Add writes (`spec/pes_savefile/model.md` "Kit
  slots and the strip table"); the tables carry `KitSlot*` rows for PES 17 alone. Add the rows
  for the other versions when the Save editor needs kit bindings. (The PES 21 team colours,
  this issue's other half, were measured in Edit mode on 2026-10-13 at the tables' bits.)
- open — FoxDen's PES 2021 gloves trampoline discards `GloveList.bin` (step 4.0, runs 4-7):
  `build_botch_21` in `Tools_4cc/FoxDen/scripts/01_patches.lua` (and the patched exe it
  reproduces) loads the gloves id from `[rsi+0x2C]` for outfield players and goalkeepers,
  overwriting the value the original code chose in `[rsp+0x70]`, which for a player whose save
  appearance id is -1 is his `GloveList.bin` row (the stock exe showed it; the patched exe showed
  bare hands). Request for FoxDen's maintainer: keep the original value when the player has no
  save appearance record, for outfield players too (p1 routes them through the goalkeeper path),
  then an in-game retest with a `GloveList.bin` row for an outfield and a goalkeeper player.
  The Team compiler's gloves output on Fox depends on it. Probably moot since 8.36: no
  `GloveList.bin` row is written, and the patch writes the record's gloves id, which
  `[rsi+0x2C]` probably holds (not checked); 8.36d's in-game check settles it.
- open — `pes_savefile::ops::fpc::is_fpc_player` finds no FPC player in a real cup save (step
  4.0 survey of the 2023 Winter Cup save): FPC teams there give each player a blank boots and
  gloves id of his own (`/a/`: 126/126 ... 148/148) rather than 55/11, which its doc comment
  already calls indistinguishable. The Save editor cannot use it to tell FPC teams apart; a hide
  strip (long sleeves, tucked, short socks) plus boots no other player wears marked them in the
  survey (`.tmp/phase_04/4_0/apptest`, `fpc` subcommand). Settle with the FPC toggle, built with the
  Save editor's view in Phase 8 (DECISIONS 2026-10-15 "the session wiring only the view
  drives").
- **Texport write is unverified in-game** (2.17h): `Texport::new` synthesizes 18-21 files from
  measured templates and `to_bytes` rewrites read files; both round-trip byte-identical, but no
  generated file has been imported by the game yet (`plans/pes_savefile/verification.md` "Texport write": manual,
  per version; a `new` file and an edited round-tripped one). PES 15/16/20 texports have no
  fixture at all (offsets/key index are the reference's; PES 20's size is derived).
- open — FoxDen's LiveCPK gaps that sideload mode depends on (forwarded by the maintainer to
  FoxDen's maintainer, 2026-10-03; source `Tools_4cc/FoxDen/src/livecpk.c`, `patterns.c`):
  (1) the call sites are found on PES 2018 only, 2019 and 2021 still to locate; (2) PES 2020 is
  not recognised (proxy only); (3) a served path is limited to about 99 characters (redirect plus
  name in the read request's 0x80-byte field; a face texture's folder alone is 59), longer names
  silently fall back to the CPK; (4) only boot-time shader loads are verified in game, model,
  texture and kit-bin loads under `livecpk\` are not (Sider's buffered-copy and CPK-size hooks
  were not ported). Phase 4's sideload acceptance waits on them; recheck FoxDen when itemizing.
  FoxDen session 3 (2026-10-14, PES 2019 on the stock 1.6.0.0 exe, scratch
  `.tmp/release_0_0_2/foxden/plan.md`): (5) FoxDen's own build runs proxy-only on every PES 2019 exe here:
  `host_is_game()` wants a static `dinput8.dll` import, which neither exe has, on disk or in
  the running image (the game loads DirectInput at run time); (6) with that check relaxed
  (test copy), the game dies right after the livecpk Lua state is created, on the stock exe as
  on the patched one: `lua_tolstring` resolves to a misaligned address (RVA 0x4bbba5f, the
  pattern's second pass), most likely mid-function. FoxDen session 4 (2026-10-20, its new
  version on the same stock exe, scratch `.tmp/release_0_0_2/foxden/plan.md`): (5) and (6) are fixed (the
  host check reads the version resource; `lua_tolstring` resolves aligned), the game runs
  with every script loaded, and the kit-swapper script works on a Studio CPK: kit 1 in Edit
  mode and kit 2 in a match each load the per-kit face package (`<id>p<K>`) or the `kitK`
  texture, falling back to `kitN` when neither exists. Not yet checked: the team as away,
  boots and gloves packages, `dummy_kit`, a `kitN/` folder set.
- open — a run that does not end cleanly leaves its staging (3.z): closing the window while
  the GUI's compile runs ends the process mid-run, and the coordinator's `Cancelled` path does
  not discard the staging either, so `output/.staging/<run>/` can be left beside the previous
  CPK (which stays intact). Fixed with cancellation (`spec/core/gui.md` "Cancellation", Phase 8): the
  window stops the run and the cancelled path calls `deploy::discard`. The same step adds the
  check between a texture task's textures (4.6's slice (c)); a test cancels a real run partway.
- open — a texture link beside a combined shared folder's texture of the same stem (4.11d):
  a player holding `hair.dds.common` and a link to a shared face whose folder holds
  `hair.dds` compiles with no finding; the shared folder's copy wins, the link is ignored,
  and `hair` is packed twice (the player's home and the team's Common output).
  `texture_stem_conflict` covers the player folder's own files alone. The plan is silent;
  the likely rule is a conflict finding, since collisions are rejected and never silently
  resolved, and it belongs with the stem namespace in `aesthetics_export` or with the
  textures task's conflicts. Rare: it needs a shared folder combined into a player who
  also links one of its stems to `Common/`.
- open (2026-10-09, 4.y-fix4) — the deep pass answers "does this link feed the player's own
  package" with an empty hand-weighted set (`deep/mod.rs` `FaceUse::of_player`), because it
  is the pass that finds the hand-weighted models, while planning and validation answer
  with the real set. For a Fox player whose gloves link combines only through his split
  hands, the linked folder's textures do not count for his `.mtl` paths there, so a
  `mtl_texture_not_found` Warning may be spurious (read, not run); nothing is dropped.
  `FaceUse::combined`'s doc comment ("Empty for a team player ...") was already untrue on
  Fox. Fix: find the hand-weighted models before the face use is built, or build the face
  use after them.
- open (2026-10-11, 5.0e review S3) — a disallowed file a lenient run keeps in a `Common/`
  subfolder no link reaches (`Common/sub/x.doc`) is reported twice on Fox:
  `common_file_disallowed` and `file_not_used` (`validation.rs`, `read_by_no_task`'s
  `!is_read_common_file` arm), where `messages.md` `file_not_used` says a file the
  structure pass already named is not reported (read, not run). Fix: skip a file the
  structure pass reported.
- open (2026-10-15, step 20.0c) — a `stat_bonus` on a stat a tier already targets at 99 gives
  an exact target no player can reach (99 is the game's top), and nothing says so: the first
  Clouds template's "Medals at giga: +2 to every stat" made every gold 101, so `check-aatf`
  placed no player in gold (a TierCount violation on every VGL26 team) instead of naming the
  rule. `aatf_rules.md` is silent. Settle with the next interpreter step: clamp a raised target
  at 99 (the bonus then means nothing for that tier), or refuse it on load where a tier's
  value plus a special's bonus passes 99 (a special's condition may never meet that tier, so
  a load rule could refuse a playable file). The same logic version change fixes the Phase 5
  converge audit's finding (`.tmp/phase_05/5_6/audit_shapes.md` row 1.7): an explicit `()` on an
  optional `Condition` or `Effects` key (`captain: ()`, `fielded_only_at: ()`, `forbid: ()`)
  reads as `None` in Rust but as present in the logic, which tests the key's presence; the
  logic's `num`/`list`/`table`/`flag` helpers already treat `()` as absent. Only a hand-written
  file reaches it.

- open 2026-10-29 — the 120 s hang guards in the Save editor's tests (`frames_until`,
  `portraits`' `served`) cost a mutation run up to 120 s per mutant that silences a worker
  (they were 2 s and 30 s before, both failing correct code under load). At the next
  `save_editor` mutation run, read those mutants' durations from its outcomes; if they add
  notable time, lower the bound to the longest a loaded machine was measured to need
- open 2026-11-02 (f1g9a) — `studio_core/src/shell/mod.rs` reached 1313 lines with the update
  check's poll and its three tests (1089 before; past the thousand already). After 0.0.1's
  cut, move the update poll and its tests beside `updater.rs`, and the shell's tests into a
  `tests.rs` of their own, as `settings_menu` has

---

## Log

The dated log is `docs/LOG.md`.
- 2026-11-05 — 0.0.2-poc1/poc2 (the maintainer's answers): a custom boots or gloves ID no folder holds
  becomes the hide ID with a Warning (`custom_model_hidden`), and Fox's missing texture is a Warning keeping
  the package. All 53 VGL26 exports run on copies first: 31 upgrades refused, 279 players. SWE-2 sidekick.
