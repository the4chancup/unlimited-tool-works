//! `team.toml`'s player half: `[players.NN]` and its `stats`, `positions`,
//! `skills`, `edit_flags` and `appearance` subtables. The scalar keys run
//! through `player_keys`' table; the non-scalar keys (names, number,
//! `playable`, `playing_style`, the skill arrays and `appearance`) are
//! handled here.

use std::collections::{BTreeMap, HashSet};

use pes_version::PesVersion;
use toml_edit::{Item, TableLike, Value};

use crate::codec::{TextMisfit, text_max, text_misfit};
use crate::interchange::players_key;
use crate::interchange::team_toml::labels;
use crate::interchange::team_toml::player_keys::PlayerKey;
use crate::interchange::team_toml::{
    ImportNote, PlayerSection, PositionsSection, SkillsSection, TeamTomlError,
};
use crate::model::names::{crop_player_text, single_byte};
use crate::model::player::PlayerEntry;
use crate::model::playstyle::PlayStyle;
use crate::model::team::{TeamEntry, roster_order_player_id};
use crate::schema::fields::{PlayerField, PlayerText, RosterField};
use crate::schema::ingame_face::INGAME_FACE_FIELDS;
use crate::schema::limits::{face_type_cap, motion_top};
use crate::schema::{bit_width, playstyle, portable_text_max, schema_for};
use crate::settings_toml::document::{
    AppearanceDocument, emit_appearance, leaf_path, padded, parse_appearance,
};
use crate::settings_toml::keys::{Kind, SettingKey, Source};
use crate::settings_toml::{
    AppearanceSettings, appearance_from, apply_appearance, get_appearance, held_on, set_appearance,
};

use super::items::{as_table, check_text, check_width, emit, gated, opt, reject, text, u16_val};

// ---------------------------------------------------------------------------
// From the model

/// The `[players.NN]` map of `team`'s rostered players, keyed by 1-based
/// roster slot. An empty `players` is a tactics-only dump — no sections. A
/// non-empty pool that lacks a rostered id is `PlayerMissing`: a partial pool
/// is a caller bug, and silently dropping players is the loss this crate
/// exists to avoid.
pub(super) fn players_from(
    version: PesVersion,
    team: &TeamEntry,
    players: &[&PlayerEntry],
) -> Result<BTreeMap<u8, PlayerSection>, TeamTomlError> {
    let mut out = BTreeMap::new();
    if players.is_empty() {
        return Ok(out);
    }
    // The gate is a set membership, not a schema scan per key per player.
    let fields = schema_for(version).player_field_set();
    for (i, slot) in team.roster.iter().enumerate() {
        if slot.player_id == 0 {
            continue;
        }
        let player = players
            .iter()
            .find(|player| player.id == slot.player_id)
            .ok_or(TeamTomlError::PlayerMissing { id: slot.player_id })?;
        let key = u8::try_from(i + 1).expect("a roster slot index fits u8");
        out.insert(key, player_from(version, slot.number, player, &fields)?);
    }
    Ok(out)
}

/// One roster slot's `PlayerSection`: every field `Some` where the model
/// carries it (a version-gated `Option` stays `None` and emits commented).
fn player_from(
    version: PesVersion,
    number: u16,
    player: &PlayerEntry,
    fields: &HashSet<PlayerField>,
) -> Result<PlayerSection, TeamTomlError> {
    let appearance = appearance_from(player)?;
    let mut section = PlayerSection {
        name: Some(player.name.clone()),
        shirt_name: Some(player.shirt_name.clone()),
        number: Some(number),
        positions: PositionsSection {
            playable: Some(player.positions.playable),
            playing_style: Some(playstyle::decode(version, player.positions.playing_style)?),
            ..PositionsSection::default()
        },
        skills: SkillsSection {
            skills: Some(player.skills.skills),
            com_styles: Some(player.skills.com_styles),
        },
        appearance,
        ..PlayerSection::default()
    };
    for key in PlayerKey::ALL {
        if !fields.contains(&key.spec().field) {
            continue;
        }
        if let Some(value) = key.get(player) {
            key.set_section(&mut section, value)?;
        }
    }
    Ok(section)
}

