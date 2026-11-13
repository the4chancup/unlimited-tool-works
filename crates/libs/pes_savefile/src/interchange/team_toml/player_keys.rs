//! The `[players.NN]` scalar-key table: one `PlayerKey` per scalar key of the
//! TOML block in `spec/pes_savefile/operations.md` "Team TOML", in the block's order,
//! driving parse, emit and apply for every scalar field. The non-scalar keys
//! (`name`, `shirt_name`, `number`, `positions.playable`,
//! `positions.playing_style`, `skills.skills`, `skills.com_styles`,
//! `appearance`) are handled individually in `player`.

#[cfg(test)]
use pes_version::PesVersion;
use toml_edit::Item;

use crate::interchange::team_toml::labels;
use crate::interchange::team_toml::{PlayerSection, TeamTomlError};
use crate::model::player::PlayerEntry;
use crate::schema::fields::PlayerField;
#[cfg(test)]
use crate::schema::schema_for;
use crate::settings_toml::SettingKey;

use super::items::label_position;

/// How a key's TOML value maps to the stored `u32`, and the range it accepts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    /// An ability stat, stored 0 to 127 (the game shows 40 to 99).
    Ability,
    /// A stored integer: 0 to the widest `FieldSpec` bit width's maximum,
    /// measured over every version's player and appearance record schemas.
    Stored,
    /// true | false.
    Bool,
    /// 0 to 65535.
    U16,
    /// 0 to 4294967295.
    U32,
    /// The label's index into the list.
    Label(&'static [&'static str]),
}

/// The columns of one key-table row.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeySpec {
    /// The key's dotted path within the player ("stats.finishing").
    pub path: &'static str,
    /// How the TOML value maps to the stored `u32`.
    pub kind: Kind,
    /// The savefile field: the model accessor and the apply-time gate.
    pub field: PlayerField,
    /// The comment emitted after the key, verbatim from the spec's TOML block.
    pub comment: &'static str,
}

/// One scalar key of `[players.NN]` and its `stats`/`positions`/`edit_flags`
/// subtables, in the block's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerKey {
    /// `nationality`
    Nationality,
    /// `age`
    Age,
    /// `height`
    Height,
    /// `weight`
    Weight,
    /// `boots_id`
    BootsId,
    /// `gloves_id`
    GlovesId,
    /// `base_copy_id`
    BaseCopyId,
    /// `stats.attacking_prowess`
    AttackingProwess,
    /// `stats.ball_control`
    BallControl,
    /// `stats.dribbling`
    Dribbling,
    /// `stats.tight_possession`
    TightPossession,
    /// `stats.low_pass`
    LowPass,
    /// `stats.lofted_pass`
    LoftedPass,
    /// `stats.finishing`
    Finishing,
    /// `stats.heading`
    Heading,
    /// `stats.place_kicking`
    PlaceKicking,
    /// `stats.swerve`
    Swerve,
    /// `stats.defensive_prowess`
    DefensiveProwess,
    /// `stats.ball_winning`
    BallWinning,
    /// `stats.aggression`
    Aggression,
    /// `stats.kicking_power`
    KickingPower,
    /// `stats.speed`
    Speed,
    /// `stats.explosive_power`
    ExplosivePower,
    /// `stats.body_control`
    BodyControl,
    /// `stats.physical_contact`
    PhysicalContact,
    /// `stats.jump`
    Jump,
    /// `stats.stamina`
    Stamina,
    /// `stats.goalkeeping`
    Goalkeeping,
    /// `stats.catching`
    Catching,
    /// `stats.clearing`
    Clearing,
    /// `stats.reflexes`
    Reflexes,
    /// `stats.coverage`
    Coverage,
    /// `stats.weak_foot_usage`
    WeakFootUsage,
    /// `stats.weak_foot_accuracy`
    WeakFootAccuracy,
    /// `stats.form`
    Form,
    /// `stats.injury_resistance`
    InjuryResistance,
    /// `stats.star`
    Star,
    /// `stats.playing_attitude`
    PlayingAttitude,
    /// `positions.registered`
    Registered,
    /// `positions.stronger_foot`
    StrongerFoot,
    /// `positions.stronger_hand`
    StrongerHand,
    /// `edit_flags.player`
    EditedPlayer,
    /// `edit_flags.basic_settings`
    EditedBasicSettings,
    /// `edit_flags.registered_position`
    EditedRegisteredPosition,
    /// `edit_flags.playable_positions`
    EditedPlayablePositions,
    /// `edit_flags.abilities`
    EditedAbilities,
    /// `edit_flags.skills`
    EditedSkills,
    /// `edit_flags.playing_style`
    EditedPlayingStyle,
    /// `edit_flags.com_styles`
    EditedComStyles,
    /// `edit_flags.motion`
    EditedMotion,
    /// `edit_flags.base_copy`
    EditedBaseCopy,
    /// `edit_flags.face`
    EditedFace,
    /// `edit_flags.hair`
    EditedHair,
    /// `edit_flags.physique`
    EditedPhysique,
    /// `edit_flags.strip`
    EditedStrip,
}

