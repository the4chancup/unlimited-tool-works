//! The Tactics editor's settings (the draft's `tacticsTab`, minus the
//! pitch canvas and the lineup, which are `crate::pitch`): the preset
//! selector and the Fluid formation switch of the preset bar, then one
//! `pub fn` per group panel — Formation settings, Advanced instructions
//! (`instructions`), Player assignments (`assignments`), Automatic
//! settings — so the host lays the grid's cells out. Each panel edits
//! `tactics` in place and answers whether it changed; the host decides
//! what the change means.

use pes_savefile::model::tactics::{TacticSliders, TacticStyle, TacticsPreset, TeamTactics};
use pes_savefile::schema::fields::{
    FormationPart, InstructionPart, InstructionSide, PresetField, TacticsField,
};
use pes_savefile::schema::{instruction, schema_for};
use pes_version::PesVersion;
use studio_core::theme::Palette;
use studio_core::widgets::{add_enabled, checkbox, step_choice, wheel_steps};

use crate::pitch::{FORMATIONS, position_label};

/// The role palette's armed modes, chips and note.
mod assignments;
/// The "Advanced instructions" table and its rows.
mod instructions;

pub(crate) use assignments::{Assign, OpenSlot};

/// What the tactics editor shows that the record does not hold: the
/// preset and the formation on screen, the armed assignment mode, the
/// open slot popover and the pitch's name scrolling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticsView {
    /// The preset the controls edit, 0 to 2, shown 1 to 3.
    pub preset: usize,
    /// The preset's formation the lineup and pitch show: 0 kick-off, 1
    /// in-possession, 2 out-of-possession. The formation bar's
    /// `pitch::formation_selector` picks it; it and `pitch::pitch` put it
    /// back on kick-off while the preset fields that formation alone.
    pub formation: usize,
    /// The armed assignment mode, `None` when none: a pitch click then
    /// writes the role instead of opening the slot popover.
    pub(crate) assign: Option<Assign>,
    /// The formation slot whose popover is open, `None` closed. `pitch`
    /// closes it when the preset, the formation or the team it opened on
    /// changed since.
    pub(crate) popover: Option<OpenSlot>,
    /// The formation bar's Scroll names checkbox: on, a name wider than
    /// its badge scrolls; off, it is drawn still at the badge's left. A
    /// view setting — the host owns its persistence (`scroll_names`,
    /// remembered as Live AATF is); it never edits the record.
    pub scroll_names: bool,
}

impl TacticsView {
    /// The open slot popover's `egui::Area` id (its layer's), `None`
    /// while it is closed: a host reads the popover's rect by it
    /// (`egui::Memory::area_rect`), whatever other popup is open.
    pub fn popover_area(&self) -> Option<egui::Id> {
        self.popover.map(|open| open.area_id())
    }
}

impl Default for TacticsView {
    /// The view as a save opens: preset 1, kick-off, nothing armed or
    /// popped, names scrolling.
    fn default() -> TacticsView {
        TacticsView {
            preset: 0,
            formation: 0,
            assign: None,
            popover: None,
            scroll_names: true,
        }
    }
}

/// What a roster slot shows where the editor lists it (the instruction
/// targets, the lineup and bench rows): the player's display name and
/// registered position, both `None`-equivalent on an empty slot ("" and
/// `None` read "(empty)" and "--").
#[derive(Debug, Clone, Copy)]
pub struct RosterRow<'a> {
    /// The slot's display name, "" an empty or nameless one.
    pub name: &'a str,
    /// The slot's raw savefile name, colour codes kept, "" an empty or
    /// nameless one: what the lineup rows and the slot popover draw for
    /// the coloured runs (the markers, chips and combos read `name`).
    pub raw_name: &'a str,
    /// The player's registered position byte; `None` an empty slot.
    pub registered_position: Option<u8>,
    /// The player's portrait, already uploaded by the host; `None` when none
    /// was found (or the host reads none): the marker then draws without one.
    pub portrait: Option<egui::TextureId>,
    /// The player's medal tier under the host's active ruleset (the
    /// pitch markers outline the badge in its colour); `None` for a
    /// slot with no medal or a host that reads none.
    pub medal: Option<Medal>,
    /// The player's overall at each position, indexed by the position byte
    /// (GK 0 to CF 12); `None` for an empty slot or a host that reads none.
    pub overalls: Option<[u8; 13]>,
    /// The positions he is rated A or B at, indexed by the position byte
    /// (GK 0 to CF 12): while his pitch marker is held, the pitch lights
    /// their areas and darkens the rest. All `false` for an empty slot or
    /// a host that reads none.
    pub rated: [bool; 13],
    /// The slot's `FieldedNotA` marks (the pitch paints a `✖` left of
    /// the starter's badge on the formation a mark names); empty for a
    /// slot with none or a host that reads none.
    pub fielded: &'a [FieldedMark],
}

/// One `FieldedNotA` violation on the roster slot, as the pitch's cross
/// needs it: which preset and position it names, its severity's colour
/// and its line for the tooltip.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldedMark {
    /// The violation's preset minus 1, 0 to 2 as `view.preset` counts.
    pub preset: usize,
    /// The position the violation fields him at: the stored position
    /// byte a formation slot's `position` holds.
    pub position: u8,
    /// The violation is an Error (`error_fg_color`); else a Warning
    /// (`Palette::pitch_warn`).
    pub error: bool,
    /// Its `violation_line`: the cross's tooltip shows the marks' lines,
    /// one per line.
    pub line: String,
}

