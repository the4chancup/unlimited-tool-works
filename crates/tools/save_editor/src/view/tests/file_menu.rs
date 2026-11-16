//! The File menu's and Save as's tests (SE-GUI-26): the picker, Save and
//! Save as, the discovered saves under their heading and the recent files
//! under theirs, Close save and Exit; discovery run once per opening, the
//! picker's starting folder, an open under unsaved changes, the version
//! shortcuts, Ctrl+O, Ctrl+W and Ctrl+Shift+S, and Save as's write, `.bak`
//! and running-game question.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::SystemTime;

use pes_savefile::discovery::{discover_savefiles_in, save_layout};
use studio_core::StudioTool;

use super::prompt::modal_button;
use super::*;
use crate::test_support::{journal_lines, steps_with, zip_entry};

/// The SE-GUI-26 tree under a test's scratch folder: a `Documents` folder
/// holding two PES 21 saves and one PES 19 save in the game's own layout,
/// and one more save elsewhere under the scratch folder.
struct Tree {
    /// The scratch folder's `Documents`.
    documents: PathBuf,
    /// PES 21 under the account `292733975847239680`, the older of the two.
    pes21_old: PathBuf,
    /// PES 21 under the account `76561199320748656`, the newer.
    pes21_new: PathBuf,
    /// PES 19 under the account `292733975847239680`, the newest of the three.
    pes19: PathBuf,
    /// A PES 17 save outside `Documents`.
    elsewhere: PathBuf,
    /// A PES 21 save outside `Documents`, in a folder of its own.
    other: PathBuf,
}

/// Builds the tree under `test`'s scratch folder.
fn tree(test: &str) -> Tree {
    let dir = crate::test_support::scratch(test);
    let documents = dir.join("Documents");
    let save = |version: PesVersion, account: &str, modified: SystemTime| {
        let layout = save_layout(version);
        let folder = documents
            .join("KONAMI")
            .join(layout.game_folder)
            .join(account)
            .join("save");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join(layout.file_name);
        std::fs::write(&path, fixture_save(version)).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(modified)
            .unwrap();
        path
    };
    let now = SystemTime::now();
    let hour_ago = now - Duration::from_secs(3600);
    let pes21_old = save(PesVersion::Pes21, "292733975847239680", hour_ago);
    let pes21_new = save(
        PesVersion::Pes21,
        "76561199320748656",
        now - Duration::from_secs(1800),
    );
    let pes19 = save(PesVersion::Pes19, "292733975847239680", now);
    let elsewhere_dir = dir.join("elsewhere");
    std::fs::create_dir_all(&elsewhere_dir).unwrap();
    let elsewhere = elsewhere_dir.join("EDIT00000000");
    std::fs::write(&elsewhere, fixture_save(PesVersion::Pes17)).unwrap();
    let other_dir = dir.join("other");
    std::fs::create_dir_all(&other_dir).unwrap();
    let other = other_dir.join("EDIT00000000");
    std::fs::write(&other, fixture_save(PesVersion::Pes21)).unwrap();
    Tree {
        documents,
        pes21_old,
        pes21_new,
        pes19,
        elsewhere,
        other,
    }
}

/// Discovery over `documents` for `view`, counting its calls.
fn discover_counted(view: &mut SaveView, documents: &Path) -> Rc<Cell<usize>> {
    let calls = Rc::new(Cell::new(0));
    let counter = Rc::clone(&calls);
    let documents = documents.to_path_buf();
    view.discover = Box::new(move |version| {
        counter.set(counter.get() + 1);
        discover_savefiles_in(&documents, version)
    });
    calls
}

/// The File menu's picker stubbed to cancel, recording the folder each
/// call starts in.
fn picker_starts(view: &mut SaveView) -> Rc<RefCell<Vec<Option<PathBuf>>>> {
    let starts = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&starts);
    view.open_dialog = Box::new(move |start| {
        record.borrow_mut().push(start.map(Path::to_path_buf));
        None
    });
    starts
}

/// The File menu's entries top to bottom: the buttons in the column of
/// `first`, from it down.
fn entries<'a>(nodes: &'a [accesskit::Node], first: &str) -> Vec<&'a accesskit::Node> {
    let first = menu_item(nodes, first).bounds().expect("bounds");
    let mut entries: Vec<(f64, &accesskit::Node)> = nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.label().is_some())
        .filter_map(|node| {
            let bounds = node.bounds()?;
            ((bounds.x0 - first.x0).abs() < 0.5 && bounds.y0 >= first.y0)
                .then_some((bounds.y0, node))
        })
        .collect();
    entries.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered entries"));
    entries.into_iter().map(|(_, node)| node).collect()
}

/// The PES 19 entry's label, the first of the menu (its shortcut follows).
const PES19_ENTRY: &str = "PES 2019";

/// `pressed` with Ctrl held, on one frame of the view.
fn press_ctrl(window: &mut Headless, view: &mut SaveView, pressed: egui::Key) {
    window.frame(vec![key(pressed, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, view, &|| Vec::new())
    });
}

/// Whether the frame's nodes hold the File menu's popup (a real open
/// popup, not the welcome panel's copy of the opens).
fn menu_shown(window: &Headless, nodes: &[accesskit::Node]) -> bool {
    egui::Popup::is_any_open(&window.ctx)
        && nodes.iter().any(|node| {
            node.role() == Role::Button
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Open\u{2026}"))
        })
}

/// `heading`'s label bounds sit between `above` (the separator above its
/// group) and `below` (the group's first entry's top): the heading over
/// its group, as the File menu's and the welcome panel's headings stand.
fn heading_between(nodes: &[accesskit::Node], heading: &str, above: f64, below: f64) {
    let bounds = label_bounds(nodes, heading);
    assert!(
        above <= bounds.y0 && bounds.y1 <= below,
        "{heading} between {above} and {below}: {bounds:?}"
    );
}

/// The y of each separator line in `stroke` spanning `entry`'s x range at
/// or under `top`, as `menu_entries` finds them (`ui.separator` in the
/// panel is the same `noninteractive` stroke the menu's custom one draws).
fn separator_lines(
    shapes: &[egui::epaint::ClippedShape],
    entry: accesskit::Rect,
    stroke: egui::Stroke,
    top: f64,
) -> Vec<f64> {
    let mut lines: Vec<f64> = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::LineSegment { points, stroke: s }
                if *s == stroke
                    && points[0].y == points[1].y
                    && f64::from(points[0].y) >= top
                    && f64::from(points[0].x.min(points[1].x)) <= entry.x0 + 8.0
                    && f64::from(points[0].x.max(points[1].x)) >= entry.x1 - 8.0 =>
            {
                Some(f64::from(points[0].y))
            }
            _ => None,
        })
        .collect();
    lines.sort_by(|a, b| a.partial_cmp(b).expect("ordered lines"));
    lines
}

