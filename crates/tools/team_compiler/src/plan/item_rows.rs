//! The compiled players' boots and gloves IDs, decided at planning, which the
//! `aesthetics_patch.toml`'s `[[teams]]` entries carry. They are resolved per category: the
//! folder's own models or a link file take the ID a `Models` task emits them under once it
//! commits, an authored stock ID is written as is, and else the marker or the coverage
//! decides the default.

use std::collections::BTreeSet;

use aesthetics_export::{
    ExportCoverage, FpcDirective, PlayerFolder, SharedKind, ValidatedAestheticsExport,
};
use pes_savefile::settings_toml::PlayerSettings;
use pes_version::Engine;
use vtree::ScopePath;

use super::ids::shared_folders_taking_ids;
use super::roles::{
    ModelPackage, has_effective_part, link_feeds_own_package, linked_folder, shared_kind_of,
};
use super::{BuildTask, TaskKind};
use crate::paths::PackageKey;

/// One of the two categories a player's boots or gloves ID belongs to: the key the item
/// rows, the patch's entries and `settings_model_id_conflict` share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemTable {
    /// Boots: the `boots_id` key, the `ModelPackage::Boots` package.
    Boots,
    /// Gloves: the `gloves_id` key, the `ModelPackage::Gloves` package.
    Gloves,
}

impl ItemTable {
    /// Both categories, in the order the patch's entries hold them.
    pub(crate) const ALL: [ItemTable; 2] = [ItemTable::Boots, ItemTable::Gloves];
}

/// One compiled player's resolved boots or gloves ID, as planning decides it
/// ("Bins accumulation"), which the patch writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemRow {
    /// The category the ID is in.
    pub(crate) table: ItemTable,
    /// The player's id: his team ID times 100 plus his roster slot (71405).
    pub(crate) player_id: u32,
    /// What his ID becomes.
    pub(crate) id: u32,
    /// The manifest position of the task putting the item under `id` into the CPK: the ID
    /// lands only once it commits, the save keeping the player's current one when it does
    /// not. `None` for an authored stock ID or a resolved default, which no task emits.
    pub(crate) task: Option<usize>,
}

impl ItemRow {
    /// Whether the row's write happens: it has no task, or its task is among `committed`,
    /// the manifest positions of the batches the writer committed. The patch's `settle`
    /// decides a row by it.
    pub(crate) fn counts(&self, committed: &BTreeSet<usize>) -> bool {
        self.task.is_none_or(|task| committed.contains(&task))
    }
}

/// A compiled player folder as its rows need it.
pub(super) struct RowPlayer {
    /// The folder's export path, which its own `Models` tasks compile.
    pub(super) path: ScopePath,
    /// The player id of each roster slot mapping the folder, in slot order: the order of the
    /// ids its own `Models` tasks emit their package under.
    pub(super) player_ids: Vec<u32>,
    /// The export paths of the shared folders the player folder links.
    pub(super) linked: Vec<ScopePath>,
    /// The folder's `fpc_on`/`fpc_off` marker; `None` when it has none.
    pub(super) fpc: Option<FpcDirective>,
    /// The folder's name part, which a `write_names` export's `name = true` resolves to.
    pub(super) player_name: String,
    /// The folder's settings, its `settings.toml` joined with its `ingame_face.toml`'s keys;
    /// `None` when it holds neither file (the patch's `[appearance]` is the template's).
    pub(super) settings: Option<PlayerSettings>,
    /// One of the folder's settings files was refused
    /// (`settings_toml_invalid`/`ingame_face_toml_invalid`): an invalid file holds back the
    /// player's whole look, so the patch's entry for him holds no `[appearance]`.
    pub(super) settings_refused: bool,
}

