//! The kit folders: the `<slot>[ - <label>]` grammar (`all/` aside), the
//! allowlist, texture names, layout markers and the icon marker — the own
//! findings — then the surviving `KitsFolder` with `all/` inheritance, a
//! `Full` team export's missing kinds of kit added as empty folders.

use std::collections::{BTreeMap, BTreeSet};

use kit_config::KitSlot;

use vtree::ScopePath;

use super::folders::{directly_in, fold, relative, stem, stem_conflicts};
use crate::conventions::{Marker, MetadataFile, icon_number, split_folder_name};
use crate::listing::ValidationContext;
use crate::parse::{AestheticsExportDraft, ExportKind, FileDescriptor, FolderDraft};
use crate::validate::{
    Disposition, IssueScope, KitFolder, KitLayout, KitTexture, KitTextureSource, KitsFolder,
    ValidationIssue, dropped_scopes, issue_in, strict_disposition,
};
use crate::{ExportCoverage, FileKind};

/// What a kit folder's head means: `all/`, or one of the ten kit slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum KitKind {
    /// The `all/` shared-textures folder.
    All,
    /// A kit slot (`p1`–`p9`, `g1`).
    Slot(KitSlot),
}

/// What a file directly in a kit folder may be (allowlist row "directly in a
/// kit folder"): textures, `config.toml`, `colors.txt`, the `pre-fox`, `fox`
/// and `icon_<N>` markers.
fn kit_direct_allowed(kind: FileKind) -> bool {
    matches!(
        kind,
        FileKind::Texture
            | FileKind::Metadata(MetadataFile::ConfigToml | MetadataFile::ColorsTxt)
            | FileKind::Marker(Marker::PreFox | Marker::Fox | Marker::Icon)
    )
}

/// `stem` is `kit` or starts with `kit_`, ASCII-case-insensitively.
fn is_kit_texture_name(stem: &str) -> bool {
    stem.eq_ignore_ascii_case("kit")
        || stem
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("kit_"))
}

/// The kit folders' findings and the sanitized `KitsFolder`: name grammar
/// and duplicates, each folder's own findings, `all/`'s, then building with
/// `all/` inheritance, a `Full` team export's missing kinds of kit built as
/// empty folders would be (`missing_kinds`).
pub(crate) fn check(
    draft: &AestheticsExportDraft,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) -> KitsFolder {
    // The head's kind per folder; an unparsable head is `kit_folder_invalid`.
    let mut claims: Vec<Option<KitKind>> = Vec::new();
    for folder in &draft.kits {
        let (head, _) = split_folder_name(folder.path.name());
        let kind = if head.eq_ignore_ascii_case("all") {
            Some(KitKind::All)
        } else {
            KitSlot::parse(head).map(KitKind::Slot)
        };
        if kind.is_none() {
            issues.push(issue_in(
                context,
                "kit_folder_invalid",
                IssueScope::Folder(folder.path.clone()),
                vec![],
                Disposition::DropFolder,
            ));
        }
        claims.push(kind);
    }

    // Two folders resolving to one slot (`all` included) drop each other.
    let mut by_kind: BTreeMap<KitKind, Vec<usize>> = BTreeMap::new();
    for (index, kind) in claims.iter().enumerate() {
        if let Some(kind) = kind {
            by_kind.entry(*kind).or_default().push(index);
        }
    }
    for (_, indices) in by_kind {
        if indices.len() > 1 {
            for index in indices {
                issues.push(issue_in(
                    context,
                    "kit_slot_duplicate",
                    IssueScope::Folder(draft.kits[index].path.clone()),
                    vec![],
                    Disposition::DropFolder,
                ));
            }
        }
    }

    // Each folder's own findings; `all/` gets the ignored-everything rule,
    // an invalid head none at all (it may be a misspelled `all`).
    for (index, folder) in draft.kits.iter().enumerate() {
        match claims[index] {
            Some(KitKind::All) => check_all(folder, context, issues),
            Some(KitKind::Slot(_)) => check_kit(folder, context, issues),
            None => {}
        }
    }

    // The drops so far name which folders survive (a slot-invalid or
    // duplicated folder claims nothing).
    let (dropped_folders, _) = dropped_scopes(issues);
    let surviving: Vec<usize> = (0..draft.kits.len())
        .filter(|index| {
            claims[*index].is_some()
                && !dropped_folders.contains(&draft.kits[*index].path.fold_key())
        })
        .collect();
    let all_index = surviving
        .iter()
        .copied()
        .find(|index| claims[*index] == Some(KitKind::All));
    // The surviving kit folders, then the empty ones a `Full` team export is
    // given: `all/` is unused only when there is neither.
    let mut kit_folders: Vec<(KitSlot, &FolderDraft)> = surviving
        .iter()
        .filter_map(|index| match claims[*index] {
            Some(KitKind::Slot(slot)) => Some((slot, &draft.kits[*index])),
            Some(KitKind::All) | None => None,
        })
        .collect();
    let surviving_slots: Vec<KitSlot> = kit_folders.iter().map(|(slot, _)| *slot).collect();
    let empty_folders = missing_kinds(draft, &surviving_slots);
    kit_folders.extend(empty_folders.iter().map(|(slot, folder)| (*slot, folder)));
    if let Some(index) = all_index
        && kit_folders.is_empty()
    {
        issues.push(issue_in(
            context,
            "kit_all_unused",
            IssueScope::Folder(draft.kits[index].path.clone()),
            vec![],
            Disposition::Keep,
        ));
    }

    // `all/`'s surviving textures (empty when it is absent or dropped).
    let shared: Vec<FileDescriptor> = match all_index {
        Some(index) => kit_textures(&draft.kits[index]).cloned().collect(),
        None => Vec::new(),
    };

    let kits = kit_folders
        .into_iter()
        .map(|(slot, folder)| (slot, kit_folder(folder, &shared, context, issues)))
        .collect();
    KitsFolder { kits, shared }
}

