# Unlimited Tool Works — Team compiler plan: Testing

Part of the [Team compiler plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## Testing: parity against Red (adapted from Blue)

Blue has `test_parity.py` that runs the full pipeline, compares ordinary leaf files byte-for-byte,
and compares nested CPKs by entry equivalence so container timestamp differences are accepted.
The Rust test expands Blue's comparison into the four tiers below.

```text
cargo test -p team_compiler --test parity
```

The parity harness supplies the reference-fixture location; it is not a custom `--reference` flag
passed through to Rust's standard test harness, which does not accept that option.

The reference tree is the **extracted contents** of Red's output CPK (Blue's `Singlecpk_files`
convention) — Red itself emits `patches_output/{cpk_name}.cpk`, so the fixture is produced by a
one-time extraction step. A reference tree small enough to commit is
committed as the tree itself: the tracer bullet's (`tests/fixtures/tracer/red/`, under
1 MB), the pre-Fox tracer's (`tests/fixtures/tracer_prefox/red/`, PES 2017, about 660 KB) and the
referee tracer's (`tests/fixtures/tracer_refs/red/`, PES 2021, about 4.7 MB) run their parity cases anywhere, CI
included, without Red.

Each team tracer has two cases: its hand-made Studio twin (`studio/`), and the Export upgrader's own
output for the old export (`upgraded/`, a golden: tool crates never depend on each other, so
`export_upgrader`'s `tests/upgraded_golden.rs` checks that upgrading `old/` still gives exactly
that tree, and the parity case compiles it). The upgraded case compiles the golden under the
twin's `Midcup` tag: the upgrader tags it `Full`, and a `Full` export missing a player kit gets
the compiler's placeholder kit, which the tracers' one-kit cut would add to the comparison
without telling anything about parity. Its expected differences from the twin's case are what
the member's own `face.xml`, which the upgrader keeps, decides on pre-Fox: the face model packed
under the name it lists, its entries as the member wrote them, and its `boots` typing's warning.

The referee tracer (`tests/fixtures/tracer_refs/`, one referee of a Fox referee export in the
old `face/`, `boots/`, `gloves/`, `common/` layout, mapped to three slots) has the upgraded case
alone, the Export upgrader's flattened output compiled against Red's referee CPK for the old
export (`tests/parity.rs`): the compiler reads no old layout, so the plan's "compiles
identically to the unflattened original" is this comparison. A referee export takes no coverage
tag, so the golden compiles under its own name. Its differences from Red are a referee folder
with no face model getting no face (Red packs his `face_diff.bin` alone), the dropped
`*.fpk.xml` files, and the team side written for the bins on PES 18-21, the referee kit configs
being entries of `UniformParameter.bin` (the fixture's README lists each).

Beyond the tracers, whose small trees are committed, the parity over the maintainer's whole
library of old exports is a **measurement rerun at each phase close** that changes the Team
compiler or the Export upgrader, not a committed test: every input the upgrade census upgrades
clean is upgraded, compiled, compiled by Red from a fresh copy, and compared by `red_parity`
(`team_compiler/examples/red_parity.rs`, one classified line per Red entry), and every line is
attributed to an intentional class or a defect (`scripts/library_parity/classes.md`), a new
class diagnosed before the phase closes. We do not commit a hash manifest of Red's trees as a
fixture, because the inputs are members' exports on the maintainer's machine alone (several
hundred, several GB), so no such test could run in CI; and because our output differs from
Red's by design in some thirty classes, so a manifest of either output would be a golden that
every intentional change rewrites, while the attribution, which needs judgement on each new
class, is what the run is for. One run takes about 20 minutes of Red and 17 of ours over 250
exports, with three workers. The scripts and the order of a rerun are under
`scripts/library_parity/` (its `README.md`).

Intentional differences must be accounted for:

- **Auto-assigned boots/gloves IDs** may differ from the old embedded IDs — the comparison
  normalizes ID-dependent paths and embedded references.
- **Texture locations**: every player's textures are relocated to per-player common subfolders (see
  "Texture relocation to common" in the walkthrough), where Red packs an AET-layout export's
  textures inside each model folder's structure and relocates a Pre-Studio (`Players/`) export's
  to `common/<team>/<name part>/`, keyed by the folder name without its number prefix; textures resolved from `Common/` stay in the team's Common output and
  are referenced in place. The comparison matches textures by content across locations and verifies
  that rewritten references resolve to the relocated files (`check_resolved`); that a Common
  texture appears once in the archive however many players reference it is
  `tests/cli/common_links.rs`'s check, the tracers holding no `Common/`.
