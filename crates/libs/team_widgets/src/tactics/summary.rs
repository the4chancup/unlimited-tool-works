//! A preset's settings and the team's Automatic settings as lines of text,
//! worded as the Tactics editor's controls word them, for a picture that
//! lists them beside the formations (the save editor's formation pictures).

use pes_savefile::model::tactics::{AdvancedInstruction, TacticsPreset, TeamTactics};
use pes_savefile::schema::fields::PresetField;
use pes_savefile::schema::instruction;
use pes_version::PesVersion;

use super::instructions::{NO_TARGET, target_text};
use super::{
    ATTACK_DEFENCE_LABEL, OFFSIDE_TRAP_LABEL, PRESET_CHANGE_LABEL, RosterRow, SLIDERS,
    STYLE_TOGGLES, SUBSTITUTION_LABEL, TAC_DEFENSIVE, TAC_OFFENSIVE, instruction_text,
    preset_label, preset_value, substitution_text,
};

/// One line of a preset's settings column in a formation picture.
#[derive(Debug, PartialEq, Eq)]
pub enum SettingsLine {
    /// A heading: "Formation settings" or "Advanced instructions".
    Heading(&'static str),
    /// A setting's label and its value as the editor's control names it.
    Setting {
        /// The setting's label, "Attacking style" or "Attack 1".
        label: String,
        /// Its value, "Counter Attack" or "Hug the Touchline".
        value: String,
    },
    /// The player the instruction on the line above targets.
    Target(String),
    /// Half a line's room between two groups.
    Gap,
}

/// The settings of `tactics`' preset `preset` (0 to 2) in the editor's
/// order: the Formation settings' offensive half, then its defensive half,
/// then the Advanced instructions on a version that stores them (PES 2017
/// on), each targeted player on a line under the instruction. `roster`
/// names the targets by roster slot.
pub fn preset_settings(
    version: PesVersion,
    tactics: &TeamTactics,
    preset: usize,
    roster: &[RosterRow<'_>],
) -> Vec<SettingsLine> {
    let stored = &tactics.presets[preset];
    let setting = |field: &PresetField| SettingsLine::Setting {
        label: preset_label(version, *field).to_owned(),
        value: preset_value(*field, stored_value(stored, *field)),
    };
    let mut lines = vec![SettingsLine::Heading("Formation settings")];
    lines.extend(TAC_OFFENSIVE.iter().map(setting));
    lines.push(SettingsLine::Gap);
    lines.extend(TAC_DEFENSIVE.iter().map(setting));
    let (Some(attack), Some(defence)) = (&stored.attack_instructions, &stored.defence_instructions)
    else {
        return lines;
    };
    lines.push(SettingsLine::Gap);
    lines.push(SettingsLine::Heading("Advanced instructions"));
    for (side, entries) in [("Attack", attack), ("Defence", defence)] {
        for (index, entry) in entries.iter().enumerate() {
            lines.extend(instruction_lines(
                version,
                format!("{side} {}", index + 1),
                entry,
                roster,
            ));
        }
    }
    lines
}

/// `field`'s stored value in `preset`, read through the editor's own
/// tables: a slider's number, or a style pair's flag as 0 or 1.
fn stored_value(preset: &TacticsPreset, field: PresetField) -> u32 {
    if let Some(spec) = SLIDERS.iter().find(|spec| spec.field == field) {
        return u32::from((spec.get)(&preset.sliders));
    }
    let toggle = STYLE_TOGGLES
        .iter()
        .find(|toggle| toggle.field == field)
        .expect("a Formation settings row is a slider or a style pair");
    u32::from((toggle.get)(&preset.style))
}

/// One instruction's line, `label` its row's ("Attack 1"), and its target's
/// line under it when the instruction takes one of the team's players and
/// one is set. A byte the version's table does not decode reads
/// "Unknown (0xNN)", as the unsaved-changes rows read it.
fn instruction_lines(
    version: PesVersion,
    label: String,
    entry: &AdvancedInstruction,
    roster: &[RosterRow<'_>],
) -> Vec<SettingsLine> {
    let decoded = instruction::decode(version, entry.instruction)
        .ok()
        .flatten();
    let value = instruction_text(version, entry.instruction);
    let mut lines = vec![SettingsLine::Setting { label, value }];
    if decoded.is_some_and(|inst| inst.takes_player()) && entry.player != NO_TARGET {
        lines.push(SettingsLine::Target(target_text(
            decoded,
            entry.player,
            roster,
        )));
    }
    lines
}

/// The team's Automatic settings as (label, value) pairs, in the panel's
/// order: Substitution, Preset change, Offside trap, Attack/defence
/// levels, each only when `version`'s save stores it. Substitution reads as
/// its combo's choice (its number when the stored value is none of them),
/// a flag "On" or "Off".
pub fn automatic_pairs(version: PesVersion, tactics: &TeamTactics) -> Vec<(&'static str, String)> {
    let auto = &tactics.auto;
    let on_off = |flag: bool| if flag { "On" } else { "Off" }.to_owned();
    [
        (
            SUBSTITUTION_LABEL,
            auto.substitution
                .map(|stored| substitution_text(version, stored)),
        ),
        (PRESET_CHANGE_LABEL, auto.preset_change.map(on_off)),
        (OFFSIDE_TRAP_LABEL, auto.offside_trap.map(on_off)),
        (ATTACK_DEFENCE_LABEL, auto.attack_defence_levels.map(on_off)),
    ]
    .into_iter()
    .filter_map(|(label, value)| Some((label, value?)))
    .collect()
}

#[cfg(test)]
mod tests {
    use pes_savefile::model::instruction::Instruction;

    use super::*;
    use crate::tactics::tests::named_roster;
    use crate::test_support::{fixture_tactics, roster_rows};

    /// A setting's line.
    fn setting(label: &str, value: &str) -> SettingsLine {
        SettingsLine::Setting {
            label: label.to_owned(),
            value: value.to_owned(),
        }
    }

    /// The two numbers' labels up to PES 2016.
    const NUMBERS_LABELS: [&str; 2] = ["Numbers in attack", "Numbers in defence"];
    /// The two numbers' labels from PES 2017 on.
    const AUTO_LEVEL_LABELS: [&str; 2] = ["Auto attack level", "Auto defence level"];

    /// The Formation settings' lines of the fixture's preset 1 as the test
    /// below edits it: the offensive half, a gap, the defensive half, the
    /// two numbers under `numbers` (attack, defence).
    fn formation_settings(numbers: [&str; 2]) -> Vec<SettingsLine> {
        vec![
            SettingsLine::Heading("Formation settings"),
            setting("Attacking style", "Counter Attack"),
            setting("Buildup", "Short Pass"),
            setting("Attacking zone", "Centre"),
            setting("Positioning", "Maintain"),
            setting("Support range", "4"),
            setting(numbers[0], "Medium"),
            SettingsLine::Gap,
            setting("Defensive style", "Frontline Pressure"),
            setting("Containment area", "Middle"),
            setting("Pressure", "Conservative"),
            setting("Defensive line", "5"),
            setting("Compactness", "5"),
            setting(numbers[1], "Many"),
        ]
    }

    /// Team 701's preset 1 with a few settings off their defaults, so each
    /// line reads its own field: Buildup and Pressure on, Support range 4,
    /// Numbers in defence 3.
    fn edited(version: PesVersion) -> TeamTactics {
        let mut tactics = fixture_tactics(version);
        let preset = &mut tactics.presets[0];
        assert_eq!(
            (
                preset.style.buildup,
                preset.style.pressure,
                preset.sliders.support_range,
                preset.sliders.numbers_in_defence
            ),
            (false, false, 5, 2),
            "premise: the fixture's defaults"
        );
        preset.style.buildup = true;
        preset.style.pressure = true;
        preset.sliders.support_range = 4;
        preset.sliders.numbers_in_defence = 3;
        tactics
    }

    #[test]
    fn a_preset_s_settings_read_as_the_editor_words_them() {
        let version = PesVersion::Pes21;
        let mut tactics = edited(version);
        // Attack 1 takes a player but has none set, Attack 2 targets slot 5,
        // Defence 1 takes none, Defence 2 is a byte the table lacks.
        let encode = |inst| instruction::encode(version, inst).expect("PES 21 stores it");
        let preset = &mut tactics.presets[0];
        let attack = preset
            .attack_instructions
            .as_mut()
            .expect("PES 21 has instructions");
        attack[0] = AdvancedInstruction {
            instruction: encode(Instruction::CounterTarget),
            player: NO_TARGET,
        };
        attack[1] = AdvancedInstruction {
            instruction: encode(Instruction::Defensive),
            player: 5,
        };
        let defence = preset
            .defence_instructions
            .as_mut()
            .expect("PES 21 has instructions");
        defence[0] = AdvancedInstruction {
            instruction: encode(Instruction::AttackingFullBacks),
            player: 5,
        };
        defence[1].instruction = 0xAB;
        assert_eq!(
            instruction::decode(version, 0xAB).ok().flatten(),
            None,
            "premise: 0xAB decodes to nothing"
        );
        let rows = named_roster();
        let mut expected = formation_settings(AUTO_LEVEL_LABELS);
        expected.extend([
            SettingsLine::Gap,
            SettingsLine::Heading("Advanced instructions"),
            setting("Attack 1", "Counter Target"),
            setting("Attack 2", "Defensive"),
            SettingsLine::Target("Player 06".to_owned()),
            setting("Defence 1", "Attacking Full-Backs"),
            setting("Defence 2", "Unknown (0xAB)"),
        ]);
        assert_eq!(
            preset_settings(version, &tactics, 0, &roster_rows(&rows)),
            expected
        );
    }

    #[test]
    fn a_version_without_instructions_lists_the_formation_settings_alone() {
        let version = PesVersion::Pes16;
        let tactics = edited(version);
        assert_eq!(
            tactics.presets[0].attack_instructions, None,
            "premise: PES 2016 stores no instructions"
        );
        let rows = named_roster();
        assert_eq!(
            preset_settings(version, &tactics, 0, &roster_rows(&rows)),
            formation_settings(NUMBERS_LABELS)
        );
    }

    // The two numbers are named as the version's toggles name them: Auto
    // attack level and Auto defence level from PES 2017 on.
    #[test]
    fn the_numbers_are_named_by_the_version() {
        let rows = named_roster();
        let label = |line: &SettingsLine| match line {
            SettingsLine::Setting { label, .. } => label.clone(),
            other => panic!("{other:?} is no setting"),
        };
        for (version, labels) in [
            (PesVersion::Pes16, NUMBERS_LABELS),
            (PesVersion::Pes17, AUTO_LEVEL_LABELS),
        ] {
            let tactics = fixture_tactics(version);
            let lines = preset_settings(version, &tactics, 0, &roster_rows(&rows));
            // The last line of each half, as `formation_settings` lays
            // them out.
            assert_eq!([label(&lines[6]), label(&lines[13])], labels, "{version:?}");
        }
    }

    #[test]
    fn the_automatic_settings_are_the_ones_the_version_stores() {
        let mut tactics = fixture_tactics(PesVersion::Pes21);
        tactics.auto.substitution = Some(3);
        tactics.auto.offside_trap = Some(true);
        assert_eq!(
            automatic_pairs(PesVersion::Pes21, &tactics),
            [
                ("Substitution", "Very early".to_owned()),
                ("Preset change", "Off".to_owned()),
                ("Offside trap", "On".to_owned()),
                ("Attack/defence levels", "Off".to_owned()),
            ]
        );
        // PES 2016 stores no Attack/defence levels and offers no Very
        // early: a stored 3 reads as its number.
        let mut tactics = fixture_tactics(PesVersion::Pes16);
        assert_eq!(
            tactics.auto.attack_defence_levels, None,
            "premise: PES 2016 stores none"
        );
        tactics.auto.substitution = Some(3);
        tactics.auto.preset_change = Some(true);
        assert_eq!(
            automatic_pairs(PesVersion::Pes16, &tactics),
            [
                ("Substitution", "3".to_owned()),
                ("Preset change", "On".to_owned()),
                ("Offside trap", "Off".to_owned()),
            ]
        );
    }
}
