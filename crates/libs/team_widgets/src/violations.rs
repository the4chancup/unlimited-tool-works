//! The AATF results list, drafted in `docs/prototypes/tools/save_editor.js`
//! (`liveList`, `violationSentence`, `violationDetail`, `playerHead`): one team's check grouped
//! per player, a row's click reporting the player to select. Live AATF's
//! dock and its window, Multi-team AATF's report and the Team creator
//! (planned) draw this list.

use pes_savefile::interchange::team_toml::labels::{card_name, stat_name, title_case};
use pes_version::PesVersion;

/// The failing glyph (`✖`, U+2716): the suite's symbols font holds it.
/// The icon font's subset does not.
pub const CROSS: char = '\u{2716}';

/// The gap between a Live AATF field mark's box and the label's text it
/// stands before: one figure so every mark's spacing agrees.
pub const MARK_GAP: f32 = 2.0;

/// A rostered player as the list's group header shows him.
pub struct ListedPlayer<'a> {
    /// The roster slot.
    pub slot: u8,
    /// The player id.
    pub id: u32,
    /// The shirt number, as wide as `pes_savefile`'s `RosterSlot::number` (PES 19 on stores
    /// numbers past 99).
    pub number: u16,
    /// The name as the save stores it: the host passes the raw name (colour
    /// codes included), the header and a height row's lines draw it code-free.
    pub name: &'a str,
    /// His own height in cm: a height row's line names it after him when the check read
    /// another (his height less an allowance).
    pub height: u8,
}

/// One team's check: its players by slot and `aatf::Rules::check_team`'s violations, in the
/// order the check returned them.
pub struct TeamViolations<'a> {
    /// The team id.
    pub team_id: u32,
    /// The team name.
    pub team_name: &'a str,
    /// The rostered players, for the group headers and the height rows' lines.
    pub players: &'a [ListedPlayer<'a>],
    /// The violations `check_team` returned, in its order.
    pub violations: &'a [aatf::Violation],
    /// The roster slots of the team's starting eleven. Under
    /// `ListUse::Editing` in two columns the players' groups stand by it,
    /// the starters' left and every other slot's right; one column and
    /// `ListUse::Report` list them in roster order.
    pub starting_eleven: &'a [u8],
}

/// The player a row's click selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedPlayer {
    /// The team the player is in.
    pub team_id: u32,
    /// His roster slot.
    pub slot: u8,
    /// The `FieldedNotA` violation's own fields when the row's kind is
    /// it (the click selects his team's row on the named preset's first
    /// fielded formation holding the position); `None` otherwise.
    pub fielded: Option<FieldedAt>,
}

/// Where a `FieldedNotA` row's click takes the team view: the
/// violation's own fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldedAt {
    /// The named preset (1 to 3).
    pub preset: u8,
    /// The position the violation fields him at.
    pub position: aatf::Position,
}

/// What a host lists the results for, which decides what the list draws
/// besides the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListUse {
    /// The team a member is making legal (the Live AATF dock): a team with
    /// no rows gets no header, since the host's own header says it passes;
    /// a player's row selects him on click; the suggestions follow under
    /// their own heading when `show_suggestions`.
    Editing {
        /// List the suggestions too.
        show_suggestions: bool,
        /// List the players' groups in one column, in roster order, with the
        /// rule's column `NARROW_RULE_COL` wide (Live AATF in its own window,
        /// narrower than the dock); otherwise in two columns, starters and
        /// bench, the rule's column `RULE_COL` wide.
        one_column: bool,
    },
    /// A report over several teams, read and copied out (Multi-team
    /// AATF): every team under its id and name, `All clean` under one with
    /// no rows so it reads as checked rather than missed; the rows only
    /// report, and no suggestion is listed.
    Report,
}

impl ListUse {
    /// The width of the rule name's column in this list's rows.
    fn rule_column(self) -> f32 {
        match self {
            ListUse::Editing {
                one_column: true, ..
            } => NARROW_RULE_COL,
            ListUse::Editing {
                one_column: false, ..
            }
            | ListUse::Report => RULE_COL,
        }
    }
}

/// What the rows name things from: the rules a check ran under, whose
/// labels name its tiers, brackets and height systems, and the PES version
/// of the save it checked, whose game names the stats (PES 2017's Header).
#[derive(Clone, Copy)]
pub struct Wording<'a> {
    /// The rules the check ran under.
    pub ruleset: &'a aatf::Ruleset,
    /// The checked save's PES version.
    pub version: PesVersion,
}

/// The AATF results of `teams`, worded as `wording` says (the rules they
/// were checked against and the checked save's version), grouped per
/// player, as `list_use` says, the team's own rows first. Under
/// `ListUse::Editing` each part (the violations, then the suggestions)
/// stands the players' groups under the team's own rows in two columns,
/// the starting eleven's left and the bench's right, unless `one_column`
/// asks for one; a report lists every group in one column.
/// Returns the player whose row was clicked this frame.
pub fn violations_list(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    teams: &[TeamViolations<'_>],
    wording: Wording<'_>,
    list_use: ListUse,
) -> Option<SelectedPlayer> {
    ui.push_id(id_salt, |ui| {
        let mut selected = None;
        for team in teams {
            let groups = issue_groups(team.violations);
            match list_use {
                ListUse::Editing { .. } => {
                    // A team the check found nothing wrong with gets no header of its own.
                    if groups.is_empty() {
                        continue;
                    }
                    ui.strong(format!(
                        "Team {} {}",
                        team.team_id,
                        pes_savefile::model::names::display_name(team.team_name)
                    ));
                }
                ListUse::Report => {
                    ui.strong(format!(
                        "{} {}",
                        team.team_id,
                        pes_savefile::model::names::display_name(team.team_name)
                    ));
                    if groups.is_empty() {
                        ui.weak("All clean");
                    }
                }
            }
            if let Some(player) = part(ui, team, &groups, wording, list_use) {
                selected = Some(player);
            }
        }
        let any_suggestions = teams
            .iter()
            .flat_map(|team| team.violations)
            .any(|v| v.severity == aatf::Severity::Suggestion);
        let suggestions_asked = matches!(
            list_use,
            ListUse::Editing {
                show_suggestions: true,
                ..
            }
        );
        if suggestions_asked && any_suggestions {
            ui.strong("Suggestions");
            for team in teams {
                let groups = suggestion_groups(team.violations);
                if let Some(player) = part(ui, team, &groups, wording, list_use) {
                    selected = Some(player);
                }
            }
        }
        selected
    })
    .inner
}

/// The gap between the two columns of player groups, the draft's
/// `column-gap`.
const COLUMN_GAP: f32 = 16.0;

/// One part of `team`'s list (its violations, or its suggestions), drawn as
/// `list_use` says: while editing, the team's own group first at the full
/// width, then the players' groups in two top-aligned columns of equal
/// width, the starting eleven's left and every other slot's right, each in
/// `groups`' order; a player's rows are short and the dock is wide.
/// Editing in one column, the team's own group, then the players' in
/// `groups`' order. A report reads team by team, so it keeps every group in
/// one column in `groups`' order (the team's own first), its rows only
/// reporting. Returns the player whose row was clicked.
fn part(
    ui: &mut egui::Ui,
    team: &TeamViolations<'_>,
    groups: &[(Option<u8>, Vec<&aatf::Violation>)],
    wording: Wording<'_>,
    list_use: ListUse,
) -> Option<SelectedPlayer> {
    let rule_col = list_use.rule_column();
    let one_column = match list_use {
        ListUse::Report => return draw_groups(ui, team, groups.iter(), wording, false, rule_col),
        ListUse::Editing { one_column, .. } => one_column,
    };
    let column_groups = |starting: bool| {
        groups.iter().filter(move |(slot, _)| {
            slot.is_some_and(|slot| team.starting_eleven.contains(&slot) == starting)
        })
    };
    let own = groups.iter().filter(|(slot, _)| slot.is_none());
    let mut selected = draw_groups(ui, team, own, wording, true, rule_col);
    if one_column {
        let players = groups.iter().filter(|(slot, _)| slot.is_some());
        selected = draw_groups(ui, team, players, wording, true, rule_col).or(selected);
    } else if groups.iter().any(|(slot, _)| slot.is_some()) {
        let spacing = ui.spacing().item_spacing.x;
        ui.scope(|ui| {
            // `columns_const` spaces its columns by the item spacing; each
            // column's own content keeps the list's.
            ui.spacing_mut().item_spacing.x = COLUMN_GAP;
            ui.columns_const(|[left, right]| {
                for (column, starting) in [(left, true), (right, false)] {
                    column.spacing_mut().item_spacing.x = spacing;
                    if let Some(player) = draw_groups(
                        column,
                        team,
                        column_groups(starting),
                        wording,
                        true,
                        rule_col,
                    ) {
                        selected = Some(player);
                    }
                }
            });
        });
    }
    selected
}

/// `groups` drawn one after the other, each its header then its rows.
/// Returns the player whose row was clicked.
fn draw_groups<'v>(
    ui: &mut egui::Ui,
    team: &TeamViolations<'_>,
    groups: impl Iterator<Item = &'v (Option<u8>, Vec<&'v aatf::Violation>)>,
    wording: Wording<'_>,
    selects: bool,
    rule_col: f32,
) -> Option<SelectedPlayer> {
    let mut selected = None;
    for (slot, violations) in groups {
        group_header(ui, team, *slot);
        for violation in violations {
            let listed = row(
                ui,
                team.team_id,
                team.players,
                violation,
                wording,
                selects,
                rule_col,
            );
            if let Some(player) = listed {
                selected = Some(player);
            }
        }
    }
    selected
}

/// A team's Suggestion violations in the list's order: one group per
/// slot, the team's own first, as `issue_groups` groups the others.
fn suggestion_groups(violations: &[aatf::Violation]) -> Vec<(Option<u8>, Vec<&aatf::Violation>)> {
    let suggestions = || {
        violations
            .iter()
            .filter(|v| v.severity == aatf::Severity::Suggestion)
    };
    slot_order(suggestions())
        .into_iter()
        .map(|slot| (slot, suggestions().filter(|v| v.slot == slot).collect()))
        .collect()
}

/// A team's Error and Warning violations in the list's order: one group
/// per slot, the team's own rows (`None`) first, then the players' by slot,
/// errors before warnings inside a group. A host writing the list out as
/// text walks this, so the text reads as the list does.
pub fn issue_groups(violations: &[aatf::Violation]) -> Vec<(Option<u8>, Vec<&aatf::Violation>)> {
    slot_order(
        violations
            .iter()
            .filter(|v| v.severity != aatf::Severity::Suggestion),
    )
    .into_iter()
    .map(|slot| {
        let rows = [aatf::Severity::Error, aatf::Severity::Warning]
            .into_iter()
            .flat_map(|severity| {
                violations
                    .iter()
                    .filter(move |v| v.slot == slot && v.severity == severity)
            })
            .collect();
        (slot, rows)
    })
    .collect()
}

/// The slots the given violations fall on: the team's own (`None`) first,
/// then the players' in first-appearance order (the check's, by slot). A
/// team-wide issue (a formation, a tier count) is the team's first concern,
/// ahead of any one player's, so every list of a team's issues leads with
/// it, where the check returns it last.
fn slot_order<'a>(violations: impl Iterator<Item = &'a aatf::Violation>) -> Vec<Option<u8>> {
    let mut order = Vec::new();
    for violation in violations {
        if !order.contains(&violation.slot) {
            order.push(violation.slot);
        }
    }
    // Stable: the players keep their order.
    order.sort_by_key(Option::is_some);
    order
}

/// One player's head as the groups draw it: `Player {id}  {number} {name}`
/// with the name code-free (`name` is the save's raw string). The Save
/// editor's comparator lists its players under the same head.
pub fn player_head(id: u32, number: u16, name: &str) -> String {
    format!(
        "Player {}  {} {}",
        id,
        number,
        pes_savefile::model::names::display_name(name)
    )
}

/// A violation's whole line, `rule_name`, a colon, a space, then
/// `sentence` (`Height: height 185 takes no bracket`): the row's
/// accessible label, and the line a Live AATF cross's tooltip shows so
/// both read the same.
pub fn violation_line(violation: &aatf::Violation, team_id: u32, wording: Wording<'_>) -> String {
    format!(
        "{}: {}",
        rule_name(&violation.kind),
        sentence(violation, team_id, wording)
    )
}

