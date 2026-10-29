//! Parity against the compiled output for the same export: compile
//! `fixtures/tracer/studio/egg Midcup Tracer`, the upgrader's output for
//! `fixtures/tracer/old/egg Tracer` (`fixtures/tracer/upgraded/`, copied under
//! the `Midcup` tag), and the referee golden `fixtures/tracer_refs/upgraded/`
//! through the `compile` command and compare the CPKs with the reference
//! trees `fixtures/tracer/red/` and `fixtures/tracer_refs/red/`, one explicit
//! row per reference entry under the tiers of
//! `docs/spec/team_compiler/testing.md` "Testing: parity against Red (adapted from Blue)".

mod common;
mod upgraded;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use common::Sandbox;
use fpk::FpkFile;
use studio_core::{PipelineEvent, Severity};
use uniparam::UniformParameter;
use upgraded::upgraded_as_midcup;

/// How one entry of the reference tree is compared.
enum Row {
    /// Tier 1 leaf: same bytes (`Row` carries no tier of its own; leaf entries
    /// compare per `compare_leaf`).
    Exact,
    /// Byte-identical at a different path (the path-mapping of tier 1/2).
    Relocated(&'static str),
    /// An FPK whose entry-name set must match (tier 3); contents are checked
    /// per member in `compare_fpk`.
    Container,
    /// `Container` at a different path: the planned ID replacing the ID the
    /// old export's folder name carried.
    RelocatedContainer(&'static str),
    /// Not produced yet (tier 4): the reason.
    NotProduced(&'static str),
}

/// The per-player common texture every one of the player's models points at.
const COMMON_SHIRT: &str = "Asset/model/character/common/792/05 - The Chad Stormworks Player/sourceimages/#windx11/shirt.ftex";

/// The egg tracer's relocated file names: the per-player common texture,
/// relocated by the compiler where Red packed it beside the model.
const EGG_RELOCATED: &[&str] = &["shirt.dds"];

/// One row per reference entry. The produced rows are also the allowlist our
/// CPK's entries are checked against. Red compiled the boots and gloves under
/// the ID their folders were named with, `2180`; the Studio format has no
/// ID-named folders and slot 05 of team 792 owns the planned ID 3745.
const TABLE: &[(&str, Row)] = &[
    (
        "Asset/model/character/boots/k2180/#Win/boots.fpk",
        Row::RelocatedContainer("Asset/model/character/boots/k3745/#Win/boots.fpk"),
    ),
    (
        "Asset/model/character/boots/k2180/#Win/boots.fpkd",
        Row::Relocated("Asset/model/character/boots/k3745/#Win/boots.fpkd"),
    ),
    (
        "Asset/model/character/boots/k2180/#windx11/boots.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    // The same texture the face uses: one entry serves both.
    (
        "Asset/model/character/boots/k2180/#windx11/shirt.ftex",
        Row::Relocated(COMMON_SHIRT),
    ),
    (
        "Asset/model/character/face/real/79205/#Win/face.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/face/real/79205/#Win/face.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/face/real/79205/sourceimages/#windx11/face.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/face/real/79205/sourceimages/#windx11/shirt.ftex",
        Row::Relocated(COMMON_SHIRT),
    ),
    (
        "Asset/model/character/glove/g2180/#Win/glove.fpk",
        Row::RelocatedContainer("Asset/model/character/glove/g3745/#Win/glove.fpk"),
    ),
    (
        "Asset/model/character/glove/g2180/#Win/glove.fpkd",
        Row::Relocated("Asset/model/character/glove/g3745/#Win/glove.fpkd"),
    ),
    (
        "Asset/model/character/glove/g2180/#windx11/glove.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/u0792g1.ftex",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/792/792_DEF_GK1st_realUni.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/UniColor.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/UniformParameter.bin",
        Row::Exact,
    ),
    ("common/etc/TeamColor.bin", Row::Exact),
    ("common/render/symbol/player/79205.dds", Row::Exact),
];

/// The `NotProduced` reason of a referee slot's face rows.
const NO_REFEREE_FACE: &str = "the compiler writes no face for a referee whose folder holds no face model, so the game's own referee head stays, Red packing face_diff.bin alone";

/// One row per reference entry of the referee tracer's `red/` tree. Red and
/// the compiler both point the referee's textures at his name-keyed common
/// folder `common/999/masterchef/`, so the models compare decoded with no
/// directory normalized and the textures sit at Red's own paths (hence no
/// `Relocated` rows).
const REFS_TABLE: &[(&str, Row)] = &[
    (
        "Asset/model/character/boots/k9905/#Win/boots.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/boots/k9905/#Win/boots.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/boots/k9905/#windx11/boots.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/boots/k9923/#Win/boots.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/boots/k9923/#Win/boots.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/boots/k9923/#windx11/boots.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/boots/k9924/#Win/boots.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/boots/k9924/#Win/boots.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/boots/k9924/#windx11/boots.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/common/999/masterchef/sourceimages/#windx11/arm_1_bsm.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/common/999/masterchef/sourceimages/#windx11/hat.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/common/999/masterchef/sourceimages/#windx11/masterchef.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/common/999/masterchef/sourceimages/#windx11/normal.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/common/999/masterchef/sourceimages/#windx11/spec.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/face/real/referee005/#Win/face.fpk",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee005/#Win/face.fpkd",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee005/sourceimages/#windx11/face.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/face/real/referee023/#Win/face.fpk",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee023/#Win/face.fpkd",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee023/sourceimages/#windx11/face.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/face/real/referee024/#Win/face.fpk",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee024/#Win/face.fpkd",
        Row::NotProduced(NO_REFEREE_FACE),
    ),
    (
        "Asset/model/character/face/real/referee024/sourceimages/#windx11/face.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/glove/g9905/#Win/glove.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/glove/g9905/#Win/glove.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/glove/g9905/#windx11/glove.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/glove/g9923/#Win/glove.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/glove/g9923/#Win/glove.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/glove/g9923/#windx11/glove.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/glove/g9924/#Win/glove.fpk",
        Row::Container,
    ),
    (
        "Asset/model/character/glove/g9924/#Win/glove.fpkd",
        Row::Exact,
    ),
    (
        "Asset/model/character/glove/g9924/#windx11/glove.fpk.xml",
        Row::NotProduced("the Studio format drops the source-side fpk.xml"),
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_1.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_1_srm.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_2.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_2_srm.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_3.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_3_srm.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_4.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_4_srm.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_5.ftex",
        Row::Exact,
    ),
    (
        "Asset/model/character/uniform/texture/#windx11/referee_5_srm.ftex",
        Row::Exact,
    ),
    (
        "common/character0/model/character/appearance/RefereeAppearance.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_ACL_1.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_ACL_2.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_ACL_3.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_ACL_4.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_ACL_5.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_CL_1.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_CL_2.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_CL_3.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_CL_4.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_DEF_1.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_DEF_2.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_DEF_3.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_DEF_4.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_DEF_5.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_LB_1.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_LB_2.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_LB_3.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_SDA_1.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_SDA_2.bin",
        Row::Exact,
    ),
    (
        "common/character0/model/character/uniform/team/referee/referee_SDA_3.bin",
        Row::Exact,
    ),
];

/// The paths of the team CPK a refs export compiles beside the referees CPK:
/// Red writes no team CPK for a referee export; on PES 18-21 the compiler
/// writes the team side for the bins alone, each referee kit config being
/// also its entry in `UniformParameter.bin`
/// (`docs/spec/team_compiler/pipeline.md` "5. Writer (single-threaded)", the
/// item starting "**Output modes and targets**").
const REFS_TEAM_SIDE: &[&str] = &[
    "common/character0/model/character/uniform/team/UniColor.bin",
    "common/character0/model/character/uniform/team/UniformParameter.bin",
    "common/etc/TeamColor.bin",
];

/// The folder of the referee kit configs in the reference tree; each file's
/// own name is its entry name in `UniformParameter.bin`.
const REFEREE_CONFIGS: &str = "common/character0/model/character/uniform/team/referee/";

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                pending.push(entry.path());
            } else {
                let path = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(path, fs::read(entry.path()).unwrap());
            }
        }
    }
    files
}

