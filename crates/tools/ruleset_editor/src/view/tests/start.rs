//! The start page: its entries in order (RE-GUI-01), each template and
//! Blank opening clean on General and saving back equal (RE-GUI-02),
//! and the refused-open listing (RE-FILE-03).

use super::*;

/// The buttons in `labels` appear in `expected`'s order (a subsequence,
/// not necessarily contiguous).
fn assert_order(labels: &[String], expected: &[&str]) {
    let mut at = 0;
    for entry in expected {
        let found = labels[at..]
            .iter()
            .position(|label| label == entry)
            .unwrap_or_else(|| panic!("{entry:?} not drawn after index {at}: {labels:?}"));
        at += found + 1;
    }
}

// RE-GUI-01
#[test]
fn the_start_page_offers_its_entries_in_order() {
    let (ctx, _events, mut window, mut view) = fresh();
    let frame = window.draw(&mut view, &ctx);
    let labels = labels(&frame.nodes);
    assert_order(
        &labels,
        &[
            "New from template",
            "4cc Autumn 26 (the official rules)",
            "4cc Spring 26",
            "/vg/ League 27",
            "/vg/ League 26",
            "/vt/ League 11",
            "/vt/ League 10",
            "Divegrass Before Christmas 7",
            "Blank: one tier, no brackets, the universal rules on",
            "Sensible",
            "Weird",
            "Crazy",
            "Random",
            "Open file…",
            "Recent files",
        ],
    );
}

// RE-GUI-01
#[test]
fn the_recent_files_list_the_later_open_first_and_skip_a_deleted_file() {
    let dir = crate::test_support::scratch("recent_files");
    let first = write_file(&dir, "first.4ccrules", &valid_file());
    let second = write_file(&dir, "second.4ccrules", &valid_file());
    let saved = dir.join("saved.4ccrules");
    let (ctx, _events) = crate::test_support::context();

    // Two files opened in turn, then the tool restarted: the later
    // heads the list.
    let mut view = opened_view(&ctx, &first);
    open_path(&mut view, &ctx, &second);
    let mut restarted = RulesetView::new();
    let mut window = Headless::new();
    let frame = window.draw(&mut restarted, &ctx);
    assert_order(
        &labels(&frame.nodes),
        &["second.4ccrules", "first.4ccrules"],
    );

    // One opened and one saved, restarted: the saved heads it.
    let mut view = opened_view(&ctx, &first);
    assert!(save_to(&mut view, &ctx, &saved));
    let mut restarted = RulesetView::new();
    let frame = window.draw(&mut restarted, &ctx);
    assert_order(&labels(&frame.nodes), &["saved.4ccrules", "first.4ccrules"]);

    // A listed file deleted is no longer listed.
    std::fs::remove_file(&saved).unwrap();
    let frame = window.draw(&mut restarted, &ctx);
    assert!(
        !drawn(&frame.nodes, "saved.4ccrules"),
        "the deleted file is filtered"
    );
    assert!(drawn(&frame.nodes, "first.4ccrules"));
}

/// The rail's labels in the order the rail shows them, `Custom checks`
/// when the document has them. They are spelled out, not read from
/// `Section::RAIL`, so a swapped constant fails here too.
fn rail_expected(custom_checks: bool) -> Vec<String> {
    [
        "General",
        "Tiers",
        "Heights",
        "Cards",
        "Universal rules",
        "Specials",
        "Suggestions",
        "Custom checks",
        "Summary",
    ]
    .into_iter()
    .filter(|label| *label != "Custom checks" || custom_checks)
    .map(str::to_string)
    .collect()
}

/// The rail clean: every section in order, no problem mark, and the
/// bottom bar's clean line.
fn assert_clean_rail(nodes: &[accesskit::Node], custom_checks: bool) {
    assert_eq!(rail_labels(nodes), rail_expected(custom_checks));
    assert!(
        !labels(nodes).iter().any(|label| label.contains('⚠')),
        "no rail entry carries a problem mark"
    );
    assert!(
        drawn(nodes, "✔ No problems found"),
        "the bottom bar's clean line"
    );
}

