//! The carried folders `Portraits/`, `Collars/` and `Logo/` (the
//! `spec/export_upgrader.md` "Portraits, logo, models and Common" scenarios): the portrait rename
//! and its face-folder conflict check, collars as they are, and the logo
//! members' root files and their downscale-or-drawing test.

use std::fs;

use crate::common::Sandbox;

/// `upgrade <input> --savefile <save>`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    sandbox.run(&["upgrade", &input, "--savefile", &save])
}

/// The fixture file `relative` below `tests/fixtures/logos/`.
fn logo_fixture(relative: &str) -> Vec<u8> {
    fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/logos")
            .join(relative),
    )
    .unwrap()
}

// EU-MDL-01
#[test]
fn portraits_renamed_and_collars_carried() {
    let sandbox = Sandbox::new("mdl_portraits_collars");
    for (name, bytes) in [
        ("player_XXX05.dds", b"p05" as &[u8]),
        ("player_71407.dds", b"p07"),
        ("player_09.dds", b"p09"),
    ] {
        sandbox.write(&format!("co Kits/Portraits/{name}"), bytes);
    }
    sandbox.write("co Kits/Collars/collar_120.fmdl", b"collar");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (to, bytes) in [
        ("05.dds", b"p05" as &[u8]),
        ("07.dds", b"p07"),
        ("09.dds", b"p09"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/Portraits/{to}"))).unwrap(),
            bytes,
            "{to}"
        );
    }
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Collars/collar_120.fmdl")).unwrap(),
        b"collar"
    );
    assert!(
        !run.stdout().contains("unmigrated_content"),
        "{}",
        run.stdout()
    );
}

// EU-MDL-19
#[test]
fn only_a_portrait_the_old_check_passed_migrates_the_refused_are_leftovers() {
    let sandbox = Sandbox::new("mdl_portraits_leftovers");
    // BAG VGL18.5's `player_XXX08.jpg` migrated; the six other names the
    // old check refused (a subfolder's included) never reached the game.
    let refused = [
        "XXX17.png",
        "player_XXX24.dds",
        "Vanilla vitamins.png",
        "Player_XXX05.dds",
        "player_XXX06.jpeg",
        "sub/player_XXX07.dds",
    ];
    sandbox.write("co Kits/Portraits/player_XXX08.jpg", b"portrait jpg");
    for name in refused {
        sandbox.write(&format!("co Kits/Portraits/{name}"), b"leftover");
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(sandbox.listing("co Full Kits/Portraits"), ["08.jpg"]);
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Portraits/08.jpg")).unwrap(),
        b"portrait jpg"
    );
    let stdout = run.stdout();
    for name in refused {
        assert!(
            stdout.contains(&format!("leftover_dropped at Portraits/{name}")),
            "{name}: {stdout}"
        );
    }
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-MDL-20
#[test]
fn two_portraits_of_one_slot_keep_the_first_and_report_the_second() {
    let sandbox = Sandbox::new("mdl_portraits_one_slot");
    sandbox.write("co Kits/Portraits/player_71405.dds", b"714");
    sandbox.write("co Kits/Portraits/player_XXX05.dds", b"xxx");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains("- co Kits: Error unmigrated_content at Portraits/player_XXX05.dds (reason=portrait_taken, other=Portraits/player_71405.dds)"),
        "{stdout}"
    );
    // The second file alone: the first keeps the slot.
    assert_eq!(stdout.matches("unmigrated_content").count(), 1, "{stdout}");
    sandbox.assert_nothing_written("co Full Kits", &stdout);
}

// EU-MDL-02
#[test]
fn a_downscale_small_logo_is_dropped_with_its_share() {
    let sandbox = Sandbox::new("mdl_logo_downscale");
    sandbox.copy_fixture_dir("logos/OMG VGL26", "co Kits/Logo");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/logo.png")).unwrap(),
        logo_fixture("OMG VGL26/emblem_0XXX_r_ll.png")
    );
    assert!(!sandbox.path("co Full Kits/logo_small.png").exists());
    // `_r_l` is dropped with no finding: no output file carries its bytes.
    let medium = logo_fixture("OMG VGL26/emblem_0XXX_r_l.png");
    for entry in sandbox.listing("co Full Kits") {
        let path = sandbox.path(&format!("co Full Kits/{entry}"));
        if path.is_file() {
            assert_ne!(fs::read(&path).unwrap(), medium, "{entry}");
        }
    }
    assert!(
        run.stdout()
            .contains("logo_small_dropped at Logo/emblem_0XXX_r.png (share=30%)"),
        "{}",
        run.stdout()
    );
}

// EU-MDL-03
#[test]
fn a_distinct_small_logo_is_carried() {
    let sandbox = Sandbox::new("mdl_logo_variant");
    sandbox.copy_fixture_dir("logos/VRG VGL26", "co Kits/Logo");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/logo.png")).unwrap(),
        logo_fixture("VRG VGL26/emblem_0XXX_r_ll.png")
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/logo_small.png")).unwrap(),
        logo_fixture("VRG VGL26/emblem_0XXX_r.png")
    );
    assert!(
        run.stdout()
            .contains("logo_small_carried at Logo/emblem_0XXX_r.png (share=53%)"),
        "{}",
        run.stdout()
    );
}

