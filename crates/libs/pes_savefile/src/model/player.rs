//! `PlayerEntry`: the version-independent player model. Fields a version lacks
//! are `Option` (the record has no run for them) or defaulted to 0/false; the
//! schema decides what is read and written, the model just holds the result.

use pes_version::PesVersion;

use crate::codec::{self, CodecError};
use crate::model::ingame_face::IngameFace;
use crate::schema::fields::PlayerField;
use crate::schema::schema_for;

/// The stripped value of `PlayerEntry::appearance_id`: the game's -1 (all
/// bits set). A stripped player takes his appearance from the database
/// tables, not from the appearance block.
pub const STRIPPED_APPEARANCE_ID: u32 = u32::MAX;

/// One player of the save: identity, abilities, positions, skills, motion, the
/// game's own edit flags and the appearance block (a separate record on PES
/// 15/16, part of the player record later).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerEntry {
    /// The player id (team id x 100 + slot for cup players).
    pub id: u32,
    /// The id of the player whose appearance the block holds: the player's own,
    /// or `STRIPPED_APPEARANCE_ID` (the game's -1) on a stripped save, where
    /// the database tables decide the look. `None` on PES 15/16, where the
    /// appearance record's first u32 is the join key itself (`id`).
    pub appearance_id: Option<u32>,
    /// Display name, colour codes included.
    pub name: String,
    /// Shirt name (single-byte text on disk).
    pub shirt_name: String,
    /// Nationality and physique basics.
    pub basic: PlayerBasics,
    /// The ability numbers plus form, injury and the version-gated ratings.
    pub stats: PlayerStats,
    /// Registered/playable positions and the playing style.
    pub positions: PlayerPositions,
    /// Skill and COM-playing-style flags.
    pub skills: PlayerSkills,
    /// Motion selections (hunching, arms, kick motions, celebrations).
    pub motion: PlayerMotion,
    /// The game's own "was edited" flags, one per editor section.
    pub edit_flags: PlayerEditFlags,
    /// Boots/gloves/base-copy ids, physique measurements and kit styling.
    pub appearance: PlayerAppearance,
}

/// Nationality and physique basics.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerBasics {
    /// Nationality id.
    pub nationality: u16,
    /// Age.
    pub age: u8,
    /// Height in cm.
    pub height: u8,
    /// Weight in kg.
    pub weight: u8,
}

/// The ability numbers. The `Option` fields exist only from the version on
/// that introduced them (`None` = this save's version has no such stat).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerStats {
    /// Attacking Prowess (Offensive Awareness).
    pub attacking_prowess: u8,
    /// Ball Control.
    pub ball_control: u8,
    /// Dribbling.
    pub dribbling: u8,
    /// Low Pass.
    pub low_pass: u8,
    /// Lofted Pass.
    pub lofted_pass: u8,
    /// Finishing.
    pub finishing: u8,
    /// Place Kicking.
    pub place_kicking: u8,
    /// Swerve (Curl).
    pub swerve: u8,
    /// Header (Heading).
    pub heading: u8,
    /// Defensive Prowess (Defensive Awareness).
    pub defensive_prowess: u8,
    /// Ball Winning (Tackling).
    pub ball_winning: u8,
    /// Kicking Power.
    pub kicking_power: u8,
    /// Speed.
    pub speed: u8,
    /// Explosive Power (Acceleration).
    pub explosive_power: u8,
    /// Body Control (Body Balance on PES 16).
    pub body_control: u8,
    /// Physical Contact (PES 17+).
    pub physical_contact: Option<u8>,
    /// Jump.
    pub jump: u8,
    /// Goalkeeping (GK Awareness).
    pub goalkeeping: u8,
    /// Catching (Saving on PES 15; folded into the GK set from PES 16).
    pub catching: Option<u8>,
    /// Clearing (PES 16+).
    pub clearing: Option<u8>,
    /// Reflexes (PES 16+).
    pub reflexes: Option<u8>,
    /// Coverage (PES 16+).
    pub coverage: Option<u8>,
    /// Stamina.
    pub stamina: u8,
    /// Weak Foot Usage.
    pub weak_foot_usage: u8,
    /// Weak Foot Accuracy.
    pub weak_foot_accuracy: u8,
    /// Form (Condition), 1 to 8 stored 0 to 7.
    pub form: u8,
    /// Injury Resistance.
    pub injury_resistance: u8,
    /// Star rating (PES 19+).
    pub star: Option<u8>,
    /// Tight Possession (PES 20+).
    pub tight_possession: Option<u8>,
    /// Aggression (PES 20+).
    pub aggression: Option<u8>,
    /// Playing attitude (PES 20+).
    pub playing_attitude: Option<u8>,
}

