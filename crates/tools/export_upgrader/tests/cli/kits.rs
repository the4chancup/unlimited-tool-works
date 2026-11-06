//! The kit folders' migration into `Kits/` (the `spec/export_upgrader.md` "Kits" scenarios):
//! every old binary config becomes its slot's `config.toml`, every kit
//! texture lands under the generic `kit` name, and what cannot be placed is
//! unmigrated content.

use std::fs;

use crate::common::Sandbox;

/// `upgrade <input> --savefile <save>`.
fn upgrade(sandbox: &Sandbox, input: &str, save: &str) -> crate::common::Run {
    let input = sandbox.arg(input);
    let save = sandbox.arg(save);
    sandbox.run(&["upgrade", &input, "--savefile", &save])
}

/// The `4CCG VGL26` fixture's path below `tests/fixtures/`.
fn kit_config(ordinal: &str) -> String {
    format!("kit_configs/4CCG VGL26/XXX_DEF_{ordinal}_realUni.bin")
}

/// `bytes` decoded for PES 21 and converted to the `config.toml` text the
/// upgrader writes (texture names dropped).
fn expected_toml(bytes: &[u8]) -> String {
    let mut config = kit_config::KitConfig::decode(bytes, pes_version::PesVersion::Pes21).unwrap();
    config.source_texture_names = None;
    config.to_toml()
}

/// `co Spring`'s `co Note.txt` at `<export>/co Note.txt`, its `Kit Colours:`
/// section's entries replaced by `entries` (the lines as written, the
/// fixture's `Other Notes:` tail kept).
fn note_with_kit_entries(sandbox: &Sandbox, export: &str, entries: &[&str]) {
    let fixture = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/co Spring/co Note.txt"),
    )
    .unwrap();
    let before = fixture.find("Kit Colours:").unwrap();
    let after = fixture.find("Other Notes:").unwrap();
    let note = format!(
        "{}Kit Colours:\r\n{}\r\n\r\n{}",
        &fixture[..before],
        entries.join("\r\n"),
        &fixture[after..]
    );
    sandbox.write(&format!("{export}/co Note.txt"), note.as_bytes());
}

// EU-KIT-01
#[test]
fn kits_become_slot_folders_with_config_toml_and_generic_textures() {
    let sandbox = Sandbox::new("kit_basic");
    for (ordinal, name) in [
        ("1st", "XXX_DEF_1st_realUni.bin"),
        ("2nd", "714_def_2nd_realuni.bin"),
        ("GK1st", "XXX_DEF_GK1st_realUni.bin"),
    ] {
        sandbox.copy_fixture(&kit_config(ordinal), &format!("co Kits/Kit Configs/{name}"));
    }
    for (name, bytes) in [
        ("u0XXXp1.dds", b"p1 main" as &[u8]),
        ("u0XXXp1_back.dds", b"p1 back"),
        ("u0714p2.dds", b"p2 main"),
        ("u0XXXg1.dds", b"g1 main"),
    ] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), bytes);
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        sandbox.listing("co Full Kits/Kits"),
        [
            "g1",
            "g1/config.toml",
            "g1/kit.dds",
            "p1",
            "p1/config.toml",
            "p1/kit.dds",
            "p1/kit_back.dds",
            "p2",
            "p2/config.toml",
            "p2/kit.dds",
        ]
    );
    // The exact listing already rules a `.bin`, a `pre-fox`/`fox` marker or a
    // `Kit Configs`/`Kit Textures` folder inside `Kits/`; the output's whole
    // listing rules them anywhere.
    let output = sandbox.listing("co Full Kits");
    for entry in &output {
        assert!(!entry.ends_with(".bin"), "{entry}");
        assert!(!entry.starts_with("Kit Configs"), "{entry}");
        assert!(!entry.starts_with("Kit Textures"), "{entry}");
    }
    for (path, bytes) in [
        ("Kits/p1/kit.dds", b"p1 main" as &[u8]),
        ("Kits/p1/kit_back.dds", b"p1 back"),
        ("Kits/p2/kit.dds", b"p2 main"),
        ("Kits/g1/kit.dds", b"g1 main"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/{path}"))).unwrap(),
            bytes,
            "{path}"
        );
    }
}