fn archive(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut cpk = cpk::CpkArchive::open(fs::File::open(path).unwrap()).unwrap();
    let entries = cpk.entries().to_vec();
    let mut files = BTreeMap::new();
    for entry in &entries {
        files.insert(entry.path.clone(), cpk.read(entry).unwrap());
    }
    files
}

/// One leaf entry: an `.ftex` compares by what `ftex_to_dds` returns for each
/// side (tier 2 — the chunk compression is a deflate encoder's choice, and
/// `ftex` declares byte identity with zlib's output a non-goal); anything else
/// compares by bytes (tier 1).
fn compare_leaf(failures: &mut Vec<String>, name: &str, ours: Option<&[u8]>, reference: &[u8]) {
    match ours {
        None => failures.push(format!("{name}: missing from our CPK")),
        Some(our_bytes) if name.ends_with(".ftex") => {
            let (our_info, reference_info) = (
                ftex::info(our_bytes).unwrap(),
                ftex::info(reference).unwrap(),
            );
            if our_info != reference_info {
                failures.push(format!(
                    "tier2 {name}: headers differ (ours {our_info:?}, reference {reference_info:?})"
                ));
            }
            if ftex::ftex_to_dds(our_bytes).unwrap() != ftex::ftex_to_dds(reference).unwrap() {
                failures.push(format!("tier2 {name}: decoded DDS content differs"));
            }
        }
        Some(our_bytes) => {
            if our_bytes != reference {
                failures.push(format!("tier1 {name}: bytes differ"));
            }
        }
    }
}

