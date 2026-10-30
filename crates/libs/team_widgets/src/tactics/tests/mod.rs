//! The `tactics_settings` group's tests: the version gating, the
//! per-preset writes, the no-change cases and the per-control field
//! mapping — plus the `Headless` helpers `instructions`' and
//! `assignments`' test modules use (`crate::tactics::tests`).

use egui::accesskit::{self, Role};

use pes_savefile::model::instruction::Instruction;

use super::*;
use crate::test_support::{
    Headless, centre, chip_under, chips_below, combo_row, control_below, control_right_of,
    controls_below, entry, entry_id, fixture_rows, fixture_tactics, pointer_at, roster_rows,
    tactics_settings,
};

impl Headless {
    /// One frame of `tactics_settings` over `tactics`, `view` and
    /// `roster`: the frame's AccessKit nodes and whether the frame
    /// changed `tactics`.
    pub(crate) fn tactics(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        self.tactics_at(None, version, tactics, view, roster, events)
    }

    /// `tactics` with `time` as the frame's timestamp (tooltips count
    /// their delay on it).
    pub(crate) fn tactics_at(
        &mut self,
        time: Option<f64>,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        let mut changed = None;
        let (nodes, _) = self.frame_at(time, events, |ui| {
            // The preset bar as the host draws it, then the panels.
            preset_selector(ui, view);
            changed = Some(
                fluid_formation(ui, tactics, view)
                    | tactics_settings(ui, "tactics", version, tactics, view, roster),
            );
        });
        (nodes, changed.expect("the widget ran"))
    }

    /// A click at `pos` over the widget (press frame, then the release
    /// frame `clicked()` lands on).
    pub(crate) fn click_tactics(
        &mut self,
        pos: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, bool) {
        self.tactics(version, tactics, view, roster, pointer_at(pos, true));
        self.tactics(version, tactics, view, roster, pointer_at(pos, false))
    }

    /// `tactics` keeping the nodes' AccessKit ids (for
    /// `AccessKitActionRequest` events).
    pub(crate) fn tactics_ids(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<(accesskit::NodeId, accesskit::Node)>, bool) {
        let mut changed = None;
        let (nodes, _) = self.frame_at_ids(None, events, |ui| {
            preset_selector(ui, view);
            changed = Some(
                fluid_formation(ui, tactics, view)
                    | tactics_settings(ui, "tactics", version, tactics, view, roster),
            );
        });
        (nodes, changed.expect("the widget ran"))
    }

    /// An AccessKit `Click` action on `id`: a click delivered as the
    /// accessibility event, for widgets a pointer click does not reach
    /// (a combo's popup items).
    pub(crate) fn action_click_tactics(
        &mut self,
        id: accesskit::NodeId,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, bool) {
        self.tactics(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: id,
                    data: None,
                },
            )],
        )
    }

    /// The text of the tooltip drawn while the pointer rests at `pos`
    /// containing `needle` (bounded: a missing tooltip fails rather
    /// than hangs; needs the context's `tooltip_delay` at 0). The
    /// hover must add a node holding `needle` to the frame the pointer
    /// left from: a label the view always draws with the same text (a
    /// closed combo's `value` is the selected text its tooltip repeats)
    /// cannot pass for the tooltip. The pointer first moves away for a
    /// frame: egui refuses a tooltip while the last click is more recent
    /// than the last movement.
    pub(crate) fn tactics_tooltip(
        &mut self,
        pos: egui::Pos2,
        needle: &str,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> Option<String> {
        let holding = |nodes: &[accesskit::Node]| -> Vec<String> {
            nodes
                .iter()
                .filter_map(|node| node.value())
                .filter(|value| value.contains(needle))
                .map(str::to_owned)
                .collect()
        };
        let (resting, _) = self.tactics(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(egui::Pos2::ZERO)],
        );
        let resting = holding(&resting);
        self.tactics(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(pos)],
        );
        let count =
            |values: &[String], value: &String| values.iter().filter(|v| *v == value).count();
        for _ in 0..90 {
            let (nodes, _) = self.tactics(version, tactics, view, roster, vec![]);
            let values = holding(&nodes);
            if let Some(added) = values
                .iter()
                .find(|value| count(&values, value) > count(&resting, value))
            {
                return Some(added.clone());
            }
        }
        None
    }
}

/// A click on `node`: press on one frame, release on the next.
pub(crate) fn click(
    window: &mut Headless,
    node: &accesskit::Node,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) -> (Vec<accesskit::Node>, bool) {
    window.click_tactics(centre(node), version, tactics, view, roster)
}

// A field the version lacks stays in its place, disabled, so the tab
// stays spatially stable across versions, its hover naming the first
// version that has it.
#[test]
fn fields_the_version_lacks_draw_disabled_with_their_versions() {
    let mut window = Headless::sized(800.0, 1200.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    // PES 16: the instructions panel holds the weak "(none on this
    // version)" line with "PES 17+" on hover; and Attack/defence
    // levels is disabled with "PES 17+".
    let mut tactics = fixture_tactics(PesVersion::Pes16);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes16);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(PesVersion::Pes16, &mut tactics, &mut view, &roster, vec![]);
    let line = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("(none on this version)"))
        .and_then(|node| node.bounds())
        .expect("the no-table line");
    let line_centre = egui::pos2(
        ((line.x0 + line.x1) / 2.0) as f32,
        ((line.y0 + line.y1) / 2.0) as f32,
    );
    assert_eq!(
        window.tactics_tooltip(
            line_centre,
            "PES 17+",
            PesVersion::Pes16,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("PES 17+".to_string()),
        "the no-table line's hover"
    );
    let levels = entry(&nodes, Role::CheckBox, "Attack/defence levels");
    assert!(levels.is_disabled(), "the flag is disabled on PES 16");
    assert_eq!(
        window.tactics_tooltip(
            centre(levels),
            "PES 17+",
            PesVersion::Pes16,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("PES 17+".to_string()),
        "Attack/defence levels' hover"
    );
    // And no instruction combo anywhere: the frame's only combo is
    // Automatic settings' Substitution.
    let combos: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::ComboBox)
        .collect();
    let [combo] = combos.as_slice() else {
        panic!("the Substitution combo alone, got {combos:?}")
    };
    let substitution = control_right_of(&nodes, "Substitution", Role::ComboBox);
    assert_eq!(
        combo.bounds(),
        substitution.bounds(),
        "the frame's only combo is Substitution's"
    );

    // PES 15: Fluid formation is disabled with "PES 16+".
    let mut tactics = fixture_tactics(PesVersion::Pes15);
    let mut view = TacticsView::default();
    let rows = fixture_rows(PesVersion::Pes15);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(PesVersion::Pes15, &mut tactics, &mut view, &roster, vec![]);
    let fluid = entry(&nodes, Role::CheckBox, "Fluid formation");
    assert!(fluid.is_disabled(), "the flag is disabled on PES 15");
    assert_eq!(
        window.tactics_tooltip(
            centre(fluid),
            "PES 16+",
            PesVersion::Pes15,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("PES 16+".to_string()),
        "Fluid formation's hover"
    );
}

// Every control edits `presets[view.preset]` or the team-wide fields,
// nothing else: with preset 2 on screen, a slider move and a style
// click write the second preset alone.
#[test]
fn the_controls_edit_the_preset_on_screen() {
    let mut window = Headless::sized(800.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let presets_before = tactics.presets.clone();

    // The preset selector: pick 2 (writes `view`, not the record).
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let two = entry(&nodes, Role::Button, "2");
    let (_, changed) = click(&mut window, two, version, &mut tactics, &mut view, &roster);
    assert_eq!(view.preset, 1, "preset 2 stays on screen");
    assert!(!changed, "the preset selector writes no record");
    assert_eq!(tactics.presets, presets_before, "nor touches one");

    // Move Support range's slider one stop up.
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let slider = control_below(&nodes, "Support range", Role::Slider, settings_half(&nodes));
    let bounds = slider.bounds().expect("slider bounds");
    let from = egui::pos2(((bounds.x0 + bounds.x1) / 2.0) as f32, centre(slider).y);
    let to = egui::pos2(bounds.x1 as f32 - 4.0, from.y);
    let mut changed = false;
    for events in [
        pointer_at(from, true),
        vec![egui::Event::PointerMoved(to)],
        pointer_at(to, false),
    ] {
        let (_, frame_changed) = window.tactics(version, &mut tactics, &mut view, &roster, events);
        changed |= frame_changed;
    }
    assert!(changed, "the drag changed the record");
    assert_ne!(
        tactics.presets[1].sliders, presets_before[1].sliders,
        "preset 2's sliders moved"
    );
    assert_eq!(tactics.presets[0], presets_before[0], "preset 1 untouched");
    assert_eq!(tactics.presets[2], presets_before[2], "preset 3 untouched");

    // Click a style pair's on choice.
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let possession = entry(&nodes, Role::Button, "Possession");
    let (_, changed) = click(
        &mut window,
        possession,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the click changed the record");
    assert!(
        tactics.presets[1].style.attacking_style,
        "preset 2's attacking style is Possession"
    );
    assert_eq!(tactics.presets[0], presets_before[0], "preset 1 untouched");
    assert_eq!(tactics.presets[2], presets_before[2], "preset 3 untouched");
}

// The widget reports what changed and nothing else: an idle frame, a
// hover, opening a combo and closing it, or re-picking
// the current value reports no change and leaves the record equal.
#[test]
fn no_gesture_no_change() {
    let mut window = Headless::sized(800.0, 1200.0);
    for version in [PesVersion::Pes15, PesVersion::Pes17, PesVersion::Pes21] {
        let mut tactics = fixture_tactics(version);
        // Every chip shows a name: the "Unassigned" pick found below is
        // the combo's own item.
        crate::test_support::named_assignments(&mut tactics);
        let before = tactics.clone();
        let mut view = TacticsView::default();
        let rows = fixture_rows(version);
        let roster = roster_rows(&rows);
        let (nodes, changed) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        assert!(!changed, "{version:?}: an idle frame reports no change");
        assert_eq!(tactics, before, "{version:?}: and edits nothing");

        // Hover over every interactive control: each frame reports no
        // change and edits nothing.
        let controls: Vec<egui::Pos2> = nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.role(),
                    Role::Button
                        | Role::CheckBox
                        | Role::ComboBox
                        | Role::Slider
                        | Role::SpinButton
                ) && !node.is_disabled()
            })
            .map(centre)
            .collect();
        for pos in controls {
            let (_, changed) = window.tactics(
                version,
                &mut tactics,
                &mut view,
                &roster,
                vec![egui::Event::PointerMoved(pos)],
            );
            assert!(!changed, "{version:?}: a hover reports no change");
            assert_eq!(tactics, before, "{version:?}: a hover edits nothing");
        }

        if version == PesVersion::Pes15 {
            continue;
        }
        // Open the first attack instruction's combo and close it again.
        let combo = combo_row(&nodes, 0)[0];
        let (_, changed) = click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "{version:?}: opening the combo writes nothing");
        assert_eq!(tactics, before, "{version:?}: and edits nothing");

        // Re-pick the current value inside it (every instruction Off
        // here, so the "Unassigned" found is the popup's own item, never
        // a target chip's): no change, nothing written.
        for entry in tactics.presets[0]
            .attack_instructions
            .as_mut()
            .expect("instructions")
            .iter_mut()
            .chain(
                tactics.presets[0]
                    .defence_instructions
                    .as_mut()
                    .expect("instructions")
                    .iter_mut(),
            )
        {
            entry.instruction = instruction::encode(version, Instruction::Off).expect("stored");
        }
        let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
        let same = entry_id(&ids, Role::Button, "Unassigned");
        let (_, changed) =
            window.action_click_tactics(same, version, &mut tactics, &mut view, &roster);
        assert!(!changed, "{version:?}: re-picking the value writes nothing");
        assert_eq!(tactics, before, "{version:?}: and edits nothing");

        // An out-of-range stored slider value shows as stored and is
        // not rewritten until the member moves it.
        let mut out_of_range = before.clone();
        out_of_range.presets[0].sliders.support_range = 0;
        let expected = out_of_range.clone();
        let (nodes, changed) =
            window.tactics(version, &mut out_of_range, &mut view, &roster, vec![]);
        assert!(
            !changed,
            "{version:?}: an out-of-range value is not rewritten"
        );
        assert_eq!(out_of_range, expected, "{version:?}: nor clamped on draw");
        let slider = control_below(&nodes, "Support range", Role::Slider, settings_half(&nodes));
        assert_eq!(
            slider.numeric_value(),
            Some(0.0),
            "{version:?}: the slider shows the stored value"
        );

        // A click on the open combo toggles it closed again.
        let (_, changed) = click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(!changed, "{version:?}: closing the combo writes nothing");
        assert_eq!(tactics, before, "{version:?}: and edits nothing");
    }
}

