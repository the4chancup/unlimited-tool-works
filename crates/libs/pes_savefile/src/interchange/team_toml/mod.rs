//! `team.toml`: the full-fidelity team interchange format (plan:
//! `spec/pes_savefile/operations.md`, "Interchange formats" → "Team TOML"). A
//! document holds one team's `[team]`, `[tactics]` and `[players.NN]` data
//! with every field optional: an absent key is "leave untouched" on import,
//! which is what makes one-key patches and cross-version imports safe.
//!
mod items;
/// The label tables `team.toml` writes where the record stores a number or
/// a bit (style-switch pairs, positions, instructions, ratings, styles,
/// skills, COM styles, foot/hand): the spellings other crates share.
pub mod labels;
mod player;
pub(crate) mod player_keys;
mod tactics;
mod team;

pub use team::shirt_name_from;

use std::collections::BTreeMap;

use pes_version::PesVersion;

use crate::codec::CodecError;
use crate::model::instruction::Instruction;
use crate::model::playstyle::PlayStyle;
use crate::model::tactics::FormationSlot;
use crate::model::team::{KitSlot, TeamColor};
use crate::settings_toml::AppearanceSettings;
use crate::settings_toml::SettingsError;

/// A parsed `team.toml`: the version it was written for (absent = the import
/// target's) plus three optional-bearing sections. `players` is keyed by
/// roster slot, not player id.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamToml {
    /// The save version the document was written from; `None` = the target's.
    pub pes_version: Option<PesVersion>,
    /// The `[team]` table.
    pub team: TeamSection,
    /// The `[tactics]` table.
    pub tactics: TacticsSection,
    /// The `[players.NN]` tables by roster slot.
    pub players: BTreeMap<u8, PlayerSection>,
    /// The roster slots whose player is not the roster-order rule's, as
    /// `TeamEntry::roster_order_breaks` reports them (`(slot, player_id)`,
    /// 0-based). Set by `from_team` alone: a parsed or hand-built document
    /// holds no ids to check. Emitted as a `[players.NN]` header note;
    /// `apply` never reads it (it is no key), so `parse` of the text a
    /// breaking team's `to_toml` wrote does not reproduce it.
    pub roster_breaks: Vec<(u8, u32)>,
}

/// `[team]`: identity, links and the version-gated colour/kit fields. `id` is
/// informational only and never applied.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamSection {
    /// The team id (informational; apply never writes it).
    pub id: Option<u32>,
    /// The team name.
    pub name: Option<String>,
    /// The three-letter short name.
    pub short_name: Option<String>,
    /// Manager id (PES 19+).
    pub manager_id: Option<u32>,
    /// Home stadium id (PES 19+).
    pub stadium_id: Option<u16>,
    /// First team colour (PES 17+).
    pub color_1: Option<TeamColor>,
    /// Second team colour (PES 17+).
    pub color_2: Option<TeamColor>,
    /// The ten kit slots (PES 17).
    pub kit_slots: Option<[KitSlot; 10]>,
    /// `[team.edit_flags]`.
    pub edit_flags: TeamEditFlagsSection,
}

/// `[team.edit_flags]`: the game's own "was edited" flags.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TeamEditFlagsSection {
    /// The team name was edited.
    pub name: Option<bool>,
    /// The short name was edited (PES 15 only).
    pub short_name: Option<bool>,
    /// The stadium was edited (PES 20+).
    pub stadium: Option<bool>,
    /// The strips were edited (PES 17 only).
    pub strip: Option<bool>,
}

/// `[tactics]`: the squad order, set pieces, auto flags and three presets.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TacticsSection {
    /// The eleven starters as roster slots.
    pub starting_eleven: Option<[u8; 11]>,
    /// The bench order as roster slots.
    pub bench_order: Option<[u8; 21]>,
    /// `[tactics.set_pieces]`.
    pub set_pieces: SetPiecesSection,
    /// Roster slots that join the attack.
    pub players_to_join_attack: Option<[u8; 3]>,
    /// `[tactics.auto]`.
    pub auto: AutoSection,
    /// `[tactics.preset_1]` to `[tactics.preset_3]`.
    pub presets: [PresetSection; 3],
}

