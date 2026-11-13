//! The settings.toml key table: one `SettingKey` per key of the block in
//! `spec/aesthetics_export/settings_toml.md` "Player settings in exports
//! (settings.toml)", in the block's order,
//! with the block's tables, ranges, label lists and comments reproduced
//! literally.

use pes_version::PesVersion;

use super::held_on;
use crate::schema::fields::PlayerField;
use crate::schema::ingame_face::IngameFaceField;
use crate::schema::limits::{face_type_cap, motion_top};

/// One TOML key of the table in the aesthetics export plan ("Player settings
/// in exports"), in the table's order. `ALL` lists them in that order.
/// `BootsId`/`GlovesId` are the block's two top-level keys (`table` is ""):
/// they sit beside `name`, outside every table the `appearance` walkers read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingKey {
    /// `boots_id` (top level)
    BootsId,
    /// `gloves_id` (top level)
    GlovesId,
    /// `[appearance] skin_color`
    SkinColor,
    /// `[appearance] iris_color`
    IrisColor,
    /// `[appearance.physique] neck_length`
    NeckLength,
    /// `[appearance.physique] neck_size`
    NeckSize,
    /// `[appearance.physique] shoulder_height`
    ShoulderHeight,
    /// `[appearance.physique] shoulder_width`
    ShoulderWidth,
    /// `[appearance.physique] chest`
    Chest,
    /// `[appearance.physique] waist`
    Waist,
    /// `[appearance.physique] arm_size`
    ArmSize,
    /// `[appearance.physique] arm_length`
    ArmLength,
    /// `[appearance.physique] thigh`
    Thigh,
    /// `[appearance.physique] calf`
    Calf,
    /// `[appearance.physique] leg_length`
    LegLength,
    /// `[appearance.physique] head_length`
    HeadLength,
    /// `[appearance.physique] head_width`
    HeadWidth,
    /// `[appearance.physique] head_depth`
    HeadDepth,
    /// `[appearance.strip] sleeves`
    Sleeves,
    /// `[appearance.strip] inners`
    Inners,
    /// `[appearance.strip] socks`
    Socks,
    /// `[appearance.strip] undershorts`
    Undershorts,
    /// `[appearance.strip] untucked`
    Untucked,
    /// `[appearance.strip] ankle_taping`
    AnkleTaping,
    /// `[appearance.strip] wrist_taping`
    WristTaping,
    /// `[appearance.strip] wrist_tape_color_left`
    WristTapeColorLeft,
    /// `[appearance.strip] wrist_tape_color_right`
    WristTapeColorRight,
    /// `[appearance.strip] spectacles`
    Spectacles,
    /// `[appearance.strip] spectacles_color`
    SpectaclesColor,
    /// `[appearance.strip] gloves`
    Gloves,
    /// `[appearance.strip] gloves_color`
    GlovesColor,
    /// `[appearance.motion] hunching_dribbling`
    HunchingDribbling,
    /// `[appearance.motion] hunching_running`
    HunchingRunning,
    /// `[appearance.motion] arm_movement_dribbling`
    ArmMovementDribbling,
    /// `[appearance.motion] arm_movement_running`
    ArmMovementRunning,
    /// `[appearance.motion] corner_kick`
    CornerKick,
    /// `[appearance.motion] free_kick`
    FreeKick,
    /// `[appearance.motion] penalty_kick`
    PenaltyKick,
    /// `[appearance.motion] dribbling` (PES 20+)
    Dribbling,
    /// `[appearance.motion] goal_celebration_1`
    GoalCelebration1,
    /// `[appearance.motion] goal_celebration_2`
    GoalCelebration2,
    /// `[appearance.face] cheek_type`
    CheekType,
    /// `[appearance.face] forehead_type`
    ForeheadType,
    /// `[appearance.face] facial_hair_type`
    FacialHairType,
    /// `[appearance.face] laughter_lines_type`
    LaughterLinesType,
    /// `[appearance.face] upper_eyelid_type`
    UpperEyelidType,
    /// `[appearance.face] lower_eyelid_type`
    LowerEyelidType,
    /// `[appearance.face] eyebrow_type`
    EyebrowType,
    /// `[appearance.face] neck_line_type`
    NeckLineType,
    /// `[appearance.face] nose_type`
    NoseType,
    /// `[appearance.face] upper_lip_type`
    UpperLipType,
    /// `[appearance.face] lower_lip_type`
    LowerLipType,
    /// `[appearance.face] jawline`
    Jawline,
    /// `[appearance.face] jaw_height`
    JawHeight,
    /// `[appearance.face] cheekbones`
    Cheekbones,
    /// `[appearance.face] cheekbone_height`
    CheekboneHeight,
    /// `[appearance.face] chin_width`
    ChinWidth,
    /// `[appearance.face] chin_height`
    ChinHeight,
    /// `[appearance.face] chin_depth`
    ChinDepth,
    /// `[appearance.face] face_size`
    FaceSize,
    /// `[appearance.face] face_height`
    FaceHeight,
    /// `[appearance.face] brow_width`
    BrowWidth,
    /// `[appearance.face] inner_eyebrow_height`
    InnerEyebrowHeight,
    /// `[appearance.face] eyebrow_depth`
    EyebrowDepth,
    /// `[appearance.face] temple_width`
    TempleWidth,
    /// `[appearance.face] outer_eyebrow_height`
    OuterEyebrowHeight,
    /// `[appearance.face] pupil_size`
    PupilSize,
    /// `[appearance.face] eye_depth`
    EyeDepth,
    /// `[appearance.face] inner_eye_position`
    InnerEyePosition,
    /// `[appearance.face] inner_eye_height`
    InnerEyeHeight,
    /// `[appearance.face] outer_eye_position`
    OuterEyePosition,
    /// `[appearance.face] eye_corner_height`
    EyeCornerHeight,
    /// `[appearance.face] upper_eyelid_width_inner`
    UpperEyelidWidthInner,
    /// `[appearance.face] upper_eyelid_height_inner`
    UpperEyelidHeightInner,
    /// `[appearance.face] upper_eyelid_width_outer`
    UpperEyelidWidthOuter,
    /// `[appearance.face] upper_eyelid_height_outer`
    UpperEyelidHeightOuter,
    /// `[appearance.face] bottom_eyelid_height`
    BottomEyelidHeight,
    /// `[appearance.face] horizontal_eye_position`
    HorizontalEyePosition,
    /// `[appearance.face] eye_height`
    EyeHeight,
    /// `[appearance.face] nose_height`
    NoseHeight,
    /// `[appearance.face] nose_tip_depth`
    NoseTipDepth,
    /// `[appearance.face] nostril_width`
    NostrilWidth,
    /// `[appearance.face] nose_depth`
    NoseDepth,
    /// `[appearance.face] nose_width`
    NoseWidth,
    /// `[appearance.face] mouth_height`
    MouthHeight,
    /// `[appearance.face] mouth_depth`
    MouthDepth,
    /// `[appearance.face] lip_size`
    LipSize,
    /// `[appearance.face] mouth_corner_height`
    MouthCornerHeight,
    /// `[appearance.face] lip_width`
    LipWidth,
    /// `[appearance.face] ear_length`
    EarLength,
    /// `[appearance.face] ear_width`
    EarWidth,
    /// `[appearance.face] ear_angle`
    EarAngle,
    /// `[appearance.face] pes15_jaw_depth`
    Pes15JawDepth,
    /// `[appearance.face] pes15_cheekbone_depth`
    Pes15CheekboneDepth,
    /// `[appearance.face] pes15_mouth_edge_position`
    Pes15MouthEdgePosition,
    /// `[appearance.face] pes15_eyebrow_depth`
    Pes15EyebrowDepth,
    /// `[appearance.face] pes15_bottom_eyelid_position`
    Pes15BottomEyelidPosition,
    /// `[appearance.face] pes15_nasal_bridge_position`
    Pes15NasalBridgePosition,
    /// `[appearance.face] pes15_nasal_root_height`
    Pes15NasalRootHeight,
    /// `[appearance.face] pes15_nose_angle`
    Pes15NoseAngle,
    /// `[appearance.face] eyebrow_color_red`
    EyebrowColorRed,
    /// `[appearance.face] eyebrow_color_green`
    EyebrowColorGreen,
    /// `[appearance.face] eyebrow_color_blue`
    EyebrowColorBlue,
    /// `[appearance.face] eyebrow_style`
    EyebrowStyle,
    /// `[appearance.face] eyebrow_density`
    EyebrowDensity,
    /// `[appearance.face] eyebrow_thickness`
    EyebrowThickness,
    /// `[appearance.face] facial_hair_color_red`
    FacialHairColorRed,
    /// `[appearance.face] facial_hair_color_green`
    FacialHairColorGreen,
    /// `[appearance.face] facial_hair_color_blue`
    FacialHairColorBlue,
    /// `[appearance.face] facial_hair_thickness`
    FacialHairThickness,
    /// `[appearance.face] pes15_eye_white`
    Pes15EyeWhite,
    /// `[appearance.hair] color_red`
    HairColorRed,
    /// `[appearance.hair] color_green`
    HairColorGreen,
    /// `[appearance.hair] color_blue`
    HairColorBlue,
    /// `[appearance.hair] color_preset`
    HairColorPreset,
    /// `[appearance.hair] accessory`
    HairAccessory,
    /// `[appearance.hair] accessory_color`
    HairAccessoryColor,
    /// `[appearance.hair] style`
    HairStyle,
    /// `[appearance.hair] cropped_style`
    HairCroppedStyle,
    /// `[appearance.hair] length`
    HairLength,
    /// `[appearance.hair] wave_level`
    HairWaveLevel,
    /// `[appearance.hair] variation`
    HairVariation,
    /// `[appearance.hair] front_style`
    HairFrontStyle,
    /// `[appearance.hair] parted`
    HairParted,
    /// `[appearance.hair] side_back_style`
    HairSideBackStyle,
    /// `[appearance.hair] hairline`
    Hairline,
    /// `[appearance.hair] forehead_width`
    HairForeheadWidth,
    /// `[appearance.hair] pes15_length`
    Pes15HairLength,
    /// `[appearance.hair] pes15_type`
    Pes15HairType,
    /// `[appearance.hair] pes15_variation`
    Pes15HairVariation,
}