// A control's AccessKit id: the leftmost `role` on `label`'s row to its
// right (the Automatic settings' fields keep their labels to their
// left).
fn id_right_of(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
    role: Role,
) -> accesskit::NodeId {
    let label_bounds = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|(_, node)| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
    ids.iter()
        .filter(|(_, node)| {
            node.role() == role
                && node.bounds().is_some_and(|b| {
                    (b.y0 - label_bounds.y0).abs() < 4.0 && b.x0 >= label_bounds.x1 - 1.0
                })
        })
        .min_by(|a, b| {
            let x = |(_, node): &&(accesskit::NodeId, accesskit::Node)| {
                node.bounds().expect("bounds").x0
            };
            x(a).partial_cmp(&x(b)).expect("ordered bounds")
        })
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {role:?} right of {label:?}"))
}

// The Formation settings' half width: the right label column's x less
// the left's and the 16-point gap, from the twelve labels' bounds on
// `nodes`.
fn settings_half(nodes: &[accesskit::Node]) -> f64 {
    let (min, max) = FORMATION_LABELS
        .iter()
        .map(|label| formation_label_bounds(nodes, label).x0)
        .fold((f64::MAX, f64::MIN), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    max - min - 16.0
}

/// `settings_half` on `tactics_ids`' pairs.
fn ids_half(ids: &[(accesskit::NodeId, accesskit::Node)]) -> f64 {
    let nodes: Vec<accesskit::Node> = ids.iter().map(|(_, node)| node.clone()).collect();
    settings_half(&nodes)
}

// A Formation settings cell's control's AccessKit id: the `role` under
// `label`'s, in its column (the label over its control — a slider's or
// spinner's node has no label of its own).
fn id_below(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
    role: Role,
) -> accesskit::NodeId {
    let half = ids_half(ids);
    let label_bounds = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|(_, node)| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
    ids.iter()
        .filter(|(_, node)| {
            node.role() == role
                && node.bounds().is_some_and(|b| {
                    b.y0 > label_bounds.y0
                        && b.y0 <= label_bounds.y1 + 10.0
                        && b.x0 >= label_bounds.x0 - 1.0
                        && b.x0 <= label_bounds.x0 + half + 1.0
                })
        })
        .min_by(|a, b| {
            let y = |(_, node): &&(accesskit::NodeId, accesskit::Node)| {
                node.bounds().expect("bounds").y0
            };
            y(a).partial_cmp(&y(b)).expect("ordered bounds")
        })
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {role:?} below {label:?}"))
}

// The setting `label`'s `choice` button's id (the choices' texts
// repeat across pairs — "Wide" twice — so the column pins which).
fn button_below(
    ids: &[(accesskit::NodeId, accesskit::Node)],
    label: &str,
    choice: &str,
) -> accesskit::NodeId {
    let half = ids_half(ids);
    let label_bounds = ids
        .iter()
        .find(|(_, node)| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|(_, node)| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"));
    ids.iter()
        .find(|(_, node)| {
            node.role() == Role::Button
                && node.label() == Some(choice)
                && node.bounds().is_some_and(|b| {
                    b.y0 > label_bounds.y0
                        && b.y0 <= label_bounds.y1 + 10.0
                        && b.x0 >= label_bounds.x0 - 1.0
                        && b.x0 <= label_bounds.x0 + half + 1.0
                })
        })
        .map(|(id, _)| *id)
        .unwrap_or_else(|| panic!("no {choice:?} below {label:?}"))
}

/// How a case interacts: a `SetValue` on the slider's or the
/// spinner's node, a `Click` on a style pair's choice or a checkbox, a
/// combo's open and its item's pick.
#[derive(Clone, Copy)]
enum Control {
    /// Label and the value to set (the range's top).
    Slider(&'static str, f64),
    /// The pair's label and the choice's text.
    Choice(&'static str, &'static str),
    /// The checkbox's label (the click toggles it on).
    Check(&'static str),
    /// The combo's label and the item to pick.
    Combo(&'static str, &'static str),
}

// Every Formation settings and Automatic settings control writes its
// own field: one interaction on a fresh fixture, then the record is
// the fixture with that field alone set.
#[test]
fn each_settings_control_writes_its_own_field() {
    let mut window = Headless::sized(1400.0, 1100.0);
    let version = PesVersion::Pes21;
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    // One fixture parse for every case (each wants the fresh record).
    let base = fixture_tactics(version);
    /// One case: what it is about, how to interact, the fixture write
    /// before it and the expected write after.
    struct Case {
        /// The case's name (the assertion text).
        name: &'static str,
        /// The interaction.
        control: Control,
        /// Puts the field opposite the pick (a fresh fixture may
        /// already hold the target).
        setup: fn(&mut TeamTactics),
        /// Sets the named field, on a fixture clone.
        expect: fn(&mut TeamTactics),
    }
    let cases: &[Case] = &[
        Case {
            name: "Support range",
            control: Control::Slider("Support range", 10.0),
            setup: |t| t.presets[0].sliders.support_range = 1,
            expect: |t| t.presets[0].sliders.support_range = 10,
        },
        Case {
            name: "Defensive line",
            control: Control::Slider("Defensive line", 10.0),
            setup: |t| t.presets[0].sliders.defensive_line = 1,
            expect: |t| t.presets[0].sliders.defensive_line = 10,
        },
        Case {
            name: "Compactness",
            control: Control::Slider("Compactness", 10.0),
            setup: |t| t.presets[0].sliders.compactness = 1,
            expect: |t| t.presets[0].sliders.compactness = 10,
        },
        Case {
            name: "Numbers in attack",
            control: Control::Choice("Numbers in attack", "Many"),
            setup: |t| t.presets[0].sliders.numbers_in_attack = 1,
            expect: |t| t.presets[0].sliders.numbers_in_attack = 3,
        },
        Case {
            name: "Numbers in defence",
            control: Control::Choice("Numbers in defence", "Many"),
            setup: |t| t.presets[0].sliders.numbers_in_defence = 1,
            expect: |t| t.presets[0].sliders.numbers_in_defence = 3,
        },
        Case {
            name: "Attacking style on",
            control: Control::Choice("Attacking style", "Possession"),
            setup: |t| t.presets[0].style.attacking_style = false,
            expect: |t| t.presets[0].style.attacking_style = true,
        },
        Case {
            name: "Attacking style off",
            control: Control::Choice("Attacking style", "Counter Attack"),
            setup: |t| t.presets[0].style.attacking_style = true,
            expect: |t| t.presets[0].style.attacking_style = false,
        },
        Case {
            name: "Attacking zone on",
            control: Control::Choice("Attacking zone", "Wide"),
            setup: |t| t.presets[0].style.attacking_zone = false,
            expect: |t| t.presets[0].style.attacking_zone = true,
        },
        Case {
            name: "Attacking zone off",
            control: Control::Choice("Attacking zone", "Centre"),
            setup: |t| t.presets[0].style.attacking_zone = true,
            expect: |t| t.presets[0].style.attacking_zone = false,
        },
        Case {
            name: "Buildup on",
            control: Control::Choice("Buildup", "Short Pass"),
            setup: |t| t.presets[0].style.buildup = false,
            expect: |t| t.presets[0].style.buildup = true,
        },
        Case {
            name: "Buildup off",
            control: Control::Choice("Buildup", "Long Pass"),
            setup: |t| t.presets[0].style.buildup = true,
            expect: |t| t.presets[0].style.buildup = false,
        },
        Case {
            name: "Positioning on",
            control: Control::Choice("Positioning", "Flexible"),
            setup: |t| t.presets[0].style.positioning = false,
            expect: |t| t.presets[0].style.positioning = true,
        },
        Case {
            name: "Positioning off",
            control: Control::Choice("Positioning", "Maintain"),
            setup: |t| t.presets[0].style.positioning = true,
            expect: |t| t.presets[0].style.positioning = false,
        },
        Case {
            name: "Defensive style on",
            control: Control::Choice("Defensive style", "All Out Def."),
            setup: |t| t.presets[0].style.defensive_style = false,
            expect: |t| t.presets[0].style.defensive_style = true,
        },
        Case {
            name: "Defensive style off",
            control: Control::Choice("Defensive style", "Frontline Press."),
            setup: |t| t.presets[0].style.defensive_style = true,
            expect: |t| t.presets[0].style.defensive_style = false,
        },
        Case {
            name: "Containment area on",
            control: Control::Choice("Containment area", "Wide"),
            setup: |t| t.presets[0].style.containment_area = false,
            expect: |t| t.presets[0].style.containment_area = true,
        },
        Case {
            name: "Containment area off",
            control: Control::Choice("Containment area", "Middle"),
            setup: |t| t.presets[0].style.containment_area = true,
            expect: |t| t.presets[0].style.containment_area = false,
        },
        Case {
            name: "Pressure on",
            control: Control::Choice("Pressure", "Conservative"),
            setup: |t| t.presets[0].style.pressure = false,
            expect: |t| t.presets[0].style.pressure = true,
        },
        Case {
            name: "Pressure off",
            control: Control::Choice("Pressure", "Aggressive"),
            setup: |t| t.presets[0].style.pressure = true,
            expect: |t| t.presets[0].style.pressure = false,
        },
        Case {
            name: "Fluid formation",
            control: Control::Check("Fluid formation"),
            setup: |t| t.presets[0].style.fluid = Some(false),
            expect: |t| t.presets[0].style.fluid = Some(true),
        },
        Case {
            name: "Substitution",
            control: Control::Combo("Substitution", "Flexible"),
            setup: |t| t.auto.substitution = Some(0),
            expect: |t| t.auto.substitution = Some(2),
        },
        Case {
            name: "Offside trap",
            control: Control::Check("Offside trap"),
            setup: |t| t.auto.offside_trap = Some(false),
            expect: |t| t.auto.offside_trap = Some(true),
        },
        Case {
            name: "Preset change",
            control: Control::Check("Preset change"),
            setup: |t| t.auto.preset_change = Some(false),
            expect: |t| t.auto.preset_change = Some(true),
        },
        Case {
            name: "Attack/defence levels",
            control: Control::Check("Attack/defence levels"),
            setup: |t| t.auto.attack_defence_levels = Some(false),
            expect: |t| t.auto.attack_defence_levels = Some(true),
        },
    ];
    for case in cases {
        let mut tactics = base.clone();
        (case.setup)(&mut tactics);
        let mut expected = base.clone();
        (case.expect)(&mut expected);
        let mut view = TacticsView::default();
        let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
        let (id, action, data) = match case.control {
            Control::Slider(label, value) => (
                id_below(&ids, label, Role::Slider),
                accesskit::Action::SetValue,
                Some(accesskit::ActionData::NumericValue(value)),
            ),
            Control::Choice(label, choice) => (
                button_below(&ids, label, choice),
                accesskit::Action::Click,
                None,
            ),
            Control::Check(label) => (
                entry_id(&ids, Role::CheckBox, label),
                accesskit::Action::Click,
                None,
            ),
            // The combo opens first; the click is then on its item.
            Control::Combo(label, item) => {
                let combo = id_right_of(&ids, label, Role::ComboBox);
                let (_, _) =
                    window.action_click_tactics(combo, version, &mut tactics, &mut view, &roster);
                // The popup's first frame is a sizing pass; its items
                // join the tree on the next.
                let (ids, _) =
                    window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
                (
                    entry_id(&ids, Role::Button, item),
                    accesskit::Action::Click,
                    None,
                )
            }
        };
        let (_, changed) = window.tactics(
            version,
            &mut tactics,
            &mut view,
            &roster,
            vec![egui::Event::AccessKitActionRequest(
                accesskit::ActionRequest {
                    action,
                    target_tree: accesskit::TreeId::ROOT,
                    target_node: id,
                    data,
                },
            )],
        );
        let name = case.name;
        assert!(changed, "{name}: the interaction changed the record");
        assert_eq!(
            tactics, expected,
            "{name}: the one field written, the rest untouched"
        );
    }
}

/// The Formation settings' labels, the draft's row order (each
/// offensive setting, then its defensive pair).
const FORMATION_LABELS: [&str; 12] = [
    "Attacking style",
    "Defensive style",
    "Buildup",
    "Containment area",
    "Attacking zone",
    "Pressure",
    "Positioning",
    "Defensive line",
    "Support range",
    "Compactness",
    "Numbers in attack",
    "Numbers in defence",
];

/// The bounds of the Label node showing `label` (a cell's label).
fn formation_label_bounds(nodes: &[accesskit::Node], label: &str) -> accesskit::Rect {
    nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some(label))
        .and_then(|node| node.bounds())
        .unwrap_or_else(|| panic!("no label {label:?}"))
}

// Formation settings, the draft's `.tacrows`: the twelve settings form
// six rows of pairs — each offensive setting's label left, its defensive
// pair's right, the two halves 16 apart — the six rows equal in
// height and each cell's control centred 12 under its box's top (a
// toggle's centre level with its row partner's slider's).
#[test]
fn the_settings_are_six_even_rows_of_pairs() {
    // The settings column's width as the window's width: the draft's
    // 422 panel holds a 408-wide content.
    let mut window = Headless::sized(422.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, shapes) = window.frame(vec![], |ui| {
        tactics_settings(ui, "tactics", version, &mut tactics, &mut view, &roster);
    });

    // The panel's box: the smallest stroked rect holding the heading's
    // centre; its inner edge is the labels' left x (a uniform margin).
    let heading = formation_label_bounds(&nodes, "Formation settings");
    let heading_centre = egui::pos2(
        ((heading.x0 + heading.x1) / 2.0) as f32,
        ((heading.y0 + heading.y1) / 2.0) as f32,
    );
    let group = shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape)
                if !shape.stroke.is_empty() && shape.rect.contains(heading_centre) =>
            {
                Some(shape.rect)
            }
            _ => None,
        })
        .min_by(|a, b| a.area().partial_cmp(&b.area()).expect("areas"))
        .expect("the Formation settings box");
    let inner =
        group.shrink(formation_label_bounds(&nodes, "Attacking style").x0 as f32 - group.min.x);
    let half = (inner.width() - 16.0) / 2.0;

    let pairs: [(&str, &str); 6] = [
        ("Attacking style", "Defensive style"),
        ("Buildup", "Containment area"),
        ("Attacking zone", "Pressure"),
        ("Positioning", "Defensive line"),
        ("Support range", "Compactness"),
        ("Numbers in attack", "Numbers in defence"),
    ];
    let mut xs = std::collections::BTreeSet::new();
    let mut ys = Vec::new();
    for (left, right) in pairs {
        let left_bounds = formation_label_bounds(&nodes, left);
        let right_bounds = formation_label_bounds(&nodes, right);
        xs.insert(left_bounds.x0 as i64);
        xs.insert(right_bounds.x0 as i64);
        assert!(
            (left_bounds.y0 - right_bounds.y0).abs() < 0.5,
            "{left} and {right} share a row's y"
        );
        ys.push((left, left_bounds.y0));
        // The halves equal and 16 apart, the draft's `column-gap: 16px`.
        assert!(
            (right_bounds.x0 - left_bounds.x0 - (f64::from(half) + 16.0)).abs() <= 0.5,
            "{right} is the pair's right half, 16 past the left's ({:?} vs {:?})",
            right_bounds.x0,
            left_bounds.x0
        );
    }
    for row in ys.windows(2) {
        assert!(
            row[1].1 > row[0].1,
            "the rows run in the listed order ({} above {})",
            row[0].0,
            row[1].0
        );
    }
    assert_eq!(xs.len(), 2, "the twelve labels take two columns");
    // The rows start `TITLE_GAP` under the heading: the item gap after
    // it, topped up by the panel's `add_space`.
    assert!(
        (ys[0].1 - heading.y1 - f64::from(TITLE_GAP)).abs() < 0.5,
        "the first row starts TITLE_GAP ({TITLE_GAP}) under the heading ({} vs {})",
        ys[0].1,
        heading.y1
    );
    let pitch = (ys[5].1 - ys[0].1) / 5.0;
    for row in ys.windows(2) {
        assert!(
            (row[1].1 - row[0].1 - pitch).abs() < 0.5,
            "the rows are evenly spaced"
        );
    }
    assert!(
        (ys[5].1 + pitch - f64::from(inner.max.y)).abs() <= 0.5,
        "the last row's cell ends at the box's inner bottom ({} vs {:?})",
        ys[5].1 + pitch,
        inner.max.y
    );
    // Each label's control sits in the 24-point box 2 under it, centred:
    // the control's vertical centre is 14 under the label's bottom.
    let half = settings_half(&nodes);
    for label in FORMATION_LABELS {
        let bounds = formation_label_bounds(&nodes, label);
        let controls = controls_below(&nodes, label, Role::Button, half);
        let slider = controls_below(&nodes, label, Role::Slider, half);
        assert!(
            !controls.is_empty() || !slider.is_empty(),
            "{label} has a control"
        );
        for control in controls.into_iter().chain(slider) {
            let centre = {
                let b = control.bounds().expect("bounds");
                (b.y0 + b.y1) / 2.0
            };
            assert!(
                (centre - (bounds.y1 + 14.0)).abs() < 0.5,
                "{label}'s control is centred in the box 2 under it ({centre:?} vs {:?})",
                bounds.y1 + 14.0
            );
        }
    }
}

