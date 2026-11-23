//! The cells' top boxes: the pictures each names, and when the detail window's
//! picture cache reads a file again.

use super::*;

/// The folder that `cell` opens, which its export's pictures are read from.
fn export_folder(cell: &Cell) -> PathBuf {
    match &cell.open {
        Some(Open::Folder(folder)) => folder.clone(),
        other => panic!("{} opens {other:?}", cell.text),
    }
}

/// The picture `path` of the export in `export`.
fn picture(export: &Path, path: &str) -> PictureFile {
    PictureFile {
        export: export.to_owned(),
        path: path.to_owned(),
    }
}

// TC-GUI-31: a Full export with a logo, a kit whose main texture is `kit.dds`, and 22 of
// its 23 players with a portrait: 05's in his folder, 20 without, the others' in
// `Portraits/`.
#[test]
fn each_cell_s_box_names_the_pictures_the_compile_packs() {
    let temp = sandbox("view_grid_pictures");
    let root = temp.path();
    list_teams(root);
    let name = "co Full Pictures";
    for slot in 1..=23 {
        add_player(root, name, &format!("{slot:02} - P"));
    }
    let export = root.join("exports").join(name);
    let tracer_portrait =
        fs::read(tracer().join("Players/05 - The Chad Stormworks Player/portrait.dds")).unwrap();
    for slot in (1..=23).filter(|slot| *slot != 5) {
        fs::remove_file(export.join(format!("Players/{slot:02} - P/portrait.dds"))).unwrap();
        if slot != 20 {
            add_file(
                root,
                &format!("{name}/Portraits/{slot:02}.dds"),
                &tracer_portrait,
            );
        }
    }
    add_kit(root, name, "g1");
    complete(&export, PesVersion::Pes21, &[Requirement::Portraits]);
    assert!(export.join("logo.png").is_file(), "premise: the logo");
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    let export = export_folder(&team.label);
    let logo = Some(picture(&export, "logo.png"));
    let portrait = |slot: u8| match slot {
        5 => Some(picture(&export, "Players/05 - P/portrait.dds")),
        20 => None,
        _ => Some(picture(&export, &format!("Portraits/{slot:02}.dds"))),
    };
    assert_eq!(team.label.top, TopBox::Logo(logo.clone()));
    assert_eq!(
        kit(team, 0).unwrap().top,
        TopBox::Picture(picture(&export, "Kits/g1/kit.dds"))
    );
    for slot in [5, 20, 1] {
        let expected = TopBox::Player {
            portrait: portrait(slot),
            logo: logo.clone(),
        };
        assert_eq!(player(team, slot.into()).unwrap().top, expected, "{slot}");
    }
    let expected = TopBox::Portraits {
        portraits: (1..=23).map(portrait).collect(),
        logo,
    };
    assert_eq!(portraits(team).unwrap().top, expected);
}

/// The label's box of a referee export holding `files` besides its referee, `A`.
fn referee_label_box(test: &str, files: &[&str]) -> (TopBox, PathBuf) {
    let temp = sandbox(test);
    let root = temp.path();
    add_file(root, "refs Cup/players.txt", b"01 A\n");
    add_player(root, "refs Cup", "A");
    let marker = fs::read(tracer().join("Kits/g1/kit.dds")).unwrap();
    let logo = dds_convert::encode_png(&[255; 16 * 16 * 4], 16, 16).unwrap();
    for file in files {
        let bytes = if *file == "logo.png" { &logo } else { &marker };
        add_file(root, &format!("refs Cup/{file}"), bytes);
    }
    complete(&root.join("exports/refs Cup"), PesVersion::Pes21, &[]);
    let rows = checked_rows(root);
    let label = &row(&rows, "/refs/").label;
    (label.top.clone(), export_folder(label))
}

// TC-GUI-32
#[test]
fn a_referee_export_s_label_shows_its_marker_in_place_of_its_logo() {
    let (top, export) = referee_label_box("view_grid_ref_marker", &["ref_marker.dds", "logo.png"]);
    assert_eq!(top, TopBox::Logo(Some(picture(&export, "ref_marker.dds"))));
    let (top, _) = referee_label_box("view_grid_ref_no_logo", &[]);
    assert_eq!(top, TopBox::Logo(None));
}