// EU-KIT-02
#[test]
fn a_config_toml_carries_the_undecoded_bits_and_no_texture_names() {
    let sandbox = Sandbox::new("kit_unknown_bits");
    sandbox.copy_fixture(
        &kit_config("1st"),
        "co Kits/Kit Configs/XXX_DEF_1st_realUni.bin",
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let text = fs::read_to_string(sandbox.path("co Full Kits/Kits/p1/config.toml")).unwrap();
    let parsed: toml::Table = toml::from_str(&text).unwrap();
    assert!(!parsed.contains_key("source_texture_names"), "{text}");
    // The fixture's set undecoded bit survives: offset 0x17 = 4.
    assert_eq!(parsed["unknown"]["0x17"], toml::Value::Integer(4), "{text}");
    // And every other bit is kept too: the TOML parses back to the decoded
    // config with only the texture names dropped.
    let fixture = include_bytes!("../fixtures/kit_configs/4CCG VGL26/XXX_DEF_1st_realUni.bin");
    let mut expected =
        kit_config::KitConfig::decode(fixture, pes_version::PesVersion::Pes21).unwrap();
    expected.source_texture_names = None;
    assert_eq!(kit_config::KitConfig::from_toml(&text).unwrap(), expected);
}

// EU-KIT-08
#[test]
fn textures_no_kit_row_names_are_leftovers_in_a_full_export() {
    let sandbox = Sandbox::new("kit_texture_orphan");
    for ordinal in ["1st", "2nd", "3rd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    sandbox.copy_fixture("co Spring/co Note.txt", "co Kits/co Note.txt");
    for (name, bytes) in [
        ("u0XXXp5.dds", b"p5 main" as &[u8]),
        ("u0XXXp2 - Copy.dds", b"p2 copy"),
        ("back.dds", b"back"),
        ("u0XXXp3.png", b"p3 png"),
    ] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), bytes);
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for path in [
        "Kit Textures/u0XXXp5.dds",
        "Kit Textures/u0XXXp2 - Copy.dds",
        "Kit Textures/back.dds",
        "Kit Textures/u0XXXp3.png",
    ] {
        assert!(
            stdout.contains(&format!("leftover_dropped at {path}")),
            "{path}: {stdout}"
        );
    }
    assert!(!sandbox.path("co Full Kits/Kits/p5").exists());
    // No `Kits/p5/` and no copy of the four anywhere in the output.
    for entry in sandbox.listing("co Full Kits") {
        let name = entry.rsplit('/').next().unwrap_or(entry.as_str());
        assert!(
            ![
                "u0XXXp5.dds",
                "u0XXXp2 - Copy.dds",
                "back.dds",
                "u0XXXp3.png"
            ]
            .contains(&name),
            "{entry}"
        );
    }
}

