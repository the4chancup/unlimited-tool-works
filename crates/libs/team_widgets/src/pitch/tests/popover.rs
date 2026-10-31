//! The slot popover's tests and the armed assign modes': the open
//! and close cases, the fields' writes, the role chips and join.

use egui::accesskit::{self, Role};
use pes_savefile::model::instruction::Instruction;
use pes_savefile::model::tactics::SetPieceTakers;
use pes_savefile::schema::fields::InstructionSide;
use pes_savefile::schema::instruction;
use pes_version::PesVersion;

use super::*;
use crate::test_support::{
    Headless, button_labels, centre, chip_under, chips_below, combo_row, entry, entry_id,
    fixture_rows, fixture_tactics, pointer_at, roster_rows,
};

// The draft's role palette: arm a chip and the note asks for its pitch
// player; the click writes his roster slot to the taker and disarms.
#[test]
fn a_role_chip_then_a_marker_click_writes_the_taker_and_disarms() {
    // The window is wide enough to hold the whole chips row (the
    // settings pane's content is wider than the pitch column).
    let mut window = Headless::sized(1400.0, 1100.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let index = 7;
    // A non-identity eleven: an arm writing the marker's index for the
    // roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(2, index);
    let starter = tactics.starting_eleven[index];
    assert_ne!(starter, index as u8, "premise: slot and index differ");
    assert_ne!(
        tactics.set_pieces.penalty, starter,
        "premise: the marker's player is not already the taker"
    );

    // Arm the Penalty kick chip (the first chip under its label).
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chips_below(&nodes, "Penalty kick")
        .into_iter()
        .next()
        .expect("the penalty chip");
    let (_, changed) =
        window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(!changed, "arming writes no record");
    assert_eq!(view.assign, Some(Assign::Role(5)), "the mode is armed");

    // The marker click writes the taker. No note is drawn while armed: the
    // highlighted chip and the candidates' corner brackets say it.
    let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Button && matches!(node.label(), Some("Cancel") | Some("Done"))
        }),
        "no Cancel or Done while armed"
    );
    assert!(
        !nodes.iter().any(|node| {
            node.role() == Role::Label
                && node
                    .value()
                    .is_some_and(|value| value.starts_with("Click the pitch player"))
        }),
        "no armed note's text"
    );
    let formation = tactics.presets[0].formations[0].clone();
    let at = marker_point(&nodes, &formation.players, index);
    let (_, changed) = window.click_pitch(at, version, &mut tactics, &mut view, &roster);
    assert!(changed, "the assignment writes the record");
    assert_eq!(
        tactics.set_pieces.penalty, starter,
        "the marker's player takes it"
    );
    assert_eq!(view.assign, None, "and the mode disarms");
}

