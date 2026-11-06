//! One kit (`spec/team_compiler/pipeline.md` "4. Per-export non-model steps", Kits): its textures as
//! FTEX (Fox) or DDS (pre-Fox) under the kit's game names, its config encoded with those
//! names, and its menu colors and icon as its `UniColor.bin` entry ("Bins accumulation", "Kit
//! colors fallback").

use std::sync::Arc;

use aesthetics_export::{ExportCoverage, KitFolder, KitLayout, read_colors_txt};
use anyhow::Context;
use dds_convert::{SourceFormat, Target, TextureRole, decode};
use kit_config::{KitConfig, KitSlot, TexturePresence, apply_fpc, matches_fpc, texture_names};
use pes_version::Engine;
use pipeline::MemoryBudget;

use super::texture::TextureError;
use super::{CompileContext, Entry, Finding, TaskFailure, TaskFiles, kit_layout, take, texture};
use crate::bins::{KIT_COLORS, KitColorEntry, Rgb, uni_color_kit_number};
use crate::messages::Code;
use crate::paths::{self, REFEREE_MARKER_COLLAR};
use crate::plan::roles::{KIT_TEXTURE_STEMS, texture_format};
use crate::plan::{EffectiveTeamKitFpc, TeamKitEdits, keeps_installed};

/// The menu icon of a kit without an `icon_<N>` marker.
const DEFAULT_ICON: u8 = 3;

/// The menu colors of a kit with none to give: magenta, then black. A pair nobody would pick,
/// so the gap shows in the game's menus instead of looking chosen; it matches the placeholder
/// texture's checkerboard.
const MISSING_COLORS: [Rgb; KIT_COLORS] = [[255, 0, 255], [0, 0, 0]];

/// What a kit task commits (`kit`): its CPK entries, its config's `UniformParameter.bin`
/// entry when it has one, and its `UniColor.bin` entry when it has one.
pub(super) struct KitCommit {
    /// The files written: the held textures converted, and the config's loose file.
    pub(super) entries: Vec<Entry>,
    /// The config's `UniformParameter.bin` entry (name, bytes); `None` on PES 15-17 and
    /// for a `Midcup` kit emitting no config.
    pub(super) uniform_parameter: Option<Entry>,
    /// The kit's `UniColor.bin` entry; `None` for a `Midcup` kit with no colors to give.
    pub(super) uni_color: Option<KitColorEntry>,
}

/// What a kit task needs of its `TaskKind::Kit` variant: the slot, the folder, what the
/// export sets in the team's kit configs, the coverage, and the team id.
#[derive(Clone, Copy)]
pub(super) struct KitJob<'a> {
    /// The game's kit slot.
    pub(super) slot: KitSlot,
    /// The kit folder.
    pub(super) kit: &'a KitFolder,
    /// What its team's export sets in every kit config of the team.
    pub(super) edits: TeamKitEdits,
    /// The export's coverage.
    pub(super) coverage: ExportCoverage,
    /// The export's team id.
    pub(super) team_id: u16,
}

