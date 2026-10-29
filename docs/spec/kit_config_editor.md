# Unlimited Tool Works — Kit config editor plan

This plan specifies **`libs/kit_config`**, the lib crate holding the format
knowledge, shared by the Kit config editor, the [Team compiler](team_compiler/README.md) (config
generation, FPC reconciliation, compile-time emission), and the
[Export upgrader](export_upgrader.md) (binary → TOML migration) — one crate owns
the format so all three agree on what a valid config is. Platform context is in
the [core plan](core/README.md).

---

## Background

A kit config is a **120-byte binary file** that tells PES how to render a kit:
which shirt/shorts/collar models to use, five fallback colors, number and name
placement, sleeve-badge positions, and which kit textures exist. In game they live
under `common/character0/model/character/uniform/team/{team_id}/` with names like
`702_DEF_1st_realUni.bin`; in Fox-engine versions they are packed into
`UniformParameter.bin` (the `uniparam` lib's container format; Red bundles separate
PES 18 / 19 fallback base files whose stock entries differ, but the container layout
is the same). Every kit in every aesthetics export ships one.

| Tool | Role | Status |
|------|------|--------|
| Kit Manager PES2016 v4.0 (GOALARG) | Kit config editing GUI | Closed source, abandoned |
| Kit Manager V4 "pes2021-4cc" | The same exe, hex-patched for PES2021's shirt-name offsets | Ditto |
| Kit Manager PES2016 v5.0 (GOALARG, six weeks after v4.0) | v4.0 with collars listed 1–109 and three more collar previews; same format, same editing behavior | Ditto |

### Relationship to the old codebase

There is no old codebase: no sources, no format specification anywhere. The format
below was **recovered by disassembling the editor** (capstone scripting over the
~50KB of actual program code — the other 51MB of the exe is prerendered preview
bitmaps): every access to the loaded config buffer was cross-referenced with the
control-creation and message-handler code, giving field offsets, bit packing, value
ranges (combo item counts), and cross-field constraints. Diffing the PES2021 patch
(47 changed bytes) isolated the one field that differs between game versions. The
byte-level behaviors Red/Blue already implemented (texture-name rewriting at 0x28+,
the PES15 shirt-pattern tweak) agree with the recovered layout.

**This section is now the format's reference documentation** — the tool exists so
nobody has to do that archaeology again.

## The format

120 bytes, little-endian, no header. Multi-bit fields packed across byte boundaries
are noted explicitly; bit 0 = LSB. "?" bits are undecoded — the old editor never
reads them and writes none but 0x19's bits 6–7, which its Back number spacing control
clears; some are **set in real configs**, so they must be preserved on rewrite.

