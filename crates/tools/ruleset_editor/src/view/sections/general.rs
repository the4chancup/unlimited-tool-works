//! The General section (the draft's `VIEWS.general`): the ruleset's
//! name, its event page and logo and the two stat checklists, bound to
//! the ruleset in place.

use std::path::Path;

use aatf::{NON_STAT_TARGETS, Ruleset, Stat};
use dds_convert::SourceFormat;
use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;
use studio_core::widgets::{checkbox, text_box};
use team_widgets::event_logo::EventLogo;

use super::Section;
use crate::summary::human;
use crate::view::{FilePicker, file_label};

/// The Event logo field's state beyond the ruleset: the picture dialog, the
/// last refused pick's line and the picture drawn.
pub(crate) struct LogoField {
    /// The picture file dialog (`rfd`); stubbed in tests, as the view's
    /// other dialogs are.
    pub(crate) dialog: FilePicker,
    /// The line saying why the last pick was refused; a pick that is
    /// taken, or another document, clears it.
    pub(crate) refusal: Option<String>,
    /// The logo text `picture` was decoded from: the picture is decoded
    /// again only when the ruleset's logo differs (a pick, Remove, undo,
    /// another document), never per frame.
    drawn: Option<String>,
    /// The decoded picture; `None` with no logo or one that does not
    /// decode.
    picture: Option<EventLogo>,
}

impl LogoField {
    /// The field with nothing picked or drawn yet, `dialog` its picker.
    pub(crate) fn new(dialog: FilePicker) -> LogoField {
        LogoField {
            dialog,
            refusal: None,
            drawn: None,
            picture: None,
        }
    }

    /// `ruleset`'s logo decoded, from the last frame's when its text is
    /// unchanged.
    fn picture(&mut self, ctx: &egui::Context, ruleset: &Ruleset) -> Option<&EventLogo> {
        if self.drawn != ruleset.logo {
            self.picture = ruleset
                .logo_png()
                .and_then(|png| EventLogo::load(ctx, "ruleset_event_logo", &png));
            self.drawn = ruleset.logo.clone();
        }
        self.picture.as_ref()
    }
}

/// Which checklist a row toggles.
#[derive(Debug, Clone, Copy)]
enum Checklist {
    /// `stats.rating`.
    Rating,
    /// `stats.at_most`.
    AtMost,
}

/// The General form; `logo` is the Event logo field's state.
pub(crate) fn show(ui: &mut egui::Ui, ruleset: &mut Ruleset, logo: &mut LogoField) {
    ui.heading(Section::General.label());
    egui::Grid::new("general-fields")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Name");
            ui.add(text_box(
                egui::TextEdit::singleline(&mut ruleset.name).desired_width(220.0),
            ));
            ui.end_row();
            ui.label("Event page");
            event_page(ui, ruleset);
            ui.end_row();
            ui.label("Event logo");
            event_logo(ui, ruleset, logo);
            ui.end_row();
        });
    ui.weak(
        "The Save editor draws the logo at its top right while this ruleset is picked; a click \
         on it opens the event page.",
    );
    ui.separator();
    ui.strong("Rating stats");
    ui.weak(
        "A player's rating is the highest of these; it picks his tier. What a stat may sit at \
         is the target, the warning band and the stat total, not the rating.",
    );
    checklist(ui, ruleset, Checklist::Rating, &ABILITY_STATS);
    ui.separator();
    ui.strong("May sit below the target");
    ui.weak("Every other stat must equal its target exactly.");
    let all: Vec<&str> = ABILITY_STATS
        .iter()
        .copied()
        .chain(NON_STAT_TARGETS)
        .collect();
    checklist(ui, ruleset, Checklist::AtMost, &all);
}

/// The Event page box: an emptied box stores no page.
fn event_page(ui: &mut egui::Ui, ruleset: &mut Ruleset) {
    let mut page = ruleset.page.clone().unwrap_or_default();
    let edit = egui::TextEdit::singleline(&mut page)
        .desired_width(220.0)
        .hint_text("https://");
    if ui
        .add(text_box(edit))
        .on_hover_text("Opened by a click on the event logo")
        .changed()
    {
        ruleset.page = (!page.is_empty()).then_some(page);
    }
}

/// The Event logo field (the draft's `eventLogoField`): the picture at
/// the Save editor's size or a weak None, Choose picture…, Remove while
/// a logo is set, then the last refused pick's line.
fn event_logo(ui: &mut egui::Ui, ruleset: &mut Ruleset, field: &mut LogoField) {
    ui.horizontal(|ui| {
        match field.picture(ui.ctx(), ruleset) {
            Some(picture) => {
                ui.add(picture.image().alt_text("Event logo"));
            }
            None => {
                ui.weak("None");
            }
        }
        if ui
            .button("Choose picture\u{2026}")
            .on_hover_text("Any picture: a big one is made smaller to fit")
            .clicked()
            && let Some(path) = (field.dialog)(Path::new(""))
        {
            field.refusal = pick(ruleset, &path).err();
        }
        if ruleset.logo.is_some() && ui.button("Remove").clicked() {
            ruleset.logo = None;
        }
        if let Some(refusal) = &field.refusal {
            ui.colored_label(ui.visuals().error_fg_color, refusal);
        }
    });
}

/// The widest a stored logo is, in pixels; `LOGO_HEIGHT` is the
/// highest. Together they are the 84 × 28 points
/// `team_widgets::event_logo` draws a logo in, at 200% scaling: a bigger
/// picture would only be drawn smaller.
const LOGO_WIDTH: u32 = 168;
/// The highest a stored logo is, in pixels (see `LOGO_WIDTH`).
const LOGO_HEIGHT: u32 = 56;