// SE-GUI-26
#[test]
fn the_file_menu_lists_the_writes_then_the_saves_then_close_and_exit() {
    let tree = tree("open_menu_entries");
    let mut view = SaveView::new();
    view.open(&tree.pes21_old);
    let calls = discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();

    let nodes = open_menu(&mut window, &mut view, "File", PES19_ENTRY);
    // The recent entry: the opened save's version in brackets, then its
    // folder and its file name joined by the platform separator.
    let recent = format!("[PES 2021] save{0}EDIT00000000", std::path::MAIN_SEPARATOR);
    let labels: Vec<&str> = entries(&nodes, "Open\u{2026}")
        .iter()
        .filter_map(|node| node.label())
        .collect();
    // A button's AccessKit label is its text, then its shortcut text.
    assert_eq!(
        labels,
        [
            "Open\u{2026} Ctrl+O",
            "Save Ctrl+S",
            "Save as\u{2026} Ctrl+Shift+S",
            "PES 2019 Ctrl+9",
            "PES 2021 Ctrl+1",
            "PES 2021",
            recent.as_str(),
            "Close save Ctrl+W",
            "Exit Ctrl+Q",
        ],
        "the picker and the writes, then the discovered saves in version order, the newest \
         PES 21 first and the first of each version with its shortcut, then the recent file, \
         then Close save and Exit"
    );
    for entry in [
        "Save Ctrl+S",
        "Save as\u{2026} Ctrl+Shift+S",
        "Close save Ctrl+W",
    ] {
        assert!(
            !node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is enabled with a save open"
        );
    }

    // The two group headings are weak labels, not buttons.
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let weak = window.ctx.global_style().visuals.weak_text_color();
    for heading in ["Open from Documents", "Recent files"] {
        assert!(label_value(&nodes, heading), "the {heading} heading");
        assert_eq!(
            painted_label_color(&nodes, &shapes, heading),
            weak,
            "{heading} in the weak colour"
        );
    }
    assert!(
        !nodes.iter().any(|node| node.role() == Role::Button
            && node
                .label()
                .is_some_and(|label| label.starts_with("Open from Documents")
                    || label.starts_with("Recent files"))),
        "the headings are no buttons"
    );

    let (entries_at, lines) = menu_entries(&mut window, &mut view, "File", "Open\u{2026}");
    assert_eq!(lines.len(), 3, "three separators: {lines:?}");
    assert!(
        separated(&entries_at, &lines, 2, 3),
        "the writes apart from the saves"
    );
    assert!(
        separated(&entries_at, &lines, 5, 6),
        "the saves apart from the recents"
    );
    assert!(
        separated(&entries_at, &lines, 6, 7),
        "the recents apart from Close save"
    );
    for (above, below) in [(0, 1), (1, 2), (3, 4), (4, 5), (7, 8)] {
        assert!(
            !separated(&entries_at, &lines, above, below),
            "no separator inside a group: entries {above} and {below}"
        );
    }

    // Each heading stands over its group: under the separator above it,
    // over the group's first entry.
    let nodes = draw(&mut window, &mut view);
    heading_between(&nodes, "Open from Documents", lines[0], entries_at[3].1);
    heading_between(&nodes, "Recent files", lines[1], entries_at[6].1);

    // Each save's and the recent's tooltip is its own full path, found by
    // its place in the menu (the two PES 21 entries read alike); `Open…`'s
    // says where the picker opens.
    let tips = [
        (0usize, "Pick a save file, starting in the folder of the save opened last (or else of the newest save found)".to_owned()),
        (3, tree.pes19.display().to_string()),
        (4, tree.pes21_new.display().to_string()),
        (5, tree.pes21_old.display().to_string()),
        (6, tree.pes21_old.display().to_string()),
    ];
    for (place, tip) in tips {
        let nodes = draw(&mut window, &mut view);
        let at = centre(entries(&nodes, "Open\u{2026}")[place]);
        assert_eq!(
            tooltip_containing(&mut window, &mut view, at, &tip),
            Some(tip.clone()),
            "entry {place}'s tooltip"
        );
    }

    // The menu stayed open through every frame above: discovery ran once
    // per version, when it opened.
    let nodes = draw(&mut window, &mut view);
    assert!(
        menu_shown(&window, &nodes),
        "premise: the menu is still open"
    );
    assert_eq!(calls.get(), PesVersion::ALL.len(), "one call per version");

    // A discovered entry opens its save.
    click(&mut window, menu_item(&nodes, PES19_ENTRY), &mut view);
    assert!(
        !egui::Popup::is_any_open(&window.ctx),
        "the click closed the menu"
    );
    let session = view.session.as_ref().expect("a save is open");
    assert_eq!(session.path(), tree.pes19.as_path());
    assert_eq!(session.version(), PesVersion::Pes19);

    // Ctrl+Shift+S opens Save as's picker even while the Name field holds
    // the keyboard (the stub records the dialog's call; it cancels here,
    // so nothing is written).
    let handed = save_as_stub(&mut view, None);
    let nodes = draw(&mut window, &mut view);
    click(&mut window, name_field(&nodes), &mut view);
    draw(&mut window, &mut view);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Name field holds the keyboard"
    );
    let shift_s = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    window.frame(vec![key(egui::Key::S, shift_s, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        handed.borrow().as_slice(),
        [tree.pes19.as_path()],
        "Ctrl+Shift+S opened the Save as picker on the open file"
    );
}

// A separator stands under the writes (before the Open from Documents
// heading), under the saves' group before the Recent files heading when
// both lists exist, and always before Close save; never elsewhere
// (SE-GUI-26's grouped entries).
#[test]
fn the_file_menus_separators_stand_only_between_its_groups() {
    /// What a `case` frame drew: the File menu's button entries top to
    /// bottom (label, top, bottom), the separators' ys, and the frame's
    /// nodes (the headings and the "No PES saves" line are labels, not
    /// counted entries).
    struct MenuCase {
        entries: Vec<(String, f64, f64)>,
        lines: Vec<f64>,
        nodes: Vec<accesskit::Node>,
    }

    /// The File menu: `documents` discovered (None stubs empty) and
    /// `recent` opened first if given.
    fn case(documents: Option<&Path>, recent: Option<PathBuf>) -> MenuCase {
        let mut view = SaveView::new();
        match documents {
            Some(documents) => {
                discover_counted(&mut view, documents);
            }
            None => view.discover = Box::new(|_| Vec::new()),
        }
        if let Some(recent) = recent {
            view.open(&recent);
        }
        let mut window = Headless::new();
        let (entries, lines) = menu_entries(&mut window, &mut view, "File", "Open\u{2026}");
        let nodes = draw(&mut window, &mut view);
        MenuCase {
            entries,
            lines,
            nodes,
        }
    }

    // Neither list: the writes, then Close save and Exit — the headings
    // and the "No PES saves" line are labels, not counted entries, so both
    // separators land in the gap between Save as… and Close save.
    let MenuCase {
        entries,
        lines,
        nodes,
    } = case(None, None);
    assert_eq!(
        entries.len(),
        5,
        "Open…, Save, Save as, Close save, Exit: {entries:?}"
    );
    assert_eq!(lines.len(), 2, "two separators: {lines:?}");
    assert!(separated(&entries, &lines, 2, 3), "before the groups");
    for (above, below) in [(0, 1), (1, 2), (3, 4)] {
        assert!(
            !separated(&entries, &lines, above, below),
            "no separator inside a group: entries {above} and {below}"
        );
    }
    assert!(
        !label_value(&nodes, "Recent files"),
        "no Recent files heading with no recent"
    );
    assert!(
        label_value(&nodes, "No PES saves were found in Documents."),
        "the no-saves line"
    );

    // Discovered only.
    let tree = tree("open_menu_sep_discovered");
    let MenuCase {
        entries,
        lines,
        nodes,
    } = case(Some(&tree.documents), None);
    assert_eq!(
        entries.len(),
        8,
        "the opens, three saves, Close save, Exit: {entries:?}"
    );
    assert_eq!(lines.len(), 2, "two separators: {lines:?}");
    assert!(separated(&entries, &lines, 2, 3), "before the saves");
    assert!(separated(&entries, &lines, 5, 6), "before Close save");
    for (above, below) in [(0, 1), (1, 2), (3, 4), (4, 5), (6, 7)] {
        assert!(
            !separated(&entries, &lines, above, below),
            "no separator inside a group: entries {above} and {below}"
        );
    }
    assert!(
        !label_value(&nodes, "Recent files"),
        "no Recent files heading with no recent"
    );
    assert!(
        !label_value(&nodes, "No PES saves were found in Documents."),
        "no no-saves line with saves found"
    );

    // Recents only: the saves' group's separator and the Recent files
    // heading's sit between Save as… and the recent.
    let MenuCase {
        entries,
        lines,
        nodes,
    } = case(None, Some(tree.elsewhere.clone()));
    assert_eq!(
        entries.len(),
        6,
        "the opens, the recent, Close save, Exit: {entries:?}"
    );
    assert_eq!(lines.len(), 3, "three separators: {lines:?}");
    assert!(
        separated(&entries, &lines, 2, 3),
        "before the groups and before the recents"
    );
    assert!(separated(&entries, &lines, 3, 4), "before Close save");
    for (above, below) in [(0, 1), (1, 2), (4, 5)] {
        assert!(
            !separated(&entries, &lines, above, below),
            "no separator inside a group: entries {above} and {below}"
        );
    }
    assert!(
        label_value(&nodes, "Recent files"),
        "the Recent files heading"
    );
    assert!(
        label_value(&nodes, "No PES saves were found in Documents."),
        "the no-saves line"
    );

    // Both.
    let MenuCase {
        entries,
        lines,
        nodes,
    } = case(Some(&tree.documents), Some(tree.elsewhere.clone()));
    assert_eq!(
        entries.len(),
        9,
        "the opens, the saves, the recent, Close save, Exit: {entries:?}"
    );
    assert_eq!(lines.len(), 3, "three separators: {lines:?}");
    assert!(separated(&entries, &lines, 2, 3), "before the saves");
    assert!(separated(&entries, &lines, 5, 6), "before the recents");
    assert!(separated(&entries, &lines, 6, 7), "before Close save");
    for (above, below) in [(0, 1), (1, 2), (3, 4), (4, 5), (7, 8)] {
        assert!(
            !separated(&entries, &lines, above, below),
            "no separator inside a group: entries {above} and {below}"
        );
    }
    assert!(
        label_value(&nodes, "Recent files"),
        "the Recent files heading"
    );
    assert!(
        !label_value(&nodes, "No PES saves were found in Documents."),
        "no no-saves line with saves found"
    );
}

// SE-GUI-26
#[test]
fn the_picker_starts_in_the_last_opened_folder_then_the_newest_found_saves() {
    let tree = tree("open_menu_choose_file");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let starts = picker_starts(&mut view);
    let mut window = Headless::new();

    // Nothing remembered from an earlier open: the picker starts in the
    // newest discovered save's folder, of whatever version (PES 19's here,
    // the newest of the three).
    choose(&mut window, &mut view, "File", "Open\u{2026}");
    assert_eq!(
        *starts.borrow(),
        [tree.pes19.parent().map(Path::to_path_buf)],
        "the newest discovered save's folder"
    );

    // A save opened from a backups folder is remembered: the picker returns
    // there while the folder exists, and falls back to the newest discovered
    // save's once it is gone.
    let backups = crate::test_support::scratch("open_menu_backups").join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let backup_save = backups.join("EDIT00000000");
    std::fs::write(&backup_save, fixture_save(PesVersion::Pes21)).unwrap();
    view.open(&backup_save);
    choose(&mut window, &mut view, "File", "Open\u{2026}");
    std::fs::remove_dir_all(&backups).unwrap();
    choose(&mut window, &mut view, "File", "Open\u{2026}");
    assert_eq!(
        *starts.borrow(),
        [
            tree.pes19.parent().map(Path::to_path_buf),
            Some(backups.clone()),
            tree.pes19.parent().map(Path::to_path_buf),
        ],
        "the last opened folder while it exists, the newest discovered's else"
    );

    // Nothing discovered and nothing remembered: the picker starts in no
    // folder.
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    let starts = picker_starts(&mut view);
    choose(&mut window, &mut view, "File", "Open\u{2026}");
    assert_eq!(*starts.borrow(), [None], "nothing discovered: no folder");
}

// The picker's start folder is asked once per look of the menu or the
// panel, never a frame: the folder the look found is kept while the look
// is, and the next look resolves it again.
#[test]
fn the_picker_start_is_asked_once_per_look_not_every_frame() {
    let tree = tree("picker_start_once");
    let mut view = SaveView::new();
    view.open(&tree.elsewhere);
    view.session = None;
    discover_counted(&mut view, &tree.documents);
    let starts = picker_starts(&mut view);
    let mut window = Headless::new();

    // The welcome panel's look found `last_folder` (the elsewhere save's
    // folder): deleted under the kept look, the picker still starts there.
    idle_frames(&mut window, &mut view, 3);
    std::fs::remove_dir_all(tree.elsewhere.parent().unwrap()).unwrap();
    let nodes = draw(&mut window, &mut view);
    click(&mut window, menu_item(&nodes, "Open\u{2026}"), &mut view);
    assert_eq!(
        *starts.borrow(),
        [tree.elsewhere.parent().map(Path::to_path_buf)],
        "the kept look's folder, gone or not"
    );

    // A new look — the File menu opened and closed — resolves the start
    // again: the folder gone, the picker starts in the newest discovered
    // save's.
    open_menu(&mut window, &mut view, "File", "Open\u{2026}");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    click(&mut window, menu_item(&nodes, "Open\u{2026}"), &mut view);
    assert_eq!(
        *starts.borrow(),
        [
            tree.elsewhere.parent().map(Path::to_path_buf),
            tree.pes19.parent().map(Path::to_path_buf),
        ],
        "the kept look's folder first, then the newest discovered save's"
    );
}

// SE-GUI-05, SE-GUI-26: one of the File menu's opens under unsaved changes
// asks first; Discard opens the entry's save and leaves the open file as
// it was.
// SE-GUI-05, SE-GUI-26
#[test]
fn an_entry_clicked_with_unsaved_changes_prompts_and_discard_opens_it() {
    let tree = tree("open_menu_dirty");
    let first = fixture(PesVersion::Pes21, "open_menu_dirty_first");
    let original = std::fs::read(&first).unwrap();
    let mut view = SaveView::new();
    view.open(&first);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();

    choose(&mut window, &mut view, "File", PES19_ENTRY);
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Unsaved changes"),
        "the unsaved-changes prompt is up"
    );
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        first.as_path(),
        "nothing opened yet"
    );

    let discard = node_labeled(&nodes, Role::Button, "Discard");
    click(&mut window, discard, &mut view);
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        tree.pes19.as_path(),
        "Discard opened the entry's save"
    );
    assert_eq!(
        std::fs::read(&first).unwrap(),
        original,
        "the first file is unchanged"
    );
}

