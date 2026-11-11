//! Link files resolved against shared folders and `Common/`, and the
//! cascade ("Validation semantics" → "Dropped link targets", "Drop order"):
//! after every folder's own findings, a dropped `Common` file takes its
//! linking shared folders down and a dropped shared folder or `Common` file
//! takes its linking players down, then a shared folder no surviving
//! player links is orphaned.

use std::collections::BTreeSet;

use vtree::ScopePath;

use super::folders::{counts_as_link, fold, shared_folders, stem};
use super::roster::SlotMap;
use crate::FileKind;
use crate::conventions::{
    KitToken, SharedKind, classify, common_link_target, kit_set_member, names_kit_fallback,
    shared_link_name,
};
use crate::listing::ValidationContext;
use crate::parse::{AestheticsExportDraft, FileDescriptor, FolderDraft};
use crate::validate::{
    Disposition, IssueScope, ValidationIssue, common_file_dropped, dropped_scopes, is_below,
    issue_in,
};

/// The kind of link a file is, and what it names.
pub(crate) enum ResolvedLinkKind {
    /// A shared-folder link (`Crocs.boots`).
    Shared(SharedKind),
    /// A `.common` link; the path below `Common/` it looked for (`common_link_target`).
    Common(String),
}

/// One link file's resolution: its name, kind, and the target when it exists.
pub(crate) struct ResolvedLink {
    /// The link file's canonical path.
    pub link_file: ScopePath,
    /// The link file's name (`Crocs.boots.txt`).
    pub link_name: String,
    /// What the link names.
    pub kind: ResolvedLinkKind,
    /// The target's canonical path, when it exists.
    pub target: Option<ScopePath>,
}

impl ResolvedLink {
    /// Whether this is a `.common` link nothing satisfies: its target is not in `Common/`, and
    /// it is not a texture link naming one of `context`'s installed Common textures. That is
    /// `common_link_missing`'s condition, and what takes the link off a kept player's files.
    pub(crate) fn common_target_missing(&self, context: &ValidationContext) -> bool {
        match &self.kind {
            ResolvedLinkKind::Shared(_) => false,
            ResolvedLinkKind::Common(name) => {
                self.target.is_none() && !is_installed_texture(name, context)
            }
        }
    }
}

/// Whether `name`, a `.common` link's target name, is a texture an installed CPK holds in the
/// team's Common output. A model or material file never is: a Fox merge needs the source
/// model, which no CPK holds.
fn is_installed_texture(name: &str, context: &ValidationContext) -> bool {
    classify(name) == FileKind::Texture
        && context
            .installed_common_textures
            .contains(&fold(stem(name)))
}

/// Every player folder's link files resolved to their targets: its `SharedLink` and
/// `CommonLink` files wherever they count as links (`counts_as_link`): at any depth, a link
/// below a subfolder acting as one in the root. A `.common`
/// link's target is the `Common/` file at the link's own path below the folder
/// (`common_link_target`), at any depth of `Common/`.
pub(crate) fn player_links(
    folder: &FolderDraft,
    draft: &AestheticsExportDraft,
) -> Vec<ResolvedLink> {
    let mut links = Vec::new();
    for file in &folder.files {
        if !counts_as_link(file) {
            continue;
        }
        match file.kind {
            FileKind::SharedLink(kind) => {
                let (_, name) = shared_link_name(file.path.name())
                    .expect("a SharedLink kind implies a non-empty link stem");
                let target = shared_folders(draft, kind)
                    .iter()
                    .find(|target| fold(target.path.name()) == fold(&name))
                    .map(|target| target.path.clone());
                links.push(ResolvedLink {
                    link_file: file.path.clone(),
                    link_name: file.path.name().to_owned(),
                    kind: ResolvedLinkKind::Shared(kind),
                    target,
                });
            }
            FileKind::CommonLink => {
                let below = common_link_target(&file.path, &folder.path)
                    .expect("a CommonLink file of the folder names a path below `Common/`");
                let target = common_link_files(&draft.common, &below)
                    .first()
                    .map(|target| target.path.clone());
                links.push(ResolvedLink {
                    link_file: file.path.clone(),
                    link_name: file.path.name().to_owned(),
                    kind: ResolvedLinkKind::Common(below),
                    target,
                });
            }
            _ => {}
        }
    }
    links
}

