//! Undo and redo over General edits (RE-GUI-14): two edits in order,
//! undone and redone by the keys and by the Edit menu's buttons; and
//! the form scrolling sideways when the preview narrows it.

use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;

use super::*;

/// The document's `stats.at_most`.
fn at_most(view: &RulesetView) -> Vec<String> {
    view.document
        .as_ref()
        .map(|document| document.ruleset.stats.at_most.clone())
        .unwrap_or_default()
}

/// Whether the document's `stats.at_most` holds `name`.
fn has_at_most(view: &RulesetView, name: &str) -> bool {
    at_most(view).iter().any(|target| target == name)
}

/// The document's `stats.rating` as labels.
fn rating(view: &RulesetView) -> Vec<String> {
    view.document
        .as_ref()
        .map(|document| {
            document
                .ruleset
                .stats
                .rating
                .iter()
                .map(|stat| stat.label().to_string())
                .collect()
        })
        .unwrap_or_default()
}

// RE-GUI-14
#[test]
fn two_edits_undo_and_redo_by_key_and_button() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // Blank's at_most holds attacking_prowess, its rating lacks
    // tight_possession (premises the clicks reverse).
    let frame = window.draw(&mut view, &ctx);
    assert!(has_at_most(&view, "attacking_prowess"));
    assert!(!rating(&view).iter().any(|name| name == "tight_possession"));

    // Edit 1: uncheck "Attacking Prowess" in the at-most list (the
    // second same-named checkbox, the rating list's being above it).
    let prowess = nodes_labeled(&frame.nodes, "Attacking Prowess");
    assert_eq!(prowess.len(), 2, "one checkbox per checklist");
    click(&mut window, prowess[1], &mut view, &ctx);
    // Edit 2: check "Tight Possession" in the rating list (the first).
    click_drawn_nth(&mut window, &mut view, &ctx, "Tight Possession", 0);
    assert!(!has_at_most(&view, "attacking_prowess"));
    assert!(rating(&view).iter().any(|name| name == "tight_possession"));

    // Ctrl+Z undoes the second edit only; Ctrl+Y redoes it.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert!(
        !rating(&view).iter().any(|name| name == "tight_possession"),
        "the second edit undone"
    );
    assert!(!has_at_most(&view, "attacking_prowess"), "the first kept");
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Y);
    assert!(
        rating(&view).iter().any(|name| name == "tight_possession"),
        "the second edit redone"
    );

    // Two more edits, undone and redone by the Edit menu's buttons:
    // "Attacking Prowess" checked in the rating list (the first) and
    // "Defensive Prowess" unchecked in the at-most list (the second).
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 0);
    click_drawn_nth(&mut window, &mut view, &ctx, "Defensive Prowess", 1);
    assert!(rating(&view).iter().any(|name| name == "attacking_prowess"));
    assert!(!has_at_most(&view, "defensive_prowess"));

    menu_click(&mut window, &mut view, &ctx, "Edit", "Undo");
    assert!(
        has_at_most(&view, "defensive_prowess"),
        "the second edit undone by the button"
    );
    assert!(
        rating(&view).iter().any(|name| name == "attacking_prowess"),
        "the first kept"
    );
    menu_click(&mut window, &mut view, &ctx, "Edit", "Redo");
    assert!(
        !has_at_most(&view, "defensive_prowess"),
        "the second edit redone by the button"
    );
}

/// The Name field's node (a single-line TextEdit).
fn name_field<'a>(nodes: &'a [accesskit::Node], value: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some(value))
        .expect("the Name field")
}

/// The Squad size control's node (Blank's squad is 23).
fn squad_spin(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("23"))
        .expect("the Squad size control")
}

/// The Name field focused (a click on it).
fn focus_name(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, from: &str) {
    let frame = window.draw(view, ctx);
    click(window, name_field(&frame.nodes, from), view, ctx);
}

/// `text` typed into the focused Name field (the field keeps focus
/// between frames, so several calls are one typing gesture).
fn type_text(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, text: &str) {
    window.frame(vec![egui::Event::Text(text.to_string())], |ui| {
        show(ui, view, ctx)
    });
}

/// A drag on a DragValue's node over `steps` frames, `dx` pixels a frame;
/// the pointer released at the end position.
fn drag_value(
    window: &mut Headless,
    node: &accesskit::Node,
    steps: u8,
    dx: f32,
    view: &mut RulesetView,
    ctx: &ToolContext,
) {
    let at = centre(node);
    window.frame(pointer_at(at, true), |ui| show(ui, view, ctx));
    for step in 1..=steps {
        window.frame(
            vec![egui::Event::PointerMoved(egui::pos2(
                at.x + dx * f32::from(step),
                at.y,
            ))],
            |ui| show(ui, view, ctx),
        );
    }
    window.frame(
        pointer_at(egui::pos2(at.x + dx * f32::from(steps), at.y), false),
        |ui| show(ui, view, ctx),
    );
}

