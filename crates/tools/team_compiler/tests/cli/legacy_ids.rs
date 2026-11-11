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

/// One `legacy_id_out_of_block` line, as `messages` renders it, on `folder` for `id`
/// under `block` (`2301-2325`, or `none` when the table does not list the team).
fn out_of_block_line(folder: &str, id: u32, block: &str) -> String {
    format!(
        "gbfg Full Legacy: Error legacy_id_out_of_block [Keep] at {folder} (id={id}, \
         folder={folder}, block={block})"
    )
}

// TC-LEG-03
#[test]
fn an_old_id_outside_the_teams_block_is_an_error_and_not_packed() {
    let sandbox = Sandbox::new("leg_ids_block");
    // TC-LEG-01's export, plus slot 09's gloves recorded under /ksg/'s block (2522) and
    // slot 10's boots one past the block's end (2326 is 2301 + 25, the first ID outside
    // a 25-ID block).
    let export = legacy_export(
        &sandbox,
        Some(
            b"[05]\nboots = 2305\n[06]\ngloves = 2310\n[07]\ngloves = 2310\n[09]\ngloves = 2522\n[10]\nboots = 2326\n",
        ),
    );
    sandbox.write(
        &format!("{export}/Players/09 - E/glove_l.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/Players/10 - F/boots.fmdl"),
        &clean_model(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    for (folder, id) in [("Players/09 - E", 2522), ("Players/10 - F", 2326)] {
        assert!(
            lines
                .iter()
                .any(|line| *line == out_of_block_line(folder, id, "2301-2325")),
            "{lines:#?}"
        );
    }
    assert_eq!(
        boots_gloves_paths(&sandbox),
        [
            "Asset/model/character/boots/k2305/#Win/boots.fpk",
            "Asset/model/character/glove/g2310/#Win/glove.fpk",
        ],
        "the in-block old IDs packed, neither 2522 nor 2326"
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2305));
    assert_eq!(team.players[&9].gloves_id, Some(2522));
    assert_eq!(team.players[&10].boots_id, Some(2326));

    // The compile wrote the data directory's block table. Its /gbfg/ row removed, every
    // recorded old ID is out of block (its block `none`) and none of them packs; the
    // patch keeps them all.
    let table = fs::read_to_string(sandbox.root.join("data/legacy_boots_blocks.txt")).unwrap();
    let without_gbfg: String = table
        .lines()
        .filter(|line| !line.starts_with("/gbfg/"))
        .map(|line| format!("{line}\n"))
        .collect();
    sandbox.write("data/legacy_boots_blocks.txt", without_gbfg.as_bytes());
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    for (folder, id) in [
        ("Players/05 - A", 2305),
        ("Players/06 - B", 2310),
        ("Players/07 - C", 2310),
        ("Players/09 - E", 2522),
        ("Players/10 - F", 2326),
    ] {
        assert!(
            lines
                .iter()
                .any(|line| *line == out_of_block_line(folder, id, "none")),
            "{lines:#?}"
        );
    }
    assert_eq!(boots_gloves_paths(&sandbox), Vec::<String>::new());
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2305));
    assert_eq!(team.players[&6].gloves_id, Some(2310));
}

// TC-LEG-04
#[test]
fn the_block_table_is_written_for_a_legacy_compile_and_edits_take() {
    let sandbox = Sandbox::new("leg_ids_table");
    let export = legacy_export(&sandbox, None);
    let table = "data/legacy_boots_blocks.txt";

    // An export without legacy_ids.toml creates no file, and `check` creates none
    // either, of a legacy export included.
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert!(!sandbox.root.join(table).exists());
    sandbox.write(&format!("{export}/legacy_ids.toml"), LEGACY_IDS);
    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    assert!(!sandbox.root.join(table).exists(), "`check` writes nothing");

    // The first compile of a legacy export writes it from the embedded copy.
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    let text = fs::read_to_string(sandbox.root.join(table)).unwrap();
    assert_eq!(text.lines().next(), Some("Name\tFirstBootsID"));
    assert!(
        text.lines().any(|line| line == "/gbfg/\t2301"),
        "the table's /gbfg/ row: {text}"
    );

    // Its /gbfg/ row edited to 2326, TC-LEG-01's 2305 is out of block (the table the
    // run reads is the member's).
    sandbox.write(
        table,
        text.replacen("/gbfg/\t2301", "/gbfg/\t2326", 1).as_bytes(),
    );
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(
        lines
            .iter()
            .any(|line| *line == out_of_block_line("Players/05 - A", 2305, "2326-2350")),
        "{lines:#?}"
    );
}

