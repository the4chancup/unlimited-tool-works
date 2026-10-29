//! The help window, the draft `docs/prototypes/index.html`'s `drawHelp`: a non-modal,
//! resizable `egui::Window` the sidebar's `?`, F1, the About
//! popup's Help link and a notice's `ActionEffect::OpenHelp` open. It reads the chapters
//! `help::chapters` assembles (the core chapters first, then one per registered tool),
//! draws the current topic's Markdown with `egui_commonmark`, generates each chapter's
//! Messages topic from its catalog rows, and searches every paragraph and row from the
//! box above the tree.

use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::help::{HelpSection, HelpTarget, MESSAGES_SLUG};
use crate::widgets::{FORM_ROW_SPACING, text_box};

/// The window's state across frames: the chapters it was built over, whether it is open,
/// where it is (chapter index; the topic index in the list the tree shows — the written
/// topics, then `Messages` when the chapter has catalog rows), the paragraph or catalog
/// code a jump highlighted, the search text, and the Markdown cache the in-doc link
/// hooks live on.
pub(super) struct HelpWindow {
    chapters: Vec<(&'static str, HelpSection)>,
    open: bool,
    chapter: usize,
    topic: usize,
    /// The paragraph index a jump highlighted; cleared by the next plain navigation.
    highlight_para: Option<usize>,
    /// The catalog code a jump highlighted, same lifetime.
    highlight_code: Option<String>,
    search: String,
    /// The chapter whose tree header the next drawn frame opens (the settings menu's
    /// `pending` pattern).
    pending_open: Option<usize>,
    /// A jump scrolls the topic pane once: to the highlight, or to the top without one.
    scroll_pending: bool,
    cache: CommonMarkCache,
}

impl HelpWindow {
    /// The window over `chapters`, closed. Every in-doc link target (`<chapter-id>#<slug>`,
    /// `messages` included) is registered as a link hook on the cache once, so a rendered
    /// `[text](id#slug)` reports its click instead of opening a URL.
    pub(super) fn new(chapters: Vec<(&'static str, HelpSection)>) -> HelpWindow {
        let mut cache = CommonMarkCache::default();
        for (id, section) in &chapters {
            for topic in &section.topics {
                cache.add_link_hook(format!("{id}#{}", topic.slug));
            }
            if !section.messages.is_empty() {
                cache.add_link_hook(format!("{id}#messages"));
            }
        }
        HelpWindow {
            chapters,
            open: false,
            chapter: 0,
            topic: 0,
            highlight_para: None,
            highlight_code: None,
            search: String::new(),
            pending_open: None,
            scroll_pending: false,
            cache,
        }
    }

    /// Opens (or retargets) the window at `target` (the draft's `openHelp`): the chapter
    /// `target.tool` names, else the active tool's, else the first; the topic
    /// `target.topic` names (`messages` for the generated Messages topic), else the
    /// chapter's first; `target.anchor` the catalog code to highlight. The search box
    /// is cleared and the chapter's tree header opens on the next drawn frame.
    pub(super) fn open(&mut self, target: &HelpTarget, active_tool_id: &str) {
        let chapter = self
            .chapters
            .iter()
            .position(|(id, _)| *id == target.tool)
            .or_else(|| {
                self.chapters
                    .iter()
                    .position(|(id, _)| *id == active_tool_id)
            })
            .unwrap_or(0);
        let topic = match &target.topic {
            Some(slug) => self.topic_index(chapter, slug).unwrap_or(0),
            None => 0,
        };
        self.open = true;
        self.chapter = chapter;
        self.topic = topic;
        self.highlight_para = None;
        self.highlight_code = target.anchor.clone();
        self.search.clear();
        self.pending_open = Some(chapter);
        self.scroll_pending = true;
    }

    /// The extended topic index `slug` names within `chapter` — `messages` counts only
    /// when the chapter has catalog rows. `None` names no topic of the chapter.
    fn topic_index(&self, chapter: usize, slug: &str) -> Option<usize> {
        let (_, section) = &self.chapters[chapter];
        if slug == MESSAGES_SLUG {
            return (!section.messages.is_empty()).then_some(section.topics.len());
        }
        section.topics.iter().position(|topic| topic.slug == slug)
    }

    /// Shows `chapter`'s topic `topic` (its extended index), dropping any highlight and
    /// the search box's text — a tree click while the results are up would otherwise
    /// change the topic invisibly, the results staying on the right — and scrolling the
    /// pane to its top on the next frame.
    fn jump(&mut self, chapter: usize, topic: usize) {
        self.chapter = chapter;
        self.topic = topic;
        self.highlight_para = None;
        self.highlight_code = None;
        self.search.clear();
        self.pending_open = Some(chapter);
        self.scroll_pending = true;
    }

    /// A clicked in-doc link's jump (the draft's `helpLink`): `<chapter-id>#<slug>`
    /// resolves like `open`, without the fallbacks — every registered hook is a valid
    /// target.
    fn jump_to_link(&mut self, destination: &str) {
        let Some((id, slug)) = destination.split_once('#') else {
            return;
        };
        let Some(chapter) = self.chapters.iter().position(|(cid, _)| *cid == id) else {
            return;
        };
        let Some(topic) = self.topic_index(chapter, slug) else {
            return;
        };
        self.jump(chapter, topic);
    }

    /// Draws the window while open; the × closes it. egui keeps the rectangle by the
    /// window's id across a close and a reopen, for the session only. Keeping it across
    /// restarts would mean a settings save after every drag or resize of a window members
    /// open now and then.
    pub(super) fn show(&mut self, egui_ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut open = true;
        // `default_rect` counts the whole window — `default_size` is the *outer*
        // size, egui subtracts the frame's margins and the title bar inside
        // `Window::show` — so the first rect is 90% of Studio's, centered in it.
        // A moved or resized rect is kept by the window's id afterwards.
        let screen = egui_ctx.content_rect();
        let size = (screen.size() * 0.9).round();
        egui::Window::new(crate::theme::window_title_text("Help"))
            .id(egui::Id::unique("help-window"))
            .default_rect(egui::Rect::from_center_size(screen.center(), size))
            .collapsible(false)
            .resizable(true)
            .open(&mut open)
            .show(egui_ctx, |ui| self.contents(ui));
        self.open = open;
    }

    /// The window's body (the draft's `drawHelp`): the search box above, then the
    /// chapter tree in a left column and the right pane — the current topic, or the
    /// search results while the box holds non-blank text. The columns are `Panel`s,
    /// as the shell's own layout is: a `ui.horizontal` row would cap the scroll areas
    /// at one widget height and clip their interact rects out of the hit test.
    fn contents(&mut self, ui: &mut egui::Ui) {
        ui.add(text_box(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search the help")
                .desired_width(f32::INFINITY),
        ));
        ui.separator();
        egui::Panel::left("help-tree")
            .resizable(false)
            .exact_size(220.0)
            .show(ui, |ui| {
                // The default auto-shrink would size each pane to its first frame's
                // content and clip every interact below or right of it.
                egui::ScrollArea::vertical()
                    .id_salt("help-tree")
                    .auto_shrink(false)
                    .show(ui, |ui| self.draw_tree(ui));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            let searching = !self.search.trim().is_empty();
            let mut area = egui::ScrollArea::vertical()
                .id_salt("help-topic")
                .auto_shrink(false);
            if !searching && self.scroll_pending {
                // A jump with no highlight scrolls the topic to its top (the draft's
                // `helpScroll.topic = 0`); while the results are up the flag stays for
                // the topic pane's return.
                area = area.scroll_offset(egui::Vec2::ZERO);
            }
            area.show(ui, |ui| {
                if searching {
                    self.draw_results(ui);
                } else {
                    self.draw_topic(ui);
                }
            });
        });
    }

    /// The tree (the draft's `collapsing` loop): one `CollapsingHeader` per chapter in
    /// `chapters` order, salted by the chapter's id; under it one selectable label per
    /// topic and `Messages` last when the chapter has catalog rows. The chapter a jump
    /// targets opens on the frame after the jump (the settings menu's `pending`
    /// pattern); the current topic's label is drawn selected.
    fn draw_tree(&mut self, ui: &mut egui::Ui) {
        let pending = self.pending_open.take();
        let mut clicked = None;
        for (chapter, (id, section)) in self.chapters.iter().enumerate() {
            let mut header =
                crate::widgets::collapsing_header(section.title).id_salt(("help-chapter", *id));
            if pending == Some(chapter) {
                header = header.open(Some(true));
            }
            header.show(ui, |ui| {
                for (topic, entry) in section.topics.iter().enumerate() {
                    if ui
                        .selectable_label(
                            chapter == self.chapter && topic == self.topic,
                            entry.title,
                        )
                        .clicked()
                    {
                        clicked = Some((chapter, topic));
                    }
                }
                if !section.messages.is_empty()
                    && ui
                        .selectable_label(
                            chapter == self.chapter && self.topic == section.topics.len(),
                            "Messages",
                        )
                        .clicked()
                {
                    clicked = Some((chapter, section.topics.len()));
                }
            });
        }
        if let Some((chapter, topic)) = clicked {
            self.jump(chapter, topic);
        }
    }

    /// The right pane's topic (the draft's `drawHelpTopic`): its title as a heading,
    /// then the body rendered paragraph by paragraph, the highlighted one inside a
    /// filled frame and scrolled into view once, on the first drawn frame after the
    /// jump. A Messages topic is the catalog table instead. After the draw, an
    /// in-doc link whose hook reported a click jumps to its target.
    fn draw_topic(&mut self, ui: &mut egui::Ui) {
        let mut clicked_link = None;
        let mut applied = false;
        {
            let Some((_, section)) = self.chapters.get(self.chapter) else {
                self.scroll_pending = false;
                return;
            };
            let scroll = self.scroll_pending;
            if self.topic == section.topics.len() && !section.messages.is_empty() {
                ui.heading("Messages");
                let (consumed, link) = messages(
                    ui,
                    &mut self.cache,
                    section,
                    self.highlight_code.as_deref(),
                    scroll,
                );
                applied = consumed;
                clicked_link = link;
            } else if let Some(topic) = section.topics.get(self.topic) {
                ui.heading(topic.title);
                for (index, para) in paragraphs(topic.body).iter().enumerate() {
                    // A paragraph's `show` resets every link hook at its start, so the
                    // click's flag is read after each call, not once at the end.
                    let cache = &mut self.cache;
                    let response = ui
                        .push_id(("help-para", index), |ui| {
                            if self.highlight_para == Some(index) {
                                egui::Frame::new()
                                    .fill(ui.visuals().selection.bg_fill)
                                    .inner_margin(egui::Margin::same(4))
                                    .show(ui, |ui| {
                                        CommonMarkViewer::new().show(ui, cache, para);
                                    })
                                    .response
                            } else {
                                CommonMarkViewer::new().show(ui, cache, para).response
                            }
                        })
                        .response;
                    if scroll && self.highlight_para == Some(index) {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if let Some((dest, _)) = self.cache.link_hooks().iter().find(|(_, hit)| **hit) {
                        clicked_link = Some(dest.clone());
                    }
                }
                applied = !ui.is_sizing_pass();
            }
        }
        // A `Grid`'s first frame is an invisible sizing pass: a scroll consumed by it
        // would never reach the member, so it stays pending until a drawn pass.
        self.scroll_pending = self.scroll_pending && !applied;
        if let Some(dest) = clicked_link {
            self.jump_to_link(&dest);
        }
    }

    /// The results pane while the box holds non-blank text (the draft's
    /// `drawHelpResults`): one selectable label per hit, `Nothing matches.` for none.
    /// A click clears the box and jumps to the hit with the paragraph or row
    /// highlighted.
    fn draw_results(&mut self, ui: &mut egui::Ui) {
        let results = search(&self.chapters, self.search.trim());
        if results.is_empty() {
            ui.weak("Nothing matches.");
            return;
        }
        for result in results {
            if ui.selectable_label(false, &result.label).clicked() {
                self.search.clear();
                self.chapter = result.chapter;
                self.topic = result.topic;
                self.highlight_para = result.paragraph;
                self.highlight_code = result.code.map(str::to_owned);
                self.pending_open = Some(result.chapter);
                self.scroll_pending = true;
            }
        }
    }

    /// `(chapter id, topic slug — `messages` for the generated topic)` the window shows,
    /// `None` while closed. The harness reads position and highlight from state, which a
    /// frame's nodes do not carry.
    #[cfg(test)]
    pub(super) fn showing(&self) -> Option<(&'static str, &'static str)> {
        if !self.open {
            return None;
        }
        let (id, section) = &self.chapters[self.chapter];
        let slug = if self.topic < section.topics.len() {
            section.topics[self.topic].slug
        } else {
            MESSAGES_SLUG
        };
        Some((id, slug))
    }

    /// The highlighted paragraph's index or catalog code, whichever the jump set.
    #[cfg(test)]
    pub(super) fn highlight(&self) -> (Option<usize>, Option<&str>) {
        (self.highlight_para, self.highlight_code.as_deref())
    }

    /// The search box's text.
    #[cfg(test)]
    pub(super) fn search_text(&self) -> &str {
        &self.search
    }
}

/// A hit of `search`: where it lands (chapter and extended topic index, the paragraph
/// or the row's code) and the line the results list shows.
struct SearchResult {
    chapter: usize,
    topic: usize,
    /// The matching paragraph's index, for a topic hit.
    paragraph: Option<usize>,
    /// The matching row's code, for a Messages hit.
    code: Option<&'static str>,
    label: String,
}

/// A topic body split at blank lines, except inside a fenced code block, which stays
/// whole even holding a blank line. The one split `search` scans and the window renders,
/// so a hit's paragraph index is the highlight's.
fn paragraphs(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut fenced = false;
    let mut start = 0;
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        let bare = line.trim();
        if bare.starts_with("```") {
            fenced = !fenced;
        } else if !fenced && bare.is_empty() {
            let para = body[start..offset].trim();
            if !para.is_empty() {
                out.push(para);
            }
            start = offset + line.len();
        }
        offset += line.len();
    }
    let tail = body[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// The draft's `helpSearch`: case-insensitive substring over every paragraph of every
/// topic and every catalog row (code, severity, when, effect), in chapter order. A
/// paragraph hit is labeled `<chapter> › <topic> › <its first matching line>`, a row
/// hit `<chapter> › Messages › <code>: <when, or the cell that matched>`.
fn search(chapters: &[(&'static str, HelpSection)], query: &str) -> Vec<SearchResult> {
    let needle = query.trim().to_lowercase();
    let mut results = Vec::new();
    if needle.is_empty() {
        return results;
    }
    for (chapter, (_, section)) in chapters.iter().enumerate() {
        for (topic, entry) in section.topics.iter().enumerate() {
            for (index, para) in paragraphs(entry.body).iter().enumerate() {
                if !para.to_lowercase().contains(&needle) {
                    continue;
                }
                // The label's line is the paragraph's first matching physical line.
                let line = para
                    .lines()
                    .find(|line| line.to_lowercase().contains(&needle))
                    .map(str::to_owned)
                    .unwrap_or_else(|| para.replace('\n', " "));
                results.push(SearchResult {
                    chapter,
                    topic,
                    paragraph: Some(index),
                    code: None,
                    label: format!("{} › {} › {}", section.title, entry.title, line),
                });
            }
        }
        for row in &section.messages {
            let cells = [row.severity, row.when, row.effect];
            let code_hit = row.code.to_lowercase().contains(&needle);
            let cell = cells
                .iter()
                .position(|cell| cell.to_lowercase().contains(&needle));
            // A code match shows the when cell; a cell's match shows that cell.
            let text = if code_hit {
                row.when
            } else if let Some(cell) = cell {
                cells[cell]
            } else {
                continue;
            };
            results.push(SearchResult {
                chapter,
                topic: section.topics.len(),
                paragraph: None,
                code: Some(row.code),
                label: format!("{} › Messages › {}: {}", section.title, row.code, text),
            });
        }
    }
    results
}

/// The generated Messages topic's table (the draft's `messagesTable`): a header row,
/// then one row per catalog row — the code monospace, the severity colored, the two
/// text cells rendered with `CommonMarkViewer` inside a `Ui` of the text-column
/// width, so a long cell wraps instead of overflowing the pane. The highlighted
/// code's row is filled over all four cells and, on a jump frame (`scroll`), scrolled
/// into view. Returns whether the scroll landed on a drawn pass (a `Grid`'s first
/// frame is an invisible sizing one, `ui.is_sizing_pass()`) and an in-doc link's
/// destination if one of the cells' links was clicked.
fn messages(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    section: &HelpSection,
    highlight: Option<&str>,
    scroll: bool,
) -> (bool, Option<String>) {
    let measure = |text: &str, style: egui::TextStyle| {
        ui.fonts_mut(|fonts| {
            fonts.layout_no_wrap(
                text.to_owned(),
                style.resolve(ui.style()),
                egui::Color32::PLACEHOLDER,
            )
        })
        .size()
        .x
    };
    // A column is as wide as its widest cell, the header's included: "Severity"
    // is wider than "Info" or "Error", and a column measured from its rows alone
    // pushed the grid past the pane.
    let code_width = section
        .messages
        .iter()
        .map(|row| measure(row.code, egui::TextStyle::Monospace))
        .fold(measure("Code", egui::TextStyle::Body), f32::max);
    let severity_width = section
        .messages
        .iter()
        .map(|row| measure(row.severity, egui::TextStyle::Body))
        .fold(measure("Severity", egui::TextStyle::Body), f32::max);
    let text_width = text_column_width(
        ui.available_width(),
        code_width,
        severity_width,
        4.0 * ui.spacing().item_spacing.x,
    );
    let mut consumed = false;
    let mut clicked_link = None;
    // The drafts' `table.grid` rows: a line of text with 2 points above and below. A grid's
    // rows are otherwise at least a control's height, 3 points more than a line of text.
    egui::Grid::new(("help-messages", section.title))
        .num_columns(4)
        .spacing(egui::vec2(ui.spacing().item_spacing.x, FORM_ROW_SPACING))
        .min_row_height(ui.text_style_height(&egui::TextStyle::Body))
        .show(ui, |ui| {
            if scroll && !ui.is_sizing_pass() {
                consumed = true;
            }
            for head in ["Code", "Severity", "When", "Effect"] {
                ui.strong(head);
            }
            ui.end_row();
            for (index, row) in section.messages.iter().enumerate() {
                let code = ui.label(egui::RichText::new(row.code).monospace());
                let severity = ui.label(
                    egui::RichText::new(row.severity)
                        .color(severity_color(row.severity, ui.visuals())),
                );
                let (when, when_link) = text_cell(ui, cache, index, 2, text_width, row.when);
                let (effect, effect_link) = text_cell(ui, cache, index, 3, text_width, row.effect);
                // The row keeps the first link clicked, the When cell's before the
                // Effect's.
                let dest = when_link.or(effect_link);
                if clicked_link.is_none() {
                    clicked_link = dest;
                }
                if highlight == Some(row.code) {
                    // The four cells' span, marked after them — a translucent fill
                    // keeps the text readable under it.
                    let rect = code
                        .rect
                        .union(severity.rect)
                        .union(when.rect)
                        .union(effect.rect)
                        .expand2(egui::vec2(2.0, 0.0));
                    ui.painter().rect_filled(
                        rect,
                        2.0,
                        ui.visuals().selection.bg_fill.gamma_multiply(0.4),
                    );
                    if scroll {
                        code.scroll_to_me(Some(egui::Align::Center));
                    }
                }
                ui.end_row();
            }
        });
    (consumed, clicked_link)
}

/// The width each of the When and Effect columns gets: what the pane has left after
/// the code and severity columns and the `gap` of grid spacing, split in two, never
/// under 30 points each — the 60-point floor applies before the halving.
fn text_column_width(available: f32, code: f32, severity: f32, gap: f32) -> f32 {
    (available - code - severity - gap).max(60.0) / 2.0
}

/// A When/Effect cell: a `Ui` of the text-column width, the cell's Markdown rendered
/// inside it (`\|` read as `|`, the catalog table's escape), wrapping in that width;
/// the code and severity cells keep their natural width. Returns the cell's response
/// and, when the cell's link was clicked, its destination — the hooks are read right
/// after this cell's `show`, before the next cell's `show` resets them all.
fn text_cell(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    row: usize,
    column: usize,
    width: f32,
    text: &str,
) -> (egui::Response, Option<String>) {
    // A scoped child gets the grid's `available_rect`: anchored at the cell's top,
    // spanning the rows below. `allocate_ui_with_layout` would vertically center a
    // small rect in the cell instead, and a wrapping cell's Markdown would draw
    // past its bottom onto the next row. The child's `min_rect` is what the grid
    // measures the row height from.
    let inner = ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("help-msg", row, column))
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            // The last column's `available_rect` runs to the pane's edge; the cell
            // keeps the shared text-column width whatever the grid offers.
            ui.set_max_width(width);
            CommonMarkViewer::new().show(ui, cache, &text.replace("\\|", "|"))
        },
    );
    let dest = cache
        .link_hooks()
        .iter()
        .find(|(_, hit)| **hit)
        .map(|(dest, _)| dest.clone());
    (inner.response, dest)
}

/// A row's severity color (the draft's `sevCls`): a severity holding `Error` or `Fatal`
/// in red, `Warning` amber, anything else weak.
fn severity_color(severity: &str, visuals: &egui::Visuals) -> egui::Color32 {
    if severity.contains("Error") || severity.contains("Fatal") {
        visuals.error_fg_color
    } else if severity.contains("Warning") {
        visuals.warn_fg_color
    } else {
        visuals.weak_text_color()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use egui::accesskit::{self, Role, Toggled};

    use super::*;
    use crate::events::{MessageCode, Severity};
    use crate::help::{CatalogRow, HelpTopic};
    use crate::shell::test_harness::*;
    use crate::status::{ActionEffect, Notice, NoticeAction, NoticeKind};
    use crate::widgets::LogLine;

    /// Two chapters with a topic and a catalog row each, as a tool's `help()` yields.
    fn sections() -> Vec<(&'static str, HelpSection)> {
        vec![
            (
                "a",
                HelpSection {
                    title: "Alpha",
                    topics: vec![HelpTopic {
                        slug: "a-one",
                        title: "One",
                        body: "Alpha writes TEAMS.\n\nSecond paragraph.\n",
                    }],
                    messages: vec![CatalogRow {
                        code: "a_code",
                        severity: "Info",
                        when: "a trigger",
                        effect: "an effect",
                    }],
                },
            ),
            (
                "b",
                HelpSection {
                    title: "Beta",
                    topics: vec![HelpTopic {
                        slug: "b-one",
                        title: "B one",
                        body: "Beta body.\n",
                    }],
                    messages: vec![CatalogRow {
                        code: "b_code",
                        severity: "Warning",
                        when: "b trigger",
                        effect: "b effect",
                    }],
                },
            ),
        ]
    }

    #[test]
    fn paragraphs_split_at_blank_and_whitespace_lines() {
        let body = "One\nline\n\n  \nTwo.\n";
        assert_eq!(paragraphs(body), ["One\nline", "Two."]);
    }

    #[test]
    fn a_blank_line_inside_a_fenced_block_keeps_the_block_whole() {
        let body = "First.\n\n```\ncode\n\nmore code\n```\n\nAfter.\n";
        assert_eq!(
            paragraphs(body),
            ["First.", "```\ncode\n\nmore code\n```", "After."]
        );
    }

    #[test]
    fn the_search_matches_across_letter_case_and_orders_by_chapter() {
        let hits = search(&sections(), "tEaMs");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].label, "Alpha › One › Alpha writes TEAMS.");
        assert_eq!(hits[0].chapter, 0);
        assert_eq!(hits[0].topic, 0);
        assert_eq!(hits[0].paragraph, Some(0));
        assert_eq!(hits[0].code, None);

        // A code's hit is labeled with the row's when cell; every chapter's rows follow
        // its topics, chapter order holding.
        let hits = search(&sections(), "CODE");
        let labels: Vec<&str> = hits.iter().map(|hit| hit.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Alpha › Messages › a_code: a trigger",
                "Beta › Messages › b_code: b trigger",
            ],
        );

        // A non-code cell's hit is labeled with the cell that matched.
        let hits = search(&sections(), "effect");
        let labels: Vec<&str> = hits.iter().map(|hit| hit.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Alpha › Messages › a_code: an effect",
                "Beta › Messages › b_code: b effect",
            ],
        );

