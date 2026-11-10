//! File kinds, classified from the name alone. The allowlist is, per folder
//! kind, the set of kinds it admits; `Other` is admitted nowhere but as a
//! `.txt` note (`is_txt_note`), which no package reads. Of a file its folder
//! does not admit, `strict_file_type_check` changes only the disposition. A stray `.txt`
//! suffix after a link or marker name is tolerated (Windows hides known
//! extensions, so users create `Crocs.boots.txt` with Notepad).

use vtree::ScopePath;

/// What a file in an export is, semantically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileKind {
    /// A model source, in one of the three supported formats.
    Model(ModelFormat),
    /// An accepted image extension.
    Texture,
    /// A skeleton file (`.skl`).
    Skl,
    /// A Fox cloth/simulation file (`.fclo`).
    Fclo,
    /// An XML file (`.xml`).
    Xml,
    /// A pre-Fox material file (`.mtl`).
    Mtl,
    /// A glTF materials file (`materials.toml` or `<stem>.materials.toml`).
    MaterialsToml,
    /// A `.bin`: a game bin (`face_diff.bin`) or a glTF buffer; deep glTF
    /// parsing tells them apart.
    Bin,
    /// A shared-folder link file: `Crocs.boots`, `Longhair.face`, `Keeper gloves.gloves`.
    SharedLink(SharedKind),
    /// A `<name>.common` link: stands in for `Common/<name>`.
    CommonLink,
    /// A marker file (empty or a `.txt` spelling of one).
    Marker(Marker),
    /// A metadata file: known name, handled outside the allowlist; a `.txt` one
    /// (`notes.txt`) in a model folder is a note (`is_txt_note`).
    Metadata(MetadataFile),
    /// Any other kind: admitted nowhere but as a `.txt` note
    /// (`is_txt_note`), which no package reads.
    Other,
}

/// The three native model formats, by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModelFormat {
    /// `.fmdl` (Fox engine).
    Fmdl,
    /// `.model` (pre-Fox engine).
    PesModel,
    /// `.glb` or `.gltf`.
    Gltf,
}

/// A marker file's kind: the directive or fact it declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Marker {
    /// The in-game face marker's old names, `ingame_face` and its tolerated
    /// `ingame_face.txt`, recognized only to be reported
    /// (`ingame_face_marker_renamed`): the in-game face is `ingame_face.toml`.
    OldIngameFace,
    /// `fpc_on`, or a bare `fpc`: apply the FPC preset.
    FpcOn,
    /// `fpc_off`: un-apply the FPC preset.
    FpcOff,
    /// `pre-fox`: the kit's textures are drawn for the pre-Fox layout.
    PreFox,
    /// `fox`: the kit's textures are drawn for the Fox layout.
    Fox,
    /// `icon_<N>`: the kit's menu icon, numbered by the digits after the
    /// underscore (`icon_7`). Any digits make the marker, so an out-of-range
    /// `icon_99` is still a kit's icon, reported as invalid by validation.
    Icon,
    /// `write_names`: the export's players' names are the aesthetics
    /// patch's to write (an autopilot team's, over its tactical export's).
    WriteNames,
}

/// A metadata file by its known name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetadataFile {
    /// `players.txt`: the authoritative roster.
    PlayersTxt,
    /// `refs.txt`: the legacy referee-roster alias.
    RefsTxt,
    /// `ref_lists.txt`: the referee lists file.
    RefLists,
    /// `notes.txt`: the export's free-text note.
    NotesTxt,
    /// `colors.txt`: team or kit menu colors.
    ColorsTxt,
    /// `config.toml`: the authored kit config.
    ConfigToml,
    /// `settings.toml`: savefile settings for a player.
    SettingsToml,
    /// `ingame_face.toml`: the in-game face's settings; its presence marks
    /// the folder as wearing the in-game face.
    IngameFaceToml,
    /// `readme.txt`: free text for humans.
    Readme,
    /// `legacy_ids.toml`: the old export's boots and gloves IDs, which the
    /// Export upgrader writes for the Team compiler's `legacy_boots_ids`
    /// setting; a team export's root file only.
    LegacyIdsToml,
}

