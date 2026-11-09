//! The `.mtl` texture references and FMDL path-table entries into the
//! team's `Common/`. A reference naming a `Common/` file becomes the bare
//! name plus the `.common` link spelling it. One no link can spell stays a
//! game path, its Common written `common/team_folder/` (the Fox common
//! directory for an FMDL), and is reported `common_reference_kept`: a
//! link beside a local file of its stem would
//! drop the folder, and a nearer local file would win the lookup silently.
//!
//! A shared folder's (`Boots/`, `Gloves/`) or a `Common/` `.mtl` keeps its
//! Common references as written, respelled `common/team_folder/` and no
//! link, with
//! `common_reference_kept` only when `Common/` lacks what it names. An
//! `.mtl` is read by the pre-Fox compile alone, which does not read a shared
//! folder's links, so a link there would lose the texture. A shared
//! folder's `.fmdl` links as a player folder's, since the Fox compile reads
//! a shared folder's links, and a `Common/` `.fmdl` gets the shared `.mtl`
//! rule.
//!
//! A referee's file, in the flattened referee layout, is checked against
//! his folder first: a Common reference naming a texture the folder holds at that path
//! (his old `common/`'s files, now at the root) becomes `./<sub>`, no link
//! and no finding; a bare `./x` (or empty-directory) reference his folder
//! lacks and the export's `Common/` root holds gets the link, as a game
//! directory's stem does. Any other reference follows the team rules.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{
    FileKind, KitToken, ModelFormat, classify, common_link_name, kit_path_token, kit_set_member,
};
use fmdl::FmdlFile;
use pes_model::format::mtl::MaterialSet;
use pes_model::ops::paths::rewrite_texture_paths;
use studio_core::{Disposition, Message, Scope};

use crate::carried::under;
use crate::kit_magic::{Spelling, respell_split};
use crate::messages::{Code, INPUT, UnmigratedReason, tool_message, unmigrated};
use crate::players::path_scope;
use crate::source::SourceFailure;
use crate::upgrade::Identity;

/// What step 7 makes of the migrated model folders' `.mtl` and `.fmdl`
/// files: the ones rewritten, the `.common` links their references need,
/// and the findings on the input.
#[derive(Debug, Default)]
pub(crate) struct MaterialRewrites {
    /// Rewritten `.mtl`/`.fmdl` files: output path and contents; `run`
    /// writes them instead of copying their input.
    pub(crate) written: Vec<(String, Vec<u8>)>,
    /// Empty `.common` link files to write, output paths.
    pub(crate) links: Vec<String>,
    /// The `(folder, stem)` pairs the read references resolve to: the
    /// folded output directory and the folded texture stem of every place
    /// a reference can land (a `.dds`/`.ftex` pair's "used" test). A
    /// `dummy_` reference's stem is never one: the game substitutes it.
    pub(crate) used: BTreeSet<(String, String)>,
    /// Findings on the input, in input-path order.
    pub(crate) findings: Vec<Message>,
}

/// The game directory prefix the team's `Common/` stands in, matched in
/// any letter case.
const COMMON_PREFIX: &str = "model/character/uniform/common/";

/// The team's Common directory an FMDL texture names, `???` the `XXX`
/// placeholder or a baked-in team ID (`common_home` for the Fox engine:
/// `…/common/<team>/<sub>sourceimages/`).
const FOX_COMMON_PREFIX: &str = "/Assets/pes16/model/character/common/";

/// The game's kit texture folder an FMDL texture keeps
/// (`fmdl_texture_kit_path`): a directory below it is a kit texture's too
/// (the compiler's prefix check).
const FOX_KIT_TEXTURE: &str = "/Assets/pes16/model/character/uniform/texture/";

/// The `???` (three characters) and `sub` of a team-Common directory
/// `/Assets/pes16/model/character/common/<???>/<sub>sourceimages/` (the
/// prefix compared ASCII case-insensitively): `sub` the path between
/// `???/` and `sourceimages/`, `""` or ending `/`, the compiler's
/// `sourceimages/` level not the export's.
fn reference_into_fox_common(directory: &str) -> Option<(&str, &str)> {
    let rest = directory
        .get(..FOX_COMMON_PREFIX.len())?
        .eq_ignore_ascii_case(FOX_COMMON_PREFIX)
        .then(|| &directory[FOX_COMMON_PREFIX.len()..])?;
    let (marker, rest) = rest.split_once('/')?;
    if marker.chars().count() != 3 || !rest.ends_with("sourceimages/") {
        return None;
    }
    Some((marker, &rest[..rest.len() - "sourceimages/".len()]))
}

