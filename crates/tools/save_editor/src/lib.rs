//! The Save editor: the PES EDIT savefile's teams and players exchanged (Team TOML, the
//! Compiler's aesthetics patch, the teams list) and compared.
//!
//! `Tool` registers the tool with the shell: its settings section, its help chapter and its
//! `save-editor` command line (`spec/save_editor.md` "CLI"), and the window (`view/`: the
//! player list beside the selected row's player or team view). `pes_savefile` owns every byte and
//! every operation on the model; this crate owns the session (the file that is open) and the
//! command line over it.

mod cli;
mod diff;
mod events;
mod messages;
/// Replaying a repro bundle's journal (native only: bundles live on disk).
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called only from the module's tests")
)]
mod replay;
mod session;
mod settings;
mod teams_list;
#[cfg(test)]
mod test_support;
mod view;

use studio_core::widgets::{FIELD_SPACING, checkbox_without_text};
use studio_core::{CliError, HelpSection, HelpTopic, RunningPes, StudioTool, ToolContext};

use crate::messages::TOOL_ID;
use crate::session::selection::TeamCategory;
use crate::view::PositionsLayout;

/// The "Aesthetics from exports" row's hover text in the settings menu.
const AESTHETICS_FROM_EXPORTS_TIP: &str = "On when the cup's looks are compiled from exports: \
     the save's aesthetics are then read-only, behind Edit aesthetics anyway on the Appearance \
     screen.";

/// The Save editor, as registered with the `studio` binary. `view` is the window's
/// state.
pub struct Tool {
    view: view::SaveView,
    /// Whether the data folder and its imported rulesets were handed to the view: they are
    /// read on the first `view` frame, where the data folder is known; afterwards only the
    /// view's own import and × read the list again.
    imports_read: bool,
    /// The running-PES probe `Tool::view` hands `view::show`: the real
    /// process list in production, a stub reporting no game under `cargo
    /// test` — the file pickers' `#[cfg]` split
    /// (`comparator::default_open_dialog`). A unit test sets it to drive
    /// the running-game question.
    running: fn() -> Vec<RunningPes>,
}

impl Tool {
    /// The tool, with no save open.
    pub fn new() -> Tool {
        Tool {
            view: view::SaveView::new(),
            imports_read: false,
            running: running_probe(),
        }
    }

    /// The tool with `probe` standing in for the process list when it asks which PES games
    /// run: a test sets the games its command sees running, whatever the machine runs.
    #[cfg(feature = "test-support")]
    pub fn with_running_probe(self, probe: fn() -> Vec<RunningPes>) -> Tool {
        Tool {
            running: probe,
            ..self
        }
    }
}

impl Default for Tool {
    fn default() -> Tool {
        Tool::new()
    }
}

/// The running-PES probe: the real process list (`studio_core::running_pes`).
#[cfg(not(test))]
fn running_probe() -> fn() -> Vec<RunningPes> {
    studio_core::running_pes
}

/// `running_probe` under `cargo test`: no game at all — a test never reads
/// the real process list, which would open the Edit-mode question on a
/// machine with the fixture's PES running.
#[cfg(test)]
fn running_probe() -> fn() -> Vec<RunningPes> {
    || Vec::new()
}

