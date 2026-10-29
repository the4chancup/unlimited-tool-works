//! The headless-window tests of the view's scenarios: one egui context with
//! AccessKit on, run a frame at a time; what a frame draws is its AccessKit
//! nodes and painted shapes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui::accesskit::{self, Role};
use pes_savefile::interchange::team_toml::labels::{PLAYABLE_POSITIONS, POSITIONS};
use pes_savefile::model::TeamEntry;
use pes_savefile::model::ingame_face::IngameFace;
use pes_savefile::model::names::{display_name, name_spans};
use pes_savefile::model::tactics::FormationSlot;
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::{EditFile, fixture_save};
use pes_version::PesVersion;

use super::*;

/// A headless window: one egui context with AccessKit on and animations off,
/// run a frame at a time over the pane the shell leaves a tool at the
/// default window (`DEFAULT_PANE`), or another size when a test asks.
struct Headless {
    ctx: egui::Context,
    /// The screen size each frame reports.
    screen: egui::Vec2,
}

impl Headless {
    fn new() -> Headless {
        Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y)
    }

    /// A window `width` by `height` instead of the default `DEFAULT_PANE`.
    fn sized(width: f32, height: f32) -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // The window draws with these fonts and style; the test measures
        // what a member sees, whichever crates the build unifies.
        studio_core::theme::install(&ctx);
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
        Headless {
            ctx,
            screen: egui::vec2(width, height),
        }
    }

    /// Runs one frame of `contents`; returns what it returned, the frame's
    /// AccessKit nodes and its painted shapes.
    fn frame<R>(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> (R, Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        let (result, nodes, shapes) = self.frame_ids(events, contents);
        (
            result,
            nodes.into_iter().map(|(_, node)| node).collect(),
            shapes,
        )
    }

    /// `frame` keeping the nodes' AccessKit ids (for `AccessKitActionRequest`
    /// events).
    fn frame_ids<R>(
        &mut self,
        events: Vec<egui::Event>,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> (
        R,
        Vec<(accesskit::NodeId, accesskit::Node)>,
        Vec<egui::epaint::ClippedShape>,
    ) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
            events,
            ..Default::default()
        };
        let mut result = None;
        // The shell's `fn ui` draws every tool's view inside the window's
        // `CentralPanel`, whose margins the pane loses: a `Headless` window
        // stands for that panel's rect, so the tests see the widths the real
        // window leaves.
        let mut output = self.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| result = Some(contents(ui)));
        });
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        let shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (
            result.expect("run_ui calls the closure"),
            update.nodes,
            shapes,
        )
    }
}

/// Writes `version`'s fixture save under `test`'s scratch folder (fresh each
/// call, so a `.bak` an older run left cannot shadow the assertions).
fn fixture(version: PesVersion, test: &str) -> PathBuf {
    let dir = crate::test_support::scratch(test);
    let path = dir.join("EDIT00000000");
    std::fs::write(&path, fixture_save(version)).unwrap();
    path
}

/// `player_id`'s entry as the session holds it.
fn player_entry(view: &SaveView, player_id: u32) -> PlayerEntry {
    view.session
        .as_ref()
        .unwrap()
        .file()
        .player(player_id)
        .unwrap()
        .clone()
}

/// `tool_context` for the submodule tests (`use super::*`).
use crate::test_support::tool_context;

/// The centre of a node's bounds, where a click lands.
fn centre(node: &accesskit::Node) -> egui::Pos2 {
    let bounds = node.bounds().expect("a widget has bounds");
    egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    )
}

/// The primary button pressed (or released) at `pos`, the pointer moved
/// there first.
fn pointer_at(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

/// A press on `from`'s centre, the pointer dragged to `onto`'s and released:
/// a drag of `from`'s widget onto `onto`'s.
fn drag(window: &mut Headless, from: egui::Pos2, onto: egui::Pos2, view: &mut SaveView) {
    window.frame(pointer_at(from, true), |ui| show(ui, view, &|| Vec::new()));
    window.frame(vec![egui::Event::PointerMoved(onto)], |ui| {
        show(ui, view, &|| Vec::new())
    });
    window.frame(pointer_at(onto, false), |ui| show(ui, view, &|| Vec::new()));
}

/// `entry` without what a roster slot owns (`id`, `appearance_id`,
/// `appearance.base_copy_id`, and the in-game face run apart from the
/// exported fields): the record a drag reorder carries.
fn record(entry: &PlayerEntry) -> PlayerEntry {
    let mut record = entry.clone();
    record.id = 0;
    record.appearance_id = None;
    record.appearance.base_copy_id = 0;
    record.appearance.ingame_face = IngameFace::default();
    record
}

/// The list's player rows' labels in screen order (the AccessKit update is
/// unordered).
fn row_labels(nodes: &[accesskit::Node]) -> Vec<String> {
    let mut rows: Vec<&accesskit::Node> = nodes.iter().filter(|node| player_row(node)).collect();
    rows.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(0.0);
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    rows.iter()
        .filter_map(|node| node.label().map(str::to_owned))
        .collect()
}

/// A press at `at`, then the release: the release frame's nodes. `draw`
/// runs the view under test on each of the two frames.
fn press_release(
    window: &mut Headless,
    at: egui::Pos2,
    mut draw: impl FnMut(&mut egui::Ui),
) -> Vec<accesskit::Node> {
    window.frame(pointer_at(at, true), &mut draw);
    let (_, nodes, _) = window.frame(pointer_at(at, false), draw);
    nodes
}

/// A click on `node`: press on one frame, release on the next.
fn click(window: &mut Headless, node: &accesskit::Node, view: &mut SaveView) {
    press_release(window, centre(node), |ui| show(ui, view, &|| Vec::new()));
}

/// An AccessKit `Click` action on `id`: a click delivered as the
/// accessibility event, for widgets a pointer click does not reach (a
/// combo's popup items).
fn action_click(window: &mut Headless, view: &mut SaveView, id: accesskit::NodeId) {
    window.frame(
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: id,
                data: None,
            },
        )],
        |ui| show(ui, view, &|| Vec::new()),
    );
}

