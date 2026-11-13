use super::*;

/// The smallest painted rectangle of `fill` holding every one of `points`: a
/// frame as drawn, border included. A frame with a shadow paints as a
/// `Shape::Vec` of the shadow and the rectangle, so nested shapes count too.
fn painted_frame(
    shapes: &[egui::epaint::ClippedShape],
    fill: egui::Color32,
    points: &[egui::Pos2],
) -> egui::Rect {
    let mut rects = Vec::new();
    let mut stack: Vec<&egui::Shape> = shapes.iter().map(|clipped| &clipped.shape).collect();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Rect(rect) => rects.push(rect),
            egui::Shape::Vec(inner) => stack.extend(inner),
            _ => {}
        }
    }
    rects
        .iter()
        .filter(|rect| rect.fill == fill && points.iter().all(|&p| rect.rect.contains(p)))
        .map(|rect| rect.rect)
        .min_by(|a, b| a.area().total_cmp(&b.area()))
        .unwrap_or_else(|| panic!("no frame of {fill:?} around {points:?}"))
}

/// Multi-team AATF's dialog is 900 points wide, its list 874 of them (the
/// modal's padding and border around it), so a height row's sentence and
/// its players' lines have room.
#[test]
fn multi_team_aatf_is_900_points_wide() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_width");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // The modal's frame and the list's, as painted: the smallest rectangle of
    // the window's fill holding the title and the range's first combo (a
    // panel of the view behind the modal shares the fill), then the smallest
    // of the extreme background under them.
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let title = centre(
        super::prompt::dialog_nodes(&pairs, MULTI_TITLE)
            .into_iter()
            .find(|node| node.value() == Some(MULTI_TITLE))
            .expect("the title"),
    );
    let first_combo = centre(range_combos(&pairs)[0]);
    let painted = |fill, points: &[egui::Pos2]| painted_frame(&shapes, fill, points);
    let dialog = painted(visuals.window_fill(), &[title, first_combo]);
    assert!(
        (dialog.width() - 900.0).abs() <= 0.5,
        "the dialog's width: {}",
        dialog.width()
    );
    let list = painted(
        visuals.extreme_bg_color,
        &[egui::pos2(dialog.center().x, first_combo.y + 80.0)],
    );
    assert!(
        (list.width() - 874.0).abs() <= 0.5,
        "the list's outer width: {}",
        list.width()
    );
}

/// Multi-team AATF's list is 360 points tall, border included, when its
/// rows overflow it (the PES 17 sample's every team): it scrolls rather
/// than growing or shrinking.
#[test]
fn multi_team_aatfs_overflowing_list_is_360_points_tall() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_height");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    let ids = all_team_ids(&view);
    let (first, last) = {
        let file = view.session.as_ref().expect("the save opened").file();
        let name = |id: u32| format!("{id} {}", display_name(&file.team(id).unwrap().name));
        (name(ids[0]), name(ids[ids.len() - 1]))
    };
    multi_checked(&mut window, &mut view, &ids[..1]);
    pick_range_end(&mut window, &mut view, 1, &last);
    multi_checked(&mut window, &mut view, &ids);
    let (_, pairs, shapes) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let dialog = super::prompt::dialog_nodes(&pairs, MULTI_TITLE);
    let first_team = centre(
        dialog
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(first.as_str()))
            .expect("the first team's line"),
    );
    let scroll_bar = centre(
        dialog
            .iter()
            .find(|node| node.role() == Role::ScrollBar)
            .expect("premise: the rows overflow the list, so it draws a scroll bar"),
    );
    // The list's frame as painted: the extreme background holding both the
    // first team's line and the scroll bar (the player list's well behind
    // the modal holds the line alone).
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let list = painted_frame(&shapes, visuals.extreme_bg_color, &[first_team, scroll_bar]);
    assert!(
        (list.height() - 360.0).abs() <= 1.0,
        "the list's outer height: {}",
        list.height()
    );
}

/// Multi-team AATF's range is kept while the save stays open: narrowed to
/// 734 alone, the dialog closed and opened again reads 734 at both ends.
#[test]
fn a_reopened_multi_team_aatf_keeps_its_range() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_reopen");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    pick_range_end(&mut window, &mut view, 0, "734 /lit/");
    pick_range_end(&mut window, &mut view, 1, "734 /lit/");
    let pairs = multi_checked(&mut window, &mut view, &[734]);

    click(&mut window, multi_button(&pairs, "Close"), &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !pairs
            .iter()
            .any(|(_, node)| node.value() == Some(MULTI_TITLE)),
        "premise: the dialog closed"
    );
    open_multi_aatf(&mut window, &mut view);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let ends: Vec<Option<&str>> = range_combos(&pairs)
        .into_iter()
        .map(accesskit::Node::value)
        .collect();
    assert_eq!(
        ends,
        [Some("734 /lit/"), Some("734 /lit/")],
        "the range is kept"
    );
}

