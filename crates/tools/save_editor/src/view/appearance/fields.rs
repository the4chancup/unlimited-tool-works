//! The Appearance section's field tables and the helpers that read them: the
//! control rows (`PHYSIQUE`, `STRIP_ENUMS`, `MOTIONS`), the palette colour
//! tables, and the readers of `settings_toml`'s key spec that give each
//! control its scale and its labels (`number_range`, `combo_options`,
//! `named_values`).

use std::ops::RangeInclusive;

use pes_savefile::interchange::team_toml::labels::title_case;
use pes_savefile::model::{PlayerAppearance, PlayerMotion};
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::settings_toml::{Kind, SettingKey};
use pes_version::{Engine, PesVersion};

use crate::view::abilities::SmallStat;

/// One `PlayerAppearance` field's row: `(label, key, get, set)` — `label` is
/// the control's, `key` its `settings.toml` key (the scale and labels come
/// from it), `get`/`set` the stored accessors.
pub(crate) type AppearanceField = (
    &'static str,
    SettingKey,
    fn(&PlayerAppearance) -> u8,
    fn(&mut PlayerAppearance, u8),
);

/// The Cutscenes group's two stored fields: star 0-7 (PES 19+) and
/// playing attitude 0-3 (PES 20+), stored as shown. They change only
/// cutscenes, so they sit outside the export's ownership, above the
/// section's lock — `abilities`' `SmallStat` because the pair shares the
/// Profile fields' shape.
pub(crate) const CUTSCENE_STATS: [SmallStat; 2] = [
    SmallStat {
        label: "Star",
        shown: "Star",
        field: PlayerField::Star,
        min: 0,
        max: 7,
        shift: 0,
        get: |stats| stats.star,
        set: |stats, value| stats.star = Some(value),
    },
    SmallStat {
        label: "Playing attitude",
        shown: "Playing attitude",
        field: PlayerField::PlayingAttitude,
        min: 0,
        max: 3,
        shift: 0,
        get: |stats| stats.playing_attitude,
        set: |stats, value| stats.playing_attitude = Some(value),
    },
];

/// The 14 physique fields, the draft's order (`Kind::Signed` keys).
pub(crate) const PHYSIQUE: [AppearanceField; 14] = [
    (
        "Neck length",
        SettingKey::NeckLength,
        |a| a.neck_length,
        |a, v| {
            a.neck_length = v;
        },
    ),
    (
        "Neck size",
        SettingKey::NeckSize,
        |a| a.neck_size,
        |a, v| {
            a.neck_size = v;
        },
    ),
    (
        "Shoulder height",
        SettingKey::ShoulderHeight,
        |a| a.shoulder_height,
        |a, v| {
            a.shoulder_height = v;
        },
    ),
    (
        "Shoulder width",
        SettingKey::ShoulderWidth,
        |a| a.shoulder_width,
        |a, v| {
            a.shoulder_width = v;
        },
    ),
    (
        "Chest",
        SettingKey::Chest,
        |a| a.chest,
        |a, v| {
            a.chest = v;
        },
    ),
    (
        "Waist",
        SettingKey::Waist,
        |a| a.waist,
        |a, v| {
            a.waist = v;
        },
    ),
    (
        "Arm size",
        SettingKey::ArmSize,
        |a| a.arm_size,
        |a, v| {
            a.arm_size = v;
        },
    ),
    (
        "Arm length",
        SettingKey::ArmLength,
        |a| a.arm_length,
        |a, v| {
            a.arm_length = v;
        },
    ),
    (
        "Thigh",
        SettingKey::Thigh,
        |a| a.thigh,
        |a, v| {
            a.thigh = v;
        },
    ),
    (
        "Calf",
        SettingKey::Calf,
        |a| a.calf,
        |a, v| {
            a.calf = v;
        },
    ),
    (
        "Leg length",
        SettingKey::LegLength,
        |a| a.leg_length,
        |a, v| {
            a.leg_length = v;
        },
    ),
    (
        "Head length",
        SettingKey::HeadLength,
        |a| a.head_length,
        |a, v| {
            a.head_length = v;
        },
    ),
    (
        "Head width",
        SettingKey::HeadWidth,
        |a| a.head_width,
        |a, v| {
            a.head_width = v;
        },
    ),
    (
        "Head depth",
        SettingKey::HeadDepth,
        |a| a.head_depth,
        |a, v| {
            a.head_depth = v;
        },
    ),
];

