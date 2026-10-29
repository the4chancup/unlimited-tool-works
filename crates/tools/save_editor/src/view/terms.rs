//! The views' terms for a `diff` row: a field a view draws reads under the
//! view's own label, its old and new values as that view shows them, since
//! a member reads a row against the view it jumps to. So a label or scale
//! changed in a view changes the row with it. A field no view draws keeps
//! the report's name and values (`messages::diff_fields`). The same terms
//! word the unsaved-changes table's team rows (`team_row_terms`).

use pes_savefile::interchange::team_toml::labels::{
    ABILITY_STATS, COM_STYLES, PLAYABLE_POSITIONS, POSITIONS, SKILLS, stat_name, title_case,
};
use pes_savefile::model::TeamEntry;
use pes_savefile::ops::compare::{PlayerDiff, TeamDiff};
use pes_savefile::schema::fields::{
    ABILITY_FIELDS, PlayerField, PlayerText, RosterField, TeamField, TeamText,
};
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::settings_toml::{Kind, SettingKey};
use pes_version::PesVersion;
use team_widgets::cards;
use team_widgets::tactics::RosterRow;

use crate::messages::diff_fields;
use crate::view::appearance::{self, fields};
use crate::view::{abilities, info_strip, team};

/// A `diff` row's terms: the field's label and its two values as the view
/// that owns the field shows them.
pub(super) struct Term {
    /// The field's label: the view's own where a view draws the field.
    pub(super) label: String,
    /// `old` as the view shows it.
    pub(super) old: String,
    /// `new` the same way.
    pub(super) new: String,
    /// A team colour's row only: the old and new colours as the team view
    /// shows them, the changes table's swatches before the two hex values.
    /// Known from the field compared, never read back from the text, which
    /// a player's name could also spell.
    pub(super) swatches: Option<[egui::Color32; 2]>,
}

/// `diff`'s row in the views' terms.
pub(super) fn row_term(version: PesVersion, diff: &PlayerDiff) -> Term {
    match diff {
        PlayerDiff::Field { .. } => player_field_term(version, diff),
        PlayerDiff::Face { .. } => face_field_term(version, diff),
        PlayerDiff::Text { text, old, new } => match text {
            PlayerText::Name => Term {
                label: info_strip::NAME_LABEL.to_owned(),
                old: info_strip::shown_name(old),
                new: info_strip::shown_name(new),
                swatches: None,
            },
            PlayerText::ShirtName => Term {
                label: info_strip::SHIRT_LABEL.to_owned(),
                old: old.clone(),
                new: new.clone(),
                swatches: None,
            },
        },
        // The run's fingerprint row is drawn by no view.
        PlayerDiff::FaceRun { .. } => report_term(diff),
    }
}

/// The report's own terms for `diff`: the field's schema name and its
/// stored values — the row for a field no view draws.
fn report_term(diff: &PlayerDiff) -> Term {
    let (label, old, new) = diff_fields(diff);
    Term {
        label,
        old,
        new,
        swatches: None,
    }
}

