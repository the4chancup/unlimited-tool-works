//! `aesthetics_patch.toml`: the file the Team compiler writes to its output
//! folder with its CPKs, holding what the CPK's tables cannot — on Fox,
//! every compiled player's motions and a `write_names` export's names — for the
//! savefile builder to apply (`spec/pes_savefile/operations.md` "Aesthetics patch
//! (new)"). This module is the Fox half's types, text form and `apply`; the
//! pre-Fox keys are not built yet.

use std::collections::BTreeMap;

use pes_version::{Engine, PesVersion};
use toml_edit::{DocumentMut, Item, TableLike, Value};

use crate::codec::{text_max, write_player};
use crate::file::EditFile;
use crate::interchange::players_key;
use crate::model::player::PlayerEntry;
use crate::schema::fields::PlayerText;
use crate::schema::{VersionSchema, schema_for};
use crate::settings_toml::document::{emit_motion, padded, parse_motion, range_text};
use crate::settings_toml::{
    AppearanceSettings, MotionSettings, SettingKey, SettingsError, apply_appearance, check_text,
    get_appearance,
};
#[cfg(test)]
mod tests;

/// The version of the Team compiler's boots and gloves ID allocation scheme. A patch from
/// another scheme is refused.
pub const ALLOCATION_SCHEME_VERSION: u32 = 1;

/// The first line of a generated `aesthetics_patch.toml`.
const HEADER: &str = "# aesthetics_patch.toml, written by the Team compiler to its output folder\n";

/// `aesthetics_patch.toml`: the compiler's resolved savefile writes for its compiled players.
#[derive(Debug, Clone, PartialEq)]
pub struct AestheticsPatch {
    /// The PES version the compile targeted; a patch applies only to a save
    /// of the same version.
    pub pes_version: PesVersion,
    /// The version of the suite that wrote the patch.
    pub tool_version: String,
    /// The `ALLOCATION_SCHEME_VERSION` the compile ran under; a mismatch
    /// refuses the patch.
    pub allocation_scheme_version: u32,
    /// The CPKs the run published; empty for a sideload tree.
    pub cpks: Vec<String>,
    /// When the compile ran.
    pub compiled: toml_edit::Datetime,
    /// The teams whose players the patch writes, in file order.
    pub teams: Vec<PatchTeam>,
}

/// One `[[teams]]` block: the players of one team the patch writes.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchTeam {
    /// The team id; `players.NN` is player id `id × 100 + NN`.
    pub id: u32,
    /// For the reader; the id is what is applied.
    pub name: String,
    /// By `NN` (`players.NN`, player id `id × 100 + NN`).
    pub players: BTreeMap<u8, PatchPlayer>,
}

/// One `[teams.players.NN]` table: what the patch writes of one player.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchPlayer {
    /// A `write_names` export's only; `None` = not written.
    pub name: Option<String>,
    /// A `write_names` export's only, like `name`; `None` = not written.
    pub shirt_name: Option<String>,
    /// `[appearance.motion]`, as in `settings.toml`; `None` keys are not written.
    pub motion: MotionSettings,
}

/// What parsing, writing or applying an `aesthetics_patch.toml` fails with.
/// The key-level problems reuse `SettingsError`'s vocabulary (`UnknownKey`,
/// `WrongType`, `OutOfRange`, `UnknownLabel`) so both interchange formats
/// report a bad key the same way.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    /// The text is not valid TOML.
    #[error("aesthetics_patch.toml is not valid TOML: {0}")]
    Toml(String),
    /// A key a present table requires is absent.
    #[error("{key}: required in this table")]
    MissingKey {
        /// The dotted key path (`tool_version`, `teams[0].id`).
        key: String,
    },
    /// `pes_version` names a pre-Fox version; the pre-Fox patch is not
    /// built yet.
    #[error("{version}: the pre-Fox aesthetics patch is not supported yet")]
    PreFox {
        /// The version the file names.
        version: PesVersion,
    },
    /// `pes_version` is not the save's: a patch applies to the version it
    /// was compiled for, there is no cross-version application.
    #[error("the patch is for {patch}, the save is {save}")]
    VersionMismatch {
        /// The version the patch was compiled for.
        patch: PesVersion,
        /// The save's version.
        save: PesVersion,
    },
    /// `allocation_scheme_version` is not the suite's.
    #[error("the patch was compiled under allocation scheme {patch}, this suite uses {suite}")]
    SchemeMismatch {
        /// The scheme the patch was compiled under.
        patch: u32,
        /// The suite's `ALLOCATION_SCHEME_VERSION`.
        suite: u32,
    },
    /// A key-level problem, in `settings.toml`'s vocabulary.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

