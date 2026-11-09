//! The legacy kit-dependent magic and its Studio spelling: `u0???pN`
//! (`u0` and three ASCII letters or digits, `p` and a digit) becomes
//! `kit<N>` in a path and `kitF` in a file or folder name, `u0???g`
//! becomes `u0<path ID>g` (the team's, 999 for referees; kept as written
//! in model-folder mode), matched in the letter case written, anywhere
//! and every occurrence. In a path segment
//! holding a `u0???p` token, a Studio kit token already there
//! (`kit1_u0827p1.dds`, which some VGL26 exports write) gets its `kit`
//! written `kit_tex` so the respelled token stays the only one; a `u0???p`
//! token in a directory segment respells the file name's Studio tokens the
//! same (`u0XXXp2/kit1.dds` → `kit2/kit_tex1.dds`), a file name's token
//! winning over every directory segment's. Only the upgrader
//! reads the legacy spelling.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::kit_token_starts;
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, INPUT, UnmigratedReason, tool_message, unmigrated};
use crate::players::path_scope;
use crate::upgrade::Identity;

/// Where a respelled kit token stands: a file or folder name, or a path a
/// reference names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spelling {
    /// `u0???p0` becomes `kitF`: the file a kit without a variant of its
    /// own loads.
    Name,
    /// `u0???p0` becomes `kitN`: the reference to the variant of the kit
    /// picked.
    Path,
}

/// `text` with every legacy kit token respelled: `u0`, three ASCII letters
/// or digits, then `p` and a digit `d` becomes `kit<d>` (`d` = 0: `kitF`
/// or `kitN` by `spelling`); `u0`, three ASCII letters or digits, then `g`
/// becomes `u0<the identity's path ID, three digits>g`, or stays as
/// written when the identity has no path ID (model-folder mode). Matched
/// in the letter case written, anywhere, left to right, every occurrence.
/// In each
/// `/`-separated segment that holds a `p` token, every Studio kit token the
/// segment already holds gets its `kit` written `kit_tex`, so the respelled
/// token is the segment's only one for `find_token` to read
/// (`kit1_u0827p0.dds` → `kit_tex1_kitN.dds`, `u0???g` not triggering it);
/// a `p` token in a directory segment respells the file name's Studio
/// tokens the same way (`u0XXXp2/kit1.dds` → `kit2/kit_tex1.dds`), a file
/// name's token winning over every directory segment's. Borrowed when
/// nothing changed.
pub(crate) fn respell<'a>(text: &'a str, spelling: Spelling, identity: &Identity) -> Cow<'a, str> {
    let bytes = text.as_bytes();
    // The edits, as (start, end, replacement) byte ranges of `text`,
    // disjoint: the legacy respells, and the `kit`→`kit_tex` of every Studio
    // kit token in a segment holding a legacy `p` one, the file name's when
    // a directory segment holds one.
    let mut edits: Vec<(usize, usize, Cow<str>)> = Vec::new();
    let mut last = 0;
    let mut marked = BTreeSet::new();
    // The file-name segment's start: a `p` token in a directory segment
    // marks it too.
    let name_start = text.rfind('/').map_or(0, |slash| slash + 1);
    for at in 0..bytes.len() {
        // A match starting inside the one before it is not taken (the
        // earlier one wins, left to right).
        if at < last {
            continue;
        }
        // `u0` + three ASCII letters or digits + `p`/`g`: the `u` is as
        // written, lowercase.
        if bytes[at] != b'u' || bytes.get(at + 1) != Some(&b'0') {
            continue;
        }
        if bytes
            .get(at + 2..at + 5)
            .is_none_or(|chars| !chars.iter().all(|byte| byte.is_ascii_alphanumeric()))
        {
            continue;
        }
        let (replacement, end, legacy): (Cow<str>, usize, bool) = match bytes.get(at + 5) {
            Some(&b'p') => match bytes.get(at + 6) {
                Some(&b'0') => (
                    Cow::Borrowed(match spelling {
                        Spelling::Name => "kitF",
                        Spelling::Path => "kitN",
                    }),
                    at + 7,
                    true,
                ),
                Some(digit) if digit.is_ascii_digit() => (
                    Cow::Owned(format!("kit{}", char::from(*digit))),
                    at + 7,
                    true,
                ),
                _ => continue,
            },
            Some(&b'g') => match identity.path_id() {
                Some(id) => (Cow::Owned(format!("u0{id:03}g")), at + 6, false),
                // Model-folder mode has no team: the token stays as written.
                None => continue,
            },
            _ => continue,
        };
        if legacy {
            // The `/`-separated segment the token stands in.
            let start = text[..at].rfind('/').map_or(0, |slash| slash + 1);
            let end = text[at..].find('/').map_or(text.len(), |slash| at + slash);
            marked.insert((start, end));
            // The token stands in a directory segment: the file name's
            // Studio tokens are marked as its own segment's are.
            if end <= name_start {
                marked.insert((name_start, text.len()));
            }
        }
        edits.push((at, end, replacement));
        last = end;
    }
    for (start, end) in marked {
        for k in kit_token_starts(&text[start..end]) {
            edits.push((start + k, start + k + "kit".len(), Cow::Borrowed("kit_tex")));
        }
    }
    if edits.is_empty() {
        return Cow::Borrowed(text);
    }
    edits.sort_by_key(|(at, ..)| *at);
    let mut out = String::with_capacity(text.len() + edits.len() * 3);
    let mut cursor = 0;
    for (at, end, replacement) in edits {
        out.push_str(&text[cursor..at]);
        out.push_str(&replacement);
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    Cow::Owned(out)
}

