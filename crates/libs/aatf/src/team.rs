//! `TeamView`/`PlayerView`: the host's read of one team of a save, every
//! value on the game's own scale, plus the tactics answers
//! (`fielded_positions`, `fielded_only_at`, `formation_shapes`).

use pes_savefile::EditFile;
use pes_savefile::interchange::team_toml::labels::{PLAYABLE_POSITIONS, POSITIONS};
use pes_savefile::interchange::team_toml::{ImportNote, TeamToml};
use pes_savefile::model::names::name_color;
use pes_savefile::model::player::PlayerEntry;
use pes_savefile::model::playstyle::PlayStyle as SavePlayStyle;
use pes_savefile::model::tactics::{Formation, TeamTactics};
use pes_savefile::model::team::RosterSlot;
use pes_savefile::model::team::TeamEntry;
use pes_savefile::schema::playstyle;
use pes_savefile::settings_toml::AppearanceSettings;
use pes_version::PesVersion;

use crate::ruleset::{Card, Foot, PlayStyle, Position, Stat};

/// One roster player as a ruleset reads them, every value on the game's
/// scale (form 1 to 8, injury resistance 1 to 3, weak foot 1 to 4, where
/// the save stores each less one).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerView {
    /// The roster slot.
    pub(crate) slot: u8,
    /// Age in years.
    pub(crate) age: u8,
    /// Height in cm.
    pub(crate) height: u8,
    /// Weight in kg.
    pub(crate) weight: u8,
    /// Form, 1 to 8.
    pub(crate) form: u8,
    /// Injury resistance, 1 to 3.
    pub(crate) injury_resistance: u8,
    /// Weak foot usage, 1 to 4.
    pub(crate) weak_foot_usage: u8,
    /// Weak foot accuracy, 1 to 4.
    pub(crate) weak_foot_accuracy: u8,
    /// The registered position (`None` past the labels).
    pub(crate) position: Option<Position>,
    /// The playing style (`None` when the version's table has no such
    /// style).
    pub(crate) playing_style: Option<PlayStyle>,
    /// The stronger foot (`None` for a byte past 0/1).
    pub(crate) strong_foot: Option<Foot>,
    /// The name's leading colour code (`cc9900ff`).
    pub(crate) name_color: Option<String>,
    /// The held cards: the skills in table order, then the COM styles.
    pub(crate) cards: Vec<Card>,
    /// The ability stats (`labels::ABILITY_STATS`' order, indexed by
    /// `Stat`), `None` where the version lacks the stat.
    pub(crate) abilities: [Option<u8>; 25],
    /// The playable-position ratings, `Position`-indexed (GK 0 to CF 12)
    /// on the save's scale: C 0, B 1, A 2.
    pub(crate) ratings: [u8; 13],
}

impl PlayerView {
    /// The ability stat `name` names (`None` for a stat the version lacks).
    pub(crate) fn stat_by_name(&self, name: &str) -> Result<Option<u8>, String> {
        Stat::try_from(name.to_string()).map(|stat| self.abilities[usize::from(stat.0)])
    }

    /// The playable rating at `pos` on the save's scale (C 0, B 1, A 2).
    pub(crate) fn rating_at(&self, pos: &str) -> Result<u8, String> {
        Position::try_from(pos.to_string()).map(|pos| self.ratings[usize::from(pos.0)])
    }

    /// `entry`'s view at roster `slot` on `version`.
    pub(crate) fn of_entry(version: PesVersion, slot: u8, entry: &PlayerEntry) -> PlayerView {
        // A style byte past the version's table is no style (`()` in a
        // script), not an error: the logic reports it as out of range.
        let playing_style = playstyle::decode(version, entry.positions.playing_style)
            .ok()
            .and_then(|style| {
                SavePlayStyle::ALL
                    .iter()
                    .position(|s| *s == style)
                    .map(|i| PlayStyle(u8::try_from(i).expect("22 styles")))
            });
        let mut ratings = [0u8; 13];
        for (i, rating) in entry.positions.playable.iter().enumerate() {
            ratings[PLAYABLE_POSITIONS[i]] = *rating;
        }
        PlayerView {
            slot,
            age: entry.basic.age,
            height: entry.basic.height,
            weight: entry.basic.weight,
            form: entry.stats.form.saturating_add(1),
            injury_resistance: entry.stats.injury_resistance.saturating_add(1),
            weak_foot_usage: entry.stats.weak_foot_usage.saturating_add(1),
            weak_foot_accuracy: entry.stats.weak_foot_accuracy.saturating_add(1),
            position: (usize::from(entry.positions.registered) < POSITIONS.len())
                .then_some(Position(entry.positions.registered)),
            playing_style,
            strong_foot: match entry.positions.stronger_foot {
                0 => Some(Foot::Right),
                1 => Some(Foot::Left),
                _ => None,
            },
            name_color: name_color(&entry.name).map(str::to_string),
            cards: entry
                .skills
                .skills
                .iter()
                .enumerate()
                .filter(|(_, held)| **held)
                .map(|(i, _)| Card::Skill(u8::try_from(i).expect("41 skills")))
                .chain(
                    entry
                        .skills
                        .com_styles
                        .iter()
                        .enumerate()
                        .filter(|(_, held)| **held)
                        .map(|(i, _)| Card::ComStyle(u8::try_from(i).expect("7 styles"))),
                )
                .collect(),
            abilities: entry.stats.abilities(),
            ratings,
        }
    }
}