// EU-KIT-10
#[test]
fn configs_in_a_kit_configs_subfolder_migrate() {
    let sandbox = Sandbox::new("kit_config_subfolder");
    for ordinal in ["1st", "2nd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/714/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let first = include_bytes!("../fixtures/kit_configs/4CCG VGL26/XXX_DEF_1st_realUni.bin");
    let second = include_bytes!("../fixtures/kit_configs/4CCG VGL26/XXX_DEF_2nd_realUni.bin");
    for (slot, bytes) in [("p1", &first[..]), ("p2", &second[..])] {
        assert_eq!(
            fs::read_to_string(sandbox.path(&format!("co Full Kits/Kits/{slot}/config.toml")))
                .unwrap(),
            expected_toml(bytes),
            "{slot}"
        );
    }
}

// EU-KIT-16
#[test]
fn a_texture_in_a_kit_textures_subfolder_is_a_leftover() {
    let sandbox = Sandbox::new("kit_texture_subfolder");
    // EU-KIT-10's config, which migrates, beside the texture a subfolder
    // hides: the run stays clean.
    sandbox.copy_fixture(
        &kit_config("1st"),
        "co Kits/Kit Configs/714/XXX_DEF_1st_realUni.bin",
    );
    sandbox.write("co Kits/Kit Textures/714/u0XXXp1.dds", b"p1 main");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    let stdout = run.stdout();
    assert_eq!(run.exit_code(), 0, "{stdout}");
    assert!(
        stdout.contains("leftover_dropped at Kit Textures/714/u0XXXp1.dds"),
        "{stdout}"
    );
    assert_eq!(sandbox.listing("co Full Kits/Kits/p1"), ["config.toml"]);
}

// EU-KIT-12
#[test]
fn a_second_goalkeeper_kits_config_and_texture_are_leftovers() {
    let sandbox = Sandbox::new("kit_second_gk");
    for (ordinal, name) in [
        ("1st", "XXX_DEF_1st_realUni.bin"),
        ("GK1st", "XXX_DEF_GK1st_realUni.bin"),
        ("GK1st", "XXX_DEF_GK2nd_realUni.bin"),
    ] {
        sandbox.copy_fixture(&kit_config(ordinal), &format!("co Kits/Kit Configs/{name}"));
    }
    sandbox.write("co Kits/Kit Textures/u0XXXg1.dds", b"g1 main");
    sandbox.write("co Kits/Kit Textures/u0XXXg2.dds", b"g2 main");
    note_with_kit_entries(
        &sandbox,
        "co Kits",
        &["- 1st player: 1 2 3 - 4 5 6", "- 1st GK: 7 8 9 - 10 11 12"],
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for path in [
        "Kit Configs/XXX_DEF_GK2nd_realUni.bin",
        "Kit Textures/u0XXXg2.dds",
    ] {
        assert!(
            stdout.contains(&format!("leftover_dropped at {path}")),
            "{path}: {stdout}"
        );
    }
    assert_eq!(
        sandbox.listing("co Full Kits/Kits/g1"),
        ["colors.txt", "config.toml", "kit.dds"]
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/g1/kit.dds")).unwrap(),
        b"g1 main"
    );
    // No second goalkeeper kit folder anywhere.
    for entry in sandbox.listing("co Full Kits") {
        assert!(!entry.contains("g2"), "{entry}");
    }
}

// EU-KIT-03
#[test]
fn the_notes_kit_entries_give_each_kits_colors_and_icon() {
    let sandbox = Sandbox::new("kit_note_colors");
    for (ordinal, name) in [
        ("1st", "XXX_DEF_1st_realUni.bin"),
        ("2nd", "714_def_2nd_realuni.bin"),
        ("GK1st", "XXX_DEF_GK1st_realUni.bin"),
    ] {
        sandbox.copy_fixture(&kit_config(ordinal), &format!("co Kits/Kit Configs/{name}"));
    }
    note_with_kit_entries(
        &sandbox,
        "co Kits",
        &[
            "- 1st player: 211 74 79 - 162 62 77 - 3",
            "- 2nd player: 30 30 30 - 250 250 250 - 5",
            "- 1st GK: 30 30 30 - 60 60 60",
        ],
    );
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p1/colors.txt")).unwrap(),
        b"211 74 79\n162 62 77\n"
    );
    assert!(
        sandbox
            .listing("co Full Kits/Kits/p1")
            .iter()
            .all(|entry| !entry.starts_with("icon_")),
        "{}",
        run.stdout()
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p2/colors.txt")).unwrap(),
        b"30 30 30\n250 250 250\n"
    );
    assert!(sandbox.path("co Full Kits/Kits/p2/icon_5").is_file());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/g1/colors.txt")).unwrap(),
        b"30 30 30\n60 60 60\n"
    );
}

// EU-KIT-04
#[test]
fn a_second_gk_entry_is_a_leftover_and_a_tenth_player_entry_unmigrated() {
    let sandbox = Sandbox::new("kit_entries_overflow");
    let mut entries: Vec<String> = (1..=9)
        .map(|n| {
            format!(
                "- {n}{} player: {n} {n} {n} - {} {} {}",
                if n == 1 {
                    "st"
                } else if n == 2 {
                    "nd"
                } else if n == 3 {
                    "rd"
                } else {
                    "th"
                },
                n + 1,
                n + 1,
                n + 1
            )
        })
        .collect();
    // The losing entries' colors are distinct from every given kit's.
    entries.push("- 10th player: 100 100 100 - 101 101 101".to_owned());
    entries.push("- 1st GK: 11 11 11 - 12 12 12".to_owned());
    entries.push("- 2nd GK: 200 200 200 - 201 201 201".to_owned());
    let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
    note_with_kit_entries(&sandbox, "co Kits", &refs);
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    let stdout = run.stdout();
    assert!(
        stdout.contains(
            "unmigrated_content at co Note.txt (reason=kit_entry_past_ninth, line=- 10th player: 100 100 100 - 101 101 101)"
        ),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("leftover_dropped at co Note.txt (line=- 2nd GK: 200 200 200 - 201 201 201)"),
        "{stdout}"
    );
    sandbox.assert_nothing_written("co Full Kits", &stdout);
}

