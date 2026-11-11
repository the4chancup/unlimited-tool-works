//! A run's event stream: what `check` and `compile` report to the shell or the console, export
//! by export, and the worst severity among it, which decides the command's exit code.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use studio_core::{
    Disposition, ExportId, Message, PipelineEvent, PipelineEventEnvelope, RunId, Scope, Severity,
    ToolContext,
};
use vtree::ScopePath;

use crate::messages::Code;
use crate::reader::ExportSource;

/// A command is one run per process, so every event carries the same run id.
const RUN_ID: RunId = RunId(1);

/// A run's event stream, and the worst severity among the messages it carried, which decides
/// the command's exit code.
pub(crate) struct RunEvents {
    ctx: ToolContext,
    /// The run's start when the run times its findings, so a `Message` can carry when it
    /// was made (`--timestamps`); `None` when it does not.
    started: Option<Instant>,
    worst: Option<Severity>,
    /// The exports a finding refused whole (`DropExport`), by their id, for `Complete`'s
    /// summary: one export counts once however many findings refused it.
    skipped_exports: BTreeSet<ExportId>,
    /// The folders a finding's verdict dropped (`DropFolder`), mapped as
    /// `folders_with_errors`' are: one folder counts once however many findings it has.
    skipped_folders: BTreeSet<(ExportId, ScopePath)>,
    /// The entries an `overrides/` file replaced (`duplicate_path`), for `Complete`'s summary.
    duplicates: usize,
    /// The folders that reported an error-level finding, by their canonical path inside their
    /// export: one folder counts once however many findings it has.
    folders_with_errors: BTreeSet<(ExportId, ScopePath)>,
}

impl RunEvents {
    /// Events sent through `ctx`, none yet. `timestamps` starts the clock each finding's
    /// `made_at` is measured from.
    pub(crate) fn new(ctx: &ToolContext, timestamps: bool) -> RunEvents {
        // A clone shares the context's channels, so the events reach the same receivers.
        RunEvents {
            ctx: ctx.clone(),
            started: timestamps.then(Instant::now),
            worst: None,
            skipped_exports: BTreeSet::new(),
            skipped_folders: BTreeSet::new(),
            duplicates: 0,
            folders_with_errors: BTreeSet::new(),
        }
    }

    /// `source` entered the pipeline.
    pub(crate) fn started(&self, source: &ExportSource) {
        // The console names the export by its file name, extension included, so a folder and
        // an archive with one stem stay apart in every line (TC-SRC-02).
        self.emit(
            Some(source.export_id),
            PipelineEvent::ExportStarted {
                export_id: source.export_id,
                display_name: source.file_name.clone(),
            },
        );
    }

    /// One finding, addressed to the export its scope names, made when it is reported.
    pub(crate) fn message(&mut self, message: Message) {
        self.message_made(message, Instant::now());
    }

    /// One finding made at `made`: a task's findings were made when the task finished on
    /// its worker, however long the batch then waited to be reported.
    pub(crate) fn message_made(&mut self, message: Message, made: Instant) {
        self.worst = self.worst.max(Some(message.severity));
        if message.disposition == Disposition::DropExport
            && let Some(export_id) = message.scope.export_id()
        {
            self.skipped_exports.insert(export_id);
        }
        if message.disposition == Disposition::DropFolder
            && let Some(folder) = scope_folder(&message.scope)
        {
            self.skipped_folders.insert(folder);
        }
        if message.code.code.as_ref() == Code::DuplicatePath.as_str() {
            self.duplicates += 1;
        }
        // A file's error is its folder's too; a roster line's or an export's counts no folder.
        if message.severity >= Severity::Error
            && let Some(folder) = scope_folder(&message.scope)
        {
            self.folders_with_errors.insert(folder);
        }
        let made_at = self
            .started
            .map(|start| made.saturating_duration_since(start));
        self.emit_at(
            message.scope.export_id(),
            PipelineEvent::Message(message),
            made_at,
        );
    }

    /// The export left the pipeline, whatever its outcome.
    pub(crate) fn processed(&self, export_id: ExportId) {
        self.emit(
            Some(export_id),
            PipelineEvent::ExportProcessed { export_id },
        );
    }

    /// `processed` of the manifest's `total` tasks committed (`Progress`, the console's parts
    /// counter). Run-wide: a task is the run's work, no export's.
    pub(crate) fn progress(&self, processed: usize, total: usize) {
        self.emit(None, PipelineEvent::Progress { processed, total });
    }

    /// The run ended after compiling (`Complete`): `files_written` entries put in place (the
    /// output folder, or the game's `download/` when the run deploys), the counts the findings
    /// kept, and `success` when nothing error-level was reported.
    pub(crate) fn complete(&self, files_written: usize) {
        self.emit(
            None,
            PipelineEvent::Complete {
                success: self.worst.is_none_or(|worst| worst < Severity::Error),
                files_written,
                skipped_exports: self.skipped_exports.len(),
                skipped_folders: self.skipped_folders.len(),
                duplicates: self.duplicates,
                folders_with_errors: self.folders_with_errors.len(),
            },
        );
    }

    /// The worst severity reported so far; `None` when no message was.
    pub(crate) fn worst(&self) -> Option<Severity> {
        self.worst
    }

    fn emit(&self, export_id: Option<ExportId>, event: PipelineEvent) {
        self.emit_at(export_id, event, None);
    }

    fn emit_at(
        &self,
        export_id: Option<ExportId>,
        event: PipelineEvent,
        made_at: Option<Duration>,
    ) {
        self.ctx.emit(PipelineEventEnvelope {
            run_id: RUN_ID,
            export_id,
            export_revision: None,
            made_at,
            event,
        });
    }
}

/// The folder `scope` names, as a `(export, path)` count key: its own for a folder, its
/// file's parent for a file; `None` for a scope that names no folder (the run, a whole
/// export, a roster line, whose finding is often about a malformed line or a missing
/// target rather than the folder holding the file).
fn scope_folder(scope: &Scope) -> Option<(ExportId, ScopePath)> {
    match scope {
        Scope::Folder { export_id, path } => Some((*export_id, path.clone())),
        Scope::File { export_id, path } => path.parent().map(|parent| (*export_id, parent)),
        Scope::Run | Scope::Export { .. } | Scope::RosterEntry { .. } => None,
    }
}
