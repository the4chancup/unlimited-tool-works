//! The `aesthetics_patch.toml` planning decides (`spec/team_compiler/pipeline.md` "6. Post-processing"):
//! each compiled team export's players' looks at their resolved values, their boots and
//! gloves IDs as `export_rows` resolves them, and a `write_names` export's names.
//! `patch_names` is the names rule `check` reports and `compile` builds with, restated from
//! the export alone the way `item_rows::conflicting_id_keys` is.

use std::collections::{BTreeMap, BTreeSet};

use pes_savefile::interchange::aesthetics_patch::{PatchPlayer, PatchTeam};
use pes_savefile::settings_toml::{AppearanceSettings, NameField, NameSetting, PlayerSettings};
use pes_version::PesVersion;

use super::item_rows::{ItemRow, ItemTable, RowPlayer};

/// What the names rule gives of one player folder: the names the patch writes and the facts
/// its findings read.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct PatchNames {
    /// The player's `name` write; `None` when the file gives none or the field refuses it.
    pub(crate) name: Option<String>,
    /// The player's `shirt_name` write, like `name`.
    pub(crate) shirt_name: Option<String>,
    /// Each key whose text the version's field refused (`settings_name_invalid`, the key its
    /// context), `name` then `shirt_name`.
    pub(crate) invalid: Vec<&'static str>,
    /// The folder gives `name` or `shirt_name` and more than one player maps it
    /// (`settings_toml_name_shared`), whether or not a check then drops one.
    pub(crate) shared: bool,
}

/// The names rule (`spec/aesthetics_export/settings_toml.md` "Name-writing rules"), for a kept
/// player folder of a `write_names` team export. `mapped` is how many players
/// the roster maps it to, `player_name` its folder's name part. `name = true` resolves to
/// `player_name`, `name` or `shirt_name` as a string is written as is, each resolved text
/// checked as `PlayerSettings::apply` checks it (`NameField::check`), a refused one dropped and
/// its key in `invalid`. An export without the marker resolves nothing and reports nothing,
/// whatever the file says; a folder with no `settings.toml` (`settings` `None`, or the
/// defaults when the file was refused or the folder holds only an `ingame_face.toml`) resolves
/// nothing either.
pub(crate) fn patch_names(
    settings: Option<&PlayerSettings>,
    player_name: &str,
    mapped: usize,
    write_names: bool,
    version: PesVersion,
) -> PatchNames {
    let mut names = PatchNames::default();
    let Some(settings) = settings.filter(|_| write_names) else {
        return names;
    };
    names.shared = mapped > 1 && (settings.name.is_some() || settings.shirt_name.is_some());
    let name = settings.name.as_ref().map(|setting| match setting {
        NameSetting::FromFolder => player_name.to_owned(),
        NameSetting::Explicit(name) => name.clone(),
    });
    names.name = checked(NameField::Name, name, version, &mut names.invalid);
    names.shirt_name = checked(
        NameField::ShirtName,
        settings.shirt_name.clone(),
        version,
        &mut names.invalid,
    );
    names
}

/// `text` when the version's field takes it (`field.check`); `None` else, the key noted in
/// `invalid`.
fn checked(
    field: NameField,
    text: Option<String>,
    version: PesVersion,
    invalid: &mut Vec<&'static str>,
) -> Option<String> {
    let text = text?;
    match field.check(&text, version) {
        Ok(()) => Some(text),
        Err(_) => {
            invalid.push(field.key());
            None
        }
    }
}

/// A `[[teams]]` block of the run's `aesthetics_patch.toml` as planning decides it: the
/// team itself, with each player's boots and gloves IDs provisional, beside the export's
/// item rows the IDs came from, for `settle`.
pub(crate) struct PlannedPatchTeam {
    /// The patch's `[[teams]]` entry: each `boots_id`/`gloves_id` the one `rows` gives,
    /// a task-gated row's included though the task commits only later.
    pub(crate) team: PatchTeam,
    /// The export's item rows the entries' ID keys were read from (`patch_team`): `settle`
    /// removes an ID whose row's task the writer did not commit.
    pub(crate) rows: Vec<ItemRow>,
}

