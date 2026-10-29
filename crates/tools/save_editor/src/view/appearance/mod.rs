//! The Appearance section (the draft's `appearanceTab` and `paletteRow`):
//! the Cutscenes group first (star and playing attitude,
//! which change only cutscenes and so sit outside the lock), then the
//! groups — Edit flags, Physique, Strip style, Motion, Colours. With the
//! "Aesthetics from exports" setting off the section edits as every other does;
//! with it on, the ownership line and the "Edit aesthetics anyway" switch
//! (`SaveView`'s `edit_aesthetics`, off at every start of the tool) draw
//! over the groups, every control disabled under the ownership line as
//! hover and the switch re-enabling them for the rest of the run. A
//! stripped player gets the draft's stripped line either way. Edits go
//! through `Session::edit_player` under the widget's id,
//! one undo entry per gesture, as `abilities.rs` does; a stored value past
//! a control's range draws as stored (`clamp_existing_to_range` off) and
//! changes only when edited.

pub(super) mod fields;

use pes_savefile::model::{
    PlayerAppearance, PlayerEditFlags, PlayerEntry, PlayerMotion, PlayerStats,
};
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::ingame_face::IngameFaceField;
use pes_savefile::schema::limits::motion_top;
use pes_savefile::schema::player_max_value;
use pes_savefile::settings_toml::{Kind, SettingKey};
use pes_version::{Engine, PesVersion};
use studio_core::widgets::{FORM_ROW_SPACING, step_choice, step_number, wheel_steps};

use crate::session::{Session, is_stripped};
use crate::view::abilities::{gating_tooltip, maybe_end_row, number, small_stat};
use crate::view::appearance::fields::{
    MOTIONS, MotionField, PHYSIQUE, Palette, STRIP_ENUMS, combo_options, number_range, option_name,
    palette,
};
use crate::view::{SaveView, edit_player, selected_player};

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
/// The wrist taping combo.
pub(super) const WRIST_TAPING_LABEL: &str = "Wrist taping";
/// The spectacles combo.
pub(super) const SPECTACLES_LABEL: &str = "Spectacles";
/// The player-gloves checkbox.
pub(super) const PLAYER_GLOVES_LABEL: &str = "Player gloves";
/// The dribbling motion's number.
pub(super) const DRIBBLING_MOTION_LABEL: &str = "Dribbling motion";

/// The Appearance section's content for the selected player, inside the
/// player view's collapsing section.
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
    let (version, stripped, player) = {
        let file = session.file();
        let Some(player) = file.player(player_id) else {
            return;
        };
        (file.version(), is_stripped(file, player_id), player.clone())
    };
    // Every widget id under the section is the player's within this open,
    // its own salt since it draws beside the Abilities & Skills content
    // (whose `push_id` salts the same pair): two scopes with one salt would
    // hand two widgets one id — an in-progress edit and its undo key would
    // cross between the sections.
    let mut open_patch = false;
    ui.push_id(("appearance", generation, player_id), |ui| {
        cutscenes(ui, session, player_id, &player.stats);
        if setting {
            open_patch = ownership_line(ui, version);
            studio_core::widgets::checkbox(ui, edit_aesthetics, "Edit aesthetics anyway");
            if *edit_aesthetics {
                ui.weak(match version.engine() {
                    Engine::PreFox => "A later patch overwrites whatever is edited by hand.",
                    Engine::Fox => {
                        "On a stripped save the game still reads the motions, \
                         but the look comes from the database tables."
                    }
                });
            }
        }
        if stripped {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This player is stripped, so the game takes his look from the database \
                 tables, but it still reads the motions.",
            );
        }
        ui.add_enabled_ui(!locked, |ui| {
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
        .on_disabled_hover_text(ownership_tip(version));
    });
    // One floating panel at a time, as the Savefile menu's entries.
    if open_patch {
        view.patch.open = true;
        view.comparator.open = false;
    }
}