impl PlayerStats {
    /// The ability stats in `interchange::team_toml::labels::ABILITY_STATS`'
    /// order (this struct's field order), `None` for a stat this save's
    /// version lacks (the `Option` fields).
    pub fn abilities(&self) -> [Option<u8>; 25] {
        [
            Some(self.attacking_prowess),
            Some(self.ball_control),
            Some(self.dribbling),
            Some(self.low_pass),
            Some(self.lofted_pass),
            Some(self.finishing),
            Some(self.place_kicking),
            Some(self.swerve),
            Some(self.heading),
            Some(self.defensive_prowess),
            Some(self.ball_winning),
            Some(self.kicking_power),
            Some(self.speed),
            Some(self.explosive_power),
            Some(self.body_control),
            self.physical_contact,
            Some(self.jump),
            Some(self.goalkeeping),
            self.catching,
            self.clearing,
            self.reflexes,
            self.coverage,
            Some(self.stamina),
            self.tight_possession,
            self.aggression,
        ]
    }

    /// Sets ability `index` to `value`, in `abilities`' order
    /// (`interchange::team_toml::labels::ABILITY_STATS`'); an `Option` field
    /// the version lacks stays `None`. An index past 24 panics, a
    /// programming error, as indexing `abilities()` would.
    pub fn set_ability(&mut self, index: usize, value: u8) {
        let gated = |field: &mut Option<u8>| {
            if field.is_some() {
                *field = Some(value);
            }
        };
        match index {
            0 => self.attacking_prowess = value,
            1 => self.ball_control = value,
            2 => self.dribbling = value,
            3 => self.low_pass = value,
            4 => self.lofted_pass = value,
            5 => self.finishing = value,
            6 => self.place_kicking = value,
            7 => self.swerve = value,
            8 => self.heading = value,
            9 => self.defensive_prowess = value,
            10 => self.ball_winning = value,
            11 => self.kicking_power = value,
            12 => self.speed = value,
            13 => self.explosive_power = value,
            14 => self.body_control = value,
            15 => gated(&mut self.physical_contact),
            16 => self.jump = value,
            17 => self.goalkeeping = value,
            18 => gated(&mut self.catching),
            19 => gated(&mut self.clearing),
            20 => gated(&mut self.reflexes),
            21 => gated(&mut self.coverage),
            22 => self.stamina = value,
            23 => gated(&mut self.tight_possession),
            24 => gated(&mut self.aggression),
            other => panic!("ability stat index {other} is past the 25 ability stats"),
        }
    }
}

/// Registered/playable positions and the playing style.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerPositions {
    /// Registered position, GK 0 to CF 12.
    pub registered: u8,
    /// Playable position rating (0 C, 1 B, 2 A; 3 held by no save), in the
    /// order CF, SS, LWF, RWF, AMF, DMF, CMF, LMF, RMF, CB, LB, RB, GK.
    pub playable: [u8; 13],
    /// Playing style, a version-specific index.
    pub playing_style: u8,
    /// Stronger foot (0 right, 1 left).
    pub stronger_foot: u8,
    /// Stronger hand (PES 20+).
    pub stronger_hand: Option<u8>,
}

/// Skill and COM-playing-style flags.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSkills {
    /// Player skills, 0 Scissors Feint to 40 Through Passing. Older versions
    /// carry fewer skills; the schema maps them into these canonical slots.
    pub skills: [bool; 41],
    /// COM playing styles, 0 Trickster to 6 Long Ranger.
    pub com_styles: [bool; 7],
}

// [bool; 41] is past the size std derives `Default` for.
impl Default for PlayerSkills {
    fn default() -> Self {
        Self {
            skills: [false; 41],
            com_styles: [false; 7],
        }
    }
}

