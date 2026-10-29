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
use crate::view::{Refusal, SaveView, selected_player};

/// The disabled controls' tooltip on a team selection.
const TEAM_TIP: &str = "The make buttons act on a player: pick a player row";

/// The columns' gap (the draft's `.qagrid` gap): the halves' width
/// subtracts it too, so the spacing and the widths share the constant.
const COLUMN_GAP: f32 = 6.0;

/// The gap between a column's cells (the draft's `.qacol` gap).
const ROW_GAP: f32 = 3.0;

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
    // Each column is half the list's frame less the gap (the draft's
    // `1fr 1fr`), read from the panel's own width: inside the columns a
    // cell's `available_width` is the column's, not the panel's.
    let set_stats = set_stats_widths(ui);
    let cell = egui::vec2(
        (ui.available_width() - COLUMN_GAP) / 2.0,
        QUICK_ACTION_HEIGHT,
    );
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(COLUMN_GAP, ROW_GAP);
        for (index, tiers) in [&left, &right].into_iter().enumerate() {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = ROW_GAP;
                for tier in tiers {
                    tier_button(ui, session, refusal, rules, tier, player, cell);
                }
                // Set stats ends the right column, under the other tiers.
                if index == 1 {
                    set_stats_cell(ui, session, stat_value, player, set_stats, cell);
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
/// field sharing the cell, the field taking what the button leaves of it
/// (the draft's shrinking input).
fn set_stats_cell(
    ui: &mut egui::Ui,
    session: &mut Option<Session>,
    stat_value: &mut u8,
    player: Option<(u32, u8, u32)>,
    widths: SetStatsWidths,
    size: egui::Vec2,
) {
    let enabled = player.is_some();
    ui.allocate_ui(size, |ui| {
        ui.horizontal(|ui| {
            // The field keeps its two-digit floor; the button takes the
            // rest of the cell and elides past it (the same rule as the
            // Make buttons).
            let button_width = widths
                .button
                .min((size.x - ui.spacing().item_spacing.x - widths.field).max(0.0));
            let button = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized(
                        [button_width, size.y],
                        egui::Button::new("Set stats").truncate(),
                    )
                })
                .inner;
            let left = size.x - button.rect.width() - ui.spacing().item_spacing.x;
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
            // The field's `min_size` is what the button leaves of the
            // cell: a `DragValue`'s own minimum would be the 40-point
            // `interact_size`, past the cell's edge.
            let field = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add(
                        egui::DragValue::new(stat_value)
                            .range(40..=99)
                            .min_size(egui::vec2(left, size.y)),
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

/// The Set stats cell's two sub-widths: the button's natural width and
/// the field's two-digit floor, needed by `set_stats_cell`'s split.
#[derive(Clone, Copy)]
struct SetStatsWidths {
    button: f32,
    field: f32,
}

/// A cell's natural width: its full text's galley plus the button
/// padding.
fn cell_width(ui: &egui::Ui, text: &str) -> f32 {
    let button = egui::TextStyle::Button.resolve(ui.style());
    ui.fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), button, egui::Color32::PLACEHOLDER))
        .size()
        .x
        + 2.0 * ui.spacing().button_padding.x
}

/// The Set stats cell's sub-widths (`cell_width` for its button, the
/// field's two digits the same way — both widgets take a padding each).
fn set_stats_widths(ui: &egui::Ui) -> SetStatsWidths {
    let field = ui.style().drag_value_text_style.resolve(ui.style());
    SetStatsWidths {
        button: cell_width(ui, "Set stats"),
        field: ui
            .fonts_mut(|fonts| {
                fonts.layout_no_wrap("99".to_owned(), field, egui::Color32::PLACEHOLDER)
            })
            .size()
            .x
            + 2.0 * ui.spacing().button_padding.x,
    }
}
