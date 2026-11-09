//! The output folder's name (`spec/export_upgrader.md` "CLI"): the resolved
//! team's key, the coverage tag, then the rest of the old name — so the
//! compiler reads the output's team from its first word. The tag and what
//! decided it come back with the name, reported as `coverage_tag_chosen`, so
//! the name and the report cannot disagree.

use aesthetics_export::{ExportCoverage, coverage, is_name_separator, team_name};
use anyhow::anyhow;
use studio_core::CliError;

use crate::cli::Coverage;
use crate::upgrade::{Identity, invalid};

/// What decided a team export's coverage tag, as `coverage_tag_chosen`'s
/// `source` context prints it (`spec/export_upgrader.md` "CLI").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagSource {
    /// The `--coverage` flag.
    Flag,
    /// The name's second word already is the tag.
    Name,
    /// The `midcup` text the name holds.
    Midcup,
    /// The `additions` text the name holds.
    Additions,
    /// The name holds none of them: `Full` by default.
    Default,
}

impl TagSource {
    /// The `source` context's value.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TagSource::Flag => "flag",
            TagSource::Name => "name",
            TagSource::Midcup => "midcup",
            TagSource::Additions => "additions",
            TagSource::Default => "default",
        }
    }
}

/// The coverage tag a team export's output name takes and what decided it:
/// `coverage_tag_chosen`'s `tag` and `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TagChoice {
    /// `Full` or `Midcup`, as the name spells it.
    pub(crate) tag: &'static str,
    /// What decided the tag.
    pub(crate) source: TagSource,
}

/// The output name `output_name` builds.
#[derive(Debug)]
pub(crate) struct OutputName {
    /// The output folder's name.
    pub(crate) name: String,
    /// The coverage tag and its source; `None` for a referee export, which
    /// takes no tag.
    pub(crate) tag: Option<TagChoice>,
}

/// `name` minus its first token, the separator between kept
/// (`co - Spring` → ` - Spring`).
fn after_first_token(name: &str) -> &str {
    let Some(start) = name.find(|c: char| !is_name_separator(c)) else {
        return "";
    };
    match name[start..].find(is_name_separator) {
        Some(end) => &name[start + end..],
        None => "",
    }
}

/// The tag as an export's name spells it.
fn tag_name(tag: ExportCoverage) -> &'static str {
    match tag {
        ExportCoverage::Full => "Full",
        ExportCoverage::Midcup => "Midcup",
    }
}

/// `export_name`'s coverage tag for a team export (`spec/export_upgrader.md`
/// "CLI"), and the first `source` that holds: `name` when the name's second
/// word already is `Full` or `Midcup` (a `--coverage` naming the other tag
/// is refused, the member's word kept rather than renamed), `flag` for
/// `--coverage`, `midcup` or `additions` for the text the name holds (the
/// first of them found), `default` when it holds neither.
pub(crate) fn coverage_choice(
    export_name: &str,
    flag: Option<Coverage>,
) -> Result<TagChoice, CliError> {
    let carried = coverage(export_name);
    if let Some(flag) = flag {
        let flagged = match flag {
            Coverage::Full => ExportCoverage::Full,
            Coverage::Midcup => ExportCoverage::Midcup,
        };
        return match carried {
            None => Ok(TagChoice {
                tag: tag_name(flagged),
                source: TagSource::Flag,
            }),
            Some(carried) if carried == flagged => Ok(TagChoice {
                tag: tag_name(flagged),
                source: TagSource::Name,
            }),
            Some(carried) => Err(invalid(anyhow!(
                "--coverage {} does not apply, because the export's name already carries \
                 the {} tag",
                flag.as_str(),
                tag_name(carried)
            ))),
        };
    }
    Ok(match carried {
        Some(tag) => TagChoice {
            tag: tag_name(tag),
            source: TagSource::Name,
        },
        None => {
            let lowered = export_name.to_lowercase();
            let found = match (lowered.find("midcup"), lowered.find("additions")) {
                (None, None) => None,
                (Some(_), None) => Some(TagSource::Midcup),
                (None, Some(_)) => Some(TagSource::Additions),
                (Some(midcup), Some(additions)) => Some(if midcup <= additions {
                    TagSource::Midcup
                } else {
                    TagSource::Additions
                }),
            };
            match found {
                Some(source) => TagChoice {
                    tag: "Midcup",
                    source,
                },
                None => TagChoice {
                    tag: "Full",
                    source: TagSource::Default,
                },
            }
        }
    })
}