/// `index`'s template opened through its button: the document's name,
/// General drawn, the rail's sections in order with no problem mark,
/// and File > Save as… writing a file `Rules::load` reads back equal.
fn template_opens_clean(index: usize, test: &str) {
    let templates = crate::view::start::templates();
    let template = &templates[index];
    let dir = crate::test_support::scratch(test);
    let (ctx, _events, mut window, mut view) = fresh();
    click_drawn(&mut window, &mut view, &ctx, &template.ruleset.name);

    let document = view.document.as_ref().expect("a document opened");
    assert_eq!(document.ruleset.name, template.ruleset.name);
    assert!(document.path.is_none(), "an unsaved document");
    assert!(document.is_dirty(), "unsaved is dirty");
    assert_eq!(view.section, Section::General);
    let frame = window.draw(&mut view, &ctx);
    assert_clean_rail(&frame.nodes, template.custom_checks.is_some());

    let path = dir.join("saved.4ccrules");
    save_as_via_menu(&mut window, &mut view, &ctx, &path);
    assert!(path.is_file(), "Save as… wrote the file");
    let text = std::fs::read_to_string(&path).unwrap();
    let rules = aatf::Rules::load(&text).expect("the saved file loads");
    assert_eq!(rules.ruleset(), &template.ruleset);
}

// RE-GUI-02
#[test]
fn each_template_opens_clean_and_saves_back_equal() {
    for index in 0..crate::view::start::templates().len() {
        template_opens_clean(index, &format!("template_{index}"));
    }
}

// RE-GUI-02
#[test]
fn blank_opens_as_new_ruleset_and_saves_back_equal() {
    let dir = crate::test_support::scratch("template_blank");
    let (ctx, _events, mut window, mut view) = fresh();
    click_drawn(
        &mut window,
        &mut view,
        &ctx,
        "Blank: one tier, no brackets, the universal rules on",
    );

    let document = view.document.as_ref().expect("a document opened");
    assert_eq!(document.ruleset.name, "New ruleset");
    assert!(document.path.is_none(), "unsaved");
    assert!(document.is_dirty(), "unsaved is dirty");
    assert_eq!(view.section, Section::General);
    let frame = window.draw(&mut view, &ctx);
    assert_clean_rail(&frame.nodes, false);

    let path = dir.join("saved.4ccrules");
    save_as_via_menu(&mut window, &mut view, &ctx, &path);
    assert!(path.is_file(), "Save as… wrote the file");
    let text = std::fs::read_to_string(&path).unwrap();
    let rules = aatf::Rules::load(&text).expect("the saved file loads");
    assert_eq!(rules.ruleset(), &crate::document::blank());
}

/// `text` opened as `rules.4ccrules` under a fresh scratch folder: it is not
/// opened, and the start page's labels as drawn (the file's name, then
/// one line per finding).
fn refused_open_labels(test: &str, text: &str) -> Vec<String> {
    let dir = crate::test_support::scratch(test);
    let file = write_file(&dir, "rules.4ccrules", text);
    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &file);
    assert!(view.document.is_none(), "not opened in the forms");
    let frame = window.draw(&mut view, &ctx);
    labels(&frame.nodes)
}

/// `labels` (the refused-open listing) holds a finding matching
/// `predicate`.
fn assert_finding(labels: &[String], predicate: impl Fn(&str) -> bool) {
    assert!(
        labels.iter().any(|label| predicate(label)),
        "the finding names it: {labels:?}"
    );
}

// RE-FILE-03
#[test]
fn a_parse_error_is_refused_with_its_line_and_column() {
    let labels = refused_open_labels("open_parse", &crate::test_support::parse_error_file());
    assert_finding(&labels, |label| label == "Could not open rules.4ccrules:");
    assert_finding(&labels, |label| label.starts_with("line=12, column="));
}

