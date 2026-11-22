//! The detail window (the draft's `.tc-detail`): what one cell stands for, opened by resting
//! the pointer on the cell, kept open by a click that pins it. It shows the cell's folder,
//! its state and its findings with the catalog's texts, and offers Open folder and, on a
//! team label, Copy messages.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Align2, Color32, FontId, Key, Layout, Modifiers, Pos2, Rect, RichText, Sense};
use egui::{TextStyle, TextureHandle, Vec2, pos2, vec2};
use studio_core::theme::NO_TINT;
use studio_core::{CatalogRow, Message, MessageCode, Scope, Severity};

use crate::cells::{Cell, CellState, Open, Outline, PictureFile, Position, TopBox, state_name};
use crate::rows::{Columns, Row, RowBody};

/// How long the pointer rests on a cell before its window opens. It is longer than the
/// sidebar's 0.1 s, so a pointer crossing the grid does not flash a window this big over
/// every cell it passes.
const HOVER_DELAY: f64 = 0.15;
/// How soon after a hover window closes the next cell's opens at once: crossing the gap
/// between two cells closes the window for a moment, and it should not wait its delay again.
const TOOLTIP_GRACE: f64 = 0.2;
/// How long Copy messages reads "Copied" after a click.
const COPIED_FOR: f64 = 1.5;
/// The window's width.
const WIDTH: f32 = 300.0;
/// The top box's height: the 3D preview's, on every cell, so the text under it stays in
/// place as the window moves from cell to cell.
const BOX_HEIGHT: f32 = 180.0;
/// The title bar's height.
const BAR_HEIGHT: f32 = 22.0;
/// The room around the window's body.
const BODY_MARGIN: i8 = 8;
/// The room above each finding.
const FINDING_GAP: f32 = 6.0;
/// The window's distance from the grid's right and top edges until it is moved.
const INSET: f32 = 8.0;
/// The pin, on a pinned cell and in the window's title bar.
const PIN: &str = "📌";
/// The pin badge's radius, and its glyph's size on the badge and in the title bar.
const BADGE_RADIUS: f32 = 7.0;
/// How far in from a cell's top right corner the badge's centre stands, across and down.
const BADGE_INSET: f32 = 2.0;
const BADGE_GLYPH: f32 = 11.0;
const BAR_GLYPH: f32 = 15.0;
/// The largest a picture fitted in the top box is drawn (the draft's `.preview img`).
const PICTURE_MAX: Vec2 = vec2(180.0, 160.0);
/// The portraits cell's grid: its tiles across and down, and the room around and between
/// them.
const TILES_ACROSS: usize = 6;
const TILES_DOWN: usize = 4;
const TILE_GAP: f32 = 2.0;
/// A player's corner tiles: a portraits tile's size, `TILE_GAP` in from the box's edges.
const CORNER: Vec2 = vec2(44.0, 43.0);
/// The font size of "No logo".
const NO_LOGO_SIZE: f32 = 9.0;

/// A cell of the grid: its row's key and its place in the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CellKey {
    pub(crate) row: String,
    pub(crate) place: Place,
}

/// Where a cell stands in its row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    /// The team label.
    Label,
    /// A position: its line, its column group, and its index in the group.
    Position {
        line: usize,
        group: usize,
        index: usize,
    },
}

/// What the detail window keeps from one frame to the next: which cell is pinned, which one
/// the pointer is on, where the window stands. A tool keeps one per grid, for the session.
#[derive(Debug, Default)]
pub struct Detail {
    /// The pinned cell: the window stays open on it.
    pinned: Option<CellKey>,
    /// The pin as it was before the last click on a cell. A double-click starts with a click
    /// that pins or unpins, and puts this back.
    pinned_before_click: Option<CellKey>,
    /// The cell under the pointer.
    under: Option<Under>,
    /// When the last window a hover opened closed.
    hover_closed_at: Option<f64>,
    /// Where the window's top left corner stands once the member has moved it.
    position: Option<Pos2>,
    /// The row whose label's Copy messages was clicked, and until when it reads "Copied".
    copied: Option<(String, f64)>,
}

/// The cell under the pointer, and when its window opens on hover.
#[derive(Debug)]
struct Under {
    cell: CellKey,
    /// `None`: it opens nothing until the pointer leaves the cell. The pointer came onto it
    /// while another cell was pinned, or a click on it unpinned, or Esc closed its window.
    opens_at: Option<f64>,
}

