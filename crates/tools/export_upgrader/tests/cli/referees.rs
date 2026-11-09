//! Referee exports (the `spec/export_upgrader.md` "Referee exports" scenarios): `refs.txt`
//! rewritten as the root `players.txt`, `ref_marker.dds` carried, the referee
//! folders under `Players/` copied with their old `face/`, `boots/`, `gloves/`
//! and `common/` subfolders folded in, and a `refs` export without a roster
//! refused with nothing written. The folder names are real referees'
//! (`arbiter`, `eiki`, from a real `refs` export).

use std::fs;

use crate::carried::{mtl, mtl_text};
use crate::common::Sandbox;
use crate::players::face;

/// `upgrade <input>`, no flag: a referee export takes neither.
fn upgrade(sandbox: &Sandbox, input: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    sandbox.run(&["upgrade", &input])
}

// EU-REF-01
#[test]
fn refs_txt_becomes_the_canonical_players_txt_and_the_marker_is_carried() {
    let sandbox = Sandbox::new("ref_roster");
    // 35 slots, written unpadded as real exports write them, two folders
    // backing them all: the repeats are the rarity control.
    let folder = |n: u8| if n % 2 == 1 { "arbiter" } else { "eiki" };
    let refs_txt: String = (1..=35u8).map(|n| format!("{n} {}\n", folder(n))).collect();
    let expected: String = (1..=35u8)
        .map(|n| format!("{n:02} {}\n", folder(n)))
        .collect();
    assert_ne!(refs_txt, expected, "the input is not already canonical");
    sandbox.write("refs Spring/refs.txt", refs_txt.as_bytes());
    sandbox.write("refs Spring/ref_marker.dds", b"marker");
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/arbiter/boots.skl", b"skl");
    sandbox.write("refs Spring/Players/arbiter/arbiter.dds", b"arbiter");
    sandbox.write("refs Spring/Players/eiki/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/eiki/boots.skl", b"skl");
    // A Note txt: nothing compiled it, referees have no colors (step 9).
    sandbox.write("refs Spring/refs Note.txt", b"notes");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("- refs Spring: Warning leftover_dropped at refs Note.txt\n"),
        "{}",
        run.stdout()
    );
    assert_eq!(
        fs::read_to_string(sandbox.path("refs Spring upgraded/players.txt")).unwrap(),
        expected,
        "{}",
        run.stdout()
    );
    let listing = sandbox.listing("refs Spring upgraded");
    assert!(
        !listing
            .iter()
            .any(|path| path.to_ascii_lowercase().ends_with("refs.txt")),
        "{listing:?}"
    );
    assert_eq!(
        fs::read(sandbox.path("refs Spring upgraded/ref_marker.dds")).unwrap(),
        b"marker"
    );
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
}

// EU-REF-10
#[test]
fn refs_list_txt_and_root_referee_folders_migrate_the_same_way() {
    // The real "refs for Winter 25 Day 2" export's layout: the older
    // roster name, the referee folders directly at the export's root.
    let sandbox = Sandbox::new("ref_list_layout");
    sandbox.write(
        "refs Winter/Refs list.txt",
        b"01 boston\n02 cena\n03 boston\n",
    );
    sandbox.write("refs Winter/boston/Face/face.model", b"face");
    sandbox.write("refs Winter/boston/Face/face_diff.bin", b"diff");
    sandbox.write("refs Winter/boston/Common/kwt.dds", b"kwt");
    sandbox.write("refs Winter/cena/Face/face.model", b"face");
    sandbox.write("refs Winter/Common/shared.dds", b"shared");

    let run = upgrade(&sandbox, "refs Winter");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read_to_string(sandbox.path("refs Winter upgraded/players.txt")).unwrap(),
        "01 boston\n02 cena\n03 boston\n",
        "{}",
        run.stdout()
    );
    let listing = sandbox.listing("refs Winter upgraded");
    for path in [
        "Players/boston/face.model",
        "Players/boston/kwt.dds",
        "Players/cena/face.model",
        "Common/shared.dds",
    ] {
        assert!(
            listing.iter().any(|listed| listed == path),
            "{path}: {listing:?}"
        );
    }
    assert!(
        !listing
            .iter()
            .any(|path| path.to_ascii_lowercase().ends_with("refs list.txt")),
        "{listing:?}"
    );
    assert!(
        !listing.iter().any(|path| path.starts_with("boston")),
        "{listing:?}"
    );
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
}

