//! The Live AATF list, the draft's `livePanel` and `liveWindow`: a
//! `BottomDock` at the bottom of the view, or a window of its own over it,
//! listing the open team's violations under `N issues · <ruleset>`,
//! re-checked on the shared worker (`aatf_check`) after every settled edit
//! so the list shrinks as the member fixes them; a row's click selects the
//! player. The toolbar's toggle is the `live_aatf` setting
//! (`view.live_aatf`); off, nothing is drawn and no check starts. Where the
//! list stands is `live_window`: the dock's pop-out icon moves it into the
//! window, the window's dock icon back. Its Show suggestions checkbox is
//! `live_suggestions`: on, the check asks for the suggestions too, listed
//! under `Suggestions` after the violations. Copy to clipboard, left of the
//! dock's pop-out icon and at the count line's right end in the window,
//! writes the list as it shows (`copy_text`), a notice saying it was done.

use pes_savefile::EditFile;
use pes_savefile::model::names::display_name;
use studio_core::widgets::{BottomDock, add_enabled, well_frame};
use studio_core::{Notice, NoticeKind};
use team_widgets::violations::{ListUse, SelectedPlayer, Wording, issue_groups, suggestion_groups};

use crate::session::rules::{self, TeamCheck};
use crate::view::aatf_check::{self, CheckKey, TeamsCheck};
use crate::view::{SaveView, edit_settled, multi_aatf};

/// The pop-out and dock icons' size, the draft's `.live-icon`.
const ICON_SIZE: f32 = 14.0;

/// The window's first size, the draft's `liveWindow` (460 × 360): narrower
/// than the dock, so its players' groups stand in one column.
const WINDOW_SIZE: egui::Vec2 = egui::vec2(460.0, 360.0);

/// How far inside the screen's bottom-right corner the window first stands,
/// the draft's `right: 16px; bottom: 40px`: where the dock stood, out of the
/// floating panels' top-right spot.
const WINDOW_INSET: egui::Vec2 = egui::vec2(16.0, 40.0);

/// The pop-out icon's tooltip and accessible name.
const POP_OUT_TOOLTIP: &str = "Open Live AATF in a window of its own";

/// The window's dock icon's tooltip and accessible name.
const DOCK_TOOLTIP: &str = "Put Live AATF back in its dock";

/// The close buttons' tooltip, and the dock's close button's accessible
/// name.
const CLOSE_TOOLTIP: &str = "Turn Live AATF off";

/// The dock's and the window's state on `SaveView` (`view.live`): the
/// dock's frame and the open team's check, which the window shares.
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
                150.0,
            ),
            check: TeamsCheck::new(),
        }
    }
}

/// What the dock and the window both draw from on a frame: the open file,
/// the team, the ruleset and the held result.
struct Listing<'a> {
    file: &'a EditFile,
    /// The open team, if any.
    team_id: Option<u32>,
    /// The active ruleset's rules, which word the rows.
    ruleset: &'a aatf::Ruleset,
    /// The held result, when one was checked under the active ruleset.
    held: Option<&'a (CheckKey, Vec<TeamCheck>)>,
    /// A check is running for a newer edit than the held result's.
    checking: bool,
    /// The Show suggestions checkbox.
    suggestions: bool,
}

