//! The deep pass's model checks (`spec/team_compiler/messages.md` "Model checks"): which rules
//! the format crates' checks fire on a native model (`.fmdl`, `.model`) or a `.mtl`, one
//! entry per code, whether a model (`.fmdl` or `.model`) carries hand weights, and which
//! directories below the game's Fox common directory an FMDL's texture table names.

use std::collections::BTreeSet;

use aesthetics_export::legacy_kit_tokens;
use fmdl::{FmdlFile, Model};
use model_convert::formats::fmdl::every_mesh_hidden;
use model_convert::materials::{MaterialFamily, from_fox_shader};
use model_convert::ops::hand_split::{fox_has_hand_weights, prefox_has_hand_weights};
use pes_model::format::PreFoxModel;
use pes_model::format::mtl::{Material, MaterialEntry, MaterialSet};

/// The format crates' far-vertex codes, both reported as `vertex_too_far_from_origin`.
pub(crate) const FAR_VERTEX_CODES: [&str; 2] = [
    "fmdl_vertex_far_from_origin",
    "model_vertex_far_from_origin",
];

/// Which format crate checks a model or material set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelKind {
    /// A Fox model, checked by `fmdl`.
    Fmdl,
    /// A pre-Fox model, checked by `pes_model`.
    PreFoxModel,
    /// A pre-Fox material set, checked by `pes_model`.
    Mtl,
}

/// One rule a format crate's check fired on a file, or on a converted model in its target form
/// (`processing::conversion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Fired {
    /// The format crate's code.
    pub(crate) code: &'static str,
    /// Whether the format crate rates it an Error, which drops what holds the file.
    pub(crate) error: bool,
    /// How many items tripped the rule.
    pub(crate) count: usize,
}

impl Fired {
    /// `fmdl`'s finding `found` as a rule fired: its code, whether it is an Error, its count.
    pub(crate) fn fox(found: fmdl::check::Finding) -> Fired {
        let error = match found.severity {
            fmdl::check::Severity::Error => true,
            fmdl::check::Severity::Warning | fmdl::check::Severity::Info => false,
        };
        Fired {
            code: found.code,
            error,
            count: found.count,
        }
    }

    /// `pes_model`'s finding `found` as a rule fired: its code, whether it is an Error, its
    /// count.
    pub(crate) fn pre_fox(found: pes_model::check::Finding) -> Fired {
        let error = match found.severity {
            pes_model::check::Severity::Error => true,
            pes_model::check::Severity::Warning | pes_model::check::Severity::Info => false,
        };
        Fired {
            code: found.code,
            error,
            count: found.count,
        }
    }
}

/// What the deep pass learns from one parsed model or material set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelRead {
    /// The rules the format crate's check fired on it.
    pub(super) fired: Vec<Fired>,
    /// A model whose vertices carry a positive weight on a hand-skeleton bone (`skh_*_l`,
    /// `skh_*_r`), an FMDL or a pre-Fox `.model`, each read by its own format's check: on Fox
    /// planning gives a player folder holding it as a face part a gloves task, and the face and
    /// gloves tasks split it; on pre-Fox the face task splits it into two more `face.xml`
    /// entries. Always `false` for a material set.
    pub(super) hand_weighted: bool,
    /// An FMDL with a material the converter reads as the `metal` family
    /// (`model_convert::materials::from_fox_shader`): converted for PES 15-17 it is a
    /// `Basic_CNSR` material, which needs an environment map (`ContentPass::metal_models`).
    /// Always `false` for a pre-Fox model or a material set.
    pub(super) metal: bool,
    /// An FMDL every mesh of which is hidden (`invisible`), so that its PES 15-17 conversion
    /// leaves nothing of it: the `.model` export's own rule
    /// (`model_convert::formats::fmdl::every_mesh_hidden`), an FMDL with no mesh not counting.
    /// On a PES 15-17 run the deep pass drops such a file (`model_hidden_dropped`). Always
    /// `false` for a pre-Fox model or a material set.
    pub(super) hidden: bool,
    /// Its materials, in order: for a pre-Fox model the names it lists (`Model::materials`, as
    /// `pes_model::check::check_bundle` compares them, a name no mesh uses included), for a
    /// material set the ones it defines with their texture paths; empty for an FMDL, whose
    /// materials are not paired with a material set. The deep pass compares a model's names
    /// with its `.mtl`'s (`model_material_undefined`) and looks for the textures of the ones
    /// its meshes bind (`mtl_texture_not_found`).
    pub(super) materials: Vec<MaterialRead>,
    /// The texture directories an FMDL's table names that `fmdl_common_path_invalid`
    /// refuses: below the game's Fox common directory (`paths::FOX_COMMON_DIRECTORY`)
    /// whose folder is not `team_folder` or `sourceimages`, or written short with
    /// anything after `common_folder/` or starting `common/`, one per distinct
    /// directory (folded) in table order. Every entry is checked, a `dummy_` stem's
    /// included. Always empty for a pre-Fox model or a material set.
    pub(super) common_directories: Vec<CommonDirectory>,
    /// The texture paths an FMDL's table names by the kit stand-in's old spelling, the
    /// game's, which `dummy_kit_name_old` refuses (`paths::is_old_kit_stand_in`):
    /// directory and file name as written, one per distinct texture (folded) in table
    /// order. Every entry is checked, a mesh-unused one's included. Always empty for a
    /// pre-Fox model or a material set.
    pub(super) old_kit_names: Vec<String>,
    /// The texture paths an FMDL's table names into a folder of the export
    /// (`names_export_directory`) holding a legacy kit token, which `kit_token_old` refuses
    /// (`aesthetics_export::legacy_kit_tokens`): directory and file name as written, one per
    /// distinct texture (folded) in table order. Every such entry is checked, a mesh-unused
    /// one's included, but one naming a texture the game supplies (`paths::game_supplied`),
    /// whose path the Export upgrader keeps as written. Always empty for a pre-Fox model or a
    /// material set.
    pub(super) legacy_kit_paths: Vec<String>,
}

