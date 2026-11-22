//! `compile` over exports holding a root logo: the game's three square PNGs, made from the
//! export's `logo*` image by its mode and, for the smallest, from its `logo_small*` image when
//! there is one.

use std::collections::BTreeMap;

use dds_convert::{SourceFormat, decode, encode_png};

use crate::common::Sandbox;
use crate::compile::{
    compiled_kits, compiled_players, cpk_entries, pes_settings, pes21_settings, tracer_kit,
    tracer_portrait,
};
use crate::referees::write_ref_a;
use crate::{CLEAN_PLAYER, TEAM_COLORS_MISSING, clean_model, findings_of};

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// A `width` x `height` PNG whose texel at (x, y) is `color(x, y)`.
pub(crate) fn png(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&color(x, y));
        }
    }
    encode_png(&pixels, width, height).unwrap()
}

/// The logo PNGs of the CPK the sandbox's last `compile` wrote, by path, each decoded to its
/// side and RGBA8 pixels, asserting each is square.
fn compiled_logos(sandbox: &Sandbox) -> BTreeMap<String, (u32, Vec<u8>)> {
    cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"))
        .into_iter()
        .filter(|(path, _)| path.starts_with("common/render/symbol/flag/"))
        .map(|(path, bytes)| {
            let mut decoded = decode(&bytes, SourceFormat::Png).unwrap();
            assert_eq!(decoded.width, decoded.height, "{path}");
            (path, (decoded.width, decoded.mips.swap_remove(0)))
        })
        .collect()
}

/// PES 21's three logo paths of team `team_id`, largest first.
fn logo_paths(team_id: u16) -> [String; 3] {
    ["_r_ll", "_r_l", "_r"]
        .map(|size| format!("common/render/symbol/flag/e_000{team_id}{size}.png"))
}

/// The logo of team `team_id` among `logos`: its three PNGs largest first, as (side, pixels).
fn team_logo(logos: &BTreeMap<String, (u32, Vec<u8>)>, team_id: u16) -> [&(u32, Vec<u8>); 3] {
    logo_paths(team_id).map(|path| &logos[&path])
}

/// The texel at (x, y) of `pixels`, `side` pixels wide.
fn texel(pixels: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * side + x) * 4) as usize;
    pixels[at..at + 4].try_into().unwrap()
}

/// Asserts every texel of the `side`-pixel `pixels` is `expected(x, y)`.
fn assert_texels(pixels: &[u8], side: u32, expected: impl Fn(u32, u32) -> [u8; 4]) {
    for y in 0..side {
        for x in 0..side {
            assert_eq!(
                texel(pixels, side, x, y),
                expected(x, y),
                "({x}, {y}) of {side}"
            );
        }
    }
}

fn identified(team: &str, id: u16) -> String {
    format!("Info export_identified [Keep] (team=/{team}/, id={id})")
}

