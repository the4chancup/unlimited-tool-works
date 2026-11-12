//! The referee export (spec/export_upgrader.md "Migrates referee exports",
//! "Flattens the AET referee layout's subfolders"): `refs.txt` rewritten as
//! the root `players.txt` in the canonical grammar, the root files, and each
//! referee folder under `Players/` with the old layout's `face/`, `boots/`,
//! `gloves/` and `common/` subfolders folded into its root:
//!
//! - a file of one of the four lands at its path below it (`face/kit/x.dds`
//!   becomes `kit/x.dds`), every other file stays where it is;
//! - one of the four names again inside one of them (`face/face/`) is the
//!   refused layout once more: reported, the folder left as it was;
//! - either `ingame_face` spelling that lands at the root becomes the one
//!   `ingame_face.toml`, as for a team's player folder;
//! - a `common/` model no `face.xml` of the folder names is reported and left
//!   out: a `face.xml` `path` is the only reference that names a model;
//! - one name in two locations is copied once when the bytes agree and is a
//!   clash otherwise, as is one texture stem from two locations; a clash
//!   leaves the folder as it was;
//! - a folder left as it was keeps its old paths in the planned layout, out
//!   of the model renames and reference rewrites (`PlayerFolders::untouched`),
//!   so the run's later findings name the author's files where he will find
//!   them. Its finding is an Error, so the run writes nothing: the planned
//!   layout is what the report is made from, never an output.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{FileKind, Marker, classify};
use studio_core::{Disposition, Message, Scope};

use crate::carried::under;
use crate::face_xml::named_paths;
use crate::messages::{INPUT, UnmigratedReason, unmigrated};
use crate::players::{PlayerFolders, is_stray, path_scope};
use crate::source::{OLD_FOLDERS, SourceFailure};

/// The old referee layout's subfolder names, folded: the compiler refuses a
/// referee folder holding one directly (`player_layout_proto`).
const LAYOUT_SUBFOLDERS: [&str; 4] = ["face", "boots", "gloves", "common"];

/// The referee export's roster file of `files` (directly at the root, any
/// letter case): `refs.txt` when there is one, else `refs list.txt`, the
/// refs compiler's older spelling of its layout.
pub(crate) fn find_refs_txt(files: &[String]) -> Option<&str> {
    files
        .iter()
        .find(|file| file.eq_ignore_ascii_case("refs.txt"))
        .or_else(|| {
            files
                .iter()
                .find(|file| file.eq_ignore_ascii_case("refs list.txt"))
        })
        .map(String::as_str)
}

