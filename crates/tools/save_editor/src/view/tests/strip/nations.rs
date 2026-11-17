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

/// `player_id`'s nationality as the session holds it.
fn nation_of(view: &SaveView, player_id: u32) -> u16 {
    player_entry(view, player_id).basic.nationality
}

/// The Nation combo's node.
fn nation_combo(nodes: &[accesskit::Node]) -> &accesskit::Node {
    right_of_label(nodes, "Nation", Role::ComboBox)
        .first()
        .copied()
        .expect("the Nation combo")
}

/// The node of the combo list's row reading `text`.
fn combo_row(pairs: &[(accesskit::NodeId, accesskit::Node)], text: &str) -> accesskit::NodeId {
    pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(text))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("the {text:?} row"))
}

/// The combo's list opened and one frame to draw it.
fn open_nation_list(window: &mut Headless, view: &mut SaveView) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    click(window, nation_combo(&nodes), view);
    window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
}

/// The popup's clip this frame: the tightest clip a `" - "` text is
/// clipped by. The match is wider than the popup's rows — the Registered
/// position combo's `CB - Centre Back` reads the same — but anything
/// outside the popup is pane-clipped, so the tightest is the popup's.
fn popup_clip(shapes: &[egui::epaint::ClippedShape]) -> Option<egui::Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text.contains(" - ") => {
                Some(clipped.clip_rect)
            }
            _ => None,
        })
        .min_by(|a, b| a.area().total_cmp(&b.area()))
}

/// `text`'s row bounds when they lie inside the list's clip this frame,
/// `None` while the row is off it or undrawn.
fn row_inside_clip(
    window: &mut Headless,
    view: &mut SaveView,
    text: &str,
) -> Option<accesskit::Rect> {
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let clip = popup_clip(&shapes)?;
    // A row past the clip reports no bounds.
    let row = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(text))
        .and_then(|(_, node)| node.bounds())?;
    (row.y0 >= f64::from(clip.min.y) - 0.5 && row.y1 <= f64::from(clip.max.y) + 0.5).then_some(row)
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
// list opens scrolled to the held row — there are over 200 entries —
// and a wheel scroll after leaves it where the member put it.
#[test]
fn a_pick_in_the_nation_list_stores_it_and_the_held_row_shows_on_open() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_pick", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    // The closed combo's wheel steps through the entries, as the
    // Registered position combo's does: one notch down takes 136.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(nation_combo(&nodes));
    notch(&mut window, &mut view, at, false);
    assert_eq!(
        nation_of(&view, player_id),
        136,
        "a notch down takes the next nation"
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation_of(&view, player_id), 135, "the notch undone");

    // Open the list and pick 231 - SAN MARINO through AccessKit (its
    // row sits past the popup's clip a pointer click reaches).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, nation_combo(&nodes), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    action_click(
        &mut window,
        &mut view,
        combo_row(&pairs, "231 - SAN MARINO"),
    );
    assert_eq!(nation_of(&view, player_id), 231, "the pick stored 231");
    // The AccessKit click leaves the popup open (egui closes it on a
    // pointer click or Escape alone): Escape closes it.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );

    // Reopen: the held row lies inside the list's clip — the open
    // scrolls it into view, not the list's top. The bounds hold still
    // only once the open's `scroll_to_me` has landed on a pass, so the
    // row settles once they hold for a frame.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, nation_combo(&nodes), &mut view);
    let (mut settled, mut last) = (None, None);
    for _ in 0..48 {
        let (_, pairs, shapes) =
            window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let Some(clip) = popup_clip(&shapes) else {
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
    assert_eq!(nation_of(&view, player_id), 135, "one undo restores 135");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the pick was one undo entry"
    );
}

// Digits typed in the open list build the ID as typed: 2, 3 and 1 land
// on 231 - SAN MARINO, each pick under the combo's key, so the three of
// them are one undo entry and one Ctrl+Z puts 135 back.
#[test]
fn typed_digits_pick_a_nation_as_one_undo_entry() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_digits", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    open_nation_list(&mut window, &mut view);
    // A digit a frame, each well inside the run's second (a frame's
    // clock step is 1/60 s).
    for digit in ["2", "3", "1"] {
        window.frame(vec![egui::Event::Text(digit.to_owned())], |ui| {
            show(ui, &mut view, &|| Vec::new())
        });
    }
    assert_eq!(
        nation_of(&view, player_id),
        231,
        "the typed 231 picked SAN MARINO"
    );
    // The pick scrolled into view: its row lies inside the list's clip
    // once its bounds hold for a frame (the `scroll_to_me` lands on a
    // later pass).
    let (mut last, mut inside) = (None, None);
    for _ in 0..48 {
        let row = row_inside_clip(&mut window, &mut view, "231 - SAN MARINO");
        if row.is_some() && row == last {
            inside = row;
            break;
        }
        last = row;
    }
    inside.expect("the picked row never settled inside the list's clip");

    // Escape closes the list; one Ctrl+Z restores 135, nothing left.
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation_of(&view, player_id), 135, "one undo restores 135");
    assert!(
        !view.session.as_ref().unwrap().can_undo(),
        "the typed picks were one undo entry"
    );
}

