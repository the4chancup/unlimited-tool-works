//! What the structure pass reports. The crate decides each issue's
//! disposition; the consumer maps each issue to its own message (severity,
//! text, hint), testing that every code in `ISSUE_CODES` has a catalog row.

use std::collections::BTreeSet;

use vtree::ScopePath;

use crate::listing::ValidationContext;

/// One finding of validation, with what it is about and what becomes of it:
/// the structure pass's own, or a consumer's content finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// The stable code: one of `ISSUE_CODES` for the structure pass's own
    /// findings, the consumer's code for a `ContentFinding`.
    pub code: &'static str,
    /// What the issue is about.
    pub scope: IssueScope,
    /// Structured fields, in the message template's order.
    pub context: Vec<(&'static str, String)>,
    /// The effective disposition: `Keep` when `pass_through` kept the item.
    pub disposition: Disposition,
    /// `pass_through` turned a `DropFile`/`DropFolder` into `Keep`.
    pub passed_through: bool,
}

/// One finding of the consumer's content checks (the deep pass, which reads
/// the files of the sanitized export this crate cannot read), with what it
/// drops: `ValidationReport::with_content_findings` turns it into a
/// `ValidationIssue` of the same code, scope and context, so it drops,
/// cascades and passes through like the structure pass's own findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFinding {
    /// The consumer's own stable code; `logo_file_invalid`, for a logo
    /// that does not decode, is the one `ISSUE_CODES` member a consumer
    /// also reports.
    pub code: &'static str,
    /// What the finding is about: the item its disposition acts on (a
    /// `Folder` scope a player, shared or kit folder; a `File` scope a
    /// `Common/`, `Portraits/` or `Collars/` file, a logo file, a player
    /// folder's `settings.toml` or portrait, or a `colors.txt`).
    pub scope: IssueScope,
    /// Structured fields, in the message template's order.
    pub context: Vec<(&'static str, String)>,
    /// What becomes of the item unless `pass_through` keeps it.
    pub disposition: Disposition,
    /// Whether `pass_through` may turn a `DropFile`/`DropFolder` into `Keep`.
    pub pass_through_eligible: bool,
}

/// What a `ValidationIssue` is about, within the one export this crate sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueScope {
    /// The export itself.
    Export,
    /// One folder (player, shared or kit).
    Folder(ScopePath),
    /// One file.
    File(ScopePath),
    /// One roster-file line: the file, the 1-based line number, and the slot
    /// when it parsed.
    RosterEntry {
        /// The roster file's path.
        file: ScopePath,
        /// The 1-based line number.
        line: usize,
        /// The parsed slot, when the line carries one.
        slot: Option<u16>,
    },
}

/// What becomes of the item an issue's scope names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Disposition {
    /// Kept unchanged.
    Keep,
    /// One file dropped.
    DropFile,
    /// One normalized roster assignment dropped.
    DropSlot,
    /// One folder dropped.
    DropFolder,
    /// The whole export dropped.
    DropExport,
}

