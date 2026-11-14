//! The Appearance screen (the draft's `appearanceTab` and `paletteRow`):
//! six titled groups in `Frame::group` frames, laid out to show whole in
//! the default window — Full Player Customization with Colors at its
//! right, Strip style, Physique with Motion over Edit flags at its right,
//! and Presentation last (star and playing attitude, aesthetics like the
//! rest, so under the lock too). With the "Aesthetics from exports"
//! setting off the screen edits as every other does; with it on, the
//! ownership line and the "Edit aesthetics anyway" switch (`SaveView`'s
//! `edit_aesthetics`, off at every start of the tool) draw over the
//! groups, every control disabled under the ownership line as hover and
//! the switch re-enabling them for the rest of the run. Edits go through
//! `Session::edit_player` under the widget's id, one undo entry per
//! gesture, as `abilities.rs` does; a stored value past a control's range
//! draws as stored (`clamp_existing_to_range` off) and changes only when
//! edited.

pub(super) mod fields;

use pes_savefile::model::{
    PlayerAppearance, PlayerEditFlags, PlayerEntry, PlayerMotion, PlayerStats,
};
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::schema::limits::motion_top;
use pes_savefile::schema::player_max_value;
use pes_savefile::settings_toml::{Kind, SettingKey};
use pes_version::PesVersion;
use studio_core::widgets::{
    FORM_ROW_SPACING, add_enabled, add_enabled_ui, combo_item, number_arrows, step_choice,
    step_number, wheel_steps,
};

use crate::session::Session;
use crate::view::abilities::{self, gating_tooltip, maybe_end_row, number, small_stat};
use crate::view::appearance::fields::{
    MOTIONS, MotionField, PHYSIQUE, Palette, STRIP_ENUMS, combo_options, number_range, palette,
};
use crate::view::{Panel, SaveView, edit_player, selected_player};

/// `edit_flags`' Face checkbox.
pub(super) const FACE_LABEL: &str = "Face";
/// `edit_flags`' Hair checkbox.
pub(super) const HAIR_LABEL: &str = "Hair";
/// `edit_flags`' Physique checkbox, which the group's heading shares.
pub(super) const PHYSIQUE_LABEL: &str = "Physique";
/// `edit_flags`' Strip checkbox.
pub(super) const STRIP_LABEL: &str = "Strip";
/// `edit_flags`' Motion checkbox, which the group's heading shares.
pub(super) const MOTION_LABEL: &str = "Motion";
/// `edit_flags`' Base copy checkbox.
pub(super) const BASE_COPY_LABEL: &str = "Base copy";
/// `edit_flags`' copy-from id number.
pub(super) const COPY_FROM_ID_LABEL: &str = "Copy from ID";
/// The strip grid's boots id number.
pub(super) const BOOTS_ID_LABEL: &str = "Boots ID";
/// The strip grid's gloves id number.
pub(super) const GLOVES_ID_LABEL: &str = "Gloves ID";
/// The strip grid's shirttail-out checkbox.
pub(super) const SHIRTTAIL_OUT_LABEL: &str = "Shirttail out";
/// The strip grid's ankle-taping checkbox.
pub(super) const ANKLE_TAPING_LABEL: &str = "Ankle taping";
/// The strip grid's wrist taping combo.
pub(super) const WRIST_TAPING_LABEL: &str = "Wrist taping";
/// The strip grid's left wrist-tape colour dropdown.
pub(super) const WRIST_TAPE_LEFT_LABEL: &str = "Wrist tape left";
/// The strip grid's right wrist-tape colour dropdown.
pub(super) const WRIST_TAPE_RIGHT_LABEL: &str = "Wrist tape right";
/// The strip grid's spectacles combo.
pub(super) const SPECTACLES_LABEL: &str = "Spectacles";
/// The strip grid's spectacles colour dropdown.
pub(super) const SPECTACLES_COLOR_LABEL: &str = "Spectacles color";
/// The strip grid's gloves colour dropdown.
pub(super) const GLOVES_COLOR_LABEL: &str = "Gloves color";
/// The strip grid's outfielder-gloves dropdown.
pub(super) const OUTFIELDER_GLOVES_LABEL: &str = "Outfielder gloves";
/// The outfielder-gloves dropdown's two readings — `gloves == 0` and any
/// non-zero — in the combo's pick order.
pub(super) const GLOVES_OPTIONS: [&str; 2] = ["Off", "Worn in winter"];
/// The dribbling motion's number.
pub(super) const DRIBBLING_MOTION_LABEL: &str = "Dribbling motion";
/// The Presentation group's title.
const PRESENTATION_LABEL: &str = "Presentation";
/// The star box's note.
const STAR_NOTE: &str =
    "How likely the player is to show up as the team's featured player in the pre-match screen.";
/// The playing attitude box's note.
const ATTITUDE_NOTE: &str = "Only affects cutscenes.";

/// Between two groups one under another (the draft's `.group` margin).
const GROUP_GAP: f32 = 4.0;
/// Between two groups side by side (the draft's `.aprow` `gap`).
const SIDE_GAP: f32 = 8.0;
/// A strip cell's label slot (the draft's `.slabel`).
const LABEL_W: f32 = 104.0;
/// Between a strip cell's label and its field (the draft's `.scell` `gap`).
const LABEL_FIELD_GAP: f32 = 8.0;
/// A strip cell's field slot: the dropdowns, the two ID boxes and the
/// checkboxes' cells (the draft's `.scell` inputs).
const FIELD_W: f32 = 118.0;
/// Between a strip cell's field and the next cell's label (the draft's
/// `.scell` padding, 14 on each side).
const CELL_GAP: f32 = 28.0;
/// One strip column's pitch: its cell and the gap after it.
const STRIP_PITCH: f32 = LABEL_W + LABEL_FIELD_GAP + FIELD_W + CELL_GAP;
/// A strip grid row's height (the draft's `.scell` `height`).
const STRIP_ROW_H: f32 = 24.0;
/// A physique row's height: the rows a little further apart, so the group
/// fills to the right column's height (the draft's physique `td` padding).
const PHYSIQUE_ROW_H: f32 = 34.0;
/// A physique field's length (the draft's `input[type=range]` 82).
const PHYSIQUE_FIELD_W: f32 = 82.0;
/// An edit-flags cell's height (the draft's `#edit-flags-grid td`).
const FLAG_CELL_H: f32 = 22.0;
/// Between an edit-flags row's cells (the draft's `td` padding-right).
const FLAG_CELL_GAP: f32 = 12.0;

