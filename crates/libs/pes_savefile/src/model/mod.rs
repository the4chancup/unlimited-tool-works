//! What consumers edit: the version-independent model the codec fills. No I/O
//! and no version knowledge lives here; a field a version lacks is `None` or
//! defaulted, and the schema decides what is read and written.

/// The illegal formation rule (`Formation::illegal_breaks`) and the
/// game's per-position pitch areas it reads.
pub mod formation_rule;
/// The ingame-face run: opaque bytes with typed accessors.
pub mod ingame_face;
/// The canonical advanced-instruction enum (the stored value is a per-version
/// index into `schema::instruction`).
pub mod instruction;
/// Save-name helpers: colour-code stripping for display.
pub mod names;
/// A player's overall rating at a position (`PlayerEntry::overall_at`).
mod overall;
/// The player model.
pub mod player;
/// The canonical playing-style enum (the stored value is a per-version index
/// into `schema::playstyle`).
pub mod playstyle;
/// The tactics model.
pub mod tactics;
/// The team model.
pub mod team;

pub use formation_rule::{FormationBreak, FormationLine};
pub use ingame_face::IngameFace;
pub use instruction::Instruction;
pub use player::{
    PlayerAppearance, PlayerBasics, PlayerEditFlags, PlayerEntry, PlayerMotion, PlayerPositions,
    PlayerSkills, PlayerStats,
};
pub use playstyle::PlayStyle;
pub use tactics::{
    AdvancedInstruction, Formation, FormationSlot, SetPieceTakers, TacticSliders, TacticStyle,
    TacticsPreset, TeamAutoFlags, TeamTactics,
};
pub use team::{KitSlot, RosterSlot, TeamColor, TeamEditFlags, TeamEntry, roster_order_player_id};
