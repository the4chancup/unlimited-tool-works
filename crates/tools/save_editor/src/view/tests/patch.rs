//! The Apply aesthetics patch panel's tests (SE-GUI-18, SE-GUI-22): the
//! preview in the views' terms, Apply as one undoable edit, a refusal of
//! another version's patch, the skips' warning lines, the preview
//! following the save, the row's click, and the entries that open it.

use std::collections::BTreeMap;

use pes_savefile::interchange::aesthetics_patch::{
    ALLOCATION_SCHEME_VERSION, AestheticsPatch, PatchPlayer, PatchTeam,
};
use pes_savefile::settings_toml::{AppearanceSettings, PlayerSettings, SettingKey};

use super::*;

/// Team `/co/`, 714 in the PES 21 fixture, whose slot 3 is player 71403.
const CO: u32 = 714;
const CO_SLOT_3: u32 = 71403;

/// The panel's title, its window's label.
const TITLE: &str = "Apply aesthetics patch";

/// A patch text for `version` under `scheme`, holding `teams`.
fn patch_text(version: PesVersion, scheme: u32, teams: Vec<PatchTeam>) -> String {
    AestheticsPatch {
        pes_version: version,
        tool_version: "0.1.0".to_owned(),
        allocation_scheme_version: scheme,
        cpks: vec![],
        compiled: "2026-10-13T00:00:00Z".parse().unwrap(),
        teams,
    }
    .to_toml()
    .unwrap()
}

/// A `[[teams]]` block.
fn team(id: u32, name: &str, players: BTreeMap<u8, PatchPlayer>) -> PatchTeam {
    PatchTeam {
        id,
        name: name.to_owned(),
        players,
    }
}

/// A patch entry holding nothing: every key `None`.
fn bare() -> PatchPlayer {
    PatchPlayer {
        name: None,
        shirt_name: None,
        boots_id: None,
        gloves_id: None,
        base_copy_id: None,
        appearance: None,
    }
}

/// `player`'s own appearance resolved to `version`, `changes` (stored
/// values, as the patch's table holds them) on top: a whole `[appearance]`
/// whose diff against the save is `changes` alone.
fn look_of(
    player: &PlayerEntry,
    version: PesVersion,
    changes: &[(SettingKey, u8)],
) -> AppearanceSettings {
    let mut settings = PlayerSettings::from_player(player).unwrap();
    for (key, value) in changes {
        settings.set(*key, *value).unwrap();
    }
    settings.appearance.resolved(version)
}

/// `id`'s entry in the open save.
fn file_player(view: &SaveView, id: u32) -> &PlayerEntry {
    view.session
        .as_ref()
        .expect("an open save")
        .file()
        .player(id)
        .expect("the player")
}

/// Team `/co/`'s block: slot 03 renamed, his own look with two motions
/// changed (hunching dribbling and corner kick, shown 2 and 3).
fn co_block(view: &SaveView) -> PatchTeam {
    team(
        CO,
        "/co/",
        BTreeMap::from([(
            3,
            PatchPlayer {
                name: Some("PATCHED".to_owned()),
                shirt_name: Some("PATCHED".to_owned()),
                appearance: Some(look_of(
                    file_player(view, CO_SLOT_3),
                    PesVersion::Pes21,
                    &[
                        (SettingKey::HunchingDribbling, 1),
                        (SettingKey::CornerKick, 2),
                    ],
                )),
                ..bare()
            },
        )]),
    )
}

/// `version`'s fixture save open under `test`, in a window tall enough
/// for the panel's list.
fn open_save(version: PesVersion, test: &str) -> (SaveView, Headless) {
    let path = fixture(version, test);
    let mut view = SaveView::new();
    view.open(&path);
    (view, Headless::sized(DEFAULT_PANE.x, 800.0))
}

/// `text` written as `patch.toml` beside the open save, in the test's
/// scratch folder.
fn write_patch(view: &SaveView, text: &str) -> PathBuf {
    let path = view
        .session
        .as_ref()
        .expect("an open save")
        .path()
        .with_file_name("patch.toml");
    std::fs::write(&path, text).unwrap();
    path
}

/// The panel opened from the Savefile menu's entry.
fn open_panel(window: &mut Headless, view: &mut SaveView) {
    let nodes = open_menu(window, view, "Savefile", "Apply aesthetics patch");
    click(
        window,
        node_labeled(&nodes, Role::Button, "Apply aesthetics patch\u{2026}"),
        view,
    );
}