/// The shared folder a link file references: `Crocs.boots` → `Boots/Crocs/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SharedKind {
    /// The `.face` link kind (`Faces/<name>/`).
    Face,
    /// The `.boots` link kind (`Boots/<name>/`).
    Boots,
    /// The `.gloves` link kind (`Gloves/<name>/`).
    Gloves,
}

/// `<stem>.boots` / `.face` / `.gloves`: the extension gives the kind.
const SHARED_LINKS: [(&str, SharedKind); 3] = [
    (".boots", SharedKind::Boots),
    (".face", SharedKind::Face),
    (".gloves", SharedKind::Gloves),
];

/// Marker names with a fixed spelling: `pre-fox` before `fox` only because
/// they read as a pair. A bare `fpc` reads as `fpc_on` (presence means on).
/// The icon marker carries a number and is matched by `icon_digits`.
const MARKERS: [(&str, Marker); 7] = [
    ("ingame_face", Marker::OldIngameFace),
    ("fpc_on", Marker::FpcOn),
    ("fpc_off", Marker::FpcOff),
    ("fpc", Marker::FpcOn),
    ("pre-fox", Marker::PreFox),
    ("fox", Marker::Fox),
    ("write_names", Marker::WriteNames),
];

/// Known metadata names, whole-name case-insensitive.
const METADATA: [(&str, MetadataFile); 10] = [
    ("players.txt", MetadataFile::PlayersTxt),
    ("refs.txt", MetadataFile::RefsTxt),
    ("ref_lists.txt", MetadataFile::RefLists),
    ("notes.txt", MetadataFile::NotesTxt),
    ("colors.txt", MetadataFile::ColorsTxt),
    ("config.toml", MetadataFile::ConfigToml),
    ("settings.toml", MetadataFile::SettingsToml),
    ("ingame_face.toml", MetadataFile::IngameFaceToml),
    ("readme.txt", MetadataFile::Readme),
    ("legacy_ids.toml", MetadataFile::LegacyIdsToml),
];

/// Extensions, last `.`-separated segment: each with its `FileKind`.
const EXTENSIONS: [(&str, FileKind); 19] = [
    ("fmdl", FileKind::Model(ModelFormat::Fmdl)),
    ("model", FileKind::Model(ModelFormat::PesModel)),
    ("glb", FileKind::Model(ModelFormat::Gltf)),
    ("gltf", FileKind::Model(ModelFormat::Gltf)),
    ("dds", FileKind::Texture),
    ("ftex", FileKind::Texture),
    ("png", FileKind::Texture),
    ("jpg", FileKind::Texture),
    ("jpeg", FileKind::Texture),
    ("bmp", FileKind::Texture),
    ("webp", FileKind::Texture),
    ("tga", FileKind::Texture),
    ("tif", FileKind::Texture),
    ("tiff", FileKind::Texture),
    ("skl", FileKind::Skl),
    ("fclo", FileKind::Fclo),
    ("xml", FileKind::Xml),
    ("mtl", FileKind::Mtl),
    ("bin", FileKind::Bin),
];

/// Whether `name` is a root `logo*` texture name: `logo`-prefixed (ASCII-
/// case-insensitively) and an accepted image extension.
pub(crate) fn is_logo_texture(name: &str) -> bool {
    name.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("logo"))
        && classify(name) == FileKind::Texture
}

/// A shared-folder link's (kind, target name): `Crocs.boots` and the tolerated
/// `Crocs.boots.txt` alike give `(Boots, "Crocs")`.
pub(crate) fn shared_link_name(name: &str) -> Option<(SharedKind, String)> {
    let name = strip_suffix_ci(name, ".txt").unwrap_or(name);
    for (suffix, kind) in SHARED_LINKS {
        if let Some(stem) = strip_suffix_ci(name, suffix)
            && !stem.is_empty()
        {
            return Some((kind, stem.to_owned()));
        }
    }
    None
}

