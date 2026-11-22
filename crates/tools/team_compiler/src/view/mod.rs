//! The Compiler's window, render-only (`spec/team_compiler/README.md` "Crate layout"): it
//! reads the tool's state (the run log, the grid's rows) and reports the user's intent
//! (Compile clicked); it never calls into the pipeline itself. It shows the Compile button,
//! the export grid under its legend, and the log panel; the toolbar is not built yet.

use std::time::Duration;

use export_grid::{CellState, ColumnGroup, Columns, LegendEntry, Outline, Row, Width};
use studio_core::widgets::add_enabled;
use studio_core::{HelpTarget, LogPanel, ToolContext};

use crate::gui_run::RunLog;
use crate::messages::TOOL_ID;

/// The grid's column groups: a player's 23 positions, the portraits, the 10 kits.
const GROUPS: [ColumnGroup; 3] = [
    ColumnGroup {
        title: "Players",
        columns: 23,
        width: Width::Square,
    },
    ColumnGroup {
        title: "Portraits",
        columns: 1,
        width: Width::Wide,
    },
    ColumnGroup {
        title: "Kits",
        columns: 10,
        width: Width::Square,
    },
];

/// The grid's columns: the team's ID beside its label, the groups, and the row actions'
/// column, empty until the row actions are built.
const COLUMNS: Columns = Columns {
    id_title: Some("ID"),
    line_names: false,
    groups: &GROUPS,
    last_title: "Actions",
};

/// A legend entry for `state` with `outline`, named `name`.
const fn entry(state: CellState, outline: Option<Outline>, name: &'static str) -> LegendEntry {
    LegendEntry {
        state,
        outline,
        name,
    }
}

/// The legend's columns: pairs of related states, and the two states with no pair alone.
const LEGEND: [&[LegendEntry]; 6] = [
    &[
        entry(CellState::Unchecked, None, "Unchecked"),
        entry(CellState::Checking, None, "Checking"),
    ],
    &[entry(
        CellState::PartialOk,
        None,
        "Shallow check done\n(archive)",
    )],
    &[
        entry(CellState::FullOk, Some(Outline::Warning), "Warning"),
        entry(CellState::FullOk, Some(Outline::Error), "Error"),
    ],
    &[
        entry(CellState::DoneWithWarnings, None, "Done with warnings"),
        entry(CellState::DoneWithErrors, None, "Done with errors"),
    ],
    &[
        entry(CellState::FullOk, None, "Checked"),
        entry(CellState::Done, None, "Done"),
    ],
    &[entry(CellState::Blocked, None, "Blocked")],
];

