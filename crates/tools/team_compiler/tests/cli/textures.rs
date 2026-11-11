//! `compile` over textures in every accepted image format
//! (`spec/team_compiler/pipeline.md` "Texture conversion"): a player folder's, a shared
//! folder's, Common's and a kit's, each converted for the target version's codec with its mip
//! chain, observed through the emitted FTEX's header and blocks. Also the texture findings the
//! deep pass reports from their headers, at `check` and at `compile`.

use std::fs;
use std::path::Path;

use dds_convert::{
    BlockCodec, Blocks, Decoded, SourceFormat, Target, TextureRole, decode, encode_dds, encode_png,
};
use fmdl::ops::paths::{rewrite_texture_paths, texture_paths, used_texture_paths};
use fmdl::{FmdlFile, Model};
use ftex::PixelFormat;
use pes_version::PesVersion;

use crate::bins::{install_cpk, install_names};
use crate::common::Sandbox;
use crate::common_links::{model_naming_in, texture_directories};
use crate::compile::{
    compiled_kits, compiled_players, cpk_entries, kit_texture, pass_through_settings, pes_settings,
    pes21_settings, pixels_cut_dds, tracer_kit, tracer_player_file,
};
use crate::deploy::templates_folder;
use crate::models::face_package;
use crate::prefox_faces::{
    CLEAN, card_materials, card_model, compile_pes17, face_cpk, face_folder, nested_entries,
    sampler_paths, small_dds,
};
use crate::{CLEAN_PLAYER, clean_model, command_args, findings_of};
use pes_model::format::mtl::MaterialEntry;

/// The player's texture home, the per-player common subfolder.
const PLAYER_TEXTURES: &str = "Asset/model/character/common/714/05 - A/sourceimages/#windx11";

/// The bytes of `tests/fixtures/textures/<name>`, described in
/// `tests/fixtures/textures/README.md`.
pub(crate) fn texture_fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/textures")
            .join(name),
    )
    .unwrap()
}

/// The tracer's hair model naming the texture `file_name` (`skin.dds`) instead of `shirt.dds`,
/// through `fmdl`'s model API.
pub(crate) fn hair_model_naming(file_name: &str) -> Vec<u8> {
    let file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        for (_, texture) in &mut material.textures {
            if texture.file_name == "shirt.dds" {
                texture.file_name = file_name.to_owned();
            }
        }
    }
    model.to_file().unwrap().write()
}

/// Writes slot `slot` of `exports/<export>`, the folder `<slot> - A`, as the hair model naming
/// `skin` beside the texture file `texture_name` holding `bytes`.
fn write_skin_player(
    sandbox: &Sandbox,
    export: &str,
    slot: &str,
    texture_name: &str,
    bytes: &[u8],
) {
    let player = format!("exports/{export}/Players/{slot} - A");
    sandbox.write(
        &format!("{player}/fcl_hair.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(&format!("{player}/{texture_name}"), bytes);
}

/// Compiles the sandbox for PES `version`, asserting the export `name`, slot 05 written by
/// `write_skin_player`, reports only the hair model's unnormalized weights and its identity,
/// and returns the entry at `path`.
fn compile_entry(sandbox: &Sandbox, name: &str, version: u8, path: &str) -> Vec<u8> {
    let run = sandbox.run(&pes_settings(sandbox, version), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&run.messages(), name),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ],
        "PES {version}"
    );
    assert_eq!(run.exit_code(), 0, "PES {version}");
    let mut entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    entries
        .remove(path)
        .unwrap_or_else(|| panic!("PES {version}: no {path} among {:?}", entries.keys()))
}

/// The emitted FTEX `bytes` converted back to a DDS and decoded: asserts the codec, the
/// 1024x1024 size and the full 11-level chain on the *payload*, which `ftex::info`'s
/// header-only read cannot see (TC-TEX-01, TC-TEX-10).
fn assert_decodes_as(bytes: &[u8], codec: BlockCodec, label: &str) {
    let decoded = decode(&ftex::ftex_to_dds(bytes).unwrap(), SourceFormat::Dds).unwrap();
    let blocks = decoded.blocks.as_ref().unwrap();
    assert_eq!(blocks.codec, codec, "{label}");
    assert_eq!((decoded.width, decoded.height), (1024, 1024), "{label}");
    assert_eq!(decoded.mips.len(), 11, "{label}: 1024x1024 down to 1x1");
    assert_eq!(blocks.mips.len(), 11, "{label}: a block buffer per level");
}

// TC-TEX-01
#[test]
fn a_png_skin_is_bc3_with_a_full_mip_chain_on_pes_21_and_pes_18() {
    let sandbox = Sandbox::new("tex_png_skin");
    write_skin_player(
        &sandbox,
        "co Midcup Skin",
        "05",
        "skin.png",
        &texture_fixture("skin.png"),
    );
    let skin = format!("{PLAYER_TEXTURES}/skin.ftex");

    for (version, format, codec) in [
        (21, PixelFormat::Bc3, BlockCodec::Bc3),
        (18, PixelFormat::Bc3, BlockCodec::Bc3),
    ] {
        let emitted = compile_entry(&sandbox, "co Midcup Skin", version, &skin);
        let info = ftex::info(&emitted).unwrap();
        assert_eq!(info.format, format, "PES {version}");
        assert_eq!(info.mipmaps, 11, "PES {version}: 1024x1024 down to 1x1");
        assert_eq!((info.width, info.height), (1024, 1024), "PES {version}");
        assert_decodes_as(&emitted, codec, &format!("PES {version}"));
    }
}

// TC-TEX-02
#[test]
fn a_bc7_dds_skin_keeps_its_blocks_on_pes_21_and_is_bc3_on_pes_18() {
    let sandbox = Sandbox::new("tex_bc7_skin");
    let source = texture_fixture("bc7.dds");
    write_skin_player(&sandbox, "co Midcup Skin", "05", "skin.dds", &source);
    let skin = format!("{PLAYER_TEXTURES}/skin.ftex");

    let emitted = compile_entry(&sandbox, "co Midcup Skin", 21, &skin);
    let round_trip = ftex::ftex_to_dds(&emitted).unwrap();
    let source_blocks = decode(&source, SourceFormat::Dds).unwrap().blocks.unwrap();
    let emitted_blocks = decode(&round_trip, SourceFormat::Dds)
        .unwrap()
        .blocks
        .unwrap();
    assert_eq!(source_blocks.mips.len(), 6, "the fixture's full chain");
    assert_eq!(
        emitted_blocks, source_blocks,
        "the same BC7 blocks, mip by mip"
    );

    let info = ftex::info(&compile_entry(&sandbox, "co Midcup Skin", 18, &skin)).unwrap();
    assert_eq!(info.format, PixelFormat::Bc3);
    assert_eq!(info.mipmaps, 6);
}

// TC-TEX-10
#[test]
fn a_raster_nrm_texture_is_a_bc3_normal_map_on_pes_21() {
    let sandbox = Sandbox::new("tex_png_nrm");
    // A color raster of the same size would be BC7 on PES 21; the `_nrm` stem makes it a
    // normal map, which is BC3 (DXT5nm) on every version.
    write_skin_player(
        &sandbox,
        "co Midcup Nrm",
        "05",
        "skin_nrm.png",
        &texture_fixture("kit.png"),
    );

    let emitted = compile_entry(
        &sandbox,
        "co Midcup Nrm",
        21,
        &format!("{PLAYER_TEXTURES}/skin_nrm.ftex"),
    );

    let info = ftex::info(&emitted).unwrap();
    assert_eq!(info.format, PixelFormat::Bc3);
    assert_eq!((info.width, info.height, info.mipmaps), (256, 128, 9));
}

// TC-TEX-10
#[test]
fn a_per_kit_raster_nrm_texture_is_a_bc3_normal_map_on_pes_21() {
    let sandbox = Sandbox::new("tex_png_nrm_kit");
    // The kit token sits after the role suffix: `pants_nrm_kit1` is a normal map.
    write_skin_player(
        &sandbox,
        "co Midcup Nrm",
        "05",
        "pants_nrm_kit1.png",
        &texture_fixture("kit.png"),
    );

    let emitted = compile_entry(
        &sandbox,
        "co Midcup Nrm",
        21,
        &format!("{PLAYER_TEXTURES}/pants_nrm_kit1.ftex"),
    );

    let info = ftex::info(&emitted).unwrap();
    assert_eq!(info.format, PixelFormat::Bc3);
    assert_eq!((info.width, info.height, info.mipmaps), (256, 128, 9));
}

/// The tracer's hair model naming `skin.dds` and `skin_nrm.dds`: its `shirt.dds` and
/// `dummy_nrm.dds` renamed.
fn hair_model_naming_skin_and_nrm() -> Vec<u8> {
    let file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    let mut model = Model::from_file(&file).unwrap();
    for material in &mut model.materials {
        for (_, texture) in &mut material.textures {
            if texture.file_name == "shirt.dds" {
                texture.file_name = "skin.dds".to_owned();
            } else if texture.file_name == "dummy_nrm.dds" {
                texture.file_name = "skin_nrm.dds".to_owned();
            }
        }
    }
    model.to_file().unwrap().write()
}

// TC-TEX-10
#[test]
fn an_opaque_skin_is_bc1_and_its_normal_map_bc3_with_x_in_alpha_y_in_green_on_pes_18() {
    let sandbox = Sandbox::new("tex_pes18_codecs");
    let export = "exports/co Midcup Codecs";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/fcl_hair.fmdl"),
        &hair_model_naming_skin_and_nrm(),
    );
    // 1024x1024, fully opaque: a color texture PES 18 encodes as BC1.
    let skin_png = encode_png(&[40, 100, 160, 255].repeat(1024 * 1024), 1024, 1024).unwrap();
    // A flat normal map, every source pixel (X, Y) = (10, 200).
    let nrm_png = encode_png(&[10, 200, 90, 255].repeat(16 * 16), 16, 16).unwrap();
    sandbox.write(&format!("{player}/skin.png"), &skin_png);
    sandbox.write(&format!("{player}/skin_nrm.png"), &nrm_png);

    let run = sandbox.run(&pes_settings(&sandbox, 18), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Codecs"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));

    let skin = &entries[&format!("{PLAYER_TEXTURES}/skin.ftex")];
    let info = ftex::info(skin).unwrap();
    assert_eq!(info.format, PixelFormat::Bc1);
    assert_eq!((info.width, info.height, info.mipmaps), (1024, 1024, 11));
    assert_decodes_as(skin, BlockCodec::Bc1, "skin.ftex");

    let nrm = &entries[&format!("{PLAYER_TEXTURES}/skin_nrm.ftex")];
    assert_eq!(ftex::info(nrm).unwrap().format, PixelFormat::Bc3);
    // The bytes the normal-map conversion of the source gives.
    let decoded_source = decode(&nrm_png, SourceFormat::Png).unwrap();
    let expected = dds_convert::convert(
        &decoded_source,
        Target {
            version: PesVersion::Pes18,
            role: TextureRole::Normal,
            keep_uncompressed: true,
        },
    )
    .unwrap();
    assert_eq!(*nrm, expected);
    // The Fox DXT5nm layout: X (the source's red) in alpha, Y (its green) in green, with
    // red 255 and blue 0; the flat block decodes each channel as written.
    let decoded = decode(&ftex::ftex_to_dds(nrm).unwrap(), SourceFormat::Dds).unwrap();
    assert_eq!((decoded.width, decoded.height), (16, 16));
    for pixel in decoded.mips[0].as_chunks::<4>().0 {
        assert_eq!((pixel[0], pixel[2], pixel[3]), (255, 0, 10));
        assert!(pixel[1].abs_diff(200) <= 2, "green: {}", pixel[1]);
    }
}