// Player assignments' and Advanced instructions' first rows start
// `TITLE_GAP` under their headings, as Formation settings' do (the PES
// 15/16 "(none on this version)" line too), and each assignment's label
// stands 4 points under the chip above it, the Join the attack label
// included.
#[test]
fn the_side_panels_rows_start_title_gap_under_the_headings() {
    let mut window = Headless::sized(800.0, 1200.0);
    let label_y = |nodes: &[accesskit::Node], text: &str| {
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(text))
            .and_then(|node| node.bounds())
            .unwrap_or_else(|| panic!("no label {text:?}"))
    };
    for version in [PesVersion::Pes21, PesVersion::Pes15] {
        let mut tactics = fixture_tactics(version);
        let mut view = TacticsView::default();
        let rows = fixture_rows(version);
        let roster = roster_rows(&rows);
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);

        // The first row of each side panel `TITLE_GAP` under its heading.
        let assignments = label_y(&nodes, "Player assignments");
        let first = label_y(&nodes, "Long free kick");
        assert!(
            (first.y0 - assignments.y1 - f64::from(TITLE_GAP)).abs() < 0.5,
            "{version:?}: the first assignment label {TITLE_GAP} under the heading \
             ({} vs {})",
            first.y0,
            assignments.y1
        );
        let instructions = label_y(&nodes, "Advanced instructions");
        let first_top = match version {
            PesVersion::Pes15 | PesVersion::Pes16 => label_y(&nodes, "(none on this version)").y0,
            // The header row is one line of text (the draft's header
            // cells are plain text), so its top is the text's.
            _ => label_y(&nodes, "Attack").y0,
        };
        assert!(
            (first_top - instructions.y1 - f64::from(TITLE_GAP)).abs() < 0.5,
            "{version:?}: the instructions' first row {TITLE_GAP} under the heading \
             ({first_top} vs {})",
            instructions.y1
        );

        // Each assignment label after the first 4 under the chip above:
        // the role labels in order, "Join the attack" last.
        for (label, above) in [
            ("Short free kick", "Long free kick"),
            ("Second free kick", "Short free kick"),
            ("Left corner", "Second free kick"),
            ("Right corner", "Left corner"),
            ("Penalty kick", "Right corner"),
            ("Captain", "Penalty kick"),
            ("Join the attack", "Captain"),
        ] {
            let label_y0 = label_y(&nodes, label).y0;
            let chip = chips_below(&nodes, above)
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("no chip under {above:?}"));
            let chip_y1 = chip.bounds().expect("bounds").y1;
            assert!(
                (label_y0 - chip_y1 - 4.0).abs() < 0.5,
                "{version:?}: {label} 4 under {above}'s chip ({label_y0} vs {chip_y1})"
            );
        }
    }
}

