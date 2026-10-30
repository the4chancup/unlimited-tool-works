//! The "Player assignments" palette and the armed click-to-assign modes:
//! the six set-piece takers, the captain and the join-attack pick as
//! chips that arm a pitch click. No note explains an armed mode: the
//! highlighted chip and the candidates' corner brackets on the pitch say
//! it.

use pes_savefile::model::tactics::{SetPieceTakers, TeamTactics};
use pes_savefile::schema::fields::{InstructionSide, TacticsField};
use pes_savefile::schema::instruction;
use pes_version::PesVersion;

use super::{RosterRow, TacticsView, name_at};

/// The armed click-to-assign mode (the draft's `S.assign`): which field
/// the next pitch click writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Assign {
    /// One of `SET_PIECES`' roles (the six takers, then the captain): the
    /// click writes the clicked starter's roster slot and disarms.
    Role(usize),
    /// The players joining the attack: each click toggles the clicked
    /// starter's membership; armed until the chip's click disarms.
    Join,
    /// An instruction's target-player pick (the draft's `S.assign.kind
    /// === "target"`): the click writes the clicked starter's roster
    /// slot and disarms.
    Target {
        /// Which side's instruction list (`attack_instructions` or
        /// `defence_instructions`).
        side: InstructionSide,
        /// The instruction's index in it (0 or 1).
        index: usize,
    },
}

impl Assign {
    /// Whether the armed mode can take the starter in roster `slot`, so
    /// his marker gets the candidates' corners: join takes its members (a
    /// click removes them) and anyone else only while the list has room;
    /// a role and a target take every starter.
    pub(crate) fn takes(self, tactics: &TeamTactics, slot: u8) -> bool {
        match self {
            Assign::Join => {
                let list = &tactics.players_to_join_attack;
                list.contains(&slot) || list.contains(&u8::MAX)
            }
            Assign::Role(_) | Assign::Target { .. } => true,
        }
    }

    /// A click on starter `index` (a `starting_eleven` index) while armed
    /// (the draft's `assignClick`): a role writes the starter's roster
    /// slot — a same-player write is skipped — and disarms; join toggles
    /// the starter's membership and writes the list packed, the members
    /// first and 255 filling the end (a member leaving moves the later
    /// ones up, a non-member joins after the members, a fourth on a full
    /// list is ignored); join stays
    /// armed; a target writes the starter's roster slot and disarms, but
    /// only while that slot's instruction still takes a player (an
    /// instruction changed while armed writes nothing and disarms).
    /// `version` is the save's (the target's instruction decode).
    pub(crate) fn write(
        self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        index: usize,
    ) {
        let slot = tactics.starting_eleven[index];
        match self {
            Assign::Role(role) => {
                let role = &SET_PIECES[role];
                if (role.get)(&tactics.set_pieces) != slot {
                    (role.set)(&mut tactics.set_pieces, slot);
                }
                view.assign = None;
            }
            Assign::Join => {
                // Rebuilt from the members on every toggle, so a stored
                // list that already holds a gap is written packed too.
                let list = &mut tactics.players_to_join_attack;
                let mut members: Vec<u8> = list
                    .iter()
                    .copied()
                    .filter(|&entry| entry != u8::MAX)
                    .collect();
                if members.contains(&slot) {
                    members.retain(|&member| member != slot);
                } else if members.len() < list.len() {
                    members.push(slot);
                }
                members.resize(list.len(), u8::MAX);
                list.copy_from_slice(&members);
            }
            Assign::Target { side, index } => {
                let preset = &mut tactics.presets[view.preset.min(2)];
                let entries = match side {
                    InstructionSide::Attack => preset.attack_instructions.as_mut(),
                    InstructionSide::Defence => preset.defence_instructions.as_mut(),
                };
                if let Some(entry) = entries.and_then(|entries| entries.get_mut(index))
                    && entry.player != slot
                    && instruction::decode(version, entry.instruction)
                        .ok()
                        .flatten()
                        .is_some_and(|inst| inst.takes_player())
                {
                    entry.player = slot;
                }
                view.assign = None;
            }
        }
    }
}

/// The pitch marker whose popover is open, with what it opened on:
/// `pitch` keeps the popover only while `scope` (the widget's `egui::Id`,
/// which the host's pushed id keys to the team and the open), `preset`
/// and `formation` are still what they were at open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpenSlot {
    /// The pitch widget's `ui.scope_id()` at open.
    pub(crate) scope: egui::Id,
    /// `view.preset` at open.
    pub(crate) preset: usize,
    /// `view.formation` at open.
    pub(crate) formation: usize,
    /// The formation slot, 0 to 10.
    pub(crate) slot: usize,
    /// The marker's bottom-left corner as it stood at the click,
    /// relative to the canvas' min corner: the popover's top-right stays
    /// on it while the slot's X and Y move the marker, so the fields'
    /// arrows stay under the pointer — and it scrolls with the pitch
    /// (canvas-relative, never a screen point).
    pub(crate) at: egui::Vec2,
}