// EU-KIT-09
#[test]
fn a_kit_entry_without_a_config_still_forms_its_kit() {
    let sandbox = Sandbox::new("kit_without_config");
    for ordinal in ["1st", "2nd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    sandbox.copy_fixture("co Spring/co Note.txt", "co Kits/co Note.txt");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // The third entry's slot has no config: the folder holds its colors
    // alone.
    assert_eq!(sandbox.listing("co Full Kits/Kits/p3"), ["colors.txt"]);
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p3/colors.txt")).unwrap(),
        b"255 000 000\n255 255 255\n"
    );
    assert!(
        run.stdout()
            .contains("kit_without_config at co Note.txt (slot=p3)"),
        "{}",
        run.stdout()
    );
}

// EU-KIT-11
#[test]
fn an_entry_after_other_notes_gives_no_kit() {
    let sandbox = Sandbox::new("kit_after_notes");
    for ordinal in ["1st", "2nd", "3rd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    // The kit entries, then `- 3rd player:` as the first line after `Other
    // Notes:` — the reading ends there, so it gives no kit.
    let fixture = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/co Spring/co Note.txt"),
    )
    .unwrap();
    let before = fixture.find("Kit Colours:").unwrap();
    let note = format!(
        "{}Kit Colours:\r\n{}\r\n\r\nOther Notes:\r\n- 3rd player: 1 2 3 - 4 5 6\r\n",
        &fixture[..before],
        [
            "- 1st player: #D34A4F - #A23E4D",
            "- 2nd player: 30 30 30 - 250 250 250",
        ]
        .join("\r\n"),
    );
    sandbox.write("co Kits/co Note.txt", note.as_bytes());
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p1/colors.txt")).unwrap(),
        b"#D34A4F\n#A23E4D\n"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p2/colors.txt")).unwrap(),
        b"30 30 30\n250 250 250\n"
    );
    // No third player entry: p3 holds its config alone.
    assert_eq!(sandbox.listing("co Full Kits/Kits/p3"), ["config.toml"]);
    assert!(!sandbox.path("co Full Kits/Kits/p4").exists());
    // No colors.txt holds the line read after `Other Notes:`.
    for entry in sandbox.listing("co Full Kits/Kits") {
        if entry.ends_with("colors.txt") {
            let text =
                fs::read_to_string(sandbox.path(&format!("co Full Kits/Kits/{entry}"))).unwrap();
            assert!(!text.contains("1 2 3"), "{entry}: {text}");
        }
    }
}