/// The folded stem of a file name (`hair.dds` → `hair`, `Hair.PNG` the
/// same).
fn folded_stem(name: &str) -> String {
    let stem = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name);
    vtree::fold_name(stem)
}

/// Whether the file `name` claims the folded texture stem `stem` in its
/// folder: a texture of that stem, or a `.common` link to one (a model's
/// link, `hair.model.common`, claims no texture stem) — or, for a `kitN`
/// `set_reference`, a texture or texture link that is a member of its set.
fn claims_stem(name: &str, stem: &str, set_reference: Option<&str>) -> bool {
    let claims = |name: &str| {
        classify(name) == FileKind::Texture
            && (folded_stem(name) == stem
                || set_reference.is_some_and(|reference| kit_set_member(reference, name).is_some()))
    };
    if claims(name) {
        return true;
    }
    common_link_name(name).is_some_and(|target| claims(&target))
}

/// A reference's `???` marker and `sub` (the path below the Common
/// directory, ending in `/` or empty) when its directory is
/// `model/character/uniform/common/` + a non-empty segment + `/`, in any
/// letter case; `None` otherwise. The compiler refuses any folder after
/// `common/` but `team_folder`, so whatever stands there is the team's
/// Common (`respell_marker` respells it). A file directly below `common/`
/// is the game's own, not the team's.
fn reference_into_common(directory: &str) -> Option<(&str, &str)> {
    let prefix = directory.get(..COMMON_PREFIX.len())?;
    if !prefix.eq_ignore_ascii_case(COMMON_PREFIX) {
        return None;
    }
    let (marker, sub) = directory[COMMON_PREFIX.len()..].split_once('/')?;
    (!marker.is_empty()).then_some((marker, sub))
}

/// Which kind of folder a `.mtl` lands in: a player folder, a shared
/// `Boots/`/`Gloves/` one, or `Common/` (a shared folder's rules for its
/// references).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderKind {
    /// `Players/<folder>/`.
    Player,
    /// `Boots/<name>/`, `Gloves/<name>/`.
    Shared,
    /// `Common/`.
    Common,
}

/// The model folder an output path sits in (`Players/07 - A`,
/// `Boots/Crocs`, `Gloves/Mitts`, `Common`) and the file's path below it,
/// or `None`.
fn model_folder_of(output: &str) -> Option<(FolderKind, &str, &str)> {
    let (top, rest) = output.split_once('/')?;
    let kind = match top {
        "Players" => FolderKind::Player,
        "Boots" | "Gloves" => FolderKind::Shared,
        "Common" => return Some((FolderKind::Common, "Common", rest)),
        _ => return None,
    };
    let (folder, rest) = rest.split_once('/')?;
    Some((kind, &output[..top.len() + 1 + folder.len()], rest))
}

/// `directory`'s and `file_name`'s kit magic respelled (`Spelling::Path`)
/// as one path: a `u0???p` token in `directory` marks `file_name`'s Studio
/// tokens as it does a whole path's.
fn respell_reference(directory: &mut String, file_name: &mut String) {
    let (dir, name) = respell_split(directory, file_name, Spelling::Path);
    if let Cow::Owned(new) = dir {
        *directory = new;
    }
    if let Cow::Owned(new) = name {
        *file_name = new;
    }
}

/// `directory`, a kept Common game path, written in the Studio spelling:
/// `prefix` (`common/` for a `.mtl`'s short form, the Fox common
/// directory's own for an FMDL's kept whole path) then `team_folder/`
/// where the input's folder stood, then `rest` as written (`sub` for an
/// `.mtl`, `sub` + `sourceimages/` for an FMDL). The Studio format names
/// no team ID: the compiler reads `team_folder` as the team's Common and
/// refuses any other folder after `common/`.
fn respell_marker(directory: &mut String, prefix: &str, rest: &str) {
    *directory = format!("{prefix}team_folder/{rest}");
}