// EU-MDL-04
#[test]
fn the_fallback_medium_member_becomes_the_logo() {
    let sandbox = Sandbox::new("mdl_logo_fallback");
    for name in ["emblem_0XXX_r_l.png", "emblem_0XXX_r.png"] {
        sandbox.write(
            &format!("co Kits/Logo/{name}"),
            &logo_fixture(&format!("VRG VGL26/{name}")),
        );
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/logo.png")).unwrap(),
        logo_fixture("VRG VGL26/emblem_0XXX_r_l.png")
    );
    assert!(!sandbox.path("co Full Kits/logo_small.png").exists());
    assert!(
        run.stdout()
            .contains("logo_fallback at Logo/emblem_0XXX_r_l.png"),
        "{}",
        run.stdout()
    );
}

// EU-MDL-15
#[test]
fn a_portrait_of_other_bytes_in_the_face_folder_is_a_conflict() {
    let sandbox = Sandbox::new("mdl_portrait_conflict");
    sandbox.write("co Spring/Faces/XXX07 - A/face.fmdl", b"face");
    sandbox.write("co Spring/Faces/XXX07 - A/portrait.dds", b"face portrait");
    sandbox.write("co Spring/Portraits/player_XXX07.dds", b"other portrait");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout()
            .contains("portrait_conflict at Portraits/player_XXX07.dds (slot=07)"),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Spring", &run.stdout());
}

/// A `.mtl` of one material holding one sampler per `paths` entry.
pub(crate) fn mtl(paths: &[&str]) -> Vec<u8> {
    let samplers: String = paths
        .iter()
        .map(|path| {
            format!("        <sampler name=\"DiffuseMap\" path=\"{path}\" srgb=\"1\" minfilter=\"linear\" magfilter=\"linear\" uaddr=\"clamp\" vaddr=\"clamp\"/>\r\n")
        })
        .collect();
    format!(
        "<materialset>\r\n    <material name=\"face_phong\" shader=\"Wrinkle_Face\">\r\n{samplers}    </material>\r\n</materialset>\r\n"
    )
    .into_bytes()
}

/// The one `.mtl` output's text.
pub(crate) fn mtl_text(sandbox: &Sandbox, output: &str) -> String {
    fs::read_to_string(sandbox.path(output)).unwrap()
}

/// An FMDL of the `konami_highneck` fixture with its texture entries set
/// from `entries` (`(directory, file_name)` pairs, in table order).
pub(crate) fn fmdl(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut file = fmdl::FmdlFile::read(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/fmdl/tests/fixtures/konami_highneck.fmdl"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut entries = entries.iter();
    fmdl::ops::paths::rewrite_texture_paths(&mut file, |path| {
        if let Some((directory, name)) = entries.next() {
            path.directory = (*directory).to_owned();
            path.file_name = (*name).to_owned();
        }
    })
    .unwrap();
    file.write()
}

/// The `(directory, file_name)` entries of `output`'s written FMDL.
pub(crate) fn fmdl_paths(sandbox: &Sandbox, output: &str) -> Vec<(String, String)> {
    let file = fmdl::FmdlFile::read(&fs::read(sandbox.path(output)).unwrap()).unwrap();
    fmdl::ops::paths::texture_paths(&file)
        .unwrap()
        .iter()
        .map(|path| (path.directory.clone(), path.file_name.clone()))
        .collect()
}

/// The `fmdl` crate's fixture `name`, lifted to its `Model`.
fn fixture_model(name: &str) -> fmdl::Model {
    let file = fmdl::FmdlFile::read(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/fmdl/tests/fixtures")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap();
    fmdl::Model::from_file(&file).unwrap()
}

/// The `konami_highneck` FMDL with every vertex moved `dx` units on x:
/// `6000.0` makes the model far in every vertex, as
/// `fmdl_vertex_far_from_origin` measures it.
fn shifted_fmdl(dx: f32) -> Vec<u8> {
    let mut model = fixture_model("konami_highneck.fmdl");
    for mesh in &mut model.meshes {
        for position in &mut mesh.vertices.positions {
            position[0] += dx;
        }
    }
    model.to_file().unwrap().write()
}

/// A face-high FMDL of the `addon_oral` fixture's mesh duplicated: two
/// meshes each requesting an anti-blur copy, the copies flagged
/// `is_antiblur_mesh` on a fuzzblock material as `encode` writes them.
fn antiblur_fmdl() -> Vec<u8> {
    let mut model = fixture_model("addon_oral.fmdl");
    model.meshes.push(model.meshes[0].clone());
    model.mesh_groups[0].meshes.push(1);
    model.extensions.antiblur = false;
    for mesh in &mut model.meshes {
        mesh.has_antiblur_meshes = true;
    }
    fmdl::ops::antiblur::encode(&mut model).unwrap();
    model.to_file().unwrap().write()
}

// EU-MDL-05
#[test]
fn common_references_become_the_bare_name_and_the_link() {
    let sandbox = Sandbox::new("mdl_common_mtl");
    sandbox.write("co Spring/Faces/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write("co Spring/Faces/XXX07 - A/face_srm.dds", b"srm");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.mtl",
        &mtl(&[
            "./face_srm.dds",
            "model/character/uniform/common/XXX/hair.dds",
            "model/character/boots/k0002/ek0062_n.dds",
        ]),
    );
    sandbox.write(
        "co Spring/Faces/XXX07 - A/realkit/apc.mtl",
        &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
    );
    sandbox.write("co Spring/Common/hair.dds", b"hair");
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // The face folder's own names stay; the Common reference becomes the
    // bare name plus the link; the other game path is kept as written.
    let face = mtl_text(&sandbox, "co Full Spring/Players/07 - A/face.mtl");
    for expected in [
        "./face_srm.dds",
        "./hair.dds",
        "model/character/boots/k0002/ek0062_n.dds",
    ] {
        assert!(face.contains(expected), "{expected}: {face}");
    }
    let apc = mtl_text(&sandbox, "co Full Spring/Players/07 - A/realkit/apc.mtl");
    assert!(apc.contains("./hair.dds"), "{apc}");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - A/hair.dds.common")).unwrap(),
        b""
    );
    assert!(
        !sandbox
            .path("co Full Spring/Players/07 - A/realkit/hair.dds.common")
            .exists()
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/hair.dds")).unwrap(),
        b"hair"
    );
}

// EU-MDL-34
#[test]
fn a_reference_whose_common_file_is_missing_is_kept_and_warned() {
    let sandbox = Sandbox::new("mdl_common_missing");
    sandbox.write("co Spring/Faces/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.mtl",
        &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
    );
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let face = mtl_text(&sandbox, "co Full Spring/Players/07 - A/face.mtl");
    assert!(face.contains("team_common_folder/hair.dds"), "{face}");
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "common_reference_kept at Faces/XXX07 - A/face.mtl (reference=model/character/uniform/common/XXX/hair.dds)"
        ),
        "{stdout}"
    );
}