/// The empty kit folders a `Full` team export is compiled with where it has
/// no surviving kit of a kind, given its `surviving` kit slots: `Kits/p1`
/// when no player kit survived, `Kits/g1` when `g1` did not. Every team
/// needs one player kit and one goalkeeper kit. A `Midcup` export adds to the
/// kits installed and a
/// referee export has no team kits, so neither gets one.
fn missing_kinds(
    draft: &AestheticsExportDraft,
    surviving: &[KitSlot],
) -> Vec<(KitSlot, FolderDraft)> {
    let full_team = match (draft.kind(), draft.coverage) {
        (ExportKind::Team, Some(ExportCoverage::Full)) => true,
        (ExportKind::Team, Some(ExportCoverage::Midcup) | None)
        | (ExportKind::Referees, Some(ExportCoverage::Full | ExportCoverage::Midcup) | None) => {
            false
        }
    };
    if !full_team {
        return Vec::new();
    }
    let has_player_kit = surviving.iter().any(|slot| *slot != KitSlot::G1);
    let has_goalkeeper_kit = surviving.contains(&KitSlot::G1);
    [
        (KitSlot::P1, "Kits/p1", has_player_kit),
        (KitSlot::G1, "Kits/g1", has_goalkeeper_kit),
    ]
    .into_iter()
    .filter(|(_, _, present)| !present)
    .map(|(slot, path, _)| {
        let folder = FolderDraft {
            path: ScopePath::new(path).expect("`Kits/p1` and `Kits/g1` are valid scope paths"),
            files: Vec::new(),
        };
        (slot, folder)
    })
    .collect()
}

