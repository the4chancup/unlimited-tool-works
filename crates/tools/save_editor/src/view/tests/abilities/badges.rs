use super::*;

// The Cards used badge is one badge, the player's own tier under the
// active ruleset (`label: paid of limit`); a ticked skill that no free
// group covers raises it by one; the active ruleset's reading is the
// badge's.
#[test]
fn the_cards_used_badge_reads_the_players_own_tier() {
    /// The badge text `rules` reads for team 701's roster `slot` on the
    /// session's file: his own tier's label over his card economy under
    /// it. The checked player must read as a tier that is not the
    /// ruleset's first — the badge then proves his own tier was read,
    /// not the list's head.
    fn expected(view: &SaveView, rules: &aatf::Rules, slot: u8) -> String {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        let player = team.player(slot).unwrap();
        let tier = rules
            .player_tier(&team, player)
            .unwrap()
            .expect("the checked player matches a tier");
        assert_ne!(
            tier.id,
            rules.ruleset().tiers[0].id,
            "premise: slot {slot}'s own tier is not the ruleset's first"
        );
        let limits = rules.card_limits(&team, player, &tier.id).unwrap();
        format!("{}: {} of {}", tier.label, limits.paid, limits.limit)
    }
    /// The `paid` an `expected` badge text carries (`"Regular: 3 of 7"`
    /// reads 3).
    fn paid_of(view: &SaveView, rules: &aatf::Rules, slot: u8) -> i32 {
        expected(view, rules, slot)
            .split(": ")
            .nth(1)
            .expect("a badge text")
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .expect("the paid count")
    }

    let path = fixture(PesVersion::Pes21, "cards_used");
    // Slot 1's player holds Heading (skill 6), a paid card on every tier:
    // his badge is one lower than slot 0's unticked player's.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let second = file.team(701).unwrap().roster[1].player_id;
    file.player_mut(second).unwrap().skills.skills[6] = true;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let rules = aatf::Rules::official();
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [expected(&view, &rules, 0)],
        "the one badge, his own tier's"
    );

    // "Heading" (skill 6) is unticked and counts paid on every tier: the
    // tick raises the badge by one.
    let before = paid_of(&view, &rules, 0);
    // The checkbox sits below the fold under the player header: the tick
    // goes through its AccessKit action (a pointer click lands only
    // inside the scroll's clip).
    let (_, nodes, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (heading, _) = nodes
        .iter()
        .find(|(_, node)| node.role() == Role::CheckBox && node.label() == Some("Heading"))
        .expect("the Heading checkbox");
    action_click(&mut window, &mut view, *heading);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [expected(&view, &rules, 0)],
        "after the tick"
    );
    assert_eq!(paid_of(&view, &rules, 0), before + 1, "one more card used");

    // Ctrl+Z unticks it: the badge is recomputed, back to the old count.
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [expected(&view, &rules, 0)],
        "after the undo"
    );
    assert_eq!(paid_of(&view, &rules, 0), before, "the badge restored");

    // The selection recomputes it: slot 1's player, his Heading held,
    // shows his own one-more count — not slot 0's.
    view.selected = Some(Selection::Player(701, 1));
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [expected(&view, &rules, 1)],
        "his badge, not slot 0's"
    );

    // The ruleset's reading is the badge's: the same view now set to DBC7
    // reloads on the changed setting (no fresh window).
    let dbc7 = aatf::builtin("dbc7").expect("DBC7 is built in");
    view.aatf_ruleset = "dbc7".to_owned();
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [expected(&view, &dbc7, 1)],
        "the badge under DBC7"
    );
}

