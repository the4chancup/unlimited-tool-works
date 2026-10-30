//! The Save editor's window: a toolbar over a master-detail layout, the
//! player list left (`player_list`, its team and player rows, the quick
//! actions under it) and, right, the view the selected row names: the team
//! view (`team`) on a team row, the player view — the title
//! (`info_strip`), the three columns (`abilities` with `info_strip`'s
//! General info and Other stats) and the Appearance section (`appearance`)
//! — on a player row. Render-only over the session's state: it opens a
//! save, filters and selects players, reorders a team's list by drag
//! (`Session::move_player`), edits the strip's fields through
//! `Session::edit_player`/`edit_team` (one undo entry per gesture), saves, and
//! shows a refusal in a modal.

mod aatf_check;
mod abilities;
mod appearance;
mod changes_popup;
mod comparator;
mod event_logo;
mod generate;
mod info_strip;
mod interchange;
mod live_aatf;
mod live_marks;
mod menus;
mod multi_aatf;
mod open_menu;
mod patch;
mod player_list;
mod portraits;
pub(crate) mod prompt;
mod quick_actions;
mod ruleset_picker;
mod tactics;
mod team;
pub(crate) mod terms;
#[cfg(test)]
mod tests;
mod worker;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use pes_savefile::discovery::SavefileCandidate;
use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_version::PesVersion;
use studio_core::widgets::{
    add_enabled, collapsing_header, modal, modal_buttons, toolbar_separator,
};
use studio_core::{ActionEffect, Notice, NoticeAction, NoticeKind, RunningPes};

use crate::session::Session;
use crate::session::rules::{self, Imported};
use crate::session::selection::{self, TeamChoice};

/// The list row the selection names: a team by id, or a player by team
/// and roster slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    /// A team's own row (the detail shows the team view).
    Team(u32),
    /// A player row: the team he is rostered on and his roster slot.
    Player(u32, u8),
}

/// The floating panels, of which one at a time is open (`SaveView::panel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    /// The Savefile menu's Comparator.
    Comparator,
    /// The Savefile menu's Apply aesthetics patch.
    Patch,
    /// The Team menu's team file panel.
    Interchange,
}

