use egui::accesskit::{self, Role};

use super::*;
use crate::tactics::TacticsView;
use crate::tactics::tests::click;
use crate::test_support::{
    Headless, button_labels, centre, chip_under, combo_row, entry_id, fixture_rows,
    fixture_tactics, named_assignments, pointer_at, roster_rows,
};

// An advanced instruction's list, as the draft's `instructionsFor`
// builds it: an attack combo lists the version's stored table in its
// stored order, the attacking instructions plus `Off`,
// less Tight Marking; a defence combo the defending ones plus
// `Off`, less Tight Marking. A defence slot holding an attacking
// instruction still shows it as the current choice.
#[test]
fn each_combo_lists_its_side_less_tight_marking() {
    let mut window = Headless::sized(800.0, 1200.0);
    for version in [PesVersion::Pes17, PesVersion::Pes19, PesVersion::Pes21] {
        let mut tactics = fixture_tactics(version);
        // Every chip shows a name, so no "Unassigned" chip counts
        // as an offered item below; defence 2 holds an attacking
        // instruction, kept as its current choice.
        named_assignments(&mut tactics);
        // Every instruction `Off` but defence 2's held attacking
        // one, so the target chips read "No target" and cannot
        // count as the combos' "Unassigned" item below.
        let encode = |inst| instruction::encode(version, inst).expect("the version stores it");
        for entry in tactics.presets[0]
            .attack_instructions
            .as_mut()
            .expect("instructions")
            .iter_mut()
            .chain(
                tactics.presets[0]
                    .defence_instructions
                    .as_mut()
                    .expect("instructions")
                    .iter_mut(),
            )
        {
            entry.instruction = encode(Instruction::Off);
        }
        tactics.presets[0]
            .defence_instructions
            .as_mut()
            .expect("instructions")[1]
            .instruction = encode(Instruction::HugTheTouchline);
        let mut view = TacticsView::default();
        let rows = fixture_rows(version);
        let roster = roster_rows(&rows);
        for (column, side, index, wrong_side) in [
            (0, InstructionSide::Attack, 0, false),
            (1, InstructionSide::Defence, 0, false),
            (1, InstructionSide::Defence, 1, true),
        ] {
            // Every version's list holds its own side's instructions
            // and `Off` alone.
            for inst in offered(version, side) {
                assert!(
                    inst == Instruction::Off || inst.side() == Some(side),
                    "{version:?} {side:?} offers {inst:?}, of the other side"
                );
            }
            let (closed, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
            let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
            let combo = combo_row(&nodes, index)[column];
            click(
                &mut window,
                combo,
                version,
                &mut tactics,
                &mut view,
                &roster,
            );
            // The popup's first frame is a sizing pass; egui 0.37
            // hides invisible widgets from the AccessKit tree, so
            // its items join from the next frame on.
            let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
            let (open, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
            // The popup's buttons, unfiltered: every button the open
            // frame holds that the closed one did not, in draw order,
            // so an item outside the expectation (Tight Marking, the
            // other side's) shows here rather than being filtered out.
            let before: std::collections::HashSet<accesskit::NodeId> =
                closed.iter().map(|(id, _)| *id).collect();
            let popup: Vec<accesskit::Node> = open
                .into_iter()
                .filter(|(id, _)| !before.contains(id))
                .map(|(_, node)| node)
                .collect();
            let drawn = button_labels(&popup, |_| true);
            // PES 2021's lists, from the draft's `instructionsFor`:
            // the stored table's order, its own side plus `Off`, less
            // Tight Marking.
            let mut expected: Vec<&str> = match (version, side) {
                (PesVersion::Pes21, InstructionSide::Attack) => vec![
                    "Unassigned",
                    "Hug the Touchline",
                    "False No. 9",
                    "False Full-Backs",
                    "Attacking Full-Backs",
                    "Wing Rotation",
                    "Tiki-Taka",
                    "Centering Targets",
                    "Defensive",
                    "False Winger",
                    "Anchoring",
                ],
                (PesVersion::Pes21, InstructionSide::Defence) => vec![
                    "Unassigned",
                    "Swarm the Box",
                    "Deep Defensive Line",
                    "Gegenpress",
                    "Counter Target",
                    "Wing-Back",
                ],
                _ => offered(version, side)
                    .iter()
                    .map(|inst| instruction_label(*inst))
                    .collect(),
            };
            if wrong_side {
                expected.push("Hug the Touchline");
            }
            assert_eq!(
                drawn, expected,
                "{version:?} {side:?}'s items in stored order"
            );
            if wrong_side {
                assert_eq!(
                    combo_row(&nodes, index)[column].value(),
                    Some("Hug the Touchline"),
                    "{version:?} a held wrong-side instruction shows selected"
                );
            }
            // Close the combo again for the next case's frame.
            let _ = click(
                &mut window,
                combo,
                version,
                &mut tactics,
                &mut view,
                &roster,
            );
        }
    }
}

// The draft's `.tacinstr` grid: the headers Attack and Defence
// left-aligned over their columns, each instruction's two combos on one
// y and equal in width, and each target chip its combo's own x and
// width, 3 under it (the draft's `column-gap: 8px; row-gap: 3px`).
#[test]
fn the_instructions_grid_is_two_equal_columns() {
    // The settings column's width as the window's: the draft's
    // 422 panel holds a 408-wide content.
    let mut window = Headless::sized(422.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let bounds = |node: &egui::accesskit::Node| node.bounds().expect("bounds");

    for row in 0..2 {
        let [attack, defence] = combo_row(&nodes, row);
        let (a, d) = (bounds(attack), bounds(defence));
        assert!(
            (a.y0 - d.y0).abs() < 0.5,
            "row {row}'s combos share a y ({a:?} vs {d:?})"
        );
        assert!(
            ((a.x1 - a.x0) - (d.x1 - d.x0)).abs() <= 0.5,
            "row {row}'s columns are equal ({a:?} vs {d:?})"
        );
        for (column, combo) in [(0, attack), (1, defence)] {
            let chip = chip_under(&nodes, combo);
            let (c, k) = (bounds(combo), bounds(chip));
            assert!(
                (k.x0 - c.x0).abs() <= 0.5,
                "row {row} column {column}: the chip starts at its combo's x ({k:?} vs {c:?})"
            );
            assert!(
                ((k.x1 - k.x0) - (c.x1 - c.x0)).abs() <= 0.5,
                "row {row} column {column}: the chip is its combo's width ({k:?} vs {c:?})"
            );
            assert!(
                (k.y0 - c.y1 - 3.0).abs() <= 0.5,
                "row {row} column {column}: the chip sits 3 under its combo ({k:?} vs {c:?})"
            );
        }
    }
    // The headers are plain left-aligned cells at their columns' x.
    for (header, combo) in [
        ("Attack", combo_row(&nodes, 0)[0]),
        ("Defence", combo_row(&nodes, 0)[1]),
    ] {
        let header_bounds = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(header))
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no {header} header"));
        assert!(
            (header_bounds.x0 - bounds(combo).x0).abs() <= 0.5,
            "{header} starts at its column's x ({header_bounds:?} vs {:?})",
            bounds(combo)
        );
    }
}

// The disabled target, as the draft's `pick` draws it: under an
// instruction that takes none of ours the chip is
// disabled and names no player of ours — "An opponent" under a
// held Tight Marking (its byte is the opposing formation's slot),
// "No target" under a target-less one.
#[test]
fn a_target_less_or_opponent_target_shows_no_player_of_ours() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let attack = tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions");
    attack[0].instruction =
        instruction::encode(version, Instruction::TightMarking).expect("stored");
    attack[0].player = 4;
    attack[1].instruction = instruction::encode(version, Instruction::TikiTaka).expect("stored");
    attack[1].player = 0;
    let defence = tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 has instructions");
    defence[0].instruction =
        instruction::encode(version, Instruction::CounterTarget).expect("stored");
    defence[0].player = 0xFF;
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);

    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let combos = |index| combo_row(&nodes, index);
    let chip1 = chip_under(&nodes, combos(0)[0]);
    assert!(chip1.is_disabled(), "the Tight Marking chip is disabled");
    assert_eq!(
        chip1.label(),
        Some("An opponent"),
        "a held Tight Marking's byte is the opponent's"
    );
    let chip2 = chip_under(&nodes, combos(1)[0]);
    assert!(chip2.is_disabled(), "the target-less chip is disabled");
    assert_eq!(
        chip2.label(),
        Some("No target"),
        "a target-less instruction names none"
    );
    // An unset target under a player-taking instruction reads
    // "Unassigned" (no "(empty)" name).
    let chip3 = chip_under(&nodes, combos(0)[1]);
    assert!(!chip3.is_disabled(), "the Counter Target chip is enabled");
    assert_eq!(
        chip3.label(),
        Some("Unassigned"),
        "an unset target under a player-taking instruction names none"
    );
    for chip in [chip1, chip2] {
        let shown = chip.label().expect("the chip's text");
        assert!(
            roster
                .iter()
                .all(|row| row.name.is_empty() || !shown.ends_with(row.name)),
            "{shown}: no roster name appears in a disabled chip"
        );
    }
}

