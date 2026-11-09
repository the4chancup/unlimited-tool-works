//! The pre-Fox model renames. The leading type the old export generated
//! `face.xml` entries from moves to the stem's end (`gloveL_foo` →
//! `foo_gloveL`), only when `_` follows it: moving letters off a glued word
//! would make a name nobody wrote. A model's `oral_`/`_win32` wrappers drop
//! (`oral_parts_body_win32` → `parts_body`), in `Common/` too, and the
//! `.mtl` matched against the unwrapped name follows (`gloveL_foo.mtl` →
//! `foo_gloveL.mtl`). A folder that ships its own `face.xml` keeps every
//! name.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::common_link_name;

use crate::face_xml::xml_folders;

/// The type spellings the old export put first, tried in this order (plan
/// step 7): `face_neck`/`pants_nocloth`'s own `_` is no letter the stem
/// must carry — matching skips `_`s on both sides.
const TYPES: [&str; 10] = [
    "face_neck",
    "handL",
    "handR",
    "gloveL",
    "gloveR",
    "uniform",
    "shirt",
    "pants_nocloth",
    "eye",
    "mouth",
];

/// The part between `oral_` and `_win32` when `stem` starts and ends with
/// them (ASCII case-insensitive) and something is left between
/// (`oral_parts_body_win32` → `parts_body`; `oral_win32`, `oral_foo`,
/// `face_high_win32` → `None`).
fn without_wrappers(stem: &str) -> Option<&str> {
    let head = stem.get(..5)?;
    let tail = stem.get(stem.len().checked_sub(6)?..)?;
    if !head.eq_ignore_ascii_case("oral_") || !tail.eq_ignore_ascii_case("_win32") {
        return None;
    }
    stem.get(5..stem.len() - 6)
        .filter(|inner| !inner.is_empty())
}

/// `stem`'s leading type moved to its end — the rest + `_` + the matched
/// part as written (`gloveL_foo` → `foo_gloveL`, `glove_l_foo` →
/// `foo_glove_l`, `uniform_shirt` → `shirt_uniform`) — `None` when the stem
/// holds `model_type_` (ASCII case-insensitive, anywhere) or no type leads
/// with `_` and more of the name after it.
fn swapped(stem: &str) -> Option<String> {
    if vtree::fold_name(stem).contains("model_type_") {
        return None;
    }
    for type_name in TYPES {
        if let Some(matched) = leading_type(stem, type_name) {
            let rest = stem[matched..].strip_prefix('_')?;
            let rest = rest.trim_start_matches('_');
            if rest.is_empty() {
                return None;
            }
            return Some(format!("{rest}_{}", &stem[..matched]));
        }
    }
    None
}

/// The length in `stem` of `type_name`'s leading match: the type's letters
/// (its own `_`s not among them — `face_neck` matches as `faceneck`) at the
/// stem's start, ASCII case-insensitive, `_` skipped in the stem before
/// every letter but the first (`face_neck` matches `face_neck_sid`,
/// `faceneck_sid` and `face__neck_sid` alike); `None` when it does not
/// lead.
fn leading_type(stem: &str, type_name: &str) -> Option<usize> {
    let mut rest = stem;
    let mut consumed = 0;
    for (index, letter) in type_name.bytes().filter(|byte| *byte != b'_').enumerate() {
        if index > 0 {
            let skipped = rest.len() - rest.trim_start_matches('_').len();
            rest = &rest[skipped..];
            consumed += skipped;
        }
        let byte = *rest.as_bytes().first()?;
        if !byte.eq_ignore_ascii_case(&letter) {
            return None;
        }
        rest = &rest[1..];
        consumed += 1;
    }
    Some(consumed)
}

/// `stem`'s new name by step 7's two rules: the wrappers dropped when both
/// stand, then the leading type swapped — `base` alone when only the
/// wrappers went.
fn renamed_stem(stem: &str) -> Cow<'_, str> {
    let base = without_wrappers(stem).unwrap_or(stem);
    match swapped(base) {
        Some(new) => Cow::Owned(new),
        None => Cow::Borrowed(base),
    }
}

