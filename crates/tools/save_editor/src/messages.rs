//! The Save editor's message catalog: each code the commands report, its severity, and the
//! `Message` it is sent as. Every finding is run-wide (`Scope::Run`) and keeps what it is
//! about (`Disposition::Keep`): a command works on one save, not on exports, and a finding
//! that drops something says so with its own code. Each `CATALOG` row also carries the
//! member-facing `when`/`effect` the help's generated Messages topic shows (`spec/save_editor.md`
//! "Messages"); the console prints the code with its context fields.

use std::borrow::Cow;

use aatf::Violation;
use pes_savefile::interchange::aesthetics_patch::PatchSkip;
use pes_savefile::interchange::team_toml::ImportNote;
use pes_savefile::model::names::display_name;
use pes_savefile::ops::compare::PlayerDiff;
use pes_version::PesVersion;
use studio_core::{CatalogRow, Disposition, Message, MessageCode, Scope, Severity};
use team_widgets::violations::{Wording, rule_name, sentence};

/// The tool's id: its CLI subcommand, its settings section and the owner of its message codes.
pub(crate) const TOOL_ID: &str = "save-editor";

/// The codes the commands report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Code {
    /// `export-toml` wrote one team's file (`team`, `path`).
    TeamExported,
    /// `import-toml`: a key the document's version stores and the save's does not; dropped
    /// (`path`).
    ImportKeyNotInVersion,
    /// `import-toml`: a value the save's version cannot store (an instruction it lacks);
    /// written as off (`path`, `label`).
    ImportValueNotEncodable,
    /// `import-toml`: a face type above the save's version's cap, or a skin the version has
    /// no custom slot for; written as `to` (`path`, `from`, `to`).
    ImportValueCapped,
    /// `import-toml`: a `[players.NN]` whose roster slot is empty; skipped (`slot`).
    ImportSlotEmpty,
    /// `import-toml`: a name or shirt name longer than the save's field; cropped to it
    /// (`path`, `cropped`).
    ImportTextCropped,
    /// `apply-patch`: the save holds the team and at least one of its patch players was
    /// written (`team`). A team whose players the save all lacks is not listed.
    PatchTeamApplied,
    /// `apply-patch`: the save has no team with this id; its players were not looked at
    /// (`team`).
    PatchTeamNotInSave,
    /// `apply-patch`: the save has no player with this id (`player`).
    PatchPlayerNotInSave,
    /// `diff`: one field of a player in both saves, old to new (`player`, `field`, `old`,
    /// `new`).
    PlayerDiff,
    /// `diff`: a player in one save alone (`player`, `save`: `first` or `second`).
    PlayerOnlyIn,
    /// `export-teams-list`: a team of the save the list lacked, appended or put into a
    /// placeholder's slot (`team`, `name`).
    TeamsListAdded,
    /// `export-teams-list`: a name in both whose id becomes the save's (`name`, `old`, `new`).
    TeamsListOverridden,
    /// `export-teams-list`: a change dropped because its id collided with another row of the
    /// merged list (`team`, `name`).
    TeamsListUnresolved,
    /// `export-teams-list`: a team of the save left out because an earlier team folds to the
    /// same name, which the list can hold once (`team`, `name`, `kept`).
    TeamsListNameDuplicate,
    /// `export-teams-list`: the working rows that ended unchanged (`count`).
    TeamsListKept,
    /// `export-teams-list`: the merged list was written (`path`).
    TeamsListWritten,
    /// `export-teams-list` without `--yes`: the summary only, the file untouched (`path`).
    TeamsListNotWritten,
    /// `export-teams-list --yes`: the data directory cannot take the file; nothing is
    /// written (`path`, `error`).
    TeamsListReadOnly,
    /// `check-aatf`: one rule the team breaks under the ruleset (`team`; `player` for a
    /// player's violation; `rule`, the rule's name in words; `problem`, what is wrong). The
    /// severity is the violation's, so `aatf_violation` sets it on the message it builds.
    AatfViolation,
    /// `check-aatf`: the ruleset's check itself failed on the team, which the other teams'
    /// results do not cover (`team`, `error`).
    AatfCheckFailed,
    /// `check-aatf` without `--rules`: the `aatf_ruleset` setting names a ruleset that is gone
    /// from the data folder (or no longer loads), so the first built-in one is checked instead
    /// (`ruleset`, `used`).
    AatfRulesetGone,
    /// The Appearance section's FPC group: the player wears inners under a hidden body, which
    /// breaks the hiding.
    FpcInnersBreakHiding,
    /// The Appearance section's FPC group: the player wears undershorts under a hidden body,
    /// which breaks the hiding.
    FpcUndershortsBreakHiding,
    /// The Appearance section's FPC group: the player wears wrist or ankle taping under a
    /// hidden body, so the taped pieces still show.
    FpcTapingShowsPieces,
    /// The Appearance section's FPC group: the gloves checkbox is on with gloves ID 0, so the
    /// player wears winter gloves in winter conditions.
    FpcGlovesCheckboxWinterGloves,
    /// The Appearance section's FPC group: a custom skin on a player whose body is not hidden
    /// hides the body but not the jersey.
    FpcCustomSkinOnNonFpc,
}

