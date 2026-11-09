//! A selected model of the other engine's format converted for the target through
//! `model_convert::convert`, which also moves it onto the target version's skeleton. On PES
//! 15-17 an `.fmdl`, with its paired `.skl` as the bind pose, is written as a `.model` and its
//! material set. On PES 18-21 a `.model`, with the `.mtl` its search finds (a collar's, the
//! templates' `uniform.mtl`), is written as an FMDL and, when the conversion makes one, a
//! skeleton. A model of the target's own format that the conversion pre-check
//! (`needs_conversion`) finds posed off the version's skeleton is converted the same way: an
//! FMDL on PES 18-21, a `.model` on PES 15-17.
//!
//! Every converted model is checked in its target form before it is written (a far vertex is
//! `vertex_too_far_from_origin`): what the source passed, the conversion may still have moved
//! or split past a limit.

use model_convert::{Converted, NativeModelBundle, Subject, convert, loss, needs_conversion};
use pes_model::format::PreFoxModel;
use pes_model::format::mtl::MaterialSet;
use pes_version::PesVersion;

use super::{CompileContext, Finding, TaskFailure};
use crate::deep::{FAR_VERTEX_CODES, Fired, summed};
use crate::messages::Code;
use crate::paths;

/// What the materials of a model converted to the other engine's format are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConvertedMaterials {
    /// The converter's: in a `.model`, names the material set written beside it defines; in
    /// an FMDL, the materials the converter builds with their textures. A player's face, boots
    /// or gloves model.
    Converted,
    /// A collar's, which the game dresses with the kit: in a
    /// `.model` converted from an FMDL, the stock collars' names, which the game's shared
    /// `uniform.mtl` defines (`stock_collar_materials`; a `.model` collar moved onto the
    /// version's skeleton keeps its author's, `model_for_pre_fox`); in an FMDL, the stock
    /// collars' materials by name (`stock_collar_fox_materials`). A collar writes no material
    /// set of its own, so the conversion's losses about a material are not reported
    /// (`reports`).
    StockCollar,
}

impl ConvertedMaterials {
    /// Whether the conversion's `loss` is reported for a model whose materials are so. A
    /// collar's skips every loss about a material: it writes no material set and its
    /// materials are renamed or replaced by the stock ones, so such a loss describes nothing in
    /// the output the member can change.
    fn reports(self, loss: &loss::Finding) -> bool {
        match self {
            ConvertedMaterials::Converted => true,
            ConvertedMaterials::StockCollar => !matches!(loss.subject, Subject::Material(_)),
        }
    }
}

/// A model converted for a PES 15-17 target: an FMDL, or a `.model` the pre-check found posed
/// off the version's skeleton (`model_for_pre_fox`, which keeps the `.model` alone).
pub(super) struct PreFoxConversion {
    /// The `.model` written, packed in the FMDL's place.
    pub(super) model: Vec<u8>,
    /// The converter's material set, under the converter's names whatever `ConvertedMaterials`
    /// renamed in the `.model`: packed as `<stem>.mtl` beside a player's model; the collar task
    /// does not write it, the shared `uniform.mtl` dressing a collar.
    pub(super) materials: MaterialSet,
}

/// A model converted for a PES 18-21 target: a `.model`, or an FMDL the pre-check found posed
/// off the version's skeleton (`fmdl_for_fox`).
pub(super) struct FoxConversion {
    /// The FMDL written, a part in the source's place.
    pub(super) model: Vec<u8>,
    /// The skeleton the conversion writes when a bone the model keeps is outside the game's
    /// skeleton tables (`ExportedFox::skl`): the part's skeleton, as a member's `.skl` of the
    /// model's stem would be. `None` when every bone is the game's own.
    pub(super) skeleton: Option<Vec<u8>>,
}

/// The FMDL `name` (as its findings name it: `named_on_folder`, a collar's file name), `bytes`,
/// converted for `ctx.version`, a PES 15-17 target,
/// `skeleton` being the bytes of its paired `.skl`, its bind pose, when its folder holds one
/// (`None` binds it to the version's body table), its `.model`'s material names as
/// `materials` says. The conversion's parsed forms are charged to the run's memory budget at
/// the source's size while they live. What the conversion reports that the member is told is
/// noted in `findings` naming the model (`reported`), but for a collar's losses about a
/// material (`ConvertedMaterials::reports`). A model or skeleton the conversion cannot read,
/// convert or write fails the task with `model_conversion_failed`, naming the model, and so
/// does a `.model` written that `pes_model`'s check finds an Error in (`target_form_failure`).
/// The deep pass drops an FMDL whose every mesh is hidden before any task is planned
/// (`model_hidden_dropped`).
pub(super) fn fmdl_for_pre_fox(
    name: &str,
    bytes: &[u8],
    skeleton: Option<&[u8]>,
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
    materials: ConvertedMaterials,
) -> Result<PreFoxConversion, TaskFailure> {
    let (converted, losses) = {
        // The FMDL, its IR and the `.model` written, charged at the source's size, an estimate
        // of each form, while they are built.
        let _conversion_charge = ctx.budget.charge(bytes.len());
        let bundle = fox_bundle(bytes, skeleton).map_err(|error| failed(name, error))?;
        let converted = convert(bundle, ctx.version).map_err(|error| failed(name, error.into()))?;
        pre_fox_written(name, converted, ctx.version, materials)?
    };
    findings.extend(
        losses
            .iter()
            .filter(|loss| materials.reports(loss))
            .filter_map(|loss| reported(name, loss)),
    );
    Ok(converted)
}