// SE-GUI-26
#[test]
fn a_versions_shortcut_opens_its_newest_save_with_the_menu_closed() {
    let tree = tree("open_shortcut");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    assert!(view.session.is_none(), "premise: no save open");
    assert!(!menu_shown(&window, &nodes), "premise: the menu is closed");

    press_ctrl(&mut window, &mut view, egui::Key::Num1);
    let session = view.session.as_ref().expect("Ctrl+1 opened a save");
    assert_eq!(
        session.path(),
        tree.pes21_new.as_path(),
        "the newer PES 21 save"
    );
    assert_eq!(session.version(), PesVersion::Pes21);

    press_ctrl(&mut window, &mut view, egui::Key::Num9);
    let session = view.session.as_ref().expect("a save is open");
    assert_eq!(session.path(), tree.pes19.as_path(), "Ctrl+9's save");
    assert_eq!(session.version(), PesVersion::Pes19);
}

// SE-GUI-26
#[test]
fn a_shortcut_with_no_save_found_opens_the_picker() {
    let tree = tree("open_shortcut_picker");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let starts = picker_starts(&mut view);
    let mut window = Headless::new();
    draw(&mut window, &mut view);

    // No PES 17 save under the test's Documents: the picker starts in the
    // newest discovered save's folder, of whatever version.
    press_ctrl(&mut window, &mut view, egui::Key::Num7);
    assert_eq!(
        *starts.borrow(),
        [tree.pes19.parent().map(Path::to_path_buf)],
        "Ctrl+7 opened the picker once, in the newest discovered save's folder"
    );
    // Ctrl+O asks the same start rule.
    press_ctrl(&mut window, &mut view, egui::Key::O);
    assert_eq!(
        *starts.borrow(),
        [
            tree.pes19.parent().map(Path::to_path_buf),
            tree.pes19.parent().map(Path::to_path_buf),
        ],
        "Ctrl+O opened the picker in the same folder"
    );

    // A folder is remembered (`last_folder`, the folder the last save was
    // opened from): Ctrl+O starts the picker there, ahead of the newest
    // discovered save's.
    view.last_folder = Some(tree.documents.clone());
    press_ctrl(&mut window, &mut view, egui::Key::O);
    assert_eq!(
        starts.borrow().last(),
        Some(&Some(tree.documents.clone())),
        "Ctrl+O opened the picker in the remembered folder"
    );
    assert!(view.session.is_none(), "a cancelled picker opens nothing");
}

