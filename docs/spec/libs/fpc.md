# Unlimited Tool Works — Library crates plan: fpc

Part of the [Library crates plan](README.md). Section headings are unchanged from the single-file plan, so an existing pointer to a section still names it; only the file part of the pointer changed.

## `libs/fpc`

Full Player Customization is one *system* that spans two formats: kit configs (every kit of the
team, GK included, must carry shirt model 176, shorts model 16, collar 105, winter collar 105) and
the player's appearance fields (the savefile's on pre-Fox; on Fox the `PlayerAppearance.bin`
row's, the IDs in the `BootsList`/`GloveList` rows). The presets: the hide preset: long sleeves, tucked shirt, short socks, a
nonexistent boots ID such as 55, a nonexistent gloves ID such as 11; the un-hide preset: short
sleeves, untucked, standard socks, boots 0, gloves 0, where gloves 0 is a set of normal
hands, since the cup's gloves system lets any player wear a gloves model; the partial-hide
preset for a custom body inside a stock jersey: short sleeves, untucked, standard socks, skin color
Custom. Neither `kit_config` nor `pes_savefile` is the natural owner, and having each hold half
would either duplicate the knowledge or make one format crate depend on the other. So the system's
knowledge is a **leaf crate whose only dependencies are `pes_version` and `thiserror`** (for the
hide IDs' error type), holding data and pure rules only:

- `kit.rs` — the kit-config FPC values per PES version (the modern system: PES 19+ and the 2024
  reimplementation for 16/17; the retro 16/17 system is documented as legacy and not supported).
  `kit_values()` returns the four values (shirt model 176, shorts model 16, collar 105, winter
  collar 105), each a `u8`, the width a kit config stores it in. They are the same on every
  supported version, so the function takes no version: a parameter nothing reads is how PES 15
  and 18 once got a wrong `None`. The
  wiki page it was written from documents the PES 17 values and calls the PES 19+ system "mostly
  identical", and says nothing of PES 15 or 18, but the cup's installs have FPC on all of them.
  Counted on the maintainer's installs (every `*_realUni.bin` in each install's `download`
  CPKs, decoded with `kit_config`): exactly these four values in 301 of 360 kit configs on PES
  15, 285 of 338 on PES 16, 344 of 407 on PES 17, 35 of 49 on PES 19 and 300 of 314 on PES 21 (the last VGL's DLC; 474 of 637 on an
  older cup's), which also settles that PES 19+ uses the same four. Each install, PES 15 and 18 included, has
  its own `4cc_04_fpc.cpk` supplying the empty `collar_105` and `pants_016` models; the stock
  PES 15 game has no collar 105 (its collars end at 101), so there the collar exists only
  through that CPK. The PES 18 install holds no team kit config to count; its FPC CPK holds
  the same files as PES 19's, and the maintainer confirmed it uses the same four values.
  The PES 20 install (a base DLC only) holds no team kit config either; its FPC CPK
  supplies the same empty `collar_105` and `pants_016`.
- `player.rs` — the three appearance presets above, expressed in the crate's own small vocabulary
  (`Sleeves`, `Tuck`, `Socks`, boots/gloves IDs, skin color), not in `pes_savefile` field terms;
  and `custom_skin_available(version) -> bool`, true for PES 15 to 17 only (the Fox games dropped
  the Custom skin; the reference editor resets it to the default there), so the partial-hide
  preset's one version-dependent fact lives with the rest of the FPC knowledge.
- `interference.rs` — the settings that break or bend FPC, as findings with severity: inners ≠ None
  and undershorts ≠ Off/Off break hiding (error on an FPC player); wrist/ankle taping ≠ None
  selectively shows pieces (info — deliberate in custom setups); the Gloves checkbox with gloves ID
  0 causes winter gloves in winter conditions (warning); skin color Custom on a non-FPC player hides
  the body but not the jersey (warning).

Consumers: `kit_config` (`apply_fpc(&mut config)` / `matches_fpc(&config)` use `fpc::kit`; there is no revert, since the
template config already carries the FPC values and the compiler never auto-reverts them), `pes_savefile`
(`ops/fpc.rs` maps `fpc::player` presets onto `PlayerEntry` and runs the interference check), and
through them the Team compiler (`fpc_on`/`fpc_off` markers, kit reconciliation, `settings.toml`
validation); the editors' uses are in `plans/libs/fpc.md`. The wiki's FPC guide
(`resources/FPC.wikitext` in this repo) is the source text; each constant carries a doc comment pointing at
its line there.

---