/// The member's `.model` `name` (as its findings name it, `named_on_folder`), `source`, for a PES
/// 15-17 target, `mtl` being the bytes of the `.mtl` its search found (a collar's, the
/// templates' `uniform.mtl`), which the IR import reads its materials from, run through the
/// conversion pre-check for `ctx.version`: the bytes to pack, `source` as it is when
/// re-binding would change nothing, otherwise the `.model` moved onto the version's skeleton.
/// The material set the conversion writes is dropped: the member's `.mtl` is packed and
/// pointed as for any model of theirs (a collar writes none), and moving the bones changes no
/// material. The `.model` keeps its own material names whatever `materials` says: a collar
/// already in the target's format keeps its author's, so
/// `materials` decides only which losses are reported (`ConvertedMaterials::reports`).
/// Charged, reported and failed as `fmdl_for_pre_fox` is.
pub(super) fn model_for_pre_fox(
    name: &str,
    source: Vec<u8>,
    mtl: &[u8],
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
    materials: ConvertedMaterials,
) -> Result<Vec<u8>, TaskFailure> {
    let (converted, losses) = {
        // The `.model`, its IR and the `.model` written, charged at the source's size, an
        // estimate of each form, while they are built.
        let _conversion_charge = ctx.budget.charge(source.len());
        let bundle = pre_fox_bundle(&source, mtl).map_err(|error| failed(name, error))?;
        if !needs_conversion(&bundle, ctx.version) {
            return Ok(source);
        }
        // A `.model` has no hidden flag, so `EveryMeshHidden` cannot come of it. Written as
        // `Converted`, which renames nothing: the names are the source's own.
        let converted = convert(bundle, ctx.version).map_err(|error| failed(name, error.into()))?;
        pre_fox_written(name, converted, ctx.version, ConvertedMaterials::Converted)?
    };
    findings.extend(
        losses
            .iter()
            .filter(|loss| materials.reports(loss))
            .filter_map(|loss| reported(name, loss)),
    );
    Ok(converted.model)
}

/// The model `name` (as its findings name it), `converted` for the PES 15-17 `version`,
/// written, its materials named as `materials` says, with the conversion's loss findings: the
/// `.model`, checked by `pes_model`'s check (`target_form_failure`), and the converter's
/// material set. Failed with `model_conversion_failed`, naming the model, when it cannot be
/// written.
fn pre_fox_written(
    name: &str,
    converted: Converted,
    version: PesVersion,
    materials: ConvertedMaterials,
) -> Result<(PreFoxConversion, Vec<loss::Finding>), TaskFailure> {
    let Converted { bundle, findings } = converted;
    let (mut model, material_set) = match bundle {
        NativeModelBundle::PreFox { model, mtl } => (model, mtl),
        NativeModelBundle::Fox { .. } => {
            unreachable!("`convert` returns the target's format, a `.model` for PES 15-17")
        }
    };
    match materials {
        ConvertedMaterials::Converted => {}
        ConvertedMaterials::StockCollar => stock_collar_materials(&mut model, version),
    }
    let fired = pes_model::check::check(&model)
        .into_iter()
        .map(Fired::pre_fox);
    target_form_failure(name, fired.collect())?;
    let written = model.to_file().and_then(|file| file.write());
    let model = written.map_err(|error| failed(name, error.into()))?;
    let converted = PreFoxConversion {
        model,
        materials: material_set,
    };
    Ok((converted, findings))
}

/// Renames the materials of `model`, an FMDL collar converted for the pre-Fox `version`, to
/// the stock collars', which the game's shared `uniform.mtl`
/// defines, so the collar is dressed as the stock ones are. On PES 17 its first material, the
/// FMDL's first, becomes `uni_collar` and every other `uni_shirts`, the list collapsed to
/// those names with each mesh pointed at its own, since a `.model` lists a material once. On
/// PES 15-16 every material becomes `uni_shirts`, the one name, every mesh on it: those
/// versions' `uniform.mtl` defines no `uni_collar`.
fn stock_collar_materials(model: &mut pes_model::model::Model, version: PesVersion) {
    let defines_uni_collar = match version {
        PesVersion::Pes15 | PesVersion::Pes16 => false,
        PesVersion::Pes17 => true,
        PesVersion::Pes18 | PesVersion::Pes19 | PesVersion::Pes20 | PesVersion::Pes21 => {
            unreachable!("a `.model` is written for PES 15-17 alone")
        }
    };
    if !defines_uni_collar {
        model.materials = vec!["uni_shirts".to_owned()];
        for mesh in &mut model.meshes {
            mesh.material = 0;
        }
        return;
    }
    let mut names = vec!["uni_collar".to_owned()];
    if model.materials.len() > 1 {
        names.push("uni_shirts".to_owned());
    }
    model.materials = names;
    for mesh in &mut model.meshes {
        mesh.material = usize::from(mesh.material != 0);
    }
}

