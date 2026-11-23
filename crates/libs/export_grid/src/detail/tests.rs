use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use egui::epaint::Shape;
use studio_core::{Disposition, ExportId, message_line};
use vtree::ScopePath;

use super::*;
use crate::cells::Width;
use crate::grid::show;
use crate::rows::{IdCell, Line};
use crate::testing::{COLUMNS, Drawn, Headless, dark, press, release};

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

/// The scene's pictures and their sizes in texels: /co/'s logo (40 × 40, smaller than the
/// box), its goalkeeper kit's texture (400 × 200, 2:1, larger than the box), player 01's
/// portrait in his folder and the `Portraits/` files of every slot but 20 (2:3, a
/// portrait's shape).
fn pictures() -> Vec<(String, [usize; 2])> {
    let mut pictures = vec![
        (LOGO.to_owned(), [40, 40]),
        (KIT.to_owned(), [400, 200]),
        (PORTRAIT_01.to_owned(), [2, 3]),
    ];
    pictures.extend(
        (1..=23)
            .filter(|slot| *slot != 20)
            .map(|slot| (portraits_file(slot), [2, 3])),
    );
    pictures
}

const LOGO: &str = "logo.png";
const KIT: &str = "Kits/g1/kit.dds";
const PORTRAIT_01: &str = "Players/01 - A/portrait.dds";

/// The `Portraits/` file of player `slot`.
fn portraits_file(slot: u8) -> String {
    format!("Portraits/{slot:02}.dds")
}

/// The picture `path` of /co/'s export.
fn picture(path: &str) -> PictureFile {
    PictureFile {
        export: PathBuf::from("exports/co Midcup Map"),
        path: path.to_owned(),
    }
}

/// /co/'s row: its label holds an orphaned shared folder's Warning; players 01 (clean)
/// and 03 (an Info, then an Error, as the check reported them); the portraits, which have
/// no folder to open; the goalkeeper kit and K6, which the window covers at its first
/// place. Its log lines are its three findings'. Its boxes show its logo, 01's portrait
/// (03 has none), the kit's texture, and every portrait but 20's; the portraits cell is
/// drawn without the logo, so its last tile reads "No logo".
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
    let player = |portrait: Option<&str>| TopBox::Player {
        portrait: portrait.map(picture),
        logo: Some(picture(LOGO)),
    };
    let mut players = vec![Position::Empty; 23];
    players[0] = Position::Cell(Box::new(Cell {
        top: player(Some(PORTRAIT_01)),
        ..cell("01", "Players/01 - A", Vec::new())
    }));
    players[2] = Position::Cell(Box::new(Cell {
        top: player(None),
        ..cell("03", "Players/03 - B", vec![weights, fpc])
    }));
    let portraits = Cell {
        width: Width::Wide,
        open: None,
        top: TopBox::Portraits {
            portraits: (1..=23)
                .map(|slot| (slot != 20).then(|| picture(&portraits_file(slot))))
                .collect(),
            logo: None,
        },
        ..cell("2/2", "Portraits in the player folders", Vec::new())
    };
    let mut kits = vec![Position::Empty; 10];
    kits[0] = Position::Cell(Box::new(Cell {
        top: TopBox::Picture(picture(KIT)),
        ..cell("GK", "Kits/g1", Vec::new())
    }));
    kits[6] = Position::Cell(Box::new(cell("K6", "Kits/p6", Vec::new())));
    Row {
        key: "co Midcup Map".to_owned(),
        label: Cell {
            open: Some(Open::Folder(PathBuf::from("exports/co Midcup Map"))),
            top: TopBox::Logo(Some(picture(LOGO))),
            ..cell("/co/", "co Midcup Map", vec![orphaned])
        },
        id: Some(IdCell::Known("714".to_owned())),
        body: RowBody::Lines(vec![Line {
            name: None,
            groups: vec![players, vec![Position::Cell(Box::new(portraits))], kits],
        }]),
        log_lines,
    }
}

/// /a/'s row: a clean label alone, no log lines, no logo.
fn a_row() -> Row {
    Row {
        key: "a Midcup Grid".to_owned(),
        label: Cell {
            top: TopBox::Logo(None),
            ..cell("/a/", "a Midcup Grid", Vec::new())
        },
        id: Some(IdCell::Known("702".to_owned())),
        body: RowBody::Lines(Vec::new()),
        log_lines: Vec::new(),
    }
}

