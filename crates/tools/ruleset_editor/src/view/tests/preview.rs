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
    let whole = whole_logo_line(&view);
    assert!(
        view.changed.contains(&whole),
        "the pick marks the whole line"
    );
    assert!(!view.changed.contains(short), "never the short one");
    assert!(view.previous_lines.contains(&whole));

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

/// VGL's logo, a PNG of the same size in KB as DBC7's.
fn vgl_logo() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/prototypes/assets/logos/vgl.png")
}

/// The data block's whole logo line in `view`'s document.
fn whole_logo_line(view: &RulesetView) -> String {
    view.document
        .as_ref()
        .unwrap()
        .ruleset
        .to_data_block()
        .lines()
        .find(|line| line.starts_with("    logo: "))
        .expect("the block holds a logo line")
        .to_owned()
}

#[test]
fn a_replaced_logo_of_the_same_size_marks_its_line() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    choose_png(&mut window, &mut view, &ctx, &dbc7_logo());
    let dbc7_line = whole_logo_line(&view);
    choose_png(&mut window, &mut view, &ctx, &vgl_logo());
    view.show_preview = true;
    let frame = settled(&mut window, &mut view, &ctx);

    let vgl_line = whole_logo_line(&view);
    assert_ne!(
        vgl_line, dbc7_line,
        "premise: the second pick replaced the logo"
    );
    let short = "    logo: \"iVBORw0KGgoAAAANSUhEUgAA…\",  // a 16 KB PNG, shortened in this view";
    let drawn_line = frame
        .nodes
        .iter()
        .filter_map(accesskit::Node::value)
        .find(|text| text.starts_with("    logo: "))
        .expect("the logo's line is drawn");
    assert!(
        drawn_line == short,
        "the drawn line is DBC7's shortened one too: {drawn_line:.120}"
    );
    assert!(
        view.changed.contains(&vgl_line),
        "the replacement marks the whole new line: {:.120?}",
        view.changed
    );
}
