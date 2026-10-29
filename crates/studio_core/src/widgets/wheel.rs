//! The mouse wheel over a closed combo box or a drag value steps its value,
//! as the system's own controls do: `wheel_steps` reads the wheel for one
//! control, and `step_choice` and `step_number` turn its steps into the
//! combo's next item or the number's next value.

use std::ops::RangeInclusive;

use egui::emath::Numeric;

/// A notch's worth of a touchpad's small deltas, in points.
const NOTCH: f32 = 50.0;
/// How long, in seconds, a wheel scrolling a pane must rest before a control
/// under the pointer takes it.
const REST: f64 = 0.5;

/// The steps the wheel gives the control `response` belongs to this frame:
/// `+1` for a notch up, `-1` for a notch down, 0 otherwise. A notch is one
/// line of a mouse wheel, or 50 points of a touchpad's small deltas added
/// up. A control takes the wheel only while it is hovered, enabled and, for
/// a combo box, closed; the wheel it takes no scroll area around it scrolls.
/// A wheel that began by scrolling a pane keeps scrolling it while the
/// pointer passes over such controls, until it has rested 500 ms, so
/// scrolling a pane never edits the boxes it sweeps past. One step at most
/// a frame, the rest of a fast turn coming on the next frames: an editor
/// seals its undo entry between frames, so each notch is its own undo step.
/// A notch first takes the keyboard from whatever widget holds it, the
/// control itself included, and steps two frames later: an editor seals
/// only once no widget holds the keyboard, so a step while a field kept it
/// would join that field's undo entry, and a field that commits on losing
/// the keyboard learns of the loss only on the next frame, where a step
/// would join its commit.
/// The caller applies the steps through the control's own change path
/// (`step_choice`, `step_number`).
pub fn wheel_steps(ui: &egui::Ui, response: &egui::Response) -> i32 {
    let ctx = ui.ctx();
    // The context's data is shared by every window it draws, and each
    // window has its own wheel.
    let key = egui::Id::unique(("studio_core wheel gesture", ctx.viewport_id()));
    let mut gesture = ctx
        .data(|data| data.get_temp::<Gesture>(key))
        .unwrap_or_default();
    let steps = gesture.steps(ui, response);
    ctx.data_mut(|data| data.insert_temp(key, gesture));
    if steps != 0 {
        // The control drew its old value this frame, and the rest of a fast
        // turn steps on the next.
        ctx.request_repaint();
    }
    steps
}

/// The wheel's state across frames, one per window.
#[derive(Clone, Copy)]
struct Gesture {
    /// The pass whose wheel input `input_at` and `taker` describe.
    input_pass: u64,
    /// When that input came.
    input_at: f64,
    /// The control that took that input; `None` while it scrolls a pane.
    taker: Option<egui::Id>,
    /// When the wheel last scrolled a pane.
    pane_at: f64,
    /// The taker's turn not stepped yet, in points, up positive.
    pending: f32,
    /// The first pass a step may come on: two after the last pass on which
    /// a due step took the keyboard.
    step_from: u64,
}

impl Default for Gesture {
    fn default() -> Gesture {
        Gesture {
            input_pass: u64::MAX,
            input_at: f64::NEG_INFINITY,
            taker: None,
            pane_at: f64::NEG_INFINITY,
            pending: 0.0,
            step_from: 0,
        }
    }
}

