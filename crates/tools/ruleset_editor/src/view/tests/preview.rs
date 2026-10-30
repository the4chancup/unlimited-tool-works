//! The data-block preview: **Show preview** draws the block read-only,
//! and the last recorded edit's changed lines are the view's `changed`
//! set. The highlight's own scenario waits for a later build.

use super::general::choose_picture;
use super::*;

/// A real event logo the drafts draw, `stem` naming it (`dbc7`); each
/// is 84 pixels high, so a pick stores it made smaller.
fn draft_logo(stem: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../docs/prototypes/assets/logos/{stem}.png"))
}

/// The document's logo, as the PNG's bytes.
fn logo_png(view: &RulesetView) -> Vec<u8> {
    view.document
        .as_ref()
        .and_then(|document| document.ruleset.logo_png())
        .expect("a logo")
}

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
    choose_picture(&mut window, &mut view, &ctx, &draft_logo("dbc7"));
    let stored = logo_png(&view);
    view.show_preview = true;
    let frame = settled(&mut window, &mut view, &ctx);

    let short = "    logo: \"iVBORw0KGgoAAAANSUhEUgAA…\",  // a 10 KB PNG, shortened in this view";
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
        Some(stored),
        "the file holds the whole logo"
    );
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
    // Autumn 26's and VGL's logos, made smaller, are both 9 KB.
    choose_picture(&mut window, &mut view, &ctx, &draft_logo("autumn26"));
    let first_kb = logo_png(&view).len().div_ceil(1024);
    let first_line = whole_logo_line(&view);
    choose_picture(&mut window, &mut view, &ctx, &draft_logo("vgl"));
    assert_eq!(
        (first_kb, logo_png(&view).len().div_ceil(1024)),
        (9, 9),
        "premise: two logos of one size in KB"
    );
    view.show_preview = true;
    let frame = settled(&mut window, &mut view, &ctx);

    let vgl_line = whole_logo_line(&view);
    assert_ne!(
        vgl_line, first_line,
        "premise: the second pick replaced the logo"
    );
    let short = "    logo: \"iVBORw0KGgoAAAANSUhEUgAA…\",  // a 9 KB PNG, shortened in this view";
    let drawn_line = frame
        .nodes
        .iter()
        .filter_map(accesskit::Node::value)
        .find(|text| text.starts_with("    logo: "))
        .expect("the logo's line is drawn");
    assert!(
        drawn_line == short,
        "the drawn line is Autumn 26's shortened one too: {drawn_line:.120}"
    );
    assert!(
        view.changed.contains(&vgl_line),
        "the replacement marks the whole new line: {:.120?}",
        view.changed
    );
}
