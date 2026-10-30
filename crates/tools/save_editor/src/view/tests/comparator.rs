//! The comparator panel's tests (SE-GUI-13): the pick and its refusals, the
//! rows in the views' terms, the scope toggles and the team filter, the
//! only-in heads, the click's selection, the `Comparing…` mark and
//! Export.

use std::path::PathBuf;

use super::*;

use crate::test_support::{TEST_SALT, cli_stdout};

/// The PES 17 fixture with `edit` applied, written under `dir` as `name`:
/// the picked save the dialog stub returns.
fn picked_save(dir: &Path, name: &str, edit: impl FnOnce(&mut EditFile)) -> PathBuf {
    let mut file =
        EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).expect("the fixture loads");
    edit(&mut file);
    let path = dir.join(name);
    std::fs::write(&path, file.to_bytes(&TEST_SALT).expect("the save writes")).unwrap();
    path
}

/// The test's scratch folder: the one the open save was written to.
fn scratch_of(view: &SaveView) -> PathBuf {
    view.session
        .as_ref()
        .expect("an open save")
        .path()
        .parent()
        .expect("the scratch dir")
        .to_path_buf()
}

/// The comparator opened from the toolbar's Savefile menu: the
/// "Comparator…" item clicked, the next frame's node pairs returned.
fn open_comparator(
    window: &mut Headless,
    view: &mut SaveView,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    let nodes = draw(window, view);
    click(
        window,
        node_labeled(&nodes, Role::Button, &menu_label("Savefile")),
        view,
    );
    let nodes = draw(window, view);
    click(
        window,
        node_labeled(&nodes, Role::Button, "Comparator\u{2026}"),
        view,
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    pairs
}

/// `path` stubbed as the Pick dialog's choice; the "Pick…" button clicked.
fn pick(window: &mut Headless, view: &mut SaveView, path: PathBuf) {
    view.open_save_dialog = Box::new(move || Some(path.clone()));
    let nodes = draw(window, view);
    click(
        window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        view,
    );
}

/// `name` picked in the comparator's Team combo: the combo clicked, its
/// popup's `name` item picked through the AccessKit action — a popup
/// item's bounds sit outside the window, where a pointer click lands on
/// nothing.
fn pick_comparator_team(window: &mut Headless, view: &mut SaveView, name: &str) {
    // A pick's popup can still be open on the next call's first frame
    // (its close lands a frame after the item's action): click the combo
    // only until `name`'s item is drawn — clicking while it is open
    // closes it. Bounded, so a missing item fails rather than hangs.
    for _ in 0..10 {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        if let Some((item, _)) = pairs
            .iter()
            .find(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
        {
            action_click(window, view, *item);
            return;
        }
        let combo = comparator_nodes(&pairs)
            .into_iter()
            .find(|node| node.role() == Role::ComboBox)
            .expect("the Team combo");
        click(window, combo, view);
    }
    panic!("the Team combo never showed {name:?}")
}

/// Frames until the comparison lands: `pending` empty and the result keyed
/// to the session's current revision, the picked session's and the scope
/// (bounded, so a worker that never replies fails rather than hangs).
/// Returns the landing frame's node pairs.
fn compared(
    window: &mut Headless,
    view: &mut SaveView,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    frames_until(
        window,
        view,
        |view| {
            let session = view.session.as_ref().expect("an open save");
            let picked = view
                .comparator
                .picked
                .as_ref()
                .map(|(_, picked)| picked.revision());
            view.comparator.comparison.pending.is_none()
                && view
                    .comparator
                    .comparison
                    .result
                    .as_ref()
                    .is_some_and(|(key, _)| {
                        key.revision == session.revision()
                            && Some(key.picked) == picked
                            && key.scope == view.comparator.scope
                    })
        },
        "the comparison never returned",
    )
}

/// The comparator window's nodes out of `pairs` (`frame_ids`' output).
fn comparator_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    nodes_in_window(pairs, "Comparator")
        .into_iter()
        .map(|(_, node)| node)
        .collect()
}

/// The `(NodeId, node)` pair of the Button labeled `label` inside the
/// comparator window (the player list's buttons share the screen); each
/// caller takes the half it needs.
fn comparator_button<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    label: &str,
) -> (accesskit::NodeId, &'a accesskit::Node) {
    nodes_in_window(pairs, "Comparator")
        .into_iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no window button {label:?}"))
}

