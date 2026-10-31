//! The bit runs inside the ingame-face run, as `FieldSpec<IngameFaceField>`
//! rows (the one place these offsets appear), and the run's measured-unused
//! bits. The offsets are the appearance block's bytes 22 onward, relative to
//! the run's first byte. Every bit was measured in the games
//! (`spec/pes_savefile/model.md`, "The ingame-face run").

use pes_version::PesVersion;

use super::FieldSpec;

/// The two layouts of the run: PES 15's 46 bytes and PES 16 to 21's 50. They
/// share the face part, the colours and the accessory bits; PES 15 stores its
/// hair (and its nose angle) differently and has no bytes 46 to 49.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceLayout {
    /// PES 15.
    Pes15,
    /// PES 16 to 21.
    Pes16To21,
}

impl FaceLayout {
    /// The layout of `version`'s run.
    pub fn of(version: PesVersion) -> Self {
        match version {
            PesVersion::Pes15 => FaceLayout::Pes15,
            PesVersion::Pes16
            | PesVersion::Pes17
            | PesVersion::Pes18
            | PesVersion::Pes19
            | PesVersion::Pes20
            | PesVersion::Pes21 => FaceLayout::Pes16To21,
        }
    }
}

/// A bit run inside the ingame-face run. Offsets are relative to the run's
/// first byte (byte 22 of the appearance block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IngameFaceField {
    /// Player (outfield) gloves, byte 0 bit 0.
    PlayerGloves,
    /// Player gloves colour, byte 0 bit 1.
    PlayerGlovesColor,
    /// Jawline, byte 0 bit 4.
    Jawline,
    /// Jaw height, byte 1 bit 0.
    JawHeight,
    /// Jaw depth, byte 1 bit 4. Drawn by PES 15 only.
    Pes15JawDepth,
    /// Cheekbones, byte 2 bit 0.
    Cheekbones,
    /// Cheekbone height, byte 2 bit 4. PES 15's slider; PES 16 to 21 have none but draw it.
    CheekboneHeight,
    /// Cheekbone depth, byte 3 bit 0. Drawn by PES 15 only.
    Pes15CheekboneDepth,
    /// Brow width, byte 4 bit 4.
    BrowWidth,
    /// Inner eyebrow height, byte 5 bit 0.
    InnerEyebrowHeight,
    /// Eyebrow depth, byte 5 bit 4.
    EyebrowDepth,
    /// Temple width, byte 6 bit 0.
    TempleWidth,
    /// Outer eyebrow height, byte 6 bit 4.
    OuterEyebrowHeight,
    /// Eyebrow depth, byte 7 bit 0. Drawn by PES 15 only.
    Pes15EyebrowDepth,
    /// Pupil size, byte 7 bit 4.
    PupilSize,
    /// Eye depth, byte 8 bit 0.
    EyeDepth,
    /// Inner eye position, byte 8 bit 4.
    InnerEyePosition,
    /// Inner eye height, byte 9 bit 0.
    InnerEyeHeight,
    /// Outer eye position, byte 9 bit 4.
    OuterEyePosition,
    /// Eye corner height, byte 10 bit 0.
    EyeCornerHeight,
    /// Upper eyelid width inner, byte 10 bit 4.
    UpperEyelidWidthInner,
    /// Upper eyelid height inner, byte 11 bit 0.
    UpperEyelidHeightInner,
    /// Upper eyelid width outer, byte 11 bit 4.
    UpperEyelidWidthOuter,
    /// Upper eyelid height outer, byte 12 bit 0.
    UpperEyelidHeightOuter,
    /// Bottom eyelid position, byte 12 bit 4. Drawn by PES 15 only.
    Pes15BottomEyelidPosition,
    /// Bottom eyelid height, byte 13 bit 0.
    BottomEyelidHeight,
    /// Horizontal eye position, byte 13 bit 4.
    HorizontalEyePosition,
    /// Eye height, byte 14 bit 0.
    EyeHeight,
    /// Nose height, byte 14 bit 4.
    NoseHeight,
    /// Nose tip depth, byte 15 bit 0.
    NoseTipDepth,
    /// Nostril width, byte 15 bit 4.
    NostrilWidth,
    /// Nasal bridge position, byte 16 bit 0. Drawn by PES 15 only.
    Pes15NasalBridgePosition,
    /// Nose depth, byte 16 bit 4.
    NoseDepth,
    /// Nose width, byte 17 bit 0.
    NoseWidth,
    /// Nasal root height, byte 17 bit 4. Drawn by PES 15 only.
    Pes15NasalRootHeight,
    /// Mouth height, byte 18 bit 0.
    MouthHeight,
    /// Mouth depth, byte 18 bit 4.
    MouthDepth,
    /// Lip size, byte 19 bit 0.
    LipSize,
    /// Mouth corner height, byte 20 bit 0.
    MouthCornerHeight,
    /// Lip width, byte 20 bit 4.
    LipWidth,
    /// Mouth edge position, byte 21 bit 0. Drawn by PES 15 only.
    Pes15MouthEdgePosition,
    /// Chin width, byte 21 bit 4.
    ChinWidth,
    /// Chin height, byte 22 bit 0.
    ChinHeight,
    /// Chin depth, byte 22 bit 4.
    ChinDepth,
    /// Skin colour, byte 23 bit 0.
    SkinColor,
    /// Cheek type, byte 23 bit 3.
    CheekType,
    /// Forehead type, byte 24 bit 0.
    ForeheadType,
    /// Facial hair type, byte 24 bit 3.
    FacialHairType,
    /// Laughter lines type, byte 25 bit 0.
    LaughterLinesType,
    /// Upper eyelid type, byte 25 bit 3.
    UpperEyelidType,
    /// Lower eyelid type, byte 26 bit 0.
    LowerEyelidType,
    /// Ear length, byte 26 bit 4.
    EarLength,
    /// Ear width, byte 27 bit 0.
    EarWidth,
    /// Ear angle, byte 27 bit 4.
    EarAngle,
    /// Eyebrow type, byte 28 bit 0.
    EyebrowType,
    /// Neck line type, byte 28 bit 5.
    NeckLineType,
    /// Face size, byte 29 bit 0.
    FaceSize,
    /// Face height, byte 29 bit 4.
    FaceHeight,
    /// Nose type, byte 30 bit 0. On PES 15 it holds 3 × type + the nose angle's code (`IngameFace::nose`).
    NoseType,
    /// Upper lip type, byte 31 bit 0.
    UpperLipType,
    /// Lower lip type, byte 31 bit 3.
    LowerLipType,
    /// Hair colour red, byte 31 bit 6.
    HairColorRed,
    /// Hair colour green, byte 32 bit 4.
    HairColorGreen,
    /// Hair colour blue, byte 33 bit 2.
    HairColorBlue,
    /// Hair variation, PES 15 only; byte 34 bit 0. The editor's grid position minus 1.
    Pes15HairVariation,
    /// Hair style, PES 16 to 21 only; byte 34 bit 0.
    HairStyle,
    /// Hair cropped style, PES 16 to 21 only; byte 34 bit 4.
    HairCroppedStyle,
    /// Eyebrow colour red, byte 35 bit 0.
    EyebrowColorRed,
    /// Eyebrow style, byte 35 bit 6.
    EyebrowStyle,
    /// Eyebrow colour green, byte 36 bit 0.
    EyebrowColorGreen,
    /// Eyebrow density, byte 36 bit 6.
    EyebrowDensity,
    /// Eyebrow colour blue, byte 37 bit 0.
    EyebrowColorBlue,
    /// Eyebrow thickness, byte 37 bit 6.
    EyebrowThickness,
    /// Hair length, PES 15 only; byte 38 bit 0.
    Pes15HairLength,
    /// Hair length, PES 16 to 21 only; byte 38 bit 0.
    HairLength,
    /// Hair type, PES 15 only; byte 38 bit 3.
    Pes15HairType,
    /// Hair wave level, PES 16 to 21 only; byte 38 bit 3. Stored as the level plus 1.
    HairWaveLevel,
    /// Eye white, byte 38 bit 6. Drawn by PES 15 only. Brighter eye whites, 0 to 3.
    Pes15EyeWhite,
    /// Hair accessory, byte 39 bit 0.
    HairAccessory,
    /// Hair accessory colour, byte 39 bit 1.
    HairAccessoryColor,
    /// Facial hair colour red, byte 39 bit 4.
    FacialHairColorRed,
    /// Facial hair colour green, byte 40 bit 2.
    FacialHairColorGreen,
    /// Facial hair colour blue, byte 41 bit 0.
    FacialHairColorBlue,
    /// Facial hair thickness, byte 41 bit 6.
    FacialHairThickness,
    /// Iris colour, byte 42 bit 0.
    IrisColor,
    /// Hair variation, PES 16 to 21 only; byte 42 bit 4.
    HairVariation,
    /// Hair colour preset, byte 43 bit 4. 0 custom, else the game's internal preset id.
    HairColorPreset,
    /// Hair front style, PES 16 to 21 only; byte 44 bit 0.
    HairFrontStyle,
    /// Hair parted, PES 16 to 21 only; byte 44 bit 3.
    HairParted,
    /// Hair side back style, PES 16 to 21 only; byte 44 bit 6.
    HairSideBackStyle,
    /// Hairline, PES 16 to 21 only; byte 45 bit 1.
    Hairline,
    /// Hair forehead width, PES 16 to 21 only; byte 45 bit 4.
    HairForeheadWidth,
}

