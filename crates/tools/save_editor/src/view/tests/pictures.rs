//! The formation picture dialog's tests (SE-GUI-86): the Team menu's entry
//! end to end through `Tool::view`, so the program's folder and the three
//! remembered settings are the tool context's.

use studio_core::{ActionEffect, Notice, NoticeAction, NoticeKind, ShellRequest, StudioTool};

use super::*;
use crate::view::pictures::picture_file_name;

/// The PES 2021 fixture's /f/.
const F: u32 = 718;

/// The Team menu's entry.
const ENTRY: &str = "Save a picture of the formations\u{2026}";

/// The grain box's label.
const FLAT_BOX: &str = "Without the pitch's grain (smaller file)";

/// The settings box's label.
const SETTINGS_BOX: &str = "With each preset's settings beside its row";

/// The Savefile menu's entry.
const RANGE_ENTRY: &str = "Save formation pictures for a range of teams\u{2026}";

/// The range dialog's title.
const RANGE_TITLE: &str = "Save formation pictures for a range of teams";

/// The range the run tests write: three of the PES 2021 fixture's cup
/// teams in a row.
const FIRST: u32 = 716;
const MIDDLE: u32 = 717;
const LAST: u32 = 718;

/// The size of /f/'s picture: one column of three pitches under the title.
const ONE_COLUMN: (u32, u32) = (32 + 96 + 320, 32 + 30 + 3 * 384 + 2 * 12);

/// The PES 2021 fixture open in a tool on a player row of /f/ (the menu
/// acts on his team), its settings the defaults without
/// `formation_pictures_folder`, in a context whose `exe_dir` is under the
/// test's scratch folder.
struct Scene {
    tool: crate::Tool,
    ctx: studio_core::ToolContext,
    requests: crossbeam_channel::Receiver<ShellRequest>,
    window: Headless,
    /// The test's scratch folder.
    dir: PathBuf,
}

impl Scene {
    fn new(test: &str) -> Scene {
        let path = fixture(PesVersion::Pes21, test);
        let dir = path.parent().expect("the scratch dir").to_path_buf();
        let mut tool = crate::Tool::new();
        let mut table = tool.default_settings();
        table.remove("formation_pictures_folder");
        let mut settings = studio_core::Settings::default();
        settings.set_tool("save-editor", table);
        let (ctx, _events, requests) = tool_context(&dir, settings);
        tool.view.open(&path);
        select_f(&mut tool.view);
        let mut scene = Scene {
            tool,
            ctx,
            requests,
            window: Headless::new(),
            dir,
        };
        scene.draw();
        scene
    }

    /// A new tool on the same settings, the save open again on /f/: the
    /// editor restarted.
    fn restart(&mut self) {
        let path = self.dir.join("EDIT00000000");
        self.tool = crate::Tool::new();
        self.tool.view.open(&path);
        select_f(&mut self.tool.view);
        self.draw();
    }

    /// One frame of the tool's window: its nodes.
    fn draw(&mut self) -> Vec<accesskit::Node> {
        let Scene {
            tool, ctx, window, ..
        } = self;
        window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
    }

    /// A click on `node`, then the next frame's nodes.
    fn click(&mut self, node: &accesskit::Node) -> Vec<accesskit::Node> {
        let Scene {
            tool, ctx, window, ..
        } = self;
        press_release(window, centre(node), |ui| tool.view(ui, ctx));
        self.draw()
    }

    /// A click on the button labelled `label` in the frame drawn now.
    fn press(&mut self, label: &str) -> Vec<accesskit::Node> {
        let nodes = self.draw();
        self.click(node_labeled(&nodes, Role::Button, label))
    }

    /// The dialog opened from the Team menu (`open_from`).
    fn open_dialog(&mut self) -> Vec<accesskit::Node> {
        self.open_from("Team", ENTRY, "Save a picture of the formations")
    }

    /// The range dialog opened from the Savefile menu (`open_from`).
    fn open_range(&mut self) -> Vec<accesskit::Node> {
        self.open_from("Savefile", RANGE_ENTRY, RANGE_TITLE)
    }

    /// The dialog titled `title` opened from `menu`'s `entry`: the nodes of
    /// the first frame it stands where the frame before put it. Its grid
    /// takes egui a few frames to size, and the dialog moves while it does,
    /// so a place read earlier is not where a click lands.
    fn open_from(&mut self, menu: &str, entry: &str, title: &str) -> Vec<accesskit::Node> {
        choose(&mut self.window, &mut self.tool.view, menu, entry);
        let place =
            |nodes: &[accesskit::Node]| node_labeled(nodes, Role::CheckBox, FLAT_BOX).bounds();
        let mut last = place(&self.draw());
        for _ in 0..10 {
            let nodes = self.draw();
            if place(&nodes) == last {
                assert!(label_value(&nodes, title), "the entry opens the dialog");
                return nodes;
            }
            last = place(&nodes);
        }
        panic!("the dialog never stood still");
    }

    /// One frame on `events`: its nodes with their AccessKit ids.
    fn draw_with(&mut self, events: Vec<egui::Event>) -> Vec<(accesskit::NodeId, accesskit::Node)> {
        let Scene {
            tool, ctx, window, ..
        } = self;
        window.frame_ids(events, |ui| tool.view(ui, ctx)).1
    }