/// The `.model` `name` (as its findings name it, `named_on_folder`), `bytes`, converted for
/// `ctx.version`, a PES 18-21 target, `mtl` being the bytes of the `.mtl` its search found
/// (`mtl_search::mtl_for`), or a collar's, the templates' `uniform.mtl`, which defines its
/// materials, the FMDL's materials as `materials` says. Charged, reported and failed as
/// `fmdl_for_pre_fox` is, the FMDL written checked by `fmdl`'s check.
pub(super) fn model_for_fox(
    name: &str,
    bytes: &[u8],
    mtl: &[u8],
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
    materials: ConvertedMaterials,
) -> Result<FoxConversion, TaskFailure> {
    let (converted, losses) = {
        // The `.model`, its IR and the FMDL written, charged at the source's size, an estimate
        // of each form, while they are built.
        let _conversion_charge = ctx.budget.charge(bytes.len());
        let bundle = pre_fox_bundle(bytes, mtl).map_err(|error| failed(name, error))?;
        fox_written(name, bundle, ctx.version, materials)?
    };
    findings.extend(
        losses
            .iter()
            .filter(|loss| materials.reports(loss))
            .filter_map(|loss| reported(name, loss)),
    );
    Ok(converted)
}

/// The member's FMDL `name` (as its findings name it, `named_on_folder`), `bytes`, for a PES
/// 18-21 target, with `skeleton`, the bytes of the `.skl` paired with it, as its bind pose
/// (`None`: PES 21's pose, `needs_conversion`'s assumption), run through the conversion
/// pre-check for `ctx.version`: `None` when re-binding would change nothing, and the caller
/// packs `bytes` as they are; otherwise the FMDL moved onto the version's skeleton and the
/// skeleton the conversion writes, if any. Charged, reported and failed as `fmdl_for_pre_fox`
/// is, the FMDL written checked by `fmdl`'s check.
pub(super) fn fmdl_for_fox(
    name: &str,
    bytes: &[u8],
    skeleton: Option<&[u8]>,
    ctx: &CompileContext,
    findings: &mut Vec<Finding>,
) -> Result<Option<FoxConversion>, TaskFailure> {
    let (converted, losses) = {
        // The FMDL, its IR and the FMDL written, charged at the source's size, an estimate of
        // each form, while they are built.
        let _conversion_charge = ctx.budget.charge(bytes.len());
        let bundle = fox_bundle(bytes, skeleton).map_err(|error| failed(name, error))?;
        if !needs_conversion(&bundle, ctx.version) {
            return Ok(None);
        }
        fox_written(name, bundle, ctx.version, ConvertedMaterials::Converted)?
    };
    findings.extend(losses.iter().filter_map(|loss| reported(name, loss)));
    Ok(Some(converted))
}

/// The `.model` `bytes` read, with the `.mtl` `mtl` defining its materials.
fn pre_fox_bundle(bytes: &[u8], mtl: &[u8]) -> anyhow::Result<NativeModelBundle> {
    let model = pes_model::model::Model::from_file(&PreFoxModel::read(bytes)?)?;
    let mtl = MaterialSet::read(mtl)?;
    Ok(NativeModelBundle::PreFox { model, mtl })
}

/// The FMDL `bytes` read, with the `.skl` `skeleton` as its bind pose when there is one.
/// Its texture directories' short forms are read as their long forms first
/// (`paths::export_fmdl_directory`), so the conversion sees the long form alone and a
/// `.mtl` written from it names what the long spelling would.
fn fox_bundle(bytes: &[u8], skeleton: Option<&[u8]>) -> anyhow::Result<NativeModelBundle> {
    let mut model = fmdl::Model::from_file(&fmdl::FmdlFile::read(bytes)?)?;
    for material in &mut model.materials {
        for (_, texture) in &mut material.textures {
            texture.directory = paths::export_fmdl_directory(&texture.directory);
        }
    }
    let skl = skeleton.map(fmdl::SklFile::read).transpose()?;
    Ok(NativeModelBundle::Fox { model, skl })
}

/// The model `name` (as its findings name it), `bundle`, converted for the PES 18-21
/// `version` and written, its materials as `materials` says, with the conversion's loss
/// findings: the FMDL, checked by `fmdl`'s check (`target_form_failure`), and the skeleton the
/// conversion writes, if any. Failed with `model_conversion_failed`, naming the model, when it
/// cannot be converted or written.
fn fox_written(
    name: &str,
    bundle: NativeModelBundle,
    version: PesVersion,
    materials: ConvertedMaterials,
) -> Result<(FoxConversion, Vec<loss::Finding>), TaskFailure> {
    let Converted { bundle, findings } =
        convert(bundle, version).map_err(|error| failed(name, error.into()))?;
    let (mut model, skeleton) = match bundle {
        NativeModelBundle::Fox { model, skl } => (model, skl),
        NativeModelBundle::PreFox { .. } => {
            unreachable!("`convert` returns the target's format, an FMDL for PES 18-21")
        }
    };
    match materials {
        ConvertedMaterials::Converted => {}
        ConvertedMaterials::StockCollar => {
            stock_collar_fox_materials(&mut model).map_err(|error| failed(name, error))?;
        }
    }
    let fired = fmdl::check::check(&model).into_iter().map(Fired::fox);
    target_form_failure(name, fired.collect())?;
    let file = model
        .to_file()
        .map_err(|error| failed(name, error.into()))?;
    let converted = FoxConversion {
        model: file.write(),
        skeleton: skeleton.map(|skeleton| skeleton.write()),
    };
    Ok((converted, findings))
}

