//! The canonical model: a superset of both FMDL and `.model`, so a conversion is one decode
//! path and one encode path per format instead of a direct pair per format combination.
//! `Option<T>` fields are format-specific: `Some` when imported from the format that has the
//! field, `None` otherwise; exporters fill in defaults for missing fields.

mod validate;

use std::collections::BTreeSet;

use crate::affine::Affine;

pub use crate::materials::{
    Address, Filter, FoxMaterial, Material, MaterialFamily, PreFoxMaterial, SamplerSettings,
    TextureRole,
};
pub use validate::ValidationError;
pub(crate) use validate::validate;

/// Which native format a model was imported from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// FMDL (PES 2018–2021).
    Fox,
    /// `.model` (PES 2015–2017).
    PreFox,
}

/// A whole model: skeleton, meshes, groups, materials and textures.
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalModel {
    /// The bones the meshes' skinning refers to; parents always precede their children.
    pub bones: Vec<Bone>,
    /// The meshes.
    pub meshes: Vec<Mesh>,
    /// The mesh groups (FMDL's grouping; a `.model` import makes one per mesh).
    pub mesh_groups: Vec<MeshGroup>,
    /// The materials.
    pub materials: Vec<Material>,
    /// The textures.
    pub textures: Vec<Texture>,
    /// Model-level header lines neither format crate types (`Skeleton-Type: Simplified`,
    /// ...), kept raw for the round trip.
    pub extension_headers: BTreeSet<String>,
    /// Which engine's format the model was imported from.
    pub source_format: SourceFormat,
}

/// One bone: name, hierarchy link and bind transform.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The bone's name.
    pub name: String,
    /// The hierarchy: FMDL stores it; `.model` does not (it is resolved by name from the
    /// template skeleton's render parents).
    pub parent: Option<usize>,
    /// The model-space 3x4 bind transform. Sources: FMDL's companion `.skl` (or the
    /// template); `.model`'s inline per-bone inverse bind matrix, inverted; glTF skin bind
    /// data.
    pub matrix: Affine,
    /// FMDL-specific: the bone's model-space position. `None` on other formats; an exporter
    /// without it derives it from `matrix`.
    pub global_position: Option<[f32; 4]>,
    /// FMDL-specific: `global_position` minus the parent's (equal to it on roots); an
    /// exporter without it derives both from `matrix` and `parent`.
    pub local_position: Option<[f32; 4]>,
    /// FMDL-specific per-bone bounds; `None` on other formats.
    pub bounding_box: Option<BoundingBox>,
}

/// One mesh: its vertex data, faces, bone group and material.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The vertex attributes.
    pub vertices: Vertices,
    /// Triangle faces; one winding for the whole IR (FMDL's) — a `.model` import reverses.
    pub faces: Vec<[u16; 3]>,
    /// Indices into `CanonicalModel::bones`; `vertices.bone_indices` index this list.
    pub bone_group: Vec<usize>,
    /// Index into `CanonicalModel::materials`.
    pub material: usize,
    /// Per-mesh header lines the format crate leaves raw.
    pub extension_headers: BTreeSet<String>,
    /// The FMDL `Custom-Bounding-Box-Meshes` entry; `None` on other formats.
    pub custom_bounding_box: Option<BoundingBox>,
}

