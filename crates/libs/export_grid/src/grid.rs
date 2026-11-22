//! The grid's painting (the draft's `.tc-grid`): the title row, the rows, and two vertical
//! lines down the whole area, in a scroll area both ways whose title row stays in view. Every
//! cell is a rectangle the painter fills, with a node of its own for screen readers and
//! tests; nothing in it reacts to the pointer.

use egui::{Align2, Color32, FontId, Rect, Sense, TextStyle, pos2, vec2};
use studio_core::theme::palette;

use crate::cells::{Cell, CellState, Outline, Position, Width, look, paint};
use crate::rows::{Columns, IdCell, Line, Row, RowBody, Tone};

/// A square position's side, and a line's height.
const SQUARE: f32 = 26.0;
/// The room between two cells, across and down (the draft table's `border-spacing`).
const SPACING: f32 = 2.0;
/// A wide cell's width.
const WIDE: f32 = 50.0;
/// The team label's width, the same for every team.
const LABEL_WIDTH: f32 = 52.0;
/// The room the label's text has: its width less 4 at each side.
const LABEL_TEXT_WIDTH: f32 = 44.0;
/// The space between the label's edge and its text.
const LABEL_PADDING: f32 = 4.0;
const ID_WIDTH: f32 = 36.0;
/// The line-name column's width (the draft's `.eu-line`: 44 and 4 at each side).
const LINE_NAME_WIDTH: f32 = 52.0;
/// A vertical line's column, the line 6 in from its left.
const RULE_WIDTH: f32 = 13.0;
const RULE_AT: f32 = 6.0;
/// The gap between two column groups.
const GAP: f32 = 4.0;
/// The scroll area's own margin at the left: with the spacing, 8 to the first cell.
const MARGIN: f32 = 6.0;
/// The room right of the last column's title.
const RIGHT_MARGIN: f32 = 8.0;
/// An empty position's and the ID cell's corner radius, a cell's.
const RADIUS: f32 = 2.0;
const CELL_FONT: f32 = 11.0;
const ID_FONT: f32 = 12.0;
/// The room above and below the titles' text.
const TITLE_ABOVE: f32 = 6.0;
const TITLE_BELOW: f32 = 2.0;
/// The label's font sizes: the usual 12, stepped down by half points while the name does not
/// fit; one line down to 9, and below that the name in two halves.
const NAME_LARGEST: f32 = 12.0;
const NAME_ONE_LINE: f32 = 9.0;
const NAME_SMALLEST: f32 = 5.0;

/// Draws `rows` under a title row from `columns`, in the room `ui` has left. With `checking`,
/// every cell and label is drawn in the Checking colours, whatever state it holds. `id_salt`
/// keeps the scroll position apart from another grid's.
pub fn show(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    columns: &Columns,
    rows: &[Row],
    checking: bool,
) {
    let id = ui.make_persistent_id(id_salt);
    let layout = Layout::new(columns);
    let body_font = TextStyle::Body.resolve(ui.style());
    let weak = ui.visuals().weak_text_color();
    let title_height = TITLE_ABOVE + ui.text_style_height(&TextStyle::Body) + TITLE_BELOW;
    let rows_top = SPACING + title_height + SPACING;
    let last_title =
        ui.painter()
            .layout_no_wrap(columns.last_title.to_owned(), body_font.clone(), weak);
    let width = layout.last + last_title.size().x + RIGHT_MARGIN;
    let height = rows_top + rows.iter().map(row_height).sum::<f32>();
    egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let left = area.left();
            let view = ui.clip_rect();
            let mut top = area.top() + rows_top;
            for (index, row) in rows.iter().enumerate() {
                let height = row_height(row);
                let span = egui::Rangef::new(top, top + height);
                // A row out of view is skipped: a cup's hundred rows would otherwise give
                // every frame thousands of nodes nobody sees.
                if view.y_range().intersects(span) {
                    let grid = Grid {
                        ui,
                        layout: &layout,
                        left,
                        id: id.with(index),
                        checking,
                    };
                    grid.row(row, top);
                }
                top += height;
            }
            // The title row is painted last, over the rows scrolled under it: it stays at
            // the top of the view.
            let title_top = view.top().max(area.top());
            let cover = Rect::from_x_y_ranges(view.x_range(), title_top..=title_top + rows_top);
            ui.painter()
                .rect_filled(cover, 0.0, ui.visuals().panel_fill);
            let titles = Grid {
                ui,
                layout: &layout,
                left,
                id: id.with("titles"),
                checking,
            };
            titles.titles(columns, title_top + SPACING, &body_font);
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            let bottom = area.bottom().max(view.bottom());
            for rule in layout.rules {
                let x = left + rule + RULE_AT + 0.5;
                ui.painter().vline(x, title_top..=bottom, stroke);
            }
        });
}

