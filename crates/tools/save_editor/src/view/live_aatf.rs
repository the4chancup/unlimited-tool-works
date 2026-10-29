//! The Live AATF dock, the draft's `livePanel`: a `BottomDock` at the
//! bottom of the view listing the open team's violations under
//! `N issues · <ruleset>`, re-checked on the shared worker (`aatf_check`)
//! after every settled edit so the list shrinks as the member fixes them;
//! a row's click selects the player. The toolbar's toggle is the
//! `live_aatf` setting (`view.live_aatf`); off, nothing is drawn and no
//! check starts. Its Show suggestions checkbox is `live_suggestions`: on,
//! the check asks for the suggestions too, listed under `Suggestions`
//! after the violations.

use studio_core::widgets::{BottomDock, well_frame};
use team_widgets::violations::ListUse;

use crate::view::aatf_check::{self, CheckKey, TeamsCheck};
use crate::view::{SaveView, edit_settled};

/// The dock's state on `SaveView` (`view.live`): the dock's frame and the
/// open team's check.
pub(super) struct LiveAatf {
    /// The bottom dock's frame: the chevron, the title and the height.
    dock: BottomDock,
    /// The open team's check.
    pub(super) check: TeamsCheck,
}

impl LiveAatf {
    /// The dock, open, nothing checked yet; its body opens 150 points high
    /// (the draft's `LIVE.height`), lower than the log's, since the views
    /// above it need the height more.
    pub(super) fn new() -> LiveAatf {
        LiveAatf {
            dock: BottomDock::new(
                egui::Id::unique("save_editor_live_aatf"),
                "Live AATF",
                "Live AATF",
            )
            .body_height(150.0),
            check: TeamsCheck::new(),
        }
    }
}

/// One frame of the dock, while `view.live_aatf` and a save is open: the
/// open team's check polled and re-spawned for the session's current
/// revision and ruleset once the edit settles, the header counting the
/// held result's Error and Warning violations (the draft's `livePanel`
/// wording) and the body listing them through the shared `results`.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        live,
        live_suggestions,
        session,
        team,
        filter,
        selected,
        rules,
        focused,
        tactics,
        ..
    } = view;
    let (Some(session), Some(active)) = (session.as_ref(), rules.as_ref()) else {
        return;
    };
    let file = session.file();
    let (_, settled) = edit_settled(ui, *focused);
    // The open team: the selected row's team (a player's or a team row's
    // own — the team the view is editing), else the selector's.
    let team_id = aatf_check::current_team(*team, *selected);
    let key = CheckKey {
        revision: session.revision(),
        ids: team_id.into_iter().collect(),
        ruleset: active.key.clone(),
        suggestions: *live_suggestions,
    };
    let checking =
        aatf_check::update_check(&mut live.check, ui.ctx(), file, &active.rules, key, settled);
    let ruleset = &active.rules.ruleset().name;
    live.dock.show(
        ui,
        |ui| {
            // The header after the title (the draft's `livePanel` wording);
            // a failed check draws no count — the body says why.
            match &live.check.result {
                Some((result_key, checks)) => {
                    if checks.iter().all(Result::is_ok) {
                        let issues = aatf_check::issues(checks);
                        if issues == 0 {
                            let name = aatf_check::team_name(
                                file,
                                *result_key
                                    .ids
                                    .first()
                                    .expect("a held result checked a team"),
                            );
                            ui.strong(format!("{name} passes {ruleset} with no issues."));
                        } else {
                            let plural = if issues == 1 { "issue" } else { "issues" };
                            ui.strong(format!("{issues} {plural} · {ruleset}"));
                        }
                    }
                }
                None => {
                    if let Some(id) = team_id {
                        ui.strong(format!(
                            "Checking {}\u{2026}",
                            aatf_check::team_name(file, id)
                        ));
                    }
                }
            }
            if checking && live.check.result.is_some() {
                ui.weak("checking\u{2026}");
            }
        },
        |ui| {
            // The list scrolls inside the dock's well (the draft's
            // `.live-body`): a long one does not grow it. egui's solid
            // scroll bar stands beside the rows, inside the well, as the
            // draft's bar does: a floating one lies over each row's right
            // end, its hover and its sentence's last words.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            let clicked = well_frame(ui.visuals())
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("save_editor_live_results")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let mut clicked = None;
                            if team_id.is_none() {
                                ui.weak("No team is open.");
                            } else if let Some((result_key, checks)) = &live.check.result {
                                clicked = aatf_check::results(
                                    ui,
                                    file,
                                    "aatf_live",
                                    result_key,
                                    checks,
                                    ListUse::Editing {
                                        show_suggestions: result_key.suggestions,
                                    },
                                );
                            }
                            clicked
                        })
                        .inner
                })
                .inner;
            if let Some(player) = clicked {
                // A listed row's team is one of the open file's.
                let team_tactics = file
                    .team(player.team_id)
                    .map(|entry| &entry.tactics)
                    .expect("a listed row's team is in the open file");
                aatf_check::select_player(player, selected, team, filter, team_tactics, tactics);
            }
        },
    );
}