/// One team of a save as a ruleset reads it: its roster's players and its
/// tactics.
#[derive(Debug, Clone, PartialEq)]
pub struct TeamView {
    /// The save's version.
    pub(crate) version: PesVersion,
    /// The roster's players in slot order, empty slots left out.
    pub(crate) players: Vec<PlayerView>,
    /// The captain's roster slot.
    pub(crate) captain: Option<u8>,
    /// The team's tactics (starting eleven, presets, formations).
    pub(crate) tactics: TeamTactics,
    /// The roster slots whose player is not the roster-order rule's, as
    /// `TeamEntry::roster_order_breaks` reports them (`(slot, player_id)`,
    /// 0-based): the host's `RosterOrder` check reads this. `new` fills it
    /// from the save's team record; `from_team_toml` leaves it empty (a
    /// Team TOML holds no ids).
    pub(crate) roster_breaks: Vec<(u8, u32)>,
}

/// Why a team cannot be viewed.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TeamViewError {
    /// The save holds no team with this id.
    #[error("unknown team {team_id}")]
    UnknownTeam {
        /// The id looked up.
        team_id: u32,
    },
    /// A roster slot names a player the save does not hold. The text names
    /// the place as the player list numbers it, from 01.
    #[error(
        "place {:02} of the roster names player {player_id}, whom the save does not hold",
        u32::from(*.slot) + 1
    )]
    MissingPlayer {
        /// The roster slot, numbered from 0.
        slot: u8,
        /// The missing player id.
        player_id: u32,
    },
    /// A formation slot's position byte is past the position labels.
    #[error(
        "preset {preset} formation {formation} slot {slot}: position byte {byte} is past the labels"
    )]
    FormationPosition {
        /// The preset, numbered from 1.
        preset: u8,
        /// The formation within it, numbered from 1.
        formation: u8,
        /// The formation slot (0 to 10).
        slot: u8,
        /// The position byte.
        byte: u8,
    },
    /// The document names no `pes_version` to build the team on.
    #[error("the document names no pes_version")]
    NoVersion,
    /// The document does not apply onto bare records: `message` is the
    /// `TeamTomlError`'s text.
    #[error("the document does not apply: {message}")]
    TeamToml {
        /// The error's text.
        message: String,
    },
    /// The document applies only with a change (a key the version lacks, a
    /// value it cannot store): the first import note, which the tool words.
    #[error("the document does not apply as written")]
    ImportNote {
        /// The first note `apply` returned.
        note: ImportNote,
    },
}

/// The defender/midfielder/forward counts of one formation the game can
/// field, presets and formations numbered from 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FormationShape {
    /// The preset (1 to 3).
    pub(crate) preset: u8,
    /// The formation within it (1 to 3).
    pub(crate) formation: u8,
    /// `[defenders, midfielders, forwards]`; GK counts in no group.
    pub(crate) shape: [u8; 3],
}

/// One slot of a formation the game can field: the starting-eleven
/// player's roster slot, the position and the coordinates as stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FormationSlotView {
    /// The roster slot of the starting-eleven player the slot fields.
    pub(crate) slot: u8,
    /// The slot's position.
    pub(crate) position: Position,
    /// The horizontal coordinate, as stored.
    pub(crate) x: u8,
    /// The vertical coordinate, as stored.
    pub(crate) y: u8,
}

impl TeamView {
    /// The team `team_id` of `file`.
    pub fn new(file: &EditFile, team_id: u32) -> Result<TeamView, TeamViewError> {
        let team = file
            .team(team_id)
            .ok_or(TeamViewError::UnknownTeam { team_id })?;
        Self::view(file.version(), team, team.roster_order_breaks(), |id| {
            file.player(id)
        })
    }

    /// The team a Team TOML document describes, on the version its
    /// `pes_version` names: every `players` key one bare slot.
    pub fn from_team_toml(document: &TeamToml) -> Result<TeamView, TeamViewError> {
        let (version, team, players) = records_from_team_toml(document)?;
        Self::view(version, &team, Vec::new(), |id| {
            players.iter().find(|player| player.id == id)
        })
    }

