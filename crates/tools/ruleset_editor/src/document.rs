//! One rules file being edited: the egui-free document the view and the CLI
//! share. `Document::open` runs `Rules::load` (validation steps 1 to 3,
//! `spec/aatf_rules.md` "Loading and validation"), then reads the logic
//! section's identity; `save` writes the file only through the same
//! validation, so an invalid ruleset is never written.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use aatf::{
    Condition, Heights, LogicIdentity, Rules, Ruleset, Tier, Universal, WeakFoot, assemble_file,
    file_sections,
};

/// One rules file being edited: its ruleset, its logic and its custom checks
/// as read.
#[derive(Debug)]
pub(crate) struct Document {
    /// Where it was read from or last saved; `None` for one never saved.
    pub(crate) path: Option<PathBuf>,
    /// What the forms edit.
    pub(crate) ruleset: Ruleset,
    /// The file's logic section.
    pub(crate) logic: Logic,
    /// The CUSTOM CHECKS section as read, written back byte for byte but its
    /// trailing whitespace (the file's layout); `None`: no checks.
    pub(crate) custom_checks: Option<String>,
    /// The document as opened or last saved (`None`: never saved); the
    /// difference from it is what the unsaved marker shows.
    saved: Option<Snapshot>,
    /// The undo stacks.
    history: History,
}

/// One undo snapshot: the ruleset and the logic together, so an undo past
/// an Update logic brings the file's own code back.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    /// The ruleset.
    pub(crate) ruleset: Ruleset,
    /// The logic.
    pub(crate) logic: Logic,
}

/// The undo stacks: whole snapshots, since a ruleset is a few kilobytes.
#[derive(Debug, Default)]
pub(crate) struct History {
    /// The snapshots undone to the oldest, newest on top.
    undo: Vec<Snapshot>,
    /// The snapshots undone away, newest on top; cleared by every record.
    redo: Vec<Snapshot>,
}

impl History {
    /// `undo`/`redo`'s shared body: `from`'s top replaces `current`'s
    /// (pushed onto `to`); `false` when `from` is empty.
    fn step(from: &mut Vec<Snapshot>, to: &mut Vec<Snapshot>, current: &mut Snapshot) -> bool {
        let Some(snapshot) = from.pop() else {
            return false;
        };
        to.push(std::mem::replace(current, snapshot));
        true
    }
}

/// The file's logic section, against the logic this build ships.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Logic {
    /// The current logic, written fresh on every save.
    Current,
    /// An older official version, kept as read until `update_logic`.
    Older {
        /// The version the file's section hashes to.
        version: u32,
        /// The section as read, banner included, written back byte for byte
        /// but its trailing whitespace.
        section: String,
    },
    /// Not Studio's: kept as read on every save.
    Modified {
        /// The section as read, banner included, written back byte for byte
        /// but its trailing whitespace; empty when the file carries no LOGIC banner at all, and such a
        /// file opens but cannot be saved: the rewrite would hold no logic.
        section: String,
    },
}

/// Why a rules file did not open.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenError {
    /// The file could not be read.
    #[error("{}: cannot read the file: {error}", .path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// The io error.
        error: io::Error,
    },
    /// It read but failed load-time validation
    /// (`spec/aatf_rules.md` "Loading and validation").
    #[error("{0}")]
    Invalid(#[from] aatf::LoadError),
}

/// Why a rules file was not written.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveError {
    /// The file could not be written.
    #[error("{}: cannot write the file: {error}", .path.display())]
    Write {
        /// The file.
        path: PathBuf,
        /// The io error.
        error: io::Error,
    },
    /// The assembled file failed the same validation `open` runs; nothing is
    /// written.
    #[error("{0}")]
    Invalid(#[from] aatf::LoadError),
}