/// One frame of Live AATF, while `view.live_aatf` and a save is open: the
/// open team's check polled and re-spawned for the session's current
/// revision and ruleset once the edit settles, then the list in the dock or,
/// while `view.live_window`, in its own window.
pub(super) fn show(ui: &mut egui::Ui, view: &mut SaveView) {
    let SaveView {
        live,
        live_aatf,
        live_window,
        live_suggestions,
        session,
        team,
        filter,
        selected,
        rules,
        focused,
        tactics,
        notice,
        ..
    } = view;
    let (Some(session), Some(active)) = (session.as_ref(), rules.as_ref()) else {
        return;
    };
    let file = session.file();
    let (_, end) = edit_settled(ui, *focused);
    // The open team: the selected row's team (a player's or a team row's
    // own — the team the view is editing), else the selector's.
    let team_id = aatf_check::current_team(*team, *selected);
    let key = CheckKey {
        revision: session.revision(),
        ids: team_id.into_iter().collect(),
        ruleset: active.key.clone(),
        suggestions: *live_suggestions,
    };
    let checking = aatf_check::update_check(
        &mut live.check,
        ui.ctx(),
        file,
        &active.rules,
        key,
        end.settled(),
    );
    let listing = Listing {
        file,
        team_id,
        ruleset: active.rules.ruleset(),
        held: aatf_check::held_result(&live.check, &active.key),
        checking,
        suggestions: *live_suggestions,
    };
    let mut copied = false;
    let clicked = if *live_window {
        let shown = window(ui, &listing);
        if shown.docked {
            *live_window = false;
        }
        if shown.closed {
            *live_aatf = false;
        }
        copied = shown.copied;
        shown.clicked
    } else {
        let mut clicked = None;
        live.dock.show(
            ui,
            |ui| {
                headline(ui, &listing);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if close_button(ui).clicked() {
                        *live_aatf = false;
                    }
                    // The window's close button stands centred in its slot,
                    // one item spacing from the dock icon's: the slot's half
                    // left of the cross and the pop-out icon in a slot of
                    // its own keep the dock's pair spaced as the window's,
                    // the cross itself at the header's right end.
                    let slot = title_slot(ui);
                    ui.add_space((slot - ui.spacing().icon_width) / 2.0);
                    let pop_out = ui.allocate_ui_with_layout(
                        egui::vec2(slot, ui.spacing().interact_size.y),
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| icon_button(ui, studio_core::icons::POP_OUT, POP_OUT_TOOLTIP),
                    );
                    if pop_out.inner.clicked() {
                        *live_window = true;
                    }
                    copied = copy_button(ui, has_rows(&listing)).clicked();
                });
            },
            |ui| clicked = rows(ui, &listing, false),
        );
        clicked
    };
    if copied && let Some((key, checks)) = listing.held {
        ui.ctx().copy_text(copy_text(
            file,
            listing.ruleset,
            key,
            checks,
            key.suggestions && listing.suggestions,
        ));
        let id = *key.ids.first().expect("a held result checked a team");
        *notice = Some(Notice {
            text: format!(
                "Copied the Live AATF list of {}.",
                multi_aatf::team_label(file, id)
            ),
            kind: NoticeKind::Success,
            action: None,
        });
    }
    if let Some(player) = clicked {
        // A listed row's team is one of the open file's.
        let team_tactics = file
            .team(player.team_id)
            .map(|entry| &entry.tactics)
            .expect("a listed row's team is in the open file");
        aatf_check::select_player(player, selected, team, filter, team_tactics, tactics);
    }
}

/// The header's count line and checking mark, the dock's after its title
/// and the window's under its title bar (the draft's `liveHeadline`): a
/// failed check draws no count, since the body says why.
fn headline(ui: &mut egui::Ui, listing: &Listing<'_>) {
    let ruleset = &listing.ruleset.name;
    match listing.held {
        Some((result_key, checks)) => {
            if checks.iter().all(Result::is_ok) {
                let issues = aatf_check::issues(checks);
                if issues == 0 {
                    let name = display_name(aatf_check::team_name(
                        listing.file,
                        *result_key
                            .ids
                            .first()
                            .expect("a held result checked a team"),
                    ));
                    ui.strong(format!("{name} passes {ruleset} with no issues."));
                } else {
                    let plural = if issues == 1 { "issue" } else { "issues" };
                    ui.strong(format!("{issues} {plural} · {ruleset}"));
                }
            }
        }
        None => {
            if let Some(id) = listing.team_id {
                ui.strong(format!(
                    "Checking {}\u{2026}",
                    display_name(aatf_check::team_name(listing.file, id))
                ));
            }
        }
    }
    if listing.checking && listing.held.is_some() {
        ui.weak("checking\u{2026}");
    }
}