/// An AccessKit `Focus` action on `id`: keyboard focus without a click.
fn action_focus(window: &mut Headless, view: &mut SaveView, id: accesskit::NodeId) {
    window.frame(
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::Focus,
                target_tree: accesskit::TreeId::ROOT,
                target_node: id,
                data: None,
            },
        )],
        |ui| show(ui, view, &|| Vec::new()),
    );
}

/// A key pressed (or released) with `modifiers` held.
fn key(key: egui::Key, modifiers: egui::Modifiers, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        modifiers,
        repeat: false,
    }
}

/// The Button whose label starts with `label` (a menu item's label carries
/// its shortcut text after the name).
fn menu_item<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button && node.label().is_some_and(|item| item.starts_with(label))
        })
        .unwrap_or_else(|| panic!("no menu item {label:?}"))
}

/// The label of the menu button named `menu`, as
/// `studio_core::widgets::menu_button` draws it: the name, then the arrow.
fn menu_label(menu: &str) -> String {
    format!("{menu} \u{23f7}")
}

/// The toolbar menu button named `label` (the topmost button with it).
fn menu_button<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    let label = menu_label(label);
    nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.label() == Some(label.as_str()))
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().map(|r| r.y0).unwrap_or(f64::MAX);
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .unwrap_or_else(|| panic!("the {label:?} menu button"))
}

/// `menu`'s popup open for one frame: the pointer moved onto its button,
/// which opens it while another of the toolbar's menus is open (a click
/// then would close it again), else a click on it; either skipped when
/// `item` is visible already. Then the frame's nodes.
fn open_menu(
    window: &mut Headless,
    view: &mut SaveView,
    menu: &str,
    item: &str,
) -> Vec<accesskit::Node> {
    let shows_item = |nodes: &[accesskit::Node]| {
        nodes.iter().any(|node| {
            node.role() == Role::Button && node.label().is_some_and(|l| l.starts_with(item))
        })
    };
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    if shows_item(&nodes) {
        return nodes;
    }
    let button = centre(menu_button(&nodes, menu));
    window.frame(vec![egui::Event::PointerMoved(button)], |ui| {
        show(ui, view, &|| Vec::new())
    });
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    if shows_item(&nodes) {
        return nodes;
    }
    click(window, menu_button(&nodes, menu), view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    nodes
}

/// The Open menu opened and its `Choose file…` clicked, discovery stubbed
/// to find no save, so the menu holds the recent files and the picker.
fn choose_file(window: &mut Headless, view: &mut SaveView) {
    view.discover = Box::new(|_| Vec::new());
    let nodes = open_menu(window, view, "Open", "Choose file");
    click(window, menu_item(&nodes, "Choose file"), view);
}

/// Replaces the focused text control's contents with `text`: End moves the
/// caret past the text, the backspaces clear it, then the characters land.
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

/// The AccessKit node of the row labeled `label`.
fn row<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no row {label:?}"))
}

/// Whether `node` is a player row of the list (`{number} {position} …`): a
/// Button whose label's second word is a position. A team row's label
/// (`{id} {name}`) starts with a digit too, but its second word is the
/// team's name.
fn player_row(node: &accesskit::Node) -> bool {
    node.role() == Role::Button
        && node
            .label()
            .and_then(|label| label.split_whitespace().nth(1))
            .is_some_and(|word| word == "?" || POSITIONS.contains(&word))
}

/// The team row's node (`{id} {display name}`): `team_id`'s own row in the
/// list.
fn team_row<'a>(
    nodes: &'a [accesskit::Node],
    view: &SaveView,
    team_id: u32,
) -> &'a accesskit::Node {
    let name = view
        .session
        .as_ref()
        .and_then(|session| session.file().team(team_id))
        .map(|team| display_name(&team.name))
        .unwrap_or_else(|| panic!("no team {team_id}"));
    let label = format!("{team_id} {name}");
    nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label.as_str()))
        .unwrap_or_else(|| panic!("no team row {label:?}"))
}

/// A click on `team_id`'s row in the list: the team view's selection.
fn open_team_view(window: &mut Headless, view: &mut SaveView, team_id: u32) {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    click(window, team_row(&nodes, view, team_id), view);
    assert_eq!(
        view.selected,
        Some(Selection::Team(team_id)),
        "the click selected team {team_id}'s row"
    );
}

/// The Appearance section opened by its header's click (already open: no
/// click, which would close it). The click goes through the AccessKit
/// action: the header sits below the fold on small panes, where a pointer
/// click cannot reach it.
fn open_appearance(window: &mut Headless, view: &mut SaveView) {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let (id, expanded) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::DisclosureTriangle && node.label() == Some("Appearance")
        })
        .map(|(id, node)| (*id, node.is_expanded()))
        .expect("the Appearance section's header");
    if expanded != Some(true) {
        action_click(window, view, id);
    }
}