/// `path` stubbed as the Open patch dialog's choice; `Open patch…`
/// clicked in the panel.
fn open_patch(window: &mut Headless, view: &mut SaveView, path: PathBuf) {
    view.patch_dialog = Box::new(move || Some(path.clone()));
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let button = panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some("Open patch\u{2026}"))
        .expect("the Open patch button");
    click(window, button, view);
}

/// The panel open from the menu with `text` opened as its patch.
fn previewed(window: &mut Headless, view: &mut SaveView, text: &str) -> PathBuf {
    let path = write_patch(view, text);
    open_panel(window, view);
    open_patch(window, view, path.clone());
    path
}

/// The panel window's nodes out of `pairs` (`frame_ids`' output): the
/// subtree of the node labeled exactly `TITLE` that is not the ownership
/// line's link.
fn panel_nodes(pairs: &[(accesskit::NodeId, accesskit::Node)]) -> Vec<&accesskit::Node> {
    let (window, _) = pairs
        .iter()
        .find(|(_, node)| node.label() == Some(TITLE) && node.role() != Role::Link)
        .expect("the patch panel");
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        pairs.iter().map(|(id, node)| (*id, node)).collect();
    let mut nodes = Vec::new();
    let mut stack = vec![*window];
    while let Some(id) = stack.pop() {
        let node = by_id[&id];
        stack.extend(node.children());
        nodes.push(node);
    }
    nodes
}

/// The panel's Label values and Button labels from `Open patch…` down, in
/// top-to-bottom, left-to-right order: what a member reads (the window's
/// resize handles left out).
fn panel_lines(window: &mut Headless, view: &mut SaveView) -> Vec<String> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    let mut lines: Vec<(f64, f64, String)> = Vec::new();
    for node in panel_nodes(&pairs) {
        let text = match node.role() {
            Role::Label => node.value().map(str::to_owned),
            _ => node.label().map(str::to_owned),
        };
        if let (Some(text), Some(bounds)) = (text, node.bounds()) {
            lines.push((bounds.y0, bounds.x0, text));
        }
    }
    lines.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("drawn bounds")
            .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
    });
    lines
        .into_iter()
        .map(|(_, _, text)| text)
        .skip_while(|text| text != "Open patch\u{2026}")
        .filter(|text| !text.starts_with("Resize window"))
        .collect()
}

/// The panel's Button labeled `label`, `None` when it is not drawn.
fn panel_button(
    window: &mut Headless,
    view: &mut SaveView,
    label: &str,
) -> Option<accesskit::Node> {
    let (_, pairs, _) = window.frame_ids(Vec::new(), |ui| show(ui, view, &|| Vec::new()));
    panel_nodes(&pairs)
        .into_iter()
        .find(|node| node.role() == Role::Button && node.label() == Some(label))
        .cloned()
}

/// The open save's revision.
fn revision(view: &SaveView) -> u64 {
    view.session.as_ref().expect("an open save").revision()
}

/// `player_id`'s entry after `text`'s patch is applied, through the lib,
/// to a copy of the open save.
fn patched_entry(view: &SaveView, text: &str, player_id: u32) -> PlayerEntry {
    let mut copy = view.session.as_ref().expect("an open save").file().clone();
    AestheticsPatch::parse(text)
        .expect("the patch parses")
        .apply(&mut copy)
        .expect("the patch applies");
    copy.player(player_id).expect("the patched player").clone()
}

/// The `/co/` patch's table rows for 71403 on the PES 21 fixture (item,
/// old value, new value), in the views' terms and the comparison's order:
/// the fixture stores his corner kick and hunching (dribbling) motions 0
/// (shown 1), his base copy is player 7511, and his name and shirt name
/// are `PLACEHOLDER`.
const CO_ROWS: [[&str; 3]; 5] = [
    ["PLACEHOLDER \u{b7} Corner kick", "1", "3"],
    ["PLACEHOLDER \u{b7} Hunching (dribbling)", "1", "2"],
    ["PLACEHOLDER \u{b7} Copy from ID", "7511", "71403"],
    ["PLACEHOLDER \u{b7} Name", "PLACEHOLDER", "PATCHED"],
    ["PLACEHOLDER \u{b7} Shirt", "PLACEHOLDER", "PATCHED"],
];

