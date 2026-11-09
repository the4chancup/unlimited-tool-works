//! The `upgrade-models` command line (the `spec/export_upgrader.md` "CLI" scenarios): the input
//! and output refusals, and a run's output folders and report.

use std::fs;

use crate::carried::{fmdl, fmdl_paths, mtl, mtl_text};
use crate::common::{Run, Sandbox, snapshot};
use crate::players::unmigrated_found;

/// `upgrade-models <input> <extra_args>`.
fn upgrade_models(sandbox: &Sandbox, input: &str, extra_args: &[&str]) -> Run {
    let input = sandbox.arg(input);
    let mut args = vec!["upgrade-models", &input];
    args.extend(extra_args);
    sandbox.run(&args)
}

// EU-CLI-04
#[test]
fn a_missing_input_or_a_file_is_refused() {
    let sandbox = Sandbox::new("models_cli_refusals");
    fs::write(sandbox.path("notes.txt"), b"text").unwrap();
    let before = sandbox.listing("");

    upgrade_models(&sandbox, "gone", &[]).assert_refused(2, &["gone", "does not exist"]);
    upgrade_models(&sandbox, "notes.txt", &[]).assert_refused(2, &["notes.txt", "is not a folder"]);

    assert_eq!(sandbox.listing(""), before, "nothing is written");
}

// EU-CLI-05
#[test]
fn an_output_folder_that_exists_or_cannot_be_created_is_refused() {
    let sandbox = Sandbox::new("models_cli_output_exists");
    fs::create_dir_all(sandbox.path("new models/XXX15 - A")).unwrap();
    fs::write(sandbox.path("new models/XXX15 - A/face.fmdl"), b"face").unwrap();
    fs::create_dir_all(sandbox.path("new models upgraded")).unwrap();
    fs::write(sandbox.path("new models upgraded/keep.txt"), b"my edit").unwrap();

    let before = snapshot(&sandbox);
    upgrade_models(&sandbox, "new models", &[]).assert_refused(3, &["new models upgraded"]);
    assert_eq!(snapshot(&sandbox), before, "nothing is written");

    // An output that cannot be created: -o names a path below a file.
    fs::write(sandbox.path("blocked"), b"a file").unwrap();
    let out = sandbox.arg("blocked/out");
    let before = snapshot(&sandbox);
    upgrade_models(&sandbox, "new models", &["-o", &out]).assert_refused(3, &["blocked"]);
    assert_eq!(snapshot(&sandbox), before, "nothing is written");
}