/// A PES 21 fixture written under `dir`, for a pick of another version.
fn picked_pes21(dir: &Path) -> PathBuf {
    let path = dir.join("picked21");
    std::fs::write(&path, fixture_save(PesVersion::Pes21)).unwrap();
    path
}

/// Team 734's fixture save open with 73401's height edited to 183, the
/// unedited fixture picked, the comparator open and its result landed:
/// the pair the row tests build on.
fn compared_73401(
    test: &str,
) -> (
    SaveView,
    Headless,
    Vec<(accesskit::NodeId, accesskit::Node)>,
) {
    let (mut view, mut window) = open_734_with(test, &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let pairs = compared(&mut window, &mut view);
    (view, window, pairs)
}

/// Wheels over the comparator's list until a node holding `text` (a
/// Label's value or a Button's label) is drawn — a frame draws only the
/// rows in view, so a row deep in the list is scrolled to — and returns
/// the pairs of a frame where it held still (the wheel's offset target
/// keeps animating past the found frame). Bounded: a row that never
/// shows fails rather than hangs.
fn scroll_to(
    window: &mut Headless,
    view: &mut SaveView,
    text: &str,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    // The window's centre, where the list is.
    let at = egui::pos2(400.0, 400.0);
    let mut bounds = None;
    for _ in 0..600 {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let nodes = comparator_nodes(&pairs);
        let node = nodes
            .iter()
            .find(|node| node.value() == Some(text) || node.label() == Some(text));
        if let Some(node) = node {
            let next = node.bounds();
            if next.is_some() && next == bounds {
                return pairs;
            }
            bounds = next;
            continue;
        }
        bounds = None;
        // Not in view: one notch down; the next settle frame reads where
        // its animation lands.
        window.frame_ids(
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -300.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, view, &|| Vec::new()),
        );
    }
    panic!("the list never showed {text:?}")
}

// SE-GUI-13
#[test]
fn the_panel_lists_the_differences_per_team_and_a_rows_click_selects() {
    let (mut view, mut window) = open_734_with("comparator_rows", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    let pairs = open_comparator(&mut window, &mut view);
    assert!(
        pairs
            .iter()
            .any(|(_, node)| node.label() == Some("Comparator")),
        "the window's title"
    );
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Player 73401  49 Muh Spooks");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Team 734 /lit/")),
        "734's heading"
    );
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label() == Some("Player 73401  49 Muh Spooks")
        }),
        "73401's head"
    );
    let (_, row) = comparator_button(&pairs, "Height 185 \u{2192} 183");

    // The selector on another team: the click selects the player and
    // moves the selector to his, as the AATF panel's click does.
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    click(&mut window, row, &mut view);
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "73401's team and slot"
    );
    assert_eq!(view.team, TeamChoice::Team(734), "the selector moved");
}

// SE-GUI-13
#[test]
fn a_motion_row_reads_as_the_view_shows_it() {
    let (mut view, mut window) = open_734_with("comparator_motion", &[], 183);
    let dir = scratch_of(&view);
    // The open save's 73401 free-kick motion stored 14 (the picked's
    // stored 0): the view shows one above the stored.
    let mut entry = player_entry(&view, 73401);
    assert_eq!(entry.motion.free_kick, 0, "premise: stored 0");
    entry.motion.free_kick = 14;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Free kick 1 \u{2192} 15");
    comparator_button(&pairs, "Free kick 1 \u{2192} 15");
}