/// The lines under a height row naming its players, the draft's
/// `violationDetail`. A `BracketQuota`'s players are grouped by the height
/// the check read, tallest first (`At 185: Stack Bros. (189), HELLOHELL`), a
/// player whose own height differs from it with his own after his name. An
/// `ExtraHeight`'s draws are grouped by bracket and centimetres, most first
/// (`6 in Tall slots, 10 cm each (60): …`), the bracket by its label in
/// `wording.ruleset`. Inside a line the players stand in roster order, named
/// code-free from `players` (the team's); a slot none of them holds reads
/// `Slot {slot}`, as the group headers name it. Every other kind has none.
pub fn violation_detail(
    violation: &aatf::Violation,
    players: &[ListedPlayer<'_>],
    wording: Wording<'_>,
) -> Vec<String> {
    let player = |slot: u8| players.iter().find(|player| player.slot == slot);
    let name = |slot: u8| {
        player(slot).map_or_else(
            || format!("Slot {slot}"),
            |player| pes_savefile::model::names::display_name(player.name),
        )
    };
    use aatf::ViolationKind as K;
    match &violation.kind {
        K::BracketQuota { players: read, .. } => {
            let mut read = read.clone();
            read.sort_by_key(|&(slot, cm)| (std::cmp::Reverse(cm), slot));
            grouped(read.iter().map(|&(slot, cm)| (cm, slot)))
                .into_iter()
                .map(|(cm, slots)| {
                    let names: Vec<String> = slots
                        .into_iter()
                        .map(|slot| match player(slot) {
                            Some(own) if own.height != cm => {
                                format!("{} ({})", name(slot), own.height)
                            }
                            _ => name(slot),
                        })
                        .collect();
                    format!("At {cm}: {}", names.join(", "))
                })
                .collect()
        }
        K::ExtraHeight { draws, .. } => {
            let mut draws = draws.clone();
            draws.sort_by_key(|&(slot, _, cm)| (std::cmp::Reverse(cm), slot));
            grouped(
                draws
                    .iter()
                    .map(|(slot, bracket, cm)| ((bracket.as_str(), *cm), *slot)),
            )
            .into_iter()
            .map(|((bracket, cm), slots)| {
                let names: Vec<String> = slots.iter().map(|&slot| name(slot)).collect();
                format!(
                    "{} in {} slots, {cm} cm each ({}): {}",
                    slots.len(),
                    bracket_label(wording.ruleset, bracket),
                    slots.len() * usize::from(cm),
                    names.join(", ")
                )
            })
            .collect()
        }
        K::RegisteredNotA { .. }
        | K::PositionRange
        | K::FieldedNotA { .. }
        | K::BRating { .. }
        | K::GkSecondA { .. }
        | K::Age { .. }
        | K::Weight { .. }
        | K::PlayingStyleRange
        | K::NoTier { .. }
        | K::Stat { .. }
        | K::StatTotal { .. }
        | K::Form { .. }
        | K::InjuryResistance { .. }
        | K::WeakFootUsage { .. }
        | K::WeakFootAccuracy { .. }
        | K::NameColor { .. }
        | K::Cards { .. }
        | K::MinCards { .. }
        | K::SkillCardCap { .. }
        | K::ComStyleCap { .. }
        | K::RequiredCard { .. }
        | K::APositions { .. }
        | K::Height { .. }
        | K::TierCount { .. }
        | K::MedalSystem { .. }
        | K::NearestSystem { .. }
        | K::Captain
        | K::Goalkeepers { .. }
        | K::TeamSetting { .. }
        | K::FormationShape { .. }
        | K::Special { .. }
        | K::Custom { .. }
        | K::Unused { .. }
        | K::UnusedChoice { .. }
        | K::RosterOrder { .. } => Vec::new(),
    }
}

/// `items`' slots grouped by their key: the groups in the order their keys
/// first appear, each group's slots in `items`' order.
fn grouped<K: PartialEq>(items: impl Iterator<Item = (K, u8)>) -> Vec<(K, Vec<u8>)> {
    let mut groups: Vec<(K, Vec<u8>)> = Vec::new();
    for (key, slot) in items {
        match groups.iter_mut().find(|(group, _)| *group == key) {
            Some((_, slots)) => slots.push(slot),
            None => groups.push((key, vec![slot])),
        }
    }
    groups
}

/// The rule a violation breaks, named in words (`Fielded position`, `Tier
/// count`): the row's left column and the start of its line. A rules file
/// names the kind otherwise (`FieldedNotA`, `ViolationKind::rule`), a
/// spelling for the program alone.
pub fn rule_name(kind: &aatf::ViolationKind) -> &'static str {
    use aatf::ViolationKind as K;
    match kind {
        K::RegisteredNotA { .. } | K::PositionRange => "Registered position",
        K::FieldedNotA { .. } => "Fielded position",
        K::BRating { .. } => "B rating",
        K::GkSecondA { .. } => "GK rating",
        K::Age { .. } => "Age",
        K::Weight { .. } => "Weight",
        K::PlayingStyleRange => "Playing style",
        K::NoTier { .. } => "Tier",
        K::Stat { .. } => "Stat",
        K::StatTotal { .. } => "Stat total",
        K::Form { .. } => "Form",
        K::InjuryResistance { .. } => "Injury Resistance",
        K::WeakFootUsage { .. } => "Weak Foot Usage",
        K::WeakFootAccuracy { .. } => "Weak Foot Accuracy",
        K::NameColor { .. } => "Name colour",
        K::Cards { .. } | K::MinCards { .. } => "Cards",
        K::SkillCardCap { .. } => "Skill cards",
        K::ComStyleCap { .. } => "COM styles",
        K::RequiredCard { .. } => "Required card",
        K::APositions { .. } => "A positions",
        K::Height { .. } => "Height",
        K::TierCount { .. } => "Tier count",
        K::BracketQuota { .. } => "Height quota",
        K::MedalSystem { .. } => "Medal system",
        K::ExtraHeight { .. } => "Extra height",
        K::NearestSystem { .. } => "Nearest system",
        K::Captain => "Captain",
        K::Goalkeepers { .. } => "Goalkeepers",
        K::TeamSetting { .. } => "Team setting",
        K::FormationShape { .. } => "Formation",
        K::Special { .. } => "Special rule",
        K::Custom { .. } => "Custom check",
        K::Unused { .. } => "Unused",
        K::UnusedChoice { .. } => "Unused choice",
        K::RosterOrder { .. } => "Roster order",
    }
}

/// One group's header: `Player {id}  {number} {name}` for a rostered player,
/// `Slot {slot}` for one the host lists no player for, `Team-wide issues`
/// for the team's own rows.
fn group_header(ui: &mut egui::Ui, team: &TeamViolations<'_>, slot: Option<u8>) {
    let text = match slot {
        None => "Team-wide issues".to_string(),
        Some(slot) => match team.players.iter().find(|p| p.slot == slot) {
            Some(player) => player_head(player.id, player.number, player.name),
            None => format!("Slot {slot}"),
        },
    };
    ui.weak(text);
}

/// The tooltip of a row whose click switches the view to its player: the
/// list's player rows, the Save editor's comparator rows and its patch
/// preview's items, so every such row says the same.
pub const SWITCH_TOOLTIP: &str = "Click to switch to this player";

/// The rule name's column, the draft's `.f`: 190 points of proportional text,
/// cut with an ellipsis when longer, never wrapping.
const RULE_COL: f32 = 190.0;

/// The rule name's column in a one-column list (Live AATF's own window): 140
/// points, since the window is narrow and the sentence beside it needs the
/// room. The longest rule name, `Weak Foot Accuracy`, draws about 114 points
/// wide, so none is cut.
const NARROW_RULE_COL: f32 = 140.0;

/// `text` laid out in `font`/`color`, wrapped as `wrap` says.
fn layout(
    ui: &egui::Ui,
    text: String,
    font: egui::FontId,
    color: egui::Color32,
    wrap: egui::text::TextWrapping,
) -> std::sync::Arc<egui::Galley> {
    let mut job =
        egui::text::LayoutJob::single_section(text, egui::TextFormat::simple(font, color));
    job.wrap = wrap;
    ui.fonts_mut(|f| f.layout_job(job))
}

/// One row, the draft's `.drow`: the rule's name in `rule_col` points of
/// severity-colored proportional text, one line cut with an ellipsis (the
/// draft's `text-overflow: ellipsis`), then `sentence` in the strong text
/// color, wrapped so a long custom check's text stays readable, and under it
/// `violation_detail`'s lines (the draft's `.drow.detail`) over `players`, the
/// team's, in the plain text color, wrapped in the same column; the whole row
/// one click target. While `selects`, a player's row returns
/// `SelectedPlayer` on click and its tooltip is `SWITCH_TOOLTIP`; a team
/// row, or any row of a list that does not select, only hovers, with no
/// tooltip.
fn row(
    ui: &mut egui::Ui,
    team_id: u32,
    players: &[ListedPlayer<'_>],
    violation: &aatf::Violation,
    wording: Wording<'_>,
    selects: bool,
    rule_col: f32,
) -> Option<SelectedPlayer> {
    let (rule_color, strong_color, text_color) = {
        let visuals = ui.visuals();
        (
            match violation.severity {
                aatf::Severity::Error => visuals.error_fg_color,
                aatf::Severity::Warning => visuals.warn_fg_color,
                aatf::Severity::Suggestion => visuals.text_color(),
            },
            visuals.strong_text_color(),
            visuals.text_color(),
        )
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let rule_galley = layout(
        ui,
        rule_name(&violation.kind).to_string(),
        font.clone(),
        rule_color,
        egui::text::TextWrapping::truncate_at_width(rule_col),
    );
    let column = (ui.available_width() - rule_col).max(0.0);
    let sentence_galley = layout(
        ui,
        sentence(violation, team_id, wording),
        font.clone(),
        strong_color,
        egui::text::TextWrapping::wrap_at_width(column),
    );
    let detail = violation_detail(violation, players, wording);
    let detail_galley = (!detail.is_empty()).then(|| {
        layout(
            ui,
            detail.join("\n"),
            font,
            text_color,
            egui::text::TextWrapping::wrap_at_width(column),
        )
    });
    let sentence_height = sentence_galley.size().y;
    let detail_height = detail_galley.as_ref().map_or(0.0, |galley| galley.size().y);
    let height = rule_galley.size().y.max(sentence_height + detail_height);
    let fielded = if let aatf::ViolationKind::FieldedNotA {
        preset, position, ..
    } = violation.kind
    {
        Some(FieldedAt { preset, position })
    } else {
        None
    };
    let select = violation
        .slot
        .filter(|_| selects)
        .map(|slot| SelectedPlayer {
            team_id,
            slot,
            fielded,
        });
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        if select.is_some() {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let mut label = violation_line(violation, team_id, wording);
    for line in &detail {
        label.push_str("; ");
        label.push_str(line);
    }
    // Only a row that switches players says so. A team-wide row selects
    // nothing and its group's heading already says what it is, and a
    // report's rows select nothing either.
    let hover = select.map(|_| SWITCH_TOOLTIP);
    let mut shapes = vec![
        egui::Shape::galley(rect.min, rule_galley, rule_color),
        egui::Shape::galley(
            rect.min + egui::vec2(rule_col, 0.0),
            sentence_galley,
            strong_color,
        ),
    ];
    if let Some(galley) = detail_galley {
        shapes.push(egui::Shape::galley(
            rect.min + egui::vec2(rule_col, sentence_height),
            galley,
            text_color,
        ));
    }
    row_paint(ui, response, &shapes, &label, select, hover)
}

/// The paint and click tail the violations list's rows and the Save
/// editor's comparator lines share: `shapes` painted in order, a hovered
/// row outlined in the hovered widget's stroke inside its rect and never
/// filled — a fill under the row's coloured words, an error's red rule
/// name, leaves them hard to read — `label` the AccessKit label under the
/// `Button` role on a clickable row, `Label` on one that only reports,
/// and `hover` the tooltip; a `select` row's click reports its player.
pub fn row_paint(
    ui: &egui::Ui,
    response: egui::Response,
    shapes: &[egui::Shape],
    label: &str,
    select: Option<SelectedPlayer>,
    hover: Option<&str>,
) -> Option<SelectedPlayer> {
    let rect = response.rect;
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                2.0,
                ui.visuals().widgets.hovered.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().extend(shapes.iter().cloned());
    }
    match select {
        Some(player) => {
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label)
            });
            let response = match hover {
                Some(hover) => response.on_hover_text(hover),
                None => response,
            };
            response.clicked().then_some(player)
        }
        None => {
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Label, ui.is_enabled(), label)
            });
            if let Some(hover) = hover {
                response.on_hover_text(hover);
            }
            None
        }
    }
}

/// A bound check's sentence: "{label} {value}, over the {bound} the rules
/// allow" above it, "under the {bound} the rules require" below it, true
/// whether the bound is a maximum (the official ruleset's `at_most`) or an
/// exact target (the invitationals'), which `max` is either way.
fn bound(label: &str, value: u8, bound: u8) -> String {
    if value > bound {
        format!("{label} {value}, over the {bound} the rules allow")
    } else {
        format!("{label} {value}, under the {bound} the rules require")
    }
}

/// The label paired with `id` in `entries`, a ruleset's tiers, brackets or
/// systems as `(id, label)`. An id the ruleset does not hold prints as
/// itself: a custom logic may report ids of its own, and a held check may
/// be from the ruleset in use before another was picked.
fn label_of<'a>(mut entries: impl Iterator<Item = (&'a str, &'a str)>, id: &'a str) -> &'a str {
    entries
        .find(|(entry_id, _)| *entry_id == id)
        .map_or(id, |(_, label)| label)
}

/// The label of the height bracket `id` in `ruleset` (`label_of`).
fn bracket_label<'a>(ruleset: &'a aatf::Ruleset, id: &'a str) -> &'a str {
    label_of(
        ruleset
            .heights
            .brackets
            .iter()
            .map(|bracket| (bracket.id.as_str(), bracket.label.as_str())),
        id,
    )
}

