//! The team file panel's export row's tests (SE-GUI-16, SE-GUI-17): the
//! row's texts and the separator under it, a Team TOML written byte for
//! byte as the command line writes it, a Texport of the save's version that
//! the import half reads back to the same team, the Texport disabled on
//! PES 2015 and 2016, and the open save refused as the destination.

use std::sync::mpsc;

use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;
use pes_savefile::interchange::texport::Texport;
use pes_savefile::model::TeamColor;
use studio_core::NoticeKind;

use crate::test_support::cli_stdout;
use crate::view::interchange::Export;

use super::*;

/// The panel's title, its window's label.
const TITLE: &str = "Team file interchange";

/// `/co/`, a team of every fixture save.
const CO: u32 = 714;

/// `version`'s fixture open under `test` with team `target`'s row selected
/// and the team file panel open, its measuring first frame drawn.
fn open_on(version: PesVersion, test: &str, target: u32) -> (SaveView, Headless) {
    let mut view = SaveView::new();
    view.open(&fixture(version, test));
    view.team = TeamChoice::Team(target);
    view.selected = Some(Selection::Team(target));
    view.panel = Some(Panel::Interchange);
    let mut window = Headless::sized(DEFAULT_PANE.x, 1400.0);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    (view, window)
}

/// The folder the open save sits in, the test's scratch folder.
fn save_folder(view: &SaveView) -> PathBuf {
    view.session
        .as_ref()
        .expect("an open save")
        .path()
        .parent()
        .expect("the save sits in a folder")
        .to_owned()
}

/// The panel's nodes on a fresh frame, and that frame's painted shapes.
fn panel_frame(
    window: &mut Headless,
    view: &mut SaveView,
) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let nodes = nodes_in_window(&pairs, TITLE)
        .into_iter()
        .map(|(_, node)| node.clone())
        .collect();
    (nodes, shapes)
}

/// The panel's button labeled `label`, which must be drawn.
fn button(window: &mut Headless, view: &mut SaveView, label: &str) -> accesskit::Node {
    panel_frame(window, view)
        .0
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("the {label:?} button"))
}

/// The panel's Label values and widget labels in top-to-bottom,
/// left-to-right order, the window's title, close button and resize
/// handles left out.
fn panel_lines(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    window_lines(window, view, TITLE)
        .into_iter()
        .filter(|text| {
            text != TITLE && text != "Close window" && !text.starts_with("Resize window")
        })
        .collect()
}

/// A click on the export row's `label` button, the Save dialog stubbed to
/// pick `destination`: what the dialog was asked for, the format and the
/// suggested path.
fn export(
    window: &mut Headless,
    view: &mut SaveView,
    label: &str,
    destination: &Path,
) -> (Export, PathBuf) {
    let (sender, asked) = mpsc::channel();
    let destination = destination.to_owned();
    view.team_export_dialog = Box::new(move |export, suggested| {
        sender
            .send((export, suggested.to_owned()))
            .expect("the test holds the receiver");
        Some(destination.clone())
    });
    let button = button(window, view, label);
    assert!(!button.is_disabled(), "premise: {label:?} is enabled");
    click(window, &button, view);
    asked.try_recv().expect("the Save dialog opened")
}

/// Asserts `written` equals `expected` byte for byte, naming the first
/// differing offset (a whole file in the message would bury it).
fn assert_same_bytes(written: &[u8], expected: &[u8]) {
    let first = written
        .iter()
        .zip(expected)
        .position(|(a, b)| a != b)
        .or((written.len() != expected.len()).then(|| written.len().min(expected.len())));
    if let Some(offset) = first {
        panic!(
            "first difference at byte {offset} (lengths {} and {}): {:?} against {:?}",
            written.len(),
            expected.len(),
            String::from_utf8_lossy(&written[offset..(offset + 40).min(written.len())]),
            String::from_utf8_lossy(&expected[offset..(offset + 40).min(expected.len())]),
        );
    }
}

#[test]
fn the_export_row_names_the_team_and_offers_both_formats_above_a_separator() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_row", CO);
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines[..4],
        [
            "Export /co/ (714) as ",
            "Team TOML\u{2026}",
            "Texport\u{2026}",
            "Import into /co/ (714) ",
        ],
        "the export row first, then the import row: {lines:#?}"
    );
    let toml = button(&mut window, &mut view, "Team TOML\u{2026}");
    let texport = button(&mut window, &mut view, "Texport\u{2026}");
    let (toml, texport) = (
        toml.bounds().expect("drawn"),
        texport.bounds().expect("drawn"),
    );
    assert!(
        toml.x1 <= texport.x0 && toml.y0 < texport.y1 && texport.y0 < toml.y1,
        "one row, Team TOML first: {toml:?} {texport:?}"
    );

    // The separator: a horizontal line across the panel between the
    // export row and the import row.
    let (nodes, shapes) = panel_frame(&mut window, &mut view);
    let import = nodes
        .iter()
        .find(|node| node.value() == Some("Import into /co/ (714) "))
        .and_then(|node| node.bounds())
        .expect("the import row");
    let separated = shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::LineSegment { points, .. } => {
            let (y, left, right) = (
                f64::from(points[0].y),
                f64::from(points[0].x.min(points[1].x)),
                f64::from(points[0].x.max(points[1].x)),
            );
            points[0].y == points[1].y
                && y > toml.y1
                && y < import.y0
                && left <= toml.x0
                && right >= texport.x1
        }
        _ => false,
    });
    assert!(separated, "a separator under the export row");
}

