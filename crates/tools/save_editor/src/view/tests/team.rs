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
    // The one undo restores the edit whole, the flag back to clear (a
    // frame first, so the field has given up the keyboard and Ctrl+Z is
    // the session's undo, not the text's own).
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
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

// The drafts' window on a version without kit slots: the short dialog is the
// window's 360 points wide, OK at its right inside the stroke and the
// 12-point margin.
#[test]
fn the_kit_slots_dialog_is_360_wide_with_ok_at_the_right() {
    let path = fixture(PesVersion::Pes21, "kit_slots_window");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Kit slots\u{2026}"),
        &mut view,
    );
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "(this version does not store kit slots)"),
        "premise: PES 21 stores no kit slots"
    );
    let outer = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_kit_slots")))
        .expect("the dialog's modal has an area");
    let ok = node_labeled(&nodes, Role::Button, "OK")
        .bounds()
        .expect("OK has bounds");
    let width = outer.width();
    assert!(
        (width - 360.0).abs() <= 0.5,
        "the dialog is 360 wide, not {width}"
    );
    let right = f64::from(outer.right()) - ok.x1;
    assert!(
        (right - 13.0).abs() <= 0.5,
        "OK's right is 13 inside the dialog's, not {right}"
    );
}

// The bar's name field gets its 140 points (the draft's `#team-name`)
// from `add_sized`: the field keeps its own width rather than the wrapped
// row's offer, measured on the frame the team view first draws (the
// click's release frame), not a later one where a collapsed first frame
// would already be past.
#[test]
fn the_team_name_field_gets_its_full_width_on_the_first_frame() {
    let path = fixture(PesVersion::Pes21, "team_name_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The release frame is the first one that draws the team view.
    let nodes = click_at(&mut window, centre(team_row(&nodes, &view, 701)), &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(701)),
        "the click selected team 701's row"
    );
    let name = nodes
        .iter()
        .find(|node| node.role() == Role::TextInput && node.value() == Some("/3/"))
        .expect("the team's Name field");
    let bounds = name.bounds().expect("bounds");
    assert!(
        (bounds.x1 - bounds.x0 - 140.0).abs() <= 1.5,
        "the Name field draws at its 140 points on the first frame, got {bounds:?}"
    );
}

// The team view's title is `<id> <name>` in the body text's size, its
// name in the strong weight, not a heading's size.
#[test]
fn the_title_is_the_teams_name_in_the_bodys_strong_weight() {
    let path = fixture(PesVersion::Pes21, "team_title_body");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (body, strong) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            egui::TextStyle::Body.resolve(&style),
            style.visuals.strong_text_color(),
        )
    };
    let title = label_shape(&nodes, &shapes, "701 /3/");
    for section in &title.galley.job.sections {
        assert_eq!(
            section.format.font_id.size, body.size,
            "the title's font is the body's"
        );
    }
    assert_eq!(
        title
            .galley
            .job
            .sections
            .last()
            .expect("the name's run")
            .format
            .color,
        strong,
        "the title's plain name run in the strong colour"
    );
}

// Each team colour's `#RRGGBB` beside its swatch is in the weak grey, as the
// draft draws it, and in the monospace font.
#[test]
fn the_colour_codes_are_in_the_weak_grey() {
    let path = fixture(PesVersion::Pes21, "team_colour_codes_weak");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (weak, monospace) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            style.visuals.weak_text_color(),
            egui::TextStyle::Monospace.resolve(&style),
        )
    };
    for label in ["Col. 1", "Col. 2"] {
        let hex = right_of_label(&nodes, label, Role::Label)
            .into_iter()
            .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
            .unwrap_or_else(|| panic!("{label}'s hex"))
            .value()
            .expect("the hex's text")
            .to_owned();
        assert_eq!(
            painted_label_color(&nodes, &shapes, &hex),
            weak,
            "{label}'s {hex} in the weak grey"
        );
        let painted = label_shape(&nodes, &shapes, &hex);
        for section in &painted.galley.job.sections {
            assert_eq!(section.format.font_id, monospace, "{label}'s {hex} font");
        }
    }
}

// At the 1280 x 780 default pane the team view's fields, the ids and the
// `Kit slots…` button share one row, the name box is the draft's 140
// points and the colours are labelled `Col. 1`/`Col. 2`.
#[test]
fn the_fields_ids_and_kit_slots_button_share_one_row_at_the_default_pane() {
    let path = fixture(PesVersion::Pes21, "team_header_row");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let row_y = label_bounds(&nodes, "Short").y0;
    for label in ["Name", "Col. 1", "Col. 2", "Manager ID", "Stadium ID"] {
        let bounds = nodes
            .iter()
            .filter(|node| node.role() == Role::Label && node.value() == Some(label))
            .filter_map(|node| node.bounds())
            .min_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"))
            .unwrap_or_else(|| panic!("no {label:?} label"));
        assert!(
            (bounds.y0 - row_y).abs() < 4.0,
            "{label} shares the fields' row ({bounds:?} vs {row_y})"
        );
    }
    let button = node_labeled(&nodes, Role::Button, "Kit slots\u{2026}")
        .bounds()
        .expect("the Kit slots button's bounds");
    assert!(
        (button.y0 - row_y).abs() < 4.0,
        "the Kit slots button shares the row ({button:?} vs {row_y})"
    );
    let name = bar_name_field(&nodes).bounds().expect("bounds");
    assert!(
        (name.x1 - name.x0 - 140.0).abs() <= 1.5,
        "the name box is the draft's 140 points ({name:?})"
    );
}

