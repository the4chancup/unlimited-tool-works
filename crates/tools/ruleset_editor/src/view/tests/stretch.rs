//! The view in a window larger than the default's: it keeps the layout it
//! has at `DEFAULT_PANE`, centred across the pane — the bars, the rail and
//! the section — the rail and the section alone taking the extra height.
//! With the preview on, the bars span the pane and the preview stands at
//! its right edge at its fixed width (the data block's 100 columns beside
//! its scroll bar's strip); the rail and the section keep the width they
//! have without it, centred in the room it leaves where that is enough,
//! or from its left edge where it is not.

use std::collections::{HashMap, HashSet};

use super::*;

/// The default window's pane.
const DEFAULT_PANE: egui::Vec2 = egui::vec2(1224.0, 756.0);

/// A maximized window's pane, 640 points wider and 229 taller than
/// `DEFAULT_PANE`, so a kept place stands 320 right of its default spot.
const WIDE_PANE: egui::Vec2 = egui::vec2(1864.0, 985.0);

/// A pane wide enough for the room left of the preview to hold the rail
/// and the section at their default width — they stand centred in it
/// there, the check `WIDE_PANE` is too narrow for, since the preview's
/// fixed width leaves them less than that width.
const HUGE_PANE: egui::Vec2 = egui::vec2(2504.0, 985.0);

/// The shift a kept place takes at `WIDE_PANE`: half its extra width.
const SHIFT: f64 = 320.0;

/// The drop a bottom-bar place takes at `WIDE_PANE`: the pane's whole
/// extra height (the bar stays at its bottom).
const DOWN: f64 = 229.0;

/// The drop a pane-centred place takes at `WIDE_PANE`: half the extra
/// height (the start page's card centres in the pane).
const HALF_DOWN: f64 = 114.5;

/// Below this `y0` a no-document frame holds only the start page's card
/// (the toolbar's row ends short of it).
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

/// `view` drawn at `pane`: the frame's AccessKit nodes with their ids and
/// its painted shapes, once the screens' own measurement pass has settled.
fn draw_at(
    view: &mut RulesetView,
    ctx: &ToolContext,
    pane: egui::Vec2,
) -> (Pairs, Frame, egui::Context) {
    let mut window = Headless::sized(pane.x, pane.y);
    window.frame_ids(Vec::new(), |ui| show(ui, view, ctx));
    let (pairs, frame) = window.frame_ids(Vec::new(), |ui| show(ui, view, ctx));
    (pairs, frame, window.ctx)
}

/// A frame's AccessKit nodes with their ids.
type Pairs = Vec<(accesskit::NodeId, accesskit::Node)>;

/// `pairs` keyed by node id.
fn by_id(pairs: &Pairs) -> HashMap<accesskit::NodeId, &accesskit::Node> {
    pairs.iter().map(|(id, node)| (*id, node)).collect()
}

/// The innermost `Pane` node over a node `inside` accepts (the rail's pane
/// over a rail entry, the preview's over its title), `None` when no node
/// matches.
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

/// `default`'s labeled, bounded nodes that no `wide` node of the same role
/// and label stands `(SHIFT, down)` from at the same size — each misfit
/// one line. `down` answers the node's own drop (the bottom bar's `DOWN`,
/// a centred card's `HALF_DOWN`).
fn misfits(
    default: &Pairs,
    wide: &Pairs,
    down: impl Fn(&accesskit::NodeId, &accesskit::Node) -> f64,
) -> Vec<String> {
    default
        .iter()
        .filter(|(_, node)| node.bounds().is_some() && text(node).is_some())
        .filter_map(|(id, node)| {
            let bounds = node.bounds().expect("bounded above");
            let label = text(node).expect("labeled above");
            let moved = wide.iter().any(|(_, candidate)| {
                candidate.role() == node.role()
                    && text(candidate).as_deref() == Some(label.as_str())
                    && candidate
                        .bounds()
                        .is_some_and(|at| shifted(bounds, at, down(id, node)))
            });
            (!moved).then(|| format!("{:?} {label:?} at {bounds:?}", node.role()))
        })
        .collect()
}

/// A node's drop at `WIDE_PANE`: `DOWN` in the bottom bar, `HALF_DOWN`
/// on a pane-centred card, `0` else.
type Down = Box<dyn Fn(&accesskit::NodeId, &accesskit::Node) -> f64>;