/// The picture at `path` set as `ruleset`'s logo; the line saying why
/// when it cannot be read or is no picture, the logo then unchanged. A
/// PNG that already fits `LOGO_WIDTH` × `LOGO_HEIGHT` is stored as it
/// is, byte for byte; any other picture is scaled down to fit (never
/// up) and stored as a PNG. `set_logo`'s 64 KB rule cannot refuse a PNG
/// of that size, so it has no line here.
fn pick(ruleset: &mut Ruleset, path: &Path) -> Result<(), String> {
    let name = file_label(path);
    let bytes =
        std::fs::read(path).map_err(|error| format!("{name} could not be read: {error}"))?;
    let not_a_picture = || format!("{name} is not a picture");
    let format = path
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(SourceFormat::from_extension)
        .ok_or_else(not_a_picture)?;
    let decoded = dds_convert::decode(&bytes, format).map_err(|error| {
        log::debug!("the event logo {name} does not decode: {error}");
        not_a_picture()
    })?;
    let (width, height) = fitted(decoded.width, decoded.height);
    // A PNG `set_logo` refuses (one with large metadata chunks) is
    // encoded again below, which drops them.
    if format == SourceFormat::Png
        && (width, height) == (decoded.width, decoded.height)
        && ruleset.set_logo(&bytes).is_ok()
    {
        return Ok(());
    }
    let pixels = decoded.mips.first().ok_or_else(not_a_picture)?;
    let png = dds_convert::resize(pixels, decoded.width, decoded.height, width, height)
        .and_then(|scaled| dds_convert::encode_png(&scaled, width, height))
        .map_err(|error| {
            log::debug!("the event logo {name} could not be scaled: {error}");
            not_a_picture()
        })?;
    ruleset
        .set_logo(&png)
        .expect("a PNG of at most 168 × 56 pixels is under 64 KB");
    Ok(())
}

/// `width` × `height` scaled down to fit `LOGO_WIDTH` × `LOGO_HEIGHT`,
/// its proportions kept; a picture that fits keeps its size. The side
/// that does not fill the box is rounded, and is at least 1.
fn fitted(width: u32, height: u32) -> (u32, u32) {
    if width <= LOGO_WIDTH && height <= LOGO_HEIGHT {
        return (width, height);
    }
    // The side further over its limit decides the scale: compared as
    // width / LOGO_WIDTH against height / LOGO_HEIGHT, multiplied out.
    if u64::from(width) * u64::from(LOGO_HEIGHT) >= u64::from(height) * u64::from(LOGO_WIDTH) {
        (LOGO_WIDTH, scaled_side(height, LOGO_WIDTH, width))
    } else {
        (scaled_side(width, LOGO_HEIGHT, height), LOGO_HEIGHT)
    }
}

/// `side` × `numerator` / `denominator` rounded to the nearest pixel,
/// at least 1.
fn scaled_side(side: u32, numerator: u32, denominator: u32) -> u32 {
    let doubled = 2 * u64::from(side) * u64::from(numerator) + u64::from(denominator);
    let rounded = doubled / (2 * u64::from(denominator));
    u32::try_from(rounded.max(1)).expect("a fitted side is at most the box's side")
}

/// A checklist grid over `names`, five columns as the draft's
/// `statGrid`; each checkbox toggles `name` in `list`.
fn checklist(ui: &mut egui::Ui, ruleset: &mut Ruleset, list: Checklist, names: &[&str]) {
    egui::Grid::new((
        "checklist",
        match list {
            Checklist::Rating => "rating",
            Checklist::AtMost => "at_most",
        },
    ))
    .num_columns(5)
    .show(ui, |ui| {
        for (index, &name) in names.iter().enumerate() {
            let mut on = holds(ruleset, list, name);
            if checkbox(ui, &mut on, human(name)).changed() {
                toggle(ruleset, list, name, on);
            }
            if index % 5 == 4 {
                ui.end_row();
            }
        }
    });
}

/// Whether `ruleset`'s `list` holds `name`.
fn holds(ruleset: &Ruleset, list: Checklist, name: &str) -> bool {
    match list {
        Checklist::Rating => ruleset.stats.rating.iter().any(|stat| stat.label() == name),
        Checklist::AtMost => ruleset.stats.at_most.iter().any(|target| target == name),
    }
}

/// `name` added to or removed from `ruleset`'s `list`.
fn toggle(ruleset: &mut Ruleset, list: Checklist, name: &str, on: bool) {
    match list {
        Checklist::Rating => {
            let stat = Stat::try_from(name.to_string()).expect("an ability stat");
            ruleset.stats.rating.retain(|kept| *kept != stat);
            if on {
                ruleset.stats.rating.push(stat);
            }
        }
        Checklist::AtMost => {
            ruleset.stats.at_most.retain(|kept| kept != name);
            if on {
                ruleset.stats.at_most.push(name.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture that fits keeps its size; a bigger one is scaled to
    /// the box by its side furthest over, the other side rounded and at
    /// least 1.
    #[test]
    fn fitted_scales_down_to_168_by_56_keeping_proportions() {
        for (size, expected) in [
            ((40, 40), (40, 40)),
            ((168, 56), (168, 56)),
            ((1000, 500), (112, 56)),
            ((336, 56), (168, 28)),
            ((169, 10), (168, 10)),
            ((1000, 3), (168, 1)),
            ((5000, 1), (168, 1)),
            ((10, 5000), (1, 56)),
        ] {
            assert_eq!(fitted(size.0, size.1), expected, "{size:?}");
        }
    }
}