/// /co/'s row with one cell, player 07's, whose Open folder selects the archive
/// `co Midcup Map.zip` in the exports folder: an export packed in an archive has
/// no folder of its own on disk.
fn archive_row() -> Row {
    let mut players = vec![Position::Empty; 23];
    players[6] = Position::Cell(Box::new(Cell {
        open: Some(Open::Select(PathBuf::from("exports/co Midcup Map.zip"))),
        ..cell("07", "co Midcup Map.zip", Vec::new())
    }));
    Row {
        key: "co Midcup Map".to_owned(),
        label: Cell {
            top: TopBox::Logo(None),
            ..cell("/co/", "co Midcup Map", Vec::new())
        },
        body: RowBody::Lines(vec![Line {
            name: None,
            groups: vec![players, vec![Position::Empty], vec![Position::Empty; 10]],
        }]),
        ..a_row()
    }
}

/// The grid of /co/ and /a/ in a headless window, its detail window's state kept from
/// frame to frame.
struct Scene {
    window: Headless,
    detail: Detail,
    rows: Vec<Row>,
    catalog: Vec<CatalogRow>,
    /// The textures the window's pictures get, uploaded to the window's context.
    textures: HashMap<PictureFile, egui::TextureHandle>,
    /// Every picture the window asked for, in the order it asked.
    asked: Vec<PictureFile>,
    /// The cell the tool's log line under the pointer stands for, handed to `point_at`
    /// before each frame.
    pointing: Option<CellKey>,
    /// A modal drawn over the grid (a tool's dialog). `modal_closed` holds whether its
    /// `should_close` has fired.
    modal: bool,
    modal_closed: bool,
    /// An `egui::Window` drawn over `rect` every frame (the Help window over the detail
    /// window). `help_clicked` holds whether its button has taken a click.
    help: Option<Rect>,
    help_clicked: bool,
}

impl Scene {
    fn new() -> Scene {
        let window = Headless::new();
        let textures = pictures()
            .into_iter()
            .map(|(path, size)| {
                let image = egui::ColorImage::filled(size, egui::Color32::WHITE);
                let texture = window
                    .ctx
                    .load_texture(&path, image, egui::TextureOptions::LINEAR);
                (picture(&path), texture)
            })
            .collect();
        Scene {
            window,
            detail: Detail::default(),
            rows: vec![co_row(), a_row()],
            catalog: catalog(),
            textures,
            asked: Vec::new(),
            pointing: None,
            modal: false,
            modal_closed: false,
            help: None,
            help_clicked: false,
        }
    }

