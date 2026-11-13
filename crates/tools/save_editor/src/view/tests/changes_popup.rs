//! The toolbar's list of unsaved changes (SE-GUI-43): a click on the
//! `• unsaved changes` label opens the close prompt's table in a popup
//! under it, with none of the prompt's answers, and each closing path
//! closes it.

use super::prompt::table_rows;
use super::*;
use crate::view::team::{channel_to_show, format_hex};

/// The label's text.
const LABEL: &str = "\u{2022} unsaved changes";

/// The text of a row's X.
const X: &str = "\u{d7}";

/// The nodes under the label's: egui hangs the popup's area there, so
/// these are the list's own cells (none while it is closed).
fn list_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let label = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some(LABEL))
        .map(|(id, _)| *id)
        .expect("the label is drawn");
    let mut nodes = Vec::new();
    let mut stack = by_id[&label].children().to_vec();
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        nodes.push(node);
    }
    nodes
}

/// One frame of the view: the list's rows, empty while it is closed.
fn list_rows(window: &mut Headless, view: &mut SaveView) -> Vec<(String, String, String)> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    table_rows(&list_nodes(&pairs))
}

/// A click on the label, then a frame for the popup's first sizing pass.
fn click_label(window: &mut Headless, view: &mut SaveView) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(LABEL))
        .expect("the label is drawn");
    click(window, label, view);
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
}

/// One frame of the open list: the label's bounds and the popup's outer
/// box, the frame's painted rect (the draft's `.chpop`: the menu's corner
/// radius, the panel's fill, the noninteractive stroke, painted inside the
/// box).
/// egui paints a shadowed frame as a `Shape::Vec` of the shadow and the
/// rect, so the shapes are walked into.
fn label_and_popup(window: &mut Headless, view: &mut SaveView) -> (accesskit::Rect, egui::Rect) {
    fn rects<'a>(shape: &'a egui::Shape, found: &mut Vec<&'a egui::epaint::RectShape>) {
        match shape {
            egui::Shape::Rect(rect) => found.push(rect),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| rects(shape, found)),
            _ => {}
        }
    }
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let label = label_bounds(&nodes, LABEL);
    let visuals = window.ctx.global_style().visuals.clone();
    let mut found = Vec::new();
    for clipped in &shapes {
        rects(&clipped.shape, &mut found);
    }
    let frames: Vec<egui::Rect> = found
        .into_iter()
        .filter(|rect| {
            rect.corner_radius == visuals.menu_corner_radius
                && rect.fill == visuals.panel_fill
                && rect.stroke == visuals.widgets.noninteractive.bg_stroke
        })
        .map(|rect| rect.rect)
        .collect();
    assert_eq!(frames.len(), 1, "one popup frame painted: {frames:?}");
    (label, frames[0])
}

