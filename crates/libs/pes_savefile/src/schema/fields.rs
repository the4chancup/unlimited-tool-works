//! The version-neutral field vocabularies the schema tables name and the codec fills.
//! Indexed variants carry the element index (`Skill(3)`); the tables say where each
//! version stores it.

use serde::{Deserialize, Serialize};

/// A bit-run field of the player record (the appearance record on PES 15/16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerField {
    /// The player id (team id x 100 + slot for cup players).
    Id,
    /// Nationality id.
    Nationality,
    /// Height in cm.
    Height,
    /// Weight in kg.
    Weight,
    /// First goal celebration.
    GoalCelebration1,
    /// Second goal celebration.
    GoalCelebration2,
    /// Attacking Prowess (Offensive Awareness).
    AttackingProwess,
    /// Defensive Prowess (Defensive Awareness).
    DefensiveProwess,
    /// Goalkeeping (GK Awareness).
    Goalkeeping,
    /// Dribbling.
    Dribbling,
    /// Free kick motion.
    FreeKickMotion,
    /// Finishing.
    Finishing,
    /// Low Pass.
    LowPass,
    /// Lofted Pass.
    LoftedPass,
    /// Header (Heading).
    Heading,
    /// Form (Condition), 1 to 8 stored 0 to 7.
    Form,
    /// The player was created or edited.
    EditedPlayer,
    /// Swerve (Curl).
    Swerve,
    /// Catching (Saving on PES 15, GK Catching later).
    Catching,
    /// Clearing (GK Clearing).
    Clearing,
    /// Reflexes (GK Reflexes).
    Reflexes,
    /// Injury Resistance.
    InjuryResistance,
    /// The basic settings were edited.
    EditedBasicSettings,
    /// Body Control (Body Balance on PES 16).
    BodyControl,
    /// Physical Contact.
    PhysicalContact,
    /// Kicking Power.
    KickingPower,
    /// Explosive Power (Acceleration).
    ExplosivePower,
    /// Arm movement while dribbling.
    ArmMovementDribbling,
    /// The registered position was edited.
    EditedRegisteredPosition,
    /// Age.
    Age,
    /// Registered position, GK 0 to CF 12.
    RegisteredPosition,
    /// Playing style, a version-specific index (see `schema::playstyle`).
    PlayingStyle,
    /// Ball Control.
    BallControl,
    /// Ball Winning (Tackling).
    BallWinning,
    /// Weak Foot Accuracy.
    WeakFootAccuracy,
    /// Jump.
    Jump,
    /// Arm movement while running.
    ArmMovementRunning,
    /// Corner kick motion.
    CornerKickMotion,
    /// Coverage (GK Coverage).
    Coverage,
    /// Weak Foot Usage.
    WeakFootUsage,
    /// Playable position rating `n` of the save's order (0 C, 1 B, 2 A; 3
    /// held by no save), CF at 0 to GK at 12.
    PlayablePosition(u8),
    /// Hunching while dribbling.
    HunchingDribbling,
    /// Hunching while running.
    HunchingRunning,
    /// Penalty kick motion.
    PenaltyKickMotion,
    /// Place Kicking.
    PlaceKicking,
    /// The playable positions were edited.
    EditedPlayablePositions,
    /// The abilities were edited.
    EditedAbilities,
    /// The skills were edited.
    EditedSkills,
    /// Stamina.
    Stamina,
    /// Speed.
    Speed,
    /// The playing style was edited.
    EditedPlayingStyle,
    /// The COM playing styles were edited.
    EditedComStyles,
    /// The motions were edited.
    EditedMotion,
    /// The player is a base copy of another.
    BaseCopy,
    /// Stronger foot (0 right, 1 left).
    StrongerFoot,
    /// COM playing style `n` (0 Trickster to 6 Long Ranger).
    ComStyle(u8),
    /// Player skill `n` (0 Scissors Feint to 40 Through Passing).
    Skill(u8),
    /// Star rating (PES 19+).
    Star,
    /// Dribbling motion (PES 20+).
    DribblingMotion,
    /// Tight Possession (PES 20+).
    TightPossession,
    /// Aggression (PES 20+).
    Aggression,
    /// Playing attitude (PES 20+).
    PlayingAttitude,
    /// Stronger hand (PES 20+).
    StrongerHand,
    /// The id of the player whose appearance the block holds: the player's own, or -1 (all bits set) on a stripped Fox save, where the database tables decide the look.
    AppearanceId,
    /// The face was edited.
    EditedFace,
    /// The hairstyle was edited.
    EditedHair,
    /// The physique was edited.
    EditedPhysique,
    /// The strip style was edited.
    EditedStrip,
    /// Boots model id.
    BootsId,
    /// Gloves model id (any player; 0 = normal hands).
    GlovesId,
    /// Base copy player id (the player's own id when unset).
    BaseCopyId,
    /// Neck length.
    NeckLength,
    /// Neck size.
    NeckSize,
    /// Shoulder height.
    ShoulderHeight,
    /// Shoulder width.
    ShoulderWidth,
    /// Chest measurement.
    Chest,
    /// Waist size.
    Waist,
    /// Arm size.
    ArmSize,
    /// Arm length.
    ArmLength,
    /// Thigh size.
    Thigh,
    /// Calf size.
    Calf,
    /// Leg length.
    LegLength,
    /// Head length.
    HeadLength,
    /// Head width.
    HeadWidth,
    /// Head depth.
    HeadDepth,
    /// Left wrist tape colour.
    WristTapeColorLeft,
    /// Right wrist tape colour.
    WristTapeColorRight,
    /// Wrist taping.
    WristTaping,
    /// Spectacles frame colour.
    SpectaclesColor,
    /// Spectacles style.
    SpectaclesStyle,
    /// Sleeves.
    Sleeves,
    /// Long-sleeved inners.
    Inners,
    /// Sock length.
    Socks,
    /// Undershorts.
    Undershorts,
    /// Shirttail out.
    Untucked,
    /// Ankle taping.
    AnkleTaping,
}