// SE-GUI-13
#[test]
fn the_scope_toggles_narrow_the_rows() {
    let (mut view, mut window) = open_734_with("comparator_scopes", &[73401], 183);
    let dir = scratch_of(&view);
    // A gameplay row (height, through `open_734_with`) and an aesthetics
    // one (73401's boots id, 0 in the fixture).
    let mut entry = player_entry(&view, 73401);
    assert_eq!(entry.appearance.boots_id, 0, "premise: no boots");
    entry.appearance.boots_id = 3601;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Boots ID 0 \u{2192} 3601");
    comparator_button(&pairs, "Boots ID 0 \u{2192} 3601");
    comparator_button(&pairs, "Height 185 \u{2192} 183");

    let has = |pairs: &[(accesskit::NodeId, accesskit::Node)], label: &str| {
        comparator_nodes(pairs)
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some(label))
    };
    // Gameplay: the height alone; Aesthetics: the boots alone. `compared`
    // waits for the new scope's result before the scroll, or the held
    // previous result's rows would read as the new scope's.
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Gameplay"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Height 185 \u{2192} 183");
    assert!(has(&pairs, "Height 185 \u{2192} 183"), "the gameplay row");
    assert!(
        !has(&pairs, "Boots ID 0 \u{2192} 3601"),
        "the aesthetics row narrowed out"
    );
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Aesthetics"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Boots ID 0 \u{2192} 3601");
    assert!(
        has(&pairs, "Boots ID 0 \u{2192} 3601"),
        "the aesthetics row"
    );
    assert!(
        !has(&pairs, "Height 185 \u{2192} 183"),
        "the gameplay row narrowed out"
    );
}

/// An edit after the first result marks the held list `Comparing…` and
/// starts no run until the gesture ends — the mark read on a held-pointer
/// frame, where no reply can land (`an_edit_rechecks_and_the_old_result_marks_checking`'s
/// premise).
#[test]
fn an_edit_marks_the_held_list_comparing() {
    let (mut view, mut window, _) = compared_73401("comparator_mark");
    scroll_to(&mut window, &mut view, "Height 185 \u{2192} 183");

    // The pointer held on the toolbar's empty end so the re-run waits for
    // the release: a started run could reply before the mark is read.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    // 73401 back to his fixture height: the saves are identical again.
    let mut entry = player_entry(&view, 73401);
    entry.basic.height = 185;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.comparator.comparison.pending.is_none(),
        "premise: no run started while the pointer is held"
    );
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Comparing\u{2026}")),
        "the edit marks the held result"
    );
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label() == Some("Height 185 \u{2192} 183")
        }),
        "the stale row stays while comparing"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        view.comparator.comparison.pending.is_some(),
        "the run starts on the release frame"
    );
    let pairs = compared(&mut window, &mut view);
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("No differences.")),
        "identical saves read empty"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Height"))
        }),
        "the old row is gone"
    );
}

// SE-GUI-13
#[test]
fn a_save_of_another_version_is_refused_naming_both() {
    let (mut view, mut window) = open_734_with("comparator_version", &[], 183);
    let dir = scratch_of(&view);
    let picked = picked_pes21(&dir);
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2021 save and the open save a PES 2017 one, but both must be of the same version"),
        "the refusal's literal text: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "(none)"),
        "the pick dropped: {lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("Player ") || line.starts_with("Team ")),
        "no rows: {lines:?}"
    );
    assert!(
        view.comparator.comparison.result.is_none() && view.comparator.comparison.pending.is_none(),
        "no comparison ran"
    );
}

// SE-GUI-13
#[test]
fn the_players_only_in_one_save_list_under_their_headings() {
    let (mut view, mut window) = open_734_with("comparator_only_in", &[], 183);
    let dir = scratch_of(&view);
    // As `tests/cli/diff.rs`'s `diff_pair`: one id renumbered out, so
    // 9_999_999 is in the picked save alone and 73410 in the open alone.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73410).expect("73410 is a player").id = 9_999_999;
    });
    let slot = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(734)
        .expect("team 734")
        .roster
        .iter()
        .position(|slot| slot.player_id == 73410)
        .map(|slot| u8::try_from(slot).expect("a roster slot"))
        .expect("premise: 73410 rostered");
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Only in the open save");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Only in the picked save")),
        "the picked-only heading"
    );
    // His record keeps the name; no roster holds him in the picked save.
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Player 9999999 DFW")),
        "the picked-only head"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|label| label.contains("9999999"))
        }),
        "the picked-only head is no click target"
    );
    // The open-only head clicks as a row does (an AccessKit Click: the
    // scroll's animation keeps its bounds moving past the found frame, so
    // a pointer position can already be stale).
    view.selected = Some(Selection::Player(734, 0));
    action_click(
        &mut window,
        &mut view,
        comparator_button(&pairs, "Player 73410  46 DFW").0,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, slot)),
        "73410 selected"
    );
}

