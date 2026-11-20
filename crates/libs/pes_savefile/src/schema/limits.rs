//! The per-version caps on the ingame-face type fields
//! (`spec/pes_savefile/operations.md`, "Cross-version player conversion",
//! whose table has PES 15's column) and the per-version tops of the motion
//! fields (`spec/aesthetics_export/settings_toml.md` "Player settings in
//! exports", the measured table). A field
//! that is not a face type (a slider, a colour, the hair, gloves, skin, iris)
//! has no cap, and a player field that is not a measured motion has no top:
//! `None`.

use pes_version::PesVersion;

use crate::schema::fields::PlayerField;
use crate::schema::ingame_face::{INGAME_FACE_FIELDS, IngameFaceField};

use IngameFaceField as F;

/// The highest value `field` takes on `version`, `None` for the fields that
/// are not face types. The measured columns are PES 15, PES 16, PES 19's
/// cheek and forehead types, and PES 20-21; a column no game measured (all
/// eleven types on PES 2014, 17 and 18, PES 19's other nine) is the field's
/// bit maximum (`spec/pes_savefile/operations.md` "Cross-version player
/// conversion"). For `NoseType` the cap is on the nose type, which PES 15
/// codes with the nose angle (`IngameFace::nose`), not on the raw field.
pub fn face_type_cap(version: PesVersion, field: IngameFaceField) -> Option<u8> {
    // (PES 15, PES 16, PES 19, PES 20-21): the measured columns. PES 15's
    // column is its editor's range joined with the values the game's own
    // faces use; PES 19's cheek and forehead are measured in its editor —
    // `Some` for those two, `None` (unmeasured) for the rest, as are PES 17
    // and 18's columns.
    let (pes15, pes16, pes19, newer) = match field {
        F::PlayerGloves
        | F::PlayerGlovesColor
        | F::Jawline
        | F::JawHeight
        | F::Pes15JawDepth
        | F::Cheekbones
        | F::CheekboneHeight
        | F::Pes15CheekboneDepth
        | F::BrowWidth
        | F::InnerEyebrowHeight
        | F::EyebrowDepth
        | F::TempleWidth
        | F::OuterEyebrowHeight
        | F::Pes15EyebrowDepth
        | F::PupilSize
        | F::EyeDepth
        | F::InnerEyePosition
        | F::InnerEyeHeight
        | F::OuterEyePosition
        | F::EyeCornerHeight
        | F::UpperEyelidWidthInner
        | F::UpperEyelidHeightInner
        | F::UpperEyelidWidthOuter
        | F::UpperEyelidHeightOuter
        | F::Pes15BottomEyelidPosition
        | F::BottomEyelidHeight
        | F::HorizontalEyePosition
        | F::EyeHeight
        | F::NoseHeight
        | F::NoseTipDepth
        | F::NostrilWidth
        | F::Pes15NasalBridgePosition
        | F::NoseDepth
        | F::NoseWidth
        | F::Pes15NasalRootHeight
        | F::MouthHeight
        | F::MouthDepth
        | F::LipSize
        | F::MouthCornerHeight
        | F::LipWidth
        | F::Pes15MouthEdgePosition
        | F::ChinWidth
        | F::ChinHeight
        | F::ChinDepth
        | F::SkinColor
        | F::EarLength
        | F::EarWidth
        | F::EarAngle
        | F::FaceSize
        | F::FaceHeight
        | F::HairColorRed
        | F::HairColorGreen
        | F::HairColorBlue
        | F::Pes15HairVariation
        | F::HairStyle
        | F::HairCroppedStyle
        | F::EyebrowColorRed
        | F::EyebrowStyle
        | F::EyebrowColorGreen
        | F::EyebrowDensity
        | F::EyebrowColorBlue
        | F::EyebrowThickness
        | F::Pes15HairLength
        | F::HairLength
        | F::Pes15HairType
        | F::HairWaveLevel
        | F::Pes15EyeWhite
        | F::HairAccessory
        | F::HairAccessoryColor
        | F::FacialHairColorRed
        | F::FacialHairColorGreen
        | F::FacialHairColorBlue
        | F::FacialHairThickness
        | F::IrisColor
        | F::HairVariation
        | F::HairColorPreset
        | F::HairFrontStyle
        | F::HairParted
        | F::HairSideBackStyle
        | F::Hairline
        | F::HairForeheadWidth => return None,
        F::CheekType => (14, 3, Some(5), 3),
        F::ForeheadType => (6, 5, Some(6), 5),
        F::FacialHairType => (29, 12, None, 19),
        F::LaughterLinesType => (6, 4, None, 4),
        F::UpperEyelidType => (14, 6, None, 7),
        F::LowerEyelidType => (14, 2, None, 6),
        F::EyebrowType => (17, 5, None, 7),
        F::NeckLineType => (5, 2, None, 3),
        F::NoseType => (11, 6, None, 7),
        F::UpperLipType => (7, 3, None, 4),
        F::LowerLipType => (7, 2, None, 4),
    };
    match version {
        PesVersion::Pes15 => Some(pes15),
        PesVersion::Pes16 => Some(pes16),
        // The unmeasured columns are the field's bit maximum.
        PesVersion::Pes14 | PesVersion::Pes17 | PesVersion::Pes18 => bit_max(field),
        PesVersion::Pes19 => pes19.or_else(|| bit_max(field)),
        PesVersion::Pes20 | PesVersion::Pes21 => Some(newer),
    }
}

