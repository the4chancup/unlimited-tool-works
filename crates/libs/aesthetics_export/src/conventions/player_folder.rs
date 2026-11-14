//! The model-name suffix table the structure pass needs ("Model names: a free
//! part plus a suffix"). Pre-Fox type names join it with the deep pass.

use super::kit_token::without_kit_token;

/// The recognized model-name suffixes: what a model file's stem tail says the
/// model is. The structure pass checks shared folders' names against it; the
/// Compiler routes a player folder's models by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModelSuffix {
    /// `face_high`.
    FaceHigh,
    /// `hair_high`.
    HairHigh,
    /// `oral`.
    Oral,
    /// `fcl_hair`.
    FclHair,
    /// `boots`.
    Boots,
    /// `glove_l` / `gloveL`.
    GloveL,
    /// `glove_r` / `gloveR`.
    GloveR,
    /// `handL`.
    HandL,
    /// `handR`.
    HandR,
}

/// The suffixes' underscore-free lowercase spellings.
const MODEL_SUFFIXES: [(&str, ModelSuffix); 9] = [
    ("facehigh", ModelSuffix::FaceHigh),
    ("hairhigh", ModelSuffix::HairHigh),
    ("oral", ModelSuffix::Oral),
    ("fclhair", ModelSuffix::FclHair),
    ("boots", ModelSuffix::Boots),
    ("glovel", ModelSuffix::GloveL),
    ("glover", ModelSuffix::GloveR),
    ("handl", ModelSuffix::HandL),
    ("handr", ModelSuffix::HandR),
];

/// A stem's trailing suffix (`ends_with_name` over the table). `face_high`,
/// `FaceHigh`, `x_hair_high`, `gloveL` match; `myface_high`, `coral`, `torso`
/// do not; a per-kit `boots_kit1` or `kit1_boots` is boots on both engines.
pub fn model_suffix(stem: &str) -> Option<ModelSuffix> {
    MODEL_SUFFIXES
        .into_iter()
        .find(|(spelling, _)| ends_with_name(stem, spelling))
        .map(|(_, suffix)| suffix)
}

/// Whether a model's stem ends with the type name `name`, the way a model name
/// ends with its suffix: walking the stem from the end and skipping `_` in
/// both, its last non-underscore characters equal `name`'s
/// ASCII-case-insensitively, and what precedes them is empty or ends with `_`
/// (`body_uniform` and `uniform` end with `uniform`, `xuniform` does not). The
/// stem is read without its kit token and one delimiter next to it
/// (`without_kit_token`), so a per-kit model is typed as its set is.
pub fn ends_with_name(stem: &str, name: &str) -> bool {
    let stem = without_kit_token(stem);
    let stem = stem.as_ref();
    let mut reversed = stem.char_indices().rev().filter(|(_, c)| *c != '_');
    let mut start = stem.len();
    for want in name.chars().rev().filter(|c| *c != '_') {
        match reversed.next() {
            Some((index, c)) if c.eq_ignore_ascii_case(&want) => start = index,
            _ => return false,
        }
    }
    start == 0 || stem[..start].ends_with('_')
}

/// The per-player singletons: a player has one of each, so each counts directly
/// in the player folder alone, as the markers do. Two from different folders
/// of his tree would have no way to merge. Their
/// kinds (`Xml`, `Bin`, `Fclo`) admit other files below a subfolder, so they
/// are told by name.
const PLAYER_SINGLETONS: [&str; 4] = [
    "face.xml",
    "face_diff.bin",
    "face_diff.xml",
    "fcl_hair_sim.fclo",
];

/// Whether `file_name` names a per-player singleton (`face.xml`,
/// `face_diff.bin`, `face_diff.xml`, `fcl_hair_sim.fclo`),
/// ASCII-case-insensitively as `classify` matches names: a file that counts
/// directly in a player folder alone, below a subfolder `file_type_disallowed`.
pub fn is_player_singleton(file_name: &str) -> bool {
    PLAYER_SINGLETONS
        .iter()
        .any(|singleton| file_name.eq_ignore_ascii_case(singleton))
}

/// The explicitly-named face models (`face_high`, `hair_high`, `oral`): they
/// have no skeleton slot and cannot exist under `ingame_face`.
pub(crate) fn is_explicit_face(suffix: ModelSuffix) -> bool {
    matches!(
        suffix,
        ModelSuffix::FaceHigh | ModelSuffix::HairHigh | ModelSuffix::Oral
    )
}

/// The boots suffix.
pub(crate) fn is_boots(suffix: ModelSuffix) -> bool {
    suffix == ModelSuffix::Boots
}

/// The gloves suffixes (including the hand-skeleton models, which have
/// nowhere else to go on Fox).
pub(crate) fn is_gloves(suffix: ModelSuffix) -> bool {
    matches!(
        suffix,
        ModelSuffix::GloveL | ModelSuffix::GloveR | ModelSuffix::HandL | ModelSuffix::HandR
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_suffix_reads_the_stem_tail() {
        let cases: [(&str, Option<ModelSuffix>); 26] = [
            ("face_high", Some(ModelSuffix::FaceHigh)),
            ("FaceHigh", Some(ModelSuffix::FaceHigh)),
            ("x_hair_high", Some(ModelSuffix::HairHigh)),
            ("oral", Some(ModelSuffix::Oral)),
            ("fcl_hair", Some(ModelSuffix::FclHair)),
            ("kit_boots", Some(ModelSuffix::Boots)),
            ("gloveL", Some(ModelSuffix::GloveL)),
            ("glove_l", Some(ModelSuffix::GloveL)),
            ("glove_r", Some(ModelSuffix::GloveR)),
            ("handL", Some(ModelSuffix::HandL)),
            ("handR", Some(ModelSuffix::HandR)),
            ("kit_boots_", Some(ModelSuffix::Boots)),
            ("myface_high", None),
            ("coral", None),
            ("torso", None),
            ("ボコ", None),
            ("face", None),
            ("_oral", Some(ModelSuffix::Oral)),
            ("hair-high", None),
            ("x_-boots", None),
            ("boots_kit1", Some(ModelSuffix::Boots)),
            ("kit1_boots", Some(ModelSuffix::Boots)),
            ("face_high-kitN", Some(ModelSuffix::FaceHigh)),
            ("glove_l_kit2", Some(ModelSuffix::GloveL)),
            ("pants_kit1", None),
            ("boots_kit1_x", None),
        ];
        for (stem, expected) in cases {
            assert_eq!(model_suffix(stem), expected, "{stem:?}");
        }
    }

    #[test]
    fn ends_with_name_reads_the_stem_tail_without_its_kit_token() {
        for (stem, name, expected) in [
            ("body_uniform", "uniform", true),
            ("uniform", "uniform", true),
            ("x_pants_nocloth", "pants_nocloth", true),
            ("xuniform", "uniform", false),
            ("body_uniform_kit1", "uniform", true),
            ("Body_UNIFORM", "uniform", true),
        ] {
            assert_eq!(ends_with_name(stem, name), expected, "{stem:?} {name:?}");
        }
    }
}
