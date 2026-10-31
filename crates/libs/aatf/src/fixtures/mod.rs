//! The per-ruleset fixtures (`spec/aatf_rules.md` "Development phase and
//! verification"): a known-legal team reports nothing, and each
//! known-violating fixture, a real team changed in its test by the
//! smallest edit that breaks one rule, reports exactly that rule's
//! violation. The base is the sample team (`sample/team.toml`: team 714
//! of the 2026 Summer Cup save, a PES 15 team legal under Spring 26, the
//! ruleset it was built for); each ruleset's module makes it legal under
//! its own rules first.

use std::collections::{BTreeMap, BTreeSet};

use pes_savefile::interchange::team_toml::TeamToml;
use pes_savefile::interchange::team_toml::labels::PLAYABLE_POSITIONS;
use pes_savefile::model::player::PlayerEntry;
use pes_savefile::model::team::TeamEntry;
use pes_version::PesVersion;

use crate::ruleset::{Card, Position, Severity, Stat, WeakFoot};
use crate::team::records_from_team_toml;
use crate::values::TierValues;
use crate::violation::ViolationKind;
use crate::{PlayerView, Rules, TeamView, Violation};

mod autumn25;
mod dbc7;
mod interpreter;
mod official;
mod spring26;
mod vgl25;
mod vgl26;
mod vgl27;
mod vtl10;
mod vtl11;

/// A team as records, what `apply_tier` writes into, viewed on demand.
struct Records {
    version: PesVersion,
    team: TeamEntry,
    players: Vec<PlayerEntry>,
}

impl Records {
    /// The sample team's records.
    fn sample() -> Records {
        let document = TeamToml::parse(include_str!("../../sample/team.toml"))
            .expect("the sample team parses");
        let (version, team, players) =
            records_from_team_toml(&document).expect("the sample team builds records");
        Records {
            version,
            team,
            players,
        }
    }

    /// The sample team's records on `version`: a Team TOML document is
    /// version-neutral, so setting its `pes_version` before the build
    /// gives the same team on another version (the VGL cups play on
    /// PES 21, whose presets can be fluid; PES 15's cannot).
    fn sample_on(version: PesVersion) -> Records {
        let mut document = TeamToml::parse(include_str!("../../sample/team.toml"))
            .expect("the sample team parses");
        document.pes_version = Some(version);
        let (version, team, players) = records_from_team_toml(&document)
            .expect("the sample team builds records on the version");
        Records {
            version,
            team,
            players,
        }
    }

    /// The records' view.
    fn view(&self) -> TeamView {
        TeamView::view(self.version, &self.team, Vec::new(), |id| {
            self.players.iter().find(|player| player.id == id)
        })
        .expect("the records view")
    }

    /// The record of roster `slot`.
    fn player_mut(&mut self, slot: u8) -> &mut PlayerEntry {
        let id = self.team.roster[usize::from(slot)].player_id;
        self.players
            .iter_mut()
            .find(|player| player.id == id)
            .unwrap_or_else(|| panic!("roster slot {slot} is filled"))
    }
}

/// `entry` at a tier's targets: every ability at `rate`, his form and
/// injury resistance set, weak foot kept (through `TierValues::write`,
/// which the fixtures then exercise too).
fn set_tier_record(entry: &mut PlayerEntry, rate: u8, form: u8, injury_resistance: u8) {
    TierValues {
        stats: (0..25u8).map(|index| (Stat(index), rate)).collect(),
        form,
        injury_resistance,
        weak_foot: WeakFoot {
            usage: entry.stats.weak_foot_usage + 1,
            accuracy: entry.stats.weak_foot_accuracy + 1,
        },
    }
    .write(entry);
}

/// The violations of `team` under `rules` that are not suggestions: what
/// a member must fix.
fn problems(rules: &Rules, team: &TeamView) -> Vec<Violation> {
    rules
        .check_team(team)
        .expect("the check runs")
        .into_iter()
        .filter(|violation| violation.severity != Severity::Suggestion)
        .collect()
}