// The Cards used badge, painted: paid past the tier's limit reads the
// error colour (slot 0's player made to hold paid cards past his own
// tier's limit by ticking skills in the fixture copy, as the Heading tick
// above does). A colour is no AccessKit property, so it is read back from
// the frame's painted shapes.
#[test]
fn an_overdrawn_badge_paints_in_the_error_colour() {
    /// The color the text `text` was painted in this frame (`galley_color`
    /// on its shape).
    fn painted_color(shapes: &[egui::epaint::ClippedShape], text: &str) -> egui::Color32 {
        let shape = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no painted text {text:?}"));
        galley_color(shape)
    }

    let path = fixture(PesVersion::Pes21, "cards_used_overdrawn");
    // The first dozen skills ticked: enough paid cards that his own
    // tier's limit is passed.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    let id = file.team(701).unwrap().roster[0].player_id;
    for skill in 0..12 {
        file.player_mut(id).unwrap().skills.skills[skill] = true;
    }
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let rules = aatf::Rules::official();
    let badge = {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        let player = team.player(0).unwrap();
        let tier = rules
            .player_tier(&team, player)
            .unwrap()
            .expect("the player matches a tier");
        let limits = rules.card_limits(&team, player, &tier.id).unwrap();
        assert!(
            i32::from(limits.paid) > i32::from(limits.limit),
            "premise: overdrawn (paid {} of {})",
            limits.paid,
            limits.limit
        );
        format!("{}: {} of {}", tier.label, limits.paid, limits.limit)
    };
    badge_landed(&mut window, &mut view);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let error_color = window
        .ctx
        .style_of(window.ctx.theme())
        .visuals
        .error_fg_color;
    assert_eq!(painted_color(&shapes, &badge), error_color);
}

// The cards' labels paint the normal text colour, `Stronger foot`'s (a
// card is a player's field, not a setting); a hovered card's label still
// brightens to the hovered state's.
// SE-GUI-66
#[test]
fn the_cards_labels_paint_the_normal_text_colour() {
    let path = fixture(PesVersion::Pes21, "cards_label_colour");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let (text_color, weak_color, hovered_color) = (
        visuals.text_color(),
        visuals.weak_text_color(),
        visuals.widgets.hovered.fg_stroke.color,
    );
    assert_ne!(text_color, weak_color, "premise: the two greys differ");
    assert_ne!(text_color, hovered_color, "premise: the hover's differs");
    let colour_of = |nodes: &[accesskit::Node], shapes: &[egui::epaint::ClippedShape]| {
        let bounds = node_labeled(nodes, Role::CheckBox, "Heel Trick")
            .bounds()
            .expect("bounds");
        let shape = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if shape.galley.job.text == "Heel Trick"
                        && f64::from(shape.pos.x) >= bounds.x0 - 1.0
                        && f64::from(shape.pos.x) <= bounds.x1 + 1.0
                        && f64::from(shape.pos.y) >= bounds.y0 - 1.0
                        && f64::from(shape.pos.y) <= bounds.y1 + 1.0 =>
                {
                    Some(shape)
                }
                _ => None,
            })
            .next()
            .unwrap_or_else(|| panic!("no painted Heel Trick inside {bounds:?}"));
        galley_color(shape)
    };
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        colour_of(&nodes, &shapes),
        text_color,
        "Heel Trick's label in the normal colour"
    );

    // The pointer on it: the label brightens as a button's hover does.
    let node = node_labeled(&nodes, Role::CheckBox, "Heel Trick").clone();
    let at = centre(&node);
    let (_, _, shapes) = window.frame(vec![egui::Event::PointerMoved(at)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        colour_of(&nodes, &shapes),
        hovered_color,
        "Heel Trick's label brightens on hover"
    );
}

// The Cards used badge: a player no tier's `when` holds for (`check_team`'s
// `NoTier`) gets the stand-in. VTL11's tiers each gate on an exact
// height (gold 195, silver 195, regular 180, buffed 188), so most real
// players match none — PES 21 fixture team 781 slot 0, player 78101.
#[test]
fn a_player_no_tier_matches_gets_the_stand_in() {
    let path = fixture(PesVersion::Pes21, "cards_used_no_tier");
    let rules = aatf::builtin("vtl11").expect("VTL11 is built in");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vtl11".to_owned();
    view.selected = Some(Selection::Player(781, 0));
    {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 781).unwrap();
        let player = team.player(0).unwrap();
        assert_eq!(
            rules.player_tier(&team, player).unwrap().map(|t| &t.id),
            None,
            "premise: 78101 matches no VTL11 tier"
        );
    }
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        ["No tier matches"],
        "the stand-in, the one badge"
    );
}

