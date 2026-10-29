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
            },
            None,
        ),
        (
            K::MedalSystem {
                systems: vec!["sys".to_owned()],
            },
            None,
        ),
        (K::ExtraHeight { used: 5, total: 4 }, None),
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
        ("weak_foot", F::WeakFootUsage),
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
/// `✖` in the error colour on an Error, else a `✔`: orange on a Warning,
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
        (&[], "No errors", '✔', success),
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
/// the result one team's `Ok` — a failed check, another shape of result
/// or nothing yet marks nothing (the Live AATF bullet's last sub-bullet).
#[test]
fn held_violations_is_one_teams_ok_result_while_on() {
    let mut check = aatf_check::TeamsCheck::new();
    let key = |ids: &[u32]| aatf_check::CheckKey {
        revision: 1,
        ids: ids.to_vec(),
        ruleset: "autumn26".to_owned(),
        suggestions: false,
    };
    let held = |check: &_| live_marks::held_violations(true, check).map(|(id, v)| (id, v.len()));
    check.result = Some((
        key(&[734]),
        vec![Ok(vec![violation(
            None,
            aatf::Severity::Error,
            aatf::ViolationKind::Captain,
        )])],
    ));
    assert_eq!(held(&check), Some((734, 1)));
    assert_eq!(
        live_marks::held_violations(false, &check),
        None,
        "off marks nothing"
    );
    check.result = Some((key(&[734]), vec![Err("boom".to_owned())]));
    assert_eq!(held(&check), None, "a failed check");
    check.result = Some((key(&[734, 735]), vec![Ok(vec![]), Ok(vec![])]));
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
            "No errors",
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
/// column).
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
                &pes_savefile::interchange::team_toml::labels::title_case(name),
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

/// Three ability stats one over the regulars' 77 target (still regular:
/// a medal's recognition threshold is higher) raise a `Stat` error each:
/// the labels being left-aligned, each stat's cross stands at the label
/// column's left edge, its box ending 2 points before it whether the
/// label is long (Attacking Prowess) or short (Speed), the three crosses
/// on one centre x, and no label moved from the run with Live AATF off.
#[test]
fn the_stats_marks_line_up_at_the_label_columns_left_edge() {
    let (mut view, mut window) = open_734("live_marks_stat");
    let stats = [
        ("attacking_prowess", "Attacking Prowess"),
        ("ball_control", "Ball Control"),
        ("speed", "Speed"),
    ];
    let (_, nodes_off, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    view.live_aatf = true;
    let mut entry = player_entry(&view, 73401);
    for (name, _) in stats {
        let index = ABILITY_STATS
            .iter()
            .position(|stat| *stat == name)
            .expect("an ability stat");
        assert_ne!(
            entry.stats.abilities()[index],
            Some(78),
            "premise: {name} is not 78 yet"
        );
        entry.stats.set_ability(index, 78);
    }
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let violations = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    let error = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    let marks = field_marks(&shapes);
    let mut centres = Vec::new();
    for (name, label) in stats {
        assert!(
            violations.iter().any(|violation| matches!(
                &violation.kind,
                aatf::ViolationKind::Stat { stat, .. } if *stat == aatf::Stat::try_from(name.to_owned()).unwrap()
            ) && violation.severity == aatf::Severity::Error),
            "premise: a Stat error on {name}"
        );
        let bounds = label_bounds(&nodes, label);
        let mark = mark_on_row(&marks, bounds, label);
        assert_eq!(mark.galley.job.text, "\u{2716}", "{label}: a cross");
        assert_eq!(galley_color(mark), error, "{label}: the error colour");
        assert_before_stat_column(mark, &nodes, label);
        assert_eq!(
            bounds,
            label_bounds(&nodes_off, label),
            "{label}'s label did not move"
        );
        centres.push(mark_centre(mark).x);
    }
    assert!(
        centres.iter().all(|x| (x - centres[0]).abs() <= 0.01),
        "the column's crosses share one centre x: {centres:?}"
    );
}

/// One row alone on the selected player: exactly one mark painted in the
/// view, on its field's label's row, its 11-point box ending 2 points
/// before the label's text — SE-GUI-36's "the cross beside the field the
/// violation is about" for every field a kind reaches but `Pitch` and
/// `Stat` (tested elsewhere): General info's and Other stats' labels and
/// the panels' headers, the card budget on the Skills header and a
/// suggestion on its allowance's field.
#[test]
fn each_fields_mark_stands_left_of_its_label() {
    use aatf::Severity::{Error, Suggestion};
    use aatf::ViolationKind as K;
    let unused = |allowance: &str| K::Unused {
        allowance: allowance.to_owned(),
        used: 1,
        allowed: 2,
    };
    let cases: Vec<(aatf::Severity, K, &str)> = vec![
        (
            Error,
            K::Height {
                height: 183,
                fits: vec![],
                room: false,
            },
            "Height",
        ),
        (
            Error,
            K::Age {
                age: 30,
                min: 15,
                max: 50,
            },
            "Age",
        ),
        (
            Error,
            K::Weight {
                weight: 80,
                min: 30,
                max: 100,
            },
            "Weight",
        ),
        (
            Error,
            K::InjuryResistance { value: 2, max: 1 },
            "Injury resistance",
        ),
        (
            Error,
            K::WeakFootUsage { value: 3, max: 2 },
            "Weak foot: Usage",
        ),
        (
            Error,
            K::Form {
                value: 2,
                target: 4,
            },
            "Form",
        ),
        (Error, K::WeakFootAccuracy { value: 3, max: 2 }, "Accuracy"),
        (
            Error,
            K::NameColor {
                color: Some("cc9900ff".to_owned()),
                expected: None,
            },
            "Name (",
        ),
        (
            Error,
            K::RegisteredNotA { rated_a: vec![] },
            "Registered position",
        ),
        (Error, K::PlayingStyleRange, "Playing style"),
        (Error, K::BRating { rated_b: vec![] }, "Playable positions"),
        (Error, K::SkillCardCap { count: 11, max: 10 }, "Skills"),
        (Error, K::Cards { paid: 4, limit: 3 }, "Skills"),
        (
            Error,
            K::ComStyleCap { count: 3, max: 2 },
            "COM playing styles",
        ),
        (Suggestion, unused("tricks"), "Tricks"),
        (Suggestion, unused("cards"), "Skills"),
        (Suggestion, unused("weak_foot"), "Weak foot: Usage"),
    ];
    let (mut view, mut window) = open_734("live_marks_each_field");
    view.live_aatf = true;
    let revision = view.session.as_ref().expect("the save opened").revision();
    for (severity, kind, label) in &cases {
        view.live.check.result = Some((
            aatf_check::CheckKey {
                revision,
                ids: vec![734],
                ruleset: "autumn26".to_owned(),
                suggestions: false,
            },
            vec![Ok(vec![violation(Some(0), *severity, kind.clone())])],
        ));
        // The dock's bottom panel can clip a low control (the COM header
        // at ~582 sits below it at 800 tall): scroll the label into
        // view, a no-op when it already shows.
        scroll_into_view(&mut window, &mut view, |node| {
            node.value().is_some_and(|value| value.starts_with(label))
        });
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let marks = field_marks(&shapes);
        let [mark] = *marks.as_slice() else {
            panic!(
                "{kind:?}: expected one mark in the view, got {}",
                marks.len()
            )
        };
        let bounds = label_starting(&nodes, label);
        assert_eq!(
            mark_on_row(&marks, bounds, label).pos,
            mark.pos,
            "{kind:?}: the mark is on {label:?}'s row"
        );
        assert_left_of_label(mark, bounds, &format!("{kind:?}"));
    }
}

/// A warning-only violation's mark is an orange warning sign: the
/// official file gains a warning band on the regular tier, and 73401's
/// attacking prowess goes one over its 77 target (a `Stat` warning, not
/// an error).
#[test]
fn a_warnings_mark_is_an_orange_warning_sign() {
    let (mut view, mut window) = open_734("live_marks_warning");
    view.live_aatf = true;
    pick_imported(
        &mut view,
        "warn.4ccrules",
        &aatf::official_file().replacen("cards: 3,", "stat_warning: 5, cards: 3,", 1),
    );
    let mut entry = player_entry(&view, 73401);
    let prowess = ABILITY_STATS
        .iter()
        .position(|name| *name == "attacking_prowess")
        .expect("attacking_prowess is an ability stat");
    entry.stats.set_ability(prowess, 78);
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.live.check.result.as_ref().is_some_and(|(_, checks)| {
            checks[0].as_ref().is_ok_and(|violations| {
                violations.iter().any(|violation| {
                    matches!(violation.kind, aatf::ViolationKind::Stat { .. })
                        && violation.severity == aatf::Severity::Warning
                })
            })
        }),
        "premise: the check holds a Stat warning"
    );
    let label = label_bounds(&nodes, "Attacking Prowess");
    let mark = mark_on_row(&field_marks(&shapes), label, "Attacking Prowess");
    assert_before_stat_column(mark, &nodes, "Attacking Prowess");
    assert_eq!(mark.galley.job.text, "\u{26a0}", "a warning sign");
    assert_eq!(
        galley_color(mark),
        window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .warn_fg_color,
        "the warning colour"
    );
}

/// The team's own violations (`slot: None`) mark the team row — 734's
/// three tier counts are the team's own, broken from the start; a team
/// with none shows `✔` (the Live AATF bullet's last list sub-bullet).
#[test]
fn the_teams_own_violations_mark_the_team_row() {
    let (mut view, mut window) = open_734("live_marks_team_row");
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 734);
    let bounds = row.bounds().expect("the team row's bounds");
    let mark = mark_shapes(&shapes, "\u{2716}")
        .into_iter()
        .find(|shape| {
            f64::from(shape.pos.y) >= bounds.y0
                && f64::from(shape.pos.y) <= bounds.y1
                && f64::from(shape.pos.x) >= bounds.x1 - 30.0
        })
        .expect("a `✖` on the team row");
    let tip_at = egui::pos2(mark.pos.x + 2.0, mark.pos.y + 2.0);
    assert_eq!(
        tooltip_containing(&mut window, &mut view, tip_at, "3 errors").as_deref(),
        Some("3 errors"),
        "the team's own count: the three tier counts"
    );

    // A clean team's row passes (the imported 714 holds only
    // suggestions, which count as neither).
    let (mut view, mut window) = open_legal_714(
        "live_marks_team_row_clean",
        Headless::sized(DEFAULT_PANE.x, 800.0),
    );
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[714]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 714);
    let bounds = row.bounds().expect("the team row's bounds");
    assert!(
        mark_shapes(&shapes, "\u{2714}").into_iter().any(|shape| {
            f64::from(shape.pos.y) >= bounds.y0
                && f64::from(shape.pos.y) <= bounds.y1
                && f64::from(shape.pos.x) >= bounds.x1 - 30.0
        }),
        "a `✔` on the clean team's row"
    );
    assert!(
        mark_shapes(&shapes, "\u{2716}").is_empty(),
        "no cross anywhere"
    );
}

