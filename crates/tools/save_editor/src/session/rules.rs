//! The AATF ruleset `check-aatf` and the window share, so the quick
//! actions, the Cards left badge and Live AATF read a player as the
//! command line does: the `aatf_ruleset` setting resolved one way for
//! both (`resolve`) against the built-in rulesets and the ones imported
//! into the data folder's `rules/` (`imported`), which the window's import
//! (`check_import`, `import`) and × (`remove`) change. `team_views`/
//! `check_teams` are the check both the command line and Live AATF run.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use aatf::{LoadError, Rules, TeamView, Violation};
use anyhow::anyhow;
use pes_savefile::EditFile;
use rayon::prelude::*;

/// One team's AATF check: its violations, or the error text (a team view
/// or the script).
pub(crate) type TeamCheck = Result<Vec<Violation>, String>;

/// The first built-in ruleset's stem: the setting's default, and the
/// ruleset a choice that is gone falls back to.
pub(crate) const FIRST_BUILTIN: &str = "autumn26";

/// The data folder's subfolder the imported rulesets' copies live in.
const RULES_FOLDER: &str = "rules";

/// An imported ruleset: a rules file in the data folder's `rules/` that
/// loads.
pub(crate) struct Imported {
    /// The copy's file name (`cup.4ccrules`): the `aatf_ruleset` value
    /// that picks it.
    pub(crate) file_name: String,
    /// The copy, loaded.
    pub(crate) rules: Arc<Rules>,
}

/// What the `aatf_ruleset` setting resolved to.
pub(crate) struct Choice {
    /// The ruleset, loaded.
    pub(crate) rules: Arc<Rules>,
    /// The built-in stem or the imported file name the rules came from:
    /// the setting itself, or the first built-in stem when what it named
    /// is gone.
    pub(crate) key: String,
    /// The warning to show when the setting named a ruleset that is gone
    /// and the first built-in one stands in for it.
    pub(crate) notice: Option<String>,
}

/// A rules file picked for import, checked before anything is written.
pub(crate) enum ImportCheck {
    /// The file does not read or does not load: one line per finding,
    /// each naming its line and column or its key path.
    Errors(Vec<String>),
    /// The file's description is the built-in ruleset's of that
    /// description: two rows would read the same, and a built-in one is
    /// never replaced.
    BuiltinName(String),
    /// The file loads: `import` copies it.
    Clean(Import),
}

/// A rules file that loads, ready to be copied into `rules/`.
pub(crate) struct Import {
    /// The copy's file name: the picked file's, ending in `.4ccrules` so
    /// that `imported` lists it, or an import's whose name differs from
    /// that only in case.
    pub(crate) file_name: String,
    /// The file's description (its `name`).
    pub(crate) description: String,
    /// The earlier imports the copy replaces, each its description and
    /// file name: the one of the same file name and the one of the same
    /// description.
    pub(crate) replaces: Vec<(String, String)>,
    /// The text as read and loaded, which the copy is: the file is not
    /// read again, so a confirmation's wait cannot copy another text than
    /// the one checked.
    text: String,
}

/// The built-in rulesets' stems and descriptions, in the templates' order,
/// read once: `aatf::templates` parses every embedded block, and the list
/// never changes while the binary runs.
pub(crate) fn builtins() -> &'static [(&'static str, String)] {
    static BUILTINS: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    BUILTINS.get_or_init(|| {
        aatf::templates()
            .into_iter()
            .map(|template| (template.stem, template.ruleset.name))
            .collect()
    })
}

/// The rules file at `path` checked for import against the `imported`
/// rulesets, as `Rules::load` loads it. Writes nothing.
pub(crate) fn check_import(path: &Path, imported: &[Imported]) -> ImportCheck {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return ImportCheck::Errors(vec![format!("cannot read the file: {error}")]),
    };
    let rules = match Rules::load(&text) {
        Ok(rules) => rules,
        Err(LoadError::Rules(errors)) => {
            return ImportCheck::Errors(errors.iter().map(ToString::to_string).collect());
        }
        Err(error) => return ImportCheck::Errors(vec![error.to_string()]),
    };
    let description = rules.ruleset().name.clone();
    if builtins().iter().any(|(_, name)| *name == description) {
        return ImportCheck::BuiltinName(description);
    }
    let picked_name = path
        .file_name()
        .expect("a path that read as a file ends in its file name");
    let picked_name = Path::new(picked_name)
        .with_extension(aatf::RULES_FILE_EXTENSION)
        .to_string_lossy()
        .into_owned();
    // Windows file names ignore case, so two names differing only in case
    // are one file there: writing the pick under its own spelling would
    // overwrite the import in place, and deleting the import by its name
    // would then delete the new copy. The copy takes the import's name.
    let file_name = imported
        .iter()
        .find(|import| import.file_name.eq_ignore_ascii_case(&picked_name))
        .map_or(picked_name, |import| import.file_name.clone());
    let replaces = imported
        .iter()
        .filter(|import| {
            import.file_name == file_name || import.rules.ruleset().name == description
        })
        .map(|import| {
            (
                import.rules.ruleset().name.clone(),
                import.file_name.clone(),
            )
        })
        .collect();
    ImportCheck::Clean(Import {
        file_name,
        description,
        replaces,
        text,
    })
}

