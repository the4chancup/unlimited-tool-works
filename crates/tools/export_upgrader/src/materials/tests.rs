use studio_core::Scope;
use teams_list::TeamName;

use super::*;

/// The resolved team, `/co/`.
fn team() -> Identity {
    Identity::Team {
        name: TeamName::new("co").unwrap(),
    }
}

/// A referee export's identity.
fn refs() -> Identity {
    Identity::Referees
}

/// The resolved team `/vg/`.
fn vg() -> Identity {
    Identity::Team {
        name: TeamName::new("vg").unwrap(),
    }
}

/// A `.mtl` of one material holding one sampler per `paths` entry.
fn mtl(paths: &[&str]) -> Vec<u8> {
    let samplers: String = paths
            .iter()
            .map(|path| {
                format!("        <sampler name=\"DiffuseMap\" path=\"{path}\" srgb=\"1\" minfilter=\"linear\" magfilter=\"linear\" uaddr=\"clamp\" vaddr=\"clamp\"/>\n")
            })
            .collect();
    format!(
            "<materialset>\n    <material name=\"face_phong\" shader=\"Wrinkle_Face\">\n{samplers}    </material>\n</materialset>\n"
        )
        .into_bytes()
}

/// An FMDL of the `konami_highneck` fixture with its four texture
/// entries set from `entries` (`(directory, file_name)` pairs, in
/// table order).
fn fmdl(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut file = FmdlFile::read(include_bytes!(
        "../../../../libs/fmdl/tests/fixtures/konami_highneck.fmdl"
    ))
    .unwrap();
    let mut entries = entries.iter();
    fmdl::ops::paths::rewrite_texture_paths(&mut file, |path| {
        if let Some((directory, name)) = entries.next() {
            path.directory = (*directory).to_owned();
            path.file_name = (*name).to_owned();
        }
    })
    .unwrap();
    file.write()
}

/// The `(directory, file_name)` entries of `output`'s written FMDL.
fn fmdl_paths(rewrites: &MaterialRewrites, output: &str) -> Vec<(String, String)> {
    let (_, bytes) = rewrites
        .written
        .iter()
        .find(|(path, _)| path == output)
        .expect("the FMDL was rewritten");
    let file = FmdlFile::read(bytes).unwrap();
    fmdl::ops::paths::texture_paths(&file)
        .unwrap()
        .iter()
        .map(|path| (path.directory.clone(), path.file_name.clone()))
        .collect()
}