/// The label of the height system `id` in `ruleset` (`label_of`).
fn system_label<'a>(ruleset: &'a aatf::Ruleset, id: &'a str) -> &'a str {
    label_of(
        ruleset
            .heights
            .systems
            .iter()
            .map(|system| (system.id.as_str(), system.label.as_str())),
        id,
    )
}

/// `items` as a sentence lists them: `a`, `a and b`, `a, b and c`, with
/// `last` (`and` or `or`) before the final one.
fn joined(items: &[String], last: &str) -> String {
    match items.split_last() {
        Some((final_item, rest)) if !rest.is_empty() => {
            format!("{} {last} {final_item}", rest.join(", "))
        }
        _ => items.join(""),
    }
}

/// Positions as a sentence lists them, joined with `and`.
fn positions_text(positions: &[aatf::Position]) -> String {
    let labels: Vec<String> = positions
        .iter()
        .map(|position| position.label().to_string())
        .collect();
    joined(&labels, "and")
}

/// Inclusive height ranges as a sentence lists them, joined with `or`:
/// a range of one height as `180`, one from 0 as `up to 175`, any other
/// as `175 to 210`.
fn heights_text(ranges: &[(u8, u8)]) -> String {
    let spans: Vec<String> = ranges
        .iter()
        .map(|&(low, high)| {
            if low == high {
                low.to_string()
            } else if low == 0 {
                format!("up to {high}")
            } else {
                format!("{low} to {high}")
            }
        })
        .collect();
    joined(&spans, "or")
}

/// The parenthesis naming a player's A positions after `prefix`, or
/// saying he has none.
fn rated_a_text(prefix: &str, rated_a: &[aatf::Position]) -> String {
    if rated_a.is_empty() {
        " (no position rated A)".to_string()
    } else {
        format!(" ({prefix}{})", positions_text(rated_a))
    }
}

/// A team setting's id in words; an id the list does not know prints as
/// itself.
fn setting_words(setting: &str) -> &str {
    match setting {
        "man_marking" => "man marking",
        "auto_substitution" => "automatic substitutions",
        "auto_offside_trap" => "the automatic offside trap",
        "auto_preset_change" => "automatic preset changes",
        "auto_attack_defence_levels" => "automatic attack and defence levels",
        other => other,
    }
}

/// What is wrong with `violation`, in the draft's `violationSentence` wording:
/// one arm per `ViolationKind`, each stat named as the game of
/// `wording.version` names it (PES 2017's Header), each card as the game
/// names it (Heading) and each tier, bracket and height system by its
/// label in `wording.ruleset`, the rules the check ran under. `team_id` is the
/// team's, for the rules whose expected value it sets (the roster order's
/// `team id x 100 + slot + 1`). The row's text after the rule's name; a host
/// writing the list out as text writes it so.
pub fn sentence(violation: &aatf::Violation, team_id: u32, wording: Wording<'_>) -> String {
    use aatf::ViolationKind as K;
    let ruleset = wording.ruleset;
    match &violation.kind {
        K::RegisteredNotA { rated_a } => format!(
            "the registered position is not rated A{}",
            rated_a_text("rated A at ", rated_a)
        ),
        K::FieldedNotA {
            preset,
            position,
            rated_a,
        } => format!(
            "fielded at {} in preset {preset} without an A rating{}",
            position.label(),
            rated_a_text("rated A at ", rated_a)
        ),
        K::BRating { rated_b } if rated_b.is_empty() => {
            "a B rating, which the rules refuse".to_string()
        }
        K::BRating { rated_b } => format!(
            "a B rating, which the rules refuse (B at {})",
            positions_text(rated_b)
        ),
        K::GkSecondA { rated_a } if rated_a.is_empty() => "a second A rating at GK".to_string(),
        K::GkSecondA { rated_a } => format!(
            "a second A rating at GK (also A at {})",
            positions_text(rated_a)
        ),
        K::Age { age, min, max } => format!("age {age} outside {min} to {max}"),
        K::Weight { weight, min, max } => format!("weight {weight} outside {min} to {max}"),
        K::PositionRange => "a registered position this PES version does not have".to_string(),
        K::PlayingStyleRange => "a playing style this PES version does not have".to_string(),
        K::NoTier { heights } if heights.is_empty() => "fits no tier".to_string(),
        K::NoTier { heights } => {
            format!(
                "fits no tier (the tiers take heights {})",
                heights_text(heights)
            )
        }
        K::Stat {
            stat,
            value,
            target,
            ..
        } => {
            let stat = stat_name(wording.version, stat.label());
            // A warning is an `at_most` stat inside its band; an error, over
            // a maximum or off an exact target, reads as any bound does.
            if violation.severity == aatf::Severity::Warning {
                format!("{stat} {value}, just over the {target} the rules allow")
            } else {
                bound(&stat, *value, *target)
            }
        }
        K::StatTotal { total, limit } => {
            format!("the stats add up to {total}, over the limit of {limit}")
        }
        K::Form { value, target } => format!(
            "{} {value}, where the rules want {target}",
            stat_name(wording.version, "form")
        ),
        K::InjuryResistance { value, max } => bound(
            &stat_name(wording.version, "injury_resistance"),
            *value,
            *max,
        ),
        K::WeakFootUsage { value, max } => {
            bound(&stat_name(wording.version, "weak_foot_usage"), *value, *max)
        }
        K::WeakFootAccuracy { value, max } => bound(
            &stat_name(wording.version, "weak_foot_accuracy"),
            *value,
            *max,
        ),
        K::NameColor { color, expected } => match expected.as_deref() {
            Some(expected) => format!(
                "name colour {}, where his tier wants {}",
                color.as_deref().unwrap_or("none"),
                expected
            ),
            // A colour another tier reserved on a player outside it.
            None => format!(
                "name colour {}, which belongs to another tier",
                color.as_deref().unwrap_or("none")
            ),
        },
        K::Cards { paid, limit } => format!("{paid} paid cards, over the limit of {limit}"),
        K::MinCards { paid, min } => {
            format!("{paid} paid skill cards and extra positions, under the minimum of {min}")
        }
        K::SkillCardCap { count, max } => {
            format!("{count} skill cards, over the cap of {max}")
        }
        K::ComStyleCap { count, max } => format!("{count} COM styles, over the cap of {max}"),
        K::RequiredCard { card } => {
            format!("the required card {} is missing", card_name(card.label()))
        }
        K::APositions {
            count,
            max,
            rated_a,
        } => format!(
            "{count} A positions, over the allowance of {max}{}",
            rated_a_text("", rated_a)
        ),
        K::Height { height, fits, .. } if fits.is_empty() => {
            format!("height {height} takes no bracket")
        }
        K::Height { height, fits, room } => {
            let room = match (room, fits.len()) {
                (false, _) => "",
                (true, 1) => " has room",
                (true, _) => " have room",
            };
            format!(
                "height {height} takes no bracket ({}{room})",
                heights_text(fits)
            )
        }
        K::TierCount {
            tier,
            count,
            expected,
        } => format!(
            "{count} {} players, where the rules want {expected}",
            label_of(
                ruleset
                    .tiers
                    .iter()
                    .map(|tier| (tier.id.as_str(), tier.label.as_str())),
                tier
            )
        ),
        K::BracketQuota {
            system,
            bracket,
            count,
            quota,
            ..
        } => format!(
            "{count} players in the {} bracket of the {} system, where its quota is {quota}",
            bracket_label(ruleset, bracket),
            system_label(ruleset, system)
        ),
        K::MedalSystem { systems } => format!(
            "the tier counts match none of the medal systems {}",
            joined(systems, "or")
        ),
        K::ExtraHeight { used, total, .. } => {
            format!("{used} cm of extra height, over the pool of {total}")
        }
        K::NearestSystem { system, off } => {
            let off: Vec<String> = off
                .iter()
                .map(|(bracket, count, quota)| {
                    format!("{} {count} of {quota}", bracket_label(ruleset, bracket))
                })
                .collect();
            format!(
                "fits no quota system; the nearest is {}, with {}",
                system_label(ruleset, system),
                joined(&off, "and")
            )
        }
        K::Captain => "no captain".to_string(),
        K::Goalkeepers { count, min } => {
            format!("{count} goalkeepers, under the minimum of {min}")
        }
        K::TeamSetting {
            label,
            setting,
            wanted,
        } => format!(
            "team setting {label}: {} should be {}",
            setting_words(setting),
            if *wanted { "on" } else { "off" }
        ),
        K::FormationShape {
            preset,
            formation,
            shape,
        } => {
            let shape: Vec<String> = shape.iter().map(u8::to_string).collect();
            format!(
                "formation {formation} of preset {preset} is a {}, which the rules forbid",
                shape.join("-")
            )
        }
        K::Special { label, text } => format!("{label}: {text}"),
        K::Custom { text } => text.clone(),
        K::Unused {
            allowance,
            used,
            allowed,
        } => unused_sentence(allowance, *used, *allowed, wording.version),
        K::UnusedChoice { special } => format!("no option of {special} taken"),
        K::RosterOrder { player } => match violation.slot {
            Some(slot) => format!(
                "the slot holds player {player}, not {}",
                pes_savefile::model::roster_order_player_id(team_id, slot)
            ),
            None => format!("a roster slot holds player {player} outside the roster order"),
        },
    }
}