// EU-MDL-35
#[test]
fn a_prefox_common_segment_of_any_name_is_the_team_s_common() {
    let sandbox = Sandbox::new("mdl_common_segment");
    // `vg` and `VG` are the team's Common as `XXX` is: the batch compiler
    // wrote a pre-Fox export's Common under its name there.
    sandbox.write("vg Spring/Faces/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write(
        "vg Spring/Faces/XXX07 - A/face.mtl",
        &mtl(&[
            "model/character/uniform/common/vg/env.dds",
            "model/character/uniform/common/VG/fbm_s.dds",
        ]),
    );
    sandbox.write("vg Spring/Common/env.dds", b"env");
    // /vg/'s records in the fixture save wear custom boots and gloves the
    // export does not carry: zero them, or `custom_model_missing` is all
    // the run reports.
    sandbox.save_edited("EDIT16", pes_version::PesVersion::Pes16, |file| {
        for slot in 1..=23 {
            if let Some(player) = file.player_mut(75800 + slot) {
                player.appearance.boots_id = 0;
                player.appearance.gloves_id = 0;
            }
        }
    });

    let run = upgrade(&sandbox, "vg Spring", "EDIT16");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    let face = mtl_text(&sandbox, "vg Full Spring/Players/07 - A/face.mtl");
    assert!(face.contains("./env.dds"), "{face}");
    assert!(face.contains("team_common_folder/fbm_s.dds"), "{face}");
    assert_eq!(
        fs::read(sandbox.path("vg Full Spring/Players/07 - A/env.dds.common")).unwrap(),
        b""
    );
    assert!(stdout.contains("common_reference_kept"), "{stdout}");
}

// EU-MDL-07
#[test]
fn a_fox_face_folder_s_fmdl_names_its_own_and_common_textures() {
    let sandbox = Sandbox::new("mdl_fox_fmdl");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/common/000/sourceimages/",
                "hair",
            ),
            (
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "skin",
            ),
        ]),
    );
    sandbox.write("co Spring/Common/hair.dds", b"hair");
    sandbox.write("co Spring/Common/skin.dds", b"skin");
    fs::write(
        sandbox.path("EDIT"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
    )
    .unwrap();

    upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl")[..2],
        [
            ("./".to_owned(), "hair".to_owned()),
            ("./".to_owned(), "skin".to_owned()),
        ]
    );
    for link in ["hair.dds.common", "skin.dds.common"] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Spring/Players/07 - A/{link}"))).unwrap(),
            b""
        );
    }
}

// EU-MDL-08
#[test]
fn a_fox_face_folder_s_common_kit_reference_links_the_set() {
    let sandbox = Sandbox::new("mdl_fox_kit_set");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[(
            "/Assets/pes16/model/character/common/000/u0XXXp0/sourceimages/",
            "armor_bsm",
        )]),
    );
    sandbox.write("co Spring/Common/u0XXXp1/armor_bsm.dds", b"armor");
    fs::write(
        sandbox.path("EDIT"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl")[0],
        ("./kitN/".to_owned(), "armor_bsm".to_owned())
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - A/kitN/armor_bsm.dds.common")).unwrap(),
        b""
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/kit1/armor_bsm.dds")).unwrap(),
        b"armor"
    );
    let stdout = run.stdout();
    assert!(!stdout.contains("common_link_missing"), "{stdout}");
}

