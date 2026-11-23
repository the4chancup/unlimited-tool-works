//! The toolbar: the strip summing up the grid, its counts showing only their lines in the
//! log, and the log starting collapsed when, open, it would hide some of the grid's rows.

use export_grid::Line;
use studio_core::{LogFilter, Message, Severity};
use teams_list::TeamsList;

use crate::testing::ScratchFolder;

use super::*;
use crate::view::toolbar::{Part, Summary, strip, summary};

/// The first `count` teams of the bundled teams list, each a clean row holding only the
/// portraits cell every team export draws.
fn listed_rows(count: usize) -> Vec<Row> {
    TeamsList::UPSTREAM
        .lines()
        .skip(1)
        .take(count)
        .map(|line| {
            let (id, name) = line
                .split_once('\t')
                .expect("a teams list line is ID, tab, name");
            let portraits = Cell {
                width: Width::Wide,
                ..Cell::new("0/1", CellState::FullOk)
            };
            Row {
                key: format!("{name} Midcup"),
                label: Cell::new(name, CellState::FullOk),
                id: Some(IdCell::Known(id.to_owned())),
                body: RowBody::Lines(vec![Line {
                    name: None,
                    groups: vec![
                        vec![Position::Empty; 23],
                        vec![Position::Cell(Box::new(portraits))],
                        vec![Position::Empty; 10],
                    ],
                }]),
                log_lines: Vec::new(),
            }
        })
        .collect()
}

/// `row` with one line of the Compiler's columns, whose player 01 alone has a cell, holding
/// `messages`.
fn with_player_findings(mut row: Row, messages: Vec<Message>) -> Row {
    let player = Cell {
        messages,
        ..Cell::new("01", CellState::FullOk)
    };
    let mut players = vec![Position::Cell(Box::new(player))];
    players.extend(vec![Position::Empty; 22]);
    row.body = RowBody::Lines(vec![Line {
        name: None,
        groups: vec![players, vec![Position::Empty], vec![Position::Empty; 10]],
    }]);
    row
}

/// `row` as the check leaves an export it drops whole: its player 01 blocked, and the cause, an
/// Error, on its label.
fn dropped(row: Row) -> Row {
    let mut row = with_player_findings(row, Vec::new());
    let RowBody::Lines(lines) = &mut row.body else {
        unreachable!("with_player_findings gives the row lines")
    };
    let Position::Cell(cell) = &mut lines[0].groups[0][0] else {
        unreachable!("player 01 has a cell")
    };
    cell.state = CellState::Blocked;
    row.label.messages = vec![finding("team_name_unknown", Severity::Error)];
    row
}

#[test]
fn the_summary_counts_rows_by_the_findings_of_their_labels_and_cells() {
    let mut rows = listed_rows(7);
    // /3/: an error on its label. /a/: a warning on a player. /aco/: an error on its label and
    // a warning on a player, counted in both. /adv/: a message in place of its cells, its
    // finding on the label, so skipped, and not with errors. /an/: only an Info. The sixth:
    // one player cell holding an error and a warning, which shows the error alone, so it
    // counts only with errors. The seventh: dropped whole, its one cell blocked, so skipped.
    // The last: dropped before its files were listed, its label alone drawn, so skipped too.
    rows[0].label.messages = vec![finding("player_folder_number_invalid", Severity::Error)];
    rows[1] = with_player_findings(
        rows[1].clone(),
        vec![finding("player_unlisted", Severity::Warning)],
    );
    rows[2] = with_player_findings(
        rows[2].clone(),
        vec![finding("player_unlisted", Severity::Warning)],
    );
    rows[2].label.messages = vec![finding("player_folder_number_invalid", Severity::Error)];
    rows[3].body = RowBody::Message {
        text: "Old export format".to_owned(),
        tone: Tone::Error,
    };
    rows[3].label.messages = vec![finding("export_layout_old", Severity::Error)];
    rows[4].label.messages = vec![finding("export_identified", Severity::Info)];
    rows[5] = with_player_findings(
        rows[5].clone(),
        vec![
            finding("fpc_conflict", Severity::Error),
            finding("player_unlisted", Severity::Warning),
        ],
    );
    rows[6] = dropped(rows[6].clone());
    rows.push(Row {
        key: "v Midcup Map".to_owned(),
        label: Cell {
            messages: vec![finding("team_name_unknown", Severity::Error)],
            ..Cell::new("/v/", CellState::FullOk)
        },
        id: Some(IdCell::Missing),
        body: RowBody::Lines(Vec::new()),
        log_lines: Vec::new(),
    });
    assert_eq!(
        summary(&rows),
        Summary {
            exports: 8,
            skipped: 3,
            with_errors: 3,
            with_warnings: 2,
        }
    );
}

