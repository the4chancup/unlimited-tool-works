//! The suite's disabled widgets: egui's, faded to half opacity as egui fades
//! them, with their text dimmed further to the palette's `disabled_text`.
//!
//! egui fades a disabled widget whole, so its text keeps the resting text's
//! grey at half opacity, which leaves a disabled button reading almost as
//! plainly as an enabled one. egui's one `disabled_alpha` cannot keep the box
//! and darken the text, so a disabled scope draws its widgets' resting text
//! in a darker colour, which the same fade then draws at the drafts' disabled
//! grey. egui never draws a disabled widget hovered or pressed, so the
//! resting colour is the only one it shows.

/// Adds `widget`, disabled unless `enabled`: egui's `Ui::add_enabled`, the
/// widget's text dimmed to the palette's `disabled_text` while disabled.
/// Not egui's own, whose disabled text is barely dimmer than an enabled one's.
pub fn add_enabled(ui: &mut egui::Ui, enabled: bool, widget: impl egui::Widget) -> egui::Response {
    if enabled {
        return ui.add(widget);
    }
    // egui disables the widget in `ui` itself, not in a child scope, so its id
    // and its place in a wrapping row are those an enabled widget gets. The
    // colour is set on `ui` too, and its style put back afterwards.
    let style = ui.style().clone();
    dim_text(ui);
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's disabled scope is egui's with the text colour set"
    )]
    let response = ui.add_enabled(false, widget);
    ui.set_style(style);
    response
}

/// Adds the widgets `add_contents` draws in a scope that is disabled unless
/// `enabled`: egui's `Ui::add_enabled_ui`, its widgets' text dimmed to the
/// palette's `disabled_text` while disabled. Not egui's own, whose disabled
/// text is barely dimmer than an enabled one's.
pub fn add_enabled_ui<R>(
    ui: &mut egui::Ui,
    enabled: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the suite's disabled scope is egui's with the text colour set"
    )]
    let scope = ui.add_enabled_ui(enabled, |ui| {
        if !enabled {
            dim_text(ui);
        }
        add_contents(ui)
    });
    scope
}

/// Draws the resting widgets' text in `ui` in the palette's `disabled_text`.
fn dim_text(ui: &mut egui::Ui) {
    let dimmed = crate::theme::palette(ui.visuals()).disabled_text;
    ui.visuals_mut().widgets.inactive.fg_stroke.color = dimmed;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context with the suite's look installed, as the window has it, in
    /// `theme`.
    fn context(theme: egui::Theme) -> egui::Context {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        ctx.set_theme(theme);
        ctx
    }

    /// One 800 by 600 frame drawing `contents` with `events` as its input.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, contents: impl FnMut(&mut egui::Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        };
        let mut contents = contents;
        let output = ctx.run_ui(input, |ui| contents(ui));
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
    }

    /// The resting widgets' text colour in `ui`, and whether `ui` is enabled.
    fn resting(ui: &egui::Ui) -> (egui::Color32, bool) {
        (
            ui.visuals().widgets.inactive.fg_stroke.color,
            ui.is_enabled(),
        )
    }

    /// The resting text colour and enabled state seen inside a disabled and
    /// an enabled `add_enabled_ui`, then inside a disabled and an enabled
    /// `add_enabled`, in `theme`.
    fn seen_inside(theme: egui::Theme) -> Vec<(egui::Color32, bool)> {
        let mut seen = Vec::new();
        frame(&context(theme), Vec::new(), |ui| {
            for enabled in [false, true] {
                add_enabled_ui(ui, enabled, |ui| seen.push(resting(ui)));
            }
            for enabled in [false, true] {
                add_enabled(ui, enabled, |ui: &mut egui::Ui| {
                    seen.push(resting(ui));
                    ui.label("Undo")
                });
            }
        });
        seen
    }

    #[test]
    fn a_disabled_scope_draws_the_dark_resting_text_in_the_disabled_grey() {
        let resting = egui::Color32::from_gray(180);
        let disabled = (egui::Color32::from_gray(134), false);
        assert_eq!(
            seen_inside(egui::Theme::Dark),
            vec![disabled, (resting, true), disabled, (resting, true)],
            "add_enabled_ui disabled and enabled, then add_enabled"
        );
    }

    #[test]
    fn a_disabled_scope_keeps_the_light_resting_text() {
        let resting = egui::Color32::from_gray(60);
        assert_eq!(
            seen_inside(egui::Theme::Light),
            vec![
                (resting, false),
                (resting, true),
                (resting, false),
                (resting, true)
            ],
            "add_enabled_ui disabled and enabled, then add_enabled"
        );
    }

    /// Whether an "Undo" button added through `add_enabled` with `enabled`
    /// reports itself enabled, and whether a click on it is reported.
    fn click_undo(enabled: bool) -> (bool, bool) {
        let ctx = context(egui::Theme::Dark);
        let rect = std::cell::Cell::new(egui::Rect::NOTHING);
        let seen = std::cell::Cell::new((false, false));
        let mut undo = |ui: &mut egui::Ui| {
            let response = add_enabled(ui, enabled, egui::Button::new("Undo"));
            rect.set(response.rect);
            seen.set((response.enabled(), seen.get().1 || response.clicked()));
        };
        frame(&ctx, Vec::new(), &mut undo);
        let pos = rect.get().center();
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        frame(
            &ctx,
            vec![egui::Event::PointerMoved(pos), button(true)],
            &mut undo,
        );
        frame(&ctx, vec![button(false)], &mut undo);
        frame(&ctx, Vec::new(), &mut undo);
        seen.get()
    }

    #[test]
    fn a_disabled_button_reports_no_click() {
        assert_eq!(click_undo(false), (false, false), "(enabled, clicked)");
        assert_eq!(click_undo(true), (true, true), "(enabled, clicked)");
    }

    #[test]
    fn the_disabled_grey_does_not_leak_into_the_parent() {
        let mut seen = Vec::new();
        frame(&context(egui::Theme::Dark), Vec::new(), |ui| {
            add_enabled_ui(ui, false, |ui| ui.label("Undo"));
            seen.push(resting(ui));
            add_enabled(ui, false, egui::Button::new("Undo"));
            seen.push(resting(ui));
        });
        let parent = (egui::Color32::from_gray(180), true);
        assert_eq!(
            seen,
            vec![parent, parent],
            "after add_enabled_ui, then after add_enabled"
        );
    }
}