// EU-REF-12
#[test]
fn a_root_referee_folders_model_counts_toward_the_pre_fox_version() {
    let sandbox = Sandbox::new("ref_root_model");
    // A `.model` inside a root-layout referee folder is inside the old
    // format: it reads the export as pre-Fox (`upgrade.rs`'s version
    // check), under which `hair.xml` is an edit-hair file a model folder
    // may not hold. Validated as PES 21 — the read the version check's
    // `referees_at_root` exists to prevent — the same file means nothing.
    sandbox.write("refs Winter/Refs list.txt", b"01 boston\n");
    sandbox.write("refs Winter/boston/Face/face.model", b"face");
    sandbox.write("refs Winter/boston/Face/face.mtl", &mtl(&[]));
    sandbox.write("refs Winter/boston/Face/hair.xml", b"<config/>");

    let run = upgrade(&sandbox, "refs Winter");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("edithair_unsupported at Players/boston"),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("refs Winter upgraded", &run.stdout());
}

/// A `refs Spring` export whose `refs.txt` lists the two real referee
/// folders `arbiter` and `eiki`; the caller writes their files.
fn referee_export(sandbox: &Sandbox) {
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n02 eiki\n");
}

// EU-REF-02
#[test]
fn the_old_subfolders_fold_into_the_referee_folder_and_a_twin_is_kept_once() {
    let sandbox = Sandbox::new("ref_flatten");
    referee_export(&sandbox);
    for (path, bytes) in [
        ("face/face_diff.bin", &b"diff"[..]),
        ("face/portrait.dds", b"portrait"),
        ("boots/boots.fmdl", b"boots"),
        ("boots/boots.skl", b"skl"),
        ("boots/arbiter.dds", b"arbiter"),
        ("gloves/glove_l.fmdl", b"left"),
        ("gloves/glove_r.fmdl", b"right"),
        ("common/arbiter.dds", b"arbiter"),
    ] {
        sandbox.write(&format!("refs Spring/Players/arbiter/{path}"), bytes);
    }
    sandbox.write("refs Spring/Players/eiki/boots.fmdl", b"boots");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        sandbox.listing("refs Spring upgraded/Players/arbiter"),
        [
            "arbiter.dds",
            "boots.fmdl",
            "boots.skl",
            "face_diff.bin",
            "glove_l.fmdl",
            "glove_r.fmdl",
            "portrait.dds",
        ],
        "{}",
        run.stdout()
    );
    assert_eq!(
        fs::read(sandbox.path("refs Spring upgraded/Players/arbiter/arbiter.dds")).unwrap(),
        b"arbiter"
    );
}

// EU-REF-03
#[test]
fn one_name_with_different_bytes_leaves_the_referee_folder_as_it_was() {
    let sandbox = Sandbox::new("ref_clash");
    referee_export(&sandbox);
    sandbox.write("refs Spring/Players/arbiter/boots/x.dds", b"a");
    sandbox.write("refs Spring/Players/arbiter/gloves/x.dds", b"b");
    sandbox.write("refs Spring/Players/arbiter/boots/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/arbiter/boots/u0XXXp1.dds", b"kit");
    sandbox.write("refs Spring/Players/eiki/boots.fmdl", b"boots");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    for (path, other) in [
        (
            "Players/arbiter/boots/x.dds",
            "Players/arbiter/gloves/x.dds",
        ),
        (
            "Players/arbiter/gloves/x.dds",
            "Players/arbiter/boots/x.dds",
        ),
    ] {
        assert!(
            run.stdout().contains(&format!(
                "unmigrated_content at {path} (reason=name_taken, other={other})"
            )),
            "{path}: {}",
            run.stdout()
        );
    }
    sandbox.assert_nothing_written("refs Spring upgraded", &run.stdout());
}