/// `directory` (ending in `/` when it names one) and `file_name`, the two
/// halves of one path an FMDL path-table entry or a `.mtl` sampler stores
/// apart, respelled as `respell` respells their joined path: a `u0???p`
/// token in `directory` marks `file_name`'s Studio tokens as it does a
/// whole path's. Both borrowed when nothing changed.
pub(crate) fn respell_split<'a, 'b>(
    directory: &'a str,
    file_name: &'b str,
    spelling: Spelling,
    identity: &Identity,
) -> (Cow<'a, str>, Cow<'b, str>) {
    let Cow::Owned(respelled) = respell(&format!("{directory}{file_name}"), spelling, identity)
    else {
        return (Cow::Borrowed(directory), Cow::Borrowed(file_name));
    };
    let (directory, file_name) = match respelled.rsplit_once('/') {
        Some((directory, file_name)) => (format!("{directory}/"), file_name.to_owned()),
        // No directory half was given, and the respelled name holds none.
        None => (String::new(), respelled),
    };
    (Cow::Owned(directory), Cow::Owned(file_name))
}

/// The copies with their output names respelled (`Spelling::Name`) below a
/// player folder (`Players/<folder>/`), a shared folder (`Boots/<name>/`,
/// `Gloves/<name>/`) or `Common/`; every other output, and those folders'
/// own names, as they are.
pub(crate) fn respell_outputs(
    copies: Vec<(String, String)>,
    identity: &Identity,
) -> Vec<(String, String)> {
    copies
        .into_iter()
        .map(|(from, to)| {
            let to = match to.split_once('/') {
                Some((top, rest)) => {
                    // The path the magic can reach: `Players`/`Boots`/
                    // `Gloves` keep their folder's own name and respell
                    // below it; `Common/` respells the whole path below.
                    let (keep, below) = match top {
                        "Players" | "Boots" | "Gloves" => match rest.split_once('/') {
                            Some((_folder, below)) => (&to[..to.len() - below.len()], below),
                            None => (&to[..], ""),
                        },
                        "Common" => ("Common/", rest),
                        _ => (&to[..], ""),
                    };
                    match respell(below, Spelling::Name, identity) {
                        Cow::Owned(below) => format!("{keep}{below}"),
                        Cow::Borrowed(_) => to,
                    }
                }
                _ => to,
            };
            (from, to)
        })
        .collect()
}