// The row's controls stand where the draft's do, measured from the Name
// box's left edge (the draft at 1280 x 780, PES 21 save, team 701): the
// units 8 apart, egui's item spacing, a colour's or an id's parts a space
// apart, the ids' cells padded 4 on each side.
#[test]
fn the_rows_controls_stand_where_the_drafts_do_from_the_name_box() {
    let path = fixture(PesVersion::Pes21, "team_row_offsets");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let name = bar_name_field(&nodes).bounds().expect("bounds").x0;
    let left_of = |label: &str, role: Role| {
        right_of_label(&nodes, label, role)
            .first()
            .copied()
            .unwrap_or_else(|| panic!("no {role:?} right of {label:?}"))
            .bounds()
            .expect("bounds")
            .x0
    };
    let kit_slots = node_labeled(&nodes, Role::Button, crate::view::team::KIT_SLOTS_LABEL)
        .bounds()
        .expect("bounds")
        .x0;
    let label = |text: &str| label_bounds(&nodes, text).x0;
    let edges = [
        ("the Short label", label("Short"), 148.0),
        ("the Short box", left_of("Short", Role::TextInput), 186.6),
        ("Col. 1's swatch", left_of("Col. 1", Role::ColorWell), 270.0),
        ("Col. 2's swatch", left_of("Col. 2", Role::ColorWell), 389.2),
        ("the Manager ID label", label("Manager ID"), 476.8),
        (
            "the Manager ID box",
            left_of("Manager ID", Role::SpinButton),
            544.9,
        ),
        ("the Stadium ID label", label("Stadium ID"), 598.9),
        (
            "the Stadium ID box",
            left_of("Stadium ID", Role::SpinButton),
            663.7,
        ),
        ("the Kit slots button", kit_slots, 721.7),
    ];
    for (control, left, draft) in edges {
        assert!(
            (left - name - draft).abs() <= 1.0,
            "{control} stands {draft} right of the Name box, got {}",
            left - name
        );
    }
}

