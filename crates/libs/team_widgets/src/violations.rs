//! The AATF results list, drafted in `docs/prototypes/tools/save_editor.js`
//! (`aatfPanel`, `violationSentence`, `playerHead`): one team's check grouped
//! per player, a row's click reporting the player to select. The AATF panel,
//! the Live AATF dock and the Team creator all draw this list.

use pes_savefile::interchange::team_toml::labels::title_case;

/// The failing glyph (`✖`, U+2716): egui's bundled emoji fonts hold
/// it, and the theme keeps those fonts in the fallback chain. The icon
/// font's subset does not hold it.
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
    /// codes included), the header draws it code-free.
    pub name: &'a str,
}

/// One team's check: its players by slot and `aatf::Rules::check_team`'s violations, in the
/// order the check returned them.
pub struct TeamViolations<'a> {
    /// The team id.
    pub team_id: u32,
    /// The team name.
    pub team_name: &'a str,
    /// The rostered players, for the group headers.
    pub players: &'a [ListedPlayer<'a>],
    /// The violations `check_team` returned, in its order.
    pub violations: &'a [aatf::Violation],
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
    },
    /// A report over several teams, read and copied out (Multi-team
    /// AATF): every team under its id and name, `All clean` under one with
    /// no rows so it reads as checked rather than missed; the rows only
    /// report, and no suggestion is listed.
    Report,
}

