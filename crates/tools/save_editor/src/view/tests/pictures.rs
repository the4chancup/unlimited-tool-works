//! The formation picture dialog's tests (SE-GUI-86): the Team menu's entry
//! end to end through `Tool::view`, so the program's folder and the two
//! remembered settings are the tool context's.

use studio_core::{ActionEffect, Notice, NoticeAction, NoticeKind, ShellRequest, StudioTool};

use super::*;

/// The PES 2021 fixture's /f/.
const F: u32 = 718;

/// The Team menu's entry.
const ENTRY: &str = "Save a picture of the formations\u{2026}";

/// The grain box's label.
const FLAT_BOX: &str = "Without the pitch's grain (smaller file)";

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

    /// The dialog opened from the Team menu: the nodes of the first frame
    /// it stands where the frame before put it. Its grid takes egui a few
    /// frames to size, and the dialog moves while it does, so a place read
    /// earlier is not where a click lands.
    fn open_dialog(&mut self) -> Vec<accesskit::Node> {
        choose(&mut self.window, &mut self.tool.view, "Team", ENTRY);
        let place =
            |nodes: &[accesskit::Node]| node_labeled(nodes, Role::CheckBox, FLAT_BOX).bounds();
        let mut last = place(&self.draw());
        for _ in 0..10 {
            let nodes = self.draw();
            if place(&nodes) == last {
                assert!(dialog_open(&nodes), "the entry opens the dialog");
                return nodes;
            }
            last = place(&nodes);
        }
        panic!("the dialog never stood still");
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
