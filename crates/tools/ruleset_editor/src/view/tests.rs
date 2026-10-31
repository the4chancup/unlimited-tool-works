//! The headless-window tests of the view's scenarios: one egui context
//! with AccessKit on, run a frame at a time. What a frame draws is its
//! AccessKit nodes, what the clipboard holds is its platform output, and a
//! mark's colour, which no AccessKit node carries, is read back from the
//! view state the frame paints from.

mod custom_checks;
mod file;
mod general;
mod preview;
mod problems;
mod prompt;
mod start;
mod summary;
mod tiers;
mod universal;

use std::path::{Path, PathBuf};

use crossbeam_channel::Receiver;
use egui::accesskit::{self, Role};
use studio_core::{PipelineEventEnvelope, ToolContext};

use crate::test_support::{valid_file, write_file};

use super::*;

/// A headless window: one egui context with AccessKit on and animations
/// off, run a frame at a time over an 800 by 600 screen, `sized` when a
/// test asks for another (the Save editor's `Headless`).
struct Headless {
    ctx: egui::Context,
    /// The screen size each frame reports.
    screen: egui::Vec2,
}

/// What a frame drew: its AccessKit nodes, its platform output (the
/// clipboard's `CopyText` command) and the shapes it painted (a text's
/// size and colour, which no node carries).
struct Frame {
    nodes: Vec<accesskit::Node>,
    platform: egui::PlatformOutput,
    /// egui asked for the next frame at once (the root viewport's
    /// `repaint_delay` is zero): an animation, a scroll among them, is
    /// still moving.
    repaints: bool,
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Headless {
    fn new() -> Headless {
        Headless::sized(800.0, 600.0)
    }

    /// A window `width` by `height` instead of the default 800 by 600.
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

    /// Runs one frame of `contents`; returns what it drew.
    fn frame(&mut self, events: Vec<egui::Event>, contents: impl FnMut(&mut egui::Ui)) -> Frame {
        self.run(events, true, contents)
    }

    /// One frame of `contents` with the window unfocused (`focused`
    /// off in `RawInput`): an Alt+Tab mid-edit.
    fn unfocused_frame(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Frame {
        self.run(events, false, contents)
    }

    /// Runs one frame of `contents` with `focused` as the window's
    /// focus; returns what it drew.
    fn run(
        &mut self,
        events: Vec<egui::Event>,
        focused: bool,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Frame {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
            events,
            focused,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, contents);
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        let platform = std::mem::take(&mut output.platform_output);
        let repaints = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .expect("every frame produces the root viewport")
            .repaint_delay
            .is_zero();
        let shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are
        // dropped on purpose.
        output.drop_without_applying_deltas();
        Frame {
            nodes: update.nodes.into_iter().map(|(_, node)| node).collect(),
            platform,
            repaints,
            shapes,
        }
    }

    /// One quiet frame of the view.
    fn draw(&mut self, view: &mut RulesetView, ctx: &ToolContext) -> Frame {
        self.frame(Vec::new(), |ui| show(ui, view, ctx))
    }
}

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

/// A click on `node`: press on one frame, release on the next.
fn click(window: &mut Headless, node: &accesskit::Node, view: &mut RulesetView, ctx: &ToolContext) {
    let at = centre(node);
    window.frame(pointer_at(at, true), |ui| show(ui, view, ctx));
    window.frame(pointer_at(at, false), |ui| show(ui, view, ctx));
}

/// Draws two frames and returns the second: a `Grid`'s first draw is
/// an invisible sizing pass with degenerate bounds, so node lookups
/// and clicks run on the frame after it.
fn settled(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext) -> Frame {
    window.draw(view, ctx);
    window.draw(view, ctx)
}

/// A click on the `nth` node labeled `label`, top to bottom (the two
/// stat checklists share names).
fn click_nth(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    label: &str,
    nth: usize,
    view: &mut RulesetView,
    ctx: &ToolContext,
) {
    click(window, nodes_labeled(nodes, label)[nth], view, ctx);
}

/// A click on the node `nodes` holds labeled exactly `label`.
fn click_labeled(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    label: &str,
    view: &mut RulesetView,
    ctx: &ToolContext,
) {
    click(window, node(nodes, label), view, ctx);
}

/// A click on the first node whose label starts with `label`, as drawn
/// on a fresh frame (menu items carry their shortcut in the label).
fn click_drawn(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, label: &str) {
    let frame = window.draw(view, ctx);
    click_prefix(window, &frame.nodes, label, view, ctx);
}

/// `click_drawn` for the `nth` of several same-labeled nodes.
fn click_drawn_nth(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    label: &str,
    nth: usize,
) {
    let frame = window.draw(view, ctx);
    click_nth(window, &frame.nodes, label, nth, view, ctx);
}

/// File's "New from template…" opens the start page over the document
/// at once — nothing is lost there, so no prompt.
fn open_start_modal(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext) {
    menu_click(window, view, ctx, "File", "New from template…");
    window.draw(view, ctx);
    assert!(view.start_open, "the start page opened");
    assert!(
        view.pending_action.is_none(),
        "no prompt for the page itself"
    );
}

/// The toolbar's `menu` button's label, as `studio_core::widgets::menu_button`
/// draws it: the name, then the arrow.
fn menu_label(menu: &str) -> String {
    format!("{menu} \u{23f7}")
}

/// `menu` opened, then its `item` clicked (a menu's items live on the
/// frame after its button).
fn menu_click(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    menu: &str,
    item: &str,
) {
    let frame = window.draw(view, ctx);
    click_labeled(window, &frame.nodes, &menu_label(menu), view, ctx);
    let frame = window.draw(view, ctx);
    click_prefix(window, &frame.nodes, item, view, ctx);
}

/// A click on the node whose label starts with `prefix` (menu items
/// carry their shortcut in the label).
fn click_prefix(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    prefix: &str,
    view: &mut RulesetView,
    ctx: &ToolContext,
) {
    let target = nodes
        .iter()
        .find(|node| node.label().is_some_and(|label| label.starts_with(prefix)))
        .unwrap_or_else(|| panic!("no node labeled {prefix:?}…"));
    click(window, target, view, ctx);
}

/// A key pressed with `modifiers` held.
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        modifiers,
        repeat: false,
    }
}