/// A row's height: a line's for each of its lines, one at least.
fn row_height(row: &Row) -> f32 {
    let lines = match &row.body {
        RowBody::Lines(lines) => lines.len().max(1),
        RowBody::Message { .. } => 1,
    };
    (SQUARE + SPACING) * lines as f32
}

/// Where each column starts, from the grid's left edge.
struct Layout {
    label: f32,
    id: Option<f32>,
    line_name: Option<f32>,
    /// The two vertical lines' columns.
    rules: [f32; 2],
    /// Each column group's start and its columns' width.
    groups: Vec<(f32, f32)>,
    /// The last column, right of the second vertical line.
    last: f32,
}

impl Layout {
    fn new(columns: &Columns) -> Layout {
        let mut x = MARGIN + SPACING;
        let label = take(&mut x, LABEL_WIDTH);
        let id = columns.id_title.map(|_| take(&mut x, ID_WIDTH));
        let line_name = columns.line_names.then(|| take(&mut x, LINE_NAME_WIDTH));
        let first = take(&mut x, RULE_WIDTH);
        let mut groups = Vec::new();
        for (index, group) in columns.groups.iter().enumerate() {
            if index > 0 {
                take(&mut x, GAP);
            }
            let column = column_width(group.width);
            groups.push((x, column));
            for _ in 0..group.columns {
                take(&mut x, column);
            }
        }
        let second = take(&mut x, RULE_WIDTH);
        Layout {
            label,
            id,
            line_name,
            rules: [first, second],
            groups,
            last: x,
        }
    }
}

/// A column `width` wide at `x`, which then moves past it and the spacing after it; returns
/// where the column starts.
fn take(x: &mut f32, width: f32) -> f32 {
    let at = *x;
    *x += width + SPACING;
    at
}

/// The width of a cell, or of a column, of `width`.
fn column_width(width: Width) -> f32 {
    match width {
        Width::Square => SQUARE,
        Width::Wide => WIDE,
    }
}

/// What painting one row, or the title row, needs.
struct Grid<'a> {
    ui: &'a egui::Ui,
    layout: &'a Layout,
    /// The grid's left edge on screen.
    left: f32,
    id: egui::Id,
    checking: bool,
}