/// The window's state: the open save and what the master-detail shows of it.
pub(crate) struct SaveView {
    /// The open save, if one was opened.
    session: Option<Session>,
    /// Which team the list shows.
    team: TeamChoice,
    /// The filter box's text.
    filter: String,
    /// The selected row.
    selected: Option<Selection>,
    /// A page key moved the selection: the list scrolls to its row on the
    /// next draw, then clears this.
    reveal_selected: bool,
    /// The team view's Tactics state (`team_widgets`' `TacticsView`:
    /// the preset its panels edit, the formation its lists show, the
    /// armed assignment mode, the open slot popover and Scroll names);
    /// back to preset 1 and kick-off on every open.
    pub(crate) tactics: team_widgets::tactics::TacticsView,
    /// The widget holding the keyboard at the last frame's end. A
    /// widget's gesture ends when it loses the keyboard (a click
    /// elsewhere, a Tab): sealing then keeps two gestures' writes from
    /// merging into one undo entry, since an entry is one gesture.
    /// A `None` to `Some` move never seals: a widget can take focus as
    /// its gesture starts (a DragValue's click), and that gesture stays
    /// one step.
    focused: Option<egui::Id>,
    /// The unsaved-changes prompt's state while it is open (SE-GUI-05):
    /// the pending action and the table's rows, computed once at open.
    pub(crate) prompt: Option<prompt::Prompt>,
    /// The toolbar's list of unsaved changes while it is open (SE-GUI-43):
    /// its rows, computed once at open.
    changes_list: Option<changes_popup::ChangesList>,
    /// The running-game question a Save or Save as first asks while the
    /// save's own PES runs (SE-GUI-21's window rule).
    edit_mode_question: Option<EditModeQuestion>,
    /// The Discard-on-close answer's ask to close again, drained by
    /// `Tool::view` into `ctx.request_close()` (the shell owns the send).
    pub(crate) request_close: bool,
    /// The Open menu's `Choose file…` picker (`rfd`, stubbed in tests).
    open_dialog: open_menu::OpenDialog,
    /// Save as's picker (`rfd`, stubbed in tests).
    save_as_dialog: SaveAsDialog,
    /// The Open menu's discovery (`discover_savefiles`, stubbed in tests).
    discover: open_menu::Discover,
    /// What discovery found when the Open menu opened, each save with its
    /// version, in `PesVersion::ALL`'s order; `None` while the menu is
    /// closed, so the next opening looks again.
    discovered: Option<Vec<(PesVersion, SavefileCandidate)>>,
    /// The files opened or written by Save as in this run of the program,
    /// the most recent first: the Open menu's middle group. Not a setting.
    recents: Vec<PathBuf>,
    /// The sidebar's PES version, refreshed from the common settings each
    /// `Tool::view` frame: `Choose file…` starts in its save folder. PES 21
    /// in the tests' view-only frames until a test sets it.
    pub(crate) pes_version: PesVersion,
    /// A refusal to show until dismissed (an open the save refused).
    refusal: Option<Refusal>,
    /// The `aatf_ruleset` setting: `Tool::view` refreshes it each frame,
    /// the toolbar's ruleset dropdown writes a pick into it and
    /// `Tool::view` writes that back, as `live_aatf` is; the first
    /// built-in ruleset in the tests' view-only frames.
    pub(crate) aatf_ruleset: String,
    /// The imported rulesets the dropdown lists after the built-in ones:
    /// `Tool::view` reads them from the data folder on its first frame;
    /// none in the tests' view-only frames.
    pub(crate) imported: Vec<Imported>,
    /// The data folder the imported rulesets' copies live in (its
    /// `rules/`): `Tool::view` sets it with `imported`; none in the tests'
    /// view-only frames, so an import there is refused.
    pub(crate) data_dir: Option<PathBuf>,
    /// The ruleset dropdown's Import rules file… picker (`rfd` on
    /// `.4ccrules` files, stubbed in tests: the `open_dialog` pattern).
    rules_file_dialog: comparator::OpenSaveDialog,
    /// The import's or the ×'s dialog while one is up, or the picker due
    /// at the frame's end; `None` otherwise.
    rules_dialog: Option<ruleset_picker::RulesDialog>,
    /// The ruleset the quick actions, the Cards left badge, the medal
    /// outlines and Live AATF use, resolved from `aatf_ruleset` on the
    /// first frame and again only when the setting changes.
    rules: Option<ActiveRules>,
    /// The floating panel drawn over the views, if one is open: one at a
    /// time, since each would otherwise stand under the other; none at
    /// start.
    panel: Option<Panel>,
    /// The comparator panel's state.
    comparator: comparator::Comparator,
    /// The comparator's Pick dialog (`rfd`, stubbed in tests — the
    /// toolbar's Open dialog pattern).
    open_save_dialog: comparator::OpenSaveDialog,
    /// The Apply aesthetics patch panel's state.
    patch: patch::Patch,
    /// The patch panel's Open patch dialog (`rfd` on `.toml` files,
    /// stubbed in tests: the `open_save_dialog` pattern).
    patch_dialog: comparator::OpenSaveDialog,
    /// The team file panel's state (the Team menu's `Import / export team
    /// file…`).
    interchange: interchange::Interchange,
    /// The team file panel's Pick dialog (`rfd`, its filters ordered by
    /// the save's version, stubbed in tests: the `open_save_dialog` pattern).
    interchange_dialog: interchange::InterchangeDialog,
    /// The team file panel's Save dialog for its exports (`rfd`, stubbed
    /// in tests: the `save_as_dialog` pattern).
    team_export_dialog: interchange::ExportDialog,
    /// The Generate dialog's state while it is open (the Player menu's
    /// `settings.toml` and in-game face items); `None` while it is closed.
    generate: Option<generate::Generate>,
    /// The Generate dialog's Pick-a-folder dialog (`rfd`, stubbed in
    /// tests — the same pattern as `open_save_dialog`).
    pick_folder_dialog: generate::PickFolderDialog,
    /// The status-bar notice the last action raised; `Tool::view` forwards
    /// it to the shell's `notify` after the frame.
    pub(crate) notice: Option<Notice>,
    /// The version warning lives: set by every open, so a new save starts
    /// it over, and cleared once the member has seen it (its action, its
    /// dismiss, or another notice taking the bar from it). The selector
    /// matching the save only hides it: it shows again when they part.
    pub(crate) version_warning: bool,
    /// The Live AATF toolbar toggle's state: `Tool::view` refreshes it
    /// from the `live_aatf` setting and writes the setting back when the
    /// toggle changes it; off in the tests' view-only frames.
    pub(crate) live_aatf: bool,
    /// The Show suggestions checkbox beside the toggle, refreshed from and
    /// written back to the `live_suggestions` setting as `live_aatf` is:
    /// on, Live AATF's check asks for the suggestions too.
    pub(crate) live_suggestions: bool,
    /// The Live AATF dock's state (the panel docked under the panes);
    /// drawn while `live_aatf` and a save is open.
    live: live_aatf::LiveAatf,
    /// The Multi-team AATF dialog's state (the Savefile menu's entry): open
    /// or closed, its range and its check.
    multi_aatf: multi_aatf::MultiAatf,
    /// The comparator's Export dialog (`rfd`, stubbed in tests — the
    /// Ruleset editor's picker pattern).
    export_dialog: comparator::SaveFileDialog,
    /// Output rosters to TSV's Save dialog (`rfd`, stubbed in tests: the
    /// `export_dialog` pattern).
    rosters_dialog: comparator::SaveFileDialog,
    /// The "Cards left" badge as last computed and the `BadgeKey` it was
    /// computed for: `player_tier` then `card_limits` run again only when
    /// the key differs, not on every frame.
    cards: Option<(BadgeKey, OwnBadge)>,
    /// The pitch badges' medal letters per roster slot, computed on a
    /// worker thread for a `MedalKey`: the last answer with its key and
    /// the run in flight. `player_tier` runs again only when the key
    /// differs, not on every frame.
    medals: tactics::Medals,
    /// The Tactics pitch's portraits, read from the selected game's
    /// `download/` CPKs off the UI thread (`select_portraits` points it at
    /// the PES folder each frame).
    portraits: portraits::Portraits,
    /// Which open this is: part of the pushed widget id, so egui widget
    /// state (a text field's undo history, a control's in-progress edit)
    /// never crosses into the next save's same player id.
    generation: u64,
    /// "Aesthetics from exports", which makes the save's aesthetics
    /// read-only, refreshed from the setting each `Tool::view` frame as
    /// `live_aatf` is; off in the tests' view-only frames, which set it by
    /// hand as `edit_aesthetics`'s GIVEN.
    pub(crate) aesthetics_from_exports: bool,
    /// "Edit aesthetics anyway": off at every start of the tool, so a
    /// machine set to "Aesthetics from exports" cannot quietly stay
    /// unlocked, and deliberately not reset when another save opens — a
    /// session switch, not a per-save one.
    edit_aesthetics: bool,
    /// The Set boots and gloves dialog's state while it is open (the Team
    /// menu's `…` item); `None` while it is closed.
    boglo: Option<menus::Boglo>,
    /// The Set all stats / Bump all stats dialog's state while it is open
    /// (the Player menu's `…` items, and Ctrl+B); `None` while it is
    /// closed.
    stat_num: Option<menus::StatNum>,
    /// The Bump dialog's last N used in this window (its field's
    /// prefill), starting at 0.
    bump: i32,
    /// The quick actions' Set stats number (the draft's `set-stats`
    /// field), starting at 77.
    set_stats: u8,
    /// The Copy/Swap stats dialog's state while it is open (the Player
    /// menu's `…` items); `None` while it is closed.
    copy_swap: Option<menus::CopySwap>,
    /// The Fix database confirmation is open (the Savefile menu's item);
    /// `false` while it is closed.
    fix_database: bool,
    /// The Kit slot assignments dialog's team while it is open (the team
    /// view's `Kit slots…` button); `None` while it is closed.
    kit_slots: Option<u32>,
    /// The hide IDs the common settings give (`fpc_hide_boots_id` and
    /// `fpc_hide_gloves_id`), refreshed from `ctx.common()` each `Tool::view`
    /// frame: every FPC path (Hide/Show default body, Ctrl+I, the classifier's
    /// reading, the Team menu's Set/Remove) reads them, and an out-of-band
    /// pair disables those paths behind the error's text.
    pub(crate) hide_ids: Result<fpc::HideIds, fpc::HideIdsError>,
    /// Where the Appearance section's Randomize draws a motion value in
    /// `1..=top` from: a field so the tests pin the draws; `new` seeds a
    /// xorshift from the wall clock (the picks need only be unpredictable
    /// to a member, not cryptographic).
    randomize_draw: Box<dyn FnMut(u8) -> u8>,
}

