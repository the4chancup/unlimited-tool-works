use pes_savefile::schema::fields::TeamText;
use pes_savefile::schema::schema_for;

use super::*;

/// The registered position's full name, GK 0 to CF 12 (`session::team_ops`'
/// table, pinned here so a wrong spelling fails against a literal).
const POSITION_NAMES: [&str; 13] = [
    "Goalkeeper",
    "Center Back",
    "Left Back",
    "Right Back",
    "Defensive Midfielder",
    "Center Midfielder",
    "Left Midfielder",
    "Right Midfielder",
    "Attacking Midfielder",
    "Left Wing Forward",
    "Right Wing Forward",
    "Second Striker",
    "Centre Forward",
];

/// Replaces the focused text control's contents with `text`: End moves the
/// caret past the text, the backspaces clear it, then the characters land
/// (`color_tools`' `type_text`).
fn type_text(window: &mut Headless, view: &mut SaveView, text: &str) {
    let mut events = vec![key(egui::Key::End, egui::Modifiers::NONE, true)];
    for _ in 0..16 {
        events.push(key(egui::Key::Backspace, egui::Modifiers::NONE, true));
    }
    events.push(egui::Event::Text(text.to_owned()));
    window.frame(events, |ui| show(ui, view, &|| Vec::new()));
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
}

/// The rostered `(slot, player_id)` pairs of `team_id`, in slot order.
fn rostered(view: &SaveView, team_id: u32) -> Vec<(usize, u32)> {
    team_entry(view, team_id)
        .roster
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.player_id != 0)
        .map(|(slot, roster_slot)| (slot, roster_slot.player_id))
        .collect()
}

/// The team view's name field: the TextInput right of the topmost "Name"
/// label.
fn bar_name_field(nodes: &[accesskit::Node]) -> &accesskit::Node {
    let label = nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some("Name"))
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map(|b| b.y0).unwrap_or(f64::MAX);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("the view's Name label");
    right_of(nodes, label, Role::TextInput)
}

/// The `TextShape` painted inside the Label node `value`'s bounds (the
/// team view's title).
fn label_shape<'a>(
    nodes: &[accesskit::Node],
    shapes: &'a [egui::epaint::ClippedShape],
    value: &str,
) -> &'a egui::epaint::TextShape {
    let bounds = label_bounds(nodes, value);
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if shape.galley.job.text == value
                    && f64::from(shape.pos.x) >= bounds.x0 - 1.0
                    && f64::from(shape.pos.x) <= bounds.x1 + 1.0
                    && f64::from(shape.pos.y) >= bounds.y0 - 1.0
                    && f64::from(shape.pos.y) <= bounds.y1 + 1.0 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted {value:?} inside {bounds:?}"))
}

/// The Team menu button (the toolbar's, not the view's "Team" label):
/// the topmost button with the label.
fn team_menu_button(nodes: &[accesskit::Node]) -> &accesskit::Node {
    let label = menu_label("Team");
    nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.label() == Some(label.as_str()))
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(f64::MAX);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("the Team menu button")
}

/// The Team menu's popup open for one frame: a click on the Team button
/// (skipped when a previous call left it open), then the frame's nodes.
fn open_team_menu(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    if nodes
        .iter()
        .any(|node| node.role() == Role::Button && node.label() == Some("Set edited flag"))
    {
        return nodes;
    }
    click(window, team_menu_button(&nodes), view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    nodes
}

/// The Kit slots dialog open over `team_id`'s view: the view open, the
/// `Kit slots…` button clicked, then two frames (the modal re-centres
/// once its content's size is known, so the first frame's bounds are
/// not the steady ones). The last frame's nodes and shapes.
fn open_kit_slots(
    window: &mut Headless,
    view: &mut SaveView,
    team_id: u32,
) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
    open_team_view(window, view, team_id);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    click(
        window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        view,
    );
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    (nodes, shapes)
}

