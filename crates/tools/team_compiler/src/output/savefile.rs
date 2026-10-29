//! The savefile update (`spec/team_compiler/pipeline.md` "6. Post-processing"): a run whose
//! output the game now loads applies the `aesthetics_patch.toml` it
//! just wrote to the local savefile through `AestheticsPatch::apply`: the compiler has no
//! second savefile write path, so the local result and the savefile builder's later
//! `apply-patch` are the same bytes by construction. The save is skipped while the compile
//! target's PES runs, unless the run was given `--edit-mode-closed`: the game writes the
//! save it holds over the file when the member leaves Edit mode after changing anything
//! there, so a write made with it closed stands. Every outcome is a finding.

use std::path::{Path, PathBuf};

use pes_savefile::EditFile;
use pes_savefile::discovery::SavefileCandidate;
use pes_savefile::interchange::aesthetics_patch::{AestheticsPatch, PatchSkip};
use pes_savefile::model::player::STRIPPED_APPEARANCE_ID;
use pes_version::PesVersion;
use studio_core::{Disposition, RunningPes, Scope};

use crate::events::RunEvents;
use crate::messages::{Code, tool_message};
use crate::settings::SavefilePath;

/// The save editor's command that applies a patch to a save, which `savefile_missing` names
/// for the user who applies it by hand.
pub(crate) const APPLY_COMMAND: &str =
    concat!(studio_core::exe_name!(), " save-editor apply-patch");

/// The `compile` flag by which the member says the game's Edit mode is closed, which
/// `savefile_skipped_pes_running` names for the member to compile again with.
pub(crate) const EDIT_MODE_CLOSED_FLAG: &str = "--edit-mode-closed";

/// Where the step reads and writes the save.
#[derive(Debug, PartialEq)]
pub(crate) enum Located {
    /// The configured path, which names something or could not be checked: a file, or what
    /// the load then refuses (a folder, a file the user cannot read) as `savefile_invalid`.
    Configured(PathBuf),
    /// `auto`'s newest candidate, and how many candidates there were.
    Detected { path: PathBuf, candidates: usize },
    /// Nothing there: the configured path, `None` for `auto`.
    Missing(Option<PathBuf>),
}

/// The save `setting` resolves to: `discover` (the Documents\KONAMI walk, whose list is
/// newest first) is called for `Auto` alone, a configured path only checked to exist: a
/// folder named by mistake is a save that cannot be read, an error, not a missing save. A
/// path the check itself fails on (a folder the user cannot read) counts as existing too,
/// so the load reports the error as `savefile_invalid` rather than the save reading as
/// missing.
pub(crate) fn locate(
    setting: &SavefilePath,
    discover: impl FnOnce() -> Vec<SavefileCandidate>,
) -> Located {
    match setting {
        SavefilePath::Path(path) => match path.try_exists() {
            Ok(true) | Err(_) => Located::Configured(path.clone()),
            Ok(false) => Located::Missing(Some(path.clone())),
        },
        SavefilePath::Auto => {
            let candidates = discover();
            match candidates.first() {
                Some(first) => Located::Detected {
                    path: first.path.clone(),
                    candidates: candidates.len(),
                },
                None => Located::Missing(None),
            }
        }
    }
}

/// Only the target version's game counts, as in the Save editor's writing commands
/// (`spec/save_editor.md` "Writing a save"): another version's never writes this save back.
fn target_running(running: &[RunningPes], version: PesVersion) -> bool {
    running.iter().any(|game| game.version == version)
}

