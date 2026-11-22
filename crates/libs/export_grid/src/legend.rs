//! The legend bar above the grid (the draft's `.tc-legend-bar`): what each cell colour means,
//! folding to its title.

use egui::{Rect, RichText, Sense, TextStyle, pos2, vec2};

use crate::cells::{CellState, Outline, look, paint};

/// One state the legend names: a swatch in its colours, then its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegendEntry {
    /// The swatch's state.
    pub state: CellState,
    /// The swatch's ring.
    pub outline: Option<Outline>,
    /// The state's name; a `\n` breaks it in two lines.
    pub name: &'static str,
}

/// A swatch's side.
const SWATCH: f32 = 14.0;
/// The space between a swatch and its name.
const SWATCH_GAP: f32 = 4.0;
/// The space between the legend's two rows, and between its columns.
const ROW_GAP: f32 = 4.0;
const COLUMN_GAP: f32 = 16.0;
/// The legend's title's size, larger than its names.
const TITLE_SIZE: f32 = 14.0;

/// The legend bar: its title, then `columns` of entries, each column a pair one over the
/// other or one entry centred over both rows, and a line under the bar. A click on the title
/// or its chevron folds the legend to its title, or opens it again; the fold lasts the
/// session. `id_salt` keeps the fold apart from another legend's.
pub fn legend(ui: &mut egui::Ui, id_salt: impl egui::AsIdSalt, columns: &[&[LegendEntry]]) {
    let id = ui.make_persistent_id(id_salt);
    let mut shut = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    let bar = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let tip = if shut {
                    "Show the legend"
                } else {
                    "Hide the legend"
                };
                let chevron = if shut { "▸" } else { "▾" };
                let chevron = ui
                    .add(egui::Button::new(chevron).frame(false))
                    .on_hover_text(tip);
                let title = RichText::new("Legend").size(TITLE_SIZE).weak();
                let title = ui
                    .add(egui::Label::new(title).sense(Sense::click()))
                    .on_hover_text(tip);
                if chevron.clicked() || title.clicked() {
                    shut = !shut;
                }
                if !shut {
                    ui.add_space(COLUMN_GAP - 2.0);
                    entries(ui, id, columns);
                }
            });
        });
    ui.data_mut(|data| data.insert_temp(id, shut));
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.painter()
        .hline(ui.max_rect().x_range(), bar.response.rect.bottom(), stroke);
}

/// The legend's columns, left to right.
fn entries(ui: &mut egui::Ui, id: egui::Id, columns: &[&[LegendEntry]]) {
    let font = TextStyle::Body.resolve(ui.style());
    let colour = ui.visuals().weak_text_color();
    let row = ui.text_style_height(&TextStyle::Body).max(SWATCH);
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            ui.add_space(COLUMN_GAP - 2.0);
        }
        let names: Vec<_> = column
            .iter()
            .map(|entry| {
                ui.painter()
                    .layout_no_wrap(entry.name.to_owned(), font.clone(), colour)
            })
            .collect();
        let width = names
            .iter()
            .map(|name| SWATCH + SWATCH_GAP + name.size().x)
            .fold(0.0, f32::max);
        let (rect, _) = ui.allocate_exact_size(vec2(width, 2.0 * row + ROW_GAP), Sense::hover());
        for (place, (entry, name)) in column.iter().zip(names).enumerate() {
            let centre = if column.len() == 1 {
                rect.center().y
            } else {
                rect.top() + row / 2.0 + (row + ROW_GAP) * place as f32
            };
            let swatch = Rect::from_min_size(
                pos2(rect.left(), centre - SWATCH / 2.0),
                vec2(SWATCH, SWATCH),
            );
            let look = look(entry.state, entry.outline, ui.visuals());
            paint(ui.painter(), &look, swatch);
            let at = pos2(swatch.right() + SWATCH_GAP, centre - name.size().y / 2.0);
            let text = Rect::from_min_size(at, name.size());
            ui.painter().galley(at, name, colour);
            let response = ui.interact(text, id.with((index, place)), Sense::hover());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, entry.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Headless, press, release};

    /// The Export upgrader's pairs: two columns of two states.
    const COLUMNS: [&[LegendEntry]; 2] = [
        &[
            LegendEntry {
                state: CellState::Unchecked,
                outline: None,
                name: "Unchecked",
            },
            LegendEntry {
                state: CellState::Checking,
                outline: None,
                name: "Checking",
            },
        ],
        &[LegendEntry {
            state: CellState::FullOk,
            outline: Some(Outline::Warning),
            name: "Warning",
        }],
    ];

    #[test]
    fn the_legend_names_its_states_and_folds_to_its_title_and_back() {
        let mut window = Headless::new();
        let mut bar = |events| window.frame(events, |ui| legend(ui, "legend", &COLUMNS));
        let drawn = bar(Vec::new());
        let (unchecked, checking) = (drawn.one("Unchecked"), drawn.one("Checking"));
        assert!(
            checking.top() > unchecked.bottom(),
            "a pair, one over the other"
        );
        let warning = drawn.one("Warning");
        assert!(warning.left() > unchecked.right(), "the next column");
        let middle = (unchecked.top() + checking.bottom()) / 2.0;
        assert!(
            (warning.center().y - middle).abs() < 1.0,
            "alone, centred over both rows"
        );
        let chevron = drawn.one("▾").center();
        bar(press(chevron));
        bar(release(chevron));
        let drawn = bar(Vec::new());
        assert!(drawn.all("Unchecked").is_empty(), "folded to its title");
        // A label's node and its text run's both read it.
        assert!(!drawn.all("Legend").is_empty(), "the title stays");
        let chevron = drawn.one("▸").center();
        bar(press(chevron));
        bar(release(chevron));
        bar(Vec::new()).one("Unchecked");
    }
}
