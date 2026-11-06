//! The `.mtl` texture references and FMDL path-table entries into the
//! team's `Common/`. A reference naming a `Common/` file becomes the bare
//! name plus the `.common` link spelling it. One no link can spell stays as
//! written, its `???` respelled the identity's path ID, and is reported
//! `common_reference_kept`: a link beside a local file of its stem would
//! drop the folder, and a nearer local file would win the lookup silently.
//!
//! A shared folder's (`Boots/`, `Gloves/`) or a `Common/` `.mtl` keeps its
//! Common references as written, `???` respelled and no link, with
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
use crate::kit_magic::{Spelling, respell};
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
/// letter case; `None` otherwise. The compiler reads any three-character
/// segment as the team's Common and refuses any other, so one of another
/// length is still the team's (`common_segment` respells it).
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

/// `directory`'s and `file_name`'s kit magic respelled (`Spelling::Path`,
/// every `u0???p`/`u0???g` token in each).
fn respell_reference(directory: &mut String, file_name: &mut String, identity: &Identity) {
    if let Cow::Owned(new) = respell(directory, Spelling::Path, identity) {
        *directory = new;
    }
    if let Cow::Owned(new) = respell(file_name, Spelling::Path, identity) {
        *file_name = new;
    }
}

/// The segment a Common path's `???` marker is written as: the identity's
/// path ID, three digits (a team export's, 999 the referees'); in
/// model-folder mode (no path ID) `written` kept when it is three
/// characters and `XXX` otherwise — the compiler reads any
/// three-character segment as the team's Common and refuses any other.
pub(crate) fn common_segment(written: &str, identity: &Identity) -> String {
    match identity.path_id() {
        Some(id) => format!("{id:03}"),
        None if written.chars().count() == 3 => written.to_owned(),
        None => "XXX".to_owned(),
    }
}

