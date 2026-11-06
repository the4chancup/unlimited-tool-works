//! The carried-over folders, `Portraits/`, `Collars/` and `Logo/`
//! (spec/export_upgrader.md "Carries over"). Portraits are renamed to the
//! only name the compiler accepts, with a bytes check against the face
//! folder's own portrait. Collars are kept as they are. The logo members go
//! into the root `logo.png` and, only when it is a different drawing,
//! `logo_small.png`: the compile regenerates a small logo that is a
//! downscale. The drawings are compared through the compiler's own
//! resampler, so the verdict matches what the compile makes. What cannot be
//! placed is left for the run's fallback, which reports it as unmigrated
//! content or drops it as a leftover.

use std::collections::{BTreeMap, BTreeSet};

use aesthetics_export::{FileKind, PlayerSlot, classify};
use dds_convert::{Decoded, SourceFormat};
use studio_core::{Disposition, Message, Scope};

use crate::messages::{Code, INPUT, UnmigratedReason, read_failed, tool_message, unmigrated};
use crate::players::{file_name, path_scope};
use crate::source::SourceFailure;

/// The channel distance past which two pixels differ (of 255), as plan step
/// 7's rule names it.
const TOLERANCE: u8 = 32;
/// The share of differing pixels past which the old small logo is a
/// different drawing: measured on the 94 real `Logo/` pairs with this
/// resampler, every downscale scored under it and every deliberate variant
/// over (plan step 7).
const LOGO_SMALL_DISTINCT_SHARE: usize = 40;

/// What step 7 makes of an export's `Portraits/`, `Collars/` and `Logo/`:
/// the files copied, the input files they account for, and the findings on
/// the input.
#[derive(Debug, Default)]
pub(crate) struct CarriedFiles {
    /// Input path → output path, copied as they are.
    pub(crate) copies: Vec<(String, String)>,
    /// Input files accounted for; the run's fallback reports every other one.
    pub(crate) consumed: BTreeSet<String>,
    /// Input files the step recognized as leftovers (step 10): the run's
    /// fallback reports each `leftover_dropped`, the run clean, rather than
    /// `unmigrated_content`.
    pub(crate) leftovers: BTreeSet<String>,
    /// Findings on the input, in input-path order, a file's own findings in
    /// the order met.
    pub(crate) findings: Vec<Message>,
}

/// The role of an old `Logo/` member, by its name's ending.
#[derive(Clone, Copy)]
enum LogoRole {
    /// `_r_ll.png`, 512²: becomes `logo.png`.
    Large = 0,
    /// `_r_l.png`, 256²: the fallback's first choice, else dropped.
    Medium = 1,
    /// `_r.png`, 128²: `logo_small.png` when a distinct drawing, else
    /// dropped.
    Small = 2,
}

/// The rest of `path` below `top` (its name in any letter case), or `None`.
pub(crate) fn under<'a>(path: &'a str, top: &str) -> Option<&'a str> {
    let (head, rest) = path.split_once('/')?;
    head.eq_ignore_ascii_case(top).then_some(rest)
}

/// What an old portrait's name gives when it is the shape step 7 names: the
/// slot's two digits and the extension as written, for a name the old format
/// admitted: `player_` in that letter case, then anything,
/// then two ASCII digits making a slot 01-23 right before a three-letter
/// extension `classify` reads as a texture (`player_XXX05.dds` → `05`,
/// `player_71407.dds` → `07`, `player_XXX08.jpg` → `08`). A name that fails
/// it (`Player_XXX05.dds`, `player_XXX05.jpeg`, `player_XXX24.dds`,
/// `XXX17.png`) gives `None`: refused, the file never reached the game.
fn portrait_name(file_name: &str) -> Option<(&str, &str)> {
    let (stem, ext) = file_name.rsplit_once('.')?;
    if ext.len() != 3
        || !stem.starts_with("player_")
        || !matches!(classify(file_name), FileKind::Texture)
    {
        return None;
    }
    let nn = stem.get(stem.len().checked_sub(2)?..)?;
    let slot = nn
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| nn.parse::<u8>().ok())
        .flatten()?;
    PlayerSlot::new(slot).map(|_| (nn, ext))
}

