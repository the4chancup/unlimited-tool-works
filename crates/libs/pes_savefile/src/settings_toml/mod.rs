//! `PlayerSettings`: the model behind the export format's `settings.toml`
//! and `ingame_face.toml` files — every authorable aesthetic setting of a
//! player, as stored values
//! (`Option` throughout: `apply` writes a `None` key's default,
//! `apply_appearance` leaves the savefile field untouched), `keys`'s table
//! behind every setting, and `document`'s parse, emit and in-place update
//! of the TOML text. The codec reads and writes the model through
//! `from_player`/`apply`.

pub(crate) mod document;
/// The settings.toml key table: `SettingKey` and its `KeySpec` columns.
pub(crate) mod keys;

#[cfg(test)]
mod tests;

use pes_version::PesVersion;

use document::{dotted, range_text, toml_form};
use keys::Source;

use crate::codec::{CodecError, TextMisfit, text_max, text_misfit};
use crate::model::ingame_face::in_layout;
use crate::model::player::PlayerEntry;
use crate::schema::fields::{PlayerField, PlayerText};
use crate::schema::ingame_face::{FaceLayout, IngameFaceField};
use crate::schema::schema_for;

pub use keys::{KeySpec, KeyTable, Kind, SettingKey};

/// The player-name setting: `true` in TOML derives from the folder, a string
/// is written as is, absent leaves the savefile name untouched.
#[derive(Debug, Clone, PartialEq)]
pub enum NameSetting {
    /// Derive the name from the folder (`name = true`); `apply` does not
    /// resolve it — the caller (the Team compiler) does, since the folder
    /// rules are its.
    FromFolder,
    /// Write the string as is, colour codes included.
    Explicit(String),
}

/// One of the two player-name fields `PlayerSettings::apply` checks a text
/// against before writing it. `check` is that check alone: the compiler
/// drops a name `apply` would refuse, with this one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameField {
    /// `name`, the player's savefile name (UTF-8, colour codes included).
    Name,
    /// `shirt_name`, single-byte text.
    ShirtName,
}

impl NameField {
    /// The field's TOML key (`name`, `shirt_name`).
    pub fn key(self) -> &'static str {
        match self {
            NameField::Name => "name",
            NameField::ShirtName => "shirt_name",
        }
    }

    /// What `PlayerSettings::apply` checks of this field's text: `check_text`
    /// against the version's field capacity, `shirt_name` single-byte.
    pub fn check(self, text: &str, version: PesVersion) -> Result<(), SettingsError> {
        let (field, single_byte) = match self {
            NameField::Name => (PlayerText::Name, false),
            NameField::ShirtName => (PlayerText::ShirtName, true),
        };
        let max = text_max(schema_for(version).player.texts, field);
        check_text(self.key(), text, max, single_byte)
    }
}

/// The `[appearance]` table's stored values: every leaf `Option`;
/// `PlayerSettings::apply` writes a `None` key's default (`KeySpec::default`),
/// `apply_appearance` (Team TOML) leaves the savefile value alone. Physique
/// numbers are the stored value (game number + 7), motion numbers are stored
/// (1-based keys minus 1), label keys are the index into the key's label
/// list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppearanceSettings {
    /// Skin colour.
    pub skin_color: Option<u8>,
    /// Iris colour.
    pub iris_color: Option<u8>,
    /// `[appearance.physique]`.
    pub physique: PhysiqueSettings,
    /// `[appearance.strip]`.
    pub strip: StripSettings,
    /// `[appearance.motion]`.
    pub motion: MotionSettings,
    /// `[appearance.face]`.
    pub face: FaceSettings,
    /// `[appearance.hair]`.
    pub hair: HairSettings,
}

/// The `[appearance.physique]` table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhysiqueSettings {
    /// Neck length, stored (game number + 7).
    pub neck_length: Option<u8>,
    /// Neck size, stored.
    pub neck_size: Option<u8>,
    /// Shoulder height, stored.
    pub shoulder_height: Option<u8>,
    /// Shoulder width, stored.
    pub shoulder_width: Option<u8>,
    /// Chest, stored.
    pub chest: Option<u8>,
    /// Waist, stored.
    pub waist: Option<u8>,
    /// Arm size, stored.
    pub arm_size: Option<u8>,
    /// Arm length, stored.
    pub arm_length: Option<u8>,
    /// Thigh, stored.
    pub thigh: Option<u8>,
    /// Calf, stored.
    pub calf: Option<u8>,
    /// Leg length, stored.
    pub leg_length: Option<u8>,
    /// Head length, stored.
    pub head_length: Option<u8>,
    /// Head width, stored.
    pub head_width: Option<u8>,
    /// Head depth, stored.
    pub head_depth: Option<u8>,
}

/// The `[appearance.strip]` table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StripSettings {
    /// Sleeves (label index).
    pub sleeves: Option<u8>,
    /// Inners (label index).
    pub inners: Option<u8>,
    /// Socks (label index).
    pub socks: Option<u8>,
    /// Undershorts (label index).
    pub undershorts: Option<u8>,
    /// Shirttail out.
    pub untucked: Option<bool>,
    /// Ankle taping.
    pub ankle_taping: Option<bool>,
    /// Wrist taping (label index).
    pub wrist_taping: Option<u8>,
    /// Left wrist tape colour.
    pub wrist_tape_color_left: Option<u8>,
    /// Right wrist tape colour.
    pub wrist_tape_color_right: Option<u8>,
    /// Spectacles style.
    pub spectacles: Option<u8>,
    /// Spectacles frame colour.
    pub spectacles_color: Option<u8>,
    /// Player (outfield) gloves.
    pub gloves: Option<bool>,
    /// Player gloves colour.
    pub gloves_color: Option<u8>,
}

/// The `[appearance.motion]` table, stored values (1-based keys minus 1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionSettings {
    /// Hunching while dribbling, stored.
    pub hunching_dribbling: Option<u8>,
    /// Hunching while running, stored.
    pub hunching_running: Option<u8>,
    /// Arm movement while dribbling, stored.
    pub arm_movement_dribbling: Option<u8>,
    /// Arm movement while running, stored.
    pub arm_movement_running: Option<u8>,
    /// Corner kick motion, stored.
    pub corner_kick: Option<u8>,
    /// Free kick motion, stored.
    pub free_kick: Option<u8>,
    /// Penalty kick motion, stored.
    pub penalty_kick: Option<u8>,
    /// Dribbling motion (PES 20+).
    pub dribbling: Option<u8>,
    /// First goal celebration.
    pub goal_celebration_1: Option<u8>,
    /// Second goal celebration.
    pub goal_celebration_2: Option<u8>,
}

impl AppearanceSettings {
    /// `self` with every appearance key `version` stores filled: a set key
    /// keeps its value, an absent one takes its `KeySpec` default, a key
    /// the version does not store (`dribbling` before PES 20, a `pes15_`
    /// key on PES 16 to 21) is `None`. The aesthetics patch carries a whole
    /// look, so a key the file leaves out is written at its default — the
    /// same values `PlayerSettings::apply`'s appearance write resolves.
    pub fn resolved(&self, version: PesVersion) -> AppearanceSettings {
        let mut appearance = self.clone();
        for key in SettingKey::ALL {
            let spec = key.spec();
            if spec.table == KeyTable::Top {
                continue;
            }
            let value = if held_on(key, version) {
                get_appearance(&appearance, key).or(spec.default)
            } else {
                None
            };
            set_appearance(&mut appearance, key, value);
        }
        appearance
    }
}

