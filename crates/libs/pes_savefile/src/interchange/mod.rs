//! The interchange formats: files other tools and the game itself exchange
//! teams in. Each module reads its format into `model/` entries; the crypto
//! and layout tables live in `container`/`schema`.

/// `aesthetics_patch.toml`: the Compiler's resolved savefile writes.
pub mod aesthetics_patch;
/// The `.4ccs` and `.4cct` interchange formats, read-only.
pub mod legacy;
/// `team.toml`: the full-fidelity team interchange format.
pub mod team_toml;
/// PES's own in-game team export (`.ted` on 18-21, `TEXPORT00000000` on 15-17).
pub mod texport;

#[cfg(test)]
mod legacy_golden;
#[cfg(test)]
mod texport_golden;

/// Whether `name` is a `players.NN` key: two ASCII digits inside `range`
/// (`01` to `40` a `team.toml` roster slot, `01` to `99` the patch's
/// `team id × 100 + NN`). Both formats read the same grammar.
pub(crate) fn players_key(name: &str, range: std::ops::RangeInclusive<u8>) -> bool {
    name.len() == 2
        && name.bytes().all(|b| b.is_ascii_digit())
        && name.parse::<u8>().is_ok_and(|slot| range.contains(&slot))
}
