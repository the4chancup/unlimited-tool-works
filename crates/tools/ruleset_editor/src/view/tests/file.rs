//! Opening and saving: a file Save wrote opens with no banner
//! (RE-FILE-01), a hand-edited logic shows the banner and is kept
//! byte for byte (RE-FILE-02), comments inside `RULESET` go at save
//! (RE-FILE-05), a failed write is reported with nothing beside the
//! file (RE-FILE-07), and the event's page and logo are kept outside
//! the logic (RE-FILE-09).

use super::*;

/// `dir`'s file names, sorted.
fn folder_contents(dir: &Path) -> Vec<String> {
    let mut inside: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    inside.sort();
    inside
}

// RE-FILE-01
#[test]
fn a_file_save_wrote_opens_with_no_banner() {
    let (ctx, _events, path, mut view) = official_opened("file_01");
    // Save is available on an unedited document.
    assert!(save_to(&mut view, &ctx, &path));

    let mut view = opened_view(&ctx, &path);
    let mut window = Headless::new();
    let frame = window.draw(&mut view, &ctx);
    assert!(
        view.document
            .as_ref()
            .is_some_and(|document| { matches!(document.logic, crate::document::Logic::Current) }),
        "the file opens in the forms with the current logic"
    );
    assert!(!contains_label(&frame.nodes, "checking code"), "no banner");
}

// RE-FILE-02
#[test]
fn a_modified_logic_file_shows_the_banner_and_save_keeps_its_code() {
    let dir = crate::test_support::scratch("file_02");
    let text = crate::test_support::modified_logic_file();
    let kept = aatf::file_sections(&text).logic.expect("the section");
    let path = write_file(&dir, "rules.4ccrules", &text);
    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &path);
    let frame = window.draw(&mut view, &ctx);
    assert!(
        contains_label(&frame.nodes, "checking code was changed by hand"),
        "the banner says the file's checking code is not this build's"
    );
    assert!(
        contains_label(&frame.nodes, "special rules may not behave"),
        "and that its special rules may not behave as the forms describe"
    );

    // An edit (General's name) and Save: the file's logic kept byte for
    // byte, the RULESET holding the edit.
    edit_ruleset(&mut view, |ruleset| ruleset.name = "Renamed".to_string());
    assert!(save_to(&mut view, &ctx, &path));
    let written = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        aatf::file_sections(&written)
            .logic
            .as_deref()
            .map(str::trim_end),
        Some(kept.trim_end()),
        "the file's own logic, byte for byte"
    );
    let rules = aatf::Rules::load(&written).expect("the saved file loads");
    assert_eq!(rules.ruleset().name, "Renamed", "the edit is in it");

    // Reopened: the banner again.
    let mut view = opened_view(&ctx, &path);
    let frame = window.draw(&mut view, &ctx);
    assert!(
        contains_label(&frame.nodes, "checking code was changed by hand"),
        "the banner again"
    );
    assert!(contains_label(&frame.nodes, "special rules may not behave"));
}

// RE-FILE-05
#[test]
fn a_save_with_no_edit_writes_the_same_values_and_no_comments() {
    // The DBC7 block with the comments inside its RULESET, assembled as
    // a file (current logic, the empty-checks stub).
    let aatf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf");
    let block = std::fs::read_to_string(aatf_dir.join("rulesets/dbc7.rhai")).unwrap();
    assert!(
        block.contains("// No card or position rule"),
        "premise: comments inside RULESET"
    );
    let logic = aatf::file_sections(&valid_file())
        .logic
        .expect("the current section");
    let stub = crate::test_support::CUSTOM_CHECKS_STUB;
    let text = format!("{}\n\n{}\n\n{}\n", block.trim_end(), logic.trim_end(), stub);
    let dir = crate::test_support::scratch("file_05");
    let path = write_file(&dir, "dbc7.4ccrules", &text);

    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &path);
    // "Save is available on an unedited document": the bottom bar's Save.
    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Save", &mut view, &ctx);
    let written = std::fs::read_to_string(&path).unwrap();
    let data = &written[..written.find("// ==== LOGIC").expect("the logic banner")];
    assert!(
        !data.contains("// No card or position rule"),
        "none of the comments is written"
    );
    let rules = aatf::Rules::load(&written).expect("the saved file loads");
    assert_eq!(
        rules.ruleset(),
        &aatf::Ruleset::from_data_block(&block).expect("the block reads"),
        "the ruleset reads back equal"
    );
}