/// The role a `Logo/` file's name gives, the longest ending first.
fn logo_role(file_name: &str) -> Option<LogoRole> {
    let lowered = file_name.to_ascii_lowercase();
    if lowered.ends_with("_r_ll.png") {
        Some(LogoRole::Large)
    } else if lowered.ends_with("_r_l.png") {
        Some(LogoRole::Medium)
    } else if lowered.ends_with("_r.png") {
        Some(LogoRole::Small)
    } else {
        None
    }
}

/// `decoded`'s top mip resampled to 128 × 128 (a side already 128 is left
/// unfiltered by `resize`; no squaring: every real member is square).
fn at_128(decoded: &Decoded) -> Vec<u8> {
    dds_convert::resize(&decoded.mips[0], decoded.width, decoded.height, 128, 128)
        .expect("a decoded image's top mip resizes: nonzero size and a packed RGBA buffer")
}

/// Whether two RGBA pixels differ: any channel further apart than
/// `TOLERANCE`, two pixels both fully transparent being equal whatever
/// their other channels hold.
fn pixel_differs(a: &[u8; 4], b: &[u8; 4]) -> bool {
    if a[3] == 0 && b[3] == 0 {
        return false;
    }
    a.iter().zip(b).any(|(a, b)| a.abs_diff(*b) > TOLERANCE)
}

/// How many of the 128 × 128 compared pixels differ between `large` and
/// `small` (step 7's downscale-or-drawing test), each resampled there when
/// it is not already.
fn differing_pixels(large: &Decoded, small: &Decoded) -> usize {
    let (large, small) = (at_128(large), at_128(small));
    let (a, b) = (large.as_chunks::<4>().0, small.as_chunks::<4>().0);
    a.iter()
        .zip(b.iter())
        .filter(|(a, b)| pixel_differs(a, b))
        .count()
}

/// The small logo is a different drawing, and `logo_small.png`, when more
/// than `LOGO_SMALL_DISTINCT_SHARE` percent of the compared pixels differ.
fn is_distinct_drawing(differing: usize, total: usize) -> bool {
    differing * 100 > LOGO_SMALL_DISTINCT_SHARE * total
}

/// The differing pixels' share of `total`, rounded half up, formatted like
/// `30%`.
fn share_text(differing: usize, total: usize) -> String {
    format!("{}%", (differing * 200 + total) / (2 * total))
}

