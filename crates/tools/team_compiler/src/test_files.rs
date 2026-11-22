//! The real bytes of the files `aesthetics_export::test_exports` adds to make an export clean,
//! for tests that write an export on disk.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use aesthetics_export::test_exports::{CleanFiles, Requirement, missing};
use pes_version::PesVersion;

/// The bytes of the files a clean export lacks, real enough that `check` and `compile` find
/// nothing in them: built once, shared by every test.
pub fn clean_files() -> &'static CleanFiles {
    static FILES: OnceLock<CleanFiles> = OnceLock::new();
    FILES.get_or_init(|| {
        let materials = String::from_utf8(pre_fox_fixture("cardhead_materials.mtl")).unwrap();
        assert!(materials.contains("./texture.dds"), "{materials}");
        let materials = materials.replace("./texture.dds", "./skin.dds");
        CleanFiles {
            fox_player: vec![("face_high.fmdl".to_owned(), clean_model())],
            pre_fox_player: vec![
                (
                    "face_high.model".to_owned(),
                    pre_fox_fixture("cardhead_face_high.model"),
                ),
                ("face_high.mtl".to_owned(), materials.into_bytes()),
                (
                    "skin.dds".to_owned(),
                    fixture("tests/fixtures/textures/single_level.dds"),
                ),
            ],
            portrait: fixture(
                "tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/portrait.dds",
            ),
            // At 512 x 512 the logo needs no upscale, so it adds no finding.
            logo: dds_convert::encode_png(&[0, 128, 0, 255].repeat(512 * 512), 512, 512).unwrap(),
        }
    })
}

/// A Fox model in which `fmdl`'s check finds nothing (the tracer's right glove), for a test
/// whose model only has to be one: both commands read every model, so an empty file would be
/// `model_broken`. Its hand bones are renamed from `skh_` to `skx_`, so a face model written
/// with it is not hand auto-split: the glove's vertices weigh on its finger bones, which would
/// give the player gloves and a `model_hand_split` the test is not about.
pub fn clean_model() -> Vec<u8> {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tracer/studio/egg Midcup Tracer/Players/05 - The Chad Stormworks Player/glove_r.fmdl"),
    )
    .unwrap();
    let mut model = fmdl::Model::from_file(&fmdl::FmdlFile::read(&bytes).unwrap()).unwrap();
    for bone in &mut model.bones {
        if let Some(finger) = bone.name.strip_prefix("skh_") {
            bone.name = format!("skx_{finger}");
        }
    }
    model.to_file().unwrap().write()
}

