use egui::accesskit::{self, Role};
use pes_version::PesVersion;

use super::*;
use crate::test_support::{Headless, centre, control_right_of, pointer_at};

/// The grass-drop position rule's tests.
mod drop_rule;
/// The drag-and-drop and swap tests.
mod drops;
/// The lineup lists' rows' tests.
mod lineup;
/// The geometry and painted-output tests.
mod paint;
/// The slot popover and armed assign modes' tests.
mod popover;

impl Headless {
    /// One frame of `lineup` over `tactics`, `view` and `roster`: the
    /// frame's AccessKit nodes and whether it changed `tactics`.
    pub(crate) fn lineup(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        let mut changed = None;
        let (nodes, _) = self.frame(events, |ui| {
            changed = Some(super::lineup(ui, "lineup", version, tactics, view, roster));
        });
        (nodes, changed.expect("the widget ran"))
    }

    /// A drag from `from` to `onto` over the widget (press, move,
    /// release): whether the drop changed `tactics`.
    pub(crate) fn drag_lineup(
        &mut self,
        from: egui::Pos2,
        onto: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> bool {
        self.lineup(version, tactics, view, roster, pointer_at(from, true));
        self.lineup(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(onto)],
        );
        let (_, changed) = self.lineup(version, tactics, view, roster, pointer_at(onto, false));
        changed
    }

    /// One frame of `pitch` over `tactics`, `view` and `roster`: the
    /// frame's AccessKit nodes and whether it changed `tactics`.
    pub(crate) fn pitch(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        let mut changed = None;
        let (nodes, _) = self.frame(events, |ui| {
            changed = Some(super::pitch(ui, "pitch", version, tactics, view, roster));
        });
        (nodes, changed.expect("the widget ran"))
    }

    /// `pitch` keeping the nodes' AccessKit ids (for
    /// `AccessKitActionRequest` events on a combo's popup items).
    fn pitch_ids(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> Vec<(accesskit::NodeId, accesskit::Node)> {
        let (nodes, _) = self.frame_at_ids(None, vec![], |ui| {
            super::pitch(ui, "pitch", version, tactics, view, roster);
        });
        nodes
    }

    /// An AccessKit `Click` action on `id`: a click delivered as the
    /// accessibility event, for widgets a pointer click does not
    /// reach (a combo's popup items).
    fn action_click_pitch(
        &mut self,
        id: accesskit::NodeId,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, bool) {
        self.pitch(
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

    /// One frame of `lineup` keeping the painted shapes: the name
    /// assertions read the rows' text runs.
    pub(crate) fn lineup_paint(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        self.frame(vec![], |ui| {
            super::lineup(ui, "lineup", version, tactics, view, roster);
        })
    }

    /// One frame of `pitch` keeping the painted shapes: the paint
    /// assertions read the markers' fills, strokes and text anchors.
    fn pitch_paint(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        self.frame(vec![], |ui| {
            super::pitch(ui, "pitch", version, tactics, view, roster);
        })
    }

    /// One frame of `pitch` with `events`, keeping the painted shapes:
    /// the shapes and whether it changed `tactics`.
    fn pitch_shapes(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<egui::epaint::ClippedShape>, bool) {
        let mut changed = None;
        let (_, shapes) = self.frame(events, |ui| {
            changed = Some(super::pitch(ui, "pitch", version, tactics, view, roster));
        });
        (shapes, changed.expect("the widget ran"))
    }

    /// `pitch_paint` at `time` as the frame's timestamp (the name
    /// scroll's clock and the repaint delay read it).
    fn pitch_paint_at(
        &mut self,
        time: f64,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
        self.frame_at(Some(time), vec![], |ui| {
            super::pitch(ui, "pitch", version, tactics, view, roster);
        })
    }

    /// A click at `pos` over `pitch` (press on one frame, release on
    /// the next): the release frame's nodes and change.
    pub(crate) fn click_pitch(
        &mut self,
        pos: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, bool) {
        self.pitch(version, tactics, view, roster, pointer_at(pos, true));
        self.pitch(version, tactics, view, roster, pointer_at(pos, false))
    }

    /// `text` typed into the open popover's spin field right of `label`
    /// (`"X"` or `"Y"`) over `pitch`: a frame to find the field, a click
    /// on it, the text on the next frame and Enter on the one after.
    pub(crate) fn type_in_spin_field(
        &mut self,
        label: &str,
        text: &str,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) {
        let (nodes, _) = self.pitch(version, tactics, view, roster, vec![]);
        let field = control_right_of(&nodes, label, Role::SpinButton);
        self.click_pitch(centre(field), version, tactics, view, roster);
        self.pitch(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::Text(text.to_owned())],
        );
        self.pitch(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
                repeat: false,
            }],
        );
    }

