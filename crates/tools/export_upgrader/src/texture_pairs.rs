//! The `.dds`/`.ftex` pairs an output folder can hold: two texture files
//! claiming one folded stem in one directory, which the shared validation
//! refuses as `texture_stem_conflict`. A pair is resolved before the
//! output is written: the `.ftex`
//! drops first, so step 7's links are spelled after the kept `.dds` alone,
//! and once `material_references`' `used` says which stems the export's
//! models wear, an unused stem's `.dds` follows as a leftover; a `dummy_`
//! stem counts as unused, the game substituting it. Any other pair of
//! extensions stays the validation's Error.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{FileKind, classify, common_link_name};
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, INPUT, tool_message};
use crate::players::{file_name, path_scope};

/// One `.dds`/`.ftex` pair claiming a folded stem in one output directory.
#[derive(Debug)]
pub(crate) struct StemPair {
    /// The directory holding the pair, folded (`common`, `players/05 - a`).
    pub(crate) dir: String,
    /// The stem the pair claims, folded.
    pub(crate) stem: String,
    /// The `.dds` file's (input, output) paths.
    pub(crate) dds: (String, String),
    /// The `.ftex` file's (input, output) paths.
    pub(crate) ftex: (String, String),
}

/// The name `name` claims in a directory's texture namespace, as the shared
/// validation's claim reads it: a texture's own, or a texture `.common`
/// link's target name.
fn texture_claim(name: &str) -> Option<String> {
    match classify(name) {
        FileKind::Texture => Some(name.to_owned()),
        FileKind::CommonLink => {
            common_link_name(name).filter(|target| classify(target) == FileKind::Texture)
        }
        _ => None,
    }
}

/// The `.dds`/`.ftex` pairs among `copies`' output paths: a folded stem in
/// one directory claimed by exactly a `.dds` and a `.ftex` file and nothing
/// else, folded as the shared validation folds.
pub(crate) fn stem_pairs<'a>(copies: impl Iterator<Item = &'a (String, String)>) -> Vec<StemPair> {
    let mut namespaces: BTreeMap<(String, String), Vec<&'a (String, String)>> = BTreeMap::new();
    for copy in copies {
        let Some(claimed) = texture_claim(file_name(&copy.1)) else {
            continue;
        };
        let Some(stem) = claimed.rsplit_once('.').map(|(stem, _)| stem) else {
            continue;
        };
        if stem.is_empty() {
            continue;
        }
        let dir = copy.1.rsplit_once('/').map_or("", |(dir, _)| dir);
        namespaces
            .entry((vtree::fold_name(dir), vtree::fold_name(stem)))
            .or_default()
            .push(copy);
    }
    let mut pairs = Vec::new();
    for ((dir, stem), claims) in namespaces {
        if claims.len() != 2 {
            continue;
        }
        let mut dds = None;
        let mut ftex = None;
        for copy in &claims {
            let name = file_name(&copy.1);
            if classify(name) != FileKind::Texture {
                continue;
            }
            match name
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
                .as_deref()
            {
                Some("dds") => dds = Some((*copy).clone()),
                Some("ftex") => ftex = Some((*copy).clone()),
                _ => {}
            }
        }
        if let (Some(dds), Some(ftex)) = (dds, ftex) {
            pairs.push(StemPair {
                dir,
                stem,
                dds,
                ftex,
            });
        }
    }
    pairs
}