// A second's pause between digits starts a new ID: after the typed 2
// picked BAHRAIN, a 1 landing once the run's second has passed picks
// 1 - AFGHANISTAN, not a continuation to 21.
#[test]
fn a_seconds_pause_between_digits_starts_a_new_id() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_pause", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    open_nation_list(&mut window, &mut view);
    window.frame(vec![egui::Event::Text("2".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation_of(&view, player_id), 2, "the typed 2 picked BAHRAIN");
    // 61 frames with no input move egui's clock past the run's second.
    idle_frames(&mut window, &mut view, 61);
    window.frame(vec![egui::Event::Text("1".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        1,
        "the 1 after the pause started a new ID, not 21"
    );
}

// A digit is a pick only while the list is open and only a digit: a
// typed 5 with it closed changes nothing, and a non-digit in it neither
// picks nor joins the run — the 2 after it picks 2 on its own.
#[test]
fn a_digit_picks_only_in_the_open_list_and_only_a_digit() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_non_digit", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        135,
        "a digit with the list closed is no pick"
    );
    open_nation_list(&mut window, &mut view);
    window.frame(vec![egui::Event::Text("x".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        135,
        "a non-digit in the open list is no pick"
    );
    window.frame(vec![egui::Event::Text("2".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        2,
        "the x joined no run: the 2 picks BAHRAIN on its own"
    );
}

// A held ID the version lacks draws as `ID - ?`, first and clickable,
// but it is no typed run's candidate: the run searches the version's
// own list alone. Held 100 on PES 2021 (whose list lacks it), a typed
// 1 lands on 1 - AFGHANISTAN, never on the `100 - ?` row the 1 prefixes
// first.
#[test]
fn a_typed_run_searches_the_versions_list_not_the_unlisted_held() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_typed_unlisted", 100);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 100, "premise: 100 held");

    open_nation_list(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The `?` row is drawn first: the topmost of the list's rows.
    let top = pairs
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| label.contains(" - "))
                && node.bounds().is_some()
        })
        .min_by(|(_, a), (_, b)| {
            let y = |node: &accesskit::Node| node.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .and_then(|(_, node)| node.label());
    assert_eq!(top, Some("100 - ?"), "the unlisted held's `?` row is first");

    window.frame(vec![egui::Event::Text("1".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        1,
        "the typed 1 picked AFGHANISTAN, not the `?` row's no-op 100"
    );
}

// A typed pick sets the next frame's one selected row before the rows
// draw: on the frame a run lands, its row alone reads selected (the
// held row it leaves is not still marked beside it — AccessKit's
// toggled state on the row buttons).
#[test]
fn a_typed_picks_frame_marks_its_row_alone_selected() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_typed_marked", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    open_nation_list(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(vec![egui::Event::Text("210".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let selected: Vec<Option<&str>> = pairs
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| label.contains(" - "))
                && node.toggled() == Some(accesskit::Toggled::True)
        })
        .map(|(_, node)| node.label())
        .collect();
    assert_eq!(
        selected,
        [Some("210 - GERMANY")],
        "the picked row reads selected, and it alone"
    );
    assert_eq!(nation_of(&view, player_id), 210, "the typed 210 stored");
}

// A run that prefixes no ID picks nothing, and the run it builds stays
// to prefix the next digit's: the 0 lands on no nation, and the 1
// inside the run's second makes `01`, still no ID's prefix — not a new
// run's 1.
#[test]
fn a_run_that_prefixes_no_id_picks_nothing() {
    let (mut view, player_id) = open_with_nation(PesVersion::Pes21, "nation_bad_run", 135);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    assert_eq!(nation_of(&view, player_id), 135, "premise: 135 held");

    open_nation_list(&mut window, &mut view);
    window.frame(vec![egui::Event::Text("0".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(nation_of(&view, player_id), 135, "0 prefixes no ID");
    window.frame(vec![egui::Event::Text("1".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        nation_of(&view, player_id),
        135,
        "the run 01 prefixes no ID either"
    );
}