// EU-MDL-09
#[test]
fn a_prefox_face_folder_s_kit_magic_files_references_and_face_xml_migrate() {
    let sandbox = Sandbox::new("mdl_kit_magic");
    sandbox.write("co Spring/Faces/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write("co Spring/Faces/XXX07 - A/u0XXXp1.dds", b"p1");
    sandbox.write("co Spring/Faces/XXX07 - A/u0714p2.dds", b"p2");
    sandbox.write("co Spring/Faces/XXX07 - A/u0XXXp0/armor.dds", b"armor");
    sandbox.write("co Spring/Faces/XXX07 - A/jacket_u0XXXp1.model", b"jacket");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.mtl",
        &mtl(&[
            "./u0XXXp1.dds",
            "./u0714p2.dds",
            "./u0XXXp0/armor.dds",
            "./dummy_kit.dds",
        ]),
    );
    // The folder's FMDL names the two files in `face/real/`, the armor in
    // `./u0XXXp0/`, and `dummy_kit.dds` in `common/sourceimages/`.
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "u0XXXp1.dds",
            ),
            (
                "/Assets/pes16/model/character/face/real/00000/sourceimages/",
                "u0714p2.dds",
            ),
            ("./u0XXXp0/", "armor"),
            (
                "/Assets/pes16/model/character/common/sourceimages/",
                "dummy_kit.dds",
            ),
        ]),
    );
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.xml",
        &wezlib::compress(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<config>\n    <model level=\"0\" type=\"parts\" path=\"./jacket_u0XXXp0.model\" material=\"./face.mtl\"/>\n</config>\n"
                .as_bytes(),
        ),
    );
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    upgrade(&sandbox, "co Spring", "EDIT16");
    for path in [
        "Players/07 - A/kit1.dds",
        "Players/07 - A/kit2.dds",
        "Players/07 - A/kitF/armor.dds",
        "Players/07 - A/jacket_kit1.model",
    ] {
        assert!(
            sandbox.path(&format!("co Full Spring/{path}")).exists(),
            "{path}"
        );
    }
    let set = pes_model::format::mtl::MaterialSet::read(
        &fs::read(sandbox.path("co Full Spring/Players/07 - A/face.mtl")).unwrap(),
    )
    .unwrap();
    let paths: Vec<String> = pes_model::ops::paths::texture_paths(&set)
        .iter()
        .map(|path| format!("{}{}", path.directory, path.file_name))
        .collect();
    assert_eq!(
        paths,
        [
            "./kit1.dds",
            "./kit2.dds",
            "./kitN/armor.dds",
            "./dummy_kit.dds"
        ]
    );
    let xml = fs::read(sandbox.path("co Full Spring/Players/07 - A/face.xml")).unwrap();
    assert!(!wezlib::is_wrapped(&xml));
    let text = String::from_utf8(xml).unwrap();
    assert!(text.contains("path=\"./jacket_kitN.model\""), "{text}");
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl"),
        [
            ("./".to_owned(), "kit1.dds".to_owned()),
            ("./".to_owned(), "kit2.dds".to_owned()),
            ("./kitN/".to_owned(), "armor".to_owned()),
            ("common_folder/".to_owned(), "dummy_kit.dds".to_owned()),
        ]
    );
}

// EU-MDL-31
#[test]
fn a_fox_common_keeper_file_and_reference_respell_to_gk_kit() {
    let sandbox = Sandbox::new("mdl_fox_keeper");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "u0714g1",
        )]),
    );
    // `u0XXXg1` beside `u0XXXp1` of different bytes: the keeper's set is
    // `gk_kit`, its own beside the outfielders' `kit`.
    sandbox.write("co Spring/Common/u0XXXg1.dds", b"gk");
    sandbox.write("co Spring/Common/u0XXXp1.dds", b"out");
    fs::write(
        sandbox.path("EDIT"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
    )
    .unwrap();

    upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/gk_kit1.dds")).unwrap(),
        b"gk"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/kit1.dds")).unwrap(),
        b"out"
    );
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "gk_kit1".to_owned())
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - A/gk_kit1.dds.common")).unwrap(),
        b""
    );
}

// EU-MDL-21
#[test]
fn two_files_the_old_id_rename_made_one_keep_the_last_renamed() {
    let sandbox = Sandbox::new("mdl_id_rename");
    // The rename replaced the file under its target name in listing order:
    // `u0XXX` sorts after `u0714`/`u0715` folded, so the game loaded the
    // `u0XXX` file of each pair, different bytes each.
    sandbox.write("co Spring/Faces/XXX07 - Ann/u0714p2.dds", b"older p2");
    sandbox.write("co Spring/Faces/XXX07 - Ann/u0XXXp2.dds", b"newer p2");
    sandbox.write("co Spring/Faces/XXX07 - Ann/u0715p3.dds", b"other p3");
    sandbox.write("co Spring/Faces/XXX07 - Ann/u0XXXp3.dds", b"team p3");
    fs::write(
        sandbox.path("EDIT"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(stdout.contains("migration_clean"), "{stdout}");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - Ann/kit2.dds")).unwrap(),
        b"newer p2"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - Ann/kit3.dds")).unwrap(),
        b"team p3"
    );
    for line in [
        "leftover_dropped at Faces/XXX07 - Ann/u0714p2.dds",
        "leftover_dropped at Faces/XXX07 - Ann/u0715p3.dds",
    ] {
        assert!(stdout.contains(line), "{line}: {stdout}");
    }
}

// EU-MDL-37
#[test]
fn without_a_save_the_lettered_copy_still_wins_the_old_rename_s_collision() {
    let sandbox = Sandbox::new("mdl_id_rename_loose");
    // EU-MDL-21's export, no save: the lettered `u0XXX` copy is what the old
    // compile shipped whatever ID the team had, so the loose run keeps it as
    // the saved one does.
    for (name, bytes) in [
        ("u0714p2.dds", b"older p2" as &[u8]),
        ("u0XXXp2.dds", b"newer p2"),
        ("u0715p3.dds", b"other p3"),
        ("u0XXXp3.dds", b"team p3"),
    ] {
        sandbox.write(&format!("co Spring/Faces/XXX07 - Ann/{name}"), bytes);
    }

    let input = sandbox.arg("co Spring");
    let run = sandbox.run(&["upgrade", &input, "--loose"]);
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(stdout.contains("migration_clean"), "{stdout}");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - Ann/kit2.dds")).unwrap(),
        b"newer p2"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - Ann/kit3.dds")).unwrap(),
        b"team p3"
    );
    for line in [
        "leftover_dropped at Faces/XXX07 - Ann/u0714p2.dds",
        "leftover_dropped at Faces/XXX07 - Ann/u0715p3.dds",
    ] {
        assert!(stdout.contains(line), "{line}: {stdout}");
    }
}

