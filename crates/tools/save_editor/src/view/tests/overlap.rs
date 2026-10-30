//! The overlap test: every Save editor screen drawn at the default pane,
//! on a PES 21 and a PES 17 save, failing when two widgets' rectangles
//! intersect. egui draws a row too wide for its room over its neighbour
//! rather than refusing it, so a collision a later change brings (a longer
//! label, a widened field) would otherwise show on no capture until
//! someone looks.

use pes_savefile::model::names::display_name;

use super::*;
use crate::session::rules::{self, ImportCheck};
use crate::view::prompt::{self, Pending};
use crate::view::ruleset_picker::RulesDialog;

/// The widget and label roles the pane-fit test reads, plus `Unknown` (the
/// pitch canvas' and the lineup rows' role).
const NODE_ROLES: [Role; 12] = [
    Role::Button,
    Role::CheckBox,
    Role::ComboBox,
    Role::RadioButton,
    Role::SpinButton,
    Role::Slider,
    Role::TextInput,
    Role::ColorWell,
    Role::ScrollBar,
    Role::Link,
    Role::Label,
    Role::Unknown,
];

/// How far two rectangles may run into each other, in points, in either
/// direction, and how far past another's edge a node drawn inside it may
/// reach: the drafts' tolerance, since two rows touching share an edge.
const TOLERANCE: f32 = 1.0;

