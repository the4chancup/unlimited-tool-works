//! The Savefile menu's Multi-team AATF dialog, the draft's `dlgMultiAatf`:
//! an organizer's check of a range of the save's teams under the toolbar's
//! ruleset, a report read once and copied out, while Live AATF is where a
//! team gets fixed. The range's two ends are picked by team on one row,
//! both on the save's first team by default, the last never before the
//! first, and kept while the dialog is closed and opened again on the same
//! save. The teams are checked in parallel on the AATF check's worker
//! (`aatf_check`) whenever the ruleset changes, and whenever the range
//! changes once its ends have rested `REST`. A range change keeps the last
//! list, dimmed and marked as checking until the new one lands; a ruleset
//! change drops it, since its words (tier labels, medal systems, the copied
//! text's first line) would be the new ruleset's over findings checked
//! under the old one. Copy to clipboard writes the list as plain text
//! (`copy_text`) for a cup's thread.

use std::time::Duration;

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{
    add_enabled, combo_item, modal, modal_buttons, step_choice, wheel_steps,
};
use studio_core::{Notice, NoticeKind};
use team_widgets::violations::{
    ListUse, ListedPlayer, Wording, issue_groups, rule_name, sentence, violation_detail,
};

use crate::session::rules::{self, TeamCheck};
use crate::view::SaveView;
use crate::view::aatf_check::{self, CheckKey, TeamsCheck};

/// The list's outer width, which sets the dialog's: the draft's 900-point
/// window less the modal's padding and border, as wide as the window's
/// 960-point minimum leaves room for. At the 640 points other dialogs keep, a
/// height row's sentence and its players' lines ran past the list.
const LIST_WIDTH: f32 = 874.0;

/// The list's outer height, the draft's; a longer list scrolls.
const LIST_HEIGHT: f32 = 360.0;

/// The list frame's margins inside its border, the draft's `.live-body`
/// padding.
const LIST_MARGIN: egui::Margin = egui::Margin::symmetric(6, 2);

/// How long, in seconds, the range's ends hold still before the range is
/// checked: the wheel over an end steps one team a frame, and a check
/// started per step left the window unresponsive for about two minutes
/// over 220 teams.
const REST: f64 = 0.5;

/// The list's opacity while it is checking, the draft's
/// `.live-body.checking`.
const CHECKING_OPACITY: f32 = 0.55;

/// The range's ends as picked and as checked.
struct Range {
    /// The first and last team ids as picked, the last never before the
    /// first.
    picked: (u32, u32),
    /// The ends the check is for: `picked` once it has held still for
    /// `REST`.
    checked: (u32, u32),
    /// The input time `picked` last changed at.
    changed_at: f64,
}

/// The dialog's state on `SaveView` (`view.multi_aatf`).
pub(super) struct MultiAatf {
    /// The dialog is up.
    open: bool,
    /// The range; `None` until the dialog first opens on the open save.
    range: Option<Range>,
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

    /// The Savefile menu's entry: the dialog up, both ends of its range on
    /// the save's first team the first time it opens on `file`, checked at
    /// once.
    pub(super) fn open(&mut self, file: &EditFile) {
        if self.range.is_none() {
            let first = file.teams().iter().map(|team| team.id).min();
            self.range = first.map(|first| Range {
                picked: (first, first),
                checked: (first, first),
                changed_at: f64::NEG_INFINITY,
            });
        }
        self.open = true;
    }
}