/// The ruleset the window's AATF controls use, the one `check-aatf` runs
/// without `--rules`: the setting it was resolved from, what that resolved
/// to and the loaded rules.
pub(crate) struct ActiveRules {
    /// The `aatf_ruleset` value `key` was resolved from: a setting naming
    /// a ruleset that is gone differs from `key`, and is not resolved (nor
    /// its notice shown) again while it stays.
    setting: String,
    /// The built-in stem or imported file name the rules came from: the
    /// key the caches below and the AATF checks are computed for.
    key: String,
    /// The loaded rules. The `Arc` lets Live AATF's worker check teams on
    /// the same loaded file the badge reads.
    rules: Arc<aatf::Rules>,
    /// The ruleset's event logo, decoded when the ruleset is resolved;
    /// `None` when it carries none or it does not decode.
    logo: Option<team_widgets::event_logo::EventLogo>,
}

/// What the Skills column's Cards left badge was computed for: a change of
/// the open save (`Session::revision`), of the selection or of the
/// ruleset recomputes it; anything else keeps a frame's `player_tier` and
/// `card_limits` cost off.
#[derive(PartialEq)]
pub(crate) struct BadgeKey {
    /// `Session::revision` when computed.
    revision: u64,
    /// The selected team.
    team: u32,
    /// The selected roster slot (0-based).
    slot: u8,
    /// The ruleset the badge was computed under (`ActiveRules::key`).
    ruleset: String,
}

/// What the pitch badges' medal letters were computed for: the Cards
/// left badge's `BadgeKey` minus the selection — a change of the open
/// save (`Session::revision`), of the team or of the ruleset recomputes
/// them, and any other frame draws the kept ones.
#[derive(Clone, PartialEq)]
pub(crate) struct MedalKey {
    /// `Session::revision` when computed.
    revision: u64,
    /// The team the medals are for.
    team: u32,
    /// The ruleset the medals were computed under (`ActiveRules::key`).
    ruleset: String,
}

/// The badge `cards` holds for a `BadgeKey`, owned: what the rules read the
/// selected player as.
pub(crate) enum OwnBadge {
    /// The tier `Rules::player_tier` reads him as: its display label and
    /// his card economy under it.
    Tier {
        label: String,
        limits: aatf::CardLimits,
    },
    /// No tier's condition holds for him (`check_team`'s `NoTier`).
    NoTier,
    /// `player_tier` or `card_limits` failed (or the team could not be
    /// read), with the error's text: the row draws the red click line
    /// `TIER_CHECK_FAILED_LINE` in the badge's place, opening the error.
    /// The AATF check never calls
    /// these two functions, so a member's logic edit that breaks only them
    /// would otherwise show nowhere. Kept under its key like the others, so
    /// the failing script does not run again on every frame.
    Failed(String),
}

/// The running-game question while it is up: the game found running and
/// the file its Save writes (the open file for Save, the picked one for
/// Save as).
struct EditModeQuestion {
    /// The save's own version, whose game runs.
    game: PesVersion,
    /// Where the answered Save writes.
    target: PathBuf,
}

/// Save as's picker: the open file's path, whose folder and name it
/// starts on, to the picked path (`rfd` in the window, a stub in the
/// tests: a native dialog cannot be driven headless).
pub(crate) type SaveAsDialog = Box<dyn FnMut(&Path) -> Option<PathBuf>>;

/// The `rfd` Save dialog for Save as.
#[cfg(not(test))]
fn default_save_as_dialog() -> SaveAsDialog {
    Box::new(|current| {
        let mut dialog = rfd::FileDialog::new();
        if let Some(folder) = current.parent() {
            dialog = dialog.set_directory(folder);
        }
        if let Some(name) = current.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        dialog.save_file()
    })
}

/// `default_save_as_dialog` under `cargo test`: a test reaching a real
/// file dialog panics; tests stub `view.save_as_dialog`.
#[cfg(test)]
fn default_save_as_dialog() -> SaveAsDialog {
    Box::new(|_| panic!("a test reached a real file dialog: stub save_as_dialog"))
}

/// A refusal shown as a dialog: a title, the message and OK.
struct Refusal {
    /// The dialog's heading.
    title: &'static str,
    /// What was refused, as the operation reported it.
    message: String,
}

