//! Golden data for `schema::playstyle`: the reference editor's twelve playing-style
//! conversion arrays (`menu_lists.cpp` lines 70-81, verbatim), reproduced through
//! `encode(to, decode(from, i))`, plus the fixture census that confirmed their index
//! spaces (plan: `spec/pes_savefile/operations.md` "Cross-version player conversion").

use pes_version::PesVersion;

use crate::codec::{CodecError, read_player};
use crate::model::playstyle::PlayStyle;
use crate::schema::playstyle::{decode, encode};
use crate::schema::schema_for;
use crate::test_support::{FIXTURES, count, payload, record};

const V16: &[PesVersion] = &[PesVersion::Pes15, PesVersion::Pes16];
const V1718: &[PesVersion] = &[PesVersion::Pes17, PesVersion::Pes18];
const V19: &[PesVersion] = &[PesVersion::Pes19];
const V2021: &[PesVersion] = &[PesVersion::Pes20, PesVersion::Pes21];

/// `(from group, to group, array)`: "playstyle number in version X goes to array[X] in version Y".
const REFERENCE_ARRAYS: &[(&[PesVersion], &[PesVersion], &[u8])] = &[
    (
        V16,
        V1718,
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 16, 17,
        ],
    ),
    (
        V1718,
        V16,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18],
    ),
    (
        V16,
        V19,
        &[
            0, 1, 2, 3, 6, 9, 10, 11, 14, 12, 19, 16, 18, 4, 5, 15, 0, 20, 21,
        ],
    ),
    (
        V19,
        V16,
        &[
            0, 1, 2, 3, 13, 14, 4, 0, 0, 5, 6, 7, 9, 0, 8, 15, 11, 0, 12, 10, 17, 18,
        ],
    ),
    (
        V16,
        V2021,
        &[
            0, 1, 2, 3, 6, 9, 10, 11, 14, 12, 19, 15, 17, 4, 5, 18, 0, 20, 21,
        ],
    ),
    (
        V2021,
        V16,
        &[
            0, 1, 2, 3, 13, 14, 4, 0, 0, 5, 6, 7, 9, 0, 8, 11, 0, 12, 15, 10, 17, 18,
        ],
    ),
    (
        V1718,
        V19,
        &[
            0, 1, 2, 3, 6, 9, 10, 11, 14, 12, 19, 16, 18, 4, 5, 15, 20, 21,
        ],
    ),
    (
        V19,
        V1718,
        &[
            0, 1, 2, 3, 13, 14, 4, 0, 0, 5, 6, 7, 9, 0, 8, 15, 11, 0, 12, 10, 16, 17,
        ],
    ),
    (
        V1718,
        V2021,
        &[
            0, 1, 2, 3, 6, 9, 10, 11, 14, 12, 19, 15, 17, 4, 5, 18, 20, 21,
        ],
    ),
    (
        V2021,
        V1718,
        &[
            0, 1, 2, 3, 13, 14, 4, 0, 0, 5, 6, 7, 9, 0, 8, 11, 0, 12, 15, 10, 16, 17,
        ],
    ),
    (
        V19,
        V2021,
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 15, 16, 17, 19, 20, 21,
        ],
    ),
    (
        V2021,
        V19,
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 15, 19, 20, 21,
        ],
    ),
];

#[test]
fn the_twelve_reference_arrays_are_reproduced() {
    for (from_group, to_group, array) in REFERENCE_ARRAYS {
        for &from in *from_group {
            for &to in *to_group {
                for (i, &expected) in array.iter().enumerate() {
                    let i = i as u8;
                    match decode(from, i) {
                        Ok(style) => {
                            let got = encode(to, style).unwrap_or(0);
                            assert_eq!(got, expected, "{from:?} {i} ({style:?}) -> {to:?}");
                        }
                        Err(CodecError::UnknownPlayingStyle { version, value }) => {
                            // The one hole the arrays carry: PES 15/16's index 16 maps to 0.
                            assert_eq!((version, value), (from, 16), "unexpected hole");
                            assert!(V16.contains(&from), "{from:?} has no hole");
                            assert_eq!(expected, 0, "{from:?} {i} -> {to:?}");
                        }
                        Err(e) => panic!("{from:?} {i}: {e}"),
                    }
                }
            }
        }
    }
}

#[test]
fn every_list_round_trips_and_none_is_zero_everywhere() {
    for version in std::iter::once(PesVersion::Pes14).chain(PesVersion::ALL) {
        assert_eq!(encode(version, PlayStyle::None), Some(0), "{version:?}");
        for style in PlayStyle::ALL {
            if let Some(i) = encode(version, style) {
                assert_eq!(
                    decode(version, i).expect("listed"),
                    style,
                    "{version:?} {style:?}"
                );
            }
        }
        assert!(matches!(
            decode(version, 31),
            Err(CodecError::UnknownPlayingStyle { value: 31, .. })
        ));
    }
    assert!(matches!(
        decode(PesVersion::Pes16, 16),
        Err(CodecError::UnknownPlayingStyle { value: 16, .. })
    ));
}

/// PES 2014's own list: PES 15/16's with The Instigator at 15, where
/// 15/16 has Build Up, and Solid Player filling the hole at 16. Neither
/// style exists on another version's list.
#[test]
fn pes14_decodes_its_own_two_styles() {
    for (index, style) in [
        (15, PlayStyle::TheInstigator),
        (16, PlayStyle::SolidPlayer),
        (17, PlayStyle::OffensiveGoalkeeper),
        (18, PlayStyle::DefensiveGoalkeeper),
    ] {
        assert_eq!(
            decode(PesVersion::Pes14, index).ok(),
            Some(style),
            "stored {index}"
        );
        assert_eq!(encode(PesVersion::Pes14, style), Some(index), "{style:?}");
    }
    assert!(matches!(
        decode(PesVersion::Pes14, 19),
        Err(CodecError::UnknownPlayingStyle { value: 19, .. })
    ));
    for style in [PlayStyle::TheInstigator, PlayStyle::SolidPlayer] {
        for version in PesVersion::ALL {
            assert_eq!(encode(version, style), None, "{version:?} {style:?}");
        }
    }
}

#[test]
fn every_fixture_player_decodes_and_goalkeepers_carry_goalkeeper_styles() {
    for version in FIXTURES {
        let payload = payload(version);
        let schema = schema_for(version);
        let (mut gk_styled, mut gk_styled_as_gk) = (0u32, 0u32);
        for i in 0..count(&payload, &schema.players) {
            let rec = record(&payload, &schema.players, schema.player.size, i);
            let p = read_player(version, rec, schema.player).expect("decode");
            let style = decode(version, p.positions.playing_style)
                .unwrap_or_else(|e| panic!("{version:?} player {}: {e}", p.id));
            if p.positions.registered == 0 && style != PlayStyle::None {
                gk_styled += 1;
                if matches!(
                    style,
                    PlayStyle::OffensiveGoalkeeper | PlayStyle::DefensiveGoalkeeper
                ) {
                    gk_styled_as_gk += 1;
                }
            }
        }
        // The PES 20 fixture is a day-0 save with 414 named players (28 styled goalkeepers).
        let floor = if version == PesVersion::Pes20 { 20 } else { 50 };
        assert!(
            gk_styled >= floor,
            "{version:?}: only {gk_styled} styled goalkeepers"
        );
        assert!(
            gk_styled_as_gk * 100 >= gk_styled * 95,
            "{version:?}: {gk_styled_as_gk} of {gk_styled} styled goalkeepers have a GK style"
        );
    }
}
