//! The Save editor's window: a toolbar over a master-detail layout, the
//! player list left (`player_list`, its team and player rows, the quick
//! actions under it) and, right, the screen the selected row and the
//! title's toggle name: `Gameplan`, the team view (`team`), on a team
//! row; the player's `Stats` — the three columns (`abilities` with
//! `info_strip`'s General info and Main stats) — or `Appearance`
//! (`appearance`) on a player row; every screen under the same title's
//! row (`info_strip`) with the toggle at its left. Render-only over the
//! session's state: it opens a save, filters and selects players, reorders
//! a team's list by drag (`Session::move_player`), edits the strip's fields through
//! `Session::edit_player`/`edit_team` (one undo entry per gesture), saves, and
//! shows a refusal in a modal.

mod aatf_check;
mod abilities;
mod appearance;
mod changes_popup;
mod clipboard;
mod comparator;
mod copy_stats;
mod event_logo;
mod file_menu;
mod generate;
mod group_frame;
mod info_strip;
mod interchange;
mod live_aatf;
mod live_marks;
mod menus;
mod multi_aatf;
mod patch;
mod pictures;
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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pes_savefile::model::{PlayerEntry, TeamEntry};
use pes_version::PesVersion;
use studio_core::shell::DEFAULT_PANE_WIDTH;
use studio_core::widgets::{
    add_enabled, consume_ctrl, menu_button, menu_separator, modal, modal_buttons, resting_edge,
    toolbar_framed, toolbar_separator,
};
use studio_core::{ExeCheck, Notice, NoticeKind, Recorder, RunningPes};

use crate::session::Session;
use crate::session::journal;
use crate::session::rules::{self, Imported};
use crate::session::selection::{self, TeamCategory, TeamChoice};
use crate::settings::RecentSave;

pub(crate) use abilities::PositionsLayout;

/// The list row the selection names: a team by id, or a player by team
/// and roster slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    /// A team's own row (the detail shows the team view).
    Team(u32),
    /// A player row: the team he is rostered on and his roster slot.
    Player(u32, u8),
}

/// The player screen the detail's toggle shows on a player row (`Stats`
/// or `Appearance`; the third segment, `Gameplan`, is the team view a
/// team row selects). `Stats` when a save opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerScreen {
    /// The three columns (General info, Skills, Ability stats).
    Stats,
    /// The Appearance content, a screen of its own.
    Appearance,
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

/// How far the list scrolls to the selected row (`SaveView::reveal_selected`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reveal {
    /// The least that shows it.
    Nearest,
    /// To the list's top edge, as far as the list's end allows, so a team
    /// row's players stand under it.
    Top,
}

/// The `Reveal` for the selected row by its kind: `Top` for a team's row,
/// so its players stand under it, `Nearest` for a player's row. The arrow
/// keys, the page toggle's segments and a team row's double click set
/// `Nearest` themselves, even on a team's row.
fn reveal_for(selected: Option<Selection>) -> Reveal {
    if matches!(selected, Some(Selection::Team(_))) {
        Reveal::Top
    } else {
        Reveal::Nearest
    }
}

