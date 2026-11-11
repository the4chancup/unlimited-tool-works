//! Kit-dependent assets (`spec/model_format.md` "Kit-dependent assets (`kitN`)"): the token a file
//! stem carries to say which kit number it belongs to, `kit1` to `kit9`, that it stands for
//! whichever kit is picked in the game, `kitN`, or that it is the set's fallback, `kitF`
//! (`spec/team_compiler/pipeline.md` "4. Per-export non-model steps", Kit-dependent assets).

use std::borrow::Cow;

/// A stem's kit token: the reference `kitN`, the variant for one kit number, or the set's
/// fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitToken {
    /// `kitN`: the variant of the kit number picked in the game.
    Reference,
    /// `kit1` to `kit9`: the variant shown when that kit number is picked.
    Variant(u8),
    /// `kitF`: the set's fallback, what a kit number with no variant of its own gets. Never
    /// a file under its own name, so a path naming it finds nothing.
    Fallback,
}

/// The first kit token of `stem` and the stem's reference spelling (the token as `kitN`),
/// or `None` when it holds none.
pub fn kit_token(stem: &str) -> Option<(KitToken, String)> {
    let (at, token) = find_token(stem)?;
    Some((token, with_token_char(stem, at, 'N')))
}

/// The kit token of `path`, a `/`-separated path below a folder with or without an
/// extension, and `path` with that token respelled as `kitN`: the file name's first token
/// when it holds one (`kit_token`, a `.` delimiting it as it does a stem's fragments, so
/// `a.kit1.dds` reads as `a.kit1` would), which wins over every directory segment
/// (`kit1/pants_kit2` is a `kit2` file in a `kit1` folder); else the first directory
/// segment spelled exactly as one (`kit1`…`kit9`, `kitF`, `kitN`, `kit_folder`).
/// `jessie/pants_kit1` gives `(Variant(1), "jessie/pants_kitN")`, `kit2/armor_bsm`
/// `(Variant(2), "kitN/armor_bsm")`.
pub fn kit_path_token(path: &str) -> Option<(KitToken, String)> {
    let (at, token) = find_path_token(path)?;
    Some((token, with_token_char(path, at, 'N')))
}

/// `path`, a `/`-separated path whose kit token `kit_path_token` reads, with that token
/// respelled for kit `kit` (`kitF/armor_bsm` and 3 give `kit3/armor_bsm`); `None` when it
/// holds none.
pub fn variant_path(path: &str, kit: u8) -> Option<String> {
    let (at, _) = find_path_token(path)?;
    Some(with_token_char(path, at, char::from(b'0' + kit)))
}

/// The member of the kit set `reference` names that `path` is, both paths below one folder
/// (`/`-separated, extension included): `Variant(k)` for a `kit1`…`kit9` member, `Fallback`
/// for the `kitF` one, `None` when `reference` names no set or `path` is not one of its
/// members. Two forms (`spec/model_format.md` "Kit-dependent assets"): a `kitN` stem's set is its
/// own directory's files with a member token of the same reference, whatever their
/// extensions (`pants_kitN.dds` names `pants_kit1.dds`, `pants_kitF.png`); a `kitN`
/// directory segment's is the files of the same folded stem at the same path under the
/// `kit1`…`kit9` and `kitF` spellings of that segment (`kitN/armor_bsm.dds` and
/// `kitN/textures/armor_bsm.dds` name `kit1/armor_bsm.dds`, `kitF/armor_bsm.png`,
/// `kit1/textures/armor_bsm.dds`), the token spelled exactly as the stem's is and the
/// other segments comparing folded. A member counts whatever its file kind: the caller
/// picks (`common_link_files` keeps textures).
pub fn kit_set_member(reference: &str, path: &str) -> Option<KitToken> {
    let Some((KitToken::Reference, reference)) = kit_path_token(without_extension(reference))
    else {
        return None;
    };
    let Some((token @ (KitToken::Variant(_) | KitToken::Fallback), member)) =
        kit_path_token(without_extension(path))
    else {
        return None;
    };
    (vtree::fold_name(&reference) == vtree::fold_name(&member)).then_some(token)
}

/// Whether `path`, a file's path below a folder (`/`-separated), names a kit set's `kitF`
/// fallback — its file stem's token (`pants_kitF.dds`) or a `kitF` directory segment
/// (`kitF/a.dds`, `kitF/textures/a.dds`): a path that finds nothing, `kitF` never being a
/// file under its own name (`spec/model_format.md` "Kit-dependent assets").
pub(crate) fn names_kit_fallback(path: &str) -> bool {
    matches!(kit_path_token(path), Some((KitToken::Fallback, _)))
}