/// One texture directory a parsed FMDL names that `fmdl_common_path_invalid` refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommonDirectory {
    /// The texture path the finding names, directory and file name as written (the first
    /// entry spelling `directory`).
    pub(super) texture: String,
    /// The directory as written.
    pub(super) directory: String,
}

/// One material a parsed pre-Fox file names (`ModelRead::materials`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MaterialRead {
    /// Its name: the one a model binds, or the one a material set defines.
    pub(super) name: String,
    /// The texture paths its samplers name, in order, as written; empty for a model.
    pub(super) paths: Vec<String>,
    /// For a model's, whether one of its meshes binds it: a name no mesh binds
    /// (`model_material_unused`) is never drawn, so its textures are never loaded. `false` for
    /// a material set's.
    pub(super) mesh_used: bool,
}

/// What `bytes`, read as `kind`, tells the deep pass (`ModelRead`), or the reader's error text
/// when they do not parse. The model is parsed once for both questions.
pub(super) fn fired(kind: ModelKind, bytes: &[u8]) -> Result<ModelRead, String> {
    let read = match kind {
        ModelKind::Fmdl => {
            let file = FmdlFile::read(bytes).map_err(|error| error.to_string())?;
            let model = Model::from_file(&file).map_err(|error| error.to_string())?;
            let table = texture_table(&file)?;
            ModelRead {
                fired: fmdl::check::check(&model)
                    .into_iter()
                    .map(Fired::fox)
                    .collect(),
                hand_weighted: fox_has_hand_weights(&model),
                // The converter's own family rule, so the deep pass and the conversion agree
                // on which material is metal (a shader naming both `glass` and `ggx` is glass).
                metal: model.materials.iter().any(|material| {
                    from_fox_shader(&material.shader).family == MaterialFamily::Metal
                }),
                hidden: every_mesh_hidden(&model),
                materials: Vec::new(),
                common_directories: table.common_directories,
                old_kit_names: table.old_kit_names,
                legacy_kit_paths: table.legacy_kit_paths,
            }
        }
        ModelKind::PreFoxModel => {
            let model = PreFoxModel::read(bytes)
                .and_then(|file| pes_model::model::Model::from_file(&file))
                .map_err(|error| error.to_string())?;
            let materials = model
                .materials
                .iter()
                .enumerate()
                .map(|(index, name)| MaterialRead {
                    name: name.clone(),
                    paths: Vec::new(),
                    mesh_used: model.meshes.iter().any(|mesh| mesh.material == index),
                })
                .collect();
            ModelRead {
                fired: pes_model::check::check(&model)
                    .into_iter()
                    .map(Fired::pre_fox)
                    .collect(),
                hand_weighted: prefox_has_hand_weights(&model),
                metal: false,
                hidden: false,
                materials,
                common_directories: Vec::new(),
                old_kit_names: Vec::new(),
                legacy_kit_paths: Vec::new(),
            }
        }
        ModelKind::Mtl => {
            let set = MaterialSet::read(bytes).map_err(|error| error.to_string())?;
            ModelRead {
                fired: pes_model::check::check_materials(&set)
                    .into_iter()
                    .map(Fired::pre_fox)
                    .collect(),
                hand_weighted: false,
                metal: false,
                hidden: false,
                materials: set.materials.into_iter().map(material_read).collect(),
                common_directories: Vec::new(),
                old_kit_names: Vec::new(),
                legacy_kit_paths: Vec::new(),
            }
        }
    };
    Ok(read)
}