// The settings column is as narrow as the widest toggle choice allows.
// As the draft's `.tset .rate` draws it, a two-choice toggle spans its
// half, the two choices equal in width and a 1-point gap between them,
// the chosen in the selection fill; the defensive style's choices show
// the shortened texts (`STYLE_SHORT` — the column is sized for them),
// the full names the hover.
#[test]
fn the_two_choice_toggle_spans_its_half() {
    // The settings column's width as the window's width.
    let mut window = Headless::sized(422.0, 900.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let half = settings_half(&nodes);

    let choices = controls_below(&nodes, "Defensive style", Role::Button, half);
    assert_eq!(choices.len(), 2, "the two choices");
    let frontline = choices[0];
    let all_out = choices[1];
    assert_eq!(frontline.label(), Some("Frontline Press."));
    assert_eq!(all_out.label(), Some("All Out Def."));
    let width = |node: &accesskit::Node| {
        let b = node.bounds().expect("bounds");
        b.x1 - b.x0
    };
    assert!(
        (width(frontline) - width(all_out)).abs() < 0.5,
        "the choices equal in width"
    );
    assert!(
        (width(frontline) + width(all_out) + 1.0 - half).abs() < 1.0,
        "their sum plus the gap is the half's width"
    );
    // The chosen one shows selected — and the shortened text fits the
    // choice at this width, which the column is sized for.
    let stored = tactics.presets[0].style.defensive_style;
    assert_eq!(
        if stored { all_out } else { frontline }.toggled(),
        Some(accesskit::Toggled::True),
        "the stored choice is selected"
    );
    let mut shown = 0.0f32;
    window.frame(vec![], |ui| shown = text_width(ui, "Frontline Press."));
    assert!(
        f64::from(shown) <= width(frontline) + 0.5,
        "Frontline Press. fits its choice ({shown} vs {})",
        width(frontline)
    );
    // The hover gives the full name.
    assert_eq!(
        window.tactics_tooltip(
            centre(frontline),
            "Frontline Pressure",
            version,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("Frontline Pressure".to_owned()),
        "the shortened choice's hover is the full name"
    );
    // A click on the other choice writes the field.
    let (_, changed) = click(
        &mut window,
        all_out,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the click changed the record");
    assert!(
        tactics.presets[0].style.defensive_style,
        "All Out Defence stored"
    );
}

// The numbers in attack and defence are each a three-choice toggle,
// "Few | Medium | Many" (the draft's `.rate.tri`), rather than a slider
// whose number a member had to hover to read. The three choices are
// equal across the half and the stored 1 to 3 is picked. A stored value
// outside (a hand-edited save) selects none and writes nothing until a
// choice is clicked.
#[test]
fn the_numbers_are_three_choice_toggles() {
    let mut window = Headless::sized(422.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.presets[0].sliders.numbers_in_attack = 2;
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);

    // Stored 2 shows Medium chosen, the three equal across the half.
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let half = settings_half(&nodes);
    let choices = controls_below(&nodes, "Numbers in attack", Role::Button, half);
    assert_eq!(choices.len(), 3, "the three choices");
    let labels: Vec<Option<&str>> = choices.iter().map(|node| node.label()).collect();
    assert_eq!(
        labels,
        [Some("Few"), Some("Medium"), Some("Many")],
        "the choices' texts"
    );
    assert_eq!(
        choices[1].toggled(),
        Some(accesskit::Toggled::True),
        "Medium chosen for the stored 2"
    );
    let width = |node: &accesskit::Node| {
        let b = node.bounds().expect("bounds");
        b.x1 - b.x0
    };
    for pair in choices.windows(2) {
        assert!(
            (width(pair[0]) - width(pair[1])).abs() < 0.5,
            "the three choices equal in width"
        );
    }
    let total: f64 = choices.iter().map(|&node| width(node)).sum();
    assert!(
        (total + 2.0 - half).abs() < 1.0,
        "their sum plus the gaps is the half's width"
    );

    // Clicking Many stores 3.
    let (_, changed) = click(
        &mut window,
        choices[2],
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    assert!(changed, "the click changed the record");
    assert_eq!(
        tactics.presets[0].sliders.numbers_in_attack, 3,
        "Many stored"
    );

    // From the stored 3, Few stores 1 and then Medium 2, nothing else.
    for (choice, stored) in [(0, 1), (1, 2)] {
        assert_ne!(
            tactics.presets[0].sliders.numbers_in_attack, stored,
            "premise: a real change"
        );
        let mut expect = tactics.clone();
        expect.presets[0].sliders.numbers_in_attack = stored;
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let choices = controls_below(&nodes, "Numbers in attack", Role::Button, half);
        let (_, changed) = click(
            &mut window,
            choices[choice],
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        assert!(changed, "the click changed the record");
        assert_eq!(tactics, expect, "{stored} stored, nothing else");
    }

    // A stored value outside 1-3 selects none and is not rewritten.
    tactics.presets[0].sliders.numbers_in_attack = 7;
    let (nodes, changed) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let choices = controls_below(&nodes, "Numbers in attack", Role::Button, half);
    for node in &choices {
        assert_ne!(
            node.toggled(),
            Some(accesskit::Toggled::True),
            "no choice selected for a stored 7"
        );
    }
    assert!(!changed, "the frame reports no change");
    assert_eq!(
        tactics.presets[0].sliders.numbers_in_attack, 7,
        "the stored 7 is kept"
    );
}

// The armed mode draws no note: while join is armed the Player
// assignments panel holds no label beyond its own rows' and the chips' —
// the armed chips say it.
#[test]
fn the_armed_mode_draws_no_note() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);

    // The Player assignments panel's texts: its labels' values and the
    // chips' labels, sorted (the panel's region is its heading down to
    // Automatic settings').
    let panel_text = |nodes: &[accesskit::Node]| {
        let heading_y = |label: &str| {
            nodes
                .iter()
                .find(|node| node.role() == Role::Label && node.value() == Some(label))
                .and_then(|node| node.bounds())
                .unwrap_or_else(|| panic!("no {label:?} heading"))
                .y0
        };
        let (top, bottom) = (
            heading_y("Player assignments"),
            heading_y("Automatic settings"),
        );
        let mut texts: Vec<String> = nodes
            .iter()
            .filter(|node| node.bounds().is_some_and(|b| b.y0 >= top && b.y0 < bottom))
            .filter_map(|node| match node.role() {
                Role::Label => node.value().map(str::to_owned),
                Role::Button => node.label().map(str::to_owned),
                _ => None,
            })
            .collect();
        texts.sort();
        texts
    };
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let unarmed = panel_text(&nodes);

    // Arm join: the panel's texts are the unarmed frame's, exactly.
    view.assign = Some(Assign::Join);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    assert_eq!(
        panel_text(&nodes),
        unarmed,
        "armed: no label in the panel beyond the chips' own"
    );
}

/// A roster of named slots for `tactics_term` and the role chips: the
/// fixture's own names are all "PLACEHOLDER".
pub(crate) fn named_roster() -> Vec<(String, String, Option<u8>)> {
    (0..32)
        .map(|i| (format!("Player {:02}", i + 1), String::new(), None))
        .collect()
}

/// `tactics_term` words the preset fields with the tab's labels: the
/// sliders' numbers, the style pairs' texts, the fluid switch's
/// "off"/"on".
#[test]
fn the_preset_fields_terms() {
    let version = PesVersion::Pes21;
    let tactics = fixture_tactics(version);
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 0,
                field: PresetField::SupportRange,
            },
            5,
            7
        ),
        (
            "Preset 1 · Support range".to_owned(),
            "5".to_owned(),
            "7".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 0,
                field: PresetField::AttackingStyle,
            },
            0,
            1
        ),
        (
            "Preset 1 · Attacking style".to_owned(),
            "Counter Attack".to_owned(),
            "Possession".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 2,
                field: PresetField::DefensiveStyle,
            },
            1,
            0
        ),
        (
            "Preset 3 · Defensive style".to_owned(),
            "All Out Defence".to_owned(),
            "Frontline Pressure".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 1,
                field: PresetField::FluidFormation,
            },
            0,
            1
        ),
        (
            "Preset 2 · Fluid formation".to_owned(),
            "off".to_owned(),
            "on".to_owned()
        )
    );
    // The two numbers word as their three-choice toggles do.
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 0,
                field: PresetField::NumbersInAttack,
            },
            1,
            3
        ),
        (
            "Preset 1 · Numbers in attack".to_owned(),
            "Few".to_owned(),
            "Many".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Preset {
                preset: 0,
                field: PresetField::NumbersInDefence,
            },
            2,
            0
        ),
        (
            "Preset 1 · Numbers in defence".to_owned(),
            "Medium".to_owned(),
            "0".to_owned()
        )
    );
}