#[test]
fn the_strip_leaves_out_a_count_of_zero() {
    let texts = |exports, skipped, with_errors, with_warnings| -> Vec<String> {
        strip(&Summary {
            exports,
            skipped,
            with_errors,
            with_warnings,
        })
        .into_iter()
        .map(|part| part.text)
        .collect()
    };
    assert_eq!(texts(1, 0, 0, 0), ["1 export"]);
    assert_eq!(texts(14, 0, 0, 1), ["14 exports", " · ", "1 with warnings"]);
    assert_eq!(
        texts(14, 6, 3, 1),
        [
            "14 exports",
            " · ",
            "6 skipped",
            " · ",
            "3 with errors",
            " · ",
            "1 with warnings"
        ]
    );
    assert_eq!(texts(0, 0, 0, 0), Vec::<String>::new());
    let parts = strip(&Summary {
        exports: 2,
        skipped: 0,
        with_errors: 1,
        with_warnings: 0,
    });
    assert_eq!(
        parts.last(),
        Some(&Part {
            text: "1 with errors".to_owned(),
            link: Some((LogFilter::Errors, "Show only the errors in the log")),
        })
    );
}

/// The strip's parts as the member reads them: the labels left of the Compile button on its
/// row, left to right. A label's text run is a node of its own, with the same text: it is
/// skipped.
fn strip_parts(nodes: &[accesskit::Node]) -> Vec<&str> {
    let compile = button(nodes, "Compile")
        .bounds()
        .expect("a widget has bounds");
    let mut parts: Vec<(f64, &str)> = nodes
        .iter()
        .filter(|node| node.role() == accesskit::Role::Label)
        .filter_map(|node| {
            let bounds = node.bounds()?;
            let text = node.value()?;
            let middle = (bounds.y0 + bounds.y1) / 2.0;
            (bounds.x1 <= compile.x0 && compile.y0 < middle && middle < compile.y1)
                .then_some((bounds.x0, text))
        })
        .collect();
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    parts.into_iter().map(|(_, text)| text).collect()
}

#[test]
fn a_strip_too_long_for_its_room_is_cut_short_with_an_ellipsis() {
    let mut rows = listed_rows(4);
    for row in &mut rows[..2] {
        row.label.messages = vec![finding("player_folder_number_invalid", Severity::Error)];
    }
    rows[2] = with_player_findings(
        rows[2].clone(),
        vec![finding("player_unlisted", Severity::Warning)],
    );
    let ctx = settings_only_context();
    let log = RunLog::default();
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    // A view 290 points wide leaves the strip room for `4 exports · ` and part of the
    // count after it.
    let narrow = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(290.0, 780.0));
    let (_, nodes) = window.frame(Vec::new(), |ui| {
        ui.scope_builder(egui::UiBuilder::new().max_rect(narrow), |ui| {
            show(ui, &ctx, &log, &mut panel, &rows, false, &mut detail)
        })
    });
    assert_eq!(strip_parts(&nodes), ["4 exports", " · ", "2 with errors"]);
    let cut = window
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == "2 with errors" => {
                Some(shape.galley.elided)
            }
            _ => None,
        });
    assert_eq!(cut, Some(true), "the errors count ends in an ellipsis");
}

/// The colour the last frame painted the text run `text` in.
fn colour_of(window: &Headless, text: &str) -> egui::Color32 {
    window
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                let colour = shape.galley.job.sections[0].format.color;
                Some(if colour == egui::Color32::PLACEHOLDER {
                    shape.fallback_color
                } else {
                    colour
                })
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text run of {text:?}"))
}

/// The label reading `text`.
fn reading<'a>(nodes: &'a [accesskit::Node], text: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == accesskit::Role::Label && node.value() == Some(text))
        .unwrap_or_else(|| panic!("no label reads {text:?}"))
}

/// The log's open chevron, on the row of its `Log` title: the legend has a `▾` of its own.
fn log_chevron(nodes: &[accesskit::Node]) -> &accesskit::Node {
    let title = reading(nodes, "Log").bounds().expect("a label has bounds");
    nodes
        .iter()
        .find(|node| {
            node.label() == Some("▾")
                && node.bounds().is_some_and(|bounds| {
                    let middle = (bounds.y0 + bounds.y1) / 2.0;
                    title.y0 < middle && middle < title.y1
                })
        })
        .expect("the open log's chevron")
}

