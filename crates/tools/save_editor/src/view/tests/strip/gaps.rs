//! The gaps between the first column's panels' rows (the draft's `.gen`
//! and `.pairs` gaps): 6 points between boxes, between blocks as inside
//! one.

use super::*;

// Between the counted fields and the pair row, inside Main stats' pairs,
// and between the pairs and the combos, a box stands 6 points under the
// last (FIELD_SPACING).
#[test]
fn the_rows_of_the_first_column_leave_six_between_their_boxes() {
    let path = fixture(PesVersion::Pes21, "strip_block_gaps");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    let field = |label: &str, role: Role| {
        right_of_label(&nodes, label, role)
            .first()
            .expect("the field")
            .bounds()
            .expect("the field's bounds")
    };
    let shirt = field_right_of_label(&nodes, "Shirt name (", Role::TextInput)
        .bounds()
        .expect("the Shirt name box");
    let number = field("Shirt number", Role::SpinButton);
    let height = field("Height", Role::SpinButton);
    let injury = field("Injury Resistance", Role::SpinButton);
    let weak = field("Weak Foot: Usage", Role::SpinButton);
    for (above, label, role, what) in [
        (
            shirt,
            "Shirt number",
            Role::SpinButton,
            "Shirt name to Shirt number",
        ),
        (
            height,
            "Injury Resistance",
            Role::SpinButton,
            "Height to Injury Resistance",
        ),
        (number, "Nation", Role::ComboBox, "Shirt number to Nation"),
        (
            injury,
            "Weak Foot: Usage",
            Role::SpinButton,
            "Injury Resistance to Weak Foot: Usage",
        ),
        // The foot toggle's buttons are the row's height, so one's top is
        // the combos' first row's top.
        (
            weak,
            "Stronger foot",
            Role::Button,
            "Weak Foot: Usage to the foot toggle",
        ),
    ] {
        let below = field(label, role);
        let gap = below.y0 - above.y1;
        assert!((gap - 6.0).abs() <= 0.3, "{what}: a 6-point gap ({gap})");
    }
}