/// The texture-table entries of `file`, a parsed FMDL, the deep pass refuses, in one
/// pass over the table: the directories `fmdl_common_path_invalid` refuses, one per
/// distinct directory (folded) in table order, each `texture` the first entry spelling
/// it, and the texture paths naming the kit stand-in's old spelling, which
/// `dummy_kit_name_old` refuses (`paths::is_old_kit_stand_in`), one per distinct
/// texture (folded) in table order, the path as written, and the texture paths holding a
/// legacy kit token in a folder of the export (`names_export_directory`), which
/// `kit_token_old` refuses, the same way, but for a texture the game supplies
/// (`paths::game_supplied`), whose path the Export upgrader keeps as written. Every entry
/// of the texture table is checked by the other two rules, a `dummy_` stem's and a
/// mesh-unused one's included, and by this one but for those textures: the spelling is the
/// export's, so the rules cover them all. The `.mtl`'s two short
/// forms are read as their long forms first
/// (`paths::export_fmdl_directory`), so `team_common_folder/<sub>` and
/// `common_folder/` are admitted as `team_folder` and `sourceimages`; refused are a
/// folder below the game's Fox common directory (`paths::FOX_COMMON_DIRECTORY`)
/// other than `team_folder` or `sourceimages` (each matched in any letter case), a
/// short directory with anything after `common_folder/` (`common_folder/jessie/`),
/// and one starting `common/`, a CPK's root folder, each as the `.mtl`'s
/// `user_face_xml::reference` reads them.
fn texture_table(file: &FmdlFile) -> Result<TextureTable, String> {
    let mut seen_directories = BTreeSet::new();
    let mut seen_textures = BTreeSet::new();
    let mut seen_legacy = BTreeSet::new();
    let mut table = TextureTable::default();
    for path in fmdl::ops::paths::texture_paths(file).map_err(|error| error.to_string())? {
        let invalid = match path
            .directory
            .strip_prefix(crate::user_face_xml::COMMON_FOLDER)
        {
            // `common_folder/` alone is the game's own, admitted by the folder check
            // below; anything after it is refused.
            Some(rest) if !rest.is_empty() => true,
            _ => {
                path.directory.starts_with("common/")
                    || fox_common_folder_invalid(&crate::paths::export_fmdl_directory(
                        &path.directory,
                    ))
            }
        };
        let texture = format!("{}{}", path.directory, path.file_name);
        let in_export = names_export_directory(&path.directory);
        if invalid && seen_directories.insert(vtree::fold_name(&path.directory)) {
            table.common_directories.push(CommonDirectory {
                texture: texture.clone(),
                directory: path.directory,
            });
        }
        if in_export
            && !crate::paths::game_supplied(&path.file_name)
            && !legacy_kit_tokens(&texture).is_empty()
            && seen_legacy.insert(vtree::fold_name(&texture))
        {
            table.legacy_kit_paths.push(texture.clone());
        }
        if crate::paths::is_old_kit_stand_in(&path.file_name)
            && seen_textures.insert(vtree::fold_name(&texture))
        {
            table.old_kit_names.push(texture);
        }
    }
    Ok(table)
}

/// What `texture_table` finds in an FMDL's texture table: `ModelRead`'s fields of the same
/// names.
#[derive(Debug, Default)]
struct TextureTable {
    common_directories: Vec<CommonDirectory>,
    old_kit_names: Vec<String>,
    legacy_kit_paths: Vec<String>,
}

/// Whether `directory`, an FMDL texture path's directory as written, names a folder of the
/// export: a relative one (the model's own folder) or, read as its long form
/// (`paths::export_fmdl_directory`), the team's Common (`team_folder` below the game's Fox
/// common directory, compared ASCII case-insensitively). Any other directory (the game's kit
/// texture folder, `common_folder/`'s `sourceimages/`, any other `/Assets/` one) is the
/// game's, which names its kit textures `u0???pN`, so `kit_token_old` checks only these.
fn names_export_directory(directory: &str) -> bool {
    let long = crate::paths::export_fmdl_directory(directory);
    let team_common = format!(
        "{}{}/",
        crate::paths::FOX_COMMON_DIRECTORY,
        crate::user_face_xml::TEAM_FOLDER
    );
    !long.starts_with('/')
        || long
            .get(..team_common.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(&team_common))
}

/// Whether `directory`, an FMDL texture path's directory with the short forms already
/// read as their long forms, names a folder below the game's Fox common directory
/// (`paths::FOX_COMMON_DIRECTORY`, matched in any letter case) other than
/// `team_folder` or `sourceimages` (each matched in any letter case).
fn fox_common_folder_invalid(directory: &str) -> bool {
    let Some(folder) = directory
        .get(..crate::paths::FOX_COMMON_DIRECTORY.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(crate::paths::FOX_COMMON_DIRECTORY))
        .map(|_| {
            directory[crate::paths::FOX_COMMON_DIRECTORY.len()..]
                .split('/')
                .next()
                .unwrap_or("")
        })
    else {
        return false;
    };
    !(folder.is_empty()
        || folder.eq_ignore_ascii_case(crate::user_face_xml::TEAM_FOLDER)
        || folder.eq_ignore_ascii_case("sourceimages"))
}