/// A starter's medal tier: the colour his pitch badge's outline
/// carries — gold `#cc9900` (the game's gold name colour), silver
/// `#c0c0c0` or bronze `#8c4a20`, far darker than the metal's
/// `#cd7f32`, which a thin outline let pass for gold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Medal {
    /// The `gold` tier.
    Gold,
    /// The `silver` tier.
    Silver,
    /// The `bronze` tier.
    Bronze,
}

impl Medal {
    /// The medal a ruleset's tier `id` reads as — `gold`, `silver` or
    /// `bronze`, the ids every ruleset shares; `None` for any other.
    pub fn from_tier_id(id: &str) -> Option<Medal> {
        match id {
            "gold" => Some(Medal::Gold),
            "silver" => Some(Medal::Silver),
            "bronze" => Some(Medal::Bronze),
            _ => None,
        }
    }

    /// The badge outline's colour in `palette`: gold in the game's gold
    /// name colour, silver its metal's, bronze darkened so a thin outline
    /// does not read as gold.
    pub(crate) fn colour(self, palette: &Palette) -> egui::Color32 {
        match self {
            Medal::Gold => palette.medal_gold,
            Medal::Silver => palette.medal_silver,
            Medal::Bronze => palette.medal_bronze,
        }
    }
}

/// One of the five sliders' rows (the draft's `SLIDERS` and
/// `TRI_SETTINGS`: the two "numbers" stay in this table for their
/// label and field, and are drawn as three-choice toggles): the
/// label, the range top and the `TacticSliders` field it edits.
struct SliderSpec {
    /// The cell's label.
    label: &'static str,
    /// The preset field the slider writes (`tactics_term`'s key into the
    /// table, so a row's label is never a second copy).
    field: PresetField,
    /// The range's top (1 is always the bottom).
    max: u8,
    /// The stored value.
    get: fn(&TacticSliders) -> u8,
    /// The stored write.
    set: fn(&mut TacticSliders, u8),
}

/// The five sliders, in the draft's order.
const SLIDERS: [SliderSpec; 5] = [
    SliderSpec {
        label: "Support range",
        field: PresetField::SupportRange,
        max: 10,
        get: |sliders| sliders.support_range,
        set: |sliders, value| sliders.support_range = value,
    },
    SliderSpec {
        label: "Defensive line",
        field: PresetField::DefensiveLine,
        max: 10,
        get: |sliders| sliders.defensive_line,
        set: |sliders, value| sliders.defensive_line = value,
    },
    SliderSpec {
        label: "Compactness",
        field: PresetField::Compactness,
        max: 10,
        get: |sliders| sliders.compactness,
        set: |sliders, value| sliders.compactness = value,
    },
    SliderSpec {
        label: "Numbers in attack",
        field: PresetField::NumbersInAttack,
        max: 3,
        get: |sliders| sliders.numbers_in_attack,
        set: |sliders, value| sliders.numbers_in_attack = value,
    },
    SliderSpec {
        label: "Numbers in defence",
        field: PresetField::NumbersInDefence,
        max: 3,
        get: |sliders| sliders.numbers_in_defence,
        set: |sliders, value| sliders.numbers_in_defence = value,
    },
];

/// One of the seven style pairs (the draft's `STYLE_TOGGLES`): its label,
/// the two choices' texts as the game names them and the `TacticStyle`
/// field it edits.
struct StyleToggle {
    /// The cell's label.
    label: &'static str,
    /// The preset field the pair writes (`tactics_term`'s key into the
    /// table, so a row's label and texts are never a second copy).
    field: PresetField,
    /// The stored 0's text.
    off: &'static str,
    /// The stored 1's text.
    on: &'static str,
    /// The stored flag.
    get: fn(&TacticStyle) -> bool,
    /// The stored write.
    set: fn(&mut TacticStyle, bool),
}

/// The seven style pairs, in the draft's order.
const STYLE_TOGGLES: [StyleToggle; 7] = [
    StyleToggle {
        label: "Attacking style",
        field: PresetField::AttackingStyle,
        off: "Counter Attack",
        on: "Possession",
        get: |style| style.attacking_style,
        set: |style, value| style.attacking_style = value,
    },
    StyleToggle {
        label: "Attacking zone",
        field: PresetField::AttackingZone,
        off: "Centre",
        on: "Wide",
        get: |style| style.attacking_zone,
        set: |style, value| style.attacking_zone = value,
    },
    StyleToggle {
        label: "Buildup",
        field: PresetField::Buildup,
        off: "Long Pass",
        on: "Short Pass",
        get: |style| style.buildup,
        set: |style, value| style.buildup = value,
    },
    StyleToggle {
        label: "Positioning",
        field: PresetField::Positioning,
        off: "Maintain",
        on: "Flexible",
        get: |style| style.positioning,
        set: |style, value| style.positioning = value,
    },
    StyleToggle {
        label: "Defensive style",
        field: PresetField::DefensiveStyle,
        off: "Frontline Pressure",
        on: "All Out Defence",
        get: |style| style.defensive_style,
        set: |style, value| style.defensive_style = value,
    },
    StyleToggle {
        label: "Containment area",
        field: PresetField::ContainmentArea,
        off: "Middle",
        on: "Wide",
        get: |style| style.containment_area,
        set: |style, value| style.containment_area = value,
    },
    StyleToggle {
        label: "Pressure",
        field: PresetField::Pressure,
        off: "Aggressive",
        on: "Conservative",
        get: |style| style.pressure,
        set: |style, value| style.pressure = value,
    },
];

