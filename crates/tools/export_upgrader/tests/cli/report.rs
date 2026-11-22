//! The report's own rules (the `spec/export_upgrader.md` "Report" scenarios): every
//! entry no step migrates is named as unmigrated content and the output
//! written without it, and a migration needing no manual attention ends
//! clean.

use std::fs;

use pes_version::PesVersion;
use studio_core::{PipelineEvent, Severity};

use crate::common::Sandbox;
use crate::players::{boots, face, unmigrated_found, upgrade, wears};

// EU-REP-01
#[test]
fn each_other_file_is_named_and_dropped() {
    let sandbox = Sandbox::new("rep_other");
    sandbox.logo("co Spring");
    face(&sandbox, "co Spring/Faces/XXX05 - Bob");
    sandbox.write("co Spring/Other/readme.txt", b"x");
    sandbox.write("co Spring/Other/docs/scan.pdf", b"pdf");
    sandbox.save("EDIT");

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let stdout = run.stdout();
    for path in ["Other/readme.txt", "Other/docs/scan.pdf"] {
        assert!(
            stdout.contains(&format!(
                "- co Spring: Warning leftover_dropped at {path}\n"
            )),
            "{path} not in:\n{stdout}"
        );
    }
    assert!(stdout.contains("- Info migration_clean"), "{stdout}");
    assert!(!sandbox.path("co Full Spring/Other").exists());
    for entry in sandbox.listing("co Full Spring") {
        let name = entry.rsplit('/').next().unwrap_or(&entry);
        assert!(name != "readme.txt" && name != "scan.pdf", "{entry}");
    }
    assert_eq!(unmigrated_found(&run), 0);
}

// EU-REP-04
#[test]
fn a_leftover_model_decides_no_engine() {
    let sandbox = Sandbox::new("rep_leftover_model");
    sandbox.logo("co Spring");
    // As the real KFG VGL26 export: a Fox boots folder beside a side folder
    // of `.model` copies and a root file. The leftover `.model` counted
    // would read the export as pre-Fox, which never merges item folders.
    sandbox.write("co Spring/Boots/k0128 - Rex/boots.fmdl", b"boots");
    sandbox.write("co Spring/FMDL Boots/k0128 - Rex/boots.model", b"model");
    sandbox.write("co Spring/readme.pdf", b"pdf");

    let input = sandbox.arg("co Spring");
    let run = sandbox.run(&["upgrade", &input, "--loose"]);
    let stdout = run.stdout();
    for path in ["FMDL Boots/k0128 - Rex/boots.model", "readme.pdf"] {
        assert!(
            stdout.contains(&format!("Warning leftover_dropped at {path}")),
            "{path} not in:\n{stdout}"
        );
    }
    // Loose mode: k0128 is slot 03 of the block its kind's lowest ID opens.
    assert_eq!(
        fs::read(sandbox.path("co Full Spring/Players/03 - Rex/boots.fmdl")).unwrap(),
        b"boots",
        "{stdout}"
    );
    assert!(!sandbox.path("co Full Spring/Boots").exists(), "{stdout}");
    assert_eq!(run.exit_code(), 0, "{stdout}");
}

// EU-REP-03
#[test]
fn a_migration_needing_no_attention_is_clean() {
    let sandbox = Sandbox::new("rep_clean");
    sandbox.logo("co Spring");
    face(&sandbox, "co Spring/Faces/XXX05 - Bob");
    boots(&sandbox, "co Spring/Boots/k0430 - Crocs");
    sandbox.save_edited("EDIT", PesVersion::Pes21, |file| {
        wears(file, 71405, 430, 0);
    });

    let run = upgrade(&sandbox, "co Spring", "EDIT");
    assert_eq!(run.exit_code(), 0, "{}", run.stdout());
    let offenders: Vec<String> = run
        .events
        .iter()
        .filter_map(|envelope| match &envelope.event {
            PipelineEvent::Message(message) if message.severity > Severity::Info => {
                Some(format!("{:?} {}", message.severity, message.code.code))
            }
            _ => None,
        })
        .collect();
    assert!(offenders.is_empty(), "{offenders:?}");
    let last = run
        .events
        .iter()
        .rev()
        .find_map(|envelope| match &envelope.event {
            PipelineEvent::Message(message) => Some(message),
            _ => None,
        })
        .expect("the verdict is a message");
    assert_eq!(last.code.code, "migration_clean");
    assert_eq!(unmigrated_found(&run), 0);
}