/// The window's state: the open save and what the master-detail shows of it.
pub(crate) struct SaveView {
    /// The open save, if one was opened.
    pub(crate) session: Option<Session>,
    /// The repro journal `Session::open` and the session's changes record
    /// into: `Tool::view` refreshes it from `ctx.recorder()` each frame, a
    /// default one in the tests' view-only frames.
    pub(crate) recorder: Recorder,
    /// Which team the list shows.
    team: TeamChoice,
    /// The category the selector's radio buttons pick: the dropdown's and
    /// the list's teams are its alone. `Tool::view` syncs it with the
    /// `team_category` setting as `scroll_names` is synced, so it stands
    /// when another save opens.
    pub(crate) team_category: TeamCategory,
    /// The layout the Playable positions grid draws in (the toggle under
    /// the panel's title): `Tool::view` syncs it with the
    /// `positions_layout` setting as `scroll_names` is synced, and a
    /// press on a segment writes the pick back.
    pub(crate) positions_layout: abilities::PositionsLayout,
    /// The filter box's text.
    filter: String,
    /// The selected row.
    selected: Option<Selection>,
    /// The player rows selected together, as `(team id, roster slot)`
    /// with `selected`'s own player first: a Ctrl+click on a row adds
    /// it or takes it out. On while it holds two or more and its first
    /// entry is the selected row (`group_on`); every other way of
    /// selecting empties it.
    group: Vec<(u32, u8)>,
    /// The player screen the title's toggle shows on a player row
    /// (`Stats` on a save's open). The toggle's third segment,
    /// `Gameplan`, is the team view a team row selects — the selection
    /// holds it, never this field.
    page: PlayerScreen,
    /// Per team, the roster slot of the player last shown on it since the
    /// save opened (emptied on open): the row a player screen's segment
    /// selects from the team's row, the team's first filled slot when
    /// none was shown.
    last_shown: HashMap<u32, u8>,
    /// The Tactics selector the last press named for Ctrl+C and Ctrl+V
    /// (the preset's or the formation's, its team's id with it); `None`
    /// means the selected row. `show` clears a `Some` whose team is no
    /// longer the selected row at the frame's end, so the target lives
    /// only while its team's row stays selected.
    click_kind: Option<clipboard::ClickKind>,
    /// How the list scrolls to the selected row on its next draw, which
    /// clears this. Set where the selection may land off screen: a row
    /// selected from elsewhere (an unsaved change, a violation, the
    /// Comparator, the import window), the keys and the page toggle that
    /// walk the list, a team row's double click, an undo or a redo, and a pick in
    /// the team selector or category that names a team or relists the
    /// rows around the selection.
    reveal_selected: Option<Reveal>,
    /// The team view's Tactics state (`team_widgets`' `TacticsView`:
    /// the preset its panels edit, the formation its lists show, the
    /// armed assignment mode, the open slot popover, Scroll and Portraits);
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
    /// The ask to close the window, set by the File menu's `Exit` and by
    /// `go_on` running `Pending::Close` — the close prompt's Discard or
    /// written Save, or its last row's × that left the save clean.
    /// Drained by `Tool::view` into `ctx.request_close()` (the shell owns
    /// the send).
    pub(crate) request_close: bool,
    /// The File menu's `Open…` picker (`rfd`, stubbed in tests;
    /// `pub(crate)` so the crate root's tests reach it too).
    pub(crate) open_dialog: file_menu::OpenDialog,
    /// Save as's picker (`rfd`, stubbed in tests).
    save_as_dialog: SaveAsDialog,
    /// The File menu's discovery (`discover_savefiles`, stubbed in tests;
    /// `pub(crate)` so the crate root's tests reach it too).
    pub(crate) discover: file_menu::Discover,
    /// What the File menu's look found when it opened: the discovered
    /// saves and the recent saves still on disk; `None` while the menu is
    /// closed, so the next opening looks again.
    discovered: Option<file_menu::Found>,
    /// The welcome panel's own look, taken once when it appears (the
    /// tool's first frame, or a save closing) and kept while it stays up,
    /// never a scan a frame; `open` drops it, so the next stay looks again.
    welcome_discovered: Option<file_menu::Found>,
    /// The last 10 saves opened or saved, newest first — `Tool::view` reads
    /// the `recent_files` setting into the list at start and writes it back
    /// when the list moves (`pub(crate)` so it and the crate root's tests
    /// reach it): the File menu's and the welcome panel's Recent files. A
    /// path gone from disk is skipped when shown and kept in the setting.
    pub(crate) recents: Vec<RecentSave>,
    /// The folder the last save was opened from: `Tool::view` reads the
    /// `last_folder` setting into it on its first frame and writes it back
    /// when an open changes it: `Open…` and the Open shortcuts' picker
    /// start there while the folder exists. `None` in the tests' view-only
    /// frames until a test sets it.
    pub(crate) last_folder: Option<PathBuf>,
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
    /// The ruleset the quick actions, the Cards used badge, the medal
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
    /// The Pick-a-folder dialog of the Generate dialog, of the formation
    /// picture dialog's Browse and of the pitch's Set PES path button
    /// (`rfd`, stubbed in tests — the same pattern as `open_save_dialog`).
    pick_folder_dialog: generate::PickFolderDialog,
    /// The formation picture dialog's state while it is open (the Team
    /// menu's `Save a picture of the formations…`); `None` while it is
    /// closed.
    picture: Option<pictures::PictureDialog>,
    /// The range picture dialog's state while it is open (the Savefile
    /// menu's `Save formation pictures for a range of teams…`); `None` while
    /// it is closed.
    pictures_range: Option<pictures::RangeDialog>,
    /// The `formation_pictures_folder` setting: `Tool::view` refreshes it
    /// each frame, a picture's Save sets the folder it wrote to and
    /// `Tool::view` writes that back, as `positions_layout` is. A relative
    /// path starts from `exe_dir`.
    pub(crate) formation_pictures_folder: String,
    /// The picture dialog's "Without the pitch's grain" box, refreshed
    /// from and written back to the `formation_pictures_flat` setting as
    /// `live_aatf` is.
    pub(crate) formation_pictures_flat: bool,
    /// The picture dialogs' "With each preset's settings beside its row"
    /// box, refreshed from and written back to the
    /// `formation_pictures_settings` setting as `live_aatf` is.
    pub(crate) formation_pictures_settings: bool,
    /// The program's own folder, where a picture's default folder sits:
    /// `Tool::view` sets it on its first frame; empty in the tests'
    /// view-only frames.
    pub(crate) exe_dir: PathBuf,
    /// The exe check of the open save's version's PES folder, refreshed from
    /// the context each `Tool::view` frame: while it is `Some` and not
    /// `Found`, the pitch carries the Set PES path button. `None` in the
    /// tests' view-only frames until a test sets it.
    pub(crate) pes_exe_check: Option<ExeCheck>,
    /// The PES folder the open save's version's folder resolves to, refreshed
    /// with `pes_exe_check`: the Set PES path button's tooltip names it.
    /// `None` in the tests' view-only frames until a test sets it.
    pub(crate) pes_folder: Option<PathBuf>,
    /// The exe check `select_portraits` last saw: `Found` after another
    /// verdict restarts the portrait source, the folder having been repaired
    /// where a worker answered empty. A re-picked folder restarts where the
    /// pick lands (`tactics::show`).
    portraits_check: Option<ExeCheck>,
    /// The save's version and the folder the Set PES path button's picker
    /// returned, drained by `Tool::view` into `ctx.set_pes_folder_for` (the
    /// shell owns the settings).
    pub(crate) picked_pes_folder: Option<(PesVersion, PathBuf)>,
    /// The status-bar notice the last action raised; `Tool::view` forwards
    /// it to the shell's `notify` after the frame.
    pub(crate) notice: Option<Notice>,
    /// The Live AATF toolbar toggle's state: `Tool::view` refreshes it
    /// from the `live_aatf` setting and writes the setting back when the
    /// toggle changes it; off in the tests' view-only frames.
    pub(crate) live_aatf: bool,
    /// The Show suggestions checkbox beside the toggle, refreshed from and
    /// written back to the `live_suggestions` setting as `live_aatf` is:
    /// on, Live AATF's check asks for the suggestions too.
    pub(crate) live_suggestions: bool,
    /// Live AATF's list in a window of its own rather than its dock,
    /// refreshed from and written back to the `live_window` setting as
    /// `live_aatf` is; off (docked) in the tests' view-only frames.
    pub(crate) live_window: bool,
    /// The Live AATF dock's and window's state (the list docked under
    /// the panes, or in its own window); drawn while `live_aatf` and a
    /// save is open.
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
    /// The "Cards used" badge's last answer with the `BadgeKey` it ran
    /// for, and the run in flight: `player_tier` then `card_limits` on a
    /// worker thread, each reading the whole team under every candidate
    /// height system (50 to 70 ms apiece on a PES 2017 save). On the UI
    /// thread they made one wheel notch over a stat box cost over 100 ms.
    cards: abilities::Cards,
    /// The pitch badges' medal letters per roster slot, computed on a
    /// worker thread for a `MedalKey`: the last answer with its key and
    /// the run in flight. `player_tier` runs again only when the key
    /// differs, not on every frame.
    medals: tactics::Medals,
    /// The Tactics pitch's portraits, read from the selected game's
    /// `download/` CPKs off the UI thread (`select_portraits` points it at
    /// the PES folder each frame).
    portraits: portraits::Portraits,
    /// Which open this is: part of the pushed widget id and of the
    /// list's scroll area id, so egui widget and scroll state (a text
    /// field's undo history, a control's in-progress edit, the scrolled
    /// offset) never crosses into the next save's same player id.
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
    /// The Copy/swap stats window's state while it is open (the Player
    /// menu's `…` item); `None` while it is closed.
    copy_swap: Option<copy_stats::CopySwap>,
    /// The Fix database confirmation is open (the Savefile menu's item);
    /// `false` while it is closed.
    fix_database: bool,
    /// The Kit slot assignments dialog's team while it is open (the team
    /// view's `Kit slots…` button); `None` while it is closed.
    kit_slots: Option<u32>,
    /// The hide IDs the common settings give (`fpc_hide_boots_id` and
    /// `fpc_hide_gloves_id`), refreshed from `ctx.common()` each `Tool::view`
    /// frame: every FPC path (the Appearance section's Hide/Show default
    /// body, Ctrl+I, the classifier's reading, the Team menu's Hide
    /// default body (Enable FPC) and Show default body (Disable FPC))
    /// reads them, and an out-of-band pair disables those paths behind
    /// the error's text.
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

/// What the Skills column's Cards used badge was computed for: a change of
/// the open save (`Session::revision`), of the selection or of the
/// ruleset recomputes it; anything else keeps a frame's `player_tier` and
/// `card_limits` cost off.
#[derive(Clone, PartialEq)]
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
/// them. Until the new ones land, the badges draw the kept ones if only
/// the revision moved, and none for another team or ruleset.
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
            recorder: Recorder::default(),
            team: TeamChoice::All,
            team_category: TeamCategory::All,
            positions_layout: abilities::PositionsLayout::Pitch,
            filter: String::new(),
            selected: None,
            group: Vec::new(),
            page: PlayerScreen::Stats,
            last_shown: HashMap::new(),
            click_kind: None,
            reveal_selected: None,
            tactics: team_widgets::tactics::TacticsView::default(),
            focused: None,
            prompt: None,
            changes_list: None,
            edit_mode_question: None,
            request_close: false,
            open_dialog: file_menu::default_open_dialog(),
            save_as_dialog: default_save_as_dialog(),
            discover: file_menu::default_discover(),
            discovered: None,
            welcome_discovered: None,
            recents: Vec::new(),
            last_folder: None,
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
            picture: None,
            pictures_range: None,
            formation_pictures_folder: String::new(),
            formation_pictures_flat: false,
            formation_pictures_settings: false,
            exe_dir: PathBuf::new(),
            pes_exe_check: None,
            pes_folder: None,
            portraits_check: None,
            picked_pes_folder: None,
            notice: None,
            live_aatf: false,
            live_suggestions: false,
            live_window: false,
            live: live_aatf::LiveAatf::new(),
            multi_aatf: multi_aatf::MultiAatf::new(),
            export_dialog: comparator::default_export_dialog(),
            rosters_dialog: menus::default_rosters_dialog(),
            cards: abilities::Cards::new(),
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
    /// each frame with the open save's PES and its version's PES folder, as it refreshes `hide_ids`.
    /// The exe check turning `Found` restarts the source: the folder it answers for was
    /// repaired where a worker read empty, so its `None` answers are stale.
    pub(crate) fn select_portraits(
        &mut self,
        ctx: &egui::Context,
        pes_folder: &Path,
        version: PesVersion,
    ) {
        let repaired = self.pes_exe_check == Some(ExeCheck::Found)
            && self
                .portraits_check
                .is_some_and(|check| check != ExeCheck::Found);
        self.portraits_check = self.pes_exe_check;
        self.portraits.select(ctx, pes_folder, version);
        if repaired {
            self.portraits.restart(ctx);
        }
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

    /// Opens the save at `path`: on success the selector reads `All
    /// teams`, the filter is empty and the first player row is selected
    /// (a team's own row is not a player); a save holding no cup team
    /// selects nothing and raises a warning notice saying so. A file the
    /// save refuses is shown as a refusal and the open session is untouched.
    /// The aesthetics lock is deliberately not reset (`edit_aesthetics`).
    /// `Session::open` records the open — a refused one names the file
    /// that failed and keeps the bytes it read.
    pub(crate) fn open(&mut self, path: &Path) {
        match Session::open(path, self.recorder.clone()) {
            Ok(session) => {
                self.drop_save();
                self.selected = selection::list(session.file(), &self.team, "", self.team_category)
                    .into_iter()
                    .find_map(|row| match row {
                        selection::ListRow::Player(row) => {
                            Some(Selection::Player(row.team_id, row.slot))
                        }
                        selection::ListRow::Team(_) => None,
                    });
                self.comparator.reset(session.version(), &self.recorder);
                file_menu::push_recent(&mut self.recents, path, session.version());
                self.last_folder = path.parent().map(Path::to_path_buf);
                // A save holding no cup team (one made from the stock game)
                // opens to an empty list; the notice says why.
                if session.cup_team_ids().next().is_none() {
                    self.notice = Some(Notice {
                        text: "This save holds no cup team (ids 701 to 920).".to_owned(),
                        kind: NoticeKind::Warning,
                        action: None,
                    });
                }
                self.session = Some(session);
                self.generation += 1;
            }
            Err(error) => {
                self.refusal = Some(Refusal {
                    title: "Cannot open the save",
                    message: format!("{error:#}"),
                });
            }
        }
    }

    /// The open save's half of the view back to a start's — called by an
    /// open over it and by `close_save`: the selector, filter, selection
    /// and player screen, the save's dialogs and popups, a refusal, the
    /// checks' and badges' worker results and runs in flight (a Live AATF
    /// reply landing after the drop changes nothing: its channel is
    /// dropped with them), and the welcome panel's scan, which the panel
    /// runs again. The member's view settings (`scroll_names`, `portraits`),
    /// `recents` and `last_folder` are not the save's to drop.
    fn drop_save(&mut self) {
        self.team = TeamChoice::All;
        self.filter.clear();
        self.page = PlayerScreen::Stats;
        self.last_shown.clear();
        self.click_kind = None;
        self.selected = None;
        self.group.clear();
        // View settings, not the save's: `scroll_names` and `portraits`
        // are the member's, so they survive (the rest is the save's:
        // back to preset 1 and kick-off).
        let scroll_names = self.tactics.scroll_names;
        let portraits = self.tactics.portraits;
        self.tactics = team_widgets::tactics::TacticsView::default();
        self.tactics.scroll_names = scroll_names;
        self.tactics.portraits = portraits;
        self.prompt = None;
        self.changes_list = None;
        self.edit_mode_question = None;
        self.refusal = None;
        self.boglo = None;
        self.stat_num = None;
        self.copy_swap = None;
        self.generate = None;
        self.picture = None;
        self.pictures_range = None;
        self.fix_database = false;
        self.kit_slots = None;
        self.live.check.reset();
        self.multi_aatf.reset();
        self.cards.reset();
        self.medals.reset();
        self.welcome_discovered = None;
    }

    /// Closes the open save (File > Close save, Ctrl+W, and Discard on
    /// the window's close prompt): records the close in the journal, clears
    /// the session, any open floating panel and the Comparator, then resets
    /// the save's view state (`drop_save`).
    pub(crate) fn close_save(&mut self) {
        if let Some(session) = self.session.take() {
            journal::closed(session.path(), session.recorder());
        }
        self.panel = None;
        self.comparator = comparator::Comparator::new();
        self.drop_save();
    }

    /// The open save's version, `None` with none open: `Tool::view` reads it
    /// for the per-version asks (the exe check, the PES folder, the pick's
    /// store): the editor follows the save's version, never the selector.
    pub(crate) fn save_version(&self) -> Option<PesVersion> {
        self.session.as_ref().map(Session::version)
    }

    /// The open save's state files for `Tool::repro_state` (the repro
    /// bundle's `state/`): the save encoded as it would be written now,
    /// named as its file; none with no save open.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn repro_state(&self) -> Vec<(String, Vec<u8>)> {
        self.session
            .as_ref()
            .and_then(Session::repro_state)
            .into_iter()
            .collect()
    }