/// Frames scrolling the detail pane until `target`'s node is drawn inside
/// the scroll's clip, bounded: a control below or above the fold takes
/// pointer clicks and hovers only once the clip covers it. `target` names
/// the node the caller then clicks, hovers or types into (its `bounds`
/// change as the pane scrolls, so it is found again on the returned frame).
fn scroll_into_view(
    window: &mut Headless,
    view: &mut SaveView,
    target: impl Fn(&accesskit::Node) -> bool,
) -> Vec<accesskit::Node> {
    // The pointer rests inside the scroll's clip: the MouseWheel scrolls
    // the pane under it, never the list's.
    let at = egui::pos2(600.0, 400.0);
    // egui spreads a wheel delta of 8 points or more over the next frames
    // (`WheelState::after_events`, not configurable), so the target found
    // inside the clip may still be moving: it is returned only once its
    // bounds held for a frame, else a hover or click lands where it was.
    let mut last = None;
    for _ in 0..48 {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        // The clip the detail's ScrollArea leaves the rows: the rightmost
        // bar's span when it exists, the window's own otherwise.
        let clip = nodes
            .iter()
            .filter(|node| node.role() == Role::ScrollBar)
            .filter_map(|node| node.bounds())
            .max_by(|a, b| a.x0.partial_cmp(&b.x0).expect("ordered bounds"))
            .map(|bounds| (bounds.y0, bounds.y1))
            .unwrap_or((0.0, f64::from(window.screen.y)));
        let found = nodes.iter().find_map(|node| {
            if target(node) {
                node.bounds().map(|bounds| (node, bounds))
            } else {
                None
            }
        });
        // The widget's centre inside the clip is what a click, a hover or
        // a typed edit needs — the half above may clip and it still lands.
        let settled = found.is_some_and(|(_, bounds)| last == Some(bounds));
        last = found.map(|(_, bounds)| bounds);
        if let Some((_, bounds)) = found
            && (bounds.y0 + bounds.y1) / 2.0 >= clip.0
            && bounds.y1 <= clip.1
        {
            if settled {
                return nodes;
            }
            // Inside but still moving: a plain frame lets the wheel's rest
            // apply before the target is looked at again.
            continue;
        }
        // Below the fold: scroll down; above it (scrolled past): back up.
        let delta = match found {
            Some((_, bounds)) if (bounds.y0 + bounds.y1) / 2.0 < clip.0 => 400.0,
            _ => -400.0,
        };
        window.frame(
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, delta),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, view, &|| Vec::new()),
        );
    }
    panic!("the target never scrolled into view")
}

/// The painted text shape whose galley text starts with `starts_with`.
fn text_shape<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    starts_with: &str,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text.starts_with(starts_with) => {
                Some(shape)
            }
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted text starting {starts_with:?}"))
}

/// The ink of `shape`'s painted glyphs: each glyph's bitmap where the
/// tessellator places it (`galley`'s row, the glyph's baseline position
/// and its bitmap's offset), spaces skipped.
fn ink(shape: &egui::epaint::TextShape) -> egui::Rect {
    let mut rect = egui::Rect::NOTHING;
    for row in &shape.galley.rows {
        for glyph in &row.row.glyphs {
            if glyph.uv_rect.is_nothing() {
                continue;
            }
            let min = shape.pos + row.pos.to_vec2() + glyph.pos.to_vec2() + glyph.uv_rect.offset;
            rect = rect.union(egui::Rect::from_min_size(min, glyph.uv_rect.size));
        }
    }
    rect
}

/// The colour `shape`'s text paints in: a section's own colour when one
/// was set, else the shape's fallback (the widget's).
fn galley_color(shape: &egui::epaint::TextShape) -> egui::Color32 {
    if let Some(color) = shape.override_text_color {
        return color;
    }
    shape
        .galley
        .job
        .sections
        .iter()
        .map(|section| section.format.color)
        .find(|&color| color != egui::Color32::PLACEHOLDER)
        .unwrap_or(shape.fallback_color)
}

/// The colour `value`'s label painted in: the `TextShape` with the
/// label's own text inside the Label node's bounds (a Button's painted
/// label can share the text, so the bounds pick the view's own).
fn painted_label_color(
    nodes: &[accesskit::Node],
    shapes: &[egui::epaint::ClippedShape],
    value: &str,
) -> egui::Color32 {
    let bounds = label_bounds(nodes, value);
    let shape = shapes
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
        .unwrap_or_else(|| panic!("no painted {value:?} inside {bounds:?}"));
    galley_color(shape)
}

/// The Live AATF mark text shapes (`✖` or `✔`) painted inside `bounds`
/// (a list row's marks).
fn marks_in_row(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
) -> Vec<&egui::epaint::TextShape> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape)
                if (shape.galley.job.text == "\u{2716}" || shape.galley.job.text == "\u{2714}")
                    && f64::from(shape.pos.y) >= bounds.y0
                    && f64::from(shape.pos.y) <= bounds.y1
                    && f64::from(shape.pos.x) >= bounds.x0
                    && f64::from(shape.pos.x) <= bounds.x1 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .collect()
}

/// The `●` text shape painted inside `bounds` (a dirty row's dot).
fn dot_in_row(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
) -> &egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "\u{25cf}"
                    && f64::from(text.pos.y) >= bounds.y0
                    && f64::from(text.pos.y) <= bounds.y1
                    && f64::from(text.pos.x) >= bounds.x0
                    && f64::from(text.pos.x) <= bounds.x1 =>
            {
                Some(text)
            }
            _ => None,
        })
        .next()
        .expect("a `●` painted on the row")
}

/// The fill under `bounds`' centre: the smallest `Shape::Rect` covering
/// it (the field's own frame; the panel's background would win an
/// `intersects` match instead).
fn fill_at(
    shapes: &[egui::epaint::ClippedShape],
    bounds: accesskit::Rect,
) -> Option<egui::Color32> {
    let centre = egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    );
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.rect.contains(centre) => {
                Some((shape.fill, shape.rect.area()))
            }
            _ => None,
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).expect("areas"))
        .map(|(fill, _)| fill)
}

