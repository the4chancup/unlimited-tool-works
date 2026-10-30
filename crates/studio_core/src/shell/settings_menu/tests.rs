use std::fs;
use std::path::{Path, PathBuf};

use egui::accesskit::{self, Role};
use pes_version::PesVersion;

use crate::pes_process::RunningPes;
use crate::settings::{CommonSettings, Settings, Theme, pes_folder_pattern};
use crate::shell::sidebar::SETTINGS_LABEL;
use crate::shell::test_harness::*;
use crate::status::{ActionEffect, Notice, NoticeAction, NoticeKind};
use crate::tool::AppPaths;

/// The node carrying `text` as a label-role node's `value`.
fn text_node<'a>(nodes: &'a [accesskit::Node], text: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.value() == Some(text))
        .unwrap_or_else(|| panic!("no node with text {text:?}"))
}

/// The widget of `role` drawn on `label`'s grid row (the same y span).
fn widget_on_row<'a>(nodes: &'a [accesskit::Node], label: &str, role: Role) -> &'a accesskit::Node {
    let row = text_node(nodes, label)
        .bounds()
        .expect("the label has bounds");
    nodes
        .iter()
        .find(|node| {
            node.role() == role
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.y0 < row.y1 && bounds.y1 > row.y0)
        })
        .unwrap_or_else(|| panic!("no {role:?} on the {label:?} row"))
}

/// The `Role::TextRun` node on `label`'s row whose value is `text`: a text
/// edit's run is the whole galley line, unclipped; the row's label carries a
/// `TextRun` of its own text too, hence the value filter.
fn text_run_on_row<'a>(
    nodes: &'a [accesskit::Node],
    label: &str,
    text: &str,
) -> &'a accesskit::Node {
    let row = text_node(nodes, label)
        .bounds()
        .expect("the label has bounds");
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::TextRun
                && node.value() == Some(text)
                && node
                    .bounds()
                    .is_some_and(|bounds| bounds.y0 < row.y1 && bounds.y1 > row.y0)
        })
        .unwrap_or_else(|| panic!("no {text:?} run on the {label:?} row"))
}

/// A key event.
fn key(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::default(),
    }
}

/// Nodes with `label` as their widget label: a tool's sidebar entry plus, when it
/// has settings, its section header.
fn labeled(nodes: &[accesskit::Node], label: &str) -> usize {
    nodes
        .iter()
        .filter(|node| node.label() == Some(label))
        .count()
}

// ST-SET-01
#[test]
fn the_gear_opens_the_menu_on_the_active_tools_section_and_resets_each_opening() {
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let (c, calls_c) = stub("c", "C", None);
    // A tool whose `default_settings()` is empty gets no section.
    calls_c.default_settings.lock().unwrap().clear();
    let dir = TestDir::new("studio_core_settings_sections");
    let Shell { mut app, ctx, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b, c],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b, calls_c],
    );
    ctx.switch_to_tool("b");
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(app.active, 1);
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    // The collapsing animation settles over a few frames.
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(has_text(&nodes, "Threads"), "the common grid is drawn");
    assert_eq!(labeled(&nodes, "A"), 2, "A's sidebar entry and its header");
    assert_eq!(labeled(&nodes, "B"), 2, "B's sidebar entry and its header");
    assert_eq!(
        labeled(&nodes, "C"),
        1,
        "C has no settings: the entry alone"
    );
    assert!(
        has_text(&nodes, "B settings body"),
        "the active tool's section is expanded"
    );
    assert!(
        !has_text(&nodes, "A settings body"),
        "the other sections are collapsed"
    );

    // The gear click leaves the pointer on the rail, the sidebar's overlay
    // open over the menu's left edge on the test's 800-point screen (the app
    // lays out at least 1280 points wide, where the two never meet); resting
    // it inside the menu closes the overlay before the clicks below.
    frame(
        &egui_ctx,
        &mut app,
        hover(widget_on_row(&nodes, "PES folder", Role::TextInput)),
        true,
    );
    idle(&egui_ctx, &mut app, 2);

    // A click on B's header collapses its section, and the collapse holds while the
    // menu stays open: the member's toggle is the state, not the open set.
    let header = nodes
        .iter()
        .find(|node| node.label() == Some("B") && node.toggled().is_none())
        .expect("B's section header");
    click(&egui_ctx, &mut app, header);
    let nodes = idle(&egui_ctx, &mut app, 15).nodes;
    assert!(
        !has_text(&nodes, "B settings body"),
        "the member's collapse holds"
    );
    for _ in 0..3 {
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            !has_text(&nodes, "B settings body"),
            "B's section stays collapsed"
        );
        assert!(has_text(&nodes, "Threads"), "the menu is still open");
    }

    // Re-expand B before the close: without the reset, a reopening would show its
    // section still open, so the collapsed assertion below proves the reset.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let header = nodes
        .iter()
        .find(|node| node.label() == Some("B") && node.toggled().is_none())
        .expect("B's section header");
    click(&egui_ctx, &mut app, header);
    let nodes = idle(&egui_ctx, &mut app, 15).nodes;
    assert!(
        has_text(&nodes, "B settings body"),
        "B expanded again: the reopening can only collapse it through the reset"
    );

    // Close, make A active, reopen: the open set is reset to the active tool.
    click(&egui_ctx, &mut app, entry(&nodes, "Close"));
    ctx.switch_to_tool("a");
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(app.active, 0);
    assert!(!has_text(&nodes, "B settings body"), "the menu is closed");
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    // B's collapse animates out over 0.2 s of frames before its body stops drawing.
    let nodes = idle(&egui_ctx, &mut app, 15).nodes;
    assert!(
        has_text(&nodes, "A settings body"),
        "now the active tool's section is expanded"
    );
    assert!(
        !has_text(&nodes, "B settings body"),
        "the open set was reset at the reopening"
    );
}

