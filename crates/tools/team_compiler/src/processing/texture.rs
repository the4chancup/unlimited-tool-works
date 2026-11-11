//! Texture conversion for the target's engine, FTEX on Fox, DDS on pre-Fox. Every accepted
//! image format goes through `dds_convert`, which picks the codec the target version reads. A
//! model folder's textures are converted as one unit, and the export's Common textures as
//! another. A portrait becomes the DDS every engine reads.
//!
//! A texture's signature and size, a portrait's included, are checked by the deep pass before
//! planning. What conversion itself finds (`texture_codec_unsupported`) the task fails with,
//! and what a failure drops is the task's unit: a folder.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use aesthetics_export::{
    FileDescriptor, KitToken, kit_path_token, variant_path, without_kit_token,
};
use dds_convert::{
    BlockCodec, ConvertError, SourceFormat, Target, TextureRole, decode, encode_dds, probe,
    source_hash,
};
use ftex::FtexError;
use ftex::dds::{DdsPixel, read_layout};
use pes_version::{Engine, PesVersion};
use pipeline::{MemoryBudget, Permit};
use vtree::ScopePath;

use super::{CompileContext, Entry, Finding, TaskFailure, TaskFiles, take};
use crate::deep::relative;
use crate::kit_variants::{self, member_kit};
use crate::messages::Code;
use crate::paths;
use crate::plan::roles::{ModelPackage, PlayerFile, file_stem, texture_format};
use crate::plan::{ENVIRONMENT_MAP_STEM, ModelFolder};

/// Why a texture could not be converted.
#[derive(Debug)]
pub(super) enum TextureError {
    /// One of the texture findings, on the file named: it is the task's failure, carrying its
    /// code.
    Finding(Code, String),
    /// Any other failure, which is the ordinary `folder_pack_failed`.
    Other(anyhow::Error),
}

impl From<TextureError> for TaskFailure {
    fn from(error: TextureError) -> TaskFailure {
        match error {
            TextureError::Finding(code, file) => TaskFailure {
                code,
                context: vec![("file", file)],
            },
            TextureError::Other(error) => TaskFailure::from(error),
        }
    }
}

/// The failure of converting the file `name`. What `dds_convert` refuses as a codec and what
/// `ftex::dds::read_layout` refuses as a kind of DDS (a signed block format, a volume texture,
/// an array, a paletted DDS, an incomplete cube map) are one class, a file of a kind the
/// target cannot be given: the catalog's file-level `texture_codec_unsupported`. A header that
/// cannot be read, or pixel data cut short, is the ordinary failure naming the file, which
/// fails the task. The cube-map route's `ftex::dds_to_ftex` error is mapped here too, so its
/// refusals take the same finding. The deep pass maps a header probe's errors by the same two
/// arms (`deep::texture::probe_failure`).
pub(super) fn conversion_failure(name: &str, error: ConvertError) -> TextureError {
    match error {
        ConvertError::Unsupported(_) | ConvertError::Ftex(FtexError::UnsupportedDds(_)) => {
            TextureError::Finding(Code::TextureCodecUnsupported, name.to_owned())
        }
        ConvertError::Ftex(_)
        | ConvertError::Wesys(_)
        | ConvertError::Image(_)
        | ConvertError::InvalidDecoded(_)
        | ConvertError::Truncated => TextureError::Other(
            anyhow::Error::from(error).context(format!("cannot convert {name}")),
        ),
    }
}

/// One source's copy of a texture: the package the source feeds, the texture's path below the
/// source folder without its extension as the source spells it (`PlayerFile::Texture`), the
/// converted bytes, and the source folder's export path for a combined folder's copy.
struct TextureCopy {
    package: ModelPackage,
    below: String,
    bytes: Vec<u8>,
    /// The source folder's export path (`Boots/Crocs`) for a combined folder's copy; `None`
    /// for a copy of the player's own folder.
    source: Option<String>,
}

/// A combined shared folder's `.common` link claiming a path (`PlayerFile::CommonTexture`):
/// the package its source feeds for the conflict rule's canonical order, the folder's export
/// path for `shared_texture_overridden`, and the path it stands at as spelled. It stands at
/// the path as its folder's copy would, and its `Common/` file serves it, so it always
/// differs from a file there, while two links at one path name the one `Common/` file.
struct LinkCopy {
    package: ModelPackage,
    folder: String,
    below: String,
}