// RE-FILE-09
#[test]
fn a_save_keeps_the_events_page_and_logo_out_of_the_logic() {
    // DBC7's block, which carries its event's page and logo, assembled
    // as a file (current logic, the empty-checks stub).
    let aatf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf");
    let block = std::fs::read_to_string(aatf_dir.join("rulesets/dbc7.rhai")).unwrap();
    let before = aatf::Ruleset::from_data_block(&block).expect("the block reads");
    assert!(
        before.page.is_some() && before.logo.is_some(),
        "premise: DBC7 carries a page and a logo"
    );
    let logic = aatf::file_sections(&valid_file())
        .logic
        .expect("the current section");
    let stub = crate::test_support::CUSTOM_CHECKS_STUB;
    let text = format!("{}\n\n{}\n\n{}\n", block.trim_end(), logic.trim_end(), stub);
    let dir = crate::test_support::scratch("file_09");
    let path = write_file(&dir, "dbc7.4ccrules", &text);

    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &path);
    let frame = window.draw(&mut view, &ctx);
    click_labeled(&mut window, &frame.nodes, "Save", &mut view, &ctx);
    let written = std::fs::read_to_string(&path).unwrap();

    let rules = aatf::Rules::load(&written).expect("the saved file loads");
    assert_eq!(rules.ruleset().page, before.page, "the page reads back");
    assert_eq!(rules.ruleset().logo, before.logo, "the logo reads back");

    let data = &written[..written.find("// ==== LOGIC").expect("the logic banner")];
    let lines: Vec<&str> = data.lines().collect();
    let name = lines
        .iter()
        .position(|line| line.starts_with("    name: "))
        .expect("the name line");
    assert!(
        lines[name + 1].starts_with("    page: "),
        "page follows name:\n{data}"
    );
    let end = lines
        .iter()
        .position(|line| *line == "};")
        .expect("the data block's end");
    assert!(
        lines[end - 1].starts_with("    logo: "),
        "logo is the last key:\n{data}"
    );

    let without: String = written
        .lines()
        .filter(|line| !line.starts_with("    page: ") && !line.starts_with("    logo: "))
        .map(|line| format!("{line}\n"))
        .collect();
    let bare = aatf::Rules::load(&without).expect("the file without them loads");
    assert_eq!(bare.ruleset().page, None, "premise: the page is removed");
    assert_eq!(bare.ruleset().logo, None, "premise: the logo is removed");
    assert_eq!(
        rules.logic(),
        bare.logic(),
        "the logic's identity ignores the two keys"
    );
}

// RE-GUI-02
#[test]
fn save_as_writes_a_copy_and_later_saves_follow_it() {
    let dir = crate::test_support::scratch("file_save_as");
    let a = write_file(&dir, "a.4ccrules", &valid_file());
    let (ctx, _events, mut window, mut view) = fresh();
    open_path(&mut view, &ctx, &a);
    let a_before = std::fs::read(&a).unwrap();

    // One checkbox edit (the at-most list's "Attacking Prowess"), then
    // File > Save as… to b.4ccrules: a.4ccrules untouched, b.4ccrules
    // holding the edit and the document's path moved to it.
    let frame = window.draw(&mut view, &ctx);
    click_nth(
        &mut window,
        &frame.nodes,
        "Attacking Prowess",
        1,
        &mut view,
        &ctx,
    );
    assert!(
        view.document.as_ref().expect("the document").is_dirty(),
        "premise: the click edited the ruleset"
    );
    let b = dir.join("b.4ccrules");
    save_as_via_menu(&mut window, &mut view, &ctx, &b);
    assert_file_unchanged(&a, &a_before);
    let rules = aatf::Rules::load(&std::fs::read_to_string(&b).expect("b.4ccrules was written"))
        .expect("b.4ccrules loads");
    assert_eq!(
        rules.ruleset(),
        &view.document.as_ref().expect("the document").ruleset,
        "b.4ccrules holds the edit"
    );
    assert_eq!(
        view.document.as_ref().unwrap().path.as_deref(),
        Some(b.as_path()),
        "the document's path is b.4ccrules"
    );

    // Another edit, then File > Save (the menu's own "Save", the
    // topmost: the bottom bar's shares the label) writes b.4ccrules and
    // leaves a.4ccrules alone.
    click_drawn_nth(&mut window, &mut view, &ctx, "Defensive Prowess", 1);
    let frame = window.draw(&mut view, &ctx);
    click_labeled(
        &mut window,
        &frame.nodes,
        &menu_label("File"),
        &mut view,
        &ctx,
    );
    let frame = window.draw(&mut view, &ctx);
    click_nth(&mut window, &frame.nodes, "Save", 0, &mut view, &ctx);
    let rules =
        aatf::Rules::load(&std::fs::read_to_string(&b).unwrap()).expect("b.4ccrules still loads");
    assert_eq!(
        rules.ruleset(),
        &view.document.as_ref().unwrap().ruleset,
        "Save wrote b.4ccrules"
    );
    assert_file_unchanged(&a, &a_before);
}