/// Vertex attributes as a struct of arrays, like both format crates' `MeshVertices`: a
/// conversion is a column copy, and no per-vertex allocation exists on a 100k-vertex model.
/// Every `Vec` is `positions.len()` long when present.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vertices {
    /// The vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Four-component normals: w is 1.0 in every Konami FMDL measured; a `.model` import
    /// sets 1.0, a `.model` export drops it.
    pub normals: Option<Vec<[f32; 4]>>,
    /// Four-component tangents; w is the handedness (+-1 in Konami files).
    pub tangents: Option<Vec<[f32; 4]>>,
    /// `.model` only; an FMDL export drops them (a reported loss).
    pub bitangents: Option<Vec<[f32; 3]>>,
    /// Per-vertex colors.
    pub colors: Option<Vec<[u8; 4]>>,
    /// The UV sets, outer per set.
    pub uvs: Vec<Vec<[f32; 2]>>,
    /// Parallel to `uvs`: whether each set is stored at high precision. FMDL only; false
    /// elsewhere.
    pub uv_high_precision: Vec<bool>,
    /// Per-vertex bone slots, indexing `Mesh::bone_group`; unused slots are 0. Absent
    /// together with `bone_weights` on an unskinned mesh.
    pub bone_indices: Option<Vec<[u8; 4]>>,
    /// Per-vertex bone weights as floats; FMDL's bytes are `/255` on import and
    /// re-quantized on export so the total is kept.
    pub bone_weights: Option<Vec<[f32; 4]>>,
    /// `.model` stores 2, 3 or 4 weights per vertex; `None` means 4.
    pub bone_weight_width: Option<u8>,
}

impl Vertices {
    /// How many vertices the mesh carries (the length of `positions`).
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether the mesh carries any vertices.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// A named group of meshes: FMDL's mesh grouping. A `.model` has no groups — an import makes
/// one per mesh named after the mesh, an export names the mesh after its group.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshGroup {
    /// The group's name.
    pub name: String,
    /// The parent group's index; `None` at the root. Parents precede their children.
    pub parent: Option<usize>,
    /// Indices into `CanonicalModel::meshes`.
    pub meshes: Vec<usize>,
    /// Whether the group renders.
    pub visible: bool,
}

/// A texture reference: the path split into directory and file name, verbatim from the
/// source (`/Assets/.../sourceimages/`, `./`); the extension is kept as found. Final
/// in-game paths are the Compiler's texture relocation step, not this crate's.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// The directory part, including any trailing slash.
    pub directory: String,
    /// The file name with its extension.
    pub file_name: String,
}

/// An axis-aligned box; the fourth components are as the source format stores them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// The minimum corner.
    pub min: [f32; 4],
    /// The maximum corner.
    pub max: [f32; 4],
}

/// Whether any mesh weights a vertex positively on `bones[bone]` (a positive weight on a
/// slot whose group entry is the bone).
pub(crate) fn bone_is_weighted(ir: &CanonicalModel, bone: usize) -> bool {
    ir.meshes.iter().any(
        |mesh| match (&mesh.vertices.bone_indices, &mesh.vertices.bone_weights) {
            (Some(indices), Some(weights)) => indices.iter().zip(weights).any(|(row, ws)| {
                row.iter().enumerate().any(|(slot, &entry)| {
                    ws[slot] > 0.0 && mesh.bone_group.get(usize::from(entry)) == Some(&bone)
                })
            }),
            _ => false,
        },
    )
}

/// The IR's parent-first bone order over `parents`: file order except a bone listed
/// before its parent, which moves right after it (placing a bone places its waiting
/// children right after it, in file order). `Err` with the first offending bone when a
/// parent index is past the list or a chain never reaches a root — both an FMDL file can
/// carry.
pub(crate) fn parent_first_order(parents: &[Option<usize>]) -> Result<Vec<usize>, usize> {
    if let Some(bone) = parents
        .iter()
        .position(|parent| parent.is_some_and(|parent| parent >= parents.len()))
    {
        return Err(bone);
    }
    let mut order: Vec<usize> = Vec::with_capacity(parents.len());
    let mut placed = vec![false; parents.len()];
    let mut waiting: Vec<Vec<usize>> = vec![Vec::new(); parents.len()];
    let mut stack = Vec::new();
    for index in 0..parents.len() {
        if placed[index] {
            continue;
        }
        if let Some(parent) = parents[index]
            && !placed[parent]
        {
            waiting[parent].push(index);
            continue;
        }
        stack.push(index);
        while let Some(bone) = stack.pop() {
            placed[bone] = true;
            order.push(bone);
            stack.extend(waiting[bone].iter().rev());
        }
    }
    match placed.iter().position(|placed| !placed) {
        None => Ok(order),
        Some(bone) => Err(bone),
    }
}

