//! The Tactics tab's pitch side: the draft's `pitchCanvas` and
//! `slotPopover` — the formation canvas, a marker per starter dragged to
//! his spot, dropped on to swap a bench player in, clicked for the slot's
//! details — plus `lineupList` and `benchList`, the starting eleven and
//! the bench, reordered and swapped by drag. The canvas is painted as PES
//! 2021's own formation screen: the game texture's turf bands, the
//! screen's slot-to-spot mapping and markers of portrait, badge and
//! coloured name.

use pes_savefile::interchange::team_toml::labels::POSITIONS;
use pes_savefile::model::tactics::{Formation, FormationSlot, TeamTactics};
use pes_version::PesVersion;
use studio_core::theme::{self, Palette};

use crate::names::append_name;
use crate::tactics::{
    Assign, FieldedMark, OpenSlot, RosterRow, TacticsView, bar_selector_spacing, name_at,
    raw_name_at,
};

/// The position a marker released on the grass takes.
mod drop_rule;
/// The "Starting eleven" and "Bench" lists, their rows and `swap_in`.
mod lineup;
/// The areas a held starter lights, and the darkening of the rest.
mod lit_area;
/// The slot popover a marker click opens beside its marker.
mod popover;

pub use lineup::lineup;
use lineup::swap_in;
use popover::slot_popover;

/// The formations' names, in the record's order (the draft's
/// `FORMATIONS`).
pub const FORMATIONS: [&str; 3] = ["Kick-off", "In-possession", "Out-of-possession"];

/// The playable positions grid's rows, the order a member reads a
/// formation in (the draft's `POS_ROWS`): the forwards left wing to right
/// wing, the attacking and wide midfielders, the central ones, the
/// defenders, GK alone below. The same rows, read back to front without
/// GK, are the formation name's lines (`formation_name`), so the grid and
/// the name cannot disagree.
pub const POSITION_ROWS: [&[&str]; 5] = [
    &["LWF", "CF", "SS", "RWF"],
    &["LMF", "AMF", "RMF"],
    &["CMF", "DMF"],
    &["LB", "CB", "RB"],
    &["GK"],
];

/// The payload a lineup row's drag carries and the pitch's markers drop
/// (the canvas lands in this module with it): `Starter` and `Marker`
/// index `TeamTactics::starting_eleven`, `Bench` `TeamTactics::bench_order`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineupDrag {
    /// A starting-eleven row.
    Starter(usize),
    /// A bench row.
    Bench(usize),
    /// A pitch marker: a lineup row treats it as its `Starter` index;
    /// another marker swaps their two slots in the shown formation, and
    /// the grass move (a row's release there never fires) is its alone.
    Marker(usize),
}

/// The canvas' size: the screen's own pitch drawn at its 720p capture's
/// scale (the widget allocates `PITCH_BOX`; the tests read this one).
#[cfg(test)]
pub(crate) const PITCH: egui::Vec2 = egui::vec2(320.0, 384.0);
/// The widget's whole extent: `PITCH` plus the draft `.pitch`'s 1 px
/// border a side. The settings and assignments panels beside it are as
/// tall.
pub const PITCH_BOX: egui::Vec2 = egui::vec2(322.0, 386.0);
/// The grid's extent in stored units.
const GRID_X: u8 = 104;
const GRID_Y: u8 = 48;

/// The badge centre's x for stored x 0 (fitted on the screen's capture).
const SPOT_X0: f32 = 12.8;
/// The badge centre's x pitch per stored x unit (fitted on the screen's
/// capture).
const SPOT_XS: f32 = 2.827;
/// The badge centre's y for stored y 0 (fitted on the screen's capture).
const SPOT_Y0: f32 = 363.8;
/// The badge centre's y pitch per stored y unit (fitted on the screen's
/// capture).
const SPOT_YS: f32 = 6.793;
/// The keeper's badge centre (the screen's keeper spot, on the goal
/// area's line), whatever his stored x and y.
const KEEPER_X: f32 = 160.0;
const KEEPER_Y: f32 = 358.5;

/// The badge and the portrait box's width.
const BADGE_W: f32 = 50.0;
/// The badge's height.
const BADGE_H: f32 = 14.0;
/// The badge texts' inset from the badge's ends: the game's ink is 2
/// px in from each end on the 720p capture.
const BADGE_TEXT_INSET: f32 = 2.0;
/// The badge texts' size: the widest pair, `CMF 109`, leaves about 2
/// between them in the 50-point badge at 11.5.
const BADGE_FONT: f32 = 11.5;
/// The portrait box's height.
const PORTRAIT_H: f32 = 50.0;
/// The marker box's reach above the badge centre: the portrait's 50 plus
/// the badge's 7 half.
const ABOVE_CENTRE: f32 = 57.0;
/// The marker box's reach below the badge centre: the name's bottom edge.
const BELOW_CENTRE: f32 = 20.0;
/// The name's centre under the badge centre's.
const NAME_DY: f32 = 13.0;

/// Live AATF's pitch cross: its ends' inset from the portrait's edges,
/// its coloured strokes' width and the black outline's width beside
/// them (the draft's `PITCH_CROSS`).
const CROSS_INSET: f32 = 6.0;
const CROSS_WIDTH: f32 = 5.0;
const CROSS_OUTLINE: f32 = 1.0;

/// The turf texture's size: the game texture's own, drawn into the
/// smaller canvas.
const TURF_SIZE: [usize; 2] = [480, 576];
/// The turf's band count: dark at both ends.
const TURF_BANDS: usize = 11;

