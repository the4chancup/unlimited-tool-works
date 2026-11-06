use super::*;

// SE-GUI-18
#[test]
fn the_preview_lists_each_changed_value_and_writes_nothing() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_preview");
    let before = revision(&view);
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    previewed(&mut window, &mut view, &text);

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03".to_owned(),
        "Apply".to_owned(),
    ];
    expected.extend(table(&CO_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(!apply.is_disabled(), "Apply is enabled");
    assert_eq!(revision(&view), before, "the preview writes nothing");
    assert!(!view.session.as_ref().unwrap().can_undo(), "no undo entry");
}

// SE-GUI-18
#[test]
fn a_patch_of_another_version_is_refused_with_the_save_untouched() {
    let (mut view, mut window) = open_save(PesVersion::Pes17, "patch_refused");
    let before = revision(&view);
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let path = previewed(&mut window, &mut view, &text);

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}".to_owned(),
            format!(
                "{}: the patch is for PES 2021, the save is PES 2017",
                path.display()
            ),
        ]
    );
    assert!(
        panel_button(&mut window, &mut view, "Apply").is_none(),
        "no Apply button"
    );
    assert_eq!(revision(&view), before, "the save untouched");
}

// SE-GUI-18
#[test]
fn a_team_the_save_lacks_is_a_warning_line_and_the_rest_previews() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_skipped_team");
    assert!(
        view.session.as_ref().unwrap().file().team(999).is_none(),
        "premise: the fixture lacks team 999"
    );
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view), zz_team()],
    );
    previewed(&mut window, &mut view, &text);

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03".to_owned(),
        "Apply".to_owned(),
        "Team 999 /zz/ is not in this save: its players are skipped.".to_owned(),
    ];
    expected.extend(table(&CO_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn a_patch_the_save_holds_no_player_of_shows_its_warnings_alone() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_all_skipped");
    assert!(
        view.session.as_ref().unwrap().file().team(999).is_none(),
        "premise: the fixture lacks team 999"
    );
    previewed(
        &mut window,
        &mut view,
        &patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![zz_team()],
        ),
    );

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml",
            "Apply",
            "Team 999 /zz/ is not in this save: its players are skipped.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing to apply");
}

// SE-GUI-18
#[test]
fn a_second_patch_opened_replaces_the_first_ones_preview() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_second_open");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let path = previewed(&mut window, &mut view, &text);
    assert_eq!(
        panel_lines(&mut window, &mut view)[1],
        "patch.toml: team 714 /co/, players 03",
        "premise: the first patch previews"
    );
    let before = revision(&view);

    // The same file rewritten with `/f/`'s block, opened again: the save
    // has not changed.
    let f_corner_kick = team(
        F,
        "/f/",
        BTreeMap::from([(
            1,
            PatchPlayer {
                appearance: Some(look_of(
                    file_player(&view, F_SLOT_1),
                    PesVersion::Pes21,
                    &[(SettingKey::CornerKick, 2)],
                )),
                ..bare()
            },
        )]),
    );
    std::fs::write(
        &path,
        patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![f_corner_kick],
        ),
    )
    .unwrap();
    open_patch(&mut window, &mut view, path);
    assert_eq!(revision(&view), before, "premise: the save is unchanged");

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 718 /f/, players 01".to_owned(),
        "Apply".to_owned(),
    ];
    expected.extend(table(&[
        ["PLACEHOLDER \u{b7} Corner kick", "1", "3"],
        ["PLACEHOLDER \u{b7} Copy from ID", "7511", "71801"],
    ]));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn the_summary_lists_the_teams_that_apply_and_each_skip_is_a_line() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_two_teams");
    let missing = co_missing_slot(&view);
    let file = view.session.as_ref().unwrap().file();
    for player in [CO_SLOT_3, CO_SLOT_5, F_SLOT_1] {
        assert!(file.player(player).is_some(), "premise: {player} is held");
    }
    assert!(
        file.team(999).is_none(),
        "premise: the fixture lacks team 999"
    );
    let text = two_team_patch(&view, missing);
    previewed(&mut window, &mut view, &text);

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03, 05; team 718 /f/, players 01".to_owned(),
        "Apply".to_owned(),
        format!(
            "Player {} is not in this save: skipped.",
            CO * 100 + missing
        ),
        "Team 999 /zz/ is not in this save: its players are skipped.".to_owned(),
    ];
    expected.extend(table(&TWO_TEAM_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn a_team_whose_every_patch_player_the_save_lacks_is_not_in_the_summary() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_team_no_players");
    let missing = co_missing_slot(&view);
    let (slot, absent) = co_absent_player(missing);
    previewed(
        &mut window,
        &mut view,
        &patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![team(CO, "/co/", BTreeMap::from([(slot, absent)])), f_team()],
        ),
    );

    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 718 /f/, players 01".to_owned(),
        "Apply".to_owned(),
        format!(
            "Player {} is not in this save: skipped.",
            CO * 100 + missing
        ),
    ];
    expected.extend(table(&TWO_TEAM_ROWS[4..]));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn a_file_that_is_not_toml_is_refused_in_the_parse_errors_words() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_not_toml");
    let before = revision(&view);
    let text = "this is not a patch = =";
    let error = AestheticsPatch::parse(text)
        .expect_err("premise: the text does not parse")
        .to_string();
    assert!(
        error.starts_with("aesthetics_patch.toml is not valid TOML: "),
        "premise: a TOML parse error, got {error:?}"
    );
    let path = previewed(&mut window, &mut view, text);

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}".to_owned(),
            format!("{}: {error}", path.display()),
        ]
    );
    assert!(
        panel_button(&mut window, &mut view, "Apply").is_none(),
        "no Apply button"
    );
    assert_eq!(revision(&view), before, "the save untouched");
}

