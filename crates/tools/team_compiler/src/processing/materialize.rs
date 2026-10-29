//! The materialize step (`spec/team_compiler/pipeline.md` "5. Writer", step 5): the one place a
//! task's output becomes the entries the writer adds. In normal and sideload mode an entry goes
//! at its game path and a model package is packed per id, into an `.fpk` on Fox and on pre-Fox
//! into a face CPK nested in the output, the boots and gloves as loose files of their folder;
//! in test mode every entry goes under its export's
//! source and the folder the task works on, a package unpacked.

use std::collections::BTreeMap;
use std::io::Cursor;

use cpk::CpkWriter;
use fpk::{FpkFile, FpkKind};
use pes_version::Engine;
use studio_core::{ExportId, TOOL_VERSION};

use super::Entry;
use crate::paths::{self, PackageKey};
use crate::plan::roles::ModelPackage;
use crate::plan::{BuildTask, TaskKind};

/// What goes before each bin's game path in test mode's tree: the bins sit in a folder of
/// their own, beside each export's.
pub(crate) const TEST_BINS_PREFIX: &str = "_bins/";

/// A model package's files by their names in the package (`fcl_hair.fmdl`, `face_diff.bin`),
/// in name order.
pub(crate) type PackageFiles = BTreeMap<String, Vec<u8>>;

/// What a task made, before `materialize` places it.
pub(crate) enum TaskOutput {
    /// Files at their game paths: a folder's textures, the Common textures, a portrait, a kit,
    /// the logo.
    Entries(Vec<Entry>),
    /// A model package's files, built once and emitted under each of `ids`.
    Package {
        /// Which package: its file stem and the game folder its keys name.
        package: ModelPackage,
        /// The keys the package is emitted under, in slot order.
        ids: Vec<PackageKey>,
        /// Its files.
        files: PackageFiles,
    },
}

/// Where a run's entries go, fixed once for the run.
pub(crate) enum EntryTarget {
    /// Normal and sideload mode: each entry at its game path, a package as one `.fpk` and an
    /// empty `.fpkd` per id on Fox, on pre-Fox a face as one face CPK per id and the boots
    /// and gloves as loose files of each id's folder.
    GamePaths {
        /// The target's engine, which decides how a package is packed.
        engine: Engine,
    },
    /// Test mode: each entry at `<source>/<folder>/<name>` (`test_entry_folder`), a package's
    /// files once each by their names in it.
    TestOutput {
        /// Each export's source as discovered, by export: its folder name, or its archive's
        /// file name with the extension, so a folder and an archive of one stem stay apart.
        sources: BTreeMap<ExportId, String>,
    },
}

/// The entries the writer adds for `output`, the output of `task`, as `target` places them.
pub(crate) fn materialize(
    output: TaskOutput,
    task: &BuildTask,
    target: &EntryTarget,
) -> Vec<Entry> {
    match target {
        EntryTarget::GamePaths { engine } => match output {
            TaskOutput::Entries(entries) => entries,
            TaskOutput::Package {
                package,
                ids,
                files,
            } => match engine {
                Engine::Fox => packed(package, &ids, files),
                Engine::PreFox => match package {
                    ModelPackage::Face => pre_fox_faces(&ids, &files),
                    ModelPackage::Boots | ModelPackage::Gloves => loose(package, &ids, files),
                },
            },
        },
        EntryTarget::TestOutput { sources } => {
            let source = sources
                .get(&task.export_id)
                .expect("every task's export is among the run's sources");
            let folder = test_entry_folder(source, &task.kind);
            match output {
                TaskOutput::Entries(entries) => entries
                    .into_iter()
                    .map(|(path, bytes)| {
                        let name = test_name(task, &path);
                        (format!("{folder}{name}"), bytes)
                    })
                    .collect(),
                TaskOutput::Package { files, .. } => files
                    .into_iter()
                    .map(|(name, bytes)| (format!("{folder}{name}"), bytes))
                    .collect(),
            }
        }
    }
}