/// A wide name's scroll period in seconds (the game's measured 13.5).
const NAME_PERIOD: f32 = 13.5;
/// The scroll's speed, canvas px a second (the game's 40).
const NAME_SPEED: f32 = 40.0;
/// The gap between a scrolling name's end and its repeat, in px (the
/// game's 16).
const NAME_GAP: f32 = 16.0;

/// The formation name's size (the draft's `FORMATION_LABEL` font): the
/// face's digits 15 points tall at it, as the game's label's are.
const LABEL_SIZE: f32 = 20.5;
/// The name's text origin, in from the canvas' left edge: the face's "3"
/// carries a 0.6-point left bearing, so its ink starts at the game's 7.
const LABEL_LEFT: f32 = 6.5;
/// The name's baseline, up from the canvas' bottom edge (its digits sit
/// on it).
const LABEL_BOTTOM: f32 = 6.0;

/// A marker as a frame's pass sees it: its index in the shown formation,
/// the slot and its (clamped) badge centre.
#[derive(Clone, Copy)]
struct Spot {
    /// The slot's index in the formation's eleven.
    index: usize,
    /// The formation slot.
    slot: FormationSlot,
    /// The badge centre the marker draws and interacts at (the held
    /// one's is his spot at this step of the drag instead).
    centre: egui::Pos2,
}

/// A held marker's drag, kept in the context's temp memory under the
/// pitch's scope from its first dragged frame to its release.
#[derive(Clone, Copy)]
struct Held {
    /// The dragged slot's index in the shown formation.
    slot: usize,
    /// The press's offset from his badge centre, kept for the whole
    /// drag: he stands at the pointer less it, so the point of his
    /// marker first pressed stays under the pointer and a press on his
    /// portrait or name does not jump his badge onto the pointer.
    offset: egui::Vec2,
}

/// The formation selector of the formation bar (the draft's
/// `formationSel`, moved out of `pitch` for the bar over the pitch's and
/// the assignments' columns): the shown preset's three formations of
/// `tactics`. While that preset fields its Kick-off formation alone, the
/// three are disabled with Kick-off selected (`lock_to_fielded`). Writes
/// `view.formation`, never the record.
pub fn formation_selector(ui: &mut egui::Ui, tactics: &TeamTactics, view: &mut TacticsView) {
    let locked = lock_to_fielded(tactics, view);
    ui.add_enabled_ui(!locked, |ui| {
        ui.horizontal(|ui| {
            bar_selector_spacing(ui);
            for (index, label) in FORMATIONS.iter().enumerate() {
                ui.selectable_value(&mut view.formation, index, *label);
            }
        })
        .response
        .on_hover_text(
            "The game uses formations 2 and 3 only while the preset's fluid formation is on",
        )
        .on_disabled_hover_text("Formations 2 and 3 are used only while Fluid formation is on");
    });
}

/// Puts `view` on Kick-off while the shown preset of `tactics` fields its
/// Kick-off formation alone (its Fluid formation off, always so on PES
/// 2015, which has no switch): the game uses the other two only while it
/// is on, so an edit there would change nothing a match uses. Answers
/// whether the preset is so locked. `formation_selector` and `pitch` both
/// call it first, so the pitch never shows a formation the selector
/// cannot pick, whichever of the two a host draws first.
fn lock_to_fielded(tactics: &TeamTactics, view: &mut TacticsView) -> bool {
    let locked = tactics.presets[view.preset.min(2)].fielded_formations() == [0];
    if locked {
        view.formation = 0;
    }
    locked
}