/// What step 7 makes of `files`' `Portraits/`, `Collars/` and `Logo/` for a
/// team export whose face folders' portraits are `face_portraits` (slot →
/// input path).
pub(crate) fn carried_files(
    files: &[String],
    face_portraits: &BTreeMap<u8, String>,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, SourceFailure>,
) -> CarriedFiles {
    let mut carried = CarriedFiles::default();
    let mut failures: BTreeMap<String, Vec<Message>> = BTreeMap::new();

    // `Portraits/` (step 7): a file directly in it whose name the old format
    // admitted becomes `Portraits/NN.<ext>` — the first of two giving one `NN`
    // by input path. A name the check refused, a subfolder's file included,
    // is a leftover (step 10); the second file of a slot stays unmigrated
    // content, two candidates for one place, `other` the file that took it.
    // A slot whose face folder carries a different portrait is
    // `portrait_conflict`: the compiler's deep pass would refuse the export.
    let mut slots: BTreeMap<String, &String> = BTreeMap::new();
    for file in files {
        let Some(rest) = under(file, "portraits") else {
            continue;
        };
        let Some((nn, ext)) = (!rest.contains('/')).then(|| portrait_name(rest)).flatten() else {
            carried.leftovers.insert(file.clone());
            continue;
        };
        if let Some(first) = slots.get(nn) {
            let first = (*first).clone();
            failures.entry(file.clone()).or_default().push(unmigrated(
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(file),
                },
                Disposition::DropFile,
                UnmigratedReason::PortraitTaken,
                Some(first),
                vec![],
            ));
            carried.consumed.insert(file.clone());
            continue;
        }
        slots.insert(nn.to_owned(), file);
        carried
            .copies
            .push((file.clone(), format!("Portraits/{nn}.{ext}")));
        carried.consumed.insert(file.clone());
        let slot = nn
            .parse::<u8>()
            .expect("the two ASCII digits `portrait_name` checked always parse");
        let Some(face_file) = face_portraits.get(&slot) else {
            continue;
        };
        let theirs = match read(file) {
            Ok(bytes) => bytes,
            // A read failure is no conflict; the copy loop's read reports
            // the file.
            Err(_) => continue,
        };
        let ours = match read(face_file) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };
        if theirs != ours {
            failures.entry(file.clone()).or_default().push(tool_message(
                Code::PortraitConflict,
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(file),
                },
                Disposition::Keep,
                vec![("slot", nn.to_owned())],
            ));
        }
    }

    // `Collars/` (step 7): every file below it, at its relative path; the
    // shared validation judges what it holds. `Common/` is carried the
    // same way. Step 10's stray rule first, a classify-unknown file no
    // package ever read being a leftover rather than a copied file the
    // strict output check would refuse the folder for.
    for file in files {
        for (top, output) in [("collars", "Collars"), ("common", "Common")] {
            if let Some(rest) = under(file, top) {
                if crate::players::is_stray(file_name(file), None) {
                    carried.leftovers.insert(file.clone());
                } else {
                    carried
                        .copies
                        .push((file.clone(), format!("{output}/{rest}")));
                    carried.consumed.insert(file.clone());
                }
            }
        }
    }

    // `Logo/` (step 7): a member is a file directly in it ending
    // `_r_ll.png`, `_r_l.png` or `_r.png` in any case, the first of one role
    // by input path — consumed whatever it decodes as. A member is usable
    // when it decodes as a PNG; one that does not is unmigrated content,
    // its error in the context. Any other file, and a second member of one
    // role (`other` the kept member), are unmigrated content too (step 10).
    let mut members: [Option<(&String, Option<Decoded>)>; 3] = [None, None, None];
    for file in files {
        let Some(rest) = under(file, "logo") else {
            continue;
        };
        if crate::players::is_stray(file_name(file), None) {
            carried.leftovers.insert(file.clone());
            continue;
        }
        let non_member = |carried: &mut CarriedFiles,
                          failures: &mut BTreeMap<String, Vec<Message>>,
                          reason: UnmigratedReason,
                          other: Option<String>| {
            failures.entry(file.clone()).or_default().push(unmigrated(
                Scope::File {
                    export_id: INPUT,
                    path: path_scope(file),
                },
                Disposition::DropFile,
                reason,
                other,
                vec![],
            ));
            carried.consumed.insert(file.clone());
        };
        if rest.contains('/') {
            non_member(
                &mut carried,
                &mut failures,
                UnmigratedReason::LogoMemberInvalid,
                None,
            );
            continue;
        }
        let Some(role) = logo_role(rest) else {
            non_member(
                &mut carried,
                &mut failures,
                UnmigratedReason::LogoMemberInvalid,
                None,
            );
            continue;
        };
        let index = role as usize;
        if let Some((first, _)) = &members[index] {
            let first = (*first).clone();
            non_member(
                &mut carried,
                &mut failures,
                UnmigratedReason::LogoRoleTaken,
                Some(first),
            );
            continue;
        }
        match read(file) {
            Err(failure) => {
                failures.entry(file.clone()).or_default().push(read_failed(
                    INPUT,
                    failure,
                    Disposition::DropFile,
                ));
                carried.consumed.insert(file.clone());
                members[index] = Some((file, None));
            }
            Ok(bytes) => {
                carried.consumed.insert(file.clone());
                match dds_convert::decode(&bytes, SourceFormat::Png) {
                    Ok(decoded) => members[index] = Some((file, Some(decoded))),
                    Err(error) => {
                        failures.entry(file.clone()).or_default().push(unmigrated(
                            Scope::File {
                                export_id: INPUT,
                                path: path_scope(file),
                            },
                            Disposition::DropFile,
                            UnmigratedReason::TextureUndecodable,
                            None,
                            vec![("error", error.to_string())],
                        ));
                        members[index] = Some((file, None));
                    }
                }
            }
        }
    }
    let usable = |role: LogoRole| {
        members[role as usize]
            .as_ref()
            .and_then(|(file, decoded)| decoded.as_ref().map(|decoded| (*file, decoded)))
    };
    let (large, medium, small) = (
        usable(LogoRole::Large),
        usable(LogoRole::Medium),
        usable(LogoRole::Small),
    );
    if let Some((large_path, large)) = large {
        // `logo.png` is the large member's bytes (the medium is dropped
        // silently: the compiler regenerates it). A small member that is a
        // different drawing becomes `logo_small.png`; a downscale is
        // dropped, both with the measured share in the report.
        carried
            .copies
            .push((large_path.clone(), "logo.png".to_owned()));
        if let Some((small_path, small)) = small {
            let differing = differing_pixels(large, small);
            let total = 128 * 128;
            let carried_small = is_distinct_drawing(differing, total);
            if carried_small {
                carried
                    .copies
                    .push((small_path.clone(), "logo_small.png".to_owned()));
            }
            failures
                .entry(small_path.clone())
                .or_default()
                .push(tool_message(
                    if carried_small {
                        Code::LogoSmallCarried
                    } else {
                        Code::LogoSmallDropped
                    },
                    Scope::File {
                        export_id: INPUT,
                        path: path_scope(small_path),
                    },
                    Disposition::Keep,
                    vec![("share", share_text(differing, total))],
                ));
        }
    } else if let Some((path, _)) = medium.or(small) {
        // No usable large member: the largest usable one becomes `logo.png`
        // — and there is nothing left to compare a small one against.
        carried.copies.push((path.clone(), "logo.png".to_owned()));
        failures.entry(path.clone()).or_default().push(tool_message(
            Code::LogoFallback,
            Scope::File {
                export_id: INPUT,
                path: path_scope(path),
            },
            Disposition::Keep,
            vec![],
        ));
    }

    // The findings in input-path order (`files` is sorted): the passes met
    // them file by file.
    for file in files {
        if let Some(messages) = failures.remove(file.as_str()) {
            carried.findings.extend(messages);
        }
    }
    carried.findings.extend(failures.into_values().flatten());
    carried
}