/// Which of `diff`'s two saves a finding is about, as the console names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    /// The first save on the command line.
    First,
    /// The second.
    Second,
}

impl Side {
    /// The word the finding's `save` field carries.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Side::First => "first",
            Side::Second => "second",
        }
    }
}

impl Code {
    /// The code as the console prints it and the help window lists it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Code::TeamExported => "team_exported",
            Code::ImportKeyNotInVersion => "import_key_not_in_version",
            Code::ImportValueNotEncodable => "import_value_not_encodable",
            Code::ImportValueCapped => "import_value_capped",
            Code::ImportSlotEmpty => "import_slot_empty",
            Code::ImportTextCropped => "import_text_cropped",
            Code::PatchTeamApplied => "patch_team_applied",
            Code::PatchTeamNotInSave => "patch_team_not_in_save",
            Code::PatchPlayerNotInSave => "patch_player_not_in_save",
            Code::PlayerDiff => "player_diff",
            Code::PlayerOnlyIn => "player_only_in",
            Code::TeamsListAdded => "teams_list_added",
            Code::TeamsListOverridden => "teams_list_overridden",
            Code::TeamsListUnresolved => "teams_list_unresolved",
            Code::TeamsListNameDuplicate => "teams_list_name_duplicate",
            Code::TeamsListKept => "teams_list_kept",
            Code::TeamsListWritten => "teams_list_written",
            Code::TeamsListNotWritten => "teams_list_not_written",
            Code::TeamsListReadOnly => "teams_list_read_only",
            Code::AatfViolation => "aatf_violation",
            Code::AatfCheckFailed => "aatf_check_failed",
            Code::AatfRulesetGone => "aatf_ruleset_gone",
            Code::FpcInnersBreakHiding => "fpc_inners_break_hiding",
            Code::FpcUndershortsBreakHiding => "fpc_undershorts_break_hiding",
            Code::FpcTapingShowsPieces => "fpc_taping_shows_pieces",
            Code::FpcGlovesCheckboxWinterGloves => "fpc_gloves_checkbox_winter_gloves",
            Code::FpcCustomSkinOnNonFpc => "fpc_custom_skin_on_non_fpc",
        }
    }

    /// How serious the finding is: an `Error` makes the command's exit code 1.
    fn severity(self) -> Severity {
        match self {
            // Differences and marks are `diff`'s output, not errors: exit 0.
            Code::TeamExported
            | Code::PatchTeamApplied
            | Code::PlayerDiff
            | Code::PlayerOnlyIn
            | Code::TeamsListAdded
            | Code::TeamsListOverridden
            | Code::TeamsListKept
            | Code::TeamsListWritten
            | Code::TeamsListNotWritten
            | Code::FpcTapingShowsPieces => Severity::Info,
            // A value the save did not take as written, or a record the game does not read:
            // worth knowing, not a failure. A cross-version import converts, with a list of
            // warnings.
            Code::ImportKeyNotInVersion
            | Code::ImportValueNotEncodable
            | Code::ImportValueCapped
            | Code::ImportSlotEmpty
            | Code::ImportTextCropped
            | Code::AatfRulesetGone
            | Code::FpcGlovesCheckboxWinterGloves
            | Code::FpcCustomSkinOnNonFpc => Severity::Warning,
            // A skipped team or player is an error and the rest is written (`apply-patch`
            // exits 1); so is a dropped teams-list change and a list that cannot be written
            // (`export-teams-list` exits 1).
            Code::PatchTeamNotInSave
            | Code::PatchPlayerNotInSave
            | Code::TeamsListUnresolved
            | Code::TeamsListNameDuplicate
            | Code::TeamsListReadOnly
            | Code::FpcInnersBreakHiding
            | Code::FpcUndershortsBreakHiding => Severity::Error,
            // `check-aatf`: a failed check is an error; a violation's own severity decides,
            // and `aatf_violation` overwrites this default with it.
            Code::AatfViolation | Code::AatfCheckFailed => Severity::Error,
        }
    }

    /// Every code, for the catalog test, in the enum's (and `CATALOG`'s) order.
    #[cfg(test)]
    const ALL: &'static [Code] = &[
        Code::TeamExported,
        Code::ImportKeyNotInVersion,
        Code::ImportValueNotEncodable,
        Code::ImportValueCapped,
        Code::ImportSlotEmpty,
        Code::ImportTextCropped,
        Code::PatchTeamApplied,
        Code::PatchTeamNotInSave,
        Code::PatchPlayerNotInSave,
        Code::PlayerDiff,
        Code::PlayerOnlyIn,
        Code::TeamsListAdded,
        Code::TeamsListOverridden,
        Code::TeamsListUnresolved,
        Code::TeamsListNameDuplicate,
        Code::TeamsListKept,
        Code::TeamsListWritten,
        Code::TeamsListNotWritten,
        Code::TeamsListReadOnly,
        Code::AatfViolation,
        Code::AatfCheckFailed,
        Code::AatfRulesetGone,
        Code::FpcInnersBreakHiding,
        Code::FpcUndershortsBreakHiding,
        Code::FpcTapingShowsPieces,
        Code::FpcGlovesCheckboxWinterGloves,
        Code::FpcCustomSkinOnNonFpc,
    ];
}

