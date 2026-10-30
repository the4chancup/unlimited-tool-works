//! The comparator: every difference between two players of the same version
//! over the whole field walk, and the save-level diff pairing players by id.
//! `ops/` reads `model/` only; the field list is the schema's.

use std::collections::{HashMap, HashSet};

use pes_version::PesVersion;

use crate::codec::{self, CodecError};
use crate::file::EditFile;
use crate::model::ingame_face::in_layout;
use crate::model::player::PlayerEntry;
use crate::model::team::TeamEntry;
use crate::ops::fingerprint::{FaceHash, face_hash};
use crate::schema::fields::{
    PlayerField, PlayerText, RosterField, TacticsField, TeamField, TeamText,
};
use crate::schema::ingame_face::{INGAME_FACE_FIELDS, IngameFaceField};
use crate::schema::{RecordSchema, VersionSchema};

/// Every difference between two players of the same version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerDiff {
    /// A bit-run field of the record.
    Field {
        /// The schema field that differs.
        field: PlayerField,
        /// `a`'s stored value.
        old: u32,
        /// `b`'s stored value.
        new: u32,
    },
    /// The name or shirt name.
    Text {
        /// The text field that differs.
        text: PlayerText,
        /// `a`'s text.
        old: String,
        /// `b`'s text.
        new: String,
    },
    /// A field of the ingame-face run's layout.
    Face {
        /// The ingame-face field that differs.
        field: IngameFaceField,
        /// `a`'s value.
        old: u8,
        /// `b`'s value.
        new: u8,
    },
    /// The run's measured-unused bits differ (`IngameFace::unused`: no field
    /// of its layout names them); the hashes are `ops::fingerprint`'s
    /// fingerprints of the two runs.
    FaceRun {
        /// `a`'s fingerprint.
        old: FaceHash,
        /// `b`'s fingerprint.
        new: FaceHash,
    },
}

/// Whether a diff row is an aesthetics change or a gameplay one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffScope {
    /// A gameplay field.
    Gameplay,
    /// An aesthetics field: the appearance block.
    Aesthetics,
}

impl PlayerDiff {
    /// Aesthetics is the appearance block: its fields (the appearance id, the
    /// four appearance edit flags, boots/gloves/base-copy IDs, physique,
    /// strip), every `Face` and `FaceRun` row — the slice
    /// `test_support::copy_appearance` moves, the appearance id aside, which
    /// the tests check this list against. Everything else (motion and the
    /// other edit flags included) is gameplay.
    pub fn scope(&self) -> DiffScope {
        use PlayerField as F;
        let field = match self {
            PlayerDiff::Field { field, .. } => *field,
            PlayerDiff::Text { .. } => return DiffScope::Gameplay,
            PlayerDiff::Face { .. } | PlayerDiff::FaceRun { .. } => {
                return DiffScope::Aesthetics;
            }
        };
        match field {
            F::AppearanceId
            | F::EditedFace
            | F::EditedHair
            | F::EditedPhysique
            | F::EditedStrip
            | F::BootsId
            | F::GlovesId
            | F::BaseCopyId
            | F::NeckLength
            | F::NeckSize
            | F::ShoulderHeight
            | F::ShoulderWidth
            | F::Chest
            | F::Waist
            | F::ArmSize
            | F::ArmLength
            | F::Thigh
            | F::Calf
            | F::LegLength
            | F::HeadLength
            | F::HeadWidth
            | F::HeadDepth
            | F::WristTapeColorLeft
            | F::WristTapeColorRight
            | F::WristTaping
            | F::SpectaclesColor
            | F::SpectaclesStyle
            | F::Sleeves
            | F::Inners
            | F::Socks
            | F::Undershorts
            | F::Untucked
            | F::AnkleTaping => DiffScope::Aesthetics,
            F::Id
            | F::Nationality
            | F::Height
            | F::Weight
            | F::GoalCelebration1
            | F::GoalCelebration2
            | F::AttackingProwess
            | F::DefensiveProwess
            | F::Goalkeeping
            | F::Dribbling
            | F::FreeKickMotion
            | F::Finishing
            | F::LowPass
            | F::LoftedPass
            | F::Heading
            | F::Form
            | F::EditedPlayer
            | F::Swerve
            | F::Catching
            | F::Clearing
            | F::Reflexes
            | F::InjuryResistance
            | F::EditedBasicSettings
            | F::BodyControl
            | F::PhysicalContact
            | F::KickingPower
            | F::ExplosivePower
            | F::ArmMovementDribbling
            | F::EditedRegisteredPosition
            | F::Age
            | F::RegisteredPosition
            | F::PlayingStyle
            | F::BallControl
            | F::BallWinning
            | F::WeakFootAccuracy
            | F::Jump
            | F::ArmMovementRunning
            | F::CornerKickMotion
            | F::Coverage
            | F::WeakFootUsage
            | F::PlayablePosition(_)
            | F::HunchingDribbling
            | F::HunchingRunning
            | F::PenaltyKickMotion
            | F::PlaceKicking
            | F::EditedPlayablePositions
            | F::EditedAbilities
            | F::EditedSkills
            | F::Stamina
            | F::Speed
            | F::EditedPlayingStyle
            | F::EditedComStyles
            | F::EditedMotion
            | F::BaseCopy
            | F::StrongerFoot
            | F::ComStyle(_)
            | F::Skill(_)
            | F::Star
            | F::DribblingMotion
            | F::TightPossession
            | F::Aggression
            | F::PlayingAttitude
            | F::StrongerHand => DiffScope::Gameplay,
        }
    }
}