// EU-REF-08
#[test]
fn a_subfolder_of_face_keeps_its_place() {
    let sandbox = Sandbox::new("ref_face_subfolder");
    referee_export(&sandbox);
    sandbox.write("refs Spring/Players/arbiter/face/kit/kit.dds", b"kit");
    sandbox.write("refs Spring/Players/arbiter/boots/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/eiki/boots.fmdl", b"boots");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("refs Spring upgraded/Players/arbiter/kit/kit.dds")).unwrap(),
        b"kit",
        "{}",
        run.stdout()
    );
    assert!(
        !sandbox
            .path("refs Spring upgraded/Players/arbiter/face")
            .exists()
    );
}

// EU-REF-11
#[test]
fn a_nested_face_leaves_the_referee_folder_unmigrated() {
    let sandbox = Sandbox::new("ref_nested_face");
    referee_export(&sandbox);
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/eiki/face/face/face.fmdl", b"face");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("unmigrated_content at Players/eiki/face/face (reason=nested_layout)"),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("refs Spring upgraded", &run.stdout());
}

// EU-REF-09
#[test]
fn a_common_model_no_face_xml_names_is_reported_and_left_out() {
    let sandbox = Sandbox::new("ref_common_model");
    referee_export(&sandbox);
    sandbox.write(
        "refs Spring/Players/arbiter/common/parts_body.model",
        b"model",
    );
    sandbox.write("refs Spring/Players/arbiter/face/materials.mtl", b"mtl");
    sandbox.write("refs Spring/Players/arbiter/face/face_diff.bin", b"diff");
    sandbox.write("refs Spring/Players/eiki/boots.fmdl", b"boots");

    let run = upgrade(&sandbox, "refs Spring");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "unmigrated_content at Players/arbiter/common/parts_body.model (reason=model_unreferenced)"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("refs Spring upgraded", &run.stdout());
}

// EU-REF-06
#[test]
fn a_referee_s_references_name_his_flattened_files_and_a_common_one_is_linked() {
    let sandbox = Sandbox::new("ref_references");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    sandbox.write(
        "refs Spring/Players/arbiter/face/face.mtl",
        &mtl(&[
            "model/character/uniform/common/XXX/hair.dds",
            "./skin.dds",
            "model/character/uniform/common/XXX/kit/sock.dds",
            "./whistle.dds",
        ]),
    );
    sandbox.write("refs Spring/Players/arbiter/face/face.model", b"face");
    sandbox.write("refs Spring/Players/arbiter/common/hair.dds", b"hair");
    sandbox.write("refs Spring/Players/arbiter/common/skin.dds", b"skin");
    sandbox.write("refs Spring/Players/arbiter/common/kit/sock.dds", b"sock");
    sandbox.write("refs Spring/Common/whistle.dds", b"whistle");

    let run = upgrade(&sandbox, "refs Spring");
    let text = mtl_text(&sandbox, "refs Spring upgraded/Players/arbiter/face.mtl");
    for path in [
        "./hair.dds",
        "./skin.dds",
        "./kit/sock.dds",
        "./whistle.dds",
    ] {
        assert!(text.contains(&format!("path=\"{path}\"")), "{path}: {text}");
    }
    let listing = sandbox.listing("refs Spring upgraded/Players/arbiter");
    for file in ["hair.dds", "skin.dds", "kit/sock.dds", "whistle.dds.common"] {
        assert!(
            listing.iter().any(|path| path == file),
            "{file}: {listing:?}"
        );
    }
    assert!(
        !run.stdout().contains("whistle") && !run.stdout().contains("common_reference_kept"),
        "{}",
        run.stdout()
    );
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
}

