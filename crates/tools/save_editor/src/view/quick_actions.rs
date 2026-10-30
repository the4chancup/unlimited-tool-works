//! The quick actions (the draft's `.qacol`s): two columns under the
//! player list — the medal tiers' "Make <tier>" buttons left in the
//! ruleset's order, the other tiers' right then the Set stats cell (the
//! Player menu's Set all stats in one click, 40 to 99), each a
//! `Session::apply_tier` as one undoable edit. They act on the selected
//! player, so a team row leaves them disabled with the reason on hover. A
//! failed `apply_tier` leaves the player as he was and shows the error in
//! the refusal dialog.

use studio_core::widgets::{step_number, wheel_steps};

use crate::session::Session;
use crate::view::{NUMBER_BOX, Refusal, SaveView, selected_player};

/// The disabled controls' tooltip on a team selection.
const TEAM_TIP: &str = "The make buttons act on a player: pick a player row";

/// A quick action cell's height, the draft's 20: the theme's
/// controls are 18 (`interact_size`), the quick actions keep their own.
const QUICK_ACTION_HEIGHT: f32 = 20.0;

/// The medal tiers' ids (the draft's `MEDAL_MARK`), which the left column
/// holds in the ruleset's order.
fn is_medal(id: &str) -> bool {
    matches!(id, "gold" | "silver" | "bronze")
}

/// The columns under the list; its controls disabled while the selection
/// names no rostered player (a team row).
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let player = selected_player(view);
    // The columns' contents: the medal tiers left in the ruleset's order,
    // the others right then the Set stats cell (the draft's `.qacol`s).
    // The tiers are borrowed off `view.rules`; `session` and `refusal`
    // come off their own fields so the clicks can write while it lives.
    let SaveView {
        session,
        rules,
        refusal,
        set_stats: stat_value,
        ..
    } = view;
    let rules: &aatf::Rules = &rules
        .as_ref()
        .expect("`show` resolves the ruleset before the list draws")
        .rules;
    let (left, right): (Vec<&aatf::Tier>, Vec<&aatf::Tier>) = rules
        .ruleset()
        .tiers
        .iter()
        .partition(|tier| is_medal(&tier.id));
    // Each column is half the list's frame less the gap, egui's item
    // spacing (the draft's `1fr 1fr`), read from the panel's own width:
    // inside the columns a cell's `available_width` is the column's, not
    // the panel's.
    let cell = egui::vec2(
        (ui.available_width() - ui.spacing().item_spacing.x) / 2.0,
        QUICK_ACTION_HEIGHT,
    );
    ui.horizontal(|ui| {
        for (index, tiers) in [&left, &right].into_iter().enumerate() {
            ui.vertical(|ui| {
                for tier in tiers {
                    tier_button(ui, session, refusal, rules, tier, player, cell);
                }
                // Set stats ends the right column, under the other tiers.
                if index == 1 {
                    set_stats_cell(ui, session, stat_value, player, cell);
                }
            });
        }
    });
}

/// One "Make <tier>" cell of a column: a
/// `Session::apply_tier` as one undoable edit, the tier's name on hover
/// (also the elided label's name), a failed run leaving the player as he
/// was with the error in `refusal`.
fn tier_button(
    ui: &mut egui::Ui,
    session: &mut Option<Session>,
    refusal: &mut Option<Refusal>,
    rules: &aatf::Rules,
    tier: &aatf::Tier,
    player: Option<(u32, u8, u32)>,
    size: egui::Vec2,
) {
    let enabled = player.is_some();
    let button = ui
        .add_enabled_ui(enabled, |ui| {
            ui.add_sized(
                size,
                egui::Button::new(format!("Make {}", tier.label)).truncate(),
            )
        })
        .inner;
    let response = if enabled {
        button.on_hover_text(format!(
            "The {} tier's values (the active ruleset)",
            tier.label
        ))
    } else {
        button.on_disabled_hover_text(TEAM_TIP)
    };
    if response.clicked()
        && let Some((team_id, slot, _)) = player
        && let Some(session) = session.as_mut()
        && let Err(error) = session.apply_tier(rules, team_id, slot, &tier.id)
    {
        *refusal = Some(Refusal {
            title: "Cannot apply the tier",
            message: format!("{error:#}"),
        });
    }
}

/// The Set stats cell (the draft's `.row`): the button then its number
/// field sharing the cell, the field at `NUMBER_BOX` and the
/// button taking the rest of the cell (the draft's growing button).
fn set_stats_cell(
    ui: &mut egui::Ui,
    session: &mut Option<Session>,
    stat_value: &mut u8,
    player: Option<(u32, u8, u32)>,
    size: egui::Vec2,
) {
    let enabled = player.is_some();
    ui.allocate_ui(size, |ui| {
        ui.horizontal(|ui| {
            // The button elides past its room, the same rule as the Make
            // buttons.
            let button_width = (size.x - ui.spacing().item_spacing.x - NUMBER_BOX).max(0.0);
            let button = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [button_width, size.y],
                        egui::Button::new("Set stats").truncate(),
                    )
                })
                .inner;
            let response = if enabled {
                button.on_hover_text("Set every ability stat to the value next to this button")
            } else {
                button.on_disabled_hover_text(TEAM_TIP)
            };
            if response.clicked()
                && let Some((_, _, player_id)) = player
                && let Some(session) = session.as_mut()
            {
                session.set_all_stats(player_id, *stat_value);
            }
            // The field's `min_size` sets its width: a `DragValue`'s own
            // minimum would be the 40-point `interact_size`.
            let field = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add(
                        egui::DragValue::new(stat_value)
                            .range(40..=99)
                            .min_size(egui::vec2(NUMBER_BOX, size.y)),
                    )
                })
                .inner;
            step_number(stat_value, wheel_steps(ui, &field), 40..=99);
            if !enabled {
                field.on_disabled_hover_text(TEAM_TIP);
            }
        });
    });
}