/// The set-piece takers' and the captain's terms: the chips' labels and
/// the holder's name, "Unassigned" for 255.
#[test]
fn the_set_pieces_terms() {
    let version = PesVersion::Pes21;
    let tactics = fixture_tactics(version);
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(TacticsField::FreeKickTakerLong, 3, 9),
        (
            "Long free kick".to_owned(),
            "Player 04".to_owned(),
            "Player 10".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::Captain, 0, 5),
        (
            "Captain".to_owned(),
            "Player 01".to_owned(),
            "Player 06".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::PenaltyTaker, 255, 2),
        (
            "Penalty kick".to_owned(),
            "Unassigned".to_owned(),
            "Player 03".to_owned()
        )
    );
}

/// A formation slot's terms name the formation and the starter playing
/// it — through `starting_eleven`, not the slot's index — and the part:
/// the position's label, the coordinates' numbers.
#[test]
fn the_formation_slots_terms() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // A non-identity eleven: wording slot 4's player as roster slot 4
    // would pass here too if `starting_eleven` were not read.
    tactics.starting_eleven[4] = 7;
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(
            TacticsField::Formation {
                preset: 0,
                formation: 0,
                slot: 4,
                part: FormationPart::Position,
            },
            9,
            10
        ),
        (
            "Preset 1 · Kick-off · Player 08 · Position".to_owned(),
            "LWF".to_owned(),
            "RWF".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Formation {
                preset: 1,
                formation: 2,
                slot: 4,
                part: FormationPart::X,
            },
            30,
            40
        ),
        (
            "Preset 2 · Out-of-possession · Player 08 · X".to_owned(),
            "30".to_owned(),
            "40".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Formation {
                preset: 0,
                formation: 1,
                slot: 4,
                part: FormationPart::Y,
            },
            20,
            44
        ),
        (
            "Preset 1 · In-possession · Player 08 · Y".to_owned(),
            "20".to_owned(),
            "44".to_owned()
        )
    );
}

/// A man-marking row names its own slot's starter and shows the marked
/// opponent's starter numbered from 1, "None" for 255.
#[test]
fn the_man_marking_terms() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.starting_eleven[5] = 2;
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(TacticsField::ManMarking { preset: 0, slot: 5 }, 255, 3),
        (
            "Preset 1 · Man marking · Player 03".to_owned(),
            "None".to_owned(),
            "Opponent's starter 4".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::ManMarking { preset: 0, slot: 5 }, 0, 255),
        (
            "Preset 1 · Man marking · Player 03".to_owned(),
            "Opponent's starter 1".to_owned(),
            "None".to_owned()
        )
    );
}

