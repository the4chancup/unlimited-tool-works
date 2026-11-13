//! The illegal formation warning in the Live AATF list (SE-GUI-76): raised
//! by a set the game's screens could not make, gone once a drag on the
//! Tactics pitch repairs it, never under a ruleset that allows illegal
//! formations or on a PES 2021 save.

use super::*;

/// The LB's formation slot in team 701's Kick-off set, on PES 17 and 21.
const LB_SLOT: usize = 4;

/// `version`'s fixture save under `test`, team 701's Kick-off LB moved to
/// x 2, y 10, beyond the pitch's left line (SE-AATF-09's save on PES 17).
fn save_with_lb_beyond_the_line(version: PesVersion, test: &str) -> PathBuf {
    let path = fixture(version, test);
    let mut file = EditFile::load(&path).unwrap();
    let slot = &mut file.team_mut(701).unwrap().tactics.presets[0].formations[0].players[LB_SLOT];
    assert_eq!(slot.position, 2, "premise: formation slot 4 is the LB");
    (slot.x, slot.y) = (2, 10);
    file.save(&path).unwrap();
    path
}

/// The list rows' lines in `nodes` naming an illegal formation (a team
/// row only reports: a Label node, its line its value).
fn illegal_formation_rows(nodes: &[accesskit::Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|node| node.label().or(node.value()))
        .filter(|line| line.starts_with("Illegal formation:"))
        .map(str::to_owned)
        .collect()
}

// SE-GUI-76
#[test]
fn an_illegal_formation_warns_in_the_list_until_a_drag_repairs_it() {
    let path = save_with_lb_beyond_the_line(PesVersion::Pes17, "live_aatf_illegal_formation");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "spring26".to_owned();
    view.live_aatf = true;
    let mut window = Headless::sized(1600.0, 1400.0);
    open_team_view(&mut window, &mut view, 701);
    let team = team_entry(&view, 701);
    let lb_id = team.roster[usize::from(team.tactics.starting_eleven[LB_SLOT])].player_id;
    let warning = format!(
        "Illegal formation: Preset 1, Kick-off: illegal formation. {} (LB) is outside the \
         area a starter can be moved in",
        display_name(&player_entry(&view, lb_id).name)
    );

    // Team 701's row selected: the list holds the warning.
    let nodes = live_checked(&mut window, &mut view, &[701]);
    assert_eq!(
        illegal_formation_rows(&nodes),
        std::slice::from_ref(&warning)
    );
    let held = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    assert!(
        held.violations.iter().any(|violation| matches!(
            violation.kind,
            aatf::ViolationKind::IllegalFormation { .. }
        ) && violation.severity == aatf::Severity::Warning),
        "the row is a warning"
    );

    // The LB dragged on the Tactics pitch to (12, 10), where the game
    // would put him: the warning is gone. The drop point is his badge's
    // there plus the press's offset from his own, since a held marker
    // stands at the pointer less it.
    let canvas = canvas_of(&draw(&mut window, &mut view));
    let players = team_entry(&view, 701).tactics.presets[0].formations[0].players;
    let from = marker_point(canvas, &players, LB_SLOT).expect("a point on the LB's marker");
    let target = FormationSlot {
        position: 2,
        x: 12,
        y: 10,
    };
    let onto = badge_centre(canvas, target) + (from - badge_centre(canvas, players[LB_SLOT]));
    drag(&mut window, from, onto, &mut view);
    assert_eq!(
        team_entry(&view, 701).tactics.presets[0].formations[0].players[LB_SLOT],
        target,
        "premise: the drop stored the LB at (12, 10)"
    );
    let nodes = live_checked(&mut window, &mut view, &[701]);
    assert_eq!(illegal_formation_rows(&nodes), Vec::<String>::new());

    // The drag undone, the warning back; DBC7 picked instead: none.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = live_checked(&mut window, &mut view, &[701]);
    assert_eq!(
        illegal_formation_rows(&nodes),
        [warning],
        "premise: the undo brought the set back"
    );
    view.aatf_ruleset = "dbc7".to_owned();
    let nodes = live_checked(&mut window, &mut view, &[701]);
    assert_eq!(illegal_formation_rows(&nodes), Vec::<String>::new());

    // A PES 21 save with the same set: none under any built-in ruleset.
    let path = save_with_lb_beyond_the_line(PesVersion::Pes21, "live_aatf_illegal_formation_21");
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Team(701));
    for template in aatf::templates() {
        view.aatf_ruleset = template.stem.to_owned();
        let nodes = live_checked(&mut window, &mut view, &[701]);
        assert_eq!(
            illegal_formation_rows(&nodes),
            Vec::<String>::new(),
            "{}",
            template.stem
        );
    }
}