    /// The PES folder the portraits read, `None` for "no folder" (the
    /// Set PES path tests' premise).
    #[cfg(test)]
    pub(crate) fn portrait_folder(&self) -> Option<&Path> {
        self.portraits.folder()
    }
}

/// The player list's column, the draft's `.plist`: the rows' frame and
/// the scroll bar's strip beside it, the panel's margins not included.
const LIST_WIDTH: f32 = 255.0;

/// A number box's width, the drafts' number input: the quick actions', the
/// boots and gloves dialog's and the number dialogs' fields. egui's own
/// least widget width is 40.
pub(super) const NUMBER_BOX: f32 = 52.0;

/// The detail pane's margin over a view's title's row (the draft's
/// `.pheader` margin-top): with the panes' start and the half point
/// `info_strip::view_title` adds, the row stands 9.5 under the toolbar
/// band's line.
const TITLE_MARGIN: i8 = 6;

/// The player list's margin over its team combo. The panes start one item
/// spacing (3 points) under the toolbar band's line, so the combo stands 4
/// under it, the draft's margin under the band.
const LIST_TOP_MARGIN: i8 = 1;

/// The room between the panes' bottom and the status bar (the draft's
/// `.center.filled` bottom padding).
const PANE_BOTTOM_MARGIN: f32 = 6.0;

