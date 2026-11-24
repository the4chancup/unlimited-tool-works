//! A grid cell: what it shows, and the colours its state paints it in (the draft's
//! `COLOURS`, `OUTLINES` and `labelColour`).

use std::path::PathBuf;

use egui::Color32;
use studio_core::Message;
use studio_core::theme::palette;

/// What a cell's fill says about its folder or file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    /// Not checked yet: the darkest fill.
    Unchecked,
    /// Being checked: the strong text colour, the panel's inverse.
    Checking,
    /// Checked from an archive's listing, its contents not read: blue.
    PartialOk,
    /// Checked through its files: purple.
    FullOk,
    /// Compiled: green.
    Done,
    /// Compiled with warnings: amber.
    DoneWithWarnings,
    /// Compiled with errors: red.
    DoneWithErrors,
    /// Its export will not compile: no fill, a dashed red edge and red text.
    Blocked,
}

/// The ring a check's worst finding draws inside a cell, over its fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outline {
    /// A Warning: amber.
    Warning,
    /// An Error: red.
    Error,
}

/// How much room a cell takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// One square position.
    Square,
    /// A wide cell, for a count or a word (`23/23`, `Note`): one column of a group of wide
    /// columns, or two columns of a group of square ones.
    Wide,
}

/// A kit's menu icon as its cell shows it: the icon's pattern drawn in the kit's two menu
/// colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitIcon {
    /// The icon's number, 0 to 23 (`color_tools::icon` draws each).
    pub number: u8,
    /// The kit's two menu colours, first then second.
    pub colors: [Color32; 2],
}

/// One cell: its text, its state, its ring, the findings it stands for, and the folder its
/// detail window names and opens.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// What the cell shows: a slot number, a kit's name, a count or a word.
    pub text: String,
    /// What its fill says.
    pub state: CellState,
    /// The ring of its worst Warning or Error; `None` without one.
    pub outline: Option<Outline>,
    /// The findings the cell stands for, in the order the tool found them.
    pub messages: Vec<Message>,
    /// How much room it takes.
    pub width: Width,
    /// Where the cell's files are, as its detail window prints it under the top box: a
    /// folder inside the export (`Players/05 - Bob`), or the export's own name for its label.
    pub place: String,
    /// What a double-click on the cell, or its window's Open folder, opens; `None` for a cell
    /// with no one folder to open, whose window then has no Open folder.
    pub open: Option<Open>,
    /// The pictures its detail window shows in the box at its top.
    pub top: TopBox,
    /// A kit cell's menu icon once its colours are known; `None` on every other cell.
    pub kit_icon: Option<KitIcon>,
}

impl Cell {
    /// A square cell showing `text` in `state`, with no ring, no finding, no folder and
    /// nothing in its window's top box.
    pub fn new(text: impl Into<String>, state: CellState) -> Cell {
        Cell {
            text: text.into(),
            state,
            outline: None,
            messages: Vec::new(),
            width: Width::Square,
            place: String::new(),
            open: None,
            top: TopBox::Empty,
            kit_icon: None,
        }
    }
}

/// A picture a cell's top box shows: a file of an export, which the tool reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PictureFile {
    /// The export's folder, or its `.zip`.
    pub export: PathBuf,
    /// The file inside it (an entry, in a `.zip`), `/`-separated as the export's listing
    /// spells it.
    pub path: String,
}

/// What the box at the top of a cell's detail window holds. A logo `None` reads "No logo"
/// where the logo would be.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum TopBox {
    /// Nothing: the box stays empty.
    #[default]
    Empty,
    /// A kit's main texture, fitted inside the box.
    Picture(PictureFile),
    /// A team's logo, or a referee export's marker, fitted inside the box.
    Logo(Option<PictureFile>),
    /// A player: his portrait in the bottom-left corner, the logo in the bottom-right one.
    Player {
        /// His portrait; `None` leaves the corner empty.
        portrait: Option<PictureFile>,
        /// The export's logo.
        logo: Option<PictureFile>,
    },
    /// The portraits cell: a 6 × 4 grid, the logo in the last tile.
    Portraits {
        /// The players' portraits in slot order; `None` leaves the tile empty.
        portraits: Vec<Option<PictureFile>>,
        /// The export's logo.
        logo: Option<PictureFile>,
    },
}

/// What the file manager opens for a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Open {
    /// This folder.
    Folder(PathBuf),
    /// This file shown in its folder ([`studio_core::files::show_in_folder`]): a cell of
    /// an export packed in an archive, which has no folder of its own on disk.
    Select(PathBuf),
}

/// The name the detail window gives a cell drawn in `state` with `outline`: a check's ring
/// names its severity, else the state is named as the legend names it.
pub(crate) fn state_name(state: CellState, outline: Option<Outline>) -> &'static str {
    match (state, outline) {
        (CellState::PartialOk | CellState::FullOk, Some(Outline::Error)) => "Error",
        (CellState::PartialOk | CellState::FullOk, Some(Outline::Warning)) => "Warning",
        (CellState::Unchecked, _) => "Unchecked",
        (CellState::Checking, _) => "Checking",
        (CellState::PartialOk, None) => "Shallow check done (archive)",
        (CellState::FullOk, None) => "Checked",
        (CellState::Done, _) => "Done",
        (CellState::DoneWithWarnings, _) => "Done with warnings",
        (CellState::DoneWithErrors, _) => "Done with errors",
        (CellState::Blocked, _) => "Blocked",
    }
}