/// A Ctrl+`pressed` press sent to the view for one frame.
fn ctrl_key(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, pressed: egui::Key) {
    window.frame(vec![key(pressed, egui::Modifiers::CTRL)], |ui| {
        show(ui, view, ctx)
    });
}

/// The topmost node labeled (or valued) `label`: a label can repeat —
/// a modal's "Save" shares it with the bottom bar's behind the
/// backdrop — and the centred modal's is always the higher one.
fn node<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes_labeled(nodes, label)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no node labeled {label:?}"))
}

/// `found` ordered top to bottom by each node's top edge; every node
/// here is drawn, so a missing bound is a broken test, not a sort key.
fn sort_top_down(found: &mut [&accesskit::Node]) {
    let y = |node: &&accesskit::Node| node.bounds().expect("a drawn node has bounds").y0;
    found.sort_by(|a, b| y(a).partial_cmp(&y(b)).expect("ordered bounds"));
}

/// Every node labeled `label` top to bottom (the two stat checklists
/// share names).
fn nodes_labeled<'a>(nodes: &'a [accesskit::Node], label: &str) -> Vec<&'a accesskit::Node> {
    let mut found: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.label() == Some(label) || node.value() == Some(label))
        .collect();
    sort_top_down(&mut found);
    found
}

/// Every node's label (or value), top to bottom then left to right
/// (the AccessKit update is unordered; a node without bounds sorts
/// last).
fn labels(nodes: &[accesskit::Node]) -> Vec<String> {
    let mut labelled: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.label().is_some() || node.value().is_some())
        .collect();
    labelled.sort_by(|a, b| {
        let at = |node: &&accesskit::Node| {
            node.bounds()
                .map(|rect| (rect.y0, rect.x0))
                .unwrap_or((f64::MAX, f64::MAX))
        };
        at(a).partial_cmp(&at(b)).expect("ordered bounds")
    });
    labelled
        .iter()
        .filter_map(|node| node.label().or_else(|| node.value()).map(str::to_owned))
        .collect()
}

/// The rail's section entries in drawn order (an entry carries a `⚠`
/// inside the label; a section's own heading is not one).
fn rail_labels(nodes: &[accesskit::Node]) -> Vec<String> {
    let section_names: Vec<&str> = Section::RAIL.iter().map(|s| s.label()).collect();
    let mut found: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|text| section_names.iter().any(|name| text.starts_with(name)))
        })
        .collect();
    sort_top_down(&mut found);
    found
        .iter()
        .filter_map(|node| node.label().map(str::to_owned))
        .collect()
}

/// Whether a node labeled `label` was drawn.
fn drawn(nodes: &[accesskit::Node], label: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.label() == Some(label) || node.value() == Some(label))
}

/// Whether a label (or value) containing `fragment` was drawn.
fn contains_label(nodes: &[accesskit::Node], fragment: &str) -> bool {
    labels(nodes).iter().any(|label| label.contains(fragment))
}

/// `view`'s Save as dialog stubbed to return `path`, then File's "Save
/// as…" clicked (a real dialog cannot be clicked headless).
fn save_as_via_menu(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, path: &Path) {
    view.save_as_dialog = save_as_picking(path.to_path_buf());
    menu_click(window, view, ctx, "File", "Save as…");
}