/// Writes into the export folder `folder` the files it still lacks to pass the export checks
/// on `version`, as `test_exports::missing` lists them from what the folder holds and from its
/// name (the export's kind), with `clean_files()`' bytes; the requirements in `without` add
/// nothing. A test writes the files it is about first, then calls this once per export.
pub fn complete(folder: &Path, version: PesVersion, without: &[Requirement]) {
    fs::create_dir_all(folder).unwrap();
    let mut held = Vec::new();
    list_under(folder, "", &mut held);
    let paths: Vec<&str> = held.iter().map(String::as_str).collect();
    let name = folder.file_name().unwrap().to_str().unwrap();
    for (path, bytes) in missing(name, version, &paths, without, clean_files()) {
        let path = folder.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

/// Adds to `held` every file and empty folder under `folder`, as `/`-separated paths that
/// start with `prefix`.
fn list_under(folder: &Path, prefix: &str, held: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let entry = entry.unwrap();
        let path = format!("{prefix}{}", entry.file_name().to_str().unwrap());
        if entry.path().is_dir() && fs::read_dir(entry.path()).unwrap().next().is_some() {
            list_under(&entry.path(), &format!("{path}/"), held);
        } else {
            held.push(path);
        }
    }
}

/// The bytes of `relative`, below the crate's folder.
fn fixture(relative: &str) -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The bytes of `pes_model`'s fixture `name`.
fn pre_fox_fixture(name: &str) -> Vec<u8> {
    fixture(&format!("../../libs/pes_model/tests/fixtures/{name}"))
}

#[cfg(test)]
mod tests {
    use crossbeam_channel::unbounded;
    use pipeline::CpkStem;
    use studio_core::{PipelineEvent, Scope, Severity};
    use teams_list::TeamsList;

    use super::*;
    use crate::cli::RunInputs;
    use crate::compile::{CpkLayout, OutputMode, RunPaths, RunningGame, run};
    use crate::output::deploy::prepare_output_folder;
    use crate::settings::{SavefilePath, TeamCompilerSettings};
    use crate::testing::{scratch, tool_context};

    /// The findings a `compile --no-deploy` of the export `name` reports about it, each as
    /// `<severity> <code> at <folder or file> (<context>)`, when the export holds only what
    /// `complete` adds on `version`; panics on a finding of the run's own above Info.
    fn compile_default(name: &str, version: PesVersion) -> Vec<String> {
        let temp = scratch(&format!("default_{}_{version:?}", name.replace(' ', "_")));
        let root = temp.path();
        complete(&root.join("exports").join(name), version, &[]);
        let output = root.join("output");
        prepare_output_folder(&output).unwrap();
        let (events_tx, events) = unbounded();
        let ctx = tool_context(root, "").with_events(events_tx);
        let mut common = ctx.common();
        common.pes_version = version;
        let inputs = RunInputs {
            settings: TeamCompilerSettings::default(),
            common,
            hide_ids: fpc::HideIds::CONVENTION,
            teams_list: TeamsList::parse("ID\tName\n714\t/co/\n").unwrap(),
            legacy_blocks: crate::plan::legacy_blocks::LegacyBlocks::embedded(),
            exports_root: root.join("exports"),
            exports: Vec::new(),
            timestamps: false,
        };
        let layout = CpkLayout::Single {
            name: CpkStem::new("4cc_99_test").unwrap(),
            cap: 3 << 30,
        };
        let paths = RunPaths {
            output_folder: output,
            savefile_path: SavefilePath::Path(root.join("save/EDIT00000000")),
            game: RunningGame {
                probe: Vec::new,
                edit_mode_closed: false,
            },
        };
        let mode = OutputMode::Normal { no_deploy: true };
        run(&inputs, &layout, None, &paths, &mode, &ctx).unwrap();
        drop(ctx);
        let mut lines = Vec::new();
        for envelope in events.iter() {
            let PipelineEvent::Message(message) = envelope.event else {
                continue;
            };
            let location = match &message.scope {
                Scope::Run => {
                    // The bins, the patch and the CPK: the run's lines, not the export's.
                    assert_eq!(message.severity, Severity::Info, "{message:?}");
                    continue;
                }
                Scope::Export { .. } | Scope::RosterEntry { .. } => String::new(),
                Scope::Folder { path, .. } | Scope::File { path, .. } => {
                    format!(" at {}", path.as_str())
                }
            };
            let context: Vec<String> = message
                .context
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect();
            lines.push(format!(
                "{:?} {}{location} ({})",
                message.severity,
                message.code.code,
                context.join(", ")
            ));
        }
        lines
    }

    #[test]
    fn each_default_export_compiles_with_only_its_own_findings() {
        let team = [
            "Info export_identified (team=/co/, id=714)",
            "Info team_colors_missing ()",
        ];
        // A `Full` export holding no kit compiles an empty `p1` and `g1`: the default holds
        // none, since a kit would add lines of its own to every test's output.
        let empty_kits = [
            "Info kit_config_generated at Kits/p1 ()",
            "Info kit_placeholder at Kits/p1 ()",
            "Info kit_config_generated at Kits/g1 ()",
            "Info kit_placeholder at Kits/g1 ()",
            "Warning kit_colors_missing at Kits/p1 ()",
            "Warning kit_colors_missing at Kits/g1 ()",
        ];
        let full: Vec<&str> = team.iter().chain(&empty_kits).copied().collect();
        let referees = ["Info export_identified (team=referees)"];
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            let cases: [(&str, &[&str]); 3] = [
                ("co Midcup Spring", &team),
                ("co Full Spring", &full),
                ("refs Cup", &referees),
            ];
            for (name, findings) in cases {
                assert_eq!(
                    compile_default(name, version),
                    findings,
                    "{name} on {version:?}"
                );
            }
        }
    }
}
