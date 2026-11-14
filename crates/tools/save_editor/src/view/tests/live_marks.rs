//! Live AATF's marks on the editor itself (SE-GUI-36): the toolbar's
//! framed toggle, the list rows' `✖`/`✔`, and the selected player's field
//! crosses.

use pes_savefile::interchange::team_toml::labels::ABILITY_STATS;

use crate::view::live_marks;

use super::*;

/// `view` on the PES 17 fixture save under `test`, team 734 open with
/// its first player selected — `open_734_with` without the height edit.
fn open_734(test: &str) -> (SaveView, Headless) {
    let path = fixture(PesVersion::Pes17, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(734);
    view.selected = Some(Selection::Player(734, 0));
    (view, Headless::sized(DEFAULT_PANE.x, 800.0))
}

/// The painted text shapes whose galley text is exactly `text` (a mark's
/// `✖` or `✔`).
fn mark_shapes<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    text: &str,
) -> Vec<&'a egui::epaint::TextShape> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
            _ => None,
        })
        .collect()
}

/// `text`'s galley width at the body font: egui truncates at a character
/// boundary, so a truncated galley may end up to one character short.
fn body_text_width(window: &mut Headless, text: &str) -> f32 {
    window
        .frame(Vec::new(), |ui| {
            ui.painter()
                .layout_no_wrap(
                    text.to_owned(),
                    egui::TextStyle::Body.resolve(ui.style()),
                    egui::Color32::WHITE,
                )
                .size()
                .x
        })
        .0
}

/// `ch`'s galley width at the body font (`body_text_width` on it).
fn body_char_width(window: &mut Headless, ch: char) -> f32 {
    body_text_width(window, &ch.to_string())
}

