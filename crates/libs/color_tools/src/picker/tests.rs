use egui::accesskit::{self, Role};

use super::*;

/// A headless window: one egui context with AccessKit on, run a frame at a time
/// over an 800 by 600 screen (the `studio_core` log panel's harness, for a bare
/// `Ui`).
struct Headless {
    ctx: egui::Context,
    /// The last frame's paint shapes (`FullOutput::shapes`), kept for assertions.
    shapes: Vec<egui::epaint::ClippedShape>,
}

impl Headless {
    fn new() -> Headless {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // The window draws with these fonts and style; the test measures
        // what a member sees, whichever crates the build unifies.
        studio_core::theme::install(&ctx);
        Headless {
            ctx,
            shapes: Vec::new(),
        }
    }

    /// Runs one frame with `events` as its input; returns the frame's AccessKit
    /// nodes.
    fn frame(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Vec<accesskit::Node> {
        self.run(events, true, contents)
    }

    /// One frame with the window unfocused (`focused` off in `RawInput`): an
    /// Alt+Tab mid-edit.
    fn unfocused_frame(
        &mut self,
        events: Vec<egui::Event>,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Vec<accesskit::Node> {
        self.run(events, false, contents)
    }

    /// Runs one frame with `events` as its input and `focused` as the window's
    /// focus; returns the frame's AccessKit nodes.
    fn run(
        &mut self,
        events: Vec<egui::Event>,
        focused: bool,
        contents: impl FnMut(&mut egui::Ui),
    ) -> Vec<accesskit::Node> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            focused,
            ..Default::default()
        };
        let mut output = self.ctx.run_ui(input, contents);
        let update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit is enabled on the context");
        self.shapes = std::mem::take(&mut output.shapes);
        // Nothing renders here, so the frame's texture deltas are dropped on
        // purpose.
        output.drop_without_applying_deltas();
        update.nodes.into_iter().map(|(_, node)| node).collect()
    }

    /// One frame of a single field over `color` with `suggestions`, returning the
    /// nodes and the widget's output.
    fn field(
        &mut self,
        color: &mut [u8; 3],
        suggestions: &[[u8; 3]],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, ColorFieldOutput) {
        let mut output = None;
        let nodes = self.frame(events, |ui| {
            output = Some(color_field(ui, "field", color, suggestions));
        });
        (nodes, output.expect("the field ran"))
    }

    /// A click at `pos`: the press frame, then the release frame whose nodes and
    /// output are returned (`clicked()` lands on the release).
    fn click_at(
        &mut self,
        pos: egui::Pos2,
        color: &mut [u8; 3],
        suggestions: &[[u8; 3]],
    ) -> (Vec<accesskit::Node>, ColorFieldOutput) {
        self.field(color, suggestions, pointer_at(pos, true));
        self.field(color, suggestions, pointer_at(pos, false))
    }

    /// One frame over two fields, returning the nodes and each field's output. The
    /// spacer keeps `b`'s button below `a`'s popup, so it stays clickable while `a`'s
    /// is open.
    fn two_fields(
        &mut self,
        a: &mut [u8; 3],
        b: &mut [u8; 3],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, ColorFieldOutput, ColorFieldOutput) {
        let mut out_a = None;
        let mut out_b = None;
        let nodes = self.frame(events, |ui| {
            out_a = Some(color_field(ui, "a", a, &[]));
            ui.add_space(400.0);
            out_b = Some(color_field(ui, "b", b, &[]));
        });
        (
            nodes,
            out_a.expect("field a ran"),
            out_b.expect("field b ran"),
        )
    }

    /// A click at `pos` over the two fields; returns the release frame.
    fn click_two(
        &mut self,
        pos: egui::Pos2,
        a: &mut [u8; 3],
        b: &mut [u8; 3],
    ) -> (Vec<accesskit::Node>, ColorFieldOutput, ColorFieldOutput) {
        self.two_fields(a, b, pointer_at(pos, true));
        self.two_fields(a, b, pointer_at(pos, false))
    }
}

/// The node carrying `label` (a `WidgetInfo::labeled` widget's label).
fn entry<'a>(nodes: &'a [accesskit::Node], label: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.label() == Some(label))
        .unwrap_or_else(|| panic!("no widget labeled {label:?}"))
}

/// Every drawn swatch label: suggestion and recents swatches are labeled by their
/// `#RRGGBB` hex.
fn swatch_labels(nodes: &[accesskit::Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|node| node.label())
        .filter(|label| label.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// The `index`th field button (labeled `color field …`, role `ColorWell`), ordered
/// top to bottom: `nodes` come back in `IdMap` order, not layout order.
fn field_button(nodes: &[accesskit::Node], index: usize) -> &accesskit::Node {
    let mut buttons: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| {
            node.label()
                .is_some_and(|label| label.starts_with("color field"))
        })
        .collect();
    buttons.sort_by(|a, b| {
        let (ay, by) = (a.bounds().map(|r| r.y0), b.bounds().map(|r| r.y0));
        ay.partial_cmp(&by).expect("drawn bounds")
    });
    buttons.get(index).expect("not enough color fields drawn")
}

/// The hex field's node: the popup's text input showing a `#RRGGBB` value.
fn hex_field(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && node.value().is_some_and(|value| value.starts_with('#'))
        })
        .expect("no hex field drawn")
}

/// The R, G or B field's node: a spin button whose value text carries its prefix
/// (`"R 128"`-shaped).
fn spin_field<'a>(nodes: &'a [accesskit::Node], prefix: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::SpinButton
                && node.value().is_some_and(|value| value.starts_with(prefix))
        })
        .unwrap_or_else(|| panic!("no field with prefix {prefix:?}"))
}

/// `node`'s AccessKit bounds as an egui `Rect`.
fn egui_rect(node: &accesskit::Node) -> egui::Rect {
    let bounds = node.bounds().expect("a drawn node has bounds");
    egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// The frame's mesh covering `rect` (within `tolerance` points of error) that also
/// satisfies `has` (bounds alone cannot tell a fill mesh from its outline stroke).
fn mesh_at(
    shapes: &[egui::epaint::ClippedShape],
    rect: egui::Rect,
    tolerance: f32,
    has: impl Fn(&egui::Mesh) -> bool,
) -> &egui::Mesh {
    shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh)
                if has(mesh)
                    && (mesh.calc_bounds().min - rect.min).length() < tolerance
                    && (mesh.calc_bounds().max - rect.max).length() < tolerance =>
            {
                Some(mesh.as_ref())
            }
            _ => None,
        })
        .expect("no mesh drawn at the expected bounds")
}

/// The color of `mesh`'s vertex nearest to `pos`.
fn color_at(mesh: &egui::Mesh, pos: egui::Pos2) -> Color32 {
    mesh.vertices
        .iter()
        .min_by(|a, b| {
            a.pos
                .distance_sq(pos)
                .partial_cmp(&b.pos.distance_sq(pos))
                .expect("no NaN positions")
        })
        .expect("a mesh has vertices")
        .color
}