impl Document {
    /// A document for a template or Blank: no path, the current logic, no
    /// custom checks, unsaved.
    pub(crate) fn new(ruleset: Ruleset) -> Document {
        Document {
            path: None,
            ruleset,
            logic: Logic::Current,
            custom_checks: None,
            saved: None,
            history: History::default(),
        }
    }

    /// Reads and validates `path` (`Rules::load`: steps 1 to 3); a file that
    /// fails is not opened.
    pub(crate) fn open(path: &Path) -> Result<Document, OpenError> {
        let text = fs::read_to_string(path).map_err(|error| OpenError::Read {
            path: path.to_path_buf(),
            error,
        })?;
        let rules = Rules::load(&text).map_err(OpenError::Invalid)?;
        let sections = file_sections(&text);
        let logic = match rules.logic() {
            LogicIdentity::Current => Logic::Current,
            LogicIdentity::Official { version } => Logic::Older {
                version,
                // An Official identity is read from the LOGIC banner's
                // section, which `file_sections` therefore has.
                section: sections
                    .logic
                    .expect("an Official identity is read from the LOGIC banner's section"),
            },
            LogicIdentity::Modified => Logic::Modified {
                section: sections.logic.unwrap_or_default(),
            },
        };
        Ok(Document {
            path: Some(path.to_path_buf()),
            ruleset: rules.ruleset().clone(),
            logic: logic.clone(),
            custom_checks: sections.custom_checks,
            saved: Some(Snapshot {
                ruleset: rules.ruleset().clone(),
                logic,
            }),
            history: History::default(),
        })
    }

    /// The document as one undo snapshot: its ruleset and its logic.
    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            ruleset: self.ruleset.clone(),
            logic: self.logic.clone(),
        }
    }

    /// Whether the document differs from the file as read or last saved:
    /// the saved snapshot's ruleset and logic. A document never saved is
    /// always dirty.
    pub(crate) fn is_dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.snapshot())
    }

    /// `before` pushed onto the undo stack, the redo stack cleared: one
    /// entry per gesture, the pre-gesture snapshot.
    pub(crate) fn record(&mut self, before: Snapshot) {
        self.history.undo.push(before);
        self.history.redo.clear();
    }

    /// The document back to the top undo snapshot; `false` when the
    /// stack is empty.
    pub(crate) fn undo(&mut self) -> bool {
        let mut current = self.snapshot();
        if !History::step(&mut self.history.undo, &mut self.history.redo, &mut current) {
            return false;
        }
        (self.ruleset, self.logic) = (current.ruleset, current.logic);
        true
    }

    /// The document back to the top redo snapshot; `false` when the
    /// stack is empty.
    pub(crate) fn redo(&mut self) -> bool {
        let mut current = self.snapshot();
        if !History::step(&mut self.history.redo, &mut self.history.undo, &mut current) {
            return false;
        }
        (self.ruleset, self.logic) = (current.ruleset, current.logic);
        true
    }

    /// Whether an undo snapshot waits.
    pub(crate) fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }

    /// Whether a redo snapshot waits.
    pub(crate) fn can_redo(&self) -> bool {
        !self.history.redo.is_empty()
    }

    /// Gives an older official logic the current one; `false` (nothing
    /// changed) for a current or modified one.
    pub(crate) fn update_logic(&mut self) -> bool {
        match self.logic {
            Logic::Older { .. } => {
                self.logic = Logic::Current;
                true
            }
            Logic::Current | Logic::Modified { .. } => false,
        }
    }

    /// Assembles the file (`aatf::assemble_file`), validates it as `open`
    /// does, and writes it to a temporary sibling renamed over `path`;
    /// nothing is written when validation fails.
    pub(crate) fn save(&mut self, path: &Path) -> Result<(), SaveError> {
        let section = match &self.logic {
            Logic::Current => None,
            Logic::Older { section, .. } | Logic::Modified { section } => Some(section.as_str()),
        };
        let text = assemble_file(&self.ruleset, section, self.custom_checks.as_deref());
        Rules::load(&text).map_err(SaveError::Invalid)?;
        write_text(path, &text).map_err(|error| SaveError::Write {
            path: path.to_path_buf(),
            error,
        })?;
        self.path = Some(path.to_path_buf());
        self.saved = Some(self.snapshot());
        Ok(())
    }
}