// The Cards used badge: a ruleset that loads but whose `player_tier` fails on
// the selected player (a member's own logic edit) draws the red click line
// in the badge's place, not `No tier matches` (which would tell him the
// opposite), whose click opens the error; and the failure is kept under
// its key like a badge, so the failing script does not run again on every
// frame.
#[test]
fn a_failing_tier_check_shows_its_error_in_the_badges_place() {
    let path = fixture(PesVersion::Pes21, "cards_used_failing_check");
    let logic = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf/logic/logic.rhai"),
    )
    .unwrap();
    let head = "fn player_tier(player, team) {\n";
    assert_eq!(logic.matches(head).count(), 1, "premise: one player_tier");
    let failing = logic.replacen(
        head,
        &format!("{head}    throw \"the tier check fails\";\n"),
        1,
    );
    let text = aatf::assemble_file(aatf::Rules::official().ruleset(), Some(&failing), None);
    let rules = aatf::Rules::load(&text).expect("premise: the file loads");
    let mut view = SaveView::new();
    view.open(&path);
    pick_imported(&mut view, "failing.4ccrules", &text);
    {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 701).unwrap();
        assert!(
            rules.player_tier(&team, team.player(0).unwrap()).is_err(),
            "premise: the check fails on the selected player"
        );
    }
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    // `Cards used:`, then the red click line in the badge's place.
    const LINE: &str = "Tier check failed, click for more information.";
    let error_node = {
        let column_x0 = skills_header(&nodes).x0;
        nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value() == Some(LINE)
                    && node.bounds().is_some_and(|b| b.x0 >= column_x0 - 1.0)
            })
            .expect("the tier check's line in the badge's place")
    };
    let style = window.ctx.style_of(window.ctx.theme());
    assert_eq!(
        galley_color(text_shape(&shapes, LINE)),
        style.visuals.error_fg_color,
        "the line in the error colour"
    );
    assert!(
        in_skills_column(&nodes, "Cards used:"),
        "Cards used: heads it"
    );
    assert!(
        !in_skills_column(&nodes, "No tier matches"),
        "no stand-in beside it"
    );
    let expected = match view.cards.result.as_ref() {
        Some((_, crate::view::OwnBadge::Failed(error))) => error.clone(),
        _ => panic!("the failure is kept under its key"),
    };
    assert!(
        expected.contains("the tier check fails"),
        "premise: the check's error: {expected}"
    );

    // The dialog is up only after the click.
    assert!(
        !label_value(&nodes, "The tier check failed"),
        "no dialog title before the click"
    );
    assert!(view.refusal.is_none(), "no refusal before the click");

    // Its click opens the refusal dialog with the check's own error.
    click(&mut window, error_node, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "The tier check failed"),
        "the dialog's title"
    );
    assert!(
        label_value(&nodes, &expected),
        "the dialog's body is the check's error"
    );
}