/// The row's mark is vertically centred on the row — a player row's and
/// the team row's alike (`paint_row_mark`).
#[test]
fn the_row_mark_centres_on_its_row() {
    let (mut view, mut window) = open_734("live_marks_centred");
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let player = nodes
        .iter()
        .find(|node| player_row(node))
        .expect("a player row");
    for (kind, row) in [("player", player), ("team", team_row(&nodes, &view, 734))] {
        let bounds = row.bounds().expect("the row's bounds");
        let marks = marks_in_row(&shapes, bounds);
        let [mark] = *marks.as_slice() else {
            panic!("expected one mark on the {kind} row, got {}", marks.len())
        };
        let centre = ((bounds.y0 + bounds.y1) / 2.0) as f32;
        assert!(
            (mark.pos.y + mark.galley.rect.height() / 2.0 - centre).abs() <= 0.5,
            "the {kind} row's mark centres on it ({} vs {centre})",
            mark.pos.y + mark.galley.rect.height() / 2.0
        );
    }
}

/// A click on a row's mark still selects the row: the mark senses hover
/// alone, so the click lands on the row (the Live AATF bullet's).
#[test]
fn a_click_on_a_rows_mark_selects_the_row() {
    let (mut view, mut window) = open_734_with("live_marks_mark_click", &[73401], 183);
    view.live_aatf = true;
    let nodes = live_checked(&mut window, &mut view, &[734]);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // Another player selected first: a click that did nothing would
    // satisfy the assertion anyway.
    view.selected = Some(Selection::Player(734, 3));
    let row = nodes
        .iter()
        .find(|node| {
            player_row(node)
                && node
                    .label()
                    .is_some_and(|label| label.contains("Muh Spooks"))
        })
        .expect("his row");
    let bounds = row.bounds().expect("the row's bounds");
    let mark = mark_shapes(&shapes, "\u{2716}")
        .into_iter()
        .find(|shape| {
            f64::from(shape.pos.y) >= bounds.y0
                && f64::from(shape.pos.y) <= bounds.y1
                && f64::from(shape.pos.x) >= bounds.x1 - 30.0
        })
        .expect("the `✖` on his row");
    click_at(
        &mut window,
        egui::pos2(
            mark.pos.x + mark.galley.rect.width() / 2.0,
            mark.pos.y + mark.galley.rect.height() / 2.0,
        ),
        &mut view,
    );
    assert_eq!(
        view.selected,
        Some(Selection::Player(734, 0)),
        "the click on the mark selected his row"
    );
}

