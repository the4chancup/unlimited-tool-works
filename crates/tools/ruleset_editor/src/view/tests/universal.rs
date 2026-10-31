//! The Universal rules section (RE-GUI-20): Blank's rules edited through
//! the page and saved, an unticked Age undone, DBC7's values drawn; each
//! control bound to its field alone, and a stored value past a control's
//! range drawn as it is.

use aatf::{Ruleset, Universal, WeightRule};

use super::tiers::{click_role, type_replacing};
use super::*;

/// The page's checkboxes, top to bottom then left to right.
const CHECKBOXES: [&str; 12] = [
    "Every player is rated A at his registered position",
    "1",
    "2",
    "3",
    "No B ratings",
    "B ratings count toward the A-position allowance",
    "GK is never a second A",
    "Age",
    "Weight",
    "A captain is required",
    "At most",
    "Illegal formations allowed (Whack Patch)",
];

/// A fresh context, a window tall enough for the whole page and a view
/// with `ruleset` opened on Universal rules.
fn universal_view(
    ruleset: Ruleset,
) -> (
    ToolContext,
    Receiver<PipelineEventEnvelope>,
    Headless,
    RulesetView,
) {
    let (ctx, events, _window, mut view) = fresh();
    open_document(&mut view, ruleset);
    view.section = Section::Universal;
    (ctx, events, Headless::sized(1200.0, 900.0), view)
}

/// The document's `universal`.
fn universal(view: &RulesetView) -> &Universal {
    &view
        .document
        .as_ref()
        .expect("a document")
        .ruleset
        .universal
}

/// A click on the page's checkbox labeled `label`.
fn tick(window: &mut Headless, view: &mut RulesetView, ctx: &ToolContext, label: &str) {
    click_role(window, view, ctx, Role::CheckBox, label);
}

/// The number control showing `value` set to `text`: a click to type,
/// the text, then Enter.
fn set_number(
    window: &mut Headless,
    view: &mut RulesetView,
    ctx: &ToolContext,
    value: &str,
    text: &str,
) {
    let frame = settled(window, view, ctx);
    click(window, spin_button(&frame.nodes, value), view, ctx);
    type_replacing(window, view, ctx, text);
    window.frame(vec![key(egui::Key::Enter, egui::Modifiers::NONE)], |ui| {
        show(ui, view, ctx)
    });
}

/// The number control showing `value`.
fn spin_button<'a>(nodes: &'a [accesskit::Node], value: &str) -> &'a accesskit::Node {
    nodes
        .iter()
        .find(|node| node.role() == Role::SpinButton && node.value() == Some(value))
        .unwrap_or_else(|| panic!("a number control showing {value:?}"))
}

/// Each page checkbox's label and whether it is drawn ticked, in
/// `CHECKBOXES`' order.
fn ticked(nodes: &[accesskit::Node]) -> Vec<(&'static str, bool)> {
    CHECKBOXES
        .iter()
        .map(|&label| {
            let checkbox = nodes
                .iter()
                .find(|node| node.role() == Role::CheckBox && node.label() == Some(label))
                .unwrap_or_else(|| panic!("a checkbox labeled {label:?}"));
            (label, checkbox.toggled() == Some(accesskit::Toggled::True))
        })
        .collect()
}

/// The `universal` of the rules file at `path`, loaded as `aatf` loads
/// it.
fn saved_universal(path: &Path) -> Universal {
    let text = std::fs::read_to_string(path).expect("the file was written");
    aatf::Rules::load(&text)
        .expect("the saved file loads")
        .ruleset()
        .universal
        .clone()
}

// RE-GUI-20
#[test]
fn blank_edited_on_the_page_saves_its_universal_and_age_comes_back_on_undo() {
    let dir = crate::test_support::scratch("universal_20");
    let path = dir.join("universal.4ccrules");
    let (ctx, _events, mut window, mut view) = universal_view(crate::document::blank());
    tick(&mut window, &mut view, &ctx, "3");
    tick(&mut window, &mut view, &ctx, "1");
    tick(&mut window, &mut view, &ctx, "Age");
    tick(&mut window, &mut view, &ctx, "Weight");
    tick(&mut window, &mut view, &ctx, "At most");
    set_number(&mut window, &mut view, &ctx, "5", "3");
    tick(
        &mut window,
        &mut view,
        &ctx,
        "Illegal formations allowed (Whack Patch)",
    );
    tick(&mut window, &mut view, &ctx, "A captain is required");
    assert!(save_to(&mut view, &ctx, &path));

    let mut expected = crate::document::blank().universal;
    expected.fielded_at_a = vec![1, 3];
    expected.age = Some((15, 50));
    expected.weight = Some(WeightRule {
        floor: 30,
        below: 129,
        above: 81,
    });
    expected.max_com_styles = Some(3);
    expected.illegal_formations = true;
    expected.captain_required = false;
    assert_eq!(saved_universal(&path), expected);

    tick(&mut window, &mut view, &ctx, "Age");
    assert!(save_to(&mut view, &ctx, &path));
    assert_eq!(saved_universal(&path).age, None, "Age unticked");
    ctrl_key(&mut window, &mut view, &ctx, egui::Key::Z);
    assert_eq!(universal(&view).age, Some((15, 50)), "Undo brings it back");
}