/// The `down` builder for a document screen: `DOWN` on the bottom bar's
/// nodes (its pane's subtree, found over its **Save** button), `0` else.
fn document_down(default: &Pairs) -> Down {
    let bottom = subtree(
        default,
        pane_over(default, |node| {
            node.role() == Role::Button && node.label() == Some("Save")
        }),
    );
    Box::new(move |id, _| if bottom.contains(id) { DOWN } else { 0.0 })
}

/// The `down` builder for the start page: `HALF_DOWN` on the card's
/// nodes (the ones below the toolbar's row), `0` else.
fn start_down(_: &Pairs) -> Down {
    Box::new(|_, node| {
        if node.bounds().is_some_and(|bounds| bounds.y0 > PANES_TOP) {
            HALF_DOWN
        } else {
            0.0
        }
    })
}

/// `screen`'s frame at `DEFAULT_PANE` and `WIDE_PANE`, with the cap
/// asserted: every labeled, bounded node of the default frame stands
/// `SHIFT` right in the wide one at the same size, its own `down` down.
/// `down` is built on the default frame (a subtree's ids live in it).
/// Returns both frames' nodes, shapes and the last context.
fn drawn_pair(
    view: &mut RulesetView,
    ctx: &ToolContext,
    screen: &str,
    down: impl FnOnce(&Pairs) -> Down,
) -> (Pairs, Pairs, Frame, Frame, egui::Context) {
    let (default, default_frame, _) = draw_at(view, ctx, DEFAULT_PANE);
    let (wide, wide_frame, ectx) = draw_at(view, ctx, WIDE_PANE);
    let found = misfits(&default, &wide, down(&default));
    assert!(
        found.is_empty(),
        "{screen}: {} of the default frame's nodes do not keep their place and size:\n{}",
        found.len(),
        found.join("\n")
    );
    (default, wide, default_frame, wide_frame, ectx)
}

/// A view with Autumn 26 open on `section`.
fn autumn_view(section: Section) -> RulesetView {
    let mut view = RulesetView::new();
    let autumn = aatf::templates()
        .into_iter()
        .find(|template| template.ruleset.name == "4cc Autumn 26")
        .expect("the Autumn 26 template")
        .ruleset;
    open_document(&mut view, autumn);
    view.section = section;
    view
}

/// The painted panel-fill rect `pred` picks in `frame`.
fn panel_rect(
    frame: &Frame,
    ectx: &egui::Context,
    pred: impl Fn(&egui::Rect) -> bool,
) -> egui::Rect {
    let fills = panel_fill_at(frame, ectx, pred);
    fills.first().copied().unwrap_or_else(|| {
        let all = panel_fill_at(frame, ectx, |_| true);
        panic!("the picked panel's fill, of {all:?}")
    })
}

/// The rail's painted frame in `frame`: the 150-point panel's fill, a
/// point narrower for the separator standing in its outer rect.
fn rail_rect(frame: &Frame, ectx: &egui::Context) -> egui::Rect {
    panel_rect(frame, ectx, |rect| {
        (rect.width() - 150.0).abs() < 1.5 && rect.height() > 100.0
    })
}

/// The section's painted frame in `frame` with the preview shown: the
/// tall panel right of the rail, short of the pane's right edge, the
/// preview's edge left to it.
fn section_rect(frame: &Frame, ectx: &egui::Context, pane_right: f32) -> egui::Rect {
    panel_rect(frame, ectx, |rect| {
        rect.height() > 100.0 && rect.width() > 200.0 && rect.right() < pane_right - 1.0
    })
}

/// The preview's frame in `frame`: the tall panel on the pane's right
/// edge between the bars (the harness's own panel fill spans the whole
/// pane, so `top`/`bottom` rule it out, as `preview_left`'s do). Its
/// separator stands a point inside the frame's left edge, so the frame
/// is a point wider than the fill.
fn preview_rect(frame: &Frame, ectx: &egui::Context, pane: egui::Vec2) -> egui::Rect {
    let fill = panel_rect(frame, ectx, |rect| {
        rect.right() == pane.x && rect.top() > 0.0 && rect.bottom() < pane.y
    });
    egui::Rect::from_min_max(egui::pos2(fill.left() - 1.0, fill.top()), fill.max)
}

/// The start page card's painted frame in `frame`: the one window-fill
/// rect (a dialog's `modal_frame` shares the fill, none is open here).
fn card_rect(frame: &Frame, ectx: &egui::Context) -> egui::Rect {
    let fill = ectx.global_style().visuals.window_fill();
    frame
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
            _ => None,
        })
        .next()
        .expect("the card's frame")
}