impl Default for SaveView {
    fn default() -> SaveView {
        SaveView::new()
    }
}

impl SaveView {
    /// The window as it starts: no save open.
    pub(crate) fn new() -> SaveView {
        SaveView {
            session: None,
            team: TeamChoice::All,
            filter: String::new(),
            selected: None,
            reveal_selected: false,
            tactics: team_widgets::tactics::TacticsView::default(),
            focused: None,
            prompt: None,
            changes_list: None,
            edit_mode_question: None,
            request_close: false,
            open_dialog: open_menu::default_open_dialog(),
            save_as_dialog: default_save_as_dialog(),
            discover: open_menu::default_discover(),
            discovered: None,
            recents: Vec::new(),
            pes_version: PesVersion::Pes21,
            refusal: None,
            aatf_ruleset: rules::FIRST_BUILTIN.to_owned(),
            imported: Vec::new(),
            data_dir: None,
            rules_file_dialog: ruleset_picker::default_rules_dialog(),
            rules_dialog: None,
            rules: None,
            panel: None,
            comparator: comparator::Comparator::new(),
            open_save_dialog: comparator::default_open_dialog(),
            patch: patch::Patch::new(),
            patch_dialog: patch::default_patch_dialog(),
            interchange: interchange::Interchange::new(),
            interchange_dialog: interchange::default_interchange_dialog(),
            team_export_dialog: interchange::default_export_dialog(),
            generate: None,
            pick_folder_dialog: generate::default_pick_folder(),
            notice: None,
            version_warning: false,
            live_aatf: false,
            live_suggestions: false,
            live: live_aatf::LiveAatf::new(),
            multi_aatf: multi_aatf::MultiAatf::new(),
            export_dialog: comparator::default_export_dialog(),
            rosters_dialog: menus::default_rosters_dialog(),
            cards: None,
            medals: tactics::Medals::new(),
            portraits: portraits::Portraits::new(),
            generation: 0,
            aesthetics_from_exports: false,
            edit_aesthetics: false,
            boglo: None,
            stat_num: None,
            bump: 0,
            set_stats: 77,
            copy_swap: None,
            fix_database: false,
            kit_slots: None,
            hide_ids: Ok(fpc::HideIds::CONVENTION),
            randomize_draw: Box::new(appearance::seeded_draw()),
        }
    }

    /// Points the pitch's portrait source at `pes_folder` for `version`: `Tool::view` calls it
    /// each frame with the sidebar's PES and the PES folder setting, as it refreshes `hide_ids`.
    pub(crate) fn select_portraits(
        &mut self,
        ctx: &egui::Context,
        pes_folder: &Path,
        version: PesVersion,
    ) {
        self.portraits.select(ctx, pes_folder, version);
    }

    /// The aesthetics lock: on while the "Aesthetics from exports" setting
    /// is and the session's "Edit aesthetics anyway" switch is not. The
    /// one function the locked
    /// paths' guards read, so no site can forget the setting.
    pub(crate) fn aesthetics_locked(&self) -> bool {
        self.aesthetics_from_exports && !self.edit_aesthetics
    }

    /// Whether the open session holds unsaved changes (`Tool::hold_close`'s
    /// ask; `false` with none open).
    pub(crate) fn is_dirty(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.is_dirty())
    }

    /// The open save's path (`Tool::title_detail`'s, for the idle window
    /// title); `None` with none open.
    pub(crate) fn save_path(&self) -> Option<&Path> {
        self.session.as_ref().map(Session::path)
    }

    /// Opens the save at `path`: on success the selector reads `ALL`, the
    /// filter is empty and the first player row is selected (a team's own
    /// row is not a player); a file the
    /// save refuses is shown as a refusal and the open session is untouched.
    /// The aesthetics lock is deliberately not reset (`edit_aesthetics`).
    pub(crate) fn open(&mut self, path: &Path) {
        match Session::open(path) {
            Ok(session) => {
                self.team = TeamChoice::All;
                self.filter.clear();
                self.selected = selection::list(session.file(), &self.team, "")
                    .into_iter()
                    .find_map(|row| match row {
                        selection::ListRow::Player(row) => {
                            Some(Selection::Player(row.team_id, row.slot))
                        }
                        selection::ListRow::Team(_) => None,
                    });
                self.comparator.reset(session.version());
                open_menu::push_recent(&mut self.recents, path);
                self.session = Some(session);
                self.generation += 1;
                self.version_warning = true;
                // A view setting, not the save's: `scroll_names` is the
                // member's, so it survives the open (the rest is the
                // save's: back to preset 1 and kick-off).
                let scroll_names = self.tactics.scroll_names;
                self.tactics = team_widgets::tactics::TacticsView::default();
                self.tactics.scroll_names = scroll_names;
                self.prompt = None;
                self.edit_mode_question = None;
                self.refusal = None;
                self.boglo = None;
                self.stat_num = None;
                self.copy_swap = None;
                self.generate = None;
                self.fix_database = false;
                self.kit_slots = None;
                self.live.check.reset();
                self.multi_aatf.reset();
            }
            Err(error) => {
                self.refusal = Some(Refusal {
                    title: "Cannot open the save",
                    message: format!("{error:#}"),
                });
            }
        }
    }

    /// The version warning while it lives and the sidebar's `selector`
    /// names another version than the open save's: a Warning whose action
    /// switches the selector to the save's version. `None` otherwise.
    pub(crate) fn version_notice(&self, selector: PesVersion) -> Option<Notice> {
        let save = self.session.as_ref()?.version();
        if !self.version_warning || selector == save {
            return None;
        }
        Some(Notice {
            text: version_warning_text(selector, save),
            kind: NoticeKind::Warning,
            action: Some(NoticeAction {
                label: format!("Switch to {save}"),
                effect: ActionEffect::SwitchVersion(save),
            }),
        })
    }
}

