//! The `check()` problems shown: the bottom bar's count (singular,
//! plural, the hover for all) and the rail's marks; and the rail's
//! selected entry.

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
// `exact_size(140)`, so resting the pointer on its right edge leaves the default
// cursor and registers no splitter.
#[test]
fn the_fixed_width_rail_shows_no_resize_edge() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    // The headless view starts at x 0, so the rail's right edge sits at x 140
    // (inside the grab band a resizable side panel gives its edge), at a height
    // no rail entry sits at.
    let edge = egui::pos2(140.0, 400.0);
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
