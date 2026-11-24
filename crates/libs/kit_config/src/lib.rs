//! Kit config codec: the 120-byte binary, its TOML form, validation, and the
//! FPC kit values (which live in the `fpc` crate).
//!
//! Decoding keeps every bit not yet decoded in [`KitConfig::unknown`] so real
//! configs round-trip bit-identically; the TOML form carries plain,
//! version-neutral values and the binary packing is applied on emission.

mod binary;
mod fpc;
mod model;
mod names;
mod toml_form;
mod validate;

pub use fpc::{apply_fpc, matches_fpc};
pub use model::{
    BackNumber, Badges, ChestNumber, Colors, FieldLimit, KitConfig, LongSleeves, NameShape,
    NameText, Numbers, Position, Rgb, Shirt, ShortSleeves, Shorts, ShortsNumber, Side,
    TEXTURE_NAME_FIELDS, field_limits,
};
pub use names::{KitSlot, TexturePresence, texture_names};
pub use validate::{Finding, OutOfRange, Severity, validate};

use pes_version::PesVersion;

impl KitConfig {
    /// The template every generated config starts from (the bundled template
    /// binary, decoded for PES 21 at first use).
    pub fn template() -> KitConfig {
        binary::template()
    }

    /// Decodes a kit config; WESYS-wrapped input is unwrapped first and the
    /// result must be exactly 120 bytes. Every version's bytes decode alike:
    /// `name.y` is PES 21's number whatever `version` is.
    pub fn decode(bytes: &[u8], version: PesVersion) -> Result<KitConfig, KitConfigError> {
        binary::decode(bytes, version)
    }

    /// Encodes the config, taking texture names from `source_texture_names`
    /// or zeros when `None`.
    pub fn encode(&self, version: PesVersion) -> [u8; 120] {
        binary::encode(self, version)
    }

    /// Encodes the config with the given five texture-name fields.
    pub fn encode_with_names(&self, version: PesVersion, names: &[[u8; 16]; 5]) -> [u8; 120] {
        binary::encode_with_names(self, version, names)
    }

    /// Builds a config from TOML text; missing sections and keys take the
    /// template's values.
    pub fn from_toml(text: &str) -> Result<KitConfig, KitConfigError> {
        toml_form::from_toml(text)
    }

    /// A fresh TOML document with the predefined per-field comments.
    pub fn to_toml(&self) -> String {
        toml_form::to_toml(self)
    }

    /// Sets every value in an existing document, preserving its comments and
    /// formatting; a present-but-not-table item at a path it writes is an
    /// error and leaves the document untouched.
    pub fn update_toml(&self, document: &mut toml_edit::DocumentMut) -> Result<(), KitConfigError> {
        toml_form::update_toml(self, document)
    }
}

