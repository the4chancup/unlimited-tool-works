//! The rows a combo's open list shares: `combo_item` (`selectable_value`)
//! and `combo_label` (`selectable_label`), each reaching half the item
//! spacing's gap up and down with its click area, so a click in the gap
//! picks the nearer row. Without it the popup's `Area` widget, which
//! senses a click anywhere in it, takes a click landing between rows,
//! and the list closes on it with nothing picked.

/// `selectable_label` with half the item spacing's gap above and below it
/// as click reach (`interact` on each strip, unioned in), so the list
/// reads as rows that meet. The strips stop at the row's edges: a widget
/// laid over the row would take its hover (egui hovers only the widget a
/// click would hit, the last one registered) and the row would lose its
/// highlight. The returned `rect` stays the painted row's.
pub fn combo_label(
    ui: &mut egui::Ui,
    checked: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let row = ui.selectable_label(checked, text);
    let (rect, half_gap) = (row.rect, ui.spacing().item_spacing.y / 2.0);
    let above = egui::Rect::from_x_y_ranges(rect.x_range(), rect.top() - half_gap..=rect.top());
    let below =
        egui::Rect::from_x_y_ranges(rect.x_range(), rect.bottom()..=rect.bottom() + half_gap);
    let above = ui.interact(above, row.id.with("gap above"), egui::Sense::click());
    let below = ui.interact(below, row.id.with("gap below"), egui::Sense::click());
    row.union(above).union(below).with_new_rect(rect)
}

/// `selectable_value` as `combo_label`: `*current` takes `item` on a
/// click in the row or in the gap beside it.
pub fn combo_item<Selection>(
    ui: &mut egui::Ui,
    current: &mut Selection,
    item: Selection,
    label: impl Into<egui::WidgetText>,
) -> egui::Response
where
    Selection: PartialEq + Clone,
{
    let mut response = combo_label(ui, *current == item, label);
    if response.clicked() && *current != item {
        *current = item;
        response.mark_changed();
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The five items of the test combo.
    const ITEMS: [&str; 5] = ["First", "Second", "Third", "Fourth", "Fifth"];

    /// A headless five-item combo (the wheel tests' window's own): the
    /// popup's rows' rects recorded each frame it is open, `scroll` capping
    /// the popup's height so its scroll area drags on a touch machine.
    struct Window {
        ctx: egui::Context,
        selected: usize,
        rows: Vec<egui::Rect>,
        ids: Vec<egui::Id>,
        button: egui::Rect,
        scroll: bool,
    }

    impl Window {
        fn new(scroll: bool) -> Window {
            Window {
                ctx: egui::Context::default(),
                selected: 1,
                rows: Vec::new(),
                ids: Vec::new(),
                button: egui::Rect::NOTHING,
                scroll,
            }
        }

        /// One frame with `events` as its input.
        fn frame(&mut self, events: Vec<egui::Event>) {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 400.0),
                )),
                events,
                ..Default::default()
            };
            let mut selected = self.selected;
            let mut rows = Vec::new();
            let mut ids = Vec::new();
            let mut button = egui::Rect::NOTHING;
            let scroll = self.scroll;
            let output = self.ctx.run_ui(input, |ui| {
                if scroll {
                    ui.spacing_mut().combo_height = 40.0;
                }
                button = egui::ComboBox::from_id_salt("items")
                    .selected_text(ITEMS[selected])
                    .show_ui(ui, |ui| {
                        for (index, item) in ITEMS.iter().enumerate() {
                            let response = combo_item(ui, &mut selected, index, *item);
                            rows.push(response.rect);
                            ids.push(response.id);
                        }
                    })
                    .response
                    .rect;
            });
            output.drop_without_applying_deltas();
            self.selected = selected;
            self.rows = rows;
            self.ids = ids;
            self.button = button;
        }

        /// A press and a release at `pos`, one frame each.
        fn click(&mut self, pos: egui::Pos2) {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            self.frame(vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
    }

    /// A click in the gap between the second and third rows, nearer
    /// `third` (`0.5` above it) or `second` (`0.5` below it).
    fn click_the_gap(window: &mut Window, nearer_third: bool) {
        let (second, third) = (window.rows[1], window.rows[2]);
        assert!(
            third.min.y - second.max.y > 1.0,
            "premise: a gap between the rows ({second:?}, {third:?})"
        );
        let y = if nearer_third {
            third.min.y - 0.5
        } else {
            second.max.y + 0.5
        };
        window.click(egui::pos2(third.center().x, y));
    }

    /// Open the combo and assert the rows are drawn.
    fn open(window: &mut Window) {
        window.frame(Vec::new());
        window.click(window.button.center());
        window.frame(Vec::new());
        assert_eq!(window.rows.len(), 5, "premise: the popup's rows are drawn");
    }

    // A click in the gap between two rows picks the nearer row, and the
    // list closes (the popup's `Area` is the widget a gap click would
    // otherwise hit).
    // ST-COMBO-01
    #[test]
    fn a_click_in_the_gap_picks_the_nearer_row() {
        let mut window = Window::new(false);
        open(&mut window);
        click_the_gap(&mut window, true);
        assert_eq!(window.selected, 2, "the nearer (third) row picked");
        window.frame(Vec::new());
        assert_eq!(window.rows.len(), 0, "the list closed");

        open(&mut window);
        click_the_gap(&mut window, false);
        assert_eq!(window.selected, 1, "the nearer (second) row picked");
    }

    // A row under the pointer is the hovered widget, so it still paints
    // its highlight: the gap's click reach does not cover the row.
    #[test]
    fn a_row_under_the_pointer_is_hovered() {
        let mut window = Window::new(false);
        open(&mut window);
        window.frame(vec![egui::Event::PointerMoved(window.rows[2].center())]);
        window.frame(Vec::new());
        let row = window.ctx.read_response(window.ids[2]);
        assert!(
            row.is_some_and(|row| row.hovered()),
            "the third row hovered"
        );
    }

    // The same, on a scrolling list with a touch seen: the scroll
    // background's drag sense, which a touch machine turns on, does not
    // take the click either.
    // ST-COMBO-01
    #[test]
    fn a_scrolling_list_on_a_touch_machine_picks_the_nearer_row_too() {
        let mut window = Window::new(true);
        // One touch event and every frame's scroll background is a drag
        // widget from then on, as on a machine with a touch screen.
        window.frame(vec![egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(1),
            phase: egui::TouchPhase::Start,
            pos: egui::pos2(250.0, 50.0),
            force: None,
        }]);
        open(&mut window);
        let (second, third) = (window.rows[1], window.rows[2]);
        assert!(
            third.min.y - second.max.y > 1.0,
            "premise: a gap between the rows ({second:?}, {third:?})"
        );
        window.click(egui::pos2(third.center().x, third.min.y - 0.5));
        assert_eq!(window.selected, 2, "the nearer (third) row picked");
    }
}