/// The rows in their well (the draft's `.live-body`), scrolling inside it so
/// a long list does not grow it; the players' groups in one column when
/// `one_column`, the starters' and the bench's side by side otherwise.
/// Returns the player whose row was clicked.
fn rows(ui: &mut egui::Ui, listing: &Listing<'_>, one_column: bool) -> Option<SelectedPlayer> {
    // egui's solid scroll bar stands beside the rows, inside the well, as
    // the draft's bar does: a floating one lies over each row's right end,
    // its hover and its sentence's last words.
    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
    well_frame(ui.visuals())
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("save_editor_live_results")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if listing.team_id.is_none() {
                        ui.weak("No team is open.");
                        return None;
                    }
                    let (result_key, checks) = listing.held?;
                    aatf_check::results(
                        ui,
                        listing.file,
                        "aatf_live",
                        result_key,
                        checks,
                        listing.ruleset,
                        ListUse::Editing {
                            // The checkbox turned off hides a held result's
                            // suggestions at once.
                            show_suggestions: result_key.suggestions && listing.suggestions,
                            one_column,
                        },
                    )
                })
                .inner
        })
        .inner
}

/// Whether the held result lists a row — an issue, or a suggestion while
/// they show: Copy to clipboard is off while the list holds no row.
fn has_rows(listing: &Listing<'_>) -> bool {
    let Some((key, checks)) = listing.held else {
        return false;
    };
    checks.iter().flatten().any(|check| {
        check.violations.iter().any(|violation| {
            violation.severity != aatf::Severity::Suggestion
                || (key.suggestions && listing.suggestions)
        })
    })
}

/// The list as it shows, as plain text (the draft's `liveText`): the
/// ruleset's name, the team and its error and warning counts worded as
/// Multi-team AATF's copy words its header line, then the team's groups,
/// then `Suggestions` and their groups while they show.
fn copy_text(
    file: &EditFile,
    ruleset: &aatf::Ruleset,
    key: &CheckKey,
    checks: &[TeamCheck],
    suggestions: bool,
) -> String {
    let wording = Wording {
        ruleset,
        version: file.version(),
    };
    let listed = rules::listed_players(file, &key.ids);
    let mut lines = Vec::new();
    for ((&id, check), players) in key.ids.iter().zip(checks).zip(&listed) {
        let Ok(check) = check else {
            continue;
        };
        let count = |severity: aatf::Severity| {
            check
                .violations
                .iter()
                .filter(|violation| violation.severity == severity)
                .count()
        };
        lines.push(format!(
            "{}, team {}: {}, {}",
            ruleset.name,
            multi_aatf::team_label(file, id),
            multi_aatf::counted(count(aatf::Severity::Error), "error"),
            multi_aatf::counted(count(aatf::Severity::Warning), "warning"),
        ));
        multi_aatf::group_lines(
            file,
            id,
            players,
            &issue_groups(&check.violations),
            wording,
            &mut lines,
        );
        if suggestions {
            let groups = suggestion_groups(&check.violations);
            if !groups.is_empty() {
                lines.push("Suggestions".to_owned());
                multi_aatf::group_lines(file, id, players, &groups, wording, &mut lines);
            }
        }
    }
    lines.join("\n")
}

/// The Copy to clipboard button (the draft's `liveCopy`), egui's plain
/// button as Multi-team AATF's is, off while the list holds no row.
fn copy_button(ui: &mut egui::Ui, enabled: bool) -> egui::Response {
    add_enabled(ui, enabled, egui::Button::new("Copy to clipboard"))
}

/// A frameless icon-font button (the draft's `liveIconButton`), named for
/// screen readers by its tooltip, since its glyph names nothing.
fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    let response =
        ui.add(egui::Button::new(egui::RichText::new(glyph).size(ICON_SIZE)).frame(false));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip));
    response.on_hover_text(tooltip)
}

/// The dock header's close button, drawn as egui draws a window's: an
/// `icon_width` square sensing clicks, its two diagonals shrunk 2 points
/// and grown by the state's expansion, in the state's foreground stroke.
/// egui keeps its own private, so the dock copies its drawing: the same
/// cross in the dock's corner and the window's reads as one control.
fn close_button(ui: &mut egui::Ui) -> egui::Response {
    let size = egui::Vec2::splat(ui.spacing().icon_width);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, CLOSE_TOOLTIP));
    let visuals = ui.style().interact(&response);
    let cross = rect.shrink(2.0).expand(visuals.expansion);
    let stroke = visuals.fg_stroke;
    ui.painter()
        .line_segment([cross.left_top(), cross.right_bottom()], stroke);
    ui.painter()
        .line_segment([cross.right_top(), cross.left_bottom()], stroke);
    response.on_hover_text(CLOSE_TOOLTIP)
}