/// `n` and `word`, plural unless `n` is 1 (`1 error`, `3 warnings`).
pub(super) fn counted(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// The teams of `file` whose id lies between `first` and `last`, in id
/// order.
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
pub(super) fn team_label(file: &EditFile, id: u32) -> String {
    format!("{id} {}", display_name(aatf_check::team_name(file, id)))
}

/// One end of the range: a combo of the save's teams from id `from` on, in
/// id order, the order the range is taken in, as wide as its widest label
/// so a pick never moves the other combo.
fn team_combo(ui: &mut egui::Ui, id_salt: &str, file: &EditFile, from: u32, picked: &mut u32) {
    let mut labels: Vec<(u32, String)> = file
        .teams()
        .iter()
        .filter(|team| team.id >= from)
        .map(|team| (team.id, team_label(file, team.id)))
        .collect();
    labels.sort_unstable_by_key(|(id, _)| *id);
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
                combo_item(ui, picked, *id, label);
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

/// `<kind>: <label> <n>, ...` over `systems`, each a `(label shown, the
/// system's word a check names)`, in `systems`' order, a system no team
/// was read under left out; `None` when the ruleset has fewer than two of
/// the kind (one system leaves no mix to break down) or none counts a
/// team. `pick` reads the kind's field off a check (the medal or the
/// height system); a team whose check failed counts toward none.
fn kind_counts(
    kind: &str,
    systems: &[(&str, &str)],
    checks: &[TeamCheck],
    pick: impl Fn(&aatf::TeamCheck) -> Option<&str>,
) -> Option<String> {
    if systems.len() < 2 {
        return None;
    }
    let named: Vec<String> = systems
        .iter()
        .filter_map(|(label, key)| {
            let n = checks
                .iter()
                .filter(|check| check.as_ref().is_ok_and(|check| pick(check) == Some(*key)))
                .count();
            (n > 0).then(|| format!("{label} {n}"))
        })
        .collect();
    (!named.is_empty()).then(|| format!("{kind}: {}", named.join(", ")))
}

/// The cup's mix (the draft's `multiAatfCounts`): how many of the
/// checked teams were read under each medal system and each height
/// system of `ruleset`, the kinds joined by ` · `, for the line above
/// the list and the copied text's first. `None` when neither kind is
/// left to show.
fn system_counts(ruleset: &aatf::Ruleset, checks: &[TeamCheck]) -> Option<String> {
    // A check names a medal system by its label and a height system by
    // its id; both show the system's label.
    let medals: Vec<(&str, &str)> = ruleset
        .medal_systems
        .iter()
        .map(|system| (system.label.as_str(), system.label.as_str()))
        .collect();
    let heights: Vec<(&str, &str)> = ruleset
        .heights
        .systems
        .iter()
        .map(|system| (system.label.as_str(), system.id.as_str()))
        .collect();
    let parts: Vec<String> = [
        kind_counts("Medal systems", &medals, checks, |check| {
            check.medal_system.as_deref()
        }),
        kind_counts("Height systems", &heights, checks, |check| {
            check.height_system.as_deref()
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
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
/// first the systems' counts when the ruleset leaves any, then the
/// ruleset's `description`, the range and the error and warning
/// counts; then each team's id and name after a `>`, which a cup's thread
/// draws as green text so each team stands out; under it `All clean`, or
/// each player's id and name (the team's own rows under `Team-wide
/// issues`) indented two spaces, his rows' severity, rule and sentence
/// (worded under `ruleset`, the active one, and `file`'s version) indented
/// four, and a height row's lines naming its players (`violation_detail`)
/// indented six under it. A team whose
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
        .flat_map(|check| &check.violations)
        .collect();
    let count = |severity: aatf::Severity| {
        violations
            .iter()
            .filter(|violation| violation.severity == severity)
            .count()
    };
    let first = key.ids.first().expect("a held result checked a team");
    let last = key.ids.last().expect("a held result checked a team");
    let mut lines: Vec<String> = system_counts(ruleset, checks).into_iter().collect();
    lines.push(format!(
        "{description}, teams {first} to {last}: {}, {}",
        counted(count(aatf::Severity::Error), "error"),
        counted(count(aatf::Severity::Warning), "warning")
    ));
    let listed = rules::listed_players(file, &key.ids);
    for ((&id, check), players) in key.ids.iter().zip(checks).zip(&listed) {
        lines.push(format!(">{}", team_label(file, id)));
        let violations = match check {
            Ok(check) => &check.violations,
            Err(error) => {
                lines.push(format!("  The AATF check failed: {error}"));
                continue;
            }
        };
        let groups = issue_groups(violations);
        if groups.is_empty() {
            lines.push("  All clean".to_owned());
        }
        group_lines(file, id, players, &groups, wording, &mut lines);
    }
    lines.join("\n")
}

/// `team_id`'s `groups` as copied lines on `lines`: each group's head
/// (`group_line`) two spaces in, each row's severity, rule and sentence four,
/// and a height row's lines naming its players (`violation_detail`) six.
/// Live AATF's copy words its team's groups with the same lines.
pub(super) fn group_lines(
    file: &EditFile,
    team_id: u32,
    players: &[ListedPlayer<'_>],
    groups: &[(Option<u8>, Vec<&aatf::Violation>)],
    wording: Wording<'_>,
    lines: &mut Vec<String>,
) {
    for (slot, rows) in groups {
        lines.push(format!("  {}", group_line(file, team_id, *slot)));
        for violation in rows {
            lines.push(format!(
                "    {}  {}  {}",
                severity_word(violation.severity),
                rule_name(&violation.kind),
                sentence(violation, team_id, wording)
            ));
            for line in violation_detail(violation, players, wording) {
                lines.push(format!("      {line}"));
            }
        }
    }
}

/// The dialog for one frame while it is up: the title naming the ruleset,
/// the range's row, the summary line, the list, then Copy to clipboard and
/// Close. The list is of the range last checked; ends picked since are
/// checked once they have rested `REST`, measured on egui's input time.
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
    let (Some(session), Some(active), Some(range)) =
        (session.as_ref(), rules.as_ref(), multi.range.as_mut())
    else {
        return;
    };
    let file = session.file();
    let description = &active.rules.ruleset().name;
    let key = CheckKey {
        revision: session.revision(),
        ids: range_ids(file, range.checked.0, range.checked.1),
        ruleset: active.key.clone(),
        suggestions: false,
    };
    // Settled: a modal is up, so no edit can be under way.
    let checking =
        aatf_check::update_check(&mut multi.check, ui.ctx(), file, &active.rules, key, true)
            || range.picked != range.checked;
    let listed = aatf_check::held_result(&multi.check, &active.key);
    let (mut first, mut last) = range.picked;
    let mut close = false;
    let id = egui::Id::unique("save_editor_multi_aatf");
    let title = format!("Multi-team AATF ({description})");
    let response = modal(ui.ctx(), id, title, |ui| {
        ui.horizontal(|ui| {
            ui.weak("Teams");
            team_combo(ui, "save_editor_multi_first", file, u32::MIN, &mut first);
            // The last end follows a first end picked past it, before it is
            // drawn, so the range is never reversed.
            last = last.max(first);
            ui.weak("to");
            team_combo(ui, "save_editor_multi_last", file, first, &mut last);
        });
        ui.horizontal(|ui| {
            if let Some((_, checks)) = listed {
                ui.weak(summary(checks));
            }
            if checking {
                ui.weak("checking\u{2026}");
            }
        });
        if let Some((_, checks)) = listed
            && let Some(counts) = system_counts(active.rules.ruleset(), checks)
        {
            ui.label(counts);
        } else {
            // The label's one auto id, skipped when it is not drawn: the
            // widgets after the line would move an id on the landing
            // frame, and a press held across it is released onto the
            // widget that took the pressed one (a press on the title
            // clicks Close).
            ui.skip_ahead_auto_ids(1);
        }
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let chrome = LIST_MARGIN.sum() + egui::Vec2::splat(2.0 * stroke.width);
        // A scope, so the dimming ends with the list's frame.
        ui.scope(|ui| {
            if checking {
                ui.multiply_opacity(CHECKING_OPACITY);
            }
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
        });
        modal_buttons(ui, |ui| {
            if ui.button("Close").clicked() {
                close = true;
            }
            if add_enabled(ui, listed.is_some(), egui::Button::new("Copy to clipboard")).clicked()
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
    let now = ui.input(|input| input.time);
    if (first, last) != range.picked {
        range.picked = (first, last);
        range.changed_at = now;
    }
    if range.picked != range.checked {
        let rested = now - range.changed_at;
        if rested >= REST {
            range.checked = range.picked;
            // The check starts on the next frame, which is asked for here
            // because no input may come to bring it.
            ui.ctx().request_repaint();
        } else {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(REST - rested));
        }
    }
    if close || response.should_close() {
        multi.open = false;
    }
}

#[cfg(test)]
mod tests {
    use pes_savefile::interchange::team_toml::TeamToml;
    use pes_version::PesVersion;
    use studio_core::Recorder;

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
        let mut session = Session::open(&path, Recorder::default()).unwrap();
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
    /// the team's own) under 4cc Autumn 26: the height systems' counts
    /// first (714 reads under Green, 718 under Red), then the header with
    /// the error and warning counts, each team's line after a `>`, `All
    /// clean` and the team's and the player's lines indented two spaces,
    /// the team's first, each row four, and the Tall quota's players
    /// (718's 185 cm players and its 189 cm goalkeeper, read at 185, in
    /// roster order) six.
    // SE-GUI-41
    // SE-GUI-51
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
                "Height systems: Green 1, Red 1",
                "4cc Autumn 26, teams 714 to 718: 6 errors, 0 warnings",
                ">714 /co/",
                "  All clean",
                ">718 /f/",
                "  Team-wide issues",
                "    Error  Tier count  2 Gold players, where the rules want 1",
                "    Error  Tier count  0 Bronze players, where the rules want 2",
                "    Error  Tier count  19 Regular players, where the rules want 18",
                "    Error  Height quota  9 players in the Tall bracket of the Red system, where its quota is 10",
                "      At 185: GARDEVOIR PAIZURI, PEPSI MAN, DA HOOD, LOLICATGIRLS, RESIZETHEFLASH! (189), HAAAAAAAAAAAAAAN~, BOURBONHOUSE, DONTSHITYOURPANTS, SHII",
                "  71801 ROADTRIP",
                "    Error  Height  height 183 takes no bracket (185 has room)",
                "    Error  Special rule  Tall goalkeepers are 189: GKs in this bracket must be 189cm",
            ]
        );
    }

    /// An `Ok` check naming `medal` and `height`, no violations.
    fn checked(medal: Option<&str>, height: Option<&str>) -> TeamCheck {
        Ok(aatf::TeamCheck {
            violations: Vec::new(),
            medal_system: medal.map(str::to_owned),
            height_system: height.map(str::to_owned),
        })
    }

    /// DBC6's 47 cup teams under DBC7: the counts line names each medal
    /// system and each height system at least one team was read under, in
    /// the ruleset's order, the kinds joined by ` · ` — 868 matching no
    /// medal system counts toward none — and narrowed to 857, its own
    /// pair.
    // SE-GUI-51
    #[test]
    fn the_counts_line_names_the_systems_the_teams_were_read_under() {
        /// `(id, medal, height)`, `-` for none: the check's answers over
        /// DBC6's save, teams 851 to 897.
        const DBC6: [(u32, &str, &str); 47] = [
            (851, "GGSS", "manlet"),
            (852, "GGSS", "manlet"),
            (853, "GSSSS", "giga"),
            (854, "GSSSS", "manlet"),
            (855, "GSSSS", "universal"),
            (856, "GSSSS", "universal"),
            (857, "GGSS", "manlet"),
            (858, "GSSSS", "giga"),
            (859, "GGSS", "universal"),
            (860, "GSSSS", "height"),
            (861, "GGSS", "manlet"),
            (862, "GSSSS", "manlet"),
            (863, "GGSS", "manlet"),
            (864, "GGSS", "height"),
            (865, "GSSSS", "manlet"),
            (866, "GSSSS", "giga"),
            (867, "GSSSS", "universal"),
            (868, "-", "giga"),
            (869, "GSSSS", "universal"),
            (870, "GGSS", "manlet"),
            (871, "GGSS", "height"),
            (872, "GGSS", "universal"),
            (873, "GSSSS", "manlet"),
            (874, "GSSSS", "manlet"),
            (875, "GGSS", "universal"),
            (876, "GGG", "giga"),
            (877, "GSSSS", "universal"),
            (878, "GSSSS", "height"),
            (879, "GSSSS", "manlet"),
            (880, "GGG", "giga"),
            (881, "GSSSS", "giga"),
            (882, "GGSS", "giga"),
            (883, "SSSSSS", "height"),
            (884, "GGSS", "giga"),
            (885, "GGSS", "universal"),
            (886, "GGG", "giga"),
            (887, "GGSS", "manlet"),
            (888, "GGSS", "giga"),
            (889, "GGSS", "manlet"),
            (890, "GGG", "giga"),
            (891, "GGSS", "universal"),
            (892, "GSSSS", "height"),
            (893, "GSSSS", "height"),
            (894, "GGSS", "universal"),
            (895, "GGSS", "giga"),
            (896, "GGSS", "giga"),
            (897, "GSSSS", "height"),
        ];
        assert!(
            DBC6.iter()
                .enumerate()
                .all(|(i, (id, ..))| *id == 851 + i as u32),
            "premise: the table runs 851 to 897"
        );
        let choice = rules::resolve("dbc7", &[]);
        let checks: Vec<TeamCheck> = DBC6
            .iter()
            .map(|(_, medal, height)| {
                checked(
                    (*medal != "-").then_some(*medal),
                    (*height != "-").then_some(*height),
                )
            })
            .collect();
        assert_eq!(
            system_counts(choice.rules.ruleset(), &checks).as_deref(),
            Some(
                "Medal systems: GGG 4, GGSS 21, GSSSS 20, SSSSSS 1 \u{b7} \
                 Height systems: Height 8, Manlet 14, Universal 11, GigaHeight 14"
            )
        );
        let at = DBC6
            .iter()
            .position(|(id, ..)| *id == 857)
            .expect("857 is in the table");
        assert_eq!(
            system_counts(choice.rules.ruleset(), &checks[at..=at]).as_deref(),
            Some("Medal systems: GGSS 1 \u{b7} Height systems: Manlet 1")
        );
    }

    /// The official ruleset holds no medal systems, so the line is the
    /// height systems' alone — 893's Green and 702's Red — and a team
    /// whose check failed counts toward none.
    // SE-GUI-51
    #[test]
    fn a_ruleset_with_no_medal_systems_lists_the_height_systems_alone() {
        let choice = rules::resolve("autumn26", &[]);
        let checks = vec![
            checked(None, Some("green")),
            checked(None, Some("red")),
            Err("the script threw".to_owned()),
        ];
        assert_eq!(
            system_counts(choice.rules.ruleset(), &checks).as_deref(),
            Some("Height systems: Green 1, Red 1")
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