    /// One frame with `events`; what the grid returned and what it drew.
    fn frame(&mut self, events: Vec<egui::Event>) -> (GridResponse, Drawn) {
        let Scene {
            window,
            detail,
            rows,
            catalog,
            textures,
            asked,
            pointing,
            modal,
            modal_closed,
            help,
            help_clicked,
        } = self;
        detail.point_at(pointing.clone());
        let mut pictures = |file: &PictureFile| {
            asked.push(file.clone());
            textures.get(file).cloned()
        };
        let mut sources = Sources {
            catalog,
            pictures: &mut pictures,
        };
        window.frame_with(events, |ui| {
            let response = show(ui, "grid", &COLUMNS, rows, false, detail, &mut sources);
            if *modal {
                *modal_closed |= egui::Modal::new(egui::Id::unique("modal"))
                    .show(ui.ctx(), |ui| {
                        ui.label("A dialog.");
                    })
                    .should_close();
            }
            if let Some(rect) = help {
                *help_clicked |= egui::Window::new("Help")
                    .fixed_rect(*rect)
                    .show(ui.ctx(), |ui| ui.button("Its button").clicked())
                    .and_then(|inner| inner.inner)
                    .unwrap_or(false);
            }
            response
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
        // A layer that just turned interactable is click-through until a frame has
        // drawn it that way; a real wait renders the repaints asked for.
        self.idle();
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
fn many_findings_scroll_inside_a_window_that_stays_on_screen() {
    let mut scene = Scene::new();
    scene.window.screen = vec2(1280.0, 780.0);
    scene.window.ctx.global_style_mut(|style| {
        style.scroll_animation = egui::style::ScrollAnimation::none();
    });

    // A window whose findings fit keeps its size and shows no scroll bar.
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    assert_eq!(window_rect(&drawn).height(), 477.0, "a window that fits");

    // Thirty findings on the label: the window and its buttons stay on screen.
    scene.rows[0].label.messages = (1..=30)
        .map(|slot| {
            finding(
                "shared_folder_orphaned",
                Severity::Warning,
                folder("Boots/Solo"),
                &[("slot", &format!("{slot:02}"))],
            )
        })
        .collect();
    let (_, drawn) = scene.click(at.label);
    let window = window_rect(&drawn);
    for button in ["Open folder", "Copy messages"] {
        let button = drawn.one(button);
        assert!(
            button.bottom() <= 780.0,
            "{button:?} inside the screen: {button:?}"
        );
    }
    assert!(
        window.bottom() <= 780.0,
        "the window inside the screen: {window:?}"
    );

    // The last finding is reached by scrolling the findings.
    let last = || drawn.one("slot=30");
    assert!(
        last().top() > window.bottom(),
        "premise: the last finding starts below: {:?}",
        last()
    );
    let findings = drawn.one("slot=01").center();
    scene.frame(wheel(findings, vec2(0.0, -6000.0)));
    scene.wait(1.0);
    scene.idle();
    let drawn = scene.idle();
    let window = window_rect(&drawn);
    let last = drawn.one("slot=30");
    assert!(
        window.contains(last.center()) && last.bottom() < drawn.one("Open folder").top(),
        "the last finding in the drawn area: {last:?}"
    );
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
fn esc_reaches_a_modal_over_the_pinned_window_and_the_window_stays() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.three);
    scene.modal = true;
    // The modal's layer registers with the frame that draws it; Esc comes a frame later.
    scene.idle();
    scene.key(Key::Escape);
    assert!(scene.modal_closed, "the modal read Esc");
    assert!(shows(&scene.idle(), "/co/ 03"), "still pinned under it");
    scene.modal = false;
    // Its layer is gone from the frame after the modal's last one.
    scene.idle();
    scene.key(Key::Escape);
    assert!(
        !any_window(&scene.idle()),
        "the next Esc reaches the window"
    );
}

/// The detail window's painted rect: the one panel-fill rectangle of its width.
fn window_rect(drawn: &Drawn) -> Rect {
    fn rects(shape: &Shape, into: &mut Vec<(Rect, Color32)>) {
        match shape {
            Shape::Rect(shape) => into.push((shape.rect, shape.fill)),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| rects(shape, into)),
            _ => {}
        }
    }
    let fill = dark().panel_fill;
    let mut all = Vec::new();
    for shape in &drawn.shapes {
        rects(shape, &mut all);
    }
    let found: Vec<Rect> = all
        .into_iter()
        .filter(|(rect, colour)| {
            *colour == fill && rect.width() > WIDTH - 5.0 && rect.width() < WIDTH + 20.0
        })
        .map(|(rect, _)| rect)
        .collect();
    assert_eq!(found.len(), 1, "one window rect: {found:?}");
    found[0]
}

/// `spot` rested on opens nothing once a window closed over it; the pointer leaving it
/// and coming back opens `title`'s window as usual.
fn resting_where_a_window_closed_opens_nothing(
    scene: &mut Scene,
    at: &Spots,
    spot: Pos2,
    title: &str,
) {
    scene.wait(HOVER_DELAY);
    assert!(!any_window(&scene.settled()), "resting on {spot:?}");
    scene.hover(at.empty);
    scene.hover(spot);
    scene.wait(HOVER_DELAY);
    assert!(shows(&scene.settled(), title), "back on {spot:?}");
}

// TC-GUI-24
#[test]
fn esc_on_a_pinned_window_over_the_pointer_opens_nothing_until_the_pointer_leaves() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    assert!(
        window_rect(&drawn).contains(at.k6),
        "premise: the window covers K6"
    );
    // The pointer rests on the window, over K6, while Esc closes it.
    scene.hover(at.k6);
    scene.key(Key::Escape);
    assert!(!any_window(&scene.idle()), "premise: closed");
    resting_where_a_window_closed_opens_nothing(&mut scene, &at, at.k6, "/co/ K6");
}

// TC-GUI-24
#[test]
fn the_pin_of_a_pinned_window_over_the_pointer_opens_nothing_until_the_pointer_leaves() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.three);
    scene.wait(APART);
    let drawn = scene.idle();
    let bar = drawn.one("/co/ 03").center();
    // The window dragged by its title bar until its pin rests on K6.
    let delta = at.k6 - drawn.one(PIN).center();
    scene.frame(press(bar));
    scene.hover(bar + delta / 2.0);
    scene.hover(bar + delta);
    scene.frame(release(bar + delta));
    let drawn = scene.settled();
    let pin = drawn.one(PIN).center();
    assert!(
        window_rect(&drawn).contains(pin) && drawn.one("K6").contains(pin),
        "premise: the pin rests on the window, over K6"
    );
    let (_, drawn) = scene.click(pin);
    assert!(!any_window(&drawn), "premise: closed");
    resting_where_a_window_closed_opens_nothing(&mut scene, &at, pin, "/co/ K6");
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
fn a_double_click_on_a_cell_the_pinned_window_covers_opens_its_folder() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    // The double-click's first click pins K6: the window opens over the cell at its
    // first place, and the second click must reach the cell through it.
    let (_, drawn) = scene.click(at.k6);
    assert!(
        window_rect(&drawn).contains(at.k6),
        "premise: the window covers K6"
    );
    scene.frame(press(at.k6));
    let response = scene.frame(release(at.k6)).0;
    let folder = PathBuf::from("exports/co Midcup Map/Kits/p6");
    assert_eq!(response.open, Some(Open::Folder(folder)));
    scene.wait(APART);
    let drawn = scene.idle();
    assert_eq!(badge(&drawn), None, "the pin is as it was");
    assert!(!shows(&drawn, "/co/ K6"), "closed as it was");
}

