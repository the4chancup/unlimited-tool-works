//! `Rules`: a loaded rules file — its ruleset, its compiled script and
//! the engine it runs on — and the host API over a `TeamView`:
//! `check_team`, `tier_values`, `card_limits`, `player_tier` and
//! `apply_tier`.

use pes_savefile::model::player::PlayerEntry;
use serde::de::DeserializeOwned;

use crate::file::{LogicIdentity, assemble, identity};
use crate::ruleset::{LoadError, Ruleset, Severity, Tier, ruleset_from_scope, run_top_level};
use crate::team::{PlayerView, TeamView};
use crate::values::{CardLimits, TierValues};
use crate::violation::{CheckError, Violation, ViolationKind, violations};

/// The official data block, embedded; also `templates.rs`' first entry.
/// The other rulesets' blocks are embedded by `templates.rs` (the
/// Ruleset editor's start page) and by their fixtures.
pub(crate) const AUTUMN26: &str = include_str!("../rulesets/autumn26.rhai");

/// The ending every rules file a member or an organizer handles
/// carries (`spec/aatf_rules.md` "The rules file"): the one the pickers
/// offer and the suggested names end in. The content stays plain Rhai;
/// the loader reads the content, never the ending.
pub const RULES_FILE_EXTENSION: &str = "4ccrules";

/// The embedded official rules file, the one `Rules::official` loads:
/// `AUTUMN26` assembled with the current logic.
pub fn official_file() -> String {
    assemble(AUTUMN26)
}

/// A loaded rules file: its ruleset, its compiled script and the engine
/// it runs on. `Send + Sync` (rhai's `sync` feature): several threads
/// check teams on one loaded file.
#[derive(Debug)]
pub struct Rules {
    /// The ruleset the file's `RULESET` deserialized into.
    ruleset: Ruleset,
    /// The compiled script (`check_team`, `tier_values`, `card_limits`,
    /// `player_tier`, `custom_checks` live in it).
    ast: rhai::AST,
    /// The engine the script runs on (`engine.rs` — same one as every
    /// other path).
    engine: rhai::Engine,
    /// The file's logic identity (`file.rs`).
    logic: LogicIdentity,
}

/// The functions the file must define, and their arities.
const REQUIRED: [(&str, usize); 4] = [
    ("check_team", 1),
    ("tier_values", 3),
    ("card_limits", 3),
    ("player_tier", 2),
];

impl Rules {
    /// Loads a rules file through steps 1 to 3 of `spec/aatf_rules.md`
    /// "Loading and validation"; the first failure is returned.
    pub fn load(text: &str) -> Result<Rules, LoadError> {
        let (engine, ast, scope) = run_top_level(text)?;
        // Step 1: the four functions with their arities, and every
        // `custom_checks` with one.
        for (name, arity) in REQUIRED {
            if !ast
                .iter_functions()
                .any(|f| f.name == name && f.params.len() == arity)
            {
                return Err(LoadError::MissingFunction { name, arity });
            }
        }
        for arity in ast
            .iter_functions()
            .filter(|f| f.name == "custom_checks")
            .map(|f| f.params.len())
        {
            if arity != 1 {
                return Err(LoadError::CustomChecksArity { arity });
            }
        }
        let ruleset = ruleset_from_scope(&scope)?;
        let rules = Rules {
            ruleset,
            ast,
            engine,
            logic: identity(text),
        };
        // Step 3: `check_team` runs on the sample team; a `CheckError`
        // there is a load failure — the file's own logic and custom
        // checks must run before any real check does.
        if let Err(error) = rules.check_team(&TeamView::sample_team()) {
            return Err(LoadError::SampleTeam { error });
        }
        Ok(rules)
    }

    /// The official ruleset, the default file: `rulesets/autumn26.rhai`
    /// assembled with the current logic (`official_file`), the first
    /// built-in ruleset. Panics only if the embedded file does not load,
    /// which a test rules out.
    pub fn official() -> Rules {
        crate::templates::builtin("autumn26").expect("autumn26 is built in")
    }

    /// The rules of an embedded data block assembled with the current
    /// logic: the built-in rulesets (`templates::builtin`) and the other
    /// rulesets' fixtures run on these.
    pub(crate) fn from_data_block(data_block: &str) -> Result<Rules, LoadError> {
        Rules::load(&assemble(data_block))
    }

    /// The file's ruleset.
    pub fn ruleset(&self) -> &Ruleset {
        &self.ruleset
    }