// A CLI-only tool has no section, whatever its table holds: its window is not built
// and its keys are edited in the settings file.
#[test]
fn a_cli_only_tool_has_no_section_even_with_settings() {
    let (a, calls_a) = stub("a", "A", None);
    let (c, calls_c) = stub("c", "C", None);
    // The Ruleset editor's shape: a table of state the tool writes itself.
    *calls_c.default_settings.lock().unwrap() = toml::Table::from_iter([
        ("last_folder".to_owned(), toml::Value::String(String::new())),
        ("recent_files".to_owned(), toml::Value::Array(Vec::new())),
    ]);
    let dir = TestDir::new("studio_core_settings_cli_only");
    let Shell { mut app, .. } = shell_in(
        test_settings(dir.path()),
        vec![a, c],
        &[],
        &["c"],
        AppPaths {
            exe_dir: PathBuf::from("exe"),
            data_dir: Some(dir.path().to_path_buf()),
        },
        None,
        vec![calls_a, calls_c],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(labeled(&nodes, "C"), 0, "C is in no planned list: no entry");
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(has_text(&nodes, "Threads"), "the menu is open");
    assert_eq!(labeled(&nodes, "A"), 2, "A's sidebar entry and its header");
    assert_eq!(labeled(&nodes, "C"), 0, "the CLI-only tool has no header");
}

// ST-SET-02
#[test]
fn a_threads_change_writes_the_common_setting_and_keeps_the_tool_tables() {
    let dir = test_dir("studio_core_settings_threads");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let table_a = a.default_settings();
    let table_b = b.default_settings();
    let mut settings = test_settings(&dir);
    settings.set_tool("a", table_a.clone());
    settings.set_tool("b", table_b.clone());
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a, b],
        &[],
        Some(data.clone()),
        vec![calls_a, calls_b],
    );
    let egui_ctx = test_context();
    assert_ne!(
        ctx.common().thread_count,
        1,
        "the test's pick really changes the setting"
    );

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    // A click on a DragValue turns it into a text field; type and press Enter.
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "Threads", Role::SpinButton),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::Text("1".to_owned())],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(ctx.common().thread_count, 1);
    frame(&egui_ctx, &mut app, Vec::new(), true);

    let text = fs::read_to_string(data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    let file: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(
        file["common"]["thread_count"],
        toml::Value::Integer(1),
        "the file holds the new value under [common]"
    );
    assert_eq!(
        file["a"],
        toml::Value::Table(table_a),
        "a's table as it was"
    );
    assert_eq!(
        file["b"],
        toml::Value::Table(table_b),
        "b's table as it was"
    );
    assert_eq!(ctx.common().thread_count, 1);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_wheel_notch_over_threads_steps_the_thread_count_by_one() {
    let dir = TestDir::new("studio_core_settings_threads_wheel");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, ctx, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    assert_eq!(ctx.common().thread_count, 0, "the premise: automatic");
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    frame(
        &egui_ctx,
        &mut app,
        wheel_notch(widget_on_row(&nodes, "Threads", Role::SpinButton), true),
        true,
    );
    // Frames for any rest of the notch to show.
    idle(&egui_ctx, &mut app, 15);

    assert_eq!(ctx.common().thread_count, 1, "one notch up is one more");
}

// The `Memory cap` row's entry range accepts a fraction of a percent (the common
// settings allow `0 < percent <= 100`).
#[test]
fn the_memory_cap_row_accepts_a_fraction_of_a_percent() {
    let dir = TestDir::new("studio_core_settings_memcap");
    let (a, calls_a) = stub("a", "A", None);
    let settings = test_settings(dir.path());
    assert_ne!(
        settings.common.memory_cap_percent, 0.5,
        "the test's pick really changes the setting"
    );
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "Memory cap", Role::SpinButton),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::Text("0.5".to_owned())],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(ctx.common().memory_cap_percent, 0.5);
}