// The id cells' padding, 4 on each side, stands them apart exactly: the
// Stadium ID label starts 8 past the Manager ID box (the two paddings, no
// row gap between the cells) and the Manager ID label 12 past Col. 2's hex
// (the row's gap and Manager ID's padding).
#[test]
fn the_id_cells_stand_their_paddings_apart() {
    let path = fixture(PesVersion::Pes21, "team_id_cell_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let colour_hex_right = right_of_label(&nodes, "Col. 2", Role::Label)
        .into_iter()
        .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
        .expect("Col. 2's hex")
        .bounds()
        .expect("bounds")
        .x1;
    let manager_box_right = right_of_label(&nodes, "Manager ID", Role::SpinButton)
        .first()
        .copied()
        .expect("the manager control")
        .bounds()
        .expect("bounds")
        .x1;
    let gaps = [
        (
            "Col. 2's hex to the Manager ID label",
            label_bounds(&nodes, "Manager ID").x0 - colour_hex_right,
            12.0,
        ),
        (
            "the Manager ID box to the Stadium ID label",
            label_bounds(&nodes, "Stadium ID").x0 - manager_box_right,
            8.0,
        ),
    ];
    for (between, gap, expected) in gaps {
        assert!(
            (gap - expected).abs() <= 0.01,
            "{between} is {expected}, got {gap}"
        );
    }
}

/// The left edges of the team row's Short box, colour swatches, id
/// controls and `Kit slots…` button, from the Name box's, on `version`'s
/// fixture: the id controls are the first control right of their labels,
/// the number box or its disabled stand-in.
fn row_edges(version: PesVersion, test: &str) -> [f64; 6] {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name = bar_name_field(&nodes).bounds().expect("bounds").x0;
    let left = |node: &accesskit::Node| node.bounds().expect("bounds").x0 - name;
    [
        left(control_right_of(&nodes, "Short")),
        left(control_right_of(&nodes, "Col. 1")),
        left(control_right_of(&nodes, "Col. 2")),
        left(control_right_of(&nodes, "Manager ID")),
        left(control_right_of(&nodes, "Stadium ID")),
        left(node_labeled(
            &nodes,
            Role::Button,
            crate::view::team::KIT_SLOTS_LABEL,
        )),
    ]
}

// On PES 17, whose save stores no manager or stadium ID, the ids' disabled
// stand-ins take the number boxes' width: the row's controls stand where
// they do on PES 21.
#[test]
fn the_rows_controls_stand_still_on_a_version_without_the_ids() {
    const CONTROLS: [&str; 6] = [
        "the Short box",
        "Col. 1's swatch",
        "Col. 2's swatch",
        "the Manager ID control",
        "the Stadium ID control",
        "the Kit slots button",
    ];
    let stored = row_edges(PesVersion::Pes21, "team_row_edges_21");
    let gated = row_edges(PesVersion::Pes17, "team_row_edges_17");
    for ((control, gated), stored) in CONTROLS.iter().zip(gated).zip(stored) {
        assert!(
            (gated - stored).abs() <= 1.0,
            "{control} stands {gated} on PES 17 against {stored} on PES 21"
        );
    }
}

// The ids share the fields' row because the Tactics editor below needs
// the height: at the default pane the view's lowest drawn stroke (the
// lists' and Automatic settings' frame bottoms) paints the draft's last
// row, pixel row 741 of the 1280 x 780 client, whose y the pane shares.
#[test]
fn the_team_view_fits_the_default_pane() {
    let path = fixture(PesVersion::Pes21, "team_view_fit");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    // Three frames: the scroll's claimed content size lands a frame late.
    let mut shapes = Vec::new();
    for _ in 0..3 {
        let (_, _, frame_shapes) =
            window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        shapes = frame_shapes;
    }
    let (left, _) = pane_frame(&shapes, window.screen);
    let bottom = shapes
        .iter()
        .map(|clipped| clipped.shape.visual_bounding_rect())
        .filter(|rect| {
            // Inside the detail pane, and not its own full-height frame.
            f64::from(rect.min.x) >= left - 1.0 && rect.height() < window.screen.y * 0.75
        })
        .map(|rect| f64::from(rect.max.y))
        .fold(0.0, f64::max);
    // A 1-point stroke paints the row above its bounding bottom.
    let last_row = bottom - 1.0;
    assert!(
        (last_row - 741.0).abs() <= 1.0,
        "the lowest stroke paints the draft's last row, 741 ({last_row})"
    );
}

/// The team view's fields' row and the line under it, from one frame of
/// team 701's view at the default pane: the bottom of the row's tallest
/// control, the line's painted row (the top of its stroke, as the draft's
/// `.sep` border covers one pixel row) and the line's left end.
fn fields_line(test: &str) -> (Vec<accesskit::Node>, f64, f64, f64) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let name = bar_name_field(&nodes).bounds().expect("bounds");
    let on_row = |b: &accesskit::Rect| b.y0 < name.y1 && b.y1 > name.y0;
    let row_bottom = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.role(),
                Role::TextInput | Role::ColorWell | Role::SpinButton | Role::Button
            )
        })
        .filter_map(accesskit::Node::bounds)
        .filter(|b| on_row(b) && b.x0 >= name.x0)
        .map(|b| b.y1)
        .fold(name.y1, f64::max);
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let (line, line_left) = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].y) > row_bottom
                    && points[0].x.min(points[1].x) <= name.x0 as f32 =>
            {
                Some((
                    f64::from(points[0].y),
                    f64::from(points[0].x.min(points[1].x)),
                ))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered ys"))
        .expect("a stroke line under the fields' row");
    let painted_row = line - f64::from(stroke.width) / 2.0;
    (nodes, row_bottom, painted_row, line_left)
}

// The draft's `.sep` under the fields' row: its painted row 11 points below
// the row's tallest control (the row's 3-point margin and the line's 8),
// the Tactics' first control 11.5 below that row (the line's own point, its
// 8-point margin and the bar's 2.5 over the Fluid formation checkbox).
#[test]
fn the_line_under_the_fields_stands_11_from_the_row_and_11_5_from_the_tactics() {
    let (nodes, row_bottom, line, line_left) = fields_line("team_separator_gaps");
    // The view's nodes only (the line spans the view): the player list's
    // rows left of it run past the line too.
    let tactics_top = nodes
        .iter()
        .filter_map(accesskit::Node::bounds)
        .filter(|b| b.y0 > line && b.x0 >= line_left)
        .map(|b| b.y0)
        .fold(f64::MAX, f64::min);
    assert!(
        (line - row_bottom - 11.0).abs() <= 0.5,
        "the line's row stands 11 below the row's tallest control ({line} vs {row_bottom})"
    );
    assert!(
        (tactics_top - line - 11.5).abs() <= 0.5,
        "the Tactics' first control stands 11.5 below the line's row ({tactics_top} vs {line})"
    );
}

// The team view's pane ends where the draft's `.detail` content does, at
// the view's right edge, 8 points short of the window's: the line under
// the fields, which spans the pane, ends there, as the draft's `.sep`
// does at 1272.
#[test]
fn the_team_views_line_ends_8_from_the_windows_edge() {
    let path = fixture(PesVersion::Pes21, "team_pane_right");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let line_right = shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && (points[0].x - points[1].x).abs() > window.screen.x * 0.5 =>
            {
                Some(f64::from(points[0].x.max(points[1].x)))
            }
            _ => None,
        })
        .expect("the line under the fields, across the pane");
    assert!(
        (line_right - f64::from(PANE_RIGHT)).abs() <= 1.0,
        "the line ends at {PANE_RIGHT}, the draft's 1272 ({line_right})"
    );
}

