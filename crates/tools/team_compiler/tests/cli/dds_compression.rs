//! The `dds_compression` setting (`spec/team_compiler/settings.md`, "DDS compression cost"): on
//! PES 2015 to 2017 every DDS a compile writes is WESYS-zlibbed when the setting is on, or
//! `auto` with `multicpk_mode` on, a DDS the export already holds wrapped written as it is; PES
//! 2018 to 2021 ignore it.

use std::collections::BTreeMap;
use std::fs;

use pes_version::PesVersion;

use crate::common::Sandbox;
use crate::compile::{cpk_entries, pes_settings, tracer_portrait};
use crate::prefox_faces::{card_materials, card_model, small_dds};
use crate::prefox_kits::write_kit;

/// The export every run compiles: team 714's.
const EXPORT: &str = "co Full Compression";

/// The CPK path of slot 05's `skin.dds`, in the player's texture home.
const SKIN: &str = "common/character1/model/character/uniform/common/714/05 - A/skin.dds";

/// The CPK path of slot 05's `hair.dds`, beside `skin.dds`.
const HAIR: &str = "common/character1/model/character/uniform/common/714/05 - A/hair.dds";

/// The CPK paths of the kits' main textures and of the masks they are given: the export's
/// p1 and the empty `g1/` a `Full` export plans.
const KIT: [&str; 4] = [
    "common/character0/model/character/uniform/texture/u0714p1.dds",
    "common/character0/model/character/uniform/texture/u0714p1_mask.dds",
    "common/character0/model/character/uniform/texture/u0714g1.dds",
    "common/character0/model/character/uniform/texture/u0714g1_mask.dds",
];

/// The CPK path of slot 05's portrait on PES 17.
const PORTRAIT: &str = "common/render/symbol/player/player_71405.dds";

/// The CPK path of slot 05's portrait on PES 21.
const FOX_PORTRAIT: &str = "common/render/symbol/player/71405.dds";

/// `hair.dds` as the export holds it: the small DDS, already WESYS-wrapped.
fn wrapped_hair() -> Vec<u8> {
    wezlib::compress(&small_dds())
}

/// A sandbox `name` holding the export, completed for `version`: slot 05 with `skin.dds`,
/// `hair.dds` already wrapped, which the folder's textures carry whether or not a material
/// names it, and a portrait; with `face_model`, also `face_high.model` and its `.mtl` naming
/// `skin.dds`; and a kit `p1` with its main texture.
fn sandbox_with_export(name: &str, version: PesVersion, face_model: bool) -> Sandbox {
    let sandbox = Sandbox::new(name);
    let player = format!("exports/{EXPORT}/Players/05 - A");
    if face_model {
        sandbox.write(&format!("{player}/face_high.model"), &card_model());
        sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
    }
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/hair.dds"), &wrapped_hair());
    sandbox.write(&format!("{player}/portrait.dds"), &tracer_portrait());
    write_kit(&sandbox, EXPORT, "p1");
    sandbox.complete_for(&format!("exports/{EXPORT}"), version, &[]);
    sandbox
}

/// Compiles the export with `compile --no-deploy` for PES `version` with the `[compiler]`
/// lines `team_compiler`, in a sandbox of its own, `name`, and asserts the run succeeded. A
/// PES 21 export holds no face model: the textures are what the tests here compare, and the
/// `.model` face would only add a conversion to the run.
fn compile(name: &str, version: u8, team_compiler: &str) -> Sandbox {
    let target = PesVersion::try_from(u16::from(version)).unwrap();
    let sandbox = sandbox_with_export(name, target, version == 17);
    let settings = format!(
        "{}[compiler]\n{team_compiler}",
        pes_settings(&sandbox, version)
    );
    let run = sandbox.run(&settings, &["compile", "--no-deploy"]);
    assert_eq!(run.exit_code(), 0, "{:#?}", run.messages());
    sandbox
}