/// `[tactics.set_pieces]`: takers as roster slots (255 = unset).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SetPiecesSection {
    /// Long free kick taker.
    pub free_kick_long: Option<u8>,
    /// Short free kick taker.
    pub free_kick_short: Option<u8>,
    /// Second free kick taker.
    pub free_kick_second: Option<u8>,
    /// Left corner taker.
    pub corner_left: Option<u8>,
    /// Right corner taker.
    pub corner_right: Option<u8>,
    /// Penalty taker.
    pub penalty: Option<u8>,
    /// Captain.
    pub captain: Option<u8>,
}

/// `[tactics.auto]`: the auto-tactics flags.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AutoSection {
    /// Auto substitution (the stored byte; 255 = auto, PES 15+).
    pub substitution: Option<u8>,
    /// Auto offside trap (PES 15+).
    pub offside_trap: Option<bool>,
    /// Auto preset change (PES 15+).
    pub preset_change: Option<bool>,
    /// Attack/defence levels (PES 17+).
    pub attack_defence_levels: Option<bool>,
}

/// `[tactics.preset_N]`: one preset's man-marking assignments, style,
/// sliders, formations and instructions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PresetSection {
    /// Man marking: own formation slot `i`'s assignment, the marked
    /// opponent's formation slot (255 = none).
    pub man_marking: Option<[u8; 11]>,
    /// `[tactics.preset_N.style]`.
    pub style: StyleSection,
    /// `[tactics.preset_N.sliders]`.
    pub sliders: SlidersSection,
    /// `[tactics.preset_N.formation_1]` to `formation_3` (kick-off, in
    /// possession, out of possession).
    pub formations: [Option<[FormationSlot; 11]>; 3],
    /// `[tactics.preset_N.instructions]` (PES 17+).
    pub instructions: Option<InstructionsSection>,
}

/// `[tactics.preset_N.style]`: the seven style switches plus fluid formation.
/// Each switch stores one bit; the label pair is `false` | `true`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StyleSection {
    /// "counter_attack" | "possession".
    pub attacking_style: Option<bool>,
    /// "centre" | "wide".
    pub attacking_zone: Option<bool>,
    /// "long_pass" | "short_pass".
    pub buildup: Option<bool>,
    /// "maintain" | "flexible".
    pub positioning: Option<bool>,
    /// "frontline_pressure" | "all_out_defence".
    pub defensive_style: Option<bool>,
    /// "middle" | "wide".
    pub containment_area: Option<bool>,
    /// "aggressive" | "conservative".
    pub pressure: Option<bool>,
    /// Fluid formation on/off (PES 16+).
    pub fluid: Option<bool>,
}

/// `[tactics.preset_N.sliders]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SlidersSection {
    /// Support range, 1 to 10.
    pub support_range: Option<u8>,
    /// Defensive line, 1 to 10.
    pub defensive_line: Option<u8>,
    /// Compactness, 1 to 10.
    pub compactness: Option<u8>,
    /// Numbers in attack, 1 few to 3 many.
    pub numbers_in_attack: Option<u8>,
    /// Numbers in defence, 1 few to 3 many.
    pub numbers_in_defence: Option<u8>,
}

/// `[tactics.preset_N.instructions]`: two attack and two defence instructions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InstructionsSection {
    /// `[tactics.preset_N.instructions]` `attack = [...]`.
    pub attack: [InstructionEntry; 2],
    /// `defence = [...]`.
    pub defence: [InstructionEntry; 2],
}

/// One `{ instruction, player }` pair of an instruction array.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InstructionEntry {
    /// The canonical instruction (`schema::instruction` translates).
    pub instruction: Instruction,
    /// The roster slot it applies to.
    pub player: u8,
}