const POSITIONS_COMMENT: &str = "\"GK\" \"CB\" \"LB\" \"RB\" \"DMF\" \"CMF\" \"LMF\" \"RMF\" \"AMF\" \"LWF\" \"RWF\" \"SS\" \"CF\"";

/// The keys `[players.NN.appearance.presentation]` replaced, `(old-place
/// key, new key)`: every file written before the table existed carries them
/// under `stats`, which an import still reads into the new place (a field
/// given in both is refused), while an export writes the new place alone.
pub(crate) const MOVED_TO_PRESENTATION: &[(PlayerKey, SettingKey)] = &[
    (PlayerKey::Star, SettingKey::Star),
    (PlayerKey::PlayingAttitude, SettingKey::PlayingAttitude),
];

const fn spec(
    path: &'static str,
    kind: Kind,
    field: PlayerField,
    comment: &'static str,
) -> KeySpec {
    KeySpec {
        path,
        kind,
        field,
        comment,
    }
}

/// A `[players.NN.stats]` ability row.
const fn ability(path: &'static str, field: PlayerField, comment: &'static str) -> KeySpec {
    spec(path, Kind::Ability, field, comment)
}

/// A `[players.NN.edit_flags]` row.
const fn flag(path: &'static str, field: PlayerField) -> KeySpec {
    spec(path, Kind::Bool, field, "")
}

impl PlayerKey {
    /// Every scalar key, in the TOML block's order (top level, stats,
    /// positions scalars, edit flags).
    pub(crate) const ALL: [PlayerKey; 55] = [
        PlayerKey::Nationality,
        PlayerKey::Age,
        PlayerKey::Height,
        PlayerKey::Weight,
        PlayerKey::BootsId,
        PlayerKey::GlovesId,
        PlayerKey::BaseCopyId,
        PlayerKey::AttackingProwess,
        PlayerKey::BallControl,
        PlayerKey::Dribbling,
        PlayerKey::TightPossession,
        PlayerKey::LowPass,
        PlayerKey::LoftedPass,
        PlayerKey::Finishing,
        PlayerKey::Heading,
        PlayerKey::PlaceKicking,
        PlayerKey::Swerve,
        PlayerKey::DefensiveProwess,
        PlayerKey::BallWinning,
        PlayerKey::Aggression,
        PlayerKey::KickingPower,
        PlayerKey::Speed,
        PlayerKey::ExplosivePower,
        PlayerKey::BodyControl,
        PlayerKey::PhysicalContact,
        PlayerKey::Jump,
        PlayerKey::Stamina,
        PlayerKey::Goalkeeping,
        PlayerKey::Catching,
        PlayerKey::Clearing,
        PlayerKey::Reflexes,
        PlayerKey::Coverage,
        PlayerKey::WeakFootUsage,
        PlayerKey::WeakFootAccuracy,
        PlayerKey::Form,
        PlayerKey::InjuryResistance,
        PlayerKey::Star,
        PlayerKey::PlayingAttitude,
        PlayerKey::Registered,
        PlayerKey::StrongerFoot,
        PlayerKey::StrongerHand,
        PlayerKey::EditedPlayer,
        PlayerKey::EditedBasicSettings,
        PlayerKey::EditedRegisteredPosition,
        PlayerKey::EditedPlayablePositions,
        PlayerKey::EditedAbilities,
        PlayerKey::EditedSkills,
        PlayerKey::EditedPlayingStyle,
        PlayerKey::EditedComStyles,
        PlayerKey::EditedMotion,
        PlayerKey::EditedBaseCopy,
        PlayerKey::EditedFace,
        PlayerKey::EditedHair,
        PlayerKey::EditedPhysique,
        PlayerKey::EditedStrip,
    ];

