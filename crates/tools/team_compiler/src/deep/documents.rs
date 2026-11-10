//! The deep pass's checks of a folder's small data files, each read whole and parsed: a face
//! folder's face diff, a kit's `config.toml`, a player's `settings.toml` and
//! `ingame_face.toml`, a kit's or the root `colors.txt`, and the root `legacy_ids.toml`.
//! Their findings are
//! `face_diff_invalid`, `xml_dif_conflict`, `kit_config_invalid`,
//! `kit_config_version_clamped`, `kit_config_option_ignored`, `settings_toml_invalid`,
//! `ingame_face_toml_invalid`, `legacy_ids_invalid` and `color_entry_invalid`. None of them is
//! pass-through-eligible: a file that cannot be read leaves no value to keep, and a clamped
//! config value, an ignored config option and a refused `colors.txt` line are Warnings, which
//! drop nothing.

use std::fmt;

use aesthetics_export::{
    ColorLineRefusal, ContentFinding, Disposition, FileDescriptor, IssueScope, KitFolder,
    PlayerFolder, RootArtifacts, read_colors_txt,
};
use kit_config::{KitConfig, apply_fpc};
use pes_savefile::settings_toml::{IngameFaceSettings, PlayerSettings};
use pes_version::PesVersion;
use vtree::ScopePath;

use super::{read, relative};
use crate::face_diff;
use crate::messages::Code;
use crate::plan::EffectiveTeamKitFpc;
use crate::plan::legacy_ids::LegacyIds;
use crate::plan::roles::{FolderModels, PlayerFile, player_file};
use crate::reader::ContentSource;

/// The form a folder gives its face diff in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FaceDiffForm {
    /// A `face_diff.bin`, packed as it is.
    Bin,
    /// A `face_diff.xml`, the `face_diff.bin` as base64 text.
    Xml,
}

/// The form of face diff a file of `role` is, or `None` for any other file. The roles are
/// planning's (`player_file`), so a file planning gives no package (a `face_diff.bin` not
/// used in a folder with no face) is not read.
fn face_diff_form(role: Option<PlayerFile>) -> Option<FaceDiffForm> {
    match role? {
        PlayerFile::Packed {
            name: "face_diff.bin",
            ..
        } => Some(FaceDiffForm::Bin),
        PlayerFile::FaceDiffXml => Some(FaceDiffForm::Xml),
        PlayerFile::Packed { .. }
        | PlayerFile::Model { .. }
        | PlayerFile::CommonModel { .. }
        | PlayerFile::Skeleton { .. }
        | PlayerFile::SlotlessSkeleton
        | PlayerFile::UnusedFaceFile
        | PlayerFile::LeftOutKitVariant
        | PlayerFile::Texture { .. }
        | PlayerFile::CommonTexture(_)
        | PlayerFile::PreFoxModel { .. }
        | PlayerFile::PreFoxPart { .. }
        | PlayerFile::PreFoxCommonModel { .. }
        | PlayerFile::Material
        | PlayerFile::CommonMaterial
        | PlayerFile::FaceXml
        | PlayerFile::ConversionSkeleton
        | PlayerFile::UnsupportedGltf => None,
    }
}

/// The face diff findings of the model folder at `folder` holding `files`, whose models are
/// `models`, for the target `models` were computed for, over the files planning gives a face
/// diff's role (`player_file`), each dropping
/// the folder before any ID is planned for it. A second copy in a subfolder, which planning
/// leaves out, is checked too: a broken file in the folder is worth its finding, whichever
/// copy would be packed. A folder holding both forms gives its face diff twice:
/// `xml_dif_conflict`, naming the `.xml`, and neither file is read. Otherwise each is read
/// and checked, a `face_diff.bin` as it is and a `face_diff.xml` decoded, and one the game
/// cannot read is `face_diff_invalid` with the reason. When `xml_dif` says a member's own
/// `face.xml` of the folder holds a `<dif>` (PES 15-17 only: Fox ignores the xml), the face
/// writes that `<dif>` and packs no `face_diff.bin`, so a `face_diff.bin` is not read.
pub(super) fn face_diff_findings(
    content: &ContentSource,
    folder: &ScopePath,
    files: &[FileDescriptor],
    models: &FolderModels,
    xml_dif: bool,
) -> Vec<ContentFinding> {
    let face_diffs: Vec<(&FileDescriptor, FaceDiffForm)> = files
        .iter()
        .filter_map(|file| {
            face_diff_form(player_file(folder, file, models)).map(|form| (file, form))
        })
        .collect();
    let scope = IssueScope::Folder(folder.clone());
    let finding = |code: Code, context| ContentFinding {
        code: code.as_str(),
        scope: scope.clone(),
        context,
        disposition: Disposition::DropFolder,
        pass_through_eligible: false,
    };
    let xml = face_diffs
        .iter()
        .find(|(_, form)| *form == FaceDiffForm::Xml);
    let holds_bin = face_diffs
        .iter()
        .any(|(_, form)| *form == FaceDiffForm::Bin);
    if let Some((xml, _)) = xml
        && holds_bin
    {
        return vec![finding(
            Code::XmlDifConflict,
            vec![("file", relative(&xml.path, folder))],
        )];
    }
    let mut findings = Vec::new();
    for (file, form) in face_diffs {
        if xml_dif && form == FaceDiffForm::Bin {
            continue;
        }
        let bytes = match read(content, file, &scope, Disposition::DropFolder) {
            Ok(bytes) => bytes,
            Err(unread) => {
                findings.push(unread);
                continue;
            }
        };
        let checked = match form {
            FaceDiffForm::Bin => face_diff::check(&bytes),
            FaceDiffForm::Xml => face_diff::from_xml(&bytes).map(drop),
        };
        if let Err(error) = checked {
            findings.push(finding(
                Code::FaceDiffInvalid,
                vec![
                    ("file", relative(&file.path, folder)),
                    ("reason", error.to_string()),
                ],
            ));
        }
    }
    findings
}