#[test]
fn the_export_buttons_say_what_each_file_is() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_hovers", CO);
    let toml = button(&mut window, &mut view, "Team TOML\u{2026}");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&toml), "full-fidelity").as_deref(),
        Some("One team's full-fidelity file, the same text the CLI's export-toml writes")
    );
    let texport = button(&mut window, &mut view, "Texport\u{2026}");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&texport), "PES itself").as_deref(),
        Some("A file PES itself can import")
    );
}

// SE-GUI-16
#[test]
fn the_team_toml_written_is_the_command_lines_export() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_toml", CO);
    let folder = save_folder(&view);
    let save = folder.join("EDIT00000000");
    let written = folder.join("panel.toml");
    assert!(!written.exists(), "premise: nothing written yet");

    let (format, suggested) = export(&mut window, &mut view, "Team TOML\u{2026}", &written);

    assert_eq!(format, Export::TeamToml);
    assert_eq!(
        (format.name(), format.extension()),
        ("Team TOML", "toml"),
        "the dialog's filter"
    );
    assert_eq!(
        suggested,
        folder.join("714.toml"),
        "<id>.toml beside the save"
    );
    let cli = folder.join("cli.toml");
    cli_stdout(
        &folder,
        &[
            "export-toml",
            save.to_str().unwrap(),
            "--team",
            "714",
            "--out",
            cli.to_str().unwrap(),
        ],
    );
    assert_same_bytes(
        &std::fs::read(&written).unwrap(),
        &std::fs::read(&cli).unwrap(),
    );
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some(("Wrote panel.toml", NoticeKind::Success))
    );
    assert!(view.refusal.is_none(), "no refusal");
}

/// Team `/co/` of `version`'s fixture exported as a Texport under the name
/// the Save dialog suggests, `suggested_name` (`<short name>.ted` on 18-21,
/// the first free `TEXPORT0000000N` on 17), the dialog filtering on
/// `filter`; then picked in the import half with every section chosen,
/// which then changes nothing.
fn texport_round_trip(version: PesVersion, test: &str, suggested_name: &str, filter: Option<&str>) {
    let (mut view, mut window) = open_on(version, test, CO);
    let folder = save_folder(&view);
    let written = folder.join(suggested_name);
    assert!(!written.exists(), "premise: nothing written yet");

    let (format, suggested) = export(&mut window, &mut view, "Texport\u{2026}", &written);

    assert_eq!(format, Export::Texport);
    assert_eq!(
        (format.name(), format.filter(&suggested)),
        ("Texport", filter),
        "the dialog's filter"
    );
    assert_eq!(suggested, written, "the suggested name beside the save");
    let bytes = std::fs::read(&written).unwrap();
    let texport = Texport::from_bytes(&bytes, None).expect("the file reads as a Texport");
    assert_eq!(texport.version(), version, "the save's version");
    assert_eq!(texport.team().id, CO, "the team's");
    let wrote = format!("Wrote {suggested_name}");
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some((wrote.as_str(), NoticeKind::Success))
    );

    view.interchange_dialog = Box::new(move |_| Some(written.clone()));
    let pick = button(&mut window, &mut view, "Pick\u{2026}");
    click(&mut window, &pick, &mut view);
    let lines = panel_lines(&mut window, &mut view);
    let intro = format!("A Texport written from a {version} save.");
    assert!(lines.contains(&intro), "{intro:?} in {lines:#?}");
    let (nodes, _) = panel_frame(&mut window, &mut view);
    let mut ticked: Vec<&str> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::CheckBox && node.toggled() == Some(accesskit::Toggled::True)
        })
        .filter_map(|node| node.label())
        .collect();
    ticked.sort_unstable();
    assert_eq!(
        ticked,
        ["Aesthetics", "Gameplay", "Tactics"],
        "every section"
    );
    // Read back onto the team it came from, every section changes nothing.
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(
        lines.last().map(String::as_str),
        Some("The team already holds every value of the chosen sections."),
        "{lines:#?}"
    );
    assert!(
        button(&mut window, &mut view, "Apply").is_disabled(),
        "Apply is disabled"
    );
}

