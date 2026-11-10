//! A team export's root `legacy_ids.toml`: the old boots and gloves IDs
//! (above the stock band) of the folders each roster slot wears, written by
//! the Export upgrader's legacy mode for VGL27. Planning emits the player's
//! own packages under them instead of his exclusive ones of the team's
//! block. It goes with VGL27.

use std::collections::BTreeMap;

use aesthetics_export::PlayerSlot;
use serde::Deserialize;

use super::item_rows::ItemTable;

/// One slot's recorded old IDs, as the file's `[NN]` table writes them: the
/// old boots and gloves ID of the folders he wears. A kind he wears no
/// migrated folder of has no key.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotIds {
    /// The old boots ID.
    boots: Option<u16>,
    /// The old gloves ID.
    gloves: Option<u16>,
}

/// A team export's parsed `legacy_ids.toml`: the old ID each slot's boots
/// and gloves were packed under, by roster slot.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LegacyIds {
    /// The slots recording an ID of a kind.
    slots: BTreeMap<PlayerSlot, SlotIds>,
}

impl LegacyIds {
    /// The `legacy_ids.toml` `text` parsed: a `[NN]` table per roster slot
    /// (two digits, 01 to 23), holding `boots` and `gloves` keys alone, each
    /// an integer above the stock band (over 100, at most 9999). Any other
    /// table name, key or value refuses the whole file.
    pub(crate) fn from_toml(text: &str) -> Result<LegacyIds, String> {
        let tables =
            toml::from_str::<BTreeMap<String, SlotIds>>(text).map_err(|error| error.to_string())?;
        let mut slots = BTreeMap::new();
        for (name, ids) in tables {
            let Some(slot) = two_digit_slot(&name) else {
                return Err(format!(
                    "table \"{name}\" is not a two-digit roster slot (01 to 23)"
                ));
            };
            for (key, id) in [("boots", ids.boots), ("gloves", ids.gloves)] {
                if let Some(id) = id
                    && !(101..=9999).contains(&id)
                {
                    return Err(format!(
                        "{name}.{key}: {id} is not an ID above 100 and at most 9999"
                    ));
                }
            }
            slots.insert(slot, ids);
        }
        Ok(LegacyIds { slots })
    }

    /// The old ID of `table`'s kind recorded for `slot`; `None` when the
    /// file gives him none of the kind.
    pub(crate) fn old_id(&self, slot: PlayerSlot, table: ItemTable) -> Option<u32> {
        let ids = self.slots.get(&slot)?;
        match table {
            ItemTable::Boots => ids.boots,
            ItemTable::Gloves => ids.gloves,
        }
        .map(u32::from)
    }
}

/// `name` as a two-digit roster slot (`"05"` is slot 5); `None` for any
/// other spelling.
fn two_digit_slot(name: &str) -> Option<PlayerSlot> {
    if name.len() != 2 || !name.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    name.parse().ok().and_then(PlayerSlot::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(number: u8) -> PlayerSlot {
        PlayerSlot::new(number).unwrap()
    }

    #[test]
    fn a_file_of_slot_tables_parses_and_a_comment_line_is_skipped() {
        let ids = LegacyIds::from_toml(
            "# The old export's boots and gloves IDs.\n\
            [05]\n\
            boots = 2305\n\
            gloves = 2310\n\
            [23]\n\
            boots = 101\n",
        )
        .unwrap();
        assert_eq!(ids.old_id(slot(5), ItemTable::Boots), Some(2305));
        assert_eq!(ids.old_id(slot(5), ItemTable::Gloves), Some(2310));
        assert_eq!(ids.old_id(slot(23), ItemTable::Boots), Some(101));
        assert_eq!(ids.old_id(slot(6), ItemTable::Boots), None);
        assert_eq!(ids.old_id(slot(23), ItemTable::Gloves), None);
    }

    #[test]
    fn a_table_name_that_is_not_a_two_digit_slot_is_refused() {
        for name in ["00", "24", "5", "005", "ab", "5x"] {
            let text = format!("[{name}]\nboots = 2305\n");
            assert!(
                LegacyIds::from_toml(&text).is_err(),
                "table [{name}] must be refused"
            );
        }
    }

    #[test]
    fn an_id_outside_the_old_id_band_is_refused() {
        for value in ["100", "10000", "\"2305\"", "1.5", "-1", "true"] {
            let text = format!("[05]\nboots = {value}\n");
            assert!(
                LegacyIds::from_toml(&text).is_err(),
                "boots = {value} must be refused"
            );
        }
    }

    #[test]
    fn an_unknown_key_and_a_top_level_value_are_refused() {
        for text in [
            "[05]\nboots = 2305\nextra = 1\n",
            "boots = 2305\n",
            "[05.b]\nboots = 2305\n",
            "not toml\n",
        ] {
            assert!(
                LegacyIds::from_toml(text).is_err(),
                "{text:?} must be refused"
            );
        }
    }
}
