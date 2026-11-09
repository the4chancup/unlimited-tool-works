//! A member's own `face.xml` in a pre-Fox face folder, the authority on what the face loads in
//! place of the one the compiler generates
//! (`spec/team_compiler/messages.md` "User-supplied face.xml"). It is read (`parse`), each
//! `path` and `material` resolved to the file it names (`reference`, `resolve`), and checked
//! (`check`): errors for what is known not to work, warnings for what the compiler cannot vouch
//! for. What it does not know is kept as the member wrote it (unknown elements and attributes),
//! for the face task to write back. The deep pass checks with it.

use std::fmt;

use aesthetics_export::{
    Disposition, FileDescriptor, FileKind, KitToken, ModelFormat, SharedModelFolder, classify,
    common_link_target, kit_token,
};
use pes_version::PesVersion;
use vtree::ScopePath;

use crate::deep::relative;
use crate::face_diff::{self, FaceDiffError};
use crate::face_xml::is_generated_type;
use crate::kit_variants::{kit_fallback_stem, member_kit, representative};
use crate::messages::Code;
use crate::plan::roles::{common_file, file_stem, role_position};
use crate::texture_lookup::split;

/// A finding `check` makes on the folder holding the xml: the code, what is done about it
/// (`DropFolder` for an Error, `Keep` for a Warning or an Info) and its context.
pub(crate) type XmlFinding = (Code, Disposition, Vec<(&'static str, String)>);

/// A member's own `face.xml`: the children of its `<config>` root, in source order. Comments
/// and processing instructions are not kept: the compiler re-serializes the file it emits.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UserFaceXml {
    /// `<config>`'s child elements, in source order.
    pub(crate) children: Vec<Child>,
}

/// One child element of a `face.xml`'s `<config>`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Child {
    /// A `<model>`: one model the face loads.
    Model(ModelElement),
    /// A `<dif>`: the face diff it holds, decoded and checked as a `face_diff.xml`'s is.
    Dif(Vec<u8>),
    /// Any other element, kept verbatim (`xml_element_unknown`).
    Other(Element),
}

/// A `<model>` element: its attributes in source order, names and values as written.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ModelElement {
    /// The attributes (name, value), in source order.
    pub(crate) attributes: Vec<(String, String)>,
}

impl ModelElement {
    /// The value of the attribute `name`, when the element has it.
    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(attribute, _)| attribute == name)
            .map(|(_, value)| value.as_str())
    }
}

/// An element the compiler does not know, as written: its name, attributes in source order,
/// text (trimmed) and child elements.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Element {
    /// The element's name.
    pub(crate) name: String,
    /// Its attributes (name, value), in source order.
    pub(crate) attributes: Vec<(String, String)>,
    /// Its text, the text nodes among its children joined and trimmed.
    pub(crate) text: String,
    /// Its child elements, in source order.
    pub(crate) children: Vec<Element>,
}

/// What a `path` or `material` value names.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reference {
    /// `./<path>`: a file of the face at that path relative to the xml's folder, the file
    /// name's `*` read as `win32` (`./face_high_*.model` names `face_high_win32.model` directly
    /// in the folder, `./jessie/body/oral_x_*.model` names `jessie/body/oral_x_win32.model`).
    /// Holds the path, its directory as written.
    Local(String),
    /// `team_common_folder/<name>` or its long form
    /// `model/character/uniform/common/team_folder/<name>` (`team_common_folder` and
    /// `team_folder` in any letter
    /// case): a file directly in the export's `Common/`, its `*` read as `win32`;
    /// `team_common_folder/<subfolder>/<name>`, a file of that subfolder of `Common/`, which on
    /// PES 15-17 the team's Common output packs at its own path. `team_folder` stands for
    /// the team, written out as the team's ID.
    Common {
        /// The file's path relative to `Common/` (its name for a direct file), the file name's
        /// `*` read as `win32`.
        file_name: String,
    },
    /// A folder after `common_folder/`, `team_folder` included (`common_folder/797/`,
    /// `common_folder/team_folder/x.dds`), or other than `team_folder` after its long form
    /// (`model/character/uniform/common/797/`), or a value starting
    /// `common/`, a CPK's root folder, which no reference names: the Studio format names no
    /// team ID (`xml_common_path_invalid` in a `face.xml`, `mtl_common_path_invalid` in a
    /// `.mtl`).
    CommonInvalid,
    /// A file directly below `common_folder/` or its long form (`common_folder/specular.dds`):
    /// the game's
    /// own file, which a Common path into a folder never names. Holds the file's name;
    /// written out, a short `common_folder/<name>` becomes the long form, the only one the
    /// game reads.
    GameCommon(String),
    /// Any other form, which the compiler cannot resolve (`xml_path_unchecked`): a face Common
    /// path, a `./` path with an empty, `.` or `..` segment, a bare name. Holds the value as
    /// written.
    Unchecked(String),
}

/// The game folder a `Common` or `GameCommon` reference names its file in, before
/// `team_folder` and the file's path relative to `Common/` for a `Common` one.
pub(crate) const UNIFORM_COMMON: &str = "model/character/uniform/common/";

/// The folder of the uniform Common folder that stands for the team's own, in a member's
/// `face.xml` or `.mtl` or an FMDL's Fox common directory: `team_folder` under the long
/// form or `team_common_folder/` written short, the game's team ID written without one.
/// Each name is compared ASCII case-insensitively.
pub(crate) const TEAM_FOLDER: &str = "team_folder";

/// The team's Common written short in a `path`/`material` value or an FMDL's texture
/// directory: `team_common_folder/`, one folder, compared ASCII case-insensitively.
pub(crate) const TEAM_COMMON_FOLDER: &str = "team_common_folder/";

/// The game's own Common folder written short in a `path`/`material` value or an
/// FMDL's texture directory: `common_folder/`, compared as written.
pub(crate) const COMMON_FOLDER: &str = "common_folder/";

/// What the `path` or `material` value `value` names (`Reference`).
pub(crate) fn reference(value: &str) -> Reference {
    if let Some(path) = value.strip_prefix("./")
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
    {
        // Only the file name's `*` is the game's placeholder; a directory is named as written.
        let (directory, name) = split(path);
        return Reference::Local(format!("{directory}{}", name.replace('*', "win32")));
    }
    // `common` is a CPK's root folder (`common/character1/…`), not a reference prefix: a
    // value starting `common/` names the archive and is refused.
    if value.starts_with("common/") {
        return Reference::CommonInvalid;
    }
    if let Some(path) = value
        .get(..TEAM_COMMON_FOLDER.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(TEAM_COMMON_FOLDER))
        .map(|_| &value[TEAM_COMMON_FOLDER.len()..])
    {
        return common_rest(value, path);
    }
    let rest = value
        .strip_prefix(UNIFORM_COMMON)
        .map(|rest| (rest, true))
        .or_else(|| value.strip_prefix(COMMON_FOLDER).map(|rest| (rest, false)));
    let Some((rest, long_form)) = rest else {
        return Reference::Unchecked(value.to_owned());
    };
    match rest.split_once('/') {
        // `team_folder` under the long form alone: the short form spells the team's Common
        // `team_common_folder/`, so a folder after `common_folder/` is refused.
        Some((folder, path)) if long_form && folder.eq_ignore_ascii_case(TEAM_FOLDER) => {
            common_rest(value, path)
        }
        Some(_) => Reference::CommonInvalid,
        None if rest.is_empty() => Reference::Unchecked(value.to_owned()),
        None => Reference::GameCommon(rest.to_owned()),
    }
}