impl Gesture {
    /// `wheel_steps` for this frame, the state updated.
    fn steps(&mut self, ui: &egui::Ui, response: &egui::Response) -> i32 {
        let ctx = ui.ctx();
        let pass = ctx.cumulative_pass_nr();
        let (turn, now) = ui.input(|input| (wheel_turn(input), input.time));
        if turn != 0.0 && self.input_pass != pass {
            // The first control drawn in a frame with wheel input files the
            // last input: one no control took scrolled a pane. Every control
            // drawn calls this, so a frame of pane scrolling is filed as long
            // as some control is on screen.
            if self.taker.is_none() {
                self.pane_at = self.input_at;
                self.pending = 0.0;
            }
            self.input_pass = pass;
            self.input_at = now;
            self.taker = None;
        }
        // egui never reports a disabled widget hovered, so a disabled
        // control leaves the wheel to the pane as well.
        if !response.hovered() || egui::ComboBox::is_open(ctx, response.id) {
            return 0;
        }
        if turn != 0.0 && self.taker.is_none() && now - self.pane_at >= REST {
            self.taker = Some(response.id);
        }
        if self.taker != Some(response.id) {
            return 0;
        }
        if self.input_pass == pass {
            self.pending += turn;
        }
        // egui spreads a mouse notch's scroll over the next frames: the
        // taker keeps every frame's share, so no scroll area moves.
        ctx.input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
        if self.pending.abs() < NOTCH {
            return 0;
        }
        if let Some(focused) = ctx.memory(|memory| memory.focused()) {
            ctx.memory_mut(|memory| memory.surrender_focus(focused));
            self.step_from = pass + 2;
        }
        if pass < self.step_from {
            // The pending turn steps once the delay has passed, with no
            // further input to wake the window.
            ctx.request_repaint();
            return 0;
        }
        if self.pending > 0.0 {
            self.pending -= NOTCH;
            1
        } else {
            self.pending += NOTCH;
            -1
        }
    }
}

/// This frame's wheel turn, in points, up positive. A mouse wheel's line or
/// page counts as one notch: egui's line is 40 points on the desktop, so in
/// points a notch would not step until the second.
fn wheel_turn(input: &egui::InputState) -> f32 {
    input
        .raw
        .events
        .iter()
        .map(|event| match event {
            egui::Event::MouseWheel {
                unit,
                delta,
                phase: egui::TouchPhase::Move,
                ..
            } => match unit {
                egui::MouseWheelUnit::Point => delta.y,
                egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page => delta.y * NOTCH,
            },
            _ => 0.0,
        })
        .sum()
}

/// The value `steps` places from `current` among `values`, a combo box's
/// items in the order its list shows them: a positive step takes an item
/// above, a negative one an item below, stopping at the first and the last.
/// `None` when that is `current` itself, or `current` is not among them.
pub fn step_choice<T: PartialEq>(
    values: impl IntoIterator<Item = T>,
    current: &T,
    steps: i32,
) -> Option<T> {
    let mut values: Vec<T> = values.into_iter().collect();
    let at = values.iter().position(|value| value == current)?;
    let distance = usize::try_from(steps.unsigned_abs()).expect("the targets have a 32-bit usize");
    let target = if steps > 0 {
        at.saturating_sub(distance)
    } else {
        at.saturating_add(distance).min(values.len() - 1)
    };
    (target != at).then(|| values.swap_remove(target))
}