/// The kit `job`'s folder in `job.slot`, compiled for the run's version from its files'
/// bytes in `files`: its textures and its config as CPK entries, on Fox the config as a
/// `UniformParameter.bin` entry (name, bytes) too (`None` on PES 15-17, which have no such
/// bin), and its `UniColor.bin` entry. A kit without a `kit` texture gets the bundled
/// placeholder as its main texture, converted like any kit texture. Besides the five textures
/// its config names, a kit emits the map the run's engine reads: on PES 15-17 its `kit_mask`
/// as `{main}_mask`, or the bundled mask template as it is when it has none; on PES 18-21 its
/// `kit_srm` as `{main}_srm`, or nothing.
/// The deep pass has already dropped a kit whose `config.toml` does not parse
/// (`kit_config_invalid`) or whose textures its checks find wrong, so a config that fails to
/// parse here is an ordinary failure; a finding conversion reports
/// (`texture_codec_unsupported`) fails the kit with the finding's code: the config names its
/// textures, so none goes out alone.
///
/// When the kit's layout marker names the other engine than the run's, its main texture and
/// its mask or srm, its own or inherited (never the placeholder or the mask template), are
/// re-laid out to the run's layout before they are converted, noted once in `findings` as
/// `kit_layout_converted` naming both layouts; its other textures are converted as they are.
///
/// `edits` is what the team's export sets in every kit config of the team. When its kit-FPC
/// status is `On`, a config lacking the FPC values gets them before it is encoded, noted in
/// `findings` as `kit_config_fpc_adjusted` (the template already carries them); then the
/// team's collar, when it has one, becomes the config's collar and winter collar, the FPC
/// collar included; otherwise the config is encoded as it is. A config whose collar or winter
/// collar is then the referees' marker collar fails the kit with `kit_collar_reserved`,
/// naming the field.
///
/// The entry's colors are the two its `colors.txt` gives (the lines it refuses are the deep
/// pass's to report); else the two its main texture gives, its own or one inherited from
/// `all/`, noted in `findings` as `kit_colors_derived`; else, for a placeholder kit or a main
/// texture that gives none, magenta and black, noted as `kit_colors_missing`.
///
/// A `Midcup` kit (`coverage`) keeps what it does not resend: without `config.toml` it
/// emits no config of its own (the slot's installed config is edited in place, or the
/// template written where none is installed, `kit_configs`), without a main texture it
/// emits no placeholder and no mask template (the installed textures stay), and with no
/// usable `colors.txt` it emits no `UniColor.bin` entry (the installed one stays, the
/// extraction not run for it, `uni_color` `None`).
pub(super) fn kit(
    job: &KitJob,
    ctx: &CompileContext,
    files: &mut TaskFiles,
    findings: &mut Vec<Finding>,
) -> Result<KitCommit, TaskFailure> {
    let (slot, kit, edits, coverage, team_id) =
        (job.slot, job.kit, job.edits, job.coverage, job.team_id);
    let listed = kit
        .colors
        .as_ref()
        .and_then(|file| listed_colors(&take(files, file)));
    // A `Midcup` kit keeps the installed menu colors: only a `Full` export derives them.
    let derives_colors = match coverage {
        ExportCoverage::Full => listed.is_none(),
        ExportCoverage::Midcup => false,
    };
    let engine = ctx.version.engine();
    let has = |stem: &str| kit.textures.iter().any(|texture| texture.stem == stem);
    // A `Midcup` kit's config names a `back`, `chest`, `leg` or `name` texture the
    // installed CPKs hold for the slot, the folder resending it or not; a `Full` kit's names
    // come from its effective set alone.
    let installed = |suffix: &str| {
        coverage == ExportCoverage::Midcup
            && ctx.installed.holds(&paths::kit_texture(
                engine,
                &format!("u0{team_id:03}{slot}{suffix}", slot = slot.as_str()),
            ))
    };
    let names = texture_names(
        team_id,
        slot,
        TexturePresence {
            // The game requires a main texture, so every compiled kit has one: the kit's own
            // or the placeholder. The other four are optional and absent ones stay zero.
            kit: true,
            back: has("kit_back") || installed("_back"),
            chest: has("kit_chest") || installed("_chest"),
            leg: has("kit_leg") || installed("_leg"),
            name: has("kit_name") || installed("_name"),
        },
    );
    // The config names the first five; the games find each map by the main texture's name
    // alone, so the two maps' names (`u0714p1_mask`, `u0714p1_srm`) are not in the config.
    let mut game_names = Vec::with_capacity(KIT_TEXTURE_STEMS.len());
    for field in &names {
        let name = std::str::from_utf8(field).context("cannot read a kit texture name")?;
        game_names.push(name.trim_end_matches('\0').to_owned());
    }
    let main = game_names[0].clone();
    game_names.extend([format!("{main}_mask"), format!("{main}_srm")]);

    // Only the kit's own or inherited textures the uniform models map are re-laid: the
    // placeholder and the mask template are flat and engine-neutral. Reported once per kit.
    let drawn_for_other = kit.layout.filter(|layout| {
        layout_engine(*layout) != engine
            && kit
                .textures
                .iter()
                .any(|texture| UV_MAPPED_STEMS.contains(&texture.stem.as_str()))
    });
    if let Some(drawn_for) = drawn_for_other {
        findings.push((
            Code::KitLayoutConverted,
            vec![
                ("from", marker_name(drawn_for).to_owned()),
                ("to", marker_name(engine_layout(engine)).to_owned()),
            ],
        ));
    }
    let mut derived = None;
    let mut entries = Vec::new();
    // Planning has dropped every texture the run does not emit (`drop_unused_kit_textures`),
    // so a `kit_mask` here is a pre-Fox run's, a `kit_srm` a Fox run's, and no other stem is.
    for (stem, name) in KIT_TEXTURE_STEMS.iter().zip(&game_names) {
        let texture = kit.textures.iter().find(|texture| texture.stem == *stem);
        let (file_name, bytes) = match texture {
            Some(texture) => (texture.file.path.name(), take(files, &texture.file)),
            // A `Midcup` kit's held textures go out as they are, the installed ones
            // staying for the rest: no placeholder, and no mask template (it would
            // lay over an installed mask).
            None if coverage == ExportCoverage::Midcup => continue,
            None if *stem == "kit" => (
                "placeholder_kit.dds",
                ctx.templates.placeholder_kit().to_vec(),
            ),
            // PES 15-17 read a mask beside every kit, Fox reads none. The template goes out
            // byte for byte: a DDS the game reads, flat, so nothing to re-lay or convert.
            None if *stem == "kit_mask" => {
                match engine {
                    Engine::PreFox => entries.push((
                        paths::kit_texture(engine, name),
                        ctx.templates.kit_mask().to_vec(),
                    )),
                    Engine::Fox => {}
                }
                continue;
            }
            None => continue,
        };
        let format = texture_format(file_name)
            .expect("a kit texture is classified by an extension `dds_convert` accepts");
        // Only the kit's own main texture gives colors, never the placeholder: its
        // checkerboard is no color anybody chose. A `Midcup` kit keeps the installed
        // colors either way, so the extraction is not run for one at all.
        if derives_colors && *stem == "kit" && texture.is_some() {
            derived = derived_colors(&ctx.budget, format, file_name, &bytes);
        }
        // PES 15-17 edit a kit's main texture in memory to place the seasonal
        // logos and soft-crash on an uncompressed one, so only it is encoded
        // there; every other uncompressed source keeps its layout.
        let keep_uncompressed = match engine {
            Engine::PreFox => *stem != "kit",
            Engine::Fox => true,
        };
        let converted = match drawn_for_other {
            Some(drawn_for) if UV_MAPPED_STEMS.contains(stem) && texture.is_some() => {
                relaid_texture(ctx, format, file_name, &bytes, drawn_for, keep_uncompressed)?
            }
            // By its shape alone, whatever the marker: the marker is about the textures the
            // uniform models map.
            Some(_) | None if NUMBER_ATLAS_STEMS.contains(stem) => {
                number_atlas(ctx, format, file_name, &bytes)?
            }
            Some(_) | None => texture::convert(ctx, format, file_name, &bytes, keep_uncompressed)?,
        };
        entries.push((paths::kit_texture(engine, name), converted));
    }

    // A `Midcup` kit without `config.toml` emits no config of its own.
    let emits_config = !keeps_installed(kit, coverage);
    let config = if emits_config {
        let mut config = match &kit.config {
            Some(file) => {
                let text = String::from_utf8(take(files, file)).with_context(|| {
                    format!("the kit config {} is not UTF-8", file.path.as_str())
                })?;
                KitConfig::from_toml(&text)
                    .with_context(|| format!("cannot read the kit config {}", file.path.as_str()))?
            }
            None => KitConfig::template(),
        };
        // Only upward: with the status unknown, FPC values a config carries are kept, since no
        // export state shows the team stopped using FPC.
        match edits.fpc {
            EffectiveTeamKitFpc::On if !matches_fpc(&config) => {
                apply_fpc(&mut config);
                findings.push((Code::KitConfigFpcAdjusted, Vec::new()));
            }
            EffectiveTeamKitFpc::On | EffectiveTeamKitFpc::Unknown => {}
        }
        // After the FPC values, so the team's collar wins over the FPC collar.
        if let Some(collar) = edits.collar {
            config.shirt.collar = collar;
            config.shirt.winter_collar = collar;
        }
        // On the collars the kit ends up with: FPC sets collar 105 and the team's collar its own
        // (never 77, which the deep pass refuses), so a config naming 77 under either no longer
        // wears the marker.
        if let Some(field) = reserved_collar_field(&config) {
            return Err(TaskFailure {
                code: Code::KitCollarReserved,
                context: vec![("field", field.to_owned())],
            });
        }
        Some(config.encode_with_names(ctx.version, &names).to_vec())
    } else {
        None
    };
    let entry_name = slot.config_name(team_id);
    // PES 15-17 have no `UniformParameter.bin`: the loose file is the kit's only config there.
    let uniform_parameter = match (engine, &config) {
        (Engine::Fox, Some(config)) => Some((entry_name.clone(), config.clone())),
        (Engine::Fox, None) | (Engine::PreFox, _) => None,
    };
    if let Some(config) = config {
        entries.push((paths::kit_config(team_id, &entry_name), config));
    }

    let colors = match (listed, derived) {
        (Some(colors), _) => Some(colors),
        // A `Midcup` kit without usable `colors.txt` keeps the installed
        // `UniColor.bin` entry, derived or not.
        (None, derived) => match coverage {
            ExportCoverage::Midcup => None,
            ExportCoverage::Full => match derived {
                Some(colors) => {
                    findings.push((Code::KitColorsDerived, Vec::new()));
                    Some(colors)
                }
                None => {
                    findings.push((Code::KitColorsMissing, Vec::new()));
                    Some(MISSING_COLORS)
                }
            },
        },
    };
    let entry = colors.map(|colors| KitColorEntry {
        kit: uni_color_kit_number(slot),
        icon: kit.icon.unwrap_or(DEFAULT_ICON),
        colors,
    });
    Ok(KitCommit {
        entries,
        uniform_parameter,
        uni_color: entry,
    })
}