// SE-GUI-17
#[test]
fn a_pes_17_texport_reads_back_to_the_same_team() {
    // No extension, so the dialog has no filter to add `.ted` with.
    texport_round_trip(
        PesVersion::Pes17,
        "export_texport_17",
        "TEXPORT00000000",
        None,
    );
}

// SE-GUI-17
#[test]
fn a_pes_18_texport_reads_back_to_the_same_team() {
    // Team 714 of the PES 18 fixture is `DIY` by its short name.
    texport_round_trip(
        PesVersion::Pes18,
        "export_texport_18",
        "DIY.ted",
        Some("ted"),
    );
}

// SE-GUI-17
#[test]
fn a_pes_19_texport_reads_back_to_the_same_team() {
    texport_round_trip(
        PesVersion::Pes19,
        "export_texport_19",
        "CO.ted",
        Some("ted"),
    );
}

// SE-GUI-17
#[test]
fn a_pes_20_texport_reads_back_to_the_same_team() {
    texport_round_trip(
        PesVersion::Pes20,
        "export_texport_20",
        "CO.ted",
        Some("ted"),
    );
}

// SE-GUI-17
#[test]
fn a_pes_21_texport_reads_back_to_the_same_team() {
    texport_round_trip(
        PesVersion::Pes21,
        "export_texport_21",
        "CO.ted",
        Some("ted"),
    );
}

/// A `TEXPORT00000000` already in the PES 2017 save's folder: the Save
/// dialog suggests the next number, so the export writes over nothing.
#[test]
fn a_pes_17_texport_suggests_the_first_number_the_folder_lacks() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "export_texport_17_taken", CO);
    let folder = save_folder(&view);
    let taken = folder.join("TEXPORT00000000");
    std::fs::write(&taken, b"an earlier export").expect("the taken name is written");
    let (sender, asked) = mpsc::channel();
    view.team_export_dialog = Box::new(move |_, suggested| {
        sender
            .send(suggested.to_owned())
            .expect("the test holds the receiver");
        None
    });
    let texport = button(&mut window, &mut view, "Texport\u{2026}");
    assert!(!texport.is_disabled(), "premise: the Texport is enabled");
    click(&mut window, &texport, &mut view);

    let suggested = asked.try_recv().expect("the Save dialog opened");
    assert_eq!(suggested, folder.join("TEXPORT00000001"));
    assert_eq!(
        std::fs::read(&taken).unwrap(),
        b"an earlier export",
        "the earlier export is untouched"
    );
}

#[test]
fn both_exports_carry_the_edits_the_save_holds_unsaved() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_unsaved", CO);
    // The save stores each channel in 6 bits, 0 to 63.
    let colour = TeamColor {
        red: 10,
        green: 20,
        blue: 30,
    };
    let mut team = team_entry(&view, CO);
    let colours = team.colors.as_mut().expect("a PES 21 team has colours");
    assert_ne!(colours[0], colour, "premise: the first colour differs");
    colours[0] = colour;
    let (index, slot) = team
        .roster
        .iter()
        .enumerate()
        .find(|(_, slot)| slot.player_id != 0)
        .expect("a rostered player");
    let key = u8::try_from(index + 1).expect("a roster key");
    let mut player = player_entry(&view, slot.player_id);
    let prowess = ABILITY_STATS
        .iter()
        .position(|name| *name == "attacking_prowess")
        .expect("attacking_prowess is an ability stat");
    assert_ne!(
        player.stats.abilities()[prowess],
        Some(78),
        "premise: Attacking Prowess is not 78"
    );
    player.stats.set_ability(prowess, 78);
    view.session
        .as_mut()
        .expect("an open save")
        .edit_entries(vec![player.clone()], vec![team]);
    let folder = save_folder(&view);
    let on_disk = EditFile::load(&folder.join("EDIT00000000")).expect("the save on disk reads");
    assert_ne!(
        on_disk.player(player.id).map(|entry| &entry.stats),
        Some(&player.stats),
        "premise: the stat is not saved"
    );

    let toml = folder.join("unsaved.toml");
    export(&mut window, &mut view, "Team TOML\u{2026}", &toml);
    let parsed =
        TeamToml::parse(&std::fs::read_to_string(&toml).unwrap()).expect("the Team TOML reads");
    assert_eq!(parsed.team.color_1, Some(colour), "the Team TOML's colour");
    assert_eq!(
        parsed.players[&key].stats.attacking_prowess,
        Some(78),
        "the Team TOML's stat"
    );

    let ted = folder.join("unsaved.ted");
    export(&mut window, &mut view, "Texport\u{2026}", &ted);
    let texport =
        Texport::from_bytes(&std::fs::read(&ted).unwrap(), None).expect("the Texport reads");
    assert_eq!(
        texport.team().colors.map(|colours| colours[0]),
        Some(colour),
        "the Texport's colour"
    );
    let exported = texport
        .players()
        .iter()
        .find(|entry| entry.id == player.id)
        .expect("the player is in the Texport");
    assert_eq!(
        exported.stats.abilities()[prowess],
        Some(78),
        "the Texport's stat"
    );
}