// Every chip writes its own field: arm it, click a starter's marker —
// the chip's field alone takes the clicked starter's roster slot (the
// fields' own names, never `SET_PIECES`'s table).
#[test]
fn each_assignment_chip_writes_its_own_field() {
    let mut window = Headless::sized(1400.0, 1100.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // A non-identity eleven: no index equals its roster slot.
    tactics.starting_eleven.rotate_left(1);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    /// One chip case: its label prefix, its field's read and the expected
    /// write — the field's own name, never `SET_PIECES`'s table.
    struct Chip {
        /// The chip's label prefix.
        label: &'static str,
        /// The field's stored read.
        get: fn(&SetPieceTakers) -> u8,
        /// The expected write, on a `TeamTactics` clone.
        set: fn(&mut SetPieceTakers, u8),
    }
    let cases: [Chip; 7] = [
        Chip {
            label: "Long free kick",
            get: |s| s.free_kick_long,
            set: |s, v| s.free_kick_long = v,
        },
        Chip {
            label: "Short free kick",
            get: |s| s.free_kick_short,
            set: |s, v| s.free_kick_short = v,
        },
        Chip {
            label: "Second free kick",
            get: |s| s.free_kick_second,
            set: |s, v| s.free_kick_second = v,
        },
        Chip {
            label: "Left corner",
            get: |s| s.corner_left,
            set: |s, v| s.corner_left = v,
        },
        Chip {
            label: "Right corner",
            get: |s| s.corner_right,
            set: |s, v| s.corner_right = v,
        },
        Chip {
            label: "Penalty kick",
            get: |s| s.penalty,
            set: |s, v| s.penalty = v,
        },
        Chip {
            label: "Captain",
            get: |s| s.captain,
            set: |s, v| s.captain = v,
        },
    ];

    // Markers a click can reach at their box's centre: no marker drawn
    // after them covers it (egui resolves to the topmost).
    let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let centres = centres_of(canvas, &formation.players);
    let order = draw_order(&centres);
    let uncovered: Vec<usize> = (1..11)
        .filter(|&index| {
            let after = order
                .iter()
                .position(|&i| i == index)
                .expect("every marker is in the order");
            let at = marker_rect(centres[index], canvas).center();
            order[after + 1..]
                .iter()
                .all(|&other| !marker_rect(centres[other], canvas).contains(at))
        })
        .collect();
    assert!(
        uncovered.len() >= cases.len(),
        "premise: enough open markers"
    );

    for (index, case) in cases.iter().enumerate() {
        let chip_label = case.label;
        // Arm the chip under its label.
        let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
        let chip = chips_below(&nodes, chip_label)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no chip under {chip_label:?}"));
        let (_, changed) =
            window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
        assert!(!changed, "{chip_label}: arming writes no record");
        assert_eq!(view.assign, Some(Assign::Role(index)), "{chip_label} armed");

        let marker_index = uncovered[index];
        let starter = tactics.starting_eleven[marker_index];
        assert_ne!(
            starter, marker_index as u8,
            "{chip_label}: slot and index differ"
        );
        assert_ne!(
            (case.get)(&tactics.set_pieces),
            starter,
            "{chip_label}: the marker's player is not already the holder"
        );
        let mut expected = tactics.clone();
        (case.set)(&mut expected.set_pieces, starter);

        let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
        let at = marker_point(&nodes, &formation.players, marker_index);
        let (_, changed) = window.click_pitch(at, version, &mut tactics, &mut view, &roster);
        assert!(changed, "{chip_label}: the click wrote the field");
        assert_eq!(tactics, expected, "{chip_label}: only its field written");
        assert_eq!(view.assign, None, "{chip_label}: disarmed");
    }
}

// The join pick toggles members until the chip's click disarms: three
// clicks fill the three entries, a fourth is ignored, and clicking a
// member removes him and moves the later ones up, so the record never
// holds a gap.
#[test]
fn join_clicks_fill_three_then_toggle_members_packed() {
    let mut window = Headless::sized(900.0, 1100.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.players_to_join_attack = [u8::MAX; 3];
    // A non-identity eleven: an arm writing the marker's index for the
    // roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(4, 9);
    tactics.starting_eleven.swap(5, 10);
    tactics.starting_eleven.swap(6, 8);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);

    // Arm join through a join chip.
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chips_below(&nodes, "Join the attack")
        .into_iter()
        .next()
        .expect("a join chip");
    window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert_eq!(view.assign, Some(Assign::Join), "the mode is armed");

    // Three marker clicks fill the list, packed.
    let starters = tactics.starting_eleven;
    assert!(
        starters[4..=6]
            .iter()
            .enumerate()
            .all(|(i, s)| *s != (i + 4) as u8),
        "premise: the clicked positions' slots differ from their indices"
    );
    let click_marker =
        |window: &mut Headless, tactics: &mut TeamTactics, view: &mut TacticsView, index: usize| {
            let (nodes, _) = window.pitch(version, tactics, view, &roster, vec![]);
            let at = marker_point(&nodes, &tactics.presets[0].formations[0].players, index);
            window.click_pitch(at, version, tactics, view, &roster)
        };
    let (_, changed) = click_marker(&mut window, &mut tactics, &mut view, 4);
    assert!(changed, "the first click wrote");
    let (_, changed) = click_marker(&mut window, &mut tactics, &mut view, 5);
    assert!(changed, "the second click wrote");
    let (_, changed) = click_marker(&mut window, &mut tactics, &mut view, 6);
    assert!(changed, "the third click wrote");
    assert_eq!(
        tactics.players_to_join_attack,
        [starters[4], starters[5], starters[6]],
        "three clicks fill three"
    );
    let (_, changed) = click_marker(&mut window, &mut tactics, &mut view, 7);
    assert!(!changed, "a fourth is ignored");
    assert_eq!(
        tactics.players_to_join_attack,
        [starters[4], starters[5], starters[6]],
        "and the list is as it was"
    );
    // Clicking the first member removes him; the list packs.
    let (_, changed) = click_marker(&mut window, &mut tactics, &mut view, 4);
    assert!(changed, "the removal wrote");
    assert_eq!(
        tactics.players_to_join_attack,
        [starters[5], starters[6], u8::MAX],
        "the first member left and the list packed"
    );
    assert_eq!(view.assign, Some(Assign::Join), "join stays armed");

    // A click on an armed join chip disarms. No note explains the armed
    // mode: the highlighted chips and the candidates' corner brackets say
    // it.
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chips_below(&nodes, "Join the attack")
        .into_iter()
        .next()
        .expect("a join chip");
    window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert_eq!(view.assign, None, "the armed chip's click disarms");
}

// An instruction's target chip arms the same pitch pick as the roles' —
// click the chip, then a starter's marker writes his roster slot as the
// target and disarms; a click on the player already the target writes
// nothing; and an instruction changed while armed disarms the mode
// without a write.
#[test]
fn a_target_chip_then_a_marker_click_writes_the_target_and_disarms() {
    // The window is wide enough to hold the whole chips row.
    let mut window = Headless::sized(1400.0, 1100.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // Defence 1 Counter Target, its pick unset; a non-identity eleven
    // (an index write would look right on the fixture's [0, 1, ...]).
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction =
        instruction::encode(version, Instruction::CounterTarget).expect("PES 21 stores it");
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("instructions")[0]
        .player = u8::MAX;
    tactics.starting_eleven.swap(3, 9);
    let starter = tactics.starting_eleven[3];
    assert_ne!(starter, 3, "premise: slot and index differ");
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    // The slot popover open: arming closes it.
    view.popover = Some(OpenSlot {
        scope: egui::Id::unique("the open slot's"),
        preset: 0,
        formation: 0,
        slot: 0,
        at: egui::Vec2::ZERO,
    });

    // The chip under defence 1's combo: enabled; its click arms
    // `Assign::Target` and closes the popover.
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[1]);
    assert!(!chip.is_disabled(), "Counter Target's chip is enabled");
    let (_, changed) =
        window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(!changed, "arming writes no record");
    assert_eq!(
        view.assign,
        Some(Assign::Target {
            side: InstructionSide::Defence,
            index: 0
        }),
        "the mode is armed"
    );
    assert_eq!(view.popover, None, "the slot popover closed");
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[1]);
    assert_eq!(
        chip.toggled(),
        Some(accesskit::Toggled::True),
        "the armed chip is selected"
    );

    // A click on the armed chip disarms (the join chip's rule); a second
    // arms it again for the marker half.
    let (_, changed) =
        window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(!changed, "disarming writes no record");
    assert_eq!(view.assign, None, "the armed chip's click disarms");
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[1]);
    assert_eq!(
        chip.toggled(),
        Some(accesskit::Toggled::False),
        "and it is no longer selected"
    );
    let (_, _) = window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(view.assign.is_some(), "re-armed");

    // The marker click writes the starter's roster slot and disarms.
    let formation = tactics.presets[0].formations[0].clone();
    let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, 3);
    let (_, changed) = window.click_pitch(at, version, &mut tactics, &mut view, &roster);
    assert!(changed, "the assignment writes the record");
    assert_eq!(
        tactics.presets[0]
            .defence_instructions
            .as_ref()
            .expect("instructions")[0]
            .player,
        starter,
        "the starter's roster slot is the target"
    );
    assert_eq!(view.assign, None, "and the mode disarms");

    // Re-arm; a click on the player already the target writes nothing
    // (the mode still disarms).
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[1]);
    let (_, _) = window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(view.assign.is_some(), "re-armed");
    let before = tactics.clone();
    let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, 3);
    let (_, changed) = window.click_pitch(at, version, &mut tactics, &mut view, &roster);
    assert!(!changed, "the same player writes nothing");
    assert_eq!(tactics, before, "the record is as it was");
    assert_eq!(view.assign, None, "still disarmed");

    // Armed again, the instruction switched to Deep Defensive Line: it
    // takes no target — the marker click writes nothing and disarms.
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[1]);
    let (_, _) = window.click_settings(centre(chip), version, &mut tactics, &mut view, &roster);
    assert!(view.assign.is_some(), "re-armed");
    // Pick Deep Defensive Line on defence 1's combo — the popup's item
    // goes through AccessKit, as the instruction picks' do.
    let (nodes, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[1];
    let (_, _) = window.click_settings(centre(combo), version, &mut tactics, &mut view, &roster);
    // The popup's first frame is a sizing pass; its items join the
    // tree on the next.
    let (_, _) = window.settings(version, &mut tactics, &mut view, &roster, vec![]);
    let (ids, _) = window.frame_at_ids(None, vec![], |ui| {
        crate::test_support::tactics_settings(
            ui,
            "tactics",
            version,
            &mut tactics,
            None,
            &mut view,
            &roster,
        );
    });
    let item = entry_id(&ids, Role::Button, "Deep Defensive Line");
    let (_, _) = window.settings(
        version,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: item,
                data: None,
            },
        )],
    );
    assert_eq!(
        tactics.presets[0]
            .defence_instructions
            .as_ref()
            .expect("instructions")[0]
            .instruction,
        instruction::encode(version, Instruction::DeepDefensiveLine).expect("PES 21 stores it"),
        "the pick changed the instruction while armed"
    );
    assert!(view.assign.is_some(), "the mode stays armed");
    // The marker click writes nothing (the instruction takes no
    // target) and disarms.
    let (nodes, _) = window.pitch(version, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, 3);
    let (_, changed) = window.click_pitch(at, version, &mut tactics, &mut view, &roster);
    assert!(!changed, "a target-less instruction writes nothing");
    assert_eq!(
        tactics.presets[0]
            .defence_instructions
            .as_ref()
            .expect("instructions")[0]
            .player,
        u8::MAX,
        "the player byte is untouched"
    );
    assert_eq!(view.assign, None, "and the mode disarms");
}