// The draft's `Memory cap` row: the box holds the number alone, and `%` stands
// after it as the row's own text.
#[test]
fn the_memory_cap_box_holds_the_number_and_the_percent_sign_follows_it() {
    let dir = TestDir::new("studio_core_settings_memcap_percent");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    let field = widget_on_row(&nodes, "Memory cap", Role::SpinButton);
    let text = field.value().expect("the box shows its value");
    assert!(
        text.parse::<f64>().is_ok(),
        "the box's text {text:?} is the number alone"
    );
    let field = field.bounds().expect("the box has bounds");
    let percent = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("%"))
        .and_then(accesskit::Node::bounds)
        .expect("a % label is drawn");
    assert!(
        percent.x0 >= field.x1 && percent.y0 < field.y1 && percent.y1 > field.y0,
        "the % {percent:?} stands right of the box {field:?}, on its row"
    );
}

// The FPC hide ID rows, after the memory cap: the boots row's entry range is
// the stock band (1 to 100), the gloves row's its own (11 to 100), and a
// typed value writes the common setting.
#[test]
fn the_hide_id_rows_write_the_common_settings() {
    let dir = TestDir::new("studio_core_settings_hide_ids");
    let (a, calls_a) = stub("a", "A", None);
    let settings = test_settings(dir.path());
    assert_ne!(
        settings.common.fpc_hide_boots_id, 60,
        "the test's pick really changes the setting"
    );
    assert_ne!(
        settings.common.fpc_hide_gloves_id, 12,
        "the test's pick really changes the setting"
    );
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    for (label, min, max) in [
        ("FPC hide boots ID", 1.0, 100.0),
        ("FPC hide gloves ID", 11.0, 100.0),
    ] {
        let field = widget_on_row(&nodes, label, Role::SpinButton);
        assert_eq!(
            (field.min_numeric_value(), field.max_numeric_value()),
            (Some(min), Some(max)),
            "{label}'s entry range"
        );
    }

    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "FPC hide boots ID", Role::SpinButton),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::Text("60".to_owned())],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(ctx.common().fpc_hide_boots_id, 60);

    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "FPC hide gloves ID", Role::SpinButton),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![egui::Event::Text("12".to_owned())],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(ctx.common().fpc_hide_gloves_id, 12);
    assert_eq!(ctx.common().fpc_hide_boots_id, 60, "the boots write stands");
}

// The update check's two rows are not drawn while nothing reads the keys:
// they wait for the updater.
#[test]
fn the_update_check_rows_are_not_drawn_yet() {
    let dir = TestDir::new("studio_core_settings_no_update_rows");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(has_text(&nodes, "Theme"), "the menu is open");
    assert!(
        !has_text(&nodes, "Check for updates"),
        "the check row waits for the updater"
    );
    assert!(
        !has_text(&nodes, "Check interval (hours)"),
        "the interval row waits with it"
    );
}

// Opening the menu changes nothing by itself: a hand-set `thread_count` above this
// machine's cores (a portable folder moved to a smaller PC) stays as written, though
// the row's drag range stops at the core count.
#[test]
fn opening_the_menu_keeps_a_thread_count_above_the_core_count() {
    let (a, calls_a) = stub("a", "A", None);
    let dir = TestDir::new("studio_core_settings_no_clamp");
    let mut settings = test_settings(dir.path());
    settings.common.thread_count = 4096;
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(has_text(&nodes, "Threads"), "the menu is open");
    assert_eq!(ctx.common().thread_count, 4096);
}

// The theme setting, never the system's theme, decides, and a pick applies at
// the next frame. No acceptance scenario covers it.
#[test]
fn the_theme_setting_decides_and_a_pick_applies_at_the_next_frame() {
    let dir = test_dir("studio_core_settings_theme");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(&dir);
    settings.common.theme = Theme::Dark;
    let Shell { mut app, ctx, .. } =
        shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
    let egui_ctx = test_context();

    // A light desktop does not lift the dark default.
    frame_theme(
        &egui_ctx,
        &mut app,
        Vec::new(),
        true,
        Some(egui::Theme::Light),
    );
    assert_eq!(
        egui_ctx.theme(),
        egui::Theme::Dark,
        "the setting wins over the system's theme"
    );

    let nodes = frame_theme(
        &egui_ctx,
        &mut app,
        Vec::new(),
        true,
        Some(egui::Theme::Light),
    )
    .nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert_eq!(
        widget_on_row(&nodes, "Theme", Role::ComboBox).value(),
        Some("Dark"),
        "the combo shows the setting's theme"
    );
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "Theme", Role::ComboBox),
    );
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "Light"));
    assert_eq!(ctx.common().theme, Theme::Light);
    frame_theme(
        &egui_ctx,
        &mut app,
        Vec::new(),
        true,
        Some(egui::Theme::Dark),
    );
    assert_eq!(
        egui_ctx.theme(),
        egui::Theme::Light,
        "the pick applied at the next frame, on a dark desktop too"
    );
    let text = fs::read_to_string(data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    let file: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(file["common"]["theme"].as_str(), Some("light"));
    fs::remove_dir_all(&dir).unwrap();
}