/// The item rows of `players`, the compiled player folders of one export whose `coverage`
/// is given, against `tasks`, the manifest so far, whose export's tasks start at `first`:
/// the resolved boots and gloves IDs the patch's entries carry. Per category
/// and in the resolution's order: a player's own package, emitted under each of his slots'
/// exclusive ids, or the package of a shared folder he links, emitted under its shared id,
/// each gated on its task, and an authored stock ID for that category then ignored:
/// validation reports `settings_model_id_conflict` for it (`conflicting_id_keys`), which
/// `check` reaches too. A link that combines makes the shared folder
/// a part of his own package, so his own task is the one. With neither, an authored stock
/// ID is written, else the default `default_id` resolves: `hide_ids`'s (the common
/// settings' hide IDs) for `fpc_on`, 0 for `fpc_off` or a `Full` export's markerless player,
/// and no row for a `Midcup` one's, the save keeping his ID. Gloves alike.
pub(super) fn export_rows(
    tasks: &[BuildTask],
    first: usize,
    players: &[RowPlayer],
    coverage: ExportCoverage,
    hide_ids: fpc::HideIds,
) -> Vec<ItemRow> {
    let mut items = Vec::new();
    for player in players {
        for table in ItemTable::ALL {
            let package = package(table);
            let own = models_task(tasks, first, &player.path, package)
                .map(|(task, keys)| (task, keys.iter().map(|key| row_id(*key)).collect()));
            let linked = || {
                player.linked.iter().find_map(|path| {
                    let (task, keys) = models_task(tasks, first, path, package)?;
                    let key = *keys
                        .first()
                        .expect("a shared folder's package is emitted under its one shared id");
                    Some((task, vec![row_id(key); player.player_ids.len()]))
                })
            };
            match own.or_else(linked) {
                Some((task, ids)) => {
                    items.extend(player.player_ids.iter().zip(ids).map(|(player_id, id)| {
                        ItemRow {
                            table,
                            player_id: *player_id,
                            id,
                            task: Some(task),
                        }
                    }));
                }
                None => {
                    let id = authored_id(player.settings.as_ref(), table)
                        .map(u32::from)
                        .or_else(|| default_id(player.fpc, table, coverage, hide_ids));
                    if let Some(id) = id {
                        items.extend(player.player_ids.iter().map(|player_id| ItemRow {
                            table,
                            player_id: *player_id,
                            id,
                            task: None,
                        }));
                    }
                }
            }
        }
    }
    items
}

/// The stock ID `settings` authors for `table`'s category; `None` for `""` or an absent
/// `boots_id`/`gloves_id` key, or no `settings.toml` at all.
fn authored_id(settings: Option<&PlayerSettings>, table: ItemTable) -> Option<u8> {
    let settings = settings?;
    match table {
        ItemTable::Boots => settings.boots_id,
        ItemTable::Gloves => settings.gloves_id,
    }
}

/// The `boots_id`/`gloves_id` key of `table`'s category, as `settings_model_id_conflict`
/// names it.
fn key(table: ItemTable) -> &'static str {
    match table {
        ItemTable::Boots => "boots_id",
        ItemTable::Gloves => "gloves_id",
    }
}

/// The `boots_id`/`gloves_id` keys `settings`, the `settings.toml` of `player`, a mapped
/// player folder of `export`, authors a stock ID for whose category's ID planning assigns on
/// a target of `engine` (`settings_model_id_conflict`): `export_rows`' `own.or_else(linked)`
/// `Some` restated from the export alone, which `check` has where the manifest is never
/// built (`category_assigned`). Boots' key first.
pub(crate) fn conflicting_id_keys(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    settings: Option<&PlayerSettings>,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> Vec<&'static str> {
    ItemTable::ALL
        .into_iter()
        .filter(|table| {
            authored_id(settings, *table).is_some()
                && category_assigned(export, player, *table, engine, hand_weighted)
        })
        .map(key)
        .collect()
}

/// Whether planning assigns `player`, a player folder of `export`, an ID for `table`'s
/// category on a target of `engine`, what `export_rows`' task lookup finds: a `Models` task
/// of the category's package for his own folder, which planning gives exactly when his
/// effective package of that kind has a part (`has_effective_part`), or for a shared folder of
/// its kind he links (`shared_folders_taking_ids` being those planning gives one).
/// `has_effective_part` reads his own files and his linked faces, not his combining boots or
/// gloves links (it is what decides their combining), while `folder_tasks` reads every source;
/// the two agree because validation confines a shared boots or gloves folder's models to its
/// own kind's names (`check_shared`, `fmdl_name_invalid`), so such a link adds no part of the
/// other kind, and a part of its own kind only where his own already plans the task.
fn category_assigned(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    table: ItemTable,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    has_effective_part(export, player, package(table), engine, hand_weighted)
        || linked_takes_id(export, player, table, engine, hand_weighted)
}

/// Whether a shared folder of `table`'s kind that `player` links takes an ID for `engine`
/// (`shared_folders_taking_ids`), planning then giving its package a `Models` task of its
/// own: `export_rows`' `linked` lookup, where the task stands for the folder.
fn linked_takes_id(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    table: ItemTable,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    let kind = shared_kind_of(package(table));
    let taking = shared_folders_taking_ids(export, engine, kind, hand_weighted);
    player
        .links
        .iter()
        .filter(|link| link.kind == kind)
        .filter_map(|link| linked_folder(export, link))
        .any(|linked| taking.iter().any(|folder| folder.path == linked.path))
}