// TC-TEX-06
#[test]
fn png_and_tga_kit_textures_are_ftex_under_the_kit_s_names_and_a_webp_portrait_is_a_dds() {
    let sandbox = Sandbox::new("tex_kit_formats");
    sandbox.write(
        "exports/co Midcup Kit/Kits/p1/kit.png",
        &texture_fixture("kit.png"),
    );
    sandbox.write(
        "exports/co Midcup Kit/Kits/p1/kit_back.tga",
        &texture_fixture("kit_back.tga"),
    );
    sandbox.write(
        "exports/co Midcup Kit/Portraits/05.webp",
        &texture_fixture("portrait.webp"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Kit"),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for (name, width, height, mipmaps) in [("u0714p1", 256, 128, 9), ("u0714p1_back", 128, 64, 8)] {
        let info = ftex::info(&entries[&kit_texture(name)]).unwrap();
        assert_eq!(info.format, PixelFormat::Bc1, "{name}");
        assert_eq!(
            (info.width, info.height, info.mipmaps),
            (width, height, mipmaps),
            "{name}"
        );
    }
    let portrait = &entries["common/render/symbol/player/71405.dds"];
    assert!(portrait.starts_with(b"DDS "));
    let layout = ftex::dds::read_layout(portrait).unwrap();
    assert_eq!(layout.pixel, ftex::dds::DdsPixel::Format(PixelFormat::Bc3));
    assert_eq!((layout.width, layout.height, layout.mipmaps), (128, 128, 8));
}

/// The paths of the CPK the sandbox's last `compile` wrote.
fn compiled_paths(sandbox: &Sandbox) -> Vec<String> {
    cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))
        .into_keys()
        .collect()
}

/// The CPK path of the face package of `/co/`'s slot `slot`.
fn co_face(slot: u32) -> String {
    format!("Asset/model/character/face/real/714{slot:02}/#Win/face.fpk")
}

/// A single-level BC1 DDS of `width`x`height`, its blocks zero: built from the blocks, so
/// nothing is encoded.
pub(crate) fn bc1_dds(width: u32, height: u32) -> Vec<u8> {
    bc1_dds_levels(width, height, 1)
}

/// `bc1_dds` with `levels` mip levels, each `max(1, side >> level)`, its block count the
/// level's size rounded up to whole 4x4 blocks.
fn bc1_dds_levels(width: u32, height: u32, levels: u32) -> Vec<u8> {
    let mut mips = Vec::new();
    let mut blocks = Vec::new();
    for level in 0..levels {
        let w = width.checked_shr(level).unwrap_or(0).max(1);
        let h = height.checked_shr(level).unwrap_or(0).max(1);
        mips.push(vec![0; (w * h * 4) as usize]);
        blocks.push(vec![0; (w.div_ceil(4) * h.div_ceil(4) * 8) as usize]);
    }
    let decoded = Decoded {
        width,
        height,
        mips,
        blocks: Some(Blocks {
            codec: BlockCodec::Bc1,
            mips: blocks,
        }),
        authored_mips: true,
        uncompressed: false,
    };
    encode_dds(&decoded, BlockCodec::Bc1).unwrap()
}

/// `dds_convert`'s fixture `rgba8.dds`: 32x16 uncompressed RGBA8 under a DX10 header, six
/// levels.
fn uncompressed_dds() -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../libs/dds_convert/tests/fixtures/rgba8.dds"),
    )
    .unwrap()
}

/// Writes `exports/co Midcup Sizes`: slot 03 a 3x3 skin, slot 04 a 300x300 one, slot 05 PNG bytes
/// under `skin.dds` (renamed, not resaved), slot 07 a 512x461 one with a full mip chain,
/// each beside the hair model naming it, and a kit, so the CPK is written whatever the
/// folders become.
fn write_texture_findings_export(sandbox: &Sandbox) {
    let export = "co Midcup Sizes";
    write_skin_player(
        sandbox,
        export,
        "03",
        "skin.png",
        &texture_fixture("tiny.png"),
    );
    write_skin_player(
        sandbox,
        export,
        "04",
        "skin.png",
        &texture_fixture("odd.png"),
    );
    write_skin_player(
        sandbox,
        export,
        "05",
        "skin.dds",
        &texture_fixture("portrait.png"),
    );
    // Fox draws a mipped texture with one side not a power of two: the full chain, ten
    // levels halving 512x461 down to 1x1.
    write_skin_player(
        sandbox,
        export,
        "07",
        "skin.dds",
        &bc1_dds_levels(512, 461, 10),
    );
    sandbox.write(&format!("exports/{export}/Kits/p1/kit.dds"), &tracer_kit());
}

/// The hair model's line on slot `slot`'s folder, which both commands report.
fn hair_weights(slot: &str) -> String {
    format!(
        "Info fmdl_weights_not_normalized [Keep] at Players/{slot} - A (file=fcl_hair.fmdl, count=1662)"
    )
}

// TC-TEX-03
#[test]
fn texture_findings_are_checked_and_each_drops_its_folder() {
    let sandbox = Sandbox::new("tex_checked");
    write_texture_findings_export(&sandbox);
    let findings = [
        hair_weights("03"),
        "Error texture_too_small [DropFolder] at Players/03 - A (file=skin.png)".to_owned(),
        hair_weights("04"),
        "Error texture_not_pow2 [DropFolder] at Players/04 - A (file=skin.png)".to_owned(),
        hair_weights("05"),
        "Error texture_type_mismatch [DropFolder] at Players/05 - A (file=skin.dds)".to_owned(),
        hair_weights("07"),
        "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
    ];

    let check = sandbox.run(&pes21_settings(&sandbox), &["check"]);
    assert_eq!(findings_of(&check.messages(), "co Midcup Sizes"), findings);
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let mut compiled = findings.to_vec();
    compiled.push("Info team_colors_missing [Keep] ()".to_owned());
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Sizes"),
        compiled
    );
    assert_eq!(compile.exit_code(), 1);
    // The kit is in the CPK; nothing of the three dropped folders is, neither a face nor a
    // texture, while slot 07's face and its skin.ftex are.
    let paths = compiled_paths(&sandbox);
    assert!(paths.contains(&kit_texture("u0714p1")), "{paths:?}");
    for slot in ["03", "04", "05"] {
        assert!(
            paths.iter().all(|path| {
                !path.starts_with(&format!("Asset/model/character/face/real/714{slot}"))
                    && !path.starts_with(&format!("Asset/model/character/common/714/{slot} - A/"))
            }),
            "{paths:?}"
        );
    }
    assert!(paths.contains(&co_face(7)), "{paths:?}");
    assert!(
        paths.contains(
            &"Asset/model/character/common/714/07 - A/sourceimages/#windx11/skin.ftex".to_owned()
        ),
        "{paths:?}"
    );
}

// TC-TEX-03
#[test]
fn pass_through_keeps_an_odd_sized_texture_s_folder_but_not_a_renamed_one() {
    let sandbox = Sandbox::new("tex_pass_through");
    write_texture_findings_export(&sandbox);

    let run = sandbox.run(
        &pass_through_settings(&sandbox),
        &["compile", "--no-deploy"],
    );

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Sizes"),
        [
            hair_weights("03"),
            "Error texture_too_small [Keep] at Players/03 - A (file=skin.png)".to_owned(),
            hair_weights("04"),
            "Error texture_not_pow2 [Keep] at Players/04 - A (file=skin.png)".to_owned(),
            hair_weights("05"),
            "Error texture_type_mismatch [DropFolder] at Players/05 - A (file=skin.dds)".to_owned(),
            hair_weights("07"),
            "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
            "Info team_colors_missing [Keep] ()".to_owned(),
        ]
    );
    assert_eq!(run.exit_code(), 1);
    // Both kept folders are packed, the 3x3 texture and the 300x300 one converted as they are.
    let paths = compiled_paths(&sandbox);
    assert!(paths.contains(&co_face(3)), "{paths:?}");
    assert!(
        paths.contains(
            &"Asset/model/character/common/714/03 - A/sourceimages/#windx11/skin.ftex".to_owned()
        ),
        "{paths:?}"
    );
    assert!(paths.contains(&co_face(4)), "{paths:?}");
    assert!(
        paths.contains(
            &"Asset/model/character/common/714/04 - A/sourceimages/#windx11/skin.ftex".to_owned()
        ),
        "{paths:?}"
    );
    assert!(!paths.contains(&co_face(5)), "{paths:?}");
    assert!(paths.contains(&co_face(7)), "{paths:?}");
}

