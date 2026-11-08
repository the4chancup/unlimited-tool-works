use super::*;

/// The Savefile menu's Multi-team AATF… on the PES 17 sample under 4cc
/// Autumn 26: the title names the ruleset; the range's two combos sit on
/// one row, both on the save's first team, 701, which the list holds
/// alone. Widened to the save's last team, the summary counts what
/// `check_teams` finds for the same teams; every team is listed under its
/// id and name with no suggestion. Narrowed to one team, it lists that
/// team alone.
// SE-GUI-41
#[test]
fn multi_team_aatf_checks_and_lists_every_team_of_the_range() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_range");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    assert_eq!(ids[0], 701, "premise: 701 is the save's lowest team id");
    let first = format!("701 {}", display_name(&team_entry(&view, 701).name));
    let pairs = multi_checked(&mut window, &mut view, &[701]);
    let lines = multi_lines(&pairs);
    assert_eq!(lines[0], MULTI_TITLE);
    let combos = range_combos(&pairs);
    assert_eq!(combos.len(), 2, "two team combos");
    let middle = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("drawn bounds");
        (bounds.y0 + bounds.y1) / 2.0
    };
    assert_eq!(
        middle(combos[0]),
        middle(combos[1]),
        "the combos share a row"
    );
    let ends: Vec<Option<&str>> = combos.iter().map(|combo| combo.value()).collect();
    assert_eq!(
        ends,
        [Some(first.as_str()), Some(first.as_str())],
        "both ends on the save's first team"
    );
    let summary = lines
        .iter()
        .find(|line| line.contains("checked,"))
        .unwrap_or_else(|| panic!("a summary in {lines:?}"));
    assert!(summary.starts_with("1 team checked, "), "{lines:?}");
    assert!(lines.contains(&first), "701 is listed: {lines:?}");
    assert!(
        !lines.iter().any(|line| line.starts_with("702 ")),
        "702 is not listed: {lines:?}"
    );

    // Widened to every team.
    let (names, expected_issues) = {
        let file = view.session.as_ref().expect("the save opened").file();
        let names: Vec<String> = ids
            .iter()
            .map(|&id| format!("{id} {}", display_name(&file.team(id).unwrap().name)))
            .collect();
        let choice = crate::session::rules::resolve("autumn26", &[]);
        let checks = crate::session::rules::check_teams(
            &choice.rules,
            &crate::session::rules::team_views(file, &ids),
        );
        let with_issues = checks
            .iter()
            .filter(|check| {
                check
                    .as_ref()
                    .expect("every sample team checks")
                    .violations
                    .iter()
                    .any(|violation| {
                        matches!(
                            violation.severity,
                            aatf::Severity::Error | aatf::Severity::Warning
                        )
                    })
            })
            .count();
        (names, with_issues)
    };
    pick_range_end(&mut window, &mut view, 1, &names[names.len() - 1]);
    let pairs = multi_checked(&mut window, &mut view, &ids);
    let lines = multi_lines(&pairs);
    let summary = format!("{} teams checked, {expected_issues} with issues", ids.len());
    assert!(lines.contains(&summary), "{summary:?} in {lines:?}");
    for name in &names {
        assert!(lines.contains(name), "{name:?} is listed");
    }
    assert!(
        !lines.iter().any(|line| line.starts_with("Unused")),
        "no suggestion is listed"
    );
    assert!(!multi_button(&pairs, "Copy to clipboard").is_disabled());

    // Narrowed to one team.
    pick_range_end(&mut window, &mut view, 0, "734 /lit/");
    pick_range_end(&mut window, &mut view, 1, "734 /lit/");
    let pairs = multi_checked(&mut window, &mut view, &[734]);
    let lines = multi_lines(&pairs);
    assert!(
        lines.contains(&"1 team checked, 1 with issues".to_owned()),
        "{lines:?}"
    );
    let listed: Vec<&String> = names.iter().filter(|name| lines.contains(name)).collect();
    assert_eq!(listed, ["734 /lit/"], "734 alone is listed");
}