/// What `apply` did: the teams written and the items the save lacked.
#[derive(Debug, Clone, PartialEq)]
pub struct PatchSummary {
    /// The ids of the patch's teams the save holds, in file order.
    pub applied_teams: Vec<u32>,
    /// The ids of the players written, in file order, a team's by slot: the
    /// patch's players the save holds. Empty means the save is unchanged.
    pub applied_players: Vec<u32>,
    /// The patch's teams and players the save lacks: teams in file order,
    /// a team's players by slot (the `players` map is a `BTreeMap`;
    /// `parse` keeps no key order).
    pub skipped: Vec<PatchSkip>,
}

/// A patch item the save has no target for; the rest of the patch applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchSkip {
    /// The save has no team with this id; its players were not looked at.
    TeamNotInSave {
        /// The team id.
        team: u32,
    },
    /// The save has no player with this id (`team id × 100 + NN`).
    PlayerNotInSave {
        /// The player id.
        player: u32,
    },
}

/// `PreFox` for a pre-Fox version: this module is the Fox half alone. The
/// one guard `parse`, `to_toml` and `apply` share.
fn fox_only(version: PesVersion) -> Result<(), PatchError> {
    match version.engine() {
        Engine::PreFox => Err(PatchError::PreFox { version }),
        Engine::Fox => Ok(()),
    }
}

/// `unix_seconds` as the `compiled` stamp's form: a UTC datetime at whole
/// seconds with offset `Z`.
pub fn utc_datetime(unix_seconds: u64) -> toml_edit::Datetime {
    let days = unix_seconds / 86_400;
    let seconds = unix_seconds % 86_400;
    // Howard Hinnant's civil-from-days: `z` counts days from 0000-03-01, the
    // shift moving the leap day to each year's end.
    let z = i64::try_from(days).expect("a day count far inside i64") + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
    let year = u16::try_from(y + i64::from(month <= 2)).expect("a TOML year");
    toml_edit::Datetime {
        date: Some(toml_edit::Date { year, month, day }),
        time: Some(toml_edit::Time {
            hour: (seconds / 3600) as u8,
            minute: (seconds % 3600 / 60) as u8,
            second: (seconds % 60) as u8,
            nanosecond: 0,
        }),
        offset: Some(toml_edit::Offset::Z),
    }
}

impl AestheticsPatch {
    /// Reads an `aesthetics_patch.toml` text. Every rule is an error, never
    /// a skip: the five header keys are `MissingKey`, any other top-level
    /// key `UnknownKey`, a pre-Fox `pes_version` `PreFox`, and each team's
    /// `[teams.players.NN]` takes the `name`, `shirt_name` and
    /// `[appearance.motion]` of the Fox patch alone.
    pub fn parse(text: &str) -> Result<Self, PatchError> {
        let document: DocumentMut = text
            .parse()
            .map_err(|e: toml_edit::TomlError| PatchError::Toml(e.to_string()))?;
        let root = document.as_table();
        let pes_version = version(required(root, "", "pes_version")?)?;
        fox_only(pes_version)?;
        let tool_version = string(required(root, "", "tool_version")?, "tool_version")?;
        let allocation_scheme_version = u32_value(
            required(root, "", "allocation_scheme_version")?,
            "allocation_scheme_version",
        )?;
        let cpks = string_array(required(root, "", "cpks")?, "cpks")?;
        let compiled = datetime(required(root, "", "compiled")?, "compiled")?;
        for (name, _) in root.iter() {
            const KNOWN: [&str; 6] = [
                "pes_version",
                "tool_version",
                "allocation_scheme_version",
                "cpks",
                "compiled",
                "teams",
            ];
            if !KNOWN.contains(&name) {
                return Err(SettingsError::UnknownKey {
                    key: name.to_string(),
                }
                .into());
            }
        }
        let mut teams = Vec::new();
        if let Some(item) = root.get("teams") {
            teams = parse_teams(item)?;
        }
        Ok(AestheticsPatch {
            pes_version,
            tool_version,
            allocation_scheme_version,
            cpks,
            compiled,
            teams,
        })
    }