/// The `[appearance.face]` table: the face of the ingame-face run, its
/// feature types, sliders, eyebrows and facial hair (stored values). The
/// `pes15_` sliders and `pes15_eye_white` are stored on every version and
/// drawn by PES 15 alone; `pes15_nose_angle` is skipped on PES 16 to 21.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FaceSettings {
    /// Cheek type.
    pub cheek_type: Option<u8>,
    /// Forehead type.
    pub forehead_type: Option<u8>,
    /// Facial hair type.
    pub facial_hair_type: Option<u8>,
    /// Laughter lines type.
    pub laughter_lines_type: Option<u8>,
    /// Upper eyelid type.
    pub upper_eyelid_type: Option<u8>,
    /// Lower eyelid type.
    pub lower_eyelid_type: Option<u8>,
    /// Eyebrow type.
    pub eyebrow_type: Option<u8>,
    /// Neck line type.
    pub neck_line_type: Option<u8>,
    /// Nose type.
    pub nose_type: Option<u8>,
    /// Upper lip type.
    pub upper_lip_type: Option<u8>,
    /// Lower lip type.
    pub lower_lip_type: Option<u8>,
    /// Jawline, stored.
    pub jawline: Option<u8>,
    /// Jaw height, stored.
    pub jaw_height: Option<u8>,
    /// Cheekbones, stored.
    pub cheekbones: Option<u8>,
    /// Cheekbone height, stored.
    pub cheekbone_height: Option<u8>,
    /// Chin width, stored.
    pub chin_width: Option<u8>,
    /// Chin height, stored.
    pub chin_height: Option<u8>,
    /// Chin depth, stored.
    pub chin_depth: Option<u8>,
    /// Face size, stored.
    pub face_size: Option<u8>,
    /// Face height, stored.
    pub face_height: Option<u8>,
    /// Brow width, stored.
    pub brow_width: Option<u8>,
    /// Inner eyebrow height, stored.
    pub inner_eyebrow_height: Option<u8>,
    /// Eyebrow depth, stored.
    pub eyebrow_depth: Option<u8>,
    /// Temple width, stored.
    pub temple_width: Option<u8>,
    /// Outer eyebrow height, stored.
    pub outer_eyebrow_height: Option<u8>,
    /// Pupil size, stored.
    pub pupil_size: Option<u8>,
    /// Eye depth, stored.
    pub eye_depth: Option<u8>,
    /// Inner eye position, stored.
    pub inner_eye_position: Option<u8>,
    /// Inner eye height, stored.
    pub inner_eye_height: Option<u8>,
    /// Outer eye position, stored.
    pub outer_eye_position: Option<u8>,
    /// Eye corner height, stored.
    pub eye_corner_height: Option<u8>,
    /// Upper eyelid width inner, stored.
    pub upper_eyelid_width_inner: Option<u8>,
    /// Upper eyelid height inner, stored.
    pub upper_eyelid_height_inner: Option<u8>,
    /// Upper eyelid width outer, stored.
    pub upper_eyelid_width_outer: Option<u8>,
    /// Upper eyelid height outer, stored.
    pub upper_eyelid_height_outer: Option<u8>,
    /// Bottom eyelid height, stored.
    pub bottom_eyelid_height: Option<u8>,
    /// Horizontal eye position, stored.
    pub horizontal_eye_position: Option<u8>,
    /// Eye height, stored.
    pub eye_height: Option<u8>,
    /// Nose height, stored.
    pub nose_height: Option<u8>,
    /// Nose tip depth, stored.
    pub nose_tip_depth: Option<u8>,
    /// Nostril width, stored.
    pub nostril_width: Option<u8>,
    /// Nose depth, stored.
    pub nose_depth: Option<u8>,
    /// Nose width, stored.
    pub nose_width: Option<u8>,
    /// Mouth height, stored.
    pub mouth_height: Option<u8>,
    /// Mouth depth, stored.
    pub mouth_depth: Option<u8>,
    /// Lip size, stored.
    pub lip_size: Option<u8>,
    /// Mouth corner height, stored.
    pub mouth_corner_height: Option<u8>,
    /// Lip width, stored.
    pub lip_width: Option<u8>,
    /// Ear length, stored.
    pub ear_length: Option<u8>,
    /// Ear width, stored.
    pub ear_width: Option<u8>,
    /// Ear angle, stored.
    pub ear_angle: Option<u8>,
    /// Pes 15 jaw depth, stored.
    pub pes15_jaw_depth: Option<u8>,
    /// Pes 15 cheekbone depth, stored.
    pub pes15_cheekbone_depth: Option<u8>,
    /// Pes 15 mouth edge position, stored.
    pub pes15_mouth_edge_position: Option<u8>,
    /// Pes 15 eyebrow depth, stored.
    pub pes15_eyebrow_depth: Option<u8>,
    /// Pes 15 bottom eyelid position, stored.
    pub pes15_bottom_eyelid_position: Option<u8>,
    /// Pes 15 nasal bridge position, stored.
    pub pes15_nasal_bridge_position: Option<u8>,
    /// Pes 15 nasal root height, stored.
    pub pes15_nasal_root_height: Option<u8>,
    /// Pes 15 nose angle, stored.
    pub pes15_nose_angle: Option<u8>,
    /// Eyebrow color red, stored.
    pub eyebrow_color_red: Option<u8>,
    /// Eyebrow color green, stored.
    pub eyebrow_color_green: Option<u8>,
    /// Eyebrow color blue, stored.
    pub eyebrow_color_blue: Option<u8>,
    /// Eyebrow style, stored.
    pub eyebrow_style: Option<u8>,
    /// Eyebrow density, stored.
    pub eyebrow_density: Option<u8>,
    /// Eyebrow thickness, stored.
    pub eyebrow_thickness: Option<u8>,
    /// Facial hair color red, stored.
    pub facial_hair_color_red: Option<u8>,
    /// Facial hair color green, stored.
    pub facial_hair_color_green: Option<u8>,
    /// Facial hair color blue, stored.
    pub facial_hair_color_blue: Option<u8>,
    /// Facial hair thickness, stored.
    pub facial_hair_thickness: Option<u8>,
    /// Pes 15 eye white, stored.
    pub pes15_eye_white: Option<u8>,
}

/// The `[appearance.hair]` table: the in-game hair of the ingame-face run
/// (stored values; the PES 16 to 21 keys and the `pes15_` ones are skipped on
/// the other layout).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HairSettings {
    /// Color red, stored.
    pub color_red: Option<u8>,
    /// Color green, stored.
    pub color_green: Option<u8>,
    /// Color blue, stored.
    pub color_blue: Option<u8>,
    /// Color preset, stored.
    pub color_preset: Option<u8>,
    /// Hair accessory worn.
    pub accessory: Option<bool>,
    /// Accessory color, stored.
    pub accessory_color: Option<u8>,
    /// Style, stored.
    pub style: Option<u8>,
    /// Cropped style, stored.
    pub cropped_style: Option<u8>,
    /// Length, stored.
    pub length: Option<u8>,
    /// Wave level, stored.
    pub wave_level: Option<u8>,
    /// Variation, stored.
    pub variation: Option<u8>,
    /// Front style, stored.
    pub front_style: Option<u8>,
    /// Parted, stored.
    pub parted: Option<u8>,
    /// Side back style, stored.
    pub side_back_style: Option<u8>,
    /// Hairline, stored.
    pub hairline: Option<u8>,
    /// Forehead width, stored.
    pub forehead_width: Option<u8>,
    /// Pes 15 length, stored.
    pub pes15_length: Option<u8>,
    /// Pes 15 type, stored.
    pub pes15_type: Option<u8>,
    /// Pes 15 variation, stored.
    pub pes15_variation: Option<u8>,
}

/// A settings.toml's player: the two names, the two stock-model IDs and the
/// appearance tables. The IDs are top-level keys, outside
/// `AppearanceSettings`, because Team TOML embeds the appearance tables
/// beside its own player-level full-range IDs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerSettings {
    /// The name setting; `None` = never write a name.
    pub name: Option<NameSetting>,
    /// The shirt name setting; `None` = never write a shirt name.
    pub shirt_name: Option<String>,
    /// A stock boots model ID (0 to 100); `None`/`""` = default.
    pub boots_id: Option<u8>,
    /// A stock gloves model ID (0 to 100; 0 = normal hands); `None`/`""` =
    /// default.
    pub gloves_id: Option<u8>,
    /// The `[appearance]` tables.
    pub appearance: AppearanceSettings,
}

