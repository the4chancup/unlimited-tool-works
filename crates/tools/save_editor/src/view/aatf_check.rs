//! The AATF check behind the Live AATF dock (`live_aatf`) and the
//! Multi-team AATF dialog (`multi_aatf`): the
//! worker-thread check's key, pending state and spawn, the team the dock
//! reads, the `violations_list` rows a check's results draw, and a row
//! click's effect on the view's selection. The check runs on a worker
//! thread, so a slow custom check never stalls a frame, started when the
//! key moves on (the session's revision, the team ids or the ruleset) and
//! the edit has settled (`edit_settled`, the same condition the undo
//! stack's seal uses); until the reply lands the last result stays, marked
//! as checking, unless it was checked under another ruleset
//! (`held_result`).

use std::collections::HashMap;
use std::sync::Arc;

use pes_savefile::EditFile;
use pes_savefile::model::PlayerEntry;
use pes_savefile::model::names::display_name;
use pes_savefile::model::tactics::TeamTactics;
use team_widgets::tactics::TacticsView;
use team_widgets::violations::{
    ListUse, ListedPlayer, SelectedPlayer, TeamViolations, Wording, violations_list,
};

use crate::session::rules::{self, TeamCheck};
use crate::session::selection::TeamChoice;
use crate::view::Selection;
use crate::view::worker::{Pending, WorkerResult};

/// What a check ran for: a change of the open save, of the ids or of the
/// ruleset recomputes it.
#[derive(Clone, PartialEq)]
pub(super) struct CheckKey {
    /// `Session::revision` when the views were built.
    pub(super) revision: u64,
    /// The teams checked, in id order.
    pub(super) ids: Vec<u32>,
    /// The ruleset the check ran under (`ActiveRules::key`).
    pub(super) ruleset: String,
    /// Whether the check asks for the suggestions: unasked, the reply
    /// holds none.
    pub(super) suggestions: bool,
}

/// A check in flight: the key it runs for and its reply channel (the
/// worker's reply is its key and the checks, in `ids`' order).
pub(super) type PendingCheck = Pending<CheckKey, Vec<TeamCheck>>;

/// A team check's state: the last result with its key and the check in
/// flight, if one is (`worker::WorkerResult`'s poll-and-spawn cycle). The
/// Live AATF dock and the Multi-team AATF dialog hold one each.
pub(super) type TeamsCheck = WorkerResult<CheckKey, Vec<TeamCheck>>;

/// The held result the views draw and copy under the ruleset keyed
/// `ruleset` (`ActiveRules::key`): `None` when it was checked under
/// another one. A result is worded with the ruleset it is drawn with, so
/// one from another ruleset would name tiers and medal systems its
/// findings were never checked against; a result held across an edit or a
/// range change stays, marked as checking.
pub(super) fn held_result<'a>(
    check: &'a TeamsCheck,
    ruleset: &str,
) -> Option<&'a (CheckKey, Vec<TeamCheck>)> {
    check
        .result
        .as_ref()
        .filter(|(key, _)| key.ruleset == ruleset)
}

/// One frame of the check cycle for `key`: the reply polled first, so
/// the frame that starts a check still marks the previous result as
/// checking; then a check spawned when the key moved on since the last
/// result and the edit has settled. Returns whether the list is checking
/// — a check in flight or due at the next settled frame — for the
/// caller's checking mark.
pub(super) fn update_check(
    check: &mut TeamsCheck,
    ctx: &egui::Context,
    file: &EditFile,
    rules: &Arc<aatf::Rules>,
    key: CheckKey,
    settled: bool,
) -> bool {
    check.update(&key, !key.ids.is_empty(), settled, |key| {
        spawn_check(ctx, file, rules, key.clone())
    })
}

/// The team the dock checks: the selected row's (a player
/// row's team or a team row's own — the team the view is editing, another
/// team's too when the filter lists its rows), else the selector's;
/// `None` with no row selected under ALL.
pub(super) fn current_team(team: TeamChoice, selected: Option<Selection>) -> Option<u32> {
    match selected {
        Some(Selection::Team(id)) | Some(Selection::Player(id, _)) => Some(id),
        None => match team {
            TeamChoice::Team(id) => Some(id),
            TeamChoice::All => None,
        },
    }
}

/// A violation row's click: the player selected and the selector moved to
/// his team unless it reads ALL (the list must show the row it selects).
/// A `FieldedNotA` row selects his team's row instead (the pitch his
/// cross is on is the team view's), the named preset shown on the first
/// of its fielded formations whose slot for his starting-eleven index
/// holds the violation's position — the kick-off one when none does (the save
/// changed since the check). `team_tactics` is his team's record,
/// `tactics_view` the view's.
pub(super) fn select_player(
    player: SelectedPlayer,
    selected: &mut Option<Selection>,
    team: &mut TeamChoice,
    filter: &mut String,
    team_tactics: &TeamTactics,
    tactics_view: &mut TacticsView,
) {
    *selected = match player.fielded {
        Some(fielded) => {
            tactics_view.preset = usize::from(fielded.preset).saturating_sub(1).min(2);
            let preset = &team_tactics.presets[tactics_view.preset];
            tactics_view.formation = team_tactics
                .starting_eleven
                .iter()
                .position(|slot| *slot == player.slot)
                .and_then(|index| {
                    preset
                        .fielded_formations()
                        .iter()
                        .copied()
                        .find(|&formation| {
                            preset.formations[formation].players[index].position
                                == fielded.position.byte()
                        })
                })
                .unwrap_or(0);
            Some(Selection::Team(player.team_id))
        }
        None => Some(Selection::Player(player.team_id, player.slot)),
    };
    if *team != TeamChoice::All {
        *team = TeamChoice::Team(player.team_id);
    }
    // As `open` clears it: the row the click selects would be hidden by a
    // filter matching nothing.
    filter.clear();
}