// An export dropped whole (a team the teams list lacks) shows its pictures from its
// listing: its logo, its kit's texture, 03's portrait in his folder and 05's in
// `Portraits/`.
// TC-GUI-31
#[test]
fn a_dropped_export_s_boxes_name_the_pictures_of_its_listing() {
    let temp = sandbox("view_grid_dropped_pictures");
    let root = temp.path();
    list_teams(root);
    let name = "v Midcup Map";
    add_player(root, name, "03 - B");
    add_player(root, name, "05 - C");
    let export = root.join("exports").join(name);
    let own = export.join("Players/05 - C/portrait.dds");
    let portrait = fs::read(&own).unwrap();
    fs::remove_file(own).unwrap();
    add_file(root, &format!("{name}/Portraits/05.dds"), &portrait);
    add_kit(root, name, "g1");
    add_file(root, &format!("{name}/logo.png"), &portrait);
    let rows = checked_rows(root);
    let team = row(&rows, "/v/");
    assert_eq!(
        player(team, 3).unwrap().state,
        CellState::Blocked,
        "premise"
    );
    let export = export_folder(&team.label);
    let logo = Some(picture(&export, "logo.png"));
    assert_eq!(team.label.top, TopBox::Logo(logo.clone()));
    assert_eq!(
        kit(team, 0).unwrap().top,
        TopBox::Picture(picture(&export, "Kits/g1/kit.dds"))
    );
    for (slot, file) in [(3, "Players/03 - B/portrait.dds"), (5, "Portraits/05.dds")] {
        let expected = TopBox::Player {
            portrait: Some(picture(&export, file)),
            logo: logo.clone(),
        };
        assert_eq!(player(team, slot).unwrap().top, expected, "{slot}");
    }
}

/// Extracts the archive at `archive` into `folder`, which it creates.
fn unzip(archive: &Path, folder: &Path) {
    let mut zip = archives::Archive::open(archive).unwrap();
    let entries: Vec<String> = zip
        .entries()
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    for entry in entries {
        let into = folder.join(&entry);
        fs::create_dir_all(into.parent().unwrap()).unwrap();
        fs::write(&into, zip.read(&entry).unwrap()).unwrap();
    }
}

/// The paths of the files `top` names, in the order the box draws them; `None` where a
/// picture's spot is empty.
fn top_paths(top: &TopBox) -> Vec<Option<&str>> {
    fn path(file: &Option<PictureFile>) -> Option<&str> {
        file.as_ref().map(|file| file.path.as_str())
    }
    match top {
        TopBox::Empty => Vec::new(),
        TopBox::Picture(file) => vec![Some(file.path.as_str())],
        TopBox::Logo(logo) => vec![path(logo)],
        TopBox::Player { portrait, logo } => vec![path(portrait), path(logo)],
        TopBox::Portraits { portraits, logo } => {
            portraits.iter().map(path).chain([path(logo)]).collect()
        }
    }
}

/// Copies the tracer `.zip` fixture in as `root`'s `/co/` export and unzips it under
/// another team's name, as `/a/`'s folder.
fn zip_and_folder(root: &Path) {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources/egg Midcup Tracer.zip");
    fs::copy(&fixture, root.join("exports/co Midcup Tracer.zip")).unwrap();
    unzip(&fixture, &root.join("exports/a Midcup Tracer"));
}

// A `.zip` gets the folder's deep check: its cells show the folder form's states, not the
// partial-ok of a `.7z` (whose listing alone is read).
// TC-GUI-04
#[test]
fn a_zip_export_s_cells_show_the_folder_form_s_states() {
    let temp = sandbox("view_grid_zip_states");
    let root = temp.path();
    list_teams(root);
    zip_and_folder(root);
    let rows = checked_rows(root);
    let (zipped, foldered) = (row(&rows, "/co/"), row(&rows, "/a/"));
    for (zip_cell, folder_cell) in [
        (&zipped.label, &foldered.label),
        (player(zipped, 5).unwrap(), player(foldered, 5).unwrap()),
        (kit(zipped, 0).unwrap(), kit(foldered, 0).unwrap()),
        (portraits(zipped).unwrap(), portraits(foldered).unwrap()),
    ] {
        assert_eq!(
            (zip_cell.state, zip_cell.outline),
            (folder_cell.state, folder_cell.outline),
            "{}",
            zip_cell.text
        );
    }
}

