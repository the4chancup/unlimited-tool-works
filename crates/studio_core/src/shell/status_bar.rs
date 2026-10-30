//! The status bar, as the draft `docs/prototypes/index.html` draws it: one 24 px row under the
//! sidebar and the tool view, drawn in every tool. Three slots: the version and the
//! held `ShellCondition`s left at their content's width, the other tools' `ToolActivity`s
//! right at their text's width capped at a third of the bar, and the latest `Notice` middle
//! in the space they leave. Tools cannot draw into the bar; they return an activity and
//! publish notices (`crate::status`).

use std::time::Duration;

use crate::shell::window_size;
use crate::status::{ActionEffect, Notice, ShellCondition};
use crate::widgets::add_enabled;

/// How long a notice without an action stays in the middle slot (the draft's
/// `noticeTimeoutS`), measured on egui's clock from the frame it is first drawn. A notice
/// with an action stays until the action or the dismiss is clicked.
const NOTICE_SECONDS: f64 = 5.0;

/// The bar's state across frames: the conditions currently held and the latest notice.
#[derive(Default)]
pub(super) struct StatusBar {
    /// The held conditions, in the order they were raised; each leaves when the shell drops
    /// it, not by timeout.
    conditions: Vec<ShellCondition>,
    /// The latest notice; a new one replaces it (older notices are not kept).
    notice: Option<Notice>,
    /// egui time of the frame a no-action notice was first drawn.
    notice_shown_at: Option<f64>,
    /// The shown notice is the active tool's standing one (`StudioTool::standing_notice`):
    /// it never expires, and its tool withdraws it.
    standing: bool,
    /// A standing notice left the bar by being seen (its action or dismiss clicked, or
    /// another notice replacing it) since the shell last asked: its tool is to be told.
    standing_ended: bool,
}

