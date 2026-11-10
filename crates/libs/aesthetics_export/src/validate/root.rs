//! The root-level checks: `Portraits/` slots, the `logo*` grammar, the
//! unexpected-file allowlist, and the small root artifacts (`notes.txt`,
//! `colors.txt`, `ref_marker.dds`).

use std::collections::BTreeMap;

use super::folders::{stem, texture_conflicts};
use crate::FileKind;
use crate::conventions::{Marker, MetadataFile, folder_slot, strip_prefix_ci};
use crate::listing::{SmallMetadata, ValidationContext, metadata_text};
use crate::parse::{AestheticsExportDraft, FileDescriptor};
use crate::slots::PlayerSlot;
use crate::validate::{
    Disposition, IssueScope, LogoFile, LogoFiles, LogoFit, RootArtifacts, ValidationIssue, issue_in,
};

/// `Portraits/NN.*`: a direct texture named by its slot as a player folder is
/// (`folder_slot`: `NN`, alone or before ` - ` and any text; `05`, `05 - Bob`
/// and `05-Bob` alike).
pub(crate) fn check_portraits(
    draft: &AestheticsExportDraft,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) -> BTreeMap<PlayerSlot, FileDescriptor> {
    let mut portraits = BTreeMap::new();
    let mut slotted: Vec<(PlayerSlot, &FileDescriptor)> = Vec::new();
    for file in &draft.portraits {
        let slot = folder_slot(stem(file.path.name()));
        if file.kind == FileKind::Texture
            && file.path.segments().count() == 2
            && let Some(slot) = slot
        {
            slotted.push((slot, file));
        } else {
            issues.push(issue_in(
                context,
                "portrait_name_invalid",
                IssueScope::File(file.path.clone()),
                vec![],
                Disposition::DropFile,
            ));
        }
    }
    // Two files claiming one slot conflict, each dropped (`stem` the slot's
    // `NN`) — the caller's `retain` applies the drop. Grouped by slot, not by
    // stem as `Common/`'s are: `05.dds` and `05 - Bob.png` are two stems one
    // slot cannot both take.
    texture_conflicts(
        context,
        slotted
            .iter()
            .map(|(slot, file)| (format!("{:02}", slot.get()), *file)),
        issues,
    );
    for (slot, file) in slotted {
        portraits.insert(slot, file.clone());
    }
    portraits
}

/// A `logo*` stem's grammar: `logo[_small][_<fit>]`, tags in that order,
/// ASCII-case-insensitive. `(small, fit)` or `None` on a bad tag.
fn logo_parts(stem: &str) -> Option<(bool, Option<LogoFit>)> {
    let rest = strip_prefix_ci(stem, "logo")?;
    let (small, rest) = match strip_prefix_ci(rest, "_small") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let fit = if rest.is_empty() {
        None
    } else if rest.eq_ignore_ascii_case("_crop") {
        Some(LogoFit::Crop)
    } else if rest.eq_ignore_ascii_case("_stretch") {
        Some(LogoFit::Stretch)
    } else if rest.eq_ignore_ascii_case("_fit") {
        Some(LogoFit::Fit)
    } else {
        return None;
    };
    Some((small, fit))
}

/// The `logo*` files: candidate = a root file whose stem starts with `logo`.
/// A non-texture or bad grammar is `logo_file_invalid`; two of one role
/// `logo_role_duplicate` on the later; a small with no main
/// `logo_small_without_main`. Any finding means no `logo` at all.
pub(crate) fn check_logo(
    draft: &AestheticsExportDraft,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) -> Option<LogoFiles> {
    let mut mains = Vec::new();
    let mut smalls = Vec::new();
    let start = issues.len();
    for file in &draft.root_files {
        if !is_logo_candidate(file) {
            continue;
        }
        match logo_parts(stem(file.path.name())) {
            Some((small, fit)) if file.kind == FileKind::Texture => {
                if small {
                    smalls.push(LogoFile {
                        file: file.clone(),
                        fit,
                    });
                } else {
                    mains.push(LogoFile {
                        file: file.clone(),
                        fit,
                    });
                }
            }
            _ => issues.push(issue_in(
                context,
                "logo_file_invalid",
                IssueScope::File(file.path.clone()),
                vec![],
                Disposition::DropFile,
            )),
        }
    }
    for group in [&mains, &smalls] {
        if let [first, rest @ ..] = group.as_slice() {
            let other = first.file.path.name().to_owned();
            for logo in rest {
                issues.push(issue_in(
                    context,
                    "logo_role_duplicate",
                    IssueScope::File(logo.file.path.clone()),
                    vec![("other", other.clone())],
                    Disposition::DropFile,
                ));
            }
        }
    }
    if mains.is_empty() {
        for logo in &smalls {
            issues.push(issue_in(
                context,
                "logo_small_without_main",
                IssueScope::File(logo.file.path.clone()),
                vec![],
                Disposition::DropFile,
            ));
        }
    }
    if issues.len() > start {
        None
    } else {
        mains.into_iter().next().map(|main| LogoFiles {
            main,
            small: smalls.into_iter().next(),
        })
    }
}

