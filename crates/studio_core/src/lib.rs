//! The platform: what runs with zero tools installed.
//!
//! `studio_core` holds the tool plugin trait, the settings framework, the pipeline event types
//! every tool emits, the status bar's typed items, the help chapter types and the widgets the
//! tools' views share. The module tree is closed: format knowledge belongs in a lib crate, and a
//! widget used by one tool in that tool's `view/`. This file only re-exports.

pub mod events;
pub mod help;
pub mod icons;
pub mod pes_process;
pub mod settings;
pub mod shell;
pub mod status;
pub mod theme;
pub mod tool;
pub mod widgets;
pub use events::{
    Disposition, EventLines, ExportId, ExportRevision, FolderStatus, Message, MessageCode,
    PipelineEvent, PipelineEventEnvelope, RunId, Scope, Severity,
};
pub use help::{CatalogRow, HelpSection, HelpTarget, HelpTopic, run_help};
pub use pes_process::{RunningPes, running_pes};
pub use settings::{COMMON_KEY, CommonSettings, Settings, SettingsError, Theme};
#[cfg(not(target_arch = "wasm32"))]
pub use settings::{SETTINGS_FILE_NAME, resolve_data_dir, settings_file_writable, user_config_dir};
pub(crate) use shell::APP_NAME;
#[cfg(not(target_arch = "wasm32"))]
pub use shell::run_gui;
pub use shell::{
    EXE_NAME, Launch, LaunchMode, StudioApp, TOOL_VERSION, parse_launch, run_cli, version_label,
};
pub use status::{ActionEffect, Notice, NoticeAction, NoticeKind, ShellCondition, ToolActivity};
pub use tool::{AppPaths, CliError, PlannedTool, ShellRequest, StudioTool, ToolContext};
pub use widgets::{BottomDock, LogLine, LogPanel};
