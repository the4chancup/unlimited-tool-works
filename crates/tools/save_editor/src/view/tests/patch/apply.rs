use super::*;
use crate::test_support::{journal_lines, zip_entry};

// SE-GUI-18
#[test]
fn apply_is_one_undoable_edit_and_the_list_gives_way_to_the_applied_line() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_apply");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let before = player_entry(&view, CO_SLOT_3);
    let expected = patched_entry(&view, &text, CO_SLOT_3);
    assert_ne!(before, expected, "premise: the patch changes 71403");
    previewed(&mut window, &mut view, &text);

    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    click(&mut window, &apply, &mut view);
    assert_eq!(player_entry(&view, CO_SLOT_3), expected, "the lib's write");
    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "Applied. The changes stay unsaved until Save.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing left to apply");

    // Ctrl+Z until the stack is empty: one press, the Apply's.
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo entry");
    assert_eq!(player_entry(&view, CO_SLOT_3), before, "71403 as before");
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(lines[3..], table(&CO_ROWS));
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(!apply.is_disabled(), "Apply is enabled again");
}

// SE-GUI-18
#[test]
fn the_preview_follows_the_save_and_an_applied_patch_lists_nothing() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_live");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let patched = patched_entry(&view, &text, CO_SLOT_3);
    previewed(&mut window, &mut view, &text);
    assert!(
        panel_lines(&mut window, &mut view).contains(&CO_ROWS[0][0].to_owned()),
        "premise: the corner kick row is listed"
    );

    // The corner kick set to the patch's value, as an edit would.
    let mut entry = player_entry(&view, CO_SLOT_3);
    assert_ne!(
        entry.motion.corner_kick, patched.motion.corner_kick,
        "premise: the patch changes the corner kick"
    );
    entry.motion.corner_kick = patched.motion.corner_kick;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    let lines = panel_lines(&mut window, &mut view);
    assert_eq!(lines[3..], table(&CO_ROWS[1..]));

    // Every value of the patch already held: nothing to list or apply.
    view.session.as_mut().unwrap().edit_players(vec![patched]);
    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "The save already holds every value this patch writes.",
        ]
    );
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    assert!(apply.is_disabled(), "nothing to apply");
}

// SE-GUI-18
#[test]
fn an_edit_after_apply_takes_the_applied_line_away() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_applied_edit");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    previewed(&mut window, &mut view, &text);
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    click(&mut window, &apply, &mut view);
    assert_eq!(
        panel_lines(&mut window, &mut view)[3..],
        ["Applied. The changes stay unsaved until Save."],
        "premise: the Applied line stands"
    );

    // An edit of a player the patch does not write.
    let mut entry = player_entry(&view, F_SLOT_1);
    entry.basic.height = if entry.basic.height == 180 { 181 } else { 180 };
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "The save already holds every value this patch writes.",
        ]
    );
}

#[test]
fn a_patch_opened_after_an_apply_says_the_save_holds_it() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_reopened_after_apply");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let applied = patched_entry(&view, &text, CO_SLOT_3);
    let path = previewed(&mut window, &mut view, &text);
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    click(&mut window, &apply, &mut view);
    assert_eq!(
        panel_lines(&mut window, &mut view)[3..],
        ["Applied. The changes stay unsaved until Save."],
        "premise: the Applied line stands"
    );
    let before = revision(&view);

    // Patch B: the look patch A wrote, which the save now holds.
    let same = team(
        CO,
        "/co/",
        BTreeMap::from([(
            3,
            PatchPlayer {
                appearance: Some(look_of(&applied, PesVersion::Pes21, &[])),
                ..bare()
            },
        )]),
    );
    std::fs::write(
        &path,
        patch_text(PesVersion::Pes21, ALLOCATION_SCHEME_VERSION, vec![same]),
    )
    .unwrap();
    open_patch(&mut window, &mut view, path);
    assert_eq!(revision(&view), before, "premise: the save is unchanged");

    assert_eq!(
        panel_lines(&mut window, &mut view),
        [
            "Open patch\u{2026}",
            "patch.toml: team 714 /co/, players 03",
            "Apply",
            "The save already holds every value this patch writes.",
        ]
    );
}

// SE-GUI-18
#[test]
fn apply_recomputes_a_preview_held_through_a_gesture() {
    let (mut view, mut window) = open_save(PesVersion::Pes21, "patch_held");
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let patched = patched_entry(&view, &text, CO_SLOT_3);
    previewed(&mut window, &mut view, &text);
    // The preview computed on a settled frame first: a patch just opened
    // has none to hold.
    assert_eq!(
        panel_lines(&mut window, &mut view)[3..],
        table(&CO_ROWS),
        "premise: the preview is computed"
    );

    // The pointer held on the toolbar's empty end: the edit below stays in
    // flight until the release.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    // 71403's corner kick set to the patch's value and his height, which
    // the patch does not write, changed.
    let mut entry = player_entry(&view, CO_SLOT_3);
    assert_ne!(
        entry.motion.corner_kick, patched.motion.corner_kick,
        "premise: the patch changes the corner kick"
    );
    entry.motion.corner_kick = patched.motion.corner_kick;
    let height = if entry.basic.height == 180 { 181 } else { 180 };
    entry.basic.height = height;
    view.session.as_mut().unwrap().edit_players(vec![entry]);
    assert_eq!(
        panel_lines(&mut window, &mut view)[3..],
        table(&CO_ROWS),
        "the held preview stays drawn while the pointer is down"
    );

    let expected = patched_entry(&view, &text, CO_SLOT_3);
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (apply, _) = pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some("Apply"))
        .expect("the Apply button");
    action_click(&mut window, &mut view, *apply);
    let written = player_entry(&view, CO_SLOT_3);
    assert_eq!(written.basic.height, height, "the edit survives");
    assert_eq!(written, expected, "the patch's values land on the edit");
}

// The Apply's journal: the pick named with its bytes kept at the pick,
// and the apply beside the field lines `edit_players` records.
#[test]
fn an_applied_patch_is_named_and_kept_in_the_journal() {
    let path = fixture(PesVersion::Pes21, "patch_apply_journal");
    let dir = path.parent().unwrap().to_path_buf();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();
    view.open(&path);
    let mut window = Headless::sized(DEFAULT_PANE.x, 800.0);
    let text = patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![co_block(&view)],
    );
    let patch_path = previewed(&mut window, &mut view, &text);
    let apply = panel_button(&mut window, &mut view, "Apply").expect("the Apply button");
    click(&mut window, &apply, &mut view);

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.contains(&format!("picked {}", patch_path.display())),
        "the pick is named: {lines:?}"
    );
    assert!(
        lines.contains(&format!(
            "applied the aesthetics patch {}",
            patch_path.display()
        )),
        "the apply names the file: {lines:?}"
    );
    assert_eq!(
        zip_entry(&dir.join("journal.zip"), "inputs/1-patch.toml"),
        text.as_bytes(),
        "the patch file's bytes are kept as an input"
    );
}