/// The parameters both of the game's own collar materials carry on Fox, in file order, as
/// PES 21's `collar_107.fmdl` has them.
const STOCK_COLLAR_PARAMETERS: [(&str, [f32; 4]); 8] = [
    ("MatParamIndex_0", [40.0, 0.0, 0.0, 0.0]),
    ("BlendNormalXParam", [0.0, 0.0, 0.0, 0.0]),
    ("BlendNormalYParam", [0.666, 0.0, 0.0, 0.0]),
    ("RepetitionParam", [80.0, 0.0, 0.0, 0.0]),
    ("BlendCoeffParam", [0.0, 0.0, 0.0, 0.0]),
    ("BlendBoostParam", [0.0, 0.0, 0.0, 0.0]),
    ("PatchAnisoRoughnessParam", [0.4, 0.0, 0.0, 0.0]),
    ("PatternIndexParam", [1.0, 0.0, 0.0, 0.0]),
];

/// Gives each material of `model`, a `.model` collar converted for Fox, the game's own collar
/// material of its name, as PES 21's `collar_107.fmdl` has it: `uni_collar` shader
/// `pes_3ddf_collar`, `uni_shirts` `pes_3ddf_shirt_nb`, each binding the one sampler
/// `Pattern_Tex_LIN` to the game's `uni_pattern.dds` and carrying `STOCK_COLLAR_PARAMETERS`;
/// every mesh drawn as that file's are, two-sided (alpha flags 32) and with shadow flags 0. A
/// material of any other name is an error naming it: a collar uses those two names alone.
fn stock_collar_fox_materials(model: &mut fmdl::Model) -> anyhow::Result<()> {
    // The converter takes a mesh's draw flags from its `.mtl` material, and `uniform.mtl`'s
    // `uni_collar` is one-sided (`twosided` 0), so its meshes would come out at alpha flags 0
    // where every mesh of the game's own collar is two-sided.
    for mesh in &mut model.meshes {
        mesh.alpha_flags = 32;
        mesh.shadow_flags = 0;
    }
    for material in &mut model.materials {
        // The game draws a collar through these two shaders and its pattern texture, which
        // the kit dresses; the converter's material for `uniform.mtl`'s `Shirt_NB`
        // (`fox3ddf_blin`, with or without its samplers) drew nothing on PES 21, so the stock
        // material is copied by name rather than mapped from the converter's.
        let (shader, technique) = match material.name.as_str() {
            "uni_collar" => ("pes_3ddf_collar", "pes3DDF_Collar_NC"),
            "uni_shirts" => ("pes_3ddf_shirt_nb", "pes3DDF_Shirt_NB_NC"),
            other => {
                anyhow::bail!("the collar material `{other}` must be `uni_collar` or `uni_shirts`")
            }
        };
        shader.clone_into(&mut material.shader);
        technique.clone_into(&mut material.technique);
        material.textures = vec![(
            "Pattern_Tex_LIN".to_owned(),
            fmdl::Texture {
                file_name: "uni_pattern.dds".to_owned(),
                directory: "/Assets/pes16/model/character/common/sourceimages/".to_owned(),
            },
        )];
        material.parameters = STOCK_COLLAR_PARAMETERS
            .iter()
            .map(|(name, value)| ((*name).to_owned(), *value))
            .collect();
    }
    Ok(())
}

/// `model_conversion_failed` for the model `name` (as its findings name it), carrying
/// `error`'s chain.
fn failed(name: &str, error: anyhow::Error) -> TaskFailure {
    TaskFailure {
        code: Code::ModelConversionFailed,
        context: vec![("model", name.to_owned()), ("error", format!("{error:#}"))],
    }
}

/// The task failure the converted model `name` (as its findings name it) is, `fired` being the
/// rules its format crate's check fired on it in its target form; `Ok` when none of them is an
/// Error. A far vertex is
/// `vertex_too_far_from_origin`, naming the model and how many vertices are far, summed over
/// the model as the deep pass sums a source's (`deep::summed`); any other Error is
/// `model_conversion_failed` with the rule's code as the error: the source passed that check,
/// so the conversion made the model the game cannot load. A Warning or an Info is not
/// reported. Either failure leaves the model's package out, `pass_through` or not, as every
/// task failure does: a failed task commits nothing, and the folder's other packages and its
/// textures stand.
fn target_form_failure(name: &str, fired: Vec<Fired>) -> Result<(), TaskFailure> {
    let errors: Vec<Fired> = summed(fired)
        .into_iter()
        .filter(|rule| rule.error)
        .collect();
    // The far vertex first: it names what the member can fix in the source.
    if let Some(far) = errors
        .iter()
        .find(|rule| FAR_VERTEX_CODES.contains(&rule.code))
    {
        return Err(TaskFailure {
            code: Code::VertexTooFarFromOrigin,
            context: vec![("model", name.to_owned()), ("count", far.count.to_string())],
        });
    }
    match errors.first() {
        Some(rule) => Err(TaskFailure {
            code: Code::ModelConversionFailed,
            context: vec![("model", name.to_owned()), ("error", rule.code.to_owned())],
        }),
        None => Ok(()),
    }
}