#[test]
fn an_fmdl_common_reference_becomes_the_stem_and_the_link() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "hair.dds",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "hair.dds".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/hair.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_shared_fmdl_common_subfolder_links_below_the_folder() {
    let boots = fmdl(&[(
        "/Assets/pes16/model/character/common/000/Boots/sourceimages/",
        "x.dds",
    )]);
    let rewrites = plan(
        &[("boots", "Boots/Crocs/boots.fmdl")],
        &["Common/Boots/x.dds"],
        &[("boots", &boots)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Boots/Crocs/boots.fmdl")[0],
        ("./Boots/".to_owned(), "x.dds".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Boots/Crocs/Boots/x.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_common_reference_with_a_studio_token_beside_the_legacy_one_links_the_set() {
    // The real VGL26 layout: the file's own `kit1_` prefix reads as the
    // author's variant, so the respelled reference is the set's only token.
    let boots = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "kit1_u0827p0.dds",
    )]);
    let rewrites = plan(
        &[("boots", "Boots/VIVLOS/boots.fmdl")],
        &[
            "Common/kit_tex1_kit1.dds",
            "Common/kit_tex1_kit2.dds",
            "Common/kit_tex1_kit3.dds",
            "Common/kit_tex1_kit4.dds",
        ],
        &[("boots", &boots)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Boots/VIVLOS/boots.fmdl")[0],
        ("./".to_owned(), "kit_tex1_kitN.dds".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Boots/VIVLOS/kit_tex1_kitN.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_common_kit_reference_links_the_set() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/common/000/u0XXXp0/sourceimages/",
        "armor_bsm",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &["Common/kit1/armor_bsm.dds"],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./kitN/".to_owned(), "armor_bsm".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/kitN/armor_bsm.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_common_keeper_reference_respells_to_gk_kit() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "u0XXXg1.dds",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &["Common/gk_kit1.dds"],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "gk_kit1.dds".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/gk_kit1.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_common_reference_whose_texture_is_missing_is_kept() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "hair.dds",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("team_common_folder/".to_owned(), "hair.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(rewrites.findings.len(), 1);
    assert_eq!(
        rewrites.findings[0].code.code.as_ref(),
        "common_reference_kept"
    );
    assert_eq!(
        rewrites.findings[0].context,
        vec![(
            "reference".to_owned(),
            "/Assets/pes16/model/character/common/000/sourceimages/hair.dds".to_owned()
        )]
    );
}

#[test]
fn an_fmdl_common_reference_the_folder_claims_is_kept() {
    // `hair.dds` is in `Common/` but the folder holds a texture of its
    // stem: the link would clash — kept and reported.
    let face = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "hair.dds",
    )]);
    let rewrites = plan(
        &[
            (
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            ),
            ("Faces/XXX07 - A/hair.dds", "Players/07 - A/hair.dds"),
        ],
        &["Common/hair.dds"],
        &[
            ("Faces/XXX07 - A/face_high.fmdl", &face),
            ("Faces/XXX07 - A/hair.dds", b"h"),
        ],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("team_common_folder/".to_owned(), "hair.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn an_fmdl_common_reference_in_another_letter_case_respells_the_marker() {
    let face = fmdl(&[(
        "/ASSETS/PES16/MODEL/CHARACTER/COMMON/000/sourceimages/",
        "hair.dds",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("team_common_folder/".to_owned(), "hair.dds".to_owned())
    );
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn an_fmdl_in_a_subfolder_naming_a_common_subfolder_is_kept() {
    // `realkit/x.fmdl` naming `common/000/Boots/…`: the path below the
    // model folder and the mirrored link cannot agree — kept.
    let kit = fmdl(&[(
        "/Assets/pes16/model/character/common/000/Boots/sourceimages/",
        "x.dds",
    )]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/realkit/x.fmdl",
            "Players/07 - A/realkit/x.fmdl",
        )],
        &["Common/Boots/x.dds"],
        &[("Faces/XXX07 - A/realkit/x.fmdl", &kit)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/realkit/x.fmdl")[0],
        ("team_common_folder/Boots/".to_owned(), "x.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn the_fox_common_reference_needs_a_folder_but_sourceimages_and_sourceimages() {
    assert_eq!(
        reference_into_fox_common("/Assets/pes16/model/character/common/000/sourceimages/"),
        Some(("000", ""))
    );
    // Any folder but the game's own `sourceimages` is the team's, however
    // long: a team ID, `XXX`, a name.
    for (directory, marker) in [
        (
            "/Assets/pes16/model/character/common/00/sourceimages/",
            "00",
        ),
        (
            "/Assets/pes16/model/character/common/0000/sourceimages/",
            "0000",
        ),
        (
            "/Assets/pes16/model/character/common/ikachan/sourceimages/",
            "ikachan",
        ),
    ] {
        assert_eq!(
            reference_into_fox_common(directory),
            Some((marker, "")),
            "{directory}"
        );
    }
    // The game's own `sourceimages` is no team's, and the rest must end in
    // `sourceimages/`.
    for directory in [
        "/Assets/pes16/model/character/common/sourceimages/",
        "/Assets/pes16/model/character/common/000/not_the_marker_end_at_all/",
    ] {
        assert_eq!(reference_into_fox_common(directory), None, "{directory}");
    }
}

#[test]
fn fmdl_entries_the_classes_keep_are_not_written() {
    // The kept spellings: the Studio short form for a `dummy_` stem, the
    // game's kit texture folder, and a directory below the game's own
    // `sourceimages/` (kept as written, not `common_folder/`), twice to set
    // the fixture's fourth entry.
    let face = fmdl(&[
        ("team_common_folder/", "dummy_nrm.dds"),
        (
            "/Assets/pes16/model/character/uniform/texture/",
            "u0XXXp1.dds",
        ),
        (
            "/Assets/pes16/model/character/common/sourceimages/sub/",
            "zero.dds",
        ),
        (
            "/Assets/pes16/model/character/common/sourceimages/sub/",
            "zero.dds",
        ),
    ]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    // Nothing respelled, nothing linked, nothing reported: no rewrite.
    assert!(rewrites.written.is_empty());
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_directory_without_sourceimages_keeps_its_path_and_names_team_folder() {
    // `common/000/foo/` is no team-Common reference (no `sourceimages/`
    // end) but its folder is the old spelling the compiler refuses: kept,
    // the folder respelled, nothing linked or reported. The game's own
    // `common/sourceimages/` beside it is written `common_folder/`.
    let face = fmdl(&[
        ("/Assets/pes16/model/character/common/000/foo/", "a.dds"),
        (
            "/Assets/pes16/model/character/common/sourceimages/",
            "zero.dds",
        ),
    ]);
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    // The first two entries: `fmdl` keeps the base model's own after them.
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[..2],
        [
            (
                "/Assets/pes16/model/character/common/team_folder/foo/".to_owned(),
                "a.dds".to_owned()
            ),
            ("common_folder/".to_owned(), "zero.dds".to_owned()),
        ]
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_other_directory_becomes_the_model_s_own_folder() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/face/real/00000/sourceimages/",
        "skin.dds",
    )]);
    // `skin.dds` in the folder: `./`, no link.
    let rewrites = plan(
        &[
            (
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            ),
            ("Faces/XXX07 - A/skin.dds", "Players/07 - A/skin.dds"),
        ],
        &[],
        &[
            ("Faces/XXX07 - A/face_high.fmdl", &face),
            ("Faces/XXX07 - A/skin.dds", b"s"),
        ],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "skin.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    // `skin.dds` only in `Common/`: `./` and the link.
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &["Common/skin.dds"],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "skin.dds".to_owned())
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/skin.dds.common".to_owned()]
    );
    // Nowhere: `./`, no link, no finding.
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "skin.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_kit_name_links_when_a_member_sits_in_the_folder() {
    let face = fmdl(&[(
        "/Assets/pes16/model/character/face/real/00000/sourceimages/",
        "u0805p0.dds",
    )]);
    let rewrites = plan(
        &[
            (
                "Faces/XXX07 - A/face_high.fmdl",
                "Players/07 - A/face_high.fmdl",
            ),
            ("Faces/XXX07 - A/kit1.dds", "Players/07 - A/kit1.dds"),
        ],
        &[],
        &[
            ("Faces/XXX07 - A/face_high.fmdl", &face),
            ("Faces/XXX07 - A/kit1.dds", b"k"),
        ],
    );
    // `kit1.dds` in the folder is a member of `kitN`'s set: `./` `kitN`,
    // no link.
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("./".to_owned(), "kitN.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_fmdl_keeps_its_common_references() {
    let boots = fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "x.dds",
    )]);
    let rewrites = plan(
        &[("common", "Common/boots.fmdl")],
        &["Common/x.dds"],
        &[("common", &boots)],
    );
    assert_eq!(
        fmdl_paths(&rewrites, "Common/boots.fmdl")[0],
        ("team_common_folder/".to_owned(), "x.dds".to_owned())
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_that_does_not_read_is_copied_as_it_is() {
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", b"fmdl")],
    );
    assert!(rewrites.written.is_empty());
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn an_fmdl_the_lift_refuses_keeps_its_rewrite_and_finding() {
    // Mesh 0's material index dangling: `FmdlFile::read` accepts the
    // file, `Model::from_file` refuses it.
    let mut file = FmdlFile::read(&fmdl(&[(
        "/Assets/pes16/model/character/common/000/sourceimages/",
        "hair.dds",
    )]))
    .unwrap();
    file.meshes[0].material_instance_id = u16::try_from(file.material_instances.len()).unwrap();
    let face = file.write();
    let rewrites = plan(
        &[(
            "Faces/XXX07 - A/face_high.fmdl",
            "Players/07 - A/face_high.fmdl",
        )],
        &[],
        &[("Faces/XXX07 - A/face_high.fmdl", &face)],
    );
    // The lift failed after the texture rewrite: the file is written
    // with it, and the finding the rewrite made is reported.
    assert_eq!(
        fmdl_paths(&rewrites, "Players/07 - A/face_high.fmdl")[0],
        ("team_common_folder/".to_owned(), "hair.dds".to_owned())
    );
    assert_eq!(codes(&rewrites), ["common_reference_kept"]);
}

/// `material_references` for the team `/co/` over `copies` and `common`,
/// each `.mtl`'s bytes behind `read`; a listed copy without bytes is a
/// test bug.
fn plan(copies: &[(&str, &str)], common: &[&str], bytes: &[(&str, &[u8])]) -> MaterialRewrites {
    plan_for(&team(), copies, common, bytes)
}

/// `plan` for an export of `identity`.
fn plan_for(
    identity: &Identity,
    copies: &[(&str, &str)],
    common: &[&str],
    bytes: &[(&str, &[u8])],
) -> MaterialRewrites {
    let copies: Vec<(String, String)> = copies
        .iter()
        .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
        .collect();
    let common: Vec<String> = common.iter().map(|file| (*file).to_owned()).collect();
    let bytes: BTreeMap<&str, &[u8]> = bytes.iter().copied().collect();
    material_references(&copies, &common, identity, |path| {
        bytes
            .get(path)
            .map(|bytes| Ok(bytes.to_vec()))
            .unwrap_or_else(|| panic!("a listed copy has no bytes: {path}"))
    })
}

#[test]
fn a_local_reference_s_kit_magic_is_respelled() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &[],
        &[("face", &mtl(&["./skin_u0758p0.dds"]))],
    );
    assert_eq!(rewrites.links.len(), 0);
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(text.contains("path=\"./skin_kitN.dds\""), "{text}");
}

#[test]
fn a_game_directory_reference_is_left_as_written() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &[],
        &[(
            "face",
            &mtl(&["model/character/uniform/texture/u0814p1.dds"]),
        )],
    );
    // Nothing changed: no rewrite at all.
    assert!(rewrites.written.is_empty());
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn the_used_set_marks_every_folder_a_non_dummy_reference_reaches() {
    // `used` feeds the `.dds`/`.ftex` pair rule: a local or a game
    // directory reference marks its folded stem in every folder its
    // lookup reaches (the file's folder up to the model folder, or the
    // `./sub` one alone); a `dummy_` stem in either place is the game's,
    // marking nothing.
    let rewrites = plan(
        &[("a", "Players/07 - A/realkit/face.mtl")],
        &[],
        &[(
            "a",
            &mtl(&[
                "shins.dds",
                "./worn/skin.ftex",
                "model/character/uniform/texture/u0814p1.dds",
                "model/character/uniform/texture/dummy_pad.dds",
                "./dummy_eye.ftex",
            ]),
        )],
    );
    let expected: BTreeSet<(String, String)> = [
        ("players/07 - a".to_owned(), "shins".to_owned()),
        ("players/07 - a/realkit".to_owned(), "shins".to_owned()),
        ("players/07 - a/worn".to_owned(), "skin".to_owned()),
        ("players/07 - a".to_owned(), "u0814p1".to_owned()),
        ("players/07 - a/realkit".to_owned(), "u0814p1".to_owned()),
    ]
    .into_iter()
    .collect();
    assert_eq!(rewrites.used, expected);
}

#[test]
fn a_kit_stand_in_s_old_spelling_is_written_the_studio_way_and_nothing_else_is() {
    for (name, studio) in [
        ("dummy_kit.dds", "team_kit.dds"),
        ("dummy_gk_kit.dds", "team_gk_kit.dds"),
        ("dummy_kit_srm.dds", "team_kit_srm.dds"),
        ("dummy_gk_kit_back.dds", "team_gk_kit_back.dds"),
        // Letter cases fold; the stem is written lowercase, the extension kept.
        ("Dummy_Kit_SRM.dds", "team_kit_srm.dds"),
        ("DUMMY_GK_KIT.DDS", "team_gk_kit.DDS"),
        // An FMDL texture entry, which carries no extension.
        ("dummy_kit", "team_kit"),
        ("Dummy_GK_Kit", "team_gk_kit"),
    ] {
        assert_eq!(studio_kit_name(name), Some(studio.to_owned()), "{name}");
    }
    for name in [
        "dummy_kitten.dds",
        "dummy_gk_kitten.dds",
        "dummy_nrm.dds",
        "dummy_srm.dds",
        "dummy_bsm.dds",
        "team_kit.dds",
        "team_kitten.dds",
        "hair.dds",
    ] {
        assert_eq!(studio_kit_name(name), None, "{name}");
    }
}

#[test]
fn a_common_keeper_reference_names_the_respelled_file() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &["Common/gk_kit1.dds"],
        &[(
            "face",
            &mtl(&["model/character/uniform/common/XXX/u0XXXg1.dds"]),
        )],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(text.contains("path=\"./gk_kit1.dds\""), "{text}");
    assert_eq!(
        rewrites.links,
        vec!["Players/07 - A/gk_kit1.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_dummy_stem_gets_its_marker_respelled_alone() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &[],
        &[(
            "face",
            &mtl(&["model/character/uniform/common/XXX/dummy_nrm.dds"]),
        )],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/dummy_nrm.dds\""),
        "{text}"
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_dummy_stem_in_a_sub_keeps_the_sub_as_written() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &[],
        &[(
            "face",
            &mtl(&["model/character/uniform/common/XXX/u0XXXp0/dummy_kit.dds"]),
        )],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/u0XXXp0/team_kit.dds\""),
        "{text}"
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_kit_reference_links_the_set_s_members() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &["Common/kit1.dds", "Common/kit2.dds"],
        &[(
            "face",
            &mtl(&["model/character/uniform/common/XXX/u0XXXp0.dds"]),
        )],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(text.contains("path=\"./kitN.dds\""), "{text}");
    assert_eq!(
        rewrites.links,
        vec!["Players/07 - A/kitN.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_kit_set_reference_with_a_member_in_the_folder_is_kept() {
    let rewrites = plan(
        &[
            ("face", "Players/07 - A/face.mtl"),
            ("kit1", "Players/07 - A/kit1.dds"),
        ],
        &["Common/kit1.dds", "Common/kit2.dds"],
        &[
            (
                "face",
                &mtl(&["model/character/uniform/common/XXX/u0XXXp0.dds"]),
            ),
            ("kit1", b"dds"),
        ],
    );
    assert_eq!(rewrites.links.len(), 0);
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/kitN.dds\""),
        "{text}"
    );
    assert_eq!(rewrites.findings.len(), 1);
    assert_eq!(
        rewrites.findings[0].code.code.as_ref(),
        "common_reference_kept"
    );
}

#[test]
fn a_shared_folder_mtl_respells_a_local_reference() {
    let rewrites = plan(
        &[("boots", "Boots/Crocs/boots.mtl")],
        &[],
        &[("boots", &mtl(&["./u0XXXp1.dds"]))],
    );
    let text = written_text(&rewrites, "Boots/Crocs/boots.mtl");
    assert!(text.contains("path=\"./kit1.dds\""), "{text}");
}

#[test]
fn a_shared_folder_mtl_reference_to_a_missing_common_file_is_kept() {
    let rewrites = plan(
        &[("boots", "Boots/Crocs/boots.mtl")],
        &[],
        &[(
            "boots",
            &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
        )],
    );
    assert!(rewrites.links.is_empty());
    let text = written_text(&rewrites, "Boots/Crocs/boots.mtl");
    assert!(
        text.contains("path=\"team_common_folder/hair.dds\""),
        "{text}"
    );
    assert_eq!(rewrites.findings.len(), 1);
    assert_eq!(
        rewrites.findings[0].code.code.as_ref(),
        "common_reference_kept"
    );
    assert_eq!(
        rewrites.findings[0].context,
        vec![(
            "reference".to_owned(),
            "model/character/uniform/common/XXX/hair.dds".to_owned()
        )]
    );
}

#[test]
fn two_references_in_one_mtl_link_their_own_common_files() {
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &["Common/hair.dds", "Common/jessie/hair.dds"],
        &[(
            "face",
            &mtl(&[
                "model/character/uniform/common/XXX/hair.dds",
                "model/character/uniform/common/XXX/jessie/hair.dds",
            ]),
        )],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(text.contains("path=\"./hair.dds\""), "{text}");
    assert!(text.contains("path=\"./jessie/hair.dds\""), "{text}");
    assert_eq!(
        rewrites.links,
        [
            "Players/07 - A/hair.dds.common".to_owned(),
            "Players/07 - A/jessie/hair.dds.common".to_owned(),
        ]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_common_mtl_keeps_its_common_references() {
    let rewrites = plan(
        &[("mtl", "Common/hair.mtl")],
        &["Common/hair.dds"],
        &[(
            "mtl",
            &mtl(&["model/character/uniform/common/XXX/hair.dds"]),
        )],
    );
    let text = written_text(&rewrites, "Common/hair.mtl");
    assert!(
        text.contains("path=\"team_common_folder/hair.dds\""),
        "{text}"
    );
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_named_common_segment_links_as_the_team_s_common() {
    // `common/vg/` is the team's Common as `XXX` is: the compiler reads
    // any segment as the team's own there.
    let face = mtl(&["model/character/uniform/common/vg/env.dds"]);
    let rewrites = plan_for(
        &vg(),
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/env.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("./env.dds"), "{out}");
    assert_eq!(rewrites.links, ["Players/07 - A/env.dds.common".to_owned()]);
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_named_common_segment_kept_is_respelled_team_folder() {
    let face = mtl(&["model/character/uniform/common/VG/fbm_s.dds"]);
    let rewrites = plan_for(
        &vg(),
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &[],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("team_common_folder/fbm_s.dds"), "{out}");
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn a_common_mtl_s_segment_of_another_length_is_respelled_team_folder() {
    let file = mtl(&["model/character/uniform/common/refs/refkit/texture.dds"]);
    let rewrites = plan_for(
        &vg(),
        &[("Common/refkit.mtl", "Common/refkit.mtl")],
        &[],
        &[("Common/refkit.mtl", &file)],
    );
    let out = written_text(&rewrites, "Common/refkit.mtl");
    assert!(
        out.contains("team_common_folder/refkit/texture.dds"),
        "{out}"
    );
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn model_folder_mode_writes_team_folder_too() {
    // No team to name: the kept references' folder after `common/` is
    // `team_folder` all the same, whatever the input spelled.
    let file = mtl(&[
        "model/character/uniform/common/vg/x.dds",
        "model/character/uniform/common/abc/x.dds",
    ]);
    let rewrites = plan_for(
        &Identity::ModelFolders,
        &[("k0431/x.mtl", "Boots/k0431/x.mtl")],
        &[],
        &[("k0431/x.mtl", &file)],
    );
    let out = written_text(&rewrites, "Boots/k0431/x.mtl");
    assert_eq!(out.matches("team_common_folder/x.dds").count(), 2, "{out}");
}

/// The one output path's `.mtl` text, parsed back.
fn written_text(rewrites: &MaterialRewrites, output: &str) -> String {
    let (_, bytes) = rewrites
        .written
        .iter()
        .find(|(path, _)| path == output)
        .expect("the .mtl was rewritten");
    String::from_utf8(bytes.clone()).unwrap()
}

#[test]
fn a_reference_into_common_becomes_the_bare_name_and_the_link() {
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            ("Faces/XXX07 - A/face.fmdl", "Players/07 - A/face.fmdl"),
        ],
        &["Common/hair.dds"],
        &[
            ("Faces/XXX07 - A/face.mtl", &face),
            ("Faces/XXX07 - A/face.fmdl", b"x"),
        ],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("./hair.dds"), "{out}");
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/hair.dds.common".to_owned()]
    );
    // The marker's own spelling and case likewise.
    let face = mtl(&["MODEL/Character/Uniform/Common/714/Hair.DDS"]);
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("./Hair.DDS"), "{out}");
    // `Common/hair.png` is found by stem and the link names its file.
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/hair.png"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/hair.png.common".to_owned()]
    );
}

#[test]
fn a_common_subfolder_reference_links_at_the_path_below_the_folder() {
    let face = mtl(&["model/character/uniform/common/XXX/jessie/hair.dds"]);
    let rewrites = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            (
                "Faces/XXX07 - A/realkit/x.mtl",
                "Players/07 - A/realkit/x.mtl",
            ),
        ],
        &["Common/jessie/hair.dds"],
        &[
            ("Faces/XXX07 - A/face.mtl", &face),
            ("Faces/XXX07 - A/realkit/x.mtl", &face),
        ],
    );
    // The root `.mtl` gets the path below the model folder and the
    // mirrored link; the subfolder `.mtl`'s is kept, `???` respelled.
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("./jessie/hair.dds"), "{out}");
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/jessie/hair.dds.common".to_owned()]
    );
    let out = written_text(&rewrites, "Players/07 - A/realkit/x.mtl");
    assert!(out.contains("team_common_folder/jessie/hair.dds"), "{out}");
    let kept = rewrites
        .findings
        .iter()
        .find(|m| m.code.code.as_ref() == "common_reference_kept")
        .expect("common_reference_kept expected");
    assert_eq!(
        kept.context,
        vec![(
            "reference".to_owned(),
            "model/character/uniform/common/XXX/jessie/hair.dds".to_owned()
        )]
    );
    assert_eq!(
        kept.scope,
        Scope::File {
            export_id: INPUT,
            path: path_scope("Faces/XXX07 - A/realkit/x.mtl")
        }
    );
}

