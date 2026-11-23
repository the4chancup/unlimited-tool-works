//! The grid's rows: the cells a finding colours, the folder a cell opens, a row's
//! log lines, and the message shown in place of a row's cells.

use super::*;

/// The codes of `cell`'s findings.
fn codes(cell: &Cell) -> Vec<&str> {
    cell.messages
        .iter()
        .map(|message| message.code.code.as_ref())
        .collect()
}

/// The texts of the painted cells on the line of the node reading `label`, left to right.
fn line_texts(nodes: &[accesskit::Node], label: &str) -> Vec<String> {
    let on = nodes
        .iter()
        .find(|node| node.value() == Some(label))
        .and_then(accesskit::Node::bounds)
        .unwrap_or_else(|| panic!("no node reads {label:?}"));
    let mut cells: Vec<(f64, String)> = nodes
        .iter()
        .filter_map(|node| {
            let bounds = node.bounds()?;
            let text = node.value()?;
            let middle = (bounds.y0 + bounds.y1) / 2.0;
            (bounds.x0 > on.x1 && on.y0 < middle && middle < on.y1)
                .then(|| (bounds.x0, text.to_owned()))
        })
        .collect();
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    cells.into_iter().map(|(_, text)| text).collect()
}

// TC-GUI-04
#[test]
fn the_view_shows_a_row_per_export_once_its_check_has_returned() {
    let temp = sandbox("view_grid_rows");
    let root = temp.path();
    list_teams(root);
    for folder in ["01 - A", "03 - B"] {
        add_player(root, "a Midcup Grid", folder);
    }
    for folder in ["g1", "p1"] {
        add_kit(root, "a Midcup Grid", folder);
    }
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources/egg Midcup Tracer.zip"),
        root.join("exports/co Midcup Tracer.zip"),
    )
    .unwrap();
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert!(
        tool.live_check.is_running(),
        "the first frame starts the check"
    );
    assert!(
        !shows_text(&nodes, "/a/"),
        "no row before the check returns"
    );

    finish_check(&mut tool, &ctx);
    let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
    assert_eq!(
        line_texts(&nodes, "/a/"),
        ["702", "01", "03", "2/2", "GK", "K1"]
    );
    assert_eq!(line_texts(&nodes, "/co/"), ["714", "05", "1/1", "GK"]);
    let rows = tool.live_check.rows();
    let folder = row(rows, "/a/");
    let mut cells = vec![player(folder, 1), player(folder, 3), portraits(folder)];
    cells.extend([kit(folder, 0), kit(folder, 1)]);
    for cell in cells.into_iter().map(Option::unwrap) {
        assert_eq!(
            (cell.state, cell.outline),
            (CellState::FullOk, None),
            "{cell:?}"
        );
    }
    assert_eq!(player(folder, 2), None, "02 is an empty position");
    assert_eq!(portraits(folder).unwrap().width, Width::Wide);
    let archive = row(rows, "/co/");
    for cell in [player(archive, 5), kit(archive, 0)]
        .into_iter()
        .map(Option::unwrap)
    {
        assert_eq!(
            (cell.state, cell.outline),
            (CellState::FullOk, None),
            "{cell:?}"
        );
    }
}

// TC-GUI-20
#[test]
fn a_referee_folder_backing_two_slots_colours_both() {
    let temp = sandbox("view_grid_referees");
    let root = temp.path();
    add_file(root, "refs Cup/players.txt", b"01 A\n02 B\n35 A\n");
    add_player(root, "refs Cup", "A");
    add_player(root, "refs Cup", "B");
    add_file(root, "refs Cup/Players/A/notes.exe", b"");
    let rows = checked_rows(root);
    let referees = row(&rows, "/refs/");
    assert_eq!(referees.id, Some(IdCell::Blank));
    let drawn: Vec<usize> = (1..=46)
        .filter(|slot| player(referees, *slot).is_some())
        .collect();
    assert_eq!(drawn, [1, 2, 35], "35 on the second line, under 12");
    let (first, last) = (player(referees, 1).unwrap(), player(referees, 35).unwrap());
    assert_eq!(first.outline, Some(Outline::Error));
    assert_eq!(
        (first.outline, &first.messages),
        (last.outline, &last.messages)
    );
    assert_eq!(codes(first), ["file_type_disallowed"]);
    assert_eq!(player(referees, 2).unwrap().outline, None);
}