impl StatusBar {
    /// Raises `condition` while `holds`, drops it when it stops. A held condition is shown
    /// once even if raised twice. Only the settings save raises one for now, and the browser
    /// build has no settings file.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn set_condition(&mut self, condition: ShellCondition, holds: bool) {
        if holds {
            if !self.conditions.contains(&condition) {
                self.conditions.push(condition);
            }
        } else {
            self.conditions.retain(|held| *held != condition);
        }
    }

    /// The latest notice; it replaces the one before it, and ends a standing one it replaces.
    pub(super) fn set_notice(&mut self, notice: Notice) {
        self.end_standing();
        self.notice = Some(notice);
        self.notice_shown_at = None;
    }

    /// The active tool's standing notice, asked for every frame. `Some` takes the slot: a
    /// reworded one replaces the text, one that just appeared drops the notice shown before
    /// it. `None` withdraws a standing notice without ending it, since the withdrawal is the
    /// tool's own doing (or the shell's, on a switch of tool) and nobody saw it end; a
    /// one-off notice in the slot stays.
    pub(super) fn stand(&mut self, notice: Option<Notice>) {
        match notice {
            Some(notice) => {
                self.notice = Some(notice);
                self.notice_shown_at = None;
                self.standing = true;
            }
            None if self.standing => {
                self.notice = None;
                self.notice_shown_at = None;
                self.standing = false;
            }
            None => {}
        }
    }

    /// Whether a standing notice ended since the last call; the call clears it, so the
    /// tool is told once.
    pub(super) fn take_standing_end(&mut self) -> bool {
        std::mem::take(&mut self.standing_ended)
    }

    /// Drops the shown notice, if any, ending it if it was standing.
    fn dismiss(&mut self) {
        self.end_standing();
        self.notice = None;
        self.notice_shown_at = None;
    }

    /// Marks a shown standing notice as ended by being seen.
    fn end_standing(&mut self) {
        if self.standing {
            self.standing = false;
            self.standing_ended = true;
        }
    }

    /// Drops a no-action notice once `NOTICE_SECONDS` have passed since the frame it was
    /// first drawn, and asks for the repaint that removes it while no input arrives. A
    /// standing notice never expires: its tool withdraws it.
    fn expire(&mut self, ui: &egui::Ui) {
        let now = ui.input(|input| input.time);
        let expired = match &self.notice {
            Some(notice) if notice.action.is_none() && !self.standing => {
                let shown_at = *self.notice_shown_at.get_or_insert(now);
                let remaining = NOTICE_SECONDS - (now - shown_at);
                if remaining > 0.0 {
                    ui.ctx()
                        .request_repaint_after(Duration::from_secs_f64(remaining));
                }
                remaining <= 0.0
            }
            _ => false,
        };
        if expired {
            self.dismiss();
        }
    }

    /// Draws the row's slots, the middle taking what the sides leave: the left and right
    /// slots are as wide as what they hold and the notice's slot is the rest.
    /// `activities` are the not-active tools' activity texts in
    /// sidebar order (the active tool's run strip already shows its own). `blocker` is the
    /// version-change blocker a tool reports, as its label and reason: while it holds, a
    /// `SwitchVersion` action is drawn disabled with the selector's hover, and the notice
    /// stays. `monitor_of` names the monitor the window stands on, from the frame's
    /// viewport and zoom (`window_size::monitor_under`), asked only when Reset window is
    /// clicked. Returns the notice action the user clicked, for the shell to apply; clicking
    /// it also dismisses the notice.
    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        activities: &[String],
        blocker: Option<(&str, &str)>,
        monitor_of: impl FnOnce(&egui::ViewportInfo, f32) -> Option<egui::Rect>,
    ) -> Option<ActionEffect> {
        self.expire(ui);
        let mut effect = None;
        let mut dismissed = false;
        let bar = ui.max_rect();
        let spacing = ui.spacing().item_spacing.x;
        let left_right = self.left_slot(ui, monitor_of);
        // Right: the other tools' activities, at their text's width capped at a third
        // of the bar (an activity line is short; the cap keeps a long one from taking
        // the notice's room), packed against the right edge and truncating past the
        // cap. No activity, no width.
        let texts = activities.join(" · ");
        let right_width = if texts.is_empty() {
            0.0
        } else {
            ui.fonts_mut(|fonts| {
                fonts.layout_no_wrap(
                    texts.clone(),
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.visuals().text_color(),
                )
            })
            .size()
            .x
            .min(bar.width() / 3.0)
        };
        if right_width > 0.0 {
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(bar.right() - right_width, bar.top()),
                        bar.right_bottom(),
                    ))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                |ui| {
                    ui.add(egui::Label::new(&texts).truncate())
                        .on_hover_text(&texts);
                },
            );
        }
        // Middle: the notice — text in its kind's colour, its action, the
        // dismiss X — centred in the space the sides leave as one group. egui
        // centres an allocated rect, not a row of widgets, so the group's width
        // is measured from the galleys and the buttons' padding and the row is
        // padded to it.
        let middle_left = left_right + spacing;
        let middle_right = if right_width > 0.0 {
            bar.right() - right_width - spacing
        } else {
            bar.right()
        }
        .max(middle_left);
        // The scope's id is pinned: the right slot's scope exists only while an
        // activity is up, and a salted id would change the notice's widget ids
        // the frame one appears.
        ui.scope_builder(
            egui::UiBuilder::new()
                .scope_id(ui.make_persistent_id("notice"))
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(middle_left, bar.top()),
                    egui::pos2(middle_right, bar.bottom()),
                )),
            |ui| {
                if let Some(notice) = &self.notice {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.horizontal(|ui| {
                            let color = crate::theme::notice_color(notice.kind, ui.visuals());
                            let spacing = ui.spacing().item_spacing.x;
                            let button = egui::TextStyle::Button.resolve(ui.style());
                            let body = egui::TextStyle::Body.resolve(ui.style());
                            let galley_width =
                                |ui: &mut egui::Ui, text: &str, font: &egui::FontId| {
                                    ui.fonts_mut(|fonts| {
                                        fonts.layout_no_wrap(text.to_owned(), font.clone(), color)
                                    })
                                    .size()
                                    .x
                                };
                            let dismiss_width = galley_width(ui, crate::icons::X, &button);
                            let action_width = notice.action.as_ref().map_or(0.0, |action| {
                                galley_width(ui, &action.label, &button)
                                    + 2.0 * ui.spacing().button_padding.x
                            });
                            let gaps = if notice.action.is_some() {
                                2.0 * spacing
                            } else {
                                spacing
                            };
                            let text_width = galley_width(ui, &notice.text, &body)
                                .min(ui.available_width() - dismiss_width - action_width - gaps)
                                .max(0.0);
                            let group = text_width + action_width + dismiss_width + gaps;
                            ui.add_space(((ui.available_width() - group) / 2.0).max(0.0));
                            ui.add_sized(
                                [text_width, ui.spacing().interact_size.y],
                                egui::Label::new(egui::RichText::new(&notice.text).color(color))
                                    .truncate(),
                            )
                            .on_hover_text(&notice.text);
                            if let Some(action) = &notice.action {
                                let blocked = match action.effect {
                                    ActionEffect::SwitchVersion(_) => blocker,
                                    ActionEffect::OpenFolder(_)
                                    | ActionEffect::OpenSettings
                                    | ActionEffect::OpenHelp(_)
                                    | ActionEffect::SwitchTool(_)
                                    | ActionEffect::FixPesPath { .. } => None,
                                };
                                let button = add_enabled(
                                    ui,
                                    blocked.is_none(),
                                    egui::Button::new(&action.label),
                                );
                                if let Some((label, reason)) = blocked {
                                    button.on_disabled_hover_text(format!("{label}: {reason}"));
                                } else if button.clicked() {
                                    dismissed = true;
                                    effect = Some(action.effect.clone());
                                }
                            }
                            let x = ui.add(egui::Button::new(crate::icons::X).frame(false));
                            x.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Button, true, "Dismiss")
                            });
                            if x.on_hover_text("Dismiss").clicked() {
                                dismissed = true;
                            }
                        });
                    });
                }
            },
        );
        if effect.is_some() || dismissed {
            self.dismiss();
        }
        effect
    }

    /// The left slot: the version, always, then the Reset window button while
    /// the window sits off the main class's fitted design size, then one
    /// icon-and-text entry per held condition, each carrying its tooltip.
    /// Returns the content's right edge, the middle slot's left bound.
    fn left_slot(
        &self,
        ui: &mut egui::Ui,
        monitor_of: impl FnOnce(&egui::ViewportInfo, f32) -> Option<egui::Rect>,
    ) -> f32 {
        let (viewport, screen_rect) =
            ui.input(|input| (input.viewport().clone(), input.raw.screen_rect));
        let zoom = ui.ctx().zoom_factor();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(crate::shell::version_label())
                    .small()
                    .weak(),
            );
            if window_size::off_design_size(&viewport, screen_rect, zoom, window_size::DEFAULT_SIZE)
            {
                let reset = ui
                    .add(
                        egui::Button::new(egui::RichText::new("Reset window").small().weak())
                            .frame(false),
                    )
                    .on_hover_text("Back to the default window size, centred on its monitor");
                if reset.clicked() {
                    window_size::reset(ui.ctx(), &viewport, monitor_of(&viewport, zoom));
                }
            }
            for condition in &self.conditions {
                // The labels are not selectable: a selectable label senses click+drag and
                // would shadow the entry's own `hovered`, keeping the tooltip shut.
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(condition.icon()).selectable(false));
                    ui.add(
                        egui::Label::new(condition.text())
                            .truncate()
                            // The elided label's own "full text" tooltip would
                            // mask the entry's, which explains the condition.
                            .show_tooltip_when_elided(false)
                            .selectable(false),
                    );
                })
                .response
                .on_hover_text(condition.tooltip());
            }
            ui.min_rect().right()
        })
        .inner
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    use egui::accesskit::Role;
    use pes_version::PesVersion;

    use crate::settings::Settings;
    use crate::shell::StudioApp;
    use crate::shell::test_harness::*;
    use crate::status::{ActionEffect, Notice, NoticeAction, NoticeKind};

    /// The `Label` nodes' texts drawn inside the bottom 24 px of the 800x600 screen (a
    /// `RichText` label also leaves a `TextRun` child, so `Label` alone is counted).
    fn bottom_texts(nodes: &[egui::accesskit::Node]) -> Vec<&str> {
        texts_below(nodes, 576.0)
    }

    /// The `Label` nodes' texts drawn at or below `y` on the frame's screen.
    fn texts_below(nodes: &[egui::accesskit::Node], y: f64) -> Vec<&str> {
        nodes
            .iter()
            .filter(|node| node.role() == Role::Label)
            .filter(|node| node.bounds().is_some_and(|bounds| bounds.y0 >= y))
            .filter_map(|node| node.value())
            .collect()
    }

    /// The `Button` nodes' labels drawn at or below `y` on the frame's screen.
    fn buttons_below(nodes: &[egui::accesskit::Node], y: f64) -> Vec<&str> {
        nodes
            .iter()
            .filter(|node| node.role() == Role::Button)
            .filter(|node| node.bounds().is_some_and(|bounds| bounds.y0 >= y))
            .filter_map(|node| node.label())
            .collect()
    }

    /// The left slot's right edge: the version label's right side (no condition is
    /// raised in these tests, so the version is the slot's last entry).
    fn left_slot_right(nodes: &[egui::accesskit::Node]) -> f64 {
        let version = crate::shell::version_label();
        nodes
            .iter()
            .find(|node| node.value() == Some(version.as_str()))
            .expect("the version label is drawn")
            .bounds()
            .unwrap()
            .x1
    }

    /// One frame's input reporting a window `size` zoom-1 points big on a
    /// `monitor`-point monitor, `maximized` or not: the screen and inner rects
    /// are `size`, the outer `size` plus (16, 39) of decorations, and the
    /// inner is inset by the 8-wide border and 31-tall title bar.
    fn placed_input(
        outer_min: egui::Pos2,
        size: egui::Vec2,
        monitor: egui::Vec2,
        maximized: bool,
    ) -> egui::RawInput {
        placed_input_zoomed(outer_min, size, monitor, maximized, 1.0)
    }

    /// `placed_input` for a pass running at `zoom`: egui-winit reports the
    /// monitor, the rects and the positions in the zoomed points, so every
    /// zoom-1 value is divided by `zoom`.
    fn placed_input_zoomed(
        outer_min: egui::Pos2,
        size: egui::Vec2,
        monitor: egui::Vec2,
        maximized: bool,
        zoom: f32,
    ) -> egui::RawInput {
        let mut input = raw_input(Vec::new(), true, None, None);
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size / zoom));
        let info = input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .expect("`RawInput::default` carries the root viewport");
        info.monitor_size = Some(monitor / zoom);
        info.outer_rect =
            Some(egui::Rect::from_min_size(outer_min, size + egui::vec2(16.0, 39.0)) / zoom);
        info.inner_rect =
            Some(egui::Rect::from_min_size(outer_min + egui::vec2(8.0, 31.0), size) / zoom);
        info.maximized = Some(maximized);
        input
    }

    /// Frames of a window `size` zoom-1 points big on `monitor` until the
    /// shell's width zoom lands, bounded: the first frame reports at zoom 1
    /// and queues it, the next runs at `zoom`'s points.
    fn land_zoom(
        ctx: &egui::Context,
        app: &mut StudioApp,
        size: egui::Vec2,
        monitor: egui::Vec2,
        maximized: bool,
    ) {
        let zoom = size.x / crate::shell::window_size::DEFAULT_SIZE.x;
        run_frame(
            ctx,
            app,
            placed_input_zoomed(egui::Pos2::ZERO, size, monitor, maximized, 1.0),
        );
        for _ in 0..10 {
            run_frame(
                ctx,
                app,
                placed_input_zoomed(egui::Pos2::ZERO, size, monitor, maximized, zoom),
            );
            if ctx.zoom_factor() == zoom {
                return;
            }
        }
        panic!("the width zoom did not land at {zoom}");
    }

    /// Whether `input`'s frame draws the Reset window button, on a fresh
    /// context so each case stands at zoom 1.
    fn draws_reset(app: &mut StudioApp, input: egui::RawInput) -> bool {
        has_label(
            &run_frame(&test_context(), app, input).nodes,
            "Reset window",
        )
    }

    /// The frame's commands to the window's place: size, position and
    /// maximize, the list the Reset window click produces.
    fn place_commands(output: &FrameOutput) -> Vec<egui::ViewportCommand> {
        output
            .commands
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    egui::ViewportCommand::InnerSize(_)
                        | egui::ViewportCommand::OuterPosition(_)
                        | egui::ViewportCommand::Maximized(_)
                )
            })
            .cloned()
            .collect()
    }

    // ST-BAR-01
    #[test]
    fn an_empty_bar_on_a_maximized_window_shows_the_version_and_reset_window() {
        let Shell { mut app, _dir, .. } = shell();
        let egui_ctx = test_context();
        let nodes = run_frame(
            &egui_ctx,
            &mut app,
            placed_input(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1041.0),
                egui::vec2(1920.0, 1080.0),
                true,
            ),
        )
        .nodes;
        let version = crate::shell::version_label();
        // The bar's strip is the bottom 24 points of the 1041-point screen.
        assert_eq!(
            texts_below(&nodes, 1017.0),
            [version.as_str()],
            "the bar's only text is the version"
        );
        assert_eq!(
            buttons_below(&nodes, 1017.0),
            ["Reset window"],
            "its only button is Reset window"
        );
    }

    // ST-BAR-06
    #[test]
    fn the_reset_button_is_drawn_only_while_the_window_is_off_its_fitted_size() {
        let Shell { mut app, _dir, .. } = shell();
        let fhd = egui::vec2(1920.0, 1080.0);

        assert!(
            !draws_reset(
                &mut app,
                placed_input(egui::Pos2::ZERO, egui::vec2(1280.0, 780.0), fhd, false)
            ),
            "1280 x 780 on a 1920 x 1080 monitor: the fitted design size"
        );
        assert!(
            draws_reset(
                &mut app,
                placed_input(egui::Pos2::ZERO, egui::vec2(1100.0, 780.0), fhd, false)
            ),
            "resized narrower than the design size"
        );
        assert!(
            draws_reset(
                &mut app,
                placed_input(egui::Pos2::ZERO, egui::vec2(1920.0, 1041.0), fhd, true)
            ),
            "maximized"
        );
        assert!(
            !draws_reset(
                &mut app,
                placed_input(
                    egui::pos2(400.0, 200.0),
                    egui::vec2(1280.0, 780.0),
                    fhd,
                    false
                )
            ),
            "moved at the design size: the position does not count"
        );
        // A monitor too narrow for the decorated design width fits the window
        // to 1184 x 780, under the design width so at zoom < 1: at that fit
        // the button is not drawn either.
        assert!(
            !draws_reset(
                &mut app,
                placed_input(
                    egui::Pos2::ZERO,
                    egui::vec2(1184.0, 780.0),
                    egui::vec2(1200.0, 2000.0),
                    false
                )
            ),
            "at the fit on a narrow monitor, where the zoom is under 1"
        );

        // The same windows on the passes where the width zoom has landed:
        // the viewport info in the zoom's points. The 1100-wide window is
        // off the fit at every zoom; the narrow-monitor fit is its size at
        // zoom 1, drawn or not by the same rule.
        let egui_ctx = test_context();
        land_zoom(&egui_ctx, &mut app, egui::vec2(1100.0, 780.0), fhd, false);
        assert!(
            has_label(
                &run_frame(
                    &egui_ctx,
                    &mut app,
                    placed_input_zoomed(
                        egui::Pos2::ZERO,
                        egui::vec2(1100.0, 780.0),
                        fhd,
                        false,
                        1100.0 / 1280.0,
                    ),
                )
                .nodes,
                "Reset window"
            ),
            "resized under the width zoom"
        );
        let egui_ctx = test_context();
        land_zoom(
            &egui_ctx,
            &mut app,
            egui::vec2(1184.0, 780.0),
            egui::vec2(1200.0, 2000.0),
            false,
        );
        assert!(
            !has_label(
                &run_frame(
                    &egui_ctx,
                    &mut app,
                    placed_input_zoomed(
                        egui::Pos2::ZERO,
                        egui::vec2(1184.0, 780.0),
                        egui::vec2(1200.0, 2000.0),
                        false,
                        1184.0 / 1280.0,
                    ),
                )
                .nodes,
                "Reset window"
            ),
            "at the narrow monitor's fit, in its zoom's points"
        );

        // A frame reporting a monitor but no window size, and one reporting
        // neither, leave no evidence the window is off its size.
        let mut no_size = raw_input(Vec::new(), true, None, None);
        no_size
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .expect("`RawInput::default` carries the root viewport")
            .monitor_size = Some(fhd);
        assert!(
            !draws_reset(&mut app, no_size),
            "a monitor but no reported size"
        );
        assert!(
            !draws_reset(&mut app, raw_input(Vec::new(), true, None, None)),
            "neither reported"
        );
    }

    // ST-BAR-05
    #[test]
    fn clicking_reset_window_restores_then_fits_and_centres_the_window() {
        let Shell { mut app, _dir, .. } = shell();
        let egui_ctx = test_context();
        let maximized = || {
            placed_input(
                egui::Pos2::ZERO,
                egui::vec2(1920.0, 1041.0),
                egui::vec2(1920.0, 1080.0),
                true,
            )
        };

        let nodes = run_frame(&egui_ctx, &mut app, maximized()).nodes;
        let button = entry(&nodes, "Reset window");
        // The press and release frames report the same maximized window; the
        // click lands on the release frame, so its output holds the commands.
        let mut press = maximized();
        press.events = pointer(button, true);
        run_frame(&egui_ctx, &mut app, press);
        let mut release = maximized();
        release.events = pointer(button, false);
        let out = run_frame(&egui_ctx, &mut app, release);

        assert_eq!(
            place_commands(&out),
            [
                egui::ViewportCommand::Maximized(false),
                egui::ViewportCommand::InnerSize(egui::vec2(1280.0, 780.0)),
                egui::ViewportCommand::OuterPosition(egui::pos2(312.0, 130.5)),
            ],
            "restored first, then the fit centring the window on its monitor"
        );
    }

    // ST-BAR-05
    #[test]
    fn clicking_reset_window_under_the_width_zoom_fits_at_zoom_one_points() {
        let Shell { mut app, _dir, .. } = shell();
        let egui_ctx = test_context();
        let monitor = egui::vec2(1920.0, 1080.0);
        let size = egui::vec2(1100.0, 780.0);
        let zoom = size.x / crate::shell::window_size::DEFAULT_SIZE.x;
        let input = |events: Vec<egui::Event>| {
            let mut input = placed_input_zoomed(egui::Pos2::ZERO, size, monitor, false, zoom);
            input.events = events;
            input
        };

        land_zoom(&egui_ctx, &mut app, size, monitor, false);
        let nodes = run_frame(&egui_ctx, &mut app, input(Vec::new())).nodes;
        let button = entry(&nodes, "Reset window");
        run_frame(&egui_ctx, &mut app, input(pointer(button, true)));
        let out = run_frame(&egui_ctx, &mut app, input(pointer(button, false)));

        // The fit worked at zoom 1: 1280 x 780 centred on the 1920 x 1080
        // monitor less the (16, 39) decorations; the commands take the
        // zoom's points, so each is the zoom-1 answer over it.
        let expected_size = egui::vec2(1280.0, 780.0) / zoom;
        let expected_position = egui::pos2(312.0, 130.5) / zoom;
        let commands = place_commands(&out);
        assert!(
            commands
                .iter()
                .all(|command| !matches!(command, egui::ViewportCommand::Maximized(_))),
            "no Maximized on a window that was not maximized: {commands:?}"
        );
        for (command, expected) in [
            (
                commands.iter().find_map(|command| match command {
                    egui::ViewportCommand::InnerSize(size) => Some(*size),
                    _ => None,
                }),
                expected_size,
            ),
            (
                commands.iter().find_map(|command| match command {
                    egui::ViewportCommand::OuterPosition(position) => Some(position.to_vec2()),
                    _ => None,
                }),
                expected_position.to_vec2(),
            ),
        ] {
            let got = command.unwrap_or_else(|| panic!("a command is missing: {commands:?}"));
            assert!(
                (got - expected).abs().max_elem() <= 0.01,
                "{got:?} vs {expected:?} in the zoom's points"
            );
        }
    }

    /// The frame's `Shape::Text` carrying exactly `text` painted inside the
    /// bottom 24 px of the 800x600 screen (the status bar's row).
    fn bar_text_shape<'a>(out: &'a FrameOutput, text: &str) -> Option<&'a egui::epaint::TextShape> {
        out.shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) => Some(shape),
                _ => None,
            })
            .find(|shape| shape.galley.text() == text && shape.pos.y >= 576.0)
    }

    // ST-BAR-02
    #[test]
    fn the_right_slot_shows_the_other_tools_activities_not_the_active_ones() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();

        calls[1].set_activity("B", Some((12, 48)), Duration::from_secs(72));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        let activity = nodes
            .iter()
            .find(|node| node.value() == Some("B · 12/48 · 01:12"))
            .expect("the second tool's activity is drawn");
        let bounds = activity.bounds().unwrap();
        assert!(
            bounds.x0 >= 1600.0 / 3.0,
            "the activity sits in the right slot: {bounds:?}"
        );

        // With the second tool active its activity leaves the slot.
        ctx.switch_to_tool("b");
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, "B · 12/48 · 01:12"),
            "the active tool's activity is its own view's, not the bar's"
        );
    }

    // ST-BAR-03
    #[test]
    fn a_notice_with_an_action_stays_a_plain_one_leaves_and_a_newer_replaces() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        // The sidebar's poll would ask for its own repaint to the same deadline and mask the
        // notice's; spending the once-per-life auto-switch leaves the expiry the only
        // requester the delay assertion sees.
        app.sidebar.auto_switch_used = true;
        let egui_ctx = test_context();

        // A no-action notice is drawn at t = 1 s and gone at t = 6.1 s.
        ctx.notify(Notice {
            text: "first".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        });
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.0)).nodes;
        assert!(has_text(&nodes, "first"), "the notice is drawn");
        // While the notice is up the expiry asks for the repaint that removes it: at t = 3 s
        // three of its five seconds are left (egui trims the request by the predicted frame
        // time, so the bound is a hair under). The t = 2 s frame is not read: the first
        // frame's own request (the window's theme sync) is still settling there.
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.0));
        let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(3.0));
        let delay = out.repaint_delay.as_secs_f64();
        assert!(
            (2.9..=3.0).contains(&delay),
            "the expiry asks for a repaint at the notice's deadline, got {delay} s"
        );
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.9)).nodes;
        assert!(
            has_text(&nodes, "first"),
            "still drawn 0.1 s short of the deadline"
        );
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(6.1)).nodes;
        assert!(
            !has_text(&nodes, "first"),
            "a notice without an action leaves after 5 s"
        );

        // A later notice replaces the earlier one.
        ctx.notify(Notice {
            text: "second".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        });
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(7.0));
        ctx.notify(Notice {
            text: "third".to_owned(),
            kind: NoticeKind::Info,
            action: None,
        });
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(8.0)).nodes;
        assert!(has_text(&nodes, "third"), "the newer notice shows");
        assert!(!has_text(&nodes, "second"), "the earlier one is replaced");

        // A notice with an action stays past 5 s; clicking the action applies it and
        // empties the slot.
        ctx.notify(Notice {
            text: "ready".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Go".to_owned(),
                effect: ActionEffect::SwitchTool("b".to_owned()),
            }),
        });
        let out = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(14.0));
        let nodes = &out.nodes;
        assert!(has_text(nodes, "ready"));
        entry(nodes, "Go");

        // The middle slot shows the group — text, action, dismiss — centred in
        // the space the sides leave: the left slot's right edge plus item
        // spacing to the bar's right margin, with no activity up.
        let text = bar_text_shape(&out, "ready").expect("the notice text is painted");
        let spacing = f64::from(egui_ctx.global_style().spacing.item_spacing.x);
        let slot_left = left_slot_right(nodes) + spacing;
        let slot_right = 800.0 - 8.0;
        let dismiss = entry(nodes, "Dismiss").bounds().unwrap();
        let centre = (f64::from(text.pos.x) + dismiss.x1) / 2.0;
        assert!(
            (centre - (slot_left + slot_right) / 2.0).abs() <= 4.0,
            "the notice group centres in the space the sides leave {slot_left}..{slot_right}, centre at {centre}"
        );
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(20.0)).nodes;
        assert!(
            has_text(&nodes, "ready"),
            "an action notice does not expire"
        );
        click(&egui_ctx, &mut app, entry(&nodes, "Go"));
        assert_eq!(app.active, 1, "the SwitchTool effect applied");
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(21.0)).nodes;
        assert!(!has_text(&nodes, "ready"), "the click emptied the slot");

        // The dismiss button empties the slot without applying anything.
        ctx.notify(Notice {
            text: "dismiss me".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Go".to_owned(),
                effect: ActionEffect::SwitchTool("a".to_owned()),
            }),
        });
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(22.0)).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Dismiss"));
        assert_eq!(app.active, 1, "dismiss applies nothing");
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(23.0)).nodes;
        assert!(!has_text(&nodes, "dismiss me"), "dismissed");
    }

    // ST-BAR-03
    #[test]
    fn a_notice_too_long_for_the_middle_slot_truncates_its_text_and_the_group_fills_the_slot() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        let egui_ctx = test_context();
        ctx.notify(Notice {
            text: "Although the sidebar is set to PES 2019, this save is from PES 2021, so the editor will keep treating it as a PES 2021 save."
                .to_owned(),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: "Go".to_owned(),
                effect: ActionEffect::SwitchTool("b".to_owned()),
            }),
        });
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

        // The middle slot is what the sides leave: the left slot's right edge
        // plus item spacing to the bar's right margin (800 less its 8 px side
        // margin, `shell::mod`), with no activity up.
        let spacing = f64::from(egui_ctx.global_style().spacing.item_spacing.x);
        let slot_left = left_slot_right(&nodes) + spacing;
        let slot_right = 800.0 - 8.0;
        let text = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|v| v.starts_with("Although"))
            })
            .expect("the notice's label")
            .bounds()
            .unwrap();
        let dismiss = entry(&nodes, "Dismiss").bounds().unwrap();
        // The text is cut to what the action and the X leave, so the group spans the slot
        // edge to edge, neither overflowing it nor leaving it half empty.
        assert!(
            (text.x0 - slot_left).abs() <= 1.0 && (dismiss.x1 - slot_right).abs() <= 1.0,
            "the group spans the slot {slot_left}..{slot_right}: text from {}, X to {}",
            text.x0,
            dismiss.x1
        );
    }

    // ST-BAR-03
    #[test]
    fn a_hundred_character_notice_is_drawn_whole_on_a_1280_point_window() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        let egui_ctx = test_context();
        const NOTICE_TEXT: &str = "Although the sidebar is set to PES 2019, this save is from PES 2021, so the editor will keep treating it as a PES 2021 save.";
        ctx.notify(Notice {
            text: NOTICE_TEXT.to_owned(),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: "Switch to PES 2021".to_owned(),
                effect: ActionEffect::SwitchTool("b".to_owned()),
            }),
        });
        let mut input = raw_input(Vec::new(), true, None, None);
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 800.0),
        ));
        let out = run_frame(&egui_ctx, &mut app, input);

        assert!(
            out.nodes
                .iter()
                .any(|node| { node.role() == Role::Label && node.value() == Some(NOTICE_TEXT) }),
            "the notice's label carries the whole text"
        );
        let shape = bar_text_shape(&out, NOTICE_TEXT).expect("the notice text is painted");
        assert!(
            !shape.galley.elided,
            "the painted galley is the notice's whole text, no ellipsis"
        );
    }

    // ST-BAR-03
    #[test]
    fn the_activity_ends_at_the_right_margin_and_the_notice_stops_before_it() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();
        calls[1].set_activity("B", Some((12, 48)), Duration::from_secs(72));
        ctx.notify(Notice {
            text: "Although the sidebar is set to PES 2019, this save is from PES 2021, so the editor will keep treating it as a PES 2021 save.".to_owned(),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: "Switch to PES 2021".to_owned(),
                effect: ActionEffect::SwitchTool("b".to_owned()),
            }),
        });
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;

        let activity = nodes
            .iter()
            .find(|node| node.value() == Some("B · 12/48 · 01:12"))
            .expect("the second tool's activity is drawn")
            .bounds()
            .unwrap();
        assert!(
            (activity.x1 - (800.0 - 8.0)).abs() <= 1.0,
            "the activity's text ends at the bar's right margin: {activity:?}"
        );
        let dismiss = entry(&nodes, "Dismiss").bounds().unwrap();
        assert!(
            dismiss.x1 <= activity.x0,
            "the notice group stops before the activity {activity:?}: {dismiss:?}"
        );
    }

    // ST-BAR-03's right-slot cap: an activity text wider than a third of
    // the bar is drawn in a slot a third of the bar wide, its right edge
    // on the bar's (the 800 window's margin holds the bar to 8..792).
    #[test]
    fn a_wide_activity_is_capped_at_a_third_of_the_bar() {
        let Shell {
            mut app,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();
        calls[1].set_activity(
            "B and its much longer tool name for the slot cap",
            Some((12, 48)),
            Duration::from_secs(72),
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        let bounds = nodes
            .iter()
            .find(|node| {
                node.value()
                    .is_some_and(|value| value.starts_with("B and its much longer"))
            })
            .expect("the activity label")
            .bounds()
            .unwrap();
        assert!(
            bounds.x1 - bounds.x0 <= 784.0 / 3.0 + 1.0,
            "the slot is a third of the bar at most ({bounds:?})"
        );
        assert!(
            (bounds.x1 - (800.0 - 8.0)).abs() <= 1.0,
            "its right edge is the bar's right edge ({bounds:?})"
        );
    }

    // ST-BAR-03's middle slot, beside an activity: the notice's group is
    // centred in the space the left and right slots leave (the left
    // slot's right edge plus spacing, the right slot's left edge minus
    // it).
    #[test]
    fn the_notice_group_centres_between_the_slots() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();
        calls[1].set_activity("B", Some((12, 48)), Duration::from_secs(72));
        ctx.notify(Notice {
            text: "ready".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Go".to_owned(),
                effect: ActionEffect::SwitchTool("b".to_owned()),
            }),
        });
        let out = frame(&egui_ctx, &mut app, Vec::new(), true);
        let nodes = &out.nodes;

        let activity = nodes
            .iter()
            .find(|node| node.value() == Some("B · 12/48 · 01:12"))
            .expect("the second tool's activity is drawn")
            .bounds()
            .unwrap();
        let text = bar_text_shape(&out, "ready").expect("the notice text is painted");
        let dismiss = entry(nodes, "Dismiss").bounds().unwrap();
        let spacing = f64::from(egui_ctx.global_style().spacing.item_spacing.x);
        let slot_left = left_slot_right(nodes) + spacing;
        let slot_right = activity.x0 - spacing;
        let centre = (f64::from(text.pos.x) + dismiss.x1) / 2.0;
        assert!(
            (centre - (slot_left + slot_right) / 2.0).abs() <= 1.0,
            "the notice group centres in {slot_left}..{slot_right}, centre at {centre}"
        );
    }

    // ST-BAR-04
    #[test]
    fn each_notice_kind_paints_its_text_in_its_own_colour() {
        let Shell {
            mut app, ctx, _dir, ..
        } = shell();
        let egui_ctx = test_context();
        let visuals = egui_ctx.global_style().visuals.clone();
        let cases = [
            (
                NoticeKind::Success,
                "did it",
                crate::theme::Palette::DARK.success,
            ),
            (NoticeKind::Info, "heads up", visuals.hyperlink_color),
            (NoticeKind::Warning, "careful", visuals.warn_fg_color),
        ];
        for (kind, text, expected) in cases {
            ctx.notify(Notice {
                text: text.to_owned(),
                kind,
                action: None,
            });
            let out = frame(&egui_ctx, &mut app, Vec::new(), true);
            let shape = bar_text_shape(&out, text).expect("the notice text is painted");
            let color = shape
                .galley
                .job
                .sections
                .first()
                .map(|section| section.format.color);
            assert_eq!(
                color,
                Some(expected),
                "{kind:?} paints its text in {expected:?}"
            );
        }
    }

    /// A Warning notice `text`, with an `Open` action that only logs when `action`.
    fn warning(text: &str, action: bool) -> Notice {
        Notice {
            text: text.to_owned(),
            kind: NoticeKind::Warning,
            action: action.then(|| NoticeAction {
                label: "Open".to_owned(),
                effect: ActionEffect::OpenFolder(PathBuf::from("out")),
            }),
        }
    }

    /// An Info notice `text` without an action.
    fn info(text: &str) -> Notice {
        Notice {
            text: text.to_owned(),
            kind: NoticeKind::Info,
            action: None,
        }
    }

    // ST-BAR-07
    #[test]
    fn a_standing_notice_stays_while_its_tool_returns_it_and_its_tool_hears_when_it_ends() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();
        let set_standing = |notice: Option<Notice>| *calls[0].standing.lock().unwrap() = notice;

        // "older" is shown, then A's standing notice appears and drops it.
        ctx.notify(info("older"));
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(1.0)).nodes;
        assert!(
            has_text(&nodes, "older"),
            "the one-off notice is drawn first"
        );
        set_standing(Some(warning("standing one", false)));
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(2.0)).nodes;
        assert!(
            has_text(&nodes, "standing one"),
            "the standing notice shows"
        );
        assert!(
            !has_text(&nodes, "older"),
            "it took the slot from the older notice"
        );

        // Past a plain notice's few seconds it is still there.
        let late = 2.0 + super::NOTICE_SECONDS + 1.0;
        frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(late - 0.5));
        let nodes = frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(late)).nodes;
        assert!(
            has_text(&nodes, "standing one"),
            "a standing notice does not expire by time"
        );

        // Reworded, with an action: the new text and its action, and no end told.
        set_standing(Some(warning("standing two", true)));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "standing two"), "the reworded text shows");
        assert!(!has_text(&nodes, "standing one"), "the old wording is gone");
        entry(&nodes, "Open");
        assert_eq!(calls[0].standing_ends(), 0, "a rewording ends nothing");

        // Withdrawn: the slot empties and A is not told.
        set_standing(None);
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, "standing two"),
            "the withdrawn notice is gone"
        );
        assert!(!has_label(&nodes, "Dismiss"), "the middle slot is empty");
        assert_eq!(calls[0].standing_ends(), 0, "a withdrawal ends nothing");

        // B made active while A returns it: from the frame after the click, B's view
        // shows no standing notice, and a notice published then leaves A untold.
        set_standing(Some(warning("standing two", true)));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "standing two"));
        click(&egui_ctx, &mut app, entry(&nodes, "B"));
        assert_eq!(app.active, 1, "the sidebar click made B active");
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, "standing two"),
            "B's view shows no standing notice"
        );
        ctx.notify(info("while away"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "while away"));
        assert_eq!(
            calls[0].standing_ends(),
            0,
            "a notice published while the standing one is not shown ends nothing"
        );

        // A active again: its standing notice is back, and a notice published then
        // replaces it and ends it, once.
        click(&egui_ctx, &mut app, entry(&nodes, "A"));
        assert_eq!(app.active, 0);
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            has_text(&nodes, "standing two"),
            "the standing notice is back"
        );
        assert!(!has_text(&nodes, "while away"), "it took the slot again");
        ctx.notify(info("newer"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "newer"), "the published notice shows");
        assert!(!has_text(&nodes, "standing two"));
        assert_eq!(calls[0].standing_ends(), 1, "A is told once");
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "newer"), "A stopped returning it");
        assert_eq!(calls[0].standing_ends(), 1, "and is not told again");

        // Once for its dismiss clicked.
        set_standing(Some(warning("standing two", true)));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Dismiss"));
        assert_eq!(calls[0].standing_ends(), 2, "the dismiss ends it");
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(!has_text(&nodes, "standing two"), "dismissed");

        // Once for its action clicked.
        set_standing(Some(warning("standing two", true)));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Open"));
        assert_eq!(calls[0].standing_ends(), 3, "the action ends it");
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, "standing two"),
            "the action emptied the slot"
        );

        // A switch and a notice drained in one frame: the switch withdrew the standing
        // notice first, so the notice ends nothing.
        set_standing(Some(warning("standing two", true)));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "standing two"));
        ctx.switch_to_tool("b");
        ctx.notify(info("after the switch"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert_eq!(app.active, 1);
        assert!(has_text(&nodes, "after the switch"));
        assert_eq!(
            calls[0].standing_ends(),
            3,
            "the switch's notice ends nothing"
        );

        // The other order: the notice drained before the switch ends A's notice, and A is
        // the one told, not B, which becomes active in the same drain. A tool made active
        // by a request is first asked on the frame after the drain.
        ctx.switch_to_tool("a");
        frame(&egui_ctx, &mut app, Vec::new(), true);
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "standing two"));
        ctx.notify(info("before the switch"));
        ctx.switch_to_tool("b");
        frame(&egui_ctx, &mut app, Vec::new(), true);
        assert_eq!(app.active, 1);
        assert_eq!(
            (calls[0].standing_ends(), calls[1].standing_ends()),
            (4, 0),
            "the tool whose notice ended is told, not the one switched to"
        );
    }

    // ST-BAR-07
    #[test]
    fn a_standing_notice_never_reaches_the_window_title() {
        let Shell {
            mut app,
            ctx,
            calls,
            _dir,
            ..
        } = shell();
        let egui_ctx = test_context();
        let out = frame(&egui_ctx, &mut app, Vec::new(), false);
        let idle = format!("A - {}", crate::APP_NAME);
        assert_eq!(titles(&out), [idle], "the idle title first");

        // Unfocused, a standing notice shows in the bar and leaves the idle title alone.
        *calls[0].standing.lock().unwrap() = Some(warning("standing one", false));
        let out = frame(&egui_ctx, &mut app, Vec::new(), false);
        assert!(has_text(&out.nodes, "standing one"), "the bar shows it");
        assert!(
            titles(&out).is_empty(),
            "the title stays idle: {:?}",
            titles(&out)
        );

        // An event's notice held in the title stays there when a standing notice takes
        // the bar from it.
        *calls[0].standing.lock().unwrap() = None;
        frame(&egui_ctx, &mut app, Vec::new(), false);
        ctx.notify(info("Compiled 48 teams"));
        let out = frame(&egui_ctx, &mut app, Vec::new(), false);
        let held = format!("Compiled 48 teams - {}", crate::APP_NAME);
        assert_eq!(titles(&out), [held], "the event's notice holds the title");
        *calls[0].standing.lock().unwrap() = Some(warning("standing two", false));
        let out = frame(&egui_ctx, &mut app, Vec::new(), false);
        assert!(has_text(&out.nodes, "standing two"), "the bar shows it");
        assert!(
            titles(&out).is_empty(),
            "the held title is not replaced: {:?}",
            titles(&out)
        );
    }

    // ST-BAR-08
    #[test]
    fn a_switch_version_action_picks_the_version_unless_a_tool_blocks_it() {
        let egui_ctx = test_context();
        let switch = || Notice {
            text: "this save is from PES 2021".to_owned(),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: "Switch to PES 2021".to_owned(),
                effect: ActionEffect::SwitchVersion(PesVersion::Pes21),
            }),
        };

        let dir = TestDir::new("studio_core_status_bar_switch_version");
        let (a, calls_a) = stub("a", "A", None);
        let (b, calls_b) = stub("b", "B", None);
        let mut settings = test_settings(dir.path());
        settings.common.pes_version = PesVersion::Pes19;
        let Shell {
            mut app,
            ctx,
            settings,
            ..
        } = shell_with(
            settings,
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        assert_ne!(
            ctx.common().pes_version,
            PesVersion::Pes21,
            "the action really changes the setting"
        );
        assert!(!app.sidebar.auto_switch_used, "the auto-switch is unspent");
        ctx.notify(switch());
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Switch to PES 2021"));
        assert_eq!(
            settings.lock().unwrap().common.pes_version,
            PesVersion::Pes21
        );
        assert!(app.sidebar.auto_switch_used, "the auto-switch is spent");
        // The save is applied at the next frame's logic.
        frame(&egui_ctx, &mut app, Vec::new(), true);
        let written = Settings::load(&dir.path().join(crate::settings::SETTINGS_FILE_NAME))
            .expect("the switch saved the settings");
        assert_eq!(written.common.pes_version, PesVersion::Pes21);

        // A tool blocking a version change: the action is disabled, its hover the
        // blocker's reason, and a click changes nothing. A fresh context: egui holds a
        // tooltip back while the last click is about as recent as the last pointer move.
        let egui_ctx = test_context();
        egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);
        let dir = TestDir::new("studio_core_status_bar_switch_blocked");
        let (a, calls_a) = stub("a", "A", Some("a compile is running"));
        let (b, calls_b) = stub("b", "B", None);
        let mut settings = test_settings(dir.path());
        settings.common.pes_version = PesVersion::Pes19;
        let Shell {
            mut app,
            ctx,
            settings,
            ..
        } = shell_with(
            settings,
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        ctx.notify(switch());
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        let button = entry(&nodes, "Switch to PES 2021");
        assert!(button.is_disabled(), "the action is disabled");
        frame(&egui_ctx, &mut app, hover(button), true);
        let nodes = idle(&egui_ctx, &mut app, 8).nodes;
        assert!(
            has_text(&nodes, "A: a compile is running"),
            "the hover is the blocking tool's reason"
        );
        click(&egui_ctx, &mut app, entry(&nodes, "Switch to PES 2021"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert_eq!(
            settings.lock().unwrap().common.pes_version,
            PesVersion::Pes19,
            "the click left the setting"
        );
        assert!(
            has_text(&nodes, "this save is from PES 2021"),
            "the notice stays in the bar"
        );
    }

    // ST-SET-03
    #[test]
    fn a_failed_settings_save_raises_the_read_only_condition_and_a_save_drops_it() {
        let egui_ctx = test_context();
        egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

        // A `data_dir` that is a regular file fails the save on every platform.
        let dir = test_dir("studio_core_status_bar_read_only");
        fs::create_dir_all(&dir).unwrap();
        let data_dir = dir.join("data");
        fs::write(&data_dir, b"").unwrap();

        let (a, calls_a) = stub("a", "A", None);
        let mut settings = test_settings(&dir);
        settings.common.pes_version = PesVersion::Pes19;
        let Shell {
            mut app,
            ctx,
            settings,
            ..
        } = shell_with(
            settings,
            vec![a],
            &[],
            Some(data_dir.clone()),
            vec![calls_a],
        );

        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, combo(&nodes));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "PES 2021"));

        // The change applies for the session even though the file cannot be written.
        assert_eq!(ctx.common().pes_version, PesVersion::Pes21);
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert_eq!(
            settings.lock().unwrap().common.pes_version,
            PesVersion::Pes21,
            "the in-memory setting kept the pick"
        );
        assert!(
            bottom_texts(&nodes)
                .iter()
                .any(|text| text.contains("Data folder is read-only")),
            "the condition is in the left slot"
        );
        assert!(
            bottom_texts(&nodes).contains(&crate::icons::WARNING),
            "the entry draws the warning glyph"
        );

        // Hovering the entry shows the tooltip. A hover that follows a click within 0.1 s is
        // suppressed, so the pick's click ages out first; `is_still` then takes about 0.1 s
        // of idle frames.
        let nodes = idle(&egui_ctx, &mut app, 8).nodes;
        let node = nodes
            .iter()
            .find(|node| node.value() == Some("Data folder is read-only"))
            .expect("the condition's text label");
        frame(&egui_ctx, &mut app, hover(node), true);
        let nodes = idle(&egui_ctx, &mut app, 8).nodes;
        assert!(
            has_text(
                &nodes,
                "Settings and the teams list cannot be saved, so changes last for this \
                 session only"
            ),
            "the entry's tooltip explains the session-only setting"
        );

        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.role() == Role::Window)
                .count(),
            1,
            "only the frame's root window node: no dialog opened"
        );

        // The folder made writable, the next save succeeds and the condition drops.
        fs::remove_file(&data_dir).unwrap();
        fs::create_dir(&data_dir).unwrap();
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, combo(&nodes));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "PES 2020"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !bottom_texts(&nodes)
                .iter()
                .any(|text| text.contains("Data folder is read-only")),
            "a save that succeeds drops the condition"
        );
        assert_eq!(ctx.common().pes_version, PesVersion::Pes20);
        fs::remove_dir_all(&dir).unwrap();
    }
}