/// The byte offsets of every kit token's first character in `stem`, left to
/// right (`kit1_x_kit2` → `[0, 7]`), an empty `Vec` when it holds none.
/// `export_upgrader` respells each so the Studio token a stem carries beside
/// a legacy `u0???p` one no longer reads as a token once the legacy spelling
/// is gone.
pub fn kit_token_starts(stem: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut from = 0;
    while let Some((at, _)) = find_token(&stem[from..]) {
        starts.push(from + at + 1 - "kitN".len());
        from += at + 1;
    }
    starts
}

/// One legacy kit token in a name or path: the old export format's kit magic, `u0`, three
/// characters, `p` (the outfielders' kit) or `g` (the keeper's) and a digit, 7 bytes in all
/// (`kit6_u0827p0.dds`, `Common/u0XXXp1/`). Only the Export upgrader reads it, respelling it
/// to the Studio token; the compiler refuses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyKitToken {
    /// The byte offset of its `u`.
    pub start: usize,
    /// Whether it is the keeper's (`g`) rather than the outfielders' (`p`).
    pub keeper: bool,
    /// Its kit digit, 0 to 9: 0 is the kit picked in the game, 1 to 9 one kit number.
    pub digit: u8,
}

/// Every legacy kit token in `text`, left to right: a lowercase `u`, `0`, three ASCII
/// letters or digits, `p` or `g`, then an ASCII digit, in that letter case (`U0714P1` and a
/// `u0XXXg` with no digit are none), anywhere in the text, whatever is around it. A match
/// starting inside the one before it is not taken, so `u0u0Zg1p1` holds one token, at 0.
/// The one match both the Export upgrader (which respells each) and the compiler (which
/// refuses each) use, so the compiler refuses exactly what the upgrader can fix.
pub fn legacy_kit_tokens(text: &str) -> Vec<LegacyKitToken> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match legacy_kit_token_at(bytes, at) {
            Some(token) => {
                tokens.push(token);
                at += LEGACY_KIT_TOKEN_LEN;
            }
            None => at += 1,
        }
    }
    tokens
}

/// The length in bytes of a legacy kit token (`u0827p0`).
const LEGACY_KIT_TOKEN_LEN: usize = 7;

/// The legacy kit token starting at byte `at` of `bytes`, if one does.
fn legacy_kit_token_at(bytes: &[u8], at: usize) -> Option<LegacyKitToken> {
    let token = bytes.get(at..at + LEGACY_KIT_TOKEN_LEN)?;
    let three = &token[2..5];
    if !token.starts_with(b"u0") || !three.iter().all(u8::is_ascii_alphanumeric) {
        return None;
    }
    let keeper = match token[5] {
        b'p' => false,
        b'g' => true,
        _ => return None,
    };
    let digit = token[6];
    digit.is_ascii_digit().then(|| LegacyKitToken {
        start: at,
        keeper,
        digit: digit - b'0',
    })
}

/// `stem`, which holds a kit token, with its first token spelled for `kit` (`pants_kit1` and
/// 3 give `pants_kit3`); `None` when `stem` holds no token.
pub fn variant_stem(stem: &str, kit: u8) -> Option<String> {
    let (at, _) = find_token(stem)?;
    Some(with_token_char(stem, at, char::from(b'0' + kit)))
}

/// `stem` without its first kit token and one delimiter next to it, the one before the token
/// when there is one, otherwise the one after (`boots_kit1` and `kit1_boots` give `boots`,
/// `a_kit2_boots` gives `a_boots`): the stem a model's type and allowed name are read from,
/// so a per-kit model is typed as its set is. Borrowed unchanged when `stem` holds no token.
pub fn without_kit_token(stem: &str) -> Cow<'_, str> {
    let Some((at, _)) = find_token(stem) else {
        return Cow::Borrowed(stem);
    };
    let start = at + 1 - "kitN".len();
    let end = at + 1;
    // The token is delimited on both sides, so a byte next to it is a one-byte delimiter.
    let (cut_start, cut_end) = if start > 0 {
        (start - 1, end)
    } else if end < stem.len() {
        (start, end + 1)
    } else {
        (start, end)
    };
    Cow::Owned(format!("{}{}", &stem[..cut_start], &stem[cut_end..]))
}

