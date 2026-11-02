use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::accesskit;

use crate::settings::Settings;
use crate::shell::StudioApp;
use crate::shell::test_harness::*;

/// The offer's text in the status bar, `ShellCondition::UpdateAvailable`'s
/// line for the fixture's `v0.0.2`.
const OFFER_TEXT: &str = "Update available: v0.0.2";
/// The dialog's sentence, for the fixture's `v0.0.2` over the built `0.0.1`.
const SENTENCE: &str = "Unlimited Tool Works v0.0.2 pre-alpha is out. You have v0.0.1 pre-alpha.";
/// The release page the fixture names.
const PAGE_URL: &str = "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2";

/// A shell whose update check answers the shared releases fixture at once:
/// the offer lands on the bar once the worker's reply is drained. Returns the
/// shell and its data folder, where the settings file is written.
fn offer_shell(name: &str) -> (Shell, PathBuf) {
    let dir = TestDir::new(name);
    let data = dir.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(dir.path());
    settings.common.check_for_updates = true;
    let mut shell = shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
    shell.app.update_fetch =
        std::sync::Arc::new(|| Ok(crate::updater::RELEASES_FIXTURE.to_owned()));
    shell._dir = Some(dir);
    (shell, data)
}

/// Frames until the offer's text is on the bar, or the deadline runs out and
/// the assert names the wait that never ended.
fn frame_until_offer(egui_ctx: &egui::Context, app: &mut StudioApp) -> FrameOutput {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let out = frame(egui_ctx, app, Vec::new(), true);
        if has_text(&out.nodes, OFFER_TEXT) {
            return out;
        }
        assert!(Instant::now() < deadline, "the offer never reached the bar");
    }
}

/// Opens the dialog through the bar's Details button and settles the modal's
/// first frames, returning the open dialog's nodes.
fn open_details(egui_ctx: &egui::Context, app: &mut StudioApp) -> Vec<accesskit::Node> {
    let nodes = frame_until_offer(egui_ctx, app).nodes;
    click(egui_ctx, app, entry(&nodes, "Details"));
    let nodes = idle(egui_ctx, app, 10).nodes;
    assert!(has_text(&nodes, SENTENCE), "the Details dialog is open");
    nodes
}

/// The node carrying `text` as a label-role node's `value`.
fn text_node<'a>(nodes: &'a [accesskit::Node], text: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.value() == Some(text))
        .unwrap_or_else(|| panic!("no node with text {text:?}"))
}

/// The painted `Shape::Text` carrying exactly `text`.
fn text_shape<'a>(out: &'a FrameOutput, text: &str) -> &'a egui::epaint::TextShape {
    out.shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) => Some(shape),
            _ => None,
        })
        .find(|shape| shape.galley.text() == text)
        .unwrap_or_else(|| panic!("no painted text {text:?}"))
}

/// The color `shape`'s text paints in: a section's own color when one was
/// set, else the shape's fallback.
fn painted_color(shape: &egui::epaint::TextShape) -> egui::Color32 {
    if let Some(overridden) = shape.override_text_color {
        return overridden;
    }
    shape
        .galley
        .job
        .sections
        .iter()
        .map(|section| section.format.color)
        .find(|&color| color != egui::Color32::PLACEHOLDER)
        .unwrap_or(shape.fallback_color)
}

/// A key event.
fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

// ST-UPD-01
#[test]
fn the_offers_text_is_in_the_link_blue_with_details_on_the_bar() {
    let (mut shell, _data) = offer_shell("studio_core_update_offer_link");
    let egui_ctx = test_context();
    let out = frame_until_offer(&egui_ctx, &mut shell.app);

    let visuals = egui_ctx.global_style().visuals.clone();
    assert_ne!(
        visuals.hyperlink_color,
        visuals.weak_text_color(),
        "premise: the two colours differ"
    );
    assert_eq!(
        painted_color(text_shape(&out, OFFER_TEXT)),
        visuals.hyperlink_color,
        "the offer's text is the link blue"
    );
    entry(&out.nodes, "Details");
}