impl IngameFaceField {
    /// Every variant, so tests can walk the closed set.
    pub const ALL: [IngameFaceField; 92] = [
        IngameFaceField::PlayerGloves,
        IngameFaceField::PlayerGlovesColor,
        IngameFaceField::Jawline,
        IngameFaceField::JawHeight,
        IngameFaceField::Pes15JawDepth,
        IngameFaceField::Cheekbones,
        IngameFaceField::CheekboneHeight,
        IngameFaceField::Pes15CheekboneDepth,
        IngameFaceField::BrowWidth,
        IngameFaceField::InnerEyebrowHeight,
        IngameFaceField::EyebrowDepth,
        IngameFaceField::TempleWidth,
        IngameFaceField::OuterEyebrowHeight,
        IngameFaceField::Pes15EyebrowDepth,
        IngameFaceField::PupilSize,
        IngameFaceField::EyeDepth,
        IngameFaceField::InnerEyePosition,
        IngameFaceField::InnerEyeHeight,
        IngameFaceField::OuterEyePosition,
        IngameFaceField::EyeCornerHeight,
        IngameFaceField::UpperEyelidWidthInner,
        IngameFaceField::UpperEyelidHeightInner,
        IngameFaceField::UpperEyelidWidthOuter,
        IngameFaceField::UpperEyelidHeightOuter,
        IngameFaceField::Pes15BottomEyelidPosition,
        IngameFaceField::BottomEyelidHeight,
        IngameFaceField::HorizontalEyePosition,
        IngameFaceField::EyeHeight,
        IngameFaceField::NoseHeight,
        IngameFaceField::NoseTipDepth,
        IngameFaceField::NostrilWidth,
        IngameFaceField::Pes15NasalBridgePosition,
        IngameFaceField::NoseDepth,
        IngameFaceField::NoseWidth,
        IngameFaceField::Pes15NasalRootHeight,
        IngameFaceField::MouthHeight,
        IngameFaceField::MouthDepth,
        IngameFaceField::LipSize,
        IngameFaceField::MouthCornerHeight,
        IngameFaceField::LipWidth,
        IngameFaceField::Pes15MouthEdgePosition,
        IngameFaceField::ChinWidth,
        IngameFaceField::ChinHeight,
        IngameFaceField::ChinDepth,
        IngameFaceField::SkinColor,
        IngameFaceField::CheekType,
        IngameFaceField::ForeheadType,
        IngameFaceField::FacialHairType,
        IngameFaceField::LaughterLinesType,
        IngameFaceField::UpperEyelidType,
        IngameFaceField::LowerEyelidType,
        IngameFaceField::EarLength,
        IngameFaceField::EarWidth,
        IngameFaceField::EarAngle,
        IngameFaceField::EyebrowType,
        IngameFaceField::NeckLineType,
        IngameFaceField::FaceSize,
        IngameFaceField::FaceHeight,
        IngameFaceField::NoseType,
        IngameFaceField::UpperLipType,
        IngameFaceField::LowerLipType,
        IngameFaceField::HairColorRed,
        IngameFaceField::HairColorGreen,
        IngameFaceField::HairColorBlue,
        IngameFaceField::Pes15HairVariation,
        IngameFaceField::HairStyle,
        IngameFaceField::HairCroppedStyle,
        IngameFaceField::EyebrowColorRed,
        IngameFaceField::EyebrowStyle,
        IngameFaceField::EyebrowColorGreen,
        IngameFaceField::EyebrowDensity,
        IngameFaceField::EyebrowColorBlue,
        IngameFaceField::EyebrowThickness,
        IngameFaceField::Pes15HairLength,
        IngameFaceField::HairLength,
        IngameFaceField::Pes15HairType,
        IngameFaceField::HairWaveLevel,
        IngameFaceField::Pes15EyeWhite,
        IngameFaceField::HairAccessory,
        IngameFaceField::HairAccessoryColor,
        IngameFaceField::FacialHairColorRed,
        IngameFaceField::FacialHairColorGreen,
        IngameFaceField::FacialHairColorBlue,
        IngameFaceField::FacialHairThickness,
        IngameFaceField::IrisColor,
        IngameFaceField::HairVariation,
        IngameFaceField::HairColorPreset,
        IngameFaceField::HairFrontStyle,
        IngameFaceField::HairParted,
        IngameFaceField::HairSideBackStyle,
        IngameFaceField::Hairline,
        IngameFaceField::HairForeheadWidth,
    ];