    /// One frame: the range dialog's own nodes, the view's behind its
    /// backdrop left out (the player list names the same teams).
    fn range_nodes(&mut self) -> Vec<accesskit::Node> {
        let pairs = self.draw_with(Vec::new());
        super::prompt::dialog_nodes(&pairs, RANGE_TITLE)
            .into_iter()
            .cloned()
            .collect()
    }

    /// The range dialog's team combo `nth` from the left clicked open: the
    /// next frame's nodes, and the combo's left and right edges and bottom
    /// as that frame draws it, which the popup's items stand under.
    fn open_combo(
        &mut self,
        nth: usize,
    ) -> (Vec<(accesskit::NodeId, accesskit::Node)>, (f64, f64, f64)) {
        let combo = combos(&self.range_nodes())[nth].clone();
        self.click(&combo);
        let pairs = self.draw_with(Vec::new());
        let nodes = super::prompt::dialog_nodes(&pairs, RANGE_TITLE);
        let bounds = combos(nodes)[nth].bounds().expect("drawn bounds");
        (pairs, (bounds.x0, bounds.x1, bounds.y1))
    }

    /// A click on the range dialog's title, which closes a combo's popup.
    fn click_title(&mut self) {
        let nodes = self.range_nodes();
        let title = nodes
            .iter()
            .find(|node| node.value() == Some(RANGE_TITLE))
            .expect("the title")
            .clone();
        self.click(&title);
    }

    /// The labels of the range dialog's team combo `nth`'s popup items, top
    /// to bottom; the popup is closed again.
    fn combo_items(&mut self, nth: usize) -> Vec<String> {
        let (pairs, (left, right, below)) = self.open_combo(nth);
        let mut items: Vec<(accesskit::Rect, String)> = pairs
            .iter()
            .filter(|(_, node)| node.role() == Role::Button)
            .filter_map(|(_, node)| {
                let (bounds, label) = (node.bounds()?, node.label()?);
                ((left..right).contains(&bounds.x0) && bounds.y0 >= below)
                    .then(|| (bounds, label.to_owned()))
            })
            .collect();
        items.sort_by(|a, b| a.0.y0.partial_cmp(&b.0.y0).expect("drawn bounds"));
        self.click_title();
        // The popup's items share its edges; a button of the view behind
        // it, under the combo too, does not.
        let first = items.first().expect("the popup's first item").0;
        items
            .into_iter()
            .filter(|(bounds, _)| bounds.x0 == first.x0 && bounds.x1 == first.x1)
            .map(|(_, label)| label)
            .collect()
    }

    /// `label` picked in the range dialog's team combo `nth`, through the
    /// popup item's AccessKit action (a popup item can sit where a pointer
    /// click lands on nothing); the popup is closed again. The nodes of
    /// the pick's own frame.
    fn pick(&mut self, nth: usize, label: &str) -> Vec<accesskit::Node> {
        let (pairs, (left, right, _)) = self.open_combo(nth);
        let (item, _) = pairs
            .iter()
            .find(|(_, node)| {
                node.role() == Role::Button
                    && node.label() == Some(label)
                    && node
                        .bounds()
                        .is_some_and(|bounds| (left..right).contains(&bounds.x0))
            })
            .unwrap_or_else(|| panic!("no team {label:?} in the combo's popup"));
        let picked = self.draw_with(vec![action_request(*item)]);
        let nodes = super::prompt::dialog_nodes(&picked, RANGE_TITLE)
            .into_iter()
            .cloned()
            .collect();
        self.click_title();
        nodes
    }

    /// Team `id` as the range's combos name it: its id, then its name
    /// without colour codes.
    fn team_label(&self, id: u32) -> String {
        format!(
            "{id} {}",
            display_name(&team_entry(&self.tool.view, id).name)
        )
    }

    /// Team `id`'s picture's file name.
    fn file_name(&self, id: u32) -> String {
        picture_file_name(id, &team_entry(&self.tool.view, id).name)
    }