/// Asserts that the value cell reading `hex` among `nodes` paints its
/// swatch: a 12 × 12 rect filled `colour`, its corners the theme's radius,
/// its 1-point stroke the theme's swatch edge, 5 points left of the hex and
/// centred on its line.
pub(super) fn assert_swatch_before(
    window: &Headless,
    nodes: &[&accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    hex: &str,
    colour: egui::Color32,
) {
    let cell = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(hex))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no value cell {hex:?}"));
    let centre_y = (cell.y0 + cell.y1) / 2.0;
    let swatch = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.fill == colour
                    && (f64::from(rect.rect.center().y) - centre_y).abs() <= 1.0
                    && f64::from(rect.rect.max.x) <= cell.x0 + 0.5
                    && f64::from(rect.rect.max.x) >= cell.x0 - 8.0 =>
            {
                Some(rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no swatch filled {colour:?} left of {hex:?} at {cell:?}"));
    assert_eq!(
        swatch.rect.size(),
        egui::vec2(12.0, 12.0),
        "the swatch is 12 x 12"
    );
    assert_eq!(
        swatch.corner_radius,
        window
            .ctx
            .global_style()
            .visuals
            .widgets
            .noninteractive
            .corner_radius
    );
    let edge = studio_core::theme::palette(&window.ctx.global_style().visuals).swatch_edge;
    assert_eq!(
        swatch.stroke,
        egui::Stroke::new(1.0, edge),
        "a 1-point dark edge"
    );
    assert!(
        (cell.x0 - f64::from(swatch.rect.max.x) - 5.0).abs() <= 0.5,
        "5 points before the hex: swatch {:?}, cell {cell:?}",
        swatch.rect
    );
}

/// The PES 21 fixture with player 70101's Height and Speed edited and
/// team 701's colour 1 set to the stored channels 10, 20, 30; the old
/// colour's shown channels.
fn three_changes(test: &str) -> (SaveView, [u8; 3]) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    assert_eq!(edited.name, "PLACEHOLDER", "premise: the fixture name");
    assert_ne!(
        (edited.basic.height, edited.stats.speed),
        (170, 90),
        "premise: real changes"
    );
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut edited = player_entry(&view, 70101);
    edited.stats.speed = 90;
    view.session.as_mut().unwrap().edit_player(2, edited);
    let mut team = team_entry(&view, 701);
    assert_eq!(display_name(&team.name), "/3/", "premise: the fixture team");
    let colors = team.colors.as_mut().expect("PES 21 stores team colours");
    let old = [colors[0].red, colors[0].green, colors[0].blue];
    assert_ne!(old, [10, 20, 30], "premise: a real change");
    colors[0].red = 10;
    colors[0].green = 20;
    colors[0].blue = 30;
    view.session.as_mut().unwrap().edit_team(3, team);
    (view, old.map(channel_to_show))
}

// SE-GUI-43
#[test]
fn the_labels_click_lists_the_changes_without_the_prompts_answers() {
    let (mut view, old) = three_changes("changes_list_open");
    let old_speed = view
        .session
        .as_ref()
        .unwrap()
        .baseline_player(70101)
        .unwrap()
        .stats
        .speed;
    let mut window = Headless::new();
    assert!(
        list_rows(&mut window, &mut view).is_empty(),
        "premise: the list starts closed"
    );

    click_label(&mut window, &mut view);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = list_nodes(&pairs);
    let old_hex = format_hex(old);
    // The stored 10, 20, 30 shown as 8-bit channels: 40, 81, 121.
    let new_hex = "#285179";
    assert_eq!(
        table_rows(&nodes),
        [
            (
                "PLACEHOLDER · Height".to_owned(),
                "180".to_owned(),
                "170".to_owned()
            ),
            (
                "PLACEHOLDER · Speed".to_owned(),
                old_speed.to_string(),
                "90".to_owned()
            ),
            (
                "/3/ · Col. 1".to_owned(),
                old_hex.clone(),
                new_hex.to_owned()
            ),
        ],
        "the prompt's three rows in its order"
    );
    assert!(
        !nodes.iter().any(|node| ["Save", "Discard", "Cancel"]
            .iter()
            .any(|answer| node.label() == Some(answer) || node.value() == Some(answer))),
        "none of the prompt's answers"
    );
    let buttons: Vec<Option<&str>> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button)
        .map(|node| node.label())
        .collect();
    assert_eq!(
        buttons,
        [Some(X); 3],
        "the list's only buttons are its rows' X's (SE-GUI-74)"
    );
    let [red, green, blue] = old;
    assert_swatch_before(
        &window,
        &nodes,
        &shapes,
        &old_hex,
        egui::Color32::from_rgb(red, green, blue),
    );
    assert_swatch_before(
        &window,
        &nodes,
        &shapes,
        new_hex,
        egui::Color32::from_rgb(40, 81, 121),
    );
}

// The list's Item cells read the rows; they are no click targets — a
// selectable table would hover `Click to switch to this player` and answer with an
// index the popup ignores.
// SE-GUI-43
#[test]
fn the_lists_item_cells_take_no_click() {
    let (mut view, _) = three_changes("changes_list_no_click");
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    super::prompt::assert_item_cell_takes_no_click(
        &mut window,
        &mut view,
        &list_nodes(&pairs),
        "PLACEHOLDER · Height",
    );
}

