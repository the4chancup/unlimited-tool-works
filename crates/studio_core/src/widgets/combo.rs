//! The rows a combo's open list shares: `combo_item` (`selectable_value`)
//! and `combo_label` (`selectable_label`), and `combo_separator` between
//! groups of them. The rows meet: each is a control's height and the item
//! spacing's gap together (21 points in the suite's theme), with nothing
//! between two, so every click in the list lands on a row. egui lays its
//! rows a gap apart, and the popup's `Area` widget, which senses a click
//! anywhere in it, takes a click landing in that gap and closes the list
//! with nothing picked. The rows meet rather than each row reaching into
//! the gap with extra click area: a reach has to be given to every kind
//! of row one by one, and the toolbars' menus were left without it. With
//! no gap there is nothing to forget. A menu's entries meet the same way
//! (`menu_button`).

/// Makes the rows `ui` lays from now on meet: a control grows by the
/// item spacing's gap and the gap goes, so rows keep their pitch and a
/// row's text stays centred in it. A second call changes nothing, since
/// the gap is then 0.
pub(crate) fn rows_meet(spacing: &mut egui::style::Spacing) {
    spacing.interact_size.y += spacing.item_spacing.y;
    spacing.item_spacing.y = 0.0;
}

/// `selectable_label` as a row of a combo's open list: 21 points high,
/// meeting the rows above and below it. A row cannot change its
/// neighbours, so it sets the list's own `Ui` (`rows_meet`), as every row
/// does; a widget added to the list after a row therefore touches it too.
pub fn combo_label(
    ui: &mut egui::Ui,
    checked: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    rows_meet(ui.spacing_mut());
    ui.selectable_label(checked, text)
}

/// The height `combo_separator` takes in a combo's open list.
pub const COMBO_SEPARATOR_SPACING: f32 = 7.0;

/// The line between two groups of rows in a combo's open list, in the
/// middle of the `COMBO_SEPARATOR_SPACING` points it stands in (the
/// draft's `.combo-pop .sep`). egui's own separator stands in 6 and relies
/// on the item spacing's gap above and below it, which the list's rows no
/// longer leave.
pub fn combo_separator(ui: &mut egui::Ui) -> egui::Response {
    rows_meet(ui.spacing_mut());
    ui.add(egui::Separator::default().spacing(COMBO_SEPARATOR_SPACING))
}

/// `selectable_value` as `combo_label`: `*current` takes `item` on a
/// click in the row.
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
    use egui::accesskit::{self, Role, Toggled};

    use super::*;

    /// The five items of the test combo.
    const ITEMS: [&str; 5] = ["First", "Second", "Third", "Fourth", "Fifth"];

    /// A headless five-item combo (the wheel tests' window's own) in the
    /// suite's theme, whose 18-point controls make a row 21: the
    /// popup's rows' rects recorded each frame it is open, `scroll` capping
    /// the popup's height so its scroll area drags on a touch machine.
    struct Window {
        ctx: egui::Context,
        selected: usize,
        rows: Vec<egui::Rect>,
        ids: Vec<egui::Id>,
        button: egui::Rect,
        scroll: bool,
        nodes: Vec<accesskit::Node>,
    }

    impl Window {
        fn new(scroll: bool) -> Window {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            crate::theme::install(&ctx);
            Window {
                ctx,
                selected: 1,
                rows: Vec::new(),
                ids: Vec::new(),
                button: egui::Rect::NOTHING,
                scroll,
                nodes: Vec::new(),
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
            let mut output = self.ctx.run_ui(input, |ui| {
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
            let nodes = output
                .platform_output
                .accesskit_update
                .take()
                .expect("AccessKit is enabled on the context")
                .nodes
                .into_iter()
                .map(|(_, node)| node)
                .collect();
            // Nothing renders here, so the frame's texture deltas are dropped
            // on purpose.
            output.drop_without_applying_deltas();
            self.selected = selected;
            self.rows = rows;
            self.ids = ids;
            self.button = button;
            self.nodes = nodes;
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

    /// Open the combo and assert the rows are drawn.
    fn open(window: &mut Window) {
        window.frame(Vec::new());
        window.click(window.button.center());
        window.frame(Vec::new());
        assert_eq!(window.rows.len(), 5, "premise: the popup's rows are drawn");
    }

    /// Asserts each of the five rows is 21 high and starts where the row
    /// above it ends.
    fn assert_rows_meet(window: &Window) {
        for row in &window.rows {
            assert_eq!(row.height(), 21.0, "each row is 21 high: {row:?}");
        }
        for pair in window.rows.windows(2) {
            assert_eq!(pair[1].min.y, pair[0].max.y, "the rows meet: {pair:?}");
        }
    }

    /// A click on the line where the second and third rows meet, with
    /// neither current: one of the two is picked and the list closes.
    fn click_where_second_and_third_meet(window: &mut Window) {
        let (second, third) = (window.rows[1], window.rows[2]);
        assert_eq!(
            third.min.y, second.max.y,
            "premise: the rows meet ({second:?}, {third:?})"
        );
        assert_eq!(window.selected, 4, "premise: neither row is current");
        window.click(egui::pos2(third.center().x, third.min.y));
        assert!(
            matches!(window.selected, 1 | 2),
            "one of the two picked, not {}",
            window.selected
        );
        window.frame(Vec::new());
        assert_eq!(window.rows.len(), 0, "the list closed");
    }

    // The rows meet, so no click lands between two of them: the popup's
    // `Area`, which senses a click anywhere in it, would take it and
    // close the list with nothing picked.
    // ST-COMBO-01
    #[test]
    fn the_rows_meet_and_a_click_where_two_meet_picks_one() {
        let mut window = Window::new(false);
        window.selected = 4;
        open(&mut window);
        assert_rows_meet(&window);
        click_where_second_and_third_meet(&mut window);
    }

    // A row under the pointer is the hovered widget, so it paints its
    // highlight.
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
    fn a_scrolling_list_on_a_touch_machine_picks_a_row_where_two_meet_too() {
        let mut window = Window::new(true);
        window.selected = 4;
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
        click_where_second_and_third_meet(&mut window);
    }

    // The open list marks the current item's row selected, and no
    // other (AccessKit's toggled state on the row buttons).
    #[test]
    fn the_open_list_marks_the_current_row_selected() {
        let mut window = Window::new(false);
        open(&mut window);
        let rows: Vec<&accesskit::Node> = window
            .nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node.label().is_some_and(|label| ITEMS.contains(&label))
            })
            .collect();
        assert_eq!(rows.len(), ITEMS.len(), "premise: the rows' nodes");
        let marked: Vec<Option<&str>> = rows
            .iter()
            .filter(|node| node.toggled() == Some(Toggled::True))
            .map(|node| node.label())
            .collect();
        assert_eq!(marked, [Some(ITEMS[1])], "the current row, and it alone");
    }
}