/// A `.common` link's target name: `torso.fmdl.common` and the tolerated
/// `torso.fmdl.common.txt` alike give `torso.fmdl`.
pub fn common_link_name(name: &str) -> Option<String> {
    let name = strip_suffix_ci(name, ".txt").unwrap_or(name);
    strip_suffix_ci(name, ".common")
        .filter(|stem| !stem.is_empty())
        .map(str::to_owned)
}

/// The `Common/` path a `.common` link at `link` stands for, below `Common/`: the link's
/// path below `folder` (the player folder holding it) with `.common` (and a tolerated
/// `.txt`) taken off its name (`jessie/body.fmdl.common` → `jessie/body.fmdl`; a link
/// directly in `folder` gives its name alone). `None` when `link` is not below `folder` or
/// its name is no `.common` link (`spec/model_format.md` "Link files": a link mirrors `Common/`'s
/// tree).
pub fn common_link_target(link: &ScopePath, folder: &ScopePath) -> Option<String> {
    // Folding is per segment, so the folded folder is a prefix of the folded link exactly
    // when the folder's segments are the link's first ones.
    let in_folder = link
        .fold_key()
        .strip_prefix(&folder.fold_key())
        .is_some_and(|rest| rest.starts_with('/'));
    if !in_folder {
        return None;
    }
    let mut below: Vec<String> = link
        .segments()
        .skip(folder.segments().count())
        .map(str::to_owned)
        .collect();
    let name = below.pop()?;
    below.push(common_link_name(&name)?);
    Some(below.join("/"))
}

impl SharedKind {
    /// The lowercase kind name (`shared_link_duplicate`'s context value).
    pub(crate) fn name(self) -> &'static str {
        match self {
            SharedKind::Face => "face",
            SharedKind::Boots => "boots",
            SharedKind::Gloves => "gloves",
        }
    }
}

/// `name` minus `prefix`, when the head equals it ASCII-case-insensitively.
pub(crate) fn strip_prefix_ci<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    if name.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
        Some(&name[prefix.len()..])
    } else {
        None
    }
}

/// `name` minus `suffix`, when the tail equals it ASCII-case-insensitively.
fn strip_suffix_ci<'a>(name: &'a str, suffix: &str) -> Option<&'a str> {
    let cut = name.len().checked_sub(suffix.len())?;
    // `get` is None off a char boundary: a non-ASCII tail is no ASCII suffix.
    if name.get(cut..)?.eq_ignore_ascii_case(suffix) {
        Some(&name[..cut])
    } else {
        None
    }
}

/// `.common` links, shared-folder links, markers — the groups the `.txt`
/// tolerance applies to.
fn link_or_marker(name: &str) -> Option<FileKind> {
    if let Some(stem) = strip_suffix_ci(name, ".common")
        && !stem.is_empty()
    {
        return Some(FileKind::CommonLink);
    }
    for (suffix, kind) in SHARED_LINKS {
        if let Some(stem) = strip_suffix_ci(name, suffix)
            && !stem.is_empty()
        {
            return Some(FileKind::SharedLink(kind));
        }
    }
    for (marker_name, marker) in MARKERS {
        if name.eq_ignore_ascii_case(marker_name) {
            return Some(FileKind::Marker(marker));
        }
    }
    if icon_digits(name).is_some() {
        return Some(FileKind::Marker(Marker::Icon));
    }
    None
}