/// The `[[teams]]` block the compiled player folders `players` of one team export give the
/// patch: `id` the team's, `name` its teams-list name (`/co/`), one `[teams.players.NN]` per
/// player a folder maps — `NN` his player id's slot — `appearance` his joined settings
/// resolved to `version` (the template's defaults when the folder has none), `name`/`shirt_name`
/// the names rule's (`write_names` the export's marker), and `boots_id`/`gloves_id` his
/// `rows`' provisionally, two players of one folder holding different ones when their
/// slots' rows differ. A folder with a refused settings file
/// (`settings_refused`) gives no look: his entry holds no `[appearance]` and none of the
/// IDs his settings would resolve — the taskless rows' authored stock ID and marker or
/// coverage default — the save keeping his whole look and his IDs, while an ID his
/// folder's models or links get still lands; left with nothing at all — no look, no
/// names, no IDs — he gets no entry
/// (one holding only IDs is kept, `settle` deciding them later).
/// `None` when `players` holds no player with an entry: a patch with no player is no patch,
/// and the export gets no `[[teams]]` entry.
pub(super) fn patch_team(
    players: &[RowPlayer],
    rows: &[ItemRow],
    team_id: u16,
    team_name: &str,
    write_names: bool,
    version: PesVersion,
) -> Option<PatchTeam> {
    let mut patch_players = BTreeMap::new();
    for player in players {
        let appearance = if player.settings_refused {
            None
        } else {
            Some(
                player
                    .settings
                    .as_ref()
                    .map_or_else(AppearanceSettings::default, |s| s.appearance.clone())
                    .resolved(version),
            )
        };
        let names = patch_names(
            player.settings.as_ref(),
            &player.player_name,
            player.player_ids.len(),
            write_names,
            version,
        );
        for player_id in &player.player_ids {
            let mut entry = PatchPlayer {
                name: names.name.clone(),
                shirt_name: names.shirt_name.clone(),
                boots_id: None,
                gloves_id: None,
                base_copy_id: None,
                appearance: appearance.clone(),
            };
            for table in ItemTable::ALL {
                if let Some(row) = rows
                    .iter()
                    .find(|row| row.table == table && row.player_id == *player_id)
                {
                    // A refused settings file holds the IDs his settings would resolve
                    // back with his whole look — the authored stock ID and the marker or
                    // coverage default — so the save keeps his; an ID his folder's
                    // models or links get still lands.
                    if player.settings_refused && row.task.is_none() {
                        continue;
                    }
                    let id = u16::try_from(row.id).expect("a boots or gloves ID fits 14 bits");
                    match table {
                        ItemTable::Boots => entry.boots_id = Some(id),
                        ItemTable::Gloves => entry.gloves_id = Some(id),
                    }
                }
            }
            if holds_nothing(&entry) {
                continue;
            }
            patch_players.insert(
                u8::try_from(player_id % 100).expect("a roster slot is under 100"),
                entry,
            );
        }
    }
    if patch_players.is_empty() {
        return None;
    }
    Some(PatchTeam {
        id: u32::from(team_id),
        name: team_name.to_owned(),
        players: patch_players,
    })
}

/// `teams`' provisional `boots_id`/`gloves_id` settled against the batches the writer
/// committed (`committed`, the manifest positions): an ID whose row does not count
/// (`ItemRow::counts`) is removed, so the save keeps his current one; an
/// entry left holding nothing is dropped (`holds_nothing`), then a team left with no
/// player. Called once every batch's outcome is known, before the patch is written.
pub(crate) fn settle(teams: Vec<PlannedPatchTeam>, committed: &BTreeSet<usize>) -> Vec<PatchTeam> {
    teams
        .into_iter()
        .filter_map(|planned| {
            let PlannedPatchTeam { mut team, rows } = planned;
            for (slot, entry) in &mut team.players {
                settle_entry(entry, *slot, team.id, &rows, committed);
            }
            team.players.retain(|_, entry| !holds_nothing(entry));
            (!team.players.is_empty()).then_some(team)
        })
        .collect()
}