/// `diffs`' rows in the views' terms, for the unsaved-changes prompt: one
/// `Term` per row in `diffs`' order — a team colour's three channel diffs
/// merge into the swatch's one row, at the first channel's place. `team`
/// is the open save's entry (its tactics' eleven name the slots);
/// `baseline_team` is the file-as-opened's, whose tactics an
/// instruction-target row's old side is worded under and whose colour a
/// colour row's old side reads. `old_roster`/`new_roster` are the
/// Tactics editor's roster rows of each side: a drag moves records between
/// the roster's ids while a stored byte still names the slot, so the old
/// side's names come from the baseline's roster, the new side's from the
/// current's.
pub(super) fn team_row_terms(
    version: PesVersion,
    team: &TeamEntry,
    baseline_team: &TeamEntry,
    old_roster: &[RosterRow<'_>],
    new_roster: &[RosterRow<'_>],
    diffs: &[TeamDiff],
) -> Vec<Term> {
    let mut terms = Vec::with_capacity(diffs.len());
    let mut colours_done = [false; 2];
    for diff in diffs {
        match colour_index(diff) {
            Some(colour) if !colours_done[colour] => {
                colours_done[colour] = true;
                terms.push(colour_term(baseline_team, team, colour));
            }
            // A later channel of an already merged colour is no row.
            Some(_) => {}
            None => terms.push(team_row_term(
                version,
                team,
                baseline_team,
                old_roster,
                new_roster,
                diff,
            )),
        }
    }
    terms
}

/// A `TeamDiff`'s row in the views' terms, the team twin of `row_term`.
fn team_row_term(
    version: PesVersion,
    team: &TeamEntry,
    baseline_team: &TeamEntry,
    old_roster: &[RosterRow<'_>],
    new_roster: &[RosterRow<'_>],
    diff: &TeamDiff,
) -> Term {
    match diff {
        TeamDiff::Text { text, old, new } => match text {
            TeamText::Name => term(team::TEAM_LABEL, old.clone(), new.clone()),
            TeamText::ShortName => term(team::SHORT_LABEL, old.clone(), new.clone()),
        },
        TeamDiff::Field { field, old, new } => match field {
            TeamField::ManagerId => numbers(team::MANAGER_LABEL, *old, *new),
            TeamField::StadiumId => numbers(team::STADIUM_LABEL, *old, *new),
            TeamField::KitSlotNumber(index) => numbers(
                format!("Slot {} · {}", index + 1, team::KIT_NUMBER_LABEL),
                *old,
                *new,
            ),
            // The view shows the bound team, `binding / 64`.
            TeamField::KitSlotTeam(index) => numbers(
                format!("Slot {} · {}", index + 1, team::BOUND_TEAM_LABEL),
                *old / 64,
                *new / 64,
            ),
            // The flags the view sets as a side effect, the id, and a
            // colour channel alone (a merge `team_row_terms` already
            // handled): the report's terms, as players' undrawn fields
            // get.
            TeamField::Id
            | TeamField::EditedName
            | TeamField::EditedShortName
            | TeamField::EditedStadium
            | TeamField::EditedStrip
            | TeamField::Color1Red
            | TeamField::Color1Green
            | TeamField::Color1Blue
            | TeamField::Color2Red
            | TeamField::Color2Green
            | TeamField::Color2Blue => team_report_term(diff),
        },
        TeamDiff::Roster { field, old, new } => match field {
            RosterField::Number(_) => numbers(info_strip::NUMBER_LABEL, *old, *new),
            RosterField::TeamId | RosterField::Player(_) => team_report_term(diff),
        },
        TeamDiff::Tactics { field, old, new } => {
            let (label, old, new) = team_widgets::tactics::tactics_term(
                version,
                *field,
                *old,
                *new,
                &baseline_team.tactics,
                &team.tactics,
                (old_roster, new_roster),
            );
            Term {
                label,
                old,
                new,
                swatches: None,
            }
        }
    }
}

/// The colour (0 or 1) a `Field` diff is a channel of, `None` for every
/// other diff.
fn colour_index(diff: &TeamDiff) -> Option<usize> {
    match diff {
        TeamDiff::Field {
            field: TeamField::Color1Red | TeamField::Color1Green | TeamField::Color1Blue,
            ..
        } => Some(0),
        TeamDiff::Field {
            field: TeamField::Color2Red | TeamField::Color2Green | TeamField::Color2Blue,
            ..
        } => Some(1),
        _ => None,
    }
}

/// The view's one row for a team colour: the swatch's label, the two
/// sides' hex (`colour_unit`'s `format_hex` over `channel_to_show`) and
/// the two shown colours, read from `baseline`'s and `team`'s stored
/// channels — the diffs the row merges only say the colour changed.
fn colour_term(baseline: &TeamEntry, team: &TeamEntry, colour: usize) -> Term {
    let shown = |entry: &TeamEntry| {
        let colors = entry
            .colors
            .expect("a colour diff is emitted only where the version stores colours");
        [
            colors[colour].red,
            colors[colour].green,
            colors[colour].blue,
        ]
        .map(team::channel_to_show)
    };
    let (old, new) = (shown(baseline), shown(team));
    let swatch = |[red, green, blue]: [u8; 3]| egui::Color32::from_rgb(red, green, blue);
    Term {
        label: team::COLOUR_LABELS[colour].to_owned(),
        old: team::format_hex(old),
        new: team::format_hex(new),
        swatches: Some([swatch(old), swatch(new)]),
    }
}

/// The report's own terms for a team diff, `report_term`'s twin: the
/// field's schema name and its stored values — the row for a field no
/// view draws.
fn team_report_term(diff: &TeamDiff) -> Term {
    let (label, old, new) = match diff {
        TeamDiff::Field { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Roster { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Tactics { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Text { text, old, new } => (format!("{text:?}"), old.clone(), new.clone()),
    };

    Term {
        label,
        old,
        new,
        swatches: None,
    }
}

/// A report row's three strings for a named field: the schema name and
/// the two stored values.
fn report_fields(field: impl std::fmt::Debug, old: u32, new: u32) -> (String, String, String) {
    (format!("{field:?}"), old.to_string(), new.to_string())
}

/// A `Field` row's term: the views' own tables first (`SMALL_STATS`,
/// `CUTSCENE_STATS`, `MOTIONS`, `PHYSIQUE`, `STRIP_ENUMS`, the ability
/// stats), then the fields the views draw one-off, then the report's terms
/// for a field no view draws — the `_` arm IS the rule for those, not an
/// oversight.
fn player_field_term(version: PesVersion, diff: &PlayerDiff) -> Term {
    let PlayerDiff::Field { field, old, new } = *diff else {
        unreachable!("player_field_term gets a Field row")
    };
    if let Some(small) = abilities::SMALL_STATS
        .iter()
        .chain(fields::CUTSCENE_STATS.iter())
        .find(|stat| stat.field == field)
    {
        let shift = u32::from(small.shift);
        return numbers(small.label, old + shift, new + shift);
    }
    if let Some(index) = ABILITY_FIELDS.iter().position(|ability| *ability == field) {
        return numbers(stat_name(version, ABILITY_STATS[index]), old, new);
    }
    if let Some(motion) = fields::MOTIONS.iter().find(|row| row.field == field) {
        let shift = u32::from(motion.one_based);
        return numbers(motion.label, old + shift, new + shift);
    }
    if let Some((label, key, ..)) = fields::PHYSIQUE
        .iter()
        .find(|(_, key, ..)| key.player_field() == Some(field))
    {
        let Kind::Signed { limit } = key.spec().kind else {
            unreachable!("the physique keys are Signed");
        };
        let limit = i64::from(limit);
        return Term {
            label: (*label).to_owned(),
            old: (i64::from(old) - limit).to_string(),
            new: (i64::from(new) - limit).to_string(),
            swatches: None,
        };
    }
    if let Some((label, key, ..)) = fields::STRIP_ENUMS
        .iter()
        .find(|(_, key, ..)| key.player_field() == Some(field))
    {
        return term(
            *label,
            fields::option_name(*key, stored_u8(old)),
            fields::option_name(*key, stored_u8(new)),
        );
    }
    match field {
        PlayerField::Height => numbers(info_strip::HEIGHT_LABEL, old, new),
        PlayerField::Weight => numbers(info_strip::WEIGHT_LABEL, old, new),
        PlayerField::Age => numbers(info_strip::AGE_LABEL, old, new),
        PlayerField::Nationality => numbers(info_strip::NATION_LABEL, old, new),
        PlayerField::RegisteredPosition => term(
            abilities::REGISTERED_POSITION_LABEL,
            abilities::position_label(stored_u8(old)),
            abilities::position_label(stored_u8(new)),
        ),
        PlayerField::PlayingStyle => term(
            abilities::PLAYING_STYLE_LABEL,
            cards::playing_style_text(version, stored_u8(old)),
            cards::playing_style_text(version, stored_u8(new)),
        ),
        PlayerField::StrongerFoot => term(
            abilities::STRONGER_FOOT_LABEL,
            abilities::foot_label(stored_u8(old)),
            abilities::foot_label(stored_u8(new)),
        ),
        PlayerField::StrongerHand => term(
            abilities::STRONGER_HAND_LABEL,
            abilities::foot_label(stored_u8(old)),
            abilities::foot_label(stored_u8(new)),
        ),
        PlayerField::PlayablePosition(index) => term(
            // The group's heading then the row's own label: a bare position
            // name would read as a registered position's value.
            format!(
                "{} {}",
                abilities::PLAYABLE_POSITIONS_LABEL,
                POSITIONS[PLAYABLE_POSITIONS[usize::from(index)]]
            ),
            rating_label(old),
            rating_label(new),
        ),
        PlayerField::Skill(index) => term(
            title_case(SKILLS[usize::from(index)]),
            on_off(old),
            on_off(new),
        ),
        PlayerField::ComStyle(index) => term(
            title_case(COM_STYLES[usize::from(index)]),
            on_off(old),
            on_off(new),
        ),
        PlayerField::DribblingMotion => numbers(appearance::DRIBBLING_MOTION_LABEL, old, new),
        PlayerField::BootsId => numbers(appearance::BOOTS_ID_LABEL, old, new),
        PlayerField::GlovesId => numbers(appearance::GLOVES_ID_LABEL, old, new),
        PlayerField::BaseCopyId => numbers(appearance::COPY_FROM_ID_LABEL, old, new),
        PlayerField::WristTaping => keyed(field, appearance::WRIST_TAPING_LABEL, old, new),
        PlayerField::SpectaclesStyle => keyed(field, appearance::SPECTACLES_LABEL, old, new),
        PlayerField::Untucked => flag(appearance::SHIRTTAIL_OUT_LABEL, old, new),
        PlayerField::AnkleTaping => flag(appearance::ANKLE_TAPING_LABEL, old, new),
        PlayerField::EditedFace => flag(appearance::FACE_LABEL, old, new),
        PlayerField::EditedHair => flag(appearance::HAIR_LABEL, old, new),
        PlayerField::EditedPhysique => flag(appearance::PHYSIQUE_LABEL, old, new),
        PlayerField::EditedStrip => flag(appearance::STRIP_LABEL, old, new),
        PlayerField::EditedMotion => flag(appearance::MOTION_LABEL, old, new),
        PlayerField::BaseCopy => flag(appearance::BASE_COPY_LABEL, old, new),
        PlayerField::WristTapeColorLeft
        | PlayerField::WristTapeColorRight
        | PlayerField::SpectaclesColor => {
            let key = key_writing(|key| key.player_field() == Some(field))
                .expect("the strip palettes have settings keys");
            let palette = fields::palette(version, key);
            Term {
                label: palette.label.to_owned(),
                old: palette.shown(stored_u8(old)),
                new: palette.shown(stored_u8(new)),
                swatches: None,
            }
        }
        // The rest is drawn by no view (the ids and gameplay edit flags
        // among them): the report's terms.
        _ => report_term(diff),
    }
}

/// A `Face` row's term: the checkbox and the palettes are drawn, the rest
/// of the run's fields are not (the report's terms).
fn face_field_term(version: PesVersion, diff: &PlayerDiff) -> Term {
    let PlayerDiff::Face { field, old, new } = *diff else {
        unreachable!("face_field_term gets a Face row")
    };
    match field {
        IngameFaceField::PlayerGloves => Term {
            label: appearance::PLAYER_GLOVES_LABEL.to_owned(),
            old: on_off(u32::from(old)).to_owned(),
            new: on_off(u32::from(new)).to_owned(),
            swatches: None,
        },
        IngameFaceField::SkinColor
        | IngameFaceField::IrisColor
        | IngameFaceField::PlayerGlovesColor => {
            let key = key_writing(|key| key.face_field() == Some(field))
                .expect("the colour fields have settings keys");
            let palette = fields::palette(version, key);
            Term {
                label: palette.label.to_owned(),
                old: palette.shown(old),
                new: palette.shown(new),
                swatches: None,
            }
        }
        _ => report_term(diff),
    }
}

/// A `SettingKey`-keyed field's term: the combo's selected text at the
/// stored index (`fields::option_name`).
fn keyed(field: PlayerField, label: &'static str, old: u32, new: u32) -> Term {
    let key = key_writing(|key| key.player_field() == Some(field))
        .expect("the keyed fields have settings keys");
    term(
        label,
        fields::option_name(key, stored_u8(old)),
        fields::option_name(key, stored_u8(new)),
    )
}

/// A term over shown texts.
fn term(label: impl Into<String>, old: impl Into<String>, new: impl Into<String>) -> Term {
    Term {
        label: label.into(),
        old: old.into(),
        new: new.into(),
        swatches: None,
    }
}

/// A term over stored numbers shown as stored (`old`/`new` unchanged).
fn numbers(label: impl Into<String>, old: u32, new: u32) -> Term {
    term(label, old.to_string(), new.to_string())
}

/// A checkbox field's term: a set bit reads "on" (the draft's `T_flag`).
fn flag(label: &'static str, old: u32, new: u32) -> Term {
    term(label, on_off(old), on_off(new))
}

/// `stored` narrowed to the byte a byte-sized control stores (the
/// combos, the palettes, and the position, style, foot and hand pickers).
fn stored_u8(stored: u32) -> u8 {
    u8::try_from(stored).expect("a byte-sized control's field is at most eight bits")
}

/// `stored`'s rating letter; a stored 3, which selects none of the three.
fn rating_label(stored: u32) -> &'static str {
    abilities::RATING_LABELS
        .iter()
        .find(|(_, rating)| u32::from(*rating) == stored)
        .map_or("none", |(label, _)| *label)
}

/// `stored`'s checkbox reading: a set bit reads "on".
fn on_off(stored: u32) -> &'static str {
    if stored == 0 { "off" } else { "on" }
}

/// The `SettingKey` that `writes` (`keys.rs`' `player_field` or
/// `face_field` compared with a field): the key table's own binding, so a
/// key remapped to another field remaps here.
fn key_writing(writes: impl Fn(SettingKey) -> bool) -> Option<SettingKey> {
    SettingKey::ALL.iter().copied().find(|key| writes(*key))
}

#[cfg(test)]
mod tests {
    use pes_savefile::ops::fingerprint::FaceHash;
    use pes_savefile::schema::fields::{PresetField, TacticsField};
    use pes_savefile::{EditFile, fixture_save};

    use super::*;

    /// A player of the PES 17 fixture's team 734 ("Muh Spooks"), for real
    /// stored values.
    fn muh_spooks() -> pes_savefile::model::PlayerEntry {
        EditFile::from_bytes(&fixture_save(PesVersion::Pes17))
            .expect("the fixture loads")
            .player(73401)
            .expect("73401 is a player of team 734")
            .clone()
    }

    fn field(field: PlayerField, old: u32, new: u32) -> PlayerDiff {
        PlayerDiff::Field { field, old, new }
    }

    fn face_row(field: IngameFaceField, old: u8, new: u8) -> PlayerDiff {
        PlayerDiff::Face { field, old, new }
    }

    /// `term` reads (label, old, new) — the literals the view draws, never
    /// the consts under test.
    fn assert_term(term: &Term, label: &str, old: &str, new: &str) {
        assert_eq!(
            (term.label.as_str(), term.old.as_str(), term.new.as_str()),
            (label, old, new)
        );
    }

    /// The info strip's fields: its labels and the stored value as is, the
    /// name with its `\x11` spelled as the strip spells it.
    #[test]
    fn the_info_strips_fields() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(
            (
                player.basic.height,
                player.basic.age,
                player.basic.nationality
            ),
            (185, 20, 210),
            "premise: 73401's fixture basics"
        );
        assert_term(
            &row_term(version, &field(PlayerField::Height, 185, 190)),
            "Height",
            "185",
            "190",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Weight, 85, 80)),
            "Weight",
            "85",
            "80",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Age, 20, 21)),
            "Age",
            "20",
            "21",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Nationality, 210, 0)),
            "Nation",
            "210",
            "0",
        );
        assert_eq!(player.name, "Muh Spooks", "premise: 73401's name");
        assert_term(
            &row_term(
                version,
                &PlayerDiff::Text {
                    text: PlayerText::Name,
                    old: "Muh \x11Spooks".to_owned(),
                    new: player.name.clone(),
                },
            ),
            "Name",
            "Muh \\x11Spooks",
            "Muh Spooks",
        );
        assert_eq!(player.shirt_name, "SEXY MAX", "premise: 73401's shirt");
        assert_term(
            &row_term(
                version,
                &PlayerDiff::Text {
                    text: PlayerText::ShirtName,
                    old: player.shirt_name.clone(),
                    new: "NO NAME".to_owned(),
                },
            ),
            "Shirt",
            "SEXY MAX",
            "NO NAME",
        );
    }

    /// The first row's combos: the position and style names and the foot's
    /// label as the view shows them.
    #[test]
    fn the_first_rows_fields() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(player.positions.registered, 1, "premise: a CB");
        assert_term(
            &row_term(version, &field(PlayerField::RegisteredPosition, 1, 0)),
            "Registered position",
            "CB - Centre Back",
            "GK - Goalkeeper",
        );
        // PES 17's stored 15 is the Build Up of the version's own list.
        assert_eq!(player.positions.playing_style, 15, "premise");
        assert_term(
            &row_term(version, &field(PlayerField::PlayingStyle, 15, 1)),
            "Playing style",
            "Build Up",
            "Goal Poacher",
        );
        // The style is the version's: PES 21's stored 15 is another one.
        assert_term(
            &row_term(PesVersion::Pes21, &field(PlayerField::PlayingStyle, 15, 1)),
            "Playing style",
            "Offensive Fullback",
            "Goal Poacher",
        );
        // A stored value PES 17's list cannot decode shows as its number,
        // as the picker shows it.
        assert_term(
            &row_term(version, &field(PlayerField::PlayingStyle, 15, 22)),
            "Playing style",
            "Build Up",
            "22",
        );
        assert_eq!(player.positions.stronger_foot, 1, "premise: left-footed");
        assert_term(
            &row_term(version, &field(PlayerField::StrongerFoot, 1, 0)),
            "Stronger foot",
            "Left",
            "Right",
        );
        assert_term(
            &row_term(PesVersion::Pes21, &field(PlayerField::StrongerHand, 0, 1)),
            "Stronger hand",
            "Right",
            "Left",
        );
    }

    /// The small stats' +1 shift, and the ability stats' names as the
    /// save's game shows them (PES 2017's Header) with the stored number.
    #[test]
    fn the_small_stats_and_abilities_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(player.stats.weak_foot_usage, 1, "premise");
        // A weak foot usage stored 2 shows 3.
        assert_term(
            &row_term(version, &field(PlayerField::WeakFootUsage, 1, 2)),
            "Weak Foot Usage",
            "2",
            "3",
        );
        assert_term(
            &row_term(version, &field(PlayerField::WeakFootAccuracy, 3, 0)),
            "Weak Foot Accuracy",
            "4",
            "1",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Form, 3, 7)),
            "Form",
            "4",
            "8",
        );
        assert_term(
            &row_term(version, &field(PlayerField::InjuryResistance, 0, 2)),
            "Injury Resistance",
            "1",
            "3",
        );
        // Star and playing attitude (PES 19/20+) show as stored.
        assert_term(
            &row_term(PesVersion::Pes21, &field(PlayerField::Star, 0, 7)),
            "Star",
            "0",
            "7",
        );
        assert_term(
            &row_term(
                PesVersion::Pes21,
                &field(PlayerField::PlayingAttitude, 3, 0),
            ),
            "Playing attitude",
            "3",
            "0",
        );
        assert_eq!(player.stats.speed, 77, "premise");
        assert_term(
            &row_term(version, &field(PlayerField::Speed, 77, 40)),
            "Speed",
            "77",
            "40",
        );
        assert_term(
            &row_term(version, &field(PlayerField::AttackingProwess, 90, 99)),
            "Attacking Prowess",
            "90",
            "99",
        );
        assert_eq!(player.stats.heading, 77, "premise");
        assert_term(
            &row_term(version, &field(PlayerField::Heading, 77, 80)),
            "Header",
            "77",
            "80",
        );
        assert_term(
            &row_term(PesVersion::Pes21, &field(PlayerField::Heading, 77, 80)),
            "Heading",
            "77",
            "80",
        );
    }

    /// The playable positions: "Playable position <POS>" with the rating's
    /// letter; a stored 3, which selects none of the three.
    #[test]
    fn the_playable_positions_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        // 73401's only A rating sits at playable index 9, the CB.
        assert_eq!(player.positions.playable[9], 2, "premise");
        assert_term(
            &row_term(version, &field(PlayerField::PlayablePosition(9), 2, 0)),
            "Playable positions CB",
            "A",
            "C",
        );
        assert_term(
            &row_term(version, &field(PlayerField::PlayablePosition(0), 3, 1)),
            "Playable positions CF",
            "none",
            "B",
        );
    }

    /// Skills and COM styles read "on"/"off" under the card's own name.
    #[test]
    fn the_skills_and_com_styles_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert!(
            !player.skills.skills[0] && !player.skills.com_styles[6],
            "premise"
        );
        assert_term(
            &row_term(version, &field(PlayerField::Skill(0), 0, 1)),
            "Scissors Feint",
            "off",
            "on",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Skill(40), 1, 0)),
            "Through Passing",
            "on",
            "off",
        );
        assert_term(
            &row_term(version, &field(PlayerField::ComStyle(6), 0, 1)),
            "Long Ranger",
            "off",
            "on",
        );
    }

    /// The 1-based motions show one above the stored, the celebrations and
    /// the dribbling motion as stored.
    #[test]
    fn the_motion_fields_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(player.motion.free_kick, 0, "premise");
        // A motion stored 14 shows 15.
        assert_term(
            &row_term(version, &field(PlayerField::FreeKickMotion, 0, 14)),
            "Free kick",
            "1",
            "15",
        );
        assert_term(
            &row_term(version, &field(PlayerField::HunchingDribbling, 0, 2)),
            "Hunching (dribbling)",
            "1",
            "3",
        );
        assert_term(
            &row_term(version, &field(PlayerField::ArmMovementRunning, 1, 0)),
            "Arm movement (running)",
            "2",
            "1",
        );
        assert_term(
            &row_term(version, &field(PlayerField::CornerKickMotion, 0, 5)),
            "Corner kick",
            "1",
            "6",
        );
        assert_term(
            &row_term(version, &field(PlayerField::PenaltyKickMotion, 2, 0)),
            "Penalty kick",
            "3",
            "1",
        );
        assert_eq!(player.motion.goal_celebration_1, 1, "premise");
        assert_term(
            &row_term(version, &field(PlayerField::GoalCelebration1, 1, 0)),
            "Goal celebration 1",
            "1",
            "0",
        );
        assert_term(
            &row_term(version, &field(PlayerField::GoalCelebration2, 0, 7)),
            "Goal celebration 2",
            "0",
            "7",
        );
        assert_term(
            &row_term(
                PesVersion::Pes21,
                &field(PlayerField::DribblingMotion, 3, 0),
            ),
            "Dribbling motion",
            "3",
            "0",
        );
    }

    /// The physique fields on their Signed scale: stored - 7.
    #[test]
    fn the_physique_fields_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(player.appearance.neck_length, 6, "premise");
        // A physique stored 7 shows 0, 6 shows -1.
        assert_term(
            &row_term(version, &field(PlayerField::NeckLength, 6, 7)),
            "Neck length",
            "-1",
            "0",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Waist, 14, 0)),
            "Waist",
            "7",
            "-7",
        );
    }

    /// The strip's fields: the combos' picked texts, the checkboxes'
    /// "on"/"off", the palettes' names and numbers, the ids as stored.
    #[test]
    fn the_strip_fields_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert_eq!(
            (
                player.appearance.sleeves,
                player.appearance.socks,
                player.appearance.undershorts
            ),
            (1, 1, 2),
            "premise"
        );
        assert_term(
            &row_term(version, &field(PlayerField::Sleeves, 1, 0)),
            "Sleeves",
            "Short",
            "Seasonal",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Inners, 0, 2)),
            "Inners",
            "Off",
            "Turtleneck",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Socks, 1, 2)),
            "Socks",
            "Long",
            "Short",
        );
        assert_term(
            &row_term(version, &field(PlayerField::Undershorts, 2, 0)),
            "Undershorts",
            "Winter Long",
            "Off",
        );
        assert_term(
            &row_term(version, &field(PlayerField::WristTaping, 0, 3)),
            "Wrist taping",
            "Off",
            "Both",
        );
        assert_term(
            &row_term(version, &field(PlayerField::SpectaclesStyle, 0, 7)),
            "Spectacles",
            "None",
            "Round full frame",
        );
        assert!(
            player.appearance.untucked && !player.appearance.ankle_taping,
            "premise"
        );
        assert_term(
            &row_term(version, &field(PlayerField::Untucked, 1, 0)),
            "Shirttail out",
            "on",
            "off",
        );
        assert_term(
            &row_term(version, &field(PlayerField::AnkleTaping, 0, 1)),
            "Ankle taping",
            "off",
            "on",
        );
        assert_eq!(
            (
                player.appearance.wrist_tape_color_left,
                player.appearance.spectacles_color
            ),
            (0, 0),
            "premise"
        );
        assert_term(
            &row_term(version, &field(PlayerField::WristTapeColorLeft, 0, 4)),
            "Wrist tape left",
            "0",
            "4",
        );
        assert_term(
            &row_term(version, &field(PlayerField::WristTapeColorRight, 4, 0)),
            "Wrist tape right",
            "4",
            "0",
        );
        assert_term(
            &row_term(version, &field(PlayerField::SpectaclesColor, 0, 3)),
            "Spectacles colour",
            "White",
            "Blue",
        );
        assert_eq!(
            (
                player.appearance.boots_id,
                player.appearance.gloves_id,
                player.appearance.base_copy_id
            ),
            (0, 0, 0),
            "premise"
        );
        assert_term(
            &row_term(version, &field(PlayerField::BootsId, 0, 3601)),
            "Boots ID",
            "0",
            "3601",
        );
        assert_term(
            &row_term(version, &field(PlayerField::GlovesId, 0, 627)),
            "Gloves ID",
            "0",
            "627",
        );
        assert_term(
            &row_term(version, &field(PlayerField::BaseCopyId, 0, 70101)),
            "Copy from ID",
            "0",
            "70101",
        );
    }

    /// The edit flags' checkboxes read "on"/"off" under the view's labels.
    #[test]
    fn the_edit_flags_rows() {
        let version = PesVersion::Pes17;
        let player = muh_spooks();
        assert!(
            !player.edit_flags.face && player.edit_flags.base_copy == Some(false),
            "premise"
        );
        for (flag_field, label) in [
            (PlayerField::EditedFace, "Face"),
            (PlayerField::EditedHair, "Hair"),
            (PlayerField::EditedPhysique, "Physique"),
            (PlayerField::EditedStrip, "Strip"),
            (PlayerField::EditedMotion, "Motion"),
            (PlayerField::BaseCopy, "Base copy"),
        ] {
            assert_term(
                &row_term(version, &field(flag_field, 0, 1)),
                label,
                "off",
                "on",
            );
        }
    }

    /// The ingame-face fields the view draws: the gloves checkbox and the
    /// colour palettes. The Custom skin is pre-Fox only — on Fox a stored
    /// 7 draws its number.
    #[test]
    fn the_face_fields_rows() {
        let player = muh_spooks();
        let get = |field| {
            player
                .appearance
                .ingame_face
                .get(field)
                .expect("the run holds the field")
        };
        assert_eq!(
            (
                get(IngameFaceField::SkinColor),
                get(IngameFaceField::IrisColor),
                get(IngameFaceField::PlayerGloves)
            ),
            (1, 2, 0),
            "premise"
        );
        assert_term(
            &row_term(
                PesVersion::Pes17,
                &face_row(IngameFaceField::SkinColor, 1, 7),
            ),
            "Skin",
            "Light",
            "Custom",
        );
        assert_term(
            &row_term(
                PesVersion::Pes21,
                &face_row(IngameFaceField::SkinColor, 1, 7),
            ),
            "Skin",
            "Light",
            "7",
        );
        assert_term(
            &row_term(
                PesVersion::Pes17,
                &face_row(IngameFaceField::IrisColor, 2, 10),
            ),
            "Iris",
            "Brown",
            "Violet",
        );
        assert_term(
            &row_term(
                PesVersion::Pes17,
                &face_row(IngameFaceField::PlayerGlovesColor, 0, 6),
            ),
            "Gloves colour",
            "0",
            "6",
        );
        assert_term(
            &row_term(
                PesVersion::Pes17,
                &face_row(IngameFaceField::PlayerGloves, 0, 1),
            ),
            "Player gloves",
            "off",
            "on",
        );
    }

    /// A field no view draws (a face field, the gameplay edit flags, the
    /// run's fingerprint) keeps the report's name and values.
    #[test]
    fn fields_no_view_draws_keep_the_reports_terms() {
        let version = PesVersion::Pes17;
        assert_term(
            &row_term(version, &face_row(IngameFaceField::NoseHeight, 3, 5)),
            "NoseHeight",
            "3",
            "5",
        );
        assert_term(
            &row_term(version, &field(PlayerField::EditedPlayer, 0, 1)),
            "EditedPlayer",
            "0",
            "1",
        );
        assert_term(
            &row_term(
                version,
                &PlayerDiff::FaceRun {
                    old: FaceHash([0; 4]),
                    new: FaceHash([0xff; 4]),
                },
            ),
            "face_run",
            "00000000",
            "ffffffff",
        );
    }

    /// Every table-driven view row has a term: a field added to a view
    /// without one falls back to the report's schema name, which is not
    /// its label — a drift this test fails on.
    #[test]
    fn every_table_driven_field_has_a_term() {
        let version = PesVersion::Pes17;
        for stat in abilities::SMALL_STATS
            .iter()
            .chain(fields::CUTSCENE_STATS.iter())
        {
            let term = row_term(version, &field(stat.field, 0, 0));
            assert_eq!(term.label, stat.label, "{:?}", stat.field);
        }
        for motion in fields::MOTIONS {
            let term = row_term(version, &field(motion.field, 0, 0));
            assert_eq!(term.label, motion.label, "{:?}", motion.field);
        }
        for (label, key, ..) in fields::PHYSIQUE {
            let field = key
                .player_field()
                .expect("a physique key's is a player field");
            let term = row_term(version, &self::field(field, 0, 0));
            assert_eq!(term.label, label, "{field:?}");
        }
        for (label, key, ..) in fields::STRIP_ENUMS {
            let field = key.player_field().expect("a strip key's is a player field");
            let term = row_term(version, &self::field(field, 0, 0));
            assert_eq!(term.label, label, "{field:?}");
        }
    }

    /// Team 701 of the PES 17 fixture (colours and kit slots live on 17),
    /// for `team_row_terms`.
    fn team_701() -> TeamEntry {
        EditFile::from_bytes(&fixture_save(PesVersion::Pes17))
            .expect("the fixture loads")
            .team(701)
            .expect("team 701")
            .clone()
    }

    /// A roster of named slots (the fixture's own names are all
    /// "PLACEHOLDER").
    fn named_rows() -> Vec<(String, String, Option<u8>)> {
        (0..32)
            .map(|i| (format!("Player {:02}", i + 1), String::new(), None))
            .collect()
    }

    /// `rows` as the `&[RosterRow]` the Tactics editor takes.
    fn roster_rows<'a>(rows: &'a [(String, String, Option<u8>)]) -> Vec<RosterRow<'a>> {
        rows.iter()
            .map(|(name, raw_name, position)| RosterRow {
                name: name.as_str(),
                raw_name: raw_name.as_str(),
                registered_position: *position,
                portrait: None,
                medal: None,
                overalls: None,
                rated: [false; 13],
                fielded: &[],
            })
            .collect()
    }

    /// A team colour's three channel diffs merge into the swatch's one
    /// row: `Col. 1`, the baseline's and the current's stored channels'
    /// hex as the view draws them.
    #[test]
    fn a_colour_edit_is_one_hex_row() {
        let version = PesVersion::Pes17;
        let baseline = team_701();
        assert_eq!(
            baseline
                .colors
                .map(|colors| [colors[0].red, colors[0].green, colors[0].blue]),
            Some([63, 42, 16]),
            "premise: the fixture's Colour 1 (63, 42, 16)"
        );
        let rows = named_rows();
        let roster = roster_rows(&rows);
        let mut team = baseline.clone();
        {
            let colors = team.colors.as_mut().expect("PES 17 stores colours");
            colors[0].red = 16;
            colors[0].green = 32;
            colors[0].blue = 48;
        }
        let terms = team_row_terms(
            version,
            &team,
            &baseline,
            &roster,
            &roster,
            &[
                TeamDiff::Field {
                    field: TeamField::Color1Red,
                    old: 63,
                    new: 16,
                },
                TeamDiff::Field {
                    field: TeamField::Color1Green,
                    old: 42,
                    new: 32,
                },
                TeamDiff::Field {
                    field: TeamField::Color1Blue,
                    old: 16,
                    new: 48,
                },
            ],
        );
        assert_eq!(terms.len(), 1, "the three channels merge");
        assert_term(&terms[0], "Col. 1", "#FFAA41", "#4182C2");

        // Only red changed: green and blue keep the stored ones.
        let mut team = baseline.clone();
        team.colors.as_mut().expect("PES 17 stores colours")[0].red = 0;
        let terms = team_row_terms(
            version,
            &team,
            &baseline,
            &roster,
            &roster,
            &[TeamDiff::Field {
                field: TeamField::Color1Red,
                old: 63,
                new: 0,
            }],
        );
        assert_eq!(terms.len(), 1);
        assert_term(&terms[0], "Col. 1", "#FFAA41", "#00AA41");
    }

    /// Colour 2's three channels merge into its swatch's one row the same
    /// way.
    #[test]
    fn colour_2s_channels_merge_into_one_hex_row() {
        let version = PesVersion::Pes17;
        let baseline = team_701();
        let stored = baseline
            .colors
            .map(|colors| [colors[1].red, colors[1].green, colors[1].blue])
            .expect("PES 17 stores colours");
        assert_eq!(stored, [0, 0, 0], "premise: the fixture's Colour 2");
        let rows = named_rows();
        let roster = roster_rows(&rows);
        let mut team = baseline.clone();
        {
            let colors = team.colors.as_mut().expect("PES 17 stores colours");
            colors[1].red = 16;
            colors[1].green = 32;
            colors[1].blue = 48;
        }
        let terms = team_row_terms(
            version,
            &team,
            &baseline,
            &roster,
            &roster,
            &[
                TeamDiff::Field {
                    field: TeamField::Color2Red,
                    old: u32::from(stored[0]),
                    new: 16,
                },
                TeamDiff::Field {
                    field: TeamField::Color2Green,
                    old: u32::from(stored[1]),
                    new: 32,
                },
                TeamDiff::Field {
                    field: TeamField::Color2Blue,
                    old: u32::from(stored[2]),
                    new: 48,
                },
            ],
        );
        assert_eq!(terms.len(), 1, "the three channels merge");
        assert_term(&terms[0], "Col. 2", "#000000", "#4182C2");
    }

    /// A name edit's two diffs in `compare_teams`' order: the side-effect
    /// flag as the report row, the `Team` row.
    #[test]
    fn a_name_edit_is_a_team_row_plus_the_flags_report_row() {
        let version = PesVersion::Pes17;
        let team = team_701();
        let rows = named_rows();
        let roster = roster_rows(&rows);
        let terms = team_row_terms(
            version,
            &team,
            &team,
            &roster,
            &roster,
            &[
                TeamDiff::Field {
                    field: TeamField::EditedName,
                    old: 0,
                    new: 1,
                },
                TeamDiff::Text {
                    text: TeamText::Name,
                    old: "/3/".to_owned(),
                    new: "/3/X".to_owned(),
                },
            ],
        );
        assert_eq!(terms.len(), 2);
        assert_term(&terms[0], "EditedName", "0", "1");
        assert_term(&terms[1], "Name", "/3/", "/3/X");
    }

    /// The team fields', the kit slots' and the roster's rows: the view's
    /// labels and the bound team shown as `binding / 64`, the shirt
    /// number under the strip's label, `Player(n)` in report terms.
    #[test]
    fn the_team_field_and_roster_rows() {
        let version = PesVersion::Pes17;
        let team = team_701();
        let rows = named_rows();
        let roster = roster_rows(&rows);
        let terms = team_row_terms(
            version,
            &team,
            &team,
            &roster,
            &roster,
            &[
                TeamDiff::Field {
                    field: TeamField::ManagerId,
                    old: 701,
                    new: 0,
                },
                TeamDiff::Field {
                    field: TeamField::StadiumId,
                    old: 29,
                    new: 0,
                },
                TeamDiff::Field {
                    field: TeamField::KitSlotNumber(0),
                    old: 0,
                    new: 128,
                },
                TeamDiff::Field {
                    field: TeamField::KitSlotTeam(0),
                    old: 701 * 64,
                    new: 702 * 64,
                },
                TeamDiff::Roster {
                    field: RosterField::Number(2),
                    old: 3,
                    new: 9,
                },
                TeamDiff::Roster {
                    field: RosterField::Player(2),
                    old: 70103,
                    new: 0,
                },
            ],
        );
        assert_eq!(terms.len(), 6);
        assert_term(&terms[0], "Manager ID", "701", "0");
        assert_term(&terms[1], "Stadium ID", "29", "0");
        assert_term(&terms[2], "Slot 1 · Number", "0", "128");
        assert_term(&terms[3], "Slot 1 · Bound team ID", "701", "702");
        assert_term(&terms[4], "Shirt number", "3", "9");
        assert_term(&terms[5], "Player(2)", "70103", "0");
    }

    /// A tactics row goes through the Tactics editor's terms.
    #[test]
    fn a_tactics_row_uses_the_tactics_terms() {
        let version = PesVersion::Pes17;
        let team = team_701();
        let rows = named_rows();
        let roster = roster_rows(&rows);
        let terms = team_row_terms(
            version,
            &team,
            &team,
            &roster,
            &roster,
            &[
                TeamDiff::Tactics {
                    field: TacticsField::Captain,
                    old: 0,
                    new: 5,
                },
                TeamDiff::Tactics {
                    field: TacticsField::Preset {
                        preset: 0,
                        field: PresetField::SupportRange,
                    },
                    old: 5,
                    new: 7,
                },
            ],
        );
        assert_eq!(terms.len(), 2);
        assert_term(&terms[0], "Captain", "Player 01", "Player 06");
        assert_term(&terms[1], "Preset 1 · Support range", "5", "7");
    }
}
