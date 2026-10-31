//! `team.toml`'s team half and the `TeamToml` entry points (the tactics half is
//! `tactics.rs`, the shared item helpers `items.rs`): `from_team` dumps a `TeamEntry`,
//! `to_toml` emits the TOML block of `spec/pes_savefile/operations.md` "Team TOML"
//! (one key per line, comments at column 33,
//! an unset key as its commented neutral line, a table only when its section
//! carries data), `parse` accepts partial documents (absent = untouched on
//! import) and `apply` writes the present keys back, gated by the target
//! schema's field tables rather than by matching on `PesVersion`.

use pes_version::PesVersion;
use toml_edit::{DocumentMut, Item, Value};

use crate::codec::text_max;
use crate::interchange::team_toml::{
    AutoSection, ImportNote, InstructionEntry, InstructionsSection, PresetSection,
    SetPiecesSection, SlidersSection, StyleSection, TacticsSection, TeamEditFlagsSection,
    TeamSection, TeamToml, TeamTomlError,
};
use crate::model::instruction::Instruction;
use crate::model::names::display_name;
use crate::model::player::PlayerEntry;
use crate::model::tactics::AdvancedInstruction;
use crate::model::team::{KitSlot, TeamColor, TeamEntry};
use crate::schema::fields::{PlayerText, TeamField, TeamText};
use crate::schema::instruction;
use crate::schema::schema_for;
use crate::settings_toml::document::padded;

use super::items::{
    as_table, boolean, check_text, check_width, color, emit, gated, integer, kit_slots, opt,
    reject, text, u16_val, u32_val,
};
use super::player;
use super::tactics::{apply_tactics, emit_tactics, parse_tactics};

impl TeamToml {
    /// Reads a `team.toml` text. Every key is optional; anything the format
    /// does not know is `UnknownKey`.
    pub fn parse(text: &str) -> Result<Self, TeamTomlError> {
        let document: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| TeamTomlError::Toml(e.to_string()))?;
        let mut parsed = TeamToml::default();
        for (name, item) in document.iter() {
            match name {
                "pes_version" => parsed.pes_version = Some(version(item)?),
                "team" => parsed.team = parse_team(item)?,
                "tactics" => parsed.tactics = parse_tactics(item)?,
                "players" => parsed.players = player::parse_players(item)?,
                _ => {
                    return Err(TeamTomlError::UnknownKey {
                        key: name.to_string(),
                    });
                }
            }
        }
        Ok(parsed)
    }

    /// The document for one team of `version`'s save. Every field the model
    /// carries is `Some`; version-gated model `Option`s stay `None` and emit
    /// commented lines. `players` is the pool to dump from: an empty slice is
    /// a team/tactics-only document, while a non-empty pool that lacks a
    /// rostered id is `PlayerMissing`.
    pub fn from_team(
        version: PesVersion,
        team: &TeamEntry,
        players: &[&PlayerEntry],
    ) -> Result<Self, TeamTomlError> {
        let player_sections = player::players_from(version, team, players)?;
        let edit = &team.edit_flags;
        let tactics = &team.tactics;
        let mut presets = [PresetSection::default(); 3];
        for (section, preset) in presets.iter_mut().zip(tactics.presets.iter()) {
            let style = &preset.style;
            let sliders = &preset.sliders;
            section.man_marking = Some(preset.man_marking);
            section.style = StyleSection {
                attacking_style: Some(style.attacking_style),
                attacking_zone: Some(style.attacking_zone),
                buildup: Some(style.buildup),
                positioning: Some(style.positioning),
                defensive_style: Some(style.defensive_style),
                containment_area: Some(style.containment_area),
                pressure: Some(style.pressure),
                fluid: style.fluid,
            };
            section.sliders = SlidersSection {
                support_range: Some(sliders.support_range),
                defensive_line: Some(sliders.defensive_line),
                compactness: Some(sliders.compactness),
                numbers_in_attack: Some(sliders.numbers_in_attack),
                numbers_in_defence: Some(sliders.numbers_in_defence),
            };
            section.formations = [
                Some(preset.formations[0].players),
                Some(preset.formations[1].players),
                Some(preset.formations[2].players),
            ];
            section.instructions = match (&preset.attack_instructions, &preset.defence_instructions)
            {
                (Some(attack), Some(defence)) => Some(InstructionsSection {
                    attack: entries(version, attack)?,
                    defence: entries(version, defence)?,
                }),
                // The codec sets both sides together; a lone side is
                // unreachable from a decoded save.
                (None, None) | (Some(_), None) | (None, Some(_)) => None,
            };
        }
        Ok(TeamToml {
            pes_version: Some(version),
            roster_breaks: team.roster_order_breaks(),
            team: TeamSection {
                id: Some(team.id),
                name: Some(team.name.clone()),
                short_name: Some(team.short_name.clone()),
                manager_id: team.manager_id,
                stadium_id: team.stadium_id,
                color_1: team.colors.map(|c| c[0]),
                color_2: team.colors.map(|c| c[1]),
                kit_slots: team.kit_slots,
                edit_flags: TeamEditFlagsSection {
                    name: Some(edit.name),
                    short_name: edit.short_name,
                    stadium: edit.stadium,
                    strip: edit.strip,
                },
            },
            tactics: TacticsSection {
                starting_eleven: Some(tactics.starting_eleven),
                bench_order: Some(tactics.bench_order),
                set_pieces: SetPiecesSection {
                    free_kick_long: Some(tactics.set_pieces.free_kick_long),
                    free_kick_short: Some(tactics.set_pieces.free_kick_short),
                    free_kick_second: Some(tactics.set_pieces.free_kick_second),
                    corner_left: Some(tactics.set_pieces.corner_left),
                    corner_right: Some(tactics.set_pieces.corner_right),
                    penalty: Some(tactics.set_pieces.penalty),
                    captain: Some(tactics.set_pieces.captain),
                },
                players_to_join_attack: Some(tactics.players_to_join_attack),
                auto: AutoSection {
                    substitution: tactics.auto.substitution,
                    offside_trap: tactics.auto.offside_trap,
                    preset_change: tactics.auto.preset_change,
                    attack_defence_levels: tactics.auto.attack_defence_levels,
                },
                presets,
            },
            players: player_sections,
        })
    }

    /// The document's text, in the TOML block's order. A `None` key emits its
    /// commented neutral line. `[team]`, `[tactics]` and its `set_pieces`,
    /// `auto` and preset `style`/`sliders` tables, and every `[players.NN]`
    /// block are emitted whole, every key shown or commented; the tables
    /// emitted only when they hold a `Some` member are `[team.edit_flags]`,
    /// a preset's `formation_M` and `instructions`, and `[players.NN]` itself
    /// exists per entry in `players`.
    pub fn to_toml(&self) -> Result<String, TeamTomlError> {
        let mut out = String::new();
        let version = self
            .pes_version
            .map(|v| u16::from(v).to_string())
            .unwrap_or_else(|| "19".to_string());
        let body = format!("pes_version = {version}");
        let line = padded(
            &body,
            "the save this was written from; import into another version converts",
        );
        match self.pes_version {
            Some(_) => out.push_str(&line),
            None => out.push_str(&format!("# {line}")),
        }
        out.push('\n');
        emit_team(&mut out, &self.team);
        emit_tactics(&mut out, &self.tactics)?;
        player::emit_players(&mut out, &self.players, self.team.id, &self.roster_breaks)?;
        Ok(out)
    }

    /// Writes the document's present keys into `team`, all-or-nothing: a
    /// cloned team is patched and swapped in only when every key applied (or
    /// was noted and skipped). `team.id` and the roster are never written —
    /// `id` is informational and Team TOML does not carry roster data.
    ///
    /// A key the target's schema lacks is a `NotInThisVersion` note when the
    /// document's `pes_version` differs from `to`, and a
    /// `TeamTomlError::NotInThisVersion` when it matches or is absent. The
    /// instructions table is dropped silently on versions with no instruction
    /// block; an instruction the target cannot store becomes `Off` with a
    /// `NotEncodable` note.
    pub fn apply(
        &self,
        to: PesVersion,
        team: &mut TeamEntry,
        players: &mut [PlayerEntry],
    ) -> Result<Vec<ImportNote>, TeamTomlError> {
        // Only the players the roster names can be written, so only they are
        // staged: cloning the whole save's players would copy thousands to
        // patch at most forty. They keep their save order, so a lookup by id
        // finds the same player it would in `players`.
        let rostered: Vec<usize> = players
            .iter()
            .enumerate()
            .filter(|(_, player)| {
                player.id != 0 && team.roster.iter().any(|slot| slot.player_id == player.id)
            })
            .map(|(index, _)| index)
            .collect();
        let mut next_players: Vec<PlayerEntry> = rostered
            .iter()
            .map(|&index| players[index].clone())
            .collect();
        let from = self.pes_version.unwrap_or(to);
        let foreign = from != to;
        let schema = schema_for(to);
        let mut notes = Vec::new();
        let mut next = team.clone();
        apply_team(&self.team, schema.team, foreign, &mut next, &mut notes)?;
        apply_tactics(
            &self.tactics,
            schema.tactic,
            to,
            foreign,
            &mut next,
            &mut notes,
        )?;
        player::apply_players(
            &self.players,
            self.team.id,
            from,
            to,
            &mut next,
            &mut next_players,
            &mut notes,
        )?;
        *team = next;
        for (index, player) in rostered.into_iter().zip(next_players) {
            players[index] = player;
        }
        Ok(notes)
    }
}