// EU-CLI-06
#[test]
fn the_output_is_a_sibling_folder_holding_one_folder_per_input_folder() {
    let sandbox = Sandbox::new("models_cli_run");
    // Face and boots folders, both Fox-spelled. No step parses the FMDL: a
    // file that does not read as one is copied as it is.
    fs::create_dir_all(sandbox.path("new models/XXX15 - Snuffy")).unwrap();
    fs::write(sandbox.path("new models/XXX15 - Snuffy/face.fmdl"), b"face").unwrap();
    fs::create_dir_all(sandbox.path("new models/k0431")).unwrap();
    fs::write(sandbox.path("new models/k0431/boots.fmdl"), b"boots").unwrap();

    let run = upgrade_models(&sandbox, "new models", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(unmigrated_found(&run), 0, "{}", run.stdout());
    assert_eq!(
        sandbox.listing("new models upgraded"),
        [
            "15 - Snuffy".to_owned(),
            "15 - Snuffy/face.fmdl".to_owned(),
            "k0431".to_owned(),
            "k0431/boots.fmdl".to_owned(),
        ],
    );
    let stdout = run.stdout();
    assert!(
        stdout.contains("model_folder_destination at 15 - Snuffy (into=Players)"),
        "{stdout}"
    );
    assert!(
        stdout.contains("model_folder_destination at k0431 (into=Boots)"),
        "{stdout}"
    );
}

// EU-CLI-06
#[test]
fn o_names_the_output_folder_itself_its_parent_made() {
    let sandbox = Sandbox::new("models_cli_o_folder");
    fs::create_dir_all(sandbox.path("new models/XXX15 - A")).unwrap();
    fs::write(sandbox.path("new models/XXX15 - A/face.fmdl"), b"face").unwrap();
    fs::create_dir_all(sandbox.path("new models/k0431")).unwrap();
    fs::write(sandbox.path("new models/k0431/boots.fmdl"), b"boots").unwrap();

    let out = sandbox.arg("out/done");
    let run = upgrade_models(&sandbox, "new models", &["-o", &out]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // The output folder is `done` itself: one folder per input folder,
    // directly in it, and no sibling "new models upgraded".
    assert_eq!(
        sandbox.listing("out/done"),
        [
            "15 - A".to_owned(),
            "15 - A/face.fmdl".to_owned(),
            "k0431".to_owned(),
            "k0431/boots.fmdl".to_owned(),
        ],
    );
    assert!(!sandbox.path("new models upgraded").exists());
}

// EU-CLI-04
#[test]
fn an_output_path_ending_in_parent_dir_is_refused() {
    let sandbox = Sandbox::new("models_cli_o_dotdot");
    fs::create_dir_all(sandbox.path("new models/XXX15 - A")).unwrap();
    fs::write(sandbox.path("new models/XXX15 - A/face.fmdl"), b"face").unwrap();
    let before = snapshot(&sandbox);

    // `gone` missing: `gone/..` has no file name on Windows either.
    let out = sandbox.arg("gone/..");
    upgrade_models(&sandbox, "new models", &["-o", &out]).assert_refused(2, &["gone"]);
    assert_eq!(snapshot(&sandbox), before, "nothing is written");
}

// EU-MF-01
#[test]
fn each_input_folder_is_migrated_and_its_destination_reported() {
    let sandbox = Sandbox::new("mf_folders");
    sandbox.write(
        "new models/XXX15 - Snuffy/face.fmdl",
        &fmdl(
            &[(
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "u0XXXp1.dds",
            ); 4],
        ),
    );
    sandbox.write("new models/XXX15 - Snuffy/u0XXXp1.dds", b"p1");
    sandbox.write("new models/g0430 - Crocs/gloveL_crocs.model", b"gloves");
    sandbox.write("new models/k0431/boots.model", b"boots");
    sandbox.write("new models/Snuffy/hair.model", b"hair");
    sandbox.write(
        "new models/Snuffy/face.xml",
        br#"<?xml version="1.0" encoding="UTF-8" ?>
<config>
    <model level="0" type="parts" path="./hair_u0XXXp1.model" material="./face.mtl"/>
</config>
"#,
    );
    sandbox.write("new models/Snuffy/hair_u0XXXp1.model", b"hair model");
    sandbox.write("new models/Snuffy/ingame_face.txt", b"");
    sandbox.write("new models/Common/u0XXXp0.dds", b"p0");

    let run = upgrade_models(&sandbox, "new models", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        sandbox
            .path("new models upgraded/15 - Snuffy/kit1.dds")
            .exists(),
        "{}",
        run.stdout()
    );
    for entry in sandbox.listing("new models upgraded") {
        assert!(!entry.contains("u0XXX"), "{entry}");
    }
    for (directory, file_name) in fmdl_paths(&sandbox, "new models upgraded/15 - Snuffy/face.fmdl")
    {
        assert_eq!((directory.as_str(), file_name.as_str()), ("./", "kit1.dds"));
    }
    assert!(
        sandbox
            .path("new models upgraded/Crocs/crocs_gloveL.model")
            .exists()
    );
    assert!(
        sandbox
            .path("new models upgraded/k0431/boots.model")
            .exists()
    );
    assert_eq!(
        fs::read(sandbox.path("new models upgraded/Snuffy/ingame_face.toml")).unwrap(),
        b""
    );
    for name in ["ingame_face", "ingame_face.txt"] {
        assert!(
            !sandbox
                .path(&format!("new models upgraded/Snuffy/{name}"))
                .exists()
        );
    }
    // The member `face.xml`'s own-model `path` gets the kit magic too, and
    // its renamed model and the untouched one are both written.
    let xml = fs::read_to_string(sandbox.path("new models upgraded/Snuffy/face.xml")).unwrap();
    assert!(xml.contains("path=\"./hair_kit1.model\""), "{xml}");
    assert!(
        sandbox
            .path("new models upgraded/Snuffy/hair_kit1.model")
            .exists()
    );
    assert!(
        sandbox
            .path("new models upgraded/Snuffy/hair.model")
            .exists()
    );
    assert!(sandbox.path("new models upgraded/Common/kitF.dds").exists());
    let stdout = run.stdout();
    assert_eq!(
        stdout.matches("model_folder_destination").count(),
        5,
        "{stdout}"
    );
    for line in [
        "model_folder_destination at 15 - Snuffy (into=Players)",
        "model_folder_destination at Snuffy (into=Players)",
        "model_folder_destination at Crocs (into=Gloves)",
        "model_folder_destination at k0431 (into=Boots)",
        "model_folder_destination at Common (into=root)",
    ] {
        assert!(stdout.contains(line), "{line}: {stdout}");
    }

    // The same model in a *boots* folder is the run that proves the pre-Fox
    // engine read: the `.model` in its output selects PES 16, whose code is
    // `model_name_invalid` (under PES 21 the same content is
    // `fmdl_name_invalid`); an unclean run writes nothing.
    sandbox.write("bad models/k0430 - Crocs/gloveL_crocs.model", b"gloves");
    let run = upgrade_models(&sandbox, "bad models", &[]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 1, "{stdout}");
    assert!(stdout.contains("model_name_invalid at Crocs"), "{stdout}");
    sandbox.assert_nothing_written("bad models upgraded", &stdout);
}

// EU-MF-10
#[test]
fn common_references_and_links_resolve_in_the_staged_export() {
    let sandbox = Sandbox::new("mf_common_links");
    sandbox.write("new models/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write(
        "new models/XXX07 - A/face.mtl",
        &mtl(&[
            "model/character/uniform/common/XXX/hair.dds",
            "model/character/uniform/common/XXX/gone.dds",
        ]),
    );
    sandbox.write("new models/XXX07 - A/kit.mtl.common", b"");
    sandbox.write("new models/Common/hair.dds", b"hair");
    let kit = mtl(&["./face.dds"]);
    sandbox.write("new models/Common/kit.mtl", &kit);
    sandbox.write("new models/Common/face.dds", b"face texture");

    let run = upgrade_models(&sandbox, "new models", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let face = mtl_text(&sandbox, "new models upgraded/07 - A/face.mtl");
    for expected in ["./hair.dds", "team_common_folder/gone.dds"] {
        assert!(face.contains(expected), "{expected}: {face}");
    }
    assert_eq!(
        fs::read(sandbox.path("new models upgraded/07 - A/hair.dds.common")).unwrap(),
        b""
    );
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "common_reference_kept at XXX07 - A/face.mtl (reference=model/character/uniform/common/XXX/gone.dds)"
        ),
        "{stdout}"
    );
    assert_eq!(
        fs::read(sandbox.path("new models upgraded/Common/kit.mtl")).unwrap(),
        kit
    );
    assert!(
        sandbox
            .path("new models upgraded/07 - A/kit.mtl.common")
            .exists()
    );
    for code in [
        "link_target_missing",
        "common_link_missing",
        "player_folder_number_invalid",
    ] {
        assert!(!stdout.contains(code), "{code}: {stdout}");
    }
}

// EU-MF-03
#[test]
fn a_folder_holding_no_file_is_unmigrated_content() {
    let sandbox = Sandbox::new("mf_unmigrated");
    sandbox.write("new models/players/Ann/face.fmdl", b"face");
    sandbox.write("new models/Thumbs/Thumbs.db", b"db");
    sandbox.write("new models/XXX07 - A/face.fmdl", b"face");

    let run = upgrade_models(&sandbox, "new models", &[]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 1, "{stdout}");
    for folder in [
        "unmigrated_content at players (reason=no_direct_files)",
        "unmigrated_content at Thumbs (reason=no_direct_files)",
    ] {
        assert!(stdout.contains(folder), "{folder}: {stdout}");
    }
    assert_eq!(unmigrated_found(&run), 1, "{stdout}");
    sandbox.assert_nothing_written("new models upgraded", &stdout);
}

// EU-MF-06
#[test]
fn a_face_folder_without_a_model_migrates_its_files() {
    let sandbox = Sandbox::new("mf_modelless_face");
    sandbox.write("new models/XXX01 - Dekinai/face_diff.bin", b"diff");
    sandbox.write("new models/XXX01 - Dekinai/portrait.dds", b"portrait");

    let run = upgrade_models(&sandbox, "new models", &[]);
    for file in [
        "new models upgraded/01 - Dekinai/face_diff.bin",
        "new models upgraded/01 - Dekinai/portrait.dds",
    ] {
        assert!(sandbox.path(file).exists(), "{file}: {}", run.stdout());
    }
    let stdout = run.stdout();
    assert!(
        stdout.contains("model_folder_destination at 01 - Dekinai (into=Players)"),
        "{stdout}"
    );
}

// EU-MF-09
#[test]
fn a_goalkeeper_token_respells_to_gk_kit_without_a_team() {
    let sandbox = Sandbox::new("mf_keeper_token");
    sandbox.write(
        "new models/XXX01 - Keeper/face.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "u0XXXg1.dds",
            ),
            (
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "u0XXXg0.dds",
            ),
        ]),
    );
    sandbox.write("new models/XXX01 - Keeper/u0XXXg1.dds", b"gk1");
    sandbox.write("new models/XXX01 - Keeper/u0XXXg2.dds", b"gk2");

    let run = upgrade_models(&sandbox, "new models", &[]);
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for name in ["gk_kit1.dds", "gk_kit2.dds"] {
        assert!(
            sandbox
                .path(&format!("new models upgraded/01 - Keeper/{name}"))
                .exists(),
            "{name}: {}",
            run.stdout()
        );
    }
    assert_eq!(
        fmdl_paths(&sandbox, "new models upgraded/01 - Keeper/face.fmdl")[..2],
        [
            ("./".to_owned(), "gk_kit1.dds".to_owned()),
            ("./".to_owned(), "gk_kitN.dds".to_owned())
        ]
    );
}