/// Steps the drag value's `value` by `steps` of 1, clamped to `range` as a
/// typed value is; true when the value changed, for the caller to
/// `mark_changed` the control's response.
pub fn step_number<N: Numeric>(value: &mut N, steps: i32, range: RangeInclusive<N>) -> bool {
    if steps == 0 {
        return false;
    }
    let (low, high) = (range.start().to_f64(), range.end().to_f64());
    let stepped = N::from_f64((value.to_f64() + f64::from(steps)).clamp(low, high));
    if stepped == *value {
        return false;
    }
    *value = stepped;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The controls' scroll area: this tall, inside a taller content.
    const VIEW_HEIGHT: f32 = 200.0;
    /// The five items of the test combo.
    const ITEMS: [&str; 5] = ["First", "Second", "Third", "Fourth", "Fifth"];
    /// One frame's time at 60 frames a second.
    const FRAME: f64 = 1.0 / 60.0;

    /// The state the test window edits, and what its last frame showed.
    struct Window {
        ctx: egui::Context,
        time: f64,
        pointer: egui::Pos2,
        text: String,
        value: u8,
        selected: usize,
        drag_enabled: bool,
        offset: f32,
        field_id: egui::Id,
        field_rect: egui::Rect,
        drag_rect: egui::Rect,
        combo_rect: egui::Rect,
        empty_rect: egui::Rect,
    }

    impl Window {
        /// ST-WHEEL-01's window: a drag value at 180 (0 to 255) and a combo
        /// on the second of five items in a scroll area taller than its view,
        /// a text field above the area, drawn once so the controls have their
        /// rects.
        fn new() -> Window {
            let mut window = Window {
                ctx: egui::Context::default(),
                time: 0.0,
                pointer: egui::Pos2::ZERO,
                text: String::new(),
                value: 180,
                selected: 1,
                drag_enabled: true,
                offset: 0.0,
                field_id: egui::Id::NULL,
                field_rect: egui::Rect::NOTHING,
                drag_rect: egui::Rect::NOTHING,
                combo_rect: egui::Rect::NOTHING,
                empty_rect: egui::Rect::NOTHING,
            };
            window.frame(Vec::new());
            window
        }

        /// One frame 1/60 s after the last, with `events` as its input.
        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += FRAME;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 400.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let mut text = std::mem::take(&mut self.text);
            let mut value = self.value;
            let mut selected = self.selected;
            let drag_enabled = self.drag_enabled;
            let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
            let mut offset = 0.0;
            let mut field = (egui::Id::NULL, egui::Rect::NOTHING);
            let output = self.ctx.run_ui(input, |ui| {
                // Above the scroll area, so the area's own layout is the
                // same with the field as without it.
                let response = ui.text_edit_singleline(&mut text);
                field = (response.id, response.rect);
                let shown = egui::ScrollArea::vertical()
                    .max_height(VIEW_HEIGHT)
                    .show(ui, |ui| {
                        // Room above the controls, so a pane turn of two
                        // notches leaves them in view.
                        ui.add_space(100.0);
                        let mut drag = ui.add_enabled(
                            drag_enabled,
                            egui::DragValue::new(&mut value).range(0..=255),
                        );
                        if step_number(&mut value, wheel_steps(ui, &drag), 0..=255) {
                            drag.mark_changed();
                        }
                        let combo = egui::ComboBox::from_id_salt("items")
                            .selected_text(ITEMS[selected])
                            .show_ui(ui, |ui| {
                                for (index, item) in ITEMS.iter().enumerate() {
                                    ui.selectable_value(&mut selected, index, *item);
                                }
                            })
                            .response;
                        if let Some(next) =
                            step_choice(0..ITEMS.len(), &selected, wheel_steps(ui, &combo))
                        {
                            selected = next;
                        }
                        rects = (drag.rect, combo.rect);
                        ui.add_space(1000.0);
                    });
                offset = shown.state.clamped_offset().y;
            });
            output.drop_without_applying_deltas();
            self.text = text;
            (self.field_id, self.field_rect) = field;
            self.value = value;
            self.selected = selected;
            self.offset = offset;
            (self.drag_rect, self.combo_rect) = rects;
            // Below the combo, inside the view: no control there.
            self.empty_rect = egui::Rect::from_min_size(
                egui::pos2(rects.1.left(), rects.1.bottom() + 20.0),
                egui::vec2(50.0, 20.0),
            );
        }

        /// One notch of the wheel (`up` or down) with the pointer at `at`,
        /// then frames until egui's smoothing of it is spent.
        fn notch(&mut self, at: egui::Pos2, up: bool) {
            self.pointer = at;
            self.frame(vec![
                egui::Event::PointerMoved(at),
                wheel(egui::MouseWheelUnit::Line, if up { 1.0 } else { -1.0 }),
            ]);
            self.settle();
        }

        /// Frames with no input for 0.25 s, the wheel's smoothing spent.
        fn settle(&mut self) {
            for _ in 0..15 {
                self.frame(Vec::new());
            }
        }

        /// Frames with no input until `seconds` have passed.
        fn rest(&mut self, seconds: f64) {
            let until = self.time + seconds;
            while self.time < until {
                self.frame(Vec::new());
            }
        }
    }

    /// A wheel event of `delta` in `unit`, up when positive.
    fn wheel(unit: egui::MouseWheelUnit, delta: f32) -> egui::Event {
        egui::Event::MouseWheel {
            unit,
            delta: egui::vec2(0.0, delta),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// The press and release of the primary button at `at`.
    fn click(at: egui::Pos2) -> [Vec<egui::Event>; 2] {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        [
            vec![egui::Event::PointerMoved(at), button(true)],
            vec![button(false)],
        ]
    }

    // ST-WHEEL-01
    #[test]
    fn a_notch_over_a_control_steps_it_and_scrolls_nothing() {
        let mut window = Window::new();
        window.notch(window.drag_rect.center(), true);
        assert_eq!(window.value, 181, "one notch up over the drag value");
        assert_eq!(window.offset, 0.0, "the scroll area took none of the notch");
        window.notch(window.drag_rect.center(), false);
        window.notch(window.drag_rect.center(), false);
        assert_eq!(window.value, 179, "two notches down");
        // At the top a notch up cannot scroll the area anyway; down can.
        assert_eq!(
            window.offset, 0.0,
            "the scroll area took none of the notches down"
        );
        assert_eq!(window.selected, 1, "the combo was not under the pointer");
        window.notch(window.combo_rect.center(), true);
        assert_eq!(window.selected, 0, "one notch up takes the item above");
        window.notch(window.combo_rect.center(), true);
        assert_eq!(window.selected, 0, "the first item stays: no wrap");
        assert_eq!(window.value, 179);
        assert_eq!(window.offset, 0.0, "no notch scrolled the area");
    }

    // ST-WHEEL-01
    #[test]
    fn two_notches_in_one_frame_step_twice_over_two_frames() {
        let mut window = Window::new();
        let at = window.drag_rect.center();
        window.frame(vec![
            egui::Event::PointerMoved(at),
            wheel(egui::MouseWheelUnit::Line, -2.0),
        ]);
        assert_eq!(window.value, 179, "the first notch steps on its own frame");
        window.frame(Vec::new());
        assert_eq!(window.value, 178, "the second on the next");
        window.settle();
        assert_eq!(window.value, 178);
        assert_eq!(window.offset, 0.0);
    }

    // ST-WHEEL-02
    #[test]
    fn a_wheel_that_began_over_the_pane_keeps_scrolling_until_it_rests() {
        let mut window = Window::new();
        let line = window
            .ctx
            .options(|options| options.input_options.line_scroll_speed);
        window.notch(window.empty_rect.center(), false);
        assert!(
            (window.offset - line).abs() < 0.5,
            "the first turn scrolled the pane: offset {}",
            window.offset
        );
        // 0.25 s of smoothing has passed: the second turn comes under 500 ms
        // after the first.
        window.notch(window.drag_rect.center(), false);
        assert_eq!(window.value, 180, "the pane's wheel left the drag value");
        assert!(
            (window.offset - 2.0 * line).abs() < 0.5,
            "the area moved by both turns: offset {}",
            window.offset
        );
        window.rest(0.5);
        window.notch(window.drag_rect.center(), true);
        assert_eq!(window.value, 181, "after a 500 ms rest the wheel steps it");
        assert!((window.offset - 2.0 * line).abs() < 0.5);
    }

    // ST-WHEEL-03
    #[test]
    fn a_disabled_drag_value_and_an_open_combo_ignore_the_wheel() {
        let mut window = Window::new();
        window.drag_enabled = false;
        window.frame(Vec::new());
        window.notch(window.drag_rect.center(), true);
        assert_eq!(window.value, 180, "the disabled drag value stays");
        window.rest(0.5);
        for events in click(window.combo_rect.center()) {
            window.frame(events);
        }
        assert!(
            egui::Popup::is_any_open(&window.ctx),
            "the click opened the combo's list"
        );
        window.notch(window.combo_rect.center(), true);
        assert_eq!(window.selected, 1, "the open combo's selection stays");
    }

    // ST-WHEEL-04
    #[test]
    fn a_notch_takes_the_keyboard_from_a_field_and_steps_two_frames_later() {
        let mut window = Window::new();
        for events in click(window.field_rect.center()) {
            window.frame(events);
        }
        let field_focused = |window: &Window| window.ctx.memory(|m| m.has_focus(window.field_id));
        assert!(
            field_focused(&window),
            "premise: the field holds the keyboard"
        );
        let at = window.drag_rect.center();
        window.frame(vec![
            egui::Event::PointerMoved(at),
            wheel(egui::MouseWheelUnit::Line, 1.0),
        ]);
        assert!(!field_focused(&window), "the notch took the keyboard");
        assert_eq!(window.value, 180, "no step on the notch's frame");
        window.frame(Vec::new());
        assert_eq!(window.value, 180, "nor on the next");
        window.frame(Vec::new());
        assert_eq!(window.value, 181, "the step comes on the frame after");
        window.settle();
        assert_eq!(window.value, 181, "one notch, one step");
        assert_eq!(window.offset, 0.0);
    }

    // ST-WHEEL-04: the stepped control itself in typing mode leaves it, and
    // the step lands on the value typed. egui's drag value commits each
    // keystroke while typing, so the typed value holds before the notch.
    #[test]
    fn a_notch_over_a_drag_value_being_typed_in_ends_typing_then_steps() {
        let mut window = Window::new();
        let at = window.drag_rect.center();
        for events in click(at) {
            window.frame(events);
        }
        // The field was never clicked: whatever holds the keyboard is the
        // drag value.
        let typing = |window: &Window| window.ctx.memory(|m| m.focused()).is_some();
        assert!(typing(&window), "premise: the click started typing");
        window.frame(vec![egui::Event::Text("200".to_owned())]);
        assert_eq!(window.value, 200, "premise: the typed value took");
        window.frame(vec![
            egui::Event::PointerMoved(at),
            wheel(egui::MouseWheelUnit::Line, 1.0),
        ]);
        assert!(!typing(&window), "the notch ended typing");
        assert_eq!(window.value, 200, "no step on the notch's frame");
        window.frame(Vec::new());
        assert_eq!(window.value, 200, "nor on the next");
        window.frame(Vec::new());
        assert_eq!(window.value, 201, "then the step");
        window.settle();
        assert_eq!(window.value, 201, "one notch, one step");
        assert_eq!(window.offset, 0.0);
    }

    #[test]
    fn a_touchpads_small_deltas_step_once_per_notchs_worth() {
        let mut window = Window::new();
        let at = window.drag_rect.center();
        for index in 0..10 {
            assert_eq!(
                window.value, 180,
                "no step before 50 points: {index} deltas"
            );
            window.frame(vec![
                egui::Event::PointerMoved(at),
                wheel(egui::MouseWheelUnit::Point, 5.0),
            ]);
        }
        assert_eq!(window.value, 181, "ten deltas of 5 points step once");
        window.settle();
        assert_eq!(window.value, 181);
        assert_eq!(window.offset, 0.0);
    }

    #[test]
    fn a_choice_steps_to_its_neighbour_and_stops_at_the_ends() {
        assert_eq!(step_choice(["a", "b", "c"], &"b", 1), Some("a"));
        assert_eq!(step_choice(["a", "b", "c"], &"b", -1), Some("c"));
        assert_eq!(step_choice(["a", "b", "c"], &"a", 1), None);
        assert_eq!(step_choice(["a", "b", "c"], &"c", -1), None);
        assert_eq!(step_choice(["a", "b", "c"], &"a", -5), Some("c"));
        assert_eq!(step_choice(["a", "b", "c"], &"b", 0), None);
        assert_eq!(step_choice(["a", "b", "c"], &"z", 1), None);
    }

    #[test]
    fn a_number_steps_by_one_inside_its_range() {
        let mut value = 7u8;
        assert!(step_number(&mut value, 1, 0..=8));
        assert_eq!(value, 8);
        assert!(!step_number(&mut value, 1, 0..=8), "the top holds");
        assert_eq!(value, 8);
        assert!(step_number(&mut value, -3, 0..=8));
        assert_eq!(value, 5);
        let mut past = 48u8;
        assert!(
            step_number(&mut past, -1, 0..=20),
            "a value past the range lands in it"
        );
        assert_eq!(past, 20);
        let mut percent = 50.0f64;
        assert!(step_number(&mut percent, 1, 0.1..=100.0));
        assert_eq!(percent, 51.0);
        let mut zero = 0u8;
        assert!(!step_number(&mut zero, 0, 0..=8));
    }
}