/// The draft's `.pitch`: the canvas inside its 1 px border — a marker
/// per starter of the shown formation, dragged to his spot, dropped on
/// to swap, clicked for the slot popover — the formation's name in its
/// bottom-left corner, and the open popover beside its marker.
/// `formation_selector` heads it in the host's bar.
///
/// A marker carries `LineupDrag::Marker`, so a release over a lineup row
/// is that row's drop (a bench row swaps him out through `swap_in`, an
/// eleven row exchanges the two in the starting eleven); a marker is
/// itself a drop target — a `Bench` row on it is `swap_in`, a `Starter`
/// row exchanges the two in the starting eleven (every formation follows,
/// the game's select-one-then-the-other), another `Marker` exchanges the
/// two slots' position and spot in the shown formation alone (the game's
/// drag) — and a release on the grass writes its slot's snapped
/// coordinates and the position the game's drop rule gives there, where
/// a row's release does nothing. A held marker moves a whole stored unit
/// at a time with the point first pressed under the pointer, his badge
/// showing the position the rule gives him at each step from his stored
/// one, and the pitch
/// outside the areas he is rated at is darkened; nothing is written
/// until the release. The keeper's marker
/// (position 0) is neither a drag source nor a marker's swap target; his
/// spot is fixed. `version` is the open save's (`swap_in`'s instruction
/// decode). Answers `true` only when `tactics` changed this frame.
pub fn pitch(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) -> bool {
    ui.push_id(id_salt, |ui| {
        lock_to_fielded(tactics, view);
        // The answer is the deep compare, as `tactics_settings`' is.
        let before = tactics.clone();
        // The popover's slot is of the scope, preset and formation it
        // opened on; a change of any of them closes it.
        if view.popover.is_some_and(|open| {
            open.scope != ui.scope_id()
                || open.preset != view.preset
                || open.formation != view.formation
        }) {
            view.popover = None;
        }
        // The name scroll's clock: the time the pitch was first shown,
        // in the context's temp memory with the frame it was last drawn
        // on; a frame without it restarts it, and a frame can run the
        // pitch more than once (egui's discard pass), so the same frame
        // number keeps the clock.
        let clock_id = ui.scope_id().with("name clock");
        let now = ui.input(|i| i.time);
        let frame_nr = ui.ctx().cumulative_frame_nr();
        let started = ui
            .ctx()
            .data_mut(|data| match data.get_temp::<(f64, u64)>(clock_id) {
                Some((started, last)) if frame_nr <= last + 1 => {
                    data.insert_temp(clock_id, (started, frame_nr));
                    started
                }
                _ => {
                    data.insert_temp(clock_id, (now, frame_nr));
                    now
                }
            });
        let seconds = (now - started) as f32;
        // The draft's `.pitch`: the canvas and its 1 px border a side
        // (`PITCH_BOX` is the widget's extent, `canvas` inside it). The
        // canvas' response is an `interact` on the already-allocated
        // rect, not a second allocation.
        let (area, _) = ui.allocate_exact_size(PITCH_BOX, egui::Sense::hover());
        let canvas = area.shrink(1.0);
        let canvas_response = ui.interact(
            canvas,
            ui.scope_id().with("pitch canvas"),
            egui::Sense::hover(),
        );
        canvas_response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Unknown, ui.is_enabled(), "pitch")
        });
        ui.painter().rect_stroke(
            area,
            0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let painter = ui.painter_at(canvas);
        let palette = theme::palette(ui.visuals());
        paint_pitch(&painter, canvas, palette);
        let players = tactics.presets[view.preset.min(2)].formations[view.formation.min(2)].players;
        let centres = players.map(|slot| clamp_centre(badge_centre(canvas.min, slot), canvas));
        let order = draw_order(&centres);
        // Interact in draw order first: egui gives an overlap to the
        // widget registered last, so the lowest marker (painted on top)
        // wins a press.
        let mut responses: [Option<egui::Response>; 11] = std::array::from_fn(|_| None);
        for index in order {
            let spot = Spot {
                index,
                slot: players[index],
                centre: centres[index],
            };
            responses[index] = Some(marker(ui, canvas, version, tactics, view, roster, spot));
        }
        // The held marker: the outfield one egui reports dragged. The
        // keeper's is skipped: his slot is fixed, he is painted at his
        // spot, never moved.
        let dragged = order.iter().copied().find(|&index| {
            players[index].position != 0
                && responses[index]
                    .as_ref()
                    .expect("every marker interacted")
                    .dragged()
        });
        // The drag as the last frame left it, and as this frame holds it:
        // his index and his slot at this step (the spot under the
        // pointer less the press's offset, and the position the rule
        // gives there from his stored one). Nothing is written until the
        // release.
        let held_id = ui.scope_id().with("held marker");
        let previous = ui.ctx().data(|data| data.get_temp::<Held>(held_id));
        let held = dragged.map(|index| {
            let held = previous
                .filter(|held| held.slot == index)
                .unwrap_or_else(|| Held {
                    slot: index,
                    // From his unclamped badge centre, the one `snap`
                    // inverts: a press with no move then keeps his spot
                    // even where the clamp has pushed his marker in from
                    // the canvas' edge.
                    offset: ui
                        .input(|i| i.pointer.press_origin())
                        .map_or(egui::Vec2::ZERO, |origin| {
                            origin - badge_centre(canvas.min, players[index])
                        }),
                });
            let (x, y) = ui
                .ctx()
                .pointer_interact_pos()
                .map_or((players[index].x, players[index].y), |pos| {
                    snap(pos - held.offset, canvas.min)
                });
            let formation = &tactics.presets[view.preset.min(2)].formations[view.formation.min(2)];
            let position = drop_rule::dropped_position(
                formation,
                index,
                x,
                y,
                overalls_of(roster, tactics, index),
            );
            ui.ctx().data_mut(|data| data.insert_temp(held_id, held));
            (index, FormationSlot { position, x, y })
        });
        // A drop hits the topmost marker first (the draft's `hitMarker`
        // checks back to front); the canvas' own is the grass. egui
        // reports a release to the closest widget within its 5-point
        // search radius, not only the one under the pointer, so a marker
        // is the target only when the release is inside his box.
        let release = ui.ctx().pointer_interact_pos();
        let mut dropped = None;
        for index in order.iter().rev() {
            let under =
                release.is_some_and(|pos| marker_rect(centres[*index], canvas).contains(pos));
            if let Some(payload) = responses[*index]
                .as_ref()
                .filter(|_| under)
                .and_then(|response| response.dnd_release_payload::<LineupDrag>())
            {
                dropped = Some((Some(*index), *payload));
                break;
            }
        }
        if dropped.is_none() {
            dropped = canvas_response
                .dnd_release_payload::<LineupDrag>()
                .map(|payload| (None, *payload));
        }
        match dropped {
            Some((Some(onto), LineupDrag::Bench(from))) => swap_in(version, tactics, onto, from),
            Some((Some(onto), LineupDrag::Starter(from))) if onto != from => {
                tactics.starting_eleven.swap(from, onto);
            }
            // A dragged marker's release reads `swap_target` — the same
            // pick the swap mark shows: over another outfield starter
            // the two slots exchange position and spot in the shown
            // formation alone, each player staying in his slot, as the
            // game's drag does (the starting-eleven exchange, which
            // reaches every formation, is the lineup rows'); over the
            // grass, his own spot or the keeper he moves to the spot the
            // held marker stood at, the release point less the press's
            // offset, taking the position the rule gives there, as his
            // badge showed it. A release egui also reports as his click (the
            // pointer moved within the click distance, though past his
            // edge) is that click, which `marker` served: no drop.
            Some((_, LineupDrag::Marker(from)))
                if !responses[from]
                    .as_ref()
                    .is_some_and(egui::Response::clicked) =>
            {
                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                    let overalls = overalls_of(roster, tactics, from);
                    let formation =
                        &mut tactics.presets[view.preset.min(2)].formations[view.formation.min(2)];
                    match swap_target(&players, &centres, canvas, pos, from) {
                        Some(onto) => formation.players.swap(from, onto),
                        None => {
                            let offset = previous
                                .filter(|held| held.slot == from)
                                .map_or(egui::Vec2::ZERO, |held| held.offset);
                            let (x, y) = snap(pos - offset, canvas.min);
                            formation.players[from].position =
                                drop_rule::dropped_position(formation, from, x, y, overalls);
                            formation.players[from].x = x;
                            formation.players[from].y = y;
                        }
                    }
                }
            }
            // A row released on the grass or on his own marker, and a
            // marker's release that was his click, do nothing.
            Some(_) | None => {}
        }
        // The drag's state ends once no marker is held: at the release,
        // whose drop has read it above, and on every idle frame.
        if held.is_none() {
            ui.ctx().data_mut(|data| data.remove::<Held>(held_id));
        }
        // The paint pass reads the written record: the spots are
        // recomputed after the drop is written, so a released marker
        // paints at his drop point on this very frame. Painted top of the
        // canvas down (a lower marker covers the one above it), the held
        // one last.
        let players = tactics.presets[view.preset.min(2)].formations[view.formation.min(2)].players;
        let centres = players.map(|slot| clamp_centre(badge_centre(canvas.min, slot), canvas));
        let order = draw_order(&centres);
        // While an outfield starter is held, the pitch outside the areas
        // he is rated at is darkened, under every marker.
        if let Some(index) = dragged {
            let rated = roster
                .get(usize::from(tactics.starting_eleven[index]))
                .map_or([false; 13], |row| row.rated);
            lit_area::paint_dim(&painter, canvas, &rated, palette);
        }
        // The starter a release now would swap with — the same pick the
        // drop makes, so the mark and the release always agree. The
        // dragged marker sits on him, so it goes half transparent over
        // him and his corners are painted over both (the draft's
        // `drawPitch` target mark).
        let swap = dragged.and_then(|index| {
            ui.ctx()
                .pointer_interact_pos()
                .and_then(|pos| swap_target(&players, &centres, canvas, pos, index))
        });
        let mut still_for: Option<f32> = None;
        let mut record = |until: Option<f32>| {
            if let Some(until) = until {
                still_for = Some(still_for.map_or(until, |min: f32| min.min(until)));
            }
        };
        for index in order {
            if Some(index) != dragged {
                record(paint_marker(
                    &painter,
                    palette,
                    tactics,
                    view,
                    roster,
                    Spot {
                        index,
                        slot: players[index],
                        centre: centres[index],
                    },
                    seconds,
                ));
            }
        }
        // The held marker's drawn place, his spot at this step (clamped
        // like every marker's): the cross pass reads it too.
        let dragged_at = held.map(|(_, slot)| clamp_centre(badge_centre(canvas.min, slot), canvas));
        if let Some((index, slot)) = held {
            // He goes half transparent while he sits on a swap target.
            let mut painter = painter.clone();
            if swap.is_some() {
                painter.multiply_opacity(0.35);
            }
            // His badge shows the position he holds, or, over a
            // teammate, the teammate's, which a release there gives him,
            // with his own overall at it.
            let shown = FormationSlot {
                position: swap.map_or(slot.position, |target| players[target].position),
                ..slot
            };
            record(paint_marker(
                &painter,
                palette,
                tactics,
                view,
                roster,
                Spot {
                    index,
                    slot: shown,
                    centre: dragged_at.expect("the held marker's centre"),
                },
                seconds,
            ));
        }
        paint_formation_name(
            &painter,
            canvas,
            &tactics.presets[view.preset.min(2)].formations[view.formation.min(2)],
            palette,
        );
        // The swap mark over everything, as the screen's selection marks
        // a target.
        if let Some(target) = swap {
            paint_corners(
                &painter,
                portrait_rect(badge_rect(centres[target])),
                palette.selected_corners,
            );
        }
        // Live AATF's `FieldedNotA` crosses, painted after every marker
        // and the swap mark: markers overlap, and a mark that asks for a
        // fix must not hide under a later marker's portrait. Each is an X
        // over the starter's portrait (`paint_portrait_cross`) in the
        // marks' highest severity's colour, its tooltip their lines over
        // the portrait. The shown formation is always one the preset
        // fields: `lock_to_fielded` keeps a non-fluid preset on Kick-off.
        for index in order {
            let marks: Vec<&FieldedMark> = roster
                .get(usize::from(tactics.starting_eleven[index]))
                .map(|row| {
                    row.fielded
                        .iter()
                        .filter(|mark| {
                            mark.preset == view.preset.min(2)
                                && mark.position == players[index].position
                        })
                        .collect()
                })
                .unwrap_or_default();
            if marks.is_empty() {
                continue;
            }
            let centre = if Some(index) == dragged {
                dragged_at.expect("the dragged marker's centre")
            } else {
                centres[index]
            };
            let portrait = portrait_rect(badge_rect(centre));
            let color = if marks.iter().any(|mark| mark.error) {
                ui.visuals().error_fg_color
            } else {
                ui.visuals().warn_fg_color
            };
            paint_portrait_cross(&painter, portrait, color, palette);
            ui.interact(
                portrait,
                ui.scope_id().with(("live aatf pitch cross", index)),
                egui::Sense::hover(),
            )
            .on_hover_text(
                marks
                    .iter()
                    .map(|mark| mark.line.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        // The next name move asks the repaint (none when every name fits).
        if let Some(still_for) = still_for {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f32(still_for));
        }
        slot_popover(ui, canvas, tactics, view, roster);
        *tactics != before
    })
    .inner
}