/// The field's bit maximum (`2^bit_width - 1`) in `INGAME_FACE_FIELDS`: the
/// cap of a version column no game measured — the biggest value the field
/// can store.
fn bit_max(field: IngameFaceField) -> Option<u8> {
    let width = INGAME_FACE_FIELDS
        .iter()
        .find(|spec| spec.field == field)
        .expect("the ingame-face table covers every variant")
        .bit_width;
    Some(u8::MAX >> (u8::BITS - width))
}

/// The highest stored value of motion field `field` on `version`, `None` for the fields that
/// are not motions with a measured range (`Dribbling` included). The 1-based motions store
/// one below what the editor shows; the goal celebrations are stored as shown, 0 being OFF.
pub fn motion_top(version: PesVersion, field: PlayerField) -> Option<u8> {
    // (PES 15, PES 16, PES 17, PES 18, PES 19, PES 20, PES 21): the spec's
    // measured table, in stored values.
    let (pes15, pes16, pes17, pes18, pes19, pes20, pes21) = match field {
        PlayerField::HunchingDribbling | PlayerField::HunchingRunning => (2, 2, 2, 2, 2, 4, 4),
        PlayerField::ArmMovementDribbling | PlayerField::ArmMovementRunning => {
            (4, 7, 7, 7, 7, 9, 9)
        }
        PlayerField::CornerKickMotion => (5, 5, 5, 5, 5, 9, 9),
        PlayerField::FreeKickMotion => (15, 15, 15, 15, 15, 19, 19),
        PlayerField::PenaltyKickMotion => (3, 3, 3, 3, 3, 6, 6),
        PlayerField::GoalCelebration1 | PlayerField::GoalCelebration2 => {
            (112, 122, 122, 127, 140, 152, 162)
        }
        PlayerField::Id
        | PlayerField::Nationality
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
        | PlayerField::EditedPlayer
        | PlayerField::Swerve
        | PlayerField::Catching
        | PlayerField::Clearing
        | PlayerField::Reflexes
        | PlayerField::InjuryResistance
        | PlayerField::EditedBasicSettings
        | PlayerField::BodyControl
        | PlayerField::PhysicalContact
        | PlayerField::KickingPower
        | PlayerField::ExplosivePower
        | PlayerField::EditedRegisteredPosition
        | PlayerField::Age
        | PlayerField::RegisteredPosition
        | PlayerField::PlayingStyle
        | PlayerField::BallControl
        | PlayerField::BallWinning
        | PlayerField::WeakFootAccuracy
        | PlayerField::Jump
        | PlayerField::Coverage
        | PlayerField::WeakFootUsage
        | PlayerField::PlayablePosition(_)
        | PlayerField::PlaceKicking
        | PlayerField::EditedPlayablePositions
        | PlayerField::EditedAbilities
        | PlayerField::EditedSkills
        | PlayerField::Stamina
        | PlayerField::Speed
        | PlayerField::EditedPlayingStyle
        | PlayerField::EditedComStyles
        | PlayerField::EditedMotion
        | PlayerField::BaseCopy
        | PlayerField::StrongerFoot
        | PlayerField::ComStyle(_)
        | PlayerField::Skill(_)
        | PlayerField::Star
        | PlayerField::DribblingMotion
        | PlayerField::TightPossession
        | PlayerField::Aggression
        | PlayerField::PlayingAttitude
        | PlayerField::StrongerHand
        | PlayerField::AppearanceId
        | PlayerField::EditedFace
        | PlayerField::EditedHair
        | PlayerField::EditedPhysique
        | PlayerField::EditedStrip
        | PlayerField::BootsId
        | PlayerField::GlovesId
        | PlayerField::BaseCopyId
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
        | PlayerField::AnkleTaping => return None,
    };
    match version {
        // PES 2015's until PES 2014's tops are measured.
        PesVersion::Pes14 => Some(pes15),
        PesVersion::Pes15 => Some(pes15),
        PesVersion::Pes16 => Some(pes16),
        PesVersion::Pes17 => Some(pes17),
        PesVersion::Pes18 => Some(pes18),
        PesVersion::Pes19 => Some(pes19),
        PesVersion::Pes20 => Some(pes20),
        PesVersion::Pes21 => Some(pes21),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use PesVersion as V;

    /// The eleven face-type fields, in `IngameFaceField::ALL` order.
    const TYPES: [IngameFaceField; 11] = [
        F::CheekType,
        F::ForeheadType,
        F::FacialHairType,
        F::LaughterLinesType,
        F::UpperEyelidType,
        F::LowerEyelidType,
        F::EyebrowType,
        F::NeckLineType,
        F::NoseType,
        F::UpperLipType,
        F::LowerLipType,
    ];

    #[test]
    fn the_non_type_fields_have_no_cap_the_eleven_types_do() {
        for version in PesVersion::ALL {
            for field in IngameFaceField::ALL {
                let cap = face_type_cap(version, field);
                if TYPES.contains(&field) {
                    assert!(cap.is_some(), "{version:?} {field:?}");
                } else {
                    assert_eq!(cap, None, "{version:?} {field:?}");
                }
            }
        }
    }

    /// The spec's table (`spec/pes_savefile/operations.md` "Cross-version
    /// player conversion"), as literals, in `TYPES` order: the measured
    /// columns PES 15, PES 16, PES 19's own (cheek and forehead, `None`
    /// where unmeasured) and PES 20-21's, and the unmeasured column of
    /// PES 17, 18 and 19's other nine — the fields' bit maxima read from
    /// `INGAME_FACE_FIELDS` (5 bits → 31, 4 → 15, 3 → 7, 6 → 63).
    const GOLDEN: [(u8, u8, u8, Option<u8>, u8); 11] = [
        (14, 3, 31, Some(5), 3),
        (6, 5, 7, Some(6), 5),
        (29, 12, 31, None, 19),
        (6, 4, 7, None, 4),
        (14, 6, 31, None, 7),
        (14, 2, 15, None, 6),
        (17, 5, 31, None, 7),
        (5, 2, 7, None, 3),
        (11, 6, 63, None, 7),
        (7, 3, 7, None, 4),
        (7, 2, 7, None, 4),
    ];

    #[test]
    fn the_caps_are_the_spec_s_values() {
        for (field, (pes15, pes16, bits, pes19, newer)) in TYPES.into_iter().zip(GOLDEN) {
            assert_eq!(
                face_type_cap(V::Pes15, field),
                Some(pes15),
                "Pes15 {field:?}"
            );
            assert_eq!(
                face_type_cap(V::Pes16, field),
                Some(pes16),
                "Pes16 {field:?}"
            );
            for version in [V::Pes17, V::Pes18] {
                assert_eq!(
                    face_type_cap(version, field),
                    Some(bits),
                    "{version:?} {field:?}"
                );
            }
            assert_eq!(
                face_type_cap(V::Pes19, field),
                Some(pes19.unwrap_or(bits)),
                "Pes19 {field:?}"
            );
            for version in [V::Pes20, V::Pes21] {
                assert_eq!(
                    face_type_cap(version, field),
                    Some(newer),
                    "{version:?} {field:?}"
                );
            }
        }
    }

    /// PES 19's own column: its editor offers cheek to 5 and forehead to 6;
    /// PES 18's unmeasured columns are the fields' bit maxima instead.
    #[test]
    fn pes19s_measured_cheek_and_forehead() {
        assert_eq!(face_type_cap(V::Pes19, F::CheekType), Some(5));
        assert_eq!(face_type_cap(V::Pes19, F::ForeheadType), Some(6));
        assert_eq!(face_type_cap(V::Pes18, F::CheekType), Some(31));
        assert_eq!(face_type_cap(V::Pes18, F::ForeheadType), Some(7));
    }

    /// The nine motion fields a measured range was taken for, in
    /// `settings.toml`'s `[appearance.motion]` order (`dribbling`, the same
    /// 0 to 3 on both versions that hold it, is not one).
    const MOTIONS: [PlayerField; 9] = [
        PlayerField::HunchingDribbling,
        PlayerField::HunchingRunning,
        PlayerField::ArmMovementDribbling,
        PlayerField::ArmMovementRunning,
        PlayerField::CornerKickMotion,
        PlayerField::FreeKickMotion,
        PlayerField::PenaltyKickMotion,
        PlayerField::GoalCelebration1,
        PlayerField::GoalCelebration2,
    ];

    /// The spec's measured tops as the editor shows them
    /// (`spec/aesthetics_export/settings_toml.md` "Player settings in
    /// exports"), one row per `MOTIONS` field, one column per version of
    /// `PesVersion::ALL`.
    const SHOWN_TOPS: [[u8; 7]; 9] = [
        [3, 3, 3, 3, 3, 5, 5],
        [3, 3, 3, 3, 3, 5, 5],
        [5, 8, 8, 8, 8, 10, 10],
        [5, 8, 8, 8, 8, 10, 10],
        [6, 6, 6, 6, 6, 10, 10],
        [16, 16, 16, 16, 16, 20, 20],
        [4, 4, 4, 4, 4, 7, 7],
        [112, 122, 122, 127, 140, 152, 162],
        [112, 122, 122, 127, 140, 152, 162],
    ];

    #[test]
    fn the_motion_tops_are_the_spec_s_table_in_stored_values() {
        for (field, shown) in MOTIONS.into_iter().zip(SHOWN_TOPS) {
            // The seven 1-based motions show stored + 1; the two
            // celebrations are stored as shown.
            let one_based = !matches!(
                field,
                PlayerField::GoalCelebration1 | PlayerField::GoalCelebration2
            );
            for (version, top) in PesVersion::ALL.into_iter().zip(shown) {
                let stored = if one_based { top - 1 } else { top };
                assert_eq!(
                    motion_top(version, field),
                    Some(stored),
                    "{version:?} {field:?}"
                );
            }
        }
    }

    /// A newer measured cap is never below PES 16's: the unmeasured columns
    /// in between are the fields' bits, which a measured cap can be under —
    /// the check runs between the two measured neighbours.
    #[test]
    fn the_newer_caps_never_narrow() {
        for field in TYPES {
            let older = face_type_cap(PesVersion::Pes16, field).expect("a type");
            let newer = face_type_cap(PesVersion::Pes20, field).expect("a type");
            assert!(newer >= older, "{field:?}: {older} -> {newer}");
        }
    }

    #[test]
    fn an_unmeasured_motion_and_a_non_motion_have_no_top() {
        for version in PesVersion::ALL {
            for field in [
                PlayerField::DribblingMotion,
                PlayerField::Dribbling,
                PlayerField::Height,
            ] {
                assert_eq!(motion_top(version, field), None, "{version:?} {field:?}");
            }
        }
    }
}
