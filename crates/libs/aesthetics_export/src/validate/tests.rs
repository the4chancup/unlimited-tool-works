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

/// The first context value of each of `report`'s issues, in issue order: the
/// files the findings name.
fn issue_files(report: &ValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.context[0].1.as_str())
        .collect()
}

/// The file names of the validated export's player `index`.
fn player_file_names(report: &ValidationReport, index: usize) -> Vec<String> {
    report.validated.as_ref().unwrap().players[index]
        .files
        .iter()
        .map(|file| file.path.name().to_owned())
        .collect()
}

/// The `context` a `common_link_missing` finding carries: the link file and
/// the `Common/` path it stood for.
fn link_path(link: &str, path: &str) -> Vec<(&'static str, String)> {
    vec![("link", link.to_owned()), ("path", path.to_owned())]
}

/// The `context` a `link_target_dropped` finding carries: the link file, the
/// dropped `target` and the `finding` that took it down.
fn link_target(link: &str, target: &str, finding: &str) -> Vec<(&'static str, String)> {
    vec![
        ("link", link.to_owned()),
        ("target", target.to_owned()),
        ("finding", finding.to_owned()),
    ]
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

/// The validated report `files` gives under the default context at `version`.
fn report_for(version: pes_version::PesVersion, files: &[(&str, u64)]) -> ValidationReport {
    report_with(&context_for(version), "egg Midcup", files, &[], &[])
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