// SE-GUI-13
#[test]
fn the_rows_group_per_team_in_the_open_saves_order() {
    let dir = crate::test_support::scratch("comparator_groups");
    // 73410's place on team 734's roster given to 74301 (so 74301 is on
    // 734 and on 743, and 73410 on no roster), and a height each on
    // 73401, 73410, 74301 and 74302. The report's order is 73401, 73410,
    // 74301, 74302 (the fixture's record order).
    let open_path = picked_save(&dir, "open", |file| {
        for id in [73401u32, 73410, 74301, 74302] {
            file.player_mut(id).expect("player").basic.height = 183;
        }
        file.team_mut(734).expect("team 734").roster[9].player_id = 74301;
    });
    let picked = picked_save(&dir, "picked", |_| {});
    let mut view = SaveView::new();
    view.open(&open_path);
    view.panel = Some(Panel::Comparator);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    let at = lines
        .iter()
        .position(|line| line == "Team 734 /lit/")
        .expect("734's heading");
    assert_eq!(
        &lines[at..at + 11],
        [
            "Team 734 /lit/",
            "Player 73401  49 Muh Spooks",
            "Height 185 \u{2192} 183",
            // 74301 is on 734 and on 743: his first team is 734, and his
            // head wears his number on 734's slot.
            "Player 74301  46 NOTHING EVER HAPPENS",
            "Height 189 \u{2192} 183",
            "Team 743 /pol/",
            "Player 74302  50 STOP WATCHING PORN",
            "Height 185 \u{2192} 183",
            "No team",
            "Player 73410 DFW",
            "Height 180 \u{2192} 183",
        ],
        "one group per team in the open save's order: {lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("Team "))
            .count(),
        2,
        "each team headed once"
    );

    // A click on 74301's row (the Height button below his head) selects
    // him on 734; 73410's head and row are not buttons.
    let pairs = scroll_to(&mut window, &mut view, "Player 73410 DFW");
    let nodes = comparator_nodes(&pairs);
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Player 73410 DFW")),
        "the no-team head is a label"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node
                        .label()
                        .is_some_and(|label| label.starts_with("Height "))
            })
            .count(),
        3,
        "the three rostered rows are buttons"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.value() == Some("Height 180 \u{2192} 183"))
            .count(),
        1,
        "73410's row is not clickable"
    );
    view.team = TeamChoice::Team(718);
    view.selected = Some(Selection::Player(718, 0));
    // 74301 alone stands at 189 in the fixture: his row's label is his.
    action_click(
        &mut window,
        &mut view,
        comparator_button(&pairs, "Height 189 \u{2192} 183").0,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 9)),
        "74301 on 734's slot"
    );
    assert_eq!(view.team, TeamChoice::Team(734), "the selector moved");
}

// SE-GUI-13
#[test]
fn a_refused_pick_clears_the_held_result() {
    let (mut view, mut window, _) = compared_73401("comparator_refusal_clears");
    let dir = scratch_of(&view);
    let picked = picked_pes21(&dir);
    pick(&mut window, &mut view, picked);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2021 save and the open save a PES 2017 one, but both must be of the same version"),
        "the refusal's literal: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Team 734 /lit/"),
        "the landed result's rows are gone: {lines:?}"
    );
    assert!(
        view.comparator.comparison.result.is_none() && view.comparator.comparison.pending.is_none(),
        "the comparison reset"
    );
}

// SE-GUI-13
#[test]
fn a_repick_of_the_same_path_recompares() {
    let (mut view, mut window, _) = compared_73401("comparator_repick");
    let dir = scratch_of(&view);
    // The picked file rewritten on disk under the same path (the member
    // changed the reference in the game): a second edit lands.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73404)
            .expect("73404 is a player")
            .basic
            .height = 180;
    });
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let pairs = scroll_to(&mut window, &mut view, "Height 180 \u{2192} 185");
    comparator_button(&pairs, "Height 180 \u{2192} 185");
    comparator_button(&pairs, "Height 185 \u{2192} 183");
}