/// The Appearance screen's content for the selected player.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((_, _, player_id)) = selected_player(view) else {
        return;
    };
    // The setting decides the lock's draw (`aesthetics_locked`): with it
    // off there is no ownership line and no switch and the groups edit;
    // with it on the ownership line and the session's switch.
    let (setting, locked) = (view.aesthetics_from_exports, view.aesthetics_locked());
    let SaveView {
        session,
        edit_aesthetics,
        randomize_draw,
        generation,
        hide_ids,
        ..
    } = view;
    let generation = *generation;
    let Some(session) = session.as_mut() else {
        return;
    };
    let (version, player) = {
        let file = session.file();
        let Some(player) = file.player(player_id) else {
            return;
        };
        (file.version(), player.clone())
    };
    // Every widget id under the section is the player's within this open,
    // its own salt since it draws beside the Abilities & Skills content
    // (whose `push_id` salts the same pair): two scopes with one salt would
    // hand two widgets one id — an in-progress edit and its undo key would
    // cross between the sections.
    let mut open_patch = false;
    ui.push_id(("appearance", generation, player_id), |ui| {
        if setting {
            open_patch = ownership_line(ui);
            studio_core::widgets::checkbox(ui, edit_aesthetics, "Edit aesthetics anyway");
            if *edit_aesthetics {
                ui.weak("A later patch overwrites whatever is edited by hand.");
            }
            ui.add_space(GROUP_GAP);
        }
        add_enabled_ui(ui, !locked, |ui| {
            groups(
                ui,
                session,
                player_id,
                version,
                &player,
                randomize_draw.as_mut(),
                *hide_ids,
            );
        })
        .response
        .on_disabled_hover_text(OWNERSHIP_TIP);
    });
    if open_patch {
        view.panel = Some(Panel::Patch);
    }
}

/// The screen's groups, `GROUP_GAP` apart: Full Player Customization with
/// Colors at its right, Strip style the pane's width, Physique with
/// Motion over Edit flags at its right, and Presentation the pane's
/// width, last.
fn groups(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    draw: &mut dyn FnMut(u8) -> u8,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    fpc_colors_row(ui, session, player_id, version, player, hide_ids);
    ui.add_space(GROUP_GAP);
    group(ui, "Strip style", |ui| {
        ui.set_min_width(ui.available_width());
        strip_grid(ui, session, player_id, version, &player.appearance);
    });
    ui.add_space(GROUP_GAP);
    physique_motion_flags_row(ui, session, player_id, version, player, draw);
    ui.add_space(GROUP_GAP);
    group(ui, PRESENTATION_LABEL, |ui| {
        ui.set_min_width(ui.available_width());
        presentation(ui, session, player_id, &player.stats);
    });
}

/// One titled group (the draft's `.group`): egui's group frame, the title
/// in the strong colour and `TITLE_GAP` above the first thing drawn under
/// it, as the Stats screen's panels leave under theirs.
fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.strong(title);
        ui.add_space(abilities::TITLE_GAP - ui.spacing().item_spacing.y);
        add(ui);
    });
}

/// `prepared`'s `content_ui` grown to `height` before its frame paints:
/// `Frame::begin` reserves the frame's shape, so the `paint` after the
/// stretch draws the taller frame (the bottom padding takes the extra).
fn stretch(prepared: &mut egui::containers::frame::Prepared, height: f32) {
    let min = prepared.content_ui.min_rect();
    prepared
        .content_ui
        .advance_cursor_after_rect(egui::Rect::from_min_max(
            min.min,
            egui::pos2(min.max.x, min.min.y + height),
        ));
}

/// A group frame's margin on one axis: inner margin and stroke, which
/// `Prepared::outer_rect` adds to the content's `min_rect` (its outer
/// margin is 0 for `Frame::group`).
fn frame_margin(frame: &egui::Frame) -> f32 {
    frame.inner_margin.sum().y + frame.stroke.width * 2.0
}

/// A group frame's inside for the rows: a top-down scope from the frame's
/// left (a right-to-left row's frame would inherit the row's layout),
/// `title` in the strong colour and `TITLE_GAP` under it, as the Stats
/// screen's panels leave under theirs.
fn frame_content(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.strong(title);
            ui.add_space(abilities::TITLE_GAP - ui.spacing().item_spacing.y);
            add(ui);
        },
    );
}

/// The egui memory id `fpc_colors_row` writes the Colors group's measured
/// outer width to each pass (FPC's width reads the previous pass's; a pass
/// that measures a different one is discarded).
const COLORS_WIDTH_ID: &str = "save_editor appearance colors width";

/// The first row: Full Player Customization filling the room Colors
/// leaves (the draft's `.aprow.fpcrow`: FPC flexes, Colors keeps its own
/// width), `SIDE_GAP` between them and the two frames' bottoms level —
/// the shorter one's frame repainted to the row's height.
fn fpc_colors_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
) {
    let group_frame = egui::Frame::group(ui.style());
    let width_id = egui::Id::unique(COLORS_WIDTH_ID);
    let measured: Option<f32> = ui.ctx().data(|data| data.get_temp(width_id));
    // Left to right, FPC first at the width the last pass's Colors leaves.
    // Not a right-to-left row adding Colors first: a frame's content in it
    // spans the whole row, and a right-aligned top-down content puts its
    // grid's cursor at -inf (`Negative desired size: [NaN 18.0]`).
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.x = SIDE_GAP;
            let fpc_width = ui.available_width() - SIDE_GAP - measured.unwrap_or(0.0);
            let mut fpc = group_frame.begin(ui);
            frame_content(&mut fpc.content_ui, "Full Player Customization", |ui| {
                ui.set_min_width(fpc_width - group_frame.total_margin().sum().x);
                fpc_group(ui, session, player_id, player, hide_ids);
            });
            fpc.allocate_space(ui);
            let mut colors = group_frame.begin(ui);
            frame_content(&mut colors.content_ui, "Colors", |ui| {
                colour_rows(ui, session, player_id, version, &player.appearance);
            });
            let width = colors.allocate_space(ui).rect.width();
            ui.ctx().data_mut(|data| data.insert_temp(width_id, width));
            if measured != Some(width) {
                ui.ctx()
                    .request_discard("the Appearance screen's Colors width changed");
            }
            let height = fpc
                .content_ui
                .min_rect()
                .height()
                .max(colors.content_ui.min_rect().height());
            stretch(&mut fpc, height);
            fpc.paint(ui);
            stretch(&mut colors, height);
            colors.paint(ui);
        },
    );
}