/// The field of `config` naming the referees' marker collar, `collar` before `winter_collar`;
/// `None` when neither does. Every player wearing that collar would wear the marker.
fn reserved_collar_field(config: &KitConfig) -> Option<&'static str> {
    if config.shirt.collar == REFEREE_MARKER_COLLAR {
        Some("collar")
    } else if config.shirt.winter_collar == REFEREE_MARKER_COLLAR {
        Some("winter_collar")
    } else {
        None
    }
}

/// The engine whose kit UV layout `layout` names.
fn layout_engine(layout: KitLayout) -> Engine {
    match layout {
        KitLayout::PreFox => Engine::PreFox,
        KitLayout::Fox => Engine::Fox,
    }
}

/// The layout marker's name for `layout`, as `kit_layout_converted` reports it.
fn marker_name(layout: KitLayout) -> &'static str {
    match layout {
        KitLayout::PreFox => "pre-fox",
        KitLayout::Fox => "fox",
    }
}

/// The kit layout `engine`'s uniform models map.
fn engine_layout(engine: Engine) -> KitLayout {
    match engine {
        Engine::PreFox => KitLayout::PreFox,
        Engine::Fox => KitLayout::Fox,
    }
}

/// The kit textures the uniform models map, sock islands included, so a kit drawn for the
/// other engine's layout has them re-laid: the main texture and the engine's map.
const UV_MAPPED_STEMS: [&str; 3] = ["kit", "kit_mask", "kit_srm"];

