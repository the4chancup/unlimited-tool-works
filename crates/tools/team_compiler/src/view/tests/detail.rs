//! The cells' detail windows driven through `Tool::view`: the view hands the grid its
//! catalog and its pictures, so what a window draws proves the wiring, not only the grid.

use super::*;

/// A click on the grid cell reading `text`, pinning its window: the nodes of the frame
/// after, the window laid out (a fresh window's first frame is a sizing pass).
fn pin_cell(
    window: &mut Headless,
    nodes: &[accesskit::Node],
    text: &str,
    tool: &mut Tool,
    ctx: &ToolContext,
) -> Vec<accesskit::Node> {
    let at = cell_rect(nodes, text).center();
    window.frame(press(at), |ui| tool.view(ui, ctx));
    window.frame(release(at), |ui| tool.view(ui, ctx));
    window.frame(Vec::new(), |ui| tool.view(ui, ctx));
    window.frame(Vec::new(), |ui| tool.view(ui, ctx)).1
}

/// The detail window's rect on screen, looked up in egui's memory by the `Area` id the
/// grid's `("compiler", "grid")` salt builds.
fn window_rect(ctx: &egui::Context) -> egui::Rect {
    let id = egui::ViewportId::ROOT
        .root_ui_id()
        .with((TOOL_ID, "grid"))
        .with("detail");
    ctx.memory(|memory| memory.area_rect(id))
        .expect("the detail window's area has a rect")
}

/// The bounds of the picture `texture` is painted as in `shapes`: its mesh.
fn painted_picture(
    shapes: &[egui::epaint::ClippedShape],
    texture: egui::TextureId,
) -> Option<egui::Rect> {
    shapes.iter().find_map(|clipped| match &clipped.shape {
        egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh.calc_bounds()),
        _ => None,
    })
}

/// The bounds inside `window` of the nodes reading `text`.
fn texts_in(nodes: &[accesskit::Node], window: egui::Rect, text: &str) -> Vec<egui::Rect> {
    nodes
        .iter()
        .filter(|node| node.value() == Some(text))
        .filter_map(node_rect)
        .filter(|rect| window.intersects(*rect))
        .collect()
}

/// Whether the open window names `place` in its place line: the first of its texts above
/// the state name's. A finding's scope names the same folder lower in the window, so a
/// bare text match cannot tell the two apart.
fn shows_place(nodes: &[accesskit::Node], ctx: &egui::Context, place: &str) -> bool {
    let window = window_rect(ctx);
    let state = [
        "Checked",
        "Done",
        "Done with warnings",
        "Done with errors",
        "Blocked",
        "Checking",
        "Unchecked",
        "Shallow check done (archive)",
    ];
    let state_top = state
        .iter()
        .flat_map(|name| texts_in(nodes, window, name))
        .map(|rect| rect.min.y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        state_top.is_finite(),
        "no state name found in the detail window {window:?}"
    );
    texts_in(nodes, window, place)
        .iter()
        .any(|rect| rect.min.y < state_top)
}

// TC-GUI-31
#[test]
fn a_player_s_window_draws_his_portrait_in_the_top_box() {
    let temp = sandbox("view_window_portrait");
    let root = temp.path();
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    let TopBox::Player {
        portrait: Some(file),
        ..
    } = &player(row(tool.live_check.rows(), "/egg/"), 5).unwrap().top
    else {
        panic!("05's box names a portrait");
    };
    let file = file.clone();
    let nodes = pin_cell(&mut window, &nodes, "05", &mut tool, &ctx);
    assert!(shows_text(&nodes, "/egg/ 05"), "premise: the window is up");

    // The picture is read on a worker thread: frames until the texture is uploaded, a
    // 120 s hang guard rather than a speed bound.
    let deadline = Instant::now() + Duration::from_secs(120);
    let texture = loop {
        window.frame(Vec::new(), |ui| tool.view(ui, &ctx));
        if let Some(texture) = tool.detail.pictures.texture(&window.ctx, &file) {
            break texture;
        }
        assert!(
            Instant::now() < deadline,
            "05's portrait was not read within 120 s"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let bounds =
        painted_picture(&window.shapes, texture.id()).expect("the portrait's texture is painted");
    let window = window_rect(&window.ctx);
    assert!(
        window.contains_rect(bounds),
        "inside the window's top box: {bounds:?} in {window:?}"
    );
}

// TC-GUI-21
#[test]
fn the_label_s_window_shows_the_catalog_s_when_text() {
    let temp = sandbox("view_window_catalog");
    let root = temp.path();
    list_teams(root);
    add_player(root, "co Midcup Map", "03 - B");
    add_file(root, "co Midcup Map/Boots/Solo/boots.fmdl", b"");
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    let nodes = pin_cell(&mut window, &nodes, "/co/", &mut tool, &ctx);
    assert!(
        shows_text(&nodes, "shared_folder_orphaned"),
        "premise: the finding's code shows"
    );
    assert!(shows_text(
        &nodes,
        "A shared Faces/, Boots/ or Gloves/ folder is linked by no player."
    ));
}

// TC-GUI-20
#[test]
fn the_windows_of_two_referee_slots_sharing_a_folder_both_name_it() {
    let temp = sandbox("view_window_refs");
    let root = temp.path();
    add_file(root, "refs Cup/players.txt", b"01 A\n02 B\n35 A\n");
    add_player(root, "refs Cup", "A");
    add_player(root, "refs Cup", "B");
    let ctx = tool_context(root, "");
    let mut tool = Tool::new();
    let mut window = Headless::new();
    let nodes = first_check(&mut tool, &ctx, &mut window);
    let nodes = pin_cell(&mut window, &nodes, "01", &mut tool, &ctx);
    assert!(
        shows_place(&nodes, &window.ctx, "Players/A"),
        "slot 01's window"
    );
    let nodes = pin_cell(&mut window, &nodes, "35", &mut tool, &ctx);
    assert!(
        shows_place(&nodes, &window.ctx, "Players/A"),
        "slot 35's window"
    );
}
