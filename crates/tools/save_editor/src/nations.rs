//! The game's nations per version, `data/nations.tsv` read once: each
//! version's `Country.bin` (in `dt10`) lists its nations in the game's
//! capitals, the lists and a few spellings differing between versions
//! (`UNITED STATES` until PES 2017, `USA` after). The General info
//! panel's Nation combo shows the open save's version's own list, so a
//! save is edited the same without the game installed.

use pes_version::PesVersion;

/// The versions' nation lists, parsed from the TSV on first read: one
/// row per ID found in any version, its English name in each version's
/// column, empty where that version lacks it.
static NATIONS: std::sync::LazyLock<[Vec<(u16, &'static str)>; 7]> =
    std::sync::LazyLock::new(parse);

/// `version`'s nations `(id, name)` ordered by ID — the Nation combo's
/// entries, each `"{id} - {name}"`.
pub(crate) fn nations(version: PesVersion) -> &'static [(u16, &'static str)] {
    let index = PesVersion::ALL
        .iter()
        .position(|&v| v == version)
        .expect("a supported version");
    &NATIONS[index]
}

/// The TSV parsed: a header row `id` then `PesVersion::ALL`'s `Display`
/// names in order — asserted, since a reordered file is a programming
/// error — then one row per ID, an empty cell meaning the version
/// lacks it.
fn parse() -> [Vec<(u16, &'static str)>; 7] {
    let mut lines = include_str!("../data/nations.tsv").lines();
    let header = lines.next().expect("nations.tsv is empty");
    let mut columns = header.split('\t');
    assert_eq!(columns.next(), Some("id"), "nations.tsv header: {header}");
    for version in PesVersion::ALL {
        let name = version.to_string();
        assert_eq!(
            columns.next(),
            Some(name.as_str()),
            "nations.tsv header: {header}"
        );
    }
    assert_eq!(columns.next(), None, "nations.tsv header: {header}");
    let mut lists: [Vec<(u16, &'static str)>; 7] = std::array::from_fn(|_| Vec::new());
    for line in lines {
        let mut cells = line.split('\t');
        let id: u16 = cells
            .next()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("nations.tsv row {line:?}: a u16 id"));
        for list in &mut lists {
            match cells.next() {
                Some("") => {}
                Some(name) => list.push((id, name)),
                None => panic!("nations.tsv row {line:?} ends early"),
            }
        }
        assert_eq!(cells.next(), None, "nations.tsv row {line:?} ends late");
    }
    lists
}

#[cfg(test)]
mod tests {
    use pes_savefile::EditFile;

    use super::*;

    /// `version`'s nation `id`'s name, `None` where the version lacks it.
    fn name(version: PesVersion, id: u16) -> Option<&'static str> {
        nations(version)
            .iter()
            .find(|(nation, _)| *nation == id)
            .map(|(_, name)| *name)
    }

    // The lists are each version's own: 189 nations in PES 2015, 191 in
    // 2016, 220 in 2017 and 214 from 2018, when six were dropped.
    #[test]
    fn each_versions_list_is_its_own_length() {
        for (version, count) in PesVersion::ALL
            .into_iter()
            .zip([189, 191, 220, 214, 214, 214, 214])
        {
            assert_eq!(
                nations(version).len(),
                count,
                "{version} holds {count} nations"
            );
        }
    }

    // A few names are spelled otherwise between versions, and six IDs
    // are dropped from PES 2018; `Others` is mixed case as the game
    // spells it.
    #[test]
    fn the_names_and_ids_follow_the_versions() {
        assert_eq!(name(PesVersion::Pes17, 135), Some("UNITED STATES"));
        assert_eq!(name(PesVersion::Pes21, 135), Some("USA"));
        assert_eq!(name(PesVersion::Pes15, 4), None, "PES 2015 lacks BHUTAN");
        assert_eq!(name(PesVersion::Pes21, 100), None, "PES 2021 lacks RÉUNION");
        assert_eq!(name(PesVersion::Pes21, 260), Some("Others"));
    }

    // Every list is ordered by ID, none twice.
    #[test]
    fn the_ids_are_strictly_increasing_in_every_version() {
        for version in PesVersion::ALL {
            let ids: Vec<u16> = nations(version).iter().map(|(id, _)| *id).collect();
            assert!(
                ids.windows(2).all(|pair| pair[0] < pair[1]),
                "{version}'s IDs are strictly increasing"
            );
        }
    }

    // Every nation ID the fixture saves carry is in their version's
    // list, with two exceptions pinned: player 75114 is rostered with
    // nationality 23 (MONGOLIA) on PES 2015 and 2016, and neither
    // game's `Country.bin` holds 23 — the saves keep it anyway, and
    // the combo draws `23 - ?`, the case the `?` entry exists for. A
    // nation dropped from a list a fixture player holds, or an
    // unlisted ID added to a fixture, fails here.
    #[test]
    fn every_fixture_players_nation_is_listed_but_the_games_own_gaps() {
        let mut unlisted = Vec::new();
        for version in PesVersion::ALL {
            let file = EditFile::from_bytes(&pes_savefile::fixture_save(version))
                .expect("the fixture save");
            unlisted.extend(
                file.players()
                    .iter()
                    .filter(|player| {
                        !nations(version)
                            .iter()
                            .any(|(nation, _)| *nation == player.basic.nationality)
                    })
                    .map(|player| (version, player.id, player.basic.nationality)),
            );
        }
        assert_eq!(
            unlisted,
            [
                (PesVersion::Pes15, 75114, 23),
                (PesVersion::Pes16, 75114, 23),
            ]
        );
    }
}