/// An icon marker's digits (`icon_07` gives `07`): `icon_`, ASCII-case-
/// insensitively, then one or more ASCII digits and nothing else.
fn icon_digits(name: &str) -> Option<&str> {
    strip_prefix_ci(name, "icon_")
        .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

/// An icon marker's number, read from its file name (`icon_07` and the
/// tolerated `icon_07.txt` alike give 7); `None` when the name is no icon
/// marker or its number does not fit a `u8` (wide enough: the game's icons
/// are numbered 0–23).
pub(crate) fn icon_number(file_name: &str) -> Option<u8> {
    let name = strip_suffix_ci(file_name, ".txt").unwrap_or(file_name);
    icon_digits(name)?.parse().ok()
}

/// Whether `name`, a file's name, is a `.txt` note: model content in the
/// allowlist, which no package reads (`file_not_used`). The name's stem
/// without `.txt` must classify as `Other`, no kind Studio knows, so
/// `notes.txt`, `readme.txt`, `players.txt` and `colors.txt` (stems `notes`,
/// `readme`, `players`, `colors`) count, while a link's or a marker's
/// tolerated `.txt` spelling (`Crocs.boots.txt`) and a real file's `.txt`
/// spelling (`settings.toml.txt`, `skin.dds.txt`, `face_high.fmdl.txt`), which
/// stays disallowed, do not.
/// ASCII-case-insensitive, as `classify` matches names.
pub fn is_txt_note(name: &str) -> bool {
    let Some(stem) = strip_suffix_ci(name, ".txt") else {
        return false;
    };
    classify(stem) == FileKind::Other
}

/// One file name's `FileKind`, ASCII-case-insensitive throughout.
pub fn classify(file_name: &str) -> FileKind {
    // The `.txt` tolerance: only when the name without it is a link or marker.
    if let Some(stripped) = strip_suffix_ci(file_name, ".txt")
        && let Some(kind) = link_or_marker(stripped)
    {
        return kind;
    }
    if let Some(kind) = link_or_marker(file_name) {
        return kind;
    }
    for (metadata_name, metadata) in METADATA {
        if file_name.eq_ignore_ascii_case(metadata_name) {
            return FileKind::Metadata(metadata);
        }
    }
    if file_name.eq_ignore_ascii_case("materials.toml")
        || strip_suffix_ci(file_name, ".materials.toml").is_some()
    {
        return FileKind::MaterialsToml;
    }
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or("");
    for (known, kind) in EXTENSIONS {
        if extension.eq_ignore_ascii_case(known) {
            return kind;
        }
    }
    FileKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_covers_the_convention_table() {
        let cases: [(&str, FileKind); 62] = [
            ("Crocs.boots", FileKind::SharedLink(SharedKind::Boots)),
            ("Crocs.BOOTS.txt", FileKind::SharedLink(SharedKind::Boots)),
            ("Longhair.face", FileKind::SharedLink(SharedKind::Face)),
            (
                "Keeper gloves.gloves",
                FileKind::SharedLink(SharedKind::Gloves),
            ),
            (".boots", FileKind::Other),
            ("torso.fmdl.common", FileKind::CommonLink),
            ("torso.fmdl.common.txt", FileKind::CommonLink),
            ("hair.png.common", FileKind::CommonLink),
            ("ボ.common.txt", FileKind::CommonLink),
            (".common", FileKind::Other),
            (".common.txt", FileKind::Other),
            ("ingame_face", FileKind::Marker(Marker::OldIngameFace)),
            ("INGAME_FACE.txt", FileKind::Marker(Marker::OldIngameFace)),
            ("fpc_on", FileKind::Marker(Marker::FpcOn)),
            ("fpc_off.txt", FileKind::Marker(Marker::FpcOff)),
            ("pre-fox", FileKind::Marker(Marker::PreFox)),
            ("Fox.txt", FileKind::Marker(Marker::Fox)),
            ("WRITE_NAMES.txt", FileKind::Marker(Marker::WriteNames)),
            ("players.txt", FileKind::Metadata(MetadataFile::PlayersTxt)),
            ("README.TXT", FileKind::Metadata(MetadataFile::Readme)),
            ("ref_lists.txt", FileKind::Metadata(MetadataFile::RefLists)),
            ("refs.txt", FileKind::Metadata(MetadataFile::RefsTxt)),
            ("notes.txt", FileKind::Metadata(MetadataFile::NotesTxt)),
            ("colors.txt", FileKind::Metadata(MetadataFile::ColorsTxt)),
            ("config.toml", FileKind::Metadata(MetadataFile::ConfigToml)),
            (
                "settings.toml",
                FileKind::Metadata(MetadataFile::SettingsToml),
            ),
            (
                "ingame_face.toml",
                FileKind::Metadata(MetadataFile::IngameFaceToml),
            ),
            (
                "legacy_ids.toml",
                FileKind::Metadata(MetadataFile::LegacyIdsToml),
            ),
            ("ingame_face.toml.txt", FileKind::Other),
            ("materials.toml", FileKind::MaterialsToml),
            ("Materials.toml", FileKind::MaterialsToml),
            ("body.materials.toml", FileKind::MaterialsToml),
            ("face_high.fmdl", FileKind::Model(ModelFormat::Fmdl)),
            ("boots.MODEL", FileKind::Model(ModelFormat::PesModel)),
            ("x.glb", FileKind::Model(ModelFormat::Gltf)),
            ("x.gltf", FileKind::Model(ModelFormat::Gltf)),
            ("x.dds", FileKind::Texture),
            ("x.ftex", FileKind::Texture),
            ("x.png", FileKind::Texture),
            ("x.jpg", FileKind::Texture),
            ("x.jpeg", FileKind::Texture),
            ("x.bmp", FileKind::Texture),
            ("x.webp", FileKind::Texture),
            ("x.tga", FileKind::Texture),
            ("x.tif", FileKind::Texture),
            ("x.tiff", FileKind::Texture),
            ("x.skl", FileKind::Skl),
            ("x.fclo", FileKind::Fclo),
            ("face_diff.bin", FileKind::Bin),
            ("x.xml", FileKind::Xml),
            ("x.mtl", FileKind::Mtl),
            ("foo.txt", FileKind::Other),
            ("x.exe", FileKind::Other),
            ("boots.txt", FileKind::Other),
            ("notes.txt.txt", FileKind::Other),
            ("settings.toml.txt", FileKind::Other),
            ("config.toml.txt", FileKind::Other),
            ("x.bat", FileKind::Other),
            ("no_extension", FileKind::Other),
            ("ボボ", FileKind::Other),
            ("スパイク.boots", FileKind::SharedLink(SharedKind::Boots)),
            ("髪.png", FileKind::Texture),
        ];
        for (name, expected) in cases {
            assert_eq!(classify(name), expected, "{name:?}");
        }
    }

    #[test]
    fn a_common_link_stands_for_the_common_path_at_its_own_path_below_its_folder() {
        let path = |text: &str| ScopePath::new(text).unwrap();
        let folder = path("Players/05 - A");
        let target = |link: &str| common_link_target(&path(link), &folder);
        assert_eq!(
            target("Players/05 - A/jessie/body.fmdl.common").as_deref(),
            Some("jessie/body.fmdl")
        );
        assert_eq!(
            target("Players/05 - A/a/B/legs.model.common.txt").as_deref(),
            Some("a/B/legs.model")
        );
        // A link directly in the folder names a direct file, by its name alone; the folder's
        // spelling is folded.
        assert_eq!(
            target("players/05 - a/body.fmdl.common").as_deref(),
            Some("body.fmdl")
        );
        // Not a link, or not below the folder.
        assert_eq!(target("Players/05 - A/jessie/body.fmdl"), None);
        assert_eq!(target("Players/05 - AB/body.fmdl.common"), None);
        assert_eq!(target("Players/06 - B/body.fmdl.common"), None);
    }

    #[test]
    fn a_marker_value_follows_an_underscore() {
        let markers = [
            ("fpc_on", Marker::FpcOn),
            ("FPC_OFF.txt", Marker::FpcOff),
            ("fpc", Marker::FpcOn),
            ("fpc.txt", Marker::FpcOn),
            ("icon_7", Marker::Icon),
            ("ICON_07.txt", Marker::Icon),
            ("icon_99", Marker::Icon),
        ];
        for (name, marker) in markers {
            assert_eq!(classify(name), FileKind::Marker(marker), "{name:?}");
        }
        for name in [
            "fpc.on", "fpc.off", "icon.txt", "icon_", "icon_x", "icon_7b",
        ] {
            assert_eq!(classify(name), FileKind::Other, "{name:?}");
        }
    }
}
