//! The two picture dialogs' vertical room: the body 12 under the title, the
//! form's rows 24 apart and the first checkbox 6 under the form, the
//! checkboxes 22 apart, the range dialog's hint 8 under them and the buttons
//! 10 under what is above them.

use super::*;

/// The bounds of `node`, which the frame drew.
fn bounds(node: &accesskit::Node) -> accesskit::Rect {
    node.bounds().expect("drawn bounds")
}

/// Asserts `lower` stands `expected` points under `upper`, within half a
/// point, naming both.
#[track_caller]
fn assert_gap(upper: f64, lower: f64, expected: f64, what: &str) {
    assert!(
        (lower - upper - expected).abs() <= 0.5,
        "{what}: {lower} under {upper} is {}, expected {expected}",
        lower - upper
    );
}

/// The three checkboxes' bounds, top to bottom. Asserts they stand 22
/// apart, top to top.
#[track_caller]
fn boxes_22_apart(nodes: &[accesskit::Node]) -> [accesskit::Rect; 3] {
    let boxes = [SETTINGS_BOX, "Portraits", FLAT_BOX]
        .map(|label| bounds(node_labeled(nodes, Role::CheckBox, label)));
    assert_gap(
        boxes[0].y0,
        boxes[1].y0,
        22.0,
        "Portraits' top under the settings box's",
    );
    assert_gap(
        boxes[1].y0,
        boxes[2].y0,
        22.0,
        "the grain box's top under Portraits'",
    );
    boxes
}

#[test]
fn the_picture_dialog_stands_12_under_its_title_its_rows_24_apart() {
    let mut scene = Scene::new("pictures_rows");
    let nodes = scene.open_dialog();
    let title = label_bounds(&nodes, "Save a picture of the formations");
    let folder = bounds(field_right_of_label(&nodes, "Folder", Role::TextInput));
    let file_name = bounds(field_right_of_label(&nodes, "File name", Role::TextInput));
    assert_gap(
        title.y1,
        folder.y0,
        12.0,
        "the Folder box's top under the title's bottom",
    );
    assert_gap(
        folder.y0,
        file_name.y0,
        24.0,
        "the File name box's top under the Folder box's",
    );
    let boxes = boxes_22_apart(&nodes);
    assert_gap(
        file_name.y1,
        boxes[0].y0,
        6.0,
        "the settings box's top under the File name box's bottom",
    );
    let save = bounds(node_labeled(&nodes, Role::Button, "Save"));
    assert_gap(
        boxes[2].y1,
        save.y0,
        10.0,
        "Save's top under the grain box's bottom",
    );
}

#[test]
fn the_range_dialog_stands_12_under_its_title_its_rows_24_apart_its_hint_8_under_the_boxes() {
    let mut scene = Scene::new("pictures_range_rows");
    scene.open_range();
    let nodes = scene.range_nodes();
    let title = label_bounds(&nodes, RANGE_TITLE);
    let first = bounds(combos(&nodes)[0]);
    let folder = bounds(field_right_of_label(&nodes, "Folder", Role::TextInput));
    assert_gap(
        title.y1,
        first.y0,
        12.0,
        "the first combo's top under the title's bottom",
    );
    assert_gap(
        first.y0,
        folder.y0,
        24.0,
        "the Folder box's top under the first combo's",
    );
    let boxes = boxes_22_apart(&nodes);
    assert_gap(
        folder.y1,
        boxes[0].y0,
        6.0,
        "the settings box's top under the Folder box's bottom",
    );
    let hint = line_starting(&nodes, "One picture per team, ").expect("the hint line");
    let hint = label_bounds(&nodes, &hint);
    assert_gap(
        boxes[2].y1,
        hint.y0,
        8.0,
        "the hint's top under the grain box's bottom",
    );
    let save = bounds(node_labeled(&nodes, Role::Button, "Save pictures"));
    assert_gap(
        hint.y1,
        save.y0,
        10.0,
        "Save pictures' top under the hint's bottom",
    );
}