/// The second row: Physique at its own width, and beside it a column
/// taking the rest — Motion over Edit flags, `GROUP_GAP` between them —
/// the two sides ending at one bottom (the draft's `.aprow` stretch: the
/// shorter side's frame repainted to the taller's height).
fn physique_motion_flags_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    draw: &mut dyn FnMut(u8) -> u8,
) {
    let group_frame = egui::Frame::group(ui.style());
    let margin = frame_margin(&group_frame);
    ui.scope_builder(
        egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            ui.spacing_mut().item_spacing.x = SIDE_GAP;
            let mut physique = group_frame.begin(ui);
            frame_content(&mut physique.content_ui, PHYSIQUE_LABEL, |ui| {
                physique_grid(ui, session, player_id, &player.appearance)
            });
            physique.allocate_space(ui);
            let physique_outer = physique.content_ui.min_rect().height() + margin;
            let row_height = ui
                .scope_builder(
                    egui::UiBuilder::new().layout(egui::Layout::top_down(egui::Align::Min)),
                    |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = GROUP_GAP;
                        let mut motion = group_frame.begin(ui);
                        frame_content(&mut motion.content_ui, MOTION_LABEL, |ui| {
                            ui.set_min_width(ui.available_width());
                            motion_grid(ui, session, player_id, version, &player.motion, draw);
                        });
                        let motion_outer = motion.content_ui.min_rect().height() + margin;
                        motion.end(ui);
                        let mut flags = group_frame.begin(ui);
                        frame_content(&mut flags.content_ui, "Edit flags", |ui| {
                            ui.set_min_width(ui.available_width());
                            edit_flags_grid(
                                ui,
                                session,
                                player_id,
                                &player.edit_flags,
                                player.appearance.base_copy_id,
                            );
                        });
                        flags.allocate_space(ui);
                        let column_height = ui.min_rect().height();
                        let row_height = physique_outer.max(column_height);
                        if column_height < row_height {
                            stretch(&mut flags, row_height - motion_outer - GROUP_GAP - margin);
                            let column = ui.min_rect();
                            ui.advance_cursor_after_rect(egui::Rect::from_min_max(
                                column.min,
                                egui::pos2(column.max.x, column.min.y + row_height),
                            ));
                        }
                        flags.paint(ui);
                        row_height
                    },
                )
                .inner;
            if physique_outer < row_height {
                stretch(&mut physique, row_height - margin);
            }
            physique.paint(ui);
        },
    );
}

/// The ownership row: the sentence itself and the Apply aesthetics patch
/// link. Whether the link was clicked: the caller opens the patch panel.
fn ownership_line(ui: &mut egui::Ui) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.label(OWNERSHIP_TIP);
        // The link is never broken across two rows: when it does not fit
        // after the sentence, it starts the next row whole.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.link("Apply aesthetics patch").clicked()
    })
    .inner
}

/// The ownership sentence (the draft's `ownershipTip`): the line over the
/// section, the disabled groups' hover and the locked Team-menu items'
/// (`menus.rs`).
pub(crate) const OWNERSHIP_TIP: &str = "The team's export owns the aesthetics, so edit its \
                                        settings.toml (or its ingame_face.toml, for the face, \
                                        hair and iris), recompile, then apply the patch.";

/// The FPC group, the first row's left side (the draft's `fpcGroup`): the
/// classifier's reading (`is_fpc_player` under `hide_ids`, the settings'
/// hide IDs — a player hidden by his model's own boots ID reads as shown,
/// so no checkbox), the Hide default body and Show default body buttons
/// with the draft's hover texts, then the interference findings of
/// `fpc::check` in its order. Under the lock the whole group is disabled
/// with the rest of the section — the reading and the findings still draw,
/// since they read and do not write. `hide_ids` an `Err` (a setting outside
/// its band): the line reads the error's text instead of the default-body
/// reading and both buttons are disabled with it as their hover; the
/// classifier has no IDs to read, so no findings either.
fn fpc_group(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    player: &PlayerEntry,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
) {
    let ids = match hide_ids {
        Ok(ids) => ids,
        Err(error) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(error.to_string());
                for label in ["Hide default body", "Show default body"] {
                    add_enabled(ui, false, egui::Button::new(label))
                        .on_disabled_hover_text(error.to_string());
                }
            });
            return;
        }
    };
    let hidden = pes_savefile::ops::fpc::is_fpc_player(player, ids)
        .expect("a loaded player's face run holds the skin field");
    ui.horizontal_wrapped(|ui| {
        ui.label(if hidden {
            "The default body is currently hidden"
        } else {
            "The default body is currently shown"
        });
        if ui
            .button("Hide default body")
            .on_hover_text(
                "Write long sleeves, a tucked shirt and short socks. \
                 A boots or gloves ID of 0 takes the hide ID from Settings (a model's \
                 own ID is kept), and a Custom skin (PES 2015 to 2017) becomes Light. \
                 Ctrl+I switches this player between hidden and shown, and the Team \
                 menu hides or shows the whole team.",
            )
            .clicked()
        {
            session.apply_fpc(player_id, fpc::Preset::Hide, ids);
        }
        if ui
            .button("Show default body")
            .on_hover_text(
                "Write short sleeves, an untucked shirt and standard \
                 socks. A boots or gloves ID that holds the hide ID goes back to 0, and \
                 a Custom skin (PES 2015 to 2017) becomes Light.",
            )
            .clicked()
        {
            session.apply_fpc(player_id, fpc::Preset::Unhide, ids);
        }
    });
    let style = pes_savefile::ops::fpc::strip_style(player)
        .expect("a loaded player's face run holds the strip fields");
    let findings = fpc::check(&style, hidden);
    if findings.is_empty() {
        ui.weak("No interference found.");
    } else {
        for finding in &findings {
            let message = crate::messages::fpc_finding(finding);
            let text = format!(
                "{}: {}",
                message.severity.name(),
                crate::messages::when_text(&message)
            );
            match message.severity {
                studio_core::Severity::Error | studio_core::Severity::Fatal => {
                    ui.colored_label(ui.visuals().error_fg_color, text);
                }
                studio_core::Severity::Warning => {
                    ui.colored_label(ui.visuals().warn_fg_color, text);
                }
                studio_core::Severity::Info => {
                    ui.label(text);
                }
            }
        }
    }
}

