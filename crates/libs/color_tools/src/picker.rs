//! The suite's color field: a color button opening the picker popup drafted in
//! `docs/prototypes/tools/save_editor.js` ("The colour picker" section). The button shows the color
//! (or the live candidate while a swatch is hovered or a drag runs); the popup edits it
//! through a saturation-value square and a hue strip, a hex field, R/G/B fields, the
//! host's suggestion swatches and the session's recent colors. `color` moves only on a
//! finished gesture, so the host's undo takes one step per gesture.

use egui::ecolor::Hsva;
use egui::emath::{lerp, remap_clamp};
use egui::{
    Align, Color32, DragValue, Key, Layout, Mesh, Popup, PopupCloseBehavior, Rect, Response, Role,
    Sense, Shape, Stroke, StrokeKind, TextEdit, TextStyle, Ui, WidgetInfo, pos2, vec2,
};
use studio_core::widgets::{step_number, wheel_steps};

/// The most recent colors the session keeps: newest first, no duplicates.
const MAX_RECENTS: usize = 8;

/// What one frame of a color field reports to its host.
pub struct ColorFieldOutput {
    /// The color button's response.
    pub response: egui::Response,
    /// A finished gesture changed the color this frame: a released drag on the square, the
    /// hue strip or an R/G/B field, a committed hex text or R/G/B value, a swatch click. True
    /// once per gesture, so the host's undo takes one step per gesture.
    pub changed: bool,
    /// The color a hovered swatch or a running drag would set, for the host's live preview;
    /// `None` when nothing is previewed.
    pub candidate: Option<[u8; 3]>,
}

/// A color button that opens the suite's picker popup. `color` changes only on a
/// finished gesture.
pub fn color_field(
    ui: &mut Ui,
    id_salt: impl egui::AsIdSalt,
    color: &mut [u8; 3],
    suggestions: &[[u8; 3]],
) -> ColorFieldOutput {
    ui.push_id(id_salt, |ui| field(ui, color, suggestions))
        .inner
}

/// The field over the pushed `id_salt` scope: the button, the popup, the open bookkeeping
/// and the recent-colors rule on close.
fn field(ui: &mut Ui, color: &mut [u8; 3], suggestions: &[[u8; 3]]) -> ColorFieldOutput {
    let popup_id = ui.make_persistent_id("picker popup");
    let state_id = ui.make_persistent_id("picker state");
    let was_open = Popup::is_id_open(ui.ctx(), popup_id);
    let mut changed = false;
    let mut state = ui
        .ctx()
        .data(|data| data.get_temp::<PickerState>(state_id))
        .unwrap_or_else(|| PickerState::new(*color));
    if !was_open {
        // egui keeps a single popup per viewport: a sibling field's popup opening, or a
        // frame where this field was not drawn, closed this one between frames, and the
        // close rules apply as if it had closed here.
        if state.open {
            let working = rgb_of(state.hsva);
            commit_pending_text(&mut state, color, &mut changed, working);
            if *color != state.opened_on {
                push_recent(ui.ctx(), *color);
            }
        }
        state = PickerState::new(*color);
    }
    if *color != state.shown {
        // The host moved the color under the popup. When the last drawn frame committed,
        // this is the host rounding that pick back (the Save editor's 6 bits): feedback,
        // so `opened_on` stays and the pick still joins the recents on close. Otherwise
        // it is a host edit (an undo, a palette apply): the working state reseeds and
        // the change is not a pick, so `opened_on` moves and it cannot join the recents.
        // The hex text follows `hsva` in the body unless the member is typing in it.
        state.hsva = hsva_of(*color);
        if !state.committed {
            state.opened_on = *color;
        }
    }

    let mut response = color_button(ui, state.preview.unwrap_or(*color), was_open);
    Popup::from_toggle_button_response(&response)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .layout(Layout::top_down_justified(Align::Min))
        .show(|ui| {
            popup_body(ui, popup_id, &mut state, color, &mut changed, suggestions);
        });

    let is_open = Popup::is_id_open(ui.ctx(), popup_id);
    if was_open && !is_open {
        let working = rgb_of(state.hsva);
        commit_pending_text(&mut state, color, &mut changed, working);
        if *color != state.opened_on {
            push_recent(ui.ctx(), *color);
        }
    }
    if changed {
        response.mark_changed();
    }
    state.open = is_open;
    state.shown = *color;
    state.committed = changed;
    let candidate = state.preview;
    ui.ctx().data_mut(|data| data.insert_temp(state_id, state));
    ColorFieldOutput {
        response,
        changed,
        candidate,
    }
}