// SE-GUI-10
#[test]
fn the_team_fields_edit_and_save_and_the_colour_is_the_nearest_the_save_holds() {
    let path = fixture(PesVersion::Pes21, "team_view_edit");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    assert_eq!(
        team_entry(&view, 701).name,
        "/3/",
        "premise: team 701's name"
    );
    assert_eq!(
        team_entry(&view, 701).manager_id,
        Some(701),
        "premise: the manager ID"
    );

    // The Name field: the team bar's text input holding the team name
    // (the header's own Name field holds the player's).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("/3/"))
        .expect("the team's Name field");
    click(&mut window, name_field, &mut view);
    type_text(&mut window, &mut view, "MY TEAM");
    assert_eq!(team_entry(&view, 701).name, "MY TEAM");
    assert!(
        team_entry(&view, 701).edit_flags.name,
        "a name edit sets the name flag"
    );

    // The Manager ID control: a typed value lands.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let manager = control_right_of(&nodes, "Manager ID");
    click(&mut window, manager, &mut view);
    window.frame(vec![egui::Event::Text("777".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(team_entry(&view, 701).manager_id, Some(777));

    // Colour 1: the picker's hex field takes #FF8000; the save keeps 6
    // bits a channel, so the stored colour is the nearest the save holds.
    // Both colour fields share the grid row, so Colour 1 is the leftmost.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::ColorWell
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("color field"))
        })
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .expect("the Colour 1 field");
    click(&mut window, field, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let hex = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && node.value().is_some_and(|value| value.starts_with('#'))
        })
        .expect("the picker's hex field");
    click(&mut window, hex, &mut view);
    type_text(&mut window, &mut view, "#FF8000");
    let color = team_entry(&view, 701)
        .colors
        .expect("PES 21 stores colours")[0];
    assert_eq!(
        (color.red, color.green, color.blue),
        (63, 32, 0),
        "#FF8000 stores as the nearest 6-bit colour"
    );
    // The hex beside the field reads the shown colour back: #FF8200.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::Label && node.value() == Some("#FF8200")),
        "the team bar shows #FF8200"
    );

    // Save; the reopened file holds the three values.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    let team = reopened.team(701).expect("team 701");
    assert_eq!(team.name, "MY TEAM");
    assert_eq!(team.manager_id, Some(777));
    let color = team.colors.expect("colours")[0];
    assert_eq!((color.red, color.green, color.blue), (63, 32, 0));
}

