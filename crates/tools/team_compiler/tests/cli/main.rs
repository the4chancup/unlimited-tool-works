//! The Team compiler's command line run in-process (`spec/team_compiler/settings.md` "CLI"): the
//! preflight's refusals, `check`'s findings and what `compile` writes, observed as the exit
//! code, the `Message` events the tool emits (which the binary prints one line each) and the
//! CPK's entries.

#[path = "../common/mod.rs"]
mod common;

mod bins;
mod check;
mod collars;
mod common_links;
mod compile;
mod compile_exports;
mod conversion;
mod dds_compression;
mod deep;
mod deploy;
mod face_folders;
mod kit_layout;
mod legacy_ids;
mod logo;
mod models;
mod multicpk;
mod patch;
mod preflight;
mod prefox_faces;
mod prefox_hand_split;
mod prefox_ingame_face;
mod prefox_kit_variants;
mod prefox_kits;
mod prefox_textures;
mod prefox_user_xml;
mod referees;
mod savefile;
mod sideload;
mod sources;
mod test_mode;
mod textures;
mod upgrade;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{Run, Sandbox};
use studio_core::{ExportId, PipelineEvent, Scope};

impl Sandbox {
    fn write(&self, relative: &str, contents: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// Copies `tests/fixtures/sources/<name>` to `<relative folder>/<name>`, leaving the fixture
    /// untouched.
    fn copy_fixture(&self, name: &str, folder: &str) {
        self.write(&format!("{folder}/{name}"), &source_fixture(name));
    }

    /// The platform's spelling of a path inside the sandbox, as a source failure names it.
    fn display(&self, relative: &str) -> String {
        relative
            .split('/')
            .fold(self.root.clone(), |path, segment| path.join(segment))
            .display()
            .to_string()
    }

    /// An absolute path inside the sandbox, as a command-line argument.
    fn arg(&self, relative: &str) -> String {
        self.root.join(relative).to_str().unwrap().to_owned()
    }
}

/// The fields of the run's one `Complete` (`compile` sends it once, `check` never).
struct CompleteCounts {
    success: bool,
    files_written: usize,
    skipped_exports: usize,
    skipped_folders: usize,
    duplicates: usize,
    folders_with_errors: usize,
}

impl Run {
    /// The run's one `Complete`, its fields; panics naming the count found when the stream
    /// holds any other number of them.
    fn complete(&self) -> CompleteCounts {
        let completes: Vec<&PipelineEvent> = self
            .events
            .iter()
            .filter_map(|envelope| match &envelope.event {
                PipelineEvent::Complete { .. } => Some(&envelope.event),
                _ => None,
            })
            .collect();
        let [
            PipelineEvent::Complete {
                success,
                files_written,
                skipped_exports,
                skipped_folders,
                duplicates,
                folders_with_errors,
            },
        ] = completes.as_slice()
        else {
            panic!("exactly one Complete: {}", completes.len());
        };
        CompleteCounts {
            success: *success,
            files_written: *files_written,
            skipped_exports: *skipped_exports,
            skipped_folders: *skipped_folders,
            duplicates: *duplicates,
            folders_with_errors: *folders_with_errors,
        }
    }

    /// The refusal, asserting its exit code and that its message names every one of `names`.
    fn assert_refused(&self, exit_code: u8, names: &[&str]) {
        let error = match &self.result {
            Ok(code) => panic!("not refused: exit code {code}"),
            Err(error) => error,
        };
        let text = error.to_string();
        assert_eq!(error.exit_code, exit_code, "{text}");
        for name in names {
            assert!(
                text.contains(name),
                "the refusal does not name {name}: {text}"
            );
        }
        assert!(self.events.is_empty(), "a refused command emits nothing");
    }

    /// Every `Message` event as one line: the export's file name (from its `ExportStarted`),
    /// severity, code, disposition, scope and context, so a test asserts all of them at once.
    fn messages(&self) -> Vec<String> {
        let mut names: BTreeMap<ExportId, String> = BTreeMap::new();
        let mut lines = Vec::new();
        for envelope in &self.events {
            match &envelope.event {
                PipelineEvent::ExportStarted {
                    export_id,
                    display_name,
                } => {
                    names.insert(*export_id, display_name.clone());
                }
                PipelineEvent::Message(message) => {
                    assert_eq!(envelope.export_id, message.scope.export_id());
                    let source = match message.scope.export_id() {
                        Some(export_id) => format!("{}: ", names[&export_id]),
                        None => String::new(),
                    };
                    let location = match &message.scope {
                        Scope::Run | Scope::Export { .. } => String::new(),
                        Scope::Folder { path, .. } | Scope::File { path, .. } => {
                            format!(" at {}", path.as_str())
                        }
                        Scope::RosterEntry {
                            file, line, slot, ..
                        } => format!(" at {} line {line} slot {slot:?}", file.as_str()),
                    };
                    let context: Vec<String> = message
                        .context
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect();
                    lines.push(format!(
                        "{source}{:?} {} [{:?}]{location} ({})",
                        message.severity,
                        message.code.code,
                        message.disposition,
                        context.join(", ")
                    ));
                }
                PipelineEvent::ExportProcessed { .. }
                | PipelineEvent::FolderStatus { .. }
                | PipelineEvent::Output(_)
                | PipelineEvent::Progress { .. }
                | PipelineEvent::Complete { .. }
                | PipelineEvent::UnmigratedContentFound => {}
            }
        }
        lines
    }
}

/// The first arguments of a run of `command`, `check` or `compile`: `compile` with
/// `--no-deploy`, so it installs nothing and is a clean run, as `check` is.
fn command_args(command: &str) -> Vec<&str> {
    match command {
        "compile" => vec!["compile", "--no-deploy"],
        _ => vec![command],
    }
}

/// The bytes of `tests/fixtures/sources/<name>`, to be written under another name.
fn source_fixture(name: &str) -> Vec<u8> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sources")
        .join(name);
    fs::read(fixture).unwrap()
}

