//! The view in a window larger than the default's: it keeps the layout
//! it has at `DEFAULT_PANE`, centred across the pane, and only what can
//! scroll there takes the extra room — the player list's height and the
//! Live AATF dock's width.

use std::collections::HashSet;

use super::*;

/// A maximized window's pane, 640 points wider and 229 taller than
/// `DEFAULT_PANE`, so what keeps its place stands 320 right of its
/// default spot.
const WIDE_PANE: egui::Vec2 = egui::vec2(1864.0, 985.0);

/// The shift a kept place takes at `WIDE_PANE`: half its extra width.
const SHIFT: f64 = 320.0;

/// The drop a pane-centred place takes at `WIDE_PANE`: half its extra
/// height (the welcome card centres in the panes).
const DOWN: f64 = 114.5;

/// Below this `y0` a no-save frame holds only the welcome card (the
/// toolbar's band ends over the panes well short of it).
const PANES_TOP: f64 = 30.0;

/// A bound's match slack.
const SLACK: f64 = 0.5;

/// The node's text: a Label's value, anything else's label.
fn text(node: &accesskit::Node) -> Option<String> {
    match node.role() {
        Role::Label => node.value().map(str::to_owned),
        _ => node.label().map(str::to_owned),
    }
}

/// `view` drawn at `pane`: the frame's AccessKit nodes with their ids
/// and its painted shapes, once the screens' own measurement passes and
/// the workers' replies have settled.
fn drawn(view: &mut SaveView, pane: egui::Vec2) -> (Pairs, Shapes) {
    let mut window = Headless::sized(pane.x, pane.y);
    window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    workers_replied(&mut window, view);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    (pairs, shapes)
}

/// A frame's AccessKit nodes with their ids.
type Pairs = Vec<(accesskit::NodeId, accesskit::Node)>;

/// A frame's painted shapes.
type Shapes = Vec<egui::epaint::ClippedShape>;

/// `pairs` keyed by node id.
fn by_id(pairs: &Pairs) -> HashMap<accesskit::NodeId, &accesskit::Node> {
    pairs.iter().map(|(id, node)| (*id, node)).collect()
}

/// The innermost `Pane` node over a node `inside` accepts: the panel
/// allowed the extra room (the player list's pane over a player row,
/// the Live AATF dock's over its title), `None` when no node matches.
fn pane_over(
    pairs: &Pairs,
    inside: impl Fn(&accesskit::Node) -> bool,
) -> Option<accesskit::NodeId> {
    let by_id = by_id(pairs);
    let mut parent = HashMap::new();
    for (id, node) in pairs {
        for child in node.children() {
            parent.insert(child, *id);
        }
    }
    pairs.iter().find_map(|(id, node)| {
        if !inside(node) {
            return None;
        }
        let mut at = Some(*id);
        while let Some(id) = at {
            if by_id[&id].role() == Role::Pane {
                return Some(id);
            }
            at = parent.get(&id).copied();
        }
        None
    })
}

/// `pairs`'s whole subtree under `root`, `root` included; the empty set
/// with `None`.
fn subtree(pairs: &Pairs, root: Option<accesskit::NodeId>) -> HashSet<accesskit::NodeId> {
    let mut ids = HashSet::new();
    let Some(root) = root else {
        return ids;
    };
    let by_id = by_id(pairs);
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if ids.insert(id) {
            stack.extend(by_id[&id].children());
        }
    }
    ids
}

/// Whether `at` is `from` shifted `SHIFT` right and `down` down at the
/// same size.
fn shifted(from: accesskit::Rect, at: accesskit::Rect, down: f64) -> bool {
    (at.x0 - from.x0 - SHIFT).abs() <= SLACK
        && (at.y0 - from.y0 - down).abs() <= SLACK
        && ((at.x1 - at.x0) - (from.x1 - from.x0)).abs() <= SLACK
        && ((at.y1 - at.y0) - (from.y1 - from.y0)).abs() <= SLACK
}