/// What the member asked of the grid this frame, for the tool to carry out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridResponse {
    /// A folder to open in the file manager: a cell was double-clicked, or its window's Open
    /// folder clicked.
    pub open: Option<Open>,
    /// A finding's code clicked in the window: the help opens at its catalog row.
    pub help: Option<MessageCode>,
}

/// What the detail window draws from beside the rows: the catalog that explains a finding,
/// and the tool's pictures, `None` while one loads or when it cannot be read.
pub struct Sources<'a> {
    /// The tool's message catalog.
    pub catalog: &'a [CatalogRow],
    /// The texture of a picture the window shows. It is asked only for the pictures of the
    /// cell whose window is drawn, each frame the window is open.
    pub pictures: &'a mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
}

/// What the window shows a cell from: the grid's columns and rows as `show` was handed them,
/// whether the grid is drawn as being checked, and the catalog that explains a finding.
pub(crate) struct Contents<'a> {
    pub(crate) columns: &'a Columns,
    pub(crate) rows: &'a [Row],
    pub(crate) checking: bool,
    pub(crate) catalog: &'a [CatalogRow],
}

/// What the pointer did on the grid's cells this frame.
#[derive(Debug, Default)]
pub(crate) struct Found {
    hovered: Option<CellKey>,
    clicked: Option<CellKey>,
    double_clicked: bool,
    /// The cell pinned when the frame began, and its rectangle. A cell a click pins gets its
    /// pin badge from the next frame on.
    pinned: Option<(CellKey, Rect)>,
}

impl Found {
    /// Takes in what `response`, the cell at `place` in the row keyed `row`, reports.
    pub(crate) fn record(
        &mut self,
        detail: &Detail,
        row: &str,
        place: Place,
        response: &egui::Response,
    ) {
        let key = || CellKey {
            row: row.to_owned(),
            place,
        };
        if response.hovered() {
            self.hovered = Some(key());
        }
        if response.clicked() {
            self.clicked = Some(key());
            self.double_clicked = response.double_clicked();
        }
        let pinned = detail.pinned.as_ref();
        if pinned.is_some_and(|pinned| pinned.row == row && pinned.place == place) {
            self.pinned = Some((key(), response.rect));
        }
    }
}