/// A style choice's text where the full name is wider than its half
/// allows (the draft's `STYLE_SHORT`): shown shortened on the choice,
/// the full name the hover and the changes table's.
fn style_short(full: &'static str) -> &'static str {
    match full {
        "Frontline Pressure" => "Frontline Press.",
        "All Out Defence" => "All Out Def.",
        _ => full,
    }
}

/// The Formation settings' halves (the draft's `TAC_OFFENSIVE` and
/// `TAC_DEFENSIVE`): six rows, each an offensive setting at the left
/// and its defensive pair at the right.
const TAC_OFFENSIVE: [PresetField; 6] = [
    PresetField::AttackingStyle,
    PresetField::Buildup,
    PresetField::AttackingZone,
    PresetField::Positioning,
    PresetField::SupportRange,
    PresetField::NumbersInAttack,
];
const TAC_DEFENSIVE: [PresetField; 6] = [
    PresetField::DefensiveStyle,
    PresetField::ContainmentArea,
    PresetField::Pressure,
    PresetField::DefensiveLine,
    PresetField::Compactness,
    PresetField::NumbersInDefence,
];

/// The two settings drawn as three-choice toggles (the draft's
/// `TRI_SETTINGS`): their labels and stored fields are `SLIDERS`'.
const NUMBERS: [PresetField; 2] = [PresetField::NumbersInAttack, PresetField::NumbersInDefence];
/// The three choices of a numbers toggle (the draft's `TRI_CHOICES`),
/// for the stored 1, 2 and 3.
const TRI_CHOICES: [&str; 3] = ["Few", "Medium", "Many"];

/// The two halves' gap in the settings rows (the draft's `.tacrows`
/// `column-gap: 16px`).
const HALVES_GAP: f32 = 16.0;
/// A cell's label-to-control gap (the draft's `.tset` `gap: 2px`).
const CELL_GAP: f32 = 2.0;
/// The title-to-rows gap (the draft's `.tacrows` `padding-top: 6px`).
const TITLE_GAP: f32 = 6.0;
/// A control's box height (the draft's `.tacrows .tset > .rate`, `.row`:
/// `height: 24px`), the control centred in it.
const CONTROL_HEIGHT: f32 = 24.0;
/// The gap inside a `.rate` control (the draft's `.rate { gap: 1px }`).
const CHOICE_GAP: f32 = 1.0;
/// A choice's padding: none — the draft's `.rate .sel` `padding: 0 5px`
/// bounds only where the ellipsis can start inside a cell the text
/// already centres in, so the whole choice's width is the text's.
const CHOICE_PADDING: egui::Vec2 = egui::vec2(0.0, 0.0);

/// The fluid switch's label (the checkbox's and `tactics_term`'s row's
/// one copy).
const FLUID_LABEL: &str = "Fluid formation";
/// The substitution field's label.
const SUBSTITUTION_LABEL: &str = "Substitution";
/// The offside-trap checkbox's label.
const OFFSIDE_TRAP_LABEL: &str = "Offside trap";
/// The preset-change checkbox's label.
const PRESET_CHANGE_LABEL: &str = "Preset change";
/// The attack/defence-levels checkbox's label.
const ATTACK_DEFENCE_LABEL: &str = "Attack/defence levels";

/// The preset selector (the draft's `presetSel`, in the tab's `tacbar`
/// beside the menus that act on the shown preset): "Preset" then 1, 2
/// and 3. Writes `view.preset`, never the record.
pub fn preset_selector(ui: &mut egui::Ui, view: &mut TacticsView) {
    ui.horizontal(|ui| {
        bar_selector_spacing(ui);
        ui.weak("Preset");
        for preset in 0..3 {
            ui.selectable_value(&mut view.preset, preset, (preset + 1).to_string());
        }
    });
}

/// The spacing of a bar's selector, the preset's and the formation's (the
/// draft's `.sel`s in a `gap: 2px` row): its items 2 points apart and each
/// choice padded 5 across and 17 tall (its text and a point over and
/// under it), where the bar spaces its items 8 apart and a button pads its
/// text 4 and stands 18 tall.
pub(crate) fn bar_selector_spacing(ui: &mut egui::Ui) {
    let spacing = ui.spacing_mut();
    spacing.item_spacing.x = 2.0;
    spacing.button_padding.x = 5.0;
    spacing.interact_size.y = 17.0;
}

/// The Fluid formation switch of the preset bar (the draft's `fluid`,
/// moved out of Formation settings onto the bar beside the preset
/// selector): the shown preset's flag. A version that stores none keeps
/// it in place, disabled with the `PES NN+` tooltip. Answers `true` only
/// when the flag changed this frame.
pub fn fluid_formation(ui: &mut egui::Ui, tactics: &mut TeamTactics, view: &TacticsView) -> bool {
    let preset = &mut tactics.presets[view.preset.min(2)];
    match &mut preset.style.fluid {
        Some(fluid) => checkbox(ui, fluid, FLUID_LABEL).changed(),
        None => {
            let mut off = false;
            add_enabled(ui, false, |ui: &mut egui::Ui| {
                checkbox(ui, &mut off, FLUID_LABEL)
            })
            .on_disabled_hover_text(gate_tip(TacticsField::Preset {
                preset: 0,
                field: PresetField::FluidFormation,
            }));
            false
        }
    }
}

/// The "Formation settings" panel (the draft's `#tac-settings` group):
/// six equal rows of setting pairs sharing the panel's free height
/// under its title — each offensive setting at the left, its defensive
/// pair at the right, the draft's `TAC_OFFENSIVE` / `TAC_DEFENSIVE`
/// order — each cell its label over its control in a box 24 points
/// tall. The frame fills the cell the host gives the panel (the
/// pitch's height). Edits the preset on screen; answers `true` only
/// when `tactics` changed this frame.
pub fn formation_settings(
    ui: &mut egui::Ui,
    tactics: &mut TeamTactics,
    view: &TacticsView,
) -> bool {
    let preset = &mut tactics.presets[view.preset.min(2)];
    // The answer is the deep compare, not the responses' `changed()`: a
    // control re-picked at its own value reports nothing.
    let before = preset.clone();
    panel(ui, "Formation settings", |ui| {
        // The frame's bottom is the cell's, as the pitch's is (the
        // grid's row of equal-height cells).
        ui.set_min_height(ui.available_height());
        // `add_space` adds to the item gap: 3 plus these 3 is the
        // `.tacrows` `padding-top` of 6.
        ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y);
        let rows_top = ui.cursor().min.y;
        let inner = ui.max_rect();
        // The six rows share the free height (`grid-auto-rows: 1fr`),
        // each its halves `HALVES_GAP` apart.
        let row_height = (inner.max.y - rows_top) / TAC_OFFENSIVE.len() as f32;
        let half = (inner.width() - HALVES_GAP) / 2.0;
        for (index, (offensive, defensive)) in TAC_OFFENSIVE.iter().zip(&TAC_DEFENSIVE).enumerate()
        {
            let top = rows_top + index as f32 * row_height;
            for (left, field) in [
                (inner.min.x, offensive),
                (inner.min.x + half + HALVES_GAP, defensive),
            ] {
                let mut cell_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(egui::Rect::from_min_size(
                            egui::pos2(left, top),
                            egui::vec2(half, row_height),
                        ))
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );
                // The cell's content at its top (`align-items: start`),
                // the control 2 under the label (`.tset`).
                cell_ui.spacing_mut().item_spacing.y = CELL_GAP;
                setting_cell(&mut cell_ui, preset, field);
            }
        }
    });
    *preset != before
}

