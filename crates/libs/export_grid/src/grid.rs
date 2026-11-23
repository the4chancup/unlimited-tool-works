//! The grid's painting (the draft's `.tc-grid`): the title row, the rows, and two vertical
//! lines down the whole area, in a scroll area both ways whose title row stays in view. Every
//! cell is a rectangle the painter fills, with a node of its own for screen readers and
//! tests, which senses the pointer for the detail window.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, TextStyle, pos2, vec2};
use studio_core::theme::palette;

use crate::cells::{
    Cell, CellState, KIT_LABEL_TOP, Outline, Position, RADIUS, Width, look, paint, paint_fill,
    paint_ring, paint_swatch, shows_icon,
};
use crate::detail::{
    CellKey, Contents, Detail, Found, GridResponse, OUTLINE_WIDTH, Place, Sources, open_of,
};
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

/// Draws `rows` under a title row from `columns`, in the room `ui` has left, and the detail
/// window of the cell `detail` has open, drawn from `sources`. With
/// `checking`, every cell and label is drawn in the Checking colours, whatever state it
/// holds. `id_salt` keeps the scroll position and the window apart from another grid's.
/// Returns what the member asked for: a folder opened, the help at a finding's code.
pub fn show(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    columns: &Columns,
    rows: &[Row],
    checking: bool,
    detail: &mut Detail,
    sources: &mut Sources<'_>,
) -> GridResponse {
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
    let scroll = egui::ScrollArea::both()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let left = area.left();
            let view = ui.clip_rect();
            // Sensed before the cells, so a click lands here only where no cell is: on the
            // grid's empty space, which unpins.
            let empty = ui.interact(view, id.with("empty"), Sense::click());
            let mut found = Found::default();
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
                    grid.row(row, top, &mut |place, response| {
                        found.record(detail, &row.key, place, response);
                    });
                }
                top += height;
            }
            let double_clicked = detail.update(ui, &found, empty.clicked());
            if let Some(key) = detail.take_scroll_target()
                && let Some(cell) = cell_rect(&layout, rows, key, pos2(left, area.top() + rows_top))
            {
                // Taller by the title row, so the cell lands under it rather than behind it.
                let target = cell.expand(OUTLINE_WIDTH);
                ui.scroll_to_rect(target.with_min_y(target.top() - rows_top), None);
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
            detail.paint_outline(ui, &found, cover.bottom());
            detail.paint_pin(ui, &found, cover.bottom());
            double_clicked
        });
    let contents = Contents {
        columns,
        rows,
        checking,
        catalog: sources.catalog,
    };
    let window = detail.window(ui, id, scroll.inner_rect, &contents, sources.pictures);
    let double_clicked = scroll.inner.and_then(|cell| open_of(rows, &cell));
    GridResponse {
        open: double_clicked.or(window.open),
        help: window.help,
        overflows: scroll.content_size.y > scroll.inner_rect.height(),
    }
}

/// A row's height: a line's for each of its lines, one at least.
fn row_height(row: &Row) -> f32 {
    let lines = match &row.body {
        RowBody::Lines(lines) => lines.len().max(1),
        RowBody::Message { .. } => 1,
    };
    (SQUARE + SPACING) * lines as f32
}

/// Where the cell `key` stands on screen, whether its row is drawn or not, in a grid whose
/// left edge and first row's top are `origin`; `None` when no row has the cell.
fn cell_rect(layout: &Layout, rows: &[Row], key: &CellKey, origin: Pos2) -> Option<Rect> {
    let at = rows.iter().position(|row| row.key == key.row)?;
    let top = origin.y + rows[..at].iter().map(row_height).sum::<f32>();
    let (x, width, line) = match key.place {
        Place::Label => (layout.label, LABEL_WIDTH, 0),
        Place::Position { line, group, index } => {
            let RowBody::Lines(lines) = &rows[at].body else {
                return None;
            };
            let positions = lines.get(line)?.groups.get(group)?;
            let Position::Cell(cell) = positions.get(index)? else {
                return None;
            };
            let (start, column) = *layout.groups.get(group)?;
            let x = positions[..index].iter().fold(start, |x, position| {
                x + span(position, column) * (column + SPACING)
            });
            (x, column_width(cell.width), line)
        }
    };
    let line_top = top + (SQUARE + SPACING) * line as f32;
    Some(Rect::from_min_size(
        pos2(origin.x + x, line_top),
        vec2(width, SQUARE),
    ))
}