/// The file `path` below the team's Common names (`Reference::Common`), read after
/// `team_folder/` or `team_common_folder/`: `*` in the file name read as `win32`;
/// `Unchecked` for an empty path or a directory alone.
fn common_rest(value: &str, path: &str) -> Reference {
    if path.is_empty() || path.ends_with('/') {
        return Reference::Unchecked(value.to_owned());
    }
    // Only the file name's `*` is the game's placeholder; a subfolder is named as
    // written.
    let file_name = match path.rsplit_once('/') {
        Some((directory, name)) => format!("{directory}/{}", name.replace('*', "win32")),
        None => path.replace('*', "win32"),
    };
    Reference::Common { file_name }
}

/// Why a `face.xml` cannot be read.
#[derive(Debug)]
pub(crate) enum XmlError {
    /// The file is not UTF-8 text (`xml_broken`).
    Utf8,
    /// The file is not well-formed XML (`xml_broken`), with the `line:column` its text ends
    /// at, where an error the parser places nowhere (a file cut short, an unclosed root, no
    /// root at all) is found.
    Xml {
        /// The parser's error.
        error: roxmltree::Error,
        /// The position just past the text's last character, `line:column` from `1:1`.
        end: String,
    },
    /// The root element, named here, is not `<config>` (`xml_root_tag_invalid`).
    Root(String),
    /// A `<dif>` that is not a face diff the game can read (`face_diff_invalid`).
    Dif(FaceDiffError),
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmlError::Utf8 => write!(f, "the file is not UTF-8 text"),
            // Most of the parser's messages carry their own `line:column`; the three that do
            // not are found at the text's end, where the parser gave up.
            XmlError::Xml { error, end } => match error {
                roxmltree::Error::NoRootNode
                | roxmltree::Error::UnclosedRootNode
                | roxmltree::Error::UnexpectedEndOfStream => {
                    write!(f, "the file is not well-formed XML: {error} at {end}")
                }
                _ => write!(f, "the file is not well-formed XML: {error}"),
            },
            XmlError::Root(name) => write!(f, "its root element is <{name}>, not <config>"),
            XmlError::Dif(error) => write!(f, "its <dif>: {error}"),
        }
    }
}

/// The member's `face.xml` `bytes`: UTF-8 text (a byte order mark skipped), well-formed XML
/// whose root is `<config>`, each child element read as a `Child`, a `<dif>`'s face diff
/// decoded and checked (`face_diff::from_dif`).
pub(crate) fn parse(bytes: &[u8]) -> Result<UserFaceXml, XmlError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| XmlError::Utf8)?;
    let document = roxmltree::Document::parse(text).map_err(|error| XmlError::Xml {
        error,
        end: end_position(text),
    })?;
    let root = document.root_element();
    let name = root.tag_name().name();
    if name != "config" {
        return Err(XmlError::Root(name.to_owned()));
    }
    let children = root
        .children()
        .filter(|node| node.is_element())
        .map(child)
        .collect::<Result<Vec<Child>, XmlError>>()?;
    Ok(UserFaceXml { children })
}

/// The position just past the last character of `text`, as `line:column` counted from `1:1`
/// in characters, the way the parser counts the positions it reports.
fn end_position(text: &str) -> String {
    let line = text.matches('\n').count() + 1;
    let last_line = text.rsplit_once('\n').map_or(text, |(_, last)| last);
    format!("{line}:{}", last_line.chars().count() + 1)
}

/// The `Child` the element `node` of `<config>` is.
fn child(node: roxmltree::Node) -> Result<Child, XmlError> {
    match node.tag_name().name() {
        "model" => Ok(Child::Model(ModelElement {
            attributes: attributes(node),
        })),
        "dif" => face_diff::from_dif(node)
            .map(Child::Dif)
            .map_err(XmlError::Dif),
        _ => Ok(Child::Other(element(node))),
    }
}

/// The element `node` as written, its child elements with it.
fn element(node: roxmltree::Node) -> Element {
    let text: String = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect();
    Element {
        name: node.tag_name().name().to_owned(),
        attributes: attributes(node),
        text: text.trim().to_owned(),
        children: node
            .children()
            .filter(|child| child.is_element())
            .map(element)
            .collect(),
    }
}

/// The attributes of `node` (name, value), in source order.
fn attributes(node: roxmltree::Node) -> Vec<(String, String)> {
    node.attributes()
        .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
        .collect()
}

/// The files a face folder's `face.xml` may name.
pub(crate) struct FaceFiles<'a> {
    /// The folder's own files; a `./` reference looks among those where they take a role
    /// (`role_position`), by their paths relative to the folder.
    pub(crate) own: &'a [FileDescriptor],
    /// The shared face folder the player links, whose files a `./` reference looks among
    /// after his own, at their paths below it; `None` when he links none.
    pub(crate) linked_face: Option<&'a SharedModelFolder>,
    /// The export's `Common/` files a target reads; a Common reference looks among them by
    /// their path relative to `Common/`.
    pub(crate) common: &'a [FileDescriptor],
    /// In a refs export on PES 15-17, the referee template tree's files in the referees'
    /// Common output (`templates::referee_common_paths`), by their paths relative to it,
    /// case-folded: a Common reference `common` holds nothing for may name one of them, which
    /// the refs CPK carries; empty for a team export and on PES 18-21.
    pub(crate) template_common: &'a [String],
    /// The face folder holding the xml.
    pub(crate) folder: &'a ScopePath,
}

impl<'a> FaceFiles<'a> {
    /// The folder's own files a `./` reference to a file of `kind` may name, where they take a
    /// role (`role_position`), each by its path relative to the folder with the file it stands
    /// for (`candidate`).
    pub(crate) fn own_local(&self, kind: FileKind) -> Vec<(String, &'a FileDescriptor)> {
        self.own
            .iter()
            .filter(|file| role_position(self.folder, file))
            .filter_map(|file| candidate(file, self.folder, kind, self.common))
            .collect()
    }

    /// The linked shared face's files of `kind` a `./` reference may name, where they take a
    /// role (`role_position`), each by its path relative to that folder (`jessie/x.model`
    /// naming `jessie/x.model` below it).
    pub(crate) fn linked_local(&self, kind: FileKind) -> Vec<(String, &'a FileDescriptor)> {
        let Some(face) = self.linked_face else {
            return Vec::new();
        };
        face.files
            .iter()
            .filter(|file| file.kind == kind && role_position(&face.path, file))
            .map(|file| (relative(&file.path, &face.path), file))
            .collect()
    }
}

