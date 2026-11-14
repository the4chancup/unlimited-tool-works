use super::*;

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
            violations.violations.iter().any(|violation| matches!(
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
            "Injury Resistance",
        ),
        (
            Error,
            K::WeakFootUsage { value: 3, max: 2 },
            "Weak Foot: Usage",
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
        (Suggestion, unused("weak_foot_usage"), "Weak Foot: Usage"),
        (Suggestion, unused("weak_foot_accuracy"), "Accuracy"),
    ];
    let (mut view, mut window) = open_734("live_marks_each_field");
    view.live_aatf = true;
    // Show suggestions on, so the suggestion cases' marks are drawn.
    view.live_suggestions = true;
    let revision = view.session.as_ref().expect("the save opened").revision();
    for (severity, kind, label) in &cases {
        live_result_held(
            &mut window,
            &mut view,
            (
                aatf_check::CheckKey {
                    revision,
                    ids: vec![734],
                    ruleset: "autumn26".to_owned(),
                    suggestions: true,
                },
                vec![checked(vec![violation(Some(0), *severity, kind.clone())])],
            ),
        );
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

/// A warning-only violation's mark is an amber warning sign: the
/// official file gains a warning band on the regular tier, and 73401's
/// attacking prowess goes one over its 77 target (a `Stat` warning, not
/// an error).
#[test]
fn a_warnings_mark_is_an_amber_warning_sign() {
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
            checks[0].as_ref().is_ok_and(|check| {
                check.violations.iter().any(|violation| {
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
/// with none shows `✔`.
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
/// alone, so the click lands on the row.
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

/// A name too long for the list truncates before the row's mark: the
/// text never runs under it.
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
    let item_spacing = f64::from(
        window
            .ctx
            .style_of(window.ctx.theme())
            .spacing
            .item_spacing
            .x,
    );
    let needle = narrow.to_string().repeat(4);
    let (nodes, shapes) = live_result_held(
        &mut window,
        &mut view,
        (
            aatf_check::CheckKey {
                revision,
                ids: vec![701],
                ruleset: "autumn26".to_owned(),
                suggestions: false,
            },
            vec![checked(Vec::new())],
        ),
    );
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
    let (nodes_mark, shapes_mark) = live_result_held(
        &mut window,
        &mut view,
        (
            aatf_check::CheckKey {
                revision,
                ids: vec![734],
                ruleset: "autumn26".to_owned(),
                suggestions: false,
            },
            vec![checked(violations)],
        ),
    );
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
        "Injury Resistance",
        "Form",
        "Weak Foot: Usage",
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
/// `✖` or `✔` is drawn.
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
/// warning amber for a Warning, a `ⓘ` in the link blue for suggestions
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
        let rules = aatf::Rules::official();
        let marks = live_marks::Marks {
            violations,
            slot: 0,
            team_id: 734,
            wording: team_widgets::violations::Wording {
                ruleset: rules.ruleset(),
                version: PesVersion::Pes17,
            },
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
        let rules = aatf::Rules::official();
        for _ in 0..frames {
            let (_, nodes, _) = window.frame(Vec::new(), |ui| {
                live_marks::Marks {
                    violations,
                    slot: 0,
                    team_id: 734,
                    wording: team_widgets::violations::Wording {
                        ruleset: rules.ruleset(),
                        version: PesVersion::Pes17,
                    },
                }
                .paint(ui, label, live_marks::MarkedField::RegisteredPosition);
            });
            if let Some(value) = nodes.iter().find_map(|node| node.value())
                && value.contains("Registered position")
            {
                return Some(value.to_owned());
            }
        }
        None
    }

    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let rules = aatf::Rules::official();
    let line = |violation: &aatf::Violation| {
        team_widgets::violations::violation_line(
            violation,
            734,
            team_widgets::violations::Wording {
                ruleset: rules.ruleset(),
                version: PesVersion::Pes17,
            },
        )
    };
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
    let (item_spacing, warn) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            f64::from(style.spacing.item_spacing.x),
            style.visuals.warn_fg_color,
        )
    };
    let (nodes, shapes) = live_result_held(
        &mut window,
        &mut view,
        (
            aatf_check::CheckKey {
                revision,
                ids: vec![701],
                ruleset: "autumn26".to_owned(),
                suggestions: false,
            },
            vec![checked(Vec::new())],
        ),
    );
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
    let (nodes, shapes) = live_result_held(
        &mut window,
        &mut view,
        (
            aatf_check::CheckKey {
                revision,
                ids: vec![701],
                ruleset: "autumn26".to_owned(),
                suggestions: false,
            },
            vec![checked(Vec::new())],
        ),
    );
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
