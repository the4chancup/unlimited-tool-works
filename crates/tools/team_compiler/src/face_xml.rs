//! The `face.xml` a pre-Fox face CPK lists its models in: each model's type, read from its file
//! name (`spec/aesthetics_export/player_folders.md` "Model names"), the name it is packed
//! under, its `ratio`, and the file itself, in the shape the game's own face CPKs carry. Also a
//! member's own `face.xml` written back in that shape (`user_face_xml`), and the `glove.xml`
//! of a shared gloves output, the same file without the face diff.

use aesthetics_export::{ModelSuffix, ends_with_name, model_suffix, without_kit_token};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pes_version::PesVersion;

use crate::user_face_xml::Element;

/// One `<model>` element of a generated `face.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct XmlEntry {
    /// The `type` attribute: what the game loads the model as (`face_neck`, `parts`).
    pub(crate) xml_type: String,
    /// The `path` attribute: the packed model, `win32` written as `*`
    /// (`./oral_face_high_*.model`).
    pub(crate) path: String,
    /// The `material` attribute: the `.mtl` the model uses, as packed (`./face_high.mtl`).
    pub(crate) material: String,
    /// The `ratio` attribute, when the model's name sets one.
    pub(crate) ratio: Option<String>,
}

impl XmlEntry {
    /// The entry's `<model>` attributes (name, value) in the order written: `level` (always
    /// `0`), `type`, `path`, `material`, then `ratio` when set.
    pub(crate) fn attributes(&self) -> Vec<(String, String)> {
        let mut attributes = vec![
            ("level".to_owned(), "0".to_owned()),
            ("type".to_owned(), self.xml_type.clone()),
            ("path".to_owned(), self.path.clone()),
            ("material".to_owned(), self.material.clone()),
        ];
        if let Some(ratio) = &self.ratio {
            attributes.push(("ratio".to_owned(), ratio.clone()));
        }
        attributes
    }
}

/// The `face.xml` type of the model with file stem `stem`, read without the `_win32` suffix a
/// packed name carries: what follows `model_type_`, the escape hatch for a type the table does
/// not know; else the type of its suffix (`face_high` is `face_neck`, the other face names and
/// the boots `parts`, each glove and hand its side's); else a native type name the stem ends
/// with (`ends_with_name`); else `parts`. A packed name's `oral_` prefix needs no stripping: a
/// suffix and a native name are read at the stem's end and `model_type_` anywhere in it, and
/// `oral_` alone is the `oral` suffix, typed `parts` as an empty stem is. The stem is read
/// without its kit token (`without_kit_token`), so a per-kit model is typed as its set is:
/// `cape_model_type_cape_kit2` is a `cape`.
pub(crate) fn xml_type(stem: &str) -> String {
    let stem = without_kit_token(without_platform(stem));
    let stem = stem.as_ref();
    // ASCII lower case keeps every byte offset, so the offset found indexes `stem`. An empty
    // name after the marker types nothing: the table's type is taken instead.
    if let Some(at) = stem.to_ascii_lowercase().find(MODEL_TYPE_MARKER) {
        let named = &stem[at + MODEL_TYPE_MARKER.len()..];
        if !named.is_empty() {
            return named.to_owned();
        }
    }
    let xml_type = match model_suffix(stem) {
        Some(ModelSuffix::FaceHigh) => "face_neck",
        Some(ModelSuffix::HairHigh | ModelSuffix::Oral | ModelSuffix::FclHair) => "parts",
        // Boots use the body skeleton, as the face's parts do.
        Some(ModelSuffix::Boots) => "parts",
        Some(ModelSuffix::GloveL) => "gloveL",
        Some(ModelSuffix::GloveR) => "gloveR",
        Some(ModelSuffix::HandL) => "handL",
        Some(ModelSuffix::HandR) => "handR",
        None => NATIVE_TYPES
            .into_iter()
            .find(|native| ends_with_name(stem, native))
            .unwrap_or("parts"),
    };
    xml_type.to_owned()
}

/// Whether `xml_type` is a `face.xml` type this module generates: a native type name, a
/// glove's or a hand's side, or `uniform_sub`, PES 2015's `uniform` (`version_type`). The
/// vocabulary verified in the game; a member's own `face.xml` naming another type is reported
/// (`xml_type_unknown`).
pub(crate) fn is_generated_type(xml_type: &str) -> bool {
    NATIVE_TYPES.contains(&xml_type)
        || ["gloveL", "gloveR", "handL", "handR", "uniform_sub"].contains(&xml_type)
}

