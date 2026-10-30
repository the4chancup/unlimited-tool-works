//! The team view's Tactics editor (the draft's `tacticsTab`):
//! `team_widgets`' pitch, lineup lists and tactics panels in the draft's
//! one grid — three columns (the settings, the pitch, the assignments) and
//! three rows, so every panel lines up with its neighbours — plus the
//! editor's own Copy/swap menus (four entries each on the selected preset
//! or formation; `session::tactics_ops`) and the Scroll names view
//! setting. The widgets edit a clone of the team's
//! tactics and report a change; the view pushes the clone back with
//! `Session::edit_team` under one key per widget call (a panel's or a
//! bar's), so a gesture's frames merge but different widgets' edits
//! never do (`seal` runs when the pointer is up and no widget has
//! focus). The grid is laid out at the design width: a narrower window
//! scales the whole UI (`spec/core/gui.md` "Scaling with the width"), so
//! there is no narrow fallback.

use std::sync::Arc;

use aatf::TeamView;
use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::PLAYABLE_POSITIONS;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{checkbox, menu_button};
use team_widgets::pitch::{self, FORMATIONS};
use team_widgets::tactics::{self, FieldedMark, Medal, RosterRow};
use team_widgets::violations::{Wording, violation_line};

use crate::session::Session;
use crate::view::worker::{Pending, WorkerResult};
use crate::view::{MedalKey, SaveView, edit_settled, edit_team, live_marks, selected_team};

/// The settings column's width (the draft's `.tacgrid` first track).
const SETTINGS_WIDTH: f32 = 422.0;
/// The gap between the grid's columns (the draft's `column-gap: 10px`).
const COLUMN_GAP: f32 = 10.0;
/// The pitch's column (the draft's `.tacgrid` second track): the canvas
/// alone, its 1-point border a side overflowing it, so the assignments'
/// column starts 8 past the pitch's border, not 10.
const PITCH_TRACK: f32 = 320.0;
/// The gap between the grid's rows (the draft's `row-gap: 6px`).
const ROW_GAP: f32 = 6.0;
/// The preset and formation bars' height (the draft's `.tacbar`): its
/// tallest item is the preset selector's `.row`, 17 points with a 3-point
/// margin above and below, so the bar is 5 taller than its buttons.
const BAR_HEIGHT: f32 = 23.0;

/// One roster slot's row data before the `RosterRow`s borrow it: the
/// display name, the raw name, the registered position, the thirteen
/// per-position overalls and the positions he is rated A or B at.
type RosterSlot = (String, String, Option<u8>, Option<[u8; 13]>, [bool; 13]);

/// The `RosterSlot` of the roster slot holding `player`: his display name
/// as the player list shows it, his raw name for the colour-coded rows,
/// his registered position, the thirteen per-position overalls the
/// badge's number and the drag's position pick read, and the positions he
/// is rated A or B at, which a held marker lights. `None`, an empty slot,
/// gives "", none and nothing rated.
pub(super) fn roster_slot(player: Option<&PlayerEntry>) -> RosterSlot {
    let Some(player) = player else {
        return (String::new(), String::new(), None, None, [false; 13]);
    };
    let mut overalls = [0; 13];
    for (position, overall) in (0u8..).zip(&mut overalls) {
        *overall = player
            .overall_at(position)
            .expect("overall_at is None only past CF");
    }
    // `playable` runs CF first, GK last, not in the position byte's
    // order; a rating of 1 is B, 2 A, and 3, held by no save, counts as
    // A, as `overall_at` reads it.
    let mut rated = [false; 13];
    for (entry, rating) in player.positions.playable.iter().enumerate() {
        rated[PLAYABLE_POSITIONS[entry]] = *rating >= 1;
    }
    (
        display_name(&player.name),
        player.name.clone(),
        Some(player.positions.registered),
        Some(overalls),
        rated,
    )
}