// RE-FILE-03
#[test]
fn a_schema_error_is_refused_with_its_key_path() {
    let labels = refused_open_labels("open_schema", &crate::test_support::schema_error_file());
    assert_finding(&labels, |label| label.contains("tiers[2].free.tricks"));
}

// RE-FILE-03
#[test]
fn a_duplicate_tier_id_is_refused_with_its_key_path() {
    let labels = refused_open_labels("open_duplicate", &crate::test_support::duplicate_id_file());
    assert_finding(&labels, |label| {
        label.contains("tiers[1]") && label.contains("gold")
    });
}

// RE-FILE-03
#[test]
fn a_broken_file_is_refused_over_an_open_document() {
    let (ctx, _events, path, mut view) = official_opened("open_over_doc");
    let dir = path.parent().expect("the scratch folder").to_path_buf();
    let broken = write_file(
        &dir,
        "broken.4ccrules",
        &crate::test_support::parse_error_file(),
    );
    open_path(&mut view, &ctx, &broken);
    assert!(view.document.is_some(), "the open document is kept");
    assert!(
        matches!(view.dialog, Some(Dialog::OpenRefused { .. })),
        "the refusal is a dialog over the document"
    );
    // Its contents join the tree one frame after the modal's sizing
    // pass.
    let mut window = Headless::new();
    window.draw(&mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(contains_label(
        &frame.nodes,
        "Could not open broken.4ccrules:"
    ));
    assert!(contains_label(&frame.nodes, "line=12"));
    click_labeled(&mut window, &frame.nodes, "OK", &mut view, &ctx);
    assert!(view.dialog.is_none(), "OK closes the refusal");
}

// A refused open's findings are gone when the start page comes back
// (the template pick replaced the document).
#[test]
fn a_template_pick_clears_the_refused_open_lines() {
    let dir = crate::test_support::scratch("stale_refusal");
    let broken = write_file(
        &dir,
        "broken.4ccrules",
        &crate::test_support::parse_error_file(),
    );
    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &broken);
    let frame = window.draw(&mut view, &ctx);
    assert!(contains_label(
        &frame.nodes,
        "Could not open broken.4ccrules:"
    ));

    // Pick a template, then File > New from template…: no "Could not
    // open" line on the page's return.
    click_prefix(&mut window, &frame.nodes, "4cc Autumn 26", &mut view, &ctx);
    assert!(view.document.is_some(), "the template opened");
    open_start_modal(&mut window, &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(
        !contains_label(&frame.nodes, "Could not open"),
        "the refusal is gone"
    );
}

// `window_size`'s smallest default is 1037 × 648; the start page as a
// modal with ten recent files is ~615 points tall — inside it, but
// over a window the member resized shorter (the shell sets no
// minimum). A vertical scroll over the modal then brings its Close
// back in reach. The window here is the smallest default's width,
// resized shorter.
#[test]
fn the_start_modal_scrolls_when_it_outgrows_the_screen() {
    let dir = crate::test_support::scratch("start_modal_scroll");
    // Ten existing recent files: only those that exist are listed.
    let recent: Vec<String> = (0..10)
        .map(|index| {
            write_file(&dir, &format!("recent_{index}.4ccrules"), &valid_file())
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let (ctx, _events) = crate::test_support::context();
    seed_settings(&ctx, &dir, recent);
    let mut window = Headless::sized(1037.0, 500.0);
    let mut view = RulesetView::new();
    blank_dirty(&mut view);
    open_start_modal(&mut window, &mut view, &ctx);

    // Premise: the modal's content is taller than the screen — Close
    // sits below its bottom edge until scrolled to.
    let frame = window.draw(&mut view, &ctx);
    let close = node(&frame.nodes, "Close");
    let over_x = centre(close).x;
    assert!(
        close.bounds().expect("drawn").y1 > 500.0,
        "premise: the modal is taller than the screen (Close {:?})",
        close.bounds()
    );

    // A vertical wheel scroll over the modal brings Close inside the
    // screen.
    scroll_until(
        &mut window,
        &mut view,
        &ctx,
        egui::pos2(over_x, 300.0),
        egui::vec2(0.0, -80.0),
        |frame| node(&frame.nodes, "Close").bounds().expect("drawn").y1 <= 500.0,
    )
    .expect("a scroll brought Close inside the screen");

    // The wheel scroll animates over several frames: the button's
    // first on-screen bounds are mid-animation, so draw until egui asks
    // for no next frame and click the fresh ones. Two equal tops are no
    // proof: the animation's tail moves under half a point a frame, so
    // its pixel-rounded top can repeat while it still moves.
    let top = |frame: &Frame| node(&frame.nodes, "Close").bounds().expect("drawn").y0;
    let mut frame = window.draw(&mut view, &ctx);
    for _ in 0..60 {
        if !frame.repaints {
            break;
        }
        frame = window.draw(&mut view, &ctx);
    }
    assert!(!frame.repaints, "the scroll still animates after 60 frames");
    assert_eq!(
        top(&frame),
        top(&window.draw(&mut view, &ctx)),
        "the scroll settled"
    );
    click(&mut window, node(&frame.nodes, "Close"), &mut view, &ctx);
    assert!(!view.start_open, "Close put the start page away");
    assert_name(&view, "Edited");
    assert!(view.document.as_ref().unwrap().is_dirty());
}

// The modal's height cap is for a window shrunk under an open modal
// (the shell sets no minimum): egui bounds a new modal's first frame
// itself (a new `Area` opens at `spacing.default_area_size`, at most
// the screen), so only then does the cap bind. Scrolled to its end,
// the modal's last row is back on the screen.
#[test]
fn the_start_modal_stays_on_a_shrunk_screen() {
    let dir = crate::test_support::scratch("start_modal_shrink");
    let recent: Vec<String> = (0..10)
        .map(|index| {
            write_file(&dir, &format!("recent_{index}.4ccrules"), &valid_file())
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let (ctx, _events) = crate::test_support::context();
    seed_settings(&ctx, &dir, recent);
    let mut window = Headless::sized(1037.0, 648.0);
    let mut view = RulesetView::new();
    blank_dirty(&mut view);
    open_start_modal(&mut window, &mut view, &ctx);

    // The modal opened on the 1037 x 648 screen and settled; its last
    // row's x is where a scroll over it lands.
    let frame = settled(&mut window, &mut view, &ctx);
    let over_x = centre(node(&frame.nodes, "Close")).x;

    // The window shrunk under the open modal: Close sits below the
    // 300-point screen until scrolled to.
    window.screen = egui::vec2(1037.0, 300.0);
    let frame = settled(&mut window, &mut view, &ctx);
    let close = node(&frame.nodes, "Close");
    assert!(
        close.bounds().expect("drawn").y1 > 300.0,
        "premise: the modal outgrows the shrunk screen (Close {:?})",
        close.bounds()
    );
    let frame = scroll_until(
        &mut window,
        &mut view,
        &ctx,
        egui::pos2(over_x, 150.0),
        egui::vec2(0.0, -80.0),
        |frame| node(&frame.nodes, "Close").bounds().expect("drawn").y1 <= 300.0,
    )
    .expect("a wheel scroll brings Close onto the shrunk screen");
    // The cap keeps the scroll viewport inside the screen: a bar's
    // track is its viewport's rect, and every drawn one (the modal's
    // among them) lies inside the screen.
    let tracks: Vec<_> = frame
        .nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .filter_map(|node| node.bounds())
        .collect();
    assert!(!tracks.is_empty(), "the modal's scroll bar is drawn");
    for track in tracks {
        assert!(
            track.y0 >= 0.0 && track.y1 <= 300.0,
            "a scroll viewport inside the 300-point screen ({track:?})"
        );
    }
}
