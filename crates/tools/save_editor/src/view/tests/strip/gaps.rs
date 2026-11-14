//! The gaps between General info's rows (the draft's `.gen` and `.pairs`
//! gaps): 6 points between boxes, between blocks as inside one.

use super::*;

// Between the counted fields and the pair rows, inside the pairs, and
// between the pairs and the combos, a box stands 6 points under the last
// (FIELD_SPACING).
#[test]
fn the_rows_of_general_info_leave_six_between_their_boxes() {
    let path = fixture(PesVersion::Pes21, "strip_block_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    let field = |label: &str| {
        right_of_label(&nodes, label, Role::SpinButton)
            .first()
            .expect("the field")
            .bounds()
            .expect("the field's bounds")
    };
    let shirt = field_right_of_label(&nodes, "Shirt (", Role::TextInput)
        .bounds()
        .expect("the Shirt box");
    let height = field("Height");
    let weight = field("Weight");
    let age = field("Age");
    // The foot toggle's buttons are the row's height, so one's top is the
    // combos' first row's top.
    let foot = right_of_label(&nodes, "Stronger foot", Role::Button)
        .first()
        .expect("the foot toggle")
        .bounds()
        .expect("the foot toggle's bounds");
    for (above, below, expected, what) in [
        (shirt, height, 6.0, "Shirt to Height"),
        (height, weight, 6.0, "Height to Weight"),
        (weight, age, 6.0, "Weight to Age"),
        (age, foot, 6.0, "Age to the foot toggle"),
    ] {
        let gap = below.y0 - above.y1;
        assert!(
            (gap - expected).abs() <= 0.3,
            "{what}: a {expected}-point gap ({gap})"
        );
    }
}
