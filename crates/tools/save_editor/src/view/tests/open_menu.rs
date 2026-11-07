//! The Open menu's and Save as's tests (SE-GUI-26): the discovered saves,
//! the recent files and the picker in the menu, discovery run once per
//! opening, the picker's starting folder, an entry under unsaved changes,
//! the version shortcuts and Ctrl+O, and Save as's write, `.bak` and
//! running-game question.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::SystemTime;

use pes_savefile::discovery::{discover_savefiles_in, save_layout};
use studio_core::StudioTool;

use super::prompt::modal_button;
use super::*;
use crate::test_support::{journal_lines, zip_entry};

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
    /// PES 19 under the account `292733975847239680`.
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
    let pes21_new = save(PesVersion::Pes21, "76561199320748656", now);
    let pes19 = save(PesVersion::Pes19, "292733975847239680", hour_ago);
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

/// The Open menu's picker stubbed to cancel, recording the folder each
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

/// The open menu's entries top to bottom: the buttons in the column of
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

/// Whether the frame's nodes hold the Open menu's popup.
fn menu_shown(nodes: &[accesskit::Node]) -> bool {
    nodes.iter().any(|node| {
        node.role() == Role::Button
            && node
                .label()
                .is_some_and(|label| label.starts_with("Choose file"))
    })
}

// SE-GUI-26
#[test]
fn the_open_menu_lists_the_discovered_saves_the_recent_file_and_the_picker() {
    let tree = tree("open_menu_entries");
    let mut view = SaveView::new();
    view.open(&tree.elsewhere);
    let calls = discover_counted(&mut view, &tree.documents);
    view.pes_version = PesVersion::Pes21;
    let mut window = Headless::new();

    let nodes = open_menu(&mut window, &mut view, "Open", PES19_ENTRY);
    // The scratch folder's name, then the save's folder and file name.
    let recent = "open_menu_entries\\elsewhere\\EDIT00000000";
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
            recent,
            "Choose file\u{2026} Ctrl+O",
        ],
        "the discovered saves in version order, each named by its version, the newest PES 21 \
         first and the first of each version with its shortcut, then the recent file, then the \
         picker"
    );

    // Each entry's tooltip is its own full path, found by its place in
    // the menu (the two PES 21 entries read alike); `Choose file…`'s says
    // where the picker opens.
    let tips = [
        tree.pes19.display().to_string(),
        tree.pes21_new.display().to_string(),
        tree.pes21_old.display().to_string(),
        tree.elsewhere.display().to_string(),
        "Opens in the selected version's save folder when it exists".to_owned(),
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
    assert!(menu_shown(&nodes), "premise: the menu is still open");
    assert_eq!(calls.get(), PesVersion::ALL.len(), "one call per version");

    // A discovered entry opens its save.
    click(&mut window, menu_item(&nodes, PES19_ENTRY), &mut view);
    let session = view.session.as_ref().expect("a save is open");
    assert_eq!(session.path(), tree.pes19.as_path());
    assert_eq!(session.version(), PesVersion::Pes19);
}

// SE-GUI-26
#[test]
fn choose_file_starts_in_the_selected_versions_save_folder() {
    let tree = tree("open_menu_choose_file");
    let mut view = SaveView::new();
    discover_counted(&mut view, &tree.documents);
    let starts = picker_starts(&mut view);
    let mut window = Headless::new();

    view.pes_version = PesVersion::Pes21;
    let nodes = open_menu(&mut window, &mut view, "Open", "Choose file");
    click(&mut window, menu_item(&nodes, "Choose file"), &mut view);
    // PES 18 has no save under the test's Documents.
    view.pes_version = PesVersion::Pes18;
    let nodes = open_menu(&mut window, &mut view, "Open", "Choose file");
    click(&mut window, menu_item(&nodes, "Choose file"), &mut view);

    assert_eq!(
        *starts.borrow(),
        [tree.pes21_new.parent().map(Path::to_path_buf), None],
        "the newest PES 21 save's folder, then no folder for PES 18"
    );
}

// SE-GUI-05, SE-GUI-26: an Open menu entry under unsaved changes asks
// first; Discard opens the entry's save and leaves the open file as it was.
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

    let nodes = open_menu(&mut window, &mut view, "Open", PES19_ENTRY);
    click(&mut window, menu_item(&nodes, PES19_ENTRY), &mut view);
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
    assert!(!menu_shown(&nodes), "premise: the menu is closed");

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

    // No PES 17 save under the test's Documents.
    view.pes_version = PesVersion::Pes21;
    press_ctrl(&mut window, &mut view, egui::Key::Num7);
    assert_eq!(
        *starts.borrow(),
        [tree.pes21_new.parent().map(Path::to_path_buf)],
        "Ctrl+7 opened the picker once, in the newest PES 21 save's folder"
    );
    // No PES 18 save either: Ctrl+O's picker starts in no folder.
    view.pes_version = PesVersion::Pes18;
    press_ctrl(&mut window, &mut view, egui::Key::O);
    assert_eq!(
        *starts.borrow(),
        [tree.pes21_new.parent().map(Path::to_path_buf), None],
        "Ctrl+O opened the picker in no folder for PES 18"
    );
    assert!(view.session.is_none(), "a cancelled picker opens nothing");
}

// SE-GUI-05, SE-GUI-26: a shortcut under unsaved changes asks first, as an
// Open menu entry does; Discard opens the version's save.
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
/// as's picker returns.
fn edited_view(path: &Path, target: &Path) -> SaveView {
    let mut view = SaveView::new();
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

// SE-GUI-26
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
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut tool.view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Save as"),
        &mut tool.view,
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

    // A fresh path: written, and no `.bak` beside it.
    let fresh = dir.join("fresh").join("EDIT00000000");
    std::fs::create_dir_all(fresh.parent().unwrap()).unwrap();
    let mut view = edited_view(&path, &fresh);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Save as"),
        &mut view,
    );
    assert!(fresh.is_file(), "the save is written");
    assert!(!bak_of(&fresh).exists(), "no .bak for a fresh path");
}

// SE-GUI-21, SE-GUI-26: Save as with the save's own PES running asks the
// running-game question first; its Save writes to the Save as target.
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

    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &running_own));
    let save_as = centre(node_labeled(&nodes, Role::Button, "Save as"));
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
    let refused = lines
        .iter()
        .find(|line| line.starts_with("could not open "))
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
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("could not open "))
            .count(),
        2,
        "{lines:?}"
    );
    assert!(
        zip::ZipArchive::new(std::fs::File::open(&bundle).unwrap())
            .unwrap()
            .by_name("inputs/2-no-such-file")
            .is_err(),
        "a file that cannot be read keeps nothing"
    );
}