/// The output folder's name (`spec/export_upgrader.md` "CLI"): the resolved
/// team's key without its slashes, then the coverage tag `coverage_choice`
/// decided, then the rest of the old name. When the name's first token
/// folds to the key, the rest is the name after that token, its separator
/// kept (`co - Spring` → `co Full - Spring`), and a second token already
/// reading as a tag is kept rather than doubled; otherwise the rest is a
/// space and the whole name (`Numbers` → `co Full Numbers`). A referee
/// export takes no tag. A result equal to `export_name` ignoring ASCII case
/// gets ` upgraded`, so the input is never written into.
pub(crate) fn output_name(
    export_name: &str,
    identity: &Identity,
    flag: Option<Coverage>,
) -> Result<OutputName, CliError> {
    let (key, choice) = match identity {
        Identity::Referees => ("refs", None),
        Identity::Team { name, .. } => (
            name.as_str().trim_matches('/'),
            Some(coverage_choice(export_name, flag)?),
        ),
        Identity::ModelFolders => {
            unreachable!("upgrade's resolve yields a team or the referees")
        }
    };
    let tag = choice.map(|choice| choice.tag);
    let first_is_key = match team_name(export_name) {
        Some(found) => match identity {
            Identity::Team { name, .. } => found == *name,
            Identity::Referees => found.is_referees(),
            Identity::ModelFolders => {
                unreachable!("upgrade's resolve yields a team or the referees")
            }
        },
        None => false,
    };
    let mut output = key.to_owned();
    if first_is_key {
        if coverage(export_name).is_none()
            && let Some(tag) = tag
        {
            output.push(' ');
            output.push_str(tag);
        }
        output.push_str(after_first_token(export_name));
    } else {
        if let Some(tag) = tag {
            output.push(' ');
            output.push_str(tag);
        }
        output.push(' ');
        output.push_str(export_name);
    }
    if output.eq_ignore_ascii_case(export_name) {
        output.push_str(" upgraded");
    }
    Ok(OutputName {
        name: output,
        tag: choice,
    })
}

#[cfg(test)]
mod tests {
    use teams_list::TeamName;

    use super::*;

    /// `/co/`'s identity (the acceptance section's test team).
    fn co() -> Identity {
        Identity::Team {
            name: TeamName::new("co").unwrap(),
        }
    }

    /// `output_name` for /co/'s identity, unwrapped.
    fn named(export_name: &str, flag: Option<Coverage>) -> OutputName {
        output_name(export_name, &co(), flag).unwrap()
    }

    #[test]
    fn the_tag_is_inserted_after_the_key_with_the_separator_kept() {
        // EU-CLI-01's four names.
        assert_eq!(named("co - Spring", None).name, "co Full - Spring");
        assert_eq!(
            named("co Day 8 Additions", None).name,
            "co Midcup Day 8 Additions"
        );
        assert_eq!(
            named("co midcup Day 5", None).name,
            "co midcup Day 5 upgraded"
        );
        assert_eq!(
            named("co Full Spring", None).name,
            "co Full Spring upgraded"
        );
        // A name whose first token is not the key keeps the whole name as the
        // rest.
        assert_eq!(named("Numbers", None).name, "co Full Numbers");
        assert_eq!(named("day5", None).name, "co Full day5");
        // The first separator is kept with the rest, whatever it is.
        assert_eq!(named("co.Spring", None).name, "co Full.Spring");
        assert_eq!(named("co+Spring", None).name, "co Full+Spring");
        assert_eq!(named("co_Spring", None).name, "co Full_Spring");
        // A leading separator is not part of the rest: the first token is
        // still `co`, and what follows it is ` Spring`.
        assert_eq!(named("_co Spring", None).name, "co Full Spring");
        // Case-insensitive equality still collides on Windows, so ` upgraded`
        // is appended.
        assert_eq!(
            named("CO Full Spring", None).name,
            "co Full Spring upgraded"
        );
        // A referee export takes no tag.
        let referees = output_name("refs Spring", &Identity::Referees, None).unwrap();
        assert_eq!(referees.name, "refs Spring upgraded");
        assert!(referees.tag.is_none());
        assert_eq!(
            output_name("refs Cup", &Identity::Referees, None)
                .unwrap()
                .name,
            "refs Cup upgraded"
        );
    }

    #[test]
    fn the_tag_and_what_decided_it_come_back_with_the_name() {
        // EU-CLI-09's sources.
        for (export_name, expected) in [
            ("co - Spring", ("Full", TagSource::Default)),
            ("co Day 8 Additions", ("Midcup", TagSource::Additions)),
            ("co midcup Day 5", ("Midcup", TagSource::Name)),
            ("co Day 5 midcup", ("Midcup", TagSource::Midcup)),
            ("co Day 5 midcup additions", ("Midcup", TagSource::Midcup)),
            (
                "co Day 8 additions midcup",
                ("Midcup", TagSource::Additions),
            ),
            ("co Full Spring", ("Full", TagSource::Name)),
        ] {
            let choice = named(export_name, None)
                .tag
                .expect("a team export's name takes a tag");
            assert_eq!((choice.tag, choice.source), expected, "{export_name}");
        }
        // `--coverage` sets the tag in place of the name's guess.
        let flagged = named("co - Spring", Some(Coverage::Midcup));
        assert_eq!(flagged.name, "co Midcup - Spring");
        assert_eq!(
            flagged.tag.map(|choice| (choice.tag, choice.source)),
            Some(("Midcup", TagSource::Flag))
        );
        // The flag naming the tag the name already carries reports `name`;
        // naming the other tag is refused.
        let agreed = named("co Full Spring", Some(Coverage::Full));
        assert_eq!(agreed.name, "co Full Spring upgraded");
        assert_eq!(
            agreed.tag.map(|choice| (choice.tag, choice.source)),
            Some(("Full", TagSource::Name))
        );
        let error = output_name("co Full Spring", &co(), Some(Coverage::Midcup))
            .expect_err("the name already carries the other tag");
        assert_eq!(error.exit_code, crate::cli::INVALID, "{error}");
    }
}
