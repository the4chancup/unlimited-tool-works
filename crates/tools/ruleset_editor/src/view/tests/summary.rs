//! The Summary section's GUI half (RE-SUM-01): the drawn text is the
//! summary and Copy puts the same text on the clipboard; and the
//! sections the window cannot edit yet send a member to it.

use super::*;

/// Each section the window cannot edit yet, picked in the rail, draws
/// its heading and the line saying where it shows and that a save keeps
/// it.
#[test]
fn each_unbuilt_section_says_where_it_shows_and_that_a_save_keeps_it() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    for name in [
        "Heights",
        "Cards",
        "Universal rules",
        "Specials",
        "Suggestions",
    ] {
        let frame = window.draw(&mut view, &ctx);
        let entry = frame
            .nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some(name))
            .unwrap_or_else(|| panic!("no rail entry {name:?}"));
        click(&mut window, entry, &mut view, &ctx);
        let frame = window.draw(&mut view, &ctx);
        assert!(
            nodes_labeled(&frame.nodes, name)
                .iter()
                .any(|node| node.role() == Role::Label),
            "{name}'s heading"
        );
        assert!(
            drawn(
                &frame.nodes,
                "This section can't be edited here yet. Summary describes it, Show preview shows it as the file holds it, and saving keeps it as it is."
            ),
            "{name}'s line"
        );
    }
}

// RE-SUM-01
#[test]
fn the_summary_draws_the_rules_text_and_copy_puts_it_on_the_clipboard() {
    let autumn = crate::view::start::templates()[0].ruleset.clone();
    let expected = crate::summary::summary(&autumn);
    let (ctx, _events, mut window, mut view) = fresh();
    open_document(&mut view, autumn);

    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Summary", &mut view, &ctx);
    let frame = window.draw(&mut view, &ctx);
    assert!(
        frame
            .nodes
            .iter()
            .any(|node| node.value().is_some_and(|text| text == expected)),
        "the drawn text is the summary"
    );
    // The CopyText command lands in the click's own frame.
    let at = centre(node(&frame.nodes, "Copy"));
    window.frame(pointer_at(at, true), |ui| show(ui, &mut view, &ctx));
    let frame = window.frame(pointer_at(at, false), |ui| show(ui, &mut view, &ctx));
    assert!(
        frame
            .platform
            .commands
            .contains(&egui::OutputCommand::CopyText(expected)),
        "Copy's CopyText is the same text"
    );
}
