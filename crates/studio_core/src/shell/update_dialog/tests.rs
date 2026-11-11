use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::{Sender, bounded};
use eframe::App;
use egui::accesskit;

use super::{Failure, Stage};
use crate::settings::Settings;
use crate::shell::test_harness::*;
use crate::shell::{StudioApp, UpdatePoll};
use crate::tool::AppPaths;
use crate::updater::test_bundle::{
    folder_with, new_files, old_files, running_assets, serve, snapshot,
};
use crate::updater::{InstallError, SwapStep};

/// The offer's text in the status bar, `ShellCondition::UpdateAvailable`'s
/// line for the fixture's `v0.0.2`.
const OFFER_TEXT: &str = "Update available: v0.0.2";
/// The dialog's sentence, for the fixture's `v0.0.2` over the built `0.0.1`.
const SENTENCE: &str = "Unlimited Tool Works v0.0.2 pre-alpha is out. You have v0.0.1 pre-alpha.";
/// The release page the fixture names.
const PAGE_URL: &str = "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2";
/// The archive's size as the download's line shows it: the size the releases
/// fixture lists for this platform's archive, in MB.
const TOTAL_MB: &str = if cfg!(windows) { "10.7" } else { "11.8" };
/// `Update ready`'s sentence for the fixture's `v0.0.2`.
const READY_SENTENCE: &str = "v0.0.2 pre-alpha is installed. Restart to start using it, the \
                              version you are running now will stay in the old folder.";
/// `Update failed`'s way out, for the fixture's `v0.0.2`.
const WAY_OUT: &str = "You can download v0.0.2 from the release page instead.";
/// `Update failed`'s second paragraph when nothing was changed.
const NOTHING_CHANGED: &str =
    "Nothing was changed. You can download v0.0.2 from the release page instead.";

/// A shell whose update check answers the shared releases fixture at once:
/// the offer lands on the bar once the worker's reply is drained. Returns the
/// shell and its data folder, where the settings file is written.
fn offer_shell(name: &str) -> (Shell, PathBuf) {
    offer_shell_in(name, PathBuf::from("exe"))
}

/// `offer_shell` with `exe_dir` as the program's folder, the one Update now
/// installs into.
fn offer_shell_in(name: &str, exe_dir: PathBuf) -> (Shell, PathBuf) {
    let dir = TestDir::new(name);
    let data = dir.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(dir.path());
    settings.common.check_for_updates = true;
    let paths = AppPaths {
        exe_dir,
        data_dir: Some(data.clone()),
    };
    let mut shell = shell_in(settings, vec![a], &[], &[], paths, None, vec![calls_a]);
    shell.app.update_fetch = Arc::new(|| Ok(crate::updater::RELEASES_FIXTURE.to_owned()));
    shell._dir = Some(dir);
    (shell, data)
}

/// A shell whose offer installs into a fresh program folder of `old_files()`
/// (`folder_with(name)`), its download serving `assets`, the archive's second
/// half held until the returned sender sends or is dropped. Returns the shell,
/// the program's folder and that sender.
fn update_shell(name: &str, assets: Vec<(String, Vec<u8>)>) -> (Shell, PathBuf, Sender<()>) {
    let bundle = folder_with(name, &old_files());
    let (mut shell, _data) = offer_shell_in(&format!("studio_core_update_{name}"), bundle.clone());
    let (go_on, gate) = bounded(0);
    shell.app.update_open_url = Arc::from(serve(assets, Some(gate)));
    (shell, bundle, go_on)
}

/// Opens the offer's dialog through Details and clicks Update now.
fn update_now(egui_ctx: &egui::Context, app: &mut StudioApp) {
    let nodes = open_details(egui_ctx, app);
    click(egui_ctx, app, entry(&nodes, "Update now"));
}

/// The download's line once `done` MB of the archive are in.
fn download_line(done: &str) -> String {
    format!("Downloading v0.0.2 pre-alpha… {done} of {TOTAL_MB} MB")
}

