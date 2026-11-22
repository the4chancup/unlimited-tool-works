//! Shared listing/metadata builders for the crate's tests.

use std::collections::{BTreeMap, BTreeSet};

use pes_version::PesVersion;

use crate::listing::{CanonicalListing, ListedEntry, ListedKind, SmallMetadata, ValidationContext};
use crate::parse::ParsedAestheticsExport;
use crate::validate::ValidationReport;

/// A listing of `name`'s source: `files` as (path, size), `folders` as paths.
pub(crate) fn listing(name: &str, files: &[(&str, u64)], folders: &[&str]) -> CanonicalListing {
    CanonicalListing {
        display_name: name.to_owned(),
        entries: files
            .iter()
            .map(|(path, size)| ListedEntry {
                path: (*path).to_owned(),
                kind: ListedKind::File { size: *size },
            })
            .chain(folders.iter().map(|path| ListedEntry {
                path: (*path).to_owned(),
                kind: ListedKind::Folder,
            }))
            .collect(),
    }
}

/// A `SmallMetadata` from (path, bytes-or-reason) pairs.
pub(crate) fn metadata(items: &[(&str, Result<&[u8], &str>)]) -> SmallMetadata {
    SmallMetadata {
        files: items
            .iter()
            .map(|(path, result)| {
                (
                    (*path).to_owned(),
                    (*result).map(|bytes| bytes.to_vec()).map_err(String::from),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

/// `parse_listing` on the listing and metadata items, unwrapped.
pub(crate) fn parsed(
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    metadata_items: &[(&str, Result<&[u8], &str>)],
) -> ParsedAestheticsExport {
    crate::parse_listing(listing(name, files, folders), metadata(metadata_items)).unwrap()
}

/// The default context for validation tests: Fox target, strict file check
/// on, pass-through off.
pub(crate) fn context() -> ValidationContext {
    ValidationContext {
        version: PesVersion::Pes21,
        strict_file_type_check: true,
        pass_through: false,
        installed_common_textures: BTreeSet::new(),
        installed_portrait_slots: None,
    }
}

/// A context with explicit strict-check and pass-through flags.
pub(crate) fn context_with(strict_file_type_check: bool, pass_through: bool) -> ValidationContext {
    ValidationContext {
        strict_file_type_check,
        pass_through,
        ..context()
    }
}

/// `parse_listing` + `validate` on the listing, with the default context.
pub(crate) fn report(
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    metadata_items: &[(&str, Result<&[u8], &str>)],
) -> ValidationReport {
    report_with(&context(), name, files, folders, metadata_items)
}

/// `parse_listing` + `validate` on the listing, with a supplied context.
pub(crate) fn report_with(
    context: &ValidationContext,
    name: &str,
    files: &[(&str, u64)],
    folders: &[&str],
    metadata_items: &[(&str, Result<&[u8], &str>)],
) -> ValidationReport {
    parsed(name, files, folders, metadata_items).validate(context)
}