    /// The file's logic identity: the current logic, an older official
    /// version, or modified.
    pub fn logic(&self) -> LogicIdentity {
        self.logic
    }

    /// `team`'s violations: the roster-order breaks of the host's own check,
    /// raised in Rust before the script runs (`spec/aatf_rules.md` "The
    /// host"), then the file's `check_team(team)` converted. The list is
    /// ordered by roster slot, each slot's own rows kept (a slot's roster
    /// break first), the team's rows (no slot) last. The violations panels
    /// display their groups in this order.
    pub fn check_team(&self, team: &TeamView) -> Result<Vec<Violation>, CheckError> {
        let mut out: Vec<Violation> = team
            .roster_breaks
            .iter()
            .map(|&(slot, player)| Violation {
                slot: Some(slot),
                severity: Severity::Error,
                kind: ViolationKind::RosterOrder { player },
            })
            .collect();
        let list = self.call("check_team", (team.clone(),))?;
        out.extend(violations(list)?);
        // A stable sort keeps each slot's own order: the break first.
        out.sort_by_key(|violation| (violation.slot.is_none(), violation.slot));
        Ok(out)
    }

    /// The targets `player` (of `team`) must carry as tier `tier`, the
    /// specials applied: the file's `tier_values`.
    pub(crate) fn tier_values(
        &self,
        team: &TeamView,
        player: &PlayerView,
        tier: &str,
    ) -> Result<TierValues, CheckError> {
        self.values("tier_values", team, player, tier)
    }

    /// The card economy of `player` (of `team`) as tier `tier`: the
    /// file's `card_limits`, behind the card pickers' remaining counts.
    pub fn card_limits(
        &self,
        team: &TeamView,
        player: &PlayerView,
        tier: &str,
    ) -> Result<CardLimits, CheckError> {
        self.values("card_limits", team, player, tier)
    }

    /// The tier `player` (of `team`) is recognized as, as `check_team`
    /// reads him: the team's height system, then the first tier in list
    /// order whose condition holds and whose expected rating he reaches,
    /// else the last whose condition holds (`spec/aatf_rules.md` "How a
    /// player is read", steps 1 and 2): the file's `player_tier`. A player
    /// rated under every such tier still has one; `None` only when no
    /// tier's condition holds, where `check_team` reports `NoTier`. Behind
    /// the Save editor's cards badge, which then asks `card_limits` for
    /// this tier.
    pub fn player_tier(
        &self,
        team: &TeamView,
        player: &PlayerView,
    ) -> Result<Option<&Tier>, CheckError> {
        let result = self.call("player_tier", (player.clone(), team.clone()))?;
        if result.is_unit() {
            return Ok(None);
        }
        let id = result
            .into_immutable_string()
            .map_err(|type_name| CheckError::Values {
                function: "player_tier",
                message: format!("{type_name} is no tier id"),
            })?;
        self.ruleset
            .tiers
            .iter()
            .find(|tier| tier.id == id.as_str())
            .map(Some)
            .ok_or_else(|| CheckError::Values {
                function: "player_tier",
                message: format!("\"{id}\" names no tier"),
            })
    }

    /// Writes tier `tier`'s targets into `entry`, the record `player`
    /// views (of `team`): stats, form, injury resistance and weak foot,
    /// and the tier's `name_color` over the name's leading colour code
    /// (a code another tier sets stripped under a tier that sets none),
    /// so the quick actions and the Team creator's defaults produce a
    /// player `check_team` accepts on those. The lib's one writing
    /// operation; the check itself never fixes anything.
    ///
    /// The values resolve against what is written, not what the player
    /// held: a special conditioned on a value the tier sets (form, a
    /// stat, the name colour) would otherwise see the old one. The
    /// colour lands on a copy first, then each pass writes the values
    /// and recomputes `tier_values` on it until they stop changing,
    /// three writes at most, then commits; still changing is
    /// `CheckError::Unsettled` and `entry` is left as it was.
    pub fn apply_tier(
        &self,
        team: &TeamView,
        player: &PlayerView,
        tier: &str,
        entry: &mut PlayerEntry,
    ) -> Result<(), CheckError> {
        let mut values = self.tier_values(team, player, tier)?;
        let tier = self
            .ruleset
            .tiers
            .iter()
            .find(|t| t.id == tier)
            .expect("tier_values accepted the tier");
        let tier_colors: Vec<&str> = self
            .ruleset
            .tiers
            .iter()
            .filter_map(|t| t.name_color.as_deref())
            .collect();
        let mut copy = entry.clone();
        crate::values::write_name_color(&mut copy.name, tier.name_color.as_deref(), &tier_colors);
        for _ in 0..3 {
            values.write(&mut copy);
            let view = PlayerView::of_entry(team.version, player.slot, &copy);
            let settled = self.tier_values(team, &view, &tier.id)?;
            if settled == values {
                *entry = copy;
                return Ok(());
            }
            values = settled;
        }
        Err(CheckError::Unsettled {
            tier: tier.id.clone(),
        })
    }