// TC-ROOT-06
#[test]
fn a_wide_logo_compiled_for_pes_21_gives_three_letterboxed_squares() {
    let sandbox = Sandbox::new("logo_fit");
    sandbox.write(
        "exports/co Midcup Logo/logo.png",
        &png(1000, 600, |_, _| RED),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Midcup Logo"),
        [
            identified("co", 714).as_str(),
            TEAM_COLORS_MISSING,
            "Info logo_fit_applied [Keep] at logo.png (file=logo.png, mode=fit)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let logos = compiled_logos(&sandbox);
    let paths: Vec<&str> = logos.keys().map(String::as_str).collect();
    // In the CPK's own order: `_r.png` sorts before `_r_l.png`.
    assert_eq!(
        paths,
        [
            "common/render/symbol/flag/e_000714_r.png",
            "common/render/symbol/flag/e_000714_r_l.png",
            "common/render/symbol/flag/e_000714_r_ll.png",
        ]
    );
    // 512 x 307 at row 102, 256 x 154 at 51, 128 x 77 at 25.
    let expected = [(512, 102..409), (256, 51..205), (128, 25..102)];
    for ((side, pixels), (expected_side, rows)) in team_logo(&logos, 714).into_iter().zip(expected)
    {
        assert_eq!(*side, expected_side);
        assert_texels(
            pixels,
            *side,
            |_, y| {
                if rows.contains(&y) { RED } else { CLEAR }
            },
        );
    }
}

// TC-ROOT-07
#[test]
fn logos_tagged_crop_and_stretch_fill_the_square_and_a_small_one_is_upscaled() {
    let sandbox = Sandbox::new("logo_modes");
    let three_bands = png(1000, 600, |x, _| match x {
        0..200 => BLUE,
        200..800 => RED,
        _ => GREEN,
    });
    sandbox.write("exports/co Midcup Crop/logo_crop.png", &three_bands);
    // The same image stretched: the bands its centre crop removes are what it keeps.
    sandbox.write("exports/dbg Midcup Stretch/logo_stretch.png", &three_bands);
    sandbox.write(
        "exports/egg Midcup Small/logo.png",
        &png(300, 300, |_, _| GREEN),
    );

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Crop"),
        [
            identified("co", 714).as_str(),
            TEAM_COLORS_MISSING,
            "Info logo_fit_applied [Keep] at logo_crop.png (file=logo_crop.png, mode=crop)",
        ]
    );
    assert_eq!(
        findings_of(&messages, "dbg Midcup Stretch"),
        [
            identified("dbg", 790).as_str(),
            TEAM_COLORS_MISSING,
            "Info logo_fit_applied [Keep] at logo_stretch.png (file=logo_stretch.png, mode=stretch)",
        ]
    );
    assert_eq!(
        findings_of(&messages, "egg Midcup Small"),
        [
            identified("egg", 792).as_str(),
            TEAM_COLORS_MISSING,
            "Warning logo_upscaled [Keep] at logo.png (file=logo.png, size=300x300, target=512)",
        ]
    );
    assert_eq!(run.exit_code(), 0);
    let logos = compiled_logos(&sandbox);
    assert_eq!(logos.len(), 9);
    // The crop keeps the red centre alone.
    for (side, pixels) in team_logo(&logos, 714) {
        assert_texels(pixels, *side, |_, _| RED);
    }
    // The stretch fills the square with the bands a crop would remove: no texel is
    // transparent, blue on the left, red in the middle, green on the right.
    for (side, pixels) in team_logo(&logos, 790) {
        for y in 0..*side {
            for x in 0..*side {
                assert_eq!(texel(pixels, *side, x, y)[3], 255, "({x}, {y}) of {side}");
            }
        }
        assert_eq!(texel(pixels, *side, 0, 0), BLUE, "{side}");
        assert_eq!(texel(pixels, *side, *side / 2, *side / 2), RED, "{side}");
        assert_eq!(texel(pixels, *side, side - 1, side - 1), GREEN, "{side}");
    }
    for (side, pixels) in team_logo(&logos, 792) {
        assert_texels(pixels, *side, |_, _| GREEN);
    }
}

// TC-ROOT-08
#[test]
fn a_small_logo_gives_the_128_pixel_one_and_an_undecodable_one_drops_the_whole_logo() {
    let sandbox = Sandbox::new("logo_small");
    sandbox.write(
        "exports/co Midcup Pair/logo.png",
        &png(600, 600, |_, _| RED),
    );
    sandbox.write(
        "exports/co Midcup Pair/logo_small.png",
        &png(200, 200, |_, _| BLUE),
    );
    sandbox.write(
        "exports/dbg Midcup Broken/logo.png",
        &png(600, 600, |_, _| RED),
    );
    sandbox.write("exports/dbg Midcup Broken/logo_small.png", b"not an image");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let messages = run.messages();
    assert_eq!(
        findings_of(&messages, "co Midcup Pair"),
        [identified("co", 714).as_str(), TEAM_COLORS_MISSING]
    );
    assert_eq!(
        findings_of(&messages, "dbg Midcup Broken"),
        [
            "Error logo_file_invalid [DropFile] at logo_small.png (file=logo_small.png, error=image decode failed: Format error decoding Png: Invalid PNG signature.)",
            identified("dbg", 790).as_str(),
            TEAM_COLORS_MISSING,
        ]
    );
    assert_eq!(run.exit_code(), 1);
    let logos = compiled_logos(&sandbox);
    let paths: Vec<&String> = logos.keys().collect();
    let mut pair = logo_paths(714).to_vec();
    pair.sort();
    assert_eq!(paths, pair.iter().collect::<Vec<_>>(), "no logo of 790");
    let colors: Vec<(u32, [u8; 4])> = team_logo(&logos, 714)
        .into_iter()
        .map(|(side, pixels)| {
            let color = texel(pixels, *side, 0, 0);
            assert_texels(pixels, *side, |_, _| color);
            (*side, color)
        })
        .collect();
    assert_eq!(colors, [(512, RED), (256, RED), (128, BLUE)]);
}

/// The findings of a `/co/` export holding kits p1 and g1 with no `config.toml` or
/// `colors.txt`, after its logo's and its players' own.
pub(crate) const CO_KITS_FINDINGS: [&str; 6] = [
    "Info export_identified [Keep] (team=/co/, id=714)",
    TEAM_COLORS_MISSING,
    "Info kit_config_generated [Keep] at Kits/p1 ()",
    "Info kit_config_generated [Keep] at Kits/g1 ()",
    "Info kit_colors_derived [Keep] at Kits/p1 ()",
    "Info kit_colors_derived [Keep] at Kits/g1 ()",
];

/// Writes `exports/<name>`: player 03 with his portrait, kits p1 and g1, and no root file.
fn write_full_export(sandbox: &Sandbox, name: &str) {
    sandbox.write(&format!("exports/{name}/{CLEAN_PLAYER}"), &clean_model());
    sandbox.write(
        &format!("exports/{name}/Players/03 - A/portrait.dds"),
        &tracer_portrait(),
    );
    for slot in ["p1", "g1"] {
        sandbox.write(
            &format!("exports/{name}/Kits/{slot}/kit.dds"),
            &tracer_kit(),
        );
    }
}

// TC-ROOT-15
#[test]
fn a_full_export_with_no_logo_file_is_logo_missing_and_compiles_without_a_logo() {
    let sandbox = Sandbox::new("logo_missing_full");
    write_full_export(&sandbox, "co Full Spring");

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Spring"),
        [&["Error logo_missing [Keep] ()"][..], &CO_KITS_FINDINGS].concat()
    );
    assert_eq!(run.exit_code(), 1);
    assert_eq!(compiled_players(&sandbox), [71403]);
    assert_eq!(compiled_kits(&sandbox), ["u0714g1", "u0714p1"]);
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.contains("e_000714_")),
        "{:?}",
        entries.keys().collect::<Vec<_>>()
    );
}