// SE-GUI-43
#[test]
fn a_click_outside_esc_and_the_label_again_close_the_list() {
    let (mut view, _) = three_changes("changes_list_close");
    let revision = view.session.as_ref().unwrap().revision();
    let mut window = Headless::new();

    // A click on the pane's empty bottom-right corner, outside the list.
    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view).len(), 3, "premise: open");
    press_release(&mut window, egui::pos2(1200.0, 740.0), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(view.changes_list.is_none(), "a click outside closes it");
    assert!(
        list_rows(&mut window, &mut view).is_empty(),
        "and nothing draws"
    );

    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view).len(), 3, "premise: open");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.changes_list.is_none(), "Esc closes it");
    assert!(list_rows(&mut window, &mut view).is_empty());

    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view).len(), 3, "premise: open");
    click_label(&mut window, &mut view);
    assert!(view.changes_list.is_none(), "the label again closes it");
    assert!(list_rows(&mut window, &mut view).is_empty());

    assert_eq!(
        view.session.as_ref().unwrap().revision(),
        revision,
        "no path edited the save"
    );
}

// SE-GUI-43
#[test]
fn an_edit_or_the_edit_menus_undo_closes_the_list() {
    let (mut view, _) = three_changes("changes_list_edit");
    let mut window = Headless::new();

    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view).len(), 3, "premise: open");
    let mut edited = player_entry(&view, 70101);
    assert_ne!(edited.basic.age, 33, "premise: a real change");
    edited.basic.age = 33;
    view.session.as_mut().unwrap().edit_player(4, edited);
    assert!(
        list_rows(&mut window, &mut view).is_empty(),
        "another edit closes it"
    );
    assert!(view.changes_list.is_none());

    // The age edit undone, the list back at its three rows.
    assert!(view.session.as_mut().unwrap().undo(), "the age is undone");
    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view).len(), 3, "premise: open");
    // The click opening the Edit menu is outside the list too, so the list
    // is closed before Undo is chosen: the rows after it are what only
    // the Undo can change.
    choose(&mut window, &mut view, "Edit", "Undo");
    assert!(
        view.changes_list.is_none(),
        "the Edit menu's Undo closes it"
    );
    assert!(list_rows(&mut window, &mut view).is_empty());

    click_label(&mut window, &mut view);
    let rows = list_rows(&mut window, &mut view);
    assert_eq!(
        rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(),
        ["PLACEHOLDER · Height", "PLACEHOLDER · Speed"],
        "the colour undone, the list two rows long"
    );
}

// SE-GUI-43
#[test]
fn the_labels_hover_shows_the_pointing_hand_and_the_tooltip() {
    let (mut view, _) = three_changes("changes_list_hover");
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(LABEL))
            .expect("the label is drawn"),
    );
    let cursor = |window: &mut Headless, view: &mut SaveView| {
        window
            .frame(vec![egui::Event::PointerMoved(at)], |ui| {
                show(ui, view, &|| Vec::new());
                ui.ctx().output(|output| output.cursor_icon)
            })
            .0
    };
    cursor(&mut window, &mut view);
    assert_eq!(
        cursor(&mut window, &mut view),
        egui::CursorIcon::PointingHand
    );
    assert_eq!(
        tooltip_containing(&mut window, &mut view, at, "Lists the changes").as_deref(),
        Some("Lists the changes since the save was opened or last saved")
    );
}

