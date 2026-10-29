//! The skill, COM playing style and playing style pickers, drafted in
//! `docs/prototypes/tools/save_editor.js` (`skillsPanel`, `comStylesPanel`,
//! `cardBadges` and the playing-style combo).
//! A player's skills and COM playing styles are his *cards*; the ruleset's
//! tiers each say how many paid ones he may hold, so the pickers show what
//! the tier he is read as leaves him.

use pes_savefile::interchange::team_toml::labels::{self, title_case};
use pes_savefile::model::playstyle::PlayStyle;
use pes_savefile::schema::fields::PlayerField;
use pes_savefile::schema::{playstyle, schema_for};
use studio_core::widgets::{checkbox, step_choice, wheel_steps};

/// A skills or COM styles column's width (the draft's `.skcols` 158px):
/// the host sizes the panels that hold the two columns from it.
pub const CARD_COLUMN_WIDTH: f32 = 158.0;

/// The gap between a card panel's two columns (the host's panels keep
/// the same 14 between one another).
const CARD_COLUMN_GAP: f32 = 14.0;

/// The width `card_columns` needs for its two columns: the host sizes
/// the column that holds them from it.
pub const TWO_COLUMN_WIDTH: f32 = 2.0 * CARD_COLUMN_WIDTH + CARD_COLUMN_GAP;

/// The badge's tier: its label and the player's card economy under it
/// (the host's `aatf::Rules::card_limits` result).
#[derive(Clone, Copy)]
pub struct TierCards<'a> {
    /// The tier's display label ("Gold", "Regular", ...).
    pub label: &'a str,
    /// The player's card economy read as this tier.
    pub limits: &'a aatf::CardLimits,
}

/// The Cards left row's one badge: the player's own tier under the active ruleset.
#[derive(Clone, Copy)]
pub enum CardsBadge<'a> {
    /// The tier he is read as, with his card economy under it.
    Tier(TierCards<'a>),
    /// No tier's condition holds for him (`check_team`'s `NoTier`).
    NoTier,
}

/// `names`' cards' checkboxes in two columns `CARD_COLUMN_WIDTH` wide,
/// `CARD_COLUMN_GAP` apart, filled column by column (the draft's
/// `.skcols`): `shown` is the indices into `names` to draw, in draw
/// order; every card is drawn on every
/// version; one the version cannot store stays in its cell, disabled
/// with a `PES NN+` tooltip naming the first version that has it.
/// Returns whether a card was ticked or unticked this frame.
pub fn card_columns(
    ui: &mut egui::Ui,
    version: pes_version::PesVersion,
    cards: &mut [bool],
    names: &[&'static str],
    shown: &[u8],
    field: impl Fn(u8) -> PlayerField,
) -> bool {
    let fields = schema_for(version).player_field_set();
    let mut changed = false;
    let per_column = shown.len().div_ceil(2).max(1);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = CARD_COLUMN_GAP;
        for chunk in shown.chunks(per_column) {
            ui.vertical(|ui| {
                ui.set_width(CARD_COLUMN_WIDTH);
                for &index in chunk {
                    let field = field(index);
                    let index = usize::from(index);
                    if fields.contains(&field) {
                        changed |=
                            checkbox(ui, &mut cards[index], title_case(names[index])).changed();
                    } else {
                        // A card the version cannot store stays, disabled,
                        // its tooltip naming the first version that has it.
                        ui.add_enabled(false, |ui: &mut egui::Ui| {
                            checkbox(ui, &mut cards[index], title_case(names[index]))
                        })
                        .on_disabled_hover_text(format!("PES {}+", first_with(field).number()));
                    }
                }
            });
        }
    });
    changed
}

/// The Cards left label and `badge`'s text, drawn inline in the caller's
/// row (the draft's `cardBadges`): `Cards left for` then the tier's
/// `label: left of limit` (no `cards` after the count, which the label
/// before it says), its part in the error color when overdrawn;
/// `Cards left:` then `No tier matches`, weak, when no tier's
/// condition holds for him.
pub fn cards_left(ui: &mut egui::Ui, badge: &CardsBadge<'_>) {
    match badge {
        CardsBadge::Tier(tier) => {
            ui.weak("Cards left for");
            let left = i32::from(tier.limits.limit) - i32::from(tier.limits.paid);
            let mut text =
                egui::RichText::new(format!("{}: {} of {}", tier.label, left, tier.limits.limit));
            if left < 0 {
                text = text.color(ui.visuals().error_fg_color);
            }
            ui.label(text).on_hover_text(format!(
                "{}: {} paid of {}",
                tier.label, tier.limits.paid, tier.limits.limit
            ));
        }
        CardsBadge::NoTier => {
            ui.weak("Cards left:");
            ui.weak("No tier matches");
        }
    }
}