// The kit slots' grid spaces its rows by the gap between their boxes, as
// the draft's does: each slot's two number boxes stand 6 under the slot
// above's, 24 apart top to top.
#[test]
fn the_kit_slot_rows_stand_24_apart() {
    let path = fixture(PesVersion::Pes17, "team_kit_slot_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (nodes, _) = open_kit_slots(&mut window, &mut view, 701);
    let tops = |slot: &str| -> Vec<f64> {
        let boxes = right_of_label(&nodes, slot, Role::SpinButton);
        assert_eq!(boxes.len(), 2, "{slot}: Number and Bound team ID");
        boxes
            .iter()
            .map(|node| node.bounds().expect("a box's bounds").y0)
            .collect()
    };
    for (upper, lower) in [("Slot 1", "Slot 2"), ("Slot 2", "Slot 3")] {
        for (above, below) in tops(upper).into_iter().zip(tops(lower)) {
            assert!(
                (below - above - 24.0).abs() <= 0.5,
                "{lower}'s boxes stand {} under {upper}'s, not 24",
                below - above
            );
        }
    }
}

// Every box of the Kit slots dialog is one width, Number and Bound team ID
// alike, wide enough for six digits: a six-digit bound id shows whole in
// a box as wide as a three-digit one's, so the rows line up.
#[test]
fn every_kit_slots_box_is_one_width_that_shows_six_digits() {
    let path = fixture(PesVersion::Pes17, "kit_slot_box_width");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).unwrap();
    let slots = file
        .team_mut(701)
        .unwrap()
        .kit_slots
        .as_mut()
        .expect("PES 17 kit slots");
    // PES 17's largest bound id (its 24-bit binding over 64), and a
    // three-digit one under it.
    slots[0].binding = 262143 * 64;
    slots[1].binding = 701 * 64;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (nodes, shapes) = open_kit_slots(&mut window, &mut view, 701);
    // A slot's boxes share its label's centre line, as a grid row centres
    // its cells: the team view's own boxes behind the dialog do not.
    let centre_y = |bounds: accesskit::Rect| (bounds.y0 + bounds.y1) / 2.0;
    let mut boxes: Vec<accesskit::Rect> = (1..=10)
        .flat_map(|slot| {
            let row = centre_y(label_bounds(&nodes, &format!("Slot {slot}")));
            right_of_label(&nodes, &format!("Slot {slot}"), Role::SpinButton)
                .into_iter()
                .map(|node| node.bounds().expect("a box's bounds"))
                .filter(move |bounds| (centre_y(*bounds) - row).abs() <= 1.0)
        })
        .collect();
    // A control on the view behind can share a slot row's centre: the
    // dialog's two box columns share one left edge each, so only the
    // groups standing on a shared left edge are the dialog's.
    let mut columns: Vec<Vec<accesskit::Rect>> = Vec::new();
    for bounds in boxes.drain(..) {
        match columns
            .iter_mut()
            .find(|group: &&mut Vec<accesskit::Rect>| (group[0].x0 - bounds.x0).abs() <= 0.5)
        {
            Some(group) => group.push(bounds),
            None => columns.push(vec![bounds]),
        }
    }
    let boxes: Vec<accesskit::Rect> = columns
        .into_iter()
        .filter(|group| group.len() >= 2)
        .flatten()
        .collect();
    assert_eq!(boxes.len(), 20, "a Number and a Bound team ID a slot");
    let widths: Vec<f64> = boxes.iter().map(accesskit::Rect::width).collect();
    assert!(
        widths.iter().all(|width| (width - widths[0]).abs() <= 0.01),
        "every box one width: {widths:?}"
    );
    // Any six digits fit, not only these: each digit's run of six in the
    // boxes' font, and a box's padding either side.
    let style = window.ctx.global_style();
    let font = style.drag_value_text_style.resolve(&style);
    let widest = window.ctx.fonts_mut(|fonts| {
        (0..=9)
            .map(|digit| {
                fonts
                    .layout_no_wrap(
                        digit.to_string().repeat(6),
                        font.clone(),
                        egui::Color32::WHITE,
                    )
                    .size()
                    .x
            })
            .fold(0.0_f32, f32::max)
    });
    let needed = f64::from(widest + 2.0 * style.spacing.button_padding.x);
    assert!(
        widths[0] >= needed - 0.01,
        "a box {} wide, six digits need {needed}",
        widths[0]
    );
    let six = right_of_label(&nodes, "Slot 1", Role::SpinButton)[1];
    assert_eq!(six.value(), Some("262143"), "premise: slot 1's six digits");
    let bounds = six.bounds().expect("bounds");
    let ink = ink(text_shape(&shapes, "262143"));
    assert!(
        f64::from(ink.min.x) >= bounds.x0 && f64::from(ink.max.x) <= bounds.x1,
        "the six digits inside their box ({ink:?} in {bounds:?})"
    );
}

// The dialog's label columns are their widest label, not the boxes' width:
// the grid's 8-point gap parts "Slot 10" from "Number" and the Number box
// from "Bound team ID", as the draft's cell padding does.
#[test]
fn the_kit_slot_label_columns_are_their_labels_wide() {
    let path = fixture(PesVersion::Pes17, "kit_slot_label_columns");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (nodes, _) = open_kit_slots(&mut window, &mut view, 701);

    let slot = label_bounds(&nodes, "Slot 10");
    let number = label_bounds(&nodes, "Number");
    assert!(
        (number.x0 - slot.x1 - 8.0).abs() <= 1.0,
        "\"Number\" stands {} after \"Slot 10\", the grid's 8-point gap",
        number.x0 - slot.x1
    );
    let box_ = right_of_label(&nodes, "Number", Role::SpinButton)
        .first()
        .and_then(|node| node.bounds())
        .expect("slot 10's Number box");
    let bound = label_bounds(&nodes, "Bound team ID");
    assert!(
        (bound.x0 - box_.x1 - 8.0).abs() <= 1.0,
        "\"Bound team ID\" stands {} after the Number box, the grid's 8-point gap",
        bound.x0 - box_.x1
    );
}

