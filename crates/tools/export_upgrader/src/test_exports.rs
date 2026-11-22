//! Clean old exports for tests. A test writes the old files it is about, then `complete_old`
//! adds what the export still lacks for its upgrade to be clean, one rule per
//! `OldRequirement` in `old_missing`. A test of one of those rules turns it off with `without`.

use std::fs;
use std::path::Path;

use crate::carried::under;
use crate::naming::coverage_choice;

/// What a clean old export holds for its upgrade to be clean, beyond the files a test gives
/// it. A test of the check that asks for one turns it off with `without`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldRequirement {
    /// Some content. When no file or empty folder sits below a root folder other than `Other/`
    /// and `Logo/`, the face folder `Faces/XXX05 - A/` holding `FACE_FILES`; for a referee
    /// export, `Players/arbiter/boots.fmdl` and a `refs.txt` of `01 arbiter`. Nothing when
    /// `Logo` adds the logo, which alone is content: `without(Content)` alone then stays clean,
    /// and a test of an empty `Full` export turns `Logo` off too.
    Content,
    /// A team export's logo (`logo_missing`). When an upgrade with no `--coverage` tags the
    /// export `Full`, as its name decides (`coverage_choice`), and no file sits in `Logo/`,
    /// `Logo/emblem_0XXX_r_ll.png`: the `co Spring` fixture's 512 x 512 logo.
    Logo,
}

/// The four files of the tracer's Fox face folder, a few bytes each: the upgrader reads a face
/// folder's names, not its formats.
pub const FACE_FILES: [(&str, &[u8]); 4] = [
    ("fcl_hair.fmdl", b"hair"),
    ("face_diff.bin", b"diff"),
    ("shirt.dds", b"shirt"),
    ("portrait.dds", b"portrait"),
];

/// Where `OldRequirement::Logo` puts the logo, in the export and in the `co Spring` fixture.
const LOGO: &str = "Logo/emblem_0XXX_r_ll.png";

/// The files the old export `name` lacks for its upgrade to be clean, given `files` and
/// `folders`, the files and the empty folders it holds (relative to its root,
/// `/`-separated), with the requirements in `without` left out: (path, bytes), sorted by
/// path. The one place an old export's requirement is written.
pub fn old_missing(
    name: &str,
    files: &[&str],
    folders: &[&str],
    without: &[OldRequirement],
) -> Vec<(String, Vec<u8>)> {
    let referee = aesthetics_export::team_name(name).is_some_and(|team| team.is_referees());
    let mut added: Vec<(String, Vec<u8>)> = Vec::new();
    let full = coverage_choice(name, None).is_ok_and(|choice| choice.tag == "Full");
    let has_logo = files.iter().any(|path| under(path, "logo").is_some());
    let adds_logo = !referee && full && !without.contains(&OldRequirement::Logo) && !has_logo;
    if adds_logo {
        let logo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/co Spring")
            .join(LOGO);
        added.push((LOGO.to_owned(), fs::read(logo).unwrap()));
    }
    // A test's own `Logo/` file may be one the upgrade drops, so only the added logo counts.
    let has_content = adds_logo || files.iter().chain(folders).any(|path| is_content(path));
    if !without.contains(&OldRequirement::Content) && !has_content {
        if referee {
            added.push(("refs.txt".to_owned(), b"01 arbiter\n".to_vec()));
            added.push(("Players/arbiter/boots.fmdl".to_owned(), b"boots".to_vec()));
        } else {
            for (file, bytes) in FACE_FILES {
                added.push((format!("Faces/XXX05 - A/{file}"), bytes.to_vec()));
            }
        }
    }
    // A default file never replaces one the test gave (a referee export's own `refs.txt`).
    added.retain(|(path, _)| !files.iter().any(|given| given.eq_ignore_ascii_case(path)));
    added.sort();
    added
}

/// Whether `path` is content: below a root folder other than `Other/` and `Logo/`, whose
/// files the upgrade drops or turns into the logo alone.
fn is_content(path: &str) -> bool {
    path.split_once('/').is_some_and(|(top, _)| {
        !top.eq_ignore_ascii_case("other") && !top.eq_ignore_ascii_case("logo")
    })
}

/// A Team Note naming `team` (`/co/`): its `Team:` line.
pub fn note(team: &str) -> String {
    format!("Team: {team}\r\n")
}