/// The text the picker shows for a stored style index: the style's label
/// after `playstyle::decode`, the number when the version's list cannot
/// decode it (so the combo still says what the save holds).
pub fn playing_style_text(version: pes_version::PesVersion, stored: u8) -> String {
    playstyle::decode(version, stored)
        .map(style_label)
        .unwrap_or_else(|_| stored.to_string())
}

/// The playing-style choice, the version's styles only. Returns whether it changed.
pub fn playing_style_picker(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    version: pes_version::PesVersion,
    style: &mut u8,
) -> bool {
    ui.push_id(id_salt, |ui| {
        // The version's styles in its stored order, not the canonical one:
        // that is the order the game's own Edit-mode list shows, and a member
        // compares the combo against it. A stored value indexes the version's
        // list, whose holes are skipped.
        let styles: Vec<(u8, String)> = (0..=u8::MAX)
            .filter_map(|index| {
                let style = playstyle::decode(version, index).ok()?;
                Some((index, style_label(style)))
            })
            .collect();
        let selected = playing_style_text(version, *style);
        let mut changed = false;
        let combo = egui::ComboBox::from_id_salt("playing style")
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (index, label) in &styles {
                    changed |= ui.selectable_value(style, *index, label).changed();
                }
            })
            .response;
        let indices = styles.iter().map(|(index, _)| *index);
        if let Some(next) = step_choice(indices, style, wheel_steps(ui, &combo)) {
            *style = next;
            changed = true;
        }
        changed
    })
    .inner
}

/// The oldest version storing `field` (for the disabled cell's `PES NN+`
/// tooltip; every card is stored by some version).
fn first_with(field: PlayerField) -> pes_version::PesVersion {
    pes_savefile::schema::first_version_with(field).expect("every card is stored by some version")
}