// The draft puts a field's label and box in one cell ("Number " then the
// input), so a box stands a space after its label's text, not the grid's
// gap after it.
#[test]
fn the_kit_slot_boxes_stand_a_space_after_their_labels() {
    let path = fixture(PesVersion::Pes17, "kit_slot_label_box_gap");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (nodes, _) = open_kit_slots(&mut window, &mut view, 701);

    // Slot 1's row by its label's centre line, so a control of the team
    // view behind the dialog is never measured.
    let centre_y = |bounds: accesskit::Rect| (bounds.y0 + bounds.y1) / 2.0;
    let row = centre_y(label_bounds(&nodes, "Slot 1"));
    let label = |value: &str| -> accesskit::Rect {
        nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value() == Some(value)
                    && node
                        .bounds()
                        .is_some_and(|b| (centre_y(b) - row).abs() <= 1.0)
            })
            .and_then(accesskit::Node::bounds)
            .unwrap_or_else(|| panic!("no {value:?} label on Slot 1's row"))
    };
    let boxes: Vec<accesskit::Rect> = right_of_label(&nodes, "Slot 1", Role::SpinButton)
        .into_iter()
        .map(|node| node.bounds().expect("a box's bounds"))
        .filter(|bounds| (centre_y(*bounds) - row).abs() <= 1.0)
        .collect();
    assert_eq!(boxes.len(), 2, "slot 1's Number and Bound team ID boxes");
    for (name, box_) in [("Number", boxes[0]), ("Bound team ID", boxes[1])] {
        let gap = box_.x0 - label(name).x1;
        assert!(
            (gap - 3.0).abs() <= 1.0,
            "{name}'s box stands {gap} after its label, not a space's 3"
        );
    }
}