/// The per-position overalls on the roster row of the starter in
/// `index` of the eleven, which the drop rule picks his position by;
/// `None` for an empty slot or a host that reads none.
fn overalls_of<'a>(
    roster: &'a [RosterRow<'_>],
    tactics: &TeamTactics,
    index: usize,
) -> Option<&'a [u8; 13]> {
    roster
        .get(usize::from(tactics.starting_eleven[index]))
        .and_then(|row| row.overalls.as_ref())
}

/// A slot's badge centre on the canvas (the screen's own spot mapping,
/// the draft's `badgeAt` before its clamp): an outfield slot at
/// (`SPOT_X0` + `SPOT_XS` x, `SPOT_Y0` - `SPOT_YS` y), high y (the attack
/// end) at the top; the keeper at (`KEEPER_X`, `KEEPER_Y`), where the
/// screen draws him. `origin` is the canvas' top-left.
fn badge_centre(origin: egui::Pos2, slot: FormationSlot) -> egui::Pos2 {
    if slot.position == 0 {
        return origin + egui::vec2(KEEPER_X, KEEPER_Y);
    }
    origin
        + egui::vec2(
            SPOT_X0 + SPOT_XS * f32::from(slot.x),
            SPOT_Y0 - SPOT_YS * f32::from(slot.y),
        )
}