        assert!(search(&sections(), "no such text").is_empty());
        assert!(
            search(&sections(), "  ").is_empty(),
            "a blank query finds nothing"
        );
    }

    /// The first stub's chapter: two topics, the first holding an in-doc link to the
    /// second stub's `beta-topic`, and two catalog rows.
    fn alpha_section() -> HelpSection {
        HelpSection {
            title: "Alpha",
            topics: vec![
                HelpTopic {
                    slug: "alpha-start",
                    title: "Alpha start",
                    body: "Alpha keeps its TEAMS in a folder.\n\nSee [the other](b#beta-topic) for more.\n",
                },
                HelpTopic {
                    slug: "alpha-more",
                    title: "Alpha more",
                    body: "More of alpha.\n",
                },
            ],
            messages: vec![
                CatalogRow {
                    code: "alpha_info",
                    severity: "Info",
                    when: "alpha informs",
                    effect: "alpha notes",
                },
                CatalogRow {
                    code: "alpha_error",
                    severity: "Error",
                    when: "alpha fails",
                    effect: "alpha skips",
                },
            ],
        }
    }

    /// The second stub's chapter: two topics and two catalog rows.
    fn beta_section() -> HelpSection {
        HelpSection {
            title: "Beta",
            topics: vec![
                HelpTopic {
                    slug: "beta-topic",
                    title: "Beta topic",
                    body: "Beta does things.\n",
                },
                HelpTopic {
                    slug: "beta-more",
                    title: "Beta more",
                    body: "More beta notes.\n",
                },
            ],
            messages: vec![
                CatalogRow {
                    code: "beta_warn",
                    severity: "Warning",
                    when: "beta warns",
                    effect: "beta continues",
                },
                CatalogRow {
                    code: "beta_fatal",
                    severity: "Fatal",
                    when: "beta dies",
                    effect: "beta stops",
                },
            ],
        }
    }

