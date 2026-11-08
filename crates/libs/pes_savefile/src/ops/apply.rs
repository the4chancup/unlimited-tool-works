//! Applying a comparator row to a player entry: the replayed side of the
//! rows `ops::compare` produces, each applied only when the entry holds
//! the row's old value.

use crate::codec::CodecError;
use crate::model::player::PlayerEntry;
use crate::ops::compare::PlayerDiff;
use crate::ops::fingerprint::face_hash;
use crate::schema::fields::PlayerText;

/// A comparator row could not be applied to the entry.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// The entry does not hold the row's old value: the model changed
    /// unrecorded, or the row belongs to another entry.
    #[error("holds {holds}")]
    Stale {
        /// What the entry holds, spelled as the journal's line spells the
        /// value (the number, the text, the run's fingerprint).
        holds: String,
    },
    /// The row's field could not be read or written on the entry.
    #[error(transparent)]
    Codec(#[from] CodecError),
}

/// Applies `row` to `entry`: the row's old value must be what the entry
/// holds (`Stale` naming what it holds, the entry untouched, otherwise),
/// then the entry takes the row's new one.
pub fn apply_player(entry: &mut PlayerEntry, row: &PlayerDiff) -> Result<(), ApplyError> {
    match row {
        PlayerDiff::Field { field, old, new } => {
            let held = entry.get(*field)?;
            if held != *old {
                return Err(ApplyError::Stale {
                    holds: held.to_string(),
                });
            }
            entry.set(*field, *new)?;
        }
        PlayerDiff::Text { text, old, new } => {
            let held = match text {
                PlayerText::Name => &mut entry.name,
                PlayerText::ShirtName => &mut entry.shirt_name,
            };
            if *held != *old {
                return Err(ApplyError::Stale {
                    holds: held.clone(),
                });
            }
            *held = new.clone();
        }
        PlayerDiff::Face { field, old, new } => {
            let face = &mut entry.appearance.ingame_face;
            let held = face.get(*field)?;
            if held != *old {
                return Err(ApplyError::Stale {
                    holds: held.to_string(),
                });
            }
            face.set(*field, *new)?;
        }
        PlayerDiff::FaceRun {
            old_bits, new_bits, ..
        } => {
            let face = &mut entry.appearance.ingame_face;
            if face.unused() != *old_bits {
                return Err(ApplyError::Stale {
                    holds: face_hash(face).to_string(),
                });
            }
            face.set_unused(new_bits)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::model::ingame_face::IngameFace;
    use crate::schema::ingame_face::IngameFaceField;
    use crate::schema::schema_for;
    use crate::test_support::{find_player, payload};

    /// The player's entry of the PES 21 fixture and `f`'s edit of it, with
    /// `f`'s single `compare_players` row.
    fn row_of(f: impl Fn(&mut PlayerEntry)) -> (PlayerEntry, PlayerEntry, PlayerDiff) {
        let a = find_player(
            &payload(PesVersion::Pes21),
            schema_for(PesVersion::Pes21),
            70101,
        );
        let mut b = a.clone();
        f(&mut b);
        let rows = crate::ops::compare::compare_players(&a, &b, schema_for(PesVersion::Pes21))
            .expect("compares");
        assert_eq!(rows.len(), 1, "one edit, one row: {rows:?}");
        (a, b, rows.into_iter().next().expect("one row"))
    }

    /// `a` with `row` applied equals `b`.
    fn applies(a: &PlayerEntry, row: &PlayerDiff, b: &PlayerEntry) {
        let mut applied = a.clone();
        apply_player(&mut applied, row).expect("the row applies");
        assert!(
            crate::ops::compare::compare_players(&applied, b, schema_for(PesVersion::Pes21))
                .expect("compares")
                .is_empty(),
            "the applied entry equals the after entry"
        );
    }

    /// `row` on `b` (whose held value is the row's `new`, not its `old`) is
    /// `Stale` naming `holds` and leaves the entry untouched.
    fn stale(b: &PlayerEntry, row: &PlayerDiff, holds: &str) {
        let mut entry = b.clone();
        match apply_player(&mut entry, row) {
            Err(ApplyError::Stale { holds: held }) => assert_eq!(held, holds),
            other => panic!("expected Stale, got {other:?}"),
        }
        assert_eq!(&entry, b, "a stale row changes nothing");
    }

    #[test]
    fn a_field_row_applies_and_a_stale_one_is_refused() {
        let (a, b, row) = row_of(|p| p.stats.speed = p.stats.speed.wrapping_add(1));
        assert!(matches!(
            row,
            PlayerDiff::Field {
                field: crate::schema::fields::PlayerField::Speed,
                ..
            }
        ));
        applies(&a, &row, &b);
        stale(&b, &row, &b.stats.speed.to_string());
    }

    #[test]
    fn a_text_row_applies_and_a_stale_one_is_refused() {
        let (a, b, row) = row_of(|p| p.shirt_name.push('Z'));
        assert!(matches!(row, PlayerDiff::Text { .. }));
        applies(&a, &row, &b);
        stale(&b, &row, &b.shirt_name);
    }

    #[test]
    fn a_face_row_applies_and_a_stale_one_is_refused() {
        let (a, b, row) = row_of(|p| {
            let nose = p
                .appearance
                .ingame_face
                .get(IngameFaceField::NoseType)
                .expect("covered");
            p.appearance
                .ingame_face
                .set(IngameFaceField::NoseType, nose ^ 1)
                .expect("fits");
        });
        assert!(matches!(row, PlayerDiff::Face { .. }));
        applies(&a, &row, &b);
        let held = b
            .appearance
            .ingame_face
            .get(IngameFaceField::NoseType)
            .expect("covered")
            .to_string();
        stale(&b, &row, &held);
    }

    #[test]
    fn a_face_run_row_applies_and_a_stale_one_is_refused() {
        let (a, b, row) = row_of(|p| {
            let mut run = p.appearance.ingame_face.bytes().to_vec();
            // The first bit an all-set run's `unused` leaves set: a bit no
            // field of the layout covers, so the row is the run row.
            let covered = IngameFace::from_bytes(vec![0xff; run.len()]).unused();
            let (byte, mask) = covered
                .iter()
                .enumerate()
                .find(|(_, mask)| **mask != 0)
                .expect("the layout leaves a bit uncovered");
            run[byte] ^= 1 << mask.trailing_zeros();
            p.appearance.ingame_face = IngameFace::from_bytes(run);
        });
        assert!(matches!(row, PlayerDiff::FaceRun { .. }));
        applies(&a, &row, &b);
        stale(&b, &row, &face_hash(&b.appearance.ingame_face).to_string());
    }
}