/// `a` lerped to `b` by `t`, per channel, as the renderer blends vertex colors.
fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let channel = |a: u8, b: u8| -> u8 { (a as f32 + (b as f32 - a as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
        channel(a.a(), b.a()),
    )
}

/// The color the strip's mesh shows at `x`: its vertex columns (one per sextant
/// boundary) lerped between the two bracketing `x`.
fn interpolated_at_x(mesh: &egui::Mesh, x: f32) -> Color32 {
    let mut columns: Vec<(f32, Color32)> = mesh
        .vertices
        .iter()
        .map(|vertex| (vertex.pos.x, vertex.color))
        .collect();
    columns.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered xs"));
    columns.dedup_by_key(|column| column.0.to_bits());
    let right = columns
        .iter()
        .position(|&(col_x, _)| col_x > x)
        .unwrap_or(columns.len() - 1);
    let (x0, c0) = columns[right - 1];
    let (x1, c1) = columns[right];
    lerp_color(c0, c1, (x - x0) / (x1 - x0))
}

/// The barycentric weights of `pos` over the triangle `(a, b, c)`: all three are
/// positive exactly when `pos` is inside the triangle.
fn barycentric(pos: egui::Pos2, a: egui::Pos2, b: egui::Pos2, c: egui::Pos2) -> (f32, f32, f32) {
    let determinant = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    let u = ((b.y - c.y) * (pos.x - c.x) + (c.x - b.x) * (pos.y - c.y)) / determinant;
    let v = ((c.y - a.y) * (pos.x - c.x) + (a.x - c.x) * (pos.y - c.y)) / determinant;
    (u, v, 1.0 - u - v)
}

/// The index of `mesh`'s triangle containing `pos` (a triplet in `indices`), and `pos`'s
/// barycentric weights over its vertices.
fn triangle_holding(mesh: &egui::Mesh, pos: egui::Pos2) -> Option<(usize, (f32, f32, f32))> {
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .enumerate()
        .find_map(|(index, triplet)| {
            let (a, b, c) = (
                mesh.vertices[triplet[0] as usize].pos,
                mesh.vertices[triplet[1] as usize].pos,
                mesh.vertices[triplet[2] as usize].pos,
            );
            let weights @ (u, v, w) = barycentric(pos, a, b, c);
            (u > 0.0 && v > 0.0 && w > 0.0).then_some((index, weights))
        })
}

/// The color the renderer shows inside a mesh triangle: its three vertex colors mixed
/// by `pos`'s barycentric weights.
fn weights_color(
    weights: (f32, f32, f32),
    a: &egui::epaint::Vertex,
    b: &egui::epaint::Vertex,
    c: &egui::epaint::Vertex,
) -> Color32 {
    let (u, v, w) = weights;
    let channel = |channel: fn(&Color32) -> u8| -> u8 {
        (u * channel(&a.color) as f32 + v * channel(&b.color) as f32 + w * channel(&c.color) as f32)
            .round() as u8
    };
    Color32::from_rgba_unmultiplied(
        channel(|color| color.r()),
        channel(|color| color.g()),
        channel(|color| color.b()),
        channel(|color| color.a()),
    )
}

/// The centre of `node`'s bounds, where a click lands.
fn centre(node: &accesskit::Node) -> egui::Pos2 {
    let bounds = node.bounds().expect("a widget has bounds");
    egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    )
}

/// The primary button pressed (or released) at `pos`, the pointer moved there first.
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

/// `key` pressed and released.
fn key(key: egui::Key) -> Vec<egui::Event> {
    let event = |pressed: bool| egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    vec![event(true), event(false)]
}

/// Replaces the focused text field's contents with `text`: End moves the caret past
/// the text, the backspaces clear it, then the characters land.
fn type_text(text: &str) -> Vec<egui::Event> {
    let mut events = key(egui::Key::End);
    for _ in 0..16 {
        events.push(egui::Event::Key {
            key: egui::Key::Backspace,
            physical_key: Some(egui::Key::Backspace),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
    events.push(egui::Event::Text(text.to_owned()));
    events
}

#[test]
fn parse_hex_reads_rrggbb_permissively() {
    for text in ["#ff8000", "FF8000", " #Ff8000 "] {
        assert_eq!(
            parse_hex(text),
            Some([255, 128, 0]),
            "{text:?} should parse"
        );
    }
    for text in ["#zz0000", "#fff", "ff80001"] {
        assert_eq!(parse_hex(text), None, "{text:?} should not parse");
    }
}

#[test]
fn format_hex_is_upper_case_rrggbb() {
    assert_eq!(format_hex([255, 130, 0]), "#FF8200");
}

// A committed hex text is one change.
#[test]
fn hex_text_committed_with_enter_is_one_change() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    // The popup's first frame is a sizing pass: its layer is hit-testable only from
    // the next frame on.
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);

    let mut events = type_text("#ff8000");
    events.extend(key(egui::Key::Enter));
    let (_, out) = window.field(&mut color, &[], events);
    assert!(out.changed, "the Enter commit is one change");
    assert_eq!(color, [255, 128, 0], "the committed color");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "changed fires on the commit frame only");
}

// The `#` is optional, and the field commits on focus loss.
#[test]
fn hex_text_without_hash_commits_on_focus_loss() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("ff3300"));

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Tab));
    assert!(out.changed, "moving focus away commits the typed text");
    assert_eq!(color, [255, 51, 0], "the committed color");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "changed fires on the commit frame only");
}

// A parseable text commits whenever the field loses focus, the popup closing included.
// The field's own button closing it is not an exception.
#[test]
fn a_typed_hex_commits_when_the_button_closes_the_popup() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#112233"));

    // The press on the field's own button settles the typed text (the draft's
    // mousedown order); the release then closes the popup.
    let button = centre(field_button(&nodes, 0));
    let (_, out) = window.field(&mut color, &[], pointer_at(button, true));
    assert!(out.changed, "the typed text commits on the button close");
    assert_eq!(color, [17, 34, 51], "the typed color committed");
    let (_, out) = window.field(&mut color, &[], pointer_at(button, false));
    assert!(!out.changed, "the commit fires once");

    // The close also joins the recents, since the color is not the one it opened on.
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        swatch_labels(&nodes).contains(&"#112233".to_owned()),
        "the close joined the recents"
    );
}

// A committed R/G/B value is one change. The keystrokes before it change nothing.
#[test]
fn a_typed_rgb_value_commits_on_enter() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);

    // The keystrokes themselves are not commits: one each frame, untouched.
    for text in ["1", "0"] {
        let (_, out) = window.field(&mut color, &[], vec![egui::Event::Text(text.to_owned())]);
        assert!(!out.changed, "a keystroke is not a commit");
        assert_eq!(color, [0, 0, 0], "the color waits for the Enter");
    }
    let (_, out) = window.field(&mut color, &[], key(egui::Key::Enter));
    assert!(out.changed, "the Enter commits once");
    assert_eq!(color, [10, 0, 0], "the typed channel committed");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "changed fires on the commit frame only");
}

// A wheel notch over an R/G/B field steps it by one, committed as a typed value is.
#[test]
fn a_wheel_notch_over_a_channel_commits_one_step() {
    let mut window = Headless::new();
    let mut color = [10, 20, 30];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let at = centre(spin_field(&nodes, "G "));
    let notch = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let (_, out) = window.field(&mut color, &[], vec![egui::Event::PointerMoved(at), notch]);
    assert!(out.changed, "the notch commits");
    assert_eq!(color, [10, 19, 30], "a notch down takes one off G");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "changed fires on the notch's frame only");
    assert_eq!(color, [10, 19, 30]);
}

// Tab between the channel fields is one edit: the last focus loss commits all three at once.
#[test]
fn tab_between_channels_commits_only_when_the_edit_ends() {
    let mut window = Headless::new();
    let mut color = [255, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);
    window.field(&mut color, &[], type_text("10"));

    // Tab to G and type there: the edit is still open, nothing commits.
    window.field(&mut color, &[], key(egui::Key::Tab));
    window.field(&mut color, &[], vec![egui::Event::Text("20".to_owned())]);
    for _ in 0..3 {
        let (_, out) = window.field(&mut color, &[], Vec::new());
        assert!(!out.changed, "the edit is still open: nothing commits");
        assert_eq!(color, [255, 0, 0], "the color waits for the last loss");
    }

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Enter));
    assert!(out.changed, "the last focus loss commits all three at once");
    assert_eq!(color, [10, 20, 0], "R's and G's edits both landed");
}

