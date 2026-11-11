//! `TeamEntry`: the version-independent team model. The save keeps a team's
//! data in three records (team, roster, tactics) keyed by team id; the entry
//! merges all three. Fields a version lacks are `Option`/`None`; the schema
//! decides what is read and written.

use crate::codec::{self, CodecError};
use crate::model::tactics::TeamTactics;
use crate::schema::fields::{RosterField, TeamField};

/// One team of the save: identity, colours, the game's edit flags, kit slots,
/// the roster and the tactics record's contents.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamEntry {
    /// The team id.
    pub id: u32,
    /// Team name (UTF-8 on disk).
    pub name: String,
    /// Three-letter short name (single-byte text on disk).
    pub short_name: String,
    /// Manager id (PES 17+).
    pub manager_id: Option<u32>,
    /// Home stadium id (PES 17+).
    pub stadium_id: Option<u16>,
    /// The two team colours (PES 17+).
    pub colors: Option<[TeamColor; 2]>,
    /// The game's own "was edited" flags.
    pub edit_flags: TeamEditFlags,
    /// The ten kit slots: kit number to bound team (PES 17 only).
    pub kit_slots: Option<[KitSlot; 10]>,
    /// The roster: one slot per roster array entry (32 or 40 by version).
    pub roster: Vec<RosterSlot>,
    /// The tactics record's contents.
    pub tactics: TeamTactics,
}

/// One team colour, 6 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TeamColor {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
}

/// The game's own "was edited" flags on the team record.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TeamEditFlags {
    /// The team name was edited.
    pub name: bool,
    /// The short name was edited (PES 15 only).
    pub short_name: Option<bool>,
    /// The home stadium was edited (PES 20/21 only).
    pub stadium: Option<bool>,
    /// The strips were edited (PES 17 only).
    pub strip: Option<bool>,
}

/// One kit slot: a kit number bound to a team.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KitSlot {
    /// Kit number from 0, or 0x80 for a goalkeeper kit.
    pub number: u8,
    /// The bound team id x 0x40, kept raw.
    pub binding: u32,
}

/// One roster entry.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RosterSlot {
    /// The player id, 0 when the slot is empty.
    pub player_id: u32,
    /// Shirt number; a 16-bit field on PES 19+ (values above 99 exist).
    pub number: u16,
}

/// The player id 4cc's roster-order rule puts in `slot` of team `team_id`
/// (`slot` 0-based as stored, so slot 0 holds `team_id * 100 + 1`):
/// `team_id * 100 + slot + 1`. The rule is the only link between the two
/// interchange formats' player addressing: an aesthetics export names
/// players by id, a Team TOML by roster slot.
/// `TeamEntry::roster_order_breaks` lists a team's slots that break it.
pub fn roster_order_player_id(team_id: u32, slot: u8) -> u32 {
    team_id * 100 + u32::from(slot) + 1
}

impl TeamEntry {
    /// The roster slots that break 4cc's roster-order rule — slot `NN`
    /// holds player `team id x 100 + NN` (`roster_order_player_id`), so
    /// stored slot 0 holds `id x 100 + 01` — as `(slot, player_id)`, in
    /// slot order; an empty slot (`player_id` 0) is not a break. The rule
    /// is the only link between the two interchange formats' player
    /// addressing: an aesthetics export names players by id, a Team TOML
    /// by roster slot.
    pub fn roster_order_breaks(&self) -> Vec<(u8, u32)> {
        let mut out = Vec::new();
        for (i, slot) in self.roster.iter().enumerate() {
            let i = u8::try_from(i).expect("a roster holds fewer than 256 slots");
            if slot.player_id != 0 && slot.player_id != roster_order_player_id(self.id, i) {
                out.push((i, slot.player_id));
            }
        }
        out
    }