/// The version warning's sentence: the save opened under its own version,
/// whatever the sidebar's selector names.
pub(crate) fn version_warning_text(selector: PesVersion, save: PesVersion) -> String {
    format!(
        "Although the sidebar is set to {selector}, this save is from {save}, so the editor \
         will keep treating it as a {save} save."
    )
}

/// The player list's column, the draft's `.plist`: the rows' frame and
/// the scroll bar's strip beside it, the panel's margins not included.
const LIST_WIDTH: f32 = 255.0;

/// A number box's width, the drafts' number input: the quick actions', the
/// boots and gloves dialog's and the number dialogs' fields. egui's own
/// least widget width is 40.
pub(super) const NUMBER_BOX: f32 = 52.0;

/// The detail pane's margin over a view's title.
const TITLE_MARGIN: i8 = 6;

/// The player list's margin over its team combo. The panes start one item
/// spacing (3 points) under the toolbar, so the combo stands 4 under the
/// toolbar's buttons, the draft's toolbar margin.
const LIST_TOP_MARGIN: i8 = 1;

/// The room between the panes' bottom and the status bar (the draft's
/// `.center.filled` bottom padding).
const PANE_BOTTOM_MARGIN: f32 = 6.0;

/// How far the Appearance header's text stands after its left edge: the
/// draft's `summary::before` arrow box, 14px wide, where egui indents 18.
const APPEARANCE_INDENT: f32 = 14.0;

/// How far the Appearance header's arrow stands from the middle of the box
/// egui centres it in (the indent's middle, on the header's centre line):
/// the draft's arrow is a symbol at the box's left, on the text's baseline,
/// so it sits left of that middle and below the line's centre.
const APPEARANCE_ARROW_OFFSET: egui::Vec2 = egui::vec2(-3.5, 2.0);

/// The Appearance section under the player's columns, closed by default,
/// its header as the draft's `details > summary`: one line of text high,
/// the text `APPEARANCE_INDENT` in, the arrow where the draft's stands
/// (`appearance_arrow`). The section's content keeps the view's spacing
/// and its width (the grids are sized for it, the body's indent would push
/// them past it).
fn appearance_section(ui: &mut egui::Ui, view: &mut SaveView) {
    let spacing = ui.spacing().clone();
    ui.scope(|ui| {
        let text_height = ui.text_style_height(&egui::TextStyle::Button);
        let header = ui.spacing_mut();
        header.indent = APPEARANCE_INDENT;
        header.interact_size.y = text_height;
        header.button_padding.y = 0.0;
        collapsing_header("Appearance")
            .id_salt("appearance")
            .icon(appearance_arrow)
            .show_unindented(ui, |ui| {
                *ui.spacing_mut() = spacing;
                appearance::show(ui, view);
            });
    });
}

/// egui's collapsing arrow, moved by `APPEARANCE_ARROW_OFFSET` from the box
/// egui gives it (`response`'s rect).
fn appearance_arrow(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
    let moved = response
        .clone()
        .with_new_rect(response.rect.translate(APPEARANCE_ARROW_OFFSET));
    egui::collapsing_header::paint_default_icon(ui, openness, &moved);
}