/// Where an output the renames reach stands: below an xml-less
/// `Players/<folder>/`, a shared `Boots/<name>/` or `Gloves/<name>/`, or
/// below `Common/`.
#[derive(Clone, Copy)]
struct Site<'a> {
    /// The output's path below the model folder or `Common/`.
    below: &'a str,
    /// Whether it is below `Common/`.
    common: bool,
}

/// `output`'s `Site`: below a `Players/<folder>/` not holding a member
/// `face.xml` (`xml` the folded names of those that do), a shared folder
/// (which holds no `face.xml`: validation refuses a boots model whose name
/// does not end in its type, `model_name_invalid`) or `Common/`; `None` for
/// an xml folder or any other top folder.
fn site_of<'a>(output: &'a str, xml: &BTreeSet<String>) -> Option<Site<'a>> {
    let (top, rest) = output.split_once('/')?;
    if top == "Common" {
        return Some(Site {
            below: rest,
            common: true,
        });
    }
    let (folder, below) = rest.split_once('/')?;
    let renamed = match top {
        "Players" => !xml.contains(&vtree::fold_name(folder)),
        "Boots" | "Gloves" => true,
        _ => false,
    };
    renamed.then_some(Site {
        below,
        common: false,
    })
}

/// `name` split before `suffix` when it ends with it (ASCII
/// case-insensitive): the stem and the suffix as written.
fn split_suffix_ci<'a>(name: &'a str, suffix: &str) -> Option<(&'a str, &'a str)> {
    let at = name.len().checked_sub(suffix.len())?;
    name.get(at..)?
        .eq_ignore_ascii_case(suffix)
        .then(|| (&name[..at], &name[at..]))
}

/// `name`'s `.common` link target and the link's own suffix as written
/// (`common_link_name`, a `.common.txt` included): `x.model.common` →
/// `("x.model", ".common")`; `None` when `name` is no link.
fn link_parts(name: &str) -> Option<(String, &str)> {
    let target = common_link_name(name)?;
    let at = target.len();
    Some((target, &name[at..]))
}

/// The stem `name` renames under step 7 — a `.model`'s own, a
/// `.model.common` link's target's — and the tail to write after the new
/// stem (the `.model` as written, or the link's `.model.common`); `None`
/// for other names or when `kept` covers the `Common/` path the name
/// stands for.
fn scoped_stem(name: &str, site: Site, kept: &BTreeSet<String>) -> Option<(String, String)> {
    let below = site.below;
    if site.common {
        let (stem, ext) = split_suffix_ci(name, ".model")?;
        if kept.contains(&vtree::fold_name(below)) {
            return None;
        }
        return Some((stem.to_owned(), ext.to_owned()));
    }
    if let Some((stem, ext)) = split_suffix_ci(name, ".model") {
        return Some((stem.to_owned(), ext.to_owned()));
    }
    let (target, suffix) = link_parts(name)?;
    let (stem, ext) = split_suffix_ci(&target, ".model")?;
    let target_below = &below[..below.len() - suffix.len()];
    if kept.contains(&vtree::fold_name(target_below)) {
        return None;
    }
    Some((stem.to_owned(), format!("{ext}{suffix}")))
}

