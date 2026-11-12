use super::*;
use crate::test_support::{file_count, progress_events, scratch, test_ctx};

#[test]
fn an_input_ending_in_a_dot_or_dot_dot_resolves_to_its_folder() {
    let root = scratch("resolve_input");
    fs::create_dir_all(root.join("co Spring/Other")).unwrap();
    for input in [
        root.join("co Spring").join("."),
        root.join("co Spring").join("Other").join(".."),
    ] {
        let resolved = resolve_input(&input).unwrap();
        assert_eq!(source::source_name(&resolved), "co Spring", "{input:?}");
        // Compared by their real paths: a trailing `..` is resolved through the
        // file system on POSIX, and the scratch folder's own name need not be its
        // real path (a temp folder reached through a link).
        assert_eq!(
            resolved.parent().unwrap().canonicalize().unwrap(),
            root.canonicalize().unwrap(),
            "{input:?}"
        );
    }
}

#[test]
fn worst_tracks_the_worst_severity_reported() {
    let (ctx, _events_rx) = test_ctx();
    let mut events = RunEvents::new(&ctx, FIRST_RUN);
    assert_eq!(events.worst(), None);
    events.message(tool_message(
        Code::MigrationClean,
        Scope::Run,
        Disposition::Keep,
        vec![],
    ));
    assert_eq!(events.worst(), Some(Severity::Info));
    events.message(unmigrated(
        Scope::Run,
        Disposition::DropFile,
        UnmigratedReason::Unclaimed,
        None,
        vec![],
    ));
    assert_eq!(events.worst(), Some(Severity::Error));
    events.message(tool_message(
        Code::MigrationNotClean,
        Scope::Run,
        Disposition::Keep,
        vec![],
    ));
    assert_eq!(events.worst(), Some(Severity::Error));
}

#[test]
fn a_warning_finding_that_drops_something_is_not_clean() {
    // The verdict's "nothing dropped" half sees the tool's own findings
    // too: a finding below Error that drops a folder still refuses
    // `migration_clean`, since the compile would lose the folder.
    let (ctx, _events_rx) = test_ctx();
    let mut events = RunEvents::new(&ctx, FIRST_RUN);
    assert!(!events.dropped());
    events.message(tool_message(
        Code::RosterSlotEmpty,
        Scope::Folder {
            export_id: INPUT,
            path: ScopePath::new("Faces/XXX15 - A").unwrap(),
        },
        Disposition::DropFolder,
        vec![],
    ));
    assert_eq!(events.worst(), Some(Severity::Warning));
    assert!(events.dropped());
    assert_eq!(
        verdict(events.worst(), events.dropped()),
        (Code::MigrationNotClean, ERRORS)
    );
}

#[test]
fn the_verdict_is_clean_only_with_no_error_and_no_drop() {
    assert_eq!(verdict(None, false), (Code::MigrationClean, CLEAN));
    assert_eq!(
        verdict(Some(Severity::Warning), false),
        (Code::MigrationClean, CLEAN)
    );
    assert_eq!(
        verdict(Some(Severity::Warning), true),
        (Code::MigrationNotClean, ERRORS)
    );
    assert_eq!(
        verdict(Some(Severity::Error), false),
        (Code::MigrationNotClean, ERRORS)
    );
}

#[test]
fn a_team_color_entry_in_a_kit_spelling_keeps_its_first_color() {
    // Step 6: two parts that each read as one color; the first is kept.
    assert_eq!(
        first_of_two_colors("048 146 082 - 255 255 255"),
        Some("048 146 082")
    );
    assert_eq!(first_of_two_colors("#0070DD - #FFF468"), Some("#0070DD"));
}

#[test]
fn an_entry_that_is_not_two_colors_keeps_nothing() {
    // Three parts that each read as one color are no two-color entry,
    // leading bullet or not.
    assert_eq!(
        first_of_two_colors("048 146 082 - 255 255 255 - 000 000 000"),
        None
    );
    assert_eq!(
        first_of_two_colors("- 1st: 048 146 082 - 255 255 255 - 000 000 000"),
        None
    );
    // Nor is a part that reads as two colors (the second past the
    // capacity), one refused outright, or one that is blank.
    assert_eq!(
        first_of_two_colors("048 146 082\n255 255 255 - #FFFFFF"),
        None
    );
    assert_eq!(
        first_of_two_colors("048 146 082 255 255 255 - #FFFFFF"),
        None
    );
    assert_eq!(first_of_two_colors("048 146 082 -"), None);
    // A part that is no color at all.
    assert_eq!(first_of_two_colors("blue - 255 255 255"), None);
    // And one color alone is no entry in a kit entry's spelling.
    assert_eq!(first_of_two_colors("048 146 082"), None);
}