/// The textures of `folder`, its own and its combined folders', converted from their bytes in
/// `files` into the folder's texture home for team `team_id`, each at its path below its source
/// folder (`PlayerFile::Texture`), compared case-folded: one entry per path, however many of the
/// folder's models use it and however many ids its packages are emitted under, so a
/// subfolder's `jessie/skin.dds` and the root's `skin.dds` are two entries
/// (`spec/aesthetics_export/player_folders.md` "Subfolders"). A combined shared folder's files
/// stand at their paths below it, so its `jessie/skin.dds` and the root's are two entries too.
/// A path several sources hold is one entry when their bytes agree.
///
/// Local files win over imported ones: a path the player's own folder holds, as a texture or
/// as a texture link standing there (`PlayerFile::CommonTexture`), is his whatever the
/// packages. His copy is the entry every model of his names, a link's path gets no entry, and
/// each combined folder's copy of the path is left out. `shared_texture_overridden` is noted
/// in `findings` when its bytes differ from his, a link always counting as differing. Two
/// combined folders of different
/// packages holding one path with different bytes is `shared_texture_conflict`, which fails
/// the task and leaves the player folder out; two sources of one package cannot disagree, the
/// player's own copy winning first and a player combining one folder of each kind. A folder
/// taking the template environment map (`ModelFolder::takes_template_environment_map`) gets
/// it as `env.dds`, emitted as it is. The textures kept then have their kit variant sets
/// completed against `kits` (`complete_kit_variants`).
pub(super) fn folder_textures(
    folder: &ModelFolder,
    kits: &[u8],
    team_id: u16,
    ctx: &CompileContext,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<Vec<Entry>, TaskFailure> {
    // Each path's copies in source order: the player's own folder's, then each combined
    // folder's; the player's own texture links, each by the folded path it stands at; and
    // each combined folder's, which stand at their paths as their folders' copies do.
    let mut copies: BTreeMap<String, Vec<TextureCopy>> = BTreeMap::new();
    let mut own_links: BTreeMap<String, String> = BTreeMap::new();
    let mut combined_links: BTreeMap<String, Vec<LinkCopy>> = BTreeMap::new();
    for (package, source_path, source_files) in folder.roles() {
        // `roles` yields the folder's own files first; a combined folder's copy carries its
        // export path for `shared_texture_overridden`'s `folder`.
        let combined = (source_path != &folder.path).then(|| source_path.as_str().to_owned());
        for (file, role) in source_files {
            match role {
                PlayerFile::Texture { below, format } => {
                    let bytes = convert(ctx, format, file.path.name(), &take(files, file), true)?;
                    copies
                        .entry(vtree::fold_name(&below))
                        .or_default()
                        .push(TextureCopy {
                            package,
                            below,
                            bytes,
                            source: combined.clone(),
                        });
                }
                PlayerFile::CommonTexture(below) => match &combined {
                    // A link of his stands at the path as the texture it names: it wins
                    // every copy there (`resolve_path`), and writes no entry of its own.
                    None => {
                        own_links.entry(vtree::fold_name(&below)).or_insert(below);
                    }
                    // A combined folder's link is a source with its folder's package, a
                    // claimant of the path like a copy (`resolve_path`).
                    Some(source) => {
                        combined_links
                            .entry(vtree::fold_name(&below))
                            .or_default()
                            .push(LinkCopy {
                                package,
                                folder: source.clone(),
                                below,
                            });
                    }
                },
                _ => {}
            }
        }
    }
    // Each path resolves as its copies and links claim it; a path only combined folders'
    // links hold needs no decision (the one `Common/` file they name serves it).
    let mut textures: Vec<(String, Vec<u8>)> = Vec::new();
    for (path, path_copies) in copies {
        let links = combined_links.get(&path).map_or(&[][..], Vec::as_slice);
        let Some(kept) = resolve_path(&path, &path_copies, &own_links, links, findings)? else {
            continue;
        };
        let copy = path_copies
            .into_iter()
            .nth(kept)
            .expect("the position found is in range");
        textures.push((copy.below, copy.bytes));
    }
    if folder.takes_template_environment_map() {
        textures.push((
            ENVIRONMENT_MAP_STEM.to_owned(),
            ctx.templates.environment_map().to_vec(),
        ));
    }
    complete_kit_variants(&mut textures, kits, ctx.version.engine(), findings);
    let entries = textures
        .into_iter()
        .map(|(below, bytes)| {
            let path = folder
                .textures
                .texture(ctx.version.engine(), team_id, &below);
            (path, bytes)
        })
        .collect();
    Ok(entries)
}

/// Completes the kit variant sets among `textures`, each a path below its folder without its
/// extension as spelled (a stem, for a texture directly in it) with its converted bytes,
/// against `kits`, the export's kit numbers, for a target of `engine`
/// (`spec/model_format.md` "Kit-dependent assets"). The members of one reference, compared
/// case-folded, are a set: its `kit1` to `kit9` variants and its `kitF` fallback, which is
/// never emitted under its own name. The set's fallback, its `kitF` member else
/// its lowest variant, serves every kit number the set lacks: on PES 15-17 it is emitted
/// under each missing number's spelling, and `kit_variant_missing` is noted in `findings`
/// for each when the fallback is the lowest variant; on PES 18-21 it is written once under
/// the reference's spelling (`pants_kitN`, `kitN/armor_bsm`), whatever `kits` holds, which
/// FoxDen's kit swapper loads for a kit without a variant, with no per-number copies. A
/// `kitF` fallback reports no finding. A variant of a number `kits` does not hold stays as
/// it is. The token may be the file name's or a whole directory segment's
/// (`kit1/armor_bsm` and `kit2/armor_bsm` are the set `kitN/armor_bsm`), spelled exactly:
/// `old_kit2_hair/skin` is no variant. A name's token wins over a directory's
/// (`kit1/pants_kit2` is `pants_kitN` in `kit1/`), and a set is one directory's: another
/// subfolder's files form their own sets there.
fn complete_kit_variants(
    textures: &mut Vec<(String, Vec<u8>)>,
    kits: &[u8],
    engine: Engine,
    findings: &mut Vec<Finding>,
) {
    // Every member as (kit number — `None` for the `kitF` fallback —, position in
    // `textures`, folded reference, reference spelled), sorted so each set is a run with
    // its fallback — its `kitF` member when it has one, else its lowest variant — first.
    let mut members: Vec<(Option<u8>, usize, String, String)> = textures
        .iter()
        .enumerate()
        // A set's key and reference are each member's path with its token respelled
        // `kitN`, the file name's token winning over a directory segment's
        // (`kit_path_token`).
        .filter_map(|(index, (below, _))| match kit_path_token(below)? {
            (token @ (KitToken::Variant(_) | KitToken::Fallback), reference) => Some((
                member_kit(token),
                index,
                vtree::fold_name(&reference),
                reference,
            )),
            (KitToken::Reference, _) => None,
        })
        .collect();
    members.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));
    // The entries to add, and the positions of the `kitF` members, which nothing emits.
    let mut added = Vec::new();
    let mut dropped = Vec::new();
    for set in members.chunk_by(|a, b| a.2 == b.2) {
        let fallback_index =
            kit_variants::fallback(set, |member| member.0).expect("every set has a member");
        let (fallback_kit, fallback, _, reference) = &set[fallback_index];
        let copied = textures[*fallback].0.clone();
        // Each entry owns its bytes: every copy of the fallback's is one.
        let bytes = || textures[*fallback].1.clone();
        let held = |kit: u8| set.iter().any(|(number, ..)| *number == Some(kit));
        match engine {
            Engine::Fox => {
                added.push((reference.clone(), bytes()));
            }
            Engine::PreFox => {
                for &kit in kits {
                    if held(kit) {
                        continue;
                    }
                    let variant = variant_path(&copied, kit)
                        .expect("a set member's path holds its kit token");
                    added.push((variant, bytes()));
                }
            }
        }
        if fallback_kit.is_none() {
            dropped.push(*fallback);
        } else {
            for &kit in kits {
                if held(kit) {
                    continue;
                }
                findings.push((
                    Code::KitVariantMissing,
                    vec![
                        ("texture", reference.clone()),
                        ("kit", kit.to_string()),
                        ("copied", copied.clone()),
                    ],
                ));
            }
        }
    }
    // Highest position first, so each removal leaves the lower positions where they were.
    dropped.sort_unstable_by(|a, b| b.cmp(a));
    for index in dropped {
        textures.remove(index);
    }
    textures.extend(added);
}