/// The file `reference` names among `files`, of `kind` (`FileKind::Model(PesModel)` for a
/// `path`, `FileKind::Mtl` for a `material`), its path compared case-folded: a `Local` one
/// among the folder's own files by their paths relative to the folder (`own_local`, a
/// `.mtl.common` link counting as a `.mtl` of its linked name and standing for the `Common/`
/// file it names), then among the linked shared face's (`linked_local`); a `Common` one among
/// `Common/`'s files by their path relative to `Common/`, a subfolder's included. A name with
/// a `kitN` token names its set in the directory it names, found when a member of it is
/// there — a variant or the `kitF` fallback — the lowest one returned. `None` when none is
/// there, and for an `Unchecked` reference.
pub(crate) fn resolve<'a>(
    reference: &Reference,
    files: &FaceFiles<'a>,
    kind: FileKind,
) -> Option<&'a FileDescriptor> {
    match reference {
        Reference::Local(path) => {
            named(&files.own_local(kind), path).or_else(|| named(&files.linked_local(kind), path))
        }
        Reference::Common { file_name } => {
            // Each by its path relative to `Common/`, which for a direct file is its name.
            let common: Vec<(String, &FileDescriptor)> = files
                .common
                .iter()
                .filter(|file| file.kind == kind)
                .map(|file| {
                    let relative: Vec<&str> = file.path.segments().skip(1).collect();
                    (relative.join("/"), file)
                })
                .collect();
            named(&common, file_name)
        }
        Reference::CommonInvalid | Reference::GameCommon(_) | Reference::Unchecked(_) => None,
    }
}

/// The path `file`, one of the own files of the folder at `folder`, goes by for a reference to
/// a file of `kind`, with the file it stands for: its own path relative to the folder, or for
/// a `.mtl` a `.mtl.common` link's linked name in the link's directory and the `common` file
/// at that path (`common_link_target`, as `mtl_search` counts one). `None` for a file of
/// another kind.
fn candidate<'a>(
    file: &'a FileDescriptor,
    folder: &ScopePath,
    kind: FileKind,
    common: &'a [FileDescriptor],
) -> Option<(String, &'a FileDescriptor)> {
    let path = relative(&file.path, folder);
    if file.kind == kind {
        return Some((path, file));
    }
    if kind != FileKind::Mtl || file.kind != FileKind::CommonLink {
        return None;
    }
    let linked = common_link_target(&file.path, folder)?;
    if classify(split(&linked).1) != FileKind::Mtl {
        return None;
    }
    let target = common_file(common, &linked)?;
    Some((linked, target))
}

/// The file among `candidates` (path, file) that the referenced path `referenced` names: the
/// one of its path, case-folded — a `kitF` file answering no name of its own — else, for a
/// `kitN` reference, the set's representative: its lowest-numbered variant, or its `kitF`
/// fallback when it has no variant (`variant_of`, `kit_variants::representative`).
fn named<'a>(
    candidates: &[(String, &'a FileDescriptor)],
    referenced: &str,
) -> Option<&'a FileDescriptor> {
    let key = vtree::fold_name(referenced);
    if let Some((_, file)) = candidates.iter().find(|(path, _)| {
        vtree::fold_name(path) == key && !kit_fallback_stem(file_stem(split(path).1))
    }) {
        return Some(file);
    }
    let members: Vec<(Option<u8>, &FileDescriptor)> = candidates
        .iter()
        .filter_map(|(path, file)| {
            variant_of(referenced, path).map(|token| (member_kit(token), *file))
        })
        .collect();
    representative(&members, |member| member.0).map(|index| members[index].1)
}

/// The token of the member of the set at `path` that the `kitN` path `referenced` names:
/// the variant's kit (`pants_kit2.model` of `pants_kitN.model`, `jessie/pants_kit2.model`
/// of `jessie/pants_kitN.model`) or the `kitF` fallback: in the same directory, and its
/// stem of the same set, both
/// compared case-folded; `None` otherwise, and when `referenced` is no kit reference. A bare
/// name is a path with no directory.
pub(crate) fn variant_of(referenced: &str, path: &str) -> Option<KitToken> {
    let (referenced_directory, referenced_name) = split(referenced);
    let (directory, name) = split(path);
    if vtree::fold_name(referenced_directory) != vtree::fold_name(directory) {
        return None;
    }
    let Some((KitToken::Reference, set)) = kit_token(file_stem(referenced_name)) else {
        return None;
    };
    let (member, held_set) = match kit_token(file_stem(name)) {
        Some((member @ (KitToken::Variant(_) | KitToken::Fallback), held_set)) => {
            (member, held_set)
        }
        Some((KitToken::Reference, _)) | None => return None,
    };
    (vtree::fold_name(&held_set) == vtree::fold_name(&set)).then_some(member)
}

/// The `<model>` attributes the compiler knows.
const KNOWN_ATTRIBUTES: [&str; 5] = ["level", "type", "path", "material", "ratio"];

/// The prefixes PES 2016 needs a model's file name to start with to load it.
const PES16_PREFIXES: [&str; 3] = ["face_high_", "hair_high_", "oral_"];

/// The findings of the member's `face.xml` `xml`, named `name` below its folder, among the
/// files `files` it may name, for the target `version`, in document order, each `<model>`
/// counted from 1 as `entry`:
///
/// - Errors, dropping the folder: a `<model>` without `type` (`xml_model_type_missing`) or
///   without `path` (`xml_model_path_missing`, and no other finding on its path); a Common
///   reference naming a folder other than `team_folder` (`xml_common_path_invalid`); a `./` or
///   Common reference naming no file `resolve` finds (`xml_model_not_found`); on PES 2016 a
///   model file name starting with none of `face_high_`, `hair_high_`, `oral_`, in any case
///   (`xml_oral_prefix_missing`); a `<dif>` beside a `face_diff.xml` (`xml_dif_conflict`; a
///   `face_diff.bin` beside it is the dual-engine layout, no finding).
/// - Warnings, kept: an unknown element (`xml_element_unknown`) or `<model>` attribute
///   (`xml_attribute_unknown`), a type the compiler does not generate (`xml_type_unknown`), a
///   `ratio` that is no finite number (`xml_ratio_invalid`), a reference the compiler cannot
///   resolve (`xml_path_unchecked`), more than one `face_neck` entry
///   (`xml_face_neck_multiple`, once, after the entries), and each `.model` of the folder's
///   own, where it takes a role, that no `path` names by its path relative to the folder
///   (`names_file`; `xml_model_unlisted`, last, in the files' order; a `kitN` reference names
///   every member of its set).
/// - Info: a `level` other than `0` (`xml_level_lod`).
///
/// An entry without `material` is not a finding: it is written without one. An empty `type`
/// or `path` is a missing one. No bytes are read: the file lists are enough.
pub(crate) fn check(
    xml: &UserFaceXml,
    name: &str,
    files: &FaceFiles,
    version: PesVersion,
) -> Vec<XmlFinding> {
    let mut findings = Vec::new();
    let mut entry = 0;
    let mut face_necks = 0;
    for child in &xml.children {
        match child {
            Child::Model(model) => {
                entry += 1;
                if model.attribute("type") == Some("face_neck") {
                    face_necks += 1;
                }
                findings.extend(model_findings(model, entry, files, version));
            }
            Child::Dif(_) if holds_face_diff_xml(files) => {
                findings.push(error(Code::XmlDifConflict, vec![("file", name.to_owned())]))
            }
            Child::Dif(_) => {}
            Child::Other(element) => findings.push(kept(
                Code::XmlElementUnknown,
                vec![("element", element.name.clone())],
            )),
        }
    }
    if face_necks > 1 {
        findings.push(kept(
            Code::XmlFaceNeckMultiple,
            vec![("count", face_necks.to_string())],
        ));
    }
    let paths: Vec<&str> = xml
        .children
        .iter()
        .filter_map(|child| match child {
            Child::Model(model) => model.attribute("path"),
            Child::Dif(_) | Child::Other(_) => None,
        })
        .collect();
    for file in files.own {
        let below = relative(&file.path, files.folder);
        let unlisted = file.kind == FileKind::Model(ModelFormat::PesModel)
            && role_position(files.folder, file)
            && !paths.iter().any(|path| names_file(path, &below));
        if unlisted {
            findings.push(kept(Code::XmlModelUnlisted, vec![("file", below)]));
        }
    }
    findings
}