/// `style`'s label: its Team TOML name, title-cased.
fn style_label(style: PlayStyle) -> String {
    let at = PlayStyle::ALL
        .iter()
        .position(|canonical| *canonical == style)
        .expect("`PlayStyle::ALL` lists every style");
    title_case(labels::PLAY_STYLES[at])
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use egui::accesskit::{self, Role};
    use pes_savefile::interchange::team_toml::labels;
    use pes_savefile::model::player::PlayerSkills;
    use pes_savefile::model::playstyle::PlayStyle;
    use pes_savefile::schema::fields::PlayerField;
    use pes_savefile::schema::{playstyle, schema_for};
    use pes_version::PesVersion;

    use super::*;
    use crate::test_support::{Headless, centre, painted_text_color, pointer_at};

    /// The picker's text for a stored style is the version's own: PES 17's
    /// stored 15 is Build Up, PES 21's Offensive Fullback; a value the
    /// version's list cannot decode reads as its number.
    #[test]
    fn a_stored_style_reads_as_the_versions_style() {
        assert_eq!(playing_style_text(PesVersion::Pes17, 15), "Build Up");
        assert_eq!(
            playing_style_text(PesVersion::Pes21, 15),
            "Offensive Fullback"
        );
        assert_eq!(playing_style_text(PesVersion::Pes17, 22), "22");
    }

    impl Headless {
        /// One frame of the card columns over `skills` — the skills' then
        /// the COM styles', as the host draws them — returning the nodes
        /// and whether the frame changed a card.
        fn columns(
            &mut self,
            version: PesVersion,
            skills: &mut PlayerSkills,
            events: Vec<egui::Event>,
        ) -> (Vec<accesskit::Node>, bool) {
            self.columns_at(None, version, skills, events)
        }

        /// `columns` with `time` as the frame's timestamp (for tooltips).
        fn columns_at(
            &mut self,
            time: Option<f64>,
            version: PesVersion,
            skills: &mut PlayerSkills,
            events: Vec<egui::Event>,
        ) -> (Vec<accesskit::Node>, bool) {
            let mut changed = None;
            let (nodes, _) = self.frame_at(time, events, |ui| {
                changed = Some(
                    card_columns(
                        ui,
                        version,
                        &mut skills.skills,
                        &labels::SKILLS,
                        &all_indices(&labels::SKILLS),
                        PlayerField::Skill,
                    ) | card_columns(
                        ui,
                        version,
                        &mut skills.com_styles,
                        &labels::COM_STYLES,
                        &all_indices(&labels::COM_STYLES),
                        PlayerField::ComStyle,
                    ),
                );
            });
            (nodes, changed.expect("the columns ran"))
        }

        /// A click at `pos` over the columns: the press frame, then the
        /// release frame whose result is returned (`clicked()` lands on
        /// the release).
        fn click_columns(
            &mut self,
            pos: egui::Pos2,
            version: PesVersion,
            skills: &mut PlayerSkills,
        ) -> (Vec<accesskit::Node>, bool) {
            self.columns(version, skills, pointer_at(pos, true));
            self.columns(version, skills, pointer_at(pos, false))
        }

        /// One frame of `cards_left` over `badge`, returning the nodes and
        /// the painted shapes.
        fn badge(
            &mut self,
            badge: CardsBadge,
        ) -> (Vec<accesskit::Node>, Vec<egui::epaint::ClippedShape>) {
            self.frame_at(None, Vec::new(), |ui| {
                ui.horizontal(|ui| cards_left(ui, &badge));
            })
        }

        /// One frame of `playing_style_picker` over `style`, returning the
        /// nodes and whether the frame changed the style.
        fn style_picker(
            &mut self,
            version: PesVersion,
            style: &mut u8,
            events: Vec<egui::Event>,
        ) -> (Vec<accesskit::Node>, bool) {
            let mut changed = None;
            let (nodes, _) = self.frame(events, |ui| {
                changed = Some(playing_style_picker(ui, "style", version, style));
            });
            (nodes, changed.expect("the picker ran"))
        }

        /// A click at `pos` over the style picker (press frame, then release).
        fn click_style(
            &mut self,
            pos: egui::Pos2,
            version: PesVersion,
            style: &mut u8,
        ) -> (Vec<accesskit::Node>, bool) {
            self.style_picker(version, style, pointer_at(pos, true));
            self.style_picker(version, style, pointer_at(pos, false))
        }
    }

    /// `names`' whole index range, the pickers' `shown` when no panel
    /// splits the kind.
    fn all_indices(names: &[&'static str]) -> Vec<u8> {
        (0..u8::try_from(names.len()).expect("a label table is byte-indexed")).collect()
    }

    /// The widget of `role` labeled `label`.
    fn entry<'a>(nodes: &'a [accesskit::Node], role: Role, label: &str) -> &'a accesskit::Node {
        nodes
            .iter()
            .find(|node| node.role() == role && node.label() == Some(label))
            .unwrap_or_else(|| panic!("no {role:?} labeled {label:?}"))
    }

    /// The `Role::Label` node carrying `text` (a Label's text lands in `value`,
    /// not `label`).
    fn text_entry<'a>(nodes: &'a [accesskit::Node], text: &str) -> &'a accesskit::Node {
        nodes
            .iter()
            .find(|node| node.role() == Role::Label && node.value() == Some(text))
            .unwrap_or_else(|| panic!("no label {text:?}"))
    }

    /// `name`'s label as the pickers show it: underscores as spaces, each word
    /// capitalized (the draft's `titleCase`, restated so the tests check it
    /// rather than share it).
    fn title_case(name: &str) -> String {
        name.split('_')
            .map(|word| {
                let (first, rest) = word.split_at(1);
                format!("{}{rest}", first.to_uppercase())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The `PlayerSkills` of player `id` in `version`'s fixture save.
    fn fixture_skills(version: PesVersion, id: u32) -> PlayerSkills {
        let file = pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(version))
            .expect("fixture save");
        file.player(id)
            .unwrap_or_else(|| panic!("player {id}"))
            .skills
            .clone()
    }

    /// The stored playing-style index of player `id` in `version`'s fixture
    /// save.
    fn fixture_style(version: PesVersion, id: u32) -> u8 {
        let file = pes_savefile::EditFile::from_bytes(&pes_savefile::fixture_save(version))
            .expect("fixture save");
        file.player(id)
            .unwrap_or_else(|| panic!("player {id}"))
            .positions
            .playing_style
    }

    /// The checkbox labels a frame drew, split by the name table they come
    /// from.
    fn drawn_cards(nodes: &[accesskit::Node]) -> (Vec<String>, Vec<String>) {
        let skills: BTreeSet<String> = labels::SKILLS.iter().map(|name| title_case(name)).collect();
        let coms: BTreeSet<String> = labels::COM_STYLES
            .iter()
            .map(|name| title_case(name))
            .collect();
        let mut drawn_skills = vec![];
        let mut drawn_coms = vec![];
        for label in nodes.iter().filter_map(|node| {
            (node.role() == Role::CheckBox)
                .then(|| node.label().map(str::to_owned))
                .flatten()
        }) {
            if skills.contains(&label) {
                drawn_skills.push(label);
            } else if coms.contains(&label) {
                drawn_coms.push(label);
            } else {
                panic!("a checkbox named no card: {label:?}");
            }
        }
        (drawn_skills, drawn_coms)
    }

    /// An empty `CardLimits` but for `paid` and `limit`.
    fn limits(paid: u8, limit: i8) -> aatf::CardLimits {
        aatf::CardLimits {
            paid,
            limit,
            free: BTreeMap::new(),
            free_cards: vec![],
            required_cards: vec![],
            a_positions: 1,
            max_a_positions: None,
        }
    }

    // Every card is drawn on every version, a card the save cannot store
    // disabled rather than hidden, so the layout stays put across versions.
    #[test]
    fn the_columns_draw_the_versions_cards() {
        /// The labels of the disabled checkboxes.
        fn disabled(nodes: &[accesskit::Node]) -> Vec<&str> {
            nodes
                .iter()
                .filter(|node| node.role() == Role::CheckBox && node.is_disabled())
                .filter_map(|node| node.label())
                .collect()
        }

        let mut window = Headless::new();
        // Player 71802 of the PES 15 fixture save; PES 15 stores 22 of the 41
        // skills and 7 COM styles (schema/pes15.rs: `malicia` at index 21 is
        // one the version does not store, so its cell is disabled).
        let mut skills = fixture_skills(PesVersion::Pes15, 71802);
        let (nodes, _) = window.columns(PesVersion::Pes15, &mut skills, vec![]);
        let (drawn_skills, drawn_coms) = drawn_cards(&nodes);
        assert_eq!((drawn_skills.len(), drawn_coms.len()), (41, 7));
        let off = disabled(&nodes);
        assert_eq!(off.len(), 41 - 22, "19 skills PES 15 does not store");
        for label in ["Malicia", "Heading", "Chip Shot", "Through Passing"] {
            assert!(off.contains(&label), "{label} disabled on PES 15");
        }
        for label in ["Scissors Feint", "Track Back", "Trickster", "Long Ranger"] {
            assert!(!entry(&nodes, Role::CheckBox, label).is_disabled());
        }

        // Player 70203 of the PES 17 fixture save; PES 17 stores the first 28
        // canonical skills.
        let mut skills = fixture_skills(PesVersion::Pes17, 70203);
        let (nodes, changed) = window.columns(PesVersion::Pes17, &mut skills, vec![]);
        assert!(!changed);
        let (drawn_skills, drawn_coms) = drawn_cards(&nodes);
        assert_eq!((drawn_skills.len(), drawn_coms.len()), (41, 7));
        let off = disabled(&nodes);
        assert_eq!(off.len(), 41 - 28, "13 skills PES 17 does not store");
        for label in ["Scissors Feint", "Track Back", "Trickster", "Long Ranger"] {
            assert!(!entry(&nodes, Role::CheckBox, label).is_disabled());
        }

        // Player 78102 of the PES 21 fixture save; PES 21 stores all 41.
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        let (nodes, _) = window.columns(PesVersion::Pes21, &mut skills, vec![]);
        let (drawn_skills, drawn_coms) = drawn_cards(&nodes);
        assert_eq!((drawn_skills.len(), drawn_coms.len()), (41, 7));
        assert!(disabled(&nodes).is_empty(), "PES 21 stores every card");
    }

    // The skills' checkboxes fill two columns column by column: the first
    // row's cells are the first card of each column, not the first two of
    // the list.
    #[test]
    fn the_columns_fill_column_by_column() {
        let mut window = Headless::new();
        // Player 78102 of the PES 21 fixture save; the 41 skills split
        // 21-and-20, so the second column starts at index 21 ("Malicia").
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        let (nodes, _) = window.columns(PesVersion::Pes21, &mut skills, vec![]);
        let skills_labels: BTreeSet<String> =
            labels::SKILLS.iter().map(|name| title_case(name)).collect();
        let mut xs: Vec<f64> = nodes
            .iter()
            .filter(|node| {
                node.role() == Role::CheckBox
                    && node.label().is_some_and(|l| skills_labels.contains(l))
            })
            .filter_map(|node| node.bounds().map(|b| b.x0))
            .collect();
        xs.sort_by(f64::total_cmp);
        xs.dedup();
        assert_eq!(xs.len(), 2, "the two columns take two x positions");
        let first = entry(&nodes, Role::CheckBox, "Scissors Feint")
            .bounds()
            .expect("bounds");
        let malicia = entry(&nodes, Role::CheckBox, "Malicia")
            .bounds()
            .expect("bounds");
        assert!(
            (malicia.x0 - xs[1]).abs() < 1.0 && (malicia.y0 - first.y0).abs() < 1.0,
            "the second column starts at index 21 beside the first cell"
        );
        let second = entry(&nodes, Role::CheckBox, "Flip Flap")
            .bounds()
            .expect("bounds");
        assert!(
            (second.x0 - xs[0]).abs() < 1.0 && second.y0 > first.y0,
            "the second card fills the first column down"
        );
    }

    // A `shown` subset draws its own cards only, filled column by column
    // the same way: its second column starts beside the first cell, and a
    // second-column cell's tick writes its own index — the panels that
    // split the skills (the ruleset's apart cards) read the same.
    #[test]
    fn a_subset_draws_only_its_cards_column_by_column() {
        let mut window = Headless::new();
        // Player 78102 of the PES 21 fixture save; the five-card subset
        // splits 3-and-2.
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        let shown: [u8; 5] = [0, 1, 4, 21, 28];
        let subset = |ui: &mut egui::Ui, skills: &mut PlayerSkills| {
            card_columns(
                ui,
                PesVersion::Pes21,
                &mut skills.skills,
                &labels::SKILLS,
                &shown,
                PlayerField::Skill,
            )
        };
        let (nodes, _) = window.frame(vec![], |ui| {
            subset(ui, &mut skills);
        });
        let (drawn_skills, _) = drawn_cards(&nodes);
        let expected: Vec<String> = shown
            .iter()
            .map(|&i| title_case(labels::SKILLS[usize::from(i)]))
            .collect();
        assert_eq!(
            drawn_skills.iter().collect::<BTreeSet<_>>(),
            expected.iter().collect::<BTreeSet<_>>(),
            "the subset's cards alone"
        );

        // The second column starts at the fourth shown card beside the
        // first cell; the column fills down in `shown` order.
        let first = entry(&nodes, Role::CheckBox, "Scissors Feint")
            .bounds()
            .expect("bounds");
        let malicia = entry(&nodes, Role::CheckBox, "Malicia")
            .bounds()
            .expect("bounds");
        assert!(
            malicia.x0 > first.x0 + 1.0 && (malicia.y0 - first.y0).abs() < 1.0,
            "the second column starts at shown[3] beside the first cell"
        );
        let double_touch = entry(&nodes, Role::CheckBox, "Double Touch");
        let dt_bounds = double_touch.bounds().expect("bounds");
        assert!(
            (dt_bounds.x0 - malicia.x0).abs() < 1.0 && dt_bounds.y0 > malicia.y0,
            "the last shown card fills the second column down"
        );

        // A tick on the subset's last cell writes its own index, 28.
        let before = skills.skills;
        let mut changed = None;
        window.frame(pointer_at(centre(double_touch), true), |ui| {
            changed = Some(subset(ui, &mut skills));
        });
        window.frame(pointer_at(centre(double_touch), false), |ui| {
            changed = Some(changed.unwrap() | subset(ui, &mut skills));
        });
        assert!(changed.unwrap(), "the tick is a change");
        let mut expected = before;
        expected[28] = !expected[28];
        assert_eq!(skills.skills, expected, "exactly index 28 flipped");
    }

    // The draft's `.skcols`: two 158-point columns 14 apart span the
    // literal 330, the width the host sizes their column from.
    #[test]
    fn the_two_columns_span_the_drafts_330() {
        assert_eq!(TWO_COLUMN_WIDTH, 330.0, "the draft's literal");
        let mut window = Headless::new();
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        let mut span = None;
        window.frame(vec![], |ui| {
            let drawn = ui.scope(|ui| {
                card_columns(
                    ui,
                    PesVersion::Pes21,
                    &mut skills.skills,
                    &labels::SKILLS,
                    &all_indices(&labels::SKILLS),
                    PlayerField::Skill,
                );
            });
            span = Some(drawn.response.rect.width());
        });
        assert_eq!(
            span,
            Some(TWO_COLUMN_WIDTH),
            "the drawn columns' span, first column's left to second's right"
        );
    }

    // A checkbox click is one change, so the host's undo takes one step per
    // click.
    #[test]
    fn a_tick_is_one_change_on_that_frame_only() {
        let mut window = Headless::new();
        // Player 70203 of the PES 17 fixture save ("SMUG ANIME FACE").
        let mut skills = fixture_skills(PesVersion::Pes17, 70203);
        // The first skill PES 17 stores that he does not hold.
        let fields = schema_for(PesVersion::Pes17).player_field_set();
        let index = (0..labels::SKILLS.len())
            .find(|&i| fields.contains(&PlayerField::Skill(i as u8)) && !skills.skills[i])
            .expect("an unticked PES 17 skill");
        let label = title_case(labels::SKILLS[index]);
        let (nodes, _) = window.columns(PesVersion::Pes17, &mut skills, vec![]);
        let checkbox = entry(&nodes, Role::CheckBox, &label);

        let (_, changed) = window.click_columns(centre(checkbox), PesVersion::Pes17, &mut skills);
        assert!(changed, "the tick is a change");
        assert!(skills.skills[index], "the flag is set");
        let (_, changed) = window.columns(PesVersion::Pes17, &mut skills, vec![]);
        assert!(!changed, "the next frame reports nothing new");

        let (_, changed) = window.click_columns(centre(checkbox), PesVersion::Pes17, &mut skills);
        assert!(changed, "the untick is a change");
        assert!(!skills.skills[index], "the flag is cleared");
    }

    // A card the version cannot store stays in its cell, disabled, so the
    // layout stays put across versions. Its tooltip names the first version
    // that has it, in the draft's wording (`PES NN+`).
    #[test]
    fn a_card_the_version_lacks_keeps_its_cell_disabled() {
        let mut window = Headless::new();
        // "Through Passing" is the last skill (index 40), PES 20 and 21 only.
        let mut skills15 = fixture_skills(PesVersion::Pes15, 71802);
        let mut skills21 = fixture_skills(PesVersion::Pes21, 78102);
        let (nodes15, _) = window.columns(PesVersion::Pes15, &mut skills15, vec![]);
        let (nodes21, _) = window.columns(PesVersion::Pes21, &mut skills21, vec![]);
        let at = |nodes: &[accesskit::Node], label: &str| {
            let bounds = entry(nodes, Role::CheckBox, label)
                .bounds()
                .expect("bounds");
            (bounds.x0, bounds.y0)
        };
        assert_eq!(
            at(&nodes15, "Through Passing"),
            at(&nodes21, "Through Passing"),
            "the same card sits in the same cell on every version"
        );
        // Malicia (index 21, mid-grid): the assert that proves the cell, not
        // only the row's left edge.
        assert_eq!(
            at(&nodes15, "Malicia"),
            at(&nodes21, "Malicia"),
            "a mid-grid card sits in the same cell on every version"
        );
        let cell = entry(&nodes15, Role::CheckBox, "Through Passing");
        assert!(cell.is_disabled(), "PES 20+, disabled on PES 15");

        // A click on the disabled cell changes nothing and reports no change.
        let held = skills15.skills[40];
        let (_, changed) = window.click_columns(centre(cell), PesVersion::Pes15, &mut skills15);
        assert!(!changed);
        assert_eq!(skills15.skills[40], held);
    }

    // The disabled cell's hover text names the first version that stores
    // the card, in the draft's wording (`PES NN+`).
    #[test]
    fn the_disabled_cell_names_the_first_version() {
        let mut window = Headless::new();
        // Player 71802 of the PES 15 fixture save; `malicia` first stores in
        // PES 16.
        let mut skills = fixture_skills(PesVersion::Pes15, 71802);
        let (nodes, _) = window.columns(PesVersion::Pes15, &mut skills, vec![]);
        let cell = entry(&nodes, Role::CheckBox, "Malicia");
        let pos = centre(cell);
        // The pointer rests on the cell while the clock passes the tooltip's
        // delay.
        window.columns_at(
            Some(0.0),
            PesVersion::Pes15,
            &mut skills,
            vec![egui::Event::PointerMoved(pos)],
        );
        let mut shown = false;
        for frame in 1..20 {
            let (nodes, _) = window.columns_at(
                Some(f64::from(frame) * 0.5),
                PesVersion::Pes15,
                &mut skills,
                vec![],
            );
            if nodes
                .iter()
                .any(|node| node.role() == Role::Label && node.value() == Some("PES 16+"))
            {
                shown = true;
                break;
            }
        }
        assert!(shown, "the tooltip names the first version, PES 16+");
    }

    // A card the version cannot store stays in its cell, disabled, and is
    // as tall as an enabled one. On PES 17, whose second skills column is
    // disabled cells, every row's two cells share their top and every cell
    // is one line tall, and the same holds for the COM styles. The widest
    // label, "Long Range Shooting" at 125.6 points at the body font, fits
    // the 158-point column: a longer one would wrap and break the
    // alignment.
    #[test]
    fn the_columns_rows_line_up_past_the_disabled_cells() {
        let mut window = Headless::new();
        // PES 17 stores the first 28 of the 41 skills and every COM
        // style: the skills' second column holds disabled cells.
        let mut skills = fixture_skills(PesVersion::Pes17, 70203);
        let (nodes, _) = window.columns(PesVersion::Pes17, &mut skills, vec![]);
        for (panel, names) in [
            ("Skills", &labels::SKILLS[..]),
            ("COM styles", &labels::COM_STYLES[..]),
        ] {
            let wanted: BTreeSet<String> = names.iter().map(|name| title_case(name)).collect();
            let cells: Vec<(f64, f64, f64, bool)> = nodes
                .iter()
                .filter(|node| {
                    node.role() == Role::CheckBox
                        && node.label().is_some_and(|label| wanted.contains(label))
                })
                .filter_map(|node| {
                    node.bounds()
                        .map(|b| (b.x0, b.y0, b.y1, node.is_disabled()))
                })
                .collect();
            assert_eq!(cells.len(), names.len(), "{panel}: every card drawn");
            if panel == "Skills" {
                assert!(
                    cells.iter().any(|cell| cell.3),
                    "premise: the skills column holds disabled cells"
                );
            }
            let mut xs: Vec<f64> = cells.iter().map(|cell| cell.0).collect();
            xs.sort_by(f64::total_cmp);
            xs.dedup();
            assert_eq!(xs.len(), 2, "{panel}: two columns");
            let column = |x: f64| {
                let mut column: Vec<(f64, f64)> = cells
                    .iter()
                    .filter(|cell| cell.0 == x)
                    .map(|cell| (cell.1, cell.2))
                    .collect();
                column.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("ordered bounds"));
                column
            };
            let (first, second) = (column(xs[0]), column(xs[1]));
            for row in 0..second.len() {
                assert!(
                    (first[row].0 - second[row].0).abs() <= 0.5,
                    "{panel} row {row}: the columns' tops differ ({} vs {})",
                    first[row].0,
                    second[row].0
                );
            }
            let one_line = first[0].1 - first[0].0;
            for (column, cells) in [&first, &second].into_iter().enumerate() {
                for (row, &(y0, y1)) in cells.iter().enumerate() {
                    assert!(
                        (y1 - y0 - one_line).abs() <= 0.5,
                        "{panel} column {column} row {row} is not one line \
                         tall ({} vs {one_line})",
                        y1 - y0
                    );
                }
            }
        }
    }

    // A COM style's tick is a change, as a skill's is.
    #[test]
    fn a_com_style_tick_is_one_change_on_that_frame_only() {
        let mut window = Headless::new();
        // Player 78102 of the PES 21 fixture save ("THE YOINKY SPLOINKY") holds
        // no COM style.
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        assert!(!skills.com_styles[0], "premise: no Trickster");
        let (nodes, _) = window.columns(PesVersion::Pes21, &mut skills, vec![]);
        let checkbox = entry(&nodes, Role::CheckBox, "Trickster");

        let (_, changed) = window.click_columns(centre(checkbox), PesVersion::Pes21, &mut skills);
        assert!(changed, "the tick is a change");
        assert!(skills.com_styles[0], "the flag is set");
        let (_, changed) = window.columns(PesVersion::Pes21, &mut skills, vec![]);
        assert!(!changed, "the next frame reports nothing new");

        let (_, changed) = window.click_columns(centre(checkbox), PesVersion::Pes21, &mut skills);
        assert!(changed, "the untick is a change");
        assert!(!skills.com_styles[0], "the flag is cleared");
    }

    // The same bullet: a card in the second column writes its own flag
    // (a wrong index in the second column's cells edits a first-column
    // card instead).
    #[test]
    fn a_second_column_tick_writes_its_own_flag() {
        let mut window = Headless::new();
        // Player 78102 of the PES 21 fixture save; PES 21 stores every
        // card, so each second-column cell is enabled. The skills split
        // 21-and-20 (the second column is indexes 21 to 40), the COM
        // styles 4-and-3 (indexes 4 to 6).
        let mut skills = fixture_skills(PesVersion::Pes21, 78102);
        let skill = (21..labels::SKILLS.len())
            .find(|&i| !skills.skills[i])
            .expect("a skill past index 20 he lacks");
        let com = (4..labels::COM_STYLES.len())
            .find(|&i| !skills.com_styles[i])
            .expect("a COM style he lacks");

        let (nodes, _) = window.columns(PesVersion::Pes21, &mut skills, vec![]);
        // Premise: each clicked cell really sits in the second column;
        // "Scissors Feint" and "Trickster" head the first ones.
        for (index, names, first) in [
            (skill, &labels::SKILLS[..], "Scissors Feint"),
            (com, &labels::COM_STYLES[..], "Trickster"),
        ] {
            let label = title_case(names[index]);
            let cell = entry(&nodes, Role::CheckBox, &label)
                .bounds()
                .expect("bounds");
            let first_x = entry(&nodes, Role::CheckBox, first)
                .bounds()
                .expect("bounds")
                .x0;
            assert!(cell.x0 > first_x + 1.0, "{label} is a second-column cell");
        }

        let before = skills.skills;
        let cell = entry(&nodes, Role::CheckBox, &title_case(labels::SKILLS[skill]));
        let (_, changed) = window.click_columns(centre(cell), PesVersion::Pes21, &mut skills);
        assert!(changed, "the tick is a change");
        let mut expected = before;
        expected[skill] = true;
        assert_eq!(skills.skills, expected, "exactly that one flag changed");

        let before = skills.com_styles;
        let cell = entry(&nodes, Role::CheckBox, &title_case(labels::COM_STYLES[com]));
        let (_, changed) = window.click_columns(centre(cell), PesVersion::Pes21, &mut skills);
        assert!(changed, "the tick is a change");
        let mut expected = before;
        expected[com] = true;
        assert_eq!(skills.com_styles, expected, "exactly that one flag changed");
    }

    // The Cards left badge, the draft's `cardBadges`: "Cards left for" then
    // the one badge, the player's own tier's `label: left of limit` (no
    // `cards` after the count: the label before it says so),
    // in the error colour when overdrawn. `NoTier` draws "Cards left:" then
    // the weak stand-in. A colour is no AccessKit property, so it is read back
    // from the frame's painted shapes: the badge's `TextShape` carries
    // the `RichText` colour in its galley's sections.
    #[test]
    fn the_badge_reads_the_own_tiers_economy_coloured_when_overdrawn() {
        let mut window = Headless::new();
        let under = limits(3, 5);
        let over = limits(7, 4);
        let spent = limits(4, 4);
        let (nodes, shapes) = window.badge(CardsBadge::Tier(TierCards {
            label: "Gold",
            limits: &under,
        }));
        text_entry(&nodes, "Cards left for");
        text_entry(&nodes, "Gold: 2 of 5");
        let text_color = window.ctx.style_of(window.ctx.theme()).visuals.text_color();
        let error_color = window
            .ctx
            .style_of(window.ctx.theme())
            .visuals
            .error_fg_color;
        assert_eq!(painted_text_color(&shapes, "Gold: 2 of 5"), text_color);
        // Overdrawn paints the error colour.
        let (_, shapes) = window.badge(CardsBadge::Tier(TierCards {
            label: "Silver",
            limits: &over,
        }));
        assert_eq!(painted_text_color(&shapes, "Silver: -3 of 4"), error_color);
        // No card left is not overdrawn.
        let (_, shapes) = window.badge(CardsBadge::Tier(TierCards {
            label: "Bronze",
            limits: &spent,
        }));
        assert_eq!(painted_text_color(&shapes, "Bronze: 0 of 4"), text_color);
        // No tier matching draws the weak stand-in.
        let (nodes, _) = window.badge(CardsBadge::NoTier);
        text_entry(&nodes, "Cards left:");
        text_entry(&nodes, "No tier matches");
    }

    // The playing-style choice lists the version's styles only.
    #[test]
    fn the_style_picker_lists_the_versions_styles() {
        let mut window = Headless::new();
        // Player 70203 of the PES 17 fixture save; stored playing style 8.
        let mut style = fixture_style(PesVersion::Pes17, 70203);
        let (nodes, changed) = window.style_picker(PesVersion::Pes17, &mut style, vec![]);
        assert!(!changed, "an idle frame is no change");
        let combo = nodes
            .iter()
            .find(|node| node.role() == Role::ComboBox)
            .expect("the combo");
        let (_, changed) = window.click_style(centre(combo), PesVersion::Pes17, &mut style);
        assert!(!changed, "opening the combo is no change");
        let (nodes, _) = window.style_picker(PesVersion::Pes17, &mut style, vec![]);
        // `nodes` come back in `IdMap` order, not layout order: sort the open
        // items top to bottom by their bounds.
        let mut items: Vec<&accesskit::Node> = nodes
            .iter()
            .filter(|node| node.role() == Role::Button)
            .collect();
        items.sort_by(|a, b| {
            let (ay, by) = (a.bounds().map(|r| r.y0), b.bounds().map(|r| r.y0));
            ay.partial_cmp(&by).expect("drawn bounds")
        });
        let open_labels: Vec<String> = items
            .iter()
            .filter_map(|node| node.label().map(str::to_owned))
            .collect();
        let expected: Vec<String> = (0..=u8::MAX)
            .filter_map(|index| playstyle::decode(PesVersion::Pes17, index).ok())
            .map(|style| {
                let at = PlayStyle::ALL
                    .iter()
                    .position(|canonical| *canonical == style)
                    .expect("a canonical style");
                title_case(labels::PLAY_STYLES[at])
            })
            .collect();
        assert_eq!(open_labels, expected, "PES 17's eighteen styles");
        assert!(
            !open_labels.contains(&"Roaming Flank".to_string()),
            "a PES 19+ style is not listed"
        );

        // The current style picked again: no change, the value unmoved.
        let item = entry(&nodes, Role::Button, "Anchor Man");
        let (_, changed) = window.click_style(centre(item), PesVersion::Pes17, &mut style);
        assert!(!changed, "the current style picked again is no change");
        assert_eq!(style, 8);

        let (_, changed) = window.click_style(centre(combo), PesVersion::Pes17, &mut style);
        assert!(!changed, "reopening the combo is no change");
        let (nodes, _) = window.style_picker(PesVersion::Pes17, &mut style, vec![]);
        let item = entry(&nodes, Role::Button, "Goal Poacher");
        let wanted = playstyle::encode(PesVersion::Pes17, PlayStyle::GoalPoacher)
            .expect("PES 17 has Goal Poacher");
        assert_ne!(style, wanted, "premise: the player is no Goal Poacher");
        let (_, changed) = window.click_style(centre(item), PesVersion::Pes17, &mut style);
        assert!(changed, "the pick is a change");
        assert_eq!(style, wanted);
    }
}