    /// The layout the field exists in; `None` on every version. An
    /// exhaustive match, so a new variant does not compile until placed.
    pub(crate) fn layout(self) -> Option<FaceLayout> {
        match self {
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
            | IngameFaceField::EyebrowColorRed
            | IngameFaceField::EyebrowStyle
            | IngameFaceField::EyebrowColorGreen
            | IngameFaceField::EyebrowDensity
            | IngameFaceField::EyebrowColorBlue
            | IngameFaceField::EyebrowThickness
            | IngameFaceField::Pes15EyeWhite
            | IngameFaceField::HairAccessory
            | IngameFaceField::HairAccessoryColor
            | IngameFaceField::FacialHairColorRed
            | IngameFaceField::FacialHairColorGreen
            | IngameFaceField::FacialHairColorBlue
            | IngameFaceField::FacialHairThickness
            | IngameFaceField::IrisColor
            | IngameFaceField::HairColorPreset => None,
            IngameFaceField::Pes15HairVariation
            | IngameFaceField::Pes15HairLength
            | IngameFaceField::Pes15HairType => Some(FaceLayout::Pes15),
            IngameFaceField::HairStyle
            | IngameFaceField::HairCroppedStyle
            | IngameFaceField::HairLength
            | IngameFaceField::HairWaveLevel
            | IngameFaceField::HairVariation
            | IngameFaceField::HairFrontStyle
            | IngameFaceField::HairParted
            | IngameFaceField::HairSideBackStyle
            | IngameFaceField::Hairline
            | IngameFaceField::HairForeheadWidth => Some(FaceLayout::Pes16To21),
        }
    }
}