/// `pos` inside the canvas (from `origin`) snapped back through the
/// mapping to the stored grid (the draft's `snapXY`): the nearest unit,
/// clamped to the grid.
fn snap(pos: egui::Pos2, origin: egui::Pos2) -> (u8, u8) {
    let local = pos - origin;
    let x = ((local.x - SPOT_X0) / SPOT_XS)
        .round()
        .clamp(0.0, f32::from(GRID_X)) as u8;
    let y = ((SPOT_Y0 - local.y) / SPOT_YS)
        .round()
        .clamp(0.0, f32::from(GRID_Y)) as u8;
    (x, y)
}

/// `centre` clamped so the marker's whole box stays inside the canvas
/// (the draft's `clampBadge`).
fn clamp_centre(centre: egui::Pos2, canvas: egui::Rect) -> egui::Pos2 {
    egui::pos2(
        centre.x.clamp(
            canvas.left() + BADGE_W / 2.0,
            canvas.right() - BADGE_W / 2.0,
        ),
        centre
            .y
            .clamp(canvas.top() + ABOVE_CENTRE, canvas.bottom() - BELOW_CENTRE),
    )
}

/// The marker's whole box from the badge centre — the badge's width, the
/// portrait's top to the name's bottom — the centre clamped so the box
/// stays inside the canvas (the draft's `markerRect`).
fn marker_rect(centre: egui::Pos2, canvas: egui::Rect) -> egui::Rect {
    let centre = clamp_centre(centre, canvas);
    egui::Rect::from_min_max(
        centre - egui::vec2(BADGE_W / 2.0, ABOVE_CENTRE),
        centre + egui::vec2(BADGE_W / 2.0, BELOW_CENTRE),
    )
}

/// The order the screen draws markers in (the draft's `drawOrder`): top
/// of the canvas first, so a lower marker covers the one above it; ties
/// keep the slot order.
fn draw_order(centres: &[egui::Pos2; 11]) -> [usize; 11] {
    let mut order: [usize; 11] = std::array::from_fn(|index| index);
    order.sort_by(|a, b| centres[*a].y.total_cmp(&centres[*b].y).then(a.cmp(b)));
    order
}

/// The starter a release at `pos` would swap with (the draft's
/// `dropTarget`): the marker drawn on top among the others — `skip` is
/// the dragged one's own spot, never a target — unless it is the
/// keeper's (position 0), whose slot's position and spot are fixed, so
/// over him there is none. The swap mark and the drop both read it, so
/// they always agree.
fn swap_target(
    players: &[FormationSlot; 11],
    centres: &[egui::Pos2; 11],
    canvas: egui::Rect,
    pos: egui::Pos2,
    skip: usize,
) -> Option<usize> {
    draw_order(centres)
        .into_iter()
        .rev()
        .find(|&index| index != skip && marker_rect(centres[index], canvas).contains(pos))
        .filter(|&index| players[index].position != 0)
}

/// The badge's rect around its centre (50 x 14).
fn badge_rect(centre: egui::Pos2) -> egui::Rect {
    egui::Rect::from_center_size(centre, egui::vec2(BADGE_W, BADGE_H))
}

/// The portrait box standing on `badge`.
fn portrait_rect(badge: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(badge.left(), badge.top() - PORTRAIT_H),
        egui::vec2(BADGE_W, PORTRAIT_H),
    )
}

/// Live AATF's pitch cross (the draft's `pitchCross`): an X over the
/// whole `portrait`, `CROSS_INSET` in from its edges, its two strokes
/// `CROSS_WIDTH` wide in `color` over the same strokes in black and
/// `CROSS_OUTLINE` wider each side, so it stays seen over a red portrait.
/// A disc at each stroke's end rounds it, as the draft's round caps do.
fn paint_portrait_cross(
    painter: &egui::Painter,
    portrait: egui::Rect,
    color: egui::Color32,
    palette: &Palette,
) {
    let r = portrait.width() / 2.0 - CROSS_INSET;
    let centre = portrait.center();
    let strokes = [
        [centre + egui::vec2(-r, -r), centre + egui::vec2(r, r)],
        [centre + egui::vec2(r, -r), centre + egui::vec2(-r, r)],
    ];
    for (layer_color, width) in [
        (palette.cross_outline, CROSS_WIDTH + 2.0 * CROSS_OUTLINE),
        (color, CROSS_WIDTH),
    ] {
        for points in strokes {
            painter.line_segment(points, egui::Stroke::new(width, layer_color));
            for end in points {
                painter.circle_filled(end, width / 2.0, layer_color);
            }
        }
    }
}

