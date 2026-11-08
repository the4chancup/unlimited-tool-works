//! `IngameFace`: the appearance block's byte-22-to-end run, carried verbatim
//! as bytes, with an accessor for each of its fields
//! (`schema::ingame_face::INGAME_FACE_FIELDS`); every other bit is
//! measured unused (`UNUSED_FACE_BITS`).

use crate::codec::{CodecError, bits};
use crate::schema::FieldSpec;
use crate::schema::ingame_face::{
    FaceLayout, INGAME_FACE_FIELDS, IngameFaceField, UNUSED_FACE_BITS,
};

/// Whether a row whose own layout is `own` (`None`: every version) exists
/// in `layout`'s run.
pub(crate) fn in_layout(own: Option<FaceLayout>, layout: FaceLayout) -> bool {
    own.is_none_or(|own| own == layout)
}

/// The ingame-face run (50 bytes, 46 on PES 15). `Default` is empty: a fresh
/// `PlayerEntry` has no run until one is read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IngameFace(Vec<u8>);

/// The `FieldSpec` row of `field`; the table covers every variant.
fn spec(field: IngameFaceField) -> &'static FieldSpec<IngameFaceField> {
    INGAME_FACE_FIELDS
        .iter()
        .find(|s| s.field == field)
        .expect("INGAME_FACE_FIELDS covers every variant; checked by schema::ingame_face::tests")
}

/// The bits `fields` cover in a `len`-byte run, as a per-byte mask (bits a
/// field holds past the run's end are skipped).
fn field_mask(len: usize, fields: impl Iterator<Item = IngameFaceField>) -> Vec<u8> {
    let mut mask = vec![0u8; len];
    for field in fields {
        let spec = spec(field);
        for bit in spec.bit_offset..spec.bit_offset + spec.bit_width {
            if let Some(byte) = mask.get_mut((bit / 8) as usize) {
                *byte |= 1 << (bit % 8);
            }
        }
    }
    mask
}

impl IngameFace {
    /// Whether the run's bytes reach the field's last bit.
    fn covers(&self, spec: &FieldSpec<IngameFaceField>) -> bool {
        (spec.bit_offset + spec.bit_width).div_ceil(8) <= self.0.len() as u32
    }

    /// The run's layout, from its length: the run carries no version, and
    /// only PES 15's is 46 bytes long.
    pub(crate) fn layout(&self) -> FaceLayout {
        if self.0.len() == 46 {
            FaceLayout::Pes15
        } else {
            FaceLayout::Pes16To21
        }
    }