/// Frames until a node's text is exactly `text`, then three more, so a stage
/// that just came up has its final layout and a click lands on the button it
/// aims at. Returns the last frame's nodes. Past the deadline the assert
/// names the text and every text on screen.
fn frame_until_text(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
    text: &str,
) -> Vec<accesskit::Node> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let nodes = frame(egui_ctx, app, Vec::new(), true).nodes;
        if nodes.iter().any(|node| node.value() == Some(text)) {
            return idle(egui_ctx, app, 3).nodes;
        }
        if Instant::now() >= deadline {
            let shown: Vec<&str> = nodes
                .iter()
                .filter_map(accesskit::Node::value)
                .filter(|value| !value.is_empty())
                .collect();
            panic!("never shown: {text:?}; on screen: {shown:?}");
        }
    }
}

/// The program's folder `before` an update as the update leaves it: the new
/// bundle's files in place, the old program in `old/`, the empty folder the
/// bundle adds, and the member's settings as they were.
fn installed(before: &BTreeMap<String, Option<String>>) -> BTreeMap<String, Option<String>> {
    let mut expected = before.clone();
    for (path, text) in new_files() {
        if path != "data/settings.toml" {
            expected.insert(path, Some(text.to_owned()));
        }
    }
    let (program, text) = old_files()
        .into_iter()
        .find(|(_, text)| *text == "old program")
        .expect("the old files hold the program");
    expected.insert("old".to_owned(), None);
    expected.insert(format!("old/{program}"), Some(text.to_owned()));
    expected.insert("exports".to_owned(), None);
    expected
}

/// Sets `last_update_check` back past the interval, so a check falls due.
fn make_check_due(shell: &Shell) {
    let mut settings = shell.settings.lock().unwrap();
    let interval = crate::updater::interval_secs(&settings.common);
    settings.common.last_update_check =
        Some(crate::updater::now_secs().saturating_sub(interval + 60));
}

/// Asserts that ten frames start no check: none is running after them and
/// `fetches` did not move. A check that started either still runs or has
/// fetched, so one of the two shows it.
fn assert_no_check(
    egui_ctx: &egui::Context,
    app: &mut StudioApp,
    fetches: &AtomicUsize,
    when: &str,
) {
    let before = fetches.load(Ordering::SeqCst);
    idle(egui_ctx, app, 10);
    assert!(
        matches!(app.update_poll, UpdatePoll::Idle { .. }),
        "no check is running {when}"
    );
    assert_eq!(
        fetches.load(Ordering::SeqCst),
        before,
        "no check fetched {when}"
    );
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
    // notes, and the four buttons.
    click(&egui_ctx, &mut shell.app, text_node(&out.nodes, OFFER_TEXT));
    let nodes = idle(&egui_ctx, &mut shell.app, 10).nodes;
    text_node(&nodes, "Update available");
    assert!(has_text(&nodes, SENTENCE), "the version sentence is drawn");
    assert!(has_text(&nodes, "Fixes and polish."), "the notes are drawn");
    entry(&nodes, "Update now");
    entry(&nodes, "Open release page");
    entry(&nodes, "Skip this version");
    entry(&nodes, "Remind me later");

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

// ST-UPD-03
#[test]
fn update_now_shows_the_download_then_installs_and_reports_ready() {
    let (mut shell, bundle, go_on) = update_shell("dialog_ready", running_assets(None));
    let before = snapshot(&bundle);
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);

    // Held after the archive's first half: its few hundred bytes are 0.0 MB
    // of the size the release lists.
    let nodes = frame_until_text(&egui_ctx, &mut shell.app, &download_line("0.0"));
    text_node(&nodes, "Updating to v0.0.2");
    assert!(
        nodes
            .iter()
            .any(|node| node.role() == accesskit::Role::ProgressIndicator),
        "the bar is drawn"
    );
    entry(&nodes, "Cancel");

    drop(go_on);
    let nodes = frame_until_text(&egui_ctx, &mut shell.app, READY_SENTENCE);
    text_node(&nodes, "Update ready");
    entry(&nodes, "Restart now");
    entry(&nodes, "Later");
    assert_eq!(snapshot(&bundle), installed(&before));
    fs::remove_dir_all(bundle).unwrap();
}

