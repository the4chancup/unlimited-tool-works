//! The referees' marker (`spec/team_compiler/referees.md` "Referee export processing"): the refs
//! export's `ref_marker.dds` converted as a Common texture is, shown under every referee. On
//! Fox it goes out in the referees' Common output, with a flat square written as the referees'
//! reserved collar, the bundled marker FMDL, its texture pointed at it. On pre-Fox it goes out
//! as the referee template tree's prop texture, in place of the tree's own: a `.mtl` beside a
//! nocloth model stops that model drawing (in game on PES 17, 2026-10-09), so a collar cannot
//! carry the marker there, and the prop the game draws under the referee by itself takes it.

use aesthetics_export::FileDescriptor;
use fmdl::ops::paths::rewrite_texture_paths;
use fmdl::{FmdlError, FmdlFile};
use pes_version::Engine;

use super::{CompileContext, Entry, TaskFailure, TaskFiles, texture};
use crate::paths::{self, REFEREE_MARKER_COLLAR, REFEREE_TEAM_ID};

/// The base texture's file name in the bundled marker model, the one the compile repoints.
const BUNDLED_TEXTURE: &str = "cup_logo.dds";

/// The stem the Fox marker texture goes out under in the referees' Common output.
const MARKER_STEM: &str = "ref_marker";

/// The marker `marker`, from its bytes in `files`, as the entries of `ctx`'s engine, its
/// texture converted as a Common texture is. On Fox: that texture under `ref_marker` in the
/// referees' Common output, then the bundled marker model with its base texture pointed at it,
/// as the referees' collar. On pre-Fox: that texture alone, at the template prop's texture path
/// (`paths::REFEREE_PROP_TEXTURE`), which the tree's own file then gives way to; no model and
/// no `.mtl`, since a `.mtl` beside a nocloth model stops it drawing in game. A failed
/// conversion fails the task with its texture code, so nothing goes out and the referees wear
/// no collar naming a texture the CPK lacks.
pub(super) fn referee_marker(
    marker: &FileDescriptor,
    ctx: &CompileContext,
    files: &mut TaskFiles,
) -> Result<Vec<Entry>, TaskFailure> {
    let texture = texture::common_texture(marker, marker.path.name(), ctx, files)?;
    let entries = match ctx.version.engine() {
        Engine::Fox => vec![
            (
                paths::common_texture(Engine::Fox, REFEREE_TEAM_ID, MARKER_STEM),
                texture,
            ),
            (
                paths::collar(Engine::Fox, REFEREE_MARKER_COLLAR),
                marker_model(ctx.templates.referee_marker())?,
            ),
        ],
        Engine::PreFox => vec![(paths::REFEREE_PROP_TEXTURE.to_owned(), texture)],
    };
    Ok(entries)
}

/// The marker model `bytes` with its base texture, `cup_logo.dds`, pointed at the marker
/// texture in the referees' Common output; its two other textures, the game's dummy normal and
/// specular maps, as they are.
fn marker_model(bytes: &[u8]) -> Result<Vec<u8>, FmdlError> {
    let mut file = FmdlFile::read(bytes)?;
    rewrite_texture_paths(&mut file, |path| {
        if path.file_name == BUNDLED_TEXTURE {
            path.directory = paths::common_texture_directory(Engine::Fox, REFEREE_TEAM_ID);
            path.file_name = format!("{MARKER_STEM}.dds");
        }
    })?;
    Ok(file.write())
}

#[cfg(test)]
mod tests {
    use fmdl::ops::paths::{TexturePath, texture_paths};
    use fmdl::{Model, Texture};

    use super::*;
    use crate::templates::Templates;

    /// The texture path `directory` + `file_name`.
    fn texture_path(directory: &str, file_name: &str) -> TexturePath {
        TexturePath {
            file_name: file_name.to_owned(),
            directory: directory.to_owned(),
        }
    }

    /// `model` with every material's texture paths blanked, its samplers kept.
    fn without_textures(mut model: Model) -> Model {
        for material in &mut model.materials {
            for (_, texture) in &mut material.textures {
                *texture = Texture {
                    file_name: String::new(),
                    directory: String::new(),
                };
            }
        }
        model
    }

    #[test]
    fn the_marker_model_s_base_texture_names_the_referees_marker_and_nothing_else_changes() {
        let bundled = Templates::embedded().referee_marker().to_vec();
        let bundled_file = FmdlFile::read(&bundled).unwrap();
        let dummies = "/Assets/pes16/model/character/common/sourceimages/";
        assert_eq!(
            texture_paths(&bundled_file).unwrap(),
            [
                texture_path(
                    "/Assets/pes16/model/character/common/000/sourceimages/",
                    "cup_logo.dds"
                ),
                texture_path(dummies, "dummy_nrm.tga"),
                texture_path(dummies, "dummy_srm.tga"),
            ],
            "the bundled model"
        );

        let marker = FmdlFile::read(&marker_model(&bundled).unwrap()).unwrap();

        assert_eq!(
            texture_paths(&marker).unwrap(),
            [
                texture_path(
                    "/Assets/pes16/model/character/common/999/sourceimages/",
                    "ref_marker.dds"
                ),
                texture_path(dummies, "dummy_nrm.tga"),
                texture_path(dummies, "dummy_srm.tga"),
            ]
        );
        // Bones, meshes, groups and materials but their textures: the semantic model is the
        // narrowest equality the crate gives that leaves the texture table aside.
        assert_eq!(
            without_textures(Model::from_file(&marker).unwrap()),
            without_textures(Model::from_file(&bundled_file).unwrap())
        );
    }
}