    /// `function` of the file called with `args` on a fresh scope; a
    /// script failure (a runtime error or a limit reached) is
    /// `CheckError::Script` with rhai's text.
    fn call(&self, function: &str, args: impl rhai::FuncArgs) -> Result<rhai::Dynamic, CheckError> {
        self.engine
            .call_fn::<rhai::Dynamic>(&mut rhai::Scope::new(), &self.ast, function, args)
            .map_err(|e| CheckError::Script {
                message: e.to_string(),
            })
    }

    /// `function` (`tier_values` or `card_limits`) called on `player` as
    /// `tier`, its map converted; an id no tier has is refused here, not
    /// by the script.
    fn values<T: DeserializeOwned>(
        &self,
        function: &'static str,
        team: &TeamView,
        player: &PlayerView,
        tier: &str,
    ) -> Result<T, CheckError> {
        if !self.ruleset.tiers.iter().any(|t| t.id == tier) {
            return Err(CheckError::UnknownTier {
                tier: tier.to_string(),
            });
        }
        let map = self.call(function, (player.clone(), tier.to_string(), team.clone()))?;
        rhai::serde::from_dynamic(&map).map_err(|e| CheckError::Values {
            function,
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pes_savefile::EditFile;
    use pes_savefile::fixture_save;
    use pes_version::PesVersion;

    /// Team 701 of `version`'s fixture save.
    fn fixture_team(version: PesVersion) -> TeamView {
        let file = EditFile::from_bytes(&fixture_save(version)).expect("fixture save");
        TeamView::new(&file, 701).expect("team 701 views")
    }

    /// The smallest file that loads: the four functions and a
    /// one-tier `RULESET`.
    fn minimal_file() -> String {
        "const RULESET = #{ schema: 1, name: \"x\", squad_size: 23, universal: #{ max_skill_cards: 10 }, tiers: [#{ id: \"regular\", label: \"Regular\", rate: 77 }] };\nfn check_team(team) { [] }\nfn tier_values(player, tier, team) { #{} }\nfn card_limits(player, tier, team) { #{} }\nfn player_tier(player, team) { () }".to_string()
    }

    /// Two threads check on one loaded file and agree: the whole-save
    /// check and the Live AATF worker share the window's `Rules`.
    #[test]
    fn two_threads_check_on_one_loaded_file() {
        let rules = Rules::official();
        let team = fixture_team(PesVersion::Pes17);
        let [first, second] = std::thread::scope(|scope| {
            [
                scope.spawn(|| rules.check_team(&team).expect("checks")),
                scope.spawn(|| rules.check_team(&team).expect("checks")),
            ]
            .map(|thread| thread.join().expect("the thread ends"))
        });
        assert_eq!(first, second);
    }

    /// The official file loads, and its check runs without a script
    /// error on team 701 of every fixture save (the legal and violating
    /// fixtures are `fixtures/`; PES 20 and 21's team 701 are placeholder
    /// players, every field 0).
    #[test]
    fn official_loads_and_checks_every_fixture_version() {
        let rules = Rules::official();
        assert_eq!(rules.ruleset().name, "4cc Autumn 26");
        for version in PesVersion::ALL {
            rules
                .check_team(&fixture_team(version))
                .unwrap_or_else(|e| panic!("{version:?}: {e}"));
        }
    }

    /// The official file's logic is the current one.
    #[test]
    fn the_official_files_logic_is_current() {
        assert_eq!(Rules::official().logic(), LogicIdentity::Current);
    }

    /// Every embedded ruleset carries its event's wiki page and the logo
    /// that page shows, byte for byte, and its logic stays the current one.
    #[test]
    fn every_embedded_ruleset_carries_its_events_page_and_logo() {
        use base64::Engine as _;

        let logos = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/prototypes/assets/logos");
        for (stem, block, page) in [
            (
                "autumn26",
                AUTUMN26,
                "https://implyingrigged.info/wiki/2026_4chan_Autumn_Babby_Cup",
            ),
            (
                "autumn25",
                include_str!("../rulesets/autumn25.rhai"),
                "https://implyingrigged.info/wiki/2025_4chan_Autumn_Babby_Cup",
            ),
            (
                "spring26",
                include_str!("../rulesets/spring26.rhai"),
                "https://implyingrigged.info/wiki/2026_4chan_Spring_Babby_Cup",
            ),
            (
                "vgl27",
                include_str!("../rulesets/vgl27.rhai"),
                "https://implyingrigged.info/wiki//vg/_League_27",
            ),
            (
                "vgl26",
                include_str!("../rulesets/vgl26.rhai"),
                "https://implyingrigged.info/wiki//vg/_League_26",
            ),
            (
                "vgl25",
                include_str!("../rulesets/vgl25.rhai"),
                "https://implyingrigged.info/wiki//vg/_League_25",
            ),
            (
                "vtl11",
                include_str!("../rulesets/vtl11.rhai"),
                "https://implyingrigged.info/wiki//vt/_League_11",
            ),
            (
                "vtl10",
                include_str!("../rulesets/vtl10.rhai"),
                "https://implyingrigged.info/wiki//vt/_League_10",
            ),
            (
                "dbc7",
                include_str!("../rulesets/dbc7.rhai"),
                "https://implyingrigged.info/wiki/Divegrass_Before_Christmas_7:_Melty_Under_The_Christmas_Tree",
            ),
        ] {
            let rules = Rules::from_data_block(block).unwrap_or_else(|e| panic!("{stem}: {e}"));
            let ruleset = rules.ruleset();
            assert_eq!(ruleset.page.as_deref(), Some(page), "{stem}");
            let logo = ruleset
                .logo
                .as_deref()
                .unwrap_or_else(|| panic!("{stem}: no logo"));
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(logo)
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
            // Every /vg/ League shares one logo, `vgl.png`.
            let asset_stem = if stem.starts_with("vgl") { "vgl" } else { stem };
            let asset = std::fs::read(logos.join(format!("{asset_stem}.png")))
                .unwrap_or_else(|e| panic!("{stem}: {e}"));
            assert!(
                decoded == asset,
                "{stem}: the logo is not the asset's bytes"
            );
            assert_eq!(rules.logic(), LogicIdentity::Current, "{stem}");
        }
    }

    /// A statement added inside the logic section makes the file's
    /// logic `Modified`: the section no longer hashes to a known
    /// version.
    #[test]
    fn a_file_with_an_edited_logic_is_modified() {
        // A comment line inside `check_team`'s body: the edit lands in
        // the LOGIC section, so the section's hash changes.
        let assembled = assemble(AUTUMN26);
        assert_eq!(assembled.matches("fn check_team(team) {").count(), 1);
        let text = assembled.replacen(
            "fn check_team(team) {",
            "fn check_team(team) {\n    // an edit\n",
            1,
        );
        let edited = text.find("// an edit").expect("the edit landed");
        let logic = text.find("// ==== LOGIC").expect("the logic banner");
        let custom = text
            .find("// ==== CUSTOM CHECKS")
            .expect("the custom checks");
        assert!(
            logic < edited && edited < custom,
            "the edit is inside the logic section"
        );
        let rules = Rules::load(&text).expect("the file loads");
        assert_eq!(rules.logic(), LogicIdentity::Modified);
    }

    /// The custom checks' violations come last in their slot's rows, after
    /// the interpreter's rows for it.
    #[test]
    fn a_custom_check_reaches_the_result() {
        // The assembled file's `fn custom_checks(team) { [] }` replaced
        // by one naming the first player's slot.
        let assembled = assemble(AUTUMN26);
        assert_eq!(
            assembled.matches("fn custom_checks(team) { [] }").count(),
            1
        );
        let text = assembled.replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { [#{ kind: \"Custom\", text: \"x\", slot: team.players[0].slot }] }",
            1,
        );
        let rules = Rules::load(&text).expect("the file loads");
        let team = fixture_team(PesVersion::Pes15);
        let found = rules.check_team(&team).expect("the check runs");
        let custom = Violation {
            slot: Some(0),
            severity: crate::ruleset::Severity::Error,
            kind: crate::violation::ViolationKind::Custom {
                text: "x".to_string(),
            },
        };
        let last_of_slot0 = found
            .iter()
            .rfind(|v| v.slot == Some(0))
            .expect("slot 0 has rows");
        assert_eq!(last_of_slot0, &custom);
        assert_eq!(found.iter().filter(|v| **v == custom).count(), 1);
        assert!(found.len() > 1, "the interpreter reported on team 701 too");
    }

    /// Every map a custom check returns lands in the result, in the
    /// check's own order, each with its own slot and severity.
    #[test]
    fn a_custom_check_returns_every_finding_in_order() {
        use crate::ruleset::Severity;
        use crate::violation::ViolationKind;

        let text = assemble(AUTUMN26).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { [#{ kind: \"Custom\", text: \"first\", slot: team.players[0].slot }, #{ kind: \"Custom\", text: \"second\", slot: team.players[1].slot, severity: \"warning\" }] }",
            1,
        );
        let rules = Rules::load(&text).expect("the file loads");
        let team = fixture_team(PesVersion::Pes15);
        let found = rules.check_team(&team).expect("the check runs");
        let expected = [
            Violation {
                slot: Some(team.players[0].slot),
                severity: Severity::Error,
                kind: ViolationKind::Custom {
                    text: "first".to_string(),
                },
            },
            Violation {
                slot: Some(team.players[1].slot),
                severity: Severity::Warning,
                kind: ViolationKind::Custom {
                    text: "second".to_string(),
                },
            },
        ];
        let customs: Vec<&Violation> = found
            .iter()
            .filter(|v| matches!(v.kind, ViolationKind::Custom { .. }))
            .collect();
        assert_eq!(customs, [&expected[0], &expected[1]]);
    }

    #[test]
    fn a_script_reads_global_ruleset() {
        // `call_fn` re-evaluates the top level on a fresh scope, so the
        // script sees `global::RULESET`: the mechanism the interpreter
        // reads its data through.
        let text = assemble(AUTUMN26).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { [#{ kind: \"Custom\", text: global::RULESET.name }] }",
            1,
        );
        let rules = Rules::load(&text).expect("the file loads");
        let team = fixture_team(PesVersion::Pes15);
        let found = rules.check_team(&team).expect("the check runs");
        assert_eq!(
            found.last().map(|v| &v.kind),
            Some(&crate::violation::ViolationKind::Custom {
                text: "4cc Autumn 26".to_string(),
            })
        );
    }

    #[test]
    fn a_file_without_custom_checks_reports_no_custom_violation() {
        let text = assemble(AUTUMN26).replacen("fn custom_checks(team) { [] }", "", 1);
        assert!(!text.contains("fn custom_checks"));
        let rules = Rules::load(&text).expect("the file loads");
        let found = rules
            .check_team(&fixture_team(PesVersion::Pes15))
            .expect("the check runs");
        assert!(!found.is_empty(), "team 701 is not legal under Autumn 26");
        assert!(
            !found
                .iter()
                .any(|v| matches!(v.kind, crate::violation::ViolationKind::Custom { .. }))
        );
    }

    /// The result lists the players by slot, then the team's rows: a
    /// roster-order break on a later slot lands in its slot's place, not
    /// ahead of every player's violations, because the violations panel
    /// shows the players by slot, then the team's own rows.
    #[test]
    fn the_result_is_ordered_by_slot_the_teams_rows_last() {
        // Team 701 of the PES 15 fixture already errors on slot 0 (Stat
        // 74 < 77); break slot 5's roster order with a real player id.
        let mut file =
            EditFile::from_bytes(&fixture_save(PesVersion::Pes15)).expect("fixture save");
        file.team_mut(701).unwrap().roster[5].player_id = 70101;
        let team = TeamView::new(&file, 701).expect("team 701 views");
        assert_eq!(team.roster_breaks, [(5, 70101)], "premise: slot 5 breaks");

        let found = Rules::official().check_team(&team).expect("the check runs");
        // Slot 0's Stat error precedes the break's RosterOrder, which leads
        // slot 5's own group; every slot's rows are contiguous and in order.
        assert_eq!(found[0].slot, Some(0));
        assert!(
            matches!(found[0].kind, ViolationKind::Stat { .. }),
            "{:?}",
            found[0].kind
        );
        let slots: Vec<Option<u8>> = found.iter().map(|v| v.slot).collect();
        assert!(
            slots
                .windows(2)
                .all(|pair| pair[0].unwrap_or(u8::MAX) <= pair[1].unwrap_or(u8::MAX)),
            "the slots are non-decreasing: {slots:?}"
        );
        let slot5: Vec<&Violation> = found.iter().filter(|v| v.slot == Some(5)).collect();
        assert!(
            matches!(slot5[0].kind, ViolationKind::RosterOrder { player: 70101 }),
            "the break leads slot 5's group: {slot5:?}"
        );
        let team_rows = found
            .iter()
            .position(|v| v.slot.is_none())
            .expect("the team has its own rows");
        assert!(found[team_rows..].iter().all(|v| v.slot.is_none()));
    }

    /// A data block's own CUSTOM CHECKS section is kept by
    /// `from_data_block` and its `custom_checks` runs.
    #[test]
    fn a_data_blocks_custom_check_reports() {
        let data = format!(
            "{AUTUMN26}\n\n{}\nfn custom_checks(team) {{ [#{{ kind: \"Custom\", text: \"x\", slot: team.players[0].slot }}] }}\n",
            crate::file::CUSTOM_CHECKS
        );
        let rules = Rules::from_data_block(&data).expect("the file loads");
        let found = rules
            .check_team(&fixture_team(PesVersion::Pes15))
            .expect("the check runs");
        let custom = Violation {
            slot: Some(0),
            severity: crate::ruleset::Severity::Error,
            kind: crate::violation::ViolationKind::Custom {
                text: "x".to_string(),
            },
        };
        assert!(found.contains(&custom), "{found:?}");
    }

    /// Every embedded ruleset on the sample team costs under a tenth of
    /// `MAX_OPERATIONS`, so the limit keeps its tenfold margin, and it is not
    /// stale: under thirty times the heaviest one's cost. Every block's count
    /// and time so far are in the assertion messages: to read the table,
    /// tighten a bound for one run.
    #[test]
    fn the_operation_limit_keeps_a_tenfold_margin() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU64, Ordering};

        let mut heaviest = 0;
        let mut table = String::new();
        for (name, block) in [
            ("autumn26", AUTUMN26),
            ("autumn25", include_str!("../rulesets/autumn25.rhai")),
            ("spring26", include_str!("../rulesets/spring26.rhai")),
            ("vgl25", include_str!("../rulesets/vgl25.rhai")),
            ("vgl26", include_str!("../rulesets/vgl26.rhai")),
            ("vgl27", include_str!("../rulesets/vgl27.rhai")),
            ("vtl10", include_str!("../rulesets/vtl10.rhai")),
            ("vtl11", include_str!("../rulesets/vtl11.rhai")),
            ("dbc7", include_str!("../rulesets/dbc7.rhai")),
        ] {
            let mut rules = Rules::from_data_block(block).unwrap_or_else(|e| panic!("{name}: {e}"));
            let peak = Arc::new(AtomicU64::new(0));
            let seen = Arc::clone(&peak);
            rules.engine.on_progress(move |operations| {
                seen.fetch_max(operations, Ordering::Relaxed);
                None
            });
            let start = std::time::Instant::now();
            rules
                .check_team(&TeamView::sample_team())
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let elapsed = start.elapsed();
            let operations = peak.load(Ordering::Relaxed);
            table.push_str(&format!("\n{name}: {operations} operations in {elapsed:?}"));
            assert!(operations > 0, "{name}: on_progress saw the run");
            assert!(
                operations * 10 <= crate::engine::MAX_OPERATIONS,
                "{name}: the margin is under tenfold; raise MAX_OPERATIONS{table}"
            );
            heaviest = heaviest.max(operations);
        }
        assert!(
            crate::engine::MAX_OPERATIONS <= heaviest * 30,
            "{heaviest} operations at the heaviest: MAX_OPERATIONS is stale; lower it to ten times that cost{table}"
        );
    }

    /// The tables the logic carries itself (the ability stats it targets,
    /// the positions it counts, the COM styles of the `com` group) are
    /// the label tables the host converts with: a stat missing from the
    /// logic's list would go unchecked without a word.
    #[test]
    fn the_logics_tables_are_the_label_tables() {
        use pes_savefile::interchange::team_toml::labels::{ABILITY_STATS, COM_STYLES, POSITIONS};

        let rules = Rules::official();
        for (function, table) in [
            ("ability_stats", ABILITY_STATS.as_slice()),
            ("positions", POSITIONS.as_slice()),
            ("com_styles", COM_STYLES.as_slice()),
        ] {
            let listed: Vec<String> = rules
                .engine
                .call_fn::<rhai::Array>(&mut rhai::Scope::new(), &rules.ast, function, ())
                .expect(function)
                .into_iter()
                .map(|name| name.into_string().expect("a name"))
                .collect();
            assert_eq!(listed, table, "{function}");
        }
    }

    /// A `tier_values` returning a map that is not the values (the
    /// minimal file's `#{}`) is `Values` naming the function.
    #[test]
    fn a_values_map_that_does_not_convert_is_values() {
        let rules = Rules::load(&minimal_file()).expect("the file loads");
        let team = fixture_team(PesVersion::Pes15);
        let player = team.player(0).expect("slot 0");
        match rules
            .tier_values(&team, player, "regular")
            .expect_err("an empty map")
        {
            CheckError::Values { function, .. } => assert_eq!(function, "tier_values"),
            other => panic!("expected Values, got {other:?}"),
        }
        match rules
            .card_limits(&team, player, "regular")
            .expect_err("an empty map")
        {
            CheckError::Values { function, .. } => assert_eq!(function, "card_limits"),
            other => panic!("expected Values, got {other:?}"),
        }
    }

    /// `player_tier`'s `()` is `None`; an id no tier has, or another
    /// type, is `Values` naming the function; a tier's id is that tier.
    #[test]
    fn player_tier_reads_unit_as_none_and_refuses_an_id_no_tier_has() {
        let team = fixture_team(PesVersion::Pes15);
        let player = team.player(0).expect("slot 0");
        let with_body = |body: &str| {
            Rules::load(&minimal_file().replacen(
                "fn player_tier(player, team) { () }",
                &format!("fn player_tier(player, team) {{ {body} }}"),
                1,
            ))
            .expect("the file loads")
        };
        assert_eq!(
            with_body("()")
                .player_tier(&team, player)
                .expect("the call runs"),
            None
        );
        let err = with_body("\"bronze\"")
            .player_tier(&team, player)
            .expect_err("an id no tier has");
        match err {
            CheckError::Values { function, message } => {
                assert_eq!(function, "player_tier");
                assert!(message.contains("bronze"), "{message}");
            }
            other => panic!("expected Values, got {other:?}"),
        }
        let err = with_body("42")
            .player_tier(&team, player)
            .expect_err("not a string");
        match err {
            CheckError::Values { function, .. } => assert_eq!(function, "player_tier"),
            other => panic!("expected Values, got {other:?}"),
        }
        let rules = with_body("\"regular\"");
        let tier = rules
            .player_tier(&team, player)
            .expect("the call runs")
            .expect("a tier");
        assert_eq!(tier.id, "regular");
    }

    /// A script that loops forever stops at the operation limit with a
    /// `Script` error, not a hang: a `loop {}` on PES 21 only, so the
    /// sample team's PES 15 load passes.
    #[test]
    fn a_script_that_loops_forever_stops_at_the_operation_limit() {
        let text = assemble(AUTUMN26).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { if team.version == 21 { loop {} } [] }",
            1,
        );
        let err = Rules::load(&text)
            .expect("the file loads")
            .check_team(&fixture_team(PesVersion::Pes21))
            .expect_err("the limit stops it");
        match &err {
            CheckError::Script { message } => {
                assert!(message.contains("operations"), "{message}");
            }
            other => panic!("expected Script, got {other:?}"),
        }
    }

    #[test]
    fn a_script_error_and_a_bad_violation_surface() {
        // `throw` on PES 21 only: the sample team's PES 15 load passes,
        // and the check on a PES 21 team is `CheckError::Script`.
        let text = assemble(AUTUMN26).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { if team.version == 21 { throw \"boom\" } [] }",
            1,
        );
        let err = Rules::load(&text)
            .expect("the file loads")
            .check_team(&fixture_team(PesVersion::Pes21))
            .expect_err("a script error");
        match &err {
            CheckError::Script { message } => assert!(message.contains("boom"), "{message}"),
            other => panic!("expected Script, got {other:?}"),
        }
        // A bad entry in the list fails at its index, on PES 21 only: the
        // custom checks' entries follow the interpreter's own.
        let own = Rules::official()
            .check_team(&fixture_team(PesVersion::Pes21))
            .expect("the check runs")
            .len();
        assert!(own > 0, "team 701 is not legal under Autumn 26");
        let text = assemble(AUTUMN26).replacen(
            "fn custom_checks(team) { [] }",
            "fn custom_checks(team) { if team.version == 21 { [#{ kind: \"Captain\" }, #{ kind: \"Nope\" }] } else { [] } }",
            1,
        );
        let err = Rules::load(&text)
            .expect("the file loads")
            .check_team(&fixture_team(PesVersion::Pes21))
            .expect_err("a bad violation");
        match &err {
            CheckError::Violation { index, message } => {
                assert_eq!(*index, own + 1);
                assert!(message.contains("Nope"), "{message}");
            }
            other => panic!("expected Violation, got {other:?}"),
        }
    }

    /// One failing file per step-1 shape: the parse position, the
    /// function arities, `RULESET` absent.
    #[test]
    fn load_step_one_refuses_each_missing_shape() {
        // A stray token: `Parse` at its line and column.
        let err = Rules::load("const A = 1;\nconst B = 2;\nhello\n").expect_err("a stray token");
        match err {
            LoadError::Parse { line, column, .. } => {
                assert_eq!((line, column), (3, 1));
            }
            other => panic!("expected Parse, got {other:?}"),
        }
        // Each required function missing.
        for (remove, name, arity) in [
            ("fn check_team(team) { [] }", "check_team", 1usize),
            (
                "fn tier_values(player, tier, team) { #{} }",
                "tier_values",
                3,
            ),
            (
                "fn card_limits(player, tier, team) { #{} }",
                "card_limits",
                3,
            ),
            ("fn player_tier(player, team) { () }", "player_tier", 2),
        ] {
            let text = minimal_file().replacen(remove, "", 1);
            assert!(!text.contains(remove));
            assert_eq!(
                Rules::load(&text).expect_err(name),
                LoadError::MissingFunction { name, arity },
                "{name}"
            );
        }
        // `check_team` exists, but with two parameters only.
        let err = Rules::load(&minimal_file().replacen(
            "fn check_team(team) { [] }",
            "fn check_team(team, extra) { [] }",
            1,
        ))
        .expect_err("wrong arity");
        assert_eq!(
            err,
            LoadError::MissingFunction {
                name: "check_team",
                arity: 1
            }
        );
        // `custom_checks` with two parameters.
        let err = Rules::load(&format!(
            "{minimal}\nfn custom_checks(team, extra) {{ [] }}",
            minimal = minimal_file()
        ))
        .expect_err("custom_checks arity");
        assert_eq!(err, LoadError::CustomChecksArity { arity: 2 });
        // No `RULESET`.
        let err = Rules::load(&minimal_file().replacen("const RULESET = ", "const OTHER = ", 1))
            .expect_err("no RULESET");
        assert_eq!(err, LoadError::NoRuleset);
        // A `let RULESET` is not a constant either.
        let err = Rules::load(&minimal_file().replacen("const RULESET = ", "let RULESET = ", 1))
            .expect_err("a let");
        assert_eq!(err, LoadError::NoRuleset);
    }

    /// Step 2: `RULESET` breaking a load-time rule gives `Rules`.
    #[test]
    fn load_step_two_reports_the_rules() {
        let text = minimal_file().replacen("name: \"x\"", "name: \"\"", 1);
        let err = Rules::load(&text).expect_err("empty name");
        assert!(matches!(err, LoadError::Rules(_)), "{err:?}");
    }

    /// Step 3: `check_team` failing on the sample team (PES 15) is a
    /// load failure, `CheckError` carried whole.
    #[test]
    fn load_step_three_runs_check_team_on_the_sample_team() {
        // A throw only on PES 15: `SampleTeam` holding `Script`.
        let err = Rules::load(&minimal_file().replacen(
            "fn check_team(team) { [] }",
            "fn check_team(team) { if team.version == 15 { throw \"boom\" } [] }",
            1,
        ))
        .expect_err("a script error on the sample team");
        match &err {
            LoadError::SampleTeam { error } => {
                assert!(
                    matches!(error, CheckError::Script { message } if message.contains("boom")),
                    "{err:?}"
                );
            }
            other => panic!("expected SampleTeam, got {other:?}"),
        }
        // A bad violation map only on PES 15: `SampleTeam` holding
        // `Violation` at index 0.
        let err = Rules::load(&minimal_file().replacen(
            "fn check_team(team) { [] }",
            "fn check_team(team) { if team.version == 15 { [#{ kind: \"Nope\" }] } else { [] } }",
            1,
        ))
        .expect_err("a bad map on the sample team");
        match &err {
            LoadError::SampleTeam {
                error: CheckError::Violation { index, .. },
            } => assert_eq!(*index, 0),
            other => panic!("expected SampleTeam holding Violation, got {other:?}"),
        }
    }
}