/// One position of a line: empty (a faint square, so the columns stay in step), or a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    /// Nothing there: a faint square.
    Empty,
    /// A cell, boxed: a cell's pictures make it several times larger than an empty position.
    Cell(Box<Cell>),
}

/// How a cell is painted: its fill, its text's colour, its ring's colour, and whether its
/// edge is dashed (a blocked cell, which has no fill).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Look {
    pub(crate) fill: Color32,
    pub(crate) text: Color32,
    pub(crate) ring: Option<Color32>,
    pub(crate) dashed: bool,
}

/// The look of a cell in `state` with `outline`, in `visuals`' theme.
pub(crate) fn look(state: CellState, outline: Option<Outline>, visuals: &egui::Visuals) -> Look {
    let colours = palette(visuals);
    let fill = match state {
        CellState::Unchecked => visuals.extreme_bg_color,
        CellState::Checking => visuals.strong_text_color(),
        CellState::PartialOk => colours.grid_partial_ok,
        CellState::FullOk => colours.grid_full_ok,
        CellState::Done => colours.grid_done,
        CellState::DoneWithWarnings => visuals.warn_fg_color,
        CellState::DoneWithErrors => visuals.error_fg_color,
        CellState::Blocked => {
            return Look {
                fill: Color32::TRANSPARENT,
                text: colours.grid_blocked,
                ring: None,
                dashed: true,
            };
        }
    };
    Look {
        fill,
        text: text_colour(fill, visuals),
        ring: outline.map(|outline| match outline {
            Outline::Warning => visuals.warn_fg_color,
            Outline::Error => visuals.error_fg_color,
        }),
        dashed: false,
    }
}

/// Whether a cell drawn in `state` shows its kit's menu icon: the state part of the
/// draft's `swatch` test, which hides it on an unchecked, checking or blocked cell.
/// The test's `rowOff` half has no counterpart here: the grid draws no off row yet.
pub(crate) fn shows_icon(state: CellState) -> bool {
    !matches!(
        state,
        CellState::Unchecked | CellState::Checking | CellState::Blocked
    )
}

/// A cell's and a swatch's corner radius.
pub(crate) const RADIUS: f32 = 2.0;
/// The width of a Warning's or an Error's ring, inside the cell's edge.
const RING: f32 = 3.0;
/// A blocked cell's dashes, and the gaps between them.
const DASH: f32 = 3.0;
/// A kit swatch's height, and its inset from the cell's left, right and bottom edges
/// (the draft's `.tc-sw`).
const SWATCH_HEIGHT: f32 = 9.0;
const SWATCH_INSET: f32 = 2.0;
/// How far under its top edge a kit cell's label stands while its swatch shows (the
/// draft's `.tc-cell.kit`).
pub(crate) const KIT_LABEL_TOP: f32 = 3.0;

/// Paints `look` in `rect`: its fill, or a blocked cell's dashed edge, then its ring.
pub(crate) fn paint(painter: &egui::Painter, look: &Look, rect: egui::Rect) {
    paint_fill(painter, look, rect);
    paint_ring(painter, look, rect);
}

/// `look`'s fill in `rect`, or a blocked cell's dashed edge.
pub(crate) fn paint_fill(painter: &egui::Painter, look: &Look, rect: egui::Rect) {
    if look.dashed {
        let edge = rect.shrink(0.5);
        let path = [
            edge.left_top(),
            edge.right_top(),
            edge.right_bottom(),
            edge.left_bottom(),
            edge.left_top(),
        ];
        painter.extend(egui::Shape::dashed_line(
            &path,
            egui::Stroke::new(1.0, look.text),
            DASH,
            DASH,
        ));
    } else {
        painter.rect_filled(rect, RADIUS, look.fill);
    }
}

/// `look`'s ring in `rect`, inside its edge.
pub(crate) fn paint_ring(painter: &egui::Painter, look: &Look, rect: egui::Rect) {
    if let Some(ring) = look.ring {
        painter.rect_stroke(
            rect,
            RADIUS,
            egui::Stroke::new(RING, ring),
            egui::StrokeKind::Inside,
        );
    }
}

/// Paints `icon`'s pattern in the strip along `rect`'s foot (the draft's `.tc-sw`).
pub(crate) fn paint_swatch(painter: &egui::Painter, icon: KitIcon, rect: egui::Rect) {
    let strip = egui::Rect::from_min_max(
        egui::pos2(
            rect.left() + SWATCH_INSET,
            rect.bottom() - SWATCH_INSET - SWATCH_HEIGHT,
        ),
        egui::pos2(rect.right() - SWATCH_INSET, rect.bottom() - SWATCH_INSET),
    );
    painter.extend(color_tools::icon::pattern_shapes(
        strip,
        icon.number,
        icon.colors,
    ));
}