/// The Shirt field's `TextInput` node (the text edit right of the "Shirt
/// (n/max)" label).
fn shirt_field(nodes: &[accesskit::Node]) -> &accesskit::Node {
    field_right_of_label(nodes, "Shirt (", Role::TextInput)
}

/// The undo stack's depth, counted by Ctrl+Z until it is empty (bounded, so
/// a stuck stack fails rather than hangs).
fn undo_entries(window: &mut Headless, view: &mut SaveView) -> usize {
    for count in 0..64 {
        if !view.session.as_ref().is_some_and(|s| s.can_undo()) {
            return count;
        }
        window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
            show(ui, view, &|| Vec::new())
        });
    }
    panic!("Ctrl+Z did not empty the undo stack in 64 presses")
}

/// The detail pane's frame: the widest full-height rect past the list's
/// own (its left edge is the player list's right edge).
fn pane_frame(shapes: &[egui::epaint::ClippedShape], screen: egui::Vec2) -> (f64, f64) {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.rect.min.x > 200.0
                    && shape.rect.width() > screen.x * 0.5
                    && shape.rect.height() > screen.y * 0.75 =>
            {
                Some((f64::from(shape.rect.min.x), f64::from(shape.rect.max.x)))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"))
        .expect("the detail pane's frame rect")
}

/// The player list's frame: the rect filled in `extreme_bg_color` under
/// the rows, left of the pane and taller than half the screen (the list's
/// scroll bar's track and the filter box share the fill, but neither is
/// that wide and tall).
fn list_frame(window: &Headless, shapes: &[egui::epaint::ClippedShape]) -> egui::Rect {
    let extreme = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .extreme_bg_color;
    let screen = window.screen;
    shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == extreme
                    && shape.rect.max.x < 300.0
                    && shape.rect.width() > 100.0
                    && shape.rect.height() > screen.y * 0.5 =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .expect("the player list's frame rect")
}

/// The player list's scroll bar: the leftmost `ScrollBar` node (the
/// detail pane scrolls too).
fn list_scroll_bar(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .filter(|node| node.role() == Role::ScrollBar)
        .filter_map(accesskit::Node::bounds)
        .min_by(|a, b| a.x0.total_cmp(&b.x0))
        .expect("the list's scroll bar")
}

/// Whether `node` sits on `anchor`'s row (same top within 4 points) and
/// right of it.
fn on_row_right_of(anchor: accesskit::Rect, node: &accesskit::Node) -> bool {
    node.bounds()
        .is_some_and(|b| (b.y0 - anchor.y0).abs() < 4.0 && b.x0 >= anchor.x1 - 1.0)
}

/// The leftmost node `keep` passes on `anchor`'s row, right of it.
fn first_right_of<'a>(
    nodes: &'a [accesskit::Node],
    anchor: &accesskit::Node,
    keep: impl Fn(&accesskit::Node) -> bool,
) -> Option<&'a accesskit::Node> {
    let bounds = anchor.bounds().expect("the anchor's bounds");
    nodes
        .iter()
        .filter(|node| keep(node) && on_row_right_of(bounds, node))
        .min_by(|a, b| {
            let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
}

/// The first node of `role` on `anchor`'s row to its right.
fn right_of<'a>(
    nodes: &'a [accesskit::Node],
    anchor: &accesskit::Node,
    role: Role,
) -> &'a accesskit::Node {
    first_right_of(nodes, anchor, |node| node.role() == role).expect("a node right of the anchor")
}

/// The `role` node on the row of the label whose value starts with
/// `prefix`, to its right — the counted fields' label-left layout puts
/// the field right of its `Name (n/max)`/`Shirt (n/max)` label.
fn field_right_of_label<'a>(
    nodes: &'a [accesskit::Node],
    prefix: &str,
    role: Role,
) -> &'a accesskit::Node {
    let label = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label
                && node.value().is_some_and(|value| value.starts_with(prefix))
        })
        .unwrap_or_else(|| panic!("no label starting {prefix:?}"));
    first_right_of(nodes, label, |node| node.role() == role)
        .unwrap_or_else(|| panic!("no {role:?} right of a {prefix:?} label"))
}

/// The Name field's `TextInput` node (the text edit right of the "Name
/// (n/max)" label).
fn name_field(nodes: &[accesskit::Node]) -> &accesskit::Node {
    field_right_of_label(nodes, "Name (", Role::TextInput)
}

/// The widget nodes of `role` on `label`'s row to its right, in x order.
fn right_of_label<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
) -> Vec<&'a accesskit::Node> {
    let label_node = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .unwrap_or_else(|| panic!("no label {label:?}"));
    let bounds = label_node.bounds().expect("the label's bounds");
    let mut found: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == role && on_row_right_of(bounds, node))
        .collect();
    found.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    found
}

/// The `C` `B` `A` buttons of `position`'s cell, in x order: the three
/// buttons on the ratings row under the cell's label, the closest to the
/// label's centre (the label is centred over its ratings).
fn rating_buttons<'a>(nodes: &'a [accesskit::Node], position: &str) -> Vec<&'a accesskit::Node> {
    let label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(position))
        .unwrap_or_else(|| panic!("no label {position:?}"));
    let bounds = label.bounds().expect("the label's bounds");
    let centre = (bounds.x0 + bounds.x1) / 2.0;
    let mut ratings: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && matches!(node.label(), Some("C" | "B" | "A"))
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 >= bounds.y1 - 1.0 && b.y0 <= bounds.y1 + 40.0)
        })
        .collect();
    ratings.sort_by(|a, b| {
        let dist = |node: &&accesskit::Node| {
            let b = node.bounds().expect("bounds");
            ((b.x0 + b.x1) / 2.0 - centre).abs()
        };
        dist(a).partial_cmp(&dist(b)).expect("ordered bounds")
    });
    ratings.truncate(3);
    ratings.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    assert_eq!(ratings.len(), 3, "{position}'s three ratings");
    ratings
}