#[cfg(test)]
mod tests {
    use studio_core::Severity;

    use super::*;

    /// A real `Logo/` member's PNG bytes, for any member a test needs.
    const PNG: &[u8] = include_bytes!("../tests/fixtures/logos/OMG VGL26/emblem_0XXX_r_ll.png");

    /// `carried_files` over `listing` with `face_portraits` and each listed
    /// file's bytes behind `read`; a listed path without bytes is a test
    /// bug, so it panics.
    fn plan(
        listing: &[&str],
        face_portraits: &[(u8, &str)],
        bytes: &[(&str, &[u8])],
    ) -> CarriedFiles {
        let bytes: BTreeMap<&str, &[u8]> = bytes.iter().copied().collect();
        let files: Vec<String> = listing.iter().map(|file| (*file).to_owned()).collect();
        let face_portraits: BTreeMap<u8, String> = face_portraits
            .iter()
            .map(|(slot, file)| (*slot, (*file).to_owned()))
            .collect();
        carried_files(&files, &face_portraits, |path| {
            bytes
                .get(path)
                .map(|bytes| Ok(bytes.to_vec()))
                .unwrap_or_else(|| panic!("a listed file has no bytes: {path}"))
        })
    }

    /// A 128 × 128 image of `color`.
    fn image(color: [u8; 4]) -> Decoded {
        image_with(color, &[])
    }