- **Automatic collar reconciliation**: unlike Red's pass-through behavior, the compiler derives the
  custom collar ID from the `collar_[ID]` filename and rewrites the team's kit configs;
  `tests/cli/collars.rs` verifies those config-field changes, the tracers carrying no collar.
- **Dropped features** (Other folder) are excluded.
- **Kit-dependent path magic**: Red emits the legacy `u0XXXp0`/`u0XXXp1`… spelling (with the `XXX`
  placeholder or a baked-in team ID, as the author wrote it), the compiler emits `kitN`/`kit1`…
  (see "Kit-dependent assets" in the [Unified model format plan](../model_format.md)). The
  compiler gets no switch to emit the legacy spelling: parity is a
  file-tree comparison, not an in-game test, and the exes can be modded to `kitN` at any time, so a
  switch would be a second output format maintained for nothing.

Comparison uses four tiers:

1. **Exact leaf bytes** for unaffected files such as DDS textures and raw bin outputs.
2. **Decoded and normalized comparison** for FMDL, MTL, XML, kit configs, and other ID/path-bearing
   content; generated IDs and relocated paths are normalized before remaining fields are compared.
   FTEX files are compared by their header fields (`ftex::info`, texture type included) and by
   the DDS each converts back to (`ftex::ftex_to_dds`), not by their own bytes: their mip data
   sits in zlib chunks, and a different deflate encoder writes
   different chunks for the same pixels (the `ftex` crate's stated non-goal). The tracer's two
   FTEX entries differ from Red's in exactly that way and convert back identically.
3. **Archive-entry equivalence** for CPK/FPK containers, whose container bytes may differ because of
   ordering metadata or timestamps while their normalized entries agree.
4. **An explicit intentional-difference allowlist** reviewed with the fixture, rather than broad
   exclusions that can hide regressions.

Maintain a versioned test matrix for PES 15–21 covering team and referee exports; folders, ZIP, and
7z; native and converted pre-Fox/Fox models; local, shared, and combined models; normal, multi-CPK,
test, and sideload modes; savefile success/failure and cancellation. Key behavior areas:

- **Rosters and identity**: an export stem such as `co - Spring.zip` resolving through `/co/`; a
  folder and archive with identical display stems staying distinct (including in test-mode output
  directories); referee identity never constructing a normal `TeamId`; duplicate folder-name slots
  vs duplicate authoritative `players.txt` slots and their distinct dispositions; line-local
  malformed and out-of-range roster entries producing `DropSlot`; an empty/all-invalid referee
  roster blocked while an empty normal-team roster supports a kit-only export; a refs export without
  `players.txt` blocked by the compiler; multiple refs exports
  (valid or not) all blocked at discovery; team-ID resolution failure; duplicate team IDs;
  multi-mapped folders (team and referee) preparing once and instantiating per slot
  atomically, with textures emitted once into the name-keyed common subfolder shared by all mapped
  slots.
- **Models and textures**: target-native plus convertible model variants; each model source owned
  exactly once; a team player with boots/settings but no face; multiple same-category shared links
  rejected; both `ingame_face` spellings normalizing identically with a local pre-Fox boots
  composite still emitted; FMDL→pre-Fox conversion emitting an atomic `.model`+`.mtl` pair;
  shared-texture dedup and
  deterministic conflict resolution under reversed rayon completion; logo production (a 1000×600
  source under each fit mode yields the three square sizes; `logo_small` replaces only the 128²;
  an undecodable `logo_small` drops all three; a 300² source is upscaled and reported); kit
  folders (`p1 - Lakers` resolving to `p1` with the label kept; `p1/` beside `p1 - Lakers/`
  discarding both; `all/kit_back.dds` emitted under every kit's ID with each config's `back` field
  set, while a kit's own `kit_back.dds` wins for that kit only; the emitted files identical to a
  compile of the same export with the shared files copied into each kit folder; `all/config.toml`
  ignored with a warning; an empty `p3/` yielding the checkerboard texture, the mask template, the
  template config and a UniColor entry carrying the magenta/black pair, with `kit_placeholder`
  and `kit_colors_missing`; the same `p3/` with a `colors.txt` carrying its colors and no
  `kit_colors_missing`; a `p4/` holding only `kit_back.dds` yielding the same plus `_back` and a
  config whose `back` field is set; a kit with a real texture and no colors deriving from the
  texture, never reaching the loud pair); kit layout (a `pre-fox` kit compiled for PES 21: every
  texel outside the sock rectangles equal to the no-marker compile's, each sock stripe's centre
  within 6 px of the golden computed from the games' uniform models
  (`tests/fixtures/kit_layout/`), `kit_layout_converted` reported; the same kit compiled for PES 17 identical to the no-marker compile; a `fox` kit for
  PES 17 taking the inverse table, and pre-Fox→Fox→pre-Fox on a synthetic texture whose bands are
  flat colors round-tripping exactly; `_back`/`_leg`/`_name` untouched either way; both markers
  discarding the kit; a marker in `all/` warned and ignored; a placeholder kit with a marker
  compiling as without one); mask/srm (a kit with `kit_mask.dds` compiled for PES 21 emitting no
  `_mask` and no `_srm`, with `kit_texture_not_used`; the same kit for PES 17 emitting the mask
  as given; a kit with `kit_srm.dds` for PES 17 emitting the mask *template* and no srm; a kit
  with both emitting exactly the target's one, the other named by `kit_texture_not_used`); user `face.xml` (a hand-written xml
  compiled for PES 17 emitted with the team ID substituted and its unknown `type` and extra
  attribute kept verbatim, each with its warning, and `level="1"` kept with `xml_level_lod` as
  info; the same folder with the xml removed
  producing the generated xml, proving filename typing is off when one is present; a `<model>`
  without `path` and a `./` reference to an absent file each discarding the folder; a
  `model/character/face/common/…` path passing with `xml_path_unchecked`; a folder with a `<dif>`
  in the xml and a `face_diff.xml` beside it discarded; an xml lacking `face_neck` gaining the dummy
  entry; an unlisted `.model` reported and absent from the output; the same folder compiled for
  PES 21 ignoring the xml and converting the models); collar validation (`collar_[ID]` filename parsing, a cross-team
  `collar_id_conflict` resolving by canonical export order, and a custom collar overriding the FPC
  collar value in every config).
- **Planning and writer**: cross-export duplicate paths producing the same manifest under reversed
  completion order; override-versus-export collision resolution; nested-root path collisions;
  partial planning continuing valid teams after refs or duplicate-team drops; a plan with no
  eligible tasks producing no output or deployment; a structural `DropFolder` still yielding a
  compilable export; a late model-task failure leaving no entries from that task while successful
  sibling tasks may still commit; a failed kit never mutating UniColor/UniformParameter; notes from a rejected export excluded from
  `teamnotes.txt`; root colors/notes/referee-marker descriptors reaching planning; scope-dependent
  `source_read_failed` and `template_override_unreadable` outcomes; pass-through refusing
  conversion/packing/logo/texture-conflict findings and
  unsafe-path/ambiguous-roster/required-metadata findings.
- **Deployment and environment**: the installed ID kept for players whose boots/gloves task failed
  (Fox: the player's installed table row; pre-Fox: his savefile ID) while independent settings
  still apply; with either FPC marker, successful standalone assets
  overriding preset IDs, failed/dropped requested assets preserving existing IDs, and absent
  standalone assets using preset IDs (including pre-Fox face-XML-local parts); a mixed
  `fpc_on`/`fpc_off` team applying per-player presets; source modification after planning producing
  `source_changed_during_run` (no final output published, no installed CPK or savefile changed);
  separate refs CPK installation; multi-CPK deployment failure; a mixed `fpc_on`/`fpc_off` team
  giving every kit config the FPC values; an FPC team's unexported kit slots
  patched from the installed cup entries (Fox and pre-Fox), including the no-existing-entry
  `kit_config_fpc_unpatched` warning; logs/overrides/output path resolution in portable and
  user-config modes; invalid-UTF-8 and empty `notes.txt`; invalid `settings.toml` leaving the
  player compiled with default settings while models continue.
- **Infrastructure**: oversized memory requests, MemoryBudget wake-up stress with multiple oversized
  waiters, and automatic worker counts on one- and two-core hosts; solid-7z reads not copying the
  archive buffer per entry; a solid 7z with unnumbered players; `CpkStem` boundary cases (empty, 29
  characters, spaces, separators, reserved device names, explicit `.cpk`, case collisions);
  standalone
  kit-config binary → TOML → binary preservation of source texture-name fields.

Separate **Red parity** (normalized archive-entry equivalence is acceptable) from **Rust
reproducibility**. Explicit CPU mode is the deterministic reference: identical inputs and encoder
configuration must produce byte-identical CPK, test, and sideload artifacts, including canonical
ordering and normalized timestamps. Savefiles use fresh randomized encryption salts, so compare
them after decryption and semantic normalization, or inject a deterministic RNG in tests.

---