impl Grid<'_> {
    /// The title row, its text's top at `top`.
    fn titles(&self, columns: &Columns, top: f32, font: &FontId) {
        let layout = self.layout;
        let title_top = top + TITLE_ABOVE;
        self.title("Team", layout.label, None, title_top, font);
        if let (Some(title), Some(x)) = (columns.id_title, layout.id) {
            self.title(title, x, Some(ID_WIDTH), title_top, font);
        }
        for (group, (x, column)) in columns.groups.iter().zip(&layout.groups) {
            let centred = (group.width == Width::Wide).then_some(*column);
            self.title(group.title, *x, centred, title_top, font);
        }
        self.title(columns.last_title, layout.last, None, title_top, font);
    }

    /// A title in the weak text at `x`, or centred over the `centred` points from `x`.
    fn title(&self, text: &str, x: f32, centred: Option<f32>, top: f32, font: &FontId) {
        let colour = self.ui.visuals().weak_text_color();
        let (at, align) = match centred {
            Some(width) => (pos2(self.left + x + width / 2.0, top), Align2::CENTER_TOP),
            None => (pos2(self.left + x, top), Align2::LEFT_TOP),
        };
        let rect = self
            .ui
            .painter()
            .text(at, align, text, font.clone(), colour);
        self.node(rect, self.id.with(text), text);
    }

    /// One row, its first line's top at `top`.
    fn row(&self, row: &Row, top: f32) {
        let layout = self.layout;
        self.label(&row.label, self.rect(layout.label, top, LABEL_WIDTH));
        if let (Some(id), Some(x)) = (&row.id, layout.id) {
            self.id_cell(id, self.rect(x, top, ID_WIDTH));
        }
        match &row.body {
            RowBody::Lines(lines) => {
                for (index, line) in lines.iter().enumerate() {
                    let line_top = top + (SQUARE + SPACING) * index as f32;
                    self.line(line, line_top, self.id.with(index));
                }
            }
            RowBody::Message { text, tone } => {
                let visuals = self.ui.visuals();
                let colour = match tone {
                    Tone::Warning => visuals.warn_fg_color,
                    Tone::Error => visuals.error_fg_color,
                };
                let x = layout.groups.first().map_or(layout.last, |(x, _)| *x);
                let at = pos2(self.left + x, top + SQUARE / 2.0);
                let font = TextStyle::Body.resolve(self.ui.style());
                let rect = self
                    .ui
                    .painter()
                    .text(at, Align2::LEFT_CENTER, text, font, colour);
                self.node(rect, self.id.with("message"), text);
            }
        }
    }

    /// One line of cells, its top at `top`.
    fn line(&self, line: &Line, top: f32, id: egui::Id) {
        if let (Some(name), Some(x)) = (line.name, self.layout.line_name) {
            let at = pos2(self.left + x + LABEL_PADDING, top + SQUARE / 2.0);
            let font = TextStyle::Body.resolve(self.ui.style());
            let colour = self.ui.visuals().weak_text_color();
            let rect = self
                .ui
                .painter()
                .text(at, Align2::LEFT_CENTER, name, font, colour);
            self.node(rect, id.with("name"), name);
        }
        for (group_index, (positions, (start, column))) in
            line.groups.iter().zip(&self.layout.groups).enumerate()
        {
            let mut x = *start;
            for (index, position) in positions.iter().enumerate() {
                let span = match position {
                    Position::Cell(cell) if cell.width == Width::Wide && *column == SQUARE => 2.0,
                    Position::Cell(_) | Position::Empty => 1.0,
                };
                match position {
                    Position::Empty => {
                        let colour = palette(self.ui.visuals()).grid_empty;
                        let rect = self.rect(x, top, *column);
                        self.ui.painter().rect_filled(rect, RADIUS, colour);
                    }
                    Position::Cell(cell) => {
                        let rect = self.rect(x, top, column_width(cell.width));
                        self.cell(cell, rect, id.with((group_index, index)));
                    }
                }
                x += span * (column + SPACING);
            }
        }
    }

    /// A rectangle one line high and `width` wide, at `x` from the grid's left.
    fn rect(&self, x: f32, top: f32, width: f32) -> Rect {
        Rect::from_min_size(pos2(self.left + x, top), vec2(width, SQUARE))
    }

    /// The state and ring `cell` is drawn in: its own, or being checked.
    fn shown(&self, cell: &Cell) -> (CellState, Option<Outline>) {
        if self.checking {
            (CellState::Checking, None)
        } else {
            (cell.state, cell.outline)
        }
    }

    /// A cell's background (`cells::paint`); returns its text's colour.
    fn background(&self, cell: &Cell, rect: Rect) -> Color32 {
        let (state, outline) = self.shown(cell);
        let look = look(state, outline, self.ui.visuals());
        paint(self.ui.painter(), &look, rect);
        look.text
    }

    /// A cell: its background, then its text centred.
    fn cell(&self, cell: &Cell, rect: Rect, id: egui::Id) {
        let colour = self.background(cell, rect);
        self.ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            &cell.text,
            FontId::monospace(CELL_FONT),
            colour,
        );
        self.node(rect, id, &cell.text);
    }

    /// The team label: its background, then its name at the left, smaller or in two lines
    /// when it does not fit at the usual size.
    fn label(&self, label: &Cell, rect: Rect) {
        let colour = self.background(label, rect);
        let painter = self.ui.painter();
        let (lines, size) = name_lines(painter, &label.text);
        let line_height = size + 1.0;
        let first = rect.center().y - line_height * (lines.len() - 1) as f32 / 2.0;
        for (index, line) in lines.iter().enumerate() {
            let at = pos2(
                rect.left() + LABEL_PADDING,
                first + line_height * index as f32,
            );
            painter.text(
                at,
                Align2::LEFT_CENTER,
                line,
                FontId::monospace(size),
                colour,
            );
        }
        self.node(rect, self.id.with("label"), &label.text);
    }

    /// The ID cell: the ID in the strong text on the widgets' grey, nothing, or `???` on the
    /// error red.
    fn id_cell(&self, id: &IdCell, rect: Rect) {
        let visuals = self.ui.visuals();
        let (fill, colour, text) = match id {
            IdCell::Known(text) => (
                visuals.widgets.inactive.bg_fill,
                visuals.strong_text_color(),
                text.as_str(),
            ),
            IdCell::Blank => (visuals.widgets.inactive.bg_fill, Color32::TRANSPARENT, ""),
            IdCell::Missing => (
                visuals.error_fg_color,
                palette(visuals).grid_text_dark,
                "???",
            ),
        };
        let painter = self.ui.painter();
        painter.rect_filled(rect, RADIUS, fill);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(ID_FONT),
            colour,
        );
        self.node(rect, self.id.with("id"), text);
    }

    /// A node for `rect`, read as `text`: what a screen reader, and a test, finds of a
    /// painted cell or title.
    fn node(&self, rect: Rect, id: egui::Id, text: &str) {
        let response = self.ui.interact(rect, id, Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
    }
}