// Typing is not a gesture. A click inside the field being edited moves the caret and
// commits nothing.
#[test]
fn a_click_inside_a_typed_channel_moves_the_caret_without_a_commit() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let red = centre(spin_field(&nodes, "R "));
    window.click_at(red, &mut color, &[]);
    window.field(&mut color, &[], type_text("200"));

    // A click inside the field to place the caret is not a gesture: no commit.
    window.click_at(red, &mut color, &[]);
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "a caret click commits nothing");
    assert_eq!(color, [0, 0, 0], "the typed value is still pending");

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Enter));
    assert!(out.changed, "the Enter commits once");
    assert_eq!(color, [200, 0, 0], "the typed channel committed");
}

// A press on another text field settles a pending text first, then the new field takes
// the focus.
#[test]
fn a_press_on_another_text_field_commits_the_pending_text() {
    let mut window = Headless::new();
    let mut color = [255, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#00FF00"));
    assert_eq!(color, [255, 0, 0], "the typed text is still pending");

    let red = centre(spin_field(&nodes, "R "));
    let (_, out) = window.field(&mut color, &[], pointer_at(red, true));
    assert!(out.changed, "the press commits the typed text");
    assert_eq!(color, [0, 255, 0], "the typed color landed");
    window.field(&mut color, &[], pointer_at(red, false));
    // The press also took the focus for the R field: a keystroke lands in it.
    window.field(&mut color, &[], vec![egui::Event::Text("9".to_owned())]);
    let (_, out) = window.field(&mut color, &[], key(egui::Key::Enter));
    assert!(out.changed, "the keystroke landed in the focused R field");
    assert_eq!(color[0], 9, "the R channel took the keystroke: {color:?}");
}

// A press on a gradient settles a pending text first, then a tap commits the tap's own
// color.
#[test]
fn a_typed_hex_then_a_one_frame_tap_on_the_strip() {
    let mut window = Headless::new();
    let mut color = [255, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#008000"));
    assert_eq!(color, [255, 0, 0], "the typed text is still pending");

    let rect = entry(&nodes, "hue").bounds().expect("the strip is drawn");
    let pos = egui::pos2(
        (rect.x0 + (rect.x1 - rect.x0) * (2.0 / 3.0)) as f32,
        (rect.y0 + rect.y1) as f32 / 2.0,
    );
    let mut events = pointer_at(pos, true);
    events.extend(pointer_at(pos, false));
    let (_, out) = window.field(&mut color, &[], events);
    assert!(out.changed, "the one-frame tap commits");
    // The typed text's saturation and value (1.0, 0.5) with the tap's hue (2/3):
    // hsv → [0, 0, 128], proving the text's value drove the pick, not the tap's alone.
    assert_eq!(
        color,
        [0, 0, 128],
        "the tap's hue over the typed value: {color:?}"
    );
}

// A typed value commits at the press, then the swatch's own commit lands on its release.
#[test]
fn a_swatch_click_after_typing_commits_the_text_then_the_swatch() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    // Seed one recent so the row is drawn.
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    let mut events = type_text("#112233");
    events.extend(key(egui::Key::Enter));
    window.field(&mut color, &[], events);
    window.field(&mut color, &[], key(egui::Key::Escape));

    color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);
    window.field(&mut color, &[], type_text("10"));
    assert_eq!(color, [0, 0, 0], "the typed value is still pending");

    let swatch = centre(entry(&nodes, "#112233"));
    let (_, out) = window.field(&mut color, &[], pointer_at(swatch, true));
    assert!(out.changed, "the press commits the typed value");
    assert_eq!(color, [10, 0, 0], "the typed channel landed");
    let (_, out) = window.field(&mut color, &[], pointer_at(swatch, false));
    assert!(out.changed, "the release commits the swatch");
    assert_eq!(color, [17, 34, 51], "the swatch's color won");
}

// A swatch click while the hex field holds a pending text settles the text first. Then
// the swatch commits and the popup closes with it.
#[test]
fn a_swatch_click_with_a_pending_hex_commits_the_swatch_and_closes() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let suggestions = [[68, 85, 102]];
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &suggestions);
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &suggestions);
    window.field(&mut color, &suggestions, type_text("#00FF00"));
    assert_eq!(color, [0, 0, 0], "the typed text is still pending");

    let swatch = centre(entry(&nodes, "#445566"));
    let (_, out) = window.field(&mut color, &suggestions, pointer_at(swatch, true));
    assert!(out.changed, "the press settles the typed text");
    assert_eq!(color, [0, 255, 0], "the typed color landed");
    let (_, out) = window.field(&mut color, &suggestions, pointer_at(swatch, false));
    assert!(out.changed, "the release commits the swatch");
    assert_eq!(color, [68, 85, 102], "the swatch's color won");

    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "the swatch click closed the popup"
    );
    assert_eq!(
        recent_colors(&window.ctx),
        vec![[68, 85, 102]],
        "the close color joined the recents"
    );
}

// A drag on an R/G/B field is a candidate while it lasts and one change when released.
#[test]
fn a_drag_on_an_rgb_field_is_one_change() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let green = centre(spin_field(&nodes, "G "));

    let mut last_candidate = None;
    for (index, dx) in [10.0, 20.0, 30.0].iter().enumerate() {
        let pos = egui::pos2(green.x + dx, green.y);
        let (_, out) = window.field(
            &mut color,
            &[],
            if index == 0 {
                pointer_at(pos, true)
            } else {
                vec![egui::Event::PointerMoved(pos)]
            },
        );
        assert!(out.candidate.is_some(), "the dragged value previews");
        assert!(!out.changed, "no commit while the drag runs");
        assert_eq!(color, [255, 128, 0], "the color waits for the release");
        last_candidate = out.candidate;
    }

    let (_, out) = window.field(
        &mut color,
        &[],
        pointer_at(egui::pos2(green.x + 30.0, green.y), false),
    );
    assert!(out.changed, "the release commits once");
    assert_eq!(
        color,
        last_candidate.expect("the drag previewed"),
        "the committed color is the last candidate"
    );
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no repeat on the idle frame");
    assert_eq!(out.candidate, None);
}

// Any close but Escape commits a parseable typed text, hex or R/G/B. The field's own
// button closing it is not an exception.
#[test]
fn a_typed_channel_commits_when_the_button_closes_the_popup() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);
    window.field(&mut color, &[], type_text("10"));

    // The press on the field's own button settles the typed value (the draft's
    // mousedown order); the release then closes the popup.
    let button = centre(field_button(&nodes, 0));
    let (_, out) = window.field(&mut color, &[], pointer_at(button, true));
    assert!(out.changed, "the typed value commits on the button close");
    assert_eq!(color, [10, 0, 0], "the typed channel committed");
    let (_, out) = window.field(&mut color, &[], pointer_at(button, false));
    assert!(!out.changed, "the commit fires once");
}

// A click on the square is a gesture too, however short. A text naming the current color
// has nothing to reset and must not wipe the click.
#[test]
fn a_square_click_after_focusing_the_hex_commits_the_point() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);

    let square = entry(&nodes, "saturation and value");
    let rect = square.bounds().expect("the square is drawn");
    let pos = egui::pos2((rect.x0 + 40.0) as f32, (rect.y0 + 60.0) as f32);
    let (_, press) = window.field(&mut color, &[], pointer_at(pos, true));
    let (_, out) = window.field(&mut color, &[], pointer_at(pos, false));
    assert!(out.changed, "the click commits");
    assert_ne!(
        color,
        [255, 128, 0],
        "the click point's color committed: {color:?}"
    );
    assert_eq!(
        press.candidate,
        Some(color),
        "the press previewed the click's point, not the field's text"
    );
}