// EU-MDL-38
#[test]
fn numbered_copies_alone_are_unmigrated_content_save_or_none() {
    for (sandbox_name, loose) in [("mdl_id_only_save", false), ("mdl_id_only_loose", true)] {
        let sandbox = Sandbox::new(sandbox_name);
        // `u0714` beside `u0799` and no lettered copy: which one the old
        // compile shipped depended on the team's ID, and an export holding
        // two is too broken to repair by rule.
        sandbox.write("co Spring/Faces/XXX07 - Ann/u0714p2.dds", b"older p2");
        sandbox.write("co Spring/Faces/XXX07 - Ann/u0799p2.dds", b"other p2");
        let input = sandbox.arg("co Spring");
        let save = sandbox.arg("EDIT");
        if !loose {
            sandbox.save("EDIT");
        }
        let args: Vec<&str> = if loose {
            vec!["upgrade", &input, "--loose"]
        } else {
            vec!["upgrade", &input, "--savefile", &save]
        };

        let run = sandbox.run(&args);
        let stdout = run.stdout();
        assert_eq!(run.exit_code(), 1, "loose={loose}: {stdout}");
        assert!(
            stdout.contains(
                "unmigrated_content at Faces/XXX07 - Ann/u0799p2.dds (reason=path_taken, other=Faces/XXX07 - Ann/u0714p2.dds)"
            ),
            "loose={loose}: {stdout}"
        );
        sandbox.assert_nothing_written("co Full Spring", &stdout);
    }
}

// EU-MDL-39
#[test]
fn an_fmdl_s_folders_below_common_are_respelled_team_folder() {
    let sandbox = Sandbox::new("mdl_fox_common_folders");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/common/000/sourceimages/",
                "dummy_kit.dds",
            ),
            (
                "/Assets/pes16/model/character/common/ikachan/sourceimages/",
                "hair",
            ),
            ("/Assets/pes16/model/character/common/000/foo/", "shoe"),
            ("/Assets/pes16/model/character/common/sourceimages/", "zero"),
        ]),
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl")[..4],
        [
            ("team_common_folder/".to_owned(), "dummy_kit.dds".to_owned(),),
            ("team_common_folder/".to_owned(), "hair".to_owned()),
            (
                "/Assets/pes16/model/character/common/team_folder/foo/".to_owned(),
                "shoe".to_owned(),
            ),
            ("common_folder/".to_owned(), "zero".to_owned()),
        ]
    );
    // `hair`, a Common reference `Common/` lacks, is reported kept; the
    // `dummy_` stem's respell reports nothing.
    assert!(
        stdout.contains(
            "common_reference_kept at Faces/XXX07 - A/face_high.fmdl (reference=/Assets/pes16/model/character/common/ikachan/sourceimages/hair)"
        ),
        "{stdout}"
    );
    assert_eq!(
        stdout.matches("common_reference_kept").count(),
        1,
        "{stdout}"
    );
}