/// `Field` rows of one record schema, in schema order. `seen`, when given,
/// is a record whose fields are skipped (the 15/16 appearance record
/// repeats `Id`).
fn record_diffs(
    a: &PlayerEntry,
    b: &PlayerEntry,
    schema: &RecordSchema<PlayerField, PlayerText>,
    seen: Option<&RecordSchema<PlayerField, PlayerText>>,
    diffs: &mut Vec<PlayerDiff>,
) -> Result<(), CodecError> {
    for (field, _, _) in codec::runs(schema) {
        if seen.is_some_and(|seen| seen.has(field)) {
            continue;
        }
        let (old, new) = (a.get(field)?, b.get(field)?);
        if old != new {
            diffs.push(PlayerDiff::Field { field, old, new });
        }
    }
    Ok(())
}

/// Every difference between two players of the same version: the schema's
/// stored fields (the player record's, plus the 15/16 appearance record's)
/// through the model, the two texts, the ingame-face fields of the run's
/// layout, and the run's measured-unused bits as one row. `Err` only when a
/// gated field the schema stores is `None` on either side (an entry not read
/// from a save of this version).
pub fn compare_players(
    a: &PlayerEntry,
    b: &PlayerEntry,
    schema: &VersionSchema,
) -> Result<Vec<PlayerDiff>, CodecError> {
    let mut diffs = Vec::new();
    let (fa, fb) = (&a.appearance.ingame_face, &b.appearance.ingame_face);
    if fa.bytes().len() != fb.bytes().len() || fa.bytes().is_empty() {
        // A run that does not reach a field is `NoIngameFaceRun` on that side;
        // an empty run is an entry never read from a record.
        return Err(CodecError::NoIngameFaceRun {
            field: String::from("ingame_face"),
        });
    }
    record_diffs(a, b, schema.player, None, &mut diffs)?;
    if let Some((_, appearance)) = &schema.appearance {
        record_diffs(a, b, appearance, Some(schema.player), &mut diffs)?;
    }
    for (text, old, new) in [
        (PlayerText::Name, &a.name, &b.name),
        (PlayerText::ShirtName, &a.shirt_name, &b.shirt_name),
    ] {
        if old != new {
            diffs.push(PlayerDiff::Text {
                text,
                old: old.clone(),
                new: new.clone(),
            });
        }
    }
    // Equal lengths, so one layout for both runs.
    let layout = fa.layout();
    for spec in INGAME_FACE_FIELDS
        .iter()
        .filter(|spec| in_layout(spec.field.layout(), layout))
    {
        let (old, new) = (fa.get(spec.field)?, fb.get(spec.field)?);
        if old != new {
            diffs.push(PlayerDiff::Face {
                field: spec.field,
                old,
                new,
            });
        }
    }
    if fa.unused() != fb.unused() {
        diffs.push(PlayerDiff::FaceRun {
            old: face_hash(fa),
            new: face_hash(fb),
        });
    }
    Ok(diffs)
}