/// Decides one path held by `copies`, several sources' in source order, and by `links`, the
/// combined folders' texture links standing at it as their folders' copies do, noting its
/// findings in `findings`: the index of the copy the path's entry is written from, `None`
/// when it gets none (a texture link of the player's own standing there, the `Common/` file
/// it names serving it). A copy of the player's own
/// (its `source` `None`) wins whatever the packages, the local file over the imported ones:
/// it is the copy the path's entry is written from, each combined copy left out,
/// `shared_texture_overridden` noted when its bytes differ from his, identical bytes
/// deduplicating in silence — and a combined folder's link left out is noted the same way,
/// the link always differing from his file. A texture link of his standing at `path`
/// (`own_links`) wins the
/// same way and is noted for every combined copy and link, a link always counting as differing,
/// nothing comparing the Common texture's bytes. With neither, the combined folders'
/// claims resolve in canonical order (`face` > `boots` > `gloves`), a link standing at the
/// path as its folder's copy would and always differing from a file: a claim of another
/// package that differs from the winner's is `shared_texture_conflict` naming the path and
/// every package claiming it, which fails the player's textures task and so leaves his folder
/// out; two links at one path name the one
/// `Common/` file and deduplicate in silence. Two
/// copies of one package cannot disagree here: the player's own would have won first, and a
/// player combines one folder of each kind.
fn resolve_path(
    path: &str,
    copies: &[TextureCopy],
    own_links: &BTreeMap<String, String>,
    links: &[LinkCopy],
    findings: &mut Vec<Finding>,
) -> Result<Option<usize>, TaskFailure> {
    let overridden = |findings: &mut Vec<Finding>, texture: String, folder: String| {
        findings.push((
            Code::SharedTextureOverridden,
            vec![("texture", texture), ("folder", folder)],
        ));
    };
    if let Some(own) = copies.iter().position(|copy| copy.source.is_none()) {
        for copy in copies {
            let Some(source) = &copy.source else {
                continue;
            };
            if copy.bytes == copies[own].bytes {
                continue;
            }
            overridden(findings, copies[own].below.clone(), source.clone());
        }
        for link in links {
            overridden(findings, copies[own].below.clone(), link.folder.clone());
        }
        return Ok(Some(own));
    }
    if let Some(link) = own_links.get(path) {
        for copy in copies {
            overridden(
                findings,
                link.clone(),
                copy.source
                    .clone()
                    .expect("no own copy stands at the path: it would have won above"),
            );
        }
        // A combined folder's link here is nothing to report: his link and it stand for
        // the one `Common/` file the path names (`shared_texture_overridden`, "the two
        // copies differing or one of them a link").
        return Ok(None);
    }
    // The winning claimant's package in canonical order, `face` first.
    let winner = ModelPackage::first_claimed(|package| {
        copies.iter().any(|copy| copy.package == package)
            || links.iter().any(|link| link.package == package)
    })
    .expect("the caller resolves a path a copy holds");
    let winner_copy = copies.iter().find(|copy| copy.package == winner);
    // A copy conflicts with the winner on different bytes, and with a winner link always; a
    // link conflicts only against a winner file — two links at one path stand for the one
    // `Common/` file the path names.
    let conflict = copies.iter().any(|copy| {
        copy.package != winner && winner_copy.is_none_or(|winner| copy.bytes != winner.bytes)
    }) || (winner_copy.is_some() && links.iter().any(|link| link.package != winner));
    if conflict {
        // Every package claiming the path is named, the winner's included: each conflicts
        // with another claimant's as much as the winner does with the losers'.
        let packages = ModelPackage::ALL
            .into_iter()
            .filter(|package| {
                copies.iter().any(|copy| copy.package == *package)
                    || links.iter().any(|link| link.package == *package)
            })
            .map(ModelPackage::name)
            .collect::<Vec<_>>()
            .join(",");
        let texture = winner_copy
            .map(|copy| copy.below.clone())
            .or_else(|| {
                links
                    .iter()
                    .find(|link| link.package == winner)
                    .map(|link| link.below.clone())
            })
            .expect("the winning package has a claimant");
        return Err(TaskFailure {
            code: Code::SharedTextureConflict,
            context: vec![("texture", texture), ("packages", packages)],
        });
    }
    Ok(winner_copy.map(|_| {
        copies
            .iter()
            .position(|copy| copy.package == winner)
            .expect("the winning package's copy stands at the path")
    }))
}