/// A name too long for the list truncates before the row's mark (the
/// Live AATF bullet's: the text never runs under it).
#[test]
fn a_long_name_truncates_before_the_rows_mark() {
    let path = fixture(PesVersion::Pes21, "live_marks_long_name");
    let mut window = Headless::new();
    // The narrowest character makes a four-point room error cross a
    // character boundary: the name is its run, as long as the field
    // holds so it still elides.
    let (narrow, char_w) = narrowest_char_width(&mut window);
    let char_w = f64::from(char_w);
    assert!(
        char_w + 0.5 < f64::from(player_list::DOT_MARK_GAP),
        "premise: {narrow:?} ({char_w} at the body font) is narrower than DOT_MARK_GAP"
    );
    let name = narrow.to_string().repeat(61);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = name.clone();
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    view.live_aatf = true;
    // The held result injected for team 701, clean: every row passes
    // and carries the `✔` the name truncates before.
    let revision = view.session.as_ref().expect("the save opened").revision();
    view.live.check.result = Some((
        aatf_check::CheckKey {
            revision,
            ids: vec![701],
            ruleset: "autumn26".to_owned(),
            suggestions: false,
        },
        vec![Ok(Vec::new())],
    ));
    let item_spacing = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .item_spacing
            .x,
    );
    let needle = narrow.to_string().repeat(4);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = nodes
        .iter()
        .find(|node| player_row(node) && node.label().is_some_and(|label| label.contains(&needle)))
        .expect("his row");
    let bounds = row.bounds().expect("the row's bounds");
    let marks = marks_in_row(&shapes, bounds);
    let [mark] = *marks.as_slice() else {
        panic!("expected one mark on his row, got {}", marks.len())
    };
    let name_shape = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains(&needle)
                    && f64::from(text.pos.y) >= bounds.y0
                    && f64::from(text.pos.y) <= bounds.y1 =>
            {
                Some(text)
            }
            _ => None,
        })
        .next()
        .expect("the row's name galley");
    assert!(name_shape.galley.elided, "the name is elided");
    // The text ends one item spacing before the mark, up to one
    // character short (egui truncates at a character boundary).
    let name_right = f64::from(name_shape.pos.x + name_shape.galley.rect.width());
    let gap = f64::from(mark.pos.x) - name_right;
    assert!(
        gap >= item_spacing - 0.5 && gap <= item_spacing + char_w + 0.5,
        "the name ends one gap before the mark ({gap} vs {item_spacing}, one char {char_w})"
    );
}