/// The sanitized kit of the surviving kit folder `folder`: its own surviving
/// textures, then the `all/` stems it lacks from `shared`, sorted by stem; an
/// inherited stem is reported.
fn kit_folder(
    folder: &FolderDraft,
    shared: &[FileDescriptor],
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) -> KitFolder {
    let own: BTreeSet<String> = kit_textures(folder)
        .map(|file| fold(stem(file.path.name())))
        .collect();
    let mut textures: Vec<KitTexture> = kit_textures(folder)
        .map(|file| KitTexture {
            stem: stem(file.path.name()).to_lowercase(),
            file: file.clone(),
            source: KitTextureSource::Own,
        })
        .collect();
    let mut inherited = Vec::new();
    for file in shared {
        let key = fold(stem(file.path.name()));
        if !own.contains(&key) {
            inherited.push(stem(file.path.name()).to_lowercase());
            textures.push(KitTexture {
                stem: stem(file.path.name()).to_lowercase(),
                file: file.clone(),
                source: KitTextureSource::Shared,
            });
        }
    }
    textures.sort_by(|a, b| a.stem.cmp(&b.stem));
    if !inherited.is_empty() {
        inherited.sort();
        let stems = inherited
            .iter()
            .map(|stem| {
                stem.strip_prefix("kit_")
                    .unwrap_or(stem.as_str())
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join(", ");
        issues.push(issue_in(
            context,
            "kit_textures_inherited",
            IssueScope::Folder(folder.path.clone()),
            vec![("stems", stems)],
            Disposition::Keep,
        ));
    }
    let (_, label) = split_folder_name(folder.path.name());
    KitFolder {
        path: folder.path.clone(),
        label: label.map(str::to_owned),
        config: direct_metadata(folder, MetadataFile::ConfigToml),
        colors: direct_metadata(folder, MetadataFile::ColorsTxt),
        icon: kit_icon(folder),
        layout: layout_of(folder),
        textures,
    }
}

/// `folder`'s direct files of the given metadata kind.
fn direct_metadata(folder: &FolderDraft, kind: MetadataFile) -> Option<FileDescriptor> {
    folder
        .files
        .iter()
        .find(|file| file.kind == FileKind::Metadata(kind) && directly_in(&file.path, &folder.path))
        .cloned()
}

/// `folder`'s markers: `pre-fox`/`fox`, or `None` (`kit_layout_conflict`
/// covered the both case).
fn layout_of(folder: &FolderDraft) -> Option<KitLayout> {
    if has_direct_marker(folder, Marker::PreFox) {
        Some(KitLayout::PreFox)
    } else if has_direct_marker(folder, Marker::Fox) {
        Some(KitLayout::Fox)
    } else {
        None
    }
}

/// The kit's menu icon: its one icon marker's number, or `None` when it has
/// no icon marker, several, or one out of range (`kit_icon_invalid` covers
/// the last two).
fn kit_icon(folder: &FolderDraft) -> Option<u8> {
    match icon_markers(folder).collect::<Vec<_>>()[..] {
        [only] => valid_icon(only),
        _ => None,
    }
}

/// `folder`'s direct icon markers (`icon_7`).
fn icon_markers(folder: &FolderDraft) -> impl Iterator<Item = &FileDescriptor> {
    folder.files.iter().filter(|file| {
        file.kind == FileKind::Marker(Marker::Icon) && directly_in(&file.path, &folder.path)
    })
}

/// An icon marker's number when it is one of the game's 24 menu icons, 0–23.
fn valid_icon(marker: &FileDescriptor) -> Option<u8> {
    icon_number(marker.path.name()).filter(|number| *number <= 23)
}

/// A kit folder's own textures: the textures directly in it named `kit` or `kit_*`. The one
/// named `kit` is the kit's main texture.
pub fn kit_textures(folder: &FolderDraft) -> impl Iterator<Item = &FileDescriptor> {
    folder.files.iter().filter(|file| {
        file.kind == FileKind::Texture
            && directly_in(&file.path, &folder.path)
            && is_kit_texture_name(stem(file.path.name()))
    })
}

/// A kit folder's own findings: allowlist, texture names, markers, stems,
/// the icon marker.
fn check_kit(folder: &FolderDraft, context: &ValidationContext, issues: &mut Vec<ValidationIssue>) {
    let scope = IssueScope::Folder(folder.path.clone());
    let direct = |path: &ScopePath| directly_in(path, &folder.path);

    // The allowlist row, per file; everything below a subfolder offends.
    for file in &folder.files {
        if !(direct(&file.path) && kit_direct_allowed(file.kind)) {
            issues.push(issue_in(
                context,
                "file_type_disallowed",
                scope.clone(),
                vec![("file", relative(&file.path, &folder.path))],
                strict_disposition(context, Disposition::DropFolder),
            ));
        }
    }

    // Texture names: `kit` or `kit_*` — a file that fails drops only itself.
    check_texture_names(folder, context, issues);

    // Both layout markers contradict.
    if has_direct_marker(folder, Marker::PreFox) && has_direct_marker(folder, Marker::Fox) {
        issues.push(issue_in(
            context,
            "kit_layout_conflict",
            scope.clone(),
            vec![],
            Disposition::DropFolder,
        ));
    }

    // Stems collide over the textures that kept their name.
    stem_conflicts(
        context,
        scope.clone(),
        kit_textures(folder)
            .map(|file| (fold(stem(file.path.name())), file.path.name().to_owned())),
        issues,
    );

    // The icon markers: one, numbered 0–23. With several, none says which
    // icon was meant, so each is invalid whatever its number.
    let icons: Vec<&FileDescriptor> = icon_markers(folder).collect();
    for icon in &icons {
        if icons.len() > 1 || valid_icon(icon).is_none() {
            issues.push(issue_in(
                context,
                "kit_icon_invalid",
                IssueScope::File(icon.path.clone()),
                vec![],
                Disposition::DropFile,
            ));
        }
    }
}

/// Whether `folder` holds `marker` directly.
fn has_direct_marker(folder: &FolderDraft, marker: Marker) -> bool {
    folder
        .files
        .iter()
        .any(|file| file.kind == FileKind::Marker(marker) && directly_in(&file.path, &folder.path))
}

/// `kit_texture_name_invalid` for each direct texture whose stem is not
/// `kit`/`kit_*`.
fn check_texture_names(
    folder: &FolderDraft,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) {
    for file in &folder.files {
        if file.kind == FileKind::Texture
            && directly_in(&file.path, &folder.path)
            && !is_kit_texture_name(stem(file.path.name()))
        {
            issues.push(issue_in(
                context,
                "kit_texture_name_invalid",
                IssueScope::File(file.path.clone()),
                vec![],
                Disposition::DropFile,
            ));
        }
    }
}

/// `all/`'s own findings: anything that is not a direct texture is ignored;
/// the textures get the same name and stem rules as a kit's.
fn check_all(folder: &FolderDraft, context: &ValidationContext, issues: &mut Vec<ValidationIssue>) {
    let direct = |path: &ScopePath| directly_in(path, &folder.path);
    for file in &folder.files {
        if !(direct(&file.path) && file.kind == FileKind::Texture) {
            issues.push(issue_in(
                context,
                "kit_all_file_ignored",
                IssueScope::File(file.path.clone()),
                vec![],
                Disposition::DropFile,
            ));
        }
    }
    check_texture_names(folder, context, issues);
    stem_conflicts(
        context,
        IssueScope::Folder(folder.path.clone()),
        kit_textures(folder)
            .map(|file| (fold(stem(file.path.name())), file.path.name().to_owned())),
        issues,
    );
}
