//! The Universal rules section (the draft's `VIEWS.universal`, its
//! `universal-rules` panel): one control per `universal` field, in the
//! field's order, bound to the ruleset in place. The team settings below
//! them have no form yet.

use aatf::{Ruleset, Universal, WeightRule};
use studio_core::widgets::checkbox;

use super::Section;
use super::widgets::number;

/// The presets a starter's A-only rule can name.
const PRESETS: [u8; 3] = [1, 2, 3];

/// The Universal rules form.
pub(crate) fn show(ui: &mut egui::Ui, ruleset: &mut Ruleset) {
    ui.heading(Section::Universal.label());
    let universal = &mut ruleset.universal;
    checkbox(
        ui,
        &mut universal.registered_position_a,
        "Every player is rated A at his registered position",
    );
    ui.horizontal(|ui| {
        ui.weak("Starters play only where rated A, in preset:");
        for preset in PRESETS {
            presets_box(ui, &mut universal.fielded_at_a, preset);
        }
    });
    checkbox(ui, &mut universal.no_b_ratings, "No B ratings");
    checkbox(
        ui,
        &mut universal.b_uses_a_allowance,
        "B ratings count toward the A-position allowance",
    );
    checkbox(ui, &mut universal.gk_not_second_a, "GK is never a second A");
    age(ui, universal);
    weight(ui, universal);
    checkbox(ui, &mut universal.captain_required, "A captain is required");
    ui.horizontal(|ui| {
        ui.label("At least");
        number(ui, &mut universal.min_goalkeepers, 0..=5);
        ui.label("goalkeepers");
    });
    ui.horizontal(|ui| {
        ui.label("At most");
        number(ui, &mut universal.max_skill_cards, 0..=41);
        ui.label("skill cards per player");
        ui.weak("(a ruleset's choice; the official rules use 10)");
    });
    com_styles(ui, universal);
    checkbox(
        ui,
        &mut universal.illegal_formations,
        "Illegal formations allowed (Whack Patch)",
    );
    ui.separator();
    ui.weak(
        "Team settings can't be edited here yet: Summary shows them, and saving leaves them \
         unchanged.",
    );
}

/// `preset`'s checkbox: ticked adds it to `presets`, which stay
/// ascending and without duplicates; cleared removes it.
fn presets_box(ui: &mut egui::Ui, presets: &mut Vec<u8>, preset: u8) {
    let mut on = presets.contains(&preset);
    if checkbox(ui, &mut on, preset.to_string()).changed() {
        presets.retain(|kept| *kept != preset);
        if on {
            presets.push(preset);
            presets.sort_unstable();
        }
    }
}

/// The Age row: ticked, the bounds start at 15 to 50.
fn age(ui: &mut egui::Ui, universal: &mut Universal) {
    ui.horizontal(|ui| {
        let mut on = universal.age.is_some();
        if checkbox(ui, &mut on, "Age").changed() {
            universal.age = on.then_some((15, 50));
        }
        match &mut universal.age {
            Some((low, high)) => {
                number(ui, low, 0..=50);
                ui.label("to");
                number(ui, high, 0..=50);
            }
            None => {
                ui.weak("any");
            }
        }
    });
}

/// The Weight row: ticked, the rule starts at height − 129, at least 30
/// kg, to height − 81.
fn weight(ui: &mut egui::Ui, universal: &mut Universal) {
    ui.horizontal(|ui| {
        let mut on = universal.weight.is_some();
        if checkbox(ui, &mut on, "Weight").changed() {
            universal.weight = on.then_some(WeightRule {
                floor: 30,
                below: 129,
                above: 81,
            });
        }
        match &mut universal.weight {
            Some(weight) => {
                ui.label("between height \u{2212}");
                number(ui, &mut weight.below, 0..=255);
                ui.label("(at least");
                number(ui, &mut weight.floor, 0..=255);
                ui.label("kg) and height \u{2212}");
                number(ui, &mut weight.above, 0..=255);
            }
            None => {
                ui.weak("any");
            }
        }
    });
}

/// The COM styles row: the cap behind its checkbox, starting at 5 when
/// ticked.
fn com_styles(ui: &mut egui::Ui, universal: &mut Universal) {
    ui.horizontal(|ui| {
        let mut on = universal.max_com_styles.is_some();
        if checkbox(ui, &mut on, "At most").changed() {
            universal.max_com_styles = on.then_some(5);
        }
        if let Some(max) = &mut universal.max_com_styles {
            number(ui, max, 0..=7);
        }
        ui.label("COM styles per player");
    });
}