// TC-GUI-21
#[test]
fn a_shared_folder_s_and_a_logo_s_findings_colour_the_label_alone() {
    let temp = sandbox("view_grid_label");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Label", "03 - A");
    add_kit(root, "co Midcup Label", "g1");
    add_file(root, "co Midcup Label/Boots/Solo/boots.fmdl", b"");
    let logo = fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
    add_file(root, "co Midcup Label/logo_small.dds", &logo);
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(
        team.label.outline,
        Some(Outline::Error),
        "the worse of the two"
    );
    let label = codes(&team.label);
    for code in ["shared_folder_orphaned", "logo_small_without_main"] {
        assert!(label.contains(&code), "{code} in {label:?}");
    }
    for cell in [player(team, 3), kit(team, 0)]
        .into_iter()
        .map(Option::unwrap)
    {
        assert_eq!(cell.outline, None, "{cell:?}");
    }
}

// A finding about a player folder, or a file in one, is that player's: a dropped
// folder's included, which keeps its slot's cell.
#[test]
fn a_player_folder_s_finding_colours_its_player() {
    let temp = sandbox("view_grid_player");
    let root = temp.path();
    list_teams(root);
    for folder in ["01 - A", "03 - B", "07 - C"] {
        add_player(root, "co Midcup Map", folder);
    }
    add_file(root, "co Midcup Map/Players/03 - B/notes.exe", b"");
    add_file(root, "co Midcup Map/Players/07 - C/fpc_on", b"");
    add_file(root, "co Midcup Map/Players/07 - C/fpc_off", b"");
    complete(&root.join("exports/co Midcup Map"), PesVersion::Pes21, &[]);
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(codes(player(team, 3).unwrap()), ["file_type_disallowed"]);
    assert_eq!(codes(player(team, 7).unwrap()), ["fpc_conflict"]);
    assert_eq!(player(team, 7).unwrap().outline, Some(Outline::Error));
    assert_eq!(player(team, 1).unwrap().outline, None);
    assert_eq!(team.label.outline, None, "{:?}", codes(&team.label));
}

// A player's portrait in his own folder is his cell's, not the portraits cell's.
#[test]
fn a_portrait_in_a_player_folder_colours_the_player() {
    let temp = sandbox("view_grid_own_portrait");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_file(
        root,
        "co Midcup Map/Players/03 - B/portrait.dds",
        b"not a texture",
    );
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let his = player(team, 3).unwrap();
    let unreadable = his
        .messages
        .iter()
        .find(|message| message.code.code == "texture_unreadable")
        .unwrap_or_else(|| panic!("{:?}", codes(his)));
    assert!(
        matches!(&unreadable.scope, studio_core::Scope::File { path, .. } if path.as_str() == "Players/03 - B/portrait.dds"),
        "{:?}",
        unreadable.scope
    );
    assert_eq!(his.outline, Some(Outline::Error));
    assert_eq!(portraits(team).unwrap().messages, Vec::new());
}

// A file of `Portraits/` is the portraits cell's.
#[test]
fn a_portraits_file_s_finding_colours_the_portraits_cell() {
    let temp = sandbox("view_grid_portraits");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    let portrait =
        fs::read(tracer().join("Players/05 - The Chad Stormworks Player/portrait.dds")).unwrap();
    add_file(root, "co Midcup Map/Portraits/player_03.dds", &portrait);
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let cell = portraits(team).unwrap();
    assert_eq!(codes(cell), ["portrait_name_invalid"]);
    assert_eq!(cell.outline, Some(Outline::Error));
    assert_eq!(player(team, 3).unwrap().outline, None);
}

// A kit folder's finding is that kit's.
#[test]
fn a_kit_folder_s_finding_colours_its_kit() {
    let temp = sandbox("view_grid_kit");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_kit(root, "co Midcup Map", "g1");
    add_kit(root, "co Midcup Map", "p1 - Home");
    add_file(root, "co Midcup Map/Kits/p1 - Home/icon_99", b"");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let home = kit(team, 1).unwrap();
    assert!(
        codes(home).contains(&"kit_icon_invalid"),
        "{:?}",
        codes(home)
    );
    assert_eq!(home.outline, Some(Outline::Warning));
    assert_eq!(kit(team, 0).unwrap().outline, None, "GK");
}

// A kit folder validation drops keeps its cell: both layout markers in one kit
// (TC-KIT-06's input), in a Midcup export, which gets no empty kit in its place.
#[test]
fn a_dropped_kit_folder_keeps_its_cell() {
    let temp = sandbox("view_grid_dropped_kit");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_kit(root, "co Midcup Map", "g1");
    add_kit(root, "co Midcup Map", "p1");
    add_file(root, "co Midcup Map/Kits/p1/pre-fox", b"");
    add_file(root, "co Midcup Map/Kits/p1/fox", b"");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let home = kit(team, 1).expect("K1 drawn");
    assert_eq!(codes(home), ["kit_layout_conflict"]);
    assert_eq!(home.outline, Some(Outline::Error));
}