// The Cards used badge's limit counts the captain's untaken choice:
// team 781's captain (roster slot 0, a regular under VGL 27) holds no
// paid cards, no second COM style and no A position past the tier's,
// so no option is needed and the choice stays untaken — the badge
// reads `of` the regular limit plus the choice's card. A COM style
// held takes the `free: com` option instead, and the limit drops back.
// SE-GUI-60
#[test]
fn the_captains_untaken_choice_counts_its_card_in_the_badges_limit() {
    let path = fixture(PesVersion::Pes19, "cards_used_captain_choice");
    let rules = aatf::builtin("vgl27").expect("VGL27 is built in");
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vgl27".to_owned();
    view.selected = Some(Selection::Player(781, 0));
    let (label, paid, tier_limit) = {
        let session = view.session.as_ref().unwrap();
        assert_eq!(
            session.file().team(781).unwrap().tactics.set_pieces.captain,
            0,
            "premise: roster slot 0 is the captain"
        );
        let team = aatf::TeamView::new(session.file(), 781).unwrap();
        let player = team.player(0).unwrap();
        let tier = rules
            .player_tier(&team, player)
            .unwrap()
            .expect("the captain matches a tier");
        assert_eq!(tier.id.as_str(), "regular", "premise: a regular");
        let limits = rules.card_limits(&team, player, &tier.id).unwrap();
        assert!(
            limits.paid <= tier.cards,
            "premise: his paid cards under the tier's limit"
        );
        (
            tier.label.clone(),
            i32::from(limits.paid),
            i32::from(tier.cards),
        )
    };
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [format!("{label}: {paid} of {}", tier_limit + 1)],
        "the choice's card counted in the limit while it is untaken"
    );

    // A COM style held (the regular tier frees none): `free: com` takes
    // the choice — the map's one `com` entry is the option's — and the
    // badge's limit drops back to the tier's.
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes19)).unwrap();
    let captain_id = file.team(781).unwrap().roster[0].player_id;
    file.player_mut(captain_id).unwrap().skills.com_styles[0] = true;
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.aatf_ruleset = "vgl27".to_owned();
    view.selected = Some(Selection::Player(781, 0));
    {
        let session = view.session.as_ref().unwrap();
        let team = aatf::TeamView::new(session.file(), 781).unwrap();
        let limits = rules
            .card_limits(&team, team.player(0).unwrap(), "regular")
            .unwrap();
        assert_eq!(
            limits.free.get("com"),
            Some(&1),
            "premise: the option's free COM style over the tier's none"
        );
    }
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        [format!("{label}: {paid} of {tier_limit}")],
        "the limit drops back to the tier's"
    );
}

// The Cards used badge recomputes on a worker thread: an edit moves its
// key and starts a new run, and until that answer lands the held badge
// of the same player stays drawn.
#[test]
fn an_edit_keeps_the_players_badge_drawn_while_the_new_one_runs() {
    let path = fixture(PesVersion::Pes17, "cards_used_kept");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    let kept = cards_used_badges(&nodes);
    assert_eq!(kept.len(), 1, "premise: the player's badge landed");

    // A stat edit, as a wheel notch over his Speed writes one: the
    // revision moves the badge's key. The new run is held in flight by
    // the channel the test keeps the sender of, keyed to the post-edit
    // key: a fast run replies between one frame's two passes, so a real
    // worker cannot be waited on here.
    let player_id = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(702)
        .unwrap()
        .roster[0]
        .player_id;
    let mut edited = player_entry(&view, player_id);
    assert_ne!(edited.stats.speed, 99, "premise: a real change");
    edited.stats.speed = 99;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let sender = hold_badge_run(&mut view);
    for _ in 0..2 {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert_eq!(
            cards_used_badges(&nodes),
            kept,
            "the kept badge while the new run is in flight"
        );
        assert!(view.cards.pending.is_some(), "the new run is in flight");
    }

    // The held run ends unanswered: the next frames start the real one,
    // which lands the post-edit badge.
    drop(sender);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(cards_used_badges(&nodes).len(), 1, "the new badge lands");
}

// Another player's badge draws nothing until his own lands: the frame
// after the selection moves draws the Skills title alone — no Cards
// used row, no kept badge — then his badge lands.
#[test]
fn another_player_draws_no_badge_until_his_lands() {
    let path = fixture(PesVersion::Pes17, "cards_used_other_player");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes).len(),
        1,
        "premise: slot 0's badge landed"
    );

    // His run held in flight by the channel the test keeps the sender
    // of, keyed to his own key: a fast run replies between one frame's
    // two passes, so a real worker cannot be waited on here.
    view.selected = Some(Selection::Player(702, 1));
    let sender = hold_badge_run(&mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(in_skills_column(&nodes, "Skills"), "the title alone");
    assert!(
        !in_skills_column(&nodes, "Cards used:"),
        "no Cards used row for another player"
    );
    assert!(
        !in_skills_column(&nodes, "Cards used for"),
        "no Cards used row for another player"
    );
    assert!(
        cards_used_badges(&nodes).is_empty(),
        "no badge drawn while his run is up"
    );

    // The held run ends unanswered: the next frames start the real one,
    // which lands his badge.
    drop(sender);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(cards_used_badges(&nodes).len(), 1, "his badge lands");
}

