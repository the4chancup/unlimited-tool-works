//! The Ruleset editor's message catalog: each code the commands report, its
//! severity, and the `Message` it is sent as. Every finding is run-wide
//! (`Scope::Run`) and keeps what it is about (`Disposition::Keep`): a command
//! works on one file, not on exports. Each `CATALOG` row also carries the
//! member-facing `when`/`effect` the help's generated Messages topic shows;
//! the console prints the code with its context fields.

use std::borrow::Cow;
use std::path::Path;

use aatf::LoadError;
use studio_core::{CatalogRow, Disposition, Message, MessageCode, Scope, Severity};

/// The tool's id: its CLI subcommand, its settings section and the owner of
/// its message codes.
pub(crate) const TOOL_ID: &str = "ruleset-editor";

/// The codes the commands report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Code {
    /// `validate`: one finding from loading the rules file; its `line` and
    /// `column` when the text is broken, its `path` and `rule` when a load
    /// rule is broken, else its `error` text.
    RulesFinding,
    /// `update-logic`: the file's checking code was an older official version
    /// and was replaced with this build's (`path`, `from` the version).
    LogicUpdated,
    /// `update-logic`: the file already carries this build's checking code and
    /// was left as it was (`path`).
    LogicAlreadyCurrent,
    /// `summary -o`: the readable rules text was written (`path`); the
    /// file it replaced is kept as `<path>.bak`.
    SummaryWritten,
}

impl Code {
    /// The code as the console prints it and the help window lists it.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Code::RulesFinding => "rules_finding",
            Code::LogicUpdated => "logic_updated",
            Code::LogicAlreadyCurrent => "logic_already_current",
            Code::SummaryWritten => "summary_written",
        }
    }

    /// How serious the finding is: an `Error` makes the command's exit code 1.
    fn severity(self) -> Severity {
        match self {
            Code::RulesFinding => Severity::Error,
            Code::LogicUpdated | Code::LogicAlreadyCurrent | Code::SummaryWritten => Severity::Info,
        }
    }

    /// Every code, for the catalog test, in the enum's (and `CATALOG`'s) order.
    #[cfg(test)]
    const ALL: &'static [Code] = &[
        Code::RulesFinding,
        Code::LogicUpdated,
        Code::LogicAlreadyCurrent,
        Code::SummaryWritten,
    ];
}

/// Every code this tool emits, in the enum's order, with the member-facing
/// `when`/`effect` the help's generated Messages topic shows.
const CATALOG: &[(Code, &str, &str)] = &[
    (
        Code::RulesFinding,
        r#"`validate`: the rules file could not be loaded; the line names where it failed (a line and column when the text is broken, the entry that broke a rule) or, failing that, what went wrong."#,
        r#"The file is unchanged; the exit code is 1. Fix the file in a text editor and run it again."#,
    ),
    (
        Code::LogicUpdated,
        r#"`update-logic`: the file's checking code was an older official version than this build's (the line gives it as `from`)."#,
        r#"The checking code is replaced with this build's; the old file is kept beside it as a .bak copy."#,
    ),
    (
        Code::LogicAlreadyCurrent,
        r#"`update-logic`: the file already carries this build's checking code."#,
        r#"The file is left as it was; nothing is written."#,
    ),
    (
        Code::SummaryWritten,
        r#"`summary -o`: the file's readable rules text was written to the file the line names."#,
        r#"The file it replaced, if there was one, is kept beside it as a .bak copy."#,
    ),
];

/// The catalog rows the help's generated Messages topic is rendered from, in
/// `CATALOG` order (`lib.rs` `help`).
pub(crate) fn catalog() -> Vec<CatalogRow> {
    CATALOG
        .iter()
        .map(|(code, when, effect)| CatalogRow {
            code: code.as_str(),
            severity: code.severity().name(),
            when,
            effect,
        })
        .collect()
}

