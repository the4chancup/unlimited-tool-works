use super::*;

/// A view with `recorder`'s journal: a PES 21 save opened on `target`,
/// which `session.recorder()` writes to.
fn journal_view(test: &str, target: u32, recorder: &Recorder) -> (SaveView, Headless, PathBuf) {
    let path = fixture(PesVersion::Pes21, test);
    let dir = path.parent().unwrap().to_path_buf();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    view.team = TeamChoice::Team(target);
    view.selected = Some(Selection::Team(target));
    (view, Headless::sized(DEFAULT_PANE.x, 1400.0), dir)
}

// A file the member picks is a kept input named `picked`; a document the
// lib's `stage` refuses is a `refused` step once per pick, not once per
// refresh.
#[test]
fn a_picked_file_is_kept_named_and_refused_once() {
    let recorder = Recorder::default();
    let (mut view, mut window, dir) = journal_view("journal_picked", CO, &recorder);
    // Parses as a document but `stage` refuses it: a short name over
    // its field's three characters.
    let text = "[team]\nid = 714\nshort_name = \"TOOLONG\"\n";
    let path = picked(&mut window, &mut view, "wide.toml", text.as_bytes());
    // A second stage after a model change: the same refusal, which
    // records nothing new.
    let mut edited = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .player(70101)
        .unwrap()
        .clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    view.session.as_mut().unwrap().edit_player(1, edited);
    window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&format!("picked {}", path.display())),
        "the pick is named: {lines:?}"
    );
    let refused = steps_with(&lines, "refused ");
    assert_eq!(refused.len(), 1, "the refusal records once: {lines:?}");
    assert!(refused[0].contains("wide.toml"), "{}", refused[0]);
    assert_eq!(
        zip_entry(&dir.join("journal.zip"), "inputs/1-wide.toml"),
        text.as_bytes(),
        "the picked file's bytes are kept as an input"
    );
}

// A file that does not parse is a `could not read` step with its bytes
// kept, like a save that fails to open.
#[test]
fn a_pick_that_fails_to_parse_records_could_not_read_and_keeps_the_bytes() {
    let recorder = Recorder::default();
    let (mut view, mut window, dir) = journal_view("journal_pick_fails", CO, &recorder);
    picked(&mut window, &mut view, "notafile.toml", b"not a toml");

    let lines = journal_lines(&recorder, &dir);
    let refused = steps_with(&lines, "could not read ");
    assert_eq!(refused.len(), 1, "{lines:?}");
    assert!(
        refused[0].contains("notafile.toml"),
        "the step names the file that failed: {}",
        refused[0]
    );
    assert_eq!(
        zip_entry(&dir.join("journal.zip"), "inputs/1-notafile.toml"),
        b"not a toml",
        "the failed file's bytes are kept as an input"
    );
}

// A squad file, re-read per target rather than parsed at the pick, is a
// kept input named `picked` like a document's.
#[test]
fn a_squad_file_is_a_kept_pick_too() {
    let recorder = Recorder::default();
    let (mut view, mut window, dir) = journal_view("journal_squad_picked", CO, &recorder);
    let bytes = lib_fixture(SQUAD_19);
    let path = picked(&mut window, &mut view, SQUAD_19, &bytes);

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&format!("picked {}", path.display())),
        "the pick is named: {lines:?}"
    );
    assert_eq!(
        zip_entry(&dir.join("journal.zip"), "inputs/1-pes19_squad.4ccs"),
        bytes,
        "the squad file's bytes are kept as an input"
    );
}

// Apply names the file applied and its target beside the field lines
// `edit_entries` records.
#[test]
fn an_apply_names_the_file_and_target_in_the_journal() {
    let recorder = Recorder::default();
    let (mut view, mut window, dir) = journal_view("journal_apply", CO, &recorder);
    // `/cm/`'s file onto `/co/`'s team: a real change, so Apply is enabled.
    let text = export(&view, CM);
    let path = picked(&mut window, &mut view, "cm.toml", text.as_bytes());
    apply(&mut window, &mut view);

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&format!("applied {} onto team 714", path.display())),
        "the apply names the file and its target: {lines:?}"
    );
}

// A squad file `read_squad` refuses is a `refused` step once per pick
// too, not once per refresh — the same rule as a staged refusal's.
#[test]
fn a_squad_file_the_read_refuses_records_the_refusal_once() {
    let recorder = Recorder::default();
    // PES 19's save, whose records the PES 19 squad file's count matches.
    let path = fixture(PesVersion::Pes19, "journal_squad_refused");
    let dir = path.parent().unwrap().to_path_buf();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    view.team = TeamChoice::Team(THREE);
    view.selected = Some(Selection::Team(THREE));
    let mut window = Headless::sized(DEFAULT_PANE.x, 1400.0);

    // A slot emptied: the file's records have one fewer rostered player
    // to land on than they hold, which `read_squad` refuses.
    let bytes = lib_fixture(SQUAD_19);
    let mut team = team_entry(&view, THREE);
    let slot = team
        .roster
        .iter()
        .position(|slot| slot.player_id != 0)
        .expect("a filled slot");
    team.roster[slot].player_id = 0;
    assert!(
        read_squad(
            &bytes,
            &team,
            view.session.as_ref().unwrap().file().players()
        )
        .is_err(),
        "premise: the read refuses the file on the emptied slot"
    );
    view.session.as_mut().unwrap().edit_team(1, team);

    let path = picked(&mut window, &mut view, SQUAD_19, &bytes);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }
    assert!(
        panel_lines(&mut window, &mut view)
            .iter()
            .any(|line| line.starts_with(&format!("{SQUAD_19}:"))),
        "premise: the read was refused"
    );

    // A second stage after a model change: the same refusal, which
    // records nothing new.
    let mut edited = view
        .session
        .as_ref()
        .unwrap()
        .file()
        .players()
        .first()
        .expect("a player")
        .clone();
    edited.basic.height = edited.basic.height.wrapping_add(1);
    view.session.as_mut().unwrap().edit_player(2, edited);
    for _ in 0..3 {
        window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    }

    let lines = journal_lines(&recorder, &dir);
    let refused = steps_with(&lines, "refused ");
    assert_eq!(refused.len(), 1, "the refusal records once: {lines:?}");
    assert!(
        refused[0].contains(&path.display().to_string()),
        "the step names the picked file: {}",
        refused[0]
    );
}
