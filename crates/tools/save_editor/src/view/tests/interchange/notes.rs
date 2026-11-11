use super::*;

// SE-GUI-16
#[test]
fn a_file_of_another_version_lists_its_notes_in_words() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "interchange_notes", CO);
    let mut source = SaveView::new();
    source.open(&fixture(PesVersion::Pes21, "interchange_notes_source"));
    let path = picked(
        &mut window,
        &mut view,
        "co21.toml",
        export(&source, CO).as_bytes(),
    );

    // The command line's import of the same document onto the same save.
    let save = view.session.as_ref().unwrap().path().to_owned();
    let out = save.with_file_name("OUT");
    let stdout = cli_stdout(
        save.parent().unwrap(),
        &[
            "import-toml",
            path.to_str().unwrap(),
            "--base",
            save.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
    );
    let expected: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("- Warning import_"))
        .collect();
    assert!(
        expected
            .iter()
            .any(|line| line.contains("(path=team.edit_flags.stadium)")),
        "premise: the command line notes the stadium flag: {expected:#?}"
    );

    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats", "Aesthetics", "Gameplan"]
    );
    let lines = panel_lines(&mut window, &mut view);
    for line in [
        "A Team TOML written from a PES 2021 save, converted to PES 2017 where it can be.",
        "Import notes (PES 2021 file onto a PES 2017 save)",
    ] {
        assert!(lines.contains(&line.to_owned()), "{line:?} in {lines:#?}");
    }

    // The catalog's text once per code the command line printed, none for
    // another code.
    let catalog = [
        (
            "import_key_not_in_version",
            "The team file holds a key the file's version stores but the save's does not. \
             The key is dropped from the import, but the rest is written.",
        ),
        (
            "import_value_not_encodable",
            "The team file holds a value the save's version cannot store: an instruction or a \
             playing style it lacks. The value is written as off (a playing style as None), and \
             the rest of the import is written.",
        ),
        (
            "import_value_capped",
            "The team file holds a value above what the save's version stores: a face type \
             above its cap, a motion above its last, or a custom skin it has no slot for. The \
             value is lowered (the face type to 0, the motion to its last, the skin to 1), and \
             the rest of the import is written.",
        ),
        (
            "import_slot_empty",
            "The team file holds a player for a roster place the team leaves empty. The player \
             is skipped, but the rest is written.",
        ),
        (
            "import_text_cropped",
            "The team file holds a name or shirt name longer than the save's version has room \
             for. The text is cut to what the line shows, and the rest of the import is written.",
        ),
    ];
    for (code, text) in catalog {
        let printed = expected
            .iter()
            .any(|line| line.starts_with(&format!("- Warning {code} ")));
        let drawn = lines.iter().filter(|line| *line == text).count();
        assert_eq!(drawn, usize::from(printed), "{code}'s text in {lines:#?}");
    }

    // A team key's group is its subject alone, in the warning colour.
    assert!(
        lines.contains(&"Stadium edit flag".to_owned()),
        "Stadium edit flag in {lines:#?}"
    );
    let warn = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .warn_fg_color;
    assert_eq!(
        painted(&mut window, &mut view, "Stadium edit flag"),
        warn,
        "a team key in the warning colour"
    );
    let mut subject_only: Vec<String> = expected
        .iter()
        .filter(|line| line.contains("(path=team.") || line.contains("(path=tactics."))
        .map(|line| note_group(line))
        .collect();
    subject_only.sort_unstable();
    subject_only.dedup();
    let labels: Vec<String> = panel_nodes(&mut window, &mut view)
        .into_iter()
        .filter(|node| node.role() == Role::Label)
        .filter_map(|node| node.value().map(str::to_owned))
        .collect();
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let warned: Vec<&String> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if galley_color(text) == warn => Some(&text.galley.job.text),
            _ => None,
        })
        .filter(|text| labels.contains(text))
        .collect();
    assert_eq!(
        warned.len(),
        subject_only.len(),
        "one subject in the warning colour per team or tactics key: {warned:#?} for {subject_only:#?}"
    );

    // One collapsed header per players' group, `<subject> · <n> players`,
    // its count the command line's notes of that key.
    let mut groups: Vec<(String, usize)> = Vec::new();
    for line in expected.iter().filter(|line| !line.contains("(path=team.")) {
        let key = note_group(line);
        if key.contains(" tactics.") {
            continue;
        }
        match groups.iter_mut().find(|(group, _)| *group == key) {
            Some((_, count)) => *count += 1,
            None => groups.push((key, 1)),
        }
    }
    let (stat, players) = groups
        .iter()
        .find_map(|(key, count)| {
            let stat = key.strip_prefix("import_key_not_in_version players.*.stats.")?;
            Some((stat.to_owned(), *count))
        })
        .expect("premise: a stat the save's version does not store");
    assert!(players > 1, "premise: several players' {stat}");
    let headers = note_headers(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes_in_window(&pairs, TITLE)
            .into_iter()
            .filter(|(_, node)| node.role() == Role::DisclosureTriangle)
            .all(|(_, node)| node.is_expanded() != Some(true)),
        "every group starts collapsed"
    );
    let stat_header = format!(
        "{} \u{b7} {players} players",
        pes_savefile::interchange::team_toml::labels::stat_name(PesVersion::Pes17, &stat)
    );
    assert!(
        headers.contains(&stat_header),
        "{stat_header:?} in {headers:#?}"
    );
    let mut shown_counts: Vec<usize> = headers
        .iter()
        .map(|header| header_players(header))
        .collect();
    let mut counts: Vec<usize> = groups.iter().map(|(_, count)| *count).collect();
    shown_counts.sort_unstable();
    counts.sort_unstable();
    assert_eq!(shown_counts, counts, "one header per players' group");

    // Opened, no line speaks the command line's words.
    open_note_groups(&mut window, &mut view);
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("import_") || line.contains("path=")),
        "no code or key path drawn: {lines:#?}"
    );

    // The top players' group's first row: the place of the command line's
    // first player note, the target's player at that place, no detail.
    let place = expected
        .iter()
        .find_map(|line| line.split_once("(path=players.")?.1.split_once('.'))
        .map(|(place, _)| place.to_owned())
        .expect("premise: a player's note");
    let at: usize = place.parse().expect("a place");
    let player = team_entry(&view, CO).roster[at - 1].player_id;
    let name = pes_savefile::model::names::display_name(&player_entry(&view, player).name);
    assert_eq!(
        row_of(&mut window, &mut view, &place),
        [place.clone(), name]
    );
}