/// The room between the window's right edge and the detail pane's own
/// scroll bar (the draft's `.pane-bar`): the pane reaches into the
/// shell's right margin for it, as it reaches into the bottom margin
/// for `PANE_BOTTOM_MARGIN`. With the bar's 10-point strip, the screens'
/// frames end 14 before the window's edge.
const PANE_BAR_MARGIN: f32 = 4.0;

/// The panes' room past the shell's 8-point left margin: the player
/// list's column stands 10 after the sidebar (the draft's
/// `.center.filled` left padding), the toolbar's first menu over it
/// (`BAND_MARGIN`). The detail pane keeps its width: it starts this much
/// later and reaches as much further right (`PANE_BAR_MARGIN`).
const PANES_LEFT_INSET: f32 = 2.0;

/// Shows the window for one frame. `running` is the running-PES probe,
/// called only when Save runs (a process snapshot is not a frame's cost).
/// The ruleset is resolved from `view.aatf_ruleset` on the first frame and
/// again only when the setting changes; a setting naming a ruleset that is
/// gone raises its warning notice then.
pub(crate) fn show(ui: &mut egui::Ui, view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) {
    set_surrender_on_press(ui.ctx());
    // The undo run's clock, before any widget draws: a widget's push
    // measures the pause since its run's last step against it.
    if let Some(session) = view.session.as_mut() {
        session.set_time(ui.input(|input| input.time));
    }
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
    // The view keeps the width it has inside the central panel's margins
    // at the default window, centred across the pane (the draft's
    // `.view-body` `max-width`): its toolbar and panes lay out in
    // `view_rect`, Live AATF's dock alone spanning the pane's width and
    // the floating panels placing off its edges as at the default size.
    let central_margin = egui::Frame::central_panel(ui.style()).inner_margin;
    let space = ui.max_rect();
    let width = (DEFAULT_PANE_WIDTH - f32::from(central_margin.left + central_margin.right))
        .min(space.width());
    let view_rect = egui::Rect::from_min_max(
        egui::pos2(space.center().x - width / 2.0, space.min.y),
        egui::pos2(space.center().x + width / 2.0, space.max.y),
    );
    // The logo's rect once, before the toolbar: a save the toolbar itself
    // opens this frame (a File menu entry or a shortcut) then draws no
    // logo over a band still running to the view's right edge, and gets
    // it on the next frame, the band already ending short.
    let logo_rect = event_logo::rect(ui, view, view_rect);
    let band_end = logo_rect.map(|logo| logo.left() - event_logo::RIGHT);
    toolbar(ui, view, running, band_end, view_rect);
    // The panes under the toolbar end `PANE_BOTTOM_MARGIN` over the
    // status bar, the draft's padding under the view, where the
    // shell's central panel keeps its own margin: they reach into
    // that margin by the difference. The shell's panel clips to its
    // whole rect, margins included, so nothing of theirs is cut.
    let mut panes = ui.available_rect_before_wrap();
    panes.min.x += PANES_LEFT_INSET;
    panes.max.y += f32::from(central_margin.bottom) - PANE_BOTTOM_MARGIN;
    let panes_ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(panes));
    // The Live AATF dock spans the panes' width under the capped view
    // (the draft's `.live-panel`, outside `.view-body`), the capped
    // panes sitting above it.
    if view.session.is_some() && view.live_aatf {
        live_aatf::show(panes_ui, view);
    }
    // The view's capped part, `view_rect`'s span of what the dock leaves.
    let mut capped_rect = panes_ui.available_rect_before_wrap();
    capped_rect.min.x = view_rect.min.x + PANES_LEFT_INSET;
    capped_rect.max.x = view_rect.max.x;
    let capped = &mut panes_ui.new_child(egui::UiBuilder::new().max_rect(capped_rect));
    if view.session.is_none() {
        // The welcome panel centres in the panes' area, not the whole
        // central panel (the band sits above it).
        file_menu::welcome_panel(capped, view);
    } else {
        // A group is on only while its first entry is the selected row
        // (`group_on`): one whose selection moved another way is emptied
        // here once a frame, before the list draws, so a pick of that
        // row later does not revive it.
        if view
            .group
            .first()
            .is_some_and(|&(team_id, slot)| view.selected != Some(Selection::Player(team_id, slot)))
        {
            view.group.clear();
        }
        // No margin before the list, so its frame stands at the view's
        // left edge, and 4 points after it: with no margin of the
        // detail's own, the screens' frames stand the draft's 14 after
        // the list's rows frame — its 10-point scroll bar strip and the
        // 4. No separator line between the panes, as the draft draws
        // none. No bottom margin: the quick actions' band is the panel's
        // content's last item and runs to the pane's bottom, so the pane
        // gets no scroll bar of its own.
        let margin = egui::Margin {
            left: 0,
            right: 4,
            top: LIST_TOP_MARGIN,
            bottom: 0,
        };
        egui::Panel::left("save_editor_players")
            // The draft's `.plist`: a 255-point column (the rows' frame and
            // the scroll bar's strip), not resizable. egui's panel size
            // counts its margins too. Under the design width the window
            // scales instead of narrowing, and a list dragged wider would
            // only take the views' room.
            .resizable(false)
            .show_separator_line(false)
            .default_size(LIST_WIDTH + margin.sum().x)
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(margin))
            .show(capped, |ui| player_list::show(ui, view));
        // The detail pane reaches `PANE_BAR_MARGIN` less than the shell's
        // margin into the right edge, for its scroll bar's strip (the
        // draft's `.pane-bar` standing past the pane), as the panes reach
        // into the bottom margin for `PANE_BOTTOM_MARGIN`.
        let mut detail_rect = capped.available_rect_before_wrap();
        detail_rect.max.x += f32::from(central_margin.right) - PANE_BAR_MARGIN;
        let ui = &mut capped.new_child(egui::UiBuilder::new().max_rect(detail_rect));
        // `TITLE_MARGIN` over the view's title. None at the sides: the
        // frames take the draft's 14 after the list's frame from the list
        // pane's own 4 and its scroll bar's strip, and end the draft's 14
        // before the window's edge, the pane's bar strip past them. None
        // at the bottom: the draft's `.detail` has no padding, a tall
        // view's content ending at the pane's bottom.
        let detail = egui::Frame::central_panel(ui.style()).inner_margin(egui::Margin {
            left: 0,
            right: 0,
            top: TITLE_MARGIN,
            bottom: 0,
        });
        egui::CentralPanel::default().frame(detail).show(ui, |ui| {
            // egui's solid scroll bar, as the list's, stands 4 after the
            // content in a 10-point strip: the strip is kept whether or
            // not the pane scrolls, so the screens' frames end at one x
            // — the content caps at the strip's left edge, and the bar,
            // when it shows, fills the strip's 6, ending
            // `PANE_BAR_MARGIN` before the window's edge.
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            let content_width = ui.available_width() - ui.spacing().scroll.allocated_width();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width(content_width);
                    match view.selected {
                        Some(Selection::Team(_)) => team::show(ui, view),
                        Some(Selection::Player(..)) | None => {
                            // The pitch that owns the slot popover is gone
                            // with the team view: close it here, since its
                            // own outside-press handling no longer runs and
                            // the arrows' gate reads it.
                            view.tactics.close_popover();
                            // The player last shown on his team is the row
                            // the toggle's player screens go back to from
                            // the team's row.
                            if let Some((team_id, slot, _)) = selected_player(view) {
                                view.last_shown.insert(team_id, slot);
                            }
                            // The title's row first, at the pane's left
                            // edge: a toggle that stood elsewhere on
                            // another screen would slip from under the
                            // pointer.
                            info_strip::show(ui, view);
                            match view.page {
                                // A group shows the first player's Stats
                                // screen whichever screen stood before
                                // (`view.page` is not changed, so it
                                // comes back when the group ends).
                                PlayerScreen::Appearance
                                    if !group_on(view.selected, &view.group) =>
                                {
                                    appearance::show(ui, view)
                                }
                                _ => abilities::show(ui, view),
                            }
                        }
                    }
                });
        });
    }
    // After the panes, so the logo paints over their frames' fill where
    // the right column's frame starts under it.
    event_logo::show(ui, view, logo_rect);
    team::kit_slots_dialog(ui, view);
    menus::boglo_dialog(ui, view);
    menus::stat_num_dialog(ui, view);
    copy_stats::window(ui, view);
    menus::fix_database_dialog(ui, view);
    multi_aatf::dialog(ui, view);
    generate::dialog(ui, view);
    pictures::dialog(ui, view);
    pictures::range_dialog(ui, view);
    comparator::show(ui, view);
    patch::show(ui, view);
    interchange::show(ui, view);
    prompt::show(ui, view, running);
    prompt::edit_mode_dialog(ui, view, running);
    ruleset_picker::dialog(ui, view);
    // Last so the newest refusal, raised by a modal's own Save, sits
    // above the modal that stays up behind it.
    refusal_dialog(ui, view);
    // One undo entry per gesture, a run of steps to one value one entry
    // across gestures: the soft end (the pointer up with no text edit
    // holding the keyboard) settles, keeping the run; the keyboard
    // moving off the widget that held it seals, ending the run too — a
    // typed value and the click after it stay two entries.
    let (focused, end) = edit_settled(ui, view.focused);
    if let Some(session) = view.session.as_mut() {
        match end {
            GestureEnd::Held => {}
            GestureEnd::Settled => session.settle(),
            GestureEnd::FocusMoved => session.seal(),
        }
    }
    view.focused = focused;
    // A selector's press names the copy target only while its own
    // team's row stays selected: whatever else the selection reached —
    // another team's row, a player row — ends it, and coming back to
    // the row does not bring it back.
    if let Some(kind) = view.click_kind
        && view.selected != Some(Selection::Team(kind.team_id()))
    {
        view.click_kind = None;
    }
}

