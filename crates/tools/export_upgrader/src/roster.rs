//! The team's roster as the save records it: each rostered player's display
//! name, his worn boots and gloves IDs for pairing the item folders with
//! their wearers, and his player ID for reading his record when
//! `settings.toml` is generated.

use std::collections::BTreeMap;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;

/// One rostered player of the resolved team, as the save records him (step 2).
#[derive(Debug)]
pub(crate) struct RosteredPlayer {
    /// The player ID the roster entry named and his record was read by
    /// (`team_id * 100 + NN`), for step 8's `save.player` read.
    pub(crate) id: u32,
    /// The save's name with its colour codes stripped (`pes_savefile`'s
    /// `display_name`).
    pub(crate) name: String,
    /// The custom boots ID his appearance record wears (the wearer map of
    /// the item folders' migration).
    pub(crate) boots_id: u32,
    /// The custom gloves ID his appearance record wears (the wearer map of
    /// the item folders' migration).
    pub(crate) gloves_id: u32,
}

/// The resolved team's rostered players by slot (1-23), read by player ID
/// (`team_id * 100 + slot`, `spec/aesthetics_export/player_folders.md`
/// "Savefile identity"), never by roster position: a real save's roster need
/// not be in slot order.
#[derive(Debug)]
pub(crate) struct Roster(pub(crate) BTreeMap<u8, RosteredPlayer>);

impl Roster {
    /// The roster of the save's team `team` (its record ID, whatever the
    /// save numbers it) in `save`: every roster entry whose player ID is the
    /// team's `team_id * 100 + NN` with NN 1-23 and whose record the save
    /// holds. A roster entry naming a player of another team (a real save's
    /// roster may hold a transferred player) is not a slot of this team's
    /// folders: it is skipped.
    pub(crate) fn read(save: &EditFile, team: u32) -> Roster {
        let mut players = BTreeMap::new();
        if let Some(team) = save.team(team) {
            for slot in &team.roster {
                // The slot number is the last two digits of the team's own
                // player ID (`team_id * 100 + NN`); 0 (empty) and any other
                // team's ID fail it.
                let Some(nn) = slot
                    .player_id
                    .checked_sub(team.id * 100)
                    .filter(|nn| (1..=23).contains(nn))
                else {
                    continue;
                };
                let Some(player) = save.player(slot.player_id) else {
                    continue;
                };
                players.insert(
                    nn as u8,
                    RosteredPlayer {
                        id: slot.player_id,
                        name: display_name(&player.name),
                        boots_id: player.appearance.boots_id,
                        gloves_id: player.appearance.gloves_id,
                    },
                );
            }
        }
        Roster(players)
    }

    /// The player rostered at `slot` (1-23), if any.
    pub(crate) fn get(&self, slot: u8) -> Option<&RosteredPlayer> {
        self.0.get(&slot)
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;

    /// The PES 21 fixture save with `edit` applied to its model, saved back.
    fn edited_save(edit: impl FnOnce(&mut EditFile)) -> EditFile {
        let mut file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21))
            .expect("the fixture save loads");
        edit(&mut file);
        file
    }

    #[test]
    fn the_roster_is_read_by_player_id_never_position() {
        let file = edited_save(|file| {
            let team = file.team_mut(714).unwrap();
            // A roster out of slot order, an entry of another team (a
            // transferred player) and an empty entry.
            team.roster.swap(0, 22);
            team.roster[5].player_id = 71523;
        });
        let roster = Roster::read(&file, 714);
        // The swapped entries land on their own slots by ID.
        assert!(roster.get(23).is_some(), "71423 at its own slot");
        assert_eq!(
            roster.get(1).unwrap().name,
            display_name(&file.player(71401).unwrap().name)
        );
        // 71523's entry contributes nothing: the slot it replaced is empty.
        assert!(roster.get(6).is_none());
    }

    #[test]
    fn an_empty_entry_is_no_slot() {
        let file = edited_save(|file| {
            let team = file.team_mut(714).unwrap();
            let position = team
                .roster
                .iter()
                .position(|slot| slot.player_id == 71423)
                .unwrap();
            team.roster[position].player_id = 0;
        });
        let roster = Roster::read(&file, 714);
        assert!(roster.get(23).is_none());
        assert!(roster.get(1).is_some());
    }
}
