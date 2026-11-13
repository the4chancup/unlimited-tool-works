use super::*;

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
    let incremental = node_labeled(nodes, Role::RadioButton, "Set values starting from:");
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

// The range's end reads as an enabled label: it takes no click or focus,
// but a disabled report would read a working readout as greyed out to a
// screen reader (its `WidgetInfo` is hand-set).
#[test]
fn the_ranges_end_reports_an_enabled_label() {
    let (_view, _window, nodes) = open_boglo("boglo_readout_enabled");
    let end = boglo_fields(&nodes).end;
    assert_widget_named(end, Role::Label, end.value().expect("the readout's value"));
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

// The option column is as wide as the widest option: its text, the radio's
// icon and the gap after the icon. The fields' column starts the 12-point
// column gap past it.
#[test]
fn the_boglo_option_column_is_as_wide_as_its_widest_option() {
    let (_, window, nodes) = open_boglo("boglo_option_column");
    let style = window.ctx.global_style();
    let button = egui::TextStyle::Button.resolve(&style);
    let widest = [
        "Unchanged",
        "Set values starting from:",
        "Set all to the same ID:",
        "Set like boot IDs",
    ]
    .iter()
    .map(|text| {
        window.ctx.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap((*text).to_owned(), button.clone(), egui::Color32::WHITE)
                .size()
                .x
        })
    })
    .fold(0.0, f32::max);
    let option = widest + style.spacing.icon_width + style.spacing.icon_spacing;
    let incremental = node_labeled(&nodes, Role::RadioButton, "Set values starting from:")
        .bounds()
        .expect("the radio's bounds");
    let start = boglo_fields(&nodes)
        .start
        .bounds()
        .expect("the start box's bounds");
    let column = start.x0 - incremental.x0;
    assert!(
        (column - f64::from(option + 12.0)).abs() <= 0.5,
        "the start box stands {column} right of the radio's left, the widest option's {option} \
         and the 12-point gap"
    );
}

// The dialog is as wide as its contents (the draft's `.window:has(.boglo)`):
// no wider than the groups, whose right edge stands the group frame's
// margin and stroke past the range's end box, or the title if it is wider,
// plus the modal frame's margin and stroke.
#[test]
fn the_boglo_dialog_is_as_wide_as_its_groups() {
    let (_, window, nodes) = open_boglo("boglo_width");
    // The dialog's node carries no bounds, so its width is its area's.
    let outer = window
        .ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_boglo")))
        .expect("the dialog's area has a rect");
    let fields = boglo_fields(&nodes);
    let title = nodes
        .iter()
        .find(|node| node.value() == Some("Set boots and gloves IDs for everyone"))
        .and_then(accesskit::Node::bounds)
        .expect("the title's bounds");
    let style = window.ctx.global_style();
    let groups_right = fields.end.bounds().expect("the end box's bounds").x1 as f32
        + egui::Frame::group(&style).total_margin().right;
    let contents_right = groups_right.max(title.x1 as f32);
    let expected = contents_right
        + studio_core::widgets::modal_frame(&style)
            .total_margin()
            .right;
    assert!(
        outer.right() <= expected + 1.0,
        "the dialog's right edge {} stands past its contents' {contents_right} plus the frame's \
         margin and stroke ({expected})",
        outer.right()
    );
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

/// PES 17 under ALL with the first team's last player selected, one
/// height edit on him made, and the boots and gloves dialog opened on
/// that team: the view, the window, the team's id, the next team's id
/// and the edited player's id.
fn boglo_on_a_teams_last_player(test: &str) -> (SaveView, Headless, u32, u32, u32) {
    let path = fixture(PesVersion::Pes17, test);
    let mut view = SaveView::new();
    view.open(&path);
    assert_eq!(view.team, TeamChoice::All, "premise: the list shows ALL");
    let rows = selection::rows(view.session.as_ref().unwrap().file(), &TeamChoice::All, "");
    let team = rows[0].team_id;
    let last = rows
        .iter()
        .rposition(|row| row.team_id == team)
        .expect("the team's rows");
    let (last, next) = (rows[last], rows[last + 1]);
    assert_ne!(
        next.team_id, team,
        "premise: the next player row is another team's"
    );
    view.selected = Some(Selection::Player(team, last.slot));
    let session = view.session.as_mut().unwrap();
    edit_player(session, last.player_id, 1, |player| {
        player.basic.height += 1
    });
    assert!(session.can_undo(), "premise: the edit is on the undo stack");
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
    assert!(
        label_value(&nodes, "Boot IDs"),
        "premise: the dialog is open"
    );
    (view, window, team, next.team_id, last.player_id)
}

// The dialog holds the page keys: PageDown under it moves no selection,
// so its Set still writes the team it was opened on, not the next one.
#[test]
fn page_down_under_the_boglo_dialog_keeps_its_team() {
    let (mut view, mut window, team, next, _) = boglo_on_a_teams_last_player("boglo_page_down");
    let selected = view.selected;
    window.frame(
        vec![key(egui::Key::PageDown, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.selected, selected,
        "PageDown under the dialog moved nothing"
    );

    let nodes = draw(&mut window, &mut view);
    let same = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::RadioButton && node.label() == Some("Set all to the same ID:")
        })
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("the boots' one-ID radio");
    click(&mut window, same, &mut view);
    let nodes = draw(&mut window, &mut view);
    let boots = boglo_fields(&nodes).boots;
    let nodes = type_into(&mut window, &mut view, boots, "4321");
    let roster_boots = |view: &SaveView, team_id: u32| -> Vec<u32> {
        team_entry(view, team_id)
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| player_entry(view, slot.player_id).appearance.boots_id)
            .collect()
    };
    for team_id in [team, next] {
        assert!(
            !roster_boots(&view, team_id).iter().all(|&id| id == 4321),
            "premise: team {team_id} does not already hold the ID"
        );
    }
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Set"),
        &mut view,
    );
    assert!(
        roster_boots(&view, team).iter().all(|&id| id == 4321),
        "Set wrote the dialog's team"
    );
    assert!(
        !roster_boots(&view, next).contains(&4321),
        "Set left the next team alone"
    );
}

// The dialog holds Ctrl+Z: the edit made before it opened stands.
#[test]
fn ctrl_z_under_the_boglo_dialog_undoes_nothing() {
    let (mut view, mut window, _, _, player) = boglo_on_a_teams_last_player("boglo_ctrl_z");
    let edited = player_entry(&view, player).basic.height;
    assert!(
        !window.ctx.egui_wants_keyboard_input(),
        "premise: no field holds the keyboard"
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        player_entry(&view, player).basic.height,
        edited,
        "the edit stands under the dialog's Ctrl+Z"
    );
    assert!(view.boglo.is_some(), "the dialog is still open");
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
        node_labeled(&nodes, Role::RadioButton, "Set values starting from:"),
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
        node_labeled(&nodes, Role::RadioButton, "Set values starting from:"),
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
