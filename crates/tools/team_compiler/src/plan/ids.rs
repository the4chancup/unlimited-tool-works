//! The per-team block of boots and gloves IDs
//! (`spec/aesthetics_export/player_folders.md` "Assigns IDs automatically"). Every team owns a
//! fixed block of 40 IDs, the first 23 being its players' exclusive ones and the other 17 its
//! shared folders'. So planning computes every ID, processing never allocates one, and the same
//! export always compiles to the same IDs.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{
    PlayerFolder, PlayerSlot, SharedKind, SharedModelFolder, ValidatedAestheticsExport,
    ValidatedRoster,
};
use pes_version::Engine;
use teams_list::TeamId;
use vtree::ScopePath;

use super::roles::{FolderModels, PlayerFile, link_feeds_own_package, package_of, player_file};
use crate::bins::shared_ids::SharedIds;

/// The first ID of the first team's block: IDs 0 to 100 are the stock band.
const FIRST_BLOCK: u16 = 101;

/// A team's block: the 23 player-exclusive IDs, then 17 for its shared folders.
const BLOCK_SIZE: u16 = 40;

/// The block's first IDs, one per roster slot.
const EXCLUSIVE_COUNT: u16 = 23;

/// The shared folders of one kind a team's block has IDs for: the block's remainder after the
/// exclusive ones. An export needing more is `boots_id_pool_exhausted` or
/// `gloves_id_pool_exhausted`.
pub(crate) const SHARED_COUNT: u16 = BLOCK_SIZE - EXCLUSIVE_COUNT;

/// One team's planned boots and gloves IDs. Boots and gloves are disjoint game namespaces
/// (`boots/k0625/`, `glove/g0625/`), so one block serves both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedModelIds {
    /// The block's first ID, slot 01's.
    block_start: u16,
}

impl PlannedModelIds {
    /// Team `team`'s block, `101 + (team - 701) * 40`: team 701 gets 101 to 140, and the last
    /// team, 920, ends at 8900, clear of the referees' `99XX` band.
    pub(crate) fn for_team(team: TeamId) -> PlannedModelIds {
        PlannedModelIds {
            block_start: FIRST_BLOCK + (team.get() - TeamId::MIN) * BLOCK_SIZE,
        }
    }

    /// The exclusive boots and gloves ID of the player at roster slot `slot`: the block's
    /// `slot`-th ID, so slot 01 gets the block's first.
    pub(crate) fn exclusive(self, slot: PlayerSlot) -> u16 {
        self.block_start + u16::from(slot.get()) - 1
    }

    /// The shared ID at `index` in the kind's alphabetical folder order: the block's IDs after
    /// the exclusive ones, so index 0 is the 24th. `None` past the pool, which planning never
    /// reaches: the structure pass drops an export needing more.
    pub(crate) fn shared(self, index: usize) -> Option<u16> {
        let index = u16::try_from(index)
            .ok()
            .filter(|index| *index < SHARED_COUNT)?;
        Some(self.block_start + EXCLUSIVE_COUNT + index)
    }
}