/// `[players.NN]`: one roster slot's player.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerSection {
    /// Display name, colour codes included.
    pub name: Option<String>,
    /// Shirt name; absent = derived from `name` by [`TeamToml::apply`]'s
    /// caller.
    pub shirt_name: Option<String>,
    /// Squad number (from the roster record).
    pub number: Option<u16>,
    /// Nationality id.
    pub nationality: Option<u16>,
    /// Age.
    pub age: Option<u8>,
    /// Height in cm.
    pub height: Option<u8>,
    /// Weight in kg.
    pub weight: Option<u8>,
    /// Boots model id.
    pub boots_id: Option<u32>,
    /// Gloves model id.
    pub gloves_id: Option<u32>,
    /// Base-copy player id.
    pub base_copy_id: Option<u32>,
    /// `[players.NN.stats]`.
    pub stats: StatsSection,
    /// `[players.NN.positions]`.
    pub positions: PositionsSection,
    /// `[players.NN.skills]`.
    pub skills: SkillsSection,
    /// `[players.NN.edit_flags]`.
    pub edit_flags: EditFlagsSection,
    /// The appearance keys, in `settings.toml`'s nested shape.
    pub appearance: AppearanceSettings,
}

/// `[players.NN.stats]`: the ability numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StatsSection {
    /// Attacking Prowess (Offensive Awareness).
    pub attacking_prowess: Option<u8>,
    /// Ball Control.
    pub ball_control: Option<u8>,
    /// Dribbling.
    pub dribbling: Option<u8>,
    /// Tight Possession (PES 20+).
    pub tight_possession: Option<u8>,
    /// Low Pass.
    pub low_pass: Option<u8>,
    /// Lofted Pass.
    pub lofted_pass: Option<u8>,
    /// Finishing.
    pub finishing: Option<u8>,
    /// Header (Heading).
    pub heading: Option<u8>,
    /// Place Kicking.
    pub place_kicking: Option<u8>,
    /// Swerve (Curl).
    pub swerve: Option<u8>,
    /// Defensive Prowess (Defensive Awareness).
    pub defensive_prowess: Option<u8>,
    /// Ball Winning (Tackling).
    pub ball_winning: Option<u8>,
    /// Aggression (PES 20+).
    pub aggression: Option<u8>,
    /// Kicking Power.
    pub kicking_power: Option<u8>,
    /// Speed.
    pub speed: Option<u8>,
    /// Explosive Power (Acceleration).
    pub explosive_power: Option<u8>,
    /// Body Control.
    pub body_control: Option<u8>,
    /// Physical Contact (PES 17+).
    pub physical_contact: Option<u8>,
    /// Jump.
    pub jump: Option<u8>,
    /// Stamina.
    pub stamina: Option<u8>,
    /// Goalkeeping (GK Awareness).
    pub goalkeeping: Option<u8>,
    /// Catching (PES 15; folded into the GK set later).
    pub catching: Option<u8>,
    /// Clearing (PES 16+).
    pub clearing: Option<u8>,
    /// Reflexes (PES 16+).
    pub reflexes: Option<u8>,
    /// Coverage (PES 16+).
    pub coverage: Option<u8>,
    /// Weak Foot Usage.
    pub weak_foot_usage: Option<u8>,
    /// Weak Foot Accuracy.
    pub weak_foot_accuracy: Option<u8>,
    /// Form (Condition), 1 to 8.
    pub form: Option<u8>,
    /// Injury Resistance.
    pub injury_resistance: Option<u8>,
    /// Star rating (PES 19+).
    pub star: Option<u8>,
    /// Playing attitude (PES 20+).
    pub playing_attitude: Option<u8>,
}

/// `[players.NN.positions]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionsSection {
    /// Registered position (GK 0 to CF 12).
    pub registered: Option<u8>,
    /// Playable position ratings, CF first.
    pub playable: Option<[u8; 13]>,
    /// Playing style (a canonical `PlayStyle` label in the document).
    pub playing_style: Option<PlayStyle>,
    /// Stronger foot (0 right, 1 left).
    pub stronger_foot: Option<u8>,
    /// Stronger hand (0 right, 1 left; PES 20+).
    pub stronger_hand: Option<u8>,
}

/// `[players.NN.skills]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkillsSection {
    /// Player skills by canonical slot (0 Scissors Feint to 40 Through Passing).
    pub skills: Option<[bool; 41]>,
    /// COM playing styles (0 Trickster to 6 Long Ranger).
    pub com_styles: Option<[bool; 7]>,
}

