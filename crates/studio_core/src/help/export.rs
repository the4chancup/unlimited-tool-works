//! The help's Markdown side: a chapter rendered for `UTW help`, and `run_help`, the
//! command itself. A chapter prints as `# <title>`, each topic as `## <title>` and its body,
//! and the tool's catalog as the Messages table last. `--export <dir>` writes each chapter as
//! a Markdown file named by the chapter's id.

use std::io::Write;
use std::path::Path;

use crate::help::{CatalogRow, HelpSection, MESSAGES_SLUG, WHATS_NEW_ID, chapters};
use crate::tool::{CliError, StudioTool};

/// The chapter as one Markdown document: `# <title>`, then each topic as `## <title>` and its
/// body, then the Messages table when the section holds catalog rows.
pub(crate) fn chapter_markdown(section: &HelpSection) -> String {
    let mut markdown = format!("# {}\n\n", section.title);
    for topic in &section.topics {
        markdown.push_str(&format!("## {}\n\n{}\n\n", topic.title, topic.body.trim()));
    }
    if !section.messages.is_empty() {
        markdown.push_str(&format!(
            "## Messages\n\n{}\n\n{}",
            crate::help::MESSAGES_INTRO,
            table(&section.messages)
        ));
    }
    markdown
}

/// One topic of a chapter as Markdown, by slug: `## <title>` and its body, or the Messages
/// table for the `messages` slug. `None` names no topic of the chapter (nor `messages` on a
/// chapter with no catalog rows).
pub(crate) fn topic_markdown(section: &HelpSection, slug: &str) -> Option<String> {
    if slug == MESSAGES_SLUG {
        if section.messages.is_empty() {
            return None;
        }
        return Some(format!(
            "## Messages\n\n{}\n\n{}",
            crate::help::MESSAGES_INTRO,
            table(&section.messages)
        ));
    }
    let topic = section.topics.iter().find(|topic| topic.slug == slug)?;
    Some(format!("## {}\n\n{}\n", topic.title, topic.body.trim()))
}

/// The Messages topic's table: one row per catalog entry.
fn table(rows: &[CatalogRow]) -> String {
    let mut table = String::from("| Code | Severity | When | Effect |\n|---|---|---|---|\n");
    for row in rows {
        table.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            row.code, row.severity, row.when, row.effect
        ));
    }
    table
}

/// `UTW help`: prints the chapter list, one chapter or one topic to `out`, or with
/// `export` writes each chapter under the folder (created as needed) as a Markdown file named
/// by the chapter's id, printing nothing. An unknown chapter or topic id is a refusal (exit 2)
/// naming the ones that exist; a folder or file that cannot be written is a failure (exit 3)
/// naming the path.
pub fn run_help(
    tools: &[Box<dyn StudioTool>],
    chapter: Option<&str>,
    topic: Option<&str>,
    export: Option<&Path>,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    let chapters = chapters(tools.iter().map(Box::as_ref));
    let out_error = |error: std::io::Error| {
        CliError::new(
            3,
            anyhow::anyhow!("the help output could not be written ({error})"),
        )
    };
    if let Some(dir) = export {
        std::fs::create_dir_all(dir).map_err(|error| {
            CliError::new(
                3,
                anyhow::anyhow!("cannot create {} ({error})", dir.display()),
            )
        })?;
        for (id, section) in &chapters {
            let path = dir.join(format!("{id}.md"));
            std::fs::write(&path, chapter_markdown(section)).map_err(|error| {
                CliError::new(
                    3,
                    anyhow::anyhow!("cannot write {} ({error})", path.display()),
                )
            })?;
        }
        return Ok(());
    }
    let slugs_of = |section: &HelpSection| {
        let mut slugs: Vec<&str> = section.topics.iter().map(|topic| topic.slug).collect();
        if !section.messages.is_empty() {
            slugs.push(MESSAGES_SLUG);
        }
        slugs.join(", ")
    };
    let Some(id) = chapter else {
        for (id, section) in &chapters {
            writeln!(out, "{id}  {} ({})", section.title, slugs_of(section)).map_err(out_error)?;
        }
        return Ok(());
    };
    let (_, section) = chapters
        .iter()
        .find(|(chapter, _)| *chapter == id)
        .ok_or_else(|| {
            let ids: Vec<&str> = chapters.iter().map(|(id, _)| *id).collect();
            CliError::new(
                2,
                anyhow::anyhow!(
                    "there is no help chapter `{id}` (the chapters are {})",
                    ids.join(", ")
                ),
            )
        })?;
    let markdown = match topic {
        None => chapter_markdown(section),
        Some(slug) => topic_markdown(section, slug).ok_or_else(|| {
            CliError::new(
                2,
                anyhow::anyhow!(
                    "the chapter has no help topic `{slug}` (its topics are {})",
                    slugs_of(section)
                ),
            )
        })?,
    };
    out.write_all(markdown.as_bytes()).map_err(out_error)
}

