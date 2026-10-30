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

/// The single-line text field holding `value` (Name holds the ruleset's
/// name, an unset Event page nothing).
fn text_field<'a>(nodes: &'a [accesskit::Node], value: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some(value))
        .unwrap_or_else(|| panic!("a text field holding {value:?}"))
}

/// Blank's one tier's Rate control on the Tiers section (its rate is
/// 75, a value no other cell of the row holds).
fn rate_spin(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("75"))
        .expect("the Rate control")
}

/// The rate of Blank's one tier.
fn rate(view: &RulesetView) -> u8 {
    view.document.as_ref().unwrap().ruleset.tiers[0].rate
}

/// The text field holding `from` focused (a click on it).
fn focus_field(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, from: &str) {
    let frame = window.draw(view, ctx);
    click(window, text_field(&frame.nodes, from), view, ctx);
}

/// `text` typed into the focused text field (the field keeps focus
/// between frames, so several calls are one typing gesture).
fn type_text(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, text: &str) {
    window.frame(vec![egui::Event::Text(text.to_string())], |ui| {
        show(ui, view, ctx)
    });
}

/// The document's ruleset's name.
fn ruleset_name(view: &RulesetView) -> String {
    view.document.as_ref().unwrap().ruleset.name.clone()
}

/// General has no Squad size field: every squad is 23.
#[test]
fn general_draws_no_squad_field() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(
        drawn(&frame.nodes, "Event logo"),
        "premise: General is drawn"
    );
    assert!(!drawn(&frame.nodes, "Squad size"), "no Squad size field");
}

/// The note under the Event logo says where the Save editor shows it.
#[test]
fn general_notes_where_the_save_editor_shows_the_logo() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(
        &frame.nodes,
        "The Save editor shows the logo on its top right while this ruleset is picked; a click \
         on it opens the event page."
    ));
}

#[test]
fn ctrl_z_during_a_drag_restores_the_value_before_it() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    view.section = Section::Tiers;
    let frame = settled(&mut window, &mut view, &ctx);
    let rate_control = rate_spin(&frame.nodes);
    // A drag over two frames, the pointer still down.
    let at = centre(rate_control);
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
    assert_ne!(rate(&view), 75, "premise: the drag changed the rate");
    // Ctrl+Z while the pointer is still down: the drag seals first, so
    // the undo lands before it.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(rate(&view), 75, "back to before the drag");
}

#[test]
fn a_text_field_and_a_click_are_two_undo_entries() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    focus_field(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "x");
    assert_eq!(ruleset_name(&view), "New rulesetx");
    // A checkbox click is its own gesture (focus moved Some→Some).
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    assert!(!has_at_most(&view, "attacking_prowess"));

    // One undo: the checkbox is back, the name kept.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(ruleset_name(&view), "New rulesetx", "the name kept");
    assert!(has_at_most(&view, "attacking_prowess"), "the checkbox back");
    // A second undo takes the name's whole gesture.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(ruleset_name(&view), "New ruleset");
}

#[test]
fn typing_in_name_is_one_entry_and_focus_move_seals_it() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // Two characters over two frames: one gesture.
    focus_field(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "a");
    type_text(&mut window, &mut view, &ctx, "b");
    // A click on dead space ends the gesture.
    let away = egui::pos2(5.0, 5.0);
    window.frame(pointer_at(away, true), |ui| show(ui, &mut view, &ctx));
    window.frame(pointer_at(away, false), |ui| show(ui, &mut view, &ctx));
    assert_eq!(ruleset_name(&view), "New rulesetab");
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(
        ruleset_name(&view),
        "New ruleset",
        "the whole gesture undone"
    );

    // Focus Name, type, then move focus to Event page and type there:
    // the name's gesture is sealed before the page edit.
    focus_field(&mut window, &mut view, &ctx, "New ruleset");
    type_text(&mut window, &mut view, &ctx, "x");
    focus_field(&mut window, &mut view, &ctx, "");
    type_text(&mut window, &mut view, &ctx, "https://a.b");
    let page = |view: &RulesetView| view.document.as_ref().unwrap().ruleset.page.clone();
    assert_eq!(
        page(&view).as_deref(),
        Some("https://a.b"),
        "premise: the page edit happened"
    );
    window.frame(pointer_at(away, true), |ui| show(ui, &mut view, &ctx));
    window.frame(pointer_at(away, false), |ui| show(ui, &mut view, &ctx));
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(page(&view), None, "the page edit undone");
    assert_eq!(ruleset_name(&view), "New rulesetx", "the name kept");
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

/// The Event logo's dialog stubbed to return `path`, then Choose
/// picture… clicked.
pub(super) fn choose_picture(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    path: &Path,
) {
    let picked = path.to_path_buf();
    view.logo.dialog = Box::new(move |_| Some(picked.clone()));
    let frame = settled(window, view, ctx);
    click_labeled(window, &frame.nodes, "Choose picture…", view, ctx);
}