#[test]
fn the_bound_team_id_shows_binding_over_64_and_an_edit_keeps_the_low_bits() {
    let path = fixture(PesVersion::Pes17, "team_kit_slots");
    // Give slot 1 a binding with low bits set: the edit must keep them.
    // The strip flag clear, so a kit-slot edit's setting it shows.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).unwrap();
    let slots = file
        .team_mut(701)
        .unwrap()
        .kit_slots
        .as_mut()
        .expect("PES 17 kit slots");
    slots[0].binding = 701 * 64 + 5;
    let binding = slots[0].binding;
    file.team_mut(701).unwrap().edit_flags.strip = Some(false);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    // The Kit slots… button opens the dialog over the view; a second frame
    // before reading nodes: the modal re-centres once its content's size
    // is known, so the first frame's bounds are not the steady ones.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "Kit slot assignments"),
        "the dialog's title"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // Slot 1's row: the weak "Slot 1" label's row holds Number and Bound
    // team ID, in x order.
    let slot_label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Slot 1"))
        .expect("the Slot 1 row");
    let row_y = slot_label.bounds().expect("bounds").y0;
    let mut controls: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::SpinButton
                && node.bounds().is_some_and(|b| (b.y0 - row_y).abs() < 4.0)
        })
        .collect();
    controls.sort_by(|a, b| {
        a.bounds()
            .expect("bounds")
            .x0
            .partial_cmp(&b.bounds().expect("bounds").x0)
            .expect("ordered bounds")
    });
    assert_eq!(controls.len(), 2, "Number and Bound team ID on the row");
    let bound = controls[1];
    assert_eq!(
        bound.value(),
        Some((binding / 64).to_string()).as_deref(),
        "Bound team ID shows binding / 64"
    );

    // A Number edit sets the team's strip flag, as the stadium edit sets
    // its own (PES 17 is the version with kit slots, so the flag is Some).
    click(&mut window, controls[0], &mut view);
    window.frame(vec![egui::Event::Text("9".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        team_entry(&view, 701).kit_slots.expect("kit slots")[0].number,
        9
    );
    assert_eq!(
        team_entry(&view, 701).edit_flags.strip,
        Some(true),
        "a kit-slot edit sets the strip flag"
    );
    // The one undo restores the edit whole, the flag back to clear. The
    // open dialog holds Ctrl+Z, so the undo is the session's own.
    assert!(view.session.as_mut().unwrap().undo(), "an entry to undo");
    assert_eq!(
        team_entry(&view, 701).edit_flags.strip,
        Some(false),
        "the undo restores the flag too"
    );

    // An edit writes `id * 64 + binding % 64`: the low 5 stays.
    click(&mut window, bound, &mut view);
    window.frame(vec![egui::Event::Text("702".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let slots = team_entry(&view, 701).kit_slots.expect("kit slots");
    assert_eq!(
        slots[0].binding,
        702 * 64 + binding % 64,
        "the low bits are kept as stored"
    );
    assert_eq!(
        team_entry(&view, 701).edit_flags.strip,
        Some(true),
        "the bound edit sets the strip flag too"
    );

    // The shown id's top is the binding field's own top / 64 (PES 17's
    // binding is 24 bits, so 262143): typing 262144 stops there.
    click(&mut window, bound, &mut view);
    window.frame(vec![egui::Event::Text("262144".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let slots = team_entry(&view, 701).kit_slots.expect("kit slots");
    assert_eq!(
        slots[0].binding,
        262143 * 64 + binding % 64,
        "past the top the id is 262143, the low bits still kept"
    );
}

// The short name's box takes three characters, so a fourth never lands:
// the game shows a three-letter abbreviation.
#[test]
fn the_short_name_box_takes_three_characters() {
    let path = fixture(PesVersion::Pes21, "short_box_three");
    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(
        team_entry(&view, 701).short_name,
        "3",
        "premise: team 701's short name"
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("3"))
        .expect("the Short field");
    click(&mut window, field, &mut view);
    // Clearing the field and typing four letters leaves three in the box:
    // the field's own limit, not the store's refusal.
    let (_, nodes, _) = window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::Backspace, egui::Modifiers::NONE, true),
            egui::Event::Text("abcd".to_owned()),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let field = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput && node.value().is_some_and(|v| v.contains("abc"))
        })
        .expect("the Short field after typing");
    assert_eq!(field.value(), Some("abc"), "the fourth letter never lands");
    assert_eq!(team_entry(&view, 701).short_name, "ABC");
}

// A stored short name longer than three characters (a hand-edited save)
// shows whole and is kept until edited.
#[test]
fn a_stored_short_name_past_three_shows_whole_until_edited() {
    let path = fixture(PesVersion::Pes21, "short_stored_long");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.team_mut(701).unwrap().short_name = "ABCD".to_owned();
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(
        team_entry(&view, 701).short_name,
        "ABCD",
        "premise: the stored short name"
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("ABCD"))
        .expect("the Short field shows the stored name whole");
    click(&mut window, field, &mut view);
    // A keystroke at the cap lands nothing: the stored name is kept.
    let (_, nodes, _) = window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            egui::Event::Text("x".to_owned()),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == Role::TextInput && node.value() == Some("ABCD")),
        "the field still shows the stored name"
    );
    assert_eq!(team_entry(&view, 701).short_name, "ABCD");
    // Deleting edits it as usual: the field takes the three letters.
    window.frame(
        vec![key(egui::Key::Backspace, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(team_entry(&view, 701).short_name, "ABC");
}

// The same refusal as the name field's, on the short name's own rule:
// `€` is a char the single-byte field never stores, so the keystroke
// changes nothing and no undo entry lands.
#[test]
fn a_char_the_short_name_field_cannot_store_is_not_taken() {
    let path = fixture(PesVersion::Pes21, "short_name_refused_char");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    assert_eq!(
        team_entry(&view, 701).short_name,
        "3",
        "premise: team 701's short name"
    );

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("3"))
        .expect("the Short name field");
    click(&mut window, field, &mut view);
    type_text(&mut window, &mut view, "€");

    let session = view.session.as_ref().unwrap();
    assert!(!session.can_undo(), "the refused keystroke pushed no edit");
    assert_eq!(
        team_entry(&view, 701).short_name,
        "3",
        "the stored short name stands"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = right_of_label(&nodes, "Short name", Role::TextInput)
        .first()
        .copied()
        .expect("the Short name field");
    assert_eq!(field.value(), Some("3"), "the field shows the stored name");
}

// `TEAM_ROW`'s claims: each unit's drawn span — measured on the one
// row the default pane gives — is covered by its claim, so no unit draws
// past the row's right edge (a stale claim under-measures and a widget
// that grew shows here).
#[test]
fn the_header_units_draw_inside_their_claims() {
    let path = fixture(PesVersion::Pes21, "team_header_claims");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let label_node = |value: &str| {
        nodes
            .iter()
            .filter(|node| node.role() == Role::Label && node.value() == Some(value))
            .min_by(|a, b| {
                let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
                y(a).partial_cmp(&y(b)).expect("ordered bounds")
            })
            .unwrap_or_else(|| panic!("no {value:?}"))
    };
    let right_x = |node: &accesskit::Node| node.bounds().expect("bounds").x1;
    let span = |value: &str, right: f64| right - label_node(value).bounds().expect("bounds").x0;
    let spans = [
        span("Name", right_x(bar_name_field(&nodes))),
        span(
            "Short name",
            right_x(
                right_of_label(&nodes, "Short name", Role::TextInput)
                    .first()
                    .copied()
                    .expect("the Short name field"),
            ),
        ),
        span(
            "Color 1",
            right_x(
                right_of_label(&nodes, "Color 1", Role::Label)
                    .into_iter()
                    .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
                    .expect("Color 1's hex"),
            ),
        ),
        span(
            "Color 2",
            right_x(
                right_of_label(&nodes, "Color 2", Role::Label)
                    .into_iter()
                    .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
                    .expect("Color 2's hex"),
            ),
        ),
        span(
            "Manager ID",
            right_x(
                right_of_label(&nodes, "Manager ID", Role::SpinButton)
                    .first()
                    .copied()
                    .expect("the manager control"),
            ),
        ),
        span(
            "Stadium ID",
            right_x(
                right_of_label(&nodes, "Stadium ID", Role::SpinButton)
                    .first()
                    .copied()
                    .expect("the stadium control"),
            ),
        ),
        {
            let b = node_labeled(&nodes, Role::Button, "Kit slots\u{2026}")
                .bounds()
                .expect("button bounds");
            b.x1 - b.x0
        },
    ];
    for ((unit, claim), &span) in crate::view::team::TEAM_ROW.iter().zip(spans.iter()) {
        assert!(
            span <= f64::from(*claim) && f64::from(*claim) <= span + 1.5,
            "{unit:?}: claim {claim} vs drawn {span}"
        );
    }
}

// `names_row`'s `changed() && team_text_fits` guard is what keeps drawing
// from being an edit: frames of the view with no input leave the flag and
// the session as they were.
#[test]
fn drawing_the_team_view_edits_nothing() {
    let path = fixture(PesVersion::Pes21, "team_view_no_input");
    let mut view = SaveView::new();
    view.open(&path);
    assert!(
        !team_entry(&view, 701).edit_flags.name,
        "premise: the name flag is clear"
    );
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let session = view.session.as_ref().unwrap();
    assert!(!session.is_dirty(), "three frames left the session clean");
    assert!(!session.can_undo(), "and pushed no undo entry");
    assert!(
        !team_entry(&view, 701).edit_flags.name,
        "the name flag stayed clear"
    );
}

// The same guard's other arm: a keystroke `team_text_fits` refuses is not
// taken — the field shows the stored text again next frame (the info
// strip's Name field's rule).
#[test]
fn a_team_name_keystroke_past_the_fields_limit_is_not_taken() {
    let path = fixture(PesVersion::Pes21, "team_name_overlong");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    // The version's own field length in bytes, read from the schema —
    // never a typed constant.
    let limit = usize::try_from(
        schema_for(PesVersion::Pes21)
            .team
            .texts
            .iter()
            .find(|spec| spec.text == TeamText::Name)
            .expect("every version stores the team name")
            .len,
    )
    .expect("a text field's length fits usize");
    let full = "A".repeat(limit);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("/3/"))
        .expect("the team's Name field");
    click(&mut window, name_field, &mut view);
    type_text(&mut window, &mut view, &full);
    assert_eq!(
        team_entry(&view, 701).name,
        full,
        "a name at the limit lands"
    );

    // One more letter past it: refused, the stored name stands.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some(full.as_str()))
        .expect("the team's Name field");
    click(&mut window, name_field, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            egui::Event::Text("B".to_owned()),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        team_entry(&view, 701).name,
        full,
        "the past-limit keystroke is not taken"
    );
}

// On the team view, fields the version does not store draw disabled in
// the same place with the gate tooltip naming the first version that has
// them.
#[test]
fn on_pes16_the_colours_manager_id_and_kit_slots_draw_disabled_with_their_versions() {
    let path = fixture(PesVersion::Pes16, "team_view_gated");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The two colour fields, disabled with "PES 17+": no edge either, as
    // a disabled control draws none.
    let mut colours: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::ColorWell
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("color field"))
        })
        .collect();
    colours.sort_by(|a, b| {
        a.bounds()
            .expect("bounds")
            .x0
            .partial_cmp(&b.bounds().expect("bounds").x0)
            .expect("ordered bounds")
    });
    assert_eq!(colours.len(), 2, "two colour fields");
    for field in &colours {
        assert!(field.is_disabled(), "the colour field is disabled");
        assert!(
            strokes_over(&shapes, field.bounds().expect("bounds")).is_empty(),
            "a disabled colour field draws no edge"
        );
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(field)),
            Some("PES 17+".to_string()),
            "the colour field's tooltip"
        );
    }
    // The Manager ID, its disabled stand-in with "PES 17+".
    let dash = disabled_dash(&nodes, "Manager ID");
    assert_eq!(
        tooltip_after_hover(&mut window, &mut view, centre(dash)),
        Some("PES 17+".to_string()),
        "the Manager ID's tooltip"
    );
    // The labels dim with their controls and answer the same hover.
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let text_color = visuals.text_color();
    let dimmed = dimmed_text_color(&visuals);
    assert_ne!(
        dimmed, text_color,
        "premise: the disabled grey is not the normal text colour"
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Color 1", "Color 2", "Manager ID"] {
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            dimmed,
            "{label}'s label dimmed with its control"
        );
        let node = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(label))
            .expect("the gated label");
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(node)),
            Some("PES 17+".to_string()),
            "{label}'s label answers a hover with the same version"
        );
    }
    // The hex beside a gated colour field is dimmed with it and answers
    // the same hover.
    let hexes: Vec<accesskit::Rect> = nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some("#000000"))
        .filter_map(accesskit::Node::bounds)
        .collect();
    assert_eq!(hexes.len(), 2, "the two colour fields' hexes");
    for bounds in hexes {
        assert_eq!(
            painted_text_color(&shapes, "#000000", bounds),
            dimmed,
            "the gated colour's hex dimmed with its field"
        );
    }
    let hex = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("#000000"))
        .expect("a gated colour's hex");
    assert_eq!(
        tooltip_after_hover(&mut window, &mut view, centre(hex)),
        Some("PES 17+".to_string()),
        "the hex answers a hover with the same version"
    );
    // The Stadium ID, which every version stores: a live number showing
    // the team's own.
    let stadium = team_entry(&view, 701)
        .stadium_id
        .expect("PES 16 stores the stadium ID");
    let control = right_of_label(&nodes, "Stadium ID", Role::SpinButton)
        .first()
        .copied()
        .expect("the Stadium ID control");
    assert!(!control.is_disabled(), "the Stadium ID is live");
    assert_eq!(control.value(), Some(stadium.to_string().as_str()));
    // The kit slots' no-records line names PES 17, inside the dialog.
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "(this version does not store kit slots)"),
        "the no-records line"
    );

    // On a version that stores them, the same labels paint the normal colour.
    let path = fixture(PesVersion::Pes21, "team_view_gated_21");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Color 1", "Color 2", "Manager ID"] {
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            text_color,
            "{label} stored on PES 21 paints the normal text colour"
        );
    }
}