// A text row commits when its edit loses focus or Enter is pressed, not per
// keystroke.
#[test]
fn a_path_row_commits_on_enter_not_per_keystroke() {
    let dir = test_dir("studio_core_settings_path_commit");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, ctx, .. } = shell_with(
        test_settings(&dir),
        vec![a],
        &[],
        Some(data.clone()),
        vec![calls_a],
    );
    let before = ctx.common().pes_folder_path;
    let egui_ctx = test_context();

    // The committed path is under the test's own folder: the next frame's exe
    // check lists it, so it must not reach a developer path. Its name holds the
    // selected PES 2019's year: a typed folder is stored as typed, never through
    // the `20**` rule a Browse pick goes through.
    let pes_folder = dir.join("PES2019").display().to_string();
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "PES folder", Role::TextInput),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Text(pes_folder.clone()),
        ],
        true,
    );
    assert_eq!(
        ctx.common().pes_folder_path,
        before,
        "a half-typed path does not reach the setting"
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(
        ctx.common().pes_folder_path,
        pes_folder,
        "Enter commits the edit"
    );
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let written = Settings::load(&data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    assert_eq!(
        written.common.pes_folder_path,
        dir.join("PES2019").display().to_string()
    );
    fs::remove_dir_all(&dir).unwrap();
}

// A half-typed path survives the window losing focus mid-edit: the buffer resets
// only on the edit's own focus loss, not on an Alt+Tab.
#[test]
fn a_path_row_edit_survives_the_window_losing_focus() {
    let dir = test_dir("studio_core_settings_focus_loss");
    let data = dir.join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, ctx, .. } = shell_with(
        test_settings(&dir),
        vec![a],
        &[],
        Some(data.clone()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let pes_folder = dir.join("pes19").display().to_string();
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "PES folder", Role::TextInput),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Text(pes_folder.clone()),
        ],
        true,
    );

    // The window loses and regains focus mid-edit.
    frame(&egui_ctx, &mut app, Vec::new(), false);
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(
        widget_on_row(&nodes, "PES folder", Role::TextInput).value(),
        Some(pes_folder.as_str()),
        "the field still shows the typed text"
    );

    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    assert_eq!(
        ctx.common().pes_folder_path,
        pes_folder,
        "Enter commits the edit"
    );
    fs::remove_dir_all(&dir).unwrap();
}

// A path row draws its Browse button right of the edit. The picker itself is
// `rfd`'s native dialog, so the test sees the button, never a click.
#[test]
fn a_path_row_draws_browse_right_of_the_edit() {
    let dir = TestDir::new("studio_core_settings_browse");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    let edit = widget_on_row(&nodes, "PES folder", Role::TextInput);
    let browse = widget_on_row(&nodes, "PES folder", Role::Button);
    assert_eq!(browse.label(), Some("Browse"));
    assert!(
        centre(browse).x > centre(edit).x,
        "the button stands right of the edit"
    );
}

// The common grid's rows stand 22 apart, as the draft's grid rows do: an
// 18-point control with its cell's 2 points above and below. The folder
// toggle's row, whose label cell is empty, is placed by its checkbox.
#[test]
fn the_common_rows_stand_22_apart() {
    let dir = TestDir::new("studio_core_settings_row_pitch");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    let row_centre = |role: Role, text: &str| {
        let node = nodes
            .iter()
            .find(|node| {
                node.role() == role && (node.value() == Some(text) || node.label() == Some(text))
            })
            .unwrap_or_else(|| panic!("no {role:?} {text:?}"));
        f64::from(centre(node).y)
    };
    let rows = [
        (Role::Label, "PES version"),
        (Role::Label, "PES folder"),
        (Role::CheckBox, "A folder per version"),
        (Role::Label, "Exports folder"),
        (Role::Label, "Threads"),
        (Role::Label, "Memory cap"),
    ];
    for pair in rows.windows(2) {
        let (upper, lower) = (pair[0].1, pair[1].1);
        let pitch = row_centre(pair[1].0, lower) - row_centre(pair[0].0, upper);
        assert!(
            (pitch - 22.0).abs() <= 0.5,
            "{lower} stands 22 below {upper}, not {pitch}"
        );
    }
}