// A team-wide edit lists more rows than fit: the table stops at 240
// points, so the popup's box is no taller than that and its frame, and
// the rows past it scroll.
// SE-GUI-43
#[test]
fn a_team_wide_edits_list_stops_at_240_points_and_scrolls() {
    let path = fixture(PesVersion::Pes21, "changes_list_cap");
    let mut view = SaveView::new();
    view.open(&path);
    let edits: Vec<PlayerEntry> = team_entry(&view, 701)
        .roster
        .iter()
        .filter(|slot| slot.player_id != 0)
        .map(|slot| {
            let mut entry = player_entry(&view, slot.player_id);
            entry.basic.height = if entry.basic.height == 170 { 171 } else { 170 };
            entry
        })
        .collect();
    assert_eq!(edits.len(), 23, "premise: 701's 23 players");
    view.session.as_mut().unwrap().edit_players(edits);
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (_, popup) = label_and_popup(&mut window, &mut view);
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke
        .width;
    let most = 240.0 + 2.0 * (8.0 + stroke);
    assert!(
        popup.height() <= most + 0.5,
        "the box is at most {most} tall: {popup:?}"
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bottom = list_nodes(&pairs)
        .iter()
        .filter_map(|node| node.bounds())
        .map(|bounds| bounds.y1)
        .fold(f64::MIN, f64::max);
    assert!(
        bottom > f64::from(popup.max.y),
        "the rows run on past the box, so the table scrolls: {bottom} vs {popup:?}"
    );
}

/// A PES 21 save open with player 70101 renamed to 52 letters, a change
/// whose row is wider than the popup's 640-point cap.
fn long_name_change(test: &str) -> SaveView {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    assert_ne!(edited.name, "W".repeat(52), "premise: a real change");
    edited.name = "W".repeat(52);
    view.session.as_mut().unwrap().edit_player(1, edited);
    view
}

/// The popup's outer box is the draft's `.chpop` width, 460 to 640
/// points with its margins and stroke counted in: three short rows hold
/// it at the 460 floor.
#[test]
fn the_list_is_460_points_wide_with_its_margins() {
    let (mut view, _) = three_changes("changes_list_width");
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (_, popup) = label_and_popup(&mut window, &mut view);
    assert!(
        (popup.width() - 460.0).abs() <= 0.5,
        "the outer box is 460 wide: {popup:?}"
    );
}

// The 640's other end: a row wider than the cap scrolls inside the box
// instead of growing it — the outer box stops at 640 (the draft's
// `.chpop` `max-width`, margins and stroke counted in).
// SE-GUI-43
#[test]
fn rows_wider_than_the_cap_hold_the_popup_at_640() {
    let mut view = long_name_change("changes_list_cap_width");
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The table is wider than the box: it scrolls sideways.
    assert!(
        list_nodes(&pairs).iter().any(|node| {
            node.role() == Role::ScrollBar
                && node.bounds().is_some_and(|b| b.x1 - b.x0 > b.y1 - b.y0)
        }),
        "premise: the rows overflow the box's width"
    );
    let (_, popup) = label_and_popup(&mut window, &mut view);
    assert!(
        (popup.width() - 640.0).abs() <= 0.5,
        "the outer box caps at 640: {popup:?}"
    );
}

// SE-GUI-43: the list opens 3 points under the label at its left edge
// where the window has room; at the default pane, where the label sits
// too far right for a list at its 640-point cap, it slides left to 8
// points inside the window's edge (the draft's `changesPopup`).
#[test]
fn the_list_opens_under_the_label_at_its_left_edge() {
    let (mut view, _) = three_changes("changes_list_place");
    let mut window = Headless::sized(1600.0, DEFAULT_PANE.y);
    click_label(&mut window, &mut view);
    let (label, popup) = label_and_popup(&mut window, &mut view);
    assert!(
        label.x0 + 460.0 < 1600.0,
        "premise: the list fits right of the label"
    );
    assert!(
        (f64::from(popup.left()) - label.x0).abs() <= 0.5,
        "the left edges meet: popup {popup:?}, label {label:?}"
    );
    assert!(
        (f64::from(popup.top()) - (label.y1 + 3.0)).abs() <= 0.5,
        "3 points under the label: popup {popup:?}, label {label:?}"
    );

    let mut view = long_name_change("changes_list_slide");
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (label, popup) = label_and_popup(&mut window, &mut view);
    assert!(
        label.x0 + 640.0 > f64::from(DEFAULT_PANE.x - 8.0),
        "premise: the list would pass 8 points inside the window's edge: label {label:?}"
    );
    assert!(
        (popup.right() - (DEFAULT_PANE.x - 8.0)).abs() <= 0.5,
        "slid left to 8 points inside the window's edge: popup {popup:?}, label {label:?}"
    );
    assert!(
        (f64::from(popup.top()) - (label.y1 + 3.0)).abs() <= 0.5,
        "still 3 points under the label: popup {popup:?}, label {label:?}"
    );
}

/// The PES 21 fixture with team 701's first eight players named `P01` to
/// `P08` and 171 to 178 cm tall, open on team 701's list: the fixture's
/// placeholders are identical records, which a drag would trade without
/// a trace.
fn told_apart(test: &str) -> SaveView {
    let path = fixture(PesVersion::Pes21, test);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for n in 1..=8u8 {
        let player = file.player_mut(70100 + u32::from(n)).unwrap();
        player.name = format!("P{n:02}");
        player.basic.height = 170 + n;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view
}

/// Drags the list's row labeled `from` onto the row labeled `onto`.
fn drag_row(window: &mut Headless, view: &mut SaveView, from: &str, onto: &str) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (from, onto) = (centre(row(&nodes, from)), centre(row(&nodes, onto)));
    drag(window, from, onto, view);
}

/// The rows of the toolbar's list and of the close prompt, asserted the
/// same: the list opened by a click on its label and closed by another,
/// then the prompt opened as the window's close opens it, and cancelled.
fn both_lists(window: &mut Headless, view: &mut SaveView) -> Vec<(String, String, String)> {
    click_label(window, view);
    let listed = list_rows(window, view);
    click_label(window, view);
    assert!(view.changes_list.is_none(), "the list closed again");
    crate::view::prompt::open(view, crate::view::prompt::Pending::Close);
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let prompted = table_rows(&super::prompt::dialog_nodes(&pairs, "Unsaved changes"));
    // Esc cancels the prompt; a frame without it lets its modal layer go,
    // which holds the shortcuts dead while it stands.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
    assert!(view.prompt.is_none(), "the prompt cancelled");
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    assert_eq!(
        listed, prompted,
        "the list and the prompt hold the same rows"
    );
    listed
}

/// A `Player order` row of team 701 (`/3/`), its two sides' lines.
fn order_row(old: &[&str], new: &[&str]) -> (String, String, String) {
    (
        "/3/ · Player order".to_owned(),
        old.join("\n"),
        new.join("\n"),
    )
}

/// The session lists no group of moved places.
fn no_moves(view: &SaveView) -> bool {
    let session = view.session.as_ref().expect("a save is open");
    session
        .unsaved_changes()
        .expect("the changes compare")
        .moves
        .is_empty()
}

/// The row of the drag from place 04 onto 07: P04 lands on 07, P05 to
/// P07 shift one place up.
fn drag_04_to_07() -> (String, String, String) {
    order_row(
        &["04 P04", "05 P05", "06 P06", "07 P07"],
        &["04 P05", "05 P06", "06 P07", "07 P04"],
    )
}

// SE-GUI-73
#[test]
fn a_dragged_player_is_one_player_order_row() {
    let mut view = told_apart("changes_list_move");
    let mut window = Headless::new();

    // The drag from 04 onto 07: one row, and no player has one.
    drag_row(&mut window, &mut view, "04 CB P04", "07 CB P07");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        &row_labels(&nodes)[3..7],
        ["04 CB P05 ●", "05 CB P06 ●", "06 CB P07 ●", "07 CB P04 ●"],
        "premise: the drag moved P04 onto 07"
    );
    assert_eq!(both_lists(&mut window, &mut view), [drag_04_to_07()]);

    // The moved player's Height edited: that row and his one Height row,
    // its old value his own.
    let mut edited = player_entry(&view, 70107);
    assert_eq!(edited.basic.height, 174, "premise: P04's height moved");
    let file_height = view
        .session
        .as_ref()
        .unwrap()
        .baseline_player(70107)
        .unwrap()
        .basic
        .height;
    assert_ne!(file_height, 174, "premise: the file's 07 is another height");
    edited.basic.height = 190;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let height_row = (
        "P04 · Height".to_owned(),
        "174".to_owned(),
        "190".to_owned(),
    );
    assert_eq!(
        both_lists(&mut window, &mut view),
        [height_row.clone(), drag_04_to_07()]
    );

    // The Height undone, then the move: the list holds the order row, then
    // nothing, and the label is gone with the last change.
    choose(&mut window, &mut view, "Edit", "Undo");
    assert_eq!(both_lists(&mut window, &mut view), [drag_04_to_07()]);
    choose(&mut window, &mut view, "Edit", "Undo");
    assert!(!view.is_dirty(), "the move undone, nothing is unsaved");
    // A clean save shows no list to read: the session's own answer, which
    // the list's rows are made from, holds no moved group either.
    assert!(no_moves(&view), "the move undone: no moved group is left");

    // P02's Height edited, then P02 dragged onto 03, then the drag undone:
    // his Height row stays his, under his name, against the file's Height
    // on his own id.
    let mut edited = player_entry(&view, 70102);
    assert_ne!(edited.basic.height, 190, "premise: a real change");
    assert_ne!(
        player_entry(&view, 70103).basic.height,
        190,
        "premise: the record the drag puts on 02 is another height"
    );
    edited.basic.height = 190;
    view.session.as_mut().unwrap().edit_player(2, edited);
    drag_row(&mut window, &mut view, "02 CB P02 ●", "03 CB P03");
    let height_row = (
        "P02 · Height".to_owned(),
        "172".to_owned(),
        "190".to_owned(),
    );
    assert_eq!(
        both_lists(&mut window, &mut view),
        [
            height_row.clone(),
            order_row(&["02 P02", "03 P03"], &["02 P03", "03 P02"])
        ],
        "his one Height row beside the order row"
    );
    choose(&mut window, &mut view, "Edit", "Undo");
    assert_eq!(
        both_lists(&mut window, &mut view),
        [height_row],
        "the drag undone: the Height row alone"
    );
    // The Height undone too, so the clauses below start from the file.
    choose(&mut window, &mut view, "Edit", "Undo");
    assert!(!view.is_dirty(), "both undone, nothing is unsaved");
    assert!(no_moves(&view), "both undone: no moved group is left");

    // The drag from 04 onto 07 again (the Height edit cleared the redo
    // stack) and a second, separate pair swapped: two rows.
    drag_row(&mut window, &mut view, "04 CB P04", "07 CB P07");
    drag_row(&mut window, &mut view, "02 CB P02", "01 CB P01");
    assert_eq!(
        both_lists(&mut window, &mut view),
        [
            order_row(&["01 P01", "02 P02"], &["01 P02", "02 P01"]),
            drag_04_to_07()
        ]
    );

    // Saved: nothing is unsaved; P04 dragged back onto 04 is a row against
    // the written order.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(!view.is_dirty(), "the save is written");
    drag_row(&mut window, &mut view, "07 CB P04", "04 CB P05");
    assert_eq!(
        both_lists(&mut window, &mut view),
        [order_row(
            &["04 P05", "05 P06", "06 P07", "07 P04"],
            &["04 P04", "05 P05", "06 P06", "07 P07"],
        )]
    );
}