/// A player's `ingame_face.toml`: the in-game face's keys, each `None` when
/// absent.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IngameFaceSettings {
    /// Iris colour.
    pub iris_color: Option<u8>,
    /// `[face]`.
    pub face: FaceSettings,
    /// `[hair]`.
    pub hair: HairSettings,
}

/// One `settings.toml` value `PlayerSettings::narrow_to` reset to its key's
/// default for the compile target: what the Team compiler's
/// `settings_value_out_of_range` reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrowedValue {
    /// The key's dotted TOML path in the file that holds it
    /// (`appearance.motion.dribbling` in `settings.toml`,
    /// `face.facial_hair_type` in `ingame_face.toml`; a top-level key is just
    /// its name).
    pub key: String,
    /// The value as the file wrote it: the TOML number (a one-based
    /// motion's stored 4 reports its shown 5).
    pub value: i64,
    /// The version's range as the file would write it ("1 to 3").
    pub range: String,
}

/// A stored value `PlayerSettings::from_player_skipping` left unset: its key's
/// kind (the widest range over every version) cannot represent it, as a PES 16
/// cup player's `goal_celebration_2` 183.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedValue {
    /// The key's dotted TOML path, as `NarrowedValue::key`.
    pub key: String,
    /// The stored value.
    pub value: i64,
    /// The key's widest range as the file would write it ("0 to 162").
    pub range: String,
}

/// A `PlayerSettings` could not be produced or applied.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// A set key has no field in this player's game version.
    #[error("{key}: this player's game version has no such setting")]
    NotInThisVersion {
        /// The TOML key.
        key: String,
    },
    /// A codec read or write failed (a player never read from a record has no
    /// ingame-face run to read or write through).
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// The text is not valid TOML. The file is not named: the finding's
    /// line names it.
    #[error("not valid TOML: {0}")]
    Toml(String),
    /// A key or table the settings schema does not know, by its dotted path
    /// (`appearance.boots_id`, `appearance.hair`, `stats`).
    #[error("{key}: unknown key")]
    UnknownKey {
        /// The dotted path.
        key: String,
    },
    /// The value is not the kind the key wants.
    #[error("{key}: expected {expected}")]
    WrongType {
        /// The dotted path (`name`, `appearance.strip.untucked`).
        key: String,
        /// What the key wants: "an integer", "a string", "true or false",
        /// "true or a string", "a table".
        expected: &'static str,
    },
    /// An integer outside the key's range.
    #[error("{key}: {value} is outside {range}")]
    OutOfRange {
        /// The dotted path.
        key: String,
        /// The value as written.
        value: i64,
        /// The range text ("-7 to 7", "1 to 10", "0 to 7").
        range: String,
    },
    /// A `name` or `shirt_name` string longer than the version's field holds.
    #[error("{key}: longer than this player's game version allows ({max})")]
    TextTooLong {
        /// The TOML key (`name`, `shirt_name`).
        key: String,
        /// The field's capacity: UTF-8 bytes for `name`, chars for the
        /// single-byte `shirt_name`.
        max: usize,
    },
    /// A string that is none of the key's labels.
    #[error("{key}: unknown value {label:?}, expected one of {allowed}")]
    UnknownLabel {
        /// The dotted path.
        key: String,
        /// The label as written.
        label: String,
        /// The labels joined by ", ", each quoted.
        allowed: String,
    },
}

/// The stored value behind an appearance key (`bool` as 0/1), `None` when
/// unset. Free-standing so `team_toml`'s per-key gating can read one key.
pub(crate) fn get_appearance(appearance: &AppearanceSettings, key: SettingKey) -> Option<u8> {
    let a = appearance;
    match key {
        // Top-level keys live on `PlayerSettings`, not in the appearance
        // tables; the appearance walks never reach them (`PlayerSettings`
        // methods match them out first).
        SettingKey::BootsId | SettingKey::GlovesId => None,
        SettingKey::SkinColor => a.skin_color,
        SettingKey::IrisColor => a.iris_color,
        SettingKey::NeckLength => a.physique.neck_length,
        SettingKey::NeckSize => a.physique.neck_size,
        SettingKey::ShoulderHeight => a.physique.shoulder_height,
        SettingKey::ShoulderWidth => a.physique.shoulder_width,
        SettingKey::Chest => a.physique.chest,
        SettingKey::Waist => a.physique.waist,
        SettingKey::ArmSize => a.physique.arm_size,
        SettingKey::ArmLength => a.physique.arm_length,
        SettingKey::Thigh => a.physique.thigh,
        SettingKey::Calf => a.physique.calf,
        SettingKey::LegLength => a.physique.leg_length,
        SettingKey::HeadLength => a.physique.head_length,
        SettingKey::HeadWidth => a.physique.head_width,
        SettingKey::HeadDepth => a.physique.head_depth,
        SettingKey::Sleeves => a.strip.sleeves,
        SettingKey::Inners => a.strip.inners,
        SettingKey::Socks => a.strip.socks,
        SettingKey::Undershorts => a.strip.undershorts,
        SettingKey::Untucked => a.strip.untucked.map(u8::from),
        SettingKey::AnkleTaping => a.strip.ankle_taping.map(u8::from),
        SettingKey::WristTaping => a.strip.wrist_taping,
        SettingKey::WristTapeColorLeft => a.strip.wrist_tape_color_left,
        SettingKey::WristTapeColorRight => a.strip.wrist_tape_color_right,
        SettingKey::Spectacles => a.strip.spectacles,
        SettingKey::SpectaclesColor => a.strip.spectacles_color,
        SettingKey::Gloves => a.strip.gloves.map(u8::from),
        SettingKey::GlovesColor => a.strip.gloves_color,
        SettingKey::HunchingDribbling => a.motion.hunching_dribbling,
        SettingKey::HunchingRunning => a.motion.hunching_running,
        SettingKey::ArmMovementDribbling => a.motion.arm_movement_dribbling,
        SettingKey::ArmMovementRunning => a.motion.arm_movement_running,
        SettingKey::CornerKick => a.motion.corner_kick,
        SettingKey::FreeKick => a.motion.free_kick,
        SettingKey::PenaltyKick => a.motion.penalty_kick,
        SettingKey::Dribbling => a.motion.dribbling,
        SettingKey::GoalCelebration1 => a.motion.goal_celebration_1,
        SettingKey::GoalCelebration2 => a.motion.goal_celebration_2,
        SettingKey::CheekType => a.face.cheek_type,
        SettingKey::ForeheadType => a.face.forehead_type,
        SettingKey::FacialHairType => a.face.facial_hair_type,
        SettingKey::LaughterLinesType => a.face.laughter_lines_type,
        SettingKey::UpperEyelidType => a.face.upper_eyelid_type,
        SettingKey::LowerEyelidType => a.face.lower_eyelid_type,
        SettingKey::EyebrowType => a.face.eyebrow_type,
        SettingKey::NeckLineType => a.face.neck_line_type,
        SettingKey::NoseType => a.face.nose_type,
        SettingKey::UpperLipType => a.face.upper_lip_type,
        SettingKey::LowerLipType => a.face.lower_lip_type,
        SettingKey::Jawline => a.face.jawline,
        SettingKey::JawHeight => a.face.jaw_height,
        SettingKey::Cheekbones => a.face.cheekbones,
        SettingKey::CheekboneHeight => a.face.cheekbone_height,
        SettingKey::ChinWidth => a.face.chin_width,
        SettingKey::ChinHeight => a.face.chin_height,
        SettingKey::ChinDepth => a.face.chin_depth,
        SettingKey::FaceSize => a.face.face_size,
        SettingKey::FaceHeight => a.face.face_height,
        SettingKey::BrowWidth => a.face.brow_width,
        SettingKey::InnerEyebrowHeight => a.face.inner_eyebrow_height,
        SettingKey::EyebrowDepth => a.face.eyebrow_depth,
        SettingKey::TempleWidth => a.face.temple_width,
        SettingKey::OuterEyebrowHeight => a.face.outer_eyebrow_height,
        SettingKey::PupilSize => a.face.pupil_size,
        SettingKey::EyeDepth => a.face.eye_depth,
        SettingKey::InnerEyePosition => a.face.inner_eye_position,
        SettingKey::InnerEyeHeight => a.face.inner_eye_height,
        SettingKey::OuterEyePosition => a.face.outer_eye_position,
        SettingKey::EyeCornerHeight => a.face.eye_corner_height,
        SettingKey::UpperEyelidWidthInner => a.face.upper_eyelid_width_inner,
        SettingKey::UpperEyelidHeightInner => a.face.upper_eyelid_height_inner,
        SettingKey::UpperEyelidWidthOuter => a.face.upper_eyelid_width_outer,
        SettingKey::UpperEyelidHeightOuter => a.face.upper_eyelid_height_outer,
        SettingKey::BottomEyelidHeight => a.face.bottom_eyelid_height,
        SettingKey::HorizontalEyePosition => a.face.horizontal_eye_position,
        SettingKey::EyeHeight => a.face.eye_height,
        SettingKey::NoseHeight => a.face.nose_height,
        SettingKey::NoseTipDepth => a.face.nose_tip_depth,
        SettingKey::NostrilWidth => a.face.nostril_width,
        SettingKey::NoseDepth => a.face.nose_depth,
        SettingKey::NoseWidth => a.face.nose_width,
        SettingKey::MouthHeight => a.face.mouth_height,
        SettingKey::MouthDepth => a.face.mouth_depth,
        SettingKey::LipSize => a.face.lip_size,
        SettingKey::MouthCornerHeight => a.face.mouth_corner_height,
        SettingKey::LipWidth => a.face.lip_width,
        SettingKey::EarLength => a.face.ear_length,
        SettingKey::EarWidth => a.face.ear_width,
        SettingKey::EarAngle => a.face.ear_angle,
        SettingKey::Pes15JawDepth => a.face.pes15_jaw_depth,
        SettingKey::Pes15CheekboneDepth => a.face.pes15_cheekbone_depth,
        SettingKey::Pes15MouthEdgePosition => a.face.pes15_mouth_edge_position,
        SettingKey::Pes15EyebrowDepth => a.face.pes15_eyebrow_depth,
        SettingKey::Pes15BottomEyelidPosition => a.face.pes15_bottom_eyelid_position,
        SettingKey::Pes15NasalBridgePosition => a.face.pes15_nasal_bridge_position,
        SettingKey::Pes15NasalRootHeight => a.face.pes15_nasal_root_height,
        SettingKey::Pes15NoseAngle => a.face.pes15_nose_angle,
        SettingKey::EyebrowColorRed => a.face.eyebrow_color_red,
        SettingKey::EyebrowColorGreen => a.face.eyebrow_color_green,
        SettingKey::EyebrowColorBlue => a.face.eyebrow_color_blue,
        SettingKey::EyebrowStyle => a.face.eyebrow_style,
        SettingKey::EyebrowDensity => a.face.eyebrow_density,
        SettingKey::EyebrowThickness => a.face.eyebrow_thickness,
        SettingKey::FacialHairColorRed => a.face.facial_hair_color_red,
        SettingKey::FacialHairColorGreen => a.face.facial_hair_color_green,
        SettingKey::FacialHairColorBlue => a.face.facial_hair_color_blue,
        SettingKey::FacialHairThickness => a.face.facial_hair_thickness,
        SettingKey::Pes15EyeWhite => a.face.pes15_eye_white,
        SettingKey::HairColorRed => a.hair.color_red,
        SettingKey::HairColorGreen => a.hair.color_green,
        SettingKey::HairColorBlue => a.hair.color_blue,
        SettingKey::HairColorPreset => a.hair.color_preset,
        SettingKey::HairAccessory => a.hair.accessory.map(u8::from),
        SettingKey::HairAccessoryColor => a.hair.accessory_color,
        SettingKey::HairStyle => a.hair.style,
        SettingKey::HairCroppedStyle => a.hair.cropped_style,
        SettingKey::HairLength => a.hair.length,
        SettingKey::HairWaveLevel => a.hair.wave_level,
        SettingKey::HairVariation => a.hair.variation,
        SettingKey::HairFrontStyle => a.hair.front_style,
        SettingKey::HairParted => a.hair.parted,
        SettingKey::HairSideBackStyle => a.hair.side_back_style,
        SettingKey::Hairline => a.hair.hairline,
        SettingKey::HairForeheadWidth => a.hair.forehead_width,
        SettingKey::Pes15HairLength => a.hair.pes15_length,
        SettingKey::Pes15HairType => a.hair.pes15_type,
        SettingKey::Pes15HairVariation => a.hair.pes15_variation,
    }
}