/// The `Common/` files a `.common` link's target `below` (a path below `Common/`, as
/// `common_link_target` gives it) reaches, its representative first: what a link resolves
/// to and the planner reads. When `below`'s file name is no texture (`classify`), the
/// literal folded match alone — a link to a model or material-file set stays one until
/// the per-kit model pipeline for Common models lands, `kitF` included. For a texture:
/// nothing when `below` names the set's `kitF` fallback (`names_kit_fallback`), a path
/// that finds nothing whatever `Common/` holds; else the `Common/` file at that path
/// alone when it is held; else every texture member of the set `below` names
/// (`kit_set_member`), the lowest `kit1`…`kit9` variant first and the `kitF` member last.
pub fn common_link_files<'a>(common: &'a [FileDescriptor], below: &str) -> Vec<&'a FileDescriptor> {
    let name = below.rsplit_once('/').map_or(below, |(_, name)| name);
    let in_common =
        |file: &FileDescriptor| file.path.segments().skip(1).collect::<Vec<_>>().join("/");
    let key = fold(below);
    let literal = common.iter().find(|file| fold(&in_common(file)) == key);
    if classify(name) != FileKind::Texture {
        return literal.into_iter().collect();
    }
    if names_kit_fallback(below) {
        return Vec::new();
    }
    if let Some(file) = literal {
        return vec![file];
    }
    let mut members: Vec<(Option<u8>, &FileDescriptor)> = common
        .iter()
        .filter(|file| file.kind == FileKind::Texture)
        .filter_map(|file| {
            let kit = match kit_set_member(below, &in_common(file))? {
                KitToken::Variant(kit) => Some(kit),
                KitToken::Fallback | KitToken::Reference => None,
            };
            Some((kit, file))
        })
        .collect();
    // The representative's order: the lowest variant first, the `kitF` member last.
    members.sort_by_key(|(kit, _)| (kit.is_none(), *kit));
    members.into_iter().map(|(_, file)| file).collect()
}

/// The `.common` links of a shared folder, resolved as `player_links` resolves a player
/// folder's, since a shared folder's tree is laid out as a player folder's. They are the
/// only links it holds: a `.face`, `.boots` or `.gloves` file there is
/// `file_type_disallowed`, never a link.
pub(crate) fn shared_links(
    folder: &FolderDraft,
    draft: &AestheticsExportDraft,
) -> Vec<ResolvedLink> {
    player_links(folder, draft)
        .into_iter()
        .filter(|link| matches!(link.kind, ResolvedLinkKind::Common(_)))
        .collect()
}

/// `links` less the links a finding dropped (`dropped_files`, as `dropped_scopes` reads
/// them): a dropped link is no link, so it names no target for the cascade, the orphan
/// pass or the kept player's `links` ("Dropped link targets").
pub(crate) fn without_dropped(
    links: Vec<ResolvedLink>,
    dropped_files: &BTreeSet<String>,
) -> Vec<ResolvedLink> {
    links
        .into_iter()
        .filter(|link| !dropped_files.contains(&link.link_file.fold_key()))
        .collect()
}

/// `common_link_missing` on the folder at `scope` for each of `resolved`'s `.common`
/// links nothing satisfies (`common_target_missing`): a player folder's and a shared
/// folder's `.common` links are checked alike.
pub(crate) fn common_link_missing(
    scope: &IssueScope,
    resolved: &[ResolvedLink],
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) {
    for link in resolved {
        if let ResolvedLinkKind::Common(name) = &link.kind
            && link.common_target_missing(context)
        {
            issues.push(issue_in(
                context,
                "common_link_missing",
                scope.clone(),
                vec![
                    ("link", link.link_name.clone()),
                    ("path", format!("Common/{name}")),
                ],
                Disposition::DropFolder,
            ));
        }
    }
}

/// Whether `file` is one of `resolved`'s `.common` links whose target is absent
/// (`common_target_missing`): it comes off a kept folder's files, as a kept player's
/// does.
pub(crate) fn common_target_absent(
    file: &FileDescriptor,
    resolved: &[ResolvedLink],
    context: &ValidationContext,
) -> bool {
    resolved
        .iter()
        .any(|link| link.common_target_missing(context) && link.link_file == file.path)
}

/// Whether a `.common` link's target counts as dropped: every `Common/` file it
/// reaches is ("Dropped link targets": a link naming a kit set falls only with
/// its last member, a dropped member being a gap the set's completion fills).
/// A link `common_link_files` finds nothing for is no link to a dropped file. A file
/// counts as dropped by `common_file_dropped`: itself, or a `Common/` subfolder above it.
fn common_link_dropped(
    common: &[FileDescriptor],
    link: &ResolvedLink,
    dropped_folders: &BTreeSet<String>,
    dropped_files: &BTreeSet<String>,
) -> bool {
    let ResolvedLinkKind::Common(below) = &link.kind else {
        return false;
    };
    let files = common_link_files(common, below);
    !files.is_empty()
        && files
            .iter()
            .all(|file| common_file_dropped(&file.path, dropped_folders, dropped_files))
}