    /// The field's row, if the run holds it: `FieldNotInLayout` for a field
    /// of the other layout, `NoIngameFaceRun` when the run does not reach it
    /// (an entry never read from a record has an empty run).
    fn held(
        &self,
        field: IngameFaceField,
    ) -> Result<&'static FieldSpec<IngameFaceField>, CodecError> {
        let spec = spec(field);
        if !self.0.is_empty() && !in_layout(field.layout(), self.layout()) {
            return Err(CodecError::FieldNotInLayout {
                field: format!("{field:?}"),
            });
        }
        if !self.covers(spec) {
            return Err(CodecError::NoIngameFaceRun {
                field: format!("{field:?}"),
            });
        }
        Ok(spec)
    }

    /// The field's value inside the run: `FieldNotInLayout` or
    /// `NoIngameFaceRun` when the run does not hold it, an error, not a
    /// silent 0, as for an unset gated field.
    pub fn get(&self, field: IngameFaceField) -> Result<u8, CodecError> {
        let spec = self.held(field)?;
        Ok(bits::read_bits(&self.0, spec.bit_offset, spec.bit_width) as u8)
    }

    /// Writes `value` into the field's bits: the errors of `get`, and
    /// `ValueTooWide` when the value does not fit the field's bits.
    pub fn set(&mut self, field: IngameFaceField, value: u8) -> Result<(), CodecError> {
        let spec = self.held(field)?;
        if u32::from(value) >= (1u32 << spec.bit_width) {
            return Err(CodecError::ValueTooWide {
                field: format!("{field:?}"),
                value: u32::from(value),
                width: spec.bit_width,
            });
        }
        bits::write_bits(
            &mut self.0,
            spec.bit_offset,
            spec.bit_width,
            u32::from(value),
        );
        Ok(())
    }

    /// The nose type and angle. PES 15 codes the angle into the nose field:
    /// 3 × type + code, the angle -1, 0, 1 coded 1, 0, 2. PES 16 to 21 store
    /// the type alone, with angle 0.
    pub fn nose(&self) -> Result<(u8, i8), CodecError> {
        let raw = self.get(IngameFaceField::NoseType)?;
        Ok(match self.layout() {
            FaceLayout::Pes15 => {
                let angle = match raw % 3 {
                    0 => 0,
                    1 => -1,
                    _ => 1,
                };
                (raw / 3, angle)
            }
            FaceLayout::Pes16To21 => (raw, 0),
        })
    }

    /// `set`'s errors when the run does not hold the nose field, then
    /// `ValueTooWide` on the nose field for a value past its 6 bits (the
    /// coded value on PES 15, the type on 16 to 21), an angle outside -1..=1,
    /// or a non-zero angle on PES 16 to 21 (the angle's magnitude).
    pub fn set_nose(&mut self, nose_type: u8, angle: i8) -> Result<(), CodecError> {
        // First, so an empty run reports `NoIngameFaceRun` as every other
        // accessor does: its length would otherwise read as PES 16 to 21's
        // layout and refuse a non-zero angle as too wide.
        self.held(IngameFaceField::NoseType)?;
        let too_wide = |value: u32| CodecError::ValueTooWide {
            field: format!("{:?}", IngameFaceField::NoseType),
            value,
            width: spec(IngameFaceField::NoseType).bit_width,
        };
        let bad_angle = too_wide(u32::from(angle.unsigned_abs()));
        let raw = match self.layout() {
            FaceLayout::Pes15 => {
                let code = match angle {
                    0 => 0,
                    -1 => 1,
                    1 => 2,
                    _ => return Err(bad_angle),
                };
                3 * u32::from(nose_type) + code
            }
            FaceLayout::Pes16To21 if angle == 0 => u32::from(nose_type),
            FaceLayout::Pes16To21 => return Err(bad_angle),
        };
        let raw = u8::try_from(raw).map_err(|_| too_wide(raw))?;
        self.set(IngameFaceField::NoseType, raw)
    }

    /// The run's bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// The run with `fields`' bits zeroed.
    fn masked(&self, fields: impl Iterator<Item = IngameFaceField>) -> Vec<u8> {
        field_mask(self.0.len(), fields)
            .iter()
            .zip(&self.0)
            .map(|(mask, byte)| byte & !mask)
            .collect()
    }

    /// The run with the player-gloves bits (the checkbox and its colour) and
    /// the skin-colour bits zeroed, seven bits in all: what the fingerprint
    /// hashes. A gloves or skin change, each a field the comparison reports
    /// on its own, leaves the fingerprint as it was.
    pub fn normalized(&self) -> Vec<u8> {
        self.masked(
            [
                IngameFaceField::PlayerGloves,
                IngameFaceField::PlayerGlovesColor,
                IngameFaceField::SkinColor,
            ]
            .into_iter(),
        )
    }

    /// The run with every field of its layout zeroed: what is left is the
    /// measured-unused bits (`UNUSED_FACE_BITS`), which the comparator reports
    /// as one row.
    pub(crate) fn unused(&self) -> Vec<u8> {
        let layout = self.layout();
        self.masked(
            IngameFaceField::ALL
                .into_iter()
                .filter(|field| in_layout(field.layout(), layout)),
        )
    }

    /// `unused`'s inverse: every bit a field of the run's layout does not
    /// cover taken from `bits`, the fields themselves kept — one run's
    /// measured-unused bits transplanted onto another run of the same
    /// layout. `RunSize` when `bits` is not the run's length.
    pub(crate) fn set_unused(&mut self, bits: &[u8]) -> Result<(), CodecError> {
        if bits.len() != self.0.len() {
            return Err(CodecError::RunSize {
                expected: self.0.len(),
                got: bits.len(),
            });
        }
        let layout = self.layout();
        let mask = field_mask(
            self.0.len(),
            IngameFaceField::ALL
                .into_iter()
                .filter(|field| in_layout(field.layout(), layout)),
        );
        for ((byte, mask), bits) in self.0.iter_mut().zip(&mask).zip(bits) {
            *byte = (*byte & mask) | (bits & !mask);
        }
        Ok(())
    }

    /// Writes every `UNUSED_FACE_BITS` row of the run's layout at its value,
    /// what the games' own editors write.
    pub(crate) fn write_unused(&mut self) {
        let layout = self.layout();
        for row in UNUSED_FACE_BITS
            .iter()
            .filter(|row| in_layout(row.layout, layout))
        {
            // A run too short for the row (one never read from a record) has
            // no bits there to write, and `write_bits` would index past it.
            if (row.bit_offset + row.bit_width).div_ceil(8) as usize <= self.0.len() {
                bits::write_bits(
                    &mut self.0,
                    row.bit_offset,
                    row.bit_width,
                    u32::from(row.value),
                );
            }
        }
    }

    /// A run around `bytes`, as copied out of a record.
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bits `spec` covers as a per-byte mask.
    fn covered(spec: &FieldSpec<IngameFaceField>, byte: usize) -> u8 {
        let mut mask = 0u8;
        for i in 0..spec.bit_width {
            let pos = spec.bit_offset + i;
            if pos / 8 == byte as u32 {
                mask |= 1 << (pos % 8);
            }
        }
        mask
    }

    /// A run with no two equal bytes.
    fn run() -> Vec<u8> {
        (0..50u8)
            .map(|i| i.wrapping_mul(37).wrapping_add(11))
            .collect()
    }

    /// Each layout's fields round-trip on a run of its length; the other
    /// layout's are `FieldNotInLayout` there.
    #[test]
    fn set_then_get_round_trips_every_field_and_touches_only_its_bits() {
        for (layout, len) in [(FaceLayout::Pes15, 46), (FaceLayout::Pes16To21, 50)] {
            for spec in INGAME_FACE_FIELDS {
                let mut face = IngameFace::from_bytes(run()[..len].to_vec());
                assert_eq!(face.layout(), layout);
                if in_layout(spec.field.layout(), layout) {
                    round_trips(&mut face, spec);
                } else {
                    let refused =
                        |result| matches!(result, Err(CodecError::FieldNotInLayout { .. }));
                    assert!(
                        refused(face.get(spec.field).map(|_| ())),
                        "{:?}",
                        spec.field
                    );
                    assert!(refused(face.set(spec.field, 0)), "{:?}", spec.field);
                }
            }
        }
    }

    /// `spec`'s widest value and 0 round-trip, touch only its bits, and one
    /// past the widest is `ValueTooWide`. Writing both means one of them
    /// differs from what the run held, whatever that was.
    fn round_trips(face: &mut IngameFace, spec: &FieldSpec<IngameFaceField>) {
        let before = face.bytes().to_vec();
        let max = u8::MAX >> (8 - spec.bit_width);
        for value in [max, 0] {
            face.set(spec.field, value).expect("the value fits");
            assert_eq!(
                face.get(spec.field).expect("covered"),
                value,
                "{:?}",
                spec.field
            );
            for (byte, (&after, &was)) in face.bytes().iter().zip(&before).enumerate() {
                let keep = !covered(spec, byte);
                assert_eq!(
                    after & keep,
                    was & keep,
                    "{:?} changed bits outside its run in byte {byte}",
                    spec.field
                );
            }
        }
        // An 8-bit field holds every `u8`: nothing is one past it.
        if spec.bit_width < 8 {
            assert!(matches!(
                face.set(spec.field, max + 1),
                Err(CodecError::ValueTooWide { .. })
            ));
        }
    }

    #[test]
    fn an_unread_run_is_no_ingame_face_run_not_a_panic() {
        let mut face = IngameFace::default();
        assert!(matches!(
            face.get(IngameFaceField::SkinColor),
            Err(CodecError::NoIngameFaceRun { .. })
        ));
        assert!(matches!(
            face.set(IngameFaceField::SkinColor, 1),
            Err(CodecError::NoIngameFaceRun { .. })
        ));
        assert!(face.normalized().is_empty());
    }

    /// A run two bytes short of PES 16 to 21's takes the unused rows that fit (bytes 46
    /// and 47) and skips the two past its end, rather than writing past it.
    #[test]
    fn write_unused_skips_a_row_past_a_short_runs_end() {
        let mut face = IngameFace::from_bytes(vec![0xAB; 48]);
        face.write_unused();
        assert_eq!(face.bytes().len(), 48);
        assert_eq!(face.bytes()[46..], [0, 0], "the rows that fit are written");
    }

    // `unused`'s inverse: another run's measured-unused bits land on the
    // run's own, every layout field kept (a FaceRun row applied is the
    // field values' own plus the other run's unused bits).
    #[test]
    fn set_unused_takes_another_runs_unused_bits_and_keeps_the_fields() {
        let mut face = IngameFace::from_bytes(run());
        let other = IngameFace::from_bytes(run().iter().map(|b| !b).collect());
        let before = face.clone();
        assert_ne!(face.unused(), other.unused(), "premise");

        face.set_unused(&other.unused()).expect("same length");

        assert_eq!(face.unused(), other.unused());
        for spec in INGAME_FACE_FIELDS
            .iter()
            .filter(|spec| in_layout(spec.field.layout(), face.layout()))
        {
            assert_eq!(
                face.get(spec.field).expect("covered"),
                before.get(spec.field).expect("covered"),
                "{:?} changed",
                spec.field
            );
        }
        assert!(matches!(
            face.set_unused(&[0; 49]),
            Err(CodecError::RunSize {
                expected: 50,
                got: 49
            })
        ));
    }

    #[test]
    fn normalized_zeroes_exactly_the_gloves_and_skin_bits() {
        let face = IngameFace::from_bytes(run());
        let out = face.normalized();
        let mut masked = vec![0u8; out.len()];
        for field in [
            IngameFaceField::PlayerGloves,
            IngameFaceField::PlayerGlovesColor,
            IngameFaceField::SkinColor,
        ] {
            for (byte, mask) in masked.iter_mut().enumerate() {
                *mask |= covered(spec(field), byte);
            }
        }
        for byte in 0..out.len() {
            assert_eq!(
                out[byte] & masked[byte],
                0,
                "byte {byte} keeps a masked bit"
            );
            assert_eq!(
                out[byte] & !masked[byte],
                face.bytes()[byte] & !masked[byte],
                "byte {byte} lost an unmasked bit"
            );
        }
        // Seven bits exactly: byte 0 low 4, byte 23 low 3.
        assert_eq!(masked.iter().map(|m| m.count_ones()).sum::<u32>(), 7);
        assert_eq!(masked[0], 0x0F);
        assert_eq!(masked[23], 0x07);
    }
}
