//! The compiled players' boots and gloves IDs, decided at planning, which the
//! `aesthetics_patch.toml`'s `[[teams]]` entries carry. They are resolved per category: the
//! folder's own models or a link file take the ID a `Models` task emits them under once it
//! commits, an authored stock ID is written as is, and else the marker or the coverage
//! decides the default.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use aesthetics_export::{
    ExportCoverage, FpcDirective, PlayerFolder, PlayerSlot, SharedKind, ValidatedAestheticsExport,
};
use pes_savefile::settings_toml::PlayerSettings;
use pes_version::Engine;
use vtree::ScopePath;

use super::ids::shared_folders_taking_ids;
use super::legacy_blocks::in_block;
use super::legacy_ids::LegacyIds;
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
    /// The packages the folder's effective boots and gloves have a part of
    /// (`has_effective_part`; computed only under `legacy_ids`, empty otherwise): under
    /// the file, a slot recording no old ID of a kind his folder packs gets no row of it.
    pub(super) own_parts: Vec<ModelPackage>,
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
///
/// `legacy_ids` is the export's `legacy_ids.toml` when it holds one: the resolution is then
/// per slot, not per folder (`legacy_rows`), and `block` the team's VGL27 old-ID block.
pub(super) fn export_rows(
    tasks: &[BuildTask],
    first: usize,
    players: &[RowPlayer],
    coverage: ExportCoverage,
    hide_ids: fpc::HideIds,
    legacy_ids: Option<&LegacyIds>,
    block: Option<&RangeInclusive<u32>>,
) -> Vec<ItemRow> {
    let mut items = Vec::new();
    for player in players {
        for table in ItemTable::ALL {
            let package = package(table);
            if let Some(legacy) = legacy_ids {
                items.extend(legacy_rows(
                    tasks,
                    first,
                    player,
                    table,
                    (legacy, block),
                    coverage,
                    hide_ids,
                ));
                continue;
            }
            let own = models_task(tasks, first, &player.path, package)
                .map(|(task, keys)| (task, keys.iter().map(|key| row_id(*key)).collect()));
            let linked = || {
                linked_id(tasks, first, player, package)
                    .map(|(task, id)| (task, vec![id; player.player_ids.len()]))
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

/// `player`'s rows of `table`'s category under `legacy`, the export's `legacy_ids.toml`
/// and its team's VGL27 block:
/// a slot the file records an old ID of the kind for takes it as his row, gated on the
/// `Models` task of `package` emitting it when the ID lies in `block`, his own folder's
/// or the earlier slot's that packs it once for both, and held unconditionally when the
/// block check refuses it (`legacy_id_out_of_block`): no task packs it, but the patch
/// keeps it, a player wearing another team's boots as his save sets it. A slot it
/// records none for gets no row when his folder packs
/// the kind anyway (`own_parts`), the save keeping his ID; when his folder has no package
/// of the kind he resolves as an export without the file does: a linked shared folder's
/// ID, an authored stock ID, then the marker's or coverage's default.
fn legacy_rows(
    tasks: &[BuildTask],
    first: usize,
    player: &RowPlayer,
    table: ItemTable,
    legacy: (&LegacyIds, Option<&RangeInclusive<u32>>),
    coverage: ExportCoverage,
    hide_ids: fpc::HideIds,
) -> Vec<ItemRow> {
    let (legacy, block) = legacy;
    let package = package(table);
    let mut items = Vec::new();
    for &player_id in &player.player_ids {
        let slot =
            PlayerSlot::new(u8::try_from(player_id % 100).expect("a roster slot is under 100"))
                .expect("a player id's last digits are his roster slot");
        if let Some(id) = legacy.old_id(slot, table) {
            if in_block(block, id) {
                // An old ID no task packs (the folder recording it holds no model of the
                // kind) gives no row: the save keeps his ID rather than naming an empty
                // package.
                if let Some(task) = task_holding(tasks, first, package, id) {
                    items.push(ItemRow {
                        table,
                        player_id,
                        id,
                        task: Some(task),
                    });
                }
            } else {
                items.push(ItemRow {
                    table,
                    player_id,
                    id,
                    task: None,
                });
            }
        } else if !player.own_parts.contains(&package) {
            // As an export without the file resolves: a linked folder's ID, an authored
            // stock ID, the marker's or coverage's default.
            let resolved = linked_id(tasks, first, player, package)
                .map(|(task, id)| (id, Some(task)))
                .or_else(|| {
                    authored_id(player.settings.as_ref(), table)
                        .map(u32::from)
                        .map(|id| (id, None))
                })
                .or_else(|| default_id(player.fpc, table, coverage, hide_ids).map(|id| (id, None)));
            if let Some((id, task)) = resolved {
                items.push(ItemRow {
                    table,
                    player_id,
                    id,
                    task,
                });
            }
        }
    }
    items
}

/// The manifest position and shared ID of the `Models` task of `tasks`, from `first` on,
/// building `package` of a shared folder `player` links; `None` when no link of his has a
/// task of that package.
fn linked_id(
    tasks: &[BuildTask],
    first: usize,
    player: &RowPlayer,
    package: ModelPackage,
) -> Option<(usize, u32)> {
    player.linked.iter().find_map(|path| {
        let (task, keys) = models_task(tasks, first, path, package)?;
        let key = *keys
            .first()
            .expect("a shared folder's package is emitted under its one shared id");
        Some((task, row_id(key)))
    })
}

/// The manifest position of the `Models` task of `tasks`, from `first` on, that emits
/// `package` under `id`, whichever folder's it is: the task a `legacy_ids.toml` slot's
/// row is gated on, his own folder's or the earlier slot's packing the old ID once for
/// both. `None` when no task emits it, the file naming an ID no folder packs: the slot
/// then gets no row.
fn task_holding(
    tasks: &[BuildTask],
    first: usize,
    package: ModelPackage,
    id: u32,
) -> Option<usize> {
    tasks
        .iter()
        .enumerate()
        .skip(first)
        .find_map(|(index, task)| match &task.kind {
            TaskKind::Models {
                package: built,
                ids,
                ..
            } if *built == package && ids.contains(&PackageKey::Id(id)) => Some(index),
            _ => None,
        })
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
    use crate::plan::legacy_ids::LegacyIds;
    use crate::plan::{PlanReport, plan_run};
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
        rows_and_models(plan_run(
            vec![to_plan(ExportId(0), export, two_team_colors(), None)],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        ))
    }

    /// `report`'s rows and `Models` tasks, as `planned` compares them.
    fn rows_and_models(report: PlanReport) -> (Vec<ItemRow>, Vec<PlannedModels>) {
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

    /// `resolved`'s export `name`, planned for PES 21 with `ids` as its
    /// `legacy_ids.toml` (parse errors are the parser's own tests'). Its rows and
    /// `Models` tasks, as `planned` gives them.
    fn planned_with_ids(
        name: &str,
        files: &[(&str, u64)],
        players_txt: Option<&[u8]>,
        ids: &str,
    ) -> (Vec<ItemRow>, Vec<PlannedModels>) {
        let export = resolved(name, files, &[], players_txt);
        let mut to_plan = to_plan(ExportId(0), export, two_team_colors(), None);
        to_plan.legacy_ids = Some(LegacyIds::from_toml(ids).unwrap());
        // The tests' old IDs sit inside this block; the block check's own cases cover
        // the out-of-block paths.
        to_plan.legacy_block = Some(2301..=2325);
        rows_and_models(plan_run(
            vec![to_plan],
            PesVersion::Pes21,
            fpc::HideIds::CONVENTION,
        ))
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

    // One old gloves ID two folders record packs once, the first slot's folder holding
    // the task both rows point at.
    #[test]
    fn an_old_id_two_folders_record_packs_once_and_gives_both_slots_its_row() {
        let (rows, models) = planned_with_ids(
            "co Midcup Rows",
            &[
                ("Players/05 - A/glove_l.fmdl", 1),
                ("Players/06 - B/glove_l.fmdl", 1),
            ],
            None,
            "[05]\ngloves = 2310\n[06]\ngloves = 2310\n",
        );

        assert_eq!(
            models,
            [
                (0, ModelPackage::Face, vec![71405]),
                (1, ModelPackage::Gloves, vec![2310]),
                (2, ModelPackage::Face, vec![71406]),
            ]
        );
        assert_eq!(
            rows,
            [
                set(ItemTable::Gloves, 71405, 2310, 1),
                set(ItemTable::Gloves, 71406, 2310, 1),
            ]
        );
    }

    // A slot the file gives no old ID of a kind gets no key of it: a folder mapping
    // two slots gives its keys per slot, so slot 05's row does not take 07's ID; a
    // folder holding parts of a kind it records nothing of packs none.
    #[test]
    fn a_slot_without_a_recorded_id_gets_no_key_and_no_row_of_the_kind() {
        let (rows, models) = planned_with_ids(
            "co Full Rows",
            &[
                ("Players/Shared/boots.fmdl", 1),
                ("Players/09 - D/boots.fmdl", 1),
            ],
            Some(b"05 Shared\n07 Shared\n09 09 - D\n"),
            "[07]\nboots = 2305\n",
        );

        // Shared's boots emit under 07's 2305 alone: no key for 05, none for 09, so
        // D's boots are not packed at all.
        assert_eq!(
            models,
            [
                (0, ModelPackage::Face, vec![71405, 71407]),
                (1, ModelPackage::Boots, vec![2305]),
                (2, ModelPackage::Face, vec![71409]),
            ]
        );
        assert_eq!(
            rows,
            [
                set(ItemTable::Boots, 71407, 2305, 1),
                default(ItemTable::Gloves, 71405, 0),
                default(ItemTable::Gloves, 71407, 0),
                default(ItemTable::Gloves, 71409, 0),
            ]
        );
    }

    // An old ID the file records for a slot whose folder holds no model of the kind is
    // packed by no task, so it gives no row: the save keeps his ID.
    #[test]
    fn a_recorded_old_id_no_task_packs_gives_no_row() {
        let (rows, models) = planned_with_ids(
            "co Full Rows",
            &[("Players/05 - A/face_high.fmdl", 1)],
            None,
            "[05]\nboots = 2305\n",
        );

        assert_eq!(models, [(0, ModelPackage::Face, vec![71405])]);
        assert_eq!(rows, [default(ItemTable::Gloves, 71405, 0)]);
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