// The Tactics bars' choices (the draft's `.sel`s, 17 tall, centred in the
// 23-point `.tacbar` 8 under the line) stand 12 under the line's painted
// row and end 29 under it: the draft's 98 to 115 under its row 86.
#[test]
fn the_tactics_bars_choices_stand_12_under_the_line() {
    let (nodes, _, line, _) = fields_line("team_bar_under_line");
    for label in [
        "1",
        "2",
        "3",
        "Kick-off",
        "In-possession",
        "Out-of-possession",
    ] {
        let choice = node_labeled(&nodes, Role::Button, label)
            .bounds()
            .expect("bounds");
        assert!(
            (choice.y0 - line - 12.0).abs() <= 1.0,
            "{label} starts 12 under the line's row ({} vs {line})",
            choice.y0
        );
        assert!(
            (choice.y1 - line - 29.0).abs() <= 1.0,
            "{label} ends 29 under the line's row ({} vs {line})",
            choice.y1
        );
    }
}

// The fields' row's boxes stand 7 under the title's line box: the draft's
// title keeps 4 points under it and the row its own 3-point margin round
// its controls (its 57 under the title's 50).
#[test]
fn the_fields_row_stands_7_under_the_title() {
    let path = fixture(PesVersion::Pes21, "team_row_under_title");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = label_bounds(&nodes, "701 /3/");
    let name = bar_name_field(&nodes).bounds().expect("bounds");
    assert!(
        (name.y0 - title.y1 - 7.0).abs() <= 1.0,
        "the Name box stands 7 under the title ({} vs {})",
        name.y0,
        title.y1
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
            "Short",
            right_x(
                right_of_label(&nodes, "Short", Role::TextInput)
                    .first()
                    .copied()
                    .expect("the Short field"),
            ),
        ),
        span(
            "Col. 1",
            right_x(
                right_of_label(&nodes, "Col. 1", Role::Label)
                    .into_iter()
                    .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
                    .expect("Col. 1's hex"),
            ),
        ),
        span(
            "Col. 2",
            right_x(
                right_of_label(&nodes, "Col. 2", Role::Label)
                    .into_iter()
                    .find(|node| node.value().is_some_and(|v| v.starts_with('#')))
                    .expect("Col. 2's hex"),
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

/// The boots and gloves dialog opened on team 701 of the PES 21 fixture,
/// a player row selected, the aesthetics lock off: the view, the window
/// and the open dialog's nodes.
fn open_boglo(test: &str) -> (SaveView, Headless, Vec<accesskit::Node>) {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(
            &nodes,
            Role::Button,
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    (view, window, nodes)
}

/// The dialog's fields: the incremental start, the read-only end box
/// right of it, and the boots' and the gloves' one-ID boxes (the fields
/// right of the two "Set all to the same ID:" radios, the boots' above).
struct BogloFields<'a> {
    start: &'a accesskit::Node,
    end: &'a accesskit::Node,
    boots: &'a accesskit::Node,
    gloves: &'a accesskit::Node,
}

fn boglo_fields(nodes: &[accesskit::Node]) -> BogloFields<'_> {
    let incremental = node_labeled(nodes, Role::RadioButton, "Set incremental value:");
    let start = right_of(nodes, incremental, Role::SpinButton);
    let end = first_right_of(nodes, start, |node| {
        node.value()
            .is_some_and(|value| value.chars().all(|c| c.is_ascii_digit()))
    })
    .expect("the end box right of the start");
    let mut same: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::RadioButton && node.label() == Some("Set all to the same ID:")
        })
        .collect();
    same.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    assert_eq!(same.len(), 2, "the boots and gloves one-ID radios");
    BogloFields {
        start,
        end,
        boots: right_of(nodes, same[0], Role::SpinButton),
        gloves: right_of(nodes, same[1], Role::SpinButton),
    }
}