// TC-GUI-24
#[test]
fn the_pinned_window_takes_a_drag_once_the_double_click_delay_has_passed() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    let title = drawn.one("/co/ 03");
    let from = title.center();
    let to = from + vec2(-100.0, 50.0);
    scene.wait(APART);
    // A layer that just turned interactable is click-through until a frame has
    // drawn it that way; a real wait renders the repaints asked for.
    scene.idle();
    scene.frame(press(from));
    scene.hover(from + vec2(-50.0, 25.0));
    scene.hover(to);
    scene.frame(release(to));
    let moved = scene.idle().one("/co/ 03");
    assert_eq!(moved.min, title.min + vec2(-100.0, 50.0));
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
    scene.click(at.three);
    scene.wait(APART);
    let button = scene.idle().one("Open folder").center();
    scene.frame(press(button));
    let (response, _) = scene.frame(release(button));
    let folder = PathBuf::from("exports/co Midcup Map/Players/03 - B");
    assert_eq!(response.open, Some(Open::Folder(folder)));
}

/// `pos`'s cell pinned and the pointer rested on its window's Open folder past
/// the tooltip delay: the frame that follows.
fn rested_on_open_folder(scene: &mut Scene, pos: Pos2) -> Drawn {
    scene.click(pos);
    // egui hides the tooltip of a widget the pointer reached in the 0.1 s after a
    // click: the wait keeps the pinning click back before the pointer moves on.
    scene.wait(0.2);
    let delay = f64::from(scene.window.ctx.global_style().interaction.tooltip_delay);
    let button = scene.idle().one("Open folder").center();
    scene.hover(button);
    scene.wait(delay + 0.1);
    // A popup's first open frame only measures: the next one shows the tooltip.
    scene.idle();
    scene.idle()
}

/// Whether a text run of `text` was painted this frame.
fn paints(drawn: &Drawn, text: &str) -> bool {
    drawn
        .shapes
        .iter()
        .any(|shape| matches!(shape, Shape::Text(shape) if shape.galley.text() == text))
}