// SE-GUI-05, SE-GUI-26: a shortcut under unsaved changes asks first, as
// the File menu's opens do; Discard opens the version's save.
// SE-GUI-05, SE-GUI-26
#[test]
fn a_shortcut_with_unsaved_changes_prompts_first() {
    let tree = tree("open_shortcut_dirty");
    let first = fixture(PesVersion::Pes21, "open_shortcut_dirty_first");
    let original = std::fs::read(&first).unwrap();
    let mut view = SaveView::new();
    view.open(&first);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();
    draw(&mut window, &mut view);

    press_ctrl(&mut window, &mut view, egui::Key::Num9);
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Unsaved changes"),
        "the unsaved-changes prompt is up"
    );
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        first.as_path(),
        "nothing opened yet"
    );

    let discard = node_labeled(&nodes, Role::Button, "Discard");
    click(&mut window, discard, &mut view);
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        tree.pes19.as_path(),
        "Discard opened PES 19's save"
    );
    assert_eq!(
        std::fs::read(&first).unwrap(),
        original,
        "the first file is unchanged"
    );
}

// SE-GUI-26
#[test]
fn the_open_shortcuts_wait_while_a_field_holds_the_keyboard() {
    let tree = tree("open_shortcut_typing");
    let first = fixture(PesVersion::Pes21, "open_shortcut_typing_first");
    let mut view = SaveView::new();
    view.open(&first);
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    click(&mut window, name_field(&nodes), &mut view);
    draw(&mut window, &mut view);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Name field holds the keyboard"
    );
    // Nothing unsaved: a Ctrl+9 that went through would open at once.
    assert!(!view.is_dirty(), "premise: nothing unsaved");

    press_ctrl(&mut window, &mut view, egui::Key::Num9);
    draw(&mut window, &mut view);
    assert_eq!(
        view.session.as_ref().unwrap().path(),
        first.as_path(),
        "Ctrl+9 opened nothing"
    );
    assert!(view.prompt.is_none(), "and raised no prompt");
}