    /// The save's cup teams' ids, in id order.
    fn cup_ids(&self) -> Vec<u32> {
        let file = self.tool.view.session.as_ref().expect("the save").file();
        let mut ids: Vec<u32> = crate::session::cup::cup_teams(file)
            .map(|team| team.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// The range dialog's width, its modal's area as the last frame drew
    /// it (its `Role::Dialog` node carries no bounds).
    fn dialog_width(&self) -> f32 {
        self.window
            .ctx
            .memory(|memory| memory.area_rect(egui::Id::unique("save_editor_pictures_range")))
            .expect("the dialog's area")
            .width()
    }

    /// The range dialog open on `FIRST` to `LAST`, its folder `folder`
    /// picked with Browse, then Save pictures pressed, and no frame after
    /// the release's: the next frame is the run's first.
    fn start_run(&mut self, folder: &Path) {
        self.open_range();
        let (first, last) = (self.team_label(FIRST), self.team_label(LAST));
        self.pick(0, &first);
        self.pick(1, &last);
        let nodes = self.browse(folder);
        let save = centre(node_labeled(&nodes, Role::Button, "Save pictures"));
        let Scene {
            tool, ctx, window, ..
        } = self;
        press_release(window, save, |ui| tool.view(ui, ctx));
    }

    /// `folder` picked with Browse.
    fn browse(&mut self, folder: &Path) -> Vec<accesskit::Node> {
        let folder = folder.to_path_buf();
        self.tool.view.pick_folder_dialog = Box::new(move || Some(folder.clone()));
        self.press("Browse\u{2026}")
    }

    /// The tool's settings table as the context holds it.
    fn table(&self) -> toml::Table {
        self.ctx.tool_settings("save-editor")
    }

    /// The notices the tool sent since the last call.
    fn notices(&self) -> Vec<Notice> {
        self.requests
            .try_iter()
            .filter_map(|request| match request {
                ShellRequest::Notify(notice) => Some(notice),
                _ => None,
            })
            .collect()
    }
}

/// A player row of /f/ selected.
fn select_f(view: &mut SaveView) {
    view.team = TeamChoice::Team(F);
    view.selected = Some(Selection::Player(F, 0));
}

/// Whether the dialog is up: its title among the frame's labels.
fn dialog_open(nodes: &[accesskit::Node]) -> bool {
    label_value(nodes, "Save a picture of the formations")
}

/// The text of the box right of the grid's label `label`.
fn field(nodes: &[accesskit::Node], label: &str) -> String {
    field_right_of_label(nodes, label, Role::TextInput)
        .value()
        .expect("a text box's value")
        .to_owned()
}

/// Whether the dialog's checkbox `label` is ticked.
fn ticked(nodes: &[accesskit::Node], label: &str) -> bool {
    node_labeled(nodes, Role::CheckBox, label).toggled() == Some(accesskit::Toggled::True)
}

/// The success notice for `f.png` written to `folder`.
fn written(folder: &Path) -> Notice {
    Notice {
        text: format!("f.png written to {}", folder.display()),
        kind: NoticeKind::Success,
        action: Some(NoticeAction {
            label: "Open folder".to_owned(),
            effect: ActionEffect::OpenFolder(folder.to_path_buf()),
        }),
    }
}

/// The button `label` of the question `question`: of the frame's buttons
/// so labelled (the dialog's own Cancel stands behind the question), the
/// nearest under it.
fn question_button<'a>(
    nodes: &'a [accesskit::Node],
    question: &str,
    label: &str,
) -> &'a accesskit::Node {
    let under = label_bounds(nodes, question).y1;
    nodes
        .iter()
        .filter(|node| node.role() == Role::Button && node.label() == Some(label))
        .filter_map(|node| Some((node, node.bounds()?.y0)))
        .filter(|(_, top)| *top >= under)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).expect("ordered bounds"))
        .map(|(node, _)| node)
        .unwrap_or_else(|| panic!("no {label:?} under {question:?}"))
}

/// `path`'s PNG size, decoded.
fn png_size(path: &Path) -> (u32, u32) {
    let image =
        image::load_from_memory_with_format(&std::fs::read(path).unwrap(), image::ImageFormat::Png)
            .expect("the file is a PNG");
    (image.width(), image.height())
}

/// The team combos among `nodes`, left to right.
fn combos<'a>(nodes: impl IntoIterator<Item = &'a accesskit::Node>) -> Vec<&'a accesskit::Node> {
    let mut combos: Vec<&accesskit::Node> = nodes
        .into_iter()
        .filter(|node| node.role() == Role::ComboBox)
        .collect();
    combos.sort_by(|a, b| {
        let x = |node: &&accesskit::Node| node.bounds().expect("drawn bounds").x0;
        x(a).partial_cmp(&x(b)).expect("ordered bounds")
    });
    combos
}

/// An AccessKit `Click` on node `id`, the event a click on it sends
/// without a pointer.
fn action_request(id: accesskit::NodeId) -> egui::Event {
    egui::Event::AccessKitActionRequest(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: id,
        data: None,
    })
}

/// The texts of `nodes` as a reader meets them: one row per line of the
/// dialog, top to bottom, each left to right. A label's or a box's text, a
/// combo's choice, a button's or checkbox's label; empty texts left out.
fn reading_rows(nodes: &[accesskit::Node]) -> Vec<Vec<String>> {
    let mut texts: Vec<(f64, f64, String)> = nodes
        .iter()
        .filter_map(|node| {
            let text = match node.role() {
                Role::Label | Role::ComboBox | Role::TextInput => node.value(),
                Role::Button | Role::CheckBox => node.label(),
                _ => None,
            }?;
            let bounds = node.bounds()?;
            (!text.is_empty()).then(|| ((bounds.y0 + bounds.y1) / 2.0, bounds.x0, text.to_owned()))
        })
        .collect();
    texts.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
    // A row's widgets are centred on one line, within a point or two of
    // each other; the next row is a whole line lower.
    let mut rows: Vec<(f64, Vec<(f64, String)>)> = Vec::new();
    for (middle, left, text) in texts {
        match rows.last_mut() {
            Some((row_middle, row)) if (middle - *row_middle).abs() <= 2.0 => {
                row.push((left, text))
            }
            _ => rows.push((middle, vec![(left, text)])),
        }
    }
    rows.into_iter()
        .map(|(_, mut row)| {
            row.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("drawn bounds"));
            row.into_iter().map(|(_, text)| text).collect()
        })
        .collect()
}