/// The finding the conversion's `loss` of the model `name` (as its findings name it) is
/// reported as, on the folder, at its catalog row's severity. Its context is the model, then
/// the index of the mesh, material or bone the loss is about, then the loss's detail under the
/// row's key when the row names one. A fold's detail,
/// `<bone> -> <target>`, names the bone itself and is given in the index's place.
/// `native_field_dropped` of the Fox mesh flag a `.mtl` cannot express, `no_shadow_cast`, is
/// `mesh_flags_dropped`, a Warning: the mesh casts a shadow where the source did not. `None`
/// for a code `model_convert::loss` does not document.
fn reported(name: &str, loss: &loss::Finding) -> Option<Finding> {
    let (code, detail_key) = match loss.code {
        "bone_matrix_unknown" => (Code::BoneMatrixUnknown, Some("name")),
        "bone_slot_dropped" => (Code::BoneSlotDropped, Some("count")),
        "material_family_approximated" => (Code::MaterialFamilyApproximated, Some("name")),
        "material_split_by_flags" => (Code::MaterialSplitByFlags, Some("name")),
        "material_texture_unused" => (Code::MaterialTextureUnused, Some("texture")),
        "material_parameter_dropped" => (Code::MaterialParameterDropped, Some("parameter")),
        "sampler_settings_defaulted" => (Code::SamplerSettingsDefaulted, Some("sampler")),
        "vertex_bitangents_dropped" => (Code::VertexBitangentsDropped, None),
        "dummy_texture_added" => (Code::DummyTextureAdded, Some("sampler")),
        "native_field_dropped" => match loss.detail.as_str() {
            "no_shadow_cast" => (Code::MeshFlagsDropped, Some("field")),
            _ => (Code::NativeFieldDropped, Some("field")),
        },
        "bone_folded_for_version" => (Code::BoneFoldedForVersion, Some("bone")),
        "bone_folded_by_position" => (Code::BoneFoldedByPosition, Some("bone")),
        "skeleton_retargeted" => (Code::SkeletonRetargeted, Some("bones")),
        "static_bone_added" => (Code::StaticBoneAdded, None),
        "weight_clamped" => (Code::WeightClamped, Some("count")),
        "empty_mesh_bone_group_dropped" => (Code::EmptyMeshBoneGroupDropped, Some("count")),
        // `loss.code` is a string `model_convert` documents rather than an enum: a code it
        // adds is unknown here until the catalog gives it a row.
        _ => return None,
    };
    let mut context = vec![("model", name.to_owned())];
    let subject = match loss.subject {
        Subject::Model => None,
        Subject::Mesh(index) => Some(("mesh", index)),
        Subject::Material(index) => Some(("material", index)),
        Subject::Bone(index) => Some(("bone", index)),
    };
    if let Some((key, index)) = subject
        && detail_key != Some(key)
    {
        context.push((key, index.to_string()));
    }
    if let Some(key) = detail_key {
        context.push((key, loss.detail.clone()));
    }
    Some((code, context))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pipeline::MemoryBudget;

    use super::*;
    use crate::bins::installed::InstalledPaths;
    use crate::processing::EntryTarget;
    use crate::templates::Templates;

    /// A run's context for `version`, with nothing installed and no memory limit.
    fn context(version: PesVersion) -> CompileContext {
        CompileContext::new(
            version,
            1,
            Templates::embedded(),
            InstalledPaths::Unknown,
            EntryTarget::GamePaths {
                engine: version.engine(),
            },
            MemoryBudget::new(usize::MAX),
            false,
        )
    }

    /// The bytes of `pes_model`'s fixture `name`: real pre-Fox models and material sets.
    fn pre_fox_fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/pes_model/tests/fixtures")
                .join(name),
        )
        .unwrap()
    }

    /// Konami's referee card, a clean one-bone `.model` whose material `konami_card_red.mtl`
    /// defines, with `edit` applied.
    fn card(edit: impl FnOnce(&mut pes_model::model::Model)) -> Vec<u8> {
        let file = PreFoxModel::read(&pre_fox_fixture("konami_card.model")).unwrap();
        let mut model = pes_model::model::Model::from_file(&file).unwrap();
        edit(&mut model);
        model.to_file().unwrap().write().unwrap()
    }

    /// The failure's code and context, or `None` when the conversion succeeded.
    fn failure<T>(result: Result<T, TaskFailure>) -> Option<(Code, Vec<(&'static str, String)>)> {
        result.err().map(|failure| (failure.code, failure.context))
    }

    #[test]
    fn a_model_converts_for_pes_21_to_an_fmdl_fmdl_reads_on_the_game_s_skeleton() {
        let source = card(|_| {});
        let mtl = pre_fox_fixture("konami_card_red.mtl");
        let mut findings = Vec::new();
        let Ok(converted) = model_for_fox(
            "card.model",
            &source,
            &mtl,
            &context(PesVersion::Pes21),
            &mut findings,
            ConvertedMaterials::Converted,
        ) else {
            panic!("the card converts");
        };
        let model =
            fmdl::Model::from_file(&fmdl::FmdlFile::read(&converted.model).unwrap()).unwrap();
        let source_model =
            pes_model::model::Model::from_file(&PreFoxModel::read(&source).unwrap()).unwrap();
        assert_eq!(
            (source_model.meshes.len(), model.meshes.len()),
            (1, 1),
            "the card's meshes, then the FMDL's"
        );
        // The card's one bone is the game's own, at its pose: no skeleton of its own, and no
        // bone moved. What the member is told is what the FMDL adds and drops: the dummy
        // maps its `Basic_C` material lacks, and the bitangents an FMDL has no place for.
        assert!(converted.skeleton.is_none());
        let model_named = |rest: &[(&'static str, &str)]| {
            let mut context = vec![("model", "card.model".to_owned())];
            context.extend(rest.iter().map(|(key, value)| (*key, (*value).to_owned())));
            context
        };
        assert_eq!(
            findings,
            [
                (
                    Code::DummyTextureAdded,
                    model_named(&[("material", "0"), ("sampler", "NormalMap_Tex_NRM")])
                ),
                (
                    Code::DummyTextureAdded,
                    model_named(&[("material", "0"), ("sampler", "SpecularMap_Tex_LIN")])
                ),
                (Code::VertexBitangentsDropped, model_named(&[("mesh", "0")])),
            ]
        );
    }

    #[test]
    fn a_far_vertex_in_the_converted_form_is_vertex_too_far_from_origin_in_both_directions() {
        let far_card = card(|model| model.meshes[0].vertices.positions[0] = [6000.0, 0.0, 0.0]);
        let mtl = pre_fox_fixture("konami_card_red.mtl");
        let mut findings = Vec::new();
        assert_eq!(
            failure(model_for_fox(
                "card.model",
                &far_card,
                &mtl,
                &context(PesVersion::Pes21),
                &mut findings,
                ConvertedMaterials::Converted
            )),
            Some((
                Code::VertexTooFarFromOrigin,
                vec![
                    ("model", "card.model".to_owned()),
                    ("count", "1".to_owned())
                ]
            ))
        );
        // The tracer's boots with one vertex 6000 units from the origin, for PES 17.
        let far_boots = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/deep/boots_far.fmdl"),
        )
        .unwrap();
        assert_eq!(
            failure(fmdl_for_pre_fox(
                "boots.fmdl",
                &far_boots,
                None,
                &context(PesVersion::Pes17),
                &mut findings,
                ConvertedMaterials::Converted
            )),
            Some((
                Code::VertexTooFarFromOrigin,
                vec![
                    ("model", "boots.fmdl".to_owned()),
                    ("count", "1".to_owned())
                ]
            ))
        );
        assert_eq!(findings, [], "a failed conversion reports nothing else");
    }

    #[test]
    fn a_model_collar_moved_for_pre_fox_keeps_its_names_and_reports_no_material_loss() {
        // PES 17's stock collar 1, posed off PES 15's skeleton, read with the templates'
        // `uniform.mtl` whose `uni_shirts` has a shader no family rule knows, which the import
        // reports as `material_family_approximated`, a loss about a material.
        let source = pre_fox_fixture("konami_collar_001.wesys.model");
        let mut set = MaterialSet::read(Templates::embedded().uniform_mtl()).unwrap();
        set.materials
            .iter_mut()
            .find(|material| material.name == "uni_shirts")
            .unwrap()
            .shader = "Unknown_Collar_Shader".to_owned();
        let mtl = set.write();
        let moved = |materials: ConvertedMaterials| {
            let mut findings = Vec::new();
            let Ok(bytes) = model_for_pre_fox(
                "collar_12.model",
                source.clone(),
                &mtl,
                &context(PesVersion::Pes15),
                &mut findings,
                materials,
            ) else {
                panic!("the collar converts");
            };
            let model =
                pes_model::model::Model::from_file(&PreFoxModel::read(&bytes).unwrap()).unwrap();
            let codes: Vec<Code> = findings.into_iter().map(|(code, _)| code).collect();
            (model.materials, codes)
        };

        // A player's `.model` is told of the approximated family.
        let (names, codes) = moved(ConvertedMaterials::Converted);
        assert_eq!(names, ["uni_shirts", "uni_collar"]);
        assert!(
            codes.contains(&Code::MaterialFamilyApproximated),
            "{codes:?}"
        );
        // A collar skips it, its names kept and its bone losses still reported.
        let (names, codes) = moved(ConvertedMaterials::StockCollar);
        assert_eq!(names, ["uni_shirts", "uni_collar"]);
        assert_eq!(
            codes,
            [
                Code::BoneFoldedForVersion,
                Code::BoneFoldedForVersion,
                Code::SkeletonRetargeted
            ]
        );
    }

    #[test]
    fn a_collar_s_first_material_is_uni_collar_and_every_other_uni_shirts() {
        // The card's one mesh, copied for each binding.
        let with_materials = |materials: &[&str], bindings: &[usize]| {
            let file = PreFoxModel::read(&card(|_| {})).unwrap();
            let mut model = pes_model::model::Model::from_file(&file).unwrap();
            model.materials = materials.iter().map(|name| (*name).to_owned()).collect();
            let mesh = model.meshes[0].clone();
            model.meshes = bindings
                .iter()
                .map(|material| pes_model::model::Mesh {
                    material: *material,
                    ..mesh.clone()
                })
                .collect();
            model
        };
        let renamed = |mut model: pes_model::model::Model, version| {
            stock_collar_materials(&mut model, version);
            let bindings: Vec<usize> = model.meshes.iter().map(|mesh| mesh.material).collect();
            (model.materials, bindings)
        };

        assert_eq!(
            renamed(
                with_materials(&["a", "b", "c"], &[0, 1, 2, 0]),
                PesVersion::Pes17
            ),
            (
                vec!["uni_collar".to_owned(), "uni_shirts".to_owned()],
                vec![0, 1, 1, 0]
            )
        );
        assert_eq!(
            renamed(with_materials(&["a"], &[0, 0]), PesVersion::Pes17),
            (vec!["uni_collar".to_owned()], vec![0, 0])
        );
        // PES 15-16's `uniform.mtl` defines no `uni_collar`: one name, every mesh on it.
        for version in [PesVersion::Pes15, PesVersion::Pes16] {
            assert_eq!(
                renamed(with_materials(&["a", "b", "c"], &[0, 1, 2, 0]), version),
                (vec!["uni_shirts".to_owned()], vec![0, 0, 0, 0]),
                "{version}"
            );
        }
    }

    #[test]
    fn a_model_collar_naming_a_material_no_stock_collar_has_fails_its_fox_conversion() {
        // PES 17's stock collar 1, its `uni_collar` renamed `skin_limb`: a material the
        // templates' `uniform.mtl` defines, so the conversion reads it and only the stock
        // collar materials' table refuses it.
        let file = PreFoxModel::read(&pre_fox_fixture("konami_collar_001.wesys.model")).unwrap();
        let mut model = pes_model::model::Model::from_file(&file).unwrap();
        for name in &mut model.materials {
            if name == "uni_collar" {
                "skin_limb".clone_into(name);
            }
        }
        let source = model.to_file().unwrap().write().unwrap();
        let ctx = context(PesVersion::Pes21);
        let mut findings = Vec::new();
        assert_eq!(
            failure(model_for_fox(
                "collar_12.model",
                &source,
                ctx.templates.uniform_mtl(),
                &ctx,
                &mut findings,
                ConvertedMaterials::StockCollar,
            )),
            Some((
                Code::ModelConversionFailed,
                vec![
                    ("model", "collar_12.model".to_owned()),
                    (
                        "error",
                        "the collar material `skin_limb` must be `uni_collar` or `uni_shirts`"
                            .to_owned()
                    )
                ]
            ))
        );
    }

    #[test]
    fn a_collar_s_conversion_reports_no_loss_about_a_material_it_does_not_write() {
        // The tracer's boots, the CLI's TC-CMN-09 collar: one of its meshes casts no shadow.
        let boots = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/boots.fmdl",
        ))
        .unwrap();
        let findings_of = |materials| {
            let mut findings = Vec::new();
            let Ok(_) = fmdl_for_pre_fox(
                "boots.fmdl",
                &boots,
                None,
                &context(PesVersion::Pes17),
                &mut findings,
                materials,
            ) else {
                panic!("the boots convert for PES 17");
            };
            findings
        };
        let names_a_material =
            |finding: &Finding| finding.1.iter().any(|(key, _)| *key == "material");

        let converted = findings_of(ConvertedMaterials::Converted);
        assert!(
            converted
                .iter()
                .any(|finding| finding.0 == Code::MeshFlagsDropped && names_a_material(finding)),
            "{converted:#?}"
        );
        let collar = findings_of(ConvertedMaterials::StockCollar);
        assert!(!collar.iter().any(names_a_material), "{collar:#?}");
        assert!(
            collar.contains(&(
                Code::NativeFieldDropped,
                vec![
                    ("model", "boots.fmdl".to_owned()),
                    ("field", "bone_matrices".to_owned())
                ]
            )),
            "{collar:#?}"
        );
    }

    #[test]
    fn any_other_error_in_the_converted_form_is_model_conversion_failed_and_the_rest_nothing() {
        let rule = |code: &'static str, error: bool, count: usize| Fired { code, error, count };
        assert_eq!(
            failure(target_form_failure(
                "boots.model",
                vec![
                    rule("fmdl_weights_not_normalized", false, 1662),
                    rule("fmdl_mesh_over_face_limit", true, 1),
                ]
            )),
            Some((
                Code::ModelConversionFailed,
                vec![
                    ("model", "boots.model".to_owned()),
                    ("error", "fmdl_mesh_over_face_limit".to_owned())
                ]
            ))
        );
        // The far vertex is named first, its count summed over the model's meshes.
        assert_eq!(
            failure(target_form_failure(
                "boots.model",
                vec![
                    rule("fmdl_mesh_over_face_limit", true, 1),
                    rule("fmdl_vertex_far_from_origin", true, 2),
                    rule("fmdl_vertex_far_from_origin", true, 3),
                ]
            )),
            Some((
                Code::VertexTooFarFromOrigin,
                vec![
                    ("model", "boots.model".to_owned()),
                    ("count", "5".to_owned())
                ]
            ))
        );
        assert_eq!(
            failure(target_form_failure(
                "boots.fmdl",
                vec![
                    rule("model_weights_not_normalized", false, 4),
                    rule("model_mesh_empty", false, 1),
                ]
            )),
            None
        );
    }

    fn loss(code: &'static str, subject: Subject, detail: &str) -> loss::Finding {
        loss::Finding {
            code,
            subject,
            detail: detail.to_owned(),
        }
    }

    /// A loss's code, subject and detail, and the code and context after the model's name
    /// that `reported` gives it.
    type ReportedCase = (
        &'static str,
        Subject,
        &'static str,
        Code,
        &'static [(&'static str, &'static str)],
    );

    #[test]
    fn every_documented_loss_code_is_reported() {
        // Each code `model_convert::loss` documents, with a subject and a detail of the kind
        // its importers and exporters give it, and the finding the member is told.
        let cases: [ReportedCase; 17] = [
            (
                "bone_matrix_unknown",
                Subject::Bone(4),
                "my_bone",
                Code::BoneMatrixUnknown,
                &[("bone", "4"), ("name", "my_bone")],
            ),
            (
                "bone_slot_dropped",
                Subject::Mesh(2),
                "3",
                Code::BoneSlotDropped,
                &[("mesh", "2"), ("count", "3")],
            ),
            (
                "material_family_approximated",
                Subject::Material(1),
                "shirt",
                Code::MaterialFamilyApproximated,
                &[("material", "1"), ("name", "shirt")],
            ),
            (
                "material_split_by_flags",
                Subject::Material(0),
                "kit_1",
                Code::MaterialSplitByFlags,
                &[("material", "0"), ("name", "kit_1")],
            ),
            (
                "material_texture_unused",
                Subject::Material(2),
                "Tex_Sampler",
                Code::MaterialTextureUnused,
                &[("material", "2"), ("texture", "Tex_Sampler")],
            ),
            (
                "material_parameter_dropped",
                Subject::Material(0),
                "SpecularColor",
                Code::MaterialParameterDropped,
                &[("material", "0"), ("parameter", "SpecularColor")],
            ),
            (
                "sampler_settings_defaulted",
                Subject::Material(1),
                "DiffuseMap",
                Code::SamplerSettingsDefaulted,
                &[("material", "1"), ("sampler", "DiffuseMap")],
            ),
            (
                "vertex_bitangents_dropped",
                Subject::Mesh(0),
                "",
                Code::VertexBitangentsDropped,
                &[("mesh", "0")],
            ),
            (
                "dummy_texture_added",
                Subject::Material(0),
                "NormalMap_Tex_NRM",
                Code::DummyTextureAdded,
                &[("material", "0"), ("sampler", "NormalMap_Tex_NRM")],
            ),
            (
                "native_field_dropped",
                Subject::Model,
                "bone_matrices",
                Code::NativeFieldDropped,
                &[("field", "bone_matrices")],
            ),
            (
                "native_field_dropped",
                Subject::Material(1),
                "no_shadow_cast",
                Code::MeshFlagsDropped,
                &[("material", "1"), ("field", "no_shadow_cast")],
            ),
            // A PES 2017 model folded for PES 2015, as `model_convert`'s retargeting reports
            // it: the detail names the bone, in the index's place.
            (
                "bone_folded_for_version",
                Subject::Bone(3),
                "dsk_deltoid_l -> dsk_upperarm_l",
                Code::BoneFoldedForVersion,
                &[("bone", "dsk_deltoid_l -> dsk_upperarm_l")],
            ),
            (
                "bone_folded_by_position",
                Subject::Bone(9),
                "dsk_back -> sk_chest",
                Code::BoneFoldedByPosition,
                &[("bone", "dsk_back -> sk_chest")],
            ),
            (
                "skeleton_retargeted",
                Subject::Model,
                "7",
                Code::SkeletonRetargeted,
                &[("bones", "7")],
            ),
            (
                "static_bone_added",
                Subject::Mesh(1),
                "",
                Code::StaticBoneAdded,
                &[("mesh", "1")],
            ),
            (
                "weight_clamped",
                Subject::Mesh(0),
                "12",
                Code::WeightClamped,
                &[("mesh", "0"), ("count", "12")],
            ),
            (
                "empty_mesh_bone_group_dropped",
                Subject::Mesh(3),
                "175",
                Code::EmptyMeshBoneGroupDropped,
                &[("mesh", "3"), ("count", "175")],
            ),
        ];
        for (code, subject, detail, expected, context) in cases {
            let mut expected_context = vec![("model", "boots.fmdl".to_owned())];
            expected_context.extend(
                context
                    .iter()
                    .map(|(key, value)| (*key, (*value).to_owned())),
            );
            assert_eq!(
                reported("boots.fmdl", &loss(code, subject, detail)),
                Some((expected, expected_context)),
                "{code} {detail}"
            );
        }
        // A `native_field_dropped` on a bone.
        assert_eq!(
            reported(
                "fcl_hair.fmdl",
                &loss("native_field_dropped", Subject::Bone(8), "skl_parent")
            ),
            Some((
                Code::NativeFieldDropped,
                vec![
                    ("model", "fcl_hair.fmdl".to_owned()),
                    ("bone", "8".to_owned()),
                    ("field", "skl_parent".to_owned()),
                ]
            ))
        );
        // A code `loss.rs` does not document is no finding.
        assert_eq!(
            reported("boots.fmdl", &loss("no_such_loss", Subject::Model, "")),
            None
        );
    }
}