/// `docs/prototypes/help.js`, the file the drafts draw their help window and their log
/// lines' texts from: two comment lines naming it, then the help the build assembles as one
/// `const BUILD_HELP` of JSON, every chapter's topics and catalog rows plus the line above a
/// Messages table. A test in `studio` fails while the file differs from this text, so a
/// changed topic or catalog row rewrites it rather than letting a hand-made copy drift. The
/// What's new chapter keeps its place but not its topic: its body is the changelog, which
/// changes with nearly every commit a member notices, and the drafts show a sample
/// changelog in its place.
pub fn draft_script(tools: &[Box<dyn StudioTool>]) -> String {
    let chapters: Vec<serde_json::Value> = chapters(tools.iter().map(Box::as_ref))
        .into_iter()
        .map(|(id, section)| {
            let topics = if id == WHATS_NEW_ID {
                &[][..]
            } else {
                &section.topics[..]
            };
            serde_json::json!({
                "id": id,
                "title": section.title,
                "topics": topics.iter().map(|topic| serde_json::json!({
                    "slug": topic.slug,
                    "title": topic.title,
                    "body": topic.body,
                })).collect::<Vec<_>>(),
                "messages": section.messages.iter().map(|row| serde_json::json!({
                    "code": row.code,
                    "severity": row.severity,
                    "when": row.when,
                    "effect": row.effect,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let help = serde_json::json!({
        "messagesIntro": crate::help::MESSAGES_INTRO,
        "chapters": chapters,
    });
    let json = serde_json::to_string_pretty(&help).expect("a serde_json::Value always serializes");
    format!(
        "// The build's help for the drafts' help window and log lines, never edited by hand:\n\
         // `just draft-help` writes it again; a test in `studio` fails while it differs from the build's.\n\
         const BUILD_HELP = {json};\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::help::{HelpTopic, chapters};

    fn section() -> HelpSection {
        HelpSection {
            title: "The Compiler",
            topics: vec![
                HelpTopic {
                    slug: "command-line",
                    title: "Running from a terminal",
                    body: "  `check` reads your exports.\n",
                },
                HelpTopic {
                    slug: "exports",
                    title: "Exports",
                    body: "An export is a folder.\n\n",
                },
            ],
            messages: vec![
                CatalogRow {
                    code: "export_extract_failed",
                    severity: "Error",
                    when: "an archive cannot be opened",
                    effect: "export skipped",
                },
                CatalogRow {
                    code: "notes_found",
                    severity: "Info",
                    when: "a valid `notes.txt` is present",
                    effect: "collected into `teamnotes.txt`",
                },
            ],
        }
    }

    #[test]
    fn a_chapter_prints_its_topics_then_the_messages_table() {
        assert_eq!(
            chapter_markdown(&section()),
            "# The Compiler\n\
             \n\
             ## Running from a terminal\n\
             \n\
             `check` reads your exports.\n\
             \n\
             ## Exports\n\
             \n\
             An export is a folder.\n\
             \n\
             ## Messages\n\
             \n\
             Where a row says \"the line\", it means the line the finding prints in the log or the terminal.\n\
             \n\
             | Code | Severity | When | Effect |\n\
             |---|---|---|---|\n\
             | `export_extract_failed` | Error | an archive cannot be opened | export skipped |\n\
             | `notes_found` | Info | a valid `notes.txt` is present | collected into `teamnotes.txt` |\n"
        );
    }

    // A row whose cells name "the line" is read under the sentence that says what
    // the line is, which stands between the heading and the table.
    #[test]
    fn the_messages_table_opens_with_what_the_line_means() {
        let markdown = chapter_markdown(&section());
        assert!(
            markdown.contains(&format!(
                "## Messages\n\n{}\n\n| Code | Severity | When | Effect |",
                crate::help::MESSAGES_INTRO
            )),
            "{markdown}"
        );
    }

    #[test]
    fn a_chapter_without_rows_has_no_messages_topic() {
        let mut section = section();
        section.messages = Vec::new();
        assert!(!chapter_markdown(&section).contains("## Messages"));
    }

    #[test]
    fn a_topic_prints_by_slug_and_messages_gives_the_table() {
        assert_eq!(
            topic_markdown(&section(), "exports"),
            Some("## Exports\n\nAn export is a folder.\n".to_owned())
        );
        let messages = topic_markdown(&section(), "messages").unwrap();
        assert!(messages.starts_with(&format!(
            "## Messages\n\n{}\n\n| Code | Severity | When | Effect |\n",
            crate::help::MESSAGES_INTRO
        )));
        assert!(messages.ends_with(
            "| `notes_found` | Info | a valid `notes.txt` is present | collected into `teamnotes.txt` |\n"
        ));
        assert_eq!(topic_markdown(&section(), "nope"), None);
        let mut bare = section();
        bare.messages = Vec::new();
        assert_eq!(topic_markdown(&bare, "messages"), None);
    }

    struct StubTool {
        id: &'static str,
        title: &'static str,
        topics: Vec<HelpTopic>,
        messages: Vec<CatalogRow>,
    }

    impl StudioTool for StubTool {
        fn id(&self) -> &'static str {
            self.id
        }
        fn label(&self) -> &'static str {
            self.title
        }
        fn icon(&self) -> &'static str {
            ""
        }
        fn view(&mut self, _ui: &mut egui::Ui, _ctx: &crate::tool::ToolContext) {}
        fn default_settings(&self) -> toml::Table {
            toml::Table::new()
        }
        fn help(&self) -> HelpSection {
            HelpSection {
                title: self.title,
                topics: self.topics.clone(),
                messages: self.messages.clone(),
            }
        }
        fn cli_command(&self) -> clap::Command {
            clap::Command::new(self.id)
        }
        fn cli_run(
            &self,
            _matches: &clap::ArgMatches,
            _ctx: &crate::tool::ToolContext,
        ) -> Result<u8, CliError> {
            Ok(0)
        }
    }

    fn stub(id: &'static str, title: &'static str) -> StubTool {
        StubTool {
            id,
            title,
            topics: vec![HelpTopic {
                slug: "topic-a",
                title: "Topic A",
                body: "Alpha's topic.\n",
            }],
            messages: Vec::new(),
        }
    }

    fn stub_tools() -> Vec<Box<dyn StudioTool>> {
        let mut alpha = stub("alpha", "Alpha");
        alpha.messages = vec![CatalogRow {
            code: "a_code",
            severity: "Error",
            when: "a trigger",
            effect: "an effect",
        }];
        let mut beta = stub("beta", "Beta");
        beta.topics = Vec::new();
        vec![Box::new(alpha), Box::new(beta)]
    }

    fn text(out: Vec<u8>) -> String {
        String::from_utf8(out).unwrap()
    }

    // The script's payload: the JSON value after `const BUILD_HELP = `, its closing `;`
    // stripped, parsed back.
    fn draft_json(script: &str) -> serde_json::Value {
        let json = script
            .split_once("const BUILD_HELP = ")
            .expect("the constant's line")
            .1
            .strip_suffix(";\n")
            .expect("the closing semicolon and newline");
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_draft_script_holds_every_chapter_in_registry_order_whats_new_without_its_topic() {
        let json = draft_json(&draft_script(&stub_tools()));
        let chapters = json["chapters"].as_array().unwrap();
        let ids: Vec<&str> = chapters
            .iter()
            .map(|chapter| chapter["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "setting-up",
                "first-run",
                "updates",
                "reporting",
                "whats-new",
                "alpha",
                "beta"
            ]
        );
        assert_eq!(chapters[4]["topics"], serde_json::json!([]));
        let alpha = &chapters[5];
        assert_eq!(
            alpha["topics"][0],
            serde_json::json!({"slug": "topic-a", "title": "Topic A", "body": "Alpha's topic.\n"})
        );
        assert_eq!(
            alpha["messages"][0],
            serde_json::json!({"code": "a_code", "severity": "Error", "when": "a trigger", "effect": "an effect"})
        );
        assert_eq!(chapters[6]["messages"], serde_json::json!([]));
        assert_eq!(json["messagesIntro"], crate::help::MESSAGES_INTRO);
    }

    #[test]
    fn the_draft_script_keeps_a_body_s_quotes_backslashes_and_newlines() {
        let body = "a \"quoted\" `C:\\path`\nnext line\n";
        let mut gamma = stub("gamma", "Gamma");
        gamma.topics = vec![HelpTopic {
            slug: "t",
            title: "T",
            body,
        }];
        let json = draft_json(&draft_script(&[Box::new(gamma)]));
        let gamma = json["chapters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|chapter| chapter["id"] == "gamma")
            .expect("the gamma chapter");
        assert_eq!(gamma["topics"][0]["body"], body);
    }

    #[test]
    fn the_draft_script_is_a_comment_then_one_constant() {
        let script = draft_script(&stub_tools());
        let (comment, rest) = script
            .split_once("const BUILD_HELP = ")
            .expect("the constant's line");
        let lines: Vec<&str> = comment.lines().collect();
        assert_eq!(lines.len(), 2, "{comment}");
        assert!(
            lines.iter().all(|line| line.starts_with("// ")),
            "{comment}"
        );
        assert!(rest.ends_with("};\n"));
    }

    #[test]
    fn the_core_chapters_come_first_then_the_tools_in_registry_order() {
        let chapters = chapters(stub_tools().iter().map(Box::as_ref));
        let ids: Vec<&str> = chapters.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ids,
            [
                "setting-up",
                "first-run",
                "updates",
                "reporting",
                "whats-new",
                "alpha",
                "beta"
            ]
        );
        let titles: Vec<&str> = chapters[..5].iter().map(|(_, s)| s.title).collect();
        assert_eq!(
            titles,
            [
                "Setting up",
                "First run and data location",
                "Updates",
                "Reporting a problem",
                "What's new"
            ]
        );
        assert_eq!(chapters[0].1.topics[0].slug, "install");
        assert!(chapters[0].1.topics[0].body.contains("unpack it anywhere"));
        assert_eq!(chapters[1].1.topics[0].slug, "data-location");
        assert_eq!(chapters[2].1.topics[0].slug, "updating");
        assert!(chapters[0].1.messages.is_empty());
    }

    // The What's new chapter is the repository's changelog after its title line, which
    // the topic's own title stands for, read here from the file itself rather than
    // through the chapter's own embedding.
    #[test]
    fn the_whats_new_chapter_is_the_changelog_after_its_title_line() {
        let chapters = chapters(stub_tools().iter().map(Box::as_ref));
        let (_, whats_new) = chapters
            .iter()
            .find(|(id, _)| *id == "whats-new")
            .expect("the What's new chapter");
        let file =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../CHANGELOG.md"))
                .unwrap();
        let after_title = file
            .strip_prefix("# Changelog\n")
            .expect("the premise: the file opens with its title line")
            .trim_start();
        assert_eq!(whats_new.topics.len(), 1);
        assert_eq!(whats_new.topics[0].slug, "changelog");
        assert_eq!(whats_new.topics[0].title, "Changelog");
        assert!(
            whats_new.topics[0].body == after_title,
            "the topic's body differs from CHANGELOG.md after its title line"
        );
        assert!(whats_new.messages.is_empty());
    }

    #[test]
    fn help_with_no_argument_lists_one_line_per_chapter() {
        let mut out = Vec::new();
        run_help(&stub_tools(), None, None, None, &mut out).unwrap();
        assert_eq!(
            text(out),
            "setting-up  Setting up (install)\n\
             first-run  First run and data location (data-location)\n\
             updates  Updates (updating)\n\
             reporting  Reporting a problem (report)\n\
             whats-new  What's new (changelog)\n\
             alpha  Alpha (topic-a, messages)\n\
             beta  Beta ()\n"
        );
    }

    #[test]
    fn a_chapter_a_topic_and_the_messages_slug_print_their_markdown() {
        let tools = stub_tools();
        let mut out = Vec::new();
        run_help(&tools, Some("alpha"), None, None, &mut out).unwrap();
        assert_eq!(text(out), chapter_markdown(&tools[0].help()));

        let mut out = Vec::new();
        run_help(&tools, Some("alpha"), Some("topic-a"), None, &mut out).unwrap();
        assert_eq!(text(out), "## Topic A\n\nAlpha's topic.\n");

        let mut out = Vec::new();
        run_help(&tools, Some("alpha"), Some("messages"), None, &mut out).unwrap();
        assert_eq!(
            text(out),
            format!(
                "## Messages\n\n{}\n\n| Code | Severity | When | Effect |\n|---|---|---|---|\n| `a_code` | Error | a trigger | an effect |\n",
                crate::help::MESSAGES_INTRO
            )
        );
    }

    #[test]
    fn an_unknown_chapter_or_topic_is_refused_naming_the_ones_that_exist() {
        let tools = stub_tools();
        let mut out = Vec::new();
        let error = run_help(&tools, Some("nope"), None, None, &mut out).unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert_eq!(
            error.to_string(),
            "there is no help chapter `nope` (the chapters are setting-up, first-run, updates, \
             reporting, whats-new, alpha, beta)"
        );
        let error = run_help(&tools, Some("alpha"), Some("nope"), None, &mut out).unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert_eq!(
            error.to_string(),
            "the chapter has no help topic `nope` (its topics are topic-a, messages)"
        );
        let error = run_help(&tools, Some("beta"), Some("messages"), None, &mut out).unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert_eq!(
            error.to_string(),
            "the chapter has no help topic `messages` (its topics are )"
        );
    }

    #[test]
    fn the_export_writes_each_chapter_and_prints_nothing() {
        let dir = std::env::temp_dir().join(format!("studio_help_export_{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        let tools = stub_tools();
        let mut out = Vec::new();
        run_help(&tools, None, None, Some(dir.as_path()), &mut out).unwrap();
        assert!(out.is_empty());
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        assert_eq!(
            files,
            [
                "alpha.md",
                "beta.md",
                "first-run.md",
                "reporting.md",
                "setting-up.md",
                "updates.md",
                "whats-new.md"
            ]
        );
        for (id, section) in chapters(tools.iter().map(Box::as_ref)) {
            assert_eq!(
                std::fs::read_to_string(dir.join(format!("{id}.md"))).unwrap(),
                chapter_markdown(&section),
                "{id}"
            );
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_export_into_a_path_that_is_a_file_fails_with_exit_3() {
        let file =
            std::env::temp_dir().join(format!("studio_help_export_file_{}", std::process::id()));
        std::fs::write(&file, "").unwrap();
        let mut out = Vec::new();
        let error =
            run_help(&stub_tools(), None, None, Some(file.as_path()), &mut out).unwrap_err();
        std::fs::remove_file(&file).unwrap();
        assert_eq!(error.exit_code, 3);
        assert!(
            error.to_string().contains(&file.display().to_string()),
            "{error}"
        );
    }
}
