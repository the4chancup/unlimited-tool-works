//! The game's nations per version, `data/nations.tsv` read once: each
//! version's `Country.bin` (in `dt10`) lists its nations in the game's
//! capitals, the lists and a few spellings differing between versions
//! (`UNITED STATES` until PES 2017, `USA` after). The General info
//! panel's Nation combo shows the open save's version's own list, so a
//! save is edited the same without the game installed.

use std::borrow::Cow;

use pes_version::PesVersion;

/// The TSV's nation columns in order; `PesVersion::ALL` cannot order them,
/// it leaves PES 2014 out.
const COLUMNS: [PesVersion; 8] = [
    PesVersion::Pes14,
    PesVersion::Pes15,
    PesVersion::Pes16,
    PesVersion::Pes17,
    PesVersion::Pes18,
    PesVersion::Pes19,
    PesVersion::Pes20,
    PesVersion::Pes21,
];

/// The versions' nation lists, parsed from the TSV on first read: one
/// row per ID found in any version, its English name in each version's
/// column, empty where that version lacks it.
static NATIONS: std::sync::LazyLock<[Vec<(u16, String)>; 8]> = std::sync::LazyLock::new(parse);

/// `version`'s nations ordered by ID — the Nation combo's entries, each
/// `(id, its "{id} - {name}" text built at load)`.
pub(crate) fn nations(version: PesVersion) -> &'static [(u16, String)] {
    let index = COLUMNS
        .iter()
        .position(|&v| v == version)
        .expect("a supported version");
    &NATIONS[index]
}

/// `id` worded as the Nation combo words it (`"{id} - {name}"`, or
/// `"{id} - ?"` where `version`'s list lacks the ID): the combo's text
/// and the changes' Nation rows share the one wording. The listed case
/// borrows the text built at load; only the `?` allocates.
pub(crate) fn nation_text(version: PesVersion, id: u16) -> Cow<'static, str> {
    let own = nations(version);
    match own.binary_search_by_key(&id, |(nation, _)| *nation) {
        Ok(at) => Cow::Borrowed(own[at].1.as_str()),
        Err(_) => Cow::Owned(format!("{id} - ?")),
    }
}

/// The TSV parsed: a header row `id` then `COLUMNS`' `Display` names
/// in order — asserted, since a reordered file is a programming
/// error — then one row per ID, an empty cell meaning the version
/// lacks it. Each row keeps its `"{id} - {name}"` text rather than the
/// bare name: the text is all the combo and the diff rows ever show,
/// and building it here pays it once for every frame and row.
fn parse() -> [Vec<(u16, String)>; 8] {
    let mut lines = include_str!("../data/nations.tsv").lines();
    let header = lines.next().expect("nations.tsv is empty");
    let mut columns = header.split('\t');
    assert_eq!(columns.next(), Some("id"), "nations.tsv header: {header}");
    for version in COLUMNS {
        let name = version.to_string();
        assert_eq!(
            columns.next(),
            Some(name.as_str()),
            "nations.tsv header: {header}"
        );
    }
    assert_eq!(columns.next(), None, "nations.tsv header: {header}");
    let mut lists: [Vec<(u16, String)>; 8] = std::array::from_fn(|_| Vec::new());
    for line in lines {
        let mut cells = line.split('\t');
        let id: u16 = cells
            .next()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("nations.tsv row {line:?}: a u16 id"));
        for list in &mut lists {
            match cells.next() {
                Some("") => {}
                Some(name) => list.push((id, format!("{id} - {name}"))),
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

    /// `version`'s nation `id`'s `"{id} - {name}"` text, `None` where the
    /// version lacks it.
    fn text(version: PesVersion, id: u16) -> Option<&'static str> {
        nations(version)
            .iter()
            .find(|(nation, _)| *nation == id)
            .map(|(_, text)| text.as_str())
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
        assert_eq!(text(PesVersion::Pes17, 135), Some("135 - UNITED STATES"));
        assert_eq!(text(PesVersion::Pes21, 135), Some("135 - USA"));
        assert_eq!(text(PesVersion::Pes15, 4), None, "PES 2015 lacks BHUTAN");
        assert_eq!(text(PesVersion::Pes21, 100), None, "PES 2021 lacks RÉUNION");
        assert_eq!(text(PesVersion::Pes21, 260), Some("260 - Others"));
    }

    // Every list is ordered by ID, none twice.
    #[test]
    fn the_ids_are_strictly_increasing_in_every_version() {
        for version in COLUMNS {
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
        for version in COLUMNS {
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

    // PES 2014's own list holds 182 nations, seven short of PES 2015's:
    // 8 (HONG KONG) is one it lacks, so it reads `8 - ?` there while
    // PES 2015 names it; 231 is SAN MARINO on every version.
    #[test]
    fn pes14s_own_list_names_its_nations() {
        assert_eq!(nations(PesVersion::Pes14).len(), 182);
        assert_eq!(nation_text(PesVersion::Pes14, 231), "231 - SAN MARINO");
        assert_eq!(nation_text(PesVersion::Pes14, 8), "8 - ?");
        assert_eq!(nation_text(PesVersion::Pes15, 8), "8 - HONG KONG");
    }
}