/// `import` copied into `data_dir`'s `rules/` (created when missing), then
/// the copies it replaces deleted. The copy is written first, so a write
/// that fails leaves the list as it was.
pub(crate) fn import(data_dir: &Path, import: &Import) -> std::io::Result<()> {
    let folder = data_dir.join(RULES_FOLDER);
    std::fs::create_dir_all(&folder)?;
    std::fs::write(folder.join(&import.file_name), &import.text)?;
    for (_, file_name) in &import.replaces {
        // The copy of the same file name was just overwritten.
        if *file_name != import.file_name {
            std::fs::remove_file(folder.join(file_name))?;
        }
    }
    Ok(())
}

/// The imported copy `file_name` deleted from `data_dir`'s `rules/`.
pub(crate) fn remove(data_dir: &Path, file_name: &str) -> std::io::Result<()> {
    std::fs::remove_file(data_dir.join(RULES_FOLDER).join(file_name))
}

/// The rules file at `path` read and loaded; a file that does not read
/// or load is the error naming it, the text `check-aatf` prints.
pub(crate) fn load_file(path: &Path) -> anyhow::Result<Rules> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow!("{}: cannot read the file: {error}", path.display()))?;
    Rules::load(&text).map_err(|error| anyhow!("{}: {error}", path.display()))
}

/// The imported rulesets: the `.4ccrules` files in `data_dir`'s `rules/`
/// that load, sorted by description. A file that does not read or load is
/// left out with no word to the member, since nobody picked it; a setting
/// naming it falls back in `resolve`, which says so. No data folder, or no
/// `rules/` in it: none.
pub(crate) fn imported(data_dir: Option<&Path>) -> Vec<Imported> {
    let Some(folder) = data_dir.map(|dir| dir.join(RULES_FOLDER)) else {
        return Vec::new();
    };
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) => {
            log::debug!("no imported rulesets in {}: {error}", folder.display());
            return Vec::new();
        }
    };
    let mut imported = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                log::debug!("{}: an entry cannot be read: {error}", folder.display());
                continue;
            }
        };
        if path.extension().and_then(|extension| extension.to_str())
            != Some(aatf::RULES_FILE_EXTENSION)
        {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        match load_file(&path) {
            Ok(rules) => imported.push(Imported {
                file_name: file_name.to_owned(),
                rules: Arc::new(rules),
            }),
            Err(error) => log::debug!("left out of the imported rulesets: {error:#}"),
        }
    }
    imported.sort_by(|a, b| a.rules.ruleset().name.cmp(&b.rules.ruleset().name));
    imported
}

/// The ruleset `setting` picks: a built-in stem's, else the imported file
/// of that name's, else (a copy gone or no longer loading, a typo) the
/// first built-in one with the notice saying so.
pub(crate) fn resolve(setting: &str, imported: &[Imported]) -> Choice {
    if let Some(rules) = aatf::builtin(setting) {
        return Choice {
            rules: Arc::new(rules),
            key: setting.to_owned(),
            notice: None,
        };
    }
    if let Some(import) = imported.iter().find(|import| import.file_name == setting) {
        return Choice {
            rules: Arc::clone(&import.rules),
            key: setting.to_owned(),
            notice: None,
        };
    }
    let rules = aatf::builtin(FIRST_BUILTIN).expect("the first built-in ruleset is built in");
    let notice = format!(
        "The AATF ruleset picked last is gone from the data folder: {} is used instead.",
        rules.ruleset().name
    );
    Choice {
        rules: Arc::new(rules),
        key: FIRST_BUILTIN.to_owned(),
        notice: Some(notice),
    }
}

/// The views of `ids`' teams, built where the `EditFile` lives (the UI
/// thread).
pub(crate) fn team_views(file: &EditFile, ids: &[u32]) -> Vec<Result<TeamView, String>> {
    ids.iter()
        .map(|&id| TeamView::new(file, id).map_err(|error| error.to_string()))
        .collect()
}