/// An overlap a screen means: the two nodes by role and text (a Label's
/// value, any other node's label), never by position, which would excuse
/// whatever moved there next.
struct Meant {
    /// The screens it is meant on, as the screen functions name them
    /// (either version).
    screens: &'static [&'static str],
    /// One node's role and text.
    first: (Role, &'static str),
    /// The other's.
    second: (Role, &'static str),
    /// Why the screen draws one over the other.
    reason: &'static str,
}

/// The screens that draw the Tactics pitch.
const PITCH_SCREENS: &[&str] = &["team view", "team view, Tactics popover", "Kit slots"];

/// Why two pitch markers overlap.
const MARKERS_STACK: &str = "each marker (portrait, badge, name) stands at its player's spot and \
     is taller than the gap between two spots of neighbouring lines, so the lower marker covers \
     the one above it, as the game draws them (the draft's `drawOrder`)";

/// The overlaps the screens mean.
const MEANT: &[Meant] = &[
    Meant {
        screens: PITCH_SCREENS,
        first: (Role::Button, "GK PLACEHOLDER"),
        second: (Role::Button, "CB PLACEHOLDER"),
        reason: MARKERS_STACK,
    },
    Meant {
        screens: PITCH_SCREENS,
        first: (Role::Button, "CB PLACEHOLDER"),
        second: (Role::Button, "DMF PLACEHOLDER"),
        reason: MARKERS_STACK,
    },
    Meant {
        screens: PITCH_SCREENS,
        first: (Role::Button, "AMF PLACEHOLDER"),
        second: (Role::Button, "CF PLACEHOLDER"),
        reason: MARKERS_STACK,
    },
];

/// A widget or label as a frame drew it: its AccessKit node, the egui
/// layer it was drawn on, and its rect clipped to what its scroll area or
/// panel shows. AccessKit's bounds are the whole rect, so a list row laid
/// out past its list's edge, which nobody sees, would collide with
/// whatever stands below the list.
struct Drawn {
    node: accesskit::Node,
    layer: egui::LayerId,
    rect: egui::Rect,
}

/// Screens as a screen function draws them: each its name and its widgets.
type Screens = Vec<(&'static str, Vec<Drawn>)>;

/// What a node shows: a Label's value, any other node's label.
fn text(node: &accesskit::Node) -> &str {
    node.label().or(node.value()).unwrap_or("")
}

/// Whether `inner` lies inside `outer`, within the tolerance: a lineup
/// row's handle, position and name, or a marker on the pitch, is part of
/// the widget it sits in.
fn contains(outer: egui::Rect, inner: egui::Rect) -> bool {
    outer.expand(TOLERANCE).contains_rect(inner)
}

/// Whether `a` and `b` run into each other deeper than the tolerance in
/// both directions.
fn collide(a: egui::Rect, b: egui::Rect) -> bool {
    let depth = a.intersect(b).size();
    depth.x > TOLERANCE && depth.y > TOLERANCE
}

/// The index of the `MEANT` entry excusing `a` over `b` on `screen`.
fn meant(screen: &str, a: &accesskit::Node, b: &accesskit::Node) -> Option<usize> {
    let names = |node: &accesskit::Node, (role, label): (Role, &str)| {
        node.role() == role && text(node) == label
    };
    MEANT.iter().position(|entry| {
        entry.screens.contains(&screen)
            && ((names(a, entry.first) && names(b, entry.second))
                || (names(a, entry.second) && names(b, entry.first)))
    })
}

/// Every collision on `screen`'s frame, one line each naming both nodes'
/// role, text and rect; a pair `MEANT` excuses marks its entry used
/// instead. Only nodes of one layer are compared: a modal, a window or a
/// popup is drawn over the view by design.
fn collisions(
    version: PesVersion,
    screen: &str,
    drawn: &[Drawn],
    used: &mut [bool],
) -> Vec<String> {
    let mut found = Vec::new();
    for (i, a) in drawn.iter().enumerate() {
        for b in &drawn[i + 1..] {
            if a.layer != b.layer
                || contains(a.rect, b.rect)
                || contains(b.rect, a.rect)
                || !collide(a.rect, b.rect)
            {
                continue;
            }
            if let Some(entry) = meant(screen, &a.node, &b.node) {
                used[entry] = true;
                continue;
            }
            found.push(format!(
                "{version:?} {screen}: {:?} {:?} {:?} overlaps {:?} {:?} {:?}",
                a.node.role(),
                text(&a.node),
                a.rect,
                b.node.role(),
                text(&b.node),
                b.rect,
            ));
        }
    }
    found
}

/// Three frames, then the last one's widgets and labels as drawn: a modal
/// re-centres once its content's size is known, and a scroll's claimed
/// content size lands a frame late.
fn settled(window: &mut Headless, view: &mut SaveView) -> Vec<Drawn> {
    let mut pairs = Vec::new();
    for _ in 0..3 {
        let (_, frame, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
        pairs = frame;
    }
    // The pass that just ended: egui moves it to `prev_pass` at its end.
    let rects: HashMap<accesskit::NodeId, egui::WidgetRect> = window.ctx.viewport(|viewport| {
        viewport
            .prev_pass
            .widgets
            .layers()
            .flat_map(|(_, rects)| rects.iter())
            .map(|rect| (rect.id.accesskit_id(), *rect))
            .collect()
    });
    pairs
        .into_iter()
        .filter(|(_, node)| NODE_ROLES.contains(&node.role()))
        .map(|(id, node)| {
            let rect = rects.get(&id).unwrap_or_else(|| {
                panic!("{:?} {:?} has no widget rect", node.role(), text(&node))
            });
            Drawn {
                node,
                layer: rect.layer_id,
                rect: rect.interact_rect,
            }
        })
        // A node clipped away whole shows nowhere, and the player list's
        // rows past its edge are most of a frame's nodes: the pairs would
        // cost seconds per screen.
        .filter(|drawn| drawn.rect.is_positive())
        .collect()
}

/// `settled`, asserting that `screen`'s dialog, window or popup is up over
/// the view: a click that opened nothing would leave only the view to
/// check, and pass.
fn settled_over(window: &mut Headless, view: &mut SaveView, screen: &str) -> Vec<Drawn> {
    let drawn = settled(window, view);
    assert!(
        drawn
            .iter()
            .any(|drawn| drawn.layer.order != egui::Order::Background),
        "premise: {screen} is up over the view"
    );
    drawn
}

/// `version`'s fixture open at the default pane under `test`'s scratch
/// folder, the first player selected.
fn opened(version: PesVersion, test: &str) -> (SaveView, Headless) {
    let path = fixture(version, &format!("overlap_{test}_{version:?}"));
    let mut view = SaveView::new();
    view.open(&path);
    (view, Headless::new())
}

/// `opened`, then `menu`'s `item` clicked.
fn from_menu(version: PesVersion, test: &str, menu: &str, item: &str) -> (SaveView, Headless) {
    let (mut view, mut window) = opened(version, test);
    let nodes = open_menu(&mut window, &mut view, menu, item);
    click(&mut window, menu_item(&nodes, item), &mut view);
    (view, window)
}

/// The scratch folder the open save was written to.
fn scratch_dir(view: &SaveView) -> PathBuf {
    view.session
        .as_ref()
        .expect("an open save")
        .path()
        .parent()
        .expect("the scratch dir")
        .to_path_buf()
}

/// The team the screens show on both fixtures.
const TEAM: u32 = 701;

/// The player view, Appearance closed then open, and the team view with
/// the Tactics popover closed then open.
fn views(version: PesVersion) -> Screens {
    let mut screens = Vec::new();
    let (mut view, mut window) = opened(version, "player");
    screens.push(("player view", settled(&mut window, &mut view)));
    open_appearance(&mut window, &mut view);
    screens.push((
        "player view, Appearance open",
        settled(&mut window, &mut view),
    ));

    let (mut view, mut window) = opened(version, "team");
    open_team_view(&mut window, &mut view, TEAM);
    screens.push(("team view", settled(&mut window, &mut view)));
    // The popover opens on a click on an outfield marker no other marker
    // covers.
    let canvas = canvas_of(&draw(&mut window, &mut view));
    let players = team_entry(&view, TEAM).tactics.presets[0].formations[0].players;
    let at = (0..11)
        .filter(|&index| players[index].position != 0)
        .find_map(|index| marker_point(canvas, &players, index))
        .expect("a marker a click can reach");
    click_at(&mut window, at, &mut view);
    assert!(
        view.tactics.popover_area().is_some(),
        "premise: the popover opened"
    );
    screens.push((
        "team view, Tactics popover",
        settled_over(&mut window, &mut view, "the Tactics popover"),
    ));
    screens
}

/// The Live AATF dock: checking, before the first result (the pointer
/// held, so no check starts), then listing the team's issues.
fn live_aatf(version: PesVersion) -> Screens {
    let (mut view, mut window) = opened(version, "live_aatf");
    view.live_aatf = true;
    // The toolbar's empty end.
    let at = egui::pos2(1090.0, 12.0);
    window.frame(pointer_at(at, true), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    let checking = settled(&mut window, &mut view);
    assert!(
        view.live.check.result.is_none(),
        "premise: no check started while the pointer is held"
    );
    window.frame(pointer_at(at, false), |ui| {
        show(ui, &mut view, &|| Vec::new())
    });
    frames_until(
        &mut window,
        &mut view,
        |view| view.live.check.pending.is_none() && view.live.check.result.is_some(),
        "the Live AATF check never returned",
    );
    let listed = settled(&mut window, &mut view);
    vec![
        ("Live AATF, checking", checking),
        ("Live AATF, listed", listed),
    ]
}

/// The team view's Kit slots dialog and the menus' dialogs.
fn menu_dialogs(version: PesVersion) -> Screens {
    let mut screens = Vec::new();
    let (mut view, mut window) = opened(version, "kit_slots");
    open_team_view(&mut window, &mut view, TEAM);
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, crate::view::team::KIT_SLOTS_LABEL),
        &mut view,
    );
    screens.push((
        "Kit slots",
        settled_over(&mut window, &mut view, "Kit slots"),
    ));

    for (screen, menu, item) in [
        (
            "Set boots and gloves",
            "Team",
            "Set boots and gloves IDs for everyone\u{2026}",
        ),
        ("Set all stats", "Player", "Set all stats\u{2026}"),
        ("Bump all stats", "Player", "Bump all stats\u{2026}"),
        ("Copy stats from", "Player", "Copy stats from\u{2026}"),
        ("Swap stats with", "Player", "Swap stats with\u{2026}"),
        ("Fix database", "Savefile", "Fix database\u{2026}"),
        (
            "Generate in-game face",
            "Player",
            "Generate in-game face\u{2026}",
        ),
    ] {
        let (mut view, mut window) = from_menu(version, screen, menu, item);
        screens.push((screen, settled_over(&mut window, &mut view, screen)));
    }

    // Generate settings.toml with no folder, then with one picked.
    let (mut view, mut window) = from_menu(
        version,
        "generate",
        "Player",
        "Generate settings.toml\u{2026}",
    );
    screens.push((
        "Generate settings.toml",
        settled_over(&mut window, &mut view, "Generate settings.toml"),
    ));
    let folder = scratch_dir(&view);
    view.pick_folder_dialog = Box::new(move || Some(folder.clone()));
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    screens.push((
        "Generate settings.toml, folder picked",
        settled_over(&mut window, &mut view, "Generate settings.toml"),
    ));
    screens
}

/// The Multi-team AATF dialog listing every team, then its range reversed,
/// which lists none.
fn multi_aatf(version: PesVersion) -> Screens {
    let (mut view, mut window) =
        from_menu(version, "multi_aatf", "Savefile", "Multi-team AATF\u{2026}");
    frames_until(
        &mut window,
        &mut view,
        |view| view.multi_aatf.check.pending.is_none() && view.multi_aatf.check.result.is_some(),
        "the Multi-team AATF check never returned",
    );
    let listed = settled_over(&mut window, &mut view, "Multi-team AATF");
    let names: Vec<String> = {
        let file = view.session.as_ref().expect("an open save").file();
        let mut ids: Vec<u32> = file.teams().iter().map(|team| team.id).collect();
        ids.sort_unstable();
        ids.iter()
            .take(2)
            .map(|&id| format!("{id} {}", display_name(&file.team(id).unwrap().name)))
            .collect()
    };
    // The last end first: the range in between is the first team alone,
    // so no check of the whole save runs again before the reversal.
    super::savefile::pick_range_end(&mut window, &mut view, 1, &names[0]);
    super::savefile::pick_range_end(&mut window, &mut view, 0, &names[1]);
    let reversed = settled_over(&mut window, &mut view, "Multi-team AATF");
    vec![
        ("Multi-team AATF, listed", listed),
        ("Multi-team AATF, reversed", reversed),
    ]
}

/// The comparator with nothing picked, then with a save of the same
/// version picked, its one edit listed.
fn comparator(version: PesVersion) -> Screens {
    let (mut view, mut window) = from_menu(version, "comparator", "Savefile", "Comparator\u{2026}");
    let empty = settled_over(&mut window, &mut view, "the comparator");
    let dir = scratch_dir(&view);
    let picked = dir.join("picked");
    let mut file = EditFile::from_bytes(&fixture_save(version)).expect("the fixture loads");
    let player_id = file.team(TEAM).expect("the team").roster[0].player_id;
    let player = file.player_mut(player_id).expect("the player");
    player.basic.height = if player.basic.height == 170 { 171 } else { 170 };
    file.save(&picked).unwrap();
    view.open_save_dialog = Box::new(move || Some(picked.clone()));
    let nodes = draw(&mut window, &mut view);
    click(
        &mut window,
        node_labeled(&nodes, Role::Button, "Pick\u{2026}"),
        &mut view,
    );
    frames_until(
        &mut window,
        &mut view,
        |view| {
            view.comparator.comparison.pending.is_none()
                && view.comparator.comparison.result.is_some()
        },
        "the comparison never returned",
    );
    let listed = settled_over(&mut window, &mut view, "the comparator");
    vec![
        ("Comparator, nothing picked", empty),
        ("Comparator, listed", listed),
    ]
}

/// The unsaved prompt over an edited player, the Edit mode question over
/// it, and the refusal of a file that is no save.
fn prompts(version: PesVersion) -> Screens {
    let (mut view, mut window) = opened(version, "prompt");
    let (_, _, player_id) = selected_player(&view).expect("a selected player");
    let mut entry = player_entry(&view, player_id);
    entry.basic.height = if entry.basic.height == 170 { 171 } else { 170 };
    view.session
        .as_mut()
        .expect("an open save")
        .edit_players(vec![entry]);
    prompt::open(&mut view, Pending::Close);
    let unsaved = settled_over(&mut window, &mut view, "the unsaved prompt");
    let target = view.save_path().expect("an open save").to_path_buf();
    view.edit_mode_question = Some(EditModeQuestion {
        game: version,
        target,
    });
    let edit_mode = settled_over(&mut window, &mut view, "the Edit mode question");

    let (mut view, mut window) = opened(version, "refusal");
    let junk = scratch_dir(&view).join("not_a_save");
    std::fs::write(&junk, b"not a save").unwrap();
    view.open(&junk);
    assert!(view.refusal.is_some(), "premise: the open was refused");
    let refusal = settled_over(&mut window, &mut view, "the refusal");
    vec![
        ("unsaved prompt", unsaved),
        ("Edit mode question", edit_mode),
        ("refusal", refusal),
    ]
}

/// The ruleset picker's three dialogs: an import's errors, an import
/// replacing an earlier one, and an imported row's removal.
fn ruleset_dialogs(version: PesVersion) -> Screens {
    let official = aatf::official_file();
    let renamed = official.replacen("name: \"4cc Autumn 26\"", "name: \"4cc Autumn 25\"", 1);
    assert_ne!(renamed, official, "premise: the name was replaced");
    let mut screens = Vec::new();

    let (mut view, mut window) = opened(version, "rules_errors");
    let dir = scratch_dir(&view).join("picked");
    std::fs::create_dir_all(&dir).unwrap();
    let broken = dir.join("autumn25.4ccrules");
    std::fs::write(
        &broken,
        renamed
            .replacen(
                "label: \"Gold\", count: 1,",
                "label: \"Gold\", count: 21,",
                1,
            )
            .replacen("suggestions: [\"cards\"", "suggestions: [\"bogus\"", 1),
    )
    .unwrap();
    let ImportCheck::Errors(findings) = rules::check_import(&broken, &[]) else {
        panic!("premise: the broken file is refused");
    };
    view.rules_dialog = Some(RulesDialog::Errors {
        file: "autumn25.4ccrules".to_owned(),
        findings,
    });
    screens.push((
        "rules file errors",
        settled_over(&mut window, &mut view, "the errors dialog"),
    ));

    let (mut view, mut window) = opened(version, "rules_replace");
    pick_imported(&mut view, "autumn25.4ccrules", &renamed);
    let dir = scratch_dir(&view).join("picked");
    std::fs::create_dir_all(&dir).unwrap();
    let again = dir.join("autumn25.4ccrules");
    std::fs::write(&again, &renamed).unwrap();
    let ImportCheck::Clean(import) = rules::check_import(&again, &view.imported) else {
        panic!("premise: the file imports");
    };
    assert!(
        !import.replaces.is_empty(),
        "premise: it replaces the import"
    );
    view.rules_dialog = Some(RulesDialog::Replace {
        file: "autumn25.4ccrules".to_owned(),
        import,
    });
    screens.push((
        "replace an imported ruleset",
        settled_over(&mut window, &mut view, "the replace dialog"),
    ));

    let (mut view, mut window) = opened(version, "rules_remove");
    pick_imported(&mut view, "autumn25.4ccrules", &renamed);
    view.rules_dialog = Some(RulesDialog::Remove {
        file_name: "autumn25.4ccrules".to_owned(),
        description: "4cc Autumn 25".to_owned(),
    });
    screens.push((
        "remove an imported ruleset",
        settled_over(&mut window, &mut view, "the remove dialog"),
    ));
    screens
}

/// Asserts that no two widgets of one layer overlap on `screens` but for
/// the overlaps `MEANT` lists, and that every entry naming a screen drawn
/// here still excuses one: an entry excusing nothing names a pair that
/// moved apart or was renamed, and would excuse whatever took its name.
fn assert_no_overlap(screens: &[(PesVersion, &str, Vec<Drawn>)]) {
    let mut used = vec![false; MEANT.len()];
    let mut found = Vec::new();
    for (version, screen, drawn) in screens {
        found.extend(collisions(*version, screen, drawn, &mut used));
    }
    let drawn_here = |entry: &Meant| {
        screens
            .iter()
            .any(|(_, screen, _)| entry.screens.contains(screen))
    };
    let stale: Vec<String> = MEANT
        .iter()
        .zip(&used)
        .filter(|&(entry, used)| !*used && drawn_here(entry))
        .map(|(entry, _)| {
            format!(
                "{:?}: {:?} {:?} over {:?} {:?} excuses nothing ({})",
                entry.screens,
                entry.first.0,
                entry.first.1,
                entry.second.0,
                entry.second.1,
                entry.reason
            )
        })
        .collect();
    assert!(
        found.is_empty() && stale.is_empty(),
        "{} collisions:\n{}\n{} stale entries:\n{}",
        found.len(),
        found.join("\n"),
        stale.len(),
        stale.join("\n")
    );
}

/// `draw`'s screens on the PES 21 fixture, then on the PES 17 one.
fn on_both(draw: fn(PesVersion) -> Screens) -> Vec<(PesVersion, &'static str, Vec<Drawn>)> {
    [PesVersion::Pes21, PesVersion::Pes17]
        .into_iter()
        .flat_map(|version| {
            draw(version)
                .into_iter()
                .map(move |(screen, drawn)| (version, screen, drawn))
        })
        .collect()
}

// One test per group of screens, so libtest draws the groups in parallel:
// every frame lays out the whole player list, and drawn one after another
// the screens take a minute.

#[test]
fn no_two_widgets_overlap_on_the_player_and_team_views() {
    assert_no_overlap(&on_both(views));
}

#[test]
fn no_two_widgets_overlap_on_the_live_aatf_dock() {
    assert_no_overlap(&on_both(live_aatf));
}

#[test]
fn no_two_widgets_overlap_on_the_menus_dialogs() {
    assert_no_overlap(&on_both(menu_dialogs));
}

#[test]
fn no_two_widgets_overlap_on_the_multi_team_aatf_dialog() {
    assert_no_overlap(&on_both(multi_aatf));
}

#[test]
fn no_two_widgets_overlap_on_the_comparator() {
    assert_no_overlap(&on_both(comparator));
}

#[test]
fn no_two_widgets_overlap_on_the_prompts_and_the_refusal() {
    assert_no_overlap(&on_both(prompts));
}

#[test]
fn no_two_widgets_overlap_on_the_ruleset_dialogs() {
    assert_no_overlap(&on_both(ruleset_dialogs));
}