// SE-GUI-13
#[test]
fn another_save_keeps_the_pick_and_recompares() {
    let (mut view, mut window, _) = compared_73401("comparator_reopen");
    let dir = scratch_of(&view);
    // A second save of the same version, carrying another edit: the pick
    // stays and the result is the new save's (a member compares several
    // saves against one reference).
    let second = picked_save(&dir, "second", |file| {
        file.player_mut(73402)
            .expect("73402 is a player")
            .basic
            .height = 180;
    });
    view.open(&second);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    let picked_path = dir.join("picked").display().to_string();
    assert!(
        lines.iter().any(|line| line == &picked_path),
        "the picked path still reads: {lines:?}"
    );
    let pairs = scroll_to(&mut window, &mut view, "Height 185 \u{2192} 180");
    comparator_button(&pairs, "Height 185 \u{2192} 180");
    assert!(
        !comparator_nodes(&pairs)
            .iter()
            .any(|node| node.label() == Some("Height 185 \u{2192} 183")),
        "the first save's row is gone"
    );

    // A save of another version: the pick is refused naming both, the
    // pick dropped.
    let pes21 = picked_pes21(&dir);
    view.open(&pes21);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "(none)"),
        "the pick dropped: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line
            == "The picked save is a PES 2017 save and the open save a PES 2021 one, but both must be of the same version"),
        "the refusal's literal: {lines:?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("Player ") || line.starts_with("Team ")),
        "no rows: {lines:?}"
    );
}

// SE-GUI-13
#[test]
fn the_team_filter_narrows_the_list_without_recomparing() {
    let (mut view, mut window) = open_734_with("comparator_filter", &[73401], 183);
    let dir = scratch_of(&view);
    // A second team's edit: 74301's height (189 in the fixture) to 183.
    let mut entry = player_entry(&view, 74301);
    assert_eq!(entry.basic.height, 189, "premise: 74301's height");
    entry.basic.height = 183;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/"),
        "premise: 734's heading under All: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Team 743 /pol/"),
        "premise: 743's heading under All: {lines:?}"
    );
    let key = view
        .comparator
        .comparison
        .result
        .as_ref()
        .expect("the comparison landed")
        .0
        .clone();

    pick_comparator_team(&mut window, &mut view, "/lit/");
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::Team(734),
        "734 picked"
    );
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/"),
        "734's heading kept: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Height 185 \u{2192} 183"),
        "73401's row kept: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Team 743 /pol/"),
        "743's heading narrowed out: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Height 189 \u{2192} 183"),
        "74301's row narrowed out: {lines:?}"
    );
    assert!(
        view.comparator.comparison.pending.is_none(),
        "the filter started no comparison"
    );
    assert_eq!(
        view.comparator.comparison.result.as_ref().unwrap().0,
        key,
        "the held result's key is the one before the filter"
    );

    pick_comparator_team(&mut window, &mut view, "ALL");
    assert_eq!(view.comparator.filter_team, TeamChoice::All, "ALL again");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Team 734 /lit/")
            && lines.iter().any(|line| line == "Team 743 /pol/"),
        "both headings under All again: {lines:?}"
    );
    assert!(
        view.comparator.comparison.pending.is_none(),
        "still no comparison run"
    );
}