#[test]
fn the_short_name_upper_cases_caps_at_three_and_sets_the_flags() {
    for (version, test) in [
        (PesVersion::Pes15, "short_name_15"),
        (PesVersion::Pes21, "short_name_21"),
    ] {
        let path = fixture(version, test);
        let mut view = SaveView::new();
        view.open(&path);
        assert_eq!(
            team_entry(&view, 701).short_name,
            "3",
            "premise: team 701's short name"
        );
        let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
        open_team_view(&mut window, &mut view, 701);
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let field = nodes
            .iter()
            .find(|node| node.role() == Role::TextInput && node.value() == Some("3"))
            .expect("the Short name field");
        click(&mut window, field, &mut view);
        type_text(&mut window, &mut view, "abc");
        let team = team_entry(&view, 701);
        assert_eq!(team.short_name, "ABC", "upper-cased as typed");
        assert!(team.edit_flags.name, "the name flag is set");

        // A fourth letter is not taken: the field shows the stored text again.
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let field = nodes
            .iter()
            .find(|node| node.role() == Role::TextInput && node.value() == Some("ABC"))
            .expect("the Short name field");
        click(&mut window, field, &mut view);
        type_text(&mut window, &mut view, "abcd");
        assert_eq!(team_entry(&view, 701).short_name, "ABC");
        assert_eq!(
            team_entry(&view, 701).edit_flags.short_name,
            (version == PesVersion::Pes15).then_some(true),
            "the short-name flag on {version}"
        );
    }
}