/// The root `players.txt` text step 9 writes for `refs_txt`'s bytes: a UTF-8
/// BOM dropped, LF line ends, blank lines left out, every other line
/// trimmed; a line whose head (up to the first whitespace) is ASCII digits
/// making a `u16` with something after it is written as the slot in two
/// digits, one space and the trimmed rest (`1 germanbro` → `01 germanbro`,
/// `2\tdonte` → `02 donte`); any other line as trimmed, for the shared
/// validation to report on the output. `None` when the bytes are not UTF-8.
pub(crate) fn players_txt(refs_txt: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(refs_txt).ok()?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // The head and rest as the roster reader splits them, so a line
        // written canonical here reads back as the same entry.
        let slot = line
            .split_once(char::is_whitespace)
            .filter(|(head, _)| head.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|(head, rest)| Some((head.parse::<u16>().ok()?, rest.trim())));
        match slot {
            Some((slot, rest)) => out.push_str(&format!("{slot:02} {rest}")),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    Some(out)
}

/// Step 9's root files and step 12's referee folders: the root roster file
/// (`refs.txt` or `refs list.txt`, any letter case — both consumed when
/// both exist) consumed (not copied: `run` writes `players.txt` from what
/// `find_refs_txt` found), a root `ref_marker.dds` (any letter case) copied
/// under its own name, and every direct subfolder of `Players/` (any letter
/// case) flattened to `Players/<folder>/` (`flatten`), its files read
/// through `read` for the byte comparisons and the `face.xml` files. When
/// `files` holds no path under `Players/` — the refs compiler's layout —
/// every root folder `OLD_FOLDERS` does not list is a referee folder the
/// same way, and a file directly at the root stays nobody's: the fallback
/// reports it, as a Note is. A file directly in `Players/` is nobody's
/// either. `portraits` and `folders` stay empty: a referee has no save
/// record.
pub(crate) fn referee_folders(
    files: &[String],
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> PlayerFolders {
    let mut plan = PlayerFolders::default();
    // Each referee folder's files by its name as written: input path and
    // path below the folder, in input order.
    let mut folders: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    // The refs compiler's layout stands the referee folders directly at the
    // root instead of under `Players/`; it is the layout only when no
    // `Players/` path exists.
    let root_layout = !files.iter().any(|file| under(file, "Players").is_some());
    for file in files {
        if file.eq_ignore_ascii_case("refs.txt") || file.eq_ignore_ascii_case("refs list.txt") {
            plan.consumed.insert(file.clone());
        } else if file.eq_ignore_ascii_case("ref_marker.dds") {
            plan.copies.push((file.clone(), file.clone()));
            plan.consumed.insert(file.clone());
        } else if let Some(rest) = under(file, "Players")
            && let Some((name, below)) = rest.split_once('/')
        {
            folders.entry(name).or_default().push((file, below));
        } else if root_layout
            && let Some((name, below)) = file.split_once('/')
            && !OLD_FOLDERS
                .iter()
                .any(|known| name.eq_ignore_ascii_case(known))
        {
            folders.entry(name).or_default().push((file, below));
        }
    }
    for (name, folder_files) in folders {
        flatten(name, &folder_files, &mut read, &mut plan);
    }
    plan
}

/// Whether `segment` is one of the old layout's subfolder names, any letter
/// case.
fn is_layout_subfolder(segment: &str) -> bool {
    LAYOUT_SUBFOLDERS.contains(&vtree::fold_name(segment).as_str())
}

/// One file of a referee folder, placed by the fold.
struct Placed<'a> {
    /// The input path.
    input: &'a str,
    /// Its path below the referee folder, as written.
    rest: &'a str,
    /// The layout subfolder it comes from, as written; `""` for the
    /// folder's root and any other subfolder.
    origin: &'a str,
    /// Its path below the referee folder once folded.
    target: &'a str,
}

impl<'a> Placed<'a> {
    fn new(input: &'a str, rest: &'a str) -> Placed<'a> {
        match rest.split_once('/') {
            Some((origin, target)) if is_layout_subfolder(origin) => Placed {
                input,
                rest,
                origin,
                target,
            },
            _ => Placed {
                input,
                rest,
                origin: "",
                target: rest,
            },
        }
    }

    /// The target's file name.
    fn file_name(&self) -> &'a str {
        self.target.rsplit('/').next().unwrap_or(self.target)
    }
}

/// An `unmigrated_content` finding on the input file `path`, `reason` the
/// cause and `other` the other candidate's input path when two candidates
/// claim one place.
fn file_finding(
    path: &str,
    disposition: Disposition,
    reason: UnmigratedReason,
    other: Option<String>,
) -> Message {
    unmigrated(
        Scope::File {
            export_id: INPUT,
            path: path_scope(path),
        },
        disposition,
        reason,
        other,
        vec![],
    )
}

/// Whether the files at `a` and `b` hold the same bytes; a pair that does not
/// read is not equal.
fn same_bytes(
    read: &mut impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
    a: &str,
    b: &str,
) -> bool {
    matches!((read(a), read(b)), (Ok(a), Ok(b)) if a == b)
}

