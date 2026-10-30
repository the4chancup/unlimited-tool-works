//! The Player menu's operations and the quick actions' `apply_tier`: the
//! four stat operations, the FPC hide/show/toggle and the tier write, each
//! one sealed `edit_players` undo entry. egui-free, so the operations are
//! testable without a frame; the lock's part (the
//! FPC items' disabled state) is the view's to apply, not these
//! functions', except Copy and Swap stats' free-kick motion, which the
//! view's lock reaches as their `aesthetics_locked`.

use anyhow::anyhow;
use pes_savefile::model::PlayerEntry;

use crate::session::Session;

impl Session {
    /// One "Make <tier>" click in the quick actions: `Rules::apply_tier` on
    /// the roster slot's player, as one sealed `edit_players` entry. A team that does
    /// not view, an empty slot or a failed `apply_tier` is the error the
    /// view's refusal dialog shows; the player is untouched.
    pub(crate) fn apply_tier(
        &mut self,
        rules: &aatf::Rules,
        team_id: u32,
        slot: u8,
        tier: &str,
    ) -> anyhow::Result<()> {
        let team = aatf::TeamView::new(&self.file, team_id)?;
        let player_id = self
            .slot_player(team_id, slot)
            .ok_or_else(|| anyhow!("team {team_id}'s roster slot {slot} is empty"))?;
        let player = team.player(slot).expect("a filled roster slot views");
        let mut edited = self
            .file
            .player(player_id)
            .expect("a viewed player exists")
            .clone();
        rules.apply_tier(&team, player, tier, &mut edited)?;
        self.edit_players(vec![edited]);
        Ok(())
    }

    /// Every ability stat the version has becomes `value` (the dialog's
    /// Set all stats: `set_ability` over the 25, a `None` one stays
    /// `None`). Weak foot, form and injury are not ability stats and stay;
    /// no edit flag is set, as the player view's own fields set none.
    pub(crate) fn set_all_stats(&mut self, player_id: u32, value: u8) {
        let Some(mut edited) = self.file.player(player_id).cloned() else {
            return;
        };
        for index in 0..edited.stats.abilities().len() {
            edited.stats.set_ability(index, value);
        }
        self.edit_players(vec![edited]);
    }

    /// Every ability stat the version has becomes `value + by` clamped to
    /// 40 to 99 (the dialog's Bump all stats; `by` is read whole, so a -10
    /// bumps by -10). The same not-an-ability list and no-flag rule as
    /// `set_all_stats`.
    pub(crate) fn bump_stats(&mut self, player_id: u32, by: i8) {
        let Some(mut edited) = self.file.player(player_id).cloned() else {
            return;
        };
        for (index, value) in edited.stats.abilities().iter().enumerate() {
            if let Some(value) = value {
                let bumped = i32::from(*value) + i32::from(by);
                edited.stats.set_ability(
                    index,
                    u8::try_from(bumped.clamp(40, 99)).expect("in 40 to 99"),
                );
            }
        }
        self.edit_players(vec![edited]);
    }

    /// Copy stats from `source_id` onto `dest_id` (the dialog's Copy
    /// stats): the field set the parity row lists, and nothing else —
    /// names, nationality, age, every motion but the free kick's, the
    /// motion and appearance flags and every appearance field stay
    /// `dest`'s (`copy_stat_fields`); under the aesthetics lock
    /// (`aesthetics_locked`) the free kick's motion stays his too.
    /// Picking the same player is a no-op (`edit_players` skips an
    /// unchanged record).
    pub(crate) fn copy_stats(&mut self, dest_id: u32, source_id: u32, aesthetics_locked: bool) {
        let (Some(source), Some(dest)) = (self.file.player(source_id), self.file.player(dest_id))
        else {
            return;
        };
        let mut edited = dest.clone();
        copy_stat_fields(source, &mut edited, aesthetics_locked);
        self.edit_players(vec![edited]);
    }

    /// Swap stats between `first_id` and `second_id` (the dialog's Swap
    /// stats): `copy_stat_fields`' set traded both ways, the free kick's
    /// motion left each player's under the aesthetics lock
    /// (`aesthetics_locked`), the whole swap one sealed undo entry.
    pub(crate) fn swap_stats(&mut self, first_id: u32, second_id: u32, aesthetics_locked: bool) {
        let (Some(first), Some(second)) = (self.file.player(first_id), self.file.player(second_id))
        else {
            return;
        };
        let mut first_after = first.clone();
        let mut second_after = second.clone();
        copy_stat_fields(second, &mut first_after, aesthetics_locked);
        copy_stat_fields(first, &mut second_after, aesthetics_locked);
        self.edit_players(vec![first_after, second_after]);
    }