    /// A drag from `from` to `onto` over `pitch` (press, move,
    /// release): whether the drop changed `tactics`.
    fn drag_pitch(
        &mut self,
        from: egui::Pos2,
        onto: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> bool {
        self.pitch(version, tactics, view, roster, pointer_at(from, true));
        self.pitch(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(onto)],
        );
        let (_, changed) = self.pitch(version, tactics, view, roster, pointer_at(onto, false));
        changed
    }

    /// One frame of `pitch` over the lineup lists, as the host's two
    /// columns draw them side by side: either's change counts.
    pub(crate) fn pitch_and_lineup(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        let mut changed = false;
        let (nodes, _) = self.frame(events, |ui| {
            super::formation_selector(ui, tactics, view);
            changed = super::pitch(ui, "pitch", version, tactics, view, roster);
            changed |= super::lineup(ui, "lineup", version, tactics, view, roster);
        });
        (nodes, changed)
    }

    /// A drag from `from` to `onto` over the pitch and the lists
    /// (press, move, release): whether the drop changed `tactics`.
    pub(crate) fn drag_pitch_and_lineup(
        &mut self,
        from: egui::Pos2,
        onto: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> bool {
        self.pitch_and_lineup(version, tactics, view, roster, pointer_at(from, true));
        self.pitch_and_lineup(
            version,
            tactics,
            view,
            roster,
            vec![egui::Event::PointerMoved(onto)],
        );
        let (_, changed) =
            self.pitch_and_lineup(version, tactics, view, roster, pointer_at(onto, false));
        changed
    }

    /// One frame of `tactics_settings` (the role chips' home for the
    /// assign-mode tests): its nodes and whether it changed `tactics`.
    fn settings(
        &mut self,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
        events: Vec<egui::Event>,
    ) -> (Vec<accesskit::Node>, bool) {
        let mut changed = None;
        let (nodes, _) = self.frame(events, |ui| {
            changed = Some(crate::test_support::tactics_settings(
                ui, "tactics", version, tactics, view, roster,
            ));
        });
        (nodes, changed.expect("the widget ran"))
    }

    /// A click at `pos` over `tactics_settings` (press on one frame,
    /// release on the next): the release frame's nodes and change.
    fn click_settings(
        &mut self,
        pos: egui::Pos2,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> (Vec<accesskit::Node>, bool) {
        self.settings(version, tactics, view, roster, pointer_at(pos, true));
        self.settings(version, tactics, view, roster, pointer_at(pos, false))
    }

    /// `tooltip` over the pitch alone.
    pub(crate) fn pitch_tooltip(
        &mut self,
        pos: egui::Pos2,
        needle: &str,
        version: PesVersion,
        tactics: &mut TeamTactics,
        view: &mut TacticsView,
        roster: &[RosterRow<'_>],
    ) -> Option<String> {
        self.tooltip(pos, needle, |window, events| {
            window.pitch(version, tactics, view, roster, events).0
        })
    }

    /// The tooltip drawn while the pointer rests at `pos` containing
    /// `needle` (bounded), `frame` drawing the widgets under test with
    /// the events it is given and answering the frame's nodes. The
    /// pointer first moves away for a frame: egui refuses a tooltip
    /// while the last click is more recent than the last movement.
    pub(crate) fn tooltip(
        &mut self,
        pos: egui::Pos2,
        needle: &str,
        mut frame: impl FnMut(&mut Headless, Vec<egui::Event>) -> Vec<accesskit::Node>,
    ) -> Option<String> {
        frame(self, vec![egui::Event::PointerMoved(egui::Pos2::ZERO)]);
        frame(self, vec![egui::Event::PointerMoved(pos)]);
        for _ in 0..90 {
            let nodes = frame(self, vec![]);
            if let Some(value) = nodes.iter().find_map(|node| {
                node.value()
                    .filter(|value| value.contains(needle))
                    .map(str::to_owned)
            }) {
                return Some(value);
            }
        }
        None
    }
}

/// `a` within `eps` of `b`, both coordinates.
fn near(a: egui::Pos2, b: egui::Pos2, eps: f32) {
    assert!(
        (a.x - b.x).abs() <= eps && (a.y - b.y).abs() <= eps,
        "{a:?} vs {b:?}"
    );
}

/// A press at `press` on `slot`'s marker: its offset from his badge
/// centre, which a held marker keeps between him and the pointer (he is
/// released at the pointer less it).
fn press_offset(canvas: egui::Rect, slot: FormationSlot, press: egui::Pos2) -> egui::Vec2 {
    press - badge_centre(canvas.min, slot)
}

/// Where an outfield `slot`'s marker pressed at `press` is painted while
/// held with the pointer at `pointer`: his badge centre at the spot under
/// the pointer less the press's offset, clamped like every marker's.
fn held_centre(
    canvas: egui::Rect,
    slot: FormationSlot,
    press: egui::Pos2,
    pointer: egui::Pos2,
) -> egui::Pos2 {
    let (x, y) = snap(pointer - press_offset(canvas, slot, press), canvas.min);
    clamp_centre(
        badge_centre(canvas.min, FormationSlot { x, y, ..slot }),
        canvas,
    )
}

/// The pitch canvas' rect (the "pitch" node's).
pub(crate) fn canvas_rect(nodes: &[accesskit::Node]) -> egui::Rect {
    let node = nodes
        .iter()
        .find(|node| node.role() == Role::Unknown && node.label() == Some("pitch"))
        .expect("the pitch canvas");
    let bounds = node.bounds().expect("canvas bounds");
    egui::Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// The clamped badge centres of `players`' markers on `canvas`.
fn centres_of(canvas: egui::Rect, players: &[FormationSlot; 11]) -> [egui::Pos2; 11] {
    players.map(|slot| clamp_centre(badge_centre(canvas.min, slot), canvas))
}

/// `index`'s marker's position in the draw order of `players` on
/// `canvas`: a marker after it wins an overlap.
fn draw_position(canvas: egui::Rect, players: &[FormationSlot; 11], index: usize) -> usize {
    draw_order(&centres_of(canvas, players))
        .iter()
        .position(|&i| i == index)
        .expect("every marker is in the order")
}

/// A point inside `index`'s marker box that no marker drawn after it
/// covers, so a press or a drop there resolves to `index`: the box's
/// centre when free, else the first free of a coarse scan.
pub(crate) fn marker_point(
    nodes: &[accesskit::Node],
    players: &[FormationSlot; 11],
    index: usize,
) -> egui::Pos2 {
    let canvas = canvas_rect(nodes);
    let centres = centres_of(canvas, players);
    let after = draw_position(canvas, players, index);
    let later: Vec<usize> = draw_order(&centres)[after + 1..].to_vec();
    let free = |point: egui::Pos2| {
        later
            .iter()
            .all(|&other| !marker_rect(centres[other], canvas).contains(point))
    };
    let rect = marker_rect(centres[index], canvas);
    if free(rect.center()) {
        return rect.center();
    }
    (0..=10)
        .flat_map(|ix| {
            (0..=15).map(move |iy| {
                rect.min
                    + egui::vec2(
                        ix as f32 * rect.width() / 10.0,
                        iy as f32 * rect.height() / 15.0,
                    )
            })
        })
        .find(|point| free(*point))
        .unwrap_or_else(|| panic!("no point on marker {index} is free of later markers"))
}

/// A point on `canvas` no marker of `players` covers and not inside
/// `blocked` (the open popover's rect — it floats over the grass, so a
/// press meant for the grass has to land outside it): the first free of
/// the grid scan.
fn grass_point(
    canvas: egui::Rect,
    players: &[FormationSlot; 11],
    blocked: egui::Rect,
) -> egui::Pos2 {
    let free = |point: egui::Pos2| {
        !blocked.contains(point)
            && players
                .iter()
                .all(|slot| !marker_rect(badge_centre(canvas.min, *slot), canvas).contains(point))
    };
    (0..=GRID_X)
        .flat_map(|x| (0..=GRID_Y).map(move |y| (x, y)))
        .map(|(x, y)| badge_centre(canvas.min, FormationSlot { position: 1, x, y }))
        .find(|point| free(*point))
        .expect("some grid point is free of markers")
}

/// The lineup rows' label triples (≡ handle, position, name) in draw
/// order: the 11 starting-eleven rows, then the bench rows. The lists
/// sit side by side, so the order is a column's top-to-bottom then the
/// next's (x, then y — the old stacked lists' order holds either way).
/// Rows are found by index, not by name — the fixture's PES 21 players
/// all read "PLACEHOLDER".
pub(crate) fn lineup_rows(nodes: &[accesskit::Node]) -> Vec<Vec<&accesskit::Node>> {
    let mut handles: Vec<&accesskit::Node> = nodes
        .iter()
        .filter(|node| node.role() == Role::Label && node.value() == Some("≡"))
        .collect();
    handles.sort_by(|a, b| {
        let key = |node: &&accesskit::Node| {
            let bounds = node.bounds().expect("handle bounds");
            (bounds.x0, bounds.y0)
        };
        key(a).partial_cmp(&key(b)).expect("ordered bounds")
    });
    handles
        .iter()
        .map(|handle| {
            let bounds = handle.bounds().expect("handle bounds");
            // The row's own three: the labels on its band right of the
            // handle — the other list's row shares the band's y.
            let mut row: Vec<&accesskit::Node> = nodes
                .iter()
                .filter(|node| {
                    node.role() == Role::Label
                        && node.bounds().is_some_and(|b| {
                            (b.y0 - bounds.y0).abs() < 4.0 && b.x0 >= bounds.x0 - 1.0
                        })
                })
                .collect();
            row.sort_by(|a, b| {
                let x = |node: &&accesskit::Node| node.bounds().expect("bounds").x0;
                x(a).partial_cmp(&x(b)).expect("ordered bounds")
            });
            row.truncate(3);
            row
        })
        .collect()
}

/// The centre of lineup `row`'s name label (the `index`th of
/// `lineup_rows`: 0 to 10 the eleven, 11 on the bench), where a drag
/// lands.
pub(crate) fn row_centre(nodes: &[accesskit::Node], index: usize) -> egui::Pos2 {
    let rows = lineup_rows(nodes);
    centre(
        rows.get(index)
            .and_then(|row| row.last())
            .copied()
            .unwrap_or_else(|| panic!("no lineup row {index}")),
    )
}

/// The position column's text of lineup `row`.
pub(crate) fn position_of_row(nodes: &[accesskit::Node], index: usize) -> String {
    let rows = lineup_rows(nodes);
    rows.get(index)
        .and_then(|row| row.get(1))
        .and_then(|node| node.value())
        .unwrap_or_else(|| panic!("no lineup row {index}"))
        .to_owned()
}

/// `label`'s position byte (its index in `POSITIONS`): "GK" to "CF".
fn position_byte(label: &str) -> u8 {
    POSITIONS
        .iter()
        .position(|&name| name == label)
        .and_then(|index| u8::try_from(index).ok())
        .unwrap_or_else(|| panic!("{label:?} is no position label"))
}

/// A formation of the eleven `(position, x, y)` slots, as the record
/// stores them.
fn formation_of(slots: [(&str, u8, u8); 11]) -> Formation {
    Formation {
        players: slots.map(|(position, x, y)| FormationSlot {
            position: position_byte(position),
            x,
            y,
        }),
    }
}

/// Team 701's preset 1 formation 1.
fn formation_701_1() -> Formation {
    formation_of([
        ("GK", 52, 3),
        ("CB", 64, 8),
        ("CB", 40, 8),
        ("RB", 88, 10),
        ("LB", 16, 10),
        ("DMF", 52, 18),
        ("CMF", 75, 26),
        ("CMF", 28, 26),
        ("AMF", 52, 34),
        ("CF", 38, 42),
        ("SS", 66, 42),
    ])
}

/// Team 718's preset 1 formation 1.
fn formation_718_1() -> Formation {
    formation_of([
        ("GK", 52, 3),
        ("SS", 69, 35),
        ("CMF", 53, 26),
        ("DMF", 66, 17),
        ("CB", 32, 6),
        ("CB", 72, 6),
        ("CB", 52, 7),
        ("RMF", 84, 27),
        ("LMF", 20, 26),
        ("DMF", 42, 17),
        ("CF", 42, 41),
    ])
}

/// PARTY HARD's thirteen overalls (GK to CF), team 718's LMF in slot 8
/// of its preset 1 formation 1: the editor's formula on his record, each
/// within 1 of those the game showed.
const PARTY_HARD: [u8; 13] = [76, 85, 88, 88, 88, 84, 95, 82, 85, 86, 82, 84, 84];

/// Team 718's preset 1 formation 1 on `tactics`' preset 1 Kick-off, and
/// PARTY HARD, its LMF in slot 8, on his `roster` row: his overalls and
/// the two positions he is rated A at, LMF and LWF.
pub(crate) fn party_hard(tactics: &mut TeamTactics, roster: &mut [RosterRow<'_>]) {
    tactics.presets[0].formations[0] = formation_718_1();
    let row = &mut roster[usize::from(tactics.starting_eleven[8])];
    row.overalls = Some(PARTY_HARD);
    row.rated[usize::from(position_byte("LMF"))] = true;
    row.rated[usize::from(position_byte("LWF"))] = true;
}

/// The PES 17 fixture save's team 702 (/a/): its tactics, and the
/// thirteen overalls (GK to CF) of COWBOY BEBOP (70223), its CF in slot 10
/// of preset 1's Kick-off at (40, 45), by the editor's formula on his
/// record (he is rated SS A and CF A).
fn team_702() -> (TeamTactics, [u8; 13]) {
    let file = pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes17))
        .expect("fixture save");
    let tactics = file
        .team(702)
        .expect("team 702 is in the fixture")
        .tactics
        .clone();
    let player = file.player(70223).expect("COWBOY BEBOP is in the fixture");
    let overalls = std::array::from_fn(|position| {
        player
            .overall_at(u8::try_from(position).expect("thirteen positions"))
            .expect("a position below 13 has an overall")
    });
    (tactics, overalls)
}

/// Team 718's preset 1 formation 3.
fn formation_718_3() -> Formation {
    formation_of([
        ("GK", 52, 3),
        ("CMF", 63, 26),
        ("CMF", 43, 26),
        ("DMF", 65, 16),
        ("CB", 32, 6),
        ("CB", 72, 6),
        ("CB", 52, 7),
        ("RMF", 84, 27),
        ("LMF", 20, 26),
        ("DMF", 42, 16),
        ("CF", 43, 40),
    ])
}