/// The screen's corner brackets around `rect` (the draft's `corners`):
/// a 9 px arm inward from each corner.
fn paint_corners(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(2.0, color);
    let arm = 9.0;
    let corner = |pos: egui::Pos2, dx: f32, dy: f32| {
        painter.line_segment([pos + egui::vec2(0.0, dy * arm), pos], stroke);
        painter.line_segment([pos, pos + egui::vec2(dx * arm, 0.0)], stroke);
    };
    corner(rect.left_top(), 1.0, 1.0);
    corner(rect.right_top(), -1.0, 1.0);
    corner(rect.left_bottom(), 1.0, -1.0);
    corner(rect.right_bottom(), -1.0, -1.0);
}

/// The formation's name in the canvas' bottom-left corner, as the game's
/// formation screen writes it (the draft's `FORMATION_LABEL`): white, in
/// the `label` family (the theme's Roboto Flex Semibold) at `LABEL_SIZE`, its
/// text origin `LABEL_LEFT` in and its baseline `LABEL_BOTTOM` up. egui
/// places a galley by its top-left corner and its first glyph's origin
/// at the row's offset plus the glyph's own (the baseline, a font ascent
/// down), so the galley goes that far above and left of the wanted
/// origin.
fn paint_formation_name(
    painter: &egui::Painter,
    canvas: egui::Rect,
    formation: &Formation,
    palette: &Palette,
) {
    let galley = painter.layout_no_wrap(
        formation_name(formation),
        egui::FontId::new(LABEL_SIZE, egui::FontFamily::Name("label".into())),
        palette.pitch_text,
    );
    let row = galley.rows.first().expect("a laid-out galley has a row");
    // An empty name (no outfield slot in a known position) draws nothing.
    let Some(glyph) = row.glyphs.first() else {
        return;
    };
    let origin = row.pos.to_vec2() + glyph.pos.to_vec2();
    let at = canvas.left_bottom() + egui::vec2(LABEL_LEFT, -LABEL_BOTTOM) - origin;
    painter.galley(at, galley, palette.pitch_text);
}

/// A name no wider than the badge fits: it is centred and never moves.
fn name_fits(width: f32) -> bool {
    width <= BADGE_W
}

/// The leftward offset of a name `width` wide `seconds` after the
/// scroll's clock started (the draft's `nameScroll`): it holds still
/// `NAME_PERIOD` less its `width + NAME_GAP` loop at `NAME_SPEED`, then
/// moves `NAME_SPEED` a second through that loop, ending where it began.
fn name_scroll(width: f32, seconds: f32) -> f32 {
    name_moving_for(width, seconds).max(0.0) * NAME_SPEED
}

/// How long until the name `width` wide next moves at `seconds`: 0 while
/// it moves.
fn name_still_for(width: f32, seconds: f32) -> f32 {
    (-name_moving_for(width, seconds)).max(0.0)
}

/// How long the name `width` wide has been moving at `seconds` in its
/// period, negative while it holds still (minus the time left before it
/// moves): the phase `name_scroll` and `name_still_for` both read.
fn name_moving_for(width: f32, seconds: f32) -> f32 {
    let moving = (width + NAME_GAP) / NAME_SPEED;
    seconds % NAME_PERIOD - (NAME_PERIOD - moving)
}

/// The position text's colour on the pitch's badges by the position's
/// group, as the game's (the draft's `POS_COLOUR` over `POSITIONS`'
/// order): a position past the table white, as `position_label` reads it
/// "?".
fn position_colour(palette: &Palette, position: u8) -> egui::Color32 {
    group_colour(
        [
            palette.keeper,
            palette.defender,
            palette.midfielder,
            palette.forward,
        ],
        palette.pitch_text,
        position,
    )
}

/// The position text's colour in the lineup lists, which sit on the
/// panel: the palette's lineup colour of the position's group, a position
/// past the table as on the pitch.
fn lineup_colour(palette: &Palette, position: u8) -> egui::Color32 {
    group_colour(
        [
            palette.lineup_keeper,
            palette.lineup_defender,
            palette.lineup_midfielder,
            palette.lineup_forward,
        ],
        palette.pitch_text,
        position,
    )
}

/// The colour of `position`'s group among the four given, in order the
/// keeper's, a defender's, a midfielder's and a forward's;
/// `past_the_table` for a position past the table.
fn group_colour(
    [keeper, defender, midfielder, forward]: [egui::Color32; 4],
    past_the_table: egui::Color32,
    position: u8,
) -> egui::Color32 {
    match position {
        0 => keeper,
        1..=3 => defender,
        4..=8 => midfielder,
        9..=12 => forward,
        _ => past_the_table,
    }
}

/// The turf image (the draft's `turf`): the game texture's size, eleven
/// horizontal bands dark at both ends, and the texture's grain as a
/// fixed-seed xorshift32 — deterministic, so the loaded texture is made
/// once per context.
fn turf_image(palette: &Palette) -> egui::ColorImage {
    let (width, height) = (TURF_SIZE[0], TURF_SIZE[1]);
    let mut pixels = Vec::with_capacity(width * height);
    // xorshift32 from a fixed seed: the same grain every run.
    let mut state = 0x9e37_79b9u32;
    for y in 0..height {
        let band = if (y * TURF_BANDS / height).is_multiple_of(2) {
            palette.turf_dark_band
        } else {
            palette.turf_light_band
        };
        let base = [band.r(), band.g(), band.b()];
        for _x in 0..width {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let noise = ((state as f32 / u32::MAX as f32 - 0.5) * 12.1).round() as i32;
            let channel = |base: u8| (i32::from(base) + noise).clamp(0, 255) as u8;
            pixels.push(egui::Color32::from_rgb(
                channel(base[0]),
                channel(base[1]),
                channel(base[2]),
            ));
        }
    }
    egui::ColorImage::new(TURF_SIZE, pixels)
}

