//! The models the old exports hid by distance: a `.fmdl` or `.model`
//! whose every vertex sits more than 5000 units from the origin draws
//! nothing a camera sees, and the compiler refuses that geometry, so the
//! upgrade drops the file with `model_far_dropped`. The steps then see
//! the folders without it: a gloves or boots folder it leaves modelless
//! hides the hands or feet as any modelless folder does.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{FileKind, ModelFormat, classify};
use fmdl::FmdlFile;
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, INPUT, tool_message};
use crate::players::{file_name, path_scope};
use crate::source::SourceFailure;

/// The drop's input files and its `model_far_dropped` findings, in
/// input-path order.
pub(crate) struct FarDrops {
    /// The input paths dropped.
    pub(crate) dropped: BTreeSet<String>,
    /// `model_far_dropped` per dropped file (`vertices`, the model's
    /// vertex count).
    pub(crate) findings: Vec<Message>,
}

/// The far models of `files` — every `.fmdl` or `.model` in scope whose
/// every vertex sits past the far checks' 5000 units — with
/// `model_far_dropped` per dropped file. `any_folder` says every file
/// below a top folder is in scope (a referee export's root layout, a
/// model-folder input's); else the old format's own folders alone are. A
/// file that does not read or does not parse is kept, the compile's to
/// report.
pub(crate) fn far_drops(
    files: &[String],
    any_folder: bool,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> FarDrops {
    // Each far model's input path to its vertex count.
    let mut far: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        let Some(format) = model_kind(file).filter(|_| in_scope(file, any_folder)) else {
            continue;
        };
        let Ok(bytes) = read(file) else {
            continue;
        };
        if let Some(vertices) = far_vertices(format, &bytes) {
            far.insert(file.clone(), vertices);
        }
    }
    FarDrops {
        dropped: far.keys().cloned().collect(),
        findings: far
            .into_iter()
            .map(|(file, vertices)| {
                tool_message(
                    Code::ModelFarDropped,
                    Scope::File {
                        export_id: INPUT,
                        path: path_scope(&file),
                    },
                    Disposition::DropFile,
                    vec![("vertices", vertices.to_string())],
                )
            })
            .collect(),
    }
}

/// Whether `path` sits where a model migrates from: inside a `Faces/`,
/// `Boots/`, `Gloves/` or `Players/` subfolder, or below `Common/` or
/// `Collars/` at any depth — or, with `any_folder`, anywhere below the
/// root. A file directly at the root, or in `Faces/`'s, `Boots/`'s or
/// `Gloves/`'s own level, is a leftover, which step 10's fallback names
/// instead.
fn in_scope(path: &str, any_folder: bool) -> bool {
    let Some((top, rest)) = path.split_once('/') else {
        return false;
    };
    if any_folder {
        return true;
    }
    if !rest.contains('/') {
        return ["common", "collars"]
            .iter()
            .any(|folder| top.eq_ignore_ascii_case(folder));
    }
    ["faces", "boots", "gloves", "players", "common", "collars"]
        .iter()
        .any(|folder| top.eq_ignore_ascii_case(folder))
}

/// The file's model format when `path` names a `.fmdl` or `.model`.
fn model_kind(path: &str) -> Option<ModelFormat> {
    if let FileKind::Model(format @ (ModelFormat::Fmdl | ModelFormat::PesModel)) =
        classify(file_name(path))
    {
        Some(format)
    } else {
        None
    }
}

