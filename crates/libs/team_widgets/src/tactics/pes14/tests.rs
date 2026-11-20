//! The `pes14` module's tests: the formation list's matching and writes
//! and the Strategy picks' slider writes, on synthetic formations and
//! game plans.

use super::*;

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
