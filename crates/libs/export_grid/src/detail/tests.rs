use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;

use egui::epaint::Shape;
use studio_core::{Disposition, ExportId, message_line};
use vtree::ScopePath;

use super::*;
use crate::cells::Width;
use crate::grid::show;
use crate::rows::{IdCell, Line};
use crate::testing::{COLUMNS, Drawn, Headless, press, release};

/// More than egui's 0.3 s between the clicks of a double-click: two clicks this far apart
/// are two clicks.
const APART: f64 = 0.5;

/// A finding of the Compiler's `code` at `severity` about `scope`.
fn finding(
    code: &'static str,
    severity: Severity,
    scope: Scope,
    context: &[(&str, &str)],
) -> Message {
    Message {
        code: MessageCode {
            tool_id: "compiler",
            code: Cow::Borrowed(code),
        },
        severity,
        disposition: Disposition::Keep,
        scope,
        context: context
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    }
}

/// The folder `path` of the export.
fn folder(path: &str) -> Scope {
    Scope::Folder {
        export_id: ExportId(1),
        path: ScopePath::new(path).unwrap(),
    }
}

/// The Compiler's catalog rows of the scene's codes.
fn catalog() -> Vec<CatalogRow> {
    [
        (
            "fpc_conflict",
            "Error",
            "A player folder holds both `fpc_on` and `fpc_off`.",
            "The folder is left out. Keep one marker.",
        ),
        (
            "fmdl_weights_not_normalized",
            "Info",
            "In PES 2018 to 2021, a vertex's weights sum to neither 255 nor 0.",
            "Nothing to do.",
        ),
        (
            "shared_folder_orphaned",
            "Warning",
            "A shared `Faces/`, `Boots/` or `Gloves/` folder is linked by no player.",
            "The folder is ignored.",
        ),
    ]
    .map(|(code, severity, when, effect)| CatalogRow {
        code,
        severity,
        when,
        effect,
    })
    .to_vec()
}

/// A checked cell showing `text` for the folder `place` of the export, holding `messages`.
fn cell(text: &str, place: &str, messages: Vec<Message>) -> Cell {
    let outline = match messages.iter().map(|message| message.severity).max() {
        Some(Severity::Error | Severity::Fatal) => Some(Outline::Error),
        Some(Severity::Warning) => Some(Outline::Warning),
        Some(Severity::Info) | None => None,
    };
    Cell {
        outline,
        messages,
        place: place.to_owned(),
        open: Some(Open::Folder(
            PathBuf::from("exports/co Midcup Map").join(place),
        )),
        ..Cell::new(text, CellState::FullOk)
    }
}

/// /co/'s row: its label holds an orphaned shared folder's Warning; players 01 (clean)
/// and 03 (an Info, then an Error, as the check reported them); the portraits, which have
/// no folder to open; the goalkeeper kit and K6, which the window covers at its first
/// place. Its log lines are its three findings'.
fn co_row() -> Row {
    let orphaned = finding(
        "shared_folder_orphaned",
        Severity::Warning,
        folder("Boots/Solo"),
        &[],
    );
    let weights = finding(
        "fmdl_weights_not_normalized",
        Severity::Info,
        folder("Players/03 - B"),
        &[("file", "boots.fmdl"), ("count", "1662")],
    );
    let fpc = finding(
        "fpc_conflict",
        Severity::Error,
        folder("Players/03 - B"),
        &[],
    );
    let log_lines = [&orphaned, &weights, &fpc]
        .map(|message| message_line(Some("co Midcup Map"), message))
        .to_vec();
    let mut players = vec![Position::Empty; 23];
    players[0] = Position::Cell(cell("01", "Players/01 - A", Vec::new()));
    players[2] = Position::Cell(cell("03", "Players/03 - B", vec![weights, fpc]));
    let portraits = Cell {
        width: Width::Wide,
        open: None,
        ..cell("2/2", "Portraits in the player folders", Vec::new())
    };
    let mut kits = vec![Position::Empty; 10];
    kits[0] = Position::Cell(cell("GK", "Kits/g1", Vec::new()));
    kits[6] = Position::Cell(cell("K6", "Kits/p6", Vec::new()));
    Row {
        key: "co Midcup Map".to_owned(),
        label: Cell {
            open: Some(Open::Folder(PathBuf::from("exports/co Midcup Map"))),
            ..cell("/co/", "co Midcup Map", vec![orphaned])
        },
        id: Some(IdCell::Known("714".to_owned())),
        body: RowBody::Lines(vec![Line {
            name: None,
            groups: vec![players, vec![Position::Cell(portraits)], kits],
        }]),
        log_lines,
    }
}