// A tactics key's group names what its notes carry after its subject, on
// its one line: an instruction PES 17 lacks, written off, is named there
// or nowhere.
// SE-GUI-68
#[test]
fn a_tactics_keys_note_names_the_instruction_written_off() {
    use pes_savefile::model::instruction::Instruction;
    use pes_savefile::model::tactics::AdvancedInstruction;
    use pes_savefile::schema::instruction::encode;

    let (mut view, mut window) = open_on(PesVersion::Pes17, "interchange_note_instruction", CO);
    let mut source = SaveView::new();
    source.open(&fixture(
        PesVersion::Pes21,
        "interchange_note_instruction_source",
    ));
    assert_eq!(
        encode(PesVersion::Pes17, Instruction::WingBack),
        None,
        "premise: PES 17 has no Wing Back"
    );
    let mut team = team_entry(&source, CO);
    let stored = |instruction| AdvancedInstruction {
        instruction: encode(PesVersion::Pes21, instruction).expect("a PES 21 instruction"),
        player: 0,
    };
    team.tactics.presets[0].attack_instructions =
        Some([stored(Instruction::WingBack), stored(Instruction::Off)]);
    source.session.as_mut().unwrap().edit_team(1, team);
    picked(
        &mut window,
        &mut view,
        "co21.toml",
        export(&source, CO).as_bytes(),
    );

    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.contains(&"Preset 1 attacking instructions: Wing Back".to_owned()),
        "the subject and the instruction on one line: {lines:#?}"
    );
}

// A note that carries a detail draws it on its player's row: the opened
// group's first row is the place, the target's player at that place and
// the lowered value.
// SE-GUI-68
#[test]
fn an_opened_groups_row_draws_the_place_the_player_and_the_detail() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "interchange_note_detail", CO);
    picked(&mut window, &mut view, SQUAD_19, &lib_fixture(SQUAD_19));
    let header = "Goal celebration 1 motion \u{b7} 23 players";
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let id = nodes_in_window(&pairs, TITLE)
        .into_iter()
        .find(|(_, node)| node.role() == Role::DisclosureTriangle && node.label() == Some(header))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("the {header:?} group"));
    action_click(&mut window, &mut view, id);

    // The group's container holds its header and, opened, its rows: each
    // row a container of labels. Read from the tree, since a row scrolled
    // out of the pairing list keeps its node where it would stand.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let (_, group) = nodes_in_window(&pairs, TITLE)
        .into_iter()
        .find(|(_, node)| node.children().contains(&id))
        .expect("the group's container");
    let mut stack: Vec<accesskit::NodeId> = group.children().iter().rev().copied().collect();
    let mut row = Vec::new();
    while let Some(next) = stack.pop() {
        let children = by_id[&next].children();
        if !children.is_empty()
            && children
                .iter()
                .all(|child| by_id[child].role() == Role::Label)
        {
            row = children
                .iter()
                .filter_map(|child| by_id[child].value())
                .collect();
            break;
        }
        stack.extend(children.iter().rev());
    }
    let player = team_entry(&view, CO).roster[0].player_id;
    let name = pes_savefile::model::names::display_name(&player_entry(&view, player).name);
    assert_eq!(row, ["01", name.as_str(), "127 \u{2192} 122"]);
}