/// The shared issue constructor: every code the crate emits must be listed in
/// `ISSUE_CODES`.
pub(crate) fn issue(
    code: &'static str,
    scope: IssueScope,
    context: Vec<(&'static str, String)>,
    disposition: Disposition,
) -> ValidationIssue {
    debug_assert!(ISSUE_CODES.contains(&code), "{code} not in ISSUE_CODES");
    ValidationIssue {
        code,
        scope,
        context,
        disposition,
        passed_through: false,
    }
}

/// The structure-pass codes `pass_through` can keep ("Validation semantics" →
/// "Pass-through eligibility"): the eligible `DropFile`/`DropFolder`
/// dispositions only.
pub(crate) const PASS_THROUGH_ELIGIBLE: [&str; 4] = [
    "link_target_missing",
    "common_link_missing",
    "file_type_disallowed",
    "common_file_disallowed",
];

/// The context-aware issue constructor: `pass_through` turns an eligible
/// `DropFile`/`DropFolder` into `Keep` and marks it (`passed_through`).
/// Every later step reads the effective disposition only.
pub(crate) fn issue_in(
    context: &crate::listing::ValidationContext,
    code: &'static str,
    scope: IssueScope,
    issue_context: Vec<(&'static str, String)>,
    disposition: Disposition,
) -> ValidationIssue {
    pass_through(
        context,
        PASS_THROUGH_ELIGIBLE.contains(&code),
        issue(code, scope, issue_context, disposition),
    )
}

/// A consumer's content finding as an issue: its code is the consumer's own
/// (`logo_file_invalid` the one `ISSUE_CODES` lists too), so `issue`'s check
/// does not apply, and its own flag says whether `pass_through` may keep its
/// item.
pub(crate) fn content_issue(
    finding: ContentFinding,
    context: &ValidationContext,
) -> ValidationIssue {
    pass_through(
        context,
        finding.pass_through_eligible,
        ValidationIssue {
            code: finding.code,
            scope: finding.scope,
            context: finding.context,
            disposition: finding.disposition,
            passed_through: false,
        },
    )
}

/// `issue` under `pass_through`: an `eligible` `DropFile`/`DropFolder`
/// becomes `Keep`, marked `passed_through`.
fn pass_through(
    context: &ValidationContext,
    eligible: bool,
    mut issue: ValidationIssue,
) -> ValidationIssue {
    if context.pass_through
        && eligible
        && matches!(
            issue.disposition,
            Disposition::DropFile | Disposition::DropFolder
        )
    {
        issue.disposition = Disposition::Keep;
        issue.passed_through = true;
    }
    issue
}

/// `disposition` when `strict_file_type_check` is on, `Keep` when off — the
/// strict/lenient split shared by the allowlist checks.
pub(crate) fn strict_disposition(
    context: &ValidationContext,
    disposition: Disposition,
) -> Disposition {
    if context.strict_file_type_check {
        disposition
    } else {
        Disposition::Keep
    }
}

/// The dropped folder and file paths, by fold key, from the issues'
/// effective dispositions (`(folders, files)`).
pub(crate) fn dropped_scopes(issues: &[ValidationIssue]) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut folders = BTreeSet::new();
    let mut files = BTreeSet::new();
    for issue in issues {
        match &issue.scope {
            IssueScope::Folder(path) if issue.disposition == Disposition::DropFolder => {
                folders.insert(path.fold_key());
            }
            IssueScope::File(path) if issue.disposition == Disposition::DropFile => {
                files.insert(path.fold_key());
            }
            IssueScope::Folder(_)
            | IssueScope::File(_)
            | IssueScope::Export
            | IssueScope::RosterEntry { .. } => {}
        }
    }
    (folders, files)
}

/// Whether the `Common/` file at `path` is dropped: a finding drops the file itself
/// (`dropped_files`) or a `Common/` subfolder above it (`dropped_folders`), as one on a
/// player, shared or kit folder drops that folder. Both sets hold fold keys, as
/// `dropped_scopes` returns them.
pub(crate) fn common_file_dropped(
    path: &ScopePath,
    dropped_folders: &BTreeSet<String>,
    dropped_files: &BTreeSet<String>,
) -> bool {
    let key = path.fold_key();
    dropped_files.contains(&key) || dropped_folders.iter().any(|folder| is_below(&key, folder))
}

/// Whether the fold key `key` names a path below the folder whose fold key is `folder`:
/// `common/a/x.dds` is below `common/a`, `common/ax.dds` is not.
pub(crate) fn is_below(key: &str, folder: &str) -> bool {
    key.strip_prefix(folder)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Every code the crate emits, listed here so consumers can test their
/// catalog against it.
pub const ISSUE_CODES: &[&str] = &[
    "nested_folders_fixed",
    "nested_root_ambiguous",
    "nested_root_conflict",
    "players_txt_invalid",
    "refs_txt_ignored",
    "source_read_failed",
    "export_empty",
    "team_name_unknown",
    "export_tag_missing",
    "export_layout_old",
    "players_txt_missing",
    "players_txt_line_invalid",
    "players_txt_slot_invalid",
    "players_txt_slot_duplicate",
    "players_txt_target_missing",
    "player_unlisted",
    "player_folder_number_invalid",
    "player_number_duplicate",
    "file_type_disallowed",
    "common_file_disallowed",
    "player_layout_proto",
    "shared_link_duplicate",
    "link_target_missing",
    "common_link_missing",
    "link_target_dropped",
    "shared_folder_orphaned",
    "fpc_conflict",
    "ingame_face_marker_renamed",
    "ingame_face_explicit_face_model",
    "texture_stem_conflict",
    "fmdl_name_invalid",
    "model_name_invalid",
    "edithair_unsupported",
    "kit_folder_invalid",
    "kit_slot_duplicate",
    "kit_texture_name_invalid",
    "kit_layout_conflict",
    "kit_icon_invalid",
    "kit_all_file_ignored",
    "kit_all_unused",
    "kit_textures_inherited",
    "portrait_name_invalid",
    "logo_file_invalid",
    "logo_role_duplicate",
    "logo_small_without_main",
    "root_file_unexpected",
    "notes_found",
    "notes_encoding_invalid",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pass_through_eligible_code_exists() {
        for code in PASS_THROUGH_ELIGIBLE {
            assert!(ISSUE_CODES.contains(&code), "{code}");
        }
    }
}
