//! The counted labels' AccessKit reports: `Name (n/max)` and
//! `Shirt (n/max)` hand-set their `WidgetInfo` (the label is painted by
//! hand, not a widget), so a disabled report would read a working label
//! as greyed out to a screen reader.

use super::*;

// The counted labels read as labels, enabled.
#[test]
fn the_counted_labels_report_enabled() {
    let path = fixture(PesVersion::Pes21, "counted_labels_enabled");
    let mut view = SaveView::new();
    view.open(&path);
    let mut window = Headless::new();
    let nodes = draw(&mut window, &mut view);
    for prefix in ["Name (", "Shirt ("] {
        let label = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|value| value.starts_with(prefix))
            })
            .unwrap_or_else(|| panic!("the {prefix} label"));
        assert_widget_named(label, Role::Label, prefix);
    }
}