/// The text of the label among `nodes` that starts with `prefix`.
fn line_starting(nodes: &[accesskit::Node], prefix: &str) -> Option<String> {
    nodes
        .iter()
        .filter(|node| node.role() == Role::Label)
        .filter_map(|node| node.value())
        .find(|value| value.starts_with(prefix))
        .map(str::to_owned)
}

/// Asserts that `nodes` hold the label `sentence` and the label `folder`
/// under it, starting where it starts, as the range dialog's result does.
/// `reading_rows` cannot tell that from a folder beside the sentence,
/// which wraps, so its middle falls on a row of its own.
fn assert_folder_under(nodes: &[accesskit::Node], sentence: &str, folder: &Path) {
    let above = label_bounds(nodes, sentence);
    let under = label_bounds(nodes, &folder.display().to_string());
    assert!(
        under.y0 >= above.y1 && under.x0 == above.x0,
        "the folder, at {under:?}, on the line under {sentence:?}, at {above:?}"
    );
}

/// The id of the button labelled `label` among `pairs`.
fn button_id(pairs: &[(accesskit::NodeId, accesskit::Node)], label: &str) -> accesskit::NodeId {
    pairs
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.label() == Some(label))
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {label:?} button"))
}

/// Whether the frame draws a button labelled `label`.
fn has_button(nodes: &[accesskit::Node], label: &str) -> bool {
    nodes
        .iter()
        .any(|node| node.role() == Role::Button && node.label() == Some(label))
}

/// The names of the files in `folder`, sorted.
fn files_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort_unstable();
    names
}

/// The files `folder` holds now, none while it is not there yet.
fn files_so_far(folder: &Path) -> Vec<String> {
    if folder.exists() {
        files_in(folder)
    } else {
        Vec::new()
    }
}

/// The nodes of `pairs`, without their ids.
fn nodes_of(pairs: Vec<(accesskit::NodeId, accesskit::Node)>) -> Vec<accesskit::Node> {
    pairs.into_iter().map(|(_, node)| node).collect()
}

// SE-GUI-86
#[test]
fn the_formations_picture_dialog_writes_one_png() {
    let mut scene = Scene::new("picture_dialog_writes");
    assert!(
        !scene.table().contains_key("formation_pictures_folder"),
        "premise: no folder setting"
    );
    assert!(
        scene.tool.view.tactics.portraits,
        "premise: the formation bar's Portraits is ticked"
    );
    let folder = scene.ctx.paths().exe_dir.join("formations");
    assert!(!folder.exists(), "premise: the folder is not there yet");

    let nodes = scene.open_dialog();
    assert_eq!(field(&nodes, "Folder"), folder.display().to_string());
    assert_eq!(field(&nodes, "File name"), "f.png");
    let left = |label: &str| {
        field_right_of_label(&nodes, label, Role::TextInput)
            .bounds()
            .expect("bounds")
            .x0
    };
    assert_eq!(left("Folder"), left("File name"), "one grid");
    assert!(ticked(&nodes, "Portraits"), "Portraits as the bar's");
    assert!(!ticked(&nodes, FLAT_BOX), "the grain drawn by default");
    for (label, tip) in [
        (
            "Portraits",
            "Draw the players' portraits in the picture. Off, the formation's shape is easier \
             to see.",
        ),
        (
            FLAT_BOX,
            "Paint the pitch in flat bands, without its grain. The picture looks plainer and \
             its file is smaller, which helps when sharing many.",
        ),
    ] {
        let at = centre(node_labeled(&nodes, Role::CheckBox, label));
        let shown = tooltip_containing(&mut scene.window, &mut scene.tool.view, at, tip);
        assert_eq!(shown.as_deref(), Some(tip), "{label}'s hover");
    }

    // The settings box, unticked between the grid and Portraits: ticking
    // it leaves the file name as it is.
    let nodes = scene.draw();
    let settings = node_labeled(&nodes, Role::CheckBox, SETTINGS_BOX);
    assert!(!ticked(&nodes, SETTINGS_BOX), "the settings box unticked");
    let file_name = field_right_of_label(&nodes, "File name", Role::TextInput);
    let portraits = node_labeled(&nodes, Role::CheckBox, "Portraits");
    let (box_top, name_bottom, portraits_top) = (
        settings.bounds().expect("bounds").y0,
        file_name.bounds().expect("bounds").y1,
        portraits.bounds().expect("bounds").y0,
    );
    assert!(
        name_bottom < box_top && box_top < portraits_top,
        "under the grid, over Portraits: {name_bottom} < {box_top} < {portraits_top}"
    );
    let nodes = scene.click(settings);
    assert!(ticked(&nodes, SETTINGS_BOX));
    assert_eq!(
        field(&nodes, "File name"),
        "f.png",
        "the name as one without"
    );
    let nodes = scene.click(node_labeled(&nodes, Role::CheckBox, SETTINGS_BOX));
    assert!(!ticked(&nodes, SETTINGS_BOX));
    assert_eq!(field(&nodes, "File name"), "f.png");

    // Portraits unticked, Save: the picture written without them, the
    // bar's setting untouched.
    let nodes = scene.draw();
    scene.click(node_labeled(&nodes, Role::CheckBox, "Portraits"));
    let nodes = scene.press("Save");
    let picture = folder.join("f.png");
    assert_eq!(png_size(&picture), ONE_COLUMN);
    let without_portraits = std::fs::read(&picture).unwrap();
    assert!(!dialog_open(&nodes), "Save closes the dialog");
    assert_eq!(scene.notices(), [written(&folder)]);
    assert!(scene.tool.view.tactics.portraits, "the bar's Portraits");
    assert_eq!(scene.table()["portraits"].as_bool(), Some(true));

    // Opened again: Portraits as the bar's again. Save asks first; Cancel
    // leaves the file as it was (bytes Save would not write), Replace
    // writes it.
    let nodes = scene.open_dialog();
    assert!(ticked(&nodes, "Portraits"), "Portraits starts as the bar's");
    let before = b"not the picture".to_vec();
    std::fs::write(&picture, &before).unwrap();
    let nodes = scene.press("Save");
    assert!(label_value(&nodes, "Replace the picture?"), "the title");
    let question = format!("f.png is already in {}. Replace it?", folder.display());
    assert!(label_value(&nodes, &question), "the question");
    let nodes = scene.click(question_button(&nodes, &question, "Cancel"));
    assert_eq!(std::fs::read(&picture).unwrap(), before, "Cancel");
    assert!(
        !label_value(&nodes, &question),
        "Cancel answers the question"
    );
    assert!(dialog_open(&nodes), "and leaves the dialog up");
    assert!(scene.notices().is_empty(), "no notice");
    let nodes = scene.press("Save");
    let nodes = scene.click(question_button(&nodes, &question, "Replace"));
    assert_eq!(png_size(&picture), ONE_COLUMN);
    assert_ne!(
        std::fs::read(&picture).unwrap(),
        without_portraits,
        "this picture has the portraits"
    );
    assert!(!dialog_open(&nodes), "Replace closes the dialog");
    assert_eq!(scene.notices(), [written(&folder)]);

    // The bar's Portraits off: the dialog opens with it off.
    let mut table = scene.table();
    table.insert("portraits".to_owned(), toml::Value::Boolean(false));
    scene.ctx.set_tool_settings("save-editor", table);
    scene.draw();
    let nodes = scene.open_dialog();
    assert!(!ticked(&nodes, "Portraits"), "Portraits as the bar's");
}