/// The `common_reference_kept` on a `.mtl`/FMDL file (`written` the
/// reference as written): the file the export ships without.
fn kept(written: &str, input: &str) -> Message {
    tool_message(
        Code::CommonReferenceKept,
        Scope::File {
            export_id: INPUT,
            path: path_scope(input),
        },
        Disposition::Keep,
        vec![("reference", written.to_owned())],
    )
}

/// The values one file's references share: the model folder it sits in
/// (`Players/07 - A`, `Boots/Crocs`, `Common`), its folder kind, its path
/// below it (`rest`), the `Common/` output paths, the plans' copies, its
/// input path and the export's identity.
#[derive(Clone, Copy)]
struct FileContext<'a> {
    /// `Players/`/`Boots/`/`Gloves/`/`Common/` (a shared `.fmdl` is
    /// mapped to `Player` by `fmdl_entry`).
    kind: FolderKind,
    /// The model folder's output path.
    model_folder: &'a str,
    /// The file's path below `model_folder` (`x.mtl`, `realkit/x.fmdl`).
    rest: &'a str,
    /// The `Common/` output paths (respelled).
    common: &'a [String],
    /// The plans' (input, output) copies.
    copies: &'a [(String, String)],
    /// The file's input path, for the finding's scope.
    input: &'a str,
    /// The export's identity, whose path ID respells `???`.
    identity: &'a Identity,
}

/// The mutable state one reference's resolution writes: the `Common/`
/// links planned so far, the `(folder, stem)` places a reference can land
/// (`MaterialRewrites::used`), and the findings pushed.
struct Pass<'a> {
    /// Link output path → the `Common/` path it names.
    planned: &'a mut BTreeMap<String, String>,
    /// The `(folder, stem)` places a reference can land, folded.
    used: &'a mut BTreeSet<(String, String)>,
    /// The findings the file's references pushed, in met order.
    findings: &'a mut Vec<Message>,
}

/// The folders a bare file-name reference resolves in, nearest first (the
/// file's folder up to the model folder), or `model_folder/sub` alone for
/// a `./sub` reference.
fn lookup_folders(ctx: &FileContext, sub: &str) -> Vec<String> {
    if !sub.is_empty() {
        return vec![format!(
            "{}/{}",
            ctx.model_folder,
            sub.trim_end_matches('/')
        )];
    }
    let file_folder = match ctx.rest.rsplit_once('/') {
        Some((dir, _)) => format!("{}/{dir}", ctx.model_folder),
        None => ctx.model_folder.to_owned(),
    };
    let mut folders = Vec::new();
    let mut folder = file_folder.as_str();
    loop {
        folders.push(folder.to_owned());
        if folder == ctx.model_folder {
            break;
        }
        folder = folder
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .expect("a folder below the model folder has a parent");
    }
    folders
}

/// `stem` (folded) marked used in every folder `sub`'s lookup reaches, on
/// the `used` set the `.dds`/`.ftex` pair rule reads: a reference resolves
/// to a texture it finds there.
fn mark_lookup(used: &mut BTreeSet<(String, String)>, ctx: &FileContext, sub: &str, stem: &str) {
    for folder in lookup_folders(ctx, sub) {
        used.insert((vtree::fold_name(&folder), stem.to_owned()));
    }
}

/// What the caller's reference is (`common_decision`'s outcomes differ):
/// a team-Common reference (`common/<???>/…`), kept and reported when it
/// cannot link; or a game directory's stem, linked when it can and
/// otherwise silent.
enum Reference {
    /// `common/<???>/…`.
    TeamCommon,
    /// `face/real/…`, `boots/…`.
    GameDir,
}