// EU-MDL-40
#[test]
fn a_fox_fmdl_s_other_directories_and_common_sourceimages_behave() {
    let sandbox = Sandbox::new("mdl_fox_dirs");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face_high.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/boots/k0915/sourceimages/",
                "skin",
            ),
            ("/Assets/pes16/model/character/common/sourceimages/", "zero"),
        ]),
    );
    sandbox.write("co Spring/Faces/XXX07 - A/skin.dds", b"skin");
    fs::write(
        sandbox.path("EDIT"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(
        fmdl_paths(&sandbox, "co Full Spring/Players/07 - A/face_high.fmdl")[..2],
        [
            ("./".to_owned(), "skin".to_owned()),
            ("common_folder/".to_owned(), "zero".to_owned()),
        ]
    );
    let stdout = run.stdout();
    assert!(
        !stdout.contains("common_reference_kept") && !stdout.contains("unmigrated_content"),
        "{stdout}"
    );
}

// EU-MDL-16
#[test]
fn a_prefox_common_kit_reference_links_the_common_set() {
    let sandbox = Sandbox::new("mdl_common_kit_set");
    sandbox.write("co Spring/Faces/XXX07 - A/face_high_win32.model", b"face");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.mtl",
        &mtl(&["model/character/uniform/common/XXX/u0XXXp0.dds"]),
    );
    sandbox.write("co Spring/Common/u0XXXp1.dds", b"p1");
    sandbox.write("co Spring/Common/u0XXXp2.dds", b"p2");
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/kit1.dds")).unwrap(),
        b"p1"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/kit2.dds")).unwrap(),
        b"p2"
    );
    let set = pes_model::format::mtl::MaterialSet::read(
        &fs::read(sandbox.path("co Full Spring/Players/07 - A/face.mtl")).unwrap(),
    )
    .unwrap();
    let paths: Vec<String> = pes_model::ops::paths::texture_paths(&set)
        .iter()
        .map(|path| format!("{}{}", path.directory, path.file_name))
        .collect();
    assert_eq!(paths, ["./kitN.dds"]);
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/07 - A/kitN.dds.common")).unwrap(),
        b""
    );
    let stdout = run.stdout();
    assert!(!stdout.contains("common_link_missing"), "{stdout}");
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
    assert_eq!(run.exit_code(), 0, "{stdout}");
}

// EU-MDL-17
#[test]
fn an_mtl_s_stray_text_is_dropped_and_the_file_written_back() {
    let sandbox = Sandbox::new("mdl_stray_text");
    // G Autumn 25's "72117 - Lisa" apc.mtl: a `\` typed after a <sampler/>.
    let apc = "<materialset>\r\n    <material name=\"apc\" shader=\"Apc\">\r\n        <sampler name=\"SpecularMap\" path=\"model/character/uniform/common/specular.dds\" srgb=\"1\" minfilter=\"linear\" magfilter=\"linear\" mipfilter=\"linear\" uaddr=\"wrap\" vaddr=\"wrap\" waddr=\"wrap\" maxaniso=\"2\" />\\\r\n    </material>\r\n</materialset>\r\n";
    sandbox.write("co Spring/Faces/72117 - Lisa/apc.mtl", apc.as_bytes());
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains("mtl_stray_text_dropped at Faces/72117 - Lisa/apc.mtl"),
        "{stdout}"
    );
    // The written file reads back with the strict reader: the `\` is gone,
    // the material is not.
    let written = fs::read(sandbox.path("co Full Spring/Players/17 - Lisa/apc.mtl")).unwrap();
    let set = pes_model::format::mtl::MaterialSet::read(&written).unwrap();
    let paths: Vec<String> = pes_model::ops::paths::texture_paths(&set)
        .iter()
        .map(|path| format!("{}{}", path.directory, path.file_name))
        .collect();
    assert_eq!(
        paths,
        ["model/character/uniform/common/specular.dds"],
        "{set:?}"
    );
    assert!(
        !String::from_utf8_lossy(&written).contains('\\'),
        "{}",
        String::from_utf8_lossy(&written)
    );
}

// EU-MDL-18
#[test]
fn an_mtl_that_does_not_parse_is_unmigrated_content() {
    let sandbox = Sandbox::new("mdl_unparsable");
    let broken = "<materialset>\r\n    <material name=\"apc\" shader=\"Apc\">\r\n";
    sandbox.write("co Spring/Faces/72117 - Lisa/apc.mtl", broken.as_bytes());
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "- co Spring: Error unmigrated_content at Faces/72117 - Lisa/apc.mtl (reason=mtl_unparsable, error="
        ),
        "{stdout}"
    );
    assert!(stdout.contains("migration_not_clean"), "{stdout}");
    sandbox.assert_nothing_written("co Full Spring", &stdout);
}

// EU-REP-02
#[test]
fn a_face_xml_in_a_subfolder_fails_the_strict_output_check() {
    let sandbox = Sandbox::new("rep_face_xml_subfolder");
    sandbox.write("co Spring/Faces/XXX05 - Bob/alt/face.xml", b"<config/>");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "- co Full Spring: Error file_type_disallowed at Players/05 - Bob (file=alt/face.xml)"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("migration_not_clean"), "{stdout}");
    sandbox.assert_nothing_written("co Full Spring", &stdout);
}

// EU-MDL-11
#[test]
fn prefox_model_names_move_the_leading_type_last_unless_a_face_xml_ships() {
    let sandbox = Sandbox::new("mdl_type_renames");
    sandbox.write("co Spring/Faces/XXX07 - A/gloveL_foo.model", b"gloves");
    sandbox.write("co Spring/Faces/XXX07 - A/bar_gloveR.model", b"bar");
    sandbox.write("co Spring/Faces/XXX07 - A/plain.model", b"plain");
    sandbox.write("co Spring/Faces/XXX08 - B/face.xml", b"<config/>");
    sandbox.write("co Spring/Faces/XXX08 - B/gloveL_baz.model", b"baz");
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    upgrade(&sandbox, "co Spring", "EDIT16");
    for path in [
        "Players/07 - A/foo_gloveL.model",
        "Players/07 - A/bar_gloveR.model",
        "Players/07 - A/plain.model",
        "Players/08 - B/gloveL_baz.model",
    ] {
        assert!(
            sandbox.path(&format!("co Full Spring/{path}")).exists(),
            "{path}"
        );
    }
    assert!(
        !sandbox
            .path("co Full Spring/Players/07 - A/gloveL_foo.model")
            .exists()
    );
}