/// How a TOML value maps to the stored `u8`, and the widest accepted range
/// (a PES 16 file may legitimately be compiled for PES 21; the per-version
/// narrowing is the compiler's finding at compile time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Stored as written: colours, celebrations, face types.
    Number {
        /// Smallest accepted value.
        min: u8,
        /// Largest accepted value.
        max: u8,
    },
    /// Physique and face sliders (limit 7), PES 15's nose angle (limit 1):
    /// TOML -limit..=limit, stored value + limit.
    Signed {
        /// Largest accepted magnitude.
        limit: u8,
    },
    /// Motion: TOML 1..=max, stored value - 1.
    OneBased {
        /// Largest accepted value.
        max: u8,
    },
    /// The hair's wave level: TOML -1..=max, stored value + 1 (the editor's
    /// 0 to max; -1, stored 0, as some saves hold it).
    PlusOne {
        /// Largest accepted value.
        max: u8,
    },
    /// The hair's colour preset: TOML `i` is stored as `map[i]`, the game's
    /// internal preset id in the editor's order.
    Mapped(&'static [u8]),
    /// Stored 0/1.
    Bool,
    /// Stored value = index into the list.
    Labels(&'static [&'static str]),
}

impl Kind {
    /// Whether a stored `u8` is one the kind can represent. A hostile save
    /// can hold a value outside it (a 2-bit label field reading 3); the
    /// parser never produces one and `from_player` refuses it.
    pub fn accepts_stored(self, stored: u8) -> bool {
        match self {
            Kind::Number { min, max } => (min..=max).contains(&stored),
            Kind::Signed { limit } => u16::from(stored) <= 2 * u16::from(limit),
            Kind::OneBased { max } => stored < max,
            Kind::PlusOne { max } => u16::from(stored) <= u16::from(max) + 1,
            Kind::Mapped(map) => map.contains(&stored),
            Kind::Bool => stored <= 1,
            Kind::Labels(labels) => usize::from(stored) < labels.len(),
        }
    }
}

/// The TOML table a settings key sits under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTable {
    /// The document root: the key is a top-level line.
    Top,
    /// `[appearance]` itself.
    Appearance,
    /// `[appearance.physique]`.
    Physique,
    /// `[appearance.strip]`.
    Strip,
    /// `[appearance.motion]`.
    Motion,
    /// `[appearance.face]`.
    Face,
    /// `[appearance.hair]`.
    Hair,
}

impl KeyTable {
    /// The table's dotted TOML path ("" for `Top`).
    pub fn path(self) -> &'static str {
        match self {
            KeyTable::Top => "",
            KeyTable::Appearance => "appearance",
            KeyTable::Physique => "appearance.physique",
            KeyTable::Strip => "appearance.strip",
            KeyTable::Motion => "appearance.motion",
            KeyTable::Face => "appearance.face",
            KeyTable::Hair => "appearance.hair",
        }
    }

    /// The sub-table name under `[appearance]`; `None` for `Top` and
    /// `Appearance` (which is the root itself).
    pub fn sub_table(self) -> Option<&'static str> {
        match self {
            KeyTable::Physique => Some("physique"),
            KeyTable::Strip => Some("strip"),
            KeyTable::Motion => Some("motion"),
            KeyTable::Face => Some("face"),
            KeyTable::Hair => Some("hair"),
            KeyTable::Top | KeyTable::Appearance => None,
        }
    }
}

/// The columns of one key-table row.
#[derive(Debug, Clone, Copy)]
pub struct KeySpec {
    /// The TOML table the key sits under.
    pub table: KeyTable,
    /// The TOML key name.
    pub name: &'static str,
    /// How the TOML value maps to the stored `u8`.
    pub kind: Kind,
    /// The range comment the template emits after the key, verbatim from the
    /// spec block.
    pub comment: &'static str,
    /// The stored value an absent key applies: the template's shown value (a
    /// physique `0` stores 7, a 1-based motion `1` stores 0, a label its
    /// index, `true` 1). `None` only for `BootsId`/`GlovesId`: an absent
    /// stock ID is the compiler's to resolve (the folder's models, the FPC
    /// marker), not a fixed value.
    pub default: Option<u8>,
}

/// Which savefile field a key writes: the gate cross-version imports check
/// (`team_toml`'s apply drops a `Source::Player` key the target schema lacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Source {
    /// A `PlayerField` of the player or appearance record.
    Player(PlayerField),
    /// A bit run inside the ingame-face run. `NoseType` is read and written
    /// through `IngameFace::nose`, since PES 15 codes the angle into it.
    Face(IngameFaceField),
    /// PES 15's nose angle, coded into the `NoseType` field
    /// (`IngameFace::nose`).
    NoseAngle,
}