// EU-MF-05
#[test]
fn the_staged_validation_leaves_the_placement_findings_to_the_destination() {
    let sandbox = Sandbox::new("mf_placement");
    // Two player folders of one slot, one folder without a number and a
    // boots folder nobody links: the numbering and link findings are the
    // destination export's to make, not this run's.
    sandbox.write("new models/XXX15 - A/face.fmdl", b"face");
    sandbox.write("new models/XXX15 - B/face.fmdl", b"face");
    sandbox.write("new models/Snuffy/face.fmdl", b"face");
    sandbox.write("new models/k0431/boots.fmdl", b"boots");

    let run = upgrade_models(&sandbox, "new models", &[]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    for code in [
        "player_folder_number_invalid",
        "player_number_duplicate",
        "shared_folder_orphaned",
    ] {
        assert!(!stdout.contains(code), "{code}: {stdout}");
    }
}

// EU-MF-07
#[test]
fn a_stray_fmdl_in_a_boots_folder_is_a_leftover() {
    let sandbox = Sandbox::new("mf_stray_fmdl");
    // `boots - Copy.fmdl` is an `.fmdl` under no name a Fox boots folder
    // loads: step 10's stray rule drops it rather than staging it for the
    // strict output check to refuse.
    sandbox.write("new models/k0431/boots.fmdl", &fmdl(&[]));
    sandbox.write("new models/k0431/boots - Copy.fmdl", &fmdl(&[]));

    let run = upgrade_models(&sandbox, "new models", &[]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(stdout.contains("migration_clean"), "{stdout}");
    assert!(
        stdout.contains("leftover_dropped at k0431/boots - Copy.fmdl"),
        "{stdout}"
    );
    assert!(
        sandbox
            .path("new models upgraded/k0431/boots.fmdl")
            .exists(),
        "{stdout}"
    );
    assert!(
        !sandbox
            .path("new models upgraded/k0431/boots - Copy.fmdl")
            .exists()
    );
}