| Offset | Bits | Field | Values |
|--------|------|-------|--------|
| 0x00 | 0–1 | Short sleeves type | 1 = normal, 2 = cut-out (shirt models 144/160 only); 13 PES 2021 stock configs (referees) carry 3, preserved as a raw value; a TOML value above 3 is kept, reported by `validate` and clamped to 3 on emission like every other packed field |
| 0x00 | 2–7 | ? | 0 in samples |
| 0x01 | all | Shirt model | literal: 144 (0x90), 160 (0xA0), 176 (0xB0) |
| 0x02 | all | Long sleeves type | two values (verified): 0x3E = "Normal & U-Shirt" — one combo entry, the editor's default; 0xBB = "Only-Undershirt" (shirt models 144/160 only; the old editor forces 0x3E on any model but 144) |
| 0x03 | all | Shorts model | 0–17 |
| 0x04–0x12 | | Five RGB triplets | shirt1, shirt2, undershirt, shorts, socks |
| 0x13 | all | ? | 0 in samples |
| 0x14 | all | Collar | 1-based; the old editor lists 1–106 (v4.0 and its PES 2021 patch) or 1–109 (v5.0), PES 2021 stock configs carry up to 131 |
| 0x15 | all | Winter collar | as above |
| 0x16 | 0 | ? | set in samples |
| 0x16 | 1–4 | Shorts number Y | 0–14 |
| 0x16 | 5 | ? | set in samples |
| 0x16 | 6–7 | Shorts number X, low 2 bits | X = (0x17[0–1] << 2) \| 0x16[6–7], 0–14 |
| 0x17 | 0–1 | Shorts number X, high 2 bits | |
| 0x17 | 2 | ? | |
| 0x17 | 3–6 | Shorts number size | 0–13 in the old editor (field holds 0–15) |
| 0x17 | 7 | Shorts number side | 0 = left, 1 = right |
| 0x18 | 0–4 | Back number Y | 0–29 |
| 0x18 | 5 | ? | |
| 0x18 | 6–7 | Back number size, low 2 bits | size = (0x19[0–1] << 2) \| 0x18[6–7], 0–13 in the old editor; stock configs carry 14 |
| 0x19 | 0–1 | Back number size, high 2 bits | |
| 0x19 | 2–3 | ? | bit 2 set in samples |
| 0x19 | 4–5 | Back number spacing | 0–2 in the old editor; stock configs carry 3 |
| 0x19 | 6–7 | ? | set in samples |
| 0x1A | 0–3 | Chest number Y | 0–7 in the old editor; stock configs use the whole field, 0–15 |
| 0x1A | 4–7 | Chest number X | 0–14 |
| 0x1B | 0–3 | Chest number size | 0–15 |
| 0x1B | 4–6 | ? | |
| 0x1B | 7 | Tight shirt | on/off (shirt models 144/160 only) |
| 0x1C | 0–2 | ? | |
| 0x1C | 3–7 | Name Y, low 5 bits | Y = (0x1D[0] << 5) \| 0x1C[3–7]: PES 2021's value, 0–39. PES ≤20 reads Y = (0x1D[0] << 4) \| 0x1C[4–7], 0–16, from the same bytes: half of it, bit 3 not part of its Y |
| 0x1D | 0 | Name Y, high bit | |
| 0x1D | 1–5 | Name size | 0–20 |
| 0x1D | 6–7 | Name shape | 0 = straight, 1/2/3 = light/medium/extreme curve |
| 0x1E | 0 | Name hidden | 1 = no name on shirt |
| 0x1E | 1 | ? | |
| 0x1E | 2–5 | Badge left-short X | 0–14 |
| 0x1E | 6–7 | Badge left-short Y, low 2 bits | Y = (0x1F[0–2] << 2) \| 0x1E[6–7], 0–31 |
| 0x1F | 0–2 | Badge left-short Y, high 3 bits | |
| 0x1F | 3 | ? | |
| 0x1F | 4–7 | Badge right-short X | 0–14 |
| 0x20 | 0–4 | Badge right-short Y | 0–31 |
| 0x20 | 5 | ? | |
| 0x20 | 6–7 | Badge left-long X, low 2 bits | X = (0x21[0–1] << 2) \| 0x20[6–7], 0–14 |
| 0x21 | 0–1 | Badge left-long X, high 2 bits | |
| 0x21 | 2–6 | Badge left-long Y | 0–31 |
| 0x21 | 7 | ? | |
| 0x22 | 0–3 | Badge right-long X | 0–14 |
| 0x22 | 4–7 | Badge right-long Y, low 4 bits | Y = (0x23[0] << 4) \| 0x22[4–7], 0–31 |
| 0x23 | 0 | Badge right-long Y, high bit | |
| 0x23 | 1–7 | ? | |
| 0x24 | 0–3 | ? | 0 in samples |
| 0x24 | 4–7 | Shirt pattern | 0–13 |
| 0x25–0x27 | | ? | 0 in samples |
| 0x28–0x77 | | Five 16-byte texture names | kit, back, chest, leg, name; ASCII, NUL-padded; empty = texture not used |

Notes:

