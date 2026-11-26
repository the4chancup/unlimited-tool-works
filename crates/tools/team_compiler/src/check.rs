//! The `check` command: validation (the structure pass and the deep pass) over every export,
//! reported export by export, with nothing compiled. The window's grid runs the same
//! validation (`live`).

pub(crate) mod live;

use studio_core::{Disposition, Scope, Severity, ToolContext};

use crate::bins::installed;
use crate::cli::RunInputs;
use crate::events::RunEvents;
use crate::messages::{Code, tool_message};
use crate::reader::{self, ExportSource};
use crate::validation::{ValidationPass, run_budget, run_pool, validation_pass};

/// The `check` command: validation, each export reported between its start and its end.
/// Textures for links and installed portraits are looked up in the installed CPKs `compile`
/// would use, with the refs CPK passed over when the exports hold a refs export, as `compile`
/// passes it over. Returns the worst severity reported; an exports folder that cannot be read
/// is an error.
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
    let pass = validate(inputs, discovery.sources)?;
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

/// `check`'s validation of `sources`: textures for links and installed portraits looked up in
/// the installed CPKs `compile` would use, the refs CPK passed over when the sources hold a
/// refs export.
pub(crate) fn validate(
    inputs: &RunInputs,
    sources: Vec<ExportSource>,
) -> anyhow::Result<ValidationPass> {
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
    validation_pass(
        inputs,
        sources,
        &installed,
        &run_budget(inputs),
        &pool,
        false,
    )
}