/// `material` of a material set as the deep pass keeps it: its name and every sampler's
/// texture path, a path two samplers name included.
fn material_read(material: Material) -> MaterialRead {
    let paths = material
        .entries
        .into_iter()
        .filter_map(|entry| match entry {
            MaterialEntry::Sampler(sampler) => Some(sampler.path),
            MaterialEntry::State(_) | MaterialEntry::Vector(_) => None,
        })
        .collect();
    MaterialRead {
        name: material.name,
        paths,
        mesh_used: false,
    }
}

/// `fired` with one entry per code, in the order each code first fired, its counts summed:
/// the checks report per mesh or per material, a member reads one line per file.
pub(crate) fn summed(fired: Vec<Fired>) -> Vec<Fired> {
    let mut summed: Vec<Fired> = Vec::new();
    for rule in fired {
        match summed.iter_mut().find(|known| known.code == rule.code) {
            Some(known) => known.count += rule.count,
            None => summed.push(rule),
        }
    }
    summed
}

#[cfg(test)]
mod tests {
    use aesthetics_export::{ContentFinding, Disposition, IssueScope};
    use pes_version::PesVersion;

    use super::*;
    use crate::deep::tests::{
        bc1_dds, counted, edited, far_boots, findings_for, findings_of, fixture, folder,
        glove_over_the_face_limit, mtl_texture, path, pre_fox_fixture, tracer_boots, tracer_file,
    };
    use crate::testing::scratch;

    /// `code` on the folder `scope`: the file `file` does not parse, for `error`.
    fn broken(code: &'static str, scope: &str, file: &str, error: String) -> ContentFinding {
        ContentFinding {
            code,
            scope: folder(scope),
            context: vec![("file", file.to_owned()), ("error", error)],
            disposition: Disposition::DropFolder,
            pass_through_eligible: false,
        }
    }

