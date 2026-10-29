//! The first-run dialog, as the draft `docs/prototypes/index.html` draws it: a modal with no
//! Cancel that asks where settings and data live (spec/core/distribution.md "Data location").
//! It is drawn instead of any other window while the app has no data directory: the choice is
//! needed to start. The browser build never asks, since it persists to browser storage, so
//! there `FirstRun` is a stub that is never up.

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use crate::settings::SETTINGS_FILE_NAME;
use crate::tool::ToolContext;

/// The dialog's state across frames: the user config folder the second choice offers
/// (`None` when the OS reports no home, which drops the choice) and the last failed
/// write's error line, shown under the choices.
#[cfg(not(target_arch = "wasm32"))]
pub(super) struct FirstRun {
    /// The folder "In my user folder" offers; `None` drops the choice.
    config_dir: Option<PathBuf>,
    /// The error line under the choices after a choice's write failed; the dialog
    /// stays open so the member picks the other location.
    error: Option<String>,
}

/// The browser build's stand-in: no data location is ever asked (the module doc).
#[cfg(target_arch = "wasm32")]
pub(super) struct FirstRun;

/// The width of each choice's column, its button as wide (the draft's `.firstrun .choice`).
#[cfg(not(target_arch = "wasm32"))]
const CHOICE_WIDTH: f32 = 299.0;
/// The space between the two choices' columns.
#[cfg(not(target_arch = "wasm32"))]
const CHOICE_GAP: f32 = 16.0;
/// The height of a choice's button.
#[cfg(not(target_arch = "wasm32"))]
const CHOICE_BUTTON_HEIGHT: f32 = 38.0;
/// The size of a choice button's text, larger than the body's.
#[cfg(not(target_arch = "wasm32"))]
const CHOICE_TEXT_SIZE: f32 = 15.0;
/// The space between a choice's button and its hint.
#[cfg(not(target_arch = "wasm32"))]
const HINT_GAP: f32 = 4.0;
/// The space between the title and the choices' row (the draft's `.firstrun`
/// top margin).
#[cfg(not(target_arch = "wasm32"))]
const ROW_TOP_GAP: f32 = 2.0;
/// The space between the choices' row and the footer line (the draft's
/// `.firstrun` bottom margin).
#[cfg(not(target_arch = "wasm32"))]
const ROW_BOTTOM_GAP: f32 = 8.0;

#[cfg(not(target_arch = "wasm32"))]
impl FirstRun {
    /// The state at startup; `config_dir` is the folder "In my user folder" offers.
    pub(super) fn new(config_dir: Option<PathBuf>) -> FirstRun {
        FirstRun {
            config_dir,
            error: None,
        }
    }

    /// Whether the dialog is up: while the app has no data directory (a first GUI
    /// start, or after a choice's write failed).
    pub(super) fn up(&self, ctx: &ToolContext) -> bool {
        ctx.paths().data_dir.is_none()
    }

    /// Draws the modal: one button per location with its hint under it, then the
    /// footer line and the write error when there is one. There is no Close or
    /// Cancel — the choice is needed to start — and Escape and the backdrop go
    /// unanswered: the dialog leaves only through a write that lands.
    pub(super) fn show(&mut self, egui_ctx: &egui::Context, ctx: &mut ToolContext) {
        let mut choice = None;
        egui::Modal::new(egui::Id::unique("first-run-data-location")).show(egui_ctx, |ui| {
            // egui puts a widget's item spacing after it, so the gap under the
            // title and the one under the row are set before each is drawn; the
            // footer and the error line keep egui's own.
            let item_spacing = ui.spacing().item_spacing.y;
            ui.spacing_mut().item_spacing.y = ROW_TOP_GAP;
            crate::theme::window_title(
                ui,
                format!(
                    "Where should {} keep its settings and data?",
                    crate::APP_NAME
                ),
            );
            ui.spacing_mut().item_spacing.y = ROW_BOTTOM_GAP;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = CHOICE_GAP;
                if choice_column(
                    ui,
                    "Next to the program",
                    "A portable data folder beside the program, which moves and is deleted \
                     with it",
                ) {
                    choice = Some(ctx.paths().exe_dir.join("data"));
                }
                if let Some(config_dir) = &self.config_dir {
                    let hint = format!(
                        "{}, which survives replacing or moving the program folder",
                        config_dir.display()
                    );
                    if choice_column(ui, "In my user folder", &hint) {
                        choice = Some(config_dir.clone());
                    }
                }
            });
            ui.spacing_mut().item_spacing.y = item_spacing;
            ui.weak("You can move the data later by moving the folder.");
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
        if let Some(dir) = choice {
            self.choose(ctx, dir);
        }
    }