/// `default`'s labeled, bounded nodes that `check` keeps and no `wide`
/// node of the same role and label stands `(SHIFT, down)` from at the
/// same size — each misfit one line. `down` is `DOWN` for a node below
/// `centred_below`, `0.0` else.
fn misfits(
    default: &Pairs,
    wide: &Pairs,
    check: impl Fn(&accesskit::NodeId) -> bool,
    centred_below: Option<f64>,
) -> Vec<String> {
    default
        .iter()
        .filter(|(id, node)| check(id) && node.bounds().is_some() && text(node).is_some())
        .filter_map(|(_, node)| {
            let bounds = node.bounds().expect("bounded above");
            let label = text(node).expect("labeled above");
            let down = centred_below
                .filter(|top| bounds.y0 > *top)
                .map_or(0.0, |_| DOWN);
            let moved = wide.iter().any(|(_, candidate)| {
                candidate.role() == node.role()
                    && text(candidate).as_deref() == Some(label.as_str())
                    && candidate
                        .bounds()
                        .is_some_and(|at| shifted(bounds, at, down))
            });
            (!moved).then(|| format!("{:?} {label:?} at {bounds:?}", node.role()))
        })
        .collect()
}

/// `screen`'s frame at `DEFAULT_PANE` and `WIDE_PANE`, with the cap
/// asserted: every labeled, bounded node of the default frame stands
/// `SHIFT` right in the wide one at the same size, the player list's
/// and the Live AATF dock's subtrees and the dock's resize grip
/// excepted — the list takes the extra height and the dock the extra
/// width, so their nodes keep neither place nor count. `centred_below`
/// names the split for a pane-centred card: under it the welcome page
/// is the only content, checked `(SHIFT, DOWN)` instead. Returns both
/// frames' nodes for a screen's own extras.
fn drawn_pair(
    view: &mut SaveView,
    screen: &str,
    centred_below: Option<f64>,
) -> (Pairs, Pairs, Shapes, Shapes) {
    let (default, default_shapes) = drawn(view, DEFAULT_PANE);
    let (wide, wide_shapes) = drawn(view, WIDE_PANE);
    let mut exempt = subtree(&default, pane_over(&default, player_row));
    exempt.extend(subtree(
        &default,
        pane_over(&default, |node| node.value() == Some("Live AATF")),
    ));
    exempt.extend(
        default
            .iter()
            .filter(|(_, node)| node.role() == Role::Splitter)
            .map(|(id, _)| *id),
    );
    let found = misfits(&default, &wide, |id| !exempt.contains(id), centred_below);
    assert!(
        found.is_empty(),
        "{screen}: {} of the default frame's nodes do not keep their place and size:\n{}",
        found.len(),
        found.join("\n")
    );
    (default, wide, default_shapes, wide_shapes)
}

/// The PES 21 fixture save open with `setup` applied to the view.
fn open_view(test: &str, setup: impl FnOnce(&mut SaveView)) -> SaveView {
    let path = fixture(PesVersion::Pes21, test);
    let mut view = SaveView::new();
    view.open(&path);
    setup(&mut view);
    view
}

#[test]
fn the_welcome_view_keeps_its_layout_centred_in_a_wider_pane() {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    drawn_pair(&mut view, "the welcome view", Some(PANES_TOP));
}