/// The shared ID each of `folders` (`folder_ids`' `shared` for `kind`, in
/// `shared_folders_taking_ids`' case-folded alphabetical order) takes, in the same order, for
/// a team of block `ids`. A `Midcup` export keeps the shared IDs the team already has:
///
/// - with `record` `None` (a `Full` export, or a `Midcup` the installed CPKs hold no record
///   of), the n-th folder takes the n-th of the block's shared IDs;
/// - with a `Midcup`'s record, a folder it names (names compare case-folded) keeps its ID, and
///   every other folder takes the lowest of the block's shared IDs neither the record's entries
///   nor the earlier new folders hold.
///
/// `Err(demand)` when the block's shared IDs do not reach. `demand` is the count of them the
/// team's folders need: the record's in-block entries plus the folders it does not name (a
/// `Full` export's being its folders). `pool_messages` reports it as `boots_id_pool_exhausted`
/// or `gloves_id_pool_exhausted`. A recorded ID outside the block takes no free slot: it is not
/// one of the seventeen.
pub(crate) fn assigned_shared_ids(
    ids: PlannedModelIds,
    kind: SharedKind,
    folders: &[&SharedModelFolder],
    record: Option<&SharedIds>,
) -> Result<Vec<u16>, usize> {
    let table = record.map(|record| record.of(kind));
    let first = ids.shared(0).expect("a team's block holds the shared IDs");
    // The record's folder names folded to the IDs they hold, the lower ID winning a folded
    // collision (a hand-edited file's).
    let held: BTreeMap<String, u16> =
        table
            .into_iter()
            .flatten()
            .fold(BTreeMap::new(), |mut held, (id, name)| {
                held.entry(vtree::fold_name(name)).or_insert(*id);
                held
            });
    // The shared IDs the record's entries hold inside the block.
    let taken = table.map_or(0, |table| {
        table
            .keys()
            .filter(|id| (first..first + SHARED_COUNT).contains(id))
            .count()
    });
    let new = folders
        .iter()
        .filter(|folder| !held.contains_key(&vtree::fold_name(&folder.folder_name)))
        .count();
    if new + taken > usize::from(SHARED_COUNT) {
        return Err(taken + new);
    }
    let mut free = (0..SHARED_COUNT)
        .map(|index| first + index)
        .filter(|id| table.is_none_or(|table| !table.contains_key(id)));
    Ok(folders
        .iter()
        .map(|folder| {
            held.get(&vtree::fold_name(&folder.folder_name))
                .copied()
                .unwrap_or_else(|| {
                    free.next()
                        .expect("the new folders fit the free shared IDs, checked above")
                })
        })
        .collect())
}

/// The shared folders of `kind` in `export` that take an ID when compiled for `engine`, in
/// the ID order of those taking a shared one: the folders at least one roster-mapped player
/// folder links plainly, by name (case folded, ties by the plain spelling), that hold a model
/// of their kind (`holds_model_of_its_kind`). Which ID each takes, its one wearer's exclusive
/// one or a shared one, is `folder_ids`' split. A shared boots or gloves folder holding no
/// model of its kind takes no ID (`shared_folders_with_no_model`): it has nothing to load, and
/// an ID would shift the IDs of the folders sorted after it for an output that emits nothing.
/// A shared face never takes one (on Fox it merges into the player's face; pre-Fox it is
/// copied per player). Neither does a folder only unmapped folders link, since they are not
/// compiled, nor a refs export's, whose every link feeds the referee's own slot package
/// (`link_feeds_own_package`). `hand_weighted` is
/// the models the deep pass found carrying hand weights, whose split hands make a player's
/// gloves link combine.
pub(crate) fn shared_folders_taking_ids<'a>(
    export: &'a ValidatedAestheticsExport,
    engine: Engine,
    kind: SharedKind,
    hand_weighted: &BTreeSet<ScopePath>,
) -> Vec<&'a SharedModelFolder> {
    let mut taking: Vec<&SharedModelFolder> =
        plainly_linked_folders(export, engine, kind, hand_weighted)
            .into_iter()
            .map(|(folder, _)| folder)
            .filter(|folder| holds_model_of_its_kind(folder, engine, kind))
            .collect();
    taking.sort_by_cached_key(|folder| {
        (
            vtree::fold_name(&folder.folder_name),
            folder.folder_name.clone(),
        )
    });
    taking
}

/// The boots or gloves folders of one kind taking an ID, split by where it comes from.
pub(crate) struct FolderIds<'a> {
    /// Each folder exactly one roster slot links plainly and the record does not name,
    /// with that slot: it takes the slot's exclusive ID.
    pub(crate) own: Vec<(&'a SharedModelFolder, PlayerSlot)>,
    /// Every other folder, in `shared_folders_taking_ids`' order: it takes a shared ID
    /// (`assigned_shared_ids`).
    pub(crate) shared: Vec<&'a SharedModelFolder>,
}