// The same for an R/G/B field focused before the click.
#[test]
fn a_square_click_after_focusing_a_channel_field_commits_the_point() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);

    let square = entry(&nodes, "saturation and value");
    let rect = square.bounds().expect("the square is drawn");
    let pos = egui::pos2((rect.x0 + 40.0) as f32, (rect.y0 + 60.0) as f32);
    let (_, out) = window.click_at(pos, &mut color, &[]);
    assert!(out.changed, "the click commits");
    assert_ne!(
        color,
        [255, 128, 0],
        "the click point's color committed: {color:?}"
    );
}

// A text that does not read as `#RRGGBB` is put back when the field loses focus.
#[test]
fn invalid_hex_text_is_put_back_on_focus_loss() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#zz0000"));

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Tab));
    assert!(!out.changed, "an invalid text commits nothing");
    assert_eq!(color, [255, 128, 0], "the color is untouched");
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert_eq!(
        hex_field(&nodes).value(),
        Some("#FF8000"),
        "the field is put back to the color's hex"
    );
}

// Closing on another color joins the session's recent colors, shared by every picker. A
// swatch click is one change.
#[test]
fn a_second_field_shares_the_firsts_recent_colors() {
    let mut window = Headless::new();
    let mut color_a = [0, 0, 0];
    let mut color_b = [64, 64, 64];

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 0)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(hex_field(&nodes)), &mut color_a, &mut color_b);
    let mut events = type_text("#112233");
    events.extend(key(egui::Key::Enter));
    let (_, out_a, _) = window.two_fields(&mut color_a, &mut color_b, events);
    assert!(
        out_a.changed && color_a == [17, 34, 51],
        "field a committed"
    );

    window.two_fields(&mut color_a, &mut color_b, key(egui::Key::Escape));
    // A's popup still drew on the frame it closed in; the next frame's hit test
    // uses it, so B's button is clickable only from the frame after.
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());

    let (_, _, _) = window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    assert!(
        swatch_labels(&nodes).contains(&"#112233".to_owned()),
        "the second field's recents hold the first's color"
    );

    let (_, _, out_b) =
        window.click_two(centre(entry(&nodes, "#112233")), &mut color_a, &mut color_b);
    assert!(out_b.changed, "a swatch click is one change");
    assert_eq!(color_b, [17, 34, 51], "the swatch's color committed");
    let (_, _, out_b) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    assert!(!out_b.changed, "changed fires on the click frame only");
}

// A color joins the recents only when the popup closes on a color other than the one it
// opened on.
#[test]
fn closing_unchanged_adds_no_recent() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];

    // Open and close with no pick is not a pick, by Escape.
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    window.field(&mut color, &[], Vec::new());
    window.field(&mut color, &[], key(egui::Key::Escape));
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.is_empty(),
        "an unchanged Escape close adds no recent: {labels:?}"
    );
    window.field(&mut color, &[], key(egui::Key::Escape));

    // Nor is a close between frames, when the field is not drawn for a frame (the
    // host switched tabs) and egui reaps the abandoned popup.
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    window.field(&mut color, &[], Vec::new());
    window.frame(Vec::new(), |_| {});
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.is_empty(),
        "an unchanged between-frame close adds no recent: {labels:?}"
    );
}

// Any close but Escape commits a parseable typed text, including a close that lands
// between frames.
#[test]
fn a_typed_hex_commits_when_the_popup_closes_between_frames() {
    let mut window = Headless::new();
    let mut color_a = [0, 0, 0];
    let mut color_b = [64, 64, 64];

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 0)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(hex_field(&nodes)), &mut color_a, &mut color_b);
    window.two_fields(&mut color_a, &mut color_b, type_text("#112233"));

    // The field is not drawn for a frame (the host switched tabs): egui reaps the
    // popup, and when the field returns its close rule commits the pending text and
    // joins it to the shared recents.
    window.frame(Vec::new(), |ui| {
        color_field(ui, "b", &mut color_b, &[]);
    });
    let (_, out_a, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    assert!(
        out_a.changed,
        "the pending text commits on the between-frame close"
    );
    assert_eq!(color_a, [17, 34, 51], "the typed color landed");

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.contains(&"#112233".to_owned()),
        "the committed color joined the recents: {labels:?}"
    );
}

// The same for a typed R, G or B value.
#[test]
fn a_typed_channel_commits_when_the_popup_closes_between_frames() {
    let mut window = Headless::new();
    let mut color_a = [0, 0, 0];
    let mut color_b = [64, 64, 64];

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 0)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(spin_field(&nodes, "R ")), &mut color_a, &mut color_b);
    window.two_fields(&mut color_a, &mut color_b, type_text("20"));

    window.frame(Vec::new(), |ui| {
        color_field(ui, "b", &mut color_b, &[]);
    });
    let (_, out_a, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    assert!(
        out_a.changed,
        "the typed channel commits on the between-frame close"
    );
    assert_eq!(color_a, [20, 0, 0], "the typed channel landed");

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.contains(&"#140000".to_owned()),
        "the committed channel joined the recents: {labels:?}"
    );
}

// A color joins the recents when the popup closes, not when the gesture commits.
#[test]
fn a_commit_joins_the_recents_only_on_close() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    // Seed one recent so the row is drawn, then commit a second color and check the
    // row while the popup is still open.
    for hex in ["#556677", "#112233"] {
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
        let mut events = type_text(hex);
        events.extend(key(egui::Key::Enter));
        window.field(&mut color, &[], events);
        if hex == "#112233" {
            let (nodes, _) = window.field(&mut color, &[], Vec::new());
            assert_eq!(
                swatch_labels(&nodes),
                vec!["#556677".to_owned()],
                "the commit is not a recent while the popup is open"
            );
        }
        window.field(&mut color, &[], key(egui::Key::Escape));
    }

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert_eq!(
        swatch_labels(&nodes),
        vec!["#112233".to_owned(), "#556677".to_owned()],
        "on close the committed color fronts the recents"
    );
}

// A click on a recent swatch commits its color and closes the popup. The close puts the
// swatch's color at the front of the recents.
#[test]
fn a_recent_swatch_click_commits_and_closes_the_popup() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    // Two seeded recents, #445566 in front, so the clicked swatch's color
    // reaching the front of the row is observable.
    for hex in ["#112233", "#445566"] {
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
        let mut events = type_text(hex);
        events.extend(key(egui::Key::Enter));
        window.field(&mut color, &[], events);
        window.field(&mut color, &[], key(egui::Key::Escape));
    }

    color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let (_, out) = window.click_at(centre(entry(&nodes, "#112233")), &mut color, &[]);
    assert!(out.changed, "a swatch click is one change");
    assert_eq!(color, [17, 34, 51], "the swatch's color committed");

    let (nodes, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "changed fires on the click frame only");
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "the swatch click closed the popup"
    );
    assert_eq!(
        recent_colors(&window.ctx),
        vec![[17, 34, 51], [68, 85, 102]],
        "the close fronted the recents with the swatch's color"
    );

    // The closed popup still opens again on the button.
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        nodes.iter().any(|n| n.role() == Role::TextInput),
        "the button reopened the popup"
    );
    window.field(&mut color, &[], key(egui::Key::Escape));
}