/// The line above the list counts the systems the checked teams were read
/// under: Autumn 26 holds two height systems and no medal ones, and 706
/// reads under Red, 707 under Green, so the range 706 to 707 draws
/// `Height systems: Green 1, Red 1` between the summary and the teams.
// SE-GUI-51
#[test]
fn the_counts_line_lists_the_systems_the_range_was_read_under() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_counts");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (first, last) = (label(&view, 706), label(&view, 707));
    pick_range_end(&mut window, &mut view, 0, &first);
    pick_range_end(&mut window, &mut view, 1, &last);
    let pairs = multi_checked(&mut window, &mut view, &[706, 707]);
    let lines = multi_lines(&pairs);
    let counts = "Height systems: Green 1, Red 1".to_owned();
    assert!(
        lines.contains(&counts),
        "the counts line is drawn: {lines:?}"
    );
    let at = lines.iter().position(|line| *line == counts).unwrap();
    let summary = lines
        .iter()
        .position(|line| line.contains("checked,"))
        .expect("the summary line");
    assert!(summary < at, "the counts sit below the summary: {lines:?}");
    // `rposition`: the first end's combo carries the same label, higher.
    let first = lines
        .iter()
        .rposition(|line| line.starts_with("706 "))
        .expect("the list's first team");
    assert!(at < first, "the counts sit above the list: {lines:?}");
}

/// The popup items of the dialog's team combo `nth` from the left, top to
/// bottom: the combo clicked open, its items read, then a click on the
/// title closes the popup.
fn range_items(window: &mut Headless, view: &mut SaveView, nth: usize) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let combo = range_combos(&pairs)[nth];
    let (left, right, below) = {
        let bounds = combo.bounds().expect("drawn bounds");
        (bounds.x0, bounds.x1, bounds.y1)
    };
    click(window, combo, view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    // The popup's items stand under the combo, as `pick_range_end` finds
    // them; its scroll area draws the items it shows.
    let mut items: Vec<(f64, String)> = pairs
        .iter()
        .filter(|(_, node)| node.role() == Role::Button)
        .filter_map(|(_, node)| {
            let (bounds, label) = (node.bounds()?, node.label()?);
            ((left..right).contains(&bounds.x0) && bounds.y0 >= below)
                .then(|| (bounds.y0, label.to_owned()))
        })
        .collect();
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    click(window, title, view);
    items.into_iter().map(|(_, label)| label).collect()
}

/// The last end follows a first end picked past it, on the frame of the
/// pick, and lists the teams from the first end's on: 718 picked on the
/// first end of the PES 17 sample's dialog, opened on 701 to 701.
// SE-GUI-41
#[test]
fn the_last_end_follows_the_first_and_lists_the_teams_from_it() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_clamp");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The opening check lands before any bounds are read: its counts
    // line moves the widgets under the summary a row, so a click aimed
    // before it could land on another.
    multi_checked(&mut window, &mut view, &[701]);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (team_717, team_718) = (label(&view, 717), label(&view, 718));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(label(&view, 701).as_str()),
        "premise: the last end opens on 701"
    );
    assert!(
        range_items(&mut window, &mut view, 1).contains(&team_717),
        "premise: the last end lists 717 while the first is on 701"
    );

    // `pick_range_end` by hand, so the pick's own frame is read.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let combo = range_combos(&pairs)[0];
    let (left, right) = {
        let bounds = combo.bounds().expect("drawn bounds");
        (bounds.x0, bounds.x1)
    };
    click(&mut window, combo, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (item, _) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some(team_718.as_str())
                && node
                    .bounds()
                    .is_some_and(|bounds| (left..right).contains(&bounds.x0))
        })
        .expect("718 in the first end's popup");
    let (_, pairs, _) = window.frame_ids(
        vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target_tree: accesskit::TreeId::ROOT,
                target_node: *item,
                data: None,
            },
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    // The first combo drew its text before its popup took the pick; the
    // last, drawn after, already reads the pick.
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(team_718.as_str()),
        "the last end follows the first on the pick's frame"
    );
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    click(&mut window, title, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ends: Vec<Option<&str>> = range_combos(&pairs)
        .into_iter()
        .map(accesskit::Node::value)
        .collect();
    assert_eq!(
        ends,
        [Some(team_718.as_str()), Some(team_718.as_str())],
        "both ends on 718"
    );

    let items = range_items(&mut window, &mut view, 1);
    assert_eq!(
        items.first(),
        Some(&team_718),
        "the last end's items start at 718: {items:?}"
    );
    assert!(!items.contains(&team_717), "717 is not offered: {items:?}");
}