/// The preview's expected width: 100 columns of the drawn block — the
/// `page:` line's bounds over its characters (long enough to give a fine
/// column and short enough not to wrap) — beside the scroll bar's strip
/// and inside the panel's margins and its separator line's width.
fn preview_width(frame: &Frame, ectx: &egui::Context) -> f64 {
    let (chars, ink) = frame
        .nodes
        .iter()
        .filter(|node| node.role() == Role::Label)
        .find_map(|node| {
            let value = node.value()?;
            if !value.starts_with("    page:") {
                return None;
            }
            node.bounds().map(|b| (value.chars().count(), b.x1 - b.x0))
        })
        .expect("the block's page: line");
    let column = ink / chars as f64;
    f64::from(
        egui::Frame::side_top_panel(&ectx.global_style())
            .inner_margin
            .sum()
            .x,
    ) + 100.0 * column
        + f64::from(egui::style::ScrollStyle::solid().allocated_width())
        + f64::from(
            ectx.global_style()
                .visuals
                .widgets
                .noninteractive
                .bg_stroke
                .width
                .round() as i8,
        )
}

#[test]
fn the_start_page_keeps_its_card_centred_in_a_wider_pane() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = RulesetView::new();
    let (_, _, default_frame, wide_frame, ectx) =
        drawn_pair(&mut view, &ctx, "the start page", start_down);

    // The card itself centres in the pane at both sizes.
    for (frame, pane) in [(&default_frame, DEFAULT_PANE), (&wide_frame, WIDE_PANE)] {
        let centre = card_rect(frame, &ectx).center();
        assert!(
            (f64::from(centre.x - pane.x / 2.0)).abs() <= SLACK
                && (f64::from(centre.y - pane.y / 2.0)).abs() <= SLACK,
            "the card centres in the {pane:?} pane ({centre:?})"
        );
    }
}

#[test]
fn the_general_section_keeps_its_layout_centred_in_a_wider_pane() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = autumn_view(Section::General);
    let (_, _, default_frame, wide_frame, ectx) =
        drawn_pair(&mut view, &ctx, "the General section", document_down);
    // The rail's frame takes the pane's whole extra height.
    let (from, at) = (
        rail_rect(&default_frame, &ectx),
        rail_rect(&wide_frame, &ectx),
    );
    assert!(
        (f64::from(at.height() - from.height()) - DOWN).abs() <= SLACK,
        "the rail's frame is {DOWN} taller at {WIDE_PANE:?} ({from:?} vs {at:?})"
    );
}

#[test]
fn the_tiers_section_keeps_its_layout_centred_in_a_wider_pane() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = autumn_view(Section::Tiers);
    drawn_pair(&mut view, &ctx, "the Tiers section", document_down);
}

#[test]
fn the_summary_keeps_its_lines_centred_in_a_wider_pane() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = autumn_view(Section::Summary);
    let (default, wide, _, _, _) =
        drawn_pair(&mut view, &ctx, "the Summary section", document_down);

    // The reflow check itself: the prose's label keeps its size — the
    // same lines at the same width — and only stands SHIFT right.
    let prose = |pairs: &Pairs| {
        pairs
            .iter()
            .map(|(_, node)| node)
            .find(|node| {
                node.role() == Role::Label && node.value().is_some_and(|text| text.len() > 500)
            })
            .and_then(|node| node.bounds())
            .expect("the summary's prose")
    };
    let (from, at) = (prose(&default), prose(&wide));
    assert!(
        shifted(from, at, 0.0),
        "the prose keeps its lines at {WIDE_PANE:?} ({from:?} vs {at:?})"
    );
}