/// The folder, `/`-terminated, a test-mode entry of a task of `kind` goes in under `source`:
/// the export folder the task works on, or for a portrait, the logo, the referees' marker and
/// the collar, whose task names a file, that file's folder (none for a file at the export's
/// root).
fn test_entry_folder(source: &str, kind: &TaskKind) -> String {
    let folder = match kind {
        TaskKind::Portrait { .. }
        | TaskKind::Logo { .. }
        | TaskKind::RefereeMarker { .. }
        | TaskKind::Collar { .. } => kind.folder_path().parent(),
        TaskKind::Models { .. }
        | TaskKind::Textures { .. }
        | TaskKind::CommonTextures { .. }
        | TaskKind::CommonModels { .. }
        | TaskKind::Kit { .. } => Some(kind.folder_path()),
    };
    match folder {
        Some(folder) => format!("{source}/{}/", folder.as_str()),
        None => format!("{source}/"),
    }
}

/// The name a test-mode entry of `task` at `path` (its game path) takes below its folder:
/// for a texture task the texture's path below it with its extension
/// (`kit1/armor_bsm.ftex`, `jessie/hair.ftex`), two textures of one name in different
/// subfolders staying two files; every other entry's file name.
fn test_name(task: &BuildTask, path: &str) -> String {
    let below = match &task.kind {
        TaskKind::Textures { folder, .. } => {
            folder
                .textures
                .texture_below(folder.engine, task.team_id, path)
        }
        TaskKind::CommonTextures { folder, .. } => paths::common_texture_below(task.team_id, path)
            .map(|below| {
                let subpath = paths::common_subpath(folder);
                below
                    .strip_prefix(subpath.as_str())
                    .unwrap_or(&below)
                    .to_owned()
            }),
        TaskKind::Models { .. }
        | TaskKind::CommonModels { .. }
        | TaskKind::Portrait { .. }
        | TaskKind::Kit { .. }
        | TaskKind::Logo { .. }
        | TaskKind::RefereeMarker { .. }
        | TaskKind::Collar { .. } => None,
    };
    below.unwrap_or_else(|| {
        path.rsplit_once('/')
            .map_or(path, |(_, name)| name)
            .to_owned()
    })
}

/// `files` packed as `package`'s `.fpk`, with an empty `.fpkd`, under each of `ids`.
fn packed(package: ModelPackage, ids: &[PackageKey], files: PackageFiles) -> Vec<Entry> {
    let mut fpk = FpkFile::new(FpkKind::Fpk);
    for (name, bytes) in files {
        fpk.insert(name, bytes);
    }
    let fpk = fpk.write();
    // The game opens a package's `.fpkd` beside its `.fpk`; with the textures in the common
    // folder there is nothing to put in it, so it is an empty package.
    let empty = FpkFile::new(FpkKind::Fpkd).write();
    let stem = package.file_stem();
    let mut entries = Vec::new();
    // The game finds a package by its id, so each slot gets its own copy; the last slot takes
    // the buffer itself rather than one more copy.
    if let Some((last_id, other_ids)) = ids.split_last() {
        for id in other_ids {
            let folder = paths::package_folder(package, *id);
            entries.push((format!("{folder}/{stem}.fpk"), fpk.clone()));
            entries.push((format!("{folder}/{stem}.fpkd"), empty.clone()));
        }
        let folder = paths::package_folder(package, *last_id);
        entries.push((format!("{folder}/{stem}.fpk"), fpk));
        entries.push((format!("{folder}/{stem}.fpkd"), empty));
    }
    entries
}

/// `files`, a pre-Fox face's, packed as one face CPK under each of `ids`
/// (`paths::pre_fox_package_folder`). Each CPK holds its own copy: its entries' paths repeat
/// its own path, which carries the id.
fn pre_fox_faces(ids: &[PackageKey], files: &PackageFiles) -> Vec<Entry> {
    ids.iter()
        .map(|id| {
            let face = paths::pre_fox_package_folder(ModelPackage::Face, *id);
            (format!("{face}.cpk"), face_cpk(&face, files))
        })
        .collect()
}

/// `files`, a pre-Fox boots or gloves `package`'s, as loose files of its folder under each of
/// `ids` (`paths::pre_fox_package_folder`), the shape of the game's own boots and glove
/// folders.
fn loose(package: ModelPackage, ids: &[PackageKey], files: PackageFiles) -> Vec<Entry> {
    let folders: Vec<String> = ids
        .iter()
        .map(|id| paths::pre_fox_package_folder(package, *id))
        .collect();
    let mut entries = Vec::new();
    for (name, bytes) in files {
        // Each id gets its own copy; the last takes the buffer itself rather than one more
        // copy.
        if let Some((last, others)) = folders.split_last() {
            for folder in others {
                entries.push((format!("{folder}/{name}"), bytes.clone()));
            }
            entries.push((format!("{last}/{name}"), bytes));
        }
    }
    entries
}