/// The byte offset in `path` (`/`-separated) of the last character of its kit token, with
/// the token: the file name's first token when it holds one, the `.` before an extension
/// delimiting as it does a stem's other fragments; else the first directory segment a
/// whole token spells (`kit_folder`). As `find_token`, for `kit_path_token` and
/// `variant_path` to respell at.
fn find_path_token(path: &str) -> Option<(usize, KitToken)> {
    let (directory, name) = directory_name(path);
    if let Some((at, token)) = find_token(name) {
        return Some((path.len() - name.len() + at, token));
    }
    let mut start = 0;
    for segment in directory.split('/') {
        if let Some(token) = kit_folder(segment) {
            return Some((start + segment.len() - 1, token));
        }
        start += segment.len() + 1;
    }
    None
}

/// The byte offset in `stem` of its first token's last character (the `N`, `F` or the
/// digit), with the token. A token is `kit` followed by `N`, `F` or `1` to `9`, spelled
/// exactly so, with `_`, `-`, `.` or the stem's end on each side: `skitN`, `kitNx` and
/// `kitf` hold none.
fn find_token(stem: &str) -> Option<(usize, KitToken)> {
    let bytes = stem.as_bytes();
    let delimits = |byte: Option<&u8>| byte.is_none_or(|byte| matches!(byte, b'_' | b'-' | b'.'));
    stem.match_indices("kit").find_map(|(start, _)| {
        let at = start + "kit".len();
        let token = match bytes.get(at) {
            Some(b'N') => KitToken::Reference,
            Some(b'F') => KitToken::Fallback,
            Some(digit @ b'1'..=b'9') => KitToken::Variant(digit - b'0'),
            _ => return None,
        };
        let before = start.checked_sub(1).and_then(|index| bytes.get(index));
        (delimits(before) && delimits(bytes.get(at + 1))).then_some((at, token))
    })
}

/// `stem` with the ASCII character at byte offset `at` replaced by `replacement`.
fn with_token_char(stem: &str, at: usize, replacement: char) -> String {
    format!("{}{replacement}{}", &stem[..at], &stem[at + 1..])
}