#[test]
fn a_folder_file_of_the_stem_keeps_the_reference() {
    // `Players/07 - A/` already holds `hair.png`: the link would clash
    // or lose to the nearer file — kept.
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            ("Faces/XXX07 - A/hair.png", "Players/07 - A/hair.png"),
        ],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("team_common_folder/hair.dds"), "{out}");
    assert!(rewrites.links.is_empty());
    // From a subfolder `.mtl` the bare name looks up to the model folder,
    // so the root's `hair.png` keeps it too.
    let rewrites = plan(
        &[
            (
                "Faces/XXX07 - A/realkit/apc.mtl",
                "Players/07 - A/realkit/apc.mtl",
            ),
            ("Faces/XXX07 - A/hair.png", "Players/07 - A/hair.png"),
        ],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/realkit/apc.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/realkit/apc.mtl");
    assert!(out.contains("team_common_folder/hair.dds"), "{out}");
    assert!(rewrites.links.is_empty());
    // A model's link of that stem claims no texture: the link is made.
    let rewrites = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            (
                "Faces/XXX07 - A/hair.model.common",
                "Players/07 - A/hair.model.common",
            ),
        ],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    assert_eq!(
        rewrites.links,
        ["Players/07 - A/hair.dds.common".to_owned()]
    );
    // Two `.mtl` files naming one `Common/` file share its one link.
    let two = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            ("Faces/XXX07 - A/other.mtl", "Players/07 - A/other.mtl"),
        ],
        &["Common/hair.dds"],
        &[
            ("Faces/XXX07 - A/face.mtl", &face),
            ("Faces/XXX07 - A/other.mtl", &face),
        ],
    );
    assert_eq!(two.links, ["Players/07 - A/hair.dds.common".to_owned()]);
    for output in ["Players/07 - A/face.mtl", "Players/07 - A/other.mtl"] {
        assert!(
            written_text(&two, output).contains("./hair.dds"),
            "{output}"
        );
    }
}