// The archive has no folder of its own: Open folder shows it in its folder, and
// the tooltip promises no selection since no Linux file manager makes one.
#[test]
fn an_archive_cell_s_open_folder_tooltip_names_only_the_folder() {
    let mut scene = Scene::new();
    scene.rows = vec![archive_row()];
    let cell = scene.idle().one("07").center();
    let drawn = rested_on_open_folder(&mut scene, cell);
    assert!(
        paints(&drawn, "Opens the folder holding this archive"),
        "the tooltip drawn"
    );
}

#[test]
fn a_folder_cell_s_open_folder_has_no_tooltip() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let drawn = rested_on_open_folder(&mut scene, at.three);
    let open = drawn
        .nodes
        .iter()
        .find(|node| {
            node.role() == egui::accesskit::Role::Button && node.label() == Some("Open folder")
        })
        .expect("the Open folder button");
    assert_eq!(open.description(), None, "no hover text on it");
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

// A window opened over the pinned window (the help, at a finding's code) takes the
// clicks it covers.
#[test]
fn a_window_over_the_pinned_window_takes_the_click() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.three);
    scene.help = Some(window_rect(&drawn));
    let drawn = scene.settled();
    let button = drawn.one("Its button").center();
    assert!(
        scene.help.unwrap().contains(button),
        "premise: the button stands over the pinned window"
    );
    let (response, _) = scene.click(button);
    assert!(scene.help_clicked, "the click reached the window over it");
    assert_eq!(response.open, None, "the pinned window took nothing");
    assert!(shows(&scene.idle(), "/co/ 03"), "still pinned under it");
}

// TC-GUI-24
#[test]
fn the_pinned_window_s_title_bar_drags_it_and_it_reopens_where_it_was_left() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.three);
    scene.wait(APART);
    let drawn = scene.idle();
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
    scene.click(at.label);
    scene.wait(APART);
    let button = scene.idle().one("Copy messages").center();
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

/// The window's top box: the one rectangle 180 high painted in the darkest fill.
fn top_box(drawn: &Drawn) -> Rect {
    let fill = dark().extreme_bg_color;
    let boxes: Vec<Rect> = drawn
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == fill && rect.rect.height() == 180.0 => {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), 1, "one top box: {boxes:?}");
    boxes[0]
}

impl Scene {
    /// Where the picture `path` was drawn this frame, each time it was.
    fn drawn_at(&self, drawn: &Drawn, path: &str) -> Vec<Rect> {
        let texture = self.textures[&picture(path)].id();
        drawn
            .shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh.calc_bounds()),
                _ => None,
            })
            .collect()
    }
}

/// How many pictures the frame drew.
fn pictures_drawn(drawn: &Drawn) -> usize {
    drawn
        .shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Mesh(_)))
        .count()
}

/// Where "No logo" was painted, its font size and its colour, each time it was.
fn no_logos(drawn: &Drawn) -> Vec<(Rect, f32, Color32)> {
    drawn
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == "No logo" => {
                let format = &text.galley.job.sections[0].format;
                let rect = text.galley.rect.translate(text.pos.to_vec2());
                Some((rect, format.font_id.size, format.color))
            }
            _ => None,
        })
        .collect()
}

/// Whether `a` and `b` stand within a hundredth of a point of each other.
fn close(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
}

/// A picture `size` texels big fitted in `area`: as large as it fits, its shape kept, centred.
fn fitted(size: egui::Vec2, area: Rect) -> Rect {
    let scale = (area.width() / size.x).min(area.height() / size.y);
    Rect::from_center_size(area.center(), size * scale)
}

// TC-GUI-31
#[test]
fn a_kit_s_texture_is_fitted_and_centred_in_the_box() {
    let mut scene = Scene::new();
    let gk = scene.idle().one("GK").center();
    let (_, drawn) = scene.click(gk);
    let top = top_box(&drawn);
    // The 2:1 texture at the draft's largest, 180 wide and 160 high: 180 × 90.
    let expected = Rect::from_center_size(top.center(), vec2(180.0, 90.0));
    let at_kit = scene.drawn_at(&drawn, KIT);
    assert_eq!(at_kit.len(), 1, "{at_kit:?}");
    assert!(close(at_kit[0], expected), "{at_kit:?} {expected:?}");
}