/// The findings `pairs` resolve to now that `used` says which stems the
/// export's models wear (the folded directory and stem of every place a
/// reference lands), and the `.dds` input paths to drop beside the already
/// dropped `.ftex`: a used pair drops its `.ftex` alone
/// (`texture_duplicate_dropped`, `kept` the `.dds`'s output path); an
/// unused pair's files are both leftovers, a `dummy_` stem counting as
/// unused.
pub(crate) fn resolve(
    pairs: &[StemPair],
    used: &BTreeSet<(String, String)>,
) -> (Vec<Message>, BTreeSet<String>) {
    let scope = |input: &str| Scope::File {
        export_id: INPUT,
        path: path_scope(input),
    };
    let mut findings = Vec::new();
    let mut dds_dropped = BTreeSet::new();
    for pair in pairs {
        if !pair.stem.starts_with("dummy_") && used.contains(&(pair.dir.clone(), pair.stem.clone()))
        {
            findings.push(tool_message(
                Code::TextureDuplicateDropped,
                scope(&pair.ftex.0),
                Disposition::DropFile,
                vec![("kept", pair.dds.1.clone())],
            ));
        } else {
            dds_dropped.insert(pair.dds.0.clone());
            for (input, _) in [&pair.dds, &pair.ftex] {
                findings.push(tool_message(
                    Code::LeftoverDropped,
                    scope(input),
                    Disposition::DropFile,
                    vec![],
                ));
            }
        }
    }
    (findings, dds_dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `stem_pairs` over (input, output) copy pairs.
    fn pairs_of(copies: &[(&str, &str)]) -> Vec<StemPair> {
        let copies: Vec<(String, String)> = copies
            .iter()
            .map(|(input, output)| ((*input).to_owned(), (*output).to_owned()))
            .collect();
        stem_pairs(copies.iter())
    }

    #[test]
    fn only_an_exact_dds_ftex_pair_is_found() {
        let pairs = pairs_of(&[
            ("Common/shiny.dds", "Common/shiny.dds"),
            ("Common/shiny.ftex", "Common/shiny.ftex"),
        ]);
        assert_eq!(pairs.len(), 1, "{pairs:?}");
        assert_eq!(pairs[0].dir, "common");
        assert_eq!(pairs[0].stem, "shiny");
        assert_eq!(pairs[0].dds.0, "Common/shiny.dds");
        assert_eq!(pairs[0].ftex.0, "Common/shiny.ftex");
        // Folded: letter case and the output directory's spelling alike.
        let pairs = pairs_of(&[
            ("Common/Sub/X.DDS", "common/sub/x.DDS"),
            ("Common/Sub/X.FTEX", "Common/Sub/X.FTEX"),
        ]);
        assert_eq!(pairs.len(), 1, "{pairs:?}");
        assert_eq!(pairs[0].dir, "common/sub");
        // A third claimant of the stem, a different extension pair, and a
        // stem in two directories are all left for the validation.
        let none = pairs_of(&[
            ("Common/x.dds", "Common/x.dds"),
            ("Common/x.ftex", "Common/x.ftex"),
            ("Common/x.png", "Common/x.png"),
            ("Common/y.dds", "Common/y.dds"),
            ("Common/y.png", "Common/y.png"),
            ("Common/z.dds", "Common/z.dds"),
            ("Common/sub/z.ftex", "Common/sub/z.ftex"),
        ]);
        assert!(none.is_empty(), "{none:?}");
    }

    #[test]
    fn a_common_link_claims_its_texture_target_s_name_only() {
        // A `.common` link stands in for `Common/<target>`: one to a
        // texture claims the target's stem, so the contested stem forms
        // no pair; one to a non-texture claims nothing and leaves the
        // pair to form.
        let contested = pairs_of(&[
            ("Common/shiny.dds", "Common/shiny.dds"),
            ("Common/shiny.ftex", "Common/shiny.ftex"),
            ("Common/shiny.dds.common", "Common/shiny.dds.common"),
        ]);
        assert!(contested.is_empty(), "{contested:?}");
        let pairs = pairs_of(&[
            ("Common/quiet.dds", "Common/quiet.dds"),
            ("Common/quiet.ftex", "Common/quiet.ftex"),
            ("Common/quiet.fmdl.common", "Common/quiet.fmdl.common"),
        ]);
        assert_eq!(pairs.len(), 1, "{pairs:?}");
        assert_eq!(pairs[0].stem, "quiet");
        assert_eq!(pairs[0].dds.0, "Common/quiet.dds");
    }

    #[test]
    fn resolve_keeps_the_dds_of_a_used_pair_and_drops_both_of_an_unused() {
        let pairs = pairs_of(&[
            ("Common/shiny.dds", "Common/shiny.dds"),
            ("Common/shiny.ftex", "Common/shiny.ftex"),
            ("Common/dummy_srm.dds", "Common/dummy_srm.dds"),
            ("Common/dummy_srm.ftex", "Common/dummy_srm.ftex"),
            ("Common/quiet.dds", "Common/quiet.dds"),
            ("Common/quiet.ftex", "Common/quiet.ftex"),
        ]);
        let used: BTreeSet<(String, String)> = [("common".to_owned(), "shiny".to_owned())]
            .into_iter()
            .collect();
        let (findings, dds_dropped) = resolve(&pairs, &used);
        let by_code = |code: &str| {
            findings
                .iter()
                .filter(|message| message.code.code.as_ref() == code)
                .count()
        };
        assert_eq!(by_code("texture_duplicate_dropped"), 1, "{findings:?}");
        let dropped = findings
            .iter()
            .find(|message| message.code.code.as_ref() == "texture_duplicate_dropped")
            .expect("the used pair's warning");
        assert_eq!(dropped.severity, studio_core::Severity::Warning);
        assert_eq!(
            dropped.context,
            [("kept".to_owned(), "Common/shiny.dds".to_owned())]
        );
        // `dummy_srm` counts as unused; `quiet` is unused.
        assert_eq!(by_code("leftover_dropped"), 4, "{findings:?}");
        assert_eq!(
            dds_dropped,
            ["Common/dummy_srm.dds", "Common/quiet.dds"]
                .iter()
                .map(|path| (*path).to_owned())
                .collect()
        );
    }
}