/// The model's vertex count when every vertex of `bytes` sits more than
/// 5000 units from the origin, as `fmdl::check` and `pes_model::check`
/// measure it (the squared distance in f64); `None` when one is nearer,
/// the model holds none, or the file does not read.
fn far_vertices(format: ModelFormat, bytes: &[u8]) -> Option<usize> {
    /// The squared-distance limit: a huge coordinate cannot overflow the
    /// f64 sum.
    const LIMIT: f64 = 5000.0 * 5000.0;
    fn all_far<'a>(positions: impl Iterator<Item = &'a [f32; 3]>) -> Option<usize> {
        let mut count = 0;
        for &[x, y, z] in positions {
            let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
            if x * x + y * y + z * z <= LIMIT {
                return None;
            }
            count += 1;
        }
        (count > 0).then_some(count)
    }
    match format {
        ModelFormat::Fmdl => {
            let model = fmdl::Model::from_file(&FmdlFile::read(bytes).ok()?).ok()?;
            all_far(
                model
                    .meshes
                    .iter()
                    .flat_map(|mesh| mesh.vertices.positions.iter()),
            )
        }
        ModelFormat::PesModel => {
            let model = pes_model::model::Model::from_file(
                &pes_model::format::PreFoxModel::read(bytes).ok()?,
            )
            .ok()?;
            all_far(
                model
                    .meshes
                    .iter()
                    .flat_map(|mesh| mesh.vertices.positions.iter()),
            )
        }
        ModelFormat::Gltf => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `konami_highneck` (204 near vertices) and `konami_card`.
    const HIGHNECK: &[u8] =
        include_bytes!("../../../libs/fmdl/tests/fixtures/konami_highneck.fmdl");
    const CARD: &[u8] = include_bytes!("../../../libs/pes_model/tests/fixtures/konami_card.model");

    /// The fixture `.fmdl` with every vertex moved `dx` units on x
    /// (`6000.0` makes the model far in every vertex).
    fn shifted_fmdl(dx: f32) -> Vec<u8> {
        shifted_fmdl_by([dx, 0.0, 0.0])
    }

    /// The fixture `.fmdl` with every vertex moved by `shift` — `[d; 3]`
    /// puts each vertex `3 d²` from the origin, `3000.0` past the far
    /// limit and `2000.0` under it.
    fn shifted_fmdl_by(shift: [f32; 3]) -> Vec<u8> {
        let mut model = fmdl::Model::from_file(&FmdlFile::read(HIGHNECK).unwrap()).unwrap();
        for mesh in &mut model.meshes {
            for position in &mut mesh.vertices.positions {
                for (component, d) in position.iter_mut().zip(shift) {
                    *component += d;
                }
            }
        }
        model.to_file().unwrap().write()
    }

    /// The fixture `.model` moved the same way.
    fn shifted_model(dx: f32) -> Vec<u8> {
        let mut model = pes_model::model::Model::from_file(
            &pes_model::format::PreFoxModel::read(CARD).unwrap(),
        )
        .unwrap();
        for mesh in &mut model.meshes {
            for position in &mut mesh.vertices.positions {
                position[0] += dx;
            }
        }
        model.to_file().unwrap().write().unwrap()
    }

    /// `far_drops` over `files`, `bytes` the read answers by name — an
    /// unlisted path does not read.
    fn drops(files: &[&str], any_folder: bool, bytes: &[(&str, &[u8])]) -> FarDrops {
        let files: Vec<String> = files.iter().map(|file| (*file).to_owned()).collect();
        far_drops(&files, any_folder, |path| {
            bytes
                .iter()
                .find(|(name, _)| *name == path)
                .map(|(_, bytes)| (*bytes).to_vec())
                .ok_or_else(|| SourceFailure {
                    path: path.to_owned(),
                    error: "not read".to_owned(),
                })
        })
    }

    /// The fixture `.fmdl` with one vertex far, the rest near.
    fn partly_far_fmdl() -> Vec<u8> {
        let mut model = fmdl::Model::from_file(&FmdlFile::read(HIGHNECK).unwrap()).unwrap();
        model.meshes[0].vertices.positions[0][0] += 6000.0;
        model.to_file().unwrap().write()
    }

    /// The fixture `.fmdl` with no mesh, so no vertex at all.
    fn vertexless_fmdl() -> Vec<u8> {
        let mut model = fmdl::Model::from_file(&FmdlFile::read(HIGHNECK).unwrap()).unwrap();
        model.meshes.clear();
        model.mesh_groups.clear();
        model.to_file().unwrap().write()
    }

    #[test]
    fn a_far_model_is_dropped_with_its_vertex_count() {
        let face = shifted_fmdl(6000.0);
        let out = drops(
            &["Faces/XXX05/face.fmdl"],
            false,
            &[("Faces/XXX05/face.fmdl", &face)],
        );
        assert_eq!(
            out.dropped.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Faces/XXX05/face.fmdl"]
        );
        let [message] = &out.findings[..] else {
            panic!("one finding per dropped model");
        };
        assert_eq!(message.code.code.as_ref(), "model_far_dropped");
        assert_eq!(message.severity, studio_core::Severity::Warning);
        assert_eq!(message.disposition, Disposition::DropFile);
        assert_eq!(
            message.context,
            vec![("vertices".to_owned(), "204".to_owned())]
        );
    }

    #[test]
    fn a_partly_far_model_is_kept() {
        // One far vertex of 204: the compiler's Error is for it to make.
        let face = partly_far_fmdl();
        let out = drops(
            &["Faces/XXX05/face.fmdl"],
            false,
            &[("Faces/XXX05/face.fmdl", &face)],
        );
        assert!(out.dropped.is_empty());
        assert!(out.findings.is_empty());
    }

    #[test]
    fn the_far_check_sums_all_three_axes() {
        // A one-axis shift leaves two of the sum's terms near zero;
        // shifting all three exercises every term: `3 * 3000²` is past
        // the limit and `3 * 2000²` under it.
        let far = shifted_fmdl_by([3000.0; 3]);
        let out = drops(
            &["Faces/XXX05/face.fmdl"],
            false,
            &[("Faces/XXX05/face.fmdl", &far)],
        );
        assert_eq!(
            out.dropped.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Faces/XXX05/face.fmdl"]
        );
        let near = shifted_fmdl_by([2000.0; 3]);
        let out = drops(
            &["Faces/XXX05/face.fmdl"],
            false,
            &[("Faces/XXX05/face.fmdl", &near)],
        );
        assert!(out.dropped.is_empty());
        assert!(out.findings.is_empty());
    }

    #[test]
    fn a_model_with_no_vertex_is_kept() {
        let face = vertexless_fmdl();
        let out = drops(
            &["Faces/XXX05/face.fmdl"],
            false,
            &[("Faces/XXX05/face.fmdl", &face)],
        );
        assert!(out.dropped.is_empty());
    }

    #[test]
    fn a_far_prefox_model_is_dropped_and_its_mtl_kept() {
        let glove_r = shifted_model(6000.0);
        // Only a model is ever dropped: the `.mtl` it alone named stays.
        let out = drops(
            &["Gloves/g0331/glove_r.model", "Gloves/g0331/glove_r.mtl"],
            false,
            &[("Gloves/g0331/glove_r.model", &glove_r)],
        );
        assert_eq!(
            out.dropped.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Gloves/g0331/glove_r.model"]
        );
        let [message] = &out.findings[..] else {
            panic!("one finding per dropped model");
        };
        assert_eq!(
            message.context,
            vec![("vertices".to_owned(), "140".to_owned())]
        );
    }

    #[test]
    fn a_far_model_below_any_folder_is_dropped_when_every_folder_is_in_scope() {
        // `any_folder` (a referee export's root layout, a model-folder
        // input's) puts every file below a top folder in scope, a
        // `Stadium/` model included; the folder list alone keeps it.
        let stadium = shifted_fmdl(6000.0);
        let out = drops(
            &["Stadium/far.fmdl"],
            true,
            &[("Stadium/far.fmdl", &stadium)],
        );
        assert_eq!(
            out.dropped.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Stadium/far.fmdl"]
        );
        let [message] = &out.findings[..] else {
            panic!("one model_far_dropped");
        };
        assert_eq!(message.code.code.as_ref(), "model_far_dropped");
        let out = drops(
            &["Stadium/far.fmdl"],
            false,
            &[("Stadium/far.fmdl", &stadium)],
        );
        assert!(out.dropped.is_empty());
        assert!(out.findings.is_empty());
    }
}