/// Why binary or TOML input is not a kit config.
#[derive(Debug, thiserror::Error)]
pub enum KitConfigError {
    /// The unwrapped binary is not 120 bytes.
    #[error("kit config is {0} bytes, expected 120")]
    WrongLength(usize),
    /// The WESYS wrapper could not be unwrapped.
    #[error("wesys unwrap failed: {0}")]
    Wesys(#[from] wezlib::Error),
    /// The TOML text does not parse.
    #[error("invalid toml: {0}")]
    Toml(#[from] toml::de::Error),
    /// A key carries a value of the wrong type or outside its encoding.
    #[error("invalid value for {key}: {value}")]
    InvalidValue {
        /// The TOML key.
        key: &'static str,
        /// The offending value.
        value: String,
    },
    /// A section `update_toml` writes is present but is not a table.
    #[error("not a table: {key}")]
    NotATable {
        /// The TOML path.
        key: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &[u8] = include_bytes!("../tests/fixtures/template_XXX_DEF_xxx_realUni.bin");
    const GK: &[u8] = include_bytes!("../tests/fixtures/konami_pes21_100_DEF_GK1st_realUni.bin");
    const MODEL144: &[u8] =
        include_bytes!("../tests/fixtures/konami_pes21_model144_100_DEF_1st_realUni.bin");
    const REFEREE: &[u8] = include_bytes!("../tests/fixtures/konami_pes21_referee_EU_1.bin");
    const UNIPARAM: &[u8] =
        include_bytes!("../../uniparam/tests/fixtures/konami_pes21_UniformParameter.bin");
    const BLUE_PES18: &[u8] = include_bytes!("../tests/fixtures/blue_pes18_UniformParameter.bin");
    const BLUE_PES19: &[u8] = include_bytes!("../tests/fixtures/blue_pes19_UniformParameter.bin");

    const FIXTURES: [&[u8]; 4] = [TEMPLATE, GK, MODEL144, REFEREE];

    #[test]
    fn binary_round_trips_bit_identically() {
        for bytes in FIXTURES {
            let config = KitConfig::decode(bytes, PesVersion::Pes21).unwrap();
            assert_eq!(config.encode(PesVersion::Pes21), bytes);
        }
    }

    #[test]
    fn every_stock_config_round_trips_and_stays_in_range() {
        let container = uniparam::UniformParameter::read(UNIPARAM).unwrap();
        let mut checked = 0;
        for (name, content) in container.entries() {
            if content.len() != 120 {
                continue;
            }
            let config = KitConfig::decode(content, PesVersion::Pes21)
                .unwrap_or_else(|_| panic!("decode {name}"));
            assert_eq!(
                config.encode(PesVersion::Pes21),
                content,
                "round trip {name}"
            );
            checked += 1;

            assert!(
                matches!(config.shirt.model, 144 | 160 | 176),
                "shirt model {name}"
            );
            assert!(
                matches!(
                    config.shirt.long_sleeves,
                    LongSleeves::Normal | LongSleeves::UndershirtOnly
                ),
                "long sleeves {name}"
            );

            // Every stock config validates clean except the 13 referee ones
            // carrying short-sleeves value 3, which earn exactly one Info.
            let findings = validate(&config, PesVersion::Pes21);
            assert!(
                findings.is_empty()
                    || findings
                        == [Finding {
                            code: "kit_unknown_sleeve_value",
                            severity: Severity::Info,
                            context: None,
                        }],
                "findings for {name}: {findings:?}"
            );
        }
        assert_eq!(checked, 1372);
    }

    #[test]
    fn every_pes18_and_pes19_stock_config_round_trips() {
        // Blue's PES 18 and 19 stock lists decode and re-encode bit-identically
        // at their own version. Every entry is 120 bytes (none skipped).
        // Unlike PES 21, 17 GK configs carry a shirt model outside 144/160/176
        // (their `kit_shirt_model_unknown` Info findings), so the PES 21 model
        // range is not asserted here; long sleeves holds on every entry.
        for (fixture, version, expected) in [
            (BLUE_PES18, PesVersion::Pes18, 2214),
            (BLUE_PES19, PesVersion::Pes19, 2210),
        ] {
            let container = uniparam::UniformParameter::read(fixture).unwrap();
            let mut checked = 0;
            for (name, content) in container.entries() {
                if content.len() != 120 {
                    continue;
                }
                let config = KitConfig::decode(content, version)
                    .unwrap_or_else(|_| panic!("decode {version:?} {name}"));
                assert_eq!(
                    config.encode(version),
                    content,
                    "round trip {version:?} {name}"
                );
                checked += 1;

                assert!(
                    matches!(
                        config.shirt.long_sleeves,
                        LongSleeves::Normal | LongSleeves::UndershirtOnly
                    ),
                    "long sleeves {version:?} {name}"
                );
                // Findings are limited to the two known Info codes: the
                // referee short-sleeves value and the out-of-set GK model.
                let findings = validate(&config, version);
                assert!(
                    findings.iter().all(|finding| matches!(
                        finding.code,
                        "kit_unknown_sleeve_value" | "kit_shirt_model_unknown"
                    ) && finding.severity == Severity::Info
                        && finding.context.is_none()),
                    "findings for {version:?} {name}: {findings:?}"
                );
            }
            assert_eq!(checked, expected, "{version:?}");
        }
    }

    #[test]
    fn template_decodes_and_encodes_identically() {
        let template = KitConfig::template();
        assert_eq!(template.shirt.model, 176);
        assert_eq!(template.shirt.long_sleeves, LongSleeves::Normal);
        assert_eq!(template.shorts.model, 16);
        assert_eq!(template.shirt.collar, 105);
        assert_eq!(template.shirt.winter_collar, 105);
        assert_eq!(template.encode(PesVersion::Pes21), TEMPLATE);
        // Not PES 15: its pattern remap rewrites the template's pattern 13
        // (0x24 = DF) as 11 (BF).
        for version in [PesVersion::Pes17, PesVersion::Pes21] {
            let decoded = KitConfig::decode(TEMPLATE, version).unwrap();
            assert_eq!(decoded.encode(version), TEMPLATE, "{version:?}");
        }
    }

    /// 0x1C bit 3 is Name Y's lowest bit on every version, so a PES 17 config with it set
    /// decodes it into `name.y`, not into the 0x1C remainder, and still round-trips.
    #[test]
    fn a_pes17_config_with_0x1c_bit_3_set_reads_it_as_name_y() {
        let mut bytes = [0u8; 120];
        bytes.copy_from_slice(TEMPLATE);
        bytes[0x1C] = 0xFA; // 11111 010: Name Y low bits 31, remainder 0x02
        let config = KitConfig::decode(&bytes, PesVersion::Pes17).unwrap();
        assert_eq!(config.name.y, 31);
        assert_eq!(config.unknown.get(&0x1C), Some(&0x02));
        assert_eq!(config.encode(PesVersion::Pes17), bytes);
    }

    /// The referee fixture decoded by hand from the offset table in
    /// `spec/kit_config_editor.md` "The format", every field a literal, so a codec that
    /// permutes two fields of equal width (two colors, two badge coordinates) and still
    /// round-trips bit for bit is caught. Bytes 0x16-0x24 of the fixture:
    /// `00 9C E0 1C 00 07 00 0E 1C 74 D0 41 07 01 00`.
    #[test]
    fn referee_fixture_matches_the_plan_table_by_hand() {
        let mut names = [[0u8; 16]; 5];
        for (field, bytes) in names.iter_mut().zip(REFEREE[0x28..0x78].chunks(16)) {
            field.copy_from_slice(bytes);
        }
        let expected = KitConfig {
            shirt: Shirt {
                model: 176,                          // 0x01 = B0
                collar: 85,                          // 0x14 = 55
                winter_collar: 85,                   // 0x15 = 55
                tight: false,                        // 0x1B bit 7 of 07
                pattern: 0,                          // 0x24 bits 4-7 of 00
                long_sleeves: LongSleeves::Normal,   // 0x02 = 3E
                short_sleeves: ShortSleeves::Raw(3), // 0x00 bits 0-1 of 03
            },
            shorts: Shorts { model: 2 }, // 0x03
            colors: Colors {
                shirt1: Rgb(0xCB, 0xF5, 0x53),     // 0x04-0x06
                shirt2: Rgb(0xCB, 0xF5, 0x53),     // 0x07-0x09
                undershirt: Rgb(0x28, 0x28, 0x28), // 0x0A-0x0C
                shorts: Rgb(0x28, 0x28, 0x28),     // 0x0D-0x0F
                socks: Rgb(0xCB, 0xFE, 0x53),      // 0x10-0x12: G differs from shirt1's
            },
            name: NameText {
                show: true,                 // 0x1E bit 0 of 1C
                shape: NameShape::Straight, // 0x1D bits 6-7 of 0E
                y: 0,                       // 0x1D bit 0 << 5 | 0x1C bits 3-7, both 0
                size: 7,                    // 0x1D bits 1-5 of 0E = 00111
            },
            numbers: Numbers {
                back: BackNumber {
                    y: 0,       // 0x18 bits 0-4 of E0
                    size: 3,    // 0x19 bits 0-1 (00) << 2 | 0x18 bits 6-7 (11)
                    spacing: 1, // 0x19 bits 4-5 of 1C = 01
                },
                chest: ChestNumber {
                    x: 0,    // 0x1A bits 4-7 of 00
                    y: 0,    // 0x1A bits 0-3
                    size: 7, // 0x1B bits 0-3 of 07
                },
                shorts: ShortsNumber {
                    side: Side::Right, // 0x17 bit 7 of 9C
                    x: 0,              // 0x17 bits 0-1 << 2 | 0x16 bits 6-7, both 0
                    y: 0,              // 0x16 bits 1-4 of 00
                    size: 3,           // 0x17 bits 3-6 of 9C = 0011
                },
            },
            badges: Badges {
                left_short: Position { x: 7, y: 16 }, // 0x1E bits 2-5 of 1C; 0x1F bits 0-2 (100) << 2 | 0x1E bits 6-7 (00)
                right_short: Position { x: 7, y: 16 }, // 0x1F bits 4-7 of 74; 0x20 bits 0-4 of D0
                left_long: Position { x: 7, y: 16 }, // 0x21 bits 0-1 (01) << 2 | 0x20 bits 6-7 (11); 0x21 bits 2-6 of 41
                right_long: Position { x: 7, y: 16 }, // 0x22 bits 0-3 of 07; 0x23 bit 0 (1) << 4 | 0x22 bits 4-7 (0)
            },
            // The "?" bits that are set: 0x17 bit 2, 0x18 bit 5, 0x19 bits 2-3, byte 0x25.
            unknown: std::collections::BTreeMap::from([
                (0x17, 0x04),
                (0x18, 0x20),
                (0x19, 0x0C),
                (0x25, 0x03),
            ]),
            source_texture_names: Some(names),
        };
        assert_eq!(
            KitConfig::decode(REFEREE, PesVersion::Pes21).unwrap(),
            expected
        );
    }

    /// No fixture carries five distinct colors, so a codec swapping two color roles in
    /// both directions would still round-trip. The template with the five triplets of
    /// the offset table written at 0x04-0x12 pins each role to its offset.
    #[test]
    fn color_roles_sit_at_the_plan_offsets() {
        let mut bytes = [0u8; 120];
        bytes.copy_from_slice(TEMPLATE);
        bytes[0x04..0x13].copy_from_slice(&[
            1, 2, 3, // shirt1
            4, 5, 6, // shirt2
            7, 8, 9, // undershirt
            10, 11, 12, // shorts
            13, 14, 15, // socks
        ]);
        let config = KitConfig::decode(&bytes, PesVersion::Pes21).unwrap();
        assert_eq!(
            config.colors,
            Colors {
                shirt1: Rgb(1, 2, 3),
                shirt2: Rgb(4, 5, 6),
                undershirt: Rgb(7, 8, 9),
                shorts: Rgb(10, 11, 12),
                socks: Rgb(13, 14, 15),
            }
        );
        assert_eq!(config.encode(PesVersion::Pes21), bytes);
    }

    /// The comments the spec's `config.toml` block shows: each whole-comment
    /// line and each `#`-tail of a key line. A `#` inside a quoted value is
    /// no comment, and a comment opening with `...`/`…` marks elided rows,
    /// which no writer writes.
    fn spec_block_comments() -> Vec<String> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/spec/kit_config_editor.md");
        let doc =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let marker_at = doc.find("winter_collar = 105").expect("the block's marker");
        let fence = doc[..marker_at]
            .rfind("```toml")
            .expect("a toml fence above the marker");
        let body = fence + "```toml".len();
        let end = doc[marker_at..]
            .find("```")
            .map(|i| marker_at + i)
            .expect("the fence closes");
        let mut comments = Vec::new();
        for line in doc[body..end].lines() {
            let line = line.trim_end();
            let comment = if line.starts_with('#') {
                line
            } else if let Some(at) = line.find(" # ") {
                &line[at + 1..]
            } else {
                continue;
            };
            let elided = comment[1..].trim_start().starts_with("...")
                || comment[1..].trim_start().starts_with('…');
            if !elided {
                comments.push(comment.to_string());
            }
        }
        comments
    }

    /// The block's comments are the writer's: every comment line and
    /// `#`-tail the spec block shows appears in a generated file. One
    /// remainder off the template makes `[unknown]` emit.
    #[test]
    fn every_comment_of_the_spec_block_is_emitted() {
        let mut config = KitConfig::template();
        config.unknown.insert(0x13, TEMPLATE[0x13].wrapping_add(1));
        let text = config.to_toml();
        assert!(text.contains("[unknown]"), "{text}");
        for comment in spec_block_comments() {
            assert!(
                text.contains(&comment),
                "a comment the writer does not emit: {comment}"
            );
        }
    }

    /// A two-digit value pushes no comment right: every `#` sits at the
    /// same column, sized for the widest `key = value` the file writes
    /// (`long_sleeves = "undershirt-only"`).
    #[test]
    fn a_two_digit_value_moves_no_comment() {
        let mut config = KitConfig::template();
        config.name.y = 31;
        let text = config.to_toml();
        let columns: Vec<usize> = text
            .lines()
            .filter(|line| line.contains(" # "))
            .map(|line| line.find(" # ").expect("filtered"))
            .collect();
        assert!(!columns.is_empty());
        // ` # ` at byte 32 is `#` at column 34.
        assert!(columns.iter().all(|&c| c == 32), "{columns:?}");
    }

    #[test]
    fn toml_round_trip_carries_everything() {
        assert!(!KitConfig::template().to_toml().contains("[unknown]"));
        // No `[source_texture_names]` table in the text, no names carried.
        let mut without_names = KitConfig::template();
        without_names.source_texture_names = None;
        assert_eq!(KitConfig::from_toml("").unwrap(), without_names);
        for bytes in FIXTURES {
            let config = KitConfig::decode(bytes, PesVersion::Pes21).unwrap();
            let parsed = KitConfig::from_toml(&config.to_toml()).unwrap();
            assert_eq!(parsed, config);
        }
        // A minimal document fills the rest from the template.
        let minimal = KitConfig::from_toml("[shirt]\nmodel = 144\n").unwrap();
        let mut expected = without_names;
        expected.shirt.model = 144;
        assert_eq!(minimal, expected);
    }

    #[test]
    fn toml_without_source_texture_names_carries_none() {
        // The table is the only carrier of texture names in TOML: without it
        // the config is `None`, emits no section and encodes zero names.
        let config = KitConfig::from_toml("").unwrap();
        assert_eq!(config.source_texture_names, None);
        assert!(
            !config.to_toml().contains("[source_texture_names]"),
            "{}",
            config.to_toml()
        );
        assert_eq!(&config.encode(PesVersion::Pes21)[0x28..0x78], &[0u8; 80]);
    }

    #[test]
    fn update_toml_preserves_comments() {
        let text = "[shirt]\nmodel = 144\n# my collar note\ncollar = 4 # inline\n";
        let mut document = text.parse::<toml_edit::DocumentMut>().unwrap();
        let mut config = KitConfig::template();
        config.shirt.model = 160;
        config.shirt.collar = 9;
        config.update_toml(&mut document).unwrap();
        let out = document.to_string();
        assert!(out.contains("# my collar note"), "{out}");
        assert!(out.contains("collar = 9"), "{out}");
        assert!(out.contains("model = 160"), "{out}");
        // Everything set round-trips back to the same config.
        assert_eq!(KitConfig::from_toml(&out).unwrap(), config);
    }

    #[test]
    fn texture_names_derive_from_team_slot_and_presence() {
        let names = texture_names(
            701,
            KitSlot::P1,
            TexturePresence {
                kit: true,
                back: true,
                ..TexturePresence::default()
            },
        );
        assert_eq!(&names[0][..7], b"u0701p1");
        assert_eq!(&names[1][..12], b"u0701p1_back");
        assert_eq!(names[2], [0u8; 16]);
        assert_eq!(names[3], [0u8; 16]);
        assert_eq!(names[4], [0u8; 16]);
    }

    #[test]
    fn kit_slots_parse_name_and_config_names() {
        let slots = [
            (KitSlot::P1, "p1", "792_DEF_1st_realUni.bin"),
            (KitSlot::P2, "p2", "792_DEF_2nd_realUni.bin"),
            (KitSlot::P3, "p3", "792_DEF_3rd_realUni.bin"),
            (KitSlot::P4, "p4", "792_DEF_4th_realUni.bin"),
            (KitSlot::P5, "p5", "792_DEF_5th_realUni.bin"),
            (KitSlot::P6, "p6", "792_DEF_6th_realUni.bin"),
            (KitSlot::P7, "p7", "792_DEF_7th_realUni.bin"),
            (KitSlot::P8, "p8", "792_DEF_8th_realUni.bin"),
            (KitSlot::P9, "p9", "792_DEF_9th_realUni.bin"),
            (KitSlot::G1, "g1", "792_DEF_GK1st_realUni.bin"),
        ];
        for (slot, name, config_name) in slots {
            assert_eq!(KitSlot::parse(name), Some(slot));
            assert_eq!(KitSlot::parse(&name.to_ascii_uppercase()), Some(slot));
            assert_eq!(slot.as_str(), name);
            assert_eq!(slot.config_name(792), config_name);
        }
        for name in ["p0", "p10", "g2", "x1", "all", "", "p1 "] {
            assert_eq!(KitSlot::parse(name), None);
        }
    }

    #[test]
    fn kit_slots_read_config_suffixes() {
        assert_eq!(KitSlot::from_config_suffix("1st"), Some(KitSlot::P1));
        assert_eq!(KitSlot::from_config_suffix("9TH"), Some(KitSlot::P9));
        assert_eq!(KitSlot::from_config_suffix("gk1st"), Some(KitSlot::G1));
        for suffix in ["GK2nd", "10th", "0th", ""] {
            assert_eq!(KitSlot::from_config_suffix(suffix), None, "{suffix}");
        }
    }

    #[test]
    fn validation_rules() {
        let mut config = KitConfig::template();
        config.shirt.short_sleeves = ShortSleeves::CutOut; // model 176
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_cut_out_requires_model_144_or_160"
                    && f.severity == Severity::Warning)
        );
        // Model 160 carries cut-out in stock data: no finding.
        config.shirt.model = 160;
        assert!(validate(&config, PesVersion::Pes21).is_empty());

        config = KitConfig::template();
        config.name.y = 30;
        // `name.y` is PES 21's number on every version; PES <= 20 holds up
        // to 33 of it. 30 gives no finding there and comes back as is.
        assert!(validate(&config, PesVersion::Pes20).is_empty());
        let decoded =
            KitConfig::decode(&config.encode(PesVersion::Pes20), PesVersion::Pes20).unwrap();
        assert_eq!(decoded.name.y, 30);
        // Over 33: one finding naming the field, and the emitted bytes decode
        // to the clamped 33.
        config.name.y = 36;
        assert_eq!(
            validate(&config, PesVersion::Pes20),
            [Finding {
                code: "kit_value_out_of_range",
                severity: Severity::Warning,
                context: Some(OutOfRange {
                    field: "name.y",
                    value: 36,
                    max: 33,
                }),
            }]
        );
        let decoded =
            KitConfig::decode(&config.encode(PesVersion::Pes20), PesVersion::Pes20).unwrap();
        assert_eq!(decoded.name.y, 33);
        // PES 21's field holds 0-39: no finding, 30 and 39 emitted as is.
        config.name.y = 30;
        assert!(validate(&config, PesVersion::Pes21).is_empty());
        let decoded =
            KitConfig::decode(&config.encode(PesVersion::Pes21), PesVersion::Pes21).unwrap();
        assert_eq!(decoded.name.y, 30);
        config.name.y = 39;
        assert!(validate(&config, PesVersion::Pes21).is_empty());
        config.name.y = 40;
        assert_eq!(
            validate(&config, PesVersion::Pes21),
            [Finding {
                code: "kit_value_out_of_range",
                severity: Severity::Warning,
                context: Some(OutOfRange {
                    field: "name.y",
                    value: 40,
                    max: 39,
                }),
            }]
        );

        // Every packed field goes through the same table.
        let mut config = KitConfig::template();
        config.numbers.back.size = 255;
        assert!(validate(&config, PesVersion::Pes21).iter().any(|f| f.code
            == "kit_value_out_of_range"
            && f.context
                == Some(OutOfRange {
                    field: "number.back.size",
                    value: 255,
                    max: 15,
                })));
        let decoded =
            KitConfig::decode(&config.encode(PesVersion::Pes21), PesVersion::Pes21).unwrap();
        assert_eq!(decoded.numbers.back.size, 15);
    }