// EU-KIT-17
#[test]
fn an_entry_whose_colors_do_not_read_is_unmigrated_content() {
    let sandbox = Sandbox::new("kit_bad_entry");
    for ordinal in ["1st", "2nd", "3rd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    // `- 2nd player: #RRGGBB - #RRGGBB`, the AET template's own line.
    note_with_kit_entries(&sandbox, "co Kits", &["- 2nd player: #RRGGBB - #RRGGBB"]);
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "unmigrated_content at co Note.txt (reason=colors_unreadable, line=- 2nd player: #RRGGBB - #RRGGBB)"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Kits", &run.stdout());
}

// EU-KIT-13
#[test]
fn a_config_without_a_kit_entry_still_forms_its_kit() {
    let sandbox = Sandbox::new("kit_config_without_entry");
    for ordinal in ["1st", "2nd", "3rd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    // A fourth player config no Note entry gives.
    sandbox.copy_fixture(
        "kit_configs/MGGG VGL26/XXX_DEF_4th_realUni.bin",
        "co Kits/Kit Configs/XXX_DEF_4th_realUni.bin",
    );
    sandbox.copy_fixture("co Spring/co Note.txt", "co Kits/co Note.txt");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(sandbox.listing("co Full Kits/Kits/p4"), ["config.toml"]);
    assert!(
        run.stdout()
            .contains("config_without_kit_entry at Kit Configs/XXX_DEF_4th_realUni.bin (slot=p4)"),
        "{}",
        run.stdout()
    );
}

// EU-KIT-05
#[test]
fn fields_naming_another_kits_textures_get_the_copies() {
    let sandbox = Sandbox::new("kit_texture_copied");
    for (fixture, name) in [
        (
            "kit_configs/MGGG VGL26/XXX_DEF_1st_realUni.bin",
            "XXX_DEF_1st_realUni.bin",
        ),
        (
            "kit_configs/MGGG VGL26/XXX_DEF_2nd_realUni.bin",
            "XXX_DEF_2nd_realUni.bin",
        ),
        (
            "kit_configs/4CCG VGL26/XXX_DEF_3rd_realUni.bin",
            "XXX_DEF_3rd_realUni.bin",
        ),
    ] {
        sandbox.copy_fixture(fixture, &format!("co Kits/Kit Configs/{name}"));
    }
    for (name, bytes) in [
        ("u0XXXp1.dds", b"p1 main" as &[u8]),
        ("u0XXXp1_back.dds", b"p1 back"),
        ("u0XXXp1_name.dds", b"p1 name"),
        ("u0XXXp2.dds", b"p2 main"),
        ("u0XXXp3.dds", b"p3 main"),
        ("u0XXXp3_back.dds", b"p3 back"),
        ("u0XXXp3_name.dds", b"p3 name"),
    ] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), bytes);
    }
    sandbox.copy_fixture("co Spring/co Note.txt", "co Kits/co Note.txt");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // The 2nd's back, leg and name fields all name kit 1's: copies of p1's.
    for (name, bytes) in [
        ("kit_back.dds", b"p1 back" as &[u8]),
        ("kit_leg.dds", b"p1 back"),
        ("kit_name.dds", b"p1 name"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/Kits/p2/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    let stdout = run.stdout();
    for (stem, from) in [
        ("kit_back", "p1/kit_back"),
        ("kit_leg", "p1/kit_back"),
        ("kit_name", "p1/kit_name"),
    ] {
        assert!(
            stdout.contains(&format!(
                "kit_texture_copied at Kit Configs/XXX_DEF_2nd_realUni.bin (kit=p2, stem={stem}, from={from})"
            )),
            "{stem}: {stdout}"
        );
    }
    // The 3rd names its own and keeps what it shipped.
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p3/kit_back.dds")).unwrap(),
        b"p3 back"
    );
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/p3/kit_name.dds")).unwrap(),
        b"p3 name"
    );
}

// EU-KIT-06
#[test]
fn a_texture_every_kit_shares_byte_identically_moves_to_all() {
    let sandbox = Sandbox::new("kit_hoist");
    for ordinal in ["1st", "2nd", "GK1st"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    for (name, bytes) in [
        ("u0XXXp1.dds", b"p1 main" as &[u8]),
        ("u0XXXp2.dds", b"p2 main"),
        ("u0XXXg1.dds", b"g1 main"),
        ("u0XXXp1_back.dds", b"shared back"),
        ("u0XXXp2_back.dds", b"shared back"),
        ("u0XXXg1_back.dds", b"shared back"),
        ("u0XXXp1_name.dds", b"shared name"),
        ("u0XXXp2_name.dds", b"shared name"),
    ] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), bytes);
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/all/kit_back.dds")).unwrap(),
        b"shared back"
    );
    // No kit folder holds `kit_back.dds` anymore; `kit_name`, absent from
    // g1, stays where it was.
    for slot in ["p1", "p2", "g1"] {
        assert!(
            !sandbox
                .path(&format!("co Full Kits/Kits/{slot}/kit_back.dds"))
                .exists(),
            "{slot}"
        );
    }
    for slot in ["p1", "p2"] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/Kits/{slot}/kit_name.dds"))).unwrap(),
            b"shared name",
            "{slot}"
        );
    }
    assert!(
        run.stdout().contains("kit_texture_hoisted (stem=kit_back)"),
        "{}",
        run.stdout()
    );
}