/// The editor for the selected row's team (a player row's or a team
/// row's own), under the team view's fields.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let Some((team_id, version, team)) = selected_team(view) else {
        return;
    };
    let SaveView {
        session,
        tactics,
        generation,
        focused,
        portraits,
        rules,
        medals,
        live,
        live_aatf,
        ..
    } = view;
    let Some(session) = session.as_mut() else {
        return;
    };
    // The roster as the widgets list it, indexed by roster slot.
    let rows: Vec<RosterSlot> = team
        .roster
        .iter()
        .map(|slot| roster_slot(session.file().player(slot.player_id)))
        .collect();
    // The roster's player ids (`None` an empty slot): the portrait source's ask.
    let player_ids: Vec<Option<u32>> = team
        .roster
        .iter()
        .map(|slot| {
            session
                .file()
                .player(slot.player_id)
                .map(|player| player.id)
        })
        .collect();
    let textures = portraits.textures(ui.ctx(), &player_ids);
    // The badges' medal outlines: `player_tier` per roster slot under the
    // active ruleset, once per `MedalKey` (the open save's revision, the
    // team, the ruleset) as the Cards left badge is — not every frame.
    // On a worker thread, started once the edit settles: the 23 script
    // calls took about 95 ms on a PES 2021 save's team in a release build,
    // all on the team view's first frame. Until the answer for this key
    // lands the badges draw no outline, never another key's.
    let held_medals = rules.as_ref().and_then(|active| {
        let key = MedalKey {
            revision: session.revision(),
            team: team_id,
            ruleset: active.key.clone(),
        };
        let (_, settled) = edit_settled(ui, *focused);
        medals.update(&key, true, settled, |key| {
            spawn_medals(
                ui.ctx(),
                session.file(),
                &active.rules,
                key.clone(),
                team.roster.len(),
            )
        });
        medals
            .result
            .as_ref()
            .filter(|(held, _)| *held == key)
            .map(|(_, marks)| marks)
    });
    // Each roster slot's `FieldedNotA` marks: the held check's Error or
    // Warning violations of that kind on the slot, while Live AATF
    // holds this team's result (`held_violations`; another team's
    // result or none marks nothing), worded under the active rules and
    // the open save's stat names.
    let held = live_marks::held_violations(*live_aatf, &live.check)
        .filter(|(held_team, _)| *held_team == team_id)
        .zip(rules.as_ref());
    let fielded: Vec<Vec<FieldedMark>> = (0..team.roster.len())
        .map(|slot| {
            let Some(((_, violations), active)) = held else {
                return Vec::new();
            };
            violations
                .iter()
                .filter(|violation| {
                    violation.slot.is_some_and(|s| usize::from(s) == slot)
                        && matches!(
                            violation.severity,
                            aatf::Severity::Error | aatf::Severity::Warning
                        )
                })
                .filter_map(|violation| {
                    if let aatf::ViolationKind::FieldedNotA {
                        preset, position, ..
                    } = violation.kind
                    {
                        Some(FieldedMark {
                            preset: usize::from(preset) - 1,
                            position: position.byte(),
                            error: violation.severity == aatf::Severity::Error,
                            line: violation_line(
                                violation,
                                team_id,
                                Wording {
                                    ruleset: active.rules.ruleset(),
                                    version,
                                },
                            ),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();
    let roster: Vec<RosterRow> = rows
        .iter()
        .zip(textures)
        .enumerate()
        .map(
            |(index, ((name, raw_name, position, overalls, rated), portrait))| RosterRow {
                name: name.as_str(),
                raw_name: raw_name.as_str(),
                registered_position: *position,
                portrait,
                medal: held_medals
                    .and_then(|marks| marks.get(index))
                    .copied()
                    .flatten(),
                overalls: *overalls,
                rated: *rated,
                fielded: fielded[index].as_slice(),
            },
        )
        .collect();
    // Every widget id under the editor is the team's within this open
    // (the view's own salt on top of the team scope's): a combo's popup,
    // a DragValue's in-progress edit and the widget's state never cross
    // teams nor saves.
    ui.push_id((*generation, team_id), |ui| {
        let mut edited = team.tactics.clone();
        let mut changed = [false; 6];
        // The draft's `.tacgrid`: the settings column's 422, the pitch's
        // track (its canvas, the border overflowing it), the assignments'
        // the rest — a cell a child `Ui` like the old columns', each row's height
        // its tallest cell's (row 2's cells end at the pitch's bottom,
        // row 3's run to the pane's; the `min_rect`s read back the drawn
        // height).
        let top_left = ui.cursor().min;
        let right = ui.max_rect().right();
        let bottom = ui.max_rect().right_bottom().y;
        let settings_right = top_left.x + SETTINGS_WIDTH;
        let pitch_left = settings_right + COLUMN_GAP;
        let assigns_left = pitch_left + PITCH_TRACK + COLUMN_GAP;
        let cell = |ui: &mut egui::Ui, left: f32, top: f32, width: f32, bottom: f32| {
            ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(left, top),
                        egui::pos2(left + width, bottom),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            )
        };
        // Row 1: the preset bar on column 1 (the draft's `presetSel`,
        // `fluid`, `presetMenu`), the formation bar over columns 2-3
        // starting at the pitch's left (`formationSel`, `formationMenu`,
        // `scrollNames`) — each `BAR_HEIGHT` tall, their contents centred
        // in it (a full-height cell would centre them in the cell, not on
        // the row).
        let bar = |ui: &mut egui::Ui, left: f32, right_edge: f32| {
            ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_max(
                        egui::pos2(left, top_left.y),
                        egui::pos2(right_edge, top_left.y + BAR_HEIGHT),
                    ))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            )
        };
        let mut preset_ui = bar(ui, top_left.x, settings_right);
        tactics::preset_selector(&mut preset_ui, tactics);
        changed[0] = tactics::fluid_formation(&mut preset_ui, &mut edited, tactics);
        copy_swap_preset_menu(&mut preset_ui, session, team_id, tactics.preset);
        let mut formbar_ui = bar(ui, pitch_left, right);
        pitch::formation_selector(&mut formbar_ui, &edited, tactics);
        copy_swap_formation_menu(&mut formbar_ui, session, team_id, tactics);
        checkbox(&mut formbar_ui, &mut tactics.scroll_names, "Scroll names");
        let row2_top = top_left.y + BAR_HEIGHT + ROW_GAP;

        // Row 2: Formation settings, the pitch, Player assignments —
        // the panels exactly as tall as the pitch (the draft's `.tacgrid`
        // stretches them to its row), their frames level with it.
        let row2_bottom = row2_top + pitch::PITCH_BOX.y;
        let mut settings_ui = cell(ui, top_left.x, row2_top, SETTINGS_WIDTH, row2_bottom);
        changed[1] = tactics::formation_settings(&mut settings_ui, &mut edited, tactics);
        let mut pitch_ui = cell(ui, pitch_left, row2_top, pitch::PITCH_BOX.x, bottom);
        changed[2] = pitch::pitch(
            &mut pitch_ui,
            "pitch",
            version,
            &mut edited,
            tactics,
            &roster,
        );
        let mut assigns_ui = cell(
            ui,
            assigns_left,
            row2_top,
            right - assigns_left,
            row2_bottom,
        );
        tactics::player_assignments(&mut assigns_ui, &edited, tactics, &roster);
        let row3_top = [
            settings_ui.min_rect().max.y,
            pitch_ui.min_rect().max.y,
            assigns_ui.min_rect().max.y,
        ]
        .into_iter()
        .fold(row2_top, f32::max)
            + ROW_GAP;

        // Row 3: the lists over columns 2-3, Advanced instructions over
        // Automatic settings on column 1 (the draft's `tacstack`) — the
        // lists drawn first so the stack's cell can end at their bottom,
        // which is what `#tac-auto`'s `flex: 1` reaches.
        let mut lists_ui = cell(ui, pitch_left, row3_top, right - pitch_left, bottom);
        changed[5] = pitch::lineup(
            &mut lists_ui,
            "lineup",
            version,
            &mut edited,
            tactics,
            &roster,
        );
        let lists_bottom = lists_ui.min_rect().max.y;
        let mut stack_ui = cell(ui, top_left.x, row3_top, SETTINGS_WIDTH, lists_bottom);
        changed[3] =
            tactics::advanced_instructions(&mut stack_ui, version, &mut edited, tactics, &roster);
        // `add_space` adds to the item gap the panel above already left.
        stack_ui.add_space(ROW_GAP - stack_ui.spacing().item_spacing.y);
        changed[4] = tactics::automatic_settings(&mut stack_ui, version, &mut edited);

        // One advance covering the grid's block (the tallest cell).
        let grid_bottom = stack_ui.min_rect().max.y.max(lists_ui.min_rect().max.y);
        ui.advance_cursor_after_rect(egui::Rect::from_min_max(
            top_left,
            egui::pos2(right, grid_bottom),
        ));
        // One undo key per widget call, since only one widget's changes
        // merge into the open entry: the pitch's drag never merges into a
        // settings click or a row drop.
        if let Some(widget) = [
            "preset bar",
            "formation settings",
            "pitch",
            "advanced instructions",
            "automatic settings",
            "lineup",
        ]
        .into_iter()
        .zip(changed)
        .find_map(|(widget, changed)| changed.then_some(widget))
        {
            // A gesture ends mid-frame when its widget loses the
            // keyboard (a press elsewhere surrenders it): the press's
            // write lands on this same frame under the same key, and
            // `view::show`'s seal runs only after the push. Seal here or
            // it merges into the ended gesture's entry, and record the
            // transition handled: the frame-end check must not seal this
            // gesture's own entry again.
            let now = ui.ctx().memory(|memory| memory.focused());
            if focused.is_some() && *focused != now {
                session.seal();
                *focused = now;
            }
            edit_team(
                session,
                team_id,
                egui::Id::unique(("tactics", team_id, widget)).value(),
                |team| team.tactics = edited,
            );
        }
    });
}

/// The preset bar's Copy/swap menu (the draft's `presetMenu`): acting on
/// the selected preset, a copy to, then a swap with, each of the other
/// two: four entries rather than all nine copy and swap pairs, each a
/// sealed undo entry (`Session::{copy_preset,swap_presets}`).
fn copy_swap_preset_menu(ui: &mut egui::Ui, session: &mut Session, team_id: u32, preset: usize) {
    menu_button(ui, "Copy/swap", |ui| {
        for to in (0..3).filter(|&to| to != preset) {
            if ui.button(format!("Copy to {}", to + 1)).clicked() {
                session.copy_preset(team_id, preset, to);
                ui.close();
            }
        }
        for to in (0..3).filter(|&to| to != preset) {
            if ui.button(format!("Swap with {}", to + 1)).clicked() {
                session.swap_presets(team_id, preset, to);
                ui.close();
            }
        }
    });
}

/// The formation bar's Copy/swap menu (the draft's `formationMenu`):
/// acting on the shown preset's selected formation, a copy to, then a
/// swap with, each of the other two in `FORMATIONS` order, each a sealed
/// undo entry (`Session::{copy_formation,swap_formations}`).
fn copy_swap_formation_menu(
    ui: &mut egui::Ui,
    session: &mut Session,
    team_id: u32,
    view: &tactics::TacticsView,
) {
    menu_button(ui, "Copy/swap", |ui| {
        for to in (0..3).filter(|&to| to != view.formation) {
            if ui.button(format!("Copy to {}", FORMATIONS[to])).clicked() {
                session.copy_formation(team_id, view.preset, view.formation, to);
                ui.close();
            }
        }
        for to in (0..3).filter(|&to| to != view.formation) {
            if ui.button(format!("Swap with {}", FORMATIONS[to])).clicked() {
                session.swap_formations(team_id, view.preset, view.formation, to);
                ui.close();
            }
        }
    });
}

/// The pitch badges' medals per roster slot (`slot_medals`) for a
/// `MedalKey`: the last answer with its key and the run in flight
/// (`worker::WorkerResult`'s poll-and-spawn cycle).
pub(super) type Medals = WorkerResult<MedalKey, Vec<Option<Medal>>>;

/// One medal run's worker: `key`'s team viewed on the caller's thread
/// (the `EditFile` lives there), then `slot_medals` on the shared ruleset
/// off it, the key riding the reply and a repaint asked for with it.
fn spawn_medals(
    ctx: &egui::Context,
    file: &EditFile,
    rules: &Arc<aatf::Rules>,
    key: MedalKey,
    slots: usize,
) -> Pending<MedalKey, Vec<Option<Medal>>> {
    let team = TeamView::new(file, key.team).ok();
    let (sender, replies) = crossbeam_channel::unbounded();
    let rules = Arc::clone(rules);
    let ctx = ctx.clone();
    let reply_key = key.clone();
    std::thread::spawn(move || {
        let marks = slot_medals(&rules, team.as_ref(), slots);
        // A newer run can replace the receiver mid-flight; that send's
        // failure is the superseded thread ending, not an error.
        if sender.send((reply_key, marks)).is_err() {
            log::debug!("the medal run's caller dropped a superseded run");
        }
        ctx.request_repaint();
    });
    Pending { key, replies }
}

/// Each of `team`'s `slots` roster slots' badge medal under `rules`
/// (the pitch markers' badge outlines): the tier `player_tier` reads the
/// slot's player as, mapped through `Medal::from_tier_id`; `None` for an
/// empty slot, no tier, a check that failed or a tier that is no medal
/// — and for every slot when the team cannot be viewed (`None`).
fn slot_medals(rules: &aatf::Rules, team: Option<&TeamView>, slots: usize) -> Vec<Option<Medal>> {
    let Some(team) = team else {
        return vec![None; slots];
    };
    (0..slots)
        .map(|slot| {
            u8::try_from(slot)
                .ok()
                .and_then(|slot| team.player(slot))
                .and_then(|player| rules.player_tier(team, player).ok().flatten())
                .and_then(|tier| Medal::from_tier_id(&tier.id))
        })
        .collect()
}