    #[test]
    fn a_shirt_model_outside_the_known_set_is_an_info_finding() {
        // 144, 160 and 176 are the only models stock data carries; another
        // value is worth a mention but breaks nothing.
        let mut config = KitConfig::template();
        config.shirt.model = 150;
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_shirt_model_unknown" && f.severity == Severity::Info)
        );
    }

    #[test]
    fn pes15_pattern_remaps_and_warns() {
        let mut config = KitConfig::template();
        // The template preserves the byte's undecoded low nibble; drop it so
        // the emitted byte shows the pattern field alone.
        config.unknown.remove(&0x24);
        let pattern_byte = |config: &KitConfig, version: PesVersion| config.encode(version)[0x24];

        config.shirt.pattern = 6;
        assert_eq!(pattern_byte(&config, PesVersion::Pes15), 0x60);
        assert!(validate(&config, PesVersion::Pes15).is_empty());

        config.shirt.pattern = 12;
        assert_eq!(pattern_byte(&config, PesVersion::Pes15), 0xA0);
        assert!(validate(&config, PesVersion::Pes15).iter().any(|f| f.code
            == "kit_pattern_unsupported_pes15"
            && f.severity == Severity::Warning
            && f.context
                == Some(OutOfRange {
                    field: "shirt.pattern",
                    value: 12,
                    max: 11,
                })));

        config.shirt.pattern = 13;
        assert_eq!(pattern_byte(&config, PesVersion::Pes15), 0xB0);

        config.shirt.pattern = 14;
        assert_eq!(pattern_byte(&config, PesVersion::Pes15), 0xE0);
        assert!(
            validate(&config, PesVersion::Pes15)
                .iter()
                .any(|f| f.code == "kit_pattern_unsupported_pes15")
        );

        // Later versions read the full 4-bit field: no remap, no finding.
        config.shirt.pattern = 12;
        assert_eq!(pattern_byte(&config, PesVersion::Pes16), 0xC0);
        assert!(
            !validate(&config, PesVersion::Pes16)
                .iter()
                .any(|f| f.code == "kit_pattern_unsupported_pes15")
        );
    }

    #[test]
    fn wrong_typed_table_is_an_error() {
        assert!(matches!(
            KitConfig::from_toml("shirt = 144"),
            Err(KitConfigError::InvalidValue { key: "shirt", .. })
        ));
        assert!(matches!(
            KitConfig::from_toml("[number]\nback = 12"),
            Err(KitConfigError::InvalidValue {
                key: "number.back",
                ..
            })
        ));
    }

    #[test]
    fn a_bad_badge_value_names_its_badge_and_coordinate() {
        let error = KitConfig::from_toml("[badge.right_long]\nx = 300\ny = 10").unwrap_err();
        assert!(
            matches!(
                &error,
                KitConfigError::InvalidValue { key: "badge.right_long.x", value } if value == "300"
            ),
            "{error:?}"
        );
        let error = KitConfig::from_toml("[badge]\nleft_short = 5").unwrap_err();
        assert!(
            matches!(
                &error,
                KitConfigError::InvalidValue { key: "badge.left_short", value } if value == "5"
            ),
            "{error:?}"
        );
    }

    #[test]
    fn non_ascii_hex_is_an_error_not_a_panic() {
        assert!(matches!(
            KitConfig::from_toml("[colors]\nshirt1 = \"#１２\""),
            Err(KitConfigError::InvalidValue { .. })
        ));
        assert!(matches!(
            KitConfig::from_toml("[source_texture_names]\nkit = \"１２３４５６７８１２01\""),
            Err(KitConfigError::InvalidValue { .. })
        ));
        let config = KitConfig::from_toml("[colors]\nshirt1 = \"#0a0B0c\"").unwrap();
        assert_eq!(config.colors.shirt1, Rgb(10, 11, 12));
    }

    #[test]
    fn fpc_apply_and_match() {
        let mut config = KitConfig::decode(GK, PesVersion::Pes21).unwrap();
        assert!(!matches_fpc(&config));
        apply_fpc(&mut config);
        assert!(matches_fpc(&config));
        assert_eq!(
            (
                config.shirt.model,
                config.shorts.model,
                config.shirt.collar,
                config.shirt.winter_collar
            ),
            (176, 16, 105, 105)
        );
    }

    #[test]
    fn matches_fpc_requires_every_field() {
        let mut config = KitConfig::template();
        apply_fpc(&mut config);
        assert!(matches_fpc(&config));
        // Three of four FPC values matching is not a match.
        for perturb in [
            |c: &mut KitConfig| c.shirt.model ^= 0xFF,
            |c: &mut KitConfig| c.shorts.model ^= 0xFF,
            |c: &mut KitConfig| c.shirt.collar ^= 0xFF,
            |c: &mut KitConfig| c.shirt.winter_collar ^= 0xFF,
        ] {
            let mut near = config.clone();
            perturb(&mut near);
            assert!(!matches_fpc(&near));
        }
    }

    #[test]
    fn name_y_size_and_shape_pack_into_1c_1d_on_every_version() {
        let mut config = KitConfig::template();
        config.name.size = 10;
        config.name.shape = NameShape::MediumCurve;
        // Assert the decoded bits alone: drop the template's preserved
        // remainder at 0x1C (0x1D is fully decoded, none preserved).
        config.unknown.remove(&0x1C);

        // 0x1C[3-7] = y & 0x1F, 0x1D[0] = y >> 5, 0x1D[1-5] = size,
        // 0x1D[6-7] = shape, the same bytes on every version.
        for version in [
            PesVersion::Pes15,
            PesVersion::Pes17,
            PesVersion::Pes20,
            PesVersion::Pes21,
        ] {
            config.name.y = 5;
            let bytes = config.encode(version);
            assert_eq!(bytes[0x1C], 0x28, "{version:?}"); // 5 << 3
            assert_eq!(bytes[0x1D], 0x94, "{version:?}"); // 0 | (10 << 1) | (2 << 6)
            // y's sixth bit carries into 0x1D bit 0.
            config.name.y = 33;
            let bytes = config.encode(version);
            assert_eq!(bytes[0x1C], 0x08, "{version:?}"); // (33 & 0x1F) << 3
            assert_eq!(bytes[0x1D], 0x95, "{version:?}"); // 1 | (10 << 1) | (2 << 6)
        }
    }

    #[test]
    fn decode_reads_every_name_shape() {
        for (bits, expected) in [
            (0u8, NameShape::Straight),
            (1, NameShape::LightCurve),
            (2, NameShape::MediumCurve),
            (3, NameShape::ExtremeCurve),
        ] {
            let mut bytes = TEMPLATE.to_vec();
            bytes[0x1D] = (bytes[0x1D] & 0x3F) | (bits << 6);
            let config = KitConfig::decode(&bytes, PesVersion::Pes21).unwrap();
            assert_eq!(config.name.shape, expected, "shape bits {bits}");
        }
    }

    #[test]
    fn toml_parses_name_shapes_and_sleeve_forms() {
        for (text, expected) in [
            ("straight", NameShape::Straight),
            ("light-curve", NameShape::LightCurve),
            ("medium-curve", NameShape::MediumCurve),
            ("extreme-curve", NameShape::ExtremeCurve),
        ] {
            let config = KitConfig::from_toml(&format!("[name]\nshape = \"{text}\"")).unwrap();
            assert_eq!(config.name.shape, expected, "{text}");
        }
        let config = KitConfig::from_toml("[shirt]\nshort_sleeves = \"cut-out\"").unwrap();
        assert_eq!(config.shirt.short_sleeves, ShortSleeves::CutOut);
        // Integer sleeve values carry the raw byte.
        let config = KitConfig::from_toml("[shirt]\nlong_sleeves = 187").unwrap();
        assert_eq!(config.shirt.long_sleeves, LongSleeves::UndershirtOnly);
        let config = KitConfig::from_toml("[shirt]\nlong_sleeves = 7").unwrap();
        assert_eq!(config.shirt.long_sleeves, LongSleeves::Raw(7));
        // Short-sleeves integers keep the raw value.
        let config = KitConfig::from_toml("[shirt]\nshort_sleeves = 5").unwrap();
        assert_eq!(config.shirt.short_sleeves, ShortSleeves::Raw(5));
    }

    #[test]
    fn raw_short_sleeves_warn_and_encode_clamped() {
        // A raw value above the field max earns `kit_value_out_of_range` and
        // clamps on emission like every other packed field.
        let mut config = KitConfig::template();
        config.unknown.remove(&0x00);
        config.shirt.short_sleeves = ShortSleeves::Raw(5);
        assert!(validate(&config, PesVersion::Pes21).iter().any(|f| f.code
            == "kit_value_out_of_range"
            && f.context
                == Some(OutOfRange {
                    field: "shirt.short_sleeves",
                    value: 5,
                    max: 3,
                })));
        assert_eq!(config.encode(PesVersion::Pes21)[0x00], 3);
        // Raw(3) stays an `kit_unknown_sleeve_value` Info, not out of range.
        config.shirt.short_sleeves = ShortSleeves::Raw(3);
        assert!(
            !validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_value_out_of_range")
        );
    }

    #[test]
    fn validate_flags_each_side_of_or_conditions() {
        // A zero collar or winter collar alone earns the finding.
        let mut config = KitConfig::template();
        config.shirt.collar = 0;
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_collar_zero")
        );
        let mut config = KitConfig::template();
        config.shirt.winter_collar = 0;
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_collar_zero")
        );
        assert!(
            !validate(&KitConfig::template(), PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_collar_zero")
        );

        // A raw value on either sleeve alone earns the finding.
        let mut config = KitConfig::template();
        config.shirt.short_sleeves = ShortSleeves::Raw(3);
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_unknown_sleeve_value")
        );
        let mut config = KitConfig::template();
        config.shirt.long_sleeves = LongSleeves::Raw(7);
        assert!(
            validate(&config, PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_unknown_sleeve_value")
        );
        assert!(
            !validate(&KitConfig::template(), PesVersion::Pes21)
                .iter()
                .any(|f| f.code == "kit_unknown_sleeve_value")
        );
    }

    #[test]
    fn the_injected_model_notes_name_the_models_validate_accepts() {
        // Each of these shirt options' comments ends in "<models> only", and
        // validate must accept the option on exactly the models it names.
        type SetOption = fn(&mut KitConfig);
        let cases: [(&str, SetOption, &str); 3] = [
            (
                "tight",
                |c| c.shirt.tight = true,
                "kit_tight_requires_model_144_or_160",
            ),
            (
                "long_sleeves",
                |c| c.shirt.long_sleeves = LongSleeves::UndershirtOnly,
                "kit_undershirt_only_requires_model_144_or_160",
            ),
            (
                "short_sleeves",
                |c| c.shirt.short_sleeves = ShortSleeves::CutOut,
                "kit_cut_out_requires_model_144_or_160",
            ),
        ];
        let text = KitConfig::template().to_toml();
        for (key, set, code) in cases {
            let prefix = format!("{key} = ");
            let line = text.lines().find(|line| line.starts_with(&prefix)).unwrap();
            let (_, comment) = line.split_once('#').unwrap();
            let (before_only, _) = comment.split_once(" only").unwrap();
            let named = before_only
                .rsplit(' ')
                .next()
                .unwrap()
                .trim_start_matches('(');
            for model in [144u8, 160, 176] {
                let mut config = KitConfig::template();
                set(&mut config);
                config.shirt.model = model;
                let accepted = !validate(&config, PesVersion::Pes21)
                    .iter()
                    .any(|f| f.code == code);
                let named_here = named.split('/').any(|m| m == model.to_string());
                assert_eq!(
                    named_here, accepted,
                    "{key} on model {model}: the comment names {named:?}"
                );
            }
        }
    }

    #[test]
    fn update_toml_writes_only_differing_unknown_keys() {
        // 0x13 is wholly undecoded, so the template's remainder there is the
        // template byte itself; any other value differs.
        let value = TEMPLATE[0x13].wrapping_add(1);
        let mut config = KitConfig::template();
        config.unknown.insert(0x13, value);
        let mut document = config.to_toml().parse::<toml_edit::DocumentMut>().unwrap();
        config.update_toml(&mut document).unwrap();
        let unknown = document["unknown"]
            .as_table()
            .expect("[unknown] must be a table");
        let entries: Vec<(String, Option<i64>)> = unknown
            .iter()
            .map(|(key, item)| (key.to_owned(), item.as_integer()))
            .collect();
        assert_eq!(entries, [("0x13".to_owned(), Some(i64::from(value)))]);

        // A template-equal config drops the table entirely.
        let mut document = "[unknown]\n\"0x13\" = 3\n"
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        KitConfig::template().update_toml(&mut document).unwrap();
        assert!(document.get("unknown").is_none(), "{document}");
    }

    #[test]
    fn update_toml_drops_a_stale_unknown_key() {
        // A [unknown] key whose offset's remainder returned to the
        // template's is removed; keys still differing are kept.
        let value = TEMPLATE[0x13].wrapping_add(1);
        let mut config = KitConfig::template();
        config.unknown.insert(0x13, value);
        let mut document = "[unknown]\n\"0x13\" = 1\n\"0x25\" = 3\n"
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        config.update_toml(&mut document).unwrap();
        let out = document.to_string();
        assert!(out.contains("\"0x13\""), "{out}");
        assert!(!out.contains("0x25"), "{out}");
        assert_eq!(KitConfig::from_toml(&out).unwrap(), config);
    }

    #[test]
    fn update_toml_removes_source_texture_names_the_config_lacks() {
        // A document's stale [source_texture_names] table is dropped when
        // the config carries no names.
        let mut document = "[source_texture_names]\nkit = \"00112233445566778899aabbccddeeff\"\n"
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        let mut config = KitConfig::template();
        config.source_texture_names = None;
        config.update_toml(&mut document).unwrap();
        let out = document.to_string();
        assert!(!out.contains("source_texture_names"), "{out}");
        assert_eq!(KitConfig::from_toml(&out).unwrap(), config);
    }

    #[test]
    fn update_toml_is_byte_identical_when_nothing_changes() {
        // A commented [unknown] key and a commented standard
        // [badge.left_short] table survive a no-op update verbatim: values
        // are written in place, not replaced.
        let mut config = KitConfig::template();
        let unknown_value = TEMPLATE[0x13].wrapping_add(1);
        config.unknown.insert(0x13, unknown_value);
        let badge = config.badges.left_short;
        let mut text = config.to_toml();
        // Swap the emitted inline left_short for a commented standard table.
        let inline = format!("left_short = {{ x = {}, y = {} }}", badge.x, badge.y);
        assert!(text.contains(&inline), "{text}");
        text = text.replacen(&inline, "", 1);
        text += &format!(
            "\n# a note on the table\n[badge.left_short]  # standard form\nx = {}  # ex\ny = {}\n",
            badge.x, badge.y
        );
        let key = format!("\"0x13\" = {unknown_value}");
        assert!(text.contains(&key), "{text}");
        text = text.replacen(&key, &format!("{key}  # kept"), 1);
        let mut document = text.parse::<toml_edit::DocumentMut>().unwrap();
        let before = document.to_string();
        config.update_toml(&mut document).unwrap();
        assert_eq!(document.to_string(), before);
    }

    #[test]
    fn update_toml_emits_a_missing_badge_entry() {
        // A document without the badge entry gets the inline form `to_toml`
        // writes, with both coordinates of the config's position.
        let mut document = "[badge]\n".parse::<toml_edit::DocumentMut>().unwrap();
        let mut config = KitConfig::template();
        config.badges.right_short = Position { x: 3, y: 9 };
        config.update_toml(&mut document).unwrap();
        let out = document.to_string();
        assert!(out.contains("right_short = { x = 3, y = 9 }"), "{out}");
        assert_eq!(KitConfig::from_toml(&out).unwrap(), config);
    }

    #[test]
    fn update_toml_updates_an_inline_unknown_table_in_place() {
        // `unknown = { ... }` is a table too: keys are updated, added and
        // removed inside it instead of being skipped.
        let mut document = "unknown = { \"0x25\" = 3 }\n"
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        let mut config = KitConfig::template();
        config
            .unknown
            .insert(0x25, TEMPLATE[0x25].wrapping_add(1).max(1));
        config
            .unknown
            .insert(0x26, TEMPLATE[0x26].wrapping_add(1).max(1));
        config.update_toml(&mut document).unwrap();
        let inline = document["unknown"]
            .as_inline_table()
            .expect("the table stays inline");
        assert_eq!(
            inline["0x25"].as_integer(),
            Some(i64::from(config.unknown[&0x25]))
        );
        assert_eq!(
            inline["0x26"].as_integer(),
            Some(i64::from(config.unknown[&0x26]))
        );
        assert_eq!(KitConfig::from_toml(&document.to_string()).unwrap(), config);
    }

    #[test]
    fn update_toml_rejects_a_scalar_where_a_table_is_expected() {
        // A present-but-not-table item at any path update_toml writes is an
        // error, and the document is left untouched.
        for (text, key) in [
            ("shirt = 144\n", "shirt"),
            ("[number]\nback = 12\n", "number.back"),
            ("unknown = 5\n", "unknown"),
        ] {
            let mut document = text.parse::<toml_edit::DocumentMut>().unwrap();
            let before = document.to_string();
            let err = KitConfig::template()
                .update_toml(&mut document)
                .unwrap_err();
            assert!(
                matches!(err, KitConfigError::NotATable { key: k } if k == key),
                "{text:?} -> {err}"
            );
            assert_eq!(document.to_string(), before, "{text:?}");
        }
    }

    #[test]
    fn unknown_keys_are_validated_against_the_codec_layout() {
        // An offset that carries no undecoded bits is rejected.
        let err = KitConfig::from_toml("[unknown]\n\"0x28\" = 1\n").unwrap_err();
        assert!(matches!(
            err,
            KitConfigError::InvalidValue { key: "unknown", .. }
        ));
        // A remainder with bits outside the offset's mask is rejected.
        let err = KitConfig::from_toml("[unknown]\n\"0x16\" = 0x40\n").unwrap_err();
        assert!(matches!(
            err,
            KitConfigError::InvalidValue { key: "unknown", .. }
        ));
        // 0x1C bit 3 is Name Y's on every version, not a remainder bit; the
        // bits below it are.
        let err = KitConfig::from_toml("[unknown]\n\"0x1C\" = 8\n").unwrap_err();
        assert!(matches!(
            err,
            KitConfigError::InvalidValue { key: "unknown", .. }
        ));
        let config = KitConfig::from_toml("[unknown]\n\"0x1C\" = 2\n").unwrap();
        assert_eq!(config.unknown.get(&0x1C), Some(&0x02));
        // Two spellings of one offset are rejected.
        let err = KitConfig::from_toml("[unknown]\n\"0x1C\" = 1\n\"0x1c\" = 2\n").unwrap_err();
        assert!(matches!(
            err,
            KitConfigError::InvalidValue { key: "unknown", .. }
        ));
        // A decoded fixture's own [unknown] section parses back equal.
        let config = KitConfig::decode(GK, PesVersion::Pes21).unwrap();
        assert_eq!(KitConfig::from_toml(&config.to_toml()).unwrap(), config);
    }

    #[test]
    fn encode_with_names_writes_the_name_fields() {
        let config = KitConfig::decode(GK, PesVersion::Pes21).unwrap();
        let names = config.source_texture_names.unwrap();
        assert_eq!(config.encode_with_names(PesVersion::Pes21, &names), GK);

        let mut other = [[0u8; 16]; 5];
        other[0][..8].copy_from_slice(b"u0701gk1");
        let bytes = config.encode_with_names(PesVersion::Pes21, &other);
        assert_eq!(&bytes[0x28..0x38], &other[0][..]);
        assert_eq!(&bytes[0x38..0x78], &[0u8; 64][..]);
        assert_eq!(&bytes[..0x28], &GK[..0x28]);
    }
}