/// `copies` minus every output whose path, folded with
/// `vtree::fold_name`, another copy's already takes: one copy per folded
/// output path, the kept ones at their own positions in the input list's
/// order. A group whose inputs were one file to the old compile's ID
/// rename (`path_id` `Some`: every input path spells one old-compile
/// name, folded, and the rename changed at least one's) keeps the
/// renamed input ordered last in Windows's name order — the rename ran
/// in listing order, replacing the file under the new name, so the last
/// renamed is what the game loaded — and reports each other input
/// `leftover_dropped`. Any other group keeps its first in list order
/// and reports each later one `unmigrated_content`, `other` naming the
/// kept input's path. Every finding is `File`-scoped `DropFile` at the
/// input path.
pub(crate) fn drop_collisions(
    copies: Vec<(String, String)>,
    path_id: Option<u16>,
) -> (Vec<(String, String)>, Vec<Message>) {
    /// What becomes of a colliding copy.
    #[derive(Clone)]
    enum Dropped {
        /// One file to the old compile's ID rename: a leftover.
        Leftover,
        /// Two candidates for the output path; `other` is the kept
        /// input's path.
        Unmigrated { other: String },
    }
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, (_, to)) in copies.iter().enumerate() {
        groups.entry(vtree::fold_name(to)).or_default().push(index);
    }
    let mut dropped: Vec<Option<Dropped>> = vec![None; copies.len()];
    for group in groups.into_values() {
        match renamed_winner(&copies, &group, path_id) {
            Some(winner) => {
                for &index in &group {
                    if index != winner {
                        dropped[index] = Some(Dropped::Leftover);
                    }
                }
            }
            None => {
                for &index in &group[1..] {
                    dropped[index] = Some(Dropped::Unmigrated {
                        other: copies[group[0]].0.clone(),
                    });
                }
            }
        }
    }
    let mut out = Vec::with_capacity(copies.len());
    let mut findings = Vec::new();
    for ((from, to), dropped) in copies.into_iter().zip(dropped) {
        let scope = Scope::File {
            export_id: INPUT,
            path: path_scope(&from),
        };
        match dropped {
            None => out.push((from, to)),
            Some(Dropped::Leftover) => findings.push(tool_message(
                Code::LeftoverDropped,
                scope,
                Disposition::DropFile,
                vec![],
            )),
            Some(Dropped::Unmigrated { other }) => findings.push(unmigrated(
                scope,
                Disposition::DropFile,
                UnmigratedReason::PathTaken,
                Some(other),
                vec![],
            )),
        }
    }
    (out, findings)
}

/// The index of the group's kept copy under the old compile's ID rename:
/// every input path spelling one old-compile name, folded, with the
/// rename changing at least one's, the winner the renamed input ordered
/// last in Windows's name order. `None` for any other group (no path ID,
/// different old-compile names, no input renamed), which the list-order
/// rule keeps.
fn renamed_winner(
    copies: &[(String, String)],
    group: &[usize],
    path_id: Option<u16>,
) -> Option<usize> {
    let id = path_id?;
    let mut names = BTreeSet::new();
    let mut changed = Vec::new();
    for &index in group {
        let old_name = old_compile_name(&copies[index].0, id);
        names.insert(vtree::fold_name(&old_name));
        if old_name != copies[index].0 {
            changed.push(index);
        }
    }
    if names.len() != 1 || changed.is_empty() {
        return None;
    }
    changed
        .into_iter()
        .max_by_key(|index| copies[*index].0.to_ascii_uppercase())
}