/// The shirt name a display name produces: colour codes stripped, uppercased,
/// cut to the version's shirt-name field minus one byte. A generated name keeps
/// one byte free for a NUL because it is new text, and no in-game check has
/// shown a full-field shirt name displaying whole.
pub fn shirt_name_from(name: &str, version: PesVersion) -> String {
    let len = schema_for(version)
        .player
        .texts
        .iter()
        .find(|spec| spec.text == PlayerText::ShirtName)
        .map(|spec| spec.len)
        .expect("every player schema stores a shirt name");
    let take = usize::try_from(len - 1).expect("the field length minus its terminator fits usize");
    display_name(name)
        .to_uppercase()
        .chars()
        .take(take)
        .collect()
}

/// One instruction side's two entries, decoded through the version's table.
fn entries(
    version: PesVersion,
    stored: &[AdvancedInstruction; 2],
) -> Result<[InstructionEntry; 2], TeamTomlError> {
    let mut out = [InstructionEntry::default(); 2];
    for (entry, stored) in out.iter_mut().zip(stored.iter()) {
        *entry = InstructionEntry {
            instruction: match instruction::decode(version, stored.instruction)? {
                Some(instruction) => instruction,
                None => Instruction::Off,
            },
            player: stored.player,
        };
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Emit

fn emit_team(out: &mut String, team: &TeamSection) {
    out.push_str("\n[team]\n");
    emit(
        out,
        "id",
        team.id.map(|v| v.to_string()),
        "0",
        "informational: the caller chooses the target team",
    );
    emit(
        out,
        "name",
        team.name
            .as_ref()
            .map(|n| Value::from(n.as_str()).to_string()),
        "\"\"",
        "",
    );
    emit(
        out,
        "short_name",
        team.short_name
            .as_ref()
            .map(|n| Value::from(n.as_str()).to_string()),
        "\"\"",
        "",
    );
    emit(
        out,
        "manager_id",
        team.manager_id.map(|v| v.to_string()),
        "0",
        "PES 19+",
    );
    emit(
        out,
        "stadium_id",
        team.stadium_id.map(|v| v.to_string()),
        "0",
        "PES 19+",
    );
    emit(
        out,
        "color_1",
        team.color_1.map(color_text),
        "[0, 0, 0]",
        "PES 17+; stored channels, 0-63 each",
    );
    emit(
        out,
        "color_2",
        team.color_2.map(color_text),
        "[0, 0, 0]",
        "PES 17+; stored channels, 0-63 each",
    );
    emit(
        out,
        "kit_slots",
        team.kit_slots.map(kit_slots_text),
        "[{ number = 0, binding = 0 }]",
        "PES 17",
    );
    let flags = &team.edit_flags;
    if flags.name.is_none()
        && flags.short_name.is_none()
        && flags.stadium.is_none()
        && flags.strip.is_none()
    {
        return;
    }
    out.push_str("\n[team.edit_flags]\n");
    emit(out, "name", flags.name.map(|v| v.to_string()), "false", "");
    emit(
        out,
        "short_name",
        flags.short_name.map(|v| v.to_string()),
        "false",
        "PES 15 only",
    );
    emit(
        out,
        "stadium",
        flags.stadium.map(|v| v.to_string()),
        "false",
        "PES 20+",
    );
    emit(
        out,
        "strip",
        flags.strip.map(|v| v.to_string()),
        "false",
        "PES 17 only",
    );
}

fn color_text(c: TeamColor) -> String {
    format!("[{}, {}, {}]", c.red, c.green, c.blue)
}

fn kit_slots_text(slots: [KitSlot; 10]) -> String {
    let entries: Vec<String> = slots
        .iter()
        .map(|s| format!("{{ number = {}, binding = {} }}", s.number, s.binding))
        .collect();
    format!("[{}]", entries.join(", "))
}

// ---------------------------------------------------------------------------
// Parse

fn version(item: &Item) -> Result<PesVersion, TeamTomlError> {
    let value = integer(item, "pes_version")?;
    let number = u16::try_from(value).map_err(|_| TeamTomlError::OutOfRange {
        key: "pes_version".to_string(),
        value,
        range: "15 to 21".to_string(),
    })?;
    PesVersion::from_number(number).ok_or_else(|| TeamTomlError::OutOfRange {
        key: "pes_version".to_string(),
        value,
        range: "15 to 21".to_string(),
    })
}

fn parse_team(item: &Item) -> Result<TeamSection, TeamTomlError> {
    let table = as_table(item, "team")?;
    let mut team = TeamSection {
        id: opt(table, "team", "id", u32_val)?,
        name: opt(table, "team", "name", text)?,
        short_name: opt(table, "team", "short_name", text)?,
        manager_id: opt(table, "team", "manager_id", u32_val)?,
        stadium_id: opt(table, "team", "stadium_id", u16_val)?,
        color_1: opt(table, "team", "color_1", color)?,
        color_2: opt(table, "team", "color_2", color)?,
        kit_slots: opt(table, "team", "kit_slots", kit_slots)?,
        edit_flags: TeamEditFlagsSection::default(),
    };
    if let Some(item) = table.get("edit_flags") {
        let flags = as_table(item, "team.edit_flags")?;
        team.edit_flags = TeamEditFlagsSection {
            name: opt(flags, "team.edit_flags", "name", boolean)?,
            short_name: opt(flags, "team.edit_flags", "short_name", boolean)?,
            stadium: opt(flags, "team.edit_flags", "stadium", boolean)?,
            strip: opt(flags, "team.edit_flags", "strip", boolean)?,
        };
        reject(
            flags,
            "team.edit_flags",
            &["name", "short_name", "stadium", "strip"],
        )?;
    }
    reject(
        table,
        "team",
        &[
            "id",
            "name",
            "short_name",
            "manager_id",
            "stadium_id",
            "color_1",
            "color_2",
            "kit_slots",
            "edit_flags",
        ],
    )?;
    Ok(team)
}

fn apply_team(
    section: &TeamSection,
    schema: &crate::schema::RecordSchema<TeamField, crate::schema::fields::TeamText>,
    foreign: bool,
    next: &mut TeamEntry,
    notes: &mut Vec<ImportNote>,
) -> Result<(), TeamTomlError> {
    if let Some(name) = &section.name {
        check_text(
            "team.name",
            name,
            text_max(schema.texts, TeamText::Name),
            false,
        )?;
        next.name = name.clone();
    }
    if let Some(short_name) = &section.short_name {
        check_text(
            "team.short_name",
            short_name,
            text_max(schema.texts, TeamText::ShortName),
            true,
        )?;
        next.short_name = short_name.clone();
    }
    if let Some(manager_id) = section.manager_id
        && gated(
            notes,
            foreign,
            schema.has(TeamField::ManagerId),
            "team.manager_id",
        )?
    {
        next.manager_id = Some(manager_id);
    }
    if let Some(stadium_id) = section.stadium_id
        && gated(
            notes,
            foreign,
            schema.has(TeamField::StadiumId),
            "team.stadium_id",
        )?
    {
        next.stadium_id = Some(stadium_id);
    }
    for (index, color, field) in [
        (0usize, section.color_1, TeamField::Color1Red),
        (1, section.color_2, TeamField::Color2Red),
    ] {
        let Some(color) = color else { continue };
        if gated(
            notes,
            foreign,
            schema.has(field),
            &format!("team.color_{}", index + 1),
        )? {
            next.colors.get_or_insert_default()[index] = color;
        }
    }
    if let Some(kit_slots) = section.kit_slots
        && gated(
            notes,
            foreign,
            schema.has(TeamField::KitSlotNumber(0)),
            "team.kit_slots",
        )?
    {
        // `binding` is a u32 in the document but the field is narrower (24
        // bits on PES 17); refuse before `write_team` would have to.
        let width = schema
            .bit_width(TeamField::KitSlotTeam(0))
            .expect("a record with kit slot numbers stores their team bindings");
        for (i, slot) in kit_slots.iter().enumerate() {
            check_width(&format!("team.kit_slots.{i}.binding"), slot.binding, width)?;
        }
        next.kit_slots = Some(kit_slots);
    }
    let flags = &section.edit_flags;
    if let Some(name) = flags.name
        && gated(
            notes,
            foreign,
            schema.has(TeamField::EditedName),
            "team.edit_flags.name",
        )?
    {
        next.edit_flags.name = name;
    }
    if let Some(short_name) = flags.short_name
        && gated(
            notes,
            foreign,
            schema.has(TeamField::EditedShortName),
            "team.edit_flags.short_name",
        )?
    {
        next.edit_flags.short_name = Some(short_name);
    }
    if let Some(stadium) = flags.stadium
        && gated(
            notes,
            foreign,
            schema.has(TeamField::EditedStadium),
            "team.edit_flags.stadium",
        )?
    {
        next.edit_flags.stadium = Some(stadium);
    }
    if let Some(strip) = flags.strip
        && gated(
            notes,
            foreign,
            schema.has(TeamField::EditedStrip),
            "team.edit_flags.strip",
        )?
    {
        next.edit_flags.strip = Some(strip);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::interchange::team_toml::{ImportNote, TeamToml, TeamTomlError};
    use crate::test_support::{FIXTURES, open, test_salt};

    /// `team` with every field defaulted except `id` and `roster`, which Team
    /// TOML never carries.
    fn blank_slate(team: &TeamEntry) -> TeamEntry {
        TeamEntry {
            id: team.id,
            roster: team.roster.clone(),
            ..TeamEntry::default()
        }
    }

    /// (a) Every fixture team: document -> text -> parse is identity, and
    /// applying onto a blank team reproduces the original field for field.
    #[test]
    fn every_fixture_team_round_trips_through_the_document() {
        for version in FIXTURES {
            let (file, _) = open(version);
            let refs: Vec<&PlayerEntry> = file.players().iter().collect();
            // Same split as the player test: the model pass (from_team ->
            // apply -> field-for-field) stays exhaustive; the text pass
            // (to_toml -> parse == document, ~60 ms a team in toml_edit) runs
            // on a sample: the first team, the last, and the team whose
            // players carry the most `Some` keys.
            let docs: Vec<TeamToml> = file
                .teams()
                .iter()
                .map(|team| TeamToml::from_team(version, team, &refs).expect("from_team"))
                .collect();
            let fullest = docs
                .iter()
                .enumerate()
                .max_by_key(|(_, doc)| doc.players.values().map(player::some_keys).sum::<usize>())
                .map(|(n, _)| n)
                .expect("a team exists");
            for (n, (team, doc)) in file.teams().iter().zip(&docs).enumerate() {
                if n == 0 || n + 1 == docs.len() || n == fullest {
                    let text = doc.to_toml().expect("to_toml");
                    let parsed = TeamToml::parse(&text).unwrap_or_else(|e| {
                        panic!(
                            "{version:?} team {} reparse: {e}
{text}",
                            team.id
                        )
                    });
                    // Identity holds for a team keeping the roster order
                    // only: `roster_breaks` is written as a header comment,
                    // never a key, so it does not parse back (every fixture
                    // team keeps the order).
                    assert_eq!(parsed, *doc, "{version:?} team {}", team.id);
                }
                let mut target = blank_slate(team);
                // `apply` only looks the rostered ids up; cloning the whole
                // save's players per team was the model pass's cost.
                let mut players: Vec<PlayerEntry> = file
                    .players()
                    .iter()
                    .filter(|player| team.roster.iter().any(|slot| slot.player_id == player.id))
                    .cloned()
                    .collect();
                let notes = doc
                    .apply(version, &mut target, &mut players)
                    .expect("same-version apply");
                assert!(notes.is_empty(), "{version:?} team {}: {notes:?}", team.id);
                assert_eq!(&target, team, "{version:?} team {}", team.id);
            }
        }
    }

    /// (b) A one-key document changes exactly that key.
    #[test]
    fn a_one_key_document_changes_exactly_that_key() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let doc = TeamToml::parse("[tactics.set_pieces]\npenalty = 3\n").expect("parses");
        let mut target = team.clone();
        let notes = doc
            .apply(PesVersion::Pes19, &mut target, &mut [])
            .expect("applies");
        assert!(notes.is_empty());
        let mut expected = team.clone();
        expected.tactics.set_pieces.penalty = 3;
        assert_eq!(target, expected);
    }

    /// A document carries each preset's man-marking assignments: a PES 17
    /// export of team 737 (whose preset 1 and 2 mark slot 6's man onto 10).
    #[test]
    fn an_export_carries_man_marking() {
        let (file, _) = open(PesVersion::Pes17);
        let team = file.team(737).expect("team 737");
        let doc = TeamToml::from_team(PesVersion::Pes17, team, &[]).expect("from_team");
        let expected = [255, 255, 255, 255, 255, 255, 10, 255, 255, 255, 255];
        assert_eq!(doc.tactics.presets[0].man_marking, Some(expected));
        assert_eq!(doc.tactics.presets[1].man_marking, Some(expected));
        assert_eq!(doc.tactics.presets[2].man_marking, Some([255; 11]));
        let text = doc.to_toml().expect("to_toml");
        assert!(
            text.contains(
                "[tactics.preset_1]\nman_marking = [255, 255, 255, 255, 255, 255, 10, 255, 255, 255, 255]"
            ),
            "{text}"
        );
    }

    /// `man_marking` takes eleven integers, each 0 to 10 or 255: anything
    /// else is refused at its key path.
    #[test]
    fn a_man_marking_value_past_the_formation_slots_is_refused() {
        let err = TeamToml::parse(
            "[tactics.preset_1]\nman_marking = [11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]\n",
        )
        .expect_err("11 is no formation slot");
        assert!(
            matches!(
                &err,
                TeamTomlError::OutOfRange { key, .. }
                    if key == "tactics.preset_1.man_marking"
            ),
            "{err:?}"
        );
        // 10, the last formation slot, is a marking.
        let doc = TeamToml::parse(
            "[tactics.preset_1]\nman_marking = [10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]\n",
        )
        .expect("10 is a formation slot");
        assert_eq!(doc.tactics.presets[0].man_marking.map(|m| m[0]), Some(10));
    }

    /// An absent `man_marking` key leaves the team's bytes as they are, like
    /// every other optional key.
    #[test]
    fn an_absent_man_marking_key_leaves_the_teams_bytes() {
        let (file, _) = open(PesVersion::Pes17);
        let team = file.team(737).expect("team 737");
        let doc =
            TeamToml::parse("[tactics.preset_1.sliders]\nsupport_range = 5\n").expect("parses");
        let mut target = team.clone();
        let notes = doc
            .apply(PesVersion::Pes17, &mut target, &mut [])
            .expect("applies");
        assert!(notes.is_empty(), "{notes:?}");
        assert_eq!(target.tactics.presets[0].sliders.support_range, 5);
        assert_eq!(
            target.tactics.presets[0].man_marking,
            team.tactics.presets[0].man_marking
        );
    }

    /// `tactics.auto.substitution` is the stored setting byte, not a slot
    /// number: a non-zero value parses to exactly that value.
    #[test]
    fn a_non_zero_substitution_parses() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let value = if team.tactics.auto.substitution == Some(7) {
            8
        } else {
            7
        };
        let text = TeamToml::from_team(PesVersion::Pes19, team, &[])
            .expect("from_team")
            .to_toml()
            .expect("to_toml");
        // The emitted `substitution` line (commented or not) becomes a live
        // `substitution = {value}`.
        let at = text.find("substitution").expect("the emitted line");
        let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        let end = text[at..].find('\n').map_or(text.len(), |i| at + i + 1);
        let text = format!("{}substitution = {value}\n{}", &text[..start], &text[end..]);
        let parsed = TeamToml::parse(&text).expect("parses");
        assert_eq!(parsed.tactics.auto.substitution, Some(value));
    }

    /// (c) A foreign document's unsupported key is a note; the target's own
    /// version (or no version) makes it an error. Either way nothing applied.
    #[test]
    fn a_version_gated_team_field_is_a_note_across_versions_and_an_error_within() {
        let (file, _) = open(PesVersion::Pes18);
        let team = file.teams().first().expect("a team");

        let foreign =
            TeamToml::parse("pes_version = 21\n\n[team]\nmanager_id = 5\n").expect("parses");
        let mut target = team.clone();
        let notes = foreign
            .apply(PesVersion::Pes18, &mut target, &mut [])
            .expect("note, not error");
        assert_eq!(
            notes,
            vec![ImportNote::NotInThisVersion {
                path: "team.manager_id".to_string()
            }]
        );
        assert_eq!(&target, team);

        for text in [
            "[team]\nmanager_id = 5\n",
            "pes_version = 18\n\n[team]\nmanager_id = 5\n",
        ] {
            let doc = TeamToml::parse(text).expect("parses");
            let mut target = team.clone();
            let err = doc
                .apply(PesVersion::Pes18, &mut target, &mut [])
                .expect_err("hard error");
            assert!(
                matches!(err, TeamTomlError::NotInThisVersion { .. }),
                "{err:?}"
            );
            assert_eq!(&target, team);
        }
    }

    /// (d) An instruction the target cannot store degrades to Off with a note;
    /// a version with no instruction table drops the table silently.
    #[test]
    fn an_unencodable_instruction_is_a_note_and_no_table_is_silent() {
        let text = "pes_version = 19\n\n[tactics.preset_1.instructions]\n\
            attack = [{ instruction = \"wing_back\", player = 2 }, { instruction = \"off\", player = 0 }]\n\
            defence = [{ instruction = \"off\", player = 0 }, { instruction = \"off\", player = 0 }]\n";
        let doc = TeamToml::parse(text).expect("parses");

        let (file17, _) = open(PesVersion::Pes17);
        let team17 = file17.teams().first().expect("a team");
        let mut target = team17.clone();
        let notes = doc
            .apply(PesVersion::Pes17, &mut target, &mut [])
            .expect("applies");
        assert_eq!(
            notes,
            vec![ImportNote::NotEncodable {
                path: "tactics.preset_1.instructions.attack".to_string(),
                label: "wing_back".to_string()
            }]
        );
        let stored = target.tactics.presets[0]
            .attack_instructions
            .expect("PES 17 has instructions");
        assert_eq!(stored[0].instruction, 0, "wing_back fell back to Off");
        assert_eq!(stored[0].player, 2);

        let (file16, _) = open(PesVersion::Pes16);
        let team16 = file16.teams().first().expect("a team");
        let mut target = team16.clone();
        let notes = doc
            .apply(PesVersion::Pes16, &mut target, &mut [])
            .expect("applies");
        assert!(notes.is_empty(), "{notes:?}");
        assert!(target.tactics.presets[0].attack_instructions.is_none());
    }

    /// (e) Parse errors carry the dotted path, the expected shape and the
    /// allowed range/labels.
    #[test]
    fn parse_errors_are_typed() {
        let err = TeamToml::parse("[team]\nbogus = 1\n").expect_err("unknown key");
        assert!(
            matches!(&err, TeamTomlError::UnknownKey { key } if key == "team.bogus"),
            "{err:?}"
        );
        let err = TeamToml::parse("[tactics]\nbench_order = \"x\"\n").expect_err("wrong type");
        assert!(matches!(err, TeamTomlError::WrongType { .. }), "{err:?}");
        let err = TeamToml::parse("[tactics.preset_1.sliders]\nsupport_range = 11\n")
            .expect_err("out of range");
        assert!(
            matches!(
                &err,
                TeamTomlError::OutOfRange { key, value: 11, range }
                    if key == "tactics.preset_1.sliders.support_range" && range == "1 to 10"
            ),
            "{err:?}"
        );
        let err = TeamToml::parse("[tactics.preset_1.style]\nattacking_style = \"posession\"\n")
            .expect_err("unknown label");
        assert!(
            matches!(
                &err,
                TeamTomlError::UnknownLabel { key, label, allowed }
                    if key == "tactics.preset_1.style.attacking_style"
                        && label == "posession"
                        && allowed == "\"counter_attack\", \"possession\""
            ),
            "{err:?}"
        );
        let ten = (0..10)
            .map(|_| "{ position = \"GK\", x = 0, y = 0 }")
            .collect::<Vec<_>>()
            .join(", ");
        let err = TeamToml::parse(&format!(
            "[tactics.preset_1.formation_1]\nplayers = [{ten}]\n"
        ))
        .expect_err("ten entries");
        assert!(matches!(err, TeamTomlError::WrongType { .. }), "{err:?}");
    }

    /// A present `[instructions]` or `[formation_N]` table requires its keys;
    /// a missing one is `MissingKey`, not a defaulted section that `apply`
    /// would then write over the target's real instructions.
    #[test]
    fn a_present_table_missing_a_required_key_is_an_error() {
        let err = TeamToml::parse(
            "[tactics.preset_1.instructions]\n\
             defence = [{ instruction = \"off\", player = 0 }, { instruction = \"off\", player = 0 }]\n",
        )
        .expect_err("attack is required once the table exists");
        assert!(
            matches!(&err, TeamTomlError::MissingKey { key }
                if key == "tactics.preset_1.instructions.attack"),
            "{err:?}"
        );
        let err = TeamToml::parse("[tactics.preset_1.formation_1]\n")
            .expect_err("players is required once the table exists");
        assert!(
            matches!(&err, TeamTomlError::MissingKey { key }
                if key == "tactics.preset_1.formation_1.players"),
            "{err:?}"
        );
    }

    /// Every version's player schema stores a shirt-name text field, so
    /// `shirt_name_from`'s lookup `expect` is provably unreachable.
    #[test]
    fn every_version_stores_a_shirt_name() {
        for version in PesVersion::ALL {
            assert!(
                schema_for(version)
                    .player
                    .texts
                    .iter()
                    .any(|spec| spec.text == PlayerText::ShirtName),
                "{version:?}"
            );
        }
        assert_eq!(shirt_name_from("lord fulp", PesVersion::Pes19), "LORD FULP");
    }

    /// A formation emits as a multi-line array, one entry per line.
    #[test]
    fn formations_emit_one_entry_per_line() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let text = TeamToml::from_team(PesVersion::Pes19, team, &[])
            .expect("from_team")
            .to_toml()
            .expect("to_toml");
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| *l == "players = [")
            .expect("a formation's players array");
        for (i, line) in lines[at + 1..at + 12].iter().enumerate() {
            assert!(
                line.starts_with("    { position = \"") && line.ends_with(" },"),
                "line {i}: {line:?}"
            );
        }
        assert_eq!(lines[at + 12], "]");
        // The multi-line form still parses back to the same document.
        let doc = TeamToml::from_team(PesVersion::Pes19, team, &[]).expect("from_team");
        assert_eq!(TeamToml::parse(&text).expect("reparse"), doc);
    }

    /// (f) The emitted text opens with the spec's TOML block's lines.
    #[test]
    fn the_emitted_text_follows_the_plan_block() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let text = TeamToml::from_team(PesVersion::Pes19, team, &[])
            .expect("from_team")
            .to_toml()
            .expect("to_toml");
        let lines: Vec<&str> = text.lines().collect();
        let version_body = "pes_version = 19";
        assert_eq!(
            lines[0],
            format!(
                "{version_body}{}# the save this was written from; import into another version converts",
                " ".repeat(32 - version_body.len())
            )
        );
        assert_eq!(lines[1], "");
        assert_eq!(lines[2], "[team]");
        assert!(lines[3].starts_with("id = "), "{:?}", lines[3]);
        let manager = lines
            .iter()
            .find(|l| l.starts_with("manager_id"))
            .expect("a manager_id line");
        let body = format!("manager_id = {}", team.manager_id.expect("PES 19 has it"));
        let expected = format!("{body}{}# PES 19+", " ".repeat(32 - body.len()));
        assert_eq!(*manager, expected, "the comment sits at column 33");
    }

    /// A tactics key the target's schema lacks is a `NotInThisVersion` note
    /// across versions and an error within — `has` and `has_preset` both.
    #[test]
    fn a_version_gated_tactics_field_is_a_note_across_versions_and_an_error_within() {
        let (file, _) = open(PesVersion::Pes15);
        let team = file.teams().first().expect("a team");

        for text in [
            "pes_version = 19\n\n[tactics.auto]\nattack_defence_levels = true\n",
            "pes_version = 19\n\n[tactics.preset_1.style]\nfluid = true\n",
        ] {
            let doc = TeamToml::parse(text).expect("parses");
            let mut target = team.clone();
            let mut players = file.players().to_vec();
            let notes = doc
                .apply(PesVersion::Pes15, &mut target, &mut players)
                .expect("a foreign gated key is a note");
            let path = if text.contains("attack_defence_levels") {
                "tactics.auto.attack_defence_levels"
            } else {
                "tactics.preset_1.style.fluid"
            };
            assert_eq!(
                notes,
                vec![ImportNote::NotInThisVersion {
                    path: path.to_string()
                }]
            );
            assert_eq!(&target, team, "{text:?}");
        }

        for text in [
            "[tactics.auto]\nattack_defence_levels = true\n",
            "pes_version = 15\n\n[tactics.preset_1.style]\nfluid = true\n",
        ] {
            let doc = TeamToml::parse(text).expect("parses");
            let mut target = team.clone();
            let mut players = file.players().to_vec();
            let err = doc
                .apply(PesVersion::Pes15, &mut target, &mut players)
                .expect_err("a same-version gated key is an error");
            assert!(matches!(err, TeamTomlError::NotInThisVersion { .. }));
            assert_eq!(&target, team, "{text:?}");
        }
    }

    /// `shirt_name_from`: upper-case, colour codes stripped, cut to the
    /// field's `len - 1` by characters: 18 bytes on PES 15 and 17 to 19
    /// (17 chars), 16 on PES 16 (15), 61 on PES 20 and 21 (60).
    #[test]
    fn shirt_name_from_uppercases_strips_and_cuts_by_chars() {
        assert_eq!(shirt_name_from("Snuffy", PesVersion::Pes19), "SNUFFY");
        // A `\x11c` colour code and its eight payload bytes are stripped.
        assert_eq!(
            shirt_name_from(
                "\x11c\x00\x00\x00\x00\x00\x00\x00\x00Snuffy",
                PesVersion::Pes19
            ),
            "SNUFFY"
        );
        // The field's length less its terminator, per version.
        let long = "a".repeat(80);
        assert_eq!(shirt_name_from(&long, PesVersion::Pes19), "A".repeat(17));
        assert_eq!(shirt_name_from(&long, PesVersion::Pes15), "A".repeat(17));
        assert_eq!(shirt_name_from(&long, PesVersion::Pes16), "A".repeat(15));
        assert_eq!(shirt_name_from(&long, PesVersion::Pes21), "A".repeat(60));
        // Accented chars upper-case and count once, not per UTF-8 byte.
        assert_eq!(shirt_name_from("aéü", PesVersion::Pes19), "AÉÜ");
        let accented = "é".repeat(30);
        assert_eq!(
            shirt_name_from(&accented, PesVersion::Pes19)
                .chars()
                .count(),
            17
        );
    }

    /// A team text of exactly the field's capacity applies; one byte over
    /// is `TextTooLong` and writes nothing.
    #[test]
    fn a_team_text_at_the_fields_capacity_applies_and_one_more_is_refused() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let texts = schema_for(PesVersion::Pes19).team.texts;
        let name_max = text_max(texts, TeamText::Name);
        let short_max = text_max(texts, TeamText::ShortName);
        let doc = |name: &str, short: &str| {
            TeamToml::parse(&format!(
                "[team]\nname = \"{name}\"\nshort_name = \"{short}\"\n"
            ))
            .expect("parses")
        };

        let mut target = team.clone();
        doc(&"N".repeat(name_max), &"S".repeat(short_max))
            .apply(PesVersion::Pes19, &mut target, &mut [])
            .expect("at capacity applies");
        assert_eq!(target.name, "N".repeat(name_max));
        assert_eq!(target.short_name, "S".repeat(short_max));

        for (name_over, short_over, key, max) in [
            (true, false, "team.name", name_max),
            (false, true, "team.short_name", short_max),
        ] {
            let name = "N".repeat(name_max + usize::from(name_over));
            let short = "S".repeat(short_max + usize::from(short_over));
            let mut target = team.clone();
            let err = doc(&name, &short)
                .apply(PesVersion::Pes19, &mut target, &mut [])
                .expect_err("one byte over");
            assert!(
                matches!(
                    err,
                    TeamTomlError::TextTooLong { ref path, max: m }
                        if path == key && m == max
                ),
                "{err:?}"
            );
            assert_eq!(&target, team, "nothing was written");
        }
    }

    /// An embedded NUL would reload as a truncated string at the codec's
    /// terminator, so it is refused at parse, like a NUL in settings.toml's name.
    #[test]
    fn a_nul_inside_a_text_is_refused_at_parse() {
        for (key, doc) in [
            ("team.name", "[team]\nname = \"A\\u0000B\"\n"),
            ("team.short_name", "[team]\nshort_name = \"A\\u0000B\"\n"),
            ("players.01.name", "[players.01]\nname = \"A\\u0000B\"\n"),
            (
                "players.01.shirt_name",
                "[players.01]\nshirt_name = \"A\\u0000B\"\n",
            ),
        ] {
            match TeamToml::parse(doc) {
                Err(TeamTomlError::WrongType {
                    key: found,
                    expected,
                }) => {
                    assert_eq!(found, key);
                    assert_eq!(expected, "a string without NUL");
                }
                other => panic!("{key}: expected WrongType, got {other:?}"),
            }
        }
    }

    /// An unknown member inside an inline record is `UnknownKey`, the same
    /// rule as top-level keys: a dropped member would not survive emission.
    #[test]
    fn an_unknown_member_of_an_inline_record_is_rejected() {
        let kit_slot = "{ number = 0, binding = 0 }";
        let formation_player = "{ position = \"CF\", x = 1, y = 1 }";
        let instruction = "{ instruction = \"off\", player = 0 }";
        for (key, doc) in [
            (
                "team.kit_slots.0.extra",
                format!(
                    "[team]\nkit_slots = [{{ number = 0, binding = 0, extra = 3 }}, {}]\n",
                    (0..9).map(|_| kit_slot).collect::<Vec<_>>().join(", ")
                ),
            ),
            (
                "tactics.preset_1.formation_1.players.0.extra",
                format!(
                    "[tactics.preset_1.formation_1]\nplayers = [{{ position = \"CF\", x = 1, y = 1, extra = 2 }}, {}]\n",
                    (0..10)
                        .map(|_| formation_player)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ),
            (
                "tactics.preset_1.instructions.attack.0.extra",
                format!(
                    "[tactics.preset_1.instructions]\nattack = [{{ instruction = \"off\", player = 0, extra = 1 }}, {instruction}]\ndefence = [{instruction}, {instruction}]\n"
                ),
            ),
        ] {
            match TeamToml::parse(&doc) {
                Err(TeamTomlError::UnknownKey { key: found }) => assert_eq!(found, key),
                other => panic!("{key}: expected UnknownKey, got {other:?}"),
            }
        }
    }

    /// The kit slot's bound team id is a 24-bit field: `binding` past
    /// 2^24 - 1 is refused at apply, not left for `write_team` to reject.
    #[test]
    fn a_kit_slot_binding_past_the_field_width_is_refused() {
        let (file, _) = open(PesVersion::Pes17);
        let team = file.teams().first().expect("a team");
        let slot = "{ number = 0, binding = 0 }";
        let doc = |binding: u32| {
            TeamToml::parse(&format!(
                "pes_version = 17\n\n[team]\nkit_slots = [{{ number = 0, binding = {binding} }}, {}]\n",
                (0..9).map(|_| slot).collect::<Vec<_>>().join(", ")
            ))
            .expect("parses")
        };

        let mut target = team.clone();
        doc(16777215)
            .apply(PesVersion::Pes17, &mut target, &mut [])
            .expect("2^24 - 1 fits");

        let mut target = team.clone();
        let err = doc(16777216)
            .apply(PesVersion::Pes17, &mut target, &mut [])
            .expect_err("2^24 does not fit 24 bits");
        assert!(
            matches!(
                err,
                TeamTomlError::OutOfRange {
                    ref key,
                    value: 16777216,
                    ..
                } if key.starts_with("team.kit_slots")
            ),
            "{err:?}"
        );
        assert_eq!(&target, team, "nothing was written");
    }

    /// `short_name` is the codec's single-byte field: a char above U+00FF is
    /// refused at apply with nothing written.
    #[test]
    fn an_unencodable_short_name_is_refused_at_apply() {
        let (file, _) = open(PesVersion::Pes19);
        let team = file.teams().first().expect("a team");
        let doc =
            TeamToml::parse("pes_version = 19\n\n[team]\nshort_name = \"Ā\"\n").expect("parses");
        let mut target = team.clone();
        let err = doc
            .apply(PesVersion::Pes19, &mut target, &mut [])
            .expect_err("Ā is above U+00FF");
        assert!(
            matches!(
                err,
                TeamTomlError::WrongType {
                    ref key,
                    ..
                } if key == "team.short_name"
            ),
            "{err:?}"
        );
        assert_eq!(&target, team, "nothing was written");
    }

    /// A team exported and its text imported into a fresh copy of the save
    /// writes the original payload back byte for byte.
    #[test]
    fn an_exported_team_imported_back_writes_byte_identical() {
        let (file, _) = open(PesVersion::Pes21);
        let refs: Vec<&PlayerEntry> = file.players().iter().collect();
        let team = file.teams().first().expect("a team");
        let text = TeamToml::from_team(PesVersion::Pes21, team, &refs)
            .expect("from_team")
            .to_toml()
            .expect("to_toml");
        let doc = TeamToml::parse(&text).expect("the emitted text reparses");
        let (mut copy, bytes) = open(PesVersion::Pes21);
        // `apply` takes the team and the players: the team's writes go back
        // through its entry.
        let mut target = copy.team(team.id).expect("the team").clone();
        doc.apply(PesVersion::Pes21, &mut target, copy.players_mut())
            .expect("applies");
        *copy.team_mut(team.id).expect("the team") = target;
        assert_eq!(
            copy.to_bytes(&test_salt()).expect("the save writes"),
            bytes,
            "the payload is the original's, byte for byte"
        );
    }

    /// Every team of the save exports naming its team, and the text of the first, /co/ (714) and the last parses as a Team
    /// TOML naming it. The text pass on all 220 took 49 s in a test build;
    /// `every_fixture_team_round_trips_through_the_document` samples it too.
    #[test]
    fn every_team_exports_and_parses_naming_it() {
        let (file, _) = open(PesVersion::Pes21);
        let refs: Vec<&PlayerEntry> = file.players().iter().collect();
        let teams = file.teams();
        for team in teams {
            let doc = TeamToml::from_team(PesVersion::Pes21, team, &refs).expect("from_team");
            assert_eq!(doc.team.id, Some(team.id), "names its team");
        }
        for id in [teams[0].id, 714, teams[teams.len() - 1].id] {
            let team = file.team(id).expect("the fixture holds the team");
            let doc = TeamToml::from_team(PesVersion::Pes21, team, &refs).expect("from_team");
            let parsed = TeamToml::parse(&doc.to_toml().expect("to_toml"))
                .expect("the emitted text reparses");
            assert_eq!(parsed.team.id, Some(id));
        }
    }

    /// An export or an import naming a team the save lacks rests on
    /// `EditFile::team` answering `None`: the lookup the CLI's refusal is
    /// built on.
    #[test]
    fn a_team_the_save_lacks_is_none() {
        let (file, _) = open(PesVersion::Pes21);
        assert!(
            file.team(999).is_none(),
            "the referee id is not in the save"
        );
    }
}