/// The `face.xml` type `xml_type` is written as for `version`: `uniform` is `uniform_sub` on
/// PES 2015, every other type, and every type on the other versions, as it is. The rewrite is
/// reported as `xml_uniform_pes15`.
pub(crate) fn version_type(version: PesVersion, xml_type: &str) -> &str {
    match version {
        PesVersion::Pes15 if xml_type == "uniform" => "uniform_sub",
        PesVersion::Pes14
        | PesVersion::Pes15
        | PesVersion::Pes16
        | PesVersion::Pes17
        | PesVersion::Pes18
        | PesVersion::Pes19
        | PesVersion::Pes20
        | PesVersion::Pes21 => xml_type,
    }
}

/// What precedes a type the `face.xml` table does not know, in a model's name
/// (`cape_model_type_cape`).
const MODEL_TYPE_MARKER: &str = "model_type_";

/// The game's own `face.xml` type names, which a model's name may end with as it ends with a
/// suffix (`body_uniform`).
const NATIVE_TYPES: [&str; 7] = [
    "uniform",
    "shirt",
    "pants_nocloth",
    "eye",
    "mouth",
    "face_neck",
    "parts",
];

/// `stem` without the `_win32` suffix of a packed name, in any case, when it carries it: the
/// suffix would hide the name's own tail (`face_high_win32` is `face_high`).
fn without_platform(stem: &str) -> &str {
    // The suffix is ASCII, so bytes that match it start on a character boundary.
    let Some(at) = stem.len().checked_sub(WIN32_SUFFIX.len()) else {
        return stem;
    };
    if stem.as_bytes()[at..].eq_ignore_ascii_case(WIN32_SUFFIX.as_bytes()) {
        &stem[..at]
    } else {
        stem
    }
}

/// The prefix of a packed model's name.
pub(crate) const ORAL_PREFIX: &str = "oral_";

/// The suffix of a packed model's stem, the platform the game loads it on.
pub(crate) const WIN32_SUFFIX: &str = "_win32";

/// The name the model with file stem `stem` is packed under in its face CPK: the stem in ASCII
/// lower case, with `oral_` before it and `_win32.model` after it, each affix added only when
/// the stem does not already carry it (`hat` and `oral_hat_win32` both give
/// `oral_hat_win32.model`).
pub(crate) fn packed_model_name(stem: &str) -> String {
    let lower = stem.to_ascii_lowercase();
    let prefix = if lower.starts_with(ORAL_PREFIX) {
        ""
    } else {
        ORAL_PREFIX
    };
    let suffix = if lower.ends_with(WIN32_SUFFIX) {
        ""
    } else {
        WIN32_SUFFIX
    };
    format!("{prefix}{lower}{suffix}.model")
}

/// The `path` attribute naming the model packed as `packed_name` in `directory`, `/`-terminated
/// (`./`, the face CPK's own folder, or the team's Common output for a Common model), `win32`
/// written as `*` (`./oral_face_high_*.model`), as the game's own `face.xml` files name their
/// models.
pub(crate) fn xml_path(directory: &str, packed_name: &str) -> String {
    let base = packed_name
        .strip_suffix("_win32.model")
        .expect("a packed model's name ends with `_win32.model` (`packed_model_name`)");
    format!("{directory}{base}_*.model")
}

/// The `ratio` the model with file stem `stem` sets: the token after `ratio_`, at the stem's
/// start or after a `_`, up to the next `_` or the stem's end (`visor_ratio_2_parts` gives
/// `2`); `None` when the stem holds no such token, or an empty one.
pub(crate) fn ratio(stem: &str) -> Option<&str> {
    const MARKER: &str = "ratio_";
    let (at, _) = stem
        .match_indices(MARKER)
        .find(|(at, _)| *at == 0 || stem.as_bytes()[at - 1] == b'_')?;
    let rest = &stem[at + MARKER.len()..];
    let token = rest.split_once('_').map_or(rest, |(token, _)| token);
    (!token.is_empty()).then_some(token)
}