/// Every code this tool emits, in the enum's (and the spec table's) order, with the
/// member-facing `when`/`effect` the help's generated Messages topic shows.
const CATALOG: &[(Code, &str, &str)] = &[
    (
        Code::TeamExported,
        r#"`export-toml` wrote one team's file."#,
        r#"Nothing to do."#,
    ),
    (
        Code::ImportKeyNotInVersion,
        r#"`import-toml` found a key the document's version stores but the save's does not."#,
        r#"The key is dropped from the import, but the rest is written."#,
    ),
    (
        Code::ImportValueNotEncodable,
        r#"`import-toml` found a value the save's version cannot store (an instruction it lacks)."#,
        r#"The value is written as off, and the rest of the import is written."#,
    ),
    (
        Code::ImportValueCapped,
        r#"`import-toml` found a face type above the cap the save's version has, or a custom skin the version has no slot for."#,
        r#"The value is lowered (the face type to 0, the skin to 1), and the rest of the import is written."#,
    ),
    (
        Code::ImportSlotEmpty,
        r#"`import-toml` found a `[players.NN]` whose roster slot is empty."#,
        r#"The player is skipped, but the rest is written."#,
    ),
    (
        Code::ImportTextCropped,
        r#"`import-toml` found a name or shirt name longer than the save's version has room for."#,
        r#"The text is cut to what the line shows, and the rest of the import is written."#,
    ),
    (
        Code::PatchTeamApplied,
        r#"`apply-patch` found the team in the save and wrote at least one of its patch players. A team whose patch players the save lacks entirely is not listed."#,
        r#"Nothing to do."#,
    ),
    (
        Code::PatchTeamNotInSave,
        r#"`apply-patch` could not find a team with this id in the save, so its players were not looked at."#,
        r#"The rest is written, and the command exits 1."#,
    ),
    (
        Code::PatchPlayerNotInSave,
        r#"`apply-patch` could not find a player with this id in the save."#,
        r#"The rest is written, and the command exits 1."#,
    ),
    (
        Code::PlayerDiff,
        r#"`diff` lists one field of a player who is in both saves, old value to new."#,
        r#"It is a report row. Differences are the output, so the command exits 0."#,
    ),
    (
        Code::PlayerOnlyIn,
        r#"`diff` found a player who is in one save alone (the line says which)."#,
        r#"It is a report row, and there is nothing to do."#,
    ),
    (
        Code::TeamsListAdded,
        r#"`export-teams-list` found a team of the save the list lacked and appended it or put it into a placeholder's slot."#,
        r#"The change is merged into the list, which is written only with `--yes`."#,
    ),
    (
        Code::TeamsListOverridden,
        r#"`export-teams-list` found a name in both the save and the list, and gave it the save's id."#,
        r#"The change is merged into the list, which is written only with `--yes`."#,
    ),
    (
        Code::TeamsListUnresolved,
        r#"`export-teams-list` dropped a change because its id collided with another row of the merged list."#,
        r#"The change is dropped, but the other changes are written with `--yes`, and the command exits 1."#,
    ),
    (
        Code::TeamsListNameDuplicate,
        r#"`export-teams-list` left a team of the save out because an earlier team folds to the same name, which the list can hold once."#,
        r#"The team is left out, and the command exits 1. Rename one of them in the save to keep both."#,
    ),
    (
        Code::TeamsListKept,
        r#"`export-teams-list` reports the rows that ended unchanged (the line gives the count)."#,
        r#"Nothing to do."#,
    ),
    (
        Code::TeamsListWritten,
        r#"`export-teams-list` wrote the merged list."#,
        r#"Nothing to do."#,
    ),
    (
        Code::TeamsListNotWritten,
        r#"`export-teams-list` ran without `--yes`, so it printed the summary only and left the file untouched."#,
        r#"Nothing is written. Run it again with `--yes` to write it."#,
    ),
    (
        Code::TeamsListReadOnly,
        r#"`export-teams-list --yes` found that the data directory cannot take the file."#,
        r#"Nothing is written, and the command exits 1."#,
    ),
    (
        Code::AatfViolation,
        r#"`check-aatf` found one rule the team breaks under the ruleset (the line names the team, the player for a player's violation, the rule and what is wrong)."#,
        r#"An error-severity violation makes the command exit 1, while a warning alone leaves it at 0."#,
    ),
    (
        Code::AatfCheckFailed,
        r#"`check-aatf` could not check the team, because the ruleset's check itself failed on it. The other teams' results do not cover it."#,
        r#"The other teams are still checked, but the command exits 1."#,
    ),
    (
        Code::AatfRulesetGone,
        r#"`check-aatf` ran without `--rules`, and the ruleset picked last in the Save editor is gone from the data folder or no longer loads (the line names it)."#,
        r#"The first built-in ruleset, which the line names, is checked instead. Pick the cup's rules again in the dropdown on the Save editor's toolbar."#,
    ),
    (
        Code::FpcInnersBreakHiding,
        r#"The player wears inners under a hidden body, which breaks the hiding."#,
        r#"Nothing is changed for you. Take the inners off, or the body will not hide."#,
    ),
    (
        Code::FpcUndershortsBreakHiding,
        r#"The player wears undershorts under a hidden body, which breaks the hiding."#,
        r#"Nothing is changed for you. Take the undershorts off, or the body will not hide."#,
    ),
    (
        Code::FpcTapingShowsPieces,
        r#"The player wears wrist or ankle taping under a hidden body, so the taped pieces still show."#,
        r#"This is deliberate in custom setups, so there is nothing to do."#,
    ),
    (
        Code::FpcGlovesCheckboxWinterGloves,
        r#"The gloves checkbox is on with gloves ID 0."#,
        r#"Nothing is changed for you. The player wears winter gloves in winter conditions, so untick the checkbox or give him a gloves ID if that is not wanted."#,
    ),
    (
        Code::FpcCustomSkinOnNonFpc,
        r#"A player wears a custom skin, but his body is not hidden by the FPC hide boots ID from Settings, so the skin hides his body but not his jersey."#,
        r#"Nothing is changed for you. Hide the body too, or pick a preset skin."#,
    ),
];

