//! The About popup the logo opens, as the draft `docs/prototypes/index.html` draws it: the
//! detailed version block a member pastes into a bug
//! report, with a Copy button and links to the release page, the repository and the
//! help: the Help link closes the popup and has the shell open the help window on the
//! `setting-up` chapter (ST-SIDE-06).

use pes_version::PesVersion;

use crate::tool::AppPaths;

/// The repository, from the workspace manifest.
pub(crate) const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
/// The release page: the repository's `releases` listing.
pub(crate) const RELEASE_PAGE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases");

/// The version block the popup shows and Copy puts on the clipboard, unchanged.
pub(crate) fn version_block(paths: &AppPaths, pes_version: PesVersion) -> String {
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let data = match &paths.data_dir {
        Some(dir) => dir.display().to_string(),
        // A GUI run with no settings file yet has no location until the first-run
        // dialog's choice lands; until then the block says so rather than guessing one.
        None => "not chosen yet".to_owned(),
    };
    [
        format!("{} {}", crate::APP_NAME, crate::shell::version_label()),
        format!("Build: {build}"),
        format!("Target: {}", env!("STUDIO_TARGET_TRIPLE")),
        format!("Data location: {data}"),
        format!("PES version: {pes_version}"),
    ]
    .join("\n")
}

/// Draws the popup while `open` holds and clears it when the user closes it (Close, the
/// backdrop or Escape). Returns `true` when the Help link was clicked, so the sidebar can
/// ask the shell for the help window — the shell closes the popup itself then, so the link
/// needs no close path of its own here. `Modal` carries no `ClosableTag`, so the Close
/// button returns a flag rather than calling `ui.close`.
pub(crate) fn show(
    ctx: &egui::Context,
    paths: &AppPaths,
    pes_version: PesVersion,
    open: &mut bool,
) -> bool {
    let block = version_block(paths, pes_version);
    let modal = crate::widgets::modal(
        ctx,
        egui::Id::unique("about-4cc-studio"),
        format!("About {}", crate::APP_NAME),
        |ui| {
            ui.label(egui::RichText::new(&block).monospace());
            let mut help = false;
            ui.horizontal(|ui| {
                ui.hyperlink_to("Release page", RELEASE_PAGE_URL);
                ui.hyperlink_to("Repository", REPOSITORY_URL);
                help = ui.link("Help").clicked();
            });
            let close = crate::widgets::modal_buttons(ui, |ui| {
                let close = ui.button("Close").clicked();
                if ui.button("Copy").clicked() {
                    ui.ctx().copy_text(block.clone());
                }
                close
            });
            (close, help)
        },
    );
    let (close, help) = modal.inner;
    if close || modal.should_close() {
        *open = false;
    }
    help
}
