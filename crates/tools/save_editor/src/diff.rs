//! The comparator's report (`spec/save_editor.md` "Comparator (gameplay + aesthetics)"):
//! two saves' players paired by id, each changed field a row, then the players only in one
//! save.

use pes_savefile::EditFile;
use pes_savefile::ops::compare::{CompareError, DiffScope, PlayerDiff, compare};
use serde::Serialize;
use studio_core::{EventLines, Message, PipelineEvent};

use crate::events::Report;
use crate::messages::{Code, Side, diff_fields, message, player_diff};
use crate::view::terms;

/// The report as data, rendered either as finding lines or as one JSON document (`--json`),
/// so the two forms cannot drift. `pub(crate)` for the comparator panel, which reads this
/// walk rather than walking the diff again.
#[derive(Serialize)]
pub(crate) struct DiffReport {
    /// `first`'s players that `second` also holds, with a row in scope, in `first`'s record
    /// order (a player only in `first` is in `only_in_first` alone).
    pub(crate) players: Vec<PlayerReport>,
    /// Ids `first` alone holds, in its record order.
    pub(crate) only_in_first: Vec<OnlyInPlayer>,
    /// Ids `second` alone holds, in its record order.
    pub(crate) only_in_second: Vec<OnlyInPlayer>,
}

/// One player of the report and his changed fields.
#[derive(Serialize)]
pub(crate) struct PlayerReport {
    /// The player id.
    pub(crate) id: u32,
    /// His rows of the scope the report was built for.
    pub(crate) rows: Vec<DiffRow>,
}

/// A player one save alone holds.
#[derive(Serialize)]
pub(crate) struct OnlyInPlayer {
    /// The player id.
    pub(crate) id: u32,
}

/// One changed field: `diff_fields`'s schema name and old/new texts, beside the
/// `PlayerDiff` they came from (`row_term` reads it for the comparator panel and the
/// findings' field label; `skip` keeps the JSON report unchanged).
#[derive(Serialize)]
pub(crate) struct DiffRow {
    /// The field's schema name, the key `--json` writes; the finding names the field as
    /// the views label it instead.
    pub(crate) field: String,
    /// `first`'s value.
    pub(crate) old: String,
    /// `second`'s value.
    pub(crate) new: String,
    /// The diff the strings render.
    #[serde(skip)]
    pub(crate) diff: PlayerDiff,
}

/// Reports the differences between `first` and `second` as findings: in `first`'s record
/// order, each player's rows of `scope` (every row when `None`); then the players only in
/// one save. Saves of two versions are the lib's `VersionMismatch`.
pub(crate) fn report(
    first: &EditFile,
    second: &EditFile,
    scope: Option<DiffScope>,
    report: &mut Report,
) -> Result<(), CompareError> {
    for finding in findings(first, second, scope)? {
        report.message(finding);
    }
    Ok(())
}

/// The report as text: the finding lines `save-editor diff <first> <second>` prints for
/// `scope`, in the command's order, formed as the console forms them: each field as the
/// views label it (PES 2017's Header), its stored values. The Comparator panel's Export
/// writes them; the `findings` walk is `report`'s, so the two cannot drift.
pub(crate) fn report_lines(
    first: &EditFile,
    second: &EditFile,
    scope: Option<DiffScope>,
) -> Result<Vec<String>, CompareError> {
    let mut lines = EventLines::new();
    Ok(findings(first, second, scope)?
        .into_iter()
        .map(|finding| {
            lines
                .line(&PipelineEvent::Message(finding))
                .expect("a Message event has a console line")
        })
        .collect())
}

/// The report's findings in `report`'s order: each player's rows of `scope`, the field
/// under the views' label (`terms::row_term`) with its stored values, then the players
/// only in one save. `report` and `report_lines` share this walk.
fn findings(
    first: &EditFile,
    second: &EditFile,
    scope: Option<DiffScope>,
) -> Result<Vec<Message>, CompareError> {
    let data = build(first, second, scope)?;
    let version = first.version();
    let mut findings = Vec::new();
    for player in &data.players {
        for row in &player.rows {
            let label = terms::row_term(version, &row.diff).label;
            findings.push(player_diff(player.id, &label, &row.old, &row.new));
        }
    }
    for (entries, side) in [
        (&data.only_in_first, Side::First),
        (&data.only_in_second, Side::Second),
    ] {
        for entry in entries {
            findings.push(message(
                Code::PlayerOnlyIn,
                vec![
                    ("player", entry.id.to_string()),
                    ("save", side.as_str().to_owned()),
                ],
            ));
        }
    }
    Ok(findings)
}