#[test]
fn a_team_without_a_short_name_suggests_its_id_and_a_cancel_writes_nothing() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_no_short_name", CO);
    let mut team = team_entry(&view, CO);
    assert_eq!(team.short_name, "CO", "premise: a short name to clear");
    team.short_name.clear();
    view.session
        .as_mut()
        .expect("an open save")
        .edit_entries(Vec::new(), vec![team]);
    let (sender, asked) = mpsc::channel();
    view.team_export_dialog = Box::new(move |_, suggested| {
        sender
            .send(suggested.to_owned())
            .expect("the test holds the receiver");
        None
    });
    let texport = button(&mut window, &mut view, "Texport\u{2026}");
    click(&mut window, &texport, &mut view);

    let suggested = asked.try_recv().expect("the Save dialog opened");
    assert_eq!(suggested, save_folder(&view).join("714.ted"));
    assert!(!suggested.exists(), "no file written");
    assert!(view.notice.is_none(), "no Wrote notice");
    assert!(view.refusal.is_none(), "no refusal");
}

#[test]
fn a_write_that_fails_is_refused_in_the_command_lines_words() {
    let (mut view, mut window) = open_on(PesVersion::Pes21, "export_write_fails", CO);
    // A folder that does not exist: the write fails.
    let written = save_folder(&view).join("missing").join("panel.toml");
    let error = std::fs::write(&written, b"").expect_err("premise: the write fails");

    export(&mut window, &mut view, "Team TOML\u{2026}", &written);

    assert_eq!(
        view.refusal
            .as_ref()
            .map(|refusal| (refusal.title, refusal.message.clone())),
        Some((
            "Cannot export the team",
            format!("{}: cannot write the file ({error})", written.display())
        ))
    );
    assert!(view.notice.is_none(), "no Wrote notice");
}

// SE-GUI-17
#[test]
fn on_pes_15_and_16_the_texport_is_disabled_and_says_why() {
    for version in [PesVersion::Pes15, PesVersion::Pes16] {
        let test = format!("export_texport_disabled_{}", version.year());
        let (mut view, mut window) = open_on(version, &test, CO);
        let texport = button(&mut window, &mut view, "Texport\u{2026}");
        assert!(texport.is_disabled(), "no Texport on {version}");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(&texport), "Texport template")
                .as_deref(),
            Some("PES 2015 and 2016 have no Texport template"),
            "{version}"
        );
        assert!(
            !button(&mut window, &mut view, "Team TOML\u{2026}").is_disabled(),
            "{version}: the Team TOML stays"
        );
    }
}

#[test]
fn the_open_save_is_refused_as_the_destination() {
    assert_save_refused("export_refused", |folder| folder.join("EDIT00000000"));
}

#[test]
fn the_open_save_named_through_another_folder_is_refused() {
    assert_save_refused("export_refused_spelling", |folder| {
        std::fs::create_dir(folder.join("sub")).expect("the sub folder is created");
        folder.join("sub").join("..").join("EDIT00000000")
    });
}

/// For both formats, a fixture opened under `test` and the destination
/// `pick` names from the save's folder, a spelling of the save: the
/// refusal names that path in the command line's sentence, the save's
/// bytes are unchanged and nothing says Wrote.
fn assert_save_refused(test: &str, pick: impl Fn(&Path) -> PathBuf) {
    for (label, format) in [
        ("Team TOML\u{2026}", "Team TOML"),
        ("Texport\u{2026}", "Texport"),
    ] {
        let test = format!("{test}_{}", format.replace(' ', "_"));
        let (mut view, mut window) = open_on(PesVersion::Pes21, &test, CO);
        let folder = save_folder(&view);
        let save = folder.join("EDIT00000000");
        let before = std::fs::read(&save).unwrap();
        let destination = pick(&folder);

        export(&mut window, &mut view, label, &destination);

        assert!(
            std::fs::read(&save).unwrap() == before,
            "{label}: the save's bytes are unchanged"
        );
        // The panel is drawn before the refusal's modal, so the modal's
        // first frame is the click's own, its sizing pass; the next one
        // holds the nodes.
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let sentence = format!(
            "{}: that is the save being read, and a {format} written over it would lose it",
            destination.display()
        );
        for text in ["Cannot export the team", sentence.as_str()] {
            assert!(
                label_value(&nodes, text),
                "{label}: the refusal shows {text:?}"
            );
        }
        assert!(view.notice.is_none(), "{label}: no Wrote notice");
    }
}
