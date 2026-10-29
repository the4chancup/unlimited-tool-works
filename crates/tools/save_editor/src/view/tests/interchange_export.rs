//! The team file panel's export row's tests (SE-GUI-16, SE-GUI-17): the
//! row's texts and the separator under it, a Team TOML written byte for
//! byte as the command line writes it, a Texport of the save's version that
//! the import half reads back to the same team, the Texport disabled on
//! PES 2017, and the open save refused as the destination.

use std::sync::mpsc;

use pes_savefile::interchange::texport::Texport;
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

/// Team `/co/` of `version`'s fixture exported as a Texport, its suggested
/// name `suggested_name` (the fixture's short name), then picked in the
/// import half with every section chosen, which then changes nothing.
fn texport_round_trip(version: PesVersion, test: &str, suggested_name: &str) {
    let (mut view, mut window) = open_on(version, test, CO);
    let folder = save_folder(&view);
    let written = folder.join("panel.ted");

    let (format, suggested) = export(&mut window, &mut view, "Texport\u{2026}", &written);

    assert_eq!(format, Export::Texport);
    assert_eq!(
        (format.name(), format.extension()),
        ("Texport", "ted"),
        "the dialog's filter"
    );
    assert_eq!(
        suggested,
        folder.join(suggested_name),
        "<short name>.ted beside the save"
    );
    let bytes = std::fs::read(&written).unwrap();
    let texport = Texport::from_bytes(&bytes, None).expect("the file reads as a Texport");
    assert_eq!(texport.version(), version, "the save's version");
    assert_eq!(texport.team().id, CO, "the team's");
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some(("Wrote panel.ted", NoticeKind::Success))
    );

    view.interchange_dialog = Box::new(move || Some(written.clone()));
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
        ["aesthetics", "gameplay", "tactics"],
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
fn a_pes_18_texport_reads_back_to_the_same_team() {
    // Team 714 of the PES 18 fixture is `DIY` by its short name.
    texport_round_trip(PesVersion::Pes18, "export_texport_18", "DIY.ted");
}

// SE-GUI-17
#[test]
fn a_pes_19_texport_reads_back_to_the_same_team() {
    texport_round_trip(PesVersion::Pes19, "export_texport_19", "CO.ted");
}

// SE-GUI-17
#[test]
fn a_pes_20_texport_reads_back_to_the_same_team() {
    texport_round_trip(PesVersion::Pes20, "export_texport_20", "CO.ted");
}

// SE-GUI-17
#[test]
fn a_pes_21_texport_reads_back_to_the_same_team() {
    texport_round_trip(PesVersion::Pes21, "export_texport_21", "CO.ted");
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
fn on_pes_17_the_texport_is_disabled_and_says_why() {
    let (mut view, mut window) = open_on(PesVersion::Pes17, "export_texport_17", CO);
    let texport = button(&mut window, &mut view, "Texport\u{2026}");
    assert!(texport.is_disabled(), "no Texport on PES 2017");
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(&texport), "Texport template").as_deref(),
        Some("PES 2015 to 2017 have no Texport template")
    );
    assert!(
        !button(&mut window, &mut view, "Team TOML\u{2026}").is_disabled(),
        "the Team TOML stays"
    );
}

#[test]
fn the_open_save_is_refused_as_the_destination() {
    for (label, format) in [
        ("Team TOML\u{2026}", "Team TOML"),
        ("Texport\u{2026}", "Texport"),
    ] {
        let test = format!("export_refused_{}", format.replace(' ', "_"));
        let (mut view, mut window) = open_on(PesVersion::Pes21, &test, CO);
        let save = save_folder(&view).join("EDIT00000000");
        let before = std::fs::read(&save).unwrap();

        export(&mut window, &mut view, label, &save);

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
            save.display()
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