// The recents are the session's, shared by every picker in the program.
#[test]
fn a_sibling_field_close_pushes_the_color_to_the_shared_recents() {
    let mut window = Headless::new();
    let mut color_a = [0, 0, 0];
    let mut color_b = [64, 64, 64];

    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 0)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(hex_field(&nodes)), &mut color_a, &mut color_b);
    let mut events = type_text("#112233");
    events.extend(key(egui::Key::Enter));
    window.two_fields(&mut color_a, &mut color_b, events);

    // Clicking field b's button opens its popup: egui keeps one popup per viewport,
    // so a's closes and its changed color joins the shared recents.
    window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.contains(&"#112233".to_owned()),
        "field b's recents hold field a's color: {labels:?}"
    );
    window.two_fields(&mut color_a, &mut color_b, key(egui::Key::Escape));

    // The same rule applies when the popup closes between frames (the field is not
    // drawn for a frame and egui reaps the abandoned popup).
    color_b = [32, 32, 32];
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(hex_field(&nodes)), &mut color_a, &mut color_b);
    let mut events = type_text("#778899");
    events.extend(key(egui::Key::Enter));
    window.two_fields(&mut color_a, &mut color_b, events);
    window.frame(Vec::new(), |ui| {
        color_field(ui, "a", &mut color_a, &[]);
    });
    let (_, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    window.click_two(centre(field_button(&nodes, 1)), &mut color_a, &mut color_b);
    let (nodes, _, _) = window.two_fields(&mut color_a, &mut color_b, Vec::new());
    let labels = swatch_labels(&nodes);
    assert!(
        labels.contains(&"#778899".to_owned()),
        "the between-frame close joined the recents: {labels:?}"
    );
}

// The host owns the color. Its own edits, such as an undo, are not picks, so the popup
// follows and nothing joins the recents.
#[test]
fn a_host_change_reseeds_the_open_popup_and_is_not_a_pick() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    window.field(&mut color, &[], Vec::new());

    // The host moves the color between frames (an undo): the popup reseeds to it.
    color = [50, 60, 70];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert_eq!(
        hex_field(&nodes).value(),
        Some("#323C46"),
        "the hex field shows the host's color"
    );

    // Closing after the host's move is not a pick: nothing joins the recents.
    window.field(&mut color, &[], key(egui::Key::Escape));
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        swatch_labels(&nodes).is_empty(),
        "a host change does not join the recents"
    );
}

// The host's rounding of the widget's own commit (the Save editor's 6 bits) arrives the
// next frame. It is still that commit, so it joins the recents on close.
#[test]
fn the_hosts_rounding_of_a_commit_still_joins_the_recents() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    let mut events = type_text("#FF8000");
    events.extend(key(egui::Key::Enter));
    window.field(&mut color, &[], events);
    assert_eq!(color, [255, 128, 0], "the pick committed");

    // The host stores the pick rounded to 6 bits a channel and shows it widened
    // back: 0x80 stores as 32, shown as 130; the widget must read that as the same
    // commit, not as an undo.
    color = [255, 130, 0];
    window.field(&mut color, &[], Vec::new());
    window.field(&mut color, &[], key(egui::Key::Escape));
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let labels = swatch_labels(&nodes);
    assert_eq!(
        labels,
        vec!["#FF8200".to_owned()],
        "the rounded commit joined the recents: {labels:?}"
    );
}

// A hovered swatch is a hover candidate for the host's live preview, not a change.
#[test]
fn hovering_a_swatch_previews_it_without_committing() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    // Two commits leave two recents; hovering the older one previews a color that is
    // not the current one.
    for hex in ["#112233", "#445566"] {
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
        let mut events = type_text(hex);
        events.extend(key(egui::Key::Enter));
        window.field(&mut color, &[], events);
        window.field(&mut color, &[], key(egui::Key::Escape));
    }
    assert_eq!(color, [68, 85, 102], "the second commit landed");

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let swatch = entry(&nodes, "#112233");
    let (_, out) = window.field(
        &mut color,
        &[],
        vec![egui::Event::PointerMoved(centre(swatch))],
    );
    assert_eq!(
        out.candidate,
        Some([17, 34, 51]),
        "a hovered swatch is the live candidate"
    );
    assert!(!out.changed, "a hover commits nothing");
    assert_eq!(color, [68, 85, 102], "the color is untouched by a hover");

    let (_, out) = window.field(
        &mut color,
        &[],
        vec![egui::Event::PointerMoved(egui::pos2(400.0, 400.0))],
    );
    assert_eq!(out.candidate, None, "moving off clears the candidate");
    assert!(!out.changed && color == [68, 85, 102]);
}

// A drag in the square is a candidate while it lasts and one change when released.
#[test]
fn a_drag_on_the_square_is_one_change() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    // The popup's first frame is a sizing pass: its layer is hit-testable only from
    // the next frame on, and its position is final then too.
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let square = entry(&nodes, "saturation and value");
    let rect = square.bounds().expect("the square is drawn");
    let point = |x: f64, y: f64| egui::pos2((rect.x0 + x) as f32, (rect.y0 + y) as f32);

    let mut last_candidate = None;
    for pos in [point(30.0, 30.0), point(60.0, 50.0), point(90.0, 70.0)] {
        let pressed = last_candidate.is_none();
        let (_, out) = window.field(
            &mut color,
            &[],
            if pressed {
                pointer_at(pos, true)
            } else {
                vec![egui::Event::PointerMoved(pos)]
            },
        );
        assert!(
            out.candidate.is_some(),
            "the dragged color is the candidate while held"
        );
        assert!(!out.changed, "no commit while the drag runs");
        assert_eq!(color, [255, 128, 0], "the color waits for the release");
        last_candidate = out.candidate;
    }

    let (_, out) = window.field(&mut color, &[], pointer_at(point(90.0, 70.0), false));
    assert!(out.changed, "the release commits once");
    assert_eq!(
        color,
        last_candidate.expect("the drag previewed"),
        "the committed color is the last candidate"
    );
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no repeat on the idle frame");
    assert_eq!(out.candidate, None);
}

// A drag on the hue strip is a candidate while it lasts and one change when released.
#[test]
fn a_drag_on_the_hue_strip_is_one_change() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let strip = entry(&nodes, "hue");
    let rect = strip.bounds().expect("the strip is drawn");
    let point = |x: f64| {
        egui::pos2(
            (rect.x0 + x * (rect.x1 - rect.x0)) as f32,
            ((rect.y0 + rect.y1) / 2.0) as f32,
        )
    };

    let mut last_candidate = None;
    for (index, x) in [0.3, 0.5, 0.8].iter().enumerate() {
        let (_, out) = window.field(
            &mut color,
            &[],
            if index == 0 {
                pointer_at(point(*x), true)
            } else {
                vec![egui::Event::PointerMoved(point(*x))]
            },
        );
        assert!(out.candidate.is_some(), "the dragged hue previews");
        assert!(!out.changed, "no commit while the drag runs");
        assert_eq!(color, [255, 128, 0], "the color waits for the release");
        last_candidate = out.candidate;
    }

    let (_, out) = window.field(&mut color, &[], pointer_at(point(0.8), false));
    assert!(out.changed, "the release commits once");
    assert_eq!(
        color,
        last_candidate.expect("the drag previewed"),
        "the committed color is the last candidate"
    );
    assert_ne!(
        hsva_of(color).h,
        hsva_of([255, 128, 0]).h,
        "the committed hue differs from the start's"
    );
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no repeat on the idle frame");
}

// Escape cancels a running gesture: the dragged candidate never lands.
#[test]
fn escape_during_a_square_drag_cancels_it() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let rect = entry(&nodes, "saturation and value")
        .bounds()
        .expect("the square is drawn");
    let point = |x: f64, y: f64| egui::pos2((rect.x0 + x) as f32, (rect.y0 + y) as f32);

    window.field(&mut color, &[], pointer_at(point(30.0, 30.0), true));
    window.field(
        &mut color,
        &[],
        vec![egui::Event::PointerMoved(point(80.0, 70.0))],
    );
    let (_, out) = window.field(&mut color, &[], key(egui::Key::Escape));
    assert!(!out.changed, "Escape commits nothing");
    assert_eq!(color, [255, 128, 0], "the dragged candidate was dropped");
}

