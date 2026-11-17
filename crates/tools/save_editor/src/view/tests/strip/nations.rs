//! The Nation combo: the open save's version's own nations, each
//! `ID - NAME` ordered by ID, the held entry shown while closed and
//! scrolled into view on open.

use super::*;

/// `version`'s fixture save written to a scratch file with `id` as the
/// selected player's nationality; the view open on him.
fn open_with_nation(version: PesVersion, test: &str, id: u16) -> (SaveView, u32) {
    let path = fixture(version, test);
    let mut file = EditFile::from_bytes(&fixture_save(version)).unwrap();
    let team = file.teams()[0].id;
    let player_id = file.teams()[0].roster[0].player_id;
    file.player_mut(player_id).unwrap().basic.nationality = id;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(team, 0));
    (view, player_id)
}

/// The Nation combo's node.
fn nation_combo(nodes: &[accesskit::Node]) -> &accesskit::Node {
    right_of_label(nodes, "Nation", Role::ComboBox)
        .first()
        .copied()
        .expect("the Nation combo")
}

/// The node of the combo list's row reading `text`.
fn list_row(pairs: &[(accesskit::NodeId, accesskit::Node)], text: &str) -> accesskit::NodeId {
    pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(text))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("the {text:?} row"))
}

// The closed combo reads the held nation as the version's own list
// spells it — `ID - NAME`, the game's own spelling, USA against
// UNITED STATES — and an ID the version lacks shows `ID - ?`. It
// spans the rows' fields' edges, as the counted fields' do.
#[test]
fn the_nation_combo_shows_the_held_entry_on_the_fields_span() {
    for (version, id, shown) in [
        (PesVersion::Pes21, 135, "135 - USA"),
        (PesVersion::Pes17, 135, "135 - UNITED STATES"),
        (PesVersion::Pes21, 100, "100 - ?"),
    ] {
        let (mut view, _) = open_with_nation(version, &format!("nation_{version:?}_{id}"), id);
        let mut window = Headless::new();
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let combo = nation_combo(&nodes);
        assert_eq!(combo.value(), Some(shown), "{version:?} holds {id}");
        let name = name_field(&nodes).bounds().expect("bounds");
        let bounds = combo.bounds().expect("bounds");
        assert!(
            (bounds.x0 - name.x0).abs() <= 0.5 && (bounds.x1 - name.x1).abs() <= 0.5,
            "the Nation combo on the Name field's span ({bounds:?} vs {name:?})"
        );
    }
}

// A pick in the open list stores its nation as one undo entry. The
// list opens showing the held row — there are over 200 entries — and
// only then: a wheel scroll after leaves it where the member put it.
#[test]
fn a_pick_in_the_nation_list_stores_it_and_the_held_row_shows_on_open() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_pick", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nation = |view: &SaveView| player_entry(view, player_id).basic.nationality;
    assert_eq!(nation(&view), 135, "premise: 135 held");

    // The closed combo's wheel steps through the entries, as the
    // Registered position combo's does: one notch down takes 136.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(nation_combo(&nodes));
    notch(&mut window, &mut view, at, false);
    assert_eq!(nation(&view), 136, "a notch down takes the next nation");
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation(&view), 135, "the notch undone");

    // Open the list and pick 231 - SAN MARINO through AccessKit (its
    // row sits past the popup's clip a pointer click reaches).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, nation_combo(&nodes), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    action_click(&mut window, &mut view, list_row(&pairs, "231 - SAN MARINO"));
    assert_eq!(nation(&view), 231, "the pick stored 231");
    // The AccessKit click leaves the popup open (egui closes it on a
    // pointer click or Escape alone): Escape closes it.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );

    // Reopen: the held row lies inside the list's clip — the open
    // scrolls it into view, not the list's top. The row settles once
    // its bounds hold for a frame (the open's scroll animates).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, nation_combo(&nodes), &mut view);
    let (mut settled, mut last) = (None, None);
    for _ in 0..48 {
        let (_, pairs, shapes) =
            window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        // The list's clip: the tightest clip a nation row's text is
        // clipped by (the closed combo's own text is clipped by the
        // pane).
        let Some(clip) = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text.contains(" - ") => {
                    Some(clipped.clip_rect)
                }
                _ => None,
            })
            .min_by(|a, b| a.area().total_cmp(&b.area()))
        else {
            continue;
        };
        // A row past the clip reports no bounds: skip the frame.
        let Some(row) = pairs
            .iter()
            .find(|(_, node)| {
                node.role() == Role::Button && node.label() == Some("231 - SAN MARINO")
            })
            .and_then(|(_, node)| node.bounds())
        else {
            continue;
        };
        if last == Some(row) {
            assert!(
                row.y0 >= f64::from(clip.min.y) - 0.5 && row.y1 <= f64::from(clip.max.y) + 0.5,
                "the held row inside the list's clip ({row:?} vs {clip:?})"
            );
            settled = Some(row);
            break;
        }
        last = Some(row);
    }
    let settled = settled.expect("the held row never settled inside the list's clip");
    // A wheel up over the list scrolls it: the row moves and is not
    // pinned back, the scroll into view being the open frame's alone.
    let at = egui::pos2(
        ((settled.x0 + settled.x1) / 2.0) as f32,
        ((settled.y0 + settled.y1) / 2.0) as f32,
    );
    notch(&mut window, &mut view, at, true);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let moved = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("231 - SAN MARINO"))
        .and_then(|(_, node)| node.bounds())
        .expect("the SAN MARINO row's bounds");
    assert_ne!(moved, settled, "the row moved: the scroll is not re-pinned");
    // One Ctrl+Z restores 135: the pick was one undo entry.
    let nodes = draw(&mut window, &mut view);
    click(&mut window, nation_combo(&nodes), &mut view);
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation(&view), 135, "one undo restores 135");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the pick was one undo entry"
    );
}
