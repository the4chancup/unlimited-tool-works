use super::*;

// SE-GUI-17
#[test]
fn a_squad_file_pairs_its_records_and_writes_stats_and_aesthetics() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_squad", CO);
    let bytes = lib_fixture("pes19_squad.4ccs");
    let file = view.session.as_ref().unwrap().file();
    let document = read_squad(&bytes, file.team(CO).unwrap(), file.players()).unwrap();
    assert_eq!(
        document.tactics,
        TacticsSection::default(),
        "premise: the fixture has no tactics block"
    );
    let (expected_team, expected) = lib_import(&view, &document, CO);
    let before = rostered(&view, CO);
    assert_ne!(
        expected, before,
        "premise: the squad changes /co/'s players"
    );
    assert!(
        expected_team
            .roster
            .iter()
            .zip(&team_entry(&view, CO).roster)
            .any(|(after, before)| after.number != before.number),
        "premise: the squad changes a shirt number"
    );
    let numbers: Vec<String> = team_entry(&view, CO)
        .roster
        .iter()
        .map(|slot| slot.number.to_string())
        .collect();
    // Every record against the player at its slot.
    let team = team_entry(&view, CO);
    let rows: Vec<[String; 5]> = document
        .players
        .iter()
        .map(|(&slot, section)| {
            let entry = team.roster[usize::from(slot - 1)];
            [
                format!("{slot:02}"),
                display_name(section.name.as_deref().expect("a record's name")),
                "\u{2192}".to_owned(),
                entry.number.to_string(),
                display_name(&player_entry(&view, entry.player_id).name),
            ]
        })
        .collect();
    assert_eq!(rows.len(), 23, "premise: 23 records");
    picked(&mut window, &mut view, "pes19_squad.4ccs", &bytes);

    let lines = panel_lines(&mut window, &mut view);
    for line in [
        "A legacy squad file (.4ccs) written from a PES 2019 save.",
        "Who gets which record",
    ] {
        assert!(lines.contains(&line.to_owned()), "{line:?} in {lines:#?}");
    }
    for row in [
        [
            "01",
            "SHOTABOT 9S",
            "\u{2192}",
            &numbers[0],
            "MR. /CO/ - SKELETOR",
        ],
        [
            "03",
            ">fujo's first husbando chart",
            "\u{2192}",
            &numbers[2],
            "BLUEY",
        ],
    ] {
        assert_eq!(row_of(&mut window, &mut view, row[1]), row);
    }
    for row in &rows {
        assert_eq!(
            row_of(&mut window, &mut view, &row[1]),
            row,
            "slot {}",
            row[0]
        );
    }
    let arrows = panel_lines(&mut window, &mut view)
        .iter()
        .filter(|line| *line == "\u{2192}")
        .count();
    assert_eq!(arrows, 23, "one row per record");
    assert_eq!(checkboxes(&mut window, &mut view), ["Stats", "Aesthetics"]);

    apply(&mut window, &mut view);
    assert_eq!(rostered(&view, CO), expected, "the lib's import");
    assert_eq!(
        team_entry(&view, CO),
        expected_team,
        "the lib's team, its shirt numbers included"
    );
}

// SE-GUI-17
#[test]
fn a_squad_file_with_more_records_than_the_roster_is_refused_alone() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_squad_short", CO);
    let mut team = team_entry(&view, CO);
    team.roster[22].player_id = 0;
    view.session
        .as_mut()
        .unwrap()
        .edit_entries(Vec::new(), vec![team]);
    assert_eq!(
        rostered(&view, CO).len(),
        22,
        "premise: 22 rostered players"
    );
    picked(
        &mut window,
        &mut view,
        "pes19_squad.4ccs",
        &lib_fixture("pes19_squad.4ccs"),
    );

    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            "pes19_squad.4ccs: 23 players in the file, but the selected team has only 22: select a team with at least 23",
        ),
        "{lines:#?}"
    );
    assert!(checkboxes(&mut window, &mut view).is_empty(), "no choice");
    assert!(
        panel_node(&mut window, &mut view, Role::Button, "Apply").is_none(),
        "no Apply"
    );
}