// The order row's new side names the players now on the places, as they
// are now: a moved player renamed after the move shows his new name there,
// while the old side keeps the file's. (A new side read from the record he
// came from would still say `P04`.)
// SE-GUI-73
#[test]
fn a_player_order_rows_new_side_names_the_players_as_they_are_now() {
    let mut view = told_apart("changes_list_move_rename");
    let mut window = Headless::new();
    drag_row(&mut window, &mut view, "04 CB P04", "07 CB P07");
    let mut renamed = player_entry(&view, 70107);
    assert_eq!(renamed.name, "P04", "premise: P04 stands on 07");
    renamed.name = "RENAMED".to_owned();
    view.session.as_mut().unwrap().edit_player(1, renamed);
    assert_eq!(
        both_lists(&mut window, &mut view),
        [
            (
                "RENAMED · Name".to_owned(),
                "P04".to_owned(),
                "RENAMED".to_owned()
            ),
            order_row(
                &["04 P04", "05 P05", "06 P06", "07 P07"],
                &["04 P05", "05 P06", "06 P07", "07 RENAMED"]
            ),
        ]
    );
}

// The order row is as high as its four places, its Item on the first
// line; the Height row beside it keeps one line.
// SE-GUI-73
#[test]
fn a_player_order_row_is_as_high_as_its_places() {
    let mut view = told_apart("changes_list_move_height");
    view.session.as_mut().unwrap().move_player(701, 3, 6);
    let mut edited = player_entry(&view, 70107);
    edited.basic.height = 190;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let nodes = list_nodes(&pairs);
    let bounds = |value: &str| {
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(value))
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no cell {value:?}"))
    };
    let (_, old, new) = drag_04_to_07();
    // A value's line; the Item's proportional font runs a point taller.
    let one_line = bounds("174");
    let line = one_line.y1 - one_line.y0;
    for cell in [bounds("P04 · Height"), bounds("190")] {
        assert!(
            cell.y1 - cell.y0 < 1.5 * line,
            "the Height row's cells are one line: {cell:?}, {one_line:?}"
        );
    }
    let item = bounds("/3/ · Player order");
    assert!(
        item.y1 - item.y0 < 1.5 * line,
        "the order row's Item is one line: {item:?}"
    );
    for value in [old.as_str(), new.as_str()] {
        let cell = bounds(value);
        assert!(
            (cell.y1 - cell.y0 - 4.0 * line).abs() < 1.0,
            "four lines high: {cell:?}, a line {line}"
        );
        assert!(
            (cell.y0 - item.y0).abs() < 1.0,
            "the Item on the values' first line: {cell:?}, {item:?}"
        );
    }
}