/// `shared_folders_taking_ids`' folders of `kind` in `export` compiled for `engine`, split by
/// where their ID comes from. A folder exactly one roster slot links plainly and `record` (a
/// `Midcup`'s installed record, names compared case-folded as `assigned_shared_ids` compares
/// them; `None` for a `Full` export) does not name takes that slot's exclusive ID, every
/// other a shared one. The wearers are counted in slots, not
/// player folders: one folder `players.txt` maps to two slots makes two.
///
/// The exclusive ID is free for it: a link is plain exactly when it does not combine, and a
/// boots or gloves link combines exactly when the player's own package of that kind has a
/// part (`link_combines`, `has_effective_part`), which is the only case planning emits that
/// package. A plain linker therefore has no package of his own under his exclusive ID of the
/// link's kind, and the folder packs there as a shared-ID folder does
/// (`TextureHome::SharedOutput`).
pub(crate) fn folder_ids<'a>(
    export: &'a ValidatedAestheticsExport,
    engine: Engine,
    kind: SharedKind,
    hand_weighted: &BTreeSet<ScopePath>,
    record: Option<&SharedIds>,
) -> FolderIds<'a> {
    let taking = shared_folders_taking_ids(export, engine, kind, hand_weighted);
    // The record's folder names of the kind, folded as `assigned_shared_ids` folds them. A
    // face has no table to read and no entry in `taking` either.
    let named: BTreeSet<String> = match kind {
        SharedKind::Boots | SharedKind::Gloves => record
            .map(|record| record.of(kind))
            .into_iter()
            .flatten()
            .map(|(_, name)| vtree::fold_name(name))
            .collect(),
        SharedKind::Face => BTreeSet::new(),
    };
    // The roster's slot to player folder map. A referee export's `taking` is empty: its
    // every link feeds the referee's own slot package.
    let slots: Vec<(PlayerSlot, &PlayerFolder)> = match &export.roster {
        ValidatedRoster::Team(slots) => slots
            .iter()
            .map(|(slot, index)| (*slot, &export.players[index.0]))
            .collect(),
        ValidatedRoster::Referees(_) => Vec::new(),
    };
    let mut ids = FolderIds {
        own: Vec::new(),
        shared: Vec::new(),
    };
    for folder in taking {
        let name_key = vtree::fold_name(&folder.folder_name);
        let mut wearers = slots
            .iter()
            .filter(|(_, player)| {
                links_plainly(export, player, engine, kind, &name_key, hand_weighted)
            })
            .map(|(slot, _)| *slot);
        match (wearers.next(), wearers.next()) {
            (Some(slot), None) if !named.contains(&name_key) => {
                ids.own.push((folder, slot));
            }
            _ => ids.shared.push(folder),
        }
    }
    ids
}

/// The shared folders of `kind` in `export` that a roster-mapped player folder links plainly
/// for `engine` but that hold no model of their kind, in the export's folder order, each with
/// the first player folder linking it plainly, in roster order: the folders
/// `shared_folders_taking_ids` leaves out for having nothing to load, which validation reports
/// as `shared_folder_no_model`. A player linking one wears the game's own. `hand_weighted` as
/// for `shared_folders_taking_ids`.
pub(crate) fn shared_folders_with_no_model<'a>(
    export: &'a ValidatedAestheticsExport,
    engine: Engine,
    kind: SharedKind,
    hand_weighted: &BTreeSet<ScopePath>,
) -> Vec<(&'a SharedModelFolder, &'a PlayerFolder)> {
    plainly_linked_folders(export, engine, kind, hand_weighted)
        .into_iter()
        .filter(|(folder, _)| !holds_model_of_its_kind(folder, engine, kind))
        .collect()
}

/// The shared folders of `kind` in `export` that at least one roster-mapped player folder
/// links plainly for `engine`, in the export's folder order, each with the first such player
/// folder in roster order; none for a shared face.
fn plainly_linked_folders<'a>(
    export: &'a ValidatedAestheticsExport,
    engine: Engine,
    kind: SharedKind,
    hand_weighted: &BTreeSet<ScopePath>,
) -> Vec<(&'a SharedModelFolder, &'a PlayerFolder)> {
    let folders = match kind {
        SharedKind::Face => return Vec::new(),
        SharedKind::Boots => &export.boots,
        SharedKind::Gloves => &export.gloves,
    };
    let players = players_in_roster_order(export);
    folders
        .iter()
        .filter_map(|folder| {
            let name_key = vtree::fold_name(&folder.folder_name);
            let player = players.iter().find(|player| {
                links_plainly(export, player, engine, kind, &name_key, hand_weighted)
            })?;
            Some((folder, *player))
        })
        .collect()
}

