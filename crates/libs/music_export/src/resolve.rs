//! Resolving an export's song paths against the `.4ccm`'s folder before a
//! match: each `SongEntry::file` looked up under the case-insensitive and
//! `_normalized` companion rules, and every file that resolves nowhere
//! collected into one list. Native only: the wasm build has no filesystem.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::model::MusicExport;

/// The songs of an export located on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Each resolved `SongEntry::file`, with the path it resolved to.
    pub found: Vec<(String, PathBuf)>,
    /// The files that resolved nowhere, each once, in the model's order
    /// (slot by slot, then the event clips).
    pub missing: Vec<String>,
}

/// Where `file` (a `SongEntry::file`, relative to `folder`) is on disk, or
/// `None`.
///
/// Two candidates are tried in the song's folder (`folder`, or its subfolder
/// when `file` names one, a `\` separator read as `/`, so a path written on
/// Windows resolves on Linux; a song is always under the folder: a leading
/// separator is dropped, and a path whose first component names a drive
/// resolves nowhere, on every OS): the
/// plain file, and its `_normalized` companion, a file whose stem is
/// `<stem>_normalized` whatever its extension (real exports pair `X.mp3` with
/// `X_normalized.opus`), the one with the song's own extension first when
/// there are several. With `normalization_applies` unset the companion comes
/// first, with it set the plain file. Each candidate is its own lookup in the
/// folder's listing, names matched case-insensitively, the exact spelling (or
/// the song's own extension) preferred and among equals the smallest name
/// (the subfolder components are used as written). The path returned carries
/// the name as it is on disk, so two spellings of one file resolve to one
/// path. A path with no file name (`sub/`) resolves nowhere, and an unreadable
/// folder is "not found" for that candidate.
pub fn resolve_song(folder: &Path, file: &str, normalization_applies: bool) -> Option<PathBuf> {
    // A song is always under the export's folder: a leading separator is
    // dropped (`/x.mp3` is the folder's `x.mp3`), and a path whose first
    // component names a drive (`C:`) resolves nowhere.
    let relative = file.replace('\\', "/");
    let relative = relative.trim_start_matches('/');
    let bytes = relative.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return None;
    }
    // The name as written, not the joined path's `file_name()`, which for
    // `sub/` is the subfolder itself.
    let name = relative.rsplit('/').next()?;
    if name.is_empty() {
        return None;
    }
    let path = folder.join(relative);
    let dir = path.parent()?;
    let (stem, extension) = split_extension(name);
    let folded_name = name.to_lowercase();
    let (companion_stem, folded_extension) = (
        format!("{stem}_normalized").to_lowercase(),
        extension.to_lowercase(),
    );
    let plain = || {
        find_in(dir, |on_disk| {
            if on_disk == name {
                Some(Rank::Preferred)
            } else if on_disk.to_lowercase() == folded_name {
                Some(Rank::Acceptable)
            } else {
                None
            }
        })
    };
    let companion = || {
        find_in(dir, |on_disk| {
            let (on_disk_stem, on_disk_extension) = split_extension(on_disk);
            if on_disk_stem.to_lowercase() != companion_stem {
                return None;
            }
            Some(if on_disk_extension.to_lowercase() == folded_extension {
                Rank::Preferred
            } else {
                Rank::Acceptable
            })
        })
    };
    if normalization_applies {
        plain().or_else(companion)
    } else {
        companion().or_else(plain)
    }
}

/// A file name's stem and extension (the extension with its dot); a name whose
/// only dot is its first byte (`.intro`) has no extension.
fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(index) if index > 0 => (&name[..index], &name[index..]),
        _ => (name, ""),
    }
}

/// How well a listed file fits a lookup; `Preferred` orders first (the derive
/// follows the declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rank {
    /// The exact name, or the companion with the song's own extension.
    Preferred,
    /// A case-folded name, or a companion of another extension.
    Acceptable,
}