#[test]
fn an_mtl_naming_nothing_stays_as_it_is_an_unparsable_one_is_unmigrated() {
    // `./face.dds` and `model/character/face/common/` are not the team's
    // Common — nothing changes, so nothing is written. `bad.mtl` does
    // not parse even dropping stray text: unmigrated content, its parse
    // error in the context (step 7); the copy loop still carries it.
    let unchanged = mtl(&["./face.dds", "model/character/face/common/face_eyelash.dds"]);
    let bad = b"mtl".to_vec();
    let plan = plan(
        &[
            ("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl"),
            ("Faces/XXX07 - A/bad.mtl", "Players/07 - A/bad.mtl"),
        ],
        &["Common/hair.dds"],
        &[
            ("Faces/XXX07 - A/face.mtl", &unchanged),
            ("Faces/XXX07 - A/bad.mtl", &bad),
        ],
    );
    assert!(plan.written.is_empty());
    assert!(plan.links.is_empty());
    assert_eq!(codes(&plan), ["unmigrated_content"]);
    let finding = &plan.findings[0];
    assert_eq!(
        finding.scope,
        Scope::File {
            export_id: INPUT,
            path: path_scope("Faces/XXX07 - A/bad.mtl"),
        }
    );
    assert_eq!(
        finding.context[0],
        ("reason".to_owned(), "mtl_unparsable".to_owned())
    );
    assert!(
        finding.context.iter().any(|(key, _)| key == "error"),
        "{:?}",
        finding.context
    );
}