// The same ruling's kept half: a save already holding Tight Marking
// shows it as the row's value, and drawing keeps it.
#[test]
fn a_stored_tight_marking_shows_its_label_and_stays() {
    let mut window = Headless::sized(800.0, 900.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction = instruction::encode(PesVersion::Pes21, Instruction::TightMarking)
        .expect("PES 21 stores Tight Marking");
    let before = tactics.clone();
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);

    let (nodes, changed) =
        window.tactics(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[0];
    assert_eq!(
        combo.value(),
        Some("Tight Marking"),
        "the stored Tight Marking shows as the combo's text"
    );
    assert!(!changed, "the frame reports no change");
    let (_, changed) = window.tactics(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    assert!(!changed, "an idle frame reports no change");
    assert_eq!(tactics, before, "and leaves the record as stored");
}

// A stored byte the version's table does not list shows its hex and
// is kept until a pick.
#[test]
fn an_unknown_stored_instruction_shows_its_hex() {
    let mut window = Headless::sized(800.0, 900.0);
    let mut tactics = fixture_tactics(PesVersion::Pes21);
    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction = 0xAB;
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes21);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(PesVersion::Pes21, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[0];
    assert_eq!(combo.value(), Some("Unknown (0xAB)"));
}

// The pick itself: an instruction's combo writes the byte, and one
// that takes no target clears the pick to 255, its chip disabled.
// The chip under a player-taking one arms the target mode
// (`Assign::Target`) — the pitch pick's write is the popover
// tests'.
#[test]
fn the_instruction_pick_writes_the_byte_and_clears_the_target() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // Every chip shows a name: the counted buttons are the popup's.
    named_assignments(&mut tactics);
    // A non-identity eleven: an arm writing the position index for
    // the roster slot would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(2, 9);
    let picked = tactics.starting_eleven[2];
    assert_ne!(picked, 2, "premise: slot and index differ");
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[0];

    // Open the instruction combo and pick Defensive (an attacking
    // instruction that takes a player — Counter Target is
    // defending, and the attack combo does not offer it).
    let (_, _) = click(
        &mut window,
        combo,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
    let item = entry_id(&ids, Role::Button, "Defensive");
    let (_, changed) = window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
    assert!(changed, "the pick changed the record");
    let attack = &mut tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("instructions")[0];
    assert_eq!(
        attack.instruction,
        instruction::encode(version, Instruction::Defensive).expect("PES 21 stores Defensive"),
        "the pick writes the stored value"
    );

    // The target chip under it is enabled and arms the mode. The
    // popup stays open past an AccessKit pick (no `any_click`), so a
    // pointer click on empty space closes it first.
    window.tactics(
        version,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(egui::pos2(5.0, 5.0), true),
    );
    window.tactics(
        version,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(egui::pos2(5.0, 5.0), false),
    );
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    assert!(!chip.is_disabled(), "Defensive enables the chip");
    let (_, changed) = click(&mut window, chip, version, &mut tactics, &mut view, &roster);
    assert!(!changed, "arming writes no record");
    assert_eq!(
        view.assign,
        Some(Assign::Target {
            side: InstructionSide::Attack,
            index: 0,
        }),
        "the mode is armed"
    );

    // Pick Tiki-Taka (attacking, no target): the byte changes, the
    // target clears to 255 and its chip disables.
    view.assign = None;
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[0];
    let (_, _) = click(
        &mut window,
        combo,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
    let item = entry_id(&ids, Role::Button, "Tiki-Taka");
    let (_, changed) = window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
    assert!(changed, "the pick changed the record");
    let attack = &tactics.presets[0]
        .attack_instructions
        .as_ref()
        .expect("instructions")[0];
    assert_eq!(
        attack.instruction,
        instruction::encode(version, Instruction::TikiTaka).expect("PES 21 stores Tiki-Taka")
    );
    assert_eq!(
        attack.player, NO_TARGET,
        "a target-less pick clears the target"
    );
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    assert!(chip.is_disabled(), "Tiki-Taka disables the chip");
    assert_eq!(chip.label(), Some("No target"), "and it shows No target");
}

/// Preset 1's first `side` instruction.
fn first_instruction(tactics: &mut TeamTactics, side: InstructionSide) -> &mut AdvancedInstruction {
    let preset = &mut tactics.presets[0];
    let column = match side {
        InstructionSide::Attack => &mut preset.attack_instructions,
        InstructionSide::Defence => &mut preset.defence_instructions,
    };
    &mut column.as_mut().expect("PES 21 has instructions")[0]
}

// A target stays only between two instructions that take one of ours: a
// held Tight Marking's byte is the opposing formation's slot, so Counter
// Target picked over it (in the defence column) clears the byte, as
// Defensive picked over a byte the table does not list does, while
// Defensive picked over a held Counter Target (in the attack column,
// which lists it as the current choice) keeps its player.
#[test]
fn a_pick_keeps_the_target_only_between_two_that_take_one_of_ours() {
    let version = PesVersion::Pes21;
    let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    for (side, held, picked, kept) in [
        (
            InstructionSide::Defence,
            encode(Instruction::TightMarking),
            Instruction::CounterTarget,
            NO_TARGET,
        ),
        (
            InstructionSide::Attack,
            encode(Instruction::CounterTarget),
            Instruction::Defensive,
            4,
        ),
        (
            InstructionSide::Attack,
            0xAB,
            Instruction::Defensive,
            NO_TARGET,
        ),
    ] {
        // A window per case: a popup a pick left open would take the next
        // case's click on the same combo as a close.
        let mut window = Headless::sized(800.0, 1200.0);
        let mut tactics = fixture_tactics(version);
        let entry = first_instruction(&mut tactics, side);
        entry.instruction = held;
        entry.player = 4;
        let mut view = TacticsView::default();
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let combo = combo_row(&nodes, 0)[match side {
            InstructionSide::Attack => 0,
            InstructionSide::Defence => 1,
        }];
        click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
        let item = entry_id(&ids, Role::Button, instruction_label(picked));
        let (_, changed) =
            window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
        assert!(
            changed,
            "{held:#04x} to {picked:?}: the pick changed the record"
        );
        let entry = *first_instruction(&mut tactics, side);
        assert_eq!(
            entry.instruction,
            encode(picked),
            "premise: {picked:?} picked"
        );
        assert_eq!(
            entry.player, kept,
            "{held:#04x} to {picked:?}: the target byte"
        );
    }
}

// The four rows route to their own entries: an instruction and a
// target picked on one row lands on that row's entry alone — the
// other three rows, the other presets and every other field stay.
#[test]
fn each_instruction_row_writes_its_own_entry() {
    let mut window = Headless::sized(800.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // The four rows start distinct and a non-identity eleven (a
    // roster slot that equals its index would hide an index write).
    let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
    let rows_init = [
        Instruction::HugTheTouchline,
        Instruction::WingRotation,
        Instruction::DeepDefensiveLine,
        Instruction::Gegenpress,
    ];
    let attack = &mut tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions");
    attack[0].instruction = encode(rows_init[0]);
    attack[0].player = 0;
    attack[1].instruction = encode(rows_init[1]);
    attack[1].player = 1;
    let defence = &mut tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 has instructions");
    defence[0].instruction = encode(rows_init[2]);
    defence[0].player = 2;
    defence[1].instruction = encode(rows_init[3]);
    defence[1].player = 3;
    tactics.starting_eleven.swap(2, 9);
    tactics.starting_eleven.swap(3, 10);
    tactics.starting_eleven.swap(4, 7);
    tactics.starting_eleven.swap(5, 8);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);

    for (index, (column, side, inst, pick)) in [
        (
            0usize,
            InstructionSide::Attack,
            Instruction::Defensive,
            2_usize,
        ),
        (0, InstructionSide::Attack, Instruction::Defensive, 3_usize),
        (
            1,
            InstructionSide::Defence,
            Instruction::CounterTarget,
            4_usize,
        ),
        (
            1,
            InstructionSide::Defence,
            Instruction::CounterTarget,
            5_usize,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let row = index % 2;
        assert_ne!(
            tactics.starting_eleven[pick], pick as u8,
            "premise: slot and index differ"
        );
        let before = tactics.clone();
        let mut expected = before.clone();
        let entries = if index < 2 {
            &mut expected.presets[0]
                .attack_instructions
                .as_mut()
                .expect("instructions")
        } else {
            &mut expected.presets[0]
                .defence_instructions
                .as_mut()
                .expect("instructions")
        };
        let entry = &mut entries[row];
        entry.instruction = encode(inst);
        entry.player = tactics.starting_eleven[pick];
        assert!(
            inst.takes_player(),
            "premise: the picked instruction takes a player"
        );

        // Pick the instruction on this row's combo.
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let combo = combo_row(&nodes, row)[column];
        let (_, _) = click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
        let item = entry_id(&ids, Role::Button, instruction_label(inst));
        let (_, changed) =
            window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
        assert!(changed, "{side:?} {row}: the pick changed the record");

        // Then the chip's pick: arm it, the pitch's write puts the
        // starter's roster slot there and disarms. The popup stays
        // open past an AccessKit pick — a pointer click on empty
        // space closes it first.
        window.tactics(
            version,
            &mut tactics,
            &mut view,
            &roster,
            pointer_at(egui::pos2(5.0, 5.0), true),
        );
        window.tactics(
            version,
            &mut tactics,
            &mut view,
            &roster,
            pointer_at(egui::pos2(5.0, 5.0), false),
        );
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let chip = chip_under(&nodes, combo_row(&nodes, row)[column]);
        assert!(!chip.is_disabled(), "{side:?} {row}: the chip is enabled");
        let (_, changed) = click(&mut window, chip, version, &mut tactics, &mut view, &roster);
        assert!(!changed, "{side:?} {row}: arming writes no record");
        // The chip armed its own row, and the write goes through the
        // armed mode: two chips arming one row would fail here.
        assert_eq!(
            view.assign,
            Some(Assign::Target { side, index: row }),
            "{side:?} {row}: the chip armed its own row"
        );
        let armed = view.assign.expect("the chip armed a mode");
        armed.write(version, &mut tactics, &mut view, pick);
        assert_eq!(
            view.assign, None,
            "{side:?} {row}: disarmed after the write"
        );

        assert_eq!(
            tactics, expected,
            "{side:?} {row}: only that row's entry changed"
        );
    }
}

// The panel writes the shown preset, not a fixed one:
// an instruction and its target picked on preset 2 land in preset 2's
// entry alone, preset 1 and every other field unchanged.
#[test]
fn an_instruction_pick_lands_in_the_shown_preset() {
    let mut window = Headless::sized(800.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // A non-identity eleven (a roster slot that equals its index would
    // hide an index write).
    tactics.starting_eleven.swap(2, 9);
    let pick = 2;
    assert_ne!(
        tactics.starting_eleven[pick], pick as u8,
        "premise: slot and index differ"
    );
    let mut view = TacticsView {
        preset: 1,
        ..TacticsView::default()
    };
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let before = tactics.clone();
    let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
    let entry = before.presets[1]
        .attack_instructions
        .as_ref()
        .expect("PES 21 has instructions")[0];
    assert_ne!(
        entry.instruction,
        encode(Instruction::Defensive),
        "premise: a real instruction change"
    );
    assert_ne!(
        entry.player, tactics.starting_eleven[pick],
        "premise: a real target change"
    );

    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let combo = combo_row(&nodes, 0)[0];
    click(
        &mut window,
        combo,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
    let item = entry_id(&ids, Role::Button, "Defensive");
    window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
    // The popup stays open past an AccessKit pick: a pointer click on
    // empty space closes it first.
    window.tactics(
        version,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(egui::pos2(5.0, 5.0), true),
    );
    window.tactics(
        version,
        &mut tactics,
        &mut view,
        &roster,
        pointer_at(egui::pos2(5.0, 5.0), false),
    );
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    click(&mut window, chip, version, &mut tactics, &mut view, &roster);
    let armed = view.assign.expect("the chip armed a mode");
    armed.write(version, &mut tactics, &mut view, pick);

    let mut expected = before;
    let entry = &mut expected.presets[1]
        .attack_instructions
        .as_mut()
        .expect("instructions")[0];
    entry.instruction = encode(Instruction::Defensive);
    entry.player = tactics.starting_eleven[pick];
    assert_eq!(
        tactics, expected,
        "only preset 2's first attack entry changed"
    );
}

// The Target chip's text is the name alone (the draft's chip reads
// it): a starter's, a bench player's, "(empty)" for a roster slot
// that holds nobody.
#[test]
fn the_target_chip_shows_the_name_alone() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // A non-identity eleven: the byte is a roster slot, so an impl
    // showing it (or its position in the eleven read as it) for the
    // lineup index would look right on the fixture's [0, 1, ...].
    tactics.starting_eleven.swap(3, 9);
    let starter = tactics.starting_eleven[3];
    assert_ne!(starter, 3, "premise: slot and index differ");
    let bench = tactics.bench_order[0];
    assert_ne!(starter, bench, "premise: two different slots");
    let attack = &mut tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0];
    attack.instruction = instruction::encode(version, Instruction::CounterTarget).expect("stored");
    attack.player = starter;
    let mut view = TacticsView::default();
    // A name per slot: the fixture's are all "PLACEHOLDER" here.
    let rows: Vec<(String, String, Option<u8>)> = (0..32)
        .map(|slot| (format!("Player {slot}"), format!("Player {slot}"), Some(0)))
        .collect();
    let roster = roster_rows(&rows);

    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    let expected = format!("Player {starter}");
    assert_eq!(
        chip.label(),
        Some(expected.as_str()),
        "a starter shows his name alone"
    );

    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .player = bench;
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    let expected = format!("Player {bench}");
    assert_eq!(
        chip.label(),
        Some(expected.as_str()),
        "a bench player shows the name alone too"
    );
}

// `roster` shorter than a slot, or an empty one, shows "(empty)" as
// the chip's text under a player-taking instruction.
#[test]
fn a_short_or_empty_roster_slice_shows_empty_on_the_chip() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction = instruction::encode(version, Instruction::CounterTarget).expect("stored");
    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("instructions")[0]
        .player = tactics.starting_eleven[2];
    let mut view = TacticsView::default();
    // Three rows for forty roster slots: slot 2, an empty name,
    // shows "(empty)".
    let rows = [
        ("ALPHA".to_owned(), "ALPHA".to_owned(), Some(0)),
        ("BETA".to_owned(), "BETA".to_owned(), Some(0)),
        (String::new(), String::new(), None),
    ];
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    assert_eq!(chip.label(), Some("(empty)"), "an empty name shows (empty)");

    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("instructions")[0]
        .player = 40;
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    assert_eq!(
        chip.label(),
        Some("(empty)"),
        "a slot past the slice shows (empty)"
    );
}

// The chip's own elide: a selected name too wide for its column
// truncates, and the closed chip's hover shows it whole (the
// `rolechip`'s `title` is the pick's hint, so a name that does not
// fit is the hover instead). The target cell is the grid's column,
// ~183 wide at this pane; a real player's name can be wider.
#[test]
fn an_elided_target_shows_the_whole_name_on_hover() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // Counter Target takes a player; the ninth starter's name,
    // made wider than the chip's column, elides.
    let ninth = tactics.starting_eleven[8];
    let attack = &mut tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0];
    attack.instruction = instruction::encode(version, Instruction::CounterTarget).expect("stored");
    attack.player = ninth;
    let mut view = TacticsView::default();
    let selected = "3DSMAX HAS STOPPED WORKING".to_owned();
    let rows: Vec<(String, String, Option<u8>)> = (0..32)
        .map(|slot| {
            let name = if slot == usize::from(ninth) {
                "3DSMAX HAS STOPPED WORKING".to_owned()
            } else {
                format!("Player {slot}")
            };
            (name.clone(), name, Some(0))
        })
        .collect();
    let roster = roster_rows(&rows);
    // The settings column's width as the window's (the real ~196
    // half at 422): the name is wider.
    let mut window = Headless::sized(422.0, 1200.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);

    let mut measured = 0.0f32;
    window.frame(vec![], |ui| measured = text_width(ui, &selected));
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    let width = chip
        .bounds()
        .map(|b| b.x1 - b.x0)
        .expect("the chip's bounds");
    assert!(
        f64::from(measured) > width - 12.0,
        "premise: {selected:?} elides inside the chip ({measured} vs {width})"
    );
    assert_eq!(
        window.tactics_tooltip(
            centre(chip),
            &selected,
            version,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some(selected.clone()),
        "the pointer resting on the chip shows the whole name"
    );
}

// The check's other side: a name that fits the chip's text area —
// its hover is the pick's hint, the draft's title, not the name.
#[test]
fn a_fitting_target_hovers_the_pick_hint() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let attack = &mut tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0];
    attack.instruction = instruction::encode(version, Instruction::CounterTarget).expect("stored");
    attack.player = 30;
    tactics.starting_eleven[0] = 30;
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    // Premise: the fixture leaves roster slot 30 without a player.
    assert_eq!(name_at(&roster, 30), "(empty)");
    let mut window = Headless::sized(800.0, 1200.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);

    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip = chip_under(&nodes, combo_row(&nodes, 0)[0]);
    assert_eq!(chip.label(), Some("(empty)"), "the chip's text");
    assert_eq!(
        window.tactics_tooltip(
            centre(chip),
            "The instruction's target",
            version,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("The instruction's target: click, then a pitch player".to_owned()),
        "a chip that fits hovers the pick's hint"
    );
}