// The `PES version` row is the sidebar selector's twin: a pick writes the setting
// through the same function, which spends the once-per-life auto-switch.
#[test]
fn a_menu_pick_writes_pes_version_and_spends_the_auto_switch() {
    fn running_19() -> Vec<RunningPes> {
        vec![RunningPes {
            version: PesVersion::Pes19,
            exe: None,
        }]
    }
    let (a, calls_a) = stub("a", "A", None);
    let dir = TestDir::new("studio_core_settings_pick");
    let mut settings = test_settings(dir.path());
    settings.common.pes_version = PesVersion::Pes19;
    let Shell { mut app, ctx, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "PES version", Role::ComboBox),
    );
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, "PES 2021"));
    assert_eq!(ctx.common().pes_version, PesVersion::Pes21);
    assert!(
        app.sidebar.auto_switch_used,
        "the pick spent the once-per-life auto-switch"
    );

    // A probe reporting another running version switches nothing at the next poll.
    app.sidebar.probe = running_19;
    frame_at(&egui_ctx, &mut app, Vec::new(), true, Some(5.0));
    assert_eq!(ctx.common().pes_version, PesVersion::Pes21);
}

// While a tool reports a `version_change_blocker` the row is disabled and its
// disabled-hover tooltip is the blocking tool's `<label>: <reason>` (the
// sidebar's rule, ST-SIDE-04).
#[test]
fn a_blocking_tool_disables_the_menu_version_row_with_its_reason() {
    let (a, calls_a) = stub("a", "A", Some("a compile is running"));
    let (b, calls_b) = stub("b", "B", None);
    let dir = TestDir::new("studio_core_settings_blocked");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    let combo = widget_on_row(&nodes, "PES version", Role::ComboBox);
    assert!(combo.is_disabled(), "the blocked row is disabled");
    // The click's tooltip cooldown ages out first, then `is_still` takes about
    // 0.1 s of idle frames.
    idle(&egui_ctx, &mut app, 8);
    frame(&egui_ctx, &mut app, hover(combo), true);
    let nodes = idle(&egui_ctx, &mut app, 8).nodes;
    assert!(
        has_text(&nodes, "A: a compile is running"),
        "the disabled hover is the blocking tool's reason"
    );
}

// A notice's `ActionEffect::OpenSettings` opens the menu on the active tool's
// section, like the gear.
#[test]
fn a_notice_with_open_settings_opens_the_menu_on_the_active_tool() {
    let (a, calls_a) = stub("a", "A", None);
    let (b, calls_b) = stub("b", "B", None);
    let dir = TestDir::new("studio_core_settings_notice");
    let Shell { mut app, ctx, .. } = shell_with(
        test_settings(dir.path()),
        vec![a, b],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a, calls_b],
    );
    ctx.switch_to_tool("b");
    ctx.notify(Notice {
        text: "choose".to_owned(),
        kind: NoticeKind::Info,
        action: Some(NoticeAction {
            label: "Open settings".to_owned(),
            effect: ActionEffect::OpenSettings,
        }),
    });
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(app.active, 1);
    click(&egui_ctx, &mut app, entry(&nodes, "Open settings"));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(
        has_text(&nodes, "B settings body"),
        "the menu opened on the active tool's section"
    );
    assert!(!has_text(&nodes, "A settings body"));
}

// A row label's tooltip leads with the settings key, then the row's tip (the
// draft's `sRow`).
#[test]
fn a_row_labels_tooltip_leads_with_the_settings_key() {
    let (a, calls_a) = stub("a", "A", None);
    let dir = TestDir::new("studio_core_settings_tooltip");
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();
    egui_ctx.all_styles_mut(|style| style.interaction.tooltip_delay = 0.0);

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    // The gear click leaves the pointer on the rail, the sidebar's overlay
    // open over the menu's left edge on the test's 800-point screen (the app
    // lays out at least 1280 points wide, where the two never meet); resting
    // it inside the menu closes the overlay before the hover below.
    frame(
        &egui_ctx,
        &mut app,
        hover(widget_on_row(&nodes, "PES folder", Role::TextInput)),
        true,
    );
    idle(&egui_ctx, &mut app, 8);
    frame(
        &egui_ctx,
        &mut app,
        hover(text_node(&nodes, "Threads")),
        true,
    );
    let nodes = idle(&egui_ctx, &mut app, 8).nodes;
    assert!(
        has_text(
            &nodes,
            "thread_count: The number of worker threads a tool's run uses, where 0 \
             means automatic (the logical cores minus one)."
        ),
        "the tooltip is the settings key, then the row's tip"
    );
}