#[test]
fn the_stats_screen_keeps_its_columns_centred_in_a_wider_pane() {
    let mut view = open_view("stretch_stats", |view| {
        view.selected = Some(Selection::Player(701, 0));
        // DBC7's event logo rides the view's top right: its keeping its
        // place is part of the check.
        view.aatf_ruleset = "dbc7".to_owned();
    });
    let (default, wide, default_shapes, wide_shapes) =
        drawn_pair(&mut view, "the Stats screen", None);
    // The list's own controls keep their place too: the team selector
    // and the filter box stand SHIFT right at the same size and y0.
    let list_pane = subtree(&default, pane_over(&default, player_row));
    let under = |pairs: &Pairs, role: Role| {
        pairs
            .iter()
            .find(|(id, node)| list_pane.contains(id) && node.role() == role)
            .and_then(|(_, node)| node.bounds())
            .unwrap_or_else(|| panic!("a {role:?} in the list pane"))
    };
    for (what, role) in [
        ("the team selector", Role::ComboBox),
        ("the filter box", Role::TextInput),
    ] {
        let (from, at) = (under(&default, role), under(&wide, role));
        assert!(
            shifted(from, at, 0.0),
            "{what} keeps its place at {WIDE_PANE:?} ({from:?} vs {at:?})"
        );
    }
    // The band's underline (a painted shape, not a node) ends where the
    // band ends: 320 further right in the wide frame, before the logo.
    let band_line = |shapes: &Shapes| {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke: _ } => Some(*points),
                _ => None,
            })
            .filter(|points| points[0].y == points[1].y)
            .max_by(|a, b| {
                let w = |points: &[egui::Pos2; 2]| points[1].x - points[0].x;
                w(a).partial_cmp(&w(b)).expect("ordered bounds")
            })
            .expect("the toolbar band's underline")
    };
    let (default_end, wide_end) = (
        band_line(&default_shapes)[1].x,
        band_line(&wide_shapes)[1].x,
    );
    assert!(
        (wide_end - default_end - SHIFT as f32).abs() <= SLACK as f32,
        "the band's underline ends 320 further right at {WIDE_PANE:?} ({default_end} vs {wide_end})"
    );
    // The player list takes the extra height: more rows show, their
    // frame keeping the default width (the rows fill it).
    let rows = |pairs: &Pairs| pairs.iter().filter(|(_, node)| player_row(node)).count();
    assert!(
        rows(&wide) > rows(&default),
        "the player list shows more rows at {WIDE_PANE:?} than at {DEFAULT_PANE:?} ({} > {})",
        rows(&wide),
        rows(&default)
    );
    let frame = |pairs: &Pairs| {
        let mut rows = pairs
            .iter()
            .filter(|(_, node)| player_row(node))
            .filter_map(|(_, node)| node.bounds());
        let first = rows.next().expect("a player row's bounds");
        rows.fold(first, |acc, bounds| accesskit::Rect {
            x0: acc.x0.min(bounds.x0),
            y0: acc.y0.min(bounds.y0),
            x1: acc.x1.max(bounds.x1),
            y1: acc.y1.max(bounds.y1),
        })
    };
    let (default_frame, wide_frame) = (frame(&default), frame(&wide));
    assert!(
        ((wide_frame.x1 - wide_frame.x0) - (default_frame.x1 - default_frame.x0)).abs() <= SLACK,
        "the player list's frame keeps its width ({default_frame:?} vs {wide_frame:?})"
    );
}

#[test]
fn the_appearance_screen_keeps_its_panels_centred_in_a_wider_pane() {
    let mut view = open_view("stretch_appearance", |view| {
        view.selected = Some(Selection::Player(701, 0));
        view.page = PlayerScreen::Appearance;
    });
    drawn_pair(&mut view, "the Appearance screen", None);
}

#[test]
fn the_gameplan_screen_keeps_its_columns_centred_in_a_wider_pane() {
    let mut view = open_view("stretch_gameplan", |view| {
        view.selected = Some(Selection::Team(701));
    });
    drawn_pair(&mut view, "the Gameplan screen", None);
}

#[test]
fn live_aatfs_dock_takes_the_extra_width_in_a_wider_pane() {
    let mut view = open_view("stretch_live_aatf", |view| {
        view.selected = Some(Selection::Player(701, 0));
        view.aatf_ruleset = "dbc7".to_owned();
        view.live_aatf = true;
    });
    let (default, wide, _, _) = drawn_pair(&mut view, "Stats with Live AATF on", None);
    // The dock's width is its resize grip's, spanning the pane; its
    // height stays while its top moves the pane's whole extra down (its
    // bottom is the pane's edge).
    let grip = |pairs: &Pairs| {
        pairs
            .iter()
            .find(|(_, node)| node.role() == Role::Splitter)
            .and_then(|(_, node)| node.bounds())
            .expect("the dock's resize grip")
    };
    let title_top = |pairs: &Pairs| {
        pairs
            .iter()
            .find(|(_, node)| node.value() == Some("Live AATF"))
            .and_then(|(_, node)| node.bounds())
            .expect("the dock's title")
            .y0
    };
    let (default_grip, wide_grip) = (grip(&default), grip(&wide));
    // The dock starts at the panes' left edge, which at the default size
    // is the player list's, and stays there in the wider pane.
    let by_id = by_id(&default);
    let list_left = subtree(&default, pane_over(&default, player_row))
        .iter()
        .filter_map(|id| by_id[id].bounds())
        .map(|bounds| bounds.x0)
        .fold(f64::INFINITY, f64::min);
    for at in [default_grip, wide_grip] {
        assert!(
            (at.x0 - list_left).abs() <= SLACK,
            "the dock starts at {} where the list starts at {list_left}",
            at.x0
        );
    }
    assert!(
        ((wide_grip.x1 - wide_grip.x0) - (default_grip.x1 - default_grip.x0) - 640.0).abs()
            <= SLACK
            && (title_top(&wide) - title_top(&default) - 229.0).abs() <= SLACK,
        "the dock is 640 wider at {WIDE_PANE:?} and as tall ({default_grip:?} vs {wide_grip:?})"
    );
}
