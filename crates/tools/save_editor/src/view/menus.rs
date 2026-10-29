//! The toolbar's menu buttons: the Player menu's four stat operations and FPC toggle,
//! the Team menu's operations over the selected row's team, each a
//! sealed undo entry (`session/{player,team}_ops.rs`), the Savefile
//! menu's Comparator, roster sheet and Fix database
//! (`session/database.rs`), and the dialogs they open (`Boglo`,
//! `StatNum`, `CopySwap` and the Fix database confirmation; the draft's
//! `dlgBoglo`, `dlgNum`, `dlgCopySwap` and `dlgFixDb`).

use std::ops::RangeInclusive;

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{menu_button, radio_value, step_choice, step_number, wheel_steps};

use crate::session::Session;
use crate::session::database;
use crate::session::team_ops::{BootsMode, GlovesMode};
use crate::view::appearance::ownership_tip;
use crate::view::generate;
use crate::view::{Refusal, SaveView, selected_player, selected_team_id};

/// The Team menu button (the draft's `teamMenu`): Set edited flag, the
/// boots/gloves dialog, Set player names to positions, Clear visual flags
/// (its Ctrl+F drawn beside) and the two FPC items. Every item acts on the
/// selected row's team — a player's or a team row's own — so the menu is
/// disabled only with no row selected; the boots/gloves dialog prefills
/// from the selected player, on a team row from the team's first filled
/// roster slot; the aesthetics items carry the ownership line as their
/// disabled hover (SE-GUI-22).
pub(crate) fn team_menu(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some(team_id) = selected_team_id(view) else {
        ui.add_enabled_ui(false, |ui| {
            menu_button(ui, "Team", |_| ());
        });
        return;
    };
    // The dialog's prefill player: the selected player's, else the first
    // filled roster slot's of the selected row's team.
    let prefill = selected_player(view).map(|(_, _, id)| id).or_else(|| {
        view.session
            .as_ref()
            .and_then(|session| first_filled(session.file(), team_id))
    });
    let (unlocked, version, hide_ids) = {
        let Some(session) = view.session.as_ref() else {
            return;
        };
        (!view.aesthetics_locked(), session.version(), view.hide_ids)
    };
    // An out-of-band hide ID disables the FPC items behind the error's
    // text; the lock's ownership line covers the rest (SE-GUI-22).
    let (fpc_ok, fpc_tip) = match hide_ids {
        Ok(_) => (unlocked, ownership_tip(version).to_owned()),
        Err(error) => (false, error.to_string()),
    };
    menu_button(ui, "Team", |ui| {
        let Some(session) = view.session.as_mut() else {
            return;
        };
        if ui.button("Set edited flag").clicked() {
            session.set_edited_flag(team_id);
            ui.close();
        }
        if ui
            .add_enabled(
                unlocked,
                egui::Button::new("Set boots and gloves IDs for everyone\u{2026}"),
            )
            .on_disabled_hover_text(ownership_tip(version))
            .clicked()
            && let Some(player_id) = prefill
        {
            view.boglo = Some(Boglo::new(session, player_id));
            ui.close();
        }
        if ui.button("Set player names to positions").clicked() {
            session.names_to_positions(team_id);
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                unlocked,
                egui::Button::new("Clear visual flags").shortcut_text("Ctrl+F"),
            )
            .on_disabled_hover_text(ownership_tip(version))
            .clicked()
        {
            session.clear_visual_flags(team_id);
            ui.close();
        }
        if ui
            .add_enabled(fpc_ok, egui::Button::new("Set FPC invisibility"))
            .on_disabled_hover_text(&fpc_tip)
            .clicked()
            && let Ok(ids) = hide_ids
        {
            session.team_fpc(team_id, fpc::Preset::Hide, ids);
            ui.close();
        }
        if ui
            .add_enabled(fpc_ok, egui::Button::new("Remove FPC invisibility"))
            .on_disabled_hover_text(&fpc_tip)
            .clicked()
            && let Ok(ids) = hide_ids
        {
            session.team_fpc(team_id, fpc::Preset::Unhide, ids);
            ui.close();
        }
    });
}