// The pick's rule as the scenario states it, at the selector's PES 2019: a picked
// folder whose own name holds the version's year stores it as `20**`, so the
// selector keeps switching folders; any other folder stores as picked.
#[cfg(windows)]
// ST-SET-04
#[test]
fn a_picked_pes_folder_stores_the_year_as_the_pattern() {
    let mut settings = CommonSettings {
        pes_folder_path: pes_folder_pattern(
            Path::new(r"D:\Games\Pro Evolution Soccer 2019"),
            PesVersion::Pes19,
        ),
        ..CommonSettings::default()
    };
    assert_eq!(
        settings.pes_folder_path, r"D:\Games\Pro Evolution Soccer 20**",
        "the year stored as 20**"
    );
    settings.pes_version = PesVersion::Pes21;
    assert_eq!(
        settings.pes_folder(),
        std::path::PathBuf::from(r"D:\Games\Pro Evolution Soccer 2021"),
        "PES 2021 resolves through the pattern"
    );

    settings.pes_folder_path = pes_folder_pattern(Path::new(r"E:\PES"), PesVersion::Pes19);
    assert_eq!(settings.pes_folder_path, r"E:\PES", "stored as given");
}

// The `/`-separated twin of the Windows case above.
#[cfg(not(windows))]
// ST-SET-04
#[test]
fn a_picked_pes_folder_stores_the_year_as_the_pattern() {
    let mut settings = CommonSettings {
        pes_folder_path: pes_folder_pattern(
            Path::new("/games/Pro Evolution Soccer 2019"),
            PesVersion::Pes19,
        ),
        ..CommonSettings::default()
    };
    assert_eq!(
        settings.pes_folder_path, "/games/Pro Evolution Soccer 20**",
        "the year stored as 20**"
    );
    settings.pes_version = PesVersion::Pes21;
    assert_eq!(
        settings.pes_folder(),
        std::path::PathBuf::from("/games/Pro Evolution Soccer 2021"),
        "PES 2021 resolves through the pattern"
    );

    settings.pes_folder_path = pes_folder_pattern(Path::new("/e/PES"), PesVersion::Pes19);
    assert_eq!(settings.pes_folder_path, "/e/PES", "stored as given");
}

// The menu sequence: the toggle draws the seven per-version rows, a committed row
// lands as its version's entry under [common.pes_folders] and locks the toggle on,
// Fix PES path writes the running version's own row, and emptying the rows frees
// the toggle.
// ST-SET-04
#[test]
fn a_folder_per_version_overrides_the_main_row_until_emptied() {
    let dir = TestDir::new("studio_core_settings_per_version");
    let data = dir.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(dir.path());
    settings.common.pes_version = PesVersion::Pes19;
    settings.common.pes_folder_path = dir
        .path()
        .join("Pro Evolution Soccer 20**")
        .display()
        .to_string();
    let main_row = settings.common.pes_folder_path.clone();
    let Shell { mut app, ctx, .. } =
        shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(has_text(&nodes, "PES folder"), "the menu is open");
    assert!(
        !has_text(&nodes, "PES 2021 folder"),
        "no per-version rows while the toggle is off"
    );

    let nodes = click(&egui_ctx, &mut app, entry(&nodes, "A folder per version")).nodes;
    for year in 2015..=2021 {
        assert!(
            has_text(&nodes, &format!("PES {year} folder")),
            "the {year} row is drawn"
        );
    }

    // Type PES 2021's folder and commit with Enter: the file holds `21` alone,
    // the version resolves there, and PES 2019 keeps the main row's resolution.
    let typed = dir.path().join("PES2021").display().to_string();
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "PES 2021 folder", Role::TextInput),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Text(typed.clone()),
        ],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let text = fs::read_to_string(data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    let file: toml::Table = toml::from_str(&text).unwrap();
    let folders = file["common"]["pes_folders"].as_table().unwrap();
    assert_eq!(folders.len(), 1, "[common.pes_folders] holds 21 alone");
    assert_eq!(folders["21"].as_str(), Some(typed.as_str()));
    let mut at_21 = ctx.common();
    at_21.pes_version = PesVersion::Pes21;
    assert_eq!(
        at_21.pes_folder(),
        std::path::PathBuf::from(&typed),
        "PES 2021 resolves to its own row"
    );
    assert_eq!(
        ctx.common().pes_folder(),
        dir.path().join("Pro Evolution Soccer 2019"),
        "PES 2019 falls back to the main row"
    );

    // A filled row locks the toggle on: the click does nothing and the rows stay.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let toggle = entry(&nodes, "A folder per version");
    assert!(toggle.is_disabled(), "a filled row locks the toggle on");
    let nodes = click(&egui_ctx, &mut app, toggle).nodes;
    assert!(has_text(&nodes, "PES 2021 folder"), "the rows stay drawn");

    // Fix PES path for a running PES 2019: the version's own row takes the folder
    // as it is and the main row stands.
    let running = dir.path().join("PES 2019");
    app.apply_notice_effect(ActionEffect::FixPesPath {
        folder: running.clone(),
        version: PesVersion::Pes19,
    });
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let text = fs::read_to_string(data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    let file: toml::Table = toml::from_str(&text).unwrap();
    let folders = file["common"]["pes_folders"].as_table().unwrap();
    assert_eq!(
        folders["19"].as_str(),
        Some(running.display().to_string().as_str()),
        "the file gains 19"
    );
    assert_eq!(
        file["common"]["pes_folder_path"].as_str(),
        Some(main_row.as_str()),
        "the main row unchanged"
    );
    // The edit's buffer follows the committed value the frame after the write.
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert_eq!(
        widget_on_row(&nodes, "PES 2019 folder", Role::TextInput).value(),
        Some(running.display().to_string().as_str()),
        "the row's edit shows the new folder"
    );

    // Emptying both rows removes the entries; only then can the toggle go off.
    for label in ["PES 2021 folder", "PES 2019 folder"] {
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(
            &egui_ctx,
            &mut app,
            widget_on_row(&nodes, label, Role::TextInput),
        );
        frame(
            &egui_ctx,
            &mut app,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                key(egui::Key::Backspace, true),
            ],
            true,
        );
        frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    }
    frame(&egui_ctx, &mut app, Vec::new(), true);
    let text = fs::read_to_string(data.join(crate::settings::SETTINGS_FILE_NAME)).unwrap();
    assert!(
        !text.contains("pes_folders"),
        "the entries are gone: {text}"
    );
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    let toggle = entry(&nodes, "A folder per version");
    assert!(!toggle.is_disabled(), "empty rows free the toggle");
    let nodes = click(&egui_ctx, &mut app, toggle).nodes;
    assert!(
        !has_text(&nodes, "PES 2021 folder"),
        "the toggle switches off and the rows are gone"
    );
}

