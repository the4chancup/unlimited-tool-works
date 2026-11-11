//! `compile` of a `legacy_ids.toml` export: a team export's root file of the old boots
//! and gloves IDs its folders packed under, which the Export upgrader writes for VGL27.
//! Each recorded slot's own package goes under the old ID, an ID several slots record is
//! packed once, and a slot recording none of a kind packs nothing of it and keeps the
//! save's ID. A file that does not parse is `legacy_ids_invalid` and the export compiles
//! as it does without the file.

use std::fs;

use crate::clean_model;
use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes21_settings};
use crate::patch::read_patch;

/// TC-LEG-01's `legacy_ids.toml`: slot 05's boots under 2305, slots 06 and 07's gloves
/// under 2310.
const LEGACY_IDS: &[u8] = b"# The old export's boots and gloves IDs, which the Team compiler packs instead of its own (VGL27).\n[05]\nboots = 2305\n[06]\ngloves = 2310\n[07]\ngloves = 2310\n";

/// The sandbox's `/gbfg/` export `gbfg Full Legacy` (team 796, its Studio block 3901 to
/// 3940): slot 05 holds boots, slots 06 and 07 the same gloves model byte for byte, slot
/// 08 boots; with `ids` as its root `legacy_ids.toml` when given.
fn legacy_export(sandbox: &Sandbox, ids: Option<&[u8]>) -> &'static str {
    // The sandbox's default teams list does not hold /gbfg/.
    sandbox.write("data/teams_list.txt", b"ID\tName\n796\t/gbfg/\n");
    let export = "exports/gbfg Full Legacy";
    for (folder, model) in [
        ("05 - A", "boots"),
        ("06 - B", "glove_l"),
        ("07 - C", "glove_l"),
        ("08 - D", "boots"),
    ] {
        sandbox.write(
            &format!("{export}/Players/{folder}/{model}.fmdl"),
            &clean_model(),
        );
    }
    if let Some(ids) = ids {
        sandbox.write(&format!("{export}/legacy_ids.toml"), ids);
    }
    export
}

/// The boots and gloves package paths of `sandbox`'s compiled CPK, sorted (the `.fpk`
/// alone of each `.fpk`/`.fpkd` pair).
fn boots_gloves_paths(sandbox: &Sandbox) -> Vec<String> {
    let mut paths: Vec<String> = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))
        .keys()
        .filter(|path| {
            (path.contains("/boots/k") || path.contains("/glove/g")) && path.ends_with(".fpk")
        })
        .cloned()
        .collect();
    paths.sort_unstable();
    paths
}

// TC-LEG-01
#[test]
fn a_legacy_export_packs_and_patches_the_old_boots_and_gloves_ids() {
    let sandbox = Sandbox::new("leg_ids");
    let export = legacy_export(&sandbox, Some(LEGACY_IDS));

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert!(
        lines.iter().all(|line| !line.contains("Players/08")),
        "no finding names slot 08's boots: {lines:#?}"
    );
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k2305/#Win/boots.fpk",
            "Asset/model/character/glove/g2310/#Win/glove.fpk",
        ],
        "the old IDs packed once each, none of the Studio block 3901-3940"
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2305));
    assert_eq!(team.players[&6].gloves_id, Some(2310));
    assert_eq!(team.players[&7].gloves_id, Some(2310));
    assert_eq!(
        team.players[&8].boots_id, None,
        "the save keeps slot 08's boots ID"
    );

    // Without the file the same export compiles on Studio's IDs.
    fs::remove_file(sandbox.root.join(format!("{export}/legacy_ids.toml"))).unwrap();
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k3905/#Win/boots.fpk",
            "Asset/model/character/boots/k3908/#Win/boots.fpk",
            "Asset/model/character/glove/g3906/#Win/glove.fpk",
            "Asset/model/character/glove/g3907/#Win/glove.fpk",
        ]
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(3905));
    assert_eq!(team.players[&8].boots_id, Some(3908));
}

// TC-LEG-02
#[test]
fn a_legacy_ids_toml_that_does_not_parse_is_an_error_and_the_export_compiles_on_studio_ids() {
    let sandbox = Sandbox::new("leg_ids_bad");
    legacy_export(&sandbox, Some(b"[05 boots = 2305\n"));

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(
        lines.iter().any(|line| line.starts_with(
            "gbfg Full Legacy: Error legacy_ids_invalid [DropFile] at legacy_ids.toml (file=legacy_ids.toml, error="
        )),
        "{lines:#?}"
    );
    // As without the file: Studio's IDs.
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k3905/#Win/boots.fpk",
            "Asset/model/character/boots/k3908/#Win/boots.fpk",
            "Asset/model/character/glove/g3906/#Win/glove.fpk",
            "Asset/model/character/glove/g3907/#Win/glove.fpk",
        ]
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(3905));
}

// TC-LEG-05
#[test]
fn a_legacy_export_packs_a_kept_shared_folder_under_its_wearers_old_id() {
    let sandbox = Sandbox::new("leg_ids_shared");
    // The sandbox's default teams list does not hold /gbfg/.
    sandbox.write("data/teams_list.txt", b"ID\tName\n796\t/gbfg/\n");
    let export = "exports/gbfg Full Legacy";
    // Slots 05 and 08 link the shared boots folder, slot 06 alone the
    // shared gloves one.
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Players/08 - D/Crocs.boots"), b"");
    sandbox.write(&format!("{export}/Players/06 - C/Mitts.gloves"), b"");
    sandbox.write(&format!("{export}/Boots/Crocs/boots.fmdl"), &clean_model());
    sandbox.write(
        &format!("{export}/Gloves/Mitts/glove_l.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/legacy_ids.toml"),
        b"[05]\nboots = 2305\n[06]\ngloves = 2310\n[08]\nboots = 2305\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k2305/#Win/boots.fpk",
            "Asset/model/character/glove/g2310/#Win/glove.fpk",
        ],
        "the old IDs packed once each, none of the Studio block 3901-3940"
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2305));
    assert_eq!(team.players[&8].boots_id, Some(2305));
    assert_eq!(team.players[&6].gloves_id, Some(2310));

    // Without the file the same export compiles on Studio's IDs: the
    // shared folder's shared ID, and slot 06's exclusive one for the
    // gloves folder he alone links.
    fs::remove_file(sandbox.root.join(format!("{export}/legacy_ids.toml"))).unwrap();
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k3924/#Win/boots.fpk",
            "Asset/model/character/glove/g3906/#Win/glove.fpk",
        ]
    );
}