    #[test]
    fn far_vertices_in_two_meshes_are_one_finding_with_their_summed_count() {
        let temp = scratch("deep_two_meshes");
        let boots = edited(&far_boots(), |model| {
            model.meshes[1].vertices.positions[0] = [0.0, 0.0, 7000.0];
        });
        let findings = findings_of(temp.path(), &[("Players/03 - A/boots.fmdl", boots)], &[]);
        let a = folder("Players/03 - A");
        assert_eq!(
            findings,
            [
                counted(
                    "vertex_too_far_from_origin",
                    &a,
                    "boots.fmdl",
                    2,
                    Disposition::DropFolder,
                    false
                ),
                counted(
                    "fmdl_weights_not_normalized",
                    &a,
                    "boots.fmdl",
                    1662,
                    Disposition::Keep,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_format_error_keeps_its_code_drops_the_folder_and_may_pass_through() {
        let temp = scratch("deep_format_error");
        let findings = findings_of(
            temp.path(),
            &[("Players/03 - A/glove_r.fmdl", glove_over_the_face_limit())],
            &[],
        );
        assert_eq!(
            findings,
            [counted(
                "fmdl_mesh_over_face_limit",
                &folder("Players/03 - A"),
                "glove_r.fmdl",
                21_846,
                Disposition::DropFolder,
                true
            )]
        );
    }

    #[test]
    fn the_tracer_s_boots_keep_their_folder_and_a_model_that_does_not_parse_drops_its_own() {
        let temp = scratch("deep_clean");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/boots.fmdl", tracer_boots()),
                ("Players/05 - B/boots.fmdl", b"not a model".to_vec()),
            ],
            &[],
        );
        let error = FmdlFile::read(b"not a model").unwrap_err().to_string();
        assert_eq!(
            findings,
            [
                counted(
                    "fmdl_weights_not_normalized",
                    &folder("Players/03 - A"),
                    "boots.fmdl",
                    1662,
                    Disposition::Keep,
                    false
                ),
                broken("model_broken", "Players/05 - B", "boots.fmdl", error),
            ]
        );
    }

    #[test]
    fn pre_fox_models_and_material_sets_are_checked() {
        let temp = scratch("deep_pre_fox");
        let findings = findings_of(
            temp.path(),
            &[
                (
                    "Players/03 - A/face_high.model",
                    pre_fox_fixture("konami_card.model"),
                ),
                (
                    "Players/03 - A/materials.mtl",
                    pre_fox_fixture("cardhead_materials.mtl"),
                ),
                (
                    "Players/05 - B/boots.model",
                    pre_fox_fixture("community_empty_geometry_boots.wesys.model"),
                ),
                (
                    "Players/05 - B/boots.mtl",
                    pre_fox_fixture("konami_shadow.mtl"),
                ),
            ],
            &[],
        );
        let b = folder("Players/05 - B");
        // The boots' two meshes are empty; the shadow's one material names no state. On PES
        // 21 each `.model`, no FMDL beside it, is converted with the `.mtl` its search finds,
        // which here defines none of its materials, and whose texture paths are looked up: the
        // card head's `./texture.dds`, of a material no mesh binds, is missing.
        assert_eq!(
            findings,
            [
                undefined(
                    "Players/03 - A",
                    "face_high.model",
                    "materials.mtl",
                    "judge_card_red"
                ),
                mtl_texture(
                    "mtl_texture_unused_missing",
                    folder("Players/03 - A"),
                    "materials.mtl",
                    "./texture.dds",
                    "card"
                ),
                counted(
                    "model_mesh_empty",
                    &b,
                    "boots.model",
                    2,
                    Disposition::Keep,
                    false
                ),
                undefined(
                    "Players/05 - B",
                    "boots.model",
                    "boots.mtl",
                    "Boots_Game_mat, Boots_Game_Alpha_mat"
                ),
                counted(
                    "mtl_state_missing",
                    &b,
                    "boots.mtl",
                    7,
                    Disposition::Keep,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_pre_fox_model_s_mtl_may_be_reached_through_a_link_and_a_model_link_s_in_common() {
        let temp = scratch("deep_pre_fox_links");
        // The card head's model and its material set, which defines the model's one material
        // and names `./texture.dds`.
        let card = || pre_fox_fixture("cardhead_face_high.model");
        let materials = || pre_fox_fixture("cardhead_materials.mtl");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                // A model whose one `.mtl` is a link, its target present.
                ("Players/03 - A/hat.model", card()),
                ("Players/03 - A/hat.mtl.common", Vec::new()),
                ("Common/hat.mtl", materials()),
                // A model link whose `.mtl` is the Common one of its name.
                ("Players/05 - B/legs.model.common", Vec::new()),
                ("Common/legs.model", card()),
                ("Common/legs.mtl", materials()),
                ("Common/texture.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(findings, []);
        // A model link with no `.mtl` anywhere, `Common/` holding none.
        let temp = scratch("deep_pre_fox_link_undefined");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - B/legs.model.common", Vec::new()),
                ("Common/legs.model", card()),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [ContentFinding {
                code: "model_material_undefined",
                scope: folder("Players/05 - B"),
                context: vec![("file", "legs.model.common".to_owned())],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }]
        );
    }

    #[test]
    fn under_ingame_face_a_model_link_with_no_mtl_anywhere_is_undefined() {
        // The link is a part of his boots, whose `boots.mtl` needs the Common model's `.mtl`.
        let temp = scratch("deep_pre_fox_marked_link_undefined");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - B/ingame_face.toml", Vec::new()),
                ("Players/05 - B/kit_boots.model.common", Vec::new()),
                (
                    "Common/kit_boots.model",
                    pre_fox_fixture("cardhead_face_high.model"),
                ),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [ContentFinding {
                code: "model_material_undefined",
                scope: folder("Players/05 - B"),
                context: vec![("file", "kit_boots.model.common".to_owned())],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }]
        );
    }

    #[test]
    fn a_link_loading_a_common_fmdl_pairs_no_mtl_and_one_loading_a_model_does() {
        let temp = scratch("deep_pre_fox_fmdl_link");
        let card = || pre_fox_fixture("cardhead_face_high.model");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                // A Common FMDL, whose conversion writes its material set: no `.mtl` needed.
                ("Players/05 - B/legs.fmdl.common", Vec::new()),
                ("Common/legs.fmdl", tracer_file("glove_r.fmdl")),
                // A Common `.model`, with no `.mtl` anywhere.
                ("Players/06 - C/hat.model.common", Vec::new()),
                ("Common/hat.model", card()),
                // An FMDL a `.model` of its stem beats: its link loads the `.model`.
                ("Players/07 - D/arm.fmdl.common", Vec::new()),
                ("Common/arm.fmdl", tracer_file("glove_r.fmdl")),
                ("Common/arm.model", card()),
            ],
            &[],
            &[],
        );
        let undefined: Vec<&ContentFinding> = findings
            .iter()
            .filter(|finding| finding.code == "model_material_undefined")
            .collect();
        assert_eq!(
            undefined,
            [
                &ContentFinding {
                    code: "model_material_undefined",
                    scope: folder("Players/06 - C"),
                    context: vec![("file", "hat.model.common".to_owned())],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
                &ContentFinding {
                    code: "model_material_undefined",
                    scope: folder("Players/07 - D"),
                    context: vec![("file", "arm.fmdl.common".to_owned())],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
            ],
            "{findings:#?}"
        );
    }

    #[test]
    fn on_fox_a_model_link_pairs_the_mtl_its_search_finds_unless_an_fmdl_of_its_stem_beats_it() {
        let card = || pre_fox_fixture("cardhead_face_high.model");
        // The findings of `codes` among the PES 21 pass's over `files`.
        let found = |name: &str, files: &[(&str, Vec<u8>)], codes: &[&str]| {
            let temp = scratch(name);
            findings_for(PesVersion::Pes21, temp.path(), files, &[], &[])
                .into_iter()
                .filter(|finding| codes.contains(&finding.code))
                .collect::<Vec<ContentFinding>>()
        };
        // A Common `.model` with no `.mtl` anywhere; one an FMDL of its stem beats loads the
        // FMDL, which carries its materials.
        let codes = ["model_material_undefined", "mtl_texture_not_found"];
        assert_eq!(
            found(
                "deep_fox_model_link_undefined",
                &[
                    ("Players/05 - B/hat.model.common", Vec::new()),
                    ("Common/hat.model", card()),
                    ("Players/06 - C/arm.model.common", Vec::new()),
                    ("Common/arm.model", card()),
                    ("Common/arm.fmdl", tracer_file("glove_r.fmdl")),
                ],
                &codes,
            ),
            [ContentFinding {
                code: "model_material_undefined",
                scope: folder("Players/05 - B"),
                context: vec![("file", "hat.model.common".to_owned())],
                disposition: Disposition::DropFolder,
                pass_through_eligible: false,
            }]
        );
        // The Common `.mtl` its search finds is read with it, its texture looked up on the
        // linking player's folder, where the conversion reads it: its `./texture.dds` is in no
        // folder `Common/`'s textures supply.
        assert_eq!(
            found(
                "deep_fox_model_link_texture",
                &[
                    ("Players/05 - B/legs.model.common", Vec::new()),
                    ("Common/legs.model", card()),
                    ("Common/legs.mtl", pre_fox_fixture("cardhead_materials.mtl")),
                ],
                &codes,
            ),
            [ContentFinding {
                code: "mtl_texture_not_found",
                scope: folder("Players/05 - B"),
                context: vec![
                    ("file", "Common/legs.mtl".to_owned()),
                    ("texture", "./texture.dds".to_owned()),
                    ("materials", "card".to_owned()),
                ],
                disposition: Disposition::Keep,
                pass_through_eligible: false,
            }]
        );
    }

    #[test]
    fn a_model_link_s_search_sees_only_the_common_files_the_common_pass_kept() {
        let temp = scratch("deep_pre_fox_link_broken_mtl");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - B/legs.model.common", Vec::new()),
                ("Common/legs.model", pre_fox_fixture("konami_card.model")),
                ("Common/legs.mtl", b"not a material set".to_vec()),
            ],
            &[],
            &[],
        );
        let error = MaterialSet::read(b"not a material set")
            .unwrap_err()
            .to_string();
        assert_eq!(
            findings,
            [
                ContentFinding {
                    code: "model_material_undefined",
                    scope: folder("Players/05 - B"),
                    context: vec![("file", "legs.model.common".to_owned())],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
                ContentFinding {
                    code: "mtl_broken",
                    scope: IssueScope::File(path("Common/legs.mtl")),
                    context: vec![("file", "legs.mtl".to_owned()), ("error", error)],
                    disposition: Disposition::DropFile,
                    pass_through_eligible: false,
                },
            ]
        );
    }

    /// `model_material_undefined` on the folder `scope`: the model `file` uses `materials`,
    /// which the `.mtl` it is paired with, `mtl`, does not define.
    fn undefined(scope: &str, file: &str, mtl: &str, materials: &str) -> ContentFinding {
        ContentFinding {
            code: "model_material_undefined",
            scope: folder(scope),
            context: vec![
                ("file", file.to_owned()),
                ("mtl", mtl.to_owned()),
                ("materials", materials.to_owned()),
            ],
            disposition: Disposition::DropFolder,
            pass_through_eligible: true,
        }
    }

    #[test]
    fn a_pre_fox_model_material_its_mtl_lacks_is_undefined_and_may_pass_through() {
        // Konami's referee card binds `judge_card_red`; the card head's material set defines
        // only `card`, which the card head's model binds.
        let temp = scratch("deep_pre_fox_named_undefined");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                (
                    "Players/03 - A/face_high.model",
                    pre_fox_fixture("konami_card.model"),
                ),
                (
                    "Players/03 - A/face_high.mtl",
                    pre_fox_fixture("cardhead_materials.mtl"),
                ),
                ("Players/03 - A/texture.dds", bc1_dds(4, 4)),
                (
                    "Players/05 - B/face_high.model",
                    pre_fox_fixture("cardhead_face_high.model"),
                ),
                (
                    "Players/05 - B/face_high.mtl",
                    pre_fox_fixture("cardhead_materials.mtl"),
                ),
                ("Players/05 - B/texture.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [undefined(
                "Players/03 - A",
                "face_high.model",
                "face_high.mtl",
                "judge_card_red"
            )]
        );
    }

    #[test]
    fn on_pes_21_a_model_no_fmdl_beats_is_paired_and_a_beaten_one_and_its_mtl_read_by_nothing() {
        let temp = scratch("deep_fox_pairings");
        let findings = findings_of(
            temp.path(),
            &[
                // Selected, its `.mtl` defining only the card head's `card`.
                (
                    "Players/03 - A/face_high.model",
                    pre_fox_fixture("konami_card.model"),
                ),
                (
                    "Players/03 - A/face_high.mtl",
                    pre_fox_fixture("cardhead_materials.mtl"),
                ),
                // Selected, with no `.mtl` for its search to find.
                (
                    "Players/05 - B/face_high.model",
                    pre_fox_fixture("konami_card.model"),
                ),
                // Beaten by the FMDL of its stem (the tracer's right glove, which `fmdl`'s
                // check finds nothing in): neither it nor the `.mtl` beside it, neither of which
                // even parses, is read, compared or reported.
                ("Players/07 - C/face_high.fmdl", tracer_file("glove_r.fmdl")),
                ("Players/07 - C/face_high.model", b"not a model".to_vec()),
                (
                    "Players/07 - C/face_high.mtl",
                    b"not a material set".to_vec(),
                ),
            ],
            &[],
        );
        // The selected model's `.mtl` has its texture paths looked up: the card head's
        // `./texture.dds`, of a material no mesh binds, is missing.
        assert_eq!(
            findings,
            [
                undefined(
                    "Players/03 - A",
                    "face_high.model",
                    "face_high.mtl",
                    "judge_card_red"
                ),
                mtl_texture(
                    "mtl_texture_unused_missing",
                    folder("Players/03 - A"),
                    "face_high.mtl",
                    "./texture.dds",
                    "card"
                ),
                ContentFinding {
                    code: "model_material_undefined",
                    scope: folder("Players/05 - B"),
                    context: vec![("file", "face_high.model".to_owned())],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
            ]
        );
    }

    #[test]
    fn a_model_link_s_common_model_is_compared_with_the_mtl_its_search_finds() {
        let temp = scratch("deep_pre_fox_link_named_undefined");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                // The folder's own `.mtl` of the link's name, which the card's material is
                // not in.
                ("Players/05 - B/legs.model.common", Vec::new()),
                (
                    "Players/05 - B/legs.mtl",
                    pre_fox_fixture("cardhead_materials.mtl"),
                ),
                ("Players/05 - B/texture.dds", bc1_dds(4, 4)),
                ("Common/legs.model", pre_fox_fixture("konami_card.model")),
                // A `Common/` `.mtl`, named by its export path.
                ("Players/07 - C/hat.model.common", Vec::new()),
                ("Common/hat.model", pre_fox_fixture("konami_card.model")),
                ("Common/hat.mtl", pre_fox_fixture("cardhead_materials.mtl")),
                ("Common/texture.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [
                undefined(
                    "Players/05 - B",
                    "legs.model.common",
                    "legs.mtl",
                    "judge_card_red"
                ),
                undefined(
                    "Players/07 - C",
                    "hat.model.common",
                    "Common/hat.mtl",
                    "judge_card_red"
                ),
            ]
        );
    }

    #[test]
    fn a_pre_fox_model_weighted_to_hand_bones_reads_hand_weighted() {
        let body = fired(ModelKind::PreFoxModel, &fixture("hand_split/body.model")).unwrap();
        assert!(body.hand_weighted);
        let card = fired(
            ModelKind::PreFoxModel,
            &pre_fox_fixture("cardhead_face_high.model"),
        )
        .unwrap();
        assert!(!card.hand_weighted);
    }

    /// A blank head's oral model, an FMDL whose one mesh is flagged invisible.
    fn hidden_oral() -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../libs/model_convert/tests/fixtures/addon_oral.fmdl"),
        )
        .unwrap()
    }