/// Shows the window for one frame. `running` is the running-PES probe,
/// called only when Save runs (a process snapshot is not a frame's cost).
/// The ruleset is resolved from `view.aatf_ruleset` on the first frame and
/// again only when the setting changes; a setting naming a ruleset that is
/// gone raises its warning notice then.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) {
    set_surrender_on_press(ui.ctx());
    if view
        .rules
        .as_ref()
        .is_none_or(|active| active.setting != view.aatf_ruleset)
    {
        let choice = rules::resolve(&view.aatf_ruleset, &view.imported);
        if let Some(text) = choice.notice {
            view.notice = Some(Notice {
                text,
                kind: NoticeKind::Warning,
                action: None,
            });
        }
        let logo =
            choice.rules.ruleset().logo_png().and_then(|png| {
                team_widgets::event_logo::EventLogo::load(ui.ctx(), &choice.key, &png)
            });
        view.rules = Some(ActiveRules {
            setting: view.aatf_ruleset.clone(),
            key: choice.key,
            rules: choice.rules,
            logo,
        });
    }
    let toolbar_end = toolbar(ui, view, running);
    if view.session.is_none() {
        ui.weak("No save is open.");
    } else {
        // The panes under the toolbar end `PANE_BOTTOM_MARGIN` over the
        // status bar, the draft's padding under the view, where the
        // shell's central panel keeps its own margin: they reach into
        // that margin by the difference. The shell's panel clips to its
        // whole rect, margins included, so nothing of theirs is cut.
        let mut panes = ui.available_rect_before_wrap();
        panes.max.y += f32::from(egui::Frame::central_panel(ui.style()).inner_margin.bottom)
            - PANE_BOTTOM_MARGIN;
        let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(panes));
        // Before the left panel, so the dock spans the view's width under
        // the player list and the detail view (egui lays a panel out
        // against the space the earlier panels left).
        if view.live_aatf {
            live_aatf::show(ui, view);
        }
        // No margin before the list, so its frame stands at the view's
        // left edge, and 4 points after it: with the detail's own 8-point
        // margin, the detail stands 12 after the list's scroll bar, the
        // draft's gap. No separator line between the panes, as the draft
        // draws none. No bottom margin: the quick actions' band is the
        // panel's content's last item and runs to the pane's bottom, so
        // the pane gets no scroll bar of its own.
        let margin = egui::Margin {
            left: 0,
            right: 4,
            top: LIST_TOP_MARGIN,
            bottom: 0,
        };
        let list_panel = egui::Panel::left("save_editor_players")
            // The draft's `.plist`: a 255-point column (the rows' frame and
            // the scroll bar's strip), not resizable. egui's panel size
            // counts its margins too. Under the design width the window
            // scales instead of narrowing, and a list dragged wider would
            // only take the views' room.
            .resizable(false)
            .show_separator_line(false)
            .default_size(LIST_WIDTH + margin.sum().x)
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(margin))
            .show(ui, |ui| player_list::show(ui, view));
        let frame_right = list_panel.inner;
        // 6 points over the view, not egui's 8: a view's title then stands
        // as far below the toolbar's buttons as its first row stands below
        // the title, in ink (`info_strip::view_title`). None at the right:
        // the shell's own margin past the pane is the view's edge.
        let detail = egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin {
            left: 8,
            right: 0,
            top: TITLE_MARGIN,
            bottom: 8,
        });
        egui::CentralPanel::default().frame(detail).show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match view.selected {
                    Some(Selection::Team(_)) => team::show(ui, view),
                    Some(Selection::Player(..)) | None => {
                        // The player view: the title, the three columns,
                        // then the Appearance section, closed by default.
                        // The room the three fixed columns leave between
                        // the player list's frame and the window's right
                        // edge is split into four equal gaps: before the
                        // first column, between the columns and after the
                        // last. The eye reads the first gap from the list's
                        // last ink and the last to the window's edge, so
                        // the pane's own margins count in neither: centred
                        // in the pane, the gap after the list read wider
                        // than those between the columns. `columns_width`
                        // is the last pass's measured width;
                        // `abilities::show` discards a pass that measures
                        // others.
                        let columns = abilities::columns_width(ui);
                        let window_right = ui.ctx().content_rect().max.x;
                        let gap = ((window_right - frame_right - columns) / 4.0).max(0.0);
                        let pad = (frame_right + gap - ui.cursor().min.x).max(0.0);
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.vertical(|ui| {
                                // The columns' stored total and the two
                                // gaps between them.
                                ui.set_width(columns + 2.0 * gap);
                                info_strip::show(ui, view);
                                abilities::show(ui, view, gap);
                                appearance_section(ui, view);
                            });
                        });
                    }
                });
        });
    }
    // After the panes, so the logo paints over their frames' fill where
    // the right column's frame starts under it.
    if view.session.is_some() {
        event_logo::show(ui, view, toolbar_end);
    }
    team::kit_slots_dialog(ui, view);
    menus::boglo_dialog(ui, view);
    menus::stat_num_dialog(ui, view);
    menus::copy_swap_dialog(ui, view);
    menus::fix_database_dialog(ui, view);
    multi_aatf::dialog(ui, view);
    generate::dialog(ui, view);
    comparator::show(ui, view);
    patch::show(ui, view);
    interchange::show(ui, view);
    prompt::show(ui, view, running);
    prompt::edit_mode_dialog(ui, view, running);
    ruleset_picker::dialog(ui, view);
    // Last so the newest refusal, raised by a modal's own Save, sits
    // above the modal that stays up behind it.
    refusal_dialog(ui, view);
    // One undo entry per gesture: seal when the pointer is up and no
    // widget holds the keyboard, and when the widget that held it ends
    // the frame without it — a gesture ends when its widget loses the
    // keyboard. A `None` to `Some` move never seals (the `focused` doc).
    let (focused, settled) = edit_settled(ui, view.focused);
    if settled && let Some(session) = view.session.as_mut() {
        session.seal();
    }
    view.focused = focused;
}

/// The widget holding the keyboard this frame and whether the edit has
/// settled: the pointer up with no widget focused, or the widget that
/// held it having lost it. `show`'s seal condition, which the AATF
/// checks' start shares, so a check never runs over a mid-gesture file.
fn edit_settled(ui: &egui::Ui, last_focused: Option<egui::Id>) -> (Option<egui::Id>, bool) {
    let focused = ui.ctx().memory(|memory| memory.focused());
    let settled = (!ui.input(|input| input.pointer.any_down()) && focused.is_none())
        || (last_focused.is_some() && last_focused != focused);
    (focused, settled)
}

/// Whatever holds the keyboard gives it up. A run's start waits for
/// `edit_settled`, which a focus kept by the field or the button would
/// hold off, and a button activated without a pointer press (Tab + Space,
/// or AccessKit's Click — a `FAKE_PRIMARY_CLICKED` that moves no focus)
/// leaves the keyboard focus where it was.
pub(super) fn surrender_focus(ui: &egui::Ui) {
    if let Some(id) = ui.ctx().memory(|memory| memory.focused()) {
        ui.ctx().memory_mut(|memory| memory.surrender_focus(id));
    }
}

/// The editor's focus mode: a press on another widget ends the focused
/// widget's keyboard gesture on the press frame (egui's own default,
/// `Clicks`, waits for a completed click, so a drag elsewhere would keep
/// the field focused — its undo entries would merge and Ctrl+Z would
/// stay swallowed while `typing` holds). The option is the context's, so
/// once the editor has drawn it holds for every view, which is the
/// behaviour wanted there too; it moves to the shell's context setup
/// when the shell gets one.
fn set_surrender_on_press(ctx: &egui::Context) {
    ctx.memory_mut(|memory| {
        memory.options.input_options.surrender_focus_on = egui::SurrenderFocusOn::Presses;
    });
}