// EU-KIT-07
#[test]
fn a_copied_texture_moves_to_all_when_every_kit_now_holds_it() {
    let sandbox = Sandbox::new("kit_copied_hoist");
    for ordinal in ["1st", "2nd", "GK1st"] {
        sandbox.copy_fixture(
            &format!("kit_configs/MGGG VGL26/XXX_DEF_{ordinal}_realUni.bin"),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    for (name, bytes) in [
        ("u0XXXp1.dds", b"p1 main" as &[u8]),
        ("u0XXXp1_back.dds", b"p1 back"),
        ("u0XXXp2.dds", b"p2 main"),
        ("u0XXXg1.dds", b"g1 main"),
    ] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), bytes);
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // p2 and g1's back and leg fields name kit 1's back: copies — and then
    // `kit_back` and `kit_leg`, held byte-identically by every kit, hoist.
    assert_eq!(
        fs::read(sandbox.path("co Full Kits/Kits/all/kit_back.dds")).unwrap(),
        b"p1 back"
    );
    assert_eq!(
        sandbox.listing("co Full Kits/Kits/all"),
        ["kit_back.dds", "kit_leg.dds"]
    );
    for slot in ["p1", "p2", "g1"] {
        assert!(
            !sandbox
                .path(&format!("co Full Kits/Kits/{slot}/kit_back.dds"))
                .exists(),
            "{slot}"
        );
    }
    let stdout = run.stdout();
    assert!(stdout.contains("kit_texture_copied"), "{stdout}");
    assert!(stdout.contains("kit_texture_hoisted"), "{stdout}");
}

// EU-KIT-14
#[test]
fn the_empty_g1_of_a_full_export_stops_the_hoist() {
    let sandbox = Sandbox::new("kit_no_gk");
    for ordinal in ["1st", "2nd"] {
        sandbox.copy_fixture(
            &kit_config(ordinal),
            &format!("co Kits/Kit Configs/XXX_DEF_{ordinal}_realUni.bin"),
        );
    }
    for name in ["u0XXXp1_back.dds", "u0XXXp2_back.dds"] {
        sandbox.write(&format!("co Kits/Kit Textures/{name}"), b"shared back");
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    // `co Full Kits` is a Full export: its empty `g1` counts, so `kit_back`
    // is not in every kit folder and nothing hoists.
    for slot in ["p1", "p2"] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/Kits/{slot}/kit_back.dds"))).unwrap(),
            b"shared back",
            "{slot}"
        );
    }
    assert!(!sandbox.path("co Full Kits/Kits/all").exists());
    assert!(
        !run.stdout().contains("kit_texture_hoisted"),
        "{}",
        run.stdout()
    );
}

/// A small BC1 DDS of one flat color, made the way `dds_convert`'s own
/// tests make a source (`Decoded` by hand, `encode_dds`).
fn flat_dds(color: [u8; 4]) -> Vec<u8> {
    let decoded = dds_convert::Decoded {
        width: 4,
        height: 4,
        mips: vec![[color; 4 * 4].concat()],
        blocks: None,
        authored_mips: false,
        uncompressed: false,
    };
    dds_convert::encode_dds(&decoded, dds_convert::BlockCodec::Bc1).unwrap()
}

// EU-KIT-15
#[test]
fn ftex_kit_textures_are_converted_to_the_dds_they_hold() {
    let sandbox = Sandbox::new("kit_ftex");
    sandbox.copy_fixture(
        &kit_config("1st"),
        "co Kits/Kit Configs/XXX_DEF_1st_realUni.bin",
    );
    let main_ftex =
        ftex::dds_to_ftex(&flat_dds([200, 30, 30, 255]), ftex::ColorSpace::Srgb).unwrap();
    let back_ftex =
        ftex::dds_to_ftex(&flat_dds([30, 30, 200, 255]), ftex::ColorSpace::Srgb).unwrap();
    sandbox.write("co Kits/Kit Textures/u0XXXp1.ftex", &main_ftex);
    sandbox.write("co Kits/Kit Textures/u0XXXp1_back.ftex", &back_ftex);
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    for (output, input) in [
        ("Kits/p1/kit.dds", &main_ftex),
        ("Kits/p1/kit_back.dds", &back_ftex),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("co Full Kits/{output}"))).unwrap(),
            ftex::ftex_to_dds(input).unwrap(),
            "{output}"
        );
    }
}