/// How the frame left the edit's gesture, the two settled cases told
/// apart for the undo stack: `show`'s end seals on one, settles on the
/// other, and the checks' start asks only `settled()`, so a check never
/// runs over a mid-gesture file.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GestureEnd {
    /// Mid-gesture: the pointer is held, or a text edit keeps the
    /// keyboard (a field, a number being typed), or a popup is open.
    Held,
    /// The pointer is up and no text edit holds the keyboard — nothing
    /// focused, or a widget with no text cursor (a button, a rating
    /// cell) keeping it: the gesture ended but a run of steps under its
    /// key may still merge (`settle`).
    Settled,
    /// The keyboard moved off the widget that held it: the hard end a
    /// run does not survive (`seal`).
    FocusMoved,
}

impl GestureEnd {
    /// Whether the edit has settled this frame: anything but mid-gesture.
    fn settled(self) -> bool {
        !matches!(self, GestureEnd::Held)
    }
}

/// The widget holding the keyboard this frame and how the frame left
/// the edit's gesture: `FocusMoved` when the widget that held it lost
/// the keyboard (a `None` to `Some` move is never one — a widget can
/// take focus as its gesture starts), `Settled` when the pointer is up
/// and no text edit holds it (`text_edit_focused`, which a focused
/// `DragValue` being typed reports as its `TextEdit` keeps its state
/// under the DragValue's own id), else `Held`. An open popup holds the
/// gesture too — a combo's open list is the gesture's own — so its
/// picks merge into the one entry, however long between them.
/// `show`'s seal rule, which the AATF checks' start shares.
fn edit_settled(ui: &egui::Ui, last_focused: Option<egui::Id>) -> (Option<egui::Id>, GestureEnd) {
    let focused = ui.ctx().memory(|memory| memory.focused());
    let end = if last_focused.is_some() && last_focused != focused {
        GestureEnd::FocusMoved
    } else if egui::Popup::is_any_open(ui.ctx()) {
        GestureEnd::Held
    } else if !ui.input(|input| input.pointer.any_down()) && !ui.ctx().text_edit_focused() {
        GestureEnd::Settled
    } else {
        GestureEnd::Held
    };
    (focused, end)
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

/// The toolbar (the draft's toolbar row): the File, Edit, Player, Team and
/// Savefile menus; a separator; the ruleset dropdown, the Live AATF toggle
/// and the unsaved-changes marker. A menu bar, so with one menu open the
/// pointer entering another's button opens that one, and its menus are
/// unframed; the dropdown and the toggle keep their frames, since a flat
/// combo reads as one more menu and a flat toggle off as a label. The
/// open save's path rides the idle window title
/// (`Tool::title_detail`), not this row, which never takes a second row.
/// The row stands on its band (`toolbar_band`), which ends at `band_end`,
/// short of the event logo, or with `None` at `view_rect`'s right edge.
fn toolbar(
    ui: &mut egui::Ui,
    view: &mut SaveView,
    running: &dyn Fn() -> Vec<RunningPes>,
    band_end: Option<f32>,
    view_rect: egui::Rect,
) {
    // Every dialog holds every shortcut dead while up (its own buttons
    // are the way out): egui's top modal layer covers whatever opened one,
    // and the two fields cover the prompt and the running-game question on
    // the frame they open, before their layer exists.
    let modal_up = view.prompt.is_some()
        || view.edit_mode_question.is_some()
        || ui.ctx().memory(|memory| memory.top_modal_layer()).is_some();
    // Ctrl+S saves even while a text field holds the keyboard, and
    // Ctrl+Shift+S opens Save as the same way (its own two-modifier match,
    // as Ctrl+Shift+Z's is); the other shortcuts are held off then (a text
    // field's own undo is egui's). A button, checkbox or rating cell
    // holding the keyboard after a Space press does not hold them off:
    // its edit already settled. Ctrl alone for Save: `consume_ctrl`
    // matches `Modifiers::CTRL` exactly, so a Ctrl+Shift+S press is not
    // Save over the open file.
    let save_key = !modal_up && ui.input_mut(|input| consume_ctrl(input, egui::Key::S));
    let save_as_key = !modal_up
        && ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)
        });
    let typing = ui.ctx().text_edit_focused() || modal_up;
    // Ctrl+Shift+Z's own two-modifier match; `consume_ctrl` covers the
    // Ctrl-only keys.
    let redo_key = !typing
        && ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z)
                || consume_ctrl(input, egui::Key::Y)
        });
    let undo_key = !typing && !redo_key && ui.input_mut(|input| consume_ctrl(input, egui::Key::Z));
    // Ctrl+F is Clear visual flags on the selected row's team (a player's
    // or a team row's own): an aesthetics write, so a no-op under the
    // lock, and a no-op while a text field holds the keyboard (`typing`).
    let clear_flags_key = !typing && ui.input_mut(|input| consume_ctrl(input, egui::Key::F));
    // Ctrl+I is the Player menu's FPC entry, the same lock as Ctrl+F;
    // Ctrl+B opens the Bump dialog, a gameplay write the lock does not
    // stop.
    let fpc_key = !typing && ui.input_mut(|input| consume_ctrl(input, egui::Key::I));
    let bump_key = !typing && ui.input_mut(|input| consume_ctrl(input, egui::Key::B));
    // Ctrl+O and a version's Ctrl+digit open as the File menu's opens do.
    // Not while a text field holds the keyboard: the menu's click ends the
    // field's edit first, and a key would not. Ctrl+W, the menu's Close
    // save, is held off the same way.
    let open_key = if typing {
        None
    } else {
        ui.input_mut(file_menu::consume_shortcut)
    };
    let close_key = !typing
        && view.session.is_some()
        && ui.input_mut(|input| consume_ctrl(input, egui::Key::W));
    // Ctrl+C and Ctrl+V copy and paste what `target` resolves (the
    // selected row, the shown preset, the shown formation). They arrive
    // as `Event::Copy`/`Event::Paste`, never `Event::Key`, held off
    // while a dialog is up or a text edit holds the keyboard — a field
    // keeps its own copy and paste; a focused button does not hold them.
    // Ctrl+C is a label's own too while one of its texts is selected:
    // the selection's copy is the text, not the save's target. A paste
    // carries the system clipboard's text, which is the copy;
    // `Event::PasteImage` means the clipboard holds an image, no copy.
    let copy_key = !typing
        && !ui
            .ctx()
            .plugin_or_default::<egui::text_selection::LabelSelectionState>()
            .lock()
            .has_selection()
        && ui.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Copy))
        });
    let pasted = if typing {
        None
    } else {
        ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Paste(text) => Some(text.clone()),
                _ => None,
            })
        })
    };
    // PageDown and PageUp move the list's selection from any field, so
    // they are not held off while a widget has the keyboard: team row to
    // team row while the list holds two or more, else seven rows at a
    // time. A held pointer button holds them off, and so does the
    // frame it is released on, where a click completes: the drag or press
    // in flight would finish on the next player.
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
    // Up and Down walk every row, the team rows too, so they wait for no
    // widget to hold the keyboard, where the page keys act from any
    // field: a focused number steps with them and a text field moves its
    // cursor, and an open popup (a menu's, a combo's) navigates itself
    // with them. The modal and gesture gates are the page keys'. Two
    // popups egui's registry never sees (the unsaved-changes list and the
    // tactics' slot popover, both the view's own state drawn through
    // `egui::Popup`) hold them off the same way.
    let arrow_step = if modal_up
        || gesture
        || view.session.is_none()
        || view.changes_list.is_some()
        || view.tactics.popover_area().is_some()
        || ui.ctx().memory(|memory| memory.focused()).is_some()
        || egui::Popup::is_any_open(ui.ctx())
    {
        None
    } else {
        ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                Some(true)
            } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                Some(false)
            } else {
                None
            }
        })
    };
    // A shortcut consumed over an open menu closes it, as an entry
    // click does: else it floats over the view or dialog the shortcut
    // raised. The two popups egui's registry never sees — the changes
    // list and the tactics' slot popover — are the view's own state,
    // closed with it.
    if save_key
        || save_as_key
        || undo_key
        || redo_key
        || clear_flags_key
        || fpc_key
        || bump_key
        || copy_key
        || close_key
        || open_key.is_some()
        || pasted.is_some()
        || page_step.is_some()
    {
        egui::Popup::close_all(ui.ctx());
        view.changes_list = None;
        view.tactics.close_popover();
    }
    toolbar_band(ui, band_end, view_rect, |ui| {
        let write = file_menu::show(ui, view);
        match write
            .or(save_key.then_some(file_menu::Write::Save))
            .or(save_as_key.then_some(file_menu::Write::SaveAs))
        {
            Some(file_menu::Write::Save) => {
                save(view, running);
            }
            Some(file_menu::Write::SaveAs) => save_as(view, running),
            None => {}
        }
        let edit = edit_menu(ui, view);
        if undo_key || edit == Some(EditAction::Undo) {
            following_selection(view, Session::undo);
        }
        if redo_key || edit == Some(EditAction::Redo) {
            following_selection(view, Session::redo);
        }
        if edit == Some(EditAction::Copy) {
            clipboard::copy(ui.ctx(), view);
        }
        if edit == Some(EditAction::Paste) {
            // The window hands over the clipboard's text as the
            // `Event::Paste` Ctrl+V produces, which `pasted` takes.
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }
        menus::player_menu(ui, view);
        menus::team_menu(ui, view);
        menus::savefile_menu(ui, view);
        toolbar_separator(ui);
        toolbar_framed(ui, |ui| {
            // The ruleset beside the check it drives, whether the check is
            // on or not: the quick actions and the card counts use it too.
            ruleset_picker::show(ui, view);
            // The Live AATF toggle as a framed button, since the toggle is
            // how a manager finds the check at all: the play sign (`▶`)
            // then the name (a check mark beside it read as on while it
            // was off), the selection fill while on; a selectable label
            // read as a label beside the menus.
            let response = ui
                .add(egui::Button::new("\u{25b6} Live AATF").selected(view.live_aatf))
                .on_hover_text("Check the open team after every edit (the setting is remembered)");
            if view.live_aatf {
                // The held toggle's edge is its fill's — the selection
                // blue's own (`edge_color`); off it keeps the resting edge.
                resting_edge(
                    ui,
                    &response,
                    studio_core::theme::edge_color(ui.visuals(), ui.visuals().selection.bg_fill),
                );
            }
            if response.clicked() {
                surrender_focus(ui);
                view.live_aatf = !view.live_aatf;
            }
            // What the check asks for, so only while it runs. In the framed
            // scope too: the bar's style strips the box's outline.
            if view.live_aatf
                && studio_core::widgets::checkbox(
                    ui,
                    &mut view.live_suggestions,
                    "Show suggestions",
                )
                .on_hover_text(
                    "Also list each player's unused allowances (cards, free tricks, weak foot...)",
                )
                .clicked()
            {
                surrender_focus(ui);
            }
        });
        // The save's path is in the window title (`title_detail`): beside the
        // unsaved mark it could wrap the toolbar onto a second row.
        changes_popup::show(ui, view);
    });
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
    if let Some(shortcut) = open_key {
        file_menu::open_shortcut(view, shortcut);
    }
    if close_key {
        file_menu::request(view, prompt::Pending::CloseSave);
    }
    if let Some(forward) = page_step
        && player_list::page_selection(view, forward)
    {
        // The field gives up the keyboard once the selection moved: its
        // gesture ends as one undo entry on the player it edited, and the
        // next player's field does not inherit the focus and merge his
        // edits into that entry. A key that moved nothing leaves it be.
        surrender_focus(ui);
        view.reveal_selected = Some(reveal_for(view.selected));
    }
    if let Some(forward) = arrow_step
        && player_list::step_selection(view, forward, 1)
    {
        view.reveal_selected = Some(Reveal::Nearest);
    }
    if copy_key {
        clipboard::copy(ui.ctx(), view);
    }
    if let Some(text) = pasted {
        clipboard::paste(view, &text);
    }
}

