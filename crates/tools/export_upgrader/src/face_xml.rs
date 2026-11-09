//! A member `face.xml`'s kit magic respelled, and its WESYS wrapper removed.
//! `path` and `material` attributes whose values name the player folder's
//! own or the team's Common get the kit magic respelled. Everything else,
//! the `<dif>` included, is byte for byte as written. A wrapped one is
//! written unwrapped, whether a rewrite touched it or not: the compiler
//! reads a member's `face.xml` as plain XML and discards a wrapped one's
//! folder. The flattening of a referee layout reads the model names its
//! `path` values give (`named_paths`).

use std::borrow::Cow;
use std::collections::BTreeSet;

use aesthetics_export::common_link_name;
use roxmltree::Document;

use crate::kit_magic::{Spelling, respell};
use crate::source::SourceFailure;
use crate::upgrade::Identity;

/// The `model/character/uniform/common/` prefix the team's `Common/`
/// stands in, as `materials.rs` spells it.
const COMMON_PREFIX: &str = "model/character/uniform/common/";

/// Whether `output` is a member `face.xml`: `Players/<folder>/face.xml`
/// directly in the player folder (a subfolder's is
/// `file_type_disallowed`, never a member).
fn is_member_face_xml(output: &str) -> bool {
    let Some(rest) = output.strip_prefix("Players/") else {
        return false;
    };
    rest.rsplit_once('/').is_some_and(|(_, name)| {
        name.eq_ignore_ascii_case("face.xml") && !rest[..rest.len() - name.len() - 1].contains('/')
    })
}

/// The folded names of the `Players/<folder>/` folders holding a member
/// `face.xml`, whose files are left entirely as named (step 7's model
/// renames do not reach them, subfolders included).
pub(crate) fn xml_folders(copies: &[(String, String)]) -> BTreeSet<String> {
    copies
        .iter()
        .filter(|(_, output)| is_member_face_xml(output))
        .filter_map(|(_, output)| {
            output["Players/".len()..]
                .split_once('/')
                .map(|(folder, _)| vtree::fold_name(folder))
        })
        .collect()
}