const SEVEN: &str = "-7 to 7";

impl SettingKey {
    /// Every key, in the spec table's order.
    pub const ALL: [SettingKey; 130] = [
        SettingKey::BootsId,
        SettingKey::GlovesId,
        SettingKey::SkinColor,
        SettingKey::IrisColor,
        SettingKey::NeckLength,
        SettingKey::NeckSize,
        SettingKey::ShoulderHeight,
        SettingKey::ShoulderWidth,
        SettingKey::Chest,
        SettingKey::Waist,
        SettingKey::ArmSize,
        SettingKey::ArmLength,
        SettingKey::Thigh,
        SettingKey::Calf,
        SettingKey::LegLength,
        SettingKey::HeadLength,
        SettingKey::HeadWidth,
        SettingKey::HeadDepth,
        SettingKey::Sleeves,
        SettingKey::Inners,
        SettingKey::Socks,
        SettingKey::Undershorts,
        SettingKey::Untucked,
        SettingKey::AnkleTaping,
        SettingKey::WristTaping,
        SettingKey::WristTapeColorLeft,
        SettingKey::WristTapeColorRight,
        SettingKey::Spectacles,
        SettingKey::SpectaclesColor,
        SettingKey::Gloves,
        SettingKey::GlovesColor,
        SettingKey::HunchingDribbling,
        SettingKey::HunchingRunning,
        SettingKey::ArmMovementDribbling,
        SettingKey::ArmMovementRunning,
        SettingKey::CornerKick,
        SettingKey::FreeKick,
        SettingKey::PenaltyKick,
        SettingKey::Dribbling,
        SettingKey::GoalCelebration1,
        SettingKey::GoalCelebration2,
        SettingKey::Jawline,
        SettingKey::JawHeight,
        SettingKey::Cheekbones,
        SettingKey::CheekboneHeight,
        SettingKey::ChinWidth,
        SettingKey::ChinHeight,
        SettingKey::ChinDepth,
        SettingKey::FaceSize,
        SettingKey::FaceHeight,
        SettingKey::BrowWidth,
        SettingKey::InnerEyebrowHeight,
        SettingKey::EyebrowDepth,
        SettingKey::TempleWidth,
        SettingKey::OuterEyebrowHeight,
        SettingKey::PupilSize,
        SettingKey::EyeDepth,
        SettingKey::InnerEyePosition,
        SettingKey::InnerEyeHeight,
        SettingKey::OuterEyePosition,
        SettingKey::EyeCornerHeight,
        SettingKey::UpperEyelidWidthInner,
        SettingKey::UpperEyelidHeightInner,
        SettingKey::UpperEyelidWidthOuter,
        SettingKey::UpperEyelidHeightOuter,
        SettingKey::BottomEyelidHeight,
        SettingKey::HorizontalEyePosition,
        SettingKey::EyeHeight,
        SettingKey::NoseHeight,
        SettingKey::NoseTipDepth,
        SettingKey::NostrilWidth,
        SettingKey::NoseDepth,
        SettingKey::NoseWidth,
        SettingKey::MouthHeight,
        SettingKey::MouthDepth,
        SettingKey::LipSize,
        SettingKey::MouthCornerHeight,
        SettingKey::LipWidth,
        SettingKey::EarLength,
        SettingKey::EarWidth,
        SettingKey::EarAngle,
        SettingKey::Pes15JawDepth,
        SettingKey::Pes15CheekboneDepth,
        SettingKey::Pes15MouthEdgePosition,
        SettingKey::Pes15EyebrowDepth,
        SettingKey::Pes15BottomEyelidPosition,
        SettingKey::Pes15NasalBridgePosition,
        SettingKey::Pes15NasalRootHeight,
        SettingKey::CheekType,
        SettingKey::ForeheadType,
        SettingKey::FacialHairType,
        SettingKey::LaughterLinesType,
        SettingKey::UpperEyelidType,
        SettingKey::LowerEyelidType,
        SettingKey::EyebrowType,
        SettingKey::NeckLineType,
        SettingKey::NoseType,
        SettingKey::Pes15NoseAngle,
        SettingKey::UpperLipType,
        SettingKey::LowerLipType,
        SettingKey::EyebrowColorRed,
        SettingKey::EyebrowColorGreen,
        SettingKey::EyebrowColorBlue,
        SettingKey::EyebrowStyle,
        SettingKey::EyebrowDensity,
        SettingKey::EyebrowThickness,
        SettingKey::FacialHairColorRed,
        SettingKey::FacialHairColorGreen,
        SettingKey::FacialHairColorBlue,
        SettingKey::FacialHairThickness,
        SettingKey::Pes15EyeWhite,
        SettingKey::HairColorRed,
        SettingKey::HairColorGreen,
        SettingKey::HairColorBlue,
        SettingKey::HairColorPreset,
        SettingKey::HairAccessory,
        SettingKey::HairAccessoryColor,
        SettingKey::HairStyle,
        SettingKey::HairCroppedStyle,
        SettingKey::HairLength,
        SettingKey::HairWaveLevel,
        SettingKey::HairVariation,
        SettingKey::HairFrontStyle,
        SettingKey::HairParted,
        SettingKey::HairSideBackStyle,
        SettingKey::Hairline,
        SettingKey::HairForeheadWidth,
        SettingKey::Pes15HairLength,
        SettingKey::Pes15HairType,
        SettingKey::Pes15HairVariation,
    ];