/// Sets an appearance key's field to `value` (`None` clears it). Free-standing
/// so `team_toml`'s cross-version pre-adjust can drop a gated key.
pub(crate) fn set_appearance(
    appearance: &mut AppearanceSettings,
    key: SettingKey,
    value: Option<u8>,
) {
    let a = appearance;
    match key {
        SettingKey::BootsId | SettingKey::GlovesId => {}
        SettingKey::SkinColor => a.skin_color = value,
        SettingKey::IrisColor => a.iris_color = value,
        SettingKey::NeckLength => a.physique.neck_length = value,
        SettingKey::NeckSize => a.physique.neck_size = value,
        SettingKey::ShoulderHeight => a.physique.shoulder_height = value,
        SettingKey::ShoulderWidth => a.physique.shoulder_width = value,
        SettingKey::Chest => a.physique.chest = value,
        SettingKey::Waist => a.physique.waist = value,
        SettingKey::ArmSize => a.physique.arm_size = value,
        SettingKey::ArmLength => a.physique.arm_length = value,
        SettingKey::Thigh => a.physique.thigh = value,
        SettingKey::Calf => a.physique.calf = value,
        SettingKey::LegLength => a.physique.leg_length = value,
        SettingKey::HeadLength => a.physique.head_length = value,
        SettingKey::HeadWidth => a.physique.head_width = value,
        SettingKey::HeadDepth => a.physique.head_depth = value,
        SettingKey::Sleeves => a.strip.sleeves = value,
        SettingKey::Inners => a.strip.inners = value,
        SettingKey::Socks => a.strip.socks = value,
        SettingKey::Undershorts => a.strip.undershorts = value,
        SettingKey::Untucked => a.strip.untucked = value.map(|v| v != 0),
        SettingKey::AnkleTaping => a.strip.ankle_taping = value.map(|v| v != 0),
        SettingKey::WristTaping => a.strip.wrist_taping = value,
        SettingKey::WristTapeColorLeft => a.strip.wrist_tape_color_left = value,
        SettingKey::WristTapeColorRight => a.strip.wrist_tape_color_right = value,
        SettingKey::Spectacles => a.strip.spectacles = value,
        SettingKey::SpectaclesColor => a.strip.spectacles_color = value,
        SettingKey::Gloves => a.strip.gloves = value.map(|v| v != 0),
        SettingKey::GlovesColor => a.strip.gloves_color = value,
        SettingKey::HunchingDribbling => a.motion.hunching_dribbling = value,
        SettingKey::HunchingRunning => a.motion.hunching_running = value,
        SettingKey::ArmMovementDribbling => a.motion.arm_movement_dribbling = value,
        SettingKey::ArmMovementRunning => a.motion.arm_movement_running = value,
        SettingKey::CornerKick => a.motion.corner_kick = value,
        SettingKey::FreeKick => a.motion.free_kick = value,
        SettingKey::PenaltyKick => a.motion.penalty_kick = value,
        SettingKey::Dribbling => a.motion.dribbling = value,
        SettingKey::GoalCelebration1 => a.motion.goal_celebration_1 = value,
        SettingKey::GoalCelebration2 => a.motion.goal_celebration_2 = value,
        SettingKey::CheekType => a.face.cheek_type = value,
        SettingKey::ForeheadType => a.face.forehead_type = value,
        SettingKey::FacialHairType => a.face.facial_hair_type = value,
        SettingKey::LaughterLinesType => a.face.laughter_lines_type = value,
        SettingKey::UpperEyelidType => a.face.upper_eyelid_type = value,
        SettingKey::LowerEyelidType => a.face.lower_eyelid_type = value,
        SettingKey::EyebrowType => a.face.eyebrow_type = value,
        SettingKey::NeckLineType => a.face.neck_line_type = value,
        SettingKey::NoseType => a.face.nose_type = value,
        SettingKey::UpperLipType => a.face.upper_lip_type = value,
        SettingKey::LowerLipType => a.face.lower_lip_type = value,
        SettingKey::Jawline => a.face.jawline = value,
        SettingKey::JawHeight => a.face.jaw_height = value,
        SettingKey::Cheekbones => a.face.cheekbones = value,
        SettingKey::CheekboneHeight => a.face.cheekbone_height = value,
        SettingKey::ChinWidth => a.face.chin_width = value,
        SettingKey::ChinHeight => a.face.chin_height = value,
        SettingKey::ChinDepth => a.face.chin_depth = value,
        SettingKey::FaceSize => a.face.face_size = value,
        SettingKey::FaceHeight => a.face.face_height = value,
        SettingKey::BrowWidth => a.face.brow_width = value,
        SettingKey::InnerEyebrowHeight => a.face.inner_eyebrow_height = value,
        SettingKey::EyebrowDepth => a.face.eyebrow_depth = value,
        SettingKey::TempleWidth => a.face.temple_width = value,
        SettingKey::OuterEyebrowHeight => a.face.outer_eyebrow_height = value,
        SettingKey::PupilSize => a.face.pupil_size = value,
        SettingKey::EyeDepth => a.face.eye_depth = value,
        SettingKey::InnerEyePosition => a.face.inner_eye_position = value,
        SettingKey::InnerEyeHeight => a.face.inner_eye_height = value,
        SettingKey::OuterEyePosition => a.face.outer_eye_position = value,
        SettingKey::EyeCornerHeight => a.face.eye_corner_height = value,
        SettingKey::UpperEyelidWidthInner => a.face.upper_eyelid_width_inner = value,
        SettingKey::UpperEyelidHeightInner => a.face.upper_eyelid_height_inner = value,
        SettingKey::UpperEyelidWidthOuter => a.face.upper_eyelid_width_outer = value,
        SettingKey::UpperEyelidHeightOuter => a.face.upper_eyelid_height_outer = value,
        SettingKey::BottomEyelidHeight => a.face.bottom_eyelid_height = value,
        SettingKey::HorizontalEyePosition => a.face.horizontal_eye_position = value,
        SettingKey::EyeHeight => a.face.eye_height = value,
        SettingKey::NoseHeight => a.face.nose_height = value,
        SettingKey::NoseTipDepth => a.face.nose_tip_depth = value,
        SettingKey::NostrilWidth => a.face.nostril_width = value,
        SettingKey::NoseDepth => a.face.nose_depth = value,
        SettingKey::NoseWidth => a.face.nose_width = value,
        SettingKey::MouthHeight => a.face.mouth_height = value,
        SettingKey::MouthDepth => a.face.mouth_depth = value,
        SettingKey::LipSize => a.face.lip_size = value,
        SettingKey::MouthCornerHeight => a.face.mouth_corner_height = value,
        SettingKey::LipWidth => a.face.lip_width = value,
        SettingKey::EarLength => a.face.ear_length = value,
        SettingKey::EarWidth => a.face.ear_width = value,
        SettingKey::EarAngle => a.face.ear_angle = value,
        SettingKey::Pes15JawDepth => a.face.pes15_jaw_depth = value,
        SettingKey::Pes15CheekboneDepth => a.face.pes15_cheekbone_depth = value,
        SettingKey::Pes15MouthEdgePosition => a.face.pes15_mouth_edge_position = value,
        SettingKey::Pes15EyebrowDepth => a.face.pes15_eyebrow_depth = value,
        SettingKey::Pes15BottomEyelidPosition => a.face.pes15_bottom_eyelid_position = value,
        SettingKey::Pes15NasalBridgePosition => a.face.pes15_nasal_bridge_position = value,
        SettingKey::Pes15NasalRootHeight => a.face.pes15_nasal_root_height = value,
        SettingKey::Pes15NoseAngle => a.face.pes15_nose_angle = value,
        SettingKey::EyebrowColorRed => a.face.eyebrow_color_red = value,
        SettingKey::EyebrowColorGreen => a.face.eyebrow_color_green = value,
        SettingKey::EyebrowColorBlue => a.face.eyebrow_color_blue = value,
        SettingKey::EyebrowStyle => a.face.eyebrow_style = value,
        SettingKey::EyebrowDensity => a.face.eyebrow_density = value,
        SettingKey::EyebrowThickness => a.face.eyebrow_thickness = value,
        SettingKey::FacialHairColorRed => a.face.facial_hair_color_red = value,
        SettingKey::FacialHairColorGreen => a.face.facial_hair_color_green = value,
        SettingKey::FacialHairColorBlue => a.face.facial_hair_color_blue = value,
        SettingKey::FacialHairThickness => a.face.facial_hair_thickness = value,
        SettingKey::Pes15EyeWhite => a.face.pes15_eye_white = value,
        SettingKey::HairColorRed => a.hair.color_red = value,
        SettingKey::HairColorGreen => a.hair.color_green = value,
        SettingKey::HairColorBlue => a.hair.color_blue = value,
        SettingKey::HairColorPreset => a.hair.color_preset = value,
        SettingKey::HairAccessory => a.hair.accessory = value.map(|v| v != 0),
        SettingKey::HairAccessoryColor => a.hair.accessory_color = value,
        SettingKey::HairStyle => a.hair.style = value,
        SettingKey::HairCroppedStyle => a.hair.cropped_style = value,
        SettingKey::HairLength => a.hair.length = value,
        SettingKey::HairWaveLevel => a.hair.wave_level = value,
        SettingKey::HairVariation => a.hair.variation = value,
        SettingKey::HairFrontStyle => a.hair.front_style = value,
        SettingKey::HairParted => a.hair.parted = value,
        SettingKey::HairSideBackStyle => a.hair.side_back_style = value,
        SettingKey::Hairline => a.hair.hairline = value,
        SettingKey::HairForeheadWidth => a.hair.forehead_width = value,
        SettingKey::Pes15HairLength => a.hair.pes15_length = value,
        SettingKey::Pes15HairType => a.hair.pes15_type = value,
        SettingKey::Pes15HairVariation => a.hair.pes15_variation = value,
    }
}