    /// Writes the choice: the folder created if needed, then the settings file saved
    /// into it — the file's existence is what makes the choice stick, a run that
    /// wrote nothing would ask again at the next start. On success the context's
    /// data directory becomes the folder, which closes the dialog; on failure the
    /// error line under the choices keeps it open.
    fn choose(&mut self, ctx: &mut ToolContext, dir: PathBuf) {
        let path = dir.join(SETTINGS_FILE_NAME);
        let written =
            std::fs::create_dir_all(&dir).and_then(|()| ctx.settings().lock().unwrap().save(&path));
        match written {
            Ok(()) => ctx.set_data_dir(dir),
            Err(error) => {
                self.error = Some(format!(
                    "Cannot write {} ({error}). Choose the other location.",
                    path.display()
                ));
            }
        }
    }
}

/// One choice's column, `CHOICE_WIDTH` wide: the button labelled `label` filling it, then
/// `hint` in the weak colour, wrapped inside the column. Returns whether the button was
/// clicked.
#[cfg(not(target_arch = "wasm32"))]
fn choice_column(ui: &mut egui::Ui, label: &str, hint: &str) -> bool {
    ui.vertical(|ui| {
        ui.set_width(CHOICE_WIDTH);
        ui.spacing_mut().item_spacing.y = HINT_GAP;
        let button = egui::Button::new(egui::RichText::new(label).size(CHOICE_TEXT_SIZE))
            .min_size(egui::vec2(CHOICE_WIDTH, CHOICE_BUTTON_HEIGHT));
        let clicked = ui.add(button).clicked();
        ui.weak(hint);
        clicked
    })
    .inner
}

#[cfg(target_arch = "wasm32")]
impl FirstRun {
    /// The browser build asks nothing; the parameter exists for the shared signature.
    pub(super) fn new(_config_dir: Option<PathBuf>) -> FirstRun {
        FirstRun
    }

    /// Never up: the browser build asks nothing.
    pub(super) fn up(&self, _ctx: &ToolContext) -> bool {
        false
    }

    /// Draws nothing: the browser build asks nothing.
    pub(super) fn show(&mut self, _egui_ctx: &egui::Context, _ctx: &mut ToolContext) {}
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::settings::Settings;
    use crate::shell::test_harness::*;
    use crate::tool::AppPaths;

    const TITLE: &str = concat!(
        "Where should ",
        crate::app_name!(),
        " keep its settings and data?"
    );

    /// A shell facing the first-run choice: `exe` as the folder holding the program
    /// (created by the caller when the choice must be writable), no data directory
    /// yet, `config_dir` as given (`None` drops the second choice).
    fn first_run_shell(settings: Settings, exe: PathBuf, config_dir: Option<PathBuf>) -> Shell {
        let (a, calls_a) = stub("a", "A", None);
        shell_in(
            settings,
            vec![a],
            &[],
            &[],
            AppPaths {
                exe_dir: exe,
                data_dir: None,
            },
            config_dir,
            vec![calls_a],
        )
    }