/// One setting's cell of the rows (the draft's `setting` for a key): a
/// slider, a numbers toggle or a style pair — never the fluid switch,
/// which lives on the preset bar.
fn setting_cell(ui: &mut egui::Ui, preset: &mut TacticsPreset, field: &PresetField) {
    if let Some(spec) = SLIDERS.iter().find(|spec| spec.field == *field) {
        ui.label(spec.label);
        control_box(ui, |ui| {
            if NUMBERS.contains(field) {
                number_choices(ui, &mut preset.sliders, spec);
            } else {
                slider(ui, &mut preset.sliders, spec);
            }
        });
    } else if let Some(toggle) = STYLE_TOGGLES.iter().find(|toggle| toggle.field == *field) {
        ui.label(toggle.label);
        control_box(ui, |ui| style_choices(ui, &mut preset.style, toggle));
    } else {
        panic!("{field:?} is no Formation settings row");
    }
}

/// The setting's control in its box under the label (the draft's
/// `.tacrows .tset > .rate`, `.row` `height: 24px; align-items:
/// center`): 24 points tall, the control centred in it, so a toggle's
/// centre is level with its row partner's slider's.
fn control_box(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), CONTROL_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        add,
    );
}

/// A settings panel of the grid (the draft's `.group` boxes): the
/// bordered box with its `strong` heading inside, spanning the cell the
/// host gives it.
fn panel(ui: &mut egui::Ui, heading: &'static str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        // The box spans the cell even where its content is narrower
        // (Player assignments fills the rest of the grid's row).
        ui.set_min_width(ui.available_width());
        ui.strong(heading);
        add(ui);
    });
}