/// The stored value behind `key` on `player`, `None` for a key the player's
/// version does not store (`held_on`). `CodecError` from the reads
/// propagates. The stock-model keys read `Some` only for the authored band 1
/// to 100: 0 is the game's default (emitted as `""`) and an ID above 100 is
/// custom content a folder owns, not a settings value.
fn read(player: &PlayerEntry, key: SettingKey) -> Result<Option<u8>, SettingsError> {
    let face = &player.appearance.ingame_face;
    Ok(match key.source() {
        Source::Player(PlayerField::BootsId) => stock_id(player.appearance.boots_id),
        Source::Player(PlayerField::GlovesId) => stock_id(player.appearance.gloves_id),
        Source::Player(PlayerField::DribblingMotion) => player.motion.dribbling,
        Source::Player(field) => {
            Some(u8::try_from(player.get(field)?).expect("an appearance or motion field is a byte"))
        }
        // The run, not a version, says which layout's keys the player has:
        // `get` refuses a field of the other layout.
        Source::Face(_) | Source::NoseAngle if !held_in(key, face.layout()) => None,
        Source::Face(IngameFaceField::NoseType) => Some(face.nose()?.0),
        Source::Face(field) => Some(face.get(field)?),
        Source::NoseAngle => Some(u8::try_from(face.nose()?.1 + 1).expect("an angle is -1 to 1")),
    })
}

/// Whether a run of `layout` holds `key`'s face source: its field's layout
/// is `None` or `layout`, and PES 15's nose angle is PES 15's alone. `true`
/// for a key of another source.
fn held_in(key: SettingKey, layout: FaceLayout) -> bool {
    match key.source() {
        Source::Player(_) => true,
        Source::Face(field) => in_layout(field.layout(), layout),
        Source::NoseAngle => layout == FaceLayout::Pes15,
    }
}