// TC-GUI-31
#[test]
fn the_portraits_fill_a_6_by_4_grid_in_slot_order_the_logo_last() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.portraits);
    let top = top_box(&drawn);
    // The draft's 2-point padding and gaps: six tiles across the box, four down.
    let tile_size = vec2(
        (top.width() - 4.0 - 5.0 * 2.0) / 6.0,
        (top.height() - 4.0 - 3.0 * 2.0) / 4.0,
    );
    let tile = |index: usize| {
        let (column, row) = ((index % 6) as f32, (index / 6) as f32);
        let min = top.min + vec2(2.0, 2.0) + vec2(column, row) * (tile_size + vec2(2.0, 2.0));
        Rect::from_min_size(min, tile_size)
    };
    for slot in (1..=23).filter(|slot| *slot != 20) {
        let expected = fitted(vec2(2.0, 3.0), tile(usize::from(slot) - 1));
        let at_slot = scene.drawn_at(&drawn, &portraits_file(slot));
        assert_eq!(at_slot.len(), 1, "slot {slot}");
        assert!(close(at_slot[0], expected), "slot {slot}: {at_slot:?}");
    }
    assert_eq!(pictures_drawn(&drawn), 22, "nothing in slot 20's tile");
    let logos = no_logos(&drawn);
    assert_eq!(logos.len(), 1, "{logos:?}");
    assert!(
        (logos[0].0.center() - tile(23).center()).length() < 0.01,
        "No logo centred in the last tile: {logos:?}"
    );
}

// TC-GUI-31
#[test]
fn a_player_s_portrait_and_the_logo_stand_in_the_box_s_bottom_corners() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.one);
    let top = top_box(&drawn);
    // Each corner a portraits tile's size, 44 × 43, 2 points in from the edges.
    let left = Rect::from_min_size(
        pos2(top.left() + 2.0, top.bottom() - 45.0),
        vec2(44.0, 43.0),
    );
    let right = Rect::from_min_size(
        pos2(top.right() - 46.0, top.bottom() - 45.0),
        vec2(44.0, 43.0),
    );
    let portrait = scene.drawn_at(&drawn, PORTRAIT_01);
    let logo = scene.drawn_at(&drawn, LOGO);
    assert_eq!((portrait.len(), logo.len()), (1, 1));
    let expected = fitted(vec2(2.0, 3.0), left);
    assert!(close(portrait[0], expected), "{portrait:?} {expected:?}");
    let expected = fitted(vec2(40.0, 40.0), right);
    assert!(close(logo[0], expected), "{logo:?} {expected:?}");
    assert_eq!(pictures_drawn(&drawn), 2);
}

// TC-GUI-32
#[test]
fn a_box_with_no_logo_reads_no_logo_where_the_logo_would_be() {
    let mut scene = Scene::new();
    let a = scene.idle().one("/a/").center();
    let (_, drawn) = scene.click(a);
    let top = top_box(&drawn);
    let logos = no_logos(&drawn);
    assert_eq!(logos.len(), 1, "{logos:?}");
    let (rect, size, colour) = logos[0];
    assert!((rect.center() - top.center()).length() < 0.01, "{rect:?}");
    assert_eq!((size, colour), (9.0, dark().weak_text_color()));
    assert_eq!(pictures_drawn(&drawn), 0);
}

// TC-GUI-31
#[test]
fn a_logo_smaller_than_the_box_stands_at_its_own_size() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let (_, drawn) = scene.click(at.label);
    let top = top_box(&drawn);
    let logo = scene.drawn_at(&drawn, LOGO);
    assert_eq!(logo.len(), 1, "{logo:?}");
    let expected = Rect::from_center_size(top.center(), vec2(40.0, 40.0));
    assert!(close(logo[0], expected), "{logo:?} {expected:?}");
}

// TC-GUI-31
#[test]
fn the_text_under_the_box_starts_at_one_height_whatever_the_box_holds() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    let gk = scene.idle().one("GK").center();
    let mut tops = Vec::new();
    for (spot, place) in [
        (at.label, "co Midcup Map"),
        (gk, "Kits/g1"),
        (at.portraits, "Portraits in the player folders"),
        (at.one, "Players/01 - A"),
    ] {
        let (_, drawn) = scene.click(spot);
        assert!(
            pictures_drawn(&drawn) > 0,
            "premise: {place}'s box holds pictures"
        );
        tops.push(drawn.one(place).top());
    }
    assert!(tops.iter().all(|top| *top == tops[0]), "{tops:?}");
}