/// The document's ruleset's name, at-most list and squad size.
fn basics(view: &RulesetView) -> (String, Vec<String>, u8) {
    let ruleset = &view.document.as_ref().unwrap().ruleset;
    (
        ruleset.name.clone(),
        ruleset.stats.at_most.clone(),
        ruleset.squad_size,
    )
}

#[test]
fn ctrl_z_during_a_drag_restores_the_value_before_it() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = window.draw(&mut view, &ctx);
    let squad = squad_spin(&frame.nodes);
    // A drag over two frames, the pointer still down.
    let at = centre(squad);
    window.frame(pointer_at(at, true), |ui| show(ui, &mut view, &ctx));
    for step in 1..=2 {
        window.frame(
            vec![egui::Event::PointerMoved(egui::pos2(
                at.x + 30.0 * step as f32,
                at.y,
            ))],
            |ui| show(ui, &mut view, &ctx),
        );
    }
    let dragged = view.document.as_ref().unwrap().ruleset.squad_size;
    assert_ne!(dragged, 23, "premise: the drag changed the squad size");
    // Ctrl+Z while the pointer is still down: the drag seals first, so
    // the undo lands before it.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(
        view.document.as_ref().unwrap().ruleset.squad_size,
        23,
        "back to before the drag"
    );
}

#[test]
fn a_text_field_and_a_click_are_two_undo_entries() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    focus_name(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "x");
    assert_eq!(basics(&view).0, "New rulesetx");
    // A checkbox click is its own gesture (focus moved Some→Some).
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    assert!(!has_at_most(&view, "attacking_prowess"));

    // One undo: the checkbox is back, the name kept.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(basics(&view).0, "New rulesetx", "the name kept");
    assert!(has_at_most(&view, "attacking_prowess"), "the checkbox back");
    // A second undo takes the name's whole gesture.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(basics(&view).0, "New ruleset");
}

#[test]
fn typing_in_name_is_one_entry_and_focus_move_seals_it() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // Two characters over two frames: one gesture.
    focus_name(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "a");
    type_text(&mut window, &mut view, &ctx, "b");
    // A click on dead space ends the gesture.
    let away = egui::pos2(5.0, 5.0);
    window.frame(pointer_at(away, true), |ui| show(ui, &mut view, &ctx));
    window.frame(pointer_at(away, false), |ui| show(ui, &mut view, &ctx));
    assert_eq!(basics(&view).0, "New rulesetab");
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(basics(&view).0, "New ruleset", "the whole gesture undone");

    // Focus Name, type, then move focus to Squad size: the name's
    // gesture is sealed before the squad edit.
    focus_name(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "x");
    let frame = window.draw(&mut view, &ctx);
    let squad = squad_spin(&frame.nodes);
    drag_value(&mut window, squad, 2, 30.0, &mut view, &ctx);
    let squad_size = basics(&view).2;
    assert_ne!(squad_size, 23, "premise: the squad edit happened");
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(basics(&view).2, 23, "the squad edit undone");
    assert_eq!(basics(&view).0, "New rulesetx", "the name kept");
}

#[test]
fn the_checklists_draw_held_stats_checked_and_a_toggle_keeps_the_rest() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = window.draw(&mut view, &ctx);
    // Blank's rating holds "Dribbling" but not "Tight Possession" (the
    // checklist's drawn state, `Toggled` on the checkbox nodes).
    let toggled = |nodes: &[accesskit::Node], label: &str| {
        nodes_labeled(nodes, label)
            .into_iter()
            .map(|node| node.toggled())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        toggled(&frame.nodes, "Dribbling"),
        [
            Some(accesskit::Toggled::True),
            Some(accesskit::Toggled::False)
        ],
        "held in rating, not in at-most"
    );
    assert_eq!(
        toggled(&frame.nodes, "Tight Possession"),
        [
            Some(accesskit::Toggled::False),
            Some(accesskit::Toggled::False)
        ],
        "held in neither"
    );

    // Uncheck "Dribbling" in the rating list (the first): the others stay.
    click(
        &mut window,
        nodes_labeled(&frame.nodes, "Dribbling")[0],
        &mut view,
        &ctx,
    );
    let rating = rating(&view);
    assert!(!rating.iter().any(|name| name == "dribbling"), "unticked");
    for held in ["ball_control", "speed", "stamina"] {
        assert!(
            rating.iter().any(|name| name == held),
            "{held} is still rated"
        );
    }
    let frame = window.draw(&mut view, &ctx);
    assert_eq!(
        toggled(&frame.nodes, "Ball Control")[0],
        Some(accesskit::Toggled::True),
        "the neighbours stay checked"
    );
}