/// `told_apart` with P04 dragged from 04 onto 07, his Height then edited
/// to 190, and team 701's preset 1 Support range edited to 9: the list's
/// Height, `Player order` and Support range rows, which it returns with
/// the file's Support range.
fn three_kinds(test: &str) -> (SaveView, [(String, String, String); 3], u8) {
    let mut view = told_apart(test);
    let session = view.session.as_mut().unwrap();
    session.move_player(701, 3, 6);
    let mut edited = session.file().player(70107).unwrap().clone();
    assert_eq!(
        (edited.name.as_str(), edited.basic.height),
        ("P04", 174),
        "premise: P04 on 07"
    );
    edited.basic.height = 190;
    session.edit_player(1, edited);
    let mut team = session.file().team(701).unwrap().clone();
    let slider = team.tactics.presets[0].sliders.support_range;
    assert_ne!(slider, 9, "premise: a real change");
    team.tactics.presets[0].sliders.support_range = 9;
    session.edit_team(2, team);
    let rows = [
        (
            "P04 · Height".to_owned(),
            "174".to_owned(),
            "190".to_owned(),
        ),
        drag_04_to_07(),
        (
            "/3/ · Preset 1 · Support range".to_owned(),
            slider.to_string(),
            "9".to_owned(),
        ),
    ];
    (view, rows, slider)
}

