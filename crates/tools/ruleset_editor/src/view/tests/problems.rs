//! The `check()` problems shown: the bottom bar's count (singular,
//! plural, the hover for all) and the rail's marks; the rail's and the
//! bottom bar's fills at the view's left edge; and the rail's rows,
//! selected entry and a problem's mark at the row's right end.

use super::*;

#[test]
fn the_problem_count_singularizes_one_and_pluralizes_more() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // One problem: an empty name is an empty-string rule break.
    edit_ruleset(&mut view, |ruleset| ruleset.name = String::new());
    let frame = window.draw(&mut view, &ctx);
    assert!(contains_label(&frame.nodes, "✖ 1 problem:"));
    assert!(
        !contains_label(&frame.nodes, "(hover for all)"),
        "one problem carries no hover hint"
    );

    // Two problems: the name and an unknown at-most target.
    edit_ruleset(&mut view, |ruleset| {
        ruleset.stats.at_most.push("bogus".to_string());
    });
    let frame = window.draw(&mut view, &ctx);
    assert!(contains_label(&frame.nodes, "✖ 2 problems:"));
    assert!(contains_label(&frame.nodes, "(hover for all)"));
}

// A fixed-width panel shows no resize edge (the draft's plain edge): the rail is
// `exact_size(150)`, so resting the pointer on its right edge leaves the default
// cursor and registers no splitter.
#[test]
fn the_fixed_width_rail_shows_no_resize_edge() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // The headless view starts at x 0, so the rail's right edge sits at x 150
    // (inside the grab band a resizable side panel gives its edge), at a height
    // no rail entry sits at.
    let edge = egui::pos2(150.0, 400.0);
    let mut frame = window.frame(vec![egui::Event::PointerMoved(edge)], |ui| {
        show(ui, &mut view, &ctx)
    });
    for _ in 0..3 {
        frame = window.draw(&mut view, &ctx);
    }
    assert_eq!(
        frame.platform.cursor_icon,
        egui::CursorIcon::Default,
        "the fixed-width edge is not a resize handle"
    );
    assert!(
        !frame
            .nodes
            .iter()
            .any(|node| node.role() == Role::Splitter && node.label() == Some("Resize panel")),
        "a fixed-width panel registers no splitter"
    );
}

/// The rail draws the drawn section's entry selected and no other
/// (egui gives AccessKit a selectable entry's state as its toggle).
#[test]
fn the_rail_selects_the_drawn_section_alone() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    for current in [Section::General, Section::Tiers, Section::Summary] {
        view.section = current;
        let frame = window.draw(&mut view, &ctx);
        let selected: Vec<&str> = frame
            .nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node.label().is_some_and(|label| {
                        Section::RAIL.iter().any(|section| section.label() == label)
                    })
                    && node.toggled() == Some(accesskit::Toggled::True)
            })
            .filter_map(|node| node.label())
            .collect();
        assert_eq!(selected, [current.label()], "{current:?}");
    }
}

#[test]
fn a_problem_marks_only_its_section_in_the_rail() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // A tier's empty id is a `tiers[0].id` problem: Tiers alone marked.
    edit_ruleset(&mut view, |ruleset| {
        ruleset.tiers[0].id = String::new();
    });
    let frame = window.draw(&mut view, &ctx);
    let rail = rail_labels(&frame.nodes);
    assert_eq!(rail[0], "General", "no mark on General");
    assert_eq!(rail[1], "Tiers ⚠", "the faulty section marked");
    assert!(
        rail[2..].iter().all(|label| !label.contains('⚠')),
        "no other entry marked: {rail:?}"
    );
}

