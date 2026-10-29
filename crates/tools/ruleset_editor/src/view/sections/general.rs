//! The General section (the draft's `VIEWS.general`): the ruleset's
//! name, its squad size and the two stat checklists, bound to the
//! ruleset in place.

use aatf::{NON_STAT_TARGETS, Ruleset, Stat};
use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;
use studio_core::widgets::{checkbox, text_box};

use super::Section;
use super::widgets;
use crate::summary::human;

/// Which checklist a row toggles.
#[derive(Debug, Clone, Copy)]
enum Checklist {
    /// `stats.rating`.
    Rating,
    /// `stats.at_most`.
    AtMost,
}

/// The General form.
pub(crate) fn show(ui: &mut egui::Ui, ruleset: &mut Ruleset) {
    ui.heading(Section::General.label());
    egui::Grid::new("general-fields")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Name");
            ui.add(text_box(
                egui::TextEdit::singleline(&mut ruleset.name).desired_width(220.0),
            ));
            ui.end_row();
            ui.label("Squad size");
            widgets::number(ui, &mut ruleset.squad_size, 1..=32);
            ui.end_row();
        });
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
