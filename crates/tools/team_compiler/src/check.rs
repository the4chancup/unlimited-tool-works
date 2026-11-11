//! The `check` command: validation (the structure pass and the deep pass) over every export,
//! reported export by export, with nothing compiled.

use studio_core::{Disposition, Scope, Severity, ToolContext};

use crate::bins::installed;
use crate::cli::RunInputs;
use crate::events::RunEvents;
use crate::messages::{Code, tool_message};
use crate::reader::{self, ExportSource};
use crate::validation::{run_budget, run_pool, validation_pass};

/// The `check` command: validation, each export reported between its start and its end, a
/// texture link looked for in the installed CPKs `compile` would look in, the refs CPK passed
/// over when the exports hold a refs export, as `compile` passes it over. Returns the worst
/// severity reported; an exports folder that cannot be read is an error.
pub(crate) fn run(inputs: &RunInputs, ctx: &ToolContext) -> anyhow::Result<Option<Severity>> {
    let mut events = RunEvents::new(ctx, inputs.timestamps);
    let discovery = reader::discover(&inputs.exports_root, &inputs.exports)?;
    for path in &discovery.skipped {
        events.message(tool_message(
            Code::ExportUpgradeUnfinished,
            Scope::Run,
            Disposition::Keep,
            vec![("path", path.display().to_string())],
        ));
    }
    let sources = discovery.sources;
    let refs_name = sources
        .iter()
        .any(ExportSource::is_referees)
        .then_some(inputs.settings.refs_cpk_name.as_str());
    let installed = installed::installed_paths(
        &inputs.common.pes_folder(),
        inputs.settings.boundary_cpk_name(),
        refs_name,
        inputs.settings.multicpk_mode,
    );
    let pool = run_pool(inputs)?;
    // No tasks follow a check, so no archive is kept past its check.
    let pass = validation_pass(
        inputs,
        sources,
        &installed,
        &run_budget(inputs),
        &pool,
        false,
    )?;
    for message in pass.run_messages {
        events.message(message);
    }
    for checked in pass.sources {
        events.started(&checked.source);
        for message in checked.messages {
            events.message(message);
        }
        events.processed(checked.source.export_id);
    }
    Ok(events.worst())
}