/// The Presentation group's two rows: star and playing attitude one under
/// the other, label then box, each with its note at the box's right in the
/// weak colour. `PRESENTATION_STATS` because the pair shares the Profile
/// fields' `SmallStat` shape.
fn presentation(ui: &mut egui::Ui, session: &mut Session, player_id: u32, stats: &PlayerStats) {
    egui::Grid::new("appearance presentation")
        .num_columns(3)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            for (small, note) in fields::PRESENTATION_STATS
                .iter()
                .zip([STAR_NOTE, ATTITUDE_NOTE])
            {
                small_stat(ui, session, player_id, stats, small);
                ui.weak(note);
                ui.end_row();
            }
        });
}

/// The edit flags' grid (the draft's `#edit-flags-grid`): the six flags
/// three a row, `Copy from ID` on a row of its own under them. `base_copy`
/// exists on PES 15 to 18 only; like a gated field it keeps its place,
/// disabled. Ticking it also sets the seven gameplay flags, since a copy's
/// sections count as edited; unticking clears `base_copy` alone.
fn edit_flags_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    flags: &PlayerEditFlags,
    base_copy_id: u32,
) {
    egui::Grid::new("appearance edit flags")
        .num_columns(3)
        .min_row_height(FLAG_CELL_H)
        .spacing(egui::vec2(FLAG_CELL_GAP, 0.0))
        .show(ui, |ui| {
            checkbox(ui, session, player_id, FACE_LABEL, flags.face, |p, v| {
                p.edit_flags.face = v;
            });
            checkbox(ui, session, player_id, HAIR_LABEL, flags.hair, |p, v| {
                p.edit_flags.hair = v;
            });
            checkbox(
                ui,
                session,
                player_id,
                PHYSIQUE_LABEL,
                flags.physique,
                |p, v| {
                    p.edit_flags.physique = v;
                },
            );
            ui.end_row();
            checkbox(ui, session, player_id, STRIP_LABEL, flags.strip, |p, v| {
                p.edit_flags.strip = v;
            });
            checkbox(
                ui,
                session,
                player_id,
                MOTION_LABEL,
                flags.motion,
                |p, v| {
                    p.edit_flags.motion = v;
                },
            );
            match flags.base_copy {
                Some(held) => checkbox(ui, session, player_id, BASE_COPY_LABEL, held, |p, v| {
                    p.edit_flags.base_copy = Some(v);
                    if v {
                        p.edit_flags.abilities = true;
                        p.edit_flags.com_styles = true;
                        p.edit_flags.motion = true;
                        p.edit_flags.playable_positions = true;
                        p.edit_flags.registered_position = true;
                        p.edit_flags.skills = true;
                        p.edit_flags.playing_style = true;
                    }
                }),
                None => {
                    add_enabled(ui, false, |ui: &mut egui::Ui| {
                        studio_core::widgets::checkbox(ui, &mut false, BASE_COPY_LABEL)
                    })
                    .on_disabled_hover_text("PES 15 to 18 only");
                }
            }
            ui.end_row();
        });
    ui.add_space(FORM_ROW_SPACING);
    ui.horizontal(|ui| {
        number(
            ui,
            session,
            player_id,
            COPY_FROM_ID_LABEL,
            base_copy_id,
            0..=u32::MAX,
            |player, shown| {
                player.appearance.base_copy_id = shown;
            },
        );
    });
}

/// The 14 physique fields in an aligned grid of label and field cells,
/// two pairs a row on `PHYSIQUE_ROW_H`-tall rows, each field `PHYSIQUE_FIELD_W`
/// long (the draft's `.stats-grid` pairs and 82-point sliders). Each field
/// is on its key's `Signed` scale (±7: `shown - limit` stores `shown +
/// limit`).
fn physique_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    appearance: &PlayerAppearance,
) {
    egui::Grid::new("appearance physique")
        .num_columns(4)
        .min_row_height(PHYSIQUE_ROW_H)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, 0.0))
        .show(ui, |ui| {
            for (index, (label, key, get, set)) in PHYSIQUE.iter().enumerate() {
                let Kind::Signed { limit } = key.spec().kind else {
                    unreachable!("the physique keys are Signed");
                };
                let limit = i32::from(limit);
                let stored = get(appearance);
                let mut shown = i32::from(stored) - limit;
                ui.label(*label);
                let height = ui.spacing().interact_size.y;
                let mut response = number_arrows(ui, &mut shown, -limit..=limit, |ui, value| {
                    ui.add(
                        egui::DragValue::new(value)
                            .range(-limit..=limit)
                            .clamp_existing_to_range(false)
                            .min_size(egui::vec2(PHYSIQUE_FIELD_W, height)),
                    )
                });
                if step_number(&mut shown, wheel_steps(ui, &response), -limit..=limit) {
                    response.mark_changed();
                }
                if response.changed() {
                    edit_player(session, player_id, response.id.value(), |player| {
                        set(
                            &mut player.appearance,
                            u8::try_from(shown + limit)
                                .expect("the control keeps the value inside the key's range"),
                        );
                    });
                }
                maybe_end_row(ui, index + 1, 2);
            }
        });
}