/// A relocated directory is an intentional layout difference (per-player
/// common): the directory of each texture `relocated` names is normalized
/// away on both sides before the decoded comparison. The other textures'
/// directories are kept literal so the team-ID substitution in them is still
/// checked.
fn decoded_model(bytes: &[u8], relocated: &[&str]) -> fmdl::Model {
    let file = fmdl::FmdlFile::read(bytes).unwrap();
    let mut model = fmdl::Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        for (_, texture) in &mut material.textures {
            if relocated.contains(&texture.file_name.as_str()) {
                texture.directory = "<relocated>".to_owned();
            }
        }
    }
    model
}

/// Every `relocated` texture reference of the FMDL `bytes` resolves to a file
/// of `compiled`: rewritten references resolve to the relocated files. The
/// reference's directory has the FMDL
/// form `/Assets/pes16/<folder>/`; the emitted texture sits at the CPK path
/// `Asset/<folder>/#windx11/<stem>.ftex`.
fn check_resolved(
    failures: &mut Vec<String>,
    name: &str,
    bytes: &[u8],
    compiled: &BTreeMap<String, Vec<u8>>,
    relocated: &[&str],
) {
    let file = fmdl::FmdlFile::read(bytes).unwrap();
    let model = fmdl::Model::from_file(&file).unwrap();
    for material in &model.materials {
        for (_, texture) in &material.textures {
            if !relocated.contains(&texture.file_name.as_str()) {
                continue;
            }
            let stem = Path::new(&texture.file_name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(texture.file_name.as_str());
            let directory = texture
                .directory
                .strip_prefix("/Assets/pes16")
                .unwrap_or(texture.directory.as_str());
            let resolved = format!("Asset{directory}#windx11/{stem}.ftex");
            if !compiled.contains_key(&resolved) {
                failures.push(format!(
                    "{name}: {}{} resolves to {resolved}, not in our CPK",
                    texture.directory, texture.file_name
                ));
            }
        }
    }
}

fn compare_fpk(
    failures: &mut Vec<String>,
    name: &str,
    ours: &[u8],
    reference: &[u8],
    compiled: &BTreeMap<String, Vec<u8>>,
    relocated: &[&str],
) {
    let ours = FpkFile::read(ours).unwrap();
    let reference = FpkFile::read(reference).unwrap();
    let our_names: BTreeSet<&str> = ours.entries().map(|(name, _)| name).collect();
    let reference_names: BTreeSet<&str> = reference.entries().map(|(name, _)| name).collect();
    if our_names != reference_names {
        failures.push(format!(
            "tier3 {name}: entry-name set differs (ours {our_names:?}, reference {reference_names:?})"
        ));
    }
    for (entry, reference_bytes) in reference.entries() {
        let member = format!("{name}/{entry}");
        match ours.get(entry) {
            None => failures.push(format!("tier3 {member}: missing")),
            Some(our_bytes) if entry.ends_with(".fmdl") => {
                check_resolved(failures, &member, our_bytes, compiled, relocated);
                if decoded_model(our_bytes, relocated) != decoded_model(reference_bytes, relocated)
                {
                    failures.push(format!("tier2 {member}: decoded models differ"));
                }
            }
            Some(our_bytes) => compare_leaf(failures, &member, Some(our_bytes), reference_bytes),
        }
    }
}

/// The per-row comparison of `ours` against `reference` per `table`, then the
/// completeness checks both ways: every entry of `ours` a produced row does
/// not cover is a failure, and every file of `reference` the table does not
/// list is one.
fn compare_table(
    failures: &mut Vec<String>,
    table: &[(&str, Row)],
    ours: &BTreeMap<String, Vec<u8>>,
    reference: &BTreeMap<String, Vec<u8>>,
    relocated: &[&str],
) {
    for (path, row) in table {
        let reference_bytes = &reference[*path];
        match row {
            Row::Exact => compare_leaf(
                failures,
                path,
                ours.get(*path).map(Vec::as_slice),
                reference_bytes,
            ),
            Row::Relocated(our_path) => compare_leaf(
                failures,
                path,
                ours.get(*our_path).map(Vec::as_slice),
                reference_bytes,
            ),
            Row::Container => match ours.get(*path) {
                None => failures.push(format!("tier3 {path}: missing from our CPK")),
                Some(our_bytes) => {
                    compare_fpk(failures, path, our_bytes, reference_bytes, ours, relocated)
                }
            },
            Row::RelocatedContainer(our_path) => match ours.get(*our_path) {
                None => failures.push(format!("tier3 {path}: missing from our CPK")),
                Some(our_bytes) => {
                    compare_fpk(failures, path, our_bytes, reference_bytes, ours, relocated)
                }
            },
            Row::NotProduced(reason) => {
                if ours.contains_key(*path) {
                    failures.push(format!("tier4 {path}: produced anyway ({reason})"));
                }
            }
        }
    }
    for path in ours.keys() {
        let produced = table.iter().any(|(reference_path, row)| match row {
            Row::Relocated(our_path) | Row::RelocatedContainer(our_path) => path == our_path,
            Row::NotProduced(_) => false,
            Row::Exact | Row::Container => path == reference_path,
        });
        if !produced {
            failures.push(format!(
                "our CPK holds {path}, not in the table's produced rows"
            ));
        }
    }
    for path in reference.keys() {
        if !table.iter().any(|(row_path, _)| path == row_path) {
            failures.push(format!(
                "reference holds {path}, which the table does not list"
            ));
        }
    }
}

/// Compiles the one export under `exports_root` and compares the CPK with the
/// reference tree, one explicit row per reference entry.
fn assert_matches_reference(sandbox: &Sandbox, exports_root: &Path) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer");
    let output = sandbox.root.join("parity output");
    let settings = format!(
        "[common]\npes_version = 21\n[team-compiler]\noutput_folder_path = '{}'\n",
        output.display()
    );
    // Nothing to install into: the reference is the CPK itself.
    let run = sandbox.run(
        &settings,
        &["compile", "--no-deploy", exports_root.to_str().unwrap()],
    );
    assert_eq!(run.exit_code(), 0);
    // Notes only, but for the three player tables: with no PES install there is no table to
    // build on, so the tracer's boots, gloves and appearance rows are left out, as Red, which
    // wrote no table, never had them.
    let mut warnings = Vec::new();
    for envelope in &run.events {
        if let PipelineEvent::Message(message) = &envelope.event
            && message.severity >= Severity::Warning
        {
            warnings.push((message.code.code.to_string(), message.context.clone()));
        }
    }
    let table_missing = |table: &str| {
        (
            "player_table_missing".to_owned(),
            vec![
                ("table".to_owned(), table.to_owned()),
                ("rows".to_owned(), "1".to_owned()),
            ],
        )
    };
    assert_eq!(
        warnings,
        [
            table_missing("BootsList.bin"),
            table_missing("GloveList.bin"),
            table_missing("PlayerAppearance.bin")
        ],
        "the tracer compiles with notes only, but for the three player tables"
    );

    let ours = archive(&output.join("4cc_99_test.cpk"));
    let reference = tree(&fixture.join("red"));

    // The team's shared-IDs record, at a path no game reads: a Midcup over no installed CPK
    // with no shared folder taking an ID writes none, and the tracer links none.
    assert!(!ours.contains_key("unlimited-tool-works/shared_ids/792.toml"));
    // The compiler's own records, at paths no game reads.
    let ours: BTreeMap<_, _> = ours
        .into_iter()
        .filter(|(path, _)| !path.starts_with("unlimited-tool-works/"))
        .collect();
    let mut failures = Vec::new();
    compare_table(&mut failures, TABLE, &ours, &reference, EGG_RELOCATED);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_tracer_bullet_matches_the_reference_tree() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer");
    let sandbox = Sandbox::new("parity");
    assert_matches_reference(&sandbox, &fixture.join("studio"));
}

