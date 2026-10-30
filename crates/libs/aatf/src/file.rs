//! Assembling a rules file from a data block, the current logic and the
//! block's custom-checks section (the empty stub when it has none), and
//! reading a file's sections back.

use crate::ruleset::Ruleset;

/// The current logic section, banner included.
pub(crate) const LOGIC: &str = include_str!("../logic/logic.rhai");

/// The CUSTOM CHECKS banner line, under which a file's own checks go.
pub(crate) const CUSTOM_CHECKS: &str = "// ==== CUSTOM CHECKS: optional, hand-written; the Ruleset editor keeps this section as it is ====";

/// The offset of `text`'s CUSTOM CHECKS section: the first line
/// starting `// ==== CUSTOM CHECKS`, `text.len()` without one.
fn custom_checks_at(text: &str) -> usize {
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        if line.starts_with("// ==== CUSTOM CHECKS") {
            return offset;
        }
        offset += line.len();
    }
    text.len()
}

/// The CUSTOM CHECKS stub every assembled file without checks carries.
fn custom_checks_stub() -> String {
    format!("{CUSTOM_CHECKS}\nfn custom_checks(team) {{ [] }}")
}

/// `data`, `logic` and `custom` joined into the whole file: each part
/// written with its trailing whitespace trimmed, separated by a blank
/// line, the file ending in one newline.
fn join_file(data: &str, logic: &str, custom: &str) -> String {
    format!(
        "{}\n\n{}\n\n{}\n",
        data.trim_end(),
        logic.trim_end(),
        custom.trim_end()
    )
}

/// The whole rules file for `data_block`: the block, the current logic
/// and the block's CUSTOM CHECKS section verbatim, or the empty stub
/// when the block has none.
pub(crate) fn assemble(data_block: &str) -> String {
    let (data, section) = data_block.split_at(custom_checks_at(data_block));
    let custom = if section.is_empty() {
        custom_checks_stub()
    } else {
        section.to_string()
    };
    join_file(data, LOGIC, &custom)
}

/// The whole rules file for `ruleset`: its data block
/// (`Ruleset::to_data_block`), then `logic` (the current logic when
/// `None`), then `custom_checks` (the empty stub when `None`). Each section
/// is passed as `file_sections` reads it, its banner line first: one
/// without its banner reads back as part of the section before it.
pub fn assemble_file(
    ruleset: &Ruleset,
    logic: Option<&str>,
    custom_checks: Option<&str>,
) -> String {
    join_file(
        &ruleset.to_data_block(),
        logic.unwrap_or(LOGIC),
        custom_checks.unwrap_or(&custom_checks_stub()),
    )
}

/// A rules file's sections after its data block, as read
/// (`spec/aatf_rules.md` "The rules file").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSections {
    /// The LOGIC section, from its banner line to the CUSTOM CHECKS
    /// banner line or the end; `None` without a LOGIC banner.
    pub logic: Option<String>,
    /// The CUSTOM CHECKS section, banner included, to the end; `None`
    /// without one, and `None` when it is the empty stub every assembled
    /// file without checks carries: the stub counts as no checks.
    pub custom_checks: Option<String>,
}

/// `text`'s sections after its data block.
pub fn file_sections(text: &str) -> FileSections {
    let custom_checks = match custom_checks_at(text) {
        at if at >= text.len() => None,
        at => {
            let section = &text[at..];
            // The stub is normalized before comparing: line endings and
            // trailing whitespace are the file's layout, not checks.
            if section.replace("\r\n", "\n").trim_end() == custom_checks_stub() {
                None
            } else {
                Some(section.to_string())
            }
        }
    };
    FileSections {
        logic: logic_section(text).map(str::to_string),
        custom_checks,
    }
}

/// What a file's logic section is, against the logic this build has
/// shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicIdentity {
    /// The current logic.
    Current,
    /// An older official version (the Ruleset editor offers to update
    /// it).
    Official {
        /// The version the hash names.
        version: u32,
    },
    /// Neither: edited by hand, or without the LOGIC banner.
    Modified,
}

/// Every logic version this build knows, oldest first: (version,
/// SHA-256 of its section, lowercase hex). The last entry is the
/// current logic: until the first release, its hash follows every edit
/// of the logic; after it, a changed logic is a new version in the
/// banner and the table.
pub(crate) const LOGIC_VERSIONS: &[(u32, &str)] = &[(
    1,
    "b2c03566f3425ab013cb8f0148dc379239b23f950b0d5a2698a8fd6eb59de706",
)];

/// `text`'s logic section: from the line starting `// ==== LOGIC` to
/// the line starting `// ==== CUSTOM CHECKS`, or to the end of the text
/// when there is none; `None` without a LOGIC line.
fn logic_section(text: &str) -> Option<&str> {
    let mut start = None;
    let mut offset = 0usize;
    let mut end = text.len();
    for line in text.split_inclusive('\n') {
        if line.starts_with("// ==== LOGIC") {
            start = Some(offset);
        } else if start.is_some() && line.starts_with("// ==== CUSTOM CHECKS") {
            end = offset;
            break;
        }
        offset += line.len();
    }
    start.map(|s| &text[s..end])
}