/// The Save as dialog stub returning `path`.
fn save_as_picking(path: PathBuf) -> SaveDialog {
    Box::new(move |_, _| Some(path.clone()))
}

/// The Open dialog stub asserting it was started in `expected_start`,
/// then returning `picked`.
fn open_picking(expected_start: PathBuf, picked: PathBuf) -> FilePicker {
    Box::new(move |start| {
        assert_eq!(
            start,
            expected_start.as_path(),
            "the Open dialog starts in the expected folder"
        );
        Some(picked.clone())
    })
}

/// `view` with Blank opened and renamed "Edited": a never-saved, dirty
/// document.
fn blank_dirty(view: &mut RulesetView) {
    open_document(view, crate::document::blank());
    edit_ruleset(view, |ruleset| ruleset.name = "Edited".to_string());
}

/// The tool's settings seeded through the typed projection (the file.rs
/// settings tests seed raw tables on purpose).
fn seed_settings(ctx: &ToolContext, last_folder: &Path, recent_files: Vec<String>) {
    ctx.set_tool_settings(
        crate::messages::TOOL_ID,
        crate::settings::RulesetEditorSettings {
            last_folder: last_folder.to_string_lossy().into_owned(),
            recent_files,
        }
        .to_table(),
    );
}

/// Up to 20 frames of a wheel scroll by `delta` with the pointer at `at`
/// (over the area to scroll: the wheel lands on the layer under the
/// pointer), until `done` holds for a frame; that frame, or `None` when
/// the scroll never got there.
fn scroll_until(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    at: egui::Pos2,
    delta: egui::Vec2,
    done: impl Fn(&Frame) -> bool,
) -> Option<Frame> {
    for _ in 0..20 {
        let frame = window.frame(
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Trackpad,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, view, ctx),
        );
        if done(&frame) {
            return Some(frame);
        }
    }
    None
}

/// `path`'s bytes are `before` still (a write left it alone).
fn assert_file_unchanged(path: &Path, before: &[u8]) {
    assert_eq!(
        std::fs::read(path).unwrap(),
        before,
        "byte for byte as it was"
    );
}

/// `test`'s scratch folder with `valid_file()` written to `name`
/// inside; folder and file.
fn scratch_file(test: &str, name: &str) -> (PathBuf, PathBuf) {
    let dir = crate::test_support::scratch(test);
    let file = crate::test_support::write_file(&dir, name, &crate::test_support::valid_file());
    (dir, file)
}

/// A view with `ruleset` opened as a new, unsaved document (a template
/// or Blank pick's result without the pick).
fn open_document(view: &mut RulesetView, ruleset: aatf::Ruleset) {
    view.set_document(Document::new(ruleset));
}

/// `view`'s document's ruleset edited in place: the tests' shortcut for
/// a change that marks the document dirty (the undo gesture itself is
/// exercised by the checkbox clicks).
fn edit_ruleset(view: &mut RulesetView, edit: impl FnOnce(&mut aatf::Ruleset)) {
    edit(&mut view.document.as_mut().expect("a document").ruleset);
}

/// A fresh context, headless window and view.
fn fresh() -> (
    ToolContext,
    Receiver<PipelineEventEnvelope>,
    Headless,
    RulesetView,
) {
    let (ctx, events) = crate::test_support::context();
    (ctx, events, Headless::new(), RulesetView::new())
}

/// `fresh()` with Blank opened as the document.
fn fresh_blank() -> (
    ToolContext,
    Receiver<PipelineEventEnvelope>,
    Headless,
    RulesetView,
) {
    let (ctx, events, window, mut view) = fresh();
    open_document(&mut view, crate::document::blank());
    (ctx, events, window, view)
}

/// `view` with `path` opened.
fn opened_view(ctx: &ToolContext, path: &Path) -> RulesetView {
    let mut view = RulesetView::new();
    open_path(&mut view, ctx, path);
    view
}

/// `test`'s scratch `rules.4ccrules` opened in a fresh view: context,
/// file, view.
fn official_opened(
    test: &str,
) -> (
    ToolContext,
    Receiver<PipelineEventEnvelope>,
    PathBuf,
    RulesetView,
) {
    let (_dir, path) = scratch_file(test, "rules.4ccrules");
    let (ctx, events) = crate::test_support::context();
    let view = opened_view(&ctx, &path);
    (ctx, events, path, view)
}

/// The view's document's ruleset is named `name`.
fn assert_name(view: &RulesetView, name: &str) {
    assert_eq!(
        view.document.as_ref().expect("a document").ruleset.name,
        name
    );
}

/// `dir`'s `file` was written with `name` as the ruleset's name.
fn assert_saved(dir: &Path, file: &str, name: &str) {
    let text = std::fs::read_to_string(dir.join(file)).unwrap();
    assert!(
        text.contains(&format!("name: \"{name}\"")),
        "the file was written first"
    );
}
