//! The Savefile menu's Multi-team AATF dialog, the draft's `dlgMultiAatf`:
//! an organizer's check of a range of the save's teams under the toolbar's
//! ruleset, a report read once and copied out, while Live AATF is where a
//! team gets fixed. The range's two ends are picked by team on one row, the
//! save's first and last team by default, and kept while the dialog is
//! closed and opened again on the same save. The teams are checked in
//! parallel on the AATF check's worker (`aatf_check`) whenever the range or
//! the ruleset changes, the last list kept and marked as checking until the
//! new one lands. Copy to clipboard writes the list as plain text
//! (`copy_text`) for a cup's thread.

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{modal, modal_buttons, step_choice, wheel_steps};
use studio_core::{Notice, NoticeKind};
use team_widgets::violations::{ListUse, Wording, issue_groups, rule_name, sentence};

use crate::session::rules::TeamCheck;
use crate::view::SaveView;
use crate::view::aatf_check::{self, CheckKey, TeamsCheck};

/// The list's outer width, the draft's: as wide as a dialog's content gets
/// (its 640-point window less the padding and the border).
const LIST_WIDTH: f32 = 614.0;

/// The list's outer height, the draft's; a longer list scrolls.
const LIST_HEIGHT: f32 = 360.0;

/// The list frame's margins inside its border, the draft's `.live-body`
/// padding.
const LIST_MARGIN: egui::Margin = egui::Margin::symmetric(6, 2);

/// The dialog's state on `SaveView` (`view.multi_aatf`).
pub(super) struct MultiAatf {
    /// The dialog is up.
    open: bool,
    /// The range's first and last team ids as picked; `None` until the
    /// dialog first opens on the open save.
    range: Option<(u32, u32)>,
    /// The range's check.
    pub(super) check: TeamsCheck,
}

impl MultiAatf {
    /// Closed, no range picked, nothing checked.
    pub(super) fn new() -> MultiAatf {
        MultiAatf {
            open: false,
            range: None,
            check: TeamsCheck::new(),
        }
    }

    /// Another save opened: the range and the check were the old save's.
    pub(super) fn reset(&mut self) {
        *self = MultiAatf::new();
    }

    /// The Savefile menu's entry: the dialog up, its range the save's first
    /// and last team the first time it opens on `file`.
    pub(super) fn open(&mut self, file: &EditFile) {
        if self.range.is_none() {
            let ids = file.teams().iter().map(|team| team.id);
            self.range = ids.clone().min().zip(ids.max());
        }
        self.open = true;
    }
}