/// Whether `version` stores `key`: a player field its schema has, a face
/// source its run's layout holds. A settings apply skips a key it does not
/// store (`dribbling` before PES 20, a `pes15_` key on PES 16 to 21), set or
/// not, so an export written for one version compiles for another.
pub(crate) fn held_on(key: SettingKey, version: PesVersion) -> bool {
    match key.source() {
        Source::Player(field) => schema_for(version).player_has(field),
        Source::Face(_) | Source::NoseAngle => held_in(key, FaceLayout::of(version)),
    }
}

/// `Some` for a stored boots/gloves ID inside the authored stock band,
/// `None` for the game's default 0 and for a custom (folder-owned) ID.
fn stock_id(id: u32) -> Option<u8> {
    u8::try_from(id).ok().filter(|id| (1..=100).contains(id))
}

/// The player's appearance settings for Team TOML: every key `Some` with its
/// raw stored value — a save can hold values past the editor's range (the
/// field's bits are wider than the selectable range), and the interchange's
/// stored-ranges mode carries them verbatim.
pub(crate) fn appearance_from(player: &PlayerEntry) -> Result<AppearanceSettings, SettingsError> {
    let mut appearance = AppearanceSettings::default();
    for key in SettingKey::ALL {
        if let Some(value) = read(player, key)? {
            set_appearance(&mut appearance, key, Some(value));
        }
    }
    Ok(appearance)
}

impl PlayerSettings {
    /// The stored value behind a key (`bool` as 0/1), `None` when unset.
    pub fn get(&self, key: SettingKey) -> Option<u8> {
        match key {
            SettingKey::BootsId => self.boots_id,
            SettingKey::GlovesId => self.gloves_id,
            _ => get_appearance(&self.appearance, key),
        }
    }

    /// Sets a stored value; `OutOfRange` when the key's kind cannot represent
    /// it, so a `PlayerSettings` built through `set` is always one the
    /// emitters can write.
    pub fn set(&mut self, key: SettingKey, value: u8) -> Result<(), SettingsError> {
        let spec = key.spec();
        if !spec.kind.accepts_stored(value) {
            return Err(SettingsError::OutOfRange {
                key: dotted(key),
                value: i64::from(value),
                range: range_text(spec.kind),
            });
        }
        match key {
            SettingKey::BootsId => self.boots_id = Some(value),
            SettingKey::GlovesId => self.gloves_id = Some(value),
            _ => set_appearance(&mut self.appearance, key, Some(value)),
        }
        Ok(())
    }

    /// Every key `Some` from the player; `name` is `Explicit` of the raw name,
    /// colour codes included. A stored value the key's kind cannot represent
    /// (a hostile save's out-of-range bits) is `OutOfRange` — the first one
    /// `from_player_skipping` collects, back as the error.
    pub fn from_player(player: &PlayerEntry) -> Result<Self, SettingsError> {
        let (settings, refused) = Self::from_player_skipping(player)?;
        if let Some(RefusedValue { key, value, range }) = refused.into_iter().next() {
            return Err(SettingsError::OutOfRange { key, value, range });
        }
        Ok(settings)
    }

    /// `from_player`'s loop with the refused values collected instead of
    /// failing the player: a `set` returning `OutOfRange` leaves the key
    /// unset and pushes a `RefusedValue` (in the key table's order), any
    /// other error propagates. `from_player` is this with the first refused
    /// value back as `OutOfRange`.
    pub fn from_player_skipping(
        player: &PlayerEntry,
    ) -> Result<(Self, Vec<RefusedValue>), SettingsError> {
        let mut settings = PlayerSettings {
            name: Some(NameSetting::Explicit(player.name.clone())),
            shirt_name: Some(player.shirt_name.clone()),
            ..PlayerSettings::default()
        };
        let mut refused = Vec::new();
        for key in SettingKey::ALL {
            let Some(value) = read(player, key)? else {
                continue;
            };
            match settings.set(key, value) {
                Ok(()) => {}
                Err(SettingsError::OutOfRange { key, value, range }) => {
                    refused.push(RefusedValue { key, value, range });
                }
                Err(error) => return Err(error),
            }
        }
        Ok((settings, refused))
    }

    /// Writes the settings onto the player, all or nothing: on `Err` the
    /// player is unchanged. `version` names the compile target: the name and
    /// shirt-name fields differ between versions, so both strings are checked
    /// against that version's field before anything is written (a misfit is
    /// `WrongType` or `TextTooLong`). `NameSetting::FromFolder` is not
    /// applied: the caller (the Team compiler) resolves it to `Explicit`
    /// first, since the folder rules are its.
    ///
    /// Every appearance key writes: a `Some` its value, a `None` the key's
    /// default (`KeySpec::default`, the template's shown value), so a compiled
    /// player's look is his file and nothing else. A key the target version
    /// does not store (`held_on`) is skipped, set or not. The two stock IDs
    /// are the exceptions: a `Some` writes the ID, a `None` leaves the stored
    /// ID for the compiler to resolve. A `Some` the key's kind cannot
    /// represent is `OutOfRange`, checked before any write. The run's
    /// measured-unused bits are then written at the games' values
    /// (`IngameFace::write_unused`).
    pub fn apply(
        &self,
        player: &mut PlayerEntry,
        version: PesVersion,
    ) -> Result<(), SettingsError> {
        for key in SettingKey::ALL {
            if let Some(value) = self.get(key) {
                let spec = key.spec();
                if !spec.kind.accepts_stored(value) {
                    return Err(SettingsError::OutOfRange {
                        key: dotted(key),
                        value: i64::from(value),
                        range: range_text(spec.kind),
                    });
                }
            }
        }
        if let Some(NameSetting::Explicit(name)) = &self.name {
            NameField::Name.check(name, version)?;
        }
        if let Some(shirt_name) = &self.shirt_name {
            NameField::ShirtName.check(shirt_name, version)?;
        }
        let mut next = player.clone();
        if let Some(NameSetting::Explicit(name)) = &self.name {
            next.name = name.clone();
        }
        if let Some(shirt_name) = &self.shirt_name {
            next.shirt_name = shirt_name.clone();
        }
        if let Some(id) = self.boots_id {
            next.appearance.boots_id = u32::from(id);
        }
        if let Some(id) = self.gloves_id {
            next.appearance.gloves_id = u32::from(id);
        }
        self.write_appearance_defaults(&mut next, version)?;
        *player = next;
        Ok(())
    }

    /// `apply`'s appearance write, without its TOML-surface checks (the kind
    /// ranges, the name lengths): every appearance key's held value or its
    /// `KeySpec` default onto `next`, the run's unused bits after, a key
    /// the target version does not store dropped. A stored value past a key's
    /// range but inside its field's (a save's own data the file could never
    /// hold) is written as is — the field's bits, `write_player`'s and
    /// `face.set`'s bound, are the check.
    fn write_appearance_defaults(
        &self,
        next: &mut PlayerEntry,
        version: PesVersion,
    ) -> Result<(), SettingsError> {
        write_appearance(&self.appearance.resolved(version), next)?;
        next.appearance.ingame_face.write_unused();
        Ok(())
    }

    /// Narrows the settings to the compile target `version`: every set value
    /// inside its key's widest range but outside the version's own
    /// (`SettingKey::version_kind`) is reset to the key's `KeySpec` default
    /// (a face type 0, a motion the template's value, a custom skin the
    /// target lacks as 1), and one `NarrowedValue` per reset key is
    /// returned, in the key table's order, for the Team compiler's
    /// `settings_value_out_of_range`. A key the version does not store is
    /// left alone: the write drops it (`held_on`), with no finding. A stored
    /// value outside the key's widest range too (a directly written field,
    /// never a parsed file) is not this method's to touch: `apply`'s checks
    /// still refuse it.
    pub fn narrow_to(&mut self, version: PesVersion) -> Vec<NarrowedValue> {
        let mut narrowed = Vec::new();
        for key in SettingKey::ALL {
            let Some(stored) = self.get(key) else {
                continue;
            };
            let spec = key.spec();
            let Some(kind) = key.version_kind(version) else {
                continue;
            };
            if !spec.kind.accepts_stored(stored) || kind.accepts_stored(stored) {
                continue;
            }
            narrowed.push(NarrowedValue {
                key: dotted(key),
                value: toml_form(key, stored, false)
                    .expect("inside the widest range, checked above")
                    .as_integer()
                    .expect("the kinds a version narrows all write a TOML integer"),
                range: range_text(kind),
            });
            self.set(key, spec.default.expect("a key that narrows has one"))
                .expect("a key's default is inside its widest range");
        }
        narrowed
    }

