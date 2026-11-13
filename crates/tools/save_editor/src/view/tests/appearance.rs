use pes_savefile::model::PlayerMotion;
use pes_savefile::schema::ingame_face::IngameFaceField;
use studio_core::StudioTool;

use super::*;

/// The node of the Appearance section's "Edit aesthetics anyway" switch.
fn edit_anyway(nodes: &[accesskit::Node]) -> &accesskit::Node {
    nodes
        .iter()
        .find(|node| {
            node.role() == Role::CheckBox && node.label() == Some("Edit aesthetics anyway")
        })
        .expect("the Edit aesthetics anyway switch")
}

/// The locked section drawn on `version` under "Aesthetics from exports": a
/// fresh `SaveView` on the Appearance section of `version`'s fixture, one
/// frame's nodes in, window and view back.
fn locked_section(version: PesVersion, test: &str) -> (SaveView, Headless, Vec<accesskit::Node>) {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    view.aesthetics_from_exports = true;
    let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    (view, window, nodes)
}

// SE-GUI-07
#[test]
fn the_section_is_locked_on_a_fox_save_until_the_switch_unlocks_it() {
    let (mut view, mut window, nodes) = locked_section(PesVersion::Pes21, "appearance_lock_fox");

    // Locked: the ownership line with the patch step and the Apply
    // aesthetics patch link, every control of the groups disabled, the
    // switch and the Cutscenes group's two controls the ones that are not
    // (star and playing attitude sit outside the lock).
    assert!(
        label_value(
            &nodes,
            "The team's export owns the aesthetics, so edit its settings.toml (or its ingame_face.toml, for the face, hair and iris), recompile, then apply the patch."
        ),
        "the ownership line"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("Apply aesthetics patch")),
        "the patch link"
    );
    for label in ["Star", "Playing attitude"] {
        let control = right_of_label(&nodes, label, Role::SpinButton)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("the {label} control"));
        assert!(
            !control.is_disabled(),
            "{label} stays editable while locked"
        );
    }
    // The self-labeled controls (checkboxes, the switch, a swatch, the
    // button) and the label/pair ones (a Label node beside its control —
    // Player gloves' box is bare, its label in the cell before it).
    for (label, role) in [
        ("Face", Role::CheckBox),
        ("Motion", Role::CheckBox),
        ("Edit aesthetics anyway", Role::CheckBox),
        ("Skin Light", Role::ColorWell),
        ("Randomize", Role::Button),
    ] {
        let control = nodes
            .iter()
            .find(|node| node.role() == role && node.label() == Some(label))
            .unwrap_or_else(|| panic!("the {label} control"));
        assert_eq!(
            control.is_disabled(),
            label != "Edit aesthetics anyway",
            "{label}"
        );
    }
    for (label, role) in [
        ("Chest", Role::SpinButton),
        ("Corner kick", Role::SpinButton),
        ("Goal celebration 1", Role::SpinButton),
        ("Boots ID", Role::SpinButton),
        ("Copy from ID", Role::SpinButton),
        ("Sleeves", Role::ComboBox),
        ("Shirttail out", Role::CheckBox),
        ("Ankle taping", Role::CheckBox),
        ("Player gloves", Role::CheckBox),
    ] {
        let control = right_of_label(&nodes, label, role)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("the {label} control"));
        assert!(control.is_disabled(), "{label}");
    }

    // On: the note warns a later patch overwrites hand edits, and the
    // controls draw enabled. The switch sits below the fold: its click goes
    // through the AccessKit action.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (anyway, _) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::CheckBox && node.label() == Some("Edit aesthetics anyway")
        })
        .expect("the Edit aesthetics anyway switch");
    action_click(&mut window, &mut view, *anyway);
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Chest")
    });
    assert!(
        label_value(
            &nodes,
            "A later patch overwrites whatever is edited by hand."
        ),
        "the note under the switch"
    );
    let chest = right_of_label(&nodes, "Chest", Role::SpinButton)
        .first()
        .copied()
        .expect("the Chest control");
    assert!(!chest.is_disabled(), "unlocked, Chest is enabled");

    // A physique edit lands in the session's player as one undo step
    // (shown -7 to 7, stored +7: typed 5 stores 12).
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        7,
        "premise: Chest stores 7 (shown 0)"
    );
    click(&mut window, chest, &mut view);
    window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .appearance
            .chest,
        12,
        "typed 5 stores 12"
    );
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");

    // A fresh view (the tool's next start) is locked again; the same view
    // opening another save keeps the switch on.
    let path = fixture(PesVersion::Pes21, "appearance_lock_fox_fresh");
    let mut fresh = SaveView::new();
    fresh.open(&path);
    fresh.aesthetics_from_exports = true;
    open_appearance(&mut window, &mut fresh);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut fresh, &|| Vec::new()));
    assert!(
        right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "the next start is locked again"
    );

    let other = fixture(PesVersion::Pes21, "appearance_lock_fox_other");
    view.open(&other);
    open_appearance(&mut window, &mut view);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        !right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "opening another save keeps it unlocked"
    );

    // The link opens the patch panel. It may sit above the fold: its click
    // goes through the AccessKit action.
    assert_ne!(view.panel, Some(Panel::Patch), "premise: no panel open");
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (link, _) = pairs
        .iter()
        .find(|(_, node)| node.label() == Some("Apply aesthetics patch"))
        .expect("the patch link");
    action_click(&mut window, &mut view, *link);
    assert_eq!(view.panel, Some(Panel::Patch), "the link opens the panel");
}