/// One strip-grid cell (the draft's `stripCells` entries): a `STRIP_ENUMS`
/// tuple's own kind, or one of the five kinds the grid holds besides
/// (the two ID boxes, the colour dropdowns, the outfielder-gloves
/// dropdown and the two checkboxes).
#[derive(Clone, Copy)]
enum StripCell {
    /// The boots ID box, on the record's own bound (`id_box`).
    BootsId,
    /// The same for the gloves ID, with the any-player hover.
    GlovesId,
    /// A `STRIP_ENUMS` entry (`label`, `SettingKey`, `get`, `set`).
    Enum(&'static fields::AppearanceField),
    /// The wrist taping dropdown.
    WristTaping,
    /// The left wrist tape's colour dropdown (`combo_options`' labels).
    WristTapeLeft,
    /// The right wrist tape's colour dropdown.
    WristTapeRight,
    /// `Off`/`Worn in winter` over the face run's PlayerGloves (not a
    /// checkbox: beside the colours it read as one of them, and it did not
    /// say when the gloves show).
    OutfielderGloves,
    /// The gloves colour dropdown over the face run's PlayerGlovesColor.
    GlovesColor,
    /// The ankle-taping checkbox, a bare box in the field slot.
    AnkleTaping,
    /// The spectacles style dropdown.
    Spectacles,
    /// The spectacles colour dropdown.
    SpectaclesColor,
    /// The shirttail-out checkbox, a bare box in the field slot.
    ShirttailOut,
}

/// The fifteen cells in the draft's order, three columns by five
/// rows (the draft's `STRIP_ORDER`).
const STRIP_ORDER: [[StripCell; 3]; 5] = [
    [
        StripCell::BootsId,
        StripCell::Enum(&STRIP_ENUMS[0]),
        StripCell::Enum(&STRIP_ENUMS[1]),
    ],
    [
        StripCell::GlovesId,
        StripCell::Enum(&STRIP_ENUMS[2]),
        StripCell::Enum(&STRIP_ENUMS[3]),
    ],
    [
        StripCell::WristTaping,
        StripCell::WristTapeLeft,
        StripCell::WristTapeRight,
    ],
    [
        StripCell::OutfielderGloves,
        StripCell::GlovesColor,
        StripCell::AnkleTaping,
    ],
    [
        StripCell::Spectacles,
        StripCell::SpectaclesColor,
        StripCell::ShirttailOut,
    ],
];

/// `cell`'s label, drawn in its `LABEL_W` slot.
fn cell_label(cell: StripCell) -> &'static str {
    match cell {
        StripCell::BootsId => BOOTS_ID_LABEL,
        StripCell::GlovesId => GLOVES_ID_LABEL,
        StripCell::Enum(&(label, ..)) => label,
        StripCell::WristTaping => WRIST_TAPING_LABEL,
        StripCell::WristTapeLeft => WRIST_TAPE_LEFT_LABEL,
        StripCell::WristTapeRight => WRIST_TAPE_RIGHT_LABEL,
        StripCell::OutfielderGloves => OUTFIELDER_GLOVES_LABEL,
        StripCell::GlovesColor => GLOVES_COLOR_LABEL,
        StripCell::AnkleTaping => ANKLE_TAPING_LABEL,
        StripCell::Spectacles => SPECTACLES_LABEL,
        StripCell::SpectaclesColor => SPECTACLES_COLOR_LABEL,
        StripCell::ShirttailOut => SHIRTTAIL_OUT_LABEL,
    }
}

/// The Strip style group's grid (the draft's `#strip-grid`): the fifteen
/// settings three columns by five rows, each cell `LABEL_W` for its label
/// then `FIELD_W` for its field, `CELL_GAP` between a field and the next
/// label, rows `STRIP_ROW_H` tall and touching, with the thin lines in
/// the group frame's stroke colour parting what belongs together — under
/// the second and third rows across the three columns, under the fourth
/// across the first two, between the columns on the first two rows and
/// at the third column's left on the last two (the draft's `lr`/`lb`
/// cells).
fn strip_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
) {
    let mut rows = Vec::with_capacity(STRIP_ORDER.len());
    ui.spacing_mut().item_spacing.y = 0.0;
    for row in STRIP_ORDER {
        let inner = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.set_min_height(STRIP_ROW_H);
            for cell in row {
                strip_cell(ui, session, player_id, version, appearance, cell);
            }
        });
        rows.push(inner.response.rect);
    }
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let painter = ui.painter();
    let left = rows[0].min.x;
    // The lines under rows two, three (the three columns) and four (the
    // first two), at the touched rows' shared bottom, ending mid-gap where
    // a column line stands.
    for (under, columns) in [(1usize, 3usize), (2, 3), (3, 2)] {
        let y = rows[under].max.y;
        painter.line_segment(
            [
                egui::pos2(left, y),
                egui::pos2(left + columns as f32 * STRIP_PITCH - CELL_GAP / 2.0, y),
            ],
            stroke,
        );
    }
    // The column lines, centred in the `CELL_GAP` between a field and the
    // next label: between the columns on rows one and two, at the third
    // column's left on rows four and five.
    for (column, first_row, last_row) in [(1usize, 0usize, 1usize), (2, 0, 1), (2, 3, 4)] {
        let x = left + column as f32 * STRIP_PITCH - CELL_GAP / 2.0;
        painter.line_segment(
            [
                egui::pos2(x, rows[first_row].min.y),
                egui::pos2(x, rows[last_row].max.y),
            ],
            stroke,
        );
    }
}

/// One strip cell: `cell`'s label in its `LABEL_W` slot, `LABEL_FIELD_GAP`,
/// then its field in the `FIELD_W` slot, `CELL_GAP` after it.
fn strip_cell(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    cell: StripCell,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(LABEL_W, STRIP_ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(LABEL_W, STRIP_ROW_H));
            ui.label(cell_label(cell));
        },
    );
    ui.add_space(LABEL_FIELD_GAP);
    ui.allocate_ui_with_layout(
        egui::vec2(FIELD_W, STRIP_ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(FIELD_W, STRIP_ROW_H));
            strip_field(ui, session, player_id, version, appearance, cell);
        },
    );
    ui.add_space(CELL_GAP);
}

