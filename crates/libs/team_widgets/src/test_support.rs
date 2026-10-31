//! The headless egui harness the crate's widget tests share: a context with
//! AccessKit on, run a frame at a time over an 800 by 600 screen, plus the
//! click and painted-text helpers.

use egui::accesskit::{self, Role};
use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use pes_savefile::model::tactics::TeamTactics;
use pes_version::PesVersion;

use crate::tactics::{
    RosterRow, TacticsView, advanced_instructions, automatic_settings, formation_settings,
    player_assignments,
};

/// The four panels in the draft's order, for tests that drive them
/// together (the tests are its only callers): Formation settings,
/// Advanced instructions, Player assignments and Automatic settings.
/// `roster` gives each roster slot's row, indexed by slot. The two
/// row-2 panels are drawn in a cell the pitch's height, as the host
/// cells them. Answers `true` only when `tactics` changed this frame.
pub(crate) fn tactics_settings(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) -> bool {
    ui.push_id(id_salt, |ui| {
        // A cell the pitch's height (the host's row 2), for the panels
        // that fill it.
        fn cell(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), crate::pitch::PITCH_BOX.y),
                egui::Layout::top_down(egui::Align::LEFT),
                add,
            )
            .inner
        }
        // `|` evaluates every panel: a `||` chain would skip the rest on
        // the frame the first reports a change.
        cell(ui, |ui| formation_settings(ui, tactics, view))
            | advanced_instructions(ui, version, tactics, view, roster)
            | cell(ui, |ui| {
                player_assignments(ui, tactics, view, roster);
                false
            })
            | automatic_settings(ui, version, tactics)
    })
    .inner
}

/// A headless window: one egui context with AccessKit on, run a frame at a
/// time over an 800 by 600 screen, for drawing a bare `Ui`'s widgets.
pub(crate) struct Headless {
    /// The context: the painted-color assertions read the theme's visuals
    /// off it.
    pub(crate) ctx: egui::Context,
    /// The screen size each frame reports.
    screen: egui::Vec2,
    /// The root viewport's `repaint_delay` of the last run frame.
    repaint_delay: std::time::Duration,
    /// The flag the pitch's frames hand `pitch`: whether the active
    /// ruleset allows illegal formations. Off unless a test sets it.
    pub(crate) illegal_formations: bool,
}

impl Headless {
    /// A window with AccessKit enabled.
    pub(crate) fn new() -> Headless {
        Headless::sized(800.0, 600.0)
    }

    /// `new` over a `width` by `height` screen.
    pub(crate) fn sized(width: f32, height: f32) -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // The window draws with these fonts and style; the test measures
        // what a member sees, whichever crates the build unifies.
        studio_core::theme::install(&ctx);
        Headless {
            ctx,
            screen: egui::vec2(width, height),
            repaint_delay: std::time::Duration::MAX,
            illegal_formations: false,
        }
    }

    /// The root viewport's `repaint_delay` of the last run frame.
    pub(crate) fn repaint_delay(&self) -> std::time::Duration {
        self.repaint_delay
    }

    /// Runs one frame with `events` as its input; returns the frame's
    /// AccessKit nodes and painted shapes.
    pub(crate) fn frame(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        self.frame_at(None, events, contents)
    }

    /// `frame` with `time` as the frame's timestamp (a tooltip's delay counts
    /// in seconds of it).
    pub(crate) fn frame_at(
        &mut self,
        time: Option<f64>,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        let (nodes, shapes) = self.frame_at_ids(time, events, contents);
        (nodes.into_iter().map(|(_, node)| node).collect(), shapes)
    }

    /// `frame_at` keeping the nodes' AccessKit ids (for
    /// `AccessKitActionRequest` events, which target a node by id).
    pub(crate) fn frame_at_ids(
        &mut self,
        time: Option<f64>,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> (
        Vec<(accesskit::NodeId, accesskit::Node)>,
        Vec<egui::epaint::ClippedShape>,
    ) {
        let input = egui::RawInput {
            time,
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
            events,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, contents);
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        let shapes = std::mem::take(&mut output.shapes);
        self.repaint_delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(std::time::Duration::MAX, |out| out.repaint_delay);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        (update.nodes, shapes)
    }
}

/// Team 701's `TeamTactics` of `version`'s fixture save.
pub(crate) fn fixture_tactics(version: PesVersion) -> TeamTactics {
    EditFile::from_bytes(&pes_savefile::fixture_save(version))
        .expect("fixture save")
        .team(701)
        .expect("team 701 is in the fixture")
        .tactics
        .clone()
}

/// Team 701's roster as the widgets' `roster` sees it, indexed by slot:
/// `(display name, raw name, registered position)`, `("", "", None)` an
/// empty slot, as the hosts build it.
pub(crate) fn fixture_rows(version: PesVersion) -> Vec<(String, String, Option<u8>)> {
    let file = EditFile::from_bytes(&pes_savefile::fixture_save(version)).expect("fixture save");
    let team = file.team(701).expect("team 701 is in the fixture");
    team.roster
        .iter()
        .map(|slot| {
            file.player(slot.player_id).map_or_else(
                || (String::new(), String::new(), None),
                |player| {
                    (
                        display_name(&player.name),
                        player.name.clone(),
                        Some(player.positions.registered),
                    )
                },
            )
        })
        .collect()
}