/// The narrowest of `l`, `i`, `I`, `.` at the body font and its width:
/// the char the truncation tests' long names are runs of, narrow enough
/// that a four-point room error crosses a character boundary.
fn narrowest_char_width(window: &mut Headless) -> (char, f32) {
    ['l', 'i', 'I', '.']
        .into_iter()
        .map(|ch| (ch, body_char_width(window, ch)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("four candidates")
}

/// A `Violation` on `slot` of `severity` and `kind`.
fn violation(
    slot: Option<u8>,
    severity: aatf::Severity,
    kind: aatf::ViolationKind,
) -> aatf::Violation {
    aatf::Violation {
        slot,
        severity,
        kind,
    }
}

/// A team's successful check holding `violations`, read under no named
/// system (the marks read the violations alone).
fn checked(violations: Vec<aatf::Violation>) -> crate::session::rules::TeamCheck {
    Ok(aatf::TeamCheck {
        violations,
        medal_system: None,
        height_system: None,
    })
}

/// `result` injected as the dock's held check, then one frame asserting
/// none started: a key that differs from the frame's own is due, and a
/// real run's reply would overwrite the injected result mid-test. Returns
/// the frame's nodes and shapes.
fn live_result_held(
    window: &mut Headless,
    view: &mut SaveView,
    result: (aatf_check::CheckKey, Vec<crate::session::rules::TeamCheck>),
) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
    view.live.check.result = Some(result);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    assert!(
        view.live.check.pending.is_none(),
        "the injected result's key differs from the frame's: a real check started"
    );
    (nodes, shapes)
}

/// `marked_field` maps each kind to the field it is about, `None` for the
/// row- and team-only kinds, one arm per kind.
#[test]
fn marked_field_maps_every_kind() {
    use aatf::ViolationKind as K;
    use live_marks::MarkedField as F;
    let stat = |name: &str| aatf::Stat::try_from(name.to_owned()).expect("a stat name");
    let position = || aatf::Position::try_from("CB".to_owned()).expect("a position name");
    let cases: &[(K, Option<F>)] = &[
        (
            K::RegisteredNotA { rated_a: vec![] },
            Some(F::RegisteredPosition),
        ),
        (
            K::FieldedNotA {
                preset: 1,
                position: position(),
                rated_a: vec![],
            },
            Some(F::Pitch),
        ),
        (K::BRating { rated_b: vec![] }, Some(F::PlayablePositions)),
        (K::GkSecondA { rated_a: vec![] }, Some(F::PlayablePositions)),
        (
            K::Age {
                age: 30,
                min: 15,
                max: 50,
            },
            Some(F::Age),
        ),
        (
            K::Weight {
                weight: 80,
                min: 30,
                max: 100,
            },
            Some(F::Weight),
        ),
        (K::PositionRange, Some(F::RegisteredPosition)),
        (K::PlayingStyleRange, Some(F::PlayingStyle)),
        (K::NoTier { heights: vec![] }, None),
        (
            K::Stat {
                stat: stat("speed"),
                value: 99,
                target: 77,
                at_most: false,
            },
            Some(F::Stat(stat("speed"))),
        ),
        (
            K::StatTotal {
                total: 800,
                limit: 700,
            },
            None,
        ),
        (
            K::Form {
                value: 2,
                target: 4,
            },
            Some(F::Form),
        ),
        (
            K::InjuryResistance { value: 2, max: 1 },
            Some(F::InjuryResistance),
        ),
        (
            K::WeakFootUsage { value: 3, max: 2 },
            Some(F::WeakFootUsage),
        ),
        (
            K::WeakFootAccuracy { value: 3, max: 2 },
            Some(F::WeakFootAccuracy),
        ),
        (
            K::NameColor {
                color: Some("cc9900ff".to_owned()),
                expected: None,
            },
            Some(F::Name),
        ),
        (K::Cards { paid: 4, limit: 3 }, Some(F::Skills)),
        (K::MinCards { paid: 1, min: 3 }, Some(F::Skills)),
        (K::SkillCardCap { count: 11, max: 10 }, Some(F::Skills)),
        (K::ComStyleCap { count: 3, max: 2 }, Some(F::ComStyles)),
        (
            K::RequiredCard {
                card: aatf::Card::Skill(0),
            },
            Some(F::Skills),
        ),
        (
            K::RequiredCard {
                card: aatf::Card::ComStyle(1),
            },
            Some(F::ComStyles),
        ),
        (
            K::APositions {
                count: 2,
                max: 1,
                rated_a: vec![],
            },
            Some(F::PlayablePositions),
        ),
        (
            K::Height {
                height: 183,
                fits: vec![],
                room: false,
            },
            Some(F::Height),
        ),
        (
            K::TierCount {
                tier: "gold".to_owned(),
                count: 0,
                expected: 1,
            },
            None,
        ),
        (
            K::BracketQuota {
                system: "red".to_owned(),
                bracket: "tall".to_owned(),
                count: 8,
                quota: 10,
                players: vec![(4, 185)],
            },
            None,
        ),
        (
            K::MedalSystem {
                systems: vec!["sys".to_owned()],
            },
            None,
        ),
        (
            K::ExtraHeight {
                used: 5,
                total: 4,
                draws: vec![(4, "tall".to_owned(), 5)],
            },
            None,
        ),
        (
            K::NearestSystem {
                system: "height".to_owned(),
                off: vec![("mid".to_owned(), 6, 5)],
            },
            None,
        ),
        (K::Captain, None),
        (K::Goalkeepers { count: 0, min: 1 }, None),
        (
            K::TeamSetting {
                label: "x".to_owned(),
                setting: "y".to_owned(),
                wanted: false,
            },
            None,
        ),
        (
            K::FormationShape {
                preset: 1,
                formation: 2,
                shape: [4, 4, 2],
            },
            None,
        ),
        (
            K::Special {
                label: "x".to_owned(),
                text: "y".to_owned(),
            },
            None,
        ),
        (
            K::Custom {
                text: "y".to_owned(),
            },
            None,
        ),
        (
            K::UnusedChoice {
                special: "x".to_owned(),
            },
            None,
        ),
        (K::RosterOrder { player: 73401 }, None),
        (
            K::IllegalFormation {
                preset: 1,
                formation: 1,
                breaks: vec![pes_savefile::model::FormationBreak::KeeperOffSpot],
                eleven: Box::new(std::array::from_fn(|_| aatf::FormationStarter {
                    name: None,
                    position: aatf::Position::try_from("GK".to_owned()).expect("GK"),
                })),
            },
            None,
        ),
    ];
    for (kind, field) in cases {
        assert_eq!(live_marks::marked_field(kind), *field, "{kind:?}");
    }
    // A suggestion marks the field of its allowance: each `suggestions`
    // word, then the card groups `com`, `tricks` and any other (`choices`
    // raises `UnusedChoice`, above).
    let unused = |allowance: &str| K::Unused {
        allowance: allowance.to_owned(),
        used: 1,
        allowed: 2,
    };
    for (allowance, field) in [
        ("cards", F::Skills),
        ("a_positions", F::PlayablePositions),
        ("weak_foot_usage", F::WeakFootUsage),
        ("weak_foot_accuracy", F::WeakFootAccuracy),
        ("injury_resistance", F::InjuryResistance),
        ("height", F::Height),
        ("free_cards", F::AsideCards),
        ("com", F::ComStyles),
        ("tricks", F::AsideCards),
        ("dribbling", F::Skills),
    ] {
        assert_eq!(
            live_marks::marked_field(&unused(allowance)),
            Some(field),
            "Unused {allowance}"
        );
    }
}

/// `row_mark` counts the slot's errors, warnings and suggestions — a zero
/// count left out, the other slots' rows counted nowhere — and draws a
/// `✖` in the error colour on an Error, else a `✔`: amber on a Warning,
/// blue on suggestions alone, green on none (the draft's `rowMark`).
#[test]
fn row_mark_counts_the_slots_rows_and_picks_its_glyph() {
    use aatf::Severity::*;
    let visuals = egui::Visuals::dark();
    let success = studio_core::theme::notice_color(studio_core::NoticeKind::Success, &visuals);
    let v = |slot, severity| violation(slot, severity, aatf::ViolationKind::Captain);
    let cases: &[(&[aatf::Violation], &str, char, egui::Color32)] = &[
        (&[v(Some(1), Error)], "1 error", '✖', visuals.error_fg_color),
        (
            &[v(Some(1), Error), v(Some(1), Error), v(Some(1), Warning)],
            "2 errors, 1 warning",
            '✖',
            visuals.error_fg_color,
        ),
        (
            &[v(Some(1), Warning), v(Some(1), Suggestion)],
            "1 warning, 1 suggestion",
            '✔',
            visuals.warn_fg_color,
        ),
        (
            &[v(Some(1), Suggestion)],
            "1 suggestion",
            '✔',
            visuals.hyperlink_color,
        ),
        (&[], "No issues", '✔', success),
    ];
    for (violations, tooltip, glyph, color) in cases {
        let mark = live_marks::row_mark(violations, Some(1));
        assert_eq!(
            (mark.tooltip.as_str(), mark.glyph(&visuals)),
            (*tooltip, (*glyph, *color)),
            "{violations:?}"
        );
    }
    assert_ne!(
        visuals.hyperlink_color, success,
        "premise: the blue and the green differ"
    );
    // The team's own violations (`slot: None`) mark the team row alone,
    // and a player's his alone.
    let mark = live_marks::row_mark(&[v(Some(1), Error), v(None, Warning)], None);
    assert_eq!(
        (mark.tooltip.as_str(), mark.glyph(&visuals)),
        ("1 warning", ('✔', visuals.warn_fg_color))
    );
    let mark = live_marks::row_mark(
        &[v(None, Error), v(None, Warning), v(Some(1), Error)],
        Some(1),
    );
    assert_eq!(
        (mark.tooltip.as_str(), mark.glyph(&visuals)),
        ("1 error", ('✖', visuals.error_fg_color))
    );
}

/// `held_violations` is the held check only while the toggle is on and
/// the result one team's `Ok` under the active ruleset — a failed check,
/// another shape of result, another ruleset's or nothing yet marks
/// nothing.
#[test]
fn held_violations_is_one_teams_ok_result_while_on() {
    let mut check = aatf_check::TeamsCheck::new();
    let key = |ids: &[u32]| aatf_check::CheckKey {
        revision: 1,
        ids: ids.to_vec(),
        ruleset: "autumn26".to_owned(),
        suggestions: false,
    };
    let held = |check: &_| {
        live_marks::held_violations(true, true, check, "autumn26").map(|(id, v)| (id, v.len()))
    };
    check.result = Some((
        key(&[734]),
        vec![checked(vec![violation(
            None,
            aatf::Severity::Error,
            aatf::ViolationKind::Captain,
        )])],
    ));
    assert_eq!(held(&check), Some((734, 1)));
    assert_eq!(
        live_marks::held_violations(false, true, &check, "autumn26"),
        None,
        "off marks nothing"
    );
    assert_eq!(
        live_marks::held_violations(true, true, &check, "spring26"),
        None,
        "checked under another ruleset"
    );
    check.result = Some((key(&[734]), vec![Err("boom".to_owned())]));
    assert_eq!(held(&check), None, "a failed check");
    check.result = Some((key(&[734, 735]), vec![checked(vec![]), checked(vec![])]));
    assert_eq!(held(&check), None, "not one team's");
    check.result = None;
    assert_eq!(held(&check), None, "nothing held yet");
}

/// The toggle is a framed `▶ Live AATF` button; the list marks the
/// broken player's row `✖` and a clean one's `✔`; the selected player's
/// field the violation is about carries a cross whose tooltip is the
/// violation's own line; fixing it clears the marks; off, none are
/// drawn.
// SE-GUI-36
#[test]
fn the_marks_follow_the_held_check() {
    let (mut view, mut window) = open_734_with("live_marks_gui36", &[73401], 183);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();

    // The toolbar's toggle is a framed button: the inactive fill off,
    // the selection fill on.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let toggle = node_labeled(&nodes, Role::Button, "\u{25b6} Live AATF");
    let bounds = toggle.bounds().expect("the toggle's bounds");
    assert_eq!(
        fill_at(&shapes, bounds),
        Some(visuals.widgets.inactive.weak_bg_fill),
        "the off toggle is a framed button"
    );
    click(&mut window, toggle, &mut view);
    assert!(view.live_aatf, "the click turned Live AATF on");
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        fill_at(&shapes, bounds),
        Some(visuals.selection.bg_fill),
        "the on toggle takes the selection fill"
    );

    // The first check returned: `✖` on the broken player's row in the
    // error colour, `✔` on the clean one's in the success colour, at
    // each row's right end, the tooltips the counts.
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for (name, glyph, color, tip) in [
        ("Muh Spooks", "\u{2716}", visuals.error_fg_color, "1 error"),
        (
            "Yuki-kun ^~^",
            "\u{2714}",
            studio_core::theme::notice_color(studio_core::NoticeKind::Success, &visuals),
            "No issues",
        ),
    ] {
        let row = nodes
            .iter()
            .find(|node| player_row(node) && node.label().is_some_and(|label| label.contains(name)))
            .unwrap_or_else(|| panic!("{name}'s row"));
        let bounds = row.bounds().expect("the row's bounds");
        let marks = marks_in_row(&shapes, bounds);
        let [mark] = *marks.as_slice() else {
            panic!("expected one mark on {name}'s row, got {}", marks.len())
        };
        assert_eq!(mark.galley.job.text, glyph, "{name}'s mark's glyph");
        assert_eq!(galley_color(mark), color, "{name}'s mark's colour");
        let tip_at = egui::pos2(mark.pos.x + 2.0, mark.pos.y + 2.0);
        assert_eq!(
            tooltip_containing(&mut window, &mut view, tip_at, tip).as_deref(),
            Some(tip),
            "{name}'s mark's tooltip"
        );
    }

    // The broken player selected: one `✖` in the view (the list's marks
    // sit left of x 250), left of the Height label; its tooltip the
    // violation's own line.
    let height_box = control_right_of(&nodes, "Height")
        .bounds()
        .expect("the Height box's bounds");
    let crosses = field_marks(&shapes);
    let [cross] = crosses.as_slice() else {
        panic!(
            "expected one field cross in the view, got {}",
            crosses.len()
        )
    };
    assert_eq!(cross.galley.job.text, "\u{2716}", "a cross");
    assert_left_of_label(cross, label_bounds(&nodes, "Height"), "Height");
    let tip_at = mark_centre(cross);
    assert_eq!(
        tooltip_containing(&mut window, &mut view, tip_at, "takes no bracket").as_deref(),
        Some("Height: height 183 takes no bracket (185 has room)"),
        "the cross's tooltip is the violation's line"
    );

    // The Height box's x is the same with Live AATF off, and no mark is
    // drawn anywhere then.
    view.live_aatf = false;
    let (_, nodes_off, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let off_box = control_right_of(&nodes_off, "Height")
        .bounds()
        .expect("the Height box's bounds");
    assert_eq!(off_box.x0, height_box.x0, "the box keeps its x");
    let (_, _, shapes_off) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        mark_shapes(&shapes_off, "\u{2716}").is_empty(),
        "no cross with Live AATF off"
    );
    assert!(
        mark_shapes(&shapes_off, "\u{2714}").is_empty(),
        "no check mark with Live AATF off"
    );

    // The height fixed and the check returned: both rows `✔`, no cross
    // in the view.
    view.live_aatf = true;
    let mut entry = player_entry(&view, 73401);
    entry.basic.height = 185;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The players' rows are `✔`, and no field carries a cross any
    // longer (the team row's `✖` stays: the tier counts are the team's
    // own, broken from the start).
    assert!(
        mark_shapes(&shapes, "\u{2716}")
            .into_iter()
            .all(|shape| shape.pos.x < 260.0),
        "no field cross once the height is fixed"
    );
    for name in ["Muh Spooks", "Yuki-kun ^~^"] {
        let row = nodes
            .iter()
            .find(|node| player_row(node) && node.label().is_some_and(|label| label.contains(name)))
            .unwrap_or_else(|| panic!("{name}'s row"));
        let bounds = row.bounds().expect("the row's bounds");
        let marks = marks_in_row(&shapes, bounds);
        let [mark] = *marks.as_slice() else {
            panic!("expected one mark on {name}'s row, got {}", marks.len())
        };
        assert_eq!(mark.galley.job.text, "\u{2714}", "{name}'s mark's glyph");
    }
}

