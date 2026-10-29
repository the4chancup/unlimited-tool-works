//! `aesthetics_patch.toml`, written into the output folder once the run's outputs are in
//! place: every compiled player's resolved motions, and a `write_names` export's names
//! (`spec/pes_savefile/operations.md` "Aesthetics patch"). A run whose plan holds no patch —
//! no compiled player, or a pre-Fox target — moves an earlier file of the name aside to
//! `.bak`, as `teamnotes` does, so a left-over patch does not speak for a run that wrote
//! none.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use pes_savefile::interchange::aesthetics_patch::{
    ALLOCATION_SCHEME_VERSION, AestheticsPatch, PatchTeam, utc_datetime,
};
use pes_version::PesVersion;
use pipeline::CpkStem;

use crate::output::deploy;
use crate::paths::replace_file;

/// The patch's name in the output folder.
pub(crate) const FILE_NAME: &str = "aesthetics_patch.toml";

/// The name an earlier patch is moved aside to, replacing an older one.
const BAK_NAME: &str = "aesthetics_patch.toml.bak";

/// Writes `<output_folder>/aesthetics_patch.toml` for `teams` through a temporary file
/// renamed into place, and returns the patch it wrote: its `teams` give `patch_written`'s
/// ids, and the savefile step applies it (`output/savefile.rs`). `teams` empty
/// is no patch: an earlier file of the name goes to `.bak` (a directory of the name is not
/// the patch and is left) and `None` is returned. `cpks` is the staged CPKs the run publishes,
/// empty in test and sideload mode, which publish loose trees.
pub(crate) fn write(
    output_folder: &Path,
    version: PesVersion,
    teams: &[PatchTeam],
    cpks: &[CpkStem],
) -> anyhow::Result<Option<AestheticsPatch>> {
    let path = output_folder.join(FILE_NAME);
    if teams.is_empty() {
        if path.is_file() {
            // The rename replaces an older backup, on Windows too (std renames with
            // `MOVEFILE_REPLACE_EXISTING`).
            fs::rename(&path, output_folder.join(BAK_NAME))?;
        }
        return Ok(None);
    }
    // A clock set before 1970 stamps the epoch, as `deploy::run_id` handles it.
    let unix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_secs(),
        Err(_) => 0,
    };
    let patch = AestheticsPatch {
        pes_version: version,
        tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
        cpks: cpks.iter().map(deploy::cpk_file_name).collect(),
        compiled: utc_datetime(unix),
        teams: teams.to_vec(),
    };
    replace_file(&path, patch.to_toml()?.as_bytes())?;
    Ok(Some(patch))
}