#[test]
fn stray_text_is_dropped_and_the_set_written_back_as_it_came() {
    // Step 7's clean (the real `72117 - Lisa`'s `apc.mtl` `\`): the text
    // is dropped on the tolerant read, the set written back — wrapped
    // again when it was — and the file reported. Neither reference
    // changes, so the write-back is the stray text's alone.
    let stray = mtl(&["./skin.dds"]);
    let stray = String::from_utf8(stray)
        .unwrap()
        .replace("    </material>", "\\\n    </material>")
        .into_bytes();
    let wrapped = wezlib::compress(&stray);
    let rewrites = plan(
        &[
            ("Faces/XXX07 - A/apc.mtl", "Players/07 - A/apc.mtl"),
            ("Faces/XXX07 - A/wrapped.mtl", "Players/07 - A/wrapped.mtl"),
        ],
        &[],
        &[
            ("Faces/XXX07 - A/apc.mtl", &stray),
            ("Faces/XXX07 - A/wrapped.mtl", &wrapped),
        ],
    );
    for (output, was_wrapped) in [
        ("Players/07 - A/apc.mtl", false),
        ("Players/07 - A/wrapped.mtl", true),
    ] {
        let (_, bytes) = rewrites
            .written
            .iter()
            .find(|(path, _)| path == output)
            .unwrap_or_else(|| panic!("{output} was not written back"));
        assert_eq!(wezlib::is_wrapped(bytes), was_wrapped, "{output}");
        let set = MaterialSet::read(bytes).expect("the written set parses");
        assert_eq!(set.materials.len(), 1, "{output}");
    }
    assert_eq!(
        codes(&rewrites),
        ["mtl_stray_text_dropped", "mtl_stray_text_dropped"]
    );
}