/// The first interactive node (any role but Label) on `label`'s row to its
/// right: the control the label names.
fn control_right_of<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    let label_node = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .unwrap_or_else(|| panic!("no label {label:?}"));
    first_right_of(nodes, label_node, |node| node.role() != Role::Label)
        .unwrap_or_else(|| panic!("no control right of {label:?}"))
}

/// The disabled `-` button right of `label` (a gated number control).
fn disabled_dash<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    right_of_label(nodes, label, Role::Button)
        .into_iter()
        .find(|node| node.is_disabled() && node.label() == Some("-"))
        .unwrap_or_else(|| panic!("no disabled '-' after {label:?}"))
}

/// The tooltip text after the pointer rests at `pos` for a while (bounded:
/// a missing tooltip fails rather than hangs). The pointer first moves away
/// for a frame: egui refuses a tooltip while the last click is more recent
/// than the last movement plus 0.1 s, and both clocks advance together, so
/// a `PointerMoved` that only repeats the clicked position never clears it.
fn tooltip_after_hover(
    window: &mut Headless,
    view: &mut SaveView,
    pos: egui::Pos2,
) -> Option<String> {
    tooltip_containing(window, view, pos, "PES ")
}

/// `tooltip_after_hover` filtered to the tooltip node holding `needle`.
/// The hover must add a node holding `needle` to the frame the pointer
/// left from: a label the view always draws with the same text (the
/// ownership line, an error line) is counted there already, so it cannot
/// pass for the tooltip.
fn tooltip_containing(
    window: &mut Headless,
    view: &mut SaveView,
    pos: egui::Pos2,
    needle: &str,
) -> Option<String> {
    let holding = |nodes: &[accesskit::Node]| -> Vec<String> {
        nodes
            .iter()
            .filter_map(|node| node.value())
            .filter(|value| value.contains(needle))
            .map(str::to_owned)
            .collect()
    };
    let (_, resting, _) = window.frame(vec![egui::Event::PointerMoved(egui::Pos2::ZERO)], |ui| {
        show(ui, view, &|| Vec::new())
    });
    let resting = holding(&resting);
    window.frame(vec![egui::Event::PointerMoved(pos)], |ui| {
        show(ui, view, &|| Vec::new())
    });
    let count = |values: &[String], value: &String| values.iter().filter(|v| *v == value).count();
    for _ in 0..90 {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        let values = holding(&nodes);
        if let Some(added) = values
            .iter()
            .find(|value| count(&values, value) > count(&resting, value))
        {
            return Some(added.clone());
        }
    }
    None
}

/// `player_id`'s first C-rated outfield position's stored byte: a spot
/// to move a starter to for a `FieldedNotA`, picked from his record as
/// `aatf`'s own `FieldedNotA` fixture picks AMF.
fn unrated_position_byte(view: &SaveView, player_id: u32) -> u8 {
    let unrated = player_entry(view, player_id).positions.playable[..12]
        .iter()
        .position(|rating| *rating == 0)
        .expect("a C-rated outfield position");
    u8::try_from(PLAYABLE_POSITIONS[unrated]).expect("a position byte")
}

/// The team's entry as the session holds it (the team view's tests).
fn team_entry(view: &SaveView, team_id: u32) -> TeamEntry {
    view.session
        .as_ref()
        .unwrap()
        .file()
        .team(team_id)
        .unwrap()
        .clone()
}

/// Whether the nodes draw a Label with `value` as its text.
fn label_value(nodes: &[accesskit::Node], value: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.role() == Role::Label && node.value() == Some(value))
}

/// The pane the shell leaves a tool's view at the 1280 x 780 default
/// window (`spec/core/gui.md` "The window's size"): the sidebar's 56-point
/// rail (`studio_core`'s `RAIL_WIDTH`, which it does not export) and the
/// 24-point status bar taken off. It is also the narrowest pane a member
/// sees: a narrower window scales the UI down and lays it out at 1280
/// (`spec/core/gui.md` "Scaling with the width").
const DEFAULT_PANE: egui::Vec2 = egui::vec2(1224.0, 756.0);

/// A `CentralPanel`'s inner margin on each side (egui's
/// `Frame::central_panel`, `inner_margin(8)`): the shell's panel and the
/// view's own each take one off the pane's edge, but at the right, where
/// the view has none (`PANE_RIGHT`).
const PANEL_MARGIN: f32 = 8.0;

/// The detail pane's right edge at `DEFAULT_PANE`: the view's, the shell's
/// margin short of the window's edge, as the draft's `.detail` content
/// ends (1272 of the 1280-point client).
const PANE_RIGHT: f32 = DEFAULT_PANE.x - PANEL_MARGIN;

/// The bounds of the Label node showing `value` (a grid's label cell).
fn label_bounds(nodes: &[accesskit::Node], value: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(value))
        .and_then(accesskit::Node::bounds)
        .unwrap_or_else(|| panic!("no label {value:?}"))
}

