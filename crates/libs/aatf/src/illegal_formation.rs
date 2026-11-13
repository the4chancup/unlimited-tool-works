//! The host's illegal formation check: a warning for each fielded
//! formation set holding something only a tool outside the game could
//! have made (`Formation::illegal_breaks`). The rule is the game's, the
//! same for every cup, so it runs in Rust beside the script rather than in
//! a rules file's logic; a ruleset only switches it off.

use pes_savefile::model::tactics::Formation;
use pes_version::PesVersion;

use crate::ruleset::{Position, Severity};
use crate::team::TeamView;
use crate::violation::{FormationStarter, Violation, ViolationKind};

/// The `IllegalFormation` warnings of `team`: one per fielded formation
/// set that breaks the rule, in preset then set order, each on the team
/// (no slot). Only the sets a preset fields are read: the game never plays
/// the others. None on a save other than PES 2017's or PES 2019's: the
/// rule was read from PES 2017's executable and seen to hold in PES 2017's
/// and PES 2019's formation screens, and another version's screens are
/// unseen, so no rule is guessed for it.
pub(crate) fn illegal_formations(team: &TeamView) -> Vec<Violation> {
    match team.version {
        PesVersion::Pes17 | PesVersion::Pes19 => {}
        PesVersion::Pes15
        | PesVersion::Pes16
        | PesVersion::Pes18
        | PesVersion::Pes20
        | PesVersion::Pes21 => return Vec::new(),
    }
    let mut out = Vec::new();
    for preset in 1..=3 {
        for (formation, set) in team.fielded_formations(preset) {
            let breaks = set.illegal_breaks();
            if breaks.is_empty() {
                continue;
            }
            out.push(Violation {
                slot: None,
                severity: Severity::Warning,
                kind: ViolationKind::IllegalFormation {
                    preset,
                    formation,
                    breaks,
                    eleven: eleven(team, set),
                },
            });
        }
    }
    out
}

/// `set`'s eleven formation slots: each slot's starter (the roster slot the
/// starting eleven holds at its index) by name, and his position in `set`.
fn eleven(team: &TeamView, set: &Formation) -> Box<[FormationStarter; 11]> {
    Box::new(std::array::from_fn(|index| FormationStarter {
        name: team
            .player(team.tactics.starting_eleven[index])
            .map(|player| player.name.clone()),
        position: Position(set.players[index].position),
    }))
}

#[cfg(test)]
mod tests {
    use pes_savefile::model::FormationBreak;
    use pes_savefile::{EditFile, fixture_save};

    use super::*;
    use crate::Rules;
    use crate::file::assemble;
    use crate::templates::{builtin, templates};

    /// The LB's formation slot in team 701's sets, on PES 17, 19 and 21.
    const LB_SLOT: usize = 4;
    /// The LB's position byte.
    const LB: u8 = 2;

    /// `version`'s fixture save.
    fn fixture(version: PesVersion) -> EditFile {
        EditFile::from_bytes(&fixture_save(version)).expect("fixture save")
    }

    /// Team 701's LB in preset `preset`'s set `formation` (both from 0)
    /// moved to x 2, y 10, beyond the pitch's left line.
    fn lb_beyond_the_line(file: &mut EditFile, preset: usize, formation: usize) {
        let slot = &mut file.team_mut(701).expect("team 701").tactics.presets[preset].formations
            [formation]
            .players[LB_SLOT];
        assert_eq!(slot.position, LB, "premise: formation slot 4 is the LB");
        assert_ne!((slot.x, slot.y), (2, 10), "premise: a real move");
        (slot.x, slot.y) = (2, 10);
    }

    /// Team 701's check under `rules`, every row.
    fn rows(file: &EditFile, rules: &Rules) -> Vec<Violation> {
        let team = TeamView::new(file, 701).expect("team 701 views");
        rules.check_team(&team).expect("the check runs").violations
    }

    /// An illegal formation row as the tests compare it:
    /// `(slot, severity, preset, formation, breaks)`.
    type Warning = (Option<u8>, Severity, u8, u8, Vec<FormationBreak>);

    /// Team 701's illegal formation rows under `rules`.
    fn warnings(file: &EditFile, rules: &Rules) -> Vec<Warning> {
        rows(file, rules)
            .into_iter()
            .filter_map(|violation| match violation.kind {
                ViolationKind::IllegalFormation {
                    preset,
                    formation,
                    breaks,
                    ..
                } => Some((
                    violation.slot,
                    violation.severity,
                    preset,
                    formation,
                    breaks,
                )),
                _ => None,
            })
            .collect()
    }

    /// The one warning of a set `(preset, formation)` whose LB stands
    /// beyond the line.
    fn lb_warning(preset: u8, formation: u8) -> Warning {
        (
            None,
            Severity::Warning,
            preset,
            formation,
            vec![FormationBreak::OutsideMovableRange { slot: LB_SLOT }],
        )
    }