/// /a/'s row: a clean label alone, no log lines.
fn a_row() -> Row {
    Row {
        key: "a Midcup Grid".to_owned(),
        label: cell("/a/", "a Midcup Grid", Vec::new()),
        id: Some(IdCell::Known("702".to_owned())),
        body: RowBody::Lines(Vec::new()),
        log_lines: Vec::new(),
    }
}

/// The grid of /co/ and /a/ in a headless window, its detail window's state kept from
/// frame to frame.
struct Scene {
    window: Headless,
    detail: Detail,
    rows: Vec<Row>,
    catalog: Vec<CatalogRow>,
}

impl Scene {
    fn new() -> Scene {
        Scene {
            window: Headless::new(),
            detail: Detail::default(),
            rows: vec![co_row(), a_row()],
            catalog: catalog(),
        }
    }

    /// One frame with `events`; what the grid returned and what it drew.
    fn frame(&mut self, events: Vec<egui::Event>) -> (GridResponse, Drawn) {
        let Scene {
            window,
            detail,
            rows,
            catalog,
        } = self;
        window.frame_with(events, |ui| {
            show(ui, "grid", &COLUMNS, rows, false, detail, catalog)
        })
    }

    /// A frame with no input.
    fn idle(&mut self) -> Drawn {
        self.frame(Vec::new()).1
    }

    /// The pointer moved to `pos`.
    fn hover(&mut self, pos: Pos2) -> Drawn {
        self.frame(vec![egui::Event::PointerMoved(pos)]).1
    }

    /// `seconds` pass before the next frame.
    fn wait(&mut self, seconds: f64) {
        self.window.time += seconds;
    }

    /// Two frames with no input, the second's drawing: the first window a session opens
    /// is laid out unseen on its first frame, since egui sizes a new area before showing
    /// it.
    fn settled(&mut self) -> Drawn {
        self.idle();
        self.idle()
    }

    /// A click at `pos`, apart from any click before it: what the release's frame
    /// returned, and what `settled` drew after it.
    fn click(&mut self, pos: Pos2) -> (GridResponse, Drawn) {
        self.wait(APART);
        self.frame(press(pos));
        let (response, _) = self.frame(release(pos));
        (response, self.settled())
    }

    /// A double-click at `pos`; what its second release's frame returned.
    fn double_click(&mut self, pos: Pos2) -> GridResponse {
        self.click(pos);
        self.frame(press(pos));
        self.frame(release(pos)).0
    }

    /// The key went down.
    fn key(&mut self, key: Key) -> Drawn {
        let event = egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![event]).1
    }
}

/// Where the scene's cells, and a spot of empty grid, stand on screen.
struct Spots {
    label: Pos2,
    one: Pos2,
    three: Pos2,
    portraits: Pos2,
    k6: Pos2,
    /// Position 02, a faint empty square.
    empty: Pos2,
}

fn spots(drawn: &Drawn) -> Spots {
    let one = drawn.one("01").center();
    let three = drawn.one("03").center();
    Spots {
        label: drawn.one("/co/").center(),
        one,
        three,
        portraits: drawn.one("2/2").center(),
        k6: drawn.one("K6").center(),
        empty: pos2((one.x + three.x) / 2.0, one.y),
    }
}

/// Whether the window is open, titled `title`.
fn shows(drawn: &Drawn, title: &str) -> bool {
    !drawn.all(title).is_empty()
}