/// The room around the toolbar's row inside its band (the draft's `.top`
/// padding): 3 points over and under the row, 10 before it, so its first
/// menu stands over the player list's left edge (`PANES_LEFT_INSET`), and
/// 8 after it.
const BAND_MARGIN: egui::Margin = egui::Margin {
    left: 10,
    right: 8,
    top: 3,
    bottom: 3,
};

/// The toolbar's band (the draft's `.top`) holding the toolbar, `row`
/// drawn in `studio_core`'s menu bar: the panel's fill with a 1-point line
/// under it in the panel's stroke, from `inside`'s edges back over the
/// shell's margin, to `end` (`inside`'s own right edge plus the margin
/// with `None`), the row `BAND_MARGIN` inside it. The band is a child
/// placed by rect, so its width moves nothing under it; `ui` goes on
/// under its line.
fn toolbar_band(
    ui: &mut egui::Ui,
    end: Option<f32>,
    inside: egui::Rect,
    row: impl FnOnce(&mut egui::Ui),
) {
    let view = inside + egui::Frame::central_panel(ui.style()).inner_margin;
    let right = end.unwrap_or(view.right());
    let band = egui::Rect::from_min_max(view.min, egui::pos2(right, view.bottom()));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(band));
    let filled = egui::Frame::side_top_panel(ui.style())
        .inner_margin(BAND_MARGIN)
        .show(&mut child, |ui| studio_core::widgets::toolbar(ui, row))
        .response
        .rect;
    // Under the fill, as a top panel draws its separator line.
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let line_y = filled.bottom() + stroke.width / 2.0;
    ui.painter().hline(filled.x_range(), line_y, stroke);
    let under = egui::Rect::from_min_max(
        inside.min,
        egui::pos2(inside.left(), filled.bottom() + stroke.width),
    );
    ui.advance_cursor_after_rect(under);
}