/// Every field of the run, each existing on the versions `layout()` names.
/// A list field is as wide as PES 15 uses it; on PES 16 to 21 its extra high
/// bits are 0 in every face the games made (set, they draw past the shipped
/// models), which the settings ranges keep.
pub const INGAME_FACE_FIELDS: &[FieldSpec<IngameFaceField>] = &[
    FieldSpec {
        field: IngameFaceField::PlayerGloves,
        bit_offset: 0,
        bit_width: 1,
    },
    FieldSpec {
        field: IngameFaceField::PlayerGlovesColor,
        bit_offset: 1,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::Jawline,
        bit_offset: 4,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::JawHeight,
        bit_offset: 8,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15JawDepth,
        bit_offset: 12,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Cheekbones,
        bit_offset: 16,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::CheekboneHeight,
        bit_offset: 20,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15CheekboneDepth,
        bit_offset: 24,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::BrowWidth,
        bit_offset: 36,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::InnerEyebrowHeight,
        bit_offset: 40,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowDepth,
        bit_offset: 44,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::TempleWidth,
        bit_offset: 48,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::OuterEyebrowHeight,
        bit_offset: 52,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15EyebrowDepth,
        bit_offset: 56,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::PupilSize,
        bit_offset: 60,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyeDepth,
        bit_offset: 64,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::InnerEyePosition,
        bit_offset: 68,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::InnerEyeHeight,
        bit_offset: 72,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::OuterEyePosition,
        bit_offset: 76,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyeCornerHeight,
        bit_offset: 80,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::UpperEyelidWidthInner,
        bit_offset: 84,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::UpperEyelidHeightInner,
        bit_offset: 88,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::UpperEyelidWidthOuter,
        bit_offset: 92,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::UpperEyelidHeightOuter,
        bit_offset: 96,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15BottomEyelidPosition,
        bit_offset: 100,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::BottomEyelidHeight,
        bit_offset: 104,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::HorizontalEyePosition,
        bit_offset: 108,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyeHeight,
        bit_offset: 112,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NoseHeight,
        bit_offset: 116,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NoseTipDepth,
        bit_offset: 120,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NostrilWidth,
        bit_offset: 124,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15NasalBridgePosition,
        bit_offset: 128,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NoseDepth,
        bit_offset: 132,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NoseWidth,
        bit_offset: 136,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15NasalRootHeight,
        bit_offset: 140,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::MouthHeight,
        bit_offset: 144,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::MouthDepth,
        bit_offset: 148,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::LipSize,
        bit_offset: 152,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::MouthCornerHeight,
        bit_offset: 160,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::LipWidth,
        bit_offset: 164,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::Pes15MouthEdgePosition,
        bit_offset: 168,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::ChinWidth,
        bit_offset: 172,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::ChinHeight,
        bit_offset: 176,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::ChinDepth,
        bit_offset: 180,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::SkinColor,
        bit_offset: 184,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::CheekType,
        bit_offset: 187,
        bit_width: 5,
    },
    FieldSpec {
        field: IngameFaceField::ForeheadType,
        bit_offset: 192,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::FacialHairType,
        bit_offset: 195,
        bit_width: 5,
    },
    FieldSpec {
        field: IngameFaceField::LaughterLinesType,
        bit_offset: 200,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::UpperEyelidType,
        bit_offset: 203,
        bit_width: 5,
    },
    FieldSpec {
        field: IngameFaceField::LowerEyelidType,
        bit_offset: 208,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EarLength,
        bit_offset: 212,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EarWidth,
        bit_offset: 216,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EarAngle,
        bit_offset: 220,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowType,
        bit_offset: 224,
        bit_width: 5,
    },
    FieldSpec {
        field: IngameFaceField::NeckLineType,
        bit_offset: 229,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::FaceSize,
        bit_offset: 232,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::FaceHeight,
        bit_offset: 236,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::NoseType,
        bit_offset: 240,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::UpperLipType,
        bit_offset: 248,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::LowerLipType,
        bit_offset: 251,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairColorRed,
        bit_offset: 254,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::HairColorGreen,
        bit_offset: 260,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::HairColorBlue,
        bit_offset: 266,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::Pes15HairVariation,
        bit_offset: 272,
        bit_width: 8,
    },
    FieldSpec {
        field: IngameFaceField::HairStyle,
        bit_offset: 272,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::HairCroppedStyle,
        bit_offset: 276,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowColorRed,
        bit_offset: 280,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowStyle,
        bit_offset: 286,
        bit_width: 2,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowColorGreen,
        bit_offset: 288,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowDensity,
        bit_offset: 294,
        bit_width: 2,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowColorBlue,
        bit_offset: 296,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::EyebrowThickness,
        bit_offset: 302,
        bit_width: 2,
    },
    FieldSpec {
        field: IngameFaceField::Pes15HairLength,
        bit_offset: 304,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairLength,
        bit_offset: 304,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::Pes15HairType,
        bit_offset: 307,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairWaveLevel,
        bit_offset: 307,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::Pes15EyeWhite,
        bit_offset: 310,
        bit_width: 2,
    },
    FieldSpec {
        field: IngameFaceField::HairAccessory,
        bit_offset: 312,
        bit_width: 1,
    },
    FieldSpec {
        field: IngameFaceField::HairAccessoryColor,
        bit_offset: 313,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::FacialHairColorRed,
        bit_offset: 316,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::FacialHairColorGreen,
        bit_offset: 322,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::FacialHairColorBlue,
        bit_offset: 328,
        bit_width: 6,
    },
    FieldSpec {
        field: IngameFaceField::FacialHairThickness,
        bit_offset: 334,
        bit_width: 2,
    },
    FieldSpec {
        field: IngameFaceField::IrisColor,
        bit_offset: 336,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::HairVariation,
        bit_offset: 340,
        bit_width: 8,
    },
    FieldSpec {
        field: IngameFaceField::HairColorPreset,
        bit_offset: 348,
        bit_width: 4,
    },
    FieldSpec {
        field: IngameFaceField::HairFrontStyle,
        bit_offset: 352,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairParted,
        bit_offset: 355,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairSideBackStyle,
        bit_offset: 358,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::Hairline,
        bit_offset: 361,
        bit_width: 3,
    },
    FieldSpec {
        field: IngameFaceField::HairForeheadWidth,
        bit_offset: 364,
        bit_width: 4,
    },
];