/// The Player menu button (the draft's `playerMenu`): the Set all stats and
/// Bump all stats dialogs, Toggle FPC settings (its Ctrl+I drawn beside), the
/// Copy and Swap stats dialogs, and the two Generate items opening
/// `generate::Generate` (settings.toml and the in-game face: file writes, so
/// the aesthetics lock does not gate them). Every item acts on the selected
/// player; the menu is disabled with no save open, its entries are disabled
/// on a team selection (they act on a player), and the FPC item carries the
/// ownership line as its disabled hover (SE-GUI-22).
pub(crate) fn player_menu(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.session.is_none() {
        ui.add_enabled_ui(false, |ui| {
            menu_button(ui, "Player", |_| ());
        });
        return;
    }
    let Some((team_id, _, player_id)) = selected_player(view) else {
        // A team row selected: every item acts on a player, so all are
        // disabled with the reason on hover.
        menu_button(ui, "Player", |ui| {
            for label in [
                "Set all stats\u{2026}",
                "Bump all stats\u{2026}",
                "Toggle FPC settings",
                "Copy stats from\u{2026}",
                "Swap stats with\u{2026}",
                "Generate settings.toml\u{2026}",
                "Generate in-game face\u{2026}",
            ] {
                ui.add_enabled(false, egui::Button::new(label))
                    .on_disabled_hover_text("The Player menu acts on a player: pick a player row");
            }
        });
        return;
    };
    let (unlocked, version, hide_ids) = {
        let Some(session) = view.session.as_ref() else {
            return;
        };
        (!view.aesthetics_locked(), session.version(), view.hide_ids)
    };
    // As on the Team menu: an out-of-band hide ID disables the toggle
    // behind the error's text.
    let (fpc_ok, fpc_tip) = match hide_ids {
        Ok(_) => (unlocked, ownership_tip(version).to_owned()),
        Err(error) => (false, error.to_string()),
    };
    menu_button(ui, "Player", |ui| {
        let Some(session) = view.session.as_mut() else {
            return;
        };
        if ui.button("Set all stats\u{2026}").clicked() {
            view.stat_num = Some(StatNum::set_all(session, player_id));
            ui.close();
        }
        if ui
            .add(egui::Button::new("Bump all stats\u{2026}").shortcut_text("Ctrl+B"))
            .clicked()
        {
            view.stat_num = Some(StatNum::bump(session, player_id, view.bump));
            ui.close();
        }
        if ui
            .add_enabled(
                fpc_ok,
                egui::Button::new("Toggle FPC settings").shortcut_text("Ctrl+I"),
            )
            .on_disabled_hover_text(&fpc_tip)
            .clicked()
            && let Ok(ids) = hide_ids
        {
            session.toggle_fpc(player_id, ids);
            ui.close();
        }
        ui.separator();
        if ui.button("Copy stats from\u{2026}").clicked() {
            view.copy_swap = Some(CopySwap::new(
                session,
                player_id,
                team_id,
                CopySwapMode::Copy,
            ));
            ui.close();
        }
        if ui.button("Swap stats with\u{2026}").clicked() {
            view.copy_swap = Some(CopySwap::new(
                session,
                player_id,
                team_id,
                CopySwapMode::Swap,
            ));
            ui.close();
        }
        ui.separator();
        if ui.button("Generate settings.toml\u{2026}").clicked() {
            view.generate = Some(generate::Generate::new(
                player_id,
                generate::GenFile::Settings,
            ));
            ui.close();
        }
        if ui.button("Generate in-game face\u{2026}").clicked() {
            view.generate = Some(generate::Generate::new(
                player_id,
                generate::GenFile::IngameFace,
            ));
            ui.close();
        }
    });
}