// SE-GUI-86
// SE-GUI-88
#[test]
fn a_ticked_settings_box_writes_the_settings_picture() {
    let mut scene = Scene::new("picture_dialog_settings");
    let folder = scene.ctx.paths().exe_dir.join("formations");
    let nodes = scene.open_dialog();
    scene.click(node_labeled(&nodes, Role::CheckBox, SETTINGS_BOX));
    scene.press("Save");
    assert_eq!(files_in(&folder), ["f.png"], "named as one without");
    assert_eq!(
        png_size(&folder.join("f.png")),
        (ONE_COLUMN.0 + 12 + 240, ONE_COLUMN.1 + 30),
        "the picture with the settings column and the Automatic settings line"
    );

    // Remembered: the next dialog starts ticked, on the same name.
    let nodes = scene.open_dialog();
    assert!(ticked(&nodes, SETTINGS_BOX));
    assert_eq!(field(&nodes, "File name"), "f.png");
}

// SE-GUI-86
#[test]
fn the_grain_box_is_remembered_and_its_picture_is_smaller() {
    let mut scene = Scene::new("picture_dialog_grain");
    let (grained, flat) = (scene.dir.join("grained"), scene.dir.join("flat"));
    scene.open_dialog();
    scene.browse(&grained);
    scene.press("Save");

    scene.open_dialog();
    let nodes = scene.browse(&flat);
    assert!(!ticked(&nodes, FLAT_BOX), "premise: unticked");
    let nodes = scene.click(node_labeled(&nodes, Role::CheckBox, FLAT_BOX));
    assert!(ticked(&nodes, FLAT_BOX));
    assert_eq!(
        scene.table()["formation_pictures_flat"].as_bool(),
        Some(true),
        "the tick is written"
    );
    scene.press("Save");
    let size = |folder: &Path| std::fs::metadata(folder.join("f.png")).unwrap().len();
    assert!(
        size(&flat) < size(&grained),
        "the flat picture, {} bytes, under the grained one's {}",
        size(&flat),
        size(&grained)
    );

    scene.restart();
    let nodes = scene.open_dialog();
    assert!(ticked(&nodes, FLAT_BOX), "the restart reads it back");
}