/// `rows` as the `&[RosterRow]` the widgets take.
pub(crate) fn roster_rows(rows: &[(String, String, Option<u8>)]) -> Vec<RosterRow<'_>> {
    rows.iter()
        .map(|(name, raw_name, position)| RosterRow {
            name: name.as_str(),
            raw_name: raw_name.as_str(),
            registered_position: *position,
            portrait: None,
            medal: None,
            overalls: None,
            rated: [false; 13],
            fielded: &[],
        })
        .collect()
}

/// Every role and the three join entries assigned to roster slots, so a
/// test that counts buttons by their text is not polluted by
/// "Unassigned" chips.
pub(crate) fn named_assignments(tactics: &mut TeamTactics) {
    let pieces = &mut tactics.set_pieces;
    pieces.free_kick_long = 0;
    pieces.free_kick_short = 1;
    pieces.free_kick_second = 2;
    pieces.corner_left = 3;
    pieces.corner_right = 4;
    pieces.penalty = 5;
    pieces.captain = 6;
    tactics.players_to_join_attack = [7, 8, 9];
}

/// The widget node of `role` labeled `label`.
pub(crate) fn entry<'a>(
    nodes: &'a [accesskit::Node],
    role: Role,
    label: &str,
) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == role && node.label() == Some(label))
        .unwrap_or_else(|| panic!("no {role:?} labeled {label:?}"))
}

/// The nodes of `role` right of `label`'s, on its row, in x order.
pub(crate) fn controls_right_of<'a>(
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
        .filter(|node| {
            node.role() == role
                && node
                    .bounds()
                    .is_some_and(|b| (b.y0 - bounds.y0).abs() < 4.0 && b.x0 >= bounds.x1 - 1.0)
        })
        .collect();
    found.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    found
}

/// The first node of `role` right of `label`'s, on its row.
pub(crate) fn control_right_of<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
) -> &'a accesskit::Node {
    controls_right_of(nodes, label, role)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no {role:?} right of {label:?}"))
}

/// The widget nodes of `role` under `label`'s, in its column and row
/// (a Formation settings cell's control: the label over its control).
/// `half` bounds the column's right edge (`controls` gives it).
pub(crate) fn controls_below<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
    half: f64,
) -> Vec<&'a accesskit::Node> {
    let bounds = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
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
        let key = |node: &&accesskit::Node| {
            let b = node.bounds().expect("bounds");
            (b.y0, b.x0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    found
}

/// The first widget node of `role` under `label`'s, in its column.
pub(crate) fn control_below<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    role: Role,
    half: f64,
) -> &'a accesskit::Node {
    controls_below(nodes, label, role, half)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no {role:?} below {label:?}"))
}

/// The chips under `label`'s, top to bottom (each `.tset`'s chip shares
/// its label's left edge — the Player assignments' rows).
pub(crate) fn chips_below<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
) -> Vec<&'a accesskit::Node> {
    let bounds = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
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

/// The instruction grid's combo row `index`'s two combos, attack then
/// defence (each in its own column, on one y — the substitution combo
/// sorts after both rows).
pub(crate) fn combo_row(nodes: &[accesskit::Node], index: usize) -> [&accesskit::Node; 2] {
    let mut combos: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .collect();
    combos.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("combo bounds");
            (bounds.y0, bounds.x0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    let pair = combos
        .get(2 * index..=2 * index + 1)
        .unwrap_or_else(|| panic!("no instruction combo row {index}"));
    [pair[0], pair[1]]
}

/// The chip under `combo` in its column (an instruction's target chip:
/// the Button sharing the combo's left edge, nearest below it).
pub(crate) fn chip_under<'a>(
    nodes: &'a [accesskit::Node],
    combo: &accesskit::Node,
) -> &'a accesskit::Node {
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

/// `nodes`' buttons' labels in draw order (top to bottom, left to
/// right), kept only when `keep` holds.
pub(crate) fn button_labels(nodes: &[accesskit::Node], keep: impl Fn(&str) -> bool) -> Vec<String> {
    let mut buttons: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.label().is_some_and(&keep))
        .collect();
    buttons.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("drawn bounds");
            (bounds.y0, bounds.x0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    buttons
        .iter()
        .filter_map(|node| node.label().map(str::to_owned))
        .collect()
}

/// The AccessKit id of the widget node of `role` labeled `label`.
pub(crate) fn entry_id(
    nodes: &[(accesskit::NodeId, accesskit::Node)],
    role: Role,
    label: &str,
) -> accesskit::NodeId {
    nodes
        .iter()
        .find(|(_, node)| node.role() == role && node.label() == Some(label))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {role:?} labeled {label:?}"))
}

/// The centre of `node`'s bounds, where a click lands.
pub(crate) fn centre(node: &accesskit::Node) -> egui::Pos2 {
    let bounds = node.bounds().expect("a widget has bounds");
    egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    )
}

/// The primary button pressed (or released) at `pos`, the pointer moved
/// there first.
pub(crate) fn pointer_at(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
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

/// The color `shape`'s text paints in: a section's own color when one was
/// set, else the shape's fallback.
pub(crate) fn painted_color(shape: &egui::epaint::TextShape) -> egui::Color32 {
    if let Some(overridden) = shape.override_text_color {
        return overridden;
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

/// The painted text shape whose galley text starts with `starts_with`.
pub(crate) fn text_shape<'a>(
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

/// The color the text `text` was painted in this frame.
pub(crate) fn painted_text_color(
    shapes: &[egui::epaint::ClippedShape],
    text: &str,
) -> egui::Color32 {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text => Some(painted_color(shape)),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted text {text:?}"))
}
