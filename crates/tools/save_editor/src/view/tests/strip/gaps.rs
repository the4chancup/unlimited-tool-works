//! The gaps between General info's blocks (the draft's `.gen` gap): 6
//! points between blocks, against the rows' own 4 inside one.

use super::*;

// Between the counted fields and the pair rows, and between the pairs
// and the combos, a block stands 6 points under the last; inside the
// pairs the rows stand their own 4 (FORM_ROW_SPACING).
#[test]
fn the_blocks_of_general_info_stand_six_apart() {
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
        (height, weight, 4.0, "Height to Weight"),
        (weight, age, 4.0, "Weight to Age"),
        (age, foot, 6.0, "Age to the foot toggle"),
    ] {
        let gap = below.y0 - above.y1;
        assert!(
            (gap - expected).abs() <= 0.3,
            "{what}: a {expected}-point gap ({gap})"
        );
    }
}