/// How many of its group's columns `position` takes, the group's columns `column` wide: a
/// wide cell in a group of square columns takes two.
fn span(position: &Position, column: f32) -> f32 {
    match position {
        Position::Cell(cell) if cell.width == Width::Wide && column == SQUARE => 2.0,
        Position::Cell(_) | Position::Empty => 1.0,
    }
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
        self.node(rect, self.id.with(text), text, Sense::hover());
    }

    /// One row, its first line's top at `top`; `on_cell` takes each cell's place and its
    /// response to the pointer, the label's included.
    fn row(&self, row: &Row, top: f32, on_cell: &mut impl FnMut(Place, &egui::Response)) {
        let layout = self.layout;
        let label = self.label(&row.label, self.rect(layout.label, top, LABEL_WIDTH));
        on_cell(Place::Label, &label);
        if let (Some(id), Some(x)) = (&row.id, layout.id) {
            self.id_cell(id, self.rect(x, top, ID_WIDTH));
        }
        match &row.body {
            RowBody::Lines(lines) => {
                for (line_index, line) in lines.iter().enumerate() {
                    let line_top = top + (SQUARE + SPACING) * line_index as f32;
                    let id = self.id.with(line_index);
                    self.line(line, line_top, id, &mut |group, index, response| {
                        let place = Place::Position {
                            line: line_index,
                            group,
                            index,
                        };
                        on_cell(place, response);
                    });
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
                self.node(rect, self.id.with("message"), text, Sense::hover());
            }
        }
    }

    /// One line of cells, its top at `top`; `on_cell` takes each cell's column group, its
    /// index in the group and its response to the pointer.
    fn line(
        &self,
        line: &Line,
        top: f32,
        id: egui::Id,
        on_cell: &mut impl FnMut(usize, usize, &egui::Response),
    ) {
        if let (Some(name), Some(x)) = (line.name, self.layout.line_name) {
            let at = pos2(self.left + x + LABEL_PADDING, top + SQUARE / 2.0);
            let font = TextStyle::Body.resolve(self.ui.style());
            let colour = self.ui.visuals().weak_text_color();
            let rect = self
                .ui
                .painter()
                .text(at, Align2::LEFT_CENTER, name, font, colour);
            self.node(rect, id.with("name"), name, Sense::hover());
        }
        for (group_index, (positions, (start, column))) in
            line.groups.iter().zip(&self.layout.groups).enumerate()
        {
            let mut x = *start;
            for (index, position) in positions.iter().enumerate() {
                match position {
                    Position::Empty => {
                        let colour = palette(self.ui.visuals()).grid_empty;
                        let rect = self.rect(x, top, *column);
                        self.ui.painter().rect_filled(rect, RADIUS, colour);
                    }
                    Position::Cell(cell) => {
                        let rect = self.rect(x, top, column_width(cell.width));
                        let response = self.cell(cell, rect, id.with((group_index, index)));
                        on_cell(group_index, index, &response);
                    }
                }
                x += span(position, *column) * (column + SPACING);
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

    /// A cell: its fill, a kit's menu icon along its foot when the state shows one,
    /// its ring, then its text, at the cell's top when an icon shows and centred
    /// without one; returns its response to the pointer.
    fn cell(&self, cell: &Cell, rect: Rect, id: egui::Id) -> egui::Response {
        let (state, outline) = self.shown(cell);
        let look = look(state, outline, self.ui.visuals());
        let painter = self.ui.painter();
        paint_fill(painter, &look, rect);
        let icon = cell.kit_icon.filter(|_| shows_icon(state));
        if let Some(icon) = icon {
            paint_swatch(painter, icon, rect);
        }
        // The ring goes over the swatch: the draft's `.ol` stands over both.
        paint_ring(painter, &look, rect);
        let (at, align) = if icon.is_some() {
            (
                pos2(rect.center().x, rect.top() + KIT_LABEL_TOP),
                Align2::CENTER_TOP,
            )
        } else {
            (rect.center(), Align2::CENTER_CENTER)
        };
        painter.text(
            at,
            align,
            &cell.text,
            FontId::monospace(CELL_FONT),
            look.text,
        );
        self.node(rect, id, &cell.text, Sense::click())
    }

    /// The team label: its background, then its name at the left, smaller or in two lines
    /// when it does not fit at the usual size; returns its response to the pointer.
    fn label(&self, label: &Cell, rect: Rect) -> egui::Response {
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
        self.node(rect, self.id.with("label"), &label.text, Sense::click())
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
        // It senses clicks, though nothing reacts to them yet, so a click on it is not one
        // on the grid's empty space, which unpins.
        self.node(rect, self.id.with("id"), text, Sense::click());
    }

    /// A node for `rect`, read as `text`: what a screen reader, and a test, finds of a
    /// painted cell or title. It senses the pointer as `sense` says.
    fn node(&self, rect: Rect, id: egui::Id, text: &str, sense: Sense) -> egui::Response {
        let response = self.ui.interact(rect, id, sense);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, text));
        response
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
    use color_tools::icon::pattern_shapes;

    use super::*;
    use crate::cells::KitIcon;
    use crate::testing::{COLUMNS, Drawn, GROUPS, Headless, dark};

    /// A checked cell showing `text`.
    fn checked(text: &str) -> Position {
        Position::Cell(Box::new(Cell::new(text, CellState::FullOk)))
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
            key: "a Midcup Grid".to_owned(),
            label: Cell::new("/a/", CellState::FullOk),
            id: Some(IdCell::Known("702".to_owned())),
            body: RowBody::Lines(vec![Line {
                name: None,
                groups: vec![players, vec![Position::Cell(Box::new(portraits))], kits],
            }]),
            log_lines: Vec::new(),
        }
    }

    /// The row's kit positions: GK, then K1 to K9.
    fn kits_mut(row: &mut Row) -> &mut Vec<Position> {
        let RowBody::Lines(lines) = &mut row.body else {
            unreachable!("the rows are lines")
        };
        &mut lines[0].groups[2]
    }

    /// `team_row` with GK's menu icon (pattern 7, red on white) under an Error ring.
    fn icon_row() -> Row {
        let mut row = team_row();
        let Position::Cell(gk) = &mut kits_mut(&mut row)[0] else {
            unreachable!("GK is a cell")
        };
        gk.outline = Some(Outline::Error);
        gk.kit_icon = Some(KitIcon {
            number: 7,
            colors: [Color32::RED, Color32::WHITE],
        });
        row
    }

    /// The grid of `rows` in `columns`, with a detail window that has nothing open.
    fn plain(ui: &mut egui::Ui, columns: &Columns, rows: &[Row], checking: bool) -> GridResponse {
        let mut sources = Sources {
            catalog: &[],
            pictures: &mut |_| None,
        };
        show(
            ui,
            "grid",
            columns,
            rows,
            checking,
            &mut Detail::default(),
            &mut sources,
        )
    }

    /// One frame of the grid of `rows` in `columns`.
    fn draw(columns: &Columns, rows: &[Row], checking: bool) -> Drawn {
        Headless::new().frame(Vec::new(), |ui| plain(ui, columns, rows, checking))
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
        assert_eq!(
            text.center().y,
            first.center().y + SQUARE + SPACING,
            "centred on the second row"
        );
    }

    // The Export upgrader's shape: three named lines a row, with word cells over two kit
    // columns each.
    #[test]
    fn a_row_of_three_named_lines_draws_its_names_and_its_wide_word_cells() {
        let word = |text: &str| {
            Position::Cell(Box::new(Cell {
                width: Width::Wide,
                ..Cell::new(text, CellState::FullOk)
            }))
        };
        let line = |name, kits: Vec<Position>| Line {
            name: Some(name),
            groups: vec![empty(23), vec![Position::Empty], kits],
        };
        let mut files = vec![word("Logo"), word("Common")];
        files.extend(empty(6));
        let row = Row {
            key: "a Midcup Grid".to_owned(),
            label: Cell::new("/a/", CellState::FullOk),
            id: None,
            body: RowBody::Lines(vec![
                line("Face", empty(10)),
                line("Boots", files),
                line("Gloves", empty(10)),
            ]),
            log_lines: Vec::new(),
        };
        let drawn = draw(&NAMED_COLUMNS, &[row], false);
        let names = ["Face", "Boots", "Gloves"].map(|name| drawn.one(name).center().y);
        assert_eq!(names[1] - names[0], SQUARE + SPACING);
        assert_eq!(names[2] - names[1], SQUARE + SPACING);
        let (logo, common) = (drawn.one("Logo"), drawn.one("Common"));
        // The first kit cell: the 52 name column at 62 + 2, the line's 13 column + 2,
        // the 23 players of (26 + 2), the 6 gap, the 50 portraits and the 6 gap: 839.
        assert_eq!(logo.left(), 839.0);
        assert_eq!(logo.width(), WIDE);
        assert_eq!(common.left() - logo.left(), 2.0 * (SQUARE + SPACING));
        assert_eq!(logo.center().y, names[1], "on the Boots line");
        drawn.one("Export type");
    }

    #[test]
    fn a_long_name_steps_its_font_down_then_takes_two_lines() {
        // The fonts load with the first frame, so the names are measured in one.
        // The last name's halves are too wide for the label even at the smallest size.
        let long = "Spring 2026 Invitational Cup Midcup Exports";
        let mut measured = Vec::new();
        let mut half_width = 0.0;
        Headless::new().frame(Vec::new(), |ui| {
            measured = ["/a/", "/civ4xg/", "Invitational 1", long]
                .map(|name| name_lines(ui.painter(), name))
                .to_vec();
            let galley = ui.painter().layout_no_wrap(
                measured[3].0[0].clone(),
                FontId::monospace(NAME_SMALLEST),
                Color32::PLACEHOLDER,
            );
            half_width = galley.size().x;
        });
        assert_eq!(measured[0], (vec!["/a/".to_owned()], 12.0));
        let (lines, size) = &measured[1];
        assert_eq!(lines, &["/civ4xg/"]);
        assert!((9.0..12.0).contains(size), "smaller, on one line: {size}");
        let (lines, size) = &measured[2];
        assert_eq!(lines, &["Invitat", "ional 1"]);
        assert!(*size <= 9.0, "{size}");
        assert!(half_width > LABEL_TEXT_WIDTH, "premise: {half_width} wide");
        let (lines, size) = &measured[3];
        assert_eq!(lines.len(), 2);
        assert_eq!(*size, NAME_SMALLEST, "no smaller than the smallest size");
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
    fn rows_taller_than_the_room_the_grid_has_overflow_it() {
        // 300 points hold the title row and about nine rows of 28.
        for (count, overflows) in [(30, true), (2, false)] {
            let mut window = Headless::new();
            window.screen = vec2(1280.0, 300.0);
            let rows = vec![team_row(); count];
            let (response, _) =
                window.frame_with(Vec::new(), |ui| plain(ui, &COLUMNS, &rows, false));
            assert_eq!(response.overflows, overflows, "{count} rows");
        }
    }

    /// The grid scroll area's offset, read without running a frame: one would
    /// advance egui's own frame count and input time.
    fn scroll_offset_of(ctx: &egui::Context) -> egui::Vec2 {
        let top = egui::ViewportId::ROOT.root_ui_id();
        let id = top.with(egui::IdSalt::new(top.with("grid")));
        egui::containers::scroll_area::State::load(ctx, id)
            .map(|state| state.unclamped_offset())
            .expect("the grid's scroll area")
    }

    /// A window whose scroll moves at once, for a test that scrolls the grid.
    fn scrolling() -> Headless {
        let window = Headless::new();
        window.ctx.global_style_mut(|style| {
            style.scroll_animation = egui::style::ScrollAnimation::none();
        });
        window
    }

    #[test]
    fn the_title_row_stays_in_view_while_the_rows_scroll_under_it() {
        let rows = vec![team_row(); 40];
        let mut window = scrolling();
        let mut grid = |events| window.frame(events, |ui| plain(ui, &COLUMNS, &rows, false));
        let drawn = grid(Vec::new());
        let title = drawn.one("Team");
        let first = drawn.all("/a/")[0];
        grid(wheel(first.center(), vec2(0.0, -300.0)));
        let drawn = grid(Vec::new());
        assert!(drawn.all("/a/")[0].top() < first.top(), "the rows scrolled");
        assert_eq!(drawn.one("Team"), title, "the title row stayed");
    }

    /// The Export upgrader's columns: no ID, a name for each line.
    const NAMED_COLUMNS: Columns = Columns {
        id_title: None,
        line_names: true,
        groups: &GROUPS,
        last_title: "Export type",
    };

    /// `text`'s painted run: the label's and cells' painted text is no node, it is a
    /// `Shape::Text`.
    fn text_shape<'a>(drawn: &'a Drawn, text: &str) -> &'a egui::epaint::TextShape {
        drawn
            .shapes
            .iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text_shape) if text_shape.galley.text() == text => {
                    Some(text_shape)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no text run of {text:?}"))
    }

    /// Where the text run of `text` is painted: `pos` is the galley's top-left corner.
    fn text_pos(drawn: &Drawn, text: &str) -> egui::Pos2 {
        text_shape(drawn, text).pos
    }

    /// The middle of `text`'s painted run, from its galley's top-left corner and size.
    fn text_center(drawn: &Drawn, text: &str) -> egui::Pos2 {
        let shape = text_shape(drawn, text);
        shape.pos + shape.galley.size() / 2.0
    }

    /// The index in `shapes` where `run` sits as one contiguous run; panics when no
    /// window of its length matches it.
    fn run_at(shapes: &[egui::Shape], run: &[egui::Shape]) -> usize {
        shapes
            .windows(run.len())
            .position(|window| window == run)
            .expect("the run of shapes")
    }

    /// The scroll area's persisted state, for the grid `show` draws under `salt`: the salt
    /// it hands the area is the grid's own id, so the state lands under it salted again.
    fn scroll_state(ctx: &egui::Context, salt: &str) -> egui::containers::scroll_area::State {
        let mut id = egui::Id::NULL;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            id = ui.make_persistent_id(egui::IdSalt::new(ui.make_persistent_id(salt)));
        })
        .drop_without_applying_deltas();
        egui::containers::scroll_area::State::load(ctx, id)
            .expect("the scroll area stored its state")
    }

    /// A wheel at `pos` scrolled by `delta`, the pointer over the grid first.
    fn wheel(pos: egui::Pos2, delta: egui::Vec2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta,
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    /// A row of three named lines under `label`.
    fn named_row(label: &str) -> Row {
        let line = |name| Line {
            name: Some(name),
            groups: vec![empty(23), vec![Position::Empty], empty(10)],
        };
        Row {
            key: label.to_owned(),
            label: Cell::new(label, CellState::FullOk),
            id: None,
            body: RowBody::Lines(vec![line("Face"), line("Boots"), line("Gloves")]),
            log_lines: Vec::new(),
        }
    }

    // The grid's area starts at the frame's (0, 0): the headless window hands the contents
    // the whole 1280 by 600 screen and the scroll area's content starts at its viewport's
    // own corner.
    #[test]
    fn the_columns_stand_at_the_drafts_positions() {
        let drawn = draw(&COLUMNS, &[team_row()], false);
        // The left margin is the scroll area's 6, then the table's 2 border-spacing.
        assert_eq!(drawn.one("/a/").left(), 8.0);
        // The ID at 8 + the 52 label + 2.
        assert_eq!(drawn.one("702").left(), 62.0);
        // The first player at 62 + the 36 ID + 2 + the line's 13 column + 2.
        assert_eq!(drawn.one("01").left(), 115.0);
        // Portraits at 115 + 23 columns of (26 + 2) + the group's 4 gap + 2.
        assert_eq!(drawn.one("1/2").left(), 765.0);
        // The first kit at 765 + the 50 portraits + 2 + the group's 4 gap + 2.
        assert_eq!(drawn.one("GK").left(), 823.0);
        // Actions at 823 + 10 columns of (26 + 2) + the second line's 13 column + 2.
        assert_eq!(drawn.one("Actions").left(), 1118.0);
        // The title text's top: the spacing 2, then the head's 6 padding above its text.
        assert_eq!(drawn.one("Team").top(), 8.0);
        // The first row's top: 2 + the head (6 + the 14.65625 body row + 2) + 2.
        assert_eq!(drawn.one("/a/").top(), 26.65625);
    }

    #[test]
    fn the_portraits_title_is_centred_over_its_50_wide_column() {
        let drawn = draw(&COLUMNS, &[team_row()], false);
        // 765 + 25, the column's middle (the draft's .mid titles, 50 wide and centred).
        assert_eq!(drawn.one("Portraits").center().x, 790.0);
    }

    #[test]
    fn the_vertical_lines_stand_6_into_their_13_wide_columns() {
        let drawn = draw(&COLUMNS, &[team_row()], false);
        // The columns at 100 and 1103, the line 6 in from each left and the half point
        // that keeps a 1-wide line on a whole pixel.
        let xs: Vec<_> = drawn
            .shapes
            .iter()
            .filter_map(|shape| match shape {
                egui::Shape::LineSegment { points, stroke }
                    if *stroke == dark().widgets.noninteractive.bg_stroke
                        && points[0].x == points[1].x =>
                {
                    Some(points[0].x)
                }
                _ => None,
            })
            .collect();
        assert_eq!(xs, [106.5, 1109.5]);
    }

    #[test]
    fn a_row_of_three_lines_is_three_lines_high() {
        // Three lines of 26 with the 2 between them and after: 3 × 28 from the row's top,
        // the next row's label top proves it (26.65625 + 84).
        let drawn = draw(&NAMED_COLUMNS, &[named_row("/a/"), named_row("/b/")], false);
        assert_eq!(drawn.one("/b/").top(), 110.65625);
    }

    #[test]
    fn a_row_scrolled_half_above_the_view_is_still_drawn() {
        // A 300 scroll leaves the topmost line in view cut by its top: its span still
        // intersects the view, so exactly one drawn label's top is above it.
        let rows = vec![team_row(); 40];
        let mut window = scrolling();
        let mut grid = |events| window.frame(events, |ui| plain(ui, &COLUMNS, &rows, false));
        grid(Vec::new());
        grid(wheel(pos2(600.0, 300.0), vec2(0.0, -300.0)));
        let drawn = grid(Vec::new());
        let labels = drawn.all("/a/");
        let above = labels.iter().filter(|rect| rect.top() < 0.0).count();
        assert_eq!(above, 1, "the partly visible top row");
        assert!(
            labels.iter().all(|rect| rect.top() < 600.0),
            "no row drawn below the view"
        );
    }

    // A click on the tool's log line scrolls its cell into view under the title row, which
    // stays put over the rows.
    #[test]
    fn a_go_to_scrolls_the_cell_to_just_under_the_title_row() {
        let rows = vec![team_row(); 40];
        let mut detail = Detail::default();
        let mut window = scrolling();
        let ctx = window.ctx.clone();
        let mut grid = |detail: &mut Detail, events| {
            let mut sources = Sources {
                catalog: &[],
                pictures: &mut |_| None,
            };
            window.frame(events, |ui| {
                show(ui, "grid", &COLUMNS, &rows, false, detail, &mut sources)
            })
        };
        // The first row's top in an unscrolled frame is the title row's bottom.
        let title_bottom = grid(&mut detail, Vec::new())
            .all("/a/")
            .iter()
            .map(egui::Rect::top)
            .fold(f32::INFINITY, f32::min);
        // A wheel delta under 8 points lands at once; a big one is smoothed over
        // frames, and each of those frames' wheel handling would keep clearing
        // the scroll-to-cell's target.
        let mut to_bottom = vec![egui::Event::PointerMoved(pos2(600.0, 300.0))];
        to_bottom.extend((0..90).map(|_| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -7.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        }));
        grid(&mut detail, to_bottom);
        grid(&mut detail, Vec::new());
        assert!(
            scroll_offset_of(&ctx).y > 500.0,
            "premise: scrolled to the bottom: {:?}",
            scroll_offset_of(&ctx)
        );

        detail.go_to(CellKey {
            row: "a Midcup Grid".to_owned(),
            place: Place::Position {
                line: 0,
                group: 0,
                index: 0,
            },
        });
        grid(&mut detail, Vec::new());
        // The scroll lands over the next frames; run until the cell's top stops moving.
        let mut top = f32::MAX;
        let mut settled = false;
        for _ in 0..120 {
            let drawn = grid(&mut detail, Vec::new());
            let now = drawn
                .all("01")
                .iter()
                .map(egui::Rect::top)
                .fold(f32::INFINITY, f32::min);
            if now == top {
                settled = true;
                break;
            }
            top = now;
        }
        assert!(settled, "the cell's top never settled in 120 frames");
        // The rows are alike, so a scroll to another row would also put an "01" under the
        // title: the offset says it is the first row's.
        assert!(
            scroll_offset_of(&ctx).y < 28.0,
            "scrolled back up to the first row: {:?}",
            scroll_offset_of(&ctx)
        );
        assert!(top.is_finite(), "the cell is drawn: {top}");
        assert!(
            top >= title_bottom,
            "the cell's top is at or below the title row's bottom: {top} vs {title_bottom}"
        );
        // Wheel steps after the settle stay where they land: the
        // scroll-to-cell's target was taken once, so nothing pulls the
        // view back to the first row.
        let mut steps = vec![egui::Event::PointerMoved(pos2(600.0, 300.0))];
        steps.extend((0..10).map(|_| egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, -7.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        }));
        grid(&mut detail, steps);
        grid(&mut detail, Vec::new());
        let offset = scroll_offset_of(&ctx).y;
        assert!(
            offset >= 28.0,
            "the wheel moved the view off the first row: {offset}"
        );
        grid(&mut detail, Vec::new());
        assert_eq!(
            scroll_offset_of(&ctx).y,
            offset,
            "the offset holds a frame later"
        );
    }

    #[test]
    fn the_scroll_area_holds_exactly_the_content_the_rows_make() {
        // 40 rows of 28 under rows_top: the content is 26.65625 + 1120 = 1146.65625
        // high and 1118 + the 42.25 Actions title + the 8 right margin = 1168.25
        // wide. A window 1000 by 600 is too small both ways, so the farthest the
        // scroll goes is the content less the window.
        let rows = vec![team_row(); 40];
        let mut window = scrolling();
        window.screen = vec2(1000.0, 600.0);
        let mut grid = |events| window.frame(events, |ui| plain(ui, &COLUMNS, &rows, false));
        grid(Vec::new());
        grid(wheel(pos2(600.0, 300.0), vec2(-100000.0, -100000.0)));
        grid(Vec::new());
        let state = scroll_state(&window.ctx, "grid");
        assert_eq!(state.unclamped_offset(), vec2(168.25, 546.65625));
    }

    #[test]
    fn the_title_rows_cover_reaches_the_first_row() {
        // The cover over the scrolled rows: the view's width, from its top to rows_top
        // (26.65625), in the panel's fill.
        let drawn = draw(&COLUMNS, &[team_row()], false);
        let cover = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 26.65625));
        let panel = dark().panel_fill;
        assert!(
            drawn.shapes.iter().any(|shape| matches!(
                shape,
                egui::Shape::Rect(rect) if rect.fill == panel && rect.rect == cover
            )),
            "no panel-fill cover over {cover:?}"
        );
    }

    #[test]
    fn the_labels_text_stands_4_in_centred_on_one_line_or_two() {
        let long = Row {
            label: Cell::new("Invitational 1", CellState::FullOk),
            ..team_row()
        };
        let drawn = draw(&COLUMNS, &[team_row(), long], false);
        // One line at the usual 12: 4 in from the label's left (8 + 4), its 14-high galley
        // centred on the label's middle (26.65625 + 13 - 7).
        assert_eq!(text_pos(&drawn, "/a/"), pos2(12.0, 32.65625));
        // Two lines at 9, one `size + 1` apart (10): anchored 5 over and under the second
        // label's middle (54.65625 + 13), each galley 10 high.
        assert_eq!(text_pos(&drawn, "Invitat"), pos2(12.0, 57.65625));
        assert_eq!(text_pos(&drawn, "ional 1"), pos2(12.0, 67.65625));
    }

    #[test]
    fn a_lines_name_stands_4_into_its_column_centred_on_the_line() {
        let drawn = draw(&NAMED_COLUMNS, &[named_row("/a/")], false);
        // The name column at 62 (the label's 8 + 52 + 2, no ID): the text 4 in.
        assert_eq!(drawn.one("Face").left(), 66.0);
        // Centred on its 26-high line at 26.65625: 39.65625, the third line's 56 down.
        assert_eq!(drawn.one("Face").center().y, 39.65625);
        assert_eq!(drawn.one("Gloves").center().y, 95.65625);
    }

    /// The strip a kit cell's menu icon fills, worked out from the cell's `rect` as the
    /// draft's `.tc-sw` marks it out: 2 in from the left, right and bottom, 9 high.
    fn icon_strip(rect: Rect) -> Rect {
        Rect::from_min_max(
            pos2(rect.left() + 2.0, rect.bottom() - 11.0),
            pos2(rect.right() - 2.0, rect.bottom() - 2.0),
        )
    }

    #[test]
    fn a_kit_cell_paints_its_menu_icon_between_its_fill_and_its_ring() {
        let drawn = draw(&COLUMNS, &[icon_row()], false);
        let cell = drawn.one("GK");
        let pattern = pattern_shapes(icon_strip(cell), 7, [Color32::RED, Color32::WHITE]);
        let run = run_at(&drawn.shapes, &pattern);
        let visuals = dark();
        let fill = drawn
            .shapes
            .iter()
            .position(|shape| {
                matches!(
                    shape,
                    egui::Shape::Rect(rect)
                        if rect.rect == cell && rect.fill == palette(&visuals).grid_full_ok
                )
            })
            .expect("the cell's fill");
        let ring = drawn
            .shapes
            .iter()
            .position(|shape| {
                matches!(
                    shape,
                    egui::Shape::Rect(rect)
                        if rect.rect == cell && rect.stroke.color == visuals.error_fg_color
                )
            })
            .expect("the cell's Error ring");
        assert!(fill < run, "the fill paints before the pattern");
        assert!(
            run + pattern.len() <= ring,
            "the ring paints after the pattern"
        );
    }

    #[test]
    fn an_icon_cells_label_stands_at_the_top_a_plain_ones_stays_centred() {
        let mut row = icon_row();
        kits_mut(&mut row)[1] = checked("K1");
        let drawn = draw(&COLUMNS, &[row], false);
        let top = text_pos(&drawn, "GK").y - drawn.one("GK").top();
        assert!(
            (top - 3.0).abs() < 0.5,
            "the galley 3 under the cell's top: {top}"
        );
        let off = text_center(&drawn, "K1") - drawn.one("K1").center();
        assert!(off.length() < 0.5, "K1's label stays centred: {off:?}");
    }

    #[test]
    fn a_kit_cell_shows_no_icon_while_checking_blocked_or_unchecked() {
        for (checking, state) in [
            (true, CellState::FullOk),
            (false, CellState::Blocked),
            (false, CellState::Unchecked),
        ] {
            let mut row = icon_row();
            let Position::Cell(gk) = &mut kits_mut(&mut row)[0] else {
                unreachable!("GK is a cell")
            };
            gk.state = state;
            let drawn = draw(&COLUMNS, &[row], checking);
            let cell = drawn.one("GK");
            let pattern = pattern_shapes(icon_strip(cell), 7, [Color32::RED, Color32::WHITE]);
            assert!(
                !drawn
                    .shapes
                    .windows(pattern.len())
                    .any(|window| window == pattern),
                "no icon while {state:?} (checking {checking})"
            );
            let off = text_center(&drawn, "GK") - cell.center();
            assert!(
                off.length() < 0.5,
                "the label stays centred while {state:?} (checking {checking}): {off:?}"
            );
        }
    }

    #[test]
    fn a_kit_cell_shows_its_icon_on_the_states_that_keep_it() {
        for state in [
            CellState::PartialOk,
            CellState::Done,
            CellState::DoneWithErrors,
        ] {
            let mut row = icon_row();
            let Position::Cell(gk) = &mut kits_mut(&mut row)[0] else {
                unreachable!("GK is a cell")
            };
            gk.state = state;
            let drawn = draw(&COLUMNS, &[row], false);
            let cell = drawn.one("GK");
            let pattern = pattern_shapes(icon_strip(cell), 7, [Color32::RED, Color32::WHITE]);
            run_at(&drawn.shapes, &pattern);
            let top = text_pos(&drawn, "GK").y - cell.top();
            assert!(
                (top - 3.0).abs() < 0.5,
                "the label at the top while {state:?}: {top}"
            );
        }
    }
}