impl StudioTool for Tool {
    fn id(&self) -> &'static str {
        TOOL_ID
    }

    fn label(&self) -> &'static str {
        "Save editor"
    }

    fn icon(&self) -> &'static str {
        studio_core::icons::SAVE_EDITOR
    }

    /// The unsaved-changes prompt guards the window's close (SE-GUI-05):
    /// a dirty session holds it and opens the prompt for `Close` (its
    /// Discard re-asks through `request_close`, answered once the session
    /// is dropped). A prompt already up retargets.
    fn hold_close(&mut self) -> bool {
        if self.view.is_dirty() {
            view::prompt::open(&mut self.view, view::prompt::Pending::Close);
            true
        } else {
            false
        }
    }

    fn view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        // A settings error counts as the defaults (the first built-in
        // ruleset, Live AATF off, the aesthetics lock off).
        let table = ctx.tool_settings(TOOL_ID);
        let tool_settings = settings::from_table(&table).unwrap_or_default();
        if !self.imports_read {
            self.view.data_dir = ctx.paths().data_dir.clone();
            self.view.exe_dir = ctx.paths().exe_dir.clone();
            self.view.imported = session::rules::imported(self.view.data_dir.as_deref());
            // State the tool writes itself (an open's last folder, the
            // recent saves an open or a save moves), so it is read once: a
            // per-frame sync would overwrite an open that ran between
            // frames before the write-back could carry it.
            self.view.last_folder = tool_settings.last_folder.clone();
            self.view.recents = tool_settings.recent_files.clone();
            self.imports_read = true;
        }
        self.view.aatf_ruleset = tool_settings.aatf_ruleset.clone();
        self.view.live_aatf = tool_settings.live_aatf;
        self.view.live_suggestions = tool_settings.live_suggestions;
        self.view.live_window = tool_settings.live_window;
        self.view.aesthetics_from_exports = tool_settings.aesthetics_from_exports;
        self.view.tactics.scroll_names = tool_settings.scroll_names;
        self.view.tactics.portraits = tool_settings.portraits;
        let team_category = TeamCategory::from_setting(&tool_settings.team_category);
        self.view.team_category = team_category;
        let positions_layout = PositionsLayout::from_setting(&tool_settings.positions_layout);
        self.view.positions_layout = positions_layout;
        self.view.formation_pictures_folder = tool_settings.formation_pictures_folder.clone();
        self.view.formation_pictures_flat = tool_settings.formation_pictures_flat;
        self.view.formation_pictures_settings = tool_settings.formation_pictures_settings;
        let common = ctx.common();
        self.view.hide_ids = common.fpc_hide_ids();
        // Everything the editor needs of a PES version is the open save's: its
        // own version's folder, exe check and picked-folder store. With no save
        // open, nothing is asked.
        if let Some(version) = self.view.save_version() {
            self.view.pes_exe_check = Some(ctx.pes_exe_check_for(version));
            let folder = common.pes_folder_for(version);
            self.view.pes_folder = Some(folder.clone());
            self.view.select_portraits(ui.ctx(), &folder, version);
        }
        self.view.recorder = ctx.recorder().clone();
        view::show(ui, &mut self.view, &self.running);
        if std::mem::take(&mut self.view.request_close) {
            ctx.request_close();
        }
        if let Some((version, folder)) = self.view.picked_pes_folder.take() {
            ctx.set_pes_folder_for(&folder, version);
        }
        if let Some(notice) = self.view.notice.take() {
            ctx.notify(notice);
        }
        // The toolbar's ruleset dropdown, its toggle and Show suggestions,
        // where Live AATF's list stands, the formation bar's Scroll and
        // Portraits checkboxes, the positions grid's layout toggle, the
        // selector's category, the formation pictures' folder, settings box
        // and grain box, the picker's last folder and the recent saves are
        // settings: the table as read with only those keys replaced, so any
        // other key survives (as `settings_view` writes
        // `aesthetics_from_exports`).
        if self.view.aatf_ruleset != tool_settings.aatf_ruleset
            || self.view.live_aatf != tool_settings.live_aatf
            || self.view.live_suggestions != tool_settings.live_suggestions
            || self.view.live_window != tool_settings.live_window
            || self.view.tactics.scroll_names != tool_settings.scroll_names
            || self.view.tactics.portraits != tool_settings.portraits
            || self.view.positions_layout != positions_layout
            || self.view.team_category != team_category
            || self.view.formation_pictures_folder != tool_settings.formation_pictures_folder
            || self.view.formation_pictures_flat != tool_settings.formation_pictures_flat
            || self.view.formation_pictures_settings != tool_settings.formation_pictures_settings
            || self.view.last_folder != tool_settings.last_folder
            || self.view.recents != tool_settings.recent_files
        {
            let mut table = table;
            table.insert(
                "aatf_ruleset".to_owned(),
                toml::Value::String(self.view.aatf_ruleset.clone()),
            );
            table.insert(
                "live_aatf".to_owned(),
                toml::Value::Boolean(self.view.live_aatf),
            );
            table.insert(
                "live_suggestions".to_owned(),
                toml::Value::Boolean(self.view.live_suggestions),
            );
            table.insert(
                "live_window".to_owned(),
                toml::Value::Boolean(self.view.live_window),
            );
            table.insert(
                "scroll_names".to_owned(),
                toml::Value::Boolean(self.view.tactics.scroll_names),
            );
            table.insert(
                "portraits".to_owned(),
                toml::Value::Boolean(self.view.tactics.portraits),
            );
            table.insert(
                "positions_layout".to_owned(),
                toml::Value::String(self.view.positions_layout.setting().to_owned()),
            );
            table.insert(
                "team_category".to_owned(),
                toml::Value::String(self.view.team_category.setting().to_owned()),
            );
            table.insert(
                "formation_pictures_folder".to_owned(),
                toml::Value::String(self.view.formation_pictures_folder.clone()),
            );
            table.insert(
                "formation_pictures_flat".to_owned(),
                toml::Value::Boolean(self.view.formation_pictures_flat),
            );
            table.insert(
                "formation_pictures_settings".to_owned(),
                toml::Value::Boolean(self.view.formation_pictures_settings),
            );
            match &self.view.last_folder {
                Some(folder) => table.insert(
                    "last_folder".to_owned(),
                    toml::Value::String(folder.display().to_string()),
                ),
                None => table.remove("last_folder"),
            };
            table.insert(
                "recent_files".to_owned(),
                toml::Value::try_from(&self.view.recents)
                    .expect("a recent save is a path and a version number"),
            );
            ctx.set_tool_settings(TOOL_ID, table);
        }
    }

    fn has_settings_view(&self) -> bool {
        true
    }

    fn settings_view(&mut self, ui: &mut egui::Ui, ctx: &ToolContext) {
        let mut table = ctx.tool_settings(TOOL_ID);
        let mut aesthetics = table
            .get("aesthetics_from_exports")
            .and_then(toml::Value::as_bool)
            .unwrap_or_default();
        let mut committed_aesthetics = false;
        egui::Grid::new("save_editor_settings")
            .num_columns(2)
            .spacing(egui::vec2(ui.spacing().item_spacing.x, FIELD_SPACING))
            .show(ui, |ui| {
                // The common grid's row shape: the label cell's tooltip
                // leads with the key, the widget's is the sentence alone.
                let label = ui.label("Aesthetics from exports").on_hover_text(format!(
                    "aesthetics_from_exports: {AESTHETICS_FROM_EXPORTS_TIP}"
                ));
                committed_aesthetics = checkbox_without_text(ui, &mut aesthetics)
                    .labelled_by(label.id)
                    .on_hover_text(AESTHETICS_FROM_EXPORTS_TIP)
                    .changed();
                ui.end_row();
            });
        // The table as read with only the changed key replaced, so any
        // other key survives.
        if committed_aesthetics {
            table.insert(
                "aesthetics_from_exports".to_owned(),
                toml::Value::Boolean(aesthetics),
            );
            ctx.set_tool_settings(TOOL_ID, table);
        }
    }

    fn default_settings(&self) -> toml::Table {
        settings::default_table()
    }

    fn help(&self) -> HelpSection {
        HelpSection {
            title: "Save editor",
            topics: vec![
                HelpTopic {
                    slug: "command-line",
                    title: "Running from a terminal",
                    body: include_str!("../help/01_command_line.md"),
                },
                HelpTopic {
                    slug: "window",
                    title: "The window",
                    body: include_str!("../help/02_window.md"),
                },
                HelpTopic {
                    slug: "abilities",
                    title: "Abilities & Skills",
                    body: include_str!("../help/03_abilities.md"),
                },
                HelpTopic {
                    slug: "appearance",
                    title: "Appearance",
                    body: include_str!("../help/04_appearance.md"),
                },
                HelpTopic {
                    slug: "team",
                    title: "The team view and the Team menu",
                    body: include_str!("../help/05_team.md"),
                },
                HelpTopic {
                    slug: "player",
                    title: "The Player menu",
                    body: include_str!("../help/06_player.md"),
                },
                HelpTopic {
                    slug: "savefile",
                    title: "The Savefile menu",
                    body: include_str!("../help/07_savefile.md"),
                },
                HelpTopic {
                    slug: "tactics",
                    title: "The Tactics editor",
                    body: include_str!("../help/08_tactics.md"),
                },
            ],
            messages: messages::catalog(),
        }
    }

    fn cli_command(&self) -> clap::Command {
        cli::command()
    }

    /// The open save's path, bracketed in the idle window title. No view draws it, so the
    /// toolbar keeps its one row.
    fn title_detail(&self) -> Option<String> {
        self.view.save_path().map(|path| path.display().to_string())
    }

    /// The open save as the repro bundle's state file: the save encoded as it
    /// would be saved now, named as its file; none with no save open.
    #[cfg(not(target_arch = "wasm32"))]
    fn repro_state(&self) -> Vec<(String, Vec<u8>)> {
        self.view.repro_state()
    }

    fn cli_run(&self, matches: &clap::ArgMatches, ctx: &ToolContext) -> Result<u8, CliError> {
        cli::run(matches, ctx, self.running)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn the_tool_is_registered_as_the_save_editor() {
        let tool = Tool::new();
        assert_eq!(tool.id(), "save-editor");
        assert_eq!(tool.label(), "Save editor");
        assert_eq!(
            tool.default_settings()["aatf_ruleset"].as_str(),
            Some("autumn26")
        );
        assert_eq!(tool.help().topics.len(), 8);
        // The settings menu lists the tool's section.
        assert!(tool.has_settings_view());
    }

    // The Set PES path button's ask reaches the common settings on the
    // frame it was made: the picked folder stored for the save's version with
    // its year as 20**, the selector's setting unmoved.
    // SE-GUI-50
    #[test]
    fn the_picked_pes_folder_is_stored_for_the_saves_version() {
        let dir = crate::test_support::scratch("lib_set_pes_folder");
        let mut settings = studio_core::Settings::default();
        settings.common.pes_version = pes_version::PesVersion::Pes19;
        let (ctx, shared, _events_rx, _requests_rx) =
            crate::test_support::shared_tool_context(&dir, settings);
        let expected = dir.join("Pro Evolution Soccer 20**");
        assert_ne!(
            std::path::PathBuf::from(&shared.lock().unwrap().common.pes_folder_path),
            expected,
            "premise: the setting does not hold the folder yet"
        );
        let mut tool = Tool::new();
        tool.view.picked_pes_folder = Some((
            pes_version::PesVersion::Pes17,
            dir.join("Pro Evolution Soccer 2017"),
        ));
        // With no save open the welcome panel's discovery would reach the
        // real Documents folder.
        tool.view.discover = Box::new(|_| Vec::new());

        let egui_ctx = egui::Context::default();
        // The welcome panel's title draws in the theme's window-title style.
        studio_core::theme::install(&egui_ctx);
        let output = egui_ctx.run_ui(egui::RawInput::default(), |ui| tool.view(ui, &ctx));
        output.drop_without_applying_deltas();
        assert_eq!(
            std::path::PathBuf::from(&shared.lock().unwrap().common.pes_folder_path),
            expected
        );
        assert_eq!(
            shared.lock().unwrap().common.pes_version,
            pes_version::PesVersion::Pes19,
            "the selector unmoved"
        );
        assert_eq!(tool.view.picked_pes_folder, None, "the ask was taken");
    }

    // The view's exe check is the context's per-version one for the open save,
    // refreshed every `Tool::view` frame: the Set PES path button comes and
    // goes with the folder the save's version resolves, never with a verdict
    // the view once held or with the selector's.
    // SE-GUI-50
    #[test]
    fn the_view_takes_the_contexts_exe_check_for_the_saves_version() {
        use pes_version::PesVersion;

        let dir = crate::test_support::scratch("lib_exe_check");
        // The selector's PES 2019 has a folder of its own holding its exe; the
        // save's PES 2021 resolves the context's `pes`, which is missing. So
        // only an ask for the save's version reads `NoFolder`.
        let selected = dir.join("pes19");
        std::fs::create_dir_all(&selected).unwrap();
        std::fs::write(selected.join("PES2019.exe"), b"").unwrap();
        let mut settings = studio_core::Settings::default();
        settings.common.pes_version = PesVersion::Pes19;
        settings
            .common
            .pes_folders
            .insert(PesVersion::Pes19, selected.display().to_string());
        let (ctx, _events_rx, _requests_rx) = crate::test_support::tool_context(&dir, settings);
        assert_eq!(
            ctx.pes_exe_check(),
            None,
            "premise: the sidebar has not run a check"
        );
        assert_eq!(
            ctx.pes_exe_check_for(PesVersion::Pes19),
            studio_core::ExeCheck::Found,
            "premise: an ask for the selector's version would read Found"
        );
        let mut tool = Tool::new();
        // A save open: its version's PES folder is asked, not the selector's.
        let path = dir.join("EDIT00000000");
        std::fs::write(
            &path,
            pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
        )
        .unwrap();
        tool.view.open(&path);
        tool.view.pes_exe_check = Some(studio_core::ExeCheck::Found);

        let egui_ctx = egui::Context::default();
        let output = egui_ctx.run_ui(egui::RawInput::default(), |ui| tool.view(ui, &ctx));
        output.drop_without_applying_deltas();
        assert_eq!(
            tool.view.pes_exe_check,
            Some(studio_core::ExeCheck::NoFolder),
            "the view's check is the save's version's, listed by the context"
        );
    }

    /// A `Tool` over `test`'s scratch folder, which holds one PES 2021 save
    /// in `backups/`: the tool, the context (its receivers and the settings
    /// handle held for the channels) and the egui context with the theme on
    /// (the welcome panel's title draws in its window-title style).
    struct OneSave {
        path: PathBuf,
        backups: PathBuf,
        tool: Tool,
        ctx: ToolContext,
        egui_ctx: egui::Context,
        _settings: std::sync::Arc<std::sync::Mutex<studio_core::Settings>>,
        _events_rx: crossbeam_channel::Receiver<studio_core::PipelineEventEnvelope>,
        _requests_rx: crossbeam_channel::Receiver<studio_core::ShellRequest>,
    }

    fn one_save(test: &str) -> OneSave {
        let dir = crate::test_support::scratch(test);
        let backups = dir.join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let path = backups.join("EDIT00000000");
        std::fs::write(
            &path,
            pes_savefile::fixture_save(pes_version::PesVersion::Pes21),
        )
        .unwrap();
        let (ctx, _settings, _events_rx, _requests_rx) =
            crate::test_support::shared_tool_context(&dir, studio_core::Settings::default());
        let mut tool = Tool::new();
        // With no save open the welcome panel's discovery would reach the
        // real Documents folder.
        tool.view.discover = Box::new(|_| Vec::new());
        let egui_ctx = egui::Context::default();
        egui_ctx.enable_accesskit();
        studio_core::theme::install(&egui_ctx);
        OneSave {
            path,
            backups,
            tool,
            ctx,
            egui_ctx,
            _settings,
            _events_rx,
            _requests_rx,
        }
    }

    /// The screen each `Tool::view` frame here reports (the pane the shell
    /// leaves a tool at the default window).
    const SCREEN: egui::Vec2 = egui::vec2(1224.0, 756.0);

    /// One `Tool::view` frame on `tool`'s egui context.
    fn view_frame(tool: &mut Tool, egui_ctx: &egui::Context, ctx: &ToolContext) {
        let output = egui_ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| tool.view(ui, ctx),
        );
        output.drop_without_applying_deltas();
    }

    /// `view_frame`, returning the frame's AccessKit nodes.
    fn view_nodes(
        tool: &mut Tool,
        egui_ctx: &egui::Context,
        ctx: &ToolContext,
        events: Vec<egui::Event>,
    ) -> Vec<egui::accesskit::Node> {
        let mut output = egui_ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events,
                ..Default::default()
            },
            |ui| tool.view(ui, ctx),
        );
        let nodes = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context")
            .nodes
            .into_iter()
            .map(|(_, node)| node)
            .collect();
        output.drop_without_applying_deltas();
        nodes
    }

    /// A click at `at` on `tool`'s view: press on one frame, release on the
    /// next, as the view tests' `pointer_at`.
    fn view_click(tool: &mut Tool, egui_ctx: &egui::Context, ctx: &ToolContext, at: egui::Pos2) {
        for pressed in [true, false] {
            view_nodes(
                tool,
                egui_ctx,
                ctx,
                vec![
                    egui::Event::PointerMoved(at),
                    egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }

    // An open writes `last_folder` into the settings, and the editor started
    // again from the same settings reads it back into the picker's start.
    // SE-GUI-26
    #[test]
    fn the_last_folder_survives_a_restart_through_the_settings() {
        let OneSave {
            path,
            backups,
            mut tool,
            ctx,
            egui_ctx,
            ..
        } = one_save("lib_last_folder");
        assert!(
            ctx.tool_settings(TOOL_ID).get("last_folder").is_none(),
            "premise: the setting holds no folder yet"
        );
        let expected = backups.display().to_string();
        // The first frame reads the setting into the view; the open, as every
        // open, writes it back on its own frame.
        view_frame(&mut tool, &egui_ctx, &ctx);
        tool.view.open(&path);
        view_frame(&mut tool, &egui_ctx, &ctx);
        assert_eq!(
            ctx.tool_settings(TOOL_ID)["last_folder"].as_str(),
            Some(expected.as_str()),
            "the open wrote the folder into the settings"
        );

        let mut restarted = Tool::new();
        restarted.view.discover = Box::new(|_| Vec::new());
        let starts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let record = std::rc::Rc::clone(&starts);
        restarted.view.open_dialog = Box::new(move |start| {
            record
                .borrow_mut()
                .push(start.map(std::path::Path::to_path_buf));
            None
        });
        view_frame(&mut restarted, &egui_ctx, &ctx);
        assert_eq!(
            restarted.view.last_folder,
            Some(backups.clone()),
            "read back at the restart"
        );

        // And the welcome panel's Open… starts the picker in the
        // remembered folder (the restart clause drawn).
        view_frame(&mut restarted, &egui_ctx, &ctx);
        let nodes = view_nodes(&mut restarted, &egui_ctx, &ctx, Vec::new());
        let open = nodes
            .iter()
            .filter(|node| {
                node.role() == egui::accesskit::Role::Button
                    && node
                        .label()
                        .is_some_and(|label| label.starts_with("Open\u{2026}"))
            })
            .filter_map(egui::accesskit::Node::bounds)
            .next()
            .expect("the Open… entry");
        view_click(
            &mut restarted,
            &egui_ctx,
            &ctx,
            egui::pos2(
                ((open.x0 + open.x1) / 2.0) as f32,
                ((open.y0 + open.y1) / 2.0) as f32,
            ),
        );
        assert_eq!(
            starts.borrow().as_slice(),
            [Some(backups)],
            "Open… starts the picker in the last opened folder"
        );
    }

    // Opens write `recent_files` into the settings with each save's path
    // and version, the last opened first, and the editor started again from
    // the same settings lists them in the welcome panel's recents in that order.
    // SE-GUI-90
    #[test]
    fn the_recent_saves_survive_a_restart_through_the_settings() {
        let OneSave {
            path,
            backups,
            mut tool,
            ctx,
            egui_ctx,
            ..
        } = one_save("lib_recents");
        assert!(
            ctx.tool_settings(TOOL_ID).get("recent_files").is_none(),
            "premise: the setting holds no recents yet"
        );
        // A PES 2017 save in a folder of its own beside `backups`.
        let other_dir = backups
            .parent()
            .expect("backups sits under the scratch folder")
            .join("other");
        std::fs::create_dir_all(&other_dir).unwrap();
        let other = other_dir.join("EDIT00000000");
        std::fs::write(
            &other,
            pes_savefile::fixture_save(pes_version::PesVersion::Pes17),
        )
        .unwrap();
        let entry = |path: &std::path::Path, version: i64| {
            toml::Value::Table(toml::Table::from_iter([
                (
                    "path".to_owned(),
                    toml::Value::String(path.display().to_string()),
                ),
                ("version".to_owned(), toml::Value::Integer(version)),
            ]))
        };
        view_frame(&mut tool, &egui_ctx, &ctx);
        tool.view.open(&path);
        tool.view.open(&other);
        view_frame(&mut tool, &egui_ctx, &ctx);
        assert_eq!(
            ctx.tool_settings(TOOL_ID)["recent_files"],
            toml::Value::Array(vec![entry(&other, 17), entry(&path, 21)]),
            "the opens wrote both saves with their versions, the last opened first"
        );

        let mut restarted = Tool::new();
        restarted.view.discover = Box::new(|_| Vec::new());
        view_frame(&mut restarted, &egui_ctx, &ctx);
        assert_eq!(
            restarted.view.recents,
            [
                crate::settings::RecentSave {
                    path: other,
                    version: pes_version::PesVersion::Pes17,
                },
                crate::settings::RecentSave {
                    path,
                    version: pes_version::PesVersion::Pes21,
                },
            ],
            "read back at the restart in the same order"
        );

        // And the welcome panel draws them in the same order under Recent
        // files.
        view_frame(&mut restarted, &egui_ctx, &ctx);
        let nodes = view_nodes(&mut restarted, &egui_ctx, &ctx, Vec::new());
        let heading = nodes
            .iter()
            .find(|node| {
                node.role() == egui::accesskit::Role::Label && node.value() == Some("Recent files")
            })
            .and_then(egui::accesskit::Node::bounds)
            .expect("the Recent files heading");
        let mut listed: Vec<(f64, &str)> = nodes
            .iter()
            .filter(|node| node.role() == egui::accesskit::Role::Button)
            .filter_map(|node| {
                let bounds = node.bounds()?;
                let label = node.label()?;
                (bounds.y0 > heading.y0 && label.starts_with("[PES")).then_some((bounds.y0, label))
            })
            .collect();
        listed.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            listed
                .iter()
                .map(|(_, label)| *label)
                .collect::<Vec<&str>>(),
            [
                format!("[PES 2017] other{sep}EDIT00000000"),
                format!("[PES 2021] backups{sep}EDIT00000000"),
            ],
            "both recents under the heading, newest first"
        );
    }

    // The view's journal is the context's own: a step recorded through the
    // view's recorder lands in the bundle "Report a problem" writes.
    #[test]
    fn the_view_records_into_the_contexts_journal() {
        let dir = crate::test_support::scratch("lib_recorder");
        let (ctx, _events_rx, _requests_rx) =
            crate::test_support::tool_context(&dir, studio_core::Settings::default());
        let mut tool = Tool::new();
        // With no save open the welcome panel's discovery would reach the
        // real Documents folder.
        tool.view.discover = Box::new(|_| Vec::new());
        let egui_ctx = egui::Context::default();
        // The welcome panel's title draws in the theme's window-title style.
        studio_core::theme::install(&egui_ctx);
        let output = egui_ctx.run_ui(egui::RawInput::default(), |ui| tool.view(ui, &ctx));
        output.drop_without_applying_deltas();

        tool.view.recorder.step(TOOL_ID, "the marker step");
        let lines = crate::test_support::journal_lines(ctx.recorder(), &dir);
        assert!(
            lines.contains(&"the marker step".to_owned()),
            "the view's recorder writes the context's journal: {lines:?}"
        );
    }
}