/// `text`'s galley width in the body font.
pub(crate) fn text_width(ui: &egui::Ui, text: &str) -> f32 {
    ui.painter()
        .layout_no_wrap(
            text.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
}

/// One style pair's control (the draft's `.rate`: the two choices,
/// each half the half less the 1-point gap, the chosen one in the
/// selection fill, its text cut with an ellipsis at the edge — the
/// defensive style's shortened, `style_short`, its hover the full
/// name). Both choices are drawn every frame — an early return over
/// the two would skip the second on the frame the first is clicked.
fn style_choices(ui: &mut egui::Ui, style: &mut TacticStyle, toggle: &StyleToggle) {
    ui.spacing_mut().item_spacing.x = CHOICE_GAP;
    ui.spacing_mut().button_padding = CHOICE_PADDING;
    let width = (ui.available_width() - CHOICE_GAP) / 2.0;
    let mut value = (toggle.get)(style);
    for (picked, full) in [(false, toggle.off), (true, toggle.on)] {
        let response = ui
            .add_sized(
                egui::vec2(width, ui.spacing().interact_size.y),
                egui::Button::selectable(value == picked, style_short(full)).truncate(),
            )
            .on_hover_text(full);
        if response.clicked() {
            value = picked;
        }
    }
    if value != (toggle.get)(style) {
        (toggle.set)(style, value);
    }
}

/// One numbers row's control (the draft's `.rate.tri`: `TRI_CHOICES`
/// thirds of the half less the gaps, the picked one filled). A stored
/// value outside 1-3 (a hand-edited save) selects none and writes
/// nothing until a choice is clicked.
fn number_choices(ui: &mut egui::Ui, sliders: &mut TacticSliders, spec: &SliderSpec) {
    ui.spacing_mut().item_spacing.x = CHOICE_GAP;
    ui.spacing_mut().button_padding = CHOICE_PADDING;
    let width = (ui.available_width() - 2.0 * CHOICE_GAP) / 3.0;
    let mut value = (spec.get)(sliders);
    for (name, choice) in TRI_CHOICES.iter().zip(1u8..=3) {
        if ui
            .add_sized(
                egui::vec2(width, ui.spacing().interact_size.y),
                egui::Button::selectable(value == choice, *name).truncate(),
            )
            .clicked()
        {
            value = choice;
        }
    }
    if value != (spec.get)(sliders) {
        (spec.set)(sliders, value);
    }
}

/// One slider's control (the draft's `.row`: the rail filling the half
/// less the value's field and their gap, the value at the right). The
/// rail's fill and the handle take the selection blue — egui fills the
/// handle with the widget state's `bg_fill`, the rail's grey, and has
/// no field for the handle's fill alone, so it is painted over.
fn slider(ui: &mut egui::Ui, sliders: &mut TacticSliders, spec: &SliderSpec) {
    let width = (ui.available_width() - ui.spacing().item_spacing.x - ui.spacing().interact_size.x)
        .max(0.0);
    ui.spacing_mut().slider_width = width;
    let mut value = (spec.get)(sliders);
    let response = ui.add(
        egui::Slider::new(&mut value, 1..=spec.max)
            .clamping(egui::SliderClamping::Edits)
            .trailing_fill(true),
    );
    // The handle's rect as egui computes it (its slider geometry is
    // private, so it is recomputed here): the rail widget is the
    // first `slider_width` of the row `response` shares with the value
    // box, its radius the row's height / 2.5, the centre travelling
    // `radius * 0.75` in from each end for the default rectangle
    // handle. `interact` keeps the widget state's stroke (inactive,
    // hovered or active), `expansion` the state's own.
    if ui.is_rect_visible(response.rect) {
        let rail =
            egui::Rect::from_min_size(response.rect.min, egui::vec2(width, response.rect.height()));
        let radius = rail.height() / 2.5;
        let place = ((f32::from(value) - 1.0) / f32::from(spec.max - 1)).clamp(0.0, 1.0);
        let centre = egui::pos2(
            egui::lerp(rail.x_range().shrink(radius * 0.75), place),
            rail.center().y,
        );
        let visuals = ui.style().interact(&response);
        ui.painter().rect(
            egui::Rect::from_center_size(centre, egui::vec2(radius * 1.5, radius * 2.0))
                .expand(visuals.expansion),
            visuals.corner_radius,
            ui.visuals().selection.bg_fill,
            visuals.fg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    if response.changed() {
        (spec.set)(sliders, value);
    }
}

/// The "Advanced instructions" panel (the draft's `.group` over
/// `instrGrid`): the two sides' combos and their target chips of the
/// preset on screen (`instructions::instruction_table`). The chips
/// arm `view`'s target mode, never write the record — the answer is
/// `true` only when `tactics` changed this frame.
pub fn advanced_instructions(
    ui: &mut egui::Ui,
    version: PesVersion,
    tactics: &mut TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) -> bool {
    let preset = &mut tactics.presets[view.preset.min(2)];
    let before = preset.clone();
    panel(ui, "Advanced instructions", |ui| {
        // `add_space` adds to the item gap: the rows start `TITLE_GAP`
        // under the heading, as Formation settings' do.
        ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y);
        instructions::instruction_table(ui, version, preset, view, roster);
    });
    *preset != before
}

/// The "Player assignments" panel (the draft's `#tac-side` group): the
/// set-piece takers', the captain's and the join-attack chips
/// (`assignments::player_assignments`). They arm `view`'s mode and can
/// never write the record.
pub fn player_assignments(
    ui: &mut egui::Ui,
    tactics: &TeamTactics,
    view: &mut TacticsView,
    roster: &[RosterRow<'_>],
) {
    panel(ui, "Player assignments", |ui| {
        // `add_space` adds to the item gap: the rows start `TITLE_GAP`
        // under the heading, as Formation settings' do.
        ui.add_space(TITLE_GAP - ui.spacing().item_spacing.y);
        assignments::player_assignments(
            ui,
            &tactics.set_pieces,
            &tactics.players_to_join_attack,
            view,
            roster,
        );
    });
}

/// The "Automatic settings" panel (the draft's `#tac-auto` group over
/// `.tacauto`): one grid of two equal columns of 24-point rows —
/// Substitution beside Preset change, Offside trap beside
/// Attack/defence levels — its frame grown to the lists' bottom the
/// host bounds the cell to, the rows centred in it. Answers `true`
/// only when `tactics` changed this frame.
pub fn automatic_settings(
    ui: &mut egui::Ui,
    version: PesVersion,
    tactics: &mut TeamTactics,
) -> bool {
    let auto = &mut tactics.auto;
    let before = auto.clone();
    panel(ui, "Automatic settings", |ui| {
        // `#tac-auto { flex: 1 }`: the frame fills the cell the host
        // bounds to the lists' bottom.
        ui.set_min_height(ui.available_height());
        // `.tacauto`: the two 24-point rows centred in the free height
        // (`align-content: center`), the columns `1fr 1fr` 16 apart and
        // adjacent (no row-gap: `item_spacing.y` trails a widget, so it
        // is set before the first row).
        let free = ui.available_height();
        ui.add_space(((free - 2.0 * AUTO_ROW_HEIGHT) / 2.0).max(0.0));
        ui.spacing_mut().item_spacing.y = 0.0;
        auto_row(
            ui,
            |ui| substitution_cell(ui, version, &mut auto.substitution),
            |ui| {
                auto_flag(
                    ui,
                    PRESET_CHANGE_LABEL,
                    &mut auto.preset_change,
                    TacticsField::AutoPresetChange,
                );
            },
        );
        auto_row(
            ui,
            |ui| {
                auto_flag(
                    ui,
                    OFFSIDE_TRAP_LABEL,
                    &mut auto.offside_trap,
                    TacticsField::AutoOffsideTrap,
                );
            },
            |ui| {
                auto_flag(
                    ui,
                    ATTACK_DEFENCE_LABEL,
                    &mut auto.attack_defence_levels,
                    TacticsField::AutoAttackDefenceLevels,
                );
            },
        );
    });
    *auto != before
}

/// A `.tacauto` row's height (`grid-auto-rows: 24px`).
const AUTO_ROW_HEIGHT: f32 = 24.0;
/// The `.tacauto` columns' gap (`column-gap: 16px`).
const AUTO_COLUMN_GAP: f32 = 16.0;

/// One `.tacauto` row's two cells, each a 24-point row its content
/// centred in (`.tacauto` `align-items: center`), `AUTO_COLUMN_GAP`
/// apart.
fn auto_row(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    let column = (ui.available_width() - AUTO_COLUMN_GAP) / 2.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = AUTO_COLUMN_GAP;
        // `allocate_ui_with_layout` advances the row by the child's
        // *used* rect, not the size asked for: each cell claims its
        // whole column before its content, or the right cell would
        // start at the left cell's content's right edge.
        ui.allocate_ui_with_layout(
            egui::vec2(column, AUTO_ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::LEFT),
            |ui| {
                ui.set_min_size(egui::vec2(column, AUTO_ROW_HEIGHT));
                left(ui);
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(column, AUTO_ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center).with_main_align(egui::Align::LEFT),
            |ui| {
                ui.set_min_size(egui::vec2(column, AUTO_ROW_HEIGHT));
                right(ui);
            },
        );
    });
}

/// The Substitution combo's offered choices (the draft's list): Off,
/// Very late and Flexible, plus Very early from PES 2017 on (measured
/// on 2017 and 2021, the versions between taken to match).
/// `tactics_term` words the stored value the same way, so the row and
/// the combo cannot drift.
fn substitution_choices(version: PesVersion) -> Vec<(u8, &'static str)> {
    [(0, "Off"), (1, "Very late"), (2, "Flexible")]
        .into_iter()
        .chain(
            matches!(
                version,
                PesVersion::Pes17
                    | PesVersion::Pes18
                    | PesVersion::Pes19
                    | PesVersion::Pes20
                    | PesVersion::Pes21
            )
            .then_some((3, "Very early")),
        )
        .collect()
}

/// The Substitution cell (the draft's `subs` row): the label, then the
/// version's combo filling the rest of the cell — Off, Very late and
/// Flexible, plus Very early from PES 2017 on (measured on 2017 and
/// 2021, the versions between taken to match); a stored value outside
/// the offered ones appended as its number, so the combo never
/// misreports the save. A version that stores none keeps the disabled
/// cell and the `PES NN+` tooltip.
fn substitution_cell(ui: &mut egui::Ui, version: PesVersion, substitution: &mut Option<u8>) {
    ui.label(SUBSTITUTION_LABEL);
    match substitution {
        Some(stored) => {
            let mut choices: Vec<(u8, String)> = substitution_choices(version)
                .into_iter()
                .map(|(value, label)| (value, label.to_owned()))
                .collect();
            // A stored value outside the offered ones is offered as its
            // number, so the combo never misreports the save.
            if !choices.iter().any(|(value, _)| value == stored) {
                choices.push((*stored, stored.to_string()));
            }
            let selected = choices
                .iter()
                .find(|(value, _)| value == stored)
                .map(|(_, label)| label.clone())
                .expect("the stored value is always offered");
            let mut pick = *stored;
            let combo = egui::ComboBox::from_id_salt("automatic substitution")
                .width(ui.available_width())
                .selected_text(selected)
                .truncate()
                .show_ui(ui, |ui| {
                    for (value, label) in &choices {
                        ui.selectable_value(&mut pick, *value, label.clone());
                    }
                })
                .response;
            let values = choices.iter().map(|(value, _)| *value);
            if let Some(next) = step_choice(values, &pick, wheel_steps(ui, &combo)) {
                pick = next;
            }
            if pick != *stored {
                *stored = pick;
            }
        }
        None => {
            add_enabled(
                ui,
                false,
                egui::Button::new("-").min_size(ui.spacing().interact_size),
            )
            .on_disabled_hover_text(gate_tip(TacticsField::AutoSubstitution));
        }
    }
}

/// One automatic-settings flag's checkbox; a version that does not store
/// it keeps it, disabled with the `PES NN+` tooltip.
fn auto_flag(ui: &mut egui::Ui, label: &str, value: &mut Option<bool>, field: TacticsField) {
    match value {
        Some(flag) => {
            checkbox(ui, flag, label);
        }
        None => {
            let mut off = false;
            add_enabled(ui, false, |ui: &mut egui::Ui| checkbox(ui, &mut off, label))
                .on_disabled_hover_text(gate_tip(field));
        }
    }
}

/// The `PES NN+` tooltip naming the first version whose tactics record
/// stores `field` (the card pickers' rule for a field a version lacks).
fn gate_tip(field: TacticsField) -> String {
    format!("PES {}+", first_tactics_version(field).number())
}

/// The oldest version whose tactics record stores `field` (a field the
/// widget gates is stored by some version).
fn first_tactics_version(field: TacticsField) -> PesVersion {
    PesVersion::ALL
        .iter()
        .copied()
        .find(|version| schema_for(*version).tactic.has(field))
        .expect("a gated field is stored by some version")
}

/// Roster `slot`'s display name, `(empty)` when `roster` is shorter than
/// the slot or the slot names nobody.
pub(crate) fn name_at<'a>(roster: &[RosterRow<'a>], slot: u8) -> &'a str {
    roster
        .get(usize::from(slot))
        .map(|row| row.name)
        .filter(|name| !name.is_empty())
        .unwrap_or("(empty)")
}

