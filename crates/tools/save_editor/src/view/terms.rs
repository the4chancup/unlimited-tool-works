//! The views' terms for a `diff` row: a field a view draws reads under the
//! view's own label, its old and new values as that view shows them, since
//! a member reads a row against the view it jumps to. So a label or scale
//! changed in a view changes the row with it. A field no view draws is
//! named in words from the report's name, its stored values kept
//! (`field_words` over `messages::diff_fields`). The same terms word the
//! unsaved-changes table's team rows (`team_row_terms`) and the `diff`
//! report's text.

use pes_savefile::interchange::team_toml::labels::{
    ABILITY_STATS, COM_STYLES, PLAYABLE_POSITIONS, POSITIONS, SKILLS, card_name, stat_name,
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
pub(crate) struct Term {
    /// The field's label: the view's own where a view draws the field.
    pub(crate) label: String,
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

/// `diff`'s row in the views' terms. `pub(crate)` for the `diff` report's
/// text, which names each field by this label.
pub(crate) fn row_term(version: PesVersion, diff: &PlayerDiff) -> Term {
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
        // The run's fingerprint row is drawn by no view, and stands for the
        // face's bits no field names.
        PlayerDiff::FaceRun { .. } => {
            let (_, old, new) = diff_fields(diff);
            term(FACE_RUN_LABEL, old, new)
        }
    }
}

/// The fingerprint row's label: the face's bits no field names.
const FACE_RUN_LABEL: &str = "Other face data";

/// The row for a field no view draws: the field named in words from the
/// report's name (`field_words`), its stored values.
fn report_term(diff: &PlayerDiff) -> Term {
    let (name, old, new) = diff_fields(diff);
    term(field_words(&name), old, new)
}

/// `diffs`' rows in the views' terms, for the unsaved-changes prompt: one
/// `Term` per row in `diffs`' order, beside the diffs it shows (what the
/// row's X puts back) — a team colour's three channel diffs merge into the
/// swatch's one row, at the first channel's place. `team`
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
) -> Vec<(Term, Vec<TeamDiff>)> {
    let mut terms = Vec::with_capacity(diffs.len());
    let mut colours_done = [false; 2];
    for diff in diffs {
        match colour_index(diff) {
            Some(colour) if !colours_done[colour] => {
                colours_done[colour] = true;
                let channels = diffs
                    .iter()
                    .filter(|other| colour_index(other) == Some(colour))
                    .cloned()
                    .collect();
                terms.push((colour_term(baseline_team, team, colour), channels));
            }
            // A later channel of an already merged colour is no row.
            Some(_) => {}
            None => terms.push((
                team_row_term(version, team, baseline_team, old_roster, new_roster, diff),
                vec![diff.clone()],
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
            // handled): named in words, as players' undrawn fields are.
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
            RosterField::Number(slot) => {
                numbers(format!("{}'s number", slot_label(*slot)), *old, *new)
            }
            RosterField::Player(slot) => numbers(slot_label(*slot), *old, *new),
            RosterField::TeamId => team_report_term(diff),
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

/// The row for a team field no view draws, `report_term`'s twin: the field
/// named in words from its schema name, its stored values.
fn team_report_term(diff: &TeamDiff) -> Term {
    let (name, old, new) = match diff {
        TeamDiff::Field { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Roster { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Tactics { field, old, new } => report_fields(field, *old, *new),
        TeamDiff::Text { text, old, new } => (format!("{text:?}"), old.clone(), new.clone()),
    };
    term(field_words(&name), old, new)
}

/// A report row's three strings for a named field: the schema name and
/// the two stored values.
fn report_fields(field: impl std::fmt::Debug, old: u32, new: u32) -> (String, String, String) {
    (format!("{field:?}"), old.to_string(), new.to_string())
}

/// A `Field` row's term: the views' own tables first (`SMALL_STATS`,
/// `PRESENTATION_STATS`, `MOTIONS`, `PHYSIQUE`, `STRIP_ENUMS`, the ability
/// stats), then the fields the views draw one-off, then a field no view
/// draws named in words — the `_` arm IS the rule for those, not an
/// oversight.
fn player_field_term(version: PesVersion, diff: &PlayerDiff) -> Term {
    let PlayerDiff::Field { field, old, new } = *diff else {
        unreachable!("player_field_term gets a Field row")
    };
    if let Some(small) = abilities::SMALL_STATS
        .iter()
        .chain(fields::PRESENTATION_STATS.iter())
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
            card_name(SKILLS[usize::from(index)]),
            on_off(old),
            on_off(new),
        ),
        PlayerField::ComStyle(index) => term(
            card_name(COM_STYLES[usize::from(index)]),
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
        // among them): named in words.
        _ => report_term(diff),
    }
}

/// A `Face` row's term: the gloves dropdown and the palettes are drawn,
/// the rest of the run's fields are not (named in words).
fn face_field_term(version: PesVersion, diff: &PlayerDiff) -> Term {
    let PlayerDiff::Face { field, old, new } = *diff else {
        unreachable!("face_field_term gets a Face row")
    };
    match field {
        IngameFaceField::PlayerGloves => Term {
            label: appearance::OUTFIELDER_GLOVES_LABEL.to_owned(),
            old: gloves_reading(old).to_owned(),
            new: gloves_reading(new).to_owned(),
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

/// `stored`'s reading in the gloves dropdown: any non-zero is worn (the
/// flag's reading, as the view words it).
fn gloves_reading(stored: u8) -> &'static str {
    appearance::GLOVES_OPTIONS[usize::from(stored != 0)]
}

/// A field no view draws, named in words from its report name: a word
/// starts at a capital or a digit after a small letter, and at a capital
/// after a digit; the words in sentence case, Id as ID, Com as COM and
/// Color as the window spells them, a PES 2015-only field marked so
/// (`NoseHeight` is "Nose height", `Color1Red` "Color 1 red",
/// `EditedComStyles` "Edited COM styles", `Pes15JawDepth` "Jaw depth
/// (PES 2015)"). The draft's `fieldWords`.
fn field_words(name: &str) -> String {
    let (name, pes15) = match name.strip_prefix("Pes15") {
        Some(rest) => (rest, true),
        None => (name, false),
    };
    let mut spaced = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if let Some(previous) = previous
            && ((previous.is_ascii_lowercase() && (c.is_ascii_uppercase() || c.is_ascii_digit()))
                || (previous.is_ascii_digit() && c.is_ascii_uppercase()))
        {
            spaced.push(' ');
        }
        spaced.push(c);
        previous = Some(c);
    }
    let words: Vec<String> = spaced
        .split(' ')
        .map(|word| match word {
            "Id" => "ID".to_owned(),
            "Com" => "COM".to_owned(),
            _ => word.to_lowercase(),
        })
        .collect();
    let mut named = words.join(" ");
    if let Some(first) = named.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    if pes15 {
        named.push_str(" (PES 2015)");
    }
    named
}

/// A roster slot as the player list numbers it, from 01: slot 3 is
/// "Slot 04".
fn slot_label(slot: u8) -> String {
    format!("Slot {:02}", u32::from(slot) + 1)
}

/// The `SettingKey` that `writes` (`keys.rs`' `player_field` or
/// `face_field` compared with a field): the key table's own binding, so a
/// key remapped to another field remaps here.
fn key_writing(writes: impl Fn(SettingKey) -> bool) -> Option<SettingKey> {
    SettingKey::ALL.iter().copied().find(|key| writes(*key))
}

#[cfg(test)]
mod tests;