// SE-GUI-13
#[test]
fn the_team_filter_keeps_an_only_in_section_by_its_roster() {
    let (mut view, mut window) = open_734_with("comparator_only_in_filter", &[], 183);
    let dir = scratch_of(&view);
    // `the_players_only_in_one_save_list_under_their_headings`' pair:
    // 73410 renumbered out of the picked save — 9_999_999 (on no picked
    // roster) is picked-only, 73410 (on 734's open roster) open-only. And
    // 74302 renumbered with his place on 743's picked roster: 9_999_998
    // is picked-only on the picked save's 743, 74302 open-only on the open
    // save's.
    let picked = picked_save(&dir, "picked", |file| {
        file.player_mut(73410).expect("73410 is a player").id = 9_999_999;
        file.player_mut(74302).expect("74302 is a player").id = 9_999_998;
        file.team_mut(743)
            .expect("team 743")
            .roster
            .iter_mut()
            .find(|entry| entry.player_id == 74302)
            .expect("74302 on 743's roster")
            .player_id = 9_999_998;
    });
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Only in the picked save")
            && lines.iter().any(|line| line == "Only in the open save"),
        "premise: both sections under All: {lines:?}"
    );

    pick_comparator_team(&mut window, &mut view, "/lit/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines.iter().any(|line| line == "Only in the open save"),
        "73410's section keeps its heading: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "Player 73410  46 DFW"),
        "73410's head kept: {lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line == "Only in the picked save"),
        "the picked-only section's heading is gone: {lines:?}"
    );

    // On 743, each section keeps its 743 player: the picked-only one by
    // the picked save's roster, the open-only one by the open save's.
    pick_comparator_team(&mut window, &mut view, "/pol/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    // The combo's popup can still be open, its team names among the lines:
    // the sections' headings and heads alone are compared.
    assert_eq!(
        lines
            .iter()
            .map(String::as_str)
            .filter(|line| line.starts_with("Only in ") || line.starts_with("Player "))
            .collect::<Vec<_>>(),
        [
            "Only in the picked save",
            "Player 9999998  50 STOP WATCHING PORN",
            "Only in the open save",
            "Player 74302  50 STOP WATCHING PORN",
        ],
        "743's only-in players under their headings: {lines:?}"
    );

    // A team no line holds.
    pick_comparator_team(&mut window, &mut view, "/pw/");
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        !lines.iter().any(|line| line == "Only in the open save")
            && !lines.iter().any(|line| line == "Only in the picked save"),
        "no section holds a 744 player: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "No differences in this team."),
        "the empty-team read: {lines:?}"
    );
}

// SE-GUI-13
#[test]
fn export_writes_the_commands_text_whatever_the_team_filter() {
    let dir = crate::test_support::scratch("comparator_export_scope");
    // The open save written to disk with its edits: 73401's height and
    // 74301's (a gameplay row in each team) and 74301's boots id (an
    // aesthetics row, 0 in the fixture).
    let open_path = picked_save(&dir, "open", |file| {
        for id in [73401u32, 74301] {
            file.player_mut(id).expect("a player").basic.height = 183;
        }
        file.player_mut(74301).expect("74301").appearance.boots_id = 3601;
    });
    let picked = picked_save(&dir, "picked", |_| {});
    let mut view = SaveView::new();
    view.open(&open_path);
    view.panel = Some(Panel::Comparator);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    pick(&mut window, &mut view, picked.clone());
    compared(&mut window, &mut view);

    // The scope Gameplay, the team filter on 734: the export still holds
    // the whole save's report (74301's height row included).
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Gameplay"),
        &mut view,
    );
    compared(&mut window, &mut view);
    pick_comparator_team(&mut window, &mut view, "/lit/");
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |name| {
            assert_eq!(name, "diff.txt", "the suggested name");
            Some(target.clone())
        }
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let first = picked.to_str().unwrap().to_owned();
    let second = open_path.to_str().unwrap().to_owned();
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export written"),
        cli_stdout(&dir, &["diff", &first, &second, "--gameplay"]),
        "the file is `diff --gameplay`'s stdout, the team filter notwithstanding"
    );

    // All: the command's stdout with no flag.
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "All"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export rewritten"),
        cli_stdout(&dir, &["diff", &first, &second]),
        "the file is `diff`'s stdout with no flag"
    );
}

// SE-GUI-13
#[test]
fn an_empty_scopes_export_is_the_commands_empty_text() {
    // 73401's height the one difference (gameplay): the Aesthetics scope
    // holds no row, and the command prints nothing for it.
    let (mut view, mut window, _) = compared_73401("comparator_export_empty");
    let dir = scratch_of(&view);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Aesthetics"),
        &mut view,
    );
    compared(&mut window, &mut view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let open = view
        .session
        .as_ref()
        .unwrap()
        .path()
        .to_str()
        .unwrap()
        .to_owned();
    let picked = dir.join("picked").to_str().unwrap().to_owned();
    let stdout = cli_stdout(&dir, &["diff", &picked, &open, "--aesthetics"]);
    assert_eq!(stdout, "", "premise: the command prints nothing");
    assert_eq!(
        std::fs::read_to_string(&target).expect("the export written"),
        stdout,
        "an empty report exports an empty file"
    );
}