/// A typed text still in the hex field or a channel field when a gesture takes over:
/// the settle at `popup_body`'s top (a press outside the field), a toggle through the
/// field's own button, or a close that lands between frames — any close but Escape
/// commits it, like focus loss would; the channel's typed value is already in the
/// working color.
fn commit_pending_text(
    state: &mut PickerState,
    color: &mut [u8; 3],
    changed: &mut bool,
    working: [u8; 3],
) {
    if state.hex_focused
        && let Some(parsed) = parse_hex(&state.hex)
    {
        commit_color(state, color, changed, parsed);
    }
    if state.rgb_focused {
        commit_color(state, color, changed, working);
    }
}

/// The field's face: a button filled with `rgb` (`open` draws egui's open-widget style
/// while its popup is up). Its label carries the shown hex so a screen reader and the
/// tests can tell fields apart.
fn color_button(ui: &mut Ui, rgb: [u8; 3], open: bool) -> Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| {
        WidgetInfo::labeled(
            Role::ColorWell,
            ui.is_enabled(),
            format!("color field {}", format_hex(rgb)),
        )
    });
    if ui.is_rect_visible(rect) {
        let visuals = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);
        ui.painter().rect_filled(
            rect.shrink(1.0),
            visuals.corner_radius.at_most(2),
            color32(rgb),
        );
        ui.painter().rect_stroke(
            rect,
            visuals.corner_radius.at_most(2),
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
    }
    response
}

/// `#RRGGBB` upper-case, matching the export text formats: how the hex field shows the
/// color and what a swatch's hover text and label are.
fn format_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// `#RRGGBB` parsed back: the `#` optional, either case, surrounding space ignored; any
/// other text is `None` (the field is put back to the current color).
fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut rgb = [0; 3];
    for (index, channel) in rgb.iter_mut().enumerate() {
        *channel = u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(rgb)
}

/// The recent-colors rule on the recents vector itself: `color` moves to the front (no
/// duplicates) and the row never grows past `MAX_RECENTS`.
fn push_recent_color(recents: &mut Vec<[u8; 3]>, color: [u8; 3]) {
    recents.retain(|&existing| existing != color);
    recents.insert(0, color);
    recents.truncate(MAX_RECENTS);
}

/// The egui id the session's recent colors live under: temp memory (never saved), shared
/// by every field of the program.
const RECENTS_ID: &str = "4cc studio color_tools recent colors";

/// The recents row as it stands in the context's temp memory.
fn recent_colors(ctx: &egui::Context) -> Vec<[u8; 3]> {
    ctx.data(|data| data.get_temp(egui::Id::unique(RECENTS_ID)))
        .unwrap_or_default()
}

/// `color` joins the session's recents (called when a popup closes on a color other than
/// the one it opened on).
fn push_recent(ctx: &egui::Context, color: [u8; 3]) {
    ctx.data_mut(|data| {
        let recents = data.get_temp_mut_or_default::<Vec<[u8; 3]>>(egui::Id::unique(RECENTS_ID));
        push_recent_color(recents, color);
    });
}

/// `[u8; 3]` sRGB as the working hsv: the formula is the same in every color space, h,
/// s and v are parameters of it, not colors. `hsv_from_rgb` on the sRGB fractions gives
/// the triple the fields and the gradients all share.
fn hsva_of(rgb: [u8; 3]) -> Hsva {
    let (h, s, v) = egui::ecolor::hsv_from_rgb([
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
    ]);
    Hsva { h, s, v, a: 1.0 }
}

/// The working hsv as `[u8; 3]` sRGB, as the draft's `hsvToRgb` computes it: HSV taken
/// over sRGB values, the formula's output used as the sRGB value, not re-encoded. So a
/// pick equals the color the gradients show, which a renderer interpolates as sRGB.
fn rgb_of(hsva: Hsva) -> [u8; 3] {
    let [r, g, b] = egui::ecolor::rgb_from_hsv((hsva.h, hsva.s, hsva.v));
    [channel(r), channel(g), channel(b)]
}