/// The ID the resolution's default rule gives a player whose folder provides neither the
/// category's models nor a stock key: `hide_ids`'s (the common settings' hide IDs) for
/// `fpc_on`, 0 for `fpc_off` or no marker of a `Full` export's player, and no row for a
/// `Midcup` export's markerless player, the save keeping his ID.
fn default_id(
    fpc: Option<FpcDirective>,
    table: ItemTable,
    coverage: ExportCoverage,
    hide_ids: fpc::HideIds,
) -> Option<u32> {
    match fpc {
        Some(FpcDirective::On) => Some(u32::from(match table {
            ItemTable::Boots => hide_ids.boots(),
            ItemTable::Gloves => hide_ids.gloves(),
        })),
        Some(FpcDirective::Off) => Some(0),
        None => match coverage {
            ExportCoverage::Full => Some(0),
            ExportCoverage::Midcup => None,
        },
    }
}

/// The gloves ID `player` resolves to for `fpc::check`'s gloves rule, decided from the
/// export alone the way `conflicting_id_keys` restates `export_rows`' resolution for
/// `check`: a compiler-assigned ID's stand-in (the
/// rule reads only `== 0`, and an assigned ID is a real model's, never 0), else the authored
/// stock ID, else `default_id`'s marker or coverage answer. `None` is a `Midcup` export's
/// markerless player, the save keeping his ID; `check` reads no save, so his
/// gloves rule does not run. `category_assigned` does not cover a referee's gloves link,
/// which feeds his slot's package (`link_feeds_own_package`) and takes no shared ID.
pub(crate) fn check_gloves_id(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    settings: Option<&PlayerSettings>,
    engine: Engine,
    hand_weighted: &BTreeSet<ScopePath>,
    hide_ids: fpc::HideIds,
) -> Option<u16> {
    let assigned = category_assigned(export, player, ItemTable::Gloves, engine, hand_weighted)
        || player.links.iter().any(|link| {
            link.kind == SharedKind::Gloves
                && link_feeds_own_package(export, engine, player, link, hand_weighted)
        });
    if assigned {
        // A stand-in: the rule reads only whether the ID is 0, and an assigned
        // ID never is (a team block starts at 101, a referee's is his slot's).
        return Some(1);
    }
    authored_id(settings, ItemTable::Gloves)
        .map(u16::from)
        .or_else(|| {
            default_id(player.fpc, ItemTable::Gloves, export.coverage, hide_ids)
                .map(|id| u16::try_from(id).expect("the resolved defaults fit u16"))
        })
}

/// The boots or gloves id a team player's package is emitted under, which his row points at.
fn row_id(key: PackageKey) -> u32 {
    match key {
        PackageKey::Id(id) => id,
        PackageKey::Referee(_) => unreachable!("only a team export's players have rows"),
    }
}

/// The package whose IDs `table` lists.
fn package(table: ItemTable) -> ModelPackage {
    match table {
        ItemTable::Boots => ModelPackage::Boots,
        ItemTable::Gloves => ModelPackage::Gloves,
    }
}