/// The strip's `ComboBox` fields inside the grid, the draft's order (the
/// wrist-taping and spectacles combos draw beside their palettes, below;
/// `Kind::Labels` keys).
pub(crate) const STRIP_ENUMS: [AppearanceField; 4] = [
    (
        "Sleeves",
        SettingKey::Sleeves,
        |a| a.sleeves,
        |a, v| {
            a.sleeves = v;
        },
    ),
    (
        "Inners",
        SettingKey::Inners,
        |a| a.inners,
        |a, v| {
            a.inners = v;
        },
    ),
    (
        "Socks",
        SettingKey::Socks,
        |a| a.socks,
        |a, v| {
            a.socks = v;
        },
    ),
    (
        "Undershorts",
        SettingKey::Undershorts,
        |a| a.undershorts,
        |a, v| {
            a.undershorts = v;
        },
    ),
];

/// One motion control (`plans`' measured tops, the draft's `mv` rows):
/// label, the schema field `motion_top` reads, whether the shown scale is
/// 1-based (a celebration is stored as shown, 0 being OFF), and the
/// `PlayerMotion` accessors.
#[derive(Clone, Copy)]
pub(crate) struct MotionField {
    /// The control's label.
    pub(crate) label: &'static str,
    /// The schema field `motion_top` reads.
    pub(crate) field: PlayerField,
    /// A 1-based motion shows `stored + 1`.
    pub(crate) one_based: bool,
    /// The stored value.
    pub(super) get: fn(&PlayerMotion) -> u8,
    /// The stored write.
    pub(super) set: fn(&mut PlayerMotion, u8),
}

/// The motion controls in the draft's order: the two hunching and two arm
/// fields, the three kick motions, then the two celebrations (the dribbling
/// motion is drawn separately, between the kicks and the celebrations, and
/// Randomize never touches it).
pub(crate) const MOTIONS: [MotionField; 9] = [
    MotionField {
        label: "Hunching (dribbling)",
        field: PlayerField::HunchingDribbling,
        one_based: true,
        get: |m| m.hunching_dribbling,
        set: |m, v| m.hunching_dribbling = v,
    },
    MotionField {
        label: "Hunching (running)",
        field: PlayerField::HunchingRunning,
        one_based: true,
        get: |m| m.hunching_running,
        set: |m, v| m.hunching_running = v,
    },
    MotionField {
        label: "Arm movement (dribbling)",
        field: PlayerField::ArmMovementDribbling,
        one_based: true,
        get: |m| m.arm_movement_dribbling,
        set: |m, v| m.arm_movement_dribbling = v,
    },
    MotionField {
        label: "Arm movement (running)",
        field: PlayerField::ArmMovementRunning,
        one_based: true,
        get: |m| m.arm_movement_running,
        set: |m, v| m.arm_movement_running = v,
    },
    MotionField {
        label: "Corner kick",
        field: PlayerField::CornerKickMotion,
        one_based: true,
        get: |m| m.corner_kick,
        set: |m, v| m.corner_kick = v,
    },
    MotionField {
        label: "Free kick",
        field: PlayerField::FreeKickMotion,
        one_based: true,
        get: |m| m.free_kick,
        set: |m, v| m.free_kick = v,
    },
    MotionField {
        label: "Penalty kick",
        field: PlayerField::PenaltyKickMotion,
        one_based: true,
        get: |m| m.penalty_kick,
        set: |m, v| m.penalty_kick = v,
    },
    MotionField {
        label: "Goal celebration 1",
        field: PlayerField::GoalCelebration1,
        one_based: false,
        get: |m| m.goal_celebration_1,
        set: |m, v| m.goal_celebration_1 = v,
    },
    MotionField {
        label: "Goal celebration 2",
        field: PlayerField::GoalCelebration2,
        one_based: false,
        get: |m| m.goal_celebration_2,
        set: |m, v| m.goal_celebration_2 = v,
    },
];

/// The strip palettes' swatch colours, approximations of the spectacles
/// colours' names (`SettingKey::SpectaclesColor`), which the gloves colour
/// shares; the wrist tape's colours are assumed to be the same set,
/// unverified.
const STRIP_COLORS: [[u8; 3]; 8] = [
    [0xE8, 0xE8, 0xE8],
    [0x1C, 0x1C, 0x1C],
    [0xC0, 0x30, 0x28],
    [0x2A, 0x4F, 0xC0],
    [0xE0, 0xC0, 0x20],
    [0x2F, 0x8A, 0x3F],
    [0xE0, 0x80, 0xA8],
    [0x38, 0xB8, 0xB0],
];