// The popover's slot is of the preset it opened on: a preset switch
// closes it (a formation switch is
// `the_popover_closes_when_the_formation_changes`'s case).
#[test]
fn a_preset_switch_closes_the_popover() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let index = 3;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, index);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    entry(&nodes, Role::Button, "Close");

    view.preset = 1;
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(view.popover, None, "a preset switch closes the popover");
}

// The popover's Position combo lists the positions the slot's spot
// offers and writes the picked position byte to the shown formation's
// slot alone: the fixture's RB at (89, 12) is offered CB and RB, and
// picks CB.
#[test]
fn the_popovers_position_pick_writes_the_slot() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 3;
    // A non-identity eleven: a write keyed by his roster slot rather
    // than the slot's index would look right on the fixture's
    // [0, 1, ...].
    tactics.starting_eleven.swap(index, 9);
    assert_ne!(
        tactics.starting_eleven[index], index as u8,
        "premise: slot and index differ"
    );
    let before = tactics.clone();
    let formation = before.presets[0].formations[0].clone();
    let cb = position_byte("CB");
    assert_eq!(
        (
            formation.players[index].position,
            formation.players[index].x,
            formation.players[index].y
        ),
        (position_byte("RB"), 89, 12),
        "premise: the RB at (89, 12), a real change"
    );

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &formation.players, index);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let combo = entry(&nodes, Role::ComboBox, "Position");
    let closed = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.click_pitch(
        centre(combo),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    // The item pick goes through AccessKit, as the instruction
    // combos' do: a pointer click does not reach a combo's popup.
    let ids = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    // The popup's buttons, unfiltered: every button the open frame
    // holds that the closed one did not, so a GK entry would show.
    let shown: std::collections::HashSet<accesskit::NodeId> =
        closed.iter().map(|(id, _)| *id).collect();
    let popup: Vec<accesskit::Node> = ids
        .iter()
        .filter(|(id, _)| !shown.contains(id))
        .map(|(_, node)| node.clone())
        .collect();
    assert_eq!(
        button_labels(&popup, |_| true),
        ["CB", "RB"],
        "the spot's two positions, the band's default first"
    );
    let item = entry_id(&ids, Role::Button, "CB");
    let (_, changed) =
        window.action_click_pitch(item, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(changed, "the pick changed the record");
    assert_eq!(
        tactics.presets[0].formations[0].players[index].position, cb,
        "the pick wrote CB"
    );
    assert_eq!(
        tactics.presets[0].formations[1..],
        before.presets[0].formations[1..],
        "the other two formations untouched"
    );
    let mut expect = before;
    expect.presets[0].formations[0].players[index].position = cb;
    assert_eq!(tactics, expect, "the slot's position alone changed");
    assert!(view.popover.is_some(), "the pick leaves the popover open");
}

// The popover writes the shown preset's shown formation, not a fixed
// one: an X and a Y edit and a position pick on preset 2's
// In-possession change that slot there alone. The edits come first: the
// combo's two-row list stays open after an AccessKit pick, over the X
// field.
#[test]
fn the_popovers_writes_land_in_the_shown_preset_and_formation() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView {
        preset: 1,
        formation: 1,
        ..TacticsView::default()
    };
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // The fixture's preset 2 In-possession holds preset 1's Kick-off's
    // outfield slots: the slot is moved a unit up there, so the two
    // formations' slots differ.
    let index = 3;
    tactics.presets[1].formations[1].players[index].y += 1;
    // Fluid formation on, so the preset's In-possession can be shown.
    tactics.presets[1].style.fluid = Some(true);
    // A non-identity eleven: a write keyed by his roster slot rather
    // than the slot's index would look right on the fixture's
    // [0, 1, ...].
    tactics.starting_eleven.swap(index, 9);
    assert_ne!(
        tactics.starting_eleven[index], index as u8,
        "premise: slot and index differ"
    );
    let before = tactics.clone();
    let players = before.presets[1].formations[1].players;
    assert!(players[index].position != 0, "premise: an outfield slot");
    assert_ne!(
        players[index], before.presets[0].formations[0].players[index],
        "premise: the slot differs from preset 1's Kick-off"
    );
    // The roster rows hold no overalls, so the X and Y edits keep his
    // RB; his new spot (55, 30) then offers CMF and AMF, his RB listed
    // first, and the pick is CMF.
    let cmf = position_byte("CMF");
    assert_eq!(
        players[index].position,
        position_byte("RB"),
        "premise: the RB, a real position change"
    );
    assert_ne!(players[index].x, 55, "premise: a real x change");
    // The typed y is neither the slot's y nor its x, old or new, so a Y
    // field writing x (or nothing) shows.
    let y = 30;
    assert!(y <= GRID_Y, "premise: y on the grid");
    assert_ne!(players[index].y, y, "premise: a real y change");
    assert_ne!(players[index].x, y, "premise: y is not the slot's x");

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &players, index);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.type_in_spin_field(
        "X",
        "55",
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    window.type_in_spin_field(
        "Y",
        &y.to_string(),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let combo = entry(&nodes, Role::ComboBox, "Position");
    window.click_pitch(
        centre(combo),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    let ids = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let item = entry_id(&ids, Role::Button, "CMF");
    window.action_click_pitch(item, PesVersion::Pes21, &mut tactics, &mut view, &roster);

    let mut expect = before;
    expect.presets[1].formations[1].players[index].position = cmf;
    expect.presets[1].formations[1].players[index].x = 55;
    expect.presets[1].formations[1].players[index].y = y;
    assert_eq!(
        tactics, expect,
        "only preset 2's In-possession slot took the position, x and y"
    );
}

/// The open slot popover's rect: its `Area` is `open.scope`'s
/// "slot popover" layer (a combo's open popup is another Foreground
/// layer beside it, so an order scan cannot pick it out).
fn popover_rect(ctx: &egui::Context, view: &TacticsView) -> Option<egui::Rect> {
    let scope = view.popover?.scope;
    ctx.memory(|memory| memory.area_rect(scope.with("slot popover")))
}

// The header reads the starter's name, then `player <n>` — his roster
// slot counted from 1. An empty roster slot's name reads "(empty roster
// slot)".
#[test]
fn the_header_names_the_starter_and_his_roster_slot() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    // A non-identity eleven: an impl printing the clicked index for the
    // roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(3, 7);
    let index = 7;
    assert_ne!(
        tactics.starting_eleven[index], index as u8,
        "premise: slot and index differ"
    );
    let label = |nodes: &[accesskit::Node], value: &str| {
        nodes
            .iter()
            .any(|node| node.role() == Role::Label && node.value() == Some(value))
    };

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &tactics.presets[0].formations[0].players, index);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert!(label(&nodes, "player 4"), "the header's `player 4`");

    // An empty roster slot (no player, so no name) heads it "(empty
    // roster slot)".
    let mut empty_rows = rows.clone();
    empty_rows[usize::from(tactics.starting_eleven[index])]
        .1
        .clear();
    let roster = roster_rows(&empty_rows);
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert!(
        label(&nodes, "(empty roster slot)"),
        "the empty slot's name"
    );
    assert!(label(&nodes, "player 4"), "and its slot's number");
}