// A menu opened over a saved override shows the toggle on and locked; emptying that
// last row unlocks it but leaves it on, so the rows do not vanish under the edit.
// ST-SET-04
#[test]
fn emptying_the_last_folder_of_a_version_leaves_the_rows_shown() {
    let dir = TestDir::new("studio_core_settings_per_version_reopened");
    let data = dir.path().join("data");
    fs::create_dir_all(&data).unwrap();
    let (a, calls_a) = stub("a", "A", None);
    let mut settings = test_settings(dir.path());
    settings.common.pes_folders.insert(
        PesVersion::Pes21,
        dir.path().join("PES2021").display().to_string(),
    );
    let Shell { mut app, ctx, .. } =
        shell_with(settings, vec![a], &[], Some(data.clone()), vec![calls_a]);
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    assert!(
        entry(&nodes, "A folder per version").is_disabled(),
        "the saved override locks the toggle on"
    );
    click(
        &egui_ctx,
        &mut app,
        widget_on_row(&nodes, "PES 2021 folder", Role::TextInput),
    );
    frame(
        &egui_ctx,
        &mut app,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            key(egui::Key::Backspace, true),
        ],
        true,
    );
    frame(&egui_ctx, &mut app, vec![key(egui::Key::Enter, true)], true);
    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    assert!(
        ctx.common().pes_folders.is_empty(),
        "the emptied row removed the entry"
    );
    assert!(
        has_text(&nodes, "PES 2021 folder"),
        "the rows stay drawn after the last one is emptied"
    );
    assert!(
        !entry(&nodes, "A folder per version").is_disabled(),
        "the toggle is free to switch off"
    );
}