/// `output` renamed per step 7 (`mtls` the swap map `rename_models`
/// collected); `to` as given when nothing applies.
fn rename_output(
    to: String,
    xml: &BTreeSet<String>,
    kept: &BTreeSet<String>,
    mtls: &BTreeMap<(String, String), String>,
) -> String {
    let Some(site) = site_of(&to, xml) else {
        return to;
    };
    let Some((dir, name)) = to.rsplit_once('/') else {
        return to;
    };
    if let Some((stem, tail)) = scoped_stem(name, site, kept) {
        return format!("{dir}/{}{tail}", renamed_stem(&stem));
    }
    // A `.mtl.common` link follows its `Common/` target when that `.mtl`
    // follows a swap-renamed `Common/` model (a link to a renamed name
    // would name a missing file).
    if !site.common
        && let Some((target, suffix)) = link_parts(name)
        && let Some((stem, ext)) = split_suffix_ci(&target, ".mtl")
    {
        let target_below = &site.below[..site.below.len() - suffix.len()];
        let common_dir = match target_below.rsplit_once('/') {
            Some((target_dir, _)) => format!("Common/{target_dir}"),
            None => "Common".to_owned(),
        };
        return match mtls.get(&(vtree::fold_name(&common_dir), vtree::fold_name(stem))) {
            Some(new) if !kept.contains(&vtree::fold_name(target_below)) => {
                format!("{dir}/{new}{ext}{suffix}")
            }
            _ => to,
        };
    }
    // A `.mtl` in a swap-renamed model's directory, its stem the model's
    // `base`.
    let Some((stem, ext)) = split_suffix_ci(name, ".mtl") else {
        return to;
    };
    if site.common && kept.contains(&vtree::fold_name(site.below)) {
        return to;
    }
    match mtls.get(&(vtree::fold_name(dir), vtree::fold_name(stem))) {
        Some(new) => format!("{dir}/{new}{ext}"),
        None => to,
    }
}

/// Step 3's Fox hair skeleton rename ("SKL pairing" in
/// `spec/aesthetics_export/player_folders.md`): the old format names a
/// hair's skeleton after its game slot, `fcl_hair_sim.skl`, while the
/// Studio format pairs a skeleton with its model by the model's stem, so a
/// `fcl_hair_sim.skl` in an output folder that holds an `fcl_hair.fmdl`
/// and no `fcl_hair.skl` (the names compared ASCII case-insensitively)
/// becomes `fcl_hair.skl`, lower case — kept as named, the compile
/// reports it `file_not_used` and injects the template in its place. Only
/// the file name changes; its folder and bytes stay.
fn pair_hair_skeletons(copies: Vec<(String, String)>) -> Vec<(String, String)> {
    // Per output folder (folded): whether its outputs hold the model and
    // whether one already holds the paired name.
    let mut folders: BTreeMap<String, (bool, bool)> = BTreeMap::new();
    for (_, output) in &copies {
        let Some((dir, name)) = output.rsplit_once('/') else {
            continue;
        };
        let entry = folders.entry(vtree::fold_name(dir)).or_default();
        if name.eq_ignore_ascii_case("fcl_hair.fmdl") {
            entry.0 = true;
        } else if name.eq_ignore_ascii_case("fcl_hair.skl") {
            entry.1 = true;
        }
    }
    copies
        .into_iter()
        .map(|(from, to)| {
            let Some((dir, name)) = to.rsplit_once('/') else {
                return (from, to);
            };
            if name.eq_ignore_ascii_case("fcl_hair_sim.skl")
                && folders
                    .get(&vtree::fold_name(dir))
                    .is_some_and(|&(model, paired)| model && !paired)
            {
                (from, format!("{dir}/fcl_hair.skl"))
            } else {
                (from, to)
            }
        })
        .collect()
}