/// The instruction rows: the row label (`Attack 1` to `Defence 2`), the
/// instruction's text as the version's table decodes it (its hex past
/// the table), the target's combo text.
#[test]
fn the_instructions_terms() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    // The target bytes word under the instruction in force: a
    // player-taking one (Counter Target, Defensive) names them, the
    // fixture's Offs would read Unassigned.
    let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
    tactics.presets[0]
        .attack_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction = encode(Instruction::CounterTarget);
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction = encode(Instruction::Defensive);
    // A non-identity eleven: the byte is a roster slot, so an impl
    // printing it for the lineup index would look right on the
    // fixture's [0, 1, ...] (slot 5 sits at index 9 now).
    tactics.starting_eleven.swap(5, 9);
    assert_ne!(
        tactics.starting_eleven[5], 5,
        "premise: slot and index differ"
    );
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(
            TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Attack,
                index: 0,
                part: InstructionPart::Instruction,
            },
            6,
            12
        ),
        (
            "Preset 1 · Attack 1".to_owned(),
            "Tiki-Taka".to_owned(),
            "Deep Defensive Line".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Defence,
                index: 1,
                part: InstructionPart::Instruction,
            },
            6,
            0xAB
        ),
        (
            "Preset 1 · Defence 2".to_owned(),
            "Tiki-Taka".to_owned(),
            "Unknown (0xAB)".to_owned()
        )
    );
    assert_eq!(
        term(
            TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Attack,
                index: 0,
                part: InstructionPart::PlayerId,
            },
            5,
            255
        ),
        (
            "Preset 1 · Attack 1 · Target".to_owned(),
            "Player 06".to_owned(),
            "Unassigned".to_owned()
        )
    );
    // A target that is no starter names the roster slot alone.
    assert_eq!(
        term(
            TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Defence,
                index: 0,
                part: InstructionPart::PlayerId,
            },
            20,
            5
        ),
        (
            "Preset 1 · Defence 1 · Target".to_owned(),
            "Player 21".to_owned(),
            "Player 06".to_owned()
        )
    );
}

/// The remaining terms: the automatic settings, the lists and the join
/// pick, the tactics record's team id.
#[test]
fn the_rest_of_the_terms() {
    let version = PesVersion::Pes21;
    let tactics = fixture_tactics(version);
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(TacticsField::AutoSubstitution, 0, 3),
        (
            "Substitution".to_owned(),
            "Off".to_owned(),
            "Very early".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::AutoAttackDefenceLevels, 0, 1),
        (
            "Attack/defence levels".to_owned(),
            "off".to_owned(),
            "on".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::Starting(4), 4, 7),
        (
            "Starting eleven · 5".to_owned(),
            "Player 05".to_owned(),
            "Player 08".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::Bench(0), 11, 20),
        (
            "Bench · 1".to_owned(),
            "Player 12".to_owned(),
            "Player 21".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::PlayerToJoinAttack(0), 3, 255),
        (
            "Join attack · 1".to_owned(),
            "Player 04".to_owned(),
            "Unassigned".to_owned()
        )
    );
    assert_eq!(
        term(TacticsField::TeamId, 701, 702),
        ("Team ID".to_owned(), "701".to_owned(), "702".to_owned())
    );
}

/// Every `field:` key names the row its own entry's label, written out
/// as literals: a `SliderSpec`, `StyleToggle` or `RoleSpec` whose key
/// drifted — two keys swapped included — draws its row under another
/// entry's label, which an expected label read from the same table
/// cannot catch.
#[test]
fn every_table_entry_keys_its_own_label() {
    let version = PesVersion::Pes21;
    let tactics = fixture_tactics(version);
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let label =
        |field| tactics_term(version, field, 0, 0, &tactics, &tactics, (&roster, &roster)).0;
    let preset = |field| TacticsField::Preset { preset: 0, field };
    let cases: &[(TacticsField, &str)] = &[
        (
            preset(PresetField::SupportRange),
            "Preset 1 · Support range",
        ),
        (
            preset(PresetField::DefensiveLine),
            "Preset 1 · Defensive line",
        ),
        (preset(PresetField::Compactness), "Preset 1 · Compactness"),
        (
            preset(PresetField::NumbersInAttack),
            "Preset 1 · Numbers in attack",
        ),
        (
            preset(PresetField::NumbersInDefence),
            "Preset 1 · Numbers in defence",
        ),
        (
            preset(PresetField::AttackingStyle),
            "Preset 1 · Attacking style",
        ),
        (
            preset(PresetField::AttackingZone),
            "Preset 1 · Attacking zone",
        ),
        (preset(PresetField::Buildup), "Preset 1 · Buildup"),
        (preset(PresetField::Positioning), "Preset 1 · Positioning"),
        (
            preset(PresetField::DefensiveStyle),
            "Preset 1 · Defensive style",
        ),
        (
            preset(PresetField::ContainmentArea),
            "Preset 1 · Containment area",
        ),
        (preset(PresetField::Pressure), "Preset 1 · Pressure"),
        (TacticsField::FreeKickTakerLong, "Long free kick"),
        (TacticsField::FreeKickTakerShort, "Short free kick"),
        (TacticsField::FreeKickTakerSecond, "Second free kick"),
        (TacticsField::CornerTakerLeft, "Left corner"),
        (TacticsField::CornerTakerRight, "Right corner"),
        (TacticsField::PenaltyTaker, "Penalty kick"),
        (TacticsField::Captain, "Captain"),
    ];
    assert_eq!(
        cases.len(),
        SLIDERS.len() + STYLE_TOGGLES.len() + assignments::SET_PIECES.len(),
        "a literal pair per table entry"
    );
    for (field, expected) in cases {
        assert_eq!(label(*field), *expected, "{field:?}");
    }
}

/// The `PlayerId` row words its target byte under the instruction in
/// force on that side, as the tab's target chip does: "An opponent"
/// under Tight Marking, "No target" under a target-less one, a player
/// under a player-taking one.
#[test]
fn the_target_terms_read_the_instruction_in_force_on_each_side() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
    {
        let attack = tactics.presets[0]
            .attack_instructions
            .as_mut()
            .expect("PES 21 has instructions");
        attack[0].instruction = encode(Instruction::TightMarking);
        attack[0].player = 4;
        attack[1].instruction = encode(Instruction::TikiTaka);
        attack[1].player = 0;
        let defence = tactics.presets[0]
            .defence_instructions
            .as_mut()
            .expect("PES 21 has instructions");
        defence[0].instruction = encode(Instruction::CounterTarget);
        // A non-identity eleven: the byte is a roster slot, so an impl
        // printing it for the lineup index would look right on the
        // fixture's [0, 1, ...].
        tactics.starting_eleven.swap(5, 9);
        defence[0].player = tactics.starting_eleven[5];
        assert_ne!(
            tactics.starting_eleven[5], 5,
            "premise: slot and index differ"
        );
    }
    // The tab's own text for the same tactics, drawn — each chip's
    // label under its combo.
    let mut window = Headless::sized(800.0, 1200.0);
    let mut view = TacticsView::default();
    let rows = named_roster();
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let chip_text = |column, index| {
        chip_under(&nodes, combo_row(&nodes, index)[column])
            .label()
            .expect("the chip's text")
            .to_owned()
    };
    assert_eq!(chip_text(0, 0), "An opponent", "the tab's own text");
    assert_eq!(chip_text(0, 1), "No target", "the tab's own text");
    let starter_text = chip_text(1, 0);
    assert_eq!(
        starter_text,
        name_at(&roster, tactics.starting_eleven[5]).to_owned(),
        "the tab's own text: the name alone"
    );

    // The row's sides read the same words for the same instruction and
    // byte.
    let term = |side, index, old, new| {
        tactics_term(
            version,
            TacticsField::Instruction {
                preset: 0,
                side,
                index,
                part: InstructionPart::PlayerId,
            },
            old,
            new,
            &tactics,
            &tactics,
            (&roster, &roster),
        )
    };
    assert_eq!(
        term(InstructionSide::Attack, 0, 4, 255).1,
        "An opponent",
        "the row's Tight Marking side"
    );
    assert_eq!(
        term(InstructionSide::Attack, 1, 0, 9).1,
        "No target",
        "the row's target-less side"
    );
    assert_eq!(
        term(
            InstructionSide::Defence,
            0,
            u32::from(tactics.starting_eleven[5]),
            20
        )
        .1,
        starter_text,
        "the row's player-taking side"
    );

    // Each side of the row reads its own instruction: the same byte under
    // the baseline's Tight Marking and the current's target-less one.
    let baseline = tactics.clone();
    let mut current = tactics.clone();
    current.presets[0]
        .attack_instructions
        .as_mut()
        .expect("instructions")[0]
        .instruction = encode(Instruction::TikiTaka);
    let (_, old, new) = tactics_term(
        version,
        TacticsField::Instruction {
            preset: 0,
            side: InstructionSide::Attack,
            index: 0,
            part: InstructionPart::PlayerId,
        },
        4,
        4,
        &baseline,
        &current,
        (&roster, &roster),
    );
    assert_eq!(
        (old.as_str(), new.as_str()),
        ("An opponent", "No target"),
        "each side under its own instruction"
    );
}