#[test]
fn a_model_file_in_any_letter_case_makes_a_loose_run_pre_fox() {
    let root = scratch("loose_model_case");
    fs::create_dir_all(root.join("co Spring/Boots/k0430 - Crocs")).unwrap();
    fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
    // A boots-named model: a valid name under pre-Fox, so the run is
    // clean and writes its output (an invalid one would leave none).
    fs::write(
        root.join("co Spring/Boots/k0430 - Crocs/boots.MODEL"),
        b"model",
    )
    .unwrap();
    let (ctx, _events_rx) = test_ctx();
    let args = UpgradeArgs {
        exports: vec![root.join("co Spring")],
        all: None,
        savefile: None,
        loose: true,
        legacy_ids: false,
        coverage: None,
        output: None,
    };

    run(&args, &ctx).unwrap();
    // On the pre-Fox engine a worn boots folder stays shared; Fox
    // merges it into its one wearer's folder.
    assert!(
        root.join("co Full Spring/Boots/Crocs/boots.MODEL")
            .is_file(),
        "{:?}",
        root
    );
}

#[test]
fn a_refs_txt_that_is_not_utf8_is_unmigrated_and_no_players_txt_is_written() {
    let root = scratch("refs_not_utf8");
    // Notepad's ANSI: an accented name is no UTF-8 (step 9's
    // `Some((path, None))`).
    fs::create_dir_all(root.join("refs Spring/Players/arbiter")).unwrap();
    fs::write(root.join("refs Spring/refs.txt"), b"01 M\xfcller\r\n").unwrap();
    fs::write(
        root.join("refs Spring/Players/arbiter/boots.fmdl"),
        b"boots",
    )
    .unwrap();
    let (ctx, events_rx) = test_ctx();
    let args = UpgradeArgs {
        exports: vec![root.join("refs Spring")],
        all: None,
        savefile: None,
        loose: false,
        legacy_ids: false,
        coverage: None,
        output: None,
    };

    let exit = run(&args, &ctx).unwrap();
    drop(ctx);
    assert_eq!(exit, ERRORS);
    let messages: Vec<Message> = events_rx
        .iter()
        .filter_map(|envelope| match envelope.event {
            PipelineEvent::Message(message) => Some(message),
            _ => None,
        })
        .collect();
    let unmigrated = messages
        .iter()
        .filter(|m| m.code.code.as_ref() == "unmigrated_content")
        .collect::<Vec<_>>();
    assert_eq!(
        unmigrated.iter().map(|m| &m.scope).collect::<Vec<_>>(),
        [&Scope::File {
            export_id: INPUT,
            path: path_scope("refs.txt")
        }],
        "{:?}",
        messages
            .iter()
            .map(|m| m.code.code.as_ref().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        unmigrated[0].context,
        [("reason".to_owned(), "roster_not_utf8".to_owned())]
    );
    // Not clean: nothing is written, the staging folder removed.
    assert!(!root.join("refs Spring upgraded").exists());
    assert!(!root.join("refs Spring upgraded.upgrading").exists());
}

#[test]
fn a_second_note_and_an_unclaimed_file_get_their_reasons() {
    let root = scratch("unclaimed_reasons");
    // `team note.txt` is a second Note-named file (`co Note.txt` keeps
    // the place); `Players/` holds no folder a team export's steps read.
    fs::create_dir_all(root.join("co Spring/Players/05 - A")).unwrap();
    fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
    fs::write(root.join("co Spring/team note.txt"), b"Team: /co/\r\n").unwrap();
    fs::write(root.join("co Spring/Players/05 - A/face.fmdl"), b"face").unwrap();
    let (ctx, events_rx) = test_ctx();
    let args = UpgradeArgs {
        exports: vec![root.join("co Spring")],
        all: None,
        savefile: None,
        loose: true,
        legacy_ids: false,
        coverage: None,
        output: None,
    };

    let exit = run(&args, &ctx).unwrap();
    drop(ctx);
    assert_eq!(exit, ERRORS);
    let findings: Vec<(String, Vec<(String, String)>)> = events_rx
        .iter()
        .filter_map(|envelope| match envelope.event {
            PipelineEvent::Message(message)
                if message.code.code.as_ref() == "unmigrated_content" =>
            {
                let path = match &message.scope {
                    Scope::File { path, .. } | Scope::Folder { path, .. } => {
                        path.as_str().to_owned()
                    }
                    Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => String::new(),
                };
                Some((path, message.context))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        findings,
        [
            (
                "Players/05 - A/face.fmdl".to_owned(),
                vec![("reason".to_owned(), "unclaimed".to_owned())]
            ),
            (
                "team note.txt".to_owned(),
                vec![
                    ("reason".to_owned(), "note_taken".to_owned()),
                    ("other".to_owned(), "co Note.txt".to_owned()),
                ]
            ),
        ]
    );
}

#[test]
fn a_disallowed_file_type_drops_its_folder_in_the_output_validation() {
    // The output's check runs with the strict file-type check on, as the
    // compiler's default has it: a disallowed file refuses the folder and
    // stops the run.
    let output = scratch("output_issues_strict");
    fs::create_dir_all(output.join("Players/03 - A")).unwrap();
    fs::write(output.join("Players/03 - A/readme.doc"), b"x").unwrap();
    let (ctx, _events_rx) = test_ctx();
    let mut events = RunEvents::new(&ctx, FIRST_RUN);
    let issues = output_issues(
        &output,
        "co Midcup",
        PesVersion::Pes21,
        |path| path.to_owned(),
        &mut events,
    );
    let issue = issues
        .iter()
        .find(|issue| issue.code == "file_type_disallowed")
        .expect("file_type_disallowed expected");
    assert_eq!(
        issue.disposition,
        aesthetics_export::Disposition::DropFolder
    );
    assert!(!issue.passed_through);
}

#[test]
fn a_missing_shared_link_target_drops_its_folder() {
    // And without pass-through: a link whose target is missing drops
    // what it names rather than keeping it.
    let output = scratch("output_issues_pass_through");
    fs::create_dir_all(output.join("Players/03 - A")).unwrap();
    fs::write(output.join("Players/03 - A/Crocs.boots"), b"").unwrap();
    let (ctx, _events_rx) = test_ctx();
    let mut events = RunEvents::new(&ctx, FIRST_RUN);
    let issues = output_issues(
        &output,
        "co Midcup",
        PesVersion::Pes21,
        |path| path.to_owned(),
        &mut events,
    );
    let issue = issues
        .iter()
        .find(|issue| issue.code == "link_target_missing")
        .expect("link_target_missing expected");
    assert_eq!(
        issue.disposition,
        aesthetics_export::Disposition::DropFolder
    );
    assert!(!issue.passed_through);
}

#[test]
fn the_input_is_processed_once_its_named_output_is_validated() {
    // One upgrade is one export: the output is only named for its findings'
    // lines, and the input's `processed` comes after the output's
    // validation, so the console's counter reads `0/1` then `1/1`.
    let root = scratch("upgrade_processed");
    fs::create_dir_all(root.join("co Spring/Faces/XXX05 - Bob/alt")).unwrap();
    fs::write(root.join("co Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
    // `alt/face.xml` is copied as it is, and the output's check refuses it:
    // the finding is the output's, and `processed` must follow it.
    fs::write(
        root.join("co Spring/Faces/XXX05 - Bob/alt/face.xml"),
        b"<config/>",
    )
    .unwrap();
    let save = root.join("EDIT");
    fs::write(&save, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
    let (ctx, events_rx) = test_ctx();
    let args = UpgradeArgs {
        exports: vec![root.join("co Spring")],
        all: None,
        savefile: Some(save),
        loose: false,
        legacy_ids: false,
        coverage: None,
        output: None,
    };

    run(&args, &ctx).unwrap();
    drop(ctx);

    let events: Vec<String> = events_rx
        .iter()
        .filter_map(|envelope| match envelope.event {
            PipelineEvent::ExportStarted { export_id, .. } => {
                Some(format!("started {}", export_id.0))
            }
            PipelineEvent::ExportNamed { export_id, .. } => Some(format!("named {}", export_id.0)),
            PipelineEvent::ExportProcessed { export_id } => {
                Some(format!("processed {}", export_id.0))
            }
            PipelineEvent::Message(message) if message.scope.export_id() == Some(OUTPUT) => {
                Some(format!("output {}", message.code.code))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        events,
        [
            format!("started {}", INPUT.0),
            format!("named {}", OUTPUT.0),
            "output file_type_disallowed".to_owned(),
            format!("processed {}", INPUT.0),
        ]
    );
}

#[test]
fn the_progress_starts_past_the_root_files_and_rises_by_one_to_the_output_s_file_count() {
    // A team export's Note gives `colors.txt` and `notes.txt`, a referee
    // export's roster `players.txt`: files written before the planned ones,
    // which the first `Progress` already counts as done.
    let root = scratch("upgrade_progress");
    fs::create_dir_all(root.join("co Spring/Faces/XXX05 - Bob")).unwrap();
    fs::write(
        root.join("co Spring/co Note.txt"),
        include_str!("../../tests/fixtures/co Spring/co Note.txt"),
    )
    .unwrap();
    fs::write(
        root.join("co Spring/Faces/XXX05 - Bob/portrait.dds"),
        b"portrait",
    )
    .unwrap();
    fs::create_dir_all(root.join("refs Spring/Players/arbiter")).unwrap();
    fs::write(root.join("refs Spring/refs.txt"), b"01 arbiter\r\n").unwrap();
    fs::write(
        root.join("refs Spring/Players/arbiter/boots.fmdl"),
        b"boots",
    )
    .unwrap();
    // A referee export takes no `--loose`.
    for (export, loose, root_files) in [
        ("co Spring", true, ["colors.txt", "notes.txt"].as_slice()),
        ("refs Spring", false, ["players.txt"].as_slice()),
    ] {
        let (ctx, events_rx) = test_ctx();
        let args = UpgradeArgs {
            exports: vec![root.join(export)],
            all: None,
            savefile: None,
            loose,
            legacy_ids: false,
            coverage: None,
            output: Some(root.join(format!("{export} out"))),
        };

        let exit = run(&args, &ctx).unwrap();
        drop(ctx);
        assert_eq!(exit, CLEAN, "{export}");
        // The output's own name carries the coverage tag the run guesses.
        let outputs = fs::read_dir(root.join(format!("{export} out")))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        let [out] = outputs.as_slice() else {
            panic!("{export}: one output folder: {outputs:?}")
        };
        for name in root_files {
            assert!(out.join(name).is_file(), "{export}: {name}");
        }
        let files = file_count(out);
        let progress = progress_events(&events_rx);
        assert!(
            progress.iter().all(|(_, total)| *total == files),
            "{export}: every Progress carries the output's file count: {progress:?}"
        );
        assert_eq!(
            progress.iter().map(|(done, _)| *done).collect::<Vec<_>>(),
            (root_files.len()..=files).collect::<Vec<_>>(),
            "{export}"
        );
    }
}

#[test]
fn an_unused_pairs_dds_is_dropped_and_the_other_copies_written() {
    // A `.dds`/`.ftex` pair no material names the stem of: the `.dds`
    // follows its `.ftex` as a leftover wherever the pair sat, and the
    // retain drops it alone — the player folder's and the kit's other
    // files reach the output.
    let root = scratch("upgrade_unused_pair");
    fs::create_dir_all(root.join("co Midcup Spring/Faces/XXX15 - A")).unwrap();
    fs::create_dir_all(root.join("co Midcup Spring/Kit Textures")).unwrap();
    fs::write(root.join("co Midcup Spring/co Note.txt"), b"Team: /co/\r\n").unwrap();
    for name in [
        "u0XXXp1.dds",
        "u0XXXp1.ftex",
        "portrait.dds",
        "ingame_face.txt",
    ] {
        fs::write(
            root.join("co Midcup Spring/Faces/XXX15 - A").join(name),
            name,
        )
        .unwrap();
    }
    fs::write(
        root.join("co Midcup Spring/Kit Textures/u0XXXp2.dds"),
        b"kit",
    )
    .unwrap();
    let (ctx, _events_rx) = test_ctx();
    let args = UpgradeArgs {
        exports: vec![root.join("co Midcup Spring")],
        all: None,
        savefile: None,
        loose: true,
        legacy_ids: false,
        coverage: None,
        output: None,
    };

    let exit = run(&args, &ctx).unwrap();
    drop(ctx);
    assert_eq!(exit, CLEAN);
    let out = root.join("co Midcup Spring upgraded");
    let player = out.join("Players/15 - A");
    assert!(player.join("portrait.dds").is_file(), "{player:?}");
    // The pair's respelled `kit1` files are both gone.
    assert!(!player.join("kit1.dds").exists(), "{player:?}");
    assert!(!player.join("kit1.ftex").exists(), "{player:?}");
    assert!(out.join("Kits/p2/kit.dds").is_file(), "{out:?}");
}