/// The face CPK holding `files`, each at `<face>/<name>`, with no timestamps.
fn face_cpk(face: &str, files: &PackageFiles) -> Vec<u8> {
    // Written to memory under one folder's unique names, none holding a control character
    // (an export path cannot), the CPK cannot fail to write.
    const WRITTEN: &str = "a face CPK is written to memory under unique, valid paths";
    let mut cpk = CpkWriter::new(Cursor::new(Vec::new()), TOOL_VERSION).expect(WRITTEN);
    for (name, bytes) in files {
        cpk.add(&format!("{face}/{name}"), bytes, None)
            .expect(WRITTEN);
    }
    cpk.finish().expect(WRITTEN).into_inner()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use aesthetics_export::{ExportCoverage, FileDescriptor, KitFolder, LogoFile, LogoFiles};
    use kit_config::KitSlot;
    use pes_version::PesVersion;
    use vtree::ScopePath;

    use super::*;
    use crate::paths::TextureHome;
    use crate::plan::{EffectiveTeamKitFpc, ModelFolder, TeamKitEdits};

    const SOURCE: &str = "egg Midcup Tracer";
    const PLAYER: &str = "Players/05 - The Chad Stormworks Player";

    /// Test mode's target for export 4, the tracer.
    fn test_output() -> EntryTarget {
        EntryTarget::TestOutput {
            sources: BTreeMap::from([(ExportId(4), SOURCE.to_owned())]),
        }
    }

    /// A task of export 4, team 792, compiling `kind`.
    fn task(kind: TaskKind) -> BuildTask {
        BuildTask {
            export_id: ExportId(4),
            team_id: 792,
            kind,
            charge: 0,
            group: None,
        }
    }

    /// The export file at `path`, size unknown.
    fn file(path: &str) -> FileDescriptor {
        let path = ScopePath::new(path).unwrap();
        FileDescriptor {
            size: 0,
            kind: aesthetics_export::classify(path.name()),
            source: path.clone(),
            path,
        }
    }

    /// The model folder at `path`, its textures going to `textures`; its files do not matter
    /// here.
    fn folder(path: &str, textures: TextureHome) -> ModelFolder {
        ModelFolder {
            path: ScopePath::new(path).unwrap(),
            files: Vec::new(),
            ingame_face: false,
            combined: Vec::new(),
            common_models: Vec::new(),
            common_texture_stems: BTreeSet::new(),
            common_files: Vec::new(),
            hand_split: BTreeSet::new(),
            environment_map: false,
            refkit_body: false,
            textures,
            engine: Engine::Fox,
        }
    }

    fn player() -> ModelFolder {
        folder(
            PLAYER,
            TextureHome::PlayerCommon {
                folder_name: "05 - The Chad Stormworks Player".to_owned(),
            },
        )
    }

    /// Each entry `bytes` named after its game path's file name, at its game path.
    fn entries(game_paths: &[&str]) -> Vec<Entry> {
        game_paths
            .iter()
            .map(|path| ((*path).to_owned(), path.as_bytes().to_vec()))
            .collect()
    }

    /// The paths `materialize` gives `game_paths`, `task`'s entries, under `target`; asserts
    /// every entry keeps its bytes.
    fn placed(task: &BuildTask, game_paths: &[&str], target: &EntryTarget) -> Vec<String> {
        let output = TaskOutput::Entries(entries(game_paths));
        let placed = materialize(output, task, target);
        let bytes: Vec<&[u8]> = placed.iter().map(|(_, bytes)| bytes.as_slice()).collect();
        let expected: Vec<&[u8]> = game_paths.iter().map(|path| path.as_bytes()).collect();
        assert_eq!(bytes, expected);
        placed.into_iter().map(|(path, _)| path).collect()
    }

    /// Asserts `game_paths`, a task of `kind`'s entries, go to `test_paths` in test mode and
    /// stay at their game paths in normal mode.
    fn assert_placed(kind: TaskKind, game_paths: &[&str], test_paths: &[&str]) {
        let task = task(kind);
        assert_eq!(placed(&task, game_paths, &test_output()), test_paths);
        assert_eq!(placed(&task, game_paths, &fox()), game_paths);
    }

    /// Normal mode on a Fox target.
    fn fox() -> EntryTarget {
        EntryTarget::GamePaths {
            engine: Engine::Fox,
        }
    }

    /// The tracer's face package: two files.
    fn face_files() -> PackageFiles {
        BTreeMap::from([
            ("face_diff.bin".to_owned(), b"diff".to_vec()),
            ("fcl_hair.fmdl".to_owned(), b"hair".to_vec()),
        ])
    }

    fn package(package: ModelPackage, ids: &[u32], files: PackageFiles) -> TaskOutput {
        TaskOutput::Package {
            package,
            ids: ids.iter().copied().map(PackageKey::Id).collect(),
            files,
        }
    }

    /// Asserts `bytes` is an `.fpk` holding exactly `files`.
    fn assert_fpk(bytes: &[u8], files: &PackageFiles) {
        let fpk = FpkFile::read(bytes).unwrap();
        assert_eq!(fpk.kind(), FpkKind::Fpk);
        let read: PackageFiles = fpk
            .entries()
            .map(|(name, bytes)| (name.to_owned(), bytes.to_vec()))
            .collect();
        assert_eq!(&read, files);
    }

    /// Asserts `bytes` is an empty `.fpkd`.
    fn assert_empty_fpkd(bytes: &[u8]) {
        let fpkd = FpkFile::read(bytes).unwrap();
        assert_eq!(fpkd.kind(), FpkKind::Fpkd);
        assert!(fpkd.is_empty());
    }

    #[test]
    fn a_player_folder_s_package_is_unpacked_under_its_folder_in_test_mode() {
        let kind = TaskKind::Models {
            folder: player(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        };
        let output = package(ModelPackage::Face, &[79205], face_files());

        let written = materialize(output, &task(kind), &test_output());

        assert_eq!(
            written,
            [
                (format!("{SOURCE}/{PLAYER}/face_diff.bin"), b"diff".to_vec()),
                (format!("{SOURCE}/{PLAYER}/fcl_hair.fmdl"), b"hair".to_vec()),
            ]
        );
    }

    #[test]
    fn a_player_folder_s_package_is_packed_at_its_game_path_in_normal_mode() {
        let kind = TaskKind::Models {
            folder: player(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205)],
            kits: Vec::new(),
        };
        let output = package(ModelPackage::Face, &[79205], face_files());

        let written = materialize(output, &task(kind), &fox());

        let paths: Vec<&str> = written.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "Asset/model/character/face/real/79205/#Win/face.fpk",
                "Asset/model/character/face/real/79205/#Win/face.fpkd",
            ]
        );
        assert_fpk(&written[0].1, &face_files());
        assert_empty_fpkd(&written[1].1);
    }

    #[test]
    fn a_package_of_two_ids_is_emitted_once_in_test_mode_and_per_id_in_normal_mode() {
        let two_ids = task(TaskKind::Models {
            folder: player(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205), PackageKey::Id(79206)],
            kits: Vec::new(),
        });
        let output = || package(ModelPackage::Face, &[79205, 79206], face_files());

        let tested = materialize(output(), &two_ids, &test_output());
        let paths: Vec<&str> = tested.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "egg Midcup Tracer/Players/05 - The Chad Stormworks Player/face_diff.bin",
                "egg Midcup Tracer/Players/05 - The Chad Stormworks Player/fcl_hair.fmdl",
            ]
        );

        let normal = materialize(output(), &two_ids, &fox());
        let paths: Vec<&str> = normal.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "Asset/model/character/face/real/79205/#Win/face.fpk",
                "Asset/model/character/face/real/79205/#Win/face.fpkd",
                "Asset/model/character/face/real/79206/#Win/face.fpk",
                "Asset/model/character/face/real/79206/#Win/face.fpkd",
            ]
        );
        assert_fpk(&normal[0].1, &face_files());
        assert_eq!(normal[0].1, normal[2].1, "each id gets the same package");
        assert_empty_fpkd(&normal[1].1);
        assert_empty_fpkd(&normal[3].1);
    }

    #[test]
    fn a_pre_fox_face_is_one_nested_cpk_per_id_its_entries_repeating_its_path() {
        let two_ids = task(TaskKind::Models {
            folder: player(),
            package: ModelPackage::Face,
            ids: vec![PackageKey::Id(79205), PackageKey::Id(79206)],
            kits: Vec::new(),
        });
        let files = BTreeMap::from([
            ("face.xml".to_owned(), b"xml".to_vec()),
            ("oral_face_high_win32.model".to_owned(), b"model".to_vec()),
        ]);
        let output = package(ModelPackage::Face, &[79205, 79206], files.clone());

        let pre_fox = EntryTarget::GamePaths {
            engine: Engine::PreFox,
        };
        let written = materialize(output, &two_ids, &pre_fox);

        let paths: Vec<&str> = written.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "common/character0/model/character/face/real/79205.cpk",
                "common/character0/model/character/face/real/79206.cpk",
            ]
        );
        for ((_, bytes), id) in written.iter().zip([79205, 79206]) {
            let mut cpk = cpk::CpkArchive::open(Cursor::new(bytes)).unwrap();
            let entries = cpk.entries().to_vec();
            let read: Vec<(String, Vec<u8>)> = entries
                .iter()
                .map(|entry| {
                    assert_eq!(entry.modified, None, "{}", entry.path);
                    (entry.path.clone(), cpk.read(entry).unwrap())
                })
                .collect();
            let folder = format!("common/character0/model/character/face/real/{id}");
            let expected: Vec<(String, Vec<u8>)> = files
                .iter()
                .map(|(name, bytes)| (format!("{folder}/{name}"), bytes.clone()))
                .collect();
            assert_eq!(read, expected);
        }
    }

    #[test]
    fn a_pre_fox_shared_gloves_package_is_loose_files_of_its_folder() {
        let gloves = task(TaskKind::Models {
            folder: folder(
                "Gloves/Keeper",
                TextureHome::SharedOutput {
                    package: ModelPackage::Gloves,
                    id: 644,
                },
            ),
            package: ModelPackage::Gloves,
            ids: vec![PackageKey::Id(644)],
            kits: Vec::new(),
        });
        let files = BTreeMap::from([
            ("glove.xml".to_owned(), b"xml".to_vec()),
            ("glove_l.model".to_owned(), b"model".to_vec()),
        ]);
        let pre_fox = EntryTarget::GamePaths {
            engine: Engine::PreFox,
        };

        let written = materialize(
            package(ModelPackage::Gloves, &[644], files),
            &gloves,
            &pre_fox,
        );

        assert_eq!(
            written,
            [
                (
                    "common/character0/model/character/glove/g0644/glove.xml".to_owned(),
                    b"xml".to_vec()
                ),
                (
                    "common/character0/model/character/glove/g0644/glove_l.model".to_owned(),
                    b"model".to_vec()
                ),
            ]
        );
    }

    #[test]
    fn a_shared_folder_s_package_goes_under_its_folder_in_test_mode() {
        let shared = folder(
            "Boots/Crocs",
            TextureHome::SharedOutput {
                package: ModelPackage::Boots,
                id: 644,
            },
        );
        let boots = task(TaskKind::Models {
            folder: shared,
            package: ModelPackage::Boots,
            ids: vec![PackageKey::Id(644)],
            kits: Vec::new(),
        });
        let files = BTreeMap::from([
            ("boots.fmdl".to_owned(), b"boots".to_vec()),
            ("boots.skl".to_owned(), b"skeleton".to_vec()),
        ]);

        let tested = materialize(
            package(ModelPackage::Boots, &[644], files.clone()),
            &boots,
            &test_output(),
        );
        let normal = materialize(
            package(ModelPackage::Boots, &[644], files.clone()),
            &boots,
            &fox(),
        );

        assert_eq!(
            tested,
            [
                (
                    "egg Midcup Tracer/Boots/Crocs/boots.fmdl".to_owned(),
                    b"boots".to_vec()
                ),
                (
                    "egg Midcup Tracer/Boots/Crocs/boots.skl".to_owned(),
                    b"skeleton".to_vec()
                ),
            ]
        );
        let paths: Vec<&str> = normal.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "Asset/model/character/boots/k0644/#Win/boots.fpk",
                "Asset/model/character/boots/k0644/#Win/boots.fpkd",
            ]
        );
        assert_fpk(&normal[0].1, &files);
    }

    #[test]
    fn a_player_folder_s_textures_go_under_its_folder_in_test_mode() {
        let kind = TaskKind::Textures {
            folder: player(),
            kits: Vec::new(),
        };
        assert_placed(
            kind,
            &[
                "Asset/model/character/common/792/05 - The Chad Stormworks Player/sourceimages/#windx11/shirt.ftex",
                "Asset/model/character/common/792/05 - The Chad Stormworks Player/jessie/sourceimages/#windx11/skin.ftex",
            ],
            &[
                "egg Midcup Tracer/Players/05 - The Chad Stormworks Player/shirt.ftex",
                "egg Midcup Tracer/Players/05 - The Chad Stormworks Player/jessie/skin.ftex",
            ],
        );
    }

    #[test]
    fn the_common_textures_go_under_common_in_test_mode() {
        let kind = TaskKind::CommonTextures {
            folder: ScopePath::new("Common").unwrap(),
            textures: Vec::new(),
            kits: Vec::new(),
            environment_map: false,
        };
        assert_placed(
            kind,
            &[
                "Asset/model/character/common/792/sourceimages/#windx11/hair.ftex",
                "Asset/model/character/common/792/sourceimages/#windx11/skin.ftex",
                "Asset/model/character/common/792/kit1/sourceimages/#windx11/armor_bsm.ftex",
            ],
            &[
                "egg Midcup Tracer/Common/hair.ftex",
                "egg Midcup Tracer/Common/skin.ftex",
                "egg Midcup Tracer/Common/kit1/armor_bsm.ftex",
            ],
        );
    }

    #[test]
    fn a_kit_goes_under_its_kit_folder_in_test_mode() {
        let kind = TaskKind::Kit {
            slot: KitSlot::G1,
            kit: KitFolder {
                path: ScopePath::new("Kits/g1").unwrap(),
                label: None,
                config: None,
                colors: None,
                icon: None,
                layout: None,
                textures: Vec::new(),
            },
            edits: TeamKitEdits {
                fpc: EffectiveTeamKitFpc::Unknown,
                collar: None,
            },
            coverage: ExportCoverage::Full,
        };
        assert_placed(
            kind,
            &[
                "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex",
                "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
            ],
            &[
                "egg Midcup Tracer/Kits/g1/u0792g1.ftex",
                "egg Midcup Tracer/Kits/g1/792_DEF_GK1st_realUni.bin",
            ],
        );
    }

    #[test]
    fn a_portrait_goes_in_its_file_s_folder_in_test_mode() {
        let kind = TaskKind::Portrait {
            player_id: 79205,
            file: file(&format!("{PLAYER}/portrait.dds")),
        };
        assert_placed(
            kind,
            &[pes_savefile::player_tables::portrait_path(PesVersion::Pes21, 79205).as_str()],
            &["egg Midcup Tracer/Players/05 - The Chad Stormworks Player/79205.dds"],
        );
    }

    #[test]
    fn a_root_logo_goes_at_the_source_s_root_in_test_mode() {
        let kind = TaskKind::Logo {
            logo: LogoFiles {
                main: LogoFile {
                    file: file("logo.png"),
                    fit: None,
                },
                small: None,
            },
        };
        let game_paths = paths::logo(PesVersion::Pes21, 792);
        let game_paths: Vec<&str> = game_paths.iter().map(String::as_str).collect();
        assert_placed(
            kind,
            &game_paths,
            &[
                "egg Midcup Tracer/e_000792_r_ll.png",
                "egg Midcup Tracer/e_000792_r_l.png",
                "egg Midcup Tracer/e_000792_r.png",
            ],
        );
    }

    #[test]
    fn a_collar_goes_in_its_file_s_folder_in_test_mode() {
        let kind = TaskKind::Collar {
            file: file("Collars/collar_12.fmdl"),
            id: 12,
        };
        assert_placed(
            kind,
            &[paths::collar(Engine::Fox, 12).as_str()],
            &["egg Midcup Tracer/Collars/collar_012.fmdl"],
        );
    }
}