/// Whether `file` is a logo candidate: a root file whose stem starts with
/// `logo` (ASCII-case-insensitively). `check_logo` reports these, so
/// `check_root` skips them and never reports one as `root_file_unexpected`.
fn is_logo_candidate(file: &FileDescriptor) -> bool {
    strip_prefix_ci(stem(file.path.name()), "logo").is_some()
}

/// The root-file allowlist plus the small root artifacts: `notes.txt`,
/// `colors.txt`, `ref_marker.dds` (referee only), the unexpected files,
/// folders and strays.
pub(crate) fn check_root(
    draft: &AestheticsExportDraft,
    metadata: &SmallMetadata,
    referees: bool,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) -> RootArtifacts {
    let mut artifacts = RootArtifacts {
        team_colors: None,
        notes: None,
        referee_marker: None,
        write_names: false,
        legacy_ids: None,
    };
    for file in &draft.root_files {
        if is_logo_candidate(file) {
            continue;
        }
        let name = file.path.name();
        match file.kind {
            // The roster file and metadata are admitted (`refs.*` and the
            // referee marker only on a referee export, `legacy_ids.toml`
            // only on a team's).
            FileKind::Metadata(MetadataFile::PlayersTxt) => {}
            FileKind::Metadata(MetadataFile::NotesTxt) => {
                check_notes(file, metadata, context, &mut artifacts, issues);
            }
            FileKind::Metadata(MetadataFile::RefsTxt | MetadataFile::RefLists) if referees => {}
            FileKind::Metadata(MetadataFile::LegacyIdsToml) if !referees => {
                artifacts.legacy_ids = Some(file.clone());
            }
            FileKind::Metadata(MetadataFile::ColorsTxt) => {
                artifacts.team_colors = Some(file.clone());
            }
            FileKind::Metadata(MetadataFile::Readme) => {}
            FileKind::Texture if referees && name.eq_ignore_ascii_case("ref_marker.dds") => {
                artifacts.referee_marker = Some(file.clone());
            }
            FileKind::Marker(Marker::WriteNames) if !referees => {
                artifacts.write_names = true;
            }
            _ => issues.push(issue_in(
                context,
                "root_file_unexpected",
                IssueScope::File(file.path.clone()),
                vec![],
                Disposition::DropFile,
            )),
        }
    }
    for folder in &draft.root_folders {
        issues.push(issue_in(
            context,
            "root_file_unexpected",
            IssueScope::Folder(folder.clone()),
            vec![],
            Disposition::DropFolder,
        ));
    }
    for file in &draft.stray_files {
        issues.push(issue_in(
            context,
            "root_file_unexpected",
            IssueScope::File(file.path.clone()),
            vec![],
            Disposition::DropFile,
        ));
    }
    artifacts
}

/// `notes.txt`: read, BOM, strict UTF-8; non-whitespace content is kept as
/// `root.notes` with `notes_found`.
fn check_notes(
    file: &FileDescriptor,
    metadata: &SmallMetadata,
    context: &ValidationContext,
    artifacts: &mut RootArtifacts,
    issues: &mut Vec<ValidationIssue>,
) {
    let scope = IssueScope::File(file.path.clone());
    let bytes = match metadata.bytes_for(file) {
        Ok(bytes) => bytes,
        Err(reason) => {
            issues.push(issue_in(
                context,
                "source_read_failed",
                scope,
                vec![("reason", reason)],
                Disposition::DropFile,
            ));
            return;
        }
    };
    let Ok(text) = metadata_text(bytes) else {
        issues.push(issue_in(
            context,
            "notes_encoding_invalid",
            scope,
            vec![],
            Disposition::DropFile,
        ));
        return;
    };
    if text.trim().is_empty() {
        return;
    }
    issues.push(issue_in(
        context,
        "notes_found",
        scope,
        vec![],
        Disposition::Keep,
    ));
    artifacts.notes = Some(file.clone());
}