/// `fraction` of 1.0 as an sRGB byte.
fn channel(fraction: f32) -> u8 {
    (fraction.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The working hsv as `Color32` for vertex colors and swatch fills.
fn srgb_of(hsva: Hsva) -> Color32 {
    let [r, g, b] = rgb_of(hsva);
    Color32::from_rgb(r, g, b)
}

/// `rgb` as `Color32`.
fn color32(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// The open popup's working state, kept in egui temp memory under the field's id so
/// typing, drags and the close rule survive the frames they span.
#[derive(Clone)]
struct PickerState {
    /// The color the popup opened on: the recent-colors rule compares the close color to
    /// it.
    opened_on: [u8; 3],
    /// The working color: what the square, the strip and the fields edit until a gesture
    /// commits it.
    hsva: Hsva,
    /// The hex field's text, kept while it is being typed.
    hex: String,
    /// The preview the button face and the host show right now (a hovered swatch or a
    /// running drag); `None` when nothing is previewed.
    preview: Option<[u8; 3]>,
    /// A press on the square, the strip or an R/G/B field is still held: the working color
    /// is only a candidate until it releases.
    dragging: bool,
    /// Whether the popup was open at the end of the last frame drawn: egui keeps one popup
    /// per viewport, so a sibling field's popup opening, or a frame where this field is not
    /// drawn, closes this one between frames.
    open: bool,
    /// The hex field had keyboard focus the last frame the popup's body drew: a typed
    /// text then commits when the popup closes another way (the field's own button, a
    /// sibling popup), as losing focus would have committed it.
    hex_focused: bool,
    /// An R, G or B field had keyboard focus the last frame the popup's body drew: its
    /// typed value commits on the same closes as the hex text.
    rgb_focused: bool,
    /// The color the field showed at the end of the last frame drawn: anything different
    /// now is the host moving the color (an undo, a palette apply), not a pick.
    shown: [u8; 3],
    /// Whether the last frame drawn committed a pick: the host's rounding of that pick,
    /// arriving as a color change on the next frame, is feedback and not an undo, so it
    /// must not move `opened_on`.
    committed: bool,
    /// The text field's rect as of the last frame drawn (the hex field's or a channel's,
    /// whichever has keyboard focus): a press landing outside it settles its text before
    /// the gesture runs.
    text_rect: Option<Rect>,
}

impl PickerState {
    /// The state of a popup opening on `color`.
    fn new(color: [u8; 3]) -> PickerState {
        PickerState {
            opened_on: color,
            hsva: hsva_of(color),
            hex: format_hex(color),
            preview: None,
            dragging: false,
            open: false,
            hex_focused: false,
            rgb_focused: false,
            shown: color,
            committed: false,
            text_rect: None,
        }
    }
}

/// The square's width; the draft's `PK_W` (`docs/prototypes/tools/save_editor.js`).
const POPUP_W: f32 = 168.0;
/// The square's height (the draft's `PK_SV`).
const SQUARE_H: f32 = 120.0;
/// The hue strip's height (the draft's `PK_HUE`).
const STRIP_H: f32 = 14.0;

/// The saturation-value square (egui's `color_slider_2d`, drawn the same way: a
/// vertex-colored mesh): `x` is saturation, `y` is value, `interact_pointer_pos` edits
/// `hsva` while a press or a drag is held.
fn sv_square(ui: &mut Ui, hsva: &mut Hsva) -> Response {
    let (rect, response) = ui.allocate_at_least(vec2(POPUP_W, SQUARE_H), Sense::click_and_drag());
    response
        .widget_info(|| WidgetInfo::labeled(Role::Slider, ui.is_enabled(), "saturation and value"));
    if let Some(pos) = response.interact_pointer_pos() {
        hsva.s = remap_clamp(pos.x, rect.left()..=rect.right(), 0.0..=1.0);
        hsva.v = remap_clamp(pos.y, rect.bottom()..=rect.top(), 0.0..=1.0);
    }
    if ui.is_rect_visible(rect) {
        // In sRGB terms each channel is v·(a + b·s), bilinear in (s, v), so a small grid
        // already matches the gradient a click picks; egui's own square samples its
        // gamma-space variant more densely, which is not bilinear.
        const SUBDIVISIONS: u32 = 8;
        let color_at = |s: f32, v: f32| -> Color32 {
            srgb_of(Hsva {
                h: hsva.h,
                s,
                v,
                a: 1.0,
            })
        };
        let mut mesh = Mesh::default();
        for xi in 0..=SUBDIVISIONS {
            for yi in 0..=SUBDIVISIONS {
                let (x, y) = (
                    xi as f32 / SUBDIVISIONS as f32,
                    yi as f32 / SUBDIVISIONS as f32,
                );
                mesh.colored_vertex(
                    pos2(
                        lerp(rect.left()..=rect.right(), x),
                        lerp(rect.bottom()..=rect.top(), y),
                    ),
                    color_at(x, y),
                );
            }
        }
        for xi in 0..SUBDIVISIONS {
            for yi in 0..SUBDIVISIONS {
                let tl = yi * (SUBDIVISIONS + 1) + xi;
                mesh.add_triangle(tl, tl + 1, tl + SUBDIVISIONS + 1);
                mesh.add_triangle(tl + 1, tl + SUBDIVISIONS + 2, tl + SUBDIVISIONS + 1);
            }
        }
        ui.painter().add(Shape::mesh(mesh));
        ui.painter().rect_stroke(
            rect,
            0.0,
            ui.style().interact(&response).bg_stroke,
            StrokeKind::Inside,
        );
        // The ring marks the working color's point, as the draft draws it.
        let center = pos2(
            lerp(rect.left()..=rect.right(), hsva.s),
            lerp(rect.bottom()..=rect.top(), hsva.v),
        );
        ui.painter()
            .circle_stroke(center, 4.0, Stroke::new(3.0, Color32::BLACK));
        ui.painter()
            .circle_stroke(center, 4.0, Stroke::new(1.0, Color32::WHITE));
    }
    response
}

/// The hue strip (egui's `color_slider_1d` for hue): `x` is hue, `interact_pointer_pos`
/// edits `hsva.h` while a press or a drag is held.
fn hue_strip(ui: &mut Ui, hsva: &mut Hsva) -> Response {
    let (rect, response) = ui.allocate_at_least(vec2(POPUP_W, STRIP_H), Sense::click_and_drag());
    response.widget_info(|| WidgetInfo::labeled(Role::Slider, ui.is_enabled(), "hue"));
    if let Some(pos) = response.interact_pointer_pos() {
        hsva.h = remap_clamp(pos.x, rect.left()..=rect.right(), 0.0..=1.0);
    }
    if ui.is_rect_visible(rect) {
        // The hue wheel's six sectors are linear in RGB within each, so seven vertex
        // columns render the strip exactly.
        let mut mesh = Mesh::default();
        for sector in 0..=6_u32 {
            let h = sector as f32 / 6.0;
            let color: Color32 = srgb_of(Hsva {
                h,
                s: 1.0,
                v: 1.0,
                a: 1.0,
            });
            let x = lerp(rect.left()..=rect.right(), h);
            mesh.colored_vertex(pos2(x, rect.top()), color);
            mesh.colored_vertex(pos2(x, rect.bottom()), color);
        }
        for sector in 0..6_u32 {
            let left = 2 * sector;
            mesh.add_triangle(left, left + 1, left + 2);
            mesh.add_triangle(left + 1, left + 3, left + 2);
        }
        ui.painter().add(Shape::mesh(mesh));
        ui.painter().rect_stroke(
            rect,
            0.0,
            ui.style().interact(&response).bg_stroke,
            StrokeKind::Inside,
        );
        // The draft's 3px white bar with a black edge marks the hue's x.
        let x = lerp(rect.left()..=rect.right(), hsva.h);
        let marker =
            Rect::from_center_size(pos2(x, rect.center().y), vec2(3.0, rect.height() + 2.0));
        ui.painter().rect_filled(marker, 0.0, Color32::WHITE);
        ui.painter()
            .rect_stroke(marker, 0.0, (1.0, Color32::BLACK), StrokeKind::Inside);
    }
    response
}

/// One swatch in a suggestion or recents row: a small square of `rgb` that previews on
/// hover; a click commits it and closes the popup. Its label is its hex.
fn color_swatch(ui: &mut Ui, rgb: [u8; 3]) -> Response {
    let size = vec2(ui.spacing().interact_size.y, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::ColorWell, ui.is_enabled(), format_hex(rgb)));
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        ui.painter().rect_filled(
            rect.shrink(1.0),
            visuals.corner_radius.at_most(2),
            color32(rgb),
        );
        ui.painter().rect_stroke(
            rect,
            visuals.corner_radius.at_most(2),
            visuals.bg_stroke,
            StrokeKind::Inside,
        );
    }
    response
}

/// A finished gesture lands: `color` moves once, and the working hsv is rebuilt from
/// the committed RGB, the right reset for a typed text, a swatch or a host edit. A commit
/// naming the color already held changes nothing at all (a text that parses to the
/// current color has nothing to reset).
fn commit_color(state: &mut PickerState, color: &mut [u8; 3], changed: &mut bool, c: [u8; 3]) {
    if c != *color {
        *color = c;
        *changed = true;
        state.hsva = hsva_of(c);
    }
}

/// A gesture release on the square, the strip or an R/G/B field lands: one commit for the
/// whole gesture. The working hsv is *kept* rather than rebuilt from the committed RGB:
/// white, gray and black hold no hue, so rebuilding would drop a hue the member just
/// chose on the strip before moving into the square.
fn commit_working(state: &mut PickerState, color: &mut [u8; 3], changed: &mut bool) {
    let working = rgb_of(state.hsva);
    if working != *color {
        *color = working;
        *changed = true;
    }
}

/// One row of swatches (suggestions or recents): each previews into `hovered` while the
/// pointer is on it; a click commits its color and closes the popup with it (egui's own
/// popup close, so the field's ordinary close rules apply), reusing a color being the
/// rows' common case.
fn swatch_row(
    ui: &mut Ui,
    popup_id: egui::Id,
    swatches: &[[u8; 3]],
    hovered: &mut Option<[u8; 3]>,
    state: &mut PickerState,
    color: &mut [u8; 3],
    changed: &mut bool,
) {
    for &swatch in swatches {
        let response = color_swatch(ui, swatch).on_hover_text(format_hex(swatch));
        if response.hovered() {
            *hovered = Some(swatch);
        }
        if response.clicked() {
            commit_color(state, color, changed, swatch);
            Popup::close_id(ui.ctx(), popup_id);
        }
    }
}

/// The popup's contents, top to bottom as the draft lays them out: the square and the
/// strip, the hex field, the R/G/B fields, the host's suggestions, the session's recents.
/// The gesture rule: a press on the square, the strip or an R/G/B field previews while
/// held and commits once on release. A typed value, a committed hex text and a swatch
/// click are one change each. A swatch click also closes the popup (`popup_id`); the
/// square, the strip and the fields keep it open, being the fine-tuning path.
fn popup_body(
    ui: &mut Ui,
    popup_id: egui::Id,
    state: &mut PickerState,
    color: &mut [u8; 3],
    changed: &mut bool,
    suggestions: &[[u8; 3]],
) {
    // Settle the text before anything else draws: a field focused on the last frame,
    // and a primary press this frame outside its rect, commits its pending text and
    // loses the focus, the draft's mousedown order. One step serving a tap or drag on
    // the square or the strip, a swatch click, and a press on another field.
    let working_at_start = rgb_of(state.hsva);
    if (state.hex_focused || state.rgb_focused)
        && let Some(rect) = state.text_rect
        && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary))
        && ui
            .input(|input| {
                input
                    .pointer
                    .press_origin()
                    .or_else(|| input.pointer.interact_pos())
            })
            .is_some_and(|pos| !rect.contains(pos))
    {
        commit_pending_text(state, color, changed, working_at_start);
        if let Some(id) = ui.memory(|memory| memory.focused()) {
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }
    }

    let sv = sv_square(ui, &mut state.hsva);
    let hue = hue_strip(ui, &mut state.hsva);
    let mut down = sv.is_pointer_button_down_on() || hue.is_pointer_button_down_on();

    // The hex field shows the working color while it is not being typed in (a drag
    // mirrors into it); on Enter or focus loss its text commits, and a text that does
    // not read as `#RRGGBB` is put back. Escape cancels the edit instead (the popup is
    // closing anyway, as in the draft where the field is already gone on blur).
    let hex_id = ui.make_persistent_id("hex");
    if !ui.memory(|memory| memory.has_focus(hex_id)) {
        state.hex = format_hex(rgb_of(state.hsva));
    }
    let hex_response = ui.add(
        TextEdit::singleline(&mut state.hex)
            .id(hex_id)
            .font(TextStyle::Monospace)
            .desired_width(f32::INFINITY),
    );
    let hex_lost_focus = hex_response.lost_focus();
    state.hex_focused = ui.memory(|memory| memory.has_focus(hex_id));

    // R/G/B fields: typing moves the working color the popup shows (`update_while_editing`'s
    // default; the host's own candidate is the preview in the gesture tail), and commits on
    // Enter or focus loss (or through `commit_pending_text` when the popup closes first); a
    // drag only previews until its release lands in the gesture below.
    let mut channels = rgb_of(state.hsva);
    let mut channel_changed = false;
    let mut channel_wheeled = false;
    let mut channel_focused = false;
    let mut channel_lost_focus = false;
    // Focus reads egui's memory, not `Response::has_focus`, which is also false
    // while the window is unfocused: an Alt+Tab mid-edit would drop `text_rect`
    // and `rgb_focused`, and with them the settle that commits the typed text.
    let mut text_rect = ui
        .memory(|memory| memory.has_focus(hex_id))
        .then_some(hex_response.rect);
    ui.horizontal(|ui| {
        for (index, prefix) in ["R ", "G ", "B "].iter().enumerate() {
            let mut channel = channels[index];
            let response = ui.add(DragValue::new(&mut channel).speed(0.5).prefix(*prefix));
            let wheeled = step_number(&mut channel, wheel_steps(ui, &response), 0..=u8::MAX);
            channels[index] = channel;
            channel_changed |= response.changed() || wheeled;
            channel_wheeled |= wheeled;
            channel_focused |= ui.memory(|memory| memory.has_focus(response.id));
            channel_lost_focus |= response.lost_focus();
            // A field in text-editing mode is not a gesture control: clicks and
            // drag-selects inside it move the caret, not the color.
            if ui.memory(|memory| memory.has_focus(response.id)) {
                text_rect = Some(response.rect);
            } else {
                down |= response.is_pointer_button_down_on();
            }
        }
    });
    state.rgb_focused = channel_focused;
    state.text_rect = text_rect;
    if channel_changed {
        state.hsva = hsva_of(channels);
    }
    // A wheel step is one finished change, as a typed value is.
    if channel_wheeled {
        commit_color(state, color, changed, channels);
    }
    // A text field's `lost_focus` commits only when no gesture is down this frame:
    // egui's second `lost_focus` frame while a press is held must not commit the
    // candidate early (the settle at the top already did). And only when no channel
    // holds the focus now: Tab between the fields is one edit, so R's blur must not
    // commit G's unfinished text; the last focus loss commits all three at once.
    if channel_lost_focus
        && !ui.input(|input| input.key_pressed(Key::Escape))
        && !down
        && !channel_focused
    {
        commit_color(state, color, changed, channels);
    }
    if hex_lost_focus && !ui.input(|input| input.key_pressed(Key::Escape)) && !down {
        if let Some(parsed) = parse_hex(&state.hex) {
            commit_color(state, color, changed, parsed);
        }
        state.hex = format_hex(rgb_of(state.hsva));
    }

    let mut hovered = None;
    if !suggestions.is_empty() {
        ui.horizontal(|ui| {
            swatch_row(
                ui,
                popup_id,
                suggestions,
                &mut hovered,
                state,
                color,
                changed,
            );
        });
    }
    let recents = recent_colors(ui.ctx());
    if !recents.is_empty() {
        ui.horizontal(|ui| {
            swatch_row(ui, popup_id, &recents, &mut hovered, state, color, changed);
        });
    }

    // A press still held previews; a release after one (`state.dragging`) and a click
    // completed in a single frame (`clicked()`) each commit the working color once,
    // unless Escape cancelled the gesture (`commit_working` keeps the working hsv, so a
    // hue chosen on a hueless color survives).
    if down {
        state.preview = Some(rgb_of(state.hsva));
    } else {
        state.preview = hovered;
        if (state.dragging || sv.clicked() || hue.clicked())
            && !ui.input(|input| input.key_pressed(Key::Escape))
        {
            commit_working(state, color, changed);
        }
    }
    state.dragging = down;
}

#[cfg(test)]
mod tests;
