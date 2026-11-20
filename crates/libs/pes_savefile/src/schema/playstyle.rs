//! The per-version playing-style lists (`spec/pes_savefile/operations.md`
//! "Cross-version player conversion"). The stored `u8` indexes the list of the
//! version's group; `None` marks a hole no style occupies (PES 15/16's 16).

use pes_version::PesVersion;

use crate::codec::CodecError;
use crate::model::playstyle::PlayStyle;

use PlayStyle as P;

/// PES 15/16: 19 slots; index 16 is unused (no player carries it).
const PES16: &[Option<PlayStyle>] = &[
    Some(P::None),
    Some(P::GoalPoacher),
    Some(P::DummyRunner),
    Some(P::FoxInTheBox),
    Some(P::ProlificWinger),
    Some(P::ClassicNo10),
    Some(P::HolePlayer),
    Some(P::BoxToBox),
    Some(P::AnchorMan),
    Some(P::TheDestroyer),
    Some(P::ExtraFrontman),
    Some(P::OffensiveFullback),
    Some(P::DefensiveFullback),
    Some(P::TargetMan),
    Some(P::CreativePlaymaker),
    Some(P::BuildUp),
    None,
    Some(P::OffensiveGoalkeeper),
    Some(P::DefensiveGoalkeeper),
];

/// PES 17/18: 18 slots; the two goalkeeper styles move to 16/17.
const PES1718: &[Option<PlayStyle>] = &[
    Some(P::None),
    Some(P::GoalPoacher),
    Some(P::DummyRunner),
    Some(P::FoxInTheBox),
    Some(P::ProlificWinger),
    Some(P::ClassicNo10),
    Some(P::HolePlayer),
    Some(P::BoxToBox),
    Some(P::AnchorMan),
    Some(P::TheDestroyer),
    Some(P::ExtraFrontman),
    Some(P::OffensiveFullback),
    Some(P::DefensiveFullback),
    Some(P::TargetMan),
    Some(P::CreativePlaymaker),
    Some(P::BuildUp),
    Some(P::OffensiveGoalkeeper),
    Some(P::DefensiveGoalkeeper),
];

/// PES 19: the full 22-style list in its own order.
const PES19: &[Option<PlayStyle>] = &[
    Some(P::None),
    Some(P::GoalPoacher),
    Some(P::DummyRunner),
    Some(P::FoxInTheBox),
    Some(P::TargetMan),
    Some(P::CreativePlaymaker),
    Some(P::ProlificWinger),
    Some(P::RoamingFlank),
    Some(P::CrossingSpecialist),
    Some(P::ClassicNo10),
    Some(P::HolePlayer),
    Some(P::BoxToBox),
    Some(P::TheDestroyer),
    Some(P::Orchestrator),
    Some(P::AnchorMan),
    Some(P::BuildUp),
    Some(P::OffensiveFullback),
    Some(P::FullbackFinisher),
    Some(P::DefensiveFullback),
    Some(P::ExtraFrontman),
    Some(P::OffensiveGoalkeeper),
    Some(P::DefensiveGoalkeeper),
];

/// PES 20/21: the same 22 styles; the canonical enum's order.
const PES2021: &[Option<PlayStyle>] = &[
    Some(P::None),
    Some(P::GoalPoacher),
    Some(P::DummyRunner),
    Some(P::FoxInTheBox),
    Some(P::TargetMan),
    Some(P::CreativePlaymaker),
    Some(P::ProlificWinger),
    Some(P::RoamingFlank),
    Some(P::CrossingSpecialist),
    Some(P::ClassicNo10),
    Some(P::HolePlayer),
    Some(P::BoxToBox),
    Some(P::TheDestroyer),
    Some(P::Orchestrator),
    Some(P::AnchorMan),
    Some(P::OffensiveFullback),
    Some(P::FullbackFinisher),
    Some(P::DefensiveFullback),
    Some(P::BuildUp),
    Some(P::ExtraFrontman),
    Some(P::OffensiveGoalkeeper),
    Some(P::DefensiveGoalkeeper),
];

/// The list `version`'s stored values index. PES 2014's is an empty list
/// until it is measured, so every stored index decodes as unknown rather
/// than as another version's style.
fn list(version: PesVersion) -> &'static [Option<PlayStyle>] {
    match version {
        PesVersion::Pes14 => &[],
        PesVersion::Pes15 | PesVersion::Pes16 => PES16,
        PesVersion::Pes17 | PesVersion::Pes18 => PES1718,
        PesVersion::Pes19 => PES19,
        PesVersion::Pes20 | PesVersion::Pes21 => PES2021,
    }
}

/// The style at stored index `value` in `version`'s list. An index outside the
/// list or on a hole is `CodecError::UnknownPlayingStyle`.
pub fn decode(version: PesVersion, value: u8) -> Result<PlayStyle, CodecError> {
    match list(version).get(usize::from(value)) {
        Some(Some(style)) => Ok(*style),
        Some(None) | None => Err(CodecError::UnknownPlayingStyle { version, value }),
    }
}

/// The stored index `style` has in `version`'s list, `None` when the version
/// has no such style.
pub fn encode(version: PesVersion, style: PlayStyle) -> Option<u8> {
    list(version)
        .iter()
        .position(|slot| *slot == Some(style))
        .map(|i| i as u8)
}