// SE-GUI-86
// SE-GUI-89
#[test]
fn the_settings_box_is_remembered_and_shared_by_the_two_dialogs() {
    let mut scene = Scene::new("picture_settings_remembered");
    let nodes = scene.open_dialog();
    assert!(!ticked(&nodes, SETTINGS_BOX), "premise: unticked");
    let nodes = scene.click(node_labeled(&nodes, Role::CheckBox, SETTINGS_BOX));
    assert!(ticked(&nodes, SETTINGS_BOX));
    assert_eq!(
        scene
            .table()
            .get("formation_pictures_settings")
            .and_then(toml::Value::as_bool),
        Some(true),
        "the tick is written"
    );

    scene.restart();
    let nodes = scene.open_dialog();
    assert!(ticked(&nodes, SETTINGS_BOX), "the restart reads it back");
    assert_eq!(field(&nodes, "File name"), "f.png");
    scene.press("Cancel");
    scene.open_range();
    let nodes = scene.range_nodes();
    assert!(ticked(&nodes, SETTINGS_BOX), "the range dialog's box too");
}

#[test]
fn the_folder_a_picture_was_saved_to_is_the_next_dialog_s() {
    let mut scene = Scene::new("picture_dialog_folder");
    let picked = scene.dir.join("cup pictures");
    scene.open_dialog();
    scene.browse(&picked);
    assert!(
        !scene.table().contains_key("formation_pictures_folder"),
        "a pick alone writes nothing"
    );
    scene.press("Save");
    assert_eq!(
        scene
            .table()
            .get("formation_pictures_folder")
            .and_then(toml::Value::as_str),
        Some(picked.display().to_string().as_str()),
        "Save writes the folder"
    );
    scene.restart();
    let nodes = scene.open_dialog();
    assert_eq!(field(&nodes, "Folder"), picked.display().to_string());
}

// SE-GUI-89
#[test]
fn the_range_dialog_picks_its_teams_folder_and_boxes() {
    let mut scene = Scene::new("pictures_range_picking");
    let ids = scene.cup_ids();
    let first = ids[0];
    let labels: Vec<String> = ids.iter().map(|id| scene.team_label(*id)).collect();
    let (first_label, last_label) = (labels[0].clone(), labels[ids.len() - 1].clone());
    let first_name = team_entry(&scene.tool.view, first).name;
    let line = format!(
        "One picture per team, named after it ({}), overwriting files with the same names.",
        picture_file_name(first, &first_name)
    );
    let folder = scene.ctx.paths().exe_dir.join("formations");
    let count = format!("{} teams", ids.len());

    // The controls in their order, on the save's first and last cup team.
    scene.open_range();
    let nodes = scene.range_nodes();
    assert_eq!(
        reading_rows(&nodes),
        [
            vec![RANGE_TITLE.to_owned()],
            vec![
                "Teams".to_owned(),
                first_label.clone(),
                "to".to_owned(),
                last_label.clone(),
                count.clone(),
            ],
            vec![
                "Folder".to_owned(),
                folder.display().to_string(),
                "Browse\u{2026}".to_owned(),
            ],
            vec![SETTINGS_BOX.to_owned()],
            vec!["Portraits".to_owned()],
            vec![FLAT_BOX.to_owned()],
            vec![line.clone()],
            vec!["Save pictures".to_owned(), "Cancel".to_owned()],
        ]
    );
    assert!(!ticked(&nodes, SETTINGS_BOX), "the settings box unticked");
    assert!(ticked(&nodes, "Portraits"), "Portraits as the bar's");
    assert!(!ticked(&nodes, FLAT_BOX), "the grain drawn by default");

    // The settings box writes its setting, and the line names the same
    // file in both states, so the dialog keeps its width when it flips.
    let unticked = scene.dialog_width();
    scene.click(node_labeled(&nodes, Role::CheckBox, SETTINGS_BOX));
    let nodes = scene.range_nodes();
    assert!(ticked(&nodes, SETTINGS_BOX));
    assert_eq!(
        scene
            .table()
            .get("formation_pictures_settings")
            .and_then(toml::Value::as_bool),
        Some(true),
        "the tick is written"
    );
    assert!(label_value(&nodes, &line), "the same file named");
    assert_eq!(
        scene.dialog_width(),
        unticked,
        "the width holds when the box flips"
    );

    // The right combo lists the teams from the left one's on.
    assert_eq!(
        scene.combo_items(1),
        labels,
        "every cup team while the left is on the first"
    );
    scene.pick(0, &labels[2]);
    assert_eq!(
        scene.combo_items(1),
        labels[2..],
        "the teams from the left one's on"
    );
    let nodes = scene.range_nodes();
    assert!(label_value(&nodes, &format!("{} teams", ids.len() - 2)));

    // A left team picked past the right one moves it, on the pick's frame.
    scene.pick(1, &labels[3]);
    let picked = scene.pick(0, &labels[5]);
    assert_eq!(
        combos(&picked)[1].value(),
        Some(labels[5].as_str()),
        "the right end follows the left on the pick's frame"
    );
    assert!(label_value(&picked, "1 team"), "a range of one");

    // Cancel, then open again: the defaults, not the last picks.
    scene.press("Cancel");
    scene.open_range();
    let nodes = scene.range_nodes();
    let ends: Vec<Option<&str>> = combos(&nodes).iter().map(|combo| combo.value()).collect();
    assert_eq!(
        ends,
        [Some(first_label.as_str()), Some(last_label.as_str())],
        "the save's first and last cup team each time it opens"
    );
}