/// The toolbar (the draft's toolbar row), its groups apart by separators:
/// the Open menu, Save and Save as; Undo and Redo; the menus; the ruleset
/// dropdown, the Live AATF toggle and the unsaved-changes marker. A menu
/// bar, so with one menu open the pointer entering another's button opens
/// that one. The open save's path rides the idle window title
/// (`Tool::title_detail`), not this row, which never takes a second row.
/// Returns the x where the row's last widget ends.
fn toolbar(ui: &mut egui::Ui, view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) -> f32 {
    // Every dialog holds every shortcut dead while up (its own buttons
    // are the way out): egui's top modal layer covers whatever opened one,
    // and the two fields cover the prompt and the running-game question on
    // the frame they open, before their layer exists.
    let modal_up = view.prompt.is_some()
        || view.edit_mode_question.is_some()
        || ui.ctx().memory(|memory| memory.top_modal_layer()).is_some();
    // Ctrl+S saves even while a widget holds the keyboard; the other
    // shortcuts do nothing then (a text field's own undo is egui's).
    let save_key =
        !modal_up && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::S));
    let typing = ui.ctx().egui_wants_keyboard_input() || modal_up;
    // Ctrl+Shift+Z first: `consume_key(Ctrl, Z)` matches it too, so the more
    // specific shortcut has to be tested before the plainer one.
    let redo_key = !typing
        && ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                || input.consume_key(egui::Modifiers::CTRL, egui::Key::Y)
        });
    let undo_key = !typing
        && !redo_key
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Z));
    // Ctrl+F is Clear visual flags on the selected row's team (a player's
    // or a team row's own): an aesthetics write, so a no-op under the
    // lock, and a no-op while a widget holds the keyboard (`typing`).
    let clear_flags_key =
        !typing && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F));
    // Ctrl+I is Toggle FPC settings, the same lock as Ctrl+F; Ctrl+B
    // opens the Bump dialog, a gameplay write the lock does not stop.
    let fpc_key =
        !typing && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::I));
    let bump_key =
        !typing && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::B));
    // PageDown and PageUp walk the list's players from any field, so they
    // are not held off while a widget has the keyboard. A held pointer
    // button holds them off, and so does the frame it is released on,
    // where a click completes: the drag or press in flight would finish
    // on the next player.
    let gesture = ui.input(|input| input.pointer.any_down() || input.pointer.any_released());
    let page_step = if modal_up || gesture {
        None
    } else {
        ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::PageDown) {
                Some(true)
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::PageUp) {
                Some(false)
            } else {
                None
            }
        })
    };
    let end = studio_core::widgets::toolbar(ui, |ui| {
        open_menu::show(ui, view);
        if add_enabled(ui, view.session.is_some(), egui::Button::new("Save")).clicked() || save_key
        {
            save(view, running);
        }
        if add_enabled(ui, view.session.is_some(), egui::Button::new("Save as")).clicked() {
            save_as(view, running);
        }
        toolbar_separator(ui);
        let (can_undo, can_redo) = view
            .session
            .as_ref()
            .map(|session| (session.can_undo(), session.can_redo()))
            .unwrap_or((false, false));
        if (add_enabled(ui, can_undo, egui::Button::new("Undo")).clicked() || undo_key)
            && let Some(session) = view.session.as_mut()
        {
            session.undo();
        }
        if (add_enabled(ui, can_redo, egui::Button::new("Redo")).clicked() || redo_key)
            && let Some(session) = view.session.as_mut()
        {
            session.redo();
        }
        toolbar_separator(ui);
        menus::player_menu(ui, view);
        menus::team_menu(ui, view);
        menus::savefile_menu(ui, view);
        toolbar_separator(ui);
        // The ruleset beside the check it drives, whether the check is on
        // or not: the quick actions and the card counts use it too.
        ruleset_picker::show(ui, view);
        // The Live AATF toggle as a framed button, since the toggle is how
        // a manager finds the check at all: the play sign (`▶`) then the
        // name — a check mark beside it read as on while it was off — the
        // selection fill while on; a selectable label read as a label
        // beside the menus.
        if ui
            .add(egui::Button::new("\u{25b6} Live AATF").selected(view.live_aatf))
            .on_hover_text(
                "Check the open team after every edit, in a panel at the bottom (the setting is \
                 remembered)",
            )
            .clicked()
        {
            surrender_focus(ui);
            view.live_aatf = !view.live_aatf;
        }
        // What the check asks for, so only while it runs.
        if view.live_aatf
            && studio_core::widgets::checkbox(ui, &mut view.live_suggestions, "Show suggestions")
                .on_hover_text(
                    "Also list each player's unused allowances (cards, free tricks, weak foot...)",
                )
                .clicked()
        {
            surrender_focus(ui);
        }
        // The save's path is in the window title (`title_detail`): beside the
        // unsaved mark it could wrap the toolbar onto a second row.
        changes_popup::show(ui, view);
        // The bar spans the view's width; its last widget ends one item
        // spacing before the cursor.
        ui.cursor().min.x - ui.spacing().item_spacing.x
    })
    .inner;
    if clear_flags_key
        && !view.aesthetics_locked()
        && let Some(team_id) = selected_team_id(view)
        && let Some(session) = view.session.as_mut()
    {
        session.clear_visual_flags(team_id);
    }
    if fpc_key
        && !view.aesthetics_locked()
        && let Ok(ids) = view.hide_ids
        && let Some((_, _, player_id)) = selected_player(view)
        && let Some(session) = view.session.as_mut()
    {
        session.toggle_fpc(player_id, ids);
    }
    if bump_key
        && let Some((_, _, player_id)) = selected_player(view)
        && let Some(session) = view.session.as_mut()
    {
        view.stat_num = Some(menus::StatNum::bump(session, player_id, view.bump));
    }
    if let Some(forward) = page_step
        && player_list::step_player(view, forward)
    {
        // The field gives up the keyboard once the selection moved: its
        // gesture ends as one undo entry on the player it edited, and the
        // next player's field does not inherit the focus and merge his
        // edits into that entry. A key that moved nothing leaves it be.
        surrender_focus(ui);
        view.reveal_selected = true;
    }
    end
}