/// Whether the shared boots or gloves `folder` of `kind` holds a model of its kind for a
/// target of `engine`: on Fox a model its package takes (`PlayerFile::Model`), a `.common`
/// link to a Common model included (`PlayerFile::CommonModel`), on pre-Fox a
/// model its output packs (`PlayerFile::PreFoxModel`), or on either a glTF selected for its
/// stem (`PlayerFile::UnsupportedGltf`). The glTF counts: planning drops the folder for it
/// with `model_gltf_unsupported`, which is what the member is to be told, not that the folder
/// has no model.
fn holds_model_of_its_kind(folder: &SharedModelFolder, engine: Engine, kind: SharedKind) -> bool {
    let models = FolderModels::of_shared(&folder.path, &folder.files, kind, engine);
    folder.files.iter().any(|file| {
        let role = player_file(&folder.path, file, &models);
        matches!(
            role,
            Some(PlayerFile::Model { package, .. } | PlayerFile::CommonModel { package, .. })
                if package == package_of(kind)
        ) || matches!(
            role,
            Some(PlayerFile::PreFoxModel { .. } | PlayerFile::UnsupportedGltf)
        )
    })
}

/// Every player folder a roster slot maps, in roster order: slot order, a folder several
/// referee slots map once per slot.
fn players_in_roster_order(export: &ValidatedAestheticsExport) -> Vec<&PlayerFolder> {
    let indices: Vec<usize> = match &export.roster {
        ValidatedRoster::Team(slots) => slots.values().map(|index| index.0).collect(),
        ValidatedRoster::Referees(slots) => slots.values().map(|index| index.0).collect(),
    };
    indices
        .into_iter()
        .map(|index| &export.players[index])
        .collect()
}