/// The findings on `kit`'s `config.toml`, when it has one, for the target `version`.
/// `kit_config_invalid` when it is not UTF-8 or `KitConfig::from_toml` refuses it: the kit is
/// dropped, its textures with it, since the config names them. Otherwise one
/// `kit_config_version_clamped` per value the version's kit config cannot hold, on the file
/// and kept, naming the field, the value and the version's maximum, in `kit_config::validate`'s
/// order: the value is clamped when the config is emitted. After them, one
/// `kit_config_option_ignored` per sleeve or fit option the config's shirt model does not take,
/// on the file and kept, naming the option and the model: the config is emitted as written and
/// the game ignores the option. `validate`'s other findings (its two Infos and
/// `kit_collar_zero`) are not reported (`spec/team_compiler/pipeline.md` "Kit configs").
///
/// The config is checked as it is emitted: when the team's kit-FPC status `fpc` is `On`, with
/// the FPC values applied, because they set the shirt model to 176, which takes none of the
/// options.
pub(super) fn kit_config_findings(
    content: &ContentSource,
    kit: &KitFolder,
    version: PesVersion,
    fpc: EffectiveTeamKitFpc,
) -> Vec<ContentFinding> {
    let Some(file) = kit.config.as_ref() else {
        return Vec::new();
    };
    let mut config = match parsed_toml(
        content,
        file,
        Code::KitConfigInvalid,
        &IssueScope::Folder(kit.path.clone()),
        Disposition::DropFolder,
        KitConfig::from_toml,
    ) {
        Ok(config) => config,
        Err(invalid) => return vec![invalid],
    };
    // Once, before both checks: the FPC values touch no field the version clamps, so the
    // clamps come out the same either way.
    match fpc {
        EffectiveTeamKitFpc::On => apply_fpc(&mut config),
        EffectiveTeamKitFpc::Unknown => {}
    }
    let clamped = version_clamped(&config, version)
        .into_iter()
        .map(|context| (Code::KitConfigVersionClamped, context));
    let ignored = option_ignored(&config, version)
        .into_iter()
        .map(|context| (Code::KitConfigOptionIgnored, context));
    clamped
        .chain(ignored)
        .map(|(code, context)| ContentFinding {
            code: code.as_str(),
            scope: IssueScope::File(file.path.clone()),
            context,
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        })
        .collect()
}

/// The context of one `kit_config_option_ignored` per sleeve or fit option of `config` that
/// its shirt model does not take (`kit_config::validate`'s three
/// `kit_*_requires_model_144_or_160` findings), in `validate`'s order: the option's TOML key
/// and the shirt model.
fn option_ignored(config: &KitConfig, version: PesVersion) -> Vec<Vec<(&'static str, String)>> {
    kit_config::validate(config, version)
        .into_iter()
        // `validate`'s codes are strings, not one of our enums, so the catch-all arm is the
        // only form: it is every finding this function does not report.
        .filter_map(|finding| match finding.code {
            "kit_cut_out_requires_model_144_or_160" => Some("shirt.short_sleeves"),
            "kit_undershirt_only_requires_model_144_or_160" => Some("shirt.long_sleeves"),
            "kit_tight_requires_model_144_or_160" => Some("shirt.tight"),
            _ => None,
        })
        .map(|option| {
            vec![
                ("option", option.to_owned()),
                ("model", config.shirt.model.to_string()),
            ]
        })
        .collect()
}

