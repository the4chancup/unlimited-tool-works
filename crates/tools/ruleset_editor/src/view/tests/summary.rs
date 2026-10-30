//! The Summary section's GUI half (RE-SUM-01): the drawn text is the
//! summary and Copy puts the same text on the clipboard; and the
//! sections the window cannot edit yet send a member to it and link to
//! their demo online.

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
                "You can't edit this section yet. Summary shows what's in it, and saving leaves it unchanged."
            ),
            "{name}'s line"
        );
    }
}

/// The demo link's text.
const DEMO_LINK: &str = "Check out a demo of the planned section online";

/// Each section the window cannot edit yet links to its draft's demo
/// online: a click opens that section's address; the sections the
/// window edits or shows draw no such link.
#[test]
fn each_unbuilt_section_links_to_its_demo_online() {
    let (ctx, _events, mut window, mut view) = fresh_blank();
    for (section, id) in [
        (Section::Heights, "heights"),
        (Section::Cards, "cards"),
        (Section::Universal, "universal"),
        (Section::Specials, "specials"),
        (Section::Suggestions, "suggestions"),
    ] {
        view.section = section;
        let frame = settled(&mut window, &mut view, &ctx);
        let at = centre(node(&frame.nodes, DEMO_LINK));
        window.frame(pointer_at(at, true), |ui| show(ui, &mut view, &ctx));
        let frame = window.frame(pointer_at(at, false), |ui| show(ui, &mut view, &ctx));
        let address = format!(
            "https://the4chancup.github.io/unlimited-tool-works/prototypes/#ruleset_editor/{id}?demo=1&start=0"
        );
        assert!(
            frame
                .platform
                .commands
                .contains(&egui::OutputCommand::OpenUrl(egui::OpenUrl::new_tab(
                    &address
                ))),
            "{id}: the click opens {address}: {:?}",
            frame.platform.commands
        );
    }
    for section in [Section::General, Section::Tiers, Section::Summary] {
        view.section = section;
        let frame = settled(&mut window, &mut view, &ctx);
        assert!(
            !drawn(&frame.nodes, DEMO_LINK),
            "{section:?} draws no demo link"
        );
    }
    // Custom checks shows only over a document that carries checks; over one
    // without, the rail switches to General. VTL11 carries some.
    let vtl11 = &crate::view::start::templates()[4];
    let mut document = Document::new(vtl11.ruleset.clone());
    document.custom_checks = vtl11.custom_checks.clone();
    assert!(
        document.custom_checks.is_some(),
        "premise: VTL11 carries checks"
    );
    view.set_document(document);
    view.section = Section::CustomChecks;
    let frame = settled(&mut window, &mut view, &ctx);
    assert_eq!(
        view.section,
        Section::CustomChecks,
        "premise: Custom checks is drawn"
    );
    assert!(
        !drawn(&frame.nodes, DEMO_LINK),
        "Custom checks draws no demo link"
    );
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