/// The label's lines and their font size (the draft's `nameSize` and `nameLines`, the text
/// measured rather than estimated): the largest half-point size from 12 down to 9 at which
/// `name` fits one line; else its two halves, one over the other, at 9 or at what the first
/// half needs.
fn name_lines(painter: &egui::Painter, name: &str) -> (Vec<String>, f32) {
    let fits = |text: &str, size: f32| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::monospace(size),
            Color32::PLACEHOLDER,
        );
        galley.size().x <= LABEL_TEXT_WIDTH
    };
    let largest = |text: &str, smallest: f32| {
        let mut size = NAME_LARGEST;
        while size > smallest && !fits(text, size) {
            size -= 0.5;
        }
        size
    };
    let size = largest(name, NAME_ONE_LINE);
    if fits(name, size) {
        return (vec![name.to_owned()], size);
    }
    let half = name.chars().count().div_ceil(2);
    let cut = name
        .char_indices()
        .nth(half)
        .map_or(name.len(), |(at, _)| at);
    let (first, second) = name.split_at(cut);
    let size = largest(first, NAME_SMALLEST).min(NAME_ONE_LINE);
    (vec![first.to_owned(), second.to_owned()], size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::ColumnGroup;
    use crate::testing::{Drawn, Headless, dark};

    /// The Compiler's columns: 23 players, the portraits, 10 kits.
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
    const COLUMNS: Columns = Columns {
        id_title: Some("ID"),
        line_names: false,
        groups: &GROUPS,
        last_title: "Actions",
    };

    /// A checked cell showing `text`.
    fn checked(text: &str) -> Position {
        Position::Cell(Cell::new(text, CellState::FullOk))
    }

    /// `count` empty positions.
    fn empty(count: usize) -> Vec<Position> {
        vec![Position::Empty; count]
    }

    /// /a/'s row, 702: players 01 and 03, its portraits `1/2`, its goalkeeper kit.
    fn team_row() -> Row {
        let mut players = vec![checked("01"), Position::Empty, checked("03")];
        players.extend(empty(20));
        let portraits = Cell {
            width: Width::Wide,
            ..Cell::new("1/2", CellState::FullOk)
        };
        let mut kits = vec![checked("GK")];
        kits.extend(empty(9));
        Row {
            label: Cell::new("/a/", CellState::FullOk),
            id: Some(IdCell::Known("702".to_owned())),
            body: RowBody::Lines(vec![Line {
                name: None,
                groups: vec![players, vec![Position::Cell(portraits)], kits],
            }]),
        }
    }

    /// One frame of the grid of `rows` in `columns`.
    fn draw(columns: &Columns, rows: &[Row], checking: bool) -> Drawn {
        Headless::new().frame(Vec::new(), |ui| show(ui, "grid", columns, rows, checking))
    }

    #[test]
    fn a_row_draws_its_cells_at_their_positions_and_a_faint_square_where_it_has_none() {
        let drawn = draw(&COLUMNS, &[team_row()], false);
        for title in ["Team", "ID", "Players", "Portraits", "Kits", "Actions"] {
            drawn.one(title);
        }
        let (first, third) = (drawn.one("01"), drawn.one("03"));
        assert_eq!(first.size(), vec2(SQUARE, SQUARE));
        assert_eq!(third.left() - first.left(), 2.0 * (SQUARE + SPACING));
        assert!(drawn.all("02").is_empty(), "no cell at 02");
        let second = first.translate(vec2(SQUARE + SPACING, 0.0));
        let visuals = dark();
        assert_eq!(drawn.fill_at(second), Some(palette(&visuals).grid_empty));
        assert_eq!(drawn.one("1/2").width(), WIDE);
        assert_eq!(drawn.one("GK").top(), first.top());
        assert_eq!(drawn.one("702").width(), ID_WIDTH);
        assert_eq!(drawn.one("/a/").width(), LABEL_WIDTH);
    }

    #[test]
    fn a_grid_being_checked_draws_every_cell_and_label_in_the_checking_colour() {
        let visuals = dark();
        for (checking, fill) in [
            (false, palette(&visuals).grid_full_ok),
            (true, visuals.strong_text_color()),
        ] {
            let drawn = draw(&COLUMNS, &[team_row()], checking);
            for text in ["/a/", "01", "1/2", "GK"] {
                assert_eq!(
                    drawn.fill_at(drawn.one(text)),
                    Some(fill),
                    "{text} {checking}"
                );
            }
        }
    }

    #[test]
    fn a_team_the_list_lacks_shows_question_marks_on_the_error_red() {
        let row = Row {
            id: Some(IdCell::Missing),
            ..team_row()
        };
        let drawn = draw(&COLUMNS, &[row], false);
        let visuals = dark();
        assert_eq!(
            drawn.fill_at(drawn.one("???")),
            Some(visuals.error_fg_color)
        );
    }

    #[test]
    fn a_message_stands_where_the_cells_would() {
        let message = Row {
            body: RowBody::Message {
                text: "Old export format".to_owned(),
                tone: Tone::Error,
            },
            ..team_row()
        };
        let drawn = draw(&COLUMNS, &[team_row(), message], false);
        let first = drawn.all("01")[0];
        let text = drawn.one("Old export format");
        assert_eq!(text.left(), first.left());
        assert!(text.top() > first.bottom(), "on the second row");
    }

    // The Export upgrader's shape: three named lines a row, with word cells over two kit
    // columns each.
    #[test]
    fn a_row_of_three_named_lines_draws_its_names_and_its_wide_word_cells() {
        const COLUMNS: Columns = Columns {
            id_title: None,
            line_names: true,
            groups: &GROUPS,
            last_title: "Export type",
        };
        let word = |text: &str| {
            Position::Cell(Cell {
                width: Width::Wide,
                ..Cell::new(text, CellState::FullOk)
            })
        };
        let line = |name, kits: Vec<Position>| Line {
            name: Some(name),
            groups: vec![empty(23), vec![Position::Empty], kits],
        };
        let mut files = vec![word("Logo"), word("Common")];
        files.extend(empty(6));
        let row = Row {
            label: Cell::new("/a/", CellState::FullOk),
            id: None,
            body: RowBody::Lines(vec![
                line("Face", empty(10)),
                line("Boots", files),
                line("Gloves", empty(10)),
            ]),
        };
        let drawn = draw(&COLUMNS, &[row], false);
        let names = ["Face", "Boots", "Gloves"].map(|name| drawn.one(name).center().y);
        assert_eq!(names[1] - names[0], SQUARE + SPACING);
        assert_eq!(names[2] - names[1], SQUARE + SPACING);
        let (logo, common) = (drawn.one("Logo"), drawn.one("Common"));
        assert_eq!(logo.width(), WIDE);
        assert_eq!(common.left() - logo.left(), 2.0 * (SQUARE + SPACING));
        assert_eq!(logo.center().y, names[1], "on the Boots line");
        drawn.one("Export type");
    }

    #[test]
    fn a_long_name_steps_its_font_down_then_takes_two_lines() {
        // The fonts load with the first frame, so the names are measured in one.
        let mut measured = Vec::new();
        Headless::new().frame(Vec::new(), |ui| {
            measured = ["/a/", "/civ4xg/", "Invitational 1"]
                .map(|name| name_lines(ui.painter(), name))
                .to_vec();
        });
        assert_eq!(measured[0], (vec!["/a/".to_owned()], 12.0));
        let (lines, size) = &measured[1];
        assert_eq!(lines, &["/civ4xg/"]);
        assert!((9.0..12.0).contains(size), "smaller, on one line: {size}");
        let (lines, size) = &measured[2];
        assert_eq!(lines, &["Invitat", "ional 1"]);
        assert!(*size <= 9.0, "{size}");
    }

    #[test]
    fn the_two_vertical_lines_run_to_the_foot_of_the_area() {
        let drawn = draw(&COLUMNS, &[team_row()], false);
        let stroke = dark().widgets.noninteractive.bg_stroke;
        let feet: Vec<_> = drawn
            .shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::LineSegment {
                    points,
                    stroke: drawn,
                } if *drawn == stroke && points[0].x == points[1].x => Some(points[1].y),
                _ => None,
            })
            .collect();
        assert_eq!(feet, [600.0, 600.0], "two lines to the screen's foot");
    }

    #[test]
    fn only_the_rows_in_view_are_drawn() {
        let rows = vec![team_row(); 200];
        let drawn = draw(&COLUMNS, &rows, false);
        let labels = drawn.all("/a/").len();
        assert!((1..40).contains(&labels), "{labels} labels drawn");
    }

    #[test]
    fn the_title_row_stays_in_view_while_the_rows_scroll_under_it() {
        let rows = vec![team_row(); 40];
        let mut window = Headless::new();
        window.ctx.global_style_mut(|style| {
            style.scroll_animation = egui::style::ScrollAnimation::none();
        });
        let mut grid = |events| window.frame(events, |ui| show(ui, "grid", &COLUMNS, &rows, false));
        let drawn = grid(Vec::new());
        let title = drawn.one("Team");
        let first = drawn.all("/a/")[0];
        let at = first.center();
        grid(vec![
            egui::Event::PointerMoved(at),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -300.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::default(),
            },
        ]);
        let drawn = grid(Vec::new());
        assert!(drawn.all("/a/")[0].top() < first.top(), "the rows scrolled");
        assert_eq!(drawn.one("Team"), title, "the title row stayed");
    }
}