// RE-FILE-07
#[test]
fn save_leaves_no_sibling_and_reports_a_read_only_file() {
    let (ctx, _events, path, mut view) = official_opened("file_07");
    edit_ruleset(&mut view, |ruleset| ruleset.name = "Renamed".to_string());
    assert!(save_to(&mut view, &ctx, &path));
    let dir = path.parent().expect("the scratch folder").to_path_buf();
    let inside: Vec<String> = folder_contents(&dir);
    assert_eq!(
        inside,
        ["rules.4ccrules"],
        "no other file is left beside it"
    );

    // The read-only leg: the write is refused, the file is byte for
    // byte as it was, nothing is left beside it, the document stays
    // dirty.
    let before = std::fs::read(&path).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();
    edit_ruleset(&mut view, |ruleset| {
        ruleset.name = "Renamed again".to_string()
    });
    let saved = save_to(&mut view, &ctx, &path);
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "a test fixture must get its writable bit back or the scratch folder cannot be cleared"
    )]
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
    assert!(!saved, "Save reports the failure");
    assert_file_unchanged(&path, &before);
    assert_eq!(
        folder_contents(&dir),
        ["rules.4ccrules"],
        "no other file beside it"
    );
    assert!(
        view.document.as_ref().expect("the document").is_dirty(),
        "the document stays unsaved"
    );
    let mut window = Headless::new();
    let frame = window.draw(&mut view, &ctx);
    assert!(
        labels(&frame.nodes)
            .iter()
            .any(|label| label.contains("rules.4ccrules") && label.contains("read-only")),
        "the window names the file and the error"
    );
}

// A test reaching the default file dialogs panics instead of opening a
// real window (the pickers are stubbed per test).
#[test]
#[should_panic(expected = "a test reached a real file dialog: stub open_dialog/save_as_dialog")]
fn a_test_reaching_the_default_dialog_panics() {
    let (ctx, _events, _window, mut view) = fresh();
    open_document(&mut view, crate::document::blank());
    // A never-saved document, Save as: the default picker.
    let settings = crate::settings::RulesetEditorSettings::default();
    save_now(&mut view, &ctx, &settings);
}

// The suggested Save as name is the ruleset's name slugged
// (the draft's `save()`), the `.4ccrules` ending a member's file carries.
#[test]
fn save_as_suggests_the_rulesets_name_slugged() {
    assert_eq!(
        crate::view::suggested_name("Autumn 26"),
        "autumn_26.4ccrules"
    );
    assert_eq!(crate::view::suggested_name("DBC7"), "dbc7.4ccrules");
    assert_eq!(
        crate::view::suggested_name("Spring Cup"),
        "spring_cup.4ccrules"
    );
}

// A name's slash, colon or other punctuation never reaches the suggested
// name: the Save dialog would read a slash as a folder.
#[test]
fn save_as_suggests_no_punctuation_from_the_name() {
    assert_eq!(
        crate::view::suggested_name("/vg/ League 27"),
        "vg_league_27.4ccrules"
    );
    assert_eq!(
        crate::view::suggested_name("4cc Autumn 26"),
        "4cc_autumn_26.4ccrules"
    );
    assert_eq!(
        crate::view::suggested_name("Divegrass Before Christmas 7"),
        "divegrass_before_christmas_7.4ccrules"
    );
}

// A second document gets a fresh widget-state salt (egui state never
// crosses into it).
#[test]
fn a_second_document_gets_a_fresh_widget_salt() {
    let mut view = RulesetView::new();
    open_document(&mut view, crate::document::blank());
    let first = view.generation;
    open_document(&mut view, crate::document::blank());
    assert_ne!(view.generation, first, "the id salt differs per document");
}

// A settings table this build cannot read is not written over: its recent
// files stay as they were, not emptied.
#[test]
fn a_broken_settings_table_is_not_written_over() {
    let (_dir, path) = scratch_file("settings_broken", "rules.4ccrules");
    let (ctx, _events) = crate::test_support::context();
    let mut broken = toml::Table::new();
    broken.insert("recent_files".to_string(), toml::Value::Integer(3));
    ctx.set_tool_settings(crate::messages::TOOL_ID, broken);
    let mut view = RulesetView::new();
    open_path(&mut view, &ctx, &path);
    assert!(view.document.is_some(), "the file still opens");
    let table = ctx.tool_settings(crate::messages::TOOL_ID);
    assert_eq!(
        table["recent_files"],
        toml::Value::Integer(3),
        "the table is left as it was"
    );
}

// spec/core/gui.md "Settings menu": a key in the tool's table this
// build does not know is kept as read — a newer version's settings
// file loses nothing when this one writes.
#[test]
fn an_unknown_settings_key_survives_an_open() {
    let (_dir, path) = scratch_file("settings_future", "rules.4ccrules");
    let (ctx, _events) = crate::test_support::context();
    let mut table = toml::Table::new();
    table.insert(
        "future_key".to_string(),
        toml::Value::String("kept".to_string()),
    );
    ctx.set_tool_settings(crate::messages::TOOL_ID, table);
    let mut view = RulesetView::new();
    open_path(&mut view, &ctx, &path);
    let table = ctx.tool_settings(crate::messages::TOOL_ID);
    assert_eq!(
        table["future_key"],
        toml::Value::String("kept".to_string()),
        "the unknown key is kept"
    );
    let settings = crate::settings::from_table(&table).unwrap();
    assert_eq!(
        settings.recent_files,
        [path.to_string_lossy().into_owned()],
        "the opened file heads the recent files"
    );
}