/// What the model renames make of the copies' output names. Below a
/// `Players/<folder>/` holding no member `face.xml` (subfolders included),
/// a shared `Boots/<name>/` or
/// `Gloves/<name>/`, and below `Common/`, a `.model`
/// is renamed by `renamed_stem` (its leading type moved last, its
/// `oral_`/`_win32` wrappers off), a `.model.common` link follows its
/// `Common/` target's rename, and a `.mtl` named after a swap-renamed
/// stem follows it, as a `.mtl.common` link follows such a `Common/`
/// `.mtl`. A `Common/` path in `kept` — one a member `face.xml` or a link
/// in an xml folder names — and every other output keep their names; a
/// folder shipping a `face.xml` keeps every name. `copies` holds the player
/// and shared folders' outputs and `Common/`'s alike, so a link sees its
/// target's rename. Before any of it, the `fcl_hair_sim.skl` pairing
/// (`pair_hair_skeletons`) applies to every output folder alike.
pub(crate) fn rename_models(
    copies: Vec<(String, String)>,
    kept: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let copies = pair_hair_skeletons(copies);
    let xml = xml_folders(&copies);
    // The `.mtl` a swap-renamed model or link brings along keys on the
    // model's directory and its `base` (the stem the swap started from),
    // both folded.
    let mut mtls: BTreeMap<(String, String), String> = BTreeMap::new();
    for (_, output) in &copies {
        let Some(site) = site_of(output, &xml) else {
            continue;
        };
        let Some((dir, name)) = output.rsplit_once('/') else {
            continue;
        };
        let Some((stem, _)) = scoped_stem(name, site, kept) else {
            continue;
        };
        let base = without_wrappers(&stem).unwrap_or(&stem);
        if let Some(new) = swapped(base) {
            mtls.insert((vtree::fold_name(dir), vtree::fold_name(base)), new);
        }
    }
    copies
        .into_iter()
        .map(|(from, to)| (from, rename_output(to, &xml, kept, &mtls)))
        .collect()
}

#[cfg(test)]
mod tests {
    use studio_core::Scope;

    use super::*;
    use crate::messages::INPUT;
    use crate::players::path_scope;

    /// `renamed_stem(stem)` as a plain string for the assertions.
    fn stem(stem: &str) -> String {
        renamed_stem(stem).into_owned()
    }

    #[test]
    fn the_leading_type_moves_to_the_end_as_written() {
        assert_eq!(stem("gloveL_foo"), "foo_gloveL");
        assert_eq!(stem("glove_l_foo"), "foo_glove_l");
        assert_eq!(stem("GLOVEL_Foo"), "Foo_GLOVEL");
        assert_eq!(stem("glover_rabbed"), "rabbed_glover");
        assert_eq!(stem("face_neck_sid"), "sid_face_neck");
        assert_eq!(stem("pants_nocloth_x"), "x_pants_nocloth");
        assert_eq!(stem("uniform_shirt"), "shirt_uniform");
    }

    #[test]
    fn the_wrappers_drop_and_the_swap_applies_to_the_base() {
        assert_eq!(stem("oral_parts_body_win32"), "parts_body");
        assert_eq!(stem("ORAL_x_WIN32"), "x");
        assert_eq!(stem("oral_gloveL_foo_win32"), "foo_gloveL");
    }

    #[test]
    fn everything_else_keeps_its_stem() {
        for unchanged in [
            "eyebrow_x",
            "gloveL",
            "gloveL_",
            "_gloveL_foo",
            "foo_gloveL",
            "model_type_cape_foo",
            "gloveL_model_type_x",
            "oral_foo",
            "face_high_win32",
            "oral_win32",
            "plain",
        ] {
            assert_eq!(stem(unchanged), unchanged, "{unchanged}");
        }
    }

    /// `rename_models` over `copies`, `kept` the folded `Common/` paths.
    fn renamed(copies: &[&str], kept: &[&str]) -> Vec<String> {
        let copies: Vec<(String, String)> = copies
            .iter()
            .map(|path| ((*path).to_owned(), (*path).to_owned()))
            .collect();
        let kept: BTreeSet<String> = kept.iter().map(|path| vtree::fold_name(path)).collect();
        rename_models(copies, &kept)
            .into_iter()
            .map(|(_, output)| output)
            .collect()
    }

    #[test]
    fn an_xml_folder_keeps_every_name_an_xml_less_one_renames() {
        assert_eq!(
            renamed(
                &[
                    "Players/07 - A/face.xml",
                    "Players/07 - A/gloveL_baz.model",
                    "Players/08 - B/gloveL_foo.model",
                    "Players/08 - B/gloveL_foo.mtl",
                ],
                &[],
            ),
            [
                "Players/07 - A/face.xml",
                "Players/07 - A/gloveL_baz.model",
                "Players/08 - B/foo_gloveL.model",
                "Players/08 - B/foo_gloveL.mtl",
            ]
        );
    }