/// The outer rect of the panel `salt` names (egui persists a panel's
/// `PanelState` under its scope's id; the central panel's right edge is
/// the preview panel's left).
fn panel_outer_rect(window: &Headless, salt: &str) -> egui::Rect {
    let id = window
        .ctx
        .viewport_id()
        .root_ui_id()
        .with_salt(egui::IdSalt::new(salt));
    egui::containers::panel::PanelState::load(&window.ctx, id)
        .unwrap_or_else(|| panic!("the {salt} panel's state"))
        .outer_rect
}

// The draft's `.center` scrolls, it never clips: at the smallest default
// window (1037 × 648, `window_size`'s smallest) with Show preview on,
// the General checklist's fifth column starts past the central panel's
// right edge and a horizontal scroll over the form brings it in reach.
#[test]
fn the_form_scrolls_sideways_to_its_rightmost_checkbox() {
    let (ctx, _events) = crate::test_support::context();
    let mut window = Headless::sized(1037.0, 648.0);
    let mut view = RulesetView::new();
    open_document(&mut view, crate::document::blank());
    view.show_preview = true;

    // The first checklist row's rightmost checkbox (the rating list's
    // fifth stat); the same label sits in the at-most list below, so the
    // first drawn top to bottom is the rating one's.
    let label = crate::summary::human(ABILITY_STATS[4]);
    let frame = window.draw(&mut view, &ctx);
    let right = f64::from(panel_outer_rect(&window, "ruleset_preview").min.x);
    let bounds = nodes_labeled(&frame.nodes, &label)[0]
        .bounds()
        .expect("the checkbox is drawn");
    assert!(
        bounds.x1 > right,
        "premise: the rightmost checkbox starts past the panel's right edge ({bounds:?} vs x={right})"
    );

    // A horizontal wheel scroll over the form.
    let frame = scroll_until(
        &mut window,
        &mut view,
        &ctx,
        egui::pos2(300.0, 300.0),
        egui::vec2(-80.0, 0.0),
        |frame| {
            nodes_labeled(&frame.nodes, &label)[0]
                .bounds()
                .expect("the checkbox is drawn")
                .x1
                <= right
        },
    )
    .expect("a horizontal scroll brought the checkbox inside");

    let was_rated = rating(&view).iter().any(|name| name == ABILITY_STATS[4]);
    click(
        &mut window,
        nodes_labeled(&frame.nodes, &label)[0],
        &mut view,
        &ctx,
    );
    assert_eq!(
        rating(&view).iter().any(|name| name == ABILITY_STATS[4]),
        !was_rated,
        "the click toggled the stat"
    );
}

/// A real event logo: DBC7's, as the drafts draw it (16 KB).
fn dbc7_logo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/prototypes/assets/logos/dbc7.png")
}

/// The Event logo's dialog stubbed to return `path`, then Choose PNG…
/// clicked.
fn choose_png(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, path: &Path) {
    let picked = path.to_path_buf();
    view.logo.dialog = Box::new(move |_| Some(picked.clone()));
    let frame = settled(window, view, ctx);
    click_labeled(window, &frame.nodes, "Choose PNG…", view, ctx);
}

/// The document's logo text.
fn logo_text(view: &RulesetView) -> Option<String> {
    view.document
        .as_ref()
        .expect("a document")
        .ruleset
        .logo
        .clone()
}

/// The bounds of the node labeled (or valued) `label`.
fn bounds_of(nodes: &[accesskit::Node], label: &str) -> accesskit::Rect {
    node(nodes, label)
        .bounds()
        .unwrap_or_else(|| panic!("{label:?} has bounds"))
}

/// The Event logo field's picture: the image node (the row's label
/// carries the same words).
fn logo_picture(nodes: &[accesskit::Node]) -> Option<accesskit::Rect> {
    nodes
        .iter()
        .find(|node| node.role() == Role::Image && node.label() == Some("Event logo"))
        .and_then(accesskit::Node::bounds)
}

/// Whether `right` stands on `left`'s row, after it.
fn beside(left: accesskit::Rect, right: accesskit::Rect) -> bool {
    right.x0 >= left.x1 && right.y0 < left.y1 && right.y1 > left.y0
}

