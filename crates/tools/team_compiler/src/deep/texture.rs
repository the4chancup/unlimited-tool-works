//! The deep pass's texture checks (`spec/team_compiler/messages.md` "Textures"), from a
//! texture's header alone: a file renamed from another format, a header the converter cannot
//! read or refuses, a side under one block, and the size rules a texture is held to by where
//! it is and the target's engine.

use dds_convert::{ConvertError, SourceFormat};
use ftex::FtexError;
use pes_version::{Engine, PesVersion};

use crate::messages::Code;

/// The size rule a texture is held to past the smallest side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SizeRule {
    /// A kit's main texture on PES 15-17: `kit_texture_too_big` past 2048 pixels a side or
    /// on a side that is not a power of two.
    MainKit,
    /// Any other texture on a Fox target, a kit's main texture included: `texture_not_pow2`
    /// when it carries a mip chain and neither side is a power of two.
    FoxMipmapped,
    /// Any other texture on a pre-Fox target: `texture_not_div4` on a side that is not a
    /// multiple of 4, since every pre-Fox texture is written block-compressed.
    PreFox,
    /// A portrait, a `Portraits/` file or a player folder's `portrait.*`, on any target:
    /// `texture_not_pow2` on a side that is not a power of two, whatever its level count.
    Portrait,
}

impl SizeRule {
    /// The rule of every texture on `version`'s engine but a kit's main one on PES 15-17.
    pub(super) fn of(version: PesVersion) -> SizeRule {
        match version.engine() {
            Engine::Fox => SizeRule::FoxMipmapped,
            Engine::PreFox => SizeRule::PreFox,
        }
    }
}

/// The accepted formats that open with a fixed signature, and that signature; WebP's `RIFF`
/// is checked with its `WEBP` tag below, and TGA has none.
const SIGNATURES: [(&[u8], SourceFormat); 9] = [
    (b"DDS ", SourceFormat::Dds),
    (b"FTEX", SourceFormat::Ftex),
    (b"\x89PNG", SourceFormat::Png),
    (&[0xff, 0xd8, 0xff], SourceFormat::Jpeg),
    (b"BM", SourceFormat::Bmp),
    (b"II*\0", SourceFormat::Tiff),
    (b"MM\0*", SourceFormat::Tiff),
    // JPEG XL as a bare codestream, and in its ISOBMFF container (the `JXL ` box).
    (&[0xff, 0x0a], SourceFormat::Jxl),
    (b"\0\0\0\x0cJXL \r\n\x87\n", SourceFormat::Jxl),
];

/// The accepted format whose signature `bytes` open with, if any.
fn signature_format(bytes: &[u8]) -> Option<SourceFormat> {
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(SourceFormat::WebP);
    }
    SIGNATURES
        .iter()
        .find(|(signature, _)| bytes.starts_with(signature))
        .map(|(_, format)| *format)
}

/// The one texture finding `bytes`, a texture in `format` held to `rule`, get, by the first
/// rule that fires: `texture_type_mismatch` when they open with another accepted format's
/// signature (a file renamed, not resaved; its header is not read), then, from the header,
/// `texture_too_small` on a side under 4 pixels (one block), then the size rule. A header
/// the probe refuses is a finding on the file too (`probe_failure`), so the texture's task never
/// meets it: a Common texture's would fail every Common texture with it.
pub(super) fn texture_finding(format: SourceFormat, rule: SizeRule, bytes: &[u8]) -> Option<Code> {
    if signature_format(bytes).is_some_and(|sniffed| sniffed != format) {
        return Some(Code::TextureTypeMismatch);
    }
    // The probe walks a 2D texture and refuses a cube map, which the game takes and the
    // compile never decodes (`processing::texture::convert` sends it out as it is, or through
    // the FTEX container conversion): its header is not read here.
    if format == SourceFormat::Dds && ftex::dds::is_cube_map(bytes) {
        return None;
    }
    let probe = match dds_convert::probe(bytes, format) {
        Ok(probe) => probe,
        Err(error) => return Some(probe_failure(&error)),
    };
    if probe.width < 4 || probe.height < 4 {
        return Some(Code::TextureTooSmall);
    }
    let power_of_two = probe.width.is_power_of_two() && probe.height.is_power_of_two();
    match rule {
        SizeRule::MainKit => (probe.width > 2048 || probe.height > 2048 || !power_of_two)
            .then_some(Code::KitTextureTooBig),
        SizeRule::FoxMipmapped => {
            (probe.mipmaps > 1 && !probe.width.is_power_of_two() && !probe.height.is_power_of_two())
                .then_some(Code::TextureNotPow2)
        }
        SizeRule::PreFox => {
            (probe.width % 4 != 0 || probe.height % 4 != 0).then_some(Code::TextureNotDiv4)
        }
        SizeRule::Portrait => (!power_of_two).then_some(Code::TextureNotPow2),
    }
}