/// The skin palette's swatch colours, the draft's approximations; the last
/// is Custom (the invisible body), which pre-Fox alone has.
const SKIN_COLORS: [[u8; 3]; 8] = [
    [0xF2, 0xD5, 0xC0],
    [0xE8, 0xBF, 0xA0],
    [0xD8, 0xA8, 0x80],
    [0xB8, 0x87, 0x5F],
    [0x9A, 0x73, 0x50],
    [0x6E, 0x4B, 0x30],
    [0x3F, 0x2A, 0x1C],
    [0x2A, 0x3A, 0x1E],
];

/// The iris palette's swatch colours, the draft's approximations.
const IRIS_COLORS: [[u8; 3]; 11] = [
    [0x14, 0x10, 0x0E],
    [0x2E, 0x1F, 0x14],
    [0x5A, 0x3A, 0x22],
    [0x6E, 0x52, 0x38],
    [0x2A, 0x3A, 0x5C],
    [0x3A, 0x3A, 0x3A],
    [0x8C, 0x8C, 0x8C],
    [0x3A, 0x6E, 0xC0],
    [0x8A, 0x4B, 0x24],
    [0x3F, 0x7A, 0x46],
    [0x6A, 0x4B, 0x8A],
];

/// What a palette row draws: the widget label's stem, the `settings.toml`
/// key the entry names come from, the last index drawn and the swatch
/// colours (the draft's approximations).
pub(crate) struct Palette {
    /// The swatch labels' stem (`"Skin"`, `"Wrist tape left"`).
    pub(crate) label: &'static str,
    /// The settings key the entry names come from (`named_values`).
    pub(super) key: SettingKey,
    /// The last entry index drawn (`key`'s `Number` max, skin's one less on
    /// Fox).
    pub(super) top: u8,
    /// The swatch colours, `top` + 1 of them.
    pub(super) colors: &'static [[u8; 3]],
}

impl Palette {
    /// `stored` as the palette shows it: its swatch name, or its number
    /// when past the drawn entries (a Fox save's Custom skin draws as 7).
    pub(crate) fn shown(&self, stored: u8) -> String {
        if stored > self.top {
            stored.to_string()
        } else {
            option_name(self.key, stored)
        }
    }
}

/// The palette `key` draws at `version`, in `colour_rows`' and
/// `strip_rows`' place: its label stem, swatch set and drawn top.
pub(crate) fn palette(version: PesVersion, key: SettingKey) -> Palette {
    let (label, colors) = match key {
        SettingKey::SkinColor => ("Skin", SKIN_COLORS.as_slice()),
        SettingKey::IrisColor => ("Iris", IRIS_COLORS.as_slice()),
        SettingKey::WristTapeColorLeft => ("Wrist tape left", STRIP_COLORS.as_slice()),
        SettingKey::WristTapeColorRight => ("Wrist tape right", STRIP_COLORS.as_slice()),
        SettingKey::SpectaclesColor => ("Spectacles color", STRIP_COLORS.as_slice()),
        SettingKey::GlovesColor => ("Gloves color", STRIP_COLORS.as_slice()),
        _ => unreachable!("{key:?} is not a palette key"),
    };
    Palette {
        label,
        key,
        top: palette_top(version, key),
        colors,
    }
}

/// `key`'s drawn top at `version`: its `Number` range's top on every
/// palette but the skin's on Fox, which drops the Custom entry (the three
/// pre-Fox versions alone have it).
fn palette_top(version: PesVersion, key: SettingKey) -> u8 {
    let top = *number_range(key).end();
    if key != SettingKey::SkinColor {
        return top;
    }
    match version.engine() {
        Engine::PreFox => top,
        Engine::Fox => top - 1,
    }
}

/// `key`'s range as a `Kind::Number` (the palettes and the numbered ids);
/// only the keys whose spec is a plain number range use it.
pub(super) fn number_range(key: SettingKey) -> RangeInclusive<u8> {
    let Kind::Number { min, max } = key.spec().kind else {
        unreachable!("{key:?} is not a Number key")
    };
    min..=max
}

/// The option labels `key` edits by in `settings.toml` (the draft's combos
/// and swatch hovers): a Labels kind's words title-cased, or the names its
/// comment lists, or its range's numbers when it names none.
pub(super) fn combo_options(key: SettingKey) -> Vec<String> {
    match key.spec().kind {
        Kind::Labels(words) => words.iter().map(|word| title_case(word)).collect(),
        Kind::Number { min, max } => named_values(key)
            .map(|names| names.iter().map(|name| title_case(name)).collect())
            .unwrap_or_else(|| (min..=max).map(|index| index.to_string()).collect()),
        Kind::Signed { .. }
        | Kind::OneBased { .. }
        | Kind::PlusOne { .. }
        | Kind::Mapped(_)
        | Kind::Bool => unreachable!("{key:?} is not a combo key"),
    }
}