/// The issue count the callers draw: the Error and Warning violations of
/// the checks that succeeded (suggestions are not issues; a failed check's
/// violations are unknown and count nothing — the body says why).
pub(super) fn issues(checks: &[TeamCheck]) -> usize {
    checks
        .iter()
        .filter_map(|check| check.as_ref().ok())
        .flatten()
        .filter(|violation| {
            matches!(
                violation.severity,
                aatf::Severity::Error | aatf::Severity::Warning
            )
        })
        .count()
}

/// Team `id`'s name (every checked id comes from the open file's teams,
/// the selector or the selection).
pub(super) fn team_name(file: &EditFile, id: u32) -> &str {
    file.team(id)
        .expect("every checked id is a team of the open file")
        .name
        .as_str()
}

/// The check's results: one error-coloured line per team whose check
/// failed (as `check-aatf` reports it), then the `violations_list` itself,
/// its rows worded under `ruleset` (the active one) and `file`'s version,
/// and drawn for `list_use`. Returns the player whose row was clicked this
/// frame.
pub(super) fn results(
    ui: &mut egui::Ui,
    file: &EditFile,
    id_salt: impl egui::AsIdSalt,
    key: &CheckKey,
    checks: &[TeamCheck],
    ruleset: &aatf::Ruleset,
    list_use: ListUse,
) -> Option<SelectedPlayer> {
    // Player ids are stable inside an open file: one map serves the
    // whole roster scan, as `selection::rows` builds its own (its
    // construction lives there inline).
    let players: HashMap<u32, &PlayerEntry> = file
        .players()
        .iter()
        .map(|player| (player.id, player))
        .collect();
    // One `ListedPlayer` list per checked team, for the group headers;
    // `teams` borrows it below, so it is filled first.
    let mut listed: Vec<Vec<ListedPlayer>> = Vec::with_capacity(key.ids.len());
    for (id, check) in key.ids.iter().zip(checks) {
        listed.push(if check.is_ok() {
            listed_players(file, &players, *id)
        } else {
            Vec::new()
        });
    }
    let mut teams = Vec::new();
    for (i, (id, check)) in key.ids.iter().zip(checks).enumerate() {
        let name = team_name(file, *id);
        match check {
            Err(error) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!(
                        "The AATF check failed for team {id} {}: {error}",
                        display_name(name)
                    ),
                );
            }
            Ok(violations) => teams.push(TeamViolations {
                team_id: *id,
                team_name: name,
                players: &listed[i],
                violations,
            }),
        }
    }
    let wording = Wording {
        ruleset,
        version: file.version(),
    };
    violations_list(ui, id_salt, &teams, wording, list_use)
}

/// Team `id`'s rostered players as the list's `ListedPlayer`s, in slot
/// order, their names read through `players`.
fn listed_players<'a>(
    file: &'a EditFile,
    players: &HashMap<u32, &'a PlayerEntry>,
    id: u32,
) -> Vec<ListedPlayer<'a>> {
    let Some(team) = file.team(id) else {
        return Vec::new();
    };
    team.roster
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.player_id != 0)
        .map(|(slot, roster)| ListedPlayer {
            slot: u8::try_from(slot).expect("a roster slot"),
            id: roster.player_id,
            number: roster.number,
            name: players
                .get(&roster.player_id)
                .map(|player| player.name.as_str())
                .unwrap_or("?"),
        })
        .collect()
}

/// One check's worker: `key`'s teams viewed on the caller's thread (the
/// `EditFile` lives there), then `check_teams` on the shared ruleset off
/// it, the key riding the reply so the caller knows what landed.
fn spawn_check(
    ctx: &egui::Context,
    file: &EditFile,
    rules: &Arc<aatf::Rules>,
    key: CheckKey,
) -> PendingCheck {
    let views = rules::team_views(file, &key.ids);
    let (sender, replies) = crossbeam_channel::unbounded();
    let rules = Arc::clone(rules);
    let ctx = ctx.clone();
    let reply_key = key.clone();
    std::thread::spawn(move || {
        let mut checks = rules::check_teams(&rules, &views);
        // Unasked, suggestions would tint the whole list blue: most
        // players of a cup team leave some allowance unused.
        if !reply_key.suggestions {
            for violations in checks.iter_mut().flatten() {
                violations.retain(|violation| violation.severity != aatf::Severity::Suggestion);
            }
        }
        // A newer check can replace the receiver mid-flight; that send's
        // failure is the superseded thread ending, not an error.
        if sender.send((reply_key, checks)).is_err() {
            log::debug!("the AATF check's caller dropped a superseded check");
        }
        ctx.request_repaint();
    });
    Pending { key, replies }
}
