# PES player skeletons (`.skl`)

The games' own player skeleton files, one folder per PES version, extracted verbatim from the game
data (pre-Fox files were WESYS-compressed inside the CPK and are stored here decompressed). They are
the source for the per-version skeleton constants described in the Model conversion plan
("Skeleton data", "Skeleton retargeting and bone conformance") and are embedded in the Studio
binary; regenerate the constants from these files, never from the legacy `PesSkeletonData.py`.

Format: 12-byte header (`u32 magic = 12`, `u32 bone_count`, `u32 record_size = 56`), then one
56-byte record per bone (`u32 name_offset`, `i32 parent_index`, `f32[12]` row-major 3×4 bind
transform), then a null-terminated name table. Reference parser: `examples/skl.py`.

| Version | Source | Files |
|---|---|---|
| pes15 | `PES2015/Data/dt32.cpk` → `common/character1/model/character/body/body.skl` | body (76 bones) |
| pes16 | `PES2016/Data/dt32_win.cpk` → same path | body (70) - byte-identical bone set and transforms to pes17 |
| pes17 | `PES2017/Data/dt32_win.cpk` → same path | body (70) |
| pes18 | `PES2018/Data/dt00_x64.cpk` → `Asset/model/character/#Win/common_package.fpk` → `/Assets/pes16/model/character/common/*.skl` | body (76), face (49), hand_l (24), hand_r (24), boots (4) |
| pes19 | `PES2019/Data/dt00_x64.cpk` → same | body (116), face, hand_l, hand_r, boots |
| pes21 | `PES2021/Data/dt00_x64.cpk` → same | body (175), face, hand_l, hand_r, boots |

PES20 is not included (never used in a 4cc event, not installed); the plan assumes it equals pes21.
Pre-Fox games ship no face or hand skeleton files; the pre-Fox targets use pes19's for those.

`boots.skl` is the game's real four-bone boots skeleton (`sk_foot_*`, `dsk_toe_*`), kept for
reference. It is **not** the compiler's `boots.skl` template: that is `body.skl` renamed, because 4cc
exports put full-body models in the boots folder (see "SKL pairing" in the Compiler's plan).
The boots Konami ships (`dt33_g4.cpk`, `Asset/model/character/boots/<id>/#Win/boots.fpk`) carry
a six-bone copy of it, `dsk_foot_*` added at `sk_foot_*`'s pose; its pose is the same on PES
2017's stock `.model` boots (measured 2026-10-09, to 0.0000) and 7° off the body table on every
version, so the converter needs no boots table (`conversion.md` "When re-binding changes
anything").