// SE-GUI-13
#[test]
fn export_carries_the_unsaved_edits() {
    // 73401's height to 183 in the session, no save written: the export
    // still holds the command's line for it.
    let (mut view, mut window) = open_734_with("comparator_export_unsaved", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    compared(&mut window, &mut view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let text = std::fs::read_to_string(&target).expect("the export written");
    assert!(
        text.contains("- Info player_diff (player=73401, field=Height, old=185, new=183)\n"),
        "the unsaved edit's line is in the export: {text}"
    );
}

// SE-GUI-13
#[test]
fn export_reports_the_written_path_and_refuses_a_folder_that_is_not() {
    let (mut view, mut window, _) = compared_73401("comparator_export_wrote");
    let dir = scratch_of(&view);
    let target = dir.join("diff.txt");
    view.export_dialog = Box::new({
        let target = target.clone();
        move |_| Some(target.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    let lines = window_lines(&mut window, &mut view, "Comparator");
    assert!(
        lines
            .iter()
            .any(|line| line == &format!("Wrote {}", target.display())),
        "the Wrote line: {lines:?}"
    );

    // A folder that does not exist: the refusal dialog names the path.
    let missing = dir.join("none").join("diff.txt");
    view.export_dialog = Box::new({
        let missing = missing.clone();
        move |_| Some(missing.clone())
    });
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Export\u{2026}"),
        &mut view,
    );
    // The refusal lands inside `comparator::show`, after `refusal_dialog`
    // ran that frame, so the modal's first drawn frame is its sizing
    // pass; the second holds the nodes.
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes
            .iter()
            .any(|node| node.value() == Some("Cannot export the differences")),
        "the refusal's title"
    );
    assert!(
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|value| value.contains(&missing.display().to_string()))
        }),
        "the refusal names the path"
    );
}

// SE-GUI-13
#[test]
fn another_save_resets_the_team_filter() {
    let (mut view, mut window, _) = compared_73401("comparator_filter_reset");
    let dir = scratch_of(&view);
    pick_comparator_team(&mut window, &mut view, "/lit/");
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::Team(734),
        "premise: 734 picked"
    );

    // Another save opened (the fixture again through `SaveView::open`):
    // its teams are not the old one's, so the filter reads ALL.
    let second = picked_save(&dir, "second", |_| {});
    view.open(&second);
    assert_eq!(
        view.comparator.filter_team,
        TeamChoice::All,
        "the filter reset"
    );
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        comparator_nodes(&pairs)
            .iter()
            .find(|node| node.role() == Role::ComboBox)
            .and_then(|combo| combo.value()),
        Some("ALL"),
        "the combo reads ALL"
    );
}