/// Every difference between two teams of the same version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamDiff {
    /// A bit-run field of the team record.
    Field {
        /// The schema field that differs.
        field: TeamField,
        /// `a`'s stored value.
        old: u32,
        /// `b`'s stored value.
        new: u32,
    },
    /// The team name or short name.
    Text {
        /// The text field that differs.
        text: TeamText,
        /// `a`'s text.
        old: String,
        /// `b`'s text.
        new: String,
    },
    /// A field of the roster record.
    Roster {
        /// The schema field that differs.
        field: RosterField,
        /// `a`'s stored value.
        old: u32,
        /// `b`'s stored value.
        new: u32,
    },
    /// A field of the tactics record.
    Tactics {
        /// The schema field that differs.
        field: TacticsField,
        /// `a`'s stored value.
        old: u32,
        /// `b`'s stored value.
        new: u32,
    },
}

/// Every difference between two teams of the same version, for the Save
/// editor's unsaved-changes prompt: the schema's stored fields through
/// the model — the team record's runs in schema order, the two texts
/// (name, then short name), the roster record's runs, then the tactics
/// record's runs. `Err` only when a gated field the schema stores is
/// `None` on either side (an entry not read from a save of this version),
/// or with `Index` on a roster or tactics entry shorter than the schema.
pub fn compare_teams(
    a: &TeamEntry,
    b: &TeamEntry,
    schema: &VersionSchema,
) -> Result<Vec<TeamDiff>, CodecError> {
    let mut diffs = Vec::new();
    for (field, _, _) in codec::runs(schema.team) {
        let (old, new) = (a.get(field)?, b.get(field)?);
        if old != new {
            diffs.push(TeamDiff::Field { field, old, new });
        }
    }
    for (text, old, new) in [
        (TeamText::Name, &a.name, &b.name),
        (TeamText::ShortName, &a.short_name, &b.short_name),
    ] {
        if old != new {
            diffs.push(TeamDiff::Text {
                text,
                old: old.clone(),
                new: new.clone(),
            });
        }
    }
    for (field, _, _) in codec::runs(schema.roster) {
        let (old, new) = (a.roster_get(field)?, b.roster_get(field)?);
        if old != new {
            diffs.push(TeamDiff::Roster { field, old, new });
        }
    }
    for (field, _, _) in codec::tactics_runs(schema.tactic) {
        let (old, new) = (a.tactics_get(field)?, b.tactics_get(field)?);
        if old != new {
            diffs.push(TeamDiff::Tactics { field, old, new });
        }
    }
    Ok(diffs)
}

/// The player-level diff of two saves, players paired by id — roster-slot
/// pairing would report a reshuffle as edits.
#[derive(Debug, Default)]
pub struct SaveDiff {
    /// Players in both saves with at least one difference, in `a`'s record
    /// order.
    pub players: Vec<(u32, Vec<PlayerDiff>)>,
    /// Ids `a` has and `b` lacks, in `a`'s record order.
    pub only_in_a: Vec<u32>,
    /// Ids `b` has and `a` lacks, in `b`'s record order.
    pub only_in_b: Vec<u32>,
}