/// A tier with the suite's stock defaults (form 4, injury resistance
/// 1, weak foot 2 and 2, 3 paid cards, 1 A position, no conditions or
/// extras): Blank's `regular` and the Tiers form's added tier both
/// start from it (the draft's `tier()`).
pub(crate) fn tier(id: &str, label: &str, count: Option<u8>, rate: u8) -> Tier {
    Tier {
        id: id.to_string(),
        label: label.to_string(),
        count,
        counts_as: None,
        when: Condition::default(),
        rate,
        stats: BTreeMap::new(),
        form: 4,
        injury_resistance: 1,
        weak_foot: WeakFoot {
            usage: 2,
            accuracy: 2,
        },
        cards: 3,
        min_cards: 0,
        free: BTreeMap::new(),
        a_positions: 1,
        name_color: None,
        stat_warning: 0,
        total_within_targets: false,
    }
}

/// The start page's Blank ruleset: "New ruleset", squad 23, Autumn 26's rating
/// and at-most stats, one `regular` tier (the rest of the squad, rate 75,
/// form 4, injury resistance 1, weak foot 2 and 2, 3 paid cards, 1 A
/// position), no brackets, no specials, and the universal rules on.
pub(crate) fn blank() -> Ruleset {
    let official = aatf::Rules::official();
    Ruleset {
        schema: 1,
        name: "New ruleset".to_string(),
        page: None,
        logo: None,
        stats: official.ruleset().stats.clone(),
        tiers: vec![tier("regular", "Regular", None, 75)],
        card_groups: BTreeMap::new(),
        medal_systems: Vec::new(),
        heights: Heights::default(),
        universal: Universal {
            registered_position_a: true,
            fielded_at_a: Vec::new(),
            no_b_ratings: true,
            b_uses_a_allowance: false,
            gk_not_second_a: true,
            age: None,
            weight: None,
            captain_required: true,
            min_goalkeepers: 1,
            max_skill_cards: 10,
            max_com_styles: None,
        },
        specials: Vec::new(),
        team_settings: Vec::new(),
        suggestions: Vec::new(),
    }
}

