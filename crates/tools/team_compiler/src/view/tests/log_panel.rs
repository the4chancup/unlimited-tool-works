//! The log panel: a line arriving while it is scrolled to the bottom keeps the
//! last line in view; the check's findings come first, each line acting for its cell.

use crate::testing::ScratchFolder;

use super::*;

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
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    // The log overflows the panel's body: stuck to the end, the offset is off the top.
    window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let before = scroll_offset(&window.ctx);
    assert!(before > 0.0, "stuck to the bottom of a log that overflows");
    // A line arrives: the offset grows with the content, the last line still shown.
    log.append_test_lines(&["log line 41"]);
    window.frame(Vec::new(), |ui| {
        show(ui, &ctx, &log, &mut panel, &[], false, &mut detail)
    });
    let after = scroll_offset(&window.ctx);
    assert!(
        after > before,
        "the bottom follows new lines: {after} vs {before}"
    );
}

/// /co/'s export with a disallowed file in player 03's folder and an orphaned shared boots
/// folder, the sandbox's /egg/, and a referee export whose folder backing slots 01 and 35
/// holds a disallowed file.
fn check_sandbox(name: &str) -> ScratchFolder {
    let temp = sandbox(name);
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_file(root, "co Midcup Map/Players/03 - B/notes.exe", b"");
    add_file(root, "co Midcup Map/Boots/Solo/boots.fmdl", b"");
    add_file(root, "refs Cup/players.txt", b"01 A\n02 B\n35 A\n");
    add_player(root, "refs Cup", "A");
    add_player(root, "refs Cup", "B");
    add_file(root, "refs Cup/Players/A/notes.exe", b"");
    temp
}

#[test]
fn the_log_holds_the_check_s_findings_ahead_of_a_compile_s_lines() {
    let temp = check_sandbox("view_log_check_lines");
    let root = temp.path();
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    tool.live_check.start(&ctx);
    finish_check(&mut tool, &ctx);
    // Row by row, each row's label, then its cells left to right; the referee folder's
    // finding once, for slot 01.
    let weights = |at: &str, file: &str, count: u16| {
        format!("Info fmdl_weights_not_normalized at {at} (file={file}, count={count})")
    };
    let egg = "- egg Midcup Tracer: ";
    let player = "Players/05 - The Chad Stormworks Player";
    let check = [
        "- co Midcup Map: Warning shared_folder_orphaned at Boots/Solo".to_owned(),
        "- co Midcup Map: Info export_identified (team=/co/, id=714)".to_owned(),
        "- co Midcup Map: Error file_type_disallowed at Players/03 - B (file=notes.exe)".to_owned(),
        format!("{egg}Info export_identified (team=/egg/, id=792)"),
        format!("{egg}{}", weights(player, "boots.fmdl", 1662)),
        format!("{egg}{}", weights(player, "fcl_hair.fmdl", 1662)),
        format!("{egg}{}", weights(player, "glove_l.fmdl", 2)),
        "- refs Cup: Info export_identified (team=referees)".to_owned(),
        "- refs Cup: Warning file_not_used (file=Players/B/portrait.dds)".to_owned(),
        "- refs Cup: Error file_type_disallowed at Players/A (file=notes.exe)".to_owned(),
        format!("- refs Cup: {}", weights("Players/B", "boots.fmdl", 1662)),
        format!(
            "- refs Cup: {}",
            weights("Players/B", "fcl_hair.fmdl", 1662)
        ),
        format!("- refs Cup: {}", weights("Players/B", "glove_l.fmdl", 2)),
    ];
    assert_eq!(tool.run_log.texts(), check);

    tool.run_log.start_compile(&ctx, Vec::new);
    assert_eq!(tool.run_log.texts(), check, "a Compile keeps them");
    poll_until_done(|| {
        tool.tick(&ctx);
        tool.run_log.is_running()
    });
    let texts = tool.run_log.texts();
    assert!(texts.len() > check.len(), "premise: the run logged lines");
    assert_eq!(texts[..check.len()], check, "the check's lines first");
    let run: Vec<String> = texts[check.len()..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();

    // A later check's lines replace them, ahead of the run's.
    finish_check(&mut tool, &ctx);
    add_file(root, "co Midcup Map/Players/03 - B/setup.exe", b"");
    tool.live_check.start(&ctx);
    finish_check(&mut tool, &ctx);
    let mut later = check.to_vec();
    later.insert(
        3,
        "- co Midcup Map: Error file_type_disallowed at Players/03 - B (file=setup.exe)".to_owned(),
    );
    later.extend(run);
    assert_eq!(tool.run_log.texts(), later);
}

/// The view over a check's `rows`, its log holding their lines, drawn a frame at a time.
struct Scene {
    window: Headless,
    ctx: ToolContext,
    log: RunLog,
    panel: LogPanel,
    detail: GridDetail,
    rows: Vec<Row>,
}

impl Scene {
    fn new(rows: Vec<Row>) -> Scene {
        let mut log = RunLog::default();
        log.set_check(&rows);
        Scene {
            window: Headless::new(),
            ctx: settings_only_context(),
            log,
            panel: LogPanel::new(TOOL_ID),
            detail: GridDetail::new(),
            rows,
        }
    }

    /// One frame with `events`; its nodes.
    fn frame(&mut self, events: Vec<egui::Event>) -> Vec<accesskit::Node> {
        let Scene {
            window,
            ctx,
            log,
            panel,
            detail,
            rows,
        } = self;
        window
            .frame(events, |ui| show(ui, ctx, log, panel, rows, false, detail))
            .1
    }

    /// Idle frames past the detail window's hover delay, 0.15 s: twelve of a 60th of a
    /// second, egui's default frame time. The last one's nodes.
    fn rest(&mut self) -> Vec<accesskit::Node> {
        for _ in 0..11 {
            self.frame(Vec::new());
        }
        self.frame(Vec::new())
    }

    /// A click at `at`; the nodes of the frame after the release.
    fn click(&mut self, at: egui::Pos2) -> Vec<accesskit::Node> {
        self.frame(press(at));
        self.frame(release(at));
        self.frame(Vec::new())
    }

    /// The rectangles the last frame stroked in the selection colour, 2 points wide: the
    /// outline a line's click draws.
    fn outlines(&self) -> Vec<egui::Rect> {
        let colour = self
            .window
            .ctx
            .global_style()
            .visuals
            .selection
            .stroke
            .color;
        self.window
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect)
                    if rect.stroke.color == colour && rect.stroke.width == 2.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .collect()
    }
}