/// The `.dds` entries of every CPK the run in `sandbox` wrote, by path.
fn written_dds(sandbox: &Sandbox) -> BTreeMap<String, Vec<u8>> {
    let mut entries = BTreeMap::new();
    for file in fs::read_dir(sandbox.root.join("output")).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "cpk") {
            entries.extend(dds_entries(cpk_entries(&path)));
        }
    }
    entries
}

/// The `.dds` entries of `entries`.
fn dds_entries(entries: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    entries
        .into_iter()
        .filter(|(path, _)| path.ends_with(".dds"))
        .collect()
}

/// Asserts that `wrapped`'s `.dds` entries are `plain`'s, each but `hair.dds` WESYS-wrapped
/// and decompressing to `plain`'s bytes, `hair.dds` the source's bytes in both.
fn assert_wrapped_as(wrapped: &BTreeMap<String, Vec<u8>>, plain: &BTreeMap<String, Vec<u8>>) {
    assert_eq!(
        wrapped.keys().collect::<Vec<_>>(),
        plain.keys().collect::<Vec<_>>()
    );
    for (path, bytes) in wrapped {
        if path == HAIR {
            assert_eq!(bytes, &wrapped_hair(), "hair.dds is emitted as it is");
            assert_eq!(plain[path], wrapped_hair(), "hair.dds is emitted as it is");
            continue;
        }
        assert!(wezlib::is_wrapped(bytes), "{path} is not wrapped");
        assert_eq!(
            wezlib::decompress(bytes).unwrap(),
            plain[path],
            "{path} decompresses to the plain run's bytes"
        );
    }
}

// TC-TEX-08
#[test]
fn pes_17_wraps_every_dds_when_dds_compression_resolves_on_and_pes_21_ignores_it() {
    let on = written_dds(&compile(
        "dds_compression_17_on",
        17,
        "dds_compression = true\n",
    ));
    let off = written_dds(&compile(
        "dds_compression_17_off",
        17,
        "dds_compression = false\n",
    ));
    let auto_single = written_dds(&compile(
        "dds_compression_17_auto_single",
        17,
        "dds_compression = \"auto\"\nmulticpk_mode = false\n",
    ));
    let auto_multi = written_dds(&compile(
        "dds_compression_17_auto_multi",
        17,
        "dds_compression = \"auto\"\nmulticpk_mode = true\n",
    ));

    for path in [SKIN, HAIR, PORTRAIT].into_iter().chain(KIT) {
        assert!(off.contains_key(path), "{path} in {:#?}", off.keys());
    }
    assert!(
        !wezlib::is_wrapped(&off[SKIN]),
        "a plain source stays plain with the setting off"
    );
    assert_wrapped_as(&on, &off);
    assert_eq!(auto_single, off, "auto without multicpk_mode is off");
    assert_wrapped_as(&auto_multi, &off);

    let fox_on = compile("dds_compression_21_on", 21, "dds_compression = true\n");
    let fox_off = compile("dds_compression_21_off", 21, "dds_compression = false\n");
    // The portrait is a DDS on every version, so the PES 21 CPK holds one, plain.
    let fox_dds = written_dds(&fox_on);
    assert!(fox_dds.contains_key(FOX_PORTRAIT), "{:#?}", fox_dds.keys());
    assert!(!wezlib::is_wrapped(&fox_dds[FOX_PORTRAIT]));
    let cpk = |sandbox: &Sandbox| fs::read(sandbox.root.join("output/4cc_99_test.cpk")).unwrap();
    assert!(
        cpk(&fox_on) == cpk(&fox_off),
        "the PES 21 runs write the same CPK whatever the setting"
    );
}

// TC-CLI-05
#[test]
fn a_dds_compression_other_than_true_false_or_auto_is_a_configuration_error_naming_it() {
    for (name, value) in [("integer", "1"), ("word", "\"yes\"")] {
        let sandbox = Sandbox::new(&format!("dds_compression_invalid_{name}"));
        let settings = format!(
            "{}[compiler]\ndds_compression = {value}\n",
            pes_settings(&sandbox, 17)
        );
        let run = sandbox.run(&settings, &["check"]);
        run.assert_refused(2, &["the [compiler] settings", "dds_compression"]);
    }
}