/// Step 12 on the referee folder `Players/<name>/`, its files (input path,
/// path below the folder) in input order: folded into its root, or, on a
/// nested layout subfolder or a clash, left as it was (the module doc's
/// rules).
fn flatten(
    name: &str,
    folder_files: &[(&str, &str)],
    read: &mut impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
    plan: &mut PlayerFolders,
) {
    // Step 10's stray rule first: a classify-unknown file no package read
    // is a leftover, not a file the fold migrates or clashes over.
    let (strays, placed): (Vec<Placed>, Vec<Placed>) = folder_files
        .iter()
        .map(|(input, rest)| Placed::new(input, rest))
        .partition(|placed| is_stray(placed.file_name(), None));
    plan.leftovers
        .extend(strays.iter().map(|placed| placed.input.to_owned()));
    // The findings that leave the folder as it was, each with the input path
    // it sorts by.
    let mut refusals = nested_layouts(&placed);
    // The models a `face.xml` of the folder names, wherever it stands.
    let mut named = BTreeSet::new();
    for file in &placed {
        let file_name = file.rest.rsplit('/').next().unwrap_or(file.rest);
        if vtree::fold_name(file_name) == "face.xml" {
            named.extend(named_paths(file.input, read));
        }
    }
    let mut marker = false;
    let mut unnamed_models = Vec::new();
    let mut groups: BTreeMap<String, Vec<&Placed>> = BTreeMap::new();
    for file in &placed {
        let folded = vtree::fold_name(file.target);
        if !file.target.contains('/')
            && classify(file.target) == FileKind::Marker(Marker::OldIngameFace)
        {
            marker = true;
        } else if vtree::fold_name(file.origin) == "common"
            && matches!(classify(file.file_name()), FileKind::Model(_))
            && !named.contains(&vtree::fold_name(file.file_name()))
        {
            unnamed_models.push(file_finding(
                file.input,
                Disposition::DropFile,
                UnmigratedReason::ModelUnreferenced,
                None,
            ));
        } else {
            groups.entry(folded).or_default().push(file);
        }
    }
    let mut copies = Vec::new();
    for members in groups.values() {
        let (first, others) = members
            .split_first()
            .expect("a group is made with its first member");
        if others
            .iter()
            .all(|other| same_bytes(read, first.input, other.input))
        {
            copies.push((
                first.input.to_owned(),
                format!("Players/{name}/{}", first.target),
            ));
        } else {
            for member in members {
                // Two candidates for the one folded name: `other` is the
                // other file's input path (the first different member).
                let other = members
                    .iter()
                    .find(|other| other.input != member.input)
                    .map(|other| other.input.to_owned());
                refusals.push((
                    member.input,
                    file_finding(
                        member.input,
                        Disposition::Keep,
                        UnmigratedReason::NameTaken,
                        other,
                    ),
                ));
            }
        }
    }
    stem_clashes(&groups, &mut refusals);
    plan.consumed
        .extend(placed.iter().map(|file| file.input.to_owned()));
    if refusals.is_empty() {
        copies.sort();
        plan.copies.extend(copies);
        if marker {
            plan.markers
                .push(format!("Players/{name}/ingame_face.toml"));
        }
        plan.findings.extend(unnamed_models);
        return;
    }
    // Left as it was: every file at its old path, the markers and the
    // unnamed models included, and only the refusals reported.
    refusals.sort_by_key(|(key, _)| *key);
    plan.findings
        .extend(refusals.into_iter().map(|(_, message)| message));
    plan.untouched.extend(placed.iter().map(|file| {
        (
            file.input.to_owned(),
            format!("Players/{name}/{}", file.rest),
        )
    }));
}

/// One finding per layout subfolder nested in another (`face/face/`,
/// `common/boots/`), on that folder's input path, which it sorts by.
fn nested_layouts<'a>(placed: &[Placed<'a>]) -> Vec<(&'a str, Message)> {
    let mut nested = BTreeSet::new();
    let mut findings = Vec::new();
    for file in placed {
        if file.origin.is_empty() {
            continue;
        }
        let Some((segment, _)) = file.target.split_once('/') else {
            continue;
        };
        if !is_layout_subfolder(segment) {
            continue;
        }
        // The input path up to the nested segment: `input` ends with
        // `target`, which starts with it.
        let folder = &file.input[..file.input.len() - file.target.len() + segment.len()];
        if nested.insert(folder) {
            findings.push((
                folder,
                unmigrated(
                    Scope::Folder {
                        export_id: INPUT,
                        path: path_scope(folder),
                    },
                    Disposition::Keep,
                    UnmigratedReason::NestedLayout,
                    None,
                    vec![],
                ),
            ));
        }
    }
    findings
}