impl Detail {
    /// Takes in the frame's pointer and keys: the hover that opens a window, the clicks that
    /// pin and unpin, Esc. `empty_clicked` is a click on the grid's empty space. Returns the
    /// cell a double-click landed on.
    pub(crate) fn update(
        &mut self,
        ui: &egui::Ui,
        found: &Found,
        empty_clicked: bool,
    ) -> Option<CellKey> {
        let now = ui.input(|input| input.time);
        self.follow(found.hovered.as_ref(), now);
        let mut double_clicked = None;
        if let Some(cell) = &found.clicked {
            // egui reports the double-click with the click of its second press, so that
            // click must not pin or unpin again.
            if found.double_clicked {
                self.pinned = self.pinned_before_click.clone();
                self.mute(cell);
                double_clicked = Some(cell.clone());
            } else {
                self.pinned_before_click = self.pinned.clone();
                if self.pinned.as_ref() == Some(cell) {
                    self.pinned = None;
                    self.mute(cell);
                } else {
                    self.pinned = Some(cell.clone());
                }
            }
        } else if empty_clicked {
            self.pinned = None;
        }
        let open = self.pinned.is_some() || self.hover_shown(now);
        if open && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            self.pinned = None;
            if let Some(under) = &mut self.under {
                under.opens_at = None;
            }
        }
        if self.pinned.is_none()
            && let Some(opens_at) = self.under.as_ref().and_then(|under| under.opens_at)
            && opens_at > now
        {
            ui.ctx().request_repaint_after_secs((opens_at - now) as f32);
        }
        double_clicked
    }

    /// The pointer is on `hovered`, or on no cell. A cell it comes onto opens its window
    /// after the hover delay, or at once while a window is open or has just closed; on no
    /// cell, the hover window closes.
    fn follow(&mut self, hovered: Option<&CellKey>, now: f64) {
        match hovered {
            Some(cell) if self.under.as_ref().is_some_and(|under| under.cell == *cell) => {}
            Some(cell) => {
                let at_once = self.hover_shown(now)
                    || self
                        .hover_closed_at
                        .is_some_and(|closed| now - closed < TOOLTIP_GRACE);
                let opens_at = match (self.pinned.is_some(), at_once) {
                    (true, _) => None,
                    (false, true) => Some(now),
                    (false, false) => Some(now + HOVER_DELAY),
                };
                self.under = Some(Under {
                    cell: cell.clone(),
                    opens_at,
                });
            }
            None => {
                if self.hover_shown(now) {
                    self.hover_closed_at = Some(now);
                }
                self.under = None;
            }
        }
    }

    /// Whether a hover, not a pin, has the window open.
    fn hover_shown(&self, now: f64) -> bool {
        self.pinned.is_none()
            && self
                .under
                .as_ref()
                .and_then(|under| under.opens_at)
                .is_some_and(|opens_at| opens_at <= now)
    }

    /// `cell` opens nothing on hover until the pointer leaves it.
    fn mute(&mut self, cell: &CellKey) {
        self.under = Some(Under {
            cell: cell.clone(),
            opens_at: None,
        });
    }

    /// Paints the pin badge on the pinned cell, when `found` saw it drawn pinned. The rows
    /// start at `rows_top` on screen, under the title row, which covers the badge of a cell
    /// scrolled under it as it covers the cell.
    pub(crate) fn paint_pin(&self, ui: &egui::Ui, found: &Found, rows_top: f32) {
        let Some((pinned, rect)) = &found.pinned else {
            return;
        };
        if self.pinned.as_ref() != Some(pinned) {
            return;
        }
        // Over a cell's top right corner, half outside it; inside a label's, whose corner
        // touches the ID cell.
        let centre = match pinned.place {
            Place::Label => pos2(rect.right() - 8.0, rect.top() + 8.0),
            Place::Position { .. } => pos2(rect.right() - BADGE_INSET, rect.top() + BADGE_INSET),
        };
        // The rows' area starts as high as a badge on the first row reaches, so that badge
        // is drawn whole while nothing is scrolled.
        let view = ui.clip_rect();
        let rows = Rect::from_min_max(
            pos2(view.left(), rows_top - (BADGE_RADIUS - BADGE_INSET)),
            view.max,
        );
        let disc = Rect::from_center_size(centre, Vec2::splat(2.0 * BADGE_RADIUS));
        if !rows.intersects(disc) {
            return;
        }
        let visuals = ui.visuals();
        let painter = ui.painter_at(rows);
        painter.circle_filled(centre, BADGE_RADIUS, visuals.panel_fill);
        painter.text(
            centre,
            Align2::CENTER_CENTER,
            PIN,
            FontId::proportional(BADGE_GLYPH),
            visuals.strong_text_color(),
        );
    }

    /// The window over the grid drawn in `grid`, on the pinned cell or the one a hover
    /// opened, if any, from the grid's `contents`, its top box from `pictures`.
    pub(crate) fn window(
        &mut self,
        ui: &egui::Ui,
        id: egui::Id,
        grid: Rect,
        contents: &Contents,
        pictures: &mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
    ) -> GridResponse {
        let Contents {
            columns,
            rows,
            checking,
            catalog,
        } = *contents;
        let now = ui.input(|input| input.time);
        let mut response = GridResponse::default();
        let shown = match (&self.pinned, &self.under) {
            (Some(pinned), _) => Some(pinned.clone()),
            (None, Some(under)) if self.hover_shown(now) => Some(under.cell.clone()),
            (None, _) => None,
        };
        let Some(key) = shown else {
            return response;
        };
        let Some((row, cell)) = find(rows, &key) else {
            // The rows were made again without this cell.
            self.pinned = None;
            return response;
        };
        let pinned = self.pinned.is_some();
        let default = pos2(grid.right() - INSET - WIDTH, grid.top() + INSET);
        let at = self.position.unwrap_or(default);
        let copied = self
            .copied
            .as_ref()
            .is_some_and(|(copied, until)| *copied == row.key && now < *until);
        let mut unpin = false;
        let mut dragged = None;
        let mut copy = false;
        let area = egui::Area::new(id.with("detail"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .interactable(pinned)
            .show(ui.ctx(), |ui| {
                let visuals = ui.visuals().clone();
                egui::Frame::window(ui.style())
                    .fill(visuals.panel_fill)
                    .inner_margin(0)
                    .show(ui, |ui| {
                        ui.set_width(WIDTH);
                        let title = title(columns, row, cell, key.place);
                        let bar = title_bar(ui, &title, pinned);
                        unpin = bar.unpin;
                        // The frame a drag starts on carries the pointer's way onto the bar.
                        if bar.drag.dragged() && !bar.drag.drag_started() {
                            dragged = Some(bar.drag.drag_delta());
                        }
                        egui::Frame::NONE.inner_margin(BODY_MARGIN).show(ui, |ui| {
                            let shown_state = if checking {
                                (CellState::Checking, None)
                            } else {
                                (cell.state, cell.outline)
                            };
                            body(ui, cell, shown_state, catalog, pictures, &mut response);
                            let copies = key.place == Place::Label && !row.log_lines.is_empty();
                            let copy_button = copies.then_some(copied);
                            copy = buttons(ui, cell, copy_button, &mut response);
                        });
                    });
            });
        if unpin {
            self.pinned = None;
        }
        // From where the window stands, kept on the screen, not from where it was asked to.
        if let Some(delta) = dragged {
            self.position = Some(area.response.rect.min + delta);
        }
        if copy {
            ui.ctx().copy_text(row.log_lines.join("\n"));
            self.copied = Some((row.key.clone(), now + COPIED_FOR));
        }
        if let Some((_, until)) = &self.copied
            && *until > now
        {
            ui.ctx().request_repaint_after_secs((until - now) as f32);
        }
        response
    }
}

/// What a double-click on the cell `key` names in `rows` opens.
pub(crate) fn open_of(rows: &[Row], key: &CellKey) -> Option<Open> {
    find(rows, key).and_then(|(_, cell)| cell.open.clone())
}

/// The cell `key` names in `rows`, with its row; `None` when no row has it.
fn find<'a>(rows: &'a [Row], key: &CellKey) -> Option<(&'a Row, &'a Cell)> {
    let row = rows.iter().find(|row| row.key == key.row)?;
    let cell = match key.place {
        Place::Label => &row.label,
        Place::Position { line, group, index } => {
            let RowBody::Lines(lines) = &row.body else {
                return None;
            };
            match lines.get(line)?.groups.get(group)?.get(index)? {
                Position::Cell(cell) => cell,
                Position::Empty => return None,
            }
        }
    };
    Some((row, cell))
}

