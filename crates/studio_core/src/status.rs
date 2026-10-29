//! The status bar's typed items. Three slots, one item kind each; tools
//! cannot draw into the bar, they return a `ToolActivity` and publish `Notice`s. Conditions are
//! the shell's alone. There is no free-text status API, on purpose.

use std::path::PathBuf;
use std::time::Duration;

use pes_version::PesVersion;

use crate::help::HelpTarget;

/// A persistent environment state the shell computes itself (left slot). Shown while it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCondition {
    /// Settings and the teams list cannot be written; the session keeps working in memory.
    DataDirReadOnly,
    /// A newer release exists.
    UpdateAvailable {
        /// The version available.
        version: String,
    },
}

impl ShellCondition {
    /// The icon font's glyph shown beside the text (the draft's `CONDITION_DEFS`).
    pub(crate) fn icon(&self) -> &'static str {
        match self {
            ShellCondition::DataDirReadOnly => crate::icons::WARNING,
            ShellCondition::UpdateAvailable { .. } => crate::icons::DOWNLOAD_SIMPLE,
        }
    }

    /// The bar's one-line text, and the line the CLI prints for the condition.
    pub fn text(&self) -> String {
        match self {
            ShellCondition::DataDirReadOnly => "Data folder is read-only".to_owned(),
            ShellCondition::UpdateAvailable { version } => {
                format!("Update available: v{version}")
            }
        }
    }

    /// The bar's tooltip while the condition holds.
    pub(crate) fn tooltip(&self) -> String {
        match self {
            ShellCondition::DataDirReadOnly => {
                "Settings and the teams list cannot be saved, so changes last for this session \
                 only"
                    .to_owned()
            }
            ShellCondition::UpdateAvailable { .. } => "A newer release is out".to_owned(),
        }
    }
}

/// What a tool is doing in the background (right slot), shown while a different tool's view is
/// on screen, and in the window title for every tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolActivity {
    /// The tool's label (`Team compiler`).
    pub tool_label: &'static str,
    /// `(done, total)` when the work has a countable size.
    pub progress: Option<(usize, usize)>,
    /// Time since the work started.
    pub elapsed: Duration,
}

impl ToolActivity {
    /// `Team compiler · 12/48 · 01:12`, or `Team compiler · 01:12` with no progress; the
    /// elapsed minutes are not capped at 59 (the draft's `activityText`). The status bar's
    /// right slot and the window title share this line.
    pub(crate) fn text(&self) -> String {
        let elapsed = self.elapsed.as_secs();
        let clock = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);
        match self.progress {
            Some((done, total)) => format!("{} · {done}/{total} · {clock}", self.tool_label),
            None => format!("{} · {clock}", self.tool_label),
        }
    }
}

/// A one-off event (middle slot): text plus at most one action. Only the most recent is shown;
/// an event worth keeping is also a finding in a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// What happened (`Compiled 48 teams`).
    pub text: String,
    /// The notice's kind, set by its publisher: it picks the bright colour the
    /// text is drawn in (`theme::notice_color`), so every publisher says one.
    pub kind: NoticeKind,
    /// The one thing the user can do about it.
    pub action: Option<NoticeAction>,
}

/// How a notice's text is coloured: a notice is seen, not found.
/// There is no `Default`: the publisher knows which reading fits its event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    /// Something asked for is done: a file written (`Wrote settings.toml`), a compile
    /// finished (`Compiled 48 teams`). Drawn in the palette's `success`, the draft's green.
    Success,
    /// A change of state to know about, neither finished nor broken: `Switched to PES
    /// 2021`, `Updated to v1.4.0`. Drawn in the theme's link blue.
    Info,
    /// Something the member should look at: `Teams list is read-only`, a switch whose
    /// `Fix PES path` action points at another folder's exe. Drawn in the theme's
    /// warning orange.
    Warning,
}

/// The clickable part of a notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeAction {
    /// Button label (`Open output`).
    pub label: String,
    /// What clicking does.
    pub effect: ActionEffect,
}

/// The closed set of things a notice action can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionEffect {
    /// Open a folder in the system file browser.
    OpenFolder(PathBuf),
    /// Open the settings menu.
    OpenSettings,
    /// Open the help window at a target.
    OpenHelp(HelpTarget),
    /// Make another tool the active view.
    SwitchTool(String),
    /// Switch the PES version, as a manual pick in the sidebar's selector does: the setting
    /// written, the running-PES auto-switch spent, the settings saved. Disabled while a tool
    /// reports a version-change blocker, its hover the selector's reason.
    SwitchVersion(PesVersion),
    /// Point the PES folder at the folder a running PES exe lives in, the main row or the
    /// version's own entry once any exists (the shell's own action — no tool has a reason
    /// to publish it): the sidebar's auto-switch notice offers it when the running game
    /// sits outside the configured folder.
    FixPesPath {
        /// The running exe's folder.
        folder: PathBuf,
        /// The version the notice was raised for — the running game's. A pick can change the
        /// selection while the notice is up, so the click must not read it back.
        version: PesVersion,
    },
}
