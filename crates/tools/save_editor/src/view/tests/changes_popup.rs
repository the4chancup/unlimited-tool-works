//! The toolbar's list of unsaved changes (SE-GUI-43): a click on the
//! `• unsaved changes` label opens the close prompt's table in a popup
//! under it, with none of the prompt's answers, and each closing path
//! closes it.

use super::prompt::table_rows;
use super::*;
use crate::view::team::{channel_to_show, format_hex};

/// The label's text.
const LABEL: &str = "\u{2022} unsaved changes";

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

/// Asserts that the value cell reading `hex` among `nodes` paints its
/// swatch: a 12 × 12 rect filled `colour`, its corners rounded 2, its
/// 1-point stroke the theme's swatch edge, 5 points left of the hex and
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
    assert_eq!(swatch.corner_radius, egui::CornerRadius::same(2));
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
        !nodes.iter().any(|node| node.role() == Role::Button
            || ["Save", "Discard", "Cancel"]
                .iter()
                .any(|answer| node.label() == Some(answer) || node.value() == Some(answer))),
        "none of the prompt's answers"
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
fn an_edit_or_the_toolbars_undo_closes_the_list() {
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
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let undo = node_labeled(&nodes, Role::Button, "Undo");
    click(&mut window, undo, &mut view);
    assert!(view.changes_list.is_none(), "the toolbar's Undo closes it");
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