/// `name_at` on the raw name (the colour codes kept): the lineup rows
/// and the markers draw it for the coloured text runs.
pub(crate) fn raw_name_at<'a>(roster: &[RosterRow<'a>], slot: u8) -> &'a str {
    raw_name(roster, slot).unwrap_or("(empty)")
}

/// The raw name of roster `slot`, `None` for an empty slot (or one past
/// the roster): each caller words the empty case its own way (the slot
/// popover's header says "(empty roster slot)").
pub(crate) fn raw_name<'a>(roster: &[RosterRow<'a>], slot: u8) -> Option<&'a str> {
    roster
        .get(usize::from(slot))
        .map(|row| row.raw_name)
        .filter(|name| !name.is_empty())
}

/// A tactics field's row as the Tactics tab words it, for the Save
/// editor's unsaved-changes prompt: the field's label, prefixed by its
/// preset ("Preset 1 · Support range"), and the two stored values as the
/// tab shows them (a style pair's texts, a taker's or the captain's
/// player name, a slider's number). `version` decodes the stored
/// instruction bytes; `new_tactics` names the formation slots' players
/// (the starting eleven's roster slots) and `old_tactics`/`new_tactics`
/// carry the instruction each side's target byte is worded under. A
/// stored slot names its player through the roster of its own side —
/// `rosters` holds the file-as-opened's first, the current's second: a
/// drag moves records between the roster's ids while the byte still
/// names the slot, so one roster would word a side under the wrong
/// player.
pub fn tactics_term(
    version: PesVersion,
    field: TacticsField,
    old: u32,
    new: u32,
    old_tactics: &TeamTactics,
    new_tactics: &TeamTactics,
    rosters: (&[RosterRow<'_>], &[RosterRow<'_>]),
) -> (String, String, String) {
    let (old_roster, new_roster) = rosters;
    let byte = |stored: u32| u8::try_from(stored).expect("a tactics field is a byte");
    match field {
        TacticsField::TeamId => ("Team ID".to_owned(), old.to_string(), new.to_string()),
        TacticsField::Preset { preset, field: pf } => (
            format!("Preset {} · {}", preset + 1, preset_label(pf)),
            preset_value(pf, old),
            preset_value(pf, new),
        ),
        TacticsField::Formation {
            preset,
            formation,
            slot,
            part,
        } => {
            let starter = new_tactics.starting_eleven[usize::from(slot)];
            let label = format!(
                "Preset {} · {} · {} · {}",
                preset + 1,
                FORMATIONS[usize::from(formation)],
                name_at(new_roster, starter),
                match part {
                    FormationPart::Position => "Position",
                    FormationPart::X => "X",
                    FormationPart::Y => "Y",
                }
            );
            let value = |stored: u32| match part {
                FormationPart::Position => position_label(byte(stored)).to_owned(),
                FormationPart::X | FormationPart::Y => stored.to_string(),
            };
            (label, value(old), value(new))
        }
        TacticsField::Instruction {
            preset,
            side,
            index,
            part,
        } => {
            let row = format!(
                "Preset {} · {} {}",
                preset + 1,
                match side {
                    InstructionSide::Attack => "Attack",
                    InstructionSide::Defence => "Defence",
                },
                index + 1
            );
            match part {
                InstructionPart::Instruction => {
                    let value = |stored: u32| {
                        let stored = byte(stored);
                        instruction::decode(version, stored)
                            .ok()
                            .flatten()
                            .map_or_else(
                                || format!("Unknown (0x{stored:02X})"),
                                |inst| instructions::instruction_label(inst).to_owned(),
                            )
                    };
                    (row, value(old), value(new))
                }
                InstructionPart::PlayerId => (
                    format!("{row} · Target"),
                    instructions::target_text(
                        instructions::instruction_at(version, old_tactics, preset, side, index),
                        byte(old),
                        old_roster,
                    ),
                    instructions::target_text(
                        instructions::instruction_at(version, new_tactics, preset, side, index),
                        byte(new),
                        new_roster,
                    ),
                ),
            }
        }
        // The byte is the marked opponent's formation slot (255 none), so
        // a bare number would read as one of the team's own players.
        TacticsField::ManMarking { preset, slot } => {
            let starter = new_tactics.starting_eleven[usize::from(slot)];
            let value = |stored: u32| {
                if stored == u32::from(u8::MAX) {
                    "None".to_owned()
                } else {
                    format!("Opponent's starter {}", stored + 1)
                }
            };
            (
                format!(
                    "Preset {} · Man marking · {}",
                    preset + 1,
                    name_at(new_roster, starter)
                ),
                value(old),
                value(new),
            )
        }
        TacticsField::FreeKickTakerLong
        | TacticsField::FreeKickTakerShort
        | TacticsField::FreeKickTakerSecond
        | TacticsField::CornerTakerLeft
        | TacticsField::CornerTakerRight
        | TacticsField::PenaltyTaker
        | TacticsField::Captain => (
            assignments::role_label(field)
                .expect("the roles are the chips' own table")
                .to_owned(),
            role_holder(old_roster, byte(old)),
            role_holder(new_roster, byte(new)),
        ),
        TacticsField::AutoSubstitution => {
            let value = |stored: u32| {
                substitution_choices(version)
                    .iter()
                    .find(|(value, _)| u32::from(*value) == stored)
                    .map(|(_, label)| (*label).to_owned())
                    .unwrap_or_else(|| stored.to_string())
            };
            (SUBSTITUTION_LABEL.to_owned(), value(old), value(new))
        }
        TacticsField::AutoOffsideTrap => (
            OFFSIDE_TRAP_LABEL.to_owned(),
            on_off(old).to_owned(),
            on_off(new).to_owned(),
        ),
        TacticsField::AutoPresetChange => (
            PRESET_CHANGE_LABEL.to_owned(),
            on_off(old).to_owned(),
            on_off(new).to_owned(),
        ),
        TacticsField::AutoAttackDefenceLevels => (
            ATTACK_DEFENCE_LABEL.to_owned(),
            on_off(old).to_owned(),
            on_off(new).to_owned(),
        ),
        TacticsField::Starting(index) => (
            format!("Starting eleven · {}", index + 1),
            name_at(old_roster, byte(old)).to_owned(),
            name_at(new_roster, byte(new)).to_owned(),
        ),
        TacticsField::Bench(index) => (
            format!("Bench · {}", index + 1),
            name_at(old_roster, byte(old)).to_owned(),
            name_at(new_roster, byte(new)).to_owned(),
        ),
        TacticsField::PlayerToJoinAttack(index) => (
            format!("Join attack · {}", index + 1),
            role_holder(old_roster, byte(old)),
            role_holder(new_roster, byte(new)),
        ),
    }
}