#[test]
fn the_preview_stands_at_the_panes_right_edge_at_its_fixed_width() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = autumn_view(Section::General);
    view.show_preview = true;
    let (_, default_frame, default_ectx) = draw_at(&mut view, &ctx, DEFAULT_PANE);
    let (_, wide_frame, wide_ectx) = draw_at(&mut view, &ctx, WIDE_PANE);
    let (_, huge_frame, huge_ectx) = draw_at(&mut view, &ctx, HUGE_PANE);

    // At both sizes the preview's frame is its fixed width — the data
    // block's 100 columns beside its scroll bar's strip — with its right
    // edge on the pane's right edge.
    let width = preview_width(&default_frame, &default_ectx);
    for (frame, ectx, pane) in [
        (&default_frame, &default_ectx, DEFAULT_PANE),
        (&wide_frame, &wide_ectx, WIDE_PANE),
    ] {
        let preview = preview_rect(frame, ectx, pane);
        assert!(
            (f64::from(preview.width()) - width).abs() <= SLACK && preview.right() == pane.x,
            "the preview keeps its fixed {width} on the {pane:?} pane's right edge ({preview:?})"
        );
    }

    // At the default size the rail stands at the room's left edge, the
    // section taking what is left of it; `WIDE_PANE` leaves the room
    // narrower than the two still, so it is the same there.
    for (frame, ectx, pane) in [
        (&default_frame, &default_ectx, DEFAULT_PANE),
        (&wide_frame, &wide_ectx, WIDE_PANE),
    ] {
        let preview = preview_rect(frame, ectx, pane);
        let rail = rail_rect(frame, ectx);
        assert!(
            rail.left() <= SLACK as f32,
            "the rail stands at the room's left edge on the {pane:?} pane ({rail:?})"
        );
        let section = section_rect(frame, ectx, pane.x);
        assert!(
            (section.right() - preview.left()).abs() <= SLACK as f32,
            "the section fills the room to the preview on the {pane:?} pane ({section:?} vs {preview:?})"
        );
    }

    // At `HUGE_PANE` the room holds the rail and the section at their
    // width without the preview: they stand centred in it, the same gap
    // on each side. The room spans from the pane's left edge to the
    // preview's left edge.
    let huge_preview = preview_rect(&huge_frame, &huge_ectx, HUGE_PANE);
    let gap = (huge_preview.left() - DEFAULT_PANE.x) / 2.0;
    assert!(
        gap > SLACK as f32,
        "premise: the room holds the capped width"
    );
    let rail = rail_rect(&huge_frame, &huge_ectx);
    let section = section_rect(&huge_frame, &huge_ectx, HUGE_PANE.x);
    assert!(
        (rail.left() - gap).abs() <= SLACK as f32
            && (section.right() - (huge_preview.left() - gap)).abs() <= SLACK as f32,
        "the rail and the section stand centred in the room at {HUGE_PANE:?} ({rail:?} vs {section:?})"
    );
}

#[test]
fn a_hundred_column_line_stands_on_one_row() {
    let (ctx, _events) = crate::test_support::context();
    let mut view = RulesetView::new();
    open_document(&mut view, crate::document::blank());
    // A name that makes the block's `name:` line exactly 100 characters:
    // `    name: "` is 11 and `",` is 2.
    edit_ruleset(&mut view, |ruleset| ruleset.name = "x".repeat(87));
    view.show_preview = true;
    let (_, frame, _) = draw_at(&mut view, &ctx, DEFAULT_PANE);
    let name = frame
        .nodes
        .iter()
        .find(|node| node.value().is_some_and(|v| v.starts_with("    name:")))
        .expect("the name: line")
        .bounds()
        .expect("the name: line's bounds");
    // One row is ~15 high; a wrap doubles it.
    assert!(
        name.y1 - name.y0 < 24.0,
        "the 100-column line is one row high ({name:?})"
    );
}

#[test]
fn the_preview_keeps_its_scroll_strip_when_the_block_fits() {
    let (ctx, _events) = crate::test_support::context();
    // `blank`'s block is 24 lines: it fits the preview's height without
    // scrolling, and the bar keeps its strip all the same.
    let mut small = RulesetView::new();
    open_document(&mut small, crate::document::blank());
    small.show_preview = true;
    let (_, small_frame, _) = draw_at(&mut small, &ctx, DEFAULT_PANE);
    let mut autumn = autumn_view(Section::General);
    autumn.show_preview = true;
    let (_, autumn_frame, _) = draw_at(&mut autumn, &ctx, DEFAULT_PANE);

    // The rightmost ScrollBar node's left edge is the strip's: the same
    // whether the block scrolls or fits.
    let strip = |frame: &Frame| {
        frame
            .nodes
            .iter()
            .filter(|node| node.role() == Role::ScrollBar)
            .filter_map(|node| node.bounds())
            .map(|bounds| bounds.x0)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let (long, short) = (strip(&autumn_frame), strip(&small_frame));
    assert!(
        (short - long).abs() <= SLACK,
        "the strip stands in the same place ({long} vs {short})"
    );
}