    /// The model value a team bit run carries.
    pub(crate) fn get(&self, field: TeamField) -> Result<u32, CodecError> {
        let v = match field {
            TeamField::Id => self.id,
            TeamField::ManagerId => self.manager_id.ok_or_else(|| codec::missing(field))?,
            TeamField::StadiumId => {
                u32::from(self.stadium_id.ok_or_else(|| codec::missing(field))?)
            }
            TeamField::EditedName => u32::from(self.edit_flags.name),
            TeamField::EditedShortName => u32::from(
                self.edit_flags
                    .short_name
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TeamField::EditedStadium => u32::from(
                self.edit_flags
                    .stadium
                    .ok_or_else(|| codec::missing(field))?,
            ),
            TeamField::EditedStrip => {
                u32::from(self.edit_flags.strip.ok_or_else(|| codec::missing(field))?)
            }
            TeamField::Color1Red => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[0].red)
            }
            TeamField::Color1Green => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[0].green)
            }
            TeamField::Color1Blue => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[0].blue)
            }
            TeamField::Color2Red => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[1].red)
            }
            TeamField::Color2Green => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[1].green)
            }
            TeamField::Color2Blue => {
                u32::from(self.colors.ok_or_else(|| codec::missing(field))?[1].blue)
            }
            TeamField::KitSlotNumber(i) => u32::from(
                self.kit_slots
                    .as_ref()
                    .ok_or_else(|| codec::missing(field))?
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .number,
            ),
            TeamField::KitSlotTeam(i) => {
                self.kit_slots
                    .as_ref()
                    .ok_or_else(|| codec::missing(field))?
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .binding
            }
        };
        Ok(v)
    }

    /// Patches a team bit-run value into the model.
    pub(crate) fn set(&mut self, field: TeamField, value: u32) -> Result<(), CodecError> {
        match field {
            TeamField::Id => self.id = value,
            TeamField::ManagerId => self.manager_id = Some(value),
            TeamField::StadiumId => self.stadium_id = Some(value as u16),
            TeamField::EditedName => self.edit_flags.name = value != 0,
            TeamField::EditedShortName => self.edit_flags.short_name = Some(value != 0),
            TeamField::EditedStadium => self.edit_flags.stadium = Some(value != 0),
            TeamField::EditedStrip => self.edit_flags.strip = Some(value != 0),
            TeamField::Color1Red => self.colors.get_or_insert_default()[0].red = value as u8,
            TeamField::Color1Green => self.colors.get_or_insert_default()[0].green = value as u8,
            TeamField::Color1Blue => self.colors.get_or_insert_default()[0].blue = value as u8,
            TeamField::Color2Red => self.colors.get_or_insert_default()[1].red = value as u8,
            TeamField::Color2Green => self.colors.get_or_insert_default()[1].green = value as u8,
            TeamField::Color2Blue => self.colors.get_or_insert_default()[1].blue = value as u8,
            TeamField::KitSlotNumber(i) => {
                self.kit_slots
                    .get_or_insert_default()
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .number = value as u8;
            }
            TeamField::KitSlotTeam(i) => {
                self.kit_slots
                    .get_or_insert_default()
                    .get_mut(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .binding = value;
            }
        }
        Ok(())
    }

    /// The model value a roster bit run carries. `TeamId` is the owning team's
    /// id; indexed variants bound-check against the roster's length.
    pub(crate) fn roster_get(&self, field: RosterField) -> Result<u32, CodecError> {
        let v = match field {
            RosterField::TeamId => self.id,
            RosterField::Player(i) => {
                self.roster
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .player_id
            }
            RosterField::Number(i) => u32::from(
                self.roster
                    .get(usize::from(i))
                    .ok_or_else(|| codec::index(field, i))?
                    .number,
            ),
        };
        Ok(v)
    }

    /// Patches a roster bit-run value into the model; the roster grows to the
    /// schema's slot count as the array elements arrive.
    pub(crate) fn roster_set(&mut self, field: RosterField, value: u32) -> Result<(), CodecError> {
        match field {
            RosterField::TeamId => self.id = value,
            RosterField::Player(i) => {
                if usize::from(i) >= self.roster.len() {
                    self.roster
                        .resize(usize::from(i) + 1, RosterSlot::default());
                }
                self.roster[usize::from(i)].player_id = value;
            }
            RosterField::Number(i) => {
                if usize::from(i) >= self.roster.len() {
                    self.roster
                        .resize(usize::from(i) + 1, RosterSlot::default());
                }
                self.roster[usize::from(i)].number = value as u16;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::open;

    /// 4cc's roster-order rule on a real save: team 701 of the PES 21
    /// fixture keeps it, swapping two slots' player ids breaks those two,
    /// an emptied slot is not a break.
    #[test]
    fn roster_order_breaks_lists_the_slots_holding_a_wrong_player() {
        let (mut file, _) = open(PesVersion::Pes21);
        let team = file.team(701).expect("team 701");
        assert!(team.roster_order_breaks().is_empty());
        // Premise: slots 0 and 1 hold the players the rule puts there.
        assert_eq!(team.roster[0].player_id, 70101);
        assert_eq!(team.roster[1].player_id, 70102);
        let team = file.team_mut(701).expect("team 701");
        (team.roster[0].player_id, team.roster[1].player_id) = (70102, 70101);
        let team = file.team(701).expect("team 701");
        assert_eq!(team.roster_order_breaks(), vec![(0, 70102), (1, 70101)]);
        // The emptied slot is not a break; slot 1's stays.
        let team = file.team_mut(701).expect("team 701");
        team.roster[0].player_id = 0;
        assert_eq!(team.roster_order_breaks(), vec![(1, 70101)]);
    }

    /// `roster_set` grows the roster to the written slot (`i + 1`), so
    /// indexing it does not read past the end.
    #[test]
    fn roster_set_grows_the_roster_to_the_written_slot() {
        let mut team = TeamEntry::default();
        team.roster_set(RosterField::Player(0), 55).expect("slot 0");
        assert_eq!(team.roster.len(), 1);
        assert_eq!(team.roster[0].player_id, 55);
        team.roster_set(RosterField::Number(2), 9).expect("slot 2");
        assert_eq!(team.roster.len(), 3);
        assert_eq!(team.roster[2].number, 9);
    }
}
