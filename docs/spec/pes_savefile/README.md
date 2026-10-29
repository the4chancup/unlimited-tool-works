# Unlimited Tool Works — Savefile plan

Covers the `pes_savefile` lib crate: the EDIT00000000 file codec (container crypto,
schema-driven payload codec, player/team/tactics model), the cross-version player
data conversion, the save-to-save operations (the comparator, aesthetics
fingerprinting), and the interchange formats (Team TOML, legacy `.4ccs`/`.4cct`
readers, Texport, the aesthetics patch). Shared by the [Save editor](../save_editor.md), the
[Team compiler](../team_compiler/README.md), the integrated
[model conversion](../model_conversion/README.md), and the
[Export upgrader](../export_upgrader.md). Platform context is in the
[core plan](../core/README.md).

---

The plan is split into parts; section headings are unchanged from the
single-file plan.

| Part | Covers |
|---|---|
| [Container format and crypto](container.md) | Container format and crypto; Payload layout per version |
| [Save codec](codec.md) | Schema-driven save codec |
| [Data model](model.md) | Player/team/tactics model; Player settings model (settings.toml) |
| [Operations](operations.md) | Cross-version player conversion; Save-to-save operations; Interchange formats; Player section population |
| [Verification](verification.md) | Verification |

## Origin material

The crate consolidates savefile knowledge that today is spread across four codebases:

| Source | Location | Contributes |
|--------|----------|-------------|
| 4ccEditor | `Tools_4cc/4ccEditor-1` (an unofficial fork of the main `Tools_4cc/4ccEditor` with added functionality — strictly better despite being cloned first; use the fork as the reference) | Per-version payload schemas (`pes15.cpp`–`pes20.cpp`), player/team/tactics model (`editor.h`), bit engine (`data_util.cpp`), playstyle conversion tables (`menu_lists.cpp`), `.4ccs`/`.4cct`/Texport formats |
| Midcupping | `Tools_4cc/Midcupping` | Pure-Python reference implementations of the container crypto (PES 15's keyless scheme; PES 16/17 incl. their master keys), aesthetics block layout, transplant and aesthetics-diff logic |
| The converters | `Tools_4cc/4cc-aet-converter-19to16`, `Tools_4cc/aes_converter_16to21` | Container crypto for PES 16/19/21 (`save16.py`/`save19.py`/`save21.py`, incl. master keys), `convertPlayerSaveData` cross-version bitfield surgery |
| pesXdecrypter / libpesXcrypter | `Tools_4cc/pesXdecrypter` (the source of the DLLs; built copies in `Lab/4ccEditor/lib` and `Saves/4ccEditor_betaM/lib`) | All seven master keys (`src/masterkey.c`, incl. the PES 16 myClub variant) and the authoritative container spec (`src/crypt.c`) |

---

## Crate layout

The crate is organized in layers — bytes → fields → model → operations — and the layout exists to
keep per-version knowledge in exactly one layer. Everything version-specific (keys, container
shapes, bit offsets, payload section offsets) lives under `schema/` and `container/` as *data*; the
codec, the model, and every operation above them are version-generic and take a `&VersionSchema`.

```
crates/libs/pes_savefile/src/
├── lib.rs              # re-exports (PesVersion comes from the `pes_version` leaf crate, see ../libs/README.md)
├── file.rs             # EditFile: load (auto-detect) / save (.bak), retained unmodeled bytes
├── discovery.rs        # Documents\KONAMI layout per version → SavefileCandidate list
├── fixtures.rs         # the fixture payloads (each version's payload.bin.zz inflated) behind test_support and `fixture_save`
├── test_support.rs     # the fixture containers/saves the test modules share
├── container/          # bytes ↔ decrypted sections (`container.md` "Container API")
│   ├── mod.rs          #   SaveContainer, Scheme, ContainerError; decrypt (key/shape trial), to_bytes
│   ├── pes16_21.rs     #   the shared 16–21 container (salt, header, MT19937 stream, integrity)
│   ├── pes15.rs        #   the PES 15 LCG/MD5 chunk container
│   ├── mt19937.rs      #   keystream generator (known-answer tested)
│   └── keys.rs         #   MasterKey: the seven master keys, transcribed from masterkey.c
├── schema/             # per-version field tables — data only, no logic
│   ├── mod.rs          #   FieldSpec, VersionSchema, SectionLayout; schema_for(version)
│   ├── fields.rs       #   PlayerField / TeamField / TacticsField / RosterField enums (the version-neutral vocabulary)
│   ├── ingame_face.rs  #   IngameFaceField + the ingame-face run's FieldSpec rows
│   ├── limits.rs       #   per-version face-type caps (operations.md "Cross-version player conversion")
│   ├── playstyle.rs    #   one playstyle list per version group + decode/encode
│   ├── pes15.rs        #   player + separate appearance tables
│   ├── pes16.rs        #   split player/appearance blocks
│   ├── pes17.rs        #   first unified block (+ phys_cont)
│   ├── pes18.rs        #   new container header, unified block
│   ├── pes19.rs        #   + star, 39 skills
│   ├── pes20.rs        #   20/21 shared (+ mo_drib, tight_pos, aggres, play_attit, strong_hand; 41 skills)
│   ├── instruction.rs  #   canonical advanced instruction ↔ per-version stored value
│   ├── texport.rs      #   the 18–21 Texport layout (key index, size, header/coach/tail templates), 15–17 payload offsets
│   └── *_golden.rs     #   literal-offset goldens (instruction, playstyle)
├── codec/              # the one generic engine
│   ├── mod.rs          #   CodecError; the read/write dispatch over a &VersionSchema
│   ├── bits.rs         #   bit-run reads/writes crossing byte boundaries (data_util.cpp's job)
│   ├── player.rs       #   read_player / write_player (and the 15/16 appearance record)
│   ├── team.rs         #   read/write for team, roster and tactics records
│   └── tests.rs        #   codec tests over the fixture payloads
├── model/              # what consumers edit
│   ├── mod.rs
│   ├── player.rs       #   PlayerEntry: abilities, skills, appearance, playstyles
│   ├── team.rs         #   TeamEntry: identity, roster, kit refs, colors
│   ├── tactics.rs      #   presets, formations, advanced instructions
│   ├── instruction.rs  #   Instruction: the canonical (17-based) advanced instruction enum
│   ├── ingame_face.rs  #   IngameFace: the opaque run plus typed accessors
│   ├── playstyle.rs    #   PlayStyle: the canonical playstyle enum
│   └── names.rs        #   UTF-8 names, colour-code stripping (display_name)
├── settings_toml/      # PlayerSettings (the settings.toml model) ↔ PlayerEntry merge
│   ├── mod.rs          #   PlayerSettings, AppearanceSettings, SettingsError, the classification tables
│   ├── document.rs     #   the document text: header, keys, parse/emit/update; the shared [appearance] walkers
│   ├── keys.rs         #   the SettingKey table (table, name, kind, comment, default)
│   ├── appearance_row.rs #  the Fox PlayerAppearance.bin row a PlayerSettings produces
│   └── tests/          #   settings and template tests
│       ├── mod.rs      #     the shared prelude
│       ├── model.rs    #     the completeness invariant, from_player, apply
│       ├── document.rs #     parse, to_toml, update_toml
│       └── appearance_row.rs # the row's bytes and the pre-Fox refusal
├── player_tables.rs    # the three Fox player tables (PlayerAppearance/BootsList/GloveList.bin), one PlayerTable<VALUE>
├── ops/                # save-to-save operations
│   ├── mod.rs          #   the module wiring
│   ├── fingerprint.rs  #   aesthetics fingerprinting / diff
│   ├── compare.rs      #   comparator (gameplay + aesthetics)
│   ├── fpc.rs          #   maps libs/fpc player presets onto PlayerEntry; interference check
│   └── *_golden.rs     #   parity goldens (appearance block, compare)
└── interchange/        # team interchange formats (Team TOML is the one import path)
    ├── mod.rs
    ├── team_toml/      #   Team TOML read/write (full fidelity): document, key tables, apply
    │   ├── mod.rs      #     TeamToml, TeamTomlError, ImportNote
    │   ├── team.rs     #     [team] emit, parse and apply; the TeamToml entry points
    │   ├── player.rs   #     [players.NN] emit, parse and apply
    │   ├── tactics.rs  #     [tactics] emit, parse and apply
    │   ├── items.rs    #     the shared item readers, emit line and apply-side checks
    │   ├── player_keys.rs #  the [players.NN] scalar-key table (player level, stats, positions, edit_flags)
    │   └── labels.rs   #     the label vocabularies (positions, ratings, styles, skills)
    ├── legacy.rs       #   .4ccs / .4cct readers, into a TeamToml
    ├── legacy_golden.rs#   the .4ccs parity + literal-offset goldens
    ├── aesthetics_patch/ #   the compiler's aesthetics_patch.toml, the Fox half: parse, emit, apply
    │   ├── mod.rs        #     AestheticsPatch, PatchTeam/PatchPlayer, PatchError, PatchSummary
    │   └── tests.rs      #     the patch's tests
    ├── texport.rs      #   Texport read/write over the schema codec (layout: schema/texport.rs)
    └── texport_golden.rs # the Texport parity goldens
```

Placement rules:

- **A bit offset appears in `schema/pesNN.rs` or nowhere.** No literal offsets in `codec/`, `model/`,
  or `ops/`; a new field is a `PlayerField` variant plus one table row per version that has it.
  This is what makes the roundtrip tests mechanically verifying.
- **`container/` knows nothing about players; `codec/` knows nothing about encryption.** `file.rs`
  is the only module that composes them.
- **`model/` types have no I/O and no version.** A `PlayerEntry` is the same struct for every
  version; fields a version lacks are `Option`/defaulted and the schema decides what is written.
- **`ops/` and `interchange/` consume `model/` only.** They never touch bytes; if one needs a
  version-specific fact, that fact is a schema query, not a match on `PesVersion`.
- **Reference constants keep their provenance in a doc comment**: each table module says which
  reference read walk it was derived from (by version and record kind, not by tool name:
  `../../CONTRIBUTING.md` keeps legacy tool names out of code), and `codec.md` "Schema-driven save codec"
  holds the derivation method, so a mismatch found by a roundtrip test can be traced in one hop.
- Lib dependencies are `fpc` (the FPC system's data) and nothing format-specific; no Blender, egui,
  or tool dependency: this crate is consumed by `aatf`, the Team compiler, the Save
  editor, the Export upgrader, and the Player aesthetics editor, and must stay `wasm32`-checkable
  (core plan guardrail 6); discovery's filesystem walk is `cfg`-gated for non-desktop targets.

---
