//! The Team menu's operations, each one an `edit_players` batch over the
//! team's filled roster slots, so the whole operation is one sealed undo
//! entry. egui-free, so the operations are testable without a frame; the
//! menu's lock (the boots and gloves item, Clear visual flags) is the
//! view's to apply, not these functions'.

use std::ops::RangeInclusive;

use anyhow::anyhow;
use pes_savefile::model::PlayerEntry;
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::player_max_value;

use crate::session::Session;

/// The registered position's full name, in `positions.registered` order
/// (GK 0 to CF 12), which Set player names to positions writes. The
/// spellings mix "Center" and "Centre": they are the parity table's, kept
/// as they are.
const POSITION_NAMES: [&str; 13] = [
    "Goalkeeper",
    "Center Back",
    "Left Back",
    "Right Back",
    "Defensive Midfielder",
    "Center Midfielder",
    "Left Midfielder",
    "Right Midfielder",
    "Attacking Midfielder",
    "Left Wing Forward",
    "Right Wing Forward",
    "Second Striker",
    "Centre Forward",
];

/// The boots half of Set boots and gloves IDs for everyone.
pub(crate) enum BootsMode {
    /// Each player keeps his boots ID.
    Unchanged,
    /// Roster slot `k` (1-based) gets `start + k - 1`, every filled slot numbered by its slot.
    Incremental { start: u32 },
    /// Every player gets this ID.
    Same(u32),
}

/// The gloves half: kept, each player's (new) boots ID, or one ID for all.
pub(crate) enum GlovesMode {
    /// Each player keeps his gloves ID.
    Unchanged,
    /// Each player gets the boots ID the operation gave him.
    LikeBoots,
    /// Every player gets this ID.
    Same(u32),
}

impl Session {
    /// Sets `edit_flags.player` on every filled roster slot of `team_id`
    /// (the menu's Set edited flag; gameplay, so it is never locked).
    pub(crate) fn set_edited_flag(&mut self, team_id: u32) {
        let edited = self
            .edited_players(team_id, |_, player| {
                player.edit_flags.player = true;
                Ok(())
            })
            .expect("the flag edit cannot fail");
        self.edit_players(edited);
    }

    /// Clears the four visual edit flags (`face`, `hair`, `physique`,
    /// `strip`) on every filled roster slot of `team_id` (the menu's Clear
    /// visual flags).
    pub(crate) fn clear_visual_flags(&mut self, team_id: u32) {
        let edited = self
            .edited_players(team_id, |_, player| {
                player.edit_flags.face = false;
                player.edit_flags.hair = false;
                player.edit_flags.physique = false;
                player.edit_flags.strip = false;
                Ok(())
            })
            .expect("the flag edit cannot fail");
        self.edit_players(edited);
    }

    /// Renames every filled roster slot of `team_id` to its registered
    /// position's full name (`POSITION_NAMES`) and sets the edited-player
    /// flag (the menu's Set player names to positions). A registered
    /// position past the table leaves that player's name alone.
    pub(crate) fn names_to_positions(&mut self, team_id: u32) {
        let edited = self
            .edited_players(team_id, |_, player| {
                if let Some(&name) = POSITION_NAMES.get(usize::from(player.positions.registered)) {
                    player.name = name.to_owned();
                    player.edit_flags.player = true;
                }
                Ok(())
            })
            .expect("the name edit cannot fail");
        self.edit_players(edited);
    }

