//! The Tactics editor's copy and swap menu operations. A preset copy or
//! swap moves the whole `TacticsPreset`, man marking included, since the
//! game's own copy preset is how a match's man marking gets into a team. A
//! formation copy or swap moves the whole `Formation` within the shown
//! preset. The starting eleven is never touched. Each is one sealed undo entry: a
//! clone of the team, edited and passed to `edit_entries`, as `team_ops`
//! does for players. egui-free, so the operations are testable without a
//! frame.

use pes_savefile::model::tactics::{Formation, TacticsPreset};

use crate::session::Session;

impl Session {
    /// Copies preset `from` onto `to` of `team_id`'s tactics (the editor's
    /// "Copy preset N to M" menu).
    pub(crate) fn copy_preset(&mut self, team_id: u32, from: usize, to: usize) {
        let Some(preset) = self
            .file
            .team(team_id)
            .map(|team| team.tactics.presets[from].clone())
        else {
            return;
        };
        self.paste_preset(team_id, to, preset);
    }

    /// `preset` written over `to` of `team_id`'s tactics (Ctrl+V's preset
    /// paste, the source the clipboard's snapshot rather than a preset of
    /// the team): `copy_preset`'s write.
    pub(crate) fn paste_preset(&mut self, team_id: u32, to: usize, preset: TacticsPreset) {
        let Some(mut team) = self.file.team(team_id).cloned() else {
            return;
        };
        team.tactics.presets[to] = preset;
        self.edit_entries(Vec::new(), vec![team]);
    }

    /// Swaps presets `a` and `b` of `team_id`'s tactics (the editor's
    /// "Swap presets N and M" menu).
    pub(crate) fn swap_presets(&mut self, team_id: u32, a: usize, b: usize) {
        let Some(mut team) = self.file.team(team_id).cloned() else {
            return;
        };
        team.tactics.presets.swap(a, b);
        self.edit_entries(Vec::new(), vec![team]);
    }

    /// Copies formation `from` onto `to` inside `team_id`'s preset
    /// `preset` (the editor's "Copy <formation> to <formation>" menu).
    pub(crate) fn copy_formation(&mut self, team_id: u32, preset: usize, from: usize, to: usize) {
        let Some(formation) = self
            .file
            .team(team_id)
            .map(|team| team.tactics.presets[preset].formations[from].clone())
        else {
            return;
        };
        self.paste_formation(team_id, preset, to, formation);
    }

    /// `formation` written over `to` inside `team_id`'s preset `preset`
    /// (Ctrl+V's formation paste, the source the clipboard's snapshot):
    /// `copy_formation`'s write.
    pub(crate) fn paste_formation(
        &mut self,
        team_id: u32,
        preset: usize,
        to: usize,
        formation: Formation,
    ) {
        let Some(mut team) = self.file.team(team_id).cloned() else {
            return;
        };
        team.tactics.presets[preset].formations[to] = formation;
        self.edit_entries(Vec::new(), vec![team]);
    }

    /// Swaps formations `a` and `b` inside `team_id`'s preset `preset`
    /// (the editor's "Swap <formation> and <formation>" menu).
    pub(crate) fn swap_formations(&mut self, team_id: u32, preset: usize, a: usize, b: usize) {
        let Some(mut team) = self.file.team(team_id).cloned() else {
            return;
        };
        team.tactics.presets[preset].formations.swap(a, b);
        self.edit_entries(Vec::new(), vec![team]);
    }
}

#[cfg(test)]
mod tests {
    use pes_version::PesVersion;
    use studio_core::Recorder;

    use crate::test_support;

    use super::*;

    /// Writes `version`'s fixture save under the test's scratch folder and
    /// opens it as a `Session`.
    fn open(version: PesVersion, test: &str) -> Session {
        let dir = test_support::scratch(test);
        let path = dir.join("EDIT00000000");
        std::fs::write(&path, pes_savefile::fixture_save(version)).unwrap();
        Session::open(&path, Recorder::default()).unwrap()
    }

    // A preset copy moves the whole preset, man marking included (the game's
    // own copy preset is how a match's man marking gets into a team); the
    // starting eleven is the team's and stays.
    #[test]
    fn a_preset_copy_moves_the_whole_preset_man_marking_included() {
        let mut session = open(PesVersion::Pes21, "tac_copy_preset");
        // Non-255 man-marking bytes on the source, so the copy is not the
        // fixture's default.
        session.file.team_mut(701).unwrap().tactics.presets[0].man_marking = [3; 11];
        let starting_before = session.file().team(701).unwrap().tactics.starting_eleven;
        let source = session.file().team(701).unwrap().tactics.presets[0].clone();
        assert!(
            source.man_marking.iter().all(|&slot| slot == 3),
            "premise: the source carries man marking"
        );
        assert_ne!(
            session.file().team(701).unwrap().tactics.presets[1],
            source,
            "premise: preset 2 differs before the copy"
        );

        session.copy_preset(701, 0, 1);

        let team = session.file().team(701).unwrap();
        assert_eq!(
            team.tactics.presets[1], source,
            "preset 2 equals preset 1 as it was"
        );
        assert_eq!(
            team.tactics.presets[0], source,
            "preset 1 is the copy's source"
        );
        assert_eq!(
            team.tactics.presets[1].man_marking, [3; 11],
            "man marking rides along"
        );
        assert_eq!(
            team.tactics.starting_eleven, starting_before,
            "the starting eleven is untouched"
        );
        assert_ne!(team.tactics.presets[2], source, "preset 3 is untouched");

        // One undo entry for the whole operation.
        assert!(session.undo());
        assert!(!session.can_undo(), "the copy was one entry");
        assert_ne!(
            session.file().team(701).unwrap().tactics.presets[1],
            source,
            "the undo restores preset 2"
        );
    }