// ---------------------------------------------------------------------------
// Emit

/// Every `PlayerSection`, in slot order. `team_id` and `roster_breaks`
/// feed the breaking slots' header note (`roster_note`).
pub(super) fn emit_players(
    out: &mut String,
    players: &BTreeMap<u8, PlayerSection>,
    team_id: Option<u32>,
    roster_breaks: &[(u8, u32)],
) -> Result<(), TeamTomlError> {
    for (slot, section) in players {
        emit_player(out, *slot, section, team_id, roster_breaks)?;
    }
    Ok(())
}

/// One scalar key's line through the table.
fn emit_key(
    out: &mut String,
    section: &PlayerSection,
    key: PlayerKey,
) -> Result<(), TeamTomlError> {
    let spec = key.spec();
    let value = key.get_section(section).map(|v| key.text(v)).transpose()?;
    emit(out, key.name(), value, &key.neutral(), spec.comment);
    Ok(())
}

/// `path` with a `%02d` slot substituted for `NN`.
fn player_path(slot: u8) -> String {
    format!("players.{slot:02}")
}

fn emit_player(
    out: &mut String,
    slot: u8,
    section: &PlayerSection,
    team_id: Option<u32>,
    roster_breaks: &[(u8, u32)],
) -> Result<(), TeamTomlError> {
    let path = player_path(slot);
    out.push('\n');
    out.push_str(&padded(
        &format!("[{path}]"),
        &format!(
            "roster slot 01 .. 40 (32 on PES 15-18){}",
            roster_note(slot, team_id, roster_breaks)
        ),
    ));
    out.push('\n');
    emit(
        out,
        "name",
        section
            .name
            .as_ref()
            .map(|n| Value::from(n.as_str()).to_string()),
        "\"\"",
        &portable_note(PlayerText::Name, section.name.as_deref()),
    );
    emit(
        out,
        "shirt_name",
        section
            .shirt_name
            .as_ref()
            .map(|n| Value::from(n.as_str()).to_string()),
        "\"\"",
        &portable_note(PlayerText::ShirtName, section.shirt_name.as_deref()),
    );
    emit(
        out,
        "number",
        section.number.map(|v| v.to_string()),
        "1",
        "shirt number, the roster's",
    );
    for key in PlayerKey::ALL {
        if key.table().is_empty() {
            emit_key(out, section, key)?;
        }
    }

    out.push_str(&format!("\n[{path}.stats]\n"));
    for key in PlayerKey::ALL {
        if key.table() == "stats" {
            emit_key(out, section, key)?;
        }
    }

    out.push_str(&format!("\n[{path}.positions]\n"));
    emit_key(out, section, PlayerKey::Registered)?;
    emit(
        out,
        "playable",
        section.positions.playable.map(playable_text),
        &playable_text([0; 13]),
        "\"C\" | \"B\" | \"A\"",
    );
    emit(
        out,
        "playing_style",
        section
            .positions
            .playing_style
            .map(play_style_text)
            .transpose()?,
        "\"none\"",
        "canonical PlayStyle name; \"none\" for none",
    );
    emit_key(out, section, PlayerKey::StrongerFoot)?;
    emit_key(out, section, PlayerKey::StrongerHand)?;

    out.push_str(&format!("\n[{path}.skills]\n"));
    emit(
        out,
        "skills",
        section
            .skills
            .skills
            .map(|flags| flag_list(flags, &labels::SKILLS)),
        "[]",
        "the set skills, canonical names",
    );
    emit(
        out,
        "com_styles",
        section
            .skills
            .com_styles
            .map(|flags| flag_list(flags, &labels::COM_STYLES)),
        "[]",
        "\"trickster\" \"mazing_run\" \"speeding_bullet\" \"incisive_run\" \"long_ball_expert\" \"early_cross\" \"long_ranger\"",
    );

    out.push_str(&format!(
        "\n[{path}.edit_flags]         # the game's own flags, all bool\n"
    ));
    for key in PlayerKey::ALL {
        if key.table() == "edit_flags" {
            emit_key(out, section, key)?;
        }
    }

    emit_appearance(
        out,
        &format!("{path}.appearance"),
        &section.appearance,
        AppearanceDocument::Team,
    )?;
    Ok(())
}