/// Writes into the old export folder `folder` the files `old_missing` says it lacks, from what
/// the folder holds and its name; the requirements in `without` add nothing. A test writes the
/// files it is about first, then calls this once per export.
pub fn complete_old(folder: &Path, without: &[OldRequirement]) {
    fs::create_dir_all(folder).unwrap();
    let (mut files, mut folders) = (Vec::new(), Vec::new());
    list_under(folder, "", &mut files, &mut folders);
    let files: Vec<&str> = files.iter().map(String::as_str).collect();
    let folders: Vec<&str> = folders.iter().map(String::as_str).collect();
    let name = folder.file_name().unwrap().to_str().unwrap();
    for (path, bytes) in old_missing(name, &files, &folders, without) {
        let path = folder.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

/// Adds every file under `folder` to `files` and every empty folder to `folders`, as
/// `/`-separated paths that start with `prefix`.
fn list_under(folder: &Path, prefix: &str, files: &mut Vec<String>, folders: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        let path = format!("{prefix}{}", entry.file_name().to_str().unwrap());
        if !entry.path().is_dir() {
            files.push(path);
        } else if fs::read_dir(entry.path()).unwrap().next().is_some() {
            list_under(&entry.path(), &format!("{path}/"), files, folders);
        } else {
            folders.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use studio_core::PipelineEvent;

    use super::*;
    use crate::cli::UpgradeArgs;
    use crate::test_support::{scratch, test_ctx};
    use crate::upgrade::run;

    /// The exit code and the findings, each `<severity> <code>`, of an upgrade of the old
    /// export `name` (a team export `--loose`), when it holds only what `complete_old` adds.
    fn upgrade_default(name: &str, without: &[OldRequirement]) -> (u8, Vec<String>) {
        let root = scratch(&format!("default {name} {without:?}"));
        let export = root.join(name);
        complete_old(&export, without);
        let (ctx, events_rx) = test_ctx();
        let referee = name.starts_with("refs ");
        let args = UpgradeArgs {
            exports: vec![export],
            all: None,
            savefile: None,
            loose: !referee,
            coverage: None,
            output: None,
        };
        let exit = run(&args, &ctx).unwrap();
        drop(ctx);
        let findings = events_rx
            .iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::Message(message) => {
                    Some(format!("{:?} {}", message.severity, message.code.code))
                }
                _ => None,
            })
            .collect();
        (exit, findings)
    }

    /// The upgrade of each default, as (name, exit code, findings), `without` applied.
    fn upgrades(
        cases: &[(&'static str, &[OldRequirement])],
    ) -> Vec<(&'static str, u8, Vec<String>)> {
        cases
            .iter()
            .map(|(name, without)| {
                let (exit, findings) = upgrade_default(name, without);
                (*name, exit, findings)
            })
            .collect()
    }

    #[test]
    fn each_old_default_upgrades_clean() {
        let team = vec![
            "Info coverage_tag_chosen".to_owned(),
            "Info migration_clean".to_owned(),
        ];
        assert_eq!(
            upgrades(&[
                ("co Spring", &[]),
                ("co midcup Day 5", &[]),
                ("refs Spring", &[])
            ]),
            [
                ("co Spring", 0, team.clone()),
                ("co midcup Day 5", 0, team),
                ("refs Spring", 0, vec!["Info migration_clean".to_owned()]),
            ]
        );
    }

    #[test]
    fn each_old_default_holds_its_requirements_files() {
        let mut face: Vec<String> = FACE_FILES
            .iter()
            .map(|(file, _)| format!("Faces/XXX05 - A/{file}"))
            .collect();
        face.sort();
        let referee = vec![
            "Players/arbiter/boots.fmdl".to_owned(),
            "refs.txt".to_owned(),
        ];
        for (name, expected) in [
            ("co Spring", vec![LOGO.to_owned()]),
            ("co midcup Day 5", face),
            ("refs Spring", referee),
        ] {
            let paths: Vec<String> = old_missing(name, &[], &[], &[])
                .into_iter()
                .map(|(path, _)| path)
                .collect();
            assert_eq!(paths, expected, "{name}");
        }
    }

    #[test]
    fn each_requirement_turned_off_gives_its_finding() {
        let refused = |code: &str| {
            vec![
                "Info coverage_tag_chosen".to_owned(),
                format!("Error {code}"),
                "Info migration_not_clean".to_owned(),
            ]
        };
        // An empty folder is no old export, so the run stops before it reads one.
        let empty = vec!["Error no_old_export".to_owned()];
        assert_eq!(
            upgrades(&[
                ("co Spring", &[OldRequirement::Logo]),
                ("co Spring", &[OldRequirement::Content]),
                ("co midcup Day 5", &[OldRequirement::Content]),
                ("refs Spring", &[OldRequirement::Content]),
            ]),
            [
                ("co Spring", 1, refused("logo_missing")),
                // The added logo alone is content.
                (
                    "co Spring",
                    0,
                    vec![
                        "Info coverage_tag_chosen".to_owned(),
                        "Info migration_clean".to_owned(),
                    ]
                ),
                ("co midcup Day 5", 1, empty.clone()),
                ("refs Spring", 1, empty),
            ]
        );
    }
}