/// `report`'s data as one pretty-printed JSON document, sent as the run's `Output` event:
/// the only event `--json` emits on success, so stdout parses for tooling.
pub(crate) fn report_json(
    first: &EditFile,
    second: &EditFile,
    scope: Option<DiffScope>,
    report: &mut Report,
) -> Result<(), CompareError> {
    let data = build(first, second, scope)?;
    let text = serde_json::to_string_pretty(&data).expect("a report of ids and strings serializes");
    report.output(text);
    Ok(())
}

/// Both saves' report as data, each row's field by its schema name (what `--json`
/// writes) and its stored values. Saves of two versions are the lib's `VersionMismatch`.
/// `pub(crate)` for the comparator panel's worker.
pub(crate) fn build(
    first: &EditFile,
    second: &EditFile,
    scope: Option<DiffScope>,
) -> Result<DiffReport, CompareError> {
    let diff = compare(first, second)?;
    let mut data = DiffReport {
        players: Vec::new(),
        only_in_first: Vec::new(),
        only_in_second: Vec::new(),
    };
    for player in first.players() {
        if diff.only_in_a.contains(&player.id) {
            // An unmatched player is reported once, in `only_in_first`.
            continue;
        }
        let rows: Vec<DiffRow> = diff
            .players
            .iter()
            .find(|(id, _)| *id == player.id)
            .map(|(_, rows)| {
                rows.iter()
                    .filter(|row| scope.is_none_or(|scope| row.scope() == scope))
                    .map(|row| {
                        let (field, old, new) = diff_fields(row);
                        DiffRow {
                            field,
                            old,
                            new,
                            diff: row.clone(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if !rows.is_empty() {
            data.players.push(PlayerReport {
                id: player.id,
                rows,
            });
        }
    }
    for (ids, entries) in [
        (&diff.only_in_a, &mut data.only_in_first),
        (&diff.only_in_b, &mut data.only_in_second),
    ] {
        for &id in ids {
            entries.push(OnlyInPlayer { id });
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use pes_savefile::fixture_save;
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::{TEST_SALT, cli_stdout};

    /// `report_lines` is the CLI's report text for each scope: All,
    /// `--gameplay`, `--aesthetics`.
    #[test]
    fn report_lines_is_the_clis_stdout_for_each_scope() {
        let dir = crate::test_support::scratch("report_lines");
        // Two PES 21 saves: `B` carries 70105's speed +1 (a gameplay row),
        // 70106's boots +1 (an aesthetics row), and a player each side
        // lacks.
        let mut a = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
        a.player_mut(70110).unwrap().id = 9_999_999;
        std::fs::write(dir.join("A"), a.to_bytes(&TEST_SALT).unwrap()).unwrap();
        let mut b = EditFile::from_bytes(&fixture_save(PesVersion::Pes21)).unwrap();
        b.player_mut(70105).unwrap().stats.speed += 1;
        b.player_mut(70106).unwrap().appearance.boots_id += 1;
        std::fs::write(dir.join("B"), b.to_bytes(&TEST_SALT).unwrap()).unwrap();

        let first = dir.join("A").to_str().unwrap().to_owned();
        let second = dir.join("B").to_str().unwrap().to_owned();
        for (flag, scope) in [
            (&[][..], None),
            (&["--gameplay"][..], Some(DiffScope::Gameplay)),
            (&["--aesthetics"][..], Some(DiffScope::Aesthetics)),
        ] {
            let mut args = vec!["diff", first.as_str(), second.as_str()];
            args.extend(flag);
            let stdout = cli_stdout(&dir, &args);
            let lines = report_lines(&a, &b, scope).unwrap();
            assert!(!lines.is_empty(), "scope {scope:?} has rows");
            assert_eq!(stdout, lines.join("\n") + "\n", "scope {scope:?}");
        }
    }
}