/// Shows the window over `rows`, the grid's check's, drawn as being checked while `checking`;
/// `true` when Compile was clicked this frame.
pub(crate) fn show(
    ui: &mut egui::Ui,
    ctx: &ToolContext,
    log: &RunLog,
    panel: &mut LogPanel,
    rows: &[Row],
    checking: bool,
) -> bool {
    // The dock claims the bottom first (`spec/core/gui.md` "Shell layout"); the
    // button and the grid take what is left. A
    // clicked message code is a shell request, not a pipeline call: the view
    // stays render-only.
    if let Some(code) = panel.show(ui, log.lines()) {
        ctx.open_help(HelpTarget::message(&code));
    }
    let clicked = add_enabled(ui, !log.is_running(), egui::Button::new("Compile")).clicked();
    export_grid::legend(ui, (TOOL_ID, "legend"), &LEGEND);
    export_grid::show(ui, (TOOL_ID, "grid"), &COLUMNS, rows, checking);
    if log.is_running() || checking {
        // egui repaints only on input, and the run's events and the check's rows arrive on
        // their own: without this the window would advance only when the mouse moves.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    clicked
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crossbeam_channel::unbounded;
    use egui::accesskit;
    use export_grid::{Cell, IdCell, Position, RowBody};
    use studio_core::StudioTool;

    use super::*;
    use crate::Tool;
    use crate::check::live::LiveCheck;
    use crate::testing::{copy_tree, install_pes, poll_until_done, sandbox, tool_context};

    /// A headless window: one egui context with AccessKit on and animations off (so a
    /// collapsing header is fully open on the frame after its click), run a frame at a time.
    struct Headless {
        ctx: egui::Context,
    }

    impl Headless {
        fn new() -> Headless {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            // The window draws with these fonts and style; the test measures
            // what a member sees, whichever crates the build unifies.
            studio_core::theme::install(&ctx);
            ctx.global_style_mut(|style| style.animation_time = 0.0);
            Headless { ctx }
        }

        /// Runs one frame of `contents` on a 1280 by 780 screen, the window's own size, with
        /// `events` as its input; returns what `contents` returned and the frame's AccessKit
        /// nodes.
        fn frame<R>(
            &mut self,
            events: Vec<egui::Event>,
            mut contents: impl FnMut(&mut egui::Ui) -> R,
        ) -> (R, Vec<accesskit::Node>) {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 780.0),
                )),
                events,
                ..Default::default()
            };
            let mut result = None;
            let mut output = self.ctx.run_ui(input, |ui| result = Some(contents(ui)));
            let update = output
                .platform_output
                .accesskit_update
                .take()
                .expect("AccessKit is enabled on the context");
            // Nothing renders here, so the frame's texture deltas are dropped on purpose.
            output.drop_without_applying_deltas();
            let nodes = update.nodes.into_iter().map(|(_, node)| node).collect();
            (result.expect("run_ui calls the closure"), nodes)
        }
    }

    /// The node of the button (or collapsing header) labeled `label`.
    fn button<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
        nodes
            .iter()
            .find(|node| node.label() == Some(label))
            .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
    }

    /// Whether some label widget shows exactly `text`. egui puts a label's text in its
    /// AccessKit node's value, not its label.
    fn shows_text(nodes: &[accesskit::Node], text: &str) -> bool {
        nodes.iter().any(|node| node.value() == Some(text))
    }

    /// The centre of a node's bounds, where a click lands.
    fn centre(node: &accesskit::Node) -> egui::Pos2 {
        let bounds = node.bounds().expect("a widget has bounds");
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    /// The pointer moved to `pos` and the primary button went down there.
    fn press(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    /// The primary button came up at `pos`: egui reports the click on this frame.
    fn release(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }]
    }

    /// A context with no sandbox behind it, for frames that start no run.
    fn settings_only_context() -> ToolContext {
        tool_context(Path::new("exe"), "")
    }

    /// The tracer export, whose player and goalkeeper kit the grid's exports are built from.
    fn tracer() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer/studio/egg Midcup Tracer")
    }

    /// `root`'s teams list: `702 /a/`, `714 /co/` and the sandbox's `792 /egg/`.
    fn list_teams(root: &Path) {
        fs::write(
            root.join("data/teams_list.txt"),
            "ID\tName\n702\t/a/\n714\t/co/\n792\t/egg/\n",
        )
        .unwrap();
    }

    /// The player folder `folder` of `root`'s export `export`, a copy of the tracer's player,
    /// which checks clean.
    fn add_player(root: &Path, export: &str, folder: &str) {
        let player = tracer().join("Players/05 - The Chad Stormworks Player");
        copy_tree(
            &player,
            &root
                .join("exports")
                .join(export)
                .join("Players")
                .join(folder),
        );
    }

    /// The kit folder `folder` of `root`'s export `export`, a copy of the tracer's
    /// goalkeeper kit.
    fn add_kit(root: &Path, export: &str, folder: &str) {
        copy_tree(
            &tracer().join("Kits/g1"),
            &root.join("exports").join(export).join("Kits").join(folder),
        );
    }

    /// Writes `bytes` to `path` under `root`'s exports, its folders created.
    fn add_file(root: &Path, path: &str, bytes: &[u8]) {
        let path = root.join("exports").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    /// The rows the tool's grid check makes of `root`'s exports.
    fn checked_rows(root: &Path) -> Vec<Row> {
        let ctx = tool_context(root, "");
        let mut tool = Tool::new();
        tool.live_check.start(&ctx);
        finish_check(&mut tool, &ctx);
        tool.live_check.rows().to_vec()
    }

    /// The row labelled `name`.
    fn row<'a>(rows: &'a [Row], name: &str) -> &'a Row {
        rows.iter()
            .find(|row| row.label.text == name)
            .unwrap_or_else(|| panic!("no row {name:?}"))
    }

    /// The cell at `position` of `row`'s line `line`, column group `group`.
    fn position(row: &Row, line: usize, group: usize, position: usize) -> Option<&Cell> {
        let RowBody::Lines(lines) = &row.body else {
            panic!("{} has a message in place of its cells", row.label.text);
        };
        match &lines[line].groups[group][position] {
            Position::Cell(cell) => Some(cell),
            Position::Empty => None,
        }
    }

    /// Player `slot`'s cell: 01 to 23 on the first line, a referee's 24 to 35 on the second.
    fn player(row: &Row, slot: usize) -> Option<&Cell> {
        position(row, (slot - 1) / 23, 0, (slot - 1) % 23)
    }

    /// The portraits' cell.
    fn portraits(row: &Row) -> Option<&Cell> {
        position(row, 0, 1, 0)
    }

    /// The kit cell of column `column`: 0 is GK, then K1 to K9.
    fn kit(row: &Row, column: usize) -> Option<&Cell> {
        position(row, 0, 2, column)
    }

    /// The codes of `cell`'s findings.
    fn codes(cell: &Cell) -> Vec<&str> {
        cell.messages
            .iter()
            .map(|message| message.code.code.as_ref())
            .collect()
    }

    /// The texts of the painted cells on the line of the node reading `label`, left to right.
    fn line_texts(nodes: &[accesskit::Node], label: &str) -> Vec<String> {
        let on = nodes
            .iter()
            .find(|node| node.value() == Some(label))
            .and_then(accesskit::Node::bounds)
            .unwrap_or_else(|| panic!("no node reads {label:?}"));
        let mut cells: Vec<(f64, String)> = nodes
            .iter()
            .filter_map(|node| {
                let bounds = node.bounds()?;
                let text = node.value()?;
                let middle = (bounds.y0 + bounds.y1) / 2.0;
                (bounds.x0 > on.x1 && on.y0 < middle && middle < on.y1)
                    .then(|| (bounds.x0, text.to_owned()))
            })
            .collect();
        cells.sort_by(|a, b| a.0.total_cmp(&b.0));
        cells.into_iter().map(|(_, text)| text).collect()
    }

    /// Ticks `tool` until its grid check has ended and its rows are in.
    fn finish_check(tool: &mut Tool, ctx: &ToolContext) {
        poll_until_done(|| {
            tool.tick(ctx);
            tool.live_check.is_running()
        });
    }

    // TC-GUI-03
    #[test]
    fn without_a_click_nothing_is_reported_and_compile_is_enabled() {
        let ctx = settings_only_context();
        let log = RunLog::default();
        let mut panel = LogPanel::new(TOOL_ID);
        let (clicked, nodes) = Headless::new().frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        assert!(!clicked);
        assert!(!button(&nodes, "Compile").is_disabled());
    }

    #[test]
    fn a_click_on_compile_is_reported_on_the_frame_of_the_release() {
        let ctx = settings_only_context();
        let log = RunLog::default();
        let mut panel = LogPanel::new(TOOL_ID);
        let mut window = Headless::new();
        // The first frame lays the button out; egui hit-tests a click against the previous
        // frame's widgets, so the press comes on the second frame and the release on the third.
        let (_, nodes) = window.frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        let target = centre(button(&nodes, "Compile"));
        let (clicked, _) = window.frame(press(target), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        assert!(!clicked, "not before the release");
        let (clicked, _) = window.frame(release(target), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        assert!(clicked);
    }

    // TC-GUI-03
    #[test]
    fn compile_is_disabled_while_a_run_is_in_flight() {
        let ctx = settings_only_context();
        let (release_run, until) = unbounded();
        let mut log = RunLog::in_flight(until);
        let mut panel = LogPanel::new(TOOL_ID);
        let (_, nodes) = Headless::new().frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        assert!(button(&nodes, "Compile").is_disabled());
        release_run.send(()).unwrap();
        poll_until_done(|| {
            log.poll();
            log.is_running()
        });
        assert_eq!(log.texts(), ["Run finished with exit code 0"]);
    }

    // TC-GUI-02
    #[test]
    fn a_compile_click_through_the_tool_starts_a_run() {
        let temp = sandbox("view_compile_click");
        let root = temp.path();
        install_pes(root);
        let ctx = tool_context(root, "");
        let mut tool = Tool::new();
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        let target = centre(button(&nodes, "Compile"));
        window.frame(press(target), |ui| tool.view(ui, &ctx));
        window.frame(release(target), |ui| tool.view(ui, &ctx));
        assert!(tool.run_log.is_running());
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert_eq!(
            tool.run_log.texts(),
            [
                "- Info bin_source (bin=TeamColor.bin, cpk=bundled)".to_owned(),
                "- Info bin_source (bin=UniColor.bin, cpk=bundled)".to_owned(),
                "- Info bin_source (bin=UniformParameter.bin, cpk=bundled)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=boots.fmdl, count=1662)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=fcl_hair.fmdl, count=1662)".to_owned(),
                "- egg Midcup Tracer: Info fmdl_weights_not_normalized at Players/05 - The Chad Stormworks Player (file=glove_l.fmdl, count=2)".to_owned(),
                "- egg Midcup Tracer: Info export_identified (team=/egg/, id=792)".to_owned(),
                format!(
                    "- Info patch_written (path={}, teams=792)",
                    root.join("output")
                        .join("aesthetics_patch.toml")
                        .display()
                ),
                format!(
                    "- Warning savefile_missing (patch={}, path={}, command={})",
                    root.join("output")
                        .join("aesthetics_patch.toml")
                        .display(),
                    root.join("save").join("EDIT00000000").display(),
                    crate::output::savefile::APPLY_COMMAND
                ),
                "Run finished with exit code 0".to_owned(),
            ]
        );
        // The view started the grid's check on its first frame, and the run's end another.
        finish_check(&mut tool, &ctx);
    }

    // TC-GUI-04
    #[test]
    fn the_view_shows_a_row_per_export_once_its_check_has_returned() {
        let temp = sandbox("view_grid_rows");
        let root = temp.path();
        list_teams(root);
        for folder in ["01 - A", "03 - B"] {
            add_player(root, "a Midcup Grid", folder);
        }
        for folder in ["g1", "p1"] {
            add_kit(root, "a Midcup Grid", folder);
        }
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/sources/egg Midcup Tracer.zip"),
            root.join("exports/co Midcup Tracer.zip"),
        )
        .unwrap();
        let ctx = tool_context(root, "");
        let mut tool = Tool::new();
        let mut window = Headless::new();
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        assert!(
            tool.live_check.is_running(),
            "the first frame starts the check"
        );
        assert!(
            !shows_text(&nodes, "/a/"),
            "no row before the check returns"
        );

        finish_check(&mut tool, &ctx);
        let (_, nodes) = window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        assert_eq!(
            line_texts(&nodes, "/a/"),
            ["702", "01", "03", "2/2", "GK", "K1"]
        );
        assert_eq!(line_texts(&nodes, "/co/"), ["714", "05", "1/1", "GK"]);
        let rows = tool.live_check.rows();
        let folder = row(rows, "/a/");
        let mut cells = vec![player(folder, 1), player(folder, 3), portraits(folder)];
        cells.extend([kit(folder, 0), kit(folder, 1)]);
        for cell in cells.into_iter().map(Option::unwrap) {
            assert_eq!(
                (cell.state, cell.outline),
                (CellState::FullOk, None),
                "{cell:?}"
            );
        }
        assert_eq!(player(folder, 2), None, "02 is an empty position");
        let archive = row(rows, "/co/");
        for cell in [player(archive, 5), kit(archive, 0)]
            .into_iter()
            .map(Option::unwrap)
        {
            assert_eq!(
                (cell.state, cell.outline),
                (CellState::PartialOk, None),
                "{cell:?}"
            );
        }
    }

    // TC-GUI-20
    #[test]
    fn a_referee_folder_backing_two_slots_colours_both() {
        let temp = sandbox("view_grid_referees");
        let root = temp.path();
        add_file(root, "refs Cup/players.txt", b"01 A\n02 B\n35 A\n");
        add_player(root, "refs Cup", "A");
        add_player(root, "refs Cup", "B");
        add_file(root, "refs Cup/Players/A/notes.exe", b"");
        let rows = checked_rows(root);
        let referees = row(&rows, "/refs/");
        assert_eq!(referees.id, Some(IdCell::Blank));
        let drawn: Vec<usize> = (1..=46)
            .filter(|slot| player(referees, *slot).is_some())
            .collect();
        assert_eq!(drawn, [1, 2, 35], "35 on the second line, under 12");
        let (first, last) = (player(referees, 1).unwrap(), player(referees, 35).unwrap());
        assert_eq!(first.outline, Some(Outline::Error));
        assert_eq!(
            (first.outline, &first.messages),
            (last.outline, &last.messages)
        );
        assert_eq!(codes(first), ["file_type_disallowed"]);
        assert_eq!(player(referees, 2).unwrap().outline, None);
    }

    // TC-GUI-21
    #[test]
    fn a_shared_folder_s_and_a_logo_s_findings_colour_the_label_alone() {
        let temp = sandbox("view_grid_label");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Label", "03 - A");
        add_kit(root, "co Midcup Label", "g1");
        add_file(root, "co Midcup Label/Boots/Solo/boots.fmdl", b"");
        let logo = fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
        add_file(root, "co Midcup Label/logo_small.dds", &logo);
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        assert_eq!(
            team.label.outline,
            Some(Outline::Error),
            "the worse of the two"
        );
        let label = codes(&team.label);
        for code in ["shared_folder_orphaned", "logo_small_without_main"] {
            assert!(label.contains(&code), "{code} in {label:?}");
        }
        for cell in [player(team, 3), kit(team, 0)]
            .into_iter()
            .map(Option::unwrap)
        {
            assert_eq!(cell.outline, None, "{cell:?}");
        }
    }

    // A finding about a player folder, or a file in one, is that player's: a dropped
    // folder's included, which keeps its slot's cell.
    #[test]
    fn a_player_folder_s_finding_colours_its_player() {
        let temp = sandbox("view_grid_player");
        let root = temp.path();
        list_teams(root);
        for folder in ["01 - A", "03 - B", "07 - C"] {
            add_player(root, "co Midcup Map", folder);
        }
        add_file(root, "co Midcup Map/Players/03 - B/notes.exe", b"");
        add_file(root, "co Midcup Map/Players/07 - C/fpc_on", b"");
        add_file(root, "co Midcup Map/Players/07 - C/fpc_off", b"");
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        assert_eq!(codes(player(team, 3).unwrap()), ["file_type_disallowed"]);
        assert_eq!(codes(player(team, 7).unwrap()), ["fpc_conflict"]);
        assert_eq!(player(team, 7).unwrap().outline, Some(Outline::Error));
        assert_eq!(player(team, 1).unwrap().outline, None);
        assert_eq!(team.label.outline, None, "{:?}", codes(&team.label));
    }

    // A player's portrait in his own folder is his cell's, not the portraits cell's.
    #[test]
    fn a_portrait_in_a_player_folder_colours_the_player() {
        let temp = sandbox("view_grid_own_portrait");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Map", "03 - B");
        add_file(
            root,
            "co Midcup Map/Players/03 - B/portrait.dds",
            b"not a texture",
        );
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        let his = player(team, 3).unwrap();
        let unreadable = his
            .messages
            .iter()
            .find(|message| message.code.code == "texture_unreadable")
            .unwrap_or_else(|| panic!("{:?}", codes(his)));
        assert!(
            matches!(&unreadable.scope, studio_core::Scope::File { path, .. } if path.as_str() == "Players/03 - B/portrait.dds"),
            "{:?}",
            unreadable.scope
        );
        assert_eq!(his.outline, Some(Outline::Error));
        assert_eq!(portraits(team).unwrap().messages, Vec::new());
    }

    // A file of `Portraits/` is the portraits cell's.
    #[test]
    fn a_portraits_file_s_finding_colours_the_portraits_cell() {
        let temp = sandbox("view_grid_portraits");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Map", "03 - B");
        let portrait =
            fs::read(tracer().join("Players/05 - The Chad Stormworks Player/portrait.dds"))
                .unwrap();
        add_file(root, "co Midcup Map/Portraits/player_03.dds", &portrait);
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        let cell = portraits(team).unwrap();
        assert_eq!(codes(cell), ["portrait_name_invalid"]);
        assert_eq!(cell.outline, Some(Outline::Error));
        assert_eq!(player(team, 3).unwrap().outline, None);
    }

    // A kit folder's finding is that kit's.
    #[test]
    fn a_kit_folder_s_finding_colours_its_kit() {
        let temp = sandbox("view_grid_kit");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Map", "03 - B");
        add_kit(root, "co Midcup Map", "g1");
        add_kit(root, "co Midcup Map", "p1 - Home");
        add_file(root, "co Midcup Map/Kits/p1 - Home/icon_99", b"");
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        let home = kit(team, 1).unwrap();
        assert!(
            codes(home).contains(&"kit_icon_invalid"),
            "{:?}",
            codes(home)
        );
        assert_eq!(home.outline, Some(Outline::Warning));
        assert_eq!(kit(team, 0).unwrap().outline, None, "GK");
    }

    // A kit folder validation drops keeps its cell: both layout markers in one kit
    // (TC-KIT-06's input), in a Midcup export, which gets no empty kit in its place.
    #[test]
    fn a_dropped_kit_folder_keeps_its_cell() {
        let temp = sandbox("view_grid_dropped_kit");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Map", "03 - B");
        add_kit(root, "co Midcup Map", "g1");
        add_kit(root, "co Midcup Map", "p1");
        add_file(root, "co Midcup Map/Kits/p1/pre-fox", b"");
        add_file(root, "co Midcup Map/Kits/p1/fox", b"");
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        let home = kit(team, 1).expect("K1 drawn");
        assert_eq!(codes(home), ["kit_layout_conflict"]);
        assert_eq!(home.outline, Some(Outline::Error));
    }

    // An export dropped whole is drawn from its listing: here a team the teams list lacks,
    // whose portraits count reads its `Portraits/` file. The copied player's own portrait is
    // removed, so the count can come from that file alone.
    #[test]
    fn a_dropped_export_counts_its_portraits_from_its_listing() {
        let temp = sandbox("view_grid_dropped_export");
        let root = temp.path();
        list_teams(root);
        add_player(root, "v Midcup Map", "03 - B");
        let own = root.join("exports/v Midcup Map/Players/03 - B/portrait.dds");
        let portrait = fs::read(&own).unwrap();
        fs::remove_file(own).unwrap();
        add_file(root, "v Midcup Map/Portraits/03.dds", &portrait);
        let rows = checked_rows(root);
        let team = row(&rows, "/v/");
        assert_eq!(team.id, Some(IdCell::Missing));
        assert!(codes(&team.label).contains(&"team_name_unknown"));
        assert_eq!(player(team, 3).unwrap().state, CellState::Blocked);
        assert_eq!(portraits(team).unwrap().text, "1/1");
    }

    // Anything else is the label's: here a player folder whose name claims no slot.
    #[test]
    fn a_player_folder_at_no_slot_colours_the_label() {
        let temp = sandbox("view_grid_no_slot");
        let root = temp.path();
        list_teams(root);
        add_player(root, "co Midcup Map", "03 - B");
        add_player(root, "co Midcup Map", "15_Snuffy");
        let rows = checked_rows(root);
        let team = row(&rows, "/co/");
        let label = codes(&team.label);
        assert!(label.contains(&"player_folder_number_invalid"), "{label:?}");
        assert_eq!(team.label.outline, Some(Outline::Error));
        assert_eq!(player(team, 15), None);
        assert_eq!(player(team, 3).unwrap().outline, None);
    }

    /// A row a test check hands back: /egg/'s label alone.
    fn label_only_row() -> Row {
        Row {
            label: Cell::new("/egg/", CellState::FullOk),
            id: Some(IdCell::Known("792".to_owned())),
            body: RowBody::Lines(Vec::new()),
        }
    }

    #[test]
    fn a_check_s_rows_reach_the_grid_only_once_the_compile_has_ended() {
        let temp = sandbox("view_grid_compile_guard");
        let ctx = tool_context(temp.path(), "");
        let mut tool = Tool::new();
        let (release_check, check_until) = unbounded();
        tool.live_check = LiveCheck::in_flight(check_until, vec![label_only_row()]);
        let (release_run, run_until) = unbounded();
        tool.run_log = RunLog::in_flight(run_until);
        release_check.send(()).unwrap();
        poll_until_done(|| !tool.live_check.has_ended());
        tool.tick(&ctx);
        assert!(
            tool.live_check.rows().is_empty(),
            "held while the Compile runs"
        );

        release_run.send(()).unwrap();
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert_eq!(tool.live_check.rows(), [label_only_row()]);
        assert!(tool.live_check.is_running(), "the run's end starts a check");
        finish_check(&mut tool, &ctx);
        assert_eq!(tool.live_check.rows()[0].label.text, "/egg/");
    }

    #[test]
    fn a_compile_s_end_starts_a_check() {
        let temp = sandbox("view_grid_compile_end");
        let ctx = tool_context(temp.path(), "");
        let mut tool = Tool::new();
        let (release_run, run_until) = unbounded();
        tool.run_log = RunLog::in_flight(run_until);
        tool.tick(&ctx);
        assert!(!tool.live_check.is_running());
        release_run.send(()).unwrap();
        poll_until_done(|| {
            tool.tick(&ctx);
            tool.run_log.is_running()
        });
        assert!(tool.live_check.is_running());
        finish_check(&mut tool, &ctx);
        assert_eq!(
            row(tool.live_check.rows(), "/egg/").id,
            Some(IdCell::Known("792".to_owned()))
        );
    }

    #[test]
    fn a_refused_check_is_a_log_line_and_the_grid_keeps_its_rows() {
        let temp = sandbox("view_grid_refused");
        // An absolute exports folder that does not exist: the preflight refuses it.
        let missing = temp.path().join("missing");
        let setting = format!("exports_folder_path = '{}'\n", missing.display());
        let ctx = tool_context(temp.path(), &setting);
        let mut tool = Tool::new();
        let (release_check, check_until) = unbounded();
        tool.live_check = LiveCheck::in_flight(check_until, vec![label_only_row()]);
        release_check.send(()).unwrap();
        finish_check(&mut tool, &ctx);
        tool.live_check.start(&ctx);
        finish_check(&mut tool, &ctx);
        assert_eq!(tool.live_check.rows(), [label_only_row()]);
        let lines = tool.run_log.texts();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("error: "), "{lines:?}");
    }

    /// The log body's scroll offset: its `scroll_area::State` under the id the
    /// panel's `id_salt` chain builds — the root ui's top id, the open dock panel's
    /// resolved id (egui 0.37 salts the panel's child scope with it: the parent
    /// scope salted by the tool's own id), the panel frame's `child` content salt,
    /// the body frame's, then the dock id's `body` child salt (`studio_core`'s
    /// `widgets/log_panel.rs`).
    fn scroll_offset(ctx: &egui::Context) -> f32 {
        let top = egui::Id::unique((egui::ViewportId::ROOT, "__top_ui"));
        let id = top
            .with(egui::IdSalt::new(top.with(egui::Id::unique(TOOL_ID))))
            .with(egui::IdSalt::new("child"))
            .with(egui::IdSalt::new("child"))
            .with(egui::IdSalt::new(egui::Id::unique(TOOL_ID).with("body")));
        egui::scroll_area::State::load(ctx, id)
            .map(|state| state.clamped_offset().y)
            .expect("the log body's scroll area")
    }

    // TC-GUI-03
    #[test]
    fn a_log_line_arriving_keeps_the_scroll_area_at_the_bottom() {
        let ctx = settings_only_context();
        let mut log = RunLog::default();
        let lines: Vec<String> = (1..=40).map(|line| format!("log line {line}")).collect();
        log.append_test_lines(&lines.iter().map(String::as_str).collect::<Vec<_>>());
        let mut panel = LogPanel::new(TOOL_ID);
        let mut window = Headless::new();
        // The log overflows the panel's body: stuck to the end, the offset is off the top.
        window.frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        let before = scroll_offset(&window.ctx);
        assert!(before > 0.0, "stuck to the bottom of a log that overflows");
        // A line arrives: the offset grows with the content, the last line still shown.
        log.append_test_lines(&["log line 41"]);
        window.frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, &mut panel, &[], false)
        });
        let after = scroll_offset(&window.ctx);
        assert!(
            after > before,
            "the bottom follows new lines: {after} vs {before}"
        );
    }
}