// TC-LEG-03
#[test]
fn a_kept_shared_folder_whose_old_id_is_out_of_block_is_not_packed() {
    let sandbox = Sandbox::new("leg_ids_shared_block");
    // The sandbox's default teams list does not hold /gbfg/.
    sandbox.write("data/teams_list.txt", b"ID\tName\n796\t/gbfg/\n");
    let export = "exports/gbfg Full Legacy";
    // TC-LEG-05's export, the linkers' recorded boots ID out of /gbfg/'s block.
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
        b"[05]\nboots = 2522\n[06]\ngloves = 2310\n[08]\nboots = 2522\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let out_of_block: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("legacy_id_out_of_block"))
        .collect();
    // Once, on the folder that would have packed the ID: the linkers' folders
    // pack no boots of their own, so the shared folder alone is named.
    assert_eq!(
        out_of_block,
        [&out_of_block_line("Boots/Crocs", 2522, "2301-2325")]
    );
    assert_eq!(
        boots_gloves_paths(&sandbox),
        ["Asset/model/character/glove/g2310/#Win/glove.fpk"],
        "Crocs packed nowhere: not under 2522, not under a Studio ID"
    );
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2522));
    assert_eq!(team.players[&8].boots_id, Some(2522));
    assert_eq!(team.players[&6].gloves_id, Some(2310));
}

// TC-LEG-03
#[test]
fn an_own_model_and_a_linked_one_report_each_kind_on_its_packing_folder() {
    let sandbox = Sandbox::new("leg_ids_own_linked");
    // The sandbox's default teams list does not hold /gbfg/.
    sandbox.write("data/teams_list.txt", b"ID\tName\n796\t/gbfg/\n");
    let export = "exports/gbfg Full Legacy";
    // Slot 05's own boots and the shared gloves folder he links, each recorded
    // under an old ID outside /gbfg/'s block.
    sandbox.write(
        &format!("{export}/Players/05 - A/boots.fmdl"),
        &clean_model(),
    );
    sandbox.write(&format!("{export}/Players/05 - A/Mitts.gloves"), b"");
    sandbox.write(
        &format!("{export}/Gloves/Mitts/glove_l.fmdl"),
        &clean_model(),
    );
    sandbox.write(
        &format!("{export}/legacy_ids.toml"),
        b"[05]\nboots = 2522\ngloves = 2523\n",
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let out_of_block: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("legacy_id_out_of_block"))
        .collect();
    // Each kind named once on the folder that would have packed it: the shared
    // folder for the linked gloves, the player's own folder for his boots.
    assert_eq!(
        out_of_block,
        [
            &out_of_block_line("Gloves/Mitts", 2523, "2301-2325"),
            &out_of_block_line("Players/05 - A", 2522, "2301-2325"),
        ]
    );
    assert_eq!(boots_gloves_paths(&sandbox), Vec::<String>::new());
    let patch = read_patch(&sandbox);
    let [team] = patch.teams.as_slice() else {
        panic!("one [[teams]] entry")
    };
    assert_eq!(team.players[&5].boots_id, Some(2522));
    assert_eq!(team.players[&5].gloves_id, Some(2523));
}

// TC-LEG-03
#[test]
fn another_teams_old_id_is_an_error_and_only_the_owning_team_packs_it() {
    let sandbox = Sandbox::new("leg_ids_friendlies");
    // /gbfg/ (block 2301 to 2325) and /dbg/ (block 2001 to 2025), each held by the
    // embedded teams list and the block table alike.
    sandbox.write(
        "data/teams_list.txt",
        b"ID\tName\n789\t/dbg/\n796\t/gbfg/\n",
    );
    for (export, folder) in [
        ("gbfg Full Legacy", "05 - A"),
        ("dbg Full Legacy", "05 - B"),
    ] {
        sandbox.write(
            &format!("exports/{export}/Players/{folder}/boots.fmdl"),
            &clean_model(),
        );
        sandbox.write(
            &format!("exports/{export}/legacy_ids.toml"),
            b"[05]\nboots = 2305\n",
        );
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    // /dbg/'s 2305 is out of its block: an Error on it, not the duplicate path's Fatal.
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(
        lines.iter().any(|line| line
            == "dbg Full Legacy: Error legacy_id_out_of_block [Keep] at Players/05 - B \
                (id=2305, folder=Players/05 - B, block=2001-2025)"),
        "{lines:#?}"
    );
    assert_eq!(
        boots_gloves_paths(&sandbox),
        ["Asset/model/character/boots/k2305/#Win/boots.fpk"],
        "one k2305, /gbfg/'s"
    );
}