// A path edit shows Steam's PES 2019 install path whole: the drawn text run is
// the path's galley at the body font, starting inside the edit's left margin
// and ending inside its right one, and the common grid's widest row then fits
// the menu's width.
#[test]
fn every_path_edit_shows_steams_pes_2019_folder_whole() {
    const STEAM_2019: &str =
        r"C:\Program Files (x86)\Steam\steamapps\common\Pro Evolution Soccer 2019";
    let dir = TestDir::new("studio_core_settings_path_width");
    let (a, calls_a) = stub("a", "A", None);
    *calls_a.file_row.lock().unwrap() = Some(STEAM_2019.to_owned());
    let mut settings = test_settings(dir.path());
    settings.common.pes_version = PesVersion::Pes21;
    // PES 2021 has its own row pointing into the test's folder: `pes_folder()`
    // (which the version row's exe check reads) then resolves there, so the
    // test never touches the developer's install.
    settings
        .common
        .pes_folders
        .insert(PesVersion::Pes21, dir.path().display().to_string());
    settings
        .common
        .pes_folders
        .insert(PesVersion::Pes19, STEAM_2019.to_owned());
    settings.common.pes_folder_path = STEAM_2019.to_owned();
    settings.common.exports_folder_path = PathBuf::from(STEAM_2019);
    let Shell { mut app, .. } = shell_with(
        settings,
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    // The fonts exist once the first frame has run.
    let font = egui::TextStyle::Body.resolve(&egui_ctx.global_style());
    let galley = egui_ctx
        .fonts_mut(|fonts| fonts.layout_no_wrap(STEAM_2019.to_owned(), font, egui::Color32::WHITE))
        .size()
        .x;
    for label in [
        "PES folder",
        "PES 2019 folder",
        "Exports folder",
        "A tool's file",
    ] {
        let edit = widget_on_row(&nodes, label, Role::TextInput)
            .bounds()
            .expect("the edit has bounds");
        let run = text_run_on_row(&nodes, label, STEAM_2019)
            .bounds()
            .expect("the text run has bounds");
        let width = run.x1 - run.x0;
        assert!(
            (width - f64::from(galley)).abs() < 0.01,
            "{label}: the run is the path's galley at the body font \
             ({width} vs {galley})"
        );
        let margin = run.x0 - edit.x0;
        assert!(
            margin > 0.0,
            "{label}: the text is not scrolled left \
             (margin {margin}; edit {edit:?}, run {run:?})"
        );
        assert!(
            run.x1 <= edit.x1 - margin,
            "{label}: the path ends inside the edit's right margin \
             (margin {margin}; edit {edit:?}, run {run:?})"
        );
    }
    for label in ["PES folder", "PES 2019 folder", "Exports folder"] {
        let browse = widget_on_row(&nodes, label, Role::Button);
        assert_eq!(browse.label(), Some("Browse"), "{label}'s button");
        let right = browse.bounds().expect("Browse has bounds").x1;
        let left = text_node(&nodes, label)
            .bounds()
            .expect("the label has bounds")
            .x0;
        assert!(
            right - left <= f64::from(super::WIDTH),
            "{label}: the common row fits the menu's width \
             (Browse's right {right} minus the label's left {left} = {})",
            right - left
        );
    }
    // The premise of the "A tool's file" check above: the tool section's row
    // reaches past the menu's width, so its edit is measured where the
    // section, not the common grid, sets the menu's width.
    let menu_left = text_node(&nodes, "PES folder")
        .bounds()
        .expect("the label has bounds")
        .x0;
    let section_right = widget_on_row(&nodes, "A tool's file", Role::Button)
        .bounds()
        .expect("Browse has bounds")
        .x1;
    assert!(
        section_right - menu_left > f64::from(super::WIDTH),
        "the tool section's row is wider than the menu's width \
         ({section_right} - {menu_left})"
    );
}

// The drafts' window: the common grid's first row starts 8 points under the
// title, and Close stands at the right, inside the stroke and the 12-point
// margin.
#[test]
fn the_menu_starts_its_grid_8_under_the_title_and_ends_in_close_at_the_right() {
    let dir = TestDir::new("studio_core_settings_window");
    let (a, calls_a) = stub("a", "A", None);
    let Shell { mut app, .. } = shell_with(
        test_settings(dir.path()),
        vec![a],
        &[],
        Some(dir.path().to_path_buf()),
        vec![calls_a],
    );
    let egui_ctx = test_context();

    let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
    click(&egui_ctx, &mut app, entry(&nodes, SETTINGS_LABEL));
    let nodes = idle(&egui_ctx, &mut app, 10).nodes;
    let outer = egui_ctx
        .memory(|memory| memory.area_rect(egui::Id::unique("settings-menu")))
        .expect("the menu's modal has an area");
    let title = text_node(&nodes, "Settings (WIP)")
        .bounds()
        .expect("the title has bounds");
    let label = text_node(&nodes, "PES version")
        .bounds()
        .expect("the label has bounds");
    let selector = widget_on_row(&nodes, "PES version", Role::ComboBox)
        .bounds()
        .expect("the selector has bounds");
    let close = entry(&nodes, "Close").bounds().expect("Close has bounds");

    let gap = label.y0.min(selector.y0) - title.y1;
    assert!(
        (gap - 8.0).abs() <= 0.5,
        "the first row starts 8 under the title, not {gap}"
    );
    let right = f64::from(outer.right()) - close.x1;
    assert!(
        (right - 13.0).abs() <= 0.5,
        "Close's right is 13 inside the menu's, not {right}"
    );
}