/// The entry the Edit menu's frame clicked, `None` with none (one entry
/// is clicked per frame): the toolbar runs it as it does the keys'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditAction {
    /// Undo (Ctrl+Z).
    Undo,
    /// Redo (Ctrl+Y).
    Redo,
    /// What Ctrl+C does on the target (`clipboard::copy`).
    Copy,
    /// Asks egui for the system clipboard's text (`RequestPaste`), which
    /// the next frame hands over as Ctrl+V's `Event::Paste`.
    Paste,
}

/// The Edit menu: Undo and Redo with their shortcuts' text, each
/// disabled with nothing to undo or redo, then under a separator Copy
/// and Paste, which do what Ctrl+C and Ctrl+V do on the same target:
/// Copy dead when Ctrl+C would copy nothing, Paste dead only with no
/// target (the window cannot read the clipboard before it asks for it),
/// both dead with no save open, each enabled entry's hover naming the
/// copy or the target in the notices' words. The menu button itself is
/// always enabled. Returns the entry clicked this frame, `None` with
/// none: the toolbar runs it, for a click as for a key.
fn edit_menu(ui: &mut egui::Ui, view: &SaveView) -> Option<EditAction> {
    let (can_undo, can_redo) = view
        .session
        .as_ref()
        .map(|session| (session.can_undo(), session.can_redo()))
        .unwrap_or((false, false));
    let entry =
        |ui: &mut egui::Ui, enabled: bool, label: &str, shortcut: &str, tip: Option<&str>| {
            let button = egui::Button::new(label).shortcut_text(shortcut);
            let mut response = add_enabled(ui, enabled, button);
            if let Some(tip) = tip {
                response = response.on_hover_text(tip);
            }
            let clicked = response.clicked();
            if clicked {
                ui.close();
            }
            clicked
        };
    menu_button(ui, "Edit", |ui| {
        // Inside the menu's draw: the copy's answer builds an export, so
        // it is asked only while the menu is open.
        let tips = clipboard::tooltips(view);
        let (copy_tip, paste_tip) = tips.as_ref().map_or((None, None), |(copy, paste)| {
            (Some(copy.as_str()), Some(paste.as_str()))
        });
        let undo = entry(ui, can_undo, "Undo", "Ctrl+Z", None).then_some(EditAction::Undo);
        let redo = entry(ui, can_redo, "Redo", "Ctrl+Y", None).then_some(EditAction::Redo);
        menu_separator(ui);
        let copy = entry(ui, clipboard::can_copy(view), "Copy", "Ctrl+C", copy_tip)
            .then_some(EditAction::Copy);
        let paste = entry(
            ui,
            clipboard::has_target(view),
            "Paste",
            "Ctrl+V",
            paste_tip,
        )
        .then_some(EditAction::Paste);
        undo.or(redo).or(copy).or(paste)
    })
    .inner
    .flatten()
}