/// The side of the square egui gives a window's title bar close button,
/// the `Heading` row's height, which the window's dock icon and the dock's
/// pop-out icon each take as their width too.
fn title_slot(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Heading)
}

/// What the window's frame did: its dock icon or its close button clicked,
/// its copy button, or a row.
struct WindowFrame {
    docked: bool,
    closed: bool,
    clicked: Option<SelectedPlayer>,
    copied: bool,
}

/// The window, the draft's `liveWindow`: an `egui::Window` titled `Live
/// AATF`, first `WINDOW_SIZE` at the screen's bottom right, moved by its
/// title and resized from its edges, as the floating panels are (egui keeps
/// where the member leaves it). Its title bar holds the dock icon left of
/// the close button, which turns Live AATF off as the toolbar toggle does;
/// under it, the dock's count line and the rows in one column.
fn window(ui: &egui::Ui, listing: &Listing<'_>) -> WindowFrame {
    let id = egui::Id::unique("save_editor_live_aatf_window");
    let slot = title_slot(ui);
    let corner = ui.ctx().content_rect().max - WINDOW_INSET;
    let mut open = true;
    let mut docked = false;
    let mut copied = false;
    let clicked = egui::Window::new((
        studio_core::theme::window_title_text("Live AATF"),
        // Room for the dock icon in the title's own row, so egui centres
        // the title in the room the two buttons leave rather than the close
        // button alone.
        egui::Atom::custom(
            egui::IdSalt::new("live_aatf_dock_slot"),
            egui::vec2(slot, 0.0),
        ),
    ))
    .id(id)
    .default_rect(egui::Rect::from_min_max(corner - WINDOW_SIZE, corner))
    .collapsible(false)
    // After any touch, egui's default drags from anywhere and makes no
    // title bar interaction, so the dock icon would never be drawn.
    .drag_area(egui::WindowDrag::TitleBar)
    .resizable(true)
    .open(&mut open)
    .show(ui.ctx(), |ui| {
        docked = title_bar_buttons(ui, id, slot);
        // The count line, Copy to clipboard at its right end (under the
        // title bar's close button).
        ui.horizontal(|ui| {
            headline(ui, listing);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                copied = copy_button(ui, has_rows(listing)).clicked();
            });
        });
        rows(ui, listing, true)
    })
    .and_then(|shown| shown.inner)
    .flatten();
    WindowFrame {
        docked,
        closed: !open,
        clicked,
        copied,
    }
}

/// The dock icon on the title bar, left of the close button and on its
/// centre line, and the close button's tooltip. egui's title bar has no
/// room for a widget of ours, so both go over it from the body: the bar is
/// found through egui's own drag area (`__title_click`), which spans it up
/// to the close button's left edge, the close button being an
/// `icon_width` square centred in a `slot` one. The dock icon, added after
/// the drag area, takes the clicks over it; the rest of the bar still
/// drags the window. Returns whether the dock icon was clicked.
fn title_bar_buttons(ui: &mut egui::Ui, window_id: egui::Id, slot: f32) -> bool {
    let Some(bar) = ui.ctx().read_response(window_id.with("__title_click")) else {
        return false;
    };
    let bar = bar.rect;
    let icon = ui.spacing().icon_width;
    let close = egui::Rect::from_center_size(
        egui::pos2(bar.max.x + icon / 2.0, bar.center().y),
        egui::Vec2::splat(icon),
    );
    let close_slot_left = close.center().x - slot / 2.0;
    let dock_slot = egui::Rect::from_center_size(
        egui::pos2(
            close_slot_left - ui.spacing().item_spacing.x - slot / 2.0,
            bar.center().y,
        ),
        egui::Vec2::splat(slot),
    );
    ui.interact(close, window_id.with("close_tooltip"), egui::Sense::hover())
        .on_hover_text(CLOSE_TOOLTIP);
    // A child the body never allocates: the slot lies above the body, and
    // allocating it would move the body's cursor and size up to the bar.
    let mut slot_ui = ui.new_child(egui::UiBuilder::new().max_rect(dock_slot).layout(
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
    ));
    icon_button(&mut slot_ui, studio_core::icons::DOCK, DOCK_TOOLTIP).clicked()
}