    /// The key's row: path, kind, gating field, comment.
    pub(crate) fn spec(self) -> KeySpec {
        use PlayerField as F;
        match self {
            PlayerKey::Nationality => spec("nationality", Kind::U16, F::Nationality, ""),
            PlayerKey::Age => spec("age", Kind::Stored, F::Age, ""),
            PlayerKey::Height => spec("height", Kind::Stored, F::Height, "cm"),
            PlayerKey::Weight => spec("weight", Kind::Stored, F::Weight, "kg"),
            PlayerKey::BootsId => spec("boots_id", Kind::U32, F::BootsId, ""),
            PlayerKey::GlovesId => spec("gloves_id", Kind::U32, F::GlovesId, ""),
            PlayerKey::BaseCopyId => spec(
                "base_copy_id",
                Kind::U32,
                F::BaseCopyId,
                "the player whose look this one copies; his own id (the team id times 100 plus the slot) means none and becomes the target player's own id on import",
            ),
            PlayerKey::AttackingProwess => {
                ability("stats.attacking_prowess", F::AttackingProwess, "")
            }
            PlayerKey::BallControl => ability("stats.ball_control", F::BallControl, ""),
            PlayerKey::Dribbling => ability("stats.dribbling", F::Dribbling, ""),
            PlayerKey::TightPossession => ability(
                "stats.tight_possession",
                F::TightPossession,
                "PES 20 and later",
            ),
            PlayerKey::LowPass => ability("stats.low_pass", F::LowPass, ""),
            PlayerKey::LoftedPass => ability("stats.lofted_pass", F::LoftedPass, ""),
            PlayerKey::Finishing => ability("stats.finishing", F::Finishing, ""),
            PlayerKey::Heading => ability("stats.heading", F::Heading, ""),
            PlayerKey::PlaceKicking => ability("stats.place_kicking", F::PlaceKicking, ""),
            PlayerKey::Swerve => ability("stats.swerve", F::Swerve, ""),
            PlayerKey::DefensiveProwess => {
                ability("stats.defensive_prowess", F::DefensiveProwess, "")
            }
            PlayerKey::BallWinning => ability("stats.ball_winning", F::BallWinning, ""),
            PlayerKey::Aggression => ability("stats.aggression", F::Aggression, "PES 20 and later"),
            PlayerKey::KickingPower => ability("stats.kicking_power", F::KickingPower, ""),
            PlayerKey::Speed => ability("stats.speed", F::Speed, ""),
            PlayerKey::ExplosivePower => ability("stats.explosive_power", F::ExplosivePower, ""),
            PlayerKey::BodyControl => ability("stats.body_control", F::BodyControl, ""),
            PlayerKey::PhysicalContact => ability(
                "stats.physical_contact",
                F::PhysicalContact,
                "PES 17 and later",
            ),
            PlayerKey::Jump => ability("stats.jump", F::Jump, ""),
            PlayerKey::Stamina => ability("stats.stamina", F::Stamina, ""),
            PlayerKey::Goalkeeping => ability("stats.goalkeeping", F::Goalkeeping, ""),
            PlayerKey::Catching => ability("stats.catching", F::Catching, ""),
            PlayerKey::Clearing => ability("stats.clearing", F::Clearing, "PES 16 and later"),
            PlayerKey::Reflexes => ability("stats.reflexes", F::Reflexes, "PES 16 and later"),
            PlayerKey::Coverage => ability("stats.coverage", F::Coverage, "PES 16 and later"),
            PlayerKey::WeakFootUsage => spec(
                "stats.weak_foot_usage",
                Kind::Stored,
                F::WeakFootUsage,
                "0 to 3",
            ),
            PlayerKey::WeakFootAccuracy => spec(
                "stats.weak_foot_accuracy",
                Kind::Stored,
                F::WeakFootAccuracy,
                "0 to 3",
            ),
            PlayerKey::Form => spec("stats.form", Kind::Stored, F::Form, "0 to 7"),
            PlayerKey::InjuryResistance => spec(
                "stats.injury_resistance",
                Kind::Stored,
                F::InjuryResistance,
                "0 to 2",
            ),
            PlayerKey::Star => spec(
                "stats.star",
                Kind::Stored,
                F::Star,
                "PES 19 and later; 0 to 7",
            ),
            PlayerKey::PlayingAttitude => spec(
                "stats.playing_attitude",
                Kind::Stored,
                F::PlayingAttitude,
                "PES 20 and later",
            ),
            PlayerKey::Registered => spec(
                "positions.registered",
                Kind::Label(&labels::POSITIONS),
                F::RegisteredPosition,
                POSITIONS_COMMENT,
            ),
            PlayerKey::StrongerFoot => spec(
                "positions.stronger_foot",
                Kind::Label(&labels::FOOT),
                F::StrongerFoot,
                "\"right\" | \"left\"",
            ),
            PlayerKey::StrongerHand => spec(
                "positions.stronger_hand",
                Kind::Label(&labels::FOOT),
                F::StrongerHand,
                "PES 20 and later; \"right\" | \"left\"",
            ),
            PlayerKey::EditedPlayer => flag("edit_flags.player", F::EditedPlayer),
            PlayerKey::EditedBasicSettings => {
                flag("edit_flags.basic_settings", F::EditedBasicSettings)
            }
            PlayerKey::EditedRegisteredPosition => flag(
                "edit_flags.registered_position",
                F::EditedRegisteredPosition,
            ),
            PlayerKey::EditedPlayablePositions => {
                flag("edit_flags.playable_positions", F::EditedPlayablePositions)
            }
            PlayerKey::EditedAbilities => flag("edit_flags.abilities", F::EditedAbilities),
            PlayerKey::EditedSkills => flag("edit_flags.skills", F::EditedSkills),
            PlayerKey::EditedPlayingStyle => {
                flag("edit_flags.playing_style", F::EditedPlayingStyle)
            }
            PlayerKey::EditedComStyles => flag("edit_flags.com_styles", F::EditedComStyles),
            PlayerKey::EditedMotion => flag("edit_flags.motion", F::EditedMotion),
            PlayerKey::EditedBaseCopy => flag("edit_flags.base_copy", F::BaseCopy),
            PlayerKey::EditedFace => flag("edit_flags.face", F::EditedFace),
            PlayerKey::EditedHair => flag("edit_flags.hair", F::EditedHair),
            PlayerKey::EditedPhysique => flag("edit_flags.physique", F::EditedPhysique),
            PlayerKey::EditedStrip => flag("edit_flags.strip", F::EditedStrip),
        }
    }