#[test]
fn the_common_file_must_match_both_folder_and_stem() {
    // `Common/` holds `beard.dds` (the folder, not the stem) and
    // `jessie/hair.dds` (the stem, not the folder): `hair` is kept.
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/beard.dds", "Common/jessie/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let out = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(out.contains("team_common_folder/hair.dds"), "{out}");
    assert!(rewrites.links.is_empty());
}

#[test]
fn two_common_files_named_from_one_folder_both_link() {
    let face = mtl(&[
        "model/character/uniform/common/XXX/hair.dds",
        "model/character/uniform/common/XXX/beard.dds",
    ]);
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/beard.dds", "Common/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    assert_eq!(
        rewrites.links,
        [
            "Players/07 - A/beard.dds.common".to_owned(),
            "Players/07 - A/hair.dds.common".to_owned()
        ]
    );
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_shared_folder_mtl_keeps_the_reference_with_team_folder() {
    // A `.mtl` of a shared `Boots/`/`Gloves/` folder keeps its Common
    // references as written but `team_folder` — no finding and no link
    // (the pre-Fox compile reads no shared folder's `.common` links).
    let boots = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[("Boots/k0430 - Crocs/boots.mtl", "Boots/Crocs/boots.mtl")],
        &["Common/hair.dds"],
        &[("Boots/k0430 - Crocs/boots.mtl", &boots)],
    );
    let out = written_text(&rewrites, "Boots/Crocs/boots.mtl");
    assert!(out.contains("team_common_folder/hair.dds"), "{out}");
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty());
}