/// The chips under `label`'s, top to bottom (each `.tset`'s chip shares
/// its label's left edge — the Player assignments' rows).
fn chips_below<'a>(nodes: &'a [accesskit::Node], label: &str) -> Vec<&'a accesskit::Node> {
    let bounds = label_bounds(nodes, label);
    let mut found: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 > bounds.y0 && (b.x0 - bounds.x0).abs() < 1.0)
        })
        .collect();
    found.sort_by(|a, b| {
        let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
        y(a).partial_cmp(&y(b)).expect("ordered bounds")
    });
    found
}

/// The chip under `label`'s of the "Player assignments" palette (the
/// first one below it — a role's own).
fn chip<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    chips_below(nodes, label)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no chip under {label:?}"))
}

/// The instruction grid's four combos inside the "Advanced
/// instructions" panel (between its heading and the next panel's), as
/// `[[attack 1, defence 1], [attack 2, defence 2]]` — attack the left
/// column, defence the right.
fn instruction_combos(nodes: &[accesskit::Node]) -> [[&accesskit::Node; 2]; 2] {
    let panel_top = label_bounds(nodes, "Advanced instructions").y0;
    let panel_bottom = label_bounds(nodes, "Automatic settings").y0;
    let mut combos: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::ComboBox
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 > panel_top && b.y0 < panel_bottom)
        })
        .collect();
    combos.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("combo bounds");
            (bounds.x0, bounds.y0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    let [a1, a2, d1, d2] = <[&accesskit::Node; 4]>::try_from(combos.as_slice())
        .expect("the instruction grid's four combos");
    [[a1, d1], [a2, d2]]
}

/// The chip under `combo` in its column (an instruction's target chip:
/// the Button sharing the combo's left edge, nearest below it).
fn chip_under<'a>(nodes: &'a [accesskit::Node], combo: &accesskit::Node) -> &'a accesskit::Node {
    let bounds = combo.bounds().expect("combo bounds");
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .bounds()
                    .is_some_and(|b| b.y0 > bounds.y0 && (b.x0 - bounds.x0).abs() < 1.0)
        })
        .min_by(|a, b| {
            let y = |node: &&accesskit::Node| node.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("no chip under the combo")
}

/// The Formation settings' half width: the right label column's x less
/// the left's and the 16-point gap.
fn settings_half(nodes: &[accesskit::Node]) -> f64 {
    let xs = |label: &str| label_bounds(nodes, label).x0;
    xs("Defensive style") - xs("Attacking style") - 16.0
}

/// The `role` widgets under `label`'s, in its Formation settings
/// column (the label over its control).
fn widgets_below<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
) -> Vec<&'a accesskit::Node> {
    let bounds = label_bounds(nodes, label);
    let half = settings_half(nodes);
    let mut found: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == role
                && node.bounds().is_some_and(|b| {
                    b.y0 > bounds.y0
                        && b.y0 <= bounds.y1 + 10.0
                        && b.x0 >= bounds.x0 - 1.0
                        && b.x0 <= bounds.x0 + half + 1.0
                })
        })
        .collect();
    found.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    found
}

/// The Button labeled `text` under `label`'s, in its column (a
/// control's choice of a style pair or a numbers toggle).
fn choice_below<'a>(nodes: &'a [accesskit::Node], label: &str, text: &str) -> &'a accesskit::Node {
    widgets_below(nodes, label, Role::Button)
        .into_iter()
        .find(|node| node.label() == Some(text))
        .unwrap_or_else(|| panic!("no {text:?} below {label:?}"))
}

/// The AccessKit id of the `role` node on `label`'s row to its right (a
/// label/field pair's field: `right_of_label` on the node pairs `frame_ids`
/// returns).
fn node_id_right_of(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
    role: Role,
) -> accesskit::NodeId {
    let bounds = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|(_, node)| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
    ids.iter()
        .filter(|(_, node)| node.role() == role && on_row_right_of(bounds, node))
        .min_by(|a, b| {
            let x = |(_, node): &&(accesskit::NodeId, accesskit::Node)| {
                node.bounds().expect("bounds").x0
            };
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {role:?} right of {label:?}"))
}

/// The node of `role` labeled `label`.
fn node_labeled<'a>(nodes: &'a [accesskit::Node], role: Role, label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == role && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {role:?} labeled {label:?}"))
}

/// The lineup rows' name labels in draw order (the 11 starting-eleven
/// rows, then the bench rows): each row's last label is its name, found
/// through the ≡ handles since the fixture's players share names. The
/// name is the label right of the row's position label (≡, position,
/// name) — the rightmost label of the row is no answer on the
/// three-column Tactics frame, where a settings label can share the
/// row's y beside the lists. The two lists sit side by side, so the
/// order is a column's top-to-bottom then the next's (x, then y).
fn lineup_names(nodes: &[accesskit::Node]) -> Vec<&accesskit::Node> {
    let mut handles: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some("≡"))
        .collect();
    handles.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("handle bounds");
            (bounds.x0, bounds.y0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    handles
        .iter()
        .map(|handle| {
            let position = first_right_of(nodes, handle, |node| node.role() == Role::Label)
                .expect("a position label on the lineup row");
            first_right_of(nodes, position, |node| node.role() == Role::Label)
                .expect("a name on the lineup row")
        })
        .collect()
}

/// The pitch canvas' rect (the widget's "pitch" node).
fn canvas_of(nodes: &[accesskit::Node]) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.label() == Some("pitch"))
        .and_then(accesskit::Node::bounds)
        .expect("the pitch canvas")
}