/// The catalog rows the help's generated Messages topic is rendered from, in `CATALOG`
/// order (`lib.rs` `help`).
pub(crate) fn catalog() -> Vec<CatalogRow> {
    CATALOG
        .iter()
        .map(|(code, when, effect)| CatalogRow {
            code: code.as_str(),
            severity: catalog_severity(*code),
            when,
            effect,
        })
        .collect()
}

/// The severity the Messages topic spells out: the code's own, the one code whose
/// `Message` overwrites it spelled as its spec row does.
fn catalog_severity(code: Code) -> &'static str {
    match code {
        Code::AatfViolation => "Error or Warning",
        _ => code.severity().name(),
    }
}

/// The message for one import note: what the import did not apply as written, and why.
pub(crate) fn import_note(note: &ImportNote) -> Message {
    match note {
        ImportNote::NotInThisVersion { path } => {
            message(Code::ImportKeyNotInVersion, vec![("path", path.clone())])
        }
        ImportNote::NotEncodable { path, label } => message(
            Code::ImportValueNotEncodable,
            vec![("path", path.clone()), ("label", label.clone())],
        ),
        ImportNote::Capped { path, from, to } => message(
            Code::ImportValueCapped,
            vec![
                ("path", path.clone()),
                ("from", from.to_string()),
                ("to", to.to_string()),
            ],
        ),
        ImportNote::EmptySlot { slot } => {
            message(Code::ImportSlotEmpty, vec![("slot", format!("{slot:02}"))])
        }
        ImportNote::TextCropped { path, cropped } => message(
            Code::ImportTextCropped,
            // As the game draws it: a colour code's bytes are no text a member reads.
            vec![("path", path.clone()), ("cropped", display_name(cropped))],
        ),
    }
}