    /// The patch's text form: the header comment and its
    /// five keys, then each team a `[[teams]]` block with a
    /// `[teams.players.NN]` per player — the header always written, `name`
    /// and `shirt_name` only when set — and its `[…appearance.motion]`
    /// table when a motion key is set. One blank line before each header.
    /// `Err` is a value `parse` would refuse back: a pre-Fox `pes_version`
    /// (`PreFox`), a `players.NN` key outside `01` to `99` (`UnknownKey` at
    /// `teams[i].players.NN`), or a stored value a key's range cannot hold.
    pub fn to_toml(&self) -> Result<String, PatchError> {
        fox_only(self.pes_version)?;
        let mut out = HEADER.to_string();
        out.push_str(&format!("pes_version = {}\n", self.pes_version.number()));
        out.push_str(&format!(
            "tool_version = {}\n",
            Value::from(self.tool_version.as_str())
        ));
        out.push_str(&format!(
            "allocation_scheme_version = {}\n",
            self.allocation_scheme_version
        ));
        let cpks = self
            .cpks
            .iter()
            .map(|cpk| Value::from(cpk.as_str()).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("cpks = [{cpks}]\n"));
        out.push_str(&format!("compiled = {}\n", self.compiled));
        for (i, team) in self.teams.iter().enumerate() {
            out.push_str(&format!("\n[[teams]]\nid = {}\n", team.id));
            out.push_str(&padded(
                &format!("name = {}", Value::from(team.name.as_str())),
                "for the reader; the id is what is applied",
            ));
            out.push('\n');
            for (slot, player) in &team.players {
                let key = format!("{slot:02}");
                if !players_key(&key, 1..=99) {
                    return Err(SettingsError::UnknownKey {
                        key: format!("teams[{i}].players.{key}"),
                    }
                    .into());
                }
                out.push_str(&format!("\n[teams.players.{key}]\n"));
                if let Some(name) = &player.name {
                    out.push_str(&format!("name = {}\n", Value::from(name.as_str())));
                }
                if let Some(shirt_name) = &player.shirt_name {
                    out.push_str(&format!(
                        "shirt_name = {}\n",
                        Value::from(shirt_name.as_str())
                    ));
                }
                if player.motion != MotionSettings::default() {
                    out.push_str(&format!("\n[teams.players.{slot:02}.appearance.motion]\n"));
                    emit_motion(&mut out, &player.motion)?;
                }
            }
        }
        Ok(out)
    }