/// The savefile failure that ends the deployment transaction: `update` reports every other
/// outcome itself and returns this one for the caller to emit, `AbortRun` after a
/// deployment it rolls back, an Error in sideload mode.
#[derive(Debug)]
pub(crate) struct SavefileFailure {
    /// The finding to emit: `SavefileInvalid` or `SavefileWriteFailed`.
    pub(crate) code: Code,
    /// Its context entries (`path`, then `error` or `version`).
    pub(crate) context: Vec<(&'static str, String)>,
}

/// Applies `patch` (the `aesthetics_patch.toml` at `patch_path` the run just wrote) to the
/// save `located` names, reporting every outcome as a `Keep` finding but the two the caller
/// emits: no save is the warning `savefile_missing`, a save skipped while the target's game
/// runs unless `edit_mode_closed`, a patch team or player the save lacks a per-item error
/// with the rest applied (a save holding none of the patch's players is not rewritten), a
/// compiled player whose appearance id the save still sets a warning, and the save written,
/// its previous file kept as `.bak`, the info `savefile_written`. An unreadable or
/// wrong-version save is `Err`(`savefile_invalid`) and a failed `apply` or write
/// `Err`(`savefile_write_failed`), neither emitted here.
pub(crate) fn update(
    events: &mut RunEvents,
    located: Located,
    running: &[RunningPes],
    edit_mode_closed: bool,
    patch_path: &Path,
    patch: &AestheticsPatch,
) -> Result<(), SavefileFailure> {
    let report = |events: &mut RunEvents, code: Code, context: Vec<(&'static str, String)>| {
        events.message(tool_message(code, Scope::Run, Disposition::Keep, context));
    };
    let path = match &located {
        Located::Missing(configured) => {
            let mut context = vec![("patch", patch_path.display().to_string())];
            if let Some(path) = configured {
                context.push(("path", path.display().to_string()));
            }
            context.push(("command", APPLY_COMMAND.to_owned()));
            report(events, Code::SavefileMissing, context);
            return Ok(());
        }
        Located::Configured(path) => path.clone(),
        Located::Detected { path, candidates } => {
            report(
                events,
                Code::SavefileAutodetected,
                vec![
                    ("path", path.display().to_string()),
                    ("candidates", candidates.to_string()),
                ],
            );
            path.clone()
        }
    };
    if !edit_mode_closed && target_running(running, patch.pes_version) {
        report(
            events,
            Code::SavefileSkippedPesRunning,
            vec![
                ("path", path.display().to_string()),
                ("version", patch.pes_version.to_string()),
                ("flag", EDIT_MODE_CLOSED_FLAG.to_owned()),
            ],
        );
        return Ok(());
    }
    let mut file = match EditFile::load(&path) {
        Ok(file) => file,
        Err(error) => {
            return Err(SavefileFailure {
                code: Code::SavefileInvalid,
                context: vec![
                    ("path", path.display().to_string()),
                    ("error", format!("{error:#}")),
                ],
            });
        }
    };
    if file.version() != patch.pes_version {
        return Err(SavefileFailure {
            code: Code::SavefileInvalid,
            context: vec![
                ("path", path.display().to_string()),
                ("version", file.version().to_string()),
            ],
        });
    }
    let summary = match patch.apply(&mut file) {
        Ok(summary) => summary,
        Err(error) => {
            return Err(SavefileFailure {
                code: Code::SavefileWriteFailed,
                context: vec![
                    ("path", path.display().to_string()),
                    ("error", format!("{error:#}")),
                ],
            });
        }
    };
    for skip in &summary.skipped {
        match skip {
            PatchSkip::TeamNotInSave { team } => report(
                events,
                Code::SavefileTeamMissing,
                vec![("team", team.to_string())],
            ),
            PatchSkip::PlayerNotInSave { player } => report(
                events,
                Code::SavefilePlayerMissing,
                vec![("player", player.to_string())],
            ),
        }
    }
    let mut not_stripped: Vec<u32> = summary
        .applied_players
        .iter()
        .copied()
        .filter(|id| {
            file.player(*id)
                .is_some_and(|player| player.appearance_id != Some(STRIPPED_APPEARANCE_ID))
        })
        .collect();
    not_stripped.sort_unstable();
    if !not_stripped.is_empty() {
        report(
            events,
            Code::SavefilePlayerNotStripped,
            vec![(
                "players",
                not_stripped
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            )],
        );
    }
    // A save holding none of the patch's players is left as it was: writing it would change
    // nothing but turn its `.bak` over, losing the user's previous backup.
    if summary.applied_players.is_empty() {
        return Ok(());
    }
    // The teams a player was written for: a team the save holds whose every patch player
    // it lacks is not one (cup player ids are team id x 100 + slot).
    let written_teams: Vec<String> = summary
        .applied_teams
        .iter()
        .filter(|team| {
            summary
                .applied_players
                .iter()
                .any(|player| player / 100 == **team)
        })
        .map(u32::to_string)
        .collect();
    match file.save(&path) {
        Ok(()) => report(
            events,
            Code::SavefileWritten,
            vec![
                ("path", path.display().to_string()),
                ("teams", written_teams.join(", ")),
            ],
        ),
        Err(error) => {
            return Err(SavefileFailure {
                code: Code::SavefileWriteFailed,
                context: vec![
                    ("path", path.display().to_string()),
                    ("error", format!("{error:#}")),
                ],
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    use crossbeam_channel::{Receiver, unbounded};
    use pes_savefile::interchange::aesthetics_patch::{
        ALLOCATION_SCHEME_VERSION, PatchPlayer, PatchTeam, utc_datetime,
    };
    use pes_savefile::settings_toml::MotionSettings;
    use studio_core::{AppPaths, PipelineEvent, PipelineEventEnvelope, Settings, ToolContext};

    use super::*;
    use crate::testing::scratch;

    /// A discovery candidate at `path`.
    fn candidate(path: PathBuf) -> SavefileCandidate {
        SavefileCandidate {
            path,
            account: None,
            modified: SystemTime::UNIX_EPOCH,
        }
    }

    /// A Fox patch holding no teams, for a call that must not reach the save's contents.
    fn empty_patch() -> AestheticsPatch {
        AestheticsPatch {
            pes_version: PesVersion::Pes21,
            tool_version: "test".to_owned(),
            allocation_scheme_version: ALLOCATION_SCHEME_VERSION,
            cpks: Vec::new(),
            compiled: utc_datetime(0),
            teams: Vec::new(),
        }
    }

    /// Run events over a context whose channel `rx` reads the emitted messages.
    fn events() -> (RunEvents, Receiver<PipelineEventEnvelope>) {
        let settings = Settings::parse("").unwrap();
        let (events_tx, events_rx) = unbounded();
        let (requests_tx, _requests_rx) = unbounded();
        let ctx = ToolContext::new(
            Arc::new(Mutex::new(settings)),
            AppPaths {
                exe_dir: PathBuf::new(),
                data_dir: None,
            },
            events_tx,
            requests_tx,
        );
        (RunEvents::new(&ctx), events_rx)
    }

    /// The code and context pairs `rx` collected.
    fn messages(rx: &Receiver<PipelineEventEnvelope>) -> Vec<(String, Vec<(String, String)>)> {
        rx.try_iter()
            .filter_map(|envelope| match envelope.event {
                PipelineEvent::Message(message) => {
                    Some((message.code.code.to_string(), message.context))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_configured_path_is_configured_or_missing_and_never_discovers() {
        let temp = scratch("savefile_locate_configured");
        let save = temp.path().join("EDIT00000000");
        fs::write(&save, b"a save").unwrap();
        let never = || panic!("a configured path runs no discovery");
        assert_eq!(
            locate(&SavefilePath::Path(save.clone()), never),
            Located::Configured(save.clone())
        );
        let missing = temp.path().join("MISSING");
        assert_eq!(
            locate(&SavefilePath::Path(missing.clone()), never),
            Located::Missing(Some(missing))
        );
        // A folder named by mistake is there: its load fails as `savefile_invalid`.
        assert_eq!(
            locate(&SavefilePath::Path(temp.path().to_owned()), never),
            Located::Configured(temp.path().to_owned())
        );
    }

    #[test]
    fn auto_takes_the_first_candidate_and_reports_the_count() {
        let first = PathBuf::from("newest");
        let candidates = move || {
            vec![
                candidate(PathBuf::from("newest")),
                candidate(PathBuf::from("older")),
            ]
        };
        assert_eq!(
            locate(&SavefilePath::Auto, candidates),
            Located::Detected {
                path: first,
                candidates: 2
            }
        );
        assert_eq!(
            locate(&SavefilePath::Auto, Vec::new),
            Located::Missing(None)
        );
    }

    #[test]
    fn target_running_counts_only_the_target_version() {
        let running = |version| vec![RunningPes { version, exe: None }];
        assert!(target_running(
            &running(PesVersion::Pes21),
            PesVersion::Pes21
        ));
        assert!(!target_running(
            &running(PesVersion::Pes19),
            PesVersion::Pes21
        ));
        assert!(!target_running(&[], PesVersion::Pes21));
    }

    #[test]
    fn update_skips_the_save_while_the_target_runs() {
        let temp = scratch("savefile_pes_running");
        let save = temp.path().join("EDIT00000000");
        fs::write(&save, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let before = fs::read(&save).unwrap();
        let running = [RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }];
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &running,
            false,
            &temp.path().join("aesthetics_patch.toml"),
            &empty_patch(),
        )
        .unwrap();

        let messages = messages(&rx);
        let [(code, context)] = messages.as_slice() else {
            panic!("one message: {messages:?}")
        };
        assert_eq!(code, "savefile_skipped_pes_running");
        assert_eq!(
            context.as_slice(),
            [
                ("path".to_owned(), save.display().to_string()),
                ("version".to_owned(), "PES 2021".to_owned()),
                ("flag".to_owned(), "--edit-mode-closed".to_owned())
            ]
        );
        assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
    }

    #[test]
    fn update_writes_the_save_while_the_target_runs_given_edit_mode_closed() {
        let temp = scratch("savefile_edit_mode_closed");
        let save = temp.path().join("EDIT00000000");
        fs::write(&save, pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let before = fs::read(&save).unwrap();
        let running = [RunningPes {
            version: PesVersion::Pes21,
            exe: None,
        }];
        let mut patch = empty_patch();
        patch.teams.push(PatchTeam {
            id: 714,
            name: "/co/".to_owned(),
            players: BTreeMap::from([(
                5,
                PatchPlayer {
                    name: None,
                    shirt_name: None,
                    motion: MotionSettings::default().resolved(PesVersion::Pes21),
                },
            )]),
        });
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &running,
            true,
            &temp.path().join("aesthetics_patch.toml"),
            &patch,
        )
        .unwrap();

        let messages = messages(&rx);
        let codes: Vec<&str> = messages.iter().map(|(code, _)| code.as_str()).collect();
        // The fixture is not stripped: 71405 is reported, then the save is written.
        assert_eq!(codes, ["savefile_player_not_stripped", "savefile_written"]);
        assert_ne!(fs::read(&save).unwrap(), before, "the save is written");
    }

    #[test]
    fn update_writes_nothing_when_the_save_lacks_every_team_of_the_patch() {
        let temp = scratch("savefile_no_team_applied");
        let save = temp.path().join("EDIT00000000");
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        let free = (1..).find(|id| file.team(*id).is_none()).unwrap();
        file.team_mut(714).unwrap().id = free;
        fs::write(&save, file.to_bytes(&[7; 320]).unwrap()).unwrap();
        let before = fs::read(&save).unwrap();
        let mut patch = empty_patch();
        patch.teams.push(PatchTeam {
            id: 714,
            name: "/co/".to_owned(),
            players: BTreeMap::new(),
        });
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &[],
            false,
            &temp.path().join("aesthetics_patch.toml"),
            &patch,
        )
        .unwrap();

        let codes: Vec<String> = messages(&rx).into_iter().map(|(code, _)| code).collect();
        assert_eq!(codes, ["savefile_team_missing"]);
        assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
        assert!(
            !temp.path().join("EDIT00000000.bak").exists(),
            "no .bak turned over"
        );
    }

    #[test]
    fn savefile_written_names_only_the_teams_a_player_was_written_for() {
        let temp = scratch("savefile_written_teams");
        let save = temp.path().join("EDIT00000000");
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        // 71405's record renumbered to an id the save lacks: team 714 stays, he goes.
        let free = (1..).find(|id| file.player(*id).is_none()).unwrap();
        file.player_mut(71405).unwrap().id = free;
        fs::write(&save, file.to_bytes(&[7; 320]).unwrap()).unwrap();
        let slot_05 = || {
            BTreeMap::from([(
                5,
                PatchPlayer {
                    name: None,
                    shirt_name: None,
                    motion: MotionSettings::default().resolved(PesVersion::Pes21),
                },
            )])
        };
        let mut patch = empty_patch();
        for (id, name) in [(714, "/co/"), (792, "/egg/")] {
            patch.teams.push(PatchTeam {
                id,
                name: name.to_owned(),
                players: slot_05(),
            });
        }
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &[],
            false,
            &temp.path().join("aesthetics_patch.toml"),
            &patch,
        )
        .unwrap();

        let messages = messages(&rx);
        let codes: Vec<&str> = messages.iter().map(|(code, _)| code.as_str()).collect();
        // The fixture is not stripped: 79205, the one player written, is reported.
        assert_eq!(
            codes,
            [
                "savefile_player_missing",
                "savefile_player_not_stripped",
                "savefile_written"
            ]
        );
        assert_eq!(messages[1].1, [("players".to_owned(), "79205".to_owned())]);
        assert_eq!(
            messages[2].1,
            [
                ("path".to_owned(), save.display().to_string()),
                ("teams".to_owned(), "792".to_owned())
            ],
            "714's only patch player was skipped"
        );
    }

    #[test]
    fn savefile_player_not_stripped_lists_the_players_sorted_whatever_the_team_order() {
        let temp = scratch("savefile_not_stripped_sorted");
        let save = temp.path().join("EDIT00000000");
        let file = EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        fs::write(&save, file.to_bytes(&[7; 320]).unwrap()).unwrap();
        let slot_05 = || {
            BTreeMap::from([(
                5,
                PatchPlayer {
                    name: None,
                    shirt_name: None,
                    motion: MotionSettings::default().resolved(PesVersion::Pes21),
                },
            )])
        };
        let mut patch = empty_patch();
        // The patch order is the exports' order, not the teams': `applied_players` is
        // [79205, 71405], and the warning sorts it for the report.
        for (id, name) in [(792, "/egg/"), (714, "/co/")] {
            patch.teams.push(PatchTeam {
                id,
                name: name.to_owned(),
                players: slot_05(),
            });
        }
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &[],
            false,
            &temp.path().join("aesthetics_patch.toml"),
            &patch,
        )
        .unwrap();

        let messages = messages(&rx);
        let codes: Vec<&str> = messages.iter().map(|(code, _)| code.as_str()).collect();
        assert_eq!(codes, ["savefile_player_not_stripped", "savefile_written"]);
        assert_eq!(
            messages[0].1,
            [("players".to_owned(), "71405, 79205".to_owned())]
        );
        assert_eq!(
            messages[1].1,
            [
                ("path".to_owned(), save.display().to_string()),
                ("teams".to_owned(), "792, 714".to_owned())
            ]
        );
    }

    #[test]
    fn update_writes_nothing_when_the_save_holds_the_team_but_none_of_its_players() {
        let temp = scratch("savefile_no_player_applied");
        let save = temp.path().join("EDIT00000000");
        let mut file =
            EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes21)).unwrap();
        // 71405's record renumbered to an id the save lacks: team 714 stays, he goes.
        let free = (1..).find(|id| file.player(*id).is_none()).unwrap();
        file.player_mut(71405).unwrap().id = free;
        fs::write(&save, file.to_bytes(&[7; 320]).unwrap()).unwrap();
        let before = fs::read(&save).unwrap();
        let mut patch = empty_patch();
        patch.teams.push(PatchTeam {
            id: 714,
            name: "/co/".to_owned(),
            players: BTreeMap::from([(
                5,
                PatchPlayer {
                    name: None,
                    shirt_name: None,
                    motion: MotionSettings::default().resolved(PesVersion::Pes21),
                },
            )]),
        });
        let (mut events, rx) = events();

        update(
            &mut events,
            Located::Configured(save.clone()),
            &[],
            false,
            &temp.path().join("aesthetics_patch.toml"),
            &patch,
        )
        .unwrap();

        let codes: Vec<String> = messages(&rx).into_iter().map(|(code, _)| code).collect();
        assert_eq!(codes, ["savefile_player_missing"]);
        assert_eq!(fs::read(&save).unwrap(), before, "byte-identical");
        assert!(
            !temp.path().join("EDIT00000000.bak").exists(),
            "no .bak turned over"
        );
    }
}
