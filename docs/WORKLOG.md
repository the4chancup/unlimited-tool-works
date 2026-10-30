# 4cc Studio — Worklog

Where the implementation is and what comes next. The plan (`docs/plans/`) holds the *why*;
`DECISIONS.md` holds choices made where the plan was silent; this file holds *where we are*.
Never duplicate rationale here — link to the plan section instead. Procedure for using this file
is in `AGENTS.md` ("Working documents").

---

## Current status

**Phase:** 8 (GUI) is next in the phase order, after the DBC work below. Phase 6 (Export
upgrader) closed 2026-10-20; Phase 5 (Savefile integration) 2026-10-17. Phases 1 to 4 done
(Phase 4 closed 2026-10-12, Phase 3 2026-10-02, Phase 2 2026-09-30). Built ahead of their
phases: 10.0a-g (`music_export`, its plan section, `pub` audit and whole-crate mutation run
done, then matched to Rigdio's reading by the 10.0f audit; converging with Phase 10), 16.w (the platform floor) and 20.0a-d (the Ruleset editor's
coverage table and draft, the DBC7 ruleset).
**Next:** for Divegrass Before Christmas (exports due 2026-11-18; `plans/core/development_plan.md`
"Divegrass Before Christmas"), a working Save editor with a rock-solid AATF check comes first
(the maintainer, 2026-10-24): the Save editor's view (8.9: slices 1-3 done, with the Tactics
pitch as the game's, 8.9g and 8.9i; its slice 4 under way: the AATF panel and Live AATF done
(8.9j1-j3), the comparator done (8.9j4), the save-writing transplant removed (8.9j5c; the
export-writing one waits for 0.1.0); 8.9f's slice 4: v2 is
the base, ported in five subslices, 4a the frame, 4b Abilities and 4c Appearance and Tactics done, 4d (the one cards badge, on 0.0.1i's `Rules::player_tier`) done and reviewed; 8.9h closed by the window's 960 × 600 minimum; 8.29, the text pass, under way: a (the Save editor's
window, catalog, CLI, help and draft) done, c (the Team compiler and the upgrader) under way
on side-lead U's branch, c1 to c5 done, the help chapters' fact audit (c4e) included, c6 outside its locks; 8.9l, the open and save scenarios, done 2026-10-29
(SE-GUI-21 checked in game on PES 2021 and 2017, its texts corrected in 8.9m m5); 8.9m, the logic's speed
rewrite before the v1 freeze; 0.0.1j's audit read, j1 and j2 open; the maintainer's
proposal rulings of 2026-10-26 added 0.0.1o-r, k4 and 8.7c), `libs/aatf`'s hardening (the repetition rework done and
reviewed; the audit 0.0.1i done and reviewed), `ingame_face.toml` (0.0.1h, done 2026-10-25: the compiler,
the upgrader and the Save editor's generation on the new file, SE-GUI-34 checked in game), then the 0.0.1 cut (0.0.1f, its preparation f1 done on side-lead U's branch: the Save editor, DBC7's rules file, and the Team compiler
and Export upgrader through their CLI for VGL 27, PES 2019, from late November; 0.0.1a-e and g
done), then Phase 8's other views and Release 0.1.0. The Ruleset editor (Phase 20) left 0.0.1
and resumes after 0.1.0: 20.3-20.5, 20.6a1 and 20.6a2a (Tiers) done and reviewed. Done for DBC so far: 20.0d-e (the DBC7 ruleset and its organizer's choices), 8.4
(the usability pass; P1-P14 accepted 2026-10-25, planned as 8.10's row actions and 8.22;
`PROPOSALS.md` P15-P22 wait for the maintainer's rulings), 8.5 (the Save
editor's draft), the shell (8.6a-e), the color picker (8.7a-b), `team_widgets` (8.8), egui 0.37
and the release stage (8.11, 8.13). The face_diff model comparison (Issues, "a face diff is
engine-specific") follows the editor. The shell's own icon font (8.14) is built, its look in
the built window open.
No side-lead runs: tracks A (the drafts, 8.16-8.19a and 8.25's half) and B (0.0.1i) ran on
2026-10-25 and are merged; the record is `.tmp/lead/tracks.md`. 2.5b (GPU BC7) is step 16.x, its `dds_convert` half built ahead of its
phase (16.x a, unwired)
(decision entries 2026-09-21 and 2026-09-28). Release target (2026-09-28): 0.1.0 after Phase
8; phase order 1–6, 8, 0.1.0, 7, 9–16 (`plans/core/development_plan.md` "Releases");
first-class target the Fox version the cup moves to around April 2027 ("Target versions").
**0.0.1 candidate** (2026-10-29): the overlay pass (0.0.1ov) done in both themes; the
candidate waits for the maintainer's final UI pass (0.0.1f1c, `QUESTIONS.md`, branch
`release/0.0.1-candidate`); the work goes on past it.
**Blocked on:** nothing.

**Working mode (since 2026-10-10).** The lead (Claude, in Claude Code) plans, briefs, reviews
and writes what is correctness-critical; the sidekick, SWE-2 through the Devin CLI, implements;
the cross-family reviews run through the `duck` skill (Astra), the lead running them itself.
When a review stops for quota, it is queued here (commit range, plan sections) and retried
once an hour; work continues past it, but no phase closes with one outstanding.
**Handover queue:** empty; every review Phase 5 queued has run (rulings under `.tmp/5_*/`).

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

---

## Phases

Status: `todo` · `in progress` · `done` · `blocked`. A phase is done only after its last two
steps, which every phase has: **converge** (the lead's own audit first, then the cross-family
reviewer's, both against the phase's plan sections and acceptance IDs; each gap becomes a new
step above it, and the phase waits for them) and **rewrite**
(what the phase delivered moved from `docs/plans/` to `docs/spec/`, corrected where it was built
differently; `AGENTS.md` "Working documents"). Then collapse its step list
below to this one row, keeping its `manual: checked` lines (the step-level detail stays in git
history), and bring the root `README.md` to the closed phase. Tool phases (3–6, 8–15, 17, 19, 20) also
open with an **acceptance** step: the tool plan's "Acceptance" section for that phase, written
before any code (GUI scenarios that no automated test can prove are marked `manual` and proven by
a recorded check at converge — `CONTRIBUTING.md` "Testing"). Procedure: `AGENTS.md` "Working
documents".

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
| 0.0.2 | Preview build: 0.0.1 plus the Team compiler and Export upgrader through their CLI for VGL 27, with the launchers (`core/development_plan.md` "The /vg/ League 27 (VGL)") | `just release`, the launchers, the console's progress line | todo |
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
manual: checked 2026-10-09 by the lead on PES 17 through the harness (`.tmp/4_19/ingame/`, results in `.tmp/4_0/apptest/results.txt`): the referee present with the pair installed, nothing drawn; a `.mtl` beside a nocloth `.model` is read and stops the model drawing (runs E and F against 2026-10-08's run I), so the pre-Fox marker goes by Red's route instead: step 4.19f, DECISIONS 2026-10-09; the scene where the pre-Fox marker shows is a maintainer question

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
(`.tmp/4_y/ingame/build_portraits_cpk.py`), each hovered alone in Kick Off's team
sheet after its neighbour (the maintainer's method): A (DX10 header, BC3, sRGB id,
one level) and B (the same with the UNORM id) crash the game on hover; C (the
legacy `DXT5` header) and D (BC7 under DX10, full chain) draw
(`v21_portraits_sheet.png`). So the DX10 header itself, not the colour-space id,
crashes the Fox games' BC3 portrait reader, and the compiler's re-headering (4.y-fix6)
is the right fix; BC7 stays under DX10, which draws. The cup's eight BC7 portraits (ESG
XXX07-11, FGOG XXX13/21/23) are 128x128 with eight levels under DXGI 98, D's shape, so
nothing of the question is left (its QUESTIONS entry removed).
manual: checked 2026-10-10, collars beyond the stock set on PES 21: Test 3's CPK
(`.tmp/4_0/apptest/out_test3/pes21/`, team `/out/`: kit 1's collar 200, a copy of the
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
`.tmp/4_y/ingame/build_tagtex_export.py`.
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
`.tmp/4_y/uniform_mtl/`) each as `Collars/collar_012.model` of an `/out/` export with
Test 3's kit 1 (`.tmp/4_y/ingame/build_collar21_export.py [050]`, the release build of
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
test slot (`.tmp/4_y/ingame_f/build_subtex_export.py`, compiled by the release build of
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
`.tmp/4_y/ingame/frames/p20/`) is 14760 bytes (`0x39A8`, the derived size) with key
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
ankle, above the boot (`.tmp/4_y/ingame/frames/p21sock/s06_front.png`,
`s07_side.png`, `s08_back.png`, against `s17_34_out_legs.png`); with the upper part
checked 2026-10-10, the converted sock matches PES 17 feature for feature. (a) closed.
manual: checked 2026-10-12, QUESTIONS "Non-power-of-two mipped textures and kits over 2048
on Fox": Red (`e12aa01`, a scratch copy under `.tmp/5_0e/red_np2/`, `setup.py` and
`retarget.py`) compiled EGG VGL26's `Faces/XXX14` and AKG VGL26's kits as `/3/` (the
save's /vg/ teams are placeholders that draw no custom kit, `s06_vgl5.png`); its FTEX keep
the chains (`screen` 512x461 and `text` 797x64 ten levels, `u0701p1` 4096 thirteen). In
Edit mode row 14 (70114) draws the Steam page and the caption as the sources show
(`.tmp/4_y/ingame/frames/p21np2/s09_row14_face.png` against `screen_source.png`), and row
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
bonfire, triage in `.tmp/5_6/close/mutants_<crate>/triage.md`): `pes_savefile` 1830 caught, 2
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
and a sandbox PES folder (`.tmp/5_5/studio_sl/`): with PES 21 running, sideload compile
exit 0, the patch's `cpks = []`, `savefile_skipped_pes_running`, the copy byte-identical;
with only PES 17 running, `savefile_written` and the copy's `.bak` kept

SE-CLI-02 (withdrawn at 8.9l5 for SE-CLI-03, which tests prove), checked by hand 2026-10-15, with PES 21 running: `import-toml`, `apply-patch`,
`transplant`, `strip-and-seed` and `check-stripped --strip-again` each refused with exit 2
naming PES 2021, no `--out` written; with only PES 17 running each wrote its `--out`
(`.tmp/5_5/cli02/`)

5.4c2 (`strip_base_copied_face`):
manual: checked 2026-10-15, PES 21 Edit mode (`.tmp/ingame/face_check/`): a stripped
base-copied player shows Placeholder; the copied face shows with the face flag clear too

5.5 (the in-game check; runs, shots and saves in `.tmp/5_5/`, by a305229's release exe; the
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
attributed but two `red_parity` pairing ties, `.tmp/6_close2/`), the design sweep over the code
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
    manual: checked 2026-10-19, PES 21 Edit mode (`.tmp/6_16/ingame/`, the export upgraded
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

- [x] 8.0 plan fixes before itemizing (2026-10-18, side-lead C, branch `side/8.0`): the
  "Shell layout" and "Event wiring" sketches now match the spec (tools draw their own panels
  inside the central panel and drain their own sinks in `tick`); the draft-first rule is in
  Phase 8's bullets and each tool plan's build order (`shell.html` first, one page per tool);
  the watcher and the check cache have one home, `libs/pipeline` (DECISIONS); "run strip" is
  the one name (glossary row); `color_tools`' widgets are a Phase 8 bullet, the picker's first
  host the Save editor's color fields (DECISIONS); the Export upgrader plan has a "GUI"
  section; acceptance sections written: `ST` (`plans/core/gui.md`, a new code, CONTRIBUTING),
  `TC-GUI-04` to `-21`, `SE-GUI-01` to `-28`, `EU-GUI-01` to `-06`; TC-GUI-01 narrowed to the
  window and TC-GUI-02/03 automated, cited by the existing headless tests; `eframe` and `egui`
  pinned `=0.36.2` (lockfile unchanged, `cargo tree --locked`); the spec caught up
  (`PipelineEvent::Output` and its paragraph, `pes_process.rs` and the probe paragraph, the
  three registered tools; the plan's registry gained `team_creator`, `ruleset_editor`,
  `db_generator`). The Save editor's spec tree already listed `events.rs`, `diff.rs`,
  `teams_list.rs` (no edit). The upgrader's handoff to the compiler is a question
  (`QUESTIONS.md`). Reviews (checkpoint a; rulings `.tmp/8_0/rulings.md`): SWE-2 V and S1-S6 on
  the branch (S6 4), then on main Astra 8.0.A1 8 accepted, S7 6, S8 6, S9 4, Astra 8.0.A2 3,
  S10 0, every loop ended (2026-10-19, top lead): the colors drawn by storage kind, the team
  colors' 6-bit conversion; a run's events and verdict wake the GUI through the `ToolContext`;
  every close asks every tool (unsaved work, a run in flight), resumed through a `ShellRequest`;
  the GUI's Texport export on PES 18-21; new scenarios SE-GUI-29, ST-RUN-05/06, TC-GUI-22/23,
  EU-GUI-07 (DECISIONS 2026-10-19). Files: `Cargo.toml`,
  `docs/CONTRIBUTING.md`, `docs/GLOSSARY.md`, `docs/plans/core/{gui,architecture,development_plan}.md`,
  `docs/spec/core/{gui,architecture,development_plan}.md`, `docs/spec/libs/pipeline.md`,
  `docs/plans/team_compiler/{README,gui}.md`, `docs/spec/team_compiler/README.md`,
  `docs/plans/{save_editor,export_upgrader}.md`, `docs/plans/libs/color_tools.md`,
  `crates/studio_core/src/shell/mod.rs`, `crates/studio_core/src/pes_process.rs`,
  `crates/tools/team_compiler/src/{view/mod,gui_run}.rs` (citation and pointer comments only)
- [ ] 8.1 egui's bundled fonts, once the theme settles its icon font (the shell's own, 8.14;
  egui 0.37 also drops `default_fonts` from egui's default features, 8.11): the
  four default fonts are 1.37 MiB of the release exe's `.rdata` (2026-10-16,
  `.tmp/16w/rdata_parts.py`), the two emoji fonts (`NotoEmoji-Regular` 409 KiB,
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
- [x] 8.4 the usability pass before the tools' drafts (2026-10-19, the maintainer's request;
  `plans/core/development_plan.md` Phase 8's bullets): after Phase 6 closes and before
  `team_compiler.html`, the lead reads what members already touch (the export format's folder
  conventions, the compiler's and upgrader's message texts, defaults, the upgrade → compile →
  install handoffs) for usability ideas, each a `PROPOSALS.md` entry → verify: the entries are
  written and the maintainer has had them before the first tool draft starts. Done
  2026-10-20: `PROPOSALS.md` P1-P22, curated from 25 candidates a reading subagent gathered
  (`.tmp/8_4/candidates.md`; left out: the face engine marker, which waits for the face
  transformation; a narrower `portrait_conflict`; model-folder mode placing into an export;
  a `settings.toml` template, which the aesthetics editor covers), named in the turn report.
  Files: `docs/PROPOSALS.md`
- [x] 8.5 the Save editor's draft, `docs/prototypes/save_editor.html` (`plans/save_editor.md`
  "Build order and verification"; first of the tool views, for DBC), drawn whole in five
  slices, one per build-order slice, each reviewed before the next is briefed. Drawn before
  the maintainer has ruled 8.4's proposals: the two that touch it (P15, compile findings
  opening the Save editor; P22's `check-aatf` names) add to it and go in once ruled, as a
  slice that changes the draft does → verify: every SE-GUI scenario but the manual ones walked
  in a headless-Edge harness against the page, real sample data (the maintainer's PES 2017
  and PES 2021 saves through `export-toml`), only representable elements
  - [x] 8.5a the page, open and save, the team selector and player list, the info strip's
    quick actions (official, VTL11, DBC7), Abilities & Skills, Appearance (SE-GUI-01 to 09,
    20, 21, 26, 28). Done 2026-10-20 (SWE-2, one rework: two NUL bytes, the sample data one
    player per line, the PES 17 save's placeholder team 701 dropped, the gated controls
    dimmed on screen and the check reading the element): samples from the PES 2017 save
    (702 `/a/`, 718 `/f/`, whose names carry colour codes) and the PES 2021 save (701, 718,
    722) through `export-toml`; harness `.tmp/8_5/run_harness.py`, 15 checks. Drawn for the
    maintainer's eye, each marked in the page: the quick actions follow SE-GUI-09 (one per
    tier, Goalkeeper included) where the sketch shows four; the list's number is the shirt
    number; the Open menu lists discovered saves, then recent files, then the picker; the
    palette swatches' colours are invented (no palette table exists in plan or code), and
    nationality is a number. Files: `docs/prototypes/save_editor.html`
  - [x] 8.5b the Team tab, the team and player operations, FPC (SE-GUI-10, 11, 22 to 24, 27).
    Done 2026-10-20 (SWE-2, one rework): Player, Team and Savefile menus; the boots and gloves
    dialog in 4ccEditor's shape (`bogloDlgProc`); the FPC toggle reading `ops/fpc.rs`'s
    classifier; Output rosters to TSV and Fix database as 4ccEditor's `roster_data_output` and
    `fix_database()` (DECISIONS 2026-10-20, SE-GUI-27 reworded); slice 1's captain checkbox
    off by one fixed. PES 17's kit-ID repair touches container bytes the draft does not model:
    named in the confirmation. Harness 21 checks
  - [x] 8.5c the Tactics tab (SE-GUI-12, 25). Done 2026-10-20 (SWE-2, a fresh session after
    the resumed one stalled at ~20 minutes per turn): the pitch one Painter canvas, markers
    dragged on the stored grid (x 0 to 104, y 0 to 48, 4ccEditor's range on 16 to 21; the
    samples' 42570 slots sit inside it), the slot popover, XI and bench drag lists whose swap
    repoints takers, join-attack and instruction targets, the role palette and join-attack
    assignment modes, the copy/swap menus; slice 1's card badges (per tier, from each ruleset's
    `cards`, `a_positions`, `free` and free-card specials; a badge reads low, never high) and
    slice 2's bound-id width. Drawn for the maintainer's eye: the formation selector stays
    enabled with fluid off (its tooltip gives the game's rule), auto substitution is the
    model's raw setting byte, man marking is carried but not drawn (as in 4ccEditor). Harness
    24 checks
  - [x] 8.5d the comparator, transplant, AATF and patch panels (SE-GUI-13 to 15, 18). Done
    2026-10-20 (SWE-2): the panels as floating windows from a Panels menu, the AATF one also a
    button beside the quick actions; real samples made with the HEAD build
    (`.tmp/8_5/samples/`: `diff` of two PES 17 saves, a transplant and an `apply-patch` written
    to scratch copies and diffed, DBC7's `check-aatf`, two broken rules files); comparator rows
    in the tabs' terms with each field's offset measured on the samples (1-based motions +1,
    physique -7, no row disagreeing); one sentence per violation kind, marked sample wording
    (the CLI prints codes). DBC7 defines no `suggestions`, so its toggle shows none (shown
    under the official ruleset). Harness 28 checks. Three follow-ups ride on 8.5e's brief:
    player names on the panels' rows, the all-teams AATF sample and the comparator's stored
    report lines trimmed (the page grew to 873 KB)
  - [x] 8.5e the interchange dialogs and settings.toml generation (SE-GUI-16, 17, 19). Done
    2026-10-20 (SWE-2, one rework: the harness tail's output line lost in an edit, and every
    team's export text embedded where SE-GUI-16 checks one, the page at 2.5 MB): one import
    dialog for Team TOML, `.4ccs`, `.4cct` and Texport, its sections split as 4ccEditor's squad
    load (DECISIONS 2026-10-20), PES 21 files onto the PES 17 save with the CLI's 189 warnings
    and its two refusals byte for byte, the real `2HU.4ccs` and `IZA.4ccs` decoded with their
    record-to-player pairing (2HU refused on 17: a 20-byte shirt name), Texport export disabled
    on 15 to 17; `settings.toml` generation a player action, coded names written explicitly.
    Slice 4's follow-ups: player names on the panels' rows, the AATF and comparator samples
    trimmed. The page is 1.2 MB, real data: the samples with face and hair values (552 KB) and
    team 718's real export text (265 KB). Harness 31 checks
- Phase 8 opens (itemized 2026-10-20). The shell is Phase 3's slice (a label list and the
  active view, `studio_core/src/shell/mod.rs`); no `ST` or `SE-GUI` scenario is proven, and
  neither `libs/team_widgets` nor `color_tools`' widgets exist. The steps run in DBC's order
  (`plans/core/development_plan.md` "Divegrass Before Christmas"): what the 0.0.1 preview needs
  first, the Team compiler's and the Export upgrader's views after it. Each step builds against
  its draft (the host page `docs/prototypes/index.html` and its `tools/*.js`) and its plan, and cites its scenarios in tests
  through the shell's headless harness (`shell/mod.rs`'s tests). The drafts wait for the
  maintainer's remarks, so the steps that draw least of what a remark would change come first
  - [ ] 8.6 the shell (`plans/core/gui.md`, `spec/core/gui.md`; draft `index.html`), sliced
    - [x] 8.6a the sidebar: the planned-tool registry and its greyed entries, which 0.0.1
      also uses for the Team compiler and the Export upgrader (DECISIONS 2026-10-20), the
      logo's About block, the PES selector with its exe check and blocker, the icon rail
      (`egui-phosphor` registered in `theme.rs`), `last_tool` (ST-SIDE-01 to 04, 07,
      ST-RUN-02); the `?` and gear buttons are wired with their windows (8.6d, 8.6c). Done
      2026-10-20 (SWE-2, one rework): `shell/sidebar.rs`, `shell/about.rs`, `theme.rs`, a
      `build.rs` for the target triple, `PlannedTool` and `StudioTool::icon()` in the three
      tools, `egui-phosphor =0.14.0` (egui 0.36's release); the selector's colour one function,
      only files counted as the exe (as the compiler's preflight), the tests' PES folder their
      own. `mutants-diff a720c13`: 41 caught, the tools' `icon()` pinned (`studio`'s registry
      test catches them). Review S1 (SWE-2): 3 accepted of 12, rulings `.tmp/8_6/duck_rulings.md`;
      Astra ran over 8.6a and 8.6b together (A1, in 8.6b)
    - [x] 8.6b the status bar, the window title and the tools' wake callback (ST-BAR-01 to 03,
      ST-TTL-01 to 03), the `data_dir_read_only` condition a failed save raises (ST-SET-03;
      ST-CLI-01), the running-PES auto-switch and its "Fix PES path" notice (ST-SIDE-05,
      manual). Done 2026-10-20 (SWE-2, four reworks): the harness out of `shell/mod.rs`
      (`shell/test_harness.rs`, the sidebar a folder with its `tests.rs`); `shell/status_bar.rs`,
      `shell/title.rs`, the wake slot in `ToolContext` (`emit` and every shell request wake the
      GUI; `request_repaint_after` for a tool's deadlines), the poll in the sidebar (every 5 s
      while unspent, the probe's first game, a blocker deferring it, the action only for a
      folder on this system, `FixPesPath` carrying its version), the CLI's probe and notice
      printer; `Settings::save` edits the file in place (`toml_edit`), and
      `pes_install_missing` is gone (DECISIONS 2026-10-20); a save whose edit would not read
      back as the settings writes the file fresh. The review found a hang: the notice
      printer held stderr's lock, so any warning blocked a CLI command (reproduced, fixed, a
      deadline test). Review: S1-S4 (SWE-2) and A1 (Astra over 8.6a and 8.6b), rulings
      `.tmp/8_6/duck_rulings.md`; `mutants-diff 59216630`: every survivor tested or pinned
      (three pins). Open: ST-SIDE-05's manual check (a PES running, F: connected)
    - [x] 8.6c the settings menu, the first-run dialog and the close hook, sliced
      - [x] 8.6c1 the settings menu: the sidebar's gear, the common rows (the theme applied
        live among them) and one section per tool with settings (ST-SET-01, 02); a
        notice's `OpenSettings` action opens it; its `pes_version` row spends the running-PES
        auto-switch as the selector's pick does; `settings_view` takes the context
        (DECISIONS 2026-10-20). The rows' Browse buttons wait for `rfd`, which the Save
        editor's file open brings (8.9); the tools' own sections are their views' steps
        (the Save editor's `aatf_rules` row with 8.9, the Team compiler's with 8.10). Done
        2026-10-20 (SWE-2, first time): `shell/settings_menu.rs` (a `Modal`, the common grid,
        the sections reset at each opening through `CollapsingHeader::open`), the gear and
        `SidebarClick`, `Sidebar::pick_version` shared by both rows, `theme::apply`
        (`set_theme`, applied in `logic` on change); the lead's fix: the number rows clamp
        only entered values (egui's default clamp rewrote a hand-set `thread_count` on
        sight). `mutants-diff 42f7c7e6`: 2 survivors (`theme_label`), both caught by an added
        assertion. Review S5 (SWE-2) runs over the commit
      - [x] 8.6c2 the first-run data-location dialog (ST-RUN-01) and the close hook for
        unsaved work and runs in flight (ST-RUN-05, 06), which the Save editor's unsaved
        prompt (8.9) and the Team compiler's run (8.10) use. Done 2026-10-20 (SWE-2, one
        rework): `shell/first_run.rs` (a `Modal` with no Cancel while `data_dir` is `None`,
        the choice saved through `Settings::save`, a failed write keeping it open with the
        error; `StudioApp::new` takes the config folder to offer), `StudioTool::hold_close`,
        `ShellRequest::Close` and `ToolContext::request_close` (the close asked in sidebar
        order, the answer back through the window's own close); the shell tests' folders a
        self-removing `TestDir` (they leaked, and three tests shared one); S5's menu fixes.
        `mutants-diff fe5d4af2`: `hold_close`'s default caught by an added assertion, the
        browser stand-in's `up` pinned. Reviews: S6 (SWE-2, 3 of 9 accepted), A2 (Astra over 8.6c1
        and 8.6c2, 4 of 6: a held close closes the settings menu over the prompt, three tests
        made machine-independent), S7 (SWE-2 over A2's fixes, 1 of 5: the About popup closes too), rulings
        `.tmp/8_6/duck_rulings.md`
    - [ ] 8.6d the help window (and the About popup's help link, completing ST-SIDE-06), its
      generated Messages topics and search, `4cc-studio help`
      (ST-HELP-01 to 04), and the log panel widget (ST-LOG-01 to 04); a notice's `OpenHelp`
      action opens the window at its target (8.6b leaves it a dismiss)
      - [x] 8.6d1 the catalogs' member texts (`When`, `Effect`) in each tool's `messages.rs`,
        copied from the spec's tables; the core chapters `setting-up` and `first-run`;
        `4cc-studio help` and `--export` (ST-HELP-04). "Updates" and "What's new" wait for
        the update check and `CHANGELOG.md` (Release 0.2.0). Done 2026-10-20 (SWE-2):
        `CatalogRow` and `HelpSection::messages`, `help::chapters`, `help/export.rs`
        (`chapter_markdown`, `topic_markdown`, `run_help`), `LaunchMode::Help` (clap's own
        `help` subcommand disabled, `help` no tool's id), `Severity::name`; the rows filled by
        `.tmp/8_6/copy_catalog_texts.py` (234 compiler rows, 25 editor, 72 upgrader of which
        47 take the compiler's cells and 18 their code's doc comment), the spec's em dashes
        rewritten. Lead fixes: three upgrader rows whose doc comment split the wrong way, the
        compiler's table moved to `messages/catalog.rs` (data, 1415 lines), the tools' topic
        files' own `#` titles dropped. Files: `studio_core/src/help/`, `shell/launch.rs`,
        `events.rs`, `studio/src/main.rs`, `studio/tests/help_cli.rs`, each tool's
        `messages.rs` and `help/01_command_line.md`. `mutants-diff e45c9672`: 43 mutants,
        `CatalogSeverity::text`'s two missed, caught by an added test of each severity kind's
        spelling. Reviews: S8 (SWE-2, 5 of 10: a spec row wrapped over three lines and its
        copy two spaces short, every compiler row then compared with the spec; a Red state
        file; the upgrader's two shared codes given its own cells), S9 (1 of 4: one of them
        restated), rulings `.tmp/8_6/duck_rulings.md`; Astra waits to batch with 8.6d2
      - [x] 8.6d2 the help window: `egui_commonmark` `=0.25.0` (the last release on egui
        0.36), the chapter tree, the Messages topic, search, in-doc links, F1 and `?`, the
        About popup's help link, a notice's `OpenHelp` (ST-HELP-01, 02, ST-SIDE-06). Done
        2026-10-21 (SWE-2, one rework): `shell/help_window.rs` (`HelpWindow` over
        `help::chapters`, a non-modal `egui::Window` with the search box, a `Panel` tree and
        a `CentralPanel` pane, since a `ui.horizontal` row clips its scroll areas out of the
        hit test; topics drawn a paragraph per `CommonMarkViewer` call so a search hit's
        paragraph is the one highlighted; the Messages topic a wrapping `Grid`; link hooks
        for every `<id>#<slug>`), the sidebar's `?`, F1, the About popup's Help link,
        `ActionEffect::OpenHelp`, each opening closing a shell modal; geometry for the
        session only (DECISIONS). ST-HELP-01, 02, ST-HELP-03's in-doc half and ST-SIDE-06
        proven. Files: `studio_core/src/shell/{help_window,mod,about,test_harness}.rs`,
        `shell/sidebar/`, `studio_core/Cargo.toml`, the workspace `Cargo.toml`. Review A3
        (Astra on Codex brother, over 8.6d1 and 8.6d2, 6 of 8 accepted: a jump consumed by
        a `Grid`'s sizing pass, the width budget on the text columns alone, the whole row
        highlighted, the upgrader's shared rows given its own effects, the catalog cells
        through the Markdown viewer instead of a hand-rolled one, three functions narrowed
        to `pub(crate)`) and `mutants-diff 14c1f645` (81, 29 missed: tests for the tree's
        selection, the scroll to a hit, the jump's scroll to the top, the widths, the
        colors, the fallback chapter; the About popup's redundant close dropped), both in
        rework 2. Review S10 (SWE-2 over rework 2, 4 of 8 accepted: a When cell's link read
        after the Effect cell had reset it, a tree click while searching kept the results
        up, a stale module doc, width checks AccessKit's clipping made unfailable) and the
        12 survivors of `mutants-diff 14c1f645` over it, in rework 3: the width arithmetic
        in `text_column_width` with a unit test, the row-jump tests proving the pane
        moved, and the sizing-pass guards tested one by one (only the one keeping the
        scroll pending across a `Grid`'s sizing pass is needed; the others went).
        Rulings `.tmp/8_6/duck_rulings.md`; Astra A4 over reworks 2 and 3 is due
      - [x] 8.6d3 the log panel widget (ST-LOG-01 to 04) and its code links (ST-HELP-03):
        `widgets/dock.rs` `BottomDock` (the bottom panel's frame: chevron, title, the caller's
        header, the resizable divider; a second panel id while collapsed keeps the open
        height), the frame the Save editor's Live AATF list reuses; `widgets/log_panel.rs`
        `LogLine` (`from_event`, `plain`) and `LogPanel` (300 lines drawn, All / Warnings /
        Errors, colored by severity, the follow paused while scrolled up with `↓ N new`, the
        code a link); `ShellRequest::OpenHelp`, `ToolContext::open_help`,
        `HelpTarget::message`; the Team compiler's window draws its run in the panel. The
        grid cross-link waits for the grid (8.10). Also a test of the catalog grid's laid-out
        width (the two survivors of rework 3's mutation run). ST-LOG-01 to 04 and ST-HELP-03
        proven. Files: `studio_core/src/{widgets/,tool.rs,lib.rs,help/mod.rs}`,
        `shell/{mod,help_window,test_harness}.rs`, `team_compiler/src/{gui_run.rs,lib.rs,
        view/mod.rs}`. Review A4 (Astra on Codex own over 8.6d2's reworks 2 and 3, 2 of 3
        accepted: a long catalog cell overlapping the next row, in 8.6d3's rework; the
        upgrader's `common_link_missing` row promising an exception it never applies) ends
        Astra's loop on the help window. Review A5 (Astra on Codex brother over the log
        panel's reworks, 4 of 4 accepted) reworked in rework 3: a paused panel draws the lines
        it held when the pause began, past the cap too; a line never wraps; the jump and
        collapse tests observe their THENs. Astra's loop on the log panel ends. Review S12
        (SWE-2, 2 of 9 accepted: the no-wrap test asserts the width, the paused test the jump
        landing) reworked in rework 4, the tests moved to `widgets/log_panel/tests.rs`; the
        log panel's reviews are done, and the mutation run since rework 2 left it no survivor
      - [ ] when the first topic gains an in-doc link `[text](<id>#<slug>)`, `--export`
        rewrites it to `<id>.md#<slug>` and writes an anchor per topic slug (review A3-7: no
        shipped topic holds one yet, so the exported files have nothing to break)
      - [x] 8.6d4 a member wording pass over the three catalogs' texts, with the maintainer's
        remarks on the draft, to `CONTRIBUTING.md`'s "written for a layman" rule (as easy to
        understand as possible for a member who knows nothing of the code): the copied cells
        carry dispositions (`DropExport`) and pointers
        into `docs/` a member cannot follow, and name Red (the compiler's
        `fmdl_fcl_hair_fallback`, `xml_face_neck_added`, `xml_uniform_pes15`, and
        `player_layout_proto` in both the compiler's and the upgrader's rows)
        - [x] 8.6d4a the Team compiler's 234 rows. Done 2026-10-21 (SWE-2): each When says what
          the export holds, each Effect what the compiler does and, where it is plain, the fix;
          games by name, settings by their labels; `studio`'s
          `every_catalog_row_is_written_for_a_member` keeps disposition names, `context`,
          `docs/` pointers, legacy tool names and em dashes out (280 hits before, red);
          `help_cli.rs`'s two pinned cells follow. Files: `team_compiler/src/messages{.rs,
          /catalog.rs}`, `studio/src/main.rs`, `studio/tests/help_cli.rs`
        - [x] 8.6d4b the Export upgrader's 69 rows and the Save editor's 23. Done 2026-10-21
          (SWE-2): the upgrader's own rows say what the upgrade does without its step numbers, a
          structure-pass row takes the compiler's new When (its own table's two excepted) with
          the upgrade's effect; the Save editor's keep the CLI commands, drop the context lists;
          the guard covers every registered tool (19 hits before, red). Files:
          `export_upgrader/src/messages.rs`, `save_editor/src/messages.rs`, `studio/src/main.rs`,
          `studio/tests/help_cli.rs`
    - [x] 8.6e the window opens at 1440 × 900 points, centered, fitted to a smaller monitor
      (`spec/core/gui.md` "Shell layout", "The window's size"; the maintainer's requests of
      2026-10-21 and 2026-10-22), and the help window first opens at 90% of it, centered
      (`plans/core/gui.md` "Help window"), replacing its fixed 720 x 480, which a 1080p
      monitor at 125% scaling (a 768 x 432 window) cut off. Done 2026-10-23 (SWE-2):
      ST-WIN-01 and ST-HELP-05 written first, both observable headless (the frame's viewport
      commands; the help window's node bounds), so neither is manual; `shell/window_size.rs`
      (`DEFAULT_SIZE`, `fitted_size`, `fit`), the fit sent once, on the first frame that
      reports the monitor, centering the outer rectangle; the window created at the same size
      and centered, so nothing jumps where it fits; the help window's `default_rect`, which
      egui counts title bar included. Lead fix: the fit's arithmetic out of `logic` into
      `fit`. Real window checked (`.tmp/8_6/win_check.py`, a portable copy): 1456 × 939
      outer at (232, 71) on a 1920 × 1080 monitor at 100%, the scenario's values; the
      small-monitor branch is proven headless only. Files:
      `studio_core/src/shell/{window_size,mod,help_window,test_harness}.rs`,
      `docs/plans/core/gui.md`. Review: the two scenarios and the code ride with the shell's
      next Astra review (AGENTS.md: a one-crate step with no new `pub` interface)
  - [ ] 8.7 `color_tools`' colour-picker popup (`plans/libs/color_tools.md` Phase 8), the
    Save editor's team colours; the kit menu icons wait for the Team compiler's grid
    - [x] 8.7a the picker popup drawn in `docs/prototypes/save_editor.html`'s Team tab (the page
      drew the browser's own colour input), with its draft checks. Done 2026-10-21 (SWE-2):
      `colorPicker`, the host's commit and candidate callbacks, session recents, a draft
      control for a kit editor's suggestions; `T("PICKER")` in `.tmp/8_5/harness_tail.js`
      (nine checks, red before the page edit). Plan: `plans/libs/color_tools.md`'s picker
      paragraph; P23 (a swatch click closes the popup) accepted 2026-10-26, 8.7c. Files:
      `docs/prototypes/save_editor.html`
    - [x] 8.7b `color_tools`' picker widget against the draft, tested headless; the Team tab
      hosts it with its view slice (8.9). Done 2026-10-21 (SWE-2): `picker::color_field`
      (`ColorFieldOutput`: `changed` once per finished gesture, the hover or drag
      `candidate`), egui's `Popup` from the button, the square and hue strip drawn as meshes
      (egui's `color_picker_hsva_2d` adds its own rows and reports no release), recents in
      the context's temp memory; 9 headless tests and 3 unit tests, each red first. Review S1
      (SWE-2, 4 accepted) reworked: a typed hex commits on any close, a host change (an undo)
      reseeds the open popup and is no pick, 9 more tests (the painted shapes among them).
      Review A1 (Astra on Devin brother, 6 of 9 accepted) reworked: HSV over sRGB values so
      the gradients show what a click picks, the hue kept on white, gray and black, a typed
      R/G/B value committed on any close, the host's 6-bit rounding of a pick still a pick, a
      one-frame click committing; 44 tests. Rework 3 (mutation survivor): a one-frame tap
      on the hue strip tested, the commit condition's `drag_stopped()` terms dropped (the
      held press covers them). Review S2 (SWE-2, 3 of 9 accepted; the loop ends) reworked
      in rework 4: a click or a drag after the hex field had focus lands the pick, the
      tests in `picker/tests.rs`. Review A2 (Astra on Devin brother, 5 of 7 accepted)
      reworked in rework 5: one rule where typed text meets a gesture (a press outside the
      focused field settles its text before anything draws; a field's focus loss never
      commits during a gesture; typing is no gesture), Escape cancelling a gesture, the
      square's test sampling inside the painted triangles. Review S3 (SWE-2, 1 of 6; the loop
      ends) reworked in rework 6: the settle's other cases tested. Review A3 (Astra on Devin
      brother, 3 of 3; Astra's loop ends, the review done) reworked in rework 7: Tab between
      R, G and B is one edit, committed when the focus leaves all three; the tap test types a
      50% value and checks the exact pick; Escape's discard of typed text tested.
      `mutants-diff 12ffa8c3` (reworks 4 to 6) caught every `color_tools` mutant. Files:
      `color_tools/{Cargo.toml,src/lib.rs,src/picker.rs}`, `scripts/doc_pointers.py`
      (`plans/libs/color_tools.md` listed as the open phase's)
    - [x] 8.7c before the 0.0.1 cut: a click on a suggestion or recent swatch commits and
      closes the popup (P23, accepted 2026-10-26; `plans/libs/color_tools.md` "Color-picker
      widget") → verify: a headless test (a recent swatch's click: the color committed once,
      the popup closed, the color in the recents); the square and the fields still keep it
      open; the draft's `colorPicker` the same. Code done 2026-10-26 (SWE-2): `swatch_row`
      closes through egui's `Popup::close_id`, so the ordinary close (the pending text's
      settle, the recents) runs; three headless tests red first (recent, suggestion, a
      pending hex text), the square's test extended to stay open; the draft the same.
      `mutants-diff 6ff37870`: every `color_tools` mutant caught. Files:
      `color_tools/src/{picker.rs,picker/tests.rs}`, `docs/prototypes/tools/save_editor.js`
  - [ ] 8.8 `libs/team_widgets` (`plans/save_editor.md` "`libs/team_widgets`"): the card
    pickers and the violations list first, the tactics editor and its pitch with the view's
    slice 3
    - [x] 8.8a the Save editor's draft follows DBC7 as 20.0e left it (it still faked 17 cards,
      stat errors at +4 and `SkillCardCap` rows): the samples rerun with the current build,
      the panel's warning rows and the `StatTotal` sentence, the card badges' check under a
      capped ruleset. Done 2026-10-21 (SWE-2; samples by the lead, `check-aatf` on the PES 2017
      save with the assembled `dbc7.rhai`): 320 DBC7 rows for the six detail teams, 299 of
      them warnings. A gap of 20.0e's: a ruleset change reaches every draft that fakes the
      ruleset, not only the Ruleset editor's. Files: `docs/prototypes/save_editor.html`
    - [x] 8.8b the crate and its `cards` module, and `paid` in `aatf`'s `card_limits`. Done
      2026-10-21 (SWE-2): `card_pickers` (the skills grid five to a row and the COM styles,
      gated by the version's schema field set, a "Cards left" badge row per tier, `limit -
      paid`, the error color when overdrawn) and `playing_style_picker` (the version's styles,
      in stored order); the badge row stands before the grid and the section headings are the
      host's. 4 headless tests on fixture players, each red first; the badge color read from
      the painted text. Files: `aatf/{logic/logic.rhai,src/values.rs,src/file.rs,
      src/fixtures/official.rs}`, `team_widgets/{Cargo.toml,src/lib.rs,src/cards.rs}`,
      `docs/spec/aatf_rules.md`
      Review S1 (SWE-2, 2 of 7 accepted: the style order logged, PES 15's holed skill set
      tested) folded into 8.8c's rework.
    - [x] 8.8c `team_widgets::violations`: the AATF rows grouped per player, a row's click
      selecting its player (the draft's `#aatf-rows`). Done 2026-10-21 (SWE-2):
      `violations_list` (`TeamViolations`, `ListedPlayer`, `SelectedPlayer`; the check's slot
      order, the team's rows last, errors before warnings per group, suggestions on request),
      the sentences the widget's own, one per kind; `aatf`'s `ViolationKind::rule` (moved from
      the Save editor's `messages.rs`, the CLI's output unchanged); the crate's tests share
      `test_support.rs`. 6 tests, on team 734 of the PES 2017 fixture save among them, each
      red first. The player header's name is plain text until 8.9's player list draws the
      color codes. Review S2 (SWE-2, 3 of 9 accepted) reworked: the acceptance citations
      `just gates` refused, the names' color codes decoded, the row as the draft draws it (the
      rule name in a 190-point column colored by severity, the sentence strong and wrapping),
      `rule()` tested in `aatf`; rework 3 pins the row's geometry (two mutation survivors).
      Files: `team_widgets/src/{lib.rs,violations.rs,test_support.rs,cards.rs}`,
      `aatf/src/{violation.rs,fixtures/official.rs}`, `save_editor/src/messages.rs`
    - [x] 8.8 review: Astra over 8.8b-c (one review, `.tmp/8_8/duck_rulings.md`); the SWE-2
      loops ended (S1 2 of 7, S2 3 of 9). Review A1 (Astra on Codex own, 6 of 6 accepted)
      reworked in rework 4: the skills grid wraps at five, a card the version lacks stays
      disabled with a `PES NN+` tooltip, the sentences follow a value's direction and name
      what `MinCards` and a reserved name colour count (the draft's `violationSentence`
      alike), a COM style's tick tested. Review S3 (SWE-2, 3 of 7; the loop ends) reworked in
      rework 5: one bound sentence for a maximum and an exact target. Review A2 (Astra on
      Devin brother, 2 of 3; Astra's loop ends) reworked in rework 6: the playing-style test
      asserts no change on idle, open and re-pick, the grid test writes its five. Done
      2026-10-21. `mutants-diff 12ffa8c3`: one survivor, `bound`'s `>` to `>=`, equivalent
      (`.cargo/mutants.toml`)
  - [ ] 8.9 the Save editor's view in the five slices of `plans/save_editor.md` "Build order
    and verification", with the `session/` wiring each needs (undo, selection, recent files,
    the FPC toggle, AATF quick actions) and the placeholders of 8.3 gone; each slice's SE-GUI
    scenarios cited
    - [x] 8.9a slice 1's skeleton, in two runs. Done 2026-10-21 (SWE-2): (a1) Open (`rfd`
      0.17.2, `xdg-portal` only: no async runtime), the team selector with ALL, the
      cross-team filter on the display name, shirt name or id, the list with its names'
      colors (`pes_savefile`'s `name_spans`, which `display_name` now joins); (a2) the info
      strip's edits (ranges from the schema's bit widths), `session/undo.rs` (whole entries,
      boxed, one per gesture: a widget's changes merge until the frame ends with no button
      down and no keyboard focus), the dots against the file as opened or saved, Save and
      Ctrl+S refused while the save's own game runs (the CLI's rule, now the session's),
      Undo/Redo and their shortcuts, the help topic `The window`. 23 view tests, SE-GUI-01,
      02 and 04 proven; SE-GUI-03 waits for 8.9b's Abilities tab. Files:
      `save_editor/{Cargo.toml,help/02_window.md,src/lib.rs,src/cli.rs,src/session/{mod.rs,
      selection.rs,undo.rs},src/view/{mod.rs,player_list.rs,info_strip.rs,tests.rs}}`,
      `pes_savefile/src/model/names.rs`, the root `Cargo.toml`
      - [x] review: a SWE-2 loop over 8.9a, then Astra. Review S1 (SWE-2, 4 of 13; the loop
        ends, `.tmp/8_9/duck_rulings.md`) reworked in rework 1: the list lays out only its
        visible rows (`show_rows`; a frame of the PES 2021 save's ALL list 27.7 ms before,
        0.22 ms after, release), the running-game probe runs at Save only, the unsaved marker
        is a flag the session keeps, a name or shirt name the codec would refuse is not taken
        (`pes_savefile::model::names::player_text_fits`), a successful Save clears an earlier
        refusal. `mutants-diff 12ffa8c3`'s six survivors killed; its ten timeouts came from a
        test helper's unbounded Ctrl+Z loop, now bounded (CONTRIBUTING.md "A test fails, it
        never hangs"; the timeout multiplier lowered to 3 in `.cargo/mutants.toml`). Astra A1
        over 8.9a 6 of 6 accepted, its fixes landed in 8.9b1, whose Astra A2 (4 of 5) ended the
        Astra loop and whose SWE-2 loops followed (`.tmp/8_9/duck_rulings.md`)
    - [x] 8.9b1 the Abilities & Skills tab, with the 8.9a review's fixes. Done 2026-10-21
      (SWE-2, a run and two reworks): the tab (`view/abilities.rs`: the game's numbers, a
      field the version lacks drawn as a disabled "-" naming the first version with it, the
      cards row with the ruleset's cards-left badges, reflowed to fit Studio's 960 x 540
      window), the ruleset loaded from the `aatf_rules` setting (`session/rules.rs`, shared
      with `check-aatf`), the help topic `03_abilities.md`; the strip and the tab under the
      player's id (no text field or value crosses players), single-line list rows, Ctrl+Shift+Z
      before Ctrl+Z, plain runs in the label's colour, the rename and shirt flags, the upper-cased
      shirt name, Captain a radio. Files: `save_editor/{help/03_abilities.md,src/{lib.rs,cli.rs,
      messages.rs,session/{mod.rs,rules.rs},view/{mod.rs,abilities.rs,info_strip.rs,
      player_list.rs,tests.rs}},tests/cli/import_toml.rs}`, `pes_savefile/src/{schema/{mod.rs,
      fields.rs},model/{names.rs,player.rs},interchange/team_toml/{mod.rs,player/{mod.rs,
      tests.rs}}}`, `team_widgets/src/cards.rs`
      - [x] review: a SWE-2 loop over 8.9b1, then Astra round A2 (A1 accepted 6 of 6 on 8.9a,
        folded in here). S2 (two halves, 7 of 18 accepted) and the mutation run's survivors
        fixed in rework 3: the tab wraps to the pane the shell leaves (800 x 516), a COM
        styles heading, the badges cached on `Session::revision`, the merge keys per player,
        the Shirt hover's 18, `name_fits` in bytes; rulings `.tmp/8_9/duck_rulings.md`. S3 (3
        of 14) ended the SWE-2 loop; rework 4: the cross-player Speed test types a value no
        fixture player stores, `view/tests.rs` split by area into `view/tests/`, the PES 16
        Shirt hover pinned, an import marks the save dirty, `title_case` once in `labels.rs`.
        Its mutants-diff: 2 survivors (`title_case` untested in its own crate). A2 (Astra, 4 of 5)
        ended Astra's loop; rework 5: the widget ids carry the open's generation (a text
        field's undo history crossed into the next save), Name and Shirt drawn at 240 and 140
        (the grid's sizing pass gave them 40; one grid per row), the shifted fields' writes and
        the out-of-range display observed, `title_case` tested; its mutants-diff 7 of 7 caught.
        S4 (post-Astra SWE-2, 4 of 11) ended the review loops; rework 6: the Shirt hover names
        the single-byte boundary (é passes, Ł does not), the refusal tests see the typed text
        drawn first, the rules file's load error wraps, the badges follow the selection; its
        mutants-diff caught every mutant. Review closed 2026-10-22
    - [x] 8.9b2 the Appearance tab, slice 1's last part (`plans/save_editor.md` "Editing UI"
      Tabs, "Read-only aesthetics", "Stripped save"; the draft's `appearanceTab`): the tab's
      groups as the draft draws them, every value disabled under the ownership line (and on
      pre-Fox the patch step) until "Edit aesthetics anyway", a session switch off at every
      start; on a stripped player the line that the game does not read the record; the
      motion ranges and Randomize per the plan; opening a Fox save lists the players whose
      appearance id is set while others are stripped, with Strip again (one undoable edit).
      SE-GUI-07, SE-GUI-20, SE-GUI-22's Appearance-tab part (its FPC toggle and the menus are
      slice 2's, the transplant and imports slices 4 and 5); the FPC group and its findings
      wait for slice 2. Done 2026-10-22 (SWE-2, a run and one rework): `view/appearance/`
      (`mod.rs` the tab, `fields.rs` its tables; scales and names from `settings_toml`'s
      keys, motion tops from `motion_top`, swatches approximating the keys' colour names),
      the switch on `SaveView` kept across opens, `Session::strip` one sealed undo entry, the
      Strip again window's rows built at open, `help/04_appearance.md`; Fox is PES 18+ (the
      draft's `isFox` corrected). Files: `save_editor/{help/04_appearance.md,src/{lib.rs,
      session/mod.rs,view/{mod.rs,abilities.rs,player_list.rs,appearance/{mod.rs,fields.rs},
      tests.rs,tests/{abilities.rs,appearance.rs}}}}`
      - [x] review: rides with 8.9d1's (one crate, no `pub` item); Astra A4 covered both.
        `mutants-diff e34d1afc` at `0c748396`: 34 survivors, triaged in
        `.tmp/8_9/survivors_b2.md` (30 missing tests: physique, motion, combo, gloves checkbox,
        palettes, Strip again's team, the grids' row breaks, Randomize's generator; 2 equivalent;
        2 removed by giving every sealed operation one undo key); fixed with 8.9d1's rework
    - [x] the portable name limits (`plans/pes_savefile/operations.md` "Team TOML", Portable
      text limits; DECISIONS 2026-10-21), with 8.9b1 and its rework: `portable_text_max` and
      `crop_player_text` in `pes_savefile`; the info strip's Shirt field held to 18 (an
      existing longer one shortened only), its Name field to the save's own, either past the
      portable limit drawn amber with the cut text in its hover; Team TOML export's notes past
      them; `apply` cropping a name or shirt name to a shorter field with a note naming the
      text as cut (SE-TOML-06 rewritten)
    - [x] the roster-order rule (`plans/aatf_rules.md` "The host"; DECISIONS 2026-10-21), with
      the slice that builds drag reorder (SE-GUI-08): `pes_savefile::roster_order_breaks`, the
      AATF host's `RosterOrder` error for every ruleset, Team TOML export's note per slot;
      drag reorder trading the players' records between the ids (4ccEditor's spinner),
      never the roster's order. The draft's `movePlayer` still reorders the roster (its
      comment says "the ids follow the player") and is corrected first. Real saves measured
      2026-10-22 (`.tmp/8_9/roster_census/`, cup teams 701-920 of the 29 real saves on the
      machine): every current cup save (202, 210 and 220 teams; PES 17 to 21, the PES 16
      SBCF26 and refs saves) breaks the rule nowhere; the five older 192-team PES 16 saves
      break it in 13 to 25 teams each (`/vg/`, `/fit/`, `VGL 4` ... with half a roster out of
      place), and one lab PES 16 save in one team. A first AATF run flags no current cup team.
      The draft's `movePlayer` corrected 2026-10-22 (records shift, each slot keeps its id,
      base copy, number and captain mark; checked under Node). Two slices:
      - [x] 8.9c1 the rule (`TeamEntry::roster_order_breaks`, DECISIONS 2026-10-22), the AATF
        host's `RosterOrder`, the consumers' text, Team TOML export's note. Done 2026-10-22
        (SWE-2): `roster_order_player_id` the rule's one formula; `TeamView.roster_breaks`
        filled by `new`, empty from a Team TOML; the export's note on the slot's header line
        (`TeamToml.roster_breaks`, set by `from_team` alone, so a breaking team's text does
        not parse back to it). Files: `pes_savefile/src/{model/{mod.rs,team.rs},
        interchange/team_toml/{mod.rs,team.rs,player/{mod.rs,tests.rs}}}`,
        `aatf/src/{rules.rs,team.rs,violation.rs,fixtures/mod.rs}`,
        `team_widgets/src/violations.rs`, `save_editor/{src/messages.rs,tests/cli/check_aatf.rs}`
        - [x] review (checkpoint b: a new `pub` interface over four crates): a SWE-2 loop, then
          Astra; run with 8.9c2's (below)
      - [x] 8.9c2 drag reorder in the player list (SE-GUI-08): `Session::move_player`, the
        records rotating between the slots' ids as 4ccEditor's spinner trades them (`id`,
        `appearance_id` and `base_copy_id` stay with the id: its `player_export` carries no id
        and its move trades `copy_id` back), one undo entry; off in ALL and while filtering.
        Done 2026-10-22 (SWE-2): egui's `dnd_set_drag_payload`/`dnd_release_payload` on each
        row, an empty slot between skipped over as the draft does. Files: `save_editor/{help/
        02_window.md,src/{session/mod.rs,view/{mod.rs,player_list.rs,tests.rs,tests/list.rs}}}`
        - [x] rework 1 (SWE-2 round S1 on 8.9c2): every `PlayerEntry` field audited against
          `player_export`; the face run's face and hair stay with the slot, its skin, iris and
          gloves fields move (`moved_record`, tested on PES 21 and 16); two drags are two undo
          steps. Files: `save_editor/{help/02_window.md,src/{session/mod.rs,view/tests.rs}}`
        - [x] rework 2 (Astra A3 over 8.9c1, 8.9c2 and rework 1, 3 of 4 accepted, ending
          Astra's loop): `aatf::Rules::check_team` lists by slot, the team's rows last (a break
          on a later slot grouped its player first); the face and hair checked after Save and
          reopen on PES 21 and 16; the help says the filter must be empty to drag. Files:
          `aatf/src/{rules.rs,team.rs}`, `save_editor/{help/02_window.md,src/session/mod.rs,
          tests/cli/check_aatf.rs}`
        - [x] review: with 8.9c1's (one surface, the rule and its first user): SWE-2 S1 on
          each (0 of 8, 2 of 5), Astra A3 (3 of 4, rework 2), SWE-2 S2 over the reworks (0; two
          nits fixed); `mutants-diff 78b049a8` at `eca895b1`: every viable mutant caught
    - [x] 8.9d slice 2 (Team tab, batch operations, FPC), in four parts:
      - [x] 8.9d1 the Team tab (SE-GUI-10) and the Team menu but its FPC items (SE-GUI-23,
        SE-GUI-22's menu part): Set edited flag, Set boots and gloves IDs for everyone, Set
        player names to positions, Clear visual flags (Ctrl+F), on the selected player's team
        (DECISIONS 2026-10-22); `Session::edit_players`, one sealed entry, which `strip` then
        uses; `pes_savefile`'s team text check. Done 2026-10-22 (SWE-2, a run, a finish and one
        rework; the first run stalled on its own `find /`): `view/team.rs` (names with
        `team_text_fits`, the colours through `color_tools`' picker at 6 bits a channel, the
        manager and stadium ids, PES 17's kit slots as `binding / 64`), `view/menus.rs` (the
        Team menu, the boots and gloves dialog), `session/team_ops.rs` (one roster walk),
        `help/05_team.md`; every sealed operation (move, strip, team batch) through
        `edit_players` under one `SEALED_KEY`; the boots and gloves IDs bounded by the
        record's own field (`RecordSchema::max_value`, `player_max_value`), not
        `settings.toml`'s 0 to 100, which the Appearance tab had also used since 8.9b2 (a
        compiler-allocated 627 could not be typed). With it, 8.9b2's survivors' tests and
        `named_values` ending a name at "; ". Files: `pes_savefile/src/{model/names.rs,
        schema/mod.rs}`, `save_editor/{Cargo.toml,help/05_team.md,src/{lib.rs,session/{mod.rs,
        team_ops.rs},view/{mod.rs,menus.rs,team.rs,info_strip.rs,player_list.rs,appearance/{mod.rs,
        fields.rs},tests.rs,tests/{abilities.rs,appearance.rs,team.rs}}}}`, `.cargo/mutants.toml`
        - [x] rework 2 (SWE-2 S1: 3 of 10 accepted; Astra A4 over 8.9b2 and 8.9d1: 6 of 6;
          `mutants-diff 0c748396`: 12 missed, `.tmp/8_9/survivors_d1.md`): a kit-slot edit sets
          PES 17's team strip flag; ticking Base copy sets the seven gameplay flags ("Is a
          copy?"); the Team tab's name fields sized as the info strip's; a stripped player's
          motions said to be read (they sit before the appearance block); tests for the boots
          and gloves modes, the rename's flag, the dialog's end ID, the kit slots' top, the
          locked Ctrl+F, the menu's Clear visual flags, `player_max_value` and
          `first_version_with_team`; `clock_seed`'s arithmetic as a tested `odd_seed`. Files:
          `pes_savefile/src/schema/mod.rs`, `save_editor/{help/04_appearance.md,src/{session/
          team_ops.rs,view/{team.rs,menus.rs,appearance/mod.rs,tests/{team.rs,appearance.rs}}}}`,
          `.cargo/mutants.toml`
        - [x] review (checkpoint b: `team_text_fits`, `first_version_with_team`, `max_value`,
          `player_max_value`): SWE-2 S1 (3 accepted), Astra A4 (6 of 6, rework 2), SWE-2 S2 over
          rework 2 (3), Astra A5 (2, ending Astra's loop); `mutants-diff 18132ce8` at
          `4fb6c077`: 1 missed (`odd_seed -> 1`). S2's, A5's and that survivor's fixes landed with
          8.9d2's commit (lead): `odd_seed` tested on exact values, the names row in its own
          grid, the stripped-save plan text and help, the menu route's test seeding every slot
      - [x] 8.9d2 the Player menu (SE-GUI-24: Set all, Bump all, Copy and Swap stats) and FPC
        (SE-GUI-11, SE-GUI-22's FPC parts): Hide body and Show body on the Appearance tab with
        the interference findings, Ctrl+I, the Team menu's Set and Remove FPC invisibility; a
        model's own IDs kept (DECISIONS 2026-10-22). Done 2026-10-22 (SWE-2, one run):
        `session/player_ops.rs` (the four stat operations, `copy_stat_fields` audited line by
        line against `PlayerImport`'s stats half, `preset_for` keeping a model's own IDs), the
        team's `team_fpc`, the Player menu and its two dialogs (`StatNum`, `CopySwap`: team,
        then slot), Ctrl+I and Ctrl+B, the FPC group last on the Appearance tab, the five
        `fpc::check` codes in the message catalog, `help/06_player.md`. Files:
        `save_editor/{Cargo.toml,help/{04_appearance.md,05_team.md,06_player.md},src/{lib.rs,
        messages.rs,session/{mod.rs,player_ops.rs,team_ops.rs},view/{mod.rs,menus.rs,
        player_list.rs,appearance/mod.rs,tests.rs,tests/player.rs}}}`, `studio/tests/help_cli.rs`
        (the help export's last catalog row)
        - [x] review (checkpoint b: one crate, no `pub` item; rides with 8.9d3's): a SWE-2
          loop, then Astra. `mutants-diff 4fb6c077` at `e3d652da`: 10 missed, all in the
          dialogs (`.tmp/8_9/survivors_d2.md`), tested with 8.9d3's rework; the review runs
          over both commits, below
      - [x] 8.9d3 the Savefile menu (SE-GUI-27): Output rosters to TSV and Fix database behind
        its confirmation. Done 2026-10-22 (SWE-2, a run and one rework): `session/database.rs`
        (`rosters_tsv` with the original's skill and COM spellings, the playing style through
        `schema::playstyle::decode`, a BOM first; the pure `fix_database`), `edit_players`
        widened to `edit_entries` (players and teams, one sealed entry), the Savefile menu and
        the confirmation, `help/07_savefile.md`. Fix database repairs base copies on PES 15 to
        18 only, writes no PES 17 strip table and sits under the lock (DECISIONS 2026-10-22).
        The rework: the first run's own PES 19 style table had two styles swapped (a lead
        script comparing it with the original's found it), replaced by the schema's measured
        lists; 8.9d2's survivors' tests. Files: `save_editor/{help/07_savefile.md,src/{lib.rs,
        session/{mod.rs,database.rs},view/{mod.rs,menus.rs,player_list.rs,tests.rs,tests/{
        savefile.rs,player.rs}}}}`
        - [x] review (checkpoint b, with 8.9d2's): a SWE-2 loop, then Astra, over
          `e3d652da^..` the commit; `mutants-diff e3d652da` at the commit: 2 missed (the
          base-copy rule's flag-off guard). SWE-2 S1 (two halves) 4 accepted, ending its loop;
          Astra A1 5 accepted, all tests a mutant would pass (`.tmp/8_9/duck_rulings_d23.md`).
          Rework 2 (SWE-2): the FPC findings' limit and Copy/Swap's flags in the help, the Fix
          database question built from what the save has, `team_fpc` taking `fpc::Preset`, and
          the tests (base-copy halves, every team in the sheet, Set's clamp, Bump's remembered
          N, Ctrl+I both ways). `mutants-diff 7ad77f2e` at the rework (`1c6f41fc`): 3 caught,
          none missed. SWE-2 S2 3 accepted (two help texts untrue on some players or versions;
          two scenarios widened to state what their tests check), ending its loop; Astra A2 2
          accepted (help/06's Ctrl+I on a model's own boots ID, the presets' skin reset unnamed),
          ending the Astra loop. The four help fixes landed with the FPC hide IDs commit, their
          SWE-2 pass riding with that step's loop (they edit the same pages; its S1 verified
          all four)
    - [x] 8.9d4 slice 2's parity (`plans/save_editor.md` "Build order and verification", the
      second bullet): each batch operation and Fix database run on the same fixture save in
      4ccEditor (its local `Release/4ccEditor.exe`, driven through its menu command IDs in
      `resource.h`) and in Studio, the decrypted payloads compared field by field, every
      difference either a logged decision (no PES 17 strip-table write, base copies only where
      the flag exists, Set and Bump setting no flag) or a bug → verify: the difference list,
      each line ruled
      Done 2026-10-22 (SWE-2 drove the local build through Win32 messages; the lead ruled):
      on PES 16, 17 and 21 every operation gives 4ccEditor's fields, an open and save is
      byte-identical in both, and four differences are kept (DECISIONS 2026-10-22 "Slice 2's
      parity"): names to positions written (4ccEditor's never reaches the file), no Custom
      skin on a pre-Fox hide, name bytes past the terminator kept, the TSV's LF; Fix
      database's two are the earlier decision's. No bug. Outputs: `.tmp/8_9/parity/`
    - [x] the FPC hide IDs (55, 11) as a suite-common setting the editor and the Team compiler
      both read (`plans/save_editor.md` "Full Player Customization (FPC)"), in one step; both
      use `libs/fpc`'s constants until then
      - [x] briefed 2026-10-22 (`.tmp/8_9/brief_8_9_fpc_ids.md`, SWE-2): `fpc::HideIds` in place
        of the constants, `fpc_hide_boots_id`/`fpc_hide_gloves_id` in `CommonSettings` and the
        settings menu, bounded to the stock band (DECISIONS 2026-10-22), read by the editor's FPC
        paths and the compiler's `fpc_on`; SE-GUI-31, TC-BIN-43 → verify: both scenarios
        proven, a compile with an out-of-band ID refused. Done 2026-10-22 (one run; the lead
        folded `fpc_group`'s two `Ok`/`Err` branches into one match). Files: `fpc/src/{lib.rs,
        player.rs}`, `pes_savefile/src/ops/fpc.rs`, `studio_core/src/{settings/{common.rs,
        mod.rs},shell/settings_menu.rs}`, `save_editor/{help/0{4,5,6,7}_*.md,src/{lib.rs,
        messages.rs,session/{player_ops.rs,team_ops.rs},view/{mod.rs,menus.rs,player_list.rs,
        appearance/mod.rs,tests/player.rs}}}`, `team_compiler/{src/{cli.rs,compile.rs,
        validation.rs,plan/{mod.rs,ids.rs,item_rows.rs}},tests/cli/bins.rs}`
      - [x] review (checkpoints a and b: two new scenarios, a new `pub` type, five crates): a
        SWE-2 loop, then Astra. SWE-2 S1 4 accepted (a tooltip helper that a label with the same text satisfied, so
        removing a disabled hover kept its test green; stale docs), ending its loop; Astra A1 3
        accepted (the gloves row, every FPC path under changed IDs, TC-BIN-43's "no output"),
        ending the Astra loop (`.tmp/8_9/duck_rulings_fpc.md`). Rework 1 (SWE-2 and the lead:
        `tooltip_containing` now needs the hover to add its text). `mutants-diff 1c6f41fc` at
        `afb01451`: 19 caught across the five crates, none missed. SWE-2 S2 after Astra 2
        accepted (the team test's own-ID branch never fired; SE-GUI-31 widened to the paths
        its tests check), fixed by the lead in the same commit; the loops end
    - [x] 8.9e slice 3, the Tactics tab (`plans/save_editor.md` "Tactics editor" with its "What
      the controls write", "`libs/team_widgets`"; SE-GUI-12, 25; DECISIONS 2026-10-22 "The
      Tactics tab's edits"), in three parts, the pitch last:
      - [x] 8.9e1 the settings: `Instruction::takes_player` in `pes_savefile`, `team_widgets`'
        `tactics` module (preset selector, sliders, style pairs, advanced instructions with
        their target, automatic settings), the Tactics tab enabled in the editor with its
        copy/swap preset and formation menus as sealed edits (SE-GUI-25) → verify: SE-GUI-25
        proven, the instruction list per version tested on 17, 18 and 21. Done 2026-10-22
        (SWE-2, one run; the lead drew both choices of a style pair every frame, which an `||`
        skipped on a click, dropped the copy/swap ops' unreachable index guards and worded the
        substitution setting for members, the draft too): `schema::instruction::instructions`
        public (the version's list in the game's order), `TacticsView`, `tactics_settings`
        (a stored value outside a slider's range is kept until moved; the answer is a deep
        compare), help topic 08. Files: `pes_savefile/src/{model,schema}/instruction.rs`,
        `team_widgets/src/{lib.rs,tactics.rs,test_support.rs}`, `save_editor/{help/08_tactics.md,
        src/{lib.rs,session/{mod.rs,tactics_ops.rs},view/{mod.rs,player_list.rs,tactics.rs,
        tests.rs,tests/{abilities.rs,tactics.rs}}}}`
      - [x] 8.9e2 the starting eleven and bench lists, swap by drag with the roles handed over
        (the plan's last bullet) → verify: a swap's repointing tested on every role. Done
        2026-10-22 (SWE-2, one run and one rework): `team_widgets::pitch` (`lineup`,
        `LineupDrag` for the pitch's markers to drop, `swap_in`), `RosterRow` in place of the
        names, `TacticsView::formation` and its selector above the lists; an empty bench row
        (a roster slot with no player; `bench_order` has no sentinel) is no drag source and
        takes no starter; the lists in `ui.columns` (inside a `horizontal` their scroll areas
        overlapped). SE-GUI-12's bench half tested, its citation waiting for e3. Rework 1:
        `mutants-diff a7b144f0` at `610bbb5c` (8.9e1): 2 missed, the rest caught or unviable (the Target combo's
        lineup number, the formation menu's copies), both now tested. Files:
        `team_widgets/src/{lib.rs,pitch.rs,tactics.rs,test_support.rs}`,
        `save_editor/{help/08_tactics.md,src/view/{mod.rs,tactics.rs,tests/tactics.rs}}`
      - [x] 8.9e3 the pitch: markers dragged with snapping, the slot popover, a bench row
        dropped on a marker, the role palette and join-attack modes (SE-GUI-12); the draft
        first gets the keeper lock and the packed join list, and its harness
        (`.tmp/8_5/run_harness.py`) the SE-GUI-07 and 08 checks that fail since those scenarios
        changed → verify: SE-GUI-12 proven, the harness all ok. Done 2026-10-22 (SWE-2, one
        run; the lead fixed the harness's SE-GUI-07, 08 and 11 checks, 32 of 32 ok): `pitch`
        (the canvas, markers as AccessKit buttons, `snap`, the keeper not dragged, the popover
        closing on a preset, formation or team change), the assignment chips and note; a
        marker dropped on another exchanges the two starters, one released off the pitch stays
        (DECISIONS 2026-10-22, the draft synced). With rework 1 (SWE-2 S1 over e1+e2, 5 of 8
        accepted, `.tmp/8_9/duck_rulings_e.md`; a run that stopped half-way, finished by a fresh
        one and the lead): the bench move's rotation was backwards and its test copied it;
        `LineupDrag` crate-private; `tactics/` and `pitch/` split into folders (48 tests before
        and after); the Preset selector in the toolbar row beside the menus acting on it. Files:
        `team_widgets/src/{lib.rs,tactics/{mod.rs,instructions.rs,assignments.rs},pitch/{mod.rs,
        lineup.rs,popover.rs}}`, `save_editor/{help/08_tactics.md,src/view/{tactics.rs,
        tests/tactics.rs}}`
      - [x] review: a SWE-2 loop, then Astra, over e1 and e2 together, and over e3. Done
        2026-10-23:
        SWE-2 S2 over the whole tab, 5 of 9 accepted (the decision's marker-on-marker and
        off-pitch drops untested, a swap test the identity eleven let pass, the armed note
        spanning both columns as the draft draws it); `mutants-diff 610bbb5c` at `ec53b1c4`, 45
        missed in `pitch/`: 43 missing tests, 2 equivalent (`.cargo/mutants.toml`); rework 2
        (SWE-2) fixed both, the pitch tests moved to `pitch/tests.rs` (`mutants-diff ec53b1c4`:
        3, none missed). S3 accepted 4 of 5, ending the SWE-2 loop: an eleven row released on
        the grass moved his slot (a `LineupDrag::Marker` payload now), the lists and popover
        drew names without their colour codes (`team_widgets::names`), the pitch tests past a
        thousand lines (`pitch/tests/`), one undo key for the whole tab (one per widget call);
        rework 3 (SWE-2) fixed all four. Astra A1 (Devin work; Codex own ran out of its
        window first) accepted 7 of 7 (`.tmp/8_9/duck_rulings_e.md`): settings rows clipped at a
        narrow window, two gestures in one widget merged into one undo step, a swap repointed
        instructions that take no player of ours, PES 15/16 lost the instruction rows, and
        three tests could not fail; rework 4 (SWE-2) fixed all seven. SWE-2 S4 accepted 3 of
        8, ending that loop (a drag after a typed field merged with it: a press now ends a
        field's focus; rows wrapped; Tight Marking's target text), rework 5; Astra A2 (Devin
        work) accepted 2 (every instruction row and chip untested), ending Astra's loop,
        rework 6; S5 accepted 2 of 7 (two tests that could not fail), ending the last loop,
        rework 7 (tests only). Mutation runs over reworks 4-6: 17, 10 and 0 missed, the
        missing ones tested in the next rework
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
      - [x] slice 1, the frame: body text at weight 400, the team bar above every tab (name,
        short name, colours), the tab strip grouped Player / Team, the player header on the
        player tabs only, a Team settings tab
      - [x] slice 2, Abilities: Profile, the positions grid in the collaborator's order and the
        quick actions side by side; stats in four aligned columns in the game's order; one
        cards badge, the player's own tier
      - [x] slice 3: star and playing attitude in a Cutscenes group first on Appearance,
        outside the aesthetics lock; Appearance's groups aligned; Tactics in three columns,
        every control in view at 1440 x 900. The v2 harness (`.tmp/8_9f/run_harness_v2.py`)
        53 of 53, the first draft's 32 of 32
      - [ ] slice 4: v2 is the base (the maintainer, 2026-10-24; DECISIONS "The second draft
        arrangement is the base"): the built tabs of slices 1-3 ported to `save_editor_v2.html`'s
        arrangement, after 8.9j4 (the comparator, panels the same in both drafts) and before
        8.9j5; the maintainer's fine tweaks follow on v2
        - [x] 4a the frame: the team bar above every tab (the Team tab's name, short name and
          colour rows), the tab strip in Player / Team groups (`Team settings`), the player
          header (the coloured name, v2's info strip grid) on the Player tabs only, the quick
          actions a column in Abilities' top row (SE-GUI-01, 10, 32; help chapters 02, 05).
          Done 2026-10-24 (SWE-2, two reworks): `team::bar`, `Tab::TeamSettings`, the header a
          title (`append_name` at the heading size) over two grids at the draft's widths (Name
          190, Shirt 110: DECISIONS "The player header's two rows, two grids"), the quick
          actions a column atop Abilities (4b moves the call), `selected_team` the three team
          views' head; the tab-fit tests at the smallest default's 877 x 624 pane. Seen in the
          built window on a copy of the maintainer's PES 2021 save, at 1440 x 900 and
          1037 x 648 (the team bar's split colour pair there: fixed in 4b). Files: `save_editor/
          src/view/{mod.rs,team.rs,info_strip.rs,quick_actions.rs,abilities.rs,tactics.rs,
          appearance/mod.rs,tests.rs,tests/*}`, `help/{02_window,05_team}.md`
          - [x] review (`.tmp/8_9f/duck_rulings_4a.md`): SWE-2 S1 2 accepted, Astra A1 3
            accepted (SE-GUI-01 and 32 name their tab); mutants-diff 26bc1fe8 clean (19
            caught, 4 unviable); the fixes landed with 4b (the help topic's title, the quick
            actions inside the tab's id scope, the column and the title's colours tested, one
            `team_entry`/`label_value`); the post-Astra SWE-2 round is 4b's S1
        - [x] 4b Abilities: the Profile block, the positions grid in the collaborator's order
          (the registered one outlined), stats in four columns filled column by column; star
          and playing attitude to Appearance's Cutscenes group, outside the lock (SE-GUI-06,
          07; help 03, 04). Done 2026-10-24
          (SWE-2, two reworks): the top row's three blocks (Profile 4 x 2, the positions grid
          with the registered cell outlined, the quick actions) on one line or Profile over
          the other two, and the team bar one row or two, each picked by a measured width
          constant (DECISIONS "Rows picked by width, and stats columns that shrink before
          they clip": two measured-wrap designs failed first); the stats in up to four
          columns filled column by column; `CUTSCENE_STATS` in `appearance/fields.rs`, one
          `small_stat` cell for both tabs. Files: `save_editor/src/{lib.rs,view/{abilities.rs,
          team.rs,quick_actions.rs,terms.rs,appearance/{mod.rs,fields.rs},tests.rs,tests/*}}`,
          `help/{03_abilities,04_appearance}.md`
          - [x] review (`.tmp/8_9f/duck_rulings_4b.md`): SWE-2 S1 3 accepted (widget ids fixed
            per block, pair and stats cell; a boundary test per width constant; the test
            helpers' copies), Astra A1 1 accepted (the top row's constant measured with the
            Profile combos' longest texts); mutants-diff 786b91ec clean (25 caught, 2
            unviable); the fixes landed with 4c, whose SWE-2 S1 is the post-Astra round
        - [x] 4c Appearance's groups in aligned cells, Tactics in three columns (DECISIONS
          "The draft's pair counts as caps, and the Tactics settings column below on a narrow
          pane"; help 08). Done 2026-10-25 (SWE-2, one rework carrying 4b's review fixes):
          physique, strip style and motion grids at the draft's 2 / 4 / 4 pairs a row as caps,
          the strip's checkboxes a label cell and a bare box, its wrist-taping, spectacles and
          gloves rows a grid of their own; the Tactics tab's settings column (preset bar,
          note, groups) beside the pitch and the 240-point lists above a width computed from
          constants, else whole below them; the formation settings one grid, two pairs a row
          where a computed upper bound fits; the lists' names cut at their width (seen over
          the settings column in the built window). Widget ids hold across every width
          threshold through explicit scope ids (`view::scoped`: a `push_id` salt still reads
          the parent's call counter). Files: `save_editor/src/view/{mod.rs,abilities.rs,
          team.rs,tactics.rs,appearance/mod.rs,tests.rs,tests/*}`, `help/08_tactics.md`,
          `team_widgets/src/{tactics/{mod.rs,assignments.rs,tests/mod.rs},pitch/{lineup.rs,
          tests/paint.rs}}`
          - [x] review: SWE-2 S1 (2 accepted: the preset bar's wrap, the strip rows' shell),
            Astra A1 (2 accepted: the instruction combos' cap, the default window's height
            measured), mutants-diff 8699f4e6 (12 missed); rulings `.tmp/8_9f/duck_rulings_4c.md`
          - [ ] rework r2 with 8.15 (2026-10-25, SWE-2 55 min, ended on its output limit before
            its report; the lead finished it: a perturbation it left undone, the red runs, the
            offsets): the window at 1280 x 800 (ST-WIN-01, ST-HELP-05), the lists at 200, the
            default-pane tests at 1120 x 776, the preset bar wrapped, the instruction combos
            capped at the draft's 120 with the full text on hover, `strip_row`, the survivors'
            tests for `three_columns`, the lists' gap and the stacked top; the settings column
            14 points past the pane at the default (DECISIONS "The Tactics settings column
            scrolls with the tab at 1280 × 800"; the default-pane test checks the right edge
            only). Mutants-diff e14a64da: 13 missed, to r3; `two_pairs_width`'s survivor and
            the advance rect's width are equivalent (`.cargo/mutants.toml`), its height killed
            by the stacked test. Open: the post-Astra SWE-2 round over r2-r4
          - [x] rework r3 (2026-10-25, SWE-2 41 min): the closed combo's hover tested both
            ways (an elided selection shows its text, a fitting one shows none; drawn with no
            fonts, 9-point tofu glyphs, so the widths hold whichever fonts the build unifies),
            the tooltip helper counting only nodes the hover adds, `strip_rows`' last row end
            removed (a grid's last `end_row` only moves its cursor). Files:
            `team_widgets/src/tactics/{instructions.rs,tests/mod.rs}`,
            `save_editor/src/view/appearance/mod.rs`
          - [x] rework r4 (2026-10-25, SWE-2 64 min and a short r5): the Save editor's
            `Headless` draws every view inside a `CentralPanel` as the shell does (red first:
            the default-pane test at the old 1120 pane drew the settings stacked, the
            real-window defect); `TAB_AREA_OFFSET` 232, measured at four widths; the default-pane
            tests at `DEFAULT_PANE` 1224 x 776, every edge computed from the constants; the
            instruction combos capped at the row's remaining width (clip edge), so they match
            the target column's, not the "Instruction" header's 100; the preset bar's wrap
            tested at the narrowest three-column pane (360-point column). Measured at the
            default: tab area 992, settings column 456, one formation pair a row (two need
            639), the column 47 points inside the pane's bottom (24 with a note armed;
            DECISIONS "The Tactics settings column fits the pane at 1280 × 800, the lists stay
            at 200"). manual: checked 2026-10-25 on 8.25's release build at 1280 x 800
            (`.tmp/8_25/look_tactics.png`): three columns, every control in view. Files:
            `save_editor/src/view/{tactics.rs,tests.rs,tests/{tactics,appearance,abilities}.rs}`,
            `team_widgets/src/tactics/instructions.rs`. Mutants-diff 4158a3ee: 3 mutants, 2
            caught, 1 unviable. Post-Astra SWE-2 round S2 over r2-r4: 2 accepted (the
            default-pane test's bottom bound and one-line bar; the cap test's narrow half on
            the target combo), fixed in r6 with three nits; the 4c review loop ends
            (`.tmp/8_9f/duck_rulings_4c.md`)
        - [x] 4d the one cards badge, the player's own tier, through a tier recognition
          `libs/aatf` exposes (checkpoint (b)). Done 2026-10-25 (SWE-2, 31 min; brief
          `.tmp/8_9f4d/brief_8_9f4d.md`): `card_pickers` takes `Option<CardsBadge>` (`Tier`
          drawn `Gold: 6 of 8 cards`, hover `Gold: 2 paid of 8`, red when overdrawn; `NoTier`
          the weak `No tier matches`), the Save editor asks `Rules::player_tier` then
          `card_limits` on the `BadgeKey` cache. Every team 701 slot reads `regular` under every
          bundled ruleset, never the first tier; VTL11 matches no tier for 78101 (team 781).
          Lead: the cache holds `OwnBadge::{Tier, NoTier, Failed}`, so a failing check is not
          rerun each frame; one `own_badge` function; the VTL11 test file through
          `aatf::assemble_file`. Red by hand: the first tier read ("Gold: 6 of 6 cards" against
          "Regular: 3 of 3 cards"), `NoTier` drawn as nothing, the error colour dropped
          (#8C8C8C against #FF0000). The remote below its disk floor mid-run: the clone's
          build cache removed (`scripts/remote_clean_target.sh`, 28 GB; 47 GB free). Files:
          `team_widgets/src/cards.rs`, `save_editor/src/view/{abilities,mod}.rs`,
          `view/tests/abilities.rs`, `help/03_abilities.md`. `mutants-diff feeccd42`: 6 caught,
          1 unviable. Review (`.tmp/8_9f4d/rulings.md`): SWE-2 S1 accepted 2 of 4 (a failing
          tier check untested, now tested red first: "drew No tier matches"; the docs'
          leftover "badges"), Astra A1 (Devin brother) 1 of 2 (`plans/save_editor.md`'s `cards`
          bullet and `plans/team_creator.md` "4." still per tier); both loops end
        - [x] 4d's post-Astra SWE-2 round S2: 2 of 3 accepted, the loop ends. `check_team` never
          calls `player_tier` or `card_limits`, so a failed check now shows its error in the
          badge's place (DECISIONS "A failed tier check shows its error in the Cards left
          badge's place"; red: the error left undrawn); the last "badges" comments singular.
          `mutants-diff 2c6ad5ad`: 3 caught, 1 unviable
        - [x] 4e the regular-weight body font (`core/gui.md`'s theme; a bundled font file);
          re-measure the width constants taken with the light font (`team.rs`
          `TEAM_BAR_ONE_ROW`, `abilities.rs` `TOP_ROW_ONE_LINE`). Every view test harness
          installs the app's fonts itself: a lone `cargo test -p team_widgets` (and each
          crate's mutation run) builds egui without `default_fonts`, every glyph a 9-point
          tofu box, while the workspace's run gets the real fonts through `eframe` (found at
          4c r3), so a width test measures different text per invocation. The font is
          `resources/fonts/Ubuntu-Regular.ttf` (lead, 2026-10-25; DECISIONS "The body font is
          Ubuntu Regular, bundled; every view test draws with the app's fonts"); briefed to
          SWE-2 as `.tmp/8_9f4e/brief_8_9f4e.md`. The drafts' host draws body text at 400 too
          (side-lead A's 8.16, DECISIONS "The drafts' shared stylesheet draws body text at
          weight 400"). Done
          2026-10-25 (SWE-2, 27 min): `theme::install_fonts`, now public, swaps the regular
          weight in for `Ubuntu-Light` in every family (monospace's fallback too: a family
          naming a missing font panics) and is called by every view harness (`team_widgets`
          through a dev-dependency on `studio_core`) and the help window's two wrap tests;
          `fontless` is gone, its three tests re-derived on real text ("9 PLACEHOLDER", 100
          points, elides; a 300-wide pane leaves the target combo 108). "Unassigned" measures
          67.97 in a lone `-p team_widgets` run and in the workspace run alike. Re-measured:
          `TEAM_BAR_ONE_ROW` 723 → 725, `TOP_ROW_ONE_LINE` 1117 → 1127 (Profile 518, positions
          477, quick actions 115); no layout stopped fitting (the Tactics column still ends at
          729, the preset bar one line at `DEFAULT_PANE`). Putting the light weight back first
          fails only `theme.rs`'s test: the regular weight is at most a few percent wider, under
          every other test's margin. Files: `studio_core/src/{lib.rs,theme.rs,
          shell/help_window.rs,shell/sidebar/tests.rs,widgets/log_panel/tests.rs}`,
          `team_widgets/{Cargo.toml,src/test_support.rs,src/tactics/instructions.rs}`,
          `save_editor/src/view/{tests.rs,team.rs,abilities.rs}`, the Ruleset editor's and Team
          compiler's view harnesses, `resources/fonts/`. manual: checked 2026-10-25 on the
          release build at 1280 x 800 (`.tmp/8_9f4e/look_{tactics,abilities}.png`): the
          regular weight throughout, Tactics in three columns with every control in view,
          Abilities' top row on two lines as before; the maintainer looked at the regular
          weight in the window the same day: fine. Mutants-diff e704be83: 2 mutants, 2 caught.
          SWE-2 round S1: 2 accepted (`color_tools`' harness; one uniform rule, every test
          context that runs a frame installs the fonts, through `shell::test_harness::
          test_context` for the shell's 78), fixed in r1, which the regular weight made find two
          real defects: the status bar's read-only notice, once elided, showed its own text on
          hover instead of the entry's explanation (`show_tooltip_when_elided(false)`), and the
          help window's Messages grid measured its Severity column from the rows alone, the
          wider header pushing the grid 5.9 points past a 500-point pane (the headers now count)
          (`.tmp/8_9f4e/duck_rulings_4e.md`). Mutants-diff 7781f77b: 7 caught. Astra A1 (Codex):
          1 accepted, the cap test's narrow half could not fail (its "Unassigned" fit without
          the bound; now the ninth starter's name, which fills the cap); post-Astra SWE-2 S2: 1
          accepted, the same class in its wide half (now "Deep Defensive Line" at the cap); both
          fixed by the lead, red first; the 4e review loop ends
    - [ ] 8.9j slice 4 (`plans/save_editor.md` "Configurable AATF rules", "Comparator",
      "Aesthetics transplant"), the AATF parts first (the maintainer's direction of 2026-10-24),
      each part its own commit and review:
      - [x] 8.9j1 the rules file's settings row (`studio_core`'s path row with Browse, the
        common folder rows' Browse with it) and the quick actions row (SE-GUI-09, 32) →
        verify: SE-GUI-09 and 32 by view tests; Browse by hand in the built window (`manual`).
        Done 2026-10-24 (SWE-2, 31 min, one rework for the lead's review: the row's own error
        line observed, a Set stats test): `studio_core::widgets::{row_label, path_row,
        PathKind}` (Browse through `rfd`, native only), the Save editor's one-row section,
        `Session::apply_tier` (one sealed entry, an `anyhow` error for a team that does not
        view or an empty slot: the sidekick's contradiction, accepted), `view/quick_actions.rs`
        under the info strip, the load error's wording shared with the badges
        (`rules_load_error`); five FPC tests draw a 900-point window, the row pushing the
        Appearance tab's last group below 700. Files: `studio_core/{Cargo.toml,src/{shell/
        settings_menu.rs,widgets/{mod.rs,settings_row.rs}}}`, `save_editor/{help/{02_window.md,
        03_abilities.md},src/{lib.rs,session/player_ops.rs,view/{abilities.rs,mod.rs,
        player_list.rs,quick_actions.rs,tests.rs,tests/{player.rs,quick_actions.rs}}}}`
        - [x] review: checkpoint (a) for SE-GUI-32 and (b) for the new `pub` rows: a SWE-2
          loop, then Astra; `mutants-diff`. S1: 2 of 7 accepted, one rework (SWE-2, 7 min):
          `Session::slot_player`, the roster slot's one lookup for the selection and
          `apply_tier`; `row_label` crate-private; a test that the path row draws Browse,
          and no wasm `browse` stand-in (the mutation run's two survivors). Astra A1: 3 of 3
          accepted, rework 2 (SWE-2, 10 min): Set stats' number typed into its field (and
          clamped at 99), the Cards left load-error tests reading the pane's own nodes, and
          one `tool_context` test helper putting the PES folder under the test's scratch (two
          tests had started the portrait worker against the real install). S2: 3 of 3
          accepted, lead fixes: one `player_entry` test helper for four copies, SE-GUI-09's
          no-violation check covering `NameColor`, SE-GUI-09 gaining Set stats' THEN; the loop
          ends (rulings `.tmp/8_9/duck_rulings_j.md`; P25 from its notes)
        manual: checked 2026-10-24, a portable copy of the debug build at `664c98bb` (its own
        `data/settings.toml`; UI Automation and keys into its own windows only,
        `.tmp/8_9j/ui.ps1`): the AATF rules file row's Browse opens the native Open dialog
        filtered to "AATF rules (*.rhai)", a pick writes `aatf_rules` and the row shows it; the
        Exports folder row's Browse opens Select Folder, and Escape leaves `settings.toml`
        byte for byte as it was
      - [x] 8.9j2 the AATF panel (SE-GUI-15): teams picked, violations per player, a click
        selecting him, Show suggestions, Validate rules, Save a copy, the whole-save check on a
        worker. Done 2026-10-24 (SWE-2, 40 min; the open points ruled first, DECISIONS
        2026-10-24 "the AATF panel's current team…"): `aatf::{OFFICIAL_FILE_NAME,
        official_file}` (`Rules::official` loads it), `session::rules::{team_views,
        check_teams, validate}` (`check-aatf` on the same check), `view/aatf_panel.rs` (an
        `egui::Window` from Panels and the quick actions' `AATF…`, the check on a thread per
        key, `Arc` rules, started when the edit settles as the undo seal does). The
        sidekick's contradiction, accepted: no PES 17 fixture team is clean, so the
        no-violation test is PES 15 team 714 with the CLI suite's legal Team TOML. Lead: SE-GUI-15
        widened to the recheck, Select teams' failed line and the load error, which the tests
        observe; five tests' comments made citations. Files: `aatf/src/{lib.rs,rules.rs}`,
        `save_editor/{help/02_window.md,src/{cli.rs,session/rules.rs,view/{aatf_panel.rs,
        menus.rs,mod.rs,player_list.rs,quick_actions.rs,tests.rs,tests/{aatf_panel.rs,
        quick_actions.rs}}}}`
        - [x] review: checkpoint (a) for SE-GUI-15's widening and (b) for `aatf`'s two `pub`
          items: a SWE-2 loop, then Astra; `mutants-diff`. S1: 6 of 10 accepted, rework 1 (SWE-2,
          16 min): the results in a scroll area, one player map per frame, a click clearing the
          list's filter, no "No violations." over failures only, another save resetting the
          panel, member wording on the hovers. Mutants `a6077a19..d31798d9`: 15 caught, 8
          unviable, 2 missed (a dead worker, Checking… while an edit is unsettled), rework 2
          (SWE-2, 11 min): a test each. S2: 2 accepted (an all-clear beside a failed team, the
          checking mark scrolled off), lead fixes; SWE-2's loop ends. Astra A1 (Devin `work`): 4 of 5 accepted (the picker and the
          results sharing one scroll id, no test of a rules-file switch, a vacuous ALL-click
          assertion, SE-GUI-15 omitting what its tests pin), rework 3 (fresh SWE-2 session, 18
          min: an id salt each, a rules-file switch test, `checked` waiting on the file) and
          the plan widened by the lead; Astra's loop ends (rulings
          `.tmp/8_9j/duck_rulings_j2.md`). Mutants on rework 3: 2 caught. S3: 2 accepted (a
          doc claim, the tests file past 1000 lines), riding 8.9j3's rework; the review is done
      - [x] 8.9j3 Live AATF (SE-GUI-30): the toolbar toggle (`live_aatf`, remembered) and a
        `BottomDock` at the bottom of the view listing the open team's violations under `N issues
        · <ruleset>`, re-checked after every settled edit. Done 2026-10-24 (SWE-2, 26 min): the
        floating panel's check moved to `view/aatf_check.rs` (`CheckKey`, the worker, `TeamsCheck`,
        `current_team`, the shared `results` and `select_player`), both callers on it;
        `BottomDock::new` takes the chevron tooltips' noun (`Collapse Live AATF`). Files:
        `studio_core/src/widgets/{dock.rs,log_panel.rs,log_panel/tests.rs}`,
        `save_editor/{help/02_window.md,src/{lib.rs,settings.rs,view/{aatf_check.rs,aatf_panel.rs,
        live_aatf.rs,mod.rs,player_list.rs,tests.rs,tests/{aatf_panel.rs,live_aatf.rs}}}}`
        - [x] review: checkpoint (b) (`BottomDock::new`'s signature, two crates): the lead's
          review (one issue count, `live` private, U+21BB missing from egui's fonts, the first
          header untested) and S3's items in rework 1 (SWE-2, 17 min); lead: the checking-mark
          tests held the pointer (a reply between a first frame's two passes failed one run in
          eight), CONTRIBUTING's rule. Mutants `1311c1d2..55cbede0`: 33 caught, 9 unviable, 1
          missed (the mark's `&&`, killed by the lead's test), the dock's timeout a slow fail;
          on rework 1: 10 caught. S1: 2 accepted (no test of another team or rules file over a
          held result, the click helper's copies), rework 2 (SWE-2, 10 min); SWE-2's loop ends.
          Astra A1 (Codex `own`): 2 of 3 accepted (SE-GUI-30 omitting what its tests pin; it,
          SE-GUI-15 and SE-GUI-02 not naming the saves their tests open), both plan edits by the
          lead; Astra's loop ends. S2: 4 accepted (SE-GUI-15's PES 15 clause misplaced, its
          click's filter clearing, title, team group and empty pick unstated; the dock's stale rows
          unasserted), plan and test edits by the lead; the review is done (rulings
          `.tmp/8_9j/duck_rulings_j3.md`)
      - [x] 8.9j4 the comparator (SE-GUI-13); the direction, Export's scope and the tabs' terms
        ruled first (DECISIONS 2026-10-24 "The comparator's direction…"; code map
        `.tmp/8_9j/j4_map.md`)
        - [x] 8.9j4a the tabs' terms as data the tabs and the panel both read (a diff row's label
          and shown values), the tabs on it unchanged on screen; the `diff` report's text as one
          function the CLI and Export share. Done 2026-10-24 (SWE-2, 34 min, rework 1 8 min):
          `view/terms.rs` `row_term` over the tabs' tables and lifted label consts (a field no tab
          draws keeps the report's terms); `SettingKey::{player_field, face_field}`;
          `cards::playing_style_text`; `diff::report_lines` on `report`'s own walk. Both are
          `expect(dead_code)` outside tests until 8.9j4b calls them. SE-GUI-13 rewritten for
          the 2026-10-24 rulings. Files: `pes_savefile/src/settings_toml/keys.rs`,
          `team_widgets/src/cards.rs`, `save_editor/src/{diff.rs,test_support.rs,view/{mod.rs,
          terms.rs,abilities.rs,info_strip.rs,appearance/{mod.rs,fields.rs},tests.rs}}`
          - [x] review: checkpoint (b) (two crates' new `pub` fns, SE-GUI-13's rewrite): S1
            1 accepted (two near-identical key lookups), Astra A1 (Codex `own`) 1 of 3 (SE-GUI-13's
            click on players no roster holds), S2 2 of 6 (the group bullet's scope, a misattributed
            grid order); mutants 46 caught, 7 missed (3 tested in their own crates, the palette's
            restored, 2 equivalent), the fixes' 7 caught; rulings `.tmp/8_9j/duck_rulings_j4.md`
        - [x] 8.9j4b the panel (DECISIONS 2026-10-24 "The comparator's groups, its worker and a
          kept pick")
          - [x] 8.9j4b1 the pick and its version refusal, the worker's comparison, the rows per
            team in the tabs' terms, the scope toggles, a click selecting the player. Done
            2026-10-24 (SWE-2, 37 min, rework 1 for the stripped marks' words, the heads' clicks and
            four repetitions): `view/comparator.rs` on `diff::build`'s one walk (its rows carry
            their `PlayerDiff`), the worker cycle shared with the AATF checks (`view/worker.rs`),
            the rows under `ScrollArea::show_rows` (a 10k-line result's frame 11 ms, debug), the
            refusal's sentence in `messages.rs`, `EditFile: Clone` for the worker (4 ms on the PES
            21 fixture), `violations::{player_head, row_paint}` shared with the violations list.
            Files: `pes_savefile/src/file.rs`, `team_widgets/src/violations.rs`,
            `save_editor/src/{cli.rs,diff.rs,messages.rs,view/{mod.rs,menus.rs,comparator.rs,
            worker.rs,aatf_check.rs,aatf_panel.rs,live_aatf.rs,player_list.rs,appearance/mod.rs,
            terms.rs,tests.rs,tests/comparator.rs}}`
            - [x] review: checkpoint (b) (`EditFile: Clone`, `violations::{player_head,
              row_paint}`, the plan's new comparator bullets): SWE-2 S1 4 accepted (a team
              headed once per run of its players, now one group per team in the open save's
              order; a refused pick keeping the last rows; a re-pick keyed on its path; the `No
              team` group untested), Astra A1 1 (the kept pick untested), S2 0; mutants 59
              caught, 2 missed (`reset`, killed by the kept-pick test), the fixes' 4 caught
              (rework 2); rulings `.tmp/8_9j/duck_rulings_j4.md`
          - [x] 8.9j4b2 the team filter and Export. Done 2026-10-24 (SWE-2, 36 min): the Team combo
            is the player list's `team_selector` (its own salt), narrowing the held lines on the
            frame (`shown`; a 10k-line result's frame 11.6 ms filtered or not, debug) with no
            comparison run; `Export…` writes `diff::report_lines` of the scope over the whole
            save, the command's stdout byte for byte (647 ms on that result, on the click), with
            a `Wrote` line or a `Cannot export the differences` refusal; one `SaveFileDialog`
            type for both save dialogs; `cli_stdout` shared in `test_support`. Files:
            `save_editor/src/{diff.rs,test_support.rs,view/{mod.rs,comparator.rs,aatf_panel.rs,
            player_list.rs,tests/comparator.rs}}`
            - [x] review: SWE-2 S1 on `a3422f4a..` (crate-internal: no new `pub`, no Astra
              round): 2 accepted (an empty report exported a lone newline; a picked-only
              player's roster team untested), fixed by the lead; mutants 8 caught, 0 missed
      - [ ] 8.9j5 the transplant (SE-GUI-14), redirected 2026-10-25 (the maintainer; DECISIONS
        "The aesthetics transplant writes the team's export, after 0.1.0; the save-to-save
        transplant goes"): the export-writing panel is built after 0.1.0 (its plan section);
        8.9j5c removed the save-writing transplant (8.9j5a's panel, the CLI command,
        `pes_savefile`'s operation, their codes, help, spec), keeping the comparator's r2 fixes
        (the one-line pick row, the focus given up); 8.9j5b is dropped:
        - [x] 8.9j5a the panel: the donor pick (another version refused with the CLI's
          sentence), the selection text, the preview (the comparator's aesthetics rows between
          the save and a copy with the transplant applied, on its worker), Apply as one undoable
          edit behind the lock (`Session::transplant` records the undo entry); briefed to SWE-2
          as `.tmp/8_9j5/brief_8_9j5a.md`. Done 2026-10-25 (SWE-2, 24 min, plus two lead
          fixes, red first: the heads no longer carry the comparator's stripped marks, which
          name one of two saves, and a stripped-only player has no head; the draft's `N
          changed field(s)` line): `Session::transplant` restores the targets after the lib's
          check-and-copy and replays them through `edit_players` (one sealed entry; the CLI's
          output unchanged); `view/transplant.rs` (the window, the Panels menu item, the worker
          keyed on both revisions and the text; about 1 s on the PES 17 fixture pair in a debug
          build); the help topic's paragraph. Files: `save_editor/src/{session/mod.rs,
          view/{transplant.rs,mod.rs,menus.rs,comparator.rs,player_list.rs,tests.rs,
          tests/transplant.rs}}`, `save_editor/help/02_window.md`. Mutants-diff a5a2fa6f: 2
          missed, both `Transplant::reset` (untested). SWE-2 S1: 7 accepted (the applied line
          outliving an Undo or a Save; the selection edit's withdrawal, `Previewing…`, `reset`
          and a re-pick untested; the stripped-mark test unable to fail; the refusal test
          missing SE-GUI-14's GIVEN; the two stripped warnings' text carrying the CLI's
          command prefix in the window, reworded in the catalog and the spec's table), fixed
          in r1 (`.tmp/8_9j5/duck_rulings_j5.md`). Mutants-diff a5a2fa6f over r1: 22 caught,
          1 missed (the applied line's `&&`, S2-1's gap). SWE-2 S2: 2 accepted (that line after a
          Save; Apply's disabled state after Apply), the loop ends. The lead's look at the built
          window found two more, fixed in r2 with the S2 pair, red first: a Preview, Apply or
          Pick activated without a pointer press (Tab and Space, a screen reader) kept the
          keyboard focus and the run never started (the comparator's Pick too; the button now
          gives the focus up), and with a real save's path the pick row wrapped its buttons onto
          a line of their own (both panels; now one line, the path truncated)
        - [-] 8.9j5b the two-pane browser: dropped with the save-to-save transplant
        - [x] 8.9j5c remove the save-writing transplant (above); `spec/save_editor.md`'s and
          `spec/pes_savefile/`'s transplant text out, the Midcupping row saying why. Done
          2026-10-25 (SWE-2 in two runs, the five file deletions the lead's: the run denies
          deletes): the panel, `Session::transplant`, the CLI command and its two codes,
          `ops::transplant`, the help paragraphs; the scripts' slice kept as the test-only
          `test_support::copy_appearance` (the comparator's aesthetics scope and
          `ops/appearance_golden.rs` check against it; red run: the strip flag left uncopied
          fails `a_field_is_aesthetics_exactly_when_the_appearance_copy_moves_it` on
          `EditedStrip`); SE-TRN-01 to 03 withdrawn; the spec text by the lead. Files:
          `save_editor/{help/0{1,2}_*.md,src/{cli,lib,messages}.rs,src/session/mod.rs,
          src/view/{mod,menus,player_list,comparator,tests}.rs,tests/cli/main.rs}`,
          `pes_savefile/src/{test_support.rs,ops/{mod,compare,compare_golden,appearance_golden}.rs,
          interchange/aesthetics_patch/mod.rs}`, `docs/spec/{save_editor.md,pes_savefile/*,
          core/development_plan.md,README.md,export_upgrader.md}`, `plans/core/README.md`,
          `GLOSSARY.md`, `README.md`
    - [x] 8.9h the editor's tabs wrap inside a narrow window, as they do in a widget's own pane
      (found by 8.9e rework 7, 2026-10-23): the window has no minimum size, and in the editor's
      `ScrollArea::vertical` the content's width grows to fit its widest row (measured at a
      580 px window: the Tactics note's row reached 790 px), so a wrapping row overflows to
      the right, clipped, instead of wrapping; the wraps are tested only in `team_widgets`'
      pane → verify: the Tactics and Abilities tabs at a 580 px window, every control inside
      the window, by a test in `save_editor`'s view tests; folded into slice 4's layout if v2
      changes the host. Measured 2026-10-25 (SWE-2, `.tmp/8_9h/brief_8_9h_measure_r1.report.md`):
      every tab fits from 900 points (Abilities, Appearance; Tactics 780; the tab strip clips
      below 738), PES 17 and 21 alike. Ruled (DECISIONS "The window has a 960 × 600 minimum"):
      the window's minimum instead of wrapping rows → verify, replacing the 580 one: ST-WIN-01's
      minimum; every Save editor tab inside the pane at the minimum window. Done 2026-10-25
      (SWE-2): `window_size::MIN_SIZE` 960 × 600 in `native_options` (red: `min_inner_size`
      None), `every_tab_fits_the_pane_the_minimum_window_leaves` over four tabs on PES 21 and 17
      (red at an 804 pane: "Make Regular" ends at 798 past 796). Files:
      `studio_core/src/shell/window_size.rs`, `save_editor/src/view/tests/tactics.rs`.
      `mutants-diff 6fe6b3ab`: 2 of 2 caught
    - [x] 8.9i a starter whose portrait the pitch cannot show, for any reason (none in the
      CPKs, a portrait that does not decode, a `download/` folder that cannot be read), draws
      the game's own placeholder portrait instead of none (the maintainer's request of
      2026-10-23; `plans/save_editor.md` "Tactics editor", a marker and "Portraits"): the
      selected game's version of the three PNGs bundled in `resources/portraits/` (README:
      sources, hashes), embedded, decoded once, drawn as a portrait is → verify: a starter
      whose portrait the fixture CPK lacks, a portrait that does not decode, and a missing
      `download/` folder each draw the placeholder on the marker, each of the three versions
      chosen for its PES (2015, 2017, 2021); no starter with a real portrait gets it
      Done 2026-10-23 (SWE-2, 21 min): the three PNGs embedded in `view/portraits.rs`, the
      version's file by an exhaustive match, decoded and uploaded once per version; every
      filled slot gets its portrait or the placeholder, a portrait still being read included
      (DECISIONS 2026-10-23); the pitch test tells the starter's texture from the shared
      placeholder. Files: `save_editor/src/view/{portraits.rs,tactics.rs,tests/tactics.rs}`
    - [x] 8.9g the Tactics pitch as PES 2021's own (the maintainer's request of 2026-10-23;
      `plans/save_editor.md` "Tactics editor", the pitch's look): the reference measured first
      (the game's formation screen captured in game, its UI textures in the game's CPKs), the
      starters' portraits read from the selected game's `download/` CPKs; in the v2 draft
      first, with real portraits, then in `team_widgets::pitch`. Done 2026-10-23: both
      slices reviewed (SWE-2 S1-S3, Astra A1-A2, five mutation runs; rulings and triage in
      `.tmp/8_9g/duck_rulings_g.md`), seen in the built window on PES 2021
      - [x] the reference and the draft. Done 2026-10-23 (lead): PES 2021's Team Sheet/Edit
        Position captured in game (pad and OBS, /3/ and /f/ of the 4chan Cup save, backed up
        and unchanged after); its pitch texture decoded from `dt11_x64.cpk`'s
        `common/menu/general/gamePlan.bin` (WESYS, then an AFP whose TXP2 textures are
        Okumura LZSS streams of TXDT images: BC1/BC3 in byte-swapped 16-bit words, or ARGB8;
        `.tmp/8_9g/all_textures.py`) and matched to the capture (mean luma difference 1.7);
        bands, lines, grain, colours, marker geometry and the slot-to-badge mapping (fitted
        on ten slots within half a pixel; every keeper of the save at (52, 3)) measured into
        the plan. The v2 draft paints it, with the sample teams' portraits from both games'
        `download/` CPKs (138, `.tmp/8_9g/portraits.py`; PES 17's may be WESYS-wrapped) and a
        draft control for none; side by side with the capture it differs only by the
        overall rating the plan leaves out. Harness 53 of 53 (its SE-GUI-12 drop now picks
        a grass spot, since the markers grew). Files: `plans/save_editor.md`,
        `prototypes/save_editor_v2.html`
      - [x] slice 1, the look in `team_widgets::pitch`: the turf, lines, mapping, keeper spot,
        markers (badge, coloured name, captain mark, selection corners, draw order) and the
        snap through the same mapping, the portrait drawn from a texture the host hands in;
        then, in its rework, a wider name's scroll (the maintainer's idea of 2026-10-23,
        measured in game the same day: 13.5 s period, 40 px/s, 16 px gap; the draft scrolls)
        → verify: the mapping's fitted points, the keeper spot, the draw order, the snap
        round trip and the scroll's offsets by unit tests; the Tactics tab beside the draft
        at the same scale. Done 2026-10-23 (SWE-2, 38 min, and rework 1 for the scroll, 15
        min; the lead merged the scroll's two phase computations): the turf a 480 x 576
        texture made once per context, the markers interacted and painted top-down, the
        captain read from `set_pieces.captain` (the sidekick's correction of the brief), the
        scroll's clock in the context's memory restarting when a frame skips the pitch, a
        repaint asked for at the next name's move; `RosterRow::portrait` added, the host
        passing none until slice 2; the save editor's view tests ported to the new geometry.
        Rework 2 (SWE-2 review S1, 12 min): the keeper senses drag too, so a press on him where
        he covers another marker is his and moves neither (egui gives the drag hit to the
        topmost drag-sensing widget); the mutation run's 12 survivors tested (the position
        colours against literals, the grain's first pixels, a name as wide as the badge
        fitting, the corners' 9 px arms). Rework 3 (Astra review A1, 18 min): the name clock
        keeps its start across egui's discard pass (a frame can run the pitch twice; the frame
        number counts frames, not passes); the drag preview tested (the dragged marker painted
        last at the pointer, the pressed keeper at his spot).
        Seen in the built window (lead, a portable copy on a copy of the maintainer's PES 2021
        save, team /3/): beside the draft at 2x the same markers and lines, the keeper's
        badge at (160, 359), the halfway line on rows 191-192, every badge 50 x 14; a capture
        a second showed the two wide names moving about 4 s and holding about 9 s of a
        13.5 s period, the narrow one never moving. Portraits are seen with slice 2. Files:
        `team_widgets/src/{pitch/{mod.rs,popover.rs,lineup.rs,tests/*},tactics/mod.rs,
        test_support.rs}`, `save_editor/src/view/{tactics.rs,tests.rs,tests/*}`
      - [x] slice 2, the portraits: the save editor reads each starter's portrait from the
        selected game's `download/` CPKs (the sidebar's PES and the PES folder setting) in
        DpFileList order (the last listed wins; the
        compiler's portrait path: PES 15-18 `player_<id>.dds`, PES 19-21 `<id>.dds`; a
        WESYS-wrapped DDS unwrapped, as `dds_convert::decode` does), off the UI thread,
        kept until the game changes; none found, the marker draws without one → verify: a fixture CPK's
        portrait reaches its marker, a missing folder leaves every marker without one; the
        built window beside the draft with its portraits (`.tmp/8_9g/app_setup.py`,
        `app_ui.ps1`, `compare_built.py` drive and capture a portable copy). Done 2026-10-23
        (SWE-2, 30 min): the DpFileList reader moved to `cpk::dp_file_list` (a `thiserror`
        error, the same messages) and the portrait path to
        `pes_savefile::player_tables::portrait_path`, the compiler's five readers and two
        writers calling them; the editor's `view/portraits.rs`, one reading thread per selected
        game fed through `crossbeam-channel`, a failure logged once and every portrait absent;
        the lead logged a portrait that does not read or decode. Seen in the built window on
        PES 2021 (E:) with team /3/: every starter's portrait, medals included, as the draft
        draws them. Rework 1 (SWE-2 review S3, 11 min): a listed CPK that is missing leaves the
        pitch without portraits, as the game then loads none of `download/` (the brief had it
        skipped); tests for a changed selection starting over and an asked id asked once; the
        Tactics help topic describes the pitch's look and the portraits' source. Files:
        `cpk/src/{lib.rs,dp_file_list.rs}`, `cpk/tests/fixtures/{README.md,dpfl/*}`,
        `pes_savefile/src/player_tables.rs`, `team_compiler/src/{bins,paths.rs,templates.rs,
        upgrade.rs,processing}`, `team_compiler/tests/fixtures/dpfl/README.md`,
        `save_editor/{Cargo.toml,help/08_tactics.md,src/{lib.rs,test_support.rs,
        view/{mod.rs,portraits.rs,player_list.rs,tactics.rs,tests/tactics.rs}}}`,
        `spec/libs/README.md`, `scripts/provenance/fixtures/dpfl_fixtures.py`,
        `.cargo/mutants.toml`
    - [x] before slice 4 is briefed, the draft `docs/prototypes/save_editor.html` draws Live
      AATF (`plans/save_editor.md` "Configurable AATF rules", SE-GUI-30; the maintainer's
      request of 2026-10-21): the toolbar toggle, the docked bottom panel with its header,
      the list shrinking as a fix lands, the checking mark, the empty state. Done 2026-10-22
      (lead): under ALL the selected player's team is checked; the draft recomputes the
      sample's Stat, StatTotal, Height and rated-A rows from the edited player, keeps the
      team's rows (a draft control drops them, for the empty state); walked in Chrome
      (DBC7 on /f/: 48 issues, a stat fixed 47 and back on undo; official with the heights
      fixed: "No issues"; the toggle kept over a reload). Waits for the maintainer's remarks
      with the rest of the draft
    - [x] before slice 4, measure where one team's AATF check spends its time and cut what is
      not the script's own work. Done 2026-10-22 (lead): 56 ms a team, the host's part (the
      team's view, its copy, the call) under 1%, `read_player` 50 ms of it
      (`.tmp/8_9/aatf_timing/`); `rhai` built with `sync` so `Rules` is shared across threads,
      `check-aatf` checks its teams in parallel (PES 2017 save, 220 teams: 13.2 s to 2.0 s,
      output identical; DECISIONS 2026-10-22). Files: `Cargo.toml`, `aatf/src/rules.rs`,
      `save_editor/{Cargo.toml,src/cli.rs}`, `spec/aatf_rules.md`, `plans/save_editor.md`
    - [x] 8.9m the script's own work: `logic.rhai`'s `read_player` profiles every tier for each
      player, 2.2 ms a player, because rhai copies every map argument per call (measured
      without each part: choices' `option_used` 22 ms of the 56, the two walks over the
      specials 38 ms with the choices they feed). A rewrite that copies less (one card summary
      per choice rather than per option, the specials' conditions read once per player)
      updates `LOGIC_VERSIONS`' hash and must leave every fixture's violations unchanged;
      before logic version 1 is frozen at the 0.0.1 cut (the maintainer, 2026-10-25;
      DECISIONS 2026-10-25). Done 2026-10-25 (SWE-2, one rework: the marker read directly
      rather than through a copying `flag` call, `if` expressions, no `profile` overload):
      logic 1's hash `ea75b5fb…`; the 424-save census's output identical (2,775,487
      finding lines, `.tmp/8_9m/census/`), `check-aatf` at 0.85 of its time on the 12
      biggest saves, alternated (`paired_timing.txt`). A further cut, if one is wanted:
      `apply_effects` copies the profile per applied special. Files:
      `aatf/{logic/logic.rhai,src/file.rs}`, `docs/spec/aatf_rules.md`
    - [ ] 8.9k slice 5, the interchange dialogs (`plans/save_editor.md` "Build order and
      verification" item 5): Team TOML export and import with its sections (SE-GUI-16), the
      import dialog for `.4ccs`, `.4cct` and Texport (SE-GUI-17, with SE-GUI-22's and 35's
      aesthetics section under both settings), Apply aesthetics patch with its preview
      (SE-GUI-18), the Texport export checked in game per version (SE-GUI-29, manual); before
      the 0.0.1 cut, which waits on the whole view (`plans/core/development_plan.md`
      "Divegrass Before Christmas"). Not itemized until 2026-10-25, found while listing what
      0.0.1 lacks. The libs hold every operation (`pes_savefile::interchange`:
      `aesthetics_patch` parse/apply, `team_toml` parse/apply, `legacy::read_squad`/
      `read_tactics`, `texport` read/write/`to_team_toml`); the draft has every panel
      (`docs/prototypes/tools/save_editor.js` `patchPanel`, the interchange dialog from the
      comment "One dialog for Team TOML, .4ccs, .4cct and Texport", `ixExportToml`,
      `ixExportTed`; Team menu "Import / export team file…", Savefile menu "Apply aesthetics
      patch…"). Slices, each one review (draft check against the app's theme first, brief,
      sidekick, review, merge, release-build check):
      - [x] k5a 2026-10-29 Apply aesthetics patch, the Savefile menu's floating panel (SE-GUI-18, and
        SE-GUI-22's "Apply aesthetics patch is enabled"): the preview of every player it
        changes, old and new, Apply as one undoable edit, another version's patch refused
        with the save untouched; drafted 2026-10-29 (a skipped team's warning line, the
        preview following the open save, its rows in the unsaved-changes table's columns: plan
        "Read-only aesthetics", DECISIONS). Done: `view/patch.rs` (the panel, its pure
        `preview`), `prompt::changes_table` shared with the unsaved-changes prompt,
        `comparator::panel_window` shared with the Comparator, the Savefile entry and the
        pre-Fox ownership link opening it, one floating panel at a time; 12 view tests
        (`view/tests/patch.rs`). Release build checked on the real PES 21 save, read-only:
        the preview of team 739 with team 999's warning line, a row's click selecting
        TAKUMI, a PES 2020 patch refused with the save untouched; the lead's fix there, the
        table's scroll area shrunk to its rows (the panel had grown ~200 points of blank)
      - [ ] k5b the interchange panel's import half (SE-GUI-16's import, SE-GUI-17's import,
        SE-GUI-22's and SE-GUI-35's import-dialog parts): the file picked decides the format,
        the sections the file holds offered, the legacy formats read-only, the version
        mismatch's warning list, the aesthetics section under the ownership setting. The
        lib applies a Team TOML whole (no section choice), so two sub-slices:
        - [x] k5b1 2026-10-29 the section filter: a save_editor function that keeps only the chosen
          sections' keys of a `TeamToml` before `Session::import_team`, split as the plan's
          "Interchange formats in the UI" says (4ccEditor's `PlayerImport`, re-read
          2026-10-29: `mo_fk` under `importStats`); the draft's writers corrected to it the
          same day (they carried the free-kick motion with aesthetics). Done:
          `session/import_sections.rs` (`Sections`, `keep_sections`, every struct
          destructured with no `..`, emptied player tables dropped), 7 tests on the PES 19
          fixture's /co/ and /3/ (PES 21's are identical placeholders); the module is
          `#[cfg(test)]` until k5b2 calls it
        - [ ] k5b2 the panel's import half on it (`legacy::read_squad`/`read_tactics`,
          `Texport::from_bytes` + `to_team_toml`, the notes list, the `.4ccs` pairing), the
          file pick row as the Comparator's, the floating panels' open flags folded into one.
          Built and merged 2026-10-29 (`ff300ec0`): `view/interchange.rs` (the format from
          the file's name, the import staged on copies of the selected team and its rostered
          players, the sections held derived from the document, the notes grouped in the
          command line's words, Apply through `edit_entries`), `Option<Panel>` for the three
          floating panels, the Team menu's entry; 13 view tests (`view/tests/interchange.rs`,
          the base-copy own-id test the lead's, from S1c-1). Versions read `PES 2021` as the
          rest of the app (the draft followed). Release build checked on the real PES 21
          save, read-only: a PES 21 Team TOML, a real `.4ccs` (98HU's) with its pairing, a
          PES 17 Team TOML with its notes (`.tmp/8_9k5/built/b2_0*.png`). The notes'
          scroll area (rulings L1) landed with k5b2r
      - [ ] k5c its export half (SE-GUI-16's export, SE-GUI-17's export): Team TOML equal to
        the CLI's export; Texport for PES 18-21 read back to the same team values; disabled on
        PES 15-17 with the missing template in its tooltip. Built and merged 2026-10-29
        (`c8fe3e1a`): the export row (Save dialogs, the open save refused as `export-toml`
        refuses it, a success notice), `pes_savefile::file::fresh_salt` public for the
        Texport's key; 8 tests (`view/tests/interchange_export.rs`: the Team TOML byte-equal
        to the CLI's; PES 19 and 21 Texports read back with no value changed). SWE-2 round
        1: 3 accepted, the loop ends (rulings S1g; its code and tests ride k5b2r's part 2,
        `.tmp/8_9k5/brief_k5b2r2.md`, merged with k5b2r: the key seeded from ids, the
        CLI's write-failure words, PES 18 to 21 round trips); the `help/` paragraphs for the
        patch panel (`07_savefile.md`) and the team file panel (`05_team.md`). Open: the
        built-window check
      - [x] k5b2r the rework of SWE-2 round 1 on k5b2 and round 2 on k5a
        (`.tmp/8_9k5/rulings_k5.md` S1d-1, S1e-1, S1f, S2b-1 to 3, L1) and of k5c's round 1
        (S1g): an import that changes nothing disables Apply and says so (drafted, plan,
        DECISIONS 2026-10-29), the notes in a scroll area, the no-team line `Pick a team row
        to export or import.`, the missing tests. Merged 2026-10-29 (`06c2a6b4`;
        `view/interchange.rs`, `view/patch.rs`, their tests). SWE-2 round 2 on k5b2's tests: 2
        accepted, the loop ends (S2c); its four tests merged `0c5571d3` (the squad's shirt
        numbers, the lock flipped after a pick, the intro line, a player row's team)
      - [ ] k5d SE-GUI-29 manual: each Texport imported by the game's own team import in PES
        18, 19, 20 and 21 (21 on E:; 18-20 on F:, connected on request; saves backed up first)
      - [ ] k5rev the cross-family review loop (the slices are Opus sidekicks', so the lead's
        review is same-family): SWE-2 rounds on each merged slice until one accepts under
        five, one Astra round over k5a + k5b, another over k5c, SWE-2 rounds after each;
        rulings in `.tmp/8_9k5/rulings_k5.md`. Started 2026-10-29: SWE-2 round 1 on k5a, its
        code and its tests in two briefs (`review4/.tmp/k5a_review/`): code 1 accepted (the
        preview held until an edit settles, Apply recomputing a held one), tests 7; the
        rework merged `6be3f9e2` (ten tests); round 2 on the tests under way. Round 1 on
        k5b1: 1 accepted (an import must not retarget the document's `[team] id`, which the
        base-copy own-id rule reads), carried into k5b2's review
      - [ ] k5mut the loop's `mutants-diff` over k5a-k5c, survivors triaged
    - [ ] 8.9m the maintainer's pass over the 0.0.1 candidate (`d317b6da`), 2026-10-29:
      - [x] the drafts take the app's grey for a plain player name and a planned tool
        (DECISIONS 2026-10-29)
      - [x] m1 the code fixes (brief `.tmp/8_9m/brief_m1.md`): the badges' medal outlines
        computed off the UI thread (`player_tier` ×23 took ~95 ms of the team view's first
        frame on the real save); the player list's scroll track without its black fill; the
        list pane's and the quick actions' separator lines gone, widths kept; the lineup
        lists filled in the extreme colour, as the drafts' `.llist`; the lineup lists'
        rects unchanged by the Live AATF dock
        Done (`77502f57`, merged `c83d8d28`): the medals on `worker.rs`'s cycle
        (`tactics::Medals`, started once an edit settles); the scroll area in a scope
        clearing `extreme_bg_color`; both separators off, the list pane's right margin 3 → 4;
        `lineup::list` filled and `min_scrolled_height(LIST_HEIGHT)` (the detail's scroll
        area gave the lists only its visible height, egui's 64-point minimum took them to
        74). SWE-2 round S1m: 1 counted (`.tmp/8_9m/rulings_m.md`), which proved wrong
        (S1m3-2). Files: `save_editor/src/view/{mod,tactics,player_list,ruleset_picker,
        worker}.rs`, `team_widgets/src/pitch/lineup.rs`, their tests
      - [x] the drafts round the pitch's, the lineup lists' and the player list's corners
        by 2, as the other framed panels (DECISIONS 2026-10-29)
      - [x] m3 the app rounds them too (after m1, which touches the same frames): the
        player list's rows frame and the pitch's border at the noninteractive corner radius,
        the turf clipped to the border's inner curve; the lineup lists already round theirs
        Done (`22a7d0f6`, `9211b365`, merged `c83d8d28`): the rows frame's fill and stroke
        and the pitch's border at radius 2, the turf a textured `RectShape` at 1. A row the
        viewport cuts, and the first team row's faint backdrop, reach under a pixel past
        the curve (egui clips to rectangles only): accepted. SWE-2 round S1m3: 1 counted,
        the quick actions' `.resizable(false)` dropped (egui's `Panel::bottom` is not
        resizable). Files: `save_editor/src/view/{player_list,worker}.rs`,
        `team_widgets/src/pitch/mod.rs`, their tests
      - [ ] m4 the light theme's widget greys and warning colour (the maintainer, 2026-10-29;
        DECISIONS "The light theme's widget greys a step lighter, its warning colour at
        4.5"): `theme.rs` `widgets_light` resting 207, hovered 192, pressed 214, the light
        `warn_fg_color` `#C54B00`; its tests pin the values (the contrast-matching test
        rewritten to the new rule) → verify: the built window's light theme, the sidebar's
        version badge and a number box
        Code done (`8d2ab7c0`, `c60c80a2`, merged `9d9fc444`): the fills and `#C54B00` with
        their tests; the Live AATF pitch's warning cross in `Palette::pitch_warn`, `#FF8F00`
        on both themes, since the turf does not follow the theme. SWE-2 round S1m4: 2
        counted (`.tmp/8_9m/rulings_m.md`). Open: the built window's check
      - [x] m5 2026-10-29 the running-game texts (the maintainer's PES 2017 check of SE-GUI-21,
        2026-10-29; DECISIONS "The running-game dialog says how to keep the write, Edit mode
        open or closed"): the window's dialog and help say to Load before leaving an open
        Edit mode, and that a closed one shows the changes when opened; the Save editor's
        CLI refusal and help, and the compiler's `savefile_skipped_pes_running` text and
        help, state the corrected facts, `--edit-mode-closed` unchanged (brief
        `.tmp/8_9m/brief_m5.md`) → verify: the texts' tests, the help export
        Done (`1cf906e5`, merged `003dc3df`; Opus sidekick): the texts as briefed, test
        `the_question_says_how_to_keep_the_write` (red: "the open Edit mode's sentence");
        the lead reworded three doc comments ("when the member leaves Edit mode after
        changing anything there"); the sidekick's contradiction accepted:
        `spec/team_compiler/pipeline.md` "Savefile update" quoted the old shared words. The dialog's state keeps the name
        `EditModeQuestion`. Gates in its worktree: `save_editor`, `team_compiler`, `studio`
        tests, clippy, fmt, `comment_refs` green (`.tmp/8_9m/m5_gates.log`). SWE-2 round
        S1m5: 4 counted, the loop's end (`.tmp/8_9m/rulings_m.md`): the drafts now draw the
        running-game dialog where they still refused, the host's CLI help copy follows, and
        four comments no longer call the dialog's Save the member's word. Open: the dialog in
        the built window
        Files: `save_editor/src/{cli,session/mod,view/prompt}.rs`, `view/tests/prompt.rs`,
        `save_editor/help/0{1,2}_*.md`, `team_compiler/src/{messages,messages/catalog,
        output/savefile}.rs`, `team_compiler/help/01_command_line.md`
      - [ ] m2 `scripts/draft_compare.py` compares the fill and text colours of the boxes it
        pairs, not only their rects: the fills and greys above passed every geometry check
    - [x] 8.9l 2026-10-29 the open/save scenarios no slice built: the unsaved-changes prompt on closing or
      opening another save (SE-GUI-05; only the toolbar's marker exists, so closing loses
      edits silently), the Open menu's discovered saves (SE-GUI-26), the version-mismatch
      warning (SE-GUI-28; the draft has it), and SE-GUI-21's check in the built window with the
      game running (its logic is tested); before the 0.0.1 cut. Slices:
      - [x] l1 the table's rows: `pes_savefile`'s `compare_teams` (the twin of
        `compare_players`), `team_widgets`' `tactics_term` and `save_editor`'s
        `team_row_terms`, a colour's three channels one row. Done 2026-10-26 (SWE-2;
        `tactics_term` takes the version and the tactics too, which name a formation slot's
        player and decode an instruction; the kit slot's team reads "Bound team ID" as the
        tab does; the lead's review: five labels lifted into one const each, explicit
        variants for the report rows). Landed with l2. Files: `pes_savefile/src/{codec/mod,
        ops/compare}.rs`, `team_widgets/src/{pitch/mod,tactics/*}.rs`,
        `save_editor/src/view/{terms,team,info_strip}.rs`
      - [x] l2 the prompt (SE-GUI-05): `hold_close`, the toolbar's Open, Save / Discard /
        Cancel as the Ruleset editor's prompt answers them. Done 2026-10-26 (SWE-2, with
        l1's fixes): `Session::unsaved_changes`, `view/prompt.rs` (the table computed once at
        open, a shirt number's row under its player), the window's Save asking the Edit mode
        question while the save's own PES runs (`Session::save`'s `edit_mode_closed`), the
        help's saving paragraph; the lead removed `Pending::OpenPath` and its suppression
        (l3 adds it with the menu) and reworded the question. Not yet seen in a built
        window: the plan's table "spans the prompt's width", which egui's grid may not do
        (l4's check). Files: `save_editor/src/{lib,session/mod,view/{mod,prompt,
        player_list},view/tests/{prompt,shell}}.rs`, `help/02_window.md`. Rework r1
        2026-10-26 (SWE-2, the lead's review and `mutants-diff 5459e8f6`'s survivors): the
        tests no longer read the real process list (`Tool::running`, a no-game stub under
        `cfg(test)`); a close asked under the Open… prompt retargets it; a failed Save's
        refusal draws above the modals; Strip again's change is a `Stripped` row; a player
        on two rosters gets his rows once; an instruction target row words each side under
        that side's instruction through `instructions::target_text`, the tab's own; the
        roster's raw names keep their colour codes; every preset and role entry's label
        pinned; `compare_teams` tested on every version. Files: `save_editor/src/{lib,
        session/mod,view/{mod,prompt,terms,comparator},view/tests/{prompt,shell}}.rs`,
        `team_widgets/src/tactics/{mod,instructions,assignments,tests/mod}.rs`,
        `pes_savefile/src/ops/compare.rs`. Rework r2 2026-10-26 (SWE-2, SWE-2 round S2's eight
        accepted concerns, `.tmp/8_9l/rulings.md`): the Item column reads the display name;
        the refused-write test puts a directory in the save's place, refused on every OS (a
        read-only file is replaced on Linux, which failed the mutation run's remote
        baseline); backdrop clicks tested as each modal's Cancel; the prompt's Save found as
        the modal's own; `roster_names` and `roster_for` pinned. Files:
        `save_editor/src/view/{prompt,tests/prompt}.rs`. Rework r3 2026-10-26 (SWE-2, Astra
        A1's five accepted concerns): a team row's old side names the baseline's roster and
        records, the new side the current ones (`tactics_term` takes both rosters); the id
        lookups go through maps built once, an item's name only when a row names it;
        `colour_term` reads the two entries; the refusal under the Edit mode question tested;
        the tactics labels pinned as literals. The colour tests were rebuilt on real entry
        pairs (their old diffs named states `compare_teams` cannot produce). Files:
        `save_editor/src/view/{prompt,terms,tests/prompt}.rs`,
        `team_widgets/src/tactics/{mod,tests/mod}.rs`. Review closed: SWE-2 S1-S4, Astra
        A1-A2, A2 with no concerns (`.tmp/8_9l/rulings.md`)
      - [x] l3 2026-10-29 the Open menu with discovery and the session's recent files, Save as (SE-GUI-26),
        the version warning (SE-GUI-28), a status-bar notice since 0.0.1s2. The shell part is
        decided (`plans/core/gui.md` "Status bar", the bullets "A tool can keep a standing notice" and
        "A notice can switch the PES version"; DECISIONS 2026-10-29); draft checked
        (`.tmp/8_9l/draft/warning_{free,blocked}_bar.png`: the blocked Switch button greyed,
        its hover the selector's reason)
        - [x] l3a 2026-10-29 (Opus sidekick): `StudioTool::standing_notice`, asked of the
          active tool before the requests drain, and `standing_notice_ended`, told once when
          seen; `StatusBar::stand`, a switch of tool withdrawing it after telling the outgoing
          tool of an end seen earlier in the drain (the sidekick's catch: the brief's order told
          the tool switched to); `ActionEffect::SwitchVersion` through `pick_version`, disabled
          under a blocker with the selector's hover (ST-BAR-07, ST-BAR-08). Files:
          `studio_core/src/{tool,status}.rs`, `studio_core/src/shell/{mod,status_bar,
          test_harness}.rs`. Astra's critique of the plan and this code: one concern, the
          title test, added in `cdfb3fd9`
        - [x] l3b 2026-10-29 (Opus sidekick): the Save editor's version warning as a standing
          notice (SE-GUI-28): `SaveView::version_warning` set by every open, cleared by
          `standing_notice_ended`; `version_notice` a Warning naming both versions while the
          selector differs, hidden while they match, its action `SwitchVersion` to the save's.
          Help `02_window.md`. Files: `save_editor/src/{lib,test_support}.rs`,
          `save_editor/src/view/{mod,tests/shell}.rs`. Its report's question (a GUI
          scenario's test in the crate citing its scenario, which "Unit tests cite none" seemed
          to forbid) answered in `CONTRIBUTING.md` "Testing"
        - [x] l3c 2026-10-29 (Opus sidekick): the toolbar's Open menu (every discovered save
          for every version with its account folder, discovery run once per opening; this
          run's opened and written files; Choose file…, starting in the selected version's
          save folder) and Save as (`Session::save_to`, the replaced file kept as its `.bak`,
          the running-game question remembering its target); discovery and the pickers
          injectable (SE-GUI-26). Its test, on the real account folder names, found discovery
          skipping the 17-digit Steam ID64 folder of PES 2020 (fixed in `196de046`). Help
          `02_window.md`. Files: `save_editor/src/view/{open_menu,mod,prompt,comparator}.rs`,
          `save_editor/src/{lib,session/mod}.rs`, `save_editor/src/view/tests/{open_menu,
          prompt,tactics,toolbar,overlap}.rs`. Rendered against the draft's Open menu
          (`.tmp/8_9l/{draft,built}/open_menu.png`): the entries and the PES 2020 save as
          drawn, the geometry off in every menu and the toolbar (l3d)
        - [x] l3a-c's mutation run (`mutants-diff 7f23e772` at `39c0de34`, local, the minimum
          test timeout 120 s): pes_savefile 3 caught; save_editor 32 caught, 3 unviable;
          studio_core 18 caught, 3 unviable, 1 timeout (`take_standing_end` -> `true`), which
          a quiet rerun catches in 1.3 s (`a_standing_notice_stays_while_its_tool_returns_it_and
          _its_tool_hears_when_it_ends`): load, not a hang. No survivor, no rework
        - [x] l3d 2026-10-29 the drafts take egui's spacing (DECISIONS 2026-10-29 "The drafts take egui's
          spacing"), seen in l3c's render: every menu's entries 21 apart against the draft's
          19, the toolbar's buttons 8 apart for 6, every view 8 from the central panel's edges
          for 12, because the drafts' spacing was picked in a browser and each difference was
          made up in the app or missed (the menus were never measured: `rect_compare.py` sees
          closed menus only). A first brief, the app following the drafts, was stopped
          (`wt-l3d`, dropped).
          - [x] l3d1 2026-10-29 (lead): the drafts' `.menu`, the menu's place under its button,
            `.center`'s side padding, the Save editor's `.toolbar` gap and right margins at the
            theme's values; the draft's menus and toolbar then within a point of the built
            window (`.tmp/8_9l/draft_menu_rects.py`, `measure_toolbar.py`); the discovered
            entry's one space in the draft's source. Plan: `plans/core/gui.md` "Prototypes
            before implementation", the Look bullet
          - [x] l3d2 2026-10-29 (Opus sidekick): the Save editor drops its panes' hand-added 4
            points (the list's left margin 0; `PANE_RIGHT_MARGIN` gone, the detail frame's
            right margin 0, since its own 8 would double the shell's: the brief's "egui's" was
            wrong) and the Open menu's label takes one space; the tests pinning the old
            geometry follow the draft (the list frame 64 to 309, the pane's right 1272, the
            four gaps 30.4, the Tactics bars 4 left). Files: `save_editor/src/view/{mod,
            open_menu}.rs`, `save_editor/src/view/tests{,/abilities,/list,/open_menu,
            /quick_actions,/tactics,/team}.rs`. Re-measured on a release build of `8a13c577`
            (`rect_compare.py player|team`, `.tmp/8_9l/measure_toolbar.py`): every delta the
            same as ov2y's accepted table (the views moved with the draft), the ability labels
            left to ov13; the toolbar's runs the draft's to the pixel but the ruleset combo 2
            wider (as before); the Open menu's entries 1 high
          - [x] l3d3 2026-10-29 every other draft spacing that differs from the theme's, each changed to
            the theme's or kept with its reason in the stylesheet (DECISIONS "The drafts' gaps
            between controls are egui's item spacing")
            - [x] l3d3a the drafts and the plan (lead): `.row`, `.log-header`, `.live-header`
              and `.qagrid` across 6 to 8, `.plist` 4 to 3, `.rail` 2 to 3, the Formation
              settings' choices 18 high; the layout gaps kept, named in the `.row` rule's
              comment; `draft_compare.py` measures a text's ink without the spaces around it
              (the team view's "Manager ID " read 3 wide). Measured on the `8060aeef` build:
              the team view 0 over 2 px before the `.row` change (only the header row's units
              move after it, by the app's 6), the player view the same 15 at 1 to 2 px as
              before (the Skills header's units move). Files: `docs/prototypes/{index.html,
              tools/save_editor.js}`, `plans/save_editor.md`, `scripts/draft_compare.py`
            - [x] l3d3b 2026-10-29 (Opus sidekick, merge `01ff1da1`) the build drops the
              overrides (brief `.tmp/8_9l/brief_l3d3b.md`): `team.rs` `ROW_GAP` (the header's
              claims Name 182, Short 79), `abilities.rs` `HEADER_GAP`, `menus.rs`
              `BOGLO_RANGE_GAP` (the range's gap read from the theme), `player_list.rs`
              `CONTROL_GAP`, `quick_actions.rs` `COLUMN_GAP` and `ROW_GAP`, `studio_core`'s
              rail `RAIL_GAP` (its last entry ends at 533 of the 756-point rail). At review,
              the sidekick's four contradictions accepted: the list's quick actions panel 3
              under the rows' frame (it kept the old 4), the draft's rail foot 3 apart, the
              draft's boots-and-gloves column 127 (the range is 126.4 now), the quick
              actions' `ROW_GAP` gone (the theme's 3). Checkpoint `gates-changed 0365e3a9`
              green (5256 cargo tests). On a release build of `01ff1da1`: player and team 0
              over 2 px; left at 1 to 2 the accepted rows (the Formation drift, the
              checkboxes' and the team header labels' 1.5 baseline). Files: `save_editor/src/
              view/{team,abilities,menus,player_list,quick_actions}.rs`, their tests,
              `studio_core/src/shell/sidebar/{mod,tests}.rs`
            - [x] l3d3c 2026-10-29 (lead) the rest the measurement found: the Kit slots
              dialog's sentence and OK 4 and 7 points higher than the draft's (the draft's
              sentence was a `.row`, whose margins the dialog's own spacing doubled; 3 of 3
              within 1 point after); the Formation settings' rows drifting to 1.5 lower in the
              draft (its rows 58.5 apart, the build's 58: both share the panel's free height,
              `1fr`, the frames themselves within a point; accepted at 1 to 2, the cause of
              the build's 3 points less not found). `draft_compare.py --menu TEXT` measures an
              open toolbar menu (the draft's clicked, the build's rows kept where the draft's
              menu stands); `uia_dump.ps1` writes UTF-8 (the console's code page had turned
              `…` and `⏷` into `.` and `?`, so no such name paired) and a draft menu entry's
              shortcut is joined with a space, as egui names it. On the `3cd0f907` build:
              Player 7 and Team 6 entries within 1 point; Savefile and Open differ only by
              entries not built yet (8.9k slice 5, 8.33) or the draft's sample recent file,
              which widen the draft's menu and push the rows under them
            - [x] l3d3d 2026-10-29 (Opus sidekick, merge `966e15d9`) the team view's colour
              codes in the weak grey, and the colour field's swatch outlined as a resting text
              box is (DECISIONS "The colour codes in the weak grey, the colour swatch
              outlined"; `studio_core::widgets::text_box_border`). On the `f74dbd7e` release
              build: team 0 over 2 px, the ink check 0 colours apart; the black swatch's
              outline seen (`.tmp/8_9l/built3/team_row.png`). Files: `studio_core/src/widgets/
              text_box.rs`, `color_tools/src/picker.rs`, `save_editor/src/view/team.rs`, tests
            - [x] l3d3e 2026-10-29 (Opus sidekick, merge `f74dbd7e`) the rail's logo the 4cc
              clover (DECISIONS "The rail's logo is the 4cc clover"): the palette's
              `clover_ring` and `clover_leaf`, `logo` drawing U+F100 and U+F101 at 28. Its ink
              test found the glyphs 98 units high (centred on the symbols' 2027/431 line, not
              the body font's 1900/500), fixed in the generator and the font (`5b505b94`); the
              draft's logo keeps its place when the sidebar opens (`c4021640`), the build's
              test pins it. On the release build: the clover's ink 28 x 28 centred on the
              square within 0.5, the same closed and open. Files: `studio_core/src/theme.rs`,
              `shell/sidebar/{mod,tests}.rs`, `resources/symbols/f10{0,1}-*.svg`, `symbols.ttf`
            - [x] l3d3f 2026-10-29 (lead, `712966d8`) the opened sidebar without a shadow,
              edged by the rail's 1-point line (DECISIONS "The opened sidebar casts no
              shadow"): `sidebar/mod.rs` `overlay_frame`, the line in `show_overlay`, test
              `the_overlay_casts_no_shadow_and_ends_on_the_rails_line` (red twice: the shadow,
              then the missing line). On the release build: one grey-60 column at x 194, no
              shadow. Files: `studio_core/src/shell/sidebar/{mod,tests}.rs`
            - [x] l3d3h 2026-10-29 (sidekick Opus, `2b156a29`) the opened sidebar's labels
              where the draft puts them, 6 after the 24-point icon box (they were after the
              whole icon cell plus egui's 8, at 57) with 6 of padding after the longest:
              `label_offset`, `LABEL_GAP`, `LABEL_END_PADDING`; test
              `the_opened_sidebars_labels_and_width_match_the_draft` (every label at 47, the
              layer 190.63, within 1). On the release build: label ink at 47-48 and the
              layer's line at x 189, as the draft's. Files:
              `studio_core/src/shell/sidebar/{mod,tests}.rs`
            - [x] l3d3i 2026-10-29 (sidekick Opus, `2bc57823`) the rail's version selector: its year centred under the logo with no
              arrow (the opened sidebar's unchanged), and the hover delay 100 ms (DECISIONS
              "The rail's version selector centred without its arrow; the hover delay 100
              ms"; `plans/core/gui.md` "Sidebar" items 2 and 4; the draft's `#sidebar
              select`, lead, done, `.tmp/8_9l/pairs/rail_selector.png`); brief
              `.tmp/8_9l/brief_l3d3i.md`. egui's combo kept: on the rail a zero-wide arrow slot and
              an empty growing atom before the year; test
              `the_rails_selector_centres_its_year_without_an_arrow_and_the_opened_one_keeps_both`,
              the delay's sides pinned at 1.09 and 1.11. 120 ms tried in the draft and
              dropped (DECISIONS "The sidebar's hover delay stays 100 ms"). On the release
              build: `21` centred without an arrow, `PES 2021` and its arrow open
              (`.tmp/8_9l/built3/l3d3i_pair.png`). Files:
              `studio_core/src/shell/sidebar/{mod,tests}.rs`
            - [x] l3d3j 2026-10-29 (sidekick Opus, `21e9da9a`) a press on the rail's version
              selector opens the sidebar at once and the list drops from the opened selector,
              full names whatever the timing (DECISIONS "A press on the rail's version
              selector opens the sidebar at once"; the draft's `pesSelectorFromRail`): the
              field `selector_pressed`, the popup's entries always `version_label(version,
              false)`; test `a_press_on_the_rails_selector_opens_the_sidebar_at_once_with_the_full_names`.
              The harness never paints the rail's two-digit list (a popup's first frame is
              egui's invisible sizing pass), so the short list the maintainer saw was most
              likely the draft's (inferred, not reproduced). On the release build: a click on `21` shows the opened sidebar with
              `PES 2015`..`PES 2021` (`.tmp/8_9l/built3/l3d3j_pair.png`). Files:
              `studio_core/src/shell/sidebar/{mod,tests}.rs`
            - [x] l3d3k 2026-10-29 (sidekick Opus, `4db21200`) every grayed sidebar entry's
              tooltip "Coming soon™" (CLI-only ones too: the maintainer confirmed the
              command-line hint goes, CLI use is for select advanced users), the tail's labels
              and the two windows' titles "Help (WIP)" and "Settings (WIP)" (DECISIONS
              "Disabled sidebar entries say \"Coming soon™\"; Help and Settings marked WIP"
              and "The Help and Settings windows' titles marked WIP too"): `PLANNED_TOOLTIP`,
              and `HELP_LABEL`, `SETTINGS_LABEL` each feeding the entry's label, its accessible
              name and its window's title. On the release build: the tooltip, both labels and
              both titles (`.tmp/8_9l/built3/l3d3k_*.png`). Files: `studio_core/src/shell/
              {sidebar/{mod,tests}.rs, help_window.rs, settings_menu/{mod,tests}.rs, mod.rs}`
            - [x] l3d3g 2026-10-29 (lead, `712966d8`) the Windows title bar without the icon
              (DECISIONS "The title bar draws no icon on Windows" and "The title bar's icon
              slot stays blank"): `window_size::hide_title_bar_icon`, one `unsafe` call. On the
              release build: no icon, the title where it stood (the maintainer's pick over
              `WTNCA_NOSYSMENU`, which moves it but loses Alt+Space's window menu, measured on
              the candidate's window, `.tmp/8_9l/built3/titlebar_options.png`). The taskbar
              and Alt+Tab icons are the maintainer's look (QUESTIONS "The final pass").
              Files: `studio_core/src/shell/{mod.rs,window_size/mod.rs}`, `Cargo.toml`
            - [x] l3mut 2026-10-29 (Opus sidekick, merge `4c5754f9`) the loop's
              `mutants-diff 0365e3a9` (61 mutants: 46 caught, 2 missed, 4 unviable, results
              `.tmp/8_9l/mutants_r3/`): both misses were l3d3b's layout values under the draft
              tests' 1-point tolerance (`id_cell_padding_before`'s `-` to `/`, the quick
              actions' `top: 3`), now pinned exactly, each red with its mutant. Files:
              `save_editor/src/view/tests/{team,quick_actions}.rs`
            - [x] l3mut2 2026-10-29 (lead) the second loop's `mutants-diff d28e93d0` over
              l3d3d to l3d3k (52 mutants: 41 caught, 1 missed, 8 unviable, results
              `.tmp/8_9l/mutants_r4/`): the miss, `hide_title_bar_icon` replaced with `()`,
              needs the native window's handle and changes only what Windows draws, so no
              headless test can see it; excluded in `.cargo/mutants.toml` beside `run_gui`
              (checked: listed with `--no-config`, gone with it). Files: `.cargo/mutants.toml`
      - [x] l4 2026-10-29 SE-GUI-21 in the built window with the game running (lead,
        manual). SE-GUI-21 manual: checked 2026-10-29 (lead, release build of `4db21200`, PES 2021 from
        `E:/PES2021` driven by the pad, the maintainer's live save backed up first and restored
        by hash after, `.tmp/se21b/`): with the game running, Save on a renamed player (/3/'s
        QUADS ONLY to QUADS ONLYX) opened the dialog naming PES 2021 and asking to close Edit
        mode; Cancel left the live EDIT00000000's hash unchanged; with Edit mode closed in game,
        the dialog's Save wrote it (hash changed); back in Edit mode, Load ("Rigging Edit Data
        will erase current data") then Edit Player showed QUADS ONLYX, no restart
        (`.tmp/se21b/shots/12_dialog.png`, `11_after_load_squad.png`)
      - [x] l5 the same rule for the command line and the compiler
        - [x] l5a 2026-10-29 (Opus sidekick): the Save editor's `import-toml`, `apply-patch`,
          `strip-and-seed` and `check-stripped --strip-again` take `--edit-mode-closed`; with
          the save's own PES running they write only with it, and without it write nothing and
          exit 2 in the words both tools share. `session::refuse_while_running` takes the flag
          (the window's `Session::save` passes its answer through it). SE-CLI-03 built, one
          shared test checker over the four commands (saves compared by payload: each write
          re-encrypts under a fresh salt), and moved to the spec in SE-CLI-02's place (now
          withdrawn); the spec's "Writing a save" says the rule, the plan section keeps the
          window's half. Help: `01_command_line.md`. Files: `save_editor/src/{cli,session/mod}.rs`,
          `save_editor/tests/cli/{main,import_toml,apply_patch,strip}.rs`,
          `save_editor/help/01_command_line.md`
        - [x] l5b 2026-10-27 (side-lead U, SWE-2): the Team compiler's `compile` takes
          `--edit-mode-closed`, and with it the savefile step writes the save while the target's
          PES runs; without it `savefile_skipped_pes_running` skips as before, its line naming
          the flag (`flag=--edit-mode-closed`) and its catalog text and the help chapter saying
          to close Edit mode and compile again with it. The running-PES probe is the tool's
          (`Tool::with_running_probe`), so no test reads the machine's processes (the live
          probe failed nine savefile tests while PES 2021 ran). TC-SAV-17 built and moved to
          the spec in TC-SAV-10's place. The window's Edit mode question stays Phase 8's
          (`plans/team_compiler/gui.md`). Files: `team_compiler/src/{lib,cli,compile,gui_run,
          messages,messages/catalog,output/savefile}.rs`, `team_compiler/tests/{common/mod,
          cli/savefile}.rs`, `team_compiler/help/01_command_line.md`,
          `spec/team_compiler/{pipeline,settings,messages,README}.md`,
          `plans/team_compiler/{pipeline,gui,README}.md`. Review: SWE-2 S1 (6 accepted, 4 of
          them routed to the top lead) and S2 (1 accepted), `.tmp/tc_sav_17/rulings.md` on
          side-lead U's worktree
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
      to what it found
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
  - [x] 8.25 before the 0.0.1 cut: the sidebar as a rail that opens on hover over the view
    (the maintainer, 2026-10-25; `core/gui.md` "Sidebar" item 4, ST-SIDE-02; DECISIONS "The
    sidebar is a rail that opens over the view on hover"): the collapse button and the
    `collapsed` state removed, the full sidebar a floating layer opened by a rested pointer or
    focus, kept open while its popups are, as wide as its longest label; the tool icons, the
    help and the gear at 24 points (item 3, DECISIONS "The rail's icons are drawn at 24
    points"); the drafts' host (`docs/prototypes/index.html`,
    8.16's) the same. The code half goes next (2026-10-25), before 8.9f4c's rework r4, which
    moves the Save editor's tests to the pane the rail leaves (1224 x 776) inside the shell's
    panel and re-checks `TOP_ROW_ONE_LINE`, `TEAM_BAR_ONE_ROW` and the Tactics columns there
    → verify: ST-SIDE-02's test rewritten red-first (the view's x0 and width equal with the
    sidebar open and closed); the built window at 1280 x 800 by hand
    - [x] the drafts' half (side-lead A, merged as `7b5ed0ef`): the host's sidebar a 56-point
      rail of the app's Phosphor glyphs at 24 px (the font subset under
      `docs/prototypes/assets/fonts/`, its MIT notice in the page), the full sidebar a layer
      opened by hover or focus without moving the view, the collapse button and its control
      gone; the Save editor's Tactics tab in three columns at the default window. SWE-2 rounds
      9 of 15, 5 of 10, then 1 of 6 (`.tmp/8_25/rulings.md` in `../4cc-studio-wt-drafts`)
    - [x] the code half (2026-10-25, SWE-2 58 min and an 8-minute rework): the panel at
      `RAIL_WIDTH` 56, the widgets drawn once a frame, in the rail or in a `Foreground`
      `egui::Area` over the view, under one `scope_id` so a focus or the open version popup
      survives the move; opens after 0.15 s of rest, held by the pointer, the version popup
      (pinned above the overlay with `set_sublayer`), About or a keyboard focus (a click
      focuses nothing in egui 0.37), as wide as the widest label measured each frame; rows
      drawn alike in both places (icon cell, 24-point icons, the frame only when in use).
      Rework: no clippy opt-out (`SidebarFrame`), one `drawn_planned`, no gray inactive rows.
      Mutants-diff 9c632437: 15 missed, killed by five tests (the labels painted only in the
      overlay, the icon's and the label's place in a row, the accent bar on the active row
      alone, the overlay's margin band, a row focused and nothing else).
      Files: `studio_core/src/shell/{mod.rs,sidebar/{mod.rs,tests.rs},test_harness.rs}`,
      `studio_core/src/tool.rs`. manual: checked 2026-10-25 on the release build at 1280 x
      800 (`.tmp/8_25/look_{closed,open,left}.png`): the rail's 24-point icons, the overlay
      on a rested pointer with the labels beside unmoved icons and `PES 2021`, as wide as
      "Player aesthetics editor", the view unmoved, closed once the pointer left
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
    - [ ] 8.22i `upgrade <export>...` (P20; `plans/export_upgrader.md` "CLI") → verify:
      EU-CLI-11
    - [ ] 8.22j the upgrader's view items (P18's pair, P19's dialog, P32-P36;
      `plans/export_upgrader.md` "GUI"), with the view (8.10), drafted first (8.17b) → verify:
      EU-GUI scenarios for each, written with the view's Acceptance before it is built
  - [x] 8.11 egui 0.36.2 to 0.37 (`spec/core/gui.md` "egui version discipline"; agreed
    2026-10-21; the maintainer asked on 2026-10-23 to migrate as soon as possible, so fewer
    GUI tests are written twice). No `egui-phosphor` release for 0.37 yet (0.14.0,
    2026-09-10, is 0.36's; its 0.37 pull request open since 2026-10-19; `egui_commonmark`
    0.26.0 shipped with egui 0.37), so the migration drops it: the 19 glyphs the suite uses
    cut from its regular Phosphor font (MIT) into `resources/fonts/phosphor-subset.ttf`
    (7 KB, the lead's, with its `README.md`), their constants (`studio_core::icons`) and the
    font's registration (`studio_core::theme`, today `egui_phosphor::add_to_fonts`) written in
    `studio_core`; plan edit and decision entry done. Brief `.tmp/egui37/brief_8_11.md`, with
    8.13. Run after 20.6a1 lands and before 20.6a2 is briefed, with no sidekick in UI
    code. Why: 0.37 draws any glyph the system's fonts have (members'
    names carry symbols, a Japanese team's names non-Latin text that the bundled fonts draw
    as boxes). The work: `Id::new` and `AsId` deprecated or removed and `Ui::id` renamed
    `scope_id` (deprecations are errors here); `default_fonts` no longer a default feature;
    the AccessKit tree changed (the headless tests find widgets by role and label); the
    picker's tests rerun with care, since 0.37 changes popups closing on release and
    `DragValue` while typing; the drafts' look checked against 0.37's roomier buttons and
    rounder corners; `egui_extras` and `egui_animation` to their 0.37 releases where used
    Done 2026-10-23 (SWE-2, 87 min, with 8.13): `eframe`/`egui` `=0.37.0`, `egui_commonmark`
    `=0.26.0`, `egui-phosphor` gone; 24 `Id::new` to `Id::unique` (same hashes), `Ui::id` to
    `scope_id`, `WidgetType` to `Role`; `LogPanel::new` takes an `AsIdSalt` (0.37 removed
    `From<&str> for Id`); 16 tests re-driven for the AccessKit changes (a popup's or modal's
    first frame is a sizing pass, links are `Link`, headers `DisclosureTriangle`, an empty
    label unset), no THEN changed; the Tactics tab's `ui.columns` replaced by the draft's
    layout (the pitch at its fixed width, the lists in the rest), since a narrow window's
    overflowing half widened the wrap bound of the settings rows below (DECISIONS
    2026-10-23); `studio_core::icons` and the subset font registered second in
    Proportional, eframe's system-font fallback kept. Files: `Cargo.toml`, `Cargo.lock`,
    `studio_core/src/{icons.rs,theme.rs,shell/*,widgets/*,status.rs,tool.rs}`,
    `color_tools/src/picker*`, `team_widgets/src/{pitch/*,tactics/*,violations.rs}`, the
    four tools' `lib.rs` and views, `save_editor/src/view/{tactics.rs,menus.rs,tests/strip.rs}`
    manual: checked 2026-10-23, a portable copy of the debug build on a copy of the PES 2021
    save (`.tmp/egui37/app_setup.py`, `printwin.ps1`): the rail's 19 glyphs drawn, the
    status bar `v0.0.1 pre-alpha (debug)`, the Tactics tab's pitch at 320 px with the lists
    beside it, and a name typed as `山田 タロウ ★ Иван` drawn whole in the Name field and the
    roster (system-font fallback); the drafts' look against 0.37's roomier buttons not
    compared side by side
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
  - [x] 8.13 the release stage after the version (the maintainer's idea of 2026-10-23;
    `plans/core/distribution.md` "Changelog and version display"): `v0.0.1 pre-alpha` in the
    status bar's left slot, the About block and `--version`, from one `studio_core` function
    deriving the stage from the version's major and minor numbers. Before 0.0.1 is cut; rides
    with 8.11's brief (both `studio_core`) → verify: the function's three stages by unit test
    (0.0.x, 0.y.z, 1.0.0), and the empty status bar, About and `--version` showing the same
    string (ST-BAR-01's test and the About test updated)
    Done 2026-10-23 (with 8.11): `shell::version_label`, its seven cases by unit test, the
    status bar, About and `--version` (a `LazyLock` for clap) on it, `help_cli`'s binary test
    and a `render_version` test. Files: `studio_core/src/shell/{mod.rs,status_bar.rs,about.rs,
    launch.rs,sidebar/tests.rs}`, `studio/tests/help_cli.rs`
  - [ ] 8.14 (code done 2026-10-29; the built window's check open) the shell's own icon font (the maintainer's request of 2026-10-23;
    `plans/core/README.md` "GUI framework choice: egui"): single-colour icons, one SVG each;
    each tool's drawn to say what it does, the general ones (help, settings, dismiss, warning,
    download) copied from Phosphor (MIT, its licence beside them). The tools' icons are
    picked and approved (2026-10-29; DECISIONS "A tool's kind shows as one mark on its icon"
    and "The final icons approved; the upgrader's folder turned, the Refs arranger's refresh
    sign"): `resources/icons/<tool id>.svg`, 13 filled outlines (the Stadium compiler keeps
    Phosphor's buildings), drawn on a picking page with candidates per tool at the rail's
    sizes, since retired (in git history before its removal). Left: a fontTools script in
    `scripts/` building the TTF into `resources/fonts/` from those SVGs and the five general
    Phosphor glyphs, replacing 8.11's `phosphor-subset.ttf`, and `studio_core`'s glyph
    constants → verify: the script rebuilds the committed TTF byte for byte; every `icon()`
    glyph exists in the font (a unit test over the registry); the rail seen in the built
    window, the rail and opened, dark and light
    Done (`75d76461`, `8d2ab7c0`, docs `ef5245ef`, merged `9d9fc444`; Opus sidekick):
    `scripts/make_icons.py` builds `resources/fonts/icons.ttf` and the drafts' copy (8,604
    bytes, the same on every run) from the 13 tool SVGs and six Phosphor glyphs copied point
    for point as SVGs (the Stadium compiler's buildings among them), on the subset's
    metrics and codepoints; `make_icons_test.py` checks both copies byte for byte and
    refuses two icons at one codepoint; `icons.rs`'s constants named after their tools;
    the subset removed, the drafts drawing the rail in `icons.ttf` (rendered beside the old
    rail: same places and sizes). SWE-2 round S1i: 1 counted (`.tmp/8_14/rulings_icons.md`).
    Open: the rail in the built window, dark and light
  - [ ] 8.15 (code done 2026-10-25 with 8.9f4c's rework r2; the built window's check open) the window's default to 1280 × 800 (the maintainer, 2026-10-25; `spec/core/gui.md`
    "The window's size", ST-WIN-01, ST-HELP-05; DECISIONS "Studio opens at 1280 × 800
    points"): `studio_core`'s `DEFAULT_SIZE` and its tests, the help window's; the Save
    editor's Tactics lists at 200 points, and its tests named for 1440 × 900 → verify: the
    scenarios' tests; the built window at 1280 × 800 on the FHD monitor, the Tactics tab in
    three columns. Rides with 8.9f4c's review fixes (one slice)
  - [x] 8.16 the drafts as one host and a script per tool (the maintainer, 2026-10-25;
    `core/gui.md` "Prototypes before implementation"; DECISIONS "The drafts are one host page
    and a script per tool"; side-lead track A): `docs/prototypes/index.html` from
    `shell.html`, the Save editor's (v2) and the Ruleset editor's drafts ported to
    `tools/*.js` at the 1280 × 800 default, the plans' pointers moved, then
    `scripts/bundle_drafts.py` → verify: every screen the old pages drew draws the same in the
    host (side-by-side captures), the bundle opens alone from another folder
    Done 2026-10-25 (side-lead A, SWE-2 in three slices): the host from `shell.html` with a
    tool registry (`registerTool`: draw, firstShow, controls, onKey, commit), each tool script
    one unindented wrapper, the 138 portraits as files; DECISIONS "The drafts' shared stylesheet
    draws body text at weight 400", "A tool's draft in the host uses the live shell". Verified
    by 73 side-by-side Playwright captures (`.tmp/8_16/captures.py`: shell 28, Ruleset editor
    18, Save editor 27), the old pages drawn at 1280 × 800 and weight 400; every pair the same
    but for the live shell (the sidebar's 160 points and the Ruleset editor listed, the
    Ruleset editor's view beside the sidebar, the Save editor's status as a notice) and the
    shared disabled-widget dimming; the bundle's captures pixel-identical to the host's. The
    old pages stay for the top lead to compare and remove. The Save editor's Tactics lists
    drawn at 200 points, names cut at the edge (DECISIONS "Studio opens at 1280 × 800
    points"). SWE-2 rounds: 5 of 17 accepted, then 3 of 9 (`.tmp/8_16/rulings.md`), each reworked. Files: `docs/prototypes/{index.html,
    tools/ruleset_editor.js,tools/save_editor.js,assets/save_editor/portraits/}`,
    `scripts/bundle_drafts{,_test}.py`, `justfile` (the test in `test`), pointers in
    `plans/{core/gui.md,core/development_plan.md,save_editor.md,ruleset_editor.md,
    export_upgrader.md,team_compiler/README.md}`, `GLOSSARY.md`
  - [ ] 8.17 the Export upgrader's draft, `tools/export_upgrader.js` (`plans/export_upgrader.md`;
    `core/gui.md` "Prototypes before implementation"; side-lead track A, after 8.16): every
    screen the plan describes, with sample data; the lead's usability review first, ideas to
    `PROPOSALS.md`; then the maintainer's remarks
    Drawn 2026-10-25 (side-lead A, SWE-2 in two slices and a rework): the two targets, the
    arguments grid, the run with its activity, the log panel, the left-behind list, the done
    line, the notice, Move to exports and compile, the stand-in pickers, the close prompt
    (a shell draft control asking each view's new `closeRequest` hook); the samples are real
    CLI runs (`.tmp/8_17/runs.py`: Lab's EGG and @ exports, the `co Spring` fixture, the
    `refs Tracer` fixture, model folders copied from EGG). The host's log panel takes its
    state, so a view has its own. Plan gaps decided (DECISIONS "The view's arguments and
    report, where the plan was silent"); the usability review's ideas are `PROPOSALS.md`
    P32-P36. SWE-2 rounds: 8 of 17 accepted, then 3 of 7 (`.tmp/8_17/rulings.md`), each
    reworked. Waits for the maintainer's remarks. Files: `docs/prototypes/{index.html,
    tools/export_upgrader.js}`, `plans/export_upgrader.md` "GUI", `PROPOSALS.md`
    - [ ] 8.17b the accepted items drawn (P18's Full/Midcup pair, P19's dialog, P32-P36; ruled
      2026-10-26, `plans/export_upgrader.md` "GUI"), the samples' rows grouped and the
      `needs <team>` hint from the real runs → verify: the bundled drafts build; each item
      reached in the draft
  - [x] 8.18 the host's layout edit toggle (the maintainer, 2026-10-25; `core/gui.md`
    "Prototypes before implementation", "Layout edit"; DECISIONS "The drafts' host gets a
    layout edit toggle"; after 8.16, track A's next step or the top lead's after the merge):
    the toggle, move and resize handles, "Copy layout" and "Reset" in `index.html`, the
    `data-panel` names in every tool script → verify: Playwright drives a move and a resize on
    two tools' panels, the copied string names both with their rectangles before and after,
    the neighbours reflow on a resize, a reload keeps the tweaks and Reset clears them
    Done 2026-10-25 (side-lead A, merged as `139355d9`): every tool's panels named
    (`data-panel`), moves and resizes kept per screen, Copy layout's rectangles before and
    after, Reset. SWE-2 rounds 6 of 10, then 3 of 10 (`.tmp/8_18/rulings.md` in
    `../4cc-studio-wt-drafts`)
  - [ ] 8.19 the host's remarks, links and longest values (`core/gui.md` "The host's other
    review aids"; DECISIONS "The drafts' host gets remarks, links, …"; after 8.18, with it on
    track A or the top lead's): the URL fragment helper and every tool script's state keys,
    the Remark toggle and "Copy remarks", the extraction script and `assets/longest.js`
    (real values only) → verify: Playwright opens a link to a dialog's state and the capture
    shows it; a remark's copied line names its panel and control and carries a link that
    reopens the state; with Longest values on, each tool's longest text is drawn (a capture per
    tool, looked at)
    - [x] 8.19a links (side-lead A, merged as `144fedf1`): the address names what is drawn
      (`#<tool>/<screen>?keys`, host keys and named dialogs, `run=done` replaying a finished
      run); 12 states pixel-equal from a fresh page. SWE-2 rounds 8 of 16, then 4 of 7
      (`.tmp/8_19/rulings.md`)
    - [x] 8.19b remarks (Opus, 2026-10-27, `.tmp/8_19/brief_8_19b_v2.md`, report
      `.tmp/8_19/8_19b_report.md`): the Layout group's "Remark" toggle (exclusive with Layout
      edit) catches a click in `#win` in the capture phase and opens a note pinned to the
      nearest `data-panel` and control (`button, select, input, textarea, label, .sel, td`,
      named by text, title, placeholder or label); `localStorage` `drafts.remarks`; "Copy
      remarks (N)" one line per note with its link; numbered markers after each render;
      Reset clears the remarks too. Checked in Chromium and Firefox (`check_b.py`: four
      notes' links pixel-equal to their states, reload, Reset, a row click caught; red run
      with a bubble-phase listener). Left as they are: a list row or an input with no name
      gets a panel note (its link still reopens the state); two same-named controls in one
      panel share a note
    - [ ] 8.19c longest values, briefed: `.tmp/8_19/brief_8_19c.md`
  - [ ] 8.20 the host's overlay and draft shots (`core/gui.md` "The host's other review aids";
    after 8.19, whose links it walks; the overlay moved into 0.0.1 as 0.0.1ov): the Overlay control and the capture naming, the
    Save editor's first captures from the built window; `scripts/draft_shots.py` → verify: the
    overlay of the built Tactics tab on its draft at 1280 × 800, looked at; a one-line host
    stylesheet edit lists every tool's changed screens, reverted by hand, then lists none.
    Briefed: `.tmp/8_20/brief_8_20.md` (the overlay needs built-window captures, which take
    the UI automation until `--gui` opens a save)
  - [ ] 8.26 the drafts' text drift side-lead A found (2026-10-25): the drafts show `v0.1.0`
    where the build shows `v0.0.1 pre-alpha`; the host's help chapters still say `4cc-studio`
    where the help files say `UnlimitedToolWorks`; with the preview pane on at 1280, the
    Ruleset editor's form clips its stat grid (the Ruleset editor waits for 0.1.0: fix with
    20.6) → verify: a capture of each, looked at
  - [x] 8.27 the notice in the bar's middle, in its kind's colour (the maintainer, 2026-10-25,
    after trying slice 3's Generate dialogs; `core/gui.md` "Status bar", the rule "A notice is
    seen, not found"; DECISIONS "A notice sits in the bar's middle, in its kind's bright
    colour"): the draft host first (`index.html`'s status bar: the notice centred, the activity
    right, the three colours), then `studio_core`: `NoticeKind`, `Notice.kind`,
    `theme::SUCCESS_FG`, the bar's layout, every publisher's kind (the Save editor's Generate
    notice Success, `Switched to PES …` Info or Warning with its Fix action) → verify:
    ST-BAR-02 to 04 red first; the built window at 1280 x 800 with a Generate notice, looked at
    Done 2026-10-25 (SWE-2, one run; brief `.tmp/8_27/brief_8_27.md`): the draft host first
    (`--success`, `publishNotice(text, action, kind)` at every call), `NoticeKind`, `Notice.kind`,
    `theme::{SUCCESS_FG, notice_color}`, the notice group centred by measuring its galleys (egui
    centres one rect, not a row), the activity right; ST-BAR-02/03 updated red first, ST-BAR-04
    observes the painted text's colour. Lead: the light theme's own green `SUCCESS_FG_LIGHT`
    (the bright one reads at 1.7 on egui's light panel; red first), ST-BAR-03 worded as the
    group (the sidekick's contradiction, accepted), the selector's warning state called orange,
    not yellow, in its docs, tests and plan. The maintainer checked the three colours on a
    rendered page (`.tmp/8_27/notice_colors.html`): fine; orange kept for warnings. Mutants
    (`mutants-diff 5b4f72b7`): 21 caught, 7 missed, all the group's width sums, which a short
    notice never strains; a long notice's test pins the group to the slot's edges (red on
    the 2 px button-padding mutant, "X to 534" past 528). Files:
    `studio_core/src/{status,theme,lib,tool}.rs`, `shell/{status_bar,mod,sidebar/{mod,tests},
    settings_menu,help_window,launch,title}.rs`, `studio/src/console.rs`,
    `save_editor/src/view/generate.rs`, `docs/prototypes/{index.html,tools/{save_editor,
    export_upgrader}.js}`
  - [x] 8.28 the aesthetics lock behind the "Aesthetics from exports" setting, off by default
    (the maintainer, 2026-10-25: DBC7 is on PES 17 with the old compiler, so the editor keeps
    4ccEditor's aesthetics editing with no switch shown; `plans/save_editor.md` "Read-only
    aesthetics"; DECISIONS "The aesthetics lock is a setting, …"): the draft first (the
    Appearance tab's ownership line and switch only with the setting on; the settings menu's
    checkbox), then `SaveEditorSettings.aesthetics_from_exports`, its checkbox and tooltip in
    the settings menu's Save editor section, the window reading it each frame as `live_aatf`;
    every lock site (`edit_aesthetics`) gated on the setting → verify: SE-GUI-35 red first,
    SE-GUI-07, 11, 22, 23 with their new GIVENs, the default table's new key
    Done 2026-10-25 (SWE-2, one run; brief `.tmp/8_28/brief_8_28.md`): the draft's
    `aesFromExports` control and `aesUnlocked()` at every lock site; the setting, its settings
    row with the plan's tooltip; `SaveView::aesthetics_locked()` the one guard (Appearance
    groups, the Team, Player and Savefile menus, Ctrl+F, Ctrl+I); the ownership line and switch
    drawn only under the setting; help 04-07 reworded. SE-GUI-35 drives the settings row into
    the next `Tool::view` frame (red: "no ownership line under the default"; the setting
    ignored by hand: "the tab's controls are enabled"); the locked and unlocked tests set the
    setting on. The import dialog SE-GUI-22 and 35 name is not built in the window yet: the
    step that builds it tests its aesthetics section under both scenarios. `mutants-diff
    9c300b90`: 19 caught, 2 unviable, none missed. Files:
    `save_editor/src/{lib,settings}.rs`, `view/{mod,menus,player_list}.rs`,
    `view/appearance/mod.rs`, `view/tests/{appearance,player,team,savefile}.rs`,
    `help/0{4,5,6,7}_*.md`, `docs/prototypes/tools/save_editor.js`
  - [x] 8.30 WITHDRAWN (the maintainer, 2026-10-28: "keep the Nation input as it is, with no
    label"); its draft stays on `side/drafts2`, unmerged. Was (the maintainer, 2026-10-26): the Save editor's
    Nation field shows the nation's name beside its number, from a map of the game's nation
    ids to names (which versions share the ids, and where the names come from, first).
    Drafted (`side/drafts2`); answered 2026-10-27 (DECISIONS 2026-10-27 "Formation pictures, the formation's name and the nation names: the maintainer's answers"): 4cc's nations in PES 16's
    table, PES 20 read as 21; the draft revised (side-lead D's step 5), then the look
  - [x] 8.31 in 0.0.1 (the maintainer, 2026-10-29: "it's a small thing"; first placed after
    it, 2026-10-27): a click on the toolbar's `• unsaved
    changes` opens the list of the changes, the table the close prompt already shows
    (`plans/save_editor.md`, the "Closing, or opening another save" bullet: Item, Old value,
    New value) without its save, discard and cancel. Drafted first, then the plan. A popup,
    not a window (the maintainer, 2026-10-27; DECISIONS "The changes list is a popup under the
    unsaved-changes label"); a colour's values draw a 12 × 12 swatch in the shared table (the
    close prompt's too) and the list has no footer line (DECISIONS "The changes table's colours
    draw a swatch; the list has no footer line"; draft `side/drafts2` `30842fa2`). Approved
    2026-10-28: the draft on main (`ed81bdb3`, `aaae3d1e`), the plan written
    (`plans/save_editor.md` "Editing UI", the bullet after the close prompt's); left: the
    code (brief `.tmp/8_31/brief_8_31.md`), its scenario SE-GUI-43 written 2026-10-29
    Done (`09fa86e2`, `e0c7a32a`, merged `3bbe1c61`): `view/changes_popup.rs`, the label a
    click target opening an `egui::Popup` under it with `prompt::changes_table`, its rows
    computed once per opening, closed by a click outside, Esc, the label again or any
    revision move; `terms::Term` carries a team colour's two swatches, drawn before the hex
    in the list and the close prompt. The box is 460 to 640 points with its margins and
    slides to 8 points inside the window's edge, as the draft's. SWE-2 round S1u: 2
    counted (`.tmp/8_31/rulings_8_31.md`). Not yet seen in a built window. Files:
    `save_editor/src/view/{changes_popup,mod,prompt,terms,patch}.rs`, its tests
  - [x] 8.32 approved 2026-10-28 as built in 0.0.1x (the maintainer, 2026-10-27): the pitch draws the formation's name
    (`4-3-3`) in its corner, exactly as PES's formation screen draws it. First: the game's
    rule for the name from the slots' positions and the label's look, from in-game captures
    (ask before taking them); then the draft, then the plan. Moved into 0.0.1 as 0.0.1x
    (the maintainer, 2026-10-27)
  - [ ] 8.33 after 0.0.1 (the maintainer, 2026-10-27): a menu entry that saves a picture of
    the team's formations to a folder: a grid, one row per preset, one column per formation,
    a preset with fluid formation off showing its first formation only. Drafted first (which
    menu, the folder asked or a setting, the image's format and cell size), then the plan.
    Drafted (`side/drafts2`); answered 2026-10-27 (DECISIONS 2026-10-27 "Formation pictures, the formation's name and the nation names: the maintainer's answers"): pitch-size cells, a second
    set of entries with the presets' settings, a save dialog for one team, no fluid-off
    line, a narrow picture when no preset is fluid; the draft revised (side-lead D's step
    5), then the look. Approved 2026-10-28 with four changes (DECISIONS "The drafts' looks
    of 2026-10-28"): one Team-menu entry whose own dialog holds the settings checkbox,
    the dialogs' fields at one x, the range's teams picked in combos, the range dialog one
    width in both checkbox states. Drafts on main (`f3122871`, `e1b49843`, `5d668583`),
    the plan written (`plans/save_editor.md` "Tactics editor", "Formation pictures");
    revised 2026-10-29 (DECISIONS "Formation pictures draw hidden formations, label Fluid
    formation, and put a target on its own line"; `↳` added to the symbols font); left: the
    code, with 8.34, its scenarios written with it
  - [ ] 8.34 after 8.33 (the maintainer, 2026-10-27): the same for a range of teams, one
    picture per team, in one go. Drafted and approved with 8.33 (above); left: the code
  - [ ] 8.35 before 0.0.1 (P45, accepted 2026-10-30; DECISIONS "A ruleset carries its
    event's logo and page"; drafted, the drafts' `eventLogo` and `eventLogoField`): a
    ruleset's event logo and page
    - [x] a `aatf`: the `page` and `logo` keys, their load checks, the writer's order (logo
      last) → verify: RE-FILE-09; a load test per refusal (not base64, no PNG signature,
      over 64 KB, a page without http(s)); the logic's hash unchanged by either key. Done
      2026-10-30 with c (Opus sidekick, merged `b943c67c`): `Ruleset::{page, logo}`,
      `LoadRule::{PageNotWebAddress, LogoNotBase64, LogoNotPng, LogoTooLarge}`, `base64` in
      `aatf`; RE-FILE-03's fixture drops a comma elsewhere on its line 12 (the page line
      moved the old one). Open: the SWE-2 round (an Opus slice) and the (b) Astra review
      (a new `pub` interface over two crates), with b
    - [x] b the Save editor's top-right logo and the Ruleset editor's General fields, the
      picture decoded and sized once in `team_widgets::event_logo` (DECISIONS "The event
      logo is decoded and sized in `team_widgets`"; brief `.tmp/8_29d/brief_p45b.md`) →
      verify: SE-GUI-44, RE-GUI-19 headless; the built window (autonomous: no ask)
      Done 2026-10-30 (Opus sidekick, merged `7b875078`): `event_logo` in `team_widgets`
      and the Save editor (drawn after the panes, over their frames), `LogoField` in
      General, `Ruleset::{logo_png, set_logo}` with `LogoRefusal`; the "moves nothing"
      test compares DBC7 with an imported copy without its logo (every built-in ruleset
      carries one); a narrow pane under about 831 points hides the logo. Open: the SWE-2
      round and the (b) Astra review over a + b + c, the built window
    - [x] c every ruleset file under `aatf/rulesets/` gets its logo and page, the seven
      built in and the two test rulesets (`autumn25`, `vgl25`; the drafts import Autumn 25
      as their sample file): the pages are the events' wiki pages the maintainer listed
      (2026-10-30), Autumn 25's and /vg/ League 25's by the same address pattern (the
      maintainer's answer, 2026-10-30), the logos the ones those pages show, every /vg/
      League's the shared VGL logo, scaled to 84 pixels high
      (`docs/prototypes/assets/logos/<stem>.png`, every /vg/ League's `vgl.png`, from
      `.tmp/8_29d/logos_scale.py`) →
      verify: each ruleset file loads with its logo; the built window per built-in ruleset.
      Done with a: a test loads all nine, each page the table's and each logo the asset's
      bytes; the built window waits for b
  - [ ] 8.36 every engine's looks through the savefile patch, no stripped saves (the
    maintainer, 2026-10-30; DECISIONS "Every engine carries a player's looks in the savefile
    patch; no stripped saves")
    - [x] a the plans: every passage on stripped saves, `strip-and-seed`, `check-stripped`,
      the seed CPK and the compiler's Fox player tables reworded or removed, and the full-look
      patch designed (`plans/pes_savefile/operations.md` "Aesthetics patch (new)"), from the
      inventory `.tmp/8_36/inventory.md` (its sections 1, 3, 6 and 7). The spec keeps
      describing the code as it is: its passages move with b and c → verify: `just
      doc-pointers`, `just acceptance`; a grep of the plans for `strip` and the three table
      names left only where they still hold.
      Done: the patch's plan holds the look, the IDs filled after their tasks, the own id and
      the cleared base copy (with the census); the Save editor plan's "Stripped save" lists
      what goes; 8.23 lost its re-strip; four issues reworded or resolved; DECISIONS "The
      full-look patch writes the player's own appearance id…". The grep leaves new text and
      other meanings only (kit strips, the DB generator's tables, `aatf_rules.md`'s
      placeholders). Files: AGENTS.md, plans `save_editor.md`, `pes_savefile/{operations,model}.md`,
      `aesthetics_export/{settings_toml,fpc_toggle,player_folders}.md`, `team_compiler/{README,gui,pipeline}.md`,
      `core/{README,development_plan}.md`, `player_aesthetics_editor.md`
    - [x] b before 0.0.1, the Save editor: `strip-and-seed`, `check-stripped`, the stripped
      players' notes and the Strip again window, Generate's stripped skip, the Comparator's
      and `diff`'s stripped marks, Team TOML's `stripped`, their help and catalog rows, and
      what `pes_savefile` keeps only for them; `diff` gains the appearance id as a row, the
      ownership line reads the same on every engine (DECISIONS "The full-look patch writes the
      player's own appearance id…"); the spec with it (the inventory's S1-S21, S56-S57's strip
      parts, S60, S62, S67, and GLOSSARY's D3 and D4) and the plan's SE-GUI-20, withdrawn with
      its tests (brief `.tmp/8_36/brief_b.md`) → verify: the gates; the help and catalog name
      none of them.
      Done 2026-10-30 (Opus sidekick, branch `s36b`): about 2,100 lines removed (the two
      commands, the window, the marks, `EditFile::appearance_row`, `strip.rs`, EU-SET-06's
      finding); `ownership_tip` became the constant `OWNERSHIP_TIP`; `diff` prints the
      appearance id raw (`new=4294967295`), in the Aesthetics scope; `Side` keeps its first and
      second for `player_only_in`. New tests: the id's row (lib and CLI, SE-DIF-05), `stripped`
      refused, the Fox ownership line's link opening the patch panel, a player at -1
      generated. Lead fixes: SE-DIF-05's citation, the appearance id in `--aesthetics`' lists.
      The drafts went first (`929f3497`). Open: SWE-2 rounds and Astra (an Opus slice). Files:
      `save_editor` (src, tests, help), `export_upgrader` (`messages.rs`, `settings.rs`, its CLI
      test), `pes_savefile` (`file.rs`, `ops/compare.rs`, `interchange/{team_toml,texport}`),
      and the spec and plans named above
    - [ ] c after 0.0.1, the full-look patch and the Team compiler: the aesthetics patch on
      every engine (it is Fox-only today) gains the appearance block, the boots and gloves IDs
      over the whole allocation, filled after their tasks commit, and clears a compiled
      player's base copy; `apply-patch` writes them; the Fox player-table writer and its
      messages go; the spec with it (the inventory's S22-S54, S56-S57's table parts, S61,
      S64-S66, S68, and GLOSSARY's D1 and D2), its Acceptance written first → verify: the
      gates; TC-SAV-12 (a PES 17 compile writes no patch) rewritten; the library parity's
      difference classes re-diagnosed
    - [ ] d in game (PES 2021, then one more Fox version and PES 2017): a patch applied to an
      unstripped save shows the compiled look in Edit mode, boots and gloves included, on a
      player whose base copy named another and on one whose edit flags were clear
      (`scripts/ingame/`, saves backed up); with it, the FoxDen gloves issue rechecked
  - [ ] 8.37 a dialog's title centred over the dialog, as the windows' titles are (the
    maintainer, 2026-10-30, after left-aligned window titles proved out of egui's reach;
    DECISIONS "A dialog's title is centred, as the windows' titles are"). Drafted:
    `index.html` `.window .title` (`text-align: center`), rendered on Set all stats and the
    wide Open save notice → `studio_core::widgets::modal` draws the title centred over the
    modal's final width (the body decides it, so the title's place is known after the body)
    → verify: a headless test measuring a narrow and a wide modal's title centre against the
    frame's (red first); the overlap and draft-compare checks; the built window (autonomous)
    Built with b (Opus sidekick, merged `40e36efa`): the title row reserved before the body,
    the title drawn in it centred once the body set the width; `theme::window_title` gone.
    Gated green (`5ba78b61`); SWE-2 S1s37 0 accepted, Astra A1s37 no concerns
    (`.tmp/8_36/rulings_837.md`). Open: the built window
    - [ ] 8.37b Set all stats and Bump all stats: the number on the sentence's row, at the
      dialog's right edge (the maintainer, 2026-10-30; DECISIONS "Set all stats' and Bump
      all stats' number on the sentence's row"; `plans/save_editor.md` "Editing UI", the
      sentence after the menus' shortcuts). Drafted: `dlgNum`'s `.num-row`, rendered on both
      → `menus.rs` `stat_num_dialog` → verify: a headless test measuring the field's right
      edge at Cancel's and its vertical centre on the sentence's (red first); overlap tests
      Built (merged `40e36efa`): one fixed-width right-to-left row, the field at the right,
      at least 12 from the sentence; the test covers both dialogs
  - [ ] 8.29 the text pass: every line a member reads reread against CONTRIBUTING's "That text
    flows naturally" (the maintainer, 2026-10-25; DECISIONS "Text a member reads flows
    naturally, and every existing line is swept once"), rewritten where it reads dry, its meaning
    kept; the tests that pin a string follow it. One slice per surface, the 0.0.1 tools first;
    each slice's diff read line by line by the lead before it lands (a rewrite that changes what
    a line says is a finding) → verify: the gates; every rewritten line read in its place (the
    draft or a headless frame)
    - [ ] a the Save editor, in three slices:
      - [x] a1 the window's strings, tooltips and notices under `view/`: 34 lines rewritten
        (the lead retouched six: the FPC presets' tooltips, which said Ctrl+I applies the
        hide preset where it toggles, two tooltips back to the imperative, the stripped
        player's Generate line without "record"), the tests pinning them following; the plan's
        quotes of them (SE-GUI-11, 15, 30, 31 among them) brought along. `mutants-diff
        56da050d`: 21 caught, 3 unviable, none missed. Files: 12 `view/`
        modules, 13 `view/tests/` files, `docs/plans/save_editor.md`
      - [x] a2 the message catalog (`messages.rs`) and the CLI's output and `--help` text
        (`cli.rs`): 76 lines rewritten (the lead retouched ten: "written, but the rest is
        written" where nothing contrasts, `check-aatf`'s failure row naming the ruleset's
        check again, two fragments). The spec's Messages table keeps its own wording: it is
        the developers' table, carrying each code's context fields, not text a member reads.
        `mutants-diff 484625bd`: 19 caught, 6 unviable, none missed.
        Files: `save_editor/src/{messages,cli}.rs`, `view/tests/comparator.rs`
      - [x] a3 the help chapters and the draft `docs/prototypes/tools/save_editor.js`, which
        takes a1's and a2's rewrites word for word (the version banner rewritten 2026-10-25 as
        the rule's example): 93 lines (43 in the eight chapters, 50 in the draft; the lead
        retouched one), the draft's interference rows now the window's finding texts; the
        draft's review-aid prose, `team_widgets`' own texts and the generated templates left
        as they are. The help's running-game sentences follow 8.9l2 and l5. Table
        `.tmp/8_29/a3_table.md`. Files: `save_editor/help/0{1..8}_*.md`,
        `docs/prototypes/tools/save_editor.js`, `docs/plans/save_editor.md` (SE-GUI-11)
    - [x] b the shell (`studio_core`: sidebar, status bar, settings menu, help window, the
      launch and title texts) and the drafts host `docs/prototypes/index.html` (Opus, in the
      worktree `wt-829b`, brief `.tmp/8_29/brief_8_29b.md`): 35 lines rewritten (table
      `.tmp/8_29/b_table.md`; the lead retouched one, `--gui`'s "not available yet, because it
      arrives in a later version" now "`--gui` comes in a later version"), the CLI's error
      details in parentheses as a2's, the two core help chapters' colon stacks as sentences;
      the drafts take the rewrites word for word. The sidekick's contradictions, both
      accepted: a settings row's tooltip is the key then the row's own tip, written for the
      member, not the field's doc comment (`settings_row.rs`, the test's message and the
      draft's comments say so now); "the GUI phase" became "a later version". Files: 14
      `studio_core`/`studio` sources and tests, `help/core/{data_location,install}.md`,
      `docs/prototypes/index.html`
    - [ ] c the Team compiler and the Export upgrader: their message catalogs, help chapters,
      CLI output and the comments injected into `settings.toml`/`config.toml`/`materials.toml`
      (side-lead U, branch `side/text`), in six slices, the last outside its locks:
      - [x] c1 the Team compiler's catalog, its first 80 rows (`export_extract_failed` to
        `xml_ignored_fox`): 89 texts rewritten (the lead retouched ten: four the sidekick had
        left, six where the rewrite kept a hard-to-parse clause or lost a "so"), the version
        or case put in front with a colon now a clause ("In PES 2018 to 2021, ..."), the
        semicolons now *but*, *so* or a new sentence. `export_extract_failed` stays as it was
        (`studio`'s `help_cli` pins it, and it reads naturally); the spec's Catalog table keeps
        its own wording (a2's ruling). SWE-2 round S1: 24 concerns, 16 accepted (among them
        `kit_config_option_ignored` now saying the FPC values make the shirt model 176, and
        three texts whose clause could bind to the wrong noun), 7 nits, 1 rejected; S2: 7, 6
        accepted (five more clauses or pronouns with a second, wrong reading); S3: 8, 3
        accepted, 4 nits, the loop's end. Table
        `.tmp/8_29/c1_table.md`, rulings `.tmp/8_29/rulings.md` (side/text worktree).
        Files: `team_compiler/src/messages/catalog.rs`
      - [x] c2 the catalog's rows 81-161 (`xml_ignored_shared` to `source_changed_during_run`):
        96 texts rewritten (the lead retouched ten: a lost main verb, a dangling "Found on
        ...", two causes the sidekick's "so" invented or dropped, "promoted" and "rows" for a
        bin's entries), the `upgrade-dpfl` and Cup DLC rows now sentences with the command or
        mode as subject, `mtl_broken`'s position worded as c1's `xml_broken`. SWE-2 round S1:
        14 concerns, 11 accepted, 2 nits, 1 rejected; S2: 10, 9 accepted (among them a
        `.common` link that always conflicts with a file, and a missing listed CPK that makes the
        game reject all of `download/`, which the texts hid); S3: 7, 7 accepted (an Error
        naming only the first missing texture, said wrongly before); S4: 9, 5 accepted; S5: 7, 2,
        the loop's end (115 texts in all, and c1's `mtl_texture_not_found` "listed before this
        run's own"). Table `.tmp/8_29/c2_table.md`. Files: `team_compiler/src/messages/catalog.rs`
      - [x] c3 the catalog's format-check and structure-pass rows (162-237): 58 texts
        rewritten (the lead retouched one, whose "and sits below a subfolder" reached the
        disallowed-extension case too), the upgrader's 13 structure-pass copies following word
        for word (`.tmp/8_29/shared_whens.py` compares all 46 shared `when`s, side/text
        worktree); `notes_encoding_invalid` keeps its effect (`studio`'s `help_cli` pins it).
        SWE-2 round S1: 8 concerns, 4 accepted (the retouch's own scope again), 3 nits, the
        loop's end.
        Table `.tmp/8_29/c3_table.md`. Files: `team_compiler/src/messages{,/catalog}.rs`,
        `export_upgrader/src/messages.rs`
      - [x] c4 the Team compiler's CLI output, `--help`, window strings and help chapter, in three
        slices (the chapter alone is 862 lines):
        - [x] c4a the CLI's refusals and `--help` (`cli.rs`), the window's strings (`view/mod.rs`,
          `gui_run.rs`): 20 texts rewritten, the tests pinning them following; the lead's three
          follow-ups: the settings note, which said the settings menu "comes later" (it
          shipped at 8.6c1), `reader/mod.rs`'s copy of the `--export` refusal, and the spec's
          quote of the missing exports folder's refusal. Table `.tmp/8_29/c4a_table.md`.
          Files: `team_compiler/src/{cli,gui_run,reader/mod,view/mod}.rs`, two `tests/cli/`
          files, `docs/spec/team_compiler/messages.md`. SWE-2 round S1: 10 concerns, 3 accepted
          (among them the download-folder refusal, which sent a member with the right PES
          folder to change it), the loop's end; `libs/fpc`'s hide-ID refusals keep a
          "key = value: must be" shape, for whoever holds that crate
        - [x] c4b `help/01_command_line.md`, its first half (lines 1-440, to the referees CPK's
          paragraph): 55 passages rewritten, mostly a semicolon or a stacked colon made a new
          sentence and a pronoun named (the lead retouched eight and reverted one: a "that name"
          left without its noun, a face diff that "counts as the player's own" where it counts
          as a `face_diff.bin`, "an FPC team's own collar"). Two facts brought to the spec
          (`spec/model_format.md` "Pre-Fox: the `.mtl` a `.model` uses"): a model's `.mtl` is
          searched in the player's or shared folder holding it, and a `.common` link's search
          ends in the player's folder. SWE-2 round S1: 14 concerns, 12 accepted (one the rewrite's
          own: a `.mtl`'s name match said backwards; the others facts the chapter had narrower
          than the catalog or the code: PES 2015 to 2017's texture codecs, `skl_no_slot`'s and
          `face_file_not_used`'s scope, `shared_folder_no_model`'s condition, link files in
          subfolders, which work), 2 nits; S2: 11, 9 accepted (`texture_unreadable`, which the
          chapter called `folder_pack_failed`, a shared folder one player links taking his own
          ID, `.jpeg` and `.tiff`, `.common` links never the same as a real file), 2 rejected; S3: 15, 10 accepted (the
          chapter's opening still said `compile` builds PES 2018 to 2021 only, a `logo_small`
          alone builds no logo, a plain `Faces` link reports nothing), 2 nits, 3 rejected; S4: 7,
          5 accepted (among them an unlisted `.model` said to be kept), 2 nits; S5: 10, 7 accepted
          (pre-Fox and Fox rules told as one: the `.common` model link, the bundled `.fclo`, a
          `face_diff.xml`'s packing; the referee kits' bins CPK under `multicpk_mode`); S6: 18,
          8 accepted, 4 nits, 2 rejected, 4 routed to c4e; S7 (scoped to the slice's flow and its
          own fixes): 7, 4 accepted, the loop's end.
          Tables `.tmp/8_29/c4b_table.md` and `c4b_s{1,...,7}_table.md`, checked against the diff by `.tmp/8_29/prose_table_check.py`
          (side/text worktree). Files:
          `team_compiler/help/01_command_line.md`
        - [x] c4c `help/01_command_line.md`, its second half (lines 467 to the end, from the
          CPK's install to "The aesthetics patch and your save"): 62 passages rewritten, mostly
          a colon or semicolon made a new sentence, a garden path ("the kits the game lists for
          the team that the export does not hold") reordered, and "record" now the save data
          or the appearance a member sees (the lead retouched six: two pronouns named, "a CPK
          listed before the CPK being compiled" as the paragraph above says it, `auto` named
          before what it does). The catalog's wording followed: `old_cpk_locked` "most likely
          because PES is running", the `livecpk` files, the default face a bad
          `ingame_face.toml` leaves. SWE-2 round S1: 3 concerns, 2 accepted (a missing save
          "only" a warning again, a kept clause saying the number textures are never touched
          beside the sentence that re-arranges them), 1 nit, the loop's end. Tables
          `.tmp/8_29/c4c_table.md` and `c4c_s1_table.md` (side/text worktree).
          Files: `team_compiler/help/01_command_line.md`
        - [x] c4d the crate's other texts a member reads: the `anyhow` contexts and `bail!`
          texts printed as `error: ...` on the terminal and in the window, or carried in a
          finding's `error=` field (`folder_pack_failed`, `installed_bin_unreadable`): 54 texts
          rewritten, the path no longer put first and pointed back to with "it" (`cannot
          rename {from} to {}, because a file of that name already exists`), "unwrap" and
          "parse" now "decompress" and "read", the tests pinning them following (the lead
          retouched six: a `face.xml` or `.mtl` path named twice, "placed in two of the teams
          CPKs" where the check is "twice", the staging and "new tree" texts; and the
          teamnotes test's path check given its verb back). SWE-2 round S1: 8 concerns, none
          accepted (six asked for contexts the code never had or for other values than the
          texts format, two for invariants only a bug reaches), the loop's end. Table `.tmp/8_29/c4d_table.md`
          (side/text worktree). Files: 19 `team_compiler/src/` modules, six `tests/cli/`
          files. The Save editor's `view/portraits.rs` keeps its own "not a CPK the reader
          accepts", for whoever holds that crate
        - [x] c4e a fact audit of `help/01_command_line.md` (and the upgrader's) against the
          catalog and the spec: c4b's review rounds S1 to S6 found over fifty facts the chapter's first half stated
          narrower, staler or for one engine only, and their last ones need the code read
          (side-lead U's rulings, `.tmp/8_29/rulings.md`, round S6c4b, routed: pre-Fox
          `Common` subfolders, a shared folder's subfolder rules, the PES 2021 skeleton default's
          reason, the pre-Fox merge twin of `fmdl_merged`, how a `.skl` is packed); the second
          half (c4c) likely holds as many. c4c's sidekick found two: an installed
          `DpFileList.bin` that cannot be read stops the compile (`installed_bin_unreadable`),
          which the bins paragraph does not say, and `savefile_missing` excludes a configured
          path naming a folder (`savefile_invalid`), which the patch section does not say.
          The Export upgrader's chapter gets the same audit (c5b's review S1, rulings "Round
          S1c5b", routed: a finding on the output names the output's path, or none; the
          referee fold's other clash cases; an unnamed `common/` model left out only when the
          folder folds; exit code 3's other aborts). In three slices:
          - [x] c4e1 the compiler's chapter, lines 1-466: 15 facts made true, the five S6c4b
            items among them (a pre-Fox `Common` subfolder's images built as `Common`'s; a
            shared folder's tree rules; the PES 2021 skeleton default's reason; the pre-Fox
            merge, `model_merged`, boots only; a `.skl` packed under its slot on Fox and nowhere
            on pre-Fox), and the summary's skipped exports, `Portraits/` names, a texture's
            container and mip levels, `_fit`, a pre-Fox link combining only beside
            `ingame_face.toml`, the `.common` link cases (the lead retouched eight). The Common
            `.mtl` repoint brought to `spec/team_compiler/pipeline.md` as the code does it.
            SWE-2 rounds: S1 7 concerns, 7 accepted (a `face_diff.xml` standing for the bin, the
            repoint after a nearer texture, `Portraits/` subfolders, gloves named by suffix);
            S2 12, 11 (the Fox combine's parts, a `Faces` link giving a face, a link's `.mtl`
            beside its model in `Common`, per-directory `texture_stem_conflict`); S3 10, 8
            (`xml_dif_conflict` on both engines, `logo_upscaled` on either side, the bundled
            diff); S4 10, 8 (`xml_ignored_shared` pre-Fox only, a `Full` kit's built-in mask,
            `_boots` after an underscore); S5 8, 3, the loop's end. Nine catalog Whens followed
            the code (`fmdl_name_invalid`, `model_name_invalid`,
            `ingame_face_explicit_face_model`, `shared_folder_no_model`, `face_file_not_used`,
            `xml_dif_conflict`, `fmdl_fcl_hair_fallback`, `fmdl_merged`, the upgrader's copies
            kept word for word) and four spec rows. Tables `.tmp/8_29/c4e1_table.md`,
            `c4e1_s{1,...,5}_table.md`, `c4e1_evidence.md` (side/text worktree). Files:
            `team_compiler/help/01_command_line.md`, `team_compiler/src/messages/catalog.rs`,
            `export_upgrader/src/messages.rs`, `spec/team_compiler/{pipeline,messages}.md`.
            `spec/aesthetics_export/player_folders.md` ~720 still says a shared folder with no
            model of its kind takes no ID without the `.common` link case, for whoever holds it
          - [x] c4e2 the compiler's chapter from "`compile` installs the CPK into the game." to
            the end: 33 facts made true, c4c's two items among them (an unreadable installed
            `DpFileList.bin` is `installed_bin_unreadable`; a `savefile_path` naming a folder is
            `savefile_invalid`), and a pre-Fox kit config no compile can remove, a `Midcup`
            kit's kept menu colors and config, a `Full` kit's derived colors, the deep pass
            reading only what the compile uses, `model_hidden_dropped`, the twelve template
            names, the extraction refusals, the multicpk bins and `placeholder.cpk`, a save of
            another version (the lead reverted one: PES 2015's shirt patterns 14 and 15, which
            the old text had right, and retouched one). The catalog rows of
            `kit_config_collar_unpatched`, `player_table_missing` and
            `deploy_target_unwritable`, and two spec passages (`template_override_unreadable`,
            a `Midcup` kit's template over an unreadable config) brought to the code. SWE-2
            rounds: S1 12 concerns, 9 accepted (a bare `./name.dds` resolving nearest first, a
            `Full` export dropping the colors of kits it does not hold, the Fox-only boots and
            gloves clearing, a save that cannot take the patch; `kit_config_generated`'s row);
            S2 5, 4 (a combined shared folder's textures, the Fox-only `fcl_hair_sim.fclo`
            template, the bins a listed CPK is read for), the loop's end. Tables
            `.tmp/8_29/c4e2_table.md`, `c4e2_s{1,2}_table.md`, `c4e2_evidence.md` (side/text
            worktree). Files:
            `team_compiler/help/01_command_line.md`, `team_compiler/src/messages/catalog.rs`,
            `spec/team_compiler/{messages,pipeline}.md`
          - [x] c4e3 the upgrader's chapter: 12 facts made true, S1c5b's four routed items
            among them (a report line names a path only for a file or folder, the output's
            for a check of the output; the referee fold's clash cases; an unnamed `common/`
            model left out only when the folder folds; exit code 3's other aborts), and `refs`,
            the coverage words and tags matched in any letter case, a referee export refusing
            `--coverage`, a roster that is not UTF-8, `Refs list.txt` only without `refs.txt`,
            `no_old_export` before `refs_txt_missing`, a Team Note entry left unmigrated (the
            lead retouched one). The spec's report line (`spec/export_upgrader.md` "CLI" and
            "Acceptance") brought to the code. SWE-2 rounds: S1 9 concerns, 5 accepted (the
            report's last line for a run that stops at once, the usage line a referee export
            follows, a second-word tag before the `midcup` guess; the spec's usage line); S2
            4, 4 (an input of the wrong kind, an unparsable teams list), the loop's end.
            Tables `.tmp/8_29/c4e3_table.md`, `c4e3_s{1,2}_table.md`, `c4e3_evidence.md`
            (side/text worktree). Files:
            `export_upgrader/help/01_command_line.md`, `spec/export_upgrader.md`
      - [x] c5 the Export upgrader, in two slices:
        - [x] c5a its catalog (`messages.rs`): its own rows' two texts and the structure-pass
          rows' effects, whose `when`s stay the Team compiler's word for word: 45 texts
          rewritten, "entry" and "member" now the file or folder a member sees, "item folder"
          the boots or gloves folder, the bare `Removed ...`/`Kept ...` effects sentences (the
          lead retouched four: a "them" with no plural before it, a semicolon left in
          `custom_model_missing` and its save "record", and `common_reference_kept`'s two
          conditions joined by "and" where the spec and the code take either, which the
          sidekick found). SWE-2 round S1: 10 concerns, 5 accepted (an unmigrated Team Note
          entry, which "file or folder" left out; content only meant for the game; an Error
          that keeps its content also ends the run with exit code 1; `settings_value_refused`
          covering `ingame_face.toml`), 1 nit, 4 rejected; S2: 11, 7 accepted (a `refs list.txt`
          roster, which two rows left out; a `Midcup` kit without config updating only its
          colors; three folders or files the text could not tell apart), 1 nit, 3 rejected; S3:
          5, 4 accepted (`upgrade-models` input, which S1's "old export" left out), the loop's
          end. Tables `.tmp/8_29/c5a_table.md` and `c5a_s{1,2,3}_table.md` (side/text worktree).
          Files:
          `export_upgrader/src/messages.rs`. The drafts host's mock catalog
          (`docs/prototypes/index.html` 1418-1446) copies the old texts, for the top lead
        - [x] c5b its `cli.rs` (`--help`), its refusals and error texts and
          `help/01_command_line.md`: 31 strings and 12 help passages rewritten, the input
          refusals now worded as the Team compiler's `--export` ones (`{} does not exist`,
          `{} is not a folder, a .zip or a .7z file`), the save's "records" the save itself,
          the exit codes one sentence each (the lead retouched six: `--coverage midcup`'s
          line reverted, its rewrite naming what is installed where the old one named what
          the export holds; "the resolved team"; the `.upgrading` folder named the incomplete
          output twice; the help's referee roster now `refs.txt` or `Refs list.txt`, as
          c5a's catalog rows, which the sidekick found). SWE-2 round S1: 13 concerns, 1
          accepted, 5 nits, 4 facts of kept sentences routed to c4e, 3 rejected, the loop's
          end. Tables `.tmp/8_29/c5b_table.md`, `c5b_help_table.md` and their `c5b_s1_*`
          twins (side/text worktree). Files: `export_upgrader/src/{cli,upgrade,
          staging,model_folders,naming}.rs`, `help/01_command_line.md`, two `tests/cli/`
          files. The drafts `docs/prototypes/tools/export_upgrader.js` (417-424, 578, 682) and
          `docs/prototypes/index.html` (1055-1064) quote the old texts, for the top lead
      - [ ] c6 the injected comments: they live in `pes_savefile` (`settings_toml/`, the
        `settings.toml` and `ingame_face.toml` headers and key comments) and `kit_config`
        (`toml_form.rs`, `config.toml`), outside side-lead U's locks: for whoever holds those
        crates (no code writes comments into `materials.toml` today)
    - [ ] d `libs/aatf`'s violation texts and the other tools' catalogs and help
      - [ ] d1 before 0.0.1 (the maintainer, 2026-10-29, from the built app's Live AATF:
        "a sweep for user-unfriendly prints"): the AATF lists show rules-file spellings and
        kind names (`Unused` / "only 1 of 3 a_positions used", `Stat` / "attacking_prowess 82
        over the maximum 77", "ball_control 82, above the 77 the rule sets"). Every GUI text
        that can show an internal identifier (a rules-file id, a kind name, a `{:?}` print, a
        code or key) or reads badly once filled is listed first (inventory
        `.tmp/8_29d/inventory.md`), then drafted, planned and built. The CLI's `check-aatf`
        lines too (the maintainer: "the spellings are only for the program, both managers and
        rule makers prefer to see regular text like Attacking Prowess"), so
        `spec/save_editor.md` "CLI"'s rules-file spelling of its lines is replaced in the
        plan → verify: no list row, tooltip, notice or CLI line prints a snake_case id or a
        kind name, pinned by a test over every `ViolationKind`; the rows read in the draft and
        the built window
        Inventory done (`.tmp/8_29d/inventory.md`, a read-only agent): 29 list sentences, the
        kind names, the marks' and pitch crosses' tooltips, Multi-team AATF's copy, the
        Comparator's and changes lists' undrawn fields (about 95 face fields by their Rust
        names), the team file panel's notes (finding codes), the badge's and quick actions'
        refusals (tier ids, a 0-based slot), the card labels' `Gk`/`No 10` spellings. Slices:
        - [x] d1a the AATF rows (DECISIONS "Each violation kind's name in words, its sentence
          in the game's and the ruleset's names"): `team_widgets::violations::rule_name`,
          `sentence` and `unused_sentence` under the ruleset's labels in every host (the dock,
          the marks, the pitch cross, Multi-team AATF and its copy, `check-aatf`'s `rule` and
          `problem`), the logic's `weak_foot` split (logic hash `349f0fa6`); a table test of
          all 36 kinds' names and sentences; `spec/aatf_rules.md` "Hints at a valid value"
          and `spec/save_editor.md` "CLI" follow; `help/01_command_line.md` says "what is
          wrong" since 8.29e1's merge. SWE-2 S1d1a: 3 accepted, the loop ended
          (`.tmp/8_29d/rulings_d1a.md`); the lead's rework: `Unused::allowance`'s doc names the
          weak foot's halves, `logic.rhai`'s suggestions comment says `weak_foot` gives one per
          half (logic hash now `35652515`, the sandbox `cli/dbc7.4ccrules` re-logicked), the
          interpreter fixture raises both halves and asserts both rows exactly, the catalog's
          `aatf_violation` text says "what is wrong". Astra A1d over d1a + d3a: 1 accepted
          (`.tmp/8_29d/rulings_d3a.md`), merged `3c08d261`: a held AATF result is drawn
          (dock, Multi-team list and copy, row marks, field crosses, pitch marks) only under
          the ruleset it was checked with (`aatf_check::held_result`). Open: a SWE-2 round
          over that fix
        - [ ] d1b the rest of the inventory, in two slices:
          - [ ] d1b1 the fields no view draws in words (DECISIONS "A field no view draws is
            named in words; `diff --json` keeps the schema's name"; drafted, the draft's
            `fieldWords`): the Comparator's, the changes lists' and the patch preview's rows,
            and `save-editor diff`'s text report (`field=Heading` on PES 2017; SWE-2
            S1d3a-F1); `--json` unchanged → verify: a test per surface over a face field, a
            PES 2015-only one, a team colour channel, a roster slot and `FaceRun`; SE-DIF-01
            and SE-GUI-13 restated in the plan; the inventory's section 6 grep finds no `{:?}`
            label left
          - [ ] d1b2 the team file notes in the catalog's member text and its section boxes
            capitalized, the badge's and quick actions' refusals (tier labels, the list's
            1-based places), the card and playing-style labels' spellings (`GK Long Throw`,
            `Classic No. 10`), the man-marking row's value (what the stored number names,
            from the game first)
      - [ ] d3 the stats named as each version names them (P43, accepted 2026-10-29; DECISIONS
        "Stats are named as the save's PES version names them", "Before 0.0.1, only PES
        2017's stat names are checked"):
        - [x] d3a before 0.0.1, after d1a (drafted 2026-10-29, the drafts' `statLabel`; DECISIONS
          "PES 2017's stat names, read off its Edit screen"): a stat-name function in
          `pes_savefile` taking the version, PES 2017's `heading` Header, and Form, Injury
          Resistance, Weak Foot Usage, Weak Foot Accuracy in title case, read by the player
          view, the AATF rows (d1a left the four lower case in `rule_name`, `sentence` and
          `unused_sentence`, the drafts' `RULE_NAME` and sentences title case), `check-aatf`,
          the Comparator and the changes lists → verify:
          its test per stat for PES 2017 against the maintainer's captures (2026-10-29, the
          27 names of the Edit screen's ability list). Done 2026-10-30 (Opus sidekick, merged
          `02796a4a`): `pes_savefile` `labels::stat_name(version, stat)`, one exhaustive match
          per version; `team_widgets::violations::Wording { ruleset, version }` in every AATF
          host; `SMALL_STATS` labels title case; files `labels.rs`, `violations.rs`, the save
          editor's `cli.rs`, `messages.rs`, `abilities.rs`, `terms.rs`, `live_marks.rs`,
          `multi_aatf.rs`, `aatf_check.rs`, `tactics.rs` and their tests. SWE-2 S1d3a: 2
          accepted, the loop ended (`.tmp/8_29d/rulings_d3a.md`). Open: the Astra round over
          d1a + d3a
        - [ ] d3b after 0.0.1: every other version's names, gathered from each game (the
          player.rs doc comments name no versions; Marco's RE reports for PES 2021, the
          games on G: for the rest) → verify: a test per version against the gathered names
      - [ ] d2 after 0.1.0, with Phase 20 (the maintainer, 2026-10-29): the Ruleset editor
        shows the game's names (Attacking Prowess) and writes the rules-file ids
        (`attacking_prowess`)
    - [ ] e the help chapters' shape (the maintainer, 2026-10-29: "each paragraph is a single
      extremely long line, which explains multiple facts one after the other"; DECISIONS "Help
      topics in short paragraphs under subheadings, lists for parallel items", CONTRIBUTING's
      bullet): every topic restructured, its facts kept, each change read by the lead line by
      line (a fact lost or changed is a finding)
      - [x] e1 before 0.0.1: the Save editor's eight chapters, the two core ones
        (`studio_core/src/help/core/`), and the drafts' copies in `docs/prototypes/index.html`
        → verify: `studio`'s help tests and the help export; every topic read in the built
        window. Done 2026-10-30 (Opus sidekick, merged `be189f7f`): a subheading per subject,
        menus, choices, rules and exit codes as lists; the lead read every new chapter and the
        sidekick's word audit (each reworded sentence, old and new) and found no fact
        changed; `help/*.md`, `help/core/{install,data_location}.md`, `index.html`'s `HELP`.
        The drafts' `md()` draws no numbered list (none used yet: it learns them with the
        first chapter that needs one). SWE-2 S1e1: 4 accepted, the loop ended; the three
        wording fixes landed with the rulings (`.tmp/8_29d/rulings_e1.md`). Open: the read in
        the built window (the built-window check before the next candidate)
        - [x] e1r the help search's labels as plain text (S1a-4; DECISIONS "A help search
          hit's label is plain text", drafted in `index.html`'s `helpSearch`):
          `help_window.rs` `search` drops `**`, backticks and a list item's leading dash
          from the label's line → verify: a test searching "writing a save" gets the label
          `Save editor › Running from a terminal › Writing a save`, and "next to the program"
          one starting `… › Next to the program puts`
          Done 2026-10-30 (Opus sidekick, merged `4b8df44e`): `plain_text` in `search`, the test
          on the real core chapter and a Save editor-shaped one; studio_core tests green in
          its target, the lead's run at the next checkpoint
      - [ ] e2 after 0.0.1: the Team compiler's, the Export upgrader's and the other tools'
        chapters
  - [ ] 8.21 the app's font in the drafts (`core/gui.md` "The host's other review aids"; after
    8.9f4e picks the bundled font): `@font-face` rules at egui's sizes → verify: one width
    constant's row (`TOP_ROW_ONE_LINE`) measured in the draft and in the built window agree
    within a few points
- [x] 8.2 the shell's draft, `docs/prototypes/shell.html` (`core/gui.md` "Prototypes before
  implementation"; 2026-10-19, side-lead E, branch `side/8-shell-draft`, SWE-2 in seven
  slices): one self-contained page over the Ruleset editor draft's stylesheet and helpers,
  drawing every screen and state of `core/gui.md` "Sidebar" (the logo's About block with Copy, the
  PES selector's exe-check colours, blocker, once-per-life auto-switch and "Fix PES path", the
  registered tools then the planned ones grayed, the icon-rail collapse), "Status bar" (three
  slots, each item kind's lifetime), "Window title" (activities first, the held notice while
  unfocused, set only on change), "Settings menu" (common rows plus one collapsing section per
  tool, the active one open, best-effort save raising `data_dir_read_only`), "Help window"
  (core chapters then tool chapters, a Messages topic generated from the real catalogs,
  substring search, in-doc links, F1), "Log panel" (cap, paused auto-scroll with "N new",
  filters, chevron, divider, code links to the help, line links to the grid row), the
  first-run data-location dialog and the post-update notice, with a stub central panel per
  registered tool (the Team compiler's holding a simulated compile that feeds the activity,
  the notice, the title and the log). Sample data is real (version 0.1.0, teams-list rows,
  catalog codes and severities, settings defaults); every string left for the maintainer to
  refine is marked `sample wording`. A "Draft controls" strip, visibly outside Studio, flips
  each scenario's GIVEN and ends in a walkthrough naming every `ST` id (drawn, or why not:
  ST-TTL-03, ST-HELP-04, ST-RUN-02..04, ST-CLI-01..02, and ST-SIDE-05's real poll). A
  headless-Edge harness (`.tmp/8_2/run_harness.py`, 29 checks) walks ST-SIDE-01..07,
  ST-SET-01..03, ST-BAR-01..03, ST-TTL-01..02, ST-HELP-01..03, ST-LOG-01..04 and ST-RUN-01
  against the page → verify: each of those scenarios can be walked in a browser with the
  strip's controls, and the page uses only the representable elements (the lead's read of
  every slice's diff against the list). Plan gaps found: `core/gui.md` "Settings menu" lists six
  common settings while `CommonSettings` also carries `check_for_updates` and
  `check_interval_hours` (the draft shows them); the brief's "Fix PES path" example put back
  `**` where `common.rs` means two digits (`20**`), fixed in the draft. SWE-2 review loop
  (rulings `.tmp/8_2/rulings.md`): S1 12/14, S2 7/7, S3 8/9, S4 6/6, S5 5/5, S6 1/2
  accepted, ended at S6; the rounds made the Messages topics the tools' whole catalogs
  (233, 25 and 68 rows, the upgrader's from its `messages.rs` since the plan's table is
  partial by design), one row per code, the source table's own columns; `savefile_path`
  spelled `auto`; no settings section for a tool without settings; the rest wordings and
  draft-control states. For the top lead's plan loop: `update_available`'s Install action
  against 0.1.0's "step 2 as a notice, no Update now"; `ActionEffect` has no variant for
  "Fix PES path" or "Install"; `pes_install_missing` and the selector's yellow report one
  fact twice; the help window's geometry "remembered in common settings" has no
  `CommonSettings` field; `distribution.md`'s changelog heading example carries an em dash;
  two catalog texts a member will read once the Messages topic renders them, the em dash in
  `export_upgrader/src/messages.rs` `common_reference_kept`'s doc and "Red" in
  `spec/team_compiler/messages.md` `player_layout_proto`.
  No Astra round here (checkpoint b on the draft is the top lead's call). To show the
  maintainer; their remarks go into the plan before the shell slices are briefed. Files:
  `docs/prototypes/shell.html`

### Release 0.0.1 — the preview build (DBC's editors, VGL 27's compiler and upgrader)

`plans/core/development_plan.md` "Divegrass Before Christmas" and "The /vg/ League 27 (VGL)"
(the maintainer's choice of 2026-10-23: the CLI half joins the build already planned). Steps
a-e run once 8.9e and the second draft (8.9f) are done, while the maintainer rules on that
draft, and before Phase 20, so the rehearsal's findings have the most time before the league;
f is the cut, once Phase 20 closes. The usual rigor holds throughout:
briefs, review loops, gates and mutation runs as for any step.

- [x] 0.0.1a the console's progress line and run summary (`plans/core/architecture.md` "The
  console's progress line and run summary", with `indicatif` as it names; moved from 8.10) →
  verify: ST-CLI-02's piped half by a CLI test (stdout byte-identical with and without the
  printer, nothing on stderr but findings' lines), its terminal half `manual: checked` on a
  real compile. Done: `studio` `console.rs` (`print_events`, the line, the summary), the Team
  compiler's `Progress` per committed task and one `Complete` at each way out of `build`
  (`events.rs`, `compile.rs`; the entries counted in `output/writer.rs` and `parts.rs`); the
  CLI tests in `studio` and `team_compiler`; help `01_command_line.md`
    manual: checked 2026-10-23, a ConPTY pseudo-console (`.tmp/conpty_run.py`) over
    `.tmp/0_0_1/manual_run2/` (one PES 21 export, `--no-deploy`): the line drew from the first
    export with no empty frame, its clock ticked between findings to 1:37, findings printed
    above it, and `Done: 188 files written, 0 folders skipped, 0 folders with errors, 1:37`
    remained; `check` left no line after its last finding
    manual: checked again 2026-10-23 after rework 3's printer (8995b3b0), the same run: 197
    frames, the first `[ 0/1 exports | 0:00 ]`, none empty, the parts rising to 116/117, and
    `Done: 188 files written, 0 exports skipped, 0 folders skipped, 0 folders with errors, 1:37`
- [x] 0.0.1b the launchers: `quick_compile.bat` in its CLI form and `quick_upgrade.bat` over
  `old_exports/` (`plans/core/development_plan.md` "The /vg/ League 27 (VGL)") → verify: each bat run by
  a test through `cmd /c` in a temporary bundle layout over fixture exports (an old export
  upgraded into `exports/` and the tally line; no file dropped: the how-to text and nothing
  written; an export already upgraded: refused, counted, the others still run; the compile
  over `exports/` writing its output). Done: `resources/bundle/quick_{compile,upgrade}.bat`,
  `crates/studio/tests/launchers.rs` (five scenarios, the compile's CPK deployed into a fake
  PES 21 install in the sandbox)
- [x] 0.0.1c an interrupted run: Ctrl+C in the launcher's console at several points of a PES
  2019 compile (planning, packing, deploying), of a pre-Fox compile's savefile write and of an
  upgrade; what is left where the game reads it (`download/`, the DpFileList, the savefile) and
  in `exports/`, tallied per point; a half-written file the game would read is fixed (written
  under a temporary name and renamed, as the upgrader's staging does) or recorded with why not
  → verify: the tally in the Done line, each fix tested. Done (2026-10-23, SWE-2's audit
  and kill harness, `.tmp/0_0_1/interrupt/`: `write_map.md`, `tally.md` from 65 runs, each
  killed by `taskkill /F /T` and run again): no kill left a cut CPK, save, `.partial` or
  `.tmp` under a name the game reads, and every second compile ended clean, the dead
  staging swept. Recorded, not fixed: between moving a CPK aside and renaming its
  `.partial` in, a kill leaves that CPK missing (the game runs without it until the next
  compile, which installs it); a kill between the install and the savefile write leaves the
  new CPKs with the old save, until the next compile. Fixed: `upgrade-dpfl` wrote each
  placeholder straight under its CPK name, which a kill could cut short for good (it writes
  no name that exists); it writes a temporary and renames it. Fixed: a killed upgrade leaves
  `<output>.upgrading` in `exports/`, which the next compile took as an export (all 10
  fired kills); the compiler's discovery skips it with the Warning `export_upgrade_unfinished`
  (`plans/team_compiler/pipeline.md` "1. Reader", TC-SRC-11; `reader/mod.rs` `Discovery`,
  `compile.rs`, `check.rs`, the catalog and help), with 0.0.1a rework 3. The worklog's "pre-Fox compile's savefile
  write" does not exist (the pre-Fox patch half is unbuilt): the save editor's
  `import-toml` on a PES 16 save stood in, the same writer
- [x] 0.0.1d `just release <version>` (`plans/core/distribution.md` "Distribution: portable .7z
  bundle"; Windows only), under the codename (`plans/core/development_plan.md` "The project's
  name: Unlimited Tool Works": one display-name constant for the title, About, first run,
  `--help` and its usage examples, and the help pages; the executable `UnlimitedToolWorks.exe`;
  the settings folder unchanged; verify: no member-visible text names 4cc Studio, by a test
  over the help pages and the CLI's help output): the exe, the two launchers, empty `exports/` and `old_exports/`,
  the readme and the licenses, packed as the `.7z` with its SHA256 file (the teams list is
  not a bundle file: it is embedded, `spec/core/distribution.md` "Distribution: portable .7z
  bundle"); the 0.0.1 readme in a layman's words (open the exe once to pick PES and its exe on
  the sidebar, then the two launchers; Red as the fallback) → verify: the recipe from a clean
  checkout produces both files, and the archive's listing matches the plan's. Reviews: SWE-2
  S1 4 of 8 accepted, Astra A1 2 of 5 (`.tmp/0_0_1/duck_rulings_d.md`), a mutation run 0
  missed; S1-1 opened 0.0.1g
  - [x] 0.0.1d1 the codename: the two constants, every member-visible "4cc Studio" in code,
    the help pages and the findings' text, name-free file text, the test. Done:
    `studio_core` `EXE_NAME`/`APP_NAME` (with `exe_name!`/`app_name!` for the catalogs'
    `concat!`), the shell's title, About, first run and clap command, the Team compiler's and
    Export upgrader's catalogs and commands, the help pages, the AATF ruleset headers and logic
    banner (its hash updated), the shared-IDs comment; `studio/tests/help_cli.rs`
    `no_member_visible_text_names_4cc_studio` over the help export and every `--help`
  - [x] 0.0.1d2 `just release` (`scripts/release.py`), the readme, a run from a clean worktree.
    Done: `scripts/release.py` with `release_test.py` (7 tests, the listing one through the real
    7-Zip), the `justfile` recipe, `resources/bundle/README.txt`, the workspace version 0.0.1
    (with 0.1.0 no version could be released). `just release 0.0.1` from a fresh worktree at
    51e32df0: `UnlimitedToolWorks-0.0.1-windows.7z`, 10,743,690 bytes, `sha256sum -c` OK, its
    listing the plan's (one top folder, `exports/` and `old_exports/` empty, the exe, the two
    launchers CRLF, the readme, the two licenses); the extracted exe's usage line names
    `UnlimitedToolWorks`. The exe is 35.3 MiB (30.8 on 2026-10-15): past the size budget's
    "about 30 MB", inside its 50 MB ceiling; where the growth comes from is measured with
    `cargo bloat` before the cut (0.0.1f)
- [x] 0.0.1e the rehearsal on PES 2019 with FoxDen: VGL26's old exports (the upgrader
  census's copies) through `quick_upgrade.bat` and `quick_compile.bat` from the extracted
  bundle, then checked in game in Edit mode (the `F:` drive connected by the maintainer);
  every failing class diagnosed and fixed, or written down as a reason to fall back to Red →
  verify: the tallies, `manual: checked` lines for the in-game look
    Done: the 53 exports of `C:/Data/4cc/Lab/Gud` in a copy of the 0.0.1d bundle (portable
    settings written by hand, PES 19; the first-run dialog not driven). No PES 2019 save
    matches them (the live one wears none of their boots/gloves ids, the PES 21 VGL26 cup save
    names ids 31 of them lack), so two launcher passes, as an organizer with two saves:
    the VGL26 save (22 upgraded, boots and gloves kept), then the live save (30 more); AKG
    refused both times (`texture_stem_conflict`, census-known). Compile with the defaults:
    refused to install (`dpfilelist_outdated`; answered in DECISIONS 2026-10-23, the
    organizer moving to the official list); with `cpk_name = '4cc_69_midcup'`: 248 s, installed, save
    written, per-export Errors the classes of the library run's close (`fmdl_texture_not_found`
    145, `vertex_too_far_from_origin` 89); `savefile_player_not_stripped` for every player.
    A rerun of the upgrade launcher counts every upgraded export as not upgraded: the readme
    now tells the member to move them out of `old_exports/`. Game, exe, FoxDen and save
    restored, the install's listing identical to the backup's
    manual: checked 2026-10-23, PES 2019 stock exe with FoxDen, Edit mode, the save written by
    the compile: VGL 15 (/aceg/) player 01's plane head and flight suit (a full body in
    `boots.fmdl`) and the team emblem, players 02-05 each their own head
    (`.tmp/0_0_1/e_shots/13_full_aceg01.png`, `16_sheet.png`); VGL 14 (/4ccg/) in its kit
    (green stripes, sponsor) and emblem, five custom heads, player 01 the save's face (his
    folder failed `fmdl_texture_not_found`) and player 07 headless (the export has no face for
    him: its `face.fpk.xml` names a `face_diff.bin` the folder lacks) (`17_4ccg.png`,
    `18_sheet.png`); VGL 13 (/2hug/) player 01's full body (`19_2hug01.png`). Not compared
    with Red's output in game
- [x] 0.0.1g the Team compiler and the Export upgrader greyed on the sidebar while their CLI
  stays (`plans/core/development_plan.md` "Divegrass Before Christmas": "greyed out on the
  sidebar as the planned tools are"; found by 0.0.1d's review S1-1): today a registered tool
  is always drawn live and a planned entry with its id is skipped
  (`studio_core/src/shell/sidebar/mod.rs` `contents`), so the bundle's readme, which says they
  are greyed, would be false; the registry marks a tool whose view is not built as CLI-only
  (`plans/core/gui.md` "Sidebar" item 3) → verify: ST-SIDE-08 by a shell test, and the
  bundle's launcher tests (`studio/tests/launchers.rs`) still compiling and upgrading
    Done: `StudioApp::new` takes `cli_only`; `set_active` refuses a CLI-only tool on every
    switch path, a `last_tool` naming one opens the first window tool, and none holds a close
    (it could never show its prompt); the sidebar draws it as its planned entry with the
    command-line tooltip. `studio`'s `CLI_ONLY` names the two (`shell/mod.rs`,
    `shell/sidebar/{mod,tests}.rs`, `shell/test_harness.rs`, `studio/src/main.rs`)
- [x] 0.0.1h `ingame_face.toml` (the maintainer's go of 2026-10-24: before 0.0.1, after the
  reviews in flight then): the marker becomes a file holding the in-game face, and
  `settings.toml` loses the face, hair and iris keys
  (`spec/aesthetics_export/player_folders.md` "`ingame_face` marker", the Save editor's half
  in `plans/save_editor.md`'s generation paragraph,
  decision 2026-10-24, its
  acceptance TC-STR-18 to 20, TC-BIN-47 to 49, EU-PLY-21 and 22, EU-SET-10 and 11,
  SE-GUI-33 and 34). The two measurements done 2026-10-24 (below); the plan written. Open:
  - [x] Astra over the plan section and its acceptance (checkpoint a): A1 7 of 7 accepted
    (scenarios the split broke: TC-BIN-31, EU-SET-01 and 05; THENs observed after compiling;
    SE-GUI-34 on a stripped save; the Player aesthetics editor, the Team creator and the Save
    editor's lock text), fixed; A2 5 of 5 (the in-game recipe generating both files, the
    Blender manifest naming each key's file, SE-GUI-34 on non-default looks, one file refused
    leaving the other applied, the pre-Fox patch's schema), fixed; A3 none
  - [x] slice 1, `pes_savefile`, additive: the `ingame_face.toml` document beside
    `settings.toml` (`IngameFaceSettings`, its parse and template, `PlayerSettings` joined
    from it), nothing reading it yet. SWE-2, 14 min: one key-membership function
    (`SettingKey::is_ingame_face`), the shared walkers at root `""` through a third
    `AppearanceDocument`, the template pinned by the lead's fixture; lead fix: the variants
    renamed rather than a clippy opt-out (`settings_toml/{document,keys,mod}.rs`, its tests,
    `team_toml/player/mod.rs`). Its review (checkpoint b) rides with slice 2's
  - [x] slice 2, the switch, one commit, since a `settings.toml` without the keys and an
    upgrader still writing them (or a compiler refusing what the upgrader writes) is a broken
    main: `settings.toml` loses the keys, `aesthetics_export` takes the file as
    `PlayerFolder.ingame_face` and reports the old spellings, the compiler's two codes and its
    Fox rows from both files, the Export upgrader writing the file where the marker was,
    filled at step 8 (TC-STR-18 to 20, TC-BIN-47 to 49, EU-PLY-21 and 22, EU-SET-10 and 11,
    the marker scenarios renamed). Two SWE-2 passes in one session (2a 38 min, 2b 13 min),
    one commit: `settings.toml`'s walks skip the moved keys and name them by their new file's
    path; `MetadataFile::IngameFaceToml`, `Marker::OldIngameFace` reported at a folder's
    direct position (`ingame_face_marker_renamed`, structure code, the file dropped); the deep
    pass joins both files before narrowing (`ingame_face_toml_invalid` keeps the file, ignores
    its keys); step 8 fills the file, which supersedes the empty marker at the same path. The
    replaced scenarios withdrawn in the spec. Lead: the `settings.toml` template fixture, two
    comments (`pes_savefile` `settings_toml/`, `aesthetics_export` `conventions/file_types.rs`,
    `validate/`, `team_compiler` `deep/`, `messages/`, `help/01_command_line.md`,
    `export_upgrader` `players.rs`, `settings.rs`, `upgrade.rs`, their tests)
  - [x] review of slices 1 and 2 (checkpoint b: a new `pub` interface over four crates): a
    SWE-2 loop, then Astra, then SWE-2. Mutants: slice 1 23 of 23 caught, slice 2 36 caught,
    5 unviable. S1 (two halves): 2 accepted (doc comments stating the one-file settings, the
    help on the old names). Astra A1: 2 accepted (a unit test citing TC-STR-20,
    `settings_toml_invalid` saying the defaults replace the in-game face). S2 (two halves): 2
    accepted (five more one-file doc comments; the upgrader's old-spelling test written three
    times, now `classify`'s `Marker::OldIngameFace`); every fix the lead's, amended into slice
    2's commit; the loop ends (rulings `.tmp/0_0_1h/rulings.md`)
  - [x] slice 3, the Save editor: the generation split, its "Generate in-game face" action
    landing with the view's generation (SE-GUI-19's slice; SE-GUI-33, 34). Done 2026-10-25
    (SWE-2, one run; brief `.tmp/0_0_1h/brief_slice3.md`): the draft first (the split
    templates, `dlgIngameFace`, and the old save-to-save transplant panel removed, 8.9j5c's
    S1-2); `view/generate.rs`, two Player-menu items, one Modal (stripped: skip text and OK;
    folder field and Pick, the text as written, Write on an existing folder), the tool's first
    shell notice; DECISIONS "Generate settings.toml and Generate in-game face: a picked folder,
    an update in place, a whole face file"; help `06_player.md` (the Player menu's chapter, the
    sidekick's contradiction, accepted) and the Comparator paragraph in `02_window.md` (8.9j5c's
    edge). Six view tests, red first (SE-GUI-19 on the PES 21 fixture's colour-coded rostered
    player, SE-GUI-33, comments kept, a file that does not parse, the stripped skip, Write's
    gate), and two from `mutants-diff 845d7d1f`'s three survivors (the dialog's heading; a
    `settings.toml` that cannot be read is the error, never replaced), red against each mutant
    by hand. SWE-2 S1: 1 of 7 accepted (the draft's folder no longer prefilled; loop ended).
    manual: checked 2026-10-25 by the maintainer on the release build (`.tmp/0_0_1h/app`, the
    PES 21 save, /co/'s /CO/LETTE): both files written as previewed, settings.toml without
    the face keys, a hand-added comment kept on regeneration; the notice judged too subtle,
    hence 8.27
    Files: `save_editor/{Cargo.toml,help/0{2,6}_*.md,src/lib.rs,src/view/{generate,mod,
    menus,player_list,tests}.rs,src/view/tests/{generate,player}.rs}`,
    `docs/prototypes/tools/save_editor.js`
  - [x] the spec moved for slices 1 and 2. Done 2026-10-24 (SWE-2, 32 min): the marker
    paragraph, the two templates (byte-equal to the goldens, checked by script), the object
    model, the two codes, the upgrader's steps and the ten scenarios in `docs/spec/`; the plan
    section keeps the Save editor bullet and SE-GUI-33, 34; a GLOSSARY row. Files:
    `docs/spec/{aesthetics_export/{player_folders,settings_toml,object_model}.md,
    team_compiler/{README,messages}.md,export_upgrader.md}`, `docs/plans/aesthetics_export/
    {player_folders,settings_toml,object_model}.md`, `docs/GLOSSARY.md`, comment lines in
    `pes_savefile/src/settings_toml/`
  - [x] in game, after slice 3: a generated file's face on PES 21 (SE-GUI-34). PES 17's save
    route waits for the aesthetics patch's pre-Fox half (`plans/pes_savefile/operations.md`
    "Aesthetics patch (new)"), so a pre-Fox compile is checked by its resolved settings alone

  SE-GUI-34 manual: checked 2026-10-25, PES 21 Edit mode, release build of `f3d1f135`: VGL 22
  (792) slot 06 (79206) set by Team TOML to long red hair, blue iris (7) and face size 5; the
  window's Generate settings.toml and Generate in-game face wrote his folder from that record;
  the record then changed to short blond hair and green iris (9) and the save stripped
  (strip-and-seed), the tracer export compiled into the test slot: he draws the long red hair,
  and his irises measure (154, 180, 174) and (155, 180, 176), the blue iris of 0.0.1h's control
  (153, 178, 176), not green. Save and slot restored by hash (`.tmp/se34/`)
  - [x] the plan section closed for slice 3. Done 2026-10-25: SE-GUI-33 and 34 moved beside
    SE-GUI-19 in `plans/save_editor.md` (the Save editor's view stays a plan until Phase 8
    closes), SE-GUI-34's "changed in Edit mode" widened to any change of the record (the
    check changed it through Team TOML); its generation paragraph already held the section's
    Save editor bullet, now with the why and both actions skipping a stripped player; the
    section removed from `plans/aesthetics_export/player_folders.md`, the other consumers'
    changes being in their own plans. Files: `docs/plans/{save_editor,aesthetics_export/
    player_folders}.md`

  manual: checked 2026-10-24, PES 21 Edit mode, /egg/ (792) slots 05-07 compiled by
  `00ef6231`'s release exe into the test slot over a stripped copy of the save (restored by
  hash): a face model (4CCG's "Sou Biden", no eye material) with no hair model draws its own
  grey scalp, not the in-game long red hair, and blue (7) and violet (10) irises draw the same
  grey; the marker's player draws the red hair and blue irises. 1186 of the machine's 1472 Fox
  face models name no eye material
  manual: checked 2026-10-24, PES 17 Edit mode, /out/ (740) slots 05-07, the same settings
  set through the Save editor's Team TOML (a pre-Fox compile writes no save yet) and the faces
  deployed into the test slot (save and slot restored by hash): a face model (livecpk 70102)
  draws no in-game hair and the same painted iris for 7 and 10; the marker's player draws both
- [x] 0.0.1i `libs/aatf`'s audit for 0.0.1 (the maintainer's "a rock-solid AATF check comes
  first", 2026-10-24; side-lead track B, 2026-10-25): the converge audit and design-health
  pass of `AGENTS.md` "Closing a phase" over the crate alone (`plans/aatf_rules.md`,
  `spec/aatf_rules.md` and their acceptance IDs: every requirement present and tested; the
  design-tell sweep; `just mutants aatf` with every survivor triaged; `just citation-audit`;
  every `pub` item and its consumer), plus a census: every save on the machine through
  `check-aatf` under DBC7's rules file, tallied by outcome, each failing class diagnosed
  against the reference editor's rules (`aatf.cpp`); each gap a sub-step fixed in its own
  commit; then 8.9f4d's library half, the player's own tier recognition, as a `pub` API
  (checkpoint (b)) → verify: the audit's list closed, the census re-run clean or each class
  ruled, mutation runs clean
  - [x] the audit (lead, 2026-10-25, at `e14a64da`; notes and scripts `.tmp/0_0_1i/audit/`):
    the spec's and plan's type blocks against `ruleset/mod.rs` and `violation.rs` field by
    field (all present, DBC7's five fields and three kinds included); every `LoadRule` and
    `LoadError` variant has a failing-file test; the accessor table against `engine.rs`; the
    operation-limit margin and the endless-loop test; `apply_tier` then `check_team` for
    every tier of every ruleset (DBC7's through its fixtures' `legalize`, every legal team
    built by `apply_tier`); a suggestion fixture per allowance kind; DBC7's fixture list
    (`plans/aatf_rules.md` "Development phase and verification") all present. Design sweep:
    no suppression, no trait beyond `TryFrom`/`Display`, the `_ =>` arms on bytes. `pub`
    items: each has an outside caller or is a type reachable through one, but
    `Rules::tier_values` (sub-step a). Citation audit (1048 pairs, 27 flags): none on an AATF
    scenario; the flags are the Team compiler's, the Ruleset editor's and the comparator's,
    left to their crates' closes
  - [x] the census, before (`.tmp/0_0_1i/census/`, `tally_before.txt`): every save the
    Everything index finds (1112 `EDIT00000000*` paths and 40 cup archives opened, F: not
    connected), 424 distinct files by SHA-256, each copied and run through `check-aatf`
    under DBC7's assembled file (logic 1, `d09648ca…`) by `e14a64da`'s release build: 422
    checked, about 84,000 teams of 701-920, no check failure; 2 refused by the save reader,
    both outside `aatf` (a decrypted dump `EDIT00000000.dat`, not a save; a PES 17 test save
    whose player name is not UTF-8: Issues). The small classes ruled against the DBC7 fork's
    `aatf.cpp`: `InjuryResistance` 4 of 3 (a stored 3, which the fork errs on too, line
    450), `APositions` on 189 cm keepers (the fork's line 292), `TierCount` on short or
    empty squads in the cup range (the schema's rest-of-squad count; the fork counts no
    squad size). The registered-position warning on 80-100% of some saves' players is their
    placeholder teams (every rating C), absent from the cup teams
  - [x] the census, after (`tally_after.txt`), at `5020b2ff`'s release build (sub-steps a to
    c, DBC7 re-assembled with the new logic, `29cb6048…`): the same 424 files, the same
    outcomes and counts; the regression check finds 0 files whose outcome or findings differ
  - [x] a `Rules::tier_values` and `TierValues` crate-private: no consumer calls them (every
    one goes through `apply_tier`, `plans/team_creator.md`, `plans/save_editor.md`); the
    spec's "Crate layout" lists it with the host API, corrected with the change. Done
    2026-10-25 (SWE-2). Files: `aatf/src/{rules,values,lib}.rs`, `docs/spec/aatf_rules.md`
  - [x] b the whole-crate mutation run's survivors (`just mutants aatf` at `e14a64da`, local,
    4 h; summary `.tmp/0_0_1i/mutants_whole_1/`): 439 mutants, 387 caught, 36 unviable, none
    missed, 16 timeouts, which a rerun alone at a 600 s timeout settled (`rerun_timeouts/`:
    19 caught; `engine -> Default::default()` drops the operation limit, so the endless-loop
    test loops: explained, caught); the run had been starved by the census beside it. Extra
    kinds: 14, the 6 timeouts rerun; 3 survivors, each a missing test or dead code: `wrap_item`'s
    and `universal_section`'s first-field guard (`.trim()` removed: an overlong first field
    wrapped under a bare head line; a test each) and `universal_section`'s last
    `trim_end()`, a no-op (the last piece's space is popped) now removed. Done 2026-10-25
    (SWE-2; the brief's comma-less expected line was wrong, the sidekick's contradiction
    accepted). Files: `aatf/src/writer/{mod,tests}.rs`
  - [x] c 8.9f4d's library half: `Rules::player_tier` and the logic's `player_tier`
    (`plans/aatf_rules.md` "The player's own tier", DECISIONS 2026-10-25), checkpoint (b).
    Done 2026-10-25 (SWE-2, one rework: the script call one `Rules::call`, the tests' tier
    lookup one `tier_of`): the logic's fourth function over `height_system` and
    `read_player`, `Rules::load` requiring it, logic 1's hash moved (`29cb6048…`; frozen at
    the cut); the Green silver giant read under the team's system (`bronze` without it, the
    red run), DBC7's Manlet silver at 92, VTL11's no-tier silver, and `None` exactly on
    `check_team`'s `NoTier` slots over the nine blocks on three versions (135 `None`, 486
    `Some`). `mutants-diff adb35edc` 4 caught, 4 unviable. Files: `aatf/{logic/logic.rhai,
    src/{rules,file,lib}.rs,src/fixtures/{mod,official,dbc7,vtl11}.rs}`
    - [x] review: SWE-2 S1 over `e14a64da..5020b2ff` accepted 2 of 6 (the doc comments of
      `Rules::player_tier`, which left out step 2's fallback, and `CheckError::Values`; the
      lead's fixes, amended), ending the SWE-2 loop (`.tmp/0_0_1i/rulings.md`); Astra A1
      (checkpoint b, `Rules::player_tier` and the logic's `player_tier`; top lead, Codex,
      2026-10-25, on main after the merge `04749488..450c1851`): no concerns, the loop ends
  - [x] the close run, after the SWE-2 loop (`just mutants aatf` at `28c0ce53`, local, 3 h
    40 min; summary `.tmp/0_0_1i/mutants_whole_2/`): 443 mutants, 387 caught, 37 unviable,
    none missed, 19 timeouts: the engine's explained as at the first run (no operation
    limit, the endless-loop test loops), the other 18 rerun alone at 600 s with their
    siblings (`rerun_timeouts_2/`, 23 mutants): 22 caught, 1 unviable; the extra kinds 13 of
    13 caught. The timeouts are the shared machine: both runs' reruns caught every one
- [x] 0.0.1k the window's diagnostic log before the cut (`plans/core/architecture.md`
  "Diagnostic logging", both GUI bullets; the maintainer asked 2026-10-25 about a crash report
  like Red's `crash.log`): `studio.log` in the data directory through a small `log::Log`, the
  panic hook writing the message, its location and a backtrace there and telling the member in
  a dialog to post the file, the previous run's log kept as `studio.old.log`, the version as the
  first line, the home directory written as `~`. Today a GUI panic prints to the console window
  that closes with the process, so the member loses it. Measured 2026-10-26 (a probe crate,
  toolchain 1.98.1): a release exe's backtrace names its frames only with its `.pdb` beside it
  (17 MB for `4cc-studio.exe`'s); without it every frame reads `<unknown>`, while the panic's
  message and `file:line:col` are there either way; so the log writes offsets and the
  release keeps its symbols (DECISIONS 2026-10-26, the maintainer's choice)
  - [x] k1 the sink, the hook and the dialog: `crates/studio/src/gui_log.rs`, `main.rs`
    (`run_gui_mode` installs the log and the hook, `guard_gui` around the window);
    `studio_core` exports `version_label`. ST-RUN-07 proven
  - [x] k2 the backtrace as module base and offset (`backtrace` crate) in place of std's
    resolved `Backtrace`; `scripts/symbolize_log.py <log> <symbols folder>` over MSVC's
    `llvm-symbolizer`; the release profile's `debug = "line-tables-only"` and the recipe's
    `-symbols.7z` (`plans/core/distribution.md`, the release recipe's paragraph); the next
    start's report of a session that did not close normally (the maintainer asked
    2026-10-26 for a popup when or after a crash; DECISIONS 2026-10-26) → verify: ST-RUN-09; a
    test that the hook's lines read as base+offset; the script's tests; the release exe's
    size before and after the profile line (the probe's did not change; if ours grows,
    report it). Done (SWE-2, one rework): `gui_log.rs` (`frame_line`, `backtrace_lines`,
    the `session` closing lines, `InstalledLog`, `unclean_text`, `show_dialog` with the
    unclean notice at `Info`), `main.rs`, `scripts/symbolize_log.py` and its test (run by
    `doc-pointers`), `scripts/release.py`'s `-symbols.7z`; the release exe 39,195,648 B before,
    39,195,136 after, the `.pdb` 17 MB to 117 MB (`plans/core/architecture.md`)
  - [x] k3 ST-RUN-08 in the built window (manual, asked before it runs; with 0.0.1l's
    ST-RUN-04), on a release build with a panic added by hand: the dialog, exit 101, and the
    log's frames named by `symbolize_log.py` against that build's kept exe and `.pdb`; the
    next start shows no second dialog for it, while one after a kill from Task Manager does
    (ST-RUN-09)
    - ST-RUN-08 manual: checked 2026-10-28 by the lead on a release build of `825f1885` in
      the `ingame` worktree with About's Copy panicking by hand (reverted, rebuilt, the
      bundle's exe free of it), its own data folder (`.tmp/0_0_1k3/`, `k3.py`): the click
      shows a dialog repeating "k3 test panic: About's Copy clicked" and naming `studio.log`;
      once closed the process exits 101; the log holds the message, `about.rs:63:17` and 59
      frames, which `scripts/symbolize_log.py` names against the kept exe and `.pdb`
      (`about::show` at `about.rs:63`), and ends "the crash above was shown to the member"
      (kept, symbolized, as `.tmp/0_0_1k3/symbolized.log`; the later starts rotated it out).
      ST-RUN-09 the same day: no dialog on the first start (no earlier log) nor after the
      reported crash; after the process was killed, a dialog before the window naming
      `studio.old.log`; a WM_CLOSE ends the log "the window closed normally", exit 0
  - [x] k4 the crash dump (P38, accepted on a measured condition; `plans/core/architecture.md`
    "Diagnostic logging", the "crash dump" bullet; ST-RUN-10), after k2: first the release exe
    built with and without `crash-handler`, `minidumper` and `minidump-writer` wired in a
    probe; over 10 MB of growth, stop and report → verify: the measurement; ST-RUN-10, ST-RUN-11
    (manual, asked first); the
    window opening when the monitor cannot start
    - [x] k4a the measurement (`.tmp/0_0_1k4/probe/`, the release profile, the monitor, the
      hook and a stack overflow on the argument `stack` wired): 243,200 B without, 469,504 B with, so 221 KiB of
      the 10 MB allowed; the probe's overflow wrote `studio.dmp` (1 MB), read by `minidump`
      0.27 as five threads and exception `0xC00000FD` (DECISIONS 2026-10-26)
    - [x] k4b the build (the `unsafe` block allowed by the maintainer, 2026-10-27; DECISIONS
      "The crash dump's hook is one `unsafe` block of ours"): the attach through
      `crash-handler`'s `unsafe fn make_crash_event` with its `// SAFETY:` comment, the plan's
      bullet as written, `minidump-writer` only through `minidumper`
      unless our code calls it, the rows in `plans/core/README.md` "External Dependencies",
      `just deps-check`, and our release exe's size before and after (the probe's is a bound).
      Done 2026-10-27 (SWE-2, one run): `crates/studio/src/crash_dump.rs` (the hidden
      `--crash-monitor <data location> <socket>` and `--crash-test stack`, `DumpWriter`,
      `watch_crashes`, the one `unsafe` block, `overflow_the_stack`), `gui_log.rs`
      (`LastSession` in place of the unclean flag, `studio.log` opened for appending,
      `dump_path`, the dialog naming both files), `main.rs`, `tests/crash_dump.rs`
      (ST-RUN-10 end to end); 37 packages in `Cargo.lock`, licenses ok; the release exe
      39,465,984 B before, 39,609,856 after (+140.5 KiB); the release build's
      `--crash-test stack` exits `0xC00000FD` with a 1 MB dump and the monitor's line last.
      ST-RUN-11 manual: checked 2026-10-27 (k4b's release dump, its exe and `.pdb` kept in
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
      timestamps and the PDB signature), over a fresh `data/` (`.tmp/0_0_1k4c/`): the window
      opened (`window.png`), one process and no monitor, the log's second line "the crash
      monitor did not start, so this session writes no crash dump: cannot start the crash
      monitor …4cc-studio.txt: The system cannot find the file…", WM_CLOSE's closing line last.
      Linux: `tests/crash_dump.rs` passed in `just gates-remote` on bonfire (Debian,
      2026-10-28; expects SIGSEGV, 11); CI's own job ran red then for another cause, the
      section-pointer check reading the gitignored `.tmp/` (fixed the same day). Reviews:
      SWE-2 S1 7 of 18, S2 1 of 5, Astra A1 (checkpoint b) 0 of 1 (`.tmp/0_0_1k4b/rulings.md`)
- [ ] 0.0.1l no console window in the preview (`plans/core/architecture.md` "Windows
  subsystem", DECISIONS 2026-10-25, `AttachConsole` approved): `#![windows_subsystem =
  "windows"]` on every Windows build (the plan's paragraph after it says why debug too),
  `AttachConsole(ATTACH_PARENT_PROCESS)` behind one safe function when command arguments are
  present, the window mode's startup failures in a message dialog, after 0.0.1k (a panic
  needs the log first) → verify: the gates' piped CLI tests (`help_cli`, `launchers`,
  `team_compiler_cli`) on the GUI-subsystem binary; ST-RUN-04
  - [x] l1 the subsystem on the exe (not on the unit tests' harness, whose output the
    runner would lose), the attach in `main` when arguments are present, `startup` returning
    its failure's text for the CLI to print and the window to show (`show_error_dialog`);
    `tests/subsystem.rs` reads the PE header's subsystem (2); the piped CLI tests pass on it
  - [ ] l2 ST-RUN-04 in the bundle (manual: a double-click opens no console; `team-compiler
    check` from a terminal prints there; both launchers print and their `pause` waits),
    asked before it runs; with k3. Runs on the cut's own bundle (0.0.1f): `just release`
    writes the release archive, which it never overwrites, so no bundle is packed before the
    maintainer's final pass
- [x] 0.0.1m the window's fit and its scaling with the width (the maintainer's UI review,
  2026-10-26; `plans/core/gui.md` "Shell layout", the paragraphs "The window's size" and
  "Scaling with the width"): the first frame fits the height alone, keeping 1280 wide where
  the window fits; under the design width the shell sets the zoom factor from the native
  pixel width, at or above it 1 → verify: ST-WIN-01 (its new numbers), ST-WIN-02. Then an
  audit of the views' narrow-pane fallbacks the scaling makes unreachable (the Save editor's
  header rows, Profile above the quick actions, fewer stat columns, the Tactics settings
  column below the pitch: `plans/save_editor.md` "Editing UI"), each kept or removed with its
  plan text. The size classes (ST-WIN-03, `StudioTool::window_size`) land with the Music
  player's view (Phase 10), their first special tool: a trait method no tool implements
  before then would be an item without a consumer
  - [x] m1 the fit and the zoom. Done 2026-10-26 (SWE-2): `fitted_size(monitor,
    decorations)` fits the height alone; `width_zoom` set each frame from
    `RawInput::screen_rect` times the current zoom, the one size egui rescales when a queued
    zoom lands (the sidekick's contradiction: `inner_rect` keeps the zoom the input was built
    with and fed back, 0.5625 for 0.75 at 960 points); egui's keyboard zoom off (DECISIONS).
    Files: `studio_core/src/shell/{window_size,mod}.rs`
  - [x] m2 the narrow-fallback audit, after 0.0.1s2 rewrites the Save editor's layout (the
    maintainer, 2026-10-26: the player list is not resizable, DECISIONS "The player list keeps
    its 240 points"; with the zoom that leaves no narrower view, so the audit's candidates
    include the three columns' wrap, s3g's one-column cards and the quick actions' halves; if
    the one-column cards stay, Astra A2a-1 found that their cached width keeps one column
    after the pane widens again in the same context, `.tmp/0_0_1s/s3_rulings.md`); the
    built window text and ST-WIN-01/02 move to `spec/core/gui.md` with it. Audited
    2026-10-27 (Fable, `.tmp/0_0_1m/m2_audit_report.md`, rulings `m2_rulings.md`): of the
    four candidates above none is left in the code; six width branches are, five reached only
    in the one frame after a width jump (the columns' wrap, column 2 shrunk to the pane, the
    one-column cards, Appearance's fewer pairs, the team row's wrap), the quick actions'
    halves by a rules file's long label. Ruled: the shell discards the pass that changes the
    zoom, the five go, the halves stay, the view tests draw the 1224-point design pane (113
    drew 1100); plan and DECISIONS "No narrow-pane fallback, and no frame laid out under the
    design width". Done 2026-10-27 (Opus, `.tmp/0_0_1m/m2_report.md`): the shell's
    `request_discard` after a zoom change (ST-WIN-02's jump test); the columns' wrap, column
    2's shrink, the one-column cards, `pairs_per_row` and the team row's wrap removed; 112 view
    tests drawn at `DEFAULT_PANE` (1224) instead of 1100, seven tests of the removed fallbacks
    deleted; the design fit and the halves were already pinned (SE-GUI-37's test, the team
    row's, the halves test, now through `expected_columns`, whose helper read the rows' width
    for the band's). Files: `studio_core/src/shell/{mod.rs,window_size/}`,
    `team_widgets/src/cards.rs`, `save_editor/src/view/{abilities,mod,team}.rs`,
    `view/appearance/mod.rs`, `view/tests.rs`, 15 files of `view/tests/`. The paragraphs "The
    window's size" and "Scaling with the width" and ST-WIN-01/02 moved to `spec/core/gui.md`
    (its first "Acceptance" block; `spec/core/README.md`'s stale keyboard zoom corrected)
  - [x] m2b the implementation's findings (`.tmp/0_0_1m/m2_rulings.md`, brief `brief_m2b.md`):
    a player view's first frame drawn 26 points right of every later one at the design pane
    (the centring pad's widths from egui's first-show pass); `the_two_column_fit_is_the_drafts_330`
    asserting what always holds; `Headless::new()`'s 800-point window, a pane no member sees.
    Done 2026-10-27 (Opus, `.tmp/0_0_1m/m2b_report.md`): the jump came on every player
    selection (a new `Grid` sizes itself on a discarded first pass); the view discards a pass
    whose measured widths differ, and `theme::install` allows three passes (the sidekick's
    contradiction: egui ignores the discard at two; spec, DECISIONS "A frame may run three
    passes"); the span test pins `TWO_COLUMN_WIDTH`; `Headless::new()` draws `DEFAULT_PANE`,
    two list tests given their short window or an aim from the window's height; m2's settle
    frames removed. Files: `studio_core/src/theme.rs`, `team_widgets/src/cards.rs`,
    `save_editor/src/view/{abilities,mod,tests}.rs`, `view/tests/{abilities,list,live_marks}.rs`.
    The three older "The view moves after its first frame" comments in
    `view/tests/abilities.rs` (before its `scroll_into_view` loops), whose premise is gone,
    say why the loops scroll (2026-10-28)
- [x] 0.0.1n the drafts' Layout edit and scaling (the same review; `plans/core/gui.md`
  "Layout edit", the "Size:" bullet): a moved panel pinned where dropped, a columned panel
  narrowed into fewer columns, the simulated window scaled under the design width
  (`.tmp/drafts/brief_drafts_layout.md`). Done 2026-10-26 (SWE-2): a move stores window
  points and draws the panel fixed inside the always-transformed `#win-body`, a slot of its
  size left in the flow; old `dx`/`dy` tweaks migrated once; nine columned panels take their
  count from their own stored width (`COL_PANELS`, widths measured in the browser), a
  columned panel's resize storing the asked size; `#win-body` scaled by `transform`, not
  `zoom`, so the host's menus and dialogs outside it keep their placement (they draw unscaled);
  pointer maths divided by the scale. Lead fixes: the host's default fit the plan's (height
  alone, 1280 × 691 on 1366 × 768, was 16:10), the scale read from the window's own width so
  1280 draws at 1 and 960 at exactly 0.75 over a 1280-point layout. Headless check
  (`.tmp/drafts_layout_check.py`, Firefox): 12 of 12. Files: `docs/prototypes/{index.html,
  tools/save_editor.js}`
- [x] 0.0.1o Live AATF seen where the fix is (the maintainer's ruling on P37, 2026-10-26;
  DECISIONS "Live AATF stays off by default"; `plans/save_editor.md` "Configurable AATF
  rules", the "Live AATF" bullet): the toggle a framed button, the player list's cross or check mark per checked
  player, the cross beside the field a selected player's violation is about (one kind-to-field
  match), a `FieldedNotA` row of the dock or the AATF panel selecting his team's row on the
  named preset's first formation carrying the cross (0.0.1s2: the pitch is the team view's), the
  `violations` widget's event naming the violation → verify: SE-GUI-36; a test per field group of the match (one kind each, real
  violations of the PES 17 fixture's team where it has them); the draft (`tools/save_editor.js`)
  the same first, for the maintainer's look
  - [x] o1 the draft (lead, 2026-10-26): the framed toggle, the list's marks (team row
    included), the field crosses for every kind the sample run reaches (Height, Stat; StatTotal
    and Special mark the row alone); FieldedNotA has no sample row, so its pitch cross is the
    code's alone. Where the marks sit and their glyphs: DECISIONS "Live AATF's marks: where
    they sit and their glyphs". Headless check `.tmp/0_0_1s/draft_live_marks.py`: 17 of 17.
    The maintainer's look is open; the code (o2) follows s3's close without waiting on it,
    the plan having ruled the marks. Files: `docs/prototypes/tools/save_editor.js`,
    `plans/save_editor.md`
  - [x] o2 the code
    - [x] o2a the toolbar's framed toggle, the list's marks (player and team rows, their own
      hover, the text truncated before them), the field crosses beside every field the plan
      maps (`live_marks.rs`: `held_violations`, `marked_field` with no `_ =>` arm,
      `row_mark`, `field_cross`/`paint_field_cross`, one `MARK_GAP`), `violation_line` shared
      by the dock's rows and the crosses' tooltips. Tests: SE-GUI-36, one per field group, a
      click on a mark selects its row, no cross over text and no field moved. `FieldedNotA`
      maps to the pitch, which draws nothing yet. Files: `save_editor/src/view/{live_marks,
      info_strip,abilities,player_list,mod}.rs`, `view/tests/live_marks.rs`,
      `team_widgets/src/violations.rs`. SWE-2 round S1: 8 of 11 accepted
      (`.tmp/0_0_1o/o2a_rulings.md`); its rework r2 done 2026-10-26 (SWE-2): the laid-out
      crosses keep `MARK_GAP`, a row's unsaved dot paints after its truncated text, the tests
      tell the Skills header's two crosses apart, pin one mark per row, a field's several
      violations (one cross, the highest severity's colour, every line) and other teams' rows
      unmarked. Round S2 (4 of 5 accepted) and the mutation run's ten survivors in reworks r3
      and r4, 2026-10-27 (SWE-2): a dirty row's dot at the row's right end in the warning
      colour, one item spacing plus `DOT_MARK_GAP` before the mark (the draft's 9 px, measured
      by `.tmp/0_0_1o/draft_dot_mark_probe.py`), the text's truncation reserving exactly what
      paints after it (`end_room`, `dot_right`); the tests count both glyphs per row, a
      suggestion through the severity filter, the mark centred on its row, the truncation's gap
      on a run of '.', a second-half field's cross, a selected team row's fill; the card columns
      aligned past PES 17's disabled cells (`team_widgets`, the maintainer's request); the AATF
      worker's test deadline 120 s. Rulings `.tmp/0_0_1o/o2a_rulings.md`. Astra round A1 (3
      of 3 accepted, its loop ends) in rework r5, 2026-10-27 (SWE-2): the toggle reads `▶ Live
      AATF` as the plan says; one table test puts a lone violation's cross on its own control's
      row for every field a kind reaches but the pitch and the stats, pinned on x where two
      fields share a row (red: Age's control handed Weight's field, the COM header's marks
      dropped). Files: `view/mod.rs`, `view/tests/{live_marks,live_aatf}.rs`. The post-Astra
      SWE-2 round S3: 2 of 2 accepted, the review loops end; the lead's fix pins every cross on
      x (ending before its control, starting after its label; the Skills header's cross before
      the Cards left badge that shares its row), measured on the glyph, since a label in a
      wrapping row is laid out from the row's start (red: the two header marks swapped; the
      Name cross moved into its field). Open: one `mutants-diff dc02a76e` (run 2026-10-27,
      34 survivors triaged; both halves landed, the `studio`/`studio_core` tests and the views'
      and lineup's, eight survivors excluded in `.cargo/mutants.toml` with their reasons, two
      advance rects built from their corners; the eight disk-full mutants re-tested, all caught;
      left: a `mutants-diff 3d39aff3` over the two commits)
    - [x] o2b the pitch's cross for `FieldedNotA` and the dock row selecting his team's row
      on the named preset's first formation; after the Tactics tab's new layout (the
      maintainer's request of 2026-10-26, drafted first). Drafted 2026-10-27 (lead; DECISIONS
      "Live AATF's pitch cross: left of the badge, over every marker"): `fieldedRows`
      re-reads the kind from the record, the cross left of the badge after every marker, its
      tooltip, `jumpToCross`. Headless check `.tmp/0_0_1o/draft_pitch_cross.py`: 17 of 17
      (the re-read finds none on the sample team, as its real run; a moved starter's row,
      cross and tooltip; none on a formation or preset not fielding him; the jump to the
      kick-off and to a fluid preset's third formation). Code done 2026-10-27 (SWE-2, one
      session): `aatf::Position::byte`; `CROSS` and `MARK_GAP` moved to
      `team_widgets::violations`; `FieldedMark` on `RosterRow.fielded`, the Save editor filling
      it from the held check; the pitch paints the cross after every marker, `MARK_GAP` left of
      the badge, its tooltip the lines; `SelectedPlayer.fielded` (`FieldedAt`) makes the dock's
      and the AATF panel's row select the team row on the named preset's first fielded formation
      holding the position (the kick-off one when the save has moved since the check). Lead:
      the fielded-formations rule one method, `TacticsPreset::fielded_formations` in
      `pes_savefile`, read by the AATF check, the pitch and the jump (it was written three
      times); red: a fluid preset fielding its kick-off one alone fails the jump, the pitch's
      gating and four of `aatf`'s tests. Files: `pes_savefile/src/model/tactics.rs`,
      `aatf/src/{ruleset/mod.rs,team.rs}`, `team_widgets/src/{violations.rs,tactics/mod.rs,
      pitch/mod.rs,test_support.rs}`, `save_editor/src/view/{live_marks,tactics,aatf_check,
      aatf_panel,live_aatf,comparator,prompt,terms}.rs`, their tests, `help/02_window.md`
    - [x] the built-window look at the marks (their clearance at the real font), 2026-10-28 at
      `825f1885` (Live AATF on the PES 2021 save copy, Autumn 26: 67 issues;
      `.tmp/0_0_1z/shots/w5_10_live.png`, `w5_11_marks_zoom.png`, `w5_13_cards_zoom.png`):
      a truncated row's ellipsis ends 13-14 px before its mark; the Cards left badge's cross
      sat 3 px of ink after "cards", where the drafts spaced every cross 4 points
      (`.xmark`'s margin, the pitch's `PITCH_CROSS.gap`) and the app's `MARK_GAP` is 2. At 4
      the app's Registered position cross ended 2 points inside its combo (three
      `live_marks` tests failed), so the drafts took 2 instead (DECISIONS "Live AATF's crosses
      2 points from what they mark"; `.tmp/0_0_1s/draft_live_marks.py` 16 of 17, its one
      failure the toggle's label it predates, "▶ Live AATF"); the pitch's cross test
      measures the literal 2
- [x] 0.0.1p the upgrader's accepted CLI items for VGL 27's managers (P17, P18, P21; DECISIONS
  "Usability proposals P17-P24 and P32-P38 accepted"; built: `spec/export_upgrader.md` step
  10, "CLI", "Message catalog"), one commit each:
  - [x] p1 `unmigrated_content`'s `reason` and `other`: first the list of emission sites, each
    given its value → verify: EU-REP-05; every reason reached by a test (the existing
    fixtures' refusals first), `other` on the two-candidate ones. Code done 2026-10-26
    (SWE-2): `UnmigratedReason`, 28 values over 32 sites, every site through one
    `messages::unmigrated` constructor (reason, then `other`, then the site's own fields);
    the `Kit Configs`, `Kit Textures`, `Portraits/` and `Logo/` refusals the fallback used to
    report now emitted where the cause is known (same files, same severity); `other` on every
    two-candidate cause (nine). Lead: every value's test checked by a script; EU-REP-05 on
    EU-PLY-10's real-named folders. The spec's catalog row and the help say it.
    `mutants-diff 6ff37870`: one missed (`stem_clashes`' choice of `other`), its test in p2/p3's
    brief. Files: `export_upgrader/src/{messages,players,kits,carried,materials,
    referees,model_folders,kit_magic,upgrade}.rs`, `tests/cli/*.rs`, `help/01_command_line.md`,
    `spec/export_upgrader.md`
  - [x] p2 `coverage_tag_chosen` and `--coverage full|midcup` → verify: EU-CLI-09. Done
    2026-10-26 (SWE-2, one rework: `name` outranks the substring, so `co midcup Day 5`
    reports `name`): `naming::coverage_choice` returns the tag and its `TagSource`
    beside the output name, so the name and the report cannot disagree; a flag naming the
    other tag than the name's second word, or any flag on a referee export, exits 2
  - [x] p3 a leftover staging folder removed with `staging_folder_removed` → verify:
    EU-CLI-10 (EU-CLI-05's staging clause replaced); a finished output still refused. Done
    2026-10-26 with p2 (one commit): `claim_staging` removes the leftover through the finish
    step's lock retry and returns whether it did; a removal that stays locked exits 3 (a
    Windows-only test holds a file open). The built additions moved to
    `spec/export_upgrader.md` "CLI", "Message catalog" and step 10 (the plan keeps "Several
    exports at once" and the GUI). p1's missed `stem_clashes` mutant has its test
    (`referees.rs`). `mutants-diff 57ae9811`: 15 caught, 3 missed (the refusal not pinning
    `Coverage::as_str`'s value; no name holding both `midcup` and `additions`), their
    assertions added (rework r2), all three red against the mutant by hand. Files:
    `export_upgrader/src/{cli,naming,staging,upgrade,model_folders,messages,referees}.rs`,
    `tests/cli/cli.rs`, `tests/upgraded_golden.rs`, `help/01_command_line.md`,
    `spec/export_upgrader.md`, `plans/export_upgrader.md`
- [x] 0.0.1q the window reopens where it was closed (P24; `plans/core/gui.md` "Window size
  classes", ST-WIN-04; after 0.0.1m, the same code): the main class's `window_main` now, the
  special class's with the Music player's view. First how eframe 0.37 lets the shell learn the
  monitors at start (eframe's own persistence clamps through winit's monitor list), the finding
  a DECISIONS entry → verify: ST-WIN-04 headless where the harness reaches; the off-monitor
  case by hand (ASK FIRST). The finding (side-lead S, 2026-10-26, DECISIONS "eframe 0.37
  gives the shell the monitor list only once the window exists"): no route before the window
  exists, and after it the frame's window lists the monitors (`Frame::winit_window`,
  `available_monitors`), so the window is created at the place and the first frame checks it.
  The built-window checks are done (q3's, q4's and q5's `manual:` lines).
  - [x] q1 the record, the opening and the guard (`.tmp/0_0_1q/brief_q1.md`): `window_main`
    as `[x, y, width, height, scale]` (pixels, points at zoom 1, the monitor's scale), read
    each normal frame and written at exit, the window created at it and the first frame
    resending the position in its own points, `opening(place, monitors)` with the title-bar
    check → verify: ST-WIN-04's headless clauses; the special class's clause waits for the
    Music player. Done: `WindowPlace` and `window_main` in `[common]`, `place_of` (skips
    maximized, fullscreen and the zoom-landing frame), `on_a_monitor`, `opening`,
    `native_options(&Opening)`, `StudioApp::opening` and `on_exit` (eframe 0.37 on wgpu
    declares `on_exit(&mut self)`, no `glow` argument); four ST-WIN-04 tests and three unit
    tests. Built-window check for the top lead: move and resize, close, reopen (100% and the
    4K monitor's 200%), and close maximized. Files: `studio_core/src/settings/{common,mod}.rs`,
    `studio_core/src/shell/{mod,window_size}.rs`. SWE-2 round S1: 3 accepted of 12
    (`.tmp/0_0_1q/rulings.md`), among them the zoom-landing guard compared in pixels with half
    a pixel's tolerance (an exact compare skipped every frame at many positions on a 150%
    display), reworked and amended
  - [x] q2 the off-monitor check on the first frame (`.tmp/0_0_1q/brief_q2.md`; round S1's
    S1a-2 found the window's own monitor list, so no `unsafe` reader): `place_or_fit` against
    the window's monitors, a function pointer the tests replace → verify: ST-WIN-04's
    off-monitor and no-list clauses; the off-monitor case by hand (the top lead, on the
    merged code). Done: `window_size::monitors` (`Frame::winit_window`,
    `available_monitors`), `place_or_fit`, the `monitors` field, `StudioApp::opening` with no
    list, the check before the first frame's `match`; the off-monitor test rewritten, the
    on-monitor restart through the check. The `QUESTIONS.md` entry on the `unsafe` reader is
    withdrawn. SWE-2 round S2: 1 accepted of 4 (`.tmp/0_0_1q/rulings.md`: the unseen move
    holds on a desk of one scale only; wording fixed). Files:
    `studio_core/src/shell/{mod,window_size}.rs`, `plans/core/gui.md`, `DECISIONS.md`,
    `QUESTIONS.md`
  - [x] q3 Astra round A1's rework (`.tmp/0_0_1q/brief_0_0_1q_a1.md`; 2 of 3 accepted,
    `.tmp/0_0_1q/rulings.md`): the title bar measured at each monitor's own scale (a place
    recorded on a 200% monitor straddling a 100% primary passed the check once the 200%
    monitor was unplugged, then opened off every monitor), and ST-WIN-04's 200% clause at a
    size a member reaches (2200 x 1400 pixels; 550 x 350 points sat under the minimum); the
    plan says eframe's creation clamp is kept. Done 2026-10-26 (SWE-2). The post-Astra SWE-2
    round S3: 2 of 3 accepted (the clamp's sentence overstated, the bar's height scaling
    unpinned; fixed by the lead), the review loops end. manual: checked 2026-10-27 (with q4's,
    `.tmp/0_0_1q/built/findings.md`): move and resize, close and reopen at the place on the FHD
    at 100% and on the 4K at 200% (there the window is first shown on the FHD for 15 ms, the
    mixed-scale creation offset the plan records); an off-monitor record, simulated by writing
    one off every screen, reopens fitted on the primary. manual: checked 2026-10-27 (the
    maintainer): a place on the FHD, that monitor switched off, reopens fitted on the 4K. Files:
    `studio_core/src/shell/{mod,window_size}.rs`, `plans/core/gui.md`
  - [x] q4 a window closed maximized reopens maximized (P39, accepted 2026-10-27; DECISIONS
    "A window closed maximized reopens maximized"; `plans/core/gui.md` "Window size classes",
    the remembered place's paragraph): the record's sixth number, the window created
    maximized at the normal place, a five-number record read as not maximized, the
    off-monitor case unmaximized before the fit, X11's first-frame maximize as eframe's
    persistence does it → verify: ST-WIN-04's new clauses headless; by hand (the built
    window, with q's checks): close maximized, reopen maximized, restore to the place; the
    spec's settings row for `window_main` (`spec/team_compiler/settings.md`) says six numbers.
    Done 2026-10-27 (Opus, `.tmp/0_0_1q/q4_report.md`): `WindowPlace.maximized`, read from five
    or six numbers and written as six; the flag kept from the last frame not minimized; the
    window created maximized; the first frame maximizes again after its position on every
    platform, not on X11 alone (the sidekick's contradiction: winit restores a maximized
    window to move it; plan and DECISIONS); an off-monitor maximized place unmaximized before
    the fit. Files: `studio_core/src/settings/common.rs`, `shell/{mod.rs,window_size/}`,
    `spec/team_compiler/settings.md`. manual: checked 2026-10-27 with q's (release build at
    `0c1d7c3f`, a portable copy, the app's own window traced every 2 ms;
    `.tmp/0_0_1q/built/findings.md`): close maximized, reopen maximized, restore to the place,
    on the FHD and on the 4K at 200%; an off-monitor maximized record reopens unmaximized and
    fitted. How visible: two blips per maximized reopen, each under one 60 Hz frame (the
    creation shows a maximized window for 7-13 ms before eframe hides it; the first show
    restores for 2-6 ms before maximizing again). The maintainer sees a flicker, faint
    (2026-10-27): q5
  - [x] q5 the maximized reopen without the flicker. The second blip is ours: the first frame
    resends the position (the mixed-scale creation offset's fix), winit's
    `set_outer_position` clears the maximized flag and restores the window to move it, and
    the next command maximizes it again, a restore and a maximize on a shown window
    (`SW_MAXIMIZE`, which Windows may animate; the 2 ms trace sees the rectangle, not the
    animation). Creating the window unmaximized removes only the creation blip and adds a
    maximize of a shown window. To try first: when the window is created maximized, no
    position resend on the first frame → verify: the built window traced as in
    `.tmp/0_0_1q/built/findings.md`, maximized reopen on the FHD and on the 4K, then
    restore to the place; the maintainer's eye. Done 2026-10-27 (Opus,
    `.tmp/0_0_1q/q5_report.md`; plan "Window size classes", ST-WIN-04, DECISIONS "A maximized
    reopen resends its position only where the creation can have landed wrong"):
    `Monitor.primary`, `window_size::resends_position` (a maximized place skips the resend
    when its scale is the primary monitor's and the frame's), four new tests, each condition
    seen failing under its perturbation. manual: checked 2026-10-27 (release build, the
    trace): the FHD's maximized reopen shows no restore blip, only the creation's 7 ms; the
    4K's keeps the resend (11 ms, a mixed-scale desk) and maximizes there; both restore to
    the place. manual: checked 2026-10-27 (the maintainer): no flicker any more. Files:
    `studio_core/src/shell/{mod.rs,window_size/{mod,tests}.rs}`
- [x] 0.0.1r a folder per PES version (P25 with the maintainer's override rows, 2026-10-26;
  DECISIONS "A folder per PES version"; `plans/core/gui.md` "Settings menu", the "PES folder"
  bullet): the main row's `**` on a pick ending in the selected year, the A folder per version
  toggle and its rows, `pes_folder` resolving an override first, Fix PES path writing the
  running version's row → verify: ST-SET-04; `pes_folder`'s unit tests (override, empty
  fallback, the pattern); the bundled `settings.toml` reading unchanged (no such file ships:
  read as a settings file without overrides saving byte for byte). Done
  (`.tmp/0_0_1r/brief_r1.md`): `CommonSettings::pes_folders` (`[common.pes_folders]`, written
  with string keys since `toml::Value` refuses integer ones, read as `pes_version` is),
  `pes_folder` resolving a filled entry first, `pes_folder_pattern` (the one rule of Browse
  and Fix PES path), `fix_pes_folder`, `path_row_commit` telling a pick from a typed commit,
  the toggle and seven rows (emptying the last row leaves the toggle on, DECISIONS
  2026-10-26); `shell/settings_menu.rs` split into `settings_menu/{mod,tests}.rs` (past a
  thousand lines). Three ST-SET-04 tests, seven unit tests. SWE-2 round S1: 2 accepted of 8
  (`.tmp/0_0_1r/rulings.md`: a hand-written empty entry dropped at load, so the toggle and
  Fix PES path count overrides as the resolution does; the typed-commit test's folder
  holding the year). Browse in the built window, checked 2026-10-28 at `825f1885` (the
  portable copy at PES 2019, its settings restored by hash after; folders under
  `4cc-studio-temp/browse_r/`; `.tmp/0_0_1z/shots/r_02_year.png`, `r_03_plain.png`): a pick
  of `…\Pro Evolution Soccer 2019` stored `…\Pro Evolution Soccer 20**`, a pick of `…\PES`
  stored as picked. The spec's settings table has the `pes_folders` row since; the Team
  compiler view's raw `pes_folder_path` line waits for its window (CLI-only in 0.0.1, the
  view unreachable).
  Files: `studio_core/src/settings/{common,mod}.rs`, `studio_core/src/shell/{mod.rs,
  settings_menu/}`, `studio_core/src/{status.rs,widgets/{mod,settings_row}.rs}`,
  `plans/core/gui.md`, `DECISIONS.md`
  - [x] r2 the path edits wide enough for a usual install path (the maintainer's look at the
    rows, 2026-10-27; `plans/core/gui.md` "Settings menu", the path row bullet; DECISIONS "A
    settings path box shows a usual install path whole"): the menu 660 points, the edits
    holding Steam's PES 2019 path (71 characters) whole at the body font → verify: a test
    measuring that path's galley against the drawn edit's width, every section's path rows
    included. Done (`.tmp/0_0_1r2/brief_r2.md`): the path measures 456 points at egui's
    13-point body font, past the draft's 440, so the edit is 472 points (8-point margins)
    and the menu 660, which the common grid's widest row needs 656 of (DECISIONS "The
    settings path edits are 472 points in egui, the menu 660"). The test draws the PES
    folder, PES 2019 folder, exports folder and a stub tool's file row (the Team compiler's
    section draws no path row yet; the stub's section is the Save editor's, two rows) and
    checks each text run against its edit. The Save editor's section widens the menu to 708
    points while open ("Aesthetics from exports" under the header's indent). The draft's
    440-point box stays (2026-10-28, `.tmp/0_0_1z/settings_box.py`: at 472 its 660-point
    `.settings-body` wraps both Browse buttons under their boxes), as does its fixed body,
    which cannot show the Save editor's widening. Two older menu tests rest the pointer
    inside the menu first: on the test's 800-point screen the sidebar's overlay covered the
    wider menu's left edge (the app lays out at least 1280 points wide). SWE-2 round S1: 2
    accepted of 5 (`.tmp/0_0_1r2/rulings.md`). Files:
    `studio_core/src/{widgets/settings_row.rs,shell/{settings_menu/,test_harness.rs}}`,
    `plans/core/gui.md`, `DECISIONS.md`
- [x] 0.0.1s the Save editor's main panel after 4ccEditor's layout, no tabs (the maintainer,
  2026-10-26; prototyped first): General info with the playable positions under it, skills
  in two columns with the COM styles and the four non-ability values stacked under them, one
  ability column, Appearance a collapsed section below, the Make buttons in two columns under
  the list (AATF to the Panels menu), a team row before each team's players showing the
  team's fields and Tactics, kit slots behind a button
  - [x] s1 the draft (`.tmp/drafts/brief_drafts_main_panel.md`), then the maintainer's look.
    The maintainer's answers (2026-10-26, for s2's plan text): before the 0.0.1 cut ("the
    program needs to be as usable as possible before it's unveiled"; DBC7Editor stays the
    managers' fallback); the team row one click away is fine; the positions grid kept, its
    ratings coloured apart (A green, B the selection's current blue, C the quiet default); Set stats keeps its field
    inline; Skills, then COM styles, then the four non-ability values stacked, each as wide
    as the skills' two columns (rework r1, `brief_drafts_main_panel_r1.md`); the version
    banner replaced by a Warning notice in the status bar, `Switch to PES 20xx` its action and
    its dismiss the close (changes SE-GUI-28 and l3's item 3 once approved); no view draws
    its own title, the window's idle title becoming `<tool> - <app>` (`core/gui.md` "Window
    title", `shell/title.rs`, once approved); the bench list showing players only (12 for a
    4cc team), not the 21 `bench_order` entries with 9 `(empty)` (team_widgets
    `pitch/lineup.rs` draws them all too, once approved). Draft done 2026-10-26 (SWE-2, one
    rework; `.tmp/drafts/main_panel_check.py` 38 checks pass at 1280 x 800, no scroll):
    The maintainer's look (2026-10-26): C drawn chosen in a grey square; General info laid
    out as 4ccEditor's (Name above full width, ID | Shirt halves, label-left pairs, combos
    to the right edge), filling its column; the skills' and COM styles' second columns
    aligned; wider gaps, the columns spread over the view; the status bar's notice shown
    whole when it fits (its middle part takes what the left and right leave, not a third:
    `core/gui.md` "Status bar" and `shell/status_bar.rs` change with s2/s3); the version
    notice follows the selector while shown (gone on the save's version, its text updated on
    another, not back once dismissed). Rework r2 by the lead (the maintainer, 2026-10-26: the
    lead edits UI drafts itself; the sidekick's partial r2 was stopped): the four values as
    an Other stats panel under General info ("Weak foot: Usage" and "Accuracy" sharing one
    label), both panels' boxes on their halves' right edges, the columns 36 points apart,
    the host's `pesVersionChanged` view hook and `noticeShown` for the notice. 4ccEditor's
    per-player "Edited/created player" checkbox is not kept (the maintainer, 2026-10-26: a
    name or shirt name edit sets the flag, and the Team menu's Set edited flag covers a
    team): s2 says so in the plan's 1:1 list, with its DECISIONS entry. The maintainer's last
    look (2026-10-26): the ID box goes, the id becoming the title's weak prefix ("70101 QUADS
    ONLY"), Shirt then the panel's width; Name and Shirt labels carry an "nn/NN" count (the
    name's bytes against the save's field, the shirt name's letters against 18, PES 16's 16),
    following the field as typed and tinted past what every version holds, as the field is
    (lead, 63 checks pass). The maintainer then had s2 and s3 go ahead, a few small tweaks still
    to come
  - [x] s2 `plans/save_editor.md` "Editing UI" rewritten from the approved draft, DECISIONS,
    the SE-GUI scenarios naming tabs rewritten. Done 2026-10-26 (lead): "Editing UI" (the team
    row and team view, the player view's three columns, the counts, the version notice), every
    other section's tab wording, SE-GUI-01/03/06/07/10/11/12/13/22/25/28/31/32/35 reworded,
    SE-GUI-37 (the team row) and 38 (the counts) added; `core/gui.md` "Status bar" (the middle
    takes what the sides leave) and "Window title" (the idle title names the tool), ST-BAR-03
    and ST-TTL-01 to 03; four DECISIONS entries. Review: SWE-2 S1-S4 and Astra A1-A2, 32
    concerns accepted, the loops ended (`.tmp/lead/s2_rulings.md`)
  - [x] s3 the code, in slices (the view's frame and the team row; the player view's
    columns; the team view), before 0.0.1o, whose marks then land on the final layout
    - [x] s3a the shell (SWE-2): the status bar's left and right slots at their content's
      width, the right capped at a third, the notice centred in what they leave
      (`shell/status_bar.rs`, ST-BAR-03); the idle title `<tool> - <app>` (`shell/title.rs`,
      ST-TTL-01 to 03); the Ruleset editor's start page draws no heading of its name
    - [x] s3b no tabs: the selection a team or a player, the team row, the team view (Kit
      slots… a dialog), the quick actions under the list, drag under ALL within a team, the
      bench players only (`.tmp/0_0_1s/brief_0_0_1s3b.md`; SWE-2, the lead fixed four tests:
      `scroll_into_view` waits for egui's spread wheel delta to settle). `session/{mod,
      selection}.rs`, `view/{mod,player_list,quick_actions,team,menus,info_strip,abilities,
      aatf_check}.rs`, `view/appearance/mod.rs`, the tests, `help/02-08`;
      `team_widgets/src/pitch/lineup.rs`
    - [x] s3c the player view's three columns, the `<id> <name>` line, the counts; the
      review's code follow-ups: both AATF surfaces check the selected row's team, else the
      selector's (`aatf_check::current_team`, the filter lists other teams' rows), and the
      Team menu and Ctrl+F act on a team row's team too (Set boots and gloves prefills from
      the team's first filled slot); `selection::rows` is `list`'s player rows. Done
      2026-10-26 (SWE-2, `.tmp/0_0_1s/brief_0_0_1s3c.md`): `view/{info_strip,abilities,mod,
      menus,aatf_check,aatf_panel,live_aatf}.rs`, `view/appearance/fields.rs`,
      `session/selection.rs`, the tests, `help/02_window.md`, `help/03_abilities.md`
    - [x] s3d the review's tests and fixes: the mutation runs' survivors
      (`.tmp/0_0_1s/missed_s3ab.txt`, `missed_s3c.txt`), the team row's painted layout and
      faint fill, the status bar's right slot cap and middle bounds, the chosen ratings'
      colours (C's letter now the normal text colour, not the selection's); the card columns
      and the Cards left badge moved into `team_widgets::cards` (`card_pickers` gone); round
      S1's accepted findings (`.tmp/0_0_1s/s3_rulings.md`); the quick actions' cells now the
      panel's halves (the widest tier button clipped past the panel). Done 2026-10-26
      (SWE-2, `.tmp/0_0_1s/brief_0_0_1s3d.md`): `view/{player_list,abilities,team,menus,
      quick_actions,mod,tactics,terms}.rs`, comment fixes, the tests,
      `team_widgets/src/cards.rs`, `studio_core/src/shell/status_bar.rs`. Rework r1-r3
      (SWE-2 round S2's 5 accepted, `.tmp/0_0_1s/s3_rulings.md`, and the built window): a
      player row's button spans the row (the harness's AccessKit bounds had hidden its
      text-wide fill); the quick actions span the list's width, each column its widest cell's
      when the two fit, else halves (DECISIONS 2026-10-26); "the Appearance tab" left the
      member tooltip and the comments; `view/{player_list,quick_actions}.rs`, `lib.rs`, the
      tests
    - [x] s3e the built window at 1280 x 800 against the draft, the team view's tactics
      bottom measured again. manual: checked 2026-10-26 (lead, the real PES 2021 save's copy,
      `.tmp/0_0_1s/look_s3e_team.png`, `look_s3e2_start.png`): the player and team views as
      the draft lays them out; the tactics settings end at about y 790, inside the pane; the
      first run's two defects (quick actions elided, row fill text-wide) fixed in s3d's
      rework and seen fixed
    - [x] s3f SWE-2 round S3's follow-ups (1 accepted, which ends the SWE-2 loop): the
      elided tier button's tooltip tested; the quick actions' gap one const, no sentinel
      width; the test names still saying "tab" renamed. Done 2026-10-26 (SWE-2,
      `.tmp/0_0_1s/brief_0_0_1s3f.md`): `view/quick_actions.rs`, the tests
    - [x] s3g Astra round A1's accepted findings (`.tmp/0_0_1s/s3_rulings.md`, round A1).
      Done 2026-10-26 (SWE-2, one rework): the card panels narrow to one column on a pane
      under `cards::TWO_COLUMN_WIDTH` (the Skills column sized from the same constant); the
      quick actions' load error one elided line across the list, its text on hover (the
      sidekick's contradiction: a ~160-character error wrapped at the half still took 175
      points; DECISIONS); the help's Current team and Team menu follow the selected row; the
      seven test findings and the `column_widths` survivor's unit test. Lead fix: the hover
      test counted the Cards left badge's copy of the error as the tooltip (passed with the
      tooltip off); it now counts against the resting frame. Files: `team_widgets/src/cards.rs`,
      `save_editor/src/view/{abilities,quick_actions}.rs`, `help/{02_window,05_team}.md`, the
      tests
    - [x] s3h a rules error is a red line whose click opens the error (the maintainer's look at
      the draft, 2026-10-26; DECISIONS "A rules load error is a red line whose click opens the
      error"): the quick actions' line and its dialog, no Cards left badge on a load error, the
      tier check's failure in the same shape (`.tmp/0_0_1s/brief_0_0_1s3h.md`). Done
      2026-10-26 (SWE-2): both lines through the view's refusal dialog (`skills_panel` takes
      `&mut Option<Refusal>`); the sidekick's contradiction: egui wraps the message to three
      lines at 240 points (the draft two), plan and test follow the measure. Files:
      `view/{quick_actions,abilities,mod}.rs`, the tests. Owed from
      s3g's `mutants-diff 7e3bc047` (2 missed, both `cards::TWO_COLUMN_WIDTH`'s arithmetic:
      the narrow-pane test sizes its window from the constant, so it moves with it): a test
      pinning the width at the draft's 330 (two 158-point columns 14 apart, `.skcols`), with
      S4's rework unless m2 removes the one-column fallback first
    - [x] s3i the maintainer's look at the player view (2026-10-26; drafted first,
      `.tmp/0_0_1s/draft_look3_player.png`): field labels in the normal text colour, not the
      weak one (General info, Other stats, Ability stats, the team view's Team and Short); the
      counts in brackets (`Name (28/46)`); "Number" relabelled "Shirt number"; the second
      half's boxes just after a label column as wide as "Shirt number"; Playable positions in
      four centred rows, LWF CF SS RWF / LMF AMF CMF DMF RMF / LB CB RB / GK, each label
      centred over its B (`plans/save_editor.md` "Editing UI", the player view and Name and
      Shirt bullets; DECISIONS "The player view's labels, counts and positions after the
      maintainer's look"). Done 2026-10-26 (SWE-2): `pair_half` takes a `Half`, the second
      half's box after the measured "Shirt number" column and 9 (the sidekick took the draft's
      9 over the brief's 6, the draft having moved mid-run); the positions' rows centred by
      measured `add_space`; `TWO_COLUMN_WIDTH` pinned at a literal 330/329 (s3g's two
      survivors, checked dead by hand). Files: `view/{info_strip,abilities,team,terms}.rs`,
      `help/{02_window,03_abilities}.md`, `team_widgets/src/cards.rs`, the tests.
      `mutants-diff be235ad6`: 39 caught, 1 missed, 1 unviable; the miss
      (`RATINGS_WIDTH`'s `2.0 * RATING_GAP` to `+`) is equivalent, the gap being 2
    - [x] s3j the maintainer's second look (2026-10-26, drafted first,
      `.tmp/0_0_1s/draft_look4_player.png`): Name and Shirt as label-left rows sharing the
      combos' label column (as wide as "Registered position"), those six fields ending flush
      with the second half's boxes; Playable positions' midfield in two rows, LMF AMF RMF /
      CMF DMF; and the third look: the pairs' halves 28 apart, the first keeping 168 points;
      the player view centred between the list and the window's right edge (32 points each
      side in the draft); and the fourth: a label and its field at least 9 points apart on a
      pair, 12 on a label-left row and in Ability stats (26 points either side then)
      (`plans/save_editor.md` "Editing UI", the player view bullet). Done 2026-10-26 (SWE-2):
      the rows' label column measured from "Registered position", the flush edge computed once
      (349.34 points at egui's fonts, inside the 350); the first half is that column, 12 and a
      box, so its boxes start at the fields' x; the view centred by a pad from the columns'
      last-frame widths in egui memory, each column also allocated at that width (the wrap's
      fit check reads the asked width, the cursor the used one: unequal, Ability stats
      wrapped); the registered outline at the widgets' radius. Lead: red runs for the five
      rows, the centring and the radius (the sidekick ran two of the five red); the widths kept
      with `get_temp` as the workspace does, not `get_persisted`. The centring balances against
      the pane's frame, 8 points inside the window's edge: the built-window check decides
      which edge the eye reads. Files: `view/{info_strip,abilities,mod}.rs`, the tests
      - [x] r1 round S5's findings (`.tmp/0_0_1s/s3_rulings.md`, round S5, 14 accepted of 21)
        and the built window's centring (27 px after the list against 35 after Ability stats:
        the pane frame's 8 are invisible). Done 2026-10-26 (SWE-2): the gap after the list is
        half of what the list's edge and the window's leave beside the columns; the list not
        resizable; one `label_width`; the help's load-error sentence; the tests' dialog-before-
        click, not-elided, both-runs and every-label checks; the halved-columns test reached by
        a long tier label now the list cannot narrow. `mutants-diff 714de4e6` over s3j: 62
        caught, 4 missed, all `half_width`'s second-half arm, never observed (its removal
        queued with round S6's rework). Built window rechecked at 1280 x 800
        (`.tmp/0_0_1s/s3j_r1_open.png`): 31 px after the list, 31 after Ability stats
    - [x] s3k the last findings: SWE-2 round S6 (1 of 1 accepted: the halved-columns test's
      tooltip read the official ruleset's button) and Astra A2 (3 of 5: the list's 240 never
      asserted, a stale tier error after the rules file stops loading untested, the counted
      labels' hover untested), and `half_width`'s dead second arm removed (the second half
      lays out at its content's width). Done 2026-10-26 (SWE-2); rulings
      `.tmp/0_0_1s/s3_rulings.md`. The post-Astra SWE-2 round S7: 0 of 2 accepted, the
      review loops end. The step's mutation run: o2a's `mutants-diff dc02a76e` (survivors
      `31e589a5`, `55758277`), then `mutants-diff 3d39aff3` (s5's checkpoint)
  - [x] s4 the player view's last look (the maintainer, 2026-10-26; drafted and approved,
    `.tmp/0_0_1s/player_view_request.md`; plan "Editing UI", "Full Player Customization
    (FPC)", DECISIONS "The player view's last look"): Stronger foot and hand Left | Right
    toggles in the positions' grey, Registered position's full names, `Cards left for`, Motion
    in two columns, the FPC wording (help topics 04 and 06 and `view/tests/player.rs` quote
    the old text), the quick actions' columns and 20-point height, Captain centred, the view
    filling its pane → verify: SE-GUI-11, SE-GUI-31, the layout tests' new numbers. Done
    2026-10-27 (SWE-2): `info_strip::side_row` (`Button::selectable`, the held side
    `abilities::RATING_C_FILL`), `POSITIONS_FULL` and `position_label` `<abbr> - <full name>`
    (the changes table's rows follow, the values as the view shows them), `team_widgets`'
    `cards_left` (`Cards left for` / `Cards left:`, the tier-error and pending paths too),
    the quick actions in two `.qacol` columns (`is_medal`, `column_widths`), Motion two pairs
    a row, the FPC wording with help 04 and 06, Captain centred in its box, the list panel's
    bottom margin 0 so the quick actions reach the pane's bottom (it was 2 short: the brief's
    "already done?" premise was wrong). Files: `save_editor/src/view/{info_strip,abilities,
    quick_actions,mod,terms}.rs`, `view/appearance/mod.rs`, `session/player_ops.rs`,
    `view/tests/{abilities,appearance,player,quick_actions,live_marks}.rs`,
    `help/0{4,6}_*.md`, `team_widgets/src/cards.rs`
  - [x] s4t the window title's path (`core/gui.md` "Window title", `StudioTool::title_detail`),
    `studio_core`'s: after side-lead S's chain releases it; with it, `shell/status_bar.rs`'s
    test samples take the version notice's wording "so the editor will keep treating it as"
    (the maintainer, 2026-10-27; draft and CONTRIBUTING already do); and the fixed-width side
    panels' edges neutral (the maintainer on the built binary, 2026-10-27: the rail's right
    edge showed the resize cursor though a drag did nothing, the draft's is plain): egui's
    `Panel` defaults to `resizable(true)`, which `exact_size` does not turn off, so the shell's
    `sidebar`, the help window's `help-tree` and the Ruleset editor's `ruleset_rail` each take
    `.resizable(false)`, with a test that hovering each edge leaves the cursor at the default.
    Done 2026-10-27 (side-lead S, SWE-2): `StudioTool::title_detail` (default `None`),
    `WindowTitle::frame`'s idle branch `<tool> [<detail>] - <app>`, the busy and notice
    titles unchanged (ST-TTL-01 widened with the clause); the Save editor's `title_detail` its
    `SaveView::save_path`, the toolbar `• unsaved changes` alone, weak, while dirty; the three
    status bar samples reworded (the 1280-point notice still drawn whole); the three panels
    `.resizable(false)` (red: `ResizeEast` on each edge), the rule in `spec/core/gui.md`
    "Shell layout". Files: `studio_core/src/{tool.rs,shell/{title,mod,test_harness,
    status_bar,help_window}.rs,shell/sidebar/tests.rs}`, `save_editor/src/{lib.rs,view/mod.rs,
    view/tests/shell.rs}`, `save_editor/help/02_window.md` (the path's new place),
    `ruleset_editor/src/view/{mod.rs,tests/problems.rs}`. SWE-2 round S1: 1 of 6 accepted,
    the loop ends; rulings `.tmp/0_0_1s4t/rulings.md`. Astra A1 (with v2): 0 of s4t's.
    Built window checked 2026-10-27 (release at `bf8190fb`, portable, `.tmp/0_0_1s4t_v2/`):
    the title reads `Save editor - Unlimited Tool Works` with no save and `Save editor
    [<path>] - Unlimited Tool Works` with one; the toolbar draws no path; the arrow cursor all
    along the sidebar's edge (client x 55-57), the player list's (302-306) and the help tree's
    (225-232), the probe reading the I-beam over text fields to show it tells cursors apart.
    The Ruleset rail is not in the preview build (the Save editor alone is registered): its
    headless test stands for it
  - [x] s5 the Tactics editor's grid (the maintainer, 2026-10-26; drafted and approved,
    `.tmp/0_0_1s/tactics_layout_request.md`; plan "Tactics editor", DECISIONS "The Tactics
    editor is one grid"): the three-row grid, Formation settings in pairs of equal rows, the
    numbers' three-choice toggles, Join the attack's three rows, Advanced instructions' grid,
    Automatic settings' grid and Substitution combo (Very early on PES 17 and 21: the
    maintainer's look at PES 17's screen, 2026-10-27), Copy/swap on the selection, the medal
    initials on the badges → verify: SE-GUI-12, SE-GUI-25; then o2b on it. With s6 and s7 and
    the popover and pitch-picked targets (DECISIONS "The slot popover's anchor, a target picked
    on the pitch, "No target""), in slices, each its own commit and review:
    - [x] s5a the grid's frame (`view/tactics.rs`): 414 | 320 | the rest, three rows; the
      preset bar (Preset, Fluid formation, Copy/swap's four entries on the selected preset)
      level with the formation bar over columns 2-3 (the formations out of `pitch::pitch`,
      Copy/swap's four, s6's Scroll names in the tool's settings); the lists under columns 2-3,
      halves, twelve 18-point rows, no titles; the narrow fallback gone; the panels in their
      cells as they are. Done 2026-10-27 (SWE-2, one rework): one `pub fn` per panel, each in
      a group frame (`tactics_settings` now a test helper); `pitch::formation_selector`,
      `PITCH_BOX` (the canvas and its 1-point border); `TacticsView::scroll_names`, off holding
      a wide name at its badge's left; the bars one `interact_size.y` line each; the armed
      note gone. Measured at 1280 x 800: Player assignments 196 points wide; the formation
      bar's Scroll names ends 44 points inside its span; the lists' bottom 4 points past the
      pane's, so the team view scrolls by 4 (s5b's panel heights decide whether that stays).
      The rework's catch: the Scroll names test ran at 1100 wide, where the bar overflows the
      pane's clip and a press misses the checkbox; the app scales such a window instead.
      Files: `team_widgets/src/{pitch/{mod,lineup,popover}.rs,tactics/{mod,assignments,
      instructions}.rs,test_support.rs}` and their tests, `save_editor/src/{lib,settings}.rs`,
      `view/{mod,tactics}.rs`, the view tests, `help/08_tactics.md`
    - [x] s5b Formation settings (pairs of equal rows, label over a 24-point box, two-choice
      toggles the half's width, Few | Medium | Many, "Frontline Press.", "All Out Def.") and
      Player assignments (label over chip, Join the attack three rows, one picker), both as
      tall as the pitch. Done 2026-10-27 (SWE-2, one run): `formation_settings` six rows of
      `TAC_OFFENSIVE` | `TAC_DEFENSIVE` cells sharing the panel's height, `number_choices` (a
      stored value outside 1-3 selects none, writes nothing), `style_short`; the choices'
      padding zero, not the draft's 5: egui's text is wider than the browser's, and
      "Frontline Press." (91.5 points) would elide in its 95.5-point choice with it (a centred
      text that fits looks the same); `player_assignments` label-over-chip rows, three join
      chips armed together; the host's row-2 cells end at the pitch's bottom. Player
      assignments' last chip ends 50 points above its frame's bottom. Files:
      `team_widgets/src/{tactics/{mod,assignments}.rs,test_support.rs}` and their tests,
      `pitch/tests/popover.rs`, `save_editor/src/view/tactics.rs`, the view tests,
      `help/08_tactics.md`
    - [x] s5c Advanced instructions' grid (s7's sides through `pes_savefile`'s `Instruction`,
      each target a pitch pick) over Automatic settings' grid (the Substitution combo). Done
      2026-10-27 (SWE-2, three sessions: the first ran out of turns twice mid-debug, so the
      rest went in two small briefs, r2a and r2b, to a fresh session): `Instruction::side`
      (the schema's `InstructionSide`); each combo its side's list less Tight Marking, a held
      other-side value appended; the `.tacinstr` grid, headers left-aligned in their columns as
      the draft's are; `Assign::Target`, written only while the instruction takes a player;
      the target chips (`No target`, `An opponent`), `target_text`'s "No target" in the
      changes table too; `.tacauto`'s two columns (each cell claims its column:
      `allocate_ui_with_layout` advances by the used width), its frame at the lists' bottom
      (the lists drawn first), the rows centred under the heading; the Substitution combo on
      every version (PES 15 and 16 store it too). Column 1 ends at the lists' bottom; the built
      window at 1280 x 800 still cuts the grid's last row by about 5 points (the report's "3.5
      inside" counted one panel margin of the two), which s5e's header row recovers. Files:
      `pes_savefile/src/model/instruction.rs`, `team_widgets/src/{pitch/mod.rs,tactics/{mod,
      assignments,instructions}.rs,test_support.rs}` and their tests, `save_editor/src/view/
      tactics.rs`, the view tests, `help/08_tactics.md`
    - [x] s5d the pitch: the popover's anchor, outside press and `player <n>`; s6's swap mark;
      the badges' medal initials. Done 2026-10-27 (SWE-2, one session): the popover's top-right
      on the marker's bottom-left as it stood at the click (`OpenSlot::at`), its header
      `player <n>` (`(empty roster slot)` for an empty one); a primary press closes it unless
      it lands on a Foreground layer (the popover or a popup it opened), the focused field
      then surrendered so a typed edit's undo entry still seals; `swap_target` is the drop's
      and the mark's one pick (the topmost other marker, the dragged one's own spot never
      it), the mark `#14fbff` brackets over everything and the dragged marker at 35%;
      `Medal` (`from_tier_id`) on `RosterRow`, its letter at the badge's right end, the
      Save editor's cache keyed by revision, team and rules file. Lead: the popover's name
      lookup shares `raw_name` with the markers', the medal test pins the fixture team's
      letters as literals. Files: `team_widgets/src/{pitch/{mod,popover}.rs,tactics/{mod,
      assignments}.rs,test_support.rs}` and their tests, `save_editor/src/view/{mod,tactics,
      prompt,terms}.rs`, the view tests, `help/08_tactics.md`
    - [x] s5e the list and team view: s6's three-character short name, s7's picked team
      selecting its row and `[click for tactics]`; the team view's header as the draft has it
      (DECISIONS "The team view's header on one row, its title in strong body text": the title
      in strong body text, the ids and Kit slots… on the fields' row, `Col. 1`/`Col. 2`, the
      name box 140) → verify: at the 1280 x 800 default pane the team view's last row ends
      inside the pane's content (its two panel margins counted), so nothing scrolls; the built
      window shows the lists' and Automatic settings' bottom strokes. Done 2026-10-27 (SWE-2,
      one session): the header one `horizontal_wrapped` row of seven units, each claiming its
      measured width (`TEAM_ROW`), so a narrow view wraps whole units; the lowest stroke at
      740.5 against the content's 760 (19.5 to spare; the old header's 764.5 was the built
      window's overflow); the short box 40 wide with `char_limit(3)`, a stored longer name
      kept until edited; a team picked in the selector selects its row (ALL keeps it); the
      hint elided before the name (the brief had it inverted; the sidekick followed the plan).
      Lead: the units an enum (`HeaderUnit`) with the manager and stadium halves their own
      functions, in place of index arithmetic. The built window: the release build at `f1ca2342`, `.tmp/0_0_1z/shots/09_team.png`, the lists' and
      Automatic settings' bottom strokes inside the pane, the three-character short name and
      `[click for tactics]` (lead, 2026-10-28). Files:
      `save_editor/src/view/{team,player_list,mod,terms}.rs`, `view/tests/{team,list}.rs`,
      `help/{02_window,05_team}.md`
    - [x] s5's review, SWE-2 round S1 (ten parts, `.tmp/0_0_1s/tac_rulings.md`): 29 accepted,
      reworked 2026-10-27 (SWE-2, one session, `brief_0_0_1s5_r1.md`): any button's press closes
      the popover, the keeper's release is his click, the popover anchored to the canvas (it
      scrolls with the pitch), out-of-range X/Y kept on draw, the released marker painted at
      its drop on the release frame, the target chip and the changes rows worded as the
      controls (name alone, the numbers' and Substitution's labels), Scroll names kept across
      an open, `player_assignments` read-only; tests on non-identity elevens, off-grid drops,
      the five copy/swap ops undone in turn. Lead: the never-written "player assignments" undo
      key dropped (`view/tactics.rs`)
    - [x] round S2 over the rework (SWE-2, four parts): 5 accepted (a release within egui's
      click distance both clicks and drops a marker; four test gaps), reworked with 0.0.1x4 +
      y2: a release egui reports as the marker's click is never a drop (the sidekick measured
      the double fire's band: past egui's 5-point hit radius, within its 6-point click
      distance; tests at a 5.75 and a 10-point move), a second grass drop below the half-mark,
      the settings' first row pinned under the heading, the eleven's non-identity premises, the
      popover test reading the popover's own area
    - [x] round S3 over the S2 rework, x4 and y2 (SWE-2): 4 accepted, ending the SWE-2 loop;
      reworked 2026-10-27 (Opus, `brief_tac_s3_rework.md`): the XI-row swap tests drop row 1 on
      row 4 (a move would pass on adjacent rows), a marker swap on preset 2's In-possession, a
      release within the click distance over a neighbour's box is a click, a marker dropped on
      an XI row exchanges the two (help says so). S3b-1's host WHEN (preset 2 through the
      editor) landed with the views' mutants brief, `31e589a5`. Files: `team_widgets/src/pitch/{lineup.rs,tests/drops.rs}`,
      `save_editor/src/view/tests/tactics.rs`, `help/08_tactics.md`, plan "A pitch player dropped on another"
    - [x] Astra over the whole surface at `4f34855a`, then the post-Astra SWE-2 loop. Round A1
      (two parts: the pitch; the panels and the host): 9 accepted (a near-miss release read
      as a drop on a marker, the placeholder portrait the save's game's rather than the
      selected one's, the help's "without portraits", six tests passing on wrong
      implementations: the shown preset in the popover, panels and host, the swap mark's
      paint order, a name's bound, the armed chip, the side lists); reworked 2026-10-27 (Opus,
      `brief_tac_a1_rework.md`): a marker is the drop target only when the release is inside
      his box (egui reports it to the closest widget within 5 points), the placeholder follows
      the game `Portraits::select` was last given, the help's blank portrait, the six tests
      made to fail on the wrong code; the brief's off-canvas guard dropped as unreachable
      (markers are clamped inside the canvas). Files: `team_widgets/src/pitch/{mod.rs,tests/
      {drops,paint,popover}.rs}`, `tactics/instructions/tests.rs`, `save_editor/src/view/
      {portraits,tactics}.rs`, `view/tests/tactics.rs`, `help/08_tactics.md`
    - [x] round S4 over the A1 rework (SWE-2, two parts): 4 of 5 accepted, ending the post-Astra
      SWE-2 loop (a lineup row's drop target only its content, where the draft's row spans the
      list; three test premises unchecked); a row's near miss within egui's 5 points rejected
      (no other target there). Rework done 2026-10-27 (Opus, `.tmp/0_0_1s/tac_s4_rework_report.md`):
      a row's horizontal takes the list's whole width, so a release on its band past the name
      swaps as one on the name does (red run seen); the instruction picks and the near-miss
      test's starter now assert their premises (each seen failing under a perturbation).
      Files: `team_widgets/src/pitch/{lineup.rs,tests/drops.rs}`, `tactics/instructions/tests.rs`
    - [x] Astra round A2 over the whole surface (two parts at `9f36a8bc`, Devin work): 7 of 7
      accepted, all tests that pass on a wrong implementation (the popover's writes, Make GK
      and the markers' states on the identity eleven, a substitution's other takers, the Y
      field, the position combo's GK entry, the Few/Medium choices, the chips' holders read
      through their own getter). Rework done 2026-10-27 (Opus,
      `.tmp/0_0_1s/tac_a2_rework_report.md`; test code only, each test seen failing under its
      perturbation): the tests swap entries of the eleven first and compare the whole record;
      the chips' test draws seven differently named holders, since the fixture's takers were
      two players all named "PLACEHOLDER" (the sidekick's contradiction, accepted). Files:
      `team_widgets/src/pitch/{lineup.rs,popover.rs,tests/{paint,popover}.rs}`,
      `tactics/{assignments.rs,tests/mod.rs}`. Then a SWE-2 loop over it and Astra A3, then the
      surface's one mutation run (`just mutants-diff 3d39aff3`) at its loop's end
    - [x] round S5 over the A2 rework (SWE-2): 4 of 4 accepted, ending the post-Astra SWE-2
      loop (three more tests passing on the identity eleven: a substitution's taker keyed by
      the starter's index, the marker's name and label, the fielded cross; the spin field's
      typing copied three times). Rework done 2026-10-27 (Opus,
      `.tmp/0_0_1s/tac_s5_rework_report.md`; test code only, each test seen failing under its
      perturbation): the eleven shuffled with its premise asserted, the name located under
      marker 3's badge, `Headless::type_in_spin_field`. Files: `team_widgets/src/pitch/
      {lineup.rs,popover.rs,tests/{mod,paint,popover}.rs}`. Next: Astra A3
    - [x] Astra round A3 over the whole surface (two parts at `bf2c7eef`, Devin work): 4 of 4
      accepted, under five, so Astra's loop ends (`.tmp/0_0_1s/tac_rulings.md`, round A3: a
      grass drop on the identity eleven, the instruction repoint tested on PES 21 alone where
      PES 17 stores Counter Target as PES 21's Deep Defensive Line, the popover's Close never
      clicked, the medal letters never followed across an edit). Rework done 2026-10-27 (Opus,
      `.tmp/0_0_1s/tac_a3_rework_report.md`; test code only, each test seen failing under its
      perturbation). Files: `team_widgets/src/pitch/tests/{drops,popover}.rs`,
      `save_editor/src/view/tests/tactics.rs`
    - [x] round S6 over the A3 rework (SWE-2): 1 of 3 accepted, ending the post-Astra SWE-2
      loop (the grass drop pinned on Kick-off alone; the host's single-slot check and a
      future table change rejected). Rework done 2026-10-27 (Opus,
      `.tmp/0_0_1s/tac_s6_rework_report.md`): a third drop on preset 2's In possession, the
      whole record compared, seen failing on a grass write fixed to Kick-off. Files:
      `team_widgets/src/pitch/tests/drops.rs`
    - [x] the loop's checkpoint: `gates-changed c244e6e4` green, then `mutants-diff 3d39aff3`
      over ten crates (2026-10-27): 2 missed, both in `studio_core`'s `window_size`, both
      excluded with their reasons in `.cargo/mutants.toml`: `monitors`' primary `==` needs a
      real window (its body's replacement already excluded so), and `resends_position`'s
      `< 0.001` -> `<=` is equivalent for every scale (f32(0.001) an odd multiple of 2^-33, no
      difference of two scales equals it, `.tmp/0_0_1s/tolerance_proof.py`); none in
      `team_widgets` or `save_editor`. No rework brief
  - [x] s6 the maintainer's three requests of 2026-10-27 (drafted and approved, Scroll names
    remembered in the tool's settings; plan "Editing UI", DECISIONS "The short name's three
    characters, the pitch drag's swap mark, Scroll names";
    `docs/prototypes/tools/save_editor.js` `teamFields`, `hitMarker`,
    `drawPitch`, `scrollNames`; renders `.tmp/0_0_1s/draft_tactics_req3_*.png` by
    `.tmp/0_0_1s/draft_swap_scroll.py`): the team view's short name box takes three
    characters; a pitch drag held over another starter marks him with the selection's corner
    brackets, the dragged marker at 35% opacity over him, so a drag meant only to move can be
    steered off; a "Scroll names" checkbox right of the formations' Copy/swap, on by
    default, which holds a wider name still at its badge's left when off → verify: a test per
    request in the built view; the code with s5
  - [x] s7 the maintainer's second batch of 2026-10-27 (drafted and approved; plan "Editing
    UI", DECISIONS "Advanced instructions by side, a picked team's tactics, the team row's
    hint"; `save_editor.js` `DEFENDING`, `versionInstructions`, `instructionsFor`, the team
    selector's callback in `renderMaster`, `teamRow`'s `.thint`; renders
    `.tmp/0_0_1s/draft_list_req4_*.png`, `draft_instr_req5.png`): Advanced instructions'
    attack combos offer the attacking instructions and the defence combos the defending ones
    (the game's split, read on the Rigged Wiki's Tactical pages for PES 2017, 2019 and 2021,
    Konami's PES 2019 manual counting 9 and 6, every version's stored table listing the
    attacking ones first; Anchoring, new in 2020, attacking); a team picked in
    the team selector selects its team row, so the view shows its tactics (ALL keeps the
    selection); each team row reads `[click for tactics]` in the weak grey after the name,
    elided before the name is → verify: a test per request in the built view (the side's
    list per version from `pes_savefile`'s side); the code with s5
- [x] 0.0.1t a rules file a member handles ends in `.4ccrules` (the maintainer, 2026-10-26;
  DECISIONS "A rules file a member handles ends in `.4ccrules`"; `plans/aatf_rules.md` "The
  rules file"): the Save editor's settings row and AATF panel and the Ruleset editor's pickers
  offering `.4ccrules` alone, Save a copy suggesting `autumn26.4ccrules`, the written file's
  first line naming Rhai (`libs/aatf`'s writer header and the nine sources' first lines), the
  help, the spec's examples and `spec/aatf_rules.md` "The rules file" when built; the bundle's
  `rules/dbc7.4ccrules` rides with 0.0.1f → verify: the pickers' extension lists pinned by
  unit test; SE-GUI-15's suggested name; the writer's header test; `grep -rn "\.rhai"` over
  the help, the messages and the spec finding only the repository's own sources
  Done 2026-10-26 (SWE-2): `aatf::RULES_FILE_EXTENSION` the one spelling, the three pickers
  and both suggested names on it (`OFFICIAL_FILE_NAME` = `autumn26.4ccrules`); the header and
  the nine sources' first lines name Rhai, the logic hash untouched; a file loads whatever it
  ends in (no meaningful red). Lead: GLOSSARY's row; the spec's bundle clause left to 0.0.1f.
  Files: `aatf/src/{rules,lib,writer/*}.rs`, `aatf/rulesets/*.rhai`,
  `save_editor/src/{lib,view/aatf_panel,session/rules}.rs`,
  `ruleset_editor/src/{view/mod,cli}.rs`, `ruleset_editor/help/*`, the tests,
  `spec/{aatf_rules,save_editor}.md`. `mutants-diff e04c318d`: 3 caught, 1 unviable, 1 missed:
  `ruleset_dialog` to `Default`, a `#[cfg(not(test))]` native dialog no test compiles, excluded
  by name in `.cargo/mutants.toml`
- [x] 0.0.1u the theme pins egui 0.36's spacing and corner radii (the maintainer, 2026-10-26;
  `plans/core/gui.md` "Other UI components", the "Dark/light theme" row; DECISIONS "The theme
  pins egui 0.36's spacing and corner radii", "The theme pins 0.36's `interact_size` and widget
  colors too"). The drafts' controls measured 2026-10-27 (`.tmp/0_0_1u/measure_controls.py`):
  buttons, combos, number boxes, list rows 18; the quick actions 20 (their own rule); text
  boxes 20 (their border). `theme::install` (replacing `install_fonts` at every call site) sets
  0.36's `button_padding`, `interact_size` and both themes' `Widgets` whole; the quick actions
  get their own 20 (SWE-2, after s5e) → verify: a test reading the installed style's values
  for both themes; every view's layout tests rerun, their moved numbers checked against the
  drafts; the built window's buttons against the draft's. Done 2026-10-27 (SWE-2; `theme.rs`,
  every harness, `quick_actions.rs`, `team.rs`'s Kit slots claim 77 → 69, `player_list.rs`):
  the widget states checked field by field against 0.36.1's source; the player list now
  reserves its scroll bar's 10-point band (DECISIONS "The player list reserves its scroll bar's
  band": at the 4-point padding the bar's hit zone covered the rows' marks; red: three
  `live_marks` tests fail without it); tests that measured with a fresh default context read
  the window's style. Open: the built window's look, with w2's fonts (0.0.1w)
- [ ] 0.0.1v the preview's icon (the maintainer, 2026-10-27: Archer, from
  `C:\Data\Pictures\FSN\Smug\smugredman.png`; `plans/core/development_plan.md` "The icon",
  DECISIONS "The preview's icon and its build dependency"). The maintainer's split:
  the whole picture at 32 and up, a tight square on the head at 24 and 16.
  - [x] v1 the assets (lead): `scripts/make_icon.py` and `resources/icon/{icon.ico,small.png,
    window.png}`; the script reads each ICO size back and asserts it is its own crop (a
    one-image ICO fails it: its 16 is the resized whole picture). Preview
    `.tmp/0_0_1v/icon_sheet2.png`
  - [x] v2 the code (SWE-2, after S's chain releases `studio`): the window icon per platform
    and the build script with `winresource` → verify: each PNG decoded to its `IconData` size
    in a test; the release exe's icon in Explorer and the built window's title bar and
    taskbar (`manual: checked` lines; ask before the window check)
    Done 2026-10-27 (side-lead S, SWE-2): `native_options` sets `small.png` on Windows,
    `window.png` elsewhere, for both openings (`window_size.rs`, two tests: each PNG decodes
    at 64 and 256; both openings carry the platform's icon); `crates/studio/build.rs` compiles
    `icon.ico` with `winresource` 0.1.34 (Windows build hosts only), its version block naming
    Unlimited Tool Works (DECISIONS "The exe's version details carry the codename"). Read back
    from the release exe (`.tmp/0_0_1v2/exe_icon_check.py`): one icon group, the six sizes
    byte-identical to `icon.ico`, the description and product name Unlimited Tool Works; the
    exe before the change had no resources (the script fails on it). eframe 0.37 makes both
    window icons from the one image (`native/app_icon.rs`: `ICON_BIG` at `SM_CXICON`,
    `ICON_SMALL` at `SM_CXSMICON`, Lanczos-resized), so the taskbar shows the tight crop too;
    whether that reads better than the whole picture at the taskbar's size has no headless
    check. Files: `Cargo.toml`, `Cargo.lock`, `crates/studio/{Cargo.toml,build.rs}`,
    `crates/studio_core/src/shell/window_size/{mod,tests}.rs` (split from `window_size.rs`,
    past a thousand lines), `plans/core/{development_plan,README}.md`. Review: SWE-2 S1, 2
    accepted of 7 (the eframe fact above, the split), loop ended (`.tmp/0_0_1v2/rulings.md`).
    Astra A1 (with s4t): 1 accepted, a wrong fact (Wayland ignores a window's own icon; the
    plan's `window.png` bullet now says so). Built window checked 2026-10-27 (release at
    `bf8190fb`): the title bar shows the tight crop; the exe's file description and product
    name read Unlimited Tool Works (Task Manager's name), version 0.0.1. Open, the
    maintainer's eye (outside the app's own windows, which the lead does not capture): the
    taskbar's button and Alt+Tab (the tight crop: if the whole picture should be there, the
    plan's bullet on `small.png` says what that costs); Explorer's file icon on the exe.
    manual: checked 2026-10-30 by the maintainer on the candidate (Windows 10): the taskbar
    button and Alt+Tab (the tight crop), Explorer's file icon on the exe and the blank title
    bar slot "look fine"
- [x] 0.0.1w the drafts and the app draw the same fonts (the maintainer, 2026-10-27: the
  drafts named Ubuntu, installed nowhere, so they were reviewed in Segoe UI; Selawik, its
  metric twin, replaces Ubuntu; `plans/core/gui.md` "Prototypes before implementation", the
  Fonts bullet, and "Other UI components", the Dark/light theme row; DECISIONS "Selawik
  replaces Ubuntu; the drafts load the app's own font files"). The drafts' approval, withdrawn
  until the maintainer's close look at them in these fonts, was given again at w3.
  - [x] w1 the drafts' fonts (lead): the font files in `docs/prototypes/assets/fonts/`
    (licences in its README and the host's comments), every stylesheet and canvas font on the
    app's chain (`--font-body`, `--font-mono`, `FONT_BODY`), every face loaded up front (canvas
    text triggers no load), line height 1.2 with egui's 18-point floor under buttons, combos,
    number boxes, checkbox rows and list rows (all 18 again, re-measured with
    `.tmp/0_0_1u/measure_controls.py`; the mono path box 18.4 left for w1b), no bold (the team row, the pitch's labels), ✕ →
    the icon font's X, the tier grip ⠇ → ≡ (the lineup's handle), the number boxes' spin
    arrows hidden; `scripts/draft_fonts.py` → verified: Selawik, Hack, Phosphor and egui's
    two emoji fonts only, on every page (red: the old body font and a canvas `sans-serif`
    restored by hand, 714 findings); glyphs pixel-identical at line height 1.2 and normal
    (12.5 and 14 points, 1x and 2x); the team view still fits its window
  - [x] w1b the drafts' native controls as egui draws them: the checkbox (13 × 13 in Chromium),
    the sliders (Chromium's and Firefox's own), the combo's popup (the system's list), each
    measured in egui first; and the scrollbars, which a headful browser on Windows 10 draws
    17 wide in the layout where egui's float (the probes run headless and see none); the player list's reserved 10-point band (0.0.1u) drawn in the draft; the drafts' app name, "4cc Studio" in the title bar, About and the data-location dialog, against the preview's "Unlimited Tool Works".
    Done: the sliders (0.0.1z1, egui's own); the app name (2026-10-28, lead: `APP_NAME` in
    `docs/prototypes/index.html` for the title bar, About's title and first line "Unlimited
    Tool Works v0.0.1 pre-alpha", the logo's hover and the first-run prompt;
    `.tmp/0_0_1z/name_check.py`, no page errors); the scrollbars' width in the layout (w6a's
    `--sbw`); the checkbox and radio, the combo's popup and the player list's band (2026-10-28,
    lead: `docs/prototypes/index.html`'s checkbox rules and `comboPopup`, `save_editor.js`'s
    `.rows.scrolls`; each measured in the built window first, `.tmp/0_0_1z/cb_measure.py`,
    `combo_measure.py`, against the drafts by `cb_draft.py`, `combo_draft.py`, `band_check.py`:
    box 14 on grey 60, tick and labels grey 180, popup flush under its button with border 60,
    corner 6, margin 6, rows 18 high 21 apart, the hovered row and the shadow alike, a pick
    firing the select's change, Escape and a press outside closing it; the rows 10 short while
    the list scrolls, full width filtered to two; no page errors, `draft_fonts.py` 0 findings)
  - [x] w1c the symbols as SVGs (lead; the maintainer, 2026-10-27; DECISIONS "The symbols
    Selawik lacks are SVGs in the suite's icon font" and "The symbols are a font of their own,
    on the body font's metrics"). Done: the census (string literals of the crates' code, the
    drafts' sources, egui's own widgets) found 18 symbols Selawik lacks, ↻ ∉ ⏷ beyond the
    list; one SVG each in `resources/symbols/`, its advance and ink box the reviewed glyph's
    (Segoe UI or Segoe UI Symbol, measured), strokes Selawik's 129 units; `scripts/
    make_symbols.py` builds `resources/fonts/symbols.ttf` and the drafts' copy on Selawik's
    units and metrics (Selawik itself committed for it); the drafts' chains Selawik, Symbols,
    Phosphor, egui-icons (mono Hack first), egui's emoji fonts gone, w1's swaps undone, the
    status bar's condition icons and dismiss the icon font's as in the app, the title bar's
    maximize box drawn in CSS, `draft_fonts.py` letting the sample names' ♂ and fullwidth
    letters fall back → verified: `make_symbols_test.py` 6 OK (rebuild byte for byte, each
    SVG's codepoint and advance, the body font's metrics, clockwise outlines; red: the first
    build's counter-clockwise outlines, then a perturbed ascent and advance); two builds
    identical (`ac29b6db…`); `draft_fonts.py` 0 findings (red: "Symbols" out of the body
    chain, 61); the specimen's 18 symbols as wide as the reviewed ones, every UI string
    within Selawik's own difference from Segoe UI; the team view still fits (`draft_fit.py`).
    Files: `resources/symbols/*.svg`, `resources/fonts/{symbols.ttf,Selawik-Regular.ttf,
    Selawik-OFL.txt,README.md}`, `scripts/{make_symbols,make_symbols_test,draft_fonts}.py`,
    `docs/prototypes/{index.html,tools/ruleset_editor.js,tools/save_editor.js,assets/fonts/}`
  - [x] w2 the app (Opus, two runs, `.tmp/0_0_1w/brief_0_0_1w2{,_r1}.md`): Selawik, then the
    symbols font, then the icons in both chains (`theme::install_fonts`), every menu button
    through `studio_core::widgets::menu_button` (its label ending in ` ⏷`, 13 calls), and the
    drafts' text sizes pinned (DECISIONS "The app pins the drafts' text sizes": the app drew
    13 where the drafts draw 12.5, found by w2's first run). Seven layout tests ruled: the
    ruleset start modal's scroll settles when egui asks for no frame; General info's column
    keeps the draft's 350 (the positions grid off its centre); the badge crosses measured by
    advance, as the draft places them, the cross after "Registered position" by ink; the
    rules error's line two rows across the panel, as the draft; Skills 50 after General
    info's fields (the draft's 50); `TEAM_ROW` re-measured (Kit slots 60.3 against the draft's
    64.3: its buttons padded 6, the app's 4; the maintainer kept 4 and the drafts follow,
    DECISIONS "Buttons stay padded 4 × 1; the drafts follow") → verified: every
    symbol has a glyph after `install` in both families (red: 18 missing); the families and
    sizes pinned by tests (red: Small 9); `just gates-changed c96dbde5`; the built window
    (release build of `819baf39`, the real PES 2021 save's copy): Selawik, the menus' ⏷, ▶,
    ✓, ≡ and the pitch drawn from bundled files, the team header on one row (side by side with
    the drafts: w3). Files: `studio_core/src/{theme.rs,widgets/{mod,menu_button}.rs}`,
    `ruleset_editor/src/view/{mod,tests,tests/file,tests/start}.rs`,
    `save_editor/src/view/{menus,tactics,abilities,info_strip,team}.rs` and its tests,
    `team_widgets/src/pitch/tests/paint.rs`
  - [x] w3 the maintainer's close look at the drafts: approved 2026-10-28 (with w4a, w6a; w6c
    was his own requests of the same evening); the built window follows w4b and w6b
  - [x] w4 alternatives to Selawik (the maintainer, 2026-10-27: "not liking Selawik much
    compared to Segoe UI", alternatives kept open): a few bundleable fonts, each scaled
    (`FontTweak::scale`, `size-adjust` in the drafts) to Segoe UI's measured width (Steam's PES
    2019 path, 425.7 points at 12.5), shown side by side for the maintainer to pick; a pick
    rebuilds `symbols.ttf` on its metrics (`scripts/make_symbols.py`'s `BODY_FONT`). Trial
    page made (2026-10-27, the maintainer asked for one): `.tmp/0_0_1w/font_trial/index.html`,
    a font dropdown over the draft's player and team views, the candidates unmodified from
    the Google Fonts repository (`fetch.py`; Open Sans 94.9%, Noto Sans 94.2%, Source Sans 3
    105.7%, Inter 92.7%, Fira Sans 97.1%, Lato 99.2%, measured by `measure.py`) and Segoe UI
    as the reference, through the drafts' `?fontsrc=&scale=` hook (inert without the keys;
    `draft_fonts.py` 0 findings). Seen there: scaled, Open Sans, Noto Sans, Source Sans 3 and
    Fira Sans wrap "Shirt number" (Noto also "Registered position"); Inter and Lato fit as
    Selawik does. Picked (the maintainer, DECISIONS "The body font is Roboto Flex at its
    default axes, unscaled" and "Roboto Flex is cut at optical size 12.5"): Roboto Flex,
    untuned and unscaled
    - [x] w4a the cut and the drafts (lead, 2026-10-28): `.tmp/0_0_1w/cut_flex.py` cuts
      `RobotoFlex-Regular.ttf` and `-Semibold.ttf` (optical size 12.5, weight 400/600,
      reproducible bytes) into `resources/fonts/` and the drafts' assets with the OFL; the
      drafts draw them (chains, `@font-face`, licence comment, fonts README; their Selawik
      copies deleted); the formation name at 20.5 placed 6.5 in (`label_metrics.py`); the
      three widths the maintainer named: General info's label columns 107 → 109
      ("Registered position" 108.4) and 72 → 73 ("Shirt number" 72.9), the Tactics settings
      column 414 → 422 ("Frontline Press." 96.9 with its padding, 99 given since 97 still
      elided at a 2x scale), Player assignments taking the 8 points. Then (the maintainer,
      2026-10-28: the right side had room to spare, the list too little) the player list
      240 → 255, taking the Tactics view's spare strip (the scroll gutter, which the drafts'
      Tactics view drops: it never scrolls, and the app's floating bar reserves none), and
      General info 350 → 344 so the player view keeps its three columns on one row
      (`list_width_probe*.py`: at 255 with 350 they wrap). Checked:
      `elide_check.py` nothing elided or wrapped at scales 1, 1.5 and 2, and red on the old
      widths (`elide_check_red.py`); `draft_fonts.py` all suite text in the bundled files,
      the one fallback a member-typed "♂" in a player name (Selawik lacked it too)
    - [x] w4b the code (brief `.tmp/0_0_1w/brief_w4b.md`; code done 2026-10-28 by SWE-2,
      reviewed: report `.tmp/0_0_1w/report_w4b.md`; the Selawik files deleted; the
      Frontline Press. test takes the draft's 5-point padding, the only one that fails at 414;
      the three-column test pins the columns' lefts, 36 apart, since the fields' edge moves
      with the font; `TEAM_ROW`'s claims re-measured; open: the built window's views at
      1280 x 800 against the drafts, the symbols' placement on Roboto Flex's line among
      them, with the batched checks): `theme.rs`'s body and label
      families and both chains on the Roboto Flex files, the Selawik files and their README
      rows removed, `make_symbols.py`'s `BODY_FONT` and `symbols.ttf` rebuilt (both copies),
      the formation label's `FontId` and offset, the widths' constants (settings 422, list
      255, General info 344) and every test
      that pins a width or a row count re-measured → verify: the gates; the built window's
      player and team views at 1280 x 800 with the same rows as the drafts (asks before it
      takes focus). Built window looked at 2026-10-28 (release at `f1ca2342`,
      `.tmp/0_0_1z/shots/04_opened.png`, `09_team.png`): Roboto Flex throughout, the
      symbols (menu arrows, the unsaved dot, Live AATF's triangle) on the text's line, no
      wrapped row in either view; the player-by-player comparison with the drafts stays
      the maintainer's final pass (f1c)
  - [x] w5 a modal window's title at the drafts' 14 points (`.window .title`), a text style of
    its own, where the app draws egui's `Heading` (15 after w2r1). Done 2026-10-28 (SWE-2,
    brief `.tmp/0_0_1z/brief_w5.md`, reviewed; DECISIONS "A window's title is its place"):
    `theme::window_title`/`window_title_text` (14, strong) on every modal's title (studio_core
    3, save_editor 10, ruleset_editor 4) and, by the lead, the three `egui::Window` titles
    (Help, Comparator, AATF violations); pages' `h2` headings and `info_strip`'s player title
    stay `Heading`
    - [x] the built window checked 2026-10-28 at `825f1885` (`.tmp/0_0_1z/shots/w5_01_about.png`,
      `w5_02_help.png`): About's title in white at 14 (its ink 173 wide against the body's
      "Release page" 74: 2.338, where 14 gives 2.321 and 15 2.397), Help's 28 wide (14: 28,
      15: 31). egui's title bar centres the Help title where the drafts had it on the left:
      the drafts now centre it (DECISIONS "A floating window's title centred"; checked by
      `.tmp/0_0_1z/title_centre.py`: Comparator, AATF and Help titles at their room's centre,
      no page errors)
  - [x] w6 the maintainer's review of 2026-10-28 (DECISIONS "A Tricks panel under Skills" and
    "Panel titles on one line and 6.5 points above their content")
    - [x] w6a the drafts (lead): the Tricks panel (`trickCards`, `skillColumns`); the three
      column titles at one y (84, from 81 and 87) and every player-view title 6.5 points
      above its first line (General info 7); Player assignments and Advanced instructions 6
      under their titles, Player assignments' rows 4 apart (3 left free); the detail pane's
      scroll offset kept across picks and edits; the drafts' window drawn wider by the
      browser's scroll-bar width (`--sbw`, the panes' bars in that strip), since Firefox's
      wider bar wrapped Ability stats under General info. Checked (`.tmp/fb1028/`):
      `title_gaps.py`, `side_space.py`, `behaviour.py` (offset 400 kept across a pick and an
      edit; Tricks under Autumn 26, none under VTL11 and DBC7), `sbw17.py` (a 17-point bar:
      one row, the pane's content 931), `errors_min.py` (no page errors; the minimum size)
    - [x] w6c the drafts, the maintainer's second look (DECISIONS "A Free cards panel for a
      ruleset without tricks"): a ruleset with no `tricks` group shows a Free cards panel of
      the skills its `free_cards` effects name (VTL11: 17; `asideCards`, `asidePanel`); every
      player-view title 6 points above the first thing drawn under it, as the team view's;
      an open select's list in the body font with a sans fallback (Firefox drew it in its
      serif default); the duplicate `RobotoFlex-OFL.txt` dropped from the drafts' assets (the
      README holds the licence; the bundle test failed on it since 0.0.1w4a). Checked:
      `title_gaps.py` (all 6, the three titles still at y 84), `aside.py` (Tricks 11 under
      Autumn 26, Free cards 17 under VTL11, none under DBC7, no page errors),
      `draft_fonts.py` (0 findings), `bundle_drafts_test.py` (OK)
    - [x] w6b the code: the Tricks or Free cards panel (from the active ruleset's `tricks`
      group, else its `free_cards` effects' skills, none when neither or broken), the titles'
      line and 6-point spacings, Player assignments' gaps; a test
      that the detail pane's offset survives a pick (the app's one ScrollArea keeps it today,
      by reading) → verify: the gates; the built window's player and team views against the
      drafts (asks before it takes focus). The drafts approved 2026-10-28. Code done
      2026-10-28 (SWE-2, brief `.tmp/0_0_1w/brief_w6b.md`, report `.tmp/w6b.notes.md`,
      reviewed): `card_columns` takes the indices to draw; `aside_cards` (Tricks sorted into
      SKILLS order, else every free card's skill, `choice` options and conditional ones
      included); every player-view title 6 above its first painted row, the titles' rows
      top-aligned so the three column titles share a line (no badge shift needed: the app's
      badge is no taller than a title); the team view's two panels 6 under their headings,
      each assignment label 4 under the chip above; the detail pane's offset pinned across a
      pick. Open: the built views against the drafts, with the batched checks; Player
      assignments' panel keeps about 33 free at its bottom where the plan expects about 3
      (the app's `.small()` chips draw 15 tall, the plan's arithmetic about 18): compare the
      chips with the draft's there and match them
      - [x] the built window checked 2026-10-28 (`.tmp/0_0_1z/shots/assign_side_by_side.png`,
        draft by `.tmp/0_0_1z/draft_assign.py`): the draft's `.rolechip` is a `button`, filled
        `--widget` #3c3c3c, 18 tall (`button { min-height: 18px }`), `--hover` on hover, the
        selection fill armed; the app's `chip` (`tactics/assignments.rs`) is an
        `egui::Button::selectable(..).small()`: no fill at rest, about 15 tall. The rest of
        the player view, the Tricks panel and the titles match the drafts
      - [x] w6c the chip as the draft's: a framed `egui::Button` `.selected(armed)` at the
        widgets' 18-point height (no `.small()`), so the role rows step 39 and the join rows
        20 as the draft's do and the panel's bottom slack drops to the draft's; a disabled
        target chip (Advanced instructions' "No target") keeps no fill, as the draft's
        `.rolechip:disabled`. Code done 2026-10-28 (SWE-2, brief `.tmp/0_0_1z/brief_fix1.md`,
        reviewed): `chip` a `Button` `.selected(armed)`, `interact_size.y` tall; the role rows
        measured at the draft's 39-point step; a disabled chip transparent and unstroked with
        its padding kept (the sidekick's `.frame(false)` dropped the padding, the text 6
        further left than the draft's: the lead's fix and assertion, red under it). Built team
        view checked 2026-10-28 at `8978a0be` (`.tmp/0_0_1z/shots/31_assign.png`, `31_instr.png`):
        the chips filled and stepped as the draft's, the panel's bottom as the draft's, "No
        target" under the dropdowns' text
- [x] 0.0.1x the formation's name on the pitch (worklog 8.32, moved into 0.0.1 by the
  maintainer, 2026-10-27: "Since the formation name work is almost done"; DECISIONS 2026-10-27 "Formation pictures, the formation's name and the nation names: the maintainer's answers": the
  typeface need not be the game's, its stroke thickness must match the game's label; the
  formation pictures' cells carry it too, when 8.33 comes). Drafted by side-lead D
  (`side/drafts2` `0f653350`; the rule, its evidence and open points in
  `../4cc-studio-temp/wt-drafts2/.tmp/8_32/rule.md`)
  - [x] x1 the draft revised (side-lead D's step 5): rebased onto main (Selawik, the symbols
    font), the name drawn in the bundled Selawik weight whose stroke matches the game's label.
    Done on `side/drafts2` (`4323d103` the label, `5bc37265` Selawik Semibold: vertical strokes
    2.97 px against the game's 3.01 at 15 px digits, Bold 3.41; its `.tmp/8_32/look.md`)
  - [x] x2 the captures (lead, PES 2021, 2026-10-27; `.tmp/0_0_1x/captures.md`, the save
    restored by hash): every label fits one rule, the ten outfield starters counted by the
    playable-positions grid's row their position belongs to, back to front, an empty row left
    out (LB CB RB / DMF CMF / LMF AMF RMF / LWF CF SS RWF): /f/'s three formations read
    3-3-2-2, 3-3-4, 3-4-2-1, /gd/'s presets 4-2-1-3, 4-3-1-2, 4-3-2-1, /o/ 3-3-3-1; on /3/ the
    AMF given CMF at his own spot turns 3-3-1-3 into 3-4-3 (the position counts, not the spot).
    The drafted rule's LMF/RMF in the DMF/CMF line was wrong (3-5-2 and 3-6-1 on /f/). C4 not
    needed (x1 measured the stroke at 720p)
  - [x] x3 the plan text (`plans/save_editor.md` "Tactics editor", look.md's proposal with x2's
    rule; the grid's rows and the name read one table, `save_editor`'s `POS_ROWS` today), its
    acceptance scenario, D's two draft commits on main with the draft's rule corrected (lead),
    the maintainer's look at the draft. Done but the look: "The formation's name" and SE-GUI-39;
    the draft's `formationName` reads the grid's `POS_ROWS` and gives the eight captured names
    (`.tmp/0_0_1x/draft_names_check.py`; D's drafted rule failed two); Selawik Semibold and
    `--font-label` in the drafts. The look: approved 2026-10-28 (DECISIONS "The drafts' looks
    of 2026-10-28")
  - [x] x4 the code (Opus, one run with y2 and round S2's rework, `.tmp/0_0_1x/brief_0_0_1x4_y2.md`):
    `team_widgets::pitch::POSITION_ROWS` the one table (the positions grid reads it, its copy in
    `save_editor` gone), `formation_name` over it, the name painted in the canvas' corner after
    the markers in the `label` family (`theme::install_fonts`: Selawik Semibold, then the
    proportional chain), its baseline placed from epaint's glyph origin → verified: the eight
    captured formations' names from the draft's real slots, the AMF→CMF `3-4-3`, the keeper
    never counted; the label 5 in, its baseline 6 up within 0.01, white, after every badge;
    SE-GUI-39 (/f/'s three, /gd/'s three, /3/'s drag and position change); red: a stub name, LMF
    and RMF in the CMF row (`3-5-2`), the label family dropped; `just gates-changed 89f884e2`.
    Lead: `TacticsView::popover_area` kept (the S2a-1 test's one key, `pub` for that test).
    The built window: the release build at `f1ca2342`, `.tmp/0_0_1z/shots/09_team.png`, /3/'s `3-3-1-3` in the pitch's corner (lead, 2026-10-28)
- [ ] 0.0.1y the Tactics pitch's two swaps as the game's (the maintainer, 2026-10-27: drag
  swapping should behave as the game's, and the select-one-then-the-other way be offered "if
  easy enough"; DECISIONS 2026-10-27 "A pitch player dropped on another swaps the two slots in
  the shown formation, as the game's drag"; the save diffs in `.tmp/0_0_1x/captures.md`)
  - [x] y1 the draft: a marker dropped on another exchanges the two slots' position, X and Y in
    the shown formation (`swapOnPitch`), the keeper never a target (`dropTarget`); the XI list's
    drops keep the starting-eleven exchange, the game's other swap
  - [x] y2 the code (with x4): a marker dropped on another swaps the two slots of the shown
    formation (`formations[..].players.swap`), the keeper no target of the drop or the swap mark
    (`swap_target`), the lineup rows keeping the starting-eleven exchange; doc comments, the
    help (`help/08_tactics.md`) and the cited decision repointed → verified: the whole record
    against the expected swap (premises that a swap elsewhere would show), the keeper held over
    (no mark) and released on (the snapped point), SE-GUI-12's two WHENs; red: the old
    starting-eleven exchange (three tests), `swap_target` taking the keeper. Open: the built
    window; the maintainer's look: "Looks fine" (2026-10-28)
  - [x] y3 the position areas (the maintainer, 2026-10-27: "Can we try mapping the areas, as
    well?"): measured, and there are no fixed areas to map; the editor keeps the dragged
    player's position (plan "A pitch player dropped on another", DECISIONS 2026-10-27 "A
    pitch drop keeps the position: the game's rule is not a map of areas"). Evidence and
    frames in `.tmp/0_0_1y3/findings.md`:
    - [x] y3a the evidence: Marco's RE has nothing on the edit screen's drag; in game (PES 2021,
      /f/ preset 1, the save backed up and unchanged by hash), the area lit while A is held is
      not where the position changes (an LWF reads LMF one push down, inside his band), and
      four drag scans read by the cyan brackets show the same spot giving different
      positions by who is dragged (LMF to AMF, CMF to LMF at (24.7, 26)) and the switch moving
      with the teammates (an LMF turns CMF about 11.5 short of the nearest CMF, at x 41.3 or
      31.7 as that CMF stands at 53 or 43)
    - [x] y3b, y3c dropped: a rule fitted to these scans has three unknown parts, each a study
      of its own, and a wrong guess sets positions silently; the maintainer's answer: read the
      rule from the executable (0.0.1y4)
  - [x] y4 the dragged player's position rule read from PES 2016's executable (the maintainer,
    2026-10-27: "the last one with a clean exe"; plan "A pitch player dropped on another",
    DECISIONS "The dragged player's position rule is read from PES 2016's executable"); does
    not hold the cut, the drop keeping the position meanwhile. The exe copied to
    `4cc-studio-temp/pes16_re/` (sha256 `1576ee45...`): 32-bit, `.text` not packed (entropy
    6.7; `.bind` is the Steam stub); capstone and pefile approved (the maintainer)
    - [x] y4a find the Team Sheet drag's code: the routine that writes a formation slot's
      position byte after a drop (from the slot record's layout, the save's 2-byte X/Y pair
      and position byte, and the edit screen's strings), and the tables it reads. Done
      2026-10-27 (Fable, `.tmp/0_0_1y4/re_findings.md`, `re_report.md`; Ghidra work in
      `4cc-studio-temp/pes16_re/`): `matchPlan::SelectPositionEdit`'s drag update, two
      13-row rectangle tables per position
    - [x] y4b state the rule as code reads it: each frame of a drag, a point inside the current
      position's rectangle keeps it; otherwise the band's default (the first of CB, CMF, SS
      whose rectangle holds the point) or the first free of LB, RB, DMF, LMF, RMF, AMF, LWF,
      RWF, CF, CB, SS holding it, whichever the dragged player's own per-position overall
      rates higher (the default on a tie). Teammates' spots never enter it: y3's switch near a
      teammate is the marker-overlap swap preview
    - [x] y4c check it against `.tmp/0_0_1y3/findings.md`'s four PES 2021 scans (every
      transition within one push); a rule that reproduces them goes to the plan, then the code.
      `simulate.py` reproduces all four scans within one push on the overalls read in game
      (2026-10-27, `.tmp/0_0_1y4/overalls.md`, the save unchanged by hash: team 718's PARTY
      HARD LMF 95, LWF 87, AMF 85, SS 84, CMF 83). The Select Position menu (X on a starter)
      lists exactly the rule's two candidates at three spots checked, a one-press check of
      the tables anywhere. The in-game check (Opus, pad + OBS, 2026-10-27,
      `.tmp/0_0_1y4/y4c_findings.md`, the save unchanged by hash): the CF row is PES 2016's,
      stored y 40 the first CF row (the d-pad, a held CF on /f/ 718, and the menu at y 40 and
      39; none of the save's 1,890 CF spots is below 40; y3's 40.7 was the lit area's edge,
      not a measured switch). PES 2016's overall formula does not give PES 2021's: of 201
      points read on /f/ and /e/ over all 13 positions, 189 within 1 and 12 off by 2 (registered
      CB +2, CMF +1, CF -1 at 82-99, A-rated wings -1, weak foot 3 one above weak foot 1 at 77,
      which 2016's formula cannot give), so PES 2021's weights differ; every player of the
      save has his abilities equal, so the stat mapping could not be tested. Marco's notes
      name PES 2021's per-role read (`FUN_1408c7580`), a cached value whose writer is not
      found. Answered (the maintainer, DECISIONS "The drag rule's overall is PES 2016's
      formula until a version's own is measured"): PES 2016's formula wherever a version's own
      is not measured; PES 2017, DBC7's game, measured first
    - [x] y4d the same check in PES 2017 (the lead, pad + OBS, 2026-10-28,
      `.tmp/0_0_1y4/y4d_findings.md`, the save unchanged by hash): PES 2017 has no Select
      Position menu, so the drag panel stood in (A held, the d-pad, the panel's new position).
      Ten edges crossed by six players of /3/ (701), each where PES 2016's tables put it: y 16
      both ways, 26, 35 both ways, CF's 40, x 21 in three bands, x 84; the swap takes over
      between 3.8 and 5.0 units in y. PES 2016's formula gives all 150 overalls read (residual
      0 on every one; 23 players, the held frames' swap previews included, weak foot 1 and 3
      alike). Not tested: the kicking power/swerve and body control/physical contact mapping
      (no player here has those unequal), the B bonus
    - [x] y4e the code, on every version (`plans/save_editor.md` "Tactics editor", the
      pitch-drop paragraph), in two slices
      - [x] y4e1 the overall in `pes_savefile`'s player model (the lead, 2026-10-28):
        `PlayerEntry::overall_at(position) -> Option<u8>` (`model/overall.rs`, `None` past
        CF), its tables checked entry for entry against the executable's dump
        (`.tmp/0_0_1y4/check_tables_vs_dump.py`); fixture `tests/fixtures/overall_points.tsv`
        (351 in-game reads, `.tmp/0_0_1y4/make_overall_fixture.py`): PES 2017's 150 exact,
        PES 2021's 201 within 2; five tests, each red under a perturbation (a weight, the
        keeper mean's count, the B arm, the blend's rounding); PES 2015's keeper means over
        the abilities it stores (DECISIONS). 384 tests, clippy, fmt. Its cross-family review
        (a new `pub` method) rides with y4e2's
      - [x] y4e2 the rule in `team_widgets`' pitch drop (the rectangles, the band default,
        the candidates), `RosterRow` carrying each starter's thirteen overalls from the host,
        the y3 scans and y4d's edges as tests. Code done 2026-10-28 (SWE-2, brief
        `.tmp/0_0_1y4/brief_y4e2.md`; `RosterRow::overalls` came with y6b): `pitch/drop_rule.rs`
        `dropped_position`, the executable's two tables, the grass drop setting the position;
        tests on 701's formation 1 (y4d's ten edges) and 718's formations 1 and 3 (y3's scans
        less the swap previews), expected values from `.tmp/0_0_1y4/y4e2_cases.py`, plus the
        tie, the held wide candidate, the keep and no overalls; each red on a stub and under a
        perturbation (CF's y 40, `free`, the tie, the keep, the candidates' order); help
        `08_tactics.md`. The (b) cross-family review of y4e1 + y4e2 closed 2026-10-28
        (`.tmp/0_0_1y4/rulings_y4e.md`): SWE-2 round 1 (1 + 2 accepted), Astra round 1 (3:
        the keep at the default untested, the plan's keep sentence wrong, `overall_at`'s doc
        overclaiming), SWE-2 round 2 (3: the second-LMF case untested, the areas being two
        table rows, a doc's keeper-box wording); every fix landed, each new test red first.
        Built window checked 2026-10-28 (release at `f1ca2342`, `.tmp/0_0_1z/drop_edges.sh`,
        `.tmp/0_0_1z/shots/26_sheet.png`): on the PES 2017 save, 701's slot 2 dropped at
        x 40 gave CB at y 15, DMF at 16 and 25, AMF at 26 and 34, SS at 35, the game's edges;
        the badges showed the test's PES 2017 overalls (CB 83, DMF 96, CF 101, AMF 101); on
        the PES 2021 save a CMF dropped at (104, 9), where CB's wider area alone holds the
        point, took CB; every drop undone, the saves unchanged by hash. The loop's
        `just mutants-diff 96cbb688` at `f1ca2342` (228 mutants over `pes_savefile`,
        `save_editor`, `studio_core`, `team_widgets`, with y6b, w4b, w6b and z2 riding
        along): 8 survivors, each a missing assertion (the slider's width and handle travel,
        `holds`' either-row rule, Playable positions' title gap under a 0.5 tolerance), one
        timeout caught; triage `.tmp/lead/mutants_diff_96cbb688_triage.md`, the tests briefed
        with w6c and z2c (`.tmp/0_0_1z/brief_fix1.md` part 3)
  - [x] y5 the slot popover's position row (the maintainer, 2026-10-28): no Position label,
    the dropdown and Make GK filling the popover's width, the button at its own width
    - [x] y5a the draft (`5d668583` and the commit after it) and `plans/save_editor.md` "The
      pitch is the editor"; checked: the dropdown 113 and Make GK 58 on one row spanning
      the popover, no page error (`.tmp/fb1028/answers_check.py`)
    - [x] y5b the code (`team_widgets` `pitch/popover.rs`) → verify: the gates; the built
      popover against the draft (asks before it takes focus). Code done 2026-10-28 (SWE-2,
      brief `.tmp/fb1028/brief_y5b_y7b.md`): no label, "Position" the combo's hover and
      accessible name, `egui::Sides::shrink_left` giving the combo the row less Make GK's
      own width (0 on the popover's sizing pass, so it never widens it); test
      `the_position_row_spans_the_popover`, red under a fixed Make GK and a fixed combo.
      Accepted: Tab reaches Make GK before the combo (`Sides` lays the measured side out
      first; matching the draft's order would mean measuring the button by hand). Built
      popover checked 2026-10-28 (`.tmp/0_0_1z/shots/11_popover_zoom.png`): the combo and
      Make GK span the popover's row, no label
  - [x] y6 the badge's overall and medal outline (the maintainer, 2026-10-28): the starter's
    overall at his slot's position at the badge's right in white, his medal tier a 1.5-point
    outline in its colour (bronze darkened to `#8c4a20`) instead of its letter, the badge's
    texts at 11.5 pt 2 in from its ends (the badge stays 50), the captain's mark outlined
    1 px in `#242b2d`
    - [x] y6a the draft and `plans/save_editor.md` "A marker", "A starter's badge shows his
      overall and his medal tier"; checked: the draft's `overallAt` equals
      `y4d_overall.py` on 128,570 player-position pairs, gives 150 of 150 overalls read in
      PES 2017 and 201 of 201 read in PES 2021 within 2 (`.tmp/fb1028/overall_port_check.py`);
      captures `.tmp/fb1028/badges_*.png`, no page error; the margins and the captain's
      outline measured on the maintainer's 720p capture (`.tmp/fb1028/cap_pixels.py`)
    - [x] y6b the code, after y4e1 (`team_widgets`' pitch paint and `RosterRow`, the save
      editor's host filling the overalls) → verify: the gates, the paint tests' medal letter
      (`team_widgets` `pitch/tests/paint.rs`) turned into the outline's; the built pitch
      against the draft (asks before it takes focus). Code done 2026-10-28 (SWE-2, brief
      `.tmp/fb1028/brief_y6b.md`): `RosterRow::overalls` (thirteen, by position byte) filled
      by the save editor's host through `overall_at`; the badge's overall white at 2 in, both
      texts at 11.5 (`BADGE_TEXT_INSET`, `BADGE_FONT`), the medal a 1.5 inside stroke (bronze
      `#8c4a20`, `Medal::letter` gone), the captain's mark on a 14 x 13 `BADGE_FILL` box; new
      tests `the_badge_paints_the_overall_at_the_slots_position`,
      `a_medal_outlines_the_badge_in_its_tiers_colour`,
      `the_pitch_badges_show_each_starters_overall_from_his_record`, each red first and under
      a perturbation (position 0's overall, inset 3, one player's overalls for all, the old
      bronze). Built pitch checked 2026-10-28 (`.tmp/0_0_1z/shots/09_pitch_zoom.png`): the
      overall white at the right, gold and bronze outlines (`#8c4a20` sampled), the captain's
      outlined mark; a portrait's baked-in ribbon can disagree with the outline (WIP's silver
      ribbon, bronze under the active ruleset, "Bronze: -1 of 4 cards"): the ribbon is the
      cup's art, the outline the rules in use
  - [ ] y7 Substitution's Very early on PES 2018 to 2020 too (the maintainer, 2026-10-28:
    taken from PES 2017 and 2021 having it, the games between them)
    - [x] y7a the draft and `plans/save_editor.md` "Tactics editor" (the Automatic
      settings' Substitution)
    - [x] y7b the code: `team_widgets`' `substitution_choices` offers Very early from PES
      2017 on; its test's PES 2018, 2019 and 2020 rows expect it (SWE-2, with y5b; red on
      PES 2018 first) → verify: `cargo test -p team_widgets`
    - [ ] y7c later (the maintainer: "we'll check in detail in the future"): a look at PES
      2018, 2019 and 2020's Game Plan, Automatic settings, Substitution; the plan's "taken
      rather than looked at" goes with it (`QUESTIONS.md` "After the release")
- [ ] 0.0.1z the maintainer's look at the built app (2026-10-27 ~21:10): the app's sliders
  kept, the drafts made to match
  - [x] z1 the drafts: sliders drawn as egui's (8-point rail, 11 × 14 handle, value box), the
    handle and the rail up to it in the selection blue; no team name on player rows; a
    positions label sets the registered position; the player list keeps its scroll across
    a redraw (the draft rebuilt it at the top); a frameless Reset window right of the
    version; the close prompt's "The open save has unsaved changes." and "Save writes them to
    … Discard loses them." lines gone (the maintainer: redundant beside the title and the
    buttons). Plan `core/gui.md` the theme row, "Status bar" (ST-BAR-01, ST-BAR-05),
    `save_editor.md` "Editing UI" (the player rows, Playable positions, SE-GUI-40). Checked
    in Chromium and Firefox (`.tmp/draft_look/check.py`). Approved 2026-10-28 (with z4)
  - [x] z2 the code, once z1 is approved (its built-window look batched with 0.0.1's)
    - [x] z2a `team_widgets` + `save_editor`: the slider's trailing fill and handle in the
      selection blue (`tactics/mod.rs` `slider`, the handle overpainted from egui's own
      geometry, which egui keeps private; the physique sliders when built), the rows'
      team name gone (`view/player_list.rs`), the prompt's two lines gone and its width
      its widest row's up to nine tenths of the window, a wider row scrolling sideways
      (`view/prompt.rs`), a positions label's click registers it (SE-GUI-40,
      `view/abilities.rs`); help pages 02 and 03. Open: the built-window look; the
      handle's stroke brightens on hovering the value box too (egui strokes it from the
      slider part alone), cosmetic
    - [x] z2b `studio_core`: Reset window (`shell/status_bar.rs`, ST-BAR-01's test,
      ST-BAR-05, ST-BAR-06: drawn only off the active class's design size or maximized;
      `window_size` `off_design_size`, `reset`). Open: the built-window look, the click
      on a maximized window landing centred (the fit reads the maximized frame's
      decorations);
      ST-BAR-05/06's special-tool clauses wait for ST-WIN-03 (special tools' declared
      sizes, not built: the class is always the main one)
    - [x] z2 built window checked 2026-10-28 (release at `f1ca2342`, `.tmp/0_0_1z/shots/`):
      the sliders' blue fill and handle, rows without the team, the prompt's table alone and
      its width, a position label's hover text, hand cursor and click (one undo entry);
      Reset window drawn at 1100 × 800 and maximized, not at the fit; on a maximized window
      its click restores 1280 × 800 centred at (312, 121)
    - [x] z2c Reset window under zoom: on a 1100 × 800 window (zoom 1100/1280) its click
      left the window 1100 × 688 at (402, 177). `window_size::reset` sends `fit`'s size and
      position in points at the current zoom, which egui-winit multiplies by the zoomed
      pixels-per-point (800 × 0.859 = 688), and `fit` reads the monitor and decorations in
      those zoomed points too (the centre for a 2234-point monitor). Fix: `reset` fits at
      zoom 1 (monitor and decorations times the zoom) and sends size and position divided by
      the zoom; `off_design_size` compares in the same zoom-1 units (its monitor and
      decorations times the zoom too). Test ST-BAR-05 at a zoom under 1 (a 1100-wide frame
      after the width zoom lands): the commands are 1280 / zoom × 800 / zoom and the centred
      position / zoom. The ST-BAR-05/06 harness tests ran at zoom 1 only. Code done
      2026-10-28 (SWE-2, brief `.tmp/0_0_1z/brief_fix1.md`, reviewed): `fit(viewport, zoom)`
      for the first frame and `reset`; `off_design_size` converts the monitor and the
      decorations by `zoom × screen / inner`, which is the zoom on a consistent frame and the
      input's own on the pass a queued zoom lands on (the plain zoom broke the narrow-monitor
      assertion there); tests at the landed 1100/1280 zoom. A click on exactly the landing
      pass would centre with the input's monitor size: one frame, accepted. Built window checked
      2026-10-28 at `8978a0be`: from 1100 × 800 the click restored 1280 × 800 at (312, 120),
      the button gone after it
  - [x] z3 the sidebar's active-tool bar: the draft rounds its left corners, egui draws it
    square (the maintainer, 2026-10-27, "for later"). Planned 2026-10-29 (`core/gui.md`
    "Sidebar" item 3: the bar's left corners rounded 2, the right square). Done 2026-10-29
    (Opus, brief `.tmp/0_0_1ov/brief_z3.md`, reviewed): `sidebar/mod.rs`
    `ACTIVE_BAR_CORNER` 2 on `nw`/`sw`; test `the_accent_bar_rounds_its_left_corners_only`.
    Seen on screen at the release rebuild (ov4c's captures)
  - [x] z4 the maintainer's second look (2026-10-27 ~21:55), drafts and plan: Reset window
    drawn only while the window is maximized or off its active class's design size
    (`core/gui.md` "Status bar", ST-BAR-01's GIVEN, ST-BAR-06 new); the unsaved-changes prompt
    as wide as its widest row up to nine tenths of the window (`save_editor.md` "Editing
    UI"); every draft's controls in one column left of the window, in sections (`core/gui.md`
    the drafts' "Size" bullet); the font trial switches faces in place (no reload, no
    fallback frame) and lists Roboto and Roboto Flex as shipped. Checked in Chromium and
    Firefox (`.tmp/draft_look/check2.py`)
- [x] 0.0.1j the default-settings audit before the cut: every setting's default in the
  bundle read against what a member running the preview for the first time needs (the
  bundled `settings.toml`, each tool's `Default`, the "Aesthetics from exports" switch off,
  `aatf_rules` naming `rules/dbc7.4ccrules`), each finding fixed or recorded. Read 2026-10-25
  (lead), the common settings, the Save editor's and the Team compiler's; findings:
  - [x] j1 the settings menu draws "Check for updates" and its interval, which no code reads
    before Release 0.2.0's updater: the two rows go until then (DECISIONS 2026-10-25,
    `plans/core/gui.md` "Settings menu"). Done: the rows are gone, the keys and defaults stay,
    `the_update_check_rows_are_not_drawn_yet` replaces the interval test
    (`shell/settings_menu.rs`). The drafts host keeps the rows and its Updates help topic: it
    draws the design, the 0.2.0 rows included
  - [x] j2 `pes_version` defaults to PES 2019 (VGL's game); DBC's window testers are on
    PES 2017, so their first open shows SE-GUI-28's version warning until they pick 2017 in
    the sidebar. Not a bundle default, since VGL's organizer compiles from the same bundle on
    PES 2019: the readme says to pick the version first (0.0.1f's readme). Done 2026-10-27
    with 0.0.1f's preparation: the readme's "The first time" step 3
  - [x] j3 Live AATF off by default (the plan's choice): P37 proposed it on; vetoed
    2026-10-26, its visibility asked for instead (0.0.1o)
  - the rest read right for a first start: the Team compiler's names and `multicpk_mode` off
    (VGL's are the readme's, 0.0.1f), `savefile_path` auto, "Aesthetics from exports" off
    (SE-GUI-35), the hide IDs 55/11, the theme dark, the exports and output folders beside the
    exe
  - [x] j4 the audit finished (Fable, 2026-10-27, every key of every tool and the bundle's
    `settings.toml`, `.tmp/0_0_1j/j_audit_report.md`): the remaining keys read right; to fix
    (brief `.tmp/0_0_1j/brief_j4.md`): the settings menu draws a CLI-only tool's section (the
    Ruleset editor's header over nothing, the Team compiler's "not built yet" line), which
    goes (plan `core/gui.md` "Settings menu", DECISIONS "A CLI-only tool has no settings
    section"); the `aatf_rules` row's tooltip worded for a member; the readme's
    `upgrade-dpfl` step under a heading both groups read (the official-list check runs for
    PES 2017 too); stale settings text in `spec/team_compiler/settings.md`,
    `spec/save_editor.md` and `save_editor/src/settings.rs`'s module comment. Done 2026-10-27
    (Opus): `SettingsMenu::show` takes the shell's `cli_only` and skips those tools; the
    tooltip; the dpfilelist step is "Compiling"'s step 4, the VGL section pointing at it; the
    spec's "Common settings" lists all 15 `CommonSettings` keys, `pause_allow`'s row says the
    bundled launcher pauses once. ST-SIDE-08's text and test now say no section (the
    sidekick's contradiction). The Team compiler's "not built yet" line is unreachable while
    it is CLI-only, kept for its window. Files: `studio_core/src/shell/{mod.rs,settings_menu/,
    sidebar/tests.rs}`, `studio/src/main.rs` (comment), `save_editor/src/{lib,settings}.rs`,
    `resources/bundle/README.txt`, `spec/{save_editor,team_compiler/settings}.md`,
    `plans/core/gui.md`
  - [x] j5 the bundle ships VGL 27's rules file too, just in case (the maintainer,
    2026-10-27; plan `core/development_plan.md`'s 0.0.1 paragraph, DECISIONS "The preview
    bundle carries VGL 27's rules file too"): `release.py` assembles `rules/vgl27.4ccrules`
    beside DBC7's, the readme's VGL section says to pick it (with j4's brief). Done
    2026-10-27 with j4: `RULES_FILES` maps each bundled file to its data block, each checked
    by the staged exe's `validate` and `update-logic` (a staging run: both validated, logic
    current); the readme's VGL step 4 names the settings row. Files: `scripts/release{,_test}.py`,
    `resources/bundle/README.txt`, `plans/core/development_plan.md`
- [x] 0.0.1cm comments that stand on their own (the maintainer, 2026-10-27; `CONTRIBUTING.md`
  "Code style", the bullet "A comment stands on its own"; DECISIONS "A code comment stands on
  its own"): the 1044 comments `just comment-refs` lists rewritten as plain sentences, in
  slices a review each, then the check joins `just gates`
  - [x] cm1 config files, `scripts/`, `crates/dev/`, `studio` and the libs not under review
    (brief `.tmp/cm/brief_cm1.md`, Opus). Done 2026-10-27 (`.tmp/cm/cm1_report.md`): 245 hits
    in 87 files rewritten, comment lines only; a pointer kept only where the section is in the
    spec; the touched crates' tests, clippy and fmt green. The sidekick found the checker
    missing step tags without a letter (`step 4.18`, `3.7's`), letter steps (`3.w`), a
    `4.14e3` taken for an exponent and `mutants-diff <sha>` tags: `comment_refs.py` widened
    (versions such as `0.37's` and `0.y.z` still pass), and the 12 it then found in this
    slice's files fixed by the lead
  - [x] cm2 `team_compiler` (335; brief `.tmp/cm/brief_cm2.md`, Opus). Done 2026-10-27
    (`.tmp/cm/cm2_report.md`): 335 hits in 64 files, comment lines only; pipeline-stage
    pointers replaced by what the stage does, plan-only "Resolved decisions" pointers by
    their reasons (each added reason checked against the spec by the lead), pointers kept
    only into `spec/`; `team_compiler`'s tests (1537), clippy and fmt green. The sidekick's
    contradictions: resource READMEs exempted like fixture READMEs, rejected (the
    `include_bytes!` path names the folder); a pointer split across two lines escapes
    `doc_pointers.py`, accepted (cm6)
  - [x] cm3 `studio_core`, `export_upgrader`, `ruleset_editor` (brief `.tmp/cm/brief_cm3.md`,
    Opus). Done 2026-10-27 (`.tmp/cm/cm3_report.md`): 181 hits in 70 files, comment lines
    only; step numbers and decision pointers replaced by what the step does and the
    decision's reason (each checked against its DECISIONS entry or plan paragraph by the
    lead), pointers kept only into `spec/`; the three crates' and `studio`'s tests, clippy
    and fmt green; 282 hits left, all in `save_editor` and `team_widgets` (cm4)
  - [x] cm4 `save_editor` and `team_widgets` (brief `.tmp/cm/brief_cm4.md`, Opus). Done
    2026-10-27 (`.tmp/cm/cm4_report.md`): 282 hits in 61 files, comment lines only (checked
    by script); plan-only pointers replaced by the reasons their sections or decision entries
    give (spot-checked by the lead), pointers kept only into `spec/`; the lead retouched two
    wordings. `comment-refs` now 0 across the workspace, `doc_pointers.py` 0 broken.
    `save_editor`'s and `team_widgets`' tests, clippy and fmt green (the CLI tests only with
    PES 2021 closed: the Issues entry on the CLI's process list). The sidekick's
    contradictions, all accepted: briefs drop the grep against wrapped titles, which the
    widened check reads; `(the maintainer, <date>)` and `r1`-style round names join the
    scanner (cm5); the Comparator's plan-only bullets lose their pointers
  - [x] cm5 `comment-refs` in `just gates` and `gates-changed` (gate 7; CI runs `just gates`),
    CONTRIBUTING's gate list naming it. Its patterns widened (red first): an "attribution"
    kind, "maintainer" followed by a date or a deciding verb (anchored on the one word, since a
    wrapped comment leaves "the" on the line before; "the maintainer's save" passes), and "the
    brief" in the review kind. Five comments it found rewritten (`team_widgets`
    `pitch/lineup.rs`, `save_editor` `view/tests/quick_actions.rs` twice, `pes_savefile`
    `settings_toml/tests/appearance_row.rs`, `model_convert` `convert.rs`) and
    `scripts/ingame/install.py`'s; the gate fails on a hand-planted attribution, reverted.
    `r1`-style round names left out: the one in the tree also named the maintainer, and a bare
    `r\d` pattern flags `[r0, r1, r2, t]`
  - [x] cm6 `scripts/doc_pointers.py` reads a pointer whose quoted title a comment wraps onto
    its next line (today its title pattern stops at the line end, so such a pointer is
    neither resolved nor reported, and a broken one passes "0 broken") → verify: a test case
    of a wrapped comment pointer, resolved and, perturbed, reported broken (lead's: the
    check is correctness-critical). 2026-10-27: the widened check (a cut pointer joins the
    next line, a comment line in `.rs`; a decision entry's title is an anchor) reads 1282
    pointers where 1050 were read, 24 of the new ones broken; the 19 in `docs/` repointed;
    the script lands with cm4, which fixes `save_editor`'s `view/tests/appearance.rs` pointer
    into `plans/core/gui.md` "Shell layout"
- [ ] la the library crates audited again (the maintainer, 2026-10-27: "the library crates need
  to be as solid foundations as they can be"): four read-only Fable audits of every lib crate
  but `team_widgets` (brief `.tmp/lib_audit/brief.md`: each finding names its reach or is an
  edge case, over-engineering a class of its own), 55 findings, ruled by the lead
  (`.tmp/lib_audit/rulings.md`): 43 accepted (some conditionally, one after the cut), 12
  rejected with a line each. The audit of `music_export` stopped at its context limit after the format core
  (`select.rs`, `eval.rs`, `resolve.rs`, `state.rs`, `write.rs` unread; Phase 10). Rework in five slices
  under `.tmp/lib_audit/rework_common.md`, Opus sidekicks, each reviewed and gated before it
  lands, `mutants-diff` over the loop at its end
  - [x] A1 `pes_savefile`'s behavior and spec fixes (`slice_a1.md`): Anchoring takes a
    target player (PES 20/21), pinned against every fixture's tactics; an ability's range
    error names the stored 0 to 127; a Team TOML `apply` stages the rostered players only;
    the PES 15/16 appearance join through a map (25.9 to 16.6 ms on the PES 16 fixture); the
    dead PES 16 gating test split in two; the spec's text limits, team comparison and
    `is_fpc_player` brought to the code
  - [x] C the model stack (`slice_c.md`): one carried-weight threshold for the hand detectors;
    `pes_model`'s merge refuses a repeated bone; a split group with LODs or past 256 bones
    stays split; `fmdl`'s vertex encoder fails before it permutes; the anti-blur duplicate's
    two-sided bit in the spec; three test gaps. c-8 closed unchanged (Konami's PES 21 boots
    carry alpha 0 too)
  - [x] B `aatf`, `pes_version`, `kit_config`, `color_tools`, `pipeline`, `vtree` (`slice_b.md`):
    a kit config's injected notes name both models `validate` accepts (144/160), the
    upgraded tracers' goldens with them; `TeamViewError` carries the import note, not its
    `Debug`; a bad badge value names its key; the operation limit's margin checked on all
    nine embedded rulesets (DBC7 the heaviest, 786,215); `RelativeScopePath` removed (no
    user); tests for `TierValues::write`'s gate and `year`/`Display`; fixture comments
    without the legacy checker
  - [x] D `dds_convert` and the containers (`slice_d.md`): the Converter rewraps blocks a
    target keeps without decoding them (a 2048x2048 BC7 face to PES 21: median 156 to
    85 ms, bytes identical, pinned by goldens captured before); three tests' fixtures moved
    into their own crates; `GK_GLOVES_RANGE` and `FpkFile::remove` removed (no users);
    comments and spec sentences brought to the code
  - [x] A2 `pes_savefile`'s design items and `aatf`'s bare record (`slice_a2.md`): the unused
    `convert_player` deleted, the spec's cross-version conversion described as `TeamToml::apply`
    carries it; `write_player`/`write_team` through `write_record`; one
    `RecordSchema::bit_width`; the Texport roster pairing one function per direction;
    `TeamTactics::bare` names no roster slot (0xFF), `aatf`'s patches gone
  - [x] the loop's `mutants-diff ba7a4b6d` (2026-10-28, 158 mutants over 13 crates): 3
    survivors, all equivalent, excluded with their equivalences (`883eb9ad`); A2's known gaps
    tested (SWE-2, brief `.tmp/lib_audit/brief_a2_gaps.md`): both Texport readers'
    `RosterMismatch` through `from_bytes` on files whose records trade places, the writers'
    too-few and left-over refusals through `new` and `to_bytes`, the `arrays`-made fields'
    widths through `bit_width`/`widest_bit_width`; each shown to fail under a perturbation
    (`texport.rs`, `schema/mod.rs` tests)
  - [ ] after the cut: `aatf`'s nine fixture modules' repeated tests as helpers (b-3)
- [x] 0.0.1ov every built window laid over its draft before the final pass (the maintainer,
  2026-10-28: the final pass's first look found the first-run prompt unlike its draft, unequal
  columns with no gap, small buttons, a hint wrapped mid-path; the overlay of 8.20, planned in
  `plans/core/gui.md` "The host's other review aids", had never been built)
  - [x] ov1 the comparison (lead): every screen both have (the first-run prompt, the empty
    Save editor, About, Settings, Help, the player and team views, the comparator, the AATF
    panel, the unsaved prompt) captured from the built window (PrintWindow, 1280 × 800) and
    from the draft at the same size (1280 × 780 since ov2a), laid over each other at half opacity, beside each other,
    and as a difference image; every mismatch over the tolerance (one pixel, two where needed:
    `plans/core/gui.md` the Overlay bullet) listed for the maintainer → verify: the first-run
    prompt's mismatch shows in its overlay. Done 2026-10-29 in the dark theme (the light one
    is ov11b), on a release build of `67af3fd4` (`.tmp/0_0_1s4t_v2/app/`), every screen also
    measured: `.tmp/0_0_1ov/rect_compare.py SCREEN [--modal CSS]` pairs each named element of
    the app's UI Automation tree with the draft's DOM element of the same text and lists every
    pair off by more than a point (tables in `.tmp/0_0_1ov/rects/`; captures `draft/`, `built/`,
    crops `out/`). The unsaved prompt was left to ov3's headless pass. The mismatches, by
    cause, are ov2j to ov2r (the first-run prompt's in ov2j and ov2q); one is the
    maintainer's pick (QUESTIONS "Ability stats' labels: left as the draft shows, or right?").
    Matching within the tolerance: the Help window's size, the Comparator's width, the
    Settings menu's width and path box (the plan's 660 and 472, not the draft's 440). Not
    mismatches: the version warning SE-GUI-28 and the Open menu's discovered saves the draft
    draws (8.9l slice l3, not built), the rail's order (the build's window tool first, by
    design), a combo's arrow (the browser's own chevron in the draft, egui's triangle)
  - [x] ov2 the fixes (sidekick, one brief): the mismatches the maintainer confirms, each with
    a layout test of the draft's measures (the first-run prompt: two columns of equal width 16
    apart, buttons as wide as their column, 15-point text)
    - [x] ov2a the default client area 1280 × 780 in the app (the maintainer, 2026-10-28: the
      drafts drew 1280 × 800 outside, a 1278 × 775 client area; both now 1280 × 780, the drafts
      done by the lead, DECISIONS "The window's default client area is 1280 × 780") and the
      first-run prompt as drafted (Opus sidekick, brief `.tmp/0_0_1ov/brief_ov2a.md`, report
      `.tmp/sidelead/sk_ov2a.report.md`): `window_size::DEFAULT_SIZE` 1280 × 780 (ST-WIN-01
      at (312, 130.5), ST-BAR-05/06), `save_editor`'s `DEFAULT_PANE` 1224 × 756 (every fit
      test passed there unchanged); the prompt's two 299-point columns 16 apart, 38-tall
      buttons at 15 points, hints wrapped in their column
      (`the_two_choices_are_equal_columns_with_full_width_buttons_and_wrapped_hints`, red on
      the old layout, each tolerance perturbed). A real user folder's hint takes three lines
      at 299, egui breaking at the space after the path, never inside it. The draft's
      `.firstrun` margins (2 above the row, 8 below) landed with ov2b
    - [x] ov2b a checked checkbox and a selected radio button blue in the app (the maintainer,
      2026-10-28; drafts done by the lead, DECISIONS "A checked checkbox and a selected radio
      button are blue"; Opus sidekick, brief `.tmp/0_0_1ov/brief_ov2b.md`, report
      `.tmp/sidelead/sk_ov2b.report.md`): `studio_core::widgets`' `checkbox`,
      `checkbox_without_text`, `radio`, `radio_value` (`widgets/choice.rs`) paint the blue
      and the 2-point tick or dot over egui's own widget, in its layout and state border
      (egui's `CheckboxStyle` hook needs its `experimental` feature, the radio has none);
      21 checkbox and 9 radio sites moved in `studio_core`, `team_widgets`,
      `ruleset_editor`, `save_editor`, `team_widgets` now depending on `studio_core`
      (DECISIONS "team_widgets draws the suite's checkbox from studio_core"); the first-run
      row 2 under the title and 8 above the footer. Tests in `choice.rs` and
      `first_run.rs`, red on egui's plain widgets; the draft's pressed tick and dot, white,
      brought to the decision's selection text colour. The lead's `clippy.toml`
      `disallowed-methods` against egui's own checkbox and radio (DECISIONS "egui's own
      checkbox and radio button are disallowed"), red on `choice.rs` before its four
      `#[expect]`s, and on a `save_editor` site put back to `ui.checkbox`
    - [x] ov2c Live AATF's crosses as redrafted (the maintainer, 2026-10-28; DECISIONS "The
      final pass's first rulings"; drafts done): an ability stat's cross left of its label, 2
      points from the text, the column's crosses lined up and no label moved; the pitch's cross
      an X over the whole portrait, the severity's colour in a thin black outline, its tooltip
      over the portrait (`team_widgets::violations`, `pitch`, `save_editor`'s ability stats).
      Built with ov2f (Opus sidekick, brief `.tmp/0_0_1ov/brief_ov2cf.md`, report
      `.tmp/sidelead/sk_ov2cf.report.md`): `pitch::paint_portrait_cross` (black strokes 7
      wide under the colour's 5, round ends), the hover over the portrait, a press on it still
      opening his popover
    - [x] ov2d the player list's scroll bar outside its frame (the maintainer, 2026-10-28;
      drafts done): egui's solid bar, 4 points from the content and 6 wide, the strip kept
      while nothing scrolls; a row's fill reaching the frame's edge (`save_editor`'s
      `player_list.rs`, replacing the reserved floating band); the controls above and below
      the list ending at its frame, not at its bar, Set stats still fitting; a row numbered by
      the player's roster place `01` to `23`, not his shirt number; a chosen A rating
      `#129648` under a white letter (DECISIONS "The final pass's second rulings"; drafts done;
      `plans/save_editor.md` "Editing UI", the Player list bullet and the positions grid).
      Built with ov2g (Opus sidekick, brief `.tmp/0_0_1ov/brief_ov2dg.md`, rework
      `rework_ov2dg_r1.md`, report `.tmp/sidelead/sk_ov2dg.report.md`): `player_list.rs`'s
      frame drawn to the draft's `.rows`, `ScrollStyle::solid()`, rows at the frame's width;
      `row_text`'s `{place:02}`; `RATING_A_FILL`. The list column is the draft's 255 without
      egui's panel margins and separator, which `default_size(255)` had counted (17 points:
      the frame now at 12..257 in the pane, as the draft's); a team row with an elided name
      draws no hint
    - [x] ov2e the wheel over a combo box or a drag value steps it (the maintainer, 2026-10-28;
      DECISIONS "The wheel over a combo box or a drag value steps its value"; drafts done):
      `studio_core::widgets::{wheel_steps, step_choice, step_number}` (`widgets/wheel.rs`),
      called at 36 of the suite's 38 combo box and drag value sites (the two left are always
      disabled: the keeper's `GK` combo, the dribbling stand-in), `color_tools` now
      depending on `studio_core`; ST-WHEEL-01 to 03 in `wheel.rs`, a notch one undo step in
      both editors (`save_editor` tests/abilities.rs, `ruleset_editor` tests/tiers.rs), the
      picker's commit. `e6d53ee0`, merge `e9dbb1ab`. Left as is: a pane scrolled while no
      combo or drag value is drawn is not recorded, so a control under the pointer within
      500 ms steps (rare: some control is on nearly every pane)
      - [x] ov2e2 a wheel step while a text field held the keyboard joined an open undo entry
        (the editors seal only with nothing focused), where `plans/core/gui.md` "The wheel over
        a value" has one undo step per notch. Fixed in `wheel_steps` (DECISIONS "A wheel notch
        first takes the keyboard from the focused widget" and its correction): a due step takes
        the keyboard and comes two passes later, the field's commit on the loss (the Ruleset
        editor's tier id) landing in between; the drafts' listener blurs the focused input.
        Tests: ST-WHEEL-04 in `wheel.rs` (a field; the drag value itself typed in, which
        commits each keystroke), the Save editor's name then two notches (three Ctrl+Z), the
        Ruleset editor's tier id then a rate notch (two), each red before the change; one-pass
        delay perturbed, the tier id test fails. `b49a07f1`, merge `69b1d8e8` (Opus sidekick,
        brief `.tmp/0_0_1ov/brief_ov2e2.md`)
    - [x] ov2f Live AATF's marks as ruled (the maintainer, 2026-10-28; DECISIONS "The final
      pass's second rulings"; drafts done; `plans/save_editor.md` the Live AATF bullet): the
      list's check mark orange for a Warning, blue for suggestions alone; a field's mark left
      of its label everywhere (General info, Other stats, the panels' headers; the card
      budget's rows on the Skills header's mark with the skills'), a red `✖`, an orange `⚠` or a blue `ⓘ`
      (new in the symbols font: `theme.rs`'s symbol list test gains it) in an 11-point box,
      glyph centred; the check asked for the suggestions, the dock listing
      them under `Suggestions`, its header counting errors and warnings; a suggestion marking
      its allowance's field; the player view's column gaps (ov2g); Show
      suggestions right of the toggle while it is on (`live_suggestions`), the suggestions
      asked for only then; `Team-wide issues` heading the team's own rows (DECISIONS "The
      final pass's third rulings"). Built with ov2c: `live_marks`' `Marks::paint` before a
      label or a panel's title (`field_cross`, `cross_after_label` and the counted label's
      cross gone), `RowMark`'s highest severity, `unused_field` (a `height` suggestion marks
      the Height field, the plan's list completed), `CheckKey.suggestions` with the worker
      dropping unasked ones, `live_suggestions` in the settings, `help/02_window.md`. Tests in
      `pitch/tests/paint.rs`, `tests/live_marks.rs` and `tests/live_aatf.rs`, red on the old
      marks; five old mark tests merged into the per-kind
      `each_fields_mark_stands_left_of_its_label`. Open: no acceptance scenario covers Show
      suggestions or the mark kinds yet; the dock draws a `Team 734 /lit/` line the draft's
      lacks (for ov6)
    - [x] ov2g the fourth rulings' looks (the maintainer, 2026-10-28; DECISIONS "Live AATF
      replaces the AATF panel; the final pass's fourth rulings"; drafts done,
      `plans/save_editor.md` "Editing UI"): General info's and Other stats' fields 6 in from
      their titles (the first column 350); the view's four equal gaps (after the list's frame,
      between the columns, to the window's edge: 29.4 at 1280 × 780); the Cards left badge
      `Gold: 6 of 8`, no `cards`; the Name and Shirt counts `#5a5a5a` under the limit, the
      warning orange
      on it, the error red past it (SE-GUI-38). Built with ov2d: `abilities.rs`'
      `CELL_PADDING` 6 (the inset, and Ability stats' cells, which lacked the draft's outer
      6 on each side: 157.4 wide against the draft's 169.4), the four gaps in `view::show`;
      `cards::cards_left`'s format; `info_strip`'s `count_color`. Tests in `tests/list.rs` and
      `tests/abilities.rs` (the frame's edges, the column's width, the four gaps at 29.4),
      red on the old layout
    - [x] ov2i the ability stats' labels right-aligned as the draft draws them
      (`td.ilabel { text-align: right }`; found by ov2dg's sidekick, 2026-10-28), each
      stat's Live AATF mark then at the cell's left edge (the draft's `.xmark` `right:
      calc(100% - 4px)`), so the marks stay in one column; ov2cf paints it 2 before the
      label's text, which lines up only while the labels are left-aligned. Planned (DECISIONS
      "The ability stats' labels right-aligned, their marks at the cell's edge"); brief
      `.tmp/0_0_1ov/brief_ov2i.md`, after ov7a lands (both touch `abilities.rs`). Built (Opus
      sidekick, report `.tmp/sidelead/sk_ov2i.report.md`): `ability_stats` measures the label
      column (`info_strip::label_width`, now shared) and draws each label right-to-left in it,
      `Marks::paint` given the cell's rect. Tests in `tests/abilities.rs` (right edges at one x,
      12 before the box, centred on its row) and `tests/live_marks.rs` (the marks 2 before the
      column), red on the left-aligned labels and on a mark keyed to the text
    - [x] ov2h the sixth rulings' looks (the maintainer, 2026-10-28; DECISIONS "The final
      pass's sixth rulings"; drafts done, `plans/save_editor.md` "Editing UI", "Tactics
      editor"): the drag handle 4 points from the row's edge and 4 before the place, in the
      player list and the lineup lists (DECISIONS "The final pass's seventh rulings"); the
      formation pictures' range on one row, `Teams` first `to` last; Set boots and gloves IDs
      for everyone in 4ccEditor's layout (`plans/save_editor.md` "Team operations" table's row,
      DECISIONS "Set boots and gloves IDs takes 4ccEditor's layout"; `view/menus.rs`'s dialog,
      `help/05_team.md`); the toolbar in `MenuBar` (hover switches menus) in the order Open,
      Save, Save as | Undo, Redo | Player, Team, Savefile | ruleset, Live AATF, the Panels
      menu's entries moved to the Savefile and Team menus (`view/menus.rs`, the tests that open
      them through `Panels`, `help/02_window.md`; DECISIONS "The toolbar's order, no Panels
      menu, shortcuts right of their labels"); the views' titles 7 below the toolbar, the team
      view's `<id> <name>` (DECISIONS "A view's title under the toolbar, the team's with its
      id"); the team view's name field labelled `Name` (`view/team.rs` `TEAM_LABEL`, which the
      Comparator's and the changes list's rows read too); the lineup lists' position labels in
      the pitch badges' group colours (DECISIONS "The lineup lists' positions in the pitch
      badges' colours"; `team_widgets` `pitch`'s `position_colour` shared, not copied); the toolbar's `• unsaved
      changes` in the link blue (DECISIONS "The unsaved-changes label in the link blue").
      Every draft change of ov2b to ov2h
      is approved (the maintainer, 2026-10-28: "All approved"). Built 2026-10-29 (`1784c17b`, rework 1
      `e1babcce` the three dark-only greys' light colours, merge `c7fab4d2`;
      `studio_core` `widgets/{toolbar,ink}.rs`, `save_editor` `view/{mod,menus,info_strip,
      team,player_list}.rs`, `team_widgets` `pitch/lineup.rs`, help 02, 05, 07), the plan
      corrected on the end box's grey and the titles' route (DECISIONS "The range's end box
      keeps the draft's grey"). Left to the steps that build their features: the toolbar's
      Save as (l3), the Savefile menu's Apply aesthetics patch and the Team menu's Import /
      export team file (8.9k slice 5), Save a picture of the formations and the pictures
      range dialog, its range on one row (8.33, 8.34). Not yet seen in a built window (ov1)
    - [x] ov2j every modal in the draft's `.window` frame (`docs/prototypes/index.html`): 12
      points of margin across and 8 down (egui's modal frame keeps about 6), the title 8 above
      the body (`.window .title`'s margin; the build leaves egui's 3: Settings' first row 20
      under its title against the draft's 28.8, About's body 19 against 24.8, Kit slots' 19
      against 27.8), at least 360 wide, margins included (`min-width`: Kit slots and Set all
      stats shrink to their text), and the buttons in one row, right-aligned, 10 under the body
      (the draft's `window_()`; the build puts Close at the left, where DECISIONS "Set boots and
      gloves IDs takes 4ccEditor's layout" keeps "the buttons right-aligned as every dialog's
      are"); one `studio_core` helper every modal draws through: Settings, About, the first-run
      prompt, the Save editor's and the Ruleset editor's dialogs and prompts. Done 2026-10-29
      (merge `46320cdc`; Opus sidekick, landed first time; lead gates green in its target,
      merged state too: save_editor 447 + 50, studio_core 280, ruleset_editor 110 + 13,
      clippy, fmt): `studio_core::widgets::{modal, modal_frame, modal_buttons}`, all 20
      modals through it, reading orders kept; tests: a short and a wide dialog, Settings,
      Kit slots, the Ruleset editor's prompt (red at HEAD). Its contradictions, accepted: the
      first-run row stands 8 under its title (the draft's margins collapse: its test moved
      from 2); the Ruleset editor's refused-open dialog titled "Not opened" (DECISIONS); the
      Ruleset editor's scroll bound also takes the title off; the drafts' `window_` drew an
      empty button row under a dialog with none (fixed in the drafts, the first run's bottom
      now 9 under its footer as the build's). Rejected: the drafts' settings window capped at
      640 (ov2r had already fixed it; the worktree predated it)
    - [x] ov2j2 the boots and gloves dialog as wide as its groups again (the maintainer,
      2026-10-29, from the built app; DECISIONS "The modal helper's 360-point minimum has a
      contents-wide exception"): `studio_core::widgets::modal_fit_contents`, which
      `boglo_dialog` draws through; headless tests of the dialog's width against its groups'
      (red at 792 vs 752.6) and of a short fitted dialog (`widgets/modal.rs`, `view/menus.rs`,
      `view/tests/team.rs`). Open: `draft_compare.py --modal` pairing a dialog's frame, run
      over every modal in the built-window check
    - [x] ov2j3 the help window's paragraphs 8 apart (DECISIONS "The help window's paragraphs
      stand 8 points apart"): `PARAGRAPH_SPACING` in `shell/help_window.rs`, a headless test
      of two paragraphs' gap (red at 3). Open: `draft_compare.py help --modal .help-win` in
      the built-window check
    - [x] ov2j4 the help's text wrapped at 720 points (P44; DECISIONS "The help's text wraps
      at 720 points"): `READING_WIDTH` caps each paragraph, a headless test of a long
      paragraph's width (red at 898); the Messages table's tests unchanged
    - [x] ov2j5 the help window first opened at 1000 × 600, or 90% of Studio's window where
      smaller (the maintainer, 2026-10-29; DECISIONS "The help window first opens at 1000 ×
      600"): `FIRST_SIZE` in `shell/help_window.rs`; ST-HELP-05's test at 1280 × 800 (red at
      1152 × 720) and 1037 × 648
    - [x] ov2k the form grids' rows 22 apart, the draft's `table.grid` (its `td` padded 2 above
      and below an 18-point control), where egui's `Grid` spaces them by `item_spacing.y`, 3:
      each row lands a point higher than the last, 25 by the Ability stats' last row, 17 by
      Settings' Memory cap. Every `Grid` that draws a draft `table.grid` (Settings' common grid,
      General info, Other stats, Ability stats, the team view's, Kit slots). Done 2026-10-29
      (merge `409274ad`; Opus sidekick, landed first time, lead gates green: save_editor 446 +
      50, studio_core 277, clippy, fmt): `studio_core::widgets::FORM_ROW_SPACING` (4) on
      Settings' common grid and the Save editor's section, General info's and Other stats'
      stacked rows, the Ability stats, the five Appearance grids, Kit slots and the
      unsaved-changes prompt; tests: the ability stats, General info's pairs and Settings'
      rows 22 apart (red at 21). The team view's `info-grid` is one horizontal row, no row
      spacing. Its report's leftovers: the help window's Messages table (ov2q) and the Ruleset
      editor's four grids (its view, not in 0.0.1) also draw a draft `table.grid`; General
      info still stands 4 to 0.5 points above the draft (ov2s)
    - [x] ov2s General info against the draft after ov2k (offsets in the ov2k report,
      `wt-ov2k/.tmp/sidelead/sk_ov2k.report.md`): the draft's Name and Shirt number boxes 19
      tall (ov2r set text boxes and combos to 18, not number boxes), its `.gen` blocks 6 apart
      where the build's are 4, and its Stronger foot and hand toggles plain spans 15 tall where
      the build's are 18-point buttons; the lead's ruling, then the draft, plan and build to it.
      Ruled 2026-10-29 (DECISIONS "General info's blocks 6 apart, its toggles 18 tall"): the
      draft's toggles 18 tall (`save_editor.js`; measured with `.tmp/0_0_1ov/geninfo_offsets.py`:
      every row 22 apart in its block, Height 68.5, Stronger foot 136.5, Other stats 233 below
      the title); the Name and Shirt boxes were already 18 since ov2r. Left for the build
      (ov2q's brief): the blocks 6 apart where it has 4, which brings each label to the draft's
      place. Built with ov2q; re-measured 2026-10-29 on a release build of `4c448424`
      (`rect_compare.py player`): every General info label and box within 1.4 points of the
      draft's
    - [x] ov2l the team view's header as drafted: the stroke line between the fields' row and
      the Tactics, 8 above and below (`teamView`'s `.sep`; the build has none, so the Preset row
      stands 20 higher), and the colour swatches 22 × 16 (`input[type=color]`; egui's colour
      button is 40 wide, which moves the row's right half 31 to 57 points right). Planned
      2026-10-29 (DECISIONS "The team view's line under its fields, the colour buttons 22 ×
      16"); brief `.tmp/0_0_1ov/brief_ov2l.md`, after ov2j lands (same file). Round 1: the
      button landed, the line stopped on the fold (743.5 against 740), and the row's offset
      found to be egui's 8-point spacing too (DECISIONS "The team row's gaps as drafted; its
      line waits for the Tactics' height"; the draft's `.cbtn` set to 22 × 16); rework 1
      `.tmp/0_0_1ov/rework_ov2l.md`: the row's gaps, the line moved to ov2u; rework 2
      `rework2_ov2l.md`: the id boxes 46 (the draft's `.info-grid`). Done 2026-10-29
      (`85cd3c11`, merge `05911b62`; lead gates in its target after merging
      main: save_editor 455 + 50, color_tools 70, team_widgets 181, ruleset_editor 110 + 13,
      clippy, fmt): `ROW_GAP` 6, `SPACE_GAP` 3, `ID_CELL_PADDING` 4, `ID_BOX_WIDTH` 46, the
      row's six controls within a point of the draft's on PES 21 and still on PES 17
    - [x] ov2u the team view's Tactics at the draft's height (the build's stands about 23
      points taller: with ov2l's line its last control ends 3.5 under the pane's fold, the
      draft's 8 over it; measure block by block, `rect_compare.py`), then the line under the
      fields' row as the plan's team view bullet draws it (ov2l's parked
      `wt-ov2l/.tmp/ov2l/separator_test.rs`). Done 2026-10-29 (Opus, brief
      `.tmp/0_0_1ov/brief_ov2u.md`, reviewed; `b7e33105`, merge `ef99c5cb`): measured from the
      Tactics' first control the build stood 9.5 taller, not 23 (the gap above the Tactics was
      15 tighter than the draft's, which hid most of it): the bars 23 tall (`BAR_HEIGHT`, the
      preset selector's 3-point margins), the lineup lists without the group frame's margins
      above and below, Advanced instructions' header row a line of text, the stack's gap 6
      with the item spacing counted. Every block within a point of the draft's (test
      `the_tactics_blocks_stand_where_the_drafts_do_from_the_first_control`); the line 11
      under the row's controls and 10.5 over the Tactics (`LINE_ABOVE`, `LINE_BELOW`), its
      centre measured, so it stands 0.5 higher than the draft's pixel line; the lowest
      stroke at 739.5 against the 740 limit (the build's pane is 719 high, the draft's 720)
    - [x] ov2m the player list as drafted (`plans/save_editor.md` "Player list", "The quick
      actions"): a player row 18 high, its place in the weak colour, the position and name in
      their own columns (the build draws one string, 21 high); the quick actions in two columns
      as wide as the list, 119.5 each, ending at the frame's right edge as the plan says (the
      build's are as wide as their labels, 81 and 105, and end 50 short); Skills' `Goalkeeper:
      0 of 2` in its chip (the draft's `Cards left for` badge). Done 2026-10-29 (`24cb6bf8`,
      merge `f9b7668e`; Opus sidekick, brief `brief_ov2m.md`, rework `rework_ov2m.md`, lead
      gates in its target after merging main: save_editor 451 + 50, team_widgets 181,
      clippy, fmt): rows 18 touching, place/position/name columns 5 apart in a 28-point
      position column, every name through `append_name` in its caller's font (the lists,
      the popover, the strip-again dialog at the body's 12.5, where egui's default 14 put
      DMF past its column), the quick actions in halves, the tier in a widget-filled chip
    - [x] ov2n the Live AATF dock as drafted: its rows in the log panel's well (`.live-body`:
      the extreme fill, the stroke, 6 × 2 margins; the build lists them on the panel), its body
      150 high when it opens (`LIVE.height`; the build's dock opens at the log panel's 220)
    - [x] ov2o the help window's chapter tree 220 wide, its 6-point padding included
      (`.help-win .help-tree`; the build's is 160, so `First run and data location` and
      `Running from a terminal` are cut)
    - [x] ov2p a floating panel (the Comparator, the patch, the interchange) opens where the
      draft's `.panel` stands, 16 from the window's right edge and 90 from its top, 620 wide
      (the build's Comparator opens bottom-left, over the player list). ov2n, ov2o and ov2p
      done 2026-10-29 in one slice (merge `052f4e6b`; Opus sidekick, its first run ended
      waiting on a background test run and a resume finished it; lead gates green in its
      target: save_editor 446 + 50, studio_core 277, clippy, fmt): `widgets::well_frame`
      shared by the log and the Live AATF dock, `BottomDock::body_height` (150 for Live
      AATF, computed from the style), the help tree 220, the Comparator's `default_pos` from
      the content rect (kept where dragged); tests red at HEAD. Its report found the log's
      own body 194 at the 220 default, where the drafts' is 180 (ov2t)
    - [x] ov2t the log panel opens with the drafts' 180-point body (`index.html`
      `logHeight: 180`, the well's outer height), where the build's 220-point panel gives
      194: the log's dock through `BottomDock::body_height(180)` (ov2nop's report,
      `wt-ov2nop/.tmp/sidelead/sk_ov2nop.report.md`), the plan's `Panel::bottom` comment in
      `plans/core/gui.md` (its 220 by default) to the body height. Done 2026-10-29
      (`0c6e30a1`, merge `1e6f577d`; lead gates in its target after merging main:
      studio_core 286, save_editor 453 + 50, clippy, fmt): `BottomDock::new` takes the body's
      first height (the log 180, Live AATF 150), the well 180 on the first frame, ST-LOG-04's
      opening height 206
    - [x] ov2q the small ones: Set all stats' number on its own line under its sentence (the
      draft's `dlgSetStats`; the build sets it on the sentence's line); the first-run prompt's
      two choice buttons with their labels centred; Settings' Memory cap `%` after its box, not
      inside it; the help window's Messages table at `FORM_ROW_SPACING` (ov2k's report); two
      `studio` `team_compiler_cli` tests (`team_compiler_cli.rs:158`, `:198`) that compare a
      printed path with one built from `CARGO_TARGET_TMPDIR` as text, failing when
      `CARGO_TARGET_DIR` is written with forward slashes (compare as paths). Done 2026-10-29
      (`79205499` + `577b38d8`, merge `fb37e0f3`; lead gates in its target after merging
      main: studio_core 285, studio all green, save_editor 453 + 50, clippy, fmt): the
      number under its sentence, the labels centred (`atom_grow`), `%` after the box,
      General info's blocks 6 apart, the sandbox paths rebuilt from their components; the
      Messages rows 19 apart, the draft's measure, where the brief's 20 assumed a 16-point
      row (DECISIONS "A dialog's sentence stands 8 from what is around it; the help's
      Messages rows 19 apart"). Its report found the dialogs' sentences 12.5 from the
      title and the control in the draft, 8 and 3 in the build: ov2v
    - [x] ov2v a dialog's sentence 8 over the control under it (`plans/core/gui.md` "Other UI
      components", Modals; the drafts' `.window p` set to 8 by the lead, rendered
      `.tmp/0_0_1ov/vet/p_setstats.png`): the modals whose body has a sentence over a control
      (Set and Bump all stats, Copy and Swap stats, the others the drafts' `h("p"` lists).
      Done 2026-10-29 (merge of `ov/2v`; Opus sidekick, brief `brief_ov2v.md`; lead gates in
      its target after merging main: studio_core 287, save_editor 455 + 50, clippy, fmt):
      `widgets::modal_sentence_gap` in Set/Bump, Copy/Swap, strip-again and the rules-file
      errors dialog, and (its contradiction, applied by the lead) between the game-running
      prompt's two sentences
    - [x] ov2r the drafts brought to the plan (lead): the settings menu's FPC hide boots and
      gloves IDs (common rows, `plans/core/gui.md` "Settings menu") and the active tool's
      section open on opening (the Save editor's, with Aesthetics from exports), both of which
      the build has and the draft lacks. Done 2026-10-29 (`docs/prototypes/index.html`): the
      two FPC rows after Memory cap, the Save editor's section with its one row (the draft's
      own side-panel control stays the views' GIVEN); a combo and a text box 18 tall as egui's
      (the browser drew them 19, so the drafts' forms of them stood 23 apart, not 22); the
      settings window as wide as its 660-point body (`.window` stopped at 640 and its Browse
      buttons ran into the border); the help window first sized and centred on the client
      area (it measured `#win`, title bar included: 5 taller and 23 higher than the build's
      1152 × 702 at (64, 39), which follows the plan)
    - [x] ov2w the team view against the draft after ov2u, re-measured 2026-10-29 on a
      release build of `4c448424` (`rect_compare.py team`, pixel rows of
      `.tmp/0_0_1ov/{draft,built}/team.png`): the fields' row 1 point high (its boxes 56 to 73,
      the draft's 57 to 74), the line and the Tactics' buttons each another point closer
      (line at 84, buttons 95 to 112; the draft's 86 and 98 to 115), so the whole Tactics 3
      high; the pane's bottom 2 high (the Tactics' last stroke at 739, the draft's at 741,
      the status bar's line at 756 in both); the bars' items (the preset buttons 15 wide for
      the draft's 17.1, 23 apart for its 19.1; the phase tabs about 3 narrower with 8-point
      gaps for its 2; Fluid formation 11 right); Player assignments 2 right. Brief
      `.tmp/0_0_1ov/brief_ov2w.md`. Done 2026-10-29 (Opus, reviewed; `ab4c1f3f`, merge
      `b68e352e`): the panes reach the draft's 6 over the status bar (`PANE_BOTTOM_MARGIN`,
      one cause of the panes' and the quick actions' offsets), the quick actions' band 4 over
      the grid and 0 under it, Set stats' box 52 (the button the rest), the bars' selectors 2
      apart, padded 5, 17 tall (`bar_selector_spacing`), the line by its painted row, the
      assignments after the draft's 320-point pitch track (`PITCH_TRACK`; its canvas
      overflows the track, so 8 from the pitch's border, as drawn); every item within a point.
      The fields' row stands 6 under the title, not the draft's 7: at 7 the plan's even ink
      gaps around the title fail (its toolbar test: 11 above, 12.5 below; the draft's own are
      11 and 14), so the plan wins and what is under the row stands 1 point high, inside the
      tolerance (the lowest stroke at 740 for the draft's 741)
    - [x] ov2y seen by ov2w, outside its brief: the detail pane's right edge at 1264 against
      the draft's 1268, the player list's frame top at 73 against 74. Brief
      `.tmp/0_0_1ov/brief_ov2y.md`. Done 2026-10-29 (Opus, reviewed; `d27a06e9`, merge
      `b3b89a16`): the detail frame's right margin 4 (`PANE_RIGHT_MARGIN`; with the shell's 8
      the view ends 12 short of the window's edge, the draft's `.detail`), the list panel's
      top margin 1 and the list column's controls 4 apart (`CONTROL_GAP`, the draft's
      `.plist` gap), so the combo, the filter and the frame stand at the draft's 30, 52 and
      74. The frame top was 1 point off, inside the brief's own tolerance, but fixed with a
      0.5-point test, as the Goal named it. Left for the re-measure: the list frame's bottom
      679 for the draft's 680 (within 1), the player view's Ability stats ending at 1244.6
      for the draft's 1250.6 (its columns are centred between the list and the window's
      edge, so the pane's margin does not move them). Re-measured 2026-10-29 on a release
      build of `daed1a3d` (`rect_compare.py player|team`, tables
      `.tmp/0_0_1ov/rects/{player,team}_daed.txt`; the Overlay's captures retaken): the
      player and team views within 1 point, 2 where needed (the Tactics bars' choices' hit
      boxes 18 tall for the draft's 20, their fill as drawn; Advanced instructions' "No
      target" 2 high), but the known (checkbox hit areas, the toolbar's Open menu and Save as,
      the ability labels' alignment); the Ability stats' boxes at the draft's pixels in the
      built window (the 1244.6 was the headless probe's)
    - [x] ov2z the sidebar's rail against the draft, seen in the same re-measure (ink rows of
      `.tmp/0_0_1ov/{draft,built}/team.png`, `.tmp/0_0_1ov/rail_rows.py`): the logo, the
      version selector and the line under it 5 points high (logo ink 11 for the draft's 16),
      the selector 40 wide for the draft's 44, the entries 33 apart for its 32, no line above
      Help (the draft's at 681), Help and the gear 3 to 6 low. The rail's order differs by
      design (ov1). Brief `.tmp/0_0_1ov/brief_ov2z.md`, rework `brief_ov2z_r1.md`. Done
      2026-10-29 (Opus, reviewed, one rework; `529f9c37`, merge `094023ed`): the rail's panel
      and the overlay in `rail_frame` (the draft's `.rail` padding 6, not egui's side-panel 8
      by 2), its items 2 apart (`RAIL_GAP`), its lines 9 tall (`SEPARATOR_SPACING`), the line
      above Help drawn, the logo the draft's 32-point square, the only part of its row that
      senses the pointer and fills, every icon centred right of the accent bar's 3 points
      (the draft's border on every entry: x 29 on the rail and opened); six tests. The
      sidekick's two contradictions over the tolerance (the icons 1.5 left, the logo's hover
      box the whole row) were the rework; kept: the selector 43 wide for the draft's 44 (its
      select overhangs its 43-point box), the opened logo at 11.5 for the draft's 12. Seen in
      the built window 2026-10-29 (release build of `5378f359`, after a green checkpoint:
      3,194 tests): the rail's ink rows at the draft's within a point (logo 16 to 27, selector
      40 to 57, lines at 64 and 681, entries 32 apart), Help and the gear within 2 (their
      glyphs); the opened sidebar under the Settings modal's backdrop (ov11d); the Overlay's
      player and team captures retaken
    - [x] ov2x the player view and the text boxes against the draft, re-measured 2026-10-29
      (`rect_compare.py player`): every text box without the draft's 1-point border
      (`input[type=text]`: `border: 1px solid var(--stroke)`, radius 3; egui's resting text
      edit draws none); Playable positions' groups 70 apart for the draft's 66, its rows 42
      apart for 43, its cells 18 tall for 17; Skills' header row 18 tall for the draft's 15,
      so its checkboxes, Tricks and COM playing styles 2.5 low; the Appearance header about 4
      low; the quick actions 4 high (ending at 746, the draft's at 750) and Set stats 56 wide
      for 61.5. General info fits within 1.4 (ov2s's offsets gone). Not mismatches: a
      checkbox's hit area (the draft's label spans its cell; egui's is its content), the
      toolbar's Open menu and Save as (8.9l slice l3), the ability stats' labels (QUESTIONS).
      Brief `.tmp/0_0_1ov/brief_ov2x.md`. Done 2026-10-29 (Opus, reviewed; merge of `ov/2x`,
      the lead's merge commit `2f112988` also wrapping the team view's Name and Short boxes,
      which were ov2w's file then): `studio_core::widgets::text_box` (the border, radius 3,
      egui's outer size; `core/gui.md` the Dark/light theme row) at every text box but a
      test's; positions grid 4 apart within a row, rows 9 apart, label rows a line of text
      (`POS_CELL_GAP`, `POS_ROW_GAP`, `POS_LABEL_GAP`); the Skills header a line of text, its
      items 6 apart (`HEADER_GAP`); the Appearance header a line of text, indented 14, its
      arrow at the draft's place. Rulings on its contradictions: COM playing styles' first row
      stands 1.5 high (the draft's browser checkbox sits 0.5 lower per card panel than the
      plan's 6-point title rule): accepted under "2 where needed", the rule kept; a hovered
      text box keeps egui's hover stroke (the drafts have no hover rule); no clippy lint for
      bare text edits (`text_box` takes a caller-built `TextEdit`, so a lint on its
      constructors would flag every wrapped call); the text's 4-point inset (the draft's 5)
      within the tolerance; other collapsing headers keep egui's look (only Appearance was
      measured) (the maintainer, 2026-10-28): every element rendered from
    the drafts, `.tmp/0_0_1ov/live_gallery.py` → `.tmp/0_0_1ov/live/`; the first rulings
    drafted and planned (ov2f; the dock's kind codes kept). The two points left for later (a
    rules file that fails to load surfaced when it is loaded; switching between rules files,
    awkward) are answered by ov7: a file is validated when imported, and the toolbar's
    dropdown switches. `live_gallery.py`, broken since `cd40003a`, also expects its image 11's
    "fielded at DMF in preset 1" from the PES 17 sample under DBC7, gone with ov9: the
    gallery moves a starter under a ruleset that checks fielded positions instead. Rendered
    again 2026-10-29 (lead): image 06 dropped (no failed state since ov7), image 11 under the
    official rules with team 718's sixth starter made a B where preset 1 fields him (the
    gallery's copy only); looked at by the lead (02, 07, 11). Looked at by the maintainer in
    the built app (2026-10-29): the fielded-position crosses under the official rules and the
    warnings work as drawn; the rows' texts go to 8.29d1
  - [x] ov7 Live AATF replaces the AATF panel (the maintainer, 2026-10-28; DECISIONS "Live AATF
    replaces the AATF panel; the final pass's fourth rulings"; drafts and plan done,
    `plans/save_editor.md` "Configurable AATF rules", SE-GUI-15 and SE-GUI-32 rewritten, their
    tests rewritten with the code). The app side, three slices: ov7a built (Opus sidekick,
    brief `.tmp/0_0_1ov/brief_ov7a.md`, merge brief `merge_ov7a.md`, report
    `.tmp/sidelead/sk_ov7a.report.md`): the built-ins named by their descriptions and
    `aatf::builtin`, `aatf_ruleset` with `session::rules::resolve` for the window and
    `check-aatf` (`aatf_ruleset_gone`), the toolbar dropdown (`view/ruleset_picker.rs`)
    without Import or ×, the panel and the load error gone, the bundle, the Ruleset editor's
    start page labels and a suggested file name of letters and digits alone (`/vg/` read as
    folders); spec's CLI side moved. ov7b built (brief `brief_ov7b.md`, rework
    `rework_ov7b_r1.md`, report `.tmp/sidelead/sk_ov7b.report.md`): `session::rules`'
    `check_import`/`import`/`remove`, the dropdown's `Import rules file…` and each import's ×
    with their dialogs (`view/ruleset_picker.rs`), a re-import differing only in case keeping
    the earlier copy's name (Windows reads the two as one file); the ×'s and the dialogs' look
    not yet seen in the built window (ov1). ov7c built (brief `brief_ov7c.md`, report
    `.tmp/sidelead/sk_ov7c.report.md`): `view/multi_aatf.rs`, `violations_list`'s `ListUse`
    and `issue_groups` shared with the copied text; the dialog's look not yet seen in the
    built window (ov1). The spec move is done: `spec/aatf_rules.md` (the rulesets, the
    setting) and `spec/save_editor.md` "Configurable AATF rules" (the dropdown, No AATF
    panel, Multi-team AATF); the plan keeps the wheel over the dropdown (ov2e), and
    SE-GUI-15/32/41 stay with the phase's other GUI scenarios until its close. The list:
    - the toolbar's ruleset dropdown (`save_editor` view): built-ins by `name`, imports under
      a separator with an ×, `Import rules file…` after another, outside the wheel's steps;
      the import (`rfd` on `.4ccrules`, `Rules::load`, the refusal dialog with the findings,
      the copy into the data folder's `rules/`, the built-in-name refusal, the replace
      confirmation), the × with its confirmation, the fallback notice
    - `aatf_ruleset` replacing `aatf_rules` (`settings.rs`, `lib.rs`'s settings row gone,
      `session/rules.rs`, `cli.rs`'s `check-aatf` default)
    - removed: `view/aatf_panel.rs` and its tests, the Panels menu's entry, the quick actions'
      load error line and its dialog, the badge's no-file case
    - `scripts/release.py`: no `data/rules/`, the staged settings `aatf_ruleset = "dbc7"`, and
      `release_test.py`; the readme's VGL section (pick `/vg/ League 27` in the dropdown)
    - every current ruleset built into the exe (`plans/aatf_rules.md` "Embedded rulesets";
      DECISIONS "The final pass's fifth rulings"), each file's `name` its description
      (`rulesets/*.rhai`; tests naming `Autumn 26` follow)
    - Multi-team AATF in the Savefile menu (`plans/save_editor.md` "Configurable AATF rules",
      SE-GUI-41): the range's combos, the worker run, the list (every team, `All clean` under
      a clean one; DECISIONS "The final pass's sixth rulings"), Copy to clipboard (each team's
      line after a `>`)
    - when built, the spec: `spec/aatf_rules.md` "The rules file" (the pickers), "Embedded
      rulesets", "Loading and validation" (the setting) (moved; distribution's `vgl27.4ccrules`
      gone with ov7a's readme), `spec/save_editor.md` "Configurable
      AATF rules" and "CLI" (the setting `check-aatf` reads)
  - [x] ov8 a violation's sentence suggests a valid value where the check can say it cheaply
    (the maintainer, 2026-10-28; DECISIONS "The final pass's seventh rulings"; Opus sidekick,
    brief `.tmp/0_0_1ov/brief_ov8.md`, report `.tmp/sidelead/sk_ov8.report.md`; spec
    `spec/aatf_rules.md` "The interpreter", Hints at a valid value): `logic.rhai` reads every
    player first (`height_room`, `tier_heights`, `card_summary`'s `rated_a`/`rated_b`,
    `team_setting`'s `wanted`), the logic hash re-pinned; `aatf`'s eight variants with their
    fields and `rule()`'s `[a, b]` lists; `team_widgets`' sentences in the drafts' words. A
    bracket with a quota of 0 is no hint (the sidekick's evidence: Autumn 26's Red system).
    New tests in `fixtures/official.rs`, `vgl26.rs`, `interpreter.rs` and `violations.rs`,
    red on the old logic; on EDIT17 under DBC7 and Autumn 26 only the new fields changed
    (10630 and 7086 lines). A rules file saved with the old logic now fails its check on a
    missing field until its logic is updated (pre-release: no member holds one)
  - [x] ov9 the drafts' FieldedNotA against the check (found 2026-10-28 at ov8): the drafts'
    `fieldedRows` raised one for PES 17's 71822 (DMF in preset 1, rated A at CB alone) under
    Autumn 26 and DBC7, the build's `check-aatf` none. Both read the same rule (all three
    formations of a fluid preset, the kick-off one otherwise); the drafts were wrong twice
    (Opus sidekick investigation, brief `.tmp/0_0_1ov/brief_ov9.md`, report
    `.tmp/sidelead/sk_ov9.report.md` in its worktree, scratch `.tmp/0_0_1ov/ov9/`): their PES
    17 sample predated the save's re-save of 2026-10-25 (preset 1's kick-off had 71822 at DMF,
    now CB), and they checked preset 1 under every ruleset where only Autumn 26, VGL 26 and
    VGL 27 set `fielded_at_a`. Fixed in the drafts: the sample's two teams' tactics
    re-recorded from `EDIT17` (8 lines), `fielded_at_a` on those three rulesets and
    `fieldedRows` reading it. Checked in Chromium through Live AATF's rows
    (`.tmp/0_0_1ov/ov9/fielded_check.py`): no FieldedNotA on the sample under either; 71822's
    CB lowered to B, one under Autumn 26 and none under DBC7 (failing with the old constant)
  - [x] ov10 the pitch drag as the game's (the maintainer, 2026-10-28; DECISIONS "The pitch
    drag follows the game's"; drafted, `plans/save_editor.md` "Tactics editor" "A held
    starter" and the popover, SE-GUI-42): in `team_widgets`' pitch, the held marker snapped
    to whole units with the press's offset kept, the position ruled at every step from his
    stored one (DECISIONS "A held player's position is ruled from the one he had when picked
    up", the maintainer's PES 17 check of 2026-10-29, which also found the swap as the
    editor's) and shown on his badge, the release writing both; the lit area (`RosterRow`
    carrying each starter's A/B ratings); the popover's combo limited to the spot's choices,
    X and Y applying the rule; the formation selector disabled on Kick-off while the shown
    preset's Fluid formation is off (the maintainer, 2026-10-29; DECISIONS "The formation
    selector is locked on Kick-off while Fluid formation is off"). Done 2026-10-29: `164343d7`
    with rework 1 (the rule from the stored position, the lock, the band skip dropped, the
    host's `rated` fill tested), merge `8315c07e`; `team_widgets` pitch (`drop_rule.rs`, new
    `lit_area.rs`, `mod.rs`, `popover.rs`, tests), `tactics/mod.rs`; `save_editor`
    `view/tactics.rs` (`roster_slot`), help 08. Not yet seen in a built window (ov1)
  - [x] ov11 the light theme (the maintainer, 2026-10-29: is it usable? then "avoid writing any
    colors directly, ideally they should all be part of a switchable palette"; DECISIONS
    "Every colour comes from a switchable palette, none written in a view")
    - [x] ov11a the palette (`plans/core/gui.md`, the Dark/light theme row):
      `studio_core::theme::Palette`, 28 fields, `DARK` and `LIGHT` written out in full, and
      `theme::palette(&visuals)`; every colour the 46 sites wrote moved into it (only the four
      drawn on the theme's own panel differ), measuring layouts on `PLACEHOLDER`, image tints
      on `theme::NO_TINT`; tests: the palette follows the theme, the shared fields equal, the
      badge's fill pinned as a literal in a pitch test. `just colour-literals` (the lead's
      check and its test) joins `gates` and `gates-changed`. `807e281f`, merge `685df479`
    - [x] ov11b the light theme looked at: no test or capture has ever drawn a view in it
      and the drafts are dark only. Render every Save editor view and dialog in it
      (built-window captures with the maintainer's yes, the theme switched in the
      sandbox's settings, or headless frames), list what reads poorly, fix it in the
      palette. Known: the players' name colour codes, chosen for nothing in particular or
      for the pitch image's green (the maintainer, 2026-10-29), which both themes share;
      only where a list or a view draws them on the light panel may they read poorly
      (theirs, not ours to change: whether a light theme draws them on a dark chip or
      outlines them is a question for the maintainer once seen); the lineup lists' position
      text in the pitch's colours (`position_colour`, the keeper's `#ffde00`), which ov11a
      gave one value for both themes though the lists draw them on the panel: a light value
      for each by the contrast rule, or a dark chip, once seen. Looked at 2026-10-29 (lead,
      release build of `4c448424` switched to light in the sandbox's settings; captures
      `.tmp/0_0_1ov/light/`: the empty view, player, team, team with Live AATF, Settings,
      Help): what reads poorly is the resting widgets (unchecked checkboxes, radios, number
      boxes, buttons at grey 230 on 248, contrast 1.18 against the dark theme's 1.56), the
      text boxes with no edge (white on 248: ov2x's border, the light stroke grey 190), and
      the lineup lists' position codes (the keeper's yellow at 1.26). Ruled (DECISIONS "The
      light theme's widget fills keep the dark theme's contrast; its text colours read at
      4.5", values from `.tmp/0_0_1ov/light_contrast.py`): code in ov11c, after ov2x lands
      (both edit `theme.rs`). The players' name colour codes: none drawn in teams 701 and
      702 of the sandbox save, the ones captured, so not yet seen in light (they stay the
      maintainer's question once seen)
    - [x] ov11c the light rulings built (sidekick, after ov2x): `theme.rs`'s light widget
      states (resting 201, hovered 186, pressed 208, open 223), the palette's four lineup
      list colours (dark: the pitch's; light: `#847200`, `#007d99`, `#2f8314`, `#d92464`)
      drawn by `team_widgets` `pitch/lineup.rs`, tests of each value's contrast. Brief
      `.tmp/0_0_1ov/brief_ov11c.md`. Done 2026-10-29 (Opus, reviewed; `b2089e48`, merge
      `7f331e33`): `Palette`'s `lineup_keeper`, `_defender`, `_midfielder`, `_forward`,
      picked by `lineup_colour` (the pitch's `position_colour` and it share
      `group_colour`); tests: each light fill within 0.05 of the dark one's contrast, each
      light lineup colour at 4.5 to 4.7 on grey 248, every row's code in its lineup colour
      on both themes; look `.tmp/ov11c/look_light.png` in the worktree (the resting
      controls stand out, the codes read). Seen and kept: the open state's `bg_fill` stays
      220 (open combos and menu buttons fill with `weak_bg_fill`); the panel's grey text on
      a hovered row's 186 reads at 4.15 (the dark theme's 2.8 there); a negative cards
      count's red on the resting 201 at 2.41 (the dark's 2.76); a position past the table
      stays white in the lists (corrupt data only). The light captures again on the release
      build of `daed1a3d` (`.tmp/0_0_1ov/light2/`: empty, player, team, team with Live AATF,
      Settings, Help, Set all stats, the Player menu), each looked at: the resting controls
      and the text boxes' edges read, the lineup codes read in their darker hues, the modal
      and the help read; nothing new for the light theme
    - [x] ov11d seen in the light captures, both themes: with the pointer still on the rail
      after a click on the gear, the opened sidebar (`Order::Foreground`, sent to the front)
      draws over the Settings modal's dimmed backdrop, undimmed, and from a reading of egui's
      `Modal` its entries stay clickable behind the modal; the draft's `#sb-open` is "above the
      view, below menus and modals". Ruled (DECISIONS "The opened sidebar sits below modals";
      `plans/core/gui.md` "Sidebar" item 4). Brief `.tmp/0_0_1ov/brief_ov11d.md`, run beside
      ov2z (the same module, other functions: the layers, not the layout). Done 2026-10-29
      (Opus, reviewed; `97765099`, merge `22fcb3b3`): reproduced headless first (the overlay
      ranked above the Settings modal; a click on an entry switched tools, a click on Help
      opened it, behind the modal); `overlay_order` makes the overlay `Order::Middle` while
      egui reports a top modal layer, `Foreground` otherwise (egui sends a `Foreground` area to
      the front on every open and click, so it could not stay under the modal there); four
      tests (two red first, two guards: About above the overlay and taking clicks, the overlay
      above the Help window). Not tested: a modal shown again in the very frame of a click on
      the overlay could draw under it for that one frame (egui's top modal layer is the last
      frame's), without taking the click
  - [x] ov12 the Tactics popover's X and Y as steppers (DECISIONS "The Tactics popover's X
    and Y step by arrows inside the number box's width" and "The popover's stepper arrows
    take clicks, not the keyboard's focus"; `plans/save_editor.md`, the Tactics editor's
    sentence starting "X (0 to 104) and Y (0 to 48) are steppers"): `team_widgets`
    `pitch/popover.rs`' `stepper`, the arrows click-only, the wheel kept; seven tests (a
    step, PARTY HARD stepped into the forwards' band an LWF, the clamp from 110, the
    disabled ends, the keeper's four, the joined box, Tab from X to Y); `⏴` and `⏵` in
    `theme.rs`' symbols test. Each arrow measures 12.5, its glyph's advance (the box 53),
    within the one-pixel match. `0df81c08` with rework 1, merge `6e546261`. Not yet seen in
    a built window (ov1)
  - [x] ov13 the ability stats' labels left-aligned (the maintainer's pick, 2026-10-29;
    DECISIONS "The ability stats' labels left-aligned, as the approved draft shows";
    `plans/save_editor.md` "Editing UI", the Ability stats item): the draft's CSS made to say
    what it renders (render unchanged, pixel for pixel); the build's labels from
    right-aligned (ov2i) to one left column, the marks still at the label cell's left edge.
    Brief `.tmp/8_9l/brief_ov13.md`. Done 2026-10-29 (Opus sidekick, reviewed; merge
    `8060aeef`): the label a bare cell of the grid's first column, the measured label width
    gone (the grid's own sizing pass sizes the column; Jump's box at 1147.6 before and after);
    `the_stats_labels_start_at_one_x_and_their_boxes_at_another`. Measured on its release
    build with `scripts/draft_compare.py player`: every ability label within 1 px of the
    draft; over 2 px only the known Appearance header and the rail's logo
  - [x] ov14 the text colours the draft comparison measured (DECISIONS "Titles and collapsing
    headers at the drafts' strong grey"; `plans/core/gui.md` the Dark/light theme row): the
    drafts' `.sel` and position labels at egui's 180 (lead); the dark theme's strong text 220
    (the pressed widgets' text colour, which egui's strong colour is) and every collapsing
    header's title in it through `studio_core::widgets::collapsing_header`, five sites (the
    Team compiler's settings added at review); the drafts' pressed checkbox label at the strong
    grey and hovered position label at egui's hovered 240 (the sidekick's contradictions).
    Re-measured on a release build of `3cd0f907`: the player view's ink lists no colour (the
    7 titles and Appearance gone); the titles' weight reads 13 to 21% under the body text's
    (egui's heavier rasterizing shows less on bright text: Skills, at 21%, is listed and is
    the same in the renders). Files: `studio_core/src/{theme.rs,widgets/{mod,
    collapsing_header}.rs,shell/{help_window.rs,settings_menu/mod.rs}}`,
    `ruleset_editor/src/view/sections/tiers.rs`, `save_editor/src/view/mod.rs`,
    `team_compiler/src/view/mod.rs`; renders `.tmp/8_9l/pairs/colour_*.png`.
    manual: checked 2026-10-30 by the maintainer on the candidate: the windows' titles at 14
    in the strong grey "look fine"
  - [x] ov3 a general overlap test: a headless pass over every modal and view failing when two
    widgets' rectangles intersect (catches collisions, not a draft mismatch: ov1 does that;
    `plans/core/gui.md` "No two widgets overlap"; Opus sidekick, brief
    `.tmp/0_0_1ov/brief_ov3.md`) → verify: the test passes on main once every collision it
    finds is fixed or listed with its reason; a perturbation (a field widened over its
    neighbour) fails it. Done 2026-10-29 (`8ce77ba6`, merge `911d55ee`; report
    `.tmp/sidelead/sk_ov3.report.md` in `wt-ov3`): `save_editor` `view/tests/overlap.rs`, eight
    tests over 23 screens on PES 21 and 17 (Strip again on 21 alone), reading egui's clipped
    rects one layer at a time (DECISIONS "Correction: the overlap test reads egui's clipped
    rects, one layer at a time"). It found egui's floating scroll bar over the right end of the
    Live AATF dock's and the Multi-team AATF list's rows: both take the solid bar now
    (`view/{live_aatf,multi_aatf}.rs`). Three pitch-marker overlaps are listed as meant.
    Perturbations: the Name field 200 wider fails it (28 collisions: the column pushed into the
    scroll bar; 4 with the row's spacing made negative, the field over its label), the
    containment rule dropped (320), a stale meant entry. Not covered: an open combo's popup, a
    prompt long enough to scroll, Live AATF's no-issues state (only the PES 15 fixture has a
    clean team), the boglo dialog's other radio state
  - [x] ov4 8.20's host control (lead, drafts): the captures in
    `docs/prototypes/assets/captures/`, the Overlay control over the draft (8.20's brief,
    `.tmp/8_20/brief_8_20.md`, written for a worktree since gone: rebase it on main)
    - [x] ov4a the Overlay control (lead): `index.html`'s Layout group, `overlay=over|beside`
      in the link, the capture over the window body at half opacity (clicks through) or
      beside the window level with its body, a missing one naming its file;
      `assets/captures/README.md`; `bundle_drafts.py` keeps the folder's literal (test red
      first: "an extension with no MIME type"). Checked on a scratch copy with a stand-in
      (`.tmp/0_0_1ov/overlay_check.py`: over at (0, 0) 1280 × 780, beside level with the
      body, the missing line). `acc23548`, merge `670f46dc`
    - [x] ov4b `scripts/draft_shots.py` and each tool script's `shots` list (8.20's
      deliverables 2-3, sidekick). Done 2026-10-29 (Opus, brief `.tmp/0_0_1ov/brief_ov4b.md`,
      reviewed; the lead added `shots` to the host's view contract comment): 55 links, 110
      shots at the default and smallest window on a 1920 × 1080 monitor, the page clock
      frozen; a link the draft does not write back is an error. A one-line `.window` padding
      edit changed 50 shots, every dialog's; two identical runs changed none. Not listed: the
      Ruleset editor's `not-saved` (no template has a problem) and the Save editor's
      `picture-save-settings` (its window writes back `picture-save`). Its test needs Pillow,
      so it stays out of the gates. `4e156a27`, merge `83a25a08`
    - [x] ov4c the first captures (lead): the Save editor's screens from the release build
      after the next checkpoint, each dated in its corner, by a capture script beside
      `rect_compare.py`. Done 2026-10-29 on a release build of `4c448424` (checkpoint
      `gates-changed c6d17d9b` green, 5,206 tests): `save_editor_player.png` (player 701.1)
      and `save_editor_team.png` (team 701), PES 21 sandbox save, Live AATF off as the draft's
      default, by `.tmp/0_0_1ov/capture_built.py` (`gui.py shot`, the date stamped). Both
      load in the Overlay over and beside (`.tmp/0_0_1ov/overlay_real.py`); the team's shows
      ov2w's bar offsets doubled. The sidebar's rounded bar (z3) seen in the same build
  - [x] ov5 the vetting renderer into `scripts/` (the maintainer, 2026-10-28: "useful enough
    to keep"): `.tmp/0_0_1ov/vet.py`, which renders a draft change's before and after from
    `git archive` exports side by side (stacked when side by side would be over four times
    as wide as tall: the maintainer, 2026-10-28), each sheet carrying the change's reason under
    its caption (the maintainer, 2026-10-28), generalized from its cases list to a
    revision pair and a link, with its test. Done 2026-10-29 (merge `4daed600`; Opus sidekick,
    brief `.tmp/0_0_1ov/brief_ov5.md`): `scripts/vet_drafts.py --before REV --after REV
    --link FRAGMENT --why TEXT [--crop CSS] [--pad N] [--click TEXT]`, exports under
    `.tmp/vet_exports/` (the lead's change from the brief's fixed Windows path), and
    `vet_drafts_test.py` (7 tests, run by hand: it needs Pillow, which `just test` does not
    install); `vet.py`'s one-case crops (a combo's popup, an uneven box) are not reproduced
- [x] 0.0.1rn the project renamed Unlimited Tool Works (the maintainer, 2026-10-28: "when most
  convenient"): `unlimited-tool-works` as the name in paths and packages, UTW as the shorthand,
  the exe `UTW.exe` (`utw` on Linux; DECISIONS "The final pass's second rulings"). Before the
  cut is the convenient time: the first release fixes the exe's name, the data folder
  (`%APPDATA%\4cc-studio`) and the bundle's name, which a later rename would have to migrate.
  To plan first: every user-visible name (exe, data folder, bundle, window class, CLI command,
  help text), the crates' and repository's names, the CI jobs, the docs. The local folder
  keeps the working name `4cc-studio` until 1.0.0 (the maintainer, 2026-10-28). Planned
  2026-10-29: `plans/core/development_plan.md` "The project's name: Unlimited Tool Works",
  DECISIONS "The project's name in every spelling"; inventory `.tmp/0_0_1rn/inventory.md`
  - [x] rn1 the code and the files a user receives (a sidekick): the two name constants
    (`exe_name!` by platform), cargo's bin `utw` (`CARGO_BIN_EXE_utw`, `utw.pdb`), the
    settings folder, `utw.log`/`utw.old.log`/`utw.dmp`, the app id, `Tvers`, the shared-IDs
    folder, `tool_version`, the help pages, the launchers, the readme, `build.rs`, the release
    recipe and its test, the justfile and the scripts naming the exe or the folder; the
    `help_cli` guard widened to `UnlimitedToolWorks` and the old folder's name. Done 2026-10-29
    (merge `c6d17d9b`; Opus sidekick, landed first time; lead check in its target: studio 9
    test binaries, studio_core 277, clippy, fmt, `release_test.py`, `symbolize_log_test.py`):
    `Tvers` from one `studio_core::TOOL_VERSION`; the first-run layout test's sample path renamed
    by the lead, its hint allowed three lines (the draft's own wraps to three with the new
    folder name). Its contradictions: `--version` prints `UTW v…` as the spec has it (the plan
    corrected); the plan's stale "the icon switches back" removed; `LICENSE-MIT`'s "the 4cc
    Studio contributors" and the spec's `studio_version` sample go to rn3
  - [x] rn2 the drafts (the lead): `index.html`'s title, exe and folder names, help text
    (`UTW.exe`, `unlimited-tool-works`) and the draft controls' "Studio"; the Ruleset editor
    draft's file header and logic banner copied from the code's (no product named), its
    window titled as the app's; the Export upgrader draft's sample folder; the changelog's
    shared group `**General**` (`plans/core/distribution.md`, was `**Studio**`). The drafts'
    code comments keep "Studio" until rn3. Files: `docs/prototypes/{index.html,
    tools/ruleset_editor.js,tools/export_upgrader.js}`
  - [x] rn3 the docs (the lead, after rn1 lands): every plan and spec file's title, the CLI
    examples (`UTW <tool-id> ...`), the settings folder's and the log's names, "Studio" as the
    project's name in prose; README, AGENTS, CONTRIBUTING, RUNBOOK; the glossary's "Studio"
    entry kept for the code's names. Done 2026-10-29 by `.tmp/0_0_1rn/rn3_docs2.py` (dry run read line by line):
    "4cc Studio" → Unlimited Tool Works in every plan/spec title and prose, `UTW` in the CLI's
    examples, `UTW.exe`, the `unlimited-tool-works` folders, `utw.log`, `tool_version`, the
    shared-IDs path, `LICENSE-MIT`'s contributors; the rename's own plan section, the history
    files and the repository's and developers' folders untouched. The bare "Studio" stays the
    working name in prose (the plan amended: rewriting ~250 of them buys a member nothing and
    sets the docs apart from the code); GLOSSARY rows for both names
  - [x] rn4 the repository renamed `the4chancup/unlimited-tool-works` on GitHub (the
    maintainer, 2026-10-29); `Cargo.toml`'s `repository` follows, the root README's status
    brought to the 0.0.1 scope
- [x] 0.0.1g the bundle for DBC alone (the maintainer, 2026-10-29; `plans/core/development_plan.md`
  "Divegrass Before Christmas", "The /vg/ League 27 (VGL)"; DECISIONS "0.0.1 is the Save
  editor for DBC alone; VGL's tools wait for 0.0.2"): `scripts/release.py` stages neither
  launcher nor `exports/` and `old_exports/`, `resources/bundle/data/settings.toml` picks PES
  2017 (`pes_version`), the readme loses its VGL section and the "pick your PES" step, VGL27's
  rules file and the compiler's settings lines kept; `release_test.py` follows → verify: the
  staged bundle's file list; the readme read through once. 0.0.2 restores the launchers,
  folders and VGL section from git as they are now
    Done (`7f8de3b9`, merged `e8d3ca2f`): `release.py`'s `LAUNCHERS` and `EMPTY_FOLDERS`,
    both empty, drive the staging, the launcher check and `expected_entries`;
    `data/settings.toml` opens `[common]` with `pes_version = 17` (`studio_core`
    `the_bundled_settings_start_on_pes_2017` proves the app reads it there); the readme
    rewritten for DBC's testers; `release_test.py` 12 tests, with
    `test_the_bundle_ships_no_launcher_and_no_export_folder`, which 0.0.2 rewrites (not
    deletes) when it names the launchers again. The staged list is checked by the tests and
    the listing test; a real `just release` staging is 0.0.1f's. SWE-2 round: 0 counted
- [ ] 0.0.1f the cut, once the Save editor's view is done (the Ruleset editor left 0.0.1 on
  2026-10-24, `plans/core/development_plan.md` "Development Plan"): the bundled settings picking
  DBC7, built into the exe with the frozen logic version 1, as the Save editor's `aatf_ruleset`
  (ov7: no rules file beside the exe); the clean-machine check (`plans/core/development_plan.md`
  "Release 0.1.0"'s verification, both launchers included), the exe's size measured crate by
  crate with `cargo bloat` (35.3 MiB at 0.0.1d2, past the budget's "about 30 MB";
  `spec/core/distribution.md` "Distribution: portable .7z bundle" gets the new measurement),
  the readme's steps for VGL's organizer (`plans/core/development_plan.md` "The /vg/ League 27
  (VGL)": `upgrade-dpfl` once for an install with another list, `multicpk_mode` in the
  settings file), logic version 1's hash
  frozen in `LOGIC_VERSIONS` only after the maintainer's final confirmation, given once the UI
  is polished (the maintainer, 2026-10-25: never before it) (`plans/aatf_rules.md` "Loading and validation": a later logic
  change is version 2), `just release 0.0.1`, handed to DBC's testers and VGL's organizer
  - [x] f1 the preparation (side-lead U, 2026-10-27; DECISIONS "The bundle's `data/` holds its
    settings and DBC7's rules file"): `just release` stages `data/settings.toml`
    (`resources/bundle/data/`: `aatf_rules = "rules/dbc7.4ccrules"`, `multicpk_mode` and
    `cpk_name` at their defaults) and `data/rules/dbc7.4ccrules`, joined from `rulesets/dbc7.rhai`
    and `logic/logic.rhai`, refused unless the staged exe's `ruleset-editor validate` and
    `update-logic` pass it (`scripts/release.py` `stage_data`, 5 new or widened tests in
    `release_test.py`). Checked on the release exe: the staged folder holds those files alone;
    each check refuses a perturbed copy; `save-editor check-aatf` from it without `--rules`
    prints what `--rules data/rules/dbc7.4ccrules` prints (DBC7's medal systems). The readme:
    the Ruleset editor greyed out too, no first-run question (the bundle is portable), DBC's
    testers pick PES 2017 first (j2), the DBC rules file, and "For VGL's organizer"
    (`upgrade-dpfl` then `--yes`, `multicpk_mode` for the first DLC, `cpk_name` a midcup for a
    matchday). `cargo bloat` at 745ea9f4: 37.6 MiB (39,380,992 bytes), the growth the tools and
    the GUI (`spec/core/distribution.md` "Size budget"; tables `.tmp/0_0_1f/head745/`). The
    clean-machine check's "first-run dialog" does not apply to this bundle: its settings file
    makes it portable, so the first start asks nothing. Review: SWE-2 S1 3 of 7 accepted (two
    routed: `plans/core/gui.md` and `plans/core/distribution.md` still list the old bundle;
    a block's own CUSTOM CHECKS now kept), `.tmp/0_0_1f/rulings.md`
  - [ ] f1c the maintainer's final pass over the built app's UI, before anything is frozen
    (the maintainer, 2026-10-28: "wait for one final pass of the UI from me, before it goes
    golden"), once every UI step of 0.0.1 has landed (w4b, w6b, z2, y4e, y5b, y6b, y7b and the
    lead's built-window checks, which ask before they take focus). The candidate for it
    staged 2026-10-29 (`QUESTIONS.md` "The final pass over the built app's UI"): branch
    `release/0.0.1-candidate`, the bundle unpacked by `.tmp/0_0_1ov/make_candidate.py` at
    `4cc-studio-temp/candidate-0.0.1/` (its file set checked against `release.py`'s
    `expected_entries`; a copy started and closed cleanly). Work goes on past it meanwhile
    (worklog "Next"); a change the pass asks for lands on main and a new candidate is staged.
    Before it: the sidebar's
    flicker on open, which the maintainer saw when the collapsed sidebar expands under a
    hovering pointer (2026-10-28): fixed by the lead the same day. The rail's widgets left on
    the opening frame while egui faded the overlay `Area` in over its 0.2 s `animation_time`
    (opening frame at alpha 0x15); the overlay now has `fade_in(false)`, test
    `the_overlay_opens_opaque_on_every_open` (red: frame fill `#02_02_02_15`, not the panel's
    `#1B_1B_1B_FF`). Built window checked 2026-10-28 (release at `f1ca2342`,
    `.tmp/0_0_1z/shots/02_hover_sheet.png`): two hover opens captured frame by frame, each
    going from the rail to the full opaque overlay in one frame, no dim or half-empty frame
    between. In the same
    pass: 0.0.1v's open look outside the app's own windows (the taskbar button, Alt+Tab,
    Explorer's file icon on the exe) → QUESTIONS "Before the release"
  - [ ] f2 logic version 1's hash frozen (on the maintainer's confirmation, after f1c), `just
    release 0.0.1`, the clean-machine check, the hand-over

### Phase 10 — Music tools

The phase's steps are itemized when Phase 9 closes; its format crate was built ahead of it:

- [x] 10.0 `libs/music_export`: the `.4ccm` format crate (`music_player.md` "The `.4ccm` format")
  - [x] 10.0a model, tokenizer, parser, serializer, 7 fixtures, census over the machine's 61
    `.4ccm` (56 parse and read back equal, 5 editor history copies refused for unknown keywords,
    as Rigdio refuses them); `indexmap` enters the workspace; `doc_pointers.py` lists the music
    player plan as open. Files: `crates/libs/music_export/{Cargo.toml,src/{lib,model,tokens,parse,write}.rs,tests/{fixtures.rs,census.rs,fixtures/}}`,
    `scripts/provenance/music_export/list_4ccm.py`, `scripts/doc_pointers.py`, `Cargo.toml`,
    `docs/spec/core/README.md`
  - [x] 10.0b `MatchState`/`Side`/`TeamState`, `Verdict`, `Condition::eval` and `SongEntry::eval`,
    `needs_goal_minute`. Files: `src/{state,eval}.rs`
  - [x] 10.0c `PriorityList::for_slot`/`select`, `Press`, `Loaded`; 9 selection scenario tests.
    Files: `src/select.rs`
  - [x] 10.0d `resolve_song`/`resolve_export`/`Resolution` (native only): the plan's resolution
    order, names matched case-insensitively in the file's own folder and returned as on disk
    (a lead fix: the brief's exact-path shortcut returned the export's spelling on Windows);
    `select`'s skip follows its entry past an unload. SWE-2, first attempt, red runs pasted.
    Files: `src/{resolve,select,lib}.rs`
  - [x] 10.0e the plan's "Model" section carries every type and signature as built (the
    lookup's shapes under "File resolution"), the `pub` audit (`.tmp/10_0e/pub_audit.md`: six
    items with no planned consumer made `pub(crate)`), the design-tell sweep (one triple scorer
    lookup folded, one discarded `Result` made a guard); `just mutants music_export`: 310
    mutants, 282 caught, 26 unviable, 2 timeouts (real infinite loops in `select`'s counter),
    0 missed; the extra kinds' 6 survivors became 4 tests and one simplification
    (`resolve_song` joins the subfolder once), the rerun catches all 26; SWE-2 round S1 4
    accepted (an anchored path's rule named and pinned, the lookup's rules and order stated),
    loop ended; Astra round A1 (top lead, on main) 4 accepted: the `_normalized` companion by
    stem of any extension (IZAKAYA's `.mp3`/`.opus` pairs), the root-level anchor, the
    `advance` skip left out of the `randomise` draw, the `keyword()` sentence narrowed, each
    with a red-run test (DECISIONS 2026-10-18); SWE-2 round S2 on the fix 6 accepted (Unicode
    folding of the companion, `sub/` resolving nowhere, every candidate ranked, a subfolder
    companion test), S3 2 accepted (loop ended); rulings `.tmp/10_0e/rulings.md`. Side-lead B,
    branch `side/10.0e`. Files:
    `docs/plans/music_player.md`, `docs/DECISIONS.md`,
    `crates/libs/music_export/src/{lib,model,eval,state,select,resolve,parse}.rs`
- [x] 10.0f the legacy-behavior audit (the maintainer, 2026-10-21, after SE-GUI-08's reorder
  was found written from a control's name; `AGENTS.md` "Before a tool phase"): every behavior
  of Rigdio v2.2.0 (`Tools_4cc/rigdio`; the fork only where the plan adopts it) compared with
  the crate's code and the plan, citing both sides: the `.4ccm` keywords, conditions,
  selection, resolution and normalisation first (built), the player's controls second (planned
  only). Each "different" or "not stated" row ruled: a plan fix and a decision entry, a code
  fix with its test, or recorded as deliberate. Before the tool's Acceptance section is
  written. Audit done 2026-10-21 (`.tmp/10_0f/rigdio_audit.md`, with the lead's rulings):
  part 1, the built crate, has eleven rows to match to Rigdio (among them `once` spent when
  evaluated rather than when reached, so `goals >= 2;once` never plays, verified at
  `select.rs:112` and Rigdio's `legacy.py:364-370`; the fallback `goal` entries cloned per
  button where Rigdio shares one list; `randomise` and `advance` over the fallback; a leading
  BOM; an anchored song path), plan fixes, and deliberate departures to mark; part 2, the
  player's controls, ruled when the tool's plan is written. Plan edits done 2026-10-21
  (DECISIONS 2026-10-21, "A `.4ccm` reads as Rigdio reads it"). Files: `docs/plans/music_player.md`,
  `docs/plans/music_export_editor.md`, `docs/DECISIONS.md`
- [x] 10.0g the crate to the audit's rulings (`music_player.md` "Grammar", "Conditions", "File
  resolution", "Model", "Song selection"): in-order conditions and `once` spent when reached;
  `TeamLists` with one shared `goal` fallback per team, replacing `PriorityList`; the
  fallback's `randomise` group and warcries; `advance`'s last resort; `needs_goal_minute` as a
  walk without the minute; `name` first, flags case-insensitive, a byte-order mark dropped;
  anchored paths under the export; `[]` as no token, never written; `SecondTeamName` gone (a
  second `name` is a reserved word). Done 2026-10-21 (SWE-2, a run and a rework): a red-run
  test per rule; the census unchanged (56 of 61 read back equal, the same 5 refused: no real
  file uses a changed rule). Files: `crates/libs/music_export/src/{eval,select,parse,tokens,
  write,resolve,state,lib}.rs`
  - [x] review (checkpoint b, a new `pub` interface): `just mutants-diff`, a SWE-2 loop
    (`.tmp/10_0g/swe_s1.md`), then Astra. The mutation run's survivors and SWE-2 S1 (5 of 10
    accepted: the ended entry named by a stable `EntryId`) fixed in `dff1548f`; S2 (2 of 5:
    `knockouts` only alone, `has_once` without a catch-all) ends the SWE-2 loop, its fix in
    `5cf02863` (its mutation run: 5 caught, 1 unviable); Astra A1 (2 of 3, all test gaps)
    ends the Astra loop, its tests in rework 4; the closing SWE-2 round S3 (0 accepted, a nit
    fixed in passing) ends it; rulings `.tmp/10_0g/duck_rulings.md`. Done 2026-10-21
- [x] 10.0h the Music export editor's legacy-behavior audit (the maintainer, 2026-10-27: tool
  work prepared ahead while 0.0.1's build queue is busy; `AGENTS.md` "Before a tool phase"):
  every behavior of RigDJ (`Tools_4cc/rigdio` at `ad604a1`, rigdio v2.2.0, RigDJ v1.16.0:
  `rigdj.py` and what it calls) compared with `plans/music_export_editor.md`, citing both
  sides, each row ruled into the plan and DECISIONS. 42 rows (`.tmp/10_0h/rigdj_audit.md`,
  with a census of the machine's 67 `.4ccm` files): 21 same, two matched (warcries take
  Randomise Horns' `randomise`; an empty scorer or label means no argument), the rest
  deliberate, none for the maintainer. Plan "Background", "What the editor does", "New
  capabilities", "GUI view", "Testing"; four DECISIONS entries; player reordering proposed
  (PROPOSALS P40)
- [x] 10.0i 10.0f's part 2, Rigdio's player controls against `plans/music_player.md` (the
  rows 10.0f marked "later"), ruled the same way, plus the controls part 2 missed. Done
  2026-10-27 (Opus, `.tmp/10_0i/rigdio_controls_audit.md`): 22 + 19 rows, 10.0f's
  descriptions held but for one fade case; ruled into "Playback semantics" (⟲, the fade
  re-press, `speed`, Chaoshorn and Kill, sync and `pause restart`, the VA dropdown and
  timer, chants, Undo's one level, Reset's confirmation, a load zeroing both scores,
  title.log), "Event clips revived", "Loudness normalization" (the boost, the Normalize
  swap), "GUI view" (loading, missing files, the volume sliders, the chants panel),
  "Settings"; DECISIONS "The Music player's controls, ruled against Rigdio v2.2.0";
  three questions in QUESTIONS "Cup practice"

### Phase 12 — Kit config editor

The phase's steps are itemized when it opens; prepared ahead:

- [x] 12.0a the tool's GUI plan against Kit Manager's recovered behavior: the format and its
  constraints were recovered by disassembly before the repository existed
  (`spec/kit_config_editor.md` "Relationship to the old codebase"; the scripts were not kept),
  so the audit compares `plans/kit_config_editor.md`'s GUI rows with the spec's recovered
  controls and constraints, and re-reads the exe (`Kits/Configs/KitmanagerV4.exe`, capstone)
  only where a GUI behavior is not in the spec. Done 2026-10-27 (Fable,
  `.tmp/12_0a/kitmanager_audit.md`, scripts in `.tmp/12_0a/scripts/`): 43 rows, the format
  right in every offset and bit the code touches; ruled into the plan (Saving, numeric
  fields, the name's gating, texture names in the loose-file and container modes, the
  parity test's exclusions; DECISIONS "The Kit config editor saves on Save, and the rest of
  Kit Manager's audit"), the spec
  (collars 1–106 or 1–109, the unreachable cut-out rule, the spacing control's bits, v5.0),
  PROPOSALS P41-P42
- [x] 12.0b in game (PES 2016, pad and OBS): a kit with undershirt-only long sleeves
  (0x02 = 0xBB) and a long-sleeve badge position set: does the badge draw? Stopped
  2026-10-27 by the maintainer (too long for a corner of a tool not yet built), on what it had
  found (Opus, `.tmp/12_0b/findings.md`, /ck/ 0xBB against 0x3E): 0xBB is read (an undershirt
  sleeve to the wrist), and no badge draws on the undershirt forearm in Edit mode or a cup
  match, one drawing only at the shoulder of the short sleeve over it. Ruled: the editor
  grays the four fields, as Kit Manager does (plan, the GUI paragraph). Save and test slot
  restored by hash after the stop. Files: `plans/kit_config_editor.md`;
  `scripts/ingame/install.py` and `README.md` gain PES 16's test slot

### Phase 13 — Refs arranger

The phase's steps are itemized when it opens; prepared ahead:

- [x] 13.0a the plan's readiness audit: `plans/refs_arranger.md` against what it relies on
  (`aesthetics_export`'s built referee handling, FoxDen's referee hook as the plan cites it).
  38 rows (`.tmp/13_0a/refs_audit.md`): 23 hold, 7 differ, 3 unbuilt, 5 only the game can
  settle. FoxDen's module forces a uniform draw with no lists file, so the plan's "draws as an
  unmodified game" is gone and the no-file draw asked; the hook's timing, PES 2020 branch and
  stadium written in; the roster check, the `players.txt` writer and the disabled-marker test
  move into `aesthetics_export`, the replace helper into `studio_core`; the loading and saving
  edge cases ruled. Plan "The Fox referee hook" to "Open questions", development plan Phase 13;
  DECISIONS "The Refs arranger's plan checked against FoxDen and the export library";
  QUESTIONS "Cup practice" (three)
- [ ] 13.0b in game, when the phase opens (PES 2021 with FoxDen copied in for the test and
  removed after, pad and OBS): which of positions 0..4 is the main referee, the linesmen and
  the other two (`plans/refs_arranger.md` "Open questions"); FoxDen's files are not in any
  game folder today

### Phase 14 — Balls compiler

The phase's steps are itemized when it opens; prepared ahead:

- [x] 14.0a the legacy-behavior audit: every behavior of the 4cc balls compiler 2.03
  (`Tools_Mine/4cc balls compiler/4cc balls compiler 2.03`: `balls_compile.bat`, its engines'
  scripts, `Ball.bin`'s offsets) compared with `plans/balls_compiler.md`, citing both sides,
  each row ruled into the plan and DECISIONS. 49 rows (`.tmp/14_0a/balls_audit.md`): 19 same,
  the rest deliberate or matched, none left for the maintainer (the CPK-name question answered
  from the cup's PES 2021 install: `4cc_38_balls`). `Ball.bin` and `BallCondition.bin`
  re-decoded by the lead: the former as the plan had it, the latter's description corrected.
  Plan "The balls export", "Compilation pipeline", "GUI", "CLI", "Testing"; core plan
  Phase 14's verification; five DECISIONS entries

### Phase 16 — Polish and distribution

Steps are itemized when Phase 15 closes; three are fixed already:

- [x] 16.w the release exe's platform floor and size (maintainer, 2026-10-15; taken up right
  after 5.3b3's review, not at Phase 16). A survey of 2026-10-14 (`.tmp/win7/`) found the
  exe cannot load on Windows 7 (Rust std's Win8+ imports and `windows-core`'s `combase.dll`)
  and needs the VC++ redistributable, which no doc names; the maintainer ruled Windows 7 out
  (its last members moved to Linux). Slices:
  - [x] 16.w1 `docs/spec/core/distribution.md` states the floor, Windows 10 x64 (and Linux),
    with the reason (the Rust target and the GUI stack require it) → verify: `just doc-pointers`.
    Done: the "Platform floor" paragraph, DECISIONS 2026-10-15
  - [x] 16.w2 `+crt-static` for the Windows release build (measured +327 KiB, 1%), so the exe
    runs without the redistributable → verify: the release exe's import table has no
    `VCRUNTIME140.dll` and no `api-ms-win-crt-*` (`.tmp/win7/imports.py`); the GUI starts.
    Done: `.cargo/config.toml` (every MSVC build); 7 CRT imports before, none after; +225 KiB;
    the GUI started hidden, alive and responding with its window after 8 s
  - [x] 16.w3 the size growth (17.1 MiB on 2026-10-02 to 30.95 MiB on 2026-10-14): find its
    cause with `cargo bloat` (approved as a developer tool), make the low-hanging reductions
    that also shorten builds, and update the spec's measurement → verify: before and after
    sizes and clean release build times, measured the same way. Done (DECISIONS 2026-10-15):
    the growth is features (`.tmp/16w/`: the 2026-10-02 commit's crate table against today's);
    `rhai` `only_i64` -627 KiB; 32,723,968 bytes and 116 s before, 32,311,808 bytes (static
    CRT included) and 124 s after, one clean build each; the spec's "Size budget" updated

- [ ] 16.x `dds_convert` GPU BC7 (deferred from 2.5b to Phase 4, decision entry 2026-09-21, then
  past Release 0.1.0, decision entry 2026-09-28; spec `libs/README.md` "First-release desktop GPU
  BC7"): `block_compression`'s wgpu backend on a Vulkan/Metal device, CPU fallback with the
  fallback reason reported, cold pipeline creation and upload/readback measured first, then the
  texture step's bounded batches → verify: GPU and CPU outputs decode within the same tolerance
  on the `dds_convert` fixtures; the fallback path exercised by forcing no adapter
  - [x] 16.x a `dds_convert`'s half, built ahead and unwired (2026-10-19, DECISIONS "the GPU
    BC7 encoder is built ahead of 0.1.0"; measurements `.tmp/16x/findings.md`: Vulkan 0.48 s
    per 4096 chain against 16.1 s on 16 CPU threads, same PSNR; DX12 refused): `gpu::GpuEncoder`
    (Vulkan/Metal, 4-Mpixel slabs per submission, error scopes, a device failure sends that mip
    and everything after it to the CPU, a texture past the device's limit goes to the CPU
    whole), `convert_with_gpu`, `pad_to_blocks` shared, a wasm stand-in; `just gpu-tests` runs
    the `#[ignore]`d GPU tests (CONTRIBUTING); mutation run over the diff with them
    included, survivors tested or excluded (`.cargo/mutants.toml`). Files: `Cargo.toml`,
    `crates/libs/dds_convert/{Cargo.toml,src/gpu.rs,src/encode.rs,src/lib.rs}`, `justfile`,
    `.cargo/mutants.toml`, `docs/CONTRIBUTING.md`, `docs/plans/libs/README.md`
  - [ ] 16.x b the Team compiler's texture step on the encoder, after 0.1.0: the CPU starts at
    once and the GPU joins when its device is ready, started when the run has BC7 work
    (`plans/libs/README.md` "First-release desktop GPU BC7"); start it after the window's
    renderer is up, a failed start this run's CPU fallback with its reason in the report
    (`.tmp/16x/findings.md`, the transient adapter failure seen under load). The system log
    (checked 2026-10-21) has no driver reset at any of those failures (04:07, 04:15 and later
    on 2026-10-19); that day's one reset, 03:16:41, which Adrenalin reported twice (two
    `LiveKernelEvent` reports), was the probe's single-submission DX12 run of an 8192 chain.
    So the adapter blip is not a timeout: most likely the Vulkan loader with several processes
    opening it at once, unexplained
  - [ ] 16.x c the BC7 speed probe, after all of DBC's work (the maintainer, 2026-10-21;
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

- [x] 20.0 The Ruleset editor's GUI draft, right after 5.2c3-5.2e1 land with their review
  rework, before 5.2e-2's VGL/VTL rulesets (moved up by the maintainer, 2026-10-14): the draft
  needs only the schema, and input that changes the format costs less before more rulesets are
  written against it. Its purpose is to show the members that a Rhai rules file is a sound
  choice: the GUI makes it easier to edit than a simpler hand-written text format would be,
  while the file stays readable. So beside the forms the draft shows the ruleset's data block
  read-only, updated as a field changes.
  - [x] 20.0a coverage of a contributor's proposal (Clouds, written without knowing Studio;
    the maintainer's copy is `.tmp/Clouds_proposal.txt`): a plain-language rule system with
    tier and bracket groups (`medal` for every medal tier), player variables (each stat, card
    and COM style, allowed A positions, height, weight, age, strong foot, weak-foot accuracy
    and usage, injury resistance, form, registered position, playing style), per-preset
    tactical variables (each player's position and x/y per preset and fluid formation),
    `IF`/`ELSE`/`AND`/`OR`/`NOT` conditions with comparisons on any variable, additive `_mod`
    modifiers (each stat, all stats, A positions, the height allowance capped below the next
    bracket, weak foot, injury resistance, form, card limit) and `OR` between modifiers tried
    as needed before the card limits. Not adopted as a format; the check is that the
    `aatf_rules.md` schema can express everything it can, field by field, each gap either
    added to the schema (plan edit, decision entry) or answered by a custom check, with the
    reason → verify: a table in `aatf_rules.md` mapping each of the proposal's items to a
    schema field or to a custom check, no row empty. Done 2026-10-14 (lead): `spec/aatf_rules.md`
    "Coverage of a plain-language proposal", 30 rows; the gaps became fields (`ranges`,
    `strong_foot`, `any`, the `form` and `injury_resistance` effects, `team.formation` for
    custom checks), implemented at 5.2d2; `ruleset_editor.md`'s forms carry them
  - [x] 20.0b the interactive draft (`core/gui.md` "Prototypes before implementation"):
    `docs/prototypes/ruleset_editor.html` over the `ruleset_editor.md` forms, the official
    Autumn 26 ruleset as its sample data, every schema field reachable, the data-block pane
    beside the forms → verify: each of `ruleset_editor.md`'s editor sections has its form in
    the draft, each row of 20.0a's table that maps to a schema field has a widget the page
    can show it in, and the page uses only the allowed widgets (the lead's read against that
    list). Done 2026-10-14 (lead): one file, every section of the
    editor (start page, the nine sections, undo and redo, the load rules as live problems
    with the rail's marks, Save refusing while a problem stands); the data block is rebuilt
    on each edit, changed lines highlighted, and for the unedited sample holds the same
    values as `rulesets/autumn26.rhai` (a headless-Edge harness, `.tmp/20_0/run_harness.py`,
    which also renders every section and adds every condition and effect kind). Randomize,
    Open and the other templates are shown disabled. Per-group free cards moved into each
    tier's expander (`ruleset_editor.md` "The editor"). To show the members; their remarks
    go into the plan before Phase 20's itemization
  - [x] 20.0c the draft brought up to date (maintainer's request, 2026-10-15): the five other
    embedded rulesets as working templates (custom checks included), `problems()` covering
    every `LoadRule` (`GroupsOverlap`, `ReservedGroup`, `UnknownTarget`,
    `SystemConditionTooWide`, ids inside effects; a name colour is eight bytes, any bytes), and
    a sixth template, "Clouds' proposal, every item (from VGL26)", using each of 20.0a's 30
    rows (each special's label names its item; the two custom-check rows as a real section
    over `team.fielded_positions` and `team.formation`) → verify: the harness shows every
    template valid with every view rendered, each transcribed block equal to its `.rhai`, a
    red case per new rule; the Clouds block assembled as a rules file loads and runs in
    `check-aatf`. Done 2026-10-15 (SWE-2; the lead cut the Clouds height quotas to giga,
    giant and manlet, which every legal team meets, and moved three stat bonuses off stats a
    tier already holds at 99, whose targets no player can reach). Run over the VGL26 cup save's
    teams 781 to 834, VGL26's own file reports the 2 known violations; the Clouds file places
    every tier and reports only what its own specials ask for (form 8, goalkeeping 85, buffed
    stats 87). Files: `docs/prototypes/ruleset_editor.html`
  - [x] 20.0d coverage of the DBC ruleset (maintainer, 2026-10-19), right after Phase 6
    closes (`plans/core/development_plan.md` "Divegrass Before Christmas"): Divegrass Before
    Christmas (DBC), an upcoming invitational, is to be the first test ground for 4cc Studio's
    Ruleset editor and Save editor. Its rules' source of truth is the wiki page
    `https://implyingrigged.info/wiki/Divegrass_Before_Christmas_7:_Melty_Under_The_Christmas_Tree`
    ("Tactical export Rules": height brackets, medal systems; the game is PES 2017), read
    through the `stealth-browser-mcp` server (a plain fetch gets HTTP 403) and re-read when the
    step starts (snapshot of 2026-10-19: `.tmp/dbc7/wiki_rules_2026-10-19.md`). How they are
    checked is in `C:\Data\4cc\Tools_4cc\DBC7-Editor`, a small fork of the official 4cc save
    editor (C++, `RockAnon/DBC7-Editor`; the rules in `aatf.cpp`, its commits "dbc7 specific
    aatf and buttons" and "multi gk and no gk stuff" over the initial import), a reference
    where it and the page differ. Every DBC rule gets UI-class support in the Ruleset editor (a
    schema field with its form), except one niche and complex enough to be a custom check,
    each such with its reason → verify: a table in `aatf_rules.md`, as 20.0a's, mapping each
    DBC rule (read from the page, with the fork's diff against the official editor for how it
    is checked, not from memory) to a
    schema field or to a custom check, no row empty; the gaps become schema fields (plan edit,
    decision entry) and forms in `ruleset_editor.md` and the draft; a DBC ruleset file in
    `rulesets/` that `check-aatf` runs against a save the fork's check agrees with. Done
    2026-10-20 (SWE-2, lead fixes): the page re-read (unchanged since 2026-10-19); three schema
    fields, none a custom check (`plans/aatf_rules.md` "The ruleset schema", "Coverage of the
    DBC7 rules"; DECISIONS 2026-10-20): `medal_systems` (GGG, GGSS, GSSSS, SSSSSS),
    `HeightSystem::by_quotas` (a system told apart by its counts) and `extra_height` (GigaHeight's
    50 cm pool, at most 210), with the violations `MedalSystem` and `ExtraHeight` and their load
    rules; a tier is recognized before its choices (DECISIONS 2026-10-20), which the parity run
    found (a Manlet silver at 92 read as a regular); `rulesets/dbc7.rhai` and its fixtures; the
    draft's forms, load rules, summary and a DBC7 template. Parity against the fork's
    `aatf_single`, built headless, on the maintainer's two PES 2017 saves (220 teams each):
    every difference a listed class (`plans/aatf_rules.md` "DBC7"); the fork passes 29 and 36
    teams that the block fails on stats inside the fork's warning band alone (QUESTIONS "DBC7:
    the organizer's editor errs on what the page leaves free"). The draft's dumped block runs in
    `check-aatf` with the same output as `dbc7.rhai`. Reviews: SWE-2 S1 (3 accepted), Astra A1
    (2), SWE-2 S2 (1) (`.tmp/20_0d/duck_rulings.md`). Files: `crates/libs/aatf/` (`logic/
    logic.rhai`, `rulesets/dbc7.rhai`, `src/ruleset/{mod,check,errors}.rs`, `src/violation.rs`,
    `src/file.rs`, `src/fixtures/`, `src/ruleset/tests/`), `crates/tools/save_editor/src/
    messages.rs`, `docs/plans/aatf_rules.md`, `docs/plans/ruleset_editor.md`,
    `docs/prototypes/ruleset_editor.html`, `docs/DECISIONS.md`, `docs/QUESTIONS.md`
  - [x] 20.0e the DBC7 organizer's choices (2026-10-21, `plans/aatf_rules.md` "DBC7", DECISIONS
    2026-10-21): `Tier::stat_warning`, `Tier::total_within_targets` and the `stat_warning`
    effect (`plans/aatf_rules.md` "The ruleset schema", "The stat band"), the `Stat`
    violation's severity and `StatTotal`, their load rules and the Save editor's texts;
    `dbc7.rhai` (the band, the total, the registered-position warning, the 189 cm goalkeeper's
    single A, no card caps); the Ruleset editor's forms (`plans/ruleset_editor.md` "The
    editor") in the draft → verify: the 20.0d parity run against the fork on the two PES 2017
    saves, every difference a listed class (the total and the registered-position severity
    the new ones), the 29 and 36 teams the band alone failed now passing unless the total
    catches them, tallied; the stat total's two readings (the player's own targets' sum, over
    every ability stat) confirmed by the organizer 2026-10-22. Done 2026-10-21 (SWE-2): the three fields with their load rule (a choice option
    holding `stat_warning` is refused), the band's severity and `StatTotal` in `logic.rhai`,
    the Save editor's line for it, `dbc7.rhai` (the band 7 and 5, the Manlet regular's 5, the
    total, the 189 cm goalkeeper's single A, the registered-position warning over the 13
    positions, 48 paid cards and 41 skills, no cap), the draft's columns, effect and DBC7
    template (its dumped block runs as `dbc7.rhai` does). Parity (`.tmp/20_0e/parity.md`): every
    difference a listed class; the 29 and 36 teams the fork passes fail on `StatTotal` alone,
    each player 92 to 115 over (every stat at Autumn 24's +4 or +5). Files:
    `crates/libs/aatf/` (`logic/logic.rhai`, `rulesets/dbc7.rhai`, `src/ruleset/{mod,check}.rs`,
    `src/violation.rs`, `src/file.rs`, `src/fixtures/dbc7.rs`, `src/ruleset/tests/rules.rs`),
    `crates/tools/save_editor/src/messages.rs`, `docs/prototypes/ruleset_editor.html`, the
    plans and decision named above
- [x] 20.1 Autumn 26 against the official 4ccEditor's Autumn 26 edition (the maintainer,
  2026-10-21; upstream `origin/master` `cb211f4` and `82879a7`, 2026-10-20). `autumn26.rhai`
  was transcribed from the fork's `Autumn_2026_AATF` branch (`spec/aatf_rules.md` "The
  official ruleset"); upstream wrote its own, and they differ in six places
  (`.tmp/20_au26/upstream_compare.md`): D1 the medals' free A position as a trade of their
  own free COM styles (upstream) or one extra choice on top (Studio); D2 bronze free tricks 2
  or 3; D3 who gets the Red CB +4 cm (registered CB with a single A at CB, or fielded only at
  CB); D4 which height brackets that allowance shifts; D5 Green 194 cm silvers (upstream
  fails every one, a bug the fork fixed); D6 upstream's new check that preset 1's starters
  stand at A positions, which Studio has as `universal.fielded_at_a: [1]` and autumn26 leaves
  unset. Ruled 2026-10-21 against the wiki's `Appendix:Rules` (edited 2026-10-06; DECISIONS
  "Autumn 26 follows the wiki's written rules…"): D1 a trade, gold twice; D2 bronze 2; D3
  registered CB only; D4 the allowance at 189 alone, 185 staying tall (a Red CB at 185 fielded
  at CB was in no bracket: five such CBs on each of the fixture saves' teams 718 and 734); D5
  Studio stands; D6 set. Done 2026-10-21 (SWE-2, a run and a resume after the session
  crash): `autumn26.rhai`, the spec's block and prose, every fixture and consumer test that
  pinned the old reading re-derived with its ruling named. The organizer's seven points were
  answered 2026-10-22 (20.2).
  Files: `aatf/{rulesets/autumn26.rhai,src/fixtures/official.rs,src/ruleset/tests/{mod,
  rules}.rs}`, `team_widgets/src/violations.rs`, `save_editor/tests/cli/check_aatf.rs`,
  `docs/spec/aatf_rules.md`
  - [x] review (checkpoint b, three crates): a SWE-2 loop, then Astra. S1 accepted 1 of 5
    (an untaken medal trade gave no hint: `"choices"` added to `suggestions`), ending the
    SWE-2 loop; rulings `.tmp/20_au26/duck_rulings.md`. A1 (Astra) accepted 3 of 4, ending
    Astra's loop: the gold's two trades read through the card limits, SE-GUI-15's player
    groups observed again, the Autumn 25/Spring 26 spec list completed; the Red manlet bonus
    is the organizer's seventh point. S2 (post-Astra SWE-2, 2 of 4) ended the loops: a medal
    CB at 189 under Red and check-aatf's player error line pinned
- [x] 20.2 Autumn 26 takes the 4cc organizer's answers (DECISIONS 2026-10-22): mid exactly 180
  (176 to 179 in no bracket), a gold or goalkeeper at 194 an error, the Red 175 cm bonus a
  one-option `choice`; the other four answers confirm 20.1's rulings. The organizer's page
  compared the wiki with the official 4ccEditor alone, every editor claim checked against
  `82879a7`'s `aatf.cpp`. Done 2026-10-22 (SWE-2, the lead adding the bonus's tier scope): tier
  recognition reads a Red manlet at base or base plus bonus as his own tier (`profile` before
  `resolve_choices`); SE-AATF-01's warning leg now runs a rules file with a warning, the
  official file having none. Files: `aatf/{rulesets/autumn26.rhai,src/fixtures/official.rs,
  src/ruleset/tests/{conditions,mod,rules}.rs}`, `save_editor/tests/{cli/check_aatf.rs,
  fixtures/aatf/official_warning.toml}`, `docs/spec/{aatf_rules,save_editor}.md`
- [x] 20.3 the Acceptance section (lead, 2026-10-23): `plans/ruleset_editor.md` "Acceptance"
  (RE-GUI, RE-FILE, RE-SUM, RE-RND, RE-CLI), the start page's templates (DBC7 among them) and
  Blank made exact, logic version 1 frozen at the 0.0.1 cut (DECISIONS 2026-10-23) → verify:
  checkpoint (a), a SWE-2 loop, then Astra. Done 2026-10-23: SWE-2 rounds accepted 5, 8, 4,
  then 3 and 2 around Astra's 9 and 3. The fixes made the plan and the draft agree (the effects
  menu's fifteen entries, the per-tier switch, the summary's card groups and team settings, the
  height systems' drag order, "any of" and "except when" and the last one's "otherwise", the
  recent files), and a file breaking a load rule is refused at open like a parse error
  (DECISIONS 2026-10-23). Files: `docs/plans/{ruleset_editor,aatf_rules}.md`,
  `docs/prototypes/ruleset_editor.html`, `docs/DECISIONS.md`
- [x] 20.4 `libs/aatf`'s writer (`src/writer.rs`, `Ruleset::to_data_block`; `plans/aatf_rules.md`
  "Crate layout") → verify: every embedded ruleset and an every-field ruleset read back equal
  from what it writes, and writing twice gives identical bytes; the layout pinned by one test.
  Done 2026-10-23 (SWE-2, a run and one rework): each type's fields destructured exhaustively,
  so a schema field the writer misses fails to compile; a `serde(default)` field left out at its
  default; map keys bare only when Rhai's own parser takes them (`in`, `go`, `1st` quoted); the
  draft's layout at 100 columns, a special's `severity` after its label and a system's
  `by_quotas` before its quotas as the hand-written blocks write them. A test pins that a
  medal system counting no tier loads: it means "no medal players" (a tier a system leaves out
  counts 0), which 20.3's review first took for a system every team matches. Files:
  `crates/libs/aatf/src/{writer.rs,lib.rs,ruleset/tests/rules.rs}`, `docs/plans/aatf_rules.md`
- [x] 20.5 the tool crate's egui-free core (`plans/ruleset_editor.md` "Crate layout"), as
  slices:
  - 20.5a what `libs/aatf` must expose (a file's sections, the file assembled from a ruleset
    with its own logic or custom checks), the crate (`StudioTool` wiring, registered but CLI-only
    until its view, `messages.rs`), `document.rs` (open with the logic status, save through
    the load validation to a temporary sibling, the logic update) and the CLI's
    `validate` and `update-logic` (RE-CLI-01, 02): each item with a caller in the slice.
    Landed 2026-10-23 (SWE-2, a run and one rework): `aatf::{file_sections, assemble_file}`
    share `assemble`'s joining; `update-logic` keeps a `.bak` and reports the version it
    replaced; a file whose logic lost its banner opens as modified and cannot be saved (the
    rewrite would hold no logic). Files: `crates/libs/aatf/src/{file,lib}.rs`,
    `crates/tools/ruleset_editor/`, `crates/studio/{Cargo.toml,src/main.rs,tests/help_cli.rs}`
  - 20.5b `summary.rs` and the CLI's `summary` (RE-SUM through the CLI, RE-CLI-03). Landed
    2026-10-23 (SWE-2, a run and one rework): the draft's wording ported, one function per
    section; nine goldens, the draft's own `summary()` run on each embedded ruleset as the
    writer writes it (lead-made, `scripts/provenance/fixtures/ruleset_summary_*`), matched
    byte for byte; generating them fixed four draft defects (the first bracket "up to 175",
    one capital in "Every team", per-tier values in tier order, "1 A position") and review
    two more (a per-tier stat target as words, an empty `exactly` list as the label alone).
    RE-FILE-03's third file (two tiers sharing an id) joined RE-CLI-01's legs.
  Reviewed 2026-10-23 over 20.4-20.5 (SWE-2 S1a/S1b, Astra A1 on Codex brother, SWE-2 S2: 1, 2,
  2 and 2 accepted): `summary -o` refuses the rules file it reads, a refused save exits 2, a
  read-only file is not replaced on Linux either, the writer became `src/writer/` with its
  tests apart, and the plan's "byte for byte" for a kept section sets its trailing whitespace
  aside. `just mutants-diff 6447bb31`: 315 mutants, 8 missed; 6 got tests (the list wrap's
  edge, a zero per-tier value's sign, the help catalog), 2 are the placeholder view and empty
  settings 20.6a1 replaces
- Set aside on 2026-10-24 (the maintainer: not in 0.0.1, after Release 0.1.0;
  `plans/core/development_plan.md` "Phase 20"): 20.6a2a, in flight then, is reviewed and
  landed, and the steps below wait until after 0.1.0.
- [ ] 20.6 the view (`plans/ruleset_editor.md` "The editor", the draft
  `docs/prototypes/tools/ruleset_editor.js`), as slices:
  - 20.6a the window's frame and the plain forms, as subslices:
    - [x] 20.6a1 `settings.rs` (`last_folder`, the recent files), the templates (embedded in
      `libs/aatf`) and Blank, the document's dirty flag and undo snapshots, the start page,
      the toolbar (Open, Save, Save as, Undo, Redo, Show preview), the rail and its problem
      marks, the preview pane, the logic banners, the unsaved-changes prompt, General, Custom
      checks and Summary (RE-GUI-01, 02, 13 to 15, RE-SUM-01, RE-FILE-01 to 03, 05, 07).
      Done 2026-10-23 after one rework: a snapshot is the ruleset and the logic together (an
      Update logic's undo was a no-op), the forms bind the ruleset in place with the gesture's
      snapshot taken around the section call, "New from template" opens the start page
      unguarded (the picks inside prompt), a settings table that fails to read is never
      written over. `aatf/src/{templates.rs,rules.rs,lib.rs,ruleset/check.rs}`,
      `ruleset_editor/src/{document,settings,messages,summary,lib,cli,test_support}.rs`,
      `src/view/` (mod, start, prompt, preview, rail, `sections/`, tests),
      `help/02_window.md`. Review rework 2, 2026-10-23 (SWE-2 round 1's 17 accepted findings
      and the mutation run's survivors): undo seals a gesture in flight, the shortcuts wait
      behind a modal, Save as suggests the ruleset's name, `aatf::NON_STAT_TARGETS`, the file
      pickers panic under `cfg(test)`; tests for every survivor. Rework 3, 2026-10-23 (SWE-2
      round 2, 4 accepted per half; `mutants-diff d968dd50` 1 missed, covered): one modal
      helper, one refused-open block, Custom checks drawn without its banner, a cancelled
      Save keeps the prompt, the shortcuts dead behind every modal, the dialogs' arguments
      asserted; `mutants-diff 3a232581`: 24 caught, none missed. Rework 4, 2026-10-23 (Astra
      round 1 on Codex own, 4 of 4 accepted; Astra's loop ends): the forms scroll sideways
      (the preview narrowed General's checklists past reach), RE-GUI-02's Save as through the
      File menu and an already-saved document's Save as leaving the original, the Open after
      the prompt's save starting in the folder that save chose, a settings key this version
      does not know kept on write; `mutants-diff 5f3910c6`: 32 caught, the 2 missed (the
      native window's options) got a test. Rework 5, 2026-10-24 (SWE-2 round 3 after Astra, 3
      of 3 accepted; the loop ends): the modals scroll, so a long findings list or a window
      resized shorter keeps every row reachable; the tests' repeated settings seed, dialog
      stubs and dirty-document setup one helper each. 20.6a1's reviews are done; open: its
      `mutants-diff b8ed279f`
    - 20.6a2 Tiers with the medal systems, Cards, Universal rules with the team settings,
      Suggestions (RE-GUI-03 to 06, 08, RE-FILE-04, 06), as two subslices (one is past a
      review's size):
      - [x] 20.6a2a Tiers and the medal systems, the tier expander's per-stat targets and free
        cards (its conditions are 20.6b's condition editor), the refused save drawn
        (review S2b-3) (RE-GUI-03, RE-GUI-04's tiers half, 05, 06, RE-FILE-04, 06). Done
        2026-10-24 (SWE-2, 54 min; rework 1 for the lead's review, which stalled half-way and
        the lead finished): `sections/tiers.rs` (one `Grid` of the draft's columns, the handle a
        `dnd_drag_source` and every cell a drop target, each row's expander in the last column's
        cell widened back to the table's left edge, `Grid` having no colspan), the medal
        systems under it, `sections/widgets.rs`, `document::tier` shared with Blank's tier;
        RE-FILE-04's lib half compares four VTL11 teams under a renamed Silver label. Files: `ruleset_editor/{Cargo.toml,src/{document.rs,view/{sections/
        {mod.rs,tiers.rs,widgets.rs},tests.rs,tests/tiers.rs}}}`, `aatf/src/fixtures/vtl11.rs`
        - [x] review: a SWE-2 loop, then Astra, then SWE-2 (checkpoint b), M5-1 with its fixes;
          then the Ruleset editor waits for 0.1.0. S1: 4 of 7 accepted; `mutants-diff
          ff88177d`: 17 survivors in `tiers.rs`. Rework 2 (SWE-2, 75 min, four contradictions
          accepted): the open expander's cell recorded at the column's own width (it had
          widened the table by its own width: dead scroll space), a test for every Tiers
          control (add tier, the ✕'s presence, the `counts_as` combo, medal count 0, free
          count, a system's ✕ and add, rest, the name-colour checkbox, a drop on a label
          cell), the draft's two titles, one `cell` helper for the row's 16 drop targets;
          M5-1 settled (egui bounds a new modal's first frame; the cap binds on a window
          shrunk under an open modal, now tested). Astra A1: 5 of 5 accepted, rework 3 (SWE-2,
          25 min): every section number draws as stored (DBC7's 48 paid cards had been
          clamped to 20 by drawing the section) and the cards run to 48; the id commits on
          Enter or focus loss, its medal-key carry refused while ambiguous (typed per
          keystroke through `gold`, it moved Gold's counts); RE-FILE-04 and RE-GUI-03's tests
          observe the label and the preview line; each Tiers field changed alone through its
          control (decision 2026-10-24). S2: 1 of 6 accepted, lead fix: General's squad size
          drawn as stored too, with four nits. Astra A2: 3 of 5 accepted, Astra's loop ends;
          rework 4 (SWE-2, 8 min): a half-typed id or settings path survives the window
          losing focus (the buffer follows the stored value only while egui's memory holds no
          focus on the field), a tier's ✕ keeps the medal counts while another tier carries
          its id, RE-GUI-03's test reads the drawn unsaved marker; `mutants-diff 00ef6231`
          clean. S3: 2 of 6 accepted, the loop ends with rework 5: `color_tools`' picker
          keeps a typed hex or channel through a window focus loss (A2-1's class, in the
          Save editor's picker too), the Tiers tests' repeated preamble one helper
          (`74dcf473`; `mutants-diff d3bf671f` 3 of 3 caught). Review done
      - [ ] 20.6a2b Cards, Universal rules with the team settings, Suggestions (RE-GUI-08)
  - 20.6b the condition editor, the effects editor, Heights and Specials (RE-GUI-07, 09 to 12,
    and RE-GUI-04's height-systems half, whose test joins the tiers half's)
- [ ] 20.7 Randomize (`plans/ruleset_editor.md` "Randomize"): `random/`, the start page's and
  toolbar's entries, the CLI's `random` (RE-RND, RE-CLI-04) → verify: 1,000 seeds per preset,
  and the share of seeds that needed a redraw measured and reported
- [ ] 20.8 the help chapter, and the Ruleset editor active on the shell's sidebar
- [ ] 20.9 converge (AGENTS.md "Closing a phase"), RE-GUI-16 checked by hand

---

## Issues

Bugs, unexpected behavior, things to revisit. `open` / `resolved (date)`. Resolved issues are
pruned when their phase closes; they stay in git history. A question only the maintainer
can answer is not an issue: it goes to `docs/QUESTIONS.md`, and an issue that waits on one
points there.

- resolved 2026-10-28 — the Save editor's CLI tests read the real process list (found
  2026-10-27, cm4's gate run with PES 2021 open: 18 of `tests/cli`'s 46 failed, each "PES 2021
  is running: … close the game first"). Now `cli::run` takes `Tool`'s `running` probe, the
  integration tests set it through `save_editor`'s `test-support` feature
  (`Tool::with_running_probe`, a self dev-dependency; the sandbox's default reports no game),
  `import_toml`'s two tests refuse under a stub reporting the save's PES and write under one
  reporting PES 2019, and CONTRIBUTING's "A test never reaches the developer's own files"
  names the running games. A sandbox default reporting PES 2021 fails 18 tests

- open 2026-10-29 — `studio`'s `tests/crash_dump.rs`
  `a_stack_overflow_leaves_a_dump_and_the_monitors_line` failed in k5a's checkpoint
  (`gates-changed 17c23789`, four test binaries at once beside `save_editor`'s): the crashed
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
  the census's copy `.tmp/0_0_1i/census/saves/5e3d82ea3c9469ee` on side-lead B's worktree).
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
  A shape detector (`.tmp/dup_audit/clones.py`: identifiers, strings and numbers blanked,
  declarations and table rows set aside; a run of 4 shaped lines found 3 times in a crate, or
  of 10 found twice, is a cluster; `--across` for runs in two crates) measured HEAD
  `8aeb0193`: about 260 non-test clusters (pes_savefile 70, aatf 30, team_compiler 31,
  save_editor 23), 81 across crates (`fmdl`'s and `pes_model`'s `ops/split/build.rs` share
  about 180 lines), and some 65,000 repeated shaped lines in tests. The work: (1) SWE-2 rules
  every non-test cluster, five groups (`.tmp/dup_audit/swe_dup_<a-e>.md`, findings beside
  them), the lead rules the findings; (2) one rework brief per crate for the accepted ones,
  between the slices in flight, each gated and measured by `mutants-diff` like any rework,
  `writer/mod.rs` first after 20.6a1; (3) the test clusters, the same way; (4) the detector
  moves to `scripts/` with a `just clones` recipe, the thresholds the audit's rulings settle,
  and a committed keep list (each kept cluster's shape and its one-line reason, which the
  detector then skips), and runs at two points (the maintainer's question of 2026-10-23):
  in `just gates`, failing on any cluster the keep list does not hold, so a diff that adds a
  copy is fixed or ruled before it lands, with no reliance on a reviewer noticing; and at
  every phase's close, whole-crate, in the design-health pass (`AGENTS.md` "Closing a
  phase", beside `just mutants` and `just citation-audit`), where the keep list's entries for
  the phase's crates are read again, since a kept pair that grew a third copy is no longer
  a pair. `AGENTS.md` and CONTRIBUTING "Code style" say both. State: (1) done, the rulings in
  `.tmp/dup_audit/rulings.md` (group IDs `A-F<n>` to `E-F<n>`, the lead's own `C-L<n>`; keep
  that file until (2) ends); (2) the first brief, `aatf` and the Ruleset editor's summary,
  `.tmp/dup_audit/brief_rework_aatf.md`, done 2026-10-23 (SWE-2; `aatf`'s source clusters 30
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
- resolved 2026-10-21 — `mutants-diff` runs one cargo-mutants per crate, each with its own
  baseline and timeout, `timeout_multiplier` back to 5 (`RUNBOOK.md` "Mutation runs and the
  VPS", DECISIONS 2026-10-21). Measured on `mutants-diff 86492df7` (90 mutants over
  `music_export`, `pes_savefile`, `save_editor`, `team_widgets`): timeouts set to 20 s, 399 s,
  700 s and 20 s, none hit, about 28 min in all, locally; `mutants-diff 95f20eac` had one
  751 s timeout for every crate and 9 timeouts. Uneven shares by machine speed for a diff
  stay unbuilt: the timeouts were the cost
- resolved 2026-10-22 (`save_editor/src/test_support.rs` `scratch`: every unit test's folder is
  `target/<profile>/save_editor_tests/<test>`, emptied at its start; no other crate's tests left
  anything in Temp, measured the same day) — tests leave their save copies in the system Temp folder: `save_editor`'s
  view tests copy a fixture save (5 to 11 MB) per test into `%TEMP%/save_editor_view_<pid>/`
  and never remove it, so every run, a mutation run's mutants included, adds about 0.5 GB;
  42 such folders held 19.9 GB on 2026-10-22. Fifteen files call `std::env::temp_dir()`
  (`grep -rln "std::env::temp_dir()" crates`); the session tests' fixed names are overwritten,
  the pid-named ones pile up. Direction: a per-checkout folder under the crate's own `target/`
  (gone with `cargo clean`, a mutation run's copy has its own), named per test without the
  pid; measure which other crates' tests leave files first
- resolved 2026-10-22 (`scripts/mutants.py` `remote_disk_use`/`disk_refusal`, the guard every
  run that uses the host passes, and the phase-close cleanup: RUNBOOK "The scratch folder"; the
  emptying's first live run is the next run that uses the host) — the VPS half's tests use `~/studio-mutants/tmp` (`REMOTE_TMPDIR`) as their
  temp dir, and a helper that removes its folder at the test's end leaves it on every caught
  mutant (the test panics first): on 2026-10-22 it held 16,786 such folders, 28.6 GB (27 GB of
  them `save_editor`'s, fixed at `47e57a0d`; the rest `export_upgrader_*`, `pes_savefile_test_save_*`,
  `studio_core_*` and others, about 1.5 GB), the host's disk at 99%, and a `mutants-diff` lost
  5 mutants to it. Cleaned with `.tmp/lead/bonfire_tmp_cleanup.sh` (the disk at 80% after),
  then the clone's build cache `~/studio-mutants/tree/target` (22 GB; only `just gates-remote`
  builds there, so its next run starts cold) with `.tmp/lead/bonfire_target_cleanup.sh`: 66%.
  Direction: `scripts/mutants.py` empties the tests' leftovers from `REMOTE_TMPDIR` after it
  collects a remote half, or the helpers move under `target/` as `save_editor`'s did
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

- resolved 2026-10-28 (the entry above: the CLI asks `Tool`'s probe) — two `save_editor` CLI tests (`strip::tables_the_bins_lacks_are_created_from_the_seed`,
  `strip::the_tables_are_seeded_and_every_appearance_id_is_set_to_minus_one`) fail while
  PES 2019 runs on the machine: the command's running-game refusal (`src/cli.rs`, "is running:
  it writes its own save back") reads the real process list, so a gate run during an in-game
  check fails (6.16d's first run). Likely fix: the tests inject the process probe (or the
  command takes it from its context), so no test depends on what else the machine runs.
  (The game no longer needs closing for the tests' sake; a Fox PES still takes the CPU.)
- open 2026-10-19 — the maintainer saw AMD driver timeouts during the BC7 encoding tests
  (`dds_convert`'s `gpu.rs` tests, 16.x a). Logged so far: one Windows GPU reset (System log,
  event 4101 "Display driver amduw23g ... stopped responding and has successfully recovered")
  at 03:16:41, while 16.x's probe (`.tmp/16x/gpu_probe/`, edited 02:50-03:09) dispatched whole
  mips in one submission, before `gpu.rs`'s bounded slabs (`SUBMISSION_PIXELS`, c68ef99 at
  04:36); none logged since. Still to rule out: the tests' own load, each GPU test building its
  own device and `cargo test` running them in parallel, several worktrees' gates at once, a
  `dds_convert` mutation run's parallel jobs. Parked (the maintainer, 2026-10-19): no
  investigation now; during every later BC7 test (16.x b, a `dds_convert` mutation run) check
  the System log for event 4101 (`Get-WinEvent -FilterHashtable @{LogName='System'; Id=4101}`)
  against the run's times, and if one recurs, reproduce (the GPU tests looped, alone and in
  parallel processes) and serialize the tests' device use or lower the budget, whichever the
  measurement points at.

- resolved 2026-10-17 (after Phase 5's close; `scripts/mutants.py` `plan_file_split`) — a split mutation run waits on the
  VPS for crates whose tests dominate: the halves are equal (`--shard 0/2` and `1/2`), but with
  half this PC's threads per test (8 vCPUs against 16) the VPS tests `aatf`, `save_editor` and
  `pes_savefile` at 60-80% of the local rate (5.6 runs: 92 against 157 an hour, 163 against 248,
  84 against 107), while build-bound crates run at par or faster. Give the local side the larger
  share in proportion to the two sides' measured rates per crate (`pes_savefile` about 55/45,
  saving about 1.4 h of its 11.5), only if it stays exact: cargo-mutants takes one shard per
  invocation, so the share needs either several local shards of an n-way split run one after
  another (one extra baseline each, about 75 s for `pes_savefile`) or another way to hand each
  side its list; whichever is used, the merged outcomes must name every listed mutant exactly
  once (checked against `--list` in the summary, a run failing it reported, not summarized).
  `scripts/mutants.py`; RUNBOOK "Mutation runs and the VPS". Resolved: a whole-crate run
  hands each side a complementary set of source files (`examine_globs` here, `exclude_globs`
  there) sized by the two sides' measured mean seconds per mutant, checked exact against
  `--list` before launch and against the merged outcomes after (exit code 5 otherwise);
  round-robin stays for diff runs and unmeasured crates. The remote means were seeded from the
  close runs' archives (`.tmp/5_6/shards/seed_remote_costs.py`); `studio_core`'s first file
  split put 38 of 85 mutants here, the halves ending 146 s and 153 s in.
- open (2026-10-15, 5.6c census) — PES 2014 files read as PES 15 and then fail with a
  misleading error: `SaveContainer::decrypt` labels a PES 2014 `EDIT.bin` or `TEXPORT0N.bin`
  `Pes15`, and the decode fails on a player name (not UTF-8) or a tactics byte. Loud, never a
  wrong write, but it should be refused as PES 2014 (unsupported): measure what separates the
  two versions' containers (PES 2014's EDIT is 4,799,651 bytes, its texport 5,389,806; no real
  PES 15 texport is on the machine) and refuse it by name. `.tmp/5_6/audit_notes.md`.
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
- resolved 2026-10-14 (5.2f-r2) — every team of the VGL26 cup save fails VGL26's "Support settings"
  special on `auto_preset_change` (all three PES 21 cup saves hold it off): the rule follows the
  rules page, which speaks of the deadline export. Resolved the same day (step 5.2f-r2): the
  page's "switch preset tactic" is the tactics menu's option, not the byte, which VGL26 and
  VGL27 now require off.
- resolved 2026-10-30 (folded into step 8.36c: every engine's patch carries the whole look; DECISIONS "Every engine carries a player's looks in the savefile patch; no stripped saves") —
  the pre-Fox aesthetics patch (`pes_savefile/operations.md` "Aesthetics patch", the pre-Fox
  half; `spec/team_compiler/pipeline.md` "Post-processing"): every appearance field, the resolved boots and gloves
  IDs through commit-conditioned mutations, the edit flags and the base-copy ID (issue below).
  Until then a compile for PES 15-17 writes no patch and leaves the savefile alone (decision
  2026-10-11). With it, lift the Fox-only gates of `team_compiler` `validation.rs`
  `settings_id_messages` and `settings_name_messages` (their catalog rows say Fox only).
- open (2026-10-11, Phase 5 Acceptance) — a referee export's `settings.toml` reaches no table or
  patch: a referee's look is the template's `RefereeAppearance.bin` (`blue_port.md`), whose rows
  nothing writes, and `blue_port.md` has `fpc_off` apply its settings preset to a referee on
  PES 18-21. Settle with a Fox refkit (the issue on a referee's body under `fpc_off`): which
  route a referee's settings take (the aesthetics patch, as team 999, since 8.36 writes no
  player tables, or the template's table), and TC-REF-19's outcome then; with it, lift
  `settings_id_messages`' team-only gate (`validation.rs`).
- open (2026-10-11, 5.0c; deferred until the cup next plays a pre-Fox version) — what Red does
  on PES 15-17 that Studio does not (`.tmp/5_0c/xml.md`, `checks.md`), most noticed first:
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
- resolved 2026-10-14 (maintainer: `Tier::name_color`, written by `apply_tier`; `aatf_rules.md`
  "The ruleset schema", "The host"; implemented at step 5.2d2) — `apply_tier` cannot make a VTL11
  medal legal: VTL11 requires a medal's name to carry a gold or silver colour code (a
  `name_colors` condition), which no tier target expresses, so the quick action writes no name
  and the checker then rejects the player, against `aatf_rules.md`'s "neither tool can produce
  a player the checker then rejects". The VGL fork's editor writes the colour itself (`9951666`,
  and `vgl27-Ruleset-qol`'s `auto_color_names`). Recommended: a tier target for the name colour,
  which `apply_tier` writes (and strips from a tier that forbids one), over a separate action.
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
  (`--jobs 1` in `scripts/mutants.py`'s local config).
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
- resolved 2026-10-15 (Phase 5 converge: step 5.3a writes the default-ID rows, TC-BIN-25
  and 27; the gloves' look waits on the FoxDen issue below, "FoxDen's PES 2021 gloves
  trampoline") — the boots/gloves rows a `Full` export leaves (`spec/team_compiler/pipeline.md`
  "Bins accumulation", TC-BIN-17, decision "a team export's name carries `Full` or `Midcup`").
  Phase 4 (4.22) drops the installed row of a compiled player the export gives no boots or
  gloves, so he has no row until Phase 5 writes the default-ID rows. A team player the `Full`
  export does not compile at all keeps his rows as they are (decision 2026-10-05; the plan
  already says so). Test 2 (PES 21, `.tmp/4_0/apptest/out_test2/`, run 2026-10-06), same results
  on the 4cc exe and the stock exe:
  - **Boots:** a stripped player with no `BootsList` row wears the boots-0 model (plain black
    boots), not his save's boots field (1076). So a dropped row and a row naming 0 look the
    same, and 4.22's drop is safe for boots.
  - **Rows are read:** stripped players with a row wore k0629 (/gd/ boots).
  - **Kept appearance id:** the control (his own boots 1076, plus a k0629 row) wore his own
    boots, so a kept appearance id beats the row on PES 21.
  - **Gloves:** the 4cc exe shows bare hands on both goalkeepers (its gloves patch). On the
    stock exe a `GloveList` row is read (g0003, yellow). The no-row goalkeeper wore white
    gloves, which are both his save's field (g0001) and possibly the default: one with
    non-1 save gloves would tell the two apart. Settle before 4.22 only if the gloves default
    matters.
- answered (2026-10-05, below) — the seed rows and multi-CPK mode: strip-and-seed puts its rows
  into `4cc_08_bins.cpk` (decision 2026-10-05), but in multi-CPK mode that CPK is the
  compiler's own bins output (`settings.md` `bins_cpk_name`), and the bins walk starts below
  the output CPK (`spec/team_compiler/pipeline.md` "Bins accumulation"). So the DLC builder's next multi-CPK
  compile reads the three player tables from below the seed and replaces `4cc_08_bins.cpk`
  without the seed rows of every player no export compiles. Resolved (2026-10-05,
  maintainer): in multi-CPK mode the player tables' walk starts at the bins CPK itself
  (`spec/team_compiler/pipeline.md` "Bins accumulation").
- answered (2026-10-05, below) — motions on Fox: `settings.toml`'s `[appearance.motion]`
  (hunching, arm movement, kick motions, celebrations, dribbling) are player-record fields
  outside the appearance block (PES 20/21 bits 96 to 332), so neither the `PlayerAppearance.bin`
  row (the block's bytes) nor the Fox patch (names only) carries them: on Fox a compiled
  export's motions reach nobody. Resolved (2026-10-05, maintainer): the Fox patch carries
  every compiled player's motions (`pes_savefile/operations.md` "Aesthetics patch"). Untested: that the game
  uses a stripped record's motions (expected: they are outside the appearance block).
  Where a save's motions come from (2026-10-05): a PES 19+ EDIT generated by the game holds
  no player records (the vanilla PES 21 one, `.tmp/4_0/apptest/vanilla21/`); the cup's players are
  injected by `pes-db-generator`'s `player_edit.py`, every record one shared base body
  (`Player_Edit_Base_20.bin`: every motion stored 0, i.e. 1, the motion edit flag set). The
  Winter Cup 2023 save's 4,829 players with a motion other than 1 were edited after that.
- answered (maintainer, 2026-10-08, after Test 4c: a pre-Fox game never applies a
  `GloveList.bin` row, so a full pre-Fox DLC needs the savefile patch anyway, and writing
  `PlayerAppearance.bin` and `BootsList.bin` beside it would split one player's look between
  the CPK and the save for nothing; pre-Fox writes no player tables, the aesthetics stay in
  the savefile patch whole; `pipeline.md` output table, decision entry) — pre-Fox has the
  player tables too (found 2026-10-05):
  `BootsList.bin` and `GloveList.bin` in the base data of PES 15, 16 and 17 (`dt33`, and PES
  17's `dt00_win`), and in the cup's own `4cc_02_misc.cpk` on PES 15 and 17, with a row for
  every cup player id 70101-89223, all 1; `PlayerAppearance.bin` (13,242 rows of 60 bytes, as
  on Fox) and `RefereeAppearance.bin` (35 rows) in PES 17's `dt00_win`; no `PlayerAppearance.bin`
  on PES 15 or 16 (`.tmp/4_0/apptest/prefox_bins/`). Unknown: whether a pre-Fox game reads them
  for a save player (a Test 1 on PES 17 would tell), and what the cup's all-1 rows are for. If
  it does, pre-Fox could carry boots, gloves and (PES 17) appearance in the CPK as Fox does.
  The cup's all-1 rows are placeholders (maintainer). The cup's PES 17 `4cc_01_db.cpk` also
  ships a `PlayerAppearance.bin` with a row per save player. Test 4
  (`.tmp/4_0/apptest/out_test4/GUIDE.txt`, 2026-10-05) repeats Test 1 on PES 17: /b/ stripped
  (appearance id at record byte 116, body 128 to 184; the same 72-byte block as PES 21's at
  240), the three tables as loose files in Sider's livecpk root. Results (run 2026-10-06):
  - **Appearance:** PES 17 reads `PlayerAppearance.bin` for a -1 record, per player (look A,
    and look B on no. 3).
  - **Boots:** it reads `BootsList.bin`. Every /b/ player wore k0563 (brown boots), the
    control DESU included. That's unlike PES 21's control, but DESU's save boots are 0, so
    either the row beats a kept id on PES 17, or a save boots field of 0 defers to the row.
    The placeholder all-1 rows fit either reading only if k0001 looks like the default boots.
  - **Gloves:** the goalkeeper's g0556 row showed bare hands. That's inconclusive: the 4cc
    exe's gloves patch, as on PES 21, or the table is not read.
  - **Control's look:** inconclusive. DESU has a custom head and the same skin colour as look
    A.

  So pre-Fox (PES 17 at least) could carry appearance and boots in the CPK as Fox does. What
  is still needed: the boots reading above, and gloves on a stock exe.
  Test 4b (run 2026-10-08 by the lead through the in-game harness, `.tmp/4_0/ingame/game.py`;
  files by `apptest build` with `APPTEST_CONTROL2`, installed by `.tmp/4_0/apptest/test4b.py`):
  Test 4 plus a second control, no. 4 ITS A TRAP, his appearance id kept, his save boots set
  to k0571 (grey sneakers) and his BootsList row k0563 (tan boots, everyone else's). In Edit
  mode (Edit Pony > Appearance) he wore k0571 while nos. 1 to 3 wore k0563, DESU (save boots
  0, row k0563) included: **a set save boots id wins; only a save boots id of 0 defers to
  the row.** Gloves, on the 4cc exe and on the stock one (`.tmp/4_0/apptest/exe_swap17.py`; the
  stock exe is the oldest `.old` by mtime, 2016-10-20): no. 1's hands were bare in Edit mode
  and in the match walkout both times, with a GloveList row g0012 and his DB appearance row's
  PlayerGloves bit at 0. Still inconclusive: the row did not apply, but the bit may be what
  turns gloves off; the next run sets that bit on his row (the tool's `gloveflags` finds it).
  Test 4c (2026-10-08; 4b's files rebuilt by `apptest build` with `APPTEST_GLOVES_FLAG=70701`,
  one bit changed, no. 1's row body byte 10 bit 0, checked against 4b's tables; installed by
  `.tmp/4_0/apptest/test4c.py`): no. 1's hands were bare again, in Edit mode on the 4cc exe
  (the lead, `.tmp/4_0/ingame/t4d_p1_full.png`) and in a match (the maintainer). **The
  PlayerGloves bit does not turn a GloveList.bin row on; on PES 17 the row never applied, with
  the bit at 0 or 1, on either exe.** Pre-Fox gloves stay in the savefile (the full savefile
  aesthetics patch, Test 1's design answers). Also settled by the maintainer: Edit mode shows
  a player as a match does, goalkeepers included, so an in-game look test needs no match;
  only an outfielder's 2nd+ kit needs one.
- answered — referee collars, `referee_collar_<ID>` or `collar_<ID>`
  (Fox answered below: `collar_<ID>`; pre-Fox answered 2026-10-08 at the end: both, the drawn
  one being `referee_collar_<ID>`): the pre-Fox base
  `uniform_config.xml` loads a referee's collar from `nocloth/referee_collar.model` (type
  `referee_shirt`), Red's pre-Fox referee template ships `referee_collar_026.model`, the cup's
  `4cc_04_fpc.cpk` ships `referee_collar_105.model` beside `collar_105.model`, and PES 21's
  `dt35_g4` holds `referee_collar_*.fmdl`. If referees load `referee_collar_<ID>`, the marker
  (4.27) is `referee_collar_077`, teams and referees do not share collar IDs, and the 77
  reservation against teams (`kit_collar_reserved`, the conflict) guards nothing. Also: a
  collar is drawn with the kit texture, so the marker texture becomes every referee kit's
  main texture (maintainer, 2026-10-05; referee models are full-body and ignore it), and the
  bundled marker models need collar materials. Settle before 4.27. Test 5
  (`.tmp/4_0/apptest/out_test5/GUIDE.txt`, 2026-10-05) gives every referee config collar 77 and
  ships different models as `referee_collar_077` and `collar_077`, on PES 17 and PES 21. Found
  building it: on the installed PES 21 the referee configs are entries of `4cc_08_bins.cpk`'s
  `UniformParameter.bin` (`referee_ACL_1..4.bin`, `referee_DEF_1..5.bin`), and the refs CPK
  holds none, against `blue_port.md`'s "never entries of the bins CPK's
  `UniformParameter.bin`" (Red's Fox template ships them loose); PES 17's are loose files in
  `4cc_35_referees.cpk`, as planned. Results (run 2026-10-06):
  - **PES 21:** the referee wore `collar_077` (the hair). Fox referees load `collar_<ID>`,
    so on Fox the plan stands: the marker is `collar_077`, and the 77 reservation against
    teams guards a real clash.
  - **PES 21 configs:** the collar change made in the `UniformParameter.bin` entries took
    effect, so the game reads referee configs from those entries. Settled 2026-10-07
    (maintainer, from earlier tests): the game needs the loose configs and reads the values
    from the entries, for referees and team kits alike; `blue_port.md` now writes both on
    Fox, step 4.27b. 4.27a went ahead on the plan as written
    (2026-10-07, autonomous mode): the marker's model, texture and reservation hold either way,
    and only where the collar-77 configs go would change. Found then: the compiler's bundled
    `UniformParameter18.bin` and `19.bin` hold `referee_ACL_*`, `referee_DEF_*` (and on 18
    `referee_CL_*`) entries with collar 105, carried forward like the installed bin's, so a
    Fox game that prefers entries keeps showing collar 105 under the marker. The test to settle
    it: a refs CPK holding the collar-77 loose configs and `collar_077.fmdl`, with the bins
    CPK's referee entries left at 105; the marker shows, or not.
  - **PES 17:** neither model showed (with Sider). That's inconclusive. Candidate causes:
    - the loose referee configs were not read from the livecpk root;
    - the two models fail to draw as collars. They are Grigori's face and hair, skinned to
      head bones, with `uni_collar` as a material name, which the stock referee collar
      (`referee_collar_026.model`) does not use: its only material is `uni_shirts`.

    Settled 2026-10-08 (the lead, in-game harness, `.tmp/4_0/apptest/test5b.py` and
    `test5c.py`): **PES 17 referees draw `referee_collar_<ID>`, and need `collar_<ID>` to
    exist.** Found first: the cup's `referee_collar_026.model` (`4cc_35_referees.cpk`, Red's
    template file) has no faces, so the referee's usual look has no collar model at all, and
    Test 5's "neither showed" compared with nothing visible; a loose file in Sider's livecpk
    root changed nothing either way (runs A to E there were void for the same reason). Through
    the test CPK (`4cc_90_test.cpk`, the DpFileList's last entry; the base game's
    `referee_collar_026.model` from `dt35_win.cpk`, 3,204 faces, as the visible model):
    - G, that model as `referee_collar_026`, configs untouched: a V collar drawn (the CPK is
      read; the referee loads `referee_collar_026` by name).
    - H, configs 77 and the same file: the referee absent from 48 walkout frames; A, configs
      77 and the visible model as `referee_collar_077` alone: absent too.
    - B, configs 77 and the visible model as `collar_077` alone: the referee present, no
      collar.
    - I, configs 77, the visible model as `referee_collar_077` and the FPC's empty collar as
      `collar_077`: the referee present with the V collar.
    So the loose referee configs in a CPK are read, the referee's drawn collar is
    `referee_collar_<ID>`, and a missing `collar_<ID>` drops him from the cutscene. For 4.27's
    pre-Fox marker: the marker model goes out as `referee_collar_077.model` with an empty
    `collar_077.model` beside it (as the cup's `4cc_04_fpc.cpk` pairs `collar_105` with
    `referee_collar_105`), and the 77 reservation against teams keeps its point, since a
    team's `collar_077` would be the one the referee finds. "Absent" is an inference from the
    referee not appearing in any frame of a walkout where runs B, G and I show him.
- open, waits on FoxDen — Fox per-kit models: the maintainer answered (`DECISIONS.md` "on Fox
  each kit number gets a face package of its own, once FoxDen loads one"); the package layout is
  decided with FoxDen when it gains the support, and `kit_variant_model_fox` and
  `spec/model_format.md` "Kit-dependent assets" change then.
- resolved 2026-10-12 (DECISIONS "the ingame-face run's undecoded bits travel between
  versions") — the unknown bits across versions: per-version tables by record bit position
  dropped hair and sliders when a PES 16 export compiled for PES 21; the run's bits are now one
  table by position in the run, written on every version (review S5.A1 item 8).
- resolved 2026-10-30 (`plans/pes_savefile/operations.md` "Aesthetics patch (new)": a patch
  writing a player's look sets his base-copy id to his own id and clears the base-copy flag; the
  edit flags stay as the save holds them unless 8.36d shows one is needed) — the base-copy ID on pre-Fox:
  `model.md` makes it compiler-owned ("derived from what
  was written"), but no plan says what the compiler derives, and the aesthetics patch, the
  only savefile write path, has no key for it (Team TOML has `base_copy_id`). Settle with
  the pre-Fox patch: the rule and its key, with the edit flags the patch writes (review S5.A1
  item 10).
- resolved 2026-10-15 (maintainer: a Midcup carries only the changed shared folders; DECISIONS
  "a compiled CPK records its shared IDs, and a Midcup keeps them"; step 5.3a builds it) —
  shared IDs and retained rows: a team's shared boots/gloves IDs are assigned in
  alphabetical folder order from the export being compiled (`spec/aesthetics_export/player_folders.md`
  "Assigns IDs automatically"), while a player the export does not compile keeps his installed row
  (`spec/team_compiler/pipeline.md` "Bins accumulation"). A `Midcup` export holding shared folder B but not A
  gives B the ID A had in the full CPK, so A's wearers, their rows untouched, now wear B.
  The same holds for pre-Fox save IDs. Settle before 4.22: a midcup keeps the full export's
  assignment (from where?), or must carry every shared folder its team uses (review S5.A2
  item 1).
- resolved 2026-10-17 (DECISIONS "generation, portraits and the CLI settled"; plan step 8:
  the upgrader generates from the record) — generation from a stripped save: `settings_toml.md` has generation skip a player
  at -1, so an old-format export migrated against the cup's stripped Fox save gets no
  `settings.toml` for him and his next compile writes all-default rows over the look the
  tables hold. Reading his installed table rows (the bins walk) instead would keep it.
  Settle with the Export upgrader's generation, Phase 6 (review S5.10 item 3).
- resolved 2026-10-17 (same decision: no `fpc_on` marker is written, raw settings and the
  players' own models reproduce them) — the Export upgrader's `fpc_on` detection (`export_upgrader.md` step 8, "settings
  match the enable preset"): the cup's FPC players mostly ride their bodies in per-player
  boots IDs, so their boots field holds that ID, not 55, and Test 1's save had every /a/
  player FPC while `is_fpc_player` found none (`.tmp/4_0/apptest/results.txt`, run 2). Matching
  the strip fields alone would also mark a dressed player with long sleeves, tucked shirt and
  short socks. Settle the fields compared before Phase 6 (review S5.11 item 2).
- resolved 2026-10-17 for generation (same decision: a version's unmeasured range is its
  field's bits, for generation and the compiler's narrowing, `plans/aesthetics_export/settings_toml.md`;
  built in Phase 6) — real cup players past the editor ranges: `PlayerSettings::from_player` refuses a
  stored value its key's kind cannot represent (`OutOfRange`, documented as "a hostile save's
  out-of-range bits"), but the PES 18 fixture save holds 79 such players and the PES 19 one 26
  (`cheek_type` 4 to 5 where the range ends at 3, `forehead_type` 6 where it ends at 5, inside
  the fields' bits; found at 5.1b). So a `settings.toml` cannot be generated for them, and
  strip-and-seed, whose rows must equal each player's record bytes (SE-STR-01), cannot take
  the `from_player` route: `appearance_row` itself writes such a value as is (the field's bits
  are its bound), but the Save editor has no public way to build those settings. Settle before
  5.4c (a row straight from a `PlayerEntry`, or wider ranges if the game shows those values),
  and before any tool generates `settings.toml` from a save. Strip-and-seed's half settled
  2026-10-14: its rows come from the record (`EditFile::appearance_row`, step 5.4c1); open for
  generation only, settled with the Export upgrader's generation (Phase 6).
- resolved 2026-10-15 (in-game check; DECISIONS "a base-copied face is any base-copy id other
  than 0 and the player's own"; step 5.4c2 builds the warning) — strip-and-seed and a
  base-copied face, measured 2026-10-14 (step 5.4c; census
  `.tmp/5_4/base_copy_census.txt`, plan "Stripped save"): base-copied faces are reported, not
  carried. The issue: the `PlayerAppearance.bin` row carries no
  base-copy id (`spec/pes_savefile/model.md`, "Player settings model (settings.toml)"), which holds a face-transplant
  relationship (`spec/pes_savefile/operations.md` "Save-to-save operations"), so a stripped player whose
  record copied another player's face may change face, against "stripping changes nobody's
  look". Untested (Test 1 did not cover it). Measure before strip-and-seed is built; then
  carry the relationship or report the affected players (review S5.A3 item 3).
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
  offline 2026-10-15 (`.tmp/face_diff/findings.md`): one format on both engines, Konami's
  diffs carried unchanged from PES 17 to 18; the real split is version 4 (32 entries) against
  5 (33), with a mechanical conversion. The maintainer's example (2026-10-17, a member's
  anime face, `.tmp/face_diff/lapwing/findings.md`): the Fox fix is two eye floats (head 0x2C
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
- resolved 2026-10-13 (in-game; `spec/pes_savefile/model.md` "The player record") —
  unmodeled player-record bits that real saves set: PES 18 bits 252-254 and PES 16 bits 222
  and 362 are moved by no Edit setting and kept through load, edit and save: retained,
  meaning unknown, not editable. The "most likely `Star`" guess has no support.
- resolved 2026-10-15 (Phase 5 converge: `PlayerEntry::appearance_id` models the field,
  strip-and-seed sets it to -1) — `pes_savefile` player id versus appearance id (2.20i reviewer): the appearance
  block's player id (+116 on 17-19, +240 on 20/21) is unmodeled, so changing
  `PlayerEntry.id` leaves it stale. Nothing changes an id today. Step 4.0 settled what the field
  does (-1 = the player's appearance comes from the database tables), so the model gains it with
  the Phase 5 work that strips and preserves it (decision entry "aesthetics travel in the
  database tables").
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
  survey (`.tmp/4_0/apptest`, `fpc` subcommand). Settle with the FPC toggle, built with the
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
  `.tmp/foxden/plan.md`): (5) FoxDen's own build runs proxy-only on every PES 2019 exe here:
  `host_is_game()` wants a static `dinput8.dll` import, which neither exe has, on disk or in
  the running image (the game loads DirectInput at run time); (6) with that check relaxed
  (test copy), the game dies right after the livecpk Lua state is created, on the stock exe as
  on the patched one: `lua_tolstring` resolves to a misaligned address (RVA 0x4bbba5f, the
  pattern's second pass), most likely mid-function. FoxDen session 4 (2026-10-20, its new
  version on the same stock exe, scratch `.tmp/foxden/plan.md`): (5) and (6) are fixed (the
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
- done (2026-10-13; opened 2026-10-12, the maintainer) — the ATF bot's author forked Rigdio
  (v2.2.1: a local API and event clips) and rebuilt SEN:P-AI (v1.3.0: PES 2019 and 2021
  support, events sent to Rigdio's API); both are cloned in `C:/Data/4cc/Tools_Marco/`.
  Surveyed 2026-10-13: the rulings are in the music player and match tracker plans and
  DECISIONS (the fork survey's rulings), three questions in QUESTIONS; the plans name
  upstream and the forks apart, and the version scope is decided (PES 2019 first, then PES
  2021, then the others).
- resolved 2026-10-13 (in-game, PES 18; `aesthetics_export/settings_toml.md`, the paragraph
  "What a 0 draws was measured on PES 18") — what a hair list at 0 draws: a 0 in `length` or
  `hairline` draws a per-face fallback hair, not a look of its own; a 0 in a list shaven
  greys out changes nothing; changing the length writes 1 into every list at 0. The keys keep
  taking 0.
- resolved 2026-10-14 (5.4a6; `studio_core::pes_process::running_pes`) — no "PES is running" probe exists; `savefile_skipped_pes_running`
  is plan-only. The Save editor's writing commands cannot refuse while the game runs
  (SE-CLI-02) until one lands (`studio_core`'s running-PES poll, `CONTRIBUTING.md`'s `unsafe`
  list).
- resolved 2026-10-14 (5.4a5; `PipelineEvent::Output`) — a tool has no way to print a whole document to stdout: `Message`
  lines are prefixed and folded by the console printer, `println!` is the binary's alone.
  `diff --json` (SE-DIF-02 says stdout) waits on a `studio_core` output event the printer
  writes verbatim.
- resolved 2026-10-14 (5.0m3; the plan keeps him refused: 183 is past every version's top)
  — one PES 16 fixture cup player holds
  `goal_celebration_2 = 183`: PES 16's editor shows "122" (its top) for him and keeps the 183
  through a save of another row; what a match draws for 183 is unmeasured. `from_player`
  still refuses him (`OutOfRange`); `every_fixture_player_round_trips_through_his_settings`
  lists him.
- open (2026-10-15, step 20.0c) — a `stat_bonus` on a stat a tier already targets at 99 gives
  an exact target no player can reach (99 is the game's top), and nothing says so: the first
  Clouds template's "Medals at giga: +2 to every stat" made every gold 101, so `check-aatf`
  placed no player in gold (a TierCount violation on every VGL26 team) instead of naming the
  rule. `aatf_rules.md` is silent. Settle with the next interpreter step: clamp a raised target
  at 99 (the bonus then means nothing for that tier), or refuse it on load where a tier's
  value plus a special's bonus passes 99 (a special's condition may never meet that tier, so
  a load rule could refuse a playable file). The same logic version change fixes the Phase 5
  converge audit's finding (`.tmp/5_6/audit_shapes.md` row 1.7): an explicit `()` on an
  optional `Condition` or `Effects` key (`captain: ()`, `fielded_only_at: ()`, `forbid: ()`)
  reads as `None` in Rust but as present in the logic, which tests the key's presence; the
  logic's `num`/`list`/`table`/`flag` helpers already treat `()` as absent. Only a hand-written
  file reaches it.

- open 2026-10-29 — the 120 s hang guards in the Save editor's tests (`frames_until`,
  `portraits`' `served`) cost a mutation run up to 120 s per mutant that silences a worker
  (they were 2 s and 30 s before, both failing correct code under load). At the next
  `save_editor` mutation run, read those mutants' durations from its outcomes; if they add
  notable time, lower the bound to the longest a loaded machine was measured to need

---

## Log

The dated log is `docs/LOG.md`.