/// The kit textures that are number atlases, whose arrangement of the ten digits differs
/// between the engines. `kit_name`, the name atlas, keeps its letters in the same places in
/// both.
const NUMBER_ATLAS_STEMS: [&str; 3] = ["kit_back", "kit_chest", "kit_leg"];

/// The number atlas `file_name`, in `format`, holding `bytes`, converted for the run's
/// version, first re-arranged into the run's engine's arrangement when its shape is the other
/// engine's (`kit_layout::number_atlas_rearranged`). The shape is read from the header alone,
/// so an atlas already in the run's arrangement, or square, goes through `texture::convert`
/// like any kit texture and is decoded once, by its conversion; only an atlas re-arranged is
/// decoded here, and is converted from the re-arranged copy, not through the run's converter,
/// whose cache holds a source file's own conversion. The decode and the copy are charged to
/// the run's budget until the conversion is done. A failure is the file's, as
/// `texture::convert`'s are.
fn number_atlas(
    ctx: &CompileContext,
    format: SourceFormat,
    file_name: &str,
    bytes: &[u8],
) -> Result<Vec<u8>, TextureError> {
    let failure = |error| texture::conversion_failure(file_name, error);
    let to = engine_layout(ctx.version.engine());
    let size = dds_convert::probe(bytes, format).map_err(failure)?;
    if kit_layout::atlas_arrangement(size.width, size.height).is_none_or(|drawn_as| drawn_as == to)
    {
        return texture::convert(ctx, format, file_name, bytes, true);
    }
    let _decode_charge = texture::decode_charge(&ctx.budget, bytes, format).map_err(failure)?;
    let decoded = decode(bytes, format).map_err(failure)?;
    let Some(atlas) = kit_layout::number_atlas_rearranged(&decoded, to).map_err(failure)? else {
        // Not reached: a decode has the size its header gives. Converted as it is all the same.
        return texture::convert(ctx, format, file_name, bytes, true);
    };
    let _atlas_charge = ctx.budget.charge(atlas.mips.iter().map(Vec::len).sum());
    let target = Target {
        version: ctx.version,
        role: TextureRole::Color,
        keep_uncompressed: true,
    };
    dds_convert::convert(&atlas, target).map_err(failure)
}