    /// A `base`-colored 128 × 128 image with `changes` pixels replaced at
    /// indexes.
    fn image_with(base: [u8; 4], changes: &[(usize, [u8; 4])]) -> Decoded {
        let mut pixels = vec![base; 128 * 128];
        for (index, color) in changes {
            pixels[*index] = *color;
        }
        Decoded {
            width: 128,
            height: 128,
            mips: vec![pixels.into_iter().flatten().collect()],
            blocks: None,
            authored_mips: false,
            uncompressed: false,
        }
    }

    #[test]
    fn a_channel_difference_past_the_tolerance_counts() {
        let a = image([10, 10, 10, 255]);
        // 32 in one channel is not a difference; 33 is.
        let b = image_with([10, 10, 10, 255], &[(0, [42, 10, 10, 255])]);
        assert_eq!(differing_pixels(&a, &b), 0);
        let b = image_with([10, 10, 10, 255], &[(0, [43, 10, 10, 255])]);
        assert_eq!(differing_pixels(&a, &b), 1);
    }

    #[test]
    fn both_fully_transparent_pixels_are_equal() {
        let a = image_with([0, 0, 0, 255], &[(0, [255, 0, 0, 0])]);
        let b = image_with([0, 0, 0, 255], &[(0, [0, 255, 0, 0])]);
        assert_eq!(differing_pixels(&a, &b), 0);
        // Transparent on one side only differs.
        let b = image_with([0, 0, 0, 255], &[(0, [255, 0, 0, 255])]);
        assert_eq!(differing_pixels(&a, &b), 1);
    }

    #[test]
    fn the_share_line_sits_at_forty_percent() {
        let a = image([0, 0, 0, 255]);
        // 6,553 differing (39.996 %): dropped; 6,554 (40.002 %): carried.
        for (differing, expected) in [(6553, false), (6554, true)] {
            let changes: Vec<(usize, [u8; 4])> = (0..differing)
                .map(|index| (index, [255, 255, 255, 255]))
                .collect();
            let b = image_with([0, 0, 0, 255], &changes);
            let differing = differing_pixels(&a, &b);
            assert_eq!(
                is_distinct_drawing(differing, 128 * 128),
                expected,
                "{differing}"
            );
        }
    }

    #[test]
    fn exactly_the_share_is_not_a_distinct_drawing() {
        // The rule is strictly more than `LOGO_SMALL_DISTINCT_SHARE`
        // percent: exactly it is a downscale, past it another drawing.
        assert!(!is_distinct_drawing(4, 10));
        assert!(is_distinct_drawing(41, 100));
    }

    #[test]
    fn the_reported_share_rounds_half_up() {
        let total = 128 * 128;
        assert_eq!(share_text(4919, total), "30%");
        assert_eq!(share_text(8699, total), "53%");
        assert_eq!(share_text(8192, total), "50%");
    }

    #[test]
    fn the_portrait_name_grammar() {
        for (name, expected) in [
            ("player_XXX05.dds", Some(("05", "dds"))),
            ("player_71407.dds", Some(("07", "dds"))),
            ("player_09.dds", Some(("09", "dds"))),
            ("player_736917.dds", Some(("17", "dds"))),
            ("player_05.dds", Some(("05", "dds"))),
            ("player_XXX08.jpg", Some(("08", "jpg"))),
            ("Player_XXX05.dds", None),
            ("player_XXX05.jpeg", None),
            ("player_XXX24.dds", None),
            ("player_XXX00.dds", None),
            ("XXX17.jpg", None),
            ("73617.png", None),
            ("player_5.dds", None),
            ("portrait_05.dds", None),
            ("player_XXXab.dds", None),
            ("Vanilla vitamins.png", None),
            ("player_73906-.dds", None),
            ("XX17.jpg", None),
            ("Zoé1.dds", None),
            ("player_05.txt", None),
        ] {
            assert_eq!(portrait_name(name), expected, "{name}");
        }
    }