/// A press on the dialog's title held while the opening check lands: the
/// result draws the counts line, and unless its auto id is kept when it
/// is absent, every widget after it takes the next id on the landing
/// frame, so the release clicks whichever widget took the pressed id
/// (Close). Released after the landing, the press leaves the dialog open.
#[test]
fn a_press_held_while_the_check_lands_leaves_the_dialog_open() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_held_click");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The opening check's reply is kept off the frames: the real run's
    // channel is taken and replaced with the test's, its reply kept
    // until the press is down. The click's release frame started the run,
    // and no frame has polled it since: a frame here could already land a
    // fast reply.
    let real = view
        .multi_aatf
        .check
        .pending
        .take()
        .expect("the opening check is in flight");
    let (sender, replies) = crossbeam_channel::unbounded();
    view.multi_aatf.check.pending = Some(worker::Pending {
        key: real.key.clone(),
        replies,
    });
    let reply = real
        .replies
        .recv_timeout(Duration::from_secs(120))
        .expect("the opening check never replied");

    // A frame while the press is still up: no result is listed.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf.check.result.is_none(),
        "premise: no result is listed yet"
    );
    let title = super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
        .into_iter()
        .find(|node| node.value() == Some(MULTI_TITLE))
        .expect("the title");
    let at = centre(title);

    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    sender
        .send(reply)
        .expect("the held channel takes the reply");
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf
            .check
            .result
            .as_ref()
            .is_some_and(|(key, _)| key.ids == [701]),
        "premise: the reply landed while the press was held"
    );
    // The frame after the release's: a Close clicked on the release
    // frame closes the dialog on this one.
    let (_, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        pairs
            .iter()
            .any(|(_, node)| node.value() == Some(MULTI_TITLE)),
        "the dialog is still open"
    );
}

/// Thirty wheel notches over the first end, one a frame at 60 frames a
/// second, step it from 701 to 731, the last end following, and start no
/// check while they turn: the list keeps 701's result, dimmed to 0.55 and
/// marked checking. Once the input time passes 500 ms after the last
/// notch, one check starts, for 731 alone, and the list is drawn at full
/// opacity once it lands.
// SE-GUI-41
#[test]
fn a_range_turned_by_the_wheel_is_checked_once_it_has_rested() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_rest");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    assert_eq!(
        ids[..=30],
        (701..=731).collect::<Vec<u32>>()[..],
        "premise: the save's first 31 teams are 701 to 731"
    );
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let first = label(&view, 701);
    multi_checked(&mut window, &mut view, &[701]);
    // Every other run of the view lands before the turn, so no reply lands
    // between two notches: under load the second notch once went
    // unstepped, and a reply landing mid-turn is the one input whose
    // timing the test does not set.
    workers_replied(&mut window, &mut view);
    // The colour the text `text` is painted in last, as the tessellator
    // picks it: the override, else the glyphs' own, else the fallback for
    // glyphs left to it. egui dims a painter's shapes by scaling these
    // colours. Painted last: the view behind the modal and the first
    // combo paint the same text before the list does.
    let ink = |shapes: &[egui::epaint::ClippedShape], text: &str| {
        shapes
            .iter()
            .rev()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
                _ => None,
            })
            .map(|shape| {
                let glyph = shape
                    .galley
                    .rows
                    .iter()
                    .flat_map(|row| &row.row.visuals.mesh.vertices)
                    .map(|vertex| vertex.color)
                    .find(|&color| color != egui::Color32::PLACEHOLDER);
                shape
                    .override_text_color
                    .or(glyph)
                    .unwrap_or(shape.fallback_color)
            })
    };
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let at = centre(range_combos(&pairs)[0]);
    let full = ink(&shapes, &first).expect("701's line is painted");
    let time = |window: &Headless| window.ctx.input(|input| input.time);
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let mut before = time(&window);
    for step in 1..=30u32 {
        let (_, pairs, shapes) = window.frame_ids(
            vec![egui::Event::PointerMoved(at), notch_down.clone()],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        let now = time(&window);
        assert!(
            (now - before - 1.0 / 60.0).abs() < 1e-9,
            "premise: a frame is 1/60 s after the last, {before} then {now}"
        );
        before = now;
        // The first combo draws its text before the notch steps it; the
        // last, drawn after, reads the step.
        assert_eq!(
            range_combos(&pairs)[1].value(),
            Some(label(&view, 701 + step).as_str()),
            "notch {step} steps the first end, the last following"
        );
        let check = &view.multi_aatf.check;
        assert!(check.pending.is_none(), "notch {step} started a check");
        assert!(
            check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [701]),
            "701's result is kept at notch {step}"
        );
        if step > 1 {
            let lines = multi_lines(&pairs);
            assert!(lines.contains(&first), "701's list is kept: {lines:?}");
            assert!(
                lines.contains(&"checking\u{2026}".to_owned()),
                "marked checking at notch {step}: {lines:?}"
            );
            assert_eq!(
                ink(&shapes, &first),
                Some(full.gamma_multiply(0.55)),
                "701's line dimmed to 0.55 at notch {step}"
            );
        }
    }
    let last_notch = before;

    // No more input: frames until the check lands, each counting the checks
    // started on it.
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut started = Vec::new();
    let mut shapes;
    loop {
        let had_pending = view.multi_aatf.check.pending.is_some();
        (_, _, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let check = &view.multi_aatf.check;
        if let Some(pending) = &check.pending
            && !had_pending
        {
            started.push((time(&window) - last_notch, pending.key.ids.clone()));
        }
        if check.pending.is_none()
            && check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [731])
        {
            break;
        }
        assert!(Instant::now() < deadline, "731's check never returned");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(started.len(), 1, "one check started: {started:?}");
    let (after, checked) = &started[0];
    assert_eq!(checked, &[731], "the check is for the final range");
    assert!(
        *after >= 0.5 && *after < 0.5 + 3.0 / 60.0,
        "the check starts once 500 ms have passed since the last notch, not {after} s"
    );
    assert_eq!(
        ink(&shapes, &label(&view, 731)),
        Some(full),
        "731's line at full opacity, as 701's was"
    );
}