/// A measured-unused bit run: no setting moves it and no render shows it. A
/// compile writes it at `value`, what the games' own editors write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnusedBits {
    /// Bit position from the run's first byte.
    pub(crate) bit_offset: u32,
    /// Run length in bits (at most 8).
    pub(crate) bit_width: u32,
    /// The value a compile writes.
    pub(crate) value: u8,
    /// The layout the run exists in; `None` on every version.
    pub(crate) layout: Option<FaceLayout>,
}

/// The run's measured-unused bits.
pub(crate) const UNUSED_FACE_BITS: &[UnusedBits] = &[
    UnusedBits {
        bit_offset: 28,
        bit_width: 4,
        value: 0x07,
        layout: None,
    },
    UnusedBits {
        bit_offset: 32,
        bit_width: 4,
        value: 0x07,
        layout: None,
    },
    UnusedBits {
        bit_offset: 156,
        bit_width: 4,
        value: 0x07,
        layout: None,
    },
    UnusedBits {
        bit_offset: 246,
        bit_width: 2,
        value: 0x01,
        layout: None,
    },
    UnusedBits {
        bit_offset: 368,
        bit_width: 8,
        value: 0x00,
        layout: Some(FaceLayout::Pes16To21),
    },
    UnusedBits {
        bit_offset: 376,
        bit_width: 8,
        value: 0x00,
        layout: Some(FaceLayout::Pes16To21),
    },
    UnusedBits {
        bit_offset: 384,
        bit_width: 8,
        value: 0x00,
        layout: Some(FaceLayout::Pes16To21),
    },
    UnusedBits {
        bit_offset: 392,
        bit_width: 8,
        value: 0x00,
        layout: Some(FaceLayout::Pes16To21),
    },
    UnusedBits {
        bit_offset: 340,
        bit_width: 8,
        value: 0x77,
        layout: Some(FaceLayout::Pes15),
    },
    UnusedBits {
        bit_offset: 352,
        bit_width: 8,
        value: 0x00,
        layout: Some(FaceLayout::Pes15),
    },
    UnusedBits {
        bit_offset: 360,
        bit_width: 8,
        value: 0x37,
        layout: Some(FaceLayout::Pes15),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_covers_every_variant_exactly_once() {
        assert_eq!(INGAME_FACE_FIELDS.len(), IngameFaceField::ALL.len());
        for field in IngameFaceField::ALL {
            assert_eq!(
                INGAME_FACE_FIELDS
                    .iter()
                    .filter(|s| s.field == field)
                    .count(),
                1,
                "{field:?} has not exactly one row"
            );
        }
        for spec in INGAME_FACE_FIELDS {
            assert!(
                IngameFaceField::ALL.contains(&spec.field),
                "{:?} is a row outside ALL",
                spec.field
            );
        }
    }

    /// Each layout's run is covered bit for bit by exactly one field row or
    /// one unused row existing in that layout.
    #[test]
    fn each_layout_s_run_is_covered_once_per_bit() {
        for (layout, len) in [(FaceLayout::Pes15, 46), (FaceLayout::Pes16To21, 50)] {
            let present = |row: Option<FaceLayout>| row.is_none_or(|l| l == layout);
            let mut covers = vec![Vec::new(); len * 8];
            for spec in INGAME_FACE_FIELDS
                .iter()
                .filter(|s| present(s.field.layout()))
            {
                for bit in spec.bit_offset..spec.bit_offset + spec.bit_width {
                    covers[bit as usize].push(format!("{:?}", spec.field));
                }
            }
            for unused in UNUSED_FACE_BITS.iter().filter(|u| present(u.layout)) {
                assert!(unused.bit_width <= 8, "{unused:?} wider than its value");
                for bit in unused.bit_offset..unused.bit_offset + unused.bit_width {
                    covers[bit as usize].push(format!("unused at {}", unused.bit_offset));
                }
            }
            for (bit, carriers) in covers.iter().enumerate() {
                assert_eq!(carriers.len(), 1, "{layout:?} bit {bit}: {carriers:?}");
            }
        }
    }
}