    /// Sets the boots and gloves IDs of every filled roster slot of
    /// `team_id` (the menu's Set boots and gloves IDs for everyone): the
    /// boots first, then the gloves, so `GlovesMode::LikeBoots` copies the
    /// new IDs. Every written value is checked against the field's own
    /// bound (the record's `2^width - 1`, wider than `settings.toml`'s
    /// 0 to 100: the FPC hide ID and a compile-allocated ID such as 627
    /// are values the save holds) before the first write — a value outside
    /// it refuses the whole operation with an error naming the slot, the
    /// value and the range, the save untouched.
    pub(crate) fn set_boots_and_gloves(
        &mut self,
        team_id: u32,
        boots: BootsMode,
        gloves: GlovesMode,
    ) -> anyhow::Result<()> {
        let version = self.version();
        let boots_range = id_range(version, PlayerField::BootsId);
        let gloves_range = id_range(version, PlayerField::GlovesId);
        let edited = self.edited_players(team_id, |slot, player| {
            player.appearance.boots_id = match boots {
                BootsMode::Unchanged => player.appearance.boots_id,
                // Slot `k` (1-based) gets `start + k - 1`: numbered by its
                // slot, whatever lies before it.
                BootsMode::Incremental { start } => in_range(
                    "boots ID",
                    slot,
                    u64::from(start) + u64::try_from(slot).expect("a slot index fits u64"),
                    &boots_range,
                )?,
                BootsMode::Same(id) => in_range("boots ID", slot, u64::from(id), &boots_range)?,
            };
            player.appearance.gloves_id = match gloves {
                GlovesMode::Unchanged => player.appearance.gloves_id,
                GlovesMode::LikeBoots => in_range(
                    "gloves ID",
                    slot,
                    u64::from(player.appearance.boots_id),
                    &gloves_range,
                )?,
                GlovesMode::Same(id) => in_range("gloves ID", slot, u64::from(id), &gloves_range)?,
            };
            Ok(())
        })?;
        self.edit_players(edited);
        Ok(())
    }

    /// Applies `kind` (`fpc::Preset::Hide` or `Unhide`) to every filled
    /// roster slot of `team_id` (the menu's Set / Remove FPC
    /// invisibility), each record's own boots and gloves IDs kept
    /// (`player_ops::preset_for` under `ids`, the settings' hide IDs), one
    /// sealed undo entry.
    pub(crate) fn team_fpc(&mut self, team_id: u32, kind: fpc::Preset, ids: fpc::HideIds) {
        let version = self.version();
        let edited = self
            .edited_players(team_id, |_, player| {
                let appearance = crate::session::player_ops::preset_for(player, kind, ids);
                // The hide and show presets write no custom skin, and a
                // loaded player's face run holds the fields `apply` needs.
                pes_savefile::ops::fpc::apply(player, &appearance, version)
                    .expect("a loaded player's face run holds the strip fields");
                Ok(())
            })
            .expect("the fpc edit cannot fail");
        self.edit_players(edited);
    }

    /// The filled roster slots of `team_id` as edited entries, in slot
    /// order (`player_id != 0`; a rostered id the save lacks is skipped):
    /// each rostered player's entry with `edit` applied to it, the first
    /// error `edit` returns propagated with nothing applied.
    fn edited_players(
        &self,
        team_id: u32,
        mut edit: impl FnMut(usize, &mut PlayerEntry) -> anyhow::Result<()>,
    ) -> anyhow::Result<Vec<PlayerEntry>> {
        let mut edited = Vec::new();
        let Some(team) = self.file.team(team_id) else {
            return Ok(edited);
        };
        for (slot, roster_slot) in team.roster.iter().enumerate() {
            if roster_slot.player_id == 0 {
                continue;
            }
            let Some(player) = self.file.player(roster_slot.player_id) else {
                continue;
            };
            let mut player = player.clone();
            edit(slot, &mut player)?;
            edited.push(player);
        }
        Ok(edited)
    }
}

/// `field`'s whole stored range on `version` (`0..=max_value`, the codec's
/// `ValueTooWide` bound): the Appearance section's Boots ID and Gloves ID
/// controls and this check read the same bound.
fn id_range(version: pes_version::PesVersion, field: PlayerField) -> RangeInclusive<u32> {
    let max = player_max_value(version, field).expect("the id fields are stored");
    0..=u32::try_from(max).expect("the id fields' bound fits u32")
}

