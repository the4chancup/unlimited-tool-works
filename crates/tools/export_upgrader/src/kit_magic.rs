//! The legacy kit-dependent magic and its Studio spelling: `u0???pN`
//! (`u0` and three ASCII letters or digits, `p` and a digit) becomes
//! `kit<N>` in a path and `kitF` in a file or folder name, `u0???g` and a
//! digit `D` 1-9 becomes `gk_kitD` and `u0???g0` `gk_kitN` in a path,
//! `gk_kitF` in a file or folder name, as `u0???p` does — the keeper's
//! textures are a kit set read with the outfielders' kit number
//! (`spec/model_format.md`
//! "Kit-dependent assets"). Matched in the letter case written, anywhere
//! and every occurrence, in both modes. In a path segment
//! holding a `u0???p` or `u0???g` token, a Studio kit token already there
//! (`kit1_u0827p1.dds`, which some VGL26 exports write) gets its `kit`
//! written `kit_tex` so the respelled token stays the only one; a legacy
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

/// Where a respelled kit token stands: a file or folder name, or a path a
/// reference names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spelling {
    /// `u0???p0` becomes `kitF`, `u0???g0` `gk_kitF`: the file a kit
    /// without a variant of its own loads.
    Name,
    /// `u0???p0` becomes `kitN`, `u0???g0` `gk_kitN`: the reference to the
    /// variant of the kit picked.
    Path,
}