/// A text field of the player record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerText {
    /// The player name, UTF-8, colour codes included.
    Name,
    /// The shirt name, single-byte text.
    ShirtName,
}

/// The `PlayerField` of each ability stat, in `PlayerStats::abilities`' order
/// (`interchange::team_toml::labels::ABILITY_STATS`').
pub const ABILITY_FIELDS: [PlayerField; 25] = [
    PlayerField::AttackingProwess,
    PlayerField::BallControl,
    PlayerField::Dribbling,
    PlayerField::LowPass,
    PlayerField::LoftedPass,
    PlayerField::Finishing,
    PlayerField::PlaceKicking,
    PlayerField::Swerve,
    PlayerField::Heading,
    PlayerField::DefensiveProwess,
    PlayerField::BallWinning,
    PlayerField::KickingPower,
    PlayerField::Speed,
    PlayerField::ExplosivePower,
    PlayerField::BodyControl,
    PlayerField::PhysicalContact,
    PlayerField::Jump,
    PlayerField::Goalkeeping,
    PlayerField::Catching,
    PlayerField::Clearing,
    PlayerField::Reflexes,
    PlayerField::Coverage,
    PlayerField::Stamina,
    PlayerField::TightPossession,
    PlayerField::Aggression,
];

/// A bit-run field of the team record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamField {
    /// The team id.
    Id,
    /// Manager id (PES 19+).
    ManagerId,
    /// Home stadium id (PES 19+).
    StadiumId,
    /// The team name was edited.
    EditedName,
    /// The short name was edited (PES 15).
    EditedShortName,
    /// The home stadium was edited (PES 20+).
    EditedStadium,
    /// The strips were edited (PES 17).
    EditedStrip,
    /// First team colour, red, 6 bits.
    Color1Red,
    /// First team colour, green, 6 bits.
    Color1Green,
    /// First team colour, blue, 6 bits.
    Color1Blue,
    /// Second team colour, red, 6 bits.
    Color2Red,
    /// Second team colour, green, 6 bits.
    Color2Green,
    /// Second team colour, blue, 6 bits.
    Color2Blue,
    /// Kit slot `n`: kit number from 0, or 0x80 for a goalkeeper kit.
    KitSlotNumber(u8),
    /// Kit slot `n`: the bound team id x 0x40.
    KitSlotTeam(u8),
}