/// The Common `textures` of one `Common/` texture directory (`folder`, its own files and
/// the files below its kit-token folders, which join it; `plan` "texture directory"),
/// converted from their bytes in `files` for the run's engine (FTEX on Fox, DDS on
/// pre-Fox), each by its path below `folder` without its extension (`kit1/armor_bsm`; a
/// direct texture's stem as before): one per texture, whether or not a `.common` link
/// uses it, placed in the team's Common output by `common_texture_entries`. The deep pass
/// has already dropped a texture its checks find wrong. Any failure of a texture's
/// conversion — the finding kind (`texture_codec_unsupported`) included — fails the task,
/// and with it every texture of `folder`. The players linking a Common model still commit on
/// their own. With `environment_map` (`TaskKind::CommonTextures`) the template
/// environment map is emitted as `env.dds` beside them, as it is. The textures emitted
/// have their kit variant sets completed against `kits` (`complete_kit_variants`).
pub(super) fn common_textures(
    folder: &ScopePath,
    textures: &[FileDescriptor],
    kits: &[u8],
    environment_map: bool,
    ctx: &CompileContext,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<Vec<(String, Vec<u8>)>, TaskFailure> {
    let mut converted = Vec::with_capacity(textures.len());
    for file in textures {
        let name = relative(&file.path, folder);
        let bytes = common_texture(file, &name, ctx, files)?;
        converted.push((file_stem(&name).to_owned(), bytes));
    }
    if environment_map {
        converted.push((
            ENVIRONMENT_MAP_STEM.to_owned(),
            ctx.templates.environment_map().to_vec(),
        ));
    }
    complete_kit_variants(&mut converted, kits, ctx.version.engine(), findings);
    Ok(converted)
}

/// The entries of `converted`, the textures of the `Common/` directory `folder` each by its
/// path below it (`common_textures`), in team `team_id`'s Common output on a target of
/// `engine` (`paths::common_texture`), below the directory's own path there
/// (`paths::common_subpath`).
pub(super) fn common_texture_entries(
    folder: &ScopePath,
    team_id: u16,
    engine: Engine,
    converted: Vec<(String, Vec<u8>)>,
) -> Vec<Entry> {
    let subpath = paths::common_subpath(folder);
    converted
        .into_iter()
        .map(|(stem, bytes)| {
            let path = paths::common_texture(engine, team_id, &format!("{subpath}{stem}"));
            (path, bytes)
        })
        .collect()
}

/// One texture of a team's Common output, `file`, converted from its bytes in `files` as any
/// texture is (`convert`), in the format its extension names: a `Common/` texture, or the
/// referees' marker. `name` is what the conversion's findings and failures name it by: a
/// `Common/` texture's path below the task's folder (`kit2/armor_bsm.dds`), the marker's
/// own file name.
pub(super) fn common_texture(
    file: &FileDescriptor,
    name: &str,
    ctx: &CompileContext,
    files: &mut TaskFiles,
) -> Result<Vec<u8>, TextureError> {
    let format = texture_format(file.path.name()).expect(
        "planning lists a Common texture, the marker included, by an extension `dds_convert` \
         accepts",
    );
    convert(ctx, format, name, &take(files, file), true)
}

/// The texture file `name`, in `format`, converted for the run's version through its
/// converter: an FTEX on every Fox target, a DDS on pre-Fox, in the codec the version reads,
/// with the mip chain a raster source lacks generated, in the role the file's stem gives it.
/// `keep_uncompressed` is `Target`'s: whether an uncompressed source is written uncompressed
/// (`false` only for a kit's main texture on PES 15-17, which soft-crash on one).
/// On pre-Fox a WESYS-wrapped DDS whose blocks the conversion keeps as they are is returned as
/// it is, still wrapped, which the game reads as it reads a plain DDS. A plain cube-map DDS is
/// never decoded: it goes out as it is on pre-Fox and as the FTEX cube map
/// `ftex::dds_to_ftex` writes on Fox. Its signature and size are the deep pass's checks, done
/// before planning; a texture that reaches this point either passed them or is kept by
/// `pass_through`, so it is converted as it is.
pub(super) fn convert(
    ctx: &CompileContext,
    format: SourceFormat,
    name: &str,
    bytes: &[u8],
    keep_uncompressed: bool,
) -> Result<Vec<u8>, TextureError> {
    // The converter decodes into one 2D image and refuses a cube map, which the game takes on
    // both engines: on pre-Fox the DDS as it is, on Fox the FTEX cube map the container
    // conversion writes from the DDS's own blocks. Nothing is decoded, so no decode is charged:
    // the output is about the source's size, and the writer charges a task's entries. The
    // texture type is the normal-map one, the type every Fox texture the converter writes
    // carries, here with the cube bit. A WESYS-wrapped DDS's header is compressed, so a
    // wrapped cube map is not recognized and fails in the converter.
    if format == SourceFormat::Dds && ftex::dds::is_cube_map(bytes) {
        return match ctx.version.engine() {
            Engine::PreFox => Ok(bytes.to_vec()),
            Engine::Fox => ftex::dds_to_ftex(bytes, ftex::ColorSpace::Normal)
                .map_err(|error| conversion_failure(name, ConvertError::Ftex(error))),
        };
    }
    let target = Target {
        version: ctx.version,
        role: texture_role(file_stem(name)),
        keep_uncompressed,
    };
    // A decode may happen inside the converter, so one is charged before the call. Only the
    // converter knows whether it will decode (blocks the target keeps are rewrapped without
    // one), and a cache hit decodes nothing: both are charged all the same, an over-charge
    // that holds other tasks back for this call alone. It is released only after the
    // pass-through test, not right after the call: that test unwraps a wrapped source again,
    // the copy the charge's source-size share covers.
    let charge = decode_charge(&ctx.budget, bytes, format)
        .map_err(|error| conversion_failure(name, error))?;
    let converted = ctx
        .converter
        .convert(source_hash(bytes), bytes, format, target, ctx.cache)
        .map_err(|error| conversion_failure(name, error))?;
    let wrapped_dds = format == SourceFormat::Dds && wezlib::is_wrapped(bytes);
    let pass_through = match ctx.version.engine() {
        Engine::PreFox => wrapped_dds && keeps_blocks(bytes, &converted),
        Engine::Fox => false,
    };
    drop(charge);
    if pass_through {
        return Ok(bytes.to_vec());
    }
    // The converter hands out the cache's own buffer, which it may hand out again for the same
    // source; the CPK entry owns its bytes, so the one copy of the texture is here.
    Ok(converted.to_vec())
}

/// Whether `converted`, the DDS the converter made of the WESYS-wrapped DDS `wrapped`, holds
/// the source's blocks as they are: the same mip data after the header, only the header
/// rebuilt. The data alone decides it: the blocks encode the codec, the size and the chain, so
/// a converter that re-encoded, resized or re-chained the texture wrote other bytes. Read from
/// the converter's output rather than decided again here, so the rule of which codecs a
/// version keeps stays the converter's alone.
fn keeps_blocks(wrapped: &[u8], converted: &[u8]) -> bool {
    // The converter has just read both: it unwrapped `wrapped` and read its header, or a
    // cache hit stands for the same bytes read before, and wrote `converted` itself.
    let source = wezlib::decompress(wrapped).expect("the converter has unwrapped this DDS");
    let from = read_layout(&source).expect("the converter has read this DDS's header");
    let to = read_layout(converted).expect("the converter writes a DDS it can read");
    source.get(from.data_offset..) == converted.get(to.data_offset..)
}

/// The charge to `budget` of decoding `bytes`, a texture in `format`: the RGBA size of every
/// level the converted texture carries (`dds_convert::probe`, a raster source's generated
/// chain included), plus the source's own size, for the copy of its blocks or the unwrapped
/// DDS the decode makes. Read from the header alone, so it is taken before the decode; the
/// caller holds the permit as long as the decoded image, bound to a name (`_decode_charge`
/// lives to the end of its scope; `let _ =` would release it at once). A header the probe
/// cannot read is the error the decode would have met.
pub(super) fn decode_charge(
    budget: &Arc<MemoryBudget>,
    bytes: &[u8],
    format: SourceFormat,
) -> Result<Permit, ConvertError> {
    let size = probe(bytes, format)?;
    let width = u64::from(size.width);
    let height = u64::from(size.height);
    let pixels: u64 = (0..size.mipmaps)
        .map(|level| (width >> level).max(1) * (height >> level).max(1))
        .sum();
    let bytes_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let charge = pixels.saturating_mul(4).saturating_add(bytes_len);
    // A charge past `usize` (a 32-bit host only) is over any memory cap, and so is the
    // saturated value.
    Ok(budget.charge(usize::try_from(charge).unwrap_or(usize::MAX)))
}

/// The portrait file `name`, in `format`, holding `bytes`, as the DDS every engine reads: a
/// DDS source under the legacy 128-byte header as it is, unwrapped when it is WESYS-wrapped;
/// one under a DX10 extension header with the header `ftex::dds::header_bytes` writes for its
/// format and its pixel data unchanged, noted in `findings` as `portrait_header_rewritten`, or,
/// for an uncompressed layout that has no `PixelFormat` or whose rows are padded, encoded like
/// a raster source; on PES 15-18, which read BC1-BC3 only, a BC4, BC5 or BC7 portrait under
/// either header encoded like a raster source too; any other accepted format encoded to BC3
/// at its own size with the full mip chain, an FTEX's own levels dropped. Its signature and
/// size are the deep pass's checks; a portrait that reaches this point passed them or is
/// kept by `pass_through`, so it is packed whatever its size. The decode is charged to
/// `budget` while it lives.
pub(super) fn portrait(
    budget: &Arc<MemoryBudget>,
    version: PesVersion,
    format: SourceFormat,
    name: &str,
    bytes: Vec<u8>,
    findings: &mut Vec<Finding>,
) -> Result<Vec<u8>, TextureError> {
    // Decoded even when the DDS goes out as it is: the deep pass reads only the header, and
    // this decode is what fails the task of a portrait whose header or data is broken.
    let _decode_charge =
        decode_charge(budget, &bytes, format).map_err(|error| conversion_failure(name, error))?;
    let mut decoded = decode(&bytes, format).map_err(|error| conversion_failure(name, error))?;
    match format {
        // The game's own DDS reader crashed PES 19 on a BC3 portrait under a DX10 header with
        // the sRGB id, and took the same blocks under the legacy header: the blocks are fine, so
        // a DX10 header is rebuilt and the data kept (a re-encode would be lossy). An
        // uncompressed DX10 layout has no header `header_bytes` can write, so it takes the
        // raster route below, and so does one whose rows are padded past the tight row, since
        // `header_bytes` declares tightly packed rows the kept data would not match.
        SourceFormat::Dds => {
            let plain = wezlib::decompress_if_wrapped(&bytes)
                .map_err(|error| conversion_failure(name, ConvertError::Wesys(error)))?;
            let layout = read_layout(&plain)
                .map_err(|error| conversion_failure(name, ConvertError::Ftex(error)))?;
            // PES 15-18 read BC1-BC3 only (`spec/libs/dds_convert.md` "Passthrough"), so a BC4,
            // BC5 or BC7 portrait keeps no blocks under either header, the legacy one
            // included, and is decoded and encoded below like a raster source.
            let reads_bc4_to_bc7 = match version {
                PesVersion::Pes15 | PesVersion::Pes16 | PesVersion::Pes17 | PesVersion::Pes18 => {
                    false
                }
                PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => true,
            };
            let encode = !reads_bc4_to_bc7
                && matches!(
                    layout.pixel,
                    DdsPixel::Format(
                        ftex::PixelFormat::Bc4 | ftex::PixelFormat::Bc5 | ftex::PixelFormat::Bc7
                    )
                );
            if layout.data_offset == 128 && !encode {
                // Unwrapped: nothing a PES 18-21 target reads is WESYS-wrapped, and a PES
                // 15-17 run wraps every DDS entry afresh when it compresses them.
                return Ok(match plain {
                    Cow::Owned(unwrapped) => unwrapped,
                    Cow::Borrowed(_) => bytes,
                });
            }
            match layout.pixel {
                DdsPixel::Format(pixel_format) if !encode && layout.row_pitch.is_none() => {
                    // `read_layout` maps the DXGI id to a format and keeps neither; it is the
                    // DX10 header's first field, right after the 128-byte legacy header.
                    let dxgi = u32::from_le_bytes([plain[128], plain[129], plain[130], plain[131]]);
                    let mut rewritten = ftex::dds::header_bytes(
                        pixel_format,
                        layout.width,
                        layout.height,
                        layout.mipmaps,
                    );
                    rewritten.extend_from_slice(&plain[layout.data_offset..]);
                    findings.push((
                        Code::PortraitHeaderRewritten,
                        vec![("file", name.to_owned()), ("dxgi", dxgi.to_string())],
                    ));
                    return Ok(rewritten);
                }
                DdsPixel::Format(_) | DdsPixel::Uncompressed { .. } => {
                    decoded.mips.truncate(1);
                    decoded.authored_mips = false;
                    decoded.blocks = None;
                }
            }
        }
        // Every portrait that is not a DDS gets the full chain. An FTEX's authored levels, which
        // the encoder would otherwise keep, are the one thing it would add over a raster source,
        // and nothing asks for them. Its blocks go too: the encoder
        // emits a source's blocks as they are when they are already BC3, whatever their level
        // count, so a BC3 FTEX would keep its own chain.
        SourceFormat::Ftex => {
            decoded.mips.truncate(1);
            decoded.authored_mips = false;
            decoded.blocks = None;
        }
        SourceFormat::Png
        | SourceFormat::Jpeg
        | SourceFormat::Bmp
        | SourceFormat::WebP
        | SourceFormat::Tga
        | SourceFormat::Tiff
        | SourceFormat::Jxl => {}
    }
    encode_dds(&decoded, BlockCodec::Bc3).map_err(|error| conversion_failure(name, error))
}

/// The role of a texture by its `stem`: a normal map when the stem ends in `_nrm` in any
/// case, the suffix of the game's own normal maps (`skin_nrm`, `oral_nrm`), read without a
/// kit token, which sits after the role suffix (`pants_nrm_kit1`). Color otherwise. The
/// deep pass decides a texture's role by the same function, so its checks and the
/// conversion cannot disagree.
pub(crate) fn texture_role(stem: &str) -> TextureRole {
    let stem = without_kit_token(stem);
    let bytes = stem.as_bytes();
    let tail = bytes.len().checked_sub(4).map(|at| &bytes[at..]);
    if tail.is_some_and(|tail| tail.eq_ignore_ascii_case(b"_nrm")) {
        TextureRole::Normal
    } else {
        TextureRole::Color
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use dds_convert::{CachePolicy, Converter, decode};
    use pes_version::PesVersion;

    use super::*;

    fn context(version: PesVersion) -> CompileContext {
        CompileContext {
            version,
            converter: Converter::new(),
            cache: CachePolicy::Bypass,
            templates: crate::templates::Templates::embedded(),
            installed: crate::bins::installed::InstalledPaths::Unknown,
            target: crate::processing::EntryTarget::GamePaths {
                engine: version.engine(),
            },
            budget: MemoryBudget::new(usize::MAX),
            compress_dds: false,
        }
    }

    /// A budget no test fills, for the tests that check bytes, not charges.
    fn unlimited() -> Arc<MemoryBudget> {
        MemoryBudget::new(usize::MAX)
    }

    fn tracer_kit() -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Kits/g1/kit.dds"),
        )
        .unwrap()
    }

    #[test]
    fn a_texture_is_converted_for_the_run_s_version_or_fails_naming_the_file() {
        let dds = tracer_kit();
        let decoded = decode(&dds, SourceFormat::Dds).unwrap();
        for version in [PesVersion::Pes18, PesVersion::Pes21] {
            let converted =
                convert(&context(version), SourceFormat::Dds, "kit.dds", &dds, true).unwrap();
            let target = Target {
                version,
                role: TextureRole::Color,
                keep_uncompressed: true,
            };
            assert_eq!(
                converted,
                dds_convert::convert(&decoded, target).unwrap(),
                "{version}"
            );
        }
        let error = convert(
            &context(PesVersion::Pes21),
            SourceFormat::Dds,
            "kit.dds",
            b"not a DDS",
            true,
        )
        .unwrap_err();
        let TextureError::Other(error) = error else {
            panic!("a DDS without its header is no finding");
        };
        assert_eq!(format!("{error}"), "cannot convert kit.dds");
    }

    #[test]
    fn a_wrapped_dds_whose_blocks_the_version_keeps_is_emitted_as_it_is_on_pre_fox_only() {
        // The tracer's kit, BC1, which PES 17 reads as it is: the wrapped source verbatim.
        let dds = tracer_kit();
        let wrapped = wezlib::compress(&dds);
        let on_pes_17 = convert(
            &context(PesVersion::Pes17),
            SourceFormat::Dds,
            "kit.dds",
            &wrapped,
            true,
        )
        .unwrap();
        assert!(on_pes_17 == wrapped, "the wrapped source as it is");
        // On Fox it is unwrapped and converted, to the FTEX its plain bytes give.
        let decoded = decode(&dds, SourceFormat::Dds).unwrap();
        let on_pes_21 = convert(
            &context(PesVersion::Pes21),
            SourceFormat::Dds,
            "kit.dds",
            &wrapped,
            true,
        )
        .unwrap();
        let fox = Target {
            version: PesVersion::Pes21,
            role: TextureRole::Color,
            keep_uncompressed: true,
        };
        assert!(on_pes_21 == dds_convert::convert(&decoded, fox).unwrap());
        // A normal map keeps no BC1 blocks: the wrapped source is converted, to the BC3 its
        // plain bytes give, and left to the entries' wrapping.
        let normal = convert(
            &context(PesVersion::Pes17),
            SourceFormat::Dds,
            "kit_nrm.dds",
            &wrapped,
            true,
        )
        .unwrap();
        let pre_fox_normal = Target {
            version: PesVersion::Pes17,
            role: TextureRole::Normal,
            keep_uncompressed: true,
        };
        assert!(normal == dds_convert::convert(&decoded, pre_fox_normal).unwrap());
    }

    /// The bytes of `tests/fixtures/textures/<name>`.
    fn texture_fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/textures")
                .join(name),
        )
        .unwrap()
    }

    /// The (code, file) of a texture finding; panics on any other outcome.
    fn finding(result: Result<Vec<u8>, TextureError>) -> (Code, String) {
        match result {
            Ok(_) => panic!("converted"),
            Err(TextureError::Finding(code, file)) => (code, file),
            Err(TextureError::Other(error)) => panic!("{error:#}"),
        }
    }

    #[test]
    fn a_texture_of_an_odd_size_converts_as_it_is() {
        // The deep pass reports its size before planning; one `pass_through` keeps is
        // converted: 300x300 with its generated chain down to 1x1.
        let converted = convert(
            &context(PesVersion::Pes21),
            SourceFormat::Png,
            "odd.png",
            &texture_fixture("odd.png"),
            true,
        )
        .unwrap();
        let info = ftex::info(&converted).unwrap();
        assert_eq!((info.width, info.height, info.mipmaps), (300, 300, 9));
    }

    #[test]
    fn a_codec_dds_convert_does_not_decode_is_codec_unsupported() {
        assert_eq!(
            finding(convert(
                &context(PesVersion::Pes21),
                SourceFormat::Dds,
                "skin.dds",
                &texture_fixture("bc6h.dds"),
                true,
            )),
            (Code::TextureCodecUnsupported, "skin.dds".to_owned())
        );
    }

    #[test]
    fn a_single_level_texture_with_an_odd_side_converts_on_fox() {
        // 12x12 BC3 with one mip level: the power-of-two rule is for mipmapped textures, so
        // this one goes through at its size with its one level.
        let converted = convert(
            &context(PesVersion::Pes21),
            SourceFormat::Dds,
            "skin.dds",
            &texture_fixture("single_level.dds"),
            true,
        )
        .unwrap();
        let info = ftex::info(&converted).unwrap();
        assert_eq!((info.width, info.height, info.mipmaps), (12, 12, 1));
    }

    #[test]
    fn a_dds_portrait_passes_through_and_any_other_format_is_a_bc3_dds() {
        let tracer = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/portrait.dds"),
        )
        .unwrap();
        let mut findings = Vec::new();
        assert_eq!(
            portrait(
                &unlimited(),
                PesVersion::Pes21,
                SourceFormat::Dds,
                "portrait.dds",
                tracer.clone(),
                &mut findings
            )
            .unwrap(),
            tracer
        );
        assert!(findings.is_empty(), "a legacy-header DDS: {findings:?}");
        for (name, format) in [
            ("portrait.png", SourceFormat::Png),
            ("portrait.webp", SourceFormat::WebP),
        ] {
            let dds = portrait(
                &unlimited(),
                PesVersion::Pes21,
                format,
                name,
                texture_fixture(name),
                &mut findings,
            )
            .unwrap();
            let decoded = decode(&dds, SourceFormat::Dds).unwrap();
            assert_eq!(
                decoded.blocks.as_ref().map(|blocks| blocks.codec),
                Some(BlockCodec::Bc3),
                "{name}"
            );
            assert_eq!(
                (decoded.width, decoded.height, decoded.mips.len()),
                (128, 128, 8),
                "{name}"
            );
        }
        // A portrait's signature and size are the deep pass's checks: one `pass_through`
        // keeps is packed whatever its size, a 300x300 PNG as a 300x300 BC3 DDS.
        let odd = texture_fixture("odd.png");
        let dds = portrait(
            &unlimited(),
            PesVersion::Pes21,
            SourceFormat::Png,
            "05.png",
            odd.clone(),
            &mut findings,
        )
        .unwrap();
        let decoded = decode(&dds, SourceFormat::Dds).unwrap();
        assert_eq!((decoded.width, decoded.height), (300, 300));
        assert!(findings.is_empty(), "raster sources: {findings:?}");
        // Every source is still decoded, a DDS included: a BC6H DDS is the codec finding, and
        // PNG bytes under a `.dds` name fail the task.
        assert_eq!(
            finding(portrait(
                &unlimited(),
                PesVersion::Pes21,
                SourceFormat::Dds,
                "portrait.dds",
                texture_fixture("bc6h.dds"),
                &mut findings
            )),
            (Code::TextureCodecUnsupported, "portrait.dds".to_owned())
        );
        let Err(TextureError::Other(error)) = portrait(
            &unlimited(),
            PesVersion::Pes21,
            SourceFormat::Dds,
            "portrait.dds",
            odd,
            &mut findings,
        ) else {
            panic!("PNG bytes under a `.dds` name are not a DDS");
        };
        assert_eq!(format!("{error}"), "cannot convert portrait.dds");
    }

    #[test]
    fn a_wesys_wrapped_dds_portrait_goes_out_unwrapped() {
        // The tracer's portrait: BC3 under the legacy header.
        let tracer = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/portrait.dds"),
        )
        .unwrap();
        let mut findings = Vec::new();
        let unwrapped = portrait(
            &unlimited(),
            PesVersion::Pes21,
            SourceFormat::Dds,
            "portrait.dds",
            wezlib::compress(&tracer),
            &mut findings,
        )
        .unwrap();
        assert!(unwrapped.starts_with(b"DDS "));
        assert!(unwrapped == tracer, "the unwrapped source");
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_uncompressed_dx10_portrait_is_encoded_like_a_raster_one() {
        // A 4x4 R8G8B8A8 DDS under a DX10 header (DXGI 28), one level: no legacy header can
        // carry it, so it is a BC3 DDS with the full chain, and no header was rewritten.
        let mut dds = ftex::dds::header_bytes(ftex::PixelFormat::Bc7, 4, 4, 1);
        dds[128..132].copy_from_slice(&28u32.to_le_bytes());
        dds.extend_from_slice(&[0x80; 4 * 4 * 4]);
        let mut findings = Vec::new();
        let encoded = portrait(
            &unlimited(),
            PesVersion::Pes21,
            SourceFormat::Dds,
            "portrait.dds",
            dds,
            &mut findings,
        )
        .unwrap();
        let decoded = decode(&encoded, SourceFormat::Dds).unwrap();
        assert_eq!(
            decoded.blocks.as_ref().map(|blocks| blocks.codec),
            Some(BlockCodec::Bc3)
        );
        assert_eq!(
            (decoded.width, decoded.height, decoded.mips.len()),
            (4, 4, 3)
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_dx10_bgra8_portrait_with_padded_rows_is_encoded_like_a_raster_one() {
        // A 4x4 B8G8R8A8 DDS under a DX10 header (DXGI 87) declaring a 20-byte row pitch over
        // 16-byte rows: the legacy header declares tight rows, so it is not re-headered.
        let mut dds = ftex::dds::header_bytes(ftex::PixelFormat::Bc7, 4, 4, 1);
        dds[128..132].copy_from_slice(&87u32.to_le_bytes());
        // The header's flags (offset 8) gain DDSD_PITCH (0x8); the pitch is at offset 20.
        let flags = u32::from_le_bytes(dds[8..12].try_into().unwrap()) | 0x8;
        dds[8..12].copy_from_slice(&flags.to_le_bytes());
        dds[20..24].copy_from_slice(&20u32.to_le_bytes());
        for _ in 0..4 {
            dds.extend_from_slice(&[0x80; 16]);
            dds.extend_from_slice(&[0; 4]);
        }
        let layout = read_layout(&dds).unwrap();
        assert_eq!(
            (layout.pixel, layout.row_pitch),
            (DdsPixel::Format(ftex::PixelFormat::Argb8), Some(20))
        );
        let mut findings = Vec::new();
        let encoded = portrait(
            &unlimited(),
            PesVersion::Pes21,
            SourceFormat::Dds,
            "portrait.dds",
            dds,
            &mut findings,
        )
        .unwrap();
        let decoded = decode(&encoded, SourceFormat::Dds).unwrap();
        assert_eq!(
            decoded.blocks.as_ref().map(|blocks| blocks.codec),
            Some(BlockCodec::Bc3)
        );
        assert_eq!(
            (decoded.width, decoded.height, decoded.mips.len()),
            (4, 4, 3)
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_bc7_dx10_portrait_is_encoded_on_pes_18_and_re_headered_from_pes_19() {
        // A 16x16 two-level BC7 DDS under a DX10 header, arbitrary blocks: on PES 18, which
        // reads no BC7, it is decoded and encoded like a raster source: BC3 under the
        // legacy header, the full chain, and no header finding.
        let mut source = ftex::dds::header_bytes(ftex::PixelFormat::Bc7, 16, 16, 2);
        assert_eq!(source.len(), 148);
        let blocks: Vec<u8> = (0..=u8::MAX).cycle().take(16 * 16 + 4 * 16).collect();
        source.extend_from_slice(&blocks);
        let mut findings = Vec::new();
        let encoded = portrait(
            &unlimited(),
            PesVersion::Pes18,
            SourceFormat::Dds,
            "portrait.dds",
            source.clone(),
            &mut findings,
        )
        .unwrap();
        assert_eq!(&encoded[84..88], b"DXT5");
        let decoded = decode(&encoded, SourceFormat::Dds).unwrap();
        assert_eq!(
            decoded.blocks.as_ref().map(|blocks| blocks.codec),
            Some(BlockCodec::Bc3)
        );
        assert_eq!(
            (decoded.width, decoded.height, decoded.mips.len()),
            (16, 16, 5)
        );
        assert!(findings.is_empty(), "{findings:?}");
        // On PES 19 to 21 the same file goes out re-headered, its blocks kept, the finding
        // naming it with the DXGI id.
        for version in [PesVersion::Pes19, PesVersion::Pes20, PesVersion::Pes21] {
            let rewritten = portrait(
                &unlimited(),
                version,
                SourceFormat::Dds,
                "portrait.dds",
                source.clone(),
                &mut findings,
            )
            .unwrap();
            assert_eq!(rewritten, source, "{version}");
        }
        assert_eq!(
            findings,
            vec![
                (
                    Code::PortraitHeaderRewritten,
                    vec![
                        ("file", "portrait.dds".to_owned()),
                        ("dxgi", "98".to_owned())
                    ]
                );
                3
            ]
        );
    }

    #[test]
    fn a_bc4_or_bc5_portrait_under_the_legacy_header_is_encoded_on_pes_17_and_kept_on_pes_21() {
        // An 8x8 two-level BC4 or BC5 DDS under the legacy `ATI1`/`ATI2` header, which
        // `header_bytes` does not write: the DX10 header is truncated and the FourCC
        // respelled. BC4's blocks are half a BC5's.
        for (format, fourcc, block_bytes) in [
            (ftex::PixelFormat::Bc4, b"ATI1", 4 * 8 + 8),
            (ftex::PixelFormat::Bc5, b"ATI2", 4 * 16 + 16),
        ] {
            let mut source = ftex::dds::header_bytes(format, 8, 8, 2);
            source.truncate(128);
            source[84..88].copy_from_slice(fourcc);
            let blocks: Vec<u8> = (0..=u8::MAX).cycle().take(block_bytes).collect();
            source.extend_from_slice(&blocks);
            let layout = read_layout(&source).unwrap();
            assert_eq!(
                (layout.pixel, layout.data_offset),
                (DdsPixel::Format(format), 128)
            );
            // On PES 17: BC3 under the legacy header, the full chain, no finding.
            let mut findings = Vec::new();
            let encoded = portrait(
                &unlimited(),
                PesVersion::Pes17,
                SourceFormat::Dds,
                "portrait.dds",
                source.clone(),
                &mut findings,
            )
            .unwrap();
            assert_eq!(&encoded[84..88], b"DXT5", "{format:?}");
            let decoded = decode(&encoded, SourceFormat::Dds).unwrap();
            assert_eq!(
                decoded.blocks.as_ref().map(|blocks| blocks.codec),
                Some(BlockCodec::Bc3),
                "{format:?}"
            );
            assert_eq!(
                (decoded.width, decoded.height, decoded.mips.len()),
                (8, 8, 4),
                "{format:?}"
            );
            assert!(findings.is_empty(), "{format:?}: {findings:?}");
            // On PES 21, which reads BC4 and BC5, it passes through as it is.
            let kept = portrait(
                &unlimited(),
                PesVersion::Pes21,
                SourceFormat::Dds,
                "portrait.dds",
                source.clone(),
                &mut findings,
            )
            .unwrap();
            assert_eq!(kept, source, "{format:?}");
            assert!(findings.is_empty(), "{format:?}: {findings:?}");
        }
    }

    #[test]
    fn a_decode_is_charged_every_level_s_rgba_size_and_its_source_s() {
        let budget = MemoryBudget::new(usize::MAX);
        // A 300x300 PNG holds one level; its conversion generates the chain down to 1x1:
        // 300, 150, 75, 37, 18, 9, 4, 2 and 1 pixels across.
        let png = texture_fixture("odd.png");
        let levels =
            300 * 300 + 150 * 150 + 75 * 75 + 37 * 37 + 18 * 18 + 9 * 9 + 4 * 4 + 2 * 2 + 1;
        let charge = decode_charge(&budget, &png, SourceFormat::Png).unwrap();
        assert_eq!(charge.size(), 4 * levels + png.len());
        drop(charge);
        // The tracer's kit: a 16x16 BC1 DDS with its five levels.
        let kit = tracer_kit();
        let charge = decode_charge(&budget, &kit, SourceFormat::Dds).unwrap();
        assert_eq!(
            charge.size(),
            4 * (16 * 16 + 8 * 8 + 4 * 4 + 2 * 2 + 1) + kit.len()
        );
        drop(charge);
        // The tracer's portrait: a 64x128 BC3 DDS with eight levels, the narrow side held at
        // one pixel once it gets there.
        let portrait = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/portrait.dds"),
        )
        .unwrap();
        let charge = decode_charge(&budget, &portrait, SourceFormat::Dds).unwrap();
        // The last two levels are 1x2 and 1x1.
        let levels = 64 * 128 + 32 * 64 + 16 * 32 + 8 * 16 + 4 * 8 + 2 * 4 + 2 + 1;
        assert_eq!(charge.size(), 4 * levels + portrait.len());
        drop(charge);
        // A header the probe cannot read is the decode's error.
        assert!(decode_charge(&budget, b"not a DDS", SourceFormat::Dds).is_err());
    }

    #[test]
    fn a_stem_ending_in_nrm_in_any_case_is_a_normal_map() {
        // A per-kit normal map carries its kit token after the role suffix.
        for normal in [
            "skin_nrm",
            "SKIN_NRM",
            "oral_Nrm",
            "_nrm",
            "pants_nrm_kit1",
            "pants_nrm_kit2",
        ] {
            assert_eq!(texture_role(normal), TextureRole::Normal, "{normal}");
        }
        for color in [
            "skin",
            "nrm",
            "skin_nrm2",
            "skin-nrm",
            "kit",
            "",
            "pants_kit1",
        ] {
            assert_eq!(texture_role(color), TextureRole::Color, "{color}");
        }
    }

    #[test]
    fn a_set_spelled_by_kit_folders_is_completed_as_a_stem_set_is() {
        // `kit1/armor_bsm` and `kit2/armor_bsm` are the set `kitN/armor_bsm`, of different
        // bytes.
        let set = || {
            vec![
                ("kit1/armor_bsm".to_owned(), b"one".to_vec()),
                ("kit2/armor_bsm".to_owned(), b"two".to_vec()),
            ]
        };
        // On PES 18-21 the fallback, the lowest variant, is written once under the
        // reference, `kit_variant_missing` naming the kit numbers the set lacks.
        let mut textures = set();
        let mut findings = Vec::new();
        complete_kit_variants(&mut textures, &[1, 2, 3], Engine::Fox, &mut findings);
        assert_eq!(
            textures,
            [
                ("kit1/armor_bsm".to_owned(), b"one".to_vec()),
                ("kit2/armor_bsm".to_owned(), b"two".to_vec()),
                ("kitN/armor_bsm".to_owned(), b"one".to_vec()),
            ]
        );
        assert_eq!(
            findings,
            [(
                Code::KitVariantMissing,
                vec![
                    ("texture", "kitN/armor_bsm".to_owned()),
                    ("kit", "3".to_owned()),
                    ("copied", "kit1/armor_bsm".to_owned()),
                ]
            )]
        );
        // On PES 15-17 the copy goes into the missing number's folder.
        let mut textures = set();
        let mut findings = Vec::new();
        complete_kit_variants(&mut textures, &[1, 2, 3], Engine::PreFox, &mut findings);
        assert_eq!(
            textures,
            [
                ("kit1/armor_bsm".to_owned(), b"one".to_vec()),
                ("kit2/armor_bsm".to_owned(), b"two".to_vec()),
                ("kit3/armor_bsm".to_owned(), b"one".to_vec()),
            ]
        );
        // The `kitF` member serves and is never emitted: `kitN/a` holds its bytes, no
        // finding.
        let mut textures = vec![
            ("kit1/a".to_owned(), b"one".to_vec()),
            ("kitF/a".to_owned(), b"fallback".to_vec()),
        ];
        let mut findings = Vec::new();
        complete_kit_variants(&mut textures, &[1, 2], Engine::Fox, &mut findings);
        assert_eq!(
            textures,
            [
                ("kit1/a".to_owned(), b"one".to_vec()),
                ("kitN/a".to_owned(), b"fallback".to_vec()),
            ]
        );
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn a_set_s_members_are_completed_wherever_they_sit_among_the_textures() {
        // `kit2/a` does not sit next to `kit1/a`: the set is one, its members gathered by
        // folded reference, not the run of adjacent entries it happens to be.
        let mut textures = vec![
            ("kit1/a".to_owned(), b"one".to_vec()),
            ("kit1/b".to_owned(), b"other".to_vec()),
            ("kit2/a".to_owned(), b"two".to_vec()),
        ];
        let mut findings = Vec::new();
        complete_kit_variants(&mut textures, &[1, 2, 3], Engine::Fox, &mut findings);
        assert_eq!(
            textures,
            [
                ("kit1/a".to_owned(), b"one".to_vec()),
                ("kit1/b".to_owned(), b"other".to_vec()),
                ("kit2/a".to_owned(), b"two".to_vec()),
                ("kitN/a".to_owned(), b"one".to_vec()),
                ("kitN/b".to_owned(), b"other".to_vec()),
            ]
        );
        assert_eq!(
            findings,
            [
                (
                    Code::KitVariantMissing,
                    vec![
                        ("texture", "kitN/a".to_owned()),
                        ("kit", "3".to_owned()),
                        ("copied", "kit1/a".to_owned()),
                    ]
                ),
                (
                    Code::KitVariantMissing,
                    vec![
                        ("texture", "kitN/b".to_owned()),
                        ("kit", "2".to_owned()),
                        ("copied", "kit1/b".to_owned()),
                    ]
                ),
                (
                    Code::KitVariantMissing,
                    vec![
                        ("texture", "kitN/b".to_owned()),
                        ("kit", "3".to_owned()),
                        ("copied", "kit1/b".to_owned()),
                    ]
                ),
            ]
        );
    }

    #[test]
    fn a_kitf_member_is_dropped_without_touching_the_entries_after_it() {
        // Two `kitF` members at the head of `textures`: each goes at its own position,
        // whatever the other's.
        let mut textures = vec![
            ("a_kitF".to_owned(), b"one".to_vec()),
            ("z_kitF".to_owned(), b"two".to_vec()),
            ("plain".to_owned(), b"kept".to_vec()),
        ];
        let mut findings = Vec::new();
        complete_kit_variants(&mut textures, &[1, 2], Engine::Fox, &mut findings);
        assert_eq!(
            textures,
            [
                ("plain".to_owned(), b"kept".to_vec()),
                ("a_kitN".to_owned(), b"one".to_vec()),
                ("z_kitN".to_owned(), b"two".to_vec()),
            ]
        );
        assert!(findings.is_empty(), "{findings:?}");
    }
}