/// The `;`-joined addition to a `[players.NN]` header's comment when the
/// slot breaks the roster order: the player the slot holds and the one the
/// rule puts there. A Team TOML lands players by slot, so the record would
/// no longer match the aesthetics export that names it by id; the note is
/// how a member learns that before applying the file. Empty when the slot
/// keeps the order or the document carries no ids to check (a parsed or
/// hand-built one).
fn roster_note(slot: u8, team_id: Option<u32>, roster_breaks: &[(u8, u32)]) -> String {
    let Some(team_id) = team_id else {
        return String::new();
    };
    // A `players` map can carry 0 (`parse` refuses it, a built document
    // need not), as `apply_player` guards too.
    let Some(index) = slot.checked_sub(1) else {
        return String::new();
    };
    let Some(&(_, held)) = roster_breaks.iter().find(|(i, _)| *i == index) else {
        return String::new();
    };
    format!(
        "; holds player {held}, not {}: applied, this record lands on slot \
         {slot:02}, apart from {held}'s aesthetics export",
        roster_order_player_id(team_id, index)
    )
}

/// The note a `name`/`shirt_name` line carries when the text is past the
/// portable limit (`spec/pes_savefile/operations.md` "Team TOML"): which
/// versions would crop it. Empty when it fits.
fn portable_note(which: PlayerText, text: Option<&str>) -> String {
    let Some(text) = text else {
        return String::new();
    };
    let (len, note) = match which {
        PlayerText::Name => (
            text.len(),
            format!(
                "name past {} bytes: cut on PES 2015 to 2019",
                portable_text_max(PlayerText::Name)
            ),
        ),
        PlayerText::ShirtName => (
            text.chars().count(),
            format!(
                "shirt name past {} letters: cut on PES 2015 to 2019 (PES 2016: past 16)",
                portable_text_max(PlayerText::ShirtName)
            ),
        ),
    };
    if len > portable_text_max(which) {
        note
    } else {
        String::new()
    }
}

/// `{ CF = "A", SS = "B", …, GK = "C" }` — `playable`'s order
/// (`labels::PLAYABLE_POSITIONS`).
fn playable_text(playable: [u8; 13]) -> String {
    let entries: Vec<String> = (0..13)
        .map(|i| {
            let position = labels::POSITIONS[labels::PLAYABLE_POSITIONS[i]];
            let rating = labels::RATINGS[usize::from(playable[i])];
            format!("{position} = \"{rating}\"")
        })
        .collect();
    format!("{{ {} }}", entries.join(", "))
}

/// `"goal_poacher"` — the canonical label.
fn play_style_text(style: PlayStyle) -> Result<String, TeamTomlError> {
    let index = PlayStyle::ALL
        .iter()
        .position(|s| *s == style)
        .expect("a canonical style is in ALL");
    Ok(format!("\"{}\"", labels::PLAY_STYLES[index]))
}