/// The suffix (`model_suffix`) of the model with file stem `stem`, read without the packed
/// name's `_win32` suffix. Its `oral_` prefix needs no stripping, as in `xml_type`: a suffix is
/// read at the stem's end, so the prefix can only make `oral_` alone the `oral` suffix.
pub(crate) fn suffix(stem: &str) -> Option<ModelSuffix> {
    model_suffix(without_platform(stem))
}

/// The `face.xml` listing `entries` in their order, with `dif`, the face's `face_diff.bin`, as
/// its `<dif>`: the XML declaration in single quotes, the `<config>` root, one `<model>` per
/// entry indented by three spaces, `ratio` last when set, the `<dif>` base64 (standard
/// alphabet, padded) on one line of its own, CRLF line ends and no final one, the shape the
/// game's own face CPKs carry.
pub(crate) fn face_xml(entries: &[XmlEntry], dif: &[u8]) -> Vec<u8> {
    config_xml(entries, Some(dif))
}

/// The `glove.xml` of a pre-Fox shared gloves output listing `entries` in their order: the
/// `face.xml` shape (`face_xml`) with no `<dif>`, `</config>` right after the last entry, the
/// shape the game's own glove folders carry.
pub(crate) fn glove_xml(entries: &[XmlEntry]) -> Vec<u8> {
    config_xml(entries, None)
}

/// One child of the `<config>` of a member's own `face.xml` as the compiler writes it back
/// (`user_face_xml`).
pub(crate) enum WrittenChild<'a> {
    /// A `<model>`: its attributes (name, value), in the order written.
    Model(Vec<(String, String)>),
    /// Any other element, written as the member wrote it, its attributes, text and children
    /// included (`xml_element_unknown`).
    Other(&'a Element),
}

/// A member's own `face.xml` written back: `children` in their order, then `dif`, the face's
/// face diff, as its `<dif>`, in the shape of a generated one (`face_xml`): the same
/// declaration, `<config>` root, three-space indent, CRLF line ends and `<dif>` line. Each
/// child is one line: a `<model>` with its attributes in the order given, any other element
/// with its attributes, its text and its child elements inline, closed by ` />` when it has
/// neither text nor children. Values and text are escaped.
pub(crate) fn user_face_xml(children: &[WrittenChild], dif: &[u8]) -> Vec<u8> {
    let mut text = String::from(HEAD);
    for child in children {
        text.push_str(INDENT);
        match child {
            WrittenChild::Model(attributes) => {
                push_element(&mut text, "model", attributes, "", &[])
            }
            WrittenChild::Other(element) => push_element(
                &mut text,
                &element.name,
                &element.attributes,
                &element.text,
                &element.children,
            ),
        }
        text.push_str("\r\n");
    }
    push_tail(&mut text, Some(dif));
    text.into_bytes()
}

/// The first two lines of a `face.xml` or `glove.xml`: the XML declaration in single quotes
/// and the `<config>` root's start tag.
const HEAD: &str = "<?xml version='1.0' encoding='UTF-8'?>\r\n<config>\r\n";

/// The indent of each child of `<config>`.
const INDENT: &str = "   ";

/// The `<config>` document listing `entries`, with `dif` as its `<dif>` when given
/// (`face_xml`, `glove_xml`).
fn config_xml(entries: &[XmlEntry], dif: Option<&[u8]>) -> Vec<u8> {
    let mut text = String::from(HEAD);
    for entry in entries {
        text.push_str(INDENT);
        push_element(&mut text, "model", &entry.attributes(), "", &[]);
        text.push_str("\r\n");
    }
    push_tail(&mut text, dif);
    text.into_bytes()
}

/// Appends to `text` the element `name` with `attributes` (name, value) in their order, then
/// `content` and `children`, all on one line; ` />` closes it when it has neither.
fn push_element(
    text: &mut String,
    name: &str,
    attributes: &[(String, String)],
    content: &str,
    children: &[Element],
) {
    text.push('<');
    text.push_str(name);
    for (attribute, value) in attributes {
        text.push_str(&format!(" {attribute}=\"{}\"", escaped(value)));
    }
    if content.is_empty() && children.is_empty() {
        text.push_str(" />");
        return;
    }
    text.push('>');
    text.push_str(&escaped(content));
    for child in children {
        push_element(
            text,
            &child.name,
            &child.attributes,
            &child.text,
            &child.children,
        );
    }
    text.push_str(&format!("</{name}>"));
}