// Escape cancels the edit instead: a typed text is discarded, not committed.
#[test]
fn escape_discards_a_typed_hex() {
    let mut window = Headless::new();
    let mut color = [255, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#00FF00"));

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Escape));
    assert!(!out.changed, "Escape commits nothing");
    assert_eq!(color, [255, 0, 0], "the typed text was discarded");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no commit lands later either");

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        !swatch_labels(&nodes).contains(&"#00FF00".to_owned()),
        "the discarded text joined no recents"
    );
}

// The same for a typed channel value.
#[test]
fn escape_discards_a_typed_channel() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);
    window.field(&mut color, &[], type_text("20"));

    let (_, out) = window.field(&mut color, &[], key(egui::Key::Escape));
    assert!(!out.changed, "Escape commits nothing");
    assert_eq!(color, [0, 0, 0], "the typed value was discarded");
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no commit lands later either");

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        !swatch_labels(&nodes).contains(&"#140000".to_owned()),
        "the discarded value joined no recents"
    );
}

// The same for a drag on the hue strip.
#[test]
fn escape_during_a_strip_drag_cancels_it() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let rect = entry(&nodes, "hue").bounds().expect("the strip is drawn");
    let point = |x: f64| {
        egui::pos2(
            (rect.x0 + x * (rect.x1 - rect.x0)) as f32,
            ((rect.y0 + rect.y1) / 2.0) as f32,
        )
    };

    window.field(&mut color, &[], pointer_at(point(0.2), true));
    window.field(&mut color, &[], vec![egui::Event::PointerMoved(point(0.7))]);
    let (_, out) = window.field(&mut color, &[], key(egui::Key::Escape));
    assert!(!out.changed, "Escape commits nothing");
    assert_eq!(color, [255, 128, 0], "the dragged candidate was dropped");
}

// A click on the square or the slider is a gesture too, however short.
#[test]
fn a_one_frame_click_on_the_square_commits() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let square = entry(&nodes, "saturation and value");
    let rect = square.bounds().expect("the square is drawn");
    let pos = egui::pos2(
        (rect.x0 + rect.x1) as f32 / 2.0,
        (rect.y0 + rect.y1) as f32 / 2.0,
    );

    // A press and its release in one input frame (a touchpad tap) is a click:
    // one change, to the colour under the point.
    let mut events = pointer_at(pos, true);
    events.extend(pointer_at(pos, false));
    let (_, out) = window.field(&mut color, &[], events);
    assert!(out.changed, "the one-frame click commits once");
    assert_ne!(
        color,
        [0, 0, 0],
        "the click point's colour committed: {color:?}"
    );
    let (nodes, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no repeat on the idle frame");
    assert!(
        nodes.iter().any(|n| n.role() == Role::TextInput),
        "a click on the square keeps the popup open"
    );
}

// A click on the square or the slider is a gesture too, however short.
#[test]
fn a_one_frame_click_on_the_strip_commits() {
    let mut window = Headless::new();
    // A saturated color: a hue tap must land somewhere visible (on a hueless one it
    // would be a commit that changes nothing, which the widget rightly skips).
    let mut color = [255, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let strip = entry(&nodes, "hue");
    let rect = strip.bounds().expect("the strip is drawn");
    let pos = egui::pos2(
        (rect.x0 + (rect.x1 - rect.x0) * 0.4) as f32,
        (rect.y0 + rect.y1) as f32 / 2.0,
    );

    let mut events = pointer_at(pos, true);
    events.extend(pointer_at(pos, false));
    let (_, out) = window.field(&mut color, &[], events);
    assert!(out.changed, "the one-frame tap commits once");
    assert_ne!(
        color,
        [255, 0, 0],
        "the tap point's colour committed: {color:?}"
    );
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "no repeat on the idle frame");
}

// A drag only previews while it lasts: a held press commits nothing until its release.
#[test]
fn a_held_press_after_focusing_the_hex_commits_nothing_until_release() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    let rect = entry(&nodes, "saturation and value")
        .bounds()
        .expect("the square is drawn");
    let pos = egui::pos2((rect.x0 + 40.0) as f32, (rect.y0 + 60.0) as f32);

    window.field(&mut color, &[], pointer_at(pos, true));
    for _ in 0..3 {
        let (_, out) = window.field(&mut color, &[], Vec::new());
        assert!(!out.changed, "a held press commits nothing mid-gesture");
        assert_eq!(color, [255, 128, 0], "the color waits for the release");
    }
    let (_, out) = window.field(&mut color, &[], pointer_at(pos, false));
    assert!(out.changed, "the release commits once");
    assert_ne!(color, [255, 128, 0], "the release's color committed");
}

// On a hueless color, the working hue survives a held press, not just a release.
#[test]
fn a_held_press_on_gray_keeps_the_hue_chosen_on_the_strip() {
    let mut window = Headless::new();
    let mut color = [200, 200, 200];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    let strip_rect = egui_rect(entry(&nodes, "hue"));
    let square_rect = egui_rect(entry(&nodes, "saturation and value"));

    let strip_y = (strip_rect.top() + strip_rect.bottom()) / 2.0;
    let blue_x = strip_rect.left() + strip_rect.width() * (2.0 / 3.0);
    window.field(
        &mut color,
        &[],
        pointer_at(egui::pos2(blue_x, strip_y), true),
    );
    window.field(
        &mut color,
        &[],
        pointer_at(egui::pos2(blue_x, strip_y), false),
    );

    let pos = egui::pos2(
        square_rect.left() + square_rect.width() * 0.9,
        square_rect.top() + 20.0,
    );
    window.field(&mut color, &[], pointer_at(pos, true));
    for _ in 0..3 {
        let (_, out) = window.field(&mut color, &[], Vec::new());
        assert!(!out.changed, "a held press commits nothing mid-gesture");
    }
    let (_, out) = window.field(&mut color, &[], pointer_at(pos, false));
    assert!(out.changed, "the release commits once");
    assert!(
        (hsva_of(color).h - 2.0 / 3.0).abs() < 0.05,
        "the hue chosen on the strip survived the press: {color:?}"
    );
}

// A press on the square or the strip commits the pending text first. Then the gesture
// runs and ends on the dragged color.
#[test]
fn a_drag_after_focusing_the_hex_ends_on_the_dragged_color() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    let rect = entry(&nodes, "saturation and value")
        .bounds()
        .expect("the square is drawn");
    let point = |x: f64, y: f64| egui::pos2((rect.x0 + x) as f32, (rect.y0 + y) as f32);

    let mut last_candidate = None;
    for (index, pos) in [point(30.0, 30.0), point(80.0, 70.0)].iter().enumerate() {
        let (_, out) = window.field(
            &mut color,
            &[],
            if index == 0 {
                pointer_at(*pos, true)
            } else {
                vec![egui::Event::PointerMoved(*pos)]
            },
        );
        assert!(!out.changed, "no commit while the drag runs");
        last_candidate = out.candidate;
    }
    let (_, out) = window.field(&mut color, &[], pointer_at(point(80.0, 70.0), false));
    assert!(out.changed, "the release commits once");
    assert_eq!(
        color,
        last_candidate.expect("the drag previewed"),
        "the final color is the drag's, not the field's text"
    );

    // Closing afterwards does not resurrect the text either.
    window.field(&mut color, &[], key(egui::Key::Escape));
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert!(!out.changed, "closing changes nothing further");
}

// A typed text commits at the press, then the gesture's release commits again.
#[test]
fn a_drag_after_a_typed_hex_commits_the_text_then_the_drag() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#00FF00"));
    assert_eq!(color, [255, 128, 0], "still uncommitted");

    press_and_drag_on_the_square(&mut window, &mut color, &nodes, [0, 255, 0]);
}