// An export dropped whole is drawn from its listing: here a team the teams list lacks,
// whose portraits count reads its `Portraits/` file. The copied player's own portrait is
// removed, so the count can come from that file alone.
#[test]
fn a_dropped_export_counts_its_portraits_from_its_listing() {
    let temp = sandbox("view_grid_dropped_export");
    let root = temp.path();
    list_teams(root);
    add_player(root, "v Midcup Map", "03 - B");
    let own = root.join("exports/v Midcup Map/Players/03 - B/portrait.dds");
    let portrait = fs::read(&own).unwrap();
    fs::remove_file(own).unwrap();
    add_file(root, "v Midcup Map/Portraits/03.dds", &portrait);
    let rows = checked_rows(root);
    let team = row(&rows, "/v/");
    assert_eq!(team.id, Some(IdCell::Missing));
    assert!(codes(&team.label).contains(&"team_name_unknown"));
    assert_eq!(player(team, 3).unwrap().state, CellState::Blocked);
    assert_eq!(portraits(team).unwrap().text, "1/1");
}

// A dropped export's portraits count takes a `portrait.*` only directly in a player's
// folder: neither a folder holding other files and no portrait, nor one whose portrait
// sits a folder deeper, counts.
#[test]
fn a_dropped_export_counts_no_portrait_missing_from_a_player_s_own_folder() {
    let temp = sandbox("view_grid_dropped_no_portrait");
    let root = temp.path();
    list_teams(root);
    add_player(root, "v Midcup Map", "03 - B");
    add_player(root, "v Midcup Map", "05 - C");
    let players = root.join("exports/v Midcup Map/Players");
    fs::remove_file(players.join("03 - B/portrait.dds")).unwrap();
    assert!(
        fs::read_dir(players.join("03 - B"))
            .unwrap()
            .next()
            .is_some(),
        "premise: 03's folder holds other files"
    );
    fs::create_dir(players.join("05 - C/Old")).unwrap();
    fs::rename(
        players.join("05 - C/portrait.dds"),
        players.join("05 - C/Old/portrait.dds"),
    )
    .unwrap();
    let rows = checked_rows(root);
    let team = row(&rows, "/v/");
    assert_eq!(player(team, 3).unwrap().state, CellState::Blocked);
    assert_eq!(portraits(team).unwrap().text, "0/2");
}

/// The canonical path of what `open` opens, so a test compares it with a path it built
/// whatever link the temp folder is reached through.
fn opened(open: Option<&Open>) -> Option<(&'static str, PathBuf)> {
    open.map(|open| match open {
        Open::Folder(path) => ("folder", fs::canonicalize(path).unwrap()),
        Open::Select(path) => ("select", fs::canonicalize(path).unwrap()),
    })
}

/// `opened`'s answer for a folder at `path`.
fn folder_at(path: &Path) -> Option<(&'static str, PathBuf)> {
    Some(("folder", fs::canonicalize(path).unwrap()))
}

// Each cell names its folder inside the export and opens it: the label the export's own,
// the portraits cell the export's `Portraits/` when it has one.
#[test]
fn each_cell_names_and_opens_its_folder() {
    let temp = sandbox("view_grid_folders");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_kit(root, "co Midcup Map", "g1");
    let portrait =
        fs::read(tracer().join("Players/05 - The Chad Stormworks Player/portrait.dds")).unwrap();
    add_file(root, "co Midcup Map/Portraits/03.dds", &portrait);
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(team.key, "co Midcup Map");
    let export = root.join("exports/co Midcup Map");
    for (cell, place, folder) in [
        (&team.label, "co Midcup Map", export.clone()),
        (
            player(team, 3).unwrap(),
            "Players/03 - B",
            export.join("Players/03 - B"),
        ),
        (kit(team, 0).unwrap(), "Kits/g1", export.join("Kits/g1")),
        (
            portraits(team).unwrap(),
            "Portraits",
            export.join("Portraits"),
        ),
    ] {
        assert_eq!(cell.place, place);
        assert_eq!(opened(cell.open.as_ref()), folder_at(&folder), "{place}");
    }
}

// With no `Portraits/` folder, the portraits are in the player folders: the cell has no one
// folder to open.
#[test]
fn a_portraits_cell_without_its_folder_opens_nothing() {
    let temp = sandbox("view_grid_no_portraits_folder");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    let rows = checked_rows(root);
    let cell = portraits(row(&rows, "/co/")).unwrap();
    assert_eq!(cell.messages, Vec::new(), "premise: no finding");
    assert_eq!(cell.place, "Portraits in the player folders");
    assert_eq!(cell.open, None);
}