    /// The help window's own node: a `Role::Window` labeled `Help`.
    fn help_window_node(nodes: &[accesskit::Node]) -> Option<&accesskit::Node> {
        nodes
            .iter()
            .find(|node| node.role() == Role::Window && node.label() == Some("Help"))
    }

    /// The sidebar's `?` button: a `Role::Button` labeled `Help` (the About popup's
    /// link and the window's title share the label; the role tells them apart).
    fn help_button(nodes: &[accesskit::Node]) -> &accesskit::Node {
        nodes
            .iter()
            .find(|node| node.role() == Role::Button && node.label() == Some("Help"))
            .expect("the sidebar's ? button")
    }

    /// The window's search box: its one text input.
    fn search_box(nodes: &[accesskit::Node]) -> &accesskit::Node {
        nodes
            .iter()
            .find(|node| node.role() == Role::TextInput)
            .expect("the search box is drawn")
    }

    /// A key event.
    fn key(key: egui::Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// A shell over the `alpha`/`beta` stubs with their chapters, `beta` active.
    fn help_shell(dir: &std::path::Path) -> Shell {
        let (a, calls_a) = stub_help("a", "Alpha", None, alpha_section());
        let (b, calls_b) = stub_help("b", "Beta", None, beta_section());
        let shell = shell_with(
            test_settings(dir),
            vec![a, b],
            &[],
            Some(dir.to_path_buf()),
            vec![calls_a, calls_b],
        );
        shell.ctx.switch_to_tool("b");
        shell
    }

    // ST-HELP-05
    #[test]
    fn the_help_window_first_opens_at_ninety_percent_of_the_shells_window() {
        let dir = TestDir::new("studio_core_help_size");
        for (screen, min, size) in [
            (
                egui::vec2(1037.0, 648.0),
                egui::pos2(52.0, 32.5),
                egui::vec2(933.0, 583.0),
            ),
            (
                egui::vec2(1280.0, 800.0),
                egui::pos2(64.0, 40.0),
                egui::vec2(1152.0, 720.0),
            ),
        ] {
            // One context per screen: a kept area rect is per-Context, and each
            // iteration stands for a program run at that window size.
            let egui_ctx = test_context();
            let Shell { mut app, .. } = help_shell(dir.path());
            let input = |events: Vec<egui::Event>| {
                let mut input = raw_input(events, true, None, None);
                input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen));
                input
            };
            run_frame(&egui_ctx, &mut app, input(vec![key(egui::Key::F1, true)]));
            // The window fades in: its node lands a few frames after it opens.
            let mut out = run_frame(&egui_ctx, &mut app, input(Vec::new()));
            for _ in 0..15 {
                out = run_frame(&egui_ctx, &mut app, input(Vec::new()));
            }
            let bounds = help_window_node(&out.nodes)
                .expect("the help window is drawn")
                .bounds()
                .expect("the window carries bounds");
            let expected = egui::Rect::from_min_size(min, size);
            assert!(
                (bounds.x0 as f32 - expected.min.x).abs() <= 0.5
                    && (bounds.y0 as f32 - expected.min.y).abs() <= 0.5
                    && (bounds.x1 as f32 - expected.max.x).abs() <= 0.5
                    && (bounds.y1 as f32 - expected.max.y).abs() <= 0.5,
                "screen {screen:?}: bounds {bounds:?} vs {expected:?}"
            );
        }
    }

    // ST-HELP-01
    #[test]
    fn f1_or_the_question_mark_opens_the_help_on_the_active_tools_chapter() {
        let dir = TestDir::new("studio_core_help_open");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        assert_eq!(app.active, 1, "beta is the active tool");
        assert!(help_window_node(&nodes).is_none(), "no help yet");

        // F1 opens the window on the active tool's chapter, its first topic drawn.
        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        // The chapter's header animates open over about 0.2 s of frames.
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert!(
            help_window_node(&nodes).is_some(),
            "F1 opened the help window"
        );
        assert_eq!(app.help_window.showing(), Some(("b", "beta-topic")));
        assert!(
            has_text(&nodes, "Beta does things."),
            "the chapter's first topic is drawn"
        );

        // The tree lists the core chapters, then the tools' in registry order. A
        // chapter's `CollapsingHeader` is a DisclosureTriangle node in the
        // AccessKit tree (egui 0.37's role names); the topic entries are buttons.
        let top = |title: &str| {
            nodes
                .iter()
                .find(|node| node.role() == Role::DisclosureTriangle && node.label() == Some(title))
                .unwrap_or_else(|| panic!("no chapter header {title:?}"))
                .bounds()
                .unwrap()
                .y0
        };
        assert!(
            top("Setting up") < top("First run and data location")
                && top("First run and data location") < top("Alpha")
                && top("Alpha") < top("Beta"),
            "the chapters are in tree order"
        );

        // Beta's chapter alone is open: its tree entries are the two topics, then
        // Messages last (Alpha's collapsed chapter shows none of its own).
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.label() == Some("Messages"))
                .count(),
            1,
            "the open chapter alone shows its Messages entry"
        );
        let top_entry = |label: &str| entry(&nodes, label).bounds().unwrap().y0;
        assert!(
            top_entry("Beta topic") < top_entry("Beta more")
                && top_entry("Beta more") < top_entry("Messages"),
            "Messages is the chapter's last tree entry"
        );

        // Clicking it draws the catalog: every code with its severity.
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(app.help_window.showing(), Some(("b", "messages")));
        for text in ["beta_warn", "Warning", "beta_fatal", "Fatal"] {
            assert!(has_text(&nodes, text), "the catalog row is drawn: {text}");
        }

        // The × closes it; the `?` opens it again on the active chapter.
        click(&egui_ctx, &mut app, entry(&nodes, "Close window"));
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            help_window_node(&nodes).is_none(),
            "the × closed the window"
        );
        assert_eq!(app.help_window.showing(), None);

        click(&egui_ctx, &mut app, help_button(&nodes));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(
            app.help_window.showing(),
            Some(("b", "beta-topic")),
            "the ? reopened the help on the active tool's chapter"
        );
        assert!(has_text(&nodes, "Beta does things."));
    }

    // ST-HELP-02
    #[test]
    fn the_search_lists_row_and_paragraph_hits_and_jumps_to_them() {
        let dir = TestDir::new("studio_core_help_search");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        click(&egui_ctx, &mut app, search_box(&nodes));

        // A catalog code lists its row as "<chapter> › Messages › <code>: <when>".
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("beta_warn".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(
            &egui_ctx,
            &mut app,
            entry(&nodes, "Beta › Messages › beta_warn: beta warns"),
        );
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(app.help_window.showing(), Some(("b", "messages")));
        assert_eq!(
            app.help_window.highlight(),
            (None, Some("beta_warn")),
            "the clicked code is the highlight"
        );
        assert!(
            app.help_window.search_text().is_empty(),
            "the click cleared the box"
        );
        assert!(has_text(&nodes, "beta warns"), "the row is drawn");

        // A topic body's word, in another letter case, lists the paragraph hit as
        // "<chapter> › <topic> › <its first matching line>".
        click(&egui_ctx, &mut app, search_box(&nodes));
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("TeAmS".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(
            &egui_ctx,
            &mut app,
            entry(
                &nodes,
                "Alpha › Alpha start › Alpha keeps its TEAMS in a folder.",
            ),
        );
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(app.help_window.showing(), Some(("a", "alpha-start")));
        assert_eq!(
            app.help_window.highlight(),
            (Some(0), None),
            "the clicked paragraph is the highlight"
        );
        assert!(has_text(&nodes, "Alpha keeps its TEAMS in a folder."));

        // A string in no topic: the empty result.
        click(&egui_ctx, &mut app, search_box(&nodes));
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("zzz no hit".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(has_text(&nodes, "Nothing matches."));
    }

    // ST-HELP-03
    #[test]
    fn an_in_doc_link_opens_the_topic_it_names() {
        let dir = TestDir::new("studio_core_help_link");
        let (a, calls_a) = stub_help("a", "Alpha", None, alpha_section());
        let (b, calls_b) = stub_help("b", "Beta", None, beta_section());
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        let egui_ctx = test_context();

        // Alpha's chapter opens on `alpha-start`, whose second paragraph holds the link.
        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(app.help_window.showing(), Some(("a", "alpha-start")));
        click(&egui_ctx, &mut app, entry(&nodes, "the other"));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(
            app.help_window.showing(),
            Some(("b", "beta-topic")),
            "the in-doc link jumped to Beta's topic"
        );
        assert!(has_text(&nodes, "Beta does things."));
    }

    // ST-HELP-03
    #[test]
    fn a_log_lines_message_code_opens_the_help_at_its_catalog_row() {
        let dir = TestDir::new("studio_core_help_loglink");
        // Alpha's view is a log panel showing a finding line of its own catalog.
        let (a, calls_a) = stub_log(
            "a",
            "Alpha",
            alpha_section(),
            vec![LogLine {
                severity: Severity::Error,
                code: Some(MessageCode {
                    tool_id: "a",
                    code: Cow::Borrowed("alpha_error"),
                }),
                text: "- Error alpha_error at Players/01 - T".to_owned(),
            }],
        );
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        // egui reports a `ui.link` as a `Link` node carrying its text in `label`
        // (egui 0.37's `Role`, replacing `WidgetType`).
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        let link = nodes
            .iter()
            .find(|node| node.role() == Role::Link && node.label() == Some("alpha_error"))
            .expect("the code's link node");
        click(&egui_ctx, &mut app, link);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(
            app.help_window.showing(),
            Some(("a", "messages")),
            "the help opened at the code's Messages topic"
        );
        assert_eq!(
            app.help_window.highlight(),
            (None, Some("alpha_error")),
            "the row is highlighted"
        );
        assert!(has_text(&nodes, "alpha fails"), "the row is drawn");
    }

    #[test]
    fn an_openhelp_notice_opens_the_window_at_the_target_closing_a_modal() {
        let dir = TestDir::new("studio_core_help_notice");
        let Shell { mut app, ctx, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        // A notice whose action is OpenHelp opens the window at the target, the
        // anchor highlighted.
        ctx.notify(Notice {
            text: "see the help".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Read up".to_owned(),
                effect: ActionEffect::OpenHelp(HelpTarget {
                    tool: "b".to_owned(),
                    topic: Some("messages".to_owned()),
                    anchor: Some("beta_warn".to_owned()),
                }),
            }),
        });
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Read up"));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert_eq!(app.help_window.showing(), Some(("b", "messages")));
        assert_eq!(
            app.help_window.highlight(),
            (None, Some("beta_warn")),
            "the notice's anchor is the highlight"
        );
        assert!(has_text(&nodes, "beta warns"), "the row is drawn");

        // Opening the help while the settings menu is open closes the menu.
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Settings"));
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(has_text(&nodes, "Threads"), "the settings menu is open");
        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 10).nodes;
        assert!(
            !has_text(&nodes, "Threads"),
            "opening the help closed the menu"
        );
        assert_eq!(app.help_window.showing(), Some(("b", "beta-topic")));
    }

    #[test]
    fn a_long_catalog_cell_wraps_inside_the_pane() {
        let dir = TestDir::new("studio_core_help_wrap");
        // A When cell the Team compiler's size: ~400 characters.
        let long = Box::leak("plain words and more words ".repeat(16).into_boxed_str());
        let mut section = alpha_section();
        section.messages[0].when = long;
        let (a, calls_a) = stub_help("a", "Alpha", None, section);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;

        // The wrapped label is the `Label` node carrying the whole cell text; its
        // `TextRun` children are per row (each one line tall).
        let cell = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|text| text.contains("more words"))
            })
            .expect("the when cell's label node")
            .bounds()
            .unwrap();
        let code = nodes
            .iter()
            .find(|node| node.value() == Some("alpha_info"))
            .expect("the code cell's text node")
            .bounds()
            .unwrap();
        let effect = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some("alpha notes"))
            .expect("the effect cell's label node")
            .bounds()
            .unwrap();
        // The When cell is at least a third of the row it spans. Its right edge is not
        // checked against the window: AccessKit clips node bounds to the window, so that
        // check cannot fail; `text_column_width`'s own test holds the width.
        let row_width = effect.x1 - code.x0;
        assert!(
            cell.width() >= row_width / 3.0,
            "the When cell is at least a third of the row's width: {} >= {}",
            cell.width(),
            row_width / 3.0,
        );
        assert!(
            cell.height() > 2.0 * code.height(),
            "the long cell wraps onto several lines: {} vs {}",
            cell.height(),
            code.height(),
        );
    }

    #[test]
    fn a_wrapping_catalog_cell_pushes_the_next_row_down() {
        let dir = TestDir::new("studio_core_help_rowheight");
        let mut section = alpha_section();
        // Long enough to wrap onto at least four lines in the default window.
        let long = Box::leak(
            "a condition long enough to need several wrapped lines in the pane "
                .repeat(4)
                .into_boxed_str(),
        );
        section.messages = vec![
            CatalogRow {
                code: "first_code",
                severity: "Info",
                when: long,
                effect: "first effect",
            },
            CatalogRow {
                code: "second_code",
                severity: "Warning",
                when: "second",
                effect: "second effect",
            },
            CatalogRow {
                code: "third_code",
                severity: "Error",
                when: "third",
                effect: "third effect",
            },
        ];
        let (a, calls_a) = stub_help("a", "Alpha", None, section);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;

        let when = label_bounds(&nodes, "condition long enough")
            .expect("the long When cell")
            .bounds()
            .unwrap();
        let second = label_bounds(&nodes, "second_code")
            .expect("the second row's code")
            .bounds()
            .unwrap();
        let third = label_bounds(&nodes, "third_code")
            .expect("the third row's code")
            .bounds()
            .unwrap();
        // All three are inside the window: the vertical bounds are real, not
        // window-clipped.
        let window = help_window_node(&nodes)
            .expect("the help window is drawn")
            .bounds()
            .unwrap();
        for (name, bounds) in [("when", when), ("second", second), ("third", third)] {
            assert!(
                bounds.y0 >= window.y0 && bounds.y1 <= window.y1,
                "{name} is in view: {bounds:?} vs {window:?}"
            );
        }
        assert!(
            when.height() > 3.0 * second.height(),
            "the When cell wraps onto at least four lines: {} vs {}",
            when.height(),
            second.height()
        );
        assert!(
            second.y0 >= when.y1,
            "the second row starts at or below the long cell's bottom: {} >= {}",
            second.y0,
            when.y1
        );
        assert!(
            third.y0 >= second.y1,
            "the third row starts at or below the second's bottom: {} >= {}",
            third.y0,
            second.y1
        );
    }

    // The Messages table is a drafts' `table.grid`: its one-line rows stand 19 apart, a
    // 15-point line of text and the forms' row spacing of 4, as the draft's header row
    // measures in a browser.
    #[test]
    fn one_line_catalog_rows_stand_19_apart() {
        let dir = TestDir::new("studio_core_help_rowpitch");
        let mut section = alpha_section();
        section.messages = ["first_code", "second_code", "third_code"]
            .into_iter()
            .map(|code| CatalogRow {
                code,
                severity: "Info",
                when: "short",
                effect: "short effect",
            })
            .collect();
        let (a, calls_a) = stub_help("a", "Alpha", None, section);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;

        let top = |code: &str| {
            label_bounds(&nodes, code)
                .and_then(accesskit::Node::bounds)
                .unwrap_or_else(|| panic!("the {code} row"))
                .y0
        };
        for (upper, lower) in [("first_code", "second_code"), ("second_code", "third_code")] {
            let pitch = top(lower) - top(upper);
            assert!(
                (pitch - 19.0).abs() <= 0.5,
                "{lower} stands 19 under {upper}, not {pitch}"
            );
        }
    }

    #[test]
    fn f1_opens_nothing_while_the_first_run_dialog_is_up() {
        let (a, calls_a) = stub_help("a", "Alpha", None, alpha_section());
        let dir = TestDir::new("studio_core_help_firstrun");
        // No data directory: the first-run dialog is up and nothing else is drawn.
        let Shell { mut app, .. } =
            shell_with(test_settings(dir.path()), vec![a], &[], None, vec![calls_a]);
        let egui_ctx = test_context();

        let nodes = frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true).nodes;
        assert!(
            help_window_node(&nodes).is_none(),
            "F1 opens nothing while the first-run dialog is up"
        );
        assert_eq!(app.help_window.showing(), None);
    }

    #[test]
    fn the_tree_marks_the_current_topic_entry_selected() {
        let dir = TestDir::new("studio_core_help_sel");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        // A selectable_label reports its selected state as a toggle; the open
        // chapter's first topic is selected, the rest are not.
        let toggled = |nodes: &[accesskit::Node], label: &str| {
            nodes
                .iter()
                .find(|node| node.role() == Role::Button && node.label() == Some(label))
                .map(|node| node.toggled())
        };
        assert_eq!(toggled(&nodes, "Beta topic"), Some(Some(Toggled::True)));
        assert_eq!(toggled(&nodes, "Beta more"), Some(Some(Toggled::False)));
        assert_eq!(toggled(&nodes, "Messages"), Some(Some(Toggled::False)));

        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(toggled(&nodes, "Beta topic"), Some(Some(Toggled::False)));
        assert_eq!(toggled(&nodes, "Beta more"), Some(Some(Toggled::False)));
        assert_eq!(toggled(&nodes, "Messages"), Some(Some(Toggled::True)));
    }

    // The window's title bar draws its title in the window-title style (the
    // drafts' `.help-win .title`): 14 points in the strong colour, where egui's
    // title bar falls back to `Heading`'s 15.
    #[test]
    fn the_help_windows_title_is_14_points_and_strong() {
        let dir = TestDir::new("studio_core_help_title");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let shapes = idle(&egui_ctx, &mut app, 15).shapes;
        let strong = egui_ctx
            .style_of(egui_ctx.theme())
            .visuals
            .strong_text_color();
        let titles: Vec<(f32, egui::Color32)> = shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.text() == "Help" => {
                    let format = &shape.galley.job.sections[0].format;
                    Some((format.font_id.size, format.color))
                }
                _ => None,
            })
            .collect();
        assert!(
            titles.contains(&(14.0, strong)),
            "the title among the texts \"Help\": {titles:?}"
        );
    }

    #[test]
    fn the_severity_color_matches_the_severity() {
        let visuals = egui::Visuals::dark();
        for severity in ["Error", "Fatal", "Error or Fatal", "Error or Warning"] {
            assert_eq!(
                severity_color(severity, &visuals),
                visuals.error_fg_color,
                "{severity}"
            );
        }
        assert_eq!(severity_color("Warning", &visuals), visuals.warn_fg_color);
        assert_eq!(severity_color("Info", &visuals), visuals.weak_text_color());
    }

    #[test]
    fn an_unknown_target_falls_back_to_the_active_tool_then_the_first_chapter() {
        let mut window = HelpWindow::new(sections());
        let target = |tool: &str| HelpTarget {
            tool: tool.to_owned(),
            topic: None,
            anchor: None,
        };
        window.open(&target("nope"), "b");
        assert_eq!(window.showing(), Some(("b", "b-one")));
        window.open(&target("nope"), "zz");
        assert_eq!(window.showing(), Some(("a", "a-one")));
    }

    /// A stub whose first topic is 60 one-line paragraphs: the first and the 55th
    /// (index 54) are findable by text.
    fn long_section() -> HelpSection {
        let body = "firstpara opener.\n\n".to_owned()
            + &"filler text.\n\n".repeat(54)
            + "zzdistinct target.\n\n"
            + &"filler text.\n\n".repeat(5);
        HelpSection {
            title: "Long",
            topics: vec![HelpTopic {
                slug: "long",
                title: "Long topic",
                body: Box::leak(body.into_boxed_str()),
            }],
            messages: Vec::new(),
        }
    }

    /// The topic pane's top edge: the y just under the search box.
    fn pane_top(nodes: &[accesskit::Node]) -> f64 {
        search_box(nodes).bounds().expect("search box bounds").y1
    }

    /// A `Label` node's bounds by its value.
    fn label_bounds<'a>(nodes: &'a [accesskit::Node], text: &str) -> Option<&'a accesskit::Node> {
        nodes.iter().find(|node| {
            node.role() == Role::Label && node.value().is_some_and(|v| v.contains(text))
        })
    }

    #[test]
    fn the_highlighted_paragraph_is_scrolled_into_view() {
        let dir = TestDir::new("studio_core_help_parascroll");
        let (a, calls_a) = stub_help("a", "Long", None, long_section());
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, search_box(&nodes));
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("zzdistinct".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        click(
            &egui_ctx,
            &mut app,
            entry(&nodes, "Long › Long topic › zzdistinct target."),
        );
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(
            app.help_window.highlight(),
            (Some(55), None),
            "the 55th paragraph is the highlight"
        );

        let window = help_window_node(&nodes)
            .expect("the window is drawn")
            .bounds()
            .unwrap();
        let target =
            label_bounds(&nodes, "zzdistinct target").expect("the highlighted paragraph's node");
        assert!(
            target.bounds().unwrap().y1 <= window.y1,
            "the paragraph is inside the window: {} <= {}",
            target.bounds().unwrap().y1,
            window.y1
        );
        // The first paragraph scrolled away above the pane's top.
        let first = label_bounds(&nodes, "firstpara opener");
        if let Some(first) = first {
            assert!(
                first.bounds().unwrap().y1 <= pane_top(&nodes),
                "the first paragraph is scrolled out of view: {} <= {}",
                first.bounds().unwrap().y1,
                pane_top(&nodes)
            );
        }
    }

    #[test]
    fn a_jump_scrolls_the_pane_to_its_top_only_once() {
        let dir = TestDir::new("studio_core_help_scrolltop");
        let (a, calls_a) = stub_help("a", "Long", None, long_section());
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        // The first paragraph is in view at the top.
        let first = label_bounds(&nodes, "firstpara opener").expect("first paragraph");
        assert!(first.bounds().unwrap().y0 >= pane_top(&nodes));

        // Wheel down over the topic pane: the first paragraph leaves the view and
        // stays gone (no snap back).
        let pane_pos = centre(help_window_node(&nodes).expect("the window is drawn"));
        frame(
            &egui_ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pane_pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -200.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::default(),
                    source: egui::MouseWheelSource::Wheel,
                },
            ],
            true,
        );
        // egui smooths wheel input across frames: the queued delta drains over a few.
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        let first = label_bounds(&nodes, "firstpara opener");
        assert!(
            first.is_none() || first.unwrap().bounds().unwrap().y1 <= pane_top(&nodes),
            "the first paragraph is scrolled out of view and stays gone"
        );

        // A topic jump scrolls the pane back to its top.
        click(&egui_ctx, &mut app, entry(&nodes, "Long topic"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        let first = label_bounds(&nodes, "firstpara opener").expect("first paragraph");
        assert!(
            first.bounds().unwrap().y0 >= pane_top(&nodes) - 1.0,
            "the jump brought the first paragraph back into view: {} >= {}",
            first.bounds().unwrap().y0,
            pane_top(&nodes)
        );
    }

    #[test]
    fn an_openhelp_anchor_scrolls_the_catalog_row_into_view() {
        let dir = TestDir::new("studio_core_help_rowscroll");
        let mut section = beta_section();
        section.messages = (0..60)
            .map(|index| CatalogRow {
                code: Box::leak(format!("row_{index:02}").into_boxed_str()),
                severity: "Info",
                when: "when it happens",
                effect: "what the tool does",
            })
            .collect();
        let (b, calls_b) = stub_help("b", "Beta", None, section);
        let Shell { mut app, ctx, .. } = shell_with(
            test_settings(dir.path()),
            vec![b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_b],
        );
        let egui_ctx = test_context();

        ctx.notify(Notice {
            text: "open the help".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Open help".to_owned(),
                effect: ActionEffect::OpenHelp(HelpTarget {
                    tool: "b".to_owned(),
                    topic: Some("messages".to_owned()),
                    anchor: Some("row_30".to_owned()),
                }),
            }),
        });
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Open help"));
        let nodes = idle(&egui_ctx, &mut app, 20).nodes;
        assert_eq!(app.help_window.showing(), Some(("b", "messages")));
        assert_eq!(app.help_window.highlight(), (None, Some("row_30")));

        let window = help_window_node(&nodes)
            .expect("the window is drawn")
            .bounds()
            .unwrap();
        let row = label_bounds(&nodes, "row_30").expect("the row's code node");
        assert!(
            row.bounds().unwrap().y0 >= window.y0 && row.bounds().unwrap().y1 <= window.y1,
            "the anchored row is inside the window: {row:?}"
        );
        // A row scrolled away keeps a node but its bounds leave the window's span.
        for code in ["row_00", "row_59"] {
            let bounds = label_bounds(&nodes, code).and_then(|node| node.bounds());
            assert!(
                bounds.is_none_or(|rect| rect.y1 <= window.y0 || rect.y0 >= window.y1),
                "{code} is scrolled out of the window's vertical span: {bounds:?} vs {window:?}"
            );
        }
    }

    #[test]
    fn a_row_jump_scrolls_only_once() {
        let dir = TestDir::new("studio_core_help_rowonce");
        let mut section = beta_section();
        section.messages = (0..60)
            .map(|index| CatalogRow {
                code: Box::leak(format!("row_{index:02}").into_boxed_str()),
                severity: "Info",
                when: "when it happens",
                effect: "what the tool does",
            })
            .collect();
        let (b, calls_b) = stub_help("b", "Beta", None, section);
        let Shell { mut app, ctx, .. } = shell_with(
            test_settings(dir.path()),
            vec![b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_b],
        );
        let egui_ctx = test_context();

        ctx.notify(Notice {
            text: "open the help".to_owned(),
            kind: NoticeKind::Info,
            action: Some(NoticeAction {
                label: "Open help".to_owned(),
                effect: ActionEffect::OpenHelp(HelpTarget {
                    tool: "b".to_owned(),
                    topic: Some("messages".to_owned()),
                    anchor: Some("row_30".to_owned()),
                }),
            }),
        });
        let nodes = idle(&egui_ctx, &mut app, 2).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Open help"));
        let nodes = idle(&egui_ctx, &mut app, 20).nodes;
        let before = label_bounds(&nodes, "row_30")
            .expect("the anchored row")
            .bounds()
            .unwrap()
            .y0;

        // A member can scroll away from the anchored row: wheel down and it moves or
        // leaves the view.
        let pane_pos = centre(help_window_node(&nodes).expect("the window is drawn"));
        frame(
            &egui_ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pane_pos),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -400.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::default(),
                    source: egui::MouseWheelSource::Wheel,
                },
            ],
            true,
        );
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        let moved = match label_bounds(&nodes, "row_30") {
            Some(node) => before - node.bounds().unwrap().y0,
            None => f64::MAX,
        };
        assert!(
            moved >= 100.0,
            "the member scrolled 400 points: row_30 moved {moved:.1} up or out"
        );
    }

    #[test]
    fn a_when_cell_link_jumps_to_its_target() {
        let dir = TestDir::new("studio_core_help_celllink");
        let mut alpha = alpha_section();
        alpha.messages[0].when = "see [the other topic](b#beta-topic) for it";
        let (a, calls_a) = stub_help("a", "Alpha", None, alpha);
        let (b, calls_b) = stub_help("b", "Beta", None, beta_section());
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a, b],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a, calls_b],
        );
        let egui_ctx = test_context();

        // Alpha's chapter opens on `alpha-start`; click its Messages entry, then the
        // link in the first row's When cell.
        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(app.help_window.showing(), Some(("a", "alpha-start")));
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "the other topic"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert_eq!(
            app.help_window.showing(),
            Some(("b", "beta-topic")),
            "the When cell's link jumped to Beta's topic"
        );
        assert!(has_text(&nodes, "Beta does things."));
    }

    #[test]
    fn a_tree_click_during_search_shows_the_topic() {
        let dir = TestDir::new("studio_core_help_treeselect");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, search_box(&nodes));
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::Text("alpha".to_owned())],
            true,
        );
        let nodes = frame(&egui_ctx, &mut app, Vec::new(), true).nodes;
        assert!(
            has_label(
                &nodes,
                "Alpha › Alpha start › Alpha keeps its TEAMS in a folder."
            ),
            "the results list is up"
        );

        // A click on another topic's tree entry (Beta's open chapter) jumps to it
        // and clears the box.
        click(&egui_ctx, &mut app, entry(&nodes, "Beta more"));
        idle(&egui_ctx, &mut app, 15);
        assert!(
            app.help_window.search_text().is_empty(),
            "the tree click cleared the search box"
        );
        assert_eq!(
            app.help_window.showing(),
            Some(("b", "beta-more")),
            "the pane shows the clicked topic"
        );
    }

    #[test]
    fn the_text_column_budget_splits_what_is_left_after_the_short_columns() {
        // A 720-point window's pane yields roughly these inputs: the When and
        // Effect columns split 500 - 80 - 50 - 32 = 338, each 169.
        assert_eq!(
            text_column_width(500.0, 80.0, 50.0, 32.0),
            169.0,
            "(500 - 80 - 50 - 32) / 2"
        );
        // At the floor: the remaining budget goes under 30 points per column and
        // clamps there — (150 - 80 - 50 - 32) is negative, so 60 / 2 = 30.
        assert_eq!(text_column_width(150.0, 80.0, 50.0, 32.0), 30.0, "60 / 2");
    }

    #[test]
    fn the_catalog_grid_stays_inside_the_panes_width() {
        // AccessKit clips node bounds to the window, so a too-wide grid cannot be
        // caught from the drawn nodes; `ui.min_rect()` reads what the grid laid out.
        let long = Box::leak(
            "a condition long enough to wrap onto several lines when the column is narrow "
                .repeat(4)
                .into_boxed_str(),
        );
        let mut section = alpha_section();
        for row in &mut section.messages {
            row.when = long;
            row.effect = long;
        }
        let mut cache = CommonMarkCache::default();
        let egui_ctx = test_context();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(500.0, 600.0),
            )),
            ..Default::default()
        };
        let mut width = 0.0;
        for _ in 0..2 {
            egui_ctx
                .run_ui(input(), |ui| {
                    messages(ui, &mut cache, &section, None, false);
                    width = ui.min_rect().width();
                })
                .drop_without_applying_deltas();
        }
        assert!(
            width <= 500.0,
            "the catalog grid is {width} wide on a 500-point pane"
        );
    }

    #[test]
    fn a_wide_code_is_not_wrapped() {
        let dir = TestDir::new("studio_core_help_codewidth");
        let mut section = alpha_section();
        section.messages[0].code = "a_very_long_message_code_with_forty_characters_x";
        let (a, calls_a) = stub_help("a", "Alpha", None, section);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        let code = label_bounds(&nodes, "a_very_long_message_code_with_forty_characters_x")
            .expect("the code node");
        assert!(
            code.bounds().unwrap().height() < 20.0,
            "the code is one line tall: {}",
            code.bounds().unwrap().height()
        );
    }

    #[test]
    fn catalog_cells_render_markdown_not_its_source() {
        let dir = TestDir::new("studio_core_help_markup");
        let mut section = alpha_section();
        section.messages[0].when = "has **bold** and `code` inside";
        let (a, calls_a) = stub_help("a", "Alpha", None, section);
        let Shell { mut app, .. } = shell_with(
            test_settings(dir.path()),
            vec![a],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls_a],
        );
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        click(&egui_ctx, &mut app, entry(&nodes, "Messages"));
        let nodes = idle(&egui_ctx, &mut app, 15).nodes;
        assert!(has_text(&nodes, "bold"), "the strong text is drawn");
        assert!(has_text(&nodes, "code"), "the code span is drawn");
        assert!(
            !has_text(&nodes, "**"),
            "the markup is rendered, not shown: {nodes:?}"
        );
        assert!(!has_text(&nodes, "`"), "no backtick reaches the screen");
    }

    // A fixed-width panel shows no resize edge (the draft's plain edge): the tree
    // column is `exact_size(220)`, so resting the pointer on its right edge leaves
    // the default cursor and registers no splitter.
    #[test]
    fn the_tree_panels_fixed_width_edge_shows_no_resize_cursor() {
        let dir = TestDir::new("studio_core_help_tree_edge");
        let Shell { mut app, .. } = help_shell(dir.path());
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let out = idle(&egui_ctx, &mut app, 15);

        // The panel's right edge: the window's content left (the search box spans
        // the content width, so its own left edge is the panel's) plus the panel's
        // exact 220 points; the height is inside the window, below the tree's
        // entries and above the window's own bottom edge.
        let content_left = search_box(&out.nodes)
            .bounds()
            .expect("the search box carries bounds")
            .x0;
        let window_bottom = help_window_node(&out.nodes)
            .expect("the help window is drawn")
            .bounds()
            .expect("the window carries bounds")
            .y1;
        let edge = egui::pos2((content_left + 220.0) as f32, (window_bottom - 40.0) as f32);
        frame(
            &egui_ctx,
            &mut app,
            vec![egui::Event::PointerMoved(edge)],
            true,
        );
        let out = idle(&egui_ctx, &mut app, 3);
        assert_eq!(
            out.platform.cursor_icon,
            egui::CursorIcon::Default,
            "the fixed-width edge is not a resize handle"
        );
        // The resizable window's own edges register splitters too ("Resize window
        // …"); the panel's is the one labeled "Resize panel".
        assert!(
            !out.nodes
                .iter()
                .any(|node| node.role() == Role::Splitter && node.label() == Some("Resize panel")),
            "a fixed-width panel registers no splitter"
        );
    }

    /// A chapter carrying the longest topic name the tools have (each tool's
    /// `Running from a terminal`), under the Save editor's own title.
    fn terminal_section() -> HelpSection {
        HelpSection {
            title: "Save editor",
            topics: vec![HelpTopic {
                slug: "command-line",
                title: "Running from a terminal",
                body: "Run it from a terminal.\n",
            }],
            messages: Vec::new(),
        }
    }

    // The tree is 220 points wide, its padding included (the drafts'
    // `.help-tree`), so the longest chapter and topic names show whole: the
    // panel's separator line stands 220 right of the content's left edge,
    // and both names end left of it.
    #[test]
    fn the_tree_is_220_points_wide_and_holds_the_longest_names() {
        let dir = TestDir::new("studio_core_help_tree_width");
        let (tool, calls) = stub_help("save-editor", "Save editor", None, terminal_section());
        let shell = shell_with(
            test_settings(dir.path()),
            vec![tool],
            &[],
            Some(dir.path().to_path_buf()),
            vec![calls],
        );
        shell.ctx.switch_to_tool("save-editor");
        let Shell { mut app, .. } = shell;
        let egui_ctx = test_context();

        frame(&egui_ctx, &mut app, vec![key(egui::Key::F1, true)], true);
        let out = idle(&egui_ctx, &mut app, 15);
        let search = search_box(&out.nodes)
            .bounds()
            .expect("the search box carries bounds");
        let content_left = search.x0;
        let separator = egui_ctx
            .style_of(egui_ctx.theme())
            .visuals
            .widgets
            .noninteractive
            .bg_stroke;
        // The vertical lines in the separator's stroke under the search box:
        // the shell's sidebar edge spans the whole window, above it, and an
        // open chapter's indent line is shorter than the separator, which
        // spans the panel's height.
        let edge = out
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke }
                    if *stroke == separator
                        && points[0].x == points[1].x
                        && f64::from(points[0].y.min(points[1].y)) > search.y1 =>
                {
                    Some((points[0].x, (points[1].y - points[0].y).abs()))
                }
                _ => None,
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(x, _)| f64::from(x))
            .expect("the tree panel's separator line");
        for (role, label) in [
            (Role::DisclosureTriangle, "First run and data location"),
            (Role::Button, "Running from a terminal"),
        ] {
            let bounds = out
                .nodes
                .iter()
                .find(|node| node.role() == role && node.label() == Some(label))
                .and_then(accesskit::Node::bounds)
                .unwrap_or_else(|| panic!("no tree entry {label:?}"));
            assert!(
                bounds.x0 >= content_left && bounds.x1 <= edge,
                "{label:?} ends at {} past the tree's edge at {edge}",
                bounds.x1
            );
        }
        assert!(
            (edge - content_left - 220.0).abs() <= 1.0,
            "the tree's width: {}",
            edge - content_left
        );
    }
}