/// Presses on the square drawn in `nodes`, drags and releases: the press
/// commits the pending `typed` color, the release the dragged one.
fn press_and_drag_on_the_square(
    window: &mut Headless,
    color: &mut [u8; 3],
    nodes: &[accesskit::Node],
    typed: [u8; 3],
) {
    let rect = entry(nodes, "saturation and value")
        .bounds()
        .expect("the square is drawn");
    let point = |x: f64, y: f64| egui::pos2((rect.x0 + x) as f32, (rect.y0 + y) as f32);
    let (_, out) = window.field(color, &[], pointer_at(point(30.0, 30.0), true));
    assert!(out.changed, "the press commits the typed color");
    assert_eq!(*color, typed, "the typed color landed");
    window.field(
        color,
        &[],
        vec![egui::Event::PointerMoved(point(80.0, 70.0))],
    );
    let (_, out) = window.field(color, &[], pointer_at(point(80.0, 70.0), false));
    assert!(out.changed, "the release commits again");
    assert_ne!(*color, typed, "the dragged color won");
}

// The window losing focus mid-edit (an Alt+Tab) is not the field's focus loss. The
// press still settles the typed text, then the drag runs: the same outputs as the
// sequence with no unfocused frame.
#[test]
fn a_drag_after_a_typed_hex_through_a_window_focus_loss() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#00FF00"));
    assert_eq!(color, [255, 128, 0], "still uncommitted");

    window.unfocused_frame(Vec::new(), |ui| {
        color_field(ui, "field", &mut color, &[]);
    });

    press_and_drag_on_the_square(&mut window, &mut color, &nodes, [0, 255, 0]);
}

// The same for a typed channel value through the window's focus loss.
#[test]
fn a_drag_after_a_typed_channel_through_a_window_focus_loss() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(spin_field(&nodes, "R ")), &mut color, &[]);
    window.field(&mut color, &[], type_text("10"));
    assert_eq!(color, [0, 0, 0], "the typed value is still pending");

    window.unfocused_frame(Vec::new(), |ui| {
        color_field(ui, "field", &mut color, &[]);
    });

    press_and_drag_on_the_square(&mut window, &mut color, &nodes, [10, 0, 0]);
}

// The working hue survives a gesture on white, gray or black, whose RGB holds no hue.
#[test]
fn a_hue_chosen_on_a_hueless_color_survives_for_the_square() {
    let mut window = Headless::new();
    let mut color = [255, 255, 255];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let strip_rect = egui_rect(entry(&nodes, "hue"));
    let square_rect = egui_rect(entry(&nodes, "saturation and value"));

    // Drag the strip to blue's x (hue 2/3) and release: white has no hue, so the
    // release commits nothing, but the marker stays at blue's x.
    let blue_x = strip_rect.left() + strip_rect.width() * (2.0 / 3.0);
    let strip_y = (strip_rect.top() + strip_rect.bottom()) / 2.0;
    let (_, out) = window.field(
        &mut color,
        &[],
        pointer_at(egui::pos2(blue_x, strip_y), true),
    );
    assert!(out.candidate.is_some() && !out.changed);
    let (_, out) = window.field(
        &mut color,
        &[],
        pointer_at(egui::pos2(blue_x, strip_y), false),
    );
    assert!(!out.changed, "white commits nothing on release");
    assert_eq!(color, [255, 255, 255]);
    let (_, out) = window.field(&mut color, &[], Vec::new());
    assert_eq!(out.candidate, None, "the gesture ended cleanly");
    // The marker's fill rect (its stroke sibling's fill is transparent; the Area
    // fade keeps its white premultiplied until the fade is over, so match on
    // non-transparent, not white).
    let marker = window
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill != Color32::TRANSPARENT
                    && (shape.rect.width() - 3.0).abs() < 0.5
                    && (shape.rect.height() - (STRIP_H + 2.0)).abs() < 0.5 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .expect("the hue marker is drawn");
    assert!(
        (marker.rect.center().x - blue_x).abs() < 1.0,
        "the marker stays at the chosen hue: {} vs {}",
        marker.rect.center().x,
        blue_x
    );

    // A click at the square's top-right (s=1, v=1) then commits the chosen hue:
    // a saturated blue.
    let (_, out) = window.click_at(
        egui::pos2(square_rect.right(), square_rect.top()),
        &mut color,
        &[],
    );
    assert!(out.changed, "the square click commits");
    assert_eq!(color[2], 255, "blue's channel is full: {color:?}");
    assert!(
        color[0] < 20 && color[1] < 20,
        "a blue, not a warm gray: {color:?}"
    );
}

// The recent colors row keeps at most eight.
#[test]
fn nine_commits_leave_eight_recent_swatches() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    for i in 0..9 {
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
        let (nodes, _) = window.field(&mut color, &[], Vec::new());
        window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
        let mut events = type_text(&format!("#{:02x}0000", 0x20 + i * 0x10));
        events.extend(key(egui::Key::Enter));
        window.field(&mut color, &[], events);
        window.field(&mut color, &[], key(egui::Key::Escape));
    }

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert_eq!(
        swatch_labels(&nodes).len(),
        8,
        "the recents row holds the eight newest"
    );
}

// A typed text commits on an outside click too, and the closed color joins the recents.
#[test]
fn an_outside_click_commits_the_typed_text_and_joins_recents() {
    let mut window = Headless::new();
    let mut color = [0, 0, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(hex_field(&nodes)), &mut color, &[]);
    window.field(&mut color, &[], type_text("#112233"));
    assert_eq!(color, [0, 0, 0], "the typed text is still pending");

    let outside = egui::pos2(760.0, 560.0);
    let (_, out) = window.field(&mut color, &[], pointer_at(outside, true));
    assert!(out.changed, "the outside press commits the typed text");
    assert_eq!(color, [17, 34, 51], "the typed color landed");
    window.field(&mut color, &[], pointer_at(outside, false));
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "the outside click closed the popup"
    );

    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        swatch_labels(&nodes).contains(&"#112233".to_owned()),
        "the committed color joined the recents"
    );
}

// Escape or a click outside closes the popup.
#[test]
fn escape_and_an_outside_click_each_close_the_popup() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    // The popup's first frame is a sizing pass; egui 0.37 hides invisible widgets
    // from the AccessKit tree, so its contents show from the next frame on.
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        nodes.iter().any(|n| n.role() == Role::TextInput),
        "the popup opened"
    );

    // The popup still draws on the frame it closes in (the close is decided after
    // its contents run); the next frame shows it gone.
    window.field(&mut color, &[], key(egui::Key::Escape));
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "Escape closed the popup"
    );

    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        nodes.iter().any(|n| n.role() == Role::TextInput),
        "the button reopened the popup"
    );
    window.click_at(egui::pos2(760.0, 560.0), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "a click outside closed the popup"
    );
}

// The host's suggestion swatches are shown only when it passes some.
#[test]
fn the_suggestion_row_shows_only_when_the_host_passes_some() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let (nodes, _) = window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    assert!(
        swatch_labels(&nodes).is_empty(),
        "no suggestion row without host swatches"
    );
    window.field(&mut color, &[], key(egui::Key::Escape));

    let suggestions = [[200, 16, 46], [255, 255, 255], [29, 66, 138]];
    window.field(&mut color, &suggestions, Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &suggestions);
    // The popup's first frame is a sizing pass; its contents join the AccessKit
    // tree from the next frame on (egui 0.37 hides invisible widgets from it).
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    assert_eq!(
        swatch_labels(&nodes).len(),
        3,
        "three suggestions draw three swatches"
    );
}