/// `text` typed into `field` (a click, the text, Enter); the next frame's
/// nodes.
fn type_into(
    window: &mut Headless,
    view: &mut SaveView,
    field: &accesskit::Node,
    text: &str,
) -> Vec<accesskit::Node> {
    click(window, field, view);
    window.frame(vec![egui::Event::Text(text.to_owned())], |ui| {
        show(ui, view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, view, &|| Vec::new()),
    );
    draw(window, view)
}

// The dialog's layout (the draft's `dlgBoglo`): a group per side titled `Boot IDs` and
// `Glove IDs`, each option's field in one column across both groups, the
// one-ID boxes as wide as the incremental range (its start, the dash and
// its end box).
#[test]
fn the_boglo_dialogs_fields_share_one_column_across_both_groups() {
    let (_, _, nodes) = open_boglo("boglo_columns");
    for title in ["Boot IDs", "Glove IDs"] {
        assert!(label_value(&nodes, title), "the {title} group's title");
    }
    for label in ["Unchanged", "Set like boot IDs"] {
        node_labeled(&nodes, Role::RadioButton, label);
    }
    let fields = boglo_fields(&nodes);
    let bounds = |node: &accesskit::Node| node.bounds().expect("a field's bounds");
    let left = bounds(fields.start).x0;
    for (name, field) in [("boots", fields.boots), ("gloves", fields.gloves)] {
        assert!(
            (bounds(field).x0 - left).abs() <= 0.5,
            "the {name} one-ID box starts where the range does ({:?} vs {left})",
            bounds(field)
        );
        assert!(
            (bounds(field).x1 - bounds(fields.end).x1).abs() <= 1.0,
            "the {name} one-ID box ends where the range does ({:?} vs {:?})",
            bounds(field),
            bounds(fields.end)
        );
    }
}

// The range's dash stands 8 from its start box and from its end box,
// egui's item spacing, as the draft's `.row`: the end box starts 22.4
// past the start box's right (the dash 6.4 wide), and the one-ID boxes
// span the range's 126.4.
#[test]
fn the_boglo_ranges_dash_stands_8_from_either_box() {
    let (_, _, nodes) = open_boglo("boglo_range_gap");
    let fields = boglo_fields(&nodes);
    let bounds = |node: &accesskit::Node| node.bounds().expect("a field's bounds");
    let start = bounds(fields.start);
    let end = bounds(fields.end);
    assert!(
        (end.x0 - start.x1 - 22.4).abs() <= 1.0,
        "the end box starts 22.4 past the start box ({start:?} vs {end:?})"
    );
    let boots = bounds(fields.boots);
    assert!(
        (boots.x1 - boots.x0 - 126.4).abs() <= 1.0,
        "the one-ID box spans the range's 126.4 ({boots:?})"
    );
}

// The range's end box reads the start plus 22 (slot 23's ID), follows an
// edit of the start, and is no field: a click on it focuses nothing.
#[test]
fn the_end_box_reads_the_start_plus_22_and_follows_it_read_only() {
    let (mut view, mut window, nodes) = open_boglo("boglo_end_box");
    let fields = boglo_fields(&nodes);
    let start: u32 = fields
        .start
        .value()
        .expect("the start's value")
        .parse()
        .expect("a number");
    assert_eq!(fields.end.value(), Some((start + 22).to_string().as_str()));
    assert_ne!(start, 60, "premise: the edit changes the start");

    let nodes = type_into(&mut window, &mut view, fields.start, "60");
    let fields = boglo_fields(&nodes);
    assert_eq!(
        fields.start.value(),
        Some("60"),
        "premise: the start edited"
    );
    assert_eq!(fields.end.value(), Some("82"), "the end follows the start");
    assert_eq!(fields.end.role(), Role::Label, "the end box is no field");

    click(&mut window, fields.end, &mut view);
    assert_eq!(
        window.ctx.memory(|memory| memory.focused()),
        None,
        "a click on the end box focuses nothing"
    );
}

// The end box's number in the dark theme's grey (`#a0a0a0`) and in its
// light twin (`#5f5f5f`) on the light theme, which reads at the same
// contrast against the light background.
#[test]
fn the_end_boxs_number_takes_each_themes_grey() {
    for (theme, grey) in [
        (egui::Theme::Dark, egui::Color32::from_gray(0xa0)),
        (egui::Theme::Light, egui::Color32::from_gray(0x5f)),
    ] {
        let (mut view, mut window, _) = open_boglo(&format!("boglo_readout_{theme:?}"));
        window.ctx.set_theme(theme);
        draw(&mut window, &mut view);
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let end = boglo_fields(&nodes)
            .end
            .value()
            .expect("the end box's number")
            .to_owned();
        assert_eq!(
            painted_label_color(&nodes, &shapes, &end),
            grey,
            "the end box's number on the {theme:?} theme"
        );
    }
}

// Set, incremental from 60: every filled slot `k` (1-based) gets boots ID
// 59 + k, and the dialog closes.
#[test]
fn set_numbers_every_filled_slot_from_the_start_and_closes() {
    let (mut view, mut window, nodes) = open_boglo("boglo_set");
    let slots = rostered(&view, 701);
    assert!(slots.len() > 2, "premise: several filled slots");
    assert_ne!(
        player_entry(&view, slots[0].1).appearance.boots_id,
        60,
        "premise: slot 1's boots ID changes"
    );
    click(
        &mut window,
        node_labeled(&nodes, Role::RadioButton, "Set incremental value:"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    let nodes = type_into(&mut window, &mut view, boglo_fields(&nodes).start, "60");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set"),
        &mut view,
    );
    for &(slot, id) in &slots {
        let expected = 60 + u32::try_from(slot).expect("a roster has at most 40 slots");
        assert_eq!(
            player_entry(&view, id).appearance.boots_id,
            expected,
            "slot {}'s boots",
            slot + 1
        );
    }
    assert!(view.boglo.is_none(), "Set closes the dialog");
}

// A start past the boots ID's range: Set writes nothing and the error line
// shows under the groups, the dialog staying open.
#[test]
fn a_start_past_the_range_shows_the_error_under_the_groups_and_writes_nothing() {
    let (mut view, mut window, nodes) = open_boglo("boglo_past_range");
    let slots = rostered(&view, 701);
    let before: Vec<PlayerEntry> = slots
        .iter()
        .map(|&(_, id)| player_entry(&view, id))
        .collect();
    click(
        &mut window,
        node_labeled(&nodes, Role::RadioButton, "Set incremental value:"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    let nodes = type_into(&mut window, &mut view, boglo_fields(&nodes).start, "70000");
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set"),
        &mut view,
    );
    let nodes = draw(&mut window, &mut view);
    for (&(_, id), entry) in slots.iter().zip(&before) {
        assert_eq!(&player_entry(&view, id), entry, "{id} unchanged");
    }
    assert!(view.boglo.is_some(), "the dialog stays open");
    let error = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|value| value.contains("70000"))
        })
        .expect("the error line names the value");
    let gloves = boglo_fields(&nodes).gloves.bounds().expect("bounds");
    let set = node_labeled(&nodes, Role::Button, "Set")
        .bounds()
        .expect("bounds");
    let line = error.bounds().expect("bounds");
    assert!(
        gloves.y1 < line.y0 && line.y1 <= set.y0,
        "the error line under the groups, over the buttons ({line:?})"
    );
}