/// The finding on a texture whose header `dds_convert::probe` refuses with `error`: a codec
/// `dds_convert` cannot convert, or a kind of DDS `ftex::dds::read_layout` refuses (a signed
/// block format, a volume, an array, a paletted DDS), is `texture_codec_unsupported`; any
/// other error, a header cut short or bytes that are no texture, `texture_unreadable`. The
/// two codec arms are the ones `processing::texture::conversion_failure` calls a codec
/// refusal; kept apart from it, since that one builds a task failure and this a code.
fn probe_failure(error: &ConvertError) -> Code {
    match error {
        ConvertError::Unsupported(_) | ConvertError::Ftex(FtexError::UnsupportedDds(_)) => {
            Code::TextureCodecUnsupported
        }
        ConvertError::Ftex(_)
        | ConvertError::Wesys(_)
        | ConvertError::Image(_)
        | ConvertError::InvalidDecoded(_)
        | ConvertError::Truncated => Code::TextureUnreadable,
    }
}

#[cfg(test)]
mod tests {
    use aesthetics_export::Disposition;

    use super::*;
    use crate::deep::tests::{
        bc1_dds, bc1_dds_levels, findings_for, findings_of, fixture, folder, texture,
        texture_finding_on,
    };
    use crate::testing::scratch;

    #[test]
    fn each_accepted_format_s_signature_is_recognized_and_tga_has_none() {
        // A real file of each format opens with its signature (the kit and portrait fixtures
        // are real encoder output); a signature pasted onto nothing is still that format's.
        assert_eq!(
            signature_format(&fixture("tracer/studio/egg Midcup Tracer/Kits/g1/kit.dds")),
            Some(SourceFormat::Dds),
            "dds"
        );
        assert_eq!(
            signature_format(&texture("kit.png")),
            Some(SourceFormat::Png),
            "png"
        );
        assert_eq!(
            signature_format(&texture("portrait.webp")),
            Some(SourceFormat::WebP),
            "webp"
        );
        assert_eq!(
            signature_format(&texture("kit.jxl")),
            Some(SourceFormat::Jxl),
            "jxl"
        );
        assert_eq!(
            signature_format(&texture("kit_back.tga")),
            None,
            "tga has no signature"
        );
        for (bytes, format) in [
            (&b"FTEX\x00\x00\x00\x00"[..], SourceFormat::Ftex),
            (&[0xff, 0xd8, 0xff, 0xe0, 0, 0x10], SourceFormat::Jpeg),
            (&b"BM\x36\x00\x00\x00"[..], SourceFormat::Bmp),
            (&b"II*\0\x08\x00\x00\x00"[..], SourceFormat::Tiff),
            (&b"MM\0*\x00\x00\x00\x08"[..], SourceFormat::Tiff),
            (&b"RIFF\x00\x00\x00\x00WEBPVP8 "[..], SourceFormat::WebP),
            // JPEG XL's bare codestream, then its ISOBMFF container (the `JXL ` box).
            (&[0xff, 0x0a, 0xc3, 0xc1], SourceFormat::Jxl),
            (
                &b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a\x00\x00\x00\x14ftyp"[..],
                SourceFormat::Jxl,
            ),
        ] {
            assert_eq!(signature_format(bytes), Some(format), "{format:?}");
        }
        // A RIFF that is not WebP, and bytes opening with none of them, are no format.
        assert_eq!(signature_format(b"RIFF\x00\x00\x00\x00WAVEfmt "), None);
        assert_eq!(signature_format(b"RIFF"), None, "too short for the tag");
        assert_eq!(signature_format(b"not a texture"), None);
        assert_eq!(signature_format(b""), None);
    }