- **Ranges are the old editor's UI limits, not the format's.** Measured on the 1372 kit configs of
  PES 2021's `UniformParameter.bin` (the `kit_config` mass round-trip fixture), stock data exceeds
  the listed ranges where the table says so above and byte 0x13 and bytes 0x25–0x27 are nonzero in
  most configs. `kit_config` therefore preserves every value it reads (enum fields keep a raw
  variant for undocumented values) and its validation flags only what is known to be wrong: a
  collar of 0, a shirt model outside 144/160/176, a raw sleeve value, the cross-field constraints
  below, and values that do not fit the target version's encoding. A stock Konami config earns no
  warning or error.
- **Sleeve badges** are the four (X, Y) positions of the competition badge: one per
  sleeve (left/right) per kit variant (short/long sleeves).
- **"Shirt model" is a misnomer**, kept because the old editor and the FPC
  constants established it: in game the field only selects the model used for the
  **wrist-end of the long sleeves** when a player wears them — it does not change
  the shirt mesh, and the kit texture's UV layout is identical for all three
  values (which is what lets `color_tools` sample fixed shirt/shorts regions —
  see the [library crates plan](libs/README.md)). The old editor does gate other fields
  on it, as noted below.
- **Collar IDs** index the game's stock collar model set (`nocloth`). Exports can
  replace a stock collar with a custom model (the Team compiler's pass-through
  Collars folder) and point every kit config's collar at the replaced ID — the
  quick way to put a custom model on the whole team.
- **Cross-field constraints** (enforced by the old editor with modal errors, by ours
  inline as warnings): the old editor allowed undershirt-only long sleeves on shirt model 144
  only (its cut-out short sleeves rule, 144 only too, sits in code its window never reaches:
  the control stays disabled and byte 0x00 is never read, so no member ever set cut-out
  sleeves with it), but PES 2021 stock data carries both on model 160 as well (32
  configs), so `kit_config` warns only when they appear on model 176, which no stock config does;
  tight requires 144 or 160 (a warning likewise). The FPC kit values (shirt 176, shorts 16,
  collar and winter collar 105 — see "FPC toggle" in the
  [Aesthetics export plan](aesthetics_export/README.md)) are
  ordinary values of these fields.
- **Version differences**: PES2021 widens Name Y by one low bit, as noted — the only
  difference the 2021 patch makes. `name.y` is PES 2021's value on every version: the bytes are
  the same, an older game reading half of it (30 is its 15), so a config moves between versions
  with the name in the same place and nothing is converted. Only the range differs: an older
  game's Y stops at 16, so a `name.y` over 33 is over its maximum there. PES 15 reads only bits 5–7 of the pattern byte (a 3-bit index, 0–5
  valid) where later versions read bits 4–7; the TOML `pattern` is the 4-bit field every
  version shares, and PES 15 emission maps 4-bit values 12–13 (3-bit 6, which PES 15 has no
  pattern for) to 10–11 (3-bit 5), the byte-level rule the legacy compilers applied to
  configs authored for later versions. Values 14–15 (3-bit 7) have no legacy rule and are
  emitted as is; `validate` reports any value from 12 up as `kit_pattern_unsupported_pes15`
  (warning) when the target is PES 15.
- The five RGB triplets are the config's own kit colors (distinct from
  `UniColor.bin`'s menu colors, which come from `colors.txt`).

## TOML form (`config.toml`)

In the Studio export format, kit configs are canonically authored as **TOML files** —
editable without this tool, exactly like the player folders' `settings.toml`.
The compiler emits the game binary at compile time; no binary config ever appears
inside a Studio-format export. Old exports' binary configs are converted to
`Kits/pN/config.toml` by the [Export upgrader](export_upgrader.md). Auto-generated
configs ship with app-injected comments (predefined per-field documentation);
the editor's `toml_edit`-based writes preserve them.

