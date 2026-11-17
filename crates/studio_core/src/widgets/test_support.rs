//! Test helpers the widgets' test modules share: the suite's context and one
//! frame drawn headless.

/// A context with the suite's look installed, as the window has it, in
/// `theme`.
pub(super) fn context(theme: egui::Theme) -> egui::Context {
    let ctx = egui::Context::default();
    crate::theme::install(&ctx);
    ctx.set_theme(theme);
    ctx
}

/// One 400 by 200 frame of `contents` with `events` as its input; the
/// frame's shapes and what `contents` returned.
pub(super) fn frame<R>(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    contents: &mut impl FnMut(&mut egui::Ui) -> R,
) -> (Vec<egui::epaint::Shape>, R) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 200.0),
        )),
        events,
        ..Default::default()
    };
    let mut result = None;
    let mut output = ctx.run_ui(input, |ui| result = Some(contents(ui)));
    let shapes = std::mem::take(&mut output.shapes)
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect();
    // Nothing renders here, so the frame's texture deltas are dropped on
    // purpose.
    output.drop_without_applying_deltas();
    (shapes, result.expect("run_ui calls the closure"))
}