/// The window's Save (the toolbar's, Ctrl+S's and the prompt's): `save_to`
/// the open file. Answers whether the file was written — `false` while the
/// question waits or a write failed, so a prompt stays up.
pub(super) fn save(view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) -> bool {
    let Some(path) = view.save_path().map(Path::to_path_buf) else {
        return false;
    };
    save_to(view, running, path)
}

/// The toolbar's Save as: the picker, on the open file's folder and name,
/// then `save_to` the picked path. A cancelled picker does nothing.
fn save_as(view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) {
    let Some(current) = view.save_path().map(Path::to_path_buf) else {
        return;
    };
    if let Some(target) = (view.save_as_dialog)(&current) {
        save_to(view, running, target);
    }
}

/// Writes the open save to `target`: the running-game question first while
/// the save's own PES runs (its Save then writes to `target`), else the
/// write. Answers whether the file was written.
fn save_to(view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>, target: PathBuf) -> bool {
    let Some(version) = view.session.as_ref().map(|session| session.version()) else {
        return false;
    };
    if running().iter().any(|game| game.version == version) {
        view.edit_mode_question = Some(EditModeQuestion {
            game: version,
            target,
        });
        return false;
    }
    write_now(view, running, &target, false)
}

/// `session.save_to(target)` and the refusal its error takes (the draft's
/// `writeSave`); a written file goes to the top of the Open menu's recent
/// files. `edit_mode_closed` lets the write go ahead while the save's own
/// PES runs: the toolbar's Save passes `false` (the own-game check ran
/// first), the running-game dialog's Save `true`, the dialog having told
/// the member how to keep the write.
pub(super) fn write_now(
    view: &mut SaveView,
    running: &dyn Fn() -> Vec<RunningPes>,
    target: &Path,
    edit_mode_closed: bool,
) -> bool {
    let Some(session) = view.session.as_mut() else {
        return false;
    };
    match session.save_to(target, &running(), edit_mode_closed) {
        Ok(()) => {
            view.refusal = None;
            open_menu::push_recent(&mut view.recents, target);
            true
        }
        Err(error) => {
            view.refusal = Some(Refusal {
                title: "Save refused",
                message: format!("{error:#}"),
            });
            false
        }
    }
}

/// Refuses an export's `destination` when it is `save`, a save being read,
/// before the `what` (the export's name) is written over it: compared
/// canonicalized, so one file under two names still matches, and a path
/// that does not exist yet is no match.
pub(super) fn refuse_save_being_read(
    destination: &Path,
    save: &Path,
    what: &str,
) -> anyhow::Result<()> {
    if let (Ok(destination_file), Ok(save_file)) = (
        std::fs::canonicalize(destination),
        std::fs::canonicalize(save),
    ) && destination_file == save_file
    {
        anyhow::bail!(
            "{}: that is the save being read, and a {what} written over it would lose it",
            destination.display()
        );
    }
    Ok(())
}

/// Clones the player's entry, applies `edit` and pushes it under `key` (the
/// widget's `egui::Id` value, the undo stack's merge key).
pub(crate) fn edit_player(
    session: &mut Session,
    id: u32,
    key: u64,
    edit: impl FnOnce(&mut PlayerEntry),
) {
    let mut edited = session
        .file()
        .player(id)
        .expect("the selected player")
        .clone();
    edit(&mut edited);
    session.edit_player(key, edited);
}

/// `edit_player` for the team's entry (General info's Shirt number and
/// Captain, the team view's fields, the Tactics editor's edited clone).
pub(crate) fn edit_team(
    session: &mut Session,
    id: u32,
    key: u64,
    edit: impl FnOnce(&mut TeamEntry),
) {
    let mut edited = session.file().team(id).expect("the selected team").clone();
    edit(&mut edited);
    session.edit_team(key, edited);
}

/// The selected row's team id — a player row's team or a team row's own;
/// `None` with no row selected.
pub(crate) fn selected_team_id(view: &SaveView) -> Option<u32> {
    match view.selected? {
        Selection::Team(id) | Selection::Player(id, _) => Some(id),
    }
}

/// The selected player's `(team_id, roster slot, player_id)`; `None` when
/// the selection names no rostered player (a team row, a stale slot, an
/// empty save's list).
pub(crate) fn selected_player(view: &SaveView) -> Option<(u32, u8, u32)> {
    let Selection::Player(team_id, slot) = view.selected? else {
        return None;
    };
    let player_id = view.session.as_ref()?.slot_player(team_id, slot)?;
    Some((team_id, slot, player_id))
}

/// The selected row's `(team_id, save's version, team's entry cloned)`:
/// the team the row names — a player row's team or a team row's own;
/// `None` when the selection names no team or the entry is missing (the
/// team widgets' shared head).
pub(crate) fn selected_team(view: &SaveView) -> Option<(u32, PesVersion, TeamEntry)> {
    let team_id = match view.selected? {
        Selection::Team(team_id) | Selection::Player(team_id, _) => team_id,
    };
    let file = view.session.as_ref()?.file();
    let team = file.team(team_id)?.clone();
    Some((team_id, file.version(), team))
}

/// The refusal's dialog, while `view.refusal` holds one.
fn refusal_dialog(ui: &mut egui::Ui, view: &mut SaveView) {
    let mut close = false;
    if let Some(refusal) = view.refusal.as_ref() {
        let id = egui::Id::unique("save_editor_refusal");
        let response = modal(ui.ctx(), id, refusal.title, |ui| {
            ui.label(&refusal.message);
            modal_buttons(ui, |ui| {
                if ui.button("OK").clicked() {
                    close = true;
                }
            });
        });
        close |= response.should_close();
    }
    if close {
        view.refusal = None;
    }
}