// RE-GUI-20
#[test]
fn dbc7_draws_its_skill_cards_and_only_illegal_formations_ticked() {
    let dbc7 = aatf::builtin("dbc7").expect("DBC7 is built in");
    let (ctx, _events, mut window, mut view) = universal_view(dbc7.ruleset().clone());
    let frame = settled(&mut window, &mut view, &ctx);
    let skill_cards = spin_button(&frame.nodes, "41").bounds().expect("drawn");
    let words = node(&frame.nodes, "skill cards per player")
        .bounds()
        .expect("drawn");
    assert!(
        (skill_cards.y0 - words.y0).abs() < 20.0 && skill_cards.x1 <= words.x0,
        "41 is the skill cards field, before its words"
    );
    let expected: Vec<(&str, bool)> = CHECKBOXES
        .iter()
        .map(|&label| (label, label == "Illegal formations allowed (Whack Patch)"))
        .collect();
    assert_eq!(ticked(&frame.nodes), expected);
}

/// The change one control makes to Blank's `universal`.
type FieldEdit = fn(&mut Universal);

/// Each control changes its own field and nothing else: every checkbox
/// clicked once on Blank, then the goalkeepers and skill cards numbers
/// typed into.
#[test]
fn each_control_changes_its_field_alone() {
    let blank = crate::document::blank().universal;
    let checkboxes: [(&str, FieldEdit); 11] = [
        (
            "Every player is rated A at his registered position",
            |universal| universal.registered_position_a = false,
        ),
        ("2", |universal| universal.fielded_at_a = vec![2]),
        ("No B ratings", |universal| universal.no_b_ratings = false),
        (
            "B ratings count toward the A-position allowance",
            |universal| universal.b_uses_a_allowance = true,
        ),
        ("GK is never a second A", |universal| {
            universal.gk_not_second_a = false
        }),
        ("Age", |universal| universal.age = Some((15, 50))),
        ("Weight", |universal| {
            universal.weight = Some(WeightRule {
                floor: 30,
                below: 129,
                above: 81,
            })
        }),
        ("A captain is required", |universal| {
            universal.captain_required = false
        }),
        ("At most", |universal| universal.max_com_styles = Some(5)),
        ("Illegal formations allowed (Whack Patch)", |universal| {
            universal.illegal_formations = true
        }),
        ("3", |universal| universal.fielded_at_a = vec![3]),
    ];
    for (label, edit) in checkboxes {
        let (ctx, _events, mut window, mut view) = universal_view(crate::document::blank());
        tick(&mut window, &mut view, &ctx, label);
        let mut expected = blank.clone();
        edit(&mut expected);
        assert_eq!(universal(&view), &expected, "{label}");
    }

    let numbers: [(&str, &str, FieldEdit); 2] = [
        ("1", "3", |universal| universal.min_goalkeepers = 3),
        ("10", "20", |universal| universal.max_skill_cards = 20),
    ];
    for (shown, typed, edit) in numbers {
        let (ctx, _events, mut window, mut view) = universal_view(crate::document::blank());
        set_number(&mut window, &mut view, &ctx, shown, typed);
        let mut expected = blank.clone();
        edit(&mut expected);
        assert_eq!(universal(&view), &expected, "the number showing {shown}");
    }
}

/// A hand-edited file's values past the controls' ranges (60
/// goalkeepers, age to 80, 50 skill cards, 9 COM styles) are drawn as
/// stored and left unchanged by the drawing.
#[test]
fn values_past_a_range_are_drawn_as_stored() {
    let mut ruleset = crate::document::blank();
    ruleset.universal.min_goalkeepers = 60;
    ruleset.universal.age = Some((15, 80));
    ruleset.universal.max_skill_cards = 50;
    ruleset.universal.max_com_styles = Some(9);
    let (ctx, _events, mut window, mut view) = universal_view(ruleset.clone());
    let frame = settled(&mut window, &mut view, &ctx);
    for value in ["60", "80", "50", "9"] {
        spin_button(&frame.nodes, value);
    }
    assert_eq!(
        view.document.as_ref().expect("a document").ruleset,
        ruleset,
        "drawing changed nothing"
    );
}