/// The open list's X on the row whose Item cell reads `item`, on its line.
fn x_of<'a>(nodes: &[&'a accesskit::Node], item: &str) -> &'a accesskit::Node {
    let row = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(item))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no row {item:?}"));
    nodes
        .iter()
        .copied()
        .find(|node| {
            node.role() == Role::Button
                && node.label() == Some(X)
                && node
                    .bounds()
                    .is_some_and(|bounds| (bounds.y0 - row.y0).abs() < 2.0)
        })
        .unwrap_or_else(|| panic!("no X on the line of {item:?}"))
}

/// A click on the X of the open list's row `item`.
fn click_x(window: &mut Headless, view: &mut SaveView, item: &str) {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let x = x_of(&list_nodes(&pairs), item).clone();
    click(window, &x, view);
}

/// The Edit menu's Undo (which closes the list, a click outside it), then
/// the list opened again: its rows.
fn undo_and_reopen(window: &mut Headless, view: &mut SaveView) -> Vec<(String, String, String)> {
    choose(window, view, "Edit", "Undo");
    assert!(view.changes_list.is_none(), "the Undo closed the list");
    click_label(window, view);
    list_rows(window, view)
}

// SE-GUI-74
#[test]
fn a_rows_x_puts_that_change_back_alone() {
    let (mut view, three, slider) = three_kinds("changes_list_x");
    let [height_row, order, support_row] = three.clone();
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    assert_eq!(list_rows(&mut window, &mut view), three, "premise");

    // The Height's X: the file's Height, the list open without the row,
    // and one Undo sets the edited Height back.
    click_x(&mut window, &mut view, "P04 · Height");
    assert_eq!(
        player_entry(&view, 70107).basic.height,
        174,
        "P04's file Height"
    );
    assert_eq!(
        list_rows(&mut window, &mut view),
        [order.clone(), support_row.clone()],
        "the list stays open without the row"
    );
    assert_eq!(undo_and_reopen(&mut window, &mut view), three);
    assert_eq!(player_entry(&view, 70107).basic.height, 190, "one Undo");

    // The same for the Support range.
    let support = |view: &SaveView| {
        team_entry(view, 701).tactics.presets[0]
            .sliders
            .support_range
    };
    click_x(&mut window, &mut view, "/3/ · Preset 1 · Support range");
    assert_eq!(support(&view), slider, "the file's Support range");
    assert_eq!(
        list_rows(&mut window, &mut view),
        [height_row.clone(), order.clone()],
        "the list stays open without the row"
    );
    assert_eq!(undo_and_reopen(&mut window, &mut view), three);
    assert_eq!(support(&view), 9, "one Undo");

    // The `Player order` X: every player of the group back on his place
    // in the file, P04's Height edit, made after the move, still his.
    click_x(&mut window, &mut view, "/3/ · Player order");
    let names: Vec<String> = (70104..=70107)
        .map(|id| player_entry(&view, id).name)
        .collect();
    assert_eq!(names, ["P04", "P05", "P06", "P07"], "back on their places");
    assert_eq!(player_entry(&view, 70104).basic.height, 190, "his edit");
    assert_eq!(
        list_rows(&mut window, &mut view),
        [height_row.clone(), support_row.clone()],
        "his Height row, now on 04, and the Support range"
    );

    // The close prompt's table has no X.
    crate::view::prompt::open(&mut view, crate::view::prompt::Pending::Close);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dialog = super::prompt::dialog_nodes(&pairs, "Unsaved changes");
    assert_eq!(
        table_rows(&dialog),
        [height_row, support_row],
        "premise: the prompt's rows"
    );
    assert!(
        !dialog
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some(X)),
        "no X in the close prompt"
    );
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(view.prompt.is_none(), "the prompt cancelled");
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The last row's X: the list closes and the save is clean.
    if view.changes_list.is_none() {
        click_label(&mut window, &mut view);
    }
    click_x(&mut window, &mut view, "/3/ · Preset 1 · Support range");
    assert_eq!(
        list_rows(&mut window, &mut view).len(),
        1,
        "premise: one row left"
    );
    click_x(&mut window, &mut view, "P04 · Height");
    assert!(view.changes_list.is_none(), "the list closes");
    assert!(!view.is_dirty(), "the save is clean");
}