/// Whether any detail window is open: a cell's, whose title is its team and more, or a
/// label's, whose place is its export's name (its title reads as the label does).
fn any_window(drawn: &Drawn) -> bool {
    drawn
        .nodes
        .iter()
        .filter_map(|node| node.value().or(node.label()))
        .any(|text| {
            text.starts_with("/co/ ")
                || text.starts_with("/a/ ")
                || text == "co Midcup Map"
                || text == "a Midcup Grid"
        })
}

/// The centre of the pin badge's disc, if one is painted.
fn badge(drawn: &Drawn) -> Option<Pos2> {
    drawn.shapes.iter().find_map(|shape| match shape {
        Shape::Circle(circle) if circle.radius == BADGE_RADIUS => Some(circle.center),
        _ => None,
    })
}

// TC-GUI-09
#[test]
fn a_resting_pointer_opens_the_cells_window_after_the_delay_and_it_follows_at_once() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    // A window opened and closed first: the first one a session opens is unseen on its
    // first frame, which would pass "not at once" whatever the delay.
    scene.hover(at.one);
    scene.wait(HOVER_DELAY);
    assert!(
        shows(&scene.settled(), "/co/ 01"),
        "premise: a window opened"
    );
    scene.hover(at.empty);
    scene.wait(TOOLTIP_GRACE);
    scene.idle();
    assert_eq!(scene.idle().repaint_delay, Duration::MAX, "premise: idle");

    assert!(!shows(&scene.hover(at.three), "/co/ 03"), "not at once");
    // egui paints the frame after an input at once: the delay's ask shows on the next.
    scene.idle();
    let drawn = scene.idle();
    assert!(
        drawn.repaint_delay > Duration::ZERO
            && drawn.repaint_delay <= Duration::from_secs_f64(HOVER_DELAY),
        "a repaint when the delay ends: {:?}",
        drawn.repaint_delay
    );
    // Three frames of a 60th of a second have passed since the hover: 0.1 s with this.
    scene.wait(0.05);
    assert!(!shows(&scene.idle(), "/co/ 03"), "not before the delay");
    scene.wait(0.05);
    let drawn = scene.idle();
    assert!(shows(&drawn, "/co/ 03"), "after the delay");
    assert_eq!(
        drawn.all("Players/03 - B").len(),
        3,
        "the place, and each finding's"
    );
    let error = drawn.one("fpc_conflict");
    let info = drawn.one("fmdl_weights_not_normalized");
    assert!(error.top() < info.top(), "the Error over the Info");
    drawn.one("file=boots.fmdl, count=1662");
    let when = drawn.one("A player folder holds both fpc_on and fpc_off.");
    drawn.one("The folder is left out. Keep one marker.");
    assert!(when.top() < info.top(), "a finding's texts under its code");
    assert!(drawn.all("No findings.").is_empty());

    // Straight onto 01: its window at once.
    let drawn = scene.hover(at.one);
    assert!(shows(&drawn, "/co/ 01") && !shows(&drawn, "/co/ 03"));
    drawn.one("No findings.");
    // Onto the empty grid: closed.
    assert!(!any_window(&scene.hover(at.empty)));
    // Back onto a cell soon after: at once, the window having just closed.
    assert!(shows(&scene.hover(at.three), "/co/ 03"));
    // Later: the delay again.
    scene.hover(at.empty);
    scene.wait(TOOLTIP_GRACE + 0.1);
    assert!(!shows(&scene.hover(at.three), "/co/ 03"));
}

// TC-GUI-09
#[test]
fn the_window_of_a_cell_alone_in_its_group_is_named_by_the_group() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.portraits);
    let drawn = scene.idle();
    assert!(shows(&drawn, "/co/ portraits"));
    drawn.one("Portraits in the player folders");
    drawn.one("Checked");
}

// TC-GUI-09
#[test]
fn a_cell_under_the_unpinned_window_can_be_hovered() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.hover(at.three);
    scene.wait(HOVER_DELAY);
    let drawn = scene.settled();
    let window = drawn.one("/co/ 03");
    assert!(
        window.left() < at.k6.x && window.top() < at.k6.y,
        "premise: the window covers K6"
    );
    assert!(shows(&scene.hover(at.k6), "/co/ K6"));
}