/// `["a", "b"]` of the set flags' names, canonical order.
fn flag_list<const N: usize>(flags: [bool; N], labels: &[&'static str]) -> String {
    let names: Vec<String> = labels
        .iter()
        .enumerate()
        .filter(|(i, _)| flags[*i])
        .map(|(_, name)| format!("\"{name}\""))
        .collect();
    format!("[{}]", names.join(", "))
}

// ---------------------------------------------------------------------------
// Parse

/// The `[players]` table: keys are exactly two digits `01` to `40`, anything
/// else is `UnknownKey` ("players.7", "players.41", "players.bob").
pub(super) fn parse_players(item: &Item) -> Result<BTreeMap<u8, PlayerSection>, TeamTomlError> {
    let table = as_table(item, "players")?;
    let mut out = BTreeMap::new();
    for (name, item) in table.iter() {
        if !players_key(name, 1..=40) {
            return Err(TeamTomlError::UnknownKey {
                key: format!("players.{name}"),
            });
        }
        let slot = name.parse::<u8>().expect("checked above");
        out.insert(slot, parse_player(item, &player_path(slot))?);
    }
    Ok(out)
}

fn parse_player(item: &Item, path: &str) -> Result<PlayerSection, TeamTomlError> {
    let table = as_table(item, path)?;
    let stats = sub_table(table, path, "stats")?;
    let positions = sub_table(table, path, "positions")?;
    let skills = sub_table(table, path, "skills")?;
    let edit_flags = sub_table(table, path, "edit_flags")?;
    let mut section = PlayerSection {
        name: opt(table, path, "name", text)?,
        shirt_name: opt(table, path, "shirt_name", text)?,
        number: opt(table, path, "number", u16_val)?,
        ..PlayerSection::default()
    };
    for key in PlayerKey::ALL {
        let (sub, subpath) = match key.table() {
            "" => (Some(table), path.to_string()),
            "stats" => (stats, format!("{path}.stats")),
            "positions" => (positions, format!("{path}.positions")),
            "edit_flags" => (edit_flags, format!("{path}.edit_flags")),
            other => unreachable!("the key table only names known tables: {other}"),
        };
        let Some(sub) = sub else { continue };
        if let Some(value) = opt(sub, &subpath, key.name(), |i, k| key.parse(i, k))? {
            key.set_section(&mut section, value)?;
        }
    }
    if let Some(t) = positions {
        let positions_path = format!("{path}.positions");
        section.positions.playable = opt(t, &positions_path, "playable", playable)?;
        section.positions.playing_style = opt(t, &positions_path, "playing_style", play_style)?;
        reject(
            t,
            &positions_path,
            &[
                "registered",
                "playable",
                "playing_style",
                "stronger_foot",
                "stronger_hand",
            ],
        )?;
    }
    if let Some(t) = skills {
        let skills_path = format!("{path}.skills");
        section.skills.skills = opt(t, &skills_path, "skills", |i, k| {
            flag_array(i, k, &labels::SKILLS)
        })?;
        section.skills.com_styles = opt(t, &skills_path, "com_styles", |i, k| {
            flag_array(i, k, &labels::COM_STYLES)
        })?;
        reject(t, &skills_path, &["skills", "com_styles"])?;
    }
    if let Some(t) = stats {
        reject(t, &format!("{path}.stats"), &stats_names())?;
    }
    if let Some(t) = edit_flags {
        reject(t, &format!("{path}.edit_flags"), &flag_names())?;
    }
    if let Some(item) = table.get("appearance") {
        section.appearance = parse_appearance(
            item,
            &format!("{path}.appearance"),
            AppearanceDocument::Team,
        )?;
    }
    reject(
        table,
        path,
        &[
            "name",
            "shirt_name",
            "number",
            "nationality",
            "age",
            "height",
            "weight",
            "boots_id",
            "gloves_id",
            "base_copy_id",
            "stats",
            "positions",
            "skills",
            "edit_flags",
            "appearance",
        ],
    )?;
    Ok(section)
}

/// `table.name` as a table-like, `None` when absent, `WrongType` when not a
/// table.
fn sub_table<'a>(
    table: &'a dyn TableLike,
    path: &str,
    name: &str,
) -> Result<Option<&'a dyn TableLike>, TeamTomlError> {
    let Some(item) = table.get(name) else {
        return Ok(None);
    };
    item.as_table_like()
        .map(Some)
        .ok_or_else(|| TeamTomlError::WrongType {
            key: format!("{path}.{name}"),
            expected: "a table",
        })
}

/// The `[players.NN.stats]` key names.
fn stats_names() -> Vec<&'static str> {
    PlayerKey::ALL
        .iter()
        .filter(|key| key.table() == "stats")
        .map(|key| key.name())
        .collect()
}