// SE-GUI-07
#[test]
fn the_section_is_locked_on_a_pre_fox_save_with_the_patch_step() {
    let (mut view, mut window, nodes) = locked_section(PesVersion::Pes17, "appearance_lock_prefox");

    // Locked: the pre-Fox ownership line carries the patch step and the
    // Apply aesthetics patch link (disabled controls' hover is the same
    // sentence).
    assert!(
        label_value(
            &nodes,
            "The team's export owns the aesthetics, so edit its settings.toml (or its ingame_face.toml, for the face, hair and iris), recompile, then apply the patch."
        ),
        "the pre-Fox ownership line"
    );
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("Apply aesthetics patch")),
        "the patch link"
    );
    assert!(
        right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "locked on PES 17 too"
    );
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Chest")
    });
    let ownership_hover = right_of_label(&nodes, "Chest", Role::SpinButton)[0];
    assert_eq!(
        tooltip_containing(&mut window, &mut view, centre(ownership_hover), "The team's export"),
        Some(
            "The team's export owns the aesthetics, so edit its settings.toml (or its ingame_face.toml, for the face, hair and iris), recompile, then apply the patch."
                .to_string()
        ),
        "the disabled controls' hover is the ownership line"
    );

    // On: the note warns a later patch overwrites hand edits. The switch
    // sits above the fold again after the scroll: its click goes through
    // the AccessKit action.
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    let (anyway, _) = pairs
        .iter()
        .find(|(_, node)| {
            node.role() == Role::CheckBox && node.label() == Some("Edit aesthetics anyway")
        })
        .expect("the Edit aesthetics anyway switch");
    action_click(&mut window, &mut view, *anyway);
    let (_, nodes, _) = window.frame(Vec::new(), |ui| show(ui, &mut view, &|| Vec::new()));
    assert!(
        label_value(
            &nodes,
            "A later patch overwrites whatever is edited by hand."
        ),
        "the pre-Fox note under the switch"
    );
    assert!(
        !right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "unlocked on PES 17 too"
    );
    // PES 17 still has the base-copy flag, enabled with the rest.
    let base_copy = nodes
        .iter()
        .find(|node| node.role() == Role::CheckBox && node.label() == Some("Base copy"))
        .expect("the Base copy flag on PES 17");
    assert!(!base_copy.is_disabled(), "PES 17 holds the flag");
}