/// One FMDL texture entry's outcome, class by class: a `dummy_` stem is kept
/// as written (the game substitutes it); a `./` or empty directory has its
/// kit magic respelled only; the game's kit texture folder is kept, as the
/// compiler keeps it; the team's Common (`common/<3>/[sub/]sourceimages/`)
/// is linked as an `.mtl`'s Common reference (a `Common/` FMDL's kept,
/// `???` respelled, reported when `Common/` lacks the texture); any other
/// directory below `common/` is kept; every other directory becomes `./`,
/// the `Common/` root's texture linked when the folder holds none. In a
/// referee's FMDL, a team-Common entry his folder holds at `./<sub>` becomes
/// that, unlinked (`referee_folder_holds`), and a `./` or empty one is
/// linked as the last class is (`link_referee_bare`).
fn fmdl_entry(path: &mut fmdl::ops::paths::TexturePath, ctx: &FileContext, pass: &mut Pass) {
    if folded_stem(&path.file_name).starts_with("dummy_") {
        return;
    }
    if path.directory.is_empty() || path.directory.starts_with("./") {
        respell_reference(&mut path.directory, &mut path.file_name);
        mark_lookup(
            pass.used,
            ctx,
            path.directory.strip_prefix("./").unwrap_or(""),
            &folded_stem(&path.file_name),
        );
        link_referee_bare(ctx, &path.directory, &path.file_name, pass);
        return;
    }
    // The game's kit texture folder is kept (`fmdl_texture_kit_path`), a
    // directory below it a kit texture's too.
    if vtree::fold_name(&path.directory).starts_with(&vtree::fold_name(FOX_KIT_TEXTURE)) {
        mark_lookup(pass.used, ctx, "", &folded_stem(&path.file_name));
        return;
    }
    let written = format!("{}{}", path.directory, path.file_name);
    if path
        .directory
        .get(..FOX_COMMON_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(FOX_COMMON_PREFIX))
    {
        let Some((_marker, sub)) = reference_into_fox_common(&path.directory) else {
            // `common/sourceimages/`, `common/vg/sourceimages/`: not the
            // team's — kept as written.
            mark_lookup(pass.used, ctx, "", &folded_stem(&path.file_name));
            return;
        };
        let (sub, name) = respell_split(sub, &path.file_name, Spelling::Path);
        let sub = sub.into_owned();
        if let Cow::Owned(name) = name {
            path.file_name = name;
        }
        if referee_folder_holds(ctx, &sub, &path.file_name, pass.planned) {
            mark_lookup(pass.used, ctx, &sub, &folded_stem(&path.file_name));
            path.directory = format!("./{sub}");
            return;
        }
        // A shared folder's FMDL is linked as a player folder's (the Fox
        // compile reads a shared folder's links); `Common/`'s own keeps.
        let decision_ctx = FileContext {
            kind: match ctx.kind {
                FolderKind::Shared => FolderKind::Player,
                other => other,
            },
            ..*ctx
        };
        match common_decision(
            &decision_ctx,
            Reference::TeamCommon,
            &sub,
            &path.file_name,
            pass,
            &written,
        ) {
            Some(common_rest) => {
                plan_link(pass.planned, ctx, common_rest);
                path.directory = format!("./{sub}");
            }
            None => respell_marker(
                &mut path.directory,
                FOX_COMMON_PREFIX,
                &format!("{sub}sourceimages/"),
            ),
        }
        return;
    }
    // `Common/` keeps its other game directories; a player or shared
    // folder's every other directory becomes `./`, the `Common/` root's
    // texture linked when the folder holds none of the stem.
    if ctx.kind == FolderKind::Common {
        mark_lookup(pass.used, ctx, "", &folded_stem(&path.file_name));
        return;
    }
    // A `u0???p` token in the dropped directory still marks the file
    // name's Studio tokens: the entry's two halves are one path.
    if let Cow::Owned(name) = respell_split(&path.directory, &path.file_name, Spelling::Path).1 {
        path.file_name = name;
    }
    path.directory = "./".to_owned();
    if let Some(common_rest) =
        common_decision(ctx, Reference::GameDir, "", &path.file_name, pass, &written)
    {
        plan_link(pass.planned, ctx, common_rest);
    }
}