/// The `[players.NN.edit_flags]` key names.
fn flag_names() -> Vec<&'static str> {
    PlayerKey::ALL
        .iter()
        .filter(|key| key.table() == "edit_flags")
        .map(|key| key.name())
        .collect()
}

/// `playable = { GK = "C", …, CF = "A" }`: every position required, each a
/// rating label; `GK`'s value is `playable[12]` (`labels::PLAYABLE_POSITIONS`).
fn playable(item: &Item, key: &str) -> Result<[u8; 13], TeamTomlError> {
    let table = item
        .as_table_like()
        .ok_or_else(|| TeamTomlError::WrongType {
            key: key.to_string(),
            expected: "an inline table of thirteen position ratings",
        })?;
    let mut out = [0u8; 13];
    for (index, position) in labels::POSITIONS.iter().enumerate() {
        let Some(item) = table.get(position) else {
            return Err(TeamTomlError::MissingKey {
                key: format!("{key}.{position}"),
            });
        };
        let rating = label_index(item, &format!("{key}.{position}"), &labels::RATINGS)?;
        let i = labels::PLAYABLE_POSITIONS
            .iter()
            .position(|&entry| entry == index)
            .expect("PLAYABLE_POSITIONS is a permutation");
        out[i] = rating;
    }
    reject(table, key, &labels::POSITIONS)?;
    Ok(out)
}

/// A string that is one of `labels`, as its index; a stranger is
/// `UnknownLabel`.
fn label_index(item: &Item, key: &str, labels: &[&str]) -> Result<u8, TeamTomlError> {
    let text =
        item.as_value()
            .and_then(|v| v.as_str())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "a string",
            })?;
    labels
        .iter()
        .position(|label| *label == text)
        .map(|index| u8::try_from(index).expect("a label index fits u8"))
        .ok_or_else(|| TeamTomlError::UnknownLabel {
            key: key.to_string(),
            label: text.to_string(),
            allowed: labels
                .iter()
                .map(|label| format!("\"{label}\""))
                .collect::<Vec<_>>()
                .join(", "),
        })
}

/// `playing_style = "goal_poacher"`: a canonical `PlayStyle` label.
fn play_style(item: &Item, key: &str) -> Result<PlayStyle, TeamTomlError> {
    let index = label_index(item, key, &labels::PLAY_STYLES)?;
    Ok(PlayStyle::ALL[usize::from(index)])
}

