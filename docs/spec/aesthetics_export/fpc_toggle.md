# Unlimited Tool Works — Aesthetics export plan: FPC toggle

Part of the [Aesthetics export plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## FPC toggle (`fpc_on` / `fpc_off` marker files)

FPC (Full Player Customization — see the [Save editor plan](../../plans/save_editor.md)) makes the default
player model invisible via a specific mix of appearance settings (blank-model boots/gloves IDs
plus strip settings; Fox: fields of the compiled
`PlayerAppearance.bin` row, the IDs in the `BootsList`/`GloveList` rows; pre-Fox: savefile
fields), so that an FBM (Full Body Model) replaces
the player entirely. Setting it through individual `settings.toml` keys would be error-prone;
instead it gets a folder-level toggle, consistent with the principle that the
folder view tells the whole story about how the player's models render:

- Tolerances, in the same spirit as link files: `fpc_on.txt`, and a bare `fpc` or `fpc.txt`,
  read as `fpc_on` (presence reads as "on"); `fpc_off.txt` reads as `fpc_off`.
- Both markers in the same folder raise `fpc_conflict` (error; folder discarded).

**Team kit-FPC status and kit configs.** FPC also requires settings on **every one of the team's kit
configs, including the goalkeeper kit** (modern system, per the [wiki's PES17 FPC
guide](https://implyingrigged.info/wiki/Pro_Evolution_Soccer_2017/Full_Player_Customization): shirt
model 176, shorts model 16, collar 105, winter collar 105 — the same on every supported version,
stored in `libs/fpc` alongside the presets; the retro pre-2024 system differed and is not
supported). These kit values are a **team-wide
prerequisite that enables per-player FPC**, not a per-player switch: with them in place, each
player's own appearance settings decide whether that player's body is hidden, and non-FPC (head-only)
players render normally on the same team. The markers are therefore strictly **player-level** —
`fpc_off` says "this player needs its body", not "this team's kits must not be FPC" — and mixed
teams (some `fpc_on` folders, some `fpc_off` or unmarked) are ordinary, supported usage;
`fpc_conflict` only rejects both markers inside *one* folder.

An export's **team kit-FPC status** is two-state — `EffectiveTeamKitFpc::{On, Unknown}`:

- **On** when at least one player folder of the validated export carries `fpc_on` (a folder
  validation dropped compiles no player, so its marker does not count) — the configs must then carry the FPC
  values for that player's hiding to work; **Unknown** otherwise — the absence of `fpc_on` markers
  makes no claim about the team (on pre-Fox its FPC players may live only in the savefile, set
  through the save editor; on Fox a stripped record's appearance bytes are not read, so there is
  no savefile-side FPC state), and `fpc_off` markers contribute nothing here because they are
  per-player statements.
- **Generated kit configs** (the `kit_config_generated` path, when a kit folder has no
  `config.toml`) are the template, which carries the FPC values whatever the status: it is the
  cup's own generic kit config, and most of the cup's teams are FPC teams (`libs/fpc.md` has the
  counts). So a generated config never needs reconciling.
- **Supplied kit configs are reconciled only upward** (`kit_config_fpc_adjusted`): On → the FPC
  values are written into every config, which also protects against the classic user error of
  copying a kit config from another team. Unknown → supplied configs are left untouched. The
  compiler **never automatically reverts** FPC values found in supplied configs — no export state
  proves the team stopped using FPC, so de-FPC'ing a team's kits is a deliberate manual config edit
  (a future explicit team-level mechanism could revisit this).
- **Kit slots absent from the export are patched in place, not demanded back.** A midcup (partial)
  export can add an FPC player to a team whose kits need no other changes; requiring the untouched
  kit exports to be resent just so the compiler can see them would be easy to forget. When the
  status is On, the compiler edits the team's existing kit entries itself. Fox: the team's entries
  in the working `UniformParameter` get the FPC values — the working bins come from the installed
  cup CPKs (the bundled bases only serve from-scratch compiles), so a midcup compile patches the
  current cup state, and the `uniparam` container format is fully parsed (see the [library crates
  plan](../libs/README.md)), so locating a team's entries is routine. Pre-Fox: the team's current kit-config
  bins are located in the same installed CPKs, patched, and re-emitted: the walk that finds the
  working bins gathers them as it passes (nearest CPK first, so each is the copy the game loads),
  and a patched one is written where the edited bins go (the bins CPK with teams parts, the test
  prefix in test mode), not with a team's content, because it is an edit of the installed state
  as the bins are, not something the export supplied. Patched slots report
  `kit_config_fpc_adjusted` like supplied configs; a slot with no existing entry or config to patch
  reports `kit_config_fpc_unpatched` (warning) — that team genuinely needs a kit export. The
  slots are the kits the game offers the team (its `UniColor.bin` record), not all ten
  (`spec/team_compiler/pipeline.md` "Bins accumulation").
- **A custom collar wins over the FPC collar value**: collar rewriting (see "Collars" in the [Team compiler plan](../team_compiler/README.md)) runs after
  FPC reconciliation, so an FPC team with a custom collar keeps its replacement collar ID while the
  other FPC kit values stand.

The kit-config field logic this needs (reading/writing the shirt/shorts/collar model fields, plus
the compile-time binary emission) lives in **`libs/kit_config`**, shared with the [Kit config
editor](../kit_config_editor.md) tool; the FPC values themselves, the player presets, and the
interference rules come from the leaf crate **`libs/fpc`** (see [libs](../libs/README.md)), which `kit_config`
and `pes_savefile` both consume — the FPC system has exactly one description, so the compiler, the
kit config editor, and the save editor can never drift.