    /// Hide default body or Show default body on `player_id` (the Appearance
    /// section's buttons
    /// and the Player menu's Toggle FPC settings): `kind`'s preset applied
    /// through `ops::fpc::apply` with the record's own IDs substituted in
    /// (`preset_for`), one sealed undo entry. `ids` is the settings' hide
    /// IDs. The strip settings the preset does not set are not cleared —
    /// they are the interference findings' to report. No edit flag is set.
    pub(crate) fn apply_fpc(&mut self, player_id: u32, kind: fpc::Preset, ids: fpc::HideIds) {
        let version = self.version();
        let Some(mut edited) = self.file.player(player_id).cloned() else {
            return;
        };
        let appearance = preset_for(&edited, kind, ids);
        // The presets here write no custom skin, and a loaded player's
        // face run holds the fields `apply` writes through.
        pes_savefile::ops::fpc::apply(&mut edited, &appearance, version)
            .expect("a loaded player's face run holds the strip fields");
        self.edit_players(vec![edited]);
    }

    /// Ctrl+I's toggle: Show default body on a player `is_fpc_player` reads as
    /// hidden, Hide default body on any other, under `ids`.
    pub(crate) fn toggle_fpc(&mut self, player_id: u32, ids: fpc::HideIds) {
        let Some(hidden) = self.file.player(player_id).map(|player| {
            pes_savefile::ops::fpc::is_fpc_player(player, ids)
                .expect("a loaded player's face run holds the skin field")
        }) else {
            return;
        };
        self.apply_fpc(
            player_id,
            if hidden {
                fpc::Preset::Unhide
            } else {
                fpc::Preset::Hide
            },
            ids,
        );
    }
}

/// The field set Copy and Swap stats move (the parity row's field set):
/// height, weight, the whole `stats`, `positions` and `skills`,
/// `motion.free_kick` alone, and the eight gameplay edit flags. Everything
/// else — name, shirt name, nationality, age, the other motions, the
/// motion and base-copy flags, every appearance field — stays `dest`'s.
/// The free kick's motion is an `[appearance.motion]` key of the team's
/// export, so under the aesthetics lock (`aesthetics_locked`) it stays
/// `dest`'s as well.
fn copy_stat_fields(source: &PlayerEntry, dest: &mut PlayerEntry, aesthetics_locked: bool) {
    dest.basic.height = source.basic.height;
    dest.basic.weight = source.basic.weight;
    dest.stats = source.stats.clone();
    dest.positions = source.positions.clone();
    dest.skills = source.skills.clone();
    if !aesthetics_locked {
        dest.motion.free_kick = source.motion.free_kick;
    }
    dest.edit_flags.player = source.edit_flags.player;
    dest.edit_flags.basic_settings = source.edit_flags.basic_settings;
    dest.edit_flags.registered_position = source.edit_flags.registered_position;
    dest.edit_flags.playable_positions = source.edit_flags.playable_positions;
    dest.edit_flags.abilities = source.edit_flags.abilities;
    dest.edit_flags.skills = source.edit_flags.skills;
    dest.edit_flags.playing_style = source.edit_flags.playing_style;
    dest.edit_flags.com_styles = source.edit_flags.com_styles;
}

/// `kind`'s preset for `player`, his own IDs kept, since a full body model
/// may use its own boots or gloves: a preset's boots or gloves ID is
/// written only where the record holds 0 or `ids`'s hide ID, so hiding
/// turns 0 into the hide ID and keeps 627, showing turns the hide ID into
/// 0 and keeps 627. The per-player and the team operations both apply
/// this.
pub(crate) fn preset_for(
    player: &PlayerEntry,
    kind: fpc::Preset,
    ids: fpc::HideIds,
) -> fpc::Appearance {
    let mut appearance = fpc::preset(kind, ids);
    appearance.boots_id = preset_id(player.appearance.boots_id, ids.boots(), appearance.boots_id);
    appearance.gloves_id = preset_id(
        player.appearance.gloves_id,
        ids.gloves(),
        appearance.gloves_id,
    );
    appearance
}

