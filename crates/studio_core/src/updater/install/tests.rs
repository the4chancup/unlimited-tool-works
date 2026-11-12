use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use super::test_bundle::*;
use super::*;

/// Both archive kinds, with a name for the test folders and messages.
const KINDS: [(ArchiveKind, &str); 2] = [(ArchiveKind::SevenZ, "7z"), (ArchiveKind::TarGz, "tar")];

/// Installs `release`'s archive of `kind` into `bundle` through `open_url`,
/// the program's stamp taken now, nothing cancelled and the progress unread.
fn install_now(
    kind: ArchiveKind,
    release: &Release,
    bundle: &Path,
    open_url: &OpenUrl,
) -> Result<(), InstallError> {
    install_kind(
        kind,
        release,
        bundle,
        ProgramStamp::of(bundle),
        open_url,
        |_| {},
        &AtomicBool::new(false),
    )
}

/// `open_url` counting its requests in the returned counter.
fn counted(open_url: Box<OpenUrl>) -> (Box<OpenUrl>, Arc<AtomicUsize>) {
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let counting = move |url: &str| {
        counter.fetch_add(1, Ordering::SeqCst);
        open_url(url)
    };
    (Box::new(counting), requests)
}

#[test]
fn an_update_puts_the_new_bundle_in_place_and_keeps_data() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("happy_{label}"), &old_files());
        let (release, open_url) = bundle_release(kind, None);
        let mut reports = Vec::new();
        install_kind(
            kind,
            &release,
            &bundle,
            ProgramStamp::of(&bundle),
            &open_url,
            |progress| reports.push(progress),
            &AtomicBool::new(false),
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));

        let exe = exe_file_name();
        let total = release.assets[0].size;
        assert_eq!(
            reports.last(),
            Some(&Progress { done: total, total }),
            "{label}"
        );
        assert!(
            reports.len() >= 2,
            "{label}: progress between the two reads"
        );

        let expected: BTreeMap<String, Option<String>> = [
            (exe.clone(), Some("new program")),
            (format!("old/{exe}"), Some("old program")),
            ("old".to_owned(), None),
            ("README.txt".to_owned(), Some("new readme")),
            ("data".to_owned(), None),
            ("data/settings.toml".to_owned(), Some("pes_version = 21")),
            ("licenses".to_owned(), None),
            ("licenses/LICENSE-MIT".to_owned(), Some("new license")),
            ("quick_compile.bat".to_owned(), Some("new launcher")),
            ("quick_upgrade.bat".to_owned(), Some("new second launcher")),
            ("compile_log.txt".to_owned(), Some("a member's compile log")),
            (
                "data/teams_list.txt".to_owned(),
                Some("a member's teams list"),
            ),
            (KEPT_FOLDER.to_owned(), None),
            (format!("{KEPT_FOLDER}/co Full Spring"), None),
            (
                format!("{KEPT_FOLDER}/co Full Spring/notes.txt"),
                Some("a member's notes"),
            ),
            (NEW_FOLDER.to_owned(), None),
        ]
        .into_iter()
        .map(|(path, text)| (path, text.map(str::to_owned)))
        .collect();
        assert_eq!(
            snapshot(&bundle),
            expected,
            "{label}: the temp folder gone too"
        );
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn an_older_program_in_old_is_replaced_by_the_running_one() {
    for (kind, label) in KINDS {
        let mut files = old_files();
        files.push((format!("old/{}", exe_file_name()), "older program"));
        let bundle = folder_with(&format!("older_{label}"), &files);
        let (release, open_url) = bundle_release(kind, None);
        install_now(kind, &release, &bundle, &open_url)
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert_eq!(
            fs::read_to_string(bundle.join("old").join(exe_file_name())).unwrap(),
            "old program",
            "{label}"
        );
        assert!(!bundle.join(TEMP_FOLDER).exists(), "{label}");
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn a_checksum_that_does_not_match_changes_nothing() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("mismatch_{label}"), &old_files());
        let before = snapshot(&bundle);
        let (release, open_url) = bundle_release(kind, Some(to_hex(&[0x11; 32])));
        let outcome = install_now(kind, &release, &bundle, &open_url);
        assert!(
            matches!(&outcome, Err(InstallError::ChecksumMismatch(name)) if *name == archive_name(kind)),
            "{label}: {outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before, "{label}");
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn checksums_without_the_archives_line_change_nothing() {
    let kind = ArchiveKind::SevenZ;
    let bundle = folder_with("no_line", &old_files());
    let before = snapshot(&bundle);
    let archive = pack(kind, &new_files());
    let only_linux = format!(
        "{}  {}\n",
        to_hex(&Sha256::digest(&archive)),
        archive_name(ArchiveKind::TarGz)
    );
    let (release, open_url) = release(vec![
        (archive_name(kind), archive),
        (CHECKSUMS_ASSET.to_owned(), only_linux.into_bytes()),
    ]);
    let outcome = install_now(kind, &release, &bundle, &open_url);
    assert!(
        matches!(&outcome, Err(InstallError::NoChecksumLine(name)) if *name == archive_name(kind)),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before);
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn a_download_cancelled_midway_changes_nothing() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("cancel_{label}"), &old_files());
        let before = snapshot(&bundle);
        let (release, open_url) = bundle_release(kind, None);
        let cancel = AtomicBool::new(false);
        let mut reports = Vec::new();
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            ProgramStamp::of(&bundle),
            &open_url,
            |progress| {
                reports.push(progress);
                cancel.store(true, Ordering::Relaxed);
            },
            &cancel,
        );
        assert!(
            matches!(outcome, Err(InstallError::Cancelled)),
            "{label}: {outcome:?}"
        );
        let [report] = reports.as_slice() else {
            panic!("{label}: one read before the cancel, got {reports:?}");
        };
        assert!(report.done < report.total, "{label}: cancelled midway");
        assert_eq!(snapshot(&bundle), before, "{label}");
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn a_swap_failing_midway_puts_every_file_back() {
    for (kind, label) in KINDS {
        for older in [false, true] {
            // A folder where the new launcher must go, which the launcher's move
            // fails on: renaming a file over a non-empty folder is refused on
            // Windows and Linux alike. The program moves first, the bundle's
            // folders are made next and its other files move in name order, so
            // by then the program, the readme and the license have moved and
            // `old_exports/` is made, while the second launcher,
            // `quick_upgrade.bat`, has not moved.
            let mut files: Files = old_files()
                .into_iter()
                .filter(|(name, _)| name != "quick_compile.bat")
                .collect();
            files.push(("quick_compile.bat/notes.txt".to_owned(), "a member's notes"));
            if older {
                files.push((format!("old/{}", exe_file_name()), "older program"));
            }
            let bundle = folder_with(&format!("swap_{label}_{older}"), &files);
            let before = snapshot(&bundle);
            let (release, open_url) = bundle_release(kind, None);
            // The window's stamp, taken once: the program moved into `old/` and
            // back is the same file, so a second attempt is not refused as a
            // program another window replaced.
            let stamp = ProgramStamp::of(&bundle);
            for attempt in ["first", "second"] {
                let outcome = install_kind(
                    kind,
                    &release,
                    &bundle,
                    stamp,
                    &open_url,
                    |_| {},
                    &AtomicBool::new(false),
                );
                let Err(error @ InstallError::Swap { source, .. }) = &outcome else {
                    panic!("{label}, older {older}, {attempt} attempt: {outcome:?}");
                };
                assert_eq!(
                    error.to_string(),
                    format!(
                        "The update stopped while putting the new quick_compile.bat in place \
                         ({source}), and every file was put back."
                    ),
                    "{label}, older {older}, {attempt} attempt"
                );
                assert_eq!(
                    snapshot(&bundle),
                    before,
                    "{label}, older {older}, {attempt} attempt"
                );
            }
            fs::remove_dir_all(bundle).unwrap();
        }
    }
}

#[test]
fn a_release_without_this_platforms_archive_or_checksums_is_refused() {
    let other = if ArchiveKind::RUNNING == ArchiveKind::SevenZ {
        ArchiveKind::TarGz
    } else {
        ArchiveKind::SevenZ
    };
    let bundle = folder_with("assets", &old_files());
    let before = snapshot(&bundle);
    let checksums = (CHECKSUMS_ASSET.to_owned(), b"listed".to_vec());
    let (release_without_archive, open_url) =
        release(vec![(archive_name(other), b"archive".to_vec()), checksums]);
    let outcome = install(
        &release_without_archive,
        &bundle,
        ProgramStamp::of(&bundle),
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    );
    assert!(
        matches!(outcome, Err(InstallError::NoArchive(suffix)) if suffix == ArchiveKind::RUNNING.suffix()),
        "{outcome:?}"
    );
    let (release_without_checksums, open_url) = release(vec![(
        archive_name(ArchiveKind::RUNNING),
        b"archive".to_vec(),
    )]);
    let outcome = install(
        &release_without_checksums,
        &bundle,
        ProgramStamp::of(&bundle),
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    );
    assert!(
        matches!(outcome, Err(InstallError::NoChecksums)),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before);
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn the_running_platform_downloads_its_own_archive() {
    let (release, _) = release(vec![
        (archive_name(ArchiveKind::SevenZ), b"7z".to_vec()),
        (archive_name(ArchiveKind::TarGz), b"tar".to_vec()),
        (CHECKSUMS_ASSET.to_owned(), b"listed".to_vec()),
    ]);
    let (archive, _) = assets(&release, ArchiveKind::RUNNING).unwrap();
    let expected = if cfg!(windows) {
        "-windows.7z"
    } else {
        "-linux.tar.gz"
    };
    assert!(archive.name.ends_with(expected), "{}", archive.name);
}

#[test]
fn files_an_earlier_update_left_aside_stop_the_next_one() {
    let kind = ArchiveKind::SevenZ;
    let mut files = old_files();
    files.push((
        format!("{TEMP_FOLDER}/{BACKUP_FOLDER}/README.txt"),
        "the readme an earlier update moved aside",
    ));
    let bundle = folder_with("kept_backup", &files);
    let before = snapshot(&bundle);
    let (release, open_url) = bundle_release(kind, None);
    let outcome = install_now(kind, &release, &bundle, &open_url);
    let backup = bundle.join(TEMP_FOLDER).join(BACKUP_FOLDER);
    assert!(
        matches!(&outcome, Err(InstallError::OldFilesLeft { folder }) if *folder == backup),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before, "the leftover kept too");
    fs::remove_dir_all(bundle).unwrap();

    let mut files = old_files();
    files.push((
        format!("{TEMP_FOLDER}/{}", archive_name(kind)),
        "the first half of an archive",
    ));
    let bundle = folder_with("cut_download", &files);
    install_now(kind, &release, &bundle, &open_url)
        .unwrap_or_else(|error| panic!("a download cut short is cleared: {error}"));
    assert!(!bundle.join(TEMP_FOLDER).exists());
    assert_eq!(
        fs::read_to_string(bundle.join("README.txt")).unwrap(),
        "new readme"
    );
    fs::remove_dir_all(bundle).unwrap();
}

#[cfg(unix)]
#[test]
fn the_installed_linux_program_is_executable() {
    use std::os::unix::fs::PermissionsExt;

    let kind = ArchiveKind::TarGz;
    let bundle = folder_with("executable", &old_files());
    let exe = bundle.join(exe_file_name());
    let owner_may_run = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o100 != 0;
    assert!(
        !owner_may_run(&exe),
        "the premise: the old program is not executable"
    );
    let (release, open_url) = bundle_release(kind, None);
    install_now(kind, &release, &bundle, &open_url).unwrap_or_else(|error| panic!("{error}"));
    assert!(owner_may_run(&exe), "the installed program is executable");
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn an_archive_that_is_not_one_program_folder_changes_nothing() {
    let kind = ArchiveKind::SevenZ;
    let exe = exe_file_name();
    // The program in both folders, so the folders' count alone refuses it,
    // whichever folder the listing returns first.
    let two_folders: Files = vec![
        (format!("{TOP}/{exe}"), "new program"),
        (format!("other/{exe}"), "another program"),
    ];
    let no_program: Files = vec![(format!("{TOP}/README.txt"), "new readme")];
    for (case, files) in [("two_folders", two_folders), ("no_program", no_program)] {
        let bundle = folder_with(&format!("not_a_bundle_{case}"), &old_files());
        let before = snapshot(&bundle);
        let (release, open_url) = archive_release(kind, pack_entries(kind, &files, &[]), None);
        let outcome = install_now(kind, &release, &bundle, &open_url);
        assert!(
            matches!(outcome, Err(InstallError::NotABundle)),
            "{case}: {outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before, "{case}");
        fs::remove_dir_all(bundle).unwrap();
    }
}

/// A download's body that stumbles once: its first half, then one read
/// failing with `error`, then the rest.
struct Stumbling {
    /// The first half.
    first: Cursor<Vec<u8>>,
    /// The failure the read after the first half returns, `None` once
    /// returned.
    error: Option<io::Error>,
    /// The second half.
    rest: Cursor<Vec<u8>>,
}

impl Read for Stumbling {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.first.read(buf)?;
        if read > 0 {
            return Ok(read);
        }
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        self.rest.read(buf)
    }
}

/// A release whose `kind` archive holds the new bundle, its `SHA256SUMS.txt`
/// served whole and the archive's URL answered by `answer`, which is handed
/// the archive's bytes.
fn release_answering(
    kind: ArchiveKind,
    answer: impl Fn(&[u8]) -> Result<Box<dyn Read + Send>, ureq::Error> + Send + Sync + 'static,
) -> (Release, Box<OpenUrl>) {
    let assets = archive_assets(kind, pack(kind, &new_files()), None);
    let archive = assets[0].1.clone();
    let (release, served) = release(assets);
    let archive_url = release.assets[0].download_url.clone();
    let open_url = move |url: &str| {
        if url == archive_url {
            answer(&archive)
        } else {
            served(url)
        }
    };
    (release, Box::new(open_url))
}

/// A 7z release of the new bundle whose archive's body stumbles once, with
/// an error of `kind`; `SHA256SUMS.txt` reads whole.
fn stumbling_release(kind: io::ErrorKind) -> (Release, Box<OpenUrl>) {
    release_answering(ArchiveKind::SevenZ, move |archive| {
        let (first, rest) = archive.split_at(archive.len() / 2);
        Ok(Box::new(Stumbling {
            first: Cursor::new(first.to_vec()),
            error: Some(io::Error::new(kind, "the read stumbled")),
            rest: Cursor::new(rest.to_vec()),
        }))
    })
}

#[test]
fn a_read_interrupted_once_goes_on_and_installs() {
    let bundle = folder_with("interrupted", &old_files());
    let (release, open_url) = stumbling_release(io::ErrorKind::Interrupted);
    install_now(ArchiveKind::SevenZ, &release, &bundle, &open_url)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        fs::read_to_string(bundle.join(exe_file_name())).unwrap(),
        "new program"
    );
    assert!(!bundle.join(TEMP_FOLDER).exists());
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn a_read_that_fails_stops_the_download_and_changes_nothing() {
    let bundle = folder_with("read_fails", &old_files());
    let before = snapshot(&bundle);
    let (release, open_url) = stumbling_release(io::ErrorKind::ConnectionReset);
    let outcome = install_now(ArchiveKind::SevenZ, &release, &bundle, &open_url);
    assert!(
        matches!(
            &outcome,
            Err(InstallError::Download(error)) if error.kind() == io::ErrorKind::ConnectionReset
        ),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before);
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn a_program_replaced_since_the_window_started_is_refused_before_any_request() {
    let kind = ArchiveKind::SevenZ;
    // The program's new text, `None` to remove it, and how many seconds after
    // the noted date its file is dated, so each field of the stamp is
    // changed alone, whatever the file system's clock grain.
    for (case, text, later) in [
        ("same_length_later_date", Some("new program"), 10),
        ("other_length_same_date", Some(REPLACED_PROGRAM), 0),
        ("removed", None, 0),
    ] {
        let bundle = folder_with(&format!("replaced_{case}"), &old_files());
        let stamp = ProgramStamp::of(&bundle);
        let noted = stamp.expect("the premise: the program's file is read");
        let program = bundle.join(exe_file_name());
        match text {
            Some(text) => {
                let mut file = File::create(&program).unwrap();
                file.write_all(text.as_bytes()).unwrap();
                file.set_modified(noted.modified + Duration::from_secs(later))
                    .unwrap();
            }
            None => fs::remove_file(&program).unwrap(),
        }
        if let Some(now) = ProgramStamp::of(&bundle) {
            let one_field = if later > 0 {
                now.len == noted.len
            } else {
                now.modified == noted.modified
            };
            assert!(one_field, "{case}: the premise, the other field as noted");
        }
        let before = snapshot(&bundle);
        let (release, open_url) = bundle_release(kind, None);
        let (open_url, requests) = counted(open_url);
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            stamp,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
        assert!(
            matches!(outcome, Err(InstallError::ProgramReplaced)),
            "{case}: {outcome:?}"
        );
        assert_eq!(requests.load(Ordering::SeqCst), 0, "{case}: no request");
        assert_eq!(snapshot(&bundle), before, "{case}");
        fs::remove_dir_all(bundle).unwrap();
    }
}

/// A download's body that sets `cancel` as it reaches its end: the download
/// reads `cancel` before each read, so it never sees this one.
struct CancelAtEnd {
    /// The whole body.
    body: Cursor<Vec<u8>>,
    /// The flag the end of the body sets.
    cancel: Arc<AtomicBool>,
}

impl Read for CancelAtEnd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.body.read(buf)?;
        if read == 0 {
            self.cancel.store(true, Ordering::Relaxed);
        }
        Ok(read)
    }
}

#[test]
fn a_cancel_after_the_last_byte_stops_before_the_swap() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("cancel_end_{label}"), &old_files());
        let before = snapshot(&bundle);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        let (release, open_url) = release_answering(kind, move |archive| {
            Ok(Box::new(CancelAtEnd {
                body: Cursor::new(archive.to_vec()),
                cancel: Arc::clone(&flag),
            }))
        });
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            ProgramStamp::of(&bundle),
            &open_url,
            |_| {},
            &cancel,
        );
        assert!(
            matches!(outcome, Err(InstallError::Cancelled)),
            "{label}: {outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before, "{label}: no temp folder left");
        fs::remove_dir_all(bundle).unwrap();
    }
}