// SE-GUI-17
#[test]
fn a_tactics_file_offers_tactics_alone_and_writes_them() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_tactics", CO);
    let bytes = lib_fixture("pes19_tactics.4cct");
    let document = read_tactics(&bytes).unwrap();
    assert_eq!(document.team.id, Some(CM), "premise: /cm/'s tactics");
    let (expected, _) = lib_import(&view, &document, CO);
    let before = team_entry(&view, CO);
    assert_ne!(
        expected.tactics, before.tactics,
        "premise: the tactics differ"
    );
    let players = rostered(&view, CO);
    picked(&mut window, &mut view, "pes19_tactics.4cct", &bytes);

    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A legacy tactics file (.4cct) written from a PES 2019 save.".to_owned())
    );
    assert_eq!(checkboxes(&mut window, &mut view), ["Gameplan"]);
    apply(&mut window, &mut view);
    assert_eq!(
        team_entry(&view, CO).tactics,
        expected.tactics,
        "the file's tactics"
    );
    assert_eq!(rostered(&view, CO), players, "the players untouched");
}

// SE-GUI-17
#[test]
fn a_texport_offers_every_section_and_names_its_version() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_texport", CO);
    picked(
        &mut window,
        &mut view,
        "pes19_texport.ted",
        &lib_fixture("pes19_texport.ted"),
    );

    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Texport written from a PES 2019 save.".to_owned())
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats", "Aesthetics", "Gameplan"]
    );
}

/// A PES 2017 Texport has no extension (`TEXPORT00000000`): the panel reads
/// it by its bytes, the Pick dialog is handed the save's version, and Apply
/// writes the file's team and players as one undoable edit.
// SE-GUI-17
#[test]
fn a_pes17_texport_without_an_extension_imports_and_undoes() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "interchange_texport_17", CO);
    let bytes = pes17_texport();
    let texport = Texport::from_bytes(&bytes, None).expect("the fixture Texport reads");
    let file_first = &texport.players()[0];
    assert_eq!(
        file_first.id,
        texport.team().roster[0].player_id,
        "premise: the file's first player is its roster's first"
    );
    let before_team = team_entry(&view, CO);
    let before_players = rostered(&view, CO);
    assert_ne!(
        before_team.name,
        texport.team().name,
        "premise: the team names differ"
    );
    assert_ne!(
        before_players[0].name, file_first.name,
        "premise: the first players' names differ"
    );

    let path = beside_save(&view, "TEXPORT00000000", &bytes);
    open_panel(&mut window, &mut view);
    let asked = std::rc::Rc::new(std::cell::Cell::new(None));
    let seen = std::rc::Rc::clone(&asked);
    view.interchange_dialog = Box::new(move |version| {
        seen.set(Some(version));
        Some(path.clone())
    });
    let button =
        panel_node(&mut window, &mut view, Role::Button, "Pick\u{2026}").expect("the Pick button");
    click(&mut window, &button, &mut view);
    assert_eq!(
        asked.get(),
        Some(PesVersion::Pes17),
        "the dialog is handed the save's version"
    );

    assert!(
        panel_lines(&mut window, &mut view)
            .contains(&"A Texport written from a PES 2017 save.".to_owned()),
        "the intro names the file's version"
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats", "Aesthetics", "Gameplan"]
    );
    assert!(
        !panel_nodes(&mut window, &mut view)
            .iter()
            .any(|node| node.role() == Role::DisclosureTriangle),
        "no note group"
    );
    apply(&mut window, &mut view);
    assert_eq!(
        team_entry(&view, CO).name,
        texport.team().name,
        "the team's name is the file's"
    );
    assert_eq!(
        rostered(&view, CO)[0].name,
        file_first.name,
        "the first player's name is the file's"
    );

    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo entry");
    assert_eq!(team_entry(&view, CO), before_team, "/co/ as before");
    assert_eq!(rostered(&view, CO), before_players, "its players as before");
}
