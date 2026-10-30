//! The General section (the draft's `VIEWS.general`): the ruleset's
//! name, its event page and logo, its squad size and the two stat
//! checklists, bound to the ruleset in place.

use std::path::Path;

use aatf::{LogoRefusal, NON_STAT_TARGETS, Ruleset, Stat};
use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;
use studio_core::widgets::{checkbox, text_box};
use team_widgets::event_logo::EventLogo;

use super::Section;
use super::widgets;
use crate::summary::human;
use crate::view::{FilePicker, file_label};

/// The Event logo field's state beyond the ruleset: the PNG dialog, the
/// last refused pick's line and the picture drawn.
pub(crate) struct LogoField {
    /// The PNG file dialog (`rfd`); stubbed in tests, as the view's
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
            ui.label("Squad size");
            widgets::number(ui, &mut ruleset.squad_size, 1..=32);
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
/// the Save editor's size or a weak None, Choose PNG…, Remove while a
/// logo is set, then the last refused pick's line.
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
            .button("Choose PNG\u{2026}")
            .on_hover_text("A PNG picture of at most 64 KB")
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

/// The PNG at `path` set as `ruleset`'s logo; the line saying why when it
/// cannot be read, is no PNG or is over 64 KB, the logo then unchanged.
fn pick(ruleset: &mut Ruleset, path: &Path) -> Result<(), String> {
    let name = file_label(path);
    let png = std::fs::read(path).map_err(|error| format!("{name} could not be read: {error}"))?;
    ruleset.set_logo(&png).map_err(|refusal| match refusal {
        LogoRefusal::NotPng => format!("{name} is not a PNG picture"),
        LogoRefusal::TooLarge { .. } => format!("{name} is over 64 KB"),
    })
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