// ST-UPD-03
#[test]
fn a_checksum_that_does_not_match_fails_and_retry_installs() {
    let assets = running_assets(Some("11".repeat(32)));
    let archive = assets[0].0.clone();
    let (mut shell, bundle, go_on) = update_shell("dialog_mismatch", assets);
    drop(go_on);
    let before = snapshot(&bundle);
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);

    let reason = format!(
        "The downloaded {archive} is damaged: its checksum does not match the one in \
         SHA256SUMS.txt."
    );
    let nodes = frame_until_text(&egui_ctx, &mut shell.app, &reason);
    text_node(&nodes, "Update failed");
    text_node(&nodes, NOTHING_CHANGED);
    entry(&nodes, "Retry");
    entry(&nodes, "Close");
    assert_eq!(snapshot(&bundle), before, "every file as it was");

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
    text_node(&nodes, &reason);

    shell.app.update_open_url = Arc::from(serve(running_assets(None), None));
    click(&egui_ctx, &mut shell.app, entry(&nodes, "Retry"));
    frame_until_text(&egui_ctx, &mut shell.app, READY_SENTENCE);
    assert_eq!(snapshot(&bundle), installed(&before));
    fs::remove_dir_all(bundle).unwrap();
}

// ST-UPD-03
#[test]
fn cancel_during_the_download_brings_the_offer_back() {
    // An 8 MB archive that is no archive: held after its first half, the line
    // reads 4.0 MB in, and only the cancel can end its download in the offer,
    // since the whole of it would fail its checksum.
    let mut assets = running_assets(None);
    assets[0].1 = vec![0; 8 * 1_048_576];
    let (mut shell, bundle, go_on) = update_shell("dialog_cancel", assets);
    let before = snapshot(&bundle);
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);

    let nodes = frame_until_text(&egui_ctx, &mut shell.app, &download_line("4.0"));
    click(&egui_ctx, &mut shell.app, entry(&nodes, "Cancel"));
    drop(go_on);
    let nodes = frame_until_text(&egui_ctx, &mut shell.app, SENTENCE);
    entry(&nodes, "Update now");
    assert!(!has_text(&nodes, "Update failed"), "a cancel is no failure");
    assert_eq!(snapshot(&bundle), before, "every file as it was");
    fs::remove_dir_all(bundle).unwrap();
}

// A stray click or key must neither cancel a download nor hide one that runs.
#[test]
fn the_backdrop_and_escape_leave_the_download_on_screen() {
    let (mut shell, bundle, go_on) = update_shell("dialog_backdrop", running_assets(None));
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);
    let held = download_line("0.0");
    frame_until_text(&egui_ctx, &mut shell.app, &held);

    // The backdrop, left of the centred dialog.
    let outside = egui::pos2(20.0, 300.0);
    for pressed in [true, false] {
        let events = vec![
            egui::Event::PointerMoved(outside),
            egui::Event::PointerButton {
                pos: outside,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            },
        ];
        frame(&egui_ctx, &mut shell.app, events, true);
    }
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    text_node(&nodes, &held);

    frame(
        &egui_ctx,
        &mut shell.app,
        vec![key(egui::Key::Escape, true)],
        true,
    );
    let nodes = frame(&egui_ctx, &mut shell.app, Vec::new(), true).nodes;
    text_node(&nodes, &held);

    drop(go_on);
    frame_until_text(&egui_ctx, &mut shell.app, READY_SENTENCE);
    fs::remove_dir_all(bundle).unwrap();
}