/// `text` with every legacy kit token respelled: `u0`, three ASCII letters
/// or digits, then `p` and a digit `d` becomes `kit<d>` (`d` = 0: `kitF`
/// or `kitN` by `spelling`); `u0`, three ASCII letters or digits, then `g`
/// and a digit `d` becomes `gk_kit<d>` (`d` = 0: `gk_kitF` or `gk_kitN` by
/// `spelling`); a `u0???g` followed by no digit is no token. Matched
/// in the letter case written, anywhere, left to right, every occurrence.
/// In each
/// `/`-separated segment that holds a legacy token, every Studio kit token
/// the segment already holds gets its `kit` written `kit_tex`, so the
/// respelled token is the segment's only one for `find_token` to read
/// (`kit1_u0827p0.dds` → `kit_tex1_kitN.dds`, `kit1_u0XXXg2.dds` →
/// `kit_tex1_gk_kit2.dds`); a legacy token in a directory segment respells the file name's Studio
/// tokens the same way (`u0XXXp2/kit1.dds` → `kit2/kit_tex1.dds`), a file
/// name's token winning over every directory segment's. Borrowed when
/// nothing changed.
pub(crate) fn respell<'a>(text: &'a str, spelling: Spelling) -> Cow<'a, str> {
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
        // `p` the outfielders' set, `g` the keeper's: both respell to a kit
        // token (`gk_kit1` holds `kit1`), the one kit number picking both.
        let set = match bytes.get(at + 5) {
            Some(&b'p') => "kit",
            Some(&b'g') => "gk_kit",
            _ => continue,
        };
        let number = match (bytes.get(at + 6), spelling) {
            (Some(&b'0'), Spelling::Name) => 'F',
            (Some(&b'0'), Spelling::Path) => 'N',
            (Some(digit), _) if digit.is_ascii_digit() => char::from(*digit),
            // A `u0???p` or `u0???g` followed by no digit is no token.
            _ => continue,
        };
        let end = at + 7;
        // The `/`-separated segment the token stands in: the compiler reads a
        // stem's first kit token, so a Studio one already there is marked.
        let start = text[..at].rfind('/').map_or(0, |slash| slash + 1);
        let segment_end = text[at..].find('/').map_or(text.len(), |slash| at + slash);
        marked.insert((start, segment_end));
        // The token stands in a directory segment: the file name's Studio
        // tokens are marked as its own segment's are.
        if segment_end <= name_start {
            marked.insert((name_start, text.len()));
        }
        edits.push((at, end, Cow::Owned(format!("{set}{number}"))));
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
) -> (Cow<'a, str>, Cow<'b, str>) {
    let Cow::Owned(respelled) = respell(&format!("{directory}{file_name}"), spelling) else {
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
pub(crate) fn respell_outputs(copies: Vec<(String, String)>) -> Vec<(String, String)> {
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
                    match respell(below, Spelling::Name) {
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
/// rename (every input path spelling one old-compile name, folded —
/// their paths differing only in their tokens' three characters — and at
/// least one's three characters not all digits, a copy the rename rewrote
/// whatever ID the team had) keeps the
/// last of those in Windows's name order — the rename ran
/// in listing order, digits before letters, a lettered copy renamed
/// whatever the team, so the last renamed is what the game loaded — and
/// reports each other input
/// `leftover_dropped`. Any other group — different old-compile names, or
/// numbered copies alone, whose shipped pick depended on the team's ID —
/// keeps its first in list order
/// and reports each later one `unmigrated_content`, `other` naming the
/// kept input's path. Every finding is `File`-scoped `DropFile` at the
/// input path.
pub(crate) fn drop_collisions(
    copies: Vec<(String, String)>,
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
        match renamed_winner(&copies, &group) {
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
/// every input path spelling one old-compile name, folded (their paths
/// differing only in their tokens' three characters), and at least one's
/// three characters not all digits — a copy whose three characters are
/// not all digits is renamed whatever the team and lists after every
/// numbered one, so the last of those in Windows's name order is what the
/// game loaded. `None` for any other group (different old-compile names,
/// or numbered copies alone, whose shipped pick depended on the team's
/// ID), which the list-order rule keeps.
fn renamed_winner(copies: &[(String, String)], group: &[usize]) -> Option<usize> {
    let mut names = BTreeSet::new();
    let mut lettered = Vec::new();
    for &index in group {
        names.insert(vtree::fold_name(&old_compile_name(&copies[index].0)));
        if token_markers(&copies[index].0).iter().any(|marker| {
            !copies[index].0.as_bytes()[marker.clone()]
                .iter()
                .all(u8::is_ascii_digit)
        }) {
            lettered.push(index);
        }
    }
    if names.len() != 1 || lettered.is_empty() {
        return None;
    }
    lettered
        .into_iter()
        .max_by_key(|index| copies[*index].0.to_ascii_uppercase())
}

/// The `???` ranges of `path`'s `u0` + three ASCII letters or digits +
/// `p` or `g` tokens, left to right, a match starting inside the one
/// before it not taken.
fn token_markers(path: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = path.as_bytes();
    let mut markers = Vec::new();
    let mut at = 0;
    while at + 6 <= bytes.len() {
        let token = bytes[at] == b'u'
            && bytes[at + 1] == b'0'
            && bytes[at + 2..at + 5]
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric())
            && matches!(bytes[at + 5], b'p' | b'g');
        if token {
            markers.push(at + 2..at + 5);
            at += 6;
        } else {
            at += 1;
        }
    }
    markers
}

/// `path` with every `u0` + three ASCII letters or digits + `p` or `g`
/// token's three characters written `XXX`: the name the collision rule
/// compares — two inputs were one file to the old compile's ID rename
/// when these names fold equal, whatever ID it renamed under.
fn old_compile_name(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut cursor = 0;
    for marker in token_markers(path) {
        out.push_str(&path[cursor..marker.start]);
        out.push_str("XXX");
        cursor = marker.end;
    }
    out.push_str(&path[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_u0_p_token_becomes_kit_d() {
        assert_eq!(respell("u0XXXp1.dds", Spelling::Name), "kit1.dds");
        assert_eq!(respell("skin_u0771p3.dds", Spelling::Name), "skin_kit3.dds");
        // `p0` is `kitF` in a name, `kitN` in a path.
        assert_eq!(respell("u0875p0.dds", Spelling::Name), "kitF.dds");
        assert_eq!(
            respell("./u0875p0/armor.dds", Spelling::Path),
            "./kitN/armor.dds"
        );
        // Every occurrence, anywhere.
        assert_eq!(respell("u0XXXp1_u0XXXp2", Spelling::Name), "kit1_kit2");
    }

    #[test]
    fn the_keeper_token_becomes_gk_kit() {
        assert_eq!(respell("u0XXXg1.dds", Spelling::Name), "gk_kit1.dds");
        assert_eq!(respell("u0714g3.dds", Spelling::Path), "gk_kit3.dds");
        // `g0` is `gk_kitF` in a name, `gk_kitN` in a path — as `p0` is
        // `kitF`/`kitN`.
        assert_eq!(respell("u0XXXg0.dds", Spelling::Name), "gk_kitF.dds");
        assert_eq!(respell("u0XXXg0.dds", Spelling::Path), "gk_kitN.dds");
        // Its `???` are any three characters: a file and a model that
        // spelled them differently match again.
        assert_eq!(
            respell("u0714g1_u0XXXg1", Spelling::Name),
            "gk_kit1_gk_kit1"
        );
        // A `u0???g` followed by no digit is no token.
        for text in ["u0XXXg.dds", "u0XXXgray.dds"] {
            assert!(
                matches!(respell(text, Spelling::Name), Cow::Borrowed(_)),
                "{text}"
            );
        }
        // A `gk_kit` Studio token the author wrote beside a `p` one is
        // marked `kit_tex` like any other.
        assert_eq!(
            respell("u0XXXp1_gk_kit2.dds", Spelling::Name),
            "kit1_gk_kit_tex2.dds"
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
                matches!(respell(text, Spelling::Name), Cow::Borrowed(_)),
                "{text}"
            );
        }
    }

    #[test]
    fn a_match_starting_inside_a_match_is_not_taken() {
        // The `g` token at 0 ends at 7; the token at 2 starts inside it —
        // the earlier one wins, left to right.
        assert_eq!(respell("u0u0Zg1p1", Spelling::Name), "gk_kit1p1");
    }

    #[test]
    fn a_studio_token_beside_a_legacy_one_becomes_kit_tex() {
        assert_eq!(
            respell("kit1_u0827p0.dds", Spelling::Path),
            "kit_tex1_kitN.dds"
        );
        assert_eq!(
            respell("kit1_u0827p1.dds", Spelling::Name),
            "kit_tex1_kit1.dds"
        );
        assert_eq!(
            respell("kitN_u0XXXp2.dds", Spelling::Name),
            "kit_texN_kit2.dds"
        );
        assert_eq!(
            respell("kitF_u0XXXp2.dds", Spelling::Name),
            "kit_texF_kit2.dds"
        );
        assert_eq!(
            respell("kit1_kit2_u0XXXp3.dds", Spelling::Name),
            "kit_tex1_kit_tex2_kit3.dds"
        );
        assert_eq!(
            respell("skit1_u0XXXp1.dds", Spelling::Name),
            "skit1_kit1.dds"
        );
        // `Kit2` was never a token: its K stays.
        assert_eq!(respell("Kit2_u0XXXp1.dds", Spelling::Name), "Kit2_kit1.dds");
        // Only the token's own segment is rewritten.
        assert_eq!(
            respell("kit1/x_u0XXXp1.dds", Spelling::Name),
            "kit1/x_kit1.dds"
        );
        // A segment after a `/` is read from its first character.
        assert_eq!(
            respell("ltd/kit1_u0XXXp1.dds", Spelling::Name),
            "ltd/kit_tex1_kit1.dds"
        );
        // A keeper's token is a kit token too (`gk_kit2` holds `kit2`):
        // the compiler reads a stem's first, so the author's is marked.
        assert_eq!(
            respell("kit1_u0XXXg2.dds", Spelling::Name),
            "kit_tex1_gk_kit2.dds"
        );
        assert!(matches!(
            respell("kit1_x.dds", Spelling::Name),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn a_legacy_token_in_a_directory_segment_marks_the_file_name_s() {
        // A `u0???p` directory respells every Studio kit token the file
        // name below it holds, as it does its own segment's.
        assert_eq!(
            respell("u0XXXp2/kit1.dds", Spelling::Name),
            "kit2/kit_tex1.dds"
        );
        assert_eq!(respell("u0XXXp0/kit1", Spelling::Path), "kitN/kit_tex1");
        assert_eq!(
            respell("u0XXXp0/kit1.dds", Spelling::Name),
            "kitF/kit_tex1.dds"
        );
        assert_eq!(
            respell("ltd/u0XXXp1/pants_kit3.dds", Spelling::Name),
            "ltd/kit1/pants_kit_tex3.dds"
        );
        // A `g` token's directory marks them as a `p` token's does.
        assert_eq!(
            respell("u0XXXg1/kit2.dds", Spelling::Name),
            "gk_kit1/kit_tex2.dds"
        );
        // The file name's own respelled token is no Studio token of the
        // author's: it is not rewritten.
        assert_eq!(
            respell("u0XXXp1/u0XXXp2.dds", Spelling::Name),
            "kit1/kit2.dds"
        );
        assert_eq!(
            respell("u0XXXp2/armor_bsm.dds", Spelling::Name),
            "kit2/armor_bsm.dds"
        );
    }

    #[test]
    fn the_respelled_token_holds_no_token() {
        // `kit_tex` was never a token: the name reads as plain text.
        for stem in ["kit_tex1", "kit_texN", "kit_texF"] {
            assert_eq!(aesthetics_export::kit_token(stem), None, "{stem}");
        }
        // `gk_kit1` holds the token `kit1` (the keeper's textures are a
        // kit set read with the outfielders' kit number).
        assert_eq!(
            aesthetics_export::kit_token("gk_kit1"),
            Some((
                aesthetics_export::KitToken::Variant(1),
                "gk_kitN".to_owned()
            ))
        );
    }

    #[test]
    fn a_split_reference_respells_as_one_path() {
        let (dir, name) = respell_split("./u0XXXp0/", "kit1", Spelling::Path);
        assert_eq!(dir, "./kitN/");
        assert_eq!(name, "kit_tex1");
        let (dir, name) = respell_split("u0XXXp2/", "kit1.dds", Spelling::Path);
        assert_eq!(dir, "kit2/");
        assert_eq!(name, "kit_tex1.dds");
        // No legacy token in the directory: the file name decides alone.
        let (dir, name) = respell_split("./ltd/", "kit1", Spelling::Path);
        assert_eq!(dir, "./ltd/");
        assert_eq!(name, "kit1");
        // Nothing to respell: both halves are borrowed.
        let (dir, name) = respell_split("./", "armor_bsm", Spelling::Path);
        assert!(matches!(dir, Cow::Borrowed(_)));
        assert!(matches!(name, Cow::Borrowed(_)));
        // An empty directory half is no directory segment.
        let (dir, name) = respell_split("", "u0XXXp1.dds", Spelling::Name);
        assert_eq!(dir, "");
        assert_eq!(name, "kit1.dds");
    }

    #[test]
    fn player_and_common_outputs_respell_their_rest() {
        let copies = respell_outputs(vec![
            ("a".into(), "Players/07 - A/u0XXXp1.dds".into()),
            ("b".into(), "Players/07 - A/u0XXXp0/armor.dds".into()),
            ("c".into(), "Common/ltd/u0XXXp5/a.dds".into()),
            ("d".into(), "Common/u0XXXg1.dds".into()),
            ("e".into(), "Boots/Crocs/body_d_u0XXXp0.dds".into()),
            ("f".into(), "Portraits/07.dds".into()),
            ("g".into(), "Kits/p1/kit.dds".into()),
            ("h".into(), "Players/07 - u0XXXp1/x.dds".into()),
        ]);
        assert_eq!(
            copies,
            vec![
                ("a".to_owned(), "Players/07 - A/kit1.dds".to_owned()),
                ("b".to_owned(), "Players/07 - A/kitF/armor.dds".to_owned()),
                ("c".to_owned(), "Common/ltd/kit5/a.dds".to_owned()),
                ("d".to_owned(), "Common/gk_kit1.dds".to_owned()),
                ("e".to_owned(), "Boots/Crocs/body_d_kitF.dds".to_owned()),
                ("f".to_owned(), "Portraits/07.dds".to_owned()),
                ("g".to_owned(), "Kits/p1/kit.dds".to_owned()),
                ("h".to_owned(), "Players/07 - u0XXXp1/x.dds".to_owned()),
            ]
        );
    }

    #[test]
    fn respelled_names_collide_as_before() {
        let (copies, findings) = drop_collisions(respell_outputs(vec![
            ("a".into(), "Players/07 - A/u0XXXp1.dds".into()),
            ("b".into(), "Players/07 - A/u0714p1.dds".into()),
        ]));
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
        let (copies, findings) = drop_collisions(vec![
            (
                "Faces/XXX07 - Ann/u0715p3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned(),
            ),
            (
                "Faces/XXX07 - Ann/u0XXXp3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned(),
            ),
        ]);
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
    fn of_two_lettered_copies_the_later_in_name_order_wins() {
        // Both renamed whatever the team: `u0xxx` (folded `XXX`) after
        // `u0ABC`, so it replaced it, listed first here or not.
        let (copies, findings) = drop_collisions(vec![
            (
                "Faces/XXX07 - Ann/u0xxxp3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned(),
            ),
            (
                "Faces/XXX07 - Ann/u0ABCp3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned(),
            ),
        ]);
        assert_eq!(
            copies,
            vec![(
                "Faces/XXX07 - Ann/u0xxxp3.dds".to_owned(),
                "Players/07 - Ann/kit3.dds".to_owned()
            )]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "leftover_dropped");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Faces/XXX07 - Ann/u0ABCp3.dds")
            }
        );
    }

    #[test]
    fn inputs_of_different_folders_collide_as_before() {
        // Two folders' inputs are no one file to the old compile: the
        // first in list order is kept, the second unmigrated content.
        let (copies, findings) = drop_collisions(vec![
            (
                "Faces/XXX07 - Ann/u0XXXp1.dds".to_owned(),
                "Players/07 - Ann/kit1.dds".to_owned(),
            ),
            (
                "Boots/k0128 - Rex/u0XXXp1.dds".to_owned(),
                "Players/07 - Ann/kit1.dds".to_owned(),
            ),
        ]);
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
    fn numbered_copies_alone_collide_as_before() {
        // `u0714` beside `u0799`: which one the old compile shipped
        // depended on the team's ID, which the rule reads nowhere — the
        // first in list order is kept, the second unmigrated content.
        let (copies, findings) = drop_collisions(vec![
            (
                "Faces/XXX07 - Ann/u0714p2.dds".to_owned(),
                "Players/07 - Ann/kit2.dds".to_owned(),
            ),
            (
                "Faces/XXX07 - Ann/u0799p2.dds".to_owned(),
                "Players/07 - Ann/kit2.dds".to_owned(),
            ),
        ]);
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
                path: path_scope("Faces/XXX07 - Ann/u0799p2.dds")
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
    fn a_lettered_copy_beside_the_numbered_one_of_another_team_wins_too() {
        // A `u0799` input spelled for another team and a `u0XXX` one were
        // one file to the old compile, whose pick is the lettered copy
        // whatever the team's ID.
        let (copies, findings) = drop_collisions(vec![
            (
                "Players/arbiter/u0XXXp2.dds".to_owned(),
                "Players/arbiter/kit2.dds".to_owned(),
            ),
            (
                "Players/arbiter/u0999p2.dds".to_owned(),
                "Players/arbiter/kit2.dds".to_owned(),
            ),
        ]);
        assert_eq!(
            copies,
            vec![(
                "Players/arbiter/u0XXXp2.dds".to_owned(),
                "Players/arbiter/kit2.dds".to_owned()
            )]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "leftover_dropped");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Players/arbiter/u0999p2.dds")
            }
        );
    }

    #[test]
    fn old_compile_name_folds_each_token_s_three_characters_to_xxx() {
        // A token not at the start, two tokens, both letters: `u0` + three
        // letters or digits + `p`/`g`, each's `???` written `XXX`.
        assert_eq!(
            old_compile_name("x_u0XXXp1_u0abcg1.dds"),
            "x_u0XXXp1_u0XXXg1.dds"
        );
        assert_eq!(
            old_compile_name("Faces/XXX07 - Ann/u0731p2.dds"),
            "Faces/XXX07 - Ann/u0XXXp2.dds"
        );
        // A token ending the text.
        assert_eq!(old_compile_name("u0XXXp"), "u0XXXp");
        // The near-misses are no token: not `u`, not `0`, a
        // non-alphanumeric character, no `p`/`g`.
        for unchanged in ["k0abcp1.dds", "uaXXXp1.dds", "u0X-Xp1.dds", "u0XXXq1.dds"] {
            assert_eq!(old_compile_name(unchanged), unchanged);
        }
    }
}