/// `n` and `word`, plural unless `n` is 1 (`1 error`, `3 warnings`).
fn counted(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The teams of `file` whose id lies between `first` and `last`, in id
/// order; none when `first` comes after `last`.
fn range_ids(file: &EditFile, first: u32, last: u32) -> Vec<u32> {
    let mut ids: Vec<u32> = file
        .teams()
        .iter()
        .map(|team| team.id)
        .filter(|id| (first..=last).contains(id))
        .collect();
    ids.sort_unstable();
    ids
}

/// Team `id` as the combos and the list name it: its id, then its name
/// without colour codes.
fn team_label(file: &EditFile, id: u32) -> String {
    format!("{id} {}", display_name(aatf_check::team_name(file, id)))
}

/// One end of the range: a combo of the save's teams in the save's order,
/// as wide as its widest label so a pick never moves the other combo.
fn team_combo(ui: &mut egui::Ui, id_salt: &str, file: &EditFile, picked: &mut u32) {
    let labels: Vec<(u32, String)> = file
        .teams()
        .iter()
        .map(|team| (team.id, team_label(file, team.id)))
        .collect();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widest = labels
        .iter()
        .map(|(_, label)| {
            ui.painter()
                .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    // The closed combo's chrome around its text: the button's padding on
    // both sides, the gap and the arrow.
    let chrome = {
        let spacing = ui.spacing();
        2.0 * spacing.button_padding.x + spacing.icon_spacing + spacing.icon_width
    };
    let combo = egui::ComboBox::from_id_salt(id_salt)
        .selected_text(team_label(file, *picked))
        .width(widest + chrome)
        .show_ui(ui, |ui| {
            for (id, label) in &labels {
                ui.selectable_value(picked, *id, label);
            }
        })
        .response;
    let teams = labels.iter().map(|(id, _)| *id);
    if let Some(next) = step_choice(teams, picked, wheel_steps(ui, &combo)) {
        *picked = next;
    }
}

/// The line under the range: how many teams the list holds and how many
/// of them have an error or a warning.
fn summary(checks: &[TeamCheck]) -> String {
    let with_issues = checks
        .iter()
        .filter(|check| aatf_check::issues(std::slice::from_ref(check)) > 0)
        .count();
    format!(
        "{} checked, {with_issues} with issues",
        counted(checks.len(), "team")
    )
}

/// The word a row's severity is written with in the copied text.
fn severity_word(severity: aatf::Severity) -> &'static str {
    match severity {
        aatf::Severity::Error => "Error",
        aatf::Severity::Warning => "Warning",
        aatf::Severity::Suggestion => "Suggestion",
    }
}

/// The group line over a team's rows in the copied text: the player's id
/// and name, `Slot <n>` for a slot whose player the save does not hold, or
/// `Team-wide issues` for the team's own rows.
fn group_line(file: &EditFile, team_id: u32, slot: Option<u8>) -> String {
    let Some(slot) = slot else {
        return "Team-wide issues".to_owned();
    };
    file.team(team_id)
        .and_then(|team| team.roster.get(usize::from(slot)))
        .and_then(|roster| file.player(roster.player_id))
        .map_or_else(
            || format!("Slot {slot}"),
            |player| format!("{} {}", player.id, display_name(&player.name)),
        )
}

/// The list as plain text, a line per row, the draft's `multiAatfText`:
/// first the ruleset's `description`, the range and the error and warning
/// counts; then each team's id and name after a `>`, which a cup's thread
/// draws as green text so each team stands out; under it `All clean`, or
/// each player's id and name (the team's own rows under `Team-wide
/// issues`) indented two spaces, his rows' severity, rule and sentence
/// (worded under `ruleset`, the active one, and `file`'s version) indented
/// four. A team whose
/// check failed has the error under its line.
fn copy_text(
    file: &EditFile,
    ruleset: &aatf::Ruleset,
    key: &CheckKey,
    checks: &[TeamCheck],
) -> String {
    let description = &ruleset.name;
    let wording = Wording {
        ruleset,
        version: file.version(),
    };
    let violations: Vec<&aatf::Violation> = checks
        .iter()
        .filter_map(|check| check.as_ref().ok())
        .flatten()
        .collect();
    let count = |severity: aatf::Severity| {
        violations
            .iter()
            .filter(|violation| violation.severity == severity)
            .count()
    };
    let first = key.ids.first().expect("a held result checked a team");
    let last = key.ids.last().expect("a held result checked a team");
    let mut lines = vec![format!(
        "{description}, teams {first} to {last}: {}, {}",
        counted(count(aatf::Severity::Error), "error"),
        counted(count(aatf::Severity::Warning), "warning")
    )];
    for (&id, check) in key.ids.iter().zip(checks) {
        lines.push(format!(">{}", team_label(file, id)));
        let violations = match check {
            Ok(violations) => violations,
            Err(error) => {
                lines.push(format!("  The AATF check failed: {error}"));
                continue;
            }
        };
        let groups = issue_groups(violations);
        if groups.is_empty() {
            lines.push("  All clean".to_owned());
        }
        for (slot, rows) in groups {
            lines.push(format!("  {}", group_line(file, id, slot)));
            for violation in rows {
                lines.push(format!(
                    "    {}  {}  {}",
                    severity_word(violation.severity),
                    rule_name(&violation.kind),
                    sentence(violation, id, wording)
                ));
            }
        }
    }
    lines.join("\n")
}

/// The dialog for one frame while it is up: the title naming the ruleset,
/// the range's row, the summary line (or the reversed range's warning),
/// the list, then Copy to clipboard and Close.
pub(super) fn dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        multi_aatf: multi,
        session,
        rules,
        notice,
        ..
    } = view;
    if !multi.open {
        return;
    }
    let (Some(session), Some(active), Some((mut first, mut last))) =
        (session.as_ref(), rules.as_ref(), multi.range)
    else {
        return;
    };
    let file = session.file();
    let description = &active.rules.ruleset().name;
    let reversed = first > last;
    let key = CheckKey {
        revision: session.revision(),
        ids: range_ids(file, first, last),
        ruleset: active.key.clone(),
        suggestions: false,
    };
    // Settled: a modal is up, so no edit can be under way.
    let checking =
        aatf_check::update_check(&mut multi.check, ui.ctx(), file, &active.rules, key, true);
    let listed = multi.check.result.as_ref().filter(|_| !reversed);
    let mut close = false;
    let id = egui::Id::unique("save_editor_multi_aatf");
    let title = format!("Multi-team AATF ({description})");
    let response = modal(ui.ctx(), id, title, |ui| {
        ui.horizontal(|ui| {
            ui.weak("Teams");
            team_combo(ui, "save_editor_multi_first", file, &mut first);
            ui.weak("to");
            team_combo(ui, "save_editor_multi_last", file, &mut last);
        });
        ui.horizontal(|ui| {
            if reversed {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "The first team comes after the last",
                );
                return;
            }
            if let Some((_, checks)) = listed {
                ui.weak(summary(checks));
            }
            if checking {
                ui.weak("checking\u{2026}");
            }
        });
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let chrome = LIST_MARGIN.sum() + egui::Vec2::splat(2.0 * stroke.width);
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(stroke)
            .inner_margin(LIST_MARGIN)
            .show(ui, |ui| {
                ui.set_width(LIST_WIDTH - chrome.x);
                // egui's solid scroll bar stands beside the rows, as
                // the draft's `.live-body` bar does: a floating one lies
                // over each row's right end and its last words.
                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                egui::ScrollArea::vertical()
                    .id_salt("save_editor_multi_rows")
                    .auto_shrink([false, false])
                    .max_height(LIST_HEIGHT - chrome.y)
                    .show(ui, |ui| {
                        if let Some((key, checks)) = listed {
                            // A report: its rows select nothing, so the
                            // click it returns is always `None`.
                            aatf_check::results(
                                ui,
                                file,
                                "aatf_multi",
                                key,
                                checks,
                                active.rules.ruleset(),
                                ListUse::Report,
                            );
                        }
                    });
            });
        modal_buttons(ui, |ui| {
            if ui.button("Close").clicked() {
                close = true;
            }
            if ui
                .add_enabled(listed.is_some(), egui::Button::new("Copy to clipboard"))
                .clicked()
                && let Some((key, checks)) = listed
            {
                ui.ctx()
                    .copy_text(copy_text(file, active.rules.ruleset(), key, checks));
                *notice = Some(Notice {
                    text: format!("Copied the results of {}.", counted(checks.len(), "team")),
                    kind: NoticeKind::Success,
                    action: None,
                });
            }
        });
    });
    multi.range = Some((first, last));
    if close || response.should_close() {
        multi.open = false;
    }
}