/// The window's title: the team for its label; for another cell the team and the cell's
/// text, or, alone in its column group, the group's title (`/a/ portraits`).
fn title(columns: &Columns, row: &Row, cell: &Cell, place: Place) -> String {
    let team = &row.label.text;
    match place {
        Place::Label => team.clone(),
        Place::Position { group, .. } => match columns.groups.get(group) {
            Some(alone) if alone.columns == 1 => {
                format!("{team} {}", alone.title.to_lowercase())
            }
            Some(_) | None => format!("{team} {}", cell.text),
        },
    }
}

/// What the title bar reports.
struct Bar {
    /// Its area: it drags the window while pinned.
    drag: egui::Response,
    /// The pin was clicked.
    unpin: bool,
}

/// The title bar: the title, then the pin at the right while pinned, a line under them.
fn title_bar(ui: &mut egui::Ui, title: &str, pinned: bool) -> Bar {
    // The bar is allocated before its title and pin, so the pin, added after it, takes the
    // pin's clicks while the rest of the bar takes the drags.
    let sense = if pinned {
        Sense::drag()
    } else {
        Sense::hover()
    };
    let (bar, drag) = ui.allocate_exact_size(vec2(WIDTH, BAR_HEIGHT), sense);
    let pin_width = if pinned { BAR_HEIGHT } else { 0.0 };
    let title_rect = Rect::from_min_max(
        bar.min + vec2(f32::from(BODY_MARGIN), 0.0),
        bar.max - vec2(pin_width + 4.0, 0.0),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(title_rect)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.add(
                egui::Label::new(RichText::new(title).strong())
                    .selectable(false)
                    .truncate(),
            )
        },
    );
    let mut unpin = false;
    if pinned {
        let pin = Rect::from_min_size(
            pos2(bar.right() - pin_width - 4.0, bar.top()),
            vec2(pin_width, BAR_HEIGHT),
        );
        let button = egui::Button::new(RichText::new(PIN).size(BAR_GLYPH)).frame(false);
        unpin = ui.put(pin, button).on_hover_text("Unpin").clicked();
    }
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter().hline(bar.x_range(), bar.bottom(), stroke);
    Bar { drag, unpin }
}