/// `dir`'s file that `rank` likes best, by its name on disk: the best rank,
/// and among equals the smallest name, so the listing's order (the
/// filesystem's) decides nothing. A listing scan, not
/// `dir.join(name).exists()`: on a case-insensitive filesystem that answers
/// with the written spelling. An unreadable folder, or an unreadable entry,
/// finds nothing.
fn find_in(dir: &Path, rank: impl Fn(&str) -> Option<Rank>) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let on_disk = entry.file_name();
        if let Some(rank) = rank(&on_disk.to_string_lossy()) {
            candidates.push((rank, on_disk, entry.path()));
        }
    }
    candidates.into_iter().min().map(|(_, _, path)| path)
}

/// Every distinct song file of `export`, resolved, in the model's order: slot
/// by slot in order of first appearance, file order within each, then the
/// event clips; the files that resolve nowhere listed once each, in that order.
pub fn resolve_export(
    export: &MusicExport,
    folder: &Path,
    normalization_applies: bool,
) -> Resolution {
    let files = export
        .slots
        .values()
        .flatten()
        .map(|entry| entry.file.as_str())
        .chain(export.events.iter().map(|clip| clip.entry.file.as_str()));
    let mut seen = HashSet::new();
    let (mut found, mut missing) = (Vec::new(), Vec::new());
    for file in files {
        if !seen.insert(file) {
            continue;
        }
        match resolve_song(folder, file, normalization_applies) {
            Some(path) => found.push((file.to_string(), path)),
            None => missing.push(file.to_string()),
        }
    }
    Resolution { found, missing }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    /// A fresh folder holding `files` (each `<name>=<bytes>`, a `/` naming a
    /// subfolder), under the test binary's build folder
    /// (`target/<profile>/music_export_tests/resolve_<name>`). [`test_scratch::fresh`]
    /// says when the folder is removed and what `name` must be.
    fn temp_dir(name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = test_scratch::fresh(
            &test_scratch::build_folder().join("music_export_tests"),
            &format!("resolve_{name}"),
        );
        for (file, bytes) in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(path, bytes).expect("seed");
        }
        dir
    }

    /// The path `resolve_song` returns, relative to `dir`, with `/`
    /// separators: the name as it is on disk.
    fn resolved(dir: &Path, file: &str, normalization_applies: bool) -> Option<String> {
        resolve_song(dir, file, normalization_applies).map(|path| {
            path.strip_prefix(dir)
                .expect("under the folder")
                .to_string_lossy()
                .replace('\\', "/")
        })
    }

    fn export(text: &str) -> MusicExport {
        parse(format!("name;jp\n{text}").as_bytes(), "jp")
            .expect("parses")
            .export
    }

    #[test]
    fn a_song_resolves_its_file_then_the_normalized_companion() {
        let dir = temp_dir(
            "songs",
            &[
                ("Song.mp3", b"song"),
                ("song_normalized.mp3", b"normalized"),
                ("sub/Other.MP3", b"other"),
            ],
        );
        assert_eq!(
            resolved(&dir, "song.mp3", true).as_deref(),
            Some("Song.mp3"),
            "the plain file first while normalization applies, by its name on disk"
        );
        assert_eq!(
            resolved(&dir, "song.mp3", false).as_deref(),
            Some("song_normalized.mp3"),
            "the normalized companion first when it does not"
        );
        assert_eq!(
            resolved(&dir, "sub/other.mp3", true).as_deref(),
            Some("sub/Other.MP3"),
            "the name matches case-insensitively inside its own subfolder"
        );
        assert_eq!(resolved(&dir, "Gone.mp3", true), None);
    }

    #[test]
    fn a_subfolder_written_with_backslashes_resolves_on_any_os() {
        let dir = temp_dir("backslash", &[("sub/deep/Other.MP3", b"other")]);
        assert_eq!(
            resolved(&dir, "sub\\deep\\other.mp3", true).as_deref(),
            Some("sub/deep/Other.MP3"),
            "a path written on Windows, its name matched case-insensitively in its own subfolder"
        );
    }

    /// A leading separator is dropped: the song is always under the export's
    /// folder.
    #[test]
    fn a_leading_separator_is_dropped() {
        let sub = format!("music_export_resolve_anchor_{}", std::process::id());
        let relative = format!("{sub}/x.mp3");
        let dir = temp_dir("anchored", &[(relative.as_str(), b"x"), ("y.mp3", b"y")]);
        assert_eq!(
            resolved(&dir, &format!("/{relative}"), true).as_deref(),
            Some(relative.as_str()),
            "`/{sub}/x.mp3` is the folder's `{sub}/x.mp3`"
        );
        assert_eq!(
            resolved(&dir, &format!("\\{sub}\\x.mp3"), true).as_deref(),
            Some(relative.as_str())
        );
        assert_eq!(
            resolved(&dir, "/y.mp3", true).as_deref(),
            Some("y.mp3"),
            "a root-level anchor is the folder's file too"
        );
    }

    #[test]
    fn a_name_written_in_upper_case_matches_its_lower_case_file() {
        let dir = temp_dir("upper", &[("song.mp3", b"song")]);
        assert_eq!(
            resolved(&dir, "SONG.MP3", true).as_deref(),
            Some("song.mp3")
        );
    }

    /// A name whose only dot is its first byte (`.intro`) has no
    /// extension: its companion is `.intro_normalized`, not
    /// `_normalized.intro`.
    #[test]
    fn a_leading_dot_name_is_no_extension() {
        let dir = temp_dir("dotfile", &[(".intro_normalized", b"normalized")]);
        assert_eq!(
            resolved(&dir, ".intro", true).as_deref(),
            Some(".intro_normalized")
        );
    }

    /// Real exports pair `X.mp3` with `X_normalized.opus` (IZAKAYA's, 17
    /// songs): the companion is matched by stem, whatever its extension.
    #[test]
    fn a_normalized_companion_of_another_extension_is_found() {
        let dir = temp_dir(
            "companion",
            &[
                ("ANTHEM.mp3", b"song"),
                ("ANTHEM_normalized.opus", b"normalized"),
                ("gone_normalized.opus", b"gone"),
            ],
        );
        assert_eq!(
            resolved(&dir, "ANTHEM.mp3", false).as_deref(),
            Some("ANTHEM_normalized.opus"),
            "the companion first when normalization does not apply"
        );
        assert_eq!(
            resolved(&dir, "ANTHEM.mp3", true).as_deref(),
            Some("ANTHEM.mp3")
        );
        assert_eq!(
            resolved(&dir, "gone.mp3", true).as_deref(),
            Some("gone_normalized.opus"),
            "the plain file missing: the companion of another extension serves"
        );
    }

    #[test]
    fn among_several_companions_the_song_s_own_extension_wins_case_insensitively() {
        let dir = temp_dir(
            "companions",
            &[
                ("x_normalized.opus", b"opus"),
                ("x_normalized.MP3", b"mp3"),
                ("song_NORMALIZED.ogg", b"ogg"),
                ("y_normalized.opus", b"opus"),
                ("y_normalized.ogg", b"ogg"),
            ],
        );
        assert_eq!(
            resolved(&dir, "x.mp3", false).as_deref(),
            Some("x_normalized.MP3"),
            "the song's own extension, in any case, over another"
        );
        assert_eq!(
            resolved(&dir, "y.mp3", false).as_deref(),
            Some("y_normalized.ogg"),
            "neither has the song's extension: the smaller name, whatever the listing's order"
        );
        assert_eq!(
            resolved(&dir, "Song.mp3", false).as_deref(),
            Some("song_NORMALIZED.ogg"),
            "the companion's stem matches case-insensitively"
        );
    }

    /// The companion's stem folds like the plain name does, Unicode included:
    /// the lookup folds the name and the stem alike.
    #[test]
    fn a_companion_s_stem_folds_non_ascii_letters() {
        let dir = temp_dir("unicode", &[("CAFÉ_normalized.opus", b"normalized")]);
        assert_eq!(
            resolved(&dir, "café.mp3", false).as_deref(),
            Some("CAFÉ_normalized.opus")
        );
    }

    #[test]
    fn a_companion_is_looked_up_in_the_song_s_own_subfolder() {
        let dir = temp_dir(
            "subcompanion",
            &[("sub/x_normalized.opus", b"normalized"), ("x.mp3", b"top")],
        );
        assert_eq!(
            resolved(&dir, "sub/x.mp3", false).as_deref(),
            Some("sub/x_normalized.opus")
        );
        assert_eq!(
            resolved(&dir, "sub/x.mp3", true).as_deref(),
            Some("sub/x_normalized.opus"),
            "no plain file in the subfolder: the top folder's is not it"
        );
    }

    /// A name with no dot has no extension: it still folds, and its companion
    /// of any extension is found.
    #[test]
    fn a_name_without_an_extension_folds_and_finds_its_companion() {
        let dir = temp_dir(
            "noext",
            &[
                ("Anthem", b"song"),
                ("anthem_normalized.opus", b"normalized"),
            ],
        );
        assert_eq!(resolved(&dir, "anthem", true).as_deref(), Some("Anthem"));
        assert_eq!(
            resolved(&dir, "anthem", false).as_deref(),
            Some("anthem_normalized.opus")
        );
    }

    /// A path with no file name (`sub/`, the empty string) is no song: a folder
    /// of that name is not returned in its place.
    #[test]
    fn a_path_without_a_file_name_resolves_nowhere() {
        let dir = temp_dir("noname", &[("sub/x.mp3", b"x")]);
        assert_eq!(resolved(&dir, "sub/", true), None);
        assert_eq!(resolved(&dir, "sub\\", true), None);
        assert_eq!(resolve_song(&dir, "", true), None, "not the folder itself");
    }

    /// A path whose first component names a drive (`C:`) is never under the
    /// export's folder: it resolves nowhere, on every OS.
    #[test]
    fn a_drive_qualified_path_resolves_nowhere() {
        let name = format!("music_export_resolve_root_{}.mp3", std::process::id());
        // The refused paths name a file that exists. `C:` is no folder name
        // on Windows: there the path is the seeded file's own absolute
        // spelling; elsewhere `C:` is an ordinary folder, and `C:<name>` an
        // ordinary file name, both seeded inside the export folder.
        #[cfg(windows)]
        let dir = temp_dir("root", &[(name.as_str(), b"x")]);
        #[cfg(not(windows))]
        let dir = {
            let drive_rooted = format!("C:/{name}");
            let drive_relative = format!("C:{name}");
            temp_dir(
                "root",
                &[
                    (name.as_str(), b"x"),
                    (drive_rooted.as_str(), b"x"),
                    (drive_relative.as_str(), b"x"),
                ],
            )
        };
        #[cfg(windows)]
        {
            let absolute = dir.join(&name).to_string_lossy().into_owned();
            assert_eq!(resolved(&dir, &absolute, true), None);
            assert_eq!(resolved(&dir, &absolute.replace('\\', "/"), true), None);
        }
        assert_eq!(resolved(&dir, &format!("C:/{name}"), true), None);
        assert_eq!(resolved(&dir, &format!("C:\\{name}"), true), None);
        assert_eq!(
            resolved(&dir, &format!("C:{name}"), true),
            None,
            "a drive-relative path resolves nowhere too"
        );
        assert_eq!(
            resolved(&dir, &name, true).as_deref(),
            Some(name.as_str()),
            "the relative spelling is found"
        );
    }

    #[test]
    fn a_missing_plain_file_falls_back_to_the_normalized_companion() {
        let dir = temp_dir("gone", &[("gone_normalized.mp3", b"gone")]);
        assert_eq!(
            resolved(&dir, "gone.mp3", true).as_deref(),
            Some("gone_normalized.mp3"),
            "the plain file missing: the normalized file is used regardless"
        );
    }

    #[test]
    fn an_export_resolves_each_distinct_file_and_reports_the_missing_once() {
        let dir = temp_dir("export", &[("Song.mp3", b"song")]);
        let export = export(
            "goal;Song.mp3\nBilly;Song.mp3\nBilly;a.mp3\n\
             Billy;b.mp3;event red\nBilly;a.mp3;event yellow\n",
        );
        let resolution = resolve_export(&export, &dir, true);
        assert_eq!(
            resolution.found,
            [("Song.mp3".to_string(), dir.join("Song.mp3"))],
            "Song.mp3 named twice resolves once"
        );
        assert_eq!(
            resolution.missing,
            ["a.mp3".to_string(), "b.mp3".to_string()],
            "the slot and the event clip's missing files, in file order"
        );
    }
}