/// A 40 × 40 PNG of under 3 KB written to `dir`: a gradient, which
/// compresses partly, as a small logo does. Its bytes are not the ones
/// the logo field's own encoder makes of the same pixels, so a stored
/// copy equal to the file was not encoded again.
fn small_png(dir: &Path) -> PathBuf {
    use image::ImageEncoder as _;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    // The blue channel is noise from a fixed sequence (a linear
    // congruential one), which keeps the file from compressing below
    // 2 KB.
    let mut noise: u32 = 1;
    let mut pixels = Vec::new();
    for y in 0..40u8 {
        for x in 0..40u8 {
            noise = noise.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let [_, _, blue, _] = noise.to_be_bytes();
            pixels.extend_from_slice(&[x * 6, y * 6, blue, 255]);
        }
    }
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
        .write_image(&pixels, 40, 40, image::ExtendedColorType::Rgba8)
        .expect("the picture encodes");
    assert_ne!(
        png,
        dds_convert::encode_png(&pixels, 40, 40).expect("the picture encodes"),
        "premise: the file's bytes are not a re-encoding's"
    );
    write_bytes(dir, "small.png", &png)
}

/// `bytes` written to `dir/name`; the path.
fn write_bytes(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// The document's logo decoded: its width and height in pixels.
fn logo_size(view: &RulesetView) -> (u32, u32) {
    let png = view
        .document
        .as_ref()
        .expect("a document")
        .ruleset
        .logo_png()
        .expect("a logo");
    let decoded =
        dds_convert::decode(&png, dds_convert::SourceFormat::Png).expect("the logo is a PNG");
    (decoded.width, decoded.height)
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
fn a_png_that_fits_shows_28_high_and_is_stored_as_it_is() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    assert_eq!(logo_text(&view), None, "premise: Blank has no logo");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "None"), "no logo: a weak None");
    assert!(!drawn(&frame.nodes, "Remove"), "no logo, no Remove");

    let path = small_png(&crate::test_support::scratch("event_logo_small"));
    let png = std::fs::read(&path).unwrap();
    choose_picture(&mut window, &mut view, &ctx, &path);
    assert_eq!(
        view.document.as_ref().unwrap().ruleset.logo_png(),
        Some(png.clone()),
        "the picked file is the logo, byte for byte"
    );
    let frame = settled(&mut window, &mut view, &ctx);
    let picture = logo_picture(&frame.nodes).expect("the picture is drawn");
    assert_eq!(picture.y1 - picture.y0, 28.0, "28 points high");
    let choose = bounds_of(&frame.nodes, "Choose picture…");
    assert!(beside(picture, choose), "Choose picture… after the picture");
    assert!(
        beside(choose, bounds_of(&frame.nodes, "Remove")),
        "Remove beside Choose picture…"
    );
    assert!(!drawn(&frame.nodes, "None"));

    // Remove clears the logo, and the field draws None again.
    click_labeled(&mut window, &frame.nodes, "Remove", &mut view, &ctx);
    assert_eq!(logo_text(&view), None, "Remove clears the logo");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "None"));
    assert_eq!(logo_picture(&frame.nodes), None);

    // The pick and Remove are edits like any other: the first Ctrl+Z
    // brings the logo back, the second takes the pick back.
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(
        view.document.as_ref().unwrap().ruleset.logo_png(),
        Some(png),
        "Remove undone"
    );
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(logo_text(&view), None, "the pick undone");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(drawn(&frame.nodes, "None"));
    assert_eq!(logo_picture(&frame.nodes), None);
}

// RE-GUI-19
#[test]
fn a_big_jpeg_is_made_smaller_to_fit_and_stored_as_a_png() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let dir = crate::test_support::scratch("event_logo_jpeg");
    let path = dir.join("wide.jpg");
    image::RgbImage::from_pixel(1000, 500, image::Rgb([200, 30, 30]))
        .save(&path)
        .expect("the JPEG is written");
    choose_picture(&mut window, &mut view, &ctx, &path);
    assert_eq!(logo_size(&view), (112, 56), "fitted into 168 × 56");
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(
        !contains_label(&frame.nodes, "is not a picture"),
        "no refusal"
    );
}

// RE-GUI-19
#[test]
fn a_file_that_is_no_picture_is_refused_with_its_line_and_the_logo_kept() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let dir = crate::test_support::scratch("event_logo_refused");
    choose_picture(&mut window, &mut view, &ctx, &small_png(&dir));
    let kept = logo_text(&view).expect("premise: the PNG was taken");

    let text = write_bytes(&dir, "logo.png", b"not a picture");
    choose_picture(&mut window, &mut view, &ctx, &text);
    assert_eq!(logo_text(&view).as_deref(), Some(kept.as_str()));
    let frame = settled(&mut window, &mut view, &ctx);
    let line = "logo.png is not a picture";
    assert!(
        beside(
            bounds_of(&frame.nodes, "Remove"),
            bounds_of(&frame.nodes, line)
        ),
        "the refusal beside the buttons"
    );

    // A file that does not read keeps its own line, which replaces the
    // last one.
    choose_picture(&mut window, &mut view, &ctx, &dir.join("missing.png"));
    assert_eq!(logo_text(&view).as_deref(), Some(kept.as_str()));
    let frame = settled(&mut window, &mut view, &ctx);
    assert!(contains_label(
        &frame.nodes,
        "missing.png could not be read: "
    ));
    assert!(!drawn(&frame.nodes, line), "the newest refusal replaces it");

    // A pick that is taken clears the line.
    choose_picture(&mut window, &mut view, &ctx, &small_png(&dir));
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