/// Multi-team AATF's range rests and is checked with no input to wake the
/// window: each frame of the rest asks egui for another within 500 ms, and
/// the frame the rest ends on asks for the next at once, which starts the
/// check. Asked for none, the list stays dimmed until the pointer moves.
// SE-GUI-41
#[test]
fn a_resting_range_asks_for_the_frames_its_check_needs() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_repaint");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    assert_eq!(
        all_team_ids(&view)[..2],
        [701, 702],
        "premise: the save's first two teams are 701 and 702"
    );
    multi_checked(&mut window, &mut view, &[701]);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let notch_down = egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, -1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: egui::Modifiers::default(),
    };
    let events = vec![
        egui::Event::PointerMoved(centre(range_combos(&pairs)[0])),
        notch_down,
    ];
    let (_, pairs, _) = window.frame_ids(events, |ui| show(ui, &mut view, &|| Vec::new()));
    let label = format!("702 {}", display_name(&team_entry(&view, 702).name));
    assert_eq!(
        range_combos(&pairs)[1].value(),
        Some(label.as_str()),
        "premise: the notch picked 702"
    );

    // No more input: each frame's request, until the frame that starts
    // 702's check (30 frames of 1/60 s make the rest).
    let started = |view: &SaveView| {
        let check = &view.multi_aatf.check;
        check.pending.is_some()
            || check
                .result
                .as_ref()
                .is_some_and(|(key, _)| key.ids == [702])
    };
    assert!(!started(&view), "premise: the notch started no check");
    let mut delays = Vec::new();
    while !started(&view) {
        assert!(delays.len() < 90, "702's check never started");
        window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        delays.push(window.repaint_delay);
    }
    let [resting @ .., promoted, _started] = delays.as_slice() else {
        panic!("the check started on the first frame after the notch: {delays:?}");
    };
    assert!(!resting.is_empty(), "premise: the range rested a frame");
    for (frame, delay) in resting.iter().enumerate() {
        assert!(
            *delay <= Duration::from_millis(500),
            "resting frame {frame} asks for the next within the rest, not {delay:?}"
        );
    }
    assert_eq!(
        *promoted,
        Duration::ZERO,
        "the frame the rest ends on asks for the next at once"
    );
}