/// `rows` under the table's header row, as `panel_lines` reads them: cell
/// after cell, row by row.
fn table(rows: &[[&str; 3]]) -> Vec<String> {
    ["Item", "Old value", "New value"]
        .iter()
        .chain(rows.iter().flatten())
        .map(|cell| (*cell).to_owned())
        .collect()
}

/// Team 999 `/zz/`'s block: the PES 21 fixture holds every team of the
/// teams list (701 to 920), so the team it lacks is the command line
/// suite's own, no cup's. Its one player is a name alone.
fn zz_team() -> PatchTeam {
    team(
        999,
        "/zz/",
        BTreeMap::from([(
            1,
            PatchPlayer {
                name: Some("NOBODY".to_owned()),
                ..bare()
            },
        )]),
    )
}

/// Team `/f/`, 718 in the PES 21 fixture, whose slot 1 is player 71801.
const F: u32 = 718;
const F_SLOT_1: u32 = 71801;

/// `/co/`'s slot 5, player 71405 in the PES 21 fixture.
const CO_SLOT_5: u32 = 71405;

/// Team `/f/`'s block: slot 01 renamed.
fn f_team() -> PatchTeam {
    team(
        F,
        "/f/",
        BTreeMap::from([(
            1,
            PatchPlayer {
                name: Some("PATCHED".to_owned()),
                ..bare()
            },
        )]),
    )
}

/// The first `/co/` slot whose player the open save lacks (a cup player's
/// id is his team's id × 100 + his slot).
fn co_missing_slot(view: &SaveView) -> u32 {
    let file = view.session.as_ref().expect("an open save").file();
    (1..=99)
        .find(|slot| file.player(CO * 100 + slot).is_none())
        .expect("the fixture's /co/ lacks a slot's player")
}

/// `/co/`'s slot `slot` renamed, a player the save lacks in the tests.
fn co_absent_player(slot: u32) -> (u8, PatchPlayer) {
    (
        u8::try_from(slot).expect("a slot fits u8"),
        PatchPlayer {
            name: Some("NOBODY".to_owned()),
            ..bare()
        },
    )
}

/// `/co/`'s two players: 03's corner kick and 05's hunching (dribbling),
/// whose rows differ in their label.
fn co_two_players(view: &SaveView) -> BTreeMap<u8, PatchPlayer> {
    BTreeMap::from([
        (
            3,
            PatchPlayer {
                appearance: Some(look_of(
                    file_player(view, CO_SLOT_3),
                    PesVersion::Pes21,
                    &[(SettingKey::CornerKick, 2)],
                )),
                ..bare()
            },
        ),
        (
            5,
            PatchPlayer {
                appearance: Some(look_of(
                    file_player(view, CO_SLOT_5),
                    PesVersion::Pes21,
                    &[(SettingKey::HunchingDribbling, 1)],
                )),
                ..bare()
            },
        ),
    ])
}

/// The two-team patch: `/co/`'s two players and one the save lacks
/// (`missing`), team 999 the save lacks, then `/f/`'s slot 01.
fn two_team_patch(view: &SaveView, missing: u32) -> String {
    let mut players = co_two_players(view);
    let (slot, absent) = co_absent_player(missing);
    players.insert(slot, absent);
    patch_text(
        PesVersion::Pes21,
        ALLOCATION_SCHEME_VERSION,
        vec![team(CO, "/co/", players), zz_team(), f_team()],
    )
}

/// The two-team patch's rows on the PES 21 fixture: 71403's corner kick,
/// 71405's hunching (dribbling), both their base copies cleared to their
/// own ids, and 71801's name — every name `PLACEHOLDER`.
const TWO_TEAM_ROWS: [[&str; 3]; 5] = [
    ["PLACEHOLDER \u{b7} Corner kick", "1", "3"],
    ["PLACEHOLDER \u{b7} Copy from ID", "7511", "71403"],
    ["PLACEHOLDER \u{b7} Hunching (dribbling)", "1", "2"],
    ["PLACEHOLDER \u{b7} Copy from ID", "7511", "71405"],
    ["PLACEHOLDER \u{b7} Name", "PLACEHOLDER", "PATCHED"],
];

mod apply;
mod panel;
mod preview;
