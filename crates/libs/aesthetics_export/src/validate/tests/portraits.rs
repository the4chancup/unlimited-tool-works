use std::collections::BTreeSet;

use pes_version::PesVersion;

use super::*;
use crate::PlayerSlot;

/// Each `portrait_missing` finding of `report`, in issue order, as its folder and its `slot`
/// context; every one is `Keep`.
fn portraits_missing(report: &ValidationReport) -> Vec<(&str, &str)> {
    report
        .issues
        .iter()
        .filter(|issue| issue.code == "portrait_missing")
        .map(|issue| {
            assert_eq!(issue.disposition, Disposition::Keep);
            let IssueScope::Folder(path) = &issue.scope else {
                panic!("portrait_missing on {:?}", issue.scope);
            };
            let [("slot", slot)] = issue.context.as_slice() else {
                panic!("portrait_missing context {:?}", issue.context);
            };
            (path.as_str(), slot.as_str())
        })
        .collect()
}

/// The default context at `version`, the installed CPKs holding the portraits of `installed`.
fn installed_portraits(version: PesVersion, installed: Option<&[u8]>) -> ValidationContext {
    ValidationContext {
        installed_portrait_slots: installed.map(|slots| {
            slots
                .iter()
                .map(|slot| PlayerSlot::new(*slot).unwrap())
                .collect::<BTreeSet<_>>()
        }),
        ..context_for(version)
    }
}

/// Folder 01 with its own portrait, 02 with `Portraits/02.dds`, 03 with neither, and 04 with
/// only a `Portraits/` file whose name maps to no slot.
const FOUR_PLAYERS: [(&str, u64); 6] = [
    ("Players/01 - A/portrait.dds", 9),
    ("Players/02 - B/hair.dds", 9),
    ("Players/03 - C/hair.dds", 9),
    ("Players/04 - D/hair.dds", 9),
    ("Portraits/02.dds", 9),
    ("Portraits/player_04.dds", 9),
];

#[test]
fn a_full_export_reports_each_player_with_no_portrait_from_pes_17() {
    for (version, expected) in [
        (PesVersion::Pes15, vec![]),
        (PesVersion::Pes16, vec![]),
        (
            PesVersion::Pes17,
            vec![("Players/03 - C", "03"), ("Players/04 - D", "04")],
        ),
        (
            PesVersion::Pes21,
            vec![("Players/03 - C", "03"), ("Players/04 - D", "04")],
        ),
    ] {
        let report = report_with(
            &context_for(version),
            "co Full Spring",
            &[&FOUR_PLAYERS[..], &[("logo.png", 9)]].concat(),
            &[],
            &[],
        );
        assert_eq!(portraits_missing(&report), expected, "{version:?}");
        // The misnamed file is no one's portrait: it has its own finding, and its player is
        // compiled all the same.
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "portrait_name_invalid"),
            "{version:?}"
        );
        assert_eq!(report.validated.unwrap().players.len(), 4, "{version:?}");
    }
}

#[test]
fn a_midcup_export_reports_only_the_slots_the_installed_cpks_hold_no_portrait_for() {
    let files = [
        ("Players/03 - C/hair.dds", 9),
        ("Players/04 - D/hair.dds", 9),
    ];
    for (installed, expected) in [
        (Some(&[3][..]), vec![("Players/04 - D", "04")]),
        (
            Some(&[][..]),
            vec![("Players/03 - C", "03"), ("Players/04 - D", "04")],
        ),
        // Nothing is known about what is installed.
        (None, vec![]),
    ] {
        let context = installed_portraits(PesVersion::Pes21, installed);
        let report = report_with(&context, "co Midcup Day 5", &files, &[], &[]);
        assert_eq!(portraits_missing(&report), expected, "{installed:?}");
    }
}

#[test]
fn a_full_export_ignores_the_installed_portraits() {
    let context = installed_portraits(PesVersion::Pes21, Some(&[3]));
    let report = report_with(
        &context,
        "co Full Spring",
        &[("logo.png", 9), ("Players/03 - C/hair.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(portraits_missing(&report), vec![("Players/03 - C", "03")]);
}

#[test]
fn a_portrait_file_that_maps_to_the_slot_counts_whatever_a_later_check_finds() {
    // The two files of slot 03 conflict and are both dropped, but each names the slot.
    let report = report(
        "co Full Spring",
        &[
            ("logo.png", 9),
            ("Players/03 - C/hair.dds", 9),
            ("Portraits/03.dds", 9),
            ("Portraits/03 - C.png", 9),
        ],
        &[],
        &[],
    );
    assert_eq!(
        issue_codes(&report),
        vec![
            ("texture_stem_conflict", Disposition::DropFile),
            ("texture_stem_conflict", Disposition::DropFile),
        ]
    );
}

#[test]
fn a_portrait_below_a_subfolder_is_not_the_players() {
    let report = report(
        "co Full Spring",
        &[("logo.png", 9), ("Players/03 - C/old/portrait.dds", 9)],
        &[],
        &[],
    );
    assert_eq!(portraits_missing(&report), vec![("Players/03 - C", "03")]);
}

#[test]
fn a_referee_export_reports_no_portrait_missing() {
    let report = report(
        "refs Cup",
        &[("players.txt", 10), ("Players/Keeper/hair.dds", 9)],
        &[],
        &[("players.txt", Ok(b"01 Keeper"))],
    );
    assert_eq!(report.parsed.draft.coverage, Some(ExportCoverage::Full));
    assert_eq!(issue_codes(&report), vec![]);
}