    #[test]
    fn the_wrapper_drop_renames_no_mtl_but_the_swap_does() {
        assert_eq!(
            renamed(
                &[
                    "Players/07 - A/oral_parts_body_win32.model",
                    "Players/07 - A/parts_body.mtl",
                    "Players/07 - A/materials.mtl",
                    "Players/07 - A/oral_gloveL_foo_win32.model",
                    "Players/07 - A/gloveL_foo.mtl",
                    "Players/07 - A/sub/gloveL_foo.mtl",
                ],
                &[],
            ),
            [
                "Players/07 - A/parts_body.model",
                "Players/07 - A/parts_body.mtl",
                "Players/07 - A/materials.mtl",
                "Players/07 - A/foo_gloveL.model",
                "Players/07 - A/foo_gloveL.mtl",
                "Players/07 - A/sub/gloveL_foo.mtl",
            ]
        );
    }

    #[test]
    fn a_link_follows_its_target_unless_kept() {
        let copies = [
            "Players/07 - A/oral_cap_win32.model.common",
            "Players/07 - A/jessie/oral_x_win32.model.common",
            "Players/07 - A/gloveL_foo.mtl.common",
            "Common/oral_cap_win32.model",
            "Common/jessie/oral_x_win32.model",
            "Boots/Crocs/oral_boots_win32.model",
            "Gloves/G/gloveL_x.model",
            "Other/oral_x_win32.model",
        ];
        assert_eq!(
            renamed(&copies, &[]),
            [
                "Players/07 - A/cap.model.common",
                "Players/07 - A/jessie/x.model.common",
                "Players/07 - A/gloveL_foo.mtl.common",
                "Common/cap.model",
                "Common/jessie/x.model",
                "Boots/Crocs/boots.model",
                "Gloves/G/x_gloveL.model",
                "Other/oral_x_win32.model",
            ]
        );
        // `kept` covering the link's target leaves link and target alike.
        assert_eq!(
            renamed(&copies, &["oral_cap_win32.model"]),
            [
                "Players/07 - A/oral_cap_win32.model.common",
                "Players/07 - A/jessie/x.model.common",
                "Players/07 - A/gloveL_foo.mtl.common",
                "Common/oral_cap_win32.model",
                "Common/jessie/x.model",
                "Boots/Crocs/boots.model",
                "Gloves/G/x_gloveL.model",
                "Other/oral_x_win32.model",
            ]
        );
    }

    #[test]
    fn a_kept_common_mtl_stays_and_a_player_folder_s_still_follows() {
        // `kept` names `Common/` paths: a player folder's `.mtl` of the same
        // path is no `Common/` file and follows its model.
        assert_eq!(
            renamed(
                &[
                    "Players/07 - A/gloveL_foo.model",
                    "Players/07 - A/gloveL_foo.mtl",
                    "Common/gloveL_bar.model",
                    "Common/gloveL_bar.mtl",
                ],
                &["gloveL_foo.mtl", "gloveL_bar.mtl"],
            ),
            [
                "Players/07 - A/foo_gloveL.model",
                "Players/07 - A/foo_gloveL.mtl",
                "Common/bar_gloveL.model",
                "Common/gloveL_bar.mtl",
            ]
        );
    }

