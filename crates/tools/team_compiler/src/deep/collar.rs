//! The deep pass's collar checks (`spec/team_compiler/pipeline.md` "Collars"): which stock collar a
//! `Collars/` file replaces, read from its `collar_<ID>` name for the target version, and the
//! model checks any model gets. An Error drops the collar file alone: a collar is its own unit.

use aesthetics_export::{
    ContentFinding, Disposition, FileDescriptor, FileKind, IssueScope, ModelFormat,
};
use pes_version::PesVersion;

use super::model::ModelKind;
use super::{Checked, file_findings};
use crate::messages::Code;
use crate::paths::REFEREE_MARKER_COLLAR;
use crate::reader::ContentSource;

/// Why a collar file's name gives no stock collar to replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// The ID is one of the suite's own collars, held by `claimant`.
    Reserved {
        /// Who holds it: `FPC` or `referees`.
        claimant: &'static str,
    },
    /// The name is no `collar_<ID>`, or the ID is no stock collar of the version that a kit
    /// config can name.
    Invalid,
}

/// The findings of the collar file `file` compiled for `version`, each on the file. A file that
/// is no model (one a lenient file-type check keeps) gets none: it is no collar, and planning
/// passes it over. For a model, a name that gives no stock collar to replace is the one
/// finding: `collar_id_conflict` for a collar the suite holds (checked first), else
/// `collar_id_invalid`, either dropping the file and never passing through, as there is no
/// collar to keep. A Fox or pre-Fox model is then checked as any model is (`file_findings`): an
/// Error drops the file, anything below it keeps it, and a model that cannot be read or parsed
/// drops it. A glTF collar is not read.
pub(super) fn collar_findings(
    content: &ContentSource,
    file: &FileDescriptor,
    version: PesVersion,
) -> Vec<ContentFinding> {
    let scope = IssueScope::File(file.path.clone());
    let name = file.path.name();
    let refused = |code: Code, context: Vec<(&'static str, String)>| {
        vec![ContentFinding {
            code: code.as_str(),
            scope: scope.clone(),
            context,
            disposition: Disposition::DropFile,
            pass_through_eligible: false,
        }]
    };
    let kind = match file.kind {
        FileKind::Model(ModelFormat::Fmdl) => Some(ModelKind::Fmdl),
        FileKind::Model(ModelFormat::PesModel) => Some(ModelKind::PreFoxModel),
        // glTF is not read yet: its name is checked, its content not.
        FileKind::Model(ModelFormat::Gltf) => None,
        // The structure pass admits only models here, and keeps another kind only with the
        // strict check off, for `compile` to pass over: it names no collar.
        FileKind::Texture
        | FileKind::Skl
        | FileKind::Fclo
        | FileKind::Xml
        | FileKind::Mtl
        | FileKind::MaterialsToml
        | FileKind::Bin
        | FileKind::SharedLink(_)
        | FileKind::CommonLink
        | FileKind::Marker(_)
        | FileKind::Metadata(_)
        | FileKind::Other => return Vec::new(),
    };
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    match replaced_collar(stem, version) {
        Ok(_) => {}
        Err(Refusal::Reserved { claimant }) => {
            return refused(
                Code::CollarIdConflict,
                vec![("file", name.to_owned()), ("claimant", claimant.to_owned())],
            );
        }
        Err(Refusal::Invalid) => {
            return refused(Code::CollarIdInvalid, vec![("file", name.to_owned())]);
        }
    }
    let Some(kind) = kind else {
        return Vec::new();
    };
    file_findings(
        content,
        file,
        Checked::Model(kind),
        &scope,
        Disposition::DropFile,
        name,
        version.engine(),
    )
}

/// The stock collar the collar file of stem `stem` replaces on `version`, or why it replaces
/// none. The suite's own collars are checked before the version's stock set, so a file named
/// for one is a conflict on every version, even one whose stock set ends below it.
fn replaced_collar(stem: &str, version: PesVersion) -> Result<u8, Refusal> {
    let id = named_id(stem).ok_or(Refusal::Invalid)?;
    if id == fpc::kit_values().collar {
        return Err(Refusal::Reserved { claimant: "FPC" });
    }
    if id == REFEREE_MARKER_COLLAR {
        return Err(Refusal::Reserved {
            claimant: "referees",
        });
    }
    if (1..=last_stock_collar(version)).contains(&id) {
        Ok(id)
    } else {
        Err(Refusal::Invalid)
    }
}