/// Exports with errors on two of them and a warning on a third: /co/'s player folder turns
/// FPC both on and off, /a/'s is at no slot, and /3/'s players.txt leaves a folder unlisted.
/// /co/'s second export's name says neither `Full` nor `Midcup`, so it is skipped. The
/// sandbox's /egg/ checks clean.
fn strip_sandbox(name: &str) -> ScratchFolder {
    let temp = sandbox(name);
    let root = temp.path();
    fs::write(
        root.join("data/teams_list.txt"),
        "ID\tName\n701\t/3/\n702\t/a/\n714\t/co/\n792\t/egg/\n",
    )
    .unwrap();
    add_player(root, "co Midcup Map", "01 - A");
    add_player(root, "co Midcup Map", "03 - B");
    add_file(root, "co Midcup Map/Players/03 - B/fpc_on", b"");
    add_file(root, "co Midcup Map/Players/03 - B/fpc_off", b"");
    add_player(root, "a Midcup Map", "03 - B");
    add_player(root, "a Midcup Map", "15_Snuffy");
    add_player(root, "3 Midcup Roster", "Snuffy");
    add_player(root, "3 Midcup Roster", "15 - B");
    add_file(root, "3 Midcup Roster/players.txt", b"03 snuffy\n");
    add_player(root, "co Spring 2026", "03 - B");
    for export in ["co Midcup Map", "a Midcup Map", "3 Midcup Roster"] {
        complete(&root.join("exports").join(export), PesVersion::Pes21, &[]);
    }
    temp
}

// TC-GUI-26
#[test]
fn the_strip_counts_the_rows_with_errors_and_warnings_and_its_counts_filter_the_log() {
    let temp = strip_sandbox("view_toolbar_strip");
    let rows = checked_rows(temp.path());
    let ctx = settings_only_context();
    let mut log = RunLog::default();
    log.append_test_lines(&["- egg Midcup Tracer: Info export_identified (team=/egg/, id=792)"]);
    log.append_test_line(
        Severity::Warning,
        "- 3 Midcup Roster: Warning player_unlisted at Players/15 - B",
    );
    log.append_test_line(
        Severity::Error,
        "- co Midcup Map: Error fpc_conflict at Players/03 - B",
    );
    let texts: Vec<String> = log.texts().into_iter().map(str::to_owned).collect();
    // Which of the info, the warning and the error line the log draws.
    let log_shows = |nodes: &[accesskit::Node]| -> Vec<bool> {
        texts.iter().map(|line| shows_text(nodes, line)).collect()
    };
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    let mut frame = |window: &mut Headless, events| {
        window
            .frame(events, |ui| {
                show(ui, &ctx, &log, &mut panel, &rows, false, &mut detail)
            })
            .1
    };

    let nodes = frame(&mut window, Vec::new());
    assert_eq!(
        strip_parts(&nodes).concat(),
        "5 exports · 1 skipped · 2 with errors · 1 with warnings"
    );
    // The log is open on All: the counts are in the strip's own colour.
    assert_eq!(log_shows(&nodes), [true, true, true]);
    let strip_colour = colour_of(&window, "5 exports");
    for count in ["2 with errors", "1 with warnings"] {
        assert_eq!(
            colour_of(&window, count),
            strip_colour,
            "{count}, the log open"
        );
    }

    // A click on the errors leaves the log open, on its Error line alone.
    let nodes = click_on(&mut window, reading(&nodes, "2 with errors"), &mut frame);
    assert_eq!(log_shows(&nodes), [false, false, true]);

    // Collapsed, the counts are in the link blue, and a click on the warnings opens the log
    // on its Warning and Error lines.
    let nodes = click_on(&mut window, log_chevron(&nodes), &mut frame);
    assert_eq!(
        log_shows(&nodes),
        [false, false, false],
        "premise: collapsed"
    );
    let link = window.ctx.global_style().visuals.hyperlink_color;
    assert_ne!(link, strip_colour, "premise: the two colours differ");
    for count in ["2 with errors", "1 with warnings"] {
        assert_eq!(
            colour_of(&window, count),
            link,
            "{count}, the log collapsed"
        );
    }
    let nodes = click_on(&mut window, reading(&nodes, "1 with warnings"), &mut frame);
    assert_eq!(log_shows(&nodes), [false, true, true]);
}