/// The turf texture, loaded once per context under a fixed id. One texture
/// serves both themes: the turf's bands are a surface both themes share.
fn turf(ctx: &egui::Context, palette: &Palette) -> egui::TextureHandle {
    let id = egui::Id::unique("pitch turf");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let texture = ctx.load_texture(
        "pitch turf",
        turf_image(palette),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

/// The turf and the lines (the draft's `drawLines`): the halfway line,
/// the centre circle, and per end the penalty box, the goal area and the
/// penalty arc — no touchlines and no goal lines (the texture has none).
fn paint_pitch(painter: &egui::Painter, canvas: egui::Rect, palette: &Palette) {
    let stroke = egui::Stroke::new(2.0, palette.pitch_lines);
    painter.image(
        turf(painter.ctx(), palette).id(),
        canvas,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        theme::NO_TINT,
    );
    let at = |x: f32, y: f32| canvas.min + egui::vec2(x, y);
    painter.line_segment([at(0.0, 192.0), at(320.0, 192.0)], stroke);
    painter.circle_stroke(at(160.0, 192.0), 36.0, stroke);
    for top in [true, false] {
        // The end's goal line and the direction into the field.
        let (edge, dir) = if top { (0.0, 1.0) } else { (384.0, -1.0) };
        let line = |x: f32, depth: f32| at(x, edge + dir * depth);
        // The penalty box, 160 wide centred and 66 deep: its two sides
        // and its far edge (no goal line).
        for x in [80.0, 240.0] {
            painter.line_segment([line(x, 0.0), line(x, 66.0)], stroke);
        }
        painter.line_segment([line(80.0, 66.0), line(240.0, 66.0)], stroke);
        // The goal area, x 123 to 197 and 22 deep.
        for x in [123.0, 197.0] {
            painter.line_segment([line(x, 0.0), line(x, 22.0)], stroke);
        }
        painter.line_segment([line(123.0, 22.0), line(197.0, 22.0)], stroke);
        // The penalty arc: the centre circle's radius about the spot 44
        // from the goal line, only its part beyond the box's far edge
        // (where a point is more than 22 deep).
        let spot = line(160.0, 44.0);
        let half = (22.0f32 / 36.0).acos();
        let mid = if top {
            std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2
        };
        let points: Vec<egui::Pos2> = (0..17)
            .map(|i| {
                let angle = mid - half + 2.0 * half * i as f32 / 16.0;
                spot + egui::vec2(36.0 * angle.cos(), 36.0 * angle.sin())
            })
            .collect();
        painter.line(points, stroke);
    }
}

/// One marker (the draft's chip grown to the screen's marker): the
/// slot's position over the starter's name, its AccessKit node a button
/// labeled with both. The keeper's slot (position 0) senses like the
/// others but sets no drag payload, so his marker never moves — and his
/// release is his click however far the pointer moved (the draft's
/// `d.gk`). A click serves the armed mode when one is armed, else opens
/// the slot's popover. The answer is the marker's
/// response, for the caller's drop check; `paint_marker` draws it.
fn marker(
    ui: &mut egui::Ui,
    canvas: egui::Rect,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
    spot: Spot,
) -> egui::Response {
    let keeper = spot.slot.position == 0;
    // The keeper senses drag like the others although he never moves:
    // egui's hit test answers a drag with the topmost drag-sensing
    // widget, so click-only would send a press on him to the marker
    // underneath.
    let response = ui.interact(
        marker_rect(spot.centre, canvas),
        ui.scope_id().with(("pitch marker", spot.index)),
        egui::Sense::click_and_drag(),
    );
    let name = name_at(roster, tactics.starting_eleven[spot.index]);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            format!("{} {}", position_label(spot.slot.position), name),
        )
    });
    if !keeper {
        response.dnd_set_drag_payload(LineupDrag::Marker(spot.index));
    }
    // The keeper's release is his click however far the pointer moved:
    // his marker never moves (the draft's `d.gk` never sets `moved`), so
    // a dragged press on him is no drag.
    if response.clicked() || (keeper && response.drag_stopped()) {
        match view.assign {
            Some(assign) => assign.write(version, tactics, view, spot.index),
            None => {
                view.popover = Some(OpenSlot {
                    scope: ui.scope_id(),
                    preset: view.preset,
                    formation: view.formation,
                    slot: spot.index,
                    at: marker_rect(spot.centre, canvas).left_bottom() - canvas.min,
                });
            }
        }
    }
    response
}