/// The `Common/` path the link names (`sub` + the respelled stem + the
/// member's extension for a set, the matched file's own path for any
/// other stem) when `sub`/`stem`/`set_reference` resolves in `common` and
/// no texture or texture link of the stem (or of its set) stands on the
/// nearest-first way from the file's folder up to `model_folder` — `None`
/// when kept (`common_reference_kept` pushed, `written` the reference as
/// written, for a class-4 caller's kept reference or a shared folder's/
/// `Common/`'s own missing one) or, for `game_dir`, when `Common/` lacks
/// the stem or the folder claims it and nothing is reported.
fn common_decision(
    ctx: &FileContext,
    class: Reference,
    sub: &str,
    file_name: &str,
    pass: &mut Pass,
    written: &str,
) -> Option<String> {
    let game_dir = matches!(class, Reference::GameDir);
    let stem = folded_stem(file_name);
    let respelled_stem = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    let reference = format!("{sub}{file_name}");
    let set_reference = set_reference(&reference);
    let sub_folded = vtree::fold_name(sub);
    // The `Common/` candidates: the `sub` directory's members of a `kitN`
    // reference's set, or the one texture of the stem.
    let matching: Vec<&str> = ctx
        .common
        .iter()
        .filter_map(|file| {
            let rest = under(file, "common")?;
            let name = rest.rsplit('/').next().unwrap_or(rest);
            if classify(name) != FileKind::Texture {
                return None;
            }
            if let Some(reference) = set_reference {
                kit_set_member(reference, rest).map(|_| rest)
            } else {
                let (folder, name) = match rest.rsplit_once('/') {
                    Some((folder, name)) => (format!("{folder}/"), name),
                    None => (String::new(), rest),
                };
                (vtree::fold_name(&folder) == sub_folded && folded_stem(name) == stem)
                    .then_some(rest)
            }
        })
        .collect();
    // Every place the reference can land counts the stem used: the
    // `Common/` members it names, and the folders its file name is looked
    // up in, nearest first.
    for &rest in &matching {
        let (dir, name) = match rest.rsplit_once('/') {
            Some((dir, name)) => (format!("Common/{dir}"), name),
            None => ("Common".to_owned(), rest),
        };
        pass.used
            .insert((vtree::fold_name(&dir), folded_stem(name)));
    }
    mark_lookup(pass.used, ctx, "", &stem);
    if !game_dir && ctx.kind != FolderKind::Player {
        // A shared folder's or `Common/` file's Common reference is
        // respelled, no link — reported when `Common/` lacks it.
        if matching.is_empty() {
            pass.findings.push(kept(written, ctx.input));
        }
        return None;
    }
    let common_rest: Option<String> = if set_reference.is_some() {
        // A link to a set reaches its members whatever the link's own
        // extension (`common_link_files` matches them by stem), so the
        // first member's names it.
        matching.first().map(|rest| {
            let extension = rest.rsplit_once('.').map_or("", |(_, ext)| ext);
            format!("{sub}{respelled_stem}.{extension}")
        })
    } else {
        match matching.as_slice() {
            &[rest] => Some(rest.to_owned()),
            _ => None,
        }
    };
    let Some(common_rest) = common_rest else {
        if !game_dir {
            pass.findings.push(kept(written, ctx.input));
        }
        return None;
    };
    // A `.mtl`/`.fmdl` in a subfolder naming a Common subfolder: the path
    // below it and the link mirroring `Common/` cannot agree.
    if !game_dir && !sub.is_empty() && ctx.rest.contains('/') {
        pass.findings.push(kept(written, ctx.input));
        return None;
    }
    // The clash check: the folders a bare name looks in, nearest first
    // (the file's folder up to the model folder), or for a `sub` the one
    // `model folder/sub` folder.
    let folders = lookup_folders(ctx, sub);
    if folders.iter().any(|folder| {
        folder_claims(
            ctx,
            folder,
            &stem,
            set_reference,
            pass.planned,
            &common_rest,
        )
    }) {
        if !game_dir {
            pass.findings.push(kept(written, ctx.input));
        }
        return None;
    }
    Some(common_rest)
}

/// `reference` (`sub` + the respelled file name) when it is a `kitN`
/// reference, whose set's members claim it; `None` for any other.
fn set_reference(reference: &str) -> Option<&str> {
    matches!(kit_path_token(reference), Some((KitToken::Reference, _))).then_some(reference)
}