/// A team the check finds clean reads `All clean` under its line (714
/// /co/ of the PES 15 sample, made legal by the CLI's `official_legal.toml`;
/// the PES 17 sample holds no clean team under 4cc Autumn 26), and Copy
/// to clipboard says how many teams it copied.
// SE-GUI-41
// SE-GUI-51
#[test]
fn a_clean_team_reads_all_clean_and_copy_names_the_teams_copied() {
    let (mut view, mut window) = open_legal_714("multi_aatf_clean", Headless::new());
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    pick_range_end(&mut window, &mut view, 0, "714 /co/");
    pick_range_end(&mut window, &mut view, 1, "718 /f/");
    let pairs = multi_checked(&mut window, &mut view, &[714, 715, 716, 717, 718]);
    let lines = multi_lines(&pairs);
    let at = lines
        .iter()
        .position(|line| line == "714 /co/")
        .expect("714 is listed");
    assert_eq!(lines[at + 1], "All clean", "{lines:?}");
    assert_eq!(
        lines.iter().filter(|line| *line == "All clean").count(),
        1,
        "the other teams have rows"
    );

    let text = copied(&mut window, &mut view, &pairs).expect("the click copies the list");
    assert_eq!(
        view.notice.as_ref().map(|notice| notice.text.as_str()),
        Some("Results of 5 teams copied.")
    );
    let copied_lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        copied_lines[0], "Height systems: Green 2, Red 3",
        "the first line counts the systems the range read under"
    );
    assert!(
        copied_lines[1].starts_with("4cc Autumn 26, teams 714 to 718: "),
        "the second line names the ruleset and the range: {text}"
    );
    let teams: Vec<&str> = copied_lines
        .iter()
        .filter_map(|line| line.strip_prefix('>'))
        .collect();
    let expected: Vec<String> = (714..=718)
        .map(|id| format!("{id} {}", display_name(&team_entry(&view, id).name)))
        .collect();
    assert_eq!(teams, expected, "every team of the range after a `>`");
    let at = copied_lines
        .iter()
        .position(|line| *line == ">714 /co/")
        .expect("714 in the copied text");
    assert_eq!(copied_lines[at + 1], "  All clean", "{text}");
}

/// Copy to clipboard clicked (press on one frame, release on the next):
/// the text the release frame's `CopyText` command carries, `None` when it
/// copies nothing.
fn copied(
    window: &mut Headless,
    view: &mut SaveView,
    pairs: &[(accesskit::NodeId, accesskit::Node)],
) -> Option<String> {
    let at = centre(multi_button(pairs, "Copy to clipboard"));
    window.frame(pointer_at(at, true), |ui| show(ui, view, &|| Vec::new()));
    let (commands, _, _) = window.frame(pointer_at(at, false), |ui| {
        show(ui, view, &|| Vec::new());
        ui.ctx().output(|output| output.commands.clone())
    });
    commands.into_iter().find_map(|command| match command {
        egui::OutputCommand::CopyText(text) => Some(text),
        _ => None,
    })
}

/// A range where one team's check fails (743 under THROW19, whose custom
/// check throws on 743's captain) names that team with its error and lists
/// the other team's rows, in the drawn list and in the copied text.
#[test]
fn a_failed_teams_check_is_named_with_its_error_and_the_others_listed() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_failed");
    let mut view = SaveView::new();
    view.open(&path);
    pick_imported(&mut view, "throw19.4ccrules", &throw19());
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view);
    multi_checked(&mut window, &mut view, &ids[..1]);
    let label =
        |view: &SaveView, id: u32| format!("{id} {}", display_name(&team_entry(view, id).name));
    let (failed, other) = (label(&view, 743), label(&view, 744));
    let other_rows = {
        let file = view.session.as_ref().expect("the save opened").file();
        let rules = aatf::Rules::load(&throw19()).expect("THROW19 loads");
        let checks = crate::session::rules::check_teams(
            &rules,
            &crate::session::rules::team_views(file, &[743, 744]),
        );
        assert!(checks[0].is_err(), "premise: 743's check fails");
        checks[1]
            .as_ref()
            .expect("premise: 744's check runs")
            .violations
            .iter()
            .filter(|violation| violation.severity != aatf::Severity::Suggestion)
            .count()
    };
    assert!(other_rows > 0, "premise: 744 has rows");
    pick_range_end(&mut window, &mut view, 0, &failed);
    pick_range_end(&mut window, &mut view, 1, &other);
    let pairs = multi_checked(&mut window, &mut view, &[743, 744]);

    let lines = multi_lines(&pairs);
    let failed_line = format!("The AATF check failed for team {failed}: ");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with(&failed_line) && line.contains("boom")),
        "743 named with its error: {lines:?}"
    );
    assert!(
        lines.contains(&"2 teams checked, 1 with issues".to_owned()),
        "{lines:?}"
    );
    let at = lines
        .iter()
        .position(|line| *line == other)
        .expect("744 is listed");
    // The list's group headers: a 744xx player's head or the team's own.
    let group = &lines[at + 1];
    assert!(
        group.starts_with("Player 744") || group == "Team-wide issues",
        "a group header of 744's rows under its line: {lines:?}"
    );

    let text = copied(&mut window, &mut view, &pairs).expect("the click copies the list");
    let copied_lines: Vec<&str> = text.lines().collect();
    let at = copied_lines
        .iter()
        .position(|line| *line == format!(">{failed}"))
        .expect("743 in the copied text");
    assert!(
        copied_lines[at + 1].starts_with("  The AATF check failed: ")
            && copied_lines[at + 1].contains("boom"),
        "743's error under its line: {text}"
    );
    let at = copied_lines
        .iter()
        .position(|line| *line == format!(">{other}"))
        .expect("744 in the copied text");
    assert_eq!(
        copied_lines[at + 1..]
            .iter()
            .filter(|line| line.starts_with("    "))
            .count(),
        other_rows,
        "744's rows: {text}"
    );
}

