//! The toolbars' menu button: egui's menu button, its label ending in the
//! drafts' arrow and its entries meeting, and the separator between groups
//! of entries.

use egui::containers::menu::{MenuButton, MenuConfig, menu_style};

use super::combo::rows_meet;

/// A toolbar menu button: egui's menu button with `label` followed by ` ⏷`
/// (U+23F7, the symbols font's), as the drafts draw every menu button. egui
/// draws an arrow only on a submenu's button, and the arrow is what tells a
/// menu from a button.
///
/// The menu's entries meet, as a combo's rows do (`combo_label` says why):
/// each is 21 points high with nothing between two, so a click between two
/// entries acts on one of them instead of closing the menu with nothing
/// done. The menu takes egui's menu style with that change (`rows_meet`);
/// the rest of its configuration is the one egui would find for it, a
/// menu bar's when the button sits in one. Inside a menu it still opens a
/// menu, not a submenu.
pub fn menu_button<R>(
    ui: &mut egui::Ui,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let config = MenuConfig::find(ui).style(|style: &mut egui::Style| {
        menu_style(style);
        rows_meet(&mut style.spacing);
    });
    let (response, inner) = MenuButton::new(format!("{label} \u{23f7}"))
        .config(config)
        .ui(ui, add_contents);
    egui::InnerResponse::new(inner.map(|inner| inner.inner), response)
}

/// The line between two groups of a toolbar menu's entries, in the middle
/// of the 12 points it stands in: the room egui's separator takes in a
/// menu where entries are a gap apart (6, and the gap above and below
/// it). egui's separator has no style setting for that room, and with the
/// menu's entries meeting a bare one would stand in 6.
pub fn menu_separator(ui: &mut egui::Ui) -> egui::Response {
    ui.add(egui::Separator::default().spacing(12.0))
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{self, Role};

    use super::*;

    /// One frame of a toolbar holding the "Team" menu, `contents` its
    /// popup's, with `events` as its input; the frame's AccessKit nodes.
    fn frame<R>(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        mut contents: impl FnMut(&mut egui::Ui) -> R,
    ) -> Vec<accesskit::Node> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            menu_button(ui, "Team", &mut contents);
        });
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        update.nodes.into_iter().map(|(_, node)| node).collect()
    }

    fn button<'a>(nodes: &'a [accesskit::Node], label: &str) -> Option<&'a accesskit::Node> {
        nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some(label))
    }

    /// The centre of a node's bounds.
    fn centre(node: &accesskit::Node) -> egui::Pos2 {
        let bounds = node.bounds().expect("a drawn button has bounds");
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    /// The primary button pressed (or released) at `pos`, the pointer moved
    /// there first.
    fn pointer_at(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    /// A context with AccessKit on and the suite's theme.
    fn themed() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        crate::theme::install(&ctx);
        ctx
    }

    #[test]
    fn the_label_ends_in_the_arrow_and_a_click_opens_the_menu() {
        let ctx = themed();
        let item = |ui: &mut egui::Ui| ui.button("Set edited flag");
        let nodes = frame(&ctx, Vec::new(), item);
        let team = button(&nodes, "Team \u{23f7}").expect("the menu button reads \"Team ⏷\"");
        assert!(
            button(&nodes, "Set edited flag").is_none(),
            "the menu starts closed"
        );
        let pos = centre(team);
        frame(&ctx, pointer_at(pos, true), item);
        frame(&ctx, pointer_at(pos, false), item);
        let nodes = frame(&ctx, Vec::new(), item);
        assert!(
            button(&nodes, "Set edited flag").is_some(),
            "the click opened the menu"
        );
    }

    /// The "Team" menu of three entries with a `menu_separator` between
    /// the second and the third, each entry setting its own flag and
    /// closing the menu. `rects` holds the last frame's entries and
    /// separator, top to bottom, and is empty while the menu is closed.
    struct ThreeEntries {
        ctx: egui::Context,
        rects: Vec<egui::Rect>,
        acted: [bool; 3],
    }

    impl ThreeEntries {
        fn frame(&mut self, events: Vec<egui::Event>) -> Vec<accesskit::Node> {
            let (rects, acted) = (&mut self.rects, &mut self.acted);
            rects.clear();
            frame(&self.ctx, events, |ui| {
                // A popup's first frame runs twice: the last pass's rects count.
                rects.clear();
                for (index, label) in ["One", "Two", "Three"].into_iter().enumerate() {
                    if index == 2 {
                        rects.push(menu_separator(ui).rect);
                    }
                    let entry = ui.button(label);
                    rects.push(entry.rect);
                    if entry.clicked() {
                        acted[index] = true;
                        ui.close();
                    }
                }
            })
        }

        /// A press and a release at `pos`, one frame each, then a frame
        /// with no input, so `rects` shows whether the menu is still open.
        fn click(&mut self, pos: egui::Pos2) {
            self.frame(pointer_at(pos, true));
            self.frame(pointer_at(pos, false));
            self.frame(Vec::new());
        }

        /// Opens the closed menu with a click on its button, every flag
        /// cleared.
        fn open(&mut self) {
            let nodes = self.frame(Vec::new());
            let team = button(&nodes, "Team \u{23f7}").expect("the menu button");
            self.click(centre(team));
            assert_eq!(self.rects.len(), 4, "premise: the menu opened");
            self.acted = [false; 3];
        }
    }

    // ST-COMBO-02
    #[test]
    fn a_menus_entries_meet_so_every_click_on_them_acts() {
        let mut menu = ThreeEntries {
            ctx: themed(),
            rects: Vec::new(),
            acted: [false; 3],
        };
        menu.open();
        let &[first, second, separator, third] = menu.rects.as_slice() else {
            panic!("three entries and a separator: {:?}", menu.rects);
        };
        for entry in [first, second, third] {
            assert_eq!(entry.height(), 21.0, "each entry is 21 high: {entry:?}");
        }
        assert_eq!(second.min.y, first.max.y, "the first two entries meet");
        assert_eq!(
            third.min.y - second.max.y,
            12.0,
            "the separator stands in 12 between its neighbours ({separator:?})"
        );

        menu.click(egui::pos2(first.center().x, first.max.y));
        assert!(
            menu.acted[0] != menu.acted[1] && !menu.acted[2],
            "one of the first two acted: {:?}",
            menu.acted
        );
        assert!(menu.rects.is_empty(), "the menu closed");

        let x = first.center().x;
        let mut y = first.min.y;
        while y <= third.max.y {
            if y <= second.max.y || y >= third.min.y {
                menu.open();
                menu.click(egui::pos2(x, y));
                assert!(
                    menu.acted.contains(&true),
                    "a click at y = {y} acted on no entry ({first:?}, {second:?}, {third:?})"
                );
            }
            y += 0.5;
        }
    }
}