/// The boots and gloves ID fields the open dialog prefilled: the
/// incremental start, then the boots' and the gloves' one-ID boxes.
fn boglo_prefill(nodes: &[accesskit::Node]) -> [String; 3] {
    let fields = boglo_fields(nodes);
    [fields.start, fields.boots, fields.gloves]
        .map(|field| field.value().expect("a prefilled field's value").to_owned())
}

// The dialog opened from the team row prefills from the first filled
// roster slot's player — Incremental's start and the Same-ID fields hold
// his boots and gloves ids.
#[test]
fn the_boots_dialog_prefills_the_team_rows_first_filled_slot() {
    let path = fixture(PesVersion::Pes21, "boglo_team_row");
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let (boots, gloves) = {
        let first = rostered(&view, 701)[0].1;
        let player = player_entry(&view, first);
        (player.appearance.boots_id, player.appearance.gloves_id)
    };
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);

    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(
            &nodes,
            Role::Button,
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        boglo_prefill(&nodes),
        [boots.to_string(), boots.to_string(), gloves.to_string()],
        "Incremental's start and the Same-ID fields hold the first filled slot's ids"
    );
}

// The same prefill skips a roster id no player record carries: the first
// filled slot's id is dangling, so the dialog prefills from the second's
// player instead of panicking on the dangling one.
#[test]
fn the_boots_dialog_prefills_past_a_dangling_roster_id() {
    let path = fixture(PesVersion::Pes21, "boglo_dangling");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    // Team 701's slot 1 names an id no player record carries.
    file.team_mut(701).unwrap().roster[0].player_id = 99999;
    let second = file.team(701).unwrap().roster[1].player_id;
    assert!(
        file.player(99999).is_none() && file.player(second).is_some(),
        "premise: slot 1 dangles, slot 2 resolves"
    );
    let (boots, gloves) = {
        let player = file.player(second).unwrap();
        (player.appearance.boots_id, player.appearance.gloves_id)
    };
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);

    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(
            &nodes,
            Role::Button,
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        boglo_prefill(&nodes),
        [boots.to_string(), boots.to_string(), gloves.to_string()],
        "the fields hold slot 2's player's ids, not the dangling slot 1's"
    );
}

// On the team view, fields the version does not store draw disabled in
// the same place with the gate tooltip naming the first version that has
// them.
#[test]
fn on_pes16_the_colours_ids_and_kit_slots_draw_disabled_with_their_versions() {
    let path = fixture(PesVersion::Pes16, "team_view_gated");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The two colour fields, disabled with "PES 17+".
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
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(field)),
            Some("PES 17+".to_string()),
            "the colour field's tooltip"
        );
    }
    // Manager ID and Stadium ID, each its disabled stand-in with "PES 19+".
    for label in ["Manager ID", "Stadium ID"] {
        let dash = disabled_dash(&nodes, label);
        assert_eq!(
            tooltip_after_hover(&mut window, &mut view, centre(dash)),
            Some("PES 19+".to_string()),
            "{label}'s tooltip"
        );
    }
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

// The team view's fields (plans "Editing UI", "The team view" bullet):
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
    let short = right_of_label(&nodes, "Short", Role::TextInput)
        .first()
        .copied()
        .expect("the view's Short field");
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
    // A frame first, so the field has given up the keyboard and Ctrl+Z is
    // the session's undo, not the text's own.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undoable edit");

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
}