#[test]
fn the_upgraded_tracer_matches_the_reference_tree() {
    let sandbox = Sandbox::new("parity_upgraded");
    let exports_root = upgraded_as_midcup(&sandbox, "tracer");
    assert_matches_reference(&sandbox, &exports_root);
}

#[test]
fn the_upgraded_referee_tracer_matches_the_reference_tree() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tracer_refs");
    let sandbox = Sandbox::new("parity_refs");
    let output = sandbox.root.join("parity output");
    let settings = format!(
        "[common]\npes_version = 21\n[team-compiler]\noutput_folder_path = '{}'\n",
        output.display()
    );
    // The golden compiles in place: a referee export takes no coverage tag.
    let run = sandbox.run(
        &settings,
        &[
            "compile",
            "--no-deploy",
            fixture.join("upgraded").to_str().unwrap(),
        ],
    );
    assert_eq!(run.exit_code(), 0);
    let mut warnings = Vec::new();
    let mut face_not_used = false;
    for envelope in &run.events {
        if let PipelineEvent::Message(message) = &envelope.event {
            if message.severity >= Severity::Warning {
                warnings.push((message.code.code.to_string(), message.context.clone()));
            }
            face_not_used |= message.code.code.as_ref() == "face_file_not_used"
                && message.severity == Severity::Info
                && message.context == [("file".to_owned(), "face_diff.bin".to_owned())];
        }
    }
    assert!(
        warnings.is_empty(),
        "the referee tracer compiles with notes only: {warnings:?}"
    );
    // What the face rows' reason says the member is told.
    assert!(face_not_used, "face_diff.bin is reported not used");

    let ours = archive(&output.join("4cc_18_referees.cpk"));
    let reference = tree(&fixture.join("red"));

    let mut failures = Vec::new();
    compare_table(&mut failures, REFS_TABLE, &ours, &reference, &[]);

    let team = archive(&output.join("4cc_99_test.cpk"));
    let paths: BTreeSet<&str> = team.keys().map(String::as_str).collect();
    let expected_paths: BTreeSet<&str> = REFS_TEAM_SIDE.iter().copied().collect();
    if paths != expected_paths {
        failures.push(format!(
            "team CPK paths {paths:?}, expected {expected_paths:?}"
        ));
    }
    let bins = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/bins");
    for (path, name) in [
        ("common/etc/TeamColor.bin", "TeamColor.bin"),
        (
            "common/character0/model/character/uniform/team/UniColor.bin",
            "UniColor.bin",
        ),
    ] {
        if team[path] != fs::read(bins.join(name)).unwrap() {
            failures.push(format!("team CPK {path}: not the bundled {name}"));
        }
    }
    // The bundled base with every one of the reference's referee kit configs
    // inserted under its file name.
    let mut expected =
        UniformParameter::read(&fs::read(bins.join("UniformParameter19.bin")).unwrap()).unwrap();
    for (path, bytes) in &reference {
        if let Some(name) = path.strip_prefix(REFEREE_CONFIGS) {
            expected.insert(name.to_owned(), bytes.clone()).unwrap();
        }
    }
    let uniform_parameter = UniformParameter::read(
        &team["common/character0/model/character/uniform/team/UniformParameter.bin"],
    )
    .unwrap();
    let our_names: BTreeSet<&str> = uniform_parameter.entries().map(|(name, _)| name).collect();
    let expected_names: BTreeSet<&str> = expected.entries().map(|(name, _)| name).collect();
    for name in expected_names.difference(&our_names) {
        failures.push(format!("UniformParameter.bin: entry {name} missing"));
    }
    for name in our_names.difference(&expected_names) {
        failures.push(format!("UniformParameter.bin: unexpected entry {name}"));
    }
    for (name, content) in expected.entries() {
        if let Some(ours) = uniform_parameter.get(name)
            && ours != content
        {
            failures.push(format!("UniformParameter.bin: entry {name} differs"));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
