//! `PlayerSettings::narrow_to`: a value inside its key's widest range but
//! past the compile target's resets to the key's default and reports what
//! `settings_value_out_of_range` needs (the key, the value as the file
//! wrote it, the version's range as the file would write it).

use super::*;
use crate::schema::limits::face_type_cap;

/// A settings whose in-game face is `text`'s, as `ingame_face.toml` holds it.
fn with_face(text: &str) -> PlayerSettings {
    let mut settings = PlayerSettings::default();
    settings.set_ingame_face(IngameFaceSettings::parse(text).expect("parses"));
    settings
}

/// PES 16's measured facial-hair cap is 12; a parsed 15 (inside the widest
/// 31) narrows to the face type's default 0 and reports the
/// `ingame_face.toml` key, the written 15 and the target's "0 to 12".
#[test]
fn a_face_type_over_the_target_s_cap_resets_to_0_and_reports() {
    let mut settings = with_face("[face]\nfacial_hair_type = 15\n");
    let narrowed = settings.narrow_to(PesVersion::Pes16);
    assert_eq!(
        narrowed,
        [NarrowedValue {
            key: "face.facial_hair_type".to_string(),
            value: 15,
            range: "0 to 12".to_string(),
        }]
    );
    assert_eq!(settings.get(SettingKey::FacialHairType), Some(0));
}

/// An unmeasured cap is the field's bits: a stored cheek_type 5 is kept on
/// PES 18 (its column no game measured) and on PES 19 (cheek measured 0 to
/// 5), and narrows on PES 16 (measured 0 to 3).
#[test]
fn an_unmeasured_cap_is_the_fields_bits() {
    for (version, narrowed) in [
        (PesVersion::Pes18, false),
        (PesVersion::Pes19, false),
        (PesVersion::Pes16, true),
    ] {
        let mut settings = with_face("[face]\ncheek_type = 5\n");
        if narrowed {
            assert_eq!(
                settings.narrow_to(version),
                [NarrowedValue {
                    key: "face.cheek_type".to_string(),
                    value: 5,
                    range: "0 to 3".to_string(),
                }],
                "{version:?}"
            );
            assert_eq!(settings.get(SettingKey::CheekType), Some(0), "{version:?}");
        } else {
            assert_eq!(settings.narrow_to(version), [], "{version:?}");
            assert_eq!(settings.get(SettingKey::CheekType), Some(5), "{version:?}");
        }
    }
}

/// A one-based motion past the target's top resets to the template's value
/// (`1`, stored 0); the report carries the shown 5, not the stored 4, and
/// the shown range "1 to 3".
#[test]
fn a_motion_over_the_target_s_top_resets_to_the_template_s_value() {
    let mut settings = PlayerSettings::parse("[appearance.motion]\nhunching_dribbling = 5\n")
        .expect("inside the widest range");
    let narrowed = settings.narrow_to(PesVersion::Pes18);
    assert_eq!(
        narrowed,
        [NarrowedValue {
            key: "appearance.motion.hunching_dribbling".to_string(),
            value: 5,
            range: "1 to 3".to_string(),
        }]
    );
    assert_eq!(settings.get(SettingKey::HunchingDribbling), Some(0));
}

/// A celebration past the target's top (PES 18's 127) resets the same way.
#[test]
fn a_celebration_over_the_target_s_top_resets() {
    let mut settings = PlayerSettings::parse("[appearance.motion]\ngoal_celebration_1 = 162\n")
        .expect("inside the widest range");
    let narrowed = settings.narrow_to(PesVersion::Pes18);
    assert_eq!(
        narrowed,
        [NarrowedValue {
            key: "appearance.motion.goal_celebration_1".to_string(),
            value: 162,
            range: "0 to 127".to_string(),
        }]
    );
    assert_eq!(settings.get(SettingKey::GoalCelebration1), Some(0));
}

/// A custom skin (7) exists on PES 15 to 17 only: kept on 17, reset to the
/// default 1 on 18 and reported against "0 to 6".
#[test]
fn a_custom_skin_resets_only_where_the_target_lacks_one() {
    let mut settings =
        PlayerSettings::parse("[appearance]\nskin_color = 7\n").expect("inside the widest range");
    let narrowed = settings.narrow_to(PesVersion::Pes18);
    assert_eq!(
        narrowed,
        [NarrowedValue {
            key: "appearance.skin_color".to_string(),
            value: 7,
            range: "0 to 6".to_string(),
        }]
    );
    assert_eq!(settings.get(SettingKey::SkinColor), Some(1));

    let mut settings =
        PlayerSettings::parse("[appearance]\nskin_color = 7\n").expect("inside the widest range");
    assert_eq!(settings.narrow_to(PesVersion::Pes17), []);
    assert_eq!(settings.get(SettingKey::SkinColor), Some(7));
}

/// A value at the target's cap is inside its range: left alone and
/// unreported, on an unmeasured cap (PES 18, the field's bits) and a
/// measured one (PES 21).
#[test]
fn a_value_inside_the_target_s_range_is_untouched() {
    for version in [PesVersion::Pes18, PesVersion::Pes21] {
        let cap = face_type_cap(version, IngameFaceField::FacialHairType).expect("a face type");
        let mut settings = with_face(&format!("[face]\nfacial_hair_type = {cap}\n"));
        assert_eq!(settings.narrow_to(version), [], "{version:?}");
        assert_eq!(settings.get(SettingKey::FacialHairType), Some(cap));
    }
}

/// A key the target has no field for (`dribbling` before PES 20) is left
/// to `held_on`'s drop at the write: kept as authored, unreported.
#[test]
fn a_key_the_target_lacks_is_not_reported() {
    let mut settings = PlayerSettings::parse("[appearance.motion]\ndribbling = 2\n")
        .expect("inside the widest range");
    assert_eq!(settings.narrow_to(PesVersion::Pes18), []);
    // The file counts from 1, as the other motions: 2 is stored 1.
    assert_eq!(settings.get(SettingKey::Dribbling), Some(1));
    // On PES 20 the key is held and 2 is inside its 1 to 4.
    let mut settings = PlayerSettings::parse("[appearance.motion]\ndribbling = 2\n")
        .expect("inside the widest range");
    assert_eq!(settings.narrow_to(PesVersion::Pes20), []);
    assert_eq!(settings.get(SettingKey::Dribbling), Some(1));
}

/// `version_kind` is `None` exactly where `held_on` is, and narrows only
/// the keys with a measured per-version range.
#[test]
fn version_kind_is_none_where_the_version_lacks_the_key() {
    for version in PesVersion::ALL {
        for key in SettingKey::ALL {
            assert_eq!(
                key.version_kind(version).is_none(),
                !held_on(key, version),
                "{version:?} {key:?}"
            );
        }
    }
}