// SE-GUI-23
#[test]
fn the_team_menu_operations_edit_every_filled_slot_as_one_undoable_edit() {
    let path = fixture(PesVersion::Pes21, "team_menu_ops");
    // A goalkeeper among the CB placeholders, so the position names cover
    // two table entries; the visual flags set, so Clear visual flags is a
    // real change rather than a no-op batch.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().positions.registered = 0;
    let ids: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for id in ids {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    // A rostered player of another team, to prove the operations stay on
    // the selected player's team.
    let other_id = {
        let file = view.session.as_ref().unwrap().file();
        file.teams()
            .iter()
            .filter(|team| team.id != 701)
            .flat_map(|team| team.roster.iter())
            .map(|slot| slot.player_id)
            .find(|&id| id != 0)
            .expect("a player of another team")
    };
    let other_before = player_entry(&view, other_id);
    assert!(!player_entry(&view, 70101).edit_flags.player, "premise");

    // Set edited flag: every filled slot's player flag is set.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set edited flag"),
        &mut view,
    );
    let slots = rostered(&view, 701);
    assert!(slots.len() > 2, "premise: several filled slots");
    for &(_, id) in &slots {
        assert!(player_entry(&view, id).edit_flags.player, "{id} flagged");
    }
    assert_eq!(player_entry(&view, other_id), other_before, "untouched");

    // Set boots and gloves IDs for everyone: incremental from 100, gloves
    // like the boot IDs — slot k gets 100 + k - 1 on both (the check's
    // bound is the record's 14-bit field, not settings.toml's 0 to 100).
    let start_value = 100u32;
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(
            &nodes,
            Role::Button,
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let incremental = node_labeled(&nodes, Role::RadioButton, "Set incremental value:");
    click(&mut window, incremental, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let incremental = node_labeled(&nodes, Role::RadioButton, "Set incremental value:");
    let start = right_of(&nodes, incremental, Role::SpinButton);
    click(&mut window, start, &mut view);
    window.frame(vec![egui::Event::Text(start_value.to_string())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::RadioButton, "Set like boot IDs"),
        &mut view,
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set"),
        &mut view,
    );
    for &(slot, id) in &slots {
        let player = player_entry(&view, id);
        let expected = start_value + u32::try_from(slot).expect("a roster has at most 40 slots");
        assert_eq!(
            player.appearance.boots_id,
            expected,
            "slot {}'s boots",
            slot + 1
        );
        assert_eq!(
            player.appearance.gloves_id,
            expected,
            "slot {}'s gloves",
            slot + 1
        );
    }

    // Set player names to positions: the registered position's full name.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set player names to positions"),
        &mut view,
    );
    for &(slot, id) in &slots {
        let player = player_entry(&view, id);
        assert_eq!(
            player.name,
            POSITION_NAMES[usize::from(player.positions.registered)],
            "slot {}'s name",
            slot + 1
        );
    }
    assert_eq!(player_entry(&view, 70101).name, "Goalkeeper");
    assert_eq!(player_entry(&view, 70102).name, "Center Back");

    // Clear visual flags by Ctrl+F: the four flags off on every slot.
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &slots {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            !(flags.face || flags.hair || flags.physique || flags.strip),
            "{id}'s visual flags cleared"
        );
    }

    // Four operations, four undo entries; the undos put every player back.
    assert_eq!(undo_entries(&mut window, &mut view), 4, "one entry each");
    assert_eq!(player_entry(&view, 70101).name, "PLACEHOLDER");
    assert!(!player_entry(&view, 70101).edit_flags.player);
    // Redo them all, save, reopen: the file holds the operation's values.
    for _ in 0..4 {
        window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    for &(slot, id) in &slots {
        let player = reopened.player(id).expect("the player");
        let expected = start_value + u32::try_from(slot).expect("a roster has at most 40 slots");
        assert_eq!(player.appearance.boots_id, expected);
        assert_eq!(player.appearance.gloves_id, expected);
        assert_eq!(
            player.name,
            POSITION_NAMES[usize::from(player.positions.registered)]
        );
        assert!(player.edit_flags.player);
        let flags = &player.edit_flags;
        assert!(!(flags.face || flags.hair || flags.physique || flags.strip));
    }
    assert_eq!(
        reopened.player(other_id).unwrap(),
        &other_before,
        "a player of another team is unchanged"
    );
}

// SE-GUI-22 (the Team menu's part): under the lock the aesthetics items
// are disabled with the ownership line as hover, Ctrl+F is a no-op, and
// the gameplay items still run.
#[test]
fn the_locked_team_menu_disables_the_aesthetics_items_with_the_ownership_line() {
    let path = fixture(PesVersion::Pes21, "team_menu_locked");
    // Two players' visual flags set, so a guardless Ctrl+F would make a
    // visible change (the fixture's are clear, which let the guard go).
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    for id in [70101u32, 70102] {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = open_team_menu(&mut window, &mut view);

    for label in [
        "Set boots and gloves IDs for everyone\u{2026}",
        "Clear visual flags",
    ] {
        let item = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node
                        .label()
                        .is_some_and(|node_label| node_label.starts_with(label))
            })
            .unwrap_or_else(|| panic!("no menu item {label:?}"));
        assert!(item.is_disabled(), "{label} is disabled under the lock");
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(item), "The team's export"),
            Some(
                "The team's export owns the aesthetics, so edit its settings.toml and recompile."
                    .to_string(),
            ),
            "{label}'s hover is the ownership line"
        );
    }
    for label in ["Set edited flag", "Set player names to positions"] {
        assert!(
            !node_labeled(&nodes, Role::Button, label).is_disabled(),
            "{label} stays enabled"
        );
    }

    // Ctrl+F under the lock: nothing changes, the flags still set.
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "Ctrl+F changed nothing"
    );
    for id in [70101u32, 70102] {
        let flags = &player_entry(&view, id).edit_flags;
        assert!(
            flags.face && flags.hair && flags.physique && flags.strip,
            "{id}'s visual flags survived the locked Ctrl+F"
        );
    }

    // Set edited flag still runs.
    let nodes = open_team_menu(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set edited flag"),
        &mut view,
    );
    for &(_, id) in &rostered(&view, 701) {
        assert!(player_entry(&view, id).edit_flags.player, "{id} flagged");
    }
}