/// The refusal's sentence for two saves of different versions, each named as a
/// member reads it (`PES 2021`): the CLI's refusals and the comparator panel's
/// pick share the one wording.
pub(crate) fn version_mismatch_sentence(
    first_role: &str,
    first: PesVersion,
    second_role: &str,
    second: PesVersion,
) -> String {
    format!(
        "{first_role} is a {first} save and {second_role} a {second} one, but both must be of the same version"
    )
}

/// One `diff` row's texts: the field by its schema name (`BootsId`, `FacialHairType`,
/// `Name`; `face_run` for the run's bits no field names, whose values are the two runs'
/// fingerprints) and the old and new values. The finding and the JSON report carry the
/// same three strings.
pub(crate) fn diff_fields(row: &PlayerDiff) -> (String, String, String) {
    match row {
        PlayerDiff::Field { field, old, new } => {
            (format!("{field:?}"), old.to_string(), new.to_string())
        }
        PlayerDiff::Text { text, old, new } => (format!("{text:?}"), old.clone(), new.clone()),
        PlayerDiff::Face { field, old, new } => {
            (format!("{field:?}"), old.to_string(), new.to_string())
        }
        PlayerDiff::FaceRun { old, new } => {
            ("face_run".to_owned(), old.to_string(), new.to_string())
        }
    }
}

/// The message for one `diff` row of `player`: `field`, `old` and `new` are `diff_fields`'s.
pub(crate) fn player_diff(player: u32, field: &str, old: &str, new: &str) -> Message {
    message(
        Code::PlayerDiff,
        vec![
            ("player", player.to_string()),
            ("field", field.to_owned()),
            ("old", old.to_owned()),
            ("new", new.to_owned()),
        ],
    )
}

/// The message for one patch item the save had no target for.
pub(crate) fn patch_skip(skip: PatchSkip) -> Message {
    match skip {
        PatchSkip::TeamNotInSave { team } => {
            message(Code::PatchTeamNotInSave, vec![("team", team.to_string())])
        }
        PatchSkip::PlayerNotInSave { player } => message(
            Code::PatchPlayerNotInSave,
            vec![("player", player.to_string())],
        ),
    }
}