    fn spring26() -> Rules {
        builtin("spring26").expect("spring26 is built in")
    }

    #[test]
    fn a_fielded_set_the_game_could_not_make_warns_unless_the_ruleset_allows_it() {
        let mut file = fixture(PesVersion::Pes17);
        assert!(
            !spring26().ruleset().universal.illegal_formations,
            "premise: Spring 26 does not allow illegal formations"
        );
        assert_eq!(
            warnings(&file, &spring26()),
            [],
            "premise: no warning before"
        );
        lb_beyond_the_line(&mut file, 0, 0);
        assert_eq!(warnings(&file, &spring26()), [lb_warning(1, 1)]);
        let dbc7 = builtin("dbc7").expect("dbc7 is built in");
        assert!(
            dbc7.ruleset().universal.illegal_formations,
            "premise: DBC7 allows illegal formations"
        );
        assert_eq!(warnings(&file, &dbc7), []);
    }

    #[test]
    fn an_unfielded_set_gives_no_warning() {
        let mut file = fixture(PesVersion::Pes17);
        lb_beyond_the_line(&mut file, 0, 1);
        let style = &file.team(701).expect("team 701").tactics.presets[0].style;
        assert_eq!(style.fluid, Some(false), "premise: Fluid formation is off");
        assert_eq!(warnings(&file, &spring26()), []);
        file.team_mut(701).expect("team 701").tactics.presets[0]
            .style
            .fluid = Some(true);
        assert_eq!(warnings(&file, &spring26()), [lb_warning(1, 2)]);
    }

    #[test]
    fn pes_2019_warns_as_pes_2017_does_and_pes_2021_never() {
        let mut file = fixture(PesVersion::Pes19);
        assert_eq!(
            warnings(&file, &spring26()),
            [],
            "premise: no warning before"
        );
        lb_beyond_the_line(&mut file, 0, 0);
        assert_eq!(warnings(&file, &spring26()), [lb_warning(1, 1)]);

        let mut file = fixture(PesVersion::Pes21);
        lb_beyond_the_line(&mut file, 0, 0);
        for template in templates() {
            let rules = builtin(template.stem).expect("a built-in ruleset");
            assert_eq!(warnings(&file, &rules), [], "{}", template.stem);
        }
    }

    /// Two broken sets give two warnings in preset order, among the team's
    /// own rows after every player's and before the script's: a team-level
    /// custom check stands for the script's team rows.
    #[test]
    fn two_broken_sets_warn_in_preset_order_before_the_scripts_team_rows() {
        let text = assemble(include_str!("../rulesets/spring26.rhai")).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { [#{ kind: \"Custom\", text: \"team\" }] }",
            1,
        );
        let rules = Rules::load(&text).expect("the file loads");
        let mut file = fixture(PesVersion::Pes17);
        lb_beyond_the_line(&mut file, 0, 0);
        lb_beyond_the_line(&mut file, 1, 0);
        assert_eq!(
            warnings(&file, &rules),
            [lb_warning(1, 1), lb_warning(2, 1)]
        );

        let found = rows(&file, &rules);
        let team_rows: Vec<&ViolationKind> = found
            .iter()
            .skip_while(|violation| violation.slot.is_some())
            .map(|violation| &violation.kind)
            .collect();
        assert!(
            found.len() > team_rows.len(),
            "premise: players' rows come first"
        );
        assert!(
            matches!(
                team_rows[..],
                [
                    ViolationKind::IllegalFormation { preset: 1, .. },
                    ViolationKind::IllegalFormation { preset: 2, .. },
                    ..,
                    ViolationKind::Custom { .. }
                ]
            ),
            "{team_rows:?}"
        );
    }

    /// The warning names each formation slot's starter, the roster slot
    /// the starting eleven holds at its index, with his position in the
    /// set; an empty starting slot names nobody.
    #[test]
    fn the_warning_carries_each_slots_starter_and_position() {
        let mut file = fixture(PesVersion::Pes19);
        lb_beyond_the_line(&mut file, 0, 0);
        let team = file.team_mut(701).expect("team 701");
        assert_eq!(
            team.tactics.starting_eleven[LB_SLOT], 1,
            "premise: the LB's formation slot holds roster slot 1, not 4"
        );
        team.tactics.starting_eleven[10] = 0xFF;
        let team = file.team(701).expect("team 701");
        let lb_name = file
            .player(team.roster[1].player_id)
            .expect("roster slot 1's player")
            .name
            .clone();
        let found = rows(&file, &spring26());
        let eleven = found
            .iter()
            .find_map(|violation| match &violation.kind {
                ViolationKind::IllegalFormation { eleven, .. } => Some(eleven),
                _ => None,
            })
            .expect("the warning");
        assert_eq!(
            eleven[LB_SLOT],
            FormationStarter {
                name: Some(lb_name),
                position: Position(LB),
            }
        );
        assert_eq!(eleven[10].name, None);
        assert_eq!(eleven[10].position, Position(12));
    }
}
