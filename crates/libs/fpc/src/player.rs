//! The three FPC appearance presets, in this crate's own vocabulary rather
//! than savefile field names.

use std::ops::RangeInclusive;

use pes_version::PesVersion;

/// Sleeve length (`FPC.wikitext` lines 27, 36, 45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sleeves {
    /// Short sleeves.
    Short,
    /// Long sleeves.
    Long,
}

/// Whether the shirt is tucked (FPC.wikitext lines 28, 37, 46).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tuck {
    /// Untucked shirt.
    Untucked,
    /// Tucked shirt.
    Tucked,
}

/// Sock length (FPC.wikitext lines 29, 38, 47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Socks {
    /// Short socks.
    Short,
    /// Standard socks.
    Standard,
    /// Long socks.
    Long,
}

/// Whether the skin color is a preset or Custom (FPC.wikitext line 57).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkinColor {
    /// A preset skin color.
    #[default]
    Preset,
    /// Custom — hides the body but not the jersey.
    Custom,
}

/// The boots and gloves IDs the hide preset writes (FPC.wikitext lines
/// 30–31): IDs the game has no model for, so the player's own body is
/// drawn without boots, gloves or legs. They are the common settings
/// `fpc_hide_boots_id` and `fpc_hide_gloves_id`; a custom model's own
/// boots or gloves are substituted by the caller instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HideIds {
    boots: u16,
    gloves: u16,
}

impl HideIds {
    /// 4cc's convention (boots 55, gloves 11), the settings' defaults: a
    /// convention, not a game fact, since any ID the game has no model for
    /// hides.
    pub const CONVENTION: HideIds = HideIds {
        boots: 55,
        gloves: 11,
    };
    /// The band a boots hide ID is valid in: above 100 the Compiler
    /// allocates its per-team blocks, so an ID there would be another
    /// player's real model.
    pub const BOOTS: RangeInclusive<u16> = 1..=100;
    /// The band a gloves hide ID is valid in: 1 to 10 are the goalkeepers'
    /// real gloves.
    pub const GLOVES: RangeInclusive<u16> = 11..=100;

    /// The pair, or the setting's `HideIdsError` naming the ID outside
    /// its band.
    pub fn new(boots: u16, gloves: u16) -> Result<HideIds, HideIdsError> {
        if !Self::BOOTS.contains(&boots) {
            return Err(HideIdsError::Boots(boots));
        }
        if !Self::GLOVES.contains(&gloves) {
            return Err(HideIdsError::Gloves(gloves));
        }
        Ok(HideIds { boots, gloves })
    }

    /// The boots ID.
    pub fn boots(self) -> u16 {
        self.boots
    }

    /// The gloves ID.
    pub fn gloves(self) -> u16 {
        self.gloves
    }
}

/// A `fpc_hide_boots_id` or `fpc_hide_gloves_id` setting outside its band
/// (`HideIds::BOOTS`/`GLOVES`): the message names the setting and its range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HideIdsError {
    /// `fpc_hide_boots_id` outside 1 to 100.
    #[error(
        "fpc_hide_boots_id = {0}: must be 1 to 100 \
         (above 100 the Compiler allocates its own IDs)"
    )]
    Boots(u16),
    /// `fpc_hide_gloves_id` outside 11 to 100.
    #[error("fpc_hide_gloves_id = {0}: must be 11 to 100 (1 to 10 are goalkeeper gloves)")]
    Gloves(u16),
}

/// Whether `version` has a Custom skin color: true on PES 15 to 17 only; the
/// later games dropped it, so the partial-hide preset's one version-dependent
/// fact lives here (FPC.wikitext line 57).
pub fn custom_skin_available(version: PesVersion) -> bool {
    matches!(
        version,
        PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17
    )
}

/// One of the three appearance presets. The hide preset's boots/gloves ids
/// are the `HideIds` `preset` was given, the others' 0; a custom model's own
/// boots/gloves are the caller's substitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    /// Sleeve length.
    pub sleeves: Sleeves,
    /// Shirt tuck.
    pub tuck: Tuck,
    /// Sock length.
    pub socks: Socks,
    /// Boots ID.
    pub boots_id: u16,
    /// Gloves ID.
    pub gloves_id: u16,
    /// Skin color preset/custom.
    pub skin_color: SkinColor,
}

/// The preset kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Completely hide the player's default body (FPC.wikitext lines 24–31).
    Hide,
    /// Completely show the default body (lines 33–40).
    Unhide,
    /// Show the stock jersey but hide the skin (lines 42–48).
    PartialHide,
}

/// The appearance one preset prescribes. `ids` gives the hide preset's
/// boots and gloves IDs (the common settings' values); the other presets
/// ignore it.
pub fn preset(kind: Preset, ids: HideIds) -> Appearance {
    match kind {
        Preset::Hide => Appearance {
            sleeves: Sleeves::Long,
            tuck: Tuck::Tucked,
            socks: Socks::Short,
            boots_id: ids.boots,
            gloves_id: ids.gloves,
            skin_color: SkinColor::Preset,
        },
        Preset::Unhide => Appearance {
            sleeves: Sleeves::Short,
            tuck: Tuck::Untucked,
            socks: Socks::Standard,
            boots_id: 0,
            gloves_id: 0,
            skin_color: SkinColor::Preset,
        },
        Preset::PartialHide => Appearance {
            sleeves: Sleeves::Short,
            tuck: Tuck::Untucked,
            socks: Socks::Standard,
            boots_id: 0,
            gloves_id: 0,
            skin_color: SkinColor::Custom,
        },
    }
}