// The pairing list and the note groups share the room above Apply, so
// Apply stays inside the shortest pane a member sees: a window at its
// 600-point minimum height and the design width (zoom 1), the status
// bar's 24 points off.
#[test]
fn apply_stays_inside_the_shortest_window_with_a_pairing_and_notes() {
    let (mut view, _) = open_on(PesVersion::Pes17, "interchange_short_window", CO);
    let pane = egui::vec2(DEFAULT_PANE.x, 600.0 - 24.0);
    let mut window = Headless::sized(pane.x, pane.y);
    picked(&mut window, &mut view, SQUAD_19, &lib_fixture(SQUAD_19));
    open_note_groups(&mut window, &mut view);

    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = nodes_in_window(&pairs, TITLE);
    assert!(
        nodes
            .iter()
            .any(|(_, node)| node.value() == Some("Who gets which record")),
        "premise: the pairing list is drawn"
    );
    assert!(
        nodes
            .iter()
            .any(|(_, node)| node.role() == Role::DisclosureTriangle),
        "premise: the note groups are drawn"
    );
    let bounds = |role: Role, label: &str| {
        nodes
            .iter()
            .find(|(_, node)| node.role() == role && node.label() == Some(label))
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("the {label:?} {role:?}"))
    };
    let (panel, apply) = (bounds(Role::Window, TITLE), bounds(Role::Button, "Apply"));
    assert!(
        apply.y0 >= panel.y0 && apply.y1 <= panel.y1,
        "Apply {apply:?} inside the panel {panel:?}"
    );
    assert!(
        apply.y1 <= f64::from(pane.y),
        "Apply {apply:?} inside the {pane:?} pane"
    );
    // Each list's scroll bar spans its viewport's height; the pairing's is
    // the upper one.
    let mut bars: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ScrollBar)
        .filter_map(|(_, node)| node.bounds())
        .collect();
    bars.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    let [pairing, notes] = bars[..] else {
        panic!("two scroll bars, the pairing's and the notes': {bars:?}")
    };
    let (pairing, notes) = (pairing.height(), notes.height());
    for (list, height) in [("pairing", pairing), ("notes", notes)] {
        assert!(
            height < f64::from(LIST_HEIGHT),
            "the {list} list {height} saturates under {LIST_HEIGHT} on this screen"
        );
    }
    // The pairing takes half the room it measures above itself; the notes
    // take what is left once the sections row and the notes' heading are
    // drawn, which the pairing's share reserves from the style's row
    // heights rather than measuring. So the halves agree to within one
    // widget row, Apply's height.
    assert!(
        (pairing - notes).abs() <= apply.height(),
        "the pairing ({pairing}) and the notes ({notes}) share the room within a row ({})",
        apply.height()
    );
}

// The lists' room keeps the window's bottom margin and the line under
// Apply off the screen's bottom, so the line that shows once Apply is
// clicked fits the shortest window as it stands. A room that added the
// margin instead of taking it away would leave the window taller than
// the screen once the line shows, and egui would push it up the screen
// frame after frame.
#[test]
fn the_panel_holds_still_once_the_line_under_apply_shows_in_the_shortest_window() {
    let (mut view, _) = open_on(PesVersion::Pes17, "interchange_short_window_applied", CO);
    let pane = egui::vec2(DEFAULT_PANE.x, 600.0 - 24.0);
    let mut window = Headless::sized(pane.x, pane.y);
    picked(&mut window, &mut view, SQUAD_19, &lib_fixture(SQUAD_19));
    open_note_groups(&mut window, &mut view);
    let top = |window: &mut Headless, view: &mut SaveView| {
        panel_node(window, view, Role::Window, TITLE)
            .and_then(|node| node.bounds())
            .expect("the panel is drawn")
            .y0
    };
    let opened = top(&mut window, &mut view);
    let apply = panel_node(&mut window, &mut view, Role::Button, "Apply").expect("Apply");
    assert!(!apply.is_disabled(), "premise: Apply is enabled");

    click(&mut window, &apply, &mut view);

    let applied = format!("Applied {SQUAD_19}. The changes stay unsaved until Save.");
    let lines = panel_lines(&mut window, &mut view);
    assert!(
        lines.contains(&applied),
        "premise: {applied:?} under Apply in {lines:#?}"
    );
    for frame in 0..3 {
        assert_eq!(
            top(&mut window, &mut view),
            opened,
            "the panel's top on frame {frame} after Apply"
        );
    }
}