/// `text` written to `path` through a temporary sibling (`<path>.tmp`)
/// renamed over it; on failure the temporary is removed and the error
/// returned, `path` as it was. A read-only `path` is refused before anything
/// is written (`PermissionDenied`).
pub(crate) fn write_text(path: &Path, text: &str) -> io::Result<()> {
    // A rename replaces a read-only file on Linux, where the folder's
    // permission is what counts, while Windows refuses it: checking first
    // keeps one rule on both (RE-FILE-07).
    if fs::metadata(path).is_ok_and(|metadata| metadata.permissions().readonly()) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the file is read-only",
        ));
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let write_and_swap = || -> io::Result<()> {
        fs::write(&tmp, text)?;
        fs::rename(&tmp, path)
    };
    if let Err(error) = write_and_swap() {
        // Best effort: the original error is the one to report.
        if let Err(cleanup) = fs::remove_file(&tmp) {
            log::debug!("could not remove {}: {cleanup}", tmp.display());
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        CUSTOM_CHECKS_STUB, modified_logic_file, scratch, valid_file, write_file,
    };

    /// `valid_file()` as `test`'s `rules.4ccrules`, opened.
    fn open_official(test: &str) -> (PathBuf, Document) {
        let dir = scratch(test);
        let path = write_file(&dir, "rules.4ccrules", &valid_file());
        let document = Document::open(&path).expect("opens");
        (path, document)
    }

    /// `path` is byte for byte what `before` read.
    fn assert_file_unchanged(path: &Path, before: &[u8]) {
        assert_eq!(fs::read(path).unwrap(), before, "the file is unchanged");
    }

    /// Open the assembled official file: the current logic, no checks, an
    /// equal ruleset, nothing dirty.
    #[test]
    fn open_reads_the_ruleset_and_a_current_logic() {
        let (_path, doc) = open_official("open_current");
        assert!(matches!(doc.logic, Logic::Current));
        assert_eq!(doc.custom_checks, None);
        assert_eq!(&doc.ruleset, Rules::official().ruleset());
    }

    /// Open a file whose logic comment was edited: `Modified`, its section
    /// kept as read.
    #[test]
    fn open_marks_a_hand_edited_logic_as_modified() {
        let dir = scratch("open_modified");
        let text = modified_logic_file();
        let expected = file_sections(&text).logic.unwrap();
        let path = write_file(&dir, "rules.4ccrules", &text);
        let doc = Document::open(&path).expect("opens");
        match &doc.logic {
            Logic::Modified { section } => {
                assert_eq!(section.trim_end(), expected.trim_end())
            }
            other => panic!("expected Modified, got {other:?}"),
        }
    }

    /// A file that fails to load is not opened: `Invalid` carrying the
    /// `LoadError`.
    #[test]
    fn a_file_that_fails_to_load_is_not_opened() {
        let dir = scratch("open_broken");
        let path = write_file(&dir, "rules.4ccrules", "const RULESET = #{ name: \"x\",\n");
        match Document::open(&path) {
            Err(OpenError::Invalid(aatf::LoadError::Parse { line, .. })) => {
                assert!(line >= 1)
            }
            other => panic!("expected Invalid(Parse), got {other:?}"),
        }
        let missing = dir.join("missing.4ccrules");
        assert!(matches!(
            Document::open(&missing),
            Err(OpenError::Read { .. })
        ));
    }

    /// A file whose logic has no LOGIC banner opens as `Modified` with an
    /// empty section; saving it is refused (`MissingFunction`: the rewrite
    /// would hold no logic), the file unchanged.
    #[test]
    fn a_bannerless_file_opens_but_cannot_be_saved() {
        let dir = scratch("bannerless");
        let text = valid_file().replacen(
            "// ==== LOGIC (ruleset-logic 1): the Ruleset editor replaces this section ====\n",
            "",
            1,
        );
        assert!(!text.contains("// ==== LOGIC"), "premise: no banner");
        let path = write_file(&dir, "rules.4ccrules", &text);
        let mut doc = Document::open(&path).expect("opens");
        match &doc.logic {
            Logic::Modified { section } => assert!(section.is_empty()),
            other => panic!("expected Modified, got {other:?}"),
        }
        let before = fs::read(&path).unwrap();
        match doc.save(&path) {
            Err(SaveError::Invalid(aatf::LoadError::MissingFunction { .. })) => {}
            other => panic!("expected Invalid(MissingFunction), got {other:?}"),
        }
        assert_file_unchanged(&path, &before);
    }

    /// Save writes a file that reopens with an equal ruleset, the same logic
    /// status and the same checks.
    #[test]
    fn save_writes_a_file_that_reopens_equal() {
        let (path, mut doc) = open_official("save_round_trip");
        let dir = path.parent().expect("the scratch folder").to_path_buf();
        doc.ruleset.tiers[0].rate = 98;
        let out = dir.join("edited.4ccrules");
        doc.save(&out).expect("saves");
        let reopened = Document::open(&out).expect("reopens");
        assert_eq!(reopened.ruleset, doc.ruleset);
        assert!(matches!(reopened.logic, Logic::Current));
        assert_eq!(reopened.custom_checks, doc.custom_checks);
    }

    /// A modified logic section and VTL11's custom checks are written back
    /// byte for byte (trailing whitespace aside).
    #[test]
    fn a_modified_logic_and_custom_checks_are_kept_byte_for_byte() {
        let aatf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf");
        let block = fs::read_to_string(aatf_dir.join("rulesets/vtl11.rhai")).unwrap();
        let ruleset = Ruleset::from_data_block(&block).unwrap();
        let checks = file_sections(&block)
            .custom_checks
            .expect("VTL11 has checks");
        let current = file_sections(&valid_file()).logic.unwrap();
        let modified = current.replacen(
            "// The generic interpreter",
            "// The interpreter, its comment edited",
            1,
        );
        let dir = scratch("save_keeps_sections");
        let path = write_file(
            &dir,
            "rules.4ccrules",
            &assemble_file(&ruleset, Some(&modified), Some(&checks)),
        );
        let mut doc = Document::open(&path).expect("opens");
        assert!(matches!(doc.logic, Logic::Modified { .. }));
        doc.save(&path).expect("saves");
        let written = file_sections(&fs::read_to_string(&path).unwrap());
        assert_eq!(
            written.logic.as_deref().map(str::trim_end),
            Some(modified.trim_end()),
            "the file's own logic, byte for byte"
        );
        assert_eq!(
            written.custom_checks.as_deref().map(str::trim_end),
            Some(checks.trim_end()),
            "the custom checks, byte for byte"
        );
        let reopened = Document::open(&path).expect("reopens");
        assert!(matches!(reopened.logic, Logic::Modified { .. }));
    }

    /// Checks written inside the logic section (no CUSTOM CHECKS banner)
    /// load and `open` marks the logic `Modified`, but `save` refuses: the
    /// stub the assembly appends is a second `custom_checks`, which Rhai
    /// refuses to parse, so the member's checks are never shadowed by a
    /// file that loads. The pin guards that premise.
    #[test]
    fn checks_inside_the_logic_are_refused_on_save() {
        let dir = scratch("checks_in_logic");
        let stub = CUSTOM_CHECKS_STUB;
        let text = valid_file();
        assert!(text.contains(stub), "premise: the stub is appended");
        // VTL11's goalkeeper count, after the logic with no banner.
        let check = "fn custom_checks(team) {
    let found = [];
    let goalkeepers = 0;
    for player in team.players {
        if player.position == \"GK\" { goalkeepers += 1; }
    }
    if goalkeepers > 2 {
        found.push(#{ kind: \"Custom\", text: \"More than two goalkeepers\" });
    }
    found
}";
        let path = write_file(&dir, "rules.4ccrules", &text.replacen(stub, check, 1));
        let mut doc = Document::open(&path).expect("opens");
        assert!(
            matches!(doc.logic, Logic::Modified { .. }),
            "the file's logic is not this build's"
        );
        assert_eq!(doc.custom_checks, None, "no custom-checks section read");
        let before = fs::read(&path).unwrap();
        match doc.save(&path) {
            Err(SaveError::Invalid(aatf::LoadError::Parse { message, .. }))
                if message.contains("custom_checks") => {}
            other => panic!("expected the duplicate custom_checks refused, got {other:?}"),
        }
        assert_eq!(fs::read(&path).unwrap(), before, "the file is unchanged");
    }

    /// An invalid ruleset (two tiers sharing an id) is never written: the
    /// error is `Invalid` and the existing file is unchanged.
    #[test]
    fn an_invalid_ruleset_is_never_written() {
        let (path, mut doc) = open_official("save_refused");
        let before = fs::read(&path).unwrap();
        doc.ruleset.tiers[1].id = doc.ruleset.tiers[0].id.clone();
        match doc.save(&path) {
            Err(SaveError::Invalid(aatf::LoadError::Rules(_))) => {}
            other => panic!("expected Invalid(Rules), got {other:?}"),
        }
        assert_file_unchanged(&path, &before);
    }

    /// After a save the temporary sibling is renamed into place, not left
    /// beside the file.
    #[test]
    fn a_save_leaves_no_temporary_sibling() {
        let (path, mut doc) = open_official("save_no_tmp");
        doc.ruleset.name = "Renamed".to_string();
        doc.save(&path).expect("saves");
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        assert!(
            !Path::new(&tmp).exists(),
            "the temporary sibling is renamed, not left"
        );
    }

    /// A read-only file is not replaced, on Linux as on Windows: the save
    /// fails, the file is byte for byte as it was, and no temporary
    /// sibling is left.
    #[test]
    fn a_read_only_file_is_not_replaced() {
        let (path, mut doc) = open_official("save_read_only");
        let before = fs::read(&path).unwrap();
        doc.ruleset.name = "Renamed".to_string();
        let mut read_only = fs::metadata(&path).unwrap().permissions();
        read_only.set_readonly(true);
        fs::set_permissions(&path, read_only).unwrap();
        let result = doc.save(&path);
        // The file gets its writable bit back before any assertion, or the
        // scratch folder could not be cleared for the next run.
        let mut writable = fs::metadata(&path).unwrap().permissions();
        #[expect(
            clippy::permissions_set_readonly_false,
            reason = "a test fixture must get its writable bit back or the scratch folder cannot be cleared"
        )]
        writable.set_readonly(false);
        fs::set_permissions(&path, writable).unwrap();
        match result {
            Err(SaveError::Write { error, .. }) => {
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected a write failure, got {other:?}"),
        }
        assert_file_unchanged(&path, &before);
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".tmp");
        assert!(!Path::new(&tmp).exists(), "no temporary sibling is left");
    }

    /// A new document (a template or Blank): no path, the current logic,
    /// no custom checks, unsaved.
    #[test]
    fn a_new_document_is_unsaved_with_nothing_to_undo() {
        let doc = Document::new(blank());
        assert_eq!(doc.path, None);
        assert!(matches!(doc.logic, Logic::Current));
        assert_eq!(doc.custom_checks, None);
        assert!(doc.is_dirty(), "never saved is dirty");
        assert!(!doc.can_undo() && !doc.can_redo());
    }

    /// An opened document is clean, an edit dirties it, a save cleans it.
    #[test]
    fn the_dirty_flag_follows_edits_and_saves() {
        let (path, mut doc) = open_official("dirty_flag");
        assert!(!doc.is_dirty());
        doc.ruleset.name = "Renamed".to_string();
        assert!(doc.is_dirty(), "an edit dirties");
        doc.save(&path).expect("saves");
        assert!(!doc.is_dirty(), "a save cleans");
    }

    /// `record` pushes a snapshot `undo` restores and `redo` replays;
    /// a record clears the redo stack.
    #[test]
    fn undo_and_redo_step_the_snapshots_back_and_forth() {
        let mut doc = Document::new(blank());
        let original = doc.ruleset.clone();
        doc.ruleset.name = "One".to_string();
        let one = doc.ruleset.clone();
        doc.record(Snapshot {
            ruleset: original.clone(),
            logic: doc.logic.clone(),
        });
        doc.ruleset.name = "Two".to_string();
        doc.record(Snapshot {
            ruleset: one,
            logic: doc.logic.clone(),
        });
        assert!(doc.can_undo() && !doc.can_redo());

        assert!(doc.undo());
        assert_eq!(doc.ruleset.name, "One");
        assert!(doc.undo());
        assert_eq!(doc.ruleset, original);
        assert!(!doc.undo(), "the stack is empty");

        assert!(doc.redo());
        assert_eq!(doc.ruleset.name, "One");
        assert!(doc.redo());
        assert_eq!(doc.ruleset.name, "Two");
        assert!(!doc.redo(), "the redo stack is empty");

        // A record clears what was left to redo.
        doc.undo();
        doc.undo();
        assert!(doc.can_redo());
        doc.ruleset.name = "Three".to_string();
        doc.record(doc.snapshot());
        assert!(!doc.can_redo(), "a new record clears the redo stack");
    }

    /// An undo past the saved state dirties the document again; a redo
    /// back to the saved ruleset cleans it.
    #[test]
    fn undoing_past_the_save_dirties_and_redoing_back_cleans() {
        let (path, mut doc) = open_official("undo_dirty");
        let before = doc.snapshot();
        doc.ruleset.name = "Renamed".to_string();
        doc.record(before);
        doc.save(&path).expect("saves");
        assert!(!doc.is_dirty(), "saved");
        assert!(doc.undo(), "the gesture's snapshot is there");
        assert!(doc.is_dirty(), "undoing past the save dirties again");
        assert!(doc.redo(), "the redo snapshot is there");
        assert!(!doc.is_dirty(), "redoing back to the saved ruleset cleans");
    }

    /// An opened document pretending to carry official logic version 1:
    /// `logic` and `saved` agree.
    fn older_document(test: &str) -> Document {
        let (_path, mut document) = open_official(test);
        document.logic = Logic::Older {
            version: 1,
            section: String::new(),
        };
        document.saved = Some(document.snapshot());
        document
    }

    /// A snapshot is the ruleset and the logic together: Update logic's
    /// undo brings the older section back (the banner's button records
    /// the pre-click snapshot), the redo updates it again.
    #[test]
    fn update_logic_undoes_to_the_older_logic() {
        let mut doc = older_document("undo_logic");
        assert!(!doc.is_dirty());
        doc.record(doc.snapshot());
        assert!(doc.update_logic());
        assert!(matches!(doc.logic, Logic::Current));
        assert!(doc.is_dirty(), "the logic change dirties");

        assert!(doc.undo());
        assert!(
            matches!(doc.logic, Logic::Older { version: 1, .. }),
            "the older logic is back"
        );
        assert!(!doc.is_dirty(), "back to the saved snapshot");

        assert!(doc.redo());
        assert!(matches!(doc.logic, Logic::Current));
        assert!(doc.is_dirty(), "updated again");
    }

    /// Blank holds exactly the values `blank`'s doc comment lists.
    #[test]
    fn blank_is_the_plans_blank_ruleset() {
        let blank = blank();
        assert_eq!(blank.schema, 1);
        assert_eq!(blank.name, "New ruleset");
        assert_eq!(blank.stats, Rules::official().ruleset().stats.clone());
        assert_eq!(blank.tiers.len(), 1);
        let tier = &blank.tiers[0];
        assert_eq!(tier.id, "regular");
        assert_eq!(tier.label, "Regular");
        assert_eq!(tier.count, None, "the rest of the squad");
        assert_eq!(tier.rate, 75);
        assert_eq!(tier.form, 4);
        assert_eq!(tier.injury_resistance, 1);
        assert_eq!(
            tier.weak_foot,
            WeakFoot {
                usage: 2,
                accuracy: 2
            }
        );
        assert_eq!(tier.cards, 3);
        assert_eq!(tier.a_positions, 1);
        assert!(blank.heights.brackets.is_empty() && blank.heights.systems.is_empty());
        assert!(blank.specials.is_empty());
        assert!(blank.card_groups.is_empty());
        assert!(blank.medal_systems.is_empty());
        assert!(blank.team_settings.is_empty());
        assert!(blank.suggestions.is_empty());
        assert_eq!(
            blank.universal,
            Universal {
                registered_position_a: true,
                fielded_at_a: Vec::new(),
                no_b_ratings: true,
                b_uses_a_allowance: false,
                gk_not_second_a: true,
                age: None,
                weight: None,
                captain_required: true,
                min_goalkeepers: 1,
                max_skill_cards: 10,
                max_com_styles: None,
            }
        );
        assert!(blank.check().is_empty(), "Blank breaks no load rule");
        Rules::load(&assemble_file(&blank, None, None)).expect("the assembled file loads");
    }
}