// SE-GUI-89
#[test]
fn the_range_count_ends_the_teams_row_where_browse_ends() {
    let mut scene = Scene::new("pictures_range_count");
    let count = format!("{} teams", scene.cup_ids().len());

    scene.open_range();
    let nodes = scene.range_nodes();
    let at = label_bounds(&nodes, &count);
    let last = combos(&nodes)[1].bounds().expect("drawn bounds");
    let middle = (at.y0 + at.y1) / 2.0;
    assert!(
        (last.y0..=last.y1).contains(&middle),
        "the count's middle, {middle}, on the combos' row, {} to {}",
        last.y0,
        last.y1
    );
    let browse = node_labeled(&nodes, Role::Button, "Browse\u{2026}")
        .bounds()
        .expect("drawn bounds");
    assert!(
        (at.x1 - browse.x1).abs() <= 0.5,
        "the count ends at {}, where Browse ends, {}",
        at.x1,
        browse.x1
    );
}

// SE-GUI-89
#[test]
fn a_range_run_writes_one_picture_per_team_and_reports_it() {
    let mut scene = Scene::new("pictures_range_run");
    let teams = [FIRST, MIDDLE, LAST];
    let ids = scene.cup_ids();
    let at = ids.iter().position(|id| *id == FIRST).expect("a cup team");
    assert_eq!(ids[at..at + 3], teams, "premise: three cup teams in a row");
    let folder = scene.dir.join("cup pictures");
    assert!(
        !scene.table().contains_key("formation_pictures_folder"),
        "premise: no folder setting"
    );

    scene.start_run(&folder);
    // A frame is on screen while the next one runs, so each draws what the
    // next one writes: the first writes nothing, the last two both read
    // `Writing 3 of 3`. Per frame: the team named, the bar, the pictures
    // in the folder once it has run.
    let written_by = |n: usize| -> Vec<String> {
        let mut names: Vec<String> = teams[..n].iter().map(|id| scene.file_name(*id)).collect();
        names.sort_unstable();
        names
    };
    let frames = [
        (1, FIRST, 0.0, written_by(0)),
        (2, MIDDLE, 33.0, written_by(1)),
        (3, LAST, 66.0, written_by(2)),
        (3, LAST, 66.0, written_by(3)),
    ];
    for (frame, (k, id, bar_value, written)) in frames.into_iter().enumerate() {
        // Escape on the second frame: refused while the run writes.
        let events = if frame == 1 {
            vec![
                key(egui::Key::Escape, egui::Modifiers::NONE, true),
                key(egui::Key::Escape, egui::Modifiers::NONE, false),
            ]
        } else {
            Vec::new()
        };
        let nodes = nodes_of(scene.draw_with(events));
        let name = display_name(&team_entry(&scene.tool.view, id).name);
        assert_eq!(
            line_starting(&nodes, "Writing "),
            Some(format!("Writing {k} of 3: {name} ({id})")),
            "frame {frame}"
        );
        let bar = nodes
            .iter()
            .find(|node| node.role() == Role::ProgressIndicator)
            .unwrap_or_else(|| panic!("a progress bar on frame {frame}"));
        assert_eq!(bar.numeric_value(), Some(bar_value), "frame {frame}'s bar");
        assert!(has_button(&nodes, "Stop"), "Stop on frame {frame}");
        assert_eq!(files_so_far(&folder), written, "after frame {frame}");
    }
    let nodes = scene.range_nodes();
    assert_folder_under(&nodes, "3 pictures written, they are here:", &folder);
    assert!(has_button(&nodes, "Open folder"));
    let mut expected: Vec<String> = teams.iter().map(|id| scene.file_name(*id)).collect();
    expected.sort_unstable();
    assert_eq!(files_in(&folder), expected, "one picture per team");
    for name in &expected {
        png_size(&folder.join(name));
    }
    assert_eq!(
        scene
            .table()
            .get("formation_pictures_folder")
            .and_then(toml::Value::as_str),
        Some(folder.display().to_string().as_str()),
        "the run's folder is kept"
    );
    let nodes = scene.press("Close");
    assert!(!label_value(&nodes, RANGE_TITLE), "Close closes the dialog");
}