/// A list checked under 4cc Autumn 26 is not drawn once 4cc Spring 26 is
/// picked, and Copy to clipboard is off: its first line would name Spring
/// 26 over Autumn 26's findings. The dialog reads as before its first
/// result until the recheck returns, then lists Spring 26's. The recheck
/// is held unanswered by a pending check whose sender the test keeps: the
/// dialog starts one on any frame, and its worker can reply between a
/// frame's two passes.
// SE-GUI-41
#[test]
fn a_list_checked_under_another_ruleset_is_not_drawn_nor_copied() {
    let path = fixture(PesVersion::Pes17, "multi_aatf_other_ruleset");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    open_multi_aatf(&mut window, &mut view);
    // The range opens on the save's first team alone.
    let ids = all_team_ids(&view)[..1].to_vec();
    let team = format!(
        "{} {}",
        ids[0],
        display_name(&team_entry(&view, ids[0]).name)
    );
    let pairs = multi_checked(&mut window, &mut view, &ids);
    let lines = multi_lines(&pairs);
    let tier_row = |line: &String| line.starts_with("Tier count");
    assert!(
        lines.contains(&team) && lines.iter().any(tier_row),
        "premise: Autumn 26's list holds {team} and a tier count row: {lines:?}"
    );

    view.aatf_ruleset = "spring26".to_owned();
    let (sender, replies) = crossbeam_channel::unbounded();
    view.multi_aatf.check.pending = Some(aatf_check::PendingCheck {
        key: aatf_check::CheckKey {
            revision: view.session.as_ref().expect("the save opened").revision(),
            ids: ids.clone(),
            ruleset: "spring26".to_owned(),
            suggestions: false,
        },
        replies,
    });
    let title = "Multi-team AATF (4cc Spring 26)";
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.multi_aatf
            .check
            .result
            .as_ref()
            .is_some_and(|(key, _)| key.ruleset == "autumn26"),
        "premise: Autumn 26's list held"
    );
    let nodes = super::prompt::dialog_nodes(&pairs, title);
    fn text<'a>(node: &&'a accesskit::Node) -> Option<&'a str> {
        match node.role() {
            Role::Label => node.value(),
            _ => node.label(),
        }
    }
    assert!(
        !nodes
            .iter()
            .filter_map(text)
            .any(|line| line == team || line.starts_with("Tier count")),
        "the held list's rows are not drawn"
    );
    assert!(
        !nodes
            .iter()
            .filter_map(text)
            .any(|line| line.contains("checked,")),
        "no summary before a result"
    );
    let copy = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Copy to clipboard"))
        .expect("the Copy to clipboard button");
    assert!(copy.is_disabled(), "Copy to clipboard is off");

    // The held check's worker ends: the next frame starts Spring 26's.
    drop(sender);
    let pairs = frames_until(
        &mut window,
        &mut view,
        |view| {
            let check = &view.multi_aatf.check;
            check.pending.is_none()
                && check
                    .result
                    .as_ref()
                    .is_some_and(|(key, _)| key.ruleset == "spring26")
        },
        "Spring 26's check never returned",
    );
    let nodes = super::prompt::dialog_nodes(&pairs, title);
    assert!(
        nodes.iter().filter_map(text).any(|line| line == team),
        "Spring 26's list is drawn"
    );
    let summary = "1 team checked,";
    assert!(
        nodes
            .iter()
            .filter_map(text)
            .any(|line| line.starts_with(summary)),
        "Spring 26's summary"
    );
    let copy = nodes
        .iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Copy to clipboard"))
        .expect("the Copy to clipboard button");
    assert!(!copy.is_disabled(), "Copy to clipboard is on");
}