impl OpenSlot {
    /// The popover's `egui::Area` id, its layer's too: `slot_popover`
    /// shows it under this id.
    pub(crate) fn area_id(&self) -> egui::Id {
        self.scope.with("slot popover")
    }
}

/// One chip of the "Player assignments" group (the draft's `SET_PIECES`):
/// the role's label and the `SetPieceTakers` field it edits — the captain
/// is `set_pieces.captain` in this model.
pub(super) struct RoleSpec {
    /// The chip's label.
    pub(super) label: &'static str,
    /// The tactics field the chip writes (`tactics_term`'s key into the
    /// table, so a row's label is never a second copy).
    pub(super) field: TacticsField,
    /// The stored holder (a roster slot, 255 none).
    get: fn(&SetPieceTakers) -> u8,
    /// The stored write.
    set: fn(&mut SetPieceTakers, u8),
}

/// The six set-piece takers and the captain, in the draft's order.
pub(super) const SET_PIECES: [RoleSpec; 7] = [
    RoleSpec {
        label: "Long free kick",
        field: TacticsField::FreeKickTakerLong,
        get: |pieces| pieces.free_kick_long,
        set: |pieces, slot| pieces.free_kick_long = slot,
    },
    RoleSpec {
        label: "Short free kick",
        field: TacticsField::FreeKickTakerShort,
        get: |pieces| pieces.free_kick_short,
        set: |pieces, slot| pieces.free_kick_short = slot,
    },
    RoleSpec {
        label: "Second free kick",
        field: TacticsField::FreeKickTakerSecond,
        get: |pieces| pieces.free_kick_second,
        set: |pieces, slot| pieces.free_kick_second = slot,
    },
    RoleSpec {
        label: "Left corner",
        field: TacticsField::CornerTakerLeft,
        get: |pieces| pieces.corner_left,
        set: |pieces, slot| pieces.corner_left = slot,
    },
    RoleSpec {
        label: "Right corner",
        field: TacticsField::CornerTakerRight,
        get: |pieces| pieces.corner_right,
        set: |pieces, slot| pieces.corner_right = slot,
    },
    RoleSpec {
        label: "Penalty kick",
        field: TacticsField::PenaltyTaker,
        get: |pieces| pieces.penalty,
        set: |pieces, slot| pieces.penalty = slot,
    },
    RoleSpec {
        label: "Captain",
        field: TacticsField::Captain,
        get: |pieces| pieces.captain,
        set: |pieces, slot| pieces.captain = slot,
    },
];

/// The role chip's label for `field`, `None` for a field that is no role:
/// `tactics_term` reads the chips' own table, never a second copy of it.
pub(super) fn role_label(field: TacticsField) -> Option<&'static str> {
    SET_PIECES
        .iter()
        .find(|role| role.field == field)
        .map(|role| role.label)
}

/// The "Player assignments" group's rows (the draft's `#tac-side
/// .tachalf`): one column of `.tset` rows — each of the
/// six takers and the captain its label over a chip the column's width,
/// each label 4 points under the chip above it,
/// then "Join the attack" over three chips, one per joining player. A
/// chip click arms its mode and closes the slot popover; a click on an
/// armed chip disarms it (the draft's `roleChip`/`joinChip`), and any
/// of the three join chips arms the one join mode, all three filled
/// while it is armed. The panel's heading is the caller's `panel`.
pub(super) fn player_assignments(
    ui: &mut egui::Ui,
    set_pieces: &SetPieceTakers,
    joined: &[u8; 3],
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) {
    // The frame fills the cell the host gives the panel (the pitch's
    // height), as `formation_settings`' does.
    ui.set_min_height(ui.available_height());
    // `.tachalf`: a label 2 above its chip, 4 under the one above.
    ui.spacing_mut().item_spacing.y = 2.0;
    for (index, role) in SET_PIECES.iter().enumerate() {
        ui.label(role.label);
        let holder = (role.get)(set_pieces);
        let name = if holder == u8::MAX {
            "Unassigned".to_owned()
        } else {
            name_at(roster, holder).to_owned()
        };
        let armed = view.assign == Some(Assign::Role(index));
        if chip(ui, &name, armed, "Click, then a pitch player").clicked() {
            view.assign = (!armed).then_some(Assign::Role(index));
            view.popover = None;
        }
        // 4 points to the next row.
        ui.add_space(2.0);
    }
    // "Join the attack" shows its three players as three rows, one per
    // joining player — `Unassigned` where fewer join — all three the
    // one picker's chips.
    ui.label("Join the attack");
    let join_armed = view.assign == Some(Assign::Join);
    let members: Vec<u8> = joined
        .iter()
        .copied()
        .filter(|&slot| slot != u8::MAX)
        .collect();
    for k in 0..3 {
        let name = members
            .get(k)
            .map(|&slot| name_at(roster, slot).to_owned())
            .unwrap_or_else(|| "Unassigned".to_owned());
        if chip(
            ui,
            &name,
            join_armed,
            "Click, then up to three pitch players",
        )
        .clicked()
        {
            view.assign = (!join_armed).then_some(Assign::Join);
            view.popover = None;
        }
    }
}