// Opening another save drops the badge with the rest of the old save's
// state: on the same player the first frame draws no Cards used row —
// the answer it would hold was the previous save's — and the new save's
// run paints it back once it lands.
#[test]
fn opening_another_save_keeps_no_cards_used_badge_from_the_previous_one() {
    let path = fixture(PesVersion::Pes17, "cards_used_open");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    let badge = cards_used_badges(&nodes);
    assert_eq!(badge.len(), 1, "premise: the player's badge landed");

    // The same fixture opened again: the point is the open, not the
    // file.
    open_again(&mut view, &path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));

    // The new run in flight, held by the channel the test keeps the
    // sender of, keyed to the new session's key: a fast run replies
    // between one frame's two passes, so a real worker cannot be
    // waited on here.
    let sender = hold_badge_run(&mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.cards.pending.is_some(),
        "premise: the run is in flight"
    );
    assert!(
        view.cards.result.is_none(),
        "the previous save's badge went with it"
    );
    assert!(
        cards_used_badges(&nodes).is_empty(),
        "no badge drawn while the new save's run is up"
    );

    // The held run ends unanswered: the next frames start the real one,
    // which lands the badge back.
    drop(sender);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes),
        badge,
        "the new save's run lands the badge back"
    );
}

// The badge keeps one run in flight: an edit while a run is up waits
// for its reply, then starts the queued run — the frame the reply lands
// on asks for the repaint that wakes the next, so no input is needed.
// The test's own channel holds the first run pending, so no reply can
// reach a frame until `send` below.
#[test]
fn a_key_moved_mid_run_starts_its_badge_when_the_reply_lands() {
    let path = fixture(PesVersion::Pes17, "cards_used_one_run");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes).len(),
        1,
        "premise: the badge landed"
    );

    // A run in flight for key A, held by the channel the test keeps the
    // sender of: no reply reaches a frame until `send` below.
    let key_a = current_badge_key(&view);
    let sender = hold_badge_run(&mut view);

    // An edit moves the key to B: the frame starts no second run, the
    // pending one stays A's.
    let player_id = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .team(702)
        .unwrap()
        .roster[0]
        .player_id;
    let mut edited = player_entry(&view, player_id);
    assert_ne!(edited.stats.speed, 99, "premise: a real change");
    edited.stats.speed = 99;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        view.cards
            .pending
            .as_ref()
            .is_some_and(|pending| pending.key == key_a),
        "no second run: the pending one is still A's"
    );

    // A's reply lands on the next frame: it starts no run in the pass
    // that polled it (the pass entered with the run still marked in
    // flight), but asks for the frame that does.
    sender
        .send((key_a.clone(), OwnBadge::NoTier))
        .expect("the held channel takes A's reply");
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        window.repaint_delay.is_zero(),
        "the landing frame asked for the frame that starts B's run"
    );

    // No events: B's run is in flight or already landed — a fast run
    // replies between one frame's two passes — with no input asked for.
    let (_, _, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let key_b = current_badge_key(&view);
    assert!(
        view.cards
            .pending
            .as_ref()
            .is_some_and(|pending| pending.key == key_b)
            || view
                .cards
                .result
                .as_ref()
                .is_some_and(|(held, _)| *held == key_b),
        "B's run started with no input"
    );
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(cards_used_badges(&nodes).len(), 1, "B's badge lands");
}

// The badge landed and nothing is due: a frame with no input asks for
// no repaint, so a view left open repaints only on input or a reply.
#[test]
fn an_idle_view_asks_for_no_frame() {
    let path = fixture(PesVersion::Pes17, "cards_left_idle");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    view.selected = Some(Selection::Player(702, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let nodes = badge_landed(&mut window, &mut view);
    assert_eq!(
        cards_used_badges(&nodes).len(),
        1,
        "premise: the badge landed"
    );

    // No events and nothing due: the frames stop asking for a repaint. The
    // worker asks for one after it has sent the reply, so under load its
    // request can land on the frames after the landing one (egui answers a
    // request with two repainting frames); one frame asking for none is
    // the proof, where a view repainting forever asks on every frame. The
    // deadline is a hang guard, as in `frames_until`.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        if window.repaint_delay == Duration::MAX {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "an idle view keeps asking for a repaint"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