// A stadium ID edit sets the version's stadium flag (PES 20/21 store
// it), the way a name edit sets the name flag.
#[test]
fn a_stadium_id_edit_sets_the_stadium_flag() {
    let path = fixture(PesVersion::Pes21, "stadium_id_flag");
    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(
        team_entry(&view, 701).edit_flags.stadium,
        Some(false),
        "premise: the stadium flag starts clear"
    );
    let stored = team_entry(&view, 701)
        .stadium_id
        .expect("PES 21 stores the stadium ID");
    let typed = if stored == 9 { "8" } else { "9" };
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let control = right_of_label(&nodes, "Stadium ID", Role::SpinButton)
        .first()
        .copied()
        .expect("the Stadium ID control");
    click(&mut window, control, &mut view);
    window.frame(vec![egui::Event::Text(typed.to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );

    let team = team_entry(&view, 701);
    assert_eq!(
        team.stadium_id,
        Some(typed.parse().expect("a digit")),
        "the typed id stores"
    );
    assert_eq!(
        team.edit_flags.stadium,
        Some(true),
        "the edit sets the stadium flag"
    );
}

// The team view's fields:
// the team's name as the title, its name, short name and colours, the
// manager and stadium IDs, the `Kit slots…` button, then the Tactics.
// SE-GUI-10
#[test]
fn the_team_view_draws_its_fields_and_edits_them_undoable() {
    let (mut view, mut window) = open_734_with("team_view_fields", &[], 0);
    open_team_view(&mut window, &mut view, 734);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The title, the fields, the ids and the button; the kit slot rows are
    // behind the dialog, not in the view.
    assert!(
        label_value(&nodes, "734 /lit/"),
        "the title is the team's id and name"
    );
    assert_eq!(
        bar_name_field(&nodes).value(),
        Some("/lit/"),
        "the name field"
    );
    control_right_of(&nodes, "Manager ID");
    control_right_of(&nodes, "Stadium ID");
    node_labeled(&nodes, Role::Button, "Kit slots\u{2026}");
    assert!(
        !label_value(&nodes, "Slot 1"),
        "the kit slot rows stay behind the dialog"
    );
    node_labeled(&nodes, Role::Button, &menu_label("Copy/swap"));

    // A short-name edit lands in the team's entry as one undoable edit.
    let before = team_entry(&view, 734).short_name;
    assert_ne!(before, "LTZ", "premise: a real change");
    let short = right_of_label(&nodes, "Short name", Role::TextInput)
        .first()
        .copied()
        .expect("the view's Short name field");
    click(&mut window, short, &mut view);
    type_text(&mut window, &mut view, "ltz");
    assert_eq!(
        team_entry(&view, 734).short_name,
        "LTZ",
        "upper-cased as typed"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undoable edit");
}

// The Kit slot assignments dialog (the team view's `Kit slots…` button):
// it opens, a slot's Number edit is one undoable edit, OK closes it.
#[test]
fn the_kit_slots_dialog_opens_edits_and_closes() {
    let (mut view, mut window) = open_734_with("kit_slots_dialog", &[], 0);
    open_team_view(&mut window, &mut view, 734);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        &mut view,
    );

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "Kit slot assignments"),
        "the dialog's title"
    );
    let before = team_entry(&view, 734).kit_slots.expect("PES 17 kit slots")[0].number;
    // Slot 1's row: Number and Bound team ID, in x order.
    let number = right_of_label(&nodes, "Slot 1", Role::SpinButton)
        .first()
        .copied()
        .expect("slot 1's Number");
    click(&mut window, number, &mut view);
    window.frame(vec![egui::Event::Text("9".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        team_entry(&view, 734).kit_slots.expect("kit slots")[0].number,
        9
    );
    assert_ne!(before, 9, "premise: a real change");

    // OK closes the dialog (its title gone, the button back in reach).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "OK"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !label_value(&nodes, "Kit slot assignments"),
        "the dialog closed"
    );
    // The dialog held Ctrl+Z while open; closed, the edit is one undo.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undoable edit");
}

mod boglo;
mod layout;
mod menu;