/// Motion selections.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerMotion {
    /// Hunching while dribbling.
    pub hunching_dribbling: u8,
    /// Hunching while running.
    pub hunching_running: u8,
    /// Arm movement while dribbling.
    pub arm_movement_dribbling: u8,
    /// Arm movement while running.
    pub arm_movement_running: u8,
    /// Corner kick motion.
    pub corner_kick: u8,
    /// Free kick motion.
    pub free_kick: u8,
    /// Penalty kick motion.
    pub penalty_kick: u8,
    /// Dribbling motion (PES 20+).
    pub dribbling: Option<u8>,
    /// First goal celebration.
    pub goal_celebration_1: u8,
    /// Second goal celebration.
    pub goal_celebration_2: u8,
}

/// The game's own per-section "was edited" flags.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerEditFlags {
    /// The player was created or edited.
    pub player: bool,
    /// The basic settings were edited.
    pub basic_settings: bool,
    /// The registered position was edited.
    pub registered_position: bool,
    /// The playable positions were edited.
    pub playable_positions: bool,
    /// The abilities were edited.
    pub abilities: bool,
    /// The skills were edited.
    pub skills: bool,
    /// The playing style was edited.
    pub playing_style: bool,
    /// The COM playing styles were edited.
    pub com_styles: bool,
    /// The motions were edited.
    pub motion: bool,
    /// The player is a base copy of another (PES 15-18 only).
    pub base_copy: Option<bool>,
    /// The face was edited.
    pub face: bool,
    /// The hairstyle was edited.
    pub hair: bool,
    /// The physique was edited.
    pub physique: bool,
    /// The strip style was edited.
    pub strip: bool,
}

/// Boots/gloves/base-copy ids, physique measurements and kit styling.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerAppearance {
    /// Boots model id.
    pub boots_id: u32,
    /// Gloves model id (any player; 0 = normal hands).
    pub gloves_id: u32,
    /// Base copy player id (the player's own id when unset).
    pub base_copy_id: u32,
    /// Neck length.
    pub neck_length: u8,
    /// Neck size.
    pub neck_size: u8,
    /// Shoulder height.
    pub shoulder_height: u8,
    /// Shoulder width.
    pub shoulder_width: u8,
    /// Chest measurement.
    pub chest: u8,
    /// Waist size.
    pub waist: u8,
    /// Arm size.
    pub arm_size: u8,
    /// Arm length.
    pub arm_length: u8,
    /// Thigh size.
    pub thigh: u8,
    /// Calf size.
    pub calf: u8,
    /// Leg length.
    pub leg_length: u8,
    /// Head length.
    pub head_length: u8,
    /// Head width.
    pub head_width: u8,
    /// Head depth.
    pub head_depth: u8,
    /// Left wrist tape colour.
    pub wrist_tape_color_left: u8,
    /// Right wrist tape colour.
    pub wrist_tape_color_right: u8,
    /// Wrist taping.
    pub wrist_taping: u8,
    /// Spectacles frame colour.
    pub spectacles_color: u8,
    /// Spectacles style.
    pub spectacles_style: u8,
    /// Sleeves.
    pub sleeves: u8,
    /// Long-sleeved inners.
    pub inners: u8,
    /// Sock length.
    pub socks: u8,
    /// Undershorts.
    pub undershorts: u8,
    /// Shirttail out.
    pub untucked: bool,
    /// Ankle taping.
    pub ankle_taping: bool,
    /// The ingame-face run: the appearance block's byte 22 to its end, carried
    /// verbatim (50 bytes, 46 on PES 15) with `IngameFace` accessors for
    /// every field of the face and the hair.
    pub ingame_face: IngameFace,
}

impl PlayerEntry {
    /// A record of `version` with every field the version stores at zero
    /// and the fields it lacks absent (the gated `Option` fields `None`):
    /// what a document is applied onto when no save record exists, so a
    /// key the document omits reads 0 like any other field, never as a
    /// field the version lacks. Decoded from a zeroed record of the
    /// version's layout (the PES 15/16 appearance record included), so
    /// the schema alone decides what is present.
    pub fn bare(version: PesVersion) -> PlayerEntry {
        let schema = schema_for(version);
        let record = vec![0; schema.player.size];
        let mut player = codec::read_player(&record, schema.player)
            .expect("every field of the version's player record accepts 0");
        if let Some((_, appearance)) = schema.appearance {
            let record = vec![0; appearance.size];
            codec::read_player_into(&mut player, &record, appearance)
                .expect("every field of the version's appearance record accepts 0");
        }
        player
    }