/// A result checked under 4cc Autumn 26 marks nothing once 4cc Spring 26
/// is picked: its marks' tooltips would be worded with the other
/// ruleset's tiers. The broken player's row mark and his Height cross go
/// until the recheck returns, then Spring 26's result marks his row
/// again. The pick lands while the pointer is held, so the recheck waits
/// for the release: a started check can reply between a frame's two
/// passes.
// SE-GUI-36
#[test]
fn a_result_checked_under_another_ruleset_marks_nothing() {
    let (mut view, mut window) = open_734_with("live_marks_other_ruleset", &[73401], 183);
    view.live_aatf = true;
    let broken_row_marks = |nodes: &[accesskit::Node], shapes| {
        let row = nodes
            .iter()
            .find(|node| {
                player_row(node)
                    && node
                        .label()
                        .is_some_and(|label| label.contains("Muh Spooks"))
            })
            .expect("Muh Spooks's row");
        marks_in_row(shapes, row.bounds().expect("the row's bounds")).len()
    };
    live_checked(&mut window, &mut view, &[734]);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        broken_row_marks(&nodes, &shapes),
        1,
        "premise: Autumn 26's result marks his row"
    );
    assert_eq!(
        field_marks(&shapes).len(),
        1,
        "premise: Autumn 26's result crosses his Height"
    );

    // The press lands on the toolbar's empty end.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    view.aatf_ruleset = "spring26".to_owned();
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.live.check.pending.is_none()
            && view
                .live
                .check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ruleset == "autumn26"),
        "premise: Autumn 26's result held, no recheck started"
    );
    assert_eq!(
        broken_row_marks(&nodes, &shapes),
        0,
        "the held result marks no row"
    );
    assert!(
        field_marks(&shapes).is_empty(),
        "the held result crosses no field"
    );

    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    live_checked(&mut window, &mut view, &[734]);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        broken_row_marks(&nodes, &shapes),
        1,
        "Spring 26's result marks his row"
    );
}