// SE-GUI-89
#[test]
fn stop_ends_the_run_with_the_pictures_written_so_far() {
    let mut scene = Scene::new("pictures_range_stop");
    let (kept, none) = (scene.dir.join("kept"), scene.dir.join("none"));

    // Stop pressed while `Writing 2 of 3` is on screen: the frame after it
    // writes the second picture meanwhile, and the click reaches the frame
    // after that one. The two pictures stay, the third is never written.
    scene.start_run(&kept);
    let pairs = scene.draw_with(Vec::new());
    let stop = button_id(&pairs, "Stop");
    let nodes = nodes_of(scene.draw_with(Vec::new()));
    assert!(
        line_starting(&nodes, "Writing 2 of 3: ").is_some(),
        "premise: the second frame reads 2 of 3"
    );
    scene.draw();
    assert_eq!(
        files_in(&kept).len(),
        2,
        "premise: the second picture written"
    );
    let nodes = nodes_of(scene.draw_with(vec![action_request(stop)]));
    assert!(
        line_starting(&nodes, "Writing 3 of 3: ").is_some(),
        "Stop pressed on the fourth frame"
    );
    let nodes = scene.range_nodes();
    assert_folder_under(
        &nodes,
        "Stopped after 2 of 3 pictures, they are here:",
        &kept,
    );
    assert!(has_button(&nodes, "Open folder"));
    assert!(has_button(&nodes, "Close"));
    let mut expected = [scene.file_name(FIRST), scene.file_name(MIDDLE)];
    expected.sort_unstable();
    assert_eq!(files_in(&kept), expected);
    let setting = |scene: &Scene| {
        scene
            .table()
            .get("formation_pictures_folder")
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(setting(&scene), Some(kept.display().to_string()));
    scene.press("Close");

    // Stop on the run's first frame: no picture written, so no Open folder
    // and the folder setting as it was. The Stop button's id is the same in
    // every run, so the action reaches it on the frame that first draws it.
    scene.start_run(&none);
    let nodes = nodes_of(scene.draw_with(vec![action_request(stop)]));
    assert!(
        line_starting(&nodes, "Writing 1 of 3: ").is_some(),
        "Stop pressed on the first frame"
    );
    let nodes = scene.draw();
    assert!(
        label_value(&nodes, "Stopped before any picture was written"),
        "{:?}",
        line_starting(&nodes, "Stopped")
    );
    assert!(!has_button(&nodes, "Open folder"));
    assert!(has_button(&nodes, "Close"));
    assert!(!none.exists(), "nothing written");
    assert_eq!(setting(&scene), Some(kept.display().to_string()));
}

#[test]
fn a_failed_write_ends_the_run_and_names_the_error() {
    let mut scene = Scene::new("pictures_range_failed");
    let folder = scene.dir.join("a file");
    std::fs::write(&folder, b"not a folder").unwrap();
    let error = std::fs::create_dir_all(&folder).expect_err("premise: no folder can be made there");

    scene.start_run(&folder);
    // The first frame draws, the second writes the first picture and fails.
    let nodes = scene.draw();
    assert!(line_starting(&nodes, "Writing 1 of 3: ").is_some());
    scene.draw();
    let nodes = scene.draw();
    assert!(
        label_value(&nodes, "Stopped before any picture was written"),
        "the run ends at the first team: {:?}",
        line_starting(&nodes, "Stopped")
    );
    let named = format!("Could not write {}: {error}", scene.file_name(FIRST));
    assert!(label_value(&nodes, &named), "{named:?}");
    assert!(!has_button(&nodes, "Open folder"));
    assert!(
        !scene.table().contains_key("formation_pictures_folder"),
        "no folder kept"
    );
}

// A folder that is a file: the one-team dialog's write fails, the notice
// is the suite's one Error kind, the dialog stays open on the folder it
// was given, the setting keeps nothing, and the write is a `could not
// write` step.
#[test]
fn a_folder_that_is_a_file_fails_the_write_and_keeps_the_dialog_open() {
    let mut scene = Scene::new("picture_dialog_write_fails");
    let folder = scene.dir.join("a file");
    std::fs::write(&folder, b"not a folder").unwrap();
    let error = std::fs::create_dir_all(&folder).expect_err("premise: no folder can be made there");

    scene.open_dialog();
    scene.browse(&folder);
    let nodes = scene.press("Save");

    assert_eq!(
        scene.notices(),
        [Notice {
            text: format!("Could not write f.png: {error}"),
            kind: NoticeKind::Error,
            action: None,
        }],
        "the failed write's Error notice"
    );
    assert!(dialog_open(&nodes), "the dialog stays open");
    assert!(
        !scene.table().contains_key("formation_pictures_folder"),
        "no folder kept"
    );
    crate::test_support::assert_write_refused(
        scene
            .tool
            .view
            .session
            .as_ref()
            .expect("the save")
            .recorder(),
        &scene.dir,
        &folder.join("f.png"),
    );
}

/// The dialog keeps the picking state's width through the run and its
/// result, though the result's folder, a long one, would be wider.
#[test]
fn the_dialog_keeps_its_width_through_the_run() {
    let mut scene = Scene::new("pictures_range_width");
    let folder = scene
        .dir
        .join("formation pictures of every team of the cup, from the first to the last");

    scene.start_run(&folder);
    let picking = scene.dialog_width();
    let style = scene.window.ctx.style_of(scene.window.ctx.theme());
    let font = egui::TextStyle::Body.resolve(&style);
    let line = scene.window.ctx.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(folder.display().to_string(), font, egui::Color32::WHITE)
            .size()
            .x
    });
    assert!(
        line > picking,
        "premise: the folder's line, {line}, is wider than the picking state's {picking}"
    );
    for frame in 0..4 {
        let nodes = scene.draw();
        assert!(
            line_starting(&nodes, "Writing ").is_some(),
            "frame {frame} writes"
        );
        assert_eq!(scene.dialog_width(), picking, "writing frame {frame}");
    }
    let nodes = scene.range_nodes();
    assert_folder_under(&nodes, "3 pictures written, they are here:", &folder);
    assert_eq!(scene.dialog_width(), picking, "the result");
}

mod rows;