// TC-ROOT-15
#[test]
fn a_midcup_or_referee_export_with_no_logo_and_a_full_one_with_a_refused_logo_have_no_logo_missing()
{
    let sandbox = Sandbox::new("logo_missing_not_full");
    sandbox.write(
        &format!("exports/co Midcup Day 5/{CLEAN_PLAYER}"),
        &clean_model(),
    );
    write_ref_a(&sandbox, &["01"]);

    let run = sandbox.run(&pes21_settings(&sandbox), &["compile", "--no-deploy"]);

    let lines = run.messages();
    assert_eq!(
        findings_of(&lines, "co Midcup Day 5"),
        [identified("co", 714).as_str(), TEAM_COLORS_MISSING]
    );
    assert_eq!(
        findings_of(&lines, "refs Cup"),
        [
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=boots.fmdl, count=1662)",
            "Info fmdl_weights_not_normalized [Keep] at Players/Ref A (file=face_high.fmdl, count=1662)",
            "Info export_identified [Keep] (team=referees)",
        ]
    );
    assert_eq!(run.exit_code(), 0);

    let refused = Sandbox::new("logo_missing_refused");
    write_full_export(&refused, "co Full Autumn");
    refused.write("exports/co Full Autumn/logo.png", b"not an image");

    let run = refused.run(&pes21_settings(&refused), &["compile", "--no-deploy"]);

    assert_eq!(
        findings_of(&run.messages(), "co Full Autumn"),
        [
            &["Error logo_file_invalid [DropFile] at logo.png (file=logo.png, error=image decode failed: Format error decoding Png: Invalid PNG signature.)"][..],
            &CO_KITS_FINDINGS,
        ]
        .concat()
    );
    assert_eq!(run.exit_code(), 1);
}

// TC-ROOT-13
#[test]
fn a_logo_compiled_for_pes_19_gets_the_emblem_names() {
    let sandbox = Sandbox::new("logo_pes19");
    sandbox.write(
        "exports/co Midcup Logo/logo.png",
        &png(600, 600, |_, _| RED),
    );

    let run = sandbox.run(&pes_settings(&sandbox, 19), &["compile", "--no-deploy"]);

    assert_eq!(run.exit_code(), 0);
    let logos = compiled_logos(&sandbox);
    // In the CPK's own order: `_r.png` sorts before `_r_l.png`.
    let paths: Vec<&str> = logos.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        [
            "common/render/symbol/flag/emblem_0714_r.png",
            "common/render/symbol/flag/emblem_0714_r_l.png",
            "common/render/symbol/flag/emblem_0714_r_ll.png",
        ]
    );
    let entries = cpk_entries(&sandbox.root.join("output/4cc_99_test.cpk"));
    assert!(
        entries.keys().all(|path| !path.contains("e_000714_")),
        "{:?}",
        entries.keys().collect::<Vec<_>>()
    );
}