// TC-TEX-04
#[test]
fn a_texture_finding_drops_the_player_folder_naming_the_file() {
    let sandbox = Sandbox::new("tex_drop_texture_codec_unsupported");
    write_skin_player(
        &sandbox,
        "co Midcup Drop",
        "05",
        "skin.dds",
        &texture_fixture("bc6h.dds"),
    );
    // A kit beside the folder, so the CPK is written and the folder's absence observable.
    sandbox.write("exports/co Midcup Drop/Kits/p1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Drop"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Error texture_codec_unsupported [DropFolder] at Players/05 - A (file=skin.dds)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    // The kit is in the CPK; nothing of the folder is, neither its face nor its texture.
    let paths = compiled_paths(&sandbox);
    assert!(paths.contains(&kit_texture("u0714p1")), "{paths:?}");
    assert!(
        paths
            .iter()
            .all(|path| !path.starts_with("Asset/model/character/face/")
                && !path.starts_with(PLAYER_TEXTURES)),
        "{paths:?}"
    );
}

// TC-TEX-07
#[test]
fn a_pre_fox_texture_whose_side_is_no_multiple_of_4_drops_the_player_folder() {
    let sandbox = Sandbox::new("tex_not_div4");
    let export = "exports/co Midcup Skin";
    // 1002x1002 opaque grey, encoded as a real PNG.
    let skin = encode_png(&vec![128; 1002 * 1002 * 4], 1002, 1002).unwrap();
    for (slot, texture, bytes) in [
        ("05", "skin.png", skin),
        // A clean player beside it, so the CPK is written and slot 05's absence observable.
        ("07", "skin.dds", small_dds()),
    ] {
        let player = format!("{export}/Players/{slot} - A");
        sandbox.write(&format!("{player}/face_high.model"), &card_model());
        sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
        sandbox.write(&format!("{player}/{texture}"), &bytes);
    }

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Skin"),
        [
            "Error texture_not_div4 [DropFolder] at Players/05 - A (file=skin.png)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(entries.contains_key(&face_cpk(7)), "{:?}", entries.keys());
    assert!(!entries.contains_key(&face_cpk(5)), "{:?}", entries.keys());
}

// TC-TEX-03
#[test]
fn a_kit_texture_finding_drops_the_kit() {
    let sandbox = Sandbox::new("tex_kit_drop");
    sandbox.write("exports/co Midcup Kit/Kits/p1/kit.dds", &tracer_kit());
    sandbox.write(
        "exports/co Midcup Kit/Kits/p1/kit_back.png",
        &texture_fixture("tiny.png"),
    );
    sandbox.write("exports/co Midcup Kit/Kits/g1/kit.dds", &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Kit"),
        [
            "Error texture_too_small [DropFolder] at Kits/p1 (file=kit_back.png)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_kits(&sandbox), ["u0714g1"]);
}

// TC-CHK-04
#[test]
fn a_4096_kit_drops_on_pes_17_and_an_uncompressed_main_texture_is_argb8_on_pes_21() {
    let sandbox = Sandbox::new("tex_kit_too_big");
    sandbox.write(
        "exports/co Midcup Kit/Kits/p1/kit.dds",
        &bc1_dds(4096, 4096),
    );
    sandbox.write("exports/co Midcup Kit/Kits/p2/kit.dds", &uncompressed_dds());
    let too_big = "Error kit_texture_too_big [DropFolder] at Kits/p1 (file=kit.dds)";
    let identified = "Info export_identified [Keep] (team=/co/, id=714)";

    // PES 15-17 hold a kit's main texture to 2048 pixels: p1 is refused, p2 is not.
    let check = sandbox.run(&pes_settings(&sandbox, 17), &["check"]);
    assert_eq!(
        findings_of(&check.messages(), "co Midcup Kit"),
        [too_big, identified]
    );
    assert_eq!(check.exit_code(), 1);

    let compile = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Kit"),
        [
            too_big,
            identified,
            "Info team_colors_missing [Keep] ()",
            "Info kit_config_generated [Keep] at Kits/p2 ()"
        ]
    );
    assert_eq!(compile.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let pre_fox_kit =
        |name: &str| format!("common/character0/model/character/uniform/texture/{name}.dds");
    assert!(!entries.contains_key(&pre_fox_kit("u0714p1")));
    // p2's uncompressed main texture is encoded on PES 15-17, which soft-crash on one.
    let emitted = wezlib::decompress_if_wrapped(&entries[&pre_fox_kit("u0714p2")]).unwrap();
    let layout = ftex::dds::read_layout(&emitted).unwrap();
    assert_eq!(layout.pixel, ftex::dds::DdsPixel::Format(PixelFormat::Bc3));

    // On Fox a kit's main texture has no size limit of its own: p1 compiles too, and p2's
    // uncompressed one keeps its layout.
    let compile = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    assert_eq!(
        findings_of(&compile.messages(), "co Midcup Kit"),
        [identified, "Info team_colors_missing [Keep] ()",]
    );
    assert_eq!(compile.exit_code(), 0);
    assert_eq!(compiled_kits(&sandbox), ["u0714p1", "u0714p2"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let info = ftex::info(&entries[&kit_texture("u0714p2")]).unwrap();
    assert_eq!(info.format, PixelFormat::Argb8);
}

// TC-TEX-01
#[test]
fn an_uncompressed_player_texture_stays_uncompressed_on_pes_17() {
    let sandbox = Sandbox::new("tex_uncompressed_skin");
    let export = "co Midcup Uncompressed";
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.model"), &card_model());
    sandbox.write(&format!("{player}/face_high.mtl"), &card_materials());
    sandbox.write(&format!("{player}/skin.dds"), &uncompressed_dds());

    let entries = compile_pes17(&sandbox, export, &CLEAN);

    let skin = &entries["common/character1/model/character/uniform/common/714/05 - A/skin.dds"];
    let plain = wezlib::decompress_if_wrapped(skin).unwrap();
    let layout = ftex::dds::read_layout(&plain).unwrap();
    assert_eq!(
        layout.pixel,
        ftex::dds::DdsPixel::Format(PixelFormat::Argb8)
    );
    assert_eq!(
        decode(&plain, SourceFormat::Dds).unwrap().mips,
        decode(&uncompressed_dds(), SourceFormat::Dds).unwrap().mips,
        "the source's own levels, alpha and all"
    );
}

// TC-TEX-12
#[test]
fn a_common_texture_the_deep_pass_drops_takes_the_player_linking_it() {
    let sandbox = Sandbox::new("tex_common_link_dropped");
    let export = "exports/co Midcup Link";
    sandbox.write(
        &format!("{export}/Players/05 - A/fcl_hair.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(&format!("{export}/Players/05 - A/skin.png.common"), b"");
    sandbox.write(
        &format!("{export}/Common/skin.png"),
        &texture_fixture("tiny.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["check"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Link"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Error texture_too_small [DropFile] at Common/skin.png (file=skin.png)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=skin.png.common, target=Common/skin.png, finding=texture_too_small)",
            "Info export_identified [Keep] (team=/co/, id=714)",
        ]
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-CMN-31
#[test]
fn a_dropped_member_of_a_common_kit_set_leaves_the_player_linking_it() {
    let sandbox = Sandbox::new("tex_common_kit_drop");
    let export = "exports/co Midcup Kits";
    for slot in ["p1", "p2"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("armor_bsm.dds", "./kitN/"),
    );
    sandbox.write(&format!("{player}/kitN/armor_bsm.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/kit1/armor_bsm.png"),
        &texture_fixture("tiny.png"),
    );
    sandbox.write(
        &format!("{export}/Common/kit2/armor_bsm.dds"),
        &tracer_kit(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Kits"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Error texture_too_small [DropFile] at Common/kit1/armor_bsm.png (file=armor_bsm.png)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_missing [Keep] at Common (texture=kitN/armor_bsm, kit=1, copied=kit2/armor_bsm)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let face = "Asset/model/character/face/real/71405/#Win/face.fpk";
    assert!(entries.contains_key(face), "{:?}", entries.keys());
}

// TC-TEX-12
#[test]
fn a_common_texture_whose_task_fails_lets_the_players_linking_it_commit() {
    let sandbox = Sandbox::new("tex_common_task_failed");
    // The deep pass drops the 3x3 hair.png: the player linking it follows.
    let dropped = "exports/co Midcup Dropped";
    sandbox.write(
        &format!("{dropped}/Players/05 - A/fcl_hair.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{dropped}/Players/05 - A/hair.png.common"), b"");
    sandbox.write(
        &format!("{dropped}/Common/hair.png"),
        &texture_fixture("tiny.png"),
    );
    // A header the deep pass reads, over pixel data cut short: no deep-pass finding, the
    // conversion fails the export's Common task instead, which commits on its own.
    let failed = "exports/dbg Midcup Failed";
    sandbox.write(
        &format!("{failed}/Players/05 - A/fcl_hair.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{failed}/Players/05 - A/hair.dds.common"), b"");
    sandbox.write(&format!("{failed}/Common/hair.dds"), &pixels_cut_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Dropped"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Error texture_too_small [DropFile] at Common/hair.png (file=hair.png)",
            "Error link_target_dropped [DropFolder] at Players/05 - A (link=hair.png.common, target=Common/hair.png, finding=texture_too_small)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    let failed_findings = findings_of(&messages, "dbg Midcup Failed");
    assert_eq!(
        failed_findings[..3],
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/dbg/, id=790)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    let [failure] = &failed_findings[3..] else {
        panic!("{failed_findings:#?}");
    };
    assert!(
        failure.starts_with(
            "Error folder_pack_failed [DropFolder] at Common (error=cannot convert hair.dds"
        ),
        "{failure}"
    );
    assert_eq!(run.exit_code(), 1);
    // Slot 05 of /co/ is left out; slot 05 of /dbg/ commits anyway.
    assert_eq!(compiled_players(&sandbox), [79005]);
    let paths = compiled_paths(&sandbox);
    assert!(
        paths.iter().all(|path| !path.contains("face/real/71405")),
        "{paths:?}"
    );
}

// TC-TEX-12
#[test]
fn a_common_texture_only_conversion_refuses_fails_the_task_and_the_linking_player_commits() {
    // `Common/hair.dds` is BC6H, which the deep pass's header probe passes and only
    // conversion refuses: the Common task fails with `texture_codec_unsupported`, so no
    // Common texture is emitted — the sound `Common/skin.dds` included — while the player
    // linking the texture still commits.
    let sandbox = Sandbox::new("tex_common_codec");
    let export = "exports/co Midcup Codec";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/fcl_hair.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &texture_fixture("bc6h.dds"),
    );
    sandbox.write(&format!("{export}/Common/skin.dds"), &small_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Codec"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Error texture_codec_unsupported [DropFolder] at Common (file=hair.dds)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries
            .keys()
            .all(|path| !path.starts_with("Asset/model/character/common/714/sourceimages/")),
        "{:#?}",
        entries.keys()
    );
    assert_eq!(compiled_players(&sandbox), [71405]);
}

// TC-TEX-14
#[test]
fn a_common_texture_finding_leaves_that_file_out_and_the_rest_is_emitted() {
    let sandbox = Sandbox::new("tex_common_drop");
    sandbox.copy_tracer_face("exports/co Midcup Common/Players/03 - A");
    sandbox.write(
        "exports/co Midcup Common/Common/tiny.png",
        &texture_fixture("tiny.png"),
    );
    sandbox.write(
        "exports/co Midcup Common/Common/hair.png",
        &texture_fixture("kit.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Common"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/03 - A (file=glove_l.fmdl, count=2)",
            "Error texture_too_small [DropFile] at Common/tiny.png (file=tiny.png)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let common: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("Asset/model/character/common/714/sourceimages/"))
        .collect();
    assert_eq!(common, ["#windx11/hair.ftex"]);
}

// TC-TEX-01
#[test]
fn a_common_texture_and_a_shared_folder_s_texture_in_png_are_emitted_as_ftex() {
    let sandbox = Sandbox::new("tex_common_shared_png");
    let export = "exports/co Midcup Png";
    sandbox.write(&format!("{export}/Players/05 - A/legs.fmdl.common"), b"");
    sandbox.write(
        &format!("{export}/Common/legs.fmdl"),
        &tracer_player_file("fcl_hair.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Common/hair.png"),
        &texture_fixture("kit.png"),
    );
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_player_file("boots.fmdl"),
    );
    sandbox.write(
        &format!("{export}/Boots/Crocs/sole.png"),
        &texture_fixture("kit.png"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Png"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Common/legs.fmdl (file=legs.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info fmdl_fcl_hair_fallback [Keep] at Players/05 - A (file=legs.fmdl.common)",
            "Info team_colors_missing [Keep] ()"
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    for path in [
        "Asset/model/character/common/714/sourceimages/#windx11/hair.ftex",
        "Asset/model/character/boots/k0625/#windx11/sole.ftex",
    ] {
        let emitted = entries
            .get(path)
            .unwrap_or_else(|| panic!("no {path} among {:?}", entries.keys()));
        let info = ftex::info(emitted).unwrap();
        assert_eq!(info.format, PixelFormat::Bc1, "{path}");
        assert_eq!(
            (info.width, info.height, info.mipmaps),
            (256, 128, 9),
            "{path}"
        );
    }
}

// TC-CMN-04
#[test]
fn a_kit_number_without_its_texture_variant_gets_the_fallback_written_as_kit_n() {
    let sandbox = Sandbox::new("tex_kit_variants");
    let export = "exports/co Midcup Variants";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/fcl_hair.fmdl"),
        &hair_model_naming("pants_kitN.dds"),
    );
    // Two different images: the tracer's shirt and its kit.
    sandbox.write(
        &format!("{player}/pants_kit1.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/pants_kit3.dds"), &tracer_kit());
    for slot in ["p1", "p2", "p3"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Variants"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning kit_variant_missing [Keep] at Players/05 - A (texture=pants_kitN, kit=2, copied=pants_kit1)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let texture = |name: &str| {
        let path = format!("{PLAYER_TEXTURES}/{name}.ftex");
        entries.get(&path)
    };
    // The lowest variant is written once under the reference's spelling: no per-kit copy.
    assert!(texture("pants_kit2").is_none(), "{:?}", entries.keys());
    assert_eq!(
        texture("pants_kitN").expect("pants_kitN.ftex"),
        texture("pants_kit1").expect("pants_kit1.ftex"),
        "kit 2's answer is the fallback under `kitN`"
    );
    assert_ne!(texture("pants_kit3"), texture("pants_kit1"));
    let model = FmdlFile::read(face_package(&entries).get("fcl_hair.fmdl").unwrap()).unwrap();
    let paths: Vec<(String, String)> = texture_paths(&model)
        .unwrap()
        .into_iter()
        .map(|path| (path.file_name, path.directory))
        .collect();
    assert!(
        paths.contains(&(
            "pants_kitN.dds".to_owned(),
            "/Assets/pes16/model/character/common/714/05 - A/sourceimages/".to_owned()
        )),
        "{paths:?}"
    );
}

// TC-CMN-26
#[test]
fn a_kits_fallback_texture_is_written_as_kit_n_and_never_its_own_name() {
    let sandbox = Sandbox::new("tex_kit_fallback");
    let export = "exports/co Midcup Fallback";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/fcl_hair.fmdl"),
        &hair_model_naming("pants_kitN.dds"),
    );
    // Two different images: the variant the tracer's shirt, the fallback its kit.
    sandbox.write(
        &format!("{player}/pants_kit1.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/pants_kitF.dds"), &tracer_kit());
    for slot in ["p1", "p2", "p3"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Fallback"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let texture = |name: &str| {
        let path = format!("{PLAYER_TEXTURES}/{name}.ftex");
        entries.get(&path)
    };
    // The fallback is written once under the reference's spelling: `pants_kit1`'s
    // conversion beside it, no per-kit copies and no `pants_kitF` file.
    for name in ["pants_kit2", "pants_kit3", "pants_kitF", "pants_kitf"] {
        assert!(texture(name).is_none(), "{name} among {:?}", entries.keys());
    }
    let kit1 = texture("pants_kit1").expect("pants_kit1.ftex");
    let kitn = texture("pants_kitN").expect("pants_kitN.ftex");
    assert_ne!(kit1, kitn, "kit 1 keeps its own image");
    assert_eq!(
        kitn,
        &ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap(),
        "the `kitF` file's conversion under `kitN`"
    );
}

// TC-CMN-29
#[test]
fn a_kit_folder_s_set_is_found_by_the_model_naming_its_reference_and_its_fallback_written_as_kit_n()
{
    let sandbox = Sandbox::new("tex_kit_folder_set");
    let export = "exports/co Midcup Kit Folder";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("armor_bsm.dds", "./kitN/"),
    );
    // Different images: the variant the tracer's shirt, the fallback its kit.
    sandbox.write(
        &format!("{player}/kit1/armor_bsm.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/kitF/armor_bsm.dds"), &tracer_kit());
    for slot in ["p1", "p2"] {
        sandbox.write(&format!("{export}/Kits/{slot}/kit.dds"), &tracer_kit());
    }

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Kit Folder"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let armor = |folder: &str| {
        format!(
            "Asset/model/character/common/714/05 - A/{folder}/sourceimages/#windx11/armor_bsm.ftex"
        )
    };
    let kit1 = entries.get(&armor("kit1")).expect("kit1/ armor_bsm.ftex");
    let kitn = entries.get(&armor("kitN")).expect("kitN/ armor_bsm.ftex");
    assert_eq!(
        kitn,
        &ftex::dds_to_ftex(&tracer_kit(), ftex::ColorSpace::Normal).unwrap(),
        "the `kitF` file's conversion under `kitN`"
    );
    assert_ne!(kitn, kit1, "kit 1 keeps its own image");
    for path in [&armor("kit2"), &armor("kitF")] {
        assert!(
            !entries.contains_key(path),
            "{path} among {:?}",
            entries.keys()
        );
    }
    // The face's model names `armor_bsm` in the set's `kitN/` directory of the folder's
    // texture home.
    let package = face_package(&entries);
    let directories = texture_directories(package.get("face_high.fmdl").unwrap(), "armor_bsm.dds");
    assert!(!directories.is_empty());
    assert!(
        directories.iter().all(|directory| directory
            == "/Assets/pes16/model/character/common/714/05 - A/kitN/sourceimages/"),
        "{directories:?}"
    );
}

// TC-CMN-30
#[test]
fn a_model_naming_a_kit_f_folder_finds_nothing() {
    let sandbox = Sandbox::new("tex_kit_fallback_missing");
    let export = "exports/co Midcup Kit Fallback";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &model_naming_in("armor_bsm.dds", "./kitF/"),
    );
    sandbox.write(&format!("{player}/kitF/armor_bsm.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Kit Fallback"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning fmdl_texture_not_found [Keep] at Players/05 - A (model=face_high.fmdl, texture=./kitF/armor_bsm.dds)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
}

/// The game's kit texture folder, whose fixed files the modded exe swaps per match.
const KIT_TEXTURE_DIRECTORY: &str = "/Assets/pes16/model/character/uniform/texture/";

// TC-MOD-91
#[test]
fn a_texture_path_into_the_game_s_kit_folder_is_kept_with_a_warning() {
    let sandbox = Sandbox::new("tex_kit_path");
    let export = "exports/co Midcup Kit Path";
    let player = format!("{export}/Players/05 - A");
    // The tracer's hair naming `u0714p1.dds` in the game's kit texture folder.
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &tracer_model_pointing(
            "fcl_hair.fmdl",
            &[("shirt.dds", "u0714p1.dds")],
            KIT_TEXTURE_DIRECTORY,
        ),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Kit Path"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Warning fmdl_texture_kit_path [Keep] at Players/05 - A (model=face_high.fmdl, texture=/Assets/pes16/model/character/uniform/texture/u0714p1.dds)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let model = face_package(&entries);
    // The tracer's hair names the texture in two entries; neither is repointed.
    assert_eq!(
        texture_directories(model.get("face_high.fmdl").unwrap(), "u0714p1.dds"),
        vec![KIT_TEXTURE_DIRECTORY.to_owned(); 2]
    );
}

/// The `team_folder` Common texture directory, which a compile makes the team's.
const COMMON_TEAM_FOLDER: &str = "/Assets/pes16/model/character/common/team_folder/sourceimages/";

/// The tracer's model `model` (`fcl_hair.fmdl`) with each texture `(file name, new file name)`
/// of `renames` renamed and pointed at `COMMON_TEAM_FOLDER`, through `fmdl`'s texture-path
/// rewriting;
/// asserts a mesh uses every renamed texture, so the compiler looks for it.
pub(crate) fn tracer_model_renaming(model: &str, renames: &[(&str, &str)]) -> Vec<u8> {
    tracer_model_pointing(model, renames, COMMON_TEAM_FOLDER)
}

/// `tracer_model_renaming` with the renamed textures pointed at `directory` (`./textures/`).
pub(crate) fn tracer_model_pointing(
    model: &str,
    renames: &[(&str, &str)],
    directory: &str,
) -> Vec<u8> {
    let mut file = FmdlFile::read(&tracer_player_file(model)).unwrap();
    rewrite_texture_paths(&mut file, |path| {
        if let Some((_, renamed)) = renames.iter().find(|(old, _)| path.file_name == *old) {
            path.file_name = (*renamed).to_owned();
            path.directory = directory.to_owned();
        }
    })
    .unwrap();
    let used = used_texture_paths(&file).unwrap();
    for (_, renamed) in renames {
        assert!(
            used.iter()
                .any(|path| path.file_name == *renamed && path.directory == directory),
            "{renamed} unused: {used:?}"
        );
    }
    file.write()
}

/// Writes TC-TEX-05's export: slot 05's `face_high.fmdl` naming `hair.dds` in the team's Common
/// output, and no `Common/`; and slot 03's clean face, so a CPK is written whatever becomes of
/// slot 05's face.
fn write_common_hair_player(sandbox: &Sandbox) {
    sandbox.write(
        "exports/co Midcup Hair/Players/05 - A/face_high.fmdl",
        &tracer_model_renaming("fcl_hair.fmdl", &[("shirt.dds", "hair.dds")]),
    );
    sandbox.write(
        &format!("exports/co Midcup Hair/{CLEAN_PLAYER}"),
        &clean_model(),
    );
}

/// Settings compiling `4cc_62_midcup` for PES 21 with the sandbox's PES folder.
fn midcup_62_settings(sandbox: &Sandbox) -> String {
    format!(
        "{}[team-compiler]\ncpk_name = \"4cc_62_midcup\"\n",
        pes21_settings(sandbox)
    )
}

/// Installs TC-TEX-05's list, `4cc_61_midcup.cpk`, `4cc_62_midcup.cpk` and
/// `4cc_63_midcup.cpk` in that order, and the CPK `holder` holding the team's Common `hair`.
fn install_hair_in(sandbox: &Sandbox, holder: &str) {
    install_names(
        sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        sandbox,
        holder,
        &[(
            "Asset/model/character/common/714/sourceimages/#windx11/hair.ftex",
            b"compiled on an earlier day",
        )],
    );
}

/// The slot-05 finding of TC-TEX-05's missing hair.
fn hair_not_found() -> String {
    "Warning fmdl_texture_not_found [Keep] at Players/05 - A (model=face_high.fmdl, texture=/Assets/pes16/model/character/common/714/sourceimages/hair.dds)".to_owned()
}

/// The findings of TC-TEX-05's export, ending with `task`, its face task's.
fn hair_export_findings(task: &[String]) -> Vec<String> {
    let mut findings = vec![
        "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)".to_owned(),
        "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
        "Info team_colors_missing [Keep] ()".to_owned(),
    ];
    findings.extend(task.iter().cloned());
    findings
}

// TC-TEX-05
#[test]
fn a_common_texture_only_an_earlier_installed_cpk_holds_is_found_and_a_later_one_s_is_not() {
    let sandbox = Sandbox::new("tex_installed_earlier");
    write_common_hair_player(&sandbox);
    install_hair_in(&sandbox, "4cc_61_midcup.cpk");

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Hair"),
        hair_export_findings(&[])
    );
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("fmdl_texture_not_found")),
        "{messages:?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let model = face_package(&entries);
    // The tracer's hair names its `shirt.dds`, renamed `hair.dds`, in two entries.
    let hair = texture_directories(model.get("face_high.fmdl").unwrap(), "hair.dds");
    assert_eq!(
        hair,
        ["/Assets/pes16/model/character/common/714/sourceimages/"; 2]
    );
}

// TC-TEX-21
#[test]
fn a_common_texture_only_a_later_installed_cpk_holds_is_a_warning_and_the_face_is_kept() {
    let sandbox = Sandbox::new("tex_installed_later");
    write_common_hair_player(&sandbox);
    install_hair_in(&sandbox, "4cc_63_midcup.cpk");

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        hair_export_findings(&[hair_not_found()])
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    assert!(face_package(&entries).get("face_high.fmdl").is_some());
}

// TC-TEX-05
#[test]
fn a_common_texture_with_no_pes_folder_to_look_in_is_a_warning_and_the_face_is_kept() {
    let sandbox = Sandbox::new("tex_installed_unknown");
    write_common_hair_player(&sandbox);

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        hair_export_findings(&[hair_not_found()])
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    assert!(face_package(&entries).get("face_high.fmdl").is_some());
}

/// Writes TC-TEX-11's export: slot 06's `face_high.fmdl` naming `hair.dds` where the tracer
/// names its `shirt.dds`, not in the team's Common output (only the link points it there),
/// beside `hair.dds.common`, and no `Common/`; and slot 03's clean face, so a CPK is written
/// when slot 06 is dropped.
fn write_hair_link_player(sandbox: &Sandbox) {
    let player = "exports/co Midcup Hair/Players/06 - A";
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    sandbox.write(
        &format!("exports/co Midcup Hair/{CLEAN_PLAYER}"),
        &clean_model(),
    );
}

/// The findings `check` reports on TC-TEX-11's export ending with `first`, the slot-06
/// finding, then those `compile` reports: the same, then the planning's
/// `team_colors_missing`.
fn hair_link_findings(first: &str) -> [(&'static str, Vec<String>); 2] {
    let check = vec![
        first.to_owned(),
        "Info export_identified [Keep] (team=/co/, id=714)".to_owned(),
    ];
    let mut compile = check.clone();
    compile.push("Info team_colors_missing [Keep] ()".to_owned());
    [("check", check), ("compile", compile)]
}

// TC-TEX-11
#[test]
fn a_texture_link_an_earlier_installed_cpk_satisfies_points_the_model_there_and_packs_nothing() {
    let sandbox = Sandbox::new("tex_link_installed_earlier");
    write_hair_link_player(&sandbox);
    install_hair_in(&sandbox, "4cc_61_midcup.cpk");
    let settings = midcup_62_settings(&sandbox);

    for (command, findings) in hair_link_findings(
        "Info fmdl_weights_not_normalized [Keep] at Players/06 - A (file=face_high.fmdl, count=1662)",
    ) {
        let run = sandbox.run(&settings, &command_args(command));
        let messages = run.messages();
        assert_eq!(
            findings_of(&messages, "co Midcup Hair"),
            findings,
            "{command}"
        );
        assert!(
            messages
                .iter()
                .all(|line| !line.contains("common_link_missing")
                    && !line.contains("fmdl_texture_not_found")),
            "{command}: {messages:?}"
        );
        assert_eq!(run.exit_code(), 0, "{command}");
    }

    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/face/real/71406/#Win/face.fpk"])
            .unwrap();
    // The tracer's hair names its `shirt.dds`, renamed `hair.dds`, in two entries.
    let hair = texture_directories(package.get("face_high.fmdl").unwrap(), "hair.dds");
    assert_eq!(
        hair,
        ["/Assets/pes16/model/character/common/714/sourceimages/"; 2]
    );
    let packed_hair: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("hair.ftex"))
        .collect();
    assert!(packed_hair.is_empty(), "{packed_hair:?}");
}

// TC-TEX-11
#[test]
fn a_texture_link_only_a_later_installed_cpk_satisfies_is_common_link_missing() {
    let sandbox = Sandbox::new("tex_link_installed_later");
    write_hair_link_player(&sandbox);
    install_hair_in(&sandbox, "4cc_63_midcup.cpk");
    let settings = midcup_62_settings(&sandbox);

    for (command, findings) in hair_link_findings(
        "Error common_link_missing [DropFolder] at Players/06 - A (link=hair.dds.common, path=Common/hair.dds)",
    ) {
        let run = sandbox.run(&settings, &command_args(command));
        assert_eq!(
            findings_of(&run.messages(), "co Midcup Hair"),
            findings,
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }

    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let slot_06: Vec<&String> = entries
        .keys()
        .filter(|path| path.contains("71406") || path.contains("06 - A"))
        .collect();
    assert!(slot_06.is_empty(), "{slot_06:?}");
}

/// Writes slot 06 of TC-TEX-11's export: `jessie/hair_high.fmdl` naming `hair.dds` where the
/// tracer names its `shirt.dds`, and `jessie/hair.dds.common` for `Common/jessie/hair.dds`,
/// which is not in the export; and slot 03's clean face, so a CPK is written when slot 06 is
/// dropped.
fn write_nested_hair_link_player(sandbox: &Sandbox) {
    let player = "exports/co Midcup Hair/Players/06 - A";
    sandbox.write(
        &format!("{player}/jessie/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    sandbox.write(
        &format!("exports/co Midcup Hair/{CLEAN_PLAYER}"),
        &clean_model(),
    );
}

// TC-TEX-11
#[test]
fn a_nested_texture_link_an_installed_cpk_satisfies_points_the_model_at_its_own_path() {
    let sandbox = Sandbox::new("tex_nested_link_installed");
    write_nested_hair_link_player(&sandbox);
    let installed = "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex";
    install_names(&sandbox, &["4cc_61_midcup.cpk", "4cc_62_midcup.cpk"]);
    install_cpk(&sandbox, "4cc_61_midcup.cpk", &[(installed, b"earlier")]);
    let settings = midcup_62_settings(&sandbox);

    for (command, findings) in hair_link_findings(
        "Info fmdl_weights_not_normalized [Keep] at Players/06 - A (file=jessie/hair_high.fmdl, count=1662)",
    ) {
        let run = sandbox.run(&settings, &command_args(command));
        let messages = run.messages();
        assert_eq!(
            findings_of(&messages, "co Midcup Hair"),
            findings,
            "{command}"
        );
        assert!(
            messages
                .iter()
                .all(|line| !line.contains("common_link_missing")
                    && !line.contains("fmdl_texture_not_found")),
            "{command}: {messages:?}"
        );
        assert_eq!(run.exit_code(), 0, "{command}");
    }

    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/face/real/71406/#Win/face.fpk"])
            .unwrap();
    // The tracer's hair names its `shirt.dds`, renamed `hair.dds`, in two entries.
    let hair = texture_directories(package.get("hair_high.fmdl").unwrap(), "hair.dds");
    assert_eq!(
        hair,
        ["/Assets/pes16/model/character/common/714/jessie/sourceimages/"; 2]
    );
    let packed_hair: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("hair.ftex"))
        .collect();
    assert!(packed_hair.is_empty(), "{packed_hair:?}");
}

// TC-TEX-11
#[test]
fn a_nested_texture_link_only_the_direct_installed_texture_satisfies_is_common_link_missing() {
    let sandbox = Sandbox::new("tex_nested_link_installed_direct");
    write_nested_hair_link_player(&sandbox);
    install_hair_in(&sandbox, "4cc_61_midcup.cpk");
    let settings = midcup_62_settings(&sandbox);

    for (command, findings) in hair_link_findings(
        "Error common_link_missing [DropFolder] at Players/06 - A (link=hair.dds.common, path=Common/jessie/hair.dds)",
    ) {
        let run = sandbox.run(&settings, &command_args(command));
        assert_eq!(
            findings_of(&run.messages(), "co Midcup Hair"),
            findings,
            "{command}"
        );
        assert_eq!(run.exit_code(), 1, "{command}");
    }
}

/// The `jessie/` subdirectory of the `team_folder` Common output, a `Common/` subfolder's
/// texture home as a model names it.
const JESSIE_TEAM_FOLDER: &str =
    "/Assets/pes16/model/character/common/team_folder/jessie/sourceimages/";

/// Writes slot 05 of `exports/co Midcup Hair`: `face_high.fmdl` naming `hair.dds` in the
/// `jessie/` subdirectory of the team's Common output; and slot 03's clean face, so a CPK is
/// written when slot 05 is dropped.
fn write_subfolder_hair_player(sandbox: &Sandbox) {
    sandbox.write(
        "exports/co Midcup Hair/Players/05 - A/face_high.fmdl",
        &tracer_model_pointing(
            "fcl_hair.fmdl",
            &[("shirt.dds", "hair.dds")],
            JESSIE_TEAM_FOLDER,
        ),
    );
    sandbox.write(
        &format!("exports/co Midcup Hair/{CLEAN_PLAYER}"),
        &clean_model(),
    );
}

/// The slot-05 finding of the `jessie/` Common path nobody supplies.
fn subfolder_hair_not_found() -> String {
    "Warning fmdl_texture_not_found [Keep] at Players/05 - A (model=face_high.fmdl, texture=/Assets/pes16/model/character/common/714/jessie/sourceimages/hair.dds)".to_owned()
}

// TC-TEX-05
#[test]
fn a_common_subfolder_texture_nobody_holds_is_missing() {
    let sandbox = Sandbox::new("tex_common_sub_missing");
    write_subfolder_hair_player(&sandbox);
    // The list names the CPKs but none is written: the lookup is made and finds nothing.
    install_names(
        &sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Hair"),
        hair_export_findings(&[subfolder_hair_not_found()])
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    assert!(face_package(&entries).get("face_high.fmdl").is_some());
}

// TC-TEX-05
#[test]
fn a_common_subfolder_texture_the_export_holds_supplies_the_path() {
    let sandbox = Sandbox::new("tex_common_sub_held");
    write_subfolder_hair_player(&sandbox);
    // The link stands for `Common/jessie/hair.dds` in `jessie/`'s namespace; the file is in
    // the export.
    sandbox.write(
        "exports/co Midcup Hair/Players/05 - A/jessie/hair.dds.common",
        b"",
    );
    sandbox.write(
        "exports/co Midcup Hair/Common/jessie/hair.dds",
        &tracer_kit(),
    );
    install_names(&sandbox, &["4cc_61_midcup.cpk", "4cc_62_midcup.cpk"]);

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Hair"),
        hair_export_findings(&[])
    );
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("fmdl_texture_not_found")),
        "{messages:?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let model = face_package(&entries);
    // The tracer's hair names its `shirt.dds`, renamed `hair.dds`, in two entries.
    let hair = texture_directories(model.get("face_high.fmdl").unwrap(), "hair.dds");
    assert_eq!(
        hair,
        ["/Assets/pes16/model/character/common/714/jessie/sourceimages/"; 2]
    );
    let packed_hair: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("jessie/sourceimages/#windx11/hair.ftex"))
        .collect();
    assert_eq!(packed_hair.len(), 1, "{packed_hair:?}");
}

// TC-TEX-05
#[test]
fn a_common_subfolder_texture_an_earlier_installed_cpk_holds_is_found() {
    let sandbox = Sandbox::new("tex_common_sub_installed");
    write_subfolder_hair_player(&sandbox);
    install_names(
        &sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        &sandbox,
        "4cc_61_midcup.cpk",
        &[(
            "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex",
            b"compiled on an earlier day",
        )],
    );

    let run = sandbox.run(&midcup_62_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Hair"),
        hair_export_findings(&[])
    );
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("fmdl_texture_not_found")),
        "{messages:?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let model = face_package(&entries);
    let hair = texture_directories(model.get("face_high.fmdl").unwrap(), "hair.dds");
    assert_eq!(
        hair,
        ["/Assets/pes16/model/character/common/714/jessie/sourceimages/"; 2]
    );
    let packed_hair: Vec<&String> = entries
        .keys()
        .filter(|path| path.ends_with("hair.ftex"))
        .collect();
    assert!(packed_hair.is_empty(), "{packed_hair:?}");
}

/// The card-head material set with its one texture path, `./texture.dds`, replaced by
/// `path`.
fn mtl_naming(path: &str) -> Vec<u8> {
    let text = String::from_utf8(crate::prefox_faces::pre_fox_fixture(
        "cardhead_materials.mtl",
    ))
    .unwrap();
    assert!(text.contains("./texture.dds"), "{text}");
    text.replace("./texture.dds", path).into_bytes()
}

// TC-TEX-11
#[test]
fn a_common_mtl_s_stem_a_player_s_link_supplies_from_an_installed_cpk_reports_nothing() {
    let sandbox = Sandbox::new("tex_common_mtl_link_installed");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    // The links stand for `Common/jessie/body.model` and `Common/jessie/hair.dds`; the
    // texture is not in the export, an earlier CPK holds it.
    sandbox.write(&format!("{player}/jessie/body.model.common"), b"");
    sandbox.write(&format!("{player}/jessie/hair.dds.common"), b"");
    sandbox.write(&format!("{export}/Common/jessie/body.model"), &card_model());
    sandbox.write(
        &format!("{export}/Common/jessie/body.mtl"),
        &mtl_naming("hair.dds"),
    );
    install_names(
        &sandbox,
        &[
            "4cc_61_midcup.cpk",
            "4cc_62_midcup.cpk",
            "4cc_63_midcup.cpk",
        ],
    );
    install_cpk(
        &sandbox,
        "4cc_61_midcup.cpk",
        &[(
            "Asset/model/character/common/714/jessie/sourceimages/#windx11/hair.ftex",
            b"compiled on an earlier day",
        )],
    );
    let settings = midcup_62_settings(&sandbox);

    let check = sandbox.run(&settings, &["check"]);
    let messages = check.messages();
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("mtl_texture_not_found")),
        "check: {messages:?}"
    );
    assert_eq!(check.exit_code(), 0, "check: {messages:?}");

    let compile = sandbox.run(&settings, &["compile", "--no-deploy"]);
    assert_eq!(compile.exit_code(), 0, "{:?}", compile.messages());
    let entries = cpk_entries(&sandbox.root.join("output/4cc_62_midcup.cpk"));
    let package =
        fpk::FpkFile::read(&entries["Asset/model/character/face/real/71405/#Win/face.fpk"])
            .unwrap();
    assert!(package.get("face_high.fmdl").is_some());
    assert!(package.get("fcl_hair.fmdl").is_some());
}

// TC-TEX-16
#[test]
fn a_texture_link_of_the_player_s_wins_over_a_combined_folder_s_texture_of_the_stem() {
    let sandbox = Sandbox::new("tex_link_overrides_shared");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");
    sandbox.write(
        &format!("{export}/Faces/Round/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{export}/Faces/Round/hair.dds"), &tracer_kit());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info shared_texture_overridden [Keep] at Players/05 - A (texture=hair, folder=Faces/Round)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    // Both models of the face package name `hair` in the team's Common output, where the
    // link points it; the shared folder's copy is left out of slot 05's texture folder.
    let package = face_package(&entries);
    for model in ["face_high.fmdl", "hair_high.fmdl"] {
        assert_eq!(
            texture_directories(package.get(model).unwrap(), "hair.dds"),
            ["/Assets/pes16/model/character/common/714/sourceimages/"; 2],
            "{model}"
        );
    }
    let packed_hair: Vec<&str> = entries
        .keys()
        .filter(|path| path.ends_with("hair.ftex"))
        .map(String::as_str)
        .collect();
    assert_eq!(
        packed_hair,
        ["Asset/model/character/common/714/sourceimages/#windx11/hair.ftex"]
    );
}

// TC-MOD-78
#[test]
fn a_combined_folder_s_texture_link_names_its_common_texture_on_pes_21() {
    // `Faces/Round`'s `hair.dds.common` stands for `Common/hair.dds` at `hair` below the
    // player folder, as a link of his own would: `hair_high` names it there.
    let sandbox = Sandbox::new("tex_combined_link");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/Round.face"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{round}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "hair.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == "/Assets/pes16/model/character/common/714/sourceimages/"),
        "{directories:?}"
    );
    assert!(
        entries.contains_key("Asset/model/character/common/714/sourceimages/#windx11/hair.ftex"),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-17
#[test]
fn a_combined_folder_s_texture_link_loses_to_the_player_s_own() {
    // His own `hair.dds` wins the path the combined folder's `hair.dds.common` stands at,
    // as it would over the folder's own texture: `shared_texture_overridden` names it.
    let sandbox = Sandbox::new("tex_combined_link_overridden");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds"), &small_dds());
    sandbox.write(&format!("{player}/Round.face"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{round}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info shared_texture_overridden [Keep] at Players/05 - A (texture=hair, folder=Faces/Round)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    // Both models of the face package name `hair` in his own texture home, which holds
    // `hair.ftex`.
    for model in ["face_high.fmdl", "hair_high.fmdl"] {
        let directories = texture_directories(package.get(model).unwrap(), "hair.dds");
        assert!(!directories.is_empty());
        assert!(
            directories.iter().all(|directory| directory
                == "/Assets/pes16/model/character/common/714/05 - A/sourceimages/"),
            "{model}: {directories:?}"
        );
    }
    assert!(
        entries.contains_key(
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/hair.ftex"
        ),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-17
#[test]
fn a_combined_link_and_a_link_of_his_report_nothing() {
    // Two links at `hair`, his and the combined folder's, name the one `Common/` file:
    // nothing is reported for the path, and both models name it in the team's Common
    // output.
    let sandbox = Sandbox::new("tex_two_links");
    let export = "exports/co Midcup Hair";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(
        &format!("{player}/face_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{player}/hair.dds.common"), b"");
    // An unrelated texture so the textures batch emits an entry.
    sandbox.write(&format!("{player}/skin.dds"), &small_dds());
    sandbox.write(&format!("{player}/Round.face"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("hair.dds"),
    );
    sandbox.write(&format!("{round}/hair.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Hair"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=face_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    for model in ["face_high.fmdl", "hair_high.fmdl"] {
        let directories = texture_directories(package.get(model).unwrap(), "hair.dds");
        assert!(!directories.is_empty());
        assert!(
            directories
                .iter()
                .all(|directory| directory
                    == "/Assets/pes16/model/character/common/714/sourceimages/"),
            "{model}: {directories:?}"
        );
    }
}

// TC-TEX-18
#[test]
fn a_combined_texture_and_another_package_s_link_at_one_path_conflict() {
    // `Faces/Round`'s `skin.dds` copy and `Boots/Crocs`' `skin.dds.common`: the link
    // counts as differing, and the conflict leaves the whole player folder out.
    let sandbox = Sandbox::new("tex_combined_file_conflict");
    let export = "exports/co Midcup Skin";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    // His own boots model gives his boots package a part, so `Crocs.boots` combines; both
    // `shirt` materials name `skin.dds`, so the merge defines it the same way.
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(
        &format!("{round}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{crocs}/skin.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
            "Error shared_texture_conflict [DropFolder] at Players/05 - A (texture=skin, packages=face,boots)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(compiled_players(&sandbox).is_empty(), "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.contains("05 - A")),
        "{:#?}",
        entries.keys()
    );
    assert!(
        !entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-18
#[test]
fn two_combined_links_at_one_path_deduplicate() {
    // `Faces/Round`'s and `Boots/Crocs`'s `skin.dds.common` name the one `Common/` file:
    // no conflict and no drop, both packages kept, both models name `skin` in the team's
    // Common output.
    let sandbox = Sandbox::new("tex_two_combined_links");
    let export = "exports/co Midcup Skin";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    // His own boots model gives his boots package a part, so `Crocs.boots` combines; both
    // `shirt` materials name `skin.dds`, so the merge defines it the same way.
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(&format!("{round}/skin.dds.common"), b"");
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{crocs}/skin.dds.common"), b"");
    // An unrelated texture, so the textures batch emits an entry.
    sandbox.write(&format!("{crocs}/sole.dds"), &small_dds());
    sandbox.write(
        &format!("{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let package = face_package(&entries);
    let directories = texture_directories(package.get("hair_high.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == "/Assets/pes16/model/character/common/714/sourceimages/"),
        "{directories:?}"
    );
    let boots =
        fpk::FpkFile::read(&entries["Asset/model/character/boots/k0625/#Win/boots.fpk"]).unwrap();
    let directories = texture_directories(boots.get("boots.fmdl").unwrap(), "skin.dds");
    assert!(!directories.is_empty());
    assert!(
        directories
            .iter()
            .all(|directory| directory == "/Assets/pes16/model/character/common/714/sourceimages/"),
        "{directories:?}"
    );
    assert!(
        entries.contains_key("Asset/model/character/common/714/sourceimages/#windx11/skin.ftex"),
        "{:#?}",
        entries.keys()
    );
    // `sole.dds`, the one copy kept, is in his texture home; `skin`'s is not.
    assert!(
        entries.contains_key(
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/sole.ftex"
        ),
        "{:#?}",
        entries.keys()
    );
    assert!(
        !entries.contains_key(
            "Asset/model/character/common/714/05 - A/sourceimages/#windx11/skin.ftex"
        ),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-18
#[test]
fn a_combined_copy_and_a_lower_package_s_link_at_one_path_conflict() {
    // `Boots/Crocs`'s `skin.dds` copy and `Gloves/X`'s `skin.dds.common`, the gloves
    // combined by his own `glove_l.fmdl`: the conflict leaves the player folder out.
    let sandbox = Sandbox::new("tex_copy_vs_glove_link");
    let export = "exports/co Midcup Skin";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    sandbox.write(&format!("{player}/glove_l.fmdl"), &clean_model());
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    sandbox.write(&format!("{player}/X.gloves"), b"");
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(
        &format!("{crocs}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    let gloves = format!("{export}/Gloves/X");
    sandbox.write(&format!("{gloves}/glove_r.fmdl"), &clean_model());
    sandbox.write(&format!("{gloves}/skin.dds.common"), b"");
    sandbox.write(
        &format!("{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    let findings = findings_of(&lines, "co Midcup Skin");
    assert!(
        findings.contains(&"Error shared_texture_conflict [DropFolder] at Players/05 - A (texture=skin, packages=boots,gloves)"),
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(compiled_players(&sandbox).is_empty(), "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.contains("05 - A")),
        "{:#?}",
        entries.keys()
    );
    assert!(
        entries
            .keys()
            .all(|path| !path.contains("/glove/") || !path.ends_with("glove.fpk")),
        "{:#?}",
        entries.keys()
    );
    assert!(
        !entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-18
#[test]
fn a_combined_link_and_another_package_s_texture_at_one_path_conflict() {
    // `Faces/Round`'s `skin.dds.common` and `Boots/Crocs`'s `skin.dds` hold one path below
    // the player folder: a link counts as differing from a file, so `skin` is a conflict
    // leaving the whole player folder out.
    let sandbox = Sandbox::new("tex_combined_link_conflict");
    let export = "exports/co Midcup Skin";
    let player = format!("{export}/Players/05 - A");
    sandbox.write(&format!("{player}/face_high.fmdl"), &clean_model());
    // His own boots model gives his boots package a part, so `Crocs.boots` combines; both
    // `shirt` materials name `skin.dds`, so the merge defines it the same way.
    sandbox.write(
        &format!("{player}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(&format!("{player}/Round.face"), b"");
    sandbox.write(&format!("{player}/Crocs.boots"), b"");
    let round = format!("{export}/Faces/Round");
    sandbox.write(
        &format!("{round}/hair_high.fmdl"),
        &hair_model_naming("skin.dds"),
    );
    sandbox.write(&format!("{round}/skin.dds.common"), b"");
    let crocs = format!("{export}/Boots/Crocs");
    sandbox.write(
        &format!("{crocs}/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "skin.dds")]),
    );
    sandbox.write(
        &format!("{crocs}/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );
    sandbox.write(
        &format!("{export}/Common/skin.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Skin"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Faces/Round (file=hair_high.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Boots/Crocs (file=boots.fmdl, count=1662)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info link_combined [Keep] at Players/05 - A (link=Crocs.boots)",
            "Info link_combined [Keep] at Players/05 - A (link=Round.face)",
            "Info fmdl_merged [Keep] at Players/05 - A (model=boots.fmdl)",
            "Error shared_texture_conflict [DropFolder] at Players/05 - A (texture=skin, packages=face,boots)",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 1, "{lines:#?}");
    assert!(compiled_players(&sandbox).is_empty(), "{lines:#?}");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.contains("05 - A")),
        "{:#?}",
        entries.keys()
    );
    assert!(
        !entries.contains_key("Asset/model/character/boots/k0625/#Win/boots.fpk"),
        "{:#?}",
        entries.keys()
    );
}

// TC-TEX-20
#[test]
fn a_texture_entry_no_mesh_uses_is_not_looked_for() {
    let sandbox = Sandbox::new("tex_unused_entry");
    // The tracer's hair with one more texture entry, `unused.dds` in the team's Common output,
    // which no material instance's texture run names.
    let mut file = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    file.textures.push(file.textures[0].clone());
    let unused = file.textures.len() - 1;
    let mut index = 0;
    rewrite_texture_paths(&mut file, |path| {
        if index == unused {
            path.file_name = "unused.dds".to_owned();
            path.directory = COMMON_TEAM_FOLDER.to_owned();
        }
        index += 1;
    })
    .unwrap();
    let used = used_texture_paths(&file).unwrap();
    assert!(
        used.iter().all(|path| path.file_name != "unused.dds"),
        "{used:?}"
    );
    sandbox.write(
        "exports/co Midcup Unused/Players/05 - A/face_high.fmdl",
        &file.write(),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("fmdl_texture_not_found")),
        "{messages:?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let model = face_package(&entries);
    assert_eq!(
        texture_directories(model.get("face_high.fmdl").unwrap(), "unused.dds"),
        ["/Assets/pes16/model/character/common/714/sourceimages/"],
        "the entry is kept, pointed at the team"
    );
}

// TC-TEX-19
#[test]
fn a_shared_folder_s_model_naming_a_texture_in_common_finds_it_there() {
    let sandbox = Sandbox::new("tex_shared_common");
    let export = "exports/co Midcup Crocs";
    sandbox.write(&format!("{export}/Players/05 - A/Crocs.boots"), b"");
    sandbox.write(
        &format!("{export}/Boots/Crocs/boots.fmdl"),
        &tracer_model_renaming("boots.fmdl", &[("shirt.dds", "hair.dds")]),
    );
    sandbox.write(
        &format!("{export}/Common/hair.dds"),
        &tracer_player_file("shirt.dds"),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert!(
        messages
            .iter()
            .all(|line| !line.contains("fmdl_texture_not_found")),
        "{messages:?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.contains_key("Asset/model/character/common/714/sourceimages/#windx11/hair.ftex"),
        "{:?}",
        entries.keys()
    );
}

// TC-CMN-36
#[test]
fn kit_stand_in_textures_are_never_looked_for_and_written_in_the_game_spelling() {
    let sandbox = Sandbox::new("tex_team_kit");
    let export = "co Midcup Dummy";
    let player = |slot: &str| format!("exports/{export}/Players/{slot}");
    // Slot 05's face names `team_kit` and `Team_Kit_Srm` in `team_common_folder/` and
    // `team_gk_kit` in `./`; slot 06's `.mtl` names `team_common_folder/team_kit.dds`
    // and a game path. None of them is present: the exes substitute the stand-ins.
    let mut model = FmdlFile::read(&tracer_player_file("fcl_hair.fmdl")).unwrap();
    let entries = [
        ("team_kit.dds", "team_kit.dds", "team_common_folder/"),
        ("dummy_srm.dds", "Team_Kit_Srm.dds", "team_common_folder/"),
        ("dummy_nrm.dds", "team_gk_kit.dds", "./"),
    ];
    rewrite_texture_paths(&mut model, |path| {
        if let Some((_, name, directory)) =
            entries.iter().find(|(from, _, _)| path.file_name == *from)
        {
            path.file_name = (*name).to_owned();
            path.directory = (*directory).to_owned();
        }
    })
    .unwrap();
    sandbox.write(
        &format!("{}/face_high.fmdl", player("05 - A")),
        &model.write(),
    );
    sandbox.write(
        &format!("{}/face_high.model", player("06 - B")),
        &card_model(),
    );
    let game_path = "model/character/face/common/face_eyelash.dds";
    let mut set = pes_model::format::mtl::MaterialSet::read(&card_materials()).unwrap();
    let material = &mut set.materials[0];
    let MaterialEntry::Sampler(sampler) = material.entries[0].clone() else {
        panic!("the card material's first entry is its sampler");
    };
    material.entries[0] = MaterialEntry::Sampler(pes_model::format::mtl::Sampler {
        path: "team_common_folder/team_kit.dds".to_owned(),
        ..sampler.clone()
    });
    material.entries.insert(
        1,
        MaterialEntry::Sampler(pes_model::format::mtl::Sampler {
            path: game_path.to_owned(),
            ..sampler
        }),
    );
    sandbox.write(&format!("{}/face_high.mtl", player("06 - B")), &set.write());

    for (settings, version) in [
        (pes21_settings(&sandbox), 21),
        (pes_settings(&sandbox, 17), 17),
    ] {
        for command in ["check", "compile"] {
            let run = sandbox.run(&settings, &command_args(command));
            let messages = run.messages();
            assert!(
                messages.iter().all(|line| {
                    !line.contains("fmdl_texture_not_found")
                        && !line.contains("mtl_texture_not_found")
                }),
                "PES {version} {command}: {messages:?}"
            );
            assert_eq!(run.exit_code(), 0, "PES {version} {command}");
        }
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        if version == 21 {
            // Slot 05's packed model names the stand-ins in the game's spelling, the
            // keeper's kept at the `./` it was written at.
            let packed =
                FmdlFile::read(face_package(&entries).get("face_high.fmdl").unwrap()).unwrap();
            let names: Vec<String> = texture_paths(&packed)
                .unwrap()
                .into_iter()
                .filter(|path| {
                    path.file_name.starts_with("dummy_kit")
                        || path.file_name.starts_with("dummy_gk_kit")
                })
                .map(|path| format!("{}{}", path.directory, path.file_name))
                .collect();
            assert_eq!(
                names,
                [
                    "/Assets/pes16/model/character/common/714/sourceimages/dummy_kit.dds",
                    "./dummy_gk_kit.dds",
                    "/Assets/pes16/model/character/common/714/sourceimages/dummy_kit_srm.dds",
                ],
                "PES {version}"
            );
            // Slot 06's converted model names the same.
            let face_06 = "Asset/model/character/face/real/71406/#Win/face.fpk";
            let package = fpk::FpkFile::read(&entries[face_06]).unwrap();
            let converted = FmdlFile::read(package.get("face_high.fmdl").unwrap()).unwrap();
            let names: Vec<String> = texture_paths(&converted)
                .unwrap()
                .into_iter()
                .map(|path| format!("{}{}", path.directory, path.file_name))
                .collect();
            assert!(
                names.contains(
                    &"/Assets/pes16/model/character/common/714/sourceimages/dummy_kit.dds"
                        .to_owned()
                ),
                "PES {version}: {names:?}"
            );
        } else {
            // Slot 05's converted `.mtl` names every stand-in at the team's Common;
            // slot 06's `.mtl` names `dummy_kit` there and the game path as written.
            let face_05 = nested_entries(&entries[&face_cpk(5)]);
            let mtl_05 = &face_05[&format!("{}face_high.mtl", face_folder(5))];
            assert_eq!(
                sampler_paths(mtl_05),
                [
                    "model/character/uniform/common/714/dummy_kit.dds",
                    "model/character/uniform/common/714/dummy_gk_kit.dds",
                    "model/character/uniform/common/714/dummy_kit_srm.dds",
                    "/Assets/pes16/model/character/face/real/00000/sourceimages/shirt.dds",
                ],
                "PES {version}"
            );
            let face_06 = nested_entries(&entries[&face_cpk(6)]);
            let mtl_06 = &face_06[&format!("{}face_high.mtl", face_folder(6))];
            assert_eq!(
                sampler_paths(mtl_06),
                [
                    "model/character/uniform/common/714/dummy_kit.dds",
                    game_path,
                ],
                "PES {version}"
            );
        }
    }
}

// TC-TEX-13
#[test]
fn a_cube_map_dds_goes_out_as_it_is_on_pes_17_and_as_an_ftex_cube_map_on_pes_21() {
    let sandbox = Sandbox::new("tex_cube_map");
    let export = "co Midcup Env";
    // The bundled template environment map: a 128x128 DXT5 cube map with eight levels.
    let source = fs::read(templates_folder().join("env.dds")).unwrap();
    // No model: the textures task reads none, and the tracer's Fox model would add its PES 17
    // conversion's findings (dropped bone matrices) to lines that are about the texture.
    let player = format!("exports/{export}/Players/05 - A");
    sandbox.write(&format!("{player}/env.dds"), &source);
    let findings = [
        "Info export_identified [Keep] (team=/co/, id=714)",
        "Info team_colors_missing [Keep] ()",
    ];

    let run = sandbox.run(&pes_settings(&sandbox, 21), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(findings_of(&lines, export), findings, "PES 21: {lines:#?}");
    assert_eq!(run.exit_code(), 0, "PES 21");
    let mut entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let env = format!("{PLAYER_TEXTURES}/env.ftex");
    let emitted = entries
        .remove(&env)
        .unwrap_or_else(|| panic!("PES 21: no {env} among {:?}", entries.keys()));
    assert_eq!(
        emitted,
        ftex::dds_to_ftex(&source, ftex::ColorSpace::Normal).unwrap()
    );
    let info = ftex::info(&emitted).unwrap();
    assert!(info.is_cube_map);
    assert_eq!(
        info.texture_type, 0xD,
        "the normal-map type 0x9 with the cube bit"
    );

    let run = sandbox.run(&pes_settings(&sandbox, 17), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(findings_of(&lines, export), findings, "PES 17: {lines:#?}");
    assert_eq!(run.exit_code(), 0, "PES 17");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let env = "common/character1/model/character/uniform/common/714/05 - A/env.dds";
    let emitted = entries
        .get(env)
        .unwrap_or_else(|| panic!("PES 17: no {env} among {:?}", entries.keys()));
    assert_eq!(*emitted, source, "PES 17");
}

/// A 4x4 BC5_SNORM DDS (DX10 header), one zero block: a signed block format no target keeps,
/// which the decoder refuses as a kind of DDS.
fn bc5_snorm_dds() -> Vec<u8> {
    let mut dds = ftex::dds::header_bytes(PixelFormat::Bc5, 4, 4, 1);
    // The DX10 header's `dxgi_format`, right after the 4-byte magic and the 124-byte header.
    let unorm = u32::from_le_bytes(dds[128..132].try_into().unwrap());
    assert_eq!(unorm, 83, "DXGI_FORMAT_BC5_UNORM");
    dds[128..132].copy_from_slice(&84u32.to_le_bytes());
    dds.extend_from_slice(&[0; 16]);
    dds
}

// TC-TEX-14
#[test]
fn a_dds_of_a_kind_the_decoder_refuses_in_common_drops_that_file_alone() {
    let sandbox = Sandbox::new("tex_common_signed");
    let export = "exports/co Midcup Signed";
    sandbox.copy_tracer_face(&format!("{export}/Players/05 - A"));
    sandbox.write(&format!("{export}/Common/hair.dds"), &tracer_kit());
    sandbox.write(&format!("{export}/Common/bumps.dds"), &bc5_snorm_dds());

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Signed"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=fcl_hair.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/05 - A (file=glove_l.fmdl, count=2)",
            // The deep pass's, from the header, on the file before planning.
            "Error texture_codec_unsupported [DropFile] at Common/bumps.dds (file=bumps.dds)",
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let common: Vec<&str> = entries
        .keys()
        .filter_map(|path| path.strip_prefix("Asset/model/character/common/714/sourceimages/"))
        .collect();
    assert_eq!(common, ["#windx11/hair.ftex"], "hair kept, bumps left out");
}

// TC-TEX-15
#[test]
fn a_single_level_ftex_portrait_is_a_bc3_dds_with_the_full_mip_chain() {
    let sandbox = Sandbox::new("tex_ftex_portrait");
    let export = "co Midcup Portrait";
    // A 128x128 BC3 DDS with one level, zero blocks: a portrait's sides must be powers of two
    // whatever its level count (`texture_not_pow2`), so not `small_dds()`'s 12x12.
    let mut dds = ftex::dds::header_bytes(PixelFormat::Bc3, 128, 128, 1);
    dds.extend_from_slice(&[0; 128 * 128]);
    // The color space does not change the pixels; the encoder gives every Fox texture this one.
    let source = ftex::dds_to_ftex(&dds, ftex::ColorSpace::Normal).unwrap();
    let source_info = ftex::info(&source).unwrap();
    assert_eq!(source_info.mipmaps, 1, "a single-level source");
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/portrait.ftex"),
        &source,
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "{lines:#?}"
    );
    assert_eq!(run.exit_code(), 0);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let portrait = &entries["common/render/symbol/player/71405.dds"];
    let decoded = decode(portrait, SourceFormat::Dds).unwrap();
    assert!(
        matches!(
            decoded.blocks,
            Some(Blocks {
                codec: BlockCodec::Bc3,
                ..
            })
        ),
        "BC3 blocks"
    );
    let (width, height) = (u32::from(source_info.width), u32::from(source_info.height));
    assert_eq!((decoded.width, decoded.height), (width, height));
    let full_chain = width.max(height).ilog2() + 1;
    assert_eq!(decoded.mips.len(), usize::try_from(full_chain).unwrap());
}

// TC-PRT-04
#[test]
fn a_dx10_header_dds_portrait_goes_out_under_the_legacy_header_with_its_blocks() {
    let sandbox = Sandbox::new("tex_dx10_portrait");
    let export = "co Midcup Portrait";
    // The shape of the portrait that crashed PES 19: 128x128 BC3, one level, under a DX10
    // header whose DXGI id is 78 (BC3 sRGB). BC7's header is a DX10 one; its id is replaced.
    let mut source = ftex::dds::header_bytes(PixelFormat::Bc7, 128, 128, 1);
    assert_eq!(source.len(), 148, "a DX10 header");
    source[128..132].copy_from_slice(&78u32.to_le_bytes());
    let blocks: Vec<u8> = (0..=u8::MAX).cycle().take(128 * 128).collect();
    source.extend_from_slice(&blocks);
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/portrait.dds"),
        &source,
    );
    let mut expected = ftex::dds::header_bytes(PixelFormat::Bc3, 128, 128, 1);
    expected.extend_from_slice(&blocks);
    assert_eq!(expected.len(), 16512);
    assert_eq!(&expected[84..88], b"DXT5");

    for (version, path) in [
        (21, "common/render/symbol/player/71405.dds"),
        (17, "common/render/symbol/player/player_71405.dds"),
    ] {
        let run = sandbox.run(
            &pes_settings(&sandbox, version),
            &["compile", "--no-deploy"],
        );

        let lines = run.messages();
        assert_eq!(
            findings_of(&lines, export),
            [
                "Info export_identified [Keep] (team=/co/, id=714)",
                "Info team_colors_missing [Keep] ()",
                "Info portrait_header_rewritten [Keep] at Players/05 - A/portrait.dds (file=portrait.dds, dxgi=78)",
            ],
            "PES {version}: {lines:#?}"
        );
        assert_eq!(run.exit_code(), 0, "PES {version}");
        let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
        let portrait = entries
            .get(path)
            .unwrap_or_else(|| panic!("PES {version}: no {path} among {:?}", entries.keys()));
        // Not `assert_eq!`: a mismatch would print 16 KB twice.
        assert!(
            *portrait == expected,
            "PES {version}: {} bytes, not the source's blocks under the DXT5 header",
            portrait.len()
        );
    }
}

// TC-PRT-05
#[test]
fn a_bc7_portrait_is_encoded_to_bc3_on_pes_18_and_re_headered_on_pes_21() {
    let sandbox = Sandbox::new("tex_bc7_portrait");
    let export = "co Midcup Portrait";
    // A 128x128 single-level BC7 DDS under a DX10 header, arbitrary blocks: the cup's
    // BC7 portraits' shape.
    let mut source = ftex::dds::header_bytes(PixelFormat::Bc7, 128, 128, 1);
    assert_eq!(source.len(), 148, "a DX10 header");
    let blocks: Vec<u8> = (0..=u8::MAX).cycle().take(128 * 128).collect();
    source.extend_from_slice(&blocks);
    sandbox.write(
        &format!("exports/{export}/Players/05 - A/portrait.dds"),
        &source,
    );

    // PES 18, which reads BC1-BC3 only: the source is decoded and encoded like a raster
    // one: a 128x128 DXT5 DDS under the legacy header with the full mip chain.
    let run = sandbox.run(&pes_settings(&sandbox, 18), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
        ],
        "PES 18: {lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "PES 18");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let portrait = &entries["common/render/symbol/player/player_71405.dds"];
    assert_eq!(&portrait[84..88], b"DXT5", "PES 18: the legacy header");
    let decoded = decode(portrait, SourceFormat::Dds).unwrap();
    assert!(
        matches!(
            decoded.blocks,
            Some(Blocks {
                codec: BlockCodec::Bc3,
                ..
            })
        ),
        "PES 18: BC3 blocks"
    );
    assert_eq!(
        (decoded.width, decoded.height, decoded.mips.len()),
        (128, 128, 8),
        "PES 18: the full mip chain"
    );

    // PES 21: the source's BC7 blocks re-headered, the finding as TC-PRT-04's.
    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);
    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, export),
        [
            "Info export_identified [Keep] (team=/co/, id=714)",
            "Info team_colors_missing [Keep] ()",
            "Info portrait_header_rewritten [Keep] at Players/05 - A/portrait.dds (file=portrait.dds, dxgi=98)",
        ],
        "PES 21: {lines:#?}"
    );
    assert_eq!(run.exit_code(), 0, "PES 21");
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    let portrait = &entries["common/render/symbol/player/71405.dds"];
    assert!(
        *portrait == source,
        "PES 21: {} bytes, not the source's blocks under the header it wrote",
        portrait.len()
    );
}
