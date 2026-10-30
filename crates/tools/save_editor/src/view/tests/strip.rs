use super::*;

// Every widget id under the strip and the view is the selected player's,
// so a text field's undo history and a number control's in-progress edit
// never cross players.
#[test]
fn a_name_edit_does_not_reach_the_next_players_field() {
    let path = fixture(PesVersion::Pes17, "cross_player_name");
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(702);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let name_of = |view: &SaveView, id: u32| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(id)
            .unwrap()
            .name
            .clone()
    };

    // Select 70201 (roster slot 0) and type an edit into the Name field.
    let file = EditFile::load(&path).unwrap();
    let a_id = file.team(702).unwrap().roster[0].player_id;
    let b_id = file.team(702).unwrap().roster[1].player_id;
    view.selected = Some(Selection::Player(702, 0));
    let a_before = name_of(&view, a_id);
    let b_before = name_of(&view, b_id);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some(a_before.as_str())
        })
        .expect("the Name field");
    click(&mut window, name_field, &mut view);
    window.frame(vec![egui::Event::Text("ZZZ".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_ne!(name_of(&view, a_id), a_before, "the edit landed on 70201");

    // Select 70202 (roster slot 1), focus his Name field, Ctrl+Z: his
    // field has only his own edit history — never 70201's text.
    view.selected = Some(Selection::Player(702, 1));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some(b_before.as_str())
        })
        .expect("the Name field");
    click(&mut window, name_field, &mut view);
    // The field's own undo answers Ctrl+Z (the `command` bit is what its
    // `matches_logically(COMMAND)` arm tests, as a real key event carries
    // it): 70202's field has none of 70201's history to restore.
    window.frame(
        vec![key(
            egui::Key::Z,
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND,
            true,
        )],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(name_of(&view, b_id), b_before, "70202's name is unchanged");
}

// The same rule across opens: egui keeps a TextEdit's undo history by
// widget id, so the pushed id carries the open's generation — a save
// opened next starts the same player id's Name field with no history,
// and Ctrl+Z past the field's own edit restores none of the last save's
// text. (egui's undoer only commits a change to an undo point once the
// text has sat for a second, so each typed stretch waits out its 60-odd
// headless frames.)
#[test]
fn a_second_saves_name_field_carries_none_of_the_first_saves_undo_history() {
    let path_a = fixture(PesVersion::Pes21, "history_a");
    let path_b = fixture(PesVersion::Pes21, "history_b");
    let mut view = SaveView::new();
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let name_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .name
            .clone()
    };
    /// The Name field's node: the enabled text edit right of the "Name
    /// (n/max)" label, holding `value` (the Shirt field carries the same
    /// text on this fixture, so position, not value alone, names it).
    fn name_field<'a>(nodes: &'a [accesskit::Node], value: &str) -> &'a accesskit::Node {
        let field = field_right_of_label(nodes, "Name (", Role::TextInput);
        assert!(!field.is_disabled(), "the Name field is enabled");
        assert_eq!(field.value(), Some(value), "the Name field holds {value:?}");
        field
    }
    let settle = |window: &mut Headless, view: &mut SaveView| {
        for _ in 0..70 {
            window.frame(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        }
    };

    // Save A: type into the selected player's (70101's) Name field, twice,
    // letting the field's undoer commit each stretch as an undo point.
    view.open(&path_a);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, name_field(&nodes, "PLACEHOLDER"), &mut view);
    window.frame(vec![egui::Event::Text("Z".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    settle(&mut window, &mut view);
    window.frame(vec![egui::Event::Text("Y".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    settle(&mut window, &mut view);
    let a_name = name_of(&view);
    assert_ne!(a_name, "PLACEHOLDER", "premise: save A's edit landed");

    // Save B (a fresh copy): one letter into the same player's Name field,
    // then two Ctrl+Z with the focus still on it — the first undoes the
    // letter, the second has nothing of this save's left to undo and must
    // not reach into save A's history.
    view.open(&path_b);
    let stored = name_of(&view);
    assert_eq!(stored, "PLACEHOLDER", "premise: save B opens unedited");
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    click(&mut window, name_field(&nodes, "PLACEHOLDER"), &mut view);
    window.frame(vec![egui::Event::Text("X".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    // A field's own undo answers Ctrl+Z: the event's `command` bit is what
    // its `matches_logically(COMMAND)` arm tests (a real platform event
    // carries it together with `ctrl`; `Modifiers::CTRL` alone is not).
    for _ in 0..2 {
        window.frame(
            vec![key(
                egui::Key::Z,
                egui::Modifiers::CTRL | egui::Modifiers::COMMAND,
                true,
            )],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        name_of(&view),
        stored,
        "the field's undo stops at save B's own edit: {a_name} never lands",
    );
}

// The Name and Shirt fields' left edges share the rows' label column and
// its gap, their right edges flush with the second half's boxes (the
// draft's `.combos` `1fr` field), not on the panel's right edge.
#[test]
fn the_name_and_shirt_fields_share_the_rows_edges() {
    let path = fixture(PesVersion::Pes21, "field_widths");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let name = name_field(&nodes).bounds().expect("bounds");
    let shirt = shirt_field(&nodes).bounds().expect("bounds");
    assert_eq!(name.x0, shirt.x0, "the two fields start at one x");
    assert_eq!(name.x1, shirt.x1, "the two fields end on one x");
    let shirt_box = right_of_label(&nodes, "Shirt number", Role::SpinButton)[0]
        .bounds()
        .expect("bounds");
    assert!(
        (name.x1 - shirt_box.x1).abs() < 0.5,
        "the fields end flush with the second half's boxes ({name:?} vs {shirt_box:?})"
    );
}

// General info's and Other stats' pair rows stand 22 apart, as the draft's
// `.pairs` rows do: an 18-point box and the grid's 4-point row gap.
#[test]
fn the_pair_rows_stand_22_apart() {
    let path = fixture(PesVersion::Pes21, "pair_row_pitch");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for pair in [
        ["Height", "Weight"],
        ["Weight", "Age"],
        ["Injury Resistance", "Weak Foot: Usage"],
    ] {
        let upper = label_bounds(&nodes, pair[0]).y0;
        let lower = label_bounds(&nodes, pair[1]).y0;
        assert!(
            (lower - upper - 22.0).abs() <= 0.5,
            "{} stands 22 below {}, not {}",
            pair[1],
            pair[0],
            lower - upper
        );
    }
}

// General info's three blocks (Name and Shirt; the pairs; Stronger foot to
// Playing style) stand 6 apart, 2 more than their rows, as the draft's
// `.gen` blocks: each label's top below the panel title's top is the
// draft's, measured in a browser, and Other stats
// under the panel moves with them.
#[test]
fn general_infos_blocks_stand_6_apart_as_the_drafts() {
    let path = fixture(PesVersion::Pes21, "general_blocks");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let title = label_bounds(&nodes, "General info").y0;
    let name = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|v| v.starts_with("Name ("))
        })
        .and_then(accesskit::Node::bounds)
        .expect("the Name (n/max) label")
        .y0;
    let mut offsets = vec![("Name", name - title)];
    for label in [
        "Height",
        "Weight",
        "Age",
        "Stronger foot",
        "Stronger hand",
        "Registered position",
        "Playing style",
        "Other stats",
        "Injury Resistance",
        "Weak Foot: Usage",
    ] {
        offsets.push((label, label_bounds(&nodes, label).y0 - title));
    }
    let draft = [
        ("Name", 22.5),
        ("Height", 68.5),
        ("Weight", 90.5),
        ("Age", 112.5),
        ("Stronger foot", 136.5),
        ("Stronger hand", 158.5),
        ("Registered position", 180.5),
        ("Playing style", 202.5),
        ("Other stats", 233.0),
        ("Injury Resistance", 255.5),
        ("Weak Foot: Usage", 277.5),
    ];
    for ((label, offset), (_, expected)) in offsets.iter().zip(draft) {
        assert!(
            (offset - expected).abs() <= 1.0,
            "{label} stands {offset} under the title, the draft's {expected}"
        );
    }
}

// A name or shirt name edit sets the player's edited-player flag, in the
// same undo entry.
#[test]
fn a_name_or_shirt_edit_sets_the_edited_player_flag() {
    let path = fixture(PesVersion::Pes21, "edit_flag");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().edit_flags.player = false;
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let flag = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .edit_flags
            .player
    };
    assert!(!flag(&view), "premise: the flag is clear");

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let name_field = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some("PLACEHOLDER")
        })
        .expect("the Name field");
    click(&mut window, name_field, &mut view);
    window.frame(vec![egui::Event::Text("X".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(flag(&view), "the edit set the flag, one entry");
    window.frame(
        vec![
            key(egui::Key::Escape, egui::Modifiers::NONE, true),
            key(egui::Key::Escape, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![key(egui::Key::Z, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(!flag(&view), "undo clears it again");

    // The shirt field sets the same flag.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = shirt_field(&nodes);
    click(&mut window, shirt, &mut view);
    window.frame(vec![egui::Event::Text("Z".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(flag(&view), "a shirt edit sets the flag too");
}

// The shirt name is upper-cased as typed.
#[test]
fn a_shirt_name_is_upper_cased_as_typed() {
    let path = fixture(PesVersion::Pes21, "uppercase_shirt");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = shirt_field(&nodes);
    let stored = shirt.value().expect("its text").to_owned();
    click(&mut window, shirt, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![egui::Event::Text("miyako".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .shirt_name,
        format!("{stored}miyako").to_uppercase(),
        "miyako stores as MIYAKO"
    );
}

// The portable text limits (spec/pes_savefile/operations.md "Team TOML"):
// a stored shirt name past 18 letters is drawn tinted with the versions'
// hover; an edit may shorten it a keystroke at a time, never lengthen.
#[test]
fn a_stored_shirt_past_18_is_tinted_and_shortens_never_lengthens() {
    // 70101 with a 20-letter shirt name, over the portable 18 inside PES
    // 21's own field.
    let path = fixture(PesVersion::Pes21, "long_shirt");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().shirt_name = "S".repeat(20);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let tint = visuals
        .extreme_bg_color
        .lerp_to_gamma(visuals.warn_fg_color, 0.25);

    // The field draws the tint and shows the versions' hover.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = shirt_field(&nodes);
    assert_eq!(
        field.value(),
        Some("S".repeat(20).as_str()),
        "the Shirt field holds the 20 letters"
    );
    let bounds = field.bounds().expect("bounds");
    assert_eq!(
        fill_at(&shapes, bounds),
        Some(tint),
        "the shirt field is tinted"
    );
    let tooltip = tooltip_after_hover(
        &mut window,
        &mut view,
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        ),
    );
    assert_eq!(
        tooltip,
        Some("PES 2019 and older show 18 letters of a shirt name (PES 2016 shows 16)".to_string()),
        "the shirt hover"
    );

    // Two backspaces: 18 lands (each step shorter than stored).
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some("S".repeat(20).as_str())
        })
        .expect("the Shirt field");
    click(&mut window, shirt, &mut view);
    for _ in 0..2 {
        window.frame(
            vec![
                key(egui::Key::Backspace, egui::Modifiers::NONE, true),
                key(egui::Key::Backspace, egui::Modifiers::NONE, false),
            ],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
    }
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .shirt_name
            .len(),
        18,
        "the shortening landed"
    );

    // And lengthening past the current 18 is refused: two more letters do
    // not land.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some("S".repeat(18).as_str())
        })
        .expect("the Shirt field");
    click(&mut window, shirt, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    // The keystrokes land before the refusal: the 20 letters draw once.
    let (_, nodes, _) = window.frame(vec![egui::Event::Text("SS".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::TextInput && node.value() == Some("S".repeat(20).as_str())
        }),
        "the typed letters were drawn"
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .shirt_name
            .len(),
        18,
        "20 chars over the current stored are refused"
    );
}

// The portable text limits on PES 16: the field's own 16 letters bind, so a 17th
// is refused.
#[test]
fn a_pes16_strip_refuses_a_17th_shirt_letter() {
    let path = fixture(PesVersion::Pes16, "shirt_17");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = shirt_field(&nodes);
    let stored = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .player(70101)
        .unwrap()
        .shirt_name
        .clone();
    click(&mut window, shirt, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    // The keystrokes land before the refusal: the field draws the typed
    // text on the Text frame, then reverts.
    let expected = format!("{stored}{}", "S".repeat(17));
    let (_, nodes, _) = window.frame(vec![egui::Event::Text("S".repeat(17).to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::TextInput && node.value() == Some(expected.as_str())
        }),
        "the typed 17 letters were drawn"
    );
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "a 17th letter is refused on PES 16"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::TextInput && node.value() == Some(stored.as_str())
        }),
        "the field reverted"
    );
}

// The Shirt field's hover says the limit the field enforces — the smaller
// of the portable 18 and the version's own field — so PES 21's 61-wide
// field still says 18.
#[test]
fn the_shirt_hover_names_the_limit_the_field_enforces() {
    let path = fixture(PesVersion::Pes21, "shirt_hover_limit");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = shirt_field(&nodes);
    let tooltip = tooltip_containing(&mut window, &mut view, centre(shirt), "Up to");
    assert_eq!(
        tooltip,
        Some(
            "Up to 18 letters. Western European accents (é, ü, ñ) are fine, but other letters \
                 and symbols are not."
                .to_string()
        ),
        "PES 21's shirt field is 61 wide but the editor holds 18"
    );
}

// The Shirt field's hover on PES 16: its own 16-letter field binds under the
// portable 18, and the hover says so.
#[test]
fn the_pes16_shirt_hover_names_its_own_field() {
    let path = fixture(PesVersion::Pes16, "shirt_hover_16");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let shirt = shirt_field(&nodes);
    let tooltip = tooltip_containing(&mut window, &mut view, centre(shirt), "Up to");
    assert_eq!(
        tooltip,
        Some(
            "Up to 16 letters. Western European accents (é, ü, ñ) are fine, but other letters \
                 and symbols are not."
                .to_string()
        ),
        "PES 16's own 16-letter field binds under the portable 18"
    );
}

// The Name field's hover on a name the game shows whole: the version's
// byte limit, what each kind of letter costs and how a colour code is
// typed, the only place the editor says so.
#[test]
fn the_name_hover_names_the_versions_byte_limit_and_the_colour_code() {
    for (version, limit) in [(PesVersion::Pes21, 61), (PesVersion::Pes17, 46)] {
        let path = fixture(version, &format!("name_hover_{version:?}"));
        let mut view = SaveView::new();
        view.open(&path);
        let mut window = Headless::new();
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let name = name_field(&nodes);
        let tooltip = tooltip_containing(&mut window, &mut view, centre(name), "Up to");
        assert_eq!(
            tooltip,
            Some(format!(
                "Up to {limit} bytes. A plain letter takes one byte, an accented letter takes two \
                 or more, and a colour code takes ten. A colour code is typed as \\x11c and eight \
                 hex digits."
            )),
            "{version:?}'s Name hover"
        );
    }
}

// SE-GUI-38
#[test]
fn the_name_and_shirt_counts_follow_the_fields_and_the_portable_limits() {
    /// The painted colour of `text`'s counted-label run `run`
    /// (`"Name (n/max)"`/`"Shirt (n/max)"`: 0 the label, 1 the `(n/max)`
    /// count): the job's format at the run's byte (adjacent same-format
    /// runs merge in one galley section, so the run is a byte range, not
    /// a section).
    fn run_color(shapes: &[egui::epaint::ClippedShape], text: &str, run: usize) -> egui::Color32 {
        let job = &title_shape(shapes, text).galley.job;
        let byte = if run == 0 {
            egui::text::ByteIndex(0)
        } else {
            egui::text::ByteIndex(job.text.len() - 1)
        };
        job.format_at_byte(byte).color
    }

    // 70101: name a colour code then "AB" (10 + 2 = 12 bytes), shirt 20
    // letters — past the portable 18.
    let path = fixture(PesVersion::Pes21, "text_counts");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.name = "\x11cff0000ffAB".to_owned();
        player.shirt_name = "S".repeat(20);
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let tint = visuals
        .extreme_bg_color
        .lerp_to_gamma(visuals.warn_fg_color, 0.25);
    let shirt_name = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .shirt_name
            .clone()
    };

    // The labels read the counts, the colour code counting its ten bytes.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(label_value(&nodes, "Name (12/61)"), "the name's count");
    assert!(label_value(&nodes, "Shirt (20/18)"), "the shirt's count");
    let shirt = shirt_field(&nodes);
    assert_eq!(
        fill_at(&shapes, shirt.bounds().expect("bounds")),
        Some(tint),
        "the shirt field is tinted"
    );
    // The label reads the normal text colour; the count the disabled card
    // labels' grey under its limit, the error colour past it.
    let text_color = visuals.text_color();
    let grey = egui::Color32::from_rgb(0x5a, 0x5a, 0x5a);
    assert_eq!(
        run_color(&shapes, "Name (12/61)", 0),
        text_color,
        "the name's label run"
    );
    assert_eq!(
        run_color(&shapes, "Name (12/61)", 1),
        grey,
        "the name count under the limit is grey"
    );
    assert_eq!(
        run_color(&shapes, "Shirt (20/18)", 0),
        text_color,
        "the shirt's label run stays normal"
    );
    assert_eq!(
        run_color(&shapes, "Shirt (20/18)", 1),
        visuals.error_fg_color,
        "the shirt count past the limit is drawn in the error colour"
    );
    // The field's hover names the portable limit; the counted label
    // carries the same one, since the field and its count carry one
    // hover.
    let tooltip = tooltip_after_hover(&mut window, &mut view, centre(shirt));
    assert_eq!(
        tooltip,
        Some("PES 2019 and older show 18 letters of a shirt name (PES 2016 shows 16)".to_owned()),
        "the shirt hover"
    );
    let count = label_bounds(&nodes, "Shirt (20/18)");
    let label_tip = tooltip_after_hover(
        &mut window,
        &mut view,
        egui::pos2(
            ((count.x0 + count.x1) / 2.0) as f32,
            ((count.y0 + count.y1) / 2.0) as f32,
        ),
    );
    assert_eq!(label_tip, tooltip, "the count's hover is the field's");

    // A letter deleted: the count follows the field, still tinted.
    click(&mut window, shirt, &mut view);
    window.frame(
        vec![
            key(egui::Key::Backspace, egui::Modifiers::NONE, true),
            key(egui::Key::Backspace, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "Shirt (19/18)"),
        "the count follows the deleted letter"
    );
    assert_eq!(
        run_color(&shapes, "Shirt (19/18)", 0),
        text_color,
        "the label run stays normal"
    );
    assert_eq!(
        run_color(&shapes, "Shirt (19/18)", 1),
        visuals.error_fg_color,
        "still past the limit"
    );
    assert_eq!(
        fill_at(&shapes, shirt_field(&nodes).bounds().expect("bounds")),
        Some(tint),
        "still tinted"
    );

    // A letter typed after it is refused: the stored name keeps 19.
    let shirt = shirt_field(&nodes);
    click(&mut window, shirt, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
            egui::Event::Text("S".to_owned()),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        shirt_name(&view).len(),
        19,
        "the refused letter was not pushed"
    );

    // Another deleted: "(18/18)" on the limit, in the warning colour, the
    // field untinted; at "(17/18)" the count is grey.
    for (count, colour, what) in [
        ("Shirt (18/18)", visuals.warn_fg_color, "on the limit"),
        ("Shirt (17/18)", grey, "under the limit"),
    ] {
        let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        click(&mut window, shirt_field(&nodes), &mut view);
        window.frame(
            vec![
                key(egui::Key::End, egui::Modifiers::NONE, true),
                key(egui::Key::End, egui::Modifiers::NONE, false),
                key(egui::Key::Backspace, egui::Modifiers::NONE, true),
                key(egui::Key::Backspace, egui::Modifiers::NONE, false),
            ],
            |ui| show(ui, &mut view, &|| Vec::new()),
        );
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        assert!(label_value(&nodes, count), "the count reads {count}");
        assert_eq!(run_color(&shapes, count, 1), colour, "{count} {what}");
        assert_ne!(
            fill_at(&shapes, shirt_field(&nodes).bounds().expect("bounds")),
            Some(tint),
            "{count}: the field untinted"
        );
    }

    // 47 bytes as the name: "Name (47/61)", tinted, the crop hover.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = name_field(&nodes);
    click(&mut window, field, &mut view);
    type_text(&mut window, &mut view, &"Z".repeat(47));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(label_value(&nodes, "Name (47/61)"), "the name's new count");
    let field = name_field(&nodes);
    assert_eq!(
        fill_at(&shapes, field.bounds().expect("bounds")),
        Some(tint),
        "the name field is tinted"
    );
    assert_eq!(
        run_color(&shapes, "Name (47/61)", 0),
        text_color,
        "the name's label run stays normal"
    );
    assert_eq!(
        run_color(&shapes, "Name (47/61)", 1),
        visuals.error_fg_color,
        "past the portable 46 the name count is drawn in the error colour"
    );
    let tooltip = tooltip_after_hover(&mut window, &mut view, centre(field));
    assert_eq!(
        tooltip,
        Some(format!(
            "In PES 2019 and older the game cuts this name to «{}»",
            "Z".repeat(46)
        )),
        "the name's crop hover"
    );
    // The counted label carries the same hover.
    let count = label_bounds(&nodes, "Name (47/61)");
    let label_tip = tooltip_after_hover(
        &mut window,
        &mut view,
        egui::pos2(
            ((count.x0 + count.x1) / 2.0) as f32,
            ((count.y0 + count.y1) / 2.0) as f32,
        ),
    );
    assert_eq!(label_tip, tooltip, "the count's hover is the field's");

    // An accented letter counts its units: É is two UTF-8 bytes in the
    // Name's byte count and one letter in the Shirt's letter count.
    let path = fixture(PesVersion::Pes21, "text_counts_accent");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    {
        let player = file.player_mut(70101).unwrap();
        player.name = "ABÉ".to_owned();
        player.shirt_name = "SÉ".to_owned();
    }
    file.save(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, "Name (4/61)"),
        "É is two bytes in the name's count"
    );
    assert!(
        label_value(&nodes, "Shirt (2/18)"),
        "É is one letter in the shirt's count"
    );

    // PES 16: the limits are its own field's, 46 bytes and 16 letters.
    let path = fixture(PesVersion::Pes16, "text_counts_16");
    let file16 = EditFile::load(&path).unwrap();
    let first = file16.team(701).unwrap().roster[0].player_id;
    let player16 = file16.player(first).unwrap();
    let (name_n, shirt_n) = (player16.name.len(), player16.shirt_name.chars().count());
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(&nodes, &format!("Name ({name_n}/46)")),
        "PES 16's name limit is 46"
    );
    assert!(
        label_value(&nodes, &format!("Shirt ({shirt_n}/16)")),
        "PES 16's shirt limit is 16"
    );
}

// The boundary the hover names: the field's charset is one byte per
// letter, so é (U+00E9, inside) lands and ł (U+0142, past it) is refused.
#[test]
fn a_shirt_name_takes_latin1_letters_and_refuses_past_them() {
    /// The Shirt field's node (`shirt_field`; the name and shirt hold the
    /// same text on this fixture, so the label's cell, not the value,
    /// names it).
    fn shirt(nodes: &[accesskit::Node]) -> &accesskit::Node {
        shirt_field(nodes)
    }

    let path = fixture(PesVersion::Pes21, "shirt_latin1");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let shirt_name = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .shirt_name
            .clone()
    };

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let stored = shirt(&nodes).value().expect("its text").to_owned();
    click(&mut window, shirt(&nodes), &mut view);
    let type_at_end = |window: &mut Headless, view: &mut SaveView, text: &str| {
        window.frame(
            vec![
                key(egui::Key::End, egui::Modifiers::NONE, true),
                key(egui::Key::End, egui::Modifiers::NONE, false),
            ],
            |ui| show(ui, view, &|| Vec::new()),
        );
        let (_, nodes, _) = window.frame(vec![egui::Event::Text(text.to_owned())], |ui| {
            show(ui, view, &|| Vec::new())
        });
        nodes
    };

    // ł is past U+00FF: the letter draws once, then the field reverts and
    // nothing is stored.
    let nodes = type_at_end(&mut window, &mut view, "Ł");
    let typed = format!("{stored}Ł");
    assert_eq!(
        shirt(&nodes).value(),
        Some(typed.as_str()),
        "the typed ł was drawn before the revert"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(
        shirt(&nodes).value(),
        Some(stored.as_str()),
        "the field reverted to the stored shirt"
    );
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "the refused letter was not pushed"
    );

    // é is inside it: the same edit lands.
    type_at_end(&mut window, &mut view, "É");
    assert_eq!(shirt_name(&view), format!("{stored}É"), "the é landed");
}

// The portable limit's boundary: a shirt name of exactly the portable 18
// letters is no warning — the tint is for what a version would cut, and
// nothing is.
#[test]
fn a_shirt_at_the_portable_limit_draws_no_tint() {
    let path = fixture(PesVersion::Pes21, "shirt_at_limit");
    let mut file = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
    file.player_mut(70101).unwrap().shirt_name = "S".repeat(18);
    file.save(&path).unwrap();

    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let tint = visuals
        .extreme_bg_color
        .lerp_to_gamma(visuals.warn_fg_color, 0.25);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let field = shirt_field(&nodes);
    assert_eq!(
        field.value(),
        Some("S".repeat(18).as_str()),
        "the Shirt field holds the 18 letters"
    );
    let bounds = field.bounds().expect("bounds");
    assert_ne!(
        fill_at(&shapes, bounds),
        Some(tint),
        "an 18-letter shirt draws no tint"
    );
}

// The Captain box ticks on the captain's roster slot and not on another's.
#[test]
fn the_captain_box_ticks_on_the_captains_slot() {
    let path = fixture(PesVersion::Pes21, "captain_box");
    let file = EditFile::load(&path).unwrap();
    let captain = file.team(701).unwrap().tactics.set_pieces.captain;
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    let mut window = Headless::new();

    // The Captain radio is the strip's unlabeled radio; the view's skill
    // boxes carry their names. egui 0.37 leaves an empty label unset in the
    // AccessKit tree rather than writing `""`, so the node carries `None`.
    let toggled = |nodes: &[accesskit::Node]| {
        nodes
            .iter()
            .find(|node| node.role() == Role::RadioButton && node.label().is_none())
            .map(|node| node.toggled())
    };
    // Select the captain's slot: the radio is on.
    view.selected = Some(Selection::Player(701, captain));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(toggled(&nodes), Some(Some(accesskit::Toggled::True)));
    // Another slot: off.
    let other = if captain == 0 { 1 } else { 0 };
    view.selected = Some(Selection::Player(701, other));
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert_eq!(toggled(&nodes), Some(Some(accesskit::Toggled::False)));
}

// Captain is a radio that makes this slot the team's captain and is not
// unticked by a second click (the captain belongs to the slot).
#[test]
fn the_captains_radio_cannot_be_unticked() {
    let path = fixture(PesVersion::Pes21, "captain_radio");
    let file = EditFile::load(&path).unwrap();
    let captain = file.team(701).unwrap().tactics.set_pieces.captain;
    let mut view = SaveView::new();
    view.open(&path);
    view.team = TeamChoice::Team(701);
    view.selected = Some(Selection::Player(701, captain));
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let radio = nodes
        .iter()
        .find(|node| node.role() == Role::RadioButton && node.label().is_none())
        .expect("the Captain radio");

    // A second click on the captain's radio: still his slot, no undo entry.
    click(&mut window, radio, &mut view);
    let session = view.session.as_ref().unwrap();
    assert_eq!(
        session.file().team(701).unwrap().tactics.set_pieces.captain,
        captain,
        "the captain stays"
    );
    assert!(!session.can_undo(), "the second click pushed nothing");
}

// A typed value past a field's stored width is clamped to it, the codec's own
// bound; a value in range lands as typed.
#[test]
fn a_typed_number_clamps_to_the_fields_stored_width() {
    let path = fixture(PesVersion::Pes21, "clamped_number");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    // Age's maximum is the schema's own, read from the save's version, not a
    // literal.
    let schema = pes_savefile::schema::schema_for(PesVersion::Pes21);
    let age_max = schema
        .player
        .fields
        .iter()
        .find(|spec| spec.field == PlayerField::Age)
        .map(|spec| (1u64 << spec.bit_width) - 1)
        .expect("the schema stores Age");

    // Type 99 into Age (stored 20): the field clamps it.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let age = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some("20"))
        .expect("the Age control");
    click(&mut window, age, &mut view);
    window.frame(vec![egui::Event::Text("99".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .basic
            .age,
        age_max as u8,
        "99 clamps to the field's width"
    );

    // Type 23 into Shirt number (in range): it lands as typed.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let number = right_of_label(&nodes, "Shirt number", Role::SpinButton)
        .first()
        .copied()
        .expect("the Shirt number control");
    click(&mut window, number, &mut view);
    window.frame(vec![egui::Event::Text("23".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .team(701)
            .unwrap()
            .roster[0]
            .number,
        23
    );
}

// The save's own Name field bounds an edit (a PES 21 save takes 47 to 61
// bytes, a name past 46 tinted with the cut the older versions make).
#[test]
fn a_pes21_strip_takes_a_long_name_tinted_with_its_crop() {
    /// The Name field's `TextInput` node (the enabled text edit right of
    /// the "Name (n/max)" label).
    fn name_field(nodes: &[accesskit::Node], value: &str) -> accesskit::Rect {
        let field = field_right_of_label(nodes, "Name (", Role::TextInput);
        assert!(
            !field.is_disabled() && field.value() == Some(value),
            "the Name field {value:?}"
        );
        field.bounds().expect("bounds")
    }

    let path = fixture(PesVersion::Pes21, "long_name_taken");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let tint = |v: &egui::style::Visuals| v.extreme_bg_color.lerp_to_gamma(v.warn_fg_color, 0.25);

    // 70101's stored name fits: no tint, no crop hover.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = name_field(&nodes, "PLACEHOLDER");
    assert_ne!(
        fill_at(&shapes, bounds),
        Some(tint(&visuals)),
        "a fitting name draws no tint"
    );

    // Type a 52-byte name: taken (inside PES 21's 61), as one undo entry,
    // the field tinted with the 46-byte crop on hover.
    let name_bounds = name_field(&nodes, "PLACEHOLDER");
    window.frame(
        pointer_at(
            egui::pos2(
                ((name_bounds.x0 + name_bounds.x1) / 2.0) as f32,
                ((name_bounds.y0 + name_bounds.y1) / 2.0) as f32,
            ),
            true,
        ),
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(
        pointer_at(
            egui::pos2(
                ((name_bounds.x0 + name_bounds.x1) / 2.0) as f32,
                ((name_bounds.y0 + name_bounds.y1) / 2.0) as f32,
            ),
            false,
        ),
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![egui::Event::Text("Z".repeat(41).to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .name,
        format!("PLACEHOLDER{}", "Z".repeat(41)),
        "the 52-byte name was taken"
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = name_field(&nodes, &format!("PLACEHOLDER{}", "Z".repeat(41)));
    assert_eq!(
        fill_at(&shapes, bounds),
        Some(tint(&visuals)),
        "the overlong name is tinted"
    );
    // The hover shows the cut: «46 N's». Hover the tinted field.
    let tooltip = tooltip_after_hover(
        &mut window,
        &mut view,
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        ),
    );
    assert_eq!(
        tooltip,
        Some(format!(
            "In PES 2019 and older the game cuts this name to «{}{}»",
            "PLACEHOLDER",
            "Z".repeat(35)
        )),
        "the crop hover"
    );
    // Escape defocuses the field before Ctrl+Z (a focused text field's own
    // undo answers first).
    window.frame(
        vec![
            key(egui::Key::Escape, egui::Modifiers::NONE, true),
            key(egui::Key::Escape, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        undo_entries(&mut window, &mut view),
        1,
        "the name edit is one entry"
    );

    // Exactly the portable 46 bytes ("PLACEHOLDER" + 35): no tint, no crop
    // hover — the tint is a warning for what would be cut, and nothing is.
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = name_field(&nodes, "PLACEHOLDER");
    let pos = egui::pos2(
        ((bounds.x0 + bounds.x1) / 2.0) as f32,
        ((bounds.y0 + bounds.y1) / 2.0) as f32,
    );
    window.frame(pointer_at(pos, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(pointer_at(pos, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    window.frame(vec![egui::Event::Text("Z".repeat(35).to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let name46 = format!("PLACEHOLDER{}", "Z".repeat(35));
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let bounds = name_field(&nodes, &name46);
    assert_ne!(
        fill_at(&shapes, bounds),
        Some(tint(&visuals)),
        "a name at the portable limit draws no tint"
    );
    assert_eq!(
        tooltip_after_hover(
            &mut window,
            &mut view,
            egui::pos2(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            )
        ),
        None,
        "a name at the portable limit has no crop hover"
    );
}

// The same rule on PES 17: its own field holds 46 bytes, so a 47-byte name
// is refused (the field reverts and nothing is pushed).
#[test]
fn a_pes17_strip_refuses_a_name_past_its_own_field() {
    let path = fixture(PesVersion::Pes17, "overlong_name_17");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::new();
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let file = EditFile::load(&path).unwrap();
    // The field draws the raw name with `\x11` spelled out (the strip's
    // `escName`): match it as drawn.
    let stored = file
        .player(file.team(701).unwrap().roster[0].player_id)
        .unwrap()
        .name
        .replace('\x11', "\\x11");
    let name_field = nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextInput
                && !node.is_disabled()
                && node.value() == Some(stored.as_str())
        })
        .expect("the Name field");
    click(&mut window, name_field, &mut view);
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            key(egui::Key::End, egui::Modifiers::NONE, false),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    // 47 bytes past this version's own 46-byte field: the keystrokes land
    // and draw before the refusal reverts them.
    let expected = format!("{stored}{}", "Z".repeat(47));
    let (_, nodes, _) = window.frame(vec![egui::Event::Text("Z".repeat(47).to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::TextInput && node.value() == Some(expected.as_str())
        }),
        "the typed 47 bytes were drawn"
    );
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        nodes.iter().any(|node| {
            node.role() == Role::TextInput && node.value() == Some(stored.as_str())
        }),
        "the field reverted to the stored name"
    );
    assert!(
        !view.session.as_ref().unwrap().is_dirty(),
        "the refused edit was not pushed"
    );
}

/// Whether `shapes` paints `text` at the font size `size` (the header's
/// title, at the body's size).
fn title_painted(shapes: &[egui::epaint::ClippedShape], text: &str, size: f32) -> bool {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
            _ => None,
        })
        .any(|shape| {
            shape
                .galley
                .job
                .sections
                .first()
                .is_some_and(|section| (section.format.font_id.size - size).abs() < 0.5)
        })
}

/// The painted `TextShape` whose galley text is `text` (the name's runs
/// merged into one).
fn title_shape<'a>(
    shapes: &'a [egui::epaint::ClippedShape],
    text: &str,
) -> &'a egui::epaint::TextShape {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == text => Some(shape),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| panic!("no painted text {text:?}"))
}

// A count under its limit in the dark theme's grey (`#5a5a5a`) and in its
// light twin (`#9f9f9f`) on the light theme: General info's Name count for
// 70101, the PES 21 fixture's first player.
#[test]
fn a_count_under_its_limit_takes_each_themes_grey() {
    let path = fixture(PesVersion::Pes21, "count_theme");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    for (theme, grey) in [
        (egui::Theme::Dark, egui::Color32::from_gray(0x5a)),
        (egui::Theme::Light, egui::Color32::from_gray(0x9f)),
    ] {
        window.ctx.set_theme(theme);
        draw(&mut window, &mut view);
        let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let job = &text_shape(&shapes, "Name (").galley.job;
        let (shown, limit) = job.text["Name (".len()..job.text.len() - 1]
            .split_once('/')
            .expect("the count reads n/max");
        assert!(
            shown.parse::<usize>().expect("n") < limit.parse::<usize>().expect("max"),
            "premise: the name's count {} is under its limit",
            job.text
        );
        let count = job
            .format_at_byte(egui::text::ByteIndex(job.text.len() - 1))
            .color;
        assert_eq!(count, grey, "the Name count on the {theme:?} theme");
    }
}

// The player header, and no view drawing the tool's name: `<id> <name>`
// as the player view's title over the
// strip's nine labels — on the player view alone, the team view editing no
// player. The name's colour codes paint as the player list draws them: a
// coded run in the code's colour, the rest the strong text colour, the id
// the weak one, all at the body's size.
// SE-GUI-01
#[test]
fn the_player_header_tops_the_player_view_alone() {
    let (mut view, mut window) = open_734_with("player_header", &[], 0);
    // A name with a colour code and a reset (the `\x11c` + 8 hex digits the
    // player list renders, the `append_name` test's name): the title's runs
    // read the code's colour then the default one.
    let mut entry = player_entry(&view, 73401);
    entry.name = "WHEN THE \x11cb7bec5ffWORK RESULT\x11d WAS GOOD".to_owned();
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let title = "73401 WHEN THE WORK RESULT WAS GOOD";
    let (normal, weak) = {
        let visuals = &window.ctx.style_of(window.ctx.theme()).visuals;
        (visuals.strong_text_color(), visuals.weak_text_color())
    };
    let expected: Vec<(egui::Color32, &str)> = vec![
        (weak, "73401 "),
        (normal, "WHEN THE "),
        (egui::Color32::from_rgb(0xb7, 0xbe, 0xc5), "WORK RESULT"),
        (normal, " WAS GOOD"),
    ];
    let body = window.ctx.style_of(window.ctx.theme()).text_styles[&egui::TextStyle::Body].size;
    // The player view, with the Appearance section closed then open: the
    // header tops both arrangements.
    for appearance_open in [false, true] {
        if appearance_open {
            open_appearance(&mut window, &mut view);
        }
        let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
        let shape = title_shape(&shapes, title);
        let painted: Vec<(egui::Color32, String)> = shape
            .galley
            .job
            .sections
            .iter()
            .map(|section| {
                (
                    section.format.color,
                    shape.galley.job.text[section.byte_range.start.0..section.byte_range.end.0]
                        .to_owned(),
                )
            })
            .collect();
        assert_eq!(
            painted,
            expected
                .iter()
                .map(|(color, text)| (*color, text.to_string()))
                .collect::<Vec<_>>(),
            "the title's painted runs (appearance open: {appearance_open})"
        );
        assert!(
            title_painted(&shapes, title, body),
            "the name at the body's size"
        );
        // The panels' titles and the player fields' labels — the counted
        // labels carry their counts, matched by prefix.
        for label in [
            "General info",
            "Other stats",
            "Playable positions",
            "Skills",
            "Tricks",
            "COM playing styles",
            "Ability stats",
            "Shirt number",
            "Captain",
            "Nation",
            "Age",
            "Height",
            "Weight",
            "Stronger foot",
            "Stronger hand",
            "Registered position",
            "Playing style",
            "Injury Resistance",
            "Form",
        ] {
            assert!(label_value(&nodes, label), "{label} drawn");
        }
        for prefix in ["Name (", "Shirt ("] {
            assert!(
                nodes.iter().any(|node| {
                    node.role() == Role::Label
                        && node.value().is_some_and(|v| v.starts_with(prefix))
                }),
                "the {prefix}counted label drawn"
            );
        }
        assert!(
            !label_value(&nodes, "ID"),
            "the id is the title's, not a field's"
        );
    }
    // The team view draws neither the title nor the player's fields.
    open_team_view(&mut window, &mut view, 734);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !title_painted(&shapes, title, body),
        "no title on the team view"
    );
    // ("Captain" is left out: the Player assignments' own label for its
    // role chip reads so too.)
    for label in ["Nation", "Age", "Height", "Weight"] {
        assert!(!label_value(&nodes, label), "{label} not on the team view");
    }
}

// A field's label in the General info panels and in Ability stats is
// drawn in the normal text colour, as the checkboxes' labels are: every
// field label the panels name — the pairs', the combos', the counted
// labels', Ability stats' — and the team view's Team and Short paint the
// normal colour, not the weak one, which read too dark beside them.
#[test]
fn the_field_labels_paint_the_normal_text_colour() {
    let path = fixture(PesVersion::Pes21, "label_color");
    let mut view = SaveView::new();
    view.open(&path);
    view.selected = Some(Selection::Player(701, 0));
    // The columns' full width so Ability stats' labels all draw inside
    // the clip (a wrapped third column puts the last of them past it).
    let mut window = Headless::sized(DEFAULT_PANE.x, DEFAULT_PANE.y);
    let visuals = window.ctx.style_of(window.ctx.theme()).visuals.clone();
    let text_color = visuals.text_color();
    assert_ne!(
        text_color,
        visuals.weak_text_color(),
        "premise: the two colours differ"
    );
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    for label in [
        "Height",
        "Weight",
        "Age",
        "Shirt number",
        "Nation",
        "Captain",
        "Stronger foot",
        "Stronger hand",
        "Registered position",
        "Playing style",
        "Injury Resistance",
        "Weak Foot: Usage",
        "Form",
        "Accuracy",
    ] {
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            text_color,
            "{label} in the normal text colour"
        );
    }
    // The counted labels' first run too ("Name "/"Shirt " before the count).
    for prefix in ["Name (", "Shirt ("] {
        let value = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label && node.value().is_some_and(|v| v.starts_with(prefix))
            })
            .and_then(|node| node.value())
            .unwrap_or_else(|| panic!("the {prefix}counted label"))
            .to_owned();
        assert_eq!(
            painted_label_color(&nodes, &shapes, &value),
            text_color,
            "the {prefix}counted label in the normal text colour"
        );
    }
    // Ability stats' labels, the game's order.
    for name in pes_savefile::interchange::team_toml::labels::ABILITY_STATS
        .iter()
        .map(|label| pes_savefile::interchange::team_toml::labels::title_case(label))
    {
        assert_eq!(
            painted_label_color(&nodes, &shapes, &name),
            text_color,
            "{name} in the normal text colour"
        );
    }

    // The team view's field labels.
    open_team_view(&mut window, &mut view, 701);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    for label in ["Name", "Short"] {
        assert_eq!(
            painted_label_color(&nodes, &shapes, label),
            text_color,
            "the team view's {label} label in the normal text colour"
        );
    }
}