/// The context of one `kit_config_version_clamped` per value of `config` that `version`'s
/// kit config cannot hold, in `kit_config::validate`'s order: the field, the value and the
/// version's maximum. `encode` clamps each such value; `validate`'s other findings are not
/// these.
pub(crate) fn version_clamped(
    config: &KitConfig,
    version: PesVersion,
) -> Vec<Vec<(&'static str, String)>> {
    kit_config::validate(config, version)
        .into_iter()
        .filter(|finding| {
            matches!(
                finding.code,
                "kit_value_out_of_range" | "kit_pattern_unsupported_pes15"
            )
        })
        .map(|finding| {
            finding
                .context
                .expect("`validate` names the field, value and limit of both range findings")
        })
        .map(|out_of_range| {
            vec![
                ("field", out_of_range.field.to_owned()),
                ("value", out_of_range.value.to_string()),
                ("max", out_of_range.max.to_string()),
            ]
        })
        .collect()
}

/// `player`'s `settings.toml` parsed; `None` when it has none. `settings_toml_invalid` when
/// it is not UTF-8 or `PlayerSettings::parse` refuses: the file alone is dropped, the
/// folder's models still compile, and his whole look is held back from the patch, the save
/// keeping his record's.
pub(super) fn player_settings(
    content: &ContentSource,
    player: &PlayerFolder,
) -> Result<Option<PlayerSettings>, ContentFinding> {
    let Some(file) = player.settings.as_ref() else {
        return Ok(None);
    };
    parsed_toml(
        content,
        file,
        Code::SettingsTomlInvalid,
        &IssueScope::File(file.path.clone()),
        Disposition::DropFile,
        PlayerSettings::parse,
    )
    .map(Some)
}

/// `player`'s `ingame_face.toml` parsed; `None` when it has none. `ingame_face_toml_invalid`
/// when it is not UTF-8 or `IngameFaceSettings::parse` refuses: the file stays, its presence
/// still marking the in-game face, and its keys are ignored; his whole look is held back
/// from the patch, `settings.toml`'s keys and the IDs they would resolve too; a
/// `write_names` export still takes his name from them.
pub(super) fn player_ingame_face(
    content: &ContentSource,
    player: &PlayerFolder,
) -> Result<Option<IngameFaceSettings>, ContentFinding> {
    let Some(file) = player.ingame_face.as_ref() else {
        return Ok(None);
    };
    parsed_toml(
        content,
        file,
        Code::IngameFaceTomlInvalid,
        &IssueScope::File(file.path.clone()),
        Disposition::Keep,
        IngameFaceSettings::parse,
    )
    .map(Some)
}

/// One `color_entry_invalid` per line the `colors.txt` `file` refuses (`read_colors_txt`, keeping
/// `capacity` colors), on the file and kept: the line is skipped, nothing is dropped. The
/// context gives the line's number and the reason. A file that cannot be read is
/// `source_read_failed` on the file instead, dropping it.
pub(super) fn colors_findings(
    content: &ContentSource,
    file: &FileDescriptor,
    capacity: usize,
) -> Vec<ContentFinding> {
    let scope = IssueScope::File(file.path.clone());
    let bytes = match read(content, file, &scope, Disposition::DropFile) {
        Ok(bytes) => bytes,
        Err(unread) => return vec![unread],
    };
    read_colors_txt(&bytes, capacity)
        .refused
        .into_iter()
        .map(|refused| {
            let reason = match refused.reason {
                ColorLineRefusal::NotOneColor => "not one color".to_owned(),
                ColorLineRefusal::PastCapacity => format!("more than {capacity} colors"),
            };
            ContentFinding {
                code: Code::ColorEntryInvalid.as_str(),
                scope: scope.clone(),
                context: vec![("line", refused.line.to_string()), ("reason", reason)],
                disposition: Disposition::Keep,
                pass_through_eligible: false,
            }
        })
        .collect()
}

/// The export's root `legacy_ids.toml` parsed; `None` when it has none.
/// `legacy_ids_invalid` when it is not UTF-8 or `LegacyIds::from_toml` refuses: the file
/// alone is dropped, the export compiling on the compiler's own boots and gloves IDs.
pub(super) fn legacy_ids(
    content: &ContentSource,
    root: &RootArtifacts,
) -> Result<Option<LegacyIds>, ContentFinding> {
    let Some(file) = root.legacy_ids.as_ref() else {
        return Ok(None);
    };
    parsed_toml(
        content,
        file,
        Code::LegacyIdsInvalid,
        &IssueScope::File(file.path.clone()),
        Disposition::DropFile,
        LegacyIds::from_toml,
    )
    .map(Some)
}