/// An array of unique label names → the flag array with those slots set. A
/// duplicate or a name the list lacks is `UnknownLabel`.
fn flag_array<const N: usize>(
    item: &Item,
    key: &str,
    labels: &[&'static str],
) -> Result<[bool; N], TeamTomlError> {
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| TeamTomlError::WrongType {
                key: key.to_string(),
                expected: "an array of names",
            })?;
    let mut out = [false; N];
    for value in array.iter() {
        let index = usize::from(label_index(&Item::Value(value.clone()), key, labels)?);
        let slot = out
            .get_mut(index)
            .expect("the label table and the flag array share a length");
        if *slot {
            return Err(TeamTomlError::UnknownLabel {
                key: key.to_string(),
                label: labels[index].to_string(),
                allowed: labels
                    .iter()
                    .map(|label| format!("\"{label}\""))
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
        *slot = true;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Apply

/// Applies every `[players.NN]`: `NN` addresses `team`'s roster slot; an
/// empty slot is an `EmptySlot` note, a slot past the roster's width is
/// `NoSuchSlot`, a rostered id missing from `players` is `PlayerMissing`.
/// `team_id` is `[team] id` for the base-copy own-id rule.
/// One `apply` call's immutable context: the target version, whether the
/// document is foreign, its schema, and `[team] id`.
struct Context {
    to: PesVersion,
    foreign: bool,
    /// The target's stored `PlayerField`s, built once per `apply`.
    fields: HashSet<PlayerField>,
    /// The source document's stored `PlayerField`s: a key the source version
    /// cannot store says nothing about the target's value.
    from_fields: HashSet<PlayerField>,
    team_id: Option<u32>,
}

/// The context plus the things being mutated.
struct Target<'a> {
    context: Context,
    team: &'a mut TeamEntry,
    players: &'a mut [PlayerEntry],
    notes: &'a mut Vec<ImportNote>,
}

pub(super) fn apply_players(
    sections: &BTreeMap<u8, PlayerSection>,
    team_id: Option<u32>,
    from: PesVersion,
    to: PesVersion,
    team: &mut TeamEntry,
    players: &mut [PlayerEntry],
    notes: &mut Vec<ImportNote>,
) -> Result<(), TeamTomlError> {
    let foreign = from != to;
    let mut target = Target {
        context: Context {
            to,
            foreign,
            fields: schema_for(to).player_field_set(),
            from_fields: schema_for(from).player_field_set(),
            team_id,
        },
        team,
        players,
        notes,
    };
    for (slot, section) in sections {
        apply_player(section, *slot, &mut target)?;
    }
    Ok(())
}

fn apply_player(
    section: &PlayerSection,
    slot: u8,
    target: &mut Target,
) -> Result<(), TeamTomlError> {
    let path = player_path(slot);
    // `slot` is the document's key, 1-based; a `players` map can carry 0
    // (`parse` refuses it, a built document need not).
    let Some(index) = slot.checked_sub(1).map(usize::from) else {
        return Err(TeamTomlError::NoSuchSlot { slot });
    };
    if index >= target.team.roster.len() {
        return Err(TeamTomlError::NoSuchSlot { slot });
    }
    let player_id = target.team.roster[index].player_id;
    if player_id == 0 {
        target.notes.push(ImportNote::EmptySlot { slot });
        return Ok(());
    }
    // Checks first: a stored value the target's field cannot hold would fail
    // `write_player` after `apply` returned Ok, so it is refused up front.
    for key in PlayerKey::ALL {
        let Some(value) = key.get_section(section) else {
            continue;
        };
        let Some(width) = bit_width(target.context.to, key.spec().field) else {
            continue;
        };
        check_width(&format!("{path}.{}", key.spec().path), value, width)?;
    }
    let Some(player) = target.players.iter_mut().find(|p| p.id == player_id) else {
        return Err(TeamTomlError::PlayerMissing { id: player_id });
    };
    if let Some(number) = section.number {
        // The roster's number field is 8 bits on the ≤ 18 schemas; a wider
        // value would fail `write_roster` after `apply` said Ok, so it is
        // refused here instead of clamped.
        let max = schema_for(target.context.to)
            .roster
            .arrays
            .iter()
            .find(|a| matches!((a.make)(0), RosterField::Number(0)))
            .map_or(u32::from(u16::MAX), |a| (1 << a.bit_width) - 1);
        if u32::from(number) > max {
            return Err(TeamTomlError::OutOfRange {
                key: format!("{path}.number"),
                value: i64::from(number),
                range: format!("0 to {max}"),
            });
        }
        target.team.roster[index].number = number;
    }
    if let Some(name) = &section.name {
        player.name = apply_text(
            &format!("{path}.name"),
            PlayerText::Name,
            name,
            target.context.to,
            target.notes,
        )?;
    }
    if let Some(shirt_name) = &section.shirt_name {
        player.shirt_name = apply_text(
            &format!("{path}.shirt_name"),
            PlayerText::ShirtName,
            shirt_name,
            target.context.to,
            target.notes,
        )?;
    }
    for key in PlayerKey::ALL {
        let Some(value) = key.get_section(section) else {
            continue;
        };
        let spec = key.spec();
        if !gated(
            target.notes,
            target.context.foreign,
            target.context.fields.contains(&spec.field),
            &format!("{path}.{}", spec.path),
        )? {
            continue;
        }
        let value = if key == PlayerKey::BaseCopyId
            && target.context.team_id.is_some_and(|id| {
                id.checked_mul(100)
                    .and_then(|base| base.checked_add(u32::from(slot)))
                    == Some(value)
            }) {
            // The file's own-id convention: equal to team id x 100 + slot
            // means "unset" — the target's own id stands in.
            player.id
        } else {
            value
        };
        key.set(player, value)?;
    }
    if let Some(playable) = section.positions.playable {
        player.positions.playable = playable;
    }
    if let Some(style) = section.positions.playing_style {
        player.positions.playing_style = match playstyle::encode(target.context.to, style) {
            Some(stored) => stored,
            None => {
                let index = PlayStyle::ALL
                    .iter()
                    .position(|s| *s == style)
                    .expect("a canonical style is in ALL");
                target.notes.push(ImportNote::NotEncodable {
                    path: format!("{path}.positions.playing_style"),
                    label: labels::PLAY_STYLES[index].to_string(),
                });
                playstyle::encode(target.context.to, PlayStyle::None)
                    .expect("every version's style list holds PlayStyle::None")
            }
        };
    }
    if let Some(skills) = section.skills.skills {
        for (i, set) in skills.iter().enumerate() {
            let field = PlayerField::Skill(u8::try_from(i).expect("41 skills fit u8"));
            // A skill the source version cannot store says nothing — the
            // target's value stands, note-free (a pair-wide rule).
            if !target.context.from_fields.contains(&field) {
                continue;
            }
            if *set {
                if gated(
                    target.notes,
                    target.context.foreign,
                    target.context.fields.contains(&field),
                    &format!("{path}.skills.skills.{}", labels::SKILLS[i]),
                )? {
                    player.skills.skills[i] = true;
                }
            } else if target.context.fields.contains(&field) {
                player.skills.skills[i] = false;
            }
        }
    }
    if let Some(com_styles) = section.skills.com_styles {
        for (i, set) in com_styles.iter().enumerate() {
            let field = PlayerField::ComStyle(u8::try_from(i).expect("7 com styles fit u8"));
            if !target.context.from_fields.contains(&field) {
                continue;
            }
            if *set {
                if gated(
                    target.notes,
                    target.context.foreign,
                    target.context.fields.contains(&field),
                    &format!("{path}.skills.com_styles.{}", labels::COM_STYLES[i]),
                )? {
                    player.skills.com_styles[i] = true;
                }
            } else if target.context.fields.contains(&field) {
                player.skills.com_styles[i] = false;
            }
        }
    }
    let context = &target.context;
    apply_appearance_table(&section.appearance, player, context, target.notes, &path)?;
    Ok(())
}

/// `apply`'s `name`/`shirt_name` rule (`spec/pes_savefile/operations.md`
/// "Team TOML"): a text longer than `version`'s `which` field writes its
/// `crop_player_text` result (whole characters and colour codes) with a
/// `TextCropped` note naming the path and the text as cut, never a refusal;
/// a text the field refuses for content is the `check_text` error. Returns
/// the text to store.
fn apply_text(
    path: &str,
    which: PlayerText,
    text: &str,
    version: PesVersion,
    notes: &mut Vec<ImportNote>,
) -> Result<String, TeamTomlError> {
    let max = text_max(schema_for(version).player.texts, which);
    if matches!(
        text_misfit(text, max, single_byte(which)),
        Some(TextMisfit::TooLong { .. })
    ) {
        let cropped = crop_player_text(version, which, text).into_owned();
        notes.push(ImportNote::TextCropped {
            path: path.to_string(),
            cropped: cropped.clone(),
        });
        return Ok(cropped);
    }
    check_text(path, text, max, single_byte(which))?;
    Ok(text.to_string())
}

/// The `[players.NN.appearance]` keys onto `player`. A foreign document's
/// keys the target cannot hold are pre-adjusted on a clone: a `PlayerField`
/// the version lacks, or a face or hair key of the other layout (`held_on`),
/// is dropped with a `NotInThisVersion` note, a face type
/// over `face_type_cap` resets to 0 with a `Capped` note, and skin 7 on a
/// target without a custom skin becomes 1.
/// Same-version documents write the values as stored (a
/// `SettingsError` — a `Some` the version lacks — propagates).
fn apply_appearance_table(
    section: &AppearanceSettings,
    player: &mut PlayerEntry,
    context: &Context,
    notes: &mut Vec<ImportNote>,
    path: &str,
) -> Result<(), TeamTomlError> {
    let mut appearance = section.clone();
    if context.foreign {
        for key in SettingKey::ALL {
            let Some(value) = get_appearance(&appearance, key) else {
                continue;
            };
            let leaf = leaf_path(&format!("{path}.appearance"), &key.spec());
            match key.source() {
                Source::Player(field) => {
                    if !context.fields.contains(&field) {
                        set_appearance(&mut appearance, key, None);
                        notes.push(ImportNote::NotInThisVersion { path: leaf });
                    } else if let Some(top) = motion_top(context.to, field)
                        && value > top
                    {
                        // A motion past the target's top is capped, stored
                        // value against the stored top (`motion_top`'s
                        // units); the note reports the document's numbers
                        // (a 1-based key's stored value plus one, a
                        // celebration's as stored).
                        set_appearance(&mut appearance, key, Some(top));
                        let shown = |v: u8| match key.spec().kind {
                            Kind::OneBased { .. } => v.saturating_add(1),
                            _ => v,
                        };
                        notes.push(ImportNote::Capped {
                            path: leaf,
                            from: shown(value),
                            to: shown(top),
                        });
                    }
                }
                Source::Face(_) | Source::NoseAngle if !held_on(key, context.to) => {
                    set_appearance(&mut appearance, key, None);
                    notes.push(ImportNote::NotInThisVersion { path: leaf });
                }
                Source::NoseAngle => {}
                Source::Face(field) => {
                    if let Some(cap) = face_type_cap(context.to, field)
                        && value > cap
                    {
                        // An unencodable face type resets to 0; it is not
                        // clamped to the target's cap.
                        set_appearance(&mut appearance, key, Some(0));
                        notes.push(ImportNote::Capped {
                            path: leaf,
                            from: value,
                            to: 0,
                        });
                    }
                }
            }
        }
        if appearance.skin_color == Some(7) && !fpc::custom_skin_available(context.to) {
            set_appearance(&mut appearance, SettingKey::SkinColor, Some(1));
            notes.push(ImportNote::Capped {
                path: format!("{path}.appearance.skin_color"),
                from: 7,
                to: 1,
            });
        }
    }
    // The same width check on the post-adjustment values: a value the
    // target's field cannot hold is refused before any write.
    for key in SettingKey::ALL {
        let Some(value) = get_appearance(&appearance, key) else {
            continue;
        };
        let width = match key.source() {
            Source::Player(field) => bit_width(context.to, field),
            Source::Face(field) => INGAME_FACE_FIELDS
                .iter()
                .find(|spec| spec.field == field)
                .map(|spec| spec.bit_width),
            // No field of its own: `set_nose` checks the coded value.
            Source::NoseAngle => None,
        };
        let Some(width) = width else {
            continue;
        };
        check_width(
            &leaf_path(&format!("{path}.appearance"), &key.spec()),
            u32::from(value),
            width,
        )?;
    }
    // Absent = untouched: no `write_unused`, unlike a settings apply.
    apply_appearance(&appearance, player)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests

/// How many of `section`'s keys are `Some` — the document's density, for the
/// tests' "densest team" text-pass sample.
#[cfg(test)]
pub(super) fn some_keys(section: &PlayerSection) -> usize {
    let scalars = PlayerKey::ALL
        .iter()
        .filter(|key| key.get_section(section).is_some())
        .count();
    let named = [
        section.name.is_some(),
        section.shirt_name.is_some(),
        section.number.is_some(),
        section.positions.playable.is_some(),
        section.positions.playing_style.is_some(),
        section.skills.skills.is_some(),
        section.skills.com_styles.is_some(),
    ];
    let appearance = SettingKey::ALL
        .iter()
        .filter(|key| get_appearance(&section.appearance, **key).is_some())
        .count();
    scalars + named.iter().filter(|set| **set).count() + appearance
}

#[cfg(test)]
mod tests;