/// The view's field marks (`✖`, `⚠`, `ⓘ`) painted right of x 250 (the
/// player list's own marks stand left of it).
fn field_marks(shapes: &[egui::epaint::ClippedShape]) -> Vec<&egui::epaint::TextShape> {
    ["\u{2716}", "\u{26a0}", "\u{24d8}"]
        .into_iter()
        .flat_map(|text| mark_shapes(shapes, text))
        .filter(|shape| shape.pos.x > 250.0)
        .collect()
}

/// A field mark's centre: its glyph's galley's, the centre of the
/// 11-point box it is drawn in.
fn mark_centre(mark: &egui::epaint::TextShape) -> egui::Pos2 {
    mark.pos + mark.galley.rect.size() / 2.0
}

/// The one field mark whose centre lies on `label`'s row.
fn mark_on_row<'a>(
    marks: &[&'a egui::epaint::TextShape],
    label: accesskit::Rect,
    what: &str,
) -> &'a egui::epaint::TextShape {
    let on_row: Vec<_> = marks
        .iter()
        .filter(|mark| {
            let y = f64::from(mark_centre(mark).y);
            y >= label.y0 - 1.0 && y <= label.y1 + 1.0
        })
        .collect();
    let [mark] = on_row.as_slice() else {
        panic!(
            "{what}: expected one mark on its label's row, got {}",
            on_row.len()
        )
    };
    mark
}