/// Every file under `root`, by relative path, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let bytes = fs::read(&path).unwrap();
                files.insert(path.strip_prefix(root).unwrap().to_path_buf(), bytes);
            }
        }
    }
    files
}

/// The lines of `lines` about `source`, without their `<source>: ` prefix, so the findings of
/// sources holding the same export compare equal.
fn findings_of<'a>(lines: &'a [String], source: &str) -> Vec<&'a str> {
    let prefix = format!("{source}: ");
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(prefix.as_str()))
        .collect()
}

/// One face of a model: its three vertex positions in the order the face lists its vertices,
/// each coordinate as its `f32` bit pattern, so a face is the same whatever its vertices'
/// indices, and a face whose winding is flipped is another face.
type Face = [[u32; 3]; 3];

/// The faces of the meshes `meshes`, each given as its (positions, faces), sorted: compared
/// with another model's, the multiset of the faces each draws.
fn sorted_faces<'a>(meshes: impl Iterator<Item = (&'a [[f32; 3]], &'a [[u16; 3]])>) -> Vec<Face> {
    let mut faces: Vec<Face> = meshes
        .flat_map(|(positions, faces)| {
            faces
                .iter()
                .map(|face| face.map(|index| positions[usize::from(index)].map(f32::to_bits)))
        })
        .collect();
    faces.sort_unstable();
    faces
}

/// The findings a PES 21 `compile --no-deploy` reports first when its PES folder does not
/// exist: each working bin taken from its bundled base.
const BUNDLED_BINS: [&str; 3] = [
    "Info bin_source [Keep] (bin=TeamColor.bin, cpk=bundled)",
    "Info bin_source [Keep] (bin=UniColor.bin, cpk=bundled)",
    "Info bin_source [Keep] (bin=UniformParameter.bin, cpk=bundled)",
];

/// `BUNDLED_BINS`, then `findings`: every line of a PES 21 `compile` with no PES folder whose
/// exports report `findings`.
fn bundled_bins_then(findings: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    BUNDLED_BINS
        .iter()
        .map(|line| (*line).to_owned())
        .chain(findings.into_iter().map(Into::into))
        .collect()
}

/// The note a `compile --no-deploy` in `sandbox` that writes a CPK ends with, naming the CPK it
/// leaves in the output folder.
fn deploy_skipped(sandbox: &Sandbox) -> String {
    format!(
        "Info deploy_skipped_by_flag [Keep] (path={})",
        sandbox.display("output/4cc_99_test.cpk")
    )
}

/// The `patch_written` line a Fox compile that publishes reports, naming `sandbox`'s output
/// patch and `teams` (`"714"`, `"714, 792"`).
fn patch_written(sandbox: &Sandbox, teams: &str) -> String {
    format!(
        "Info patch_written [Keep] (path={}, teams={teams})",
        sandbox.display("output/aesthetics_patch.toml")
    )
}

/// The `savefile_missing` warning a deployed or sideloaded Fox compile whose save the
/// sandbox lacks reports after `patch_written`, naming the output patch and the sandbox's
/// `savefile_path`.
fn savefile_missing(sandbox: &Sandbox) -> String {
    format!(
        "Warning savefile_missing [Keep] (patch={}, path={}, command={} save-editor apply-patch)",
        sandbox.display("output/aesthetics_patch.toml"),
        sandbox.display("save/EDIT00000000"),
        studio_core::EXE_NAME
    )
}

/// `bundled_bins_then(findings)`, then `deploy_skipped(sandbox)`: every line of a PES 21
/// `compile --no-deploy` in `sandbox` with no PES folder that writes a CPK, whose exports report
/// `findings`.
fn no_deploy_lines(
    sandbox: &Sandbox,
    findings: impl IntoIterator<Item = impl Into<String>>,
) -> Vec<String> {
    let mut lines = bundled_bins_then(findings);
    lines.push(deploy_skipped(sandbox));
    lines
}

/// The note `compile` adds for a team export with no root `colors.txt`, which `check` does not
/// print: a test comparing both commands' findings appends it for `compile`.
const TEAM_COLORS_MISSING: &str = "Info team_colors_missing [Keep] ()";

/// One player folder with a face model: written with `clean_model()`, an export that validates
/// with no finding.
const CLEAN_PLAYER: &str = "Players/03 - A/face_high.fmdl";

/// A Fox model in which `fmdl`'s check finds nothing (the tracer's right glove), for a test
/// whose model only has to be one: both commands read every model, so an empty file would be
/// `model_broken`. Its hand bones are renamed from `skh_` to `skx_`, so a face model written
/// with it is not hand auto-split: the glove's vertices weigh on its finger bones, which would
/// give the player gloves and a `model_hand_split` the test is not about.
fn clean_model() -> Vec<u8> {
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