/// The findings of the `<model>` `model`, the `entry`th, for `check`.
fn model_findings(
    model: &ModelElement,
    entry: usize,
    files: &FaceFiles,
    version: PesVersion,
) -> Vec<XmlFinding> {
    let mut findings = Vec::new();
    let present = |name: &str| model.attribute(name).filter(|value| !value.is_empty());
    let xml_type = present("type");
    let path = present("path");
    if xml_type.is_none() {
        let mut context = vec![("entry", entry.to_string())];
        context.extend(path.map(|path| ("path", path.to_owned())));
        findings.push(error(Code::XmlModelTypeMissing, context));
    }
    if path.is_none() {
        let mut context = vec![("entry", entry.to_string())];
        context.extend(xml_type.map(|xml_type| ("type", xml_type.to_owned())));
        findings.push(error(Code::XmlModelPathMissing, context));
    }
    for (attribute, _) in &model.attributes {
        if !KNOWN_ATTRIBUTES.contains(&attribute.as_str()) {
            findings.push(kept(
                Code::XmlAttributeUnknown,
                vec![
                    ("entry", entry.to_string()),
                    ("attribute", attribute.clone()),
                ],
            ));
        }
    }
    if let Some(xml_type) = xml_type
        && !is_generated_type(xml_type)
    {
        findings.push(kept(
            Code::XmlTypeUnknown,
            vec![("type", xml_type.to_owned())],
        ));
    }
    if let Some(level) = model.attribute("level")
        && level != "0"
    {
        findings.push(kept(Code::XmlLevelLod, vec![("level", level.to_owned())]));
    }
    if let Some(ratio) = model.attribute("ratio")
        && !ratio.trim().parse::<f64>().is_ok_and(f64::is_finite)
    {
        findings.push(kept(
            Code::XmlRatioInvalid,
            vec![("ratio", ratio.to_owned())],
        ));
    }
    if let Some(path) = path {
        if version == PesVersion::Pes16 && !has_pes16_prefix(path) {
            findings.push(error(
                Code::XmlOralPrefixMissing,
                vec![("path", path.to_owned())],
            ));
        }
        findings.extend(reference_finding(
            "path",
            path,
            files,
            FileKind::Model(ModelFormat::PesModel),
        ));
    }
    if let Some(material) = present("material") {
        findings.extend(reference_finding(
            "material",
            material,
            files,
            FileKind::Mtl,
        ));
    }
    findings
}

/// The finding on the `attribute` (`path` or `material`) of value `value`, naming a file of
/// `kind`: `xml_path_unchecked` for a form the compiler cannot resolve (a file directly
/// below `common_folder/` included),
/// `xml_common_path_invalid` for a Common reference naming a folder other than
/// `team_folder`,
/// `xml_model_not_found` when the file it names is not among `files`: neither one `resolve`
/// finds nor, for a Common reference, one of `files.template_common` of that kind.
fn reference_finding(
    attribute: &'static str,
    value: &str,
    files: &FaceFiles,
    kind: FileKind,
) -> Option<XmlFinding> {
    let context = vec![
        ("attribute", attribute.to_owned()),
        ("value", value.to_owned()),
    ];
    let reference = reference(value);
    match &reference {
        Reference::GameCommon(_) | Reference::Unchecked(_) => {
            return Some(kept(Code::XmlPathUnchecked, context));
        }
        Reference::CommonInvalid => {
            return Some(error(Code::XmlCommonPathInvalid, context));
        }
        Reference::Local(_) | Reference::Common { .. } => {}
    }
    if resolve(&reference, files, kind).is_some() {
        return None;
    }
    // Not through `resolve`, which returns the export file named: a template file is none,
    // and the face packs nothing for it, naming it in the Common output where the refs CPK
    // holds it unless the export's file of that path replaced it.
    let in_template = match &reference {
        Reference::Common { file_name } => {
            classify(file_name) == kind
                && files.template_common.contains(&vtree::fold_name(file_name))
        }
        Reference::Local(_)
        | Reference::CommonInvalid
        | Reference::GameCommon(_)
        | Reference::Unchecked(_) => false,
    };
    (!in_template).then(|| error(Code::XmlModelNotFound, context))
}

