//! How the save editor and the widgets draw a savefile name: its colour
//! codes applied as text colours, the in-game look. `append_name` is the
//! draw; `plain_format` is the run the codes leave plain.

use pes_savefile::model::names::name_spans;

/// The format of a plain run (no code, after a reset or a non-hex code)
/// at `font`: `PLACEHOLDER`, which the widget replaces with its own text
/// color — `TextFormat::default()`'s gray is not it.
pub fn plain_format(font: egui::FontId) -> egui::TextFormat {
    egui::TextFormat::simple(font, egui::Color32::PLACEHOLDER)
}

/// Appends `name`'s runs to `job` at `font`, each colored as `name_spans`
/// gives it (`None` runs the normal text color). Sections' byte ranges
/// are relative to `job`'s own text, so the spans append, never merge
/// another job's.
pub fn append_name(job: &mut egui::text::LayoutJob, name: &str, font: &egui::FontId) {
    for (color, text) in name_spans(name) {
        let format = match color {
            Some([red, green, blue]) => {
                egui::TextFormat::simple(font.clone(), egui::Color32::from_rgb(red, green, blue))
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