/// `cell`'s field in its `FIELD_W` slot.
fn strip_field(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
    cell: StripCell,
) {
    match cell {
        StripCell::BootsId => {
            id_box(
                ui,
                session,
                player_id,
                version,
                PlayerField::BootsId,
                appearance.boots_id,
                |p, v| {
                    p.appearance.boots_id = v;
                },
            );
        }
        StripCell::GlovesId => {
            id_box(
                ui,
                session,
                player_id,
                version,
                PlayerField::GlovesId,
                appearance.gloves_id,
                |p, v| {
                    p.appearance.gloves_id = v;
                },
            )
            .on_hover_text(
                "Any player can wear gloves, not only goalkeepers; 0 means normal hands.",
            );
        }
        StripCell::Enum(&(label, key, get, set)) => {
            combo_box(
                ui,
                session,
                player_id,
                label,
                &combo_options(key),
                get(appearance),
                move |player, shown| {
                    set(&mut player.appearance, shown);
                },
            );
        }
        StripCell::WristTaping => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPING_LABEL,
            &combo_options(SettingKey::WristTaping),
            appearance.wrist_taping,
            |player, shown| {
                player.appearance.wrist_taping = shown;
            },
        ),
        StripCell::WristTapeLeft => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPE_LEFT_LABEL,
            &combo_options(SettingKey::WristTapeColorLeft),
            appearance.wrist_tape_color_left,
            |player, shown| {
                player.appearance.wrist_tape_color_left = shown;
            },
        ),
        StripCell::WristTapeRight => combo_box(
            ui,
            session,
            player_id,
            WRIST_TAPE_RIGHT_LABEL,
            &combo_options(SettingKey::WristTapeColorRight),
            appearance.wrist_tape_color_right,
            |player, shown| {
                player.appearance.wrist_tape_color_right = shown;
            },
        ),
        StripCell::OutfielderGloves => {
            let gloves = face(appearance, IngameFaceField::PlayerGloves);
            combo_box(
                ui,
                session,
                player_id,
                OUTFIELDER_GLOVES_LABEL,
                &GLOVES_OPTIONS.map(str::to_owned),
                u8::from(gloves != 0),
                |player, shown| {
                    player
                        .appearance
                        .ingame_face
                        .set(IngameFaceField::PlayerGloves, shown)
                        .expect("a loaded player's face run holds player gloves");
                },
            );
        }
        StripCell::GlovesColor => {
            let stored = face(appearance, IngameFaceField::PlayerGlovesColor);
            combo_box(
                ui,
                session,
                player_id,
                GLOVES_COLOR_LABEL,
                &combo_options(SettingKey::GlovesColor),
                stored,
                |player, shown| {
                    player
                        .appearance
                        .ingame_face
                        .set(IngameFaceField::PlayerGlovesColor, shown)
                        .expect("a loaded player's face run holds the gloves colour");
                },
            );
        }
        StripCell::AnkleTaping => {
            checkbox(
                ui,
                session,
                player_id,
                "",
                appearance.ankle_taping,
                |p, v| {
                    p.appearance.ankle_taping = v;
                },
            );
        }
        StripCell::Spectacles => combo_box(
            ui,
            session,
            player_id,
            SPECTACLES_LABEL,
            &combo_options(SettingKey::Spectacles),
            appearance.spectacles_style,
            |player, shown| {
                player.appearance.spectacles_style = shown;
            },
        ),
        StripCell::SpectaclesColor => {
            combo_box(
                ui,
                session,
                player_id,
                SPECTACLES_COLOR_LABEL,
                &combo_options(SettingKey::SpectaclesColor),
                appearance.spectacles_color,
                |player, shown| {
                    player.appearance.spectacles_color = shown;
                },
            );
        }
        StripCell::ShirttailOut => {
            checkbox(ui, session, player_id, "", appearance.untucked, |p, v| {
                p.appearance.untucked = v;
            });
        }
    }
}

/// A strip cell's ID box: a `FIELD_W` `DragValue` over the record's own
/// bound (`player_max_value`, wider than `settings.toml`'s stock range —
/// a compile-allocated ID such as 627 is a value the save holds), the
/// wheel and arrow-key steps `abilities`' `number` gives.
fn id_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    field: PlayerField,
    stored: u32,
    set: impl FnOnce(&mut PlayerEntry, u32),
) -> egui::Response {
    let range = 0..=u32::try_from(
        player_max_value(version, field).expect("the id is stored on every version"),
    )
    .expect("the id's bound fits u32");
    let mut shown = stored;
    let height = ui.spacing().interact_size.y;
    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
        ui.add(
            egui::DragValue::new(value)
                .range(range.clone())
                .clamp_existing_to_range(false)
                .min_size(egui::vec2(FIELD_W, height)),
        )
    });
    if step_number(&mut shown, wheel_steps(ui, &response), range) {
        response.mark_changed();
    }
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, shown);
        });
    }
    response
}