/// The value a preset ID slot takes on a record holding `own`: the
/// preset's where `own` is 0 or the hide preset's `hide_id`, `own`
/// otherwise (a model's own ID is kept).
fn preset_id(own: u32, hide_id: u16, preset: u16) -> u16 {
    if own == 0 || own == u32::from(hide_id) {
        preset
    } else {
        u16::try_from(own).expect("a stored boots or gloves ID fits u16")
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;

    use super::*;

    /// Writes `version`'s fixture save under the test's scratch folder and
    /// opens it as a `Session`.
    fn open(version: PesVersion, test: &str) -> Session {
        let dir = crate::test_support::scratch(test);
        let path = dir.join("EDIT00000000");
        std::fs::write(&path, pes_savefile::fixture_save(version)).unwrap();
        Session::open(&path).unwrap()
    }

    // A preset's ID lands only where the record holds 0 or the hide ID;
    // any other stays (a model's own ID).
    #[test]
    fn a_preset_keeps_a_models_own_ids() {
        let ids = fpc::HideIds::CONVENTION;
        let mut player = PlayerEntry::default();
        player.appearance.boots_id = 0;
        player.appearance.gloves_id = 0;
        let hide = preset_for(&player, fpc::Preset::Hide, ids);
        assert_eq!(hide.boots_id, 55);
        assert_eq!(hide.gloves_id, 11);

        player.appearance.boots_id = 627;
        player.appearance.gloves_id = 21;
        let hide = preset_for(&player, fpc::Preset::Hide, ids);
        assert_eq!(hide.boots_id, 627, "the model's own boots ID stays");
        assert_eq!(hide.gloves_id, 21, "the model's own gloves ID stays");
        let show = preset_for(&player, fpc::Preset::Unhide, ids);
        assert_eq!(show.boots_id, 627);
        assert_eq!(show.gloves_id, 21);

        player.appearance.boots_id = 55;
        player.appearance.gloves_id = 11;
        let show = preset_for(&player, fpc::Preset::Unhide, ids);
        assert_eq!(show.boots_id, 0, "the hide ID shows as 0");
        assert_eq!(show.gloves_id, 0);
        let hide = preset_for(&player, fpc::Preset::Hide, ids);
        assert_eq!(hide.boots_id, 55);
        assert_eq!(hide.gloves_id, 11);
    }

    // The bump's whole-number read and the 40 to 99 clamp, the parts a
    // view test cannot reach without the dialog: each ability's value
    // moves by `by`, a `None` one stays `None`, and the non-ability
    // fields stand.
    #[test]
    fn bump_stats_moves_every_ability_and_no_other_field() {
        let mut session = open(PesVersion::Pes21, "bump_stats");
        session
            .file
            .player_mut(70101)
            .unwrap()
            .stats
            .attacking_prowess = 60;
        let before = session.file().player(70101).unwrap().clone();
        assert_eq!(before.stats.attacking_prowess, 60, "premise");

        session.bump_stats(70101, -10);

        let bumped = session.file().player(70101).unwrap();
        assert_eq!(bumped.stats.attacking_prowess, 50, "-10 moves by -10");
        for (index, ability) in bumped.stats.abilities().iter().enumerate() {
            match (ability, before.stats.abilities()[index]) {
                (Some(now), Some(was)) => {
                    assert_eq!(*now, was.saturating_sub(10).max(40), "ability {index}")
                }
                (now, was) => assert_eq!(now, &was, "ability {index} stays absent"),
            }
        }
        assert_eq!(bumped.stats.form, before.stats.form);
        assert_eq!(
            bumped.stats.injury_resistance,
            before.stats.injury_resistance
        );
        assert_eq!(bumped.stats.weak_foot_usage, before.stats.weak_foot_usage);
        assert_eq!(bumped.edit_flags, before.edit_flags, "no edit flag set");
        assert!(session.undo(), "one sealed entry");
        assert_eq!(session.file().player(70101).unwrap(), &before);
    }

    // The quick actions' operation: `apply_tier` writes the tier's values
    // as the lib computes them, as one sealed undo entry.
    #[test]
    fn apply_tier_writes_the_tiers_values_as_one_undoable_edit() {
        let mut session = open(PesVersion::Pes21, "apply_tier");
        let before = session.file().player(70101).unwrap().clone();
        let rules = aatf::Rules::official();
        let expected = {
            let team = aatf::TeamView::new(session.file(), 701).unwrap();
            let player = team.player(0).unwrap();
            let mut entry = before.clone();
            rules.apply_tier(&team, player, "gold", &mut entry).unwrap();
            entry
        };
        assert_ne!(expected, before, "premise: Gold changes him");

        session.apply_tier(&rules, 701, 0, "gold").unwrap();

        assert_eq!(session.file().player(70101).unwrap(), &expected);
        assert!(session.can_undo(), "one sealed entry");
        assert!(session.undo());
        assert_eq!(session.file().player(70101).unwrap(), &before);
    }

    // Set all stats leaves a `None` ability alone on a version that lacks
    // it (PES 16's tight possession and aggression).
    #[test]
    fn set_all_stats_leaves_absent_abilities_absent() {
        let mut session = open(PesVersion::Pes16, "set_all_absent");
        let before = session.file().player(70101).unwrap().clone();
        assert_eq!(
            before.stats.tight_possession, None,
            "premise: PES 16 lacks it"
        );

        session.set_all_stats(70101, 75);

        let player = session.file().player(70101).unwrap();
        for (index, ability) in player.stats.abilities().iter().enumerate() {
            if let Some(ability) = ability {
                assert_eq!(*ability, 75, "ability {index}");
            }
        }
        assert_eq!(player.stats.tight_possession, None);
        assert_eq!(player.stats.aggression, None);
        assert_eq!(player.stats.form, before.stats.form);
        assert_eq!(player.edit_flags, before.edit_flags);
    }
}
