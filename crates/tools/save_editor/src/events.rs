//! A command's event stream: the findings it reports to the console (one line each) or the
//! shell, and the worst severity among them, which decides the exit code.

use studio_core::{Message, PipelineEvent, PipelineEventEnvelope, RunId, Severity, ToolContext};

/// A command is one run per process, so every event carries the same run id.
const RUN_ID: RunId = RunId(1);

/// A command's findings, sent as they are made, and the worst severity among them.
pub(crate) struct Report {
    ctx: ToolContext,
    worst: Option<Severity>,
}

impl Report {
    /// Findings sent through `ctx`, none yet.
    pub(crate) fn new(ctx: &ToolContext) -> Report {
        // A clone shares the context's channels, so the events reach the same receivers.
        Report {
            ctx: ctx.clone(),
            worst: None,
        }
    }

    /// One finding. A command's findings are all run-wide: there is no export to address.
    pub(crate) fn message(&mut self, message: Message) {
        self.worst = self.worst.max(Some(message.severity));
        self.emit(PipelineEvent::Message(message));
    }

    /// The command's result text (a JSON report): the console prints it as it is. Not a
    /// finding, so it does not touch the exit code's severity.
    pub(crate) fn output(&mut self, text: String) {
        self.emit(PipelineEvent::Output(text));
    }

    /// The worst severity reported so far; `None` when no finding was.
    pub(crate) fn worst(&self) -> Option<Severity> {
        self.worst
    }

    fn emit(&self, event: PipelineEvent) {
        self.ctx.emit(PipelineEventEnvelope {
            run_id: RUN_ID,
            export_id: None,
            export_revision: None,
            made_at: None,
            event,
        });
    }
}