// EU-KIT-18
#[test]
fn an_ftex_second_to_a_dds_of_its_stem_is_unmigrated_content() {
    let sandbox = Sandbox::new("kit_ftex_second");
    sandbox.copy_fixture(
        &kit_config("2nd"),
        "co Kits/Kit Configs/XXX_DEF_2nd_realUni.bin",
    );
    let other_ftex =
        ftex::dds_to_ftex(&flat_dds([30, 200, 30, 255]), ftex::ColorSpace::Srgb).unwrap();
    sandbox.write("co Kits/Kit Textures/u0XXXp2.dds", b"p2 main");
    sandbox.write("co Kits/Kit Textures/u0XXXp2.ftex", &other_ftex);
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "unmigrated_content at Kit Textures/u0XXXp2.ftex (reason=texture_name_taken, other=Kit Textures/u0XXXp2.dds)"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Kits", &run.stdout());
}

// EU-KIT-18
#[test]
fn a_dds_wins_its_name_over_an_ftex_whatever_the_input_order() {
    let sandbox = Sandbox::new("kit_ftex_loses");
    sandbox.copy_fixture(
        &kit_config("1st"),
        "co Kits/Kit Configs/XXX_DEF_1st_realUni.bin",
    );
    // `u0XXXp1.FTEX` sorts before `u0XXXp1.dds` byte-wise; the DDS still
    // takes the output name, so the FTEX is the one reported.
    let ftex = ftex::dds_to_ftex(&flat_dds([9, 9, 9, 255]), ftex::ColorSpace::Srgb).unwrap();
    sandbox.write("co Kits/Kit Textures/u0XXXp1.FTEX", &ftex);
    sandbox.write("co Kits/Kit Textures/u0XXXp1.dds", b"p1 main");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Kits", "EDIT");
    assert_eq!(run.exit_code(), 1, "{}", run.stdout());
    assert!(
        run.stdout().contains(
            "unmigrated_content at Kit Textures/u0XXXp1.FTEX (reason=texture_name_taken, other=Kit Textures/u0XXXp1.dds)"
        ),
        "{}",
        run.stdout()
    );
    sandbox.assert_nothing_written("co Full Kits", &run.stdout());
}

// EU-KIT-19
#[test]
fn a_midcup_texture_of_a_slot_no_row_names_is_that_kits_update() {
    let sandbox = Sandbox::new("kit_midcup_texture_only");
    sandbox.copy_fixture(
        &kit_config("1st"),
        "co Midcup Day 5/Kit Configs/XXX_DEF_1st_realUni.bin",
    );
    note_with_kit_entries(
        &sandbox,
        "co Midcup Day 5",
        &["- 1st player: 1 2 3 - 4 5 6"],
    );
    for (name, bytes) in [
        ("u0XXXp1.dds", b"p1 main" as &[u8]),
        ("u0XXXp3.dds", b"p3 main"),
        ("u0XXXp3_back.dds", b"p3 back"),
        ("u0XXXg2.dds", b"g2 main"),
        ("back.dds", b"back"),
    ] {
        sandbox.write(&format!("co Midcup Day 5/Kit Textures/{name}"), bytes);
    }
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Midcup Day 5", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let output = "co Midcup Day 5 upgraded";
    // p3's kit folder holds its textures alone: no `config.toml` (the
    // slot's installed config stays), no `colors.txt`.
    assert_eq!(
        sandbox.listing(&format!("{output}/Kits/p3")),
        ["kit.dds", "kit_back.dds"]
    );
    for (name, bytes) in [
        ("kit.dds", b"p3 main" as &[u8]),
        ("kit_back.dds", b"p3 back"),
    ] {
        assert_eq!(
            fs::read(sandbox.path(&format!("{output}/Kits/p3/{name}"))).unwrap(),
            bytes,
            "{name}"
        );
    }
    let stdout = run.stdout();
    for path in ["Kit Textures/u0XXXg2.dds", "Kit Textures/back.dds"] {
        assert!(
            stdout.contains(&format!("leftover_dropped at {path}")),
            "{path}: {stdout}"
        );
    }
    assert!(!sandbox.path(&format!("{output}/Kits/g2")).exists());
    // No finding names p3 or its textures.
    assert!(!stdout.contains("p3"), "{stdout}");
}