/// The first dropping issue code for a path, in issue order: one on the path itself, or a
/// `DropFolder` one on a folder above it (a `Common/` subfolder holding a linked file).
fn first_drop(issues: &[ValidationIssue], key: &str) -> &'static str {
    issues
        .iter()
        .find(|issue| {
            matches!(
                issue.disposition,
                Disposition::DropFile | Disposition::DropFolder
            ) && match &issue.scope {
                IssueScope::Folder(path) => {
                    let folder = path.fold_key();
                    folder == *key
                        || (issue.disposition == Disposition::DropFolder && is_below(key, &folder))
                }
                IssueScope::File(path) => path.fold_key() == *key,
                IssueScope::Export | IssueScope::RosterEntry { .. } => false,
            }
        })
        .map(|issue| issue.code)
        .expect("a dropped target has a dropping issue")
}

/// The cascade: `link_target_dropped` for each surviving shared folder or player's link
/// to a dropped target (a `.common` link's: every `Common/` file it reaches, a kit set
/// falling only with its last member) — the shared folders' `.common` links first, so
/// the players linking a folder one drops follow in the same pass — then
/// `shared_folder_orphaned` for shared folders no surviving player links.
pub(crate) fn cascade(
    draft: &AestheticsExportDraft,
    slot_map: &SlotMap,
    context: &ValidationContext,
    issues: &mut Vec<ValidationIssue>,
) {
    let (dropped_folders, dropped_files) = dropped_scopes(issues);
    let mapped = slot_map.mapped();

    // A shared folder's `.common` link to a dropped `Common/` file takes the folder
    // down. On PES 15-17 a shared folder's links stay unread (a link there is
    // `file_not_used`, the consumer's), so none resolves one.
    if let pes_version::Engine::Fox = context.version.engine() {
        for kind in [SharedKind::Face, SharedKind::Boots, SharedKind::Gloves] {
            for folder in shared_folders(draft, kind) {
                if dropped_folders.contains(&folder.path.fold_key()) {
                    continue;
                }
                for link in without_dropped(shared_links(folder, draft), &dropped_files) {
                    let Some(target) = &link.target else {
                        continue;
                    };
                    let key = target.fold_key();
                    if common_link_dropped(&draft.common, &link, &dropped_folders, &dropped_files) {
                        issues.push(issue_in(
                            context,
                            "link_target_dropped",
                            IssueScope::Folder(folder.path.clone()),
                            vec![
                                ("link", link.link_name.clone()),
                                ("target", target.as_str().to_owned()),
                                ("finding", first_drop(issues, &key).to_owned()),
                            ],
                            Disposition::DropFolder,
                        ));
                    }
                }
            }
        }
    }

    // link_target_dropped: every player not yet dropped whose link names a
    // dropped target, a shared folder one above dropped included. Never
    // pass-through-eligible itself. A link a finding
    // drops itself (a content finding's, with its target) is no link.
    let (dropped_folders, dropped_files) = dropped_scopes(issues);
    let player_dropped =
        |index: usize| dropped_folders.contains(&draft.players[index].path.fold_key());
    for (index, folder) in draft.players.iter().enumerate() {
        if !mapped.contains(&index) || player_dropped(index) {
            continue;
        }
        for link in without_dropped(player_links(folder, draft), &dropped_files) {
            let Some(target) = &link.target else { continue };
            let key = target.fold_key();
            let dropped = match link.kind {
                ResolvedLinkKind::Shared(_) => dropped_folders.contains(&key),
                ResolvedLinkKind::Common(_) => {
                    common_link_dropped(&draft.common, &link, &dropped_folders, &dropped_files)
                }
            };
            if dropped {
                issues.push(issue_in(
                    context,
                    "link_target_dropped",
                    IssueScope::Folder(folder.path.clone()),
                    vec![
                        ("link", link.link_name.clone()),
                        ("target", target.as_str().to_owned()),
                        ("finding", first_drop(issues, &key).to_owned()),
                    ],
                    Disposition::DropFolder,
                ));
            }
        }
    }

    // Refresh the drops after the cascade: a shared folder no surviving
    // player links is orphaned (a dropped one gets no finding).
    let (dropped_folders, dropped_files) = dropped_scopes(issues);
    let player_dropped =
        |index: usize| dropped_folders.contains(&draft.players[index].path.fold_key());
    let mut linked: BTreeSet<String> = BTreeSet::new();
    for index in &mapped {
        if player_dropped(*index) {
            continue;
        }
        for link in without_dropped(player_links(&draft.players[*index], draft), &dropped_files) {
            if let Some(target) = link.target {
                linked.insert(target.fold_key());
            }
        }
    }
    for kind in [SharedKind::Face, SharedKind::Boots, SharedKind::Gloves] {
        for folder in shared_folders(draft, kind) {
            if !dropped_folders.contains(&folder.path.fold_key())
                && !linked.contains(&folder.path.fold_key())
            {
                issues.push(issue_in(
                    context,
                    "shared_folder_orphaned",
                    IssueScope::Folder(folder.path.clone()),
                    vec![],
                    Disposition::DropFolder,
                ));
            }
        }
    }
}