/// `[players.NN.edit_flags]`: the game's own per-section flags.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EditFlagsSection {
    /// The player was created or edited.
    pub player: Option<bool>,
    /// The basic settings were edited.
    pub basic_settings: Option<bool>,
    /// The registered position was edited.
    pub registered_position: Option<bool>,
    /// The playable positions were edited.
    pub playable_positions: Option<bool>,
    /// The abilities were edited.
    pub abilities: Option<bool>,
    /// The skills were edited.
    pub skills: Option<bool>,
    /// The playing style was edited.
    pub playing_style: Option<bool>,
    /// The COM playing styles were edited.
    pub com_styles: Option<bool>,
    /// The motions were edited.
    pub motion: Option<bool>,
    /// The player is a base copy.
    pub base_copy: Option<bool>,
    /// The face was edited.
    pub face: Option<bool>,
    /// The hairstyle was edited.
    pub hair: Option<bool>,
    /// The physique was edited.
    pub physique: Option<bool>,
    /// The strip style was edited.
    pub strip: Option<bool>,
}

/// A field the import skipped, and why.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportNote {
    /// `path` exists in the document's version but not the target's.
    NotInThisVersion {
        /// The dotted key path ("team.manager_id").
        path: String,
    },
    /// `path`'s value has no stored form in the target version (an instruction
    /// the target lacks): `Off` was written in its place.
    NotEncodable {
        /// The dotted key path.
        path: String,
        /// The value's label.
        label: String,
    },
    /// A face type above the target's cap, skin 7 into a no-custom-skin
    /// version, or a motion above the target's top; capped to `to`.
    Capped {
        /// The dotted key path.
        path: String,
        /// The value as written.
        from: u8,
        /// The value written in its place.
        to: u8,
    },
    /// A `[players.NN]` whose target roster slot is empty; skipped.
    EmptySlot {
        /// The roster slot (1-based, as the key spells it).
        slot: u8,
    },
    /// `path`'s text was longer than the target field and was cut to it;
    /// `cropped` is the text as stored, colour codes as in the save.
    TextCropped {
        /// The dotted key path.
        path: String,
        /// The text as stored.
        cropped: String,
    },
}

/// What parsing, emitting or applying a `team.toml` can fail with.
#[derive(Debug, thiserror::Error)]
pub enum TeamTomlError {
    /// The document is not valid TOML.
    #[error("invalid TOML: {0}")]
    Toml(String),
    /// A key the format does not know.
    #[error("unknown key {key:?}")]
    UnknownKey {
        /// The dotted key path.
        key: String,
    },
    /// A key a present table requires is absent.
    #[error("{key}: required in this table")]
    MissingKey {
        /// The dotted key path.
        key: String,
    },
    /// A key holds the wrong kind of TOML value.
    #[error("{key}: expected {expected}")]
    WrongType {
        /// The dotted key path.
        key: String,
        /// What the key takes.
        expected: &'static str,
    },
    /// An integer outside its range.
    #[error("{key}: {value} is out of range ({range})")]
    OutOfRange {
        /// The dotted key path.
        key: String,
        /// The offending value.
        value: i64,
        /// The allowed range as text ("1 to 10").
        range: String,
    },
    /// A string that is not one of the key's labels.
    #[error("{key}: unknown label {label:?} (allowed: {allowed})")]
    UnknownLabel {
        /// The dotted key path.
        key: String,
        /// The offending label.
        label: String,
        /// The allowed labels, quoted.
        allowed: String,
    },
    /// The document's own version stores the field but the target's does not.
    #[error("{path}: not in this version")]
    NotInThisVersion {
        /// The dotted key path.
        path: String,
    },
    /// A codec failure (a stored instruction value the version does not list).
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// An appearance-table failure.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The roster names a player id `players` does not hold.
    #[error("player {id} is not in the save")]
    PlayerMissing {
        /// The missing player id.
        id: u32,
    },
    /// A `[players.NN]` past the target team's roster width.
    #[error("roster has no slot {slot}")]
    NoSuchSlot {
        /// The roster slot (1-based, as the key spells it).
        slot: u8,
    },
    /// A text longer than the target version's field.
    #[error("{path}: does not fit the field (max {max})")]
    TextTooLong {
        /// The dotted key path.
        path: String,
        /// The field's capacity: its byte length.
        max: usize,
    },
}