    #[test]
    fn each_texture_rule_drops_its_player_folder_with_one_finding() {
        let temp = scratch("deep_texture_rules");
        let findings = findings_of(
            temp.path(),
            &[
                ("Players/03 - A/tiny.png", texture("tiny.png")),
                ("Players/04 - B/face/odd.png", texture("odd.png")),
                // PNG bytes under a `.dds` and a `.tga` name: renamed, not resaved; TGA has
                // no signature, but PNG's is another's.
                ("Players/05 - C/skin.dds", texture("portrait.png")),
                ("Players/06 - D/skin.tga", texture("tiny.png")),
            ],
            &[],
        );
        assert_eq!(
            findings,
            [
                texture_finding_on(
                    "texture_too_small",
                    &folder("Players/03 - A"),
                    "tiny.png",
                    Disposition::DropFolder,
                    true
                ),
                texture_finding_on(
                    "texture_not_pow2",
                    &folder("Players/04 - B"),
                    "face/odd.png",
                    Disposition::DropFolder,
                    true
                ),
                texture_finding_on(
                    "texture_type_mismatch",
                    &folder("Players/05 - C"),
                    "skin.dds",
                    Disposition::DropFolder,
                    false
                ),
                texture_finding_on(
                    "texture_type_mismatch",
                    &folder("Players/06 - D"),
                    "skin.tga",
                    Disposition::DropFolder,
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_single_level_odd_dds_on_fox_and_an_odd_png_on_pre_fox_pass() {
        let temp = scratch("deep_texture_fox_single_level");
        let findings = findings_of(
            temp.path(),
            &[("Players/03 - A/skin.dds", bc1_dds(300, 300))],
            &[],
        );
        assert_eq!(findings, []);
        let temp = scratch("deep_texture_pre_fox");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[("Players/03 - A/odd.png", texture("odd.png"))],
            &[],
            &[],
        );
        assert_eq!(findings, []);
    }

    #[test]
    fn a_pre_fox_texture_whose_side_is_no_multiple_of_4_drops_its_folder_even_with_pass_through() {
        let temp = scratch("deep_texture_not_div4");
        let findings = findings_for(
            PesVersion::Pes17,
            temp.path(),
            &[("Players/03 - A/skin.dds", bc1_dds(1002, 1004))],
            &[],
            &[],
        );
        assert_eq!(
            findings,
            [texture_finding_on(
                "texture_not_div4",
                &folder("Players/03 - A"),
                "skin.dds",
                Disposition::DropFolder,
                false
            )]
        );
    }

    #[test]
    fn a_texture_whose_header_is_cut_is_unreadable_and_drops_its_folder() {
        let temp = scratch("deep_texture_cut");
        let findings = findings_of(
            temp.path(),
            &[
                (
                    "Players/03 - A/skin.dds",
                    texture("bc7.dds")[..100].to_vec(),
                ),
                (
                    "Players/03 - A/hair.png",
                    texture("tiny.png")[..20].to_vec(),
                ),
            ],
            &[],
        );
        // Each in file order, on the folder as every texture finding in a model folder, and
        // never eligible: packed, the folder's texture task would fail on it.
        assert_eq!(
            findings,
            ["hair.png", "skin.dds"].map(|file| texture_finding_on(
                "texture_unreadable",
                &folder("Players/03 - A"),
                file,
                Disposition::DropFolder,
                false
            ))
        );
    }

    #[test]
    fn a_texture_of_an_unsupported_codec_drops_its_folder_and_is_never_passed_through() {
        let temp = scratch("deep_texture_codec");
        // A volume DDS: packed, the folder's textures task would fail on it, so it is never
        // pass-through-eligible.
        let mut volume = bc1_dds(4, 4);
        volume[112..116].copy_from_slice(&0x0020_0000_u32.to_le_bytes());
        let findings = findings_of(temp.path(), &[("Players/03 - A/skin.dds", volume)], &[]);
        assert_eq!(
            findings,
            [texture_finding_on(
                "texture_codec_unsupported",
                &folder("Players/03 - A"),
                "skin.dds",
                Disposition::DropFolder,
                false
            )]
        );
    }

    #[test]
    fn a_texture_the_probe_refuses_is_unreadable_or_of_an_unsupported_codec() {
        let rule = SizeRule::FoxMipmapped;
        assert_eq!(
            texture_finding(SourceFormat::Dds, rule, b"junk"),
            Some(Code::TextureUnreadable)
        );
        // A 4x4 BC1 DDS with `DDSCAPS2_VOLUME` (0x200000) set in its `capabilities2`, the
        // 4-byte field at file offset 112 (the magic, then 108 bytes into the header): a
        // volume texture, a kind of DDS the target cannot be given.
        let mut volume = bc1_dds(4, 4);
        assert_eq!(texture_finding(SourceFormat::Dds, rule, &volume), None);
        volume[112..116].copy_from_slice(&0x0020_0000_u32.to_le_bytes());
        assert_eq!(
            texture_finding(SourceFormat::Dds, rule, &volume),
            Some(Code::TextureCodecUnsupported)
        );
    }

    #[test]
    fn a_kit_s_main_texture_is_held_to_2048_on_pre_fox_and_the_plain_rule_on_fox() {
        let temp = scratch("deep_texture_kits");
        let files = [
            ("Kits/p1/kit.dds", bc1_dds(4096, 4096)),
            ("Kits/p2/kit.png", texture("odd.png")),
            ("Kits/g1/kit.dds", bc1_dds(2048, 2048)),
            ("Kits/p3/kit.png", texture("kit.png")),
            ("Kits/p3/kit_back.png", texture("odd.png")),
        ];
        // PES 15-17 hold a kit's main texture to 2048 pixels a side, both powers of two.
        assert_eq!(
            findings_for(PesVersion::Pes17, temp.path(), &files, &[], &[]),
            [
                texture_finding_on(
                    "kit_texture_too_big",
                    &folder("Kits/p1"),
                    "kit.dds",
                    Disposition::DropFolder,
                    true
                ),
                texture_finding_on(
                    "kit_texture_too_big",
                    &folder("Kits/p2"),
                    "kit.png",
                    Disposition::DropFolder,
                    true
                ),
            ]
        );
        // On Fox it is any other texture: the 4096 single-level kit passes, and a mipped
        // texture is refused only with neither side a power of two.
        assert_eq!(
            findings_of(temp.path(), &files, &[]),
            [
                texture_finding_on(
                    "texture_not_pow2",
                    &folder("Kits/p2"),
                    "kit.png",
                    Disposition::DropFolder,
                    true
                ),
                texture_finding_on(
                    "texture_not_pow2",
                    &folder("Kits/p3"),
                    "kit_back.png",
                    Disposition::DropFolder,
                    true
                ),
            ]
        );
    }

    #[test]
    fn each_size_rule_reads_each_side_on_its_own() {
        // One side past a bound and the other not, and a side exactly at a bound. `levels`
        // is the DDS's own mip count: only `FoxMipmapped` reads it, its rule asking for a
        // chain before a non-power-of-two side matters at all.
        for (rule, width, height, levels, expected) in [
            (SizeRule::FoxMipmapped, 3, 8, 1, Some(Code::TextureTooSmall)),
            (SizeRule::FoxMipmapped, 8, 3, 1, Some(Code::TextureTooSmall)),
            (SizeRule::FoxMipmapped, 4, 8, 1, None),
            (SizeRule::FoxMipmapped, 8, 4, 1, None),
            (SizeRule::MainKit, 256, 300, 1, Some(Code::KitTextureTooBig)),
            (SizeRule::MainKit, 300, 256, 1, Some(Code::KitTextureTooBig)),
            (
                SizeRule::MainKit,
                4096,
                2048,
                1,
                Some(Code::KitTextureTooBig),
            ),
            (
                SizeRule::MainKit,
                2048,
                4096,
                1,
                Some(Code::KitTextureTooBig),
            ),
            (SizeRule::MainKit, 2048, 1024, 1, None),
            (SizeRule::PreFox, 3, 8, 1, Some(Code::TextureTooSmall)),
            (SizeRule::PreFox, 3, 1002, 1, Some(Code::TextureTooSmall)),
            (SizeRule::PreFox, 1002, 1004, 1, Some(Code::TextureNotDiv4)),
            (SizeRule::PreFox, 1004, 1002, 1, Some(Code::TextureNotDiv4)),
            (SizeRule::PreFox, 1002, 1002, 1, Some(Code::TextureNotDiv4)),
            (SizeRule::PreFox, 1004, 1004, 1, None),
            (SizeRule::PreFox, 300, 300, 1, None),
            // One power-of-two side is enough on a Fox texture with a mip chain; neither
            // being one is the finding, and a single level passes at any size.
            (SizeRule::FoxMipmapped, 512, 461, 10, None),
            (SizeRule::FoxMipmapped, 461, 512, 10, None),
            (
                SizeRule::FoxMipmapped,
                300,
                300,
                9,
                Some(Code::TextureNotPow2),
            ),
            (SizeRule::FoxMipmapped, 300, 300, 1, None),
            (SizeRule::FoxMipmapped, 1002, 1004, 1, None),
            (SizeRule::FoxMipmapped, 1004, 1002, 1, None),
            (SizeRule::FoxMipmapped, 1002, 1002, 1, None),
            (SizeRule::FoxMipmapped, 1004, 1004, 1, None),
            (SizeRule::Portrait, 256, 300, 1, Some(Code::TextureNotPow2)),
            (SizeRule::Portrait, 300, 256, 1, Some(Code::TextureNotPow2)),
            (SizeRule::Portrait, 4096, 8, 1, None),
            (SizeRule::Portrait, 3, 8, 1, Some(Code::TextureTooSmall)),
        ] {
            assert_eq!(
                texture_finding(
                    SourceFormat::Dds,
                    rule,
                    &bc1_dds_levels(width, height, levels)
                ),
                expected,
                "{rule:?} {width}x{height} x{levels}"
            );
        }
    }
}