// SE-GUI-18
#[test]
fn a_file_that_cannot_be_read_is_refused() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_missing_file");
    let path = view
        .session
        .as_ref()
        .unwrap()
        .path()
        .with_file_name("missing.toml");
    let error = std::fs::read_to_string(&path)
        .expect_err("premise: no file at the path")
        .to_string();
    open_panel(&mut window, &mut view);
    open_patch(&mut window, &mut view, path.clone());

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}".to_owned(),
            format!("{}: cannot read the file ({error})", path.display()),
        ]
    );
    assert!(
        panel_button(&mut window, &mut view, "Apply").is_none(),
        "no Apply button"
    );
}

// SE-GUI-18
#[test]
fn the_preview_follows_another_save_opened() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_reopen");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let patched = patched_entry(&view, &text, CO_SLOT_3);
    previewed(&mut window, &mut view, &text);
    let mut entry = player_entry(&view, CO_SLOT_3);
    assert_ne!(
        entry.motion.corner_kick, patched.motion.corner_kick,
        "premise: the patch changes the corner kick"
    );
    entry.motion.corner_kick = patched.motion.corner_kick;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    assert_eq!(
        panel_lines(&mut window, &mut view)[3..],
        table(&CO_ROWS[1..]),
        "premise: the corner kick's row goes"
    );

    // The fixture opened again from a fresh copy: its corner kick is the
    // fixture's.
    view.open(&fixture(PesVersion::Pes21, "patch_reopen_fresh"));
    assert_eq!(view.panel, Some(Panel::Patch), "the panel stays open");
    let mut expected = vec![
        "Open patch\u{2026}".to_owned(),
        "patch.toml: team 714 /co/, players 03".to_owned(),
        "Apply".to_owned(),
    ];
    expected.extend(table(&CO_ROWS));
    assert_eq!(panel_lines(&mut window, &mut view), expected);
}

// SE-GUI-18
#[test]
fn a_refusal_is_painted_in_the_error_colour_and_a_skip_in_the_warning_colour() {
    let (mut view, mut window) = open_save(PesVersion::Pes17, "patch_colour_refusal");
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    assert_ne!(
        visuals.error_fg_color, visuals.warn_fg_color,
        "premise: the two colours differ"
    );
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let path = previewed(&mut window, &mut view, &text);
    let refusal = format!(
        "{}: the patch is for PES 2021, the save is PES 2017",
        path.display()
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        painted_label_color(&nodes, &shapes, &refusal),
        visuals.error_fg_color,
        "the refusal in the error colour"
    );

    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_colour_skip");
    previewed(
        &mut window,
        &mut view,
        &patch_text(
            PesVersion::Pes21,
            ALLOCATION_SCHEME_VERSION,
            vec![zz_team()],
        ),
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        painted_label_color(
            &nodes,
            &shapes,
            "Team 999 /zz/ is not in this save: its players are skipped."
        ),
        visuals.warn_fg_color,
        "the skip in the warning colour"
    );
}

#[test]
fn the_previews_values_are_in_the_monospace_font() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_monospace");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    previewed(&mut window, &mut view, &text);
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let monospace = egui::TextStyle::Monospace.resolve(&window.ctx.style_of(window.ctx.theme()));
    // The Name row's new value, a text no other cell starts with.
    let painted = text_shape(&shapes, "PATCHED");
    assert!(
        !painted.galley.job.sections.is_empty(),
        "premise: painted sections"
    );
    for section in &painted.galley.job.sections {
        assert_eq!(section.format.font_id, monospace, "the value's font");
    }
}
