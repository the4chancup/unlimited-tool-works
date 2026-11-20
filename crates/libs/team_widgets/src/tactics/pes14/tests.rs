//! The `pes14` module's tests: the formation list's matching and writes
//! and the Strategy picks' slider writes, on synthetic formations and
//! game plans.

use egui::accesskit::Role;
use pes_savefile::EditFile;
use pes_version::PesVersion;

use super::*;
use crate::test_support::{Headless, centre, control_below};

/// `entry`'s formation as a `Formation`, `keeper` its slot 0.
fn listed(entry: usize, keeper: FormationSlot) -> Formation {
    let mut players = [keeper; 11];
    for (slot, &(position, x, y)) in players[1..].iter_mut().zip(&LIST_FORMATIONS[entry].1) {
        *slot = FormationSlot { position, x, y };
    }
    Formation { players }
}

// Every list entry is found at its own index; a formation off one slot's
// x by 1 is none of them.
#[test]
fn listed_formation_matches_each_entry_and_no_other() {
    let keeper = FormationSlot {
        position: 0,
        x: 52,
        y: 4,
    };
    for (entry, _) in LIST_FORMATIONS.iter().enumerate() {
        assert_eq!(
            listed_formation(&listed(entry, keeper)),
            Some(entry),
            "entry {entry} is found at its index"
        );
    }
    let mut off = listed(0, keeper);
    off.players[1].x += 1;
    assert_eq!(
        listed_formation(&off),
        None,
        "one slot's x off by 1 matches no entry"
    );
}

// A pick writes the entry's ten outfield slots and puts the keeper at
// depth 3, his width and position kept.
#[test]
fn pick_formation_writes_the_entry_and_the_keeper_depth() {
    let mut formation = Formation {
        players: [FormationSlot {
            position: 9,
            x: 40,
            y: 40,
        }; 11],
    };
    formation.players[0] = FormationSlot {
        position: 0,
        x: 52,
        y: 4,
    };
    let four_three_three = 6;
    pick_formation(&mut formation, four_three_three);
    assert_eq!(
        formation.players[1..]
            .iter()
            .map(|slot| (slot.position, slot.x, slot.y))
            .collect::<Vec<_>>(),
        vec![
            (1, 63, 10),
            (1, 41, 10),
            (3, 89, 12),
            (2, 15, 12),
            (5, 52, 24),
            (5, 78, 27),
            (5, 20, 27),
            (10, 81, 42),
            (9, 23, 42),
            (12, 52, 43),
        ],
        "the outfield slots are 4-3-3's"
    );
    assert_eq!(
        formation.players[0],
        FormationSlot {
            position: 0,
            x: 52,
            y: 3
        },
        "the keeper keeps his width and position, depth 3"
    );
}

// A strategy pick writes its index and the sliders its row sets: Quick
// Counter's, Long Ball's (Defensive Compactness and Shifting to Attack
// alone), Custom's none.
#[test]
fn pick_strategy_writes_the_strategy_and_its_sliders() {
    let sliders = [13, 8, 9, 9, 8, 12, 10, 10];
    let mut plan = GamePlan {
        strategy: 1,
        sliders,
    };
    pick_strategy(&mut plan, 2);
    assert_eq!(
        plan,
        GamePlan {
            strategy: 2,
            sliders: [10, 10, 9, 15, 15, 15, 15, 10]
        },
        "Quick Counter writes its sliders and keeps Position Switch"
    );
    let mut plan = GamePlan {
        strategy: 1,
        sliders,
    };
    pick_strategy(&mut plan, 1);
    assert_eq!(
        plan,
        GamePlan {
            strategy: 1,
            sliders: [13, 8, 9, 9, 8, 5, 15, 10]
        },
        "Long Ball sets Defensive Compactness and Shifting to Attack alone"
    );
    let mut plan = GamePlan {
        strategy: 1,
        sliders,
    };
    pick_strategy(&mut plan, 0);
    assert_eq!(
        plan,
        GamePlan {
            strategy: 0,
            sliders
        },
        "Custom sets no slider"
    );
}

/// Three Combination Play slots, each `(area, strategy)`.
fn slots(entries: [(u8, u8); 3]) -> [CombinationSlot; 3] {
    entries.map(|(area, strategy)| CombinationSlot { area, strategy })
}

// A strategy's free areas are its own less those the other two slots
// hold. An empty slot holds nothing, its area 0 or a stale byte alike,
// and the slot asked about does not hold its own area. Cut Inside takes
// 3 and 5, Overlap and Pull Back 4, 5, 7 and 10.
#[test]
fn free_areas_leave_out_the_other_slots_areas() {
    let cut_inside = 3;
    let overlap_and_pull_back = 8;
    for (entries, index, id, expected) in [
        ([(0, 0), (3, 1), (5, 2)], 0, cut_inside, vec![]),
        ([(0, 0), (3, 1), (0, 0)], 0, cut_inside, vec![5]),
        ([(0, 0), (3, 1), (5, 0)], 0, cut_inside, vec![5]),
        ([(3, 3), (6, 1), (0, 0)], 0, cut_inside, vec![3, 5]),
        (
            [(7, 9), (0, 0), (4, 8)],
            1,
            overlap_and_pull_back,
            vec![5, 10],
        ),
        ([(0, 0), (0, 0), (0, 0)], 0, 15, vec![]),
    ] {
        assert_eq!(
            free_areas(&slots(entries), index, id),
            expected,
            "strategy {id} in slot {index} of {entries:?}"
        );
    }
}

