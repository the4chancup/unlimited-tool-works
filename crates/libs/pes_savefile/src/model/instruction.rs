//! The canonical advanced-instruction enum: the sixteen values PES 18 and 19
//! store, and Anchoring, which PES 20 and 21 add. The stored value is a
//! per-version index (PES 15/16 have no
//! instruction table at all); `schema::instruction` maps it, this enum
//! carries no version knowledge.

use crate::schema::fields::InstructionSide;

/// One advanced instruction, in the canonical order (PES 17's stored order for
/// the first thirteen; the PES 18 additions at the end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// No instruction.
    Off,
    /// Hug the Touchline.
    HugTheTouchline,
    /// False No. 9.
    FalseNo9,
    /// False Full-Backs.
    FalseFullBacks,
    /// Attacking Full-Backs.
    AttackingFullBacks,
    /// Wing Rotation.
    WingRotation,
    /// Tiki-Taka.
    TikiTaka,
    /// Centring Targets.
    CenteringTargets,
    /// Swarm the Box.
    SwarmTheBox,
    /// Deep Defensive Line.
    DeepDefensiveLine,
    /// Gegenpress.
    Gegenpress,
    /// Tight Marking.
    TightMarking,
    /// Counter Target.
    CounterTarget,
    /// Defensive (PES 18+).
    Defensive,
    /// False Winger (PES 18+).
    FalseWinger,
    /// Wing-Back (PES 18+).
    WingBack,
    /// Anchoring (PES 20/21 only).
    Anchoring,
}

impl Instruction {
    /// Every value, in the canonical order.
    pub const ALL: [Instruction; 17] = [
        Instruction::Off,
        Instruction::HugTheTouchline,
        Instruction::FalseNo9,
        Instruction::FalseFullBacks,
        Instruction::AttackingFullBacks,
        Instruction::WingRotation,
        Instruction::TikiTaka,
        Instruction::CenteringTargets,
        Instruction::SwarmTheBox,
        Instruction::DeepDefensiveLine,
        Instruction::Gegenpress,
        Instruction::TightMarking,
        Instruction::CounterTarget,
        Instruction::Defensive,
        Instruction::FalseWinger,
        Instruction::WingBack,
        Instruction::Anchoring,
    ];

    /// Whether the instruction is given to one of the team's own players,
    /// picked with it: the instructions whose target-player field applies.
    pub fn takes_player(self) -> bool {
        matches!(
            self,
            Instruction::CounterTarget | Instruction::Defensive | Instruction::Anchoring
        )
    }

    /// The side the instruction plays for in the game's Game Plan: the
    /// defending ones are Swarm the Box, Deep Defensive Line, Gegenpress,
    /// Tight Marking, Counter Target and Wing-Back; every other one is an
    /// attacking one (Defensive and Anchoring included). `None` for
    /// `Off`, which is no instruction.
    pub fn side(self) -> Option<InstructionSide> {
        match self {
            Instruction::Off => None,
            Instruction::SwarmTheBox
            | Instruction::DeepDefensiveLine
            | Instruction::Gegenpress
            | Instruction::TightMarking
            | Instruction::CounterTarget
            | Instruction::WingBack => Some(InstructionSide::Defence),
            Instruction::HugTheTouchline
            | Instruction::FalseNo9
            | Instruction::FalseFullBacks
            | Instruction::AttackingFullBacks
            | Instruction::WingRotation
            | Instruction::TikiTaka
            | Instruction::CenteringTargets
            | Instruction::Defensive
            | Instruction::FalseWinger
            | Instruction::Anchoring => Some(InstructionSide::Attack),
        }
    }
}

impl Default for Instruction {
    /// The empty instruction (`Off` is the game's own "unset" value).
    fn default() -> Self {
        Instruction::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The Tactics editor enables the target-player pick only for the
    // instructions that take one of ours, Counter Target, Defensive and
    // Anchoring. This is the one place the rule lives.
    #[test]
    fn only_counter_target_defensive_and_anchoring_take_a_player() {
        for instruction in Instruction::ALL {
            assert_eq!(
                instruction.takes_player(),
                matches!(
                    instruction,
                    Instruction::CounterTarget | Instruction::Defensive | Instruction::Anchoring
                ),
                "{instruction:?}"
            );
        }
    }

    // The rule above, against the saves: in every fixture tactics record, a
    // slot whose instruction takes a player names a filled roster slot, and
    // every other slot holds 0 or 0xFF.
    #[test]
    fn only_the_instructions_that_take_a_player_carry_one_in_the_fixtures() {
        let mut targeted = 0usize;
        for version in crate::test_support::FIXTURES {
            let (file, _) = crate::test_support::open(version);
            for team in file.teams() {
                for preset in &team.tactics.presets {
                    let sides = [&preset.attack_instructions, &preset.defence_instructions];
                    for entry in sides.into_iter().flatten().flatten() {
                        let instruction =
                            crate::schema::instruction::decode(version, entry.instruction)
                                .expect("decodes")
                                .expect("a version with instructions decodes to Some");
                        let context = format!(
                            "{version:?} team {} {instruction:?} player {}",
                            team.id, entry.player
                        );
                        if instruction.takes_player() {
                            let slot = team.roster.get(usize::from(entry.player));
                            assert!(
                                slot.is_some_and(|slot| slot.player_id != 0),
                                "{context}: not a filled roster slot"
                            );
                            targeted += 1;
                        } else {
                            assert!(
                                entry.player == 0 || entry.player == 0xFF,
                                "{context}: a target on an instruction that takes none"
                            );
                        }
                    }
                }
            }
        }
        assert!(targeted >= 100, "only {targeted} targeted slots");
    }

    // The same bullet: an Attack combo lists the attacking instructions,
    // a Defence combo the defending ones — the game's own split (the
    // Rigged Wiki's Tactical pages, Konami's PES 2019 manual counting 9
    // and 6): the defending six are Swarm the Box, Deep Defensive Line,
    // Gegenpress, Tight Marking, Counter Target and Wing-Back.
    #[test]
    fn the_side_of_each_value() {
        let defence = InstructionSide::Defence;
        let attack = InstructionSide::Attack;
        let cases: &[(Instruction, Option<InstructionSide>)] = &[
            (Instruction::Off, None),
            (Instruction::HugTheTouchline, Some(attack)),
            (Instruction::FalseNo9, Some(attack)),
            (Instruction::FalseFullBacks, Some(attack)),
            (Instruction::AttackingFullBacks, Some(attack)),
            (Instruction::WingRotation, Some(attack)),
            (Instruction::TikiTaka, Some(attack)),
            (Instruction::CenteringTargets, Some(attack)),
            (Instruction::SwarmTheBox, Some(defence)),
            (Instruction::DeepDefensiveLine, Some(defence)),
            (Instruction::Gegenpress, Some(defence)),
            (Instruction::TightMarking, Some(defence)),
            (Instruction::CounterTarget, Some(defence)),
            (Instruction::Defensive, Some(attack)),
            (Instruction::FalseWinger, Some(attack)),
            (Instruction::WingBack, Some(defence)),
            (Instruction::Anchoring, Some(attack)),
        ];
        assert_eq!(cases.len(), Instruction::ALL.len(), "a case per value");
        for (instruction, expected) in cases {
            assert_eq!(instruction.side(), *expected, "{instruction:?}");
        }
    }
}