/// One marker painted (the draft's `drawPitch` body): the portrait, the
/// badge with the position's abbreviation in its group's colour, the
/// starter's overall at the slot's position at its right end in white
/// and his medal's outline when he has a tier, the name in its colour
/// codes' colours (white where it has none), the captain's mark when he
/// is the starter, and the selection corners the screen's marks lend
/// the editor's states. `painter` is clipped to the canvas, which a wide
/// name's cut keeps to. `spot.centre` is the clamped badge
/// centre (a held marker's at his spot at this step, `spot.slot` the
/// position his badge shows); `seconds` is the name
/// scroll's clock. Answers how long until the name next moves, `None`
/// when it fits and never moves.
fn paint_marker(
    painter: &egui::Painter,
    palette: &Palette,
    tactics: &TeamTactics,
    view: &TacticsView,
    roster: &[RosterRow<'_>],
    spot: Spot,
    seconds: f32,
) -> Option<f32> {
    let roster_slot = tactics.starting_eleven[spot.index];
    let badge = badge_rect(spot.centre);
    let portrait = portrait_rect(badge);
    if let Some(texture) = roster
        .get(usize::from(roster_slot))
        .and_then(|row| row.portrait)
    {
        painter.image(
            texture,
            portrait,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            theme::NO_TINT,
        );
    }
    let fill = if view.assign == Some(Assign::Join)
        && tactics.players_to_join_attack.contains(&roster_slot)
    {
        palette.join_fill
    } else {
        palette.badge_fill
    };
    painter.rect_filled(badge, egui::CornerRadius::same(2), fill);
    // His medal tier outlines the badge in its colour, inside the
    // badge's edge — an outline, not a fill, since the position
    // colours read at a contrast of 1.0 to 2.4 on the metals. A slot
    // with no medal draws none.
    if let Some(medal) = roster
        .get(usize::from(roster_slot))
        .and_then(|row| row.medal)
    {
        painter.rect_stroke(
            badge,
            egui::CornerRadius::same(2),
            egui::Stroke::new(1.5, medal.colour(palette)),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        egui::pos2(badge.left() + BADGE_TEXT_INSET, badge.center().y),
        egui::Align2::LEFT_CENTER,
        position_label(spot.slot.position),
        egui::FontId::proportional(BADGE_FONT),
        position_colour(palette, spot.slot.position),
    );
    // His overall at the slot's position at the badge's right end,
    // where the game draws it, white as the game's.
    if let Some(overall) = roster
        .get(usize::from(roster_slot))
        .and_then(|row| row.overalls)
        .and_then(|overalls| overalls.get(usize::from(spot.slot.position)).copied())
    {
        painter.text(
            egui::pos2(badge.right() - BADGE_TEXT_INSET, badge.center().y),
            egui::Align2::RIGHT_CENTER,
            overall.to_string(),
            egui::FontId::proportional(BADGE_FONT),
            palette.pitch_text,
        );
    }
    // The name in its colour codes' colours (white where it has none),
    // centred under the badge or cut at its right when wider.
    let mut job = egui::text::LayoutJob::default();
    append_name(
        &mut job,
        raw_name_at(roster, roster_slot),
        &egui::FontId::proportional(12.0),
    );
    for section in &mut job.sections {
        if section.format.color == egui::Color32::PLACEHOLDER {
            section.format.color = palette.pitch_text;
        }
    }
    let galley = painter.layout_job(job);
    let size = galley.size();
    let wide = !name_fits(size.x);
    // Scroll names off: the scroll's resting phase — the name still at
    // the badge's left, cut to its width — and no repaint to ask for.
    let scroll = if wide && view.scroll_names {
        name_scroll(size.x, seconds)
    } else {
        0.0
    };
    let left = if wide {
        badge.left() - scroll
    } else {
        spot.centre.x - size.x / 2.0
    };
    let clip = egui::Rect::from_x_y_ranges(badge.x_range(), painter.clip_rect().y_range());
    let clipped = painter.with_clip_rect(clip);
    let top = spot.centre.y + NAME_DY - size.y / 2.0;
    clipped.galley(egui::pos2(left, top), galley.clone(), palette.pitch_text);
    if wide {
        // Its repeat a loop to the right, so the start follows the end.
        clipped.galley(
            egui::pos2(left + size.x + NAME_GAP, top),
            galley,
            palette.pitch_text,
        );
    }
    if roster_slot == tactics.set_pieces.captain {
        // The white box's 1 px outline in the badge's colour, as the
        // game's: on a light portrait the box would have no edge.
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(badge.left() + 1.0, badge.top() - 17.0),
                egui::pos2(badge.left() + 15.0, badge.top() - 4.0),
            ),
            egui::CornerRadius::same(3),
            palette.badge_fill,
        );
        let mark = egui::Rect::from_min_max(
            egui::pos2(badge.left() + 2.0, badge.top() - 16.0),
            egui::pos2(badge.left() + 14.0, badge.top() - 5.0),
        );
        painter.rect_filled(mark, egui::CornerRadius::same(2), palette.captain_fill);
        painter.text(
            mark.center(),
            egui::Align2::CENTER_CENTER,
            "C",
            egui::FontId::proportional(9.0),
            palette.captain_text,
        );
    }
    let corners = if view.popover.is_some_and(|open| open.slot == spot.index) {
        Some(palette.selected_corners)
    } else if view.assign.is_some() {
        Some(palette.candidate_corners)
    } else {
        None
    };
    if let Some(color) = corners {
        paint_corners(painter, portrait, color);
    }
    (wide && view.scroll_names).then(|| name_still_for(size.x, seconds))
}

/// The formation's name as PES 2021's formation screen writes it in the
/// pitch's corner (`3-3-1-3`): the ten outfield slots counted by the row
/// of `POSITION_ROWS` their position belongs to, back to front, an empty
/// row left out. The keeper never counts, and the spot plays no part: an
/// AMF given CMF at his own spot turns `3-3-1-3` into `3-4-3`.
pub(crate) fn formation_name(formation: &Formation) -> String {
    // Every row but the last, GK's.
    POSITION_ROWS[..4]
        .iter()
        .rev()
        .map(|row| {
            formation
                .players
                .iter()
                .filter(|slot| row.contains(&position_label(slot.position)))
                .count()
        })
        .filter(|&count| count > 0)
        .map(|count| count.to_string())
        .collect::<Vec<_>>()
        .join("-")
}

/// The position byte's label (`POSITIONS`), "?" past the table as the
/// player list reads it.
pub(crate) fn position_label(position: u8) -> &'static str {
    POSITIONS.get(usize::from(position)).copied().unwrap_or("?")
}

#[cfg(test)]
mod tests;