/// The Motion group: the nine measured motions in the draft's order as
/// label/field pairs, two a row, the dribbling motion between the kicks
/// and the celebrations (PES 20+, drawn disabled with the version tooltip
/// before it), Randomize under the grid.
fn motion_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    motion: &PlayerMotion,
    draw: &mut dyn FnMut(u8) -> u8,
) {
    // Two pairs a row (the draft's `COL_PANELS` motion `[2, 194, 14]`).
    let per_row = 2;
    egui::Grid::new("appearance motion")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            let mut pair = 0usize;
            for field in &MOTIONS[..7] {
                motion_number(ui, session, player_id, version, field, motion);
                pair += 1;
                maybe_end_row(ui, pair, per_row);
            }
            // The dribbling motion sits between the kicks and the celebrations
            // (the draft's second row's last cell); a version without it draws
            // it disabled at 0 with the `PES NN+` tooltip.
            ui.label(DRIBBLING_MOTION_LABEL);
            match motion.dribbling {
                Some(stored) => {
                    let mut shown = stored;
                    let range = number_range(SettingKey::Dribbling);
                    let mut response = number_arrows(ui, &mut shown, range.clone(), |ui, value| {
                        ui.add(
                            egui::DragValue::new(value)
                                .range(range.clone())
                                .clamp_existing_to_range(false),
                        )
                    });
                    if step_number(&mut shown, wheel_steps(ui, &response), range) {
                        response.mark_changed();
                    }
                    if response.changed() {
                        edit_player(session, player_id, response.id.value(), |player| {
                            player.motion.dribbling = Some(shown);
                        });
                    }
                }
                None => {
                    add_enabled_ui(ui, false, |ui| {
                        ui.add(
                            egui::DragValue::new(&mut 0u8)
                                .range(number_range(SettingKey::Dribbling)),
                        );
                    })
                    .response
                    .on_disabled_hover_text(gating_tooltip(PlayerField::DribblingMotion));
                }
            }
            pair += 1;
            maybe_end_row(ui, pair, per_row);
            for field in &MOTIONS[7..] {
                motion_number(ui, session, player_id, version, field, motion);
                pair += 1;
                maybe_end_row(ui, pair, per_row);
            }
        });
    // Every motion but the dribbling motion, from 1 to the version's shown
    // top, one undoable edit. The measured tops, not two fixed sets, so a
    // draw never writes a motion the game does not have and reaches every
    // one it has.
    if ui
        .button("Randomize")
        .on_hover_text("Set every motion to a random value, except the dribbling motion")
        .clicked()
    {
        randomize(session, player_id, version, draw);
    }
}

/// One motion's label/`DragValue` pair on the shown scale: a 1-based motion
/// shows 1 to `motion_top` + 1 (stored one below), a celebration 0 to the
/// top (stored as shown).
fn motion_number(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    field: &MotionField,
    motion: &PlayerMotion,
) {
    let top = motion_top(version, field.field).expect("a motion field has a top");
    let stored = (field.get)(motion);
    let (shown, range) = if field.one_based {
        (stored + 1, 1..=top + 1)
    } else {
        (stored, 0..=top)
    };
    let set = field.set;
    let one_based = field.one_based;
    number(
        ui,
        session,
        player_id,
        field.label,
        shown,
        range,
        move |player, shown| {
            set(
                &mut player.motion,
                if one_based { shown - 1 } else { shown },
            );
        },
    );
}

/// Randomize (the draft's button): every motion but the dribbling motion
/// drawn from 1 to its shown top, the writes landing as one undo entry.
fn randomize(
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    draw: &mut dyn FnMut(u8) -> u8,
) {
    edit_player(
        session,
        player_id,
        egui::Id::unique(("appearance randomize", player_id)).value(),
        |player| {
            for field in MOTIONS {
                let top = motion_top(version, field.field).expect("a motion field has a top");
                let shown_top = if field.one_based { top + 1 } else { top };
                let drawn = draw(shown_top);
                (field.set)(
                    &mut player.motion,
                    if field.one_based { drawn - 1 } else { drawn },
                );
            }
        },
    );
}

/// The Colors group's two swatch rows (the draft's Colours group): skin
/// and iris, each its palette's entries as swatches, the two rows' first
/// squares at one x. The skin palette drops the Custom entry on Fox (PES
/// 15 to 17 alone have it, the three pre-Fox versions).
fn colour_rows(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
) {
    egui::Grid::new("appearance colours")
        .num_columns(2)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            for key in [SettingKey::SkinColor, SettingKey::IrisColor] {
                let field = key
                    .face_field()
                    .expect("a colour palette's is a face field");
                let palette = palette(version, key);
                let stored = face(appearance, field);
                ui.label(palette.label);
                ui.horizontal(|ui| {
                    palette_row(ui, session, player_id, &palette, stored, |player, shown| {
                        player
                            .appearance
                            .ingame_face
                            .set(field, shown)
                            .expect("a loaded player's face run holds the section's colour fields");
                    });
                });
                ui.end_row();
            }
        });
}

/// One checkbox pair, the label inside the widget (the edit flags'
/// shape; `""` draws the bare box — the strip grid's checkboxes'). A
/// change writes `set` through `edit_player` under the widget's id.
fn checkbox(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    held: bool,
    set: impl FnOnce(&mut PlayerEntry, bool),
) {
    let mut value = held;
    let response = studio_core::widgets::checkbox(ui, &mut value, label);
    if response.changed() {
        edit_player(session, player_id, response.id.value(), |player| {
            set(player, value);
        });
    }
}

/// A strip cell's `ComboBox`, `FIELD_W` wide, `options` its rows' labels
/// in pick order (`combo_options`' for a `SettingKey`, `GLOVES_OPTIONS`' for
/// the outfielder gloves); `stored` the held index, its number shown when
/// past the list. A pick, or a wheel notch stepping through the rows,
/// writes `set`.
fn combo_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    options: &[String],
    stored: u8,
    set: impl FnOnce(&mut PlayerEntry, u8),
) {
    let mut picked = stored;
    let selected = options
        .get(usize::from(stored))
        .cloned()
        .unwrap_or_else(|| stored.to_string());
    let index = |at: usize| u8::try_from(at).expect("the options fit a u8");
    let combo = egui::ComboBox::from_id_salt(("appearance", label))
        .selected_text(selected)
        .width(FIELD_W)
        .show_ui(ui, |ui| {
            for (at, option) in options.iter().enumerate() {
                combo_item(ui, &mut picked, index(at), option);
            }
        });
    let steps = wheel_steps(ui, &combo.response);
    if let Some(next) = step_choice((0..options.len()).map(index), &picked, steps) {
        picked = next;
    }
    if picked != stored {
        edit_player(session, player_id, combo.response.id.value(), |player| {
            set(player, picked)
        });
    }
}