/// The five groups, the draft's order.
fn groups(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    player: &PlayerEntry,
    draw: &mut dyn FnMut(u8) -> u8,
    hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
) {
    ui.strong("Edit flags");
    ui.horizontal_wrapped(|ui| {
        edit_flags(
            ui,
            session,
            player_id,
            &player.edit_flags,
            player.appearance.base_copy_id,
        );
    });
    ui.separator();
    ui.strong(PHYSIQUE_LABEL);
    physique_grid(ui, session, player_id, &player.appearance);
    ui.separator();
    ui.strong("Strip style");
    strip_rows(ui, session, player_id, version, &player.appearance);
    ui.separator();
    ui.strong(MOTION_LABEL);
    motion_grid(ui, session, player_id, version, &player.motion, draw);
    ui.separator();
    ui.strong("Colours");
    colour_rows(ui, session, player_id, version, &player.appearance);
    ui.separator();
    ui.strong("Full Player Customization");
    fpc_group(ui, session, player_id, player, hide_ids);
}

/// The Cutscenes group (the draft's `#cutscenes`), first in the section and
/// outside the lock: star and playing attitude change only cutscenes, so
/// they are no aesthetic the export owns — editable while the groups
/// below stay locked, gated per version like the other fields.
fn cutscenes(ui: &mut egui::Ui, session: &mut Session, player_id: u32, stats: &PlayerStats) {
    ui.strong("Cutscenes");
    ui.weak("These only change how the player appears in cutscenes.");
    egui::Grid::new("appearance cutscenes")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            for small in &fields::CUTSCENE_STATS {
                small_stat(ui, session, player_id, stats, small);
            }
            ui.end_row();
        });
}

/// The ownership row: the sentence itself, and on pre-Fox the patch step
/// and the Apply aesthetics patch link. Whether the link was clicked: the
/// caller opens the patch panel.
fn ownership_line(ui: &mut egui::Ui, version: PesVersion) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.label(ownership_tip(version));
        match version.engine() {
            Engine::PreFox => ui.link("Apply aesthetics patch").clicked(),
            Engine::Fox => false,
        }
    })
    .inner
}

/// The ownership sentence (the draft's `ownershipTip`): the line over the
/// section, the disabled groups' hover and the locked Team-menu items'
/// (`menus.rs`).
pub(crate) fn ownership_tip(version: PesVersion) -> &'static str {
    match version.engine() {
        Engine::PreFox => {
            "The team's export owns the aesthetics, so edit its settings.toml and \
             recompile, then apply the patch."
        }
        Engine::Fox => {
            "The team's export owns the aesthetics, so edit its settings.toml and recompile."
        }
    }
}

/// The FPC group, last in the section (the draft's `fpcGroup`): the
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
                    ui.add_enabled(false, egui::Button::new(label))
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
                "Apply the hide preset: long sleeves, a tucked shirt and short socks. \
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
                "Apply the show preset: short sleeves, an untucked shirt and standard \
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

/// The edit flags' row (the draft's Edit flags group): the six flags, then
/// the copy-from id. `base_copy` exists on PES 15 to 18 only; like a gated
/// field it keeps its place, disabled. Ticking it also sets the seven
/// gameplay flags, since a copy's sections count as edited; unticking
/// clears `base_copy` alone.
fn edit_flags(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    flags: &PlayerEditFlags,
    base_copy_id: u32,
) {
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
            ui.add_enabled(false, |ui: &mut egui::Ui| {
                studio_core::widgets::checkbox(ui, &mut false, BASE_COPY_LABEL)
            })
            .on_disabled_hover_text("PES 15 to 18 only");
        }
    }
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
}

/// The 14 physique fields in an aligned grid of label and field cells,
/// the draft's two pairs a row. Each field is on its key's `Signed`
/// scale (±7: `shown - limit` stores `shown + limit`).
fn physique_grid(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    appearance: &PlayerAppearance,
) {
    let per_row = 2;
    egui::Grid::new("appearance physique")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            for (index, (label, key, get, set)) in PHYSIQUE.iter().enumerate() {
                let Kind::Signed { limit } = key.spec().kind else {
                    unreachable!("the physique keys are Signed");
                };
                let limit = i32::from(limit);
                let stored = get(appearance);
                let mut shown = i32::from(stored) - limit;
                ui.label(*label);
                let mut response = ui.add(
                    egui::DragValue::new(&mut shown)
                        .range(-limit..=limit)
                        .clamp_existing_to_range(false),
                );
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
                maybe_end_row(ui, index + 1, per_row);
            }
        });
}