/// A save-level compare that could not run.
#[derive(Debug, thiserror::Error)]
pub enum CompareError {
    /// The two saves are not of the same game version.
    #[error("the saves are of different versions: {a:?} vs {b:?}")]
    VersionMismatch {
        /// `a`'s version.
        a: PesVersion,
        /// `b`'s version.
        b: PesVersion,
    },
    /// A field the schema stores had no value on one side.
    #[error(transparent)]
    Codec(#[from] CodecError),
}

/// Two saves, players paired by id.
pub fn compare(a: &EditFile, b: &EditFile) -> Result<SaveDiff, CompareError> {
    if a.version() != b.version() {
        return Err(CompareError::VersionMismatch {
            a: a.version(),
            b: b.version(),
        });
    }
    let schema = crate::schema::schema_for(a.version());
    // `or_insert` keeps the first record of a duplicate id, as `player` does.
    let b_by_id: HashMap<u32, &PlayerEntry> =
        b.players().iter().fold(HashMap::new(), |mut map, p| {
            map.entry(p.id).or_insert(p);
            map
        });
    let mut diff = SaveDiff::default();
    let mut a_ids = HashSet::new();
    for pa in a.players() {
        a_ids.insert(pa.id);
        match b_by_id.get(&pa.id) {
            Some(pb) => {
                let rows = compare_players(pa, pb, schema)?;
                if !rows.is_empty() {
                    diff.players.push((pa.id, rows));
                }
            }
            None => diff.only_in_a.push(pa.id),
        }
    }
    diff.only_in_b = b
        .players()
        .iter()
        .filter(|p| !a_ids.contains(&p.id))
        .map(|p| p.id)
        .collect();
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::model::ingame_face::IngameFace;
    use crate::schema::fields::PresetField;
    use crate::schema::schema_for;
    use crate::test_support::{FIXTURES, find_player, open, payload};

    fn player(version: PesVersion, id: u32) -> PlayerEntry {
        find_player(&payload(version), schema_for(version), id)
    }

    /// The joined `TeamEntry` for `id` of `version`'s fixture save.
    fn team(version: PesVersion, id: u32) -> TeamEntry {
        open(version).0.team(id).expect("in the fixture").clone()
    }

    /// `(byte, bit)` of the run covered by no field: a measured-unused bit.
    fn an_unused_bit(len: usize) -> (usize, u32) {
        for byte in 0..len {
            for bit in 0..8u32 {
                let at = byte as u32 * 8 + bit;
                let covered = INGAME_FACE_FIELDS
                    .iter()
                    .any(|s| at >= s.bit_offset && at < s.bit_offset + s.bit_width);
                if !covered {
                    return (byte, bit);
                }
            }
        }
        panic!("the {len}-byte run has no unused bit");
    }

    #[test]
    fn a_player_equals_its_own_copy_on_every_version() {
        for version in FIXTURES {
            let p = player(version, 70101);
            assert!(
                compare_players(&p, &p, schema_for(version))
                    .expect("compares")
                    .is_empty(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn each_single_edit_is_one_row_of_the_right_scope() {
        let a = player(PesVersion::Pes21, 70101);
        let schema = schema_for(PesVersion::Pes21);

        let mut b = a.clone();
        b.stats.speed += 1;
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Field {
                field: PlayerField::Speed,
                old: u32::from(a.stats.speed),
                new: u32::from(b.stats.speed),
            }]
        );
        assert_eq!(diffs[0].scope(), DiffScope::Gameplay);

        let mut b = a.clone();
        b.name.push('X');
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Text {
                text: PlayerText::Name,
                old: a.name.clone(),
                new: b.name.clone(),
            }]
        );
        assert_eq!(diffs[0].scope(), DiffScope::Gameplay);

        let mut b = a.clone();
        b.shirt_name.push('X');
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Text {
                text: PlayerText::ShirtName,
                old: a.shirt_name.clone(),
                new: b.shirt_name.clone(),
            }]
        );