/// The ID a collar file's stem names: `collar_` in any letter case, then one or more ASCII
/// digits and nothing else, zero padding allowed (`collar_12`, `collar_012` and `COLLAR_012`
/// all name 12). `None` for any other stem, and for an ID past 255: a kit config holds a
/// collar in one byte, so no kit could wear it, however the digits run.
pub(crate) fn named_id(stem: &str) -> Option<u8> {
    const PREFIX: &str = "collar_";
    let head = stem.get(..PREFIX.len())?;
    let digits = &stem[PREFIX.len()..];
    if !head.eq_ignore_ascii_case(PREFIX)
        || digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    digits.parse().ok()
}

/// The last stock collar of `version` that a kit config can name: the stock set is 1 to this,
/// counted in each install's base data CPK.
/// The 9xx collars past it are stock too, but no kit config can name one.
fn last_stock_collar(version: PesVersion) -> u8 {
    match version {
        // PES 2014's stock collars are uncounted; PES 2015's until they are.
        PesVersion::Pes14 => 101,
        PesVersion::Pes15 => 101,
        PesVersion::Pes16 => 105,
        PesVersion::Pes17 | PesVersion::Pes18 => 116,
        PesVersion::Pes19 => 124,
        PesVersion::Pes20 => 127,
        PesVersion::Pes21 => 131,
    }
}

#[cfg(test)]
mod tests {
    use fmdl::FmdlFile;
    use pes_model::format::PreFoxModel;

    use super::*;
    use crate::deep::tests::{
        counted, edited, far_boots, findings_for, path, pre_fox_fixture, tracer_file,
    };
    use crate::testing::scratch;

    #[test]
    fn a_collar_name_is_collar_and_digits_whatever_the_case_and_padding() {
        for stem in ["collar_12", "collar_012", "COLLAR_012", "Collar_0000012"] {
            assert_eq!(named_id(stem), Some(12), "{stem}");
        }
        for stem in [
            "neck",
            "collar",
            "collar_",
            "collar_-1",
            "collar_+1",
            "collar_1a",
            "collar_ 1",
            "collar_１",
            "my_collar_12",
            "player_12",
            "collar_256",
            "collar_99999999999999999999",
        ] {
            assert_eq!(named_id(stem), None, "{stem}");
        }
        assert_eq!(named_id("collar_255"), Some(255));
    }

    #[test]
    fn the_suite_s_collars_are_a_conflict_on_every_version() {
        for version in PesVersion::ALL {
            for (stem, claimant) in [
                ("collar_105", "FPC"),
                ("collar_0105", "FPC"),
                ("collar_77", "referees"),
                ("COLLAR_077", "referees"),
            ] {
                assert_eq!(
                    replaced_collar(stem, version),
                    Err(Refusal::Reserved { claimant }),
                    "{stem} on {version:?}"
                );
            }
        }
    }

    #[test]
    fn each_version_s_stock_collars_run_from_1_to_its_last_that_a_kit_can_name() {
        let last = [
            (PesVersion::Pes15, 101),
            (PesVersion::Pes16, 105),
            (PesVersion::Pes17, 116),
            (PesVersion::Pes18, 116),
            (PesVersion::Pes19, 124),
            (PesVersion::Pes20, 127),
            (PesVersion::Pes21, 131),
        ];
        for (version, last) in last {
            let collar = |id: u32| replaced_collar(&format!("collar_{id}"), version);
            // PES 16's last stock collar is the FPC collar, the suite's own.
            let top = if last == 105 { 104 } else { last };
            assert_eq!(collar(u32::from(top)), Ok(top), "{version:?}");
            assert_eq!(collar(1), Ok(1), "{version:?}");
            for refused in [0, u32::from(last) + 1, 901] {
                assert_eq!(
                    collar(refused),
                    Err(Refusal::Invalid),
                    "{refused} on {version:?}"
                );
            }
        }
    }