// RE-GUI-19
#[test]
fn a_chosen_png_shows_28_high_with_remove_beside_choose() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    assert_eq!(logo_text(&view), None, "premise: Blank has no logo");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "None"), "no logo: a weak None");
    assert!(!drawn(&frame.nodes, "Remove"), "no logo, no Remove");

    choose_png(&mut window, &mut view, &ctx, &dbc7_logo());
    let png = std::fs::read(dbc7_logo()).unwrap();
    assert_eq!(
        view.document.as_ref().unwrap().ruleset.logo_png(),
        Some(png),
        "the picked file is the logo"
    );
    let frame = settled(&mut window, &mut view, &ctx);
    let picture = logo_picture(&frame.nodes).expect("the picture is drawn");
    assert_eq!(picture.y1 - picture.y0, 28.0, "28 points high");
    let choose = bounds_of(&frame.nodes, "Choose PNG…");
    assert!(beside(picture, choose), "Choose PNG… after the picture");
    assert!(
        beside(choose, bounds_of(&frame.nodes, "Remove")),
        "Remove beside Choose PNG…"
    );
    assert!(!drawn(&frame.nodes, "None"));

    // The pick is an edit like any other: Ctrl+Z takes it back, and the
    // field draws None again.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(logo_text(&view), None, "the pick undone");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "None"));
    assert_eq!(logo_picture(&frame.nodes), None);
}

// RE-GUI-19
#[test]
fn a_jpeg_or_an_oversize_png_is_refused_with_its_line_and_the_logo_kept() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    choose_png(&mut window, &mut view, &ctx, &dbc7_logo());
    let kept = logo_text(&view).expect("premise: the PNG was taken");

    let jpeg = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../libs/dds_convert/tests/fixtures/source_opaque.jpg");
    choose_png(&mut window, &mut view, &ctx, &jpeg);
    assert_eq!(logo_text(&view).as_deref(), Some(kept.as_str()));
    let frame = settled(&mut window, &mut view, &ctx);
    let line = "source_opaque.jpg is not a PNG picture";
    assert!(
        beside(
            bounds_of(&frame.nodes, "Remove"),
            bounds_of(&frame.nodes, line)
        ),
        "the refusal beside the buttons"
    );

    // The PNG signature, then padding to one byte over 64 KB.
    let dir = crate::test_support::scratch("event_logo_oversize");
    let mut big = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    big.resize(65537, 0);
    let big_path = dir.join("big.png");
    std::fs::write(&big_path, big).unwrap();
    choose_png(&mut window, &mut view, &ctx, &big_path);
    assert_eq!(logo_text(&view).as_deref(), Some(kept.as_str()));
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "big.png is over 64 KB"));
    assert!(!drawn(&frame.nodes, line), "the newest refusal replaces it");

    choose_png(&mut window, &mut view, &ctx, &dir.join("missing.png"));
    assert_eq!(logo_text(&view).as_deref(), Some(kept.as_str()));
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(contains_label(
        &frame.nodes,
        "missing.png could not be read: "
    ));

    // A pick that is taken clears the line.
    choose_png(&mut window, &mut view, &ctx, &dbc7_logo());
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(!contains_label(&frame.nodes, "missing.png"));
}

/// The Event page box: General's one text box not holding the name.
fn page_box<'a>(nodes: &'a [accesskit::Node], name: &str) -> &'a accesskit::Node {
    let boxes: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::TextInput && node.value() != Some(name))
        .collect();
    assert_eq!(boxes.len(), 1, "one text box beside Name");
    boxes[0]
}

// RE-GUI-19
#[test]
fn an_event_page_that_is_no_web_address_is_a_general_problem() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    assert_eq!(rail_labels(&frame.nodes)[0], "General", "premise: no mark");
    click(
        &mut window,
        page_box(&frame.nodes, "New ruleset"),
        &mut view,
        &ctx,
    );
    type_text(&mut window, &mut view, &ctx, "www.example");
    let ruleset = &view.document.as_ref().unwrap().ruleset;
    assert_eq!(ruleset.page.as_deref(), Some("www.example"));
    let frame = window.draw(&mut view, &ctx);
    assert_eq!(rail_labels(&frame.nodes)[0], "General ⚠");
    assert!(contains_label(&frame.nodes, "✖ 1 problem:"));

    // An emptied box stores no page, and the problem goes.
    let backspaces = vec![key(egui::Key::Backspace, egui::Modifiers::NONE); "www.example".len()];
    window.frame(backspaces, |ui| show(ui, &mut view, &ctx));
    assert_eq!(view.document.as_ref().unwrap().ruleset.page, None);
    let frame = window.draw(&mut view, &ctx);
    assert_eq!(rail_labels(&frame.nodes)[0], "General");
}