/// Whether `folder` (an output folder path, a trailing `/` ignored) claims
/// the folded texture stem `stem` (or `set_reference`'s set): a copy
/// directly in it, or a planned link there other than the one to
/// `common_rest` (`""` counts every planned link), that `claims_stem`.
fn folder_claims(
    ctx: &FileContext,
    folder: &str,
    stem: &str,
    set_reference: Option<&str>,
    planned: &BTreeMap<String, String>,
    common_rest: &str,
) -> bool {
    let folder = vtree::fold_name(folder.trim_end_matches('/'));
    ctx.copies.iter().any(|(_, output)| {
        let Some((parent, name)) = output.rsplit_once('/') else {
            return false;
        };
        if vtree::fold_name(parent) != folder {
            return false;
        }
        claims_stem(name, stem, set_reference)
    }) || planned.iter().any(|(link, rest)| {
        rest != common_rest
            && link.rsplit_once('/').is_some_and(|(parent, name)| {
                vtree::fold_name(parent) == folder && claims_stem(name, stem, set_reference)
            })
    })
}

/// Plans the `.common` link to `Common/<common_rest>` at the same path
/// below the file's model folder, once however many references name it.
fn plan_link(planned: &mut BTreeMap<String, String>, ctx: &FileContext, common_rest: String) {
    planned.insert(
        format!("{}/{common_rest}.common", ctx.model_folder),
        common_rest,
    );
}

/// Whether the file is a referee's (a player folder of a referee export),
/// whose references step 12 resolves against his flattened folder first.
fn is_referee_file(ctx: &FileContext) -> bool {
    match ctx.identity {
        Identity::Referees => ctx.kind == FolderKind::Player,
        Identity::Team { .. } | Identity::ModelFolders => false,
    }
}

/// Whether a referee's Common reference (`sub`, ending in `/` or empty, and
/// `file_name`, both respelled) names a texture his folder holds at
/// `<model folder>/<sub>`, where the flatten put his old `common/`'s files.
/// The caller then writes `./<sub>`, no link and no finding, wherever the
/// file came from: the compiler looks a folder's own file up before a game
/// path. A file in a subfolder naming a `sub` is never held: its `./<sub>`
/// would name a path below its own subfolder, so the team rule keeps it.
fn referee_folder_holds(
    ctx: &FileContext,
    sub: &str,
    file_name: &str,
    planned: &BTreeMap<String, String>,
) -> bool {
    if !is_referee_file(ctx) || (!sub.is_empty() && ctx.rest.contains('/')) {
        return false;
    }
    let reference = format!("{sub}{file_name}");
    folder_claims(
        ctx,
        &format!("{}/{sub}", ctx.model_folder),
        &folded_stem(file_name),
        set_reference(&reference),
        planned,
        "",
    )
}

/// A referee's bare texture reference (`directory` `./` or empty, after the
/// respell): linked to the export's `Common/` root texture of its stem when
/// his folder, nearest first, holds none (step 12 widening class 6's link to
/// him); silent otherwise, as a game directory's stem is.
fn link_referee_bare(ctx: &FileContext, directory: &str, file_name: &str, pass: &mut Pass) {
    if !is_referee_file(ctx) || !matches!(directory, "" | "./") {
        return;
    }
    let written = format!("{directory}{file_name}");
    if let Some(common_rest) =
        common_decision(ctx, Reference::GameDir, "", file_name, pass, &written)
    {
        plan_link(pass.planned, ctx, common_rest);
    }
}

/// The anti-blur write-off on `file` (`input` → `output`): the copies
/// some FMDLs carry (a duplicate per mesh that asks for one, flagged
/// `is_antiblur_mesh` on a fuzzblock material) draw every such mesh
/// twice for nothing, so they and the materials only they used are
/// written off, every mesh's `has_antiblur_meshes` request cleared so
/// nothing adds the copies back — in a model folder, the same write the
/// texture paths' rewrite uses. `rewrites` gains the decoded file and `findings` the
/// `antiblur_dropped`. `false` when the file carries no copies, or does
/// not lift, decode or write back: it still reaches the output with its
/// copies, which the compiler packs as they are.
fn antiblur_write_off(
    file: &FmdlFile,
    input: &str,
    output: &str,
    rewrites: &mut MaterialRewrites,
    findings: &mut Vec<Message>,
) -> bool {
    let Some((decoded, removed)) = fmdl::Model::from_file(file)
        .ok()
        .filter(|model| model.meshes.iter().any(|mesh| mesh.is_antiblur_mesh))
        .and_then(|mut model| {
            let removed = model
                .meshes
                .iter()
                .filter(|mesh| mesh.is_antiblur_mesh)
                .count();
            fmdl::ops::antiblur::decode(&mut model)
                .and_then(|()| {
                    for mesh in &mut model.meshes {
                        mesh.has_antiblur_meshes = false;
                    }
                    model.to_file()
                })
                .ok()
                .map(|decoded| (decoded, removed))
        })
    else {
        return false;
    };
    rewrites.written.push((output.to_owned(), decoded.write()));
    findings.push(tool_message(
        Code::AntiblurDropped,
        Scope::File {
            export_id: INPUT,
            path: path_scope(input),
        },
        Disposition::Keep,
        vec![("meshes", removed.to_string())],
    ));
    true
}