    /// The key's row in the spec table.
    pub fn spec(self) -> KeySpec {
        match self {
            SettingKey::BootsId => KeySpec {
                table: KeyTable::Top,
                name: "boots_id",
                kind: Kind::Number { min: 0, max: 100 },
                comment: "\"\" = default (fpc_on: hidden; fpc_off: 0; no marker: 0, or the save's ID in a Midcup export); 0 to 100, a stock boots model; ignored when the folder has boots models or a boots link (pre-Fox face.xml boots do not count)",
                default: None,
            },
            SettingKey::GlovesId => KeySpec {
                table: KeyTable::Top,
                name: "gloves_id",
                kind: Kind::Number { min: 0, max: 100 },
                comment: "\"\" = default (fpc_on: hidden; fpc_off: 0; no marker: 0, or the save's ID in a Midcup export); 0 to 100, a stock gloves model (0 = normal hands); ignored when the folder has gloves models or a gloves link (pre-Fox face.xml gloves do not count)",
                default: None,
            },
            SettingKey::SkinColor => KeySpec {
                table: KeyTable::Appearance,
                name: "skin_color",
                kind: Kind::Number { min: 0, max: 7 },
                comment: "0 white, 1 light, 2 fair, 3 medium, 4 olive, 5 brown, 6 black, 7 custom (invisible body, PES 15 to 17 only)",
                default: Some(1),
            },
            SettingKey::IrisColor => KeySpec {
                table: KeyTable::Appearance,
                name: "iris_color",
                kind: Kind::Number { min: 0, max: 10 },
                comment: "0 black, 1 dark brown, 2 brown, 3 sable, 4 navy blue, 5 charcoal, 6 gray, 7 blue, 8 sienna, 9 green, 10 violet",
                default: Some(0),
            },
            SettingKey::NeckLength => physique("neck_length"),
            SettingKey::NeckSize => physique("neck_size"),
            SettingKey::ShoulderHeight => physique("shoulder_height"),
            SettingKey::ShoulderWidth => physique("shoulder_width"),
            SettingKey::Chest => physique("chest"),
            SettingKey::Waist => physique("waist"),
            SettingKey::ArmSize => physique("arm_size"),
            SettingKey::ArmLength => physique("arm_length"),
            SettingKey::Thigh => physique("thigh"),
            SettingKey::Calf => physique("calf"),
            SettingKey::LegLength => physique("leg_length"),
            SettingKey::HeadLength => physique("head_length"),
            SettingKey::HeadWidth => physique("head_width"),
            SettingKey::HeadDepth => physique("head_depth"),
            SettingKey::Sleeves => KeySpec {
                table: KeyTable::Strip,
                name: "sleeves",
                kind: Kind::Labels(&["seasonal", "short", "long"]),
                comment: "\"seasonal\", \"short\", \"long\"",
                // The block's `sleeves = "short"`.
                default: Some(1),
            },
            SettingKey::Inners => KeySpec {
                table: KeyTable::Strip,
                name: "inners",
                kind: Kind::Labels(&["off", "normal", "turtleneck"]),
                comment: "\"off\", \"normal\", \"turtleneck\"",
                default: Some(0),
            },
            SettingKey::Socks => KeySpec {
                table: KeyTable::Strip,
                name: "socks",
                kind: Kind::Labels(&["standard", "long", "short"]),
                comment: "\"standard\", \"long\", \"short\"",
                default: Some(0),
            },
            SettingKey::Undershorts => KeySpec {
                table: KeyTable::Strip,
                name: "undershorts",
                kind: Kind::Labels(&["off", "short", "winter_long", "short_winter_long"]),
                comment: "\"off\", \"short\", \"winter_long\" (none in summer, long in winter), \"short_winter_long\" (short in summer, long in winter)",
                default: Some(0),
            },
            SettingKey::Untucked => bool_key("untucked", "true, false", true),
            SettingKey::AnkleTaping => bool_key("ankle_taping", "true, false", false),
            SettingKey::WristTaping => KeySpec {
                table: KeyTable::Strip,
                name: "wrist_taping",
                kind: Kind::Labels(&["off", "right", "left", "both"]),
                comment: "\"off\", \"right\", \"left\", \"both\"",
                default: Some(0),
            },
            SettingKey::WristTapeColorLeft => seven_color("wrist_tape_color_left"),
            SettingKey::WristTapeColorRight => seven_color("wrist_tape_color_right"),
            SettingKey::Spectacles => KeySpec {
                table: KeyTable::Strip,
                name: "spectacles",
                kind: Kind::Number { min: 0, max: 7 },
                comment: "0 none, 1 rectangle rimless, 2 rectangle half frame, 3 rectangle full frame, 4 oval rimless, 5 oval half frame, 6 oval full frame, 7 round full frame",
                default: Some(0),
            },
            SettingKey::SpectaclesColor => KeySpec {
                table: KeyTable::Strip,
                name: "spectacles_color",
                kind: Kind::Number { min: 0, max: 7 },
                comment: "0 white, 1 black, 2 red, 3 blue, 4 yellow, 5 green, 6 pink, 7 turquoise",
                default: Some(0),
            },
            SettingKey::Gloves => bool_key("gloves", "true, false (outfield player gloves)", false),
            SettingKey::GlovesColor => seven_color("gloves_color"),
            SettingKey::HunchingDribbling => motion_1based(
                "hunching_dribbling",
                PlayerField::HunchingDribbling,
                "1 to 3 (PES 20 and 21: 1 to 5)",
            ),
            SettingKey::HunchingRunning => motion_1based(
                "hunching_running",
                PlayerField::HunchingRunning,
                "1 to 3 (PES 20 and 21: 1 to 5)",
            ),
            SettingKey::ArmMovementDribbling => motion_1based(
                "arm_movement_dribbling",
                PlayerField::ArmMovementDribbling,
                "1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)",
            ),
            SettingKey::ArmMovementRunning => motion_1based(
                "arm_movement_running",
                PlayerField::ArmMovementRunning,
                "1 to 8 (PES 15: 1 to 5; PES 20 and 21: 1 to 10)",
            ),
            SettingKey::CornerKick => motion_1based(
                "corner_kick",
                PlayerField::CornerKickMotion,
                "1 to 6 (PES 20 and 21: 1 to 10)",
            ),
            SettingKey::FreeKick => motion_1based(
                "free_kick",
                PlayerField::FreeKickMotion,
                "1 to 16 (PES 20 and 21: 1 to 20)",
            ),
            SettingKey::PenaltyKick => motion_1based(
                "penalty_kick",
                PlayerField::PenaltyKickMotion,
                "1 to 4 (PES 20 and 21: 1 to 7)",
            ),
            SettingKey::Dribbling => KeySpec {
                table: KeyTable::Motion,
                name: "dribbling",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "0 to 3 (PES 20 and 21 only)",
                default: Some(0),
            },
            SettingKey::GoalCelebration1 => {
                celebration("goal_celebration_1", PlayerField::GoalCelebration1)
            }
            SettingKey::GoalCelebration2 => {
                celebration("goal_celebration_2", PlayerField::GoalCelebration2)
            }
            SettingKey::CheekType => face(
                "cheek_type",
                IngameFaceField::CheekType,
                "0 to 3 (PES 15: 0 to 14; PES 19: 0 to 5)",
            ),
            SettingKey::ForeheadType => face(
                "forehead_type",
                IngameFaceField::ForeheadType,
                "0 to 5 (PES 15 and 19: 0 to 6)",
            ),
            SettingKey::FacialHairType => face(
                "facial_hair_type",
                IngameFaceField::FacialHairType,
                "0 to 12 (PES 15: 0 to 29; PES 20 and 21: 0 to 19)",
            ),
            SettingKey::LaughterLinesType => face(
                "laughter_lines_type",
                IngameFaceField::LaughterLinesType,
                "0 to 4 (PES 15: 0 to 6)",
            ),
            SettingKey::UpperEyelidType => face(
                "upper_eyelid_type",
                IngameFaceField::UpperEyelidType,
                "0 to 6 (PES 15: 0 to 14; PES 20 and 21: 0 to 7)",
            ),
            SettingKey::LowerEyelidType => face(
                "lower_eyelid_type",
                IngameFaceField::LowerEyelidType,
                "0 to 2 (PES 15: 0 to 14; PES 20 and 21: 0 to 6)",
            ),
            SettingKey::EyebrowType => face(
                "eyebrow_type",
                IngameFaceField::EyebrowType,
                "0 to 5 (PES 15: 0 to 17; PES 20 and 21: 0 to 7)",
            ),
            SettingKey::NeckLineType => face(
                "neck_line_type",
                IngameFaceField::NeckLineType,
                "0 to 2 (PES 15: 0 to 5; PES 20 and 21: 0 to 3)",
            ),
            SettingKey::NoseType => face(
                "nose_type",
                IngameFaceField::NoseType,
                "0 to 6 (PES 15: 0 to 11; PES 20 and 21: 0 to 7)",
            ),
            SettingKey::UpperLipType => face(
                "upper_lip_type",
                IngameFaceField::UpperLipType,
                "0 to 3 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)",
            ),
            SettingKey::LowerLipType => face(
                "lower_lip_type",
                IngameFaceField::LowerLipType,
                "0 to 2 (PES 15: 0 to 7; PES 20 and 21: 0 to 4)",
            ),
            SettingKey::Jawline => KeySpec {
                table: KeyTable::Face,
                name: "jawline",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::JawHeight => KeySpec {
                table: KeyTable::Face,
                name: "jaw_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::Cheekbones => KeySpec {
                table: KeyTable::Face,
                name: "cheekbones",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::CheekboneHeight => KeySpec {
                table: KeyTable::Face,
                name: "cheekbone_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7 (PES 16 to 21: no editor slider)",
                default: Some(7),
            },
            SettingKey::ChinWidth => KeySpec {
                table: KeyTable::Face,
                name: "chin_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::ChinHeight => KeySpec {
                table: KeyTable::Face,
                name: "chin_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::ChinDepth => KeySpec {
                table: KeyTable::Face,
                name: "chin_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::FaceSize => KeySpec {
                table: KeyTable::Face,
                name: "face_size",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::FaceHeight => KeySpec {
                table: KeyTable::Face,
                name: "face_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::BrowWidth => KeySpec {
                table: KeyTable::Face,
                name: "brow_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::InnerEyebrowHeight => KeySpec {
                table: KeyTable::Face,
                name: "inner_eyebrow_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EyebrowDepth => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::TempleWidth => KeySpec {
                table: KeyTable::Face,
                name: "temple_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::OuterEyebrowHeight => KeySpec {
                table: KeyTable::Face,
                name: "outer_eyebrow_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::PupilSize => KeySpec {
                table: KeyTable::Face,
                name: "pupil_size",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EyeDepth => KeySpec {
                table: KeyTable::Face,
                name: "eye_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::InnerEyePosition => KeySpec {
                table: KeyTable::Face,
                name: "inner_eye_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::InnerEyeHeight => KeySpec {
                table: KeyTable::Face,
                name: "inner_eye_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::OuterEyePosition => KeySpec {
                table: KeyTable::Face,
                name: "outer_eye_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EyeCornerHeight => KeySpec {
                table: KeyTable::Face,
                name: "eye_corner_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::UpperEyelidWidthInner => KeySpec {
                table: KeyTable::Face,
                name: "upper_eyelid_width_inner",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::UpperEyelidHeightInner => KeySpec {
                table: KeyTable::Face,
                name: "upper_eyelid_height_inner",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::UpperEyelidWidthOuter => KeySpec {
                table: KeyTable::Face,
                name: "upper_eyelid_width_outer",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::UpperEyelidHeightOuter => KeySpec {
                table: KeyTable::Face,
                name: "upper_eyelid_height_outer",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::BottomEyelidHeight => KeySpec {
                table: KeyTable::Face,
                name: "bottom_eyelid_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::HorizontalEyePosition => KeySpec {
                table: KeyTable::Face,
                name: "horizontal_eye_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EyeHeight => KeySpec {
                table: KeyTable::Face,
                name: "eye_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::NoseHeight => KeySpec {
                table: KeyTable::Face,
                name: "nose_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::NoseTipDepth => KeySpec {
                table: KeyTable::Face,
                name: "nose_tip_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::NostrilWidth => KeySpec {
                table: KeyTable::Face,
                name: "nostril_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::NoseDepth => KeySpec {
                table: KeyTable::Face,
                name: "nose_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::NoseWidth => KeySpec {
                table: KeyTable::Face,
                name: "nose_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::MouthHeight => KeySpec {
                table: KeyTable::Face,
                name: "mouth_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::MouthDepth => KeySpec {
                table: KeyTable::Face,
                name: "mouth_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::LipSize => KeySpec {
                table: KeyTable::Face,
                name: "lip_size",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::MouthCornerHeight => KeySpec {
                table: KeyTable::Face,
                name: "mouth_corner_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::LipWidth => KeySpec {
                table: KeyTable::Face,
                name: "lip_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EarLength => KeySpec {
                table: KeyTable::Face,
                name: "ear_length",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EarWidth => KeySpec {
                table: KeyTable::Face,
                name: "ear_width",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::EarAngle => KeySpec {
                table: KeyTable::Face,
                name: "ear_angle",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7",
                default: Some(7),
            },
            SettingKey::Pes15JawDepth => KeySpec {
                table: KeyTable::Face,
                name: "pes15_jaw_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15CheekboneDepth => KeySpec {
                table: KeyTable::Face,
                name: "pes15_cheekbone_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15MouthEdgePosition => KeySpec {
                table: KeyTable::Face,
                name: "pes15_mouth_edge_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15EyebrowDepth => KeySpec {
                table: KeyTable::Face,
                name: "pes15_eyebrow_depth",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15BottomEyelidPosition => KeySpec {
                table: KeyTable::Face,
                name: "pes15_bottom_eyelid_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15NasalBridgePosition => KeySpec {
                table: KeyTable::Face,
                name: "pes15_nasal_bridge_position",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15NasalRootHeight => KeySpec {
                table: KeyTable::Face,
                name: "pes15_nasal_root_height",
                kind: Kind::Signed { limit: 7 },
                comment: "-7 to 7, drawn by PES 15 only",
                default: Some(7),
            },
            SettingKey::Pes15NoseAngle => KeySpec {
                table: KeyTable::Face,
                name: "pes15_nose_angle",
                kind: Kind::Signed { limit: 1 },
                comment: "-1 to 1; PES 15 only",
                default: Some(1),
            },
            SettingKey::EyebrowColorRed => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_color_red",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(14),
            },
            SettingKey::EyebrowColorGreen => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_color_green",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(10),
            },
            SettingKey::EyebrowColorBlue => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_color_blue",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(7),
            },
            SettingKey::EyebrowStyle => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_style",
                kind: Kind::Number { min: 0, max: 2 },
                comment: "0 normal, 1 thick, 2 thin",
                default: Some(0),
            },
            SettingKey::EyebrowDensity => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_density",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "0 to 3",
                default: Some(3),
            },
            SettingKey::EyebrowThickness => KeySpec {
                table: KeyTable::Face,
                name: "eyebrow_thickness",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "0 to 3",
                default: Some(1),
            },
            SettingKey::FacialHairColorRed => KeySpec {
                table: KeyTable::Face,
                name: "facial_hair_color_red",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(14),
            },
            SettingKey::FacialHairColorGreen => KeySpec {
                table: KeyTable::Face,
                name: "facial_hair_color_green",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(10),
            },
            SettingKey::FacialHairColorBlue => KeySpec {
                table: KeyTable::Face,
                name: "facial_hair_color_blue",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(7),
            },
            SettingKey::FacialHairThickness => KeySpec {
                table: KeyTable::Face,
                name: "facial_hair_thickness",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "0 to 3",
                default: Some(3),
            },
            SettingKey::Pes15EyeWhite => KeySpec {
                table: KeyTable::Face,
                name: "pes15_eye_white",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "0 to 3, brighter eye whites, drawn by PES 15 only",
                default: Some(0),
            },
            SettingKey::HairColorRed => KeySpec {
                table: KeyTable::Hair,
                name: "color_red",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(14),
            },
            SettingKey::HairColorGreen => KeySpec {
                table: KeyTable::Hair,
                name: "color_green",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(10),
            },
            SettingKey::HairColorBlue => KeySpec {
                table: KeyTable::Hair,
                name: "color_blue",
                kind: Kind::Number { min: 0, max: 63 },
                comment: "0 to 63",
                default: Some(7),
            },
            SettingKey::HairColorPreset => KeySpec {
                table: KeyTable::Hair,
                name: "color_preset",
                kind: Kind::Mapped(&[0, 5, 6, 7, 1, 3, 2, 8, 4]),
                comment: "0 custom (the colour above), 1 to 8 the editor's presets",
                default: Some(0),
            },
            SettingKey::HairAccessory => KeySpec {
                table: KeyTable::Hair,
                name: "accessory",
                kind: Kind::Bool,
                comment: "true, false (PES 15: the game sets it with the variation)",
                default: Some(0),
            },
            SettingKey::HairAccessoryColor => KeySpec {
                table: KeyTable::Hair,
                name: "accessory_color",
                kind: Kind::Number { min: 0, max: 7 },
                comment: "0 white, 1 dark grey, 2 red, 3 blue, 4 yellow, 5 green, 6 magenta, 7 cyan",
                default: Some(0),
            },
            SettingKey::HairStyle => KeySpec {
                table: KeyTable::Hair,
                name: "style",
                kind: Kind::Number { min: 0, max: 7 },
                comment: "1 normal, 2 dry, 3 mohican, 4 afro, 5 dreadlocks, 6 braided, 7 special, 0 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairCroppedStyle => KeySpec {
                table: KeyTable::Hair,
                name: "cropped_style",
                kind: Kind::Number { min: 0, max: 6 },
                comment: "1 to 6, 0 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairLength => KeySpec {
                table: KeyTable::Hair,
                name: "length",
                kind: Kind::Number { min: 0, max: 5 },
                comment: "1 shaven, 2 very short, 3 short, 4 medium, 5 long, 0 in some saves; PES 16 to 21",
                default: Some(2),
            },
            SettingKey::HairWaveLevel => KeySpec {
                table: KeyTable::Hair,
                name: "wave_level",
                kind: Kind::PlusOne { max: 6 },
                comment: "0 to 6, -1 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairVariation => KeySpec {
                table: KeyTable::Hair,
                name: "variation",
                kind: Kind::Number { min: 0, max: 29 },
                comment: "0 to 29; PES 16 to 21",
                default: Some(0),
            },
            SettingKey::HairFrontStyle => KeySpec {
                table: KeyTable::Hair,
                name: "front_style",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "1 up, 2 down, 3 combed back, 0 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairParted => KeySpec {
                table: KeyTable::Hair,
                name: "parted",
                kind: Kind::Number { min: 0, max: 6 },
                comment: "1 off, 2 left 2, 3 left 1, 4 centre, 5 right 1, 6 right 2, 0 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairSideBackStyle => KeySpec {
                table: KeyTable::Hair,
                name: "side_back_style",
                kind: Kind::Number { min: 0, max: 4 },
                comment: "1 normal, 2 reduce body, 3 shorter sides, 4 cropped, 0 in some saves; PES 16 to 21",
                default: Some(4),
            },
            SettingKey::Hairline => KeySpec {
                table: KeyTable::Hair,
                name: "hairline",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "1 to 3, 0 in some saves; PES 16 to 21",
                default: Some(1),
            },
            SettingKey::HairForeheadWidth => KeySpec {
                table: KeyTable::Hair,
                name: "forehead_width",
                kind: Kind::Number { min: 0, max: 3 },
                comment: "1 narrow, 2 normal, 3 wide, 0 in some saves; PES 16 to 21",
                default: Some(2),
            },
            SettingKey::Pes15HairLength => KeySpec {
                table: KeyTable::Hair,
                name: "pes15_length",
                kind: Kind::Number { min: 1, max: 6 },
                comment: "1 skinhead, 2 very short, 3 short, 4 medium, 5 long, 6 special; PES 15 only",
                default: Some(2),
            },
            SettingKey::Pes15HairType => KeySpec {
                table: KeyTable::Hair,
                name: "pes15_type",
                kind: Kind::Number { min: 0, max: 4 },
                comment: "0 none (skinhead, special), 1 straight, 2 wavy, 3 permed, 4 dreadlocks; PES 15 only",
                default: Some(1),
            },
            SettingKey::Pes15HairVariation => KeySpec {
                table: KeyTable::Hair,
                name: "pes15_variation",
                kind: Kind::OneBased { max: 82 },
                comment: "1 to 82, the editor's grid position (each length and type has its own grid); PES 15 only",
                default: Some(0),
            },
        }
    }

    /// The savefile field the key writes.
    pub(crate) fn source(self) -> Source {
        match self {
            SettingKey::BootsId => Source::Player(PlayerField::BootsId),
            SettingKey::GlovesId => Source::Player(PlayerField::GlovesId),
            SettingKey::SkinColor => Source::Face(IngameFaceField::SkinColor),
            SettingKey::IrisColor => Source::Face(IngameFaceField::IrisColor),
            SettingKey::NeckLength => Source::Player(PlayerField::NeckLength),
            SettingKey::NeckSize => Source::Player(PlayerField::NeckSize),
            SettingKey::ShoulderHeight => Source::Player(PlayerField::ShoulderHeight),
            SettingKey::ShoulderWidth => Source::Player(PlayerField::ShoulderWidth),
            SettingKey::Chest => Source::Player(PlayerField::Chest),
            SettingKey::Waist => Source::Player(PlayerField::Waist),
            SettingKey::ArmSize => Source::Player(PlayerField::ArmSize),
            SettingKey::ArmLength => Source::Player(PlayerField::ArmLength),
            SettingKey::Thigh => Source::Player(PlayerField::Thigh),
            SettingKey::Calf => Source::Player(PlayerField::Calf),
            SettingKey::LegLength => Source::Player(PlayerField::LegLength),
            SettingKey::HeadLength => Source::Player(PlayerField::HeadLength),
            SettingKey::HeadWidth => Source::Player(PlayerField::HeadWidth),
            SettingKey::HeadDepth => Source::Player(PlayerField::HeadDepth),
            SettingKey::Sleeves => Source::Player(PlayerField::Sleeves),
            SettingKey::Inners => Source::Player(PlayerField::Inners),
            SettingKey::Socks => Source::Player(PlayerField::Socks),
            SettingKey::Undershorts => Source::Player(PlayerField::Undershorts),
            SettingKey::Untucked => Source::Player(PlayerField::Untucked),
            SettingKey::AnkleTaping => Source::Player(PlayerField::AnkleTaping),
            SettingKey::WristTaping => Source::Player(PlayerField::WristTaping),
            SettingKey::WristTapeColorLeft => Source::Player(PlayerField::WristTapeColorLeft),
            SettingKey::WristTapeColorRight => Source::Player(PlayerField::WristTapeColorRight),
            SettingKey::Spectacles => Source::Player(PlayerField::SpectaclesStyle),
            SettingKey::SpectaclesColor => Source::Player(PlayerField::SpectaclesColor),
            SettingKey::Gloves => Source::Face(IngameFaceField::PlayerGloves),
            SettingKey::GlovesColor => Source::Face(IngameFaceField::PlayerGlovesColor),
            SettingKey::HunchingDribbling => Source::Player(PlayerField::HunchingDribbling),
            SettingKey::HunchingRunning => Source::Player(PlayerField::HunchingRunning),
            SettingKey::ArmMovementDribbling => Source::Player(PlayerField::ArmMovementDribbling),
            SettingKey::ArmMovementRunning => Source::Player(PlayerField::ArmMovementRunning),
            SettingKey::CornerKick => Source::Player(PlayerField::CornerKickMotion),
            SettingKey::FreeKick => Source::Player(PlayerField::FreeKickMotion),
            SettingKey::PenaltyKick => Source::Player(PlayerField::PenaltyKickMotion),
            SettingKey::Dribbling => Source::Player(PlayerField::DribblingMotion),
            SettingKey::GoalCelebration1 => Source::Player(PlayerField::GoalCelebration1),
            SettingKey::GoalCelebration2 => Source::Player(PlayerField::GoalCelebration2),
            SettingKey::CheekType => Source::Face(IngameFaceField::CheekType),
            SettingKey::ForeheadType => Source::Face(IngameFaceField::ForeheadType),
            SettingKey::FacialHairType => Source::Face(IngameFaceField::FacialHairType),
            SettingKey::LaughterLinesType => Source::Face(IngameFaceField::LaughterLinesType),
            SettingKey::UpperEyelidType => Source::Face(IngameFaceField::UpperEyelidType),
            SettingKey::LowerEyelidType => Source::Face(IngameFaceField::LowerEyelidType),
            SettingKey::EyebrowType => Source::Face(IngameFaceField::EyebrowType),
            SettingKey::NeckLineType => Source::Face(IngameFaceField::NeckLineType),
            SettingKey::NoseType => Source::Face(IngameFaceField::NoseType),
            SettingKey::UpperLipType => Source::Face(IngameFaceField::UpperLipType),
            SettingKey::LowerLipType => Source::Face(IngameFaceField::LowerLipType),
            SettingKey::Jawline => Source::Face(IngameFaceField::Jawline),
            SettingKey::JawHeight => Source::Face(IngameFaceField::JawHeight),
            SettingKey::Cheekbones => Source::Face(IngameFaceField::Cheekbones),
            SettingKey::CheekboneHeight => Source::Face(IngameFaceField::CheekboneHeight),
            SettingKey::ChinWidth => Source::Face(IngameFaceField::ChinWidth),
            SettingKey::ChinHeight => Source::Face(IngameFaceField::ChinHeight),
            SettingKey::ChinDepth => Source::Face(IngameFaceField::ChinDepth),
            SettingKey::FaceSize => Source::Face(IngameFaceField::FaceSize),
            SettingKey::FaceHeight => Source::Face(IngameFaceField::FaceHeight),
            SettingKey::BrowWidth => Source::Face(IngameFaceField::BrowWidth),
            SettingKey::InnerEyebrowHeight => Source::Face(IngameFaceField::InnerEyebrowHeight),
            SettingKey::EyebrowDepth => Source::Face(IngameFaceField::EyebrowDepth),
            SettingKey::TempleWidth => Source::Face(IngameFaceField::TempleWidth),
            SettingKey::OuterEyebrowHeight => Source::Face(IngameFaceField::OuterEyebrowHeight),
            SettingKey::PupilSize => Source::Face(IngameFaceField::PupilSize),
            SettingKey::EyeDepth => Source::Face(IngameFaceField::EyeDepth),
            SettingKey::InnerEyePosition => Source::Face(IngameFaceField::InnerEyePosition),
            SettingKey::InnerEyeHeight => Source::Face(IngameFaceField::InnerEyeHeight),
            SettingKey::OuterEyePosition => Source::Face(IngameFaceField::OuterEyePosition),
            SettingKey::EyeCornerHeight => Source::Face(IngameFaceField::EyeCornerHeight),
            SettingKey::UpperEyelidWidthInner => {
                Source::Face(IngameFaceField::UpperEyelidWidthInner)
            }
            SettingKey::UpperEyelidHeightInner => {
                Source::Face(IngameFaceField::UpperEyelidHeightInner)
            }
            SettingKey::UpperEyelidWidthOuter => {
                Source::Face(IngameFaceField::UpperEyelidWidthOuter)
            }
            SettingKey::UpperEyelidHeightOuter => {
                Source::Face(IngameFaceField::UpperEyelidHeightOuter)
            }
            SettingKey::BottomEyelidHeight => Source::Face(IngameFaceField::BottomEyelidHeight),
            SettingKey::HorizontalEyePosition => {
                Source::Face(IngameFaceField::HorizontalEyePosition)
            }
            SettingKey::EyeHeight => Source::Face(IngameFaceField::EyeHeight),
            SettingKey::NoseHeight => Source::Face(IngameFaceField::NoseHeight),
            SettingKey::NoseTipDepth => Source::Face(IngameFaceField::NoseTipDepth),
            SettingKey::NostrilWidth => Source::Face(IngameFaceField::NostrilWidth),
            SettingKey::NoseDepth => Source::Face(IngameFaceField::NoseDepth),
            SettingKey::NoseWidth => Source::Face(IngameFaceField::NoseWidth),
            SettingKey::MouthHeight => Source::Face(IngameFaceField::MouthHeight),
            SettingKey::MouthDepth => Source::Face(IngameFaceField::MouthDepth),
            SettingKey::LipSize => Source::Face(IngameFaceField::LipSize),
            SettingKey::MouthCornerHeight => Source::Face(IngameFaceField::MouthCornerHeight),
            SettingKey::LipWidth => Source::Face(IngameFaceField::LipWidth),
            SettingKey::EarLength => Source::Face(IngameFaceField::EarLength),
            SettingKey::EarWidth => Source::Face(IngameFaceField::EarWidth),
            SettingKey::EarAngle => Source::Face(IngameFaceField::EarAngle),
            SettingKey::Pes15JawDepth => Source::Face(IngameFaceField::Pes15JawDepth),
            SettingKey::Pes15CheekboneDepth => Source::Face(IngameFaceField::Pes15CheekboneDepth),
            SettingKey::Pes15MouthEdgePosition => {
                Source::Face(IngameFaceField::Pes15MouthEdgePosition)
            }
            SettingKey::Pes15EyebrowDepth => Source::Face(IngameFaceField::Pes15EyebrowDepth),
            SettingKey::Pes15BottomEyelidPosition => {
                Source::Face(IngameFaceField::Pes15BottomEyelidPosition)
            }
            SettingKey::Pes15NasalBridgePosition => {
                Source::Face(IngameFaceField::Pes15NasalBridgePosition)
            }
            SettingKey::Pes15NasalRootHeight => Source::Face(IngameFaceField::Pes15NasalRootHeight),
            SettingKey::Pes15NoseAngle => Source::NoseAngle,
            SettingKey::EyebrowColorRed => Source::Face(IngameFaceField::EyebrowColorRed),
            SettingKey::EyebrowColorGreen => Source::Face(IngameFaceField::EyebrowColorGreen),
            SettingKey::EyebrowColorBlue => Source::Face(IngameFaceField::EyebrowColorBlue),
            SettingKey::EyebrowStyle => Source::Face(IngameFaceField::EyebrowStyle),
            SettingKey::EyebrowDensity => Source::Face(IngameFaceField::EyebrowDensity),
            SettingKey::EyebrowThickness => Source::Face(IngameFaceField::EyebrowThickness),
            SettingKey::FacialHairColorRed => Source::Face(IngameFaceField::FacialHairColorRed),
            SettingKey::FacialHairColorGreen => Source::Face(IngameFaceField::FacialHairColorGreen),
            SettingKey::FacialHairColorBlue => Source::Face(IngameFaceField::FacialHairColorBlue),
            SettingKey::FacialHairThickness => Source::Face(IngameFaceField::FacialHairThickness),
            SettingKey::Pes15EyeWhite => Source::Face(IngameFaceField::Pes15EyeWhite),
            SettingKey::HairColorRed => Source::Face(IngameFaceField::HairColorRed),
            SettingKey::HairColorGreen => Source::Face(IngameFaceField::HairColorGreen),
            SettingKey::HairColorBlue => Source::Face(IngameFaceField::HairColorBlue),
            SettingKey::HairColorPreset => Source::Face(IngameFaceField::HairColorPreset),
            SettingKey::HairAccessory => Source::Face(IngameFaceField::HairAccessory),
            SettingKey::HairAccessoryColor => Source::Face(IngameFaceField::HairAccessoryColor),
            SettingKey::HairStyle => Source::Face(IngameFaceField::HairStyle),
            SettingKey::HairCroppedStyle => Source::Face(IngameFaceField::HairCroppedStyle),
            SettingKey::HairLength => Source::Face(IngameFaceField::HairLength),
            SettingKey::HairWaveLevel => Source::Face(IngameFaceField::HairWaveLevel),
            SettingKey::HairVariation => Source::Face(IngameFaceField::HairVariation),
            SettingKey::HairFrontStyle => Source::Face(IngameFaceField::HairFrontStyle),
            SettingKey::HairParted => Source::Face(IngameFaceField::HairParted),
            SettingKey::HairSideBackStyle => Source::Face(IngameFaceField::HairSideBackStyle),
            SettingKey::Hairline => Source::Face(IngameFaceField::Hairline),
            SettingKey::HairForeheadWidth => Source::Face(IngameFaceField::HairForeheadWidth),
            SettingKey::Pes15HairLength => Source::Face(IngameFaceField::Pes15HairLength),
            SettingKey::Pes15HairType => Source::Face(IngameFaceField::Pes15HairType),
            SettingKey::Pes15HairVariation => Source::Face(IngameFaceField::Pes15HairVariation),
        }
    }

    /// The save field the key writes when it is a player- or
    /// appearance-record field; `None` for a key of the ingame-face run
    /// (`face_field`), and `None` too for PES 15's nose angle, which no
    /// field alone holds (it is coded into `NoseType`).
    pub fn player_field(self) -> Option<PlayerField> {
        match self.source() {
            Source::Player(field) => Some(field),
            Source::Face(_) | Source::NoseAngle => None,
        }
    }

    /// The save field the key writes when it is a bit run of the
    /// ingame-face run; `None` for a player- or appearance-record key
    /// (`player_field`), and `None` too for PES 15's nose angle, which
    /// `IngameFace::nose` codes into `NoseType` rather than a field of its
    /// own.
    pub fn face_field(self) -> Option<IngameFaceField> {
        match self.source() {
            Source::Face(field) => Some(field),
            Source::Player(_) | Source::NoseAngle => None,
        }
    }

    /// The `Kind` `version` bounds the key to: the widest range narrowed to
    /// the version's own, from the same tables `spec` draws the widest from
    /// (the measured motion tops, the face-type caps, and the custom skin
    /// the PES 15 to 17 games alone have, `custom_skin_available`). `None`
    /// for a key the version does not store (`held_on`): that is a drop with
    /// no finding, not a narrowing. Every key without a per-version table
    /// entry keeps its widest range, which every version holding it shares.
    pub(crate) fn version_kind(self, version: PesVersion) -> Option<Kind> {
        if !held_on(self, version) {
            return None;
        }
        let spec = self.spec();
        Some(match self.source() {
            Source::Player(field) => match motion_top(version, field) {
                // A 1-based motion's shown top is one above the stored; a
                // celebration's stored top shows as is (0 being OFF).
                Some(top) => match spec.kind {
                    Kind::OneBased { .. } => Kind::OneBased { max: top + 1 },
                    Kind::Number { min, .. } => Kind::Number { min, max: top },
                    // `motion_top` covers only the nine motion fields, each
                    // one of the two kinds above.
                    Kind::Signed { .. }
                    | Kind::PlusOne { .. }
                    | Kind::Mapped(_)
                    | Kind::Bool
                    | Kind::Labels(_) => spec.kind,
                },
                None => spec.kind,
            },
            Source::Face(field) => match face_type_cap(version, field) {
                Some(cap) => Kind::Number { min: 0, max: cap },
                None if self == SettingKey::SkinColor && !fpc::custom_skin_available(version) => {
                    Kind::Number { min: 0, max: 6 }
                }
                None => spec.kind,
            },
            Source::NoseAngle => spec.kind,
        })
    }

    /// Whether the key is one `ingame_face.toml` holds: `iris_color` or a
    /// `[face]`/`[hair]` key, the in-game face's run. Every walk that
    /// visits a document's keys decides through this.
    pub(crate) fn is_ingame_face(self) -> bool {
        self == SettingKey::IrisColor
            || matches!(self.spec().table, KeyTable::Face | KeyTable::Hair)
    }
}

fn physique(name: &'static str) -> KeySpec {
    KeySpec {
        table: KeyTable::Physique,
        name,
        kind: Kind::Signed { limit: 7 },
        comment: SEVEN,
        // The block's `0`, stored +7.
        default: Some(7),
    }
}

fn bool_key(name: &'static str, comment: &'static str, default: bool) -> KeySpec {
    KeySpec {
        table: KeyTable::Strip,
        name,
        kind: Kind::Bool,
        comment,
        default: Some(u8::from(default)),
    }
}

fn seven_color(name: &'static str) -> KeySpec {
    KeySpec {
        table: KeyTable::Strip,
        name,
        kind: Kind::Number { min: 0, max: 7 },
        comment: "0 to 7",
        default: Some(0),
    }
}

/// The widest stored top of `field` over the versions, from the schema's
/// measured tops table, so the numbers have one home.
fn widest_motion(field: PlayerField) -> u8 {
    PesVersion::ALL
        .iter()
        .filter_map(|version| motion_top(*version, field))
        .max()
        .expect("a motion field has a top")
}

fn motion_1based(name: &'static str, field: PlayerField, comment: &'static str) -> KeySpec {
    KeySpec {
        table: KeyTable::Motion,
        name,
        // The shown top is one above the stored.
        kind: Kind::OneBased {
            max: widest_motion(field) + 1,
        },
        comment,
        // The block's `1`, stored -1.
        default: Some(0),
    }
}

fn celebration(name: &'static str, field: PlayerField) -> KeySpec {
    KeySpec {
        table: KeyTable::Motion,
        name,
        kind: Kind::Number {
            min: 0,
            max: widest_motion(field),
        },
        comment: "0 none, then PES 15: 1 to 112, 16 and 17: 1 to 122, 18: 1 to 127, 19: 1 to 140, 20: 1 to 152, 21: 1 to 162",
        default: Some(0),
    }
}

fn face(name: &'static str, field: IngameFaceField, comment: &'static str) -> KeySpec {
    // The widest range is the schema's caps table at its maximum over the
    // versions, so the numbers have one home.
    let max = PesVersion::ALL
        .iter()
        .filter_map(|version| face_type_cap(*version, field))
        .max()
        .expect("a face-type field has a cap");
    KeySpec {
        table: KeyTable::Face,
        name,
        kind: Kind::Number { min: 0, max },
        comment,
        default: Some(0),
    }
}
