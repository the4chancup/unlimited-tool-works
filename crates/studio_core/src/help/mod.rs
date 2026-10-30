//! Help chapter types. The help window is assembled from the tools: every registered tool
//! contributes one `HelpSection` through `StudioTool::help()`, and `studio_core` contributes the
//! core chapters. `chapters` lists them; `export` renders the
//! Markdown `UTW help` prints. The search and the window live in `shell::help_window`.

mod export;

use crate::events::MessageCode;
use crate::tool::StudioTool;

pub use export::run_help;

/// The generated Messages topic's slug: the one topic not in a chapter's `topics`
/// list, spelled wherever a slug or anchor names it (the window's `topic_index`
/// and `showing`, the CLI's topic lookup, a log line's link target).
pub(crate) const MESSAGES_SLUG: &str = "messages";

/// One tool's chapter of the help window: a title and its topics, in reading order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpSection {
    /// Chapter title, normally the tool's label.
    pub title: &'static str,
    /// Topics in order; the numbered file names in the tool's `help/` folder fix it.
    pub topics: Vec<HelpTopic>,
    /// The tool's whole catalog, in `messages.rs` order; empty for a core chapter, which then
    /// has no Messages topic.
    pub messages: Vec<CatalogRow>,
}

/// One topic of a chapter: a Markdown body embedded with `include_str!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpTopic {
    /// Link target within the chapter (`exports` in `[text](team-compiler#exports)`).
    pub slug: &'static str,
    /// Title shown in the chapter tree.
    pub title: &'static str,
    /// The Markdown body.
    pub body: &'static str,
}

/// Where a link, a search result or a message-id click opens the help window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HelpTarget {
    /// Tool id owning the chapter (`team-compiler`), or the core chapter's id.
    pub tool: String,
    /// Topic slug within the chapter; `None` opens the chapter's first topic.
    pub topic: Option<String>,
    /// Anchor inside the topic, such as a message code in the generated "Messages" topic.
    pub anchor: Option<String>,
}

impl HelpTarget {
    /// The catalog row `code` identifies: the generated Messages topic of the code's
    /// own tool, the code highlighted. A log line's code links here.
    pub fn message(code: &MessageCode) -> HelpTarget {
        HelpTarget {
            tool: code.tool_id.to_owned(),
            topic: Some(MESSAGES_SLUG.to_owned()),
            anchor: Some(code.code.to_string()),
        }
    }
}

/// One row of a tool's message catalog, as the help's generated Messages topic shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRow {
    /// The stable code the message carries (`export_extract_failed`).
    pub code: &'static str,
    /// The severity spelled out: `Info`, `Warning`, `Error`, `Fatal`, or two joined by
    /// ` or ` for a code whose severity depends on the run (`Error or Fatal`).
    pub severity: &'static str,
    /// What triggers the code (Markdown spans allowed).
    pub when: &'static str,
    /// What the tool does about it.
    pub effect: &'static str,
}

/// The chapter list the help window and `UTW help` share: the core chapters first,
/// then each of `tools`' `help()` under its id, in the order given (the window's is
/// the sidebar's, `UTW help`'s the registry's).
pub(crate) fn chapters<'a>(
    tools: impl IntoIterator<Item = &'a dyn StudioTool>,
) -> Vec<(&'static str, HelpSection)> {
    let mut chapters = vec![
        (
            "setting-up",
            HelpSection {
                title: "Setting up",
                topics: vec![HelpTopic {
                    slug: "install",
                    title: "Installing",
                    body: include_str!("core/install.md"),
                }],
                messages: Vec::new(),
            },
        ),
        (
            "first-run",
            HelpSection {
                title: "First run and data location",
                topics: vec![HelpTopic {
                    slug: "data-location",
                    title: "Choosing the data location",
                    body: include_str!("core/data_location.md"),
                }],
                messages: Vec::new(),
            },
        ),
    ];
    chapters.extend(tools.into_iter().map(|tool| (tool.id(), tool.help())));
    chapters
}