    /// Applies an `fpc` preset's strip keys and skin, mapped as
    /// `ops::fpc::apply` maps them onto a savefile player, the preset's boots
    /// and gloves IDs aside: an export resolves those itself, from its own
    /// models, a numeric key or its FPC marker. Returns the dotted key
    /// of each the file had set to another value, for the Team compiler's
    /// `fpc_strip_conflict`: `skin_color` only when a `SkinColor::Preset`
    /// preset resets an authored custom skin (7) to 1; any other skin it
    /// leaves alone. `SkinColor::Custom` writes 7 as `apply` does; no marker
    /// carries it, so the compiler never reaches that arm.
    pub fn apply_fpc_preset(&mut self, appearance: &fpc::Appearance) -> Vec<String> {
        let mut overridden = Vec::new();
        // The key table's order: `skin_color` before the strip keys.
        match appearance.skin_color {
            fpc::SkinColor::Preset => {
                if self.appearance.skin_color == Some(7) {
                    overridden.push(dotted(SettingKey::SkinColor));
                    self.appearance.skin_color = Some(1);
                }
            }
            fpc::SkinColor::Custom => {
                if self.appearance.skin_color.is_some_and(|set| set != 7) {
                    overridden.push(dotted(SettingKey::SkinColor));
                }
                self.appearance.skin_color = Some(7);
            }
        }
        for (key, value) in [
            (
                SettingKey::Sleeves,
                crate::ops::fpc::sleeves_value(appearance.sleeves),
            ),
            (
                SettingKey::Socks,
                crate::ops::fpc::socks_value(appearance.socks),
            ),
            (
                SettingKey::Untucked,
                u8::from(crate::ops::fpc::untucked_value(appearance.tuck)),
            ),
        ] {
            if self.get(key).is_some_and(|set| set != value) {
                overridden.push(dotted(key));
            }
            set_appearance(&mut self.appearance, key, Some(value));
        }
        overridden
    }

    /// The settings' strip settings as `fpc::check`'s `StripStyle`, the same
    /// mapping `ops::fpc::strip_style` makes of a savefile player's, an
    /// absent key taking its `KeySpec` default as the appearance write
    /// applies it. `gloves_id` is the ID the compile's resolution gives the
    /// player: the file's `gloves_id` key is only one of its sources (the
    /// folder's models and the FPC marker are the others).
    pub fn strip_style(&self, gloves_id: u16) -> fpc::StripStyle {
        let value = |key: SettingKey| {
            self.get(key)
                .or(key.spec().default)
                .expect("the strip keys all have a default")
        };
        fpc::StripStyle {
            inners_worn: value(SettingKey::Inners) != 0,
            undershorts_worn: value(SettingKey::Undershorts) != 0,
            wrist_taping: value(SettingKey::WristTaping) != 0,
            ankle_taping: value(SettingKey::AnkleTaping) != 0,
            gloves_checkbox: value(SettingKey::Gloves) != 0,
            gloves_id,
            skin_color: if value(SettingKey::SkinColor) == 7 {
                fpc::SkinColor::Custom
            } else {
                fpc::SkinColor::Preset
            },
        }
    }

    /// The in-game face's keys, as `ingame_face.toml` holds them.
    pub fn ingame_face(&self) -> IngameFaceSettings {
        IngameFaceSettings {
            iris_color: self.appearance.iris_color,
            face: self.appearance.face.clone(),
            hair: self.appearance.hair.clone(),
        }
    }

    /// The in-game face's keys set from `face`, an absent key unset.
    pub fn set_ingame_face(&mut self, face: IngameFaceSettings) {
        self.appearance.iris_color = face.iris_color;
        self.appearance.face = face.face;
        self.appearance.hair = face.hair;
    }
}

/// Maps `text_misfit`'s verdict for the `key` text onto this file's errors.
/// `pub(crate)` for the aesthetics patch, which checks its names with the
/// same rule.
pub(crate) fn check_text(
    key: &str,
    text: &str,
    max: usize,
    single_byte: bool,
) -> Result<(), SettingsError> {
    match text_misfit(text, max, single_byte) {
        None => Ok(()),
        Some(TextMisfit::Nul) => Err(SettingsError::WrongType {
            key: key.to_string(),
            expected: "a string without NUL",
        }),
        Some(TextMisfit::BeyondSingleByte) => Err(SettingsError::WrongType {
            key: key.to_string(),
            expected: "a string of chars at most U+00FF",
        }),
        Some(TextMisfit::TooLong { max }) => Err(SettingsError::TextTooLong {
            key: key.to_string(),
            max,
        }),
    }
}

/// The write half of `PlayerSettings::apply`, without the kind-range
/// validation: `team_toml` applies a document's stored values as written.
/// Still all-or-nothing and still `NotInThisVersion` for a `Some` on a field
/// the player's version lacks.
pub(crate) fn apply_appearance(
    appearance: &AppearanceSettings,
    player: &mut PlayerEntry,
) -> Result<(), SettingsError> {
    let mut next = player.clone();
    write_appearance(appearance, &mut next)?;
    *player = next;
    Ok(())
}

/// Writes every `Some` key onto `next` (called on a scratch clone).
fn write_appearance(
    appearance: &AppearanceSettings,
    next: &mut PlayerEntry,
) -> Result<(), SettingsError> {
    let not_held = |key: SettingKey| SettingsError::NotInThisVersion {
        key: key.spec().name.to_string(),
    };
    for key in SettingKey::ALL {
        let Some(value) = get_appearance(appearance, key) else {
            continue;
        };
        let face = &mut next.appearance.ingame_face;
        match key.source() {
            // `get_appearance` already filtered these out.
            Source::Player(PlayerField::BootsId | PlayerField::GlovesId) => {}
            Source::Player(PlayerField::DribblingMotion) if next.motion.dribbling.is_none() => {
                return Err(not_held(key));
            }
            Source::Player(field) => next.set(field, u32::from(value))?,
            Source::Face(_) | Source::NoseAngle if !held_in(key, face.layout()) => {
                return Err(not_held(key));
            }
            // The nose keys share one field on PES 15: each keeps the other's
            // half (`IngameFace::nose`).
            Source::Face(IngameFaceField::NoseType) => face.set_nose(value, face.nose()?.1)?,
            Source::Face(field) => face.set(field, value)?,
            Source::NoseAngle => {
                // A stored angle past 2 (no parser produces one) stays out of
                // range, for `set_nose` to refuse.
                let angle = i8::try_from(value).unwrap_or(i8::MAX) - 1;
                face.set_nose(face.nose()?.0, angle)?;
            }
        }
    }
    Ok(())
}

/// Who owns a savefile field: authored through `settings.toml`, derived by the
/// compiler, or gameplay data outside the file's scope. Test-side only so far:
/// the completeness test is its consumer.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ownership {
    /// An authorable `settings.toml` setting.
    Settings,
    /// Written by Studio's own tools, never authored in the file: the
    /// compiler's model ids, edit flags and base copy, and the Save editor's
    /// strip of the appearance id.
    CompilerOwned,
    /// Gameplay data the file does not carry.
    Gameplay,
}