// The menu item itself (the other SE-GUI-23 test runs the clear through
// Ctrl+F). Every filled slot's four visual flags set first, then cleared,
// one undo restoring every one of them, redo clearing them again, and the
// saved file holding them clear.
// SE-GUI-23
#[test]
fn the_clear_visual_flags_item_clears_every_slot_with_undo_redo_and_save() {
    let path = fixture(PesVersion::Pes21, "team_menu_clear_flags");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let filled: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for &id in &filled {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let cleared = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        !(flags.face || flags.hair || flags.physique || flags.strip)
    };
    let all_set = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        flags.face && flags.hair && flags.physique && flags.strip
    };

    let nodes = open_team_menu(&mut window, &mut view);
    let item = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Clear visual flags"))
        })
        .expect("the Clear visual flags item");
    assert!(!item.is_disabled(), "unlocked, the item is enabled");
    click(&mut window, item, &mut view);
    let slots = rostered(&view, 701);
    for &(_, id) in &slots {
        assert!(cleared(&view, id), "{id}'s visual flags cleared");
    }

    // One undo entry restores them; redo clears them again.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    for &id in &filled {
        assert!(all_set(&view, id), "{id}'s four flags restored");
    }
    window.frame(vec![key(egui::Key::Y, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &slots {
        assert!(cleared(&view, id), "{id}'s flags cleared again");
    }

    // Save; the reopened file holds them cleared.
    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let reopened = EditFile::load(&path).unwrap();
    for &(_, id) in &slots {
        let flags = &reopened.player(id).unwrap().edit_flags;
        assert!(
            !(flags.face || flags.hair || flags.physique || flags.strip),
            "{id}'s flags cleared in the saved file"
        );
    }
}

// Each Team menu operation acts on the selected row's team, a team row's
// included — Clear visual flags by the menu item and by Ctrl+F, each in
// turn from the same state; the menu is disabled only with no row
// selected.
#[test]
fn the_team_menu_and_ctrl_f_act_on_the_selected_rows_team() {
    let path = fixture(PesVersion::Pes21, "team_menu_on_team_row");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let filled: Vec<u32> = file
        .team(701)
        .unwrap()
        .roster
        .iter()
        .map(|slot| slot.player_id)
        .filter(|&id| id != 0)
        .collect();
    for &id in &filled {
        let flags = &mut file.player_mut(id).unwrap().edit_flags;
        flags.face = true;
        flags.hair = true;
        flags.physique = true;
        flags.strip = true;
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    view.edit_aesthetics = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let cleared = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        !(flags.face || flags.hair || flags.physique || flags.strip)
    };
    let all_set = |view: &SaveView, id: u32| {
        let flags = &player_entry(view, id).edit_flags;
        flags.face && flags.hair && flags.physique && flags.strip
    };

    // The team row selected: the menu item is enabled and clears.
    open_team_view(&mut window, &mut view, 701);
    let nodes = open_team_menu(&mut window, &mut view);
    let item = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Clear visual flags"))
        })
        .expect("the Clear visual flags item");
    assert!(!item.is_disabled(), "the item is enabled on a team row");
    click(&mut window, item, &mut view);
    for &(_, id) in &rostered(&view, 701) {
        assert!(cleared(&view, id), "{id}'s visual flags cleared");
    }

    // Ctrl+Z restores them; from the same state Ctrl+F clears the team's
    // flags as it does from a player row.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &id in &filled {
        assert!(all_set(&view, id), "{id}'s flags restored");
    }
    window.frame(vec![key(egui::Key::F, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    for &(_, id) in &rostered(&view, 701) {
        assert!(cleared(&view, id), "{id}'s visual flags cleared by Ctrl+F");
    }

    // With no row selected the menu is disabled.
    view.selected = None;
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        menu_button(&nodes, "Team").is_disabled(),
        "the Team menu is disabled with no row selected"
    );
}