/// The text colour on `fill`: a soft dark grey on a light fill, a soft light grey on a dark
/// one, whatever the theme. The threshold is 125, not the usual 140: the error red weighs 130
/// and reads poorly under light text, and no other fill lies between.
fn text_colour(fill: Color32, visuals: &egui::Visuals) -> Color32 {
    let colours = palette(visuals);
    let weight =
        0.299 * f32::from(fill.r()) + 0.587 * f32::from(fill.g()) + 0.114 * f32::from(fill.b());
    if weight > 125.0 {
        colours.grid_text_dark
    } else {
        colours.grid_text_light
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Headless, dark, lines};

    #[test]
    fn a_warning_or_an_error_rings_the_checked_fill_and_a_blocked_cell_has_none() {
        let visuals = dark();
        let colours = palette(&visuals);
        for (state, outline, fill, ring) in [
            (CellState::FullOk, None, colours.grid_full_ok, None),
            (
                CellState::FullOk,
                Some(Outline::Warning),
                colours.grid_full_ok,
                Some(visuals.warn_fg_color),
            ),
            (
                CellState::PartialOk,
                Some(Outline::Error),
                colours.grid_partial_ok,
                Some(visuals.error_fg_color),
            ),
            (
                CellState::Blocked,
                Some(Outline::Error),
                Color32::TRANSPARENT,
                None,
            ),
        ] {
            let painted = look(state, outline, &visuals);
            assert_eq!(painted.fill, fill, "{state:?} {outline:?}");
            assert_eq!(painted.ring, ring, "{state:?} {outline:?}");
            assert_eq!(painted.dashed, state == CellState::Blocked);
        }
    }

    #[test]
    fn the_text_is_dark_on_the_light_fills_and_light_on_the_dark_ones() {
        let visuals = dark();
        let colours = palette(&visuals);
        for (state, text) in [
            (CellState::Unchecked, colours.grid_text_light),
            (CellState::Checking, colours.grid_text_dark),
            (CellState::PartialOk, colours.grid_text_light),
            (CellState::FullOk, colours.grid_text_light),
            (CellState::Done, colours.grid_text_light),
            (CellState::DoneWithWarnings, colours.grid_text_dark),
            // The error red weighs 130: over the 125 threshold, so dark.
            (CellState::DoneWithErrors, colours.grid_text_dark),
            (CellState::Blocked, colours.grid_blocked),
        ] {
            assert_eq!(look(state, None, &visuals).text, text, "{state:?}");
        }
    }

    // A blocked cell's edge is dashes, which paint no node: they are looked up in the
    // frame's shapes.
    #[test]
    fn a_blocked_cell_paints_dashes_on_its_edge_and_no_fill() {
        let visuals = dark();
        let look = look(CellState::Blocked, None, &visuals);
        let cell = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(26.0, 26.0));
        let drawn = Headless::new().frame(Vec::new(), |ui| {
            paint_fill(ui.painter(), &look, cell);
        });
        let edge = cell.shrink(0.5);
        let dashes = lines(&drawn, egui::Stroke::new(1.0, look.text));
        assert!(!dashes.is_empty(), "dashes in the look's text colour");
        for points in dashes {
            assert!(
                points[0].x == points[1].x || points[0].y == points[1].y,
                "a dash straight along the edge: {points:?}"
            );
            for point in points {
                assert!(
                    point.y == edge.top()
                        || point.y == edge.bottom()
                        || point.x == edge.left()
                        || point.x == edge.right(),
                    "{point:?} on the cell's edge {edge:?}"
                );
            }
        }
        assert_eq!(drawn.fill_at(cell), None, "a blocked cell has no fill");
    }

    #[test]
    fn state_name_names_each_state_and_a_ring_names_its_severity_on_the_ok_fills() {
        for (state, outline, name) in [
            (CellState::Unchecked, None, "Unchecked"),
            (CellState::Checking, None, "Checking"),
            (CellState::PartialOk, None, "Shallow check done (archive)"),
            (CellState::FullOk, None, "Checked"),
            (CellState::Done, None, "Done"),
            (CellState::DoneWithWarnings, None, "Done with warnings"),
            (CellState::DoneWithErrors, None, "Done with errors"),
            (CellState::Blocked, None, "Blocked"),
            (CellState::PartialOk, Some(Outline::Warning), "Warning"),
            (CellState::PartialOk, Some(Outline::Error), "Error"),
            (CellState::FullOk, Some(Outline::Warning), "Warning"),
            (CellState::FullOk, Some(Outline::Error), "Error"),
            // An outline changes the name of the two OK states alone.
            (CellState::Done, Some(Outline::Error), "Done"),
            (CellState::Blocked, Some(Outline::Error), "Blocked"),
        ] {
            assert_eq!(state_name(state, outline), name, "{state:?} {outline:?}");
        }
    }
}