/// One palette row (the draft's `paletteRow`): a framed swatch per palette
/// entry 0 to `palette.top`, the entry's name or index in its widget label
/// and hover; a stored value past `top` draws its number after the
/// swatches. A click writes the swatch's index through `edit_player`.
fn palette_row(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    palette: &Palette,
    stored: u8,
    set: impl Fn(&mut PlayerEntry, u8),
) {
    // The swatch names once per row (each is `option_name`'s entry: the
    // palette keys' `Kind::Number` ranges start at 0, so `combo_options`
    // holds one entry per index up to the key's top, and every drawn
    // index has one).
    let names = combo_options(palette.key);
    ui.push_id(("appearance palette", palette.label), |ui| {
        for (index, rgb) in palette.colors[..=usize::from(palette.top)]
            .iter()
            .enumerate()
        {
            let name = &names[index];
            let widget_label = format!("{} {name}", palette.label);
            let response =
                swatch(ui, *rgb, index == usize::from(stored), &widget_label).on_hover_text(name);
            if response.clicked() {
                edit_player(session, player_id, response.id.value(), |player| {
                    set(
                        player,
                        u8::try_from(index).expect("a palette has at most eleven entries"),
                    );
                });
            }
        }
        if stored > palette.top {
            ui.weak(stored.to_string());
        }
    });
}

/// One palette entry's framed square (the draft's `.swatch`): the colour in
/// a dark frame, the held entry outlined as `.on` is.
fn swatch(ui: &mut egui::Ui, rgb: [u8; 3], held: bool, label: &str) -> egui::Response {
    let size = egui::vec2(14.0, 14.0);
    let (rect, response) = ui.allocate_at_least(size, egui::Sense::click());
    response.widget_info(|| {
        let mut info =
            egui::WidgetInfo::labeled(egui::Role::ColorWell, ui.is_enabled(), label.to_owned());
        info.selected = Some(held);
        info
    });
    if ui.is_rect_visible(rect) {
        let radius = ui.visuals().widgets.noninteractive.corner_radius;
        ui.painter().rect_filled(
            rect,
            radius,
            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        ui.painter().rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.0, studio_core::theme::palette(ui.visuals()).swatch_edge),
            egui::StrokeKind::Inside,
        );
        if held {
            ui.painter().rect_stroke(
                rect.expand(1.0),
                radius,
                egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                egui::StrokeKind::Outside,
            );
        }
    }
    response
}

/// `field`'s value inside `appearance`'s ingame-face run: the run a loaded
/// save's players always carry, and a field `layout()` names for every
/// version (the four the section edits are `None`, both layouts).
fn face(appearance: &PlayerAppearance, field: IngameFaceField) -> u8 {
    appearance
        .ingame_face
        .get(field)
        .expect("a loaded player's face run holds the section's colour fields")
}

/// A xorshift64 (13, 7, 17) draw in `1..=top` from `seed` (the Randomize
/// source). The picks need only be unpredictable to a member, not
/// cryptographic.
pub(crate) fn xorshift_draw(seed: u64) -> impl FnMut(u8) -> u8 {
    let mut state = seed;
    move |top| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        1 + u8::try_from(state % u64::from(top.max(1))).expect("a remainder below a u8 top fits u8")
    }
}

/// `xorshift_draw` seeded from the wall clock (`SaveView::new` wires it as
/// `randomize_draw`; the tests substitute a fixed source).
pub(crate) fn seeded_draw() -> impl FnMut(u8) -> u8 {
    xorshift_draw(clock_seed())
}

/// The wall-clock seed for `seeded_draw`, forced odd: a zero state draws 1
/// forever.
fn clock_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| odd_seed(u64::from(since.subsec_nanos()), since.as_secs()))
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

/// `clock_seed`'s arithmetic as a pure function (its test needs even
/// inputs, which a real clock does not hand it on demand): `nanos ^ secs`
/// forced odd, a zero state drawing 1 forever.
fn odd_seed(nanos: u64, secs: u64) -> u64 {
    nanos ^ secs | 1
}

#[cfg(test)]
mod tests {
    use super::{clock_seed, odd_seed, seeded_draw, xorshift_draw};

    // The Randomize generator is xorshift64 (13, 7, 17): each draw is
    // `1 + state % top` after one round of the three shifts. The first
    // state from seed 1 is 1082269761; the three draws' states are stepped
    // out here by hand, so the test never calls the code under test.
    #[test]
    fn xorshift_draw_returns_the_states_remainders_plus_one() {
        let mut state = 1u64;
        let mut expected = Vec::new();
        for _ in 0..3 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            expected.push(1 + u8::try_from(state % 162).expect("a remainder fits u8"));
        }
        let mut first_state = 1u64;
        first_state ^= first_state << 13;
        first_state ^= first_state >> 7;
        first_state ^= first_state << 17;
        assert_eq!(first_state, 1082269761, "the pinned first state");
        let mut draw = xorshift_draw(1);
        let drawn: Vec<u8> = (0..3).map(|_| draw(162)).collect();
        assert_eq!(drawn, expected);
    }

    // The clock seed must be odd (a zero state draws 1 forever).
    #[test]
    fn the_clock_seed_is_odd() {
        assert_eq!(clock_seed() % 2, 1, "an even or zero seed draws 1 forever");
    }

    // The forcing is the `| 1`: on even inputs `& 1` would hand back 0.
    #[test]
    fn the_odd_seed_is_odd_on_even_inputs() {
        // Exact values: an odd-only check passes a constant 1, and on
        // (2, 4) `| 1` -> `^ 1` gives 7 too; on (1, 2) it gives 2.
        assert_eq!(odd_seed(2, 4), 7, "2 ^ 4 forced odd");
        assert_eq!(odd_seed(1, 2), 3, "1 ^ 2, already odd, kept");
    }

    // `seeded_draw` returns values in `1..=top`, both ends occurring over a
    // long enough run (the source need only be unpredictable to a member).
    #[test]
    fn the_seeded_draw_covers_the_whole_range() {
        let mut draw = seeded_draw();
        for top in [1u8, 5, 162] {
            let (mut low, mut high) = (u8::MAX, u8::MIN);
            for _ in 0..10_000 {
                let value = draw(top);
                assert!((1..=top).contains(&value), "{value} out of 1..={top}");
                low = low.min(value);
                high = high.max(value);
            }
            assert_eq!((low, high), (1, top), "top {top}: both ends occur");
        }
    }
}