        let mut b = a.clone();
        b.appearance.sleeves = (b.appearance.sleeves + 1) % 3;
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Field {
                field: PlayerField::Sleeves,
                old: u32::from(a.appearance.sleeves),
                new: u32::from(b.appearance.sleeves),
            }]
        );
        assert_eq!(diffs[0].scope(), DiffScope::Aesthetics);
    }

    #[test]
    fn a_face_field_is_one_face_row_an_unused_bit_is_one_run_row() {
        let a = player(PesVersion::Pes21, 70101);
        let schema = schema_for(PesVersion::Pes21);
        let face = &a.appearance.ingame_face;

        // A known field: one Face row, no run row.
        let mut b = a.clone();
        let nose = face.get(IngameFaceField::NoseType).expect("covered");
        b.appearance
            .ingame_face
            .set(IngameFaceField::NoseType, nose ^ 1)
            .expect("fits");
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Face {
                field: IngameFaceField::NoseType,
                old: nose,
                new: nose ^ 1,
            }]
        );

        // A bit no field covers: one FaceRun row with the two fingerprints.
        let (byte, bit) = an_unused_bit(face.bytes().len());
        let mut run = face.bytes().to_vec();
        run[byte] ^= 1 << bit;
        let mut b = a.clone();
        b.appearance.ingame_face = IngameFace::from_bytes(run);
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::FaceRun {
                old: face_hash(face),
                new: face_hash(&b.appearance.ingame_face),
            }]
        );
        assert_ne!(face_hash(face), face_hash(&b.appearance.ingame_face));

        // The skin bit is a known field: a Face row, and masked out of the
        // run row either way.
        let mut b = a.clone();
        let skin = face.get(IngameFaceField::SkinColor).expect("covered");
        b.appearance
            .ingame_face
            .set(IngameFaceField::SkinColor, skin ^ 1)
            .expect("fits");
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Face {
                field: IngameFaceField::SkinColor,
                old: skin,
                new: skin ^ 1,
            }]
        );
    }

    #[test]
    fn a_field_is_aesthetics_exactly_when_the_appearance_copy_moves_it() {
        let schema = schema_for(PesVersion::Pes21);
        let a = player(PesVersion::Pes21, 70101);
        for (field, _, width) in crate::codec::runs(schema.player) {
            let old = a.get(field).expect("covered");
            let new = if old == 0 { 1 } else { 0 };
            assert!(width == 32 || new < (1u32 << width), "{field:?}");
            let mut b = a.clone();
            b.set(field, new).expect("fits");
            let diffs = compare_players(&a, &b, schema).expect("compares");
            assert_eq!(diffs, [PlayerDiff::Field { field, old, new }], "{field:?}");
            let mut c = a.clone();
            crate::test_support::copy_appearance(&mut c, &b);
            // The appearance id is the appearance block's, but a copied
            // look keeps the target's own id, so the copy leaves it.
            let moved = field == PlayerField::AppearanceId || c.get(field).expect("covered") == new;
            assert_eq!(
                diffs
                    .first()
                    .is_some_and(|d| d.scope() == DiffScope::Aesthetics),
                moved,
                "{field:?}"
            );
        }

        // The texts are gameplay.
        let mut b = a.clone();
        b.name.push('X');
        b.shirt_name.push('X');
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(diffs.len(), 2, "{diffs:?}");
        for diff in &diffs {
            assert_eq!(diff.scope(), DiffScope::Gameplay, "{diff:?}");
        }
    }

    #[test]
    fn a_face_field_and_an_unused_bit_are_two_rows() {
        let a = player(PesVersion::Pes21, 70101);
        let schema = schema_for(PesVersion::Pes21);
        let face = &a.appearance.ingame_face;
        let nose = face.get(IngameFaceField::NoseType).expect("covered");
        let (byte, bit) = an_unused_bit(face.bytes().len());

        let mut b = a.clone();
        let mut run = face.bytes().to_vec();
        run[byte] ^= 1 << bit;
        b.appearance.ingame_face = IngameFace::from_bytes(run);
        b.appearance
            .ingame_face
            .set(IngameFaceField::NoseType, nose ^ 1)
            .expect("fits");

        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [
                PlayerDiff::Face {
                    field: IngameFaceField::NoseType,
                    old: nose,
                    new: nose ^ 1,
                },
                PlayerDiff::FaceRun {
                    old: face_hash(face),
                    new: face_hash(&b.appearance.ingame_face),
                },
            ]
        );
    }

    #[test]
    fn the_appearance_records_id_is_not_a_second_id_row() {
        let schema = schema_for(PesVersion::Pes16);
        let a = player(PesVersion::Pes16, 70101);
        let b = player(PesVersion::Pes16, 70202);
        assert_ne!(a.id, b.id);
        let ids: Vec<PlayerDiff> = compare_players(&a, &b, schema)
            .expect("compares")
            .into_iter()
            .filter(|d| {
                matches!(
                    d,
                    PlayerDiff::Field {
                        field: PlayerField::Id,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(ids.len(), 1, "{ids:?}");
    }

    #[test]
    fn the_appearance_id_is_an_aesthetics_row() {
        let a = player(PesVersion::Pes21, 70101);
        let schema = schema_for(PesVersion::Pes21);
        assert_eq!(a.appearance_id, Some(70101));
        let mut b = a.clone();
        b.appearance_id = Some(u32::MAX);
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Field {
                field: PlayerField::AppearanceId,
                old: 70101,
                new: u32::MAX,
            }]
        );
        assert_eq!(diffs[0].scope(), DiffScope::Aesthetics);
    }

    #[test]
    fn the_appearance_record_is_walked_on_pes16() {
        let a = player(PesVersion::Pes16, 70101);
        let schema = schema_for(PesVersion::Pes16);
        assert!(schema.appearance.is_some());
        let mut b = a.clone();
        b.appearance.boots_id = b.appearance.boots_id.wrapping_add(1) % 0x4000;
        let diffs = compare_players(&a, &b, schema).expect("compares");
        assert_eq!(
            diffs,
            [PlayerDiff::Field {
                field: PlayerField::BootsId,
                old: a.appearance.boots_id,
                new: b.appearance.boots_id,
            }]
        );
    }

    #[test]
    fn compare_pairs_by_id_and_reports_unmatched() {
        let (a, _) = open(PesVersion::Pes16);
        let (b, _) = open(PesVersion::Pes16);
        let diff = compare(&a, &b).expect("same save");
        assert!(diff.players.is_empty());
        assert!(diff.only_in_a.is_empty());
        assert!(diff.only_in_b.is_empty());

        let mut b = b;
        b.player_mut(70103).expect("in b").stats.speed += 1;
        let renamed = 70104;
        assert!(b.player(renamed).is_some());
        let new_id = 999999;
        assert!(a.player(new_id).is_none());
        b.player_mut(renamed).expect("in b").id = new_id;

        let diff = compare(&a, &b).expect("compares");
        assert_eq!(diff.players.len(), 1);
        assert_eq!(diff.players[0].0, 70103);
        assert_eq!(
            diff.players[0].1,
            [PlayerDiff::Field {
                field: PlayerField::Speed,
                old: a.player(70103).expect("in a").stats.speed.into(),
                new: b.player(70103).expect("in b").stats.speed.into(),
            }]
        );
        assert_eq!(diff.only_in_a, [renamed]);
        assert_eq!(diff.only_in_b, [new_id]);
    }

    #[test]
    fn compare_refuses_a_version_mismatch() {
        let (a, _) = open(PesVersion::Pes16);
        let (b, _) = open(PesVersion::Pes17);
        assert!(matches!(
            compare(&a, &b),
            Err(CompareError::VersionMismatch {
                a: PesVersion::Pes16,
                b: PesVersion::Pes17,
            })
        ));
    }

    #[test]
    fn an_entry_never_read_has_no_face_run_to_compare() {
        let schema = schema_for(PesVersion::Pes21);
        let fixture = player(PesVersion::Pes21, 70101);
        assert!(matches!(
            compare_players(&PlayerEntry::default(), &fixture, schema),
            Err(CodecError::NoIngameFaceRun { .. })
        ));
        assert!(matches!(
            compare_players(&PlayerEntry::default(), &PlayerEntry::default(), schema),
            Err(CodecError::NoIngameFaceRun { .. })
        ));
    }

    /// `compare_teams` on a fixture team of every version: an unchanged
    /// copy gives no rows.
    #[test]
    fn a_team_equals_its_own_copy_on_every_version() {
        for version in FIXTURES {
            let (file, _) = open(version);
            let team = file.teams()[0].clone();
            assert!(
                compare_teams(&team, &team, schema_for(version))
                    .expect("compares")
                    .is_empty(),
                "{version:?}"
            );
        }
    }

    /// One changed field per record kind gives each its row on PES 16
    /// (whose team record holds no colours) and on PES 20 (the 40-slot
    /// roster): the four kinds on versions `each_single_edit_is_one_row`
    /// does not reach.
    #[test]
    fn each_record_kinds_row_on_pes16_and_pes20() {
        // PES 16: the team record holds no colour fields; its row here is
        // a flag's.
        let a = team(PesVersion::Pes16, 701);
        let schema = schema_for(PesVersion::Pes16);
        assert!(a.colors.is_none(), "premise: PES 16 has no team colours");
        let mut b = a.clone();
        b.edit_flags.name = !a.edit_flags.name;
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Field {
                field: TeamField::EditedName,
                old: u32::from(a.edit_flags.name),
                new: u32::from(b.edit_flags.name),
            }]
        );
        assert_ne!(a.name, "/XX/", "premise");
        let mut b = a.clone();
        b.name = "/XX/".to_owned();
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Text {
                text: TeamText::Name,
                old: a.name.clone(),
                new: "/XX/".to_owned(),
            }]
        );
        let mut b = a.clone();
        assert_ne!(a.roster[2].number, 99, "premise");
        b.roster[2].number = 99;
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Roster {
                field: RosterField::Number(2),
                old: u32::from(a.roster[2].number),
                new: 99,
            }]
        );
        let mut b = a.clone();
        assert_ne!(a.tactics.presets[0].sliders.compactness, 1, "premise");
        b.tactics.presets[0].sliders.compactness = 1;
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Tactics {
                field: TacticsField::Preset {
                    preset: 0,
                    field: PresetField::Compactness,
                },
                old: u32::from(a.tactics.presets[0].sliders.compactness),
                new: 1,
            }]
        );

        // PES 20: the roster runs forty slots.
        let a = team(PesVersion::Pes20, 701);
        let schema = schema_for(PesVersion::Pes20);
        assert_eq!(a.roster.len(), 40, "premise: PES 20's roster size");
        let mut b = a.clone();
        b.manager_id = Some(0);
        assert_ne!(a.manager_id, Some(0), "premise");
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Field {
                field: TeamField::ManagerId,
                old: a.manager_id.expect("PES 20 stores it"),
                new: 0,
            }]
        );
        assert_ne!(a.short_name, "XXX", "premise");
        let mut b = a.clone();
        b.short_name = "XXX".to_owned();
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Text {
                text: TeamText::ShortName,
                old: a.short_name.clone(),
                new: "XXX".to_owned(),
            }]
        );
        let last = a.roster.len() - 1;
        assert_ne!(a.roster[last].number, 99, "premise");
        let mut b = a.clone();
        b.roster[last].number = 99;
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Roster {
                field: RosterField::Number(u8::try_from(last).expect("a slot")),
                old: u32::from(a.roster[last].number),
                new: 99,
            }]
        );
        let mut b = a.clone();
        assert_ne!(a.tactics.presets[0].sliders.compactness, 1, "premise");
        b.tactics.presets[0].sliders.compactness = 1;
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Tactics {
                field: TacticsField::Preset {
                    preset: 0,
                    field: PresetField::Compactness,
                },
                old: u32::from(a.tactics.presets[0].sliders.compactness),
                new: 1,
            }]
        );
    }

    /// `compare_teams` over a fixture team: an unchanged copy gives no
    /// rows, one changed field of each record gives exactly its row.
    #[test]
    fn each_single_edit_is_one_row() {
        let a = team(PesVersion::Pes21, 701);
        let schema = schema_for(PesVersion::Pes21);
        assert_eq!(a.manager_id, Some(701), "premise");
        assert!(
            compare_teams(&a, &a.clone(), schema)
                .expect("compares")
                .is_empty()
        );

        let mut b = a.clone();
        b.manager_id = Some(0);
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Field {
                field: TeamField::ManagerId,
                old: 701,
                new: 0,
            }]
        );

        assert_ne!(a.short_name, "XXX", "premise");
        let mut b = a.clone();
        b.short_name = "XXX".to_owned();
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Text {
                text: TeamText::ShortName,
                old: a.short_name.clone(),
                new: "XXX".to_owned(),
            }]
        );

        let mut b = a.clone();
        b.roster[2].number = 99;
        assert_ne!(a.roster[2].number, 99, "premise");
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Roster {
                field: RosterField::Number(2),
                old: u32::from(a.roster[2].number),
                new: 99,
            }]
        );

        let mut b = a.clone();
        b.tactics.presets[1].sliders.support_range = 1;
        assert_ne!(a.tactics.presets[1].sliders.support_range, 1, "premise");
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [TeamDiff::Tactics {
                field: TacticsField::Preset {
                    preset: 1,
                    field: PresetField::SupportRange,
                },
                old: u32::from(a.tactics.presets[1].sliders.support_range),
                new: 1,
            }]
        );
    }

    /// The rows' order is the records': the team record's runs, the
    /// texts, the roster's runs, the tactics' runs.
    #[test]
    fn the_rows_follow_the_records_order() {
        let a = team(PesVersion::Pes21, 701);
        let schema = schema_for(PesVersion::Pes21);
        let mut b = a.clone();
        b.id = 999;
        b.name.push('X');
        b.roster[2].number = 99;
        b.tactics.presets[0].sliders.support_range = 1;
        // The id is read back in each record: the roster's and the
        // tactics' `TeamId` runs diff with the team record's `Id`.
        assert_eq!(
            compare_teams(&a, &b, schema).expect("compares"),
            [
                TeamDiff::Field {
                    field: TeamField::Id,
                    old: 701,
                    new: 999,
                },
                TeamDiff::Text {
                    text: TeamText::Name,
                    old: a.name.clone(),
                    new: b.name.clone(),
                },
                TeamDiff::Roster {
                    field: RosterField::TeamId,
                    old: 701,
                    new: 999,
                },
                TeamDiff::Roster {
                    field: RosterField::Number(2),
                    old: u32::from(a.roster[2].number),
                    new: 99,
                },
                TeamDiff::Tactics {
                    field: TacticsField::TeamId,
                    old: 701,
                    new: 999,
                },
                TeamDiff::Tactics {
                    field: TacticsField::Preset {
                        preset: 0,
                        field: PresetField::SupportRange,
                    },
                    old: u32::from(a.tactics.presets[0].sliders.support_range),
                    new: 1,
                },
            ]
        );
    }

    /// A gated field `None` on one side (an entry not read from a save of
    /// this version) is `Err`, as `compare_players` documents.
    #[test]
    fn a_gated_field_none_on_one_side_is_an_error() {
        let schema = schema_for(PesVersion::Pes21);
        let mut a = team(PesVersion::Pes21, 701);
        a.manager_id = None;
        let b = team(PesVersion::Pes21, 701);
        assert!(matches!(
            compare_teams(&a, &b, schema),
            Err(CodecError::Missing { .. })
        ));
    }
}