/// `slot`'s badge centre on `canvas` (the widget's spot mapping, fitted
/// on the screen's capture): an outfield slot at (12.8 + 2.827 x,
/// 363.8 - 6.793 y) from the canvas' top-left, high y (the attack end) on
/// top; the keeper at (160, 358.5), where the screen draws him.
fn badge_centre(canvas: accesskit::Rect, slot: FormationSlot) -> egui::Pos2 {
    if slot.position == 0 {
        return egui::pos2(canvas.x0 as f32 + 160.0, canvas.y0 as f32 + 358.5);
    }
    egui::pos2(
        canvas.x0 as f32 + 12.8 + f32::from(slot.x) * 2.827,
        canvas.y0 as f32 + 363.8 - f32::from(slot.y) * 6.793,
    )
}

/// `slot`'s badge centre clamped so the marker's whole box (the badge's
/// 50 width, 57 above and 20 below the centre) stays inside the canvas.
fn clamped_centre(canvas: accesskit::Rect, slot: FormationSlot) -> egui::Pos2 {
    let centre = badge_centre(canvas, slot);
    egui::pos2(
        centre
            .x
            .clamp(canvas.x0 as f32 + 25.0, canvas.x1 as f32 - 25.0),
        centre
            .y
            .clamp(canvas.y0 as f32 + 57.0, canvas.y1 as f32 - 20.0),
    )
}

/// `slot`'s marker box on `canvas` (the badge's width, the portrait's
/// top to the name's bottom).
fn marker_rect(canvas: accesskit::Rect, slot: FormationSlot) -> egui::Rect {
    let centre = clamped_centre(canvas, slot);
    egui::Rect::from_min_max(
        egui::pos2(centre.x - 25.0, centre.y - 57.0),
        egui::pos2(centre.x + 25.0, centre.y + 20.0),
    )
}

/// The indices' draw order (the widget's): markers sort by their clamped
/// badge centre's y, top of the canvas first, and a press or drop hits
/// the marker drawn last.
fn draw_order(canvas: accesskit::Rect, players: &[FormationSlot; 11]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..11).collect();
    order.sort_by(|&a, &b| {
        clamped_centre(canvas, players[a])
            .y
            .total_cmp(&clamped_centre(canvas, players[b]).y)
            .then(a.cmp(&b))
    });
    order
}

/// A point inside `index`'s marker box that no marker drawn after it
/// covers, so a press or a drop there resolves to `index`: the box's
/// centre when free, else the first free of a coarse scan; `None` when
/// the box is fully covered.
fn marker_point(
    canvas: accesskit::Rect,
    players: &[FormationSlot; 11],
    index: usize,
) -> Option<egui::Pos2> {
    let order = draw_order(canvas, players);
    let after = order
        .iter()
        .position(|&i| i == index)
        .expect("every marker is in the order");
    let free = |point: egui::Pos2| {
        order[after + 1..]
            .iter()
            .all(|&other| !marker_rect(canvas, players[other]).contains(point))
    };
    let rect = marker_rect(canvas, players[index]);
    if free(rect.center()) {
        return Some(rect.center());
    }
    (0..=10)
        .flat_map(|ix| {
            (0..=15).map(move |iy| {
                rect.min
                    + egui::vec2(
                        ix as f32 * rect.width() / 10.0,
                        iy as f32 * rect.height() / 15.0,
                    )
            })
        })
        .find(|point| free(*point))
}

/// A click at `pos` (press on one frame, release on the next): the
/// release frame's nodes.
fn click_at(window: &mut Headless, pos: egui::Pos2, view: &mut SaveView) -> Vec<accesskit::Node> {
    press_release(window, pos, |ui| show(ui, view, &|| Vec::new()))
}

/// One frame, its nodes.
fn draw(window: &mut Headless, view: &mut SaveView) -> Vec<accesskit::Node> {
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    nodes
}

/// `text` loaded as an imported ruleset named `file_name` and picked, as
/// `Tool::view` would hand the view a copy read from the data folder's
/// `rules/` and the setting naming it.
fn pick_imported(view: &mut SaveView, file_name: &str, text: &str) {
    let rules = aatf::Rules::load(text).expect("the imported rules load");
    view.imported.push(crate::session::rules::Imported {
        file_name: file_name.to_owned(),
        rules: std::sync::Arc::new(rules),
    });
    view.aatf_ruleset = file_name.to_owned();
}