/// The Savefile menu button (the draft's `databaseMenu`), the whole save's
/// work grouped by what it acts on: the Comparator, which reads a second
/// save against this one; then the reports, Output rosters to TSV, a pure
/// read of the open save so never locked — `rfd`'s save dialog with
/// `EDIT.tsv` as the default name, a write error shown as the refusal —
/// and Multi-team AATF, a read too (`multi_aatf`); then Fix database,
/// under the aesthetics lock with the ownership line as its disabled hover
/// (SE-GUI-22). The menu is disabled with no save open.
pub(crate) fn savefile_menu(ui: &mut egui::Ui, view: &mut SaveView) {
    let (unlocked, version) = {
        let Some(session) = view.session.as_ref() else {
            ui.add_enabled_ui(false, |ui| {
                menu_button(ui, "Savefile", |_| ());
            });
            return;
        };
        (!view.aesthetics_locked(), session.version())
    };
    menu_button(ui, "Savefile", |ui| {
        let Some(session) = view.session.as_mut() else {
            return;
        };
        if ui.button("Comparator\u{2026}").clicked() {
            view.comparator.open = true;
            ui.close();
        }
        ui.separator();
        if ui.button("Output rosters to TSV\u{2026}").clicked() {
            let written = rfd::FileDialog::new()
                .set_file_name("EDIT.tsv")
                .save_file()
                .map(|path| database::write_rosters_tsv(session.file(), &path));
            if let Some(Err(error)) = written {
                view.refusal = Some(Refusal {
                    title: "Cannot write the roster sheet",
                    message: format!("{error:#}"),
                });
            }
            ui.close();
        }
        if ui.button("Multi-team AATF\u{2026}").clicked() {
            view.multi_aatf.open(session.file());
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(unlocked, egui::Button::new("Fix database\u{2026}"))
            .on_disabled_hover_text(ownership_tip(version))
            .clicked()
        {
            view.fix_database = true;
            ui.close();
        }
    });
}

/// The Fix database confirmation's question for the open save: each
/// clause the repair writes where the records carry it — base copies
/// where a player has the base-copy flag (PES 15 to 18), the teams'
/// strip flag and the kit slots where a team has them (PES 17), the
/// visual flags always.
fn fix_database_question(file: &EditFile) -> String {
    let mut clauses = Vec::new();
    if file
        .players()
        .iter()
        .any(|player| player.edit_flags.base_copy.is_some())
    {
        clauses.push("repair base copies".to_owned());
    }
    let mut clear = "clear every player's visual flags".to_owned();
    if file
        .teams()
        .iter()
        .any(|team| team.edit_flags.strip.is_some())
    {
        clear.push_str(" and the teams' strip flag");
    }
    clauses.push(clear);
    if file.teams().iter().any(|team| team.kit_slots.is_some()) {
        clauses.push("reset the kit slots to their defaults".to_owned());
    }
    let last = clauses
        .pop()
        .expect("the visual-flags clause is always pushed");
    let joined = if clauses.is_empty() {
        last
    } else {
        let separator = if clauses.len() == 1 {
            " and "
        } else {
            ", and "
        };
        format!("{}{separator}{last}", clauses.join(", "))
    };
    let (head, tail) = joined.split_at(1);
    format!("{}{tail}?", head.to_uppercase())
}

/// The Fix database confirmation while `view.fix_database` holds (the
/// draft's `dlgFixDb`): the repair's line (`fix_database_question`), Fix
/// database applies it as one sealed undo entry
/// (`Session::fix_database`), Cancel or Escape closes with nothing
/// changed.
pub(crate) fn fix_database_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    if !view.fix_database {
        return;
    }
    let mut close = false;
    let response =
        egui::Modal::new(egui::Id::unique("save_editor_fix_database")).show(ui.ctx(), |ui| {
            studio_core::theme::window_title(ui, "Fix database");
            if let Some(session) = view.session.as_ref() {
                ui.label(fix_database_question(session.file()));
            }
            ui.horizontal(|ui| {
                if ui.button("Fix database").clicked()
                    && let Some(session) = view.session.as_mut()
                {
                    session.fix_database();
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if close || response.should_close() {
        view.fix_database = false;
    }
}

/// The Set boots and gloves IDs for everyone dialog's state (`view.boglo`
/// while it is open): the mode radios and the three ID fields, prefilled
/// from the selected player's own IDs — on a team row the team's first
/// filled roster slot's — with Unchanged picked on both sides
/// (the draft's defaults); `error` holds the refusal Set last got, shown
/// under the groups while the dialog stays open.
pub(crate) struct Boglo {
    /// The boots mode radio.
    boots: BootsPick,
    /// The gloves mode radio.
    gloves: GlovesPick,
    /// Incremental's start ID.
    start: u32,
    /// Same-ID-for-all's boots ID.
    boots_id: u32,
    /// Same-ID-for-all's gloves ID.
    gloves_id: u32,
    /// The refusal `set_boots_and_gloves` returned on Set.
    error: Option<String>,
}

/// The boots radio's pick; the IDs it needs live on `Boglo` (`start`,
/// `boots_id`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum BootsPick {
    /// Each player keeps his boots ID.
    Unchanged,
    /// `start + k - 1` for slot `k`.
    Incremental,
    /// `boots_id` for everyone.
    Same,
}

/// The gloves radio's pick.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GlovesPick {
    /// Each player keeps his gloves ID.
    Unchanged,
    /// Each player's (new) boots ID.
    LikeBoots,
    /// `gloves_id` for everyone.
    Same,
}

impl Boglo {
    /// The dialog as it opens: both sides on Unchanged, the IDs prefilled
    /// from `player`'s own.
    fn new(session: &Session, player_id: u32) -> Boglo {
        let player = session
            .file()
            .player(player_id)
            .expect("the selected player");
        Boglo {
            boots: BootsPick::Unchanged,
            gloves: GlovesPick::Unchanged,
            start: player.appearance.boots_id,
            boots_id: player.appearance.boots_id,
            gloves_id: player.appearance.gloves_id,
            error: None,
        }
    }

    /// The radios' picks as `set_boots_and_gloves`' modes.
    fn modes(&self) -> (BootsMode, GlovesMode) {
        (
            match self.boots {
                BootsPick::Unchanged => BootsMode::Unchanged,
                BootsPick::Incremental => BootsMode::Incremental { start: self.start },
                BootsPick::Same => BootsMode::Same(self.boots_id),
            },
            match self.gloves {
                GlovesPick::Unchanged => GlovesMode::Unchanged,
                GlovesPick::LikeBoots => GlovesMode::LikeBoots,
                GlovesPick::Same => GlovesMode::Same(self.gloves_id),
            },
        )
    }
}

/// The dialog's option texts, both groups': the option column is as wide
/// as the widest, so every field stands at one x.
const BOGLO_OPTIONS: [&str; 4] = [
    "Unchanged",
    "Set incremental value:",
    "Set all to the same ID:",
    "Set like boot IDs",
];
/// A field's width in the dialog (the draft's number input).
const BOGLO_BOX: f32 = 52.0;
/// The gap either side of the range's dash (the draft's `.row` gap).
const BOGLO_RANGE_GAP: f32 = 6.0;
/// The gap from the option column to the fields' (the draft's `.boglo`
/// column gap).
const BOGLO_COLUMN_GAP: f32 = 12.0;
/// The gap between a group's rows, and under its title (the draft's
/// `.boglo` row gap and top margin).
const BOGLO_ROW_GAP: f32 = 6.0;
/// How far the range's last ID lies past its start: slot 23's, a roster's
/// last.
const RANGE_SPAN: u64 = 22;

/// The dialog's two columns: the option column's width, the widest
/// option's, and the fields' column's, the range's (its start, the dash
/// and its end box), which the one-ID boxes take too.
struct BogloColumns {
    option: f32,
    field: f32,
}

impl BogloColumns {
    fn measure(ui: &egui::Ui) -> BogloColumns {
        let text_width = |text: &str| {
            ui.painter()
                .layout_no_wrap(
                    text.to_owned(),
                    egui::TextStyle::Button.resolve(ui.style()),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
        };
        let widest = BOGLO_OPTIONS
            .iter()
            .map(|text| text_width(text))
            .fold(0.0, f32::max);
        BogloColumns {
            option: widest + ui.spacing().icon_width + ui.spacing().icon_spacing,
            field: 2.0 * BOGLO_BOX + 2.0 * BOGLO_RANGE_GAP + text_width("\u{2013}"),
        }
    }
}

/// A titled group of the dialog (the draft's `.group`), as wide as its
/// rows: the title in the strong colour, then `add`'s rows.
fn boglo_group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        // Every gap in a row is given explicitly, so the fields' column
        // lands at one x in both groups.
        ui.spacing_mut().item_spacing = egui::vec2(0.0, BOGLO_ROW_GAP);
        ui.label(egui::RichText::new(title).strong());
        add(ui);
    });
}

/// `add`'s option in a cell the option column's width, then the gap to the
/// fields' column.
fn option_cell(ui: &mut egui::Ui, width: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            add(ui);
        },
    );
    ui.add_space(BOGLO_COLUMN_GAP);
}

