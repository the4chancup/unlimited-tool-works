//! `compile --mode test` (`spec/team_compiler/pipeline.md` "5. Writer", step 5): what the compiler
//! made of each export, as loose files under the output folder's `test_output/`, one folder per
//! export, the model packages unpacked and the bins under `_bins/`.

use std::collections::BTreeMap;

use fmdl::FmdlFile;
use fmdl::ops::paths::{TexturePath, texture_paths};
use fpk::FpkFile;

use crate::bins::install_names;
use crate::common::{Run, Sandbox};
use crate::common_links::model_naming_in;
use crate::compile::{cpk_entries, pes21_settings, tracer_kit, tracer_player_file};
use crate::compile_exports::{
    TEAM_COLOR, UNI_COLOR, UNIFORM_PARAMETER, bundled_uni_color, uni_record,
};
use crate::deploy::tracer_copy;
use crate::sideload::slashed;
use crate::snapshot;
use crate::textures::hair_model_naming;

/// The tracer's player folder in `test_output/`: its source's name, then its folder.
const PLAYER: &str = "egg Midcup Tracer/Players/05 - The Chad Stormworks Player";

/// The CPK path of the tracer player's face package.
const FACE_FPK: &str = "Asset/model/character/face/real/79205/#Win/face.fpk";

/// The bins a PES 21 run with no installed CPK writes: a Midcup with nothing recorded
/// writes no shared-IDs record.
const BINS: [&str; 3] = [UNIFORM_PARAMETER, TEAM_COLOR, UNI_COLOR];

/// Every file under the sandbox's `output/test_output/`, by its path there spelled with `/`,
/// with its bytes.
fn test_output(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    snapshot(&sandbox.root.join("output/test_output"))
        .into_iter()
        .map(|(path, bytes)| (slashed(&path), bytes))
        .collect()
}

/// The files of `files` under `_bins/`, by their path below it.
fn bins_of(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    files
        .iter()
        .filter_map(|(path, bytes)| Some((path.strip_prefix("_bins/")?.to_owned(), bytes.clone())))
        .collect()
}

/// The `BINS` entries of the CPK entries `cpk`.
fn bins_in_cpk(cpk: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    BINS.iter()
        .map(|path| ((*path).to_owned(), cpk[*path].clone()))
        .collect()
}

/// `compile --mode test` over `export` in `sandbox` with PES 21 settings.
fn compile_test_mode(sandbox: &Sandbox, export: &str) -> Run {
    sandbox.run(
        &pes21_settings(sandbox),
        &["compile", "--mode", "test", "--export", export],
    )
}

/// A normal compile of `export` in `sandbox`, its `DpFileList.bin` listing the run's CPK: the
/// run, and its CPK's entries.
fn compile_normal(sandbox: &Sandbox, export: &str) -> (Run, BTreeMap<String, Vec<u8>>) {
    install_names(sandbox, &["4cc_99_test.cpk"]);
    let run = sandbox.run(
        &pes21_settings(sandbox),
        &["compile", "--no-deploy", "--export", export],
    );
    let cpk = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    (run, cpk)
}

/// The texture paths of the FMDL `bytes`.
fn hair_textures(bytes: &[u8]) -> Vec<TexturePath> {
    texture_paths(&FmdlFile::read(bytes).unwrap()).unwrap()
}