#[test]
fn a_non_texture_of_the_stem_in_common_is_no_link_target() {
    // `Common/hair.mtl` shares the stem but is no texture: kept and
    // warned, no link.
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/hair.mtl"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn two_textures_of_the_stem_in_one_common_directory_keep_the_reference() {
    // `Common/hair.dds` beside `Common/hair.png`: both textures of the
    // stem in the directory, so no single file the link could spell —
    // kept and warned.
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/hair.dds", "Common/hair.png"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(
        rewrites
            .findings
            .iter()
            .map(|m| m.code.code.as_ref())
            .collect::<Vec<_>>(),
        ["common_reference_kept"]
    );
}

#[test]
fn a_rewritten_mtl_keeps_the_wesys_wrap() {
    let face = wezlib::compress(&mtl(&["model/character/uniform/common/XXX/hair.dds"]));
    let rewrites = plan(
        &[("Faces/XXX07 - A/face.mtl", "Players/07 - A/face.mtl")],
        &["Common/hair.dds"],
        &[("Faces/XXX07 - A/face.mtl", &face)],
    );
    let (_, bytes) = rewrites
        .written
        .iter()
        .find(|(path, _)| path == "Players/07 - A/face.mtl")
        .expect("the .mtl was rewritten");
    assert!(wezlib::is_wrapped(bytes));
    let out = String::from_utf8(wezlib::decompress(bytes).unwrap()).unwrap();
    assert!(out.contains("./hair.dds"), "{out}");
}

/// The finding codes of `rewrites`, in order.
fn codes(rewrites: &MaterialRewrites) -> Vec<&str> {
    rewrites
        .findings
        .iter()
        .map(|m| m.code.code.as_ref())
        .collect()
}

#[test]
fn a_referee_common_reference_his_folder_holds_names_it_there() {
    let face = mtl(&["model/character/uniform/common/XXX/hair.dds"]);
    let rewrites = plan_for(
        &refs(),
        &[
            ("face", "Players/arbiter/face.mtl"),
            ("hair", "Players/arbiter/hair.dds"),
        ],
        &[],
        &[("face", &face), ("hair", b"h")],
    );
    let text = written_text(&rewrites, "Players/arbiter/face.mtl");
    assert!(text.contains("path=\"./hair.dds\""), "{text}");
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    // A team's folder holding the stem keeps the reference, as before.
    let rewrites = plan(
        &[
            ("face", "Players/07 - A/face.mtl"),
            ("hair", "Players/07 - A/hair.dds"),
        ],
        &[],
        &[("face", &face), ("hair", b"h")],
    );
    let text = written_text(&rewrites, "Players/07 - A/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/hair.dds\""),
        "{text}"
    );
    assert!(rewrites.links.is_empty());
    assert_eq!(codes(&rewrites), ["common_reference_kept"]);
}