/// An invoked Pick… (Tab + Space, or AccessKit's Click — which leaves the
/// keyboard focus where it was) starts the comparison: the button's hold
/// on the keyboard is given up, so `edit_settled` reports settled.
#[test]
fn an_invoked_pick_starts_the_comparison() {
    let (mut view, mut window) = open_734_with("comparator_invoke", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, "picked", |_| {});
    view.panel = Some(Panel::Comparator);
    view.open_save_dialog = Box::new(move || Some(picked.clone()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (pick, _) = nodes_in_window(&pairs, "Comparator")
        .into_iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Pick\u{2026}"))
        .expect("the Pick button");
    action_focus(&mut window, &mut view, pick);
    action_click(&mut window, &mut view, pick);
    compared(&mut window, &mut view);
}

/// The pick row stays one line however long the picked save's path is:
/// the label, the truncated path and the buttons share a top, every
/// button inside the window.
#[test]
fn the_pick_row_keeps_the_buttons_on_one_line_with_a_long_path() {
    let (mut view, mut window) = open_734_with("comparator_row", &[73401], 183);
    let dir = scratch_of(&view);
    let picked = picked_save(&dir, &"picked".repeat(40), |_| {});
    view.panel = Some(Panel::Comparator);
    pick(&mut window, &mut view, picked);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, window_node) = pairs
        .iter()
        .find(|(_, node)| {
            node.label()
                .is_some_and(|label| label.starts_with("Comparator"))
        })
        .expect("the window");
    let window_rect = window_node.bounds().expect("the window's bounds");
    let nodes = comparator_nodes(&pairs);
    let top = |keep: &dyn Fn(&accesskit::Node) -> bool| -> f64 {
        nodes
            .iter()
            .find(|node| keep(node))
            .and_then(|node| node.bounds())
            .expect("a drawn node's bounds")
            .y0
    };
    let tops = [
        top(&|node| node.role() == Role::Label && node.value() == Some("Second save ")),
        top(&|node| {
            node.role() == Role::Label
                && node
                    .value()
                    .is_some_and(|value| value.contains("pickedpicked"))
        }),
        top(&|node| node.role() == Role::Button && node.label() == Some("Pick\u{2026}")),
        top(&|node| node.role() == Role::Button && node.label() == Some("Export\u{2026}")),
    ];
    for (a, b) in tops.iter().zip(tops.iter().skip(1)) {
        assert!((a - b).abs() < 0.5, "the row's tops: {tops:?}");
    }
    for label in ["Pick\u{2026}", "Export\u{2026}"] {
        let bounds = comparator_button(&pairs, label)
            .1
            .bounds()
            .expect("the button's bounds");
        assert!(
            bounds.x1 <= window_rect.x1,
            "{label} ends inside the window: {bounds:?} vs {window_rect:?}"
        );
    }
}

// The panel's title bar draws its title in the window-title style (the
// drafts' `.panel .title`): 14 points in the strong colour, where egui's
// title bar falls back to `Heading`'s 15.
#[test]
fn the_panels_title_is_14_points_and_strong() {
    let (mut view, mut window) = open_734_with("comparator_title", &[73401], 183);
    view.panel = Some(Panel::Comparator);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == "Comparator" => Some(shape),
            _ => None,
        })
        .next()
        .expect("the panel's title is painted");
    assert_eq!(
        title.galley.job.sections[0].format.font_id.size, 14.0,
        "the drafts' .panel .title"
    );
    let strong = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .strong_text_color();
    assert_eq!(
        title.galley.job.sections[0].format.color, strong,
        "the title's strong colour"
    );
}

#[test]
fn the_close_button_closes_the_panel() {
    let (mut view, mut window) = open_734_with("comparator_close", &[73401], 183);
    let pairs = open_comparator(&mut window, &mut view);
    assert_eq!(view.panel, Some(Panel::Comparator), "premise: open");
    let close = comparator_button(&pairs, "Close window").1.clone();
    click(&mut window, &close, &mut view);
    assert_eq!(view.panel, None, "the panel closes");
}

/// The panel's outer rect, from its AccessKit window node.
fn panel_rect(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Window && node.label() == Some("Comparator"))
        .and_then(accesskit::Node::bounds)
        .expect("the panel's window")
}

/// Whether `rect`'s left and top are `left` and `top`, within a point.
fn placed_at(rect: &accesskit::Rect, left: f64, top: f64) -> bool {
    (rect.x0 - left).abs() <= 1.0 && (rect.y0 - top).abs() <= 1.0
}

// A floating panel first opens where the drafts' `.panel` stands: at the
// 1280 x 780 client, 620 wide, its top 90 below the window's and its right
// edge 1 inside it, so (659, 90). Dragged, it stays where the member left
// it, across a close and a reopen.
#[test]
fn the_panel_first_opens_at_the_right_and_stays_where_dragged() {
    let (mut view, _) = open_734_with("comparator_place", &[73401], 183);
    let mut window = Headless::sized(1280.0, 780.0);
    open_comparator(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let rect = panel_rect(&nodes);
    assert!(
        placed_at(&rect, 659.0, 90.0) && (rect.x1 - rect.x0 - 620.0).abs() <= 1.0,
        "the panel's first rect: {rect:?}"
    );

    // Dragged by its title bar, left of the title.
    let from = egui::pos2(rect.x0 as f32 + 8.0, rect.y0 as f32 + 10.0);
    drag(
        &mut window,
        from,
        from + egui::vec2(-300.0, 100.0),
        &mut view,
    );
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dragged = panel_rect(&nodes);
    assert!(
        placed_at(&dragged, 359.0, 190.0),
        "the dragged panel: {dragged:?}"
    );

    view.panel = None;
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    view.panel = Some(Panel::Comparator);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let reopened = panel_rect(&nodes);
    assert!(
        placed_at(&reopened, 359.0, 190.0),
        "the reopened panel: {reopened:?}"
    );
}