// A click on a suggestion swatch commits its color and closes the popup, like a recent.
#[test]
fn a_suggestion_swatch_click_commits_and_closes_the_popup() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let suggestions = [[200, 16, 46]];
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &suggestions);
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    let (_, out) = window.click_at(centre(entry(&nodes, "#C8102E")), &mut color, &suggestions);
    assert!(out.changed, "a swatch click is one change");
    assert_eq!(color, [200, 16, 46], "the suggestion's color committed");

    let (nodes, out) = window.field(&mut color, &suggestions, Vec::new());
    assert!(!out.changed, "changed fires on the click frame only");
    assert!(
        !nodes.iter().any(|n| n.role() == Role::TextInput),
        "the swatch click closed the popup"
    );
    assert_eq!(
        recent_colors(&window.ctx),
        vec![[200, 16, 46]],
        "the close color joined the recents"
    );
}

#[test]
fn recents_keep_the_eight_newest_without_duplicates() {
    let mut recents = Vec::new();
    for n in 0..9 {
        push_recent_color(&mut recents, [n, 0, 0]);
    }
    assert_eq!(
        recents.len(),
        MAX_RECENTS,
        "the ninth push drops the oldest"
    );
    assert_eq!(recents[0], [8, 0, 0], "newest first");
    assert!(recents.contains(&[1, 0, 0]) && !recents.contains(&[0, 0, 0]));

    push_recent_color(&mut recents, [3, 0, 0]);
    assert_eq!(recents[0], [3, 0, 0], "a repeat moves to the front");
    assert_eq!(
        recents.iter().filter(|&&c| c == [3, 0, 0]).count(),
        1,
        "no duplicate"
    );
}

// The popup holds egui's saturation-value square and hue strip, opaque, and the swatch
// rows.
#[test]
fn the_painted_shapes_match_the_working_color() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let suggestions = [[200, 16, 46]];
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &suggestions);
    let (nodes, _) = window.field(&mut color, &suggestions, Vec::new());

    let square_rect = egui_rect(entry(&nodes, "saturation and value"));
    let strip_rect = egui_rect(entry(&nodes, "hue"));
    let swatch_rect = egui_rect(entry(&nodes, "#C8102E"));

    // egui's Areas fade in over about a third of a second; the paint assertions
    // need the fade done, which takes some twenty frames at the harness's dt.
    for _ in 0..25 {
        window.field(&mut color, &suggestions, Vec::new());
    }

    // The square runs white to the hue color left to right, and darkens top to
    // bottom (the draft's canvas).
    let square = mesh_at(&window.shapes, square_rect, 0.5, |mesh| {
        mesh.vertices.len() > 20
    });
    let pure: Color32 = srgb_of(Hsva {
        h: hsva_of(color).h,
        s: 1.0,
        v: 1.0,
        a: 1.0,
    });
    assert_eq!(
        color_at(square, square_rect.left_top()),
        Color32::WHITE,
        "the top-left is white"
    );
    assert_eq!(
        color_at(square, square_rect.right_top()),
        pure,
        "the top-right is the pure hue"
    );
    for corner in [square_rect.left_bottom(), square_rect.right_bottom()] {
        assert_eq!(
            color_at(square, corner),
            Color32::BLACK,
            "the bottom edge is black"
        );
    }

    // The strip runs the hues: red at both edges.
    let strip = mesh_at(&window.shapes, strip_rect, 0.5, |mesh| {
        mesh.vertices
            .iter()
            .any(|vertex| vertex.color == Color32::RED)
    });
    for vertex in &strip.vertices {
        if vertex.pos.x == strip_rect.left() || vertex.pos.x == strip_rect.right() {
            assert_eq!(vertex.color, Color32::RED, "the hue edges are red");
        }
    }

    // The hue marker is the draft's 3px bar, two points taller than the strip and
    // centred on the hue's x.
    let marker = window
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == Color32::WHITE
                    && (shape.rect.width() - 3.0).abs() < 0.5
                    && (shape.rect.height() - (STRIP_H + 2.0)).abs() < 0.5 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .expect("a 3px marker over the strip");
    let hue_x = lerp(strip_rect.left()..=strip_rect.right(), hsva_of(color).h);
    assert!(
        (marker.rect.center().x - hue_x).abs() < 0.5,
        "the marker centres on the hue's x: {} vs {}",
        marker.rect.center().x,
        hue_x
    );

    // A swatch's fill is its color.
    let fill = window
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if shape.fill == color32([200, 16, 46])
                    && (shape.rect.center() - swatch_rect.center()).length() < 0.5 =>
            {
                Some(shape)
            }
            _ => None,
        })
        .expect("a filled swatch at its bounds");
    assert_eq!(
        fill.fill,
        color32([200, 16, 46]),
        "the swatch fills with its color"
    );
}

// The gradients show the color a click there picks, HSV being taken over sRGB values.
#[test]
fn a_click_inside_the_gradients_picks_the_shown_color() {
    let mut window = Headless::new();
    let mut color = [255, 128, 0];
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    window.click_at(centre(field_button(&nodes, 0)), &mut color, &[]);
    let (nodes, _) = window.field(&mut color, &[], Vec::new());
    let strip_rect = egui_rect(entry(&nodes, "hue"));
    let square_rect = egui_rect(entry(&nodes, "saturation and value"));
    // The Area fade-in is over before the paints are read.
    for _ in 0..25 {
        window.field(&mut color, &[], Vec::new());
    }

    // A click at hue 1/12, halfway between red's and yellow's columns, commits the
    // color the strip shows there.
    let strip = mesh_at(&window.shapes, strip_rect, 0.5, |mesh| {
        mesh.vertices
            .iter()
            .any(|vertex| vertex.color == Color32::RED)
    });
    let hue_x = strip_rect.left() + strip_rect.width() / 12.0;
    let shown = interpolated_at_x(strip, hue_x);
    let pos = egui::pos2(hue_x, (strip_rect.top() + strip_rect.bottom()) / 2.0);
    window.click_at(pos, &mut color, &[]);
    let channels = |a: [u8; 3], b: Color32| {
        [a[0], a[1], a[2]]
            .iter()
            .zip([b.r(), b.g(), b.b()])
            .map(|(&picked, shown)| picked.abs_diff(shown))
            .max()
            .unwrap_or(0)
    };
    assert!(
        channels(color, shown) <= 1,
        "the pick matches the shown color: {color:?} vs {shown:?}"
    );

    // A click strictly inside the square commits the color the painted mesh shows at
    // that exact point: find the emitted triangle containing it by `indices` and
    // barycentric weights (a point whose cell loses its bottom-left half has no
    // containing triangle and fails the lookup), then read the drawn color off the same
    // weights — two points, one in each half of a cell.
    let points: Vec<(egui::Pos2, Color32)> = {
        let square = mesh_at(&window.shapes, square_rect, 0.5, |mesh| {
            mesh.vertices.len() > 20
        });
        [(0.3, 0.7), (0.7, 0.3)]
            .iter()
            .map(|&(fx, fy)| {
                let pos = egui::pos2(
                    square_rect.left() + square_rect.width() * fx,
                    square_rect.top() + square_rect.height() * fy,
                );
                let (triplet, weights) =
                    triangle_holding(square, pos).expect("no painted triangle contains the point");
                let triplet = &square.indices[triplet * 3..triplet * 3 + 3];
                let shown = weights_color(
                    weights,
                    &square.vertices[triplet[0] as usize],
                    &square.vertices[triplet[1] as usize],
                    &square.vertices[triplet[2] as usize],
                );
                (pos, shown)
            })
            .collect()
    };
    for (pos, shown) in points {
        window.click_at(pos, &mut color, &[]);
        assert!(
            channels(color, shown) <= 1,
            "the pick matches the drawn color at {pos:?}: {color:?} vs {shown:?}"
        );
    }
}