/// The kit texture `file_name` the uniform models map (its main texture, its mask or its srm),
/// in `format`, holding `bytes`, drawn for the `drawn_for` layout: decoded, re-laid out for
/// the other one (`kit_layout::relaid`) and converted for the run's version. Not through the
/// run's converter, whose cache holds a source file's own conversion, which this is not.
/// `keep_uncompressed` is `Target`'s, as `kit` computes it per stem. A
/// failure is the file's, as `texture::convert`'s are. The decode and its re-laid copy are
/// charged to the run's budget until the conversion is done.
fn relaid_texture(
    ctx: &CompileContext,
    format: SourceFormat,
    file_name: &str,
    bytes: &[u8],
    drawn_for: KitLayout,
    keep_uncompressed: bool,
) -> Result<Vec<u8>, TextureError> {
    let failure = |error| texture::conversion_failure(file_name, error);
    let _decode_charge = texture::decode_charge(&ctx.budget, bytes, format).map_err(failure)?;
    let decoded = decode(bytes, format).map_err(failure)?;
    // The re-laid copy has the decode's size: its pixels, and its blocks when it keeps them.
    let pixels = decoded.mips.iter().map(Vec::len);
    let blocks = decoded
        .blocks
        .iter()
        .flat_map(|blocks| &blocks.mips)
        .map(Vec::len);
    let _copy_charge = ctx.budget.charge(pixels.chain(blocks).sum());
    let relaid = kit_layout::relaid(&ctx.budget, &decoded, drawn_for).map_err(failure)?;
    let target = Target {
        version: ctx.version,
        role: TextureRole::Color,
        keep_uncompressed,
    };
    dds_convert::convert(&relaid, target).map_err(failure)
}

/// The two colors a kit's `colors.txt` holding `bytes` gives, or `None` when it gives fewer.
fn listed_colors(bytes: &[u8]) -> Option<[Rgb; KIT_COLORS]> {
    match read_colors_txt(bytes, KIT_COLORS).colors.as_slice() {
        [first, second] => Some([*first, *second]),
        _ => None,
    }
}

/// The two menu colors the main texture `file_name`, in `format`, holding `bytes`, gives: its
/// top level's dominant colors (`spec/libs/color_tools.md` "Dominant kit-color extraction"). `None`
/// when it does not decode or its shirt region has no opaque pixel. The texture is decoded
/// here and again by its conversion: `dds_convert` has no decode that both could share
/// through the conversion cache, and only a kit without two listed colors pays the second.
/// This decode is charged to `budget` until it is freed, on return, before the conversion's.
fn derived_colors(
    budget: &Arc<MemoryBudget>,
    format: SourceFormat,
    file_name: &str,
    bytes: &[u8],
) -> Option<[Rgb; KIT_COLORS]> {
    let _decode_charge = match texture::decode_charge(budget, bytes, format) {
        Ok(charge) => charge,
        // Its conversion fails on the same error and reports it, failing the kit.
        Err(error) => {
            log::debug!(
                "{file_name}: no kit colors from a texture whose header does not read: {error}"
            );
            return None;
        }
    };
    let decoded = match decode(bytes, format) {
        Ok(decoded) => decoded,
        // Its conversion fails on the same error and reports it, failing the kit.
        Err(error) => {
            log::debug!("{file_name}: no kit colors from a texture that does not decode: {error}");
            return None;
        }
    };
    let top = decoded.mips.first()?;
    color_tools::kit::extract_kit_colors(top, decoded.width, decoded.height)
        .map(|colors| [colors.color1, colors.color2])
}
