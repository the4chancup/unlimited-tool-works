//! How the save editor and the widgets draw a savefile name: its colour
//! codes applied as text colours, the in-game look. `append_name` is the
//! draw, `plain_format` the run the codes leave plain, `name_background`
//! the background a name on the theme's panel sits on.

use pes_savefile::model::names::name_spans;

/// The format of a plain run (no code, after a reset or a non-hex code)
/// at `font`: `PLACEHOLDER`, which the widget replaces with its own text
/// color — `TextFormat::default()`'s gray is not it.
pub fn plain_format(font: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font, egui::Color32::PLACEHOLDER)
}

/// What `append_name`'s `on` takes for a name drawn on the theme's
/// panel: the light theme's `panel_fill`, `None` in the dark theme,
/// which draws every code as it is. A name on a surface both themes
/// share (the pitch's turf under a badge) passes `None` itself.
pub fn name_background(visuals: &egui::Visuals) -> Option<egui::Color32> {
    (!visuals.dark_mode).then_some(visuals.panel_fill)
}

/// Appends `name`'s runs to `job` at `font`, each colored as `name_spans`
/// gives it (`None` runs the normal text color). `on` is the background
/// the name sits on (`name_background` for one on the theme's panel):
/// `Some` darkens a coded run that would read under 4.5 on it
/// (`theme::darkened_to_read`), `None` draws every code as it is.
/// Sections' byte ranges are relative to `job`'s own text, so the spans
/// append, never merge another job's.
pub fn append_name(
    job: &mut egui::text::LayoutJob,
    name: &str,
    font: &egui::FontId,
    on: Option<egui::Color32>,
) {
    for (color, text) in name_spans(name) {
        let format = match color {
            Some([red, green, blue]) => {
                let color = egui::Color32::from_rgb(red, green, blue);
                let color = match on {
                    Some(background) => studio_core::theme::darkened_to_read(color, background),
                    None => color,
                };
                egui::TextFormat::simple(font.clone(), color)
            }
            None => plain_format(font.clone()),
        };
        job.append(text, 0.0, format);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A coded run takes its code's color; a plain run, before a code or
    // after a reset, takes `PLACEHOLDER`, so the widget's own text color
    // replaces it.
    #[test]
    fn a_names_runs_carry_their_code_or_the_placeholder() {
        let mut job = egui::text::LayoutJob::default();
        append_name(
            &mut job,
            "WHEN THE \x11cb7bec5ffWORK RESULT\x11d WAS GOOD",
            &egui::FontId::proportional(12.5),
            None,
        );
        let runs: Vec<(&str, egui::Color32)> = job
            .sections
            .iter()
            .map(|section| {
                let range = section.byte_range.start.0..section.byte_range.end.0;
                (&job.text[range], section.format.color)
            })
            .collect();
        assert_eq!(
            runs,
            [
                ("WHEN THE ", egui::Color32::PLACEHOLDER),
                ("WORK RESULT", egui::Color32::from_rgb(0xb7, 0xbe, 0xc5)),
                (" WAS GOOD", egui::Color32::PLACEHOLDER),
            ]
        );
    }

    // `on` a background a coded run reading under 4.5 is darkened until
    // it reads there; one already at 4.5 keeps its code's colour, and a
    // plain run stays `PLACEHOLDER` either way. `None` draws every code
    // as it is, the dark theme's look.
    #[test]
    fn a_code_reading_under_4_5_on_the_background_is_darkened() {
        let panel = egui::Visuals::light().panel_fill;
        let pep = egui::Color32::from_rgb(0xc9, 0x00, 0x2b);
        assert!(
            studio_core::theme::contrast(pep, panel) >= 4.5,
            "premise: PEP's code already reads on the light panel"
        );
        let name = "\x11cc9002bffPEP\x11cffffffffSI \x11c004b93ffMAN";
        let font = egui::FontId::proportional(12.5);
        let runs = |on: Option<egui::Color32>| {
            let mut job = egui::text::LayoutJob::default();
            append_name(&mut job, name, &font, on);
            job.sections
                .iter()
                .map(|section| {
                    let range = section.byte_range.start.0..section.byte_range.end.0;
                    (job.text[range].to_owned(), section.format.color)
                })
                .collect::<Vec<(String, egui::Color32)>>()
        };
        let on_panel = runs(Some(panel));
        let si = on_panel
            .iter()
            .find(|(text, _)| text == "SI ")
            .map(|(_, color)| *color)
            .expect("the SI run");
        assert!(
            studio_core::theme::contrast(si, panel) >= 4.5,
            "the white code darkened to {si:?} reads on the light panel"
        );
        assert_eq!(
            on_panel
                .iter()
                .find(|(text, _)| text == "PEP")
                .map(|(_, color)| *color),
            Some(pep),
            "a code reading at 4.5 keeps its colour"
        );
        let mut plain = egui::text::LayoutJob::default();
        append_name(&mut plain, "A \x11cffffffffB", &font, Some(panel));
        assert_eq!(
            plain.sections.first().map(|section| section.format.color),
            Some(egui::Color32::PLACEHOLDER),
            "a plain run stays PLACEHOLDER"
        );
        assert_eq!(
            runs(None),
            [
                ("PEP".to_owned(), pep),
                ("SI ".to_owned(), egui::Color32::WHITE),
                ("MAN".to_owned(), egui::Color32::from_rgb(0x00, 0x4b, 0x93)),
            ],
            "with None every run takes its code's colour"
        );
    }

    // Every run, coded or plain, takes the font the caller passes: the
    // body style, 12.5, for a name the views draw at the body size.
    #[test]
    fn every_run_takes_the_callers_font() {
        let window = crate::test_support::Headless::new();
        let body = egui::TextStyle::Body.resolve(&window.ctx.style_of(window.ctx.theme()));
        assert_eq!(
            body,
            egui::FontId::proportional(12.5),
            "premise: body is 12.5"
        );
        let mut job = egui::text::LayoutJob::default();
        append_name(
            &mut job,
            "WHEN THE \x11cb7bec5ffWORK RESULT\x11d WAS GOOD",
            &body,
            None,
        );
        assert_eq!(
            job.sections.len(),
            3,
            "premise: a coded run between two plain"
        );
        for section in &job.sections {
            assert_eq!(section.format.font_id, body, "every run at 12.5");
        }
    }
}