/// The draft's Strip style group: the ids, the enums and the two
/// checkboxes as label and field cells in one grid (the draft's four
/// pairs a row — its two table rows fall out at four: the ids and the
/// first two enums, then the rest with the checkboxes), then the
/// wrist-taping, spectacles and player-gloves rows as a second grid, a
/// label cell and a contents cell each. The draft's
/// colspan rows cannot share the first grid (an egui grid has no span),
/// so their labels align among themselves instead.
fn strip_rows(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
) {
    let per_row = 4;
    egui::Grid::new("appearance strip")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            // The id fields take the record's own bound (`player_max_value`,
            // wider than `settings.toml`'s stock range): a compile-allocated
            // ID such as 627 is a value the save holds.
            number(
                ui,
                session,
                player_id,
                BOOTS_ID_LABEL,
                appearance.boots_id,
                0..=u32::try_from(
                    player_max_value(version, PlayerField::BootsId)
                        .expect("the boots id is stored on every version"),
                )
                .expect("the boots id's bound fits u32"),
                |player, shown| {
                    player.appearance.boots_id = shown;
                },
            );
            maybe_end_row(ui, 1, per_row);
            number(
                ui,
                session,
                player_id,
                GLOVES_ID_LABEL,
                appearance.gloves_id,
                0..=u32::try_from(
                    player_max_value(version, PlayerField::GlovesId)
                        .expect("the gloves id is stored on every version"),
                )
                .expect("the gloves id's bound fits u32"),
                |player, shown| {
                    player.appearance.gloves_id = shown;
                },
            )
            .on_hover_text("Any player's gloves ID works; 0 means normal hands.");
            maybe_end_row(ui, 2, per_row);
            for (index, (label, key, get, set)) in STRIP_ENUMS.iter().enumerate() {
                combo(
                    ui,
                    session,
                    player_id,
                    label,
                    *key,
                    get(appearance),
                    move |player, shown| {
                        set(&mut player.appearance, shown);
                    },
                );
                maybe_end_row(ui, 3 + index, per_row);
            }
            // The checkboxes draw as a label cell and a bare box in the field
            // cell (the header's Captain shape).
            bare_checkbox(
                ui,
                session,
                player_id,
                SHIRTTAIL_OUT_LABEL,
                appearance.untucked,
                |p, v| {
                    p.appearance.untucked = v;
                },
            );
            maybe_end_row(ui, 3 + STRIP_ENUMS.len(), per_row);
            bare_checkbox(
                ui,
                session,
                player_id,
                ANKLE_TAPING_LABEL,
                appearance.ankle_taping,
                |p, v| {
                    p.appearance.ankle_taping = v;
                },
            );
        });
    egui::Grid::new("appearance strip rows")
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .show(ui, |ui| {
            strip_row(ui, WRIST_TAPING_LABEL, |ui| {
                combo_box(
                    ui,
                    session,
                    player_id,
                    WRIST_TAPING_LABEL,
                    SettingKey::WristTaping,
                    appearance.wrist_taping,
                    |player, shown| {
                        player.appearance.wrist_taping = shown;
                    },
                );
                ui.label("Left");
                palette_row(
                    ui,
                    session,
                    player_id,
                    &palette(version, SettingKey::WristTapeColorLeft),
                    appearance.wrist_tape_color_left,
                    |player, shown| {
                        player.appearance.wrist_tape_color_left = shown;
                    },
                );
                ui.label("Right");
                palette_row(
                    ui,
                    session,
                    player_id,
                    &palette(version, SettingKey::WristTapeColorRight),
                    appearance.wrist_tape_color_right,
                    |player, shown| {
                        player.appearance.wrist_tape_color_right = shown;
                    },
                );
            });
            strip_row(ui, SPECTACLES_LABEL, |ui| {
                combo_box(
                    ui,
                    session,
                    player_id,
                    SPECTACLES_LABEL,
                    SettingKey::Spectacles,
                    appearance.spectacles_style,
                    |player, shown| {
                        player.appearance.spectacles_style = shown;
                    },
                );
                palette_row(
                    ui,
                    session,
                    player_id,
                    &palette(version, SettingKey::SpectaclesColor),
                    appearance.spectacles_color,
                    |player, shown| {
                        player.appearance.spectacles_color = shown;
                    },
                );
            });
            strip_row(ui, PLAYER_GLOVES_LABEL, |ui| {
                let gloves = face(appearance, IngameFaceField::PlayerGloves);
                checkbox(ui, session, player_id, "", gloves != 0, |p, v| {
                    p.appearance
                        .ingame_face
                        .set(IngameFaceField::PlayerGloves, u8::from(v))
                        .expect("a loaded player's face run holds player gloves");
                });
                let gloves_color = face(appearance, IngameFaceField::PlayerGlovesColor);
                palette_row(
                    ui,
                    session,
                    player_id,
                    &palette(version, SettingKey::GlovesColor),
                    gloves_color,
                    |player, shown| {
                        player
                            .appearance
                            .ingame_face
                            .set(IngameFaceField::PlayerGlovesColor, shown)
                            .expect("a loaded player's face run holds the gloves colour");
                    },
                );
            });
        });
}