/// `path`'s directory and file name: `("a/b", "c.dds")` for `a/b/c.dds`, `("", "c.dds")`
/// for `c.dds`.
fn directory_name(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

/// A file name's stem and extension: `("a.b", "dds")` for `a.b.dds`, `("x", "")` for `x`.
fn stem_extension(name: &str) -> (&str, &str) {
    name.rsplit_once('.').unwrap_or((name, ""))
}

/// `path` (`/`-separated) with its file name's extension dropped: `a/b.dds` gives `a/b`,
/// `a/b` itself unchanged.
fn without_extension(path: &str) -> &str {
    let (_, name) = directory_name(path);
    let (stem, _) = stem_extension(name);
    &path[..path.len() - name.len() + stem.len()]
}

/// The token a whole folder name is, spelled exactly as the stem token is (`kitN`,
/// `kitF`, `kit1`…`kit9`): `x_kit1` and `KIT1` are none.
pub fn kit_folder(segment: &str) -> Option<KitToken> {
    if segment.len() != "kitN".len() {
        return None;
    }
    kit_token(segment).map(|(token, _)| token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_kit_n_or_kit_1_to_9_between_delimiters_or_the_stem_s_ends() {
        let cases = [
            ("pants_kitN", KitToken::Reference, "pants_kitN"),
            ("pants_kitF", KitToken::Fallback, "pants_kitN"),
            ("pants_kit3", KitToken::Variant(3), "pants_kitN"),
            ("kit2_pants", KitToken::Variant(2), "kitN_pants"),
            ("kit1", KitToken::Variant(1), "kitN"),
            ("a-kit4.b", KitToken::Variant(4), "a-kitN.b"),
            ("pants_kit9", KitToken::Variant(9), "pants_kitN"),
        ];
        for (stem, token, reference) in cases {
            assert_eq!(
                kit_token(stem),
                Some((token, reference.to_owned())),
                "{stem}"
            );
        }
        for stem in [
            "skitN",
            "pants_kit0",
            "pants_kit10",
            "pants_kit",
            "pants_KIT1",
            "pants_kitn",
            "pants_kitf",
            "kitNx",
            "kit",
        ] {
            assert_eq!(kit_token(stem), None, "{stem}");
        }
    }

    #[test]
    fn a_path_s_token_is_its_file_name_s_else_its_first_token_folder_s() {
        let cases = [
            (
                "jessie/pants_kit1",
                KitToken::Variant(1),
                "jessie/pants_kitN",
            ),
            ("kit2/armor_bsm", KitToken::Variant(2), "kitN/armor_bsm"),
            (
                "kitN/armor_bsm.dds",
                KitToken::Reference,
                "kitN/armor_bsm.dds",
            ),
            ("jessie/kitF/a", KitToken::Fallback, "jessie/kitN/a"),
            // The file name's token wins over a directory's.
            ("kit1/pants_kit2", KitToken::Variant(2), "kit1/pants_kitN"),
        ];
        for (path, token, reference) in cases {
            assert_eq!(
                kit_path_token(path),
                Some((token, reference.to_owned())),
                "{path}"
            );
        }
        for path in [
            // A segment is a token only spelled whole.
            "old_kit2_hair/skin",
            "KIT1/a",
            "jessie/armor_bsm",
        ] {
            assert_eq!(kit_path_token(path), None, "{path}");
        }
    }

    #[test]
    fn a_variant_path_respells_the_path_s_token_for_the_kit() {
        assert_eq!(
            variant_path("kitF/armor_bsm", 3),
            Some("kit3/armor_bsm".to_owned())
        );
        assert_eq!(
            variant_path("jessie/pants_kit1.dds", 9),
            Some("jessie/pants_kit9.dds".to_owned())
        );
        assert_eq!(variant_path("kitN/a", 4), Some("kit4/a".to_owned()));
        assert_eq!(variant_path("pants", 1), None);
    }

    #[test]
    fn token_starts_lists_every_token_left_to_right() {
        assert_eq!(kit_token_starts("kit1_x_kit2"), [0, 7]);
        assert_eq!(kit_token_starts("kit1_kitN_kitF"), [0, 5, 10]);
        assert_eq!(kit_token_starts("pants_kit3"), [6]);
        // A token after a non-token `kit` still counts.
        assert_eq!(kit_token_starts("kit1kit2_kit3"), [9]);
        for stem in ["skit1", "kit1kit2", "kitNx", "KIT1", "pants"] {
            assert_eq!(kit_token_starts(stem), Vec::<usize>::new(), "{stem}");
        }
    }

    #[test]
    fn a_legacy_token_is_u0_three_letters_or_digits_p_or_g_and_a_digit_as_written() {
        let token = |start, keeper, digit| LegacyKitToken {
            start,
            keeper,
            digit,
        };
        assert_eq!(legacy_kit_tokens("kit6_u0827p0.dds"), [token(5, false, 0)]);
        assert_eq!(legacy_kit_tokens("Common/u0XXXp1/"), [token(7, false, 1)]);
        assert_eq!(legacy_kit_tokens("u0714g1"), [token(0, true, 1)]);
        assert_eq!(legacy_kit_tokens("./u0714g9.dds"), [token(2, true, 9)]);
        // Two in one path, each found, left to right.
        assert_eq!(
            legacy_kit_tokens("u0XXXp2/kit1_u0827g0.dds"),
            [token(0, false, 2), token(13, true, 0)]
        );
        // A match starting inside the one before it is not taken.
        assert_eq!(legacy_kit_tokens("u0u0Zg1p1"), [token(0, true, 1)]);
        for text in [
            // Capitals: the upgrader leaves it as it is.
            "U0714P1",
            "u0714P1",
            // A `g` with no digit.
            "u0XXXg_back",
            // Two characters, not three.
            "u0XXp1",
            "u0XXXp",
            "u1XXXp1",
            "u0X_Xp1",
            "kit1_back",
        ] {
            assert_eq!(legacy_kit_tokens(text), [], "{text}");
        }
    }

    #[test]
    fn a_variant_stem_respells_the_first_token_alone() {
        assert_eq!(variant_stem("pants_kit1", 3), Some("pants_kit3".to_owned()));
        assert_eq!(
            variant_stem("kit3_a-kit1", 9),
            Some("kit9_a-kit1".to_owned())
        );
        assert_eq!(variant_stem("skit1_kit2", 4), Some("skit1_kit4".to_owned()));
        // The fallback respells like a variant.
        assert_eq!(variant_stem("pants_kitF", 2), Some("pants_kit2".to_owned()));
        assert_eq!(variant_stem("pants", 1), None);
    }

    #[test]
    fn the_token_goes_with_the_delimiter_before_it_or_else_the_one_after() {
        let cases = [
            ("boots_kit1", "boots"),
            ("boots_kitF", "boots"),
            ("kit1_boots", "boots"),
            ("face_high-kitN", "face_high"),
            ("a_kit2_boots", "a_boots"),
            // Two different delimiters show which one goes.
            ("a-kit2_boots", "a_boots"),
            ("kit2-a_boots", "a_boots"),
            ("x.kitN.y", "x.y"),
            ("kit1", ""),
            ("bootskit1", "bootskit1"),
        ];
        for (stem, expected) in cases {
            assert_eq!(without_kit_token(stem), expected, "{stem}");
        }
        assert!(matches!(without_kit_token("boots"), Cow::Borrowed("boots")));
    }

    #[test]
    fn a_kitn_directory_names_the_set_of_the_same_named_files_under_its_member_siblings() {
        let members = [
            (
                "kitN/armor_bsm.dds",
                "kit2/armor_bsm.dds",
                KitToken::Variant(2),
            ),
            (
                "kitN/armor_bsm.dds",
                "kitF/armor_bsm.dds",
                KitToken::Fallback,
            ),
            // The members are one set by stem, whatever their extensions.
            ("kitN/a.dds", "kitF/a.png", KitToken::Fallback),
            (
                "jessie/kitN/a.dds",
                "Jessie/kit1/A.dds",
                KitToken::Variant(1),
            ),
            // The token may sit anywhere in the path, a folder name included.
            (
                "kitN/textures/a.dds",
                "kit1/textures/a.dds",
                KitToken::Variant(1),
            ),
        ];
        for (reference, path, token) in members {
            assert_eq!(kit_set_member(reference, path), Some(token), "{path}");
        }
        for (reference, path) in [
            // Another directory's set.
            ("kitN/a.dds", "jessie/kit1/a.dds"),
            // A member folder is spelled exactly, as the stem token is.
            ("kitN/a.dds", "KIT1/a.dds"),
            ("kitN/a.dds", "kitf/a.dds"),
            // The reference itself is no member, nor is another file name.
            ("kitN/a.dds", "kitN/a.dds"),
            ("kitN/a.dds", "kit1/b.dds"),
            // One `kitN` segment's place names the set: the directories match segment-wise.
            ("jessie/kitN/x/a.dds", "jessie/kit2/y/a.dds"),
            ("kitN/a.dds", "kit1/x/a.dds"),
        ] {
            assert_eq!(kit_set_member(reference, path), None, "{path}");
        }
    }

    #[test]
    fn a_kitn_stem_names_the_set_of_its_directory_s_member_stems() {
        assert_eq!(
            kit_set_member("pants_kitN.dds", "pants_kit1.dds"),
            Some(KitToken::Variant(1))
        );
        assert_eq!(
            kit_set_member("pants_kitN.dds", "pants_kitF.dds"),
            Some(KitToken::Fallback)
        );
        // The members are one set by stem, whatever their extensions.
        assert_eq!(
            kit_set_member("pants_kitN.dds", "pants_kit1.png"),
            Some(KitToken::Variant(1))
        );
        for (reference, path) in [
            // The directory matches, folded.
            ("jessie/pants_kitN.dds", "pants_kit1.dds"),
            // A member's reference is the reference's stem.
            ("pants_kitN.dds", "socks_kit1.dds"),
            // A path with no token names no set and is no member.
            ("pants.dds", "pants.dds"),
            ("pants_kitN.dds", "pants_kitN.dds"),
            ("pants_kit1.dds", "pants_kit2.dds"),
        ] {
            assert_eq!(kit_set_member(reference, path), None, "{path}");
        }
    }

    #[test]
    fn a_path_naming_the_kitf_fallback_finds_nothing() {
        for path in [
            "pants_kitF.dds",
            "kitF/a.dds",
            "jessie/kitF/a.dds",
            "kitF/textures/a.dds",
        ] {
            assert!(names_kit_fallback(path), "{path}");
        }
        for path in [
            "kitf/a.dds",
            "pants.dds",
            "kit1/a.dds",
            "pants_kitN.dds",
            "kitN/a.dds",
        ] {
            assert!(!names_kit_fallback(path), "{path}");
        }
    }
}