    // ST-RUN-01
    #[test]
    fn the_first_run_dialog_writes_the_settings_file_to_the_user_folder() {
        let guard = TestDir::new("studio_core_first_run_config");
        let dir = guard.path();
        let config = dir.join("config");
        let settings = test_settings(dir);
        let Shell { mut app, calls, .. } =
            first_run_shell(settings.clone(), dir.join("exe"), Some(config.clone()));
        let egui_ctx = test_context();

        // The centered modal settles into place a frame after it first draws (its
        // first frame is a sizing pass; egui 0.37 hides invisible widgets from the
        // AccessKit tree), so the nodes come from a settled frame.
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        assert!(
            has_text(&nodes, TITLE),
            "the dialog asks before any tool view"
        );
        entry(&nodes, "Next to the program");
        entry(&nodes, "In my user folder");
        assert!(has_text(
            &nodes,
            "portable data folder beside the program, which moves and is deleted with it"
        ));
        assert!(has_text(
            &nodes,
            "survives replacing or moving the program folder"
        ));
        assert!(has_text(
            &nodes,
            "You can move the data later by moving the folder."
        ));
        assert_eq!(calls[0].views(), 0, "no tool view while the dialog is up");

        click(&egui_ctx, &mut app, entry(&nodes, "In my user folder"));
        let path = config.join(crate::settings::SETTINGS_FILE_NAME);
        assert!(path.is_file(), "the choice wrote the settings file");
        let written = Settings::load(&path).expect("the settings file parses");
        assert_eq!(written, settings, "the in-memory settings landed");
        assert_eq!(
            app.ctx.paths().data_dir.as_deref(),
            Some(config.as_path()),
            "the context's data directory is the chosen folder"
        );

        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, TITLE),
            "the dialog is gone once the file exists"
        );
        assert_eq!(calls[0].views(), 1, "the active tool's view runs now");
    }

    // The draft's `.firstrun` row: two columns 299 points wide and 16 apart, each a
    // button as wide as its column and 38 tall, then its hint wrapped inside the
    // column on as many lines as its text needs. The user folder is the draft's own
    // sample path, whose hint takes three lines, as in the draft; a longer user name
    // takes more.
    #[test]
    fn the_two_choices_are_equal_columns_with_full_width_buttons_and_wrapped_hints() {
        let dir = TestDir::new("studio_core_first_run_layout");
        let config = PathBuf::from(r"C:\Users\you\AppData\Roaming\unlimited-tool-works");
        let Shell { mut app, .. } = first_run_shell(
            test_settings(dir.path()),
            dir.path().join("exe"),
            Some(config),
        );
        let egui_ctx = test_context();

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        let bounds = |node: &egui::accesskit::Node| node.bounds().expect("a drawn node has bounds");
        let hint = |needle: &str| {
            bounds(
                nodes
                    .iter()
                    .find(|node| node.value().is_some_and(|text| text.contains(needle)))
                    .unwrap_or_else(|| panic!("no hint containing {needle:?}")),
            )
        };
        let buttons = [
            bounds(entry(&nodes, "Next to the program")),
            bounds(entry(&nodes, "In my user folder")),
        ];
        let hints = [
            hint("portable data folder beside the program"),
            hint("which survives replacing or moving the program folder"),
        ];
        // A body text row as a label lays it out: 15 points, where the 12.5-point
        // font's own row height is 14.66.
        let row_height = 15.0;

        for (button, hint) in buttons.iter().zip(&hints) {
            assert!(
                (button.width() - 299.0).abs() <= 0.5 && (button.height() - 38.0).abs() <= 0.5,
                "a button fills its 299-point column and is 38 tall: {button:?}"
            );
            assert!(
                hint.width() <= 299.5 && (hint.x0 - button.x0).abs() <= 0.5,
                "a hint wraps inside its column {button:?}: {hint:?}"
            );
            assert!(
                hint.height() <= 3.0 * row_height + 0.5,
                "the sample hints take at most three lines of {row_height}: {hint:?}"
            );
        }
        let gap = buttons[1].x0 - buttons[0].x1;
        assert!(
            (gap - 16.0).abs() <= 0.5,
            "the columns stand 16 apart, not {gap}"
        );
    }

    // The draft's `.firstrun { margin: 2px 0 8px }`, box to box: the choices'
    // row starts 2 points under the title's line, and the footer line 8 points
    // under the row's lowest hint.
    #[test]
    fn the_choices_row_sits_2_under_the_title_and_8_above_the_footer() {
        let dir = TestDir::new("studio_core_first_run_margins");
        let config = PathBuf::from(r"C:\Users\you\AppData\Roaming\unlimited-tool-works");
        let Shell { mut app, .. } = first_run_shell(
            test_settings(dir.path()),
            dir.path().join("exe"),
            Some(config),
        );
        let egui_ctx = test_context();

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        let bounds_of = |needle: &str| {
            nodes
                .iter()
                .find(|node| node.value().is_some_and(|text| text.contains(needle)))
                .unwrap_or_else(|| panic!("no text containing {needle:?}"))
                .bounds()
                .expect("a drawn node has bounds")
        };
        let title = bounds_of(TITLE);
        let row_top = entry(&nodes, "Next to the program")
            .bounds()
            .expect("a drawn button has bounds")
            .y0;
        let row_bottom = bounds_of("portable data folder beside the program")
            .y1
            .max(bounds_of("which survives replacing or moving the program folder").y1);
        let footer = bounds_of("You can move the data later by moving the folder.");

        let above = row_top - title.y1;
        assert!(
            (above - 2.0).abs() <= 0.5,
            "the row starts 2 under the title, not {above}"
        );
        let below = footer.y0 - row_bottom;
        assert!(
            (below - 8.0).abs() <= 0.5,
            "the footer starts 8 under the row, not {below}"
        );
    }

    // ST-RUN-01
    #[test]
    fn the_portable_choice_writes_next_to_the_program() {
        let guard = TestDir::new("studio_core_first_run_portable");
        let dir = guard.path();
        let exe = dir.join("exe");
        fs::create_dir_all(&exe).unwrap();
        let Shell { mut app, calls, .. } =
            first_run_shell(test_settings(dir), exe.clone(), Some(dir.join("config")));
        let egui_ctx = test_context();

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Next to the program"));

        let data = exe.join("data");
        let written = data.join(crate::settings::SETTINGS_FILE_NAME);
        assert!(
            written.is_file(),
            "the choice wrote <exe>/data/settings.toml"
        );
        // `load` on a missing file yields the defaults, so the parse is asserted only
        // for a file that exists.
        Settings::load(&written).expect("the settings file parses");
        assert_eq!(
            app.ctx.paths().data_dir.as_deref(),
            Some(data.as_path()),
            "the context's data directory is the portable folder"
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(!has_text(&nodes, TITLE));
        assert_eq!(calls[0].views(), 1);
    }

    // ST-RUN-01
    #[test]
    fn an_existing_data_dir_shows_no_dialog() {
        let dir = TestDir::new("studio_core_first_run_existing");
        let (a, calls_a) = stub("a", "A", None);
        let Shell { mut app, calls, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(!has_text(&nodes, TITLE), "no dialog: the location exists");
        assert!(!has_label(&nodes, "Next to the program"));
        assert_eq!(calls[0].views(), 1, "the view runs on the first frame");
    }

    // ST-RUN-01
    #[test]
    fn without_a_config_dir_the_dialog_offers_the_portable_choice_alone() {
        let dir = TestDir::new("studio_core_first_run_noconfig");
        let Shell { mut app, .. } =
            first_run_shell(test_settings(dir.path()), dir.path().join("exe"), None);
        let egui_ctx = test_context();

        // The modal's first frame is a sizing pass; its contents join the
        // AccessKit tree from the next frame on.
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        assert!(has_text(&nodes, TITLE));
        entry(&nodes, "Next to the program");
        assert!(
            !has_label(&nodes, "In my user folder"),
            "no user-folder choice without a config dir"
        );
    }

    // A location that cannot be written keeps the dialog open with the error, so the
    // member picks the other. The exe path is a
    // regular file, so `<exe>/data` cannot be created on any platform.
    #[test]
    fn a_failed_write_keeps_the_dialog_open_with_the_error() {
        let guard = TestDir::new("studio_core_first_run_denied");
        let dir = guard.path();
        let exe = dir.join("exe");
        fs::write(&exe, b"not a folder").unwrap();
        let config = dir.join("config");
        let Shell { mut app, calls, .. } =
            first_run_shell(test_settings(dir), exe.clone(), Some(config.clone()));
        let egui_ctx = test_context();

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Next to the program"));
        // The failed write is processed after the modal's widgets; the error line is
        // drawn from the next frame — and the taller modal settles a frame after that.
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        let wanted = exe.join("data").join("settings.toml");
        assert!(
            has_text(&out.nodes, &format!("Cannot write {}", wanted.display())),
            "the error names the settings file that failed"
        );
        assert!(has_text(&out.nodes, TITLE), "the dialog stays open");
        assert_eq!(app.ctx.paths().data_dir, None, "no data directory yet");
        assert_eq!(calls[0].views(), 0, "still no tool view");

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "In my user folder"));
        let written = config.join(crate::settings::SETTINGS_FILE_NAME);
        assert!(
            written.is_file(),
            "the other location wrote the settings file"
        );
        Settings::load(&written).expect("the settings file parses");
        assert_eq!(app.ctx.paths().data_dir.as_deref(), Some(config.as_path()));
    }
}