// TC-GUI-24
#[test]
fn a_click_pins_the_window_and_another_click_pins_another_cell() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    assert!(shows(&drawn, "/co/ 03"), "open with the click");
    let three = drawn.one("03");
    assert_eq!(
        badge(&drawn),
        Some(pos2(three.right() - 2.0, three.top() + 2.0))
    );
    drawn.one(PIN);

    scene.hover(at.one);
    scene.wait(APART);
    let drawn = scene.idle();
    assert!(shows(&drawn, "/co/ 03") && !shows(&drawn, "/co/ 01"));

    let (_, drawn) = scene.click(at.one);
    assert!(shows(&drawn, "/co/ 01") && !shows(&drawn, "/co/ 03"));
    let one = drawn.one("01");
    assert_eq!(
        badge(&drawn),
        Some(pos2(one.right() - 2.0, one.top() + 2.0))
    );
}

// TC-GUI-24
#[test]
fn a_label_s_pin_stands_inside_its_corner() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.label);
    let label = drawn.all("/co/")[0];
    assert_eq!(
        badge(&drawn),
        Some(pos2(label.right() - 8.0, label.top() + 8.0))
    );
}

// TC-GUI-24
#[test]
fn the_pinned_cell_the_window_s_pin_or_the_empty_grid_unpins() {
    for unpin in ["the cell", "the pin", "the empty grid"] {
        let mut scene = Scene::new();
        let at = spots(&scene.idle());
        let (_, drawn) = scene.click(at.three);
        let pin = drawn.one(PIN).center();
        let spot = match unpin {
            "the cell" => at.three,
            "the pin" => pin,
            _ => at.empty,
        };
        let (_, drawn) = scene.click(spot);
        assert!(!shows(&drawn, "/co/ 03"), "{unpin}: closed");
        assert_eq!(badge(&drawn), None, "{unpin}: no pin");
    }
}

// TC-GUI-24
#[test]
fn a_cell_a_click_unpinned_opens_nothing_until_the_pointer_has_left_it() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.three);
    scene.click(at.three);
    scene.wait(APART);
    assert!(!shows(&scene.idle(), "/co/ 03"), "the pointer rests on it");
    scene.hover(at.empty);
    scene.hover(at.three);
    scene.wait(HOVER_DELAY);
    assert!(shows(&scene.settled(), "/co/ 03"), "back after leaving it");
}

// TC-GUI-24
#[test]
fn esc_closes_the_window_pinned_or_not_as_an_unpinning_click_does() {
    for pinned in [false, true] {
        let mut scene = Scene::new();
        let at = spots(&scene.idle());
        if pinned {
            scene.click(at.three);
        } else {
            scene.hover(at.three);
            scene.wait(HOVER_DELAY);
        }
        assert!(
            shows(&scene.settled(), "/co/ 03"),
            "premise: open, pinned {pinned}"
        );
        assert!(
            !shows(&scene.key(Key::Escape), "/co/ 03"),
            "pinned {pinned}"
        );
        scene.wait(APART);
        assert!(!shows(&scene.idle(), "/co/ 03"), "muted, pinned {pinned}");
    }
}

// TC-GUI-24
#[test]
fn a_double_click_opens_the_cell_s_folder_and_leaves_the_pin_as_it_was() {
    let folder = Some(Open::Folder(PathBuf::from(
        "exports/co Midcup Map/Players/03 - B",
    )));
    // Nothing pinned before, or 01: the double-click's first click pins 03 either way.
    for pinned_before in [None, Some("/co/ 01")] {
        let mut scene = Scene::new();
        let at = spots(&scene.idle());
        if pinned_before.is_some() {
            scene.click(at.one);
        }
        assert_eq!(scene.double_click(at.three).open, folder);
        scene.wait(APART);
        let drawn = scene.idle();
        assert!(!shows(&drawn, "/co/ 03"), "{pinned_before:?}");
        if let Some(title) = pinned_before {
            assert!(shows(&drawn, title));
        }
    }
}