// A pick keeps the slot's area when it is free, else moves to the area
// the game's picker opens on, else to the first free one: Overlap and
// Pull Back opens on 7, and with 7 held lands on 4; Hold up the ball in
// area 5 becoming Cut Inside stays in 5, not Cut Inside's opening 3.
#[test]
fn pick_combination_keeps_a_free_area_or_moves_to_the_opening_one() {
    let cut_inside = 3;
    let overlap_and_pull_back = 8;
    for (entries, index, id, expected) in [
        (
            [(0, 0), (0, 0), (0, 0)],
            0,
            overlap_and_pull_back,
            [(7, 8), (0, 0), (0, 0)],
        ),
        (
            [(0, 0), (7, 9), (0, 0)],
            0,
            overlap_and_pull_back,
            [(4, 8), (7, 9), (0, 0)],
        ),
        (
            [(5, 1), (0, 0), (0, 0)],
            0,
            cut_inside,
            [(5, 3), (0, 0), (0, 0)],
        ),
    ] {
        let mut picked = slots(entries);
        pick_combination(&mut picked, index, id);
        assert_eq!(
            picked,
            slots(expected),
            "strategy {id} into slot {index} of {entries:?}"
        );
    }
}

// Clear stores strategy 0 and area 0 in its slot alone.
#[test]
fn clear_combination_empties_its_slot_alone() {
    let mut cleared = slots([(2, 10), (6, 1), (8, 5)]);
    clear_combination(&mut cleared, 1);
    assert_eq!(cleared, slots([(2, 10), (0, 0), (8, 5)]));
}

// A wheel notch over a strategy combo steps to the next pickable entry,
// skipping a strategy the other slots' areas leave no room for: Cut Inside
// (areas 3 and 5, both held) is greyed in slot 1's list, which runs
// Diagonal Run, Cut Inside, Diagonal Dribble — a notch toward the list's
// end lands on Diagonal Dribble with a free area, the other slots
// untouched. Without the skip the pick would reach `pick_combination`'s
// `expect` and panic.
#[test]
fn the_wheel_skips_a_strategy_with_no_free_area() {
    let mut window = Headless::new();
    let file =
        EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes14)).expect("fixture save");
    let mut tactics = file
        .team(1)
        .expect("team 1 is in the fixture")
        .tactics
        .clone();
    tactics.combination = Some(slots([(4, 2), (5, 1), (3, 5)]));
    let held = tactics.combination.expect("the slots just set");
    assert!(
        free_areas(&held, 0, 3).is_empty(),
        "premise: Cut Inside has no free area for slot 1"
    );

    // Slot 1's strategy combo is the one showing "Diagonal Run".
    let (nodes, _) = window.frame(vec![], |ui| {
        combination_play(ui, &mut tactics, None);
    });
    let combo = nodes
        .iter()
        .find(|node| node.role() == Role::ComboBox && node.value() == Some("Diagonal Run"))
        .expect("slot 1's strategy combo");
    let notch = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let events = vec![egui::Event::PointerMoved(centre(combo)), notch];
    window.frame(events, |ui| {
        combination_play(ui, &mut tactics, None);
    });
    assert_eq!(
        tactics.combination,
        Some(slots([(4, 4), (5, 1), (3, 5)])),
        "slot 1 holds Diagonal Dribble in a free area, the rest untouched"
    );
}

// The two dropdowns stand `CELL_GAP` under their labels, as the draft's
// selects do: the Formation combo's top is the "Formation" label's
// bottom plus it, and the Strategy combo's likewise.
#[test]
fn the_dropdowns_stand_cell_gap_under_their_labels() {
    let mut window = Headless::new();
    let file =
        EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes14)).expect("fixture save");
    let mut tactics = file
        .team(1)
        .expect("team 1 is in the fixture")
        .tactics
        .clone();
    let view = TacticsView::default();
    let (nodes, _) = window.frame(vec![], |ui| {
        preset_tactics(ui, &mut tactics, &view, None);
    });
    for label in ["Formation", STRATEGY_LABEL] {
        let bounds = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(label))
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no label {label:?}"));
        let top = control_below(&nodes, label, Role::ComboBox, 100.0)
            .bounds()
            .expect("the combo's bounds");
        assert!(
            (top.y0 - (bounds.y1 + f64::from(CELL_GAP))).abs() <= 0.5,
            "{label}: the combo's top {} is the label's bottom {} plus {CELL_GAP}",
            top.y0,
            bounds.y1,
        );
    }
}