/// The ownership of one player-record field.
#[cfg(test)]
pub(crate) fn ownership(field: PlayerField) -> Ownership {
    match field {
        PlayerField::BaseCopyId
        | PlayerField::BaseCopy
        | PlayerField::EditedPlayer
        | PlayerField::EditedBasicSettings
        | PlayerField::EditedRegisteredPosition
        | PlayerField::EditedPlayablePositions
        | PlayerField::EditedAbilities
        | PlayerField::EditedSkills
        | PlayerField::EditedPlayingStyle
        | PlayerField::EditedComStyles
        | PlayerField::EditedMotion
        | PlayerField::EditedFace
        | PlayerField::EditedHair
        | PlayerField::EditedPhysique
        | PlayerField::EditedStrip
        | PlayerField::AppearanceId => Ownership::CompilerOwned,
        PlayerField::BootsId
        | PlayerField::GlovesId
        | PlayerField::NeckLength
        | PlayerField::NeckSize
        | PlayerField::ShoulderHeight
        | PlayerField::ShoulderWidth
        | PlayerField::Chest
        | PlayerField::Waist
        | PlayerField::ArmSize
        | PlayerField::ArmLength
        | PlayerField::Thigh
        | PlayerField::Calf
        | PlayerField::LegLength
        | PlayerField::HeadLength
        | PlayerField::HeadWidth
        | PlayerField::HeadDepth
        | PlayerField::WristTapeColorLeft
        | PlayerField::WristTapeColorRight
        | PlayerField::WristTaping
        | PlayerField::SpectaclesColor
        | PlayerField::SpectaclesStyle
        | PlayerField::Sleeves
        | PlayerField::Inners
        | PlayerField::Socks
        | PlayerField::Undershorts
        | PlayerField::Untucked
        | PlayerField::AnkleTaping
        | PlayerField::HunchingDribbling
        | PlayerField::HunchingRunning
        | PlayerField::ArmMovementDribbling
        | PlayerField::ArmMovementRunning
        | PlayerField::CornerKickMotion
        | PlayerField::FreeKickMotion
        | PlayerField::PenaltyKickMotion
        | PlayerField::DribblingMotion
        | PlayerField::GoalCelebration1
        | PlayerField::GoalCelebration2 => Ownership::Settings,
        PlayerField::Id
        | PlayerField::Nationality
        | PlayerField::Age
        | PlayerField::Height
        | PlayerField::Weight
        | PlayerField::AttackingProwess
        | PlayerField::DefensiveProwess
        | PlayerField::Goalkeeping
        | PlayerField::Dribbling
        | PlayerField::Finishing
        | PlayerField::LowPass
        | PlayerField::LoftedPass
        | PlayerField::Heading
        | PlayerField::Form
        | PlayerField::Swerve
        | PlayerField::Catching
        | PlayerField::Clearing
        | PlayerField::Coverage
        | PlayerField::Reflexes
        | PlayerField::InjuryResistance
        | PlayerField::BodyControl
        | PlayerField::PhysicalContact
        | PlayerField::KickingPower
        | PlayerField::ExplosivePower
        | PlayerField::RegisteredPosition
        | PlayerField::PlayingStyle
        | PlayerField::BallControl
        | PlayerField::BallWinning
        | PlayerField::WeakFootAccuracy
        | PlayerField::WeakFootUsage
        | PlayerField::Jump
        | PlayerField::PlaceKicking
        | PlayerField::Stamina
        | PlayerField::Speed
        | PlayerField::StrongerFoot
        | PlayerField::StrongerHand
        | PlayerField::Star
        | PlayerField::TightPossession
        | PlayerField::Aggression
        | PlayerField::PlayingAttitude
        | PlayerField::PlayablePosition(_)
        | PlayerField::ComStyle(_)
        | PlayerField::Skill(_) => Ownership::Gameplay,
    }
}

/// The ownership of one ingame-face field: every one is a setting, still an
/// exhaustive match so a new field fails to compile until classified.
#[cfg(test)]
pub(crate) fn face_ownership(field: IngameFaceField) -> Ownership {
    match field {
        IngameFaceField::PlayerGloves
        | IngameFaceField::PlayerGlovesColor
        | IngameFaceField::Jawline
        | IngameFaceField::JawHeight
        | IngameFaceField::Pes15JawDepth
        | IngameFaceField::Cheekbones
        | IngameFaceField::CheekboneHeight
        | IngameFaceField::Pes15CheekboneDepth
        | IngameFaceField::BrowWidth
        | IngameFaceField::InnerEyebrowHeight
        | IngameFaceField::EyebrowDepth
        | IngameFaceField::TempleWidth
        | IngameFaceField::OuterEyebrowHeight
        | IngameFaceField::Pes15EyebrowDepth
        | IngameFaceField::PupilSize
        | IngameFaceField::EyeDepth
        | IngameFaceField::InnerEyePosition
        | IngameFaceField::InnerEyeHeight
        | IngameFaceField::OuterEyePosition
        | IngameFaceField::EyeCornerHeight
        | IngameFaceField::UpperEyelidWidthInner
        | IngameFaceField::UpperEyelidHeightInner
        | IngameFaceField::UpperEyelidWidthOuter
        | IngameFaceField::UpperEyelidHeightOuter
        | IngameFaceField::Pes15BottomEyelidPosition
        | IngameFaceField::BottomEyelidHeight
        | IngameFaceField::HorizontalEyePosition
        | IngameFaceField::EyeHeight
        | IngameFaceField::NoseHeight
        | IngameFaceField::NoseTipDepth
        | IngameFaceField::NostrilWidth
        | IngameFaceField::Pes15NasalBridgePosition
        | IngameFaceField::NoseDepth
        | IngameFaceField::NoseWidth
        | IngameFaceField::Pes15NasalRootHeight
        | IngameFaceField::MouthHeight
        | IngameFaceField::MouthDepth
        | IngameFaceField::LipSize
        | IngameFaceField::MouthCornerHeight
        | IngameFaceField::LipWidth
        | IngameFaceField::Pes15MouthEdgePosition
        | IngameFaceField::ChinWidth
        | IngameFaceField::ChinHeight
        | IngameFaceField::ChinDepth
        | IngameFaceField::SkinColor
        | IngameFaceField::CheekType
        | IngameFaceField::ForeheadType
        | IngameFaceField::FacialHairType
        | IngameFaceField::LaughterLinesType
        | IngameFaceField::UpperEyelidType
        | IngameFaceField::LowerEyelidType
        | IngameFaceField::EarLength
        | IngameFaceField::EarWidth
        | IngameFaceField::EarAngle
        | IngameFaceField::EyebrowType
        | IngameFaceField::NeckLineType
        | IngameFaceField::FaceSize
        | IngameFaceField::FaceHeight
        | IngameFaceField::NoseType
        | IngameFaceField::UpperLipType
        | IngameFaceField::LowerLipType
        | IngameFaceField::HairColorRed
        | IngameFaceField::HairColorGreen
        | IngameFaceField::HairColorBlue
        | IngameFaceField::Pes15HairVariation
        | IngameFaceField::HairStyle
        | IngameFaceField::HairCroppedStyle
        | IngameFaceField::EyebrowColorRed
        | IngameFaceField::EyebrowStyle
        | IngameFaceField::EyebrowColorGreen
        | IngameFaceField::EyebrowDensity
        | IngameFaceField::EyebrowColorBlue
        | IngameFaceField::EyebrowThickness
        | IngameFaceField::Pes15HairLength
        | IngameFaceField::HairLength
        | IngameFaceField::Pes15HairType
        | IngameFaceField::HairWaveLevel
        | IngameFaceField::Pes15EyeWhite
        | IngameFaceField::HairAccessory
        | IngameFaceField::HairAccessoryColor
        | IngameFaceField::FacialHairColorRed
        | IngameFaceField::FacialHairColorGreen
        | IngameFaceField::FacialHairColorBlue
        | IngameFaceField::FacialHairThickness
        | IngameFaceField::IrisColor
        | IngameFaceField::HairVariation
        | IngameFaceField::HairColorPreset
        | IngameFaceField::HairFrontStyle
        | IngameFaceField::HairParted
        | IngameFaceField::HairSideBackStyle
        | IngameFaceField::Hairline
        | IngameFaceField::HairForeheadWidth => Ownership::Settings,
    }
}