// The popover's top-right corner sits on the marker's bottom-left
// corner as it stood at the click, and there it stays while the slot's
// X and Y move the marker under it, so the fields' arrows stay under the
// pointer rather than moving off with the marker.
#[test]
fn the_popover_stays_where_it_opened_while_the_marker_moves() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    // Room for the popover left of the marker's bottom-left and below
    // it, or the popup would flip to another corner and move the point
    // the test reads.
    let index = 4;
    let slot = &mut tactics.presets[0].formations[0].players[index];
    assert_ne!(slot.position, 0, "premise: an outfield slot");
    slot.x = 100;
    slot.y = 10;
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let players = tactics.presets[0].formations[0].players;
    let marker_bottom_left = marker_rect(centres_of(canvas, &players)[index], canvas).left_bottom();
    window.click_pitch(
        marker_point(&nodes, &players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let anchored = popover_rect(&window.ctx, &view).expect("the open popover's area");
    near(anchored.right_top(), marker_bottom_left, 0.5);

    // An AccessKit SetValue on the X field moves the marker left; the
    // popover stays.
    let ids = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let x_label = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some("X"))
        .and_then(|(_, node)| node.bounds())
        .expect("the X label");
    let x_field = ids
        .iter()
        .filter(|(_, node)| {
            node.role() == Role::SpinButton
                && node
                    .bounds()
                    .is_some_and(|b| (b.y0 - x_label.y0).abs() < 4.0 && b.x0 >= x_label.x1 - 1.0)
        })
        .min_by(|a, b| {
            let x =
                |pair: &&(accesskit::NodeId, accesskit::Node)| pair.1.bounds().expect("bounds").x0;
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .map(|(id, _)| *id)
        .expect("the X field");
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::SetValue,
                target_tree: accesskit::TreeId::ROOT,
                target_node: x_field,
                data: Some(accesskit::ActionData::NumericValue(30.0)),
            },
        )],
    );
    assert_eq!(
        tactics.presets[0].formations[0].players[index].x, 30,
        "premise: the SetValue moved the marker"
    );
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let after = popover_rect(&window.ctx, &view).expect("the popover still open");
    near(after.right_top(), anchored.right_top(), 0.5);
    near(after.left_bottom(), anchored.left_bottom(), 0.5);
}