// Every cell of an archive export opens the folder holding it, the archive selected.
#[test]
fn every_cell_of_an_archive_selects_the_archive() {
    let temp = sandbox("view_grid_archive_open");
    let root = temp.path();
    list_teams(root);
    let archive = root.join("exports/co Midcup Tracer.zip");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources/egg Midcup Tracer.zip"),
        &archive,
    )
    .unwrap();
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(team.key, "co Midcup Tracer.zip");
    let selected = Some(("select", fs::canonicalize(&archive).unwrap()));
    for cell in [
        &team.label,
        player(team, 5).unwrap(),
        kit(team, 0).unwrap(),
        portraits(team).unwrap(),
    ] {
        assert_eq!(opened(cell.open.as_ref()), selected, "{}", cell.text);
    }
    assert_eq!(
        player(team, 5).unwrap().place,
        "Players/05 - The Chad Stormworks Player"
    );
}

// The label's Copy messages copies the export's findings as the log prints them, in the
// order the check reported them: three findings on two players and the label, then the
// Info every identified export carries.
#[test]
fn a_row_s_log_lines_are_its_export_s_findings_as_the_log_prints_them() {
    let temp = sandbox("view_grid_log_lines");
    let root = temp.path();
    list_teams(root);
    for folder in ["03 - B", "07 - C"] {
        add_player(root, "co Midcup Map", folder);
    }
    add_file(root, "co Midcup Map/Players/03 - B/notes.exe", b"");
    add_file(root, "co Midcup Map/Players/07 - C/fpc_on", b"");
    add_file(root, "co Midcup Map/Players/07 - C/fpc_off", b"");
    add_file(root, "co Midcup Map/Boots/Solo/boots.fmdl", b"");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(
        team.log_lines,
        [
            "- co Midcup Map: Error file_type_disallowed at Players/03 - B (file=notes.exe)",
            "- co Midcup Map: Error fpc_conflict at Players/07 - C",
            "- co Midcup Map: Warning shared_folder_orphaned at Boots/Solo",
            "- co Midcup Map: Info export_identified (team=/co/, id=714)",
        ]
    );
}

// TC-GUI-25: an export with no Warning and no Error has nothing to send its member, so its
// label offers no Copy messages, though the check's Info `export_identified` is in the log.
#[test]
fn a_clean_export_s_row_has_no_log_lines() {
    let temp = sandbox("view_grid_clean_log_lines");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(
        team.label.outline, None,
        "premise: no Warning or Error on the label"
    );
    assert_eq!(team.log_lines, Vec::<String>::new());
}

// An export whose name says neither `Full` nor `Midcup` shows a warning in place of its
// cells.
#[test]
fn an_export_without_its_coverage_tag_shows_its_message() {
    let temp = sandbox("view_grid_tag_missing");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Spring 2026", "03 - B");
    let rows = checked_rows(root);
    let message = RowBody::Message {
        text: "Full or Midcup?".to_owned(),
        tone: Tone::Warning,
    };
    assert_eq!(row(&rows, "/co/").body, message);
}

// An export in the old layout shows an error in place of its cells.
#[test]
fn an_old_layout_export_shows_its_message() {
    let temp = sandbox("view_grid_layout_old");
    let root = temp.path();
    list_teams(root);
    for path in [
        "Faces/01 - A/face.model",
        "Kit Configs/co_DEF_1st_realUni.bin",
    ] {
        add_file(root, &format!("co Midcup Spring/{path}"), b"old");
    }
    let rows = checked_rows(root);
    let message = RowBody::Message {
        text: "Old export format".to_owned(),
        tone: Tone::Error,
    };
    assert_eq!(row(&rows, "/co/").body, message);
}

// A dropped player's portrait is still in the export, so the portraits cell counts him.
#[test]
fn a_dropped_player_s_portrait_still_counts() {
    let temp = sandbox("view_grid_dropped_portrait");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "01 - A");
    add_player(root, "co Midcup Map", "03 - B");
    add_file(root, "co Midcup Map/Players/03 - B/fpc_on", b"");
    add_file(root, "co Midcup Map/Players/03 - B/fpc_off", b"");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    assert_eq!(codes(player(team, 3).unwrap()), ["fpc_conflict"]);
    assert_ne!(player(team, 1).unwrap().state, CellState::Blocked);
    assert_eq!(portraits(team).unwrap().text, "2/2");
}

// Anything else is the label's: here a player folder whose name claims no slot.
#[test]
fn a_player_folder_at_no_slot_colours_the_label() {
    let temp = sandbox("view_grid_no_slot");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_player(root, "co Midcup Map", "15_Snuffy");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let label = codes(&team.label);
    assert!(label.contains(&"player_folder_number_invalid"), "{label:?}");
    assert_eq!(team.label.outline, Some(Outline::Error));
    assert_eq!(player(team, 15), None);
    assert_eq!(player(team, 3).unwrap().outline, None);
}