    #[test]
    fn a_collar_is_refused_by_its_name_checked_as_any_model_and_its_errors_drop_the_file() {
        let temp = scratch("deep_collars");
        // The tracer's right glove, clean, with its second bone named as its first: one
        // `fmdl_duplicate_bone_name`, a Warning.
        let duplicate_bone = edited(&tracer_file("glove_r.fmdl"), |model| {
            model.bones[1].name = model.bones[0].name.clone();
        });
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("Collars/collar_105.fmdl", far_boots()),
                ("Collars/collar_77.fmdl", far_boots()),
                ("Collars/neck.fmdl", far_boots()),
                ("Collars/neck.glb", b"not a model".to_vec()),
                ("Collars/collar_12.fmdl", b"not a model".to_vec()),
                ("Collars/collar_13.fmdl", far_boots()),
                ("Collars/collar_14.model", b"not a model".to_vec()),
                (
                    "Collars/collar_15.model",
                    pre_fox_fixture("community_empty_geometry_boots.wesys.model"),
                ),
                ("Collars/collar_16.glb", b"not a model".to_vec()),
                ("Collars/collar_17.fmdl", duplicate_bone),
            ],
            &[],
            &[],
        );
        let collar_13 = IssueScope::File(path("Collars/collar_13.fmdl"));
        let finding =
            |file: &str, code: &'static str, context: Vec<(&'static str, String)>| ContentFinding {
                code,
                scope: IssueScope::File(path(&format!("Collars/{file}"))),
                context,
                disposition: Disposition::DropFile,
                pass_through_eligible: false,
            };
        let named = |file: &str| ("file", file.to_owned());
        let fox_error = FmdlFile::read(b"not a model").unwrap_err().to_string();
        let pre_fox_error = PreFoxModel::read(b"not a model").unwrap_err().to_string();
        // In the files' order. A refused name is not read; a collar that reads gets its
        // model checks' findings on its file, an Error dropping it and anything below keeping
        // it; a glTF collar is not read, but its name is checked.
        assert_eq!(
            findings,
            [
                finding(
                    "collar_105.fmdl",
                    "collar_id_conflict",
                    vec![named("collar_105.fmdl"), ("claimant", "FPC".to_owned())],
                ),
                finding(
                    "collar_12.fmdl",
                    "model_broken",
                    vec![named("collar_12.fmdl"), ("error", fox_error)],
                ),
                counted(
                    "vertex_too_far_from_origin",
                    &collar_13,
                    "collar_13.fmdl",
                    1,
                    Disposition::DropFile,
                    false,
                ),
                counted(
                    "fmdl_weights_not_normalized",
                    &collar_13,
                    "collar_13.fmdl",
                    1662,
                    Disposition::Keep,
                    false,
                ),
                finding(
                    "collar_14.model",
                    "model_broken",
                    vec![named("collar_14.model"), ("error", pre_fox_error)],
                ),
                counted(
                    "model_mesh_empty",
                    &IssueScope::File(path("Collars/collar_15.model")),
                    "collar_15.model",
                    2,
                    Disposition::Keep,
                    false,
                ),
                counted(
                    "fmdl_duplicate_bone_name",
                    &IssueScope::File(path("Collars/collar_17.fmdl")),
                    "collar_17.fmdl",
                    1,
                    Disposition::Keep,
                    false,
                ),
                finding(
                    "collar_77.fmdl",
                    "collar_id_conflict",
                    vec![named("collar_77.fmdl"), ("claimant", "referees".to_owned())],
                ),
                finding("neck.fmdl", "collar_id_invalid", vec![named("neck.fmdl")]),
                finding("neck.glb", "collar_id_invalid", vec![named("neck.glb")]),
            ]
        );
    }

    #[test]
    fn a_collar_that_cannot_be_read_is_source_read_failed_on_its_file() {
        let temp = scratch("deep_collar_unread");
        let findings = findings_for(
            PesVersion::Pes21,
            temp.path(),
            &[],
            &["Collars/collar_12.fmdl"],
            &[],
        );
        let [finding] = findings.as_slice() else {
            panic!("{findings:?}");
        };
        assert_eq!(finding.code, "source_read_failed");
        assert_eq!(
            finding.scope,
            IssueScope::File(path("Collars/collar_12.fmdl"))
        );
        assert_eq!(finding.disposition, Disposition::DropFile);
        assert!(!finding.pass_through_eligible);
    }
}