/// The heads of the log lines drawn, `- <export>: <Severity> `, the text before a code.
fn line_heads(nodes: &[accesskit::Node]) -> Vec<&str> {
    nodes
        .iter()
        .filter_map(accesskit::Node::value)
        .filter(|text| text.starts_with("- ") && text.ends_with(' '))
        .collect()
}

// The strip's count filters the check's own lines, which the log opens with.
// TC-GUI-26
#[test]
fn the_errors_count_shows_the_check_s_error_lines() {
    let temp = strip_sandbox("view_toolbar_check_lines");
    let ctx = tool_context(temp.path(), "");
    let mut tool = Tool::new();
    tool.live_check.start(&ctx);
    finish_check(&mut tool, &ctx);
    let mut window = Headless::new();
    let mut frame =
        |window: &mut Headless, events| window.frame(events, |ui| tool.view(ui, &ctx)).1;
    let nodes = frame(&mut window, Vec::new());
    assert!(
        line_heads(&nodes).contains(&"- 3 Midcup Roster: Warning "),
        "premise: the log shows All, {:?}",
        line_heads(&nodes)
    );
    let nodes = click_on(&mut window, reading(&nodes, "2 with errors"), &mut frame);
    let mut heads = line_heads(&nodes);
    heads.sort_unstable();
    heads.dedup();
    assert_eq!(
        heads,
        [
            "- a Midcup Map: Error ",
            "- co Midcup Map: Error ",
            "- co Spring 2026: Error "
        ]
    );
}

/// Frames of the view over `rows` with an empty log, `frames` of them.
fn draw_frames(
    window: &mut Headless,
    panel: &mut LogPanel,
    detail: &mut GridDetail,
    rows: &[Row],
    frames: usize,
) {
    let ctx = settings_only_context();
    let log = RunLog::default();
    for _ in 0..frames {
        window.frame(Vec::new(), |ui| {
            show(ui, &ctx, &log, panel, rows, false, detail)
        });
    }
}

// TC-GUI-27
#[test]
fn the_log_starts_collapsed_when_open_it_would_hide_rows() {
    // At the window's 1280 by 780, the open log leaves the grid room for about 15 rows.
    for (count, open) in [(30, false), (2, true)] {
        let rows = listed_rows(count);
        let mut panel = LogPanel::new(TOOL_ID);
        let mut detail = GridDetail::new();
        let mut window = Headless::new();
        // The check has not returned yet: the first frames have no rows.
        draw_frames(&mut window, &mut panel, &mut detail, &[], 2);
        assert!(panel.is_expanded(), "premise: open before the rows");
        // The first frame with rows is drawn with the log still open, the collapse
        // decided after the log and the toolbar were drawn, so that frame asks for the
        // next one. The grid's scroll bar appearing asks on the same frame, so the
        // collapse's own ask is told apart by its cause.
        draw_frames(&mut window, &mut panel, &mut detail, &rows, 1);
        let asked = if open { Duration::MAX } else { Duration::ZERO };
        assert_eq!(window.repaint_delay, asked, "{count} rows, the first frame");
        // The frame after, whose own draw makes the first one's causes readable.
        draw_frames(&mut window, &mut panel, &mut detail, &rows, 1);
        assert_eq!(panel.is_expanded(), open, "{count} rows");
        let causes: Vec<String> = window
            .ctx
            .repaint_causes()
            .iter()
            .map(|cause| cause.file.replace('\\', "/"))
            .collect();
        assert_eq!(
            causes.iter().any(|file| file.ends_with("view/mod.rs")),
            !open,
            "{count} rows, the first frame's ask is the collapse's: {causes:?}"
        );
    }
}

// TC-GUI-27
#[test]
fn a_later_check_adding_rows_leaves_the_log_open() {
    let mut panel = LogPanel::new(TOOL_ID);
    let mut detail = GridDetail::new();
    let mut window = Headless::new();
    draw_frames(&mut window, &mut panel, &mut detail, &listed_rows(2), 2);
    assert!(panel.is_expanded(), "premise: two rows leave the log open");
    // Thirty rows collapse the log at the start (the test above); here they come later.
    draw_frames(&mut window, &mut panel, &mut detail, &listed_rows(30), 2);
    assert!(panel.is_expanded());
}
