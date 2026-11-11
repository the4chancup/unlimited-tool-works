use std::collections::BTreeMap;
use std::io::Cursor;

use super::test_bundle::*;
use super::*;

/// Both archive kinds, with a name for the test folders and messages.
const KINDS: [(ArchiveKind, &str); 2] = [(ArchiveKind::SevenZ, "7z"), (ArchiveKind::TarGz, "tar")];

#[test]
fn an_update_puts_the_new_bundle_in_place_and_keeps_data() {
    for (kind, label) in KINDS {
        let bundle = folder_with(&format!("happy_{label}"), &old_files());
        let (release, open_url) = bundle_release(kind, None);
        let mut reports = Vec::new();
        let installed = install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |progress| reports.push(progress),
            &AtomicBool::new(false),
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));

        let exe = exe_file_name();
        assert_eq!(installed.version, Version::new(0, 0, 2));
        assert_eq!(installed.old_exe, bundle.join("old").join(&exe));
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
        install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        )
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
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
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
    let outcome = install_kind(
        kind,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    );
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
            // Windows and Linux alike. The launcher comes last by name, so the
            // program, the readme and the license have moved by then.
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
            let outcome = install_kind(
                kind,
                &release,
                &bundle,
                &open_url,
                |_| {},
                &AtomicBool::new(false),
            );
            let Err(error @ InstallError::Swap { source, .. }) = &outcome else {
                panic!("{label}, older {older}: {outcome:?}");
            };
            assert_eq!(
                error.to_string(),
                format!(
                    "The update stopped while putting the new quick_compile.bat in place \
                     ({source}), and every file was put back."
                ),
                "{label}, older {older}"
            );
            assert_eq!(snapshot(&bundle), before, "{label}, older {older}");
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
    let (archive, checksums) = assets(&release, ArchiveKind::RUNNING).unwrap();
    let expected = if cfg!(windows) {
        "-windows.7z"
    } else {
        "-linux.tar.gz"
    };
    assert!(archive.name.ends_with(expected), "{}", archive.name);
    assert_eq!(checksums.name, CHECKSUMS_ASSET);
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
    let outcome = install_kind(
        kind,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    );
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
    install_kind(
        kind,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    )
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
    install_kind(
        kind,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    )
    .unwrap_or_else(|error| panic!("{error}"));
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
        let outcome = install_kind(
            kind,
            &release,
            &bundle,
            &open_url,
            |_| {},
            &AtomicBool::new(false),
        );
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

/// A 7z release of the new bundle whose archive's body stumbles once, with
/// an error of `kind`; `SHA256SUMS.txt` reads whole.
fn stumbling_release(kind: io::ErrorKind) -> (Release, Box<OpenUrl>) {
    let assets = archive_assets(
        ArchiveKind::SevenZ,
        pack(ArchiveKind::SevenZ, &new_files()),
        None,
    );
    let archive = assets[0].1.clone();
    let (release, served) = release(assets);
    let archive_url = release.assets[0].download_url.clone();
    let open_url = move |url: &str| -> Result<Box<dyn Read + Send>, ureq::Error> {
        if url != archive_url {
            return served(url);
        }
        let (first, rest) = archive.split_at(archive.len() / 2);
        Ok(Box::new(Stumbling {
            first: Cursor::new(first.to_vec()),
            error: Some(io::Error::new(kind, "the read stumbled")),
            rest: Cursor::new(rest.to_vec()),
        }))
    };
    (release, Box::new(open_url))
}

#[test]
fn a_read_interrupted_once_goes_on_and_installs() {
    let bundle = folder_with("interrupted", &old_files());
    let (release, open_url) = stumbling_release(io::ErrorKind::Interrupted);
    install_kind(
        ArchiveKind::SevenZ,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    )
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
    let outcome = install_kind(
        ArchiveKind::SevenZ,
        &release,
        &bundle,
        &open_url,
        |_| {},
        &AtomicBool::new(false),
    );
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
fn the_bundles_program_is_utw() {
    let expected = if cfg!(windows) { "UTW.exe" } else { "utw" };
    assert_eq!(exe_file_name(), expected);
}