    /// The model value a bit run carries, in codec field order.
    pub(crate) fn get(&self, field: PlayerField) -> Result<u32, CodecError> {
        let v = match field {
            PlayerField::Id => self.id,
            PlayerField::Nationality => u32::from(self.basic.nationality),
            PlayerField::Height => u32::from(self.basic.height),
            PlayerField::Weight => u32::from(self.basic.weight),
            PlayerField::GoalCelebration1 => u32::from(self.motion.goal_celebration_1),
            PlayerField::GoalCelebration2 => u32::from(self.motion.goal_celebration_2),
            PlayerField::AttackingProwess => u32::from(self.stats.attacking_prowess),
            PlayerField::DefensiveProwess => u32::from(self.stats.defensive_prowess),
            PlayerField::Goalkeeping => u32::from(self.stats.goalkeeping),
            PlayerField::Dribbling => u32::from(self.stats.dribbling),
            PlayerField::FreeKickMotion => u32::from(self.motion.free_kick),
            PlayerField::Finishing => u32::from(self.stats.finishing),
            PlayerField::LowPass => u32::from(self.stats.low_pass),
            PlayerField::LoftedPass => u32::from(self.stats.lofted_pass),
            PlayerField::Heading => u32::from(self.stats.heading),
            PlayerField::Form => u32::from(self.stats.form),
            PlayerField::EditedPlayer => u32::from(self.edit_flags.player),
            PlayerField::Swerve => u32::from(self.stats.swerve),
            PlayerField::Catching => {
                u32::from(self.stats.catching.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::Clearing => {
                u32::from(self.stats.clearing.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::Reflexes => {
                u32::from(self.stats.reflexes.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::InjuryResistance => u32::from(self.stats.injury_resistance),
            PlayerField::EditedBasicSettings => u32::from(self.edit_flags.basic_settings),
            PlayerField::BodyControl => u32::from(self.stats.body_control),
            PlayerField::PhysicalContact => u32::from(
                self.stats
                    .physical_contact
                    .ok_or_else(|| codec::missing(field))?,
            ),
            PlayerField::KickingPower => u32::from(self.stats.kicking_power),
            PlayerField::ExplosivePower => u32::from(self.stats.explosive_power),
            PlayerField::ArmMovementDribbling => u32::from(self.motion.arm_movement_dribbling),
            PlayerField::EditedRegisteredPosition => u32::from(self.edit_flags.registered_position),
            PlayerField::Age => u32::from(self.basic.age),
            PlayerField::RegisteredPosition => u32::from(self.positions.registered),
            PlayerField::PlayingStyle => u32::from(self.positions.playing_style),
            PlayerField::BallControl => u32::from(self.stats.ball_control),
            PlayerField::BallWinning => u32::from(self.stats.ball_winning),
            PlayerField::WeakFootAccuracy => u32::from(self.stats.weak_foot_accuracy),
            PlayerField::Jump => u32::from(self.stats.jump),
            PlayerField::ArmMovementRunning => u32::from(self.motion.arm_movement_running),
            PlayerField::CornerKickMotion => u32::from(self.motion.corner_kick),
            PlayerField::Coverage => {
                u32::from(self.stats.coverage.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::WeakFootUsage => u32::from(self.stats.weak_foot_usage),
            PlayerField::PlayablePosition(i) => u32::from(
                *self
                    .positions
                    .playable
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
            PlayerField::HunchingDribbling => u32::from(self.motion.hunching_dribbling),
            PlayerField::HunchingRunning => u32::from(self.motion.hunching_running),
            PlayerField::PenaltyKickMotion => u32::from(self.motion.penalty_kick),
            PlayerField::PlaceKicking => u32::from(self.stats.place_kicking),
            PlayerField::EditedPlayablePositions => u32::from(self.edit_flags.playable_positions),
            PlayerField::EditedAbilities => u32::from(self.edit_flags.abilities),
            PlayerField::EditedSkills => u32::from(self.edit_flags.skills),
            PlayerField::Stamina => u32::from(self.stats.stamina),
            PlayerField::Speed => u32::from(self.stats.speed),
            PlayerField::EditedPlayingStyle => u32::from(self.edit_flags.playing_style),
            PlayerField::EditedComStyles => u32::from(self.edit_flags.com_styles),
            PlayerField::EditedMotion => u32::from(self.edit_flags.motion),
            PlayerField::BaseCopy => u32::from(
                self.edit_flags
                    .base_copy
                    .ok_or_else(|| codec::missing(field))?,
            ),
            PlayerField::StrongerFoot => u32::from(self.positions.stronger_foot),
            PlayerField::ComStyle(i) => u32::from(
                *self
                    .skills
                    .com_styles
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
            PlayerField::Skill(i) => u32::from(
                *self
                    .skills
                    .skills
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?,
            ),
            PlayerField::Star => u32::from(self.stats.star.ok_or_else(|| codec::missing(field))?),
            PlayerField::DribblingMotion => {
                u32::from(self.motion.dribbling.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::TightPossession => u32::from(
                self.stats
                    .tight_possession
                    .ok_or_else(|| codec::missing(field))?,
            ),
            PlayerField::Aggression => {
                u32::from(self.stats.aggression.ok_or_else(|| codec::missing(field))?)
            }
            PlayerField::PlayingAttitude => u32::from(
                self.stats
                    .playing_attitude
                    .ok_or_else(|| codec::missing(field))?,
            ),
            PlayerField::StrongerHand => u32::from(
                self.positions
                    .stronger_hand
                    .ok_or_else(|| codec::missing(field))?,
            ),
            PlayerField::AppearanceId => self.appearance_id.ok_or_else(|| codec::missing(field))?,
            PlayerField::EditedFace => u32::from(self.edit_flags.face),
            PlayerField::EditedHair => u32::from(self.edit_flags.hair),
            PlayerField::EditedPhysique => u32::from(self.edit_flags.physique),
            PlayerField::EditedStrip => u32::from(self.edit_flags.strip),
            PlayerField::BootsId => self.appearance.boots_id,
            PlayerField::GlovesId => self.appearance.gloves_id,
            PlayerField::BaseCopyId => self.appearance.base_copy_id,
            PlayerField::NeckLength => u32::from(self.appearance.neck_length),
            PlayerField::NeckSize => u32::from(self.appearance.neck_size),
            PlayerField::ShoulderHeight => u32::from(self.appearance.shoulder_height),
            PlayerField::ShoulderWidth => u32::from(self.appearance.shoulder_width),
            PlayerField::Chest => u32::from(self.appearance.chest),
            PlayerField::Waist => u32::from(self.appearance.waist),
            PlayerField::ArmSize => u32::from(self.appearance.arm_size),
            PlayerField::ArmLength => u32::from(self.appearance.arm_length),
            PlayerField::Thigh => u32::from(self.appearance.thigh),
            PlayerField::Calf => u32::from(self.appearance.calf),
            PlayerField::LegLength => u32::from(self.appearance.leg_length),
            PlayerField::HeadLength => u32::from(self.appearance.head_length),
            PlayerField::HeadWidth => u32::from(self.appearance.head_width),
            PlayerField::HeadDepth => u32::from(self.appearance.head_depth),
            PlayerField::WristTapeColorLeft => u32::from(self.appearance.wrist_tape_color_left),
            PlayerField::WristTapeColorRight => u32::from(self.appearance.wrist_tape_color_right),
            PlayerField::WristTaping => u32::from(self.appearance.wrist_taping),
            PlayerField::SpectaclesColor => u32::from(self.appearance.spectacles_color),
            PlayerField::SpectaclesStyle => u32::from(self.appearance.spectacles_style),
            PlayerField::Sleeves => u32::from(self.appearance.sleeves),
            PlayerField::Inners => u32::from(self.appearance.inners),
            PlayerField::Socks => u32::from(self.appearance.socks),
            PlayerField::Undershorts => u32::from(self.appearance.undershorts),
            PlayerField::Untucked => u32::from(self.appearance.untucked),
            PlayerField::AnkleTaping => u32::from(self.appearance.ankle_taping),
        };
        Ok(v)
    }

    /// Patches a bit-run value into the model, in codec field order. A gated
    /// field becomes `Some` because the schema that produced it has the run.
    pub(crate) fn set(&mut self, field: PlayerField, value: u32) -> Result<(), CodecError> {
        match field {
            PlayerField::Id => self.id = value,
            PlayerField::Nationality => self.basic.nationality = value as u16,
            PlayerField::Height => self.basic.height = value as u8,
            PlayerField::Weight => self.basic.weight = value as u8,
            PlayerField::GoalCelebration1 => self.motion.goal_celebration_1 = value as u8,
            PlayerField::GoalCelebration2 => self.motion.goal_celebration_2 = value as u8,
            PlayerField::AttackingProwess => self.stats.attacking_prowess = value as u8,
            PlayerField::DefensiveProwess => self.stats.defensive_prowess = value as u8,
            PlayerField::Goalkeeping => self.stats.goalkeeping = value as u8,
            PlayerField::Dribbling => self.stats.dribbling = value as u8,
            PlayerField::FreeKickMotion => self.motion.free_kick = value as u8,
            PlayerField::Finishing => self.stats.finishing = value as u8,
            PlayerField::LowPass => self.stats.low_pass = value as u8,
            PlayerField::LoftedPass => self.stats.lofted_pass = value as u8,
            PlayerField::Heading => self.stats.heading = value as u8,
            PlayerField::Form => self.stats.form = value as u8,
            PlayerField::EditedPlayer => self.edit_flags.player = value != 0,
            PlayerField::Swerve => self.stats.swerve = value as u8,
            PlayerField::Catching => self.stats.catching = Some(value as u8),
            PlayerField::Clearing => self.stats.clearing = Some(value as u8),
            PlayerField::Reflexes => self.stats.reflexes = Some(value as u8),
            PlayerField::InjuryResistance => self.stats.injury_resistance = value as u8,
            PlayerField::EditedBasicSettings => self.edit_flags.basic_settings = value != 0,
            PlayerField::BodyControl => self.stats.body_control = value as u8,
            PlayerField::PhysicalContact => self.stats.physical_contact = Some(value as u8),
            PlayerField::KickingPower => self.stats.kicking_power = value as u8,
            PlayerField::ExplosivePower => self.stats.explosive_power = value as u8,
            PlayerField::ArmMovementDribbling => self.motion.arm_movement_dribbling = value as u8,
            PlayerField::EditedRegisteredPosition => {
                self.edit_flags.registered_position = value != 0;
            }
            PlayerField::Age => self.basic.age = value as u8,
            PlayerField::RegisteredPosition => self.positions.registered = value as u8,
            PlayerField::PlayingStyle => self.positions.playing_style = value as u8,
            PlayerField::BallControl => self.stats.ball_control = value as u8,
            PlayerField::BallWinning => self.stats.ball_winning = value as u8,
            PlayerField::WeakFootAccuracy => self.stats.weak_foot_accuracy = value as u8,
            PlayerField::Jump => self.stats.jump = value as u8,
            PlayerField::ArmMovementRunning => self.motion.arm_movement_running = value as u8,
            PlayerField::CornerKickMotion => self.motion.corner_kick = value as u8,
            PlayerField::Coverage => self.stats.coverage = Some(value as u8),
            PlayerField::WeakFootUsage => self.stats.weak_foot_usage = value as u8,
            PlayerField::PlayablePosition(i) => {
                *self
                    .positions
                    .playable
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value as u8;
            }
            PlayerField::HunchingDribbling => self.motion.hunching_dribbling = value as u8,
            PlayerField::HunchingRunning => self.motion.hunching_running = value as u8,
            PlayerField::PenaltyKickMotion => self.motion.penalty_kick = value as u8,
            PlayerField::PlaceKicking => self.stats.place_kicking = value as u8,
            PlayerField::EditedPlayablePositions => {
                self.edit_flags.playable_positions = value != 0;
            }
            PlayerField::EditedAbilities => self.edit_flags.abilities = value != 0,
            PlayerField::EditedSkills => self.edit_flags.skills = value != 0,
            PlayerField::Stamina => self.stats.stamina = value as u8,
            PlayerField::Speed => self.stats.speed = value as u8,
            PlayerField::EditedPlayingStyle => self.edit_flags.playing_style = value != 0,
            PlayerField::EditedComStyles => self.edit_flags.com_styles = value != 0,
            PlayerField::EditedMotion => self.edit_flags.motion = value != 0,
            PlayerField::BaseCopy => self.edit_flags.base_copy = Some(value != 0),
            PlayerField::StrongerFoot => self.positions.stronger_foot = value as u8,
            PlayerField::ComStyle(i) => {
                *self
                    .skills
                    .com_styles
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value != 0;
            }
            PlayerField::Skill(i) => {
                *self
                    .skills
                    .skills
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))? = value != 0;
            }
            PlayerField::Star => self.stats.star = Some(value as u8),
            PlayerField::DribblingMotion => self.motion.dribbling = Some(value as u8),
            PlayerField::TightPossession => self.stats.tight_possession = Some(value as u8),
            PlayerField::Aggression => self.stats.aggression = Some(value as u8),
            PlayerField::PlayingAttitude => self.stats.playing_attitude = Some(value as u8),
            PlayerField::StrongerHand => self.positions.stronger_hand = Some(value as u8),
            PlayerField::AppearanceId => self.appearance_id = Some(value),
            PlayerField::EditedFace => self.edit_flags.face = value != 0,
            PlayerField::EditedHair => self.edit_flags.hair = value != 0,
            PlayerField::EditedPhysique => self.edit_flags.physique = value != 0,
            PlayerField::EditedStrip => self.edit_flags.strip = value != 0,
            PlayerField::BootsId => self.appearance.boots_id = value,
            PlayerField::GlovesId => self.appearance.gloves_id = value,
            PlayerField::BaseCopyId => self.appearance.base_copy_id = value,
            PlayerField::NeckLength => self.appearance.neck_length = value as u8,
            PlayerField::NeckSize => self.appearance.neck_size = value as u8,
            PlayerField::ShoulderHeight => self.appearance.shoulder_height = value as u8,
            PlayerField::ShoulderWidth => self.appearance.shoulder_width = value as u8,
            PlayerField::Chest => self.appearance.chest = value as u8,
            PlayerField::Waist => self.appearance.waist = value as u8,
            PlayerField::ArmSize => self.appearance.arm_size = value as u8,
            PlayerField::ArmLength => self.appearance.arm_length = value as u8,
            PlayerField::Thigh => self.appearance.thigh = value as u8,
            PlayerField::Calf => self.appearance.calf = value as u8,
            PlayerField::LegLength => self.appearance.leg_length = value as u8,
            PlayerField::HeadLength => self.appearance.head_length = value as u8,
            PlayerField::HeadWidth => self.appearance.head_width = value as u8,
            PlayerField::HeadDepth => self.appearance.head_depth = value as u8,
            PlayerField::WristTapeColorLeft => self.appearance.wrist_tape_color_left = value as u8,
            PlayerField::WristTapeColorRight => {
                self.appearance.wrist_tape_color_right = value as u8
            }
            PlayerField::WristTaping => self.appearance.wrist_taping = value as u8,
            PlayerField::SpectaclesColor => self.appearance.spectacles_color = value as u8,
            PlayerField::SpectaclesStyle => self.appearance.spectacles_style = value as u8,
            PlayerField::Sleeves => self.appearance.sleeves = value as u8,
            PlayerField::Inners => self.appearance.inners = value as u8,
            PlayerField::Socks => self.appearance.socks = value as u8,
            PlayerField::Undershorts => self.appearance.undershorts = value as u8,
            PlayerField::Untucked => self.appearance.untucked = value != 0,
            PlayerField::AnkleTaping => self.appearance.ankle_taping = value != 0,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writing index `i` reads back through `abilities()` at `i`; a stat the
    /// version lacks stays `None`.
    #[test]
    fn set_ability_writes_the_field_abilities_reads() {
        // PES 21 shape: every gated stat present.
        let mut stats = PlayerStats {
            physical_contact: Some(0),
            catching: Some(0),
            clearing: Some(0),
            reflexes: Some(0),
            coverage: Some(0),
            tight_possession: Some(0),
            aggression: Some(0),
            ..Default::default()
        };
        for i in 0..25u8 {
            stats.set_ability(usize::from(i), 30 + i);
        }
        let want: Vec<Option<u8>> = (0..25u8).map(|i| Some(30 + i)).collect();
        assert_eq!(stats.abilities().to_vec(), want);
        // PES 15 shape: the gated stats absent stay absent.
        let mut stats = PlayerStats::default();
        for i in 0..25u8 {
            stats.set_ability(usize::from(i), 50);
        }
        let abilities = stats.abilities();
        for index in [15usize, 18, 19, 20, 21, 23, 24] {
            assert_eq!(abilities[index], None);
        }
        assert_eq!(abilities.iter().flatten().count(), 18);
        assert!(abilities.iter().flatten().all(|v| *v == 50));
    }

    /// `bare` carries every field the version stores at zero and leaves
    /// the fields it lacks absent: `abilities()` is `Some(0)` exactly
    /// where the schema has the stat, and the tactics' auto flags are
    /// `Some` zero or false exactly where the tactics schema has them.
    #[test]
    fn bare_records_carry_the_versions_fields() {
        use crate::model::tactics::TeamTactics;
        use crate::schema::fields::TacticsField;

        use crate::schema::fields::ABILITY_FIELDS;

        for version in PesVersion::ALL {
            let schema = schema_for(version);
            let abilities = PlayerEntry::bare(version).stats.abilities();
            for (index, field) in ABILITY_FIELDS.iter().enumerate() {
                let expected = schema.player_has(*field).then_some(0);
                assert_eq!(abilities[index], expected, "{version:?} {field:?}");
            }
            let auto = TeamTactics::bare(version).auto;
            assert_eq!(
                auto.substitution.is_some_and(|value| value == 0),
                schema.tactic.has(TacticsField::AutoSubstitution),
                "{version:?} substitution"
            );
            assert_eq!(
                auto.offside_trap.is_some_and(|value| !value),
                schema.tactic.has(TacticsField::AutoOffsideTrap),
                "{version:?} offside_trap"
            );
            assert_eq!(
                auto.preset_change.is_some_and(|value| !value),
                schema.tactic.has(TacticsField::AutoPresetChange),
                "{version:?} preset_change"
            );
            assert_eq!(
                auto.attack_defence_levels.is_some_and(|value| !value),
                schema.tactic.has(TacticsField::AutoAttackDefenceLevels),
                "{version:?} attack_defence_levels"
            );
        }
        // The gated fields the versions differ on, spelled out.
        let pes15 = PlayerEntry::bare(PesVersion::Pes15);
        assert_eq!(pes15.stats.physical_contact, None);
        assert_eq!(pes15.stats.catching, Some(0));
        assert_eq!(
            PlayerEntry::bare(PesVersion::Pes17).stats.physical_contact,
            Some(0)
        );
        for version in [PesVersion::Pes15, PesVersion::Pes16] {
            assert_eq!(
                TeamTactics::bare(version).auto.attack_defence_levels,
                None,
                "{version:?}"
            );
        }
    }

    /// A bare tactics record names no roster slot: every set-piece taker,
    /// man-marking byte, starting-eleven and bench entry is 0xFF, the
    /// record's none, so a document that omits them reads as nobody rather
    /// than as roster slot 0.
    #[test]
    fn bare_tactics_name_no_roster_slot() {
        use crate::model::tactics::{SetPieceTakers, TeamTactics};

        let none = SetPieceTakers {
            free_kick_long: 0xFF,
            free_kick_short: 0xFF,
            free_kick_second: 0xFF,
            corner_left: 0xFF,
            corner_right: 0xFF,
            penalty: 0xFF,
            captain: 0xFF,
        };
        for version in PesVersion::ALL {
            let tactics = TeamTactics::bare(version);
            assert_eq!(tactics.set_pieces, none, "{version:?}");
            for preset in &tactics.presets {
                assert_eq!(preset.man_marking, [0xFF; 11], "{version:?}");
            }
            assert_eq!(tactics.starting_eleven, [0xFF; 11], "{version:?}");
            assert_eq!(tactics.bench_order, [0xFF; 21], "{version:?}");
            assert_eq!(tactics.players_to_join_attack, [0xFF; 3], "{version:?}");
        }
    }
}