// TC-GUI-24
#[test]
fn a_cell_with_no_folder_opens_nothing_and_its_window_has_no_open_folder() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    assert_eq!(scene.double_click(at.portraits).open, None);
    let (_, drawn) = scene.click(at.portraits);
    assert!(shows(&drawn, "/co/ portraits"));
    assert!(drawn.all("Open folder").is_empty());
    let (_, drawn) = scene.click(at.three);
    let button = drawn.one("Open folder").center();
    let (response, _) = scene.click(button);
    let folder = PathBuf::from("exports/co Midcup Map/Players/03 - B");
    assert_eq!(response.open, Some(Open::Folder(folder)));
}

#[test]
fn a_finding_s_code_clicked_asks_for_the_help_at_it() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    let link = drawn.one("fpc_conflict").center();
    let (response, _) = scene.click(link);
    let code = response.help.expect("the help asked for");
    assert_eq!(
        (code.tool_id, code.code.as_ref()),
        ("compiler", "fpc_conflict")
    );
}

// TC-GUI-24
#[test]
fn the_pinned_window_s_title_bar_drags_it_and_it_reopens_where_it_was_left() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    let title = drawn.one("/co/ 03");
    let from = title.center();
    let to = from + vec2(-100.0, 50.0);
    scene.frame(press(from));
    scene.hover(from + vec2(-50.0, 25.0));
    scene.hover(to);
    scene.frame(release(to));
    let moved = scene.idle().one("/co/ 03");
    assert_eq!(moved.min, title.min + vec2(-100.0, 50.0));
    scene.click(at.three);
    let (_, drawn) = scene.click(at.one);
    assert_eq!(drawn.one("/co/ 01").min, moved.min);
}

// A pin names its row by key, so it survives the rows being made again, and goes with
// its cell.
#[test]
fn a_pin_survives_new_rows_and_goes_with_its_cell() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.three);
    scene.rows = vec![a_row(), co_row()];
    scene.hover(at.empty);
    let drawn = scene.idle();
    assert!(shows(&drawn, "/co/ 03"), "the same cell, a row lower");
    scene.rows = vec![a_row()];
    assert!(!any_window(&scene.idle()));
    scene.rows = vec![co_row(), a_row()];
    assert!(!any_window(&scene.idle()), "the pin was dropped");
}

/// The text the frame put on the clipboard, if any.
fn clipboard(drawn: &Drawn) -> Option<&str> {
    drawn.commands.iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text.as_str()),
        _ => None,
    })
}

// TC-GUI-25
#[test]
fn copy_messages_puts_the_export_s_log_lines_on_the_clipboard() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.label);
    let button = drawn.one("Copy messages").center();
    scene.wait(APART);
    scene.frame(press(button));
    let (_, drawn) = scene.frame(release(button));
    assert_eq!(
        clipboard(&drawn),
        Some(
            "- co Midcup Map: Warning shared_folder_orphaned at Boots/Solo\n\
             - co Midcup Map: Info fmdl_weights_not_normalized at Players/03 - B \
             (file=boots.fmdl, count=1662)\n\
             - co Midcup Map: Error fpc_conflict at Players/03 - B"
        )
    );
    scene.idle().one("Copied");
    // egui paints the frame after an input at once: the ask for "Copied" to end shows on
    // the next.
    let drawn = scene.idle();
    assert!(
        drawn.repaint_delay > Duration::ZERO
            && drawn.repaint_delay <= Duration::from_secs_f64(COPIED_FOR),
        "a repaint when Copied ends: {:?}",
        drawn.repaint_delay
    );
    scene.wait(COPIED_FOR);
    scene.idle().one("Copy messages");
}

// TC-GUI-25
#[test]
fn copy_messages_is_on_a_label_with_log_lines_alone() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    for (spot, title) in [(at.three, "/co/ 03"), (at.portraits, "/co/ portraits")] {
        let (_, drawn) = scene.click(spot);
        assert!(shows(&drawn, title));
        assert!(drawn.all("Copy messages").is_empty(), "{title}");
    }
    let a = scene.idle().one("/a/").center();
    let (_, drawn) = scene.click(a);
    drawn.one("a Midcup Grid");
    assert!(drawn.all("Copy messages").is_empty(), "/a/");
}