// TC-OUT-07
#[test]
fn test_mode_writes_each_export_s_files_unpacked_and_the_bins_under_bins() {
    let normal = Sandbox::new("test_mode_normal_twin");
    let (normal_run, cpk) = compile_normal(&normal, &tracer_copy(&normal));

    let sandbox = Sandbox::new("test_mode_tracer");
    install_names(&sandbox, &["4cc_99_test.cpk"]);
    let pes = snapshot(&sandbox.root.join("PES"));

    let run = compile_test_mode(&sandbox, &tracer_copy(&sandbox));

    assert_eq!(run.exit_code(), normal_run.exit_code());
    let written = test_output(&sandbox);
    for name in [
        "fcl_hair.fmdl",
        "shirt.ftex",
        "face_diff.bin",
        "fcl_hair_sim.fclo",
        "fcl_hair_sim.skl",
    ] {
        assert!(
            written.contains_key(&format!("{PLAYER}/{name}")),
            "{name}: {:#?}",
            written.keys()
        );
    }
    // The face package's files are the normal CPK's, unpacked: the hair's texture paths are
    // pointed as there.
    let face = FpkFile::read(&cpk[FACE_FPK]).unwrap();
    assert_eq!(
        hair_textures(&written[&format!("{PLAYER}/fcl_hair.fmdl")]),
        hair_textures(face.get("fcl_hair.fmdl").unwrap())
    );
    for (name, bytes) in face.entries() {
        assert!(
            written.get(&format!("{PLAYER}/{name}")).map(Vec::as_slice) == Some(bytes),
            "{name} differs from the face package's"
        );
    }
    let packed: Vec<&String> = written
        .keys()
        .filter(|path| path.ends_with(".fpk") || path.ends_with(".fpkd"))
        .collect();
    assert_eq!(packed, Vec::<&String>::new());
    let bins = bins_of(&written);
    let expected = bins_in_cpk(&cpk);
    assert_eq!(
        bins.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    assert!(bins == expected, "the bins differ from the normal CPK's");
    assert_eq!(snapshot(&sandbox.root.join("PES")), pes);
    assert!(!sandbox.root.join("output/4cc_99_test.cpk").exists());
    assert!(!sandbox.root.join("output/.staging").exists());
}

// TC-OUT-11
#[test]
fn test_mode_writes_teamnotes_and_the_kit_s_uni_color_entry_under_bins() {
    let tracer_with_notes = |sandbox: &Sandbox| {
        sandbox.copy_tracer("egg Midcup Tracer");
        sandbox.write("exports/egg Midcup Tracer/notes.txt", b"Notes.\n");
        sandbox.complete("exports/egg Midcup Tracer");
        sandbox.arg("exports/egg Midcup Tracer")
    };
    let normal = Sandbox::new("test_mode_notes_normal_twin");
    let export = tracer_with_notes(&normal);
    let (_, cpk) = compile_normal(&normal, &export);
    let sandbox = Sandbox::new("test_mode_notes");
    let export = tracer_with_notes(&sandbox);

    let run = compile_test_mode(&sandbox, &export);

    assert_eq!(run.exit_code(), 0);
    assert_eq!(
        std::fs::read_to_string(sandbox.root.join("output/teamnotes.txt")).unwrap(),
        "--- /egg/ ---\nNotes.\n"
    );
    let bins = bins_of(&test_output(&sandbox));
    let uni_color = &bins[UNI_COLOR];
    assert!(
        uni_record(uni_color, 792) != uni_record(&bundled_uni_color(), 792),
        "the kit's entry is in team 792's record"
    );
    assert!(
        uni_color == &cpk[UNI_COLOR],
        "UniColor.bin is the normal run's"
    );
}

// TC-OUT-17
#[test]
fn test_mode_does_not_apply_the_overrides() {
    let normal = Sandbox::new("test_mode_overrides_normal_twin");
    let (_, cpk) = compile_normal(&normal, &tracer_copy(&normal));
    let sandbox = Sandbox::new("test_mode_overrides");
    sandbox.write(
        &format!("data/overrides/{TEAM_COLOR}"),
        b"the operator's TeamColor.bin",
    );

    let run = compile_test_mode(&sandbox, &tracer_copy(&sandbox));

    assert_eq!(run.exit_code(), 0);
    let written = test_output(&sandbox);
    assert!(
        bins_of(&written)[TEAM_COLOR] == cpk[TEAM_COLOR],
        "TeamColor.bin is the compiled one"
    );
    // Nor is the override anywhere else in the tree.
    let overridden: Vec<&String> = written
        .iter()
        .filter(|(_, bytes)| bytes.as_slice() == b"the operator's TeamColor.bin")
        .map(|(path, _)| path)
        .collect();
    assert_eq!(overridden, Vec::<&String>::new());
    for code in ["overrides_active", "duplicate_path"] {
        assert!(
            run.messages().iter().all(|line| !line.contains(code)),
            "{code}: {:#?}",
            run.messages()
        );
    }
}

// TC-OUT-07, TC-OUT-03
#[test]
fn test_mode_replaces_test_output_whole_and_a_failed_run_leaves_it_as_it_was() {
    let sandbox = Sandbox::new("test_mode_stale");
    sandbox.write("output/test_output/old/x.txt", b"an earlier test run");

    let run = compile_test_mode(&sandbox, &tracer_copy(&sandbox));

    assert_eq!(run.exit_code(), 0);
    let written = test_output(&sandbox);
    assert!(!written.contains_key("old/x.txt"));
    assert!(written.contains_key(&format!("{PLAYER}/fcl_hair.fmdl")));

    let sandbox = Sandbox::new("test_mode_write_failed");
    sandbox.write("output/test_output/old/x.txt", b"an earlier test run");
    // A file where the run's staging folder goes: the loose tree cannot be written in it.
    sandbox.write("output/.staging", b"in the way");
    let before = snapshot(&sandbox.root.join("output"));

    let run = compile_test_mode(&sandbox, &tracer_copy(&sandbox));

    let lines = run.messages();
    let last = lines.last().unwrap();
    let prefix = format!(
        "Fatal cpk_write_failed [AbortRun] (path={}, error=",
        sandbox.display("output/test_output")
    );
    assert!(last.starts_with(&prefix), "{last}");
    assert_eq!(run.exit_code(), 3);
    assert_eq!(snapshot(&sandbox.root.join("output")), before);
}

// TC-CMN-21, TC-OUT-07
#[test]
fn a_common_kit_folder_set_lands_at_its_subfolders_in_test_mode() {
    let sandbox = Sandbox::new("test_mode_kit_folder_set");
    let export = "exports/co Midcup Kits";
    for slot in ["p1", "p2", "p3"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("armor_bsm.dds", "./kitN/"),
    );
    sandbox.write(&format!("{player}/kitN/armor_bsm.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/kit1/armor_bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(
        &format!("{export}/Common/kit2/armor_bsm.dds"),
        &tracer_kit(),
    );
    sandbox.complete(export);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--mode", "test"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let written = test_output(&sandbox);
    for path in [
        "co Midcup Kits/Common/kit1/armor_bsm.ftex",
        "co Midcup Kits/Common/kit2/armor_bsm.ftex",
        "co Midcup Kits/Common/kitN/armor_bsm.ftex",
    ] {
        assert!(written.contains_key(path), "{path}: {:#?}", written.keys());
    }
    assert_eq!(
        written["co Midcup Kits/Common/kitN/armor_bsm.ftex"],
        written["co Midcup Kits/Common/kit1/armor_bsm.ftex"],
        "the fallback, the lowest variant, under `kitN`"
    );
}

// TC-MOD-65, TC-OUT-07
#[test]
fn a_texture_in_a_subfolder_lands_below_its_folder_in_test_mode() {
    let sandbox = Sandbox::new("test_mode_subfolder_texture");
    let export = "exports/co Midcup Hairs";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    // Different images: the root's `hair` the tracer's shirt, `jessie`'s its kit.
    sandbox.write(
        &format!("{player}/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds"), &tracer_kit());
    sandbox.complete(export);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--mode", "test"]);

    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let written = test_output(&sandbox);
    let root = "co Midcup Hairs/Players/05 - A";
    for path in [
        format!("{root}/hair.ftex"),
        format!("{root}/jessie/hair.ftex"),
    ] {
        assert!(written.contains_key(&path), "{path}: {:#?}", written.keys());
    }
}

// TC-OUT-07
#[test]
fn an_archive_export_s_folder_in_test_output_is_named_with_its_extension() {
    let sandbox = Sandbox::new("test_mode_archive");
    sandbox.copy_fixture("egg Midcup Tracer.zip", "exports");

    let run = compile_test_mode(&sandbox, &sandbox.arg("exports/egg Midcup Tracer.zip"));

    assert_eq!(run.exit_code(), 0);
    let written = test_output(&sandbox);
    assert!(
        written.contains_key(
            "egg Midcup Tracer.zip/Players/05 - The Chad Stormworks Player/fcl_hair.fmdl"
        ),
        "{:#?}",
        written.keys()
    );
}