/// One strip row of the second grid: `label`'s cell, then `add`'s
/// controls in one `ui.horizontal`, `end_row` (the draft's colspan
/// rows' shape).
fn strip_row(ui: &mut egui::Ui, label: &'static str, add: impl FnOnce(&mut egui::Ui)) {
    ui.label(label);
    ui.horizontal(add);
    ui.end_row();
}

/// The draft's Motion group: the nine measured motions in the draft's
/// order as label/field pairs, the draft's two a row as a cap a narrower
/// pane lowers, the dribbling motion
/// between the kicks and the celebrations (PES 20+, drawn disabled with
/// the version tooltip before it).
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
                    let mut response = ui.add(
                        egui::DragValue::new(&mut shown)
                            .range(range.clone())
                            .clamp_existing_to_range(false),
                    );
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
                    ui.add_enabled_ui(false, |ui| {
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

/// The draft's Colours group: skin and iris, each its palette's entries as
/// swatches. The skin palette drops the Custom entry on Fox (PES 15 to 17
/// alone have it, the three pre-Fox versions).
fn colour_rows(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    version: PesVersion,
    appearance: &PlayerAppearance,
) {
    for key in [SettingKey::SkinColor, SettingKey::IrisColor] {
        let field = key
            .face_field()
            .expect("a colour palette's is a face field");
        let palette = palette(version, key);
        let stored = face(appearance, field);
        ui.horizontal_wrapped(|ui| {
            ui.label(palette.label);
            palette_row(ui, session, player_id, &palette, stored, |player, shown| {
                player
                    .appearance
                    .ingame_face
                    .set(field, shown)
                    .expect("a loaded player's face run holds the section's colour fields");
            });
        });
    }
}

/// One checkbox pair, the label inside the widget (the edit flags'
/// shape; `""` draws the bare box — the strip grids' checkboxes and the
/// gloves row's). A change writes `set` through `edit_player` under the
/// widget's id.
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

/// `label` in its own cell, then `checkbox`'s bare box in the field cell
/// (the strip grids' pair shape, as the header's Captain cell); the same
/// write.
fn bare_checkbox(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    held: bool,
    set: impl FnOnce(&mut PlayerEntry, bool),
) {
    ui.label(label);
    checkbox(ui, session, player_id, "", held, set);
}

/// One label/`ComboBox` pair over `key`'s option labels (`combo_options`);
/// a stored index past the list shows its number, a pick writes it.
fn combo(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    key: SettingKey,
    stored: u8,
    set: impl FnOnce(&mut PlayerEntry, u8),
) {
    ui.label(label);
    combo_box(ui, session, player_id, label, key, stored, set);
}

/// `combo`'s `ComboBox` alone, for a caller whose label sits in its own
/// cell (the strip rows' grid).
fn combo_box(
    ui: &mut egui::Ui,
    session: &mut Session,
    player_id: u32,
    label: &'static str,
    key: SettingKey,
    stored: u8,
    set: impl FnOnce(&mut PlayerEntry, u8),
) {
    let options = combo_options(key);
    let mut picked = stored;
    let selected = option_name(key, stored);
    let index = |at: usize| u8::try_from(at).expect("the key's options fit a u8");
    let combo = egui::ComboBox::from_id_salt(("appearance", label))
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (at, option) in options.iter().enumerate() {
                ui.selectable_value(&mut picked, index(at), option);
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
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(2),
            egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(2),
            egui::Stroke::new(1.0, studio_core::theme::palette(ui.visuals()).swatch_edge),
            egui::StrokeKind::Inside,
        );
        if held {
            ui.painter().rect_stroke(
                rect.expand(1.0),
                egui::CornerRadius::same(2),
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