/// Whether the model file name the `path` value `path` ends with (after its last `/`, the
/// whole value when it has none) starts, in any case, with a prefix PES 2016 needs.
fn has_pes16_prefix(path: &str) -> bool {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    let name = name.to_ascii_lowercase();
    PES16_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Whether the `path` or `material` value `path` names the file of the folder at `below`, its
/// path relative to the folder (`jessie/body/x.model`, its name for a file directly in it): a
/// `./` reference to that path, case-folded, or to the `kitN` set it is a variant of.
pub(crate) fn names_file(path: &str, below: &str) -> bool {
    let Reference::Local(referenced) = reference(path) else {
        return false;
    };
    vtree::fold_name(&referenced) == vtree::fold_name(below)
        || variant_of(&referenced, below).is_some()
}

/// Whether the folder holds a `face_diff.xml` where it takes a role: a second source
/// beside a `<dif>`. A `face_diff.bin` is none: beside a `<dif>` it is the dual-engine layout,
/// the `<dif>` going out on PES 15-17 and the bin on PES 18-21.
fn holds_face_diff_xml(files: &FaceFiles) -> bool {
    files.own.iter().any(|file| {
        file.path.name().eq_ignore_ascii_case("face_diff.xml") && role_position(files.folder, file)
    })
}

/// An Error finding with `context`: the folder is dropped.
fn error(code: Code, context: Vec<(&'static str, String)>) -> XmlFinding {
    (code, Disposition::DropFolder, context)
}

/// A Warning or Info finding with `context`: the folder is kept.
fn kept(code: Code, context: Vec<(&'static str, String)>) -> XmlFinding {
    (code, Disposition::Keep, context)
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    /// The one real member `face.xml` in the fixtures, the pre-Fox tracer's Fumos face.
    const FUMOS_XML: &[u8] = include_bytes!(
        "../tests/fixtures/tracer_prefox/old/jp Tracer/Faces/XXX20 - Fumos/face.xml"
    );

    /// The files beside it, as its folder lists them.
    const FUMOS_FILES: [&str; 20] = [
        "boots.mtl",
        "eye_occlusion.dds",
        "face.dds",
        "face.mtl",
        "face.xml",
        "face_high_win32.model",
        "face_normal.dds",
        "face_normal_detail.dds",
        "face_specular_roughness.dds",
        "glove_c.dds",
        "glove_l.mtl",
        "glove_n.dds",
        "glove_r.mtl",
        "glove_sr.dds",
        "k2012_c.dds",
        "k2012_n.dds",
        "k2012_sr.dds",
        "oral_boots_win32.model",
        "oral_glove_l_win32.model",
        "oral_glove_r_win32.model",
    ];

    /// The folder the tests' files sit in.
    const FOLDER: &str = "Players/05 - A";

    /// The descriptor of the export file at `path`, classified by its name.
    fn descriptor(path: &str) -> FileDescriptor {
        let path = ScopePath::new(path).unwrap();
        FileDescriptor {
            size: 1,
            kind: classify(path.name()),
            source: path.clone(),
            path,
        }
    }

    /// The descriptors of `names`, each below `folder`.
    fn files_in(folder: &str, names: &[&str]) -> Vec<FileDescriptor> {
        names
            .iter()
            .map(|name| descriptor(&format!("{folder}/{name}")))
            .collect()
    }

    /// The shared face folder `Faces/Round` holding the files `names`.
    fn round(names: &[&str]) -> SharedModelFolder {
        SharedModelFolder {
            path: ScopePath::new("Faces/Round").unwrap(),
            folder_name: "Round".to_owned(),
            files: files_in("Faces/Round", names),
        }
    }

    /// The export's `Common/` files the tests name.
    fn common() -> Vec<FileDescriptor> {
        files_in("Common", &["legs.model", "body.mtl"])
    }

    /// Each finding as one line: code, disposition, context.
    fn lines(findings: Vec<XmlFinding>) -> Vec<String> {
        findings
            .into_iter()
            .map(|(code, disposition, context)| {
                let context: Vec<String> = context
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect();
                format!(
                    "{} [{disposition:?}] ({})",
                    code.as_str(),
                    context.join(", ")
                )
            })
            .collect()
    }

    /// `check`'s findings, as `lines`, on the xml `text` in `FOLDER` holding `own` and linking
    /// the shared face `Faces/Round` holding `hair_high.model`, for `version`.
    fn checked(text: &str, own: &[&str], version: PesVersion) -> Vec<String> {
        let own = files_in(FOLDER, own);
        let linked_face = round(&["hair_high.model"]);
        let common = common();
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &own,
            linked_face: Some(&linked_face),
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        lines(check(
            &parse(text.as_bytes()).unwrap(),
            "face.xml",
            &files,
            version,
        ))
    }

    /// The face model's entry, which every hand-written document holds first, its files
    /// `face_high.model` and `face_high.mtl` beside it: it gives no finding.
    const FACE: &str = r#"<model level="0" type="face_neck" path="./face_high.model" material="./face_high.mtl"/>"#;

    /// The face's own files, which `FACE` lists.
    const FACE_FILES: [&str; 2] = ["face_high.model", "face_high.mtl"];

    /// `checked` on a `<config>` holding `FACE` then `rest`, for PES 17, the folder holding
    /// `FACE_FILES` and `own`.
    fn with_face(rest: &str, own: &[&str]) -> Vec<String> {
        let own: Vec<&str> = FACE_FILES
            .iter()
            .copied()
            .chain(own.iter().copied())
            .collect();
        checked(
            &format!("<config>{FACE}{rest}</config>"),
            &own,
            PesVersion::Pes17,
        )
    }

    #[test]
    fn the_fumos_xml_reads_as_four_models_and_its_face_diff() {
        let xml = parse(FUMOS_XML).unwrap();
        let paths: Vec<Option<&str>> = xml
            .children
            .iter()
            .filter_map(|child| match child {
                Child::Model(model) => Some(model.attribute("path")),
                Child::Dif(_) | Child::Other(_) => None,
            })
            .collect();
        assert_eq!(
            paths,
            [
                Some("./face_high_*.model"),
                Some("./oral_glove_l_*.model"),
                Some("./oral_glove_r_*.model"),
                Some("./oral_boots_*.model"),
            ]
        );
        let difs: Vec<&Vec<u8>> = xml
            .children
            .iter()
            .filter_map(|child| match child {
                Child::Dif(dif) => Some(dif),
                Child::Model(_) | Child::Other(_) => None,
            })
            .collect();
        let [dif] = difs.as_slice() else {
            panic!("{difs:?}");
        };
        face_diff::check(dif).unwrap();
        assert_eq!(xml.children.len(), 5);
        // The attributes keep their source order.
        let Child::Model(first) = &xml.children[0] else {
            panic!("{:?}", xml.children[0]);
        };
        let names: Vec<&str> = first
            .attributes
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(names, ["level", "type", "path", "material"]);
    }

    #[test]
    fn the_fumos_xml_s_one_finding_is_its_boots_type_on_pes_16_and_17() {
        let own = files_in(FOLDER, &FUMOS_FILES);
        let common = common();
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &own,
            linked_face: None,
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        let xml = parse(FUMOS_XML).unwrap();
        for version in [PesVersion::Pes16, PesVersion::Pes17] {
            assert_eq!(
                lines(check(&xml, "face.xml", &files, version)),
                ["xml_type_unknown [Keep] (type=boots)"],
                "{version:?}"
            );
        }
    }

    #[test]
    fn a_reference_s_form_decides_what_it_names() {
        assert_eq!(
            reference("./face_high_*.model"),
            Reference::Local("face_high_win32.model".to_owned())
        );
        for value in [
            "team_common_folder/legs_*.model",
            "model/character/uniform/common/team_folder/legs_*.model",
            "model/character/uniform/common/Team_Folder/legs_*.model",
        ] {
            assert_eq!(
                reference(value),
                Reference::Common {
                    file_name: "legs_win32.model".to_owned(),
                },
                "{value}"
            );
        }
        // A subfolder of `Common/` is named as written, the file name's `*` read as `win32`,
        // `team_common_folder` in any letter case.
        assert_eq!(
            reference("model/character/uniform/common/team_folder/a/b.model"),
            Reference::Common {
                file_name: "a/b.model".to_owned(),
            }
        );
        assert_eq!(
            reference("Team_Common_Folder/a/b_*.model"),
            Reference::Common {
                file_name: "a/b_win32.model".to_owned(),
            }
        );
        // Any folder after `common_folder/`, `team_folder` included, is refused; so is any
        // other folder after the long form, a team ID, `XXX` or another name, however
        // long; and any value starting `common/`, a CPK's root folder, whatever follows it.
        for invalid in [
            "model/character/uniform/common/797/x.dds",
            "model/character/uniform/common/XXX/a/b.model",
            "model/character/uniform/common/XX/legs.model",
            "common_folder/797/x.dds",
            "common_folder/XXX/a/",
            "common_folder/XXX/",
            "common_folder/o/env.dds",
            "common_folder/refs/refkit/x.dds",
            "common_folder/team_folder/legs.model",
            "common_folder/team_folder/",
            "common/team_folder/legs.model",
            "common/797/x.dds",
            "common/specular.dds",
            "common/character1/model/character/uniform/common/714/x.dds",
            "common/",
        ] {
            assert_eq!(reference(invalid), Reference::CommonInvalid, "{invalid}");
        }
        // A file directly below `common_folder/` is the game's own, in either spelling.
        for (value, name) in [
            ("common_folder/specular.dds", "specular.dds"),
            ("model/character/uniform/common/legs.model", "legs.model"),
            ("common_folder/team_folder", "team_folder"),
        ] {
            assert_eq!(
                reference(value),
                Reference::GameCommon(name.to_owned()),
                "{value}"
            );
        }
        // A `./` path into a subfolder names the file at that path, relative to the xml's
        // folder: only its file name's `*` is read as `win32`.
        assert_eq!(
            reference("./jessie/body/oral_x_*.model"),
            Reference::Local("jessie/body/oral_x_win32.model".to_owned())
        );
        assert_eq!(
            reference("./Jessie/*_kit/x.mtl"),
            Reference::Local("Jessie/*_kit/x.mtl".to_owned())
        );
        for unchecked in [
            "model/character/face/common/x.model",
            "./",
            "./a/",
            "./a//b.model",
            "./a/./b.model",
            "./../b.model",
            "./a/../b.model",
            "x.model",
            "common_folder/",
            "team_common_folder/",
            "team_common_folder/a/",
            "model/character/uniform/common/team_folder/",
        ] {
            assert_eq!(
                reference(unchecked),
                Reference::Unchecked(unchecked.to_owned()),
                "{unchecked}"
            );
        }
    }

    #[test]
    fn a_reference_resolves_in_the_folder_then_the_linked_face_and_a_material_through_a_link() {
        let own = files_in(
            FOLDER,
            &[
                "Face_High.model",
                "face/hat.model",
                "boots/deep.model",
                "pants_kit2.model",
                "pants_kit1.model",
                "body.mtl.common",
            ],
        );
        let linked_face = round(&["hair_high.model", "face_high.model"]);
        let common = common();
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &own,
            linked_face: Some(&linked_face),
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        let model = FileKind::Model(ModelFormat::PesModel);
        let found = |value: &str, kind| {
            resolve(&reference(value), &files, kind).map(|file| file.path.as_str().to_owned())
        };
        let path = |text: &str| Some(text.to_owned());
        // The folder's own first, case-folded, then the linked face's.
        assert_eq!(
            found("./face_high.model", model),
            path("Players/05 - A/Face_High.model")
        );
        assert_eq!(
            found("./hair_high.model", model),
            path("Faces/Round/hair_high.model")
        );
        // A subfolder's file is named by its path below the folder, case-folded, and by
        // nothing else: `./name` names a file directly in the folder.
        assert_eq!(
            found("./FACE/hat.model", model),
            path("Players/05 - A/face/hat.model")
        );
        assert_eq!(found("./hat.model", model), None);
        assert_eq!(
            found("./boots/deep.model", model),
            path("Players/05 - A/boots/deep.model")
        );
        assert_eq!(found("./deep.model", model), None);
        assert_eq!(found("./face/deep.model", model), None);
        // A kit set's reference finds its lowest variant.
        assert_eq!(
            found("./pants_kitN.model", model),
            path("Players/05 - A/pants_kit1.model")
        );
        // A `.mtl.common` link counts as a `.mtl` of its linked name, standing for Common's.
        assert_eq!(found("./body.mtl", FileKind::Mtl), path("Common/body.mtl"));
        assert_eq!(found("./body.mtl", model), None);
        assert_eq!(
            found("team_common_folder/legs.model", model),
            path("Common/legs.model")
        );
        assert_eq!(found("model/character/face/common/legs.model", model), None);
        // A Common file of another kind is not the one named.
        assert_eq!(
            found("model/character/uniform/common/team_folder/body.mtl", model),
            None
        );
    }

    #[test]
    fn a_local_reference_names_a_file_at_its_path_relative_to_the_xml_s_folder() {
        let own = files_in(
            FOLDER,
            &[
                "face_high.model",
                "jessie/body/x_win32.model",
                "jessie/pants_kit2.model",
                "jessie/pants_kit1.model",
                "pants_kit3.model",
            ],
        );
        // A file below the shared face folder, kept by a lenient check, takes no role there.
        let linked_face = round(&["hair_high.model", "extra/x_win32.model"]);
        let common = common();
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &own,
            linked_face: Some(&linked_face),
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        let model = FileKind::Model(ModelFormat::PesModel);
        let found = |value: &str| {
            resolve(&reference(value), &files, model).map(|file| file.path.as_str().to_owned())
        };
        assert_eq!(found("./x_*.model"), None);
        assert_eq!(found("./x_win32.model"), None);
        assert_eq!(
            found("./jessie/body/x_*.model").as_deref(),
            Some("Players/05 - A/jessie/body/x_win32.model")
        );
        assert_eq!(
            found("./Jessie/Body/X_win32.model").as_deref(),
            Some("Players/05 - A/jessie/body/x_win32.model")
        );
        assert_eq!(found("./body/x_win32.model"), None);
        // A kit reference with a directory names the set in that directory alone.
        assert_eq!(
            found("./jessie/pants_kitN.model").as_deref(),
            Some("Players/05 - A/jessie/pants_kit1.model")
        );
        assert_eq!(
            found("./pants_kitN.model").as_deref(),
            Some("Players/05 - A/pants_kit3.model")
        );
        assert_eq!(found("./body/pants_kitN.model"), None);
    }

    #[test]
    fn a_kit_reference_resolves_to_the_set_s_fallback_which_its_own_name_does_not_name() {
        let own = files_in(FOLDER, &["pants_kit1.model", "pants_kitF.model"]);
        let linked_face = round(&[]);
        let common = common();
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &own,
            linked_face: Some(&linked_face),
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        let model = FileKind::Model(ModelFormat::PesModel);
        let found = |value: &str| {
            resolve(&reference(value), &files, model).map(|file| file.path.as_str().to_owned())
        };
        // `kitN` resolves to the set's representative: the lowest-numbered variant when
        // it has one, the `kitF` fallback alone.
        assert_eq!(
            found("./pants_kitN.model").as_deref(),
            Some("Players/05 - A/pants_kit1.model")
        );
        // A `kitF` name names nothing: the file is never emitted under its own name, in
        // any letter case.
        assert_eq!(found("./pants_kitF.model"), None);
        assert_eq!(found("./PANTS_KITF.model"), None);
        let own_without_variant = files_in(FOLDER, &["pants_kitF.model"]);
        let files = FaceFiles {
            own: &own_without_variant,
            ..files
        };
        let found_alone = |value: &str| {
            resolve(&reference(value), &files, model).map(|file| file.path.as_str().to_owned())
        };
        assert_eq!(
            found_alone("./pants_kitN.model").as_deref(),
            Some("Players/05 - A/pants_kitF.model")
        );
    }

    #[test]
    fn a_linked_face_s_subfolder_file_is_named_by_its_path_below_it() {
        // A shared folder's tree is a player folder's: `./extra/x` names the file at `extra/`
        // below it; a bare name never goes down.
        let folder = ScopePath::new(FOLDER).unwrap();
        let linked_face = round(&[
            "hair_high.model",
            "hair_high.mtl",
            "extra/x_win32.model",
            "jessie/face.xml",
        ]);
        let files = FaceFiles {
            own: &[],
            linked_face: Some(&linked_face),
            common: &[],
            template_common: &[],
            folder: &folder,
        };
        let model = FileKind::Model(ModelFormat::PesModel);
        let found = |value: &str, kind: FileKind| {
            resolve(&reference(value), &files, kind).map(|file| file.path.as_str().to_owned())
        };
        assert_eq!(
            found("./hair_high.model", model).as_deref(),
            Some("Faces/Round/hair_high.model")
        );
        assert_eq!(
            found("./extra/x_win32.model", model).as_deref(),
            Some("Faces/Round/extra/x_win32.model")
        );
        assert_eq!(found("./x_win32.model", model), None);
        // The filter is kind and role: a model asked as `.mtl` is nothing, and a per-player
        // singleton below a subfolder takes no role.
        assert_eq!(found("./hair_high.model", FileKind::Mtl), None);
        assert_eq!(
            found("./hair_high.mtl", FileKind::Mtl).as_deref(),
            Some("Faces/Round/hair_high.mtl")
        );
        assert_eq!(found("./jessie/face.xml", FileKind::Xml), None);
    }

    #[test]
    fn a_common_reference_names_a_file_by_its_path_below_common() {
        let common = files_in(
            "Common",
            &[
                "legs.model",
                "refkit/Oral_Thigh_win32.model",
                "refkit/refkit.mtl",
            ],
        );
        let folder = ScopePath::new(FOLDER).unwrap();
        let files = FaceFiles {
            own: &[],
            linked_face: None,
            common: &common,
            template_common: &[],
            folder: &folder,
        };
        let model = FileKind::Model(ModelFormat::PesModel);
        let found = |value: &str, kind| {
            resolve(&reference(value), &files, kind).map(|file| file.path.as_str().to_owned())
        };
        assert_eq!(
            found(
                "model/character/uniform/common/team_folder/refkit/oral_thigh_*.model",
                model
            )
            .as_deref(),
            Some("Common/refkit/Oral_Thigh_win32.model")
        );
        assert_eq!(
            found("team_common_folder/REFKIT/refkit.mtl", FileKind::Mtl).as_deref(),
            Some("Common/refkit/refkit.mtl")
        );
        // A subfolder's file is no direct one, and a direct file is in no subfolder.
        assert_eq!(
            found(
                "model/character/uniform/common/team_folder/oral_thigh_win32.model",
                model
            ),
            None
        );
        assert_eq!(
            found(
                "model/character/uniform/common/team_folder/refkit/legs.model",
                model
            ),
            None
        );
    }

    #[test]
    fn a_common_reference_the_export_lacks_may_name_a_file_of_the_template_tree() {
        let folder = ScopePath::new(FOLDER).unwrap();
        let template_common =
            ["refkit/oral_arm_win32.model", "refkit/refkit.mtl"].map(str::to_owned);
        let checked_with = |template_common: &[String], path: &str| {
            let files = FaceFiles {
                own: &[],
                linked_face: None,
                common: &[],
                template_common,
                folder: &folder,
            };
            let text = format!(
                r#"<config><model level="0" type="parts" path="{path}" material="model/character/uniform/common/team_folder/refkit/refkit.mtl"/></config>"#
            );
            lines(check(
                &parse(text.as_bytes()).unwrap(),
                "face.xml",
                &files,
                PesVersion::Pes17,
            ))
        };
        let arm = "model/character/uniform/common/team_folder/refkit/oral_arm_*.model";
        assert_eq!(checked_with(&template_common, arm), Vec::<String>::new());
        // Compared case-folded, as the export's own files are.
        let arm_upper = "model/character/uniform/common/Team_Folder/REFKIT/Oral_Arm_*.model";
        assert_eq!(
            checked_with(&template_common, arm_upper),
            Vec::<String>::new()
        );
        let nothing = "team_common_folder/refkit/oral_nothing_*.model";
        assert_eq!(
            checked_with(&template_common, nothing),
            [format!(
                "xml_model_not_found [DropFolder] (attribute=path, value={nothing})"
            )]
        );
        // A file of the tree is named by its own kind only.
        let mtl_as_model = "model/character/uniform/common/team_folder/refkit/refkit.mtl";
        assert_eq!(
            checked_with(&template_common, mtl_as_model),
            [format!(
                "xml_model_not_found [DropFolder] (attribute=path, value={mtl_as_model})"
            )]
        );
        // Without the tree, a team export's, neither file is there.
        let material = "model/character/uniform/common/team_folder/refkit/refkit.mtl";
        assert_eq!(
            checked_with(&[], arm),
            [
                format!("xml_model_not_found [DropFolder] (attribute=path, value={arm})"),
                format!("xml_model_not_found [DropFolder] (attribute=material, value={material})"),
            ]
        );
    }

    #[test]
    fn each_error_of_a_model_entry_drops_the_folder() {
        let cases: [(&str, &[&str], &[&str]); 5] = [
            (
                r#"<model type="parts"/>"#,
                &[],
                &["xml_model_path_missing [DropFolder] (entry=2, type=parts)"],
            ),
            (
                r#"<model path="./hat.model"/>"#,
                &["hat.model"],
                &["xml_model_type_missing [DropFolder] (entry=2, path=./hat.model)"],
            ),
            (
                r#"<model/>"#,
                &[],
                &[
                    "xml_model_type_missing [DropFolder] (entry=2)",
                    "xml_model_path_missing [DropFolder] (entry=2)",
                ],
            ),
            (
                r#"<model type="parts" path="model/character/uniform/common/XX/legs.model"/>"#,
                &[],
                &[
                    "xml_common_path_invalid [DropFolder] (attribute=path, value=model/character/uniform/common/XX/legs.model)",
                ],
            ),
            (
                r#"<model type="parts" path="./hat.model" material="model/character/uniform/common/team_folder/hat.mtl"/>"#,
                &[],
                &[
                    "xml_model_not_found [DropFolder] (attribute=path, value=./hat.model)",
                    "xml_model_not_found [DropFolder] (attribute=material, value=model/character/uniform/common/team_folder/hat.mtl)",
                ],
            ),
        ];
        for (entry, own, expected) in cases {
            assert_eq!(with_face(entry, own), expected, "{entry}");
        }
    }

    #[test]
    fn a_common_or_linked_face_reference_that_exists_and_a_missing_material_are_no_finding() {
        assert_eq!(
            with_face(
                r#"<model type="parts" path="model/character/uniform/common/team_folder/legs.model" material="model/character/uniform/common/team_folder/body.mtl"/><model type="parts" path="./hair_high.model"/>"#,
                &[]
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn on_pes_16_a_model_name_without_its_prefix_drops_the_folder_whatever_its_path_form() {
        let text = r#"<config>
            <model type="face_neck" path="./Face_High_x.model"/>
            <model type="parts" path="./oral_hat.model"/>
            <model type="uniform" path="./body_uniform.model"/>
            <model type="parts" path="model/character/uniform/common/team_folder/legs.model"/>
            <model type="parts" path="model/character/face/common/hat.model"/>
        </config>"#;
        let own = ["Face_High_x.model", "oral_hat.model", "body_uniform.model"];
        assert_eq!(
            checked(text, &own, PesVersion::Pes16),
            [
                "xml_oral_prefix_missing [DropFolder] (path=./body_uniform.model)",
                "xml_oral_prefix_missing [DropFolder] (path=model/character/uniform/common/team_folder/legs.model)",
                "xml_oral_prefix_missing [DropFolder] (path=model/character/face/common/hat.model)",
                "xml_path_unchecked [Keep] (attribute=path, value=model/character/face/common/hat.model)",
            ]
        );
        assert_eq!(
            checked(text, &own, PesVersion::Pes17),
            [
                "xml_path_unchecked [Keep] (attribute=path, value=model/character/face/common/hat.model)"
            ]
        );
    }

    #[test]
    fn a_dif_beside_a_face_diff_xml_is_a_conflict_and_beside_a_bin_none() {
        let dif = base64::engine::general_purpose::STANDARD
            .encode(crate::templates::Templates::embedded().face_diff());
        let rest = format!("<dif>{dif}</dif>");
        assert_eq!(with_face(&rest, &[]), Vec::<String>::new());
        // The dual-engine layout: the `<dif>` for PES 15-17, the bin for PES 18-21.
        assert_eq!(with_face(&rest, &["face_diff.bin"]), Vec::<String>::new());
        assert_eq!(
            with_face(&rest, &["Face_Diff.xml"]),
            ["xml_dif_conflict [DropFolder] (file=face.xml)"]
        );
        // A subfolder's face diff is none of the folder's: the structure pass names it
        // (`file_type_disallowed`), and a lenient check keeps it unread.
        for face_diff_xml in ["face/face_diff.xml", "boots/face_diff.xml"] {
            assert_eq!(
                with_face(&rest, &[face_diff_xml]),
                Vec::<String>::new(),
                "{face_diff_xml}"
            );
        }
    }

    #[test]
    fn what_the_compiler_cannot_vouch_for_is_kept_and_warned() {
        let cases: [(&str, &[&str]); 10] = [
            (
                r#"<extra a="1">text</extra>"#,
                &["xml_element_unknown [Keep] (element=extra)"],
            ),
            (
                r#"<model type="parts" path="./face_high.model" glow="1"/>"#,
                &["xml_attribute_unknown [Keep] (entry=2, attribute=glow)"],
            ),
            (
                r#"<model type="cape" path="./face_high.model"/>"#,
                &["xml_type_unknown [Keep] (type=cape)"],
            ),
            (
                // Every type the compiler itself generates is known, `handL`/`handR` included.
                r#"<model type="uniform_sub" path="./face_high.model"/><model type="gloveL" path="./face_high.model"/><model type="handL" path="./face_high.model"/><model type="handR" path="./face_high.model"/>"#,
                &[],
            ),
            (
                r#"<model level="1" type="parts" path="./face_high.model"/>"#,
                &["xml_level_lod [Keep] (level=1)"],
            ),
            (
                r#"<model type="parts" path="./face_high.model" ratio="abc"/><model type="parts" path="./face_high.model" ratio="inf"/>"#,
                &[
                    "xml_ratio_invalid [Keep] (ratio=abc)",
                    "xml_ratio_invalid [Keep] (ratio=inf)",
                ],
            ),
            (
                r#"<model type="parts" path="./face_high.model" ratio=" 1.5 "/>"#,
                &[],
            ),
            (
                r#"<model type="parts" path="./a/../b.model" material="model/character/face/common/x.mtl"/>"#,
                &[
                    "xml_path_unchecked [Keep] (attribute=path, value=./a/../b.model)",
                    "xml_path_unchecked [Keep] (attribute=material, value=model/character/face/common/x.mtl)",
                ],
            ),
            (
                r#"<model type="face_neck" path="./face_high.model"/>"#,
                &["xml_face_neck_multiple [Keep] (count=2)"],
            ),
            (
                // A comment is dropped, and is no element.
                r#"<!-- a note --><model type="parts" path="./face_high.model"/>"#,
                &[],
            ),
        ];
        for (rest, expected) in cases {
            assert_eq!(with_face(rest, &[]), expected, "{rest}");
        }
    }

    #[test]
    fn a_model_of_the_folder_no_entry_lists_is_unlisted_in_file_order() {
        assert_eq!(
            with_face(
                r#"<model type="parts" path="./Pants_kitN.model"/>"#,
                &[
                    "pants_kit1.model",
                    "face/hat.model",
                    "pants_kit2.model",
                    "boots/x.model",
                    "torso.model",
                ]
            ),
            [
                "xml_model_unlisted [Keep] (file=face/hat.model)",
                "xml_model_unlisted [Keep] (file=boots/x.model)",
                "xml_model_unlisted [Keep] (file=torso.model)",
            ]
        );
    }

    #[test]
    fn an_unknown_element_is_kept_as_written() {
        let xml = parse(br#"<config><extra a="1"> text <!-- c --></extra></config>"#).unwrap();
        assert_eq!(
            xml.children,
            [Child::Other(Element {
                name: "extra".to_owned(),
                attributes: vec![("a".to_owned(), "1".to_owned())],
                text: "text".to_owned(),
                children: Vec::new(),
            })]
        );
    }

    #[test]
    fn a_file_that_is_no_config_document_is_refused_with_its_reason() {
        let error = parse(b"<config><dif>RkFD</dif></config>").unwrap_err();
        assert!(matches!(error, XmlError::Dif(_)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "its <dif>: the face diff is 3 bytes long, shorter than its 80-byte header"
        );
        let error = parse(b"<cfg/>").unwrap_err();
        assert!(
            matches!(&error, XmlError::Root(root) if root == "cfg"),
            "{error:?}"
        );
        let error = parse(b"<config><model").unwrap_err();
        assert!(matches!(error, XmlError::Xml { .. }), "{error:?}");
        assert_eq!(
            error.to_string(),
            "the file is not well-formed XML: the root node was opened but never closed at 1:15"
        );
        let error = parse(b"<config><model></config>").unwrap_err();
        assert_eq!(
            error.to_string(),
            "the file is not well-formed XML: expected 'model' tag, not 'config' at 1:16"
        );
        assert!(matches!(parse(b"\xFF\xFE"), Err(XmlError::Utf8)));
        // A byte order mark is skipped.
        assert!(parse(b"\xEF\xBB\xBF<config/>").is_ok());
    }
}