/// The TOML file `file`, directly in its folder, as `parse` reads it; or `code` on `scope`
/// with `disposition` when it is not UTF-8 text or `parse` refuses it, the context naming the
/// file and the error. A file that cannot be read is `source_read_failed` instead.
fn parsed_toml<T, E: fmt::Display>(
    content: &ContentSource,
    file: &FileDescriptor,
    code: Code,
    scope: &IssueScope,
    disposition: Disposition,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<T, ContentFinding> {
    let bytes = read(content, file, scope, disposition)?;
    let error = match std::str::from_utf8(&bytes) {
        Ok(text) => match parse(text) {
            Ok(parsed) => return Ok(parsed),
            Err(error) => error.to_string(),
        },
        Err(_) => "the file is not UTF-8 text".to_owned(),
    };
    Err(ContentFinding {
        code: code.as_str(),
        scope: scope.clone(),
        context: vec![("file", file.path.name().to_owned()), ("error", error)],
        disposition,
        pass_through_eligible: false,
    })
}

#[cfg(test)]
mod tests {
    use aesthetics_export::{ContentFinding, Disposition, IssueScope, PlayerSlot};
    use kit_config::KitConfig;
    use pes_savefile::settings_toml::PlayerSettings;
    use pes_version::PesVersion;
    use vtree::ScopePath;

    use crate::deep::tests::{
        dropping, findings_for, findings_of, fixture, folder, pass_for, path, texture, tracer_file,
    };
    use crate::plan::EffectiveTeamKitFpc;
    use crate::plan::item_rows::ItemTable;
    use crate::testing::scratch;

    /// A Fox model in which `fmdl`'s check finds nothing (the tracer's right glove), for a
    /// folder whose models only have to be there.
    fn clean_model() -> Vec<u8> {
        tracer_file("glove_r.fmdl")
    }

    /// The face diff fixture `name` (`tests/fixtures/face_diff/README.md`).
    fn face_diff(name: &str) -> Vec<u8> {
        fixture(&format!("face_diff/{name}"))
    }

    /// `dif.bin` one byte shorter than the 944 bytes its header gives.
    fn cut_face_diff() -> Vec<u8> {
        face_diff("dif.bin")[..943].to_vec()
    }

    /// The reason `face_diff_invalid` gives for `cut_face_diff`.
    const CUT_REASON: &str = "the face diff is 943 bytes long, but its header gives 944: the game would read past its end";

    /// `dif.xml` with one payload character replaced by one base64 does not use.
    fn corrupt_face_diff_xml() -> Vec<u8> {
        let mut xml = face_diff("dif.xml");
        let payload = xml.windows(4).position(|bytes| bytes == b"RkFD").unwrap();
        assert!(xml[payload + 100].is_ascii_alphanumeric());
        xml[payload + 100] = b'*';
        xml
    }

    #[test]
    fn a_face_diff_bin_shorter_than_its_header_gives_drops_its_folder_and_a_whole_one_passes() {
        let temp = scratch("deep_face_diff_bin");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/face_high.fmdl", clean_model()),
                ("Players/03 - A/face_diff.bin", face_diff("dif.bin")),
                ("Players/05 - B/face_high.fmdl", clean_model()),
                ("Players/05 - B/face_diff.bin", cut_face_diff()),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [dropping(
                "face_diff_invalid",
                "Players/05 - B",
                &[("file", "face_diff.bin"), ("reason", CUT_REASON)]
            )]
        );
    }

    #[test]
    fn a_face_diff_xml_whose_base64_is_corrupt_drops_its_folder_and_a_valid_one_passes() {
        let temp = scratch("deep_face_diff_xml");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/face_high.fmdl", clean_model()),
                ("Players/03 - A/face_diff.xml", face_diff("dif.xml")),
                ("Players/05 - B/face_high.fmdl", clean_model()),
                ("Players/05 - B/face_diff.xml", corrupt_face_diff_xml()),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [dropping(
                "face_diff_invalid",
                "Players/05 - B",
                &[
                    ("file", "face_diff.xml"),
                    ("reason", "the base64 text holds '*' where base64 cannot")
                ]
            )]
        );
    }

    #[test]
    fn a_folder_giving_its_face_diff_in_both_forms_is_a_conflict_and_neither_is_read() {
        let temp = scratch("deep_face_diff_twice");
        // The `.bin` is cut and the `.xml` corrupt: neither is reported, being unread.
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/face_high.fmdl", clean_model()),
                ("Players/03 - A/face_diff.bin", cut_face_diff()),
                ("Players/03 - A/face_diff.xml", corrupt_face_diff_xml()),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [dropping(
                "xml_dif_conflict",
                "Players/03 - A",
                &[("file", "face_diff.xml")]
            )]
        );
    }

    #[test]
    fn a_shared_face_folder_s_cut_face_diff_drops_that_folder() {
        let temp = scratch("deep_face_diff_shared");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/Round.face", Vec::new()),
                ("Faces/Round/hair_high.fmdl", clean_model()),
                ("Faces/Round/face_diff.bin", cut_face_diff()),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [dropping(
                "face_diff_invalid",
                "Faces/Round",
                &[("file", "face_diff.bin"), ("reason", CUT_REASON)]
            )]
        );
    }

    #[test]
    fn a_face_diff_bin_planning_gives_no_role_is_not_read() {
        let temp = scratch("deep_face_diff_no_role");
        // In a folder with no face model and no face link, a face file has no package to go in.
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/05 - B/kit_boots.fmdl", clean_model()),
                ("Players/05 - B/face_diff.bin", cut_face_diff()),
            ],
            &[],
        );
        assert_eq!(findings, []);
    }

    /// `kit_config_invalid` on `Kits/p1` with `error`.
    fn kit_config_invalid(error: &str) -> ContentFinding {
        dropping(
            "kit_config_invalid",
            "Kits/p1",
            &[("file", "config.toml"), ("error", error)],
        )
    }

    /// The deep pass's findings on a kit `p1` whose `config.toml` holds `config`.
    fn kit_findings(name: &str, config: &[u8]) -> Vec<ContentFinding> {
        kit_findings_for(
            name,
            PesVersion::Pes21,
            EffectiveTeamKitFpc::Unknown,
            config,
        )
    }

    /// `kit_findings` for PES `version`, the team's kit-FPC status `fpc`: when `On`, slot 05's
    /// folder, holding a face model, carries `fpc_on`.
    fn kit_findings_for(
        name: &str,
        version: PesVersion,
        fpc: EffectiveTeamKitFpc,
        config: &[u8],
    ) -> Vec<ContentFinding> {
        let temp = scratch(name);
        let mut files = vec![
            ("Kits/p1/kit.png", texture("kit.png")),
            ("Kits/p1/config.toml", config.to_vec()),
        ];
        match fpc {
            EffectiveTeamKitFpc::On => files.extend([
                ("Players/05 - A/face_high.fmdl", clean_model()),
                ("Players/05 - A/fpc_on", Vec::new()),
            ]),
            EffectiveTeamKitFpc::Unknown => {}
        }
        findings_for(version, temp.path(), &files, &[], &[])
    }

    /// `kit_config_version_clamped` on `Kits/p1/config.toml`, kept, for `field`, `value` and
    /// `max`.
    fn clamped(field: &str, value: &str, max: &str) -> ContentFinding {
        ContentFinding {
            code: "kit_config_version_clamped",
            scope: IssueScope::File(path("Kits/p1/config.toml")),
            context: vec![
                ("field", field.to_owned()),
                ("value", value.to_owned()),
                ("max", max.to_owned()),
            ],
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        }
    }

    #[test]
    fn a_kit_config_value_over_the_version_s_maximum_is_reported_and_kept() {
        let unknown = EffectiveTeamKitFpc::Unknown;
        assert_eq!(
            kit_findings_for(
                "deep_kit_name_y_18",
                PesVersion::Pes18,
                unknown,
                b"[name]\ny = 36\n"
            ),
            [clamped("name.y", "36", "33")]
        );
        assert_eq!(
            kit_findings_for(
                "deep_kit_name_y_21",
                PesVersion::Pes21,
                unknown,
                b"[name]\ny = 39\n"
            ),
            []
        );
        assert_eq!(
            kit_findings_for(
                "deep_kit_pattern_15",
                PesVersion::Pes15,
                unknown,
                b"[shirt]\npattern = 12\n"
            ),
            [clamped("shirt.pattern", "12", "11")]
        );
        // Two values out of range, in `validate`'s order.
        assert_eq!(
            kit_findings_for(
                "deep_kit_two_clamped_15",
                PesVersion::Pes15,
                unknown,
                b"[shirt]\npattern = 12\n\n[name]\ny = 36\n"
            ),
            [
                clamped("name.y", "36", "33"),
                clamped("shirt.pattern", "12", "11")
            ]
        );
    }

    /// `kit_config_option_ignored` on `Kits/p1/config.toml`, kept, for `option` and `model`.
    fn ignored(option: &str, model: &str) -> ContentFinding {
        ContentFinding {
            code: "kit_config_option_ignored",
            scope: IssueScope::File(path("Kits/p1/config.toml")),
            context: vec![("option", option.to_owned()), ("model", model.to_owned())],
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        }
    }

    #[test]
    fn a_sleeve_or_fit_option_the_shirt_model_does_not_take_is_reported_and_kept() {
        let three =
            "short_sleeves = \"cut-out\"\nlong_sleeves = \"undershirt-only\"\ntight = true\n";
        assert_eq!(
            kit_findings(
                "deep_kit_options_176",
                format!("[shirt]\nmodel = 176\n{three}").as_bytes()
            ),
            [
                ignored("shirt.short_sleeves", "176"),
                ignored("shirt.long_sleeves", "176"),
                ignored("shirt.tight", "176"),
            ]
        );
        let model_144 = format!("[shirt]\nmodel = 144\n{three}");
        assert_eq!(
            kit_findings("deep_kit_options_144", model_144.as_bytes()),
            []
        );
        // With the team's kit-FPC status On the config is emitted with shirt model 176, which
        // takes none of the three.
        assert_eq!(
            kit_findings_for(
                "deep_kit_options_144_fpc_on",
                PesVersion::Pes21,
                EffectiveTeamKitFpc::On,
                model_144.as_bytes()
            ),
            [
                ignored("shirt.short_sleeves", "176"),
                ignored("shirt.long_sleeves", "176"),
                ignored("shirt.tight", "176"),
            ]
        );
        // After the clamped values.
        assert_eq!(
            kit_findings_for(
                "deep_kit_option_after_clamp",
                PesVersion::Pes18,
                EffectiveTeamKitFpc::Unknown,
                b"[shirt]\nmodel = 176\ntight = true\n\n[name]\ny = 36\n"
            ),
            [clamped("name.y", "36", "33"), ignored("shirt.tight", "176")]
        );
    }

    #[test]
    fn a_kit_config_that_does_not_parse_drops_its_kit_and_a_valid_one_passes() {
        let wrong_type = KitConfig::from_toml("shirt = 144").unwrap_err().to_string();
        assert_eq!(
            kit_findings("deep_kit_config_wrong_type", b"shirt = 144"),
            [kit_config_invalid(&wrong_type)]
        );
        assert_eq!(
            kit_findings("deep_kit_config_not_utf8", b"\xFF\xFE"),
            [kit_config_invalid("the file is not UTF-8 text")]
        );
        let tracer = fixture("tracer/studio/egg Midcup Tracer/Kits/g1/config.toml");
        assert_eq!(kit_findings("deep_kit_config_valid", &tracer), []);
    }

    /// The deep pass's findings on slot 03's folder holding a face model and a
    /// `settings.toml` of `settings`.
    fn settings_findings(name: &str, settings: &[u8]) -> Vec<ContentFinding> {
        let temp = scratch(name);
        findings_of(
            temp.path(),
            &[
                ("Players/03 - A/face_high.fmdl", clean_model()),
                ("Players/03 - A/settings.toml", settings.to_vec()),
            ],
            &[],
        )
    }

    #[test]
    fn a_settings_toml_that_does_not_parse_drops_that_file_alone_and_the_template_passes() {
        let wrong_type = PlayerSettings::parse("name = 5").unwrap_err().to_string();
        assert_eq!(
            settings_findings("deep_settings_wrong_type", b"name = 5"),
            [ContentFinding {
                code: "settings_toml_invalid",
                scope: IssueScope::File(ScopePath::new("Players/03 - A/settings.toml").unwrap()),
                context: vec![("file", "settings.toml".to_owned()), ("error", wrong_type)],
                disposition: Disposition::DropFile,
                pass_through_eligible: false,
            }]
        );
        let not_utf8 = settings_findings("deep_settings_not_utf8", b"\xFF\xFE");
        assert_eq!(not_utf8.len(), 1, "{not_utf8:?}");
        assert_eq!(
            not_utf8[0].context[1],
            ("error", "the file is not UTF-8 text".to_owned())
        );
        let template = PlayerSettings::default().to_toml().unwrap();
        assert_eq!(
            settings_findings("deep_settings_template", template.as_bytes()),
            []
        );
        assert_eq!(settings_findings("deep_settings_empty", b""), []);
    }

    /// The tracer's kit texture, which the deep pass finds nothing in.
    fn tracer_kit() -> Vec<u8> {
        fixture("tracer/studio/egg Midcup Tracer/Kits/g1/kit.dds")
    }

    /// `color_entry_invalid` on the `colors.txt` at `file`, kept, for `line` and `reason`.
    fn color_entry_invalid(file: &str, line: &str, reason: &str) -> ContentFinding {
        ContentFinding {
            code: "color_entry_invalid",
            scope: IssueScope::File(path(file)),
            context: vec![("line", line.to_owned()), ("reason", reason.to_owned())],
            disposition: Disposition::Keep,
            pass_through_eligible: false,
        }
    }

    /// The deep pass's findings on a kit `p1` holding the tracer's `kit.dds` and `files`
    /// (path, contents) beside it.
    fn colors_findings(name: &str, files: &[(&str, &[u8])]) -> Vec<ContentFinding> {
        let temp = scratch(name);
        let mut written = vec![("Kits/p1/kit.dds", tracer_kit())];
        written.extend(files.iter().map(|(path, bytes)| (*path, bytes.to_vec())));
        findings_of(temp.path(), &written, &[])
    }

    #[test]
    fn a_kit_s_colors_txt_line_that_gives_no_color_is_kept_with_its_number_and_reason() {
        assert_eq!(
            colors_findings(
                "deep_kit_colors_clean",
                &[("Kits/p1/colors.txt", b"#c11200\n")]
            ),
            []
        );
        assert_eq!(
            colors_findings(
                "deep_kit_colors_bad_line",
                &[("Kits/p1/colors.txt", b"#c11200\nbad\n")]
            ),
            [color_entry_invalid(
                "Kits/p1/colors.txt",
                "2",
                "not one color"
            )]
        );
        assert_eq!(
            colors_findings(
                "deep_kit_colors_three",
                &[("Kits/p1/colors.txt", b"#c11200\n#414141\n211 74 79\n")]
            ),
            [color_entry_invalid(
                "Kits/p1/colors.txt",
                "3",
                "more than 2 colors"
            )]
        );
    }

    #[test]
    fn the_root_colors_txt_keeps_four_colors_and_reports_a_fifth() {
        assert_eq!(
            colors_findings(
                "deep_team_colors_four",
                &[("colors.txt", b"#c11200\n#414141\n211 74 79\n1 2 3\n")]
            ),
            []
        );
        assert_eq!(
            colors_findings(
                "deep_team_colors_five",
                &[("colors.txt", b"#c11200\n#414141\n211 74 79\n1 2 3\n4 5 6\n")]
            ),
            [color_entry_invalid("colors.txt", "5", "more than 4 colors")]
        );
    }

    #[test]
    fn a_kit_s_colors_stand_between_its_config_and_its_textures_and_the_root_s_come_last() {
        let findings = colors_findings(
            "deep_colors_order",
            &[
                ("colors.txt", b"bad\n"),
                ("logo.png", b"not an image"),
                ("Kits/p1/kit_back.png", &texture("tiny.png")),
                ("Kits/p1/colors.txt", b"bad\n"),
                ("Kits/p1/config.toml", b"shirt = 144"),
            ],
        );
        let order: Vec<(&str, IssueScope)> = findings
            .iter()
            .map(|finding| (finding.code, finding.scope.clone()))
            .collect();
        assert_eq!(
            order,
            [
                ("kit_config_invalid", folder("Kits/p1")),
                (
                    "color_entry_invalid",
                    IssueScope::File(path("Kits/p1/colors.txt"))
                ),
                ("texture_too_small", folder("Kits/p1")),
                ("logo_file_invalid", IssueScope::File(path("logo.png"))),
                ("color_entry_invalid", IssueScope::File(path("colors.txt"))),
            ]
        );
    }

    #[test]
    fn a_folder_s_face_diff_and_settings_stand_with_it_and_a_kit_s_config_before_its_textures() {
        let temp = scratch("deep_documents_order");
        let findings = findings_of(
            temp.path(),
            &[
                ("Kits/p1/kit_back.png", texture("tiny.png")),
                ("Kits/p1/kit.png", texture("kit.png")),
                ("Kits/p1/config.toml", b"shirt = 144".to_vec()),
                ("Players/05 - B/face_high.fmdl", clean_model()),
                ("Players/05 - B/face_diff.bin", cut_face_diff()),
                ("Players/03 - A/settings.toml", b"name = 5".to_vec()),
                ("Players/03 - A/portrait.png", texture("tiny.png")),
                ("Players/03 - A/skin.png", texture("tiny.png")),
                ("Players/03 - A/face_high.fmdl", clean_model()),
                ("Players/03 - A/face_diff.bin", cut_face_diff()),
            ],
            &[],
        );
        let order: Vec<(&str, &str)> = findings
            .iter()
            .map(|finding| (finding.code, finding.context[0].1.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                ("texture_too_small", "skin.png"),
                ("face_diff_invalid", "face_diff.bin"),
                ("texture_too_small", "portrait.png"),
                ("settings_toml_invalid", "settings.toml"),
                ("face_diff_invalid", "face_diff.bin"),
                ("kit_config_invalid", "config.toml"),
                ("texture_too_small", "kit_back.png"),
            ]
        );
        assert_eq!(findings[4].scope, folder("Players/05 - B"));
    }

    /// A player holding either file gets a `settings` entry: a `settings.toml`
    /// alone, an `ingame_face.toml` alone giving his face keys, none for a
    /// folder holding neither.
    #[test]
    fn a_player_with_either_settings_file_gets_an_entry() {
        let temp = scratch("deep_settings_entry");
        let pass = pass_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                (
                    "Players/03 - A/settings.toml",
                    b"[appearance]\nskin_color = 4\n".to_vec(),
                ),
                ("Players/03 - A/face_high.fmdl", clean_model()),
                (
                    "Players/05 - B/ingame_face.toml",
                    b"iris_color = 7\n[hair]\nlength = 5\n".to_vec(),
                ),
                ("Players/07 - C/face_high.fmdl", clean_model()),
            ],
            &[],
            &[],
        );
        assert_eq!(pass.findings, [], "{:?}", pass.findings);
        assert_eq!(pass.settings.len(), 2);
        let joined = &pass.settings[&path("Players/05 - B")];
        assert_eq!(joined.appearance.iris_color, Some(7));
        assert_eq!(joined.appearance.hair.length, Some(5));
        assert_eq!(joined.appearance.skin_color, None);
    }

    /// One file refused leaves the other's keys in the settings, both ways: a
    /// `settings.toml` that does not parse is `settings_toml_invalid` and the face
    /// file's keys are kept; an `ingame_face.toml` that does not parse is
    /// `ingame_face_toml_invalid` and the settings file's are kept. Either way the
    /// player is marked refused, which holds his look back from the patch.
    #[test]
    fn a_refused_settings_or_ingame_face_file_leaves_the_other_s_keys() {
        for (name, files, code, file, disposition, key, value) in [
            (
                "deep_refused_settings",
                vec![
                    ("Players/03 - A/settings.toml", b"name = 5".to_vec()),
                    (
                        "Players/03 - A/ingame_face.toml",
                        b"iris_color = 7\n".to_vec(),
                    ),
                ],
                "settings_toml_invalid",
                "Players/03 - A/settings.toml",
                Disposition::DropFile,
                "iris_color",
                Some(7u8),
            ),
            (
                "deep_refused_face",
                vec![
                    (
                        "Players/03 - A/settings.toml",
                        b"[appearance]\nskin_color = 4\n".to_vec(),
                    ),
                    ("Players/03 - A/ingame_face.toml", b"iris_color = ".to_vec()),
                ],
                "ingame_face_toml_invalid",
                "Players/03 - A/ingame_face.toml",
                Disposition::Keep,
                "skin_color",
                Some(4u8),
            ),
        ] {
            let temp = scratch(name);
            let pass = pass_for(PesVersion::Pes21, temp.path(), &files, &[], &[]);
            assert_eq!(pass.findings.len(), 1, "{files:?}: {:?}", pass.findings);
            let finding = &pass.findings[0];
            assert_eq!(finding.code, code);
            assert_eq!(finding.scope, IssueScope::File(path(file)));
            assert_eq!(finding.disposition, disposition);
            assert_eq!(finding.context[0].0, "file");
            assert!(finding.context[1].0 == "error" && !finding.context[1].1.is_empty());
            let settings = &pass.settings[&path("Players/03 - A")];
            let got = match key {
                "iris_color" => settings.appearance.iris_color,
                _ => settings.appearance.skin_color,
            };
            assert_eq!(got, value, "{name}: the applied file's key");
            assert!(
                pass.settings_refused.contains(&path("Players/03 - A")),
                "{name}: a refused file holds his whole look back"
            );
        }
    }

    /// A root `legacy_ids.toml` parses into the pass, and one that does not parse is
    /// `legacy_ids_invalid` on the file and gives it nothing.
    #[test]
    fn a_root_legacy_ids_toml_parses_into_the_pass_and_an_invalid_one_is_dropped() {
        let temp = scratch("deep_legacy_ids");
        let pass = pass_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("legacy_ids.toml", b"[05]\nboots = 2305\n".to_vec()),
                ("Players/03 - A/face_high.fmdl", clean_model()),
            ],
            &[],
            &[],
        );
        assert_eq!(pass.findings, [], "{:?}", pass.findings);
        let ids = pass.legacy_ids.expect("the file parsed");
        assert_eq!(
            ids.old_id(PlayerSlot::new(5).unwrap(), ItemTable::Boots),
            Some(2305)
        );

        let temp = scratch("deep_legacy_ids_bad");
        let pass = pass_for(
            PesVersion::Pes21,
            temp.path(),
            &[
                ("legacy_ids.toml", b"[05]\nboots = 2305\n[24]\n".to_vec()),
                ("Players/03 - A/face_high.fmdl", clean_model()),
            ],
            &[],
            &[],
        );
        assert_eq!(pass.legacy_ids, None);
        let [finding] = pass.findings.as_slice() else {
            panic!("one finding: {:?}", pass.findings)
        };
        assert_eq!(finding.code, "legacy_ids_invalid");
        assert_eq!(finding.scope, IssueScope::File(path("legacy_ids.toml")));
        assert_eq!(finding.disposition, Disposition::DropFile);
        assert_eq!(finding.context[0], ("file", "legacy_ids.toml".to_owned()));
        assert!(finding.context[1].0 == "error" && !finding.context[1].1.is_empty());
    }
}