/// The `???` of `directory`'s Common path respelled (`common_segment`) in
/// place: `prefix_len` the `model/character/…/common/` prefix's length
/// (either format's) and `rest` what follows it (`sub` for an `.mtl`,
/// `sub` + `sourceimages/` for an FMDL), still applied.
fn respell_marker(directory: &mut String, prefix_len: usize, rest: &str, identity: &Identity) {
    let end = directory[prefix_len..]
        .find('/')
        .expect("the callers matched the marker and the `/` after it");
    let marker = common_segment(&directory[prefix_len..prefix_len + end], identity);
    *directory = format!("{}{marker}/{rest}", &directory[..prefix_len]);
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
        respell_reference(&mut path.directory, &mut path.file_name, ctx.identity);
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
        let sub = respell(sub, Spelling::Path, ctx.identity).into_owned();
        if let Cow::Owned(name) = respell(&path.file_name, Spelling::Path, ctx.identity) {
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
                FOX_COMMON_PREFIX.len(),
                &format!("{sub}sourceimages/"),
                ctx.identity,
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
    if let Cow::Owned(name) = respell(&path.file_name, Spelling::Path, ctx.identity) {
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
                respell_reference(&mut path.directory, &mut path.file_name, ctx.identity);
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
                respell_marker(&mut path.directory, COMMON_PREFIX.len(), &sub, ctx.identity);
                return;
            }
            // The `sub` and the file name respelled (`Spelling::Path`).
            let sub = respell(sub, Spelling::Path, ctx.identity).into_owned();
            if let Cow::Owned(name) = respell(&path.file_name, Spelling::Path, ctx.identity) {
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
                None => {
                    respell_marker(&mut path.directory, COMMON_PREFIX.len(), &sub, ctx.identity)
                }
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
mod tests {
    use studio_core::Scope;
    use teams_list::{TeamId, TeamName};

    use super::*;

    /// The resolved team, `/co/` (714).
    fn team() -> Identity {
        Identity::Team {
            id: TeamId::new(714).unwrap(),
            name: TeamName::new("co").unwrap(),
        }
    }

    /// A referee export's identity.
    fn refs() -> Identity {
        Identity::Referees
    }

    /// The resolved team `/vg/` (758).
    fn vg() -> Identity {
        Identity::Team {
            id: TeamId::new(758).unwrap(),
            name: TeamName::new("vg").unwrap(),
        }
    }

    /// A `.mtl` of one material holding one sampler per `paths` entry.
    fn mtl(paths: &[&str]) -> Vec<u8> {
        let samplers: String = paths
            .iter()
            .map(|path| {
                format!("        <sampler name=\"DiffuseMap\" path=\"{path}\" srgb=\"1\" minfilter=\"linear\" magfilter=\"linear\" uaddr=\"clamp\" vaddr=\"clamp\"/>\n")
            })
            .collect();
        format!(
            "<materialset>\n    <material name=\"face_phong\" shader=\"Wrinkle_Face\">\n{samplers}    </material>\n</materialset>\n"
        )
        .into_bytes()
    }

    /// An FMDL of the `konami_highneck` fixture with its four texture
    /// entries set from `entries` (`(directory, file_name)` pairs, in
    /// table order).
    fn fmdl(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut file = FmdlFile::read(include_bytes!(
            "../../../libs/fmdl/tests/fixtures/konami_highneck.fmdl"
        ))
        .unwrap();
        let mut entries = entries.iter();
        fmdl::ops::paths::rewrite_texture_paths(&mut file, |path| {
            if let Some((directory, name)) = entries.next() {
                path.directory = (*directory).to_owned();
                path.file_name = (*name).to_owned();
            }
        })
        .unwrap();
        file.write()
    }

    /// The `(directory, file_name)` entries of `output`'s written FMDL.
    fn fmdl_paths(rewrites: &MaterialRewrites, output: &str) -> Vec<(String, String)> {
        let (_, bytes) = rewrites
            .written
            .iter()
            .find(|(path, _)| path == output)
            .expect("the FMDL was rewritten");
        let file = FmdlFile::read(bytes).unwrap();
        fmdl::ops::paths::texture_paths(&file)
            .unwrap()
            .iter()
            .map(|path| (path.directory.clone(), path.file_name.clone()))
            .collect()
    }

    #[test]
    fn an_fmdl_common_reference_becomes_the_stem_and_the_link() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "hair.dds",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "hair.dds".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/hair.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_shared_fmdl_common_subfolder_links_below_the_folder() {
        let boots = fmdl(&[(
            "/Assets/pes16/model/character/common/000/Boots/sourceimages/",
            "x.dds",
        )]);
        let rewrites = plan(
            &[("boots", "Boots/Crocs/boots.fmdl")],
            &["Common/Boots/x.dds"],
            &[("boots", &boots)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Boots/Crocs/boots.fmdl")[0],
            ("./Boots/".to_owned(), "x.dds".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Boots/Crocs/Boots/x.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_common_reference_with_a_studio_token_beside_the_legacy_one_links_the_set() {
        // The real VGL26 layout: the file's own `kit1_` prefix reads as the
        // author's variant, so the respelled reference is the set's only token.
        let boots = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "kit1_u0827p0.dds",
        )]);
        let rewrites = plan(
            &[("boots", "Boots/VIVLOS/boots.fmdl")],
            &[
                "Common/Kit1_kit1.dds",
                "Common/Kit1_kit2.dds",
                "Common/Kit1_kit3.dds",
                "Common/Kit1_kit4.dds",
            ],
            &[("boots", &boots)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Boots/VIVLOS/boots.fmdl")[0],
            ("./".to_owned(), "Kit1_kitN.dds".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Boots/VIVLOS/Kit1_kitN.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_common_kit_reference_links_the_set() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/common/000/u0XXXp0/sourceimages/",
            "armor_bsm",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &["Common/kit1/armor_bsm.dds"],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./kitN/".to_owned(), "armor_bsm".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/kitN/armor_bsm.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_common_keeper_reference_respells_the_team() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "u0XXXg1.dds",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &["Common/u0714g1.dds"],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "u0714g1.dds".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/u0714g1.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_common_reference_whose_texture_is_missing_is_kept() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "hair.dds",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            (
                "/Assets/pes16/model/character/common/714/sourceimages/".to_owned(),
                "hair.dds".to_owned()
            )
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(rewrites.findings.len(), 1);
        assert_eq!(
            rewrites.findings[0].code.code.as_ref(),
            "common_reference_kept"
        );
        assert_eq!(
            rewrites.findings[0].context,
            vec![(
                "reference".to_owned(),
                "/Assets/pes16/model/character/common/000/sourceimages/hair.dds".to_owned()
            )]
        );
    }

    #[test]
    fn an_fmdl_common_reference_the_folder_claims_is_kept() {
        // `hair.dds` is in `Common/` but the folder holds a texture of its
        // stem: the link would clash — kept and reported.
        let face = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "hair.dds",
        )]);
        let rewrites = plan(
            &[
                (
                    "Faces/XXX07 - A/face_high.fmdl",
                    "Players/07 - A/face_high.fmdl",
                ),
                ("Faces/XXX07 - A/hair.dds", "Players/07 - A/hair.dds"),
            ],
            &["Common/hair.dds"],
            &[
                ("Faces/XXX07 - A/face_high.fmdl", &face),
                ("Faces/XXX07 - A/hair.dds", b"h"),
            ],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            (
                "/Assets/pes16/model/character/common/714/sourceimages/".to_owned(),
                "hair.dds".to_owned()
            )
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn an_fmdl_common_reference_in_another_letter_case_respells_the_marker() {
        let face = fmdl(&[(
            "/ASSETS/PES16/MODEL/CHARACTER/COMMON/000/sourceimages/",
            "hair.dds",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            (
                "/ASSETS/PES16/MODEL/CHARACTER/COMMON/714/sourceimages/".to_owned(),
                "hair.dds".to_owned()
            )
        );
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn an_fmdl_in_a_subfolder_naming_a_common_subfolder_is_kept() {
        // `realkit/x.fmdl` naming `common/000/Boots/…`: the path below the
        // model folder and the mirrored link cannot agree — kept.
        let kit = fmdl(&[(
            "/Assets/pes16/model/character/common/000/Boots/sourceimages/",
            "x.dds",
        )]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/realkit/x.fmdl",
                "Players/07 - A/realkit/x.fmdl",
            )],
            &["Common/Boots/x.dds"],
            &[("Faces/XXX07 - A/realkit/x.fmdl", &kit)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/realkit/x.fmdl")[0],
            (
                "/Assets/pes16/model/character/common/714/Boots/sourceimages/".to_owned(),
                "x.dds".to_owned()
            )
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn the_fox_common_reference_needs_the_marker_and_sourceimages() {
        assert_eq!(
            reference_into_fox_common("/Assets/pes16/model/character/common/000/sourceimages/"),
            Some(("000", ""))
        );
        // A marker of another length is none even when the rest ends in
        // `sourceimages/`; a three-character marker's rest must end in it.
        for directory in [
            "/Assets/pes16/model/character/common/00/sourceimages/",
            "/Assets/pes16/model/character/common/0000/sourceimages/",
            "/Assets/pes16/model/character/common/000/not_the_marker_end_at_all/",
        ] {
            assert_eq!(reference_into_fox_common(directory), None, "{directory}");
        }
    }

    #[test]
    fn fmdl_entries_the_classes_keep_are_not_written() {
        let face = fmdl(&[
            (
                "/Assets/pes16/model/character/common/000/sourceimages/",
                "dummy_kit.dds",
            ),
            (
                "/Assets/pes16/model/character/uniform/texture/",
                "u0XXXp1.dds",
            ),
            (
                "/Assets/pes16/model/character/common/sourceimages/",
                "zero.dds",
            ),
        ]);
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        // Nothing respelled, nothing linked, nothing reported: no rewrite.
        assert!(rewrites.written.is_empty());
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_other_directory_becomes_the_model_s_own_folder() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/face/real/00000/sourceimages/",
            "skin.dds",
        )]);
        // `skin.dds` in the folder: `./`, no link.
        let rewrites = plan(
            &[
                (
                    "Faces/XXX07 - A/face_high.fmdl",
                    "Players/07 - A/face_high.fmdl",
                ),
                ("Faces/XXX07 - A/skin.dds", "Players/07 - A/skin.dds"),
            ],
            &[],
            &[
                ("Faces/XXX07 - A/face_high.fmdl", &face),
                ("Faces/XXX07 - A/skin.dds", b"s"),
            ],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "skin.dds".to_owned())
        );
        assert!(rewrites.links.is_empty());
        // `skin.dds` only in `Common/`: `./` and the link.
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &["Common/skin.dds"],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "skin.dds".to_owned())
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/skin.dds.common".to_owned()]
        );
        // Nowhere: `./`, no link, no finding.
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "skin.dds".to_owned())
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_kit_name_links_when_a_member_sits_in_the_folder() {
        let face = fmdl(&[(
            "/Assets/pes16/model/character/face/real/00000/sourceimages/",
            "u0805p0.dds",
        )]);
        let rewrites = plan(
            &[
                (
                    "Faces/XXX07 - A/face_high.fmdl",
                    "Players/07 - A/face_high.fmdl",
                ),
                ("Faces/XXX07 - A/kit1.dds", "Players/07 - A/kit1.dds"),
            ],
            &[],
            &[
                ("Faces/XXX07 - A/face_high.fmdl", &face),
                ("Faces/XXX07 - A/kit1.dds", b"k"),
            ],
        );
        // `kit1.dds` in the folder is a member of `kitN`'s set: `./` `kitN`,
        // no link.
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            ("./".to_owned(), "kitN.dds".to_owned())
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_fmdl_keeps_its_common_references() {
        let boots = fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "x.dds",
        )]);
        let rewrites = plan(
            &[("common", "Common/boots.fmdl")],
            &["Common/x.dds"],
            &[("common", &boots)],
        );
        assert_eq!(
            fmdl_paths(&rewrites, "Common/boots.fmdl")[0],
            (
                "/Assets/pes16/model/character/common/714/sourceimages/".to_owned(),
                "x.dds".to_owned()
            )
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_that_does_not_read_is_copied_as_it_is() {
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", b"fmdl")],
        );
        assert!(rewrites.written.is_empty());
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn an_fmdl_the_lift_refuses_keeps_its_rewrite_and_finding() {
        // Mesh 0's material index dangling: `FmdlFile::read` accepts the
        // file, `Model::from_file` refuses it.
        let mut file = FmdlFile::read(&fmdl(&[(
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "hair.dds",
        )]))
        .unwrap();
        file.meshes[0].material_instance_id = u16::try_from(file.material_instances.len()).unwrap();
        let face = file.write();
        let rewrites = plan(
            &[(
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            )],
            &[],
            &[("Faces/XXX07 - A/face_high.fmdl", &face)],
        );
        // The lift failed after the texture rewrite: the file is written
        // with it, and the finding the rewrite made is reported.
        assert_eq!(
            fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
            (
                "/Assets/pes16/model/character/common/714/sourceimages/".to_owned(),
                "hair.dds".to_owned()
            )
        );
        assert_eq!(codes(&rewrites), ["common_reference_kept"]);
    }

    /// `material_references` for the team `/co/` over `copies` and `common`,
    /// each `.mtl`'s bytes behind `read`; a listed copy without bytes is a
    /// test bug.
    fn plan(copies: &[(&str, &str)], common: &[&str], bytes: &[(&str, &[u8])]) -> MaterialRewrites {
        plan_for(&team(), copies, common, bytes)
    }

    /// `plan` for an export of `identity`.
    fn plan_for(
        identity: &Identity,
        copies: &[(&str, &str)],
        common: &[&str],
        bytes: &[(&str, &[u8])],
    ) -> MaterialRewrites {
        let copies: Vec<(String, String)> = copies
            .iter()
            .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
            .collect();
        let common: Vec<String> = common.iter().map(|file| (*file).to_owned()).collect();
        let bytes: BTreeMap<&str, &[u8]> = bytes.iter().copied().collect();
        material_references(&copies, &common, identity, |path| {
            bytes
                .get(path)
                .map(|bytes| Ok(bytes.to_vec()))
                .unwrap_or_else(|| panic!("a listed copy has no bytes: {path}"))
        })
    }

    #[test]
    fn a_local_reference_s_kit_magic_is_respelled() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &[],
            &[("face", &mtl(&["./skin_u0758p0.dds"]))],
        );
        assert_eq!(rewrites.links.len(), 0);
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(text.contains("path=\"./skin_kitN.dds\""), "{text}");
    }

    #[test]
    fn a_game_directory_reference_is_left_as_written() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &[],
            &[(
                "face",
                &mtl(&["model/character/uniform/texture/u0814p1.dds"]),
            )],
        );
        // Nothing changed: no rewrite at all.
        assert!(rewrites.written.is_empty());
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_keeper_reference_names_the_respelled_file() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &["Common/u0714g1.dds"],
            &[(
                "face",
                &mtl(&["model/character/uniform/common/XXX/u0XXXg1.dds"]),
            )],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(text.contains("path=\"./u0714g1.dds\""), "{text}");
        assert_eq!(
            rewrites.links,
            vec!["Players/07 - A/u0714g1.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_dummy_stem_gets_its_marker_respelled_alone() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &[],
            &[(
                "face",
                &mtl(&["model/character/uniform/common/XXX/dummy_kit.dds"]),
            )],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/dummy_kit.dds\""),
            "{text}"
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_dummy_stem_in_a_sub_keeps_the_sub_as_written() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &[],
            &[(
                "face",
                &mtl(&["model/character/uniform/common/XXX/u0XXXp0/dummy_kit.dds"]),
            )],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/u0XXXp0/dummy_kit.dds\""),
            "{text}"
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_kit_reference_links_the_set_s_members() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &["Common/kit1.dds", "Common/kit2.dds"],
            &[(
                "face",
                &mtl(&["model/character/uniform/common/XXX/u0XXXp0.dds"]),
            )],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(text.contains("path=\"./kitN.dds\""), "{text}");
        assert_eq!(
            rewrites.links,
            vec!["Players/07 - A/kitN.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_kit_set_reference_with_a_member_in_the_folder_is_kept() {
        let rewrites = plan(
            &[
                ("face", "Players/07 - A/face.mtl"),
                ("kit1", "Players/07 - A/kit1.dds"),
            ],
            &["Common/kit1.dds", "Common/kit2.dds"],
            &[
                (
                    "face",
                    &mtl(&["model/character/uniform/common/XXX/u0XXXp0.dds"]),
                ),
                ("kit1", b"dds"),
            ],
        );
        assert_eq!(rewrites.links.len(), 0);
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/kitN.dds\""),
            "{text}"
        );
        assert_eq!(rewrites.findings.len(), 1);
        assert_eq!(
            rewrites.findings[0].code.code.as_ref(),
            "common_reference_kept"
        );
    }

    #[test]
    fn a_shared_folder_mtl_respells_a_local_reference() {
        let rewrites = plan(
            &[("boots", "Boots/Crocs/boots.mtl")],
            &[],
            &[("boots", &mtl(&["./u0XXXp1.dds"]))],
        );
        let text = written_text(&rewrites, "Boots/Crocs/boots.mtl");
        assert!(text.contains("path=\"./kit1.dds\""), "{text}");
    }

    #[test]
    fn a_shared_folder_mtl_reference_to_a_missing_common_file_is_kept() {
        let rewrites = plan(
            &[("boots", "Boots/Crocs/boots.mtl")],
            &[],
            &[(
                "boots",
                &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
            )],
        );
        assert!(rewrites.links.is_empty());
        let text = written_text(&rewrites, "Boots/Crocs/boots.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/hair.dds\""),
            "{text}"
        );
        assert_eq!(rewrites.findings.len(), 1);
        assert_eq!(
            rewrites.findings[0].code.code.as_ref(),
            "common_reference_kept"
        );
        assert_eq!(
            rewrites.findings[0].context,
            vec![(
                "reference".to_owned(),
                "model/character/uniform/common/XXX/hair.dds".to_owned()
            )]
        );
    }

    #[test]
    fn two_references_in_one_mtl_link_their_own_common_files() {
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &["Common/hair.dds", "Common/jessie/hair.dds"],
            &[(
                "face",
                &mtl(&[
                    "model/character/uniform/common/XXX/hair.dds",
                    "model/character/uniform/common/XXX/jessie/hair.dds",
                ]),
            )],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(text.contains("path=\"./hair.dds\""), "{text}");
        assert!(text.contains("path=\"./jessie/hair.dds\""), "{text}");
        assert_eq!(
            rewrites.links,
            [
                "Players/07 - A/hair.dds.common".to_owned(),
                "Players/07 - A/jessie/hair.dds.common".to_owned(),
            ]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_common_mtl_keeps_its_common_references() {
        let rewrites = plan(
            &[("mtl", "Common/hair.mtl")],
            &["Common/hair.dds"],
            &[(
                "mtl",
                &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
            )],
        );
        let text = written_text(&rewrites, "Common/hair.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/hair.dds\""),
            "{text}"
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_named_common_segment_links_as_the_team_s_common() {
        // `common/vg/` is the team's Common as `XXX` is: the compiler reads
        // any segment as the team's own there.
        let face = mtl(&["model/character/uniform/common/vg/env.dds"]);
        let rewrites = plan_for(
            &vg(),
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/env.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(out.contains("./env.dds"), "{out}");
        assert_eq!(rewrites.links, ["Players/07 - A/env.dds.common".to_owned()]);
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_named_common_segment_kept_is_respelled_the_id() {
        let face = mtl(&["model/character/uniform/common/VG/fbm_s.dds"]);
        let rewrites = plan_for(
            &vg(),
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &[],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            out.contains("model/character/uniform/common/758/fbm_s.dds"),
            "{out}"
        );
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn a_common_mtl_s_segment_of_another_length_is_respelled_the_id() {
        let file = mtl(&["model/character/uniform/common/refs/refkit/texture.dds"]);
        let rewrites = plan_for(
            &vg(),
            &[("Common/refkit.mtl", "Common/refkit.mtl")],
            &[],
            &[("Common/refkit.mtl", &file)],
        );
        let out = written_text(&rewrites, "Common/refkit.mtl");
        assert!(
            out.contains("model/character/uniform/common/758/refkit/texture.dds"),
            "{out}"
        );
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn model_folder_mode_respells_a_short_segment_to_the_placeholder() {
        let file = mtl(&[
            "model/character/uniform/common/vg/x.dds",
            "model/character/uniform/common/abc/x.dds",
        ]);
        let rewrites = plan_for(
            &Identity::ModelFolders,
            &[("k0431/x.mtl", "Boots/k0431/x.mtl")],
            &[],
            &[("k0431/x.mtl", &file)],
        );
        let out = written_text(&rewrites, "Boots/k0431/x.mtl");
        assert!(
            out.contains("model/character/uniform/common/XXX/x.dds"),
            "{out}"
        );
        assert!(
            out.contains("model/character/uniform/common/abc/x.dds"),
            "{out}"
        );
    }

    /// The one output path's `.mtl` text, parsed back.
    fn written_text(rewrites: &MaterialRewrites, output: &str) -> String {
        let (_, bytes) = rewrites
            .written
            .iter()
            .find(|(path, _)| path == output)
            .expect("the .mtl was rewritten");
        String::from_utf8(bytes.clone()).unwrap()
    }

    #[test]
    fn a_reference_into_common_becomes_the_bare_name_and_the_link() {
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                ("Faces/XXX07 - A/face.fmdl", "Players/07 - A/face.fmdl"),
            ],
            &["Common/hair.dds"],
            &[
                ("Faces/XXX07 - A/face.mtl", &face),
                ("Faces/XXX07 - A/face.fmdl", b"x"),
            ],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(out.contains("./hair.dds"), "{out}");
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/hair.dds.common".to_owned()]
        );
        // The marker's own spelling and case likewise.
        let face = mtl(&["MODEL/Character/Uniform/Common/714/Hair.DDS"]);
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(out.contains("./Hair.DDS"), "{out}");
        // `Common/hair.png` is found by stem and the link names its file.
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/hair.png"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/hair.png.common".to_owned()]
        );
    }

    #[test]
    fn a_common_subfolder_reference_links_at_the_path_below_the_folder() {
        let face = mtl(&["model/character/uniform/common/XXX/jessie/hair.dds"]);
        let rewrites = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                (
                    "Faces/XXX07 - A/realkit/x.mtl",
                    "Players/07 - A/realkit/x.mtl",
                ),
            ],
            &["Common/jessie/hair.dds"],
            &[
                ("Faces/XXX07 - A/face.mtl", &face),
                ("Faces/XXX07 - A/realkit/x.mtl", &face),
            ],
        );
        // The root `.mtl` gets the path below the model folder and the
        // mirrored link; the subfolder `.mtl`'s is kept, `???` respelled.
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(out.contains("./jessie/hair.dds"), "{out}");
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/jessie/hair.dds.common".to_owned()]
        );
        let out = written_text(&rewrites, "Players/07 - A/realkit/x.mtl");
        assert!(
            out.contains("model/character/uniform/common/714/jessie/hair.dds"),
            "{out}"
        );
        let kept = rewrites
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "common_reference_kept")
            .expect("common_reference_kept expected");
        assert_eq!(
            kept.context,
            vec![(
                "reference".to_owned(),
                "model/character/uniform/common/XXX/jessie/hair.dds".to_owned()
            )]
        );
        assert_eq!(
            kept.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Faces/XXX07 - A/realkit/x.mtl")
            }
        );
    }

    #[test]
    fn a_folder_file_of_the_stem_keeps_the_reference() {
        // `Players/07 - A/` already holds `hair.png`: the link would clash
        // or lose to the nearer file — kept.
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                ("Faces/XXX07 - A/hair.png", "Players/07 - A/hair.png"),
            ],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            out.contains("model/character/uniform/common/714/hair.dds"),
            "{out}"
        );
        assert!(rewrites.links.is_empty());
        // From a subfolder `.mtl` the bare name looks up to the model folder,
        // so the root's `hair.png` keeps it too.
        let rewrites = plan(
            &[
                (
                    "Faces/XXX07 - A/realkit/apc.mtl",
                    "Players/07 - A/realkit/apc.mtl",
                ),
                ("Faces/XXX07 - A/hair.png", "Players/07 - A/hair.png"),
            ],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/realkit/apc.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/realkit/apc.mtl");
        assert!(
            out.contains("model/character/uniform/common/714/hair.dds"),
            "{out}"
        );
        assert!(rewrites.links.is_empty());
        // A model's link of that stem claims no texture: the link is made.
        let rewrites = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                (
                    "Faces/XXX07 - A/hair.model.common",
                    "Players/07 - A/hair.model.common",
                ),
            ],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        assert_eq!(
            rewrites.links,
            ["Players/07 - A/hair.dds.common".to_owned()]
        );
        // Two `.mtl` files naming one `Common/` file share its one link.
        let two = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                ("Faces/XXX07 - A/other.mtl", "Players/07 - A/other.mtl"),
            ],
            &["Common/hair.dds"],
            &[
                ("Faces/XXX07 - A/face.mtl", &face),
                ("Faces/XXX07 - A/other.mtl", &face),
            ],
        );
        assert_eq!(two.links, ["Players/07 - A/hair.dds.common".to_owned()]);
        for output in ["Players/07 - A/face.mtl", "Players/07 - A/other.mtl"] {
            assert!(
                written_text(&two, output).contains("./hair.dds"),
                "{output}"
            );
        }
    }

    #[test]
    fn an_mtl_naming_nothing_stays_as_it_is_an_unparsable_one_is_unmigrated() {
        // `./face.dds` and `model/character/face/common/` are not the team's
        // Common — nothing changes, so nothing is written. `bad.mtl` does
        // not parse even dropping stray text: unmigrated content, its parse
        // error in the context (step 7); the copy loop still carries it.
        let unchanged = mtl(&["./face.dds", "model/character/face/common/face_eyelash.dds"]);
        let bad = b"mtl".to_vec();
        let plan = plan(
            &[
                ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
                ("Faces/XXX07 - A/bad.mtl", "Players/07 - A/bad.mtl"),
            ],
            &["Common/hair.dds"],
            &[
                ("Faces/XXX07 - A/face.mtl", &unchanged),
                ("Faces/XXX07 - A/bad.mtl", &bad),
            ],
        );
        assert!(plan.written.is_empty());
        assert!(plan.links.is_empty());
        assert_eq!(codes(&plan), ["unmigrated_content"]);
        let finding = &plan.findings[0];
        assert_eq!(
            finding.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Faces/XXX07 - A/bad.mtl"),
            }
        );
        assert_eq!(
            finding.context[0],
            ("reason".to_owned(), "mtl_unparsable".to_owned())
        );
        assert!(
            finding.context.iter().any(|(key, _)| key == "error"),
            "{:?}",
            finding.context
        );
    }

    #[test]
    fn stray_text_is_dropped_and_the_set_written_back_as_it_came() {
        // Step 7's clean (the real `72117 - Lisa`'s `apc.mtl` `\`): the text
        // is dropped on the tolerant read, the set written back — wrapped
        // again when it was — and the file reported. Neither reference
        // changes, so the write-back is the stray text's alone.
        let stray = mtl(&["./skin.dds"]);
        let stray = String::from_utf8(stray)
            .unwrap()
            .replace("    </material>", "\\\n    </material>")
            .into_bytes();
        let wrapped = wezlib::compress(&stray);
        let rewrites = plan(
            &[
                ("Faces/XXX07 - A/apc.mtl", "Players/07 - A/apc.mtl"),
                ("Faces/XXX07 - A/wrapped.mtl", "Players/07 - A/wrapped.mtl"),
            ],
            &[],
            &[
                ("Faces/XXX07 - A/apc.mtl", &stray),
                ("Faces/XXX07 - A/wrapped.mtl", &wrapped),
            ],
        );
        for (output, was_wrapped) in [
            ("Players/07 - A/apc.mtl", false),
            ("Players/07 - A/wrapped.mtl", true),
        ] {
            let (_, bytes) = rewrites
                .written
                .iter()
                .find(|(path, _)| path == output)
                .unwrap_or_else(|| panic!("{output} was not written back"));
            assert_eq!(wezlib::is_wrapped(bytes), was_wrapped, "{output}");
            let set = MaterialSet::read(bytes).expect("the written set parses");
            assert_eq!(set.materials.len(), 1, "{output}");
        }
        assert_eq!(
            codes(&rewrites),
            ["mtl_stray_text_dropped", "mtl_stray_text_dropped"]
        );
    }

    #[test]
    fn the_common_file_must_match_both_folder_and_stem() {
        // `Common/` holds `beard.dds` (the folder, not the stem) and
        // `jessie/hair.dds` (the stem, not the folder): `hair` is kept.
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/beard.dds", "Common/jessie/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let out = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            out.contains("model/character/uniform/common/714/hair.dds"),
            "{out}"
        );
        assert!(rewrites.links.is_empty());
    }

    #[test]
    fn two_common_files_named_from_one_folder_both_link() {
        let face = mtl(&[
            "model/character/uniform/common/XXX/hair.dds",
            "model/character/uniform/common/XXX/beard.dds",
        ]);
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/beard.dds", "Common/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        assert_eq!(
            rewrites.links,
            [
                "Players/07 - A/beard.dds.common".to_owned(),
                "Players/07 - A/hair.dds.common".to_owned()
            ]
        );
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_shared_folder_mtl_keeps_the_reference_with_the_team_id() {
        // A `.mtl` of a shared `Boots/`/`Gloves/` folder keeps its Common
        // references as written but `???`, respelled the team's ID — no
        // finding and no link (the pre-Fox compile reads no shared folder's
        // `.common` links).
        let boots = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[("Boots/k0430 - Crocs/boots.mtl", "Boots/Crocs/boots.mtl")],
            &["Common/hair.dds"],
            &[("Boots/k0430 - Crocs/boots.mtl", &boots)],
        );
        let out = written_text(&rewrites, "Boots/Crocs/boots.mtl");
        assert!(
            out.contains("model/character/uniform/common/714/hair.dds"),
            "{out}"
        );
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty());
    }

    #[test]
    fn a_non_texture_of_the_stem_in_common_is_no_link_target() {
        // `Common/hair.mtl` shares the stem but is no texture: kept and
        // warned, no link.
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/hair.mtl"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn two_textures_of_the_stem_in_one_common_directory_keep_the_reference() {
        // `Common/hair.dds` beside `Common/hair.png`: both textures of the
        // stem in the directory, so no single file the link could spell —
        // kept and warned.
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/hair.dds", "Common/hair.png"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(
            rewrites
                .findings
                .iter()
                .map(|m| m.code.code.as_ref())
                .collect::<Vec<_>>(),
            ["common_reference_kept"]
        );
    }

    #[test]
    fn a_rewritten_mtl_keeps_the_wesys_wrap() {
        let face = wezlib::compress(&mtl(&["model/character/uniform/common/XXX/hair.dds"]));
        let rewrites = plan(
            &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
            &["Common/hair.dds"],
            &[("Faces/XXX07 - A/face.mtl", &face)],
        );
        let (_, bytes) = rewrites
            .written
            .iter()
            .find(|(path, _)| path == "Players/07 - A/face.mtl")
            .expect("the .mtl was rewritten");
        assert!(wezlib::is_wrapped(bytes));
        let out = String::from_utf8(wezlib::decompress(bytes).unwrap()).unwrap();
        assert!(out.contains("./hair.dds"), "{out}");
    }

    /// The finding codes of `rewrites`, in order.
    fn codes(rewrites: &MaterialRewrites) -> Vec<&str> {
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect()
    }

    #[test]
    fn a_referee_common_reference_his_folder_holds_names_it_there() {
        let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
        let rewrites = plan_for(
            &refs(),
            &[
                ("face", "Players/arbiter/face.mtl"),
                ("hair", "Players/arbiter/hair.dds"),
            ],
            &[],
            &[("face", &face), ("hair", b"h")],
        );
        let text = written_text(&rewrites, "Players/arbiter/face.mtl");
        assert!(text.contains("path=\"./hair.dds\""), "{text}");
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
        // A team's folder holding the stem keeps the reference, as before.
        let rewrites = plan(
            &[
                ("face", "Players/07 - A/face.mtl"),
                ("hair", "Players/07 - A/hair.dds"),
            ],
            &[],
            &[("face", &face), ("hair", b"h")],
        );
        let text = written_text(&rewrites, "Players/07 - A/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/hair.dds\""),
            "{text}"
        );
        assert!(rewrites.links.is_empty());
        assert_eq!(codes(&rewrites), ["common_reference_kept"]);
    }

    #[test]
    fn a_referee_common_subfolder_reference_names_the_folder_s_subfolder() {
        let face = mtl(&["model/character/uniform/common/XXX/kit/sock.dds"]);
        let rewrites = plan_for(
            &refs(),
            &[
                ("face", "Players/arbiter/face.mtl"),
                ("sock", "Players/arbiter/kit/sock.dds"),
            ],
            &[],
            &[("face", &face), ("sock", b"s")],
        );
        let text = written_text(&rewrites, "Players/arbiter/face.mtl");
        assert!(text.contains("path=\"./kit/sock.dds\""), "{text}");
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
        // From a subfolder's `.mtl`, `./kit/` would name `kit/kit/`: the
        // team rule keeps the reference.
        let rewrites = plan_for(
            &refs(),
            &[
                ("face", "Players/arbiter/kit/face.mtl"),
                ("sock", "Players/arbiter/kit/sock.dds"),
            ],
            &[],
            &[("face", &face), ("sock", b"s")],
        );
        let text = written_text(&rewrites, "Players/arbiter/kit/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/999/kit/sock.dds\""),
            "{text}"
        );
        assert_eq!(codes(&rewrites), ["common_reference_kept"]);
    }

    #[test]
    fn a_referee_common_reference_his_folder_lacks_links_the_export_s_common() {
        let face = mtl(&["model/character/uniform/common/XXX/moe.dds"]);
        let rewrites = plan_for(
            &refs(),
            &[("face", "Players/arbiter/face.mtl")],
            &["Common/moe.dds"],
            &[("face", &face)],
        );
        let text = written_text(&rewrites, "Players/arbiter/face.mtl");
        assert!(text.contains("path=\"./moe.dds\""), "{text}");
        assert_eq!(
            rewrites.links,
            ["Players/arbiter/moe.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    }

    #[test]
    fn a_referee_bare_reference_only_common_holds_is_linked() {
        let face = mtl(&["./whistle.dds"]);
        let rewrites = plan_for(
            &refs(),
            &[("face", "Players/arbiter/face.mtl")],
            &["Common/whistle.dds"],
            &[("face", &face)],
        );
        // The reference is already the Studio spelling: no rewrite.
        assert!(rewrites.written.is_empty());
        assert_eq!(
            rewrites.links,
            ["Players/arbiter/whistle.dds.common".to_owned()]
        );
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
        // A team's bare reference gets no link.
        let rewrites = plan(
            &[("face", "Players/07 - A/face.mtl")],
            &["Common/whistle.dds"],
            &[("face", &face)],
        );
        assert!(rewrites.written.is_empty());
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    }

    #[test]
    fn a_referee_bare_reference_his_folder_holds_gets_no_link() {
        let face = mtl(&["./skin.dds"]);
        let rewrites = plan_for(
            &refs(),
            &[
                ("face", "Players/arbiter/face.mtl"),
                ("skin", "Players/arbiter/skin.dds"),
            ],
            &["Common/skin.dds"],
            &[("face", &face), ("skin", b"s")],
        );
        assert!(rewrites.written.is_empty());
        assert!(rewrites.links.is_empty());
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    }

    #[test]
    fn a_referee_fmdl_names_his_folder_s_texture_and_links_a_bare_common_one() {
        let boots = fmdl(&[
            (
                "/Assets/pes16/model/character/common/000/sourceimages/",
                "arbiter.dds",
            ),
            ("./", "whistle"),
            ("", "horn"),
        ]);
        let rewrites = plan_for(
            &refs(),
            &[
                ("boots", "Players/arbiter/boots.fmdl"),
                ("arbiter", "Players/arbiter/arbiter.dds"),
            ],
            &["Common/horn.dds", "Common/whistle.dds"],
            &[("boots", &boots), ("arbiter", b"a")],
        );
        let paths = fmdl_paths(&rewrites, "Players/arbiter/boots.fmdl");
        assert_eq!(paths[0], ("./".to_owned(), "arbiter.dds".to_owned()));
        assert_eq!(paths[1], ("./".to_owned(), "whistle".to_owned()));
        assert_eq!(paths[2], ("".to_owned(), "horn".to_owned()));
        // An empty directory is a bare reference too.
        assert_eq!(
            rewrites.links,
            [
                "Players/arbiter/horn.dds.common".to_owned(),
                "Players/arbiter/whistle.dds.common".to_owned()
            ]
        );
        assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    }

    #[test]
    fn a_model_folders_common_reference_keeps_the_marker_as_written() {
        // Model-folder mode has no team: the kept reference's `XXX` stays,
        // its kit magic still respelled, the missing file still reported.
        let face = mtl(&["model/character/uniform/common/XXX/u0XXXp1/missing.dds"]);
        let rewrites = plan_for(
            &Identity::ModelFolders,
            &[("face", "Players/15 - Snuffy/face.mtl")],
            &[],
            &[("face", &face)],
        );
        let text = written_text(&rewrites, "Players/15 - Snuffy/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/XXX/kit1/missing.dds\""),
            "{text}"
        );
        assert_eq!(codes(&rewrites), ["common_reference_kept"]);
        // A team's marker becomes its ID.
        let rewrites = plan(
            &[("face", "Players/15 - Snuffy/face.mtl")],
            &[],
            &[("face", &face)],
        );
        let text = written_text(&rewrites, "Players/15 - Snuffy/face.mtl");
        assert!(
            text.contains("path=\"model/character/uniform/common/714/kit1/missing.dds\""),
            "{text}"
        );
    }
}