// EU-REF-14
#[test]
fn a_referee_s_missing_common_reference_is_kept_as_team_folder() {
    let sandbox = Sandbox::new("ref_kept_common");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    // Neither the referee's `common/` nor the export's `Common/` holds the
    // texture: kept a game path, its Common written `team_common_folder/`.
    sandbox.write(
        "refs Spring/Players/arbiter/face/face.mtl",
        &mtl(&["model/character/uniform/common/999/refkit/texture.dds"]),
    );
    sandbox.write("refs Spring/Players/arbiter/face/face.model", b"face");

    let run = upgrade(&sandbox, "refs Spring");
    let text = mtl_text(&sandbox, "refs Spring upgraded/Players/arbiter/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/refkit/texture.dds\""),
        "{text}"
    );
    assert!(
        run.stdout().contains(
            "common_reference_kept at Players/arbiter/face/face.mtl (reference=model/character/uniform/common/999/refkit/texture.dds)"
        ),
        "{}",
        run.stdout()
    );
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
}

// EU-PLY-22
#[test]
fn either_ingame_face_spelling_in_a_referee_folder_becomes_ingame_face_toml() {
    let sandbox = Sandbox::new("ref_ingame");
    referee_export(&sandbox);
    sandbox.write(
        "refs Spring/Players/arbiter/face/ingame_face.txt",
        b"ignored",
    );
    sandbox.write("refs Spring/Players/arbiter/boots/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/eiki/ingame_face", b"");
    sandbox.write("refs Spring/Players/eiki/boots/boots.fmdl", b"boots");

    let run = upgrade(&sandbox, "refs Spring");
    for referee in ["Players/arbiter", "Players/eiki"] {
        let folder = format!("refs Spring upgraded/{referee}");
        // A referee export has no record to fill it with: the file is empty.
        assert_eq!(
            fs::read(sandbox.path(&format!("{folder}/ingame_face.toml"))).unwrap(),
            b"",
            "{referee}: {}",
            run.stdout()
        );
        for name in ["ingame_face", "ingame_face.txt", "face"] {
            assert!(
                !sandbox.path(&format!("{folder}/{name}")).exists(),
                "{referee}/{name}"
            );
        }
    }
}

// EU-REF-04
#[test]
fn a_subfolder_with_a_face_model_is_carried_in_both_kinds_of_player_folder() {
    let sandbox = Sandbox::new("ref_alt_subfolder");
    face(&sandbox, "co Spring/Faces/XXX07 - A");
    sandbox.write("co Spring/Faces/XXX07 - A/alt/face_high.fmdl", b"team alt");
    let save = sandbox.arg("EDIT");
    sandbox.save("EDIT");
    sandbox.write("refs Spring/refs.txt", b"01 arbiter\n");
    sandbox.write("refs Spring/Players/arbiter/boots.fmdl", b"boots");
    sandbox.write("refs Spring/Players/arbiter/alt/face_high.fmdl", b"ref alt");

    let input = sandbox.arg("co Spring");
    let team = sandbox.run(&["upgrade", &input, "--savefile", &save]);
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - A/alt/face_high.fmdl")).unwrap(),
        b"team alt",
        "{}",
        team.stdout()
    );
    let referees = upgrade(&sandbox, "refs Spring");
    assert_eq!(
        fs::read(sandbox.path("refs Spring upgraded/Players/arbiter/alt/face_high.fmdl")).unwrap(),
        b"ref alt",
        "{}",
        referees.stdout()
    );
}

// EU-REF-07
#[test]
fn a_refs_export_without_refs_txt_is_reported_and_nothing_is_written() {
    let sandbox = Sandbox::new("ref_no_roster");
    sandbox.write("refs Summer/Players/arbiter/boots.fmdl", b"boots");
    let before = sandbox.listing("");

    let run = upgrade(&sandbox, "refs Summer");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains("refs_txt_missing"),
        "{}",
        run.stdout()
    );
    assert_eq!(sandbox.listing(""), before, "nothing is written");
}