/// The window's body above its buttons: the top box with the cell's pictures, the place,
/// the state's name, then the findings worst first, or "No findings.". A finding's code
/// clicked lands in `response`.
fn body(
    ui: &mut egui::Ui,
    cell: &Cell,
    (state, outline): (CellState, Option<Outline>),
    catalog: &[CatalogRow],
    pictures: &mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
    response: &mut GridResponse,
) {
    let visuals = ui.visuals().clone();
    let (top_box, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), BOX_HEIGHT), Sense::hover());
    ui.painter()
        .rect_filled(top_box, 2.0, visuals.extreme_bg_color);
    let painter = ui.painter_at(top_box);
    paint_top_box(&painter, &cell.top, pictures, visuals.weak_text_color());
    ui.label(RichText::new(&cell.place).monospace());
    ui.label(RichText::new(state_name(state, outline)).color(visuals.weak_text_color()));
    let mut messages: Vec<&Message> = cell.messages.iter().collect();
    messages.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.code.code.cmp(&b.code.code))
    });
    if messages.is_empty() {
        ui.add_space(FINDING_GAP);
        ui.label(RichText::new("No findings.").color(visuals.weak_text_color()));
    }
    for message in messages {
        ui.add_space(FINDING_GAP);
        let colour = match message.severity {
            Severity::Fatal | Severity::Error => visuals.error_fg_color,
            Severity::Warning => visuals.warn_fg_color,
            Severity::Info => visuals.text_color(),
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(message.severity.name()).color(colour));
            let code = ui
                .link(RichText::new(message.code.code.as_ref()).monospace())
                .on_hover_text("Open the help at this message");
            if code.clicked() {
                response.help = Some(message.code.clone());
            }
        });
        if let Some(about) = about(&message.scope) {
            ui.label(RichText::new(about).monospace().color(colour));
        }
        if !message.context.is_empty() {
            let fields: Vec<String> = message
                .context
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect();
            ui.label(RichText::new(fields.join(", ")).monospace().color(colour));
        }
        if let Some(row) = catalog.iter().find(|row| row.code == message.code.code) {
            ui.label(code_spans(ui, row.when, visuals.text_color()));
            ui.label(code_spans(ui, row.effect, visuals.text_color()));
        }
    }
    ui.add_space(FINDING_GAP);
}

/// How a picture in the top box is fitted in its area: `Shrink` draws it at most at its
/// own size, shrunk to fit; `Fill` as large as fits.
enum Fit {
    /// At most at its own size: a small picture is not enlarged, which would blur it.
    Shrink,
    /// As large as the area fits, its shape kept.
    Fill,
}

/// Draws what `top` holds in the top box `painter` is clipped to: each picture, asked of
/// `pictures`, straight on the box's background (a cut-out portrait or a crest shows its own
/// shape), and "No logo" in `weak` where a missing logo would be.
fn paint_top_box(
    painter: &egui::Painter,
    top: &TopBox,
    pictures: &mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
    weak: Color32,
) {
    let area = painter.clip_rect();
    let middle = Rect::from_center_size(area.center(), PICTURE_MAX.min(area.size()));
    match top {
        TopBox::Empty => {}
        TopBox::Picture(file) => paint_picture(painter, pictures, file, middle, Fit::Shrink),
        TopBox::Logo(logo) => {
            paint_logo(painter, pictures, logo.as_ref(), middle, weak, Fit::Shrink)
        }
        TopBox::Player { portrait, logo } => {
            let top = area.bottom() - TILE_GAP - CORNER.y;
            let left = Rect::from_min_size(pos2(area.left() + TILE_GAP, top), CORNER);
            let right_x = area.right() - TILE_GAP - CORNER.x;
            let right = Rect::from_min_size(pos2(right_x, top), CORNER);
            if let Some(portrait) = portrait {
                paint_picture(painter, pictures, portrait, left, Fit::Fill);
            }
            paint_logo(painter, pictures, logo.as_ref(), right, weak, Fit::Fill);
        }
        TopBox::Portraits { portraits, logo } => {
            let last = TILES_ACROSS * TILES_DOWN - 1;
            for (index, portrait) in portraits.iter().enumerate().take(last) {
                if let Some(portrait) = portrait {
                    paint_picture(painter, pictures, portrait, tile(area, index), Fit::Fill);
                }
            }
            paint_logo(
                painter,
                pictures,
                logo.as_ref(),
                tile(area, last),
                weak,
                Fit::Fill,
            );
        }
    }
}