/// A held check carrying an Error on the selected player for every field
/// General info and Other stats mark: no cross's painted rect intersects
/// another text's, every label, box and combo keeps its x with the marks
/// drawn, and General info's first column of labels has its crosses on
/// one centre x whatever the labels' lengths.
#[test]
fn no_cross_overlaps_text_and_no_field_moves() {
    use aatf::ViolationKind as K;
    let (mut view, mut window) = open_734("live_marks_no_overlap");
    view.live_aatf = true;
    let error = |kind| violation(Some(0), aatf::Severity::Error, kind);
    let violations = vec![
        error(K::Height {
            height: 183,
            fits: vec![],
            room: false,
        }),
        error(K::Age {
            age: 30,
            min: 15,
            max: 50,
        }),
        error(K::Weight {
            weight: 80,
            min: 30,
            max: 100,
        }),
        error(K::Form {
            value: 2,
            target: 4,
        }),
        error(K::InjuryResistance { value: 2, max: 1 }),
        error(K::WeakFootUsage { value: 3, max: 2 }),
        error(K::WeakFootAccuracy { value: 3, max: 2 }),
        error(K::NameColor {
            color: Some("cc9900ff".to_owned()),
            expected: None,
        }),
        error(K::RegisteredNotA { rated_a: vec![] }),
        error(K::PlayingStyleRange),
    ];
    let revision = view.session.as_ref().expect("the save opened").revision();
    view.live.check.result = Some((
        aatf_check::CheckKey {
            revision,
            ids: vec![734],
            ruleset: "autumn26".to_owned(),
            suggestions: false,
        },
        vec![Ok(violations)],
    ));
    let (_, nodes_mark, shapes_mark) =
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let crosses: Vec<_> = mark_shapes(&shapes_mark, "\u{2716}")
        .into_iter()
        .filter(|shape| f64::from(shape.pos.x) > 250.0)
        .collect();
    assert_eq!(crosses.len(), 10, "one cross per marked field");

    // General info's first column: each cross 2 points before its label,
    // all on one centre x.
    let column: Vec<f32> = [
        "Name (",
        "Height",
        "Weight",
        "Age",
        "Registered position",
        "Playing style",
    ]
    .into_iter()
    .map(|label| {
        let bounds = label_starting(&nodes_mark, label);
        let cross = mark_on_row(&crosses, bounds, label);
        assert_left_of_label(cross, bounds, label);
        mark_centre(cross).x
    })
    .collect();
    assert!(
        column.iter().all(|x| (x - column[0]).abs() <= 0.01),
        "General info's crosses share one centre x: {column:?}"
    );

    // No cross touches another text's galley; the narrowest clearance
    // between a cross and any text sharing its row is reported. The
    // view's texts start at x 256 (the player list's end is 248).
    let texts: Vec<&egui::epaint::TextShape> = shapes_mark
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text != "\u{2716}" && f64::from(text.pos.x) > 250.0 =>
            {
                Some(text)
            }
            _ => None,
        })
        .collect();
    let mut clearance = f32::MAX;
    for cross in &crosses {
        let cross_rect = egui::Rect::from_min_size(cross.pos, cross.galley.rect.size());
        for text in &texts {
            let text_rect = egui::Rect::from_min_size(text.pos, text.galley.rect.size());
            assert!(
                !cross_rect.intersects(text_rect),
                "the cross at {:?} overlaps {:?} at {text_rect:?}",
                cross.pos,
                text.galley.job.text
            );
            if cross_rect.min.y < text_rect.max.y && text_rect.min.y < cross_rect.max.y {
                clearance = clearance.min(
                    (text_rect.min.x - cross_rect.max.x).max(cross_rect.min.x - text_rect.max.x),
                );
            }
        }
    }
    assert!(
        clearance >= live_marks::MARK_GAP,
        "a cross {clearance} points from a text, under MARK_GAP"
    );

    // Every marked field's box or combo keeps its x with Live AATF off.
    view.live_aatf = false;
    let (_, nodes_off, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in [
        "Height",
        "Shirt number",
        "Weight",
        "Nation",
        "Age",
        "Captain",
        "Injury resistance",
        "Form",
        "Weak foot: Usage",
        "Accuracy",
        "Stronger foot",
        "Stronger hand",
        "Registered position",
        "Playing style",
    ] {
        let marked = control_right_of(&nodes_mark, label)
            .bounds()
            .expect("the control's bounds");
        let off = control_right_of(&nodes_off, label)
            .bounds()
            .expect("the control's bounds");
        assert_eq!(marked.x0, off.x0, "{label}'s control kept its x");
        assert_eq!(
            label_bounds(&nodes_mark, label),
            label_bounds(&nodes_off, label),
            "{label}'s label kept its place"
        );
    }
    for field in [name_field, shirt_field] {
        let marked = field(&nodes_mark).bounds().expect("the field's bounds");
        let off = field(&nodes_off).bounds().expect("the field's bounds");
        assert_eq!(marked.x0, off.x0, "the counted field kept its x");
    }
}