// With a document open the layout runs to the view's edges (the draft's
// `.bleed`): the bottom bar's and the rail's fills stand at the view's
// left edge, the rail the draft's 150 wide to its line.
#[test]
fn the_rails_and_bottom_bars_fills_start_at_the_views_left_edge() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    let rail = panel_fill_at(&frame, &window.ctx, |rect| {
        rect.left() == 0.0 && rect.height() > 100.0 && rect.height() < 600.0
    });
    assert_eq!(
        rail.iter().map(egui::Rect::width).collect::<Vec<_>>(),
        [149.0],
        "the rail's fill from the left edge, the draft's 150 less its line's point: {rail:?}"
    );
    let stroke = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let line = frame
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].x == points[1].x
                    && points[0].x > 140.0
                    && points[0].x < 160.0 =>
            {
                Some(f64::from(points[0].x))
            }
            _ => None,
        });
    assert_eq!(line, Some(149.5), "the rail's line centred on 149.5");
    let bottom = panel_fill_at(&frame, &window.ctx, |rect| {
        rect.left() == 0.0 && rect.bottom() == 600.0 && rect.height() < 100.0
    });
    assert_eq!(
        bottom.len(),
        1,
        "the bottom bar's fill from the left edge: {bottom:?}"
    );
}

// A rail entry is a row, not a control: the selected one's blue draws
// no edge at rest (the draft's `.rail .sel` carries none, held or not).
#[test]
fn the_selected_rail_entry_draws_its_blue_with_no_edge() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    let blue = window.ctx.global_style().visuals.selection.bg_fill;
    let entry = frame
        .nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("General"))
        .and_then(accesskit::Node::bounds)
        .expect("the selected rail entry");
    let row = rect_of_fill_at(&frame.shapes, entry, blue);
    assert_eq!(
        row.stroke,
        egui::Stroke::NONE,
        "a row draws no resting edge"
    );
}

// A rail entry fills its row (the draft's `.rail .sel`), 6 in from the
// rail's edge each side.
#[test]
fn a_rail_entry_fills_the_rails_row() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    let entry = frame
        .nodes
        .iter()
        .find(|node| {
            node.role() == Role::Button
                && node.label() == Some("General")
                && node.toggled() == Some(accesskit::Toggled::True)
        })
        .and_then(accesskit::Node::bounds)
        .expect("the selected rail entry's bounds");
    assert_eq!(entry.x0, 6.0, "the rail's padding before the row");
    assert_eq!(
        entry.x1, 143.0,
        "the row ends the padding short of the line"
    );
}

// The rail's rows stand 24 points apart (the draft's `.rail .sel`
// `padding: 3px 6px` — three points above and below each row), where
// egui's own item spacing would put them 21.
#[test]
fn the_rails_rows_stand_24_points_apart() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    let frame = settled(&mut window, &mut view, &ctx);
    let mut rows: Vec<&accesskit::Node> = frame
        .nodes
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node.label().is_some_and(|text| {
                    Section::RAIL
                        .iter()
                        .any(|section| text.starts_with(section.label()))
                })
        })
        .collect();
    sort_top_down(&mut rows);
    assert!(rows.len() >= 2, "premise: the rail has several rows");
    for pair in rows.windows(2) {
        let (a, b) = (
            pair[0].bounds().expect("row bounds"),
            pair[1].bounds().expect("row bounds"),
        );
        assert!(
            (b.y0 - a.y0 - 24.0).abs() <= 0.5,
            "consecutive rail rows 24 apart ({a:?} then {b:?})"
        );
    }
}

// A problem's "⚠" stands at the entry's right end (the draft's
// space-between row), its own text shape in the error colour.
#[test]
fn a_problems_mark_stands_at_the_entrys_right_end() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    edit_ruleset(&mut view, |ruleset| {
        ruleset.tiers[0].id = String::new();
    });
    let frame = settled(&mut window, &mut view, &ctx);
    let entry = frame
        .nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Tiers ⚠"))
        .and_then(accesskit::Node::bounds)
        .expect("the marked entry's bounds");
    let mark = frame
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.text() == "⚠"
                    && f64::from(text.pos.y) >= entry.y0
                    && f64::from(text.pos.y) < entry.y1 =>
            {
                Some(f64::from(text.pos.x + text.galley.size().x))
            }
            _ => None,
        })
        .expect("the ⚠ text inside the entry's row");
    assert_eq!(mark, entry.x1 - 4.0, "the mark the button padding in");
}