/// The label `key`'s `stored` index shows as: a combo's selected text and
/// a palette's swatch name — `combo_options`' entry at `stored`, its
/// number when past the list (a stored value the key cannot represent).
pub(crate) fn option_name(key: SettingKey, stored: u8) -> String {
    combo_options(key)
        .get(usize::from(stored))
        .cloned()
        .unwrap_or_else(|| stored.to_string())
}

/// The value names `key`'s `settings.toml` comment lists (`"0 white, 1
/// black, ..."`), one per value its `Kind::Number` range gives; `None` when
/// the comment names no per-value list (a `"0 to N"` range comment). An
/// entry's parenthesised note (skin's "custom (invisible body, ...)") is
/// not part of its name; a comma inside it does not start an entry — only a
/// numeric prefix does.
fn named_values(key: SettingKey) -> Option<Vec<String>> {
    let Kind::Number { min, max } = key.spec().kind else {
        return None;
    };
    let mut entries: Vec<String> = Vec::new();
    for part in key.spec().comment.split(", ") {
        let starts_an_entry = part
            .split(' ')
            .next()
            .is_some_and(|first| first.bytes().all(|byte| byte.is_ascii_digit()));
        if starts_an_entry || entries.is_empty() {
            entries.push(part.to_owned());
        } else if let Some(last) = entries.last_mut() {
            last.push_str(", ");
            last.push_str(part);
        }
    }
    if entries.len() != usize::from(max - min) + 1 {
        return None;
    }
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let (digits, name) = entry.split_once(' ')?;
            let entry_index = digits.parse::<u16>().ok()?;
            (entry_index == u16::from(min) + u16::try_from(index).ok()?)
                // A name ends at a note: " (…)" after it, or "; …" after the
                // list's last one ("6 special; PES 15 only").
                .then(|| {
                    name.split([';', '('])
                        .next()
                        .unwrap_or(name)
                        .trim_end()
                        .to_owned()
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The names the key comments carry, pinned in full so a reworded
    // comment fails here instead of silently dropping the labels.
    #[test]
    fn named_values_gives_the_keys_full_name_lists() {
        assert_eq!(
            named_values(SettingKey::SkinColor),
            Some(vec![
                "white".to_owned(),
                "light".to_owned(),
                "fair".to_owned(),
                "medium".to_owned(),
                "olive".to_owned(),
                "brown".to_owned(),
                "black".to_owned(),
                "custom".to_owned(),
            ]),
        );
        assert_eq!(
            named_values(SettingKey::IrisColor),
            Some(vec![
                "black".to_owned(),
                "dark brown".to_owned(),
                "brown".to_owned(),
                "sable".to_owned(),
                "navy blue".to_owned(),
                "charcoal".to_owned(),
                "gray".to_owned(),
                "blue".to_owned(),
                "sienna".to_owned(),
                "green".to_owned(),
                "violet".to_owned(),
            ]),
        );
        let spectacles = Some(vec![
            "none".to_owned(),
            "rectangle rimless".to_owned(),
            "rectangle half frame".to_owned(),
            "rectangle full frame".to_owned(),
            "oval rimless".to_owned(),
            "oval half frame".to_owned(),
            "oval full frame".to_owned(),
            "round full frame".to_owned(),
        ]);
        assert_eq!(named_values(SettingKey::Spectacles), spectacles);
        assert_eq!(
            named_values(SettingKey::SpectaclesColor),
            Some(vec![
                "white".to_owned(),
                "black".to_owned(),
                "red".to_owned(),
                "blue".to_owned(),
                "yellow".to_owned(),
                "green".to_owned(),
                "pink".to_owned(),
                "turquoise".to_owned(),
            ]),
        );
        assert_eq!(named_values(SettingKey::WristTapeColorLeft), None);
        assert_eq!(
            named_values(SettingKey::GlovesColor),
            named_values(SettingKey::SpectaclesColor)
        );
    }

    // A named list over a range that does not start at 0: the entry count is
    // `max - min + 1`, not `max + min + 1` (Pes15HairLength's 1 to 6).
    #[test]
    fn named_values_counts_a_nonzero_mins_list() {
        assert_eq!(
            named_values(SettingKey::Pes15HairLength),
            Some(vec![
                "skinhead".to_owned(),
                "very short".to_owned(),
                "short".to_owned(),
                "medium".to_owned(),
                "long".to_owned(),
                "special".to_owned(),
            ]),
        );
    }

    // The same source for a Labels key: the words title-cased.
    #[test]
    fn combo_options_gives_the_labels_words_title_cased() {
        assert_eq!(
            combo_options(SettingKey::Sleeves),
            ["Seasonal", "Short", "Long"]
        );
    }
}