/// A check that fails marks nothing: the dock reports the failure and no
/// `✖` or `✔` is drawn (the Live AATF bullet's last sub-bullet).
#[test]
fn a_failed_check_marks_nothing() {
    let path = fixture(PesVersion::Pes17, "live_marks_failed");
    let mut view = SaveView::new();
    view.open(&path);
    pick_imported(&mut view, "throw19.4ccrules", &throw19());
    view.team = TeamChoice::Team(743);
    view.selected = Some(Selection::Player(743, 0));
    view.live_aatf = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let nodes = live_checked(&mut window, &mut view, &[743]);
    assert!(
        nodes.iter().any(|node| {
            node.value()
                .is_some_and(|v| v.contains("The AATF check failed"))
        }),
        "premise: the dock reports the failed check"
    );
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(mark_shapes(&shapes, "\u{2716}").is_empty(), "no cross");
    assert!(mark_shapes(&shapes, "\u{2714}").is_empty(), "no check mark");
}

/// A field's mark: one for every row about it, for the highest severity
/// among them — a `✖` in the error colour for an Error, a `⚠` in the
/// warning orange for a Warning, a `ⓘ` in the link blue for suggestions
/// alone — its tooltip every row's line in the check's order. It is drawn
/// in an 11-point box whose right edge stands 2 points left of the
/// label's text, the glyph centred in the box and on the label's height.
#[test]
fn a_fields_mark_draws_its_highest_severitys_glyph_left_of_the_label() {
    use aatf::Severity::{Error, Suggestion, Warning};
    use aatf::ViolationKind as K;
    let label = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(60.0, 16.0));

    /// `Marks::paint` over `violations` beside `label` on one frame (the
    /// pointer left where `events` put it): what `paint` returned and the
    /// shapes.
    fn paint_in(
        window: &mut Headless,
        violations: &[aatf::Violation],
        label: egui::Rect,
        events: Vec<egui::Event>,
    ) -> (Option<egui::Response>, Vec<egui::epaint::ClippedShape>) {
        let marks = live_marks::Marks {
            violations,
            slot: 0,
            team_id: 734,
        };
        let mut painted = None;
        let (_, _, shapes) = window.frame(events, |ui| {
            painted = marks.paint(ui, label, live_marks::MarkedField::RegisteredPosition);
        });
        (painted, shapes)
    }

    /// The tooltip after `frames` idle frames with the pointer at `at`
    /// (bounded, so a missing one fails rather than hangs).
    fn tooltip_of(
        window: &mut Headless,
        violations: &[aatf::Violation],
        label: egui::Rect,
        at: egui::Pos2,
        frames: usize,
    ) -> Option<String> {
        paint_in(
            window,
            violations,
            label,
            vec![egui::Event::PointerMoved(at)],
        );
        for _ in 0..frames {
            let (_, nodes, _) = window.frame(Vec::new(), |ui| {
                live_marks::Marks {
                    violations,
                    slot: 0,
                    team_id: 734,
                }
                .paint(ui, label, live_marks::MarkedField::RegisteredPosition);
            });
            if let Some(value) = nodes.iter().find_map(|node| node.value())
                && (value.contains("RegisteredNotA") || value.contains("PositionRange"))
            {
                return Some(value.to_owned());
            }
        }
        None
    }

    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let line =
        |violation: &aatf::Violation| team_widgets::violations::violation_line(violation, 734);
    let cases = [
        (
            vec![
                violation(Some(0), Warning, K::PositionRange),
                violation(Some(0), Error, K::RegisteredNotA { rated_a: vec![] }),
            ],
            "\u{2716}",
            visuals.error_fg_color,
        ),
        (
            vec![
                violation(Some(0), Warning, K::RegisteredNotA { rated_a: vec![] }),
                violation(Some(0), Warning, K::PositionRange),
            ],
            "\u{26a0}",
            visuals.warn_fg_color,
        ),
        (
            vec![violation(Some(0), Suggestion, K::PositionRange)],
            "\u{24d8}",
            visuals.hyperlink_color,
        ),
    ];
    // The box's centre: 2 points left of the label, then half of 11.
    let box_centre = egui::pos2(label.min.x - 2.0 - 5.5, label.center().y);
    for (violations, glyph, color) in &cases {
        let (painted, shapes) = paint_in(&mut window, violations, label, vec![]);
        assert!(painted.is_some(), "{glyph}: a mark is painted");
        let marks: Vec<_> = ["\u{2716}", "\u{26a0}", "\u{24d8}"]
            .into_iter()
            .flat_map(|text| mark_shapes(&shapes, text))
            .collect();
        let [mark] = *marks.as_slice() else {
            panic!("{glyph}: expected one mark, got {}", marks.len())
        };
        assert_eq!(mark.galley.job.text, *glyph, "the highest severity's glyph");
        assert_eq!(galley_color(mark), *color, "{glyph}: the severity's colour");
        let centre = mark.pos + mark.galley.rect.size() / 2.0;
        assert!(
            (centre.x - box_centre.x).abs() <= 0.01,
            "{glyph}: centred in the 11-point box 2 points left of the label ({} vs {})",
            centre.x,
            box_centre.x
        );
        assert!(
            (centre.y - box_centre.y).abs() <= 0.5,
            "{glyph}: centred on the label's height ({} vs {})",
            centre.y,
            box_centre.y
        );
        let expected = violations.iter().map(line).collect::<Vec<_>>().join("\n");
        assert_eq!(
            tooltip_of(&mut window, violations, label, box_centre, 30).as_deref(),
            Some(expected.as_str()),
            "{glyph}: every row's line in the check's order"
        );
    }

    // Another slot's rows and another field's mark nothing here.
    let others = [
        violation(Some(1), Error, K::RegisteredNotA { rated_a: vec![] }),
        violation(Some(0), Error, K::PlayingStyleRange),
    ];
    let (painted, _) = paint_in(&mut window, &others, label, vec![]);
    assert!(painted.is_none(), "no row of this slot and field");
}