/// The stem clashes the fold makes, into `refusals`: the textures among the
/// `groups`' targets keyed by folded parent directory and folded stem, two
/// or more targets under one key coming from two or more locations (the
/// folder's root counting as one), each input reported unless it already
/// is. One key from one location alone is the compile's
/// `texture_stem_conflict`, as for any folder.
fn stem_clashes<'a>(
    groups: &BTreeMap<String, Vec<&Placed<'a>>>,
    refusals: &mut Vec<(&'a str, Message)>,
) {
    // Per key: how many targets, and every input of their groups.
    let mut stems: BTreeMap<(String, String), (usize, Vec<&Placed<'a>>)> = BTreeMap::new();
    for members in groups.values() {
        let target = members[0].target;
        let (parent, file_name) = target.rsplit_once('/').unwrap_or(("", target));
        if classify(file_name) != FileKind::Texture {
            continue;
        }
        let stem = file_name
            .rsplit_once('.')
            .map_or(file_name, |(stem, _)| stem);
        let entry = stems
            .entry((vtree::fold_name(parent), vtree::fold_name(stem)))
            .or_default();
        entry.0 += 1;
        entry.1.extend(members.iter().copied());
    }
    for (targets, files) in stems.values() {
        let origins: BTreeSet<String> = files
            .iter()
            .map(|file| vtree::fold_name(file.origin))
            .collect();
        if *targets < 2 || origins.len() < 2 {
            continue;
        }
        for file in files {
            if !refusals.iter().any(|(input, _)| *input == file.input) {
                // `other` names a file from the other location the stem
                // clashes over.
                let other = files
                    .iter()
                    .find(|other| {
                        other.input != file.input
                            && vtree::fold_name(other.origin) != vtree::fold_name(file.origin)
                    })
                    .map(|other| other.input.to_owned());
                refusals.push((
                    file.input,
                    file_finding(
                        file.input,
                        Disposition::Keep,
                        UnmigratedReason::StemTaken,
                        other,
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use studio_core::Scope;

    use super::*;

    #[test]
    fn players_txt_writes_each_parsed_line_canonical() {
        assert_eq!(
            players_txt(b"\xEF\xBB\xBF1 germanbro\r\n2\tdonte\r\n\r\n 07 x \n").as_deref(),
            Some("01 germanbro\n02 donte\n07 x\n")
        );
    }

    #[test]
    fn players_txt_keeps_a_line_that_does_not_parse_as_written() {
        assert_eq!(
            players_txt(b"foo bar\n03\n135 x\n").as_deref(),
            Some("foo bar\n03\n135 x\n")
        );
    }

    #[test]
    fn players_txt_writes_one_space_however_many_the_name_has_before_it() {
        assert_eq!(
            players_txt(b"1   germanbro\n").as_deref(),
            Some("01 germanbro\n")
        );
    }

    #[test]
    fn players_txt_of_bytes_that_are_not_utf8_is_none() {
        assert_eq!(players_txt(b"\xFF\xFE"), None);
    }

    #[test]
    fn referee_folders_copies_the_marker_and_the_folders_and_consumes_the_roster() {
        let files: Vec<String> = [
            "players/arbiter/boots.fmdl",
            "Players/readme.txt",
            "Refs.txt",
            "REF_MARKER.DDS",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(find_refs_txt(&files), Some("Refs.txt"));
        let plan = referee_folders(&files, |path| {
            Err(SourceFailure {
                path: path.to_owned(),
                error: "not read".to_owned(),
            })
        });
        assert_eq!(
            plan.copies,
            vec![
                ("REF_MARKER.DDS".to_owned(), "REF_MARKER.DDS".to_owned()),
                (
                    "players/arbiter/boots.fmdl".to_owned(),
                    "Players/arbiter/boots.fmdl".to_owned()
                ),
            ]
        );
        assert_eq!(
            plan.consumed.iter().map(String::as_str).collect::<Vec<_>>(),
            ["REF_MARKER.DDS", "Refs.txt", "players/arbiter/boots.fmdl"]
        );
        assert!(plan.findings.is_empty());
    }

    /// `referee_folders` over `files` (sorted, as the source lists them),
    /// with `bytes` as the reads' answers: each input path to its bytes; a
    /// path not listed does not read.
    fn plan(files: &[&str], bytes: &[(&str, &[u8])]) -> PlayerFolders {
        let mut files: Vec<String> = files.iter().map(|file| (*file).to_owned()).collect();
        files.sort();
        let bytes_of = |path: &str| {
            bytes
                .iter()
                .find(|(listed, _)| *listed == path)
                .map(|(_, bytes)| *bytes)
        };
        referee_folders(&files, |path| {
            bytes_of(path)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| SourceFailure {
                    path: path.to_owned(),
                    error: "not listed".to_owned(),
                })
        })
    }

    /// `pairs` as owned (input, output) pairs, for comparing with `PlayerFolders::copies`.
    fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
            .collect()
    }

    /// Every finding as `<scope kind> <path> <disposition> (<context>)`, each
    /// checked to be `unmigrated_content`.
    fn findings(plan: &PlayerFolders) -> Vec<String> {
        plan.findings
            .iter()
            .map(|message| {
                assert_eq!(message.code.code.as_ref(), "unmigrated_content");
                let (kind, path) = match &message.scope {
                    Scope::File { path, .. } => ("file", path.as_str()),
                    Scope::Folder { path, .. } => ("folder", path.as_str()),
                    Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => ("other", ""),
                };
                let context = message
                    .context
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{kind} {path} {:?} ({context})", message.disposition)
            })
            .collect()
    }

    #[test]
    fn the_four_subfolders_fold_into_the_root_and_others_stay() {
        let plan = plan(
            &[
                "Players/a/face/face_diff.bin",
                "Players/a/boots/boots.fmdl",
                "Players/a/gloves/glove_l.fmdl",
                "Players/a/common/kit/sock.dds",
                "Players/a/Common_shared/y.dds",
                "Players/a/alt/x.fmdl",
                "Players/a/root.txt",
            ],
            &[],
        );
        assert_eq!(
            plan.copies,
            pairs(&[
                (
                    "Players/a/Common_shared/y.dds",
                    "Players/a/Common_shared/y.dds"
                ),
                ("Players/a/alt/x.fmdl", "Players/a/alt/x.fmdl"),
                ("Players/a/boots/boots.fmdl", "Players/a/boots.fmdl"),
                ("Players/a/common/kit/sock.dds", "Players/a/kit/sock.dds"),
                ("Players/a/face/face_diff.bin", "Players/a/face_diff.bin"),
                ("Players/a/gloves/glove_l.fmdl", "Players/a/glove_l.fmdl"),
                ("Players/a/root.txt", "Players/a/root.txt"),
            ])
        );
        assert!(plan.untouched.is_empty());
        assert!(plan.findings.is_empty());
        assert_eq!(plan.consumed.len(), 7);
    }

    #[test]
    fn one_name_with_equal_bytes_in_two_subfolders_is_one_copy_from_the_first() {
        let plan = plan(
            &[
                "Players/a/boots/arbiter.dds",
                "Players/a/common/arbiter.dds",
            ],
            &[
                ("Players/a/boots/arbiter.dds", b"a"),
                ("Players/a/common/arbiter.dds", b"a"),
            ],
        );
        assert_eq!(
            plan.copies,
            pairs(&[("Players/a/boots/arbiter.dds", "Players/a/arbiter.dds")])
        );
        assert!(plan.findings.is_empty());
        assert!(plan.consumed.contains("Players/a/common/arbiter.dds"));
    }

    #[test]
    fn one_name_with_different_bytes_leaves_the_folder_as_it_was() {
        let files = [
            "Players/a/boots/arbiter.dds",
            "Players/a/boots/boots.fmdl",
            "Players/a/common/arbiter.dds",
            "Players/a/ingame_face",
        ];
        let plan = plan(
            &files,
            &[
                ("Players/a/boots/arbiter.dds", b"a"),
                ("Players/a/common/arbiter.dds", b"b"),
            ],
        );
        assert_eq!(
            findings(&plan),
            [
                "file Players/a/boots/arbiter.dds Keep (reason=name_taken, other=Players/a/common/arbiter.dds)",
                "file Players/a/common/arbiter.dds Keep (reason=name_taken, other=Players/a/boots/arbiter.dds)",
            ]
        );
        assert!(plan.copies.is_empty());
        assert!(plan.markers.is_empty(), "the marker is not normalized");
        assert_eq!(plan.untouched, pairs(&files.map(|file| (file, file))));
        assert_eq!(plan.consumed.len(), 4);
    }

    #[test]
    fn names_differing_in_letter_case_are_one_name() {
        let plan = plan(
            &["Players/a/face/Skin.dds", "Players/a/common/skin.dds"],
            &[
                ("Players/a/face/Skin.dds", b"s"),
                ("Players/a/common/skin.dds", b"s"),
            ],
        );
        // The first by input path is `common/`'s: `c` sorts before `f`.
        assert_eq!(
            plan.copies,
            pairs(&[("Players/a/common/skin.dds", "Players/a/skin.dds")])
        );
        assert!(plan.findings.is_empty());
    }

    #[test]
    fn a_nested_subfolder_of_the_four_names_leaves_the_folder_as_it_was() {
        let files = [
            "players/a/boots/boots.fmdl",
            "players/a/face/face/x.fmdl",
            "players/a/face/face/y.dds",
        ];
        let plan = plan(&files, &[]);
        assert_eq!(
            findings(&plan),
            ["folder players/a/face/face Keep (reason=nested_layout)"]
        );
        assert!(plan.copies.is_empty());
        assert_eq!(
            plan.untouched,
            pairs(&[
                ("players/a/boots/boots.fmdl", "Players/a/boots/boots.fmdl"),
                ("players/a/face/face/x.fmdl", "Players/a/face/face/x.fmdl"),
                ("players/a/face/face/y.dds", "Players/a/face/face/y.dds"),
            ])
        );
    }

    #[test]
    fn one_stem_from_two_locations_is_a_clash_and_from_one_is_not() {
        let across = plan(
            &["Players/a/face/skin.dds", "Players/a/common/skin.png"],
            &[],
        );
        assert_eq!(
            findings(&across),
            [
                "file Players/a/common/skin.png Keep (reason=stem_taken, other=Players/a/face/skin.dds)",
                "file Players/a/face/skin.dds Keep (reason=stem_taken, other=Players/a/common/skin.png)",
            ]
        );
        assert!(across.copies.is_empty());
        assert_eq!(across.untouched.len(), 2);

        let within = plan(&["Players/a/face/skin.dds", "Players/a/face/skin.png"], &[]);
        assert!(within.findings.is_empty());
        assert_eq!(
            within.copies,
            pairs(&[
                ("Players/a/face/skin.dds", "Players/a/skin.dds"),
                ("Players/a/face/skin.png", "Players/a/skin.png"),
            ])
        );

        let same = plan(
            &["Players/a/face/skin.dds", "Players/a/common/skin.dds"],
            &[
                ("Players/a/face/skin.dds", b"s"),
                ("Players/a/common/skin.dds", b"s"),
            ],
        );
        assert!(same.findings.is_empty());
        assert_eq!(same.copies.len(), 1);
    }

    #[test]
    fn either_ingame_face_spelling_anywhere_is_one_marker() {
        let plan = plan(
            &[
                "Players/a/face/ingame_face.txt",
                "Players/a/INGAME_FACE",
                "Players/a/boots/boots.fmdl",
            ],
            &[],
        );
        assert_eq!(plan.markers, ["Players/a/ingame_face.toml"]);
        assert_eq!(
            plan.copies,
            pairs(&[("Players/a/boots/boots.fmdl", "Players/a/boots.fmdl")])
        );
        assert_eq!(plan.consumed.len(), 3);
    }

    #[test]
    fn a_common_model_no_face_xml_names_is_left_out() {
        let unnamed = plan(
            &[
                "Players/a/common/parts_body.model",
                "Players/a/face/face_diff.bin",
            ],
            &[],
        );
        assert_eq!(
            findings(&unnamed),
            ["file Players/a/common/parts_body.model DropFile (reason=model_unreferenced)"]
        );
        assert_eq!(
            unnamed.copies,
            pairs(&[("Players/a/face/face_diff.bin", "Players/a/face_diff.bin")])
        );
        assert!(
            unnamed
                .consumed
                .contains("Players/a/common/parts_body.model")
        );

        let xml = b"<config><model path=\"model/character/uniform/common/XXX/parts_body.model\"/></config>";
        let named = plan(
            &[
                "Players/a/common/parts_body.model",
                "Players/a/face/face.xml",
            ],
            &[("Players/a/face/face.xml", xml)],
        );
        assert!(named.findings.is_empty());
        assert_eq!(
            named.copies,
            pairs(&[
                (
                    "Players/a/common/parts_body.model",
                    "Players/a/parts_body.model"
                ),
                ("Players/a/face/face.xml", "Players/a/face.xml"),
            ])
        );

        let starred = plan(
            &[
                "Players/a/common/oral_x_win32.model",
                "Players/a/face/FACE.XML",
            ],
            &[(
                "Players/a/face/FACE.XML",
                b"<config><model path=\"./oral_x_*.model\"/></config>",
            )],
        );
        assert!(starred.findings.is_empty());
        assert_eq!(starred.copies.len(), 2);

        let material = plan(&["Players/a/common/x.mtl"], &[]);
        assert!(material.findings.is_empty());
        assert_eq!(
            material.copies,
            pairs(&[("Players/a/common/x.mtl", "Players/a/x.mtl")])
        );
    }

    #[test]
    fn find_refs_txt_prefers_refs_txt_then_the_older_spelling() {
        let files = |names: &[&str]| {
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            find_refs_txt(&files(&["Refs list.txt", "refs.txt"])),
            Some("refs.txt")
        );
        assert_eq!(
            find_refs_txt(&files(&["Refs list.txt"])),
            Some("Refs list.txt")
        );
    }

    #[test]
    fn the_refs_compilers_root_folders_are_the_referee_folders() {
        // No `Players/`: `boston` is a referee folder, folded like one;
        // the root `Common/` and `Other/` are not (step 1 knows them).
        let plan = plan(
            &[
                "Refs list.txt",
                "boston/Face/face.xml",
                "boston/Common/kwt.dds",
                "Common/shared.dds",
                "Other/readme.txt",
            ],
            &[("boston/Face/face.xml", b"<config/>")],
        );
        assert_eq!(
            plan.copies,
            pairs(&[
                ("boston/Common/kwt.dds", "Players/boston/kwt.dds"),
                ("boston/Face/face.xml", "Players/boston/face.xml"),
            ])
        );
        for consumed in [
            "Refs list.txt",
            "boston/Common/kwt.dds",
            "boston/Face/face.xml",
        ] {
            assert!(plan.consumed.contains(consumed), "{consumed}");
        }
        assert!(!plan.consumed.contains("Common/shared.dds"));
        assert!(!plan.consumed.contains("Other/readme.txt"));
        assert!(plan.findings.is_empty());
    }

    #[test]
    fn a_players_path_means_root_folders_are_not_referee_folders() {
        let plan = plan(
            &[
                "Refs list.txt",
                "Players/eiki/boots.fmdl",
                "boston/Face/x.model",
            ],
            &[],
        );
        assert_eq!(
            plan.copies,
            pairs(&[("Players/eiki/boots.fmdl", "Players/eiki/boots.fmdl")])
        );
        assert!(!plan.consumed.contains("boston/Face/x.model"));
    }

    #[test]
    fn a_stray_in_a_referee_folder_is_a_leftover() {
        // Step 10's stray rule, inside the fold too: the backup file is no
        // `.txt` note and no kind `classify` knows, so it is a leftover —
        // not a file the fold migrates or clashes over.
        let plan = plan(
            &["Players/a/boots.fmdl", "Players/a/face/face.mtl.bak"],
            &[],
        );
        assert_eq!(
            plan.copies,
            pairs(&[("Players/a/boots.fmdl", "Players/a/boots.fmdl")])
        );
        assert!(plan.leftovers.contains("Players/a/face/face.mtl.bak"));
        assert!(!plan.consumed.contains("Players/a/face/face.mtl.bak"));
        assert!(plan.findings.is_empty());
    }

    #[test]
    fn a_stem_clash_names_a_file_of_the_other_location_not_its_sibling() {
        // A clash group with two files of one location beside one of
        // another: `face/` holds `arbiter.dds` and `arbiter.png`, the root
        // an `arbiter.dds` of other bytes. Each `face/` file's `stem_taken`
        // names the root file as `other`, never its `face/` sibling. The
        // inputs are given in the order that puts the `face/` file first
        // among the stem's other-input files (`referee_folders` iterates the
        // input's order; the `plan` helper's sort would put the root file
        // first and never show a wrong choice).
        let files: Vec<String> = [
            "Players/arbiter/face/arbiter.dds",
            "Players/arbiter/face/arbiter.png",
            "Players/arbiter/arbiter.dds",
        ]
        .map(str::to_owned)
        .to_vec();
        let plan = referee_folders(&files, |path| {
            Ok(if path == "Players/arbiter/arbiter.dds" {
                b"root".to_vec()
            } else {
                b"face".to_vec()
            })
        });
        assert_eq!(
            findings(&plan),
            [
                "file Players/arbiter/arbiter.dds Keep (reason=name_taken, other=Players/arbiter/face/arbiter.dds)",
                "file Players/arbiter/face/arbiter.dds Keep (reason=name_taken, other=Players/arbiter/arbiter.dds)",
                "file Players/arbiter/face/arbiter.png Keep (reason=stem_taken, other=Players/arbiter/arbiter.dds)",
            ]
        );
        assert!(plan.copies.is_empty());
    }
}