```toml
# config.toml — one per kit folder (Kits/p1 … p9, g1)

[shirt]
model = 176                 # 144 / 160 / 176
collar = 105                # 1-107
winter_collar = 105         # 1-107
tight = false               # 144/160 only
pattern = 3                 # 0-13 (PES15 output supports 0-5)
long_sleeves = "normal"     # "normal" / "undershirt-only" (144/160 only)
short_sleeves = "normal"    # "normal" / "cut-out" (144/160 only)

[shorts]
model = 16                  # 0-17

[colors]                    # the config's five RGB fields
shirt1 = "#079144"
shirt2 = "#079144"
undershirt = "#056D34"
shorts = "#43AD58"
socks = "#079144"

[name]
show = true
shape = "straight"          # straight / light-curve / medium-curve / extreme-curve
y = 8                       # 0-39 (PES 15-20: 0-33)
size = 14                   # 0-20

[number.back]
y = 18                      # 0-29
size = 12                   # 0-13
spacing = 2                 # 0-2

[number.chest]
x = 7                       # 0-14
y = 2                       # 0-7
size = 10                   # 0-15

[number.shorts]
side = "left"               # left / right
x = 9                       # 0-14
y = 2                       # 0-14
size = 5                    # 0-13

[badge]                     # sleeve badge positions; x 0-14, y 0-31
right_short = { x = 7, y = 17 }
left_short  = { x = 7, y = 17 }
right_long  = { x = 7, y = 18 }
left_long   = { x = 7, y = 18 }

[unknown]                   # undecoded bits, preserved verbatim (format table above);
"0x19" = 0xC4               # keys only present when they differ from the template
```

For standalone binary conversion only, the optional source-preservation table stores
all five original 16-byte fields as fixed 32-digit hex strings, including NUL padding,
so even noncanonical padding survives exactly:

```toml
[source_texture_names]
kit = "75303730317031000000000000000000"
back = "753037303170315F6261636B00000000"
chest = "00000000000000000000000000000000"
leg = "00000000000000000000000000000000"
name = "00000000000000000000000000000000"
```

- **No authored texture-name fields in exports**: the five 16-byte names are derived
  at compile time from the textures actually present in the kit folder (`kit.dds` →
  `u0{team_id}{slot}`, `kit_back.dds` → `…_back`, and so on) — one less thing to
  desync, and the reason an export config can't reference a missing texture anymore.
  Standalone binary → TOML conversion is the exception: because it has no team/slot/
  folder context from which to rederive the original bytes, it preserves imported
  names in an optional source-only `[source_texture_names]` table. Export-folder mode
  and the Team compiler ignore that table and always rederive names from actual files.
- **Version-neutral**: the TOML stores plain values; the version-specific bit
  packing (Name Y, PES15 pattern) is applied when emitting binary. A value that
  doesn't fit the target version (Name Y 36 for PES ≤20, which holds up to 33) is a
  compile/check warning (`kit_value_out_of_range`, naming the field, the value and the
  version's maximum) and is clamped to that maximum on emission; one table of per-field,
  per-version maxima drives both the finding and the clamp, so they cannot disagree. The
  same finding covers every packed field (number sizes, positions, spacing and
  badge coordinates), whose maximum is the field's width.
- **Unknown bits** live in an `[unknown]` table keyed by byte offset, holding the
  byte's undecoded-bit remainder (known bits zeroed). Known fields plus `[unknown]`
  round-trip bit-identically; the derived texture-name region is excluded from that
  guarantee in export mode. Standalone CLI conversion is fully bit-identical when it
  carries the optional `[source_texture_names]` table described above. Keys equal to
  the template default are omitted, so hand-written configs never need `[unknown]`.

## `libs/kit_config`

The format knowledge as a lib crate:

- `KitConfig` struct ↔ 120-byte binary (per-version encode/decode) ↔ TOML, including
  optional raw source texture-name bytes for standalone roundtrips; WESYS-wrapped input is
  unwrapped on read (`tryDecompress` behavior carried over). The TOML side follows the
  suite rule that user-facing TOML is edited, never regenerated: `update_toml` writes every
  value into the existing document in place (an `[unknown]` key or a badge table keeps its
  comments and its form), refuses a document whose section is not a table instead of
  panicking, and `from_toml` refuses an `[unknown]` entry the codec could not carry (an
  offset with no undecoded bits, a remainder outside that offset's mask, two spellings of one
  offset) rather than dropping it on the next write.
- The template defaults (from the bundled `XXX_DEF_xxx_realUni.bin` template) and
  `apply_fpc` / `matches_fpc`, used by the Team compiler's reconciliation
  (`kit_config_fpc_adjusted`) and by this tool's FPC indicator. The FPC values
  themselves come from the leaf crate `libs/fpc` (see [libs](libs/README.md)), which also holds the
  player-side presets `pes_savefile` applies — one description of the system for every tool.
- `KitSlot`, the game's ten kit slots (`p1`–`p9`, `g1`): `KitSlot::parse` reads a slot name
  ASCII-case-insensitively (the kit folder grammar's slot part, `aesthetics_export`'s parse),
  `as_str` gives the lowercase name the texture names carry, and `config_name(team_id)` the
  slot's config entry name, `{team_id}_DEF_{1st…9th|GK1st}_realUni.bin`. `all` is folder
  grammar, not a slot.
- Texture-name derivation from a kit's *effective* texture set (own files plus what it inherits
  from `Kits/all/`, as resolved by `aesthetics_export`) + team id + `KitSlot`. The team id stays a
  plain `u16` whose range is the caller's: the crate does not depend on `teams_list`.