    #[test]
    fn an_mtl_link_follows_its_common_target_s_rename() {
        let copies = [
            "Players/07 - A/gloveL_foo.mtl.common",
            "Players/07 - A/sub/gloveL_bar.mtl.common",
            "Common/gloveL_foo.model",
            "Common/gloveL_foo.mtl",
            "Common/sub/gloveL_bar.model",
            "Common/sub/gloveL_bar.mtl",
        ];
        assert_eq!(
            renamed(&copies, &[]),
            [
                "Players/07 - A/foo_gloveL.mtl.common",
                "Players/07 - A/sub/bar_gloveL.mtl.common",
                "Common/foo_gloveL.model",
                "Common/foo_gloveL.mtl",
                "Common/sub/bar_gloveL.model",
                "Common/sub/bar_gloveL.mtl",
            ]
        );
        // A kept `Common/` `.mtl` keeps its name, and so does its link.
        assert_eq!(
            renamed(&copies, &["gloveL_foo.mtl"])[..4],
            [
                "Players/07 - A/gloveL_foo.mtl.common",
                "Players/07 - A/sub/bar_gloveL.mtl.common",
                "Common/foo_gloveL.model",
                "Common/gloveL_foo.mtl",
            ]
        );
    }

    #[test]
    fn a_hair_skeleton_beside_its_model_pairs_as_fcl_hair_skl() {
        assert_eq!(
            renamed(
                &[
                    "Players/05 - Bob/fcl_hair.fmdl",
                    "Players/05 - Bob/fcl_hair_sim.skl",
                ],
                &[],
            ),
            [
                "Players/05 - Bob/fcl_hair.fmdl",
                "Players/05 - Bob/fcl_hair.skl",
            ]
        );
    }

    #[test]
    fn a_hair_skeleton_without_the_model_keeps_its_name() {
        assert_eq!(
            renamed(&["Players/06 - Ann/fcl_hair_sim.skl"], &[],),
            ["Players/06 - Ann/fcl_hair_sim.skl"]
        );
    }

    #[test]
    fn a_folder_holding_fcl_hair_skl_already_keeps_all_three_names() {
        assert_eq!(
            renamed(
                &[
                    "Players/05 - Bob/fcl_hair.fmdl",
                    "Players/05 - Bob/fcl_hair.skl",
                    "Players/05 - Bob/fcl_hair_sim.skl",
                ],
                &[],
            ),
            [
                "Players/05 - Bob/fcl_hair.fmdl",
                "Players/05 - Bob/fcl_hair.skl",
                "Players/05 - Bob/fcl_hair_sim.skl",
            ]
        );
    }

    #[test]
    fn a_folder_holding_fcl_hair_skl_in_another_case_keeps_all_three_names() {
        assert_eq!(
            renamed(
                &[
                    "Players/05 - Bob/fcl_hair.fmdl",
                    "Players/05 - Bob/FCL_HAIR.skl",
                    "Players/05 - Bob/fcl_hair_sim.skl",
                ],
                &[],
            ),
            [
                "Players/05 - Bob/fcl_hair.fmdl",
                "Players/05 - Bob/FCL_HAIR.skl",
                "Players/05 - Bob/fcl_hair_sim.skl",
            ]
        );
    }

    #[test]
    fn the_hair_skeleton_pairing_compares_names_in_any_letter_case() {
        assert_eq!(
            renamed(
                &[
                    "Players/05 - Bob/FCL_HAIR.FMDL",
                    "Players/05 - Bob/Fcl_Hair_Sim.skl",
                ],
                &[],
            ),
            [
                "Players/05 - Bob/FCL_HAIR.FMDL",
                "Players/05 - Bob/fcl_hair.skl",
            ]
        );
    }

    #[test]
    fn a_rename_colliding_with_an_earlier_output_drops_the_later() {
        let (copies, findings) = crate::kit_magic::drop_collisions(rename_models(
            vec![
                (
                    "a".to_owned(),
                    "Players/07 - A/oral_x_win32.model".to_owned(),
                ),
                ("b".to_owned(), "Players/07 - A/x.model".to_owned()),
            ],
            &BTreeSet::new(),
        ));
        assert_eq!(
            copies,
            vec![("a".to_owned(), "Players/07 - A/x.model".to_owned())]
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code.code.as_ref(), "unmigrated_content");
        assert_eq!(
            findings[0].scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("b"),
            }
        );
    }
}