/// Whether `player`, a player folder of `export`, links the shared folder of `kind` whose
/// folded name is `name_key` so that the shared output is loaded as it is: a link that does
/// not feed the player's own package (`link_feeds_own_package`, the answer planning gives).
fn links_plainly(
    export: &ValidatedAestheticsExport,
    player: &PlayerFolder,
    engine: Engine,
    kind: SharedKind,
    name_key: &str,
    hand_weighted: &BTreeSet<ScopePath>,
) -> bool {
    player.links.iter().any(|link| {
        link.kind == kind
            && vtree::fold_name(&link.name) == name_key
            && !link_feeds_own_package(export, engine, player, link, hand_weighted)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{resolved, resolved_with_issues};

    fn slot(number: u8) -> PlayerSlot {
        PlayerSlot::new(number).unwrap()
    }

    fn team(id: u16) -> TeamId {
        TeamId::new(id).unwrap()
    }

    #[test]
    fn each_slot_gets_its_team_block_s_id_in_slot_order() {
        let co = PlannedModelIds::for_team(team(714));
        assert_eq!(co.exclusive(slot(5)), 625);
        assert_eq!(co.exclusive(slot(23)), 643);
        assert_eq!(
            PlannedModelIds::for_team(team(792)).exclusive(slot(5)),
            3745
        );
        assert_eq!(PlannedModelIds::for_team(team(701)).exclusive(slot(1)), 101);
        assert_eq!(
            PlannedModelIds::for_team(team(TeamId::MAX)).exclusive(slot(23)),
            8883,
            "the last team's last exclusive ID stays below the referee band"
        );
    }

    // The blocks sit above the stock band the hide IDs live in
    // (`fpc::HideIds::BOOTS` ends at 100): an allocated ID can never be
    // read as a hide ID, and a hide ID never as an allocated one.
    #[test]
    fn the_blocks_start_above_the_hide_id_band() {
        assert!(FIRST_BLOCK > *fpc::HideIds::BOOTS.end());
        assert!(FIRST_BLOCK > *fpc::HideIds::GLOVES.end());
    }

    #[test]
    fn the_shared_ids_follow_the_exclusive_ones_and_stop_at_seventeen() {
        let co = PlannedModelIds::for_team(team(714));
        assert_eq!(co.shared(0), Some(644));
        assert_eq!(co.shared(16), Some(660));
        assert_eq!(co.shared(17), None);
        assert_eq!(SHARED_COUNT, 17);
        assert_eq!(
            PlannedModelIds::for_team(team(TeamId::MAX)).shared(16),
            Some(8900),
            "the last team's block ends at 8900"
        );
    }

    /// The folder names of `shared_folders_taking_ids` over the export `co Midcup Shared`.
    fn taking(
        files: &[&str],
        players_txt: Option<&[u8]>,
        engine: Engine,
        kind: SharedKind,
    ) -> Vec<String> {
        let files: Vec<(&str, u64)> = files.iter().map(|path| (*path, 1)).collect();
        let export = resolved("co Midcup Shared", &files, &[], players_txt).export;
        shared_folders_taking_ids(&export, engine, kind, &BTreeSet::new())
            .iter()
            .map(|folder| folder.folder_name.clone())
            .collect()
    }

    #[test]
    fn plainly_linked_folders_take_ids_in_case_folded_name_order() {
        let files = [
            "Players/03 - A/Zebra.boots",
            "Players/07 - B/apple.boots",
            "Players/11 - C/Mango.boots",
            "Players/11 - C/Grip.gloves",
            "Boots/Zebra/boots.fmdl",
            "Boots/apple/boots.fmdl",
            "Boots/Mango/boots.fmdl",
            "Gloves/Grip/glove_l.fmdl",
        ];
        assert_eq!(
            taking(&files, None, Engine::Fox, SharedKind::Boots),
            ["apple", "Mango", "Zebra"]
        );
        assert_eq!(
            taking(&files, None, Engine::Fox, SharedKind::Gloves),
            ["Grip"]
        );
        assert_eq!(
            taking(&files, None, Engine::Fox, SharedKind::Face),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_folder_linked_by_a_combining_player_takes_no_id() {
        // Slot 03 holds boots of its own, so on Fox its link combines; slot 07's gloves do
        // not touch its boots link.
        let files = [
            "Players/03 - A/Crocs.boots",
            "Players/03 - A/kit_boots.fmdl",
            "Players/07 - B/Mud.boots",
            "Players/07 - B/glove_l.fmdl",
            "Boots/Crocs/boots.fmdl",
            "Boots/Mud/boots.fmdl",
        ];
        assert_eq!(
            taking(&files, None, Engine::Fox, SharedKind::Boots),
            ["Mud"]
        );
    }

    #[test]
    fn pre_fox_a_boots_link_combines_and_takes_no_id_only_under_ingame_face() {
        // Without the marker slot 03's boots model is a part of his face, and his link loads
        // the shared output by its id.
        let files = [
            "Players/03 - A/Crocs.boots",
            "Players/03 - A/kit_boots.model",
            "Players/07 - B/Mud.boots",
            "Players/07 - B/glove_l.model",
            "Boots/Crocs/boots.model",
            "Boots/Mud/boots.model",
        ];
        assert_eq!(
            taking(&files, None, Engine::PreFox, SharedKind::Boots),
            ["Crocs", "Mud"]
        );
        // Under it his boots model is a part of his own boots, which Crocs's model joins.
        let marked = [files.as_slice(), &["Players/03 - A/ingame_face.toml"]].concat();
        assert_eq!(
            taking(&marked, None, Engine::PreFox, SharedKind::Boots),
            ["Mud"]
        );
    }

    #[test]
    fn a_shared_folder_holding_a_model_link_of_its_kind_holds_a_model_on_fox_only() {
        let files = [
            ("Players/05 - A/Crocs.boots", 0),
            ("Boots/Crocs/boots.fmdl.common", 0),
            ("Common/boots.fmdl", 1),
        ];
        let export = resolved("co Midcup Shared", &files, &[], None).export;
        let crocs = &export.boots[0];
        assert!(holds_model_of_its_kind(
            crocs,
            Engine::Fox,
            SharedKind::Boots
        ));
        assert!(!holds_model_of_its_kind(
            crocs,
            Engine::PreFox,
            SharedKind::Boots
        ));
    }

    #[test]
    fn a_plainly_linked_folder_holding_no_model_takes_no_id() {
        // Studs holds textures alone, so Zebra, sorted after it, takes the block's first
        // shared id.
        for (engine, model) in [
            (Engine::Fox, "Boots/Zebra/boots.fmdl"),
            (Engine::PreFox, "Boots/Zebra/boots.model"),
        ] {
            let files = [
                "Players/05 - A/Studs.boots",
                "Players/06 - B/Zebra.boots",
                "Boots/Studs/studs.dds",
                model,
            ];
            assert_eq!(
                taking(&files, None, engine, SharedKind::Boots),
                ["Zebra"],
                "{engine:?}"
            );
        }
        // Linked beside slot 05's own boots, Combi is a texture source of his boots: not in
        // the list, with or without a model.
        let files = [
            "Players/05 - A/Combi.boots",
            "Players/05 - A/boots.fmdl",
            "Boots/Combi/combi.dds",
        ];
        assert_eq!(
            taking(&files, None, Engine::Fox, SharedKind::Boots),
            Vec::<String>::new()
        );
    }

    /// `folder_ids` over the export `co Midcup Shared` for `engine`, `record` a `Midcup`'s
    /// installed one when given: `own` as (folder name, the wearer's slot number) pairs,
    /// `shared` as folder names.
    fn own_and_shared(
        files: &[&str],
        players_txt: Option<&[u8]>,
        record: Option<&SharedIds>,
    ) -> (Vec<(String, u8)>, Vec<String>) {
        let files: Vec<(&str, u64)> = files.iter().map(|path| (*path, 1)).collect();
        let export = resolved("co Midcup Shared", &files, &[], players_txt).export;
        let ids = folder_ids(
            &export,
            Engine::Fox,
            SharedKind::Boots,
            &BTreeSet::new(),
            record,
        );
        (
            ids.own
                .iter()
                .map(|(folder, slot)| (folder.folder_name.clone(), slot.get()))
                .collect(),
            ids.shared
                .iter()
                .map(|folder| folder.folder_name.clone())
                .collect(),
        )
    }

    #[test]
    fn a_folder_one_slot_links_plainly_takes_that_slot_s_exclusive_id() {
        let files = ["Players/05 - A/Crocs.boots", "Boots/Crocs/boots.fmdl"];
        assert_eq!(
            own_and_shared(&files, None, None),
            (vec![("Crocs".to_owned(), 5)], Vec::new())
        );
    }

    #[test]
    fn a_folder_two_slots_link_plainly_takes_a_shared_id() {
        let files = [
            "Players/05 - A/Crocs.boots",
            "Players/07 - B/Crocs.boots",
            "Boots/Crocs/boots.fmdl",
        ];
        assert_eq!(
            own_and_shared(&files, None, None),
            (Vec::new(), vec!["Crocs".to_owned()])
        );
    }

    #[test]
    fn a_folder_one_player_folder_two_slots_link_takes_a_shared_id() {
        // `players.txt` maps folder A to slots 06 and 08: Crocs has two wearers, not one.
        let files = ["Players/A/Crocs.boots", "Boots/Crocs/boots.fmdl"];
        assert_eq!(
            own_and_shared(&files, Some(b"06 A\n08 A\n"), None),
            (Vec::new(), vec!["Crocs".to_owned()])
        );
    }

    #[test]
    fn a_folder_the_record_names_takes_a_shared_id_with_one_wearer() {
        // The record names `crocs`, folded: the main CPK's other wearers still wear it.
        let files = ["Players/05 - A/Crocs.boots", "Boots/Crocs/boots.fmdl"];
        let record = SharedIds {
            boots: BTreeMap::from([(645, "CROCS".to_owned())]),
            gloves: BTreeMap::new(),
        };
        assert_eq!(
            own_and_shared(&files, None, Some(&record)),
            (Vec::new(), vec!["Crocs".to_owned()])
        );
    }

    #[test]
    fn a_combining_slot_is_no_wearer_and_the_folder_takes_the_plain_linker_s_id() {
        // Slot 05's own boots make his link combine; slot 07, Crocs's only plain wearer,
        // lends it his exclusive id.
        let files = [
            "Players/05 - A/Crocs.boots",
            "Players/05 - A/kit_boots.fmdl",
            "Players/07 - B/Crocs.boots",
            "Boots/Crocs/boots.fmdl",
        ];
        assert_eq!(
            own_and_shared(&files, None, None),
            (vec![("Crocs".to_owned(), 7)], Vec::new())
        );
    }

    /// A shared folder `Boots/<name>` as `assigned_shared_ids` sees it.
    fn shared_folder(name: &str) -> SharedModelFolder {
        SharedModelFolder {
            path: ScopePath::new(&format!("Boots/{name}")).unwrap(),
            folder_name: name.to_owned(),
            files: Vec::new(),
        }
    }

    /// `assigned_shared_ids` over `folders` for team 714's boots block, `record` its
    /// installed one when given.
    fn assignment(
        folders: &[&SharedModelFolder],
        record: Option<&SharedIds>,
    ) -> Result<Vec<u16>, usize> {
        assigned_shared_ids(
            PlannedModelIds::for_team(team(714)),
            SharedKind::Boots,
            folders,
            record,
        )
    }

    #[test]
    fn a_folder_the_record_names_keeps_its_id_compared_case_folded() {
        let crocs = shared_folder("crocs");
        let new = shared_folder("New");
        let record = SharedIds {
            boots: BTreeMap::from([(645, "Crocs".to_owned())]),
            gloves: BTreeMap::new(),
        };
        assert_eq!(
            assignment(&[&crocs, &new], Some(&record)),
            Ok(vec![645, 644])
        );
    }

    #[test]
    fn new_folders_take_the_lowest_free_ids_alphabetically_around_a_recorded_one() {
        let alpha = shared_folder("Alpha");
        let bravo = shared_folder("Bravo");
        let mango = shared_folder("Mango");
        let record = SharedIds {
            boots: BTreeMap::from([(645, "Mango".to_owned())]),
            gloves: BTreeMap::new(),
        };
        // Alpha and Bravo sort before and beside Mango: the first free ID, then the next.
        assert_eq!(
            assignment(&[&alpha, &bravo, &mango], Some(&record)),
            Ok(vec![644, 646, 645])
        );
    }

    #[test]
    fn the_pool_is_exhausted_when_the_new_folders_outnumber_the_free_ids() {
        let record = SharedIds {
            boots: (0..16u16)
                .map(|index| (644 + index, format!("R{index:02}")))
                .collect(),
            gloves: BTreeMap::new(),
        };
        let alpha = shared_folder("Alpha");
        let bravo = shared_folder("Bravo");
        // 16 recorded entries and 2 new folders need 18 of the block's 17 shared IDs.
        assert_eq!(assignment(&[&alpha, &bravo], Some(&record)), Err(18));
        // One new folder: the last free ID serves it.
        assert_eq!(assignment(&[&alpha], Some(&record)), Ok(vec![660]));
        // And with no record the folders are numbered from 644, the same 16.
        let ids: Vec<u16> = (644..660).collect();
        let folders: Vec<SharedModelFolder> = record
            .boots
            .values()
            .map(|name| shared_folder(name))
            .collect();
        let refs: Vec<&SharedModelFolder> = folders.iter().collect();
        assert_eq!(assignment(&refs, None), Ok(ids));
    }

    #[test]
    fn a_recorded_id_outside_the_block_takes_none_of_its_shared_ids() {
        // 700 is no ID of team 714's block (644 to 660): all 17 stay free for new folders.
        let record = SharedIds {
            boots: BTreeMap::from([(700, "Stray".to_owned())]),
            gloves: BTreeMap::new(),
        };
        let folders: Vec<SharedModelFolder> = (0..17)
            .map(|index| shared_folder(&format!("F{index:02}")))
            .collect();
        let refs: Vec<&SharedModelFolder> = folders.iter().collect();
        assert_eq!(
            assignment(&refs, Some(&record)),
            Ok((644..=660).collect::<Vec<u16>>())
        );
    }

    #[test]
    fn a_referee_s_plain_link_takes_no_shared_id() {
        let export = resolved(
            "refs Cup",
            &[
                ("Players/Ref A/Studs.boots", 0),
                ("Boots/Studs/boots.fmdl", 1),
            ],
            &[],
            Some(b"01 Ref A\n"),
        )
        .export;
        for engine in [Engine::Fox, Engine::PreFox] {
            assert!(
                shared_folders_taking_ids(&export, engine, SharedKind::Boots, &BTreeSet::new())
                    .is_empty(),
                "{engine:?}"
            );
        }
    }

    #[test]
    fn a_folder_only_an_unmapped_folder_links_takes_no_id() {
        let (export, issues) = resolved_with_issues(
            "co Midcup Shared",
            &[
                ("Players/A/Crocs.boots", 1),
                ("Players/Unlisted/Mud.boots", 1),
                ("Boots/Crocs/boots.fmdl", 1),
                ("Boots/Mud/boots.fmdl", 1),
            ],
            &[],
            Some(b"03 A\n"),
        );
        assert_eq!(issues, ["player_unlisted", "shared_folder_orphaned"]);
        let names: Vec<&str> = shared_folders_taking_ids(
            &export.export,
            Engine::Fox,
            SharedKind::Boots,
            &BTreeSet::new(),
        )
        .iter()
        .map(|folder| folder.folder_name.as_str())
        .collect();
        assert_eq!(names, ["Crocs"]);
    }
}