/// Asserts `mark`'s 11-point box ends 2 points (± 0.5) before `label`'s
/// text. Both figures are written out, so a changed constant fails.
fn assert_left_of_label(mark: &egui::epaint::TextShape, label: accesskit::Rect, what: &str) {
    let box_right = f64::from(mark_centre(mark).x) + 5.5;
    assert!(
        (label.x0 - box_right - 2.0).abs() <= 0.5,
        "{what}: the mark's box ends at {box_right}, 2 points before the label at {}",
        label.x0
    );
}

/// Asserts an ability stat's `mark` has its 11-point box end 2 points
/// (± 0.5) before the stats' label column, whose left edge is the leftmost
/// stat label's (the labels are left-aligned, so every one starts the
/// column). The view is the PES 17 fixture save's, so the labels are
/// PES 2017's names.
fn assert_before_stat_column(
    mark: &egui::epaint::TextShape,
    nodes: &[accesskit::Node],
    what: &str,
) {
    let column_left = ABILITY_STATS
        .iter()
        .map(|name| {
            label_bounds(
                nodes,
                &pes_savefile::interchange::team_toml::labels::stat_name(PesVersion::Pes17, name),
            )
            .x0
        })
        .fold(f64::MAX, f64::min);
    let box_right = f64::from(mark_centre(mark).x) + 5.5;
    assert!(
        (column_left - box_right - 2.0).abs() <= 0.5,
        "{what}: the mark's box ends at {box_right}, 2 points before the label column at {column_left}"
    );
}