    /// The player at roster `slot`, if the roster fills it.
    pub fn player(&self, slot: u8) -> Option<&PlayerView> {
        self.players.iter().find(|player| player.slot == slot)
    }

    /// The builder `new` and `from_team_toml` share: a version, a
    /// `&TeamEntry`, its roster-order breaks (`Vec::new()` where the
    /// records came from no save) and a player lookup by id.
    pub(crate) fn view<'a>(
        version: PesVersion,
        team: &'a TeamEntry,
        roster_breaks: Vec<(u8, u32)>,
        player: impl Fn(u32) -> Option<&'a PlayerEntry>,
    ) -> Result<TeamView, TeamViewError> {
        // Every formation's position bytes are checked here so the
        // tactics answers cannot fail on one.
        for (p, preset) in team.tactics.presets.iter().enumerate() {
            for (f, formation) in preset.formations.iter().enumerate() {
                for (s, slot) in formation.players.iter().enumerate() {
                    if usize::from(slot.position) >= POSITIONS.len() {
                        return Err(TeamViewError::FormationPosition {
                            preset: u8::try_from(p + 1).expect("three presets"),
                            formation: u8::try_from(f + 1).expect("three formations"),
                            slot: u8::try_from(s).expect("eleven slots"),
                            byte: slot.position,
                        });
                    }
                }
            }
        }
        let mut players = Vec::new();
        for (i, slot) in team.roster.iter().enumerate() {
            if slot.player_id == 0 {
                continue;
            }
            let entry = player(slot.player_id).ok_or(TeamViewError::MissingPlayer {
                slot: u8::try_from(i).expect("a roster holds fewer than 256 slots"),
                player_id: slot.player_id,
            })?;
            players.push(PlayerView::of_entry(
                version,
                u8::try_from(i).expect("a roster holds fewer than 256 slots"),
                entry,
            ));
        }
        let captain =
            (team.tactics.set_pieces.captain != 0xFF).then_some(team.tactics.set_pieces.captain);
        Ok(TeamView {
            version,
            players,
            captain,
            tactics: team.tactics.clone(),
            roster_breaks,
        })
    }
}

/// The records a Team TOML document describes, on the version its
/// `pes_version` names: the team and one bare `PlayerEntry` per
/// `players` key (its id the key), the document applied onto them.
pub(crate) fn records_from_team_toml(
    document: &TeamToml,
) -> Result<(PesVersion, TeamEntry, Vec<PlayerEntry>), TeamViewError> {
    let version = document.pes_version.ok_or(TeamViewError::NoVersion)?;
    // Appearance is not applied: no ruleset reads it, and it lives
    // only over a real player's face data.
    let mut document = document.clone();
    for section in document.players.values_mut() {
        section.appearance = AppearanceSettings::default();
    }
    // Bare records: one roster slot per index up to the highest
    // `players` key, `i + 1` where the key is present, and one bare
    // `PlayerEntry` per present slot. The bare records carry every
    // field the version has at zero (a field naming a roster slot at
    // 0xFF, none), so a key the document omits reads 0 like any other
    // field, never as a field the version lacks.
    let mut team = TeamEntry {
        tactics: TeamTactics::bare(version),
        ..TeamEntry::default()
    };
    let highest = document.players.keys().max().copied().unwrap_or(0);
    team.roster = (1..=highest)
        .map(|slot| RosterSlot {
            player_id: if document.players.contains_key(&slot) {
                u32::from(slot)
            } else {
                0
            },
            ..Default::default()
        })
        .collect();
    let mut players: Vec<PlayerEntry> = document
        .players
        .keys()
        .map(|slot| PlayerEntry {
            id: u32::from(*slot),
            ..PlayerEntry::bare(version)
        })
        .collect();
    let notes = document
        .apply(version, &mut team, &mut players)
        .map_err(|e| TeamViewError::TeamToml {
            message: e.to_string(),
        })?;
    if let Some(note) = notes.into_iter().next() {
        return Err(TeamViewError::ImportNote { note });
    }
    Ok((version, team, players))
}

impl TeamView {
    /// The sample team `Rules::load` step 3 runs `check_team` on: a
    /// real, anonymized cup team as a Team TOML document
    /// (`sample/team.toml`).
    pub(crate) fn sample_team() -> TeamView {
        TeamView::from_team_toml(
            &TeamToml::parse(include_str!("../sample/team.toml")).expect("the sample team parses"),
        )
        .expect("the sample team builds a view")
    }