#[test]
fn only_the_shown_cell_s_pictures_are_asked_for() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.one);
    scene.asked.clear();
    scene.idle();
    let mut asked: Vec<&str> = scene.asked.iter().map(|file| file.path.as_str()).collect();
    asked.sort_unstable();
    assert_eq!(asked, [PORTRAIT_01, LOGO]);
}

// The title row covers the badge of a pinned cell scrolled under it, as it covers the cell.
// TC-GUI-24
#[test]
fn a_pinned_cell_scrolled_under_the_title_row_shows_no_badge() {
    let mut scene = Scene::new();
    scene.window.ctx.global_style_mut(|style| {
        style.scroll_animation = egui::style::ScrollAnimation::none();
    });
    scene.rows.extend(vec![a_row(); 30]);
    let at = spots(&scene.idle());
    scene.click(at.three);
    let first_row = scene.idle().one("03").top();
    // The rows scrolled up by 10 more than the title row's height: 03 stands whole under
    // it, its row still in view and drawn.
    scene.frame(wheel(at.empty, vec2(0.0, -first_row - 10.0)));
    // egui spreads a wheel's delta over the frames that follow: a second has passed it all.
    scene.wait(1.0);
    scene.idle();
    let drawn = scene.idle();
    let three = drawn.one("03");
    assert!(
        three.bottom() > 0.0 && three.bottom() <= first_row,
        "premise: 03 under the title row, at {three:?}"
    );
    assert!(shows(&drawn, "/co/ 03"), "premise: still pinned");
    assert_eq!(badge(&drawn), None);
}

/// A wheel's events at `pos`: the pointer moved there, then the wheel turned by `delta`.
fn wheel(pos: Pos2, delta: Vec2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        },
    ]
}

// TC-GUI-24
#[test]
fn the_title_row_takes_the_pointer_for_a_cell_scrolled_under_it() {
    let mut scene = Scene::new();
    scene.window.ctx.global_style_mut(|style| {
        style.scroll_animation = egui::style::ScrollAnimation::none();
    });
    scene.rows.extend(vec![a_row(); 30]);
    let first_row = scene.idle().one("03").top();
    // The rows scrolled up by one row: /co/'s row stands under the title row, still in
    // view and drawn, with the first /a/ row just below it.
    scene.frame(wheel(pos2(600.0, 300.0), vec2(0.0, -28.0)));
    scene.wait(1.0);
    scene.idle();
    let drawn = scene.idle();
    let three = drawn.one("03");
    assert!(
        three.center().y < first_row,
        "premise: 03 under the title row: {three:?}"
    );

    // Resting the pointer on the title row over it opens nothing, and a click pins
    // nothing.
    scene.hover(three.center());
    scene.wait(HOVER_DELAY);
    assert!(!any_window(&scene.settled()), "hover on the title row");
    let (_, drawn) = scene.click(three.center());
    assert!(!any_window(&drawn), "click on the title row");

    // A cell just below the title row opens its window as usual.
    let below = drawn.all("/a/")[0];
    scene.hover(below.center());
    scene.wait(HOVER_DELAY);
    assert!(
        shows(&scene.settled(), "a Midcup Grid"),
        "below the title row"
    );
}

/// /co/'s cell at `place`.
fn co(place: Place) -> CellKey {
    CellKey {
        row: "co Midcup Map".to_owned(),
        place,
    }
}

/// /co/'s player 03, its portraits and its kit K6.
const THREE: Place = Place::Position {
    line: 0,
    group: 0,
    index: 2,
};
const PORTRAITS: Place = Place::Position {
    line: 0,
    group: 1,
    index: 0,
};
const K6: Place = Place::Position {
    line: 0,
    group: 2,
    index: 6,
};

