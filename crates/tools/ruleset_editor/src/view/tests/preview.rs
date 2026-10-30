//! The data-block preview: **Show preview** draws the block read-only,
//! and the last recorded edit's changed lines are the view's `changed`
//! set. The highlight's own scenario waits for a later build.

use super::general::{choose_png, dbc7_logo};
use super::*;

#[test]
fn the_preview_draws_the_block_and_the_edit_marks_its_changed_lines() {
    let (ctx, _events, mut window, mut view) = fresh_blank();

    // Off at every start: no preview lines.
    let frame = window.draw(&mut view, &ctx);
    assert!(
        !drawn(&frame.nodes, "Rules file: data block"),
        "the preview is off at start"
    );

    // Show preview on, through the toolbar toggle.
    click_labeled(&mut window, &frame.nodes, "Show preview", &mut view, &ctx);
    assert!(view.show_preview);
    let frame = window.draw(&mut view, &ctx);
    assert!(drawn(&frame.nodes, "Rules file: data block"));
    let block_line = frame
        .nodes
        .iter()
        .find(|node| {
            node.value()
                .is_some_and(|text| text.starts_with("const RULESET"))
        })
        .expect("the block's lines are drawn");
    assert_eq!(
        block_line.role(),
        Role::Label,
        "read-only lines, not editors"
    );

    // A checkbox edit in the at-most list (the second same-named
    // checkbox): one gesture, one recorded snapshot, and `changed` is
    // the block's lines the edit rewrote.
    let before = view.document.as_ref().unwrap().ruleset.clone();
    click_drawn_nth(&mut window, &mut view, &ctx, "Attacking Prowess", 1);
    assert_ne!(
        view.document.as_ref().unwrap().ruleset,
        before,
        "premise: the click changed the ruleset"
    );
    let before = before.to_data_block();
    let after = view.document.as_ref().unwrap().ruleset.to_data_block();
    let expected: std::collections::BTreeSet<String> = after
        .lines()
        .filter(|line| !before.lines().any(|old| old == *line))
        .map(str::to_owned)
        .collect();
    assert!(!expected.is_empty(), "premise: the block changed");
    assert_eq!(
        view.changed, expected,
        "the changed lines are the new block's lines the old one lacked"
    );
    assert!(
        !view.changed.iter().any(|line| line.contains("name:")),
        "an untouched line is not marked: {:?}",
        view.changed
    );
}

#[test]
fn the_preview_shortens_the_logos_line_and_the_save_keeps_it_whole() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    choose_png(&mut window, &mut view, &ctx, &dbc7_logo());
    view.show_preview = true;
    let frame = settled(&mut window, &mut view, &ctx);

    let short = "    logo: \"iVBORw0KGgoAAAANSUhEUgAA…\",  // a 16 KB PNG, shortened in this view";
    let drawn_line = frame
        .nodes
        .iter()
        .filter_map(accesskit::Node::value)
        .find(|text| text.starts_with("    logo: "))
        .expect("the logo's line is drawn");
    assert!(drawn_line == short, "the drawn line: {drawn_line:.120}");
    assert!(
        view.changed.contains(short),
        "the pick marks the short line"
    );
    assert!(view.previous_lines.iter().any(|line| line == short));

    let dir = crate::test_support::scratch("preview_logo");
    let path = dir.join("logo.4ccrules");
    save_as_via_menu(&mut window, &mut view, &ctx, &path);
    let written = std::fs::read_to_string(&path).expect("the save wrote the file");
    let rules = aatf::Rules::load(&written).expect("the saved file loads");
    assert_eq!(
        rules.ruleset().logo_png(),
        Some(std::fs::read(dbc7_logo()).unwrap()),
        "the file holds the whole logo"
    );
}