/// The message for one `check-aatf` violation of `team`, worded as the window's AATF rows
/// word it: `player` is the rostered player's id when the violation names a slot,
/// `severity` the violation's (a suggestion is filtered by the caller, which prints nothing
/// for it), `rule` the rule's name in words and `problem` the sentence saying what is
/// wrong, worded as `wording` says: tiers, brackets and systems by their labels in the rules
/// the check ran under, stats as the checked save's game names them.
pub(crate) fn aatf_violation(
    team: u32,
    player: Option<u32>,
    violation: &Violation,
    wording: Wording<'_>,
    severity: Severity,
) -> Message {
    let mut context = vec![("team", team.to_string())];
    if let Some(player) = player {
        context.push(("player", player.to_string()));
    }
    context.push(("rule", rule_name(&violation.kind).to_owned()));
    context.push(("problem", sentence(violation, team, wording)));
    let mut message = message(Code::AatfViolation, context);
    message.severity = severity;
    message
}

/// The message for a team whose `check-aatf` run failed (a bug in the logic or in a custom
/// check): `error` is the `TeamViewError`'s or `CheckError`'s text.
pub(crate) fn aatf_check_failed(team: u32, error: String) -> Message {
    message(
        Code::AatfCheckFailed,
        vec![("team", team.to_string()), ("error", error)],
    )
}

/// The message for an `aatf_ruleset` setting naming a ruleset that is gone: `ruleset` is
/// the setting's value, `used` the description of the first built-in ruleset checked instead.
pub(crate) fn aatf_ruleset_gone(ruleset: &str, used: &str) -> Message {
    message(
        Code::AatfRulesetGone,
        vec![("ruleset", ruleset.to_owned()), ("used", used.to_owned())],
    )
}

/// The message for one `fpc::check` finding of the Appearance section's FPC group: the code
/// it maps to and the finding's own severity. A code `fpc::check` does not emit is a
/// programming error and panics naming it.
pub(crate) fn fpc_finding(finding: &fpc::Finding) -> Message {
    let code = match finding.code {
        "fpc_inners_break_hiding" => Code::FpcInnersBreakHiding,
        "fpc_undershorts_break_hiding" => Code::FpcUndershortsBreakHiding,
        "fpc_taping_shows_pieces" => Code::FpcTapingShowsPieces,
        "fpc_gloves_checkbox_winter_gloves" => Code::FpcGlovesCheckboxWinterGloves,
        "fpc_custom_skin_on_non_fpc" => Code::FpcCustomSkinOnNonFpc,
        other => panic!("fpc::check emits its five codes, got {other}"),
    };
    let mut message = message(code, vec![]);
    message.severity = match finding.severity {
        fpc::Severity::Info => Severity::Info,
        fpc::Severity::Warning => Severity::Warning,
        fpc::Severity::Error => Severity::Error,
    };
    message
}

/// The `when` text `CATALOG` spells for `message`'s code: the sentence a view draws
/// for the finding (the Appearance section's FPC findings).
pub(crate) fn when_text(message: &Message) -> &'static str {
    CATALOG
        .iter()
        .find(|(code, ..)| code.as_str() == message.code.code)
        .map(|(_, when, _)| *when)
        .expect("every emitted code has a catalog row")
}