// SE-GUI-68
#[test]
fn every_empty_place_a_file_names_is_one_group() {
    // PES 21's team 701 fills places 01 to 23 of its 32.
    let (mut view, mut window) = open_on(PesVersion::Pes21, "interchange_empty_places", 701);
    let team = team_entry(&view, 701);
    assert_eq!(
        team.roster.iter().position(|slot| slot.player_id == 0),
        Some(23),
        "premise: places 01 to 23 filled"
    );
    assert_eq!(team.roster[24].player_id, 0, "premise: place 25 empty");
    let text = "[players.24]\nname = \"GHOST\"\n\n[players.25]\nname = \"GHOST\"\n";
    picked(&mut window, &mut view, "ghosts.toml", text.as_bytes());

    assert_eq!(
        note_headers(&mut window, &mut view),
        ["Empty roster place \u{b7} 2 players"]
    );
}

/// The players' groups' headers, top to bottom.
fn note_headers(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let mut headers: Vec<(f64, String)> = nodes_in_window(&pairs, TITLE)
        .into_iter()
        .filter(|(_, node)| node.role() == Role::DisclosureTriangle)
        .map(|(_, node)| {
            (
                node.bounds().expect("drawn bounds").y0,
                node.label().expect("a labeled header").to_owned(),
            )
        })
        .collect();
    headers.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    headers.into_iter().map(|(_, header)| header).collect()
}

/// The group of the command line's note `line` (`- Warning <code>
/// (<key>=<value>, …)`): its code and its path, a player's `players.NN.`
/// written `players.*.`; every empty slot's note is its code alone.
fn note_group(line: &str) -> String {
    let (code, context) = line
        .strip_prefix("- Warning ")
        .and_then(|rest| rest.split_once(" ("))
        .unwrap_or_else(|| panic!("a note line: {line:?}"));
    let (key, at) = context.split_once('=').expect("a context key");
    if key == "slot" {
        return code.to_owned();
    }
    let at = at.split([',', ')']).next().expect("a context value");
    let at = match at
        .strip_prefix("players.")
        .and_then(|rest| rest.split_once('.'))
    {
        Some((_, tail)) => format!("players.*.{tail}"),
        None => at.to_owned(),
    };
    format!("{code} {at}")
}

// SE-GUI-16
#[test]
fn a_refusal_of_a_chosen_section_stands_under_the_choices_until_it_is_unticked() {
    let (mut view, mut window) = open_on(PesVersion::Pes19, "interchange_refusal", THREE);
    let text = export(&view, CO);
    let name_line = "name = \"/co/\"";
    assert_eq!(text.matches(name_line).count(), 1, "premise: [team] name");
    let edited = text.replacen(name_line, &format!("name = \"{}\"", "X".repeat(200)), 1);
    let document = TeamToml::parse(&edited).expect("the edited text parses");
    let refusal = document
        .apply(
            PesVersion::Pes19,
            &mut team_entry(&view, THREE),
            &mut rostered(&view, THREE),
        )
        .expect_err("premise: the lib refuses the long team name")
        .to_string();
    picked(&mut window, &mut view, "long.toml", edited.as_bytes());

    let lines = panel_lines(&mut window, &mut view);
    let at = |text: &str| lines.iter().position(|line| line == text);
    let refused = at(&refusal).unwrap_or_else(|| panic!("{refusal:?} in {lines:#?}"));
    assert!(
        at("Gameplan").is_some_and(|choice| choice < refused),
        "under the choices"
    );
    assert!(
        at("Apply").is_some_and(|apply| refused < apply),
        "above Apply"
    );
    let error = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    assert_eq!(
        painted(&mut window, &mut view, &refusal),
        error,
        "the refusal in the error colour"
    );
    assert_eq!(
        checkboxes(&mut window, &mut view),
        ["Stats", "Aesthetics", "Gameplan"]
    );
    assert!(
        apply_button(&mut window, &mut view).is_disabled(),
        "Apply is disabled"
    );

    toggle(&mut window, &mut view, "Stats");
    assert!(
        !panel_lines(&mut window, &mut view).contains(&refusal),
        "the refusal goes with stats"
    );
    assert!(
        !apply_button(&mut window, &mut view).is_disabled(),
        "Apply is enabled"
    );
}