// SE-GUI-74
#[test]
fn every_x_stays_at_the_lists_right_edge_while_a_wide_row_scrolls() {
    let mut view = long_name_change("changes_list_x_wide");
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(2, edited);
    let mut window = Headless::new();
    click_label(&mut window, &mut view);
    let long = "W".repeat(52);
    let items = [format!("{long} · Name"), format!("{long} · Height")];
    // The X's, and the long value's cell, as the frame draws them.
    let read = |window: &mut Headless, view: &mut SaveView| {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let nodes = list_nodes(&pairs);
        let xs: Vec<accesskit::Rect> = items
            .iter()
            .map(|item| x_of(&nodes, item).bounds().expect("an X's bounds"))
            .collect();
        let cell = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(long.as_str()))
            .and_then(|node| node.bounds())
            .expect("the long name's cell");
        (xs, cell)
    };
    let (before, cell) = read(&mut window, &mut view);
    let (_, popup) = label_and_popup(&mut window, &mut view);
    assert!(
        cell.x1 > before[0].x0,
        "premise: the row runs under the X's column: {cell:?}, {:?}",
        before[0]
    );

    // Wheeled sideways over the table until it moves no further.
    // Over the row's start, inside the list: the long value itself begins
    // past the list's right edge.
    let at = egui::pos2(popup.left() + 60.0, (cell.y0 + cell.y1) as f32 / 2.0);
    let mut last = cell;
    for notch in 0.. {
        assert!(notch < 20, "the table never reached its end");
        window.frame(
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(-400.0, 0.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        idle_frames(&mut window, &mut view, 31);
        let (_, cell) = read(&mut window, &mut view);
        if (cell.x0 - last.x0).abs() < 0.5 {
            break;
        }
        last = cell;
    }
    let (after, cell) = read(&mut window, &mut view);
    assert!(
        cell.x1 <= after[0].x0,
        "premise: scrolled to its end, the long value ends left of the X's: {cell:?}"
    );
    for (x_before, x_after) in before.iter().zip(&after) {
        assert!(
            (x_before.x0 - x_after.x0).abs() < 0.5 && (x_before.y0 - x_after.y0).abs() < 0.5,
            "the X stands where it stood: {x_before:?}, {x_after:?}"
        );
        assert!(
            f64::from(popup.right()) - x_after.x1 <= 10.0,
            "at the list's right edge: {x_after:?}, the list {popup:?}"
        );
    }
}