/// The program another window's update put in place.
const REPLACED_PROGRAM: &str = "the program another window installed";

/// A download's body that rewrites the program's file as it reaches its end,
/// as another window's update ending meanwhile would.
struct ReplaceAtEnd {
    /// The whole body.
    body: Cursor<Vec<u8>>,
    /// The program's file in the program's folder.
    program: PathBuf,
}

impl Read for ReplaceAtEnd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.body.read(buf)?;
        if read == 0 {
            fs::write(&self.program, REPLACED_PROGRAM)?;
        }
        Ok(read)
    }
}

#[test]
fn a_program_replaced_while_the_download_ran_is_refused_before_the_swap() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("replaced_late_{label}"), &old_files());
        let mut expected = snapshot(&bundle);
        expected.insert(exe_file_name(), Some(REPLACED_PROGRAM.to_owned()));
        let program = bundle.join(exe_file_name());
        let (release, open_url) = release_answering(kind, move |archive| {
            Ok(Box::new(ReplaceAtEnd {
                body: Cursor::new(archive.to_vec()),
                program: program.clone(),
            }))
        });
        let outcome = install_now(kind, &release, &bundle, &open_url);
        assert!(
            matches!(outcome, Err(InstallError::ProgramReplaced)),
            "{label}: {outcome:?}"
        );
        assert_eq!(
            snapshot(&bundle),
            expected,
            "{label}: only the program rewritten, no old/ and no temp folder"
        );
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn the_temp_folder_stays_only_when_the_old_files_could_not_be_put_back() {
    let files: Files = vec![(format!("{BACKUP_FOLDER}/README.txt"), "old readme")];
    let temp = folder_with("remove_temp", &files);
    let not_restored = Err(InstallError::NotRestored {
        step: SwapStep::PlaceFile(PathBuf::from("README.txt")),
        source: io::Error::other("the move failed"),
        restore: io::Error::other("the undo failed"),
        kept: temp.join(BACKUP_FOLDER),
    });
    remove_temp(&temp, &not_restored);
    assert_eq!(
        fs::read_to_string(temp.join(BACKUP_FOLDER).join("README.txt")).ok(),
        Some("old readme".to_owned()),
        "the old files kept"
    );
    for (case, outcome) in [
        ("cancelled", Err(InstallError::Cancelled)),
        ("done", Ok(())),
    ] {
        write_files(&temp, &files);
        remove_temp(&temp, &outcome);
        assert!(!temp.exists(), "{case}");
    }
}