/// The range's last ID (the draft's `.readout`): a flat box a shade off
/// the panel, its number in the palette's `readout`. No field and no disabled
/// widget: it takes no click or focus, and reads as a label.
fn readout(ui: &mut egui::Ui, value: u64) {
    let size = egui::vec2(BOGLO_BOX, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let text = value.to_string();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &text));
    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        ui.painter().rect_filled(
            rect,
            visuals.widgets.inactive.corner_radius,
            visuals.faint_bg_color,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::TextStyle::Body.resolve(ui.style()),
            studio_core::theme::palette(visuals).readout,
        );
    }
}

/// The dialog while `view.boglo` holds one (the draft's `dlgBoglo`): a
/// group per side, `Boot IDs` and `Glove IDs`,
/// each option's field in one column across both, the incremental
/// range's last ID read-only beside its start and following it; the last
/// refusal under the groups. Set applies and closes, a refusal keeps it
/// open, and Close or Escape closes it.
pub(crate) fn boglo_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.boglo.is_none() {
        return;
    }
    let Some(team_id) = selected_team_id(view) else {
        return;
    };
    let Some(session) = view.session.as_mut() else {
        return;
    };
    let boglo = view.boglo.as_mut().expect("checked above");
    let mut close = false;
    let response = egui::Modal::new(egui::Id::unique("save_editor_boglo")).show(ui.ctx(), |ui| {
        studio_core::theme::window_title(ui, "Set boots and gloves IDs for everyone");
        let columns = BogloColumns::measure(ui);
        let field_size =
            |ui: &egui::Ui, width: f32| egui::vec2(width, ui.spacing().interact_size.y);
        boglo_group(ui, "Boot IDs", |ui| {
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(ui, &mut boglo.boots, BootsPick::Unchanged, "Unchanged");
                });
            });
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(
                        ui,
                        &mut boglo.boots,
                        BootsPick::Incremental,
                        "Set incremental value:",
                    );
                });
                let start = ui.add_sized(
                    field_size(ui, BOGLO_BOX),
                    egui::DragValue::new(&mut boglo.start),
                );
                step_number(&mut boglo.start, wheel_steps(ui, &start), 0..=u32::MAX);
                ui.add_space(BOGLO_RANGE_GAP);
                ui.label("\u{2013}");
                ui.add_space(BOGLO_RANGE_GAP);
                readout(ui, u64::from(boglo.start) + RANGE_SPAN);
            });
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(
                        ui,
                        &mut boglo.boots,
                        BootsPick::Same,
                        "Set all to the same ID:",
                    );
                });
                let boots_id = ui.add_sized(
                    field_size(ui, columns.field),
                    egui::DragValue::new(&mut boglo.boots_id),
                );
                step_number(
                    &mut boglo.boots_id,
                    wheel_steps(ui, &boots_id),
                    0..=u32::MAX,
                );
            });
        });
        boglo_group(ui, "Glove IDs", |ui| {
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(ui, &mut boglo.gloves, GlovesPick::Unchanged, "Unchanged");
                });
            });
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(
                        ui,
                        &mut boglo.gloves,
                        GlovesPick::LikeBoots,
                        "Set like boot IDs",
                    );
                });
            });
            ui.horizontal(|ui| {
                option_cell(ui, columns.option, |ui| {
                    radio_value(
                        ui,
                        &mut boglo.gloves,
                        GlovesPick::Same,
                        "Set all to the same ID:",
                    );
                });
                let gloves_id = ui.add_sized(
                    field_size(ui, columns.field),
                    egui::DragValue::new(&mut boglo.gloves_id),
                );
                step_number(
                    &mut boglo.gloves_id,
                    wheel_steps(ui, &gloves_id),
                    0..=u32::MAX,
                );
            });
        });
        if let Some(error) = &boglo.error {
            ui.colored_label(ui.visuals().error_fg_color, error.as_str());
        }
        ui.horizontal(|ui| {
            if ui.button("Set").clicked() {
                let (boots, gloves) = boglo.modes();
                match session.set_boots_and_gloves(team_id, boots, gloves) {
                    Ok(()) => close = true,
                    Err(error) => boglo.error = Some(format!("{error:#}")),
                }
            }
            if ui.button("Close").clicked() {
                close = true;
            }
        });
    });
    if close || response.should_close() {
        view.boglo = None;
    }
}

