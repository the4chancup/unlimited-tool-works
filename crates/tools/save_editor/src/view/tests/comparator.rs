//! The comparator panel's tests (SE-GUI-13): the pick and its refusals, the
//! rows in the views' terms, the scope toggles and the team filter, the
//! only-in heads, the click's selection, the `Comparing…` mark and
//! Export.

use std::path::PathBuf;

use pes_savefile::ops::compare::DiffScope;

use super::*;

use crate::test_support::{TEST_SALT, cli_stdout, journal_lines};

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

mod export;
mod filter;
mod rows;