/// The rectangles the frame stroked in the selection colour, 2 points wide: the outline.
fn outlines(drawn: &Drawn) -> Vec<Rect> {
    let colour = dark().selection.stroke.color;
    drawn
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if rect.stroke.color == colour && rect.stroke.width == 2.0 => {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn cell_places_walks_the_label_then_each_line_s_cells_in_order() {
    let places: Vec<Place> = cell_places(&co_row()).map(|(place, _)| place).collect();
    let player = |index| Place::Position {
        line: 0,
        group: 0,
        index,
    };
    assert_eq!(
        places,
        [
            Place::Label,
            player(0),
            player(2),
            PORTRAITS,
            Place::Position {
                line: 0,
                group: 2,
                index: 0
            },
            K6
        ]
    );
}

// The pointer touches no cell of the grid in these tests: the window opens from the log
// line alone.
#[test]
fn a_log_line_pointing_at_a_cell_opens_its_window_after_the_hover_delay() {
    let mut scene = Scene::new();
    scene.idle();
    scene.pointing = Some(co(THREE));
    scene.idle();
    scene.wait(0.05);
    assert!(!shows(&scene.settled(), "/co/ 03"), "not before the delay");
    scene.wait(HOVER_DELAY);
    assert!(shows(&scene.settled(), "/co/ 03"), "after the delay");
    scene.pointing = None;
    assert!(!any_window(&scene.idle()), "closed when the line is left");
}

#[test]
fn a_log_line_pointing_at_a_cell_opens_nothing_while_another_is_pinned() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.click(at.one);
    // The pointer leaves the grid for the log.
    scene.frame(vec![egui::Event::PointerGone]);
    scene.pointing = Some(co(THREE));
    scene.wait(APART);
    let drawn = scene.settled();
    assert!(shows(&drawn, "/co/ 01") && !shows(&drawn, "/co/ 03"));
    // Unpinned, a line pointing at another cell opens it: the pin was what held 03 back.
    assert!(
        !any_window(&scene.key(Key::Escape)),
        "premise: Esc closed 01"
    );
    scene.pointing = Some(co(PORTRAITS));
    scene.idle();
    scene.wait(HOVER_DELAY);
    assert!(shows(&scene.settled(), "/co/ portraits"));
}

#[test]
fn go_to_pins_and_outlines_the_cell_and_a_second_unpins_it_keeping_the_outline() {
    let mut scene = Scene::new();
    let at = spots(&scene.idle());
    scene.detail.go_to(co(THREE));
    let drawn = scene.settled();
    assert!(shows(&drawn, "/co/ 03"), "pinned");
    assert!(badge(&drawn).is_some(), "its pin shows");
    assert_eq!(outlines(&drawn), [drawn.one("03")]);

    scene.detail.go_to(co(THREE));
    let drawn = scene.settled();
    assert!(!any_window(&drawn), "unpinned");
    assert_eq!(outlines(&drawn), [drawn.one("03")], "the outline stays");

    // A click in the grid pins another cell and leaves the outline where it is.
    let (_, drawn) = scene.click(at.one);
    assert!(shows(&drawn, "/co/ 01"));
    assert_eq!(outlines(&drawn), [drawn.one("03")]);

    // Another line's click moves it, to the label too.
    scene.detail.go_to(co(Place::Label));
    let drawn = scene.settled();
    assert_eq!(outlines(&drawn), [drawn.all("/co/")[0]]);
}

#[test]
fn go_to_scrolls_a_cell_out_of_view_into_it_once() {
    let mut scene = Scene::new();
    scene.window.ctx.global_style_mut(|style| {
        style.scroll_animation = egui::style::ScrollAnimation::none();
    });
    scene.window.screen = vec2(1280.0, 300.0);
    let others = (1..40).map(|n| Row {
        key: format!("a Midcup {n}"),
        ..a_row()
    });
    scene.rows = others.chain([co_row()]).collect();
    let drawn = scene.settled();
    assert!(
        drawn.all("K6").is_empty(),
        "premise: the last row is not drawn"
    );
    let rows_top = drawn.all("/a/")[0].top();

    scene.detail.go_to(co(K6));
    scene.idle();
    let drawn = scene.settled();
    let k6 = drawn.one("K6");
    assert!(
        k6.top() >= rows_top && k6.bottom() <= 300.0,
        "K6 in view, under the title row: {k6:?}"
    );

    // Scrolled back up by the member, the grid stays there.
    scene.frame(wheel(pos2(600.0, 150.0), vec2(0.0, 2000.0)));
    scene.wait(1.0);
    scene.idle();
    assert!(scene.idle().all("K6").is_empty(), "not scrolled to again");
}