/// Where a click lands on the log line whose text after its code starts with `rest`.
fn on_line(nodes: &[accesskit::Node], rest: &str) -> egui::Pos2 {
    centre(
        nodes
            .iter()
            .find(|node| node.value().is_some_and(|text| text.starts_with(rest)))
            .unwrap_or_else(|| panic!("no log line goes on {rest:?}")),
    )
}

/// The rectangle of the painted grid cell reading `text`.
fn cell_rect(nodes: &[accesskit::Node], text: &str) -> egui::Rect {
    let found: Vec<egui::Rect> = nodes
        .iter()
        .filter(|node| node.role() == accesskit::Role::Label && node.value() == Some(text))
        .filter_map(accesskit::Node::bounds)
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .collect();
    assert_eq!(found.len(), 1, "one cell reads {text:?}");
    found[0]
}

// TC-GUI-15
#[test]
fn a_warning_line_opens_its_cell_s_window_on_hover_and_a_click_outlines_and_pins_it() {
    let temp = sandbox("view_log_line_cells");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_kit(root, "co Midcup Map", "p1 - Home");
    add_file(root, "co Midcup Map/Kits/p1 - Home/icon_99", b"");
    let mut scene = Scene::new(checked_rows(root));
    scene.frame(Vec::new());
    let nodes = scene.frame(Vec::new());
    assert!(!shows_text(&nodes, "/co/ K1"), "premise: no window open");
    let kit_line = on_line(&nodes, " at Kits/p1 - Home");

    // The pointer rests on the line, away from the grid.
    let nodes = scene.frame(vec![egui::Event::PointerMoved(kit_line)]);
    assert!(!shows_text(&nodes, "/co/ K1"), "not before the delay");
    assert!(shows_text(&scene.rest(), "/co/ K1"), "open on its kit");
    assert!(scene.outlines().is_empty(), "no outline from a hover");

    // A click outlines the kit and pins its window.
    let nodes = scene.click(kit_line);
    assert!(shows_text(&nodes, "/co/ K1"));
    button(&nodes, "📌");
    assert_eq!(scene.outlines(), [cell_rect(&nodes, "K1")]);

    // A second click closes the window, and the outline stays.
    let nodes = scene.click(kit_line);
    assert!(!shows_text(&nodes, "/co/ K1"), "closed");
    assert_eq!(scene.outlines(), [cell_rect(&nodes, "K1")]);

    // A line about the export as a whole opens the team label's window.
    let identified = on_line(&nodes, " (team=/co/, id=714)");
    scene.frame(vec![egui::Event::PointerMoved(identified)]);
    assert!(
        shows_text(&scene.rest(), "co Midcup Map"),
        "the label's window"
    );

    // That line is an Info, which takes no click: the outline stays on the kit.
    let nodes = scene.click(identified);
    assert_eq!(scene.outlines(), [cell_rect(&nodes, "K1")]);
    assert!(
        nodes.iter().all(|node| node.label() != Some("📌")),
        "no pin"
    );
}

// TC-GUI-15
#[test]
fn an_error_line_s_click_outlines_its_cell_and_pins_its_window() {
    let temp = sandbox("view_log_error_line");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_file(root, "co Midcup Map/Players/03 - B/notes.exe", b"");
    let mut scene = Scene::new(checked_rows(root));
    scene.frame(Vec::new());
    let nodes = scene.frame(Vec::new());
    assert!(!shows_text(&nodes, "/co/ 03"), "premise: no window open");
    let error_line = on_line(&nodes, " at Players/03 - B");

    let nodes = scene.click(error_line);
    assert!(shows_text(&nodes, "/co/ 03"), "the player's window");
    button(&nodes, "📌");
    assert_eq!(scene.outlines(), [cell_rect(&nodes, "03")]);
}