/// An `Unused` suggestion's sentence, the draft's `unusedSentence`: an
/// allowance the rules give that the player leaves unspent, in the words the
/// Ruleset editor's Suggestions form uses for it; a card group's free cards
/// under the group's name, a stat under the name the game of `version`
/// gives it.
fn unused_sentence(allowance: &str, used: u8, allowed: u8, version: PesVersion) -> String {
    match allowance {
        "cards" => format!("{used} of {allowed} paid cards used"),
        "a_positions" => format!("{used} of {allowed} A positions used"),
        "free_cards" => format!("{used} of {allowed} free cards held"),
        "weak_foot_usage" | "weak_foot_accuracy" | "injury_resistance" => format!(
            "{} {used}, where the rules allow {allowed}",
            stat_name(version, allowance)
        ),
        "height" => format!("height {used}, where the bracket allows {allowed}"),
        "com" => format!("{used} of {allowed} free COM styles held"),
        group => format!("{used} of {allowed} free {} cards held", title_case(group)),
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::{self, Role};
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::{
        Headless, centre, painted_color, painted_text_color, pointer_at, text_shape,
    };

    impl Headless {
        /// One frame of `violations_list` over `teams`, returning the nodes,
        /// the painted shapes and the selected player.
        fn list(
            &mut self,
            teams: &[TeamViolations],
            show_suggestions: bool,
            events: Vec<egui::Event>,
        ) -> (
            Vec<accesskit::Node>,
            Vec<egui::epaint::ClippedShape>,
            Option<SelectedPlayer>,
        ) {
            let list_use = ListUse::Editing {
                show_suggestions,
                one_column: false,
            };
            self.list_as(teams, list_use, events)
        }

        /// `list` drawn as `list_use` says.
        fn list_as(
            &mut self,
            teams: &[TeamViolations],
            list_use: ListUse,
            events: Vec<egui::Event>,
        ) -> (
            Vec<accesskit::Node>,
            Vec<egui::epaint::ClippedShape>,
            Option<SelectedPlayer>,
        ) {
            let mut selected = None;
            let (nodes, shapes) = self.frame(events, |ui| {
                selected = Some(violations_list(ui, "list", teams, official_17(), list_use));
            });
            (nodes, shapes, selected.expect("the list ran"))
        }

        /// A click at `pos` over the list: the press frame, then the release
        /// frame whose result is returned (`clicked()` lands on the release).
        fn click_list(
            &mut self,
            pos: egui::Pos2,
            teams: &[TeamViolations],
            show_suggestions: bool,
        ) -> (
            Vec<accesskit::Node>,
            Vec<egui::epaint::ClippedShape>,
            Option<SelectedPlayer>,
        ) {
            self.list(teams, show_suggestions, pointer_at(pos, true));
            self.list(teams, show_suggestions, pointer_at(pos, false))
        }
    }

    /// Every header and row the frame drew, in top-to-bottom order: a Label's
    /// `value` or a Button's `label` (`nodes` come in `IdMap` order, not
    /// layout order).
    fn drawn_lines(nodes: &[accesskit::Node]) -> Vec<String> {
        let mut lines: Vec<(f64, f64, String)> = Vec::new();
        for node in nodes {
            let text = match node.role() {
                Role::Label => node.value().map(str::to_owned),
                Role::Button => node.label().map(str::to_owned),
                _ => None,
            };
            if let (Some(text), Some(bounds)) = (text, node.bounds()) {
                lines.push((bounds.y0, bounds.x0, text));
            }
        }
        lines.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("drawn bounds")
                .then(a.1.partial_cmp(&b.1).expect("drawn bounds"))
        });
        lines.into_iter().map(|(_, _, text)| text).collect()
    }

    /// The official ruleset's data block, Autumn 26's: the ruleset the
    /// fixture checks run under, whose labels the rows print.
    fn official() -> &'static aatf::Ruleset {
        static OFFICIAL: std::sync::LazyLock<aatf::Ruleset> =
            std::sync::LazyLock::new(|| aatf::Rules::official().ruleset().clone());
        &OFFICIAL
    }

    /// The official ruleset's wording on a PES 2017 save: the fixture
    /// save's version, whose game names the stats.
    fn official_17() -> Wording<'static> {
        Wording {
            ruleset: official(),
            version: PesVersion::Pes17,
        }
    }

    /// A `Violation` on `slot` of `severity` and `kind`.
    fn violation(
        slot: Option<u8>,
        severity: aatf::Severity,
        kind: aatf::ViolationKind,
    ) -> aatf::Violation {
        aatf::Violation {
            slot,
            severity,
            kind,
        }
    }

    /// The stat `name` names.
    fn stat(name: &str) -> aatf::Stat {
        aatf::Stat::try_from(name.to_string()).expect("a stat name")
    }

    /// The position `name` names.
    fn position(name: &str) -> aatf::Position {
        aatf::Position::try_from(name.to_string()).expect("a position name")
    }

    /// A `ListedPlayer` (the roster slot, id, number and name), 180 cm tall:
    /// a test of a height row's lines sets his height itself.
    fn listed(slot: u8, id: u32, number: u16, name: &str) -> ListedPlayer<'_> {
        ListedPlayer {
            slot,
            id,
            number,
            name,
            height: 180,
        }
    }

    /// The PES 17 fixture save.
    fn fixture() -> pes_savefile::EditFile {
        pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(PesVersion::Pes17))
            .expect("fixture save")
    }

    /// Team `id`'s `ListedPlayer`s in `file` (the rostered ones, in slot
    /// order).
    fn listed_players<'a>(file: &'a pes_savefile::EditFile, id: u32) -> Vec<ListedPlayer<'a>> {
        let team = file.team(id).unwrap_or_else(|| panic!("team {id}"));
        team.roster
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.player_id != 0)
            .map(|(i, slot)| {
                let player = file.player(slot.player_id).expect("a rostered player");
                ListedPlayer {
                    slot: u8::try_from(i).expect("a slot fits u8"),
                    id: slot.player_id,
                    number: slot.number,
                    name: player.name.as_str(),
                    height: player.basic.height,
                }
            })
            .collect()
    }

    /// Team `id`'s check under the official ruleset on `file`.
    fn check(file: &pes_savefile::EditFile, id: u32) -> Vec<aatf::Violation> {
        let view = aatf::TeamView::new(file, id).expect("the team views");
        aatf::Rules::official()
            .check_team(&view)
            .expect("the check runs")
            .violations
    }

    /// The official ruleset's check on team 734 ("/lit/") of the PES 17
    /// fixture save, with two of its tall regulars (slots 0 and 3, 185
    /// cm) set to 183, which no Red bracket takes: 14 violations, a
    /// `Height` error on each of the two, the team's own (three
    /// `TierCount`, and the red tall quota at 8 of 10) and eight `Unused`
    /// suggestions on slots 9, 10, 14 and 22. The list draws the team's own
    /// group, then each player's in roster order.
    // SE-GUI-15
    #[test]
    fn the_list_groups_the_checks_violations_per_player() {
        let mut file = fixture();
        for id in [73401, 73404] {
            let player = file.player_mut(id).expect("rostered");
            assert_eq!(player.basic.height, 185, "premise: {id} is tall");
            player.basic.height = 183;
        }
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        assert_eq!(violations.len(), 14, "the check's count");
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, selected) = window.list(&teams, false, vec![]);
        assert_eq!(selected, None);
        assert_eq!(
            drawn_lines(&nodes),
            [
                "Team 734 /lit/",
                "Team-wide issues",
                "Tier count: 2 Gold players, where the rules want 1",
                "Tier count: 0 Bronze players, where the rules want 2",
                "Tier count: 19 Regular players, where the rules want 18",
                TALL_734,
                "Player 73401  49 Muh Spooks",
                "Height: height 183 takes no bracket (185 has room)",
                "Player 73404  45 Yuki-kun ^~^",
                "Height: height 183 takes no bracket (185 has room)",
            ]
        );
    }

    /// The Red tall quota's row of 734 /lit/ with the grouping test's two
    /// short regulars: its sentence, then its players, the 185 cm ones and
    /// the two 189 cm goalkeepers read at 185, in roster order.
    const TALL_734: &str = "Height quota: 8 players in the Tall bracket of the Red system, \
        where its quota is 10; At 185: knee-chee, The Judge, Huge Orson Welles, Start With the \
        Greeks, The Entire Western Canon (189), or, The Whale (189), Tolstoyevsky's War and \
        Punishment, my diary desu";

    /// The report over two teams: 734 /lit/ of the PES 17 fixture save with
    /// the grouping test's two short regulars (its errors, and its eight
    /// `Unused` suggestions), and 714 /co/ given only its own check's
    /// suggestions. Each team stands under its id and name; 714, whose
    /// rows are all suggestions, reads `All clean`; no suggestion is
    /// listed; a player's row reports rather than selects.
    #[test]
    fn a_report_heads_every_team_and_lists_no_suggestion() {
        let mut file = fixture();
        for id in [73401, 73404] {
            let player = file.player_mut(id).expect("rostered");
            assert_eq!(player.basic.height, 185, "premise: {id} is tall");
            player.basic.height = 183;
        }
        let players_734 = listed_players(&file, 734);
        let violations_734 = check(&file, 734);
        let players_714 = listed_players(&file, 714);
        let suggestions_714: Vec<aatf::Violation> = check(&file, 714)
            .into_iter()
            .filter(|v| v.severity == aatf::Severity::Suggestion)
            .collect();
        assert!(
            violations_734
                .iter()
                .any(|v| v.severity == aatf::Severity::Suggestion),
            "premise: 734's check holds suggestions"
        );
        assert!(!suggestions_714.is_empty(), "premise: 714 has suggestions");
        let team_734 = file.team(734).expect("team 734");
        let team_714 = file.team(714).expect("team 714");
        let teams = [
            TeamViolations {
                team_id: 734,
                team_name: &team_734.name,
                players: &players_734,
                violations: &violations_734,
                starting_eleven: &team_734.tactics.starting_eleven,
            },
            TeamViolations {
                team_id: 714,
                team_name: &team_714.name,
                players: &players_714,
                violations: &suggestions_714,
                starting_eleven: &team_714.tactics.starting_eleven,
            },
        ];
        let mut window = Headless::new();
        let report = |window: &mut Headless, events| {
            let mut selected = None;
            let (nodes, _) = window.frame(events, |ui| {
                selected = Some(violations_list(
                    ui,
                    "list",
                    &teams,
                    official_17(),
                    ListUse::Report,
                ));
            });
            (nodes, selected.expect("the list ran"))
        };
        let (nodes, _) = report(&mut window, vec![]);
        let height = "Height: height 183 takes no bracket (185 has room)";
        assert_eq!(
            drawn_lines(&nodes),
            [
                "734 /lit/",
                "Team-wide issues",
                "Tier count: 2 Gold players, where the rules want 1",
                "Tier count: 0 Bronze players, where the rules want 2",
                "Tier count: 19 Regular players, where the rules want 18",
                TALL_734,
                "Player 73401  49 Muh Spooks",
                height,
                "Player 73404  45 Yuki-kun ^~^",
                height,
                "714 /co/",
                "All clean",
            ]
        );
        let player_row = nodes
            .iter()
            .find(|node| node.value() == Some(height) || node.label() == Some(height))
            .expect("a player's row");
        assert_eq!(
            player_row.role(),
            Role::Label,
            "a report's row only reports"
        );
        let at = centre(player_row);
        report(&mut window, pointer_at(at, true));
        let (_, selected) = report(&mut window, pointer_at(at, false));
        assert_eq!(selected, None, "a click selects nobody");
    }

    // A group's errors come before its warnings, the check's order kept
    // inside each.
    #[test]
    fn a_groups_errors_come_before_its_warnings() {
        let violations = [
            violation(
                Some(5),
                aatf::Severity::Warning,
                aatf::ViolationKind::Stat {
                    stat: stat("attacking_prowess"),
                    value: 78,
                    target: 77,
                    at_most: true,
                },
            ),
            violation(
                Some(5),
                aatf::Severity::Error,
                aatf::ViolationKind::StatTotal {
                    total: 1772,
                    limit: 1771,
                },
            ),
            violation(
                Some(5),
                aatf::Severity::Error,
                aatf::ViolationKind::Height {
                    height: 185,
                    fits: vec![],
                    room: false,
                },
            ),
        ];
        let players = [listed(5, 1, 9, "X")];
        let teams = [TeamViolations {
            team_id: 1,
            team_name: "T",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, shapes, _) = window.list(&teams, false, vec![]);
        let error_color = window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .error_fg_color;
        let warn_color = window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .warn_fg_color;
        let stat_total = "Stat total: the stats add up to 1772, over the limit of 1771";
        let height = "Height: height 185 takes no bracket";
        let stat = "Stat: Attacking Prowess 78, just over the 77 the rules allow";
        assert_eq!(
            drawn_lines(&nodes),
            ["Team 1 T", "Player 1  9 X", stat_total, height, stat]
        );
        let strong_color = window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .strong_text_color();
        // The rule name carries the severity's color, the sentence always the
        // strong text color (the draft's `.f`/`.v`).
        for (rule_name, color) in [
            ("Stat total", error_color),
            ("Height", error_color),
            ("Stat", warn_color),
        ] {
            assert_eq!(painted_text_color(&shapes, rule_name), color, "{rule_name}");
        }
        for sentence in [
            "the stats add up to 1772, over the limit of 1771",
            "height 185 takes no bracket",
            "Attacking Prowess 78, just over the 77 the rules allow",
        ] {
            assert_eq!(
                painted_text_color(&shapes, sentence),
                strong_color,
                "{sentence}"
            );
        }
    }

    /// Clicking a player's row selects him; a team's row is not clickable.
    // SE-GUI-15
    #[test]
    fn a_player_rows_click_selects_him_on_that_frame_only() {
        let violations = [
            violation(
                Some(3),
                aatf::Severity::Error,
                aatf::ViolationKind::Height {
                    height: 185,
                    fits: vec![],
                    room: false,
                },
            ),
            violation(None, aatf::Severity::Error, aatf::ViolationKind::Captain),
        ];
        let players = [listed(3, 70103, 3, "NAME")];
        let teams = [TeamViolations {
            team_id: 701,
            team_name: "/3/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let player_row = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node.label() == Some("Height: height 185 takes no bracket")
            })
            .expect("the player's row");
        let (_, _, selected) = window.click_list(centre(player_row), &teams, false);
        assert_eq!(
            selected,
            Some(SelectedPlayer {
                team_id: 701,
                slot: 3,
                fielded: None,
            })
        );
        let (_, _, selected) = window.list(&teams, false, vec![]);
        assert_eq!(selected, None, "the next frame reports nothing new");

        let team_row = nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some("Captain: no captain"))
            .expect("the team's row");
        let (_, _, selected) = window.click_list(centre(team_row), &teams, false);
        assert_eq!(selected, None, "a team row is not clickable");
    }

    /// Suggestions draw only when asked, in their own group after the
    /// violations: team 734's eight `Unused` rows on slots 9, 10, 14 and 22.
    // SE-GUI-15
    #[test]
    fn suggestions_draw_on_request_in_their_own_group() {
        let file = fixture();
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        assert!(
            !drawn_lines(&nodes)
                .iter()
                .any(|line| line == "Suggestions" || line.starts_with("Unused")),
            "no suggestion drawn when off"
        );
        let (nodes, shapes, _) = window.list(&teams, true, vec![]);
        let lines = drawn_lines(&nodes);
        let at = lines
            .iter()
            .position(|line| line == "Suggestions")
            .expect("the Suggestions header");
        // D1: 73410's extra A position now costs a card against the
        // gold's 6 (his trade covers the COM style, not the position).
        assert_eq!(
            lines[at..],
            [
                "Suggestions",
                "Player 73410  46 DFW",
                "Unused: 4 of 5 paid cards used",
                "Unused: Weak Foot Usage 2, where the rules allow 4",
                "Player 73411  1 Start With the Greeks",
                "Unused: 5 of 6 paid cards used",
                "Unused: Weak Foot Usage 2, where the rules allow 4",
                "Player 73415  87 A.M. Pinecone",
                "Unused: 4 of 5 paid cards used",
                "Unused: Weak Foot Usage 2, where the rules allow 4",
                "Player 73423  2 my diary desu",
                "Unused: 4 of 5 paid cards used",
                "Unused: Weak Foot Usage 2, where the rules allow 4",
            ]
        );
        assert_eq!(
            painted_text_color(&shapes, "Unused"),
            window.ctx.style_of(window.ctx.theme()).visuals.text_color(),
            "a suggestion's rule name is the normal text color"
        );
    }

    // The row's sentence is the draft's `violationSentence` wording, so
    // both hosts word a violation alike (`Stat`'s warning wording is
    // its `sev === "Warning"` branch, an absent `NameColor` value `none`).
    #[test]
    fn the_row_sentences_word_the_kinds() {
        use aatf::Severity::{Error, Warning};
        use aatf::ViolationKind as K;
        let player = |severity, kind| violation(Some(1), severity, kind);
        let sent = |v: &aatf::Violation| sentence(v, 701, official_17());
        assert_eq!(
            sent(&player(
                Error,
                K::FieldedNotA {
                    preset: 1,
                    position: position("CB"),
                    rated_a: vec![position("DMF")]
                }
            )),
            "fielded at CB in preset 1 without an A rating (rated A at DMF)"
        );
        assert_eq!(
            sent(&player(
                Error,
                K::Stat {
                    stat: stat("speed"),
                    value: 80,
                    target: 92,
                    at_most: false
                }
            )),
            "Speed 80, under the 92 the rules require"
        );
        assert_eq!(
            sent(&player(
                Error,
                K::Stat {
                    stat: stat("speed"),
                    value: 95,
                    target: 92,
                    at_most: false
                }
            )),
            "Speed 95, over the 92 the rules allow"
        );
        assert_eq!(
            sent(&player(
                Error,
                K::Stat {
                    stat: stat("attacking_prowess"),
                    value: 82,
                    target: 77,
                    at_most: true
                }
            )),
            "Attacking Prowess 82, over the 77 the rules allow"
        );
        assert_eq!(
            sent(&player(
                Warning,
                K::Stat {
                    stat: stat("attacking_prowess"),
                    value: 78,
                    target: 77,
                    at_most: true
                }
            )),
            "Attacking Prowess 78, just over the 77 the rules allow"
        );
        assert_eq!(
            sent(&player(
                Error,
                K::NameColor {
                    color: None,
                    expected: Some("cc9900ff".to_string())
                }
            )),
            "name colour none, where his tier wants cc9900ff"
        );
        assert_eq!(
            sent(&player(Error, K::Cards { paid: 10, limit: 8 })),
            "10 paid cards, over the limit of 8"
        );
        assert_eq!(
            sent(&player(
                Error,
                K::TierCount {
                    tier: "gold".to_string(),
                    count: 0,
                    expected: 1
                }
            )),
            "0 Gold players, where the rules want 1"
        );

        // A value off its bound says which side it sits on: VTL11's weak foot
        // is an exact target (`aatf`'s `weak_foot_below_the_exact_target`,
        // slot 14), 3 against the required 4; VTL10's usage 4 is over its
        // exact 2 (fixtures/vtl10.rs:272); the official regulars' weak foot
        // at_most 2 is a maximum.
        for (kind, expected) in [
            (
                K::WeakFootUsage { value: 3, max: 4 },
                "Weak Foot Usage 3, under the 4 the rules require",
            ),
            (
                K::WeakFootAccuracy { value: 3, max: 4 },
                "Weak Foot Accuracy 3, under the 4 the rules require",
            ),
            (
                K::InjuryResistance { value: 1, max: 3 },
                "Injury Resistance 1, under the 3 the rules require",
            ),
            (
                K::WeakFootUsage { value: 4, max: 2 },
                "Weak Foot Usage 4, over the 2 the rules allow",
            ),
            (
                K::WeakFootUsage { value: 3, max: 2 },
                "Weak Foot Usage 3, over the 2 the rules allow",
            ),
            (
                K::MinCards { paid: 2, min: 5 },
                "2 paid skill cards and extra positions, under the minimum of 5",
            ),
            (
                K::NameColor {
                    color: Some("cc9900ff".to_string()),
                    expected: None,
                },
                "name colour cc9900ff, which belongs to another tier",
            ),
        ] {
            assert_eq!(sent(&player(Error, kind.clone())), expected, "{kind:?}");
        }

        // `RosterOrder` words the held id and the rule's, the latter from
        // the team id and the slot: slot 4 of team 701 is player 70105.
        assert_eq!(
            sentence(
                &violation(Some(4), Error, K::RosterOrder { player: 70106 }),
                701,
                official_17()
            ),
            "the slot holds player 70106, not 70105"
        );
    }

    // A hint names a valid value in the draft's `violationSentence` words:
    // a list joined with "and" or "or", a range of one height as itself,
    // one from 0 as "up to 175", any other as "175 to 210". An empty list adds no
    // parenthesis, except the A positions', which says there are none.
    #[test]
    fn the_hints_word_a_valid_value() {
        use aatf::ViolationKind as K;
        let sent = |kind: K| {
            sentence(
                &violation(Some(1), aatf::Severity::Error, kind),
                701,
                official_17(),
            )
        };
        let positions = |names: &[&str]| names.iter().map(|name| position(name)).collect();
        let height = |fits: &[(u8, u8)], room| K::Height {
            height: 183,
            fits: fits.to_vec(),
            room,
        };
        for (kind, expected) in [
            (
                height(&[(180, 180), (185, 185)], true),
                "height 183 takes no bracket (180 or 185 have room)",
            ),
            (
                height(&[(185, 185)], true),
                "height 183 takes no bracket (185 has room)",
            ),
            (
                height(&[(0, 175), (180, 180), (185, 185)], false),
                "height 183 takes no bracket (up to 175, 180 or 185)",
            ),
            (
                height(&[(175, 210)], false),
                "height 183 takes no bracket (175 to 210)",
            ),
            (height(&[], false), "height 183 takes no bracket"),
            (
                K::NoTier {
                    heights: vec![(180, 180), (185, 185), (190, 190), (195, 195)],
                },
                "fits no tier (the tiers take heights 180, 185, 190 or 195)",
            ),
            (K::NoTier { heights: vec![] }, "fits no tier"),
            (
                K::RegisteredNotA {
                    rated_a: positions(&["CF", "SS"]),
                },
                "the registered position is not rated A (rated A at CF and SS)",
            ),
            (
                K::RegisteredNotA { rated_a: vec![] },
                "the registered position is not rated A (no position rated A)",
            ),
            (
                K::FieldedNotA {
                    preset: 1,
                    position: position("CB"),
                    rated_a: positions(&["DMF"]),
                },
                "fielded at CB in preset 1 without an A rating (rated A at DMF)",
            ),
            (
                K::APositions {
                    count: 3,
                    max: 2,
                    rated_a: positions(&["CF", "SS", "AMF"]),
                },
                "3 A positions, over the allowance of 2 (CF, SS and AMF)",
            ),
            (
                K::GkSecondA {
                    rated_a: positions(&["CB"]),
                },
                "a second A rating at GK (also A at CB)",
            ),
            (K::GkSecondA { rated_a: vec![] }, "a second A rating at GK"),
            (
                K::BRating {
                    rated_b: positions(&["LB", "RB"]),
                },
                "a B rating, which the rules refuse (B at LB and RB)",
            ),
            (
                K::BRating { rated_b: vec![] },
                "a B rating, which the rules refuse",
            ),
        ] {
            assert_eq!(sent(kind.clone()), expected, "{kind:?}");
        }
        // Each setting in words, the value the entry wants as on or off; an
        // id outside the list prints as itself.
        for (setting, wanted, expected) in [
            (
                "man_marking",
                false,
                "team setting No man marking: man marking should be off",
            ),
            (
                "auto_substitution",
                true,
                "team setting No man marking: automatic substitutions should be on",
            ),
            (
                "auto_offside_trap",
                false,
                "team setting No man marking: the automatic offside trap should be off",
            ),
            (
                "auto_preset_change",
                false,
                "team setting No man marking: automatic preset changes should be off",
            ),
            (
                "auto_attack_defence_levels",
                false,
                "team setting No man marking: automatic attack and defence levels should be off",
            ),
            (
                "new_flag",
                true,
                "team setting No man marking: new_flag should be on",
            ),
        ] {
            assert_eq!(
                sent(K::TeamSetting {
                    label: "No man marking".to_string(),
                    setting: setting.to_string(),
                    wanted,
                }),
                expected
            );
        }
    }

    /// Every kind's name and sentence in words, one row per kind (more for
    /// a kind whose wording branches), under real embedded rulesets: Autumn
    /// 26's tiers, brackets and systems, and DBC7's `giga` system, whose
    /// label `GigaHeight` is not its id title-cased. No name or sentence
    /// carries a rules file's spelling: no `_`, no `[`, no id the ruleset
    /// labels otherwise, no name stepping from lower to upper case inside a
    /// word (`FieldedNotA`).
    #[test]
    fn every_kind_is_named_and_worded_in_words() {
        use aatf::Severity::{Error, Suggestion, Warning};
        use aatf::ViolationKind as K;
        let dbc7 = aatf::builtin("dbc7").expect("dbc7 is built in");
        let dbc7 = Wording {
            ruleset: dbc7.ruleset(),
            version: PesVersion::Pes17,
        };
        let positions = |names: &[&str]| names.iter().map(|name| position(name)).collect();
        let unused = |allowance: &str, used, allowed| K::Unused {
            allowance: allowance.to_string(),
            used,
            allowed,
        };
        let a26 = official_17();
        let rows: Vec<(Wording, aatf::Severity, K, &str, &str)> = vec![
            (
                a26,
                Error,
                K::RegisteredNotA {
                    rated_a: positions(&["CF", "SS"]),
                },
                "Registered position",
                "the registered position is not rated A (rated A at CF and SS)",
            ),
            (
                a26,
                Error,
                K::FieldedNotA {
                    preset: 1,
                    position: position("CB"),
                    rated_a: positions(&["DMF"]),
                },
                "Fielded position",
                "fielded at CB in preset 1 without an A rating (rated A at DMF)",
            ),
            (
                a26,
                Error,
                K::BRating {
                    rated_b: positions(&["LB", "RB"]),
                },
                "B rating",
                "a B rating, which the rules refuse (B at LB and RB)",
            ),
            (
                a26,
                Error,
                K::BRating { rated_b: vec![] },
                "B rating",
                "a B rating, which the rules refuse",
            ),
            (
                a26,
                Error,
                K::GkSecondA {
                    rated_a: positions(&["CB"]),
                },
                "GK rating",
                "a second A rating at GK (also A at CB)",
            ),
            (
                a26,
                Error,
                K::Age {
                    age: 14,
                    min: 15,
                    max: 50,
                },
                "Age",
                "age 14 outside 15 to 50",
            ),
            (
                a26,
                Error,
                K::Weight {
                    weight: 29,
                    min: 30,
                    max: 104,
                },
                "Weight",
                "weight 29 outside 30 to 104",
            ),
            (
                a26,
                Error,
                K::PositionRange,
                "Registered position",
                "a registered position this PES version does not have",
            ),
            (
                a26,
                Error,
                K::PlayingStyleRange,
                "Playing style",
                "a playing style this PES version does not have",
            ),
            (
                a26,
                Error,
                K::NoTier {
                    heights: vec![(0, 175), (180, 180)],
                },
                "Tier",
                "fits no tier (the tiers take heights up to 175 or 180)",
            ),
            (
                a26,
                Error,
                K::Stat {
                    stat: stat("attacking_prowess"),
                    value: 82,
                    target: 77,
                    at_most: true,
                },
                "Stat",
                "Attacking Prowess 82, over the 77 the rules allow",
            ),
            (
                a26,
                Warning,
                K::Stat {
                    stat: stat("attacking_prowess"),
                    value: 78,
                    target: 77,
                    at_most: true,
                },
                "Stat",
                "Attacking Prowess 78, just over the 77 the rules allow",
            ),
            (
                a26,
                Error,
                K::Stat {
                    stat: stat("speed"),
                    value: 80,
                    target: 92,
                    at_most: false,
                },
                "Stat",
                "Speed 80, under the 92 the rules require",
            ),
            (
                a26,
                Error,
                K::Stat {
                    stat: stat("heading"),
                    value: 82,
                    target: 77,
                    at_most: true,
                },
                "Stat",
                "Header 82, over the 77 the rules allow",
            ),
            (
                Wording {
                    version: PesVersion::Pes21,
                    ..a26
                },
                Error,
                K::Stat {
                    stat: stat("heading"),
                    value: 82,
                    target: 77,
                    at_most: true,
                },
                "Stat",
                "Heading 82, over the 77 the rules allow",
            ),
            (
                a26,
                Error,
                K::StatTotal {
                    total: 1772,
                    limit: 1771,
                },
                "Stat total",
                "the stats add up to 1772, over the limit of 1771",
            ),
            (
                a26,
                Error,
                K::Form {
                    value: 8,
                    target: 4,
                },
                "Form",
                "Form 8, where the rules want 4",
            ),
            (
                a26,
                Error,
                K::InjuryResistance { value: 3, max: 1 },
                "Injury Resistance",
                "Injury Resistance 3, over the 1 the rules allow",
            ),
            (
                a26,
                Error,
                K::WeakFootUsage { value: 3, max: 4 },
                "Weak Foot Usage",
                "Weak Foot Usage 3, under the 4 the rules require",
            ),
            (
                a26,
                Error,
                K::WeakFootAccuracy { value: 3, max: 2 },
                "Weak Foot Accuracy",
                "Weak Foot Accuracy 3, over the 2 the rules allow",
            ),
            (
                a26,
                Error,
                K::NameColor {
                    color: None,
                    expected: Some("cc9900ff".to_string()),
                },
                "Name colour",
                "name colour none, where his tier wants cc9900ff",
            ),
            (
                a26,
                Error,
                K::NameColor {
                    color: Some("cc9900ff".to_string()),
                    expected: None,
                },
                "Name colour",
                "name colour cc9900ff, which belongs to another tier",
            ),
            (
                a26,
                Error,
                K::Cards { paid: 4, limit: 3 },
                "Cards",
                "4 paid cards, over the limit of 3",
            ),
            (
                a26,
                Error,
                K::MinCards { paid: 2, min: 5 },
                "Cards",
                "2 paid skill cards and extra positions, under the minimum of 5",
            ),
            (
                a26,
                Error,
                K::SkillCardCap { count: 11, max: 10 },
                "Skill cards",
                "11 skill cards, over the cap of 10",
            ),
            (
                a26,
                Error,
                K::ComStyleCap { count: 3, max: 2 },
                "COM styles",
                "3 COM styles, over the cap of 2",
            ),
            (
                a26,
                Error,
                K::RequiredCard {
                    card: aatf::Card::try_from("gk_long_throw".to_string()).expect("a card"),
                },
                "Required card",
                "the required card GK Long Throw is missing",
            ),
            (
                a26,
                Error,
                K::APositions {
                    count: 3,
                    max: 2,
                    rated_a: positions(&["CF", "SS", "AMF"]),
                },
                "A positions",
                "3 A positions, over the allowance of 2 (CF, SS and AMF)",
            ),
            (
                a26,
                Error,
                K::Height {
                    height: 183,
                    fits: vec![(185, 185)],
                    room: true,
                },
                "Height",
                "height 183 takes no bracket (185 has room)",
            ),
            (
                a26,
                Error,
                K::TierCount {
                    tier: "gold".to_string(),
                    count: 2,
                    expected: 1,
                },
                "Tier count",
                "2 Gold players, where the rules want 1",
            ),
            (
                a26,
                Error,
                K::BracketQuota {
                    system: "red".to_string(),
                    bracket: "tall".to_string(),
                    count: 8,
                    quota: 10,
                    players: vec![(4, 185)],
                },
                "Height quota",
                "8 players in the Tall bracket of the Red system, where its quota is 10",
            ),
            (
                dbc7,
                Error,
                K::BracketQuota {
                    system: "giga".to_string(),
                    bracket: "mid".to_string(),
                    count: 9,
                    quota: 10,
                    players: vec![(0, 185), (1, 185)],
                },
                "Height quota",
                "9 players in the Mid bracket of the GigaHeight system, where its quota is 10",
            ),
            (
                dbc7,
                Warning,
                K::NearestSystem {
                    system: "height".to_string(),
                    off: vec![("mid".to_string(), 6, 5), ("tall".to_string(), 5, 6)],
                },
                "Nearest system",
                "fits no quota system; the nearest is Height, with Mid 6 of 5 and Tall 5 of 6",
            ),
            (
                dbc7,
                Error,
                K::MedalSystem {
                    systems: ["GGG", "GGSS", "GSSSS", "SSSSSS"]
                        .iter()
                        .map(|label| label.to_string())
                        .collect(),
                },
                "Medal system",
                "the tier counts match none of the medal systems GGG, GGSS, GSSSS or SSSSSS",
            ),
            (
                a26,
                Error,
                K::ExtraHeight {
                    used: 51,
                    total: 50,
                    draws: vec![(4, "tall".to_string(), 10)],
                },
                "Extra height",
                "51 cm of extra height, over the pool of 50",
            ),
            (a26, Error, K::Captain, "Captain", "no captain"),
            (
                a26,
                Error,
                K::Goalkeepers { count: 0, min: 1 },
                "Goalkeepers",
                "0 goalkeepers, under the minimum of 1",
            ),
            (
                a26,
                Error,
                K::TeamSetting {
                    label: "No man marking".to_string(),
                    setting: "man_marking".to_string(),
                    wanted: false,
                },
                "Team setting",
                "team setting No man marking: man marking should be off",
            ),
            (
                a26,
                Error,
                K::FormationShape {
                    preset: 1,
                    formation: 2,
                    shape: [3, 4, 3],
                },
                "Formation",
                "formation 2 of preset 1 is a 3-4-3, which the rules forbid",
            ),
            (
                a26,
                Error,
                K::Special {
                    label: "Medals cannot be goalkeepers".to_string(),
                    text: "Medals cannot play as GK".to_string(),
                },
                "Special rule",
                "Medals cannot be goalkeepers: Medals cannot play as GK",
            ),
            (
                a26,
                Error,
                K::Custom {
                    text: "More than two goalkeepers".to_string(),
                },
                "Custom check",
                "More than two goalkeepers",
            ),
            (
                a26,
                Suggestion,
                unused("cards", 4, 5),
                "Unused",
                "4 of 5 paid cards used",
            ),
            (
                a26,
                Suggestion,
                unused("a_positions", 1, 2),
                "Unused",
                "1 of 2 A positions used",
            ),
            (
                a26,
                Suggestion,
                unused("free_cards", 0, 1),
                "Unused",
                "0 of 1 free cards held",
            ),
            (
                a26,
                Suggestion,
                unused("weak_foot_usage", 2, 4),
                "Unused",
                "Weak Foot Usage 2, where the rules allow 4",
            ),
            (
                a26,
                Suggestion,
                unused("weak_foot_accuracy", 2, 4),
                "Unused",
                "Weak Foot Accuracy 2, where the rules allow 4",
            ),
            (
                a26,
                Suggestion,
                unused("injury_resistance", 1, 3),
                "Unused",
                "Injury Resistance 1, where the rules allow 3",
            ),
            (
                a26,
                Suggestion,
                unused("height", 170, 175),
                "Unused",
                "height 170, where the bracket allows 175",
            ),
            (
                a26,
                Suggestion,
                unused("com", 0, 1),
                "Unused",
                "0 of 1 free COM styles held",
            ),
            (
                a26,
                Suggestion,
                unused("tricks", 1, 3),
                "Unused",
                "1 of 3 free Tricks cards held",
            ),
            (
                a26,
                Suggestion,
                K::UnusedChoice {
                    special: "Medals trade a free COM style for a free A position".to_string(),
                },
                "Unused choice",
                "no option of Medals trade a free COM style for a free A position taken",
            ),
            (
                a26,
                Error,
                K::RosterOrder { player: 73406 },
                "Roster order",
                "the slot holds player 73406, not 73405",
            ),
        ];
        let kinds: std::collections::BTreeSet<&str> =
            rows.iter().map(|(_, _, kind, ..)| kind.rule().0).collect();
        assert_eq!(kinds.len(), 37, "one row or more for each of the 37 kinds");
        for (wording, severity, kind, name, expected) in rows {
            // Slot 4 of team 734: the roster order's player 73405.
            let violation = violation(Some(4), severity, kind);
            assert_eq!(rule_name(&violation.kind), name, "{:?}", violation.kind);
            let text = sentence(&violation, 734, wording);
            assert_eq!(text, expected, "{:?}", violation.kind);
            let camel = name
                .as_bytes()
                .windows(2)
                .any(|pair| pair[0].is_ascii_lowercase() && pair[1].is_ascii_uppercase());
            assert!(!name.contains('_') && !camel, "{name:?}");
            assert!(!text.contains('_') && !text.contains('['), "{text:?}");
            let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
            let labelled = wording
                .ruleset
                .tiers
                .iter()
                .map(|tier| (&tier.id, &tier.label))
                .chain(
                    wording
                        .ruleset
                        .heights
                        .brackets
                        .iter()
                        .map(|bracket| (&bracket.id, &bracket.label)),
                )
                .chain(
                    wording
                        .ruleset
                        .heights
                        .systems
                        .iter()
                        .map(|system| (&system.id, &system.label)),
                );
            for (id, label) in labelled {
                assert!(
                    id == label || !words.contains(&id.as_str()),
                    "{text:?} spells {id:?}, labelled {label:?}"
                );
            }
        }
    }

    /// An id the ruleset does not hold (a custom logic's own, or a check
    /// held from the ruleset in use before another was picked) prints as
    /// itself.
    #[test]
    fn an_id_the_ruleset_does_not_hold_prints_as_itself() {
        let kind = aatf::ViolationKind::BracketQuota {
            system: "purple".to_string(),
            bracket: "tall".to_string(),
            count: 0,
            quota: 10,
            players: vec![],
        };
        assert!(
            official().heights.systems.iter().all(|s| s.id != "purple"),
            "premise: Autumn 26 has no purple system"
        );
        assert_eq!(
            sentence(
                &violation(None, aatf::Severity::Error, kind),
                734,
                official_17()
            ),
            "0 players in the Tall bracket of the purple system, where its quota is 10"
        );
    }

    /// DBC7's wording on a PES 2017 save: its brackets' labels (Manlet, Mid,
    /// Tall, Giant) name a height row's lines.
    fn dbc7_17() -> Wording<'static> {
        static DBC7: std::sync::LazyLock<aatf::Rules> =
            std::sync::LazyLock::new(|| aatf::builtin("dbc7").expect("dbc7 is built in"));
        Wording {
            ruleset: DBC7.ruleset(),
            version: PesVersion::Pes17,
        }
    }

    /// Players of team 897 /暁Records/ in DBC6's save, from its Team TOML:
    /// roster slot (0-based), name and own height.
    fn records_897() -> Vec<ListedPlayer<'static>> {
        [
            (0, "Stack Bros.", 189),
            (1, "HELLOHELL", 185),
            (2, "Let's Ghost", 180),
            (3, "Love East", 180),
            (5, "BANG BANG BANKI BANKI SEKIIIIBANKI", 195),
            (8, "dokidoki wakuwaku", 195),
            (12, "Kenkon Itteki!", 189),
        ]
        .into_iter()
        .map(|(slot, name, height)| ListedPlayer {
            height,
            ..listed(slot, 89701 + u32::from(slot), u16::from(slot) + 1, name)
        })
        .collect()
    }

    /// A `BracketQuota`'s players by the height read, tallest first: a tie
    /// in roster order whatever the field's order, a player read at
    /// another height than his own (an allowance) with his own after his
    /// name, a coded name drawn plain and a slot no listed player holds
    /// named by its slot.
    #[test]
    fn a_bracket_quotas_lines_group_its_players_by_the_height_read() {
        let mut players = records_897();
        let coded = "\u{11}cff0000ffLove East";
        players[3].name = coded;
        let kind = aatf::ViolationKind::BracketQuota {
            system: "giga".to_string(),
            bracket: "mid".to_string(),
            count: 6,
            quota: 10,
            players: vec![(2, 180), (12, 185), (0, 185), (3, 180), (1, 185), (20, 180)],
        };
        assert_eq!(
            violation_detail(
                &violation(None, aatf::Severity::Error, kind),
                &players,
                dbc7_17()
            ),
            [
                "At 185: Stack Bros. (189), HELLOHELL, Kenkon Itteki! (189)",
                "At 180: Let's Ghost, Love East, Slot 20",
            ]
        );
    }

    /// An `ExtraHeight`'s draws by bracket and centimetres, most first,
    /// each line counting them, naming the bracket by its label and adding
    /// up its centimetres; roster order inside a line. Tall draws at two
    /// centimetres and Tall and Mid at one keep three lines apart, which a
    /// grouping by the bracket or the centimetres alone would merge.
    #[test]
    fn an_extra_heights_lines_group_its_draws_by_bracket_and_centimetres() {
        let kind = aatf::ViolationKind::ExtraHeight {
            used: 30,
            total: 25,
            draws: vec![
                (0, "mid".to_string(), 5),
                (1, "mid".to_string(), 5),
                (5, "tall".to_string(), 10),
                (8, "tall".to_string(), 5),
                (12, "mid".to_string(), 5),
            ],
        };
        assert_eq!(
            violation_detail(
                &violation(None, aatf::Severity::Error, kind),
                &records_897(),
                dbc7_17()
            ),
            [
                "1 in Tall slots, 10 cm each (10): BANG BANG BANKI BANKI SEKIIIIBANKI",
                "3 in Mid slots, 5 cm each (15): Stack Bros., HELLOHELL, Kenkon Itteki!",
                "1 in Tall slots, 5 cm each (5): dokidoki wakuwaku",
            ]
        );
    }

    /// A height row's lines stand under its sentence, in the sentence's
    /// column and the plain text colour, wrapping at the column's right edge;
    /// the row's height covers them and its label carries them, each after
    /// `; `.
    #[test]
    fn a_height_rows_lines_stand_under_its_sentence() {
        let violations = [violation(
            None,
            aatf::Severity::Error,
            aatf::ViolationKind::ExtraHeight {
                used: 65,
                total: 50,
                draws: vec![
                    (0, "mid".to_string(), 5),
                    (1, "mid".to_string(), 5),
                    (5, "tall".to_string(), 10),
                    (8, "tall".to_string(), 10),
                    (12, "mid".to_string(), 5),
                ],
            },
        )];
        let players = records_897();
        let teams = [TeamViolations {
            team_id: 897,
            team_name: "/暁Records/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::sized(500.0, 400.0);
        let (nodes, shapes) = window.frame(vec![], |ui| {
            violations_list(ui, "list", &teams, dbc7_17(), ListUse::Report);
        });
        let label = "Extra height: 65 cm of extra height, over the pool of 50; \
                     2 in Tall slots, 10 cm each (20): BANG BANG BANKI BANKI SEKIIIIBANKI, \
                     dokidoki wakuwaku; 3 in Mid slots, 5 cm each (15): Stack Bros., \
                     HELLOHELL, Kenkon Itteki!";
        assert_eq!(
            drawn_lines(&nodes),
            ["897 /暁Records/", "Team-wide issues", label]
        );
        let row = nodes
            .iter()
            .find(|node| node.value() == Some(label))
            .expect("the Extra height row")
            .bounds()
            .expect("bounds");
        let sentence = text_shape(&shapes, "65 cm of extra height");
        let lines = text_shape(&shapes, "2 in Tall slots");
        assert_eq!(
            lines.galley.job.text,
            "2 in Tall slots, 10 cm each (20): BANG BANG BANKI BANKI SEKIIIIBANKI, \
             dokidoki wakuwaku\n3 in Mid slots, 5 cm each (15): Stack Bros., HELLOHELL, \
             Kenkon Itteki!"
        );
        assert_eq!(
            painted_color(lines),
            window.ctx.style_of(window.ctx.theme()).visuals.text_color(),
            "the plain text colour"
        );
        assert!(
            (lines.pos.x - sentence.pos.x).abs() <= 0.5,
            "the lines start at the sentence's column"
        );
        assert!(
            lines.pos.y >= sentence.pos.y + sentence.galley.rect.height() - 0.5,
            "the lines stand under the sentence"
        );
        assert!(
            lines.galley.rows.len() > 2,
            "a line longer than the column wraps: {} rows",
            lines.galley.rows.len()
        );
        assert!(
            f64::from(lines.pos.x + lines.galley.rect.width()) <= row.x1 + 0.5,
            "the lines do not run past the row's right edge"
        );
        assert!(
            f64::from(lines.pos.y + lines.galley.rect.height()) <= row.y1 + 0.5,
            "the row's height covers the lines"
        );
    }

    /// Only the height rows that name players have lines: a `NearestSystem`
    /// or a `Height` row has none.
    #[test]
    fn other_kinds_have_no_lines() {
        for kind in [
            aatf::ViolationKind::NearestSystem {
                system: "height".to_string(),
                off: vec![("mid".to_string(), 6, 5)],
            },
            aatf::ViolationKind::Height {
                height: 183,
                fits: vec![],
                room: false,
            },
        ] {
            assert_eq!(
                violation_detail(
                    &violation(Some(0), aatf::Severity::Error, kind),
                    &records_897(),
                    dbc7_17()
                ),
                Vec::<String>::new()
            );
        }
    }

    // A team the check found nothing wrong with gets no header of its own.
    #[test]
    fn a_team_with_no_violations_draws_no_header() {
        let empty = TeamViolations {
            team_id: 1,
            team_name: "EMPTY",
            players: &[],
            violations: &[],
            starting_eleven: &[],
        };
        let violations = [violation(
            Some(2),
            aatf::Severity::Error,
            aatf::ViolationKind::Height {
                height: 185,
                fits: vec![],
                room: false,
            },
        )];
        let players = [listed(2, 70202, 2, "GUILTY")];
        let guilty = TeamViolations {
            team_id: 702,
            team_name: "/a/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        };
        let teams = [empty, guilty];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        assert_eq!(
            drawn_lines(&nodes),
            [
                "Team 702 /a/",
                "Player 70202  2 GUILTY",
                "Height: height 185 takes no bracket",
            ],
            "the empty team's header is not drawn"
        );
    }

    // The group header draws the name code-free (a cup save's medal
    // players carry colour codes).
    #[test]
    fn a_coded_name_draws_as_plain_text() {
        // Player 71801 of the PES 17 fixture save: his name carries two colour
        // codes.
        let file = fixture();
        let raw = file.player(71801).expect("player 71801").name.as_str();
        assert!(
            pes_savefile::model::names::name_color(raw).is_some(),
            "premise: the fixture name is coded"
        );
        let violations = [violation(
            Some(0),
            aatf::Severity::Error,
            aatf::ViolationKind::Height {
                height: 185,
                fits: vec![],
                room: false,
            },
        )];
        let players = [listed(0, 71801, 50, raw)];
        let teams = [TeamViolations {
            team_id: 718,
            team_name: "/f/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let lines = drawn_lines(&nodes);
        assert_eq!(
            lines,
            [
                "Team 718 /f/",
                "Player 71801  50 ROADTRIP",
                "Height: height 185 takes no bracket",
            ]
        );
        assert!(
            lines.iter().all(|line| !line.contains('\u{11}')),
            "no colour code byte is drawn"
        );
    }

    #[test]
    fn a_coded_team_name_heads_its_rows_as_plain_text() {
        // Team 718 of the PES 17 fixture save, its name given a colour
        // code ahead of its own letters.
        let file = fixture();
        let name = file.team(718).expect("team 718").name.as_str();
        let coded = format!("\x11cff0000ff{name}");
        let violations = [violation(
            Some(0),
            aatf::Severity::Error,
            aatf::ViolationKind::Height {
                height: 185,
                fits: vec![],
                room: false,
            },
        )];
        let players = [listed(0, 71801, 50, "ROADTRIP")];
        let teams = [TeamViolations {
            team_id: 718,
            team_name: &coded,
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let lines = drawn_lines(&nodes);
        assert_eq!(lines[0], format!("Team 718 {name}"), "{lines:?}");
        assert!(
            lines.iter().all(|line| !line.contains('\u{11}')),
            "no colour code byte is drawn: {lines:?}"
        );
    }

    // As the draft's `.drow` does, a sentence longer than the row wraps
    // under its own column, so a long check's text stays readable rather
    // than cut off.
    #[test]
    fn a_long_sentence_wraps_rather_than_being_cut() {
        let violations = [
            violation(
                Some(3),
                aatf::Severity::Error,
                aatf::ViolationKind::Height {
                    height: 185,
                    fits: vec![],
                    room: false,
                },
            ),
            violation(
                Some(3),
                aatf::Severity::Error,
                aatf::ViolationKind::MedalSystem {
                    systems: [
                        "ggss regulars",
                        "ggs regulars",
                        "gggs regulars",
                        "gssss regulars",
                        "ggss giants",
                        "ggs giants",
                        "gggs giants",
                        "gssss giants",
                        "ggss manlets",
                        "ggs manlets",
                        "gggs manlets",
                        "gssss manlets",
                    ]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                },
            ),
        ];
        let players = [listed(3, 70103, 3, "NAME")];
        let teams = [TeamViolations {
            team_id: 701,
            team_name: "/3/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, shapes, _) = window.list(&teams, false, vec![]);
        let bounds_of = |text: &str| {
            nodes
                .iter()
                .find(|node| node.role() == Role::Button && node.label() == Some(text))
                .unwrap_or_else(|| panic!("no row {text:?}"))
                .bounds()
                .expect("bounds")
        };
        let height_of = |text: &str| {
            let bounds = bounds_of(text);
            bounds.y1 - bounds.y0
        };
        let one_line = height_of("Height: height 185 takes no bracket");
        let longest_label = "Medal system: the tier counts match none of the medal systems ggss regulars, ggs regulars, gggs regulars, gssss regulars, ggss giants, ggs giants, gggs giants, gssss giants, ggss manlets, ggs manlets, gggs manlets or gssss manlets";
        let longest = height_of(longest_label);
        assert!(
            longest >= 2.0 * one_line,
            "the long sentence wraps onto more rows: {longest} vs {one_line}"
        );

        // The sentence sits in its own column, inside the row: its origin is
        // the row's left edge plus `RULE_COL`, its right edge inside the row.
        let medal_bounds = bounds_of(longest_label);
        let rule_shape = text_shape(&shapes, "Medal system");
        let sentence_shape = text_shape(&shapes, "the tier counts match");
        assert!(
            (f64::from(rule_shape.pos.x) - medal_bounds.x0).abs() <= 0.5,
            "the rule name starts at the row's left edge"
        );
        assert!(
            (f64::from(sentence_shape.pos.x) - (medal_bounds.x0 + f64::from(RULE_COL))).abs()
                <= 0.5,
            "the sentence starts at its column"
        );
        assert!(
            f64::from(sentence_shape.pos.x + sentence_shape.galley.rect.width())
                <= medal_bounds.x1 + 0.5,
            "the sentence does not run past the row's right edge"
        );
    }

    // The `Suggestions` header needs a suggestion row to head, not only
    // `show_suggestions`.
    #[test]
    fn no_suggestions_header_without_a_suggestion() {
        let violations = [violation(
            Some(2),
            aatf::Severity::Error,
            aatf::ViolationKind::Height {
                height: 185,
                fits: vec![],
                room: false,
            },
        )];
        let players = [listed(2, 70202, 2, "NAME")];
        let teams = [TeamViolations {
            team_id: 702,
            team_name: "/a/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, true, vec![]);
        assert!(
            !drawn_lines(&nodes).iter().any(|line| line == "Suggestions"),
            "a team with violations but no suggestion draws no Suggestions header"
        );
    }

    /// A `FieldedNotA` row's click returns the violation's own fields in
    /// `fielded` (the host jumps to the team's row on the named preset's
    /// first fielded formation holding the position); another kind's
    /// `None`.
    #[test]
    fn a_fielded_rows_click_returns_its_fielded_at() {
        let violations = [
            violation(
                Some(3),
                aatf::Severity::Error,
                aatf::ViolationKind::FieldedNotA {
                    preset: 1,
                    position: position("AMF"),
                    rated_a: vec![position("SS")],
                },
            ),
            violation(
                Some(3),
                aatf::Severity::Error,
                aatf::ViolationKind::Height {
                    height: 185,
                    fits: vec![],
                    room: false,
                },
            ),
        ];
        let players = [listed(3, 70103, 3, "NAME")];
        let teams = [TeamViolations {
            team_id: 701,
            team_name: "/3/",
            players: &players,
            violations: &violations,
            starting_eleven: &[],
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let fielded_row = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node.label()
                        == Some(
                            "Fielded position: fielded at AMF in preset 1 without an A \
                             rating (rated A at SS)",
                        )
            })
            .expect("the Fielded position row");
        let (_, _, selected) = window.click_list(centre(fielded_row), &teams, false);
        assert_eq!(
            selected,
            Some(SelectedPlayer {
                team_id: 701,
                slot: 3,
                fielded: Some(FieldedAt {
                    preset: 1,
                    position: position("AMF"),
                }),
            })
        );
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let height_row = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node.label() == Some("Height: height 185 takes no bracket")
            })
            .expect("the Height row");
        let (_, _, selected) = window.click_list(centre(height_row), &teams, false);
        assert_eq!(
            selected,
            Some(SelectedPlayer {
                team_id: 701,
                slot: 3,
                fielded: None,
            })
        );
    }

    /// The bounds of the one drawn line whose text (a Label's value or a
    /// Button's label) starts with `start`.
    fn line_bounds(nodes: &[accesskit::Node], start: &str) -> accesskit::Rect {
        let found: Vec<accesskit::Rect> = nodes
            .iter()
            .filter(|node| {
                let text = match node.role() {
                    Role::Label => node.value(),
                    Role::Button => node.label(),
                    _ => None,
                };
                text.is_some_and(|text| text.starts_with(start))
            })
            .filter_map(accesskit::Node::bounds)
            .collect();
        assert_eq!(found.len(), 1, "one drawn line starts with {start:?}");
        found[0]
    }

    /// The bottom of the lowest player row (a Button: a team row only
    /// reports) whose left edge is `left`, a column's.
    fn column_bottom(nodes: &[accesskit::Node], left: f64) -> f64 {
        nodes
            .iter()
            .filter(|node| node.role() == Role::Button)
            .filter_map(accesskit::Node::bounds)
            .filter(|bounds| bounds.x0 == left)
            .map(|bounds| bounds.y1)
            .reduce(f64::max)
            .expect("a row in the column")
    }

    /// The grouping test's team 734 with one starter made short the same
    /// way: 73401 and 73404 (slots 0 and 3) sit on the bench, and 73402
    /// (slot 1) starts. While editing, the team's own rows stand above both
    /// columns, the starter's group in the left column and the bench's two
    /// in the right, both from the same top; a report keeps one column.
    #[test]
    fn the_starters_groups_stand_left_of_the_benchs() {
        let mut file = fixture();
        for id in [73401, 73404, 73402] {
            let player = file.player_mut(id).expect("rostered");
            assert_eq!(player.basic.height, 185, "premise: {id} is tall");
            player.basic.height = 183;
        }
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let eleven = team.tactics.starting_eleven;
        assert!(eleven.contains(&1), "premise: slot 1 starts");
        assert!(
            !eleven.contains(&0) && !eleven.contains(&3),
            "premise: slots 0 and 3 are on the bench"
        );
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let starter = line_bounds(&nodes, "Player 73402 ");
        let bench = line_bounds(&nodes, "Player 73401 ");
        let team_rows_bottom = line_bounds(&nodes, "Height quota: ").y1;
        assert!(
            starter.x0 < bench.x0,
            "the starter's group left of the bench's: {starter:?}, {bench:?}"
        );
        let starter_bottom = column_bottom(&nodes, starter.x0);
        assert!(
            bench.y0 < starter_bottom,
            "the bench's column starts beside the starter's rows: {bench:?}, {starter_bottom}"
        );
        assert!(
            team_rows_bottom <= starter.y0 && team_rows_bottom <= bench.y0,
            "the team's own rows above both columns: {team_rows_bottom}, {starter:?}, {bench:?}"
        );

        let (nodes, _) = window.frame(vec![], |ui| {
            violations_list(ui, "list", &teams, official_17(), ListUse::Report);
        });
        let headers: Vec<f64> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Label
                    && node.value().is_some_and(|text| {
                        text.starts_with("Player ") || text == "Team-wide issues"
                    })
            })
            .filter_map(accesskit::Node::bounds)
            .map(|bounds| bounds.x0)
            .collect();
        assert_eq!(headers.len(), 4, "three players' headers and the team's");
        assert!(
            headers.iter().all(|&left| left == headers[0]),
            "a report's headers share one left edge: {headers:?}"
        );
    }

    /// How far each painted sentence starting with `sentence` stands right
    /// of its row's left edge: the rule's column.
    fn rule_columns(
        nodes: &[accesskit::Node],
        shapes: &[egui::epaint::ClippedShape],
        sentence: &str,
    ) -> Vec<f64> {
        let rows: Vec<accesskit::Rect> = nodes
            .iter()
            .filter(|node| node.role() == Role::Button)
            .filter_map(accesskit::Node::bounds)
            .collect();
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if shape.galley.job.text.starts_with(sentence) => {
                    Some(shape.pos)
                }
                _ => None,
            })
            .map(|pos| {
                let (x, y) = (f64::from(pos.x), f64::from(pos.y) + 1.0);
                let row = rows
                    .iter()
                    .find(|row| row.x0 <= x && x <= row.x1 && row.y0 <= y && y <= row.y1)
                    .unwrap_or_else(|| panic!("no row holds the sentence at {pos:?}"));
                x - row.x0
            })
            .collect()
    }

    /// The two-column test's team 734, listed in one column: every group
    /// shares one left edge, in roster order, so the starter 73402 (slot 1)
    /// stands between the bench's 73401 (slot 0) above him and 73404 (slot
    /// 3) under him. The rule's column is 140 points there, 190 in two.
    #[test]
    fn one_column_lists_the_groups_in_roster_order_with_a_narrower_rule_column() {
        let mut file = fixture();
        for id in [73401, 73404, 73402] {
            let player = file.player_mut(id).expect("rostered");
            assert_eq!(player.basic.height, 185, "premise: {id} is tall");
            player.basic.height = 183;
        }
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let height_sentence = "height 183 takes no bracket";
        let mut window = Headless::new();

        let (nodes, shapes, _) = window.list(&teams, false, vec![]);
        let two = rule_columns(&nodes, &shapes, height_sentence);
        assert_eq!(two.len(), 3, "premise: three Height rows");
        assert!(
            two.iter().all(|column| (column - 190.0).abs() <= 0.5),
            "two columns keep the 190-point rule column: {two:?}"
        );

        let one_column = ListUse::Editing {
            show_suggestions: false,
            one_column: true,
        };
        let (nodes, shapes, _) = window.list_as(&teams, one_column, vec![]);
        let first_bench = line_bounds(&nodes, "Player 73401 ");
        let starter = line_bounds(&nodes, "Player 73402 ");
        let second_bench = line_bounds(&nodes, "Player 73404 ");
        assert!(
            first_bench.x0 == starter.x0 && starter.x0 == second_bench.x0,
            "one left edge: {first_bench:?}, {starter:?}, {second_bench:?}"
        );
        let group_bottom = |header: accesskit::Rect, next: accesskit::Rect| {
            nodes
                .iter()
                .filter(|node| node.role() == Role::Button)
                .filter_map(accesskit::Node::bounds)
                .filter(|row| row.y0 >= header.y1 && row.y1 <= next.y0 + 0.5)
                .map(|row| row.y1)
                .reduce(f64::max)
                .expect("the group's rows")
        };
        assert!(
            starter.y0 >= group_bottom(first_bench, starter),
            "the starter's group under 73401's: {starter:?}, {first_bench:?}"
        );
        assert!(
            second_bench.y0 >= group_bottom(starter, second_bench),
            "73404's group under the starter's: {second_bench:?}, {starter:?}"
        );
        let one = rule_columns(&nodes, &shapes, height_sentence);
        assert_eq!(one.len(), 3, "premise: three Height rows");
        assert!(
            one.iter().all(|column| (column - 140.0).abs() <= 0.5),
            "one column narrows the rule column to 140: {one:?}"
        );
    }

    /// The PES 17 fixture with each of `ids`, 185 cm players of team 734,
    /// made 183, which no Red bracket takes: a `Height` error on each, beside
    /// the team's own tier counts and tall quota.
    fn short_734(ids: &[u32]) -> pes_savefile::EditFile {
        let mut file = fixture();
        for &id in ids {
            let player = file.player_mut(id).expect("rostered");
            assert_eq!(player.basic.height, 185, "premise: {id} is tall");
            player.basic.height = 183;
        }
        file
    }

    /// The two-column test's team 734 (the bench's 73401 and 73404 and the
    /// starter 73402 made short) in every list use, two columns, one and a
    /// report: `Team-wide issues` and its rows, down to the tall quota, the
    /// team's last, stand above every player's group, since a tier count is
    /// the team's first concern.
    // SE-GUI-30
    #[test]
    fn the_team_wide_issues_stand_above_every_players_group() {
        let file = short_734(&[73401, 73404, 73402]);
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        for list_use in [
            ListUse::Editing {
                show_suggestions: false,
                one_column: false,
            },
            ListUse::Editing {
                show_suggestions: false,
                one_column: true,
            },
            ListUse::Report,
        ] {
            let (nodes, _, _) = window.list_as(&teams, list_use, vec![]);
            let heading = line_bounds(&nodes, "Team-wide issues");
            let quota = line_bounds(&nodes, "Height quota: ");
            assert!(
                heading.y1 <= quota.y0,
                "{list_use:?}: the quota under its heading: {heading:?}, {quota:?}"
            );
            let player_heads: Vec<accesskit::Rect> = nodes
                .iter()
                .filter(|node| {
                    node.role() == Role::Label
                        && node.value().is_some_and(|text| text.starts_with("Player "))
                })
                .filter_map(accesskit::Node::bounds)
                .collect();
            assert_eq!(player_heads.len(), 3, "{list_use:?}: three players' heads");
            for head in player_heads {
                assert!(
                    quota.y1 <= head.y0,
                    "{list_use:?}: a player's group under the team's rows: {quota:?}, {head:?}"
                );
            }
        }
    }

    /// The texts the pointer resting at `pos` adds to the list's frame, a
    /// tooltip's: every value thirty frames with the pointer there draw
    /// that the frame with the pointer away did not (bounded, the tooltip
    /// delay 0, so a missing tooltip fails rather than hangs).
    fn hover_texts(
        window: &mut Headless,
        teams: &[TeamViolations],
        pos: egui::Pos2,
    ) -> Vec<String> {
        window
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let values = |nodes: &[accesskit::Node]| -> Vec<String> {
            nodes
                .iter()
                .filter_map(|node| node.value())
                .map(str::to_owned)
                .collect()
        };
        let (away, _, _) = window.list(
            teams,
            false,
            vec![egui::Event::PointerMoved(egui::Pos2::ZERO)],
        );
        let away = values(&away);
        window.list(teams, false, vec![egui::Event::PointerMoved(pos)]);
        let mut added = Vec::new();
        for _ in 0..30 {
            let (nodes, _, _) = window.list(teams, false, vec![]);
            for value in values(&nodes) {
                if !away.contains(&value) && !added.contains(&value) {
                    added.push(value);
                }
            }
        }
        added
    }

    /// Team 734 with its two short bench players: hovering a player's row
    /// (a `Height` one) shows `Click to switch to this player`, what its
    /// click does, and hovering a team-wide row (its Gold tier count)
    /// shows no tooltip.
    // SE-GUI-30
    #[test]
    fn a_players_row_hovers_what_its_click_does_and_a_team_row_nothing() {
        let file = short_734(&[73401, 73404]);
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let player_row = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Button
                    && node.label() == Some("Height: height 183 takes no bracket (185 has room)")
            })
            .expect("a player's row")
            .clone();
        let team_row = nodes
            .iter()
            .find(|node| {
                node.role() == Role::Label
                    && node.value() == Some("Tier count: 2 Gold players, where the rules want 1")
            })
            .expect("a team-wide row")
            .clone();
        assert_eq!(
            hover_texts(&mut window, &teams, centre(&player_row)),
            ["Click to switch to this player"],
            "a player's row"
        );
        assert_eq!(
            hover_texts(&mut window, &teams, centre(&team_row)),
            Vec::<String>::new(),
            "a team-wide row"
        );
    }

    /// Whether a `Rect` shape's rect is `bounds`' row (within a point).
    fn paints_row(rect: egui::Rect, bounds: accesskit::Rect) -> bool {
        (f64::from(rect.min.x) - bounds.x0).abs() <= 1.0
            && (f64::from(rect.min.y) - bounds.y0).abs() <= 1.0
            && (f64::from(rect.max.x) - bounds.x1).abs() <= 1.0
            && (f64::from(rect.max.y) - bounds.y1).abs() <= 1.0
    }

    // A row under the pointer is outlined in the hovered widget's stroke,
    // never filled: a fill under the row's coloured words left an error's
    // red rule name hard to read. Team 734 with two of its regulars made
    // short: one's `Height` row hovered strokes its rect and fills none;
    // the other's, not hovered, paints neither.
    #[test]
    fn a_hovered_row_is_outlined_not_filled() {
        let file = short_734(&[73401, 73404]);
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, false, vec![]);
        let mut rows: Vec<accesskit::Rect> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::Button
                    && node.label() == Some("Height: height 183 takes no bracket (185 has room)")
            })
            .filter_map(accesskit::Node::bounds)
            .collect();
        rows.sort_by(|a, b| a.y0.partial_cmp(&b.y0).expect("ordered bounds"));
        let [hovered, other] = rows.as_slice() else {
            panic!("premise: both Height rows drawn");
        };
        let at = egui::pos2(
            ((hovered.x0 + hovered.x1) / 2.0) as f32,
            ((hovered.y0 + hovered.y1) / 2.0) as f32,
        );
        let (_, shapes, _) = window.list(&teams, false, vec![egui::Event::PointerMoved(at)]);
        let widgets = &window.ctx.style_of(window.ctx.theme()).visuals.widgets;
        let rect_of = |bounds: accesskit::Rect| {
            shapes
                .iter()
                .filter_map(move |clipped| match &clipped.shape {
                    egui::Shape::Rect(shape) if paints_row(shape.rect, bounds) => Some(shape),
                    _ => None,
                })
        };
        assert!(
            rect_of(*hovered).all(|shape| shape.fill != widgets.hovered.bg_fill),
            "the hovered row paints no hovered-widget fill"
        );
        let outline = rect_of(*hovered)
            .find(|shape| {
                shape.stroke.width > 0.0 && shape.stroke.color != egui::Color32::TRANSPARENT
            })
            .expect("the hovered row's outline");
        assert_eq!(
            outline.stroke.color, widgets.hovered.bg_stroke.color,
            "the outline in the hovered widget's stroke colour"
        );
        assert_eq!(
            outline.stroke.width, widgets.hovered.bg_stroke.width,
            "the outline in the hovered widget's stroke width"
        );
        assert!(
            rect_of(*other).all(|shape| {
                shape.fill != widgets.hovered.bg_fill
                    && (shape.stroke.width == 0.0
                        || shape.stroke.color == egui::Color32::TRANSPARENT
                        || shape.stroke != widgets.hovered.bg_stroke)
            }),
            "the unhovered row paints neither fill nor outline"
        );
    }

    /// The suggestions part stands in the same two columns: 734's starters'
    /// `Unused` rows (slots 9, 10, 14 and 22 all start) left, and those of
    /// 73401 (slot 0, on the bench), his Weak Foot Usage lowered to 1,
    /// right.
    #[test]
    fn the_suggestions_stand_in_the_same_two_columns() {
        let mut file = fixture();
        let player = file.player_mut(73401).expect("rostered");
        assert_eq!(
            player.stats.weak_foot_usage, 1,
            "premise: 73401's Weak Foot Usage is 1 on the save's 0-3 scale, 2 as the check reads it"
        );
        player.stats.weak_foot_usage = 0;
        let players = listed_players(&file, 734);
        let violations = check(&file, 734);
        let team = file.team(734).expect("team 734");
        let eleven = team.tactics.starting_eleven;
        assert!(eleven.contains(&9), "premise: slot 9 starts");
        assert!(!eleven.contains(&0), "premise: slot 0 is on the bench");
        let teams = [TeamViolations {
            team_id: 734,
            team_name: &team.name,
            players: &players,
            violations: &violations,
            starting_eleven: &team.tactics.starting_eleven,
        }];
        let mut window = Headless::new();
        let (nodes, _, _) = window.list(&teams, true, vec![]);
        assert!(
            drawn_lines(&nodes)
                .contains(&"Unused: Weak Foot Usage 1, where the rules allow 2".to_string()),
            "premise: 73401's suggestion is listed"
        );
        let heading = line_bounds(&nodes, "Suggestions");
        let starter = line_bounds(&nodes, "Player 73410 ");
        let bench = line_bounds(&nodes, "Player 73401 ");
        assert!(
            starter.y0 > heading.y0 && bench.y0 > heading.y0,
            "premise: both groups are under the Suggestions heading"
        );
        assert!(
            starter.x0 < bench.x0,
            "the starter's suggestions left of the bench player's: {starter:?}, {bench:?}"
        );
        assert!(
            bench.y0 < starter.y1 && starter.y0 < bench.y1,
            "both columns start from the same top: {starter:?}, {bench:?}"
        );
    }
}
