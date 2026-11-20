//! The per-version advanced-instruction mappings (plan:
//! `spec/pes_savefile/model.md`, "Version quirks"). PES 15/16 have no instruction
//! table (`decode` is `Ok(None)` for every value, `encode` `None`). PES 17's
//! stored order IS the canonical order for the thirteen it has; PES 18/19
//! keep 0x00-0x07 and permute the rest into 0x08-0x0F; PES 20/21 insert
//! Anchoring at 0x0A, 18/19's 0x0A-0x0F moving to 0x0B-0x10.

use pes_version::PesVersion;

use crate::codec::CodecError;
use crate::model::instruction::Instruction;

use Instruction as I;

/// PES 18/19: the stored value indexes this list (the table ends at 0x0F).
const STORED_18: [Instruction; 16] = [
    I::Off,
    I::HugTheTouchline,
    I::FalseNo9,
    I::FalseFullBacks,
    I::AttackingFullBacks,
    I::WingRotation,
    I::TikiTaka,
    I::CenteringTargets,
    I::Defensive,
    I::FalseWinger,
    I::SwarmTheBox,
    I::DeepDefensiveLine,
    I::Gegenpress,
    I::TightMarking,
    I::CounterTarget,
    I::WingBack,
];

/// PES 20/21: the 18/19 list with `Anchoring`, the instruction PES 2020
/// added, inserted at 0x0A, the 18/19 entries from SwarmTheBox on shifted
/// up one (WingBack to 0x10). Measured in-game on PES 21 (Edit > Teams >
/// Game Plan); fits every 20/21 save's attack and defence slots.
const STORED_20: [Instruction; 17] = [
    I::Off,
    I::HugTheTouchline,
    I::FalseNo9,
    I::FalseFullBacks,
    I::AttackingFullBacks,
    I::WingRotation,
    I::TikiTaka,
    I::CenteringTargets,
    I::Defensive,
    I::FalseWinger,
    I::Anchoring,
    I::SwarmTheBox,
    I::DeepDefensiveLine,
    I::Gegenpress,
    I::TightMarking,
    I::CounterTarget,
    I::WingBack,
];

/// The instructions `version` lists, in the order the game lists them:
/// its stored table, the index the stored value. Empty on PES 14/15/16,
/// which have no instruction table.
pub fn instructions(version: PesVersion) -> &'static [Instruction] {
    match version {
        PesVersion::Pes14 | PesVersion::Pes15 | PesVersion::Pes16 => &[],
        PesVersion::Pes17 => &Instruction::ALL[..13],
        PesVersion::Pes18 | PesVersion::Pes19 => &STORED_18,
        PesVersion::Pes20 | PesVersion::Pes21 => &STORED_20,
    }
}

/// The instruction stored as `value` in `version`'s table: `Ok(None)` when the
/// version has no instruction table, `Err(UnknownInstruction)` for a value its
/// table does not list.
pub fn decode(version: PesVersion, value: u8) -> Result<Option<Instruction>, CodecError> {
    let table = instructions(version);
    if table.is_empty() {
        return Ok(None);
    }
    match table.get(usize::from(value)) {
        Some(instruction) => Ok(Some(*instruction)),
        None => Err(CodecError::UnknownInstruction { version, value }),
    }
}

/// The stored value `instruction` has in `version`'s table, `None` when the
/// version has no table or no such instruction.
pub fn encode(version: PesVersion, instruction: Instruction) -> Option<u8> {
    instructions(version)
        .iter()
        .position(|i| *i == instruction)
        .map(|index| u8::try_from(index).expect("the tables fit u8"))
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::model::instruction::Instruction;

    // `instructions` lists the version's stored table in its stored order
    // (the widget's combo offers it, ordered as the game lists it); empty
    // on PES 15/16, which have no instruction table.
    #[test]
    fn instructions_lists_the_versions_table_or_nothing() {
        assert!(instructions(PesVersion::Pes15).is_empty());
        assert!(instructions(PesVersion::Pes16).is_empty());
        assert_eq!(
            instructions(PesVersion::Pes17),
            &Instruction::ALL[..13],
            "PES 17's thirteen in canonical order, ending CounterTarget"
        );
        assert_eq!(
            instructions(PesVersion::Pes17)[12],
            Instruction::CounterTarget
        );
        assert_eq!(instructions(PesVersion::Pes18).len(), 16);
        assert_eq!(instructions(PesVersion::Pes19).len(), 16);
        for version in [PesVersion::Pes20, PesVersion::Pes21] {
            let table = instructions(version);
            assert_eq!(table.len(), 17, "{version:?}'s table");
            assert_eq!(table[0x0A], Instruction::Anchoring, "{version:?}'s 0x0A");
        }
    }
}