/// Whether `teams`' patch still holds an entry once its provisional IDs are settled
/// against `committed`: `settle`'s own rule, asked without settling, the writer's
/// `finish_team` reading it to decide whether the run writes a CPK for the patch at
/// all — a patch is the run's work as an override is, so one holding an entry forces
/// the CPK and one that settles to nothing forces none.
pub(crate) fn settles_to_anything(teams: &[PlannedPatchTeam], committed: &BTreeSet<usize>) -> bool {
    teams.iter().any(|planned| {
        planned.team.players.iter().any(|(slot, entry)| {
            let mut entry = entry.clone();
            settle_entry(&mut entry, *slot, planned.team.id, &planned.rows, committed);
            !holds_nothing(&entry)
        })
    })
}

/// `entry`, slot `slot` of team `team_id`, settled against `committed` as `settle`
/// settles it: an ID whose row's task did not commit is removed.
fn settle_entry(
    entry: &mut PatchPlayer,
    slot: u8,
    team_id: u32,
    rows: &[ItemRow],
    committed: &BTreeSet<usize>,
) {
    let player_id = team_id * 100 + u32::from(slot);
    for table in ItemTable::ALL {
        let key = match table {
            ItemTable::Boots => &mut entry.boots_id,
            ItemTable::Gloves => &mut entry.gloves_id,
        };
        if let Some(row) = rows
            .iter()
            .find(|row| row.table == table && row.player_id == player_id)
            && !row.counts(committed)
        {
            *key = None;
        }
    }
}