/// The AATF results of `teams` under `ruleset`, the rules they were checked
/// against (the rows name its tiers, brackets and systems by their labels),
/// grouped per player, as `list_use` says. Returns the player whose row was
/// clicked this frame.
pub fn violations_list(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    teams: &[TeamViolations<'_>],
    ruleset: &aatf::Ruleset,
    list_use: ListUse,
) -> Option<SelectedPlayer> {
    ui.push_id(id_salt, |ui| {
        let mut selected = None;
        let selects = matches!(list_use, ListUse::Editing { .. });
        for team in teams {
            let groups = issue_groups(team.violations);
            match list_use {
                ListUse::Editing { .. } => {
                    // A team the check found nothing wrong with gets no header of its own.
                    if groups.is_empty() {
                        continue;
                    }
                    ui.strong(format!("Team {} {}", team.team_id, team.team_name));
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
            for (slot, violations) in groups {
                group_header(ui, team, slot);
                for violation in violations {
                    if let Some(player) = row(ui, team.team_id, violation, ruleset, selects) {
                        selected = Some(player);
                    }
                }
            }
        }
        let any_suggestions = teams
            .iter()
            .flat_map(|team| team.violations)
            .any(|v| v.severity == aatf::Severity::Suggestion);
        let suggestions_asked = matches!(
            list_use,
            ListUse::Editing {
                show_suggestions: true
            }
        );
        if suggestions_asked && any_suggestions {
            ui.strong("Suggestions");
            for team in teams {
                for slot in slot_order(
                    team.violations
                        .iter()
                        .filter(|v| v.severity == aatf::Severity::Suggestion),
                ) {
                    group_header(ui, team, slot);
                    for violation in team
                        .violations
                        .iter()
                        .filter(|v| v.slot == slot && v.severity == aatf::Severity::Suggestion)
                    {
                        if let Some(player) = row(ui, team.team_id, violation, ruleset, selects) {
                            selected = Some(player);
                        }
                    }
                }
            }
        }
        selected
    })
    .inner
}

/// A team's Error and Warning violations in the list's order: one group
/// per slot, in the check's order (players by slot, the team's own rows
/// last, `None`), errors before warnings inside a group. A host writing the
/// list out as text walks this, so the text reads as the list does.
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

/// The slots the given violations fall on, in first-appearance order (the
/// check's: players by slot, the team's own rows last).
fn slot_order<'a>(violations: impl Iterator<Item = &'a aatf::Violation>) -> Vec<Option<u8>> {
    let mut order = Vec::new();
    for violation in violations {
        if !order.contains(&violation.slot) {
            order.push(violation.slot);
        }
    }
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
pub fn violation_line(
    violation: &aatf::Violation,
    team_id: u32,
    ruleset: &aatf::Ruleset,
) -> String {
    format!(
        "{}: {}",
        rule_name(&violation.kind),
        sentence(violation, team_id, ruleset)
    )
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
        K::InjuryResistance { .. } => "Injury resistance",
        K::WeakFootUsage { .. } => "Weak foot usage",
        K::WeakFootAccuracy { .. } => "Weak foot accuracy",
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

/// The rule name's column, the draft's `.f`: 190 points of proportional text,
/// cut with an ellipsis when longer, never wrapping.
const RULE_COL: f32 = 190.0;

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

/// One row, the draft's `.drow`: the rule's name in `RULE_COL` points of
/// severity-colored proportional text, one line cut with an ellipsis (the
/// draft's `text-overflow: ellipsis`), then `sentence` in the strong text
/// color, wrapped so a long custom check's text stays readable; the whole row
/// one click target. While `selects`, a player's row returns
/// `SelectedPlayer` on click; a team row, or any row of a list that does
/// not select, only hovers.
fn row(
    ui: &mut egui::Ui,
    team_id: u32,
    violation: &aatf::Violation,
    ruleset: &aatf::Ruleset,
    selects: bool,
) -> Option<SelectedPlayer> {
    let (rule_color, strong_color) = {
        let visuals = ui.visuals();
        (
            match violation.severity {
                aatf::Severity::Error => visuals.error_fg_color,
                aatf::Severity::Warning => visuals.warn_fg_color,
                aatf::Severity::Suggestion => visuals.text_color(),
            },
            visuals.strong_text_color(),
        )
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let rule_galley = layout(
        ui,
        rule_name(&violation.kind).to_string(),
        font.clone(),
        rule_color,
        egui::text::TextWrapping::truncate_at_width(RULE_COL),
    );
    let text = sentence(violation, team_id, ruleset);
    let sentence_galley = layout(
        ui,
        text.clone(),
        font,
        strong_color,
        egui::text::TextWrapping::wrap_at_width((ui.available_width() - RULE_COL).max(0.0)),
    );
    let height = rule_galley.size().y.max(sentence_galley.size().y);
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
    let label = violation_line(violation, team_id, ruleset);
    // A report's rows select nothing, so no tooltip talks of selecting.
    let hover = match (select, selects) {
        (Some(_), _) => Some("Select this player"),
        (None, true) => Some("A team-level violation"),
        (None, false) => None,
    };
    row_paint(
        ui,
        response,
        &[
            egui::Shape::galley(rect.min, rule_galley, rule_color),
            egui::Shape::galley(
                rect.min + egui::vec2(RULE_COL, 0.0),
                sentence_galley,
                strong_color,
            ),
        ],
        &label,
        select,
        hover,
    )
}

/// The paint and click tail the violations list's rows and the Save
/// editor's comparator lines share: `shapes` painted in order under a
/// hovered row's fill, `label` the AccessKit label under the `Button`
/// role on a clickable row, `Label` on one that only reports, and `hover`
/// the tooltip; a `select` row's click reports its player.
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
            ui.painter()
                .rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
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
/// one arm per `ViolationKind`, each stat and card named as the game names
/// it (Attacking Prowess) and each tier, bracket and height system by its
/// label in `ruleset`, the rules the check ran under. `team_id` is the
/// team's, for the rules whose expected value it sets (the roster order's
/// `team id x 100 + slot + 1`). The row's text after the rule's name; a host
/// writing the list out as text writes it so.
pub fn sentence(violation: &aatf::Violation, team_id: u32, ruleset: &aatf::Ruleset) -> String {
    use aatf::ViolationKind as K;
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
            let stat = title_case(stat.label());
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
        K::Form { value, target } => format!("form {value}, where the rules want {target}"),
        K::InjuryResistance { value, max } => bound("injury resistance", *value, *max),
        K::WeakFootUsage { value, max } => bound("weak foot usage", *value, *max),
        K::WeakFootAccuracy { value, max } => bound("weak foot accuracy", *value, *max),
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
            format!("the required card {} is missing", title_case(card.label()))
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
        } => format!(
            "{count} players in the {} bracket of the {} system, where its quota is {quota}",
            label_of(
                ruleset
                    .heights
                    .brackets
                    .iter()
                    .map(|bracket| (bracket.id.as_str(), bracket.label.as_str())),
                bracket
            ),
            label_of(
                ruleset
                    .heights
                    .systems
                    .iter()
                    .map(|system| (system.id.as_str(), system.label.as_str())),
                system
            )
        ),
        K::MedalSystem { systems } => format!(
            "the tier counts match none of the medal systems {}",
            joined(systems, "or")
        ),
        K::ExtraHeight { used, total } => {
            format!("{used} cm of extra height, over the pool of {total}")
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
        } => unused_sentence(allowance, *used, *allowed),
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
/// under the group's name.
fn unused_sentence(allowance: &str, used: u8, allowed: u8) -> String {
    match allowance {
        "cards" => format!("{used} of {allowed} paid cards used"),
        "a_positions" => format!("{used} of {allowed} A positions used"),
        "free_cards" => format!("{used} of {allowed} free cards held"),
        "weak_foot_usage" => format!("weak foot usage {used}, where the rules allow {allowed}"),
        "weak_foot_accuracy" => {
            format!("weak foot accuracy {used}, where the rules allow {allowed}")
        }
        "injury_resistance" => {
            format!("injury resistance {used}, where the rules allow {allowed}")
        }
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
    use crate::test_support::{Headless, centre, painted_text_color, pointer_at, text_shape};

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
            let mut selected = None;
            let (nodes, shapes) = self.frame(events, |ui| {
                selected = Some(violations_list(
                    ui,
                    "list",
                    teams,
                    official(),
                    ListUse::Editing { show_suggestions },
                ));
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

    /// A `ListedPlayer` (the roster slot, id, number and name).
    fn listed(slot: u8, id: u32, number: u16, name: &str) -> ListedPlayer<'_> {
        ListedPlayer {
            slot,
            id,
            number,
            name,
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
            .map(|(i, slot)| ListedPlayer {
                slot: u8::try_from(i).expect("a slot fits u8"),
                id: slot.player_id,
                number: slot.number,
                name: file
                    .player(slot.player_id)
                    .expect("a rostered player")
                    .name
                    .as_str(),
            })
            .collect()
    }

    /// Team `id`'s check under the official ruleset on `file`.
    fn check(file: &pes_savefile::EditFile, id: u32) -> Vec<aatf::Violation> {
        let view = aatf::TeamView::new(file, id).expect("the team views");
        aatf::Rules::official()
            .check_team(&view)
            .expect("the check runs")
    }

    /// The official ruleset's check on team 734 ("/lit/") of the PES 17
    /// fixture save, with two of its tall regulars (slots 0 and 3, 185
    /// cm) set to 183, which no Red bracket takes: 14 violations, a
    /// `Height` error on each of the two, the team's own (three
    /// `TierCount`, and the red tall quota at 8 of 10) and eight `Unused`
    /// suggestions on slots 9, 10, 14 and 22. The list draws each player's
    /// group in roster order, then the team's.
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
        }];
        let mut window = Headless::new();
        let (nodes, _, selected) = window.list(&teams, false, vec![]);
        assert_eq!(selected, None);
        assert_eq!(
            drawn_lines(&nodes),
            [
                "Team 734 /lit/",
                "Player 73401  49 Muh Spooks",
                "Height: height 183 takes no bracket (185 has room)",
                "Player 73404  45 Yuki-kun ^~^",
                "Height: height 183 takes no bracket (185 has room)",
                "Team-wide issues",
                "Tier count: 2 Gold players, where the rules want 1",
                "Tier count: 0 Bronze players, where the rules want 2",
                "Tier count: 19 Regular players, where the rules want 18",
                "Height quota: 8 players in the Tall bracket of the Red system, where its quota is 10",
            ]
        );
    }

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
            },
            TeamViolations {
                team_id: 714,
                team_name: &team_714.name,
                players: &players_714,
                violations: &suggestions_714,
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
                    official(),
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
                "Player 73401  49 Muh Spooks",
                height,
                "Player 73404  45 Yuki-kun ^~^",
                height,
                "Team-wide issues",
                "Tier count: 2 Gold players, where the rules want 1",
                "Tier count: 0 Bronze players, where the rules want 2",
                "Tier count: 19 Regular players, where the rules want 18",
                "Height quota: 8 players in the Tall bracket of the Red system, where its quota is 10",
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
                "Unused: weak foot usage 2, where the rules allow 4",
                "Player 73411  1 Start With the Greeks",
                "Unused: 5 of 6 paid cards used",
                "Unused: weak foot usage 2, where the rules allow 4",
                "Player 73415  87 A.M. Pinecone",
                "Unused: 4 of 5 paid cards used",
                "Unused: weak foot usage 2, where the rules allow 4",
                "Player 73423  2 my diary desu",
                "Unused: 4 of 5 paid cards used",
                "Unused: weak foot usage 2, where the rules allow 4",
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
        let sent = |v: &aatf::Violation| sentence(v, 701, official());
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
                "weak foot usage 3, under the 4 the rules require",
            ),
            (
                K::WeakFootAccuracy { value: 3, max: 4 },
                "weak foot accuracy 3, under the 4 the rules require",
            ),
            (
                K::InjuryResistance { value: 1, max: 3 },
                "injury resistance 1, under the 3 the rules require",
            ),
            (
                K::WeakFootUsage { value: 4, max: 2 },
                "weak foot usage 4, over the 2 the rules allow",
            ),
            (
                K::WeakFootUsage { value: 3, max: 2 },
                "weak foot usage 3, over the 2 the rules allow",
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
                official()
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
                official(),
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
        let dbc7 = dbc7.ruleset();
        let positions = |names: &[&str]| names.iter().map(|name| position(name)).collect();
        let unused = |allowance: &str, used, allowed| K::Unused {
            allowance: allowance.to_string(),
            used,
            allowed,
        };
        let a26 = official();
        let rows: Vec<(&aatf::Ruleset, aatf::Severity, K, &str, &str)> = vec![
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
                "form 8, where the rules want 4",
            ),
            (
                a26,
                Error,
                K::InjuryResistance { value: 3, max: 1 },
                "Injury resistance",
                "injury resistance 3, over the 1 the rules allow",
            ),
            (
                a26,
                Error,
                K::WeakFootUsage { value: 3, max: 4 },
                "Weak foot usage",
                "weak foot usage 3, under the 4 the rules require",
            ),
            (
                a26,
                Error,
                K::WeakFootAccuracy { value: 3, max: 2 },
                "Weak foot accuracy",
                "weak foot accuracy 3, over the 2 the rules allow",
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
                    card: aatf::Card::try_from("captaincy".to_string()).expect("a card"),
                },
                "Required card",
                "the required card Captaincy is missing",
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
                },
                "Height quota",
                "9 players in the Mid bracket of the GigaHeight system, where its quota is 10",
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
                "weak foot usage 2, where the rules allow 4",
            ),
            (
                a26,
                Suggestion,
                unused("weak_foot_accuracy", 2, 4),
                "Unused",
                "weak foot accuracy 2, where the rules allow 4",
            ),
            (
                a26,
                Suggestion,
                unused("injury_resistance", 1, 3),
                "Unused",
                "injury resistance 1, where the rules allow 3",
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
        assert_eq!(kinds.len(), 36, "one row or more for each of the 36 kinds");
        for (ruleset, severity, kind, name, expected) in rows {
            // Slot 4 of team 734: the roster order's player 73405.
            let violation = violation(Some(4), severity, kind);
            assert_eq!(rule_name(&violation.kind), name, "{:?}", violation.kind);
            let text = sentence(&violation, 734, ruleset);
            assert_eq!(text, expected, "{:?}", violation.kind);
            let camel = name
                .as_bytes()
                .windows(2)
                .any(|pair| pair[0].is_ascii_lowercase() && pair[1].is_ascii_uppercase());
            assert!(!name.contains('_') && !camel, "{name:?}");
            assert!(!text.contains('_') && !text.contains('['), "{text:?}");
            let words: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).collect();
            let labelled = ruleset
                .tiers
                .iter()
                .map(|tier| (&tier.id, &tier.label))
                .chain(
                    ruleset
                        .heights
                        .brackets
                        .iter()
                        .map(|bracket| (&bracket.id, &bracket.label)),
                )
                .chain(
                    ruleset
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
        };
        assert!(
            official().heights.systems.iter().all(|s| s.id != "purple"),
            "premise: Autumn 26 has no purple system"
        );
        assert_eq!(
            sentence(
                &violation(None, aatf::Severity::Error, kind),
                734,
                official()
            ),
            "0 players in the Tall bracket of the purple system, where its quota is 10"
        );
    }

    // A team the check found nothing wrong with gets no header of its own.
    #[test]
    fn a_team_with_no_violations_draws_no_header() {
        let empty = TeamViolations {
            team_id: 1,
            team_name: "EMPTY",
            players: &[],
            violations: &[],
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
}