/// Which of the two number dialogs OK runs (the draft's `dlgNum`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum StatNumOp {
    /// Set all stats: N 40 to 99, prefilled with the player's first
    /// ability.
    Set,
    /// Bump all stats: N -10 to 10, prefilled with the window's last N.
    Bump,
}

/// A stat number dialog's state (`view.stat_num` while it is open): which
/// operation OK runs, the player it runs on, the line above the field and
/// the field's value.
pub(crate) struct StatNum {
    /// Which operation OK runs.
    op: StatNumOp,
    /// The player the operation runs on.
    player_id: u32,
    /// The line over the field (names the player).
    line: String,
    /// The field's value.
    value: i32,
}

impl StatNum {
    /// Set all stats as it opens on `player_id`: N in 40 to 99, prefilled
    /// with his Attacking Prowess, the first ability stat.
    fn set_all(session: &Session, player_id: u32) -> StatNum {
        let player = session
            .file()
            .player(player_id)
            .expect("the selected player");
        StatNum {
            op: StatNumOp::Set,
            player_id,
            line: format!(
                "Set all of {}'s ability stats to",
                display_name(&player.name)
            ),
            value: i32::from(player.stats.attacking_prowess),
        }
    }

    /// Bump all stats as it opens on `player_id`: N in -10 to 10,
    /// prefilled with `last` (the last N used in this window, `view.bump`).
    pub(crate) fn bump(session: &Session, player_id: u32, last: i32) -> StatNum {
        let player = session
            .file()
            .player(player_id)
            .expect("the selected player");
        StatNum {
            op: StatNumOp::Bump,
            player_id,
            line: format!(
                "Add to all of {}'s ability stats, clamped to 40 to 99",
                display_name(&player.name)
            ),
            value: last,
        }
    }