// SE-GUI-26
#[test]
fn ctrl_shift_and_a_digit_opens_nothing() {
    let tree = tree("open_shortcut_shift");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();
    draw(&mut window, &mut view);

    let shift = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    window.frame(vec![key(egui::Key::Num9, shift, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(view.session.is_none(), "Ctrl+Shift+9 opened nothing");
    press_ctrl(&mut window, &mut view, egui::Key::Num9);
    assert_eq!(
        view.session.as_ref().map(|session| session.path()),
        Some(tree.pes19.as_path()),
        "premise: Ctrl+9 alone opens PES 19's save"
    );
}

/// The open save edited (player 70101's height), `target` the path Save
/// as's picker returns, discovery stubbed to find no save (the File menu
/// runs it when it opens).
fn edited_view(path: &Path, target: &Path) -> SaveView {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    view.open(path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let target = target.to_path_buf();
    view.save_as_dialog = Box::new(move |_| Some(target.clone()));
    view
}

/// `path` with `.bak` after its name, where a write keeps the file it
/// replaces.
fn bak_of(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.bak", path.display()))
}

/// The Save as picker stubbed to return `answer` (`None` cancels),
/// recording the path it is handed (the open file's, as `save_as` starts
/// the picker there).
fn save_as_stub(view: &mut SaveView, answer: Option<&Path>) -> Rc<RefCell<Vec<PathBuf>>> {
    let handed = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&handed);
    let answer = answer.map(Path::to_path_buf);
    view.save_as_dialog = Box::new(move |current| {
        record.borrow_mut().push(current.to_path_buf());
        answer.clone()
    });
    handed
}

// SE-GUI-26, SE-GUI-79
#[test]
fn save_as_writes_the_picked_path_and_keeps_the_replaced_file_once() {
    let path = fixture(PesVersion::Pes21, "save_as_replace");
    let dir = path.parent().unwrap().to_path_buf();
    let target = dir.join("other").join("EDIT00000000");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let replaced = fixture_save(PesVersion::Pes17);
    std::fs::write(&target, &replaced).unwrap();
    let original = std::fs::read(&path).unwrap();

    let mut tool = crate::Tool::new();
    tool.view = edited_view(&path, &target);
    let handed = save_as_stub(&mut tool.view, Some(&target));
    let mut window = Headless::new();
    choose(&mut window, &mut tool.view, "File", "Save as\u{2026}");
    assert_eq!(
        handed.borrow().as_slice(),
        [path.as_path()],
        "the picker is handed the open file's path"
    );

    let session = tool.view.session.as_ref().unwrap();
    let reopened = Session::open(&target, Recorder::default()).expect("the target holds a save");
    // `assert!`, not `assert_eq!`: a failure would print every record.
    assert!(
        reopened.file().players() == session.file().players()
            && reopened.file().teams() == session.file().teams(),
        "the target reopens as the model"
    );
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the edit reached the target"
    );
    assert_eq!(
        std::fs::read(bak_of(&target)).unwrap(),
        replaced,
        "the replaced file is kept as its .bak"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the opened file is unchanged"
    );
    assert!(!tool.view.is_dirty(), "the written model is the baseline");
    assert_eq!(
        tool.title_detail(),
        Some(target.display().to_string()),
        "the title names the new path"
    );
    assert_eq!(
        tool.view.recents.first(),
        Some(&crate::settings::RecentSave {
            path: target.clone(),
            version: PesVersion::Pes21,
        }),
        "the recents' top, with the save's version stored"
    );
    assert_eq!(
        tool.view
            .notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some((
            "EDIT00000000 saved (the replaced file is kept once as EDIT00000000.bak)",
            NoticeKind::Success
        )),
        "the notice names the kept .bak, in the success colour"
    );

    // A fresh path: written, and no `.bak` beside it.
    let fresh = dir.join("fresh").join("EDIT00000000");
    std::fs::create_dir_all(fresh.parent().unwrap()).unwrap();
    let mut view = edited_view(&path, &fresh);
    choose(&mut window, &mut view, "File", "Save as\u{2026}");
    assert!(fresh.is_file(), "the save is written");
    assert!(!bak_of(&fresh).exists(), "no .bak for a fresh path");
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some(("EDIT00000000 saved", NoticeKind::Success)),
        "the plain saved notice, nothing replaced, in the success colour"
    );
}

// A cancelled Save as writes nothing: the session keeps the open file's
// path and its dirty mark, and the file on disk stands as it was.
#[test]
fn a_cancelled_save_as_writes_nothing() {
    let path = fixture(PesVersion::Pes21, "save_as_cancelled");
    let original = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    view.save_as_dialog = Box::new(|_| None);
    let mut window = Headless::new();

    choose(&mut window, &mut view, "File", "Save as\u{2026}");

    assert_eq!(
        view.save_path(),
        Some(path.as_path()),
        "the session's path is the open file's"
    );
    assert!(view.is_dirty(), "the edit is still pending");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the file is unchanged"
    );
}

// SE-GUI-21, SE-GUI-26: Save as with the save's own PES running asks the
// running-game question first; its Save writes to the Save as target.
// SE-GUI-21, SE-GUI-26
#[test]
fn save_as_with_the_game_running_asks_then_writes_the_target() {
    let path = fixture(PesVersion::Pes21, "save_as_running");
    let original = std::fs::read(&path).unwrap();
    let target = path.parent().unwrap().join("target_EDIT00000000");
    let running_own = || {
        vec![RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }]
    };
    let mut view = edited_view(&path, &target);
    let mut window = Headless::new();

    let nodes = open_menu(&mut window, &mut view, "File", "Save as\u{2026}");
    let save_as = centre(menu_item(&nodes, "Save as\u{2026}"));
    press_release(&mut window, save_as, |ui| show(ui, &mut view, &running_own));
    assert!(!target.exists(), "nothing written while the question waits");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &running_own));
    let save = centre(modal_button(&pairs, "PES 2021 is running", "Save"));
    press_release(&mut window, save, |ui| show(ui, &mut view, &running_own));

    let reopened =
        Session::open(&target, Recorder::default()).expect("the question's Save wrote the target");
    assert_eq!(reopened.file().player(70101).unwrap().basic.height, 170);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "the opened file is unchanged"
    );
    assert_eq!(view.save_path(), Some(target.as_path()));
}