/// The player at roster `slot`.
fn player(team: &TeamView, slot: u8) -> &PlayerView {
    team.players
        .iter()
        .find(|player| player.slot == slot)
        .unwrap_or_else(|| panic!("roster slot {slot} is filled"))
}

/// The id of the tier `rules` recognizes roster `slot`'s player as
/// (`Rules::player_tier`), `None` for no tier.
fn tier_of<'r>(rules: &'r Rules, team: &TeamView, slot: u8) -> Option<&'r str> {
    rules
        .player_tier(team, player(team, slot))
        .expect("the call runs")
        .map(|tier| tier.id.as_str())
}

/// The player at roster `slot`, to edit.
fn player_mut(team: &mut TeamView, slot: u8) -> &mut PlayerView {
    team.players
        .iter_mut()
        .find(|player| player.slot == slot)
        .unwrap_or_else(|| panic!("roster slot {slot} is filled"))
}

/// `player` at a tier's targets: every ability the version has at `rate`,
/// his form and injury resistance set; cards and weak foot untouched.
fn set_tier(player: &mut PlayerView, rate: u8, form: u8, injury_resistance: u8) {
    for ability in player.abilities.iter_mut().flatten() {
        *ability = rate;
    }
    player.form = form;
    player.injury_resistance = injury_resistance;
}

/// The suggestions alone, for roster `slot`, as kinds.
fn suggestions_for(rules: &Rules, team: &TeamView, slot: u8) -> Vec<ViolationKind> {
    rules
        .check_team(team)
        .expect("the check runs")
        .into_iter()
        .filter(|violation| violation.severity == Severity::Suggestion)
        .filter(|violation| violation.slot == Some(slot))
        .map(|violation| violation.kind)
        .collect()
}

/// An error-severity violation on roster `slot` (`None`: the team).
fn error(slot: Option<u8>, kind: ViolationKind) -> Violation {
    Violation {
        slot,
        severity: Severity::Error,
        kind,
    }
}

/// A warning on roster `slot`.
fn warning(slot: u8, kind: ViolationKind) -> Violation {
    Violation {
        slot: Some(slot),
        severity: Severity::Warning,
        kind,
    }
}

/// A `BracketQuota` of height system `system`, `players` its players as
/// (roster slot, height read).
fn quota(system: &str, bracket: &str, count: u8, quota: u8, players: &[(u8, u8)]) -> Violation {
    error(
        None,
        ViolationKind::BracketQuota {
            system: system.to_string(),
            bracket: bracket.to_string(),
            count,
            quota,
            players: players.to_vec(),
        },
    )
}

/// A `Height` violation's kind: `height` in no bracket, `fits` the
/// heights it could take, `room` whether they are the brackets with room.
fn no_bracket(height: u8, fits: &[(u8, u8)], room: bool) -> ViolationKind {
    ViolationKind::Height {
        height,
        fits: fits.to_vec(),
        room,
    }
}

/// A `Custom` violation on roster `slot` (`None`: the team).
fn custom(slot: Option<u8>, text: &str) -> Violation {
    error(
        slot,
        ViolationKind::Custom {
            text: text.to_string(),
        },
    )
}

/// An `Unused` suggestion's kind.
fn unused(allowance: &str, used: u8, allowed: u8) -> ViolationKind {
    ViolationKind::Unused {
        allowance: allowance.to_string(),
        used,
        allowed,
    }
}

/// The card `name` names.
fn card(name: &str) -> Card {
    Card::try_from(name.to_string()).expect("a card name")
}

/// The position `name` names.
fn position(name: &str) -> Position {
    Position::try_from(name.to_string()).expect("a position name")
}

/// The ability stat `name` names.
fn stat(name: &str) -> Stat {
    Stat::try_from(name.to_string()).expect("a stat name")
}