    /// The dialog's heading.
    fn title(&self) -> &'static str {
        match self.op {
            StatNumOp::Set => "Set all stats",
            StatNumOp::Bump => "Bump all stats",
        }
    }

    /// The field's range for the operation.
    fn range(&self) -> RangeInclusive<i32> {
        match self.op {
            StatNumOp::Set => 40..=99,
            StatNumOp::Bump => -10..=10,
        }
    }

    /// OK's operation on `session`; a bump's N, for `view.bump` to keep.
    fn apply(&self, session: &mut Session) -> Option<i32> {
        match self.op {
            StatNumOp::Set => {
                session.set_all_stats(
                    self.player_id,
                    u8::try_from(self.value).expect("the range holds 40 to 99"),
                );
                None
            }
            StatNumOp::Bump => {
                session.bump_stats(
                    self.player_id,
                    i8::try_from(self.value).expect("the range holds -10 to 10"),
                );
                Some(self.value)
            }
        }
    }
}

/// The number dialog while `view.stat_num` holds one (the draft's
/// `dlgNum`): the line, the field on it inside the operation's range, OK
/// and Cancel; OK applies and closes (a Bump's N stays as `view.bump`),
/// Cancel or Escape closes.
pub(crate) fn stat_num_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.stat_num.is_none() {
        return;
    }
    let mut close = false;
    let mut bumped = None;
    let response =
        egui::Modal::new(egui::Id::unique("save_editor_stat_num")).show(ui.ctx(), |ui| {
            let stat_num = view.stat_num.as_mut().expect("checked above");
            studio_core::theme::window_title(ui, stat_num.title());
            let range = stat_num.range();
            ui.horizontal(|ui| {
                ui.label(&stat_num.line);
                let field = ui.add(egui::DragValue::new(&mut stat_num.value).range(range.clone()));
                step_number(&mut stat_num.value, wheel_steps(ui, &field), range);
            });
            ui.horizontal(|ui| {
                if ui.button("OK").clicked()
                    && let Some(session) = view.session.as_mut()
                {
                    bumped = stat_num.apply(session);
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if let Some(n) = bumped {
        view.bump = n;
    }
    if close || response.should_close() {
        view.stat_num = None;
    }
}

/// Copy or swap, as the dialog's mode.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CopySwapMode {
    /// Copy the picked player's stats onto the selected one.
    Copy,
    /// Trade them.
    Swap,
}

/// The Copy/Swap stats dialog's state (`view.copy_swap` while it is
/// open): the mode, the selected player it runs on, and the other player
/// picked by team then roster slot (the draft's `dlgCopySwap`'s two
/// combo boxes — one list of a cup save's 4,600 players cannot be
/// browsed).
pub(crate) struct CopySwap {
    /// Copy or swap.
    mode: CopySwapMode,
    /// The selected player (the destination, the first swap side).
    player_id: u32,
    /// The picked team's id.
    team_id: u32,
    /// The picked player's id, a filled slot of `team_id`.
    other_id: u32,
}

impl CopySwap {
    /// The dialog as it opens: the picked team the selected player's own,
    /// the picked player its first filled roster slot.
    fn new(session: &Session, player_id: u32, team_id: u32, mode: CopySwapMode) -> CopySwap {
        let other_id = first_filled(session.file(), team_id).unwrap_or(player_id);
        CopySwap {
            mode,
            player_id,
            team_id,
            other_id,
        }
    }

    /// The dialog's heading.
    fn title(&self) -> &'static str {
        match self.mode {
            CopySwapMode::Copy => "Copy stats",
            CopySwapMode::Swap => "Swap stats",
        }
    }

    /// OK's operation on `session`.
    fn apply(&self, session: &mut Session) {
        match self.mode {
            CopySwapMode::Copy => session.copy_stats(self.player_id, self.other_id),
            CopySwapMode::Swap => session.swap_stats(self.player_id, self.other_id),
        }
    }
}

/// The first filled roster slot's player id on `team_id`, skipping an id
/// no player record carries (`None` for an empty or missing team).
fn first_filled(file: &EditFile, team_id: u32) -> Option<u32> {
    file.team(team_id).and_then(|team| {
        team.roster
            .iter()
            .map(|slot| slot.player_id)
            .find(|&id| id != 0 && file.player(id).is_some())
    })
}

/// The player combo's `(player_id, label)` options for `team_id`: every
/// filled roster slot in slot order, each labeled by its shirt number,
/// registered position and display name, as the player list shows them.
fn roster_options(file: &EditFile, team_id: u32) -> Vec<(u32, String)> {
    let mut options = Vec::new();
    let Some(team) = file.team(team_id) else {
        return options;
    };
    for slot in &team.roster {
        if slot.player_id == 0 {
            continue;
        }
        let label = match file.player(slot.player_id) {
            Some(player) => {
                let position = POSITIONS
                    .get(usize::from(player.positions.registered))
                    .copied()
                    .unwrap_or("?");
                format!(
                    "{} {} {}",
                    slot.number,
                    position,
                    display_name(&player.name)
                )
            }
            None => format!("{} ?", slot.number),
        };
        options.push((slot.player_id, label));
    }
    options
}

/// The Copy/Swap stats dialog while `view.copy_swap` holds one (the
/// draft's `dlgCopySwap`): the heading and line naming the selected
/// player, the team combo prefilled with his own team, the player combo
/// listing that team's filled roster slots; OK applies and closes,
/// Cancel or Escape closes.
pub(crate) fn copy_swap_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    if view.copy_swap.is_none() {
        return;
    }
    let mut close = false;
    let response =
        egui::Modal::new(egui::Id::unique("save_editor_copy_swap")).show(ui.ctx(), |ui| {
            let (Some(session), Some(dialog)) = (view.session.as_mut(), view.copy_swap.as_mut())
            else {
                return;
            };
            studio_core::theme::window_title(ui, dialog.title());
            let name = session
                .file()
                .player(dialog.player_id)
                .map(|player| display_name(&player.name))
                .unwrap_or_else(|| dialog.player_id.to_string());
            match dialog.mode {
                CopySwapMode::Copy => {
                    ui.label(format!("Copy the picked player's stats onto {name}."));
                }
                CopySwapMode::Swap => {
                    ui.label(format!("Swap {name}'s stats with the picked player's."));
                }
            }
            ui.horizontal(|ui| {
                ui.label("Team");
                let mut picked = dialog.team_id;
                let combo = egui::ComboBox::from_id_salt("save_editor_copy_swap_team")
                    .selected_text(
                        session
                            .file()
                            .team(picked)
                            .map(|team| team.name.clone())
                            .unwrap_or_else(|| picked.to_string()),
                    )
                    .show_ui(ui, |ui| {
                        for team in session.file().teams() {
                            ui.selectable_value(&mut picked, team.id, &team.name);
                        }
                    })
                    .response;
                let teams = session.file().teams().iter().map(|team| team.id);
                if let Some(next) = step_choice(teams, &picked, wheel_steps(ui, &combo)) {
                    picked = next;
                }
                if picked != dialog.team_id {
                    dialog.team_id = picked;
                    // The new team's first filled slot is the new pick.
                    dialog.other_id =
                        first_filled(session.file(), picked).unwrap_or(dialog.player_id);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Player");
                let options = roster_options(session.file(), dialog.team_id);
                let mut picked = dialog.other_id;
                let combo = egui::ComboBox::from_id_salt("save_editor_copy_swap_player")
                    .selected_text(
                        options
                            .iter()
                            .find(|(id, _)| *id == picked)
                            .map(|(_, label)| label.clone())
                            .unwrap_or_else(|| "?".to_owned()),
                    )
                    .show_ui(ui, |ui| {
                        for (id, label) in &options {
                            ui.selectable_value(&mut picked, *id, label);
                        }
                    })
                    .response;
                let players = options.iter().map(|(id, _)| *id);
                if let Some(next) = step_choice(players, &picked, wheel_steps(ui, &combo)) {
                    picked = next;
                }
                dialog.other_id = picked;
            });
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    dialog.apply(session);
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if close || response.should_close() {
        view.copy_swap = None;
    }
}