/// Whether `entry` holds a key to write: a player the patch gives nothing
/// gets no `[teams.players.NN]` entry.
fn holds_nothing(entry: &PatchPlayer) -> bool {
    entry.name.is_none()
        && entry.shirt_name.is_none()
        && entry.boots_id.is_none()
        && entry.gloves_id.is_none()
        && entry.base_copy_id.is_none()
        && entry.appearance.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `RowPlayer` of one folder mapped to `id` alone: `settings` its joined
    /// files (default when not parsed), `settings_refused` whether one was
    /// refused.
    fn row_player(id: u32, settings_refused: bool) -> RowPlayer {
        RowPlayer {
            path: vtree::ScopePath::new(format!("Players/{id}").as_str()).unwrap(),
            player_ids: vec![id],
            linked: Vec::new(),
            fpc: None,
            own_parts: Vec::new(),
            player_name: "A".to_owned(),
            settings: Some(PlayerSettings::default()),
            settings_refused,
        }
    }

    /// A `PatchPlayer` holding the resolved default look alone.
    fn default_entry() -> PatchPlayer {
        PatchPlayer {
            name: None,
            shirt_name: None,
            boots_id: None,
            gloves_id: None,
            base_copy_id: None,
            appearance: Some(AppearanceSettings::default().resolved(PesVersion::Pes21)),
        }
    }

    /// An `ItemRow` of `table` for `player_id` taking `id`, gated on `task` (`Some`),
    /// or unconditional (`None`).
    fn row(table: ItemTable, player_id: u32, id: u32, task: Option<usize>) -> ItemRow {
        ItemRow {
            table,
            player_id,
            id,
            task,
        }
    }

    /// A player the patch gives nothing — a refused settings file and no
    /// names or IDs for him — gets no `[teams.players.NN]` entry; one the
    /// patch has something for keeps his.
    #[test]
    fn a_player_left_with_nothing_to_write_gets_no_entry() {
        // His settings file was refused and nothing else of his is to be
        // written: no entry, and no `[[teams]]` block when he is the team.
        assert_eq!(
            patch_team(
                &[row_player(71403, true)],
                &[],
                714,
                "/co/",
                false,
                PesVersion::Pes21
            ),
            None,
            "no player with an entry, no team block"
        );
        // Beside a kept player, the refused folder is the one missing.
        let kept = row_player(71405, false);
        let team = patch_team(
            &[row_player(71403, true), kept],
            &[],
            714,
            "/co/",
            false,
            PesVersion::Pes21,
        )
        .expect("a team block");
        assert!(
            !team.players.contains_key(&3),
            "no entry for a refused file"
        );
        assert!(
            team.players[&5].appearance.is_some(),
            "the kept player's resolved look"
        );
    }

    /// Each player id's entry gets the IDs of his rows provisionally, the task-gated
    /// ones included: a folder two slots map gives each entry its own slot's row's ID,
    /// and an entry holding only an ID is kept for `settle` to decide.
    #[test]
    fn an_entry_holds_his_rows_ids_provisionally() {
        let mut player = row_player(71405, false);
        player.player_ids = vec![71405, 71408];
        let rows = [
            row(ItemTable::Boots, 71405, 625, Some(1)),
            row(ItemTable::Boots, 71408, 628, Some(1)),
            row(ItemTable::Gloves, 71405, 630, Some(2)),
            row(ItemTable::Gloves, 71408, 630, Some(2)),
        ];
        let team = patch_team(&[player], &rows, 714, "/co/", false, PesVersion::Pes21)
            .expect("a team block");
        assert_eq!(team.players[&5].boots_id, Some(625));
        assert_eq!(team.players[&5].gloves_id, Some(630));
        assert_eq!(team.players[&8].boots_id, Some(628));
        assert_eq!(team.players[&8].gloves_id, Some(630));

        // A refused settings file takes his look and the IDs his settings would resolve;
        // a task-gated boots row's ID still lands and alone makes an entry.
        let refused = row_player(71407, true);
        let rows = [
            row(ItemTable::Boots, 71407, 627, Some(1)),
            row(ItemTable::Gloves, 71407, 0, None),
        ];
        let team = patch_team(&[refused], &rows, 714, "/co/", false, PesVersion::Pes21)
            .expect("the ID alone keeps his entry");
        let entry = &team.players[&7];
        assert_eq!(entry.boots_id, Some(627), "the task-gated row's ID lands");
        assert_eq!(entry.gloves_id, None, "a taskless row's does not");
        assert_eq!(entry.appearance, None);
    }

    /// `settle`: an ID whose row's task committed stays, one whose did not is removed —
    /// a taskless row's unconditionally; an entry left with nothing to write is dropped,
    /// then a team left with no player.
    #[test]
    fn settle_drops_uncommitted_ids_then_empty_entries_and_teams() {
        let mut committed_entry = default_entry();
        committed_entry.boots_id = Some(625);
        let mut failed_entry = default_entry();
        failed_entry.boots_id = Some(626);
        failed_entry.gloves_id = Some(630);
        let id_only = PatchPlayer {
            name: None,
            shirt_name: None,
            boots_id: Some(627),
            gloves_id: None,
            base_copy_id: None,
            appearance: None,
        };
        let taskless = PatchPlayer {
            boots_id: Some(12),
            ..id_only.clone()
        };
        let teams = vec![
            PlannedPatchTeam {
                team: PatchTeam {
                    id: 714,
                    name: "/co/".to_owned(),
                    players: BTreeMap::from([
                        (5, committed_entry.clone()),
                        (6, failed_entry),
                        (7, id_only.clone()),
                        (8, taskless.clone()),
                    ]),
                },
                rows: vec![
                    row(ItemTable::Boots, 71405, 625, Some(1)),
                    row(ItemTable::Boots, 71406, 626, Some(2)),
                    row(ItemTable::Gloves, 71406, 630, Some(2)),
                    row(ItemTable::Boots, 71407, 627, Some(2)),
                    row(ItemTable::Boots, 71408, 12, None),
                ],
            },
            // Its one player left with nothing: no `[[teams]]` block for it either.
            PlannedPatchTeam {
                team: PatchTeam {
                    id: 792,
                    name: "/egg/".to_owned(),
                    players: BTreeMap::from([(3, id_only)]),
                },
                rows: vec![row(ItemTable::Boots, 79203, 627, Some(2))],
            },
        ];

        let settled = settle(teams, &BTreeSet::from([1]));

        let [team] = settled.as_slice() else {
            panic!("the team with a kept entry alone survives: {settled:?}")
        };
        assert_eq!(team.id, 714);
        assert_eq!(team.players[&5].boots_id, Some(625), "committed");
        assert_eq!(team.players[&6].boots_id, None, "uncommitted");
        assert_eq!(team.players[&6].gloves_id, None, "uncommitted");
        assert!(team.players[&6].appearance.is_some(), "his look stays");
        assert!(
            !team.players.contains_key(&7),
            "the ID his failed task gave was his entry's whole write"
        );
        assert_eq!(
            team.players[&8].boots_id,
            Some(12),
            "a taskless row's ID stays"
        );
    }

    /// `settles_to_anything` answers as `settle` does, without settling: false
    /// for a team whose one entry holds only a task-gated `boots_id` whose task
    /// did not commit, true when it committed, and true for an entry holding an
    /// `[appearance]` whatever committed.
    #[test]
    fn settles_to_anything_decides_whether_the_patch_still_holds_an_entry() {
        let gated = PatchPlayer {
            name: None,
            shirt_name: None,
            boots_id: Some(627),
            gloves_id: None,
            base_copy_id: None,
            appearance: None,
        };
        let team = |entry: PatchPlayer| PlannedPatchTeam {
            team: PatchTeam {
                id: 714,
                name: "/co/".to_owned(),
                players: BTreeMap::from([(5, entry)]),
            },
            rows: vec![row(ItemTable::Boots, 71405, 627, Some(2))],
        };

        assert!(
            !settles_to_anything(&[team(gated.clone())], &BTreeSet::from([1])),
            "the ID his uncommitted task gave was his entry's whole write"
        );
        assert!(
            settles_to_anything(&[team(gated)], &BTreeSet::from([2])),
            "the committed task's ID keeps the entry"
        );
        // An `[appearance]` is no task's to drop: the entry survives even with
        // no batch committed.
        let mut with_look = default_entry();
        with_look.boots_id = Some(627);
        assert!(settles_to_anything(&[team(with_look)], &BTreeSet::new()));
    }

    /// `patch_names` over one folder's settings, `mapped` players, `write_names` as given.
    fn names(settings: &PlayerSettings, mapped: usize, write_names: bool) -> PatchNames {
        patch_names(
            Some(settings),
            "Snuffy",
            mapped,
            write_names,
            PesVersion::Pes21,
        )
    }

    #[test]
    fn an_export_without_the_marker_resolves_and_reports_nothing() {
        let settings = PlayerSettings::parse("name = \"X\"\nshirt_name = \"Y\"\n").unwrap();
        assert_eq!(names(&settings, 2, false), PatchNames::default());
    }

    #[test]
    fn name_true_resolves_to_the_folder_name_and_explicit_as_written() {
        let settings = PlayerSettings::parse("name = true\n").unwrap();
        let resolved = names(&settings, 1, true);
        assert_eq!(resolved.name.as_deref(), Some("Snuffy"));
        let settings = PlayerSettings::parse("name = \"Ace\"\n").unwrap();
        assert_eq!(names(&settings, 1, true).name.as_deref(), Some("Ace"));
    }

    #[test]
    fn a_name_the_field_refuses_is_dropped_with_its_key() {
        let long = format!("name = \"{}\"\nshirt_name = \"Ā\"\n", "a".repeat(70));
        let settings = PlayerSettings::parse(&long).unwrap();
        let resolved = names(&settings, 1, true);
        assert_eq!(resolved.name, None);
        assert_eq!(resolved.shirt_name, None);
        assert_eq!(resolved.invalid, ["name", "shirt_name"]);
    }

    #[test]
    fn a_folder_mapped_to_several_giving_a_name_is_shared_checked_or_not() {
        let settings = PlayerSettings::parse("name = \"Shared\"\n").unwrap();
        assert!(names(&settings, 2, true).shared);
        assert!(!names(&settings, 1, true).shared);
        // The check drops the name; the folder still gave it for two players.
        let settings = PlayerSettings::parse(&format!("name = \"{}\"\n", "a".repeat(70))).unwrap();
        let resolved = names(&settings, 2, true);
        assert!(resolved.shared);
        assert_eq!(resolved.invalid, ["name"]);
        // No name key at all: nothing shared.
        let settings =
            PlayerSettings::parse("[appearance.motion]\nhunching_dribbling = 2\n").unwrap();
        assert!(!names(&settings, 2, true).shared);
    }
}