/// `input`'s `face.xml` read and unwrapped, as both passes over it read it
/// (`face_xml_rewrites`, `kept_common_files`): whether it was WESYS-wrapped
/// (a rewrite is written unwrapped) and the payload; `None` when the read
/// or the unwrap fails — such a file is left as written and names nothing.
fn read_face_xml(
    input: &str,
    read: &mut impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> Option<(bool, Vec<u8>)> {
    let bytes = read(input).ok()?;
    let payload = wezlib::decompress_if_wrapped(&bytes).ok()?.into_owned();
    Some((wezlib::is_wrapped(&bytes), payload))
}

/// `payload`'s XML text past a BOM, and whether a BOM stood before it;
/// `None` when `payload` is not UTF-8.
fn xml_text(payload: &[u8]) -> Option<(bool, &str)> {
    let text = std::str::from_utf8(payload).ok()?;
    Some(match text.strip_prefix('\u{feff}') {
        Some(text) => (true, text),
        None => (false, text),
    })
}

/// The member `face.xml` files to write instead of copying: output path
/// and contents. One is here when its input is WESYS-wrapped (written
/// unwrapped, step 1) or a `path`/`material` value changed.
pub(crate) fn face_xml_rewrites(
    copies: &[(String, String)],
    identity: &Identity,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> Vec<(String, Vec<u8>)> {
    let mut rewrites = Vec::new();
    for (input, output) in copies {
        if !is_member_face_xml(output) {
            continue;
        }
        // The copy loop's own read reports a failure; a file that does not
        // unwrap is copied as it is (the compile reports it).
        let Some((wrapped, payload)) = read_face_xml(input, &mut read) else {
            continue;
        };
        // The flattened-value rewrite is a referee's alone: a team's
        // `face.xml` needs no folder listing.
        let folder_files = match identity {
            Identity::Referees => folder_files(copies, output),
            Identity::Team { .. } | Identity::ModelFolders => BTreeSet::new(),
        };
        let respelled = xml_text(&payload).and_then(|(bom, text)| {
            match respell_face_xml(text, identity, &folder_files) {
                Cow::Owned(respelled) => {
                    let mut out = String::new();
                    if bom {
                        out.push('\u{feff}');
                    }
                    out.push_str(&respelled);
                    Some(out.into_bytes())
                }
                Cow::Borrowed(_) => None,
            }
        });
        let new = match respelled {
            Some(new) => new,
            // Unchanged, or not UTF-8: a wrapped file still lands
            // unwrapped, untouched.
            None if wrapped => payload,
            None => continue,
        };
        rewrites.push((output.clone(), new));
    }
    rewrites
}

/// The folded paths below its player folder of every copy in the member
/// `face.xml` `output`'s folder (`Players/arbiter/kit/kit1.mtl` →
/// `kit/kit1.mtl`), the files a referee's Common value may name there.
fn folder_files(copies: &[(String, String)], output: &str) -> BTreeSet<String> {
    let Some((folder, _)) = output
        .strip_prefix("Players/")
        .and_then(|rest| rest.split_once('/'))
    else {
        return BTreeSet::new();
    };
    let folder = vtree::fold_name(folder);
    copies
        .iter()
        .filter_map(|(_, output)| output.strip_prefix("Players/")?.split_once('/'))
        .filter(|(copy_folder, _)| vtree::fold_name(copy_folder) == folder)
        .map(|(_, below)| vtree::fold_name(below))
        .collect()
}

/// A referee's Common `path`/`material` value whose file the flatten moved
/// into his folder (`common_below` one of `folder_files`), spelled below
/// the folder: `model/character/uniform/common/XXX/kit/x.mtl` →
/// `./kit/x.mtl`, the rest as written (`spec/export_upgrader.md` step 12).
/// `None` for any other value, which the team rule's kit magic alone
/// respells; a `./` value is never changed here, a missing one being the
/// compile's `xml_model_not_found` (no link resolves a `path`).
fn flattened_common_value(raw: &str, folder_files: &BTreeSet<String>) -> Option<String> {
    let below = common_below(raw)?;
    if !folder_files.contains(&below) {
        return None;
    }
    let (_segment, rest) = strip_prefix_ci(raw, COMMON_PREFIX)?.split_once('/')?;
    Some(format!("./{rest}"))
}

/// `value` minus `prefix` when it starts with it (ASCII
/// case-insensitive).
fn strip_prefix_ci<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

/// `value`, a `path`/`material` value under `COMMON_PREFIX`, respelled to
/// the Studio spelling of the team's Common: the prefix and whatever
/// folder stood after it (`XXX`, a team ID, a team's name) become
/// `team_common_folder/`, the rest of the value kept. A value without a
/// segment after `common/` is borrowed, as is an empty one.
fn respell_common_segment(value: &str) -> Cow<'_, str> {
    let Some(rest) = strip_prefix_ci(value, COMMON_PREFIX) else {
        return Cow::Borrowed(value);
    };
    let Some((segment, path)) = rest.split_once('/') else {
        return Cow::Borrowed(value);
    };
    if segment.is_empty() {
        return Cow::Borrowed(value);
    }
    Cow::Owned(format!("team_common_folder/{path}"))
}

/// The `Common/` path a `face.xml` `path`/`material` value names, as the
/// compiler reads it (`model/character/uniform/common/<segment>/<path>`,
/// the file name's `*` read as `win32`, directories as written), respelled
/// as the `Common/` outputs are (`Spelling::Name`) and folded; `None`
/// when it names none (a value naming a folder gives a path no file has,
/// which keeps nothing).
fn common_below(value: &str) -> Option<String> {
    let rest = strip_prefix_ci(value, COMMON_PREFIX)?;
    let (_segment, path) = rest.split_once('/')?;
    let path = match path.rsplit_once('/') {
        Some((dir, name)) => format!("{dir}/{}", name.replace('*', "win32")),
        None => path.replace('*', "win32"),
    };
    Some(vtree::fold_name(&respell(&path, Spelling::Name)))
}

/// The `Common/` paths (folded, below `Common/`) that keep their names
/// through the model renames: those a member `face.xml`'s `path`/`material`
/// values name, and those a `.common` link in such a folder names. The link
/// is left as written with its folder, so its target keeps its name too.
pub(crate) fn kept_common_files(
    copies: &[(String, String)],
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> BTreeSet<String> {
    let xml = xml_folders(copies);
    let mut kept = BTreeSet::new();
    for (input, output) in copies {
        if !is_member_face_xml(output) {
            continue;
        }
        let Some((_, payload)) = read_face_xml(input, &mut read) else {
            continue;
        };
        let Some((_, text)) = xml_text(&payload) else {
            continue;
        };
        let Ok(document) = Document::parse(text) else {
            continue;
        };
        for attribute in document
            .descendants()
            .filter(|node| node.is_element())
            .flat_map(|element| element.attributes())
        {
            if matches!(attribute.name(), "path" | "material")
                && let Some(below) = common_below(&text[attribute.range_value()])
            {
                kept.insert(below);
            }
        }
    }
    for (_, output) in copies {
        let Some(rest) = output.strip_prefix("Players/") else {
            continue;
        };
        let Some((folder, below)) = rest.split_once('/') else {
            continue;
        };
        if !xml.contains(&vtree::fold_name(folder)) {
            continue;
        }
        let name = below.rsplit('/').next().unwrap_or(below);
        let Some(target) = common_link_name(name) else {
            continue;
        };
        let suffix_len = name.len() - target.len();
        kept.insert(vtree::fold_name(&below[..below.len() - suffix_len]));
    }
    kept
}

/// The folded file names the `path` attributes of the `face.xml` at `input`
/// name (`./oral_parts_body_*.model` → `oral_parts_body_win32.model`,
/// `model/character/uniform/common/XXX/x.model` → `x.model`: the last `/`
/// segment, its `*` read as `win32`, as the compiler reads it); empty when
/// the file does not read, unwrap or parse.
pub(crate) fn named_paths(
    input: &str,
    read: &mut impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    let Some((_, payload)) = read_face_xml(input, read) else {
        return named;
    };
    let Some((_, text)) = xml_text(&payload) else {
        return named;
    };
    let Ok(document) = Document::parse(text) else {
        return named;
    };
    for attribute in document
        .descendants()
        .filter(|node| node.is_element())
        .flat_map(|element| element.attributes())
        .filter(|attribute| attribute.name() == "path")
    {
        let value = &text[attribute.range_value()];
        let name = value.rsplit('/').next().unwrap_or(value);
        named.insert(vtree::fold_name(&name.replace('*', "win32")));
    }
    named
}

/// `text`, a parseable `face.xml`, with the `path` and `material`
/// attribute values that name the player folder's own (`./`) or the
/// team's Common (`model/character/uniform/common/`, any letter case) kit
/// magic respelled — a Common value respelled `team_common_folder/` first
/// (`respell_common_segment`) — a
/// referee's Common value naming one of `folder_files` first spelled
/// below his folder (`flattened_common_value`); `Cow::Borrowed` when
/// nothing changed.
fn respell_face_xml<'a>(
    text: &'a str,
    identity: &Identity,
    folder_files: &BTreeSet<String>,
) -> Cow<'a, str> {
    let Ok(document) = Document::parse(text) else {
        return Cow::Borrowed(text);
    };
    let edits: Vec<(std::ops::Range<usize>, String)> = document
        .descendants()
        .filter(|node| node.is_element())
        .flat_map(|element| element.attributes())
        .filter_map(|attribute| {
            if !matches!(attribute.name(), "path" | "material") {
                return None;
            }
            let raw = &text[attribute.range_value()];
            let flattened = match identity {
                Identity::Referees => flattened_common_value(raw, folder_files),
                Identity::Team { .. } | Identity::ModelFolders => None,
            };
            let value = flattened.as_deref().unwrap_or(raw);
            if !(value.starts_with("./") || strip_prefix_ci(value, COMMON_PREFIX).is_some()) {
                return None;
            }
            // The folder after `common/` becomes `team_folder`, whatever
            // it was, then the kit magic as today; a flattened value's
            // `./` has no folder left.
            let respelled = respell_common_segment(value);
            let new = respell(&respelled, Spelling::Path);
            (new != raw).then(|| (attribute.range_value(), new.into_owned()))
        })
        .collect();
    if edits.is_empty() {
        return Cow::Borrowed(text);
    }
    let mut out = text.to_owned();
    // The ranges name the original text: last first, so an earlier splice
    // does not shift the ones after it.
    for (range, new) in edits.into_iter().rev() {
        out.replace_range(range, &new);
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use teams_list::TeamName;

    use super::*;

    fn team() -> Identity {
        Identity::Team {
            name: TeamName::new("co").unwrap(),
        }
    }

    /// A `face.xml` in the real one's shape: an `<?xml` declaration, a
    /// `<config>` of multi-line `<model>` elements, comments, a `<dif>`
    /// block whose text mentions a kit token.
    fn face_xml_text() -> String {
        [
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>",
            "<config>",
            "    <!-- The player's jacket: u0XXXp0/jacket_win32.model -->",
            "    <model level=\"0\" type=\"parts\" path=\"./u0XXXp0/jacket_win32.model\"",
            "                                      material=\"./face.mtl\"/>",
            "    <dif>u0XXXp1</dif>",
            "</config>",
        ]
        .join("\n")
    }

    fn reader(
        entries: Vec<(String, Vec<u8>)>,
    ) -> impl FnMut(&str) -> Result<Vec<u8>, SourceFailure> {
        move |path| {
            entries
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, bytes)| bytes.clone())
                .ok_or_else(|| SourceFailure {
                    path: path.to_owned(),
                    error: "not listed".to_owned(),
                })
        }
    }

    #[test]
    fn a_wrapped_face_xml_is_written_unwrapped_with_path_respelled() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let wrapped = wezlib::compress(face_xml_text().as_bytes());
        let mut read = reader(vec![("Faces/x".to_owned(), wrapped)]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        let (_, bytes) = &rewrites[0];
        assert!(!wezlib::is_wrapped(bytes));
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(
            text.contains("path=\"./kitN/jacket_win32.model\""),
            "{text}"
        );
        assert!(
            text.contains("<!-- The player's jacket: u0XXXp0/jacket_win32.model -->"),
            "{text}"
        );
        assert!(text.contains("<dif>u0XXXp1</dif>"), "{text}");
        assert!(text.contains("material=\"./face.mtl\""), "{text}");
        assert!(
            text.contains("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>"),
            "{text}"
        );
    }

    #[test]
    fn a_common_path_respells_team_folder_and_the_keeper_token() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text =
            "<face><model path=\"model/character/uniform/common/XXX/u0XXXg1_shirt.model\"/></face>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        let text = String::from_utf8(rewrites[0].1.clone()).unwrap();
        assert_eq!(
            text,
            "<face><model path=\"team_common_folder/gk_kit1_shirt.model\"/></face>"
        );
    }

    #[test]
    fn a_plain_face_xml_without_tokens_is_not_rewritten() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<face><model path=\"./face.model\"/></face>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert!(rewrites.is_empty());
    }

    #[test]
    fn a_wrapped_face_xml_without_tokens_is_written_unwrapped() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<face><model path=\"./face.model\"/></face>";
        let wrapped = wezlib::compress(text.as_bytes());
        let mut read = reader(vec![("Faces/x".to_owned(), wrapped)]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(rewrites[0].1, text.as_bytes().to_vec());
    }

    #[test]
    fn a_wrapped_non_xml_payload_is_written_as_is() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let wrapped = wezlib::compress(b"face");
        let mut read = reader(vec![("Faces/x".to_owned(), wrapped)]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(rewrites[0].1, b"face".to_vec());
    }

    #[test]
    fn a_model_with_two_respelled_attributes_stays_intact() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<config><model path=\"./jacket_u0XXXp0.model\" material=\"./jacket_u0XXXp0.mtl\"/></config>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(
            String::from_utf8(rewrites[0].1.clone()).unwrap(),
            "<config><model path=\"./jacket_kitN.model\" material=\"./jacket_kitN.mtl\"/></config>"
        );
    }

    #[test]
    fn a_face_xml_in_another_letter_case_is_a_member() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/Face.xml".to_owned())];
        let mut read = reader(vec![("Faces/x".to_owned(), face_xml_text().into_bytes())]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        let text = String::from_utf8(rewrites[0].1.clone()).unwrap();
        assert!(
            text.contains("path=\"./kitN/jacket_win32.model\""),
            "{text}"
        );
    }

    #[test]
    fn a_face_xml_in_a_subfolder_is_not_a_member() {
        let copies = vec![(
            "Faces/x".to_owned(),
            "Players/07 - A/kitF/face.xml".to_owned(),
        )];
        let mut read = reader(vec![("Faces/x".to_owned(), face_xml_text().into_bytes())]);
        assert!(face_xml_rewrites(&copies, &team(), &mut read).is_empty());
    }

    #[test]
    fn kept_common_files_names_the_values_common_paths() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<config><model path=\"model/character/uniform/common/o/oral_hand_l_*.model\" material=\"model/character/uniform/common/XXX/sub/x.mtl\"/></config>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let kept = kept_common_files(&copies, &mut read);
        assert_eq!(
            kept.iter().collect::<Vec<_>>(),
            ["oral_hand_l_win32.model", "sub/x.mtl"]
        );
    }

    #[test]
    fn a_common_path_s_folder_becomes_team_folder_whatever_it_was() {
        // `o`, `XXX`, an ID: the compiler refuses any folder after
        // `common/` but `team_folder`, so whatever stood there is
        // respelled to it.
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<config><model path=\"model/character/uniform/common/o/oral_shirt_*.model\" material=\"model/character/uniform/common/XXX/a.mtl\"/></config>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &team(), &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(
            String::from_utf8(rewrites[0].1.clone()).unwrap(),
            "<config><model path=\"team_common_folder/oral_shirt_*.model\" material=\"team_common_folder/a.mtl\"/></config>"
        );
    }

    #[test]
    fn model_folder_mode_writes_team_folder_too() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<config><model path=\"model/character/uniform/common/o/x.model\"/></config>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &Identity::ModelFolders, &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(
            String::from_utf8(rewrites[0].1.clone()).unwrap(),
            "<config><model path=\"team_common_folder/x.model\"/></config>"
        );
    }

    #[test]
    fn kept_common_files_ignores_local_and_bare_values() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let text = "<config><model path=\"./x.model\" material=\"model/character/uniform/common/\"/><model path=\"model/character/uniform/common/XXX\"/></config>";
        let mut read = reader(vec![("Faces/x".to_owned(), text.as_bytes().to_vec())]);
        assert!(kept_common_files(&copies, &mut read).is_empty());
    }

    #[test]
    fn kept_common_files_counts_a_link_in_the_xml_folder() {
        let copies = vec![
            ("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned()),
            (
                "Faces/x.link".to_owned(),
                "Players/07 - A/oral_y_win32.model.common".to_owned(),
            ),
        ];
        let mut read = reader(vec![("Faces/x".to_owned(), b"<config/>".to_vec())]);
        let kept = kept_common_files(&copies, &mut read);
        assert_eq!(kept.iter().collect::<Vec<_>>(), ["oral_y_win32.model"]);
    }

    #[test]
    fn kept_common_files_reads_a_wrapped_face_xml() {
        let copies = vec![("Faces/x".to_owned(), "Players/07 - A/face.xml".to_owned())];
        let wrapped = wezlib::compress(
            "<config><model path=\"model/character/uniform/common/o/oral_hand_l_*.model\"/></config>"
                .as_bytes(),
        );
        let mut read = reader(vec![("Faces/x".to_owned(), wrapped)]);
        let kept = kept_common_files(&copies, &mut read);
        assert_eq!(kept.iter().collect::<Vec<_>>(), ["oral_hand_l_win32.model"]);
    }

    #[test]
    fn named_paths_are_the_path_values_file_names_folded() {
        let mut read = reader(vec![
            ("x/face.xml".to_owned(), face_xml_text().into_bytes()),
            (
                "y/face.xml".to_owned(),
                b"<config><model path=\"./a_*.model\" material=\"./m.mtl\"/><model path=\"model/character/uniform/common/XXX/sub/B.model\"/></config>".to_vec(),
            ),
        ]);
        assert_eq!(
            named_paths("x/face.xml", &mut read)
                .iter()
                .collect::<Vec<_>>(),
            ["jacket_win32.model"]
        );
        assert_eq!(
            named_paths("y/face.xml", &mut read)
                .iter()
                .collect::<Vec<_>>(),
            ["a_win32.model", "b.model"]
        );
        assert!(named_paths("z/face.xml", &mut read).is_empty());
    }

    /// A referee export's identity.
    fn refs() -> Identity {
        Identity::Referees
    }

    #[test]
    fn a_referee_common_value_naming_a_flattened_file_names_it_in_the_folder() {
        let text = "<config><model path=\"model/character/uniform/common/XXX/parts_body.model\"/><model path=\"./oral_x_*.model\" material=\"model/character/uniform/common/XXX/kit/u0XXXp1.mtl\"/></config>";
        let rewrite = |identity: &Identity, folder: &str| {
            let copies = vec![
                ("x".to_owned(), format!("Players/{folder}/face.xml")),
                ("b".to_owned(), format!("Players/{folder}/parts_body.model")),
                (
                    "o".to_owned(),
                    format!("Players/{folder}/oral_x_win32.model"),
                ),
                ("k".to_owned(), format!("Players/{folder}/kit/kit1.mtl")),
            ];
            let mut read = reader(vec![("x".to_owned(), text.as_bytes().to_vec())]);
            let rewrites = face_xml_rewrites(&copies, identity, &mut read);
            assert_eq!(rewrites.len(), 1);
            String::from_utf8(rewrites[0].1.clone()).unwrap()
        };
        assert_eq!(
            rewrite(&refs(), "arbiter"),
            "<config><model path=\"./parts_body.model\"/><model path=\"./oral_x_*.model\" material=\"./kit/kit1.mtl\"/></config>"
        );
        // A team's: `team_folder`, then the kit magic.
        assert_eq!(
            rewrite(&team(), "07 - A"),
            "<config><model path=\"team_common_folder/parts_body.model\"/><model path=\"./oral_x_*.model\" material=\"team_common_folder/kit/kit1.mtl\"/></config>"
        );
    }

    #[test]
    fn a_referee_common_value_naming_nothing_in_the_folder_keeps_the_team_rule() {
        let copies = vec![
            ("x".to_owned(), "Players/arbiter/face.xml".to_owned()),
            ("e".to_owned(), "Players/eiki/parts_body.model".to_owned()),
        ];
        let text = "<config><model path=\"model/character/uniform/common/XXX/parts_body.model\"/></config>";
        let mut read = reader(vec![("x".to_owned(), text.as_bytes().to_vec())]);
        let rewrites = face_xml_rewrites(&copies, &refs(), &mut read);
        assert_eq!(rewrites.len(), 1);
        assert_eq!(
            String::from_utf8(rewrites[0].1.clone()).unwrap(),
            "<config><model path=\"team_common_folder/parts_body.model\"/></config>"
        );
    }
}
