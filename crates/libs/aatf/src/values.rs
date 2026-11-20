//! `TierValues` and `CardLimits`: what a file's `tier_values` and
//! `card_limits` answer for one player read as one tier, converted from
//! the script's maps, and the write of `TierValues` into a player record,
//! the lib's one writing operation (`spec/aatf_rules.md` "The host").

use std::collections::BTreeMap;

use pes_savefile::model::names::name_color;
use pes_savefile::model::player::PlayerEntry;
use serde::Deserialize;

use crate::ruleset::{Card, GroupId, Stat, WeakFoot};

/// The targets a player must carry as one tier, the specials applied:
/// per ability stat, form, injury resistance and weak foot, on the
/// game's scale.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TierValues {
    /// The target of every ability stat (the write skips those the
    /// version lacks).
    pub(crate) stats: BTreeMap<Stat, u8>,
    /// The target form (1 to 8).
    pub(crate) form: u8,
    /// The target injury resistance (1 to 3).
    pub(crate) injury_resistance: u8,
    /// The weak-foot limits (1 to 4 each).
    pub(crate) weak_foot: WeakFoot,
}

/// The card economy of a player as one tier.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardLimits {
    /// Paid cards the player holds now: his cards that no free group or
    /// free card covers.
    pub paid: u8,
    /// Paid cards allowed, the player's extra A positions deducted;
    /// below zero when they outnumber the cards.
    pub limit: i8,
    /// Free cards per group (`com` included), the specials' added.
    pub free: BTreeMap<GroupId, u8>,
    /// Cards that cost nothing.
    pub free_cards: Vec<Card>,
    /// Cards the player must hold.
    pub required_cards: Vec<Card>,
    /// A positions allowed, the registered one counted.
    pub a_positions: u8,
    /// The cap on A positions, when a special sets one.
    pub max_a_positions: Option<u8>,
}

impl TierValues {
    /// Writes the values into `entry`: every ability the version has at
    /// its target, form, injury resistance and weak foot at theirs. The
    /// record stores each of the last four one below the game's scale.
    pub(crate) fn write(&self, entry: &mut PlayerEntry) {
        for (stat, value) in &self.stats {
            entry.stats.set_ability(usize::from(stat.0), *value);
        }
        entry.stats.form = self.form.saturating_sub(1);
        entry.stats.injury_resistance = self.injury_resistance.saturating_sub(1);
        entry.stats.weak_foot_usage = self.weak_foot.usage.saturating_sub(1);
        entry.stats.weak_foot_accuracy = self.weak_foot.accuracy.saturating_sub(1);
    }
}

/// Writes `code` over `name`'s leading colour code — `\x11c` plus its
/// eight bytes replaced when one is there, the code prepended when not.
/// With `code` `None`, a leading code `tier_colors` holds (another
/// tier's) is removed and any other leading code kept; the rest of the
/// name is untouched either way. No length check: an over-long name
/// fails the save's write, as any name too long for its field does.
/// (`spec/aatf_rules.md` "The host".)
pub(crate) fn write_name_color(name: &mut String, code: Option<&str>, tier_colors: &[&str]) {
    let body = name_color(name).map_or(name.as_str(), |c| &name[2 + c.len()..]);
    match code {
        Some(code) => *name = format!("\u{11}c{code}{body}"),
        None => {
            if name_color(name).is_some_and(|c| tier_colors.contains(&c)) {
                *name = body.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four scaled values land one below the game's scale.
    #[test]
    fn the_write_stores_the_scaled_values_one_below() {
        let mut entry = PlayerEntry::default();
        let values = TierValues {
            stats: BTreeMap::new(),
            form: 8,
            injury_resistance: 3,
            weak_foot: WeakFoot {
                usage: 4,
                accuracy: 2,
            },
        };
        values.write(&mut entry);
        assert_eq!(entry.stats.form, 7);
        assert_eq!(entry.stats.injury_resistance, 2);
        assert_eq!(entry.stats.weak_foot_usage, 3);
        assert_eq!(entry.stats.weak_foot_accuracy, 1);
    }

    /// A PES 15 record has no physical contact: the write leaves it
    /// absent while every stat the version has takes its target.
    #[test]
    fn the_write_skips_the_stats_the_version_lacks() {
        let mut entry = PlayerEntry::bare(pes_version::PesVersion::Pes15);
        assert_ne!(entry.stats.abilities()[1], Some(77));
        let values = TierValues {
            stats: (0..26).map(|index| (Stat(index), 77)).collect(),
            form: 8,
            injury_resistance: 3,
            weak_foot: WeakFoot {
                usage: 4,
                accuracy: 2,
            },
        };
        values.write(&mut entry);
        assert_eq!(entry.stats.abilities()[1], Some(77));
        assert_eq!(entry.stats.abilities()[15], None);
    }
}