/// A text field of the team record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamText {
    /// The team name, UTF-8.
    Name,
    /// The three-letter short name.
    ShortName,
}

/// A field of the roster record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RosterField {
    /// The team id the roster belongs to.
    TeamId,
    /// Roster slot `n`: player id, 0 when empty.
    Player(u8),
    /// Roster slot `n`: shirt number.
    Number(u8),
}

/// A field of the tactics record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TacticsField {
    /// The team id the tactics belong to.
    TeamId,
    /// Per-preset setting `field` of preset `preset` (0 to 2).
    Preset {
        /// The preset (0 to 2).
        preset: u8,
        /// The setting within the preset.
        field: PresetField,
    },
    /// Formation `formation` (0 kick-off, 1 in possession, 2 out of possession) of preset `preset`, player `slot` (0 to 10).
    Formation {
        /// The preset (0 to 2).
        preset: u8,
        /// The formation (0 kick-off, 1 in possession, 2 out of possession).
        formation: u8,
        /// The player slot (0 to 10).
        slot: u8,
        /// Which value of the slot.
        part: FormationPart,
    },
    /// Advanced instruction `index` (0 or 1) on `side` of preset `preset`.
    Instruction {
        /// The preset (0 to 2).
        preset: u8,
        /// Attacking or defending instruction.
        side: InstructionSide,
        /// The instruction index (0 or 1).
        index: u8,
        /// Which value of the instruction.
        part: InstructionPart,
    },
    /// Man-marking assignment of own formation slot `slot` in preset `preset`: the marked opponent's formation slot, 0xFF none.
    ManMarking {
        /// The preset (0 to 2).
        preset: u8,
        /// The own formation slot (0 to 10).
        slot: u8,
    },
    /// Long free kick taker (roster slot, 0xFF none).
    FreeKickTakerLong,
    /// Short free kick taker.
    FreeKickTakerShort,
    /// Second free kick taker.
    FreeKickTakerSecond,
    /// Left corner taker.
    CornerTakerLeft,
    /// Right corner taker.
    CornerTakerRight,
    /// Penalty taker.
    PenaltyTaker,
    /// Captain (roster slot).
    Captain,
    /// Auto substitution setting.
    AutoSubstitution,
    /// Auto offside trap.
    AutoOffsideTrap,
    /// Auto preset change.
    AutoPresetChange,
    /// Auto attack/defence level change.
    AutoAttackDefenceLevels,
    /// Starting eleven position `n`: roster slot.
    Starting(u8),
    /// Bench order entry `n`: roster slot.
    Bench(u8),
    /// Player `n` joining the attack.
    PlayerToJoinAttack(u8),
}

/// A per-preset tactics setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresetField {
    /// 0 counter attack, 1 possession.
    AttackingStyle,
    /// 0 long pass, 1 short pass.
    Buildup,
    /// 0 centre, 1 wide.
    AttackingZone,
    /// 0 maintain, 1 flexible.
    Positioning,
    /// 0 frontline pressure, 1 all-out defence.
    DefensiveStyle,
    /// 0 middle, 1 wide.
    ContainmentArea,
    /// 0 aggressive, 1 conservative.
    Pressure,
    /// Fluid formation on/off (PES 16+).
    FluidFormation,
    /// Support range, 1 to 10.
    SupportRange,
    /// Defensive line, 1 to 10.
    DefensiveLine,
    /// Compactness, 1 to 10.
    Compactness,
    /// Numbers in attack, 1 few to 3 many.
    NumbersInAttack,
    /// Numbers in defence, 1 few to 3 many.
    NumbersInDefence,
}

/// Which value of a formation slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormationPart {
    /// Position, GK 0 to CF 12.
    Position,
    /// Horizontal coordinate.
    X,
    /// Vertical coordinate.
    Y,
}

/// Attacking or defending instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionSide {
    /// An attacking instruction.
    Attack,
    /// A defending instruction.
    Defence,
}

/// Which value of an advanced instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionPart {
    /// The instruction id (version-specific meaning).
    Instruction,
    /// The targeted player, for instructions that take one.
    PlayerId,
}