#[cfg(test)]
mod tests {
    use pes_savefile::interchange::team_toml::TeamToml;
    use pes_version::PesVersion;

    use super::*;
    use crate::session::Session;
    use crate::session::rules;

    /// The PES 15 sample save with the CLI's `official_legal.toml` imported
    /// onto 714 /co/, which the official ruleset (4cc Autumn 26) then finds
    /// clean, and 718 /f/'s slot-0 player, 71801 ROADTRIP, a goalkeeper
    /// moved from 189 to 183 cm, which no height bracket takes.
    fn legal_714_and_short_71801(test: &str) -> Session {
        let path = crate::test_support::scratch(test).join("EDIT00000000");
        std::fs::write(&path, pes_savefile::fixture_save(PesVersion::Pes15)).unwrap();
        let mut session = Session::open(&path).unwrap();
        let toml = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/aatf/official_legal.toml"
        ))
        .unwrap();
        session
            .import_team(&TeamToml::parse(&toml).unwrap())
            .expect("the legal team imports");
        let mut entry = session.file().player(71801).unwrap().clone();
        assert_eq!(entry.basic.height, 189, "premise: 71801 is 189 cm");
        entry.basic.height = 183;
        session.edit_players(vec![entry]);
        session
    }

    /// The copied text of 714 /co/ (clean) and 718 /f/ (a player's rows and
    /// the team's own) under 4cc Autumn 26: the header with the counts, each
    /// team's line after a `>`, `All clean` and the player's and the team's
    /// lines indented two spaces, each row four.
    #[test]
    fn the_copied_text_lists_each_team_after_a_marker_and_its_rows_indented() {
        let session = legal_714_and_short_71801("multi_aatf_copy_text");
        let file = session.file();
        let choice = rules::resolve("autumn26", &[]);
        let key = CheckKey {
            revision: session.revision(),
            ids: vec![714, 718],
            ruleset: choice.key,
            suggestions: false,
        };
        let checks = rules::check_teams(&choice.rules, &rules::team_views(file, &key.ids));
        let text = copy_text(file, choice.rules.ruleset(), &key, &checks);
        assert_eq!(
            text.lines().collect::<Vec<_>>(),
            [
                "4cc Autumn 26, teams 714 to 718: 6 errors, 0 warnings",
                ">714 /co/",
                "  All clean",
                ">718 /f/",
                "  71801 ROADTRIP",
                "    Error  Height  height 183 takes no bracket (185 has room)",
                "    Error  Special rule  Tall goalkeepers are 189: GKs in this bracket must be 189cm",
                "  Team-wide issues",
                "    Error  Tier count  2 Gold players, where the rules want 1",
                "    Error  Tier count  0 Bronze players, where the rules want 2",
                "    Error  Tier count  19 Regular players, where the rules want 18",
                "    Error  Height quota  9 players in the Tall bracket of the Red system, where its quota is 10",
            ]
        );
    }

    /// A count reads in the singular at 1 and in the plural otherwise.
    #[test]
    fn a_count_is_singular_at_one() {
        assert_eq!(counted(0, "error"), "0 errors");
        assert_eq!(counted(1, "warning"), "1 warning");
        assert_eq!(counted(2, "team"), "2 teams");
    }
}