// The popover is anchored to the canvas, not the screen: the host's
// scroll moves the pitch and the popover follows it by the same delta
// (the draft's popover lives in the pitch's wrapper).
#[test]
fn the_popover_scrolls_with_the_pitch() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let index = 3;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let players = tactics.presets[0].formations[0].players;
    window.click_pitch(
        marker_point(&nodes, &players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas_before = canvas_rect(&nodes);
    let popover_before = popover_rect(&window.ctx, &view).expect("the popover is open");

    // The canvas 40 points lower, as the scrolled host draws it: the
    // popover's rect moves by the same delta.
    let (nodes, _) = window.frame(vec![], |ui| {
        ui.add_space(40.0);
        pitch(
            ui,
            "pitch",
            PesVersion::Pes21,
            &mut tactics,
            &mut view,
            &roster,
            false,
        );
    });
    let canvas_after = canvas_rect(&nodes);
    let popover_after = popover_rect(&window.ctx, &view).expect("the popover still open");
    let moved = canvas_after.min - canvas_before.min;
    assert!(moved.y > 30.0, "premise: the canvas scrolled");
    near(popover_after.min, popover_before.min + moved, 0.5);
}

// A primary press anywhere outside the popover closes it, as Close
// does, and as the draft's document `pointerdown` does: a press on the
// grass closes it; a press inside it, or inside a popup it opened — the
// Position combo's list — is not outside; a press
// then release on another marker closes it and that marker's click then
// opens his.
#[test]
fn a_press_outside_closes_the_popover_but_a_press_inside_does_not() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let index = 3;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    window.click_pitch(
        marker_point(&nodes, &formation.players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(index),
        "premise: the popover is open"
    );

    // A press on the grass closes it: outside the open popover too,
    // which floats over the pitch's lower-left here.
    let grass = grass_point(
        canvas,
        &formation.players,
        popover_rect(&window.ctx, &view).expect("the popover is open"),
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(grass, true),
    );
    assert_eq!(view.popover, None, "the press on the grass closed it");
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(grass, false),
    );

    // Again: a press on the popover's Position combo, then one inside
    // the open list — neither is outside.
    window.click_pitch(
        marker_point(&nodes, &formation.players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let combo = entry(&nodes, Role::ComboBox, "Position");
    window.click_pitch(
        centre(combo),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(
        view.popover.is_some(),
        "the press on the combo is inside the popover"
    );
    let ids = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    // The RB's spot offers CB and RB.
    let cb = position_byte("CB");
    assert_ne!(
        formation.players[index].position, cb,
        "premise: a real change"
    );
    let (item, item_node) = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("CB"))
        .map(|(id, node)| (*id, node.clone()))
        .expect("the list's CB");
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(centre(&item_node), true),
    );
    assert!(
        view.popover.is_some(),
        "a press inside the combo's open list keeps it"
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(grass, false),
    );
    assert!(view.popover.is_some(), "and it is still open");
    let (_, changed) =
        window.action_click_pitch(item, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(changed, "the pick wrote the position");
    assert_eq!(
        tactics.presets[0].formations[0].players[index].position, cb,
        "the pick wrote CB"
    );
    assert!(view.popover.is_some(), "the pick leaves the popover open");

    // A press then release on another marker: the press closes the open
    // popover, the click opens that marker's. The marker's point has to
    // be outside the popover, which floats over the pitch's lower-left.
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let popover = popover_rect(&window.ctx, &view).expect("the popover is open");
    let other = (0..11)
        .filter(|&i| i != index)
        .find(|&i| !popover.contains(marker_point(&nodes, &formation.players, i)))
        .expect("some marker is free of the popover");
    window.click_pitch(
        marker_point(&nodes, &formation.players, other),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(other),
        "the press closed it and the click opened the other's"
    );

    // Its Close button, a press inside it, closes it and writes nothing.
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let close = centre(entry(&nodes, Role::Button, "Close"));
    let before = tactics.clone();
    window.click_pitch(close, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(view.popover, None, "Close closed it");
    assert!(
        nodes.iter().all(|node| node.label() != Some("Close")),
        "no popover is drawn on the next frame"
    );
    assert_eq!(tactics, before, "Close wrote nothing");
}

// Any button's press outside the popover closes it, not only the
// primary's (the draft's document `pointerdown` fires on every button):
// a secondary press on the grass.
#[test]
fn a_secondary_press_on_the_grass_closes_the_popover() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let index = 3;

    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    window.click_pitch(
        marker_point(&nodes, &formation.players, index),
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(index),
        "premise: the popover is open"
    );

    let grass = grass_point(
        canvas,
        &formation.players,
        popover_rect(&window.ctx, &view).expect("the popover is open"),
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![
            egui::Event::PointerMoved(grass),
            egui::Event::PointerButton {
                pos: grass,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ],
    );
    assert_eq!(
        view.popover, None,
        "a secondary press on the grass closed it"
    );
}

// The keeper's marker never moves (the draft's `d.gk`: `moved` never
// sets for him), so his release is his click however far the pointer
// moved: a press on him dragged past the click distance and released
// opens his popover — and, while a mode is armed, assigns him.
#[test]
fn the_keepers_release_is_his_click_however_far_the_pointer_moved() {
    let mut window = Headless::sized(600.0, 450.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let formation = tactics.presets[0].formations[0].clone();
    let keeper = formation
        .players
        .iter()
        .position(|slot| slot.position == 0)
        .expect("the fixture has a keeper slot");
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let canvas = canvas_rect(&nodes);
    let from = marker_point(&nodes, &formation.players, keeper);
    let onto = grass_point(canvas, &formation.players, egui::Rect::NOTHING);

    // Press, dragged far (past the click distance), released: his click.
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(onto)],
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(onto, false),
    );
    assert_eq!(
        view.popover.map(|open| open.slot),
        Some(keeper),
        "the far release is his click: the popover opened"
    );

    // Armed: the same release assigns him and disarms.
    view.popover = None;
    view.assign = Some(Assign::Role(5));
    let starter = tactics.starting_eleven[keeper];
    assert_ne!(
        tactics.set_pieces.penalty, starter,
        "premise: the keeper is not already the taker"
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(from, true),
    );
    window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        vec![egui::Event::PointerMoved(onto)],
    );
    let (_, changed) = window.pitch(
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(onto, false),
    );
    assert!(changed, "the assignment wrote the record");
    assert_eq!(
        tactics.set_pieces.penalty, starter,
        "the far release is his click: the keeper takes the role"
    );
    assert_eq!(view.assign, None, "and the mode disarmed");
}

/// The Position combo's list in slot `index`'s popover, opened by a
/// click on his marker and then on the combo: the labels of the buttons
/// the open list adds, top to bottom.
fn position_list(
    window: &mut Headless,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
    index: usize,
) -> Vec<String> {
    let players = tactics.presets[0].formations[0].players;
    let (nodes, _) = window.pitch(PesVersion::Pes21, tactics, view, roster, vec![]);
    let at = marker_point(&nodes, &players, index);
    window.click_pitch(at, PesVersion::Pes21, tactics, view, roster);
    let (nodes, _) = window.pitch(PesVersion::Pes21, tactics, view, roster, vec![]);
    let combo = entry(&nodes, Role::ComboBox, "Position");
    let closed = window.pitch_ids(PesVersion::Pes21, tactics, view, roster);
    window.click_pitch(centre(combo), PesVersion::Pes21, tactics, view, roster);
    let open = window.pitch_ids(PesVersion::Pes21, tactics, view, roster);
    let shown: std::collections::HashSet<accesskit::NodeId> =
        closed.iter().map(|(id, _)| *id).collect();
    let popup: Vec<accesskit::Node> = open
        .iter()
        .filter(|(id, _)| !shown.contains(id))
        .map(|(_, node)| node.clone())
        .collect();
    button_labels(&popup, |_| true)
}

// The Position combo lists the positions the slot's spot offers, as
// the game's Select Position menu does, on team 718's preset 1
// formation 1: its CMF at (53, 26) is offered CMF and AMF; its middle
// CB at (52, 7) CB alone; and the CMF made an LB, which the spot does
// not offer, lists his LB first.
#[test]
fn the_position_combo_lists_the_positions_the_spot_offers() {
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.presets[0].formations[0] = formation_718_1();
    let players = tactics.presets[0].formations[0].players;
    assert_eq!(
        (players[2].position, players[2].x, players[2].y),
        (position_byte("CMF"), 53, 26),
        "premise: slot 2, the CMF at (53, 26)"
    );
    assert_eq!(
        (players[6].position, players[6].x, players[6].y),
        (position_byte("CB"), 52, 7),
        "premise: slot 6, the CB at (52, 7)"
    );
    for (index, position, expected) in [
        (2, "CMF", vec!["CMF", "AMF"]),
        (6, "CB", vec!["CB"]),
        (2, "LB", vec!["LB", "CMF", "AMF"]),
    ] {
        let mut window = Headless::sized(600.0, 450.0);
        let mut view = TacticsView::default();
        tactics.presets[0].formations[0].players[index].position = position_byte(position);
        assert_eq!(
            position_list(&mut window, &mut tactics, &mut view, &roster, index),
            expected,
            "slot {index} as {position}"
        );
    }
}

// Under a ruleset that allows illegal formations the Position combo
// lists every outfield position, CB to CF, whatever the spot: team
// 718's middle CB at (52, 7), whose spot offers CB alone, is offered all
// twelve, and LB picked there makes him an LB, his X and Y kept.
#[test]
fn under_illegal_formations_the_position_combo_lists_every_outfield_position() {
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.presets[0].formations[0] = formation_718_1();
    let index = 6;
    let before = tactics.clone();
    let slot = before.presets[0].formations[0].players[index];
    assert_eq!(
        (slot.position, slot.x, slot.y),
        (position_byte("CB"), 52, 7),
        "premise: slot 6, the CB at (52, 7)"
    );
    let mut window = Headless::sized(600.0, 450.0);
    window.illegal_formations = true;
    let mut view = TacticsView::default();
    assert_eq!(
        position_list(&mut window, &mut tactics, &mut view, &roster, index),
        [
            "CB", "LB", "RB", "DMF", "CMF", "LMF", "RMF", "AMF", "LWF", "RWF", "SS", "CF"
        ],
        "the twelve outfield positions"
    );

    let ids = window.pitch_ids(PesVersion::Pes21, &mut tactics, &mut view, &roster);
    let item = entry_id(&ids, Role::Button, "LB");
    let (_, changed) =
        window.action_click_pitch(item, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    assert!(changed, "the pick changed the record");
    let mut expect = before;
    expect.presets[0].formations[0].players[index].position = position_byte("LB");
    assert_eq!(
        tactics, expect,
        "his position alone changed, his X and Y kept"
    );
}

// PARTY HARD's popover at (20, 26) lists CMF and LMF alone; his Y set to
// 36 puts him in the forwards' band, where he is offered SS and LWF and
// holds neither, so his higher overall makes him an LWF, 86 against 84,
// on the same frame as the Y.
// SE-GUI-42
#[test]
fn party_hards_popover_lists_cmf_and_lmf_and_a_y_of_36_makes_him_lwf() {
    let rows = fixture_rows(PesVersion::Pes21);
    let mut roster = roster_rows(&rows);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    party_hard(&mut tactics, &mut roster);
    let mut window = Headless::sized(600.0, 450.0);
    let mut view = TacticsView::default();
    assert_eq!(
        position_list(&mut window, &mut tactics, &mut view, &roster, 8),
        ["CMF", "LMF"],
        "the spot's two positions"
    );

    let mut window = Headless::sized(600.0, 450.0);
    let mut view = TacticsView::default();
    let before = tactics.clone();
    let (nodes, _) = window.pitch(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let at = marker_point(&nodes, &before.presets[0].formations[0].players, 8);
    window.click_pitch(at, PesVersion::Pes21, &mut tactics, &mut view, &roster);
    window.type_in_spin_field(
        "Y",
        "36",
        PesVersion::Pes21,
        &mut tactics,
        &mut view,
        &roster,
    );
    let mut expect = before;
    expect.presets[0].formations[0].players[8] = FormationSlot {
        position: position_byte("LWF"),
        x: 20,
        y: 36,
    };
    assert_eq!(tactics, expect, "his Y is 36 and he is an LWF, alone");
}