// Every slot-valued term words its old side through the first roster
// and its new side and its label through the second: after a drag the
// two rosters name one slot differently, and a side read through the
// other roster names the wrong player.
#[test]
fn each_side_names_its_slots_through_its_own_roster() {
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.presets[0]
        .defence_instructions
        .as_mut()
        .expect("PES 21 has instructions")[0]
        .instruction =
        instruction::encode(version, Instruction::CounterTarget).expect("PES 21 stores it");
    // A non-identity eleven: the byte is a roster slot, so an impl
    // printing it for the lineup index would look right on the
    // fixture's [0, 1, ...].
    tactics.starting_eleven.swap(5, 9);
    assert_ne!(
        tactics.starting_eleven[5], 5,
        "premise: slot and index differ"
    );
    let old_rows: Vec<(String, String, Option<u8>)> = (0..32)
        .map(|i| (format!("Old {:02}", i + 1), String::new(), None))
        .collect();
    let new_rows: Vec<(String, String, Option<u8>)> = (0..32)
        .map(|i| (format!("New {:02}", i + 1), String::new(), None))
        .collect();
    let (old_roster, new_roster) = (roster_rows(&old_rows), roster_rows(&new_rows));
    let term = |field, old, new| {
        tactics_term(
            version,
            field,
            old,
            new,
            &tactics,
            &tactics,
            (&old_roster, &new_roster),
        )
    };
    let sides = |field, old, new| {
        let (_, old, new) = term(field, old, new);
        (old, new)
    };
    let pair = |old: &str, new: &str| (old.to_owned(), new.to_owned());
    assert_eq!(
        sides(TacticsField::Starting(0), 2, 3),
        pair("Old 03", "New 04")
    );
    assert_eq!(
        sides(TacticsField::Bench(0), 12, 13),
        pair("Old 13", "New 14")
    );
    assert_eq!(
        sides(TacticsField::PlayerToJoinAttack(0), 4, 5),
        pair("Old 05", "New 06")
    );
    assert_eq!(sides(TacticsField::Captain, 0, 1), pair("Old 01", "New 02"));
    assert_eq!(
        sides(TacticsField::PenaltyTaker, 6, 6),
        pair("Old 07", "New 07")
    );
    let starter = tactics.starting_eleven[5];
    assert_eq!(
        sides(
            TacticsField::Instruction {
                preset: 0,
                side: InstructionSide::Defence,
                index: 0,
                part: InstructionPart::PlayerId,
            },
            u32::from(starter),
            u32::from(starter),
        ),
        (
            old_rows[usize::from(starter)].0.clone(),
            new_rows[usize::from(starter)].0.clone()
        )
    );
    // The labels name the slot's starter as the tab shows him now.
    let starter_new = new_rows[usize::from(tactics.starting_eleven[4])].0.clone();
    let (label, _, _) = term(
        TacticsField::Formation {
            preset: 0,
            formation: 0,
            slot: 4,
            part: FormationPart::X,
        },
        1,
        2,
    );
    assert_eq!(label, format!("Preset 1 · Kick-off · {starter_new} · X"));
    let (label, _, _) = term(TacticsField::ManMarking { preset: 0, slot: 4 }, 255, 3);
    assert_eq!(label, format!("Preset 1 · Man marking · {starter_new}"));
}

// The draft's `.tacauto` grid: the Automatic settings' four controls
// form two equal columns of 24-point rows — the right column's left edge
// 16 plus a half over the left's — the two rows 24 apart.
#[test]
fn the_automatic_settings_are_a_two_column_grid() {
    let mut window = Headless::sized(422.0, 900.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);

    // The two halves equal, 16 apart: the left column's edge is the
    // Substitution label's and the left checkboxes', the right's the
    // right checkboxes' (the combo sits inside its cell beside the
    // label). The columns' width is the panel's inner less 16 halved
    // (the frame's margin and stroke are 7 a side).
    let inner = 422.0 - 14.0;
    let half = (inner - 16.0) / 2.0;
    let substitution_label = nodes
        .iter()
        .find(|node| node.role() == Role::Label && node.value() == Some("Substitution"))
        .and_then(|node| node.bounds())
        .expect("the Substitution label");
    let checkboxes = [
        entry(&nodes, Role::CheckBox, "Preset change"),
        entry(&nodes, Role::CheckBox, "Offside trap"),
        entry(&nodes, Role::CheckBox, "Attack/defence levels"),
    ];
    let mut xs: Vec<f64> = vec![substitution_label.x0];
    let mut centres = Vec::new();
    for node in checkboxes {
        let bounds = node.bounds().expect("checkbox bounds");
        xs.push(bounds.x0);
        centres.push((bounds.y0 + bounds.y1) / 2.0);
    }
    let combo = control_right_of(&nodes, "Substitution", Role::ComboBox);
    centres.push({
        let b = combo.bounds().expect("combo bounds");
        (b.y0 + b.y1) / 2.0
    });
    xs.sort_by(|a, b| a.partial_cmp(b).expect("ordered"));
    xs.dedup_by(|a, b| (*b - *a).abs() < 1.0);
    assert_eq!(xs.len(), 2, "the controls take two columns ({xs:?})");
    assert!(
        (xs[1] - xs[0] - (half + 16.0)).abs() <= 1.0,
        "the columns' x a half and 16 apart ({xs:?}, half {half})"
    );
    centres.sort_by(|a, b| a.partial_cmp(b).expect("ordered"));
    centres.dedup_by(|a, b| (*b - *a).abs() < 0.5);
    assert_eq!(centres.len(), 2, "the controls take two rows");
    assert!(
        (centres[1] - centres[0] - 24.0).abs() <= 0.5,
        "the rows' centres are 24 apart ({centres:?})"
    );

    // The `.tacauto` pairing: Preset change on Substitution's row,
    // Offside trap on Attack/defence levels'.
    let centre_y = |node: &accesskit::Node| {
        let b = node.bounds().expect("bounds");
        (b.y0 + b.y1) / 2.0
    };
    let combo_centre = centre_y(combo);
    let [preset_change, offside_trap, levels] = [
        entry(&nodes, Role::CheckBox, "Preset change"),
        entry(&nodes, Role::CheckBox, "Offside trap"),
        entry(&nodes, Role::CheckBox, "Attack/defence levels"),
    ];
    assert!(
        (centre_y(preset_change) - combo_centre).abs() <= 0.5,
        "Preset change shares Substitution's row"
    );
    assert!(
        (centre_y(offside_trap) - centre_y(levels)).abs() <= 0.5,
        "Offside trap shares Attack/defence levels' row"
    );
    let x0 = |node: &accesskit::Node| node.bounds().expect("bounds").x0;
    assert!(
        (x0(preset_change) - xs[1]).abs() <= 1.0,
        "Preset change is Substitution's row's right column"
    );
    assert!(
        (x0(offside_trap) - xs[0]).abs() <= 1.0,
        "Offside trap is the second row's left column"
    );
    assert!(
        (x0(levels) - xs[1]).abs() <= 1.0,
        "Attack/defence levels is its right"
    );
}

/// The buttons `after` draws that `before` did not (no button with the
/// same label and bounds), in draw order top to bottom: the items an
/// opened combo's popup added to the tree. Labels alone cannot pick
/// them out — the popup's "Flexible" is also a Positioning choice.
fn popup_items<'a>(
    before: &[accesskit::Node],
    after: &'a [accesskit::Node],
) -> Vec<&'a accesskit::Node> {
    let mut items: Vec<&accesskit::Node> = after
        .iter()
        .filter(|node| {
            node.role() == Role::Button
                && node.bounds().is_some()
                && !before.iter().any(|old| {
                    old.role() == node.role()
                        && old.label() == node.label()
                        && old.bounds() == node.bounds()
                })
        })
        .collect();
    items.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("item bounds");
            (bounds.y0, bounds.x0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    items
}