    #[test]
    fn a_logo_member_in_upper_case_has_its_role() {
        assert!(matches!(
            logo_role("EMBLEM_0XXX_R_LL.PNG"),
            Some(LogoRole::Large)
        ));
        assert!(matches!(
            logo_role("EMBLEM_0XXX_R_L.PNG"),
            Some(LogoRole::Medium)
        ));
        assert!(matches!(
            logo_role("EMBLEM_0XXX_R.PNG"),
            Some(LogoRole::Small)
        ));
    }

    #[test]
    fn equal_bytes_give_no_portrait_conflict_and_different_bytes_give_one() {
        let listing = &["Faces/XXX07 - A/portrait.dds", "Portraits/player_XXX07.dds"];
        let portraits = &[(7u8, "Faces/XXX07 - A/portrait.dds")];
        let equal = plan(
            listing,
            portraits,
            &[
                ("Faces/XXX07 - A/portrait.dds", b"same"),
                ("Portraits/player_XXX07.dds", b"same"),
            ],
        );
        assert_eq!(
            equal.copies,
            vec![(
                "Portraits/player_XXX07.dds".to_owned(),
                "Portraits/07.dds".to_owned()
            )]
        );
        assert!(
            equal
                .findings
                .iter()
                .all(|m| m.code.code.as_ref() != "portrait_conflict")
        );
        let different = plan(
            listing,
            portraits,
            &[
                ("Faces/XXX07 - A/portrait.dds", b"mine"),
                ("Portraits/player_XXX07.dds", b"theirs"),
            ],
        );
        let conflict = different
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "portrait_conflict")
            .expect("portrait_conflict expected");
        assert_eq!(conflict.severity, Severity::Error);
        assert_eq!(conflict.disposition, Disposition::Keep);
        assert_eq!(conflict.context, vec![("slot".to_owned(), "07".to_owned())]);
        assert_eq!(
            conflict.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Portraits/player_XXX07.dds")
            }
        );
    }

    #[test]
    fn a_stray_in_common_collars_or_the_logo_folder_is_a_leftover() {
        let plan = plan(
            &[
                "Collars/collar_120.fmdl",
                "Collars/face.mtl.backup",
                "Common/kit/sock.dds",
                "Common/materials.mtl.bak",
                "Logo/emblem_0XXX_r_ll.png",
                "Logo/emblem.psd",
            ],
            &[],
            &[
                ("Collars/collar_120.fmdl", b"collar"),
                ("Common/kit/sock.dds", b"sock"),
                ("Logo/emblem_0XXX_r_ll.png", PNG),
            ],
        );
        for (from, to) in [
            ("Collars/collar_120.fmdl", "Collars/collar_120.fmdl"),
            ("Common/kit/sock.dds", "Common/kit/sock.dds"),
            ("Logo/emblem_0XXX_r_ll.png", "logo.png"),
        ] {
            assert!(
                plan.copies.contains(&(from.to_owned(), to.to_owned())),
                "{to}"
            );
        }
        for stray in [
            "Collars/face.mtl.backup",
            "Common/materials.mtl.bak",
            "Logo/emblem.psd",
        ] {
            assert!(plan.leftovers.contains(stray), "{stray}");
            assert!(!plan.consumed.contains(stray), "{stray}");
        }
    }

    #[test]
    fn a_logo_entry_that_is_no_member_is_unmigrated_content() {
        // `foo.png` ends in no member role and `sub/foo.png` sits in a
        // subfolder: neither is a member (step 7) and neither is a stray —
        // unmigrated content (step 10), not a leftover.
        let plan = plan(
            &[
                "Logo/emblem_0XXX_r.png",
                "Logo/emblem_0XXX_r_l.png",
                "Logo/emblem_0XXX_r_ll.png",
                "Logo/foo.png",
                "Logo/sub/foo.png",
            ],
            &[],
            &[
                ("Logo/emblem_0XXX_r.png", PNG),
                ("Logo/emblem_0XXX_r_l.png", PNG),
                ("Logo/emblem_0XXX_r_ll.png", PNG),
                ("Logo/foo.png", b"stray"),
                ("Logo/sub/foo.png", b"stray"),
            ],
        );
        for member in [
            "Logo/emblem_0XXX_r.png",
            "Logo/emblem_0XXX_r_l.png",
            "Logo/emblem_0XXX_r_ll.png",
        ] {
            assert!(plan.consumed.contains(member), "{member}");
        }
        for file in ["Logo/foo.png", "Logo/sub/foo.png"] {
            assert!(plan.consumed.contains(file), "{file}");
            assert!(!plan.leftovers.contains(file), "{file}");
            let finding = plan
                .findings
                .iter()
                .find(|m| {
                    m.scope
                        == Scope::File {
                            export_id: INPUT,
                            path: path_scope(file),
                        }
                })
                .expect("unmigrated_content expected");
            assert_eq!(finding.code.code.as_ref(), "unmigrated_content");
            assert_eq!(
                finding.context,
                [("reason".to_owned(), "logo_member_invalid".to_owned())],
                "{file}"
            );
        }
    }

    #[test]
    fn a_second_member_of_one_logo_role_is_unmigrated_content() {
        // `emblem_0XXX_r.png` sorts before `team_r.png`: it keeps the
        // small role and the second is unmigrated, `other` the kept file.
        let plan = plan(
            &[
                "Logo/emblem_0XXX_r.png",
                "Logo/emblem_0XXX_r_ll.png",
                "Logo/team_r.png",
            ],
            &[],
            &[
                ("Logo/emblem_0XXX_r.png", PNG),
                ("Logo/emblem_0XXX_r_ll.png", PNG),
                ("Logo/team_r.png", PNG),
            ],
        );
        assert!(plan.consumed.contains("Logo/team_r.png"));
        let finding = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "unmigrated_content")
            .expect("unmigrated_content expected");
        assert_eq!(
            finding.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Logo/team_r.png")
            }
        );
        assert_eq!(
            finding.context,
            [
                ("reason".to_owned(), "logo_role_taken".to_owned()),
                ("other".to_owned(), "Logo/emblem_0XXX_r.png".to_owned()),
            ]
        );
    }

    #[test]
    fn a_large_member_that_does_not_decode_falls_back_to_the_medium() {
        let plan = plan(
            &[
                "Logo/emblem_0XXX_r.png",
                "Logo/emblem_0XXX_r_l.png",
                "Logo/emblem_0XXX_r_ll.png",
            ],
            &[],
            &[
                ("Logo/emblem_0XXX_r.png", PNG),
                ("Logo/emblem_0XXX_r_l.png", PNG),
                ("Logo/emblem_0XXX_r_ll.png", b"not a png"),
            ],
        );
        assert_eq!(
            plan.copies,
            [("Logo/emblem_0XXX_r_l.png".to_owned(), "logo.png".to_owned())]
        );
        // `unmigrated_content` at the `_r_ll` with its decode error in the
        // context; `logo_fallback` at the `_r_l` it fell back to; no small
        // logo is written or judged.
        let unmigrated = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "unmigrated_content")
            .expect("unmigrated_content expected");
        assert_eq!(
            unmigrated.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Logo/emblem_0XXX_r_ll.png")
            }
        );
        assert_eq!(
            unmigrated.context[0],
            ("reason".to_owned(), "texture_undecodable".to_owned())
        );
        assert!(
            unmigrated.context.iter().any(|(key, _)| key == "error"),
            "{:?}",
            unmigrated.context
        );
        let fallback = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "logo_fallback")
            .expect("logo_fallback expected");
        assert_eq!(
            fallback.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Logo/emblem_0XXX_r_l.png")
            }
        );
        assert!(
            plan.findings
                .iter()
                .all(|m| !m.code.code.as_ref().starts_with("logo_small"))
        );
    }

    #[test]
    fn both_larger_members_undecodable_fall_back_to_the_small_one() {
        let plan = plan(
            &[
                "Logo/emblem_0XXX_r.png",
                "Logo/emblem_0XXX_r_l.png",
                "Logo/emblem_0XXX_r_ll.png",
            ],
            &[],
            &[
                ("Logo/emblem_0XXX_r.png", PNG),
                ("Logo/emblem_0XXX_r_l.png", b"not a png"),
                ("Logo/emblem_0XXX_r_ll.png", b"not a png"),
            ],
        );
        assert_eq!(
            plan.copies,
            [("Logo/emblem_0XXX_r.png".to_owned(), "logo.png".to_owned())]
        );
        let unmigrated: Vec<&Message> = plan
            .findings
            .iter()
            .filter(|m| m.code.code.as_ref() == "unmigrated_content")
            .collect();
        assert_eq!(
            unmigrated.iter().map(|m| &m.scope).collect::<Vec<_>>(),
            [
                &Scope::File {
                    export_id: INPUT,
                    path: path_scope("Logo/emblem_0XXX_r_l.png")
                },
                &Scope::File {
                    export_id: INPUT,
                    path: path_scope("Logo/emblem_0XXX_r_ll.png")
                }
            ]
        );
        for finding in &unmigrated {
            assert_eq!(
                finding.context[0],
                ("reason".to_owned(), "texture_undecodable".to_owned())
            );
        }
        let fallback = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "logo_fallback")
            .expect("logo_fallback expected");
        assert_eq!(
            fallback.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Logo/emblem_0XXX_r.png")
            }
        );
    }

    #[test]
    fn two_portraits_giving_one_slot_keep_the_first() {
        let plan = plan(
            &["Portraits/player_71405.dds", "Portraits/player_XXX05.dds"],
            &[],
            &[
                ("Portraits/player_71405.dds", b"714"),
                ("Portraits/player_XXX05.dds", b"xxx"),
            ],
        );
        // `player_71405.dds` sorts before `player_XXX05.dds`: the first by
        // input path wins the slot. The second is unmigrated content — two
        // candidates for one place, no leftover (step 10) — `other` the
        // file that took it.
        assert_eq!(
            plan.copies,
            vec![(
                "Portraits/player_71405.dds".to_owned(),
                "Portraits/05.dds".to_owned()
            )]
        );
        assert!(plan.consumed.contains("Portraits/player_71405.dds"));
        assert!(plan.consumed.contains("Portraits/player_XXX05.dds"));
        assert!(!plan.leftovers.contains("Portraits/player_XXX05.dds"));
        let finding = plan
            .findings
            .iter()
            .find(|m| m.code.code.as_ref() == "unmigrated_content")
            .expect("unmigrated_content expected");
        assert_eq!(
            finding.scope,
            Scope::File {
                export_id: INPUT,
                path: path_scope("Portraits/player_XXX05.dds")
            }
        );
        assert_eq!(
            finding.context,
            [
                ("reason".to_owned(), "portrait_taken".to_owned()),
                ("other".to_owned(), "Portraits/player_71405.dds".to_owned()),
            ]
        );
    }

    #[test]
    fn a_portrait_the_old_check_refused_is_a_leftover() {
        // `XXX17.png` starts no `player_`, the subfolder's file is not
        // directly in `Portraits/`: each a leftover (step 10), not consumed.
        let plan = plan(
            &[
                "Portraits/player_XXX08.jpg",
                "Portraits/XXX17.png",
                "Portraits/sub/player_XXX05.dds",
            ],
            &[],
            &[],
        );
        assert_eq!(
            plan.copies,
            vec![(
                "Portraits/player_XXX08.jpg".to_owned(),
                "Portraits/08.jpg".to_owned()
            )]
        );
        for file in ["Portraits/XXX17.png", "Portraits/sub/player_XXX05.dds"] {
            assert!(plan.leftovers.contains(file), "{file}");
            assert!(!plan.consumed.contains(file), "{file}");
        }
    }
}