// SE-GUI-06
#[test]
fn the_gated_star_and_attitude_draw_disabled_in_place_on_pes17() {
    // The Cutscenes group's half of the scenario (the stronger hand's is
    // the Abilities test): star and playing attitude draw in the
    // Appearance section, disabled `-` with the `PES NN+` tooltip on PES 17,
    // enabled in the same rects on PES 21.
    let bounds_of = |version: PesVersion, test: &str| {
        let path = fixture(version, test);
        let mut view = SaveView::new();
        view.open(&path);
        let mut window = Headless::sized(DEFAULT_PANE.x, 700.0);
        open_appearance(&mut window, &mut view);
        // The group's controls sit below the fold: the hovers need them
        // inside the scroll's clip (the attitude's row is the lower one).
        let nodes = scroll_into_view(&mut window, &mut view, |node| {
            node.role() == Role::Label && node.value() == Some("Playing attitude")
        });
        let star = control_right_of(&nodes, "Star").bounds().expect("bounds");
        let attitude = control_right_of(&nodes, "Playing attitude")
            .bounds()
            .expect("bounds");
        (view, window, nodes, (star, attitude))
    };
    let (mut view17, mut window17, _nodes17, (star17, attitude17)) =
        bounds_of(PesVersion::Pes17, "gated_cutscenes_17");
    let (_view21, _window21, nodes21, (star21, attitude21)) =
        bounds_of(PesVersion::Pes21, "gated_cutscenes_21");

    for (control, tooltip) in [("Star", "PES 19+"), ("Playing attitude", "PES 20+")] {
        // Each hover needs its own dash inside the clip: scrolling for one
        // can push the other out.
        let nodes = scroll_into_view(&mut window17, &mut view17, |node| {
            node.role() == Role::Label && node.value() == Some(control)
        });
        let button = disabled_dash(&nodes, control);
        assert_eq!(
            tooltip_after_hover(&mut window17, &mut view17, centre(button)),
            Some(tooltip.to_string()),
            "{control}'s tooltip"
        );
    }
    for label in ["Star", "Playing attitude"] {
        assert!(
            !control_right_of(&nodes21, label).is_disabled(),
            "{label} is enabled on PES 21"
        );
    }
    assert_eq!(star17, star21, "the Star control's rect");
    assert_eq!(
        attitude17, attitude21,
        "the Playing attitude control's rect"
    );
}

// SE-GUI-07
#[test]
fn the_cutscenes_group_stays_editable_above_the_ownership_line() {
    // Locked PES 21: the group's two controls draw above the ownership
    // line, enabled, while the groups below it stay disabled; a Star edit
    // lands as one undoable edit.
    let (mut view, mut window, nodes) = locked_section(PesVersion::Pes21, "cutscenes_editable");
    let ownership_y = nodes
        .iter()
        .find(|node| {
            node.role() == Role::Label
                && node
                    .value()
                    .is_some_and(|v| v.starts_with("The team's export owns"))
        })
        .and_then(accesskit::Node::bounds)
        .map(|bounds| bounds.y0)
        .expect("the ownership line");
    for label in ["Star", "Playing attitude"] {
        let control = right_of_label(&nodes, label, Role::SpinButton)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("the {label} control"));
        assert!(!control.is_disabled(), "{label} is editable while locked");
        assert!(
            control.bounds().expect("bounds").y0 < ownership_y,
            "{label} sits above the ownership line"
        );
    }
    assert!(
        right_of_label(&nodes, "Chest", Role::SpinButton)[0].is_disabled(),
        "the groups below the line stay locked"
    );

    let star_of = |view: &SaveView| {
        view.session
            .as_ref()
            .unwrap()
            .file()
            .player(70101)
            .unwrap()
            .stats
            .star
    };
    assert_eq!(star_of(&view), Some(0), "premise: star 0");
    let nodes = scroll_into_view(&mut window, &mut view, |node| {
        node.role() == Role::Label && node.value() == Some("Playing attitude")
    });
    let star = right_of_label(&nodes, "Star", Role::SpinButton)
        .first()
        .copied()
        .expect("the Star control");
    click(&mut window, star, &mut view);
    window.frame(vec![egui::Event::Text("5".to_owned())], |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    window.frame(
        vec![key(egui::Key::Enter, egui::Modifiers::NONE, true)],
        |ui| show(ui, &mut view, &|| Vec::new()),
    );
    assert_eq!(star_of(&view), Some(5), "the typed star stores");
    assert_eq!(undo_entries(&mut window, &mut view), 1, "one undo step");
    assert_eq!(star_of(&view), Some(0), "the undo restores it");
}

mod controls;
mod layout;