/// What the upgrade makes of the migrated model folders' `.mtl` and `.fmdl`
/// files for an export of `identity`. It is one pass with one `planned` link
/// map, so a link is planned once and each format's clash walk sees the
/// other's links. `copies` is the (input, output) list of the player folders'
/// files and the carried `Common/` files; `common` is the `Common/` output
/// paths, the respelled names the links are spelled after.
pub(crate) fn material_references(
    copies: &[(String, String)],
    common: &[String],
    identity: &Identity,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> MaterialRewrites {
    let mut rewrites = MaterialRewrites::default();
    let mut failures: BTreeMap<String, Vec<Message>> = BTreeMap::new();
    let mut planned: BTreeMap<String, String> = BTreeMap::new();
    let mut used: BTreeSet<(String, String)> = BTreeSet::new();

    for (input, output) in copies {
        let Some(name) = output.rsplit('/').next() else {
            continue;
        };
        let fmdl = match classify(name) {
            FileKind::Mtl => false,
            FileKind::Model(ModelFormat::Fmdl) => true,
            _ => continue,
        };
        let Some((kind, model_folder, rest)) = model_folder_of(output) else {
            // An `.fmdl` outside a model folder (a carried `Collars/`
            // file) still gets the write-off, its texture paths kept:
            // the rewrite's rules are a model folder's. A `.mtl` there is
            // carried as it is, and so is an `.fmdl` that does not read or
            // lift.
            if fmdl
                && let Ok(bytes) = read(input)
                && let Ok(file) = FmdlFile::read(&bytes)
            {
                let mut file_findings = Vec::new();
                antiblur_write_off(&file, input, output, &mut rewrites, &mut file_findings);
                if !file_findings.is_empty() {
                    failures
                        .entry(input.clone())
                        .or_default()
                        .extend(file_findings);
                }
            }
            continue;
        };
        let ctx = FileContext {
            kind,
            model_folder,
            rest,
            common,
            copies,
            input,
            identity,
        };
        // The copy loop's own read reports a failure; a file that does not
        // read is the compile's to report, copied as it is.
        let Ok(bytes) = read(input) else {
            continue;
        };
        if fmdl {
            let Ok(mut file) = FmdlFile::read(&bytes) else {
                continue;
            };
            let mut file_findings = Vec::new();
            let mut pass = Pass {
                planned: &mut planned,
                used: &mut used,
                findings: &mut file_findings,
            };
            let changed = match fmdl::ops::paths::rewrite_texture_paths(&mut file, |path| {
                fmdl_entry(path, &ctx, &mut pass);
            }) {
                Ok(changed) => changed,
                // The string table could not hold the respelled paths:
                // copied as it is.
                Err(_) => continue,
            };
            if !antiblur_write_off(&file, input, output, &mut rewrites, &mut file_findings)
                && changed > 0
            {
                rewrites.written.push((output.clone(), file.write()));
            }
            if !file_findings.is_empty() {
                failures
                    .entry(input.clone())
                    .or_default()
                    .extend(file_findings);
            }
            continue;
        }
        let wrapped = wezlib::is_wrapped(&bytes);
        // Step 7's clean: read tolerantly, text between elements dropped
        // and reported; one that does not parse even so is unmigrated
        // content, its parse error in the context — the compile would drop
        // its folder as `mtl_broken`, so the output is not clean. The copy
        // loop still carries it, so the finding's disposition is Keep (the
        // Error alone is what the verdict reads).
        let (mut set, stray_text) = match MaterialSet::read_dropping_stray_text(&bytes) {
            Ok(parsed) => parsed,
            Err(error) => {
                failures.entry(input.clone()).or_default().push(unmigrated(
                    Scope::File {
                        export_id: INPUT,
                        path: path_scope(input),
                    },
                    Disposition::Keep,
                    UnmigratedReason::MtlUnparsable,
                    None,
                    vec![("error", error.to_string())],
                ));
                continue;
            }
        };
        let mut file_findings = Vec::new();
        if stray_text {
            file_findings.push(tool_message(
                Code::MtlStrayTextDropped,
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(input),
                },
                Disposition::Keep,
                vec![],
            ));
        }
        let mut pass = Pass {
            planned: &mut planned,
            used: &mut used,
            findings: &mut file_findings,
        };
        let changed = rewrite_texture_paths(&mut set, |path| {
            // `./` or no directory: a local reference — respell it (a
            // referee's bare one linked when only `Common/` holds it) and
            // stop.
            if path.directory.is_empty() || path.directory.starts_with("./") {
                respell_reference(&mut path.directory, &mut path.file_name);
                if !folded_stem(&path.file_name).starts_with("dummy_") {
                    mark_lookup(
                        pass.used,
                        &ctx,
                        path.directory.strip_prefix("./").unwrap_or(""),
                        &folded_stem(&path.file_name),
                    );
                }
                link_referee_bare(&ctx, &path.directory, &path.file_name, &mut pass);
                return;
            }
            // Every other reference (a game directory's) stays as written,
            // kit magic included; its file name is still looked up in the
            // file's folder first.
            let Some((_marker, sub)) = reference_into_common(&path.directory) else {
                if !folded_stem(&path.file_name).starts_with("dummy_") {
                    mark_lookup(pass.used, &ctx, "", &folded_stem(&path.file_name));
                }
                return;
            };
            // The reference as written, for the finding's context.
            let written = format!("{}{}", path.directory, path.file_name);
            // A `dummy_` stem is the game's; its `???` alone is respelled,
            // no lookup and no kit magic (MA1-2: the game supplies it).
            if folded_stem(&path.file_name).starts_with("dummy_") {
                // `sub` borrows `path.directory`: owned for the rewrite.
                let sub = sub.to_owned();
                respell_marker(&mut path.directory, "common/", &sub);
                return;
            }
            // The `sub` and the file name respelled (`Spelling::Path`) as
            // one path.
            let (sub, name) = respell_split(sub, &path.file_name, Spelling::Path);
            let sub = sub.into_owned();
            if let Cow::Owned(name) = name {
                path.file_name = name;
            }
            if referee_folder_holds(&ctx, &sub, &path.file_name, pass.planned) {
                mark_lookup(pass.used, &ctx, &sub, &folded_stem(&path.file_name));
                path.directory = format!("./{sub}");
                return;
            }
            match common_decision(
                &ctx,
                Reference::TeamCommon,
                &sub,
                &path.file_name,
                &mut pass,
                &written,
            ) {
                // `./` + `sub` + the name as respelled, and the link for
                // the `Common/` file's own name, planned once.
                Some(common_rest) => {
                    plan_link(pass.planned, &ctx, common_rest);
                    path.directory = format!("./{sub}");
                }
                None => respell_marker(&mut path.directory, "common/", &sub),
            }
        });
        if changed > 0 || stray_text {
            let out = set.write();
            rewrites.written.push((
                output.clone(),
                if wrapped { wezlib::compress(&out) } else { out },
            ));
        }
        if !file_findings.is_empty() {
            failures
                .entry(input.clone())
                .or_default()
                .extend(file_findings);
        }
    }
    rewrites.links = planned.keys().cloned().collect();
    rewrites.used = used;

    for (input, _) in copies {
        if let Some(messages) = failures.remove(input.as_str()) {
            rewrites.findings.extend(messages);
        }
    }
    rewrites.findings.extend(failures.into_values().flatten());
    rewrites
}

#[cfg(test)]
mod tests;