/// `value` inside `range` as the `u32` the field stores, else the refusal
/// `set_boots_and_gloves` returns: the 1-based slot, the value and the
/// bounds, with nothing written yet.
fn in_range(
    what: &str,
    slot: usize,
    value: u64,
    range: &RangeInclusive<u32>,
) -> anyhow::Result<u32> {
    if u64::from(*range.start()) <= value && value <= u64::from(*range.end()) {
        Ok(u32::try_from(value).expect("inside a u32 range"))
    } else {
        Err(anyhow!(
            "slot {}: {what} {value} is outside {} to {}",
            slot + 1,
            range.start(),
            range.end()
        ))
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;
    use studio_core::Recorder;

    use super::*;

    /// Writes `version`'s fixture save under the test's scratch folder and
    /// opens it as a `Session`.
    fn open(version: PesVersion, test: &str) -> Session {
        let dir = crate::test_support::scratch(test);
        let path = dir.join("EDIT00000000");
        std::fs::write(&path, pes_savefile::fixture_save(version)).unwrap();
        Session::open(&path, Recorder::default()).unwrap()
    }

    /// `team_id`'s filled roster slots as `(slot, player_id)`, in slot order.
    fn rostered(session: &Session, team_id: u32) -> Vec<(usize, u32)> {
        session
            .file()
            .team(team_id)
            .unwrap()
            .roster
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.player_id != 0)
            .map(|(slot, roster_slot)| (slot, roster_slot.player_id))
            .collect()
    }

    // Set boots and gloves IDs for everyone: a value past the field's own
    // bound (`max_value`, the codec's `ValueTooWide` limit — 14 bits, so
    // 16383 on every version) refuses the whole operation before the first
    // write: the error names the slot, the value and the range.
    #[test]
    fn a_boots_id_past_the_range_refuses_with_the_save_untouched() {
        let mut session = open(PesVersion::Pes21, "boglo_refused");
        let bound = player_max_value(PesVersion::Pes21, PlayerField::BootsId)
            .expect("PES 21 stores the boots id");
        let before: Vec<PlayerEntry> = rostered(&session, 701)
            .iter()
            .map(|&(_, id)| session.file().player(id).unwrap().clone())
            .collect();

        let error = session
            .set_boots_and_gloves(
                701,
                BootsMode::Same(u32::try_from(bound + 1).expect("the bound is narrower than u32")),
                GlovesMode::Unchanged,
            )
            .unwrap_err();

        let text = format!("{error:#}");
        assert!(text.contains("slot 1"), "{text}");
        assert!(text.contains(&(bound + 1).to_string()), "{text}");
        assert!(text.contains(&format!("0 to {bound}")), "{text}");
        for (&(_, id), entry) in rostered(&session, 701).iter().zip(&before) {
            assert_eq!(session.file().player(id).unwrap(), entry);
        }
        assert!(!session.can_undo(), "nothing was pushed");
        assert!(!session.is_dirty(), "the save is untouched");
    }

    // The same row's numbering: slot `k` (1-based) gets `start + k - 1`,
    // an empty slot skipped without shifting the slots after it; the
    // gloves copy the new boots IDs.
    #[test]
    fn incremental_ids_number_the_filled_slots_by_their_slot() {
        let mut session = open(PesVersion::Pes21, "boglo_incremental");
        // Empty slot 1 (index): its player stays in the pool, unrostered,
        // and the slots after it keep their numbers.
        session.file.team_mut(701).unwrap().roster[1].player_id = 0;
        let unrostered = session.file().player(70102).unwrap().clone();

        session
            .set_boots_and_gloves(
                701,
                BootsMode::Incremental { start: 50 },
                GlovesMode::LikeBoots,
            )
            .unwrap();

        for (slot, id) in rostered(&session, 701) {
            let player = session.file().player(id).unwrap();
            let expected = 50 + u32::try_from(slot).expect("a roster has at most 40 slots");
            assert_eq!(
                player.appearance.boots_id,
                expected,
                "slot {}'s boots",
                slot + 1
            );
            assert_eq!(
                player.appearance.gloves_id,
                expected,
                "slot {}'s gloves copy the boots",
                slot + 1
            );
        }
        assert_eq!(
            session.file().player(70102).unwrap(),
            &unrostered,
            "the unrostered player is untouched"
        );
        // One undo entry for the whole team.
        assert!(session.undo());
        assert!(!session.can_undo(), "the operation was one entry");
    }

    // The kept half of each side: `Unchanged` boots leave every filled
    // slot's ID as it was while a `Same` gloves ID lands on all of them.
    #[test]
    fn unchanged_boots_and_one_gloves_id_for_all() {
        let mut session = open(PesVersion::Pes21, "boglo_same_gloves");
        // Distinct boots IDs and gloves IDs off 11, so a kept or written
        // value is not the fixture's by coincidence.
        for &(_, id) in &rostered(&session, 701) {
            let appearance = &mut session.file.player_mut(id).unwrap().appearance;
            appearance.boots_id = 300 + id % 100;
            appearance.gloves_id = 400 + id % 100;
        }
        let before: Vec<(usize, u32, u32)> = rostered(&session, 701)
            .iter()
            .map(|&(slot, id)| {
                let boots = session.file().player(id).unwrap().appearance.boots_id;
                (slot, id, boots)
            })
            .collect();

        session
            .set_boots_and_gloves(701, BootsMode::Unchanged, GlovesMode::Same(11))
            .unwrap();

        for (slot, id, boots) in before {
            let player = session.file().player(id).unwrap();
            assert_eq!(
                player.appearance.boots_id,
                boots,
                "slot {}'s boots kept",
                slot + 1
            );
            assert_eq!(
                player.appearance.gloves_id,
                11,
                "slot {}'s gloves",
                slot + 1
            );
        }
    }

    // The other kept half: a `Same` boots ID on every filled slot while
    // `Unchanged` gloves keep each slot's own.
    #[test]
    fn one_boots_id_for_all_and_unchanged_gloves() {
        let mut session = open(PesVersion::Pes21, "boglo_same_boots");
        // Distinct gloves IDs and boots IDs off 627, the same premise.
        for &(_, id) in &rostered(&session, 701) {
            let appearance = &mut session.file.player_mut(id).unwrap().appearance;
            appearance.boots_id = 300 + id % 100;
            appearance.gloves_id = 400 + id % 100;
        }
        let before: Vec<(usize, u32, u32)> = rostered(&session, 701)
            .iter()
            .map(|&(slot, id)| {
                let gloves = session.file().player(id).unwrap().appearance.gloves_id;
                (slot, id, gloves)
            })
            .collect();

        session
            .set_boots_and_gloves(701, BootsMode::Same(627), GlovesMode::Unchanged)
            .unwrap();

        for (slot, id, gloves) in before {
            let player = session.file().player(id).unwrap();
            assert_eq!(player.appearance.boots_id, 627, "slot {}'s boots", slot + 1);
            assert_eq!(
                player.appearance.gloves_id,
                gloves,
                "slot {}'s gloves kept",
                slot + 1
            );
        }
    }

    // The rename's other half: it sets the edited-player flag, not only
    // the name (the SE-GUI-23 view test runs Set edited flag first, so no
    // test saw the flag's set).
    #[test]
    fn names_to_positions_sets_the_edited_player_flag() {
        let mut session = open(PesVersion::Pes21, "names_to_positions_flag");
        session.file.player_mut(70101).unwrap().edit_flags.player = false;
        assert!(
            !session.file().player(70101).unwrap().edit_flags.player,
            "premise: the flag starts clear"
        );

        session.names_to_positions(701);

        let player = session.file().player(70101).unwrap();
        assert!(player.edit_flags.player, "the rename sets the flag");
        assert_eq!(
            player.name,
            POSITION_NAMES[usize::from(player.positions.registered)],
            "his registered position's name"
        );
    }
}