/// `views` checked on `rules` in parallel, in order.
pub(crate) fn check_teams(rules: &Rules, views: &[Result<TeamView, String>]) -> Vec<TeamCheck> {
    views
        .par_iter()
        .map(|view| {
            view.as_ref()
                .map_err(Clone::clone)
                .and_then(|view| rules.check_team(view).map_err(|error| error.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `dir`'s `rules/` holding `name`, the official file renamed
    /// `description`.
    fn copy_in(dir: &Path, name: &str, description: &str) {
        let folder = dir.join(RULES_FOLDER);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(name), renamed(description)).unwrap();
    }

    /// The official file with its description `description`.
    fn renamed(description: &str) -> String {
        let text = aatf::official_file().replacen(
            "name: \"4cc Autumn 26\"",
            &format!("name: \"{description}\""),
            1,
        );
        assert!(text.contains(description), "premise: the name was replaced");
        text
    }

    /// `text` written as `dir`'s `picked/<name>`, outside `rules/`: the
    /// file a member picks to import. Its path.
    fn picked(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
        let folder = dir.join("picked");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `check_import`'s clean outcome, or a panic naming the other.
    fn clean(check: ImportCheck) -> Import {
        match check {
            ImportCheck::Clean(import) => import,
            ImportCheck::Errors(findings) => panic!("refused with {findings:?}"),
            ImportCheck::BuiltinName(name) => panic!("refused as the built-in {name}"),
        }
    }

    /// The imported rows of `dir`, each its file name and description.
    fn rows(dir: &Path) -> Vec<(String, String)> {
        imported(Some(dir))
            .iter()
            .map(|import| {
                (
                    import.file_name.clone(),
                    import.rules.ruleset().name.clone(),
                )
            })
            .collect()
    }

    /// `(file name, description)` as owned strings, for comparing rows.
    fn row(file_name: &str, description: &str) -> (String, String) {
        (file_name.to_owned(), description.to_owned())
    }

    /// A built-in stem resolves to its ruleset, no notice; with no data
    /// folder there are no imports, the built-ins alone.
    #[test]
    fn a_builtin_stem_resolves_to_its_ruleset() {
        assert!(imported(None).is_empty(), "no data folder: no imports");
        let choice = resolve("vtl11", &[]);
        assert_eq!(choice.rules.ruleset().name, "/vt/ League 11");
        assert_eq!(choice.key, "vtl11");
        assert_eq!(choice.notice, None);
    }

    /// The `.4ccrules` files of `rules/` that load are the imports, by
    /// description; a file that does not load and a file of another ending
    /// are left out; an imported file name resolves to its copy.
    #[test]
    fn the_imported_files_that_load_are_listed_and_resolve() {
        let dir = crate::test_support::scratch("rules_imported");
        // The descriptions sort the other way round from the file names.
        copy_in(&dir, "alpha.4ccrules", "Zeta Cup");
        copy_in(&dir, "zeta.4ccrules", "Beta Cup");
        copy_in(&dir, "notes.txt", "Text Cup");
        std::fs::write(dir.join(RULES_FOLDER).join("broken.4ccrules"), "not rhai {").unwrap();

        let list = imported(Some(&dir));
        let rows: Vec<(&str, &str)> = list
            .iter()
            .map(|import| {
                (
                    import.file_name.as_str(),
                    import.rules.ruleset().name.as_str(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("zeta.4ccrules", "Beta Cup"),
                ("alpha.4ccrules", "Zeta Cup")
            ]
        );

        let choice = resolve("alpha.4ccrules", &list);
        assert_eq!(choice.rules.ruleset().name, "Zeta Cup");
        assert_eq!(choice.key, "alpha.4ccrules");
        assert_eq!(choice.notice, None);
    }

    /// A setting naming a copy no longer in `rules/` (or one that no
    /// longer loads) falls back to the first built-in ruleset, with the
    /// notice naming it.
    #[test]
    fn a_ruleset_that_is_gone_falls_back_to_the_first_builtin_with_a_notice() {
        let dir = crate::test_support::scratch("rules_gone");
        copy_in(&dir, "cup.4ccrules", "Cup");
        std::fs::write(dir.join(RULES_FOLDER).join("broken.4ccrules"), "not rhai {").unwrap();
        let list = imported(Some(&dir));
        for setting in ["gone.4ccrules", "broken.4ccrules"] {
            let choice = resolve(setting, &list);
            assert_eq!(choice.rules.ruleset().name, "4cc Autumn 26", "{setting}");
            assert_eq!(choice.key, "autumn26", "{setting}");
            assert_eq!(
                choice.notice.as_deref(),
                Some(
                    "The AATF ruleset picked last is gone from the data folder: 4cc Autumn 26 \
                     is used instead."
                ),
                "{setting}"
            );
        }
    }

    /// The loader reads the content, never the ending (`spec/aatf_rules.md`
    /// "The rules file"): a valid file loads whatever its name ends in.
    #[test]
    fn the_rules_file_loads_whatever_it_ends_in() {
        let dir = crate::test_support::scratch("rules_extension_ignored");
        for name in ["rules.4ccrules", "rules.txt"] {
            let path = dir.join(name);
            std::fs::write(&path, aatf::official_file()).unwrap();
            let rules = load_file(&path).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(rules.ruleset().name, "4cc Autumn 26");
        }
    }

    /// A file that does not load is refused with one line per finding,
    /// `LoadError::Rules` fanning out to one line per broken rule and any
    /// other error its own line; a file that does not read is one line too;
    /// nothing is written.
    #[test]
    fn a_file_that_does_not_load_is_refused_with_its_findings_and_nothing_written() {
        let dir = crate::test_support::scratch("import_errors");
        // Two values broken: the tier counts over the squad, and an
        // unknown suggestion word.
        let two = renamed("4cc Autumn 25")
            .replacen(
                "label: \"Gold\", count: 1,",
                "label: \"Gold\", count: 21,",
                1,
            )
            .replacen("suggestions: [\"cards\"", "suggestions: [\"bogus\"", 1);
        let cases = [
            (
                picked(&dir, "two.4ccrules", &two),
                vec![
                    "tiers: counts total 25 over the squad of 23".to_owned(),
                    "suggestions[0]: unknown suggestion \"bogus\"".to_owned(),
                ],
            ),
            (
                picked(&dir, "parse.4ccrules", "let x = ;"),
                vec!["1:9: Unexpected ';' (line 1, position 9)".to_owned()],
            ),
        ];
        for (path, expected) in cases {
            match check_import(&path, &[]) {
                ImportCheck::Errors(findings) => assert_eq!(findings, expected, "{path:?}"),
                _ => panic!("{path:?} was not refused"),
            }
        }
        match check_import(&dir.join("picked").join("none.4ccrules"), &[]) {
            ImportCheck::Errors(findings) => {
                assert_eq!(findings.len(), 1, "{findings:?}");
                assert!(
                    findings[0].starts_with("cannot read the file: "),
                    "{findings:?}"
                );
            }
            _ => panic!("a file that does not read was not refused"),
        }
        assert!(!dir.join(RULES_FOLDER).exists(), "nothing written");
    }

    /// A file whose description a built-in ruleset has is refused naming
    /// it, whichever built-in it is.
    #[test]
    fn a_builtin_description_is_refused_naming_it() {
        let dir = crate::test_support::scratch("import_builtin");
        for description in ["4cc Autumn 26", "/vg/ League 27"] {
            let path = picked(&dir, "cup.4ccrules", &renamed(description));
            match check_import(&path, &[]) {
                ImportCheck::BuiltinName(name) => assert_eq!(name, description),
                _ => panic!("{description} was not refused"),
            }
        }
        assert!(!dir.join(RULES_FOLDER).exists(), "nothing written");
    }

    /// A first import is copied into `rules/`, created when missing, under
    /// its file name and as its text, and listed; a picked file of another
    /// ending is copied ending in `.4ccrules`, the ending the list reads.
    #[test]
    fn a_first_import_is_copied_and_listed() {
        let dir = crate::test_support::scratch("import_first");
        assert!(!dir.join(RULES_FOLDER).exists(), "premise: no rules/ yet");
        let text = renamed("4cc Autumn 25");
        let import_check = clean(check_import(&picked(&dir, "autumn25.4ccrules", &text), &[]));
        assert_eq!(import_check.file_name, "autumn25.4ccrules");
        assert_eq!(import_check.description, "4cc Autumn 25");
        assert!(import_check.replaces.is_empty(), "nothing replaced");
        import(&dir, &import_check).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join(RULES_FOLDER).join("autumn25.4ccrules")).unwrap(),
            text,
            "the copy is the file"
        );
        assert_eq!(rows(&dir), [row("autumn25.4ccrules", "4cc Autumn 25")]);

        let text = picked(&dir, "vgl25.txt", &renamed("/vg/ League 25"));
        let other_ending = clean(check_import(&text, &imported(Some(&dir))));
        assert_eq!(other_ending.file_name, "vgl25.4ccrules");
        import(&dir, &other_ending).unwrap();
        assert_eq!(
            rows(&dir),
            [
                row("vgl25.4ccrules", "/vg/ League 25"),
                row("autumn25.4ccrules", "4cc Autumn 25")
            ]
        );
    }

    /// A file of an import's file name, or of its description, names the
    /// import it replaces before anything is written; once imported, one
    /// row remains, the new file's. One matching two imports, one by each,
    /// replaces both.
    #[test]
    fn an_import_of_an_earlier_ones_name_or_description_replaces_it() {
        // (picked file name, its description)
        let cases = [
            ("autumn25.4ccrules", "4cc Autumn 25 corrected"),
            ("autumn25_fixed.4ccrules", "4cc Autumn 25"),
        ];
        for (i, (file_name, description)) in cases.into_iter().enumerate() {
            let dir = crate::test_support::scratch(&format!("import_replace_{i}"));
            copy_in(&dir, "autumn25.4ccrules", "4cc Autumn 25");
            let before =
                std::fs::read_to_string(dir.join(RULES_FOLDER).join("autumn25.4ccrules")).unwrap();
            let path = picked(&dir, file_name, &renamed(description));
            let replacing = clean(check_import(&path, &imported(Some(&dir))));
            assert_eq!(
                replacing.replaces,
                [row("4cc Autumn 25", "autumn25.4ccrules")],
                "{file_name}"
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(RULES_FOLDER).join("autumn25.4ccrules")).unwrap(),
                before,
                "{file_name}: nothing written before the confirmation"
            );
            assert_eq!(rows(&dir).len(), 1, "{file_name}: nothing written");
            import(&dir, &replacing).unwrap();
            assert_eq!(rows(&dir), [row(file_name, description)], "{file_name}");
        }

        let dir = crate::test_support::scratch("import_replace_two");
        copy_in(&dir, "autumn25.4ccrules", "4cc Autumn 25");
        copy_in(&dir, "vgl25.4ccrules", "/vg/ League 25");
        let path = picked(&dir, "autumn25.4ccrules", &renamed("/vg/ League 25"));
        let replacing = clean(check_import(&path, &imported(Some(&dir))));
        assert_eq!(
            replacing.replaces,
            [
                row("/vg/ League 25", "vgl25.4ccrules"),
                row("4cc Autumn 25", "autumn25.4ccrules")
            ]
        );
        import(&dir, &replacing).unwrap();
        assert_eq!(rows(&dir), [row("autumn25.4ccrules", "/vg/ League 25")]);
    }

    /// The × deletes the copy, and the list no longer holds it.
    #[test]
    fn remove_deletes_the_copy() {
        let dir = crate::test_support::scratch("import_remove");
        copy_in(&dir, "autumn25.4ccrules", "4cc Autumn 25");
        copy_in(&dir, "vgl25.4ccrules", "/vg/ League 25");
        remove(&dir, "autumn25.4ccrules").unwrap();
        assert!(!dir.join(RULES_FOLDER).join("autumn25.4ccrules").exists());
        assert_eq!(rows(&dir), [row("vgl25.4ccrules", "/vg/ League 25")]);
    }

    /// A pick whose file name differs from an import's only in case
    /// replaces that import under the import's own file name, whatever its
    /// description: on Windows the two names are one file, and the copy is
    /// never written over and then deleted.
    #[test]
    fn a_reimport_differing_only_in_case_keeps_the_imports_file_name() {
        for (i, description) in ["Cup", "Cup corrected"].into_iter().enumerate() {
            let dir = crate::test_support::scratch(&format!("import_case_{i}"));
            copy_in(&dir, "cup.4ccrules", "Cup");
            let text = renamed(description);
            let path = picked(&dir, "Cup.4ccrules", &text);
            let replacing = clean(check_import(&path, &imported(Some(&dir))));
            import(&dir, &replacing).unwrap();
            assert_eq!(
                rows(&dir),
                [row("cup.4ccrules", description)],
                "{description}"
            );
            assert_eq!(
                std::fs::read_to_string(dir.join(RULES_FOLDER).join("cup.4ccrules")).unwrap(),
                text,
                "{description}: the copy is the new file"
            );
            assert_eq!(replacing.file_name, "cup.4ccrules", "{description}");
            assert_eq!(
                replacing.replaces,
                [row("Cup", "cup.4ccrules")],
                "{description}"
            );
        }
    }
}