/// A dirty row keeps its `●` through truncation (the dot holds the
/// row's right end before the mark, so a name that overflows loses
/// neither it nor the mark).
#[test]
fn a_dirty_rows_dot_survives_truncation() {
    let path = fixture(PesVersion::Pes21, "live_marks_dirty_dot");
    let mut window = Headless::new();
    // The narrowest character makes a four-point room error cross a
    // character boundary: the names are its run, as long as the field
    // holds so they still elide.
    let (narrow, char_w) = narrowest_char_width(&mut window);
    let char_w = f64::from(char_w);
    assert!(
        char_w + 0.5 < f64::from(player_list::DOT_MARK_GAP),
        "premise: {narrow:?} ({char_w} at the body font) is narrower than DOT_MARK_GAP"
    );
    let needle = narrow.to_string().repeat(4);
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().name = narrow.to_string().repeat(61);
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    view.live_aatf = true;
    // An edit makes him dirty; the held result injected so every row
    // carries its mark.
    let mut entry = player_entry(&view, 70101);
    entry.basic.height = u8::MAX;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_players(vec![entry]);
    assert!(
        view.session
            .as_ref()
            .expect("the save opened")
            .player_dirty(701, 0),
        "premise: the player is dirty"
    );
    let revision = view.session.as_ref().expect("the save opened").revision();
    view.live.check.result = Some((
        aatf_check::CheckKey {
            revision,
            ids: vec![701],
            ruleset: "autumn26".to_owned(),
            suggestions: false,
        },
        vec![Ok(Vec::new())],
    ));
    let (item_spacing, warn) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            f64::from(style.spacing.item_spacing.x),
            style.visuals.warn_fg_color,
        )
    };
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = nodes
        .iter()
        .find(|node| player_row(node) && node.label().is_some_and(|label| label.contains(&needle)))
        .expect("his row");
    let bounds = row.bounds().expect("the row's bounds");
    let name_shape = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.contains(&needle)
                    && f64::from(text.pos.y) >= bounds.y0
                    && f64::from(text.pos.y) <= bounds.y1 =>
            {
                Some(text)
            }
            _ => None,
        })
        .next()
        .expect("the row's name galley");
    assert!(name_shape.galley.elided, "the name is elided");
    let marks = marks_in_row(&shapes, bounds);
    let [mark] = *marks.as_slice() else {
        panic!("expected one mark on his row, got {}", marks.len())
    };
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (f64::from(mark.pos.x) - dot_right - item_spacing - f64::from(player_list::DOT_MARK_GAP))
            .abs()
            <= 0.5,
        "the dot stands one gap plus DOT_MARK_GAP before the mark ({dot_right} vs {})",
        mark.pos.x
    );
    let text_right = f64::from(name_shape.pos.x + name_shape.galley.rect.width());
    assert!(
        f64::from(dot.pos.x) >= text_right,
        "the dot sits right of the text"
    );
    // The text ends one item spacing before the dot, up to one
    // character short (egui truncates at a character boundary).
    let gap = f64::from(dot.pos.x) - text_right;
    assert!(
        gap >= item_spacing - 0.5 && gap <= item_spacing + char_w + 0.5,
        "the text ends one gap before the dot ({gap} vs {item_spacing}, one char {char_w})"
    );
    assert!(mark_shapes(&shapes, "\u{25cf}").len() == 1, "one dot");

    // A team row whose name truncates: its dirty dot too, if the
    // fixture lets it.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.team_mut(701).unwrap().name = "T".repeat(52);
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    view.live_aatf = true;
    let mut team = team_entry(&view, 701);
    team.name = narrow.to_string().repeat(70);
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_entries(vec![], vec![team]);
    assert!(
        view.session
            .as_ref()
            .expect("the save opened")
            .team_dirty(701),
        "premise: the team is dirty"
    );
    let revision = view.session.as_ref().expect("the save opened").revision();
    view.live.check.result = Some((
        aatf_check::CheckKey {
            revision,
            ids: vec![701],
            ruleset: "autumn26".to_owned(),
            suggestions: false,
        },
        vec![Ok(Vec::new())],
    ));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let row = team_row(&nodes, &view, 701);
    let bounds = row.bounds().expect("the team row's bounds");
    let name_shape = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text)
                if text.galley.job.text.starts_with(&needle)
                    && f64::from(text.pos.y) >= bounds.y0
                    && f64::from(text.pos.y) <= bounds.y1 =>
            {
                Some(text)
            }
            _ => None,
        })
        .next()
        .expect("the team row's name galley");
    assert!(name_shape.galley.elided, "the name is elided");
    let marks = marks_in_row(&shapes, bounds);
    let [mark] = *marks.as_slice() else {
        panic!("expected one mark on the team row, got {}", marks.len())
    };
    let dot = dot_in_row(&shapes, bounds);
    assert_eq!(galley_color(dot), warn, "the dot in the warning colour");
    let dot_right = f64::from(dot.pos.x + dot.galley.rect.width());
    assert!(
        (f64::from(mark.pos.x) - dot_right - item_spacing - f64::from(player_list::DOT_MARK_GAP))
            .abs()
            <= 0.5,
        "the dot stands one gap plus DOT_MARK_GAP before the mark ({dot_right} vs {})",
        mark.pos.x
    );
    let text_right = f64::from(name_shape.pos.x + name_shape.galley.rect.width());
    assert!(
        f64::from(dot.pos.x) >= text_right,
        "the dot sits right of the text"
    );
    let gap = f64::from(dot.pos.x) - text_right;
    assert!(
        gap >= item_spacing - 0.5 && gap <= item_spacing + char_w + 0.5,
        "the text ends one gap before the dot ({gap} vs {item_spacing}, one char {char_w})"
    );
}