    // The same ruling's swap half.
    #[test]
    fn a_preset_swap_trades_the_whole_presets() {
        let mut session = open(PesVersion::Pes21, "tac_swap_preset");
        // Distinct sliders per preset (the fixture's three are the same
        // placeholder), so a swap is a real change, not a no-op.
        for (preset, value) in [(0, 7), (1, 5), (2, 1)] {
            session.file.team_mut(701).unwrap().tactics.presets[preset]
                .sliders
                .support_range = value;
        }
        let [one, two, three] = session.file().team(701).unwrap().tactics.presets.clone();
        assert_ne!(one, three, "premise: the two presets differ");

        session.swap_presets(701, 0, 2);

        let presets = &session.file().team(701).unwrap().tactics.presets;
        assert_eq!(presets[0], three, "preset 3 took preset 1's place");
        assert_eq!(presets[2], one, "preset 1 took preset 3's");
        assert_eq!(presets[1], two, "preset 2 is untouched");

        assert!(session.undo());
        assert!(!session.can_undo(), "the swap was one entry");
        assert_eq!(
            session.file().team(701).unwrap().tactics.presets[0],
            one,
            "the undo restores preset 1"
        );
    }

    // A swap or copy of a preset or formation onto itself writes nothing (no
    // undo entry, no unsaved flag): `edit_entries` skips an unchanged team.
    #[test]
    fn a_self_swap_or_copy_writes_nothing() {
        let mut session = open(PesVersion::Pes21, "tac_swap_noop");
        session.swap_presets(701, 1, 1);
        session.swap_formations(701, 0, 2, 2);
        session.copy_preset(701, 0, 0);
        session.copy_formation(701, 0, 1, 1);
        // A copy whose contents are already equal pushes nothing either
        // (the fixture's three presets are the same placeholder).
        session.copy_preset(701, 0, 1);
        assert!(!session.can_undo(), "nothing pushed");
        assert!(!session.is_dirty(), "and nothing marked");
    }

    // A formation copy or swap moves the whole `Formation` inside the
    // shown preset: the other presets and the starting eleven stay.
    #[test]
    fn a_formation_swap_moves_one_preset_formations_only() {
        let mut session = open(PesVersion::Pes21, "tac_swap_formation");
        // Distinct coordinates per formation (the fixture's three are the
        // same placeholder), so the swap and copy are real changes.
        {
            let tactics = &mut session.file.team_mut(701).unwrap().tactics;
            for (formation, x) in [(0, 10), (1, 20), (2, 30)] {
                for (preset, y) in [(0, 40), (1, 42), (2, 44)] {
                    tactics.presets[preset].formations[formation].players[0].x = x;
                    tactics.presets[preset].formations[formation].players[0].y = y;
                }
            }
        }
        let presets_before = session.file().team(701).unwrap().tactics.presets.clone();
        let starting_before = session.file().team(701).unwrap().tactics.starting_eleven;
        let [kick_off, in_possession, out_of_possession] = presets_before[1].formations.clone();
        assert_ne!(kick_off, in_possession, "premise: the formations differ");

        session.swap_formations(701, 1, 0, 1);

        let team = session.file().team(701).unwrap();
        let formations = &team.tactics.presets[1].formations;
        assert_eq!(
            formations[0], in_possession,
            "in-possession took kick-off's place"
        );
        assert_eq!(formations[1], kick_off, "kick-off took in-possession's");
        assert_eq!(formations[2], out_of_possession, "out-of-possession stays");
        assert_eq!(
            team.tactics.presets[0], presets_before[0],
            "preset 1 untouched"
        );
        assert_eq!(
            team.tactics.presets[2], presets_before[2],
            "preset 3 untouched"
        );
        assert_eq!(
            team.tactics.starting_eleven, starting_before,
            "the starting eleven is untouched"
        );

        assert!(session.undo());
        assert!(!session.can_undo(), "the swap was one entry");

        // The copy half: formation 2 onto formation 0 of preset 0.
        let source = session.file().team(701).unwrap().tactics.presets[0].formations[2].clone();
        session.copy_formation(701, 0, 2, 0);
        let formations = &session.file().team(701).unwrap().tactics.presets[0].formations;
        assert_eq!(
            formations[0], source,
            "formation 1 took formation 3's contents"
        );
        assert_eq!(
            formations[1], presets_before[0].formations[1],
            "formation 2 stays"
        );
        assert!(session.undo());
        assert!(!session.can_undo(), "the copy was one entry");
    }
}
