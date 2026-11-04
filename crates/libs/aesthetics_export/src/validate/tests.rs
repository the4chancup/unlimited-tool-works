use super::*;
use crate::testing::{context_with, report, report_with};
use crate::validate::IssueScope;
use crate::{ExportCoverage, FileKind, ModelFormat, SharedKind};
use vtree::ScopePath;

fn issue_codes(report: &ValidationReport) -> Vec<(&'static str, Disposition)> {
    report
        .issues
        .iter()
        .map(|issue| (issue.code, issue.disposition))
        .collect()
}

fn folder(path: &str) -> IssueScope {
    IssueScope::Folder(ScopePath::new(path).unwrap())
}

fn file_scope(path: &str) -> IssueScope {
    IssueScope::File(ScopePath::new(path).unwrap())
}

/// The default context with the target `version`.
fn context_for(version: pes_version::PesVersion) -> ValidationContext {
    ValidationContext {
        version,
        strict_file_type_check: true,
        pass_through: false,
        installed_common_textures: std::collections::BTreeSet::new(),
    }
}

mod common_links;
mod content_findings;
mod export_kind;
mod kits;
mod name_conflicts;
mod player_folder;
mod pre_fox;
mod root_files;
mod shared_folders;
mod shared_links;
mod undecided_root;