/// A preset field's label on the tab (`SLIDERS`, `STYLE_TOGGLES` or the
/// fluid switch): `tactics_term` reads the tab's own tables, never a
/// second copy.
fn preset_label(field: PresetField) -> &'static str {
    if field == PresetField::FluidFormation {
        return FLUID_LABEL;
    }
    SLIDERS
        .iter()
        .find(|spec| spec.field == field)
        .map(|spec| spec.label)
        .or_else(|| {
            STYLE_TOGGLES
                .iter()
                .find(|toggle| toggle.field == field)
                .map(|toggle| toggle.label)
        })
        .expect("a preset field is a slider, a style pair or the fluid switch")
}

/// `stored` as the preset field's control shows it: a style pair's text,
/// "off"/"on" for the fluid switch, a numbers toggle's `TRI_CHOICES`
/// text (a stored value outside 1-3 its number, as the toggle's
/// Substitution rule), the number else.
fn preset_value(field: PresetField, stored: u32) -> String {
    if let Some(toggle) = STYLE_TOGGLES.iter().find(|toggle| toggle.field == field) {
        return if stored == 0 { toggle.off } else { toggle.on }.to_owned();
    }
    if field == PresetField::FluidFormation {
        return on_off(stored).to_owned();
    }
    if NUMBERS.contains(&field)
        && let Some(label) = stored
            .checked_sub(1)
            .and_then(|index| TRI_CHOICES.get(index as usize))
    {
        return (*label).to_owned();
    }
    stored.to_string()
}

/// A role's or join member's holder text: "Unassigned" for the none
/// byte, the roster slot's name else (the chips' own wording).
fn role_holder(roster: &[RosterRow<'_>], holder: u8) -> String {
    if holder == u8::MAX {
        return "Unassigned".to_owned();
    }
    name_at(roster, holder).to_owned()
}

/// A flag byte's text: a set bit reads "on".
fn on_off(stored: u32) -> &'static str {
    if stored == 0 { "off" } else { "on" }
}

#[cfg(test)]
mod tests;