#[test]
fn a_referee_common_subfolder_reference_names_the_folder_s_subfolder() {
    let face = mtl(&["model/character/uniform/common/XXX/kit/sock.dds"]);
    let rewrites = plan_for(
        &refs(),
        &[
            ("face", "Players/arbiter/face.mtl"),
            ("sock", "Players/arbiter/kit/sock.dds"),
        ],
        &[],
        &[("face", &face), ("sock", b"s")],
    );
    let text = written_text(&rewrites, "Players/arbiter/face.mtl");
    assert!(text.contains("path=\"./kit/sock.dds\""), "{text}");
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    // From a subfolder's `.mtl`, `./kit/` would name `kit/kit/`: the
    // team rule keeps the reference.
    let rewrites = plan_for(
        &refs(),
        &[
            ("face", "Players/arbiter/kit/face.mtl"),
            ("sock", "Players/arbiter/kit/sock.dds"),
        ],
        &[],
        &[("face", &face), ("sock", b"s")],
    );
    let text = written_text(&rewrites, "Players/arbiter/kit/face.mtl");
    assert!(
        text.contains("path=\"team_common_folder/kit/sock.dds\""),
        "{text}"
    );
    assert_eq!(codes(&rewrites), ["common_reference_kept"]);
}

#[test]
fn a_referee_common_reference_his_folder_lacks_links_the_export_s_common() {
    let face = mtl(&["model/character/uniform/common/XXX/moe.dds"]);
    let rewrites = plan_for(
        &refs(),
        &[("face", "Players/arbiter/face.mtl")],
        &["Common/moe.dds"],
        &[("face", &face)],
    );
    let text = written_text(&rewrites, "Players/arbiter/face.mtl");
    assert!(text.contains("path=\"./moe.dds\""), "{text}");
    assert_eq!(
        rewrites.links,
        ["Players/arbiter/moe.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
}

#[test]
fn a_referee_bare_reference_only_common_holds_is_linked() {
    let face = mtl(&["./whistle.dds"]);
    let rewrites = plan_for(
        &refs(),
        &[("face", "Players/arbiter/face.mtl")],
        &["Common/whistle.dds"],
        &[("face", &face)],
    );
    // The reference is already the Studio spelling: no rewrite.
    assert!(rewrites.written.is_empty());
    assert_eq!(
        rewrites.links,
        ["Players/arbiter/whistle.dds.common".to_owned()]
    );
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
    // A team's bare reference gets no link.
    let rewrites = plan(
        &[("face", "Players/07 - A/face.mtl")],
        &["Common/whistle.dds"],
        &[("face", &face)],
    );
    assert!(rewrites.written.is_empty());
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
}

#[test]
fn a_referee_bare_reference_his_folder_holds_gets_no_link() {
    let face = mtl(&["./skin.dds"]);
    let rewrites = plan_for(
        &refs(),
        &[
            ("face", "Players/arbiter/face.mtl"),
            ("skin", "Players/arbiter/skin.dds"),
        ],
        &["Common/skin.dds"],
        &[("face", &face), ("skin", b"s")],
    );
    assert!(rewrites.written.is_empty());
    assert!(rewrites.links.is_empty());
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
}

#[test]
fn a_referee_fmdl_names_his_folder_s_texture_and_links_a_bare_common_one() {
    let boots = fmdl(&[
        (
            "/Assets/pes16/model/character/common/000/sourceimages/",
            "arbiter.dds",
        ),
        ("./", "whistle"),
        ("", "horn"),
    ]);
    let rewrites = plan_for(
        &refs(),
        &[
            ("boots", "Players/arbiter/boots.fmdl"),
            ("arbiter", "Players/arbiter/arbiter.dds"),
        ],
        &["Common/horn.dds", "Common/whistle.dds"],
        &[("boots", &boots), ("arbiter", b"a")],
    );
    let paths = fmdl_paths(&rewrites, "Players/arbiter/boots.fmdl");
    assert_eq!(paths[0], ("./".to_owned(), "arbiter.dds".to_owned()));
    assert_eq!(paths[1], ("./".to_owned(), "whistle".to_owned()));
    assert_eq!(paths[2], ("".to_owned(), "horn".to_owned()));
    // An empty directory is a bare reference too.
    assert_eq!(
        rewrites.links,
        [
            "Players/arbiter/horn.dds.common".to_owned(),
            "Players/arbiter/whistle.dds.common".to_owned()
        ]
    );
    assert!(rewrites.findings.is_empty(), "{:?}", codes(&rewrites));
}

#[test]
fn a_kept_common_reference_writes_team_folder_in_either_mode() {
    // The kept reference's folder after `common/` is `team_folder` in
    // model-folder mode as in a team's run, its kit magic still
    // respelled, the missing file still reported.
    let face = mtl(&["model/character/uniform/common/XXX/u0XXXp1/missing.dds"]);
    for identity in [Identity::ModelFolders, team()] {
        let rewrites = plan_for(
            &identity,
            &[("face", "Players/15 - Snuffy/face.mtl")],
            &[],
            &[("face", &face)],
        );
        let text = written_text(&rewrites, "Players/15 - Snuffy/face.mtl");
        assert!(
            text.contains("path=\"team_common_folder/kit1/missing.dds\""),
            "{text}"
        );
        assert_eq!(codes(&rewrites), ["common_reference_kept"]);
    }
}
