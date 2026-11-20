//! The Custom checks section (RE-GUI-13): the file's checks read-only
//! with the kept-as-is line.

use super::*;

// RE-GUI-13
#[test]
fn the_custom_checks_section_shows_the_code_read_only() {
    let template = &crate::view::start::templates()[5];
    assert_eq!(
        template.ruleset.name, "/vt/ League 11",
        "premise: index 5 is VTL11"
    );
    let checks = template
        .custom_checks
        .as_ref()
        .expect("VTL11 carries checks");
    assert!(
        checks.starts_with("// ==== CUSTOM CHECKS"),
        "premise: the section as read carries its banner line"
    );
    let mut document = Document::new(template.ruleset.clone());
    document.custom_checks = Some(checks.clone());
    let (ctx, _events, mut window, mut view) = fresh();
    view.set_document(document);

    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Custom checks", &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(drawn(
        &frame.nodes,
        "This code runs after the ruleset's own checks, and is kept as it is when you save."
    ));
    // The code is drawn alone: the section's own `==== CUSTOM CHECKS`
    // banner line (kept for the byte-for-byte save) is not shown. A
    // multi-line label's node holds the whole text; wrapped fragments
    // of each line are nodes of their own.
    let code = frame
        .nodes
        .iter()
        .find(|node| {
            node.value().is_some_and(|text| {
                text.starts_with("// What the schema cannot carry:")
                    && text.contains("fn custom_checks(team) {")
            })
        })
        .expect("the custom-checks code is drawn, its banner line not");
    assert!(
        !labels(&frame.nodes)
            .iter()
            .any(|label| label.contains("==== CUSTOM CHECKS")),
        "the banner line is not drawn"
    );
    assert_eq!(
        code.role(),
        Role::Label,
        "a label, not an editor: read-only"
    );
}