// ST-UPD-02
#[test]
fn the_offer_text_and_details_each_open_the_dialog() {
    let (mut shell, _data) = offer_shell("studio_core_update_details_open");
    let egui_ctx = test_context();
    let out = frame_until_offer(&egui_ctx, &mut shell.app);

    // The text's click opens the dialog: the version sentence, the release's
    // notes, and the three buttons in the draft's order.
    click(&egui_ctx, &mut shell.app, text_node(&out.nodes, OFFER_TEXT));
    let nodes = idle(&egui_ctx, &mut shell.app, 10).nodes;
    text_node(&nodes, "Update available");
    assert!(has_text(&nodes, SENTENCE), "the version sentence is drawn");
    assert!(has_text(&nodes, "Fixes and polish."), "the notes are drawn");
    entry(&nodes, "Open release page");
    entry(&nodes, "Skip this version");
    entry(&nodes, "Remind me later");
    assert!(
        !has_label(&nodes, "Update now"),
        "no Update now stands on the dialog in 0.0.1"
    );

    // Closed, the dialog opens again on the Details button.
    click(&egui_ctx, &mut shell.app, entry(&nodes, "Remind me later"));
    let nodes = open_details(&egui_ctx, &mut shell.app);
    assert!(has_text(&nodes, SENTENCE), "Details opens the dialog again");
}

// ST-UPD-02
#[test]
fn skip_this_version_stores_the_version_and_drops_the_offer() {
    let (mut shell, data) = offer_shell("studio_core_update_skip");
    let egui_ctx = test_context();
    let nodes = open_details(&egui_ctx, &mut shell.app);

    click(
        &egui_ctx,
        &mut shell.app,
        entry(&nodes, "Skip this version"),
    );
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    assert!(!has_text(&nodes, OFFER_TEXT), "the offer leaves the bar");
    assert!(!has_text(&nodes, SENTENCE), "the dialog is closed");
    assert!(shell.app.update_release.is_none(), "the release is dropped");
    let written = Settings::load(&data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(
        written.common.skipped_version.as_deref(),
        Some("0.0.2"),
        "the settings file holds the skipped version"
    );
}

// ST-UPD-02
#[test]
fn remind_me_later_closes_the_dialog_and_keeps_the_offer() {
    let (mut shell, _data) = offer_shell("studio_core_update_remind");
    let egui_ctx = test_context();
    let nodes = open_details(&egui_ctx, &mut shell.app);

    click(&egui_ctx, &mut shell.app, entry(&nodes, "Remind me later"));
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    assert!(!has_text(&nodes, SENTENCE), "the dialog is closed");
    assert!(has_text(&nodes, OFFER_TEXT), "the offer stays on the bar");
}

// ST-UPD-02
#[test]
fn escape_closes_the_dialog_like_remind_me_later() {
    let (mut shell, _data) = offer_shell("studio_core_update_escape");
    let egui_ctx = test_context();
    open_details(&egui_ctx, &mut shell.app);

    frame(
        &egui_ctx,
        &mut shell.app,
        vec![key(egui::Key::Escape, true)],
        true,
    );
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    assert!(!has_text(&nodes, SENTENCE), "the dialog is closed");
    assert!(has_text(&nodes, OFFER_TEXT), "the offer stays on the bar");
}

// ST-UPD-02
#[test]
fn open_release_page_opens_the_page_and_keeps_the_dialog() {
    let (mut shell, _data) = offer_shell("studio_core_update_page");
    let egui_ctx = test_context();
    let nodes = open_details(&egui_ctx, &mut shell.app);

    let out = click(
        &egui_ctx,
        &mut shell.app,
        entry(&nodes, "Open release page"),
    );
    assert!(
        out.platform
            .commands
            .contains(&egui::OutputCommand::OpenUrl(egui::OpenUrl::same_tab(
                PAGE_URL
            ))),
        "the release's page opens in the browser"
    );
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    assert!(has_text(&nodes, SENTENCE), "the dialog stays open");
}

// A help link opened while the dialog is up closes it, as it closes the
// settings menu and the About popup.
#[test]
fn a_help_link_opened_over_the_dialog_closes_it() {
    let (mut shell, _data) = offer_shell("studio_core_update_help_close");
    let egui_ctx = test_context();
    open_details(&egui_ctx, &mut shell.app);

    // F1 opens the help on the active tool's chapter, as the sidebar's ? does.
    frame(
        &egui_ctx,
        &mut shell.app,
        vec![key(egui::Key::F1, true)],
        true,
    );
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    assert!(
        !has_text(&nodes, SENTENCE),
        "the help's opening closed the dialog"
    );
}