/// The message for `code` with its context fields, in template order.
pub(crate) fn message(code: Code, context: Vec<(&'static str, String)>) -> Message {
    Message {
        code: MessageCode {
            tool_id: TOOL_ID,
            code: Cow::Borrowed(code.as_str()),
        },
        severity: code.severity(),
        disposition: Disposition::Keep,
        scope: Scope::Run,
        context: context
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use aatf::ViolationKind;

    use super::*;

    #[test]
    fn a_message_carries_its_code_severity_and_context_run_wide() {
        let message = message(
            Code::TeamExported,
            vec![("team", "714".to_owned()), ("path", "714.toml".to_owned())],
        );
        assert_eq!(message.code.tool_id, "save-editor");
        assert_eq!(message.code.code, "team_exported");
        assert_eq!(message.severity, Severity::Info);
        assert_eq!(message.disposition, Disposition::Keep);
        assert_eq!(message.scope, Scope::Run);
        assert_eq!(
            message.context,
            vec![
                ("team".to_owned(), "714".to_owned()),
                ("path".to_owned(), "714.toml".to_owned())
            ]
        );
    }

    /// Severity, code and context of a message as one line, the way the tests read them.
    fn line(message: &Message) -> String {
        let context: Vec<String> = message
            .context
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        format!(
            "{:?} {} ({})",
            message.severity,
            message.code.code,
            context.join(", ")
        )
    }

    #[test]
    fn each_import_note_is_a_warning_with_its_fields() {
        let path = || "players.03.stats.star".to_owned();
        assert_eq!(
            line(&import_note(&ImportNote::NotInThisVersion { path: path() })),
            "Warning import_key_not_in_version (path=players.03.stats.star)"
        );
        assert_eq!(
            line(&import_note(&ImportNote::NotEncodable {
                path: path(),
                label: "anchoring".to_owned()
            })),
            "Warning import_value_not_encodable (path=players.03.stats.star, label=anchoring)"
        );
        assert_eq!(
            line(&import_note(&ImportNote::Capped {
                path: path(),
                from: 19,
                to: 0
            })),
            "Warning import_value_capped (path=players.03.stats.star, from=19, to=0)"
        );
        assert_eq!(
            line(&import_note(&ImportNote::EmptySlot { slot: 7 })),
            "Warning import_slot_empty (slot=07)"
        );
        assert_eq!(
            line(&import_note(&ImportNote::TextCropped {
                path: "players.02.shirt_name".to_owned(),
                cropped: "ABCDEFGHIJKLMNOP".to_owned()
            })),
            "Warning import_text_cropped (path=players.02.shirt_name, cropped=ABCDEFGHIJKLMNOP)"
        );
        // A coloured name is shown as the game draws it, its codes left out.
        assert_eq!(
            line(&import_note(&ImportNote::TextCropped {
                path: "players.03.name".to_owned(),
                cropped: "\x11cFF0000FFAKI\x11dRA".to_owned()
            })),
            "Warning import_text_cropped (path=players.03.name, cropped=AKIRA)"
        );
    }

    #[test]
    fn each_diff_row_kind_is_an_info_line_with_the_field_old_and_new() {
        use pes_savefile::ops::fingerprint::FaceHash;
        use pes_savefile::schema::fields::{PlayerField, PlayerText};
        use pes_savefile::schema::ingame_face::IngameFaceField;
        let line_of = |row: &PlayerDiff| {
            let (field, old, new) = diff_fields(row);
            line(&player_diff(70105, &field, &old, &new))
        };
        assert_eq!(
            line_of(&PlayerDiff::Field {
                field: PlayerField::BootsId,
                old: 3601,
                new: 12
            }),
            "Info player_diff (player=70105, field=BootsId, old=3601, new=12)"
        );
        assert_eq!(
            line_of(&PlayerDiff::Text {
                text: PlayerText::ShirtName,
                old: "A".to_owned(),
                new: "B".to_owned()
            }),
            "Info player_diff (player=70105, field=ShirtName, old=A, new=B)"
        );
        assert_eq!(
            line_of(&PlayerDiff::Face {
                field: IngameFaceField::NoseType,
                old: 1,
                new: 2
            }),
            "Info player_diff (player=70105, field=NoseType, old=1, new=2)"
        );
        assert_eq!(
            line_of(&PlayerDiff::FaceRun {
                old: FaceHash([0, 1, 0xab, 0xff]),
                new: FaceHash([0; 4])
            }),
            "Info player_diff (player=70105, field=face_run, old=0001abff, new=00000000)"
        );
    }

    #[test]
    fn each_patch_skip_is_an_error_naming_its_item() {
        assert_eq!(
            line(&patch_skip(PatchSkip::TeamNotInSave { team: 999 })),
            "Error patch_team_not_in_save (team=999)"
        );
        assert_eq!(
            line(&patch_skip(PatchSkip::PlayerNotInSave { player: 71499 })),
            "Error patch_player_not_in_save (player=71499)"
        );
    }

    /// A violation's line names the team, the rostered player for a player's violation, the
    /// rule in words and the sentence the window's rows print, under the rules the check ran
    /// under (Autumn 26, whose `tall` bracket and `red` system print as Tall and Red). A
    /// team's own violation names no player, and the severity is the violation's.
    #[test]
    fn a_violation_line_names_the_rule_in_words_and_the_problem() {
        let rules = aatf::Rules::official();
        let wording = Wording {
            ruleset: rules.ruleset(),
            version: PesVersion::Pes17,
        };
        let violation = |slot, severity, kind| Violation {
            slot,
            severity,
            kind,
        };
        let stat = violation(
            Some(1),
            aatf::Severity::Error,
            ViolationKind::Stat {
                stat: "attacking_prowess".to_owned().try_into().unwrap(),
                value: 82,
                target: 77,
                at_most: true,
            },
        );
        assert_eq!(
            line(&aatf_violation(
                718,
                Some(71802),
                &stat,
                wording,
                Severity::Error
            )),
            "Error aatf_violation (team=718, player=71802, rule=Stat, \
             problem=Attacking Prowess 82, over the 77 the rules allow)"
        );
        // The slot's player is the one the slot holds, which the sentence compares with
        // the id the rule puts in the slot (slot 0 of team 718: 71801).
        let roster_order = violation(
            Some(0),
            aatf::Severity::Error,
            ViolationKind::RosterOrder { player: 71802 },
        );
        assert_eq!(
            line(&aatf_violation(
                718,
                Some(71802),
                &roster_order,
                wording,
                Severity::Error
            )),
            "Error aatf_violation (team=718, player=71802, rule=Roster order, \
             problem=the slot holds player 71802, not 71801)"
        );
        let quota = violation(
            None,
            aatf::Severity::Warning,
            ViolationKind::BracketQuota {
                system: "red".to_owned(),
                bracket: "tall".to_owned(),
                count: 8,
                quota: 10,
            },
        );
        assert_eq!(
            line(&aatf_violation(
                734,
                None,
                &quota,
                wording,
                Severity::Warning
            )),
            "Warning aatf_violation (team=734, rule=Height quota, \
             problem=8 players in the Tall bracket of the Red system, where its quota is 10)"
        );
        assert_eq!(
            line(&aatf_check_failed(
                701,
                "the check's script failed: x".to_owned()
            )),
            "Error aatf_check_failed (team=701, error=the check's script failed: x)"
        );
    }

    #[test]
    fn every_code_has_exactly_one_catalog_row_and_every_row_its_text() {
        for code in Code::ALL {
            let rows = CATALOG.iter().filter(|(known, ..)| known == code).count();
            assert_eq!(rows, 1, "{}", code.as_str());
        }
        for (index, (code, ..)) in CATALOG.iter().enumerate() {
            assert!(
                CATALOG[index + 1..].iter().all(|(other, ..)| other != code),
                "{} appears twice",
                code.as_str()
            );
        }
        for (code, when, effect) in CATALOG {
            assert!(!when.is_empty() && !effect.is_empty(), "{}", code.as_str());
        }
    }

    #[test]
    fn the_help_chapter_lists_the_whole_catalog_in_order() {
        use studio_core::StudioTool;

        let help = crate::Tool::new().help();
        let codes: Vec<&str> = help.messages.iter().map(|row| row.code).collect();
        let catalog: Vec<&str> = CATALOG.iter().map(|(code, ..)| code.as_str()).collect();
        assert_eq!(codes, catalog);
    }

    #[test]
    fn a_rows_text_is_written_for_a_member() {
        let row = CATALOG
            .iter()
            .find(|(code, ..)| *code == Code::TeamExported)
            .unwrap();
        assert_eq!(row.1, "`export-toml` wrote one team's file.");
        assert_eq!(row.2, "Nothing to do.");
        // The code whose `Message` overwrites the severity, spelled out as the Messages
        // topic shows it.
        assert_eq!(catalog_severity(Code::AatfViolation), "Error or Warning");
    }
}