// A `.zip` export's boxes name the same pictures the folder form of the export gives,
// each a `PictureFile` of the archive itself, read from its own entry.
// TC-GUI-31
#[test]
fn a_zip_export_s_boxes_name_the_pictures_the_folder_form_s_do() {
    let temp = sandbox("view_grid_zip_pictures");
    let root = temp.path();
    list_teams(root);
    zip_and_folder(root);
    let archive = root.join("exports/co Midcup Tracer.zip");
    let rows = checked_rows(root);
    let (zipped, foldered) = (row(&rows, "/co/"), row(&rows, "/a/"));
    for (zip_cell, folder_cell) in [
        (&zipped.label, &foldered.label),
        (player(zipped, 5).unwrap(), player(foldered, 5).unwrap()),
        (kit(zipped, 0).unwrap(), kit(foldered, 0).unwrap()),
        (portraits(zipped).unwrap(), portraits(foldered).unwrap()),
    ] {
        assert_eq!(
            top_paths(&zip_cell.top),
            top_paths(&folder_cell.top),
            "{}",
            zip_cell.text
        );
    }
    let TopBox::Logo(Some(logo)) = &zipped.label.top else {
        panic!("the label's box is {:?}", zipped.label.top);
    };
    assert_eq!(logo.export, archive, "the export is the zip itself");
    let mut tool = Tool::new();
    let egui_ctx = egui::Context::default();
    assert_eq!(read_size(&mut tool.detail, &egui_ctx, logo), [256, 256]);
}

// A `.7z` export's boxes stay empty: reading a picture out of a solid `.7z` means
// decompressing it whole. Its cells are the shallow check's partial-ok (TC-GUI-04).
// TC-GUI-31
#[test]
fn a_7z_export_s_boxes_stay_empty() {
    let temp = sandbox("view_grid_7z_pictures");
    let root = temp.path();
    list_teams(root);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sources/egg Midcup Tracer.7z"),
        root.join("exports/co Midcup Tracer.7z"),
    )
    .unwrap();
    let rows = checked_rows(root);
    let team = row(&rows, "/co/");
    for cell in [
        &team.label,
        player(team, 5).unwrap(),
        kit(team, 0).unwrap(),
        portraits(team).unwrap(),
    ] {
        assert_eq!(cell.top, TopBox::Empty, "{}", cell.text);
        assert_eq!(cell.state, CellState::PartialOk, "{}", cell.text);
    }
}

/// `file`'s texture size once the detail window's worker has read it: a hang guard of
/// 120 s, not a speed bound.
fn read_size(detail: &mut GridDetail, ctx: &egui::Context, file: &PictureFile) -> [usize; 2] {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        detail.pictures.receive(ctx);
        if let Some(texture) = detail.pictures.texture(ctx, file) {
            return texture.size();
        }
        assert!(Instant::now() < deadline, "{file:?} not read in 120 s");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_new_check_s_rows_have_the_pictures_read_again() {
    let temp = sandbox("view_pictures_new_rows");
    let ctx = tool_context(temp.path(), "");
    let egui_ctx = egui::Context::default();
    let mut tool = Tool::new();
    let png = |width, height| {
        let pixels = vec![255; 4 * width as usize * height as usize];
        dds_convert::encode_png(&pixels, width, height).unwrap()
    };
    let export = temp.path().join("exports/egg Midcup Tracer");
    fs::write(export.join("logo.png"), png(4, 2)).unwrap();
    let file = picture(&export, "logo.png");
    assert_eq!(read_size(&mut tool.detail, &egui_ctx, &file), [4, 2]);
    fs::write(export.join("logo.png"), png(2, 4)).unwrap();
    assert_eq!(
        read_size(&mut tool.detail, &egui_ctx, &file),
        [4, 2],
        "premise: kept while the rows stay"
    );
    let (release_check, check_until) = unbounded();
    tool.live_check = LiveCheck::in_flight(check_until, vec![label_only_row()]);
    release_check.send(()).unwrap();
    finish_check(&mut tool, &ctx);
    assert_eq!(read_size(&mut tool.detail, &egui_ctx, &file), [2, 4]);
}