    /// Applies the patch to `save`: each listed player's motions and, for
    /// a `write_names` export, his names — nothing of his appearance block, so
    /// a stripped player (appearance id -1) stays stripped. Every refusal
    /// runs before any write: a pre-Fox `pes_version` (a hand-built patch
    /// can hold one), a version or allocation-scheme mismatch, and in pass
    /// 1 each player's names against the version's text fields, each `Some`
    /// motion key against its range and the field's width; an `Err` leaves
    /// the save untouched.
    /// Only a team or player the save lacks is skipped, per item, and
    /// reported in the summary.
    pub fn apply(&self, save: &mut EditFile) -> Result<PatchSummary, PatchError> {
        fox_only(self.pes_version)?;
        if self.pes_version != save.version() {
            return Err(PatchError::VersionMismatch {
                patch: self.pes_version,
                save: save.version(),
            });
        }
        if self.allocation_scheme_version != ALLOCATION_SCHEME_VERSION {
            return Err(PatchError::SchemeMismatch {
                patch: self.allocation_scheme_version,
                suite: ALLOCATION_SCHEME_VERSION,
            });
        }
        let schema = schema_for(save.version());
        let mut applied_teams = Vec::new();
        let mut skipped = Vec::new();
        // Pass 1: resolve each entry against the save, building the writes
        // on copies; an error returns with the save untouched.
        let mut built: Vec<(u32, PlayerEntry)> = Vec::new();
        for (i, team) in self.teams.iter().enumerate() {
            let team_path = format!("teams[{i}]");
            if save.team(team.id).is_none() {
                skipped.push(PatchSkip::TeamNotInSave { team: team.id });
                continue;
            }
            applied_teams.push(team.id);
            for (slot, patch_player) in &team.players {
                let player_path = format!("{team_path}.players.{slot:02}");
                // The patch's team id is any u32, so `id * 100 + NN` can
                // overflow: unchecked it wraps onto a different, real
                // player's id, who would be written with no error. An
                // unrepresentable id is the player not being in the save,
                // reported at u32::MAX for a number u32 cannot carry.
                let Some(player_id) = team
                    .id
                    .checked_mul(100)
                    .and_then(|id| id.checked_add(u32::from(*slot)))
                else {
                    skipped.push(PatchSkip::PlayerNotInSave { player: u32::MAX });
                    continue;
                };
                let Some(player) = save.player(player_id) else {
                    skipped.push(PatchSkip::PlayerNotInSave { player: player_id });
                    continue;
                };
                built.push((
                    player_id,
                    build(player, patch_player, &player_path, schema)?,
                ));
            }
        }
        let applied_players = built.iter().map(|(player_id, _)| *player_id).collect();
        // Pass 2: every built entry replaces its player.
        for (player_id, entry) in built {
            *save
                .player_mut(player_id)
                .expect("present: pass 1 found it") = entry;
        }
        Ok(PatchSummary {
            applied_teams,
            applied_players,
            skipped,
        })
    }
}

// ---------------------------------------------------------------------------
// Apply

/// The patch's writes onto a copy of `player` — the `Some` names checked
/// against the version's text fields as `PlayerSettings::apply` checks them,
/// each `Some` motion key against its kind's range — then written through
/// `apply_appearance` so a `Some` the version has no field for is
/// `NotInThisVersion`. The copy is encoded into a scratch record of the
/// version's player size so a stored value past the field's width is
/// refused here (`Codec`), not when the save is written. `path` is the
/// player's patch path (`teams[0].players.05`).
fn build(
    player: &PlayerEntry,
    patch: &PatchPlayer,
    path: &str,
    schema: &VersionSchema,
) -> Result<PlayerEntry, PatchError> {
    let mut next = player.clone();
    if let Some(name) = &patch.name {
        check_text(
            &leaf(path, "name"),
            name,
            text_max(schema.player.texts, PlayerText::Name),
            false,
        )?;
        next.name = name.clone();
    }
    if let Some(shirt_name) = &patch.shirt_name {
        check_text(
            &leaf(path, "shirt_name"),
            shirt_name,
            text_max(schema.player.texts, PlayerText::ShirtName),
            true,
        )?;
        next.shirt_name = shirt_name.clone();
    }
    let motion_path = format!("{path}.appearance.motion");
    let appearance = AppearanceSettings {
        motion: patch.motion.clone(),
        ..AppearanceSettings::default()
    };
    for key in SettingKey::ALL {
        let spec = key.spec();
        let Some(stored) = get_appearance(&appearance, key) else {
            continue;
        };
        // The patch's own version's range, not the widest the key table
        // holds; a key the version lacks is `version_kind`'s `None` and
        // `apply_appearance`'s `NotInThisVersion`, as before.
        let Some(kind) = key.version_kind(schema.version) else {
            continue;
        };
        if !kind.accepts_stored(stored) {
            return Err(SettingsError::OutOfRange {
                key: format!("{motion_path}.{}", spec.name),
                value: i64::from(stored),
                range: range_text(kind),
            }
            .into());
        }
    }
    // The only refusal the motion-only write can hit (`dribbling` on a
    // version without the field); its key names the patch path.
    apply_appearance(&appearance, &mut next).map_err(|err| match err {
        SettingsError::NotInThisVersion { key } => SettingsError::NotInThisVersion {
            key: format!("{motion_path}.{key}"),
        },
        other => other,
    })?;
    let mut record = vec![0u8; schema.player.size];
    write_player(&next, &mut record, schema.player).map_err(SettingsError::Codec)?;
    Ok(next)
}