/// The old compile's name of `path`: every `u0` + three ASCII letters or
/// digits + `p` or `g` token's three characters replaced by `id`, three
/// digits, matched in the letter case written, left to right,
/// non-overlapping.
fn old_compile_name(path: &str, id: u16) -> String {
    let bytes = path.as_bytes();
    let mut out = String::with_capacity(path.len());
    let mut cursor = 0;
    let mut at = 0;
    while at + 6 <= bytes.len() {
        let token = bytes[at] == b'u'
            && bytes[at + 1] == b'0'
            && bytes[at + 2..at + 5]
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric())
            && matches!(bytes[at + 5], b'p' | b'g');
        if token {
            out.push_str(&path[cursor..at]);
            out.push_str("u0");
            out.push_str(&format!("{id:03}"));
            out.push(char::from(bytes[at + 5]));
            cursor = at + 6;
            at += 6;
        } else {
            at += 1;
        }
    }
    out.push_str(&path[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use teams_list::{TeamId, TeamName};

    use super::*;

    fn team() -> Identity {
        Identity::Team {
            id: TeamId::new(714).unwrap(),
            name: TeamName::new("co").unwrap(),
        }
    }

    #[test]
    fn every_u0_p_token_becomes_kit_d() {
        assert_eq!(respell("u0XXXp1.dds", Spelling::Name, &team()), "kit1.dds");
        assert_eq!(
            respell("skin_u0771p3.dds", Spelling::Name, &team()),
            "skin_kit3.dds"
        );
        // `p0` is `kitF` in a name, `kitN` in a path.
        assert_eq!(respell("u0875p0.dds", Spelling::Name, &team()), "kitF.dds");
        assert_eq!(
            respell("./u0875p0/armor.dds", Spelling::Path, &team()),
            "./kitN/armor.dds"
        );
        assert_eq!(
            respell("u0XXXg1.dds", Spelling::Name, &team()),
            "u0714g1.dds"
        );
        // Every occurrence, anywhere.
        assert_eq!(
            respell("u0XXXp1_u0XXXg1", Spelling::Name, &team()),
            "kit1_u0714g1"
        );
    }

    #[test]
    fn a_non_token_is_borrowed_unchanged() {
        for text in [
            "U0XXXP1.dds",
            "u0X-Xp1.dds",
            "dummy_kit.dds",
            "u0XXXpants.dds",
            "a0XXXp1.dds",
            "uXXXXp1.dds",
            "kit1.dds",
            "portrait.dds",
        ] {
            assert!(
                matches!(respell(text, Spelling::Name, &team()), Cow::Borrowed(_)),
                "{text}"
            );
        }
    }

    #[test]
    fn a_match_starting_inside_a_match_is_not_taken() {
        // The `g` match at 0 ends at 6; the `p` at 7 starts a token at 2 —
        // inside the first match, left to right.
        assert_eq!(respell("u0u0ZgAp1", Spelling::Name, &team()), "u0714gAp1");
    }

    #[test]
    fn a_studio_token_beside_a_legacy_one_becomes_kit_tex() {
        assert_eq!(
            respell("kit1_u0827p0.dds", Spelling::Path, &team()),
            "kit_tex1_kitN.dds"
        );
        assert_eq!(
            respell("kit1_u0827p1.dds", Spelling::Name, &team()),
            "kit_tex1_kit1.dds"
        );
        assert_eq!(
            respell("kitN_u0XXXp2.dds", Spelling::Name, &team()),
            "kit_texN_kit2.dds"
        );
        assert_eq!(
            respell("kitF_u0XXXp2.dds", Spelling::Name, &team()),
            "kit_texF_kit2.dds"
        );
        assert_eq!(
            respell("kit1_kit2_u0XXXp3.dds", Spelling::Name, &team()),
            "kit_tex1_kit_tex2_kit3.dds"
        );
        assert_eq!(
            respell("skit1_u0XXXp1.dds", Spelling::Name, &team()),
            "skit1_kit1.dds"
        );
        // `Kit2` was never a token: its K stays.
        assert_eq!(
            respell("Kit2_u0XXXp1.dds", Spelling::Name, &team()),
            "Kit2_kit1.dds"
        );
        // Only the token's own segment is rewritten; a `g` token does not
        // trigger it.
        assert_eq!(
            respell("kit1/x_u0XXXp1.dds", Spelling::Name, &team()),
            "kit1/x_kit1.dds"
        );
        // A segment after a `/` is read from its first character.
        assert_eq!(
            respell("ltd/kit1_u0XXXp1.dds", Spelling::Name, &team()),
            "ltd/kit_tex1_kit1.dds"
        );
        assert_eq!(
            respell("kit1_u0XXXg1.dds", Spelling::Name, &team()),
            "kit1_u0714g1.dds"
        );
        assert!(matches!(
            respell("kit1_x.dds", Spelling::Name, &team()),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_legacy_token_in_a_directory_segment_marks_the_file_name_s() {
        // A `u0???p` directory respells every Studio kit token the file
        // name below it holds, as it does its own segment's.
        assert_eq!(
            respell("u0XXXp2/kit1.dds", Spelling::Name, &team()),
            "kit2/kit_tex1.dds"
        );
        assert_eq!(
            respell("u0XXXp0/kit1", Spelling::Path, &team()),
            "kitN/kit_tex1"
        );
        assert_eq!(
            respell("u0XXXp0/kit1.dds", Spelling::Name, &team()),
            "kitF/kit_tex1.dds"
        );
        assert_eq!(
            respell("ltd/u0XXXp1/pants_kit3.dds", Spelling::Name, &team()),
            "ltd/kit1/pants_kit_tex3.dds"
        );
        // A `g` token's directory is no `p` token's: the file name is left.
        assert_eq!(
            respell("u0XXXg1/kit1.dds", Spelling::Name, &team()),
            "u0714g1/kit1.dds"
        );
        // The file name's own respelled token is no Studio token of the
        // author's: it is not rewritten.
        assert_eq!(
            respell("u0XXXp1/u0XXXp2.dds", Spelling::Name, &team()),
            "kit1/kit2.dds"
        );
        assert_eq!(
            respell("u0XXXp2/armor_bsm.dds", Spelling::Name, &team()),
            "kit2/armor_bsm.dds"
        );
    }

    #[test]
    fn the_respelled_token_holds_no_token() {
        // `kit_tex` was never a token: the name reads as plain text.
        for stem in ["kit_tex1", "kit_texN", "kit_texF"] {
            assert_eq!(aesthetics_export::kit_token(stem), None, "{stem}");
        }
    }

    #[test]
    fn a_split_reference_respells_as_one_path() {
        let (dir, name) = respell_split("./u0XXXp0/", "kit1", Spelling::Path, &team());
        assert_eq!(dir, "./kitN/");
        assert_eq!(name, "kit_tex1");
        let (dir, name) = respell_split("u0XXXp2/", "kit1.dds", Spelling::Path, &team());
        assert_eq!(dir, "kit2/");
        assert_eq!(name, "kit_tex1.dds");
        // No legacy token in the directory: the file name decides alone.
        let (dir, name) = respell_split("./ltd/", "kit1", Spelling::Path, &team());
        assert_eq!(dir, "./ltd/");
        assert_eq!(name, "kit1");
        // Nothing to respell: both halves are borrowed.
        let (dir, name) = respell_split("./", "armor_bsm", Spelling::Path, &team());
        assert!(matches!(dir, Cow::Borrowed(_)));
        assert!(matches!(name, Cow::Borrowed(_)));
        // An empty directory half is no directory segment.
        let (dir, name) = respell_split("", "u0XXXp1.dds", Spelling::Name, &team());
        assert_eq!(dir, "");
        assert_eq!(name, "kit1.dds");
    }

    #[test]
    fn player_and_common_outputs_respell_their_rest() {
        let copies = respell_outputs(
            vec![
                ("a".into(), "Players/07 - A/u0XXXp1.dds".into()),
                ("b".into(), "Players/07 - A/u0XXXp0/armor.dds".into()),
                ("c".into(), "Common/ltd/u0XXXp5/a.dds".into()),
                ("d".into(), "Common/u0XXXg1.dds".into()),
                ("e".into(), "Boots/Crocs/body_d_u0XXXp0.dds".into()),
                ("f".into(), "Portraits/07.dds".into()),
                ("g".into(), "Kits/p1/kit.dds".into()),
                ("h".into(), "Players/07 - u0XXXp1/x.dds".into()),
            ],
            &team(),
        );
        assert_eq!(
            copies,
            vec![
                ("a".to_owned(), "Players/07 - A/kit1.dds".to_owned()),
                ("b".to_owned(), "Players/07 - A/kitF/armor.dds".to_owned()),
                ("c".to_owned(), "Common/ltd/kit5/a.dds".to_owned()),
                ("d".to_owned(), "Common/u0714g1.dds".to_owned()),
                ("e".to_owned(), "Boots/Crocs/body_d_kitF.dds".to_owned()),
                ("f".to_owned(), "Portraits/07.dds".to_owned()),
                ("g".to_owned(), "Kits/p1/kit.dds".to_owned()),
                ("h".to_owned(), "Players/07 - u0XXXp1/x.dds".to_owned()),
            ]
        );
    }

    #[test]
    fn respelled_names_collide_as_before() {
        let (copies, findings) = drop_collisions(
            respell_outputs(
                vec![
                    ("a".into(), "Players/07 - A/u0XXXp1.dds".into()),
                    ("b".into(), "Players/07 - A/u0714p1.dds".into()),
                ],
                &team(),
            ),
            team().path_id(),
        );
        assert_eq!(
            copies,
            vec![("a".to_owned(), "Players/07 - A/kit1.dds".to_owned())]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("b")
            }
        );
        assert_eq!(findings[0].disposition, Disposition::DropFile);
        assert_eq!(
            findings[0].context,
            [
                ("reason".to_owned(), "path_taken".to_owned()),
                ("other".to_owned(), "a".to_owned()),
            ]
        );
    }

    #[test]
    fn inputs_one_file_to_the_old_rename_keep_the_last_renamed() {
        // The rename replaced the file under its target name in listing
        // order: `u0XXXp2.dds`, ordered after `u0714p2.dds`, is what the
        // game loaded — whichever order the input lists them in.
        for order in [
            [
                "Faces/XXX07 - Ann/u0714p2.dds",
                "Faces/XXX07 - Ann/u0XXXp2.dds",
            ],
            [
                "Faces/XXX07 - Ann/u0XXXp2.dds",
                "Faces/XXX07 - Ann/u0714p2.dds",
            ],
        ] {
            let (copies, findings) = drop_collisions(
                order
                    .iter()
                    .map(|from| ((*from).to_owned(), "Players/07 - Ann/kit2.dds".to_owned()))
                    .collect(),
                team().path_id(),
            );
            assert_eq!(
                copies,
                vec![(
                    "Faces/XXX07 - Ann/u0XXXp2.dds".to_owned(),
                    "Players/07 - Ann/kit2.dds".to_owned()
                )],
                "{order:?}"
            );
            assert_eq!(findings.len(), 1, "{order:?}");
            assert_eq!(findings[0].code.code.as_ref(), "leftover_dropped");
            assert_eq!(
                findings[0].scope,
                Scope::File {
                    export_id: INPUT,
                    path: path_scope("Faces/XXX07 - Ann/u0714p2.dds")
                },
                "{order:?}"
            );
            assert_eq!(findings[0].disposition, Disposition::DropFile);
            assert!(findings[0].context.is_empty());
        }
    }

    #[test]
    fn the_last_renamed_in_windows_name_order_wins() {
        // `X` sorts after `7` folded: `u0XXXp3.dds` was renamed after
        // `u0715p3.dds` and replaced it.
        let (copies, findings) = drop_collisions(
            vec![
                (
                    "Faces/XXX07 - Ann/u0715p3.dds".to_owned(),
                    "Players/07 - Ann/kit3.dds".to_owned(),
                ),
                (
                    "Faces/XXX07 - Ann/u0XXXp3.dds".to_owned(),
                    "Players/07 - Ann/kit3.dds".to_owned(),
                ),
            ],
            team().path_id(),
        );
        assert_eq!(
            copies,
            vec![(
                "Faces/XXX07 - Ann/u0XXXp3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned()
            )]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "leftover_dropped");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Faces/XXX07 - Ann/u0715p3.dds")
            }
        );
    }

    #[test]
    fn inputs_of_different_folders_collide_as_before() {
        // Two folders' inputs are no one file to the old compile: the
        // first in list order is kept, the second unmigrated content.
        let (copies, findings) = drop_collisions(
            vec![
                (
                    "Faces/XXX07 - Ann/u0XXXp1.dds".to_owned(),
                    "Players/07 - Ann/kit1.dds".to_owned(),
                ),
                (
                    "Boots/k0128 - Rex/u0XXXp1.dds".to_owned(),
                    "Players/07 - Ann/kit1.dds".to_owned(),
                ),
            ],
            team().path_id(),
        );
        assert_eq!(
            copies,
            vec![(
                "Faces/XXX07 - Ann/u0XXXp1.dds".to_owned(),
                "Players/07 - Ann/kit1.dds".to_owned()
            )]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Boots/k0128 - Rex/u0XXXp1.dds")
            }
        );
        assert_eq!(
            findings[0].context,
            [
                ("reason".to_owned(), "path_taken".to_owned()),
                (
                    "other".to_owned(),
                    "Faces/XXX07 - Ann/u0XXXp1.dds".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn without_a_path_id_a_collision_keeps_the_first() {
        let (copies, findings) = drop_collisions(
            vec![
                (
                    "Faces/XXX07 - Ann/u0714p2.dds".to_owned(),
                    "Players/07 - Ann/kit2.dds".to_owned(),
                ),
                (
                    "Faces/XXX07 - Ann/u0XXXp2.dds".to_owned(),
                    "Players/07 - Ann/kit2.dds".to_owned(),
                ),
            ],
            None,
        );
        assert_eq!(
            copies,
            vec![(
                "Faces/XXX07 - Ann/u0714p2.dds".to_owned(),
                "Players/07 - Ann/kit2.dds".to_owned()
            )]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Faces/XXX07 - Ann/u0XXXp2.dds")
            }
        );
        assert_eq!(
            findings[0].context,
            [
                ("reason".to_owned(), "path_taken".to_owned()),
                (
                    "other".to_owned(),
                    "Faces/XXX07 - Ann/u0714p2.dds".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn a_referee_export_respells_the_placeholder_999() {
        assert_eq!(
            respell("u0XXXg1.dds", Spelling::Name, &Identity::Referees),
            "u0999g1.dds"
        );
        assert_eq!(
            respell_outputs(
                vec![("a".into(), "Common/u0XXXg1.dds".into())],
                &Identity::Referees
            ),
            vec![("a".to_owned(), "Common/u0999g1.dds".to_owned())]
        );
    }

    #[test]
    fn old_compile_name_gives_each_token_the_team_s_id() {
        // A token not at the start, two tokens, both letters: `u0` + three
        // letters or digits + `p`/`g`, each getting the team's ID.
        assert_eq!(
            old_compile_name("x_u0XXXp1_u0abcg1.dds", 714),
            "x_u0714p1_u0714g1.dds"
        );
        assert_eq!(
            old_compile_name("Faces/XXX07 - Ann/u0731p2.dds", 714),
            "Faces/XXX07 - Ann/u0714p2.dds"
        );
        // A token ending the text.
        assert_eq!(old_compile_name("u0XXXp", 714), "u0714p");
        // The near-misses are no token: not `u`, not `0`, a
        // non-alphanumeric character, no `p`/`g`.
        for unchanged in ["k0abcp1.dds", "uaXXXp1.dds", "u0X-Xp1.dds", "u0XXXq1.dds"] {
            assert_eq!(old_compile_name(unchanged, 714), unchanged);
        }
    }

    #[test]
    fn model_folders_leave_the_goalkeeper_token_as_written() {
        assert!(matches!(
            respell("u0XXXg1.dds", Spelling::Name, &Identity::ModelFolders),
            Cow::Borrowed(_)
        ));
        assert_eq!(
            respell("u0XXXp1_u0XXXg1", Spelling::Path, &Identity::ModelFolders),
            "kit1_u0XXXg1"
        );
    }
}