// EU-MDL-12
#[test]
fn prefox_wrapped_models_and_their_mtls_and_links_rename() {
    let sandbox = Sandbox::new("mdl_wrapper_renames");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/oral_parts_body_win32.model",
        b"body",
    );
    sandbox.write("co Spring/Faces/XXX07 - A/parts_body.mtl", &mtl(&[]));
    sandbox.write("co Spring/Faces/XXX07 - A/materials.mtl", &mtl(&[]));
    sandbox.write("co Spring/Faces/XXX07 - A/gloveL_foo.model", b"gloves");
    sandbox.write("co Spring/Faces/XXX07 - A/gloveL_foo.mtl", &mtl(&[]));
    sandbox.write("co Spring/Faces/XXX07 - A/oral_cap_win32.model.common", b"");
    sandbox.write("co Spring/Common/oral_cap_win32.model", b"cap");
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    let run = upgrade(&sandbox, "co Spring", "EDIT16");
    for path in [
        "Players/07 - A/parts_body.model",
        "Players/07 - A/parts_body.mtl",
        "Players/07 - A/materials.mtl",
        "Players/07 - A/foo_gloveL.model",
        "Players/07 - A/foo_gloveL.mtl",
        "Players/07 - A/cap.model.common",
    ] {
        assert!(
            sandbox.path(&format!("co Full Spring/{path}")).exists(),
            "{path}"
        );
    }
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/cap.model")).unwrap(),
        b"cap"
    );
    let stdout = run.stdout();
    assert!(!stdout.contains("common_link_missing"), "{stdout}");
}

// EU-MDL-36
#[test]
fn a_common_file_the_face_xml_names_keeps_its_name() {
    let sandbox = Sandbox::new("mdl_common_kept");
    sandbox.write(
        "co Spring/Faces/XXX07 - A/face.xml",
        b"<config><model level=\"0\" type=\"handL\" path=\"model/character/uniform/common/XXX/oral_hand_l_*.model\" material=\"./x.mtl\"/></config>",
    );
    sandbox.write("co Spring/Faces/XXX07 - A/x.mtl", &mtl(&[]));
    sandbox.write("co Spring/Common/oral_hand_l_win32.model", b"hand");
    fs::write(
        sandbox.path("EDIT16"),
        pes_savefile::fixture_save(pes_version::PesVersion::Pes16),
    )
    .unwrap();

    upgrade(&sandbox, "co Spring", "EDIT16");
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/oral_hand_l_win32.model")).unwrap(),
        b"hand"
    );
    let xml = fs::read_to_string(sandbox.path("co Full Spring/Players/07 - A/face.xml")).unwrap();
    assert!(
        xml.contains("path=\"team_common_folder/oral_hand_l_*.model\""),
        "{xml}"
    );
}