// ---------------------------------------------------------------------------
// Parse

/// `path.name`, or `name` at the root (a top-level key's path is its name).
fn leaf(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

/// `table.name`, `MissingKey` at `path.name` when absent.
fn required<'a>(table: &'a dyn TableLike, path: &str, name: &str) -> Result<&'a Item, PatchError> {
    table.get(name).ok_or_else(|| PatchError::MissingKey {
        key: leaf(path, name),
    })
}

/// `item` as a table-like (`teams`, `players.NN`, `appearance`); `key` is
/// its dotted path.
fn table_like<'a>(item: &'a Item, key: &str) -> Result<&'a dyn TableLike, PatchError> {
    item.as_table_like().ok_or_else(|| {
        SettingsError::WrongType {
            key: key.to_string(),
            expected: "a table",
        }
        .into()
    })
}

/// `item` as a string (`tool_version`, team and player names).
fn string(item: &Item, key: &str) -> Result<String, PatchError> {
    item.as_value()
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            SettingsError::WrongType {
                key: key.to_string(),
                expected: "a string",
            }
            .into()
        })
}

/// `item` as a `u32` (team `id`, `allocation_scheme_version`).
fn u32_value(item: &Item, key: &str) -> Result<u32, PatchError> {
    let value = item
        .as_value()
        .and_then(|v| v.as_integer())
        .ok_or_else(|| SettingsError::WrongType {
            key: key.to_string(),
            expected: "an integer",
        })?;
    u32::try_from(value).map_err(|_| {
        SettingsError::OutOfRange {
            key: key.to_string(),
            value,
            range: "0 to 4294967295".to_string(),
        }
        .into()
    })
}

/// `item` as an array of strings (the `cpks` list).
fn string_array(item: &Item, key: &str) -> Result<Vec<String>, PatchError> {
    let array =
        item.as_value()
            .and_then(|v| v.as_array())
            .ok_or_else(|| SettingsError::WrongType {
                key: key.to_string(),
                expected: "an array of strings",
            })?;
    array
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                SettingsError::WrongType {
                    key: key.to_string(),
                    expected: "an array of strings",
                }
                .into()
            })
        })
        .collect()
}

/// `item` as a TOML datetime (the `compiled` stamp).
fn datetime(item: &Item, key: &str) -> Result<toml_edit::Datetime, PatchError> {
    item.as_value()
        .and_then(|v| v.as_datetime())
        .copied()
        .ok_or_else(|| {
            SettingsError::WrongType {
                key: key.to_string(),
                expected: "a datetime",
            }
            .into()
        })
}

/// `item` as a `PesVersion` (the `pes_version` key): an integer naming a
/// version, anything else `WrongType`.
fn version(item: &Item) -> Result<PesVersion, PatchError> {
    item.as_value()
        .and_then(|v| v.as_integer())
        .and_then(|v| u16::try_from(v).ok())
        .and_then(PesVersion::from_number)
        .ok_or_else(|| {
            SettingsError::WrongType {
                key: "pes_version".to_string(),
                expected: "a PES version, 15 to 21",
            }
            .into()
        })
}