/// A chip's padding (the draft's `.rolechip` `padding: 1px 6px`).
const CHIP_PADDING: egui::Vec2 = egui::vec2(6.0, 1.0);

/// One chip of the palette (the draft's `.rolechip`): a filled button
/// the column's width and `interact_size.y` tall — the selection fill
/// while armed, no fill while disabled — the holder's name left-aligned
/// and cut with an ellipsis at the edge, the full name its hover then,
/// `hint` when it fits.
pub(super) fn chip(
    ui: &mut egui::Ui,
    name: &str,
    armed: bool,
    hint: &'static str,
) -> egui::Response {
    ui.spacing_mut().button_padding = CHIP_PADDING;
    let width = ui.available_width();
    let button = egui::Button::new(name.to_owned())
        .selected(armed)
        .truncate()
        .min_size(egui::vec2(width, ui.spacing().interact_size.y));
    // A disabled chip paints neither fill nor border but keeps its
    // padding, so its text stands where an enabled chip's does:
    // `.frame(false)` would drop the padding with the frame.
    let button = if ui.is_enabled() {
        button
    } else {
        button
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
    };
    let response = ui.add(button);
    let chrome = 2.0 * ui.spacing().button_padding.x;
    if super::text_width(ui, name) > width - chrome {
        response.on_hover_text(name.to_owned())
    } else {
        response.on_hover_text(hint)
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{self, Role};

    use pes_savefile::model::instruction::Instruction;
    use pes_version::PesVersion;

    use super::*;
    use crate::tactics::tests::{click, named_roster};
    use crate::test_support::{
        Headless, chips_below, fixture_tactics, named_assignments, roster_rows, tactics_settings,
    };

    // The draft's "Player assignments" column (`.tachalf`): each of the
    // six takers and the captain its label over a chip showing the
    // holder's name ("Unassigned" when none), then "Join the attack"
    // over three chips, one per joining player; a click arms, an armed
    // click disarms, and all three join chips show armed together.
    #[test]
    fn the_assignment_rows_name_their_holders_and_arm_the_modes() {
        let mut window = Headless::sized(1400.0, 1200.0);
        let version = PesVersion::Pes21;
        let mut tactics = fixture_tactics(version);
        // Seven different holders with different names (the fixture's
        // five takers share one player, the second taker and the captain
        // another, and its players are all "PLACEHOLDER"), so a chip
        // drawing another role's holder shows.
        named_assignments(&mut tactics);
        tactics.players_to_join_attack = [tactics.starting_eleven[2], u8::MAX, u8::MAX];
        let mut view = TacticsView::default();
        let rows = named_roster();
        let roster = roster_rows(&rows);
        // Each chip's holder read from its own field, not through the
        // getters that draw it.
        let pieces = &tactics.set_pieces;
        let holders = [
            ("Long free kick", pieces.free_kick_long),
            ("Short free kick", pieces.free_kick_short),
            ("Second free kick", pieces.free_kick_second),
            ("Left corner", pieces.corner_left),
            ("Right corner", pieces.corner_right),
            ("Penalty kick", pieces.penalty),
            ("Captain", pieces.captain),
        ];
        let names: std::collections::HashSet<&str> = holders
            .iter()
            .map(|&(_, holder)| name_at(&roster, holder))
            .collect();
        assert_eq!(names.len(), holders.len(), "premise: seven different names");
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        for (label, holder) in holders {
            let expected = name_at(&roster, holder);
            // The chip directly under the label (the next role's chip
            // is a label further down).
            let chip = chips_below(&nodes, label)
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("{label}: no chip under its label"));
            assert_eq!(
                chip.label(),
                Some(expected),
                "{label}'s chip names its holder"
            );
        }
        // "Join the attack": one chip per entry — a name where one
        // joins, "Unassigned" where none.
        let joined: Vec<String> = tactics
            .players_to_join_attack
            .iter()
            .filter(|&&slot| slot != u8::MAX)
            .map(|&slot| name_at(&roster, slot).to_owned())
            .collect();
        let expected: Vec<String> = (0..3)
            .map(|k| {
                joined
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| "Unassigned".to_owned())
            })
            .collect();
        let chips = chips_below(&nodes, "Join the attack");
        assert_eq!(chips.len(), 3, "three join chips");
        for (chip, expected) in chips.iter().zip(&expected) {
            assert_eq!(chip.label(), Some(expected.as_str()));
        }

        // A role chip click arms its mode and closes the slot popover;
        // the armed chip shows selected; a second click disarms.
        let penalty = chips_below(&nodes, "Penalty kick")[0];
        assert_eq!(
            penalty.toggled(),
            Some(accesskit::Toggled::False),
            "starts unarmed"
        );
        let (_, changed) = click(
            &mut window,
            penalty,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "arming writes no record");
        assert_eq!(view.assign, Some(Assign::Role(5)), "armed for the penalty");
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let penalty = chips_below(&nodes, "Penalty kick")[0];
        assert_eq!(
            penalty.toggled(),
            Some(accesskit::Toggled::True),
            "the armed chip is selected"
        );
        let (_, changed) = click(
            &mut window,
            penalty,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "disarming writes no record");
        assert_eq!(view.assign, None, "the second click disarms");

        // Any join chip arms the one join mode, all three selected
        // while armed; an armed click disarms.
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let third = chips_below(&nodes, "Join the attack")[2];
        let (_, changed) = click(
            &mut window,
            third,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "arming writes no record");
        assert_eq!(view.assign, Some(Assign::Join), "join armed");
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        for chip in chips_below(&nodes, "Join the attack") {
            assert_eq!(
                chip.toggled(),
                Some(accesskit::Toggled::True),
                "all three chips show armed"
            );
        }
        let third = chips_below(&nodes, "Join the attack")[2];
        let (_, changed) = click(
            &mut window,
            third,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "disarming writes no record");
        assert_eq!(view.assign, None, "the armed click disarms");
    }

    // A chip paints as the draft's `.rolechip` button: its fill over its
    // own bounds — the widgets' `weak_bg_fill` at rest, the selection
    // fill armed — the panel column's width and `interact_size.y` tall,
    // the join chips stepping that height plus the panel's gap.
    #[test]
    fn a_chip_paints_its_fill_at_the_columns_width() {
        let mut window = Headless::sized(800.0, 1200.0);
        let (interact, rest_fill, armed_fill, column) = {
            let style = window.ctx.style_of(window.ctx.theme());
            let margin = egui::Frame::group(&style).total_margin();
            (
                style.spacing.interact_size.y,
                style.visuals.widgets.inactive.weak_bg_fill,
                style.visuals.selection.bg_fill,
                800.0 - margin.left - margin.right,
            )
        };
        let version = PesVersion::Pes21;
        let mut tactics = fixture_tactics(version);
        named_assignments(&mut tactics);
        // The penalty's chip armed.
        let mut view = TacticsView {
            assign: Some(Assign::Role(5)),
            ..Default::default()
        };
        let rows = named_roster();
        let roster = roster_rows(&rows);
        let (nodes, shapes) = window.frame(vec![], |ui| {
            tactics_settings(ui, "tactics", version, &mut tactics, &mut view, &roster);
        });

        // The rect shape `fill` paints at `bounds`, or `None`.
        let painted = |fill: egui::Color32, bounds: accesskit::Rect| {
            shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(shape) if shape.fill == fill => Some(shape.rect),
                    _ => None,
                })
                .find(|rect| {
                    (f64::from(rect.min.x) - bounds.x0).abs() <= 0.5
                        && (f64::from(rect.min.y) - bounds.y0).abs() <= 0.5
                        && (f64::from(rect.max.x) - bounds.x1).abs() <= 0.5
                        && (f64::from(rect.max.y) - bounds.y1).abs() <= 0.5
                })
        };
        for (label, fill, state) in [
            ("Long free kick", rest_fill, "at rest"),
            ("Penalty kick", armed_fill, "armed"),
        ] {
            let bounds = chips_below(&nodes, label)
                .into_iter()
                .next()
                .and_then(|chip| chip.bounds())
                .unwrap_or_else(|| panic!("{label}'s chip has bounds"));
            let rect = painted(fill, bounds)
                .unwrap_or_else(|| panic!("{label}'s chip paints its fill {state}"));
            assert!(
                (rect.width() - column).abs() <= 1.0,
                "{label}'s chip is the column's width ({:?} vs {column})",
                rect.width()
            );
            assert!(
                (rect.height() - interact).abs() <= 0.5,
                "{label}'s chip is {interact} tall ({:?})",
                rect.height()
            );
        }

        // The join chips' 20-point rows: `interact_size.y` plus the
        // panel's 2-point item gap.
        let joins = chips_below(&nodes, "Join the attack");
        assert_eq!(joins.len(), 3, "three join chips");
        for pair in joins.windows(2) {
            let (above, below) = (
                pair[0].bounds().expect("chip bounds"),
                pair[1].bounds().expect("chip bounds"),
            );
            assert!(
                (below.y0 - above.y0 - f64::from(interact) - 2.0).abs() <= 0.5,
                "the join chips step {interact} plus the 2-point gap ({above:?} vs {below:?})"
            );
        }
    }

    // A disabled chip (an instruction that takes no target) keeps no
    // fill and no border — the draft's `.rolechip:disabled` — and its
    // padding: the text stands `CHIP_PADDING.x` in from its left edge.
    #[test]
    fn a_disabled_chip_paints_neither_fill_nor_stroke() {
        let mut window = Headless::sized(800.0, 1200.0);
        let version = PesVersion::Pes21;
        let mut tactics = fixture_tactics(version);
        // Every instruction a target-less one, so all four chips are
        // disabled, reading "No target".
        let off =
            instruction::encode(version, Instruction::TikiTaka).expect("PES 21 stores Tiki-Taka");
        for entry in tactics.presets[0]
            .attack_instructions
            .as_mut()
            .expect("PES 21 has instructions")
            .iter_mut()
            .chain(
                tactics.presets[0]
                    .defence_instructions
                    .as_mut()
                    .expect("PES 21 has instructions")
                    .iter_mut(),
            )
        {
            entry.instruction = off;
        }
        let mut view = TacticsView::default();
        let rows = named_roster();
        let roster = roster_rows(&rows);
        let (nodes, shapes) = window.frame(vec![], |ui| {
            tactics_settings(ui, "tactics", version, &mut tactics, &mut view, &roster);
        });

        let chips: Vec<&accesskit::Node> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node.label() == Some("No target")
                    && node.is_disabled()
            })
            .collect();
        assert_eq!(chips.len(), 4, "four disabled No target chips");
        for chip in chips {
            let bounds = chip.bounds().expect("the chip's bounds");
            let text_left = shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "No target"
                            && (f64::from(text.pos.y) - bounds.y0).abs()
                                <= bounds.y1 - bounds.y0 =>
                    {
                        Some(f64::from(text.pos.x))
                    }
                    _ => None,
                })
                .min_by(|a, b| (a - bounds.x0).abs().total_cmp(&(b - bounds.x0).abs()))
                .expect("the chip's text");
            assert!(
                (text_left - bounds.x0 - f64::from(CHIP_PADDING.x)).abs() <= 0.5,
                "a disabled chip keeps its padding ({text_left} vs {bounds:?})"
            );
            for clipped in &shapes {
                let egui::Shape::Rect(shape) = &clipped.shape else {
                    continue;
                };
                let own = (f64::from(shape.rect.min.x) - bounds.x0).abs() <= 0.5
                    && (f64::from(shape.rect.min.y) - bounds.y0).abs() <= 0.5
                    && (f64::from(shape.rect.max.x) - bounds.x1).abs() <= 0.5
                    && (f64::from(shape.rect.max.y) - bounds.y1).abs() <= 0.5;
                assert!(
                    !(own && shape.fill != egui::Color32::TRANSPARENT),
                    "a disabled chip paints no fill: {:?} at {bounds:?}",
                    shape.rect
                );
                assert!(
                    !(own && !shape.stroke.is_empty()),
                    "a disabled chip paints no border: {:?} at {bounds:?}",
                    shape.stroke
                );
            }
        }
    }

    // A stored join list that already holds a gap comes out packed from a
    // toggle: removing its first member moves the one after the gap up.
    #[test]
    fn a_join_toggle_packs_a_stored_list_with_a_gap() {
        let version = PesVersion::Pes21;
        let mut tactics = fixture_tactics(version);
        let (a, b) = (tactics.starting_eleven[0], tactics.starting_eleven[2]);
        tactics.players_to_join_attack = [a, u8::MAX, b];
        let mut view = TacticsView::default();
        Assign::Join.write(version, &mut tactics, &mut view, 0);
        assert_eq!(tactics.players_to_join_attack, [b, u8::MAX, u8::MAX]);
    }
}