/// A logic section's SHA-256, lowercase hex: line endings LF-normalized
/// and its trailing whitespace trimmed, because the blank lines before
/// the CUSTOM CHECKS banner are the file's layout, not the logic.
fn section_hash(section: &str) -> String {
    use sha2::Digest;
    let normalized = section.replace("\r\n", "\n");
    sha2::Sha256::digest(normalized.trim_end().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `text`'s logic identity against `versions`.
fn identity_with(text: &str, versions: &[(u32, &str)]) -> LogicIdentity {
    let Some(section) = logic_section(text) else {
        return LogicIdentity::Modified;
    };
    let hash = section_hash(section);
    match versions.iter().position(|&(_, known)| known == hash) {
        None => LogicIdentity::Modified,
        Some(at) if at + 1 == versions.len() => LogicIdentity::Current,
        Some(at) => LogicIdentity::Official {
            version: versions[at].0,
        },
    }
}

/// `text`'s logic identity against `LOGIC_VERSIONS`.
pub(crate) fn identity(text: &str) -> LogicIdentity {
    identity_with(text, LOGIC_VERSIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up older logic section.
    const OLD_SECTION: &str = "// ==== LOGIC (ruleset-logic 0): written by the Ruleset editor ====\nfn check_team(team) { [] }\n";

    const OFFICIAL: &str = "const RULESET = #{ schema: 1, name: \"x\", universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };\n";

    /// A file built from `section` as its logic section.
    fn file_with(section: &str) -> String {
        format!("{OFFICIAL}\n{section}\n\n{CUSTOM_CHECKS}\nfn custom_checks(team) {{ [] }}\n")
    }

    /// The assembled official file is `Current` — CRLF endings are the
    /// same logic.
    #[test]
    fn identity_is_current_and_crlf_is_the_same() {
        let older = section_hash(OLD_SECTION);
        let table = [(1u32, older.as_str()), (2, LOGIC_VERSIONS[0].1)];
        let official = assemble(OFFICIAL);
        assert_eq!(identity_with(&official, &table), LogicIdentity::Current);
        assert_eq!(
            identity_with(&official.replace('\n', "\r\n"), &table),
            LogicIdentity::Current,
            "CRLF is the same logic"
        );
    }

    /// An older official section is `Official` at its version.
    #[test]
    fn an_older_section_is_official() {
        let older = section_hash(OLD_SECTION);
        let table = [(1u32, older.as_str()), (2, LOGIC_VERSIONS[0].1)];
        assert_eq!(
            identity_with(&file_with(OLD_SECTION), &table),
            LogicIdentity::Official { version: 1 }
        );
    }

    /// One character changed, or no LOGIC banner at all, is `Modified`.
    #[test]
    fn a_changed_or_bannerless_logic_is_modified() {
        let mut changed = assemble(OFFICIAL);
        let at = changed.find("custom_checks").expect("the stub");
        changed.replace_range(at..at + 1, "C");
        assert_eq!(identity(&changed), LogicIdentity::Modified);
        // No LOGIC banner.
        let text = format!("{OFFICIAL}\nfn check_team(team) {{ [] }}\n");
        assert_eq!(identity(&text), LogicIdentity::Modified);
    }

    /// Without a CUSTOM CHECKS banner the section runs to the file's
    /// end — the same logic, `Current`.
    #[test]
    fn no_custom_checks_banner_hashes_to_the_end() {
        let text = format!("{OFFICIAL}\n{LOGIC}");
        assert_eq!(identity(&text), LogicIdentity::Current);
    }

    /// A data block's own CUSTOM CHECKS section lands after the logic,
    /// verbatim, and `custom_checks` is defined once.
    #[test]
    fn a_data_blocks_custom_checks_section_is_kept() {
        let section = format!(
            "{CUSTOM_CHECKS}\nfn custom_checks(team) {{ [#{{ kind: \"Custom\", text: \"x\" }}] }}\n"
        );
        let assembled = assemble(&format!("{OFFICIAL}\n{section}"));
        let data = assembled.find("const RULESET").expect("the data");
        let logic = assembled.find("// ==== LOGIC").expect("the logic");
        let section_at = assembled.find(CUSTOM_CHECKS).expect("the section");
        assert!(
            data < logic && logic < section_at,
            "data, logic, section, in that order"
        );
        assert!(
            assembled.ends_with(&format!("{}\n", section.trim_end())),
            "the section verbatim, to the file's end"
        );
        assert_eq!(assembled.matches("fn custom_checks").count(), 1);
    }

    /// Without the banner the file is the data, the logic and the empty
    /// stub, exactly.
    #[test]
    fn without_a_custom_checks_section_the_stub_is_emitted() {
        assert_eq!(
            assemble(OFFICIAL),
            format!(
                "{}\n\n{}\n\n{CUSTOM_CHECKS}\nfn custom_checks(team) {{ [] }}\n",
                OFFICIAL.trim_end(),
                LOGIC.trim_end()
            )
        );
    }

    /// `file_sections` of the assembled Autumn 26: the LOGIC section as
    /// read, and `None` for the stub it carries (the stub counts as
    /// none).
    #[test]
    fn file_sections_reads_autumns_logic_and_no_checks() {
        let text = assemble(include_str!("../rulesets/autumn26.rhai"));
        assert!(text.contains("fn custom_checks(team) { [] }"));
        let sections = file_sections(&text);
        assert_eq!(
            sections.logic.as_deref().map(str::trim_end),
            Some(LOGIC.trim_end())
        );
        assert_eq!(sections.custom_checks, None);
    }

    /// `file_sections` of VTL11's assembled file: its block's CUSTOM
    /// CHECKS section, banner included, as read.
    #[test]
    fn file_sections_reads_vtl11s_custom_checks() {
        let block = include_str!("../rulesets/vtl11.rhai");
        let expected = &block[custom_checks_at(block)..];
        let sections = file_sections(&assemble(block));
        assert_eq!(
            sections.custom_checks.as_deref().map(str::trim_end),
            Some(expected.trim_end())
        );
    }

    /// Without the LOGIC banner there is no section.
    #[test]
    fn file_sections_without_a_banner_is_none() {
        let sections = file_sections(OFFICIAL);
        assert_eq!(sections.logic, None);
        assert_eq!(sections.custom_checks, None);
    }

    /// `assemble_file` writes a file `Rules::load` reads back: the
    /// current logic and an equal ruleset.
    #[test]
    fn assemble_file_reads_back_current_with_an_equal_ruleset() {
        let ruleset =
            Ruleset::from_data_block(include_str!("../rulesets/autumn26.rhai")).expect("block");
        let rules = crate::rules::Rules::load(&assemble_file(&ruleset, None, None)).expect("loads");
        assert_eq!(rules.logic(), LogicIdentity::Current);
        assert_eq!(rules.ruleset(), &ruleset);
    }

    /// A file assembled with a changed logic section keeps that section
    /// and reads back `Modified`; re-assembled with `None` it is
    /// `Current` again, the replacement `update_logic` performs.
    #[test]
    fn assemble_file_keeps_a_given_section_and_none_restores_current() {
        let modified = LOGIC.replacen(
            "// The generic interpreter",
            "// The interpreter, its comment edited",
            1,
        );
        let ruleset =
            Ruleset::from_data_block(include_str!("../rulesets/autumn26.rhai")).expect("block");
        let text = assemble_file(&ruleset, Some(&modified), None);
        assert_eq!(
            file_sections(&text).logic.as_deref().map(str::trim_end),
            Some(modified.trim_end()),
            "the section is kept"
        );
        assert_eq!(
            crate::rules::Rules::load(&text).expect("loads").logic(),
            LogicIdentity::Modified
        );
        let text = assemble_file(&ruleset, None, None);
        assert_eq!(
            crate::rules::Rules::load(&text).expect("loads").logic(),
            LogicIdentity::Current
        );
    }

    /// The older-official path: a file whose section hashes to the
    /// table's first entry is `Official` at that version; re-assembled
    /// with `None` it is `Current` against the same table, where
    /// `update-logic`'s replacement is proven (no real older file
    /// exists while logic 1 is the only version).
    #[test]
    fn an_older_section_is_official_then_current_after_reassembly() {
        let older = LOGIC.replacen("// The generic interpreter", "// An older logic", 1);
        let older_hash = section_hash(&older);
        let table = [(1u32, older_hash.as_str()), (2, LOGIC_VERSIONS[0].1)];
        let ruleset = Ruleset::from_data_block(OFFICIAL).expect("block");
        let text = assemble_file(&ruleset, Some(&older), None);
        assert_eq!(
            identity_with(&text, &table),
            LogicIdentity::Official { version: 1 }
        );
        assert_eq!(
            identity_with(&assemble_file(&ruleset, None, None), &table),
            LogicIdentity::Current
        );
    }

    /// `LOGIC_VERSIONS`' last entry is the current `logic.rhai`'s
    /// hash — a released logic's hash never changes (a changed logic
    /// is a new version, in the banner and the table).
    #[test]
    fn logic_versions_last_is_the_current_logic() {
        let (version, hash_) = LOGIC_VERSIONS.last().copied().expect("an entry");
        assert_eq!(
            hash_,
            section_hash(LOGIC),
            "a released logic's hash never changes: a changed logic is a new version (update the banner and the table)"
        );
        // The banner's `ruleset-logic N` is the version.
        let banner_version: u32 = LOGIC
            .split("ruleset-logic ")
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .and_then(|n| n.parse().ok())
            .expect("the banner names a version");
        assert_eq!(version, banner_version);
    }
}