/// Frames until `landed` holds for the view's worker result (bounded, so
/// a worker that never replies fails rather than hangs); `message` is the
/// deadline's panic. Returns the landing frame's node pairs.
fn frames_until(
    window: &mut Headless,
    view: &mut SaveView,
    landed: impl Fn(&SaveView) -> bool,
    message: &str,
) -> Vec<(accesskit::NodeId, accesskit::Node)> {
    // A hang guard, not a speed bound: a passing test returns when the
    // reply lands, so the bound costs nothing and only stops a worker
    // that never replies (four AATF-panel tests starved past 30 s under
    // a mutation run's below-normal priority, 2026-10-26).
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        if landed(view) {
            return pairs;
        }
        assert!(Instant::now() < deadline, "{message}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Frames until the Live AATF dock's check lands: `pending` empty and the
/// result keyed to the session's current revision, `ids`, the active
/// ruleset and the Show suggestions checkbox (bounded, so a worker that
/// never replies fails rather than hangs).
fn live_checked(window: &mut Headless, view: &mut SaveView, ids: &[u32]) -> Vec<accesskit::Node> {
    frames_until(
        window,
        view,
        |view| {
            let revision = view.session.as_ref().expect("an open save").revision();
            let ruleset = &view
                .rules
                .as_ref()
                .expect("a frame resolved the ruleset")
                .key;
            let check = &view.live.check;
            check.pending.is_none()
                && check.result.as_ref().is_some_and(|(key, _)| {
                    key.revision == revision
                        && key.ids == ids
                        && key.ruleset == *ruleset
                        && key.suggestions == view.live_suggestions
                })
        },
        "the AATF check never returned",
    )
    .into_iter()
    .map(|(_, node)| node)
    .collect()
}

/// The group header `slot`'s player gets in the violations list
/// (`player_head`'s format).
fn player_header(view: &SaveView, team_id: u32, slot: u8) -> String {
    let file = view.session.as_ref().expect("the save opened").file();
    let team = file.team(team_id).expect("the team");
    let roster = &team.roster[usize::from(slot)];
    let player = file.player(roster.player_id).expect("rostered");
    team_widgets::violations::player_head(roster.player_id, roster.number, &player.name)
}

/// The `FieldedNotA` row under `header`'s group (two players can share
/// the row's label, so the group header picks the right one).
fn fielded_row_below<'a>(nodes: &'a [accesskit::Node], header: &str) -> &'a accesskit::Node {
    let header = nodes
        .iter()
        .find(|node| node.value() == Some(header))
        .unwrap_or_else(|| panic!("no header {header:?}"));
    let top = header.bounds().expect("header bounds").y0;
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("FieldedNotA:"))
        })
        .filter(|node| node.bounds().is_some_and(|b| b.y0 > top))
        .min_by(|a, b| {
            let y = |n: &&accesskit::Node| n.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("a FieldedNotA row below the header")
}

/// The Height row under `header`'s group (two players share the row's
/// label, so the group header picks the right one).
fn height_row_below<'a>(nodes: &'a [accesskit::Node], header: &str) -> &'a accesskit::Node {
    let header = nodes
        .iter()
        .find(|node| node.value() == Some(header))
        .unwrap_or_else(|| panic!("no header {header:?}"));
    let top = header.bounds().expect("header bounds").y0;
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node.label() == Some("Height: height 183 takes no bracket (185 has room)")
        })
        .filter(|node| node.bounds().is_some_and(|b| b.y0 > top))
        .min_by(|a, b| {
            let y = |n: &&accesskit::Node| n.bounds().expect("bounds").y0;
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .expect("a Height row below the header")
}

/// The Height-183 rows among `nodes` (the two players' row).
fn height_rows(nodes: &[accesskit::Node]) -> usize {
    nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node.label() == Some("Height: height 183 takes no bracket (185 has room)")
        })
        .count()
}

/// `view` on `version`'s fixture save under `test`, the save open and
/// team 734 selected with players `ids`' heights set to `height` as one
/// sealed edit.
fn open_734_with(test: &str, ids: &[u32], height: u8) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes17, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    let mut edits = Vec::new();
    for &id in ids {
        let mut entry = player_entry(&view, id);
        assert_eq!(entry.basic.height, 185, "premise: {id} is tall");
        entry.basic.height = height;
        edits.push(entry);
    }
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(edits);
    (view, Headless::sized(DEFAULT_PANE.x, 800.0))
}

/// The PES 15 fixture save open with the CLI's `official_legal.toml`
/// imported onto team 714: the one team the official ruleset finds
/// clean (its 32 findings are all suggestions).
fn open_legal_714(test: &str, window: Headless) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes15, test);
    let mut view = SaveView::new();
    view.open(&path);
    let toml = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/aatf/official_legal.toml"
    ))
    .unwrap();
    let document = pes_savefile::interchange::team_toml::TeamToml::parse(&toml).unwrap();
    view.session
        .as_mut()
        .expect("the save opened")
        .import_team(&document)
        .expect("the legal team imports");
    view.team = TeamChoice::Team(714);
    view.selected = Some(Selection::Player(714, 0));
    (view, window)
}

/// The nodes inside the window titled `title`, with their AccessKit ids
/// (its subtree: the player list's rows share the screen). `pairs` is
/// `frame_ids`' output.
fn nodes_in_window<'a>(
    pairs: &'a [(accesskit::NodeId, accesskit::Node)],
    title: &str,
) -> Vec<(accesskit::NodeId, &'a accesskit::Node)> {
    let (window, _) = pairs
        .iter()
        .find(|(_, node)| node.label().is_some_and(|label| label.starts_with(title)))
        .expect("the window");
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut nodes = Vec::new();
    let mut stack = vec![*window];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        nodes.push((id, node));
    }
    nodes
}

/// The window titled `title`'s Label values and Button labels, in
/// top-to-bottom order.
fn window_lines(window: &mut Headless, view: &mut SaveView, title: &str) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let nodes = nodes_in_window(&pairs, title);
    let mut lines: Vec<(f64, f64, String)> = Vec::new();
    for (_, node) in nodes {
        let text = match node.role() {
            Role::Label => node.value().map(str::to_owned),
            _ => node.label().map(str::to_owned),
        };
        if let (Some(text), Some(bounds)) = (text, node.bounds()) {
            lines.push((bounds.y0, bounds.x0, text));
        }
    }
    lines.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("drawn bounds")
            .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
    });
    lines.into_iter().map(|(_, _, text)| text).collect()
}

/// The official file with the CLI's custom check that throws on captain 19
/// (team 743's; `a_check_that_fails_on_one_team_does_not_stop_the_rest`).
fn throw19() -> String {
    aatf::official_file().replacen(
        "fn custom_checks(team) { [] }",
        "fn custom_checks(team) { if team.captain == 19 { throw \"boom\" } [] }",
        1,
    )
}

mod abilities;
mod appearance;
mod comparator;
mod generate;
mod list;
mod live_aatf;
mod live_marks;
mod open_menu;
mod overlap;
mod patch;
mod player;
mod prompt;
mod quick_actions;
mod ruleset_picker;
mod savefile;
mod shell;
mod strip;
mod tactics;
mod tactics_undo;
mod team;
mod toolbar;
