//! The Ruleset editor's command line run in-process: the findings, the exit
//! code and the files on disk, observed as the `Message` events the tool emits
//! and the files the commands write.

#[path = "../common/mod.rs"]
mod common;

mod summary;
mod update_logic;
mod validate;

use std::fs;
use std::path::Path;

/// A valid assembled rules file: the embedded official ruleset written as
/// the tool's save writes it.
pub(crate) fn valid_file() -> String {
    aatf::assemble_file(aatf::Rules::official().ruleset(), None, None)
}

/// The first RE-FILE-03 file: a parse error at line 12 (the comma between
/// `rating`'s `ball_winning` and `jump` dropped).
pub(crate) fn parse_error_file() -> String {
    let text = valid_file().replacen("\"ball_winning\", \"jump\"", "\"ball_winning\" \"jump\"", 1);
    assert!(
        text.lines()
            .nth(11)
            .unwrap_or_default()
            .contains("\"ball_winning\" \"jump\""),
        "premise: the edit sits on line 12"
    );
    text
}

/// The second RE-FILE-03 file: `tiers[2].free.tricks` is the string "two".
pub(crate) fn schema_error_file() -> String {
    let mut text = valid_file();
    let at = text.find("id: \"bronze\"").expect("the bronze tier");
    let pos = at + text[at..].find("tricks: 2").expect("its free cards");
    text.replace_range(pos..pos + "tricks: 2".len(), "tricks: \"two\"");
    text
}

/// The third RE-FILE-03 file: the second tier (`silver`) carries the
/// first tier's id (`gold`).
pub(crate) fn duplicate_id_file() -> String {
    let text = valid_file();
    let gold = text.find("id: \"gold\"").expect("the gold tier");
    let silver = text.find("id: \"silver\"").expect("the silver tier");
    assert!(gold < silver, "premise: gold is the first tier");
    text.replacen("id: \"silver\"", "id: \"gold\"", 1)
}

/// The embedded ruleset `name` (`libs/aatf`'s `rulesets/<name>.rhai`)
/// assembled as a rules file with the current logic.
pub(crate) fn embedded_file(name: &str) -> String {
    let aatf_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libs/aatf");
    let block = fs::read_to_string(aatf_dir.join(format!("rulesets/{name}.rhai"))).unwrap();
    let ruleset = aatf::Ruleset::from_data_block(&block).unwrap();
    aatf::assemble_file(&ruleset, None, None)
}

/// `text` written to `sandbox`'s `relative`; its `arg` form.
pub(crate) fn write(sandbox: &common::Sandbox, relative: &str, text: &str) -> String {
    fs::write(sandbox.path(relative), text).unwrap();
    sandbox.arg(relative)
}