// The Substitution combo names the game's choices — Off, Very late and
// Flexible on every storing version (the field is PES 15+, so PES
// 2016's cell is not gated), Very early from PES 2017 on — in the
// stored order.
#[test]
fn the_substitution_combo_names_the_games_choices() {
    let mut window = Headless::sized(800.0, 1200.0);
    for (version, very_early) in [
        (PesVersion::Pes16, false),
        (PesVersion::Pes17, true),
        (PesVersion::Pes18, true),
        (PesVersion::Pes19, true),
        (PesVersion::Pes20, true),
        (PesVersion::Pes21, true),
    ] {
        let mut tactics = fixture_tactics(version);
        // A stored 0: the offered set is the version's own, no number
        // appended.
        tactics.auto.substitution = Some(0);
        let mut view = TacticsView::default();
        let rows = fixture_rows(version);
        let roster = roster_rows(&rows);
        let (before, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let combo = control_right_of(&before, "Substitution", Role::ComboBox);
        assert_eq!(combo.value(), Some("Off"), "{version:?}: the stored 0");
        let (_, _) = click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
        // The popup's first frame is a sizing pass; its items join the
        // tree on the next.
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let mut expected = vec![Some("Off"), Some("Very late"), Some("Flexible")];
        if very_early {
            expected.push(Some("Very early"));
        }
        let items: Vec<Option<&str>> = popup_items(&before, &nodes)
            .iter()
            .map(|node| node.label())
            .collect();
        assert_eq!(items, expected, "{version:?}'s choices");
        // Close the popup for the next version's frame (a click on the
        // combo toggles it).
        let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
        let combo = control_right_of(&nodes, "Substitution", Role::ComboBox);
        let (_, _) = click(
            &mut window,
            combo,
            version,
            &mut tactics,
            &mut view,
            &roster,
        );
    }
}

// The same sentence's tail: a stored value outside the offered choices
// shows as its number — the combo never misreports the save — and a
// frame with no gesture keeps it.
#[test]
fn an_out_of_range_substitution_shows_its_number_and_stays() {
    let mut window = Headless::sized(800.0, 1200.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.auto.substitution = Some(7);
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let combo = control_right_of(&nodes, "Substitution", Role::ComboBox);
    assert_eq!(combo.value(), Some("7"), "the stored 7 shows as its number");
    let (_, changed) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    assert!(!changed, "a frame with no gesture writes nothing");
    assert_eq!(tactics.auto.substitution, Some(7), "the stored 7 stays");

    // The appended "stored as its number" row's popup index is no byte:
    // picking it writes the stored byte again — an impl writing the
    // index would land 4. The popup's items go through AccessKit, as
    // the instruction combos' do.
    let (_, _) = click(
        &mut window,
        combo,
        version,
        &mut tactics,
        &mut view,
        &roster,
    );
    let (ids, _) = window.tactics_ids(version, &mut tactics, &mut view, &roster, vec![]);
    let item = entry_id(&ids, Role::Button, "7");
    let (_, changed) = window.action_click_tactics(item, version, &mut tactics, &mut view, &roster);
    assert!(!changed, "picking the stored row writes nothing");
    assert_eq!(tactics.auto.substitution, Some(7), "the byte stays");
}

// A record that stores no substitution byte keeps the disabled cell
// with the `PES NN+` tooltip (the `None` arm — no shipped version
// lacks the field, so the tactics record is edited to it).
#[test]
fn a_version_without_substitution_keeps_the_disabled_cell() {
    let mut window = Headless::sized(800.0, 1200.0);
    window
        .ctx
        .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
    let version = PesVersion::Pes21;
    let mut tactics = fixture_tactics(version);
    tactics.auto.substitution = None;
    let mut view = TacticsView::default();
    let rows = fixture_rows(version);
    let roster = roster_rows(&rows);
    let (nodes, _) = window.tactics(version, &mut tactics, &mut view, &roster, vec![]);
    let dash = control_right_of(&nodes, "Substitution", Role::Button);
    assert!(dash.is_disabled(), "the cell's button is disabled");
    assert_eq!(dash.label(), Some("-"), "the disabled dash");
    assert_eq!(
        window.tactics_tooltip(
            centre(dash),
            "PES ",
            version,
            &mut tactics,
            &mut view,
            &roster,
        ),
        Some("PES 15+".to_owned()),
        "the gate tooltip names the first version storing the field"
    );
}

// `Medal::from_tier_id` reads the three tier ids every ruleset shares,
// and nothing else.
#[test]
fn from_tier_id_maps_the_three_medal_ids() {
    assert_eq!(Medal::from_tier_id("gold"), Some(Medal::Gold));
    assert_eq!(Medal::from_tier_id("silver"), Some(Medal::Silver));
    assert_eq!(Medal::from_tier_id("bronze"), Some(Medal::Bronze));
    for id in ["regular", "goalkeeper", "Gold", "", "goldx"] {
        assert_eq!(Medal::from_tier_id(id), None, "{id:?} is no medal");
    }
}

// A slider at a mid value paints the rail's fill from the rail's left
// to the handle's centre — egui's own overhang of the widgets' corner
// radius tucking the fill under the handle — and the handle itself in
// the selection blue, the inactive state's stroke kept.
#[test]
fn a_slider_fills_the_rail_to_the_handle_and_paints_the_handle_blue() {
    let mut window = Headless::sized(800.0, 600.0);
    let (row_height, visuals, item_gap, value_width) = {
        let style = window.ctx.style_of(window.ctx.theme());
        (
            style.spacing.interact_size.y,
            style.visuals.clone(),
            style.spacing.item_spacing.x,
            style.spacing.interact_size.x,
        )
    };
    let selection = visuals.selection.bg_fill;
    let inactive = visuals.widgets.inactive;
    let mut sliders = fixture_tactics(PesVersion::Pes21).presets[0]
        .sliders
        .clone();
    sliders.support_range = 5;
    let spec = SLIDERS
        .iter()
        .find(|spec| spec.label == "Support range")
        .expect("the Support range spec");
    let (nodes, shapes) = window.frame(vec![], |ui| slider(ui, &mut sliders, spec));

    // The rail widget's rect is the slider's node, `row_height` tall;
    // the handle's centre travels each end's `radius * 0.75` inset.
    let widget = nodes
        .iter()
        .find(|node| node.role() == Role::Slider)
        .and_then(|node| node.bounds())
        .expect("the slider's node");
    assert_eq!(
        widget.y1 - widget.y0,
        f64::from(row_height),
        "the slider's row"
    );
    // The rail is the cell's width less the value box's and their gap;
    // the value box ends at the cell's right edge.
    assert!(
        (widget.x1 - widget.x0 - (800.0 - f64::from(item_gap) - f64::from(value_width))).abs()
            <= 0.5,
        "the rail fills the cell less the value box and the gap: {widget:?}"
    );
    let value_box = nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton)
        .and_then(|node| node.bounds())
        .expect("the value box's node");
    assert!(
        (value_box.x1 - 800.0).abs() <= 0.5,
        "the value box ends at the cell's right edge: {value_box:?}"
    );
    let radius = f64::from(row_height) / 2.5;
    let inset = radius * 0.75;
    let centre_x = widget.x0 + inset + (4.0 / 9.0) * (widget.x1 - widget.x0 - 2.0 * inset);

    let selection_rects = || {
        shapes.iter().filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) if shape.fill == selection => Some(shape),
            _ => None,
        })
    };
    let fill = selection_rects()
        .map(|shape| shape.rect)
        .find(|rect| (f64::from(rect.height()) - 8.0).abs() < 1.0)
        .expect("the rail's fill in the selection blue");
    assert!(
        (f64::from(fill.min.x) - widget.x0).abs() <= 1.0,
        "the fill starts at the rail's left ({:?} vs {widget:?})",
        fill.min.x
    );
    let overhang = f64::from(inactive.corner_radius.nw);
    assert!(
        (f64::from(fill.max.x) - centre_x - overhang).abs() <= 1.0,
        "the fill reaches the handle's centre plus the corner overhang ({:?} vs {centre_x})",
        fill.max.x
    );

    // The 18-point row's 10.8 x 14.4 handle, filled the selection blue,
    // centred on the value's x, the inactive stroke kept.
    let handle = selection_rects()
        .find(|shape| {
            (f64::from(shape.rect.width()) - 2.0 * inset).abs() < 0.5
                && (f64::from(shape.rect.height()) - 2.0 * radius).abs() < 0.5
        })
        .expect("the handle in the selection blue");
    assert!(
        (f64::from(handle.rect.center().x) - centre_x).abs() <= 0.5,
        "the handle centred on the value's x ({:?} vs {centre_x})",
        handle.rect.center().x
    );
    assert!(
        (f64::from(handle.rect.center().y) - (widget.y0 + widget.y1) / 2.0).abs() <= 0.5,
        "the handle centred on the rail"
    );
    assert_eq!(
        handle.stroke, inactive.fg_stroke,
        "the inactive stroke kept"
    );

    // The handle's centre at the range's ends: the rail's edge
    // `radius * 0.75` in (at the mid value the inset moves it by too
    // little to measure).
    let handle_centre = |shapes: &[egui::epaint::ClippedShape]| {
        f64::from(
            shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(shape) if shape.fill == selection => Some(shape.rect),
                    _ => None,
                })
                .find(|rect| {
                    (f64::from(rect.width()) - 2.0 * inset).abs() < 0.5
                        && (f64::from(rect.height()) - 2.0 * radius).abs() < 0.5
                })
                .expect("the handle in the selection blue")
                .center()
                .x,
        )
    };
    for (value, edge, expect_x) in [
        (1u8, "left", widget.x0 + inset),
        (spec.max, "right", widget.x1 - inset),
    ] {
        sliders.support_range = value;
        let (_, shapes) = window.frame(vec![], |ui| slider(ui, &mut sliders, spec));
        assert!(
            (handle_centre(&shapes) - expect_x).abs() <= 0.5,
            "at {value} the handle's centre is the rail's {edge} plus the inset"
        );
    }
}