- Validation: range checks, cross-field constraints, version-fit warnings — one
  implementation for the editor's inline hints, the compiler's checks, and the CLI.

Consumers: this tool, `team_compiler` (generation, reconciliation, compile-time
emission), `export_upgrader` (binary → TOML migration). The Team compiler plan's
kit sections defer to this crate.

## Testing

- **Roundtrips**: binary → model → binary bit-identical on the three real sample
  configs, the compiler template, and **every entry of the bundled
  `UniformParameter` bins (PES 18, 19 and 21)** — thousands of real configs exercising the full
  value space; decoded fields are also asserted against the documented ranges,
  which doubles as a check of the recovered format itself.
- **TOML roundtrips** including the `[unknown]` preservation path, export-mode derivation of texture
  names, and standalone CLI bit-identical preservation through `[source_texture_names]`.
- **Version encodings**: Name Y against configs saved by both Kit Manager variants (the
  same bytes decode to the same `name.y`, a PES ≤20 config's Y being half of it); the PES15 pattern clamp against Red's behavior.

## Improvements over Kit Manager (summary)

1. **The format is documented** — recovered once, written down here, implemented in
   an open lib instead of a dead closed-source exe.
2. **TOML configs in exports** — hand-editable, diffable, self-describing; binary
   only exists as a compile output and game-file import.

## Key decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Format knowledge | Recovered by disassembling Kit Manager v4 + diffing its PES2021 patch; documented in this plan | No sources or specs exist anywhere; the plan is now the reference |
| Export storage | `config.toml` per kit folder, compiled to binary at compile time | Same philosophy as `settings.toml`: text in exports, binary only as output; editable without the tool |
| Texture-name fields | Derived from kit-folder textures in export mode; optional raw `[source_texture_names]` preservation only for standalone binary conversion | Removes export desync while retaining bit-identical context-free CLI roundtrips |
| Unknown bits | Preserved via `[unknown]` table + optional raw view | Real configs have undecoded bits set; silent loss would corrupt them; the raw view aids future decoding |
| Version handling | Global PES version selector; version-neutral TOML, encode on emission | Matches the suite; the 2021 Name Y and PES15 pattern quirks are encoding details, not user concerns |
| Format logic placement | `libs/kit_config`, shared with Team compiler and Export upgrader | One crate owns the format; all consumers agree on validity (the core plan's lib-crate rule) |
| Menu icon storage | Own optional empty `icon_<N>` marker per kit folder (0–23, absent = 3) | Fits neither `colors.txt` (it's not a color) nor `config.toml` (it's UniColor data, not kit-config data) |