// EU-MDL-23
#[test]
fn a_dds_beside_an_ftex_of_one_stem_is_resolved_by_use() {
    let sandbox = Sandbox::new("mdl_dds_ftex_pair");
    // VGL26 /akg/'s shape: `Common/` holds one stem twice, as `.dds` and
    // `.ftex`; a used stem keeps the `.dds`, an unused one drops both.
    sandbox.write(
        "co Spring/Faces/XXX05 - A/face_high.fmdl",
        &fmdl(&[
            (
                "/Assets/pes16/model/character/common/XXX/sourceimages/",
                "shiny.dds",
            ),
            (
                "/Assets/pes16/model/character/common/XXX/sourceimages/",
                "dummy_srm.dds",
            ),
        ]),
    );
    sandbox.write("co Spring/Common/shiny.dds", b"shiny dds");
    sandbox.write("co Spring/Common/shiny.ftex", b"shiny ftex");
    sandbox.write("co Spring/Common/dummy_srm.dds", b"dummy dds");
    sandbox.write("co Spring/Common/dummy_srm.ftex", b"dummy ftex");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    // The used stem's `.dds` is kept, its `.ftex` dropped and named.
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Common/shiny.dds")).unwrap(),
        b"shiny dds"
    );
    assert!(!sandbox.path("co Full Spring/Common/shiny.ftex").exists());
    assert!(
        stdout.contains("- co Spring: Warning texture_duplicate_dropped at Common/shiny.ftex (kept=Common/shiny.dds)"),
        "{stdout}"
    );
    // A `dummy_` stem counts as unused: both files are leftovers.
    for name in ["dummy_srm.dds", "dummy_srm.ftex"] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at Common/{name}\n"
            )),
            "{name}: {stdout}"
        );
    }
    for entry in sandbox.listing("co Full Spring") {
        assert!(!entry.contains("dummy_srm"), "{entry}");
        assert!(!entry.contains("shiny.ftex"), "{entry}");
    }
    assert!(!stdout.contains("texture_stem_conflict"), "{stdout}");
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-MDL-24
#[test]
fn a_model_far_in_every_vertex_is_dropped() {
    let sandbox = Sandbox::new("mdl_far_models");
    // Slot 02's face and hair are the template-hiding far models, 204
    // vertices each; his gloves folder's right glove is another.
    sandbox.write(
        "co Spring/Faces/XXX02 - GIANT/face_high.fmdl",
        &shifted_fmdl(6000.0),
    );
    sandbox.write(
        "co Spring/Faces/XXX02 - GIANT/hair_high.fmdl",
        &shifted_fmdl(6000.0),
    );
    sandbox.write(
        "co Spring/Gloves/g0331 - ONLY THREAD/glove_l.fmdl",
        &shifted_fmdl(0.0),
    );
    sandbox.write(
        "co Spring/Gloves/g0331 - ONLY THREAD/glove_r.fmdl",
        &shifted_fmdl(6000.0),
    );
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        crate::players::wears(file, 71402, 0, 331);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(
        sandbox
            .path("co Full Spring/Players/02 - GIANT/glove_l.fmdl")
            .exists(),
        "{stdout}"
    );
    for name in ["face_high.fmdl", "hair_high.fmdl", "glove_r.fmdl"] {
        assert!(
            !sandbox
                .path(&format!("co Full Spring/Players/02 - GIANT/{name}"))
                .exists(),
            "{name}: {stdout}"
        );
    }
    for path in [
        "Faces/XXX02 - GIANT/face_high.fmdl",
        "Faces/XXX02 - GIANT/hair_high.fmdl",
        "Gloves/g0331 - ONLY THREAD/glove_r.fmdl",
    ] {
        assert!(
            stdout.contains(&format!("model_far_dropped at {path} (vertices=204)")),
            "{path}: {stdout}"
        );
    }
    assert!(stdout.contains("migration_clean"), "{stdout}");
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

// EU-MDL-26
#[test]
fn a_folder_left_modelless_by_the_drop_hides_the_hands() {
    let sandbox = Sandbox::new("mdl_far_modelless");
    // Both gloves far: the folder is modelless, and its wearer gets the
    // hidden hands any modelless gloves folder gives.
    crate::players::face(&sandbox, "co Spring/Faces/XXX05 - Ann");
    sandbox.write(
        "co Spring/Gloves/g0331 - ONLY THREAD/glove_l.fmdl",
        &shifted_fmdl(6000.0),
    );
    sandbox.write(
        "co Spring/Gloves/g0331 - ONLY THREAD/glove_r.fmdl",
        &shifted_fmdl(6000.0),
    );
    sandbox.save_edited("EDIT", pes_version::PesVersion::Pes21, |file| {
        crate::players::wears(file, 71405, 0, 331);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(
        sandbox
            .path("co Full Spring/Players/05 - Ann/fpc_on")
            .exists(),
        "{stdout}"
    );
    assert!(!sandbox.path("co Full Spring/Gloves").exists(), "{stdout}");
    for name in ["glove_l.fmdl", "glove_r.fmdl"] {
        assert!(
            stdout.contains(&format!(
                "model_far_dropped at Gloves/g0331 - ONLY THREAD/{name}"
            )),
            "{name}: {stdout}"
        );
    }
    assert!(!stdout.contains("custom_model_"), "{stdout}");
    assert!(!stdout.contains("unmigrated_content"), "{stdout}");
}

/// The write-off's checks on the `path` written, read back and decoded:
/// two meshes, none an anti-blur copy or asking for one, the anti-blur
/// extension cleared and no fuzzblock material left.
fn antiblur_written_off(path: &std::path::Path) {
    let file = fmdl::FmdlFile::read(&fs::read(path).unwrap()).unwrap();
    let model = fmdl::Model::from_file(&file).unwrap();
    assert_eq!(model.meshes.len(), 2);
    for mesh in &model.meshes {
        assert!(!mesh.is_antiblur_mesh && !mesh.has_antiblur_meshes);
    }
    assert!(!model.extensions.antiblur);
    assert!(
        !model
            .materials
            .iter()
            .any(|material| material.shader.contains("fuzzblock")),
        "{:?}",
        model.materials
    );
}

// EU-MDL-25
#[test]
fn the_antiblur_copies_are_written_off() {
    let sandbox = Sandbox::new("mdl_antiblur");
    crate::players::face(&sandbox, "co Spring/Faces/XXX05 - Ann");
    sandbox.write(
        "co Spring/Faces/XXX05 - Ann/face_high.fmdl",
        &antiblur_fmdl(),
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(
        stdout.contains("antiblur_dropped at Faces/XXX05 - Ann/face_high.fmdl (meshes=2)"),
        "{stdout}"
    );
    antiblur_written_off(&sandbox.path("co Full Spring/Players/05 - Ann/face_high.fmdl"));
    assert!(stdout.contains("migration_clean"), "{stdout}");
}

// EU-MDL-27
#[test]
fn a_carried_fmdl_outside_a_model_folder_is_written_off_too() {
    // `Collars/` is no model folder's kind: the texture-path rewrite
    // does not reach it, but the write-off does — every `.fmdl` of the
    // export carrying the copies is written without them.
    let sandbox = Sandbox::new("mdl_antiblur_carried");
    crate::players::face(&sandbox, "co Spring/Faces/XXX05 - Ann");
    sandbox.write("co Spring/Collars/collar_120.fmdl", &antiblur_fmdl());
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(
        stdout.contains("antiblur_dropped at Collars/collar_120.fmdl (meshes=2)"),
        "{stdout}"
    );
    antiblur_written_off(&sandbox.path("co Full Spring/Collars/collar_120.fmdl"));
    assert!(stdout.contains("migration_clean"), "{stdout}");
}