/// Rebuilds `bones` keeping only the bones `keep` marks, in order: a kept bone's parent
/// becomes its nearest kept ancestor (`None` at a root). Returns each kept bone's new
/// index, `None` for the dropped.
pub(crate) fn rebuild_bone_list(bones: &mut Vec<Bone>, keep: &[bool]) -> Vec<Option<usize>> {
    let mut old_to_new = vec![None; bones.len()];
    let mut rebuilt = Vec::with_capacity(bones.len());
    for (old, bone) in bones.iter().enumerate() {
        if !keep[old] {
            continue;
        }
        old_to_new[old] = Some(rebuilt.len());
        let mut parent = bone.parent;
        while let Some(index) = parent {
            if keep[index] {
                break;
            }
            parent = bones[index].parent;
        }
        rebuilt.push(Bone {
            parent: parent
                .map(|index| old_to_new[index].expect("a kept parent precedes its child")),
            ..bone.clone()
        });
    }
    *bones = rebuilt;
    old_to_new
}

/// Rebuilds `mesh`'s bone group and weight slots for a rebuilt bone list. `old_to_new` is
/// `rebuild_bone_list`'s result; `redirect[old_bone]` is the new bone index a dropped bone's
/// weight moves to, or `None` to drop the weight. Kept entries remap in order first, then
/// each dropped entry's redirect is appended when absent. A slot pointing at a dropped bone
/// with no redirect becomes group index 0 with weight 0; slots that end up naming one group
/// entry merge their weight into the earliest.
pub(crate) fn remap_bone_group(
    mesh: &mut Mesh,
    old_to_new: &[Option<usize>],
    redirect: &[Option<usize>],
) {
    let mut group: Vec<usize> = Vec::with_capacity(mesh.bone_group.len());
    for &entry in &mesh.bone_group {
        if let Some(new) = old_to_new[entry] {
            group.push(new);
        }
    }
    for &entry in &mesh.bone_group {
        if let (None, Some(new)) = (old_to_new[entry], redirect[entry])
            && !group.contains(&new)
        {
            group.push(new);
        }
    }
    let slot_map: Vec<Option<u8>> = (0..mesh.bone_group.len())
        .map(|slot| {
            let entry = mesh.bone_group[slot];
            let new = match old_to_new[entry] {
                Some(new) => Some(new),
                None => redirect[entry],
            };
            new.map(|bone| {
                group
                    .iter()
                    .position(|&e| e == bone)
                    .expect("every group entry maps into the new group") as u8
            })
        })
        .collect();
    mesh.bone_group = group;
    if let (Some(indices), Some(weights)) = (
        &mut mesh.vertices.bone_indices,
        &mut mesh.vertices.bone_weights,
    ) {
        for (row, ws) in indices.iter_mut().zip(weights.iter_mut()) {
            for (slot, entry) in row.iter_mut().enumerate() {
                // Only a positive weight is remapped; a stale index in a zero-weight
                // slot (Konami files have them) becomes 0 without a lookup.
                if ws[slot] > 0.0 {
                    match slot_map[usize::from(*entry)] {
                        Some(new) => *entry = new,
                        None => {
                            *entry = 0;
                            ws[slot] = 0.0;
                        }
                    }
                } else {
                    *entry = 0;
                }
            }
            // Slots now naming one group entry merge into the earliest.
            for a in 0..4 {
                for b in (a + 1)..4 {
                    if row[a] == row[b] {
                        ws[a] += ws[b];
                        ws[b] = 0.0;
                        row[b] = 0;
                    }
                }
            }
        }
    }
}
