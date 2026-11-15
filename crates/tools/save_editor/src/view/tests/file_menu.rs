//! The File menu's and Save as's tests (SE-GUI-26): the discovered saves,
//! the recent files and the picker in the menu, then Save and Save as,
//! discovery run once per opening, the picker's starting folder, an open
//! under unsaved changes, the version shortcuts and Ctrl+O, and Save as's
//! write, `.bak` and running-game question.

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
    /// A PES 21 save outside `Documents`.
    elsewhere: PathBuf,
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
    std::fs::write(&elsewhere, fixture_save(PesVersion::Pes21)).unwrap();
    Tree {
        documents,
        pes21_old,
        pes21_new,
        pes19,
        elsewhere,
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
                    .is_some_and(|label| label.starts_with("Choose file"))
        })
}

// SE-GUI-26
#[test]
fn the_file_menu_lists_the_discovered_saves_the_recent_file_the_picker_then_save() {
    let tree = tree("open_menu_entries");
    let mut view = SaveView::new();
    view.open(&tree.pes21_old);
    let calls = discover_counted(&mut view, &tree.documents);
    let mut window = Headless::new();

    let nodes = open_menu(&mut window, &mut view, "File", PES19_ENTRY);
    // The recent entry is one of the discovered saves, its label the
    // path's last three parts: the account folder, `save`, the file.
    let recent = format!(
        "292733975847239680{0}save{0}EDIT00000000",
        std::path::MAIN_SEPARATOR
    );
    let labels: Vec<&str> = entries(&nodes, PES19_ENTRY)
        .iter()
        .filter_map(|node| node.label())
        .collect();
    // A button's AccessKit label is its text, then its shortcut text.
    assert_eq!(
        labels,
        [
            "PES 2019 Ctrl+9",
            "PES 2021 Ctrl+1",
            "PES 2021",
            recent.as_str(),
            "Choose file\u{2026} Ctrl+O",
            "Save Ctrl+S",
            "Save as\u{2026}",
        ],
        "the discovered saves in version order, each named by its version, the newest PES 21 \
         first and the first of each version with its shortcut, then the recent file, then the \
         picker, then Save with its shortcut and Save as"
    );
    for entry in ["Save Ctrl+S", "Save as\u{2026}"] {
        assert!(
            !node_labeled(&nodes, Role::Button, entry).is_disabled(),
            "{entry} is enabled with a save open"
        );
    }
    let (entries_at, lines) = menu_entries(&mut window, &mut view, "File", PES19_ENTRY);
    assert_eq!(lines.len(), 3, "three separators: {lines:?}");
    assert!(
        separated(&entries_at, &lines, 2, 3) && separated(&entries_at, &lines, 3, 4),
        "the saves, the recent file and the picker apart"
    );
    assert!(
        separated(&entries_at, &lines, 4, 5),
        "a separator between Choose file… and Save"
    );
    assert!(
        !separated(&entries_at, &lines, 5, 6),
        "Save and Save as together"
    );

    // Each entry's tooltip is its own full path, found by its place in
    // the menu (the two PES 21 entries read alike); `Choose file…`'s says
    // where the picker opens.
    let tips = [
        tree.pes19.display().to_string(),
        tree.pes21_new.display().to_string(),
        tree.pes21_old.display().to_string(),
        tree.pes21_old.display().to_string(),
        "Pick a save file, starting in the folder of the save opened last (or else of the newest save found)".to_owned(),
    ];
    for (place, tip) in tips.iter().enumerate() {
        let nodes = draw(&mut window, &mut view);
        let at = centre(entries(&nodes, PES19_ENTRY)[place]);
        assert_eq!(
            tooltip_containing(&mut window, &mut view, at, tip),
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
}

// A separator stands between the discovered saves and the recents when
// both lists exist, before Choose file… when either does, and always
// before Save; never elsewhere (SE-GUI-26's grouped entries).
#[test]
fn the_file_menus_separators_stand_only_between_its_groups() {
    /// The File menu: `documents` discovered (None stubs empty) and
    /// `recent` opened first if given; `first` is the menu's top entry.
    /// Its entries and separator ys.
    fn case(
        documents: Option<&Path>,
        recent: Option<PathBuf>,
        first: &str,
    ) -> (Vec<(String, f64, f64)>, Vec<f64>) {
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
        menu_entries(&mut window, &mut view, "File", first)
    }

    // Neither list: Choose file… alone, then the separator before Save.
    let (entries, lines) = case(None, None, "Choose file");
    assert_eq!(entries.len(), 3, "Choose file…, Save, Save as: {entries:?}");
    assert_eq!(lines.len(), 1, "one separator: {lines:?}");
    assert!(separated(&entries, &lines, 0, 1), "before Save");

    // Discovered only: the separator before Choose file… alone, then the
    // one before Save.
    let tree = tree("open_menu_sep_discovered");
    let (entries, lines) = case(Some(&tree.documents), None, PES19_ENTRY);
    assert_eq!(
        entries.len(),
        6,
        "three saves, the picker, Save, Save as: {entries:?}"
    );
    assert_eq!(lines.len(), 2, "two separators: {lines:?}");
    assert!(
        separated(&entries, &lines, 2, 3),
        "between the saves and Choose file…"
    );
    assert!(separated(&entries, &lines, 3, 4), "before Save");

    // Recents only: the separator before Choose file… alone, then the one
    // before Save.
    let (entries, lines) = case(
        None,
        Some(tree.elsewhere.clone()),
        &format!(
            "open_menu_sep_discovered{0}elsewhere{0}EDIT00000000",
            std::path::MAIN_SEPARATOR
        ),
    );
    assert_eq!(
        entries.len(),
        4,
        "the recent, the picker, Save, Save as: {entries:?}"
    );
    assert_eq!(lines.len(), 2, "two separators: {lines:?}");
    assert!(separated(&entries, &lines, 0, 1), "before Choose file…");
    assert!(separated(&entries, &lines, 1, 2), "before Save");

    // Both: a separator between the lists, one before Choose file… and
    // one before Save, none between the saves.
    let (entries, lines) = case(
        Some(&tree.documents),
        Some(tree.elsewhere.clone()),
        PES19_ENTRY,
    );
    assert_eq!(
        entries.len(),
        7,
        "the saves, the recent, the picker, Save, Save as: {entries:?}"
    );
    assert_eq!(lines.len(), 3, "three separators: {lines:?}");
    assert!(
        !separated(&entries, &lines, 0, 1) && !separated(&entries, &lines, 1, 2),
        "none between the discovered saves"
    );
    assert!(
        separated(&entries, &lines, 2, 3),
        "between the saves and the recents"
    );
    assert!(separated(&entries, &lines, 3, 4), "before Choose file…");
    assert!(separated(&entries, &lines, 4, 5), "before Save");
    assert!(
        !separated(&entries, &lines, 5, 6),
        "none between Save and Save as"
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
    choose(&mut window, &mut view, "File", "Choose file");
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
    choose(&mut window, &mut view, "File", "Choose file");
    std::fs::remove_dir_all(&backups).unwrap();
    choose(&mut window, &mut view, "File", "Choose file");
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
    choose(&mut window, &mut view, "File", "Choose file");
    assert_eq!(*starts.borrow(), [None], "nothing discovered: no folder");
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

/// The Save as picker stubbed to return `target`, recording the path it
/// is handed (the open file's, as `save_as` starts the picker there).
fn save_as_stub(view: &mut SaveView, target: &Path) -> Rc<RefCell<Vec<PathBuf>>> {
    let handed = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&handed);
    let target = target.to_path_buf();
    view.save_as_dialog = Box::new(move |current| {
        record.borrow_mut().push(current.to_path_buf());
        Some(target.clone())
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
    let handed = save_as_stub(&mut tool.view, &target);
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
    assert_eq!(tool.view.recents.first(), Some(&target), "the recents' top");
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

// A save's Ctrl shortcut is Ctrl alone: with Shift held it is not Save.
#[test]
fn ctrl_shift_s_writes_nothing_and_keeps_the_edit_pending() {
    let path = fixture(PesVersion::Pes21, "ctrl_shift_s");
    let before = std::fs::read(&path).unwrap();
    let mut view = SaveView::new();
    view.open(&path);
    let mut edited = player_entry(&view, 70101);
    edited.basic.height = 170;
    view.session.as_mut().unwrap().edit_player(1, edited);
    assert!(view.is_dirty(), "premise: unsaved changes");
    let mut window = Headless::new();

    // Ctrl+Shift+S is not Save: the file's bytes and the dirty mark
    // stand as they were.
    let shift_s = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    window.frame(vec![key(egui::Key::S, shift_s, true)], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "Ctrl+Shift+S wrote nothing"
    );
    assert!(view.is_dirty(), "the edit is still pending");
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
// panel, titled "Save editor" — the File menu's saves and picker again,
// no modal up. A file opened this run is one of the File menu's
// recents, which the panel does not list.
// SE-GUI-90
#[test]
fn the_welcome_panel_lists_the_opens_and_a_click_opens_its_save() {
    let tree = tree("welcome_panel");
    let mut view = SaveView::new();
    view.open(&tree.pes21_old);
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
        label_value(&nodes, "Saves in Documents"),
        "the saves' heading"
    );
    // A file was opened this run, so the File menu's recents hold one:
    // the panel draws no Recent heading and no entry for it (the tail of
    // the opened save's path, as the File menu's entry reads).
    let recent = format!(
        "292733975847239680{0}save{0}EDIT00000000",
        std::path::MAIN_SEPARATOR
    );
    assert!(!label_value(&nodes, "Recent"), "no Recent heading");
    assert!(
        !nodes
            .iter()
            .any(|node| { node.role() == Role::Button && node.label() == Some(recent.as_str()) }),
        "no entry for the recent file"
    );
    let labels: Vec<&str> = entries(&nodes, PES19_ENTRY)
        .iter()
        .filter_map(|node| node.label())
        .collect();
    assert_eq!(
        labels,
        [
            "PES 2019 Ctrl+9",
            "PES 2021 Ctrl+1",
            "PES 2021",
            "Choose file\u{2026} Ctrl+O",
        ],
        "the saves by version, the first of each with its shortcut, then the \
         picker — the File menu's opens without the recents"
    );
    assert!(
        window
            .ctx
            .memory(|memory| memory.top_modal_layer())
            .is_none(),
        "the panel registers no modal layer"
    );
    // The rows fill the frame, a dialog's 360 points less its 12-point
    // margins and 1-point stroke, as the draft's do; the picker stays a
    // compact button.
    let width = |node: &accesskit::Node| {
        let bounds = node.bounds().expect("an entry's bounds");
        bounds.x1 - bounds.x0
    };
    let rows = entries(&nodes, PES19_ENTRY);
    for row in &rows[..3] {
        assert!(
            (width(row) - 334.0).abs() <= 0.5,
            "{:?} is {:.1} wide",
            row.label(),
            width(row)
        );
    }
    assert!(width(rows[3]) < 300.0, "Choose file… stays compact");

    // Each save's entry carries its path as the tooltip, as the File
    // menu's entries do.
    for (row, path) in rows[..3]
        .iter()
        .zip([&tree.pes19, &tree.pes21_new, &tree.pes21_old])
    {
        let tip = path.display().to_string();
        assert_eq!(
            tooltip_containing(&mut window, &mut view, centre(row), &tip),
            Some(tip),
            "{:?}'s tooltip is its path",
            row.label()
        );
    }

    // The panel's Choose file… does the menu's: the picker in the
    // folder the last save was opened from while it exists.
    let nodes = draw(&mut window, &mut view);
    click(&mut window, menu_item(&nodes, "Choose file"), &mut view);
    assert_eq!(
        *starts.borrow(),
        [tree.pes21_old.parent().map(Path::to_path_buf)],
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
}

// SE-GUI-90's last clause: nothing discovered and none opened reads the
// weak line over Choose file….
// SE-GUI-90
#[test]
fn the_welcome_panel_with_nothing_found_reads_so_over_the_picker() {
    let mut view = SaveView::new();
    view.discover = Box::new(|_| Vec::new());
    let mut window = Headless::new();

    idle_frames(&mut window, &mut view, 3);
    let nodes = draw(&mut window, &mut view);
    let line = label_bounds(&nodes, "No PES saves were found in Documents.");
    let picker = menu_item(&nodes, "Choose file")
        .bounds()
        .expect("Choose file…'s bounds");
    assert!(line.y0 < picker.y0, "the weak line above Choose file…");
    assert!(
        !label_value(&nodes, "Recent"),
        "no Recent heading with no recent file"
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
    open_menu(&mut window, &mut view, "File", "Choose file");
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
    let nodes = open_menu(&mut window, &mut view, "File", "Choose file");
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