#[test]
fn a_folder_that_cannot_take_the_temp_folder_is_refused_before_any_request() {
    let kind = ArchiveKind::SevenZ;
    let mut files = old_files();
    files.push((TEMP_FOLDER.to_owned(), "a file where the temp folder goes"));
    let bundle = folder_with("not_writable", &files);
    let before = snapshot(&bundle);
    let (release, open_url) = bundle_release(kind, None);
    let (open_url, requests) = counted(open_url);
    let outcome = install_now(kind, &release, &bundle, &open_url);
    assert!(
        matches!(&outcome, Err(InstallError::NotWritable { folder, .. }) if *folder == bundle),
        "{outcome:?}"
    );
    assert_eq!(requests.load(Ordering::SeqCst), 0, "no request");
    assert_eq!(snapshot(&bundle), before);
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn a_request_the_server_refuses_changes_nothing() {
    let kind = ArchiveKind::SevenZ;
    let bundle = folder_with("refused", &old_files());
    let before = snapshot(&bundle);
    let (release, _) = bundle_release(kind, None);
    let unavailable = |_: &str| -> Result<Box<dyn Read + Send>, ureq::Error> {
        Err(ureq::Error::StatusCode(503))
    };
    let outcome = install_now(kind, &release, &bundle, &unavailable);
    assert!(
        matches!(
            outcome,
            Err(InstallError::Request(ureq::Error::StatusCode(503)))
        ),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before, "no temp folder left");
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn an_archive_request_the_server_refuses_after_the_checksums_changes_nothing() {
    let kind = ArchiveKind::SevenZ;
    let bundle = folder_with("archive_refused", &old_files());
    let before = snapshot(&bundle);
    let (release, open_url) = release_answering(kind, |_| Err(ureq::Error::StatusCode(503)));
    let (open_url, requests) = counted(open_url);
    let outcome = install_now(kind, &release, &bundle, &open_url);
    assert!(
        matches!(
            outcome,
            Err(InstallError::Request(ureq::Error::StatusCode(503)))
        ),
        "{outcome:?}"
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        2,
        "the checksums, then the archive"
    );
    assert_eq!(snapshot(&bundle), before, "no temp folder left");
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn an_archive_that_does_not_unpack_changes_nothing() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("no_archive_{label}"), &old_files());
        let before = snapshot(&bundle);
        let (release, open_url) =
            archive_release(kind, b"these bytes are no archive".to_vec(), None);
        let outcome = install_now(kind, &release, &bundle, &open_url);
        assert!(
            matches!(outcome, Err(InstallError::Extract(_))),
            "{label}: {outcome:?}"
        );
        assert_eq!(snapshot(&bundle), before, "{label}: no temp folder left");
        fs::remove_dir_all(bundle).unwrap();
    }
}

#[test]
fn a_program_folder_without_the_program_changes_nothing() {
    let kind = ArchiveKind::SevenZ;
    let files: Files = old_files()
        .into_iter()
        .filter(|(name, _)| *name != exe_file_name())
        .collect();
    let bundle = folder_with("no_program", &files);
    assert_eq!(ProgramStamp::of(&bundle), None, "the premise: no stamp");
    let before = snapshot(&bundle);
    let (release, open_url) = bundle_release(kind, None);
    let outcome = install_now(kind, &release, &bundle, &open_url);
    assert!(
        matches!(
            outcome,
            Err(InstallError::Swap {
                step: SwapStep::RetireProgram,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert_eq!(snapshot(&bundle), before);
    fs::remove_dir_all(bundle).unwrap();
}

#[test]
fn the_bundles_program_is_utw() {
    let expected = if cfg!(windows) { "UTW.exe" } else { "utw" };
    assert_eq!(exe_file_name(), expected);
}