/// Only the checked team's rows are marked: with ALL picked another
/// team's rows draw neither `✖` nor `✔`, and the checked team's keep
/// theirs (the Live AATF bullet: players of other teams carry none).
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

/// A `FieldedNotA` violation's cross is an X over the starter's portrait
/// on the team view's Tactics pitch (the pitch is the field the
/// violation is about); with Live AATF off the marker carries none.
#[test]
fn a_fielded_violation_crosses_the_starters_portrait_on_the_pitch() {
    let (mut view, mut window) = open_734("live_marks_fielded");
    view.live_aatf = true;
    // Move a starter of preset 1's kick-off formation onto a position
    // his ratings do not hold at A, picked from his record (as `aatf`'s
    // own `FieldedNotA` fixture moves one to AMF).
    let index = 4;
    let mut team = team_entry(&view, 734);
    let slot = team.tactics.starting_eleven[index];
    let player_id = team.roster[usize::from(slot)].player_id;
    let position_byte = unrated_position_byte(&view, player_id);
    let kick_off = &mut team.tactics.presets[0].formations[0];
    assert_ne!(
        kick_off.players[index].position, position_byte,
        "premise: the move changes his spot"
    );
    kick_off.players[index].position = position_byte;
    view.session
        .as_mut()
        .expect("the save opened")
        .edit_team(1, team);

    // The check lands: a `FieldedNotA` on him in preset 1.
    live_checked(&mut window, &mut view, &[734]);
    let violations = view.live.check.result.as_ref().unwrap().1[0]
        .as_ref()
        .unwrap();
    let (fielded_slot, position) = violations
        .iter()
        .find_map(|violation| {
            if let aatf::ViolationKind::FieldedNotA {
                preset: 1,
                position,
                ..
            } = violation.kind
            {
                Some((violation.slot, position))
            } else {
                None
            }
        })
        .expect("premise: a FieldedNotA on preset 1");
    assert_eq!(fielded_slot, Some(slot), "the moved starter's slot");
    assert_eq!(position.byte(), position_byte, "the moved-to position");

    // The team row selected (the pitch is the team view's): his
    // marker's portrait carries an X, corner to corner, its coloured
    // strokes in the error colour over black ones.
    view.selected = Some(Selection::Team(734));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let canvas = canvas_of(&nodes);
    let shown_slot = team_entry(&view, 734).tactics.presets[0].formations[0].players[index];
    let badge =
        egui::Rect::from_center_size(clamped_centre(canvas, shown_slot), egui::vec2(50.0, 14.0));
    let portrait = egui::Rect::from_min_max(
        egui::pos2(badge.left(), badge.top() - 50.0),
        egui::pos2(badge.right(), badge.top()),
    );
    let strokes = pitch_cross_strokes(&shapes);
    let [black, _, colour, _] = strokes.as_slice() else {
        panic!("expected one cross's four strokes, got {}", strokes.len())
    };
    assert_eq!(black.color, egui::Color32::BLACK, "the outline is black");
    assert_eq!(
        colour.color,
        window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .error_fg_color,
        "the error colour"
    );
    assert!(
        strokes
            .iter()
            .all(|stroke| portrait.contains(stroke.points[0])
                && portrait.contains(stroke.points[1])
                && (stroke.points[0] - stroke.points[1]).x.abs() > 30.0),
        "the strokes cross his portrait {portrait:?}"
    );

    // Live AATF off: the pitch carries no cross.
    view.live_aatf = false;
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        pitch_cross_strokes(&shapes).is_empty(),
        "no cross on the pitch with Live AATF off"
    );
}

/// A painted line segment's ends, width and colour.
struct Stroke {
    points: [egui::Pos2; 2],
    color: egui::Color32,
}

/// The frame's line segments 5 or more wide: the pitch crosses' strokes
/// (the pitch's lines and corners are 2 wide), black outlines first.
fn pitch_cross_strokes(shapes: &[egui::epaint::ClippedShape]) -> Vec<Stroke> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke } if stroke.width >= 5.0 => Some(Stroke {
                points: *points,
                color: stroke.color,
            }),
            _ => None,
        })
        .collect()
}