    /// The sub-table the key sits in (`""`, `"stats"`, `"positions"`,
    /// `"edit_flags"`).
    pub(crate) fn table(self) -> &'static str {
        match self.spec().path.split_once('.') {
            Some((table, _)) => table,
            None => "",
        }
    }

    /// The key's own name (the last path segment).
    pub(crate) fn name(self) -> &'static str {
        match self.spec().path.rsplit_once('.') {
            Some((_, name)) => name,
            None => self.spec().path,
        }
    }

    /// The player's stored value, `None` when the field is a gated `Option`
    /// this entry's version lacks. `PlayerEntry::get` answers from the model
    /// even for fields the version's schema does not store (they read their
    /// inert default), so the callers pre-gate: `player_from` checks
    /// `schema.player_has(field)` before calling, and `Err` here can only mean
    /// an unread run — treated as absent.
    pub(crate) fn get(self, player: &PlayerEntry) -> Option<u32> {
        player.get(self.spec().field).ok()
    }

    /// Patches the stored value into the model.
    pub(crate) fn set(self, player: &mut PlayerEntry, value: u32) -> Result<(), TeamTomlError> {
        player.set(self.spec().field, value)?;
        Ok(())
    }

    /// The section's value, `bool` as 0/1.
    pub(crate) fn get_section(self, section: &PlayerSection) -> Option<u32> {
        let s = section;
        match self {
            PlayerKey::Nationality => s.nationality.map(u32::from),
            PlayerKey::Age => s.age.map(u32::from),
            PlayerKey::Height => s.height.map(u32::from),
            PlayerKey::Weight => s.weight.map(u32::from),
            PlayerKey::BootsId => s.boots_id,
            PlayerKey::GlovesId => s.gloves_id,
            PlayerKey::BaseCopyId => s.base_copy_id,
            PlayerKey::AttackingProwess => s.stats.attacking_prowess.map(u32::from),
            PlayerKey::BallControl => s.stats.ball_control.map(u32::from),
            PlayerKey::Dribbling => s.stats.dribbling.map(u32::from),
            PlayerKey::TightPossession => s.stats.tight_possession.map(u32::from),
            PlayerKey::LowPass => s.stats.low_pass.map(u32::from),
            PlayerKey::LoftedPass => s.stats.lofted_pass.map(u32::from),
            PlayerKey::Finishing => s.stats.finishing.map(u32::from),
            PlayerKey::Heading => s.stats.heading.map(u32::from),
            PlayerKey::PlaceKicking => s.stats.place_kicking.map(u32::from),
            PlayerKey::Swerve => s.stats.swerve.map(u32::from),
            PlayerKey::DefensiveProwess => s.stats.defensive_prowess.map(u32::from),
            PlayerKey::BallWinning => s.stats.ball_winning.map(u32::from),
            PlayerKey::Aggression => s.stats.aggression.map(u32::from),
            PlayerKey::KickingPower => s.stats.kicking_power.map(u32::from),
            PlayerKey::Speed => s.stats.speed.map(u32::from),
            PlayerKey::ExplosivePower => s.stats.explosive_power.map(u32::from),
            PlayerKey::BodyControl => s.stats.body_control.map(u32::from),
            PlayerKey::PhysicalContact => s.stats.physical_contact.map(u32::from),
            PlayerKey::Jump => s.stats.jump.map(u32::from),
            PlayerKey::Stamina => s.stats.stamina.map(u32::from),
            PlayerKey::Goalkeeping => s.stats.goalkeeping.map(u32::from),
            PlayerKey::Catching => s.stats.catching.map(u32::from),
            PlayerKey::Clearing => s.stats.clearing.map(u32::from),
            PlayerKey::Reflexes => s.stats.reflexes.map(u32::from),
            PlayerKey::Coverage => s.stats.coverage.map(u32::from),
            PlayerKey::WeakFootUsage => s.stats.weak_foot_usage.map(u32::from),
            PlayerKey::WeakFootAccuracy => s.stats.weak_foot_accuracy.map(u32::from),
            PlayerKey::Form => s.stats.form.map(u32::from),
            PlayerKey::InjuryResistance => s.stats.injury_resistance.map(u32::from),
            PlayerKey::Star => s.stats.star.map(u32::from),
            PlayerKey::PlayingAttitude => s.stats.playing_attitude.map(u32::from),
            PlayerKey::Registered => s.positions.registered.map(u32::from),
            PlayerKey::StrongerFoot => s.positions.stronger_foot.map(u32::from),
            PlayerKey::StrongerHand => s.positions.stronger_hand.map(u32::from),
            PlayerKey::EditedPlayer => s.edit_flags.player.map(u32::from),
            PlayerKey::EditedBasicSettings => s.edit_flags.basic_settings.map(u32::from),
            PlayerKey::EditedRegisteredPosition => s.edit_flags.registered_position.map(u32::from),
            PlayerKey::EditedPlayablePositions => s.edit_flags.playable_positions.map(u32::from),
            PlayerKey::EditedAbilities => s.edit_flags.abilities.map(u32::from),
            PlayerKey::EditedSkills => s.edit_flags.skills.map(u32::from),
            PlayerKey::EditedPlayingStyle => s.edit_flags.playing_style.map(u32::from),
            PlayerKey::EditedComStyles => s.edit_flags.com_styles.map(u32::from),
            PlayerKey::EditedMotion => s.edit_flags.motion.map(u32::from),
            PlayerKey::EditedBaseCopy => s.edit_flags.base_copy.map(u32::from),
            PlayerKey::EditedFace => s.edit_flags.face.map(u32::from),
            PlayerKey::EditedHair => s.edit_flags.hair.map(u32::from),
            PlayerKey::EditedPhysique => s.edit_flags.physique.map(u32::from),
            PlayerKey::EditedStrip => s.edit_flags.strip.map(u32::from),
        }
    }

    /// Patches a stored value into the section (0/1 → `bool` for the flags);
    /// `OutOfRange` when `value` is not one the key's kind can represent, and
    /// the section is left untouched.
    pub(crate) fn set_section(
        self,
        section: &mut PlayerSection,
        value: u32,
    ) -> Result<(), TeamTomlError> {
        if !self.accepts(value) {
            return Err(TeamTomlError::OutOfRange {
                key: format!("players.NN.{}", self.spec().path),
                value: i64::from(value),
                range: self.range_text(),
            });
        }
        fn byte(value: u32) -> u8 {
            u8::try_from(value).expect("checked against the kind")
        }
        let s = section;
        match self {
            PlayerKey::Nationality => {
                s.nationality = Some(u16::try_from(value).expect("checked against the kind"))
            }
            PlayerKey::Age => s.age = Some(byte(value)),
            PlayerKey::Height => s.height = Some(byte(value)),
            PlayerKey::Weight => s.weight = Some(byte(value)),
            PlayerKey::BootsId => s.boots_id = Some(value),
            PlayerKey::GlovesId => s.gloves_id = Some(value),
            PlayerKey::BaseCopyId => s.base_copy_id = Some(value),
            PlayerKey::AttackingProwess => s.stats.attacking_prowess = Some(byte(value)),
            PlayerKey::BallControl => s.stats.ball_control = Some(byte(value)),
            PlayerKey::Dribbling => s.stats.dribbling = Some(byte(value)),
            PlayerKey::TightPossession => s.stats.tight_possession = Some(byte(value)),
            PlayerKey::LowPass => s.stats.low_pass = Some(byte(value)),
            PlayerKey::LoftedPass => s.stats.lofted_pass = Some(byte(value)),
            PlayerKey::Finishing => s.stats.finishing = Some(byte(value)),
            PlayerKey::Heading => s.stats.heading = Some(byte(value)),
            PlayerKey::PlaceKicking => s.stats.place_kicking = Some(byte(value)),
            PlayerKey::Swerve => s.stats.swerve = Some(byte(value)),
            PlayerKey::DefensiveProwess => s.stats.defensive_prowess = Some(byte(value)),
            PlayerKey::BallWinning => s.stats.ball_winning = Some(byte(value)),
            PlayerKey::Aggression => s.stats.aggression = Some(byte(value)),
            PlayerKey::KickingPower => s.stats.kicking_power = Some(byte(value)),
            PlayerKey::Speed => s.stats.speed = Some(byte(value)),
            PlayerKey::ExplosivePower => s.stats.explosive_power = Some(byte(value)),
            PlayerKey::BodyControl => s.stats.body_control = Some(byte(value)),
            PlayerKey::PhysicalContact => s.stats.physical_contact = Some(byte(value)),
            PlayerKey::Jump => s.stats.jump = Some(byte(value)),
            PlayerKey::Stamina => s.stats.stamina = Some(byte(value)),
            PlayerKey::Goalkeeping => s.stats.goalkeeping = Some(byte(value)),
            PlayerKey::Catching => s.stats.catching = Some(byte(value)),
            PlayerKey::Clearing => s.stats.clearing = Some(byte(value)),
            PlayerKey::Reflexes => s.stats.reflexes = Some(byte(value)),
            PlayerKey::Coverage => s.stats.coverage = Some(byte(value)),
            PlayerKey::WeakFootUsage => s.stats.weak_foot_usage = Some(byte(value)),
            PlayerKey::WeakFootAccuracy => s.stats.weak_foot_accuracy = Some(byte(value)),
            PlayerKey::Form => s.stats.form = Some(byte(value)),
            PlayerKey::InjuryResistance => s.stats.injury_resistance = Some(byte(value)),
            PlayerKey::Star => s.stats.star = Some(byte(value)),
            PlayerKey::PlayingAttitude => s.stats.playing_attitude = Some(byte(value)),
            PlayerKey::Registered => s.positions.registered = Some(byte(value)),
            PlayerKey::StrongerFoot => s.positions.stronger_foot = Some(byte(value)),
            PlayerKey::StrongerHand => s.positions.stronger_hand = Some(byte(value)),
            PlayerKey::EditedPlayer => s.edit_flags.player = Some(value != 0),
            PlayerKey::EditedBasicSettings => s.edit_flags.basic_settings = Some(value != 0),
            PlayerKey::EditedRegisteredPosition => {
                s.edit_flags.registered_position = Some(value != 0)
            }
            PlayerKey::EditedPlayablePositions => {
                s.edit_flags.playable_positions = Some(value != 0)
            }
            PlayerKey::EditedAbilities => s.edit_flags.abilities = Some(value != 0),
            PlayerKey::EditedSkills => s.edit_flags.skills = Some(value != 0),
            PlayerKey::EditedPlayingStyle => s.edit_flags.playing_style = Some(value != 0),
            PlayerKey::EditedComStyles => s.edit_flags.com_styles = Some(value != 0),
            PlayerKey::EditedMotion => s.edit_flags.motion = Some(value != 0),
            PlayerKey::EditedBaseCopy => s.edit_flags.base_copy = Some(value != 0),
            PlayerKey::EditedFace => s.edit_flags.face = Some(value != 0),
            PlayerKey::EditedHair => s.edit_flags.hair = Some(value != 0),
            PlayerKey::EditedPhysique => s.edit_flags.physique = Some(value != 0),
            PlayerKey::EditedStrip => s.edit_flags.strip = Some(value != 0),
        }
        Ok(())
    }

    /// Whether `value` is one the key's kind can represent (the same range
    /// `parse` accepts).
    fn accepts(self, value: u32) -> bool {
        match self.spec().kind {
            Kind::Ability | Kind::Stored => i64::from(value) <= self.stored_max(),
            Kind::Bool => value <= 1,
            Kind::U16 => value <= u32::from(u16::MAX),
            Kind::U32 => true,
            Kind::Label(labels) => usize::try_from(value).is_ok_and(|i| i < labels.len()),
        }
    }

    /// The widest stored range of the key's field over every version's player
    /// and appearance record schemas: `0` to `(1 << bits) - 1`. Memoized — the
    /// scan runs once per `PlayerKey` over the whole process lifetime, not
    /// once per key per player per parse.
    fn stored_max(self) -> i64 {
        static MAXIMA: std::sync::LazyLock<Vec<i64>> = std::sync::LazyLock::new(|| {
            PlayerKey::ALL
                .iter()
                .map(|key| key.compute_stored_max())
                .collect()
        });
        let index = PlayerKey::ALL
            .iter()
            .position(|key| *key == self)
            .expect("ALL holds every key");
        MAXIMA[index]
    }

    /// The `stored_max` computation, run once per key into `MAXIMA`.
    fn compute_stored_max(self) -> i64 {
        (1i64 << crate::schema::widest_bit_width(self.spec().field)) - 1
    }

    /// The range/allowed text the kind reports in errors.
    fn range_text(self) -> String {
        match self.spec().kind {
            Kind::Ability | Kind::Stored => format!("0 to {}", self.stored_max()),
            Kind::Bool => "true, false".to_string(),
            Kind::U16 => "0 to 65535".to_string(),
            Kind::U32 => "0 to 4294967295".to_string(),
            Kind::Label(labels) => labels
                .iter()
                .map(|label| format!("\"{label}\""))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    /// The stored `u32` behind one present item, checked per the key's kind.
    pub(crate) fn parse(self, item: &Item, path: &str) -> Result<u32, TeamTomlError> {
        let integer = |expected: &'static str| -> Result<i64, TeamTomlError> {
            item.as_value()
                .and_then(|v| v.as_integer())
                .ok_or_else(|| TeamTomlError::WrongType {
                    key: path.to_string(),
                    expected,
                })
        };
        match self.spec().kind {
            Kind::Ability | Kind::Stored | Kind::U16 | Kind::U32 => {
                let value = integer("an integer")?;
                // The editor clamps abilities to 40-99 (the comment documents
                // that); a real save can hold anything the bits do (edited
                // saves carry 100), so interchange accepts the stored range.
                let max = match self.spec().kind {
                    Kind::Ability => self.stored_max(),
                    Kind::Stored => self.stored_max(),
                    Kind::U16 => 65535,
                    Kind::U32 => i64::from(u32::MAX),
                    Kind::Bool | Kind::Label(_) => unreachable!("matched above"),
                };
                let min = 0;
                if !(min..=max).contains(&value) {
                    return Err(TeamTomlError::OutOfRange {
                        key: path.to_string(),
                        value,
                        range: self.range_text(),
                    });
                }
                u32::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
                    key: path.to_string(),
                    value,
                    range: self.range_text(),
                })
            }
            Kind::Bool => item
                .as_value()
                .and_then(|v| v.as_bool())
                .map(u32::from)
                .ok_or_else(|| TeamTomlError::WrongType {
                    key: path.to_string(),
                    expected: "true or false",
                }),
            Kind::Label(labels) => {
                let text = item.as_value().and_then(|v| v.as_str()).ok_or_else(|| {
                    TeamTomlError::WrongType {
                        key: path.to_string(),
                        expected: "a string",
                    }
                })?;
                label_position(text, path, labels)
                    .map(|index| u32::try_from(index).expect("a label index fits u32"))
            }
        }
    }

    /// The TOML text of a stored value (`"left"`, `true`, `42`).
    pub(crate) fn text(self, value: u32) -> Result<String, TeamTomlError> {
        match self.spec().kind {
            Kind::Label(labels) => labels
                .get(usize::try_from(value).unwrap_or(usize::MAX))
                .map(|label| format!("\"{label}\""))
                .ok_or_else(|| TeamTomlError::OutOfRange {
                    key: self.spec().path.to_string(),
                    value: i64::from(value),
                    range: self.range_text(),
                }),
            Kind::Bool => Ok(if value == 0 { "false" } else { "true" }.to_string()),
            Kind::Ability | Kind::Stored | Kind::U16 | Kind::U32 => Ok(value.to_string()),
        }
    }

    /// The value an unset key's commented line shows.
    pub(crate) fn neutral(self) -> String {
        match self.spec().kind {
            Kind::Ability => "40".to_string(),
            Kind::Bool => "false".to_string(),
            Kind::Stored | Kind::U16 | Kind::U32 => "0".to_string(),
            Kind::Label(labels) => format!("\"{}\"", labels[0]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::open;

    /// An out-of-width value is `OutOfRange`, never a silent `None` — and the
    /// section is left untouched.
    #[test]
    fn set_section_refuses_a_value_the_kind_cannot_represent() {
        let mut section = PlayerSection::default();
        let err = PlayerKey::Age
            .set_section(&mut section, 300)
            .expect_err("age is a u8");
        assert!(matches!(
            err,
            TeamTomlError::OutOfRange {
                ref key,
                value: 300,
                ..
            } if key == "players.NN.age"
        ));
        assert_eq!(section.age, None);
    }

    /// `get` on a field the version does not store is `None`; the callers'
    /// `player_has` pre-gate is what keeps the `.ok()` honest.
    #[test]
    fn get_on_a_field_the_version_lacks_is_none() {
        let (file, _) = open(PesVersion::Pes15);
        let player = file.players().first().expect("a player");
        assert_eq!(PlayerKey::TightPossession.get(player), None);
        assert!(!schema_for(PesVersion::Pes15).player_has(PlayerField::TightPossession));
    }

    /// `accepts` bounds each kind at its own edges: a label key at the
    /// labels' length, a `Stored` key at the field's widest bit width.
    #[test]
    fn accepts_bounds_follow_the_kind() {
        let mut section = PlayerSection::default();
        assert!(matches!(
            PlayerKey::Registered.set_section(&mut section, 13),
            Err(TeamTomlError::OutOfRange { .. })
        ));
        PlayerKey::Registered
            .set_section(&mut section, 12)
            .expect("the last position");
        assert_eq!(section.positions.registered, Some(12));

        PlayerKey::Star
            .set_section(&mut section, 7)
            .expect("a 3-bit field's maximum");
        assert_eq!(section.stats.star, Some(7));
        let err = PlayerKey::Star
            .set_section(&mut section, 8)
            .expect_err("past the width");
        assert!(
            matches!(
                err,
                TeamTomlError::OutOfRange { ref range, .. } if range == "0 to 7"
            ),
            "{err:?}"
        );
        assert_eq!(section.stats.star, Some(7), "the refused write left it");
    }

    /// An ability past its stored range is refused with that range, the one
    /// the parser checks, not the game's 40 to 99: 20 parses, 130 does not.
    #[test]
    fn an_ability_past_its_stored_range_reports_the_stored_range() {
        let path = "players.01.stats.finishing";
        assert_eq!(
            PlayerKey::Finishing
                .parse(&toml_edit::value(20), path)
                .expect("below the game's range, inside the stored one"),
            20
        );
        let err = PlayerKey::Finishing
            .parse(&toml_edit::value(130), path)
            .expect_err("past seven bits");
        assert!(
            matches!(
                err,
                TeamTomlError::OutOfRange { ref range, value: 130, .. } if range == "0 to 127"
            ),
            "{err:?}"
        );
    }
}