// A check could replace the release under the dialog, so none runs while an
// install runs or after one succeeded; a failed install leaves them as they
// were.
#[test]
fn no_check_runs_while_an_install_runs_or_after_it_succeeded() {
    let (mut shell, bundle, go_on) =
        update_shell("dialog_no_check", running_assets(Some("11".repeat(32))));
    let fetches = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&fetches);
    shell.app.update_fetch = Arc::new(move || {
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(crate::updater::RELEASES_FIXTURE.to_owned())
    });
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);
    frame_until_text(&egui_ctx, &mut shell.app, &download_line("0.0"));
    assert_eq!(
        fetches.load(Ordering::SeqCst),
        1,
        "the check that raised the offer"
    );

    make_check_due(&shell);
    assert_no_check(
        &egui_ctx,
        &mut shell.app,
        &fetches,
        "while the download runs",
    );

    // The check that fell due during the download runs once it has failed.
    drop(go_on);
    let nodes = frame_until_text(&egui_ctx, &mut shell.app, "Update failed");
    let deadline = Instant::now() + Duration::from_secs(10);
    while fetches.load(Ordering::SeqCst) < 2
        || !matches!(shell.app.update_poll, UpdatePoll::Idle { .. })
    {
        assert!(
            Instant::now() < deadline,
            "the check after a failed install never ran"
        );
        frame(&egui_ctx, &mut shell.app, Vec::new(), true);
    }

    shell.app.update_open_url = Arc::from(serve(running_assets(None), None));
    click(&egui_ctx, &mut shell.app, entry(&nodes, "Retry"));
    frame_until_text(&egui_ctx, &mut shell.app, READY_SENTENCE);
    make_check_due(&shell);
    assert_no_check(&egui_ctx, &mut shell.app, &fetches, "after Update ready");
    fs::remove_dir_all(bundle).unwrap();
}

// The window closed during a download cancels it and waits for its worker, so
// a swap is never cut midway.
#[test]
fn a_window_closed_during_the_download_waits_for_the_worker() {
    let (mut shell, bundle, go_on) = update_shell("dialog_exit", running_assets(None));
    let before = snapshot(&bundle);
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);
    frame_until_text(&egui_ctx, &mut shell.app, &download_line("0.0"));

    // The second half goes on only after a while, so a close that does not
    // wait for the worker returns while the worker still holds its temp
    // folder, which the snapshot then shows.
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        drop(go_on);
    });
    shell.app.on_exit();
    assert!(
        !bundle.join(".update").exists(),
        "the worker ended, its temp folder removed"
    );
    assert_eq!(
        snapshot(&bundle),
        before,
        "the worker ended, its update cancelled, changing nothing"
    );
    releaser.join().unwrap();
    fs::remove_dir_all(bundle).unwrap();
}

// A worker that ends without an outcome is a failure, and the dialog makes no
// promise that nothing was changed.
#[test]
fn a_worker_that_ends_without_an_outcome_is_a_failure() {
    let (mut shell, bundle, _go_on) = update_shell("dialog_worker_gone", running_assets(None));
    shell.app.update_open_url = Arc::new(|_: &str| -> Result<Box<dyn Read + Send>, ureq::Error> {
        panic!("the test's network breaks the update's worker")
    });
    let egui_ctx = test_context();
    update_now(&egui_ctx, &mut shell.app);

    let nodes = frame_until_text(
        &egui_ctx,
        &mut shell.app,
        "The update stopped unexpectedly before it could finish.",
    );
    text_node(&nodes, "Update failed");
    text_node(&nodes, WAY_OUT);
    assert!(!has_text(&nodes, "Nothing was changed"));
    fs::remove_dir_all(bundle).unwrap();
}

// Only a failure that left the program's folder as it was says so: when the
// old files could not be put back, something was changed.
#[test]
fn update_failed_says_nothing_was_changed_only_when_so() {
    let (mut shell, _data) = offer_shell("studio_core_update_not_restored");
    let egui_ctx = test_context();
    open_details(&egui_ctx, &mut shell.app);

    let step = || SwapStep::PlaceFile(PathBuf::from("quick_compile.bat"));
    let swap = InstallError::Swap {
        step: step(),
        source: std::io::Error::other("the folder is in use"),
    };
    let not_restored = InstallError::NotRestored {
        step: step(),
        source: std::io::Error::other("the folder is in use"),
        restore: std::io::Error::other("the old file is locked"),
        kept: PathBuf::from("bundle/.update/backup"),
    };
    for (error, second) in [(swap, NOTHING_CHANGED), (not_restored, WAY_OUT)] {
        shell.app.update_dialog.stage = Stage::Failed(Failure::of(&error));
        let nodes = idle(&egui_ctx, &mut shell.app, 3).nodes;
        text_node(&nodes, &error.to_string());
        text_node(&nodes, second);
        assert_eq!(
            has_text(&nodes, "Nothing was changed"),
            second == NOTHING_CHANGED,
            "{error}"
        );
    }
}