/// The manifest position and keys of the `Models` task of `tasks`, from position `first` on,
/// that builds `package` of the folder at `path`; `None` when no task does (the folder holds
/// no model of it).
fn models_task<'a>(
    tasks: &'a [BuildTask],
    first: usize,
    path: &ScopePath,
    package: ModelPackage,
) -> Option<(usize, &'a [PackageKey])> {
    tasks
        .iter()
        .enumerate()
        .skip(first)
        .find_map(|(index, task)| {
            if let TaskKind::Models {
                folder,
                package: built,
                ids,
                ..
            } = &task.kind
                && folder.path == *path
                && *built == package
            {
                Some((index, ids.as_slice()))
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;
    use studio_core::ExportId;

    use super::*;
    use crate::plan::plan_run;
    use crate::testing::{resolved, to_plan, two_team_colors};

    /// A `Models` task as the tests compare it: its manifest position, package and ids.
    type PlannedModels = (usize, ModelPackage, Vec<u32>);

    /// The `/co/` export `<coverage> Rows`, planned for PES 21: slot 05 holds boots of its own
    /// and links the shared gloves `Keeper`, slot 06 holds only a face, slot 07 holds boots of
    /// its own and links the shared boots `Crocs`, which combine with them. Its rows, and its
    /// `Models` tasks.
    fn planned(coverage: &str) -> (Vec<ItemRow>, Vec<PlannedModels>) {
        let export = resolved(
            &format!("co {coverage} Rows"),
            &[
                ("Players/05 - A/boots.fmdl", 1),
                ("Players/05 - A/Keeper.gloves", 0),
                ("Players/06 - B/face_high.fmdl", 1),
                ("Players/07 - C/kit_boots.fmdl", 1),
                ("Players/07 - C/Crocs.boots", 0),
                ("Gloves/Keeper/glove_l.fmdl", 1),
                ("Boots/Crocs/boots.fmdl", 1),
            ],
            &[],
            None,
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        let models = report
            .manifest
            .tasks
            .iter()
            .enumerate()
            .filter_map(|(index, task)| match &task.kind {
                TaskKind::Models { package, ids, .. } => Some((
                    index,
                    *package,
                    ids.iter().map(|key| row_id(*key)).collect(),
                )),
                TaskKind::Textures { .. }
                | TaskKind::CommonTextures { .. }
                | TaskKind::CommonModels { .. }
                | TaskKind::Portrait { .. }
                | TaskKind::Kit { .. }
                | TaskKind::Logo { .. }
                | TaskKind::RefereeMarker { .. }
                | TaskKind::Collar { .. } => None,
            })
            .collect();
        (report.manifest.patch_teams[0].rows.clone(), models)
    }

    /// The row setting `player_id`'s item of `table` to `id` once task `task` commits.
    fn set(table: ItemTable, player_id: u32, id: u32, task: usize) -> ItemRow {
        ItemRow {
            table,
            player_id,
            id,
            task: Some(task),
        }
    }

    /// The row setting `player_id`'s item of `table` to `id` unconditionally: an authored
    /// stock ID or a resolved default, which no task emits.
    fn default(table: ItemTable, player_id: u32, id: u32) -> ItemRow {
        ItemRow {
            table,
            player_id,
            id,
            task: None,
        }
    }

    /// The tasks `planned` gives both exports: the three faces, slot 05's and slot 07's boots
    /// under their exclusive ids (07's with `Crocs` combined in, so `Crocs` takes no id), then
    /// `Keeper`'s gloves under slot 05's exclusive id, he being its one wearer.
    fn expected_models() -> Vec<PlannedModels> {
        vec![
            (0, ModelPackage::Face, vec![71405]),
            (1, ModelPackage::Boots, vec![625]),
            (2, ModelPackage::Face, vec![71406]),
            (3, ModelPackage::Face, vec![71407]),
            (4, ModelPackage::Boots, vec![627]),
            (5, ModelPackage::Gloves, vec![625]),
        ]
    }

    #[test]
    fn a_full_export_sets_its_players_rows_from_their_tasks_and_defaults_the_rest() {
        let (rows, models) = planned("Full");
        assert_eq!(models, expected_models());
        assert_eq!(
            rows,
            [
                set(ItemTable::Boots, 71405, 625, 1),
                set(ItemTable::Gloves, 71405, 625, 5),
                default(ItemTable::Boots, 71406, 0),
                default(ItemTable::Gloves, 71406, 0),
                set(ItemTable::Boots, 71407, 627, 4),
                default(ItemTable::Gloves, 71407, 0),
            ]
        );
    }

    #[test]
    fn a_midcup_export_sets_its_players_rows_and_leaves_the_rest_alone() {
        let (rows, models) = planned("Midcup");
        assert_eq!(models, expected_models());
        assert_eq!(
            rows,
            [
                set(ItemTable::Boots, 71405, 625, 1),
                set(ItemTable::Gloves, 71405, 625, 5),
                set(ItemTable::Boots, 71407, 627, 4),
            ]
        );
    }

    #[test]
    fn a_folder_two_slots_map_gives_each_slot_its_own_row() {
        let export = resolved(
            "co Midcup Twice",
            &[
                ("Players/A/boots.fmdl", 1),
                ("Players/B/Keeper.gloves", 0),
                ("Gloves/Keeper/glove_l.fmdl", 1),
            ],
            &[],
            Some(b"03 A\n08 A\n09 B\n11 B\n"),
        );
        let report = plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        );
        // A's face (0) and boots (1) under slots 03 and 08; B's face (2); Keeper's gloves (3).
        assert_eq!(
            report.manifest.patch_teams[0].rows,
            [
                set(ItemTable::Boots, 71403, 623, 1),
                set(ItemTable::Boots, 71408, 628, 1),
                set(ItemTable::Gloves, 71409, 644, 3),
                set(ItemTable::Gloves, 71411, 644, 3),
            ]
        );
        // And each slot its own patch entry, carrying the template's look while the folder
        // has none.
        let entry_slots: Vec<u8> = report.manifest.patch_teams[0]
            .team
            .players
            .keys()
            .copied()
            .collect();
        assert_eq!(entry_slots, [3, 8, 9, 11]);
    }

    /// A taskless row counts outright; a task-gated one once its task is among the
    /// committed batches, and not before.
    #[test]
    fn a_row_counts_without_a_task_or_once_its_task_committed() {
        let committed = BTreeSet::from([1, 3]);
        assert!(
            default(ItemTable::Boots, 71405, 0).counts(&committed),
            "a taskless row counts"
        );
        assert!(set(ItemTable::Boots, 71405, 625, 1).counts(&committed));
        assert!(set(ItemTable::Boots, 71405, 625, 3).counts(&committed));
        assert!(
            !set(ItemTable::Boots, 71405, 625, 2).counts(&committed),
            "an uncommitted task's row does not"
        );
        assert!(
            !set(ItemTable::Boots, 71405, 625, 4).counts(&committed),
            "a task that never ran does not"
        );
    }
}