// Ctrl+Shift+S is the File menu's Save as…, taken as Ctrl+S is: the
// picker (handed the open file), then the write to its path — the menu
// entry's own path, the running-game check included.
#[test]
fn ctrl_shift_s_runs_the_save_as_picker_and_write() {
    let path = fixture(PesVersion::Pes21, "ctrl_shift_s");
    let before = std::fs::read(&path).unwrap();
    let target = path.parent().unwrap().join("shift_s_EDIT00000000");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    let handed = save_as_stub(&mut view, Some(&target));
    let mut window = Headless::new();

    let shift_s = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    window.frame(vec![key(egui::Key::S, shift_s, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        handed.borrow().as_slice(),
        [path.as_path()],
        "the picker is handed the open file's path"
    );
    let reopened = Session::open(&target, Recorder::default()).expect("the target holds a save");
    assert_eq!(
        reopened.file().player(70101).unwrap().basic.height,
        170,
        "the edit reached the target"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "the open file is unchanged"
    );
    assert_eq!(
        view.save_path(),
        Some(target.as_path()),
        "the editor carries on with the target"
    );
    assert!(!view.is_dirty(), "the written model is the baseline");
}

// A save written mid-gesture seals the open undo entry: the next edit is
// its own step, so one Undo goes back only to the save, never past it.
#[test]
fn a_save_in_mid_gesture_seals_the_undo_entry() {
    let path = fixture(PesVersion::Pes21, "save_mid_gesture");
    let mut view = SaveView::new();
    view.open(&path);
    let (_, _, player_id) = selected_player(&view).expect("a player row is selected");
    let before = player_entry(&view, player_id).name;
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    click(&mut window, name_field(&nodes), &mut view);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Name field holds the keyboard"
    );

    // A run of typing, then Ctrl+S without leaving the field: the file
    // now holds the first run.
    window.frame(
        vec![
            key(egui::Key::End, egui::Modifiers::NONE, true),
            egui::Event::Text("x".to_owned()),
        ],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let first = player_entry(&view, player_id).name;
    assert_eq!(first, format!("{before}x"), "premise: the first run landed");
    press_ctrl(&mut window, &mut view, egui::Key::S);
    assert!(!view.is_dirty(), "premise: the save wrote the first run");

    // Still typing into the same field: the second run. Then the field
    // is left and one Undo takes only the second run back — the model
    // reads the saved state again.
    window.frame(vec![egui::Event::Text("y".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert!(view.is_dirty(), "the second run landed");
    // The toolbar's empty end: dead space no widget covers.
    click_at(&mut window, egui::pos2(1090.0, 12.0), &mut view);
    press_ctrl(&mut window, &mut view, egui::Key::Z);
    assert_eq!(
        player_entry(&view, player_id).name,
        first,
        "only the run after the save is undone"
    );
    assert!(!view.is_dirty(), "the editor reads saved");
}

// SE-GUI-79
#[test]
fn ctrl_s_names_the_saved_file_and_a_refused_write_publishes_no_notice() {
    let path = fixture(PesVersion::Pes21, "save_notice");
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    let mut window = Headless::new();

    window.frame(vec![key(egui::Key::S, egui::Modifiers::CTRL, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        view.notice
            .as_ref()
            .map(|notice| (notice.text.as_str(), notice.kind)),
        Some((
            "EDIT00000000 saved (the replaced file is kept once as EDIT00000000.bak)",
            NoticeKind::Success
        )),
        "the file stood at the target, so the notice names the kept .bak, in the success colour"
    );

    // A target whose folder does not exist: the write fails, the refusal
    // shows, no notice.
    let dir = path.parent().unwrap().to_path_buf();
    let missing = dir.join("missing").join("EDIT00000000");
    view.notice = None;
    view.discover = Box::new(|_| Vec::new());
    view.save_as_dialog = Box::new(move |_| Some(missing.clone()));
    choose(&mut window, &mut view, "File", "Save as\u{2026}");
    assert!(
        view.refusal.is_some(),
        "the refused write shows the refusal"
    );
    assert!(view.notice.is_none(), "a refused write publishes no notice");
}

// A failed open leaves the open save's journal alone and records the file
// that failed — the one a member reports: the earlier save's step and
// input stay, and the refusal's own step and its bytes join them.
#[test]
fn a_failed_open_keeps_the_journal_and_records_the_file_that_failed() {
    let path = fixture(PesVersion::Pes21, "journal_failed_open");
    let dir = path.parent().unwrap().to_path_buf();
    let bad = dir.join("notasave.txt");
    std::fs::write(&bad, b"not a save").unwrap();
    let recorder = Recorder::default();
    let mut view = SaveView::new();
    view.recorder = recorder.clone();

    view.open(&path);
    view.open(&bad);

    let lines = journal_lines(&recorder, &dir);
    assert!(
        lines.iter().any(|line| line.starts_with("opened ")),
        "the first save's step survives the failed open: {lines:?}"
    );
    let refused = steps_with(&lines, "could not open ")
        .into_iter()
        .next()
        .expect("the failed open records its own step");
    assert!(
        refused.contains(&bad.display().to_string()),
        "the step names the file that failed: {refused}"
    );
    let bundle = dir.join("journal.zip");
    assert_eq!(
        zip_entry(&bundle, "inputs/0-EDIT00000000"),
        fixture_save(PesVersion::Pes21),
        "the first save's input stays"
    );
    assert_eq!(
        zip_entry(&bundle, "inputs/1-notasave.txt"),
        b"not a save",
        "the failed file's bytes are kept as an input"
    );

    // A file that cannot be read at all records the step and keeps
    // nothing.
    view.open(&dir.join("no-such-file"));
    let lines = journal_lines(&recorder, &dir);
    assert_eq!(steps_with(&lines, "could not open ").len(), 2, "{lines:?}");
    assert!(
        zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap())
            .unwrap()
            .by_name("inputs/2-no-such-file")
            .is_err(),
        "a file that cannot be read keeps nothing"
    );
}

// SE-GUI-90: with no save open the middle of the view is the welcome
// panel, titled "Save editor" — the File menu's opens in the menu's
// order, the strong headings included, no modal up.
// SE-GUI-90
#[test]
fn the_welcome_panel_lists_the_opens_and_a_click_opens_its_save() {
    let tree = tree("welcome_panel");
    let mut view = SaveView::new();
    // Two saves in folders outside Documents opened in an earlier run,
    // `other` the more recent: the `recent_files` list.
    view.open(&tree.elsewhere);
    view.open(&tree.other);
    view.session = None;
    discover_counted(&mut view, &tree.documents);
    let starts = picker_starts(&mut view);
    let mut window = Headless::new();

    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Save editor"),
        "the panel's title in the middle of the view"
    );
    assert!(
        label_value(&nodes, "Open from Documents"),
        "the saves' heading"
    );
    assert!(label_value(&nodes, "Recent files"), "the recents' heading");
    // Each recent entry: the save's version in brackets, its folder and
    // its file name; the list reads the last opened first.
    let other = format!("[PES 2021] other{0}EDIT00000000", std::path::MAIN_SEPARATOR);
    let recent = format!(
        "[PES 2017] elsewhere{0}EDIT00000000",
        std::path::MAIN_SEPARATOR
    );
    let labels: Vec<&str> = entries(&nodes, "Open\u{2026}")
        .iter()
        .filter_map(|node| node.label())
        .collect();
    assert_eq!(
        labels,
        [
            "Open\u{2026} Ctrl+O",
            "PES 2019 Ctrl+9",
            "PES 2021 Ctrl+1",
            "PES 2021",
            other.as_str(),
            recent.as_str(),
        ],
        "the picker, then the saves by version under Open from Documents, \
         the first of each with its shortcut, then the recents newest first \
         under Recent files"
    );
    // Each heading's text starts where an entry's does (the label's and
    // the button's painted text's left edges).
    let (_, _, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let entry_left = text_shape(&shapes, "Open\u{2026}").pos.x;
    for heading in ["Open from Documents", "Recent files"] {
        let left = text_shape(&shapes, heading).pos.x;
        assert!(
            (left - entry_left).abs() <= 0.5,
            "{heading}'s text at {left}, an entry's at {entry_left}"
        );
    }
    // Each heading stands over its group, under the separator above it.
    let sep = window
        .ctx
        .global_style()
        .visuals
        .widgets
        .noninteractive
        .bg_stroke;
    let rows = entries(&nodes, "Open\u{2026}");
    let first = rows[0].bounds().expect("an entry's bounds");
    let lines = separator_lines(&shapes, first, sep, first.y0);
    heading_between(
        &nodes,
        "Open from Documents",
        lines[0],
        rows[1].bounds().unwrap().y0,
    );
    heading_between(
        &nodes,
        "Recent files",
        lines[1],
        rows[4].bounds().unwrap().y0,
    );
    assert!(
        window
            .ctx
            .memory(|memory| memory.top_modal_layer())
            .is_none(),
        "the panel registers no modal layer"
    );
    // The rows fill the frame, a dialog's 360 points less its 12-point
    // margins and 1-point stroke, as the draft's do; `Open…` is as wide.
    let width = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("an entry's bounds");
        bounds.x1 - bounds.x0
    };
    let rows = entries(&nodes, "Open\u{2026}");
    for row in &rows {
        assert!(
            (width(row) - 334.0).abs() <= 0.5,
            "{:?} is {:.1} wide",
            row.label(),
            width(row)
        );
    }

    // Each save's and the recent's entry carries its path as the tooltip,
    // as the File menu's entries do.
    for (row, path) in rows[1..].iter().zip([
        &tree.pes19,
        &tree.pes21_new,
        &tree.pes21_old,
        &tree.other,
        &tree.elsewhere,
    ]) {
        let tip = path.display().to_string();
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(row), &tip),
            Some(tip),
            "{:?}'s tooltip is its path",
            row.label()
        );
    }

    // The panel's Open… does the menu's: the picker in the folder the
    // last save was opened from while it exists.
    let nodes = draw(&mut window, &mut view);
    click(&mut window, menu_item(&nodes, "Open\u{2026}"), &mut view);
    assert_eq!(
        *starts.borrow(),
        [tree.other.parent().map(Path::to_path_buf)],
        "the picker started in the last opened save's folder"
    );
    assert!(view.session.is_none(), "a cancelled picker opens nothing");

    let nodes = draw(&mut window, &mut view);
    click(&mut window, menu_item(&nodes, PES19_ENTRY), &mut view);
    let session = view.session.as_ref().expect("the PES 19 entry opened");
    assert_eq!(session.path(), tree.pes19.as_path());
    let nodes = draw(&mut window, &mut view);
    assert!(
        !label_value(&nodes, "Save editor"),
        "the panel is gone with a save open"
    );

    // The earlier run's save deleted: skipped in the list — the save
    // opened above keeps the heading — and once every recent is gone,
    // no Recent files heading.
    view.session = None;
    std::fs::remove_file(&tree.elsewhere).unwrap();
    idle_frames(&mut window, &mut view, 2);
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Recent files"),
        "the remaining recent keeps the heading"
    );
    assert!(
        !nodes.iter().any(|node| node.role() == Role::Button
            && node
                .label()
                .is_some_and(|label| label.starts_with("[PES 2017] elsewhere"))),
        "the gone save is skipped"
    );
    // Whether a recent save is still on disk is asked once per stay of the
    // panel, as discovery is, never a frame: deleted while the panel is
    // up, the save stays listed; the File menu's next look drops it, and
    // with every recent save gone, the heading.
    std::fs::remove_file(&tree.pes19).unwrap();
    idle_frames(&mut window, &mut view, 2);
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Recent files"),
        "the panel's look is kept while it stays up"
    );
    std::fs::remove_file(&tree.other).unwrap();
    open_menu(&mut window, &mut view, "File", "Open\u{2026}");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    idle_frames(&mut window, &mut view, 2);
    let nodes = draw(&mut window, &mut view);
    assert!(
        !label_value(&nodes, "Recent files"),
        "no Recent files heading once every recent save is gone"
    );
    assert_eq!(view.recents.len(), 3, "the gone saves stay in the list");
}

// SE-GUI-90's "no save discovered" clause: the weak line under the Open
// from Documents heading.
// SE-GUI-90
#[test]
fn the_welcome_panel_with_nothing_found_says_so_under_the_heading() {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    let mut window = Headless::new();

    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    let line = label_bounds(&nodes, "No PES saves were found in Documents.");
    let heading = label_bounds(&nodes, "Open from Documents");
    let picker = menu_item(&nodes, "Open\u{2026}")
        .bounds()
        .expect("Open…'s bounds");
    assert!(picker.y0 < heading.y0, "Open… first");
    assert!(heading.y0 < line.y0, "the weak line under the heading");
    assert!(
        !label_value(&nodes, "Recent files"),
        "no Recent files heading with no recent file"
    );
}

// SE-GUI-90's "the middle of the view": the panel's frame centres in
// the area the panes fill — under the toolbar's band, ending
// `PANE_BOTTOM_MARGIN` over the pane's bottom — not in the whole
// central panel, which the band's strip belongs to.
// SE-GUI-90
#[test]
fn the_welcome_panel_centres_in_the_area_under_the_toolbar() {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    let mut window = Headless::new();
    idle_frames(&mut window, &mut view, 3);
    let (_, nodes, shapes) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));

    // The panel's frame: the smallest window-coloured rect the title
    // sits in (the central panel's fill is the same colour, and the
    // frame's the smaller one; the frame's own shape nests inside a
    // `Shape::Vec` with its shadow).
    let title = label_bounds(&nodes, "Save editor");
    let title_mid = egui::pos2(
        ((title.x0 + title.x1) / 2.0) as f32,
        ((title.y0 + title.y1) / 2.0) as f32,
    );
    fn rect_shapes(shape: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
        match shape {
            egui::Shape::Rect(rect) => out.push(rect.clone()),
            egui::Shape::Vec(shapes) => {
                shapes.iter().for_each(|shape| rect_shapes(shape, out));
            }
            _ => {}
        }
    }
    let window_fill = window.ctx.global_style().visuals.window_fill();
    let mut rects = Vec::new();
    for clipped in &shapes {
        rect_shapes(&clipped.shape, &mut rects);
    }
    let frame = rects
        .iter()
        .filter(|rect| rect.fill == window_fill && rect.rect.contains(title_mid))
        .min_by(|a, b| a.rect.height().total_cmp(&b.rect.height()))
        .expect("the welcome panel's frame")
        .rect;

    // The panes' area: from one item spacing under the band's bottom
    // line (the File button's bottom plus the band's margin, the line's
    // stroke and the item spacing the cursor lands past it) to
    // `PANE_BOTTOM_MARGIN` over the pane's bottom.
    let button = menu_button(&nodes, "File")
        .bounds()
        .expect("the File menu button's bounds");
    let style = window.ctx.global_style();
    let stroke = style.visuals.widgets.noninteractive.bg_stroke.width;
    let top = button.y1
        + f64::from(BAND_MARGIN.bottom)
        + f64::from(stroke)
        + f64::from(style.spacing.item_spacing.y);
    let bottom = f64::from(window.screen.y) - f64::from(PANE_BOTTOM_MARGIN);
    let middle = f64::from(frame.center().y);
    assert!(
        (middle - (top + bottom) / 2.0).abs() <= 1.0,
        "the frame centred in the area: middle {middle}, area {top}..{bottom}"
    );
}