/// The portraits grid's tile `index` in `area`, counted across then down.
fn tile(area: Rect, index: usize) -> Rect {
    let gaps = vec2((TILES_ACROSS - 1) as f32, (TILES_DOWN - 1) as f32) * TILE_GAP;
    let tiles = vec2(TILES_ACROSS as f32, TILES_DOWN as f32);
    let size = (area.size() - Vec2::splat(2.0 * TILE_GAP) - gaps) / tiles;
    let at = vec2((index % TILES_ACROSS) as f32, (index / TILES_ACROSS) as f32);
    let min = area.min + Vec2::splat(TILE_GAP) + at * (size + Vec2::splat(TILE_GAP));
    Rect::from_min_size(min, size)
}

/// The logo fitted in `area` as `fit` asks, or "No logo" in `weak` at its centre when
/// there is none.
fn paint_logo(
    painter: &egui::Painter,
    pictures: &mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
    logo: Option<&PictureFile>,
    area: Rect,
    weak: Color32,
    fit: Fit,
) {
    match logo {
        Some(logo) => paint_picture(painter, pictures, logo, area, fit),
        None => {
            painter.text(
                area.center(),
                Align2::CENTER_CENTER,
                "No logo",
                FontId::proportional(NO_LOGO_SIZE),
                weak,
            );
        }
    }
}

/// `file`'s picture fitted in `area` as `fit` asks, its shape kept, centred; nothing
/// while it loads or when it cannot be read.
fn paint_picture(
    painter: &egui::Painter,
    pictures: &mut dyn FnMut(&PictureFile) -> Option<TextureHandle>,
    file: &PictureFile,
    area: Rect,
    fit: Fit,
) {
    let Some(texture) = pictures(file) else {
        return;
    };
    let size = texture.size_vec2();
    let scale = (area.width() / size.x).min(area.height() / size.y);
    let scale = match fit {
        Fit::Shrink => scale.min(1.0),
        Fit::Fill => scale,
    };
    let rect = Rect::from_center_size(area.center(), size * scale);
    let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(texture.id(), rect, whole, NO_TINT);
}

/// The window's buttons: Open folder when `cell` has a folder to open, its click landing in
/// `response`, then Copy messages when `copy` is given, reading "Copied" when it is `true`.
/// Returns whether Copy messages was clicked.
fn buttons(
    ui: &mut egui::Ui,
    cell: &Cell,
    copy: Option<bool>,
    response: &mut GridResponse,
) -> bool {
    ui.horizontal(|ui| {
        if let Some(open) = &cell.open {
            let button = ui.button("Open folder");
            let button = match open {
                Open::Folder(_) => button,
                Open::Select(_) => button.on_hover_text(
                    "The export is an archive: the folder holding it opens, with the archive \
                     selected",
                ),
            };
            if button.clicked() {
                response.open = Some(open.clone());
            }
        }
        let Some(copied) = copy else {
            return false;
        };
        let text = if copied { "Copied" } else { "Copy messages" };
        ui.button(text)
            .on_hover_text("Puts this export's log lines on the clipboard, to send to its member")
            .clicked()
    })
    .inner
}

/// The file or folder a finding is about, as the log names it; `None` for the export or the
/// run as a whole.
fn about(scope: &Scope) -> Option<String> {
    match scope {
        Scope::Run | Scope::Export { .. } => None,
        Scope::Folder { path, .. } | Scope::File { path, .. } => Some(path.as_str().to_owned()),
        Scope::RosterEntry { file, line, .. } => Some(format!("{} line {line}", file.as_str())),
    }
}

/// A catalog text in `colour`, its `code` spans in the monospace font and their backticks
/// dropped: the only Markdown the catalog's texts use.
fn code_spans(ui: &egui::Ui, text: &str, colour: Color32) -> LayoutJob {
    let body = TextStyle::Body.resolve(ui.style());
    let code = TextStyle::Monospace.resolve(ui.style());
    let mut job = LayoutJob::default();
    for (index, part) in text.split('`').enumerate() {
        let font = if index % 2 == 1 { &code } else { &body };
        job.append(part, 0.0, TextFormat::simple(font.clone(), colour));
    }
    job
}

#[cfg(test)]
mod tests;