/// The window's Save (the File menu's, Ctrl+S's and the prompt's): `save_to`
/// the open file. Answers whether the file was written — `false` while the
/// question waits or a write failed, so a prompt stays up.
pub(super) fn save(view: &mut SaveView, running: &dyn Fn() -> Vec<RunningPes>) -> bool {
    let Some(path) = view.save_path().map(Path::to_path_buf) else {
        return false;
    };
    save_to(view, running, path)
}

/// The File menu's Save as: the picker, on the open file's folder and name,
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
/// `writeSave`); a written file goes to the top of the File menu's recent
/// files and gets its notice, the `.bak` named when the write replaced a
/// file. `edit_mode_closed` lets the write go ahead while the save's own
/// PES runs: the window's Save passes `false` (the own-game check ran
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
    let replaced = target.exists();
    match session.save_to(target, &running(), edit_mode_closed) {
        Ok(()) => {
            view.refusal = None;
            file_menu::push_recent(&mut view.recents, target, session.version());
            let file = interchange::file_name(target);
            view.notice = Some(Notice {
                text: if replaced {
                    format!("{file} saved (the replaced file is kept once as {file}.bak)")
                } else {
                    format!("{file} saved")
                },
                kind: NoticeKind::Success,
                action: None,
            });
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
/// that does not exist yet is no match. The refused write is a step —
/// `could not write <path>`, the path already the step's subject so the
/// reason alone follows it.
pub(super) fn refuse_save_being_read(
    destination: &Path,
    save: &Path,
    what: &str,
    recorder: &Recorder,
) -> anyhow::Result<()> {
    if let (Ok(destination_file), Ok(save_file)) = (
        std::fs::canonicalize(destination),
        std::fs::canonicalize(save),
    ) && destination_file == save_file
    {
        let reason =
            format!("that is the open save, and writing the {what} over it would destroy the save");
        journal::write_failed(destination, recorder, &reason);
        anyhow::bail!("{}: {reason}", destination.display());
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

/// `edit_player` for the team's entry (General info's Shirt number,
/// Main stats' Captain, the team view's fields, the Tactics editor's
/// edited clone).
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

/// `step` run on the open session, `None` with none: the selection
/// follows the record it named through the step, the list scrolling to
/// the slot it lands on only when the slot moves. The selection names a
/// roster slot, not a player, so a step that trades the players' records
/// between the ids (an undo or a redo of a move, a `Player order` row's X) would
/// leave it on whoever then holds the slot: the record of the selected
/// slot's player is read before `step`, and the slot in the same team
/// whose player holds it after (a move never crosses teams, so the
/// search stays in the one roster). A step that trades nothing finds the
/// record where it was; a selection that is not a player row, or a slot
/// holding no player, is left alone.
fn following_selection<R>(view: &mut SaveView, step: impl FnOnce(&mut Session) -> R) -> Option<R> {
    let session = view.session.as_mut()?;
    let followed = match view.selected {
        Some(Selection::Player(team_id, slot)) => session
            .slot_player(team_id, slot)
            .map(|id| (team_id, session.record_of(id))),
        _ => None,
    };
    let result = step(session);
    if let Some((team_id, record)) = followed
        && let Some(team) = session.file().team(team_id)
    {
        for slot in 0..team.roster.len() {
            let slot = u8::try_from(slot).expect("a roster is at most 40 slots");
            if session
                .slot_player(team_id, slot)
                .is_some_and(|id| session.record_of(id) == record)
            {
                let landed = Some(Selection::Player(team_id, slot));
                if view.selected != landed {
                    view.selected = landed;
                    view.reveal_selected = Some(Reveal::Nearest);
                }
                break;
            }
        }
    }
    Some(result)
}

/// The selected row's team id — a player row's team or a team row's own;
/// `None` with no row selected.
pub(crate) fn selected_team_id(view: &SaveView) -> Option<u32> {
    match view.selected? {
        Selection::Team(id) | Selection::Player(id, _) => Some(id),
    }
}

/// Whether the group is on: `group` holds two or more player rows and
/// `selected` names the first of them — the draft's `S.sel ===
/// S.group[0]`, a value compare here since `Selection` names a row,
/// not an object. The one test every reader goes through; it takes the
/// pair rather than a `SaveView` because the row functions that need
/// it hold the fields, not the view.
pub(crate) fn group_on(selected: Option<Selection>, group: &[(u32, u8)]) -> bool {
    group.len() >= 2
        && group
            .first()
            .is_some_and(|&(team_id, slot)| selected == Some(Selection::Player(team_id, slot)))
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