// The panel scans the disk once when it appears and keeps the list while
// it stays up: never a scan per frame.
#[test]
fn the_welcome_panel_scans_once_not_every_frame() {
    let tree = tree("welcome_once");
    let mut view = SaveView::new();
    let calls = discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();

    idle_frames(&mut window, &mut view, 5);
    assert_eq!(
        calls.get(),
        PesVersion::ALL.len(),
        "one discovery call per version, once for the panel's whole stay"
    );
}

// The panel's list is the File menu's scan's: a save that lands in
// Documents while the window is open shows in the menu reopened and in
// the panel (each opening of the menu rescans, the last list never kept).
#[test]
fn a_save_added_after_the_window_opened_shows_in_the_panel_after_the_menu_opened() {
    let tree = tree("welcome_rescan");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();
    let pes17 = |nodes: &[accesskit::Node]| {
        nodes
            .iter()
            .any(|node| node.role() == Role::Button && node.label() == Some("PES 2017 Ctrl+7"))
    };
    idle_frames(&mut window, &mut view, 3);
    assert!(
        !pes17(&draw(&mut window, &mut view)),
        "premise: no PES 17 save was found"
    );

    // The File menu opened and closed before the save lands: its list
    // is the stale one a kept discovery would serve again.
    open_menu(&mut window, &mut view, "File", "Open\u{2026}");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    assert!(!menu_shown(&window, &nodes), "the File menu is closed");

    // A PES 17 save lands under Documents after the menu's first scan.
    let layout = save_layout(PesVersion::Pes17);
    let folder = tree
        .documents
        .join("KONAMI")
        .join(layout.game_folder)
        .join("save");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join(layout.file_name),
        fixture_save(PesVersion::Pes17),
    )
    .unwrap();

    // Opened again, the menu rescans: the save shows in its list, and
    // the panel's takes it.
    let nodes = open_menu(&mut window, &mut view, "File", "Open\u{2026}");
    assert!(pes17(&nodes), "the reopened menu lists the save");
    window.frame(
        vec![key(egui::Key::Escape, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    let nodes = draw(&mut window, &mut view);
    assert!(!menu_shown(&window, &nodes), "the File menu is closed");
    assert!(
        pes17(&nodes),
        "the panel lists the save found by the File menu's rescan"
    );
}

// SE-GUI-105
#[test]
fn close_save_drops_the_save_for_the_welcome_panel_and_exit_requests_the_close() {
    let tree = tree("close_save");
    let mut view = SaveView::new();
    view.open(&tree.pes21_old);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();

    // Close save with unsaved changes asks first, as an open does.
    choose(&mut window, &mut view, "File", "Close save");
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Unsaved changes"),
        "the unsaved-changes prompt is up"
    );
    assert!(view.session.is_some(), "nothing closed yet");

    // Discard closes the save: the welcome panel shows, and Undo has
    // nothing.
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Discard"),
        &mut view,
    );
    // The panel's frame is an `egui::Area`, which lays out invisible on
    // its first frame: the second frame shows it.
    idle_frames(&mut window, &mut view, 2);
    let nodes = draw(&mut window, &mut view);
    assert!(view.session.is_none(), "Discard closed the save");
    assert!(
        label_value(&nodes, "Save editor"),
        "the welcome panel shows"
    );
    let nodes = open_menu(&mut window, &mut view, "Edit", "Undo");
    assert!(
        node_labeled(&nodes, Role::Button, "Undo Ctrl+Z").is_disabled(),
        "nothing to undo"
    );

    // A clean save: Ctrl+W closes at once, no prompt, and a Live AATF
    // check in flight on the closed save lands nowhere.
    view.open(&tree.pes21_old);
    let key = crate::view::aatf_check::CheckKey {
        revision: 0,
        ids: vec![701],
        ruleset: "test".to_owned(),
        suggestions: false,
    };
    let (pending, replies) = crate::view::worker::Pending::<
        crate::view::aatf_check::CheckKey,
        Vec<crate::session::rules::TeamCheck>,
    >::held(key.clone());
    view.live.check.pending = Some(pending);
    view.live.check.result = Some((key.clone(), Vec::new()));
    press_ctrl(&mut window, &mut view, egui::Key::W);
    assert!(view.session.is_none(), "Ctrl+W closed the save at once");
    assert!(view.prompt.is_none(), "no prompt on a clean save");
    assert!(
        replies.send((key, Vec::new())).is_err(),
        "the closed save's check lands nowhere"
    );
    let nodes = draw(&mut window, &mut view);
    assert!(
        label_value(&nodes, "Save editor"),
        "the welcome panel shows"
    );
    assert!(
        view.live.check.result.is_none(),
        "the held result went with the save"
    );
    assert!(view.selected.is_none(), "the selection went with the save");

    // Ctrl+W while the Name field holds the keyboard closes nothing.
    view.open(&tree.pes21_old);
    let nodes = draw(&mut window, &mut view);
    click(&mut window, name_field(&nodes), &mut view);
    draw(&mut window, &mut view);
    assert!(
        window.ctx.egui_wants_keyboard_input(),
        "premise: the Name field holds the keyboard"
    );
    press_ctrl(&mut window, &mut view, egui::Key::W);
    assert!(
        view.session.is_some(),
        "Ctrl+W while typing keeps the save open"
    );

    // Exit asks the shell to close the window, as its close button does.
    choose(&mut window, &mut view, "File", "Exit");
    assert!(view.request_close, "the close request for the shell");
}