/// Appends to `text` the end of a `<config>` document: `dif`, when given, as its `<dif>`, the
/// base64 (standard alphabet, padded) on one line of its own, then `</config>` with no final
/// line end.
fn push_tail(text: &mut String, dif: Option<&[u8]>) {
    if let Some(dif) = dif {
        text.push_str("<dif>\r\n");
        text.push_str(&STANDARD.encode(dif));
        text.push_str("\r\n</dif>\r\n");
    }
    text.push_str("</config>");
}

/// `value` as an XML attribute value between double quotes, or as an element's text: `&`, `<`,
/// `>` and `"` escaped.
fn escaped(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_s_type_is_its_escape_hatch_then_its_suffix_s_then_a_native_name_then_parts() {
        for (stem, expected) in [
            ("face_high", "face_neck"),
            ("kit_boots", "parts"),
            ("hat_parts", "parts"),
            ("arm_gloveL", "gloveL"),
            ("arm_glove_r", "gloveR"),
            ("x_handR", "handR"),
            ("cape_model_type_cape", "cape"),
            // The escape-hatch marker matches letter case aside, as a suffix does.
            ("cape_MODEL_TYPE_cape", "cape"),
            ("body_uniform", "uniform"),
            ("x_pants_nocloth", "pants_nocloth"),
            ("torso", "parts"),
            ("oral_face_high_win32", "face_neck"),
            ("visor_ratio_2_parts", "parts"),
        ] {
            assert_eq!(xml_type(stem), expected, "{stem}");
        }
    }

    #[test]
    fn a_per_kit_model_is_typed_as_its_set_is() {
        for (stem, expected) in [
            ("body_uniform_kit1", "uniform"),
            ("cape_model_type_cape_kit2", "cape"),
            ("pants_kit1", "parts"),
        ] {
            assert_eq!(xml_type(stem), expected, "{stem}");
        }
    }

    #[test]
    fn a_uniform_is_written_uniform_sub_on_pes_15_only() {
        for (version, xml_type, expected) in [
            (PesVersion::Pes15, "uniform", "uniform_sub"),
            (PesVersion::Pes16, "uniform", "uniform"),
            (PesVersion::Pes17, "uniform", "uniform"),
            (PesVersion::Pes15, "parts", "parts"),
        ] {
            assert_eq!(
                version_type(version, xml_type),
                expected,
                "{version:?} {xml_type}"
            );
        }
    }

    #[test]
    fn a_model_packs_lowercased_with_each_affix_added_once() {
        assert_eq!(packed_model_name("Face_High"), "oral_face_high_win32.model");
        assert_eq!(packed_model_name("oral_hat_win32"), "oral_hat_win32.model");
        assert_eq!(packed_model_name("oral_hat"), "oral_hat_win32.model");
    }

    #[test]
    fn a_packed_model_s_xml_path_writes_its_platform_as_a_star() {
        assert_eq!(
            xml_path("./", "oral_face_high_win32.model"),
            "./oral_face_high_*.model"
        );
        assert_eq!(
            xml_path("./", "oral_dummy_win32.model"),
            "./oral_dummy_*.model"
        );
        assert_eq!(
            xml_path(
                "model/character/uniform/common/714/",
                "oral_legs_win32.model"
            ),
            "model/character/uniform/common/714/oral_legs_*.model"
        );
    }

    #[test]
    fn the_ratio_is_the_token_after_ratio_up_to_the_next_underscore() {
        assert_eq!(ratio("visor_ratio_2_parts"), Some("2"));
        assert_eq!(ratio("ratio_1.5"), Some("1.5"));
        assert_eq!(ratio("xratio_2"), None);
        assert_eq!(ratio("visor_ratio_"), None);
    }

    #[test]
    fn the_face_xml_lists_its_entries_then_the_dif_as_one_line_of_base64() {
        let entries = [
            XmlEntry {
                xml_type: "parts".to_owned(),
                path: "./oral_visor_ratio_2_parts_*.model".to_owned(),
                material: "./materials.mtl".to_owned(),
                ratio: Some("2".to_owned()),
            },
            XmlEntry {
                xml_type: "face_neck".to_owned(),
                path: "./oral_face_high_*.model".to_owned(),
                material: "./a&b.mtl".to_owned(),
                ratio: None,
            },
        ];

        let xml = face_xml(&entries, b"FAC");

        assert_eq!(
            String::from_utf8(xml).unwrap(),
            "<?xml version='1.0' encoding='UTF-8'?>\r\n\
             <config>\r\n   \
             <model level=\"0\" type=\"parts\" path=\"./oral_visor_ratio_2_parts_*.model\" material=\"./materials.mtl\" ratio=\"2\" />\r\n   \
             <model level=\"0\" type=\"face_neck\" path=\"./oral_face_high_*.model\" material=\"./a&amp;b.mtl\" />\r\n\
             <dif>\r\n\
             RkFD\r\n\
             </dif>\r\n\
             </config>"
        );
    }

    #[test]
    fn the_glove_xml_lists_its_entries_with_no_dif_and_no_final_line_end() {
        let entry = |xml_type: &str, model: &str| XmlEntry {
            xml_type: xml_type.to_owned(),
            path: format!("./{model}"),
            material: "./materials.mtl".to_owned(),
            ratio: None,
        };
        let entries = [
            entry("gloveL", "glove_l.model"),
            entry("gloveR", "glove_r.model"),
        ];

        let xml = glove_xml(&entries);

        // The installed PES 2015 DLC's `glove/g0880/glove.xml`, byte for byte.
        assert_eq!(
            String::from_utf8(xml).unwrap(),
            "<?xml version='1.0' encoding='UTF-8'?>\r\n\
             <config>\r\n   \
             <model level=\"0\" type=\"gloveL\" path=\"./glove_l.model\" material=\"./materials.mtl\" />\r\n   \
             <model level=\"0\" type=\"gloveR\" path=\"./glove_r.model\" material=\"./materials.mtl\" />\r\n\
             </config>"
        );
    }

    /// The attributes `pairs` as `WrittenChild::Model` takes them.
    fn attributes(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn a_member_s_xml_keeps_its_attribute_order_and_writes_an_unknown_element_inline() {
        let extra = Element {
            name: "extra".to_owned(),
            attributes: attributes(&[("a", "1&2")]),
            text: "x<y".to_owned(),
            children: vec![Element {
                name: "inner".to_owned(),
                attributes: attributes(&[("b", "2")]),
                text: String::new(),
                children: Vec::new(),
            }],
        };
        let empty = Element {
            name: "empty".to_owned(),
            attributes: Vec::new(),
            text: String::new(),
            children: Vec::new(),
        };
        // Text alone, and a child alone, each keep the element open.
        let text_only = Element {
            name: "note".to_owned(),
            attributes: Vec::new(),
            text: "t".to_owned(),
            children: Vec::new(),
        };
        let child_only = Element {
            name: "outer".to_owned(),
            attributes: Vec::new(),
            text: String::new(),
            children: vec![Element {
                name: "empty".to_owned(),
                attributes: Vec::new(),
                text: String::new(),
                children: Vec::new(),
            }],
        };
        let children = [
            WrittenChild::Model(attributes(&[
                ("type", "cape"),
                ("glow", "1"),
                ("path", "./a&b.model"),
                ("material", "./hat.mtl"),
                ("level", "1"),
            ])),
            WrittenChild::Other(&extra),
            WrittenChild::Other(&empty),
            WrittenChild::Other(&text_only),
            WrittenChild::Other(&child_only),
        ];

        let xml = user_face_xml(&children, b"FAC");

        assert_eq!(
            String::from_utf8(xml).unwrap(),
            "<?xml version='1.0' encoding='UTF-8'?>\r\n\
             <config>\r\n   \
             <model type=\"cape\" glow=\"1\" path=\"./a&amp;b.model\" material=\"./hat.mtl\" level=\"1\" />\r\n   \
             <extra a=\"1&amp;2\">x&lt;y<inner b=\"2\" /></extra>\r\n   \
             <empty />\r\n   \
             <note>t</note>\r\n   \
             <outer><empty /></outer>\r\n\
             <dif>\r\n\
             RkFD\r\n\
             </dif>\r\n\
             </config>"
        );
    }

    #[test]
    fn a_member_s_xml_ends_with_the_dif_tail_a_generated_one_has() {
        let dif = b"FACE and more bytes";
        let generated = face_xml(&[], dif);
        let written = user_face_xml(&[], dif);
        assert_eq!(written, generated);
        // Both end with the `<dif>` line, then `</config>` with no final line end.
        let tail = format!("<dif>\r\n{}\r\n</dif>\r\n</config>", STANDARD.encode(dif));
        assert!(written.ends_with(tail.as_bytes()));
    }
}