/// The message for one `validate` finding; `fields` carry its position
/// (`line`/`column`), its `path`/`rule`, or its `error` text.
pub(crate) fn rules_finding(fields: Vec<(&'static str, String)>) -> Message {
    message(Code::RulesFinding, fields)
}

/// The message when `update-logic` wrote the current checking code to `path`,
/// replacing official version `from`.
pub(crate) fn logic_updated(path: &Path, from: u32) -> Message {
    message(
        Code::LogicUpdated,
        vec![
            ("path", path.display().to_string()),
            ("from", from.to_string()),
        ],
    )
}

/// The message when `update-logic` left `path` as it was.
pub(crate) fn logic_already_current(path: &Path) -> Message {
    message(
        Code::LogicAlreadyCurrent,
        vec![("path", path.display().to_string())],
    )
}

/// The message when `summary -o` wrote the readable rules text to `path`.
pub(crate) fn summary_written(path: &Path) -> Message {
    message(
        Code::SummaryWritten,
        vec![("path", path.display().to_string())],
    )
}

/// The `rules_finding` messages for one `LoadError`: `Rules` fans out to one
/// message per rule, `Parse` carries its line and column, the rest their text.
/// `validate` prints them and the view's refused-open and refused-save
/// listings show them.
pub(crate) fn findings(error: &LoadError) -> Vec<Message> {
    match error {
        LoadError::Rules(rules) => rules
            .iter()
            .map(|rule| {
                rules_finding(vec![
                    ("path", rule.path.clone()),
                    ("rule", rule.rule.to_string()),
                ])
            })
            .collect(),
        LoadError::Parse {
            line,
            column,
            message,
        } => vec![rules_finding(vec![
            ("line", line.to_string()),
            ("column", column.to_string()),
            ("error", message.clone()),
        ])],
        other => vec![rules_finding(vec![("error", other.to_string())])],
    }
}

/// `error`'s findings as text lines, as `validate` prints them (the
/// view's refused-open and refused-save listings).
pub(crate) fn finding_lines(error: &LoadError) -> Vec<String> {
    findings(error).iter().map(finding_text).collect()
}

/// One finding's text, as `validate` prints it: the context fields in
/// order (`line=12, column=5, error=…`, `path=tiers[1], rule=…`).
fn finding_text(finding: &Message) -> String {
    finding
        .context
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The message for `code` with its context fields, in template order.
fn message(code: Code, context: Vec<(&'static str, String)>) -> Message {
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
    use super::*;

    /// Every code has a catalog row, in order, with its severity and a
    /// non-empty when/effect: the rows the help's Messages topic is made of.
    #[test]
    fn every_code_has_a_catalog_row() {
        let rows = catalog();
        assert_eq!(rows.len(), Code::ALL.len());
        for (code, row) in Code::ALL.iter().zip(&rows) {
            assert_eq!(row.code, code.as_str(), "catalog order follows the enum");
            assert_eq!(row.severity, code.severity().name());
            assert!(!row.when.is_empty() && !row.effect.is_empty());
        }
    }

    #[test]
    fn a_message_carries_its_code_severity_and_context_run_wide() {
        let message = logic_updated(Path::new("rules.4ccrules"), 1);
        assert_eq!(message.code.tool_id, "ruleset-editor");
        assert_eq!(message.code.code, "logic_updated");
        assert_eq!(message.severity, Severity::Info);
        assert_eq!(message.disposition, Disposition::Keep);
        assert_eq!(message.scope, Scope::Run);
        assert_eq!(
            message.context,
            vec![
                ("path".to_owned(), "rules.4ccrules".to_owned()),
                ("from".to_owned(), "1".to_owned())
            ]
        );
    }

    #[test]
    fn a_rules_finding_is_an_error() {
        let message = rules_finding(vec![("line", "12".to_owned()), ("column", "5".to_owned())]);
        assert_eq!(message.severity, Severity::Error);
    }
}