/// The `teams` array of tables; each element's path is `teams[i]`,
/// zero-based.
fn parse_teams(item: &Item) -> Result<Vec<PatchTeam>, PatchError> {
    let teams = item
        .as_array_of_tables()
        .ok_or_else(|| SettingsError::WrongType {
            key: "teams".to_string(),
            expected: "an array of tables",
        })?;
    teams
        .iter()
        .enumerate()
        .map(|(i, table)| parse_team(table, &format!("teams[{i}]")))
        .collect()
}

/// One `[[teams]]`: `id` and `name` required, `players` absent = empty,
/// anything else `UnknownKey`.
fn parse_team(table: &dyn TableLike, path: &str) -> Result<PatchTeam, PatchError> {
    let id = u32_value(required(table, path, "id")?, &leaf(path, "id"))?;
    let name = string(required(table, path, "name")?, &leaf(path, "name"))?;
    for (name, _) in table.iter() {
        if !["id", "name", "players"].contains(&name) {
            return Err(SettingsError::UnknownKey {
                key: leaf(path, name),
            }
            .into());
        }
    }
    let mut players = BTreeMap::new();
    if let Some(item) = table.get("players") {
        players = parse_players(item, &leaf(path, "players"))?;
    }
    Ok(PatchTeam { id, name, players })
}

/// A team's `players` table: keys are two digits `01` to `99` (`NN` of
/// `team id × 100 + NN`), anything else `UnknownKey`.
fn parse_players(item: &Item, path: &str) -> Result<BTreeMap<u8, PatchPlayer>, PatchError> {
    let table = table_like(item, path)?;
    let mut out = BTreeMap::new();
    for (name, item) in table.iter() {
        if !players_key(name, 1..=99) {
            return Err(SettingsError::UnknownKey {
                key: leaf(path, name),
            }
            .into());
        }
        let slot = name.parse::<u8>().expect("checked above");
        out.insert(slot, parse_player(item, &leaf(path, name))?);
    }
    Ok(out)
}

/// One `[teams.players.NN]`: `name` and `shirt_name` optional strings,
/// `appearance` a table holding `motion` alone; every other key or
/// subtable (the pre-Fox patch's `boots_id`, `appearance.face`, …) is
/// `UnknownKey`. No length checks: those are `apply`'s, against the
/// version's fields.
fn parse_player(item: &Item, path: &str) -> Result<PatchPlayer, PatchError> {
    let table = table_like(item, path)?;
    let mut player = PatchPlayer {
        name: None,
        shirt_name: None,
        motion: MotionSettings::default(),
    };
    if let Some(item) = table.get("name") {
        player.name = Some(string(item, &leaf(path, "name"))?);
    }
    if let Some(item) = table.get("shirt_name") {
        player.shirt_name = Some(string(item, &leaf(path, "shirt_name"))?);
    }
    if let Some(item) = table.get("appearance") {
        player.motion = parse_appearance(item, &leaf(path, "appearance"))?;
    }
    for (name, _) in table.iter() {
        if !["name", "shirt_name", "appearance"].contains(&name) {
            return Err(SettingsError::UnknownKey {
                key: leaf(path, name),
            }
            .into());
        }
    }
    Ok(player)
}

/// A Fox patch's `appearance` table holds `motion` alone; any other key or
/// subtable (a pre-Fox patch's `[appearance.face]`, …) is `UnknownKey`.
fn parse_appearance(item: &Item, path: &str) -> Result<MotionSettings, PatchError> {
    let table = table_like(item, path)?;
    let mut motion = MotionSettings::default();
    if let Some(item) = table.get("motion") {
        motion = parse_motion(item, &leaf(path, "motion"))?;
    }
    for (name, _) in table.iter() {
        if name != "motion" {
            return Err(SettingsError::UnknownKey {
                key: leaf(path, name),
            }
            .into());
        }
    }
    Ok(motion)
}