    #[test]
    fn an_fmdl_whose_every_mesh_is_hidden_reads_hidden() {
        assert!(fired(ModelKind::Fmdl, &hidden_oral()).unwrap().hidden);
        assert!(!fired(ModelKind::Fmdl, &tracer_boots()).unwrap().hidden);
    }

    /// `model_hidden_dropped` on the file at `file`, with `context`.
    fn hidden_dropped(file: &str, context: &[(&'static str, &str)]) -> ContentFinding {
        ContentFinding {
            code: "model_hidden_dropped",
            scope: IssueScope::File(path(file)),
            context: context
                .iter()
                .map(|(key, value)| (*key, (*value).to_owned()))
                .collect(),
            disposition: Disposition::DropFile,
            pass_through_eligible: false,
        }
    }

    // TC-MOD-69
    #[test]
    fn on_pes_17_an_fmdl_whose_every_mesh_is_hidden_is_dropped_with_a_link_naming_it() {
        let files = [
            ("Players/03 - A/oral.fmdl", hidden_oral()),
            ("Players/05 - B/oral.fmdl.common", Vec::new()),
            ("Common/oral.fmdl", hidden_oral()),
            ("Collars/collar_001.fmdl", hidden_oral()),
        ];
        let temp = scratch("deep_hidden_pes17");
        assert_eq!(
            findings_for(PesVersion::Pes17, temp.path(), &files, &[], &[]),
            [
                hidden_dropped("Players/03 - A/oral.fmdl", &[("file", "oral.fmdl")]),
                hidden_dropped(
                    "Players/05 - B/oral.fmdl.common",
                    &[("file", "oral.fmdl.common"), ("model", "Common/oral.fmdl")]
                ),
                hidden_dropped("Common/oral.fmdl", &[("file", "oral.fmdl")]),
                hidden_dropped("Collars/collar_001.fmdl", &[("file", "collar_001.fmdl")]),
            ]
        );
        // On Fox each is packed as it is, the game drawing nothing of it.
        let temp = scratch("deep_hidden_pes21");
        assert_eq!(
            findings_for(PesVersion::Pes21, temp.path(), &files, &[], &[]),
            []
        );
    }

    #[test]
    fn a_common_link_below_a_subfolder_is_dropped_with_the_hidden_target_at_its_path() {
        // `sub/oral.fmdl.common` links `Common/sub/oral.fmdl`, not the direct `Common/oral.fmdl`.
        let temp = scratch("deep_hidden_subfolder_link");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/05 - B/sub/oral.fmdl.common", Vec::new()),
                ("Common/oral.fmdl", hidden_oral()),
                ("Common/sub/oral.fmdl", hidden_oral()),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [
                hidden_dropped(
                    "Players/05 - B/sub/oral.fmdl.common",
                    &[
                        ("file", "sub/oral.fmdl.common"),
                        ("model", "Common/sub/oral.fmdl")
                    ]
                ),
                hidden_dropped("Common/oral.fmdl", &[("file", "oral.fmdl")]),
                hidden_dropped("Common/sub/oral.fmdl", &[("file", "oral.fmdl")]),
            ]
        );
    }

    #[test]
    fn a_pre_fox_model_s_far_vertex_is_vertex_too_far_from_origin() {
        let temp = scratch("deep_pre_fox_far");
        // Konami's referee card, clean, with its first vertex 6000 units from the origin.
        let file = PreFoxModel::read(&pre_fox_fixture("konami_card.model")).unwrap();
        let mut model = pes_model::model::Model::from_file(&file).unwrap();
        model.meshes[0].vertices.positions[0] = [6000.0, 0.0, 0.0];
        let card = model.to_file().unwrap().write().unwrap();
        // With the material set defining its material beside it, so the model's one finding is
        // its far vertex (the set names no state, its own finding).
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/card.model", card),
                (
                    "Players/03 - A/card.mtl",
                    pre_fox_fixture("konami_card_red.mtl"),
                ),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [
                counted(
                    "vertex_too_far_from_origin",
                    &folder("Players/03 - A"),
                    "card.model",
                    1,
                    Disposition::DropFolder,
                    false
                ),
                counted(
                    "mtl_state_missing",
                    &folder("Players/03 - A"),
                    "card.mtl",
                    7,
                    Disposition::Keep,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_pre_fox_error_keeps_its_code_drops_the_folder_and_may_pass_through() {
        let temp = scratch("deep_pre_fox_error");
        // The card head's material set, clean, with its one material listed twice, beside the
        // texture it names and no model: read for PES 17, where every `.mtl` is packed (on PES
        // 21 nothing reads a `.mtl` no `.model` is converted with).
        let mut set = MaterialSet::read(&pre_fox_fixture("cardhead_materials.mtl")).unwrap();
        set.materials.push(set.materials[0].clone());
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/03 - A/materials.mtl", set.write()),
                ("Players/03 - A/texture.dds", bc1_dds(4, 4)),
            ],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [counted(
                "mtl_material_duplicate",
                &folder("Players/03 - A"),
                "materials.mtl",
                1,
                Disposition::DropFolder,
                true
            )]
        );
    }

    #[test]
    fn a_pre_fox_model_or_material_set_that_does_not_parse_drops_its_folder() {
        let temp = scratch("deep_pre_fox_broken");
        // For PES 17, where the `.mtl` alone in its folder is read (on PES 21 nothing reads a
        // `.mtl` no `.model` is converted with).
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[
                ("Players/03 - A/boots.model", b"not a model".to_vec()),
                ("Players/05 - B/boots.mtl", b"not a material set".to_vec()),
            ],
            &[],
            &[],
        );
        let model_error = PreFoxModel::read(b"not a model").unwrap_err().to_string();
        let mtl_error = MaterialSet::read(b"not a material set")
            .unwrap_err()
            .to_string();
        // The model with no `.mtl` beside it has every material undefined too.
        assert_eq!(
            findings,
            [
                broken("model_broken", "Players/03 - A", "boots.model", model_error),
                ContentFinding {
                    code: "model_material_undefined",
                    scope: folder("Players/03 - A"),
                    context: vec![("file", "boots.model".to_owned())],
                    disposition: Disposition::DropFolder,
                    pass_through_eligible: false,
                },
                broken("mtl_broken", "Players/05 - B", "boots.mtl", mtl_error),
            ]
        );
    }
}
