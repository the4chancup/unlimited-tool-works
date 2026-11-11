//! The VGL27 teams' old boots and gloves ID blocks, `legacy_boots_blocks.txt`: the 25 IDs
//! the cup gave each team once for good, which a legacy export's recorded old IDs are
//! checked against (`legacy_id_out_of_block`). It goes with VGL27.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use teams_list::TeamName;

/// A block's span past its first ID: `FirstBootsID` to `FirstBootsID + 24`, 25 IDs.
const BLOCK_LAST: u32 = 24;

/// The parsed `legacy_boots_blocks.txt`: each team's block by its teams-list name.
#[derive(Debug)]
pub(crate) struct LegacyBlocks {
    /// The team's first boots ID, its block reaching `+ BLOCK_LAST`.
    blocks: BTreeMap<TeamName, u16>,
}

impl LegacyBlocks {
    /// The table bundled into the binary, which `compile` writes into the data directory
    /// for the member to edit when a run compiles a legacy export and the file is missing.
    pub(crate) const EMBEDDED: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/legacy_boots_blocks.txt"
    ));

    /// `EMBEDDED` parsed.
    pub(crate) fn embedded() -> LegacyBlocks {
        LegacyBlocks::parse(LegacyBlocks::EMBEDDED)
            .expect("the embedded block table parses; the module's tests check it")
    }

    /// The table `text` parsed: the header line skipped, then a `Name` TAB `FirstBootsID`
    /// row per team (LF or CRLF). A row not holding a team name and a number, a team
    /// named twice, or no team at all refuses the file.
    pub(crate) fn parse(text: &str) -> Result<LegacyBlocks, String> {
        let mut blocks = BTreeMap::new();
        for (index, line) in text.lines().enumerate().skip(1) {
            if line.is_empty() {
                continue;
            }
            let row = index + 1;
            let Some((name, first)) = line.split_once('\t') else {
                return Err(format!("line {row}: not a `Name` TAB `FirstBootsID` row"));
            };
            let name = TeamName::new(name).map_err(|error| format!("line {row}: {error}"))?;
            let first = first
                .trim()
                .parse::<u16>()
                .map_err(|_| format!("line {row}: {first:?} is not a valid first boots ID"))?;
            if blocks.insert(name, first).is_some() {
                return Err(format!("line {row}: the team is listed twice"));
            }
        }
        if blocks.is_empty() {
            return Err("the table holds no team".to_owned());
        }
        Ok(LegacyBlocks { blocks })
    }

    /// The team's old-ID block, `FirstBootsID` to `FirstBootsID + 24` for boots and gloves
    /// alike, the name compared letter case aside (`TeamName` is already folded); `None`
    /// for a team the table does not list.
    pub(crate) fn of(&self, team: &TeamName) -> Option<RangeInclusive<u32>> {
        self.blocks
            .get(team)
            .map(|first| u32::from(*first)..=u32::from(*first) + BLOCK_LAST)
    }
}

/// Whether `id` lies inside `block`: `None`, a team the table does not list, puts every
/// recorded old ID out of block.
pub(crate) fn in_block(block: Option<&RangeInclusive<u32>>, id: u32) -> bool {
    block.is_some_and(|block| block.contains(&id))
}

/// `block` as `legacy_id_out_of_block`'s `block` context spells it (`2301-2325`); `none`
/// for a team the table does not list.
pub(crate) fn block_text(block: Option<&RangeInclusive<u32>>) -> String {
    match block {
        Some(block) => format!("{}-{}", block.start(), block.end()),
        None => "none".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_parses_and_finds_teams_letter_case_aside() {
        let blocks = LegacyBlocks::embedded();
        assert_eq!(
            blocks.of(&TeamName::new("gbfg").unwrap()),
            Some(2301..=2325)
        );
        assert_eq!(
            blocks.of(&TeamName::new("GBFG").unwrap()),
            Some(2301..=2325)
        );
        assert_eq!(blocks.of(&TeamName::new("co").unwrap()), None);
    }

    #[test]
    fn rows_are_name_tab_first_id_and_a_bad_row_is_refused() {
        let blocks = LegacyBlocks::parse("Name\tFirstBootsID\r\n/a/\t2001\r\n/b/\t2101\n").unwrap();
        assert_eq!(blocks.of(&TeamName::new("a").unwrap()), Some(2001..=2025));
        assert_eq!(blocks.of(&TeamName::new("b").unwrap()), Some(2101..=2125));
        for text in [
            "",
            "Name\tFirstBootsID\n",
            "Name\tFirstBootsID\nnot a row\n",
            "Name\tFirstBootsID\n/a/\tnothing\n",
            "Name\tFirstBootsID\n/a/\t2001\n/A/\t2101\n",
        ] {
            assert!(LegacyBlocks::parse(text).is_err(), "{text:?}");
        }
        // The refusal names the row's line in the file, the header being line 1.
        assert_eq!(
            LegacyBlocks::parse("Name\tFirstBootsID\n/a/\t2001\n/A/\t2101\n").err(),
            Some("line 3: the team is listed twice".to_owned())
        );
    }

    #[test]
    fn an_id_is_in_block_within_first_plus_24_and_none_holds_none() {
        let block = Some(2301..=2325);
        for id in [2301, 2310, 2325] {
            assert!(in_block(block.as_ref(), id), "{id}");
        }
        for id in [2300, 2326] {
            assert!(!in_block(block.as_ref(), id), "{id}");
        }
        assert!(!in_block(None, 2305));
        assert_eq!(block_text(block.as_ref()), "2301-2325");
        assert_eq!(block_text(None), "none");
    }
}