/// The Label node whose value starts with `prefix` (a counted label's
/// value carries its count).
fn label_starting(nodes: &[accesskit::Node], prefix: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label
                && node.value().is_some_and(|value| value.starts_with(prefix))
        })
        .and_then(accesskit::Node::bounds)
        .unwrap_or_else(|| panic!("no label starting {prefix:?}"))
}

/// Only the checked team's rows are marked: with ALL picked another
/// team's rows draw neither `✖` nor `✔`, and the checked team's keep
/// theirs: players of other teams carry none.
#[test]
fn only_the_checked_teams_rows_are_marked() {
    let (mut view, mut window) = open_734_with("live_marks_other_teams", &[73401], 183);
    view.live_aatf = true;
    let _nodes = live_checked(&mut window, &mut view, &[734]);
    view.team = TeamChoice::All;
    // Under ALL his row sits below the fold: the wheel scrolls the list
    // until it shows (bounded, so an unreachable one fails rather than
    // hangs).
    let mut found = false;
    let at = egui::pos2(120.0, 300.0);
    for _ in 0..64 {
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let his_row = nodes.iter().find(|node| {
            player_row(node)
                && node
                    .label()
                    .is_some_and(|label| label.contains("Muh Spooks"))
        });
        if let Some(his_row) = his_row {
            let bounds = his_row.bounds().expect("the row's bounds");
            assert!(
                mark_shapes(&shapes, "\u{2716}").into_iter().any(|shape| {
                    f64::from(shape.pos.y) >= bounds.y0
                        && f64::from(shape.pos.y) <= bounds.y1
                        && f64::from(shape.pos.x) >= bounds.x0
                        && f64::from(shape.pos.x) <= bounds.x1
                }),
                "the broken player's `✖` stands"
            );
            // Every mark drawn in the list sits inside a row of his
            // team (its own or the team row's) — the other teams' rows
            // carry none.
            let team_bounds = team_row(&nodes, &view, 734)
                .bounds()
                .expect("the team row's bounds");
            // His team's rows are the player rows between its team row
            // and the next team row's (a row's label names no team).
            let file = view.session.as_ref().unwrap().file();
            let next_y0 = file
                .teams()
                .iter()
                .filter_map(|team| {
                    let label = format!("{} {}", team.id, display_name(&team.name));
                    nodes
                        .iter()
                        .find(|node| {
                            node.role() == Role::Button && node.label() == Some(label.as_str())
                        })
                        .and_then(|node| node.bounds())
                        .map(|bounds| bounds.y0)
                })
                .filter(|y0| *y0 > team_bounds.y0)
                .fold(f64::MAX, f64::min);
            let mut his_rows: Vec<accesskit::Rect> = nodes
                .iter()
                .filter_map(|node| {
                    (player_row(node)
                        && node.bounds().is_some_and(|bounds| {
                            bounds.y0 >= team_bounds.y1 && bounds.y0 < next_y0
                        }))
                    .then(|| node.bounds())
                    .flatten()
                })
                .collect();
            his_rows.push(team_bounds);
            for glyph in ["\u{2716}", "\u{2714}"] {
                for shape in mark_shapes(&shapes, glyph)
                    .into_iter()
                    .filter(|shape| shape.pos.x < 250.0)
                {
                    assert!(
                        his_rows.iter().any(|bounds| {
                            f64::from(shape.pos.y) >= bounds.y0
                                && f64::from(shape.pos.y) <= bounds.y1
                                && f64::from(shape.pos.x) >= bounds.x0
                                && f64::from(shape.pos.x) <= bounds.x1
                        }),
                        "a {glyph} outside his team's rows at {:?}",
                        shape.pos
                    );
                }
            }
            found = true;
            break;
        }
        window.frame(
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -400.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    assert!(found, "his row never scrolled into view");
}

mod marks;
mod pitch;