    /// The formations `preset` (1 to 3) can field, as
    /// `(formation number, formation)` numbered from 1: all three when
    /// fluid, the kick-off one otherwise.
    fn fielded_formations(&self, preset: u8) -> Vec<(u8, &Formation)> {
        let preset_ref = &self.tactics.presets[usize::from(preset) - 1];
        preset_ref
            .fielded_formations()
            .iter()
            .map(|&f| {
                (
                    u8::try_from(f + 1).expect("three formations"),
                    &preset_ref.formations[f],
                )
            })
            .collect()
    }

    /// The positions the starting-eleven player at roster `slot` is
    /// fielded at in `preset`'s (1 to 3) fielded formations, once each,
    /// in the position table's order (GK first, CF last); empty for a
    /// player outside the starting eleven.
    pub(crate) fn fielded_positions(&self, slot: u8, preset: u8) -> Vec<Position> {
        let mut positions = Vec::new();
        for (_, formation) in self.fielded_formations(preset) {
            for (i, &starter) in self.tactics.starting_eleven.iter().enumerate() {
                if starter == slot {
                    let position = Position(formation.players[i].position);
                    if !positions.contains(&position) {
                        positions.push(position);
                    }
                }
            }
        }
        positions.sort();
        positions
    }

    /// Whether every formation the game can field fields the player at
    /// `position`; for a player outside the starting eleven, whether
    /// `position` is his registered one.
    pub(crate) fn fielded_only_at(&self, slot: u8, position: Position) -> bool {
        if !self.tactics.starting_eleven.contains(&slot) {
            return self
                .players
                .iter()
                .find(|player| player.slot == slot)
                .is_some_and(|player| player.position == Some(position));
        }
        for preset in 1..=3 {
            for (_, formation) in self.fielded_formations(preset) {
                for (i, &starter) in self.tactics.starting_eleven.iter().enumerate() {
                    if starter == slot && Position(formation.players[i].position) != position {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// The eleven slots of `preset`'s fielded `formation` (both numbered
    /// from 1), `None` for a formation the preset does not field (a
    /// non-fluid preset's second and third included).
    pub(crate) fn formation(&self, preset: u8, formation: u8) -> Option<Vec<FormationSlotView>> {
        let (_, fielded) = self
            .fielded_formations(preset)
            .into_iter()
            .find(|(number, _)| *number == formation)?;
        Some(
            fielded
                .players
                .iter()
                .zip(self.tactics.starting_eleven)
                .map(|(slot, starter)| FormationSlotView {
                    slot: starter,
                    position: Position(slot.position),
                    x: slot.x,
                    y: slot.y,
                })
                .collect(),
        )
    }

    /// Whether any of the three presets has a man-marking assignment set:
    /// any of its eleven bytes not 0xFF (the game's none).
    pub(crate) fn man_marking(&self) -> bool {
        self.tactics
            .presets
            .iter()
            .any(|preset| preset.man_marking.iter().any(|&marked| marked != 0xFF))
    }

    /// One `FormationShape` per formation the game can field, in preset
    /// then formation order.
    pub(crate) fn formation_shapes(&self) -> Vec<FormationShape> {
        let mut out = Vec::new();
        for p in 1..=3u8 {
            for (f, formation) in self.fielded_formations(p) {
                let mut shape = [0u8; 3];
                for slot in &formation.players {
                    // `POSITIONS` bytes: CB, LB, RB; DMF, CMF, LMF, RMF,
                    // AMF; LWF, RWF, SS, CF. `new` refused any byte past
                    // them, so `_` is the GK.
                    match slot.position {
                        1..=3 => shape[0] += 1,
                        4..=8 => shape[1] += 1,
                        9..=12 => shape[2] += 1,
                        _ => {}
                    }
                }
                out.push(FormationShape {
                    preset: p,
                    formation: f,
                    shape,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pes_savefile::fixture_save;
    use pes_savefile::model::TeamEntry;

    use crate::Rules;
    use crate::ruleset::Severity;
    use crate::violation::{Violation, ViolationKind};

    /// `version`'s fixture save and its team 701's view.
    fn view(version: PesVersion) -> (EditFile, TeamView) {
        let file = EditFile::from_bytes(&fixture_save(version)).expect("fixture save");
        let team = TeamView::new(&file, 701).expect("team 701 views");
        (file, team)
    }

    /// `file`'s team 701, mutable.
    fn team_mut(file: &mut EditFile) -> &mut TeamEntry {
        file.teams_mut()
            .iter_mut()
            .find(|team| team.id == 701)
            .expect("team 701")
    }

    #[test]
    fn an_unknown_team_is_refused() {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes19)).expect("fixture save");
        assert_eq!(
            TeamView::new(&file, 9999).expect_err("no such team"),
            TeamViewError::UnknownTeam { team_id: 9999 }
        );
    }

    #[test]
    fn a_roster_slot_naming_no_player_is_refused() {
        let mut file =
            EditFile::from_bytes(&fixture_save(PesVersion::Pes19)).expect("fixture save");
        assert_ne!(team_mut(&mut file).roster[5].player_id, 9999);
        team_mut(&mut file).roster[5].player_id = 9999;
        let error = TeamView::new(&file, 701).expect_err("a missing player");
        assert_eq!(
            error,
            TeamViewError::MissingPlayer {
                slot: 5,
                player_id: 9999
            }
        );
        assert_eq!(
            error.to_string(),
            "place 06 of the roster names player 9999, whom the save does not hold"
        );
    }

    #[test]
    fn a_formation_byte_past_the_labels_is_refused() {
        let mut file =
            EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).expect("fixture save");
        team_mut(&mut file).tactics.presets[0].formations[2].players[5].position = 13;
        assert_eq!(
            TeamView::new(&file, 701).expect_err("a position past the labels"),
            TeamViewError::FormationPosition {
                preset: 1,
                formation: 3,
                slot: 5,
                byte: 13
            }
        );
    }

    #[test]
    fn the_roster_views_in_slot_order() {
        let (_, team) = view(PesVersion::Pes19);
        assert_eq!(team.players.len(), 23);
        assert_eq!(
            team.players.iter().map(|p| p.slot).collect::<Vec<_>>(),
            (0..23).collect::<Vec<u8>>()
        );
    }

    /// The roster-order rule is the host's own check, leading its slot's
    /// rows in `check_team`'s list (`spec/aatf_rules.md` "The host"): team
    /// 701 keeps it, two slots' player ids swapped raise one error each.
    #[test]
    fn roster_order_breaks_report_first() {
        let rules = Rules::official();
        let (mut file, team) = view(PesVersion::Pes21);
        assert!(team.roster_breaks.is_empty(), "team 701 keeps the order");
        let found = rules.check_team(&team).expect("the check runs");
        assert!(
            !found
                .iter()
                .any(|v| matches!(v.kind, ViolationKind::RosterOrder { .. })),
            "no RosterOrder on an ordered roster: {found:?}"
        );
        {
            let roster = &mut team_mut(&mut file).roster;
            // Premise: slots 0 and 1 hold the players the rule puts there.
            assert_eq!(roster[0].player_id, 70101);
            assert_eq!(roster[1].player_id, 70102);
            (roster[0].player_id, roster[1].player_id) = (70102, 70101);
        }
        let team = TeamView::new(&file, 701).expect("the team views");
        assert_eq!(team.roster_breaks, vec![(0, 70102), (1, 70101)]);
        let found = rules.check_team(&team).expect("the check runs");
        // Each break is the first violation of its own slot's rows.
        for (slot, player) in [(0u8, 70102u32), (1, 70101)] {
            let first = found
                .iter()
                .find(|v| v.slot == Some(slot))
                .unwrap_or_else(|| panic!("slot {slot} has rows"));
            assert_eq!(
                *first,
                Violation {
                    slot: Some(slot),
                    severity: Severity::Error,
                    kind: ViolationKind::RosterOrder { player },
                },
                "the break leads slot {slot}'s rows"
            );
        }
        assert_eq!(
            found
                .iter()
                .filter(|v| matches!(v.kind, ViolationKind::RosterOrder { .. }))
                .count(),
            2,
            "one error per breaking slot: {found:?}"
        );
    }

    #[test]
    fn fielded_positions_reads_only_fielded_formations() {
        let (mut file, team) = view(PesVersion::Pes19);
        // Slot 8 sits at formation slot 8, AMF (byte 8) everywhere.
        assert_eq!(team.tactics.presets[0].formations[1].players[8].position, 8);
        assert_eq!(team.tactics.presets[0].formations[2].players[8].position, 8);
        // Non-fluid: only the kick-off formation counts.
        assert_eq!(team.fielded_positions(8, 1), vec![Position(8)]);
        // Formations 2 and 3 both put him at LMF: the position reports
        // once, not twice.
        team_mut(&mut file).tactics.presets[0].formations[1].players[8].position = 6;
        team_mut(&mut file).tactics.presets[0].formations[2].players[8].position = 6;
        let team = TeamView::new(&file, 701).expect("the team views");
        assert_eq!(team.fielded_positions(8, 1), vec![Position(8)]);
        // The same preset made fluid: all three formations, each
        // position once (LMF 6 twice becomes once), ascending.
        team_mut(&mut file).tactics.presets[0].style.fluid = Some(true);
        let team = TeamView::new(&file, 701).expect("the team views");
        assert_eq!(team.fielded_positions(8, 1), vec![Position(6), Position(8)]);
        // A substitute: nothing.
        assert!(!team.tactics.starting_eleven.contains(&20));
        assert_eq!(team.fielded_positions(20, 1), Vec::<Position>::new());
    }

    #[test]
    fn fielded_only_at_needs_every_fielded_formation() {
        let (mut file, team) = view(PesVersion::Pes19);
        // UTAH TEAPOT (slot 10) is the CF of every formation.
        let cf = Position(12);
        assert!(team.fielded_only_at(10, cf));
        // A fielded formation moved: made fluid, formation 3 moves him
        // to SS (byte 11).
        team_mut(&mut file).tactics.presets[0].style.fluid = Some(true);
        team_mut(&mut file).tactics.presets[0].formations[2].players[10].position = 11;
        let team = TeamView::new(&file, 701).expect("the team views");
        assert!(!team.fielded_only_at(10, cf));
        // A non-fielded formation moved (a non-fluid preset's formation
        // 2): still true.
        let (mut file, _) = view(PesVersion::Pes19);
        team_mut(&mut file).tactics.presets[0].formations[1].players[10].position = 11;
        let team = TeamView::new(&file, 701).expect("the team views");
        assert!(team.fielded_only_at(10, cf));
        // Preset 3's kick-off formation: fielded even though preset 3 is
        // not fluid — moving him there breaks it too.
        let (mut file, _) = view(PesVersion::Pes19);
        assert_eq!(
            team_mut(&mut file).tactics.presets[2].style.fluid,
            Some(false)
        );
        team_mut(&mut file).tactics.presets[2].formations[0].players[10].position = 11;
        let team = TeamView::new(&file, 701).expect("the team views");
        assert!(!team.fielded_only_at(10, cf));
        // A substitute: his registered position, nothing else.
        assert!(!team.tactics.starting_eleven.contains(&20));
        let entry = file.player(70121).expect("player 70121");
        let registered_pos = Position(entry.positions.registered);
        let elsewhere = Position((entry.positions.registered + 1) % 13);
        assert!(team.fielded_only_at(20, registered_pos));
        assert!(!team.fielded_only_at(20, elsewhere));
    }

    #[test]
    fn formation_shapes_counts_every_fielded_formation() {
        let (mut file, team) = view(PesVersion::Pes17);
        // Non-fluid everywhere: one formation per preset.
        assert_eq!(team.formation_shapes().len(), 3);
        // Team 701's kick-off: GK, two CB, RB, LB, two DMF, RMF, LMF,
        // AMF, CF — GK counts in no group.
        assert_eq!(
            team.formation_shapes()[0],
            FormationShape {
                preset: 1,
                formation: 1,
                shape: [4, 5, 1]
            }
        );
        // A fluid preset adds its three formations.
        team_mut(&mut file).tactics.presets[0].style.fluid = Some(true);
        let team = TeamView::new(&file, 701).expect("the team views");
        assert_eq!(team.formation_shapes().len(), 5);
        assert_eq!(
            team.formation_shapes()[2],
            FormationShape {
                preset: 1,
                formation: 3,
                shape: [4, 5, 1]
            }
        );
    }

    #[test]
    fn from_team_toml_matches_new_on_every_version() {
        for version in PesVersion::ALL {
            let file = EditFile::from_bytes(&fixture_save(version)).expect("fixture save");
            let team = file.team(701).expect("team 701");
            let players: Vec<&PlayerEntry> = team
                .roster
                .iter()
                .filter(|slot| slot.player_id != 0)
                .map(|slot| file.player(slot.player_id).expect("a rostered player"))
                .collect();
            let document =
                TeamToml::from_team(version, team, &players).expect("from_team emits a document");
            let built = TeamView::from_team_toml(&document)
                .unwrap_or_else(|e| panic!("{version:?}: the document builds: {e}"));
            assert_eq!(
                built,
                TeamView::new(&file, 701).expect("the team views"),
                "{version:?}"
            );
        }
    }

    /// An interior empty roster slot stays empty through the document:
    /// the players after it keep their slot numbers and the tactics'
    /// slot links (the starting eleven) still name them.
    #[test]
    fn from_team_toml_matches_new_with_an_empty_interior_slot() {
        for version in PesVersion::ALL {
            let mut file = EditFile::from_bytes(&fixture_save(version)).expect("fixture save");
            // The second filled roster slot: an interior hole, never
            // the first or the last occupied one.
            let hole = {
                let occupied: Vec<usize> = team_mut(&mut file)
                    .roster
                    .iter()
                    .enumerate()
                    .filter(|(_, slot)| slot.player_id != 0)
                    .map(|(index, _)| index)
                    .collect();
                assert!(
                    occupied.len() >= 3,
                    "{version:?}: room for an interior hole"
                );
                occupied[1]
            };
            team_mut(&mut file).roster[hole].player_id = 0;
            let team = file.team(701).expect("team 701");
            let players: Vec<&PlayerEntry> = team
                .roster
                .iter()
                .filter(|slot| slot.player_id != 0)
                .map(|slot| file.player(slot.player_id).expect("a rostered player"))
                .collect();
            let document =
                TeamToml::from_team(version, team, &players).expect("from_team emits a document");
            // The hole is a `players` key the document lacks: one over
            // the roster index.
            assert!(
                !document
                    .players
                    .contains_key(&u8::try_from(hole + 1).expect("a roster slot index"))
            );
            let built = TeamView::from_team_toml(&document)
                .unwrap_or_else(|e| panic!("{version:?}: the document builds: {e}"));
            let expected = TeamView::new(&file, 701).expect("the team views");
            assert_eq!(built, expected, "{version:?}");
            if version == PesVersion::Pes19 {
                let hole = u8::try_from(hole).expect("a roster slot index");
                assert!(built.player(hole).is_none(), "the hole stays empty");
                let after = built.player(hole + 1).expect("the next player");
                assert_eq!(after.slot, hole + 1, "not shifted down");
                for shape in expected.formation_shapes() {
                    assert_eq!(
                        built.formation(shape.preset, shape.formation),
                        expected.formation(shape.preset, shape.formation),
                        "{version:?} preset {} formation {}",
                        shape.preset,
                        shape.formation
                    );
                }
            }
        }
    }

    #[test]
    fn a_document_without_a_version_is_refused() {
        let document = TeamToml::default();
        assert_eq!(
            TeamView::from_team_toml(&document).expect_err("no version"),
            TeamViewError::NoVersion
        );
    }

    #[test]
    fn a_document_that_does_not_apply_is_team_toml() {
        // `stats.physical_contact` does not exist on PES 15; the note it
        // gives is a `TeamToml` error, not a skipped key.
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes15)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        let players: Vec<&PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| file.player(slot.player_id).expect("a rostered player"))
            .collect();
        let mut document =
            TeamToml::from_team(PesVersion::Pes15, team, &players).expect("from_team emits");
        let section = document.players.get_mut(&1).expect("a first player");
        section.stats.physical_contact = Some(80);
        assert!(
            matches!(
                TeamView::from_team_toml(&document).expect_err("a version-lacking key"),
                TeamViewError::TeamToml { .. }
            ),
            "stats.physical_contact on PES 15 is refused"
        );
    }

    #[test]
    fn an_absent_set_pieces_key_reads_as_none() {
        // The bare record's takers are 0xFF, not roster slot 0: a
        // document without `[tactics.set_pieces.captain]` gives
        // `captain: None`.
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes15)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        let players: Vec<&PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| file.player(slot.player_id).expect("a rostered player"))
            .collect();
        let mut document =
            TeamToml::from_team(PesVersion::Pes15, team, &players).expect("from_team emits");
        document.tactics.set_pieces.captain = None;
        assert_eq!(
            TeamView::from_team_toml(&document)
                .expect("the team builds")
                .captain,
            None
        );
        // The sample team's captain is still `Some(10)`.
        assert_eq!(TeamView::sample_team().captain, Some(10));
    }

    #[test]
    fn an_absent_tactics_section_fields_nobody() {
        // A document without `[tactics]`: the bare record's starting
        // eleven and bench order are 0xFF, not eleven copies of slot 0.
        // Slot 0's `fielded_*` answers come from his registered position
        // alone; the sample's slot 0 is registered CB here (a GK
        // otherwise would answer `fielded_only_at` the same either way,
        // the default formations' bytes being GK).
        let mut document =
            TeamToml::parse(include_str!("../sample/team.toml")).expect("the sample team parses");
        document.tactics = Default::default();
        document
            .players
            .get_mut(&1)
            .expect("a first player")
            .positions
            .registered = Some(1);
        let view = TeamView::from_team_toml(&document).expect("the team builds");
        assert_eq!(view.fielded_positions(0, 1), Vec::<Position>::new());
        let cb = Position(1);
        assert!(view.fielded_only_at(0, cb));
        assert!(!view.fielded_only_at(0, Position(0)));
    }

    #[test]
    fn an_absent_man_marking_key_reads_as_none() {
        // The bare record's assignments are 0xFF, not a marking of slot 0:
        // a document without `man_marking` keys gives all-0xFF presets.
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes15)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        let players: Vec<&PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| file.player(slot.player_id).expect("a rostered player"))
            .collect();
        let mut document =
            TeamToml::from_team(PesVersion::Pes15, team, &players).expect("from_team emits");
        for preset in &mut document.tactics.presets {
            preset.man_marking = None;
        }
        let view = TeamView::from_team_toml(&document).expect("the team builds");
        assert!(
            view.tactics
                .presets
                .iter()
                .all(|preset| preset.man_marking == [0xFF; 11])
        );
        // A present key carries through: the document's own array lands.
        document.tactics.presets[0].man_marking = Some([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let view = TeamView::from_team_toml(&document).expect("the team builds");
        assert_eq!(
            view.tactics.presets[0].man_marking,
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        );
        assert!(
            view.tactics.presets[1..]
                .iter()
                .all(|preset| preset.man_marking == [0xFF; 11])
        );
    }

    /// The sample is legal under Spring 26, the ruleset it was built
    /// for. `catching` is a stat PES 15 has: its key removed from the
    /// document, the bare record reads 0 like any other field (not
    /// "the version lacks it"), and the check reports it against the
    /// regular rate, 77.
    #[test]
    fn an_omitted_gated_stat_reads_zero() {
        let rules = Rules::from_data_block(include_str!("../rulesets/spring26.rhai"))
            .expect("Spring 26 loads");
        let problems = |view: &TeamView| {
            rules
                .check_team(view)
                .expect("the check runs")
                .into_iter()
                .filter(|violation| violation.severity != Severity::Suggestion)
                .collect::<Vec<Violation>>()
        };
        assert_eq!(problems(&TeamView::sample_team()), vec![]);
        let mut document =
            TeamToml::parse(include_str!("../sample/team.toml")).expect("the sample parses");
        let section = document.players.get_mut(&2).expect("player 02");
        assert_eq!(section.stats.catching, Some(77));
        section.stats.catching = None;
        let view = TeamView::from_team_toml(&document).expect("the team builds");
        assert_eq!(
            problems(&view),
            vec![Violation {
                slot: Some(1),
                severity: Severity::Error,
                kind: ViolationKind::Stat {
                    stat: Stat::try_from("catching".to_string()).expect("a stat"),
                    value: 0,
                    target: 77,
                    at_most: false,
                },
            }]
        );
    }

    /// Every `[tactics.auto]` key left out on a PES 21 document: the
    /// bare tactics record carries the version's flags at zero, so
    /// `Some(0)`/`Some(false)`, not `None` ("the version lacks it").
    #[test]
    fn omitted_auto_settings_read_off() {
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        let players: Vec<&PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| file.player(slot.player_id).expect("a rostered player"))
            .collect();
        let mut document =
            TeamToml::from_team(PesVersion::Pes21, team, &players).expect("from_team emits");
        document.tactics.auto = Default::default();
        let view = TeamView::from_team_toml(&document).expect("the team builds");
        assert_eq!(view.tactics.auto.substitution, Some(0));
        assert_eq!(view.tactics.auto.offside_trap, Some(false));
        assert_eq!(view.tactics.auto.preset_change, Some(false));
        assert_eq!(view.tactics.auto.attack_defence_levels, Some(false));
    }

    #[test]
    fn an_unencodable_instruction_is_its_import_note() {
        // `anchoring` is PES 20/21 only: `apply` notes `NotEncodable`,
        // which surfaces as that note itself, for the tool to word.
        let file = EditFile::from_bytes(&fixture_save(PesVersion::Pes17)).expect("fixture save");
        let team = file.team(701).expect("team 701");
        let players: Vec<&PlayerEntry> = team
            .roster
            .iter()
            .filter(|slot| slot.player_id != 0)
            .map(|slot| file.player(slot.player_id).expect("a rostered player"))
            .collect();
        let mut document =
            TeamToml::from_team(PesVersion::Pes17, team, &players).expect("from_team emits");
        let instructions = document.tactics.presets[0]
            .instructions
            .as_mut()
            .expect("PES 17 has instructions");
        instructions.attack[0].instruction = pes_savefile::model::Instruction::Anchoring;
        let err = TeamView::from_team_toml(&document).expect_err("an unencodable instruction");
        assert_eq!(err.to_string(), "the document does not apply as written");
        assert_eq!(
            err,
            TeamViewError::ImportNote {
                note: ImportNote::NotEncodable {
                    path: "tactics.preset_1.instructions.attack".to_owned(),
                    label: "anchoring".to_owned(),
                },
            }
        );
    }

    #[test]
    fn the_sample_team_is_team_714_of_the_cup_save() {
        let team = TeamView::sample_team();
        assert_eq!(team.players.len(), 23);
        assert_eq!(team.captain, Some(10));
    }

    #[test]
    fn pes15_fields_one_formation_per_preset() {
        let (file, team) = view(PesVersion::Pes15);
        // PES 15 has no fluid formation: `fluid` is `None` on all three
        // presets, so each fields its kick-off formation only.
        for preset in &file.team(701).expect("team 701").tactics.presets {
            assert_eq!(preset.style.fluid, None);
        }
        assert_eq!(team.formation_shapes().len(), 3);
    }
}