/// A `Special` violation's kind.
fn special(label: &str, text: &str) -> ViolationKind {
    ViolationKind::Special {
        label: label.to_string(),
        text: text.to_string(),
    }
}

/// The index in `entry`'s `playable` table (the save's order, not the
/// position table's) of position `name`.
fn playable_at(name: &str) -> usize {
    PLAYABLE_POSITIONS
        .iter()
        .position(|index| *index == usize::from(position(name).0))
        .expect("a position")
}

/// One tier's economy on one member, as `card_limits` and `tier_values`
/// report it: (roster slot, tier id, paid limit, A positions, the exact
/// free counts, weak foot usage and accuracy). `max_a_positions` is
/// expected unset.
type Economy<'a> = (u8, &'a str, i8, u8, &'a [(&'a str, u8)], (u8, u8));

/// Asserts each row of `rows` against `rules` on `team`: the `free` map
/// compared whole, so a group the row does not name fails too.
fn assert_economy(rules: &Rules, team: &TeamView, rows: &[Economy]) {
    for &(slot, tier, limit, a_positions, expected_free, (usage, accuracy)) in rows {
        let limits = rules
            .card_limits(team, team.player(slot).expect("filled"), tier)
            .expect("limits");
        assert_eq!(
            (limits.limit, limits.a_positions, limits.max_a_positions),
            (limit, a_positions, None),
            "{tier} (slot {slot})"
        );
        let expected_free: BTreeMap<String, u8> = expected_free
            .iter()
            .map(|(group, count)| (group.to_string(), *count))
            .collect();
        assert_eq!(limits.free, expected_free, "{tier} (slot {slot})");
        let weak_foot = rules
            .tier_values(team, team.player(slot).expect("filled"), tier)
            .expect("values")
            .weak_foot;
        assert_eq!(
            (weak_foot.usage, weak_foot.accuracy),
            (usage, accuracy),
            "{tier} (slot {slot})"
        );
    }
}

/// `player_tier` answers `None` on exactly the slots `check_team`
/// reports `NoTier` for: every embedded data block, the sample team on
/// three versions, every slot.
#[test]
fn player_tier_is_none_exactly_where_check_team_reports_no_tier() {
    let (mut none, mut some) = (0usize, 0usize);
    for (name, block) in [
        ("autumn26", include_str!("../../rulesets/autumn26.rhai")),
        ("autumn25", include_str!("../../rulesets/autumn25.rhai")),
        ("spring26", include_str!("../../rulesets/spring26.rhai")),
        ("vgl25", include_str!("../../rulesets/vgl25.rhai")),
        ("vgl26", include_str!("../../rulesets/vgl26.rhai")),
        ("vgl27", include_str!("../../rulesets/vgl27.rhai")),
        ("vtl10", include_str!("../../rulesets/vtl10.rhai")),
        ("vtl11", include_str!("../../rulesets/vtl11.rhai")),
        ("dbc7", include_str!("../../rulesets/dbc7.rhai")),
    ] {
        let rules = Rules::from_data_block(block).unwrap_or_else(|e| panic!("{name}: {e}"));
        for version in [PesVersion::Pes15, PesVersion::Pes17, PesVersion::Pes21] {
            let team = Records::sample_on(version).view();
            let reported: BTreeSet<u8> = rules
                .check_team(&team)
                .unwrap_or_else(|e| panic!("{name} {version:?}: {e}"))
                .iter()
                .filter_map(|v| {
                    if matches!(v.kind, ViolationKind::NoTier { .. }) {
                        v.slot
                    } else {
                        None
                    }
                })
                .collect();
            let mut unanswered = BTreeSet::new();
            for p in &team.players {
                if tier_of(&rules, &team, p.slot).is_some() {
                    some += 1;
                } else {
                    none += 1;
                    unanswered.insert(p.slot);
                }
            }
            assert_eq!(reported, unanswered, "{name} {version:?}");
        }
    }
    assert!(none > 0 && some > 0, "{none} `None`, {some} `Some`");
}
